// Symlink-sidecar materialization + TWRP terminal shell provisioning.
//
// WHY THIS EXISTS (6-Z187, user report "MAKE IT ONLY SHOW GUESTS ONLY AND
// NOTHING ELSE" + "fix terminal with that 'Child processes exit.' Errors"):
//
// The RamdiskImporter cannot create real symlinks from Java's File API, so
// every cpio symlink entry lands in the rootfs as a `<name>.symlink` TEXT
// sidecar holding the target. Consequences observed in run 33114902086:
//
//   1. TWRP's terminal (gui/terminal.cpp runSlave) does
//      `execl("/sbin/sh", "sh", NULL); _exit(127);` — {rootfs}/sbin/sh
//      does NOT exist (only sh.symlink) → execve ENOENT → the child
//      exits 127 → the GUI prints "Child processes exited." and the
//      terminal is dead.
//   2. The File Manager root shows implementation artifacts
//      (charger.symlink, …) instead of the guest's real tree.
//   3. init's shebang services (/sbin/permissive.sh,
//      /sbin/pulldecryptfiles.sh) cannot exec either — their interpreter
//      /sbin/sh is the same missing symlink (exit 127 noise in recovery.log).
//
// THE FIX (host side, at boot, before any guest child is forked):
//
//   A. Patch {rootfs}/sbin/busybox's PT_INTERP in place to the absolute
//      host path {rootfs}/sbin/linker64 (same treatment recovery gets in
//      lib.rs 6-Z50/6-Z65) so any staged copy of it is self-contained.
//   B. Pre-stage busybox into {data_dir}/cache/twoyi_stage under BOTH the
//      "/sbin/busybox" and "/sbin/sh" guest keys (the noexec app-data
//      partition cannot exec the rootfs copy; the cache staging dir can —
//      ptrace_emu's 6-Z102 engine does exactly this lazily; we do it
//      eagerly so the map hit is instant AND so the symlink below has a
//      valid target even when the tracer is PEEK-blind).
//   C. Walk the rootfs and materialize EVERY `<name>.symlink` sidecar as a
//      REAL symlink (std::os::unix::fs::symlink needs no privileges on the
//      app-owned rootfs):
//        - target that resolves to the pre-staged busybox → point at the
//          STAGED copy (executable; argv[0] dispatch keeps busybox
//          multi-call applet semantics),
//        - absolute target "/X" → point at {rootfs}/X (kernel resolution
//          of the symlink from inside the sandbox then lands back in the
//          rootfs, and the tracer's backstop canonicalization sees it
//          under the rootfs),
//        - relative target → kept relative (resolves within the rootfs).
//        The sidecar file is removed after materialization so the guest
//        File Manager shows the REAL guest tree — guest only, nothing else.
//
// The walk is bounded (entry cap, depth cap, proc/sys/dev subtrees
// skipped) so a full Android ROM rootfs cannot wedge boot.

use std::collections::VecDeque;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Maximum entries visited by the sidecar walk (full ROM rootfs safety).
const WALK_MAX_ENTRIES: usize = 200_000;
/// Maximum directory depth for the walk.
const WALK_MAX_DEPTH: usize = 16;
/// Subtrees that never contain importable symlinks (they are runtime stubs).
const WALK_SKIP_DIRS: &[&str] = &["proc", "sys", "dev"];

/// Patch an ELF's PT_INTERP string to `new_interp` (absolute host path),
/// in place when it fits, else append + phdr update. Class-aware (ELF32 +
/// ELF64). Mirrors lib.rs 6-Z50/6-Z65 for the recovery binary, factored
/// here so busybox gets the identical treatment.
pub fn patch_elf_interp(file_path: &Path, new_interp: &str) -> Result<(), String> {
    use std::io::{Seek, SeekFrom, Write};
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(file_path)
        .map_err(|e| format!("open {}: {}", file_path.display(), e))?;
    let mut ehdr = [0u8; 64];
    file.read_exact(&mut ehdr)
        .map_err(|e| format!("read ehdr {}: {}", file_path.display(), e))?;
    if &ehdr[0..4] != b"\x7fELF" {
        return Err("not an ELF".to_string());
    }
    let is64 = ehdr[4] == 2;
    let (e_phoff, e_phentsize, e_phnum): (u64, usize, usize) = if is64 {
        (
            u64::from_le_bytes(ehdr[32..40].try_into().unwrap()),
            u16::from_le_bytes(ehdr[54..56].try_into().unwrap()) as usize,
            u16::from_le_bytes(ehdr[56..58].try_into().unwrap()) as usize,
        )
    } else {
        (
            u32::from_le_bytes(ehdr[28..32].try_into().unwrap()) as u64,
            u16::from_le_bytes(ehdr[42..44].try_into().unwrap()) as usize,
            u16::from_le_bytes(ehdr[44..46].try_into().unwrap()) as usize,
        )
    };
    let (p_off_field, p_sz_field, sz_bytes): (u64, u64, usize) =
        if is64 { (8, 32, 8) } else { (4, 16, 4) };
    let min_phentsize: usize = if is64 { 56 } else { 32 };
    let file_len = file.metadata().map(|m| m.len()).unwrap_or(u64::MAX);
    if e_phnum == 0
        || e_phentsize < min_phentsize
        || e_phentsize > 4096
        || e_phoff
            .checked_add((e_phentsize * e_phnum) as u64)
            .map_or(true, |end| end > file_len)
    {
        return Err(format!(
            "implausible phdr table (phoff={}, phentsize={}, phnum={}, len={})",
            e_phoff, e_phentsize, e_phnum, file_len
        ));
    }
    let mut phdrs = vec![0u8; e_phentsize * e_phnum];
    file.seek(SeekFrom::Start(e_phoff)).ok();
    file.read_exact(&mut phdrs)
        .map_err(|e| format!("read phdrs: {}", e))?;
    let mut interp_off = None;
    let mut interp_sz = None;
    for i in 0..e_phnum {
        let off = i * e_phentsize;
        let p_type = u32::from_le_bytes(phdrs[off..off + 4].try_into().unwrap());
        if p_type == 3 {
            let read_u = |base: usize, field: u64, width: usize| -> u64 {
                let s = base + field as usize;
                let mut v = 0u64;
                for b in (0..width).rev() {
                    v = (v << 8) | phdrs[s + b] as u64;
                }
                v
            };
            interp_off = Some(read_u(off, p_off_field, sz_bytes));
            interp_sz = Some(read_u(off, p_sz_field, sz_bytes) as usize);
            break;
        }
    }
    let (p_offset, p_filesz) = match (interp_off, interp_sz) {
        (Some(o), Some(s)) => (o, s),
        _ => return Err("no PT_INTERP (static binary — nothing to patch)".to_string()),
    };
    let new_bytes = format!("{}\0", new_interp).into_bytes();
    if new_bytes.len() <= p_filesz {
        let mut nb = new_bytes.clone();
        while nb.len() < p_filesz {
            nb.push(0);
        }
        file.seek(SeekFrom::Start(p_offset)).ok();
        file.write_all(&nb)
            .map_err(|e| format!("in-place interp write: {}", e))?;
        return Ok(());
    }
    // Append path: grow the file, point the phdr at the tail.
    let end = file.seek(SeekFrom::End(0)).unwrap_or(0);
    let new_offset = (end + 7) & !7u64;
    file.seek(SeekFrom::Start(new_offset)).ok();
    file.write_all(&new_bytes)
        .map_err(|e| format!("append interp write: {}", e))?;
    for i in 0..e_phnum {
        let off = i * e_phentsize;
        let p_type = u32::from_le_bytes(phdrs[off..off + 4].try_into().unwrap());
        if p_type == 3 {
            let phdr_off = e_phoff + off as u64;
            let mut wr = |field_off: u64, width: usize, val: u64| -> std::io::Result<()> {
                file.seek(SeekFrom::Start(phdr_off + field_off)).ok();
                file.write_all(&val.to_le_bytes()[..width])
            };
            wr(p_off_field, sz_bytes, new_offset)
                .map_err(|e| format!("phdr p_offset write: {}", e))?;
            wr(p_sz_field, sz_bytes, new_bytes.len() as u64)
                .map_err(|e| format!("phdr p_filesz write: {}", e))?;
            return Ok(());
        }
    }
    Err("PT_INTERP phdr vanished during append".to_string())
}

/// Lexically normalize a guest path ("/a/../b", "//x" …) without touching
/// the filesystem. Returns None when the path climbs above "/" (nothing
/// sensible to link to — the sidecar stays).
pub fn normalize_guest_path(path: &str) -> Option<String> {
    if !path.starts_with('/') {
        return None;
    }
    let mut stack: Vec<&str> = Vec::new();
    for comp in path.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                if stack.pop().is_none() {
                    return None;
                }
            }
            other => stack.push(other),
        }
    }
    Some(format!("/{}", stack.join("/")))
}

/// Resolve a sidecar target the way the GUEST kernel would:
/// `dir` is the guest directory containing the link, `target` is the raw
/// sidecar content. Returns the normalized absolute GUEST path.
pub fn resolve_guest_target(dir: &str, target: &str) -> Option<String> {
    let target = target.trim_end_matches(['\0', '\n', '\r']);
    if target.is_empty() {
        return None;
    }
    if target.starts_with('/') {
        normalize_guest_path(target)
    } else {
        normalize_guest_path(&format!("{}/{}", dir, target))
    }
}

/// Result summary of a materialization run (for the boot log line).
#[derive(Debug, Default, Clone, Copy)]
pub struct SymlinkStats {
    pub sidecars_seen: usize,
    pub links_created: usize,
    pub sidecars_removed: usize,
    pub skipped: usize,
}

/// Stage `guest_path`'s rootfs file into the exec cache dir, registering
/// it in the .twoyi-staged marker map under `register_key` (which may
/// differ from guest_path — "/sbin/sh" must map to the busybox copy).
/// Returns the staged cache path. Pure-host helper; no tracer state.
pub fn prestage_executable(
    rootfs: &str,
    data_dir: &str,
    guest_path: &str,
    register_key: &str,
) -> Result<String, String> {
    let rootfs_p = if rootfs.ends_with('/') {
        rootfs.to_string()
    } else {
        format!("{}/", rootfs)
    };
    let src = format!("{}{}", rootfs_p, guest_path.trim_start_matches('/'));
    let bytes = std::fs::read(&src).map_err(|e| format!("read {}: {}", src, e))?;
    if bytes.len() < 4 || bytes[0..4] != [0x7f, b'E', b'L', b'F'] {
        return Err(format!("{} is not ELF", src));
    }
    let stage_dir = format!("{}/cache/twoyi_stage", data_dir);
    std::fs::create_dir_all(&stage_dir).map_err(|e| format!("mkdir {}: {}", stage_dir, e))?;
    // Same deterministic cache filename the 6-Z102 engine derives for the
    // REGISTER key, so a later lazy stage of the same key REUSES this copy.
    let stem: String = register_key
        .chars()
        .take(64)
        .map(|c| {
            if c == '/' || c == '\0' || c == '\n' || c == '\r' || c == '\t' || c == ' ' {
                '_'
            } else {
                c
            }
        })
        .collect();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in register_key.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    let cache_path = format!("{}/{}_{:012x}", stage_dir, stem, h & 0xFFFF_FFFF_FFFF);
    std::fs::write(&cache_path, &bytes).map_err(|e| format!("write {}: {}", cache_path, e))?;
    std::fs::set_permissions(&cache_path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("chmod {}: {}", cache_path, e))?;
    // Register in the marker (RMW, keyed replace — mirrors
    // ptrace_emu::append_staged_marker but keyed at {data_dir}/cache).
    let marker = format!("{}/cache/twoyi-staged", data_dir);
    let existing = std::fs::read_to_string(&marker).unwrap_or_default();
    let new_line = format!("{}\t{}", register_key, cache_path);
    let mut out: Vec<String> = Vec::new();
    let mut replaced = false;
    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            out.push(line.to_string());
            continue;
        }
        if trimmed.split('\t').next().unwrap_or("") == register_key {
            out.push(new_line.clone());
            replaced = true;
        } else {
            out.push(line.to_string());
        }
    }
    if !replaced {
        out.push(new_line);
    }
    std::fs::write(&marker, out.join("\n") + "\n")
        .map_err(|e| format!("marker write {}: {}", marker, e))?;
    Ok(cache_path)
}

/// The full boot-time provisioning: PT_INTERP patch + pre-stage busybox
/// under both keys. Returns the staged busybox cache path on success.
pub fn provision_terminal_shell(rootfs: &str, data_dir: &str) -> Result<String, String> {
    let busybox = format!("{}/sbin/busybox", rootfs);
    let linker64 = format!("{}/sbin/linker64", rootfs);
    // A. patch PT_INTERP (idempotent: already-patched files match).
    match patch_elf_interp(Path::new(&busybox), &linker64) {
        Ok(()) => {}
        // Static busybox (some TWRP builds) needs no patch.
        Err(e) if e.contains("no PT_INTERP") => {}
        Err(e) if e.contains("not an ELF") => {
            return Err(format!("{}: {}", busybox, e));
        }
        Err(e) => {
            // Non-fatal: the 6-Z102 lazy stager would hit the same wall,
            // but a boot regression is worse than a dead terminal.
            crate::warning!(
                "[KR64][symlinks] busybox PT_INTERP patch failed: {} (continuing)",
                e
            );
        }
    }
    // B. pre-stage under both guest keys.
    let staged_busybox = prestage_executable(rootfs, data_dir, "/sbin/busybox", "/sbin/busybox")?;
    let _ = prestage_executable(rootfs, data_dir, "/sbin/busybox", "/sbin/sh");
    Ok(staged_busybox)
}

/// Materialize all `.symlink` sidecars under the rootfs as REAL symlinks.
/// `staged_busybox` — when Some, every link whose resolved target is
/// {rootfs}/sbin/busybox points at the staged (executable) copy instead.
pub fn materialize_symlink_sidecars(rootfs: &str, staged_busybox: Option<&str>) -> SymlinkStats {
    let rootfs_p = if rootfs.ends_with('/') {
        rootfs.to_string()
    } else {
        format!("{}/", rootfs)
    };
    let mut stats = SymlinkStats::default();
    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
    queue.push_back((PathBuf::from(rootfs), 0));
    while let Some((dir, depth)) = queue.pop_front() {
        if stats.sidecars_seen + stats.links_created > WALK_MAX_ENTRIES || depth > WALK_MAX_DEPTH {
            continue;
        }
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => {
                stats.skipped += 1;
                continue;
            }
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir {
                if depth == 0 && WALK_SKIP_DIRS.contains(&name.as_str()) {
                    continue;
                }
                queue.push_back((path, depth + 1));
                continue;
            }
            if !name.ends_with(".symlink") {
                continue;
            }
            stats.sidecars_seen += 1;
            let target = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(_) => {
                    stats.skipped += 1;
                    continue;
                }
            };
            // Guest dir of the link (rootfs-relative, "" at the root).
            let parent_rel = path
                .parent()
                .and_then(|p| p.to_str())
                .and_then(|p| p.strip_prefix(rootfs_p.as_str()))
                .unwrap_or("")
                .trim_matches('/')
                .to_string();
            let Some(guest_target) = resolve_guest_target(&format!("/{}", parent_rel), &target)
            else {
                stats.skipped += 1;
                continue;
            };
            // Host destination for the real symlink.
            let host_target: String = match staged_busybox {
                Some(sb) if guest_target == "/sbin/busybox" => sb.to_string(),
                _ => format!("{}{}", rootfs_p, guest_target.trim_start_matches('/')),
            };
            let link_path = path.with_file_name(name.trim_end_matches(".symlink"));
            // Remove whatever occupies the destination (a previous
            // materialization, or an importer-created regular file).
            let _ = std::fs::remove_file(&link_path);
            if std::os::unix::fs::symlink(&host_target, &link_path).is_ok() {
                stats.links_created += 1;
                if std::fs::remove_file(&path).is_ok() {
                    stats.sidecars_removed += 1;
                }
            } else {
                stats.skipped += 1;
            }
        }
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_basic() {
        assert_eq!(
            normalize_guest_path("/sbin/busybox").as_deref(),
            Some("/sbin/busybox")
        );
        assert_eq!(normalize_guest_path("//a//b/").as_deref(), Some("/a/b"));
        assert_eq!(normalize_guest_path("/a/../b").as_deref(), Some("/b"));
        assert_eq!(normalize_guest_path("/../etc"), None);
        assert_eq!(normalize_guest_path("relative"), None);
    }

    #[test]
    fn resolve_absolute_and_relative_targets() {
        assert_eq!(
            resolve_guest_target("/sbin", "/sbin/busybox").as_deref(),
            Some("/sbin/busybox")
        );
        assert_eq!(
            resolve_guest_target("/sbin", "busybox").as_deref(),
            Some("/sbin/busybox")
        );
        assert_eq!(
            resolve_guest_target("/sbin/etc/terminfo", "../../sh").as_deref(),
            Some("/sbin/sh")
        );
        assert_eq!(
            resolve_guest_target("/", "/sbin/healthd").as_deref(),
            Some("/sbin/healthd")
        );
        assert_eq!(resolve_guest_target("/x", ""), None);
    }

    #[test]
    fn materialize_creates_real_symlinks_and_removes_sidecars() {
        let tmp = TempGuard(tempdir_for_test());
        let root = tmp.0.to_str().unwrap().to_string();
        std::fs::create_dir_all(format!("{}/sbin", root)).unwrap();
        std::fs::write(
            format!("{}/sbin/busybox", root),
            b"\x7fELFstatic-placeholder",
        )
        .unwrap();
        // sbin/sh -> /sbin/busybox (absolute)
        std::fs::write(format!("{}/sbin/sh.symlink", root), "/sbin/busybox").unwrap();
        // sbin/cat -> busybox (relative)
        std::fs::write(format!("{}/sbin/cat.symlink", root), "busybox").unwrap();
        // charger -> /sbin/healthd (absolute, target absent)
        std::fs::write(format!("{}/charger.symlink", root), "/sbin/healthd").unwrap();
        // stub subtree that must be skipped
        std::fs::create_dir_all(format!("{}/proc/self", root)).unwrap();
        std::fs::write(format!("{}/proc/self/exe.symlink", root), "/sbin/busybox").unwrap();

        let stats = materialize_symlink_sidecars(&root, None);
        assert_eq!(stats.sidecars_seen, 3);
        assert_eq!(stats.links_created, 3);
        assert_eq!(stats.sidecars_removed, 3);

        assert!(std::fs::symlink_metadata(format!("{}/sbin/sh", root))
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false));
        let t = std::fs::read_link(format!("{}/sbin/sh", root)).unwrap();
        assert_eq!(t, Path::new(&format!("{}/sbin/busybox", root)));
        let cat = std::fs::read_link(format!("{}/sbin/cat", root)).unwrap();
        assert_eq!(cat, Path::new(&format!("{}/sbin/busybox", root)));
        assert!(!Path::new(&format!("{}/sbin/sh.symlink", root)).exists());
        // stub subtree untouched
        assert!(Path::new(&format!("{}/proc/self/exe.symlink", root)).exists());
    }

    #[test]
    fn materialize_busybox_links_point_at_staged_copy() {
        let tmp = TempGuard(tempdir_for_test());
        let root = tmp.0.to_str().unwrap().to_string();
        std::fs::create_dir_all(format!("{}/sbin", root)).unwrap();
        std::fs::write(format!("{}/sbin/busybox", root), b"\x7fELFplaceholder").unwrap();
        std::fs::write(format!("{}/sbin/sh.symlink", root), "/sbin/busybox").unwrap();
        let staged = "/cache/twoyi_stage/_sbin_busybox_deadbeef";
        let stats = materialize_symlink_sidecars(&root, Some(staged));
        assert_eq!(stats.links_created, 1);
        let t = std::fs::read_link(format!("{}/sbin/sh", root)).unwrap();
        assert_eq!(t, Path::new(staged));
    }

    #[test]
    fn prestage_registers_both_keys_and_reuses() {
        let tmp = TempGuard(tempdir_for_test());
        let root = tmp.0.to_str().unwrap().to_string();
        let data = tmp.0.join("data").to_str().unwrap().to_string();
        std::fs::create_dir_all(format!("{}/sbin", root)).unwrap();
        // Minimal valid-ELF-magic blob is enough for the staging engine.
        std::fs::write(format!("{}/sbin/busybox", root), b"\x7fELFrest").unwrap();

        let p1 = prestage_executable(&root, &data, "/sbin/busybox", "/sbin/busybox").unwrap();
        let p2 = prestage_executable(&root, &data, "/sbin/busybox", "/sbin/sh").unwrap();
        assert!(p1.contains("twoyi_stage"));
        assert!(p2.contains("twoyi_stage"));
        assert_ne!(p1, p2);
        let marker = std::fs::read_to_string(format!("{}/cache/twoyi-staged", data)).unwrap();
        assert!(marker.contains("/sbin/busybox\t"));
        assert!(marker.contains("/sbin/sh\t"));
        // Idempotent re-run replaces, not duplicates.
        let _ = prestage_executable(&root, &data, "/sbin/busybox", "/sbin/sh").unwrap();
        let marker2 = std::fs::read_to_string(format!("{}/cache/twoyi-staged", data)).unwrap();
        assert_eq!(marker2.matches("/sbin/sh\t").count(), 1);
    }

    fn tempdir_for_test() -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("kr64-symlinks-test-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).expect("tempdir");
        dir
    }

    struct TempGuard(std::path::PathBuf);
    impl Drop for TempGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
