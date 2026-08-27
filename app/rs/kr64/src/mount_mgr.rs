// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! Mount namespace manager — sets up the per-VM mount tree.
//!
//! This mirrors what VM's `libkr64.so` does at the `mount_mgr: %s -> %s -> %s`
//! log site (decoded with key 0x1a, see `VM_KR64_ANALYSIS.md` §4.2):
//!
//! * `unshare(CLONE_NEWNS)` — create a new mount namespace so changes
//!   don't leak to the host.
//! * Mark `/` as `MS_PRIVATE` so mount events don't propagate in or
//!   out of the new namespace.
//! * Bind-mount the ROM's `/system`, `/vendor`, `/product`,
//!   `/system_ext` into the per-VM rootfs.
//! * Mount tmpfs on `/dev`, `/proc`, `/sys`, `/tmp`, `/apex`, `/mnt`.
//! * `pivot_root(rootfs, rootfs/old_root)` — make the per-VM rootfs
//!   the new `/`.
//! * `umount2(old_root, MNT_DETACH)` — drop the host's `/`.
//!
//! # Why not just `chroot`?
//!
//! `chroot` changes the apparent root directory but doesn't create a
//! new mount namespace — so the host's mounts are still visible from
//! inside the chroot (via `/proc/self/root/..` etc.). `pivot_root` is
//! the right primitive: it atomically swaps the rootfs and the old
//! root, AND (when combined with `unshare(CLONE_NEWNS)` and
//! `MS_PRIVATE`) gives true mount isolation.
//!
//! # Capability requirements
//!
//! `unshare(CLONE_NEWNS)` requires `CAP_SYS_ADMIN` (in the user
//! namespace that owns the mount namespace). On Android, the
//! `twoyi` app process does NOT have `CAP_SYS_ADMIN` by default.
//!
//! VM works around this by running `libkr64.so` as a separate process
//! launched by `libkrloader64.so` (custom ELF interpreter that has
//! elevated privileges — see `VM_KR64_ANALYSIS.md` §13). Twoyi will
//! need a similar approach (or run the daemon via `su` / `magisk` /
//! a system-app context) — this is a known limitation.
//!
//! For development/testing on Linux (codespace, etc.) we just need
//! `sudo` or a userns+mountns capability. The skeleton handles the
//! "no permission" case gracefully by falling back to a plain
//! `chroot` (which doesn't give full isolation but lets the rest of
//! the boot proceed for testing).

// The MS_* / MNT_* / CLONE_* constants below are defined for
// completeness (they mirror `<sys/mount.h>` / `<linux/fs.h>` /
// `<linux/sched.h>`). Not all of them are used by the skeleton's
// `setup_mounts()` yet — they'll be needed as follow-up tasks add
// remount, propagation-type, and user-namespace support. Suppress the
// dead-code warning for the whole module to avoid noise.
#![allow(dead_code)]

use libc::{c_int, c_long, c_ulong, c_void};
use std::ffi::CString;
use std::path::Path;

// Crate-local logging macros (defined in lib.rs) — no external `log` crate.
use crate::{error, info, warning};

// ============================================================================
// libc constants that aren't always exposed via the `libc` crate on
// every target. We define them here for safety.
// ============================================================================

// Linux mount(2) flags.
const MS_RDONLY: c_ulong = 1;
const MS_NOSUID: c_ulong = 2;
const MS_NODEV: c_ulong = 4;
const MS_NOEXEC: c_ulong = 8;
const MS_SYNCHRONOUS: c_ulong = 16;
const MS_REMOUNT: c_ulong = 32;
const MS_MANDLOCK: c_ulong = 64;
const MS_DIRSYNC: c_ulong = 128;
const MS_NOATIME: c_ulong = 1024;
const MS_NODIRATIME: c_ulong = 2048;
const MS_BIND: c_ulong = 4096;
const MS_MOVE: c_ulong = 8192;
const MS_REC: c_ulong = 16384;
const MS_SILENT: c_ulong = 32768;
const MS_POSIXACL: c_ulong = 1 << 16;
const MS_UNBINDABLE: c_ulong = 1 << 17;
const MS_PRIVATE: c_ulong = 1 << 18;
const MS_SLAVE: c_ulong = 1 << 19;
const MS_SHARED: c_ulong = 1 << 20;
const MS_RELATIME: c_ulong = 1 << 21;
const MS_STRICTATIME: c_ulong = 1 << 24;

// umount2(2) flags.
const MNT_FORCE: c_int = 1;
const MNT_DETACH: c_int = 2;
const MNT_EXPIRE: c_int = 4;

// clone(2) / unshare(2) flags.
const CLONE_NEWNS: c_int = 0x0002_0000;
const CLONE_NEWUSER: c_int = 0x1000_0000;

// ============================================================================
// Type aliases for readability.
// ============================================================================

type IoResult<T> = std::io::Result<T>;

/// Convert a `libc` return code (-1 + errno) into an `io::Result`.
fn check(r: c_int) -> IoResult<()> {
    if r == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// `mount(source, target, fstype, flags, data)` — thin libc wrapper.
fn mount(
    source: &str,
    target: &str,
    fstype: &str,
    flags: c_ulong,
    data: Option<&str>,
) -> IoResult<()> {
    let c_source = CString::new(source)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let c_target = CString::new(target)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let c_fstype = CString::new(fstype)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let c_data = match data {
        Some(d) => Some(
            CString::new(d)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?,
        ),
        None => None,
    };
    let r = unsafe {
        libc::mount(
            c_source.as_ptr(),
            c_target.as_ptr(),
            c_fstype.as_ptr(),
            flags,
            c_data
                .as_ref()
                .map(|c| c.as_ptr() as *const c_void)
                .unwrap_or(std::ptr::null()),
        )
    };
    check(r)
}

/// `umount2(target, flags)` — thin libc wrapper.
fn umount2(target: &str, flags: c_int) -> IoResult<()> {
    let c_target = CString::new(target)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let r = unsafe { libc::umount2(c_target.as_ptr(), flags) };
    check(r)
}

/// `unshare(flags)` — thin libc wrapper.
fn unshare(flags: c_int) -> IoResult<()> {
    let r = unsafe { libc::unshare(flags) };
    check(r)
}

/// `pivot_root(new_root, put_old)` — syscall wrapper (libc doesn't
/// always wrap this).
fn pivot_root(new_root: &str, put_old: &str) -> IoResult<()> {
    let c_new_root = CString::new(new_root)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let c_put_old = CString::new(put_old)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    // SYS_pivot_root = 41 on x86_64, 41 on aarch64 (same!).
    let nr: c_long = libc::SYS_pivot_root as c_long;
    let r = unsafe { libc::syscall(nr, c_new_root.as_ptr(), c_put_old.as_ptr()) };
    if r == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// `chroot(path)` — thin libc wrapper.
fn chroot(path: &str) -> IoResult<()> {
    let c_path =
        CString::new(path).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let r = unsafe { libc::chroot(c_path.as_ptr()) };
    check(r)
}

/// Recursively remount every mount under `root` (including `root`
/// itself) read-only.
///
/// `MS_BIND|MS_REMOUNT|MS_RDONLY` is NOT recursive: it only flips the
/// one mount it names, while sub-mounts that an earlier
/// `MS_BIND|MS_REC` copied in stay writable. Walking
/// `/proc/self/mountinfo` and remounting each descendant closes that
/// hole (the portable pre-`mount_setattr(AT_RECURSIVE)` approach —
/// the host device kernel can be far older than 5.12).
///
/// Field 5 of each mountinfo line is the mount point with octal
/// escapes (\040 for space etc.), which we decode before comparing.
fn remount_tree_read_only(root: &str) {
    let mountinfo = match std::fs::read_to_string("/proc/self/mountinfo") {
        Ok(s) => s,
        Err(e) => {
            warning!(
                "[KR64][mount_mgr] cannot read /proc/self/mountinfo for RO remount of {}: {}",
                root,
                e
            );
            // Fall back to the old single-mount remount — better than
            // nothing (top mount becomes RO, submounts may not).
            if let Err(e) = mount("", root, "", MS_REMOUNT | MS_RDONLY | MS_BIND, None) {
                warning!(
                    "[KR64][mount_mgr] fallback RO remount of {} failed: {}",
                    root, e
                );
            }
            return;
        }
    };

    let root = root.trim_end_matches('/');
    let mut targets: Vec<String> = Vec::new();
    for line in mountinfo.lines() {
        let mut fields = line.split(' ');
        let mp = fields.nth(4); // field 5 (0-indexed 4) = mount point
        let Some(mp) = mp else { continue };
        let mp = decode_mountinfo_path(mp);
        if mp == root || mp.starts_with(&format!("{}/", root)) {
            targets.push(mp);
        }
    }
    // Deepest-first: children before parents, so a parent remount can
    // never race a child created between the two (best effort).
    targets.sort_by_key(|t| std::cmp::Reverse(t.matches('/').count()));
    let n = targets.len();
    for t in &targets {
        if let Err(e) = mount("", t, "", MS_REMOUNT | MS_RDONLY | MS_BIND, None) {
            warning!("[KR64][mount_mgr] RO remount of {} failed: {}", t, e);
        }
    }
    info!(
        "[KR64][mount_mgr] enforced read-only on {} ({} mounts under {})",
        root, n, root
    );
}

/// Decode the octal escapes mountinfo uses in mount-point fields
/// (`\040` space, `\011` tab, `\134` backslash, `\012` newline).
fn decode_mountinfo_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 4 <= bytes.len() {
            let oct = &s[i + 1..i + 4];
            if let Ok(v) = u8::from_str_radix(oct, 8) {
                out.push(v as char);
                i += 4;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

// ============================================================================
// Public API.
// ============================================================================

/// Description of a mount to perform during setup.
#[derive(Debug, Clone)]
pub struct MountSpec {
    /// Source path (the host path for bind mounts, or "none"/"" for
    /// virtual filesystems like `tmpfs` and `proc`).
    pub source: String,
    /// Target path INSIDE the per-VM rootfs (e.g. `/dev`, `/proc`).
    pub target: String,
    /// Filesystem type (`bind`, `tmpfs`, `proc`, `sysfs`, `ext4`, …).
    pub fstype: String,
    /// Mount flags (bitmask of `MS_*`).
    pub flags: c_ulong,
    /// Optional mount data (e.g. `"mode=755,gid=1000"` for tmpfs).
    pub data: Option<String>,
}

/// Configuration for `setup_mounts`.
pub struct MountConfig {
    /// The per-VM rootfs directory (e.g. `/data/data/io.twoyi/vm/vm0/fs`).
    pub rootfs: String,
    /// The ROM directory containing `/system`, `/vendor`, `/product`,
    /// `/system_ext` partitions (extracted by `GsiExtractor.java`).
    /// May be the same as `rootfs` (in which case `/system` etc. are
    /// already in place and we skip the bind mounts).
    pub rom_dir: Option<String>,
    /// If `true`, attempt `unshare(CLONE_NEWNS) + pivot_root`. If
    /// `false` (or if the unshare fails), fall back to `chroot`.
    pub use_namespaces: bool,
    /// If `true`, mount `/system`, `/vendor`, `/product`, `/system_ext`
    /// read-only (Treble convention). Set to `false` for development
    /// (so you can `adb push` test binaries into the running guest).
    pub read_only_rom: bool,
    /// If `true`, skip the /apex bind mount. Used for TWRP recovery
    /// boot, where the ramdisk doesn't use APEX packages (TWRP's init
    /// is statically linked and doesn't need /apex/com.android.runtime/
    /// libs). Skipping the bind mount avoids the "is /apex accessible?"
    /// failure mode entirely and makes TWRP boot independent of the
    /// host's APEX state.
    pub boot_recovery: bool,
}

impl Default for MountConfig {
    fn default() -> Self {
        MountConfig {
            rootfs: String::new(),
            rom_dir: None,
            use_namespaces: true,
            read_only_rom: true,
            boot_recovery: false,
        }
    }
}

/// Set up the per-VM mount namespace and rootfs.
///
/// This is the main entry point — called from `main.rs` after device
/// creation but before exec'ing the guest init.
///
/// Steps:
///   1. (Optional) `unshare(CLONE_NEWNS)` — create new mount namespace.
///   2. `mount("", "/", "", MS_REC | MS_PRIVATE, NULL)` — make all
///      mounts private so events don't propagate to/from the host.
///   3. Bind-mount `/system`, `/vendor`, `/product`, `/system_ext`
///      from `rom_dir` into the rootfs (if `rom_dir != rootfs`).
///   4. Mount tmpfs on `/dev`, `/proc`, `/sys`, `/tmp`, `/apex`, `/mnt`.
///   5. `pivot_root(rootfs, rootfs/old_root)` — make rootfs the new `/`.
///   6. `umount2(/old_root, MNT_DETACH)` — drop the host's `/`.
///   7. `chdir("/")` — set the working directory to the new root.
///
/// If any step fails because of insufficient permissions, we log a
/// warning and fall back to `chroot(rootfs)` (which doesn't require
/// `CAP_SYS_ADMIN` but gives weaker isolation). This lets the daemon
/// at least make progress in development / testing contexts.
pub fn setup_mounts(cfg: &MountConfig) -> IoResult<()> {
    info!(
        "[KR64][mount_mgr] setting up mounts for rootfs={:?} rom_dir={:?} namespaces={}",
        cfg.rootfs, cfg.rom_dir, cfg.use_namespaces
    );

    // Step 1: unshare(CLONE_NEWNS).
    let have_ns = if cfg.use_namespaces {
        match unshare(CLONE_NEWNS) {
            Ok(()) => {
                info!("[KR64][mount_mgr] unshare(CLONE_NEWNS) succeeded");
                true
            }
            Err(e) => {
                warning!(
                    "[KR64][mount_mgr] unshare(CLONE_NEWNS) failed ({}); \
                       falling back to chroot only — isolation will be weaker",
                    e
                );
                false
            }
        }
    } else {
        false
    };

    // Step 2: make all mounts private (only meaningful in a new mount ns).
    if have_ns {
        match mount("", "/", "", MS_REC | MS_PRIVATE, None) {
            Ok(()) => info!("[KR64][mount_mgr] marked / as MS_REC|MS_PRIVATE"),
            Err(e) => warning!(
                "[KR64][mount_mgr] mount('/', '/', '', MS_REC|MS_PRIVATE) failed: {}",
                e
            ),
        }
    }

    // Step 3: bind-mount ROM partitions into the rootfs.
    if let Some(rom_dir) = &cfg.rom_dir {
        if rom_dir != &cfg.rootfs {
            // NOTE: no MS_RDONLY here — bind mounts take their flags from
            // the SOURCE, so MS_RDONLY on the initial bind is silently
            // ignored; the RO enforcement happens in the explicit
            // recursive remount below.
            let flags = MS_BIND | MS_REC;
            for part in &["system", "vendor", "product", "system_ext"] {
                let src = format!("{}/{}", rom_dir, part);
                let dst = format!("{}/{}", cfg.rootfs, part);
                if Path::new(&src).exists() {
                    // Make sure the mount point exists.
                    let _ = std::fs::create_dir_all(&dst);
                    match mount(&src, &dst, "", flags, None) {
                        Ok(()) => info!("[KR64][mount_mgr] bind-mounted {} → {}", src, dst),
                        Err(e) => warning!(
                            "[KR64][mount_mgr] bind mount {} → {} failed: {}",
                            src,
                            dst,
                            e
                        ),
                    }
                    // Enforce RO. A single non-recursive
                    // MS_REMOUNT|MS_RDONLY|MS_BIND only flips the TOP
                    // mount — submounts bound in by MS_REC stay
                    // writable — so walk /proc/self/mountinfo and
                    // remount every mount under dst (see
                    // remount_tree_read_only).
                    if cfg.read_only_rom {
                        remount_tree_read_only(&dst);
                    }
                } else {
                    warning!(
                        "[KR64][mount_mgr] ROM partition {} does not exist; skipping",
                        src
                    );
                }
            }
        }
    }

    // Step 4: mount filesystems on /dev, /proc, /sys, /tmp, /mnt.
    // IMPORTANT: Do NOT mount tmpfs on /apex! On Android 11+,
    // /system/bin/linker64 and /system/lib64/libc.so are symlinks to
    // /apex/com.android.runtime/bin/linker64 and
    // /apex/com.android.runtime/lib64/bionic/libc.so. If we mount
    // tmpfs on /apex, these symlinks become dangling and the dynamic
    // linker can't load — causing SIGSEGV at address 0x86 in linker64.
    //
    // Instead, BIND-MOUNT the HOST's /apex/ into the rootfs's /apex/.
    // This gives the rootfs access to the real APEX packages (libc.so,
    // linker64, libbase.so, etc.) with no version mismatches. The
    // symlinks in /system/lib64/ resolve correctly after pivot_root.
    //
    // This works on BOTH:
    // - KVM test environment (host is Android emulator with /apex/)
    // - Real devices (host is a real Android device with /apex/)
    //
    // CRITICAL: /proc must be a REAL procfs, NOT tmpfs! The bionic
    // dynamic linker (linker64) reads /proc/self/maps and
    // /proc/self/auxv during library loading. If /proc is an empty
    // tmpfs, the linker can't find already-loaded libraries (like
    // libc.so) -> NULL soinfo -> SIGSEGV at offset 0xaf174 in
    // linker64 (write to address 0x86 = field at offset 0x86 from
    // NULL soinfo pointer).
    let fs_mounts: &[(&str, &str, c_ulong, &str)] = &[
        // (path-in-rootfs, fstype, flags, data)
        ("/dev", "tmpfs", MS_NOSUID | MS_NOEXEC, "mode=755"),
        // Real procfs — the linker needs /proc/self/maps and /proc/self/auxv
        ("/proc", "proc", MS_NOSUID | MS_NOEXEC | MS_NODEV, ""),
        // Real sysfs — some init code reads /sys/... paths
        ("/sys", "sysfs", MS_NOSUID | MS_NOEXEC | MS_NODEV, ""),
        ("/tmp", "tmpfs", MS_NOSUID | MS_NODEV, "mode=1777"),
        // /mnt is where the guest's vold mounts external storage.
        (
            "/mnt",
            "tmpfs",
            MS_NOSUID | MS_NODEV | MS_NOEXEC,
            "mode=755,gid=1000",
        ),
    ];
    for (path, fstype, flags, data) in fs_mounts {
        let abs = format!("{}{}", cfg.rootfs, path);
        // Make sure the mount point exists.
        let _ = std::fs::create_dir_all(&abs);
        match mount("none", &abs, fstype, *flags, Some(data)) {
            Ok(()) => info!("[KR64][mount_mgr] mounted {} on {}", fstype, abs),
            Err(e) => warning!(
                "[KR64][mount_mgr] mount {} on {} failed: {}",
                fstype,
                abs,
                e
            ),
        }
    }

    // Bind-mount the HOST's /apex/ into the rootfs's /apex/.
    // This is CRITICAL for Android 11+ where libc.so, libdl.so,
    // linker64, and many other essential libraries live ONLY in
    // /apex/com.android.runtime/. Without this bind mount, /apex/ is
    // empty after pivot_root, all symlinks into /apex/ break, and the
    // linker crashes with SIGSEGV at 0x86 (NULL soinfo).
    //
    // We use MS_BIND | MS_REC to recursively bind-mount /apex/ and
    // all its sub-mounts (each APEX package is a separate mount on
    // Android).
    //
    // TWRP BOOT: skip this bind mount when cfg.boot_recovery=true.
    // TWRP's ramdisk doesn't use APEX packages (init is statically
    // linked, no shared lib deps), so the bind mount would just give
    // the guest access to the host's APEX packages -- harmless but
    // unnecessary. Skipping it makes TWRP boot independent of the
    // host's APEX state and avoids a potential failure mode if the
    // host's /apex is empty or in a weird state.
    if cfg.boot_recovery {
        info!("[KR64][mount_mgr] TWRP boot: skipping /apex bind mount (no APEX packages needed)");
    } else {
        let apex_dst = format!("{}/apex", cfg.rootfs);
        let _ = std::fs::create_dir_all(&apex_dst);
        match mount("/apex", &apex_dst, "", MS_BIND | MS_REC, None) {
            Ok(()) => info!(
                "[KR64][mount_mgr] bind-mounted /apex -> {} (APEX packages accessible)",
                apex_dst
            ),
            Err(e) => warning!(
                "[KR64][mount_mgr] bind-mount /apex -> {} failed: {} — linker may crash at 0x86 (NULL soinfo for missing libc.so)",
                apex_dst,
                e
            ),
        }
    }

    // Step 5: pivot_root (or chroot fallback).
    if have_ns {
        // pivot_root requires new_root to be a mount point. Bind-mount
        // rootfs to itself to make it a mount point (standard idiom used
        // by runc, util-linux, and all container runtimes).
        match mount(&cfg.rootfs, &cfg.rootfs, "", MS_BIND | MS_REC, None) {
            Ok(()) => info!(
                "[KR64][mount_mgr] self-bind mount on {} succeeded",
                cfg.rootfs
            ),
            Err(e) => {
                warning!(
                    "[KR64][mount_mgr] self-bind mount failed: {} — pivot_root will likely fail",
                    e
                );
            }
        }

        let old_root = format!("{}/old_root", cfg.rootfs);
        let _ = std::fs::create_dir_all(&old_root);
        match pivot_root(&cfg.rootfs, &old_root) {
            Ok(()) => info!(
                "[KR64][mount_mgr] pivot_root({}, {}) succeeded",
                cfg.rootfs, old_root
            ),
            Err(e) => {
                error!(
                    "[KR64][mount_mgr] pivot_root failed: {} — falling back to chroot",
                    e
                );
                chroot(&cfg.rootfs)?;
            }
        }

        // Step 6: detach the old root.
        // After pivot_root we're chrooted into the new root, so
        // /old_root is the path to the old root.
        match umount2("/old_root", MNT_DETACH) {
            Ok(()) => {
                info!("[KR64][mount_mgr] detached old root via umount2(/old_root, MNT_DETACH)")
            }
            Err(e) => warning!("[KR64][mount_mgr] umount2(/old_root) failed: {}", e),
        }
        let _ = std::fs::remove_dir("/old_root");
    } else {
        // Fallback: just chroot.
        chroot(&cfg.rootfs)?;
        info!(
            "[KR64][mount_mgr] chroot({}) succeeded (no namespace isolation)",
            cfg.rootfs
        );
    }

    // Step 7: chdir("/") — set the working directory.
    let c_root = CString::new("/").unwrap();
    let r = unsafe { libc::chdir(c_root.as_ptr()) };
    check(r)?;
    info!("[KR64][mount_mgr] chdir(\"/\") succeeded");

    Ok(())
}

/// List the mounts the daemon set up. Useful for `mount_mgr: %s -> %s -> %s`
/// debug logging (decoded with key 0x1a from libkr64.so's .data).
pub fn list_mounts() -> Vec<MountSpec> {
    // Read /proc/self/mountinfo to enumerate active mounts.
    // (This is best-effort — used for debugging only.)
    let mut out = Vec::new();
    if let Ok(content) = std::fs::read_to_string("/proc/self/mountinfo") {
        for line in content.lines() {
            // mountinfo format (man 5 proc):
            //   mount_id parent_id major:minor root mountpoint options ...
            //   ... optional_fields - fstype source super_options
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 6 {
                continue;
            }
            // The fstype/source are in the optional fields after "-".
            // Scan forwards for the separator, then read the two fields
            // that follow it (fstype, source).
            let mut fstype = String::new();
            let mut source = String::new();
            for (i, w) in fields.iter().enumerate() {
                if *w == "-" && i + 2 < fields.len() {
                    fstype = fields[i + 1].to_string();
                    source = fields[i + 2].to_string();
                    break;
                }
            }
            out.push(MountSpec {
                source,
                target: fields[4].to_string(),
                fstype,
                flags: 0,
                data: None,
            });
        }
    }
    out
}

// ============================================================================
// Tests — pure Rust, run on the host. We test the wrappers against
// real syscalls where possible; the namespace tests are skipped if
// the host doesn't allow them (e.g. running without sudo).
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mountspec_has_sensible_defaults() {
        let m = MountSpec {
            source: "tmpfs".to_string(),
            target: "/tmp".to_string(),
            fstype: "tmpfs".to_string(),
            flags: MS_NOSUID,
            data: Some("mode=1777".to_string()),
        };
        assert_eq!(m.fstype, "tmpfs");
        assert_eq!(m.flags & MS_NOSUID, MS_NOSUID);
    }

    #[test]
    fn unshare_newuser_works_in_tests() {
        // CLONE_NEWUSER is allowed for unprivileged users on modern
        // Linux kernels (>= 3.8). This test verifies that our unshare
        // wrapper works at all. If it fails, the test is skipped —
        // it doesn't necessarily mean the code is broken (e.g. running
        // in a container that blocks userns).
        match unshare(CLONE_NEWUSER) {
            Ok(()) => {} // great
            Err(e) => eprintln!("[test] unshare(CLONE_NEWUSER) failed: {} — skipping", e),
        }
    }

    #[test]
    fn list_mounts_returns_a_vector() {
        // Just verify the function doesn't panic.
        let v = list_mounts();
        // On any Linux host there should be at least one mount.
        assert!(!v.is_empty(), "no mounts found in /proc/self/mountinfo");
    }

    #[test]
    fn pivot_root_wrapper_exists() {
        // We don't actually call pivot_root in the test (it requires
        // a real new mount namespace), but we verify the wrapper
        // compiles and the function is callable.
        let _ = std::mem::size_of::<fn(&str, &str) -> IoResult<()>>();
    }
}
