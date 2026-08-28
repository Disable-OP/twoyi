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

/// 6-Z196: read an ELF's current PT_INTERP string (the dynamic-linker
/// path the kernel will open at execve time). Class-aware (ELF32 +
/// ELF64). Returns:
///   * `Ok(None)`    — valid ELF, but static (no PT_INTERP)
///   * `Ok(Some(s))` — the interpreter path
///   * `Err(msg)`    — not an ELF / unreadable / implausible phdrs
pub fn read_elf_interp(path: &str) -> Result<Option<String>, String> {
    use std::io::{Seek, SeekFrom};
    let mut file = std::fs::File::open(path).map_err(|e| format!("open {}: {}", path, e))?;
    let mut ehdr = [0u8; 64];
    file.read_exact(&mut ehdr)
        .map_err(|e| format!("read ehdr {}: {}", path, e))?;
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
    if e_phnum == 0 {
        // No program headers at all — a well-defined "static" ELF.
        return Ok(None);
    }
    let (p_off_field, p_sz_field, sz_bytes): (u64, u64, usize) =
        if is64 { (8, 32, 8) } else { (4, 16, 4) };
    let min_phentsize: usize = if is64 { 56 } else { 32 };
    let file_len = file.metadata().map(|m| m.len()).unwrap_or(u64::MAX);
    if e_phentsize < min_phentsize
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
    for i in 0..e_phnum {
        let off = i * e_phentsize;
        let p_type = u32::from_le_bytes(phdrs[off..off + 4].try_into().unwrap());
        if p_type == 3 {
            let read_u = |field: u64| -> u64 {
                let s = off + field as usize;
                let mut v = 0u64;
                for b in (0..sz_bytes).rev() {
                    v = (v << 8) | phdrs[s + b] as u64;
                }
                v
            };
            let p_offset = read_u(p_off_field);
            let p_filesz = read_u(p_sz_field);
            if p_offset
                .checked_add(p_filesz)
                .map_or(true, |end| end > file_len)
            {
                return Err("PT_INTERP out of file bounds".to_string());
            }
            // The interp string lives at FILE offset p_offset — seek
            // there and read it (NOT from the phdr table buffer).
            let mut raw = vec![0u8; p_filesz as usize];
            file.seek(SeekFrom::Start(p_offset)).ok();
            file.read_exact(&mut raw)
                .map_err(|e| format!("read interp string: {}", e))?;
            let nul = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            return Ok(Some(String::from_utf8_lossy(&raw[..nul]).into_owned()));
        }
    }
    Ok(None)
}

/// 6-Z196: make a STAGED executable run under the GUEST'S OWN dynamic
/// linker. The kernel opens PT_INTERP itself during execve — outside
/// tracer reach — so a staged copy whose PT_INTERP is a bare guest path
/// (e.g. "/system/bin/linker64") resolves on the HOST:
///   * on an Android host it loads the HOST's linker (API-level
///     mismatch against the guest's libs — "CANNOT LINK EXECUTABLE",
///     observed run 32973154137: Android-14 linker vs Android-6
///     recovery; and run 33157500559: OrangeFox init under the host
///     linker could not satisfy libbacktrace.so),
///   * on a non-Android host it ENOENTs and execve fails outright.
/// The fix: when the guest's ramdisk SHIPS the interpreter at
/// {rootfs}<PT_INTERP>, rewrite the staged copy's PT_INTERP to that
/// absolute host path — the kernel then loads the guest's own linker,
/// giving a fully coherent guest runtime (guest linker + guest libs).
/// Idempotent: an interp that already points under the rootfs (the
/// 6-Z50/6-Z187 pre-patched forms) is left untouched. Returns the new
/// interpreter path when a patch was applied.
pub fn ensure_guest_interp(rootfs: &str, staged_path: &str) -> Option<String> {
    let rootfs_p = if rootfs.ends_with('/') {
        rootfs.to_string()
    } else {
        format!("{}/", rootfs)
    };
    // Not an ELF (scripts reach the staging engine too) / static →
    // nothing to do. Errors are non-fatal: staging still succeeds.
    let interp = match read_elf_interp(staged_path) {
        Ok(Some(i)) => i,
        _ => return None,
    };
    if !interp.starts_with('/') {
        return None; // relative interp — nothing sane to map
    }
    if interp.starts_with(rootfs_p.as_str()) || interp == rootfs.trim_end_matches('/') {
        return None; // already patched to a host-visible guest path
    }
    let host_interp = format!("{}{}", rootfs_p, interp.trim_start_matches('/'));
    if !Path::new(&host_interp).exists() {
        // The guest does not ship this interpreter — leave the staged
        // copy alone (the host fallback may still resolve it).
        return None;
    }
    match patch_elf_interp(Path::new(staged_path), &host_interp) {
        Ok(()) => Some(host_interp),
        Err(_) => None,
    }
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

/// 6-Z196: compute a guest-RELATIVE symlink target (the path from the
/// link's directory to its target): `init -> system/bin/init` instead of
/// the host-absolute `{rootfs}/system/bin/init`.
///
/// WHY: the previous materialization created links with HOST-ABSOLUTE
/// targets — kernel resolution stayed inside the rootfs (correct), but
/// the guest's `readlink("/init")` returned
/// "/data/user/0/io.twoyi.debug/rootfs/system/bin/init" — the host
/// backing path made VISIBLE as a guest pathname (the absolute VFS
/// invariant: the host backing store must never leak into the guest
/// namespace; File Manager `ls -l` showed it). A guest-relative target
/// resolves to the SAME file on both sides and leaks nothing.
pub fn guest_relative_target(link_guest_dir: &str, target_guest: &str) -> Option<String> {
    if !link_guest_dir.starts_with('/') || !target_guest.starts_with('/') {
        return None;
    }
    let dir_parts: Vec<&str> = link_guest_dir
        .split('/')
        .filter(|p| !p.is_empty())
        .collect();
    let tgt_parts: Vec<&str> = target_guest.split('/').filter(|p| !p.is_empty()).collect();
    let mut common = 0;
    while common < dir_parts.len()
        && common < tgt_parts.len()
        && dir_parts[common] == tgt_parts[common]
    {
        common += 1;
    }
    let ups = dir_parts.len() - common;
    let mut parts: Vec<&str> = vec![".."; ups];
    parts.extend_from_slice(&tgt_parts[common..]);
    if parts.is_empty() {
        // Target equals the link's own directory — no sensible relative
        // form; caller falls back to the host-absolute materialization.
        return None;
    }
    Some(parts.join("/"))
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
            let guest_dir = format!("/{}", parent_rel);
            let Some(guest_target) = resolve_guest_target(&guest_dir, &target) else {
                stats.skipped += 1;
                continue;
            };
            // Host-side symlink target.
            //
            // 6-Z196: guest-RELATIVE form ("system/bin/init", "../../sh")
            // so readlink() in the guest shows a clean guest path — the
            // previous host-absolute form
            // ("{rootfs}/system/bin/init") leaked the host backing
            // path into the guest namespace. Kernel resolution is
            // unchanged: a relative target resolves INSIDE the rootfs
            // exactly like the prefixed form did.
            //
            // EXCEPTION: links whose target is the pre-staged busybox
            // keep pointing at the STAGED copy (the rootfs partition is
            // noexec; the cache staging dir is the executable place —
            // see provision_terminal_shell). Those links' readlink
            // still shows the staged path; fixing that needs exit-side
            // readlink rewriting in the tracer (follow-up).
            let host_target: String = match staged_busybox {
                Some(sb) if guest_target == "/sbin/busybox" => sb.to_string(),
                _ => match guest_relative_target(&guest_dir, &guest_target) {
                    Some(rel) => rel,
                    None => format!("{}{}", rootfs_p, guest_target.trim_start_matches('/')),
                },
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
    fn guest_relative_target_basic() {
        // OrangeFox shape: /init -> /system/bin/init at the root.
        assert_eq!(
            guest_relative_target("/", "/system/bin/init").as_deref(),
            Some("system/bin/init")
        );
        // Same-dir target collapses to the bare name.
        assert_eq!(
            guest_relative_target("/sbin", "/sbin/busybox").as_deref(),
            Some("busybox")
        );
        // Climb out of a subtree.
        assert_eq!(
            guest_relative_target("/sbin/etc/terminfo", "/sbin/sh").as_deref(),
            Some("../../sh")
        );
        // No common prefix.
        assert_eq!(
            guest_relative_target("/system/bin", "/vendor/bin/x").as_deref(),
            Some("../../vendor/bin/x")
        );
        // Target == link dir → None (fallback case).
        assert_eq!(guest_relative_target("/sbin", "/sbin"), None);
        // Non-absolute inputs → None.
        assert_eq!(guest_relative_target("sbin", "/sbin/busybox"), None);
        assert_eq!(guest_relative_target("/sbin", "busybox"), None);
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
        // init -> /system/bin/init (absolute, target absent — the
        // OrangeFox symlink-init shape)
        std::fs::write(format!("{}/init.symlink", root), "/system/bin/init").unwrap();
        // stub subtree that must be skipped
        std::fs::create_dir_all(format!("{}/proc/self", root)).unwrap();
        std::fs::write(format!("{}/proc/self/exe.symlink", root), "/sbin/busybox").unwrap();

        let stats = materialize_symlink_sidecars(&root, None);
        assert_eq!(stats.sidecars_seen, 4);
        assert_eq!(stats.links_created, 4);
        assert_eq!(stats.sidecars_removed, 4);

        assert!(std::fs::symlink_metadata(format!("{}/sbin/sh", root))
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false));
        // 6-Z196: targets are GUEST-RELATIVE — readlink() in the guest
        // must never see the host backing path.
        let t = std::fs::read_link(format!("{}/sbin/sh", root)).unwrap();
        assert_eq!(t, Path::new("busybox"));
        let cat = std::fs::read_link(format!("{}/sbin/cat", root)).unwrap();
        assert_eq!(cat, Path::new("busybox"));
        let charger = std::fs::read_link(format!("{}/charger", root)).unwrap();
        assert_eq!(charger, Path::new("sbin/healthd"));
        let init = std::fs::read_link(format!("{}/init", root)).unwrap();
        assert_eq!(init, Path::new("system/bin/init"));
        assert!(!Path::new(&format!("{}/sbin/sh.symlink", root)).exists());
        // stub subtree untouched
        assert!(Path::new(&format!("{}/proc/self/exe.symlink", root)).exists());
        // Resolution equivalence: the relative link resolves to the same
        // file the old host-absolute form pointed at.
        let resolved = std::fs::read(format!("{}/sbin/sh", root)).unwrap();
        assert_eq!(resolved, b"\x7fELFstatic-placeholder");
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

    /// Build a minimal ELF64 with a single PT_INTERP phdr carrying `interp`.
    fn synthetic_dynamic_elf(interp: &str) -> Vec<u8> {
        let mut out = vec![0u8; 64]; // ELF64 ehdr
        out[0..4].copy_from_slice(b"\x7fELF");
        out[4] = 2; // ELFCLASS64
        out[5] = 1; // ELFDATA2LSB
        out[16..18].copy_from_slice(&2u16.to_le_bytes()); // e_type = EXEC
        out[18..20].copy_from_slice(&0xB7u16.to_le_bytes()); // e_machine = aarch64
        out[32..40].copy_from_slice(&64u64.to_le_bytes()); // e_phoff
        out[52..54].copy_from_slice(&64u16.to_le_bytes()); // e_ehsize
        out[54..56].copy_from_slice(&56u16.to_le_bytes()); // e_phentsize
        out[56..58].copy_from_slice(&1u16.to_le_bytes()); // e_phnum

        let mut interp_bytes = interp.as_bytes().to_vec();
        interp_bytes.push(0);
        let interp_off: u64 = 64 + 56;

        let mut phdr = vec![0u8; 56];
        phdr[0..4].copy_from_slice(&3u32.to_le_bytes()); // PT_INTERP
        phdr[4..8].copy_from_slice(&4u32.to_le_bytes()); // PF_R
        phdr[8..16].copy_from_slice(&interp_off.to_le_bytes()); // p_offset
        phdr[16..24].copy_from_slice(&interp_off.to_le_bytes()); // p_vaddr
        phdr[24..32].copy_from_slice(&interp_off.to_le_bytes()); // p_paddr
        phdr[32..40].copy_from_slice(&(interp_bytes.len() as u64).to_le_bytes()); // p_filesz
        phdr[40..48].copy_from_slice(&(interp_bytes.len() as u64).to_le_bytes()); // p_memsz
        phdr[48..56].copy_from_slice(&1u64.to_le_bytes()); // p_align

        out.extend_from_slice(&phdr);
        out.extend_from_slice(&interp_bytes);
        out
    }

    #[test]
    fn z196_read_elf_interp_class_aware() {
        let tmp = TempGuard(tempdir_for_test());
        let root = tmp.0.to_str().unwrap().to_string();
        let dyn_path = format!("{}/dyn", root);
        std::fs::write(&dyn_path, synthetic_dynamic_elf("/system/bin/linker64")).unwrap();
        let static_path = format!("{}/stat", root);
        let mut static_blob = b"\x7fELFstatic-placeholder".to_vec();
        static_blob.resize(128, 0); // pad past the 64-byte ehdr read
        std::fs::write(&static_path, &static_blob).unwrap();
        let script_path = format!("{}/script", root);
        std::fs::write(&script_path, b"#!/sbin/sh\n").unwrap();

        assert_eq!(
            read_elf_interp(&dyn_path).unwrap().as_deref(),
            Some("/system/bin/linker64")
        );
        assert_eq!(read_elf_interp(&static_path).unwrap(), None);
        assert!(read_elf_interp(&script_path).is_err());
        assert!(read_elf_interp(&format!("{}/missing", root)).is_err());
    }

    #[test]
    fn z196_ensure_guest_interp_patches_to_guest_linker() {
        let tmp = TempGuard(tempdir_for_test());
        let root = tmp.0.to_str().unwrap().to_string();
        // The guest ships its own linker at {root}/system/bin/linker64.
        std::fs::create_dir_all(format!("{}/system/bin", root)).unwrap();
        std::fs::write(
            format!("{}/system/bin/linker64", root),
            b"\x7fELF-linker-placeholder",
        )
        .unwrap();
        // A staged executable with the bare guest interp.
        let staged = format!("{}/staged_init", root);
        std::fs::write(&staged, synthetic_dynamic_elf("/system/bin/linker64")).unwrap();

        let patched = ensure_guest_interp(&root, &staged);
        let expected = format!("{}/system/bin/linker64", root);
        assert_eq!(patched.as_deref(), Some(expected.as_str()));
        // The staged copy now carries the host-visible guest linker path.
        assert_eq!(
            read_elf_interp(&staged).unwrap().as_deref(),
            Some(expected.as_str())
        );
        // Idempotent: an already-patched interp is left untouched.
        assert_eq!(ensure_guest_interp(&root, &staged), None);
    }

    #[test]
    fn z196_ensure_guest_interp_noop_when_guest_lacks_linker() {
        let tmp = TempGuard(tempdir_for_test());
        let root = tmp.0.to_str().unwrap().to_string();
        // No {root}/system/bin/linker64 — the guest does not ship the
        // interpreter; the staged copy must be left alone (host fallback
        // may still resolve it). Also covers static binaries.
        std::fs::create_dir_all(format!("{}/sbin", root)).unwrap();
        let staged = format!("{}/staged_init", root);
        std::fs::write(&staged, synthetic_dynamic_elf("/system/bin/linker64")).unwrap();
        assert_eq!(ensure_guest_interp(&root, &staged), None);
        assert_eq!(
            read_elf_interp(&staged).unwrap().as_deref(),
            Some("/system/bin/linker64")
        );
        // Static ELF (padded past the ehdr) → no PT_INTERP → no-op.
        let staged_static = format!("{}/staged_static", root);
        let mut static_blob2 = b"\x7fELFstatic-placeholder".to_vec();
        static_blob2.resize(128, 0);
        std::fs::write(&staged_static, &static_blob2).unwrap();
        assert_eq!(ensure_guest_interp(&root, &staged_static), None);
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
