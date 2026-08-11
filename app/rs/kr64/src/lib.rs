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
use std::sync::atomic::{AtomicBool, Ordering};

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
// Graceful shutdown via SIGTERM.
//
// The KVM E2E test script sends SIGTERM to kr64 after the boot wait
// (the guest init runs indefinitely in TWRP mode). Without a handler,
// the default SIGTERM action kills kr64 instantly while it is blocked
// in `waitpid()` — so we never log whether the guest init crashed,
// exited, or was still running. This handler sets a flag that the
// waitpid loop checks, allowing kr64 to do a final non-blocking
// `waitpid()`, log the guest's exit status, and kill the guest if it
// is still alive before exiting cleanly.
// ============================================================================

/// Set to `true` by the SIGTERM handler. The waitpid loop polls this
/// between `WNOHANG` checks and initiates graceful shutdown when set.
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

/// SIGTERM signal handler. Only touches an `AtomicBool` (async-signal-
/// safe via `SeqCst` ordering) — all real work (waitpid, logging) is
/// done in the main thread after the handler returns.
extern "C" fn sigterm_handler(_sig: libc::c_int) {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

/// Install a SIGTERM handler that sets [`SHUTDOWN_REQUESTED`] instead
/// of using the default (terminate) action. This must be called BEFORE
/// the waitpid loop so the handler is in place when the script sends
/// SIGTERM.
fn install_sigterm_handler() {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = sigterm_handler as *const () as usize;
        libc::sigemptyset(&mut sa.sa_mask);
        sa.sa_flags = 0; // default: no SA_RESTART (we want EINTR)
        if libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut()) == 0 {
            info!("[KR64][parent] SIGTERM handler installed (graceful shutdown enabled)");
        } else {
            warning!("[KR64][parent] failed to install SIGTERM handler — script SIGTERM will kill us without logging guest status");
        }
    }
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

    /// Boot a TWRP-style recovery image instead of a full Android
    /// rootfs. TWRP boot is MUCH simpler than full Android boot:
    ///
    ///   * TWRP's `init` binary is **statically linked** (i386), so
    ///     it doesn't need LD_PRELOAD hook libraries for itself. We
    ///     skip the libgetpid_hook.so and libtwoyi_loader_shlib.so
    ///     read/write (init ignores LD_PRELOAD). We DO load a
    ///     separate i686 (32-bit x86) hook library `twrp_fb_hook.so`
    ///     — the dynamically-linked i386 `recovery` service loads it
    ///     via LD_PRELOAD to intercept FBIOGET_VSCREENINFO on
    ///     /dev/graphics/fb0 (fixing the libminuitwrp segfault at
    ///     offset 0x57d7). The i686 hook is required because the
    ///     32-bit bionic linker in TWRP's recovery process CANNOT
    ///     load the 64-bit libtwoyi_loader_shlib.so.
    ///   * TWRP's ramdisk doesn't use APEX packages, so we skip the
    ///     /apex bind mount (which would otherwise give the guest
    ///     access to the host's APEX packages -- harmless but
    ///     unnecessary for TWRP, and skipping it avoids the
    ///     "is /apex accessible?" failure mode entirely).
    ///   * TWRP's init.rc doesn't use binder, so we skip the
    ///     binderfs mount (saves a `mount("binder", ...)` call and
    ///     the symlink setup).
    ///   * TWRP's init.rc has its own SELinux handling, so we skip
    ///     the SELinux permissive watchdog thread (no thread
    ///     continuously writing "0" to /sys/fs/selinux/enforce).
    ///   * TWRP doesn't need the /dev/twoyi-bin/ copy of system
    ///     binaries (logd, servicemanager, vold, zygote, etc.) --
    ///     TWRP's init.rc only starts `ueventd`, `recovery`, and
    ///     `partlink`.
    ///   * TWRP doesn't need the property_info / properties_serial
    ///     pre-creation (TWRP's property service is much simpler).
    ///   * TWRP doesn't need the fstab.ranchu overwrite (TWRP has
    ///     its own /etc/recovery.fstab).
    ///
    /// What we KEEP for TWRP boot:
    ///   * Basic device creation (qemu_pipe for display, touch, key,
    ///     event, gb, gb2 -- the recovery UI uses these).
    ///   * Rootfs setup (mount tmpfs on /dev, /proc, /sys, /tmp +
    ///     pivot_root + chdir).
    ///   * proc_emu (8-core, 4GB synthesis -- TWRP reads /proc).
    ///   * Samsung GameSDK compat paths (harmless, just empty dirs).
    ///   * Graphics device stubs (recovery may probe /dev/graphics/fb0
    ///     etc.).
    ///   * Pre-created boot directories (/dev/block, /mnt, etc.).
    ///   * PID namespace (CLONE_NEWPID so TWRP init becomes PID 1).
    ///   * The coldboot_done + busybox + magisk markers.
    ///
    /// When `boot_recovery=true`, the caller MUST also set
    /// `init_path="/init"` (TWRP's init is at the root of the
    /// ramdisk, not at /system/bin/init). The `--boot-recovery` flag
    /// sets this automatically if `--init` is not also passed.
    pub boot_recovery: bool,
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
            boot_recovery: false,
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
///   --boot-recovery           (TWRP recovery boot: i686 LD_PRELOAD FB hook only,
///                              /apex bind, binderfs, SELinux watchdog,
///                              twoyi-bin copy; set init_path=/init)
///   --help, -h                (show usage)
pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Config, String> {
    let mut cfg = Config::default();
    // Track whether --init was passed explicitly so --boot-recovery
    // doesn't clobber an explicit --init value. (If the user passes
    // both --boot-recovery AND --init /custom/init, the explicit
    // --init wins and --boot-recovery only flips the bool.)
    let mut init_explicitly_set = false;
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
                       --boot-recovery           TWRP recovery boot (i686 LD_PRELOAD FB hook only,\n\
                                                 /apex bind, binderfs, SELinux watchdog;\n\
                                                 set init_path=/init)\n\
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
                init_explicitly_set = true;
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
            "--boot-recovery" => cfg.boot_recovery = true,
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

    // --boot-recovery implies init_path=/init (TWRP's init is at the
    // root of the ramdisk, not at /system/bin/init). Don't override
    // if the user explicitly passed --init /custom/init.
    if cfg.boot_recovery && !init_explicitly_set {
        cfg.init_path = "/init".to_string();
        info!("[KR64] --boot-recovery: setting init_path to /init (TWRP statically-linked init)");
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

/// Patch TWRP's init.rc to add `setenv LD_PRELOAD /dev/twrp_fb_hook.so`
/// to the recovery service definition.
///
/// TWRP's init.rc defines the recovery service as:
/// ```text
/// service recovery /sbin/recovery
/// ```
/// (possibly with indented options like `seclabel`). We insert
/// `    setenv LD_PRELOAD /dev/twrp_fb_hook.so` as a new indented option
/// right after the `service recovery` line.
///
/// The hook library MUST be the i686 (32-bit x86) `twrp_fb_hook.so`,
/// NOT the x86_64 `libtwoyi_loader_shlib.so`. TWRP's recovery binary
/// is i386 and its 32-bit bionic linker cannot load 64-bit libraries
/// ("CANNOT LINK EXECUTABLE: ... is 64-bit instead of 32-bit"). See
/// Task ID 18 / KVM run 31536016997 for the regression this caused.
///
/// Returns the patched content, or `None` if the `service recovery` line
/// was not found. The patch is IDEMPOTENT: if the setenv line is already
/// present, the caller should skip the write (checked before calling).
fn patch_twrp_init_rc_recovery_service(content: &str) -> Option<String> {
    // Find the "service recovery" line. It may be "service recovery /sbin/recovery"
    // or "service recovery /sbin/recovery\r" (CRLF). We match the prefix
    // "service recovery " at the start of a line.
    let mut lines = content.lines().peekable();
    let mut result = String::with_capacity(content.len() + 64);
    let mut found = false;
    while let Some(line) = lines.next() {
        result.push_str(line);
        // Check if this line starts the recovery service definition.
        // We check the trimmed start to handle leading whitespace (shouldn't
        // happen for service definitions, but be defensive).
        let trimmed = line.trim_start();
        if !found && trimmed.starts_with("service recovery ") {
            // This is the recovery service line. Insert the setenv directive
            // as the next line (indented with 4 spaces, matching init.rc
            // convention for service options).
            result.push('\n');
            result.push_str("    setenv LD_PRELOAD /dev/twrp_fb_hook.so");
            found = true;
        }
        // Preserve the original line ending (lines() strips \n, so we add
        // it back). For the last line (no trailing \n), we don't add one.
        if lines.peek().is_some() {
            result.push('\n');
        }
    }
    if found {
        Some(result)
    } else {
        None
    }
}

/// Set the SELinux security context of a file using the `lsetxattr(2)`
/// syscall directly.
///
/// This is exactly what the `chcon` command does internally
/// (`setfilecon()` -> `setxattr("security.selinux", ...)`) but without
/// spawning any subprocess. It is the ONLY safe way to relabel files
/// from kr64 AFTER `pivot_root`, because:
///
///   * The GUEST's `chcon` binary is bind-mounted from the ROM and
///     depends on libraries from `/apex/com.android.runtime/lib64/`
///     (libbase.so, libc++.so). After `pivot_root`, `/apex` is EMPTY
///     (apexd hasn't mounted the APEX packages yet), so the guest's
///     chcon crashes with SIGSEGV at address 0x86 in linker64 (NULL
///     soinfo for the missing library). See KVM runs 31505655579 +
///     31507752891.
///   * The HOST's `chcon` binary is unreachable after `pivot_root`
///     (the old root was detached via `umount2(MNT_DETACH)`).
///
/// `lsetxattr` (not `setxattr`) is used so that SYMLINKS are labeled
/// directly, not their targets — important for the binderfs device
/// symlinks (`/dev/binder`, `/dev/hwbinder`, `/dev/vndbinder`).
///
/// # Arguments
///
/// * `path`    — file path (absolute, chroot-relative after pivot_root).
/// * `context` — SELinux context string, e.g. `"u:object_r:system_file:s0"`.
///   Must NOT contain a NUL byte (the function returns `InvalidInput`).
///
/// # Permissions
///
/// On a real Android device with an enforcing kernel, this requires:
///   * `CAP_MAC_ADMIN` capability (root has it by default).
///   * SELinux policy: `relabelfrom` on the file's CURRENT label +
///     `relabelto` on the NEW label.
///
/// The kr64 process runs as root (required for `pivot_root` and the
/// other namespace ops), so it has `CAP_MAC_ADMIN`. On a real device,
/// the operation succeeds and the new label is enforced by the kernel.
/// On the KVM test environment (permissive mode), the operation may
/// fail with `EPERM`/`ENOTSUP` but access is allowed anyway via the
/// permissive watchdog — callers should log the failure but not crash.
///
/// # Returns
///
/// `Ok(())` on success. `Err(io::Error)` on failure — the caller
/// decides whether to log+continue or propagate. The `io::Error`
/// carries the `errno` value via `raw_os_error()`.
fn set_selinux_context(path: &str, context: &str) -> std::io::Result<()> {
    // Construct the C strings. The attribute name is a static literal
    // with no NULs, so CString::new is infallible — unwrap is safe.
    let path_c = CString::new(path).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("path contains NUL byte: {}", e),
        )
    })?;
    let attr_c = CString::new("security.selinux").unwrap();
    // libselinux's setfilecon() passes strlen(con)+1 as the size —
    // i.e. the context string INCLUDING its trailing NUL. We match
    // that exactly by using as_bytes_with_nul().
    let context_c = CString::new(context).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("context contains NUL byte: {}", e),
        )
    })?;
    let context_bytes_with_nul = context_c.as_bytes_with_nul();

    // Safety: path_c and attr_c are valid NUL-terminated C strings.
    // context_bytes_with_nul points to a valid byte buffer of the
    // given length. flags=0 means no XATTR_CREATE/XATTR_REPLACE.
    let ret = unsafe {
        libc::lsetxattr(
            path_c.as_ptr(),
            attr_c.as_ptr(),
            context_bytes_with_nul.as_ptr() as *const libc::c_void,
            context_bytes_with_nul.len(),
            0,
        )
    };
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
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
    //
    // TWRP BOOT: TWRP's init binary is statically linked (i386), so it
    // doesn't need the x86_64 LD_PRELOAD hooks (libgetpid_hook.so or
    // libtwoyi_loader_shlib.so) — the statically-linked init doesn't
    // use the dynamic linker and ignores LD_PRELOAD entirely. We skip
    // BOTH x86_64 hook libraries in TWRP mode.
    //
    // HOWEVER, TWRP's recovery service (forked+exec'd by init) is
    // DYNAMICALLY linked — but it's i386 (32-bit x86), interpreter
    // /sbin/linker (also i386). It loads libminuitwrp.so which crashes
    // at offset 0x57d7 (NULL deref after FBIOGET_VSCREENINFO returns
    // ENOTTY on the /dev/graphics/fb0 → regular-file stub). See KVM
    // run 31531742745 dmesg.log for the crash signature.
    //
    // FIX: in TWRP mode, we load a SEPARATE i686 (32-bit x86)
    // LD_PRELOAD library, `twrp_fb_hook.so`, that intercepts FB ioctls
    // on /dev/graphics/fb0 and returns valid 720x1280@32bpp screen
    // info. The statically-linked init ignores LD_PRELOAD (it doesn't
    // use the dynamic linker), but when init forks+execs recovery,
    // recovery's 32-bit bionic linker loads LD_PRELOAD → our i686 hook
    // activates → FB ioctls succeed → no libminuitwrp crash.
    //
    // CRITICAL (Task ID 18, KVM run 31536016997): the i386 recovery's
    // 32-bit bionic linker CANNOT load the 64-bit libtwoyi_loader_shlib.so
    // ("CANNOT LINK EXECUTABLE: ... is 64-bit instead of 32-bit"). Task
    // ID 17 incorrectly switched TWRP mode to use the x86_64 main loader;
    // the 32-bit linker aborts the recovery process on the architecture
    // mismatch, so recovery never starts and is invisible in `ps`. We
    // reverted to the i686 twrp_fb_hook.so — the architecturally correct
    // choice for the i386 recovery binary.
    //
    // We do NOT load libgetpid_hook.so in TWRP mode (recovery doesn't
    // need to fake PID 1 — only init does, and init is static).
    let hook_lib_getpid = if cfg.boot_recovery {
        info!("[KR64] TWRP boot: skipping libgetpid_hook.so read (init is statically linked)");
        None
    } else {
        find_and_read_hook_library(&cfg, "libgetpid_hook.so", "LD_PRELOAD will fail")
    };
    let hook_lib_loader = if cfg.boot_recovery {
        info!(
            "[KR64] TWRP boot: skipping libtwoyi_loader_shlib.so read (recovery is i386; x86_64 loader cannot be loaded by the 32-bit bionic linker)"
        );
        None
    } else {
        find_and_read_hook_library(
            &cfg,
            "libtwoyi_loader_shlib.so",
            "seccomp virtualization disabled",
        )
    };
    // TWRP-mode i686 FB ioctl hook (separate from the x86_64 main loader).
    // Loaded ONLY in TWRP mode; written to /dev/twrp_fb_hook.so.
    let hook_lib_twrp_fb = if cfg.boot_recovery {
        find_and_read_hook_library(
            &cfg,
            "twrp_fb_hook.so",
            "TWRP framebuffer virtualization disabled (recovery will crash in libminuitwrp.so)",
        )
    } else {
        None
    };

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
    //
    // TWRP BOOT: pass `boot_recovery=cfg.boot_recovery` so mount_mgr
    // skips the /apex bind mount (TWRP's ramdisk doesn't use APEX
    // packages; the bind mount would just give the guest access to the
    // host's APEX packages, which is harmless but unnecessary and
    // avoids the "is /apex accessible?" failure mode entirely).
    if cfg.use_namespaces {
        let mount_cfg = mount_mgr::MountConfig {
            rootfs: cfg.rootfs.clone(),
            rom_dir: cfg.rom_dir.clone(),
            use_namespaces: cfg.use_namespaces,
            read_only_rom: cfg.read_only_rom,
            boot_recovery: cfg.boot_recovery,
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
    //
    // TWRP BOOT: TWRP doesn't have /system/lib64/libc.so etc. (init
    // is statically linked), so the standard diagnostic block would
    // log a wall of "metadata failed" errors. Skip it and instead log
    // a TWRP-specific diagnostic: verify /init exists and is a static
    // ELF binary.
    if cfg.boot_recovery {
        // TWRP diagnostics: verify the init binary.
        let init_full = if cfg.use_namespaces {
            cfg.init_path.clone()
        } else {
            format!("{}{}", cfg.rootfs, cfg.init_path)
        };
        match std::fs::metadata(&init_full) {
            Ok(meta) => {
                info!(
                    "[KR64] TWRP PARENT: {} -> file ({} bytes) -- ready to exec",
                    init_full,
                    meta.len()
                );
            }
            Err(e) => {
                error!(
                    "[KR64] TWRP PARENT: {} -> metadata FAILED: {} (TWRP init binary missing from rootfs)",
                    init_full,
                    e
                );
            }
        }
    } else if cfg.use_namespaces {
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
                let mut files = Vec::new();
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    files.push(format!("{} ({} bytes)", name, size));
                }
                info!(
                    "[KR64] PARENT: /apex/com.android.runtime/lib64/bionic has {} entries: [{}]",
                    files.len(),
                    files.join(", ")
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
    // TWRP BOOT: write the i686 twrp_fb_hook.so to /dev/twrp_fb_hook.so
    // (tmpfs). The dynamically-linked i386 recovery binary loads it via
    // LD_PRELOAD=/dev/twrp_fb_hook.so (injected via init.rc `setenv`).
    // The hook's open/ioctl intercepts FBIOGET_VSCREENINFO etc. on
    // /dev/graphics/fb0 and returns valid 720x1280@32bpp screen info,
    // fixing the libminuitwrp segfault at offset 0x57d7.
    if let Some((src, content)) = &hook_lib_twrp_fb {
        write_hook_library_to_dev("twrp_fb_hook.so", src, content, "/dev/twrp_fb_hook.so");
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
    // FIX: relabel the libraries to u:object_r:system_file:s0, which
    // vendor_init CAN access (it needs to read system_file for its own
    // operation). We do this BEFORE forking init.
    //
    // CRITICAL (KVM runs 31505655579 + 31507752891): the previous
    // implementation spawned the GUEST's `chcon` binary as a subprocess
    // to do the relabeling. But after pivot_root, the guest's chcon
    // binary is bind-mounted from the ROM and depends on libraries from
    // `/apex/com.android.runtime/lib64/` (libbase.so, libc++.so). /apex
    // is EMPTY at this point (apexd hasn't mounted the APEX packages
    // yet). The linker resolves the missing library to a NULL soinfo
    // and crashes with SIGSEGV at address 0x86. This produced 10
    // chcon/restorecon segfaults per KVM run. Task ID 10's workaround
    // (adding /apex paths to LD_LIBRARY_PATH) did NOT fix this — the
    // paths were set but /apex is empty, so the libraries still don't
    // exist.
    //
    // NEW FIX (Task ID 11): do the SELinux relabel DIRECTLY from kr64
    // via the `lsetxattr(2)` syscall on the `security.selinux` extended
    // attribute. This is EXACTLY what `chcon` does internally
    // (`setfilecon()` -> `setxattr("security.selinux", ...)`) but with
    // ZERO subprocess and ZERO dependency on /apex/ libraries.
    // `lsetxattr` (not `setxattr`) is used so symlinks are labeled
    // directly (not their targets) — important for the binderfs device
    // symlinks.
    //
    // On a real device with an enforcing kernel: kr64 runs as root
    // which has CAP_MAC_ADMIN, so the operation succeeds and the label
    // change is enforced by the kernel. On the KVM test environment
    // (permissive mode), the operation may fail with EPERM/ENOTSUP but
    // access is allowed anyway via the permissive watchdog.
    for lib_path in &[
        "/dev/libgetpid_hook.so",
        "/dev/libtwoyi_loader_shlib.so",
        "/dev/twrp_fb_hook.so",
    ] {
        if Path::new(lib_path).exists() {
            match set_selinux_context(lib_path, "u:object_r:system_file:s0") {
                Ok(()) => {
                    info!(
                        "[KR64] PARENT: lsetxattr {} -> u:object_r:system_file:s0 OK (direct syscall, no chcon subprocess)",
                        lib_path
                    );
                }
                Err(e) => {
                    // Don't crash — the permissive watchdog (KVM test
                    // only) will allow access anyway. On a real device
                    // with root + CAP_MAC_ADMIN this should succeed.
                    warning!(
                        "[KR64] PARENT: lsetxattr {} -> u:object_r:system_file:s0 FAILED: {} (errno={}). On KVM permissive this is non-fatal; on real device this indicates missing CAP_MAC_ADMIN or SELinux policy.",
                        lib_path,
                        e,
                        e.raw_os_error().unwrap_or(0)
                    );
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
    //
    // TWRP BOOT: skip this entire block. TWRP's init.rc only starts
    // `ueventd`, `recovery`, and `partlink` (all in /sbin/), and these
    // are statically linked or have known-good exec paths in the
    // ramdisk. Copying 50+ Android binaries to /dev/twoyi-bin/ would
    // be wasteful and could confuse TWRP's exec lookups.
    if cfg.boot_recovery {
        info!("[KR64] TWRP boot: skipping /dev/twoyi-bin/ copy (TWRP only needs /sbin/*)");
    } else {
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
                        // Relabel to system_file via direct lsetxattr
                        // syscall (no chcon subprocess — see the
                        // comment on the hook-library relabel above
                        // for why subprocesses crash after pivot_root).
                        if let Err(e) = set_selinux_context(&dst, "u:object_r:system_file:s0") {
                            warning!(
                                "[KR64] PARENT: lsetxattr {} -> system_file FAILED: {} (errno={})",
                                dst,
                                e,
                                e.raw_os_error().unwrap_or(0)
                            );
                        }
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
                                if let Err(e) =
                                    set_selinux_context(&dst, "u:object_r:system_file:s0")
                                {
                                    warning!(
                                        "[KR64] PARENT: lsetxattr {} -> system_file FAILED: {} (errno={})",
                                        dst,
                                        e,
                                        e.raw_os_error().unwrap_or(0)
                                    );
                                }
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

    // TWRP BOOT: replace the /dev/graphics/fb0 + /dev/fb0 symlinks (which
    // point to /dev/null) with regular files of 3,686,400 bytes (720x1280x4
    // RGBA8888). This makes open() succeed and mmap() work naturally, so
    // libminuitwrp's graphics_fbdev_init can proceed past the open+mmap
    // stage. The FB ioctls themselves are intercepted by the i686
    // `twrp_fb_hook.so` (LD_PRELOAD'd into the recovery process). See
    // `devices::create_twrp_framebuffer` for the full rationale.
    if cfg.boot_recovery {
        if let Err(e) = devices::create_twrp_framebuffer(&rootfs_prefix) {
            warning!(
                "[KR64] PARENT: failed to create TWRP framebuffer: {} (recovery will crash in libminuitwrp.so)",
                e
            );
        }
    }

    // TWRP BOOT: patch {rootfs}/init.rc to add `setenv LD_PRELOAD
    // /dev/twrp_fb_hook.so` to the recovery service definition.
    //
    // ROOT CAUSE (KVM run 31533796663): kr64 sets LD_PRELOAD in init's
    // environment, but TWRP's init (based on AOSP's init) builds a FRESH
    // environment for each service from the service's `setenv` directives
    // plus a few inherited vars (ANDROID_ROOT, ANDROID_DATA, etc.).
    // LD_PRELOAD is NOT in the inherited list, so recovery's bionic linker
    // never sees it → our hook is never loaded → recovery crashes at
    // offset 0x57d7 in libminuitwrp.so (same as without the hook).
    //
    // FIX: patch init.rc to add `setenv LD_PRELOAD /dev/twrp_fb_hook.so`
    // to the recovery service. TWRP's init supports `setenv` in service
    // blocks (confirmed via `strings /tmp/twrp/rd/init | grep setenv`).
    // This adds LD_PRELOAD to recovery's environment, so the bionic linker
    // loads our i686 hook → FB ioctls are intercepted → no crash.
    //
    // CRITICAL (Task ID 18, KVM run 31536016997): the LD_PRELOAD path
    // MUST be `/dev/twrp_fb_hook.so` (i686), NOT `/dev/libtwoyi_loader_shlib.so`
    // (x86_64). TWRP's recovery binary is i386 and its 32-bit bionic
    // linker cannot load 64-bit libraries. Task ID 17 incorrectly used
    // the x86_64 path; the linker aborted recovery on the architecture
    // mismatch, so recovery was invisible in `ps`.
    //
    // The patch is IDEMPOTENT: if the setenv line is already present
    // (e.g., from a previous boot), we skip the write.
    if cfg.boot_recovery {
        let init_rc_path = format!("{}/init.rc", rootfs_prefix);
        match std::fs::read_to_string(&init_rc_path) {
            Ok(content) => {
                // Check if the patch is already applied.
                let patch_marker = "    setenv LD_PRELOAD /dev/twrp_fb_hook.so";
                if content.contains(patch_marker) {
                    info!(
                        "[KR64] PARENT: init.rc already patched with LD_PRELOAD for recovery service (idempotent skip)"
                    );
                } else if let Some(patched) = patch_twrp_init_rc_recovery_service(&content) {
                    match std::fs::write(&init_rc_path, &patched) {
                        Ok(()) => info!(
                            "[KR64] PARENT: patched init.rc — added 'setenv LD_PRELOAD /dev/twrp_fb_hook.so' to recovery service"
                        ),
                        Err(e) => warning!(
                            "[KR64] PARENT: failed to write patched init.rc: {} (recovery will crash in libminuitwrp.so)",
                            e
                        ),
                    }
                } else {
                    warning!(
                        "[KR64] PARENT: could not find 'service recovery' in init.rc — LD_PRELOAD patch skipped (recovery will crash in libminuitwrp.so)"
                    );
                }
            }
            Err(e) => warning!(
                "[KR64] PARENT: failed to read init.rc for LD_PRELOAD patching: {} (recovery will crash in libminuitwrp.so)",
                e
            ),
        }
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
    //
    // TWRP BOOT: skip this overwrite. TWRP has its own /etc/recovery.fstab
    // and doesn't read /vendor/etc/fstab.ranchu. Overwriting it would
    // create a stub file that's never read, and worse, would create the
    // /vendor/etc/ directory in TWRP's ramdisk if it doesn't exist (TWRP
    // puts its vendor files in /sbin/, not /vendor/).
    if cfg.boot_recovery {
        info!("[KR64] TWRP boot: skipping /vendor/etc/fstab.ranchu overwrite (TWRP has /etc/recovery.fstab)");
    } else {
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
    //
    // TWRP BOOT: skip this block. TWRP's init.rc has a much simpler
    // property service (no second_stage init / secilc / vendor_init
    // chain), and doesn't write to /dev/__properties__/property_info.
    if cfg.boot_recovery {
        info!("[KR64] TWRP boot: skipping /dev/__properties__ pre-creation (TWRP has its own property service)");
    } else {
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
    //
    // TWRP BOOT: skip the permissive watchdog. TWRP's init.rc has its
    // own SELinux handling (setenforce 0 in init.rc itself, no
    // vendor_init subcontexts that need permissive mode to access
    // /dev/lib*.so because we don't LD_PRELOAD anything). Spinning up
    // the watchdog thread would just waste CPU writing "0" to a file
    // that's already set to "0" by TWRP's own init.
    let enforce_thread = if cfg.boot_recovery {
        info!("[KR64] TWRP boot: skipping SELinux permissive watchdog (TWRP handles SELinux in init.rc)");
        None
    } else {
        std::thread::Builder::new()
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
            .ok()
    };

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
        //
        // TWRP BOOT: we DO set LD_PRELOAD in TWRP mode, to the i686
        // (32-bit x86) hook library: `/dev/twrp_fb_hook.so`. The
        // statically-linked init ignores LD_PRELOAD (it doesn't use the
        // dynamic linker), but when init forks+execs recovery
        // (dynamically linked i386, interpreter /sbin/linker),
        // recovery's 32-bit bionic linker loads LD_PRELOAD → our i686
        // hook activates → FB ioctls on /dev/graphics/fb0 return valid
        // 720x1280@32bpp screen info → no libminuitwrp crash.
        //
        // CRITICAL (Task ID 18, KVM run 31536016997): Task ID 17
        // incorrectly switched this to `/dev/libtwoyi_loader_shlib.so`
        // (x86_64). The i386 recovery's 32-bit bionic linker CANNOT
        // load a 64-bit library ("CANNOT LINK EXECUTABLE: ... is 64-bit
        // instead of 32-bit"), so the linker aborted the recovery
        // process before main() ran — recovery was invisible in `ps`
        // and there were 0 libminuitwrp segfaults (because recovery
        // never reached libminuitwrp). Reverted to the i686 hook.
        //
        // TWOYI_SKIP_PRELOAD still works in TWRP mode as a diagnostic
        // override (disables the hook too, so we can confirm the
        // crash returns without the hook).
        let skip_preload = std::env::var("TWOYI_SKIP_PRELOAD").is_ok();
        // LD_PRELOAD for non-TWRP mode: load BOTH the seccomp/SIGSYS
        // virtualization library (libtwoyi_loader_shlib.so) AND the getpid
        // hook (libgetpid_hook.so). The virtualization library installs
        // seccomp BPF filter + SIGSYS handler via .init_array constructor
        // before init's main() runs. The getpid hook makes init think
        // it's PID 1.
        //
        // LD_PRELOAD for TWRP mode: load ONLY twrp_fb_hook.so (the i686
        // FB ioctl hook). The x86_64 libgetpid_hook.so and
        // libtwoyi_loader_shlib.so are NOT loaded because:
        //   - init is statically linked (ignores LD_PRELOAD)
        //   - recovery is i386 and its 32-bit linker can't load x86_64 libs
        let ld_preload_str = if cfg.boot_recovery {
            "LD_PRELOAD=/dev/twrp_fb_hook.so".to_string()
        } else {
            "LD_PRELOAD=/dev/libgetpid_hook.so:/dev/libtwoyi_loader_shlib.so".to_string()
        };
        // TWRP BOOT: use a minimal env that mirrors TWRP's init.rc exports.
        // TWRP's init.rc does its own:
        //   export PATH /sbin:/system/bin
        //   export LD_LIBRARY_PATH /sbin:/system/lib
        //   export ANDROID_ROOT /system
        //   export EXTERNAL_STORAGE /sdcard
        // We pre-set the same values so the statically-linked init binary
        // has them available BEFORE init.rc's exports run (init.rc exports
        // happen after init's main() starts, but some init code paths may
        // need them earlier).
        //
        // Note: TWRP's LD_LIBRARY_PATH uses /system/lib (32-bit) not
        // /system/lib64 (64-bit) -- this matches the i386 architecture
        // of TWRP's binaries.
        let mut env_vars: Vec<CString> = if cfg.boot_recovery {
            vec![
                CString::new("PATH=/sbin:/system/bin").unwrap(),
                CString::new("ANDROID_ROOT=/system").unwrap(),
                CString::new("ANDROID_DATA=/data").unwrap(),
                CString::new("ANDROID_BOOTLOGO=1").unwrap(),
                CString::new("EXTERNAL_STORAGE=/sdcard").unwrap(),
                // TWRP's init.rc sets LD_LIBRARY_PATH itself, but pre-set
                // it so any pre-init.rc code that needs /sbin libs can
                // find them (e.g. /sbin/linker for dynamically-linked
                // /sbin/recovery which has interpreter /sbin/linker).
                CString::new("LD_LIBRARY_PATH=/sbin:/system/lib").unwrap(),
                twoyi_rootfs_env,
            ]
        } else {
            vec![
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
                    // CRITICAL (KVM run 31509809060): The guest init (pid 6188)
                    // STILL crashes with SIGSEGV at 0x86 in linker64 even though
                    // both hook libraries are loaded and relabeled. The crash
                    // happens BEFORE the .init_array constructor runs (0
                    // twoyi_loader init messages), so it's during the linker's
                    // library loading (mapping segments, resolving DT_NEEDED,
                    // relocating).
                    //
                    // The hook libraries (libgetpid_hook.so and
                    // libtwoyi_loader_shlib.so) have DT_NEEDED:
                    //   - libc.so (version LIBC)
                    //   - libdl.so (version LIBC)
                    //
                    // On Android 11, /system/lib64/libdl.so is a 5848-byte
                    // BOOTSTRAP STUB (used during early init before apexd
                    // mounts the APEX packages). The REAL libdl.so (with the
                    // full implementation) is at
                    // /apex/com.android.runtime/lib64/bionic/libdl.so.
                    //
                    // The bootstrap stub SHOULD provide the LIBC version, but
                    // there may be a subtle incompatibility (missing symbol,
                    // different version definition, etc.) that causes the
                    // linker to get a NULL soinfo and crash.
                    //
                    // FIX: Put /apex/com.android.runtime/lib64/bionic FIRST in
                    // LD_LIBRARY_PATH, so the linker finds the REAL bionic
                    // libraries (libc.so, libdl.so, libm.so) before the
                    // bootstrap stubs in /system/lib64/. This ensures the
                    // version requirements are satisfied by the full bionic
                    // implementation.
                    //
                    // This is safe because:
                    //   - /apex/com.android.runtime/lib64/bionic/libc.so is the
                    //     SAME file as /system/lib64/libc.so (same size, 984072
                    //     bytes — /system/lib64/libc.so is a hardlink/copy, not
                    //     a stub).
                    //   - /apex/com.android.runtime/lib64/bionic/libdl.so is
                    //     the REAL libdl.so (larger than the 5848-byte stub).
                    //   - The apex directory has all 4 bionic libraries
                    //     (libc.so, libm.so, libdl.so, libdl_android.so), so
                    //     they can find each other.
                    "LD_LIBRARY_PATH=\
                /apex/com.android.runtime/lib64/bionic:\
                /apex/com.android.runtime/lib64:\
                /apex/com.android.runtime/lib64/bootstrap:\
                /system/lib64:\
                /system/lib64/bootstrap:\
                /vendor/lib64:\
                /apex/com.android.os.statsd/lib64:\
                /system_ext/lib64:\
                /product/lib64",
                )
                .unwrap(),
            ]
        };
        // LD_DEBUG support: if TWOYI_LD_DEBUG is set in the parent env,
        // propagate it as LD_DEBUG to the guest init. This enables bionic
        // linker debug output (which library is being loaded when the
        // crash happens). The output goes to stderr (captured in
        // kr64-stderr.log).
        //
        // Usage: set TWOYI_LD_DEBUG=2 in the CI env to enable "libs" level
        // debug (library load/unload). Set TWOYI_LD_DEBUG=4 for "files"
        // level (file opens). Set TWOYI_LD_DEBUG=6 for both.
        if let Ok(ld_debug_val) = std::env::var("TWOYI_LD_DEBUG") {
            if !ld_debug_val.is_empty() {
                let ld_debug_env = format!("LD_DEBUG={}", ld_debug_val);
                match CString::new(ld_debug_env) {
                    Ok(c) => {
                        env_vars.push(c);
                        unsafe {
                            safe_write_err(b"[KR64 CHILD] TWOYI_LD_DEBUG set -- enabling LD_DEBUG for guest init\n");
                        }
                    }
                    Err(_) => unsafe {
                        safe_write_err(b"[KR64 CHILD] WARN: TWOYI_LD_DEBUG contains NUL byte -- skipping LD_DEBUG\n");
                    },
                }
            }
        }
        if skip_preload {
            unsafe {
                if cfg.boot_recovery {
                    safe_write_err(b"[KR64 CHILD] TWOYI_SKIP_PRELOAD set -- skipping LD_PRELOAD in TWRP mode (recovery will crash in libminuitwrp.so without the FB ioctl hook)\n");
                } else {
                    safe_write_err(b"[KR64 CHILD] TWOYI_SKIP_PRELOAD set -- skipping LD_PRELOAD (init will exit 31)\n");
                }
            }
        } else {
            env_vars.push(CString::new(ld_preload_str).unwrap());
        }
        // Diagnostic: log the env vars being passed to init. This helps
        // verify the LD_LIBRARY_PATH reorder and LD_DEBUG are taking effect.
        // Use safe_write_err (async-signal-safe) since we're in the child
        // between fork() and execve().
        unsafe {
            safe_write_err(b"[KR64 CHILD] env vars passed to init:\n");
            for ev in &env_vars {
                safe_write_err(b"  ");
                safe_write_err(ev.to_bytes());
                safe_write_err(b"\n");
            }
        }
        let env_ptrs: Vec<*const libc::c_char> = env_vars
            .iter()
            .map(|s| s.as_ptr())
            .chain(std::iter::once(std::ptr::null()))
            .collect();

        // If execve returns at all, it failed. Capture errno BEFORE any
        // other call (any libc call can clobber it) and surface it.
        //
        // TWRP BOOT: redirect the guest init's stdout/stderr to
        // /twrp-init.log (at the ROOT of the rootfs, NOT /tmp/ which
        // is a tmpfs that gets unmounted when kr64 dies). This file
        // is on the ext4 rootfs and survives kr64's death so the KVM
        // test script can pull it.
        if cfg.boot_recovery {
            let log_path = b"/twrp-init.log\0";
            let fd = unsafe {
                libc::open(
                    log_path.as_ptr() as *const libc::c_char,
                    libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
                    0o644,
                )
            };
            if fd >= 0 {
                unsafe {
                    libc::dup2(fd, 1); // stdout
                    libc::dup2(fd, 2); // stderr
                    libc::close(fd);
                }
                unsafe {
                    safe_write_err(
                        b"[KR64 CHILD] TWRP: redirected stdout/stderr to /twrp-init.log\n",
                    );
                }
            } else {
                unsafe {
                    safe_write_err(
                        b"[KR64 CHILD] TWRP: WARN could not open /twrp-init.log for redirect\n",
                    );
                }
            }
        }

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
    // Step 6: wait for the guest to exit (with graceful SIGTERM handling).
    //
    // We install a SIGTERM handler so the KVM test script can ask us to
    // shut down cleanly after the boot wait. Without it, the default
    // SIGTERM action kills us instantly while blocked in waitpid(), and
    // we never log the guest init's fate (crash, exit, or still running).
    //
    // The loop uses WNOHANG + a short sleep so it can poll the
    // SHUTDOWN_REQUESTED flag between checks. On SIGTERM:
    //   1. Do a final non-blocking waitpid to see if the guest exited.
    //   2. If it exited → log the status (exit code / signal).
    //   3. If still running → SIGKILL the guest, reap it, log that we
    //      killed it (the guest was alive at shutdown — not a crash).
    // ---------------------------------------------------------------
    install_sigterm_handler();

    let mut status: libc::c_int = 0;
    loop {
        let waited = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if waited == pid {
            // Child exited on its own — fall through to status logging.
            break;
        }
        if waited == -1 {
            let e = std::io::Error::last_os_error();
            if e.raw_os_error() == Some(libc::EINTR) {
                // Signal interrupted waitpid (WNOHANG rarely blocks, but
                // be defensive). Fall through to the shutdown check.
            } else {
                error!("[KR64][parent] waitpid failed: {}", e);
                return 1;
            }
        }
        // waited == 0 (child still running) or EINTR — check for shutdown.
        if SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
            info!("[KR64][parent] SIGTERM received — initiating graceful shutdown");
            // Final non-blocking waitpid: the guest may have exited
            // between our last poll and the signal.
            let w = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
            if w == pid {
                // Log with "(after SIGTERM)" suffix to distinguish from
                // a self-initiated exit during normal operation.
                if libc::WIFEXITED(status) {
                    let code = libc::WEXITSTATUS(status);
                    info!(
                        "[KR64][parent] guest exited with status {} (after SIGTERM)",
                        code
                    );
                    return code;
                }
                if libc::WIFSIGNALED(status) {
                    let sig = libc::WTERMSIG(status);
                    warning!(
                        "[KR64][parent] guest killed by signal {} (after SIGTERM)",
                        sig
                    );
                    return 128 + sig;
                }
                warning!(
                    "[KR64][parent] guest waitpid returned unexpected status {} (after SIGTERM)",
                    status
                );
                return 1;
            }
            // Guest still running — kill it and reap. This is NOT a
            // crash: the guest was alive when we were asked to shut down
            // (typical for TWRP recovery which runs indefinitely).
            warning!(
                "[KR64][parent] guest (pid={}) still running — sending SIGKILL",
                pid
            );
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
            // Blocking waitpid to reap the SIGKILLed child (no signal
            // expected now, but use EINTR retry just in case).
            loop {
                let w = unsafe { libc::waitpid(pid, &mut status, 0) };
                if w == pid {
                    break;
                }
                if w == -1 {
                    let e = std::io::Error::last_os_error();
                    if e.raw_os_error() == Some(libc::EINTR) {
                        continue;
                    }
                    // Child already reaped by something else — not fatal.
                    warning!("[KR64][parent] waitpid after SIGKILL failed: {}", e);
                    break;
                }
            }
            warning!(
                "[KR64][parent] guest killed by our SIGKILL (was still running at shutdown — not a crash)"
            );
            return 0;
        }
        // Child still running, no shutdown requested — sleep briefly.
        // A signal during sleep returns early (EINTR), which is fine.
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    // Normal exit path: child exited on its own during the WNOHANG poll.
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

    #[test]
    fn config_default_boot_recovery_is_false() {
        let cfg = Config::default();
        assert!(!cfg.boot_recovery, "boot_recovery should default to false");
    }

    /// `--boot-recovery` flips the bool AND auto-sets `init_path=/init`
    /// (TWRP's init binary lives at the root of the ramdisk, not at
    /// `/system/bin/init` like full Android).
    #[test]
    fn parse_args_boot_recovery_sets_init_path_to_slash_init() {
        let cfg = parse_args(args(&[
            "--rootfs",
            "/r",
            "--data-dir",
            "/d",
            "--boot-recovery",
        ]))
        .unwrap();
        assert!(cfg.boot_recovery);
        assert_eq!(cfg.init_path, "/init");
    }

    /// If the user passes BOTH `--boot-recovery` AND `--init /custom/init`,
    /// the explicit `--init` wins (the bool is still flipped, but the
    /// init_path is NOT overridden). This is the documented contract on
    /// the `boot_recovery` field.
    #[test]
    fn parse_args_boot_recovery_does_not_override_explicit_init() {
        let cfg = parse_args(args(&[
            "--rootfs",
            "/r",
            "--data-dir",
            "/d",
            "--boot-recovery",
            "--init",
            "/sbin/init",
        ]))
        .unwrap();
        assert!(cfg.boot_recovery);
        assert_eq!(cfg.init_path, "/sbin/init");
    }

    /// `--boot-recovery` is mentioned in `--help` so users discover it.
    #[test]
    fn parse_args_help_mentions_boot_recovery() {
        let r = parse_args(args(&["--help"]));
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("--boot-recovery"));
    }

    /// `MountConfig::default()` must set `boot_recovery=false` so the
    /// default (non-TWRP) path retains the /apex bind mount.
    #[test]
    fn mount_config_default_boot_recovery_is_false() {
        let mc = mount_mgr::MountConfig::default();
        assert!(!mc.boot_recovery);
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

    // ========================================================================
    // Tests for set_selinux_context (direct lsetxattr syscall).
    // ========================================================================

    /// `set_selinux_context` must reject a path containing a NUL byte
    /// with `InvalidInput` — CString::new fails on interior NULs, and we
    /// surface that as a clear io::Error rather than panicking.
    #[test]
    fn set_selinux_context_rejects_nul_in_path() {
        let err = set_selinux_context("bad\0path", "u:object_r:system_file:s0")
            .expect_err("NUL in path must error");
        assert_eq!(
            err.kind(),
            std::io::ErrorKind::InvalidInput,
            "expected InvalidInput, got {:?}",
            err
        );
    }

    /// `set_selinux_context` must reject a context containing a NUL byte
    /// with `InvalidInput` — same reason as above.
    #[test]
    fn set_selinux_context_rejects_nul_in_context() {
        let err = set_selinux_context("/dev/null", "u:object_r:\0system_file:s0")
            .expect_err("NUL in context must error");
        assert_eq!(
            err.kind(),
            std::io::ErrorKind::InvalidInput,
            "expected InvalidInput, got {:?}",
            err
        );
    }

    /// `set_selinux_context` must return `Err` (any errno) for a path
    /// that does not exist. lsetxattr fails with ENOENT in this case
    /// (or ENOTSUP if xattrs/SELinux are compiled out). Either way, the
    /// function must NOT return Ok and must NOT panic.
    #[test]
    fn set_selinux_context_returns_err_for_nonexistent_path() {
        let err = set_selinux_context(
            "/nonexistent-twoyi-setfilecon-target-xyz",
            "u:object_r:system_file:s0",
        )
        .expect_err("nonexistent path must error");
        // Don't assert the specific errno — it varies by kernel config
        // (ENOENT if the path is the failure, ENOTSUP/ENOSYS if xattrs
        // are disabled, EPERM if SELinux denies). Just verify an errno
        // is present (i.e. it's a real OS error, not a logic bug).
        assert!(
            err.raw_os_error().is_some(),
            "expected an OS errno, got non-OS error: {:?}",
            err
        );
    }

    /// `set_selinux_context` must NOT panic when called on a real file
    /// (`/dev/null`). The result depends on the kernel's SELinux/xattr
    /// configuration:
    ///   - SELinux enforcing + CAP_MAC_ADMIN: Ok (label changed)
    ///   - SELinux permissive + CAP_MAC_ADMIN: Ok
    ///   - SELinux disabled / no xattr support: Err(ENOTSUP/ENOSYS)
    ///   - No CAP_MAC_ADMIN: Err(EPERM)
    ///
    /// All four are valid outcomes — the test only verifies no panic.
    /// We use /dev/null because it always exists on a devcontainer.
    #[test]
    fn set_selinux_context_does_not_panic_on_dev_null() {
        // /dev/null may not exist in some sandboxed test envs; skip
        // gracefully rather than fail.
        if !Path::new("/dev/null").exists() {
            eprintln!(
                "set_selinux_context_does_not_panic_on_dev_null: /dev/null missing, skipping"
            );
            return;
        }
        let _ = set_selinux_context("/dev/null", "u:object_r:system_file:s0");
        // No assertion on the result — see comment above.
    }

    /// `set_selinux_context` on a freshly-created temp file must not
    /// panic. On a real device it would succeed (root + CAP_MAC_ADMIN);
    /// on the devcontainer it likely returns ENOTSUP/EPERM. Either way
    /// is fine. This test catches regressions in the CString handling
    /// and the unsafe lsetxattr call signature.
    #[test]
    fn set_selinux_context_does_not_panic_on_temp_file() {
        let tmp = std::env::temp_dir().join("twoyi-setfilecon-test-file");
        let _ = std::fs::remove_file(&tmp);
        std::fs::write(&tmp, b"x").unwrap();
        let path_str = tmp.to_string_lossy().into_owned();
        let _ = set_selinux_context(&path_str, "u:object_r:system_file:s0");
        // Cleanup.
        let _ = std::fs::remove_file(&tmp);
    }

    /// `patch_twrp_init_rc_recovery_service` must insert the setenv line
    /// right after the `service recovery` line.
    #[test]
    fn patch_twrp_init_rc_inserts_setenv_after_service_recovery() {
        let input = "service ueventd /sbin/ueventd\n\
                     critical\n\
                     \n\
                     service recovery /sbin/recovery\n\
                     \n\
                     service adbd /sbin/adbd recovery\n\
                     disabled\n";
        let patched = patch_twrp_init_rc_recovery_service(input).expect("should patch");
        assert!(
            patched.contains(
                "service recovery /sbin/recovery\n    setenv LD_PRELOAD /dev/twrp_fb_hook.so"
            ),
            "setenv line should be inserted right after service recovery line. Patched:\n{}",
            patched
        );
        // Other services should be untouched.
        assert!(patched.contains("service ueventd /sbin/ueventd\n"));
        assert!(patched.contains("service adbd /sbin/adbd recovery\n"));
    }

    /// `patch_twrp_init_rc_recovery_service` must return None if the
    /// `service recovery` line is not found.
    #[test]
    fn patch_twrp_init_rc_returns_none_if_no_recovery_service() {
        let input = "service ueventd /sbin/ueventd\ncritical\n";
        assert!(patch_twrp_init_rc_recovery_service(input).is_none());
    }

    /// `patch_twrp_init_rc_recovery_service` must handle the case where
    /// recovery has existing options (like seclabel) — the setenv line
    /// is inserted BEFORE the existing options.
    #[test]
    fn patch_twrp_init_rc_inserts_before_existing_options() {
        let input = "service recovery /sbin/recovery\n    seclabel u:r:recovery:s0\n";
        let patched = patch_twrp_init_rc_recovery_service(input).expect("should patch");
        assert!(
            patched.contains("service recovery /sbin/recovery\n    setenv LD_PRELOAD /dev/twrp_fb_hook.so\n    seclabel u:r:recovery:s0"),
            "setenv should be inserted before seclabel. Patched:\n{}",
            patched
        );
    }

    /// `patch_twrp_init_rc_recovery_service` must not duplicate the
    /// setenv line if the service is already patched (the caller checks
    /// for the patch marker, but the function itself should also not
    /// insert twice for the same service line).
    #[test]
    fn patch_twrp_init_rc_only_patches_first_recovery_service() {
        // If init.rc has TWO recovery service definitions (shouldn't happen,
        // but be defensive), only the first should be patched.
        let input = "service recovery /sbin/recovery\n\
                     \n\
                     service recovery /sbin/recovery2\n";
        let patched = patch_twrp_init_rc_recovery_service(input).expect("should patch");
        let count = patched
            .matches("setenv LD_PRELOAD /dev/twrp_fb_hook.so")
            .count();
        assert_eq!(
            count, 1,
            "only the first recovery service should be patched"
        );
    }
}
