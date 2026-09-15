//! VINTF device-manifest virtualization for the HIDL servicemanager.
//!
//! 6-Z305t-67. The A11 registration pre-check
//! (`registerAsServiceInternal`, transport/ServiceManagement.cpp:872) is
//! ONE wire call: `sm->getTransport(descriptor, name)`; if the answer is
//! not HWBINDER the HAL fails with "must be in VINTF manifest in order
//! to register/get" (ServiceManagement.cpp:878). The same call is
//! EVERY HIDL client's transport discovery (`getRawServiceInternal`,
//! ServiceManagement.cpp:779): EMPTY + `PRODUCT_ENFORCE_VINTF_MANIFEST`
//! makes the service unreachable via getService EVEN WHILE RUNNING.
//!
//! The real hwservicemanager answers this from the VINTF manifests, NOT
//! from its service map (system/hwservicemanager/ServiceManager.cpp:414
//! `ServiceManager::getTransport` → Vintf.cpp:36 `getTransport`, which
//! consults `VintfObject::GetFrameworkHalManifest()` FIRST and
//! `VintfObject::GetDeviceHalManifest()` SECOND; an unregistered but
//! manifest-declared service answers HWBINDER before it ever runs).
//! The pre-6-Z305t-67 proxy answered from the bus registry only, so
//! every manifest-enforced registration pre-check failed cleanly and
//! every manifest HAL's clients took the empty-transport path.
//!
//! This module is the container analogue: parse the guest rootfs's own
//! VINTF manifests (framework group first, then device group — the same
//! consult order as Vintf.cpp) and answer `fq/instance → transport`
//! from them. The manifest files ARE the ground truth the guest image
//! ships, so the semantics stay generic (no ROM-specific tables): a GSI
//! that declares `android.hardware.foo@1.0::IFoo/default` gets exactly
//! the transport behavior a real device with that manifest has.
//!
//! Deviations from Vintf.cpp, all documented on purpose:
//! - SKU-specific manifests (`manifest_{sku}.xml`) are not consulted —
//!   the container has no vendor SKU properties.
//! - `<regex-instance>` entries are skipped (no regex engine here);
//!   their clients take the honest EMPTY/no-entry path.
//!
//! 6-Z372 (rn327 decode — the HAL registration restart-loop): the parse
//! dropped REAL manifest declarations, so `getTransport` answered EMPTY
//! for services the device DOES ship and `registerAsServiceInternal`
//! (ServiceManagement.cpp:872) refused their registration client-side —
//! no `addWithChain` ever reached the proxy, the HAL CHECK-aborted, and
//! init restart-looped it (health-hal-2-1 25×, drm clearkey/widevine
//! 24× each, nn samples, light/power/rebootescrow). Two dropped shapes,
//! now parsed:
//! - **HIDL version RANGES** (`<version>2.0-2.1</version>`): libvintf
//!   stores a VersionRange and answers getHal for ANY version inside
//!   it; the parser stored the range string verbatim so BOTH endpoints
//!   missed.
//! - **AIDL `<fqname>` entries**: the compact AIDL manifest shape
//!   (`<hal format="aidl"><name>pkg</name><fqname>IFace/instance
//!   </fqname></hal>` — hardware/interfaces light/aidl ships
//!   lights-default.xml exactly like this). libvintf accepts the
//!   relative (IFace/instance) and fully-qualified (pkg.IFace/instance)
//!   spellings; both are indexed.
//! A fragment that STILL parses to zero entries now logs a bounded
//! skeleton of its content so the next decode sees the residual shape
//! instead of a bare counter.
//! - The bus-registry fallback in the caller (`binder.rs`
//!   HIDL_SM_GET_TRANSPORT) answers HWBINDER for in-proxy virtual
//!   services — kernel-provided services that exist before any guest
//!   runs, the analogue of manifest-declared services. Real
//!   hwservicemanager would answer EMPTY for them (they are not in any
//!   manifest); the container deliberately answers HWBINDER because
//!   they ARE provided — by the virtual kernel, not by a HAL process.

use std::collections::HashMap;
use std::fs;
use std::sync::OnceLock;

use crate::{info, warning};

/// `IServiceManager1_0::Transport` wire values (hwbinder::Parcel
/// marshals the `enum Transport : uint8_t` as ONE byte).
pub const TRANSPORT_EMPTY: u8 = 0;
/// Manifest/registry answer: service is a binder service.
pub const TRANSPORT_HWBINDER: u8 = 1;
/// Manifest answer: service is a passthrough (dlopen) service.
pub const TRANSPORT_PASSTHROUGH: u8 = 2;

/// Key namespace for AIDL (`format="aidl"`) entries. Their service
/// names carry no `@version`, so they can never collide with a HIDL
/// `fq/instance` key; they are indexed for future manifest consumers.
const AIDL_KEY_PREFIX: &str = "aidl:";

/// Framework-group manifest files, in consult order (VintfObject.cpp
/// fetchFrameworkHalManifest, android-11.0.0_r1: /system/etc/vintf/
/// manifest.xml + fragments + /product + /system_ext; legacy
/// /system/manifest.xml last). Rootfs-relative.
const FRAMEWORK_MANIFESTS: &[&str] = &[
    "system/etc/vintf/manifest.xml",
    "product/etc/vintf/manifest.xml",
    "system_ext/etc/vintf/manifest.xml",
    "system/manifest.xml",
];
/// Framework fragment directories (files merged in sorted order).
const FRAMEWORK_FRAGMENT_DIRS: &[&str] = &[
    "system/etc/vintf/manifest",
    "product/etc/vintf/manifest",
    "system_ext/etc/vintf/manifest",
];
/// Device-group manifest files, in consult order (VintfObject.cpp
/// fetchDeviceHalManifest: /vendor/etc/vintf/manifest.xml + /odm + the
/// legacy /vendor/manifest.xml; SKU variants skipped — no SKU props).
const DEVICE_MANIFESTS: &[&str] = &[
    "vendor/etc/vintf/manifest.xml",
    "odm/etc/vintf/manifest.xml",
    "vendor/manifest.xml",
];
/// Device fragment directories.
const DEVICE_FRAGMENT_DIRS: &[&str] = &["vendor/etc/vintf/manifest", "odm/etc/vintf/manifest"];

static ROOTFS: OnceLock<String> = OnceLock::new();
/// Parsed manifests (or a definitive `None` when no rootfs was set or
/// nothing usable was found — cached so lookups stay O(1) after boot).
static MANIFESTS: OnceLock<Option<VintfManifests>> = OnceLock::new();

/// Record the guest rootfs directory the manifests are read from.
///
/// Called once from `lib.rs` step 2.5 (binder proxy creation). A no-op
/// on later calls; when never called (unit tests, proxy creation
/// failed), [`lookup`] answers `None` and the servicemanager falls
/// back to registry-only semantics — the pre-6-Z305t-67 behavior.
pub fn set_rootfs(rootfs: &str) {
    let _ = ROOTFS.set(rootfs.to_string());
}

/// Manifest consult for one HIDL service lookup: the transport the
/// real hwservicemanager would answer for `fq`/`instance`, or `None`
/// when the entry is in no manifest (or no manifests are available).
pub fn lookup(fq: &str, instance: &str) -> Option<u8> {
    let manifests = cached_manifests()?;
    manifests.lookup(fq, instance)
}

fn cached_manifests() -> Option<&'static VintfManifests> {
    let cached = MANIFESTS.get_or_init(|| {
        let rootfs = ROOTFS.get()?;
        Some(parse_rootfs_manifests(rootfs))
    });
    cached.as_ref()
}

/// Parse every manifest the rootfs ships. Framework group first (its
/// entries win — Vintf.cpp consults framework before device), main
/// manifests before sorted fragments, first insert wins inside a group.
fn parse_rootfs_manifests(rootfs: &str) -> VintfManifests {
    let mut out = VintfManifests::default();
    for group in [
        (FRAMEWORK_MANIFESTS, FRAMEWORK_FRAGMENT_DIRS),
        (DEVICE_MANIFESTS, DEVICE_FRAGMENT_DIRS),
    ] {
        for path in group.0 {
            merge_file(&mut out, rootfs, path);
        }
        for dir in group.1 {
            for fragment in sorted_fragments(rootfs, dir) {
                merge_file(&mut out, rootfs, &fragment);
            }
        }
    }
    if out.is_empty() {
        info!("[KR64][vintf] no usable manifest entries under {}", rootfs);
    } else {
        info!(
            "[KR64][vintf] indexed {} manifest {}",
            out.len(),
            if out.len() == 1 { "entry" } else { "entries" }
        );
    }
    out
}

fn merge_file(out: &mut VintfManifests, rootfs: &str, rel: &str) {
    let path = format!("{}/{}", rootfs.trim_end_matches('/'), rel);
    if let Ok(xml) = fs::read_to_string(&path) {
        let parsed = parse_manifest(&xml);
        if parsed.is_empty() {
            // 6-Z372: a bare counter forced a CI round-trip per shape
            // guess (rn327: 9 fragments parsed to zero and EVERY one of
            // them restarted-looped its HAL). Log a bounded skeleton of
            // the actual content — the manifest cache loads once per
            // run, so this fires at most once per broken file.
            warning!(
                "[KR64][vintf] {} parsed to zero entries — skeleton: {}",
                rel,
                xml_skeleton(&xml)
            );
        } else {
            info!("[KR64][vintf] {} → {} entries", rel, parsed.len());
        }
        out.merge_first_wins(&parsed);
    }
    // Absent manifests are the NORMAL case (GSI without vendor,
    // system_ext without entries) — silence, not a warning.
}

/// Bounded one-line skeleton of a manifest that parsed to zero
/// entries: comments stripped, whitespace collapsed, capped at 240
/// chars. Diagnostic only — the shape (or emptiness) of the file names
/// the parser gap on the next decode.
fn xml_skeleton(xml: &str) -> String {
    let clean = strip_comments(xml);
    let mut out = String::new();
    for tok in clean.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(tok);
        if out.len() >= 240 {
            break;
        }
    }
    if out.len() > 240 {
        out.truncate(240);
        out.push('…');
    }
    if out.is_empty() {
        out.push_str("<EMPTY FILE>");
    }
    out
}

/// Expand libvintf version RANGES into every declared endpoint.
///
/// Real semantic: libvintf stores `<version>2.0-2.1</version>` as a
/// VersionRange and answers `getHal(pkg, version)` for ANY version
/// inside it (HalManifest.cpp compareVersion — low <= v <= high on the
/// declared major). Storing the range string verbatim made BOTH
/// endpoint lookups miss (rn327: health@2.1 registration pre-check
/// FAILED client-side → health-hal-2-1 restart-looped 25×).
/// Multi-major ranges expand across majors; the expansion clamps at
/// 32 versions per range so a malformed range cannot flood the index.
fn expand_version_ranges(versions: &[String]) -> Vec<String> {
    const MAX_EXPANSION: usize = 32;
    let mut out = Vec::new();
    for v in versions {
        let range = match v.split_once('-') {
            Some((lo, hi)) => (lo.trim(), hi.trim()),
            None => {
                out.push(v.clone());
                continue;
            }
        };
        let parse = |s: &str| -> Option<(u32, u32)> {
            let (maj, min) = s.split_once('.')?;
            let maj = maj.trim().parse::<u32>().ok()?;
            let min = min.trim().parse::<u32>().ok()?;
            Some((maj, min))
        };
        match (parse(range.0), parse(range.1)) {
            (Some((lmaj, lmin)), Some((hmaj, hmin))) if hmaj >= lmaj => {
                let mut maj = lmaj;
                while maj <= hmaj && out.len() < MAX_EXPANSION {
                    let mlo = if maj == lmaj { lmin } else { 0 };
                    let mhi = if maj == hmaj { hmin } else { 15 };
                    let mut min = mlo;
                    while min <= mhi && out.len() < MAX_EXPANSION {
                        out.push(format!("{}.{}", maj, min));
                        min += 1;
                    }
                    maj += 1;
                }
            }
            _ => out.push(v.clone()),
        }
    }
    out
}

/// Index one manifest `<fqname>` declaration against its `<hal>` package.
///
/// The shipped manifests use `<fqname>` in BOTH formats (the 6-Z372
/// zero-entry skeleton diagnostic made the real shapes visible in the
/// rn328 decode):
/// - AIDL relative:  `<fqname>ILights/default</fqname>` (resolved
///   against `<name>`),
/// - AIDL qualified: `<fqname>android.hardware.rebootescrow.
///   IRebootEscrow/default</fqname>`,
/// - AIDL matrix:    `<fqname>IPower/2/default</fqname>` (a lone
///   numeric middle component is the matrix version — dropped),
/// - HIDL short:     `<fqname>@2.1::IHealth/default</fqname>` (the
///   health@2.1.xml / nn-sample / drm-clearkey spelling — the package
///   is carried by `<name>`, version+interface+instance inline),
/// - HIDL full:      `<fqname>pkg@M.m::IFace/instance</fqname>`.
///
/// Instance components may contain slashes (`ICameraProvider/internal/
/// 0`) — the interface/instance split takes the FIRST '/', and only a
/// lone numeric middle component is treated as a version (AIDL only).
fn insert_fqname_entry(
    fq: &str,
    package: &str,
    aidl: bool,
    transport: u8,
    out: &mut VintfManifests,
) {
    let fq = fq.trim();
    if fq.is_empty() || package.is_empty() {
        return;
    }
    // HIDL fqnames carry `pkg@M.m::IFace/instance`; AIDL fqnames carry
    // no `::`. Split on the first `::` and let the tail parse shared.
    let (head, tail) = match fq.split_once("::") {
        Some((h, t)) => (Some(h.trim()), t),
        None => (None, fq),
    };
    let (pkg, ver): (String, Option<String>) = match head {
        Some(h) => {
            if let Some(at) = h.rfind('@') {
                (
                    h[..at].trim().to_string(),
                    Some(h[at + 1..].trim().to_string()),
                )
            } else if h.chars().all(|c| c.is_ascii_digit() || c == '.') && !h.is_empty() {
                // Version-only head ("2.1::…") — package from <name>.
                (String::new(), Some(h.to_string()))
            } else {
                (h.to_string(), None)
            }
        }
        None => (String::new(), None),
    };
    // HIDL keys need a version; without one the entry is malformed.
    if !aidl && head.is_some() && ver.is_none() {
        return;
    }
    // Interface/instance: instance components may contain '/' — split
    // at the FIRST '/', and drop a lone numeric middle component (the
    // AIDL matrix version spelling).
    let mut parts = tail.split('/');
    let iface = match parts.next() {
        Some(i) if !i.trim().is_empty() => i.trim().to_string(),
        _ => return,
    };
    let rest: Vec<&str> = parts.collect();
    let instance = match rest.len() {
        0 => return,
        1 => rest[0].trim().to_string(),
        2 if aidl
            && !rest[0].trim().is_empty()
            && rest[0].trim().chars().all(|c| c.is_ascii_digit()) =>
        {
            rest[1].trim().to_string()
        }
        _ => rest.join("/").trim().to_string(),
    };
    if instance.is_empty() {
        return;
    }
    if aidl && head.is_none() {
        // AIDL: strip the package prefix on the fully-qualified spelling.
        let prefixed = format!("{}.", package);
        let iface = match iface.strip_prefix(&prefixed) {
            Some(rest) if !rest.is_empty() => rest.to_string(),
            _ => iface,
        };
        if iface.is_empty() {
            return;
        }
        out.insert(
            format!("{}{}::{}/{}", AIDL_KEY_PREFIX, package, iface, instance),
            transport,
        );
        return;
    }
    let pkg = if pkg.is_empty() {
        package.to_string()
    } else {
        pkg
    };
    let ver = match ver {
        Some(v) if !v.is_empty() => v,
        // AIDL-with-head is not a real shape; HIDL without a version was
        // gated above — index nothing rather than a key no client can
        // query.
        _ => return,
    };
    out.insert(
        format!("{}@{}::{}/{}", pkg, ver, iface, instance),
        transport,
    );
}

fn sorted_fragments(rootfs: &str, rel_dir: &str) -> Vec<String> {
    let dir = format!("{}/{}", rootfs.trim_end_matches('/'), rel_dir);
    let mut names: Vec<String> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".xml"))
            .map(|e| format!("{}/{}", rel_dir, e.file_name().to_string_lossy()))
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names
}

/// `fq/instance → transport` index (e.g.
/// `android.hardware.light@2.0::ILight/default → HWBINDER`).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct VintfManifests {
    entries: HashMap<String, u8>,
}

impl VintfManifests {
    /// Answer the transport for `fq`/`instance`, if declared.
    pub fn lookup(&self, fq: &str, instance: &str) -> Option<u8> {
        self.entries.get(&format!("{}/{}", fq, instance)).copied()
    }

    /// Number of indexed `fq/instance` entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when nothing was indexed.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Merge `other` into `self`; existing keys are NOT overwritten
    /// (first-wins — the earlier manifest in consult order prevails).
    fn merge_first_wins(&mut self, other: &VintfManifests) {
        for (k, v) in &other.entries {
            self.entries.entry(k.clone()).or_insert(*v);
        }
    }

    /// Insert (used by the parser and tests).
    fn insert(&mut self, key: String, transport: u8) {
        self.entries.insert(key, transport);
    }
}

/// Parsed `pkg@major.minor::IFoo` service name (FQName's shape).
#[derive(Debug, PartialEq, Eq)]
pub struct FqName {
    /// Manifest `<name>` package part, e.g. `android.hardware.light`.
    pub package: String,
    /// `major.minor` version string, e.g. `2.0`.
    pub version: String,
    /// Interface name after `::`, e.g. `ILight`.
    pub iface: String,
}

/// Parse a fully-qualified HIDL name. Mirrors the gates in
/// hwservicemanager Vintf.cpp:44: an unparsable name, a missing
/// version, or an empty interface name makes the caller answer EMPTY.
pub fn parse_fq(fq: &str) -> Option<FqName> {
    let (package, rest) = fq.split_once('@')?;
    let (version, iface) = rest.split_once("::")?;
    if package.is_empty() || iface.is_empty() {
        return None;
    }
    // FQName versions are `major.minor` — exactly one dot, all digits.
    let mut parts = version.split('.');
    let (major, minor) = (parts.next()?, parts.next()?);
    if parts.next().is_some() || major.is_empty() || minor.is_empty() {
        return None;
    }
    if !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    Some(FqName {
        package: package.to_string(),
        version: version.to_string(),
        iface: iface.to_string(),
    })
}

/// Parse one manifest XML document into the `fq/instance → transport`
/// index. Purpose-built scanner for machine-generated VINTF manifests
/// (libvintf's schema): `<hal>` blocks carry `<name>` (package),
/// `<transport>` (hwbinder|passthrough, versions may be nested),
/// sibling `<version>` entries, and `<interface>` blocks with
/// `<name>`/`<instance>` children. XML comments are stripped; basic
/// entities are decoded; `<regex-instance>` entries are skipped.
pub fn parse_manifest(xml: &str) -> VintfManifests {
    let clean = strip_comments(xml);
    let mut out = VintfManifests::default();
    let mut pos = 0usize;
    while let Some(off) = clean[pos..].find("<hal") {
        let abs = pos + off;
        // `<hal` must not swallow `<halfoo…`; the next char has to be
        // whitespace or the tag end.
        let after = clean[abs + 4..].chars().next();
        if !matches!(after, Some(c) if c.is_whitespace() || c == '>') {
            pos = abs + 4;
            continue;
        }
        let open_end = match clean[abs..].find('>') {
            Some(gt) => abs + gt,
            None => break,
        };
        let open_tag = &clean[abs..=open_end];
        let aidl = open_tag.contains("format=\"aidl\"");
        let close = match clean[open_end..].find("</hal>") {
            Some(c) => open_end + c,
            None => break,
        };
        let body = &clean[open_end + 1..close];
        pos = close + 6;
        parse_hal_block(body, aidl, &mut out);
    }
    out
}

/// Parse one `<hal>…</hal>` body: package, transport kind + versions,
/// interface blocks → one entry per (fq, instance).
fn parse_hal_block(body: &str, aidl: bool, out: &mut VintfManifests) {
    // Split the interface blocks off; everything before the first one
    // holds <name>/<transport>/<version>.
    let head_end = body.find("<interface").unwrap_or(body.len());
    let head = &body[..head_end];
    let package = match child_text(head, "name") {
        Some(p) if !p.is_empty() => p,
        _ => return,
    };

    // Transport kind + versions nested inside <transport …>…</transport>,
    // plus the sibling <version> entries (the modern shape).
    let (kind, mut versions) = parse_transport(head);
    for v in all_child_texts(head, "version") {
        if !v.is_empty() {
            versions.push(v);
        }
    }
    versions.dedup();
    // 6-Z372: version RANGES ("2.0-2.1") must answer for every
    // endpoint — the real HalManifest stores a VersionRange and
    // answers getHal for any version inside it; a verbatim key made
    // BOTH endpoints miss (health-hal-2-1 restart-loop, rn327).
    let versions = expand_version_ranges(&versions);

    let kind = match kind {
        Some(k) => k,
        None => {
            if aidl {
                // AIDL entries declare no <transport>; the wire transport
                // is hwbinder by definition (libvintf HalManifest).
                String::from("hwbinder")
            } else {
                // HIDL entries without a transport are malformed —
                // libvintf rejects the manifest; skip the hal block.
                return;
            }
        }
    };
    let transport = match kind.as_str() {
        "hwbinder" => TRANSPORT_HWBINDER,
        "passthrough" => TRANSPORT_PASSTHROUGH,
        _ => return,
    };

    for iface_block in interface_blocks(&body[head_end..]) {
        let iface = match child_text(iface_block, "name") {
            Some(i) if !i.is_empty() => i,
            _ => continue,
        };
        for inst in all_child_texts(iface_block, "instance") {
            if inst.is_empty() {
                continue;
            }
            if aidl {
                let key = AIDL_KEY_PREFIX.to_string() + &package + "::" + &iface + "/" + &inst;
                out.insert(key, transport);
            } else {
                for ver in &versions {
                    out.insert(
                        format!("{}@{}::{}/{}", package, ver, iface, inst),
                        transport,
                    );
                }
                // <regex-instance> entries are deliberately skipped — no
                // regex engine here; clients take the honest EMPTY path.
            }
        }
    }
    // 6-Z372/6-Z373: the compact `<fqname>` shape. BOTH formats ship it:
    // AIDL hal blocks declare the instance WITHOUT an <interface> block
    // (lights/power/rebootescrow), and the vendored HIDL fragments
    // (health@2.1.xml, the nn samples, drm clearkey/widevine) use the
    // SHORT fqname spelling `@M.m::IFace/instance` with the package
    // carried by <name> — rn328's skeleton lines are the ground truth.
    for fq in all_child_texts(body, "fqname") {
        insert_fqname_entry(&fq, &package, aidl, transport, out);
    }
}

/// `<transport …>…</transport>` → (kind text, versions nested in it).
fn parse_transport(head: &str) -> (Option<String>, Vec<String>) {
    let after_gt = match find_open_tag(head, "transport") {
        Some((_, end)) => end,
        None => return (None, Vec::new()),
    };
    let close = match head[after_gt..].find("</transport>") {
        Some(c) => after_gt + c,
        None => return (None, Vec::new()),
    };
    let inner = &head[after_gt..close];
    // Kind = text before the first nested tag (or the whole inner text).
    let text_end = inner.find('<').unwrap_or(inner.len());
    let kind_text = decode_entities(inner[..text_end].trim());
    let kind = if kind_text.is_empty() {
        None
    } else {
        Some(kind_text)
    };
    // Nested <version> tags inside <transport> (legacy shape).
    let mut versions = Vec::new();
    let mut pos = 0usize;
    while let Some(off) = inner[pos..].find("<version>") {
        let s = pos + off + "<version>".len();
        let end = match inner[s..].find("</version>") {
            Some(e) => s + e,
            None => break,
        };
        let v = inner[s..end].trim().to_string();
        if !v.is_empty() {
            versions.push(v);
        }
        pos = end;
    }
    (kind, versions)
}

/// Iterate non-nested `<interface>…</interface>` blocks.
fn interface_blocks(scope: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while let Some(off) = scope[pos..].find("<interface") {
        let abs = pos + off;
        let after = scope[abs + 10..].chars().next();
        if !matches!(after, Some(c) if c.is_whitespace() || c == '>') {
            pos = abs + 10;
            continue;
        }
        let open_end = match scope[abs..].find('>') {
            Some(gt) => abs + gt,
            None => break,
        };
        let close = match scope[open_end..].find("</interface>") {
            Some(c) => open_end + c,
            None => break,
        };
        out.push(&scope[open_end + 1..close]);
        pos = close + 12;
    }
    out
}

/// First `<tag>text</tag>` child text in `scope` (trimmed, decoded).
fn child_text(scope: &str, tag: &str) -> Option<String> {
    all_child_texts(scope, tag).into_iter().next()
}

/// Every `<tag>text</tag>` child text in `scope`, in document order.
fn all_child_texts(scope: &str, tag: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    let open_pat = format!("<{}", tag);
    let close_pat = format!("</{}>", tag);
    while let Some(off) = scope[pos..].find(&open_pat) {
        let abs = pos + off;
        let after = scope[abs + open_pat.len()..].chars().next();
        if !matches!(after, Some(c) if c.is_whitespace() || c == '>') {
            pos = abs + open_pat.len();
            continue;
        }
        let open_end = match scope[abs..].find('>') {
            Some(gt) => abs + gt,
            None => break,
        };
        let close = match scope[open_end..].find(&close_pat) {
            Some(c) => open_end + c,
            None => break,
        };
        let text = decode_entities(scope[open_end + 1..close].trim());
        out.push(text);
        pos = close + close_pat.len();
    }
    out
}

/// Locate `<tag…>` in `scope`; returns (tag start, index after '>').
fn find_open_tag(scope: &str, tag: &str) -> Option<(usize, usize)> {
    let open_pat = format!("<{}", tag);
    let mut pos = 0usize;
    while let Some(off) = scope[pos..].find(&open_pat) {
        let abs = pos + off;
        let after = scope[abs + open_pat.len()..].chars().next();
        if !matches!(after, Some(c) if c.is_whitespace() || c == '>') {
            pos = abs + open_pat.len();
            continue;
        }
        let gt = scope[abs..].find('>')?;
        return Some((abs, abs + gt + 1));
    }
    None
}

/// Strip `<!-- … -->` comments.
fn strip_comments(xml: &str) -> String {
    if !xml.contains("<!--") {
        return xml.to_string();
    }
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 4..];
        match after.find("-->") {
            Some(end) => rest = &after[end + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// Decode the entities libvintf's XML parser would resolve.
fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let semi_rel = match tail.find(';') {
            Some(sc) if sc <= 10 => Some(sc),
            _ => None,
        };
        let entity = semi_rel.map(|sc| &tail[1..sc]);
        let decoded = match entity {
            Some("amp") => Some('&'),
            Some("lt") => Some('<'),
            Some("gt") => Some('>'),
            Some("quot") => Some('"'),
            Some("apos") => Some('\''),
            Some(other) if other.starts_with("#x") || other.starts_with("#X") => {
                u32::from_str_radix(&other[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
            }
            Some(other) if other.starts_with('#') => {
                other[1..].parse::<u32>().ok().and_then(char::from_u32)
            }
            _ => None,
        };
        match (decoded, semi_rel) {
            (Some(c), Some(sc)) => {
                out.push(c);
                rest = &rest[amp + sc + 1..];
            }
            _ => {
                out.push('&');
                rest = &rest[amp + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fq_valid_and_invalid() {
        let fq = parse_fq("android.hardware.light@2.0::ILight").expect("parses");
        assert_eq!(fq.package, "android.hardware.light");
        assert_eq!(fq.version, "2.0");
        assert_eq!(fq.iface, "ILight");
        // Vintf.cpp gates: no version / no iface / bad version → None.
        assert!(parse_fq("android.hardware.light::ILight").is_none());
        assert!(parse_fq("android.hardware.light@2.0::").is_none());
        assert!(parse_fq("android.hardware.light@2::ILight").is_none());
        assert!(parse_fq("android.hardware.light@2.0.1::ILight").is_none());
        assert!(parse_fq("android.hardware.light@a.b::ILight").is_none());
        assert!(parse_fq("@2.0::ILight").is_none());
    }

    #[test]
    fn hidl_sibling_versions_and_interfaces() {
        let xml = r#"
<manifest version="6.0" type="framework">
    <hal format="hidl">
        <name>android.hardware.foo</name>
        <transport>hwbinder</transport>
        <version>1.0</version>
        <version>2.0</version>
        <interface>
            <name>IFoo</name>
            <instance>default</instance>
            <instance>slot1</instance>
        </interface>
        <interface>
            <name>IFooAlt</name>
            <instance>legacy</instance>
        </interface>
    </hal>
</manifest>
"#;
        let m = parse_manifest(xml);
        assert_eq!(
            m.lookup("android.hardware.foo@1.0::IFoo", "default"),
            Some(TRANSPORT_HWBINDER)
        );
        assert_eq!(
            m.lookup("android.hardware.foo@2.0::IFoo", "slot1"),
            Some(TRANSPORT_HWBINDER)
        );
        assert_eq!(
            m.lookup("android.hardware.foo@2.0::IFooAlt", "legacy"),
            Some(TRANSPORT_HWBINDER)
        );
        assert_eq!(m.lookup("android.hardware.foo@3.0::IFoo", "default"), None);
        assert_eq!(
            m.lookup("android.hardware.foo@1.0::INotThere", "default"),
            None
        );
    }

    // ── 6-Z372: the rn327 restart-loop shapes ────────────────────────

    #[test]
    fn version_range_answers_every_endpoint() {
        // The health-hal-2-1 class: a RANGE declaration must answer for
        // both endpoints (libvintf VersionRange semantics). Storing the
        // range verbatim made getTransport(2.1) → EMPTY → the client-side
        // registerAsServiceInternal pre-check refused the registration
        // and the HAL CHECK-aborted into an init restart-loop.
        let xml = r#"
<manifest version="1.0" type="device">
    <hal format="hidl">
        <name>android.hardware.health</name>
        <transport>hwbinder</transport>
        <version>2.0-2.1</version>
        <interface>
            <name>IHealth</name>
            <instance>default</instance>
        </interface>
    </hal>
</manifest>
"#;
        let m = parse_manifest(xml);
        assert_eq!(
            m.lookup("android.hardware.health@2.0::IHealth", "default"),
            Some(TRANSPORT_HWBINDER),
            "range low endpoint must answer"
        );
        assert_eq!(
            m.lookup("android.hardware.health@2.1::IHealth", "default"),
            Some(TRANSPORT_HWBINDER),
            "range high endpoint must answer"
        );
        assert_eq!(
            m.lookup("android.hardware.health@3.0::IHealth", "default"),
            None,
            "versions outside the range stay EMPTY"
        );
        // Sibling versions still work unchanged.
        let m2 = parse_manifest(
            r#"<hal format="hidl"><name>p</name><transport>hwbinder</transport>
               <version>1.0</version><version>2.0</version>
               <interface><name>I</name><instance>d</instance></interface></hal>"#,
        );
        assert_eq!(m2.lookup("p@1.0::I", "d"), Some(TRANSPORT_HWBINDER));
        assert_eq!(m2.lookup("p@2.0::I", "d"), Some(TRANSPORT_HWBINDER));
    }

    #[test]
    fn aidl_fqname_relative_and_fully_qualified() {
        // The lights/power/rebootescrow class: the compact AIDL shape
        // declares the instance via <fqname> with NO <interface> block.
        let xml = r#"
<manifest version="1.0" type="device">
    <hal format="aidl">
        <name>android.hardware.light</name>
        <fqname>ILights/default</fqname>
    </hal>
    <hal format="aidl">
        <name>android.hardware.rebootescrow</name>
        <fqname>android.hardware.rebootescrow.IRebootEscrow/default</fqname>
    </hal>
    <hal format="aidl">
        <name>android.hardware.power</name>
        <fqname>IPower/2/default</fqname>
    </hal>
</manifest>
"#;
        let m = parse_manifest(xml);
        assert_eq!(
            m.lookup("aidl-never-matches-hidl", "x"),
            None,
            "aidl keys are namespaced away from HIDL lookups"
        );
        let entries = m.len();
        assert!(
            entries >= 3,
            "all three fqname hal blocks must index: got {}",
            entries
        );
        // The AIDL keys use the aidl: prefix (same scheme the existing
        // interface/instance AIDL arm uses).
        let key = format!("{}android.hardware.light::ILights/default", AIDL_KEY_PREFIX);
        assert!(
            m.entries.contains_key(&key),
            "relative fqname must index as aidl:pkg::IFace/instance"
        );
        let key2 = format!(
            "{}android.hardware.rebootescrow::IRebootEscrow/default",
            AIDL_KEY_PREFIX
        );
        assert!(
            m.entries.contains_key(&key2),
            "fully-qualified fqname must strip the package prefix"
        );
        let key3 = format!("{}android.hardware.power::IPower/default", AIDL_KEY_PREFIX);
        assert!(
            m.entries.contains_key(&key3),
            "matrix spelling (IFace/version/instance) drops the version"
        );
    }

    #[test]
    fn version_range_expansion_caps_and_passthrough() {
        let v = expand_version_ranges(&["2.0".to_string(), "1.0-1.2".to_string()]);
        assert_eq!(v, vec!["2.0", "1.0", "1.1", "1.2"]);
        // Malformed ranges pass through verbatim (honest EMPTY path).
        let v2 = expand_version_ranges(&["x-y".to_string(), "2.0".to_string()]);
        assert_eq!(v2, vec!["x-y", "2.0"]);
        // Reverse ranges pass through (hmaj >= lmaj gate).
        let v3 = expand_version_ranges(&["3.0-2.0".to_string()]);
        assert_eq!(v3, vec!["3.0-2.0"]);
    }

    #[test]
    fn xml_skeleton_bounded_and_names_empty() {
        assert_eq!(xml_skeleton(""), "<EMPTY FILE>");
        assert_eq!(xml_skeleton("   \n\t "), "<EMPTY FILE>");
        let long: String = "x".repeat(5000);
        let sk = xml_skeleton(&long);
        assert!(sk.chars().count() <= 242, "skeleton must stay bounded");
        let cmt = "<manifest><!-- only a comment --></manifest>";
        assert_eq!(xml_skeleton(cmt), "<manifest></manifest>");
    }

    #[test]
    fn hidl_short_fqname_from_rn328_skeletons() {
        // The rn328 ground truth: the vendored HIDL fragments declare
        // `<fqname>@2.1::IHealth/default</fqname>` — version inline,
        // package carried by <name>. These EXACTLY reproduced the
        // health/drm/nn restart-loops after 6-Z372 (the AIDL-only
        // fqname arm skipped them).
        let xml = r#"
<manifest version="1.0" type="device">
    <hal format="hidl">
        <name>android.hardware.health</name>
        <transport>hwbinder</transport>
        <fqname>@2.1::IHealth/default</fqname>
    </hal>
    <hal format="hidl">
        <name>android.hardware.drm</name>
        <transport>hwbinder</transport>
        <fqname>@1.3::ICryptoFactory/clearkey</fqname>
        <fqname>@1.3::IDrmFactory/clearkey</fqname>
    </hal>
    <hal format="hidl">
        <name>android.hardware.neuralnetworks</name>
        <transport>hwbinder</transport>
        <fqname>@1.3::IDevice/nnapi-sample_all</fqname>
    </hal>
    <hal format="hidl">
        <name>android.hardware.camera.provider</name>
        <transport>hwbinder</transport>
        <fqname>@2.6::ICameraProvider/internal/0</fqname>
    </hal>
    <hal format="hidl">
        <name>android.hardware.full</name>
        <transport>hwbinder</transport>
        <fqname>android.hardware.full@2.0::IFull/default</fqname>
    </hal>
</manifest>
"#;
        let m = parse_manifest(xml);
        assert_eq!(
            m.lookup("android.hardware.health@2.1::IHealth", "default"),
            Some(TRANSPORT_HWBINDER),
            "health@2.1 short fqname must register"
        );
        assert_eq!(
            m.lookup("android.hardware.drm@1.3::IDrmFactory", "clearkey"),
            Some(TRANSPORT_HWBINDER),
            "drm clearkey short fqname must register"
        );
        assert_eq!(
            m.lookup("android.hardware.drm@1.3::ICryptoFactory", "clearkey"),
            Some(TRANSPORT_HWBINDER),
            "both drm fqname entries must register"
        );
        assert_eq!(
            m.lookup(
                "android.hardware.neuralnetworks@1.3::IDevice",
                "nnapi-sample_all"
            ),
            Some(TRANSPORT_HWBINDER),
            "nn sample short fqname must register"
        );
        assert_eq!(
            m.lookup(
                "android.hardware.camera.provider@2.6::ICameraProvider",
                "internal/0"
            ),
            Some(TRANSPORT_HWBINDER),
            "slash-bearing HIDL instances keep the full instance text"
        );
        assert_eq!(
            m.lookup("android.hardware.full@2.0::IFull", "default"),
            Some(TRANSPORT_HWBINDER),
            "fully-qualified HIDL fqname registers under its own package"
        );
    }

    #[test]
    fn transport_nested_versions_and_arch_attr() {
        let xml = r#"
<manifest type="device">
    <hal format="hidl">
        <name>android.hardware.bar</name>
        <transport arch="32+64">hwbinder<version>1.1</version></transport>
        <interface>
            <name>IBar</name>
            <instance>default</instance>
        </interface>
    </hal>
</manifest>
"#;
        let m = parse_manifest(xml);
        assert_eq!(
            m.lookup("android.hardware.bar@1.1::IBar", "default"),
            Some(TRANSPORT_HWBINDER)
        );
        assert_eq!(m.lookup("android.hardware.bar@1.0::IBar", "default"), None);
    }

    #[test]
    fn passthrough_transport_answered() {
        let xml = r#"
<manifest type="device">
    <hal format="hidl">
        <name>android.hardware.baz</name>
        <transport arch="32">passthrough</transport>
        <version>2.1</version>
        <interface>
            <name>IBaz</name>
            <instance>default</instance>
        </interface>
    </hal>
</manifest>
"#;
        let m = parse_manifest(xml);
        assert_eq!(
            m.lookup("android.hardware.baz@2.1::IBaz", "default"),
            Some(TRANSPORT_PASSTHROUGH)
        );
    }

    #[test]
    fn comments_stripped_and_entities_decoded() {
        let xml = r#"
<manifest type="device">
    <!-- <hal><name>android.hardware.ghost</name></hal> -->
    <hal format="hidl">
        <name>android.hardware.q&amp;x</name>
        <transport>hwbinder</transport>
        <version>1.0</version>
        <interface>
            <name>IQ</name>
            <instance>de&lt;fault&gt;</instance>
        </interface>
    </hal>
</manifest>
"#;
        let m = parse_manifest(xml);
        assert_eq!(
            m.lookup("android.hardware.ghost@1.0::IGhost", "default"),
            None
        );
        assert_eq!(
            m.lookup("android.hardware.q&x@1.0::IQ", "de<fault>"),
            Some(TRANSPORT_HWBINDER)
        );
    }

    #[test]
    fn framework_group_wins_over_device_group() {
        let framework = parse_manifest(
            r#"<manifest type="framework"><hal format="hidl">
               <name>android.hardware.duel</name>
               <transport>passthrough</transport>
               <version>1.0</version>
               <interface><name>IDuel</name><instance>default</instance></interface>
               </hal></manifest>"#,
        );
        let device = parse_manifest(
            r#"<manifest type="device"><hal format="hidl">
               <name>android.hardware.duel</name>
               <transport>hwbinder</transport>
               <version>1.0</version>
               <interface><name>IDuel</name><instance>default</instance></interface>
               </hal></manifest>"#,
        );
        let mut merged = VintfManifests::default();
        merged.merge_first_wins(&framework);
        merged.merge_first_wins(&device);
        // Vintf.cpp consults the framework manifest FIRST.
        assert_eq!(
            merged.lookup("android.hardware.duel@1.0::IDuel", "default"),
            Some(TRANSPORT_PASSTHROUGH)
        );
    }

    #[test]
    fn aidl_entries_indexed_but_never_hit_by_hidl_lookup() {
        let xml = r#"
<manifest type="device">
    <hal format="aidl">
        <name>android.hardware.health</name>
        <interface>
            <name>IHealth</name>
            <instance>default</instance>
        </interface>
    </hal>
</manifest>
"#;
        let m = parse_manifest(xml);
        assert_eq!(m.len(), 1);
        // HIDL fqs always carry @ver:: — they cannot reach AIDL keys.
        assert_eq!(
            m.lookup("android.hardware.health@2.1::IHealth", "default"),
            None
        );
        assert_eq!(
            m.lookup("android.hardware.health::IHealth", "default"),
            None
        );
    }

    #[test]
    fn regex_instance_skipped_and_malformed_hal_skipped() {
        let xml = r#"
<manifest type="device">
    <hal format="hidl">
        <name>android.hardware.regexed</name>
        <transport>hwbinder</transport>
        <version>1.0</version>
        <interface>
            <name>IRegex</name>
            <regex-instance>slot[0-9]+</regex-instance>
        </interface>
    </hal>
    <hal format="hidl">
        <name>android.hardware.notransport</name>
        <version>1.0</version>
        <interface><name>INo</name><instance>default</instance></interface>
    </hal>
</manifest>
"#;
        let m = parse_manifest(xml);
        assert!(m.is_empty());
    }

    #[test]
    fn legacy_vendor_manifest_shape_parses() {
        // Pre-Treble-style /vendor/manifest.xml: <hal> without format
        // attribute (defaults to hidl), sibling <version>.
        let xml = r#"
<manifest version="1.0" type="device">
    <hal>
        <name>android.hardware.legacy</name>
        <transport>hwbinder</transport>
        <version>1.0</version>
        <interface><name>ILegacy</name><instance>default</instance></interface>
    </hal>
</manifest>
"#;
        let m = parse_manifest(xml);
        assert_eq!(
            m.lookup("android.hardware.legacy@1.0::ILegacy", "default"),
            Some(TRANSPORT_HWBINDER)
        );
    }

    #[test]
    fn empty_and_absent_manifests() {
        assert!(parse_manifest("").is_empty());
        assert!(parse_manifest("<manifest></manifest>").is_empty());
        // Unclosed hal block must not loop forever.
        assert!(parse_manifest("<manifest><hal format=\"hidl\"><name>x</name>").is_empty());
    }

    #[test]
    fn lookup_requires_rootfs_or_answers_none() {
        // Unit tests never call set_rootfs() — the global stays unset
        // and lookup() must answer None without touching the fs.
        assert!(lookup("android.hardware.anything@1.0::IFoo", "default").is_none());
    }
}
