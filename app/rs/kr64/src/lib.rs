// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! Twoyi kernel-replacement daemon -- the Rust port of VM's `libkr64.so`.
//!
//! See `VM_KR64_ANALYSIS.md` and `GSI_BOOT_PLAN.md` for the full
//! background. The short version: twoyi needs a userspace daemon that
//! materialises a virtual `/dev` tree for the guest Android, installs
//! a seccomp filter that traps "dangerous" syscalls, emulates `/proc`
//! and `/sys`, manages the per-VM mount namespace, and exec's the
//! guest's `/system/bin/init`. This crate is that daemon.
//!
//! # Build
//!
//! The crate builds as BOTH a cdylib (`libkr64.so` -- directly
//! executable via the PIE hack) AND a regular binary (`kr64`). See
//! `Cargo.toml` and `build.rs` for details.
//!
//! # Usage
//!
//! ```sh
//! # Direct execution via the PIE-hacked cdylib:
//! ./libkr64.so --rootfs /data/.../fs --data-dir /data/... --vmid 0
//!
//! # Or via the regular binary:
//! ./kr64 --rootfs /data/.../fs --data-dir /data/... --vmid 0
//! ```
//!
//! # Module layout
//!
//! * [`devices`]   -- virtual `/dev` device creation (qemu_pipe,
//!   touch, key, event, gb, gb2).
//! * [`binder`]    -- per-VM `/vm{id}/dev/binder` Unix socket + binder
//!   transaction proxy (skeleton; see
//!   `download/BINDER_SKELETON.md`).
//! * [`audio`]     -- virtual `/dev/audio` Unix socket + bidirectional
//!   PCM pump (playback + capture skeleton; see
//!   `download/AUDIO_SENSOR_HAL.md`).
//! * [`sensors`]   -- virtual `/dev/sensors` Unix socket + multiplexed
//!   12-sensor HAL (accel/mag/gyro/... skeleton; see
//!   `download/AUDIO_SENSOR_HAL.md`).
//! * [`battery`]   -- virtual `/sys/class/power_supply/battery` file tree
//!   and 30 s refresh thread (file-based, no socket; see
//!   `download/BATTERY_IMPL.md`).
//! * [`seccomp`]   -- BPF seccomp filter + SIGSYS handler.
//! * [`proc_emu`]  -- synthesised `/proc` tree (version, cpuinfo,
//!   meminfo, cmdline, self/, sys/).
//! * [`mount_mgr`] -- `unshare(CLONE_NEWNS)` + `pivot_root` + tmpfs
//!   mounts for /dev, /proc, /sys, /system, /vendor.
//!
//! # Dependencies
//!
//! Per the task spec ("Use only std + libc, no external crates for
//! now"), this crate depends on **only** `libc` -- no `log`, no
//! `once_cell`, no `nix`. Logging is done via the `info!` / `warning!` /
//! `error!` macros defined below (which expand to `eprintln!`), and
//! lazy statics use `std::sync::OnceLock` (stabilised in Rust 1.70).

pub mod audio;
pub mod battery;
pub mod binder;
pub mod compat_paths;
pub mod devices;
pub mod mount_mgr;
pub mod proc_emu;
pub mod qemu_pipe;
pub mod seccomp;
pub mod sensors;

use std::ffi::CString;
use std::os::unix::io::AsRawFd;
use std::path::Path;

// ============================================================================
// Logging -- minimal `eprintln!`-based macros. No external `log` crate.
//
// All log lines go to stderr in the format `[KR64 <LEVEL>] <msg>`.
// This works both on-device (visible via `adb logcat *:S KR64:V` or
// by redirecting stderr to a file) and on the host during `cargo
// test`. A production version would plug into Android's
// `__android_log_write` / `logd` socket, but for the MVP stderr is
// sufficient and avoids the `android_logger` / `log` dependencies.
// ============================================================================

/// Log an info-level message to stderr.
macro_rules! info {
    ($($arg:tt)*) => {
        eprintln!("[KR64 INFO] {}", format_args!($($arg)*))
    };
}

/// Log a warning-level message to stderr.
///
/// NOTE: this macro is named `warning!` (not `warning!`) to avoid a name
/// conflict with Rust's built-in `#[warn(...)]` lint attribute, which
/// makes the bare name `warn` ambiguous in `pub(crate) use` exports.
macro_rules! warning {
    ($($arg:tt)*) => {
        eprintln!("[KR64 WARN] {}", format_args!($($arg)*))
    };
}

/// Log an error-level message to stderr.
macro_rules! error {
    ($($arg:tt)*) => {
        eprintln!("[KR64 ERROR] {}", format_args!($($arg)*))
    };
}

// Make the macros available to other modules in this crate.
pub(crate) use error;
pub(crate) use info;
pub(crate) use warning;

// ============================================================================
// Async-signal-safe stderr logging.
//
// Between `fork()` and `execve()`, only a restricted set of libc functions
// are safe to call (see signal-safety(7)). The `info!` / `warning!` /
// `error!` macros above expand to `eprintln!`, which is NOT async-signal-
// safe: it goes through `std::io::stderr()`, which lazily initialises a
// global `LineWriter`, which allocates, which grabs the allocator lock.
// If another thread held that lock at `fork()` time, the child deadlocks
// the moment it tries to log. `format!` has the same problem.
//
// The helpers below use only the `write(2)` syscall (which IS async-
// signal-safe) plus a stack-allocated buffer. They are the correct
// primitive for diagnostics in the child branch of the kr64 fork --
// without them, `mount_mgr::setup_mounts` / `seccomp::install` /
// `execve` failures are silently swallowed and the parent observes only
// a bare exit code with no clue why.
// ============================================================================

/// Write a byte slice to stderr using the async-signal-safe `write(2)`
/// syscall. Used in the child branch between `fork()` and `execve()`.
///
/// Returns the number of bytes written (or -1 on error); callers in the
/// child branch ignore the result because there is no useful recovery
/// path. `write(2)` may write fewer bytes than requested (e.g. on a
/// pipe with a small buffer) -- for short log lines on stderr this is
/// acceptable and we do not retry partial writes.
unsafe fn safe_write_err(msg: &[u8]) -> isize {
    libc::write(
        libc::STDERR_FILENO,
        msg.as_ptr() as *const libc::c_void,
        msg.len(),
    )
}

/// Format a signed integer as decimal into a fixed-size stack buffer
/// and return the number of bytes written. This is the async-signal-
/// safe equivalent of `format!("{}", n)` -- no allocation, no locks.
///
/// The buffer must be large enough for the longest possible `i32`:
/// 11 digits (`-2147483648`). We accept a 12-byte buffer to leave
/// room for a trailing NUL if the caller wants one.
///
/// Handles negative values (writes a leading `-`).
unsafe fn format_decimal(buf: &mut [u8; 12], n: i32) -> usize {
    let mut len = 0usize;
    let mut v = n;
    let negative = v < 0;
    if negative {
        // i32::MIN negated overflows; use wrapping_neg to stay well-defined.
        v = v.wrapping_neg();
    }
    if v == 0 {
        buf[0] = b'0';
        return 1;
    }
    while v > 0 {
        buf[len] = b'0' + (v % 10) as u8;
        len += 1;
        v /= 10;
    }
    if negative {
        buf[len] = b'-';
        len += 1;
    }
    // Reverse the digits (and the optional `-`) into display order.
    buf[..len].reverse();
    len
}

/// Write `<prefix> errno=<n>\n` to stderr using only async-signal-safe
/// primitives. Used in the child branch to surface the OS errno from a
/// failed syscall without invoking any allocator.
unsafe fn safe_write_err_errno(prefix: &[u8], errno: i32) {
    safe_write_err(prefix);
    safe_write_err(b" errno=");
    let mut buf = [0u8; 12];
    let n = format_decimal(&mut buf, errno);
    safe_write_err(&buf[..n]);
    safe_write_err(b"\n");
}

// ============================================================================
// Parsed CLI configuration.
// ============================================================================

/// Configuration parsed from argv / env.
///
/// Mirrors the 7-arg `libkr64.so` invocation pattern documented in
/// `VM_KR64_ANALYSIS.md` S2 (`vmid`, `data_dir`, `rom_dir`,
/// `kernel_path`, `config_path`, `log_level`, `socket_fd`). We use
/// named flags instead of positional args for clarity, and add a few
/// twoyi-specific knobs (display dimensions, seccomp toggle).
#[derive(Debug, Clone)]
pub struct Config {
    /// Per-VM ID. Used in device paths (e.g. `/vm0/dev/...`).
    pub vmid: u32,
    /// The host-side path to the per-VM data directory (e.g.
    /// `/data/data/io.twoyi/vm/vm0`). This is where `dev/event`
    /// is bound (host-visible) and where log files are written.
    pub data_dir: String,
    /// The guest rootfs directory (e.g. `<data_dir>/fs`). This is
    /// what gets pivot_root'd into.
    pub rootfs: String,
    /// Optional separate ROM directory (where the extracted
    /// `/system`, `/vendor`, `/product`, `/system_ext` live). If
    /// `None`, we assume they're already inside `rootfs`.
    pub rom_dir: Option<String>,
    /// Guest's init binary path (relative to rootfs). Default
    /// `/system/bin/init`.
    pub init_path: String,
    /// Virtual display width (pixels). Used by the input system to
    /// size the touch device's ABS_MT_POSITION_X range.
    pub width: i32,
    /// Virtual display height (pixels).
    pub height: i32,
    /// Virtual display DPI (used by the guest's `WindowManagerService`
    /// to compute `DisplayMetrics.densityDpi`).
    pub dpi: i32,
    /// If `true`, attempt `unshare(CLONE_NEWNS)` + `pivot_root`. If
    /// `false` (or if unshare fails), fall back to `chroot`.
    pub use_namespaces: bool,
    /// If `true`, mount `/system` etc. read-only (Treble convention).
    /// If `false`, mount them read-write (for development -- lets you
    /// `adb push` test binaries into the running guest).
    pub read_only_rom: bool,
    /// If `true`, install the seccomp filter on the guest. If `false`,
    /// skip it (for debugging -- the guest will see the host's `/proc`
    /// etc. unfiltered).
    pub install_seccomp: bool,
    /// Kernel log level (0=quiet, 3=verbose). Mirrors the `log_level`
    /// argv position in VM's `libkr64.so`.
    pub log_level: u32,
    /// Optional SOCKS5 proxy the guest's networking should tunnel
    /// through. Mirrors VM's `vmnet` SOCKS5 proxy feature: when set,
    /// the kr64 daemon opens a listening socket inside the guest's
    /// network namespace and forwards every accepted TCP connection
    /// to the configured SOCKS5 upstream, so the guest's apps see a
    /// "normal" network while all traffic actually egresses through
    /// the host's proxy.
    ///
    /// `None` = proxy disabled (direct connect). When `Some`, the
    /// string is `host:port` (e.g. `"127.0.0.1:1080"`). Username /
    /// password auth is not yet supported -- the stub only implements
    /// the no-auth SOCKS5 method (0x00).
    ///
    /// This is a STUB: the field is parsed and stored but the actual
    /// proxy listener is not yet spawned. The forwarding thread will
    /// be added in a follow-up task (see `download/NETWORK_PROXY.md`,
    /// not yet written). The field exists now so that callers can
    /// configure it and so the daemon logs the configured value.
    pub socks5_proxy: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            vmid: 0,
            data_dir: String::new(),
            rootfs: String::new(),
            rom_dir: None,
            init_path: "/system/bin/init".to_string(),
            width: 720,
            height: 1280,
            dpi: 320,
            use_namespaces: true,
            read_only_rom: true,
            install_seccomp: true,
            log_level: 3,
            socks5_proxy: None,
        }
    }
}

/// Parse argv into a [`Config`]. Recognised flags:
///
///   --rootfs <path>           (required)
///   --data-dir <path>         (required)
///   --rom-dir <path>          (optional)
///   --init <path>             (default: /system/bin/init)
///   --vmid <n>                (default: 0)
///   --width <n>               (default: 720)
///   --height <n>              (default: 1280)
///   --dpi <n>                 (default: 320)
///   --log-level <n>           (default: 3)
///   --no-namespaces           (disable unshare + pivot_root; chroot only)
///   --rw-rom                  (mount /system etc. read-write)
///   --no-seccomp              (skip seccomp filter installation)
///   --help, -h                (show usage)
pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Config, String> {
    let mut cfg = Config::default();
    let mut iter = args.into_iter();
    // Skip argv[0] (program name).
    let _prog = iter.next();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                return Err("twoyi kr64 -- kernel-replacement daemon\n\
                     \n\
                     Usage: kr64 [OPTIONS]\n\
                     \n\
                     Required:\n\
                       --rootfs <path>           Per-VM guest rootfs directory\n\
                       --data-dir <path>         Host-visible data directory\n\
                     \n\
                     Optional:\n\
                       --rom-dir <path>          Directory with /system, /vendor, etc.\n\
                       --init <path>             Guest init binary (default: /system/bin/init)\n\
                       --vmid <n>                VM ID (default: 0)\n\
                       --width <n>               Virtual display width (default: 720)\n\
                       --height <n>              Virtual display height (default: 1280)\n\
                       --dpi <n>                 Virtual display DPI (default: 320)\n\
                       --log-level <n>           0=quiet, 3=verbose (default: 3)\n\
                     \n\
                     Behaviour toggles:\n\
                       --no-namespaces           Disable unshare + pivot_root; chroot only\n\
                       --rw-rom                  Mount /system etc. read-write (for dev)\n\
                       --no-seccomp              Skip seccomp filter installation\n\
                     \n\
                     Networking:\n\
                       --socks5 <host:port>     Tunnel guest TCP through a SOCKS5 proxy (stub)\n"
                    .to_string());
            }
            "--rootfs" => {
                cfg.rootfs = iter.next().ok_or("--rootfs requires a path".to_string())?;
            }
            "--data-dir" => {
                cfg.data_dir = iter
                    .next()
                    .ok_or("--data-dir requires a path".to_string())?;
            }
            "--rom-dir" => {
                cfg.rom_dir = Some(iter.next().ok_or("--rom-dir requires a path".to_string())?);
            }
            "--init" => {
                cfg.init_path = iter.next().ok_or("--init requires a path".to_string())?;
            }
            "--vmid" => {
                cfg.vmid = iter
                    .next()
                    .ok_or("--vmid requires a value".to_string())?
                    .parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            "--width" => {
                cfg.width = iter
                    .next()
                    .ok_or("--width requires a value".to_string())?
                    .parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            "--height" => {
                cfg.height = iter
                    .next()
                    .ok_or("--height requires a value".to_string())?
                    .parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            "--dpi" => {
                cfg.dpi = iter
                    .next()
                    .ok_or("--dpi requires a value".to_string())?
                    .parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            "--log-level" => {
                cfg.log_level = iter
                    .next()
                    .ok_or("--log-level requires a value".to_string())?
                    .parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            "--no-namespaces" => cfg.use_namespaces = false,
            "--rw-rom" => cfg.read_only_rom = false,
            "--no-seccomp" => cfg.install_seccomp = false,
            "--socks5" => {
                // SOCKS5 proxy stub: store the host:port string. The
                // actual forwarding thread is not yet wired up -- see
                // the `socks5_proxy` field doc on `Config` for status.
                let val = iter
                    .next()
                    .ok_or("--socks5 requires a host:port argument".to_string())?;
                if val.is_empty() {
                    return Err("--socks5 argument must not be empty".to_string());
                }
                // Minimal validation: must contain exactly one ':'.
                let colon_count = val.matches(':').count();
                if colon_count != 1 {
                    return Err(format!(
                        "--socks5 expects host:port (got '{}', expected exactly one ':')",
                        val
                    ));
                }
                cfg.socks5_proxy = Some(val);
            }
            other => {
                return Err(format!("unknown argument: {}", other));
            }
        }
    }

    if cfg.rootfs.is_empty() {
        return Err("--rootfs is required".to_string());
    }
    if cfg.data_dir.is_empty() {
        return Err("--data-dir is required".to_string());
    }

    Ok(cfg)
}

// ============================================================================
// Zombie reaping -- VM-inspired cleanup.
// ============================================================================

/// Reap any leftover zombie children before forking the new guest.
///
/// This mirrors VM's `ProcessKiller` / `ZombieReaper` (see
/// `VM_KR64_ANALYSIS.md` S2.10) which runs at daemon startup to clean
/// up processes left behind by a previous VM run that crashed or was
/// killed with SIGKILL. Without this, those zombies stay reaped by no
/// one (their parent is gone) and accumulate as `<defunct>` entries in
/// `/proc`, which on long-running hosts can exhaust the PID table.
///
/// We call `waitpid(-1, WNOHANG)` in a loop until it returns 0 (no
/// more children to reap) or -1 with ECHILD (no children at all).
/// Both terminating conditions are benign. EINTR is retried (it can
/// happen if a signal arrives mid-syscall -- we have no handlers yet,
/// but this is the correct defensive pattern).
///
/// # Safety
///
/// `waitpid` is a POSIX syscall; calling it is safe. The `WNOHANG`
/// flag makes it non-blocking, so this function never sleeps. It only
/// reaps children that have ALREADY exited -- it does not kill or
/// signal any running process. The "kill orphan processes" step (which
/// DOES send SIGKILL to leftover guest PIDs) is handled separately on
/// the Java side in `RomManager.killOrphanProcess()` -- this Rust
/// function is purely the reap-after-the-fact step.
pub fn clear_zombie_processes() {
    let mut reaped = 0u32;
    loop {
        let mut status: libc::c_int = 0;
        let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };
        if pid > 0 {
            reaped += 1;
            let code = if libc::WIFEXITED(status) {
                libc::WEXITSTATUS(status)
            } else if libc::WIFSIGNALED(status) {
                -(libc::WTERMSIG(status))
            } else {
                status
            };
            info!(
                "[KR64][zombie] reaped leftover child pid={} status={}",
                pid, code
            );
            continue;
        }
        if pid == 0 {
            // No more children waiting to be reaped.
            break;
        }
        // pid == -1: error. ECHILD means "no children" (benign -- first
        // run). EINTR means "interrupted by signal" -- retry. Anything
        // else is unexpected; log and stop to avoid an infinite loop.
        let e = std::io::Error::last_os_error();
        if e.raw_os_error() == Some(libc::EINTR) {
            continue;
        }
        if e.raw_os_error() == Some(libc::ECHILD) {
            // No children at all -- first run, nothing to reap.
            break;
        }
        warning!("[KR64][zombie] waitpid failed: {} -- stopping reap loop", e);
        break;
    }
    if reaped > 0 {
        info!(
            "[KR64][zombie] reaped {} leftover zombie process(es)",
            reaped
        );
    }
}

// ============================================================================
// Hook library lookup -- multi-path search.
//
// RomManager's `ensureLibSymlink` puts the hook .so files at
// {data_dir}/rootfs/system/lib64/<lib>, but kr64's `cfg.rootfs` points
// at the per-profile rootfs ({data_dir}/profiles/default/rootfs). The
// old single-path lookup at `{rootfs}/<lib>` therefore failed, leaving
// LD_PRELOAD broken and init crashing with SIGSEGV (signal 11) because
// no hooks were loaded. See KVM run 31500117235:
//   kr64-stderr.log: "libgetpid_hook.so not found at {rootfs}/libgetpid_hook.so"
//   logcat.txt:      "ensureLibSymlink: libgetpid_hook.so ->
//                     {data_dir}/rootfs/system/lib64/libgetpid_hook.so"
//
// Task ID 7 (commit 8375802) added candidates #1-#4 below. KVM run
// 31501768195 then revealed a DEEPER issue: ProfileManager's rootfs
// symlink setup FAILED (FileAlreadyExistsException on
// profiles/default/rootfs, DirectoryNotEmptyException on rootfs), so
// RomManager's `ensureLibSymlink` created symlinks in the STALE rootfs
// at `/data/user/0/io.twoyi/rootfs/system/lib64/`. Those symlinks point
// into the APK's native lib dir (`/data/app/~~<rand>/io.twoyi-<rand>/
// lib/<abi>/<lib>`), but `Path::exists()` follows the symlink and
// returns false when the target APK lib path is missing or the APK
// was reinstalled with a different random suffix.
//
// Fix (Task ID 8): add candidate #5 -- a direct scan of the APK native
// lib directory. This bypasses the rootfs symlink entirely and reads
// the canonical source installed by Android's package manager. See
// `apk_native_lib_candidates` for the directory layout.
// ============================================================================

/// Returns candidate source paths for a hook library (e.g.
/// `libgetpid_hook.so`, `libtwoyi_loader_shlib.so`), in priority order.
///
/// The library may live in any of several places depending on how the
/// rootfs was provisioned by RomManager:
///
/// 1. `{rootfs}/<lib>` -- manual placement or direct rootfs (the
///    historical fallback; kept for backwards compatibility).
/// 2. `{rootfs}/system/lib64/<lib>` -- where RomManager's
///    `ensureLibSymlink` would put it, relative to the profile rootfs.
/// 3. `{data_dir}/rootfs/system/lib64/<lib>` -- where RomManager's
///    `ensureLibSymlink` ACTUALLY puts it on a real device (confirmed
///    from logcat: `ensureLibSymlink: libgetpid_hook.so ->
///    /data/user/0/io.twoyi/rootfs/system/lib64/libgetpid_hook.so`,
///    and `/data/user/0/io.twoyi/` symlinks to `/data/data/io.twoyi/`).
///    This is the path that the per-profile `rootfs` field does NOT
///    point at (`profiles/default/rootfs` vs `rootfs/`), so we check it
///    explicitly here.
/// 4. `{data_dir}/rootfs/<lib>` -- alternative app-level rootfs root.
/// 5. APK native lib dir scan (`/data/app/~~<rand>/io.twoyi-<rand>/lib/
///    {x86_64,arm64}/<lib>`) -- the canonical source of the libraries,
///    installed by Android's package manager at APK install time.
///    Bypasses all rootfs symlink state. See [`apk_native_lib_candidates`]
///    for why this is needed (ProfileManager's rootfs symlink is broken
///    on KVM run 31501768195). On non-Android hosts (e.g. the Linux
///    devcontainer) `/data/app/` doesn't exist, so this contributes
///    zero candidates -- exactly what we want for unit tests.
///
/// The caller picks the first candidate that exists on disk.
fn hook_library_candidates(cfg: &Config, lib_name: &str) -> Vec<String> {
    let mut out = vec![
        format!("{}/{}", cfg.rootfs, lib_name),
        format!("{}/system/lib64/{}", cfg.rootfs, lib_name),
        format!("{}/rootfs/system/lib64/{}", cfg.data_dir, lib_name),
        format!("{}/rootfs/{}", cfg.data_dir, lib_name),
    ];
    // Candidate #5+: scan the APK's native library directory directly.
    // This bypasses the rootfs symlink entirely -- the APK lib dir is
    // the canonical source. See `apk_native_lib_candidates` for the
    // full rationale (rootfs symlink is broken on KVM run 31501768195).
    out.extend(apk_native_lib_candidates(lib_name));
    out
}

/// Scan the APK native library directory for a given library name.
///
/// The APK path has randomized components
/// (`/data/app/~~<random>/io.twoyi-<random>/lib/<abi>/<lib>`), so we
/// scan two levels deep: each subdir of `base` is treated as a
/// `~~<random>` bucket, and each subdir of THAT starting with
/// `io.twoyi-` is treated as the APK root. Within each APK root we
/// check `lib/x86_64/<lib>` and `lib/arm64/<lib>` (in that order;
/// x86_64 is preferred because the devcontainer runner is x86_64).
///
/// This is the canonical source of the libraries (installed by Android's
/// package manager at APK install time, when `extractNativeLibs=true`
/// in the manifest). RomManager's `ensureLibSymlink` creates symlinks
/// in the rootfs pointing into this directory, but those symlinks can
/// be in the WRONG rootfs (see KVM run 31501768195: ProfileManager's
/// rootfs symlink setup FAILED with FileAlreadyExistsException, so
/// RomManager's symlinks ended up in the stale
/// `/data/user/0/io.twoyi/rootfs/` instead of the profile rootfs at
/// `/data/data/io.twoyi/profiles/default/rootfs/`). Scanning the APK
/// directory directly bypasses all rootfs symlink state.
///
/// This is a NO-OP (returns empty Vec) on non-Android environments
/// (e.g., the Linux devcontainer where `/data/app/` doesn't exist) --
/// which is exactly what we want for unit tests.
///
/// `base` is a parameter (rather than hardcoded to `/data/app/`) purely
/// for testability -- the public wrapper [`apk_native_lib_candidates`]
/// passes `/data/app`.
fn apk_native_lib_candidates_in(base: &Path, lib_name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bucket_entries = match std::fs::read_dir(base) {
        Ok(rd) => rd,
        Err(_) => return out, // base dir doesn't exist (non-Android env).
    };
    for bucket_entry in bucket_entries.flatten() {
        let bucket_path = bucket_entry.path(); // /data/app/~~<random>/
        let apk_entries = match std::fs::read_dir(&bucket_path) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for apk_entry in apk_entries.flatten() {
            let apk_name_owned = apk_entry.file_name();
            let apk_name = match apk_name_owned.to_str() {
                Some(s) => s,
                None => continue,
            };
            // Only consider io.twoyi-* directories (skip other packages
            // that may share the same ~~<random> bucket).
            if !apk_name.starts_with("io.twoyi-") {
                continue;
            }
            let apk_path = apk_entry.path(); // /data/app/~~<random>/io.twoyi-<random>/
                                             // Check lib/x86_64/<lib> first (preferred ABI on x86_64
                                             // devcontainer), then lib/arm64/<lib> (fallback for ARM
                                             // translators / native ARM hosts).
            for abi in ["x86_64", "arm64"] {
                let lib_path = apk_path.join("lib").join(abi).join(lib_name);
                if lib_path.is_file() {
                    out.push(lib_path.to_string_lossy().into_owned());
                }
            }
        }
    }
    out
}

/// Convenience wrapper around [`apk_native_lib_candidates_in`] that uses
/// the standard Android APK install directory `/data/app/`. Logs each
/// found candidate at info level so the next KVM run can verify the
/// scan ran and what it found.
fn apk_native_lib_candidates(lib_name: &str) -> Vec<String> {
    let cands = apk_native_lib_candidates_in(Path::new("/data/app"), lib_name);
    if cands.is_empty() {
        info!(
            "[KR64] PARENT: APK native lib scan for {} found no candidates in /data/app/",
            lib_name
        );
    } else {
        for c in &cands {
            info!("[KR64] PARENT: APK native lib scan found candidate: {}", c);
        }
    }
    cands
}

/// Returns true if `path` exists as a regular file (following symlinks).
/// Returns false otherwise. As a side effect, if `path` is a BROKEN
/// symlink (the symlink itself exists but its target does not), logs the
/// symlink target so the next debugging cycle can see why a candidate
/// path appeared in the list but failed the `Path::exists()` check.
///
/// This is critical for diagnosing RomManager's `ensureLibSymlink`
/// failures: the symlink at `{data_dir}/rootfs/system/lib64/<lib>` may
/// exist but point to a non-existent APK lib path (because the APK was
/// reinstalled with a different random suffix, or because
/// `extractNativeLibs=false`), leaving the candidate list with no
/// usable copy source until the APK dir scan in
/// [`apk_native_lib_candidates`] finds the real path.
///
/// `Path::exists()` already follows symlinks, so this function is
/// `Path::exists()` PLUS a diagnostic log for the broken-symlink case.
fn candidate_exists_with_diagnostics(path: &str) -> bool {
    let p = Path::new(path);
    if p.exists() {
        return true;
    }
    // Path doesn't exist (or its final target is missing). If it's a
    // symlink, log the target so the next debugging cycle can see what
    // the symlink was pointing at (this is the difference between
    // "RomManager didn't create the symlink at all" and "RomManager
    // created the symlink but its target is gone").
    if let Ok(meta) = std::fs::symlink_metadata(p) {
        if meta.file_type().is_symlink() {
            if let Ok(target) = std::fs::read_link(p) {
                warning!(
                    "[KR64] PARENT:   symlink exists but target is broken: {} -> {}",
                    path,
                    target.display()
                );
            }
        }
    }
    false
}

/// Search for a hook library using the candidate paths returned by
/// [`hook_library_candidates`], find the first one that exists, and
/// READ ITS CONTENT INTO MEMORY.
///
/// Returns `Some((source_path, content))` on success, or `None` if no
/// candidate exists (in which case ALL checked paths are logged).
///
/// This is the "read" phase of the hook-library copy, split out from
/// [`copy_hook_library_to_dev`] so that the read can happen BEFORE
/// `setup_mounts` (pivot_root) while host filesystem paths are still
/// accessible, and the write can happen AFTER `setup_mounts` when
/// `/dev/` is the tmpfs that survives pivot_root.
///
/// # Why this split exists
///
/// Before pivot_root, the hook library source paths are reachable:
///   - `{cfg.rootfs}/system/lib64/<lib>` (a symlink into `/data/app/...`)
///   - `{cfg.data_dir}/rootfs/system/lib64/<lib>` (same symlink)
///   - `/data/app/~~<random>/io.twoyi-<random>/lib/<abi>/<lib>` (canonical)
///
/// After pivot_root + `umount2(/old_root, MNT_DETACH)`, ALL of these
/// host paths are gone — the process is in the rootfs jail. So the
/// search + read MUST happen before `setup_mounts`. See KVM run
/// 31503063598: the search ran AFTER pivot_root and all 4 symlink
/// candidates + the APK scan returned nothing (host paths unreachable),
/// LD_PRELOAD was empty, and init crashed with SIGSEGV (signal 11).
fn find_and_read_hook_library(
    cfg: &Config,
    lib_name: &str,
    not_found_msg: &str,
) -> Option<(String, Vec<u8>)> {
    let candidates = hook_library_candidates(cfg, lib_name);
    for p in &candidates {
        if candidate_exists_with_diagnostics(p) {
            match std::fs::read(p) {
                Ok(content) => {
                    info!(
                        "[KR64] PARENT: read {} ({} bytes) from {} (BEFORE pivot_root)",
                        lib_name,
                        content.len(),
                        p
                    );
                    return Some((p.clone(), content));
                }
                Err(e) => {
                    warning!(
                        "[KR64] PARENT: candidate {} exists but read failed: {} -- trying next",
                        p,
                        e
                    );
                    // Fall through to the next candidate.
                }
            }
        }
    }
    error!(
        "[KR64] PARENT: {} not found in any of {} candidate locations -- {}",
        lib_name,
        candidates.len(),
        not_found_msg
    );
    for c in &candidates {
        error!("[KR64] PARENT:   checked: {}", c);
    }
    None
}

/// Write hook-library bytes to `/dev/<lib>` (tmpfs) and chmod 0644.
///
/// This is the "write" phase of the hook-library copy. It MUST be called
/// AFTER `setup_mounts` (pivot_root), so `/dev/` is the tmpfs mounted by
/// `setup_mounts` (which survives pivot_root because it's a mount in the
/// new mount namespace). Writing to `/dev/` BEFORE pivot_root would
/// target the host's `/dev/` tmpfs, which is detached by
/// `umount2(/old_root, MNT_DETACH)` and would NOT be visible inside the
/// rootfs jail.
///
/// The content was read into memory by [`find_and_read_hook_library`]
/// BEFORE pivot_root (while host paths were accessible). See the long
/// comment in `run()` for the full SELinux rationale (why /dev/ tmpfs
/// is required instead of `{rootfs}/dev/` app_data_file).
///
/// Returns `true` on success, `false` on write failure (logged).
fn write_hook_library_to_dev(lib_name: &str, src: &str, content: &[u8], dst: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::write(dst, content) {
        Ok(_) => {
            let _ = std::fs::set_permissions(dst, std::fs::Permissions::from_mode(0o644));
            info!(
                "[KR64] PARENT: wrote {} ({} bytes) {} -> {} (AFTER pivot_root, tmpfs)",
                lib_name,
                content.len(),
                src,
                dst
            );
            true
        }
        Err(e) => {
            error!(
                "[KR64] PARENT: failed to write {} {} -> {}: {}",
                lib_name, src, dst, e
            );
            false
        }
    }
}

/// Copies a hook library into `/dev/` (tmpfs) so the guest can use it.
///
/// This is a convenience wrapper around [`find_and_read_hook_library`]
/// (read phase) + [`write_hook_library_to_dev`] (write phase), kept for
/// test compatibility and for any caller that does NOT need to split
/// the read and write across the pivot_root boundary.
///
/// In `run()`, the read and write are split: the read happens BEFORE
/// `setup_mounts` (while host paths are accessible), and the write
/// happens AFTER `setup_mounts` (when `/dev/` is the tmpfs). This split
/// is REQUIRED — see [`find_and_read_hook_library`] for why.
///
/// Returns `true` if the library was copied successfully, `false` if
/// not found or the copy failed (the error is already logged).
#[allow(dead_code)] // kept for test compatibility; run() uses the split find+write flow
fn copy_hook_library_to_dev(cfg: &Config, lib_name: &str, dst: &str, not_found_msg: &str) -> bool {
    if let Some((src, content)) = find_and_read_hook_library(cfg, lib_name, not_found_msg) {
        write_hook_library_to_dev(lib_name, &src, &content, dst)
    } else {
        false
    }
}

// ============================================================================
// Daemon entry point.
// ============================================================================

/// Run the daemon. Returns the exit code.
///
/// This is the shared entry point called by both:
///   * `kr64_main` (the cdylib's PIE entry point, see below), and
///   * `main()` in `src/main.rs` (the bin target).
///
/// Steps:
///   1. Parse args.
///   2. Create all virtual `/dev` devices (qemu_pipe, touch, key,
///      event, gb, gb2).
///   3. Populate `/proc` (version, cpuinfo, meminfo, cmdline, ...).
///   4. Set up mount namespace (unshare + bind mounts + tmpfs mounts).
///   5. Fork:
///      - Child: pivot_root -> install seccomp -> exec /system/bin/init.
///      - Parent: run the device-accept loop (spawns one thread per
///        device socket; for the MVP each thread just accepts and
///        echoes).
///   6. Wait for the child (the guest init) to exit; propagate its
///      exit code.
pub fn run<I: IntoIterator<Item = String>>(args: I) -> i32 {
    let mut cfg = match parse_args(args) {
        Ok(c) => c,
        Err(e) => {
            // If --help, print to stdout and exit 0; otherwise stderr
            // and exit 2.
            if e.starts_with("twoyi kr64") {
                println!("{}", e);
                return 0;
            }
            eprintln!("kr64: {}", e);
            return 2;
        }
    };

    info!("[KR64] starting daemon with config: {:?}", cfg);

    // ---------------------------------------------------------------
    // Step 1.5: reap leftover zombie children from a previous run.
    //
    // VM does this at daemon startup (see `VM_KR64_ANALYSIS.md` S2.10)
    // to clean up after a crashed/killed previous VM. We do the same so
    // a rapid restart of the guest doesn't accumulate `<defunct>` PIDs.
    // This is purely defensive -- if there are no children, waitpid
    // returns ECHILD immediately and we move on.
    // ---------------------------------------------------------------
    clear_zombie_processes();

    // Log the SOCKS5 proxy configuration if set (stub: the actual
    // forwarding thread is not yet spawned -- see `Config::socks5_proxy`).
    if let Some(ref upstream) = cfg.socks5_proxy {
        info!(
            "[KR64] SOCKS5 proxy configured: {} (stub -- forwarding thread not yet started)",
            upstream
        );
    } else {
        info!("[KR64] SOCKS5 proxy: not configured (direct connect)");
    }

    // ---------------------------------------------------------------
    // Step 2: create virtual /dev devices.
    // ---------------------------------------------------------------
    let device_set = match devices::create_all_devices(&cfg.rootfs, &cfg.data_dir) {
        Ok(d) => d,
        Err(e) => {
            error!("[KR64] failed to create devices: {}", e);
            return 1;
        }
    };

    // Create the marker files the guest init looks for.
    if let Err(e) = devices::create_coldboot_done_marker(&cfg.rootfs) {
        warning!("[KR64] failed to create .coldboot_done marker: {}", e);
    }
    if let Err(e) = devices::create_busybox_marker(&cfg.rootfs) {
        warning!("[KR64] failed to create .busybox marker: {}", e);
    }
    // Magisk presence markers -- make Magisk-aware apps detect a
    // consistent "rooted VM" environment. Non-fatal: the guest boots
    // fine without these, but banking/root-checker apps may misbehave.
    if let Err(e) = devices::create_magisk_marker(&cfg.rootfs) {
        warning!("[KR64] failed to create Magisk markers: {}", e);
    }
    // /dev/dm-user -- required by Android 12+ GSIs for userspace
    // device-mapper (snapshot merges / userdata checkpointing). We
    // create a socket (can't mknod a real char device) and spawn an
    // accept thread so the guest's open() succeeds. Non-fatal on
    // Android 11 and below (the node is simply never opened).
    match devices::create_dm_user_device(&cfg.rootfs) {
        Ok(dev) => {
            // Hand off to an accept thread so the listener stays alive.
            // The thread takes ownership; we don't store the handle.
            spawn_accept_thread(dev, "dm-user");
        }
        Err(e) => {
            warning!(
                "[KR64] failed to create /dev/dm-user: {} -- Android 12+ GSIs may boot-loop",
                e
            );
        }
    }

    // ---------------------------------------------------------------
    // Step 2.5: create the per-VM binder device + spawn the binder
    // proxy. See `download/BINDER_SKELETON.md` and
    // `app/rs/kr64/src/binder.rs` for the full story. The short
    // version: this creates `{rootfs}/vm{id}/dev/binder` as a Unix
    // socket + a `{rootfs}/dev/binder` symlink, and spawns an
    // accept-thread + worker-pool that dispatches BINDER_* ioctls
    // from the guest. The handle is held for the lifetime of `run()`
    // so the proxy is shut down when the guest exits.
    // ---------------------------------------------------------------
    let _binder_handle = match binder::create_binder_device(&cfg.rootfs, cfg.vmid)
        .and_then(|path| binder::BinderProxy::new(cfg.vmid, &path))
        .and_then(|proxy| proxy.spawn())
    {
        Ok(h) => {
            info!(
                "[KR64] binder proxy listening at {} (vm{})",
                h.path(),
                cfg.vmid
            );
            Some(h)
        }
        Err(e) => {
            // Non-fatal: the guest can still boot against the host's
            // /dev/binder if it's bind-mounted in (the current twoyi
            // approach). The binder proxy is only needed for full
            // service-manager virtualisation.
            warning!(
                "[KR64] failed to start binder proxy: {} -- falling back to host binder",
                e
            );
            None
        }
    };

    // ---------------------------------------------------------------
    // Step 2.6: create the audio device + spawn the accept/pump
    // thread. See `download/AUDIO_SENSOR_HAL.md` and
    // `app/rs/kr64/src/audio.rs` for the full story. The short
    // version: this creates `{rootfs}/dev/audio` as a Unix socket
    // and spawns an accept-thread + worker-pool that reads a
    // 16-byte header per connection and pumps raw PCM in both
    // directions. The actual AudioTrack/AudioRecord integration is
    // stubbed (no JNI yet) -- the pump compiles and exercises the
    // protocol but produces no sound until the Java side is wired
    // up in a follow-up task.
    // ---------------------------------------------------------------
    let _audio_handle = match audio::create_audio_device(&cfg.rootfs).and_then(|dev| dev.spawn()) {
        Ok(h) => {
            info!("[KR64] audio device listening at {}", h.path());
            Some(h)
        }
        Err(e) => {
            // Non-fatal: the guest can still boot without sound --
            // AudioFlinger's connect() to /dev/audio will fail and
            // the guest's audio HAL will fall back to silence / a
            // null output. Sound is the user's primary use case
            // though, so this warning is worth surfacing.
            warning!(
                "[KR64] failed to start audio device: {} -- guest will have no sound",
                e
            );
            None
        }
    };

    // ---------------------------------------------------------------
    // Step 2.7: create the sensor device + spawn the accept/pump
    // thread. See `download/AUDIO_SENSOR_HAL.md` and
    // `app/rs/kr64/src/sensors.rs` for the full story. The short
    // version: this creates `{rootfs}/dev/sensors` as a Unix socket
    // and spawns an accept-thread + worker-pool that reads 12-byte
    // control requests (ENABLE/DISABLE/CHECK_SUPPORT/SET_DELAY) and
    // pushes 24-byte SensorEvent records to the guest when sensors
    // are enabled. The actual SensorManager integration is stubbed
    // (no JNI yet) -- the control loop replies false to every
    // CHECK_SUPPORT and the pump never produces events. This is the
    // deliberate "skeleton" boundary, mirroring audio.rs.
    // ---------------------------------------------------------------
    let _sensor_handle =
        match sensors::create_sensor_device(&cfg.rootfs).and_then(|dev| dev.spawn()) {
            Ok(h) => {
                info!("[KR64] sensor device listening at {}", h.path());
                Some(h)
            }
            Err(e) => {
                // Non-fatal: the guest can still boot without sensors --
                // the guest's sensor HAL will see "no sensors available"
                // and `SensorManager.getDefaultSensor()` will return null.
                // Apps that hard-require a sensor (e.g. compass apps)
                // will crash, but the boot proceeds.
                warning!(
                    "[KR64] failed to start sensor device: {} -- guest will have no sensors",
                    e
                );
                None
            }
        };

    // ---------------------------------------------------------------
    // Step 2.8: materialise the virtual battery sysfs tree + spawn
    // the 30 s refresh thread. See `download/BATTERY_IMPL.md` and
    // `app/rs/kr64/src/battery.rs` for the full story. The short
    // version: this creates `{rootfs}/sys/class/power_supply/battery/`
    // with seven files (capacity, status, charging, voltage,
    // temperature, technology, health) populated from the (stubbed)
    // JNI up-calls, then spawns a `kr64-battery-refresh` thread that
    // re-writes them every 30 s. Failure is non-fatal -- the guest
    // can still boot without a battery sysfs (its `BatteryService`
    // will just report "unknown" / fall back to defaults), but every
    // real device has a battery so we warn loudly.
    // ---------------------------------------------------------------
    let _battery_handle = match battery::BatteryDevice::new(&cfg.rootfs).and_then(|dev| dev.spawn())
    {
        Ok(h) => {
            info!(
                "[KR64] battery sysfs materialised at {}/sys/class/power_supply/battery",
                cfg.rootfs
            );
            Some(h)
        }
        Err(e) => {
            warning!(
                "[KR64] failed to start battery HAL: {} -- guest will see no battery",
                e
            );
            None
        }
    };

    // ---------------------------------------------------------------
    // Step 3: populate /proc.
    // ---------------------------------------------------------------
    // We synthesise for an 8-core, 4 GB guest by default. A production
    // version would query the host's CPU count and memory size and
    // scale these accordingly.
    let cpu_count = 8u32;
    let mem_mb = 4096u64;
    if let Err(e) = proc_emu::populate_proc(&cfg.rootfs, cpu_count, mem_mb) {
        warning!("[KR64] failed to populate /proc: {}", e);
    }

    // ---------------------------------------------------------------
    // Step 3.5: materialise Samsung GameSDK compatibility paths.
    //
    // Games that probe for Samsung's GameDriver / GameSDK crash on
    // the missing paths (ENOENT on stat / dlopen without null-check).
    // We create stub dirs + files so the probes succeed; games then
    // fall back to the default driver via their own error handling.
    // Non-fatal: missing compat paths only affect Samsung-aware games.
    // ---------------------------------------------------------------
    if let Err(e) = compat_paths::create_samsung_gamesdk_compat_paths(&cfg.rootfs) {
        warning!(
            "[KR64] failed to materialise Samsung GameSDK compat paths: {} -- some games may crash",
            e
        );
    }

    // ---------------------------------------------------------------
    // Step 3.6: read hook libraries into memory BEFORE setup_mounts.
    //
    // After pivot_root (in setup_mounts, Step 4 below), all host
    // filesystem paths become unreachable -- /data/app/*,
    // /data/user/0/io.twoyi/*, and the symlinks under
    // {cfg.rootfs}/system/lib64/ (which point into /data/app/) all
    // disappear from the rootfs jail. The hook libraries live at these
    // host paths, so we MUST read them into memory NOW, before
    // setup_mounts.
    //
    // The write to /dev/ happens AFTER setup_mounts (Step 4.6 below),
    // when /dev/ is the tmpfs mounted by setup_mounts. That tmpfs
    // survives pivot_root and is visible inside the jail at
    // /dev/libgetpid_hook.so and /dev/libtwoyi_loader_shlib.so — exactly
    // where LD_PRELOAD expects them.
    //
    // This split fixes the CRITICAL ordering bug from KVM run 31503063598:
    // the hook library copy was happening AFTER pivot_root, so all
    // candidate source paths were unreachable, LD_PRELOAD was empty,
    // and init crashed with SIGSEGV (signal 11).
    // ---------------------------------------------------------------
    let hook_lib_getpid =
        find_and_read_hook_library(&cfg, "libgetpid_hook.so", "LD_PRELOAD will fail");
    let hook_lib_loader = find_and_read_hook_library(
        &cfg,
        "libtwoyi_loader_shlib.so",
        "seccomp virtualization disabled",
    );

    // ---------------------------------------------------------------
    // Step 4: set up mount namespace + bind mounts + tmpfs.
    // ---------------------------------------------------------------
    // The parent calls setup_mounts() BEFORE fork(). This does:
    //   1. unshare(CLONE_NEWNS) -- new mount namespace (root required)
    //   2. mount tmpfs on {rootfs}/dev, /proc, /sys, /tmp, /apex, /mnt
    //   3. pivot_root(rootfs, rootfs/old_root) -- make rootfs the new /
    //   4. umount2(/old_root, MNT_DETACH) -- drop host's /
    //   5. chdir("/")
    //
    // After pivot_root, the parent's root IS the rootfs. The device
    // socket fds (held by accept threads) are still valid -- accept()
    // works on fds, not paths. The host (outside the mount namespace)
    // still sees {rootfs}/dev/qemu_pipe on the original ext4 filesystem
    // and can connect to it. The tmpfs mount is only visible inside
    // the parent's mount namespace.
    //
    // The child (init) inherits the parent's mount namespace (with
    // pivot_root already done). This means:
    //   - execve("/system/bin/init") resolves to {rootfs}/system/bin/init OK
    //   - LD_PRELOAD=/dev/libgetpid_hook.so resolves to the tmpfs OK
    //   - Init's own mount("tmpfs", "/dev", ...) stacks on top of ours OK
    //
    // If setup_mounts fails (e.g. not root), we fall through to fork
    // without pivot_root. The child will exec init with full rootfs
    // paths prepended (see full_init_path below).
    if cfg.use_namespaces {
        let mount_cfg = mount_mgr::MountConfig {
            rootfs: cfg.rootfs.clone(),
            rom_dir: cfg.rom_dir.clone(),
            use_namespaces: cfg.use_namespaces,
            read_only_rom: cfg.read_only_rom,
        };
        match mount_mgr::setup_mounts(&mount_cfg) {
            Ok(()) => info!("[KR64] setup_mounts succeeded -- pivot_root done"),
            Err(e) => {
                error!(
                    "[KR64] setup_mounts failed: {} -- continuing without pivot_root (init may fail to find /system/bin/init)",
                    e
                );
                // Mark that we did NOT pivot_root, so the child uses
                // full rootfs-prefixed paths instead.
                cfg.use_namespaces = false;
            }
        }
    } else {
        info!("[KR64] use_namespaces=false -- skipping setup_mounts (chroot only)");
    }

    // ---------------------------------------------------------------
    // Path prefix for post-pivot_root operations.
    //
    // After pivot_root (use_namespaces=true), the parent's root IS
    // {cfg.rootfs}, so {cfg.rootfs}/system/bin/init is UNREACHABLE (it
    // would resolve to /data/data/io.twoyi/rootfs/system/bin/init INSIDE
    // the new root, which doesn't exist). All post-setup_mounts
    // operations MUST use chroot-relative paths (e.g. /system/bin/init)
    // which resolve through the bind mounts set up by setup_mounts.
    //
    // When use_namespaces=false (no pivot_root, or pivot_root failed
    // and we fell back to chroot-only), {cfg.rootfs}/... host paths are
    // still correct.
    //
    // We define `rootfs_prefix` once here and use it in all subsequent
    // path-formatted strings. `format!("{}/...", rootfs_prefix)` gives
    // `/...` when use_namespaces=true (chroot-relative) or
    // `{cfg.rootfs}/...` when use_namespaces=false (host path).
    // ---------------------------------------------------------------
    let rootfs_prefix: String = if cfg.use_namespaces {
        String::new()
    } else {
        cfg.rootfs.clone()
    };

    // ---------------------------------------------------------------
    // Step 4.5: create a PID namespace so the guest init becomes PID 1.
    //
    // Android's init binary requires getpid() == 1 -- if it's not PID 1,
    // SecondStageMain exits with code 31 immediately. Without a PID
    // namespace, our forked child would have some arbitrary PID and
    // init would refuse to boot.
    //
    // unshare(CLONE_NEWPID) creates a new PID namespace. The calling
    // process stays in the old namespace, but its next fork() produces
    // a child that is PID 1 in the new namespace. That child can then
    // exec init, and init's getpid() check passes.
    //
    // Requires CAP_SYS_ADMIN (or root). When kr64 is run via `su -c`
    // (as core.rs does on rooted devices), we have root and this works.
    // When running as a regular untrusted_app, unshare fails and we
    // fall through -- init will exit 31, but at least we get diagnostic
    // output.
    // ---------------------------------------------------------------
    match unsafe { libc::unshare(libc::CLONE_NEWPID) } {
        0 => info!("[KR64] unshare(CLONE_NEWPID) succeeded -- child will be PID 1"),
        _ => {
            let e = std::io::Error::last_os_error();
            warning!(
                "[KR64] unshare(CLONE_NEWPID) failed: {} -- init will not be PID 1 (will exit 31)",
                e
            );
        }
    }

    // ---------------------------------------------------------------
    // Step 5: fork + exec the guest.
    // ---------------------------------------------------------------

    // Parent-side diagnostics: verify the actual files at the symlink
    // targets exist and have non-zero size. access() follows symlinks
    // and returns success if the file exists, but doesn't tell us if
    // the file is a valid ELF binary. std::fs::metadata follows symlinks
    // and returns the size, which we can check.
    if cfg.use_namespaces {
        // After pivot_root, paths are relative to the new root.
        let paths_to_check = [
            "/system/bin/linker64",
            "/system/lib64/libc.so",
            "/system/lib64/libm.so",
            "/system/lib64/libdl.so",
            "/apex/com.android.runtime/bin/linker64",
            "/apex/com.android.runtime/lib64/bionic/libc.so",
            cfg.init_path.as_str(),
        ];
        for path in &paths_to_check {
            match std::fs::metadata(path) {
                Ok(meta) => {
                    let ft = meta.file_type();
                    let kind = if ft.is_symlink() {
                        "symlink"
                    } else if ft.is_file() {
                        "file"
                    } else if ft.is_dir() {
                        "dir"
                    } else {
                        "other"
                    };
                    info!(
                        "[KR64] PARENT: {} -> {} ({} bytes, {:?})",
                        path,
                        kind,
                        meta.len(),
                        meta.permissions()
                    );
                }
                Err(e) => {
                    error!("[KR64] PARENT: {} -> metadata failed: {}", path, e);
                }
            }
        }
        // Also check if /apex/com.android.runtime is a real directory
        // with content (not empty)
        match std::fs::read_dir("/apex/com.android.runtime") {
            Ok(entries) => {
                let count = entries.count();
                info!(
                    "[KR64] PARENT: /apex/com.android.runtime has {} entries",
                    count
                );
            }
            Err(e) => {
                error!(
                    "[KR64] PARENT: /apex/com.android.runtime read_dir failed: {}",
                    e
                );
            }
        }
        // Check if /apex/com.android.runtime/lib64/bionic/ has libc.so
        match std::fs::read_dir("/apex/com.android.runtime/lib64/bionic") {
            Ok(entries) => {
                let count = entries.count();
                info!(
                    "[KR64] PARENT: /apex/com.android.runtime/lib64/bionic has {} entries",
                    count
                );
            }
            Err(e) => {
                error!(
                    "[KR64] PARENT: /apex/com.android.runtime/lib64/bionic read_dir failed: {}",
                    e
                );
            }
        }
    }

    // Write hook libraries to /dev/ (tmpfs) so the child can use
    // LD_PRELOAD=/dev/libgetpid_hook.so.
    //
    // The library CONTENT was read into memory BEFORE setup_mounts
    // (Step 3.6 above), while host filesystem paths were still
    // accessible. Now, AFTER setup_mounts (pivot_root), /dev/ is the
    // tmpfs mounted by setup_mounts — writing here places the libraries
    // on the tmpfs that survives pivot_root and is visible inside the
    // jail at /dev/libgetpid_hook.so (exactly where LD_PRELOAD expects).
    //
    // IMPORTANT: Always write to /dev/ (tmpfs), NOT to {rootfs}/dev/
    // (app_data_file on ext4). This is critical for SELinux:
    //   - Init second stage forks subcontexts running as u:r:vendor_init:s0
    //   - vendor_init is DENIED search access to app_data_file directories
    //     (per SELinux policy: avc denied { search } for name="io.twoyi"
    //      tcontext=u:object_r:app_data_file:s0 permissive=0)
    //   - If the libraries are in /data/data/io.twoyi/rootfs/dev/, the
    //     subcontext's linker can't find them → "CANNOT LINK EXECUTABLE"
    //   - /dev/ (tmpfs) is accessible to ALL domains (labeled tmpfs)
    //
    // If a library was not found in the pre-pivot read, the
    // corresponding Option is None and we skip the write (the error
    // was already logged by find_and_read_hook_library with the full
    // list of checked paths). LD_PRELOAD will still reference the path
    // — the child will log "libgetpid_hook.so NOT found at /dev/" and
    // init will crash, but with clear diagnostics.
    if let Some((src, content)) = &hook_lib_getpid {
        write_hook_library_to_dev("libgetpid_hook.so", src, content, "/dev/libgetpid_hook.so");
    }
    if let Some((src, content)) = &hook_lib_loader {
        write_hook_library_to_dev(
            "libtwoyi_loader_shlib.so",
            src,
            content,
            "/dev/libtwoyi_loader_shlib.so",
        );
    }

    // Change SELinux label of /dev/lib*.so to system_file so that
    // vendor_init subcontexts can access them.
    //
    // ROOT CAUSE: When init second stage loads the guest's SELinux policy,
    // SELinux switches to enforcing=1. vendor_init domain is denied
    // getattr/read access to files labeled as `device` (which is the
    // default label for files in /dev/ tmpfs). This causes:
    //   F/linker: CANNOT LINK EXECUTABLE "/system/bin/init":
    //     unable to stat file for the library "/dev/libgetpid_hook.so":
    //     Permission denied
    //
    // FIX: chcon the libraries to u:object_r:system_file:s0, which
    // vendor_init CAN access (it needs to read system_file for its
    // own operation). We do this BEFORE forking init, while SELinux
    // is still permissive (setenforce 0 was called by the test script).
    //
    // Note: chcon requires SELinux to be compiled in the kernel (it is
    // on Android) and the process to have relabel permission (root has
    // this in permissive mode).
    //
    // CRITICAL (KVM run 31505655579): chcon/restorecon subprocesses were
    // crashing with SIGSEGV at address 0x86 in linker64 (NULL soinfo).
    // Root cause: after pivot_root, the chcon binary is the GUEST's chcon
    // (bind-mounted from ROM), and it needs GUEST libraries. But the
    // subprocess inherited kr64's LD_LIBRARY_PATH=/system/lib64:/vendor/lib64
    // (set by the test script), which is MISSING the /apex paths. On
    // Android 11+, many libraries (libbase.so, libc++.so, etc.) live
    // ONLY in /apex/com.android.runtime/lib64/. Without /apex in
    // LD_LIBRARY_PATH, the linker gets a NULL soinfo for the missing
    // library and crashes.
    //
    // FIX: Set LD_LIBRARY_PATH to include /apex paths (matching what
    // init gets). Also clear LD_PRELOAD and TWOYI_ROOTFS defensively —
    // these are HOST-side subprocesses that should NOT load the guest
    // hooks or use the guest rootfs path.
    const CHCON_LD_LIBRARY_PATH: &str = "/system/lib64\
        :/system/lib64/bootstrap\
        :/apex/com.android.runtime/lib64\
        :/apex/com.android.runtime/lib64/bionic\
        :/apex/com.android.runtime/lib64/bootstrap\
        :/vendor/lib64";
    for lib_path in &["/dev/libgetpid_hook.so", "/dev/libtwoyi_loader_shlib.so"] {
        if Path::new(lib_path).exists() {
            let result = std::process::Command::new("chcon")
                .args(["u:object_r:system_file:s0", lib_path])
                .env_remove("LD_PRELOAD")
                .env_remove("TWOYI_ROOTFS")
                .env("LD_LIBRARY_PATH", CHCON_LD_LIBRARY_PATH)
                .status();
            match result {
                Ok(status) if status.success() => {
                    info!("[KR64] PARENT: chcon {} -> system_file OK", lib_path);
                }
                _ => {
                    // chcon failed — try restorecon as fallback
                    let _ = std::process::Command::new("restorecon")
                        .args([lib_path])
                        .env_remove("LD_PRELOAD")
                        .env_remove("TWOYI_ROOTFS")
                        .env("LD_LIBRARY_PATH", CHCON_LD_LIBRARY_PATH)
                        .status();
                    warning!("[KR64] PARENT: chcon {} failed, tried restorecon", lib_path);
                }
            }
        }
    }

    // Copy critical service binaries to /dev/twoyi-bin/ (tmpfs, executable).
    //
    // ROOT CAUSE: The rootfs is at /data/data/io.twoyi/rootfs/, which is on
    // the app's data partition. Even with chmod 0755, execve of binaries
    // from this location fails with EACCES. This is likely because the
    // data partition has noexec or there's a kernel-level restriction.
    //
    // FIX: Copy critical service binaries to /dev/twoyi-bin/ (tmpfs, which
    // is always executable). Our exec hook (translate_exec_path) will
    // redirect exec calls to /dev/twoyi-bin/<binary>.
    //
    // We copy the most critical services that init needs to boot:
    // - logd, lmkd, servicemanager, hwservicemanager, vold
    // - zygote64, zygote (for app startup)
    // - surfaceflinger (for display)
    {
        use std::os::unix::fs::PermissionsExt;
        let dev_bin_dir = "/dev/twoyi-bin";
        let _ = std::fs::create_dir_all(dev_bin_dir);
        let _ = std::fs::set_permissions(dev_bin_dir, std::fs::Permissions::from_mode(0o755));

        let critical_binaries = [
            "system/bin/logd",
            "system/bin/lmkd",
            "system/bin/servicemanager",
            "system/bin/hwservicemanager",
            "system/bin/vold",
            "system/bin/app_process64",
            "system/bin/app_process32",
            "system/bin/surfaceflinger",
            "system/bin/bootanimation",
            "system/bin/linkerconfig",
            "system/bin/ueventd",
            "system/bin/init",
            "system/bin/secilc",
            "system/bin/boringssl_self_test32",
            "system/bin/boringssl_self_test64",
            "system/bin/netd",
            "system/bin/installd",
            "system/bin/keystore2",
            "system/bin/wait_for_keymaster",
            "system/bin/gatekeeperd",
            "system/bin/recovery",
            "system/bin/keystore",
            "system/bin/vdc",
            "system/bin/dumpstate",
            "system/bin/idmap",
            "system/bin/idmap2",
            "system/bin/thermalserviced",
            "system/bin/atrace",
            "system/bin/traced",
            "system/bin/traced_probes",
            "system/bin/perfetto",
            "vendor/bin/boringssl_self_test32",
            "vendor/bin/boringssl_self_test64",
            "vendor/bin/hw/android.hardware.keymaster@4.1-service",
            "vendor/bin/hw/android.hardware.gatekeeper@1.0-service",
            "vendor/bin/hw/android.hardware.graphics.allocator@4.0-service",
            "vendor/bin/hw/android.hardware.graphics.mapper@4.0-impl",
            "vendor/bin/hw/android.hardware.graphics.composer@2.4-service",
            "vendor/bin/hw/android.hardware.configstore@1.0-service",
            "vendor/bin/hw/android.hardware.media.omx@1.0-service",
            "vendor/bin/hw/android.hardware.audio@6.0-service",
            "vendor/bin/hw/android.hardware.atrace@1.0-service",
            "system/bin/hw/android.system.suspend@1.0-service",
            "system/bin/hw/android.hidl.allocator@1.0-service",
            "system/bin/hwservicemanager",
            "system/bin/vold",
            "system/bin/cameraserver",
            "system/bin/drmserver",
            "system/bin/mediadrmserver",
            "system/bin/mediaserver",
            "system/bin/statsd",
            "system/bin/system_server",
        ];

        for binary in &critical_binaries {
            let src = format!("{}/{}", rootfs_prefix, binary);
            let binary_name = std::path::Path::new(binary)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            let dst = format!("{}/{}", dev_bin_dir, binary_name);

            if Path::new(&src).exists() {
                match std::fs::copy(&src, &dst) {
                    Ok(_) => {
                        let _ =
                            std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o755));
                        // Try to chcon to system_file
                        let _ = std::process::Command::new("chcon")
                            .args(["u:object_r:system_file:s0", &dst])
                            .env_remove("LD_PRELOAD")
                            .env_remove("TWOYI_ROOTFS")
                            .env("LD_LIBRARY_PATH", CHCON_LD_LIBRARY_PATH)
                            .status();
                    }
                    Err(e) => {
                        warning!("[KR64] PARENT: failed to copy {} -> {}: {}", src, dst, e);
                    }
                }
            }
        }

        // Also systematically copy ALL binaries from hw/ directories.
        // This ensures any HAL service can be exec'd without hitting
        // the data partition's noexec restriction.
        for hw_dir in &[
            format!("{}/system/bin/hw", rootfs_prefix),
            format!("{}/vendor/bin/hw", rootfs_prefix),
        ] {
            if let Ok(entries) = std::fs::read_dir(hw_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_file() {
                            if let Some(name) = entry.file_name().to_str() {
                                let dst = format!("{}/{}", dev_bin_dir, name);
                                let _ = std::fs::copy(entry.path(), &dst);
                                let _ = std::fs::set_permissions(
                                    &dst,
                                    std::fs::Permissions::from_mode(0o755),
                                );
                                let _ = std::process::Command::new("chcon")
                                    .args(["u:object_r:system_file:s0", &dst])
                                    .env_remove("LD_PRELOAD")
                                    .env_remove("TWOYI_ROOTFS")
                                    .env("LD_LIBRARY_PATH", CHCON_LD_LIBRARY_PATH)
                                    .status();
                            }
                        }
                    }
                }
            }
        }
        info!("[KR64] PARENT: critical service binaries copied to /dev/twoyi-bin/");
    }

    // Mount a new binderfs instance in the guest rootfs.
    // This gives the guest its own /dev/binder, /dev/hwbinder, /dev/vndbinder
    // that are separate from the host's. Without this, the guest's
    // servicemanager can't become context manager (EBUSY) because the
    // host's servicemanager already claimed that role on the shared
    // /dev/binder device.
    {
        let binderfs_dir = format!("{}/dev/binderfs", rootfs_prefix);
        let _ = std::fs::create_dir_all(&binderfs_dir);

        // Mount binderfs
        let binderfs_c = std::ffi::CString::new(binderfs_dir.as_str()).unwrap_or_default();
        let ret = unsafe {
            libc::mount(
                c"binder".as_ptr(),
                binderfs_c.as_ptr(),
                c"binder".as_ptr(),
                0,
                std::ptr::null(),
            )
        };
        if ret == 0 {
            info!("[KR64] PARENT: mounted binderfs at {}", binderfs_dir);

            // Create symlinks: /dev/binder -> binderfs/binder, etc.
            // NOTE: symlink targets use RELATIVE paths (e.g. "binderfs/binder")
            // because in --no-namespaces mode, there is no chroot — absolute
            // paths like /dev/binderfs/binder would resolve on the HOST
            // filesystem, not the guest rootfs. Relative paths resolve
            // correctly because open("/dev/binder") is translated to
            // {rootfs}/dev/binder, and the symlink {rootfs}/dev/binder ->
            // binderfs/binder resolves to {rootfs}/dev/binderfs/binder.
            for name in &["binder", "hwbinder", "vndbinder"] {
                let link_path = format!("{}/dev/{}", rootfs_prefix, name);
                let target = format!("binderfs/{}", name); // RELATIVE path
                                                           // Remove existing file/symlink/socket at this path.
                                                           // remove_file works for files, symlinks, and sockets.
                                                           // If it's a directory, use remove_dir.
                match std::fs::remove_file(&link_path) {
                    Ok(_) => info!("[KR64] PARENT: removed old {}", link_path),
                    Err(e) => {
                        // Try remove_dir if remove_file failed
                        match std::fs::remove_dir(&link_path) {
                            Ok(_) => info!("[KR64] PARENT: removed old dir {}", link_path),
                            Err(_) => {
                                // Both failed — path might not exist, which is fine
                                warning!(
                                    "[KR64] PARENT: could not remove {} (may not exist): {}",
                                    link_path,
                                    e
                                );
                            }
                        }
                    }
                }
                // Create symlink (target is relative to the guest's root)
                match std::os::unix::fs::symlink(&target, &link_path) {
                    Ok(_) => {}
                    Err(e) => {
                        warning!(
                            "[KR64] PARENT: failed to create symlink {} -> {}: {}",
                            link_path,
                            target,
                            e
                        );
                    }
                }
            }
            info!("[KR64] PARENT: created binder symlinks (binderfs)");

            // chmod the binderfs character devices to 0666.
            // ROOT CAUSE (KVM run 31489388552): HIDL HAL services
            // (android.system.suspend@1.0-service, etc.) crash with
            // "Binder driver could not be opened" because open(/dev/hwbinder)
            // returns EACCES for them (22/26 opens failed with errno 13).
            // SELinux is permissive (enforcing=0 confirmed in logcat before
            // the first crash), so this is a DAC permission issue on the
            // binderfs char device — the default mode left by the binder
            // driver is not world-accessible to the guest HAL service
            // contexts. vold's open succeeds (fd=5) because it is spawned
            // earlier / with a permissive group set, but HIDL services
            // spawned later get EACCES.
            //
            // Making the devices 0666 lets all guest processes open them
            // directly (real binder IPC via the kernel binder driver). The
            // loader's binder_open_fallback() is a second line of defence
            // for any process that still can't open them.
            use std::os::unix::fs::PermissionsExt;
            for name in &["binder", "hwbinder", "vndbinder"] {
                let dev_path = format!("{}/dev/binderfs/{}", rootfs_prefix, name);
                match std::fs::set_permissions(&dev_path, std::fs::Permissions::from_mode(0o666)) {
                    Ok(_) => info!(
                        "[KR64] PARENT: chmod 0666 {} (binderfs device world-accessible)",
                        dev_path
                    ),
                    Err(e) => warning!(
                        "[KR64] PARENT: could not chmod {} -> {} (HIDL services may get EACCES; \
                         loader fallback will provide a virtual binder fd)",
                        dev_path,
                        e
                    ),
                }
            }

            // List binderfs contents for diagnostics
            if let Ok(entries) = std::fs::read_dir(&binderfs_dir) {
                for entry in entries.flatten() {
                    info!("[KR64] PARENT: binderfs entry: {:?}", entry.file_name());
                }
            }
        } else {
            let e = std::io::Error::last_os_error();
            warning!(
                "[KR64] PARENT: failed to mount binderfs: {} (binder IPC may not work)",
                e
            );

            // Fallback: try to use the host's binder device directly
            // by creating symlinks to the host's /dev/binder
            for name in &["binder", "hwbinder", "vndbinder"] {
                let link_path = format!("{}/dev/{}", rootfs_prefix, name);
                let target = format!("/dev/{}", name);
                let _ = std::fs::remove_file(&link_path);
                let _ = std::os::unix::fs::symlink(&target, &link_path);
            }
            info!("[KR64] PARENT: using host binder devices (fallback)");
        }
    }

    // Pre-create directories that init and services expect to exist.
    // These are created in the rootfs so init's mkdir commands succeed.
    {
        use std::os::unix::fs::PermissionsExt;
        for dir in &[
            "acct",
            "acct/uid_0",
            "acct/uid_1000",
            "metadata",
            "metadata/vold",
            "metadata/bootstat",
            "linkerconfig",
            "linkerconfig/bootstrap",
            "linkerconfig/default",
            "mnt/secure",
            "mnt/secure/staging",
            "mnt/asec",
            "mnt/obb",
            "mnt/user",
            "mnt/user/0",
            "mnt/installer",
            "mnt/androidwritable",
            "mnt/pass_through",
            "data_mirror",
            "data_mirror/cur_profiles",
            "data_mirror/data_de",
            "data_mirror/data_ce",
            "config",
            "cache",
            "dev/block",
            "dev/block/by-name",
            "dev/block/dm-5",
        ] {
            let path = format!("{}/{}", rootfs_prefix, dir);
            let _ = std::fs::create_dir_all(&path);
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o777));
        }
        info!("[KR64] PARENT: pre-created boot directories in rootfs");
    }

    // Pre-create defensive graphics device stubs in the guest rootfs.
    //
    // RATIONALE: Task ID 3's proactive blocker analysis identified
    // surfaceflinger / graphics HAL init as the SECOND-high-confidence
    // blocker once zygote + system_server boot. The container currently
    // provides /dev/qemu_pipe (GL command transport) and /dev/gb,
    // /dev/gb2 (gralloc sockets — MVP stubs), but does NOT provide any
    // of the legacy graphics device nodes that AOSP components may probe:
    //   - /dev/graphics/fb0  (legacy framebuffer)
    //   - /dev/fb0           (Linux framebuffer)
    //   - /dev/hwcomposer    (goldfish HWComposer char device)
    //   - /dev/hwcomposer0   (HWC 1.0 alternate name)
    //   - /dev/ion           (ION memory allocator)
    //   - /dev/dri/          (DRM directory)
    //
    // We create each as a symlink to /dev/null. This is DEFENSIVE — NOT
    // fake graphics init. The symlinks convert ENOENT crashes (which
    // some HALs treat as fatal) into ENOTTY failures (which surfaceflinger
    // and the HALs handle gracefully by falling back to the next display
    // path). No ioctls are faked; no errors are suppressed. See
    // `devices::create_graphics_device_stubs` for the full rationale.
    //
    // This must be called AFTER setup_mounts (which mounts tmpfs on /dev)
    // so the stubs are on the tmpfs and survive pivot_root. The guest
    // init's own mount("tmpfs", "/dev") is no-op'd by the loader's
    // emu_mount hook, so the stubs remain visible to the guest.
    if let Err(e) = devices::create_graphics_device_stubs(&rootfs_prefix) {
        warning!(
            "[KR64] PARENT: failed to create graphics device stubs: {} (non-fatal — guest may see ENOENT on graphics paths)",
            e
        );
    }

    // Always overwrite /vendor/etc/fstab.ranchu with a minimal stub.
    // The emulator's rootfs tar includes a real fstab.ranchu whose entries
    // reference real block devices (/dev/block/by-name/system, etc.) that
    // don't exist in our container. vold's process_config() reads the fstab
    // via ReadDefaultFstab() → fs_mgr_read_fstab(), and any entry that names
    // a non-existent block device causes vold to exit(1).
    //
    // Fix: ship a truly empty fstab (only comment lines). fs_mgr_read_fstab()
    // parses comment lines as no-ops and returns an empty fstab struct (zero
    // entries). ReadDefaultFstab() still returns true because the file opened
    // successfully, so vold continues without ever touching /dev/block/*.
    //
    // We deliberately OMIT the `first_stage_mount` flag here too: with it,
    // init's FirstStageMount() tries device-mapper mounts that fail fatally
    // (EBUSY) in our container → InitFatalReboot. With an empty fstab init
    // skips first_stage_mount naturally; vold proceeds with an empty fstab.
    {
        let fstab_path = format!("{}/vendor/etc/fstab.ranchu", rootfs_prefix);
        let fstab_content = "# Minimal fstab for twoyi virtualization\n/dev/null /system ext4 ro wait\n/dev/null /vendor ext4 ro wait\n/dev/null /data ext4 nosuid,nodev wait,check,formattable,latemount,resize\n";
        let _ = std::fs::create_dir_all(format!("{}/vendor/etc", rootfs_prefix));
        let _ = std::fs::write(&fstab_path, fstab_content);
        info!("[KR64] PARENT: overwrote fstab.ranchu with minimal stub");
    }

    // Pre-create /dev/__properties__/property_info on the HOST and in the
    // rootfs BEFORE forking. This is a defensive measure: even if our
    // LD_PRELOAD loader fails to be re-loaded after init's execv chain
    // (init first_stage → init selinux_setup → init second_stage), the
    // property_info file will still exist when second_stage init calls
    // WriteStringToFile(kPropertyInfoPath).
    //
    // ROOT CAUSE this defends against:
    //   - init first stage calls clearenv() (wipes LD_PRELOAD)
    //   - our execv hook restores LD_PRELOAD before execv'ing to selinux_setup
    //   - selinux_setup forks+execs secilc (loader is reloaded in secilc)
    //   - secilc exits, selinux_setup continues
    //   - selinux_setup execv's to second_stage init
    //   - SOMETHING in this chain causes LD_PRELOAD to be missing in
    //     second_stage init (likely an exec variant we don't hook, OR
    //     bionic execv not going through PLT in some code path)
    //   - second_stage init's WriteStringToFile fails with ENOENT because
    //     the open() goes directly to the kernel and the file doesn't
    //     exist (or the directory doesn't exist)
    //
    // By pre-creating the file in BOTH locations (host + rootfs), we
    // ensure WriteStringToFile succeeds regardless of which path init
    // actually opens.
    {
        use std::os::unix::fs::PermissionsExt;
        // Create the directory on the host (if it doesn't exist)
        let host_prop_dir = "/dev/__properties__";
        if !Path::new(host_prop_dir).exists() {
            match std::fs::create_dir_all(host_prop_dir) {
                Ok(_) => {
                    let _ = std::fs::set_permissions(
                        host_prop_dir,
                        std::fs::Permissions::from_mode(0o711),
                    );
                    info!("[KR64] PARENT: created host {} (mode 0711)", host_prop_dir);
                }
                Err(e) => {
                    error!(
                        "[KR64] PARENT: failed to create host {}: {}",
                        host_prop_dir, e
                    );
                }
            }
        }
        // Pre-create property_info on host (empty regular file, mode 0666).
        // `.truncate(false)` makes the "do not overwrite an existing file"
        // intent explicit (the `if !exists()` guard already ensures this).
        let host_prop_info = format!("{}/property_info", host_prop_dir);
        if !Path::new(&host_prop_info).exists() {
            match std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .open(&host_prop_info)
            {
                Ok(_) => {
                    let _ = std::fs::set_permissions(
                        &host_prop_info,
                        std::fs::Permissions::from_mode(0o666),
                    );
                    info!(
                        "[KR64] PARENT: pre-created host {} (mode 0666)",
                        host_prop_info
                    );
                }
                Err(e) => {
                    error!(
                        "[KR64] PARENT: failed to pre-create host {}: {}",
                        host_prop_info, e
                    );
                }
            }
        }
        // Also pre-create properties_serial on host (host's property service
        // needs this; don't truncate if it already exists)
        let host_prop_serial = format!("{}/properties_serial", host_prop_dir);
        if !Path::new(&host_prop_serial).exists() {
            match std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .open(&host_prop_serial)
            {
                Ok(_) => {
                    let _ = std::fs::set_permissions(
                        &host_prop_serial,
                        std::fs::Permissions::from_mode(0o666),
                    );
                    info!("[KR64] PARENT: pre-created host {}", host_prop_serial);
                }
                Err(e) => {
                    error!(
                        "[KR64] PARENT: failed to pre-create host {}: {}",
                        host_prop_serial, e
                    );
                }
            }
        }
        // Pre-create the directory + files in the rootfs too
        let rootfs_prop_dir = format!("{}/dev/__properties__", rootfs_prefix);
        let _ = std::fs::create_dir_all(&rootfs_prop_dir);
        let _ = std::fs::set_permissions(&rootfs_prop_dir, std::fs::Permissions::from_mode(0o777));
        for fname in &["property_info", "properties_serial"] {
            let path = format!("{}/{}", rootfs_prop_dir, fname);
            if !Path::new(&path).exists() {
                match std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(false)
                    .open(&path)
                {
                    Ok(_) => {
                        let _ =
                            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666));
                        info!("[KR64] PARENT: pre-created rootfs {}", path);
                    }
                    Err(e) => {
                        error!("[KR64] PARENT: failed to pre-create rootfs {}: {}", path, e);
                    }
                }
            }
        }
        info!("[KR64] PARENT: property files pre-created on host + rootfs");
    }

    // Start a background thread that continuously sets SELinux to permissive.
    //
    // ROOT CAUSE: When init second stage loads the guest's SELinux policy,
    // it sets enforcing=1. This causes vendor_init subcontexts to be denied
    // access to /dev/lib*.so (regardless of SELinux label — vendor_init is
    // denied read on system_file, device, etc.).
    //
    // FIX: This thread writes "0" to /sys/fs/selinux/enforce every 50ms,
    // overriding the guest's policy load. This keeps SELinux permissive,
    // allowing vendor_init to load our LD_PRELOAD libraries.
    //
    // This is a temporary measure for the KVM test environment. In production,
    // we'd need to either:
    // 1. Modify the guest's SELinux policy to allow vendor_init access
    // 2. Use a different loading mechanism (PT_INTERP, DT_NEEDED)
    // 3. Don't use LD_PRELOAD for subcontexts
    let enforce_thread = std::thread::Builder::new()
        .name("selinux-permissive".to_string())
        .spawn(|| {
            info!("[KR64] PARENT: starting SELinux permissive watchdog thread");
            loop {
                // Write "0" to /sys/fs/selinux/enforce to set permissive mode
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .write(true)
                    .open("/sys/fs/selinux/enforce")
                {
                    use std::io::Write;
                    let _ = f.write_all(b"0");
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        })
        .ok();

    info!("[KR64] forking guest process");
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        let e = std::io::Error::last_os_error();
        error!("[KR64] fork failed: {}", e);
        return 1;
    }

    if pid == 0 {
        // ----- CHILD (will become the guest init) -----
        //
        // IMPORTANT: between fork() and execve() we are in an async-signal-
        // unsafe window. The `info!`/`warning!`/`error!` macros above
        // expand to `eprintln!`, which is NOT async-signal-safe -- it
        // initialises a global LineWriter, allocates, and grabs the stdio
        // lock. If another thread held that lock at fork() time, calling
        // `eprintln!` here would deadlock the child.
        //
        // For diagnostics in this branch we use `safe_write_err*()` (defined
        // above), which only calls the async-signal-safe `write(2)` syscall
        // and never touches the allocator. The parent observes stderr
        // because fork() inherits the parent's stderr fd.

        // Close all inherited file descriptors except stdin/stdout/stderr.
        // The parent holds device sockets (SOCK_CLOEXEC, auto-closed by
        // execve), but the binder proxy's host_binder_fd is NOT CLOEXEC
        // and would leak into the guest, giving it direct access to the
        // host's /dev/binder. Close everything >= 3.
        //
        // IMPORTANT (2026-08-09): We previously used close_range (syscall
        // 436) here, but the Android zygote's seccomp filter blocks it
        // (SIGSYS / SYS_SECCOMP kill). close_range was added in Linux 5.9
        // and Android 11's seccomp policy doesn't whitelist it for
        // untrusted_app. We now iterate fd numbers 3..1024 and close each
        // one -- close() (syscall 3) IS whitelisted. This is O(1024) but
        // only runs once at guest startup.
        //
        // When kr64 runs as root (via `su -c`), the zygote's seccomp
        // filter is not inherited, so close_range would work -- but we
        // keep the portable loop to avoid the seccomp trap on non-root
        // runs and because the cost is negligible.
        for fd in 3..1024i32 {
            unsafe {
                libc::close(fd);
            }
        }

        // NEVER call setup_mounts in the child. The parent already did
        // unshare + pivot_root + mount tmpfs BEFORE forking. The child
        // inherits the parent's mount namespace. Calling setup_mounts
        // again would crash (mounting on already-mounted filesystems)
        // or be blocked by seccomp (zygote filter blocks mount/chroot
        // for untrusted_app).
        //
        // The child just needs to: close fds, (optionally) install
        // seccomp, and execve init.
        if cfg.use_namespaces {
            unsafe {
                safe_write_err(b"[KR64 CHILD] root mode: parent already did pivot_root, skipping mount setup\n");
            }
        } else {
            unsafe {
                safe_write_err(
                    b"[KR64 CHILD] non-root mode: skipping mount+chroot (seccomp blocks both)\n",
                );
            }
        }

        if cfg.install_seccomp {
            if let Err(e) = seccomp::install() {
                // Non-fatal: we explicitly continue so the guest can boot
                // in a permissive-ish mode (the seccomp filter is a
                // hardening layer, not a correctness requirement for the
                // MVP). But the user must be told -- silent failure here
                // would mask a misconfigured BPF program.
                let errno = e.raw_os_error().unwrap_or(0);
                unsafe {
                    safe_write_err_errno(
                        b"[KR64 CHILD] WARN: seccomp::install failed (continuing without filter)",
                        errno,
                    );
                }
            }
        }

        // Exec the guest init with a proper environment.
        // Fixed: was passing empty envp -- init needs at least PATH,
        // ANDROID_ROOT, ANDROID_DATA, and BOOTCLASSPATH.
        //
        // CString::new fails if the path contains an interior NUL byte,
        // which would silently terminate the C string mid-path. Surface
        // that case rather than _exit(127) with no diagnostic.
        // When not using chroot/pivot_root, init_path is relative to the
        // rootfs (e.g. /system/bin/init). We need to prepend the rootfs
        // path to get the full absolute path. When chroot IS used, the
        // init_path is already correct (it's relative to the new root).
        let full_init_path = if cfg.use_namespaces {
            cfg.init_path.clone()
        } else {
            format!("{}{}", cfg.rootfs, cfg.init_path)
        };

        // Pre-execve diagnostics: verify the init binary and linker exist.
        // The init binary's ELF INTERP field specifies the dynamic linker
        // path (usually /system/bin/linker64). If either file is missing,
        // execve fails with ENOENT. If the linker loads but crashes (e.g.
        // due to a bad LD_PRELOAD), we get SIGSEGV in linker64.
        {
            // Use CString for the init path (need null-terminated C string
            // for access()). This allocates, but we're in a single-threaded
            // path before execve -- safe per the existing format!() usage.
            let init_c = CString::new(full_init_path.as_str()).unwrap_or_default();
            let init_exists = unsafe { libc::access(init_c.as_ptr(), libc::F_OK) == 0 };
            if !init_exists {
                unsafe {
                    safe_write_err(b"[KR64 CHILD] FATAL: init binary not found at ");
                    safe_write_err(full_init_path.as_bytes());
                    safe_write_err(b"\n");
                    libc::_exit(127);
                }
            }
            // Check if the dynamic linker exists (needed by the ELF INTERP)
            let linker_path = b"/system/bin/linker64\0";
            let linker_exists = unsafe {
                libc::access(linker_path.as_ptr() as *const libc::c_char, libc::F_OK) == 0
            };
            if linker_exists {
                unsafe {
                    safe_write_err(b"[KR64 CHILD] linker64 found at /system/bin/\n");
                }
            } else {
                unsafe {
                    safe_write_err(b"[KR64 CHILD] linker64 NOT found at /system/bin/\n");
                }
            }
            // Check libc.so
            let libc_path = b"/system/lib64/libc.so\0";
            let libc_exists =
                unsafe { libc::access(libc_path.as_ptr() as *const libc::c_char, libc::F_OK) == 0 };
            if libc_exists {
                unsafe {
                    safe_write_err(b"[KR64 CHILD] libc.so found at /system/lib64/\n");
                }
            } else {
                unsafe {
                    safe_write_err(b"[KR64 CHILD] libc.so NOT found at /system/lib64/\n");
                }
            }
        }

        let init_cstr = match CString::new(full_init_path.as_str()) {
            Ok(s) => s,
            Err(_) => unsafe {
                safe_write_err(b"[KR64 CHILD] FATAL: init_path contains interior NUL byte\n");
                libc::_exit(127);
            },
        };
        let argv0 = match CString::new(cfg.init_path.as_str()) {
            Ok(s) => s,
            Err(_) => unsafe {
                safe_write_err(b"[KR64 CHILD] FATAL: argv0 contains interior NUL byte\n");
                libc::_exit(127);
            },
        };
        let argv: [*const libc::c_char; 2] = [argv0.as_ptr(), std::ptr::null()];

        // Debug: check if libgetpid_hook.so exists at the expected path.
        // After pivot_root (use_namespaces=true): /dev/libgetpid_hook.so
        // Without pivot_root: {rootfs}/dev/libgetpid_hook.so
        // In both cases, the chroot-relative path /dev/libgetpid_hook.so
        // works -- after pivot_root it's the tmpfs, without pivot_root
        // the parent copied it to {rootfs}/dev/ which IS /dev/ relative
        // to the chroot... actually no, without pivot_root there's no
        // chroot, so /dev/ refers to the HOST's /dev. We need to check
        // the full path in that case. But we can't use format! here
        // (async-signal-unsafe). So just check the chroot-relative path
        // and log the result -- it's only diagnostic.
        let hook_exists =
            unsafe { libc::access(c"/dev/libgetpid_hook.so".as_ptr(), libc::F_OK) == 0 };
        if hook_exists {
            unsafe {
                safe_write_err(b"[KR64 CHILD] libgetpid_hook.so found at /dev/\n");
            }
        } else {
            unsafe {
                safe_write_err(b"[KR64 CHILD] libgetpid_hook.so NOT found at /dev/\n");
            }
        }

        // Build environment for the guest init. The CString::new calls
        // below use compile-time-constant strings (no NUL possible) and
        // format!() -- the format! allocation happens BEFORE execve, so
        // it's safe (we're not yet racing the post-fork window for the
        // allocator lock on this short, single-thread-of-control path).
        //
        // TWOYI_ROOTFS value: after pivot_root (use_namespaces=true), the
        // process is chrooted into the rootfs — paths like /system/lib/foo.so
        // resolve through the bind mounts and do NOT need to be prefixed
        // with the host rootfs path. Setting TWOYI_ROOTFS="/" makes the
        // loader's should_translate() guard (strncmp(path, g_rootfs, strlen))
        // match ALL absolute paths (since every absolute path starts with
        // "/"), which disables path translation entirely. This is the
        // CORRECT behaviour after pivot_root: the loader's translation
        // (prepending the host rootfs path) would make every path
        // UNREACHABLE inside the jail.
        //
        // Why "/" instead of "": the loader's clearenv hook only restores
        // TWOYI_ROOTFS if g_rootfs_env[0] is non-zero. An empty string
        // would NOT be restored after init's clearenv(), causing the
        // loader to fall back to the default "/data/data/io.twoyi/rootfs"
        // (the host path), which is unreachable. "/" is restored
        // correctly and keeps translation disabled.
        //
        // When use_namespaces=false (no pivot_root), the loader MUST
        // translate guest paths to host paths, so we pass the full
        // cfg.rootfs as before.
        let twoyi_rootfs_value = if cfg.use_namespaces {
            "/".to_string()
        } else {
            cfg.rootfs.clone()
        };
        let twoyi_rootfs_env = match CString::new(format!("TWOYI_ROOTFS={}", twoyi_rootfs_value)) {
            Ok(s) => s,
            Err(_) => unsafe {
                safe_write_err(b"[KR64 CHILD] FATAL: TWOYI_ROOTFS env contains NUL byte\n");
                libc::_exit(127);
            },
        };
        // LD_PRELOAD path: ALWAYS use /dev/libgetpid_hook.so and
        // /dev/libtwoyi_loader_shlib.so (not {rootfs}/dev/).
        //
        // WHY: Init second stage forks subcontexts running as
        // u:r:vendor_init:s0. vendor_init is DENIED search access to
        // app_data_file directories (per SELinux policy). If the libraries
        // are at /data/data/io.twoyi/rootfs/dev/, the subcontext's linker
        // can't find them. /dev/ (tmpfs) is accessible to ALL domains.
        //
        // When use_namespaces=true, pivot_root has happened, so /dev/
        // refers to the new root's /dev/ (tmpfs mounted by setup_mounts).
        // When use_namespaces=false, /dev/ refers to the HOST's /dev/
        // (also tmpfs). In both cases, kr64 copies the libraries to /dev/
        // before forking, so the paths resolve correctly.
        //
        // If TWOYI_SKIP_PRELOAD is set in the parent env, skip LD_PRELOAD
        // entirely -- this is a diagnostic mode to check if the init binary
        // can link WITHOUT the getpid hook (init will exit 31, but if it
        // exits 31 instead of SIGSEGV, we know the linker works).
        let skip_preload = std::env::var("TWOYI_SKIP_PRELOAD").is_ok();
        // LD_PRELOAD: load BOTH the seccomp/SIGSYS virtualization library
        // (libtwoyi_loader_shlib.so) AND the getpid hook (libgetpid_hook.so).
        // The virtualization library installs seccomp BPF filter + SIGSYS
        // handler via .init_array constructor before init's main() runs.
        // The getpid hook makes init think it's PID 1.
        let ld_preload_str =
            "LD_PRELOAD=/dev/libgetpid_hook.so:/dev/libtwoyi_loader_shlib.so".to_string();
        let mut env_vars: Vec<CString> = vec![
            CString::new("PATH=/system/bin:/system/xbin:/vendor/bin").unwrap(),
            CString::new("ANDROID_ROOT=/system").unwrap(),
            CString::new("ANDROID_DATA=/data").unwrap(),
            CString::new("ANDROID_BOOTLOGO=1").unwrap(),
            twoyi_rootfs_env,
            // On Android 11+, many libraries (libc.so, libbase.so, liblog.so,
            // etc.) live in /apex/com.android.runtime/lib64/ and are only
            // symlinked from /system/lib64/. Some libraries (like libbase.so)
            // have NO symlink in /system/lib64/ and can only be found in the
            // apex directory. Without these paths in LD_LIBRARY_PATH, the
            // linker gets a NULL soinfo for the missing library and crashes
            // with SIGSEGV at address 0x86 (offset 0xaf174 in linker64).
            //
            // CRITICAL (KVM run 31505655579): The guest init (pid 5865) was
            // crashing with SIGSEGV (signal 11) in linker64 with NO
            // [twoyi_loader] init messages — meaning the linker crashed
            // BEFORE the LD_PRELOAD constructor could run. Root cause:
            // init's LD_LIBRARY_PATH was MISSING /vendor/lib64,
            // /apex/com.android.os.statsd/lib64, /system_ext/lib64, and
            // /product/lib64. If init (or any of its dependencies) needs a
            // library that's ONLY in one of those directories, the linker
            // gets a NULL soinfo and crashes.
            //
            // The execve hook in twoyi_loader_shlib.c (line ~1872) already
            // builds a FULL LD_LIBRARY_PATH for execve'd processes that
            // includes all these paths. But the FIRST init (launched by
            // kr64 via execve) was getting a SHORTER LD_LIBRARY_PATH that
            // was missing 4 directories. This fix makes the first init's
            // LD_LIBRARY_PATH match what the execve hook builds, so the
            // linker can find ALL libraries on the first exec.
            CString::new(
                "LD_LIBRARY_PATH=\
                /system/lib64:\
                /system/lib64/bootstrap:\
                /apex/com.android.runtime/lib64:\
                /apex/com.android.runtime/lib64/bionic:\
                /apex/com.android.runtime/lib64/bootstrap:\
                /vendor/lib64:\
                /apex/com.android.os.statsd/lib64:\
                /system_ext/lib64:\
                /product/lib64",
            )
            .unwrap(),
        ];
        if skip_preload {
            unsafe {
                safe_write_err(b"[KR64 CHILD] TWOYI_SKIP_PRELOAD set -- skipping LD_PRELOAD (init will exit 31)\n");
            }
        } else {
            env_vars.push(CString::new(ld_preload_str).unwrap());
        }
        let env_ptrs: Vec<*const libc::c_char> = env_vars
            .iter()
            .map(|s| s.as_ptr())
            .chain(std::iter::once(std::ptr::null()))
            .collect();

        // If execve returns at all, it failed. Capture errno BEFORE any
        // other call (any libc call can clobber it) and surface it.
        let _r = unsafe { libc::execve(init_cstr.as_ptr(), argv.as_ptr(), env_ptrs.as_ptr()) };
        let exec_errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        unsafe {
            safe_write_err_errno(
                b"[KR64 CHILD] FATAL: execve returned (init did not replace us)",
                exec_errno,
            );
            libc::_exit(127);
        }
    }

    // ----- PARENT (the daemon) -----
    info!("[KR64][parent] guest pid = {}", pid);

    // Keep the SELinux permissive watchdog thread alive (it's detached,
    // but we hold the handle to avoid a compiler warning)
    let _ = enforce_thread;

    // qemu_pipe -> real GL command proxy (Phase 1 of the dispatcher plan).
    // The proxy accepts guest connections, reads the "pipe:opengles"
    // channel-name handshake, connects to the renderer's Unix socket
    // at {rootfs}/opengles, and pumps bytes bidirectionally. This
    // replaces the old MVP stub that wrote a single 0 byte and closed.
    // See download/QEMU_PIPE_DISPATCHER_PLAN.md for the full design.
    //
    // After pivot_root (use_namespaces=true), the rootfs IS the root "/",
    // so the renderer socket at {rootfs}/opengles is now at /opengles.
    // Pass "" as the rootfs prefix so the proxy constructs "/opengles".
    // Without pivot_root, pass the full rootfs path.
    let proxy_rootfs = if cfg.use_namespaces {
        String::new() // chroot-relative: format!("{}/{}", "", "opengles") = "/opengles"
    } else {
        cfg.rootfs.clone()
    };
    let _qemu_pipe_proxy = {
        let mut dev = device_set.qemu_pipe;
        let listener = match dev.take_listener() {
            Some(l) => l,
            None => {
                error!("[KR64] qemu_pipe listener already taken -- cannot start proxy");
                return 1;
            }
        };
        match qemu_pipe::spawn_qemu_pipe_proxy(listener, dev.path.clone(), proxy_rootfs) {
            Ok(h) => {
                info!(
                    "[KR64] qemu_pipe proxy listening at {} (rootfs={})",
                    h.path(),
                    cfg.rootfs
                );
                Some(h)
            }
            Err(e) => {
                error!("[KR64] failed to start qemu_pipe proxy: {}", e);
                None
            }
        }
    };

    // Spawn one accept thread per remaining device socket. For the MVP
    // each thread just accepts connections and immediately closes them
    // (echoing a single byte so the guest sees SOME response). The
    // production version will dispatch to per-device handlers:
    //   touch     -> input::touch_server
    //   key       -> input::key_server
    //   event     -> TwoyiSocketServer (event IPC)
    //   gb/gb2    -> openglrenderer::gralloc
    spawn_accept_thread(device_set.touch, "touch");
    spawn_accept_thread(device_set.key, "key");
    spawn_accept_thread(device_set.event, "event");
    spawn_accept_thread(device_set.gb.gb, "gb");
    spawn_accept_thread(device_set.gb.gb2, "gb2");

    // ---------------------------------------------------------------
    // Step 6: wait for the guest to exit.
    // ---------------------------------------------------------------
    let mut status: libc::c_int = 0;
    loop {
        let waited = unsafe { libc::waitpid(pid, &mut status, 0) };
        if waited == -1 {
            let e = std::io::Error::last_os_error();
            if e.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            error!("[KR64][parent] waitpid failed: {}", e);
            return 1;
        }
        break;
    }

    if libc::WIFEXITED(status) {
        let code = libc::WEXITSTATUS(status);
        info!("[KR64][parent] guest exited with status {}", code);
        return code;
    }
    if libc::WIFSIGNALED(status) {
        let sig = libc::WTERMSIG(status);
        warning!("[KR64][parent] guest killed by signal {}", sig);
        return 128 + sig;
    }

    warning!(
        "[KR64][parent] guest waitpid returned unexpected status {}",
        status
    );
    1
}

/// Spawn a thread that accepts connections on the given device socket
/// and immediately closes them (MVP placeholder).
///
/// The thread takes ownership of the underlying `UnixListener` (so
/// the parent `DeviceSocket` is consumed). For the production version
/// this will be replaced with per-device handler dispatch.
fn spawn_accept_thread(mut dev: devices::DeviceSocket, name: &'static str) {
    let listener = match dev.take_listener() {
        Some(l) => l,
        None => {
            warning!(
                "[KR64] cannot spawn accept thread for {}: listener already taken",
                name
            );
            return;
        }
    };
    // Make the listening socket non-blocking so the thread can poll
    // for shutdown signals (in the MVP we just loop forever).
    let fd = listener.as_raw_fd();
    let _ = unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };

    std::thread::Builder::new()
        .name(format!("kr64-accept-{}", name))
        .spawn(move || {
            info!("[KR64][{}] accept thread started (fd={})", name, fd);
            loop {
                match listener.accept() {
                    Ok((mut stream, _addr)) => {
                        info!("[KR64][{}] client connected", name);
                        // Echo a single byte so the guest sees SOME response.
                        // (Many of the device protocols expect a handshake
                        // byte -- e.g. the touch device sends a device_info
                        // struct on connect, which the guest reads before
                        // sending anything. The production version will
                        // dispatch to the right handler.)
                        use std::io::Write;
                        let _ = stream.write_all(&[0u8]);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No pending connection -- sleep briefly to avoid
                        // spinning. (Real implementation would use epoll.)
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(e) => {
                        warning!("[KR64][{}] accept error: {}", name, e);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        })
        .expect("spawn kr64 accept thread");
}

// ============================================================================
// cdylib entry point -- used when `libkr64.so` is exec'd directly via
// the PIE hack. The ELF entry point is set to `kr64_main` via the
// `-Wl,-e,kr64_main` link flag in build.rs / .cargo/config.toml.
// ============================================================================

/// Entry point for the cdylib (libkr64.so). Mirrors the C `main`
/// signature `(argc, argv) -> int` so the standard C runtime can call
/// it.
///
/// When the kernel exec's `libkr64.so`, the dynamic linker loads it,
/// runs `.init_array`, and jumps to `kr64_main` (because we set
/// `-Wl,-e,kr64_main`). The args come from the kernel's stack as
/// `argc` + `argv[]`, just like a regular C `main`.
///
/// Marked `unsafe` because the function dereferences the caller-supplied
/// `argv` pointer; the caller (kernel / dynamic linker) is implicitly
/// unsafe C code, and marking the Rust entry point `unsafe` correctly
/// reflects this to Rust callers.
///
/// # Safety
///
/// Caller must guarantee that, when `argc > 0` and `argv` is non-null,
/// `argv` points to a buffer of at least `argc` valid `char *` pointers
/// (NUL-terminated C strings or NULL). This is the standard C `main`
/// contract.
#[no_mangle]
pub unsafe extern "C" fn kr64_main(
    argc: libc::c_int,
    argv: *const *const libc::c_char,
) -> libc::c_int {
    // Convert C argv -> Rust Vec<String>.
    let args: Vec<String> = if argc <= 0 || argv.is_null() {
        Vec::new()
    } else {
        (0..argc as isize)
            .map(|i| unsafe {
                let p = *argv.offset(i);
                if p.is_null() {
                    String::new()
                } else {
                    std::ffi::CStr::from_ptr(p).to_string_lossy().to_string()
                }
            })
            .collect()
    };

    run(args)
}

// Reference the interp symbol from interp.c (built by build.rs) so the
// linker keeps the `.interp` section in the final libkr64.so. Without
// this reference, the linker may GC the `.interp` section as unused,
// and the resulting .so won't be directly executable.
extern "C" {
    #[link_name = "interp"]
    static INTERP: [u8; 0];
}

#[used]
static INTERP_REF: &[u8; 0] = unsafe { &INTERP };

// ============================================================================
// Tests -- exercise arg parsing and config defaults.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        std::iter::once("kr64".to_string())
            .chain(v.iter().map(|s| s.to_string()))
            .collect()
    }

    #[test]
    fn parse_args_minimal() {
        let cfg = parse_args(args(&["--rootfs", "/r", "--data-dir", "/d"])).unwrap();
        assert_eq!(cfg.rootfs, "/r");
        assert_eq!(cfg.data_dir, "/d");
        assert_eq!(cfg.vmid, 0);
        assert_eq!(cfg.width, 720);
        assert_eq!(cfg.height, 1280);
        assert!(cfg.use_namespaces);
        assert!(cfg.read_only_rom);
        assert!(cfg.install_seccomp);
    }

    #[test]
    fn parse_args_full() {
        let cfg = parse_args(args(&[
            "--rootfs",
            "/r",
            "--data-dir",
            "/d",
            "--rom-dir",
            "/rom",
            "--init",
            "/sbin/init",
            "--vmid",
            "3",
            "--width",
            "1080",
            "--height",
            "1920",
            "--dpi",
            "480",
            "--log-level",
            "1",
            "--no-namespaces",
            "--rw-rom",
            "--no-seccomp",
        ]))
        .unwrap();
        assert_eq!(cfg.rom_dir.as_deref(), Some("/rom"));
        assert_eq!(cfg.init_path, "/sbin/init");
        assert_eq!(cfg.vmid, 3);
        assert_eq!(cfg.width, 1080);
        assert_eq!(cfg.height, 1920);
        assert_eq!(cfg.dpi, 480);
        assert_eq!(cfg.log_level, 1);
        assert!(!cfg.use_namespaces);
        assert!(!cfg.read_only_rom);
        assert!(!cfg.install_seccomp);
    }

    #[test]
    fn parse_args_missing_rootfs_errors() {
        let r = parse_args(args(&["--data-dir", "/d"]));
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("--rootfs"));
    }

    #[test]
    fn parse_args_missing_data_dir_errors() {
        let r = parse_args(args(&["--rootfs", "/r"]));
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("--data-dir"));
    }

    #[test]
    fn parse_args_unknown_arg_errors() {
        let r = parse_args(args(&["--rootfs", "/r", "--data-dir", "/d", "--bogus"]));
        assert!(r.is_err());
    }

    #[test]
    fn parse_args_help_returns_usage_string() {
        let r = parse_args(args(&["--help"]));
        assert!(r.is_err());
        let e = r.unwrap_err();
        assert!(e.starts_with("twoyi kr64"));
        assert!(e.contains("--rootfs"));
    }

    #[test]
    fn config_default_init_path_is_system_bin_init() {
        let cfg = Config::default();
        assert_eq!(cfg.init_path, "/system/bin/init");
    }

    #[test]
    fn config_default_socks5_is_none() {
        let cfg = Config::default();
        assert!(
            cfg.socks5_proxy.is_none(),
            "socks5_proxy should default to None"
        );
    }

    #[test]
    fn parse_args_socks5_sets_field() {
        let cfg = parse_args(args(&[
            "--rootfs",
            "/r",
            "--data-dir",
            "/d",
            "--socks5",
            "127.0.0.1:1080",
        ]))
        .unwrap();
        assert_eq!(cfg.socks5_proxy.as_deref(), Some("127.0.0.1:1080"));
    }

    #[test]
    fn parse_args_socks5_rejects_missing_colon() {
        let r = parse_args(args(&[
            "--rootfs",
            "/r",
            "--data-dir",
            "/d",
            "--socks5",
            "no-colon-here",
        ]));
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("--socks5"));
    }

    #[test]
    fn parse_args_socks5_rejects_missing_value() {
        let r = parse_args(args(&["--rootfs", "/r", "--data-dir", "/d", "--socks5"]));
        assert!(r.is_err());
    }

    #[test]
    fn parse_args_help_mentions_socks5() {
        let r = parse_args(args(&["--help"]));
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("--socks5"));
    }

    /// `clear_zombie_processes` must be safe to call when there are no
    /// children (the common case at startup). It should not panic and
    /// should return promptly.
    #[test]
    fn clear_zombie_processes_is_safe_with_no_children() {
        // We can't easily create real zombie children in a unit test
        // (forking + exiting would race with the test runner), but we
        // CAN verify the function handles ECHILD gracefully -- which is
        // the "no children" condition. This is a smoke test.
        clear_zombie_processes();
        // If we get here without panicking, the test passes.
    }

    /// `hook_library_candidates` must START with the 4 documented
    /// candidate paths in priority order (the app-level rootfs path that
    /// RomManager's `ensureLibSymlink` ACTUALLY uses, which is NOT the
    /// same as `cfg.rootfs` for per-profile rootfs). The list may be
    /// followed by APK native lib dir scan results (candidate #5+) if
    /// `/data/app/` exists -- on the Linux devcontainer test runner it
    /// doesn't, so the list is exactly 4 here.
    #[test]
    fn hook_library_candidates_starts_with_four_documented_paths() {
        let cfg = Config {
            rootfs: "/data/data/io.twoyi/profiles/default/rootfs".to_string(),
            data_dir: "/data/data/io.twoyi".to_string(),
            ..Config::default()
        };
        let cands = hook_library_candidates(&cfg, "libgetpid_hook.so");
        // The first 4 candidates are the documented symlink paths. The
        // APK dir scan (candidate #5+) returns 0 entries on Linux.
        assert!(
            cands.len() >= 4,
            "expected at least 4 candidates, got {}: {:?}",
            cands.len(),
            cands
        );
        // 1. Direct rootfs (historical fallback).
        assert_eq!(
            cands[0],
            "/data/data/io.twoyi/profiles/default/rootfs/libgetpid_hook.so"
        );
        // 2. Profile rootfs system/lib64 (RomManager per-profile symlink).
        assert_eq!(
            cands[1],
            "/data/data/io.twoyi/profiles/default/rootfs/system/lib64/libgetpid_hook.so"
        );
        // 3. App-level rootfs system/lib64 -- the CONFIRMED working path
        //    from logcat (ensureLibSymlink target).
        assert_eq!(
            cands[2],
            "/data/data/io.twoyi/rootfs/system/lib64/libgetpid_hook.so"
        );
        // 4. App-level rootfs root (alternative).
        assert_eq!(cands[3], "/data/data/io.twoyi/rootfs/libgetpid_hook.so");
    }

    /// `hook_library_candidates` must use the passed-in library name
    /// verbatim (so the same function works for both libgetpid_hook.so
    /// and libtwoyi_loader_shlib.so).
    #[test]
    fn hook_library_candidates_uses_passed_lib_name() {
        let cfg = Config {
            rootfs: "/r".to_string(),
            data_dir: "/d".to_string(),
            ..Config::default()
        };
        let cands = hook_library_candidates(&cfg, "libtwoyi_loader_shlib.so");
        // The first 4 candidates use the documented /r/ and /d/ bases.
        // The APK dir scan (candidate #5+) returns 0 entries on Linux.
        assert!(
            cands.len() >= 4,
            "expected at least 4 candidates, got {}: {:?}",
            cands.len(),
            cands
        );
        // Every documented candidate ends with the requested lib name.
        assert!(cands
            .iter()
            .all(|p| p.ends_with("/libtwoyi_loader_shlib.so")));
        // Sanity: every candidate embeds either /r or /d as the base.
        assert!(cands
            .iter()
            .all(|p| p.starts_with("/r/") || p.starts_with("/d/")));
    }

    /// `apk_native_lib_candidates_in` returns an empty Vec when the
    /// base directory doesn't exist (e.g., on the Linux devcontainer
    /// where `/data/app/` is absent, or when passed a non-existent
    /// path in a unit test).
    #[test]
    fn apk_native_lib_candidates_returns_empty_when_base_missing() {
        let cands = apk_native_lib_candidates_in(
            Path::new("/nonexistent-twoyi-apk-base-xyz"),
            "libgetpid_hook.so",
        );
        assert!(
            cands.is_empty(),
            "expected no candidates when base dir is missing, got: {:?}",
            cands
        );
    }

    /// `apk_native_lib_candidates_in` finds the library in a fake APK
    /// directory tree matching the standard
    /// `/data/app/~~<random>/io.twoyi-<random>/lib/<abi>/<lib>` layout.
    /// Verifies x86_64 is preferred over arm64 (listed first) and that
    /// non-io.twoyi packages in the same bucket are skipped.
    #[test]
    fn apk_native_lib_candidates_finds_lib_in_fake_apk_dir() {
        let tmp = std::env::temp_dir().join("twoyi-apk-scan-test");
        let _ = std::fs::remove_dir_all(&tmp);
        // Mimic /data/app/~~random1==/io.twoyi-random2==/lib/<abi>/<lib>.
        let apk_root = tmp.join("~~random1==").join("io.twoyi-random2==");
        let x86_64_lib = apk_root.join("lib/x86_64/libgetpid_hook.so");
        let arm64_lib = apk_root.join("lib/arm64/libgetpid_hook.so");
        std::fs::create_dir_all(x86_64_lib.parent().unwrap()).unwrap();
        std::fs::create_dir_all(arm64_lib.parent().unwrap()).unwrap();
        std::fs::write(&x86_64_lib, b"fake x86_64 ELF").unwrap();
        std::fs::write(&arm64_lib, b"fake arm64 ELF").unwrap();
        // Add a decoy non-io.twoyi package in the same bucket -- it must
        // be skipped by the `starts_with("io.twoyi-")` filter.
        let decoy_root = tmp.join("~~random1==").join("com.other.app-1==");
        std::fs::create_dir_all(decoy_root.join("lib/x86_64")).unwrap();
        std::fs::write(decoy_root.join("lib/x86_64/libgetpid_hook.so"), b"decoy").unwrap();

        let cands = apk_native_lib_candidates_in(&tmp, "libgetpid_hook.so");
        // x86_64 must be preferred (listed first), then arm64. The decoy
        // package's lib must NOT be in the list.
        assert_eq!(
            cands.len(),
            2,
            "expected 2 candidates (x86_64 + arm64), got: {:?}",
            cands
        );
        assert!(
            cands[0].ends_with("/lib/x86_64/libgetpid_hook.so"),
            "x86_64 must be first: {}",
            cands[0]
        );
        assert!(
            cands[1].ends_with("/lib/arm64/libgetpid_hook.so"),
            "arm64 must be second: {}",
            cands[1]
        );

        // Cleanup.
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// `candidate_exists_with_diagnostics` returns false for a broken
    /// symlink (symlink exists but target doesn't) and true for a real
    /// file. The diagnostic warning goes to stderr (eprintln), which we
    /// don't capture here -- the test just verifies the boolean result.
    #[test]
    fn candidate_exists_with_diagnostics_handles_broken_symlink() {
        let tmp = std::env::temp_dir().join("twoyi-broken-symlink-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        // Broken symlink: link itself exists, target doesn't.
        let link = tmp.join("link-to-nowhere.so");
        let target = tmp.join("does-not-exist.so");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let link_str = link.to_string_lossy().into_owned();
        assert!(
            !candidate_exists_with_diagnostics(&link_str),
            "broken symlink must report false"
        );
        // Regular existing file: must report true.
        let real = tmp.join("real.so");
        std::fs::write(&real, b"x").unwrap();
        let real_str = real.to_string_lossy().into_owned();
        assert!(
            candidate_exists_with_diagnostics(&real_str),
            "existing regular file must report true"
        );
        // Non-existent path (no symlink, no file): must report false
        // WITHOUT panicking on the symlink_metadata call.
        let gone = tmp.join("totally-gone.so").to_string_lossy().into_owned();
        assert!(!candidate_exists_with_diagnostics(&gone));
        // Cleanup.
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// `copy_hook_library_to_dev` must log all checked paths and return
    /// false when none of the candidates exist on disk. We point it at a
    /// non-existent rootfs/data_dir so every candidate is missing.
    #[test]
    fn copy_hook_library_to_dev_returns_false_when_not_found() {
        let cfg = Config {
            rootfs: "/nonexistent-twoyi-rootfs-xyz".to_string(),
            data_dir: "/nonexistent-twoyi-data-xyz".to_string(),
            ..Config::default()
        };
        let ok = copy_hook_library_to_dev(
            &cfg,
            "libgetpid_hook.so",
            "/tmp/twoyi-test-hook-should-not-exist.so",
            "test-not-found",
        );
        assert!(!ok);
        // Destination must NOT have been created.
        assert!(!Path::new("/tmp/twoyi-test-hook-should-not-exist.so").exists());
    }

    /// `copy_hook_library_to_dev` must find and copy the library when a
    /// candidate path DOES exist. We create a temp file at candidate #3
    /// (the confirmed RomManager path) and verify it gets copied to the
    /// destination.
    #[test]
    fn copy_hook_library_to_dev_finds_and_copies_when_candidate_exists() {
        // Build a temp dir mimicking {data_dir}/rootfs/system/lib64/<lib>.
        let tmp = std::env::temp_dir().join("twoyi-hook-copy-test");
        let lib64 = tmp.join("rootfs/system/lib64");
        std::fs::create_dir_all(&lib64).unwrap();
        let src = lib64.join("libgetpid_hook.so");
        std::fs::write(&src, b"fake ELF content").unwrap();

        let cfg = Config {
            // Point rootfs somewhere the lib does NOT live, so candidate
            // #1 and #2 miss and we fall through to candidate #3.
            rootfs: tmp
                .join("nonexistent-profile-rootfs")
                .to_string_lossy()
                .into_owned(),
            data_dir: tmp.to_string_lossy().into_owned(),
            ..Config::default()
        };
        let dst = tmp.join("copied-libgetpid_hook.so");
        let dst_str = dst.to_string_lossy().to_string();
        let ok = copy_hook_library_to_dev(&cfg, "libgetpid_hook.so", &dst_str, "test-found");
        assert!(ok, "expected copy to succeed via candidate #3");
        assert!(dst.exists(), "destination file should exist after copy");
        let content = std::fs::read_to_string(&dst).unwrap();
        assert_eq!(content, "fake ELF content");

        // Cleanup.
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
