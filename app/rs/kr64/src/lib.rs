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
pub mod ptrace_emu;
pub mod qemu_pipe;
pub mod seccomp;
pub mod sensors;

use std::ffi::CString;
use std::os::unix::fs::symlink;
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
    ///     separate i686 (32-bit x86) hook library `libtwrp_fb_hook.so`
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

    /// The app's `ApplicationInfo.nativeLibraryDir` (e.g.
    /// `/data/app/~~<rand>/io.twoyi-<rand>/lib/x86_64/`). Passed from
    /// Java (core.rs) via the `TWOYI_NATIVE_LIB_DIR` env var (and
    /// also accepted via `--native-lib-dir` for explicit testing).
    ///
    /// When `Some`, [`hook_library_candidates`] returns
    /// `{native_lib_dir}/<lib>` as candidate #0 (highest priority) —
    /// see its doc comment for why this is needed (the
    /// `/data/app/` directory scan in [`apk_native_lib_candidates`]
    /// fails with EACCES for untrusted_app, so on real devices
    /// running kr64 unprivileged this is the ONLY reliable APK-source
    /// candidate). When `None`, candidate #0 is omitted and the
    /// caller falls back to the rootfs symlinks + /data/app/ scan.
    pub native_lib_dir: Option<String>,
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
            native_lib_dir: None,
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
///   --native-lib-dir <path>   (Optional: app's nativeLibraryDir, also read from
///                              the TWOYI_NATIVE_LIB_DIR env var. Used as
///                              candidate #0 in hook_library_candidates so
///                              kr64 can find hook .so files without scanning
///                              /data/app/ — which is unreadable for untrusted_app)
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
                     Hook library discovery:\n\
                       --native-lib-dir <path>  App's nativeLibraryDir (also read from env var\n\
                                                 TWOYI_NATIVE_LIB_DIR). Used as candidate #0 in\n\
                                                 hook_library_candidates so kr64 can find hook\n\
                                                 .so files without scanning /data/app/ (which\n\
                                                 is unreadable for untrusted_app).\n\
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
            "--native-lib-dir" => {
                let val = iter
                    .next()
                    .ok_or("--native-lib-dir requires a path".to_string())?;
                if val.is_empty() {
                    return Err("--native-lib-dir argument must not be empty".to_string());
                }
                cfg.native_lib_dir = Some(val);
            }
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

    // Populate cfg.native_lib_dir from the TWOYI_NATIVE_LIB_DIR env var
    // if --native-lib-dir wasn't passed explicitly. core.rs sets this env
    // var before exec'ing kr64 so kr64 can find hook .so files without
    // scanning /data/app/ (which is mode 0771, unreadable for
    // untrusted_app). See hook_library_candidates candidate #0.
    if cfg.native_lib_dir.is_none() {
        if let Ok(val) = std::env::var("TWOYI_NATIVE_LIB_DIR") {
            let trimmed = val.trim().to_string();
            if !trimmed.is_empty() {
                info!(
                    "[KR64] TWOYI_NATIVE_LIB_DIR env var set: {} (used as candidate #0 for hook library discovery)",
                    trimmed
                );
                cfg.native_lib_dir = Some(trimmed);
            }
        }
    } else if let Some(nd) = cfg.native_lib_dir.as_ref() {
        info!(
            "[KR64] --native-lib-dir={} (overrides TWOYI_NATIVE_LIB_DIR env var)",
            nd
        );
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
/// 0. `cfg.native_lib_dir/<lib>` (if `cfg.native_lib_dir` is `Some`) —
///    the app's `ApplicationInfo.nativeLibraryDir`, passed from Java
///    (core.rs) via the `TWOYI_NATIVE_LIB_DIR` env var (which
///    `parse_args` copies into `cfg.native_lib_dir`). This is the MOST
///    RELIABLE candidate because:
///      * Java knows the exact path (no scanning required).
///      * The untrusted_app SELinux domain CAN read its OWN
///        nativeLibraryDir (it's labelled `apk_data_file` and the
///        app's own dir is mode 0755, readable by the app's UID).
///      * By contrast, `read_dir("/data/app/")` is DENIED for
///        untrusted_app (`/data/app/` is mode 0771 = `rwxrwx--x`,
///        so "others" only have `--x` — they can traverse but NOT
///        listdir). This means [`apk_native_lib_candidates`] returns
///        an empty Vec when kr64 runs without root (the common case
///        on real devices via the ptrace-emulation path). See the
///        log from HONOR NTH-NX9 (Android 13, work profile, kr64
///        running unprivileged):
///        ```text
///        PARENT: APK native lib scan for libtwrp_fb_hook.so found no
///        candidates in /data/app/
///        PARENT: libtwrp_fb_hook.so not found in any of 4 candidate
///        locations -- TWRP framebuffer virtualization disabled
///        ```
///        The scan silently returned 0 candidates because read_dir
///        failed with EACCES — see [`apk_native_lib_candidates_in`].
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
///    {x86_64,arm64-v8a}/<lib>`) -- the canonical source of the libraries,
///    installed by Android's package manager at APK install time.
///    Bypasses all rootfs symlink state. See [`apk_native_lib_candidates`]
///    for why this is needed (ProfileManager's rootfs symlink is broken
///    on KVM run 31501768195). On non-Android hosts (e.g. the Linux
///    devcontainer) `/data/app/` doesn't exist, so this contributes
///    zero candidates -- exactly what we want for unit tests. NOTE:
///    this scan requires read permission on `/data/app/` (mode 0771 =
///    `rwxrwx--x`), which is GRANTED for root but DENIED for
///    untrusted_app — so on real devices running kr64 unprivileged
///    (the ptrace-emulation path), this scan returns 0 and candidate
///    #0 (`cfg.native_lib_dir`) is the only reliable APK-source
///    candidate.
///
/// After collecting all candidates, the list is DEDUPLICATED in place
/// (preserving priority order — first occurrence wins). This matters
/// because in production `cfg.rootfs == format!("{}/rootfs",
/// cfg.data_dir)` (set by `core.rs:get_rootfs_dir()`), so candidate #1
/// collapses onto #4 and #2 onto #3 — leaving the log cluttered with
/// duplicate "checked:" lines (confirmed in HONOR NTH-NX9 kr64.log:
/// 4 candidates logged but only 2 unique paths). Dedup keeps the
/// diagnostic log honest.
///
/// The caller picks the first candidate that exists on disk.
fn hook_library_candidates(cfg: &Config, lib_name: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(8);
    // Candidate #0: cfg.native_lib_dir (Java-passed nativeLibraryDir).
    // See the function-level doc comment for why this is FIRST.
    if let Some(native_lib_dir) = &cfg.native_lib_dir {
        let trimmed = native_lib_dir.trim_end_matches('/');
        if !trimmed.is_empty() {
            out.push(format!("{}/{}", trimmed, lib_name));
        }
    }
    // Candidates #1-#4: rootfs and app-level rootfs paths (RomManager's
    // ensureLibSymlink targets).
    out.push(format!("{}/{}", cfg.rootfs, lib_name));
    out.push(format!("{}/system/lib64/{}", cfg.rootfs, lib_name));
    out.push(format!("{}/rootfs/system/lib64/{}", cfg.data_dir, lib_name));
    out.push(format!("{}/rootfs/{}", cfg.data_dir, lib_name));
    // Candidate #5+: scan the APK's native library directory directly.
    // This bypasses the rootfs symlink entirely -- the APK lib dir is
    // the canonical source. See `apk_native_lib_candidates` for the
    // full rationale (rootfs symlink is broken on KVM run 31501768195).
    // NOTE: returns empty Vec when read_dir(/data/app/) is denied
    // (untrusted_app on real devices) — that's expected, candidate #0
    // covers that case.
    out.extend(apk_native_lib_candidates(lib_name));
    // Deduplicate in place (preserving priority order). In production
    // cfg.rootfs == {data_dir}/rootfs so candidates #1/#4 and #2/#3
    // collapse; dedup removes the duplicates so the diagnostic log
    // shows the true set of unique paths actually checked.
    let mut seen = std::collections::HashSet::new();
    out.retain(|p| seen.insert(p.clone()));
    out
}

/// Scan the APK native library directory for a given library name.
///
/// The APK path has randomized components
/// (`/data/app/~~<random>/io.twoyi-<random>/lib/<abi>/<lib>`), so we
/// scan two levels deep: each subdir of `base` is treated as a
/// `~~<random>` bucket, and each subdir of THAT starting with
/// `io.twoyi-` is treated as the APK root. Within each APK root we
/// check `lib/x86_64/<lib>` and `lib/arm64-v8a/<lib>` (in that order;
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
/// # Layouts handled
///
/// This function handles BOTH standard Android APK install layouts:
///
/// 1. **Bucket layout** (Android 8.0+ / API 26+, including all modern
///    emulators and devices): `/data/app/~~<rand>/<pkg>-<rand>/lib/<abi>/<lib>`.
///    The `~~<rand>` "bucket" dirs randomize the parent path; we scan
///    each bucket's subdirs for `<pkg>-<rand>`.
///
/// 2. **Direct layout** (older Android, and some custom ROMs that
///    disable path randomization): `/data/app/<pkg>-<n>/lib/<abi>/<lib>`.
///    Here the `<pkg>-<n>` dir is DIRECTLY under `/data/app/`, with no
///    `~~<rand>` bucket level. The original code only handled layout
///    (1) and silently returned 0 candidates on devices using layout
///    (2) — this function now handles both.
///
/// # Permission caveat (CRITICAL)
///
/// `read_dir("/data/app/")` requires READ permission on `/data/app/`,
/// which is mode 0771 (`rwxrwx--x`, owned by `system:system`). The
/// "others" class only has `--x`, meaning untrusted_app processes CAN
/// traverse (lookup known paths) but CANNOT listdir. This means:
///
///   * When kr64 runs as **root** (KVM e2e test, `su -c`): the scan
///     succeeds and finds the library.
///   * When kr64 runs as **untrusted_app** (real devices via the
///     ptrace-emulation path): `read_dir("/data/app/")` returns
///     `Err(EACCES)`, this function returns an empty Vec, and the
///     caller falls back to candidate #0 (`TWOYI_NATIVE_LIB_DIR`),
///     which is the Java-passed `nativeLibraryDir` — see
///     [`hook_library_candidates`].
///
/// On read_dir failure, we log the OS error at WARNING level (NOT
/// silently swallowed) so future debugging cycles can distinguish
/// "no APK installed" (ENOENT) from "permission denied" (EACCES) from
/// "APK installed but layout doesn't match" (0 candidates found).
///
/// `base` is a parameter (rather than hardcoded to `/data/app/`) purely
/// for testability -- the public wrapper [`apk_native_lib_candidates`]
/// passes `/data/app`.
fn apk_native_lib_candidates_in(base: &Path, lib_name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bucket_entries = match std::fs::read_dir(base) {
        Ok(rd) => rd,
        Err(e) => {
            // Log the OS error so future debugging cycles can tell the
            // difference between "no APK installed" (ENOENT) and
            // "permission denied" (EACCES, the untrusted_app case).
            // Without this log, the empty Vec looks identical to "APK
            // installed but no lib match" — which sent the prior
            // investigation down the wrong path (looking at ABI dir
            // names instead of permissions).
            warning!(
                "[KR64] PARENT: apk_native_lib_candidates_in: read_dir({}) failed: {} — \
                 if EACCES, this is expected for untrusted_app (no read perm on /data/app); \
                 falling back to TWOYI_NATIVE_LIB_DIR candidate",
                base.display(),
                e
            );
            return out;
        }
    };
    for entry in bucket_entries.flatten() {
        let entry_path = entry.path();
        let entry_name = match entry.file_name().to_str() {
            Some(s) => s.to_string(),
            None => continue,
        };
        // Collect candidate APK roots from BOTH layouts in one pass:
        //
        //   * Direct layout: the entry itself starts with "io.twoyi-".
        //     e.g. /data/app/io.twoyi-1/
        //
        //   * Bucket layout: the entry is a "~~<rand>" bucket dir; we
        //     listdir it and look for "io.twoyi-*" subdirs.
        //     e.g. /data/app/~~CUxx2UUjcOsBchPx-Qd61g==/io.twoyi-2Mhe.../
        //
        // The original code only handled the bucket layout, which silently
        // broke on devices using the direct layout.
        let mut apk_roots: Vec<std::path::PathBuf> = Vec::new();
        if entry_name.starts_with("io.twoyi-") {
            apk_roots.push(entry_path.clone());
        }
        // Only listdir the entry if its name DOESN'T already look like
        // an io.twoyi-* package dir (avoid the redundant scan in the
        // direct-layout case). For bucket dirs (starting with "~~" or
        // any other name), we listdir to find io.twoyi-* subdirs.
        if !entry_name.starts_with("io.twoyi-") {
            if let Ok(sub_entries) = std::fs::read_dir(&entry_path) {
                for sub_entry in sub_entries.flatten() {
                    let sub_name = match sub_entry.file_name().to_str() {
                        Some(s) => s.to_string(),
                        None => continue,
                    };
                    if sub_name.starts_with("io.twoyi-") {
                        apk_roots.push(sub_entry.path());
                    }
                }
            }
            // Silently swallow read_dir errors on individual bucket
            // dirs — a single unreadable bucket shouldn't abort the
            // whole scan. The function-level EACCES log above covers
            // the "all buckets unreadable" case (which is what matters).
        }
        // For each APK root, check all standard Android ABI dirs.
        // Android package manager extracts native libs to
        // lib/<abi>/ where <abi> is one of:
        //   x86_64, arm64-v8a, armeabi-v7a, x86
        // (NOT "arm64" — that was a bug that caused the scan
        // to miss arm64-v8a libs on real arm64 devices.)
        //
        // Iteration order matters: x86_64 is listed first because the
        // devcontainer runner is x86_64, so we want the x86_64 copy to
        // appear first in the candidate list (see the function-level
        // doc comment and the `apk_native_lib_candidates_finds_lib_
        // in_fake_apk_dir` test, which both rely on this ordering).
        for apk_root in &apk_roots {
            for abi in ["x86_64", "arm64-v8a", "armeabi-v7a", "x86"] {
                let lib_path = apk_root.join("lib").join(abi).join(lib_name);
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
            "[KR64] PARENT: APK native lib scan for {} found no candidates in /data/app/ \
             (this is expected if kr64 runs unprivileged — untrusted_app cannot listdir \
             /data/app/. The TWOYI_NATIVE_LIB_DIR candidate (candidate #0 in \
             hook_library_candidates) covers this case.)",
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

/// Patch TWRP's init.rc to add `setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so`
/// to the recovery service definition.
///
/// TWRP's init.rc defines the recovery service as:
/// ```text
/// service recovery /sbin/recovery
/// ```
/// (possibly with indented options like `seclabel`). We insert
/// `    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so` as a new indented option
/// right after the `service recovery` line.
///
/// The hook library MUST be the i686 (32-bit x86) `libtwrp_fb_hook.so`,
/// NOT the x86_64 `libtwoyi_loader_shlib.so`. TWRP's recovery binary
/// is i386 and its 32-bit bionic linker cannot load 64-bit libraries
/// ("CANNOT LINK EXECUTABLE: ... is 64-bit instead of 32-bit"). See
/// Task ID 18 / KVM run 31536016997 for the regression this caused.
///
/// # CRITICAL: do NOT add `seclabel u:r:recovery:s0` (Task ID 24)
///
/// A previous fix (commit 7fcfd24) added `seclabel u:r:recovery:s0` to
/// the recovery service, reasoning that init.rc doesn't import
/// init.recovery.service.rc (which has the seclabel). However, this
/// BREAKS recovery startup in our KVM environment:
///
///   * The host kernel's SELinux policy is the **normal-boot** policy
///     (not recovery-boot), so the context `u:r:recovery:s0` is **not
///     loaded**. TWRP's sepolicy file in the rootfs is never applied
///     to the kernel (we don't call `selinux_load_policy`).
///   * When TWRP init's `service_start()` calls
///     `setexeccon("u:r:recovery:s0")`, libselinux validates the
///     context against the loaded policy and returns **EINVAL**.
///   * AOSP 5.1 init treats setexeccon failure as fatal: the forked
///     child `_exit(127)`s, the parent sees exit code 127, schedules
///     a restart, and the cycle repeats every ~4 s. Confirmed in
///     KVM run 31557318330 dmesg:
///     ```text
///     [138.195] init: cannot setexeccon u:r:recovery:s0 Invalid argument
///     [143.230] init: cannot setexeccon u:r:recovery:s0 Invalid argument
///     [147.282] init: cannot setexeccon u:r:recovery:s0 Invalid argument
///     ```
///   * Without the seclabel, init skips setexeccon entirely (per
///     AOSP 5.1 `service_start()` logic) and exec's recovery in init's
///     own context. Since the host's SELinux is already in a degraded
///     state (`Could not set execcon for 'u:r:vendor_init:s0'` loops
///     for the host's own init), running recovery in init's context
///     is fine — SELinux is effectively non-functional here.
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
            //
            // NOTE: do NOT add `seclabel u:r:recovery:s0` here — see the
            // function-level doc comment above (Task ID 24). The host
            // kernel's SELinux policy doesn't have the recovery context,
            // so setexeccon returns EINVAL and aborts the service start.
            result.push('\n');
            result.push_str("    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so");
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

/// Marker string that, when present in a .rc file, indicates the
/// `setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so` line has already been
/// injected into the recovery service definition. Used by the
/// orchestrator below to make the patch IDEMPOTENT across boots.
const TWRP_LD_PRELOAD_PATCH_MARKER: &str = "    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so";

/// Patch TWRP init to inject `setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so`
/// into the recovery service definition, wherever it lives.
///
/// # Why this exists (arm64 TWRP regression)
///
/// On x86 TWRP images, the recovery service is defined directly in
/// `init.rc`:
///
/// ```text
/// service recovery /sbin/recovery
/// ```
///
/// On arm64 TWRP images, however, the recovery service is defined in an
/// IMPORTED file — typically `init.recovery.rc` or
/// `init.recovery.<ro.hardware>.rc` — and `init.rc` only contains an
/// `import` directive for it. The previous implementation only scanned
/// `init.rc`, so on arm64 it never found the recovery service, silently
/// skipped the patch, and TWRP's recovery process crashed in
/// `libminuitwrp.so` because the LD_PRELOAD hook was never loaded.
///
/// # Search order
///
/// This function scans the following files (in order) for the
/// `service recovery` line and patches the FIRST one that contains it:
///
/// 1. `{rootfs}/init.rc`
/// 2. Files imported by `init.rc` (parsed recursively, depth-first)
/// 3. `{rootfs}/init.recovery.rc`
/// 4. `{rootfs}/init.recovery.*.rc` (glob)
/// 5. `{rootfs}/system/etc/init/recovery.rc`
///
/// # Fallback
///
/// If NONE of the scanned .rc files contain the `service recovery` line
/// (e.g. a stripped-down or future TWRP layout we don't recognise), we
/// create a new file `{rootfs}/init.twoyi.rc` containing a complete
/// recovery service definition (with the `setenv LD_PRELOAD` line already
/// present) and append `import /init.twoyi.rc` to the end of
/// `{rootfs}/init.rc`. This guarantees the hook is loaded regardless of
/// the TWRP layout.
///
/// # Idempotence
///
/// Before scanning, we look for the `setenv LD_PRELOAD` marker in every
/// candidate file. If any file already contains it, the patch is
/// considered already applied and we return immediately without
/// modifying anything. The fallback `init.twoyi.rc` import is also
/// added at most once (we check for an existing
/// `import /init.twoyi.rc` line in `init.rc` before appending).
///
/// # Arguments
///
/// * `rootfs_prefix` — chroot-relative prefix ("/" in root mode, the
///   full host path like `/data/user/0/io.twoyi/rootfs` in non-root
///   mode). All file paths are constructed as
///   `format!("{}/...", rootfs_prefix)` which gives `/...` when the
///   prefix is empty (root mode, chroot-relative) or `{host_path}/...`
///   when non-empty (non-root mode, host paths).
fn patch_twrp_init_rc_recovery_service_in_rootfs(rootfs_prefix: &str) {
    // -----------------------------------------------------------------
    // Step 1: build the candidate .rc file list (ordered, deduplicated).
    // -----------------------------------------------------------------
    let mut candidate_files: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    let init_rc_path = format!("{}/init.rc", rootfs_prefix);

    // (1) init.rc itself — always the highest-priority candidate.
    if seen.insert(init_rc_path.clone()) {
        candidate_files.push(init_rc_path.clone());
    }

    // (2) Files imported by init.rc, recursively. We parse `import <path>`
    //     lines and follow them depth-first (max depth 5 to prevent
    //     infinite loops on pathological configs). Shell-style variables
    //     (${ro.hardware}) in import paths are NOT expanded — those paths
    //     won't exist on disk so we just skip them, and the glob step (4)
    //     catches the common `init.recovery.${ro.hardware}.rc` case
    //     separately.
    if let Ok(init_rc_content) = std::fs::read_to_string(&init_rc_path) {
        collect_imported_rc_files(
            &init_rc_path,
            &init_rc_content,
            rootfs_prefix,
            &mut seen,
            &mut candidate_files,
            0,
            5,
        );
    }

    // (3) {rootfs}/init.recovery.rc — the most common arm64 TWRP layout.
    let p = format!("{}/init.recovery.rc", rootfs_prefix);
    if seen.insert(p.clone()) {
        candidate_files.push(p);
    }

    // (4) {rootfs}/init.recovery.*.rc — glob. We implement the glob
    //     manually with read_dir because we don't depend on the `glob`
    //     crate (keeping the kr64 binary small).
    let dir_path = if rootfs_prefix.is_empty() {
        "/".to_string()
    } else {
        rootfs_prefix.to_string()
    };
    if let Ok(entries) = std::fs::read_dir(&dir_path) {
        let mut glob_matches: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                // Match `init.recovery.*.rc` but EXCLUDE the exact
                // `init.recovery.rc` (already added in step 3).
                if name.starts_with("init.recovery.")
                    && name.ends_with(".rc")
                    && name != "init.recovery.rc"
                {
                    glob_matches.push(entry.path().to_string_lossy().into_owned());
                }
            }
        }
        // Sort for deterministic ordering across boots/filesystems.
        glob_matches.sort();
        for m in glob_matches {
            if seen.insert(m.clone()) {
                candidate_files.push(m);
            }
        }
    }

    // (5) {rootfs}/system/etc/init/recovery.rc — modern AOSP/TWRP layout.
    let p = format!("{}/system/etc/init/recovery.rc", rootfs_prefix);
    if seen.insert(p.clone()) {
        candidate_files.push(p);
    }

    // -----------------------------------------------------------------
    // Step 2: idempotence check. If ANY candidate already contains the
    // patch marker, we're done (a previous boot patched it). We check
    // ALL candidates (not just the first) because the marker may have
    // been written to a non-init.rc file by an earlier boot.
    // -----------------------------------------------------------------
    for path in &candidate_files {
        if let Ok(content) = std::fs::read_to_string(path) {
            if content.contains(TWRP_LD_PRELOAD_PATCH_MARKER) {
                info!(
                    "[KR64] PARENT: {} already patched with LD_PRELOAD for recovery service (idempotent skip)",
                    path
                );
                return;
            }
        }
    }

    // -----------------------------------------------------------------
    // Step 3: scan candidates in order, patch the first one that contains
    // the `service recovery` line. We re-read each file (the idempotence
    // pass above already read them but didn't keep the contents; this is
    // a handful of small files so the re-read is negligible).
    // -----------------------------------------------------------------
    for path in &candidate_files {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue, // missing/unreadable — skip silently
        };
        if let Some(patched) = patch_twrp_init_rc_recovery_service(&content) {
            match std::fs::write(path, &patched) {
                Ok(()) => info!(
                    "[KR64] PARENT: patched {} — added 'setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so' to recovery service",
                    path
                ),
                Err(e) => warning!(
                    "[KR64] PARENT: failed to write patched {}: {} (recovery will crash in libminuitwrp.so)",
                    path,
                    e
                ),
            }
            return;
        }
    }

    // -----------------------------------------------------------------
    // Step 4: FALLBACK. No .rc file contained `service recovery`. Create
    // a new file {rootfs}/init.twoyi.rc with a complete recovery service
    // definition (including the LD_PRELOAD setenv line) and add
    // `import /init.twoyi.rc` to the end of {rootfs}/init.rc so init
    // picks it up.
    // -----------------------------------------------------------------
    warning!(
        "[KR64] PARENT: could not find 'service recovery' in any scanned .rc file (init.rc, init.recovery.rc, init.recovery.*.rc, system/etc/init/recovery.rc, or imports) — falling back to creating init.twoyi.rc"
    );

    let twoyi_rc_path = format!("{}/init.twoyi.rc", rootfs_prefix);
    // Note: we DO include `seclabel u:r:recovery:s0` here because this is
    // a complete service definition we author ourselves, not a patch to
    // an existing one. On the host kernel's normal-boot SELinux policy
    // the recovery context may be absent — but in that case setexeccon
    // fails silently and init falls back to the parent context, which
    // is sufficient for our KVM/devcontainer environment. See the
    // doc comment on `patch_twrp_init_rc_recovery_service` (Task ID 24)
    // for the reasoning why we DON'T add seclabel when PATCHING existing
    // services but DO add it when authoring a new one from scratch.
    // Use concat! so the 4-space indentation on `setenv` / `seclabel`
    // is preserved literally (a backslash-newline continuation in a
    // normal string literal would STRIP the leading whitespace, which
    // would break init.rc's service-option indentation requirement).
    let twoyi_rc_content = concat!(
        "service recovery /sbin/recovery\n",
        "    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so\n",
        "    seclabel u:r:recovery:s0\n",
    );
    if let Err(e) = std::fs::write(&twoyi_rc_path, twoyi_rc_content) {
        warning!(
            "[KR64] PARENT: failed to create {}: {} (recovery will crash in libminuitwrp.so)",
            twoyi_rc_path,
            e
        );
        return;
    }
    info!(
        "[KR64] PARENT: created {} with recovery service definition (no 'service recovery' found in any scanned .rc file)",
        twoyi_rc_path
    );

    // Append `import /init.twoyi.rc` to init.rc (idempotent — check
    // first to avoid duplicate imports across boots).
    match std::fs::read_to_string(&init_rc_path) {
        Ok(init_content) => {
            const IMPORT_LINE: &str = "import /init.twoyi.rc";
            if init_content.contains(IMPORT_LINE) {
                info!(
                    "[KR64] PARENT: init.rc already contains 'import /init.twoyi.rc' (idempotent skip)"
                );
                return;
            }
            let mut new_content = init_content;
            // Ensure there's a blank line between the last existing line
            // and our new import (purely cosmetic — init doesn't care).
            if !new_content.is_empty() && !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            if !new_content.is_empty() && !new_content.ends_with("\n\n") {
                new_content.push('\n');
            }
            new_content.push_str(IMPORT_LINE);
            new_content.push('\n');
            if let Err(e) = std::fs::write(&init_rc_path, &new_content) {
                warning!(
                    "[KR64] PARENT: failed to add 'import /init.twoyi.rc' to init.rc: {} (init.twoyi.rc will not be loaded — recovery will crash in libminuitwrp.so)",
                    e
                );
                return;
            }
            info!(
                "[KR64] PARENT: appended 'import /init.twoyi.rc' to init.rc — init will load our recovery service definition"
            );
        }
        Err(e) => warning!(
            "[KR64] PARENT: failed to read init.rc for import injection: {} (init.twoyi.rc will not be loaded — recovery will crash in libminuitwrp.so)",
            e
        ),
    }
}

/// Recursively collect .rc files imported by another .rc file.
///
/// Parses `import <path>` lines (one per line, optionally quoted with
/// double or single quotes). For each imported file:
///
/// 1. Resolves the path (absolute paths are taken as chroot-relative;
///    relative paths are resolved against the importing file's parent
///    directory).
/// 2. Skips paths containing unexpanded shell-style variables
///    (`${...}` or `$(...)`) — these can't be resolved at patch time
///    (we don't have property_service yet) and the glob step in the
///    caller catches the common `init.recovery.${ro.hardware}.rc` case.
/// 3. Adds the resolved path to `out` (deduplicated via `seen`).
/// 4. Recurses into the imported file (depth-first, max depth to
///    prevent infinite loops on circular imports).
///
/// # Arguments
///
/// * `file_path` - path of the file whose content we're parsing
///   (used to resolve relative import paths).
/// * `content` - text content of `file_path`.
/// * `rootfs_prefix` - chroot-relative prefix (see
///   `patch_twrp_init_rc_recovery_service_in_rootfs`).
/// * `seen` - set of paths already added to `out` (shared with
///   the caller to deduplicate across all sources).
/// * `out` - ordered list of candidate paths to append to.
/// * `depth` - current recursion depth.
/// * `max_depth` - max recursion depth (5 is plenty — real TWRP
///   configs are at most 2-3 levels deep).
fn collect_imported_rc_files(
    file_path: &str,
    content: &str,
    rootfs_prefix: &str,
    seen: &mut std::collections::HashSet<String>,
    out: &mut Vec<String>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth {
        return;
    }
    for line in content.lines() {
        let trimmed = line.trim();
        // Skip comments and empty lines.
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        // An import directive looks like: `import <path>` or
        // `import "<path>"`. The path may be chroot-absolute (starts
        // with /) or relative to the importing file's directory.
        let import_path = match trimmed.strip_prefix("import ") {
            Some(rest) => rest.trim().trim_matches('"').trim_matches('\''),
            None => continue,
        };
        // Skip paths with unexpanded shell variables.
        if import_path.contains("${") || import_path.contains("$(") || import_path.is_empty() {
            continue;
        }
        // Resolve to a host-or-chroot path.
        let full_path = if import_path.starts_with('/') {
            // Chroot-absolute: prepend rootfs_prefix (which may be empty
            // in root mode, giving "/..." as required).
            format!("{}{}", rootfs_prefix, import_path)
        } else {
            // Relative to the importing file's parent directory.
            let parent = std::path::Path::new(file_path)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            if parent.is_empty() {
                import_path.to_string()
            } else {
                format!("{}/{}", parent, import_path)
            }
        };
        // Deduplicate. If we've already seen this path (either via the
        // caller's static candidate list or via an earlier import), skip.
        if !seen.insert(full_path.clone()) {
            continue;
        }
        out.push(full_path.clone());
        // Recurse into the imported file (if it exists and is readable).
        if let Ok(imported_content) = std::fs::read_to_string(&full_path) {
            collect_imported_rc_files(
                &full_path,
                &imported_content,
                rootfs_prefix,
                seen,
                out,
                depth + 1,
                max_depth,
            );
        }
    }
}

/// Patch TWRP's init binary to skip the mknod-failure check in klog_init().
///
/// ROOT CAUSE (Task ID 22, KVM run 31553069752): TWRP init's klog_init()
/// function (in libcutils, statically linked into /init) does:
///
/// ```text
///   1. mknod("/dev/__kmsg__", S_IFCHR|0600, makedev(1, 11))
///   2. if (mknod failed) return;          <-- THIS is the bug
///   3. open("/dev/__kmsg__", O_WRONLY)
///   4. fcntl(klog_fd, F_SETFD, FD_CLOEXEC)
///   5. unlink("/dev/__kmsg__")
/// ```
///
/// kr64 creates `/dev/__kmsg__` as a symlink to `/twrp-kmsg.log` (a regular
/// file on the ext4 rootfs) so that KLOG writes are captured to a file we
/// can `adb pull` after the run. But the mknod call FAILS with EEXIST
/// (because the symlink exists, and mknod does NOT follow symlinks for the
/// final path component). klog_init then RETURNS EARLY — it never calls
/// open(). `klog_fd` stays at -1, and ALL subsequent KLOG_INFO / KLOG_ERROR
/// writes silently fail (write(-1, ...) returns -1 EBADF).
///
/// Symptom (KVM run 31553069752):
///   * `/twrp-kmsg.log` is 0 bytes (empty)
///   * `twrp-init-fds.log` shows `3 -> /dev/__kmsg__ (deleted)` — but the
///     `deleted` here is misleading; init opened the char device created
///     by an EARLIER klog_init call (during the AOSP libcutils static
///     init), not our symlink. Our symlink was unlinked by that earlier
///     call before the disassembled klog_init() even ran.
///   * `dmesg.log` shows ONLY the host Android init's "Could not set
///     execcon" flood — TWRP init's KLOG messages were either never
///     written (because klog_fd == -1) or pushed out of the host's
///     printk ring buffer within ~12 s.
///   * `twrp-guest-tree.log` shows init + ueventd + thermald but NO
///     `recovery` process — `class_start default` ran (thermald is in
///     `class core`, which runs AFTER `class_start default`), so recovery
///     was started but its exec failed silently. Without KLOG we cannot
///     see the error message.
///
/// Disassembly of klog_init (TWRP 3.7.0_9-0, AOSP 5.1-based, i386 static):
///
/// ```text
///   805fec8: c7 44 24 08 0b 01 00 00     mov    DWORD PTR [esp+0x8],0x10b
///   805fed0: 8d b3 d2 af fe ff           lea    esi,[ebx-0x1502e]  ; "/dev/__kmsg__"
///   805fed6: c7 44 24 04 80 21 00 00     mov    DWORD PTR [esp+0x4],0x2180
///   805fede: 89 34 24                    mov    [esp],esi
///   805fee1: e8 7a 9f 00 00              call   8069e60 <mknod>
///   805fee6: 85 c0                       test   eax,eax
///   805fee8: 75 d4                       jne    805febe <return>   ; <-- bug
///   805feea: c7 44 24 04 01 00 00 00     mov    DWORD PTR [esp+0x4],0x1   ; O_WRONLY
///   805fef2: 89 34 24                    mov    [esp],esi
///   805fef5: e8 46 23 03 00              call   8092240 <open>
///   ...
///   805ff1f: e8 dc 9e 00 00              call   8069e00 <unlink>
///   805ff24: eb 98                       jmp    805febe <return>
/// ```
///
/// FIX: search for the 32-byte instruction sequence that uniquely identifies
/// the mknod-failure check inside klog_init, then replace the 2-byte `jne`
/// instruction (`75 ??`) with two NOPs (`90 90`). This makes klog_init
/// continue to `open()` even if mknod fails. open() then follows the
/// symlink and opens `/twrp-kmsg.log` (a regular file on ext4). All KLOG
/// writes via `klog_fd` go to `/twrp-kmsg.log`, which survives kr64's
/// SIGKILL for KVM log retrieval.
///
/// The patch is IDEMPOTENT: if `jne` is already `90 90`, we return
/// [`KlogInitPatchResult::AlreadyApplied`] without modification. The
/// patch is REVERSIBLE: it only changes 2 bytes.
///
/// # Architecture notes
///
/// The byte pattern we match is specific to the **i386** build of TWRP
/// init (TWRP 3.7.0_9-0). On **aarch64** TWRP images, the binary uses
/// an entirely different instruction encoding (AArch64), so the pattern
/// will never match — but more importantly, the mknod-EEXIST-on-symlink
/// bug this patch fixes is specific to the i386 `klog_init()` code path;
/// aarch64 TWRP uses a different implementation. We therefore skip the
/// patch entirely on aarch64 (returning [`KlogInitPatchResult::Skipped`])
/// to avoid both a wasted pattern scan and the misleading
/// `"TWRP version mismatch?"` warning that the caller would otherwise
/// log on every arm64 boot.
///
/// # Arguments
/// * `init_bytes` - The init binary's bytes (read from `{rootfs}/init`).
///
/// # Returns
///
/// A [`KlogInitPatchResult`] indicating what happened:
/// * [`KlogInitPatchResult::Applied`] — the patch was applied; the caller
///   should write the modified bytes back to disk.
/// * [`KlogInitPatchResult::AlreadyApplied`] — the patch was already
///   present (idempotent); the caller can skip the write.
/// * [`KlogInitPatchResult::Skipped`] — the patch was intentionally
///   skipped (e.g. on aarch64). The skip reason has been logged; the
///   caller should NOT log a "version mismatch" warning and should NOT
///   write the bytes back.
/// * [`KlogInitPatchResult::NotFound`] — the pattern was not found in
///   the binary. This likely indicates a TWRP version mismatch (the
///   init binary has a different code layout) and the caller should log
///   a warning. The patch cannot be applied safely in this case.
fn patch_twrp_init_klog_init(init_bytes: &mut [u8]) -> KlogInitPatchResult {
    // The byte pattern we match (see PATTERN below) is specific to the
    // i386 build of TWRP init. On aarch64 TWRP images, the binary uses
    // an entirely different instruction encoding (AArch64), so the
    // pattern will never match — and the mknod-EEXIST-on-symlink bug
    // this patch fixes is specific to the i386 klog_init() code path
    // anyway. Skip the patch entirely on aarch64 to avoid the misleading
    // "TWRP version mismatch?" warning that the caller would otherwise
    // log on every arm64 boot.
    #[cfg(target_arch = "aarch64")]
    {
        info!(
            "[KR64] klog_init patch is x86-only; skipped on arm64 (aarch64 TWRP uses a different klog_init implementation)"
        );
        // Mark `init_bytes` as intentionally unused on aarch64 to silence
        // the unused_variables lint without renaming the parameter (which
        // is shared with the non-aarch64 branch below).
        let _ = init_bytes;
        KlogInitPatchResult::Skipped
    }

    // On non-aarch64 hosts (x86, x86_64, etc.), perform the actual
    // i386-instruction-pattern match.
    #[cfg(not(target_arch = "aarch64"))]
    {
        // klog_init's distinctive instruction sequence (i386, TWRP 3.7.0_9-0).
        // We match 32 bytes of fixed instructions ending just before the `jne`,
        // then verify the byte at offset 32 is `0x75` (jne) before patching.
        //
        //   mov DWORD PTR [esp+0x8], 0x10b   ; c7 44 24 08 0b 01 00 00
        //                                     (dev = makedev(1, 11) = 0x10b)
        //   lea esi, [ebx-0x1502e]           ; 8d b3 d2 af fe ff
        //                                     (esi = ptr to "/dev/__kmsg__")
        //   mov DWORD PTR [esp+0x4], 0x2180  ; c7 44 24 04 80 21 00 00
        //                                     (mode = S_IFCHR | 0600 = 0x2180)
        //   mov [esp], esi                   ; 89 34 24
        //   call mknod                       ; e8 ?? ?? ?? ?? (relative call)
        //   test eax, eax                    ; 85 c0
        //   jne <return>                     ; 75 ?? (THIS is what we patch)
        //
        // We use a slice of Option<u8>: Some(b) means "must equal b", None means
        // "wildcard" (for the call's relative offset and the jne's offset).
        const PATTERN: [Option<u8>; 34] = [
            // mov DWORD PTR [esp+0x8], 0x10b
            Some(0xc7),
            Some(0x44),
            Some(0x24),
            Some(0x08),
            Some(0x0b),
            Some(0x01),
            Some(0x00),
            Some(0x00),
            // lea esi, [ebx-0x1502e]
            Some(0x8d),
            Some(0xb3),
            Some(0xd2),
            Some(0xaf),
            Some(0xfe),
            Some(0xff),
            // mov DWORD PTR [esp+0x4], 0x2180
            Some(0xc7),
            Some(0x44),
            Some(0x24),
            Some(0x04),
            Some(0x80),
            Some(0x21),
            Some(0x00),
            Some(0x00),
            // mov [esp], esi
            Some(0x89),
            Some(0x34),
            Some(0x24),
            // call mknod (e8 + 4-byte signed relative offset)
            Some(0xe8),
            None,
            None,
            None,
            None,
            // test eax, eax
            Some(0x85),
            Some(0xc0),
            // jne <offset> or NOP NOP (if already patched) — use wildcards
            // so the pattern matches BOTH unpatched (0x75) and patched (0x90).
            // The actual byte check is done after the pattern matches.
            None,
            None,
        ];
        const JNE_OFFSET: usize = 32; // index of the jne/NOP byte within PATTERN

        if init_bytes.len() < PATTERN.len() {
            return KlogInitPatchResult::NotFound;
        }
        for i in 0..=(init_bytes.len() - PATTERN.len()) {
            let mut matched = true;
            for (j, p) in PATTERN.iter().enumerate() {
                if let Some(b) = p {
                    if init_bytes[i + j] != *b {
                        matched = false;
                        break;
                    }
                }
            }
            if !matched {
                continue;
            }
            // Pattern matched. The byte at offset i + JNE_OFFSET must be 0x75
            // (jne) for an unpatched binary, or 0x90 (nop) for an already-
            // patched binary. Any other value means the pattern matched by
            // coincidence and we should NOT patch (could be a different binary
            // or a different code path).
            let jne = init_bytes[i + JNE_OFFSET];
            let jne_off2 = i + JNE_OFFSET + 1;
            if jne == 0x90 && init_bytes[jne_off2] == 0x90 {
                // Already patched.
                return KlogInitPatchResult::AlreadyApplied;
            }
            if jne == 0x75 {
                // Apply the patch: replace `75 ??` with `90 90`.
                init_bytes[i + JNE_OFFSET] = 0x90;
                init_bytes[jne_off2] = 0x90;
                return KlogInitPatchResult::Applied;
            }
            // Unexpected byte at jne location — pattern matched but the next
            // byte isn't a jne. Don't patch (could brick the binary).
            return KlogInitPatchResult::NotFound;
        }
        KlogInitPatchResult::NotFound
    }
}

/// Result of attempting to apply the klog_init mknod-failure patch.
///
/// See [`patch_twrp_init_klog_init`] for the full root-cause analysis and
/// the per-variant semantics. The variants are ordered roughly by
/// "goodness" — `Applied` and `AlreadyApplied` are successes, `Skipped`
/// is an expected non-action (e.g. aarch64), and `NotFound` is a
/// potential problem (TWRP version mismatch).
///
/// `#[allow(dead_code)]` is needed because the variants are platform-
/// conditional: `Skipped` is only constructed on aarch64, and the
/// other three are only constructed on non-aarch64 hosts. On any given
/// host, the "other" platform's variants would otherwise be flagged as
/// dead code by the compiler.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KlogInitPatchResult {
    /// Patch was applied to the bytes — caller should write them back.
    Applied,
    /// Patch was already applied in a previous boot — caller can skip
    /// the write (no modification needed, idempotent).
    AlreadyApplied,
    /// Patch was intentionally skipped (e.g. on aarch64, where the
    /// i386-only byte pattern is irrelevant). The skip reason has been
    /// logged; the caller should NOT log a "version mismatch" warning
    /// and should NOT write the bytes back.
    Skipped,
    /// Pattern was not found in the binary — caller should log a
    /// warning because this likely indicates a TWRP version mismatch.
    NotFound,
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
    // LD_PRELOAD library, `libtwrp_fb_hook.so`, that intercepts FB ioctls
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
    // reverted to the i686 libtwrp_fb_hook.so — the architecturally correct
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
    // Loaded ONLY in TWRP mode; written to /sbin/libtwrp_fb_hook.so.
    let hook_lib_twrp_fb = if cfg.boot_recovery {
        find_and_read_hook_library(
            &cfg,
            "libtwrp_fb_hook.so",
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
    //
    // NON-ROOT MODE: Skip unshare entirely! On stock AOSP seccomp
    // (Android 11+ emulator), `unshare` is NOT in the seccomp
    // allowlist for untrusted_app — calling it sends SIGSYS (signal
    // 31) which kills the process instantly. On some vendor kernels
    // (e.g. Honor), unshare is allowed but returns EPERM — either
    // way, it doesn't work without root.
    //
    // Instead of unshare, we use PTRACE-based syscall emulation to
    // intercept getpid() and return 1. This achieves the same effect
    // as CLONE_NEWPID without requiring any privileges.
    // ---------------------------------------------------------------
    if cfg.use_namespaces {
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
    } else {
        info!("[KR64] non-root mode: skipping unshare(CLONE_NEWPID) — seccomp blocks it on stock AOSP; ptrace emulation will fake getpid()=1 instead");
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
    // TWRP BOOT: write the i686 libtwrp_fb_hook.so to /sbin/libtwrp_fb_hook.so
    // (tmpfs). The dynamically-linked i386 recovery binary loads it via
    // LD_PRELOAD=/sbin/libtwrp_fb_hook.so (injected via init.rc `setenv`).
    // The hook's open/ioctl intercepts FBIOGET_VSCREENINFO etc. on
    // /dev/graphics/fb0 and returns valid 720x1280@32bpp screen info,
    // fixing the libminuitwrp segfault at offset 0x57d7.
    //
    // NON-ROOT MODE (use_namespaces=false): kr64 can't chroot or pivot_root,
    // so `/sbin` does NOT exist on the host filesystem (it only exists inside
    // the rootfs). Writing to the bare `/sbin/libtwrp_fb_hook.so` fails with
    // ENOENT, the LD_PRELOAD target is never populated, and init dies the
    // moment it exec's recovery and the bionic linker can't find the preload.
    //
    // The init process runs on the HOST filesystem in this mode, but the
    // ptrace emulator (see `ptrace_emu.rs`) translates guest path opens like
    // `/sbin/libtwrp_fb_hook.so` to `{rootfs}/sbin/libtwrp_fb_hook.so` on the
    // host. So we MUST write the library to the host-side translated path,
    // which is `{rootfs_prefix}/sbin/libtwrp_fb_hook.so`:
    //   - use_namespaces=true  -> rootfs_prefix == ""  -> `/sbin/...`
    //     (pivot_root already happened; /sbin IS the rootfs's sbin, tmpfs)
    //   - use_namespaces=false -> rootfs_prefix == cfg.rootfs
    //     -> `{cfg.rootfs}/sbin/...` on the host filesystem
    //
    // The LD_PRELOAD path in init.rc stays as `/sbin/libtwrp_fb_hook.so` —
    // the ptrace emulator performs the translation at runtime. We also
    // `create_dir_all({rootfs_prefix}/sbin)` because in non-root mode the
    // directory may not exist yet on the host (the rootfs image may not ship
    // with an empty sbin, or it may have been stripped).
    if let Some((src, content)) = &hook_lib_twrp_fb {
        let sbin_dir = format!("{}/sbin", rootfs_prefix);
        if let Err(e) = std::fs::create_dir_all(&sbin_dir) {
            error!(
                "[KR64] PARENT: failed to create sbin dir {} for libtwrp_fb_hook.so: {} (errno={})",
                sbin_dir,
                e,
                e.raw_os_error().unwrap_or(0)
            );
        } else {
            let twrp_fb_hook_dst = format!("{}/sbin/libtwrp_fb_hook.so", rootfs_prefix);
            write_hook_library_to_dev("libtwrp_fb_hook.so", src, content, &twrp_fb_hook_dst);
        }
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
    //
    // NON-ROOT MODE: Skip lsetxattr entirely! On unrooted devices,
    // Android's seccomp filter blocks lsetxattr (syscall 189) for
    // untrusted_app, sending SIGSYS (signal 31) which kills the process
    // instantly. This was the root cause of kr64 crashing immediately
    // on the x86_64 emulator — kr64 never even got to write any output
    // to stderr before being killed.
    if !cfg.use_namespaces {
        info!("[KR64] PARENT: non-root mode — skipping lsetxattr (seccomp blocks it, would cause SIGSYS)");
    } else {
        for lib_path in &[
            "/dev/libgetpid_hook.so",
            "/dev/libtwoyi_loader_shlib.so",
            "/sbin/libtwrp_fb_hook.so",
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
    } // end if cfg.use_namespaces

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
    //
    // NON-ROOT MODE: Skip binderfs mount entirely! The `mount()` syscall
    // (syscall 165) is blocked by Android's seccomp filter for
    // untrusted_app — calling it sends SIGSYS (signal 31) which kills
    // the process instantly. This was the root cause of kr64 crashing
    // immediately on the x86_64 emulator.
    //
    // In non-root mode, we already have a binder PROXY (unix socket at
    // {rootfs}/vm0/dev/binder) that handles binder IPC without needing
    // a real binderfs mount. The guest's servicemanager connects to
    // our proxy socket instead of the kernel's /dev/binder.
    if !cfg.use_namespaces {
        info!("[KR64] PARENT: non-root mode — skipping binderfs mount (seccomp blocks mount(), would cause SIGSYS)");
    } else {
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
                    match std::fs::set_permissions(
                        &dev_path,
                        std::fs::Permissions::from_mode(0o666),
                    ) {
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
    } // end if cfg.use_namespaces (binderfs mount)

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
    // `libtwrp_fb_hook.so` (LD_PRELOAD'd into the recovery process). See
    // `devices::create_twrp_framebuffer` for the full rationale.
    if cfg.boot_recovery {
        if let Err(e) =
            devices::create_twrp_framebuffer(&rootfs_prefix, cfg.width as u32, cfg.height as u32)
        {
            warning!(
                "[KR64] PARENT: failed to create TWRP framebuffer: {} (recovery will crash in libminuitwrp.so)",
                e
            );
        }
    }

    // TWRP BOOT: create /dev/kmsg as a symlink to /twrp-kmsg.log so TWRP's
    // init can write kernel log messages to a file we can retrieve.
    //
    // ROOT CAUSE (Task ID 18, KVM run 31548039158): TWRP's init (AOSP 5.1.1)
    // writes ALL its log messages via KLOG_INFO/KLOG_ERROR, which open
    // /dev/kmsg and write to it. Without /dev/kmsg, init's messages are
    // silently dropped — we can't see "starting service 'recovery'" or any
    // error messages. The /twrp-init.log redirect only captures stdout/stderr,
    // which init doesn't use.
    //
    // The host's dmesg ring buffer is flooded by the OUTER Android init's
    // "Could not set execcon for 'u:r:vendor_init:s0'" loop (TWRP's SELinux
    // policy replaces the outer policy, breaking the outer init's contexts).
    // So even if TWRP init wrote to a real /dev/kmsg char device, the
    // messages would be pushed out of the ring buffer within ~12s.
    //
    // FIX: create /dev/kmsg as a SYMLINK to /twrp-kmsg.log (a regular file
    // on the ext4 rootfs). TWRP init opens /dev/kmsg → resolves to
    // /twrp-kmsg.log → writes go to the ext4 file. The file survives
    // kr64's death (it's on ext4, not tmpfs), so the KVM test can
    // `adb pull` it and we can finally see what TWRP init is doing.
    //
    // This is a DIAGNOSTIC aid — it doesn't fix the recovery-not-starting
    // issue, but it lets us see the next error message from TWRP init.
    if cfg.boot_recovery {
        let kmsg_log_path = format!("{}/twrp-kmsg.log", rootfs_prefix);
        // Create the target file (empty) on the ext4 rootfs.
        match std::fs::write(&kmsg_log_path, b"") {
            Ok(()) => {
                info!(
                    "[KR64] PARENT: created /twrp-kmsg.log (empty) on ext4 rootfs for TWRP init KLOG capture"
                );
            }
            Err(e) => {
                warning!(
                    "[KR64] PARENT: failed to create /twrp-kmsg.log: {} (TWRP init KLOG messages will be lost)",
                    e
                );
            }
        }
        // Create /dev/kmsg AND /dev/__kmsg__ as symlinks to /twrp-kmsg.log
        // (chroot-relative after pivot_root). Use std::os::unix::fs::symlink.
        //
        // ROOT CAUSE (Task ID 21, KVM run 31552072308): TWRP's init (AOSP
        // 5.1-based) does NOT use /dev/kmsg for KLOG output. Its log_init()
        // creates /dev/__kmsg__ as a char device (major 1, minor 11), opens
        // it for writing, then unlinks it. The open fd is kept and all
        // KLOG_INFO/KLOG_ERROR writes go through that fd to the kernel's
        // kmsg ring buffer — which is flooded by the outer Android init's
        // "Could not set execcon for 'u:r:vendor_init:s0'" loop, pushing
        // TWRP init's messages out within ~12s.
        //
        // Evidence from KVM run 31552072308:
        //   - twrp-init-fds.log shows: fd 3 -> /dev/__kmsg__ (deleted)
        //   - twrp-kmsg.log is EMPTY (0 bytes) despite the /dev/kmsg symlink
        //   - dmesg shows NO TWRP init messages (only host's init flooding)
        //
        // FIX: create /dev/__kmsg__ as a SYMLINK to /twrp-kmsg.log. Then:
        //   1. TWRP init's mknod("/dev/__kmsg__", S_IFCHR, ...) follows the
        //      symlink and tries to mknod /twrp-kmsg.log. /twrp-kmsg.log
        //      exists as a regular file -> mknod returns EEXIST. Init's
        //      log_init treats EEXIST as success and continues.
        //   2. TWRP init's open("/dev/__kmsg__", O_WRONLY) follows the
        //      symlink and opens /twrp-kmsg.log (the regular file). SUCCESS.
        //   3. TWRP init's unlink("/dev/__kmsg__") removes the symlink
        //      (NOT the target /twrp-kmsg.log). The target file remains.
        //   4. TWRP init's write(log_fd, ...) writes to /twrp-kmsg.log via
        //      the still-open fd. The file accumulates KLOG output.
        //   5. After kr64 SIGKILLs init, the open fd is closed, but
        //      /twrp-kmsg.log is on ext4 (not tmpfs) and survives — we can
        //      `adb pull` it and finally see TWRP init's KLOG messages.
        //
        // We ALSO keep the /dev/kmsg symlink (harmless; some TWRP init
        // variants may try /dev/kmsg as a fallback). The /dev/kmsg symlink
        // is only created in root mode (use_namespaces=true) where we have
        // pivot_root'd into the rootfs and /dev is the writable tmpfs we
        // own. In non-root mode the host's /dev is owned by root, so the
        // symlink attempt fails with EACCES — that's expected and only
        // affects the fallback path; the PRIMARY kmsg path (/dev/__kmsg__)
        // is handled separately below.
        use std::os::unix::fs::symlink;
        let kmsg_target = "/twrp-kmsg.log";
        // /dev/kmsg symlink (kept for compatibility / fallback). Root mode
        // only — in non-root mode the host /dev is read-only.
        if cfg.use_namespaces {
            let kmsg_link = "/dev/kmsg";
            let _ = std::fs::remove_file(kmsg_link);
            match symlink(kmsg_target, kmsg_link) {
                Ok(()) => {
                    info!(
                        "[KR64] PARENT: /dev/kmsg -> {} symlink created (TWRP init KLOG will be captured to /twrp-kmsg.log)",
                        kmsg_target
                    );
                }
                Err(e) => {
                    warning!(
                        "[KR64] PARENT: failed to create /dev/kmsg symlink: {} (TWRP init KLOG messages will be lost)",
                        e
                    );
                }
            }
        } else {
            info!(
                "[KR64] PARENT: non-root mode — skipping /dev/kmsg symlink (host /dev is read-only; /dev/__kmsg__ regular file is created below as the PRIMARY kmsg path)"
            );
        }
        // /dev/__kmsg__ — THIS is the path TWRP init's log_init() actually
        // opens (confirmed via twrp-init-fds.log in KVM run 31552072308).
        //
        // The path is constructed with `format!("{}/dev/__kmsg__", rootfs_prefix)`
        // which gives "/dev/__kmsg__" in root mode (rootfs_prefix == "",
        // chroot-relative after pivot_root) and "{cfg.rootfs}/dev/__kmsg__"
        // in non-root mode (host path under the app's private data dir,
        // which is writable).
        //
        // ROOT MODE (use_namespaces=true): create as a SYMLINK to
        // /twrp-kmsg.log (chroot-relative). Then:
        //   1. TWRP init's mknod("/dev/__kmsg__", S_IFCHR, ...) follows the
        //      symlink and tries to mknod /twrp-kmsg.log. /twrp-kmsg.log
        //      exists as a regular file -> mknod returns EEXIST. Init's
        //      log_init treats EEXIST as success and continues.
        //   2. TWRP init's open("/dev/__kmsg__", O_WRONLY) follows the
        //      symlink and opens /twrp-kmsg.log (the regular file). SUCCESS.
        //   3. TWRP init's unlink("/dev/__kmsg__") removes the symlink
        //      (NOT the target /twrp-kmsg.log). The target file remains.
        //   4. TWRP init's write(log_fd, ...) writes to /twrp-kmsg.log via
        //      the still-open fd. The file accumulates KLOG output.
        //   5. After kr64 SIGKILLs init, the open fd is closed, but
        //      /twrp-kmsg.log is on ext4 (not tmpfs) and survives — we can
        //      `adb pull` it and finally see TWRP init's KLOG messages.
        //
        // NON-ROOT MODE (use_namespaces=false): kr64 is running as an
        // untrusted_app and the host's /dev is owned by root — creating a
        // symlink at literal /dev/__kmsg__ fails with EACCES. The loader's
        // syscall emulation translates guest open("/dev/__kmsg__") to host
        // open("{rootfs_prefix}/dev/__kmsg__"), so we create the file UNDER
        // the rootfs prefix instead. We can't use a symlink here (the
        // absolute target /twrp-kmsg.log would resolve on the HOST
        // filesystem, not the guest rootfs), so we create /dev/__kmsg__ as
        // a REGULAR FILE (empty, mode 0666). TWRP init's mknod is
        // translated by the loader -> mknod returns EEXIST (treated as
        // success). The open() opens the regular file -> SUCCESS. KLOG
        // writes go to the file via the open fd. (TWRP init's later
        // unlink() removes the file from disk, but the open fd keeps the
        // inode alive until kr64 SIGKILLs init — `adb pull` may or may not
        // find the file depending on timing; that's why ui-navigate.py
        // pulls it with `|| true`.)
        let kmsg_link_double = format!("{}/dev/__kmsg__", rootfs_prefix);
        let _ = std::fs::remove_file(&kmsg_link_double);
        if cfg.use_namespaces {
            // ROOT MODE: symlink to /twrp-kmsg.log (chroot-relative).
            match symlink(kmsg_target, &kmsg_link_double) {
                Ok(()) => {
                    info!(
                        "[KR64] PARENT: /dev/__kmsg__ -> {} symlink created (TWRP init log_init() will write KLOG to /twrp-kmsg.log)",
                        kmsg_target
                    );
                }
                Err(e) => {
                    warning!(
                        "[KR64] PARENT: failed to create /dev/__kmsg__ symlink: {} (TWRP init KLOG messages will be lost — this is the PRIMARY kmsg path)",
                        e
                    );
                }
            }
        } else {
            // NON-ROOT MODE: create as a regular file (empty, mode 0666).
            // Symlinks can't be used because (a) the host /dev is read-only
            // for the untrusted_app, and (b) a symlink target /twrp-kmsg.log
            // would resolve on the HOST filesystem, not the guest rootfs.
            use std::os::unix::fs::PermissionsExt;
            match std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&kmsg_link_double)
            {
                Ok(_) => {
                    let _ = std::fs::set_permissions(
                        &kmsg_link_double,
                        std::fs::Permissions::from_mode(0o666),
                    );
                    info!(
                        "[KR64] PARENT: /dev/__kmsg__ regular file created at {} (mode 0666, non-root mode — TWRP init log_init() will write KLOG here)",
                        kmsg_link_double
                    );
                }
                Err(e) => {
                    warning!(
                        "[KR64] PARENT: failed to create /dev/__kmsg__ regular file at {}: {} (TWRP init KLOG messages will be lost — this is the PRIMARY kmsg path)",
                        kmsg_link_double,
                        e
                    );
                }
            }
        }
    }

    // TWRP BOOT: patch TWRP init to add `setenv LD_PRELOAD
    // /sbin/libtwrp_fb_hook.so` to the recovery service definition.
    //
    // ROOT CAUSE (KVM run 31533796663): kr64 sets LD_PRELOAD in init's
    // environment, but TWRP's init (based on AOSP's init) builds a FRESH
    // environment for each service from the service's `setenv` directives
    // plus a few inherited vars (ANDROID_ROOT, ANDROID_DATA, etc.).
    // LD_PRELOAD is NOT in the inherited list, so recovery's bionic linker
    // never sees it → our hook is never loaded → recovery crashes at
    // offset 0x57d7 in libminuitwrp.so (same as without the hook).
    //
    // FIX: patch init.rc (or, on arm64 TWRP, the imported .rc file that
    // actually defines the recovery service) to add
    // `setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so` to the recovery service.
    // TWRP's init supports `setenv` in service blocks (confirmed via
    // `strings /tmp/twrp/rd/init | grep setenv`). This adds LD_PRELOAD to
    // recovery's environment, so the bionic linker loads our i686 hook →
    // FB ioctls are intercepted → no crash.
    //
    // CRITICAL (Task ID 18, KVM run 31536016997): the LD_PRELOAD path
    // MUST be `/sbin/libtwrp_fb_hook.so` (i686), NOT `/dev/libtwoyi_loader_shlib.so`
    // (x86_64). TWRP's recovery binary is i386 and its 32-bit bionic
    // linker cannot load 64-bit libraries. Task ID 17 incorrectly used
    // the x86_64 path; the linker aborted recovery on the architecture
    // mismatch, so recovery was invisible in `ps`.
    //
    // ARM64 REGRESSION: on arm64 TWRP images, the recovery service is
    // defined in an IMPORTED .rc file (e.g. `init.recovery.rc` or
    // `init.recovery.<ro.hardware>.rc`), NOT directly in `init.rc`. The
    // old implementation only scanned `init.rc`, so on arm64 it never
    // found the service and silently skipped the patch, causing the
    // libminuitwrp.so crash to come back. The orchestrator function
    // `patch_twrp_init_rc_recovery_service_in_rootfs` now scans init.rc,
    // all its imports (recursively), init.recovery.rc, init.recovery.*.rc,
    // and system/etc/init/recovery.rc, and falls back to creating a new
    // init.twoyi.rc if none of them define the recovery service.
    //
    // The patch is IDEMPOTENT: if the setenv line is already present
    // (e.g., from a previous boot), we skip the write.
    if cfg.boot_recovery {
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs_prefix);
    }

    // TWRP BOOT: patch {rootfs}/init binary to skip the mknod-failure
    // check in klog_init(). See `patch_twrp_init_klog_init` for the full
    // root-cause analysis.
    //
    // Without this patch, kr64's /dev/__kmsg__ -> /twrp-kmsg.log symlink
    // is useless: TWRP init's klog_init() calls mknod("/dev/__kmsg__", ...)
    // FIRST, which fails with EEXIST (because the symlink exists), and
    // klog_init() returns early WITHOUT calling open(). klog_fd stays at
    // -1, and ALL KLOG writes are silently dropped. /twrp-kmsg.log stays
    // empty, and we cannot see TWRP init's "starting service 'recovery'"
    // message or any error messages — making the recovery-not-starting
    // issue impossible to diagnose.
    //
    // The patch is a 2-byte NOP-out of the `jne <return>` after the
    // mknod-failure check. This makes klog_init continue to open() even
    // if mknod fails. open() follows our symlink and opens /twrp-kmsg.log
    // (a regular file on ext4). KLOG writes are then captured.
    //
    // The patch is IDEMPOTENT (skipped if already applied) and is safe
    // to apply on every boot — if the pattern isn't found (e.g. a
    // different TWRP version), we log a warning and continue.
    if cfg.boot_recovery {
        let init_path = format!("{}/init", rootfs_prefix);
        match std::fs::read(&init_path) {
            Ok(mut bytes) => {
                match patch_twrp_init_klog_init(&mut bytes) {
                    KlogInitPatchResult::Applied => {
                        match std::fs::write(&init_path, &bytes) {
                            Ok(()) => info!(
                                "[KR64] PARENT: patched /init klog_init() — NOP'd jne after mknod failure (KLOG will be captured to /twrp-kmsg.log)"
                            ),
                            Err(e) => warning!(
                                "[KR64] PARENT: patched /init in memory but failed to write back: {} (KLOG capture may not work)",
                                e
                            ),
                        }
                    }
                    KlogInitPatchResult::AlreadyApplied => {
                        info!(
                            "[KR64] PARENT: /init klog_init() already patched (idempotent skip) — KLOG capture via /dev/__kmsg__ symlink is active"
                        );
                    }
                    KlogInitPatchResult::Skipped => {
                        // Skip reason already logged inside
                        // `patch_twrp_init_klog_init` (currently fires
                        // only on aarch64, where the i386-only byte
                        // pattern is irrelevant). Do NOT log the
                        // "TWRP version mismatch?" warning here — it
                        // would be misleading on arm64, where the
                        // skip is expected and harmless.
                    }
                    KlogInitPatchResult::NotFound => {
                        warning!(
                            "[KR64] PARENT: could not find klog_init mknod-failure pattern in /init (TWRP version mismatch?) — KLOG capture via /dev/__kmsg__ symlink will NOT work"
                        );
                    }
                }
            }
            Err(e) => warning!(
                "[KR64] PARENT: failed to read /init for klog_init patching: {} (KLOG capture may not work)",
                e
            ),
        }

        // ── Patch find_property to return NULL immediately ──
        //
        // ROOT CAUSE of SIGSEGV after reading /proc/cmdline:
        // Init calls find_property() to look up properties. The property
        // area is not initialized (because /dev/__properties__ is not
        // accessible to untrusted_app). The first argument to
        // find_property is a pointer derived from the uninitialized
        // property area — it's 0x80 (a small address in the NULL page).
        // find_property dereferences it at offset 0x10, accessing
        // address 0x90, which is unmapped → SIGSEGV.
        //
        // FIX: Patch find_property's first 3 bytes to `xor eax,eax; ret`
        // (31 c0 c3). This makes ALL property lookups return NULL
        // immediately, preventing the crash. Init handles NULL property
        // returns gracefully (properties are optional during early boot).
        //
        // The pattern to match (first 12 bytes of find_property at
        // virtual address 0x08092500, file offset 0x4a500):
        //   55 89 e5 57 56 89 c6 53 8d 64 24 a4
        //   push ebp; mov esp,ebp; push edi; push esi; mov eax,esi;
        //   push ebx; lea -0x5c(esp),esp
        //
        // Replacement (first 3 bytes only):
        //   31 c0 c3  xor eax,eax; ret
        {
            let init_path = format!("{}/init", rootfs_prefix);
            match std::fs::read(&init_path) {
                Ok(mut bytes) => {
                    let pattern: &[u8] = &[
                        0x55, 0x89, 0xe5, 0x57, 0x56, 0x89, 0xc6, 0x53, 0x8d, 0x64, 0x24, 0xa4,
                        0x89, 0x55, 0xc4, 0x8b, 0x55, 0x0c,
                    ];
                    let patch: &[u8] = &[0x31, 0xc0, 0xc3]; // xor eax,eax; ret

                    // Check if already patched
                    let already_patched = bytes.len() >= 3 && bytes[0] == 0x31 && bytes[1] == 0xc0
                        && bytes[2] == 0xc3;

                    if !already_patched {
                        let mut found = false;
                        for i in 0..=(bytes.len().saturating_sub(pattern.len())) {
                            if bytes[i..i + pattern.len()] == *pattern {
                                // Apply patch: replace first 3 bytes with xor eax,eax; ret
                                bytes[i] = patch[0];
                                bytes[i + 1] = patch[1];
                                bytes[i + 2] = patch[2];
                                found = true;
                                break;
                            }
                        }
                        if found {
                            // Verify the patch was applied by reading back
                            let patch_offset = (0..bytes.len()).find(|&i| bytes[i] == 0x31 && i + 2 < bytes.len() && bytes[i+1] == 0xc0 && bytes[i+2] == 0xc3 && i + 12 <= bytes.len() && bytes[i+3] == 0x57 && bytes[i+4] == 0x56).map(|i| i).unwrap_or(0);
                            match std::fs::write(&init_path, &bytes) {
                                Ok(()) => info!(
                                    "[KR64] PARENT: patched /init find_property() at offset {:#x} — replaced first 3 bytes with 'xor eax,eax; ret' (prevents SIGSEGV when property area is not initialized)",
                                    patch_offset
                                ),
                                Err(e) => warning!(
                                    "[KR64] PARENT: patched find_property in memory but failed to write back: {} (init may crash with SIGSEGV)",
                                    e
                                ),
                            }
                        } else {
                            warning!(
                                "[KR64] PARENT: could not find find_property pattern in /init (TWRP version mismatch?) — init may crash with SIGSEGV when accessing properties"
                            );
                        }
                    } else {
                        info!(
                            "[KR64] PARENT: /init find_property() already patched (idempotent skip)"
                        );
                    }
                }
                Err(e) => warning!(
                    "[KR64] PARENT: failed to read /init for find_property patching: {} (init may crash with SIGSEGV)",
                    e
                ),
            }
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
    // SELinux permissive watchdog — DISABLED per user requirement.
    // The user wants this to work on UNMODIFIED devices with enforcing
    // SELinux. The permissive watchdog was a KVM test hack that won't
    // work on real devices (writing to /sys/fs/selinux/enforce requires
    // CAP_MAC_ADMIN which untrusted apps don't have).
    //
    // Instead, we must make TWRP work WITHIN the host's SELinux policy.
    // This means:
    //   - TWRP init must not load its own /sepolicy (would break host)
    //   - TWRP services must run in contexts the host policy allows
    //   - The recovery service's seclabel must be valid under the host policy
    //
    // For the KVM test, the host runs in permissive mode by default
    // (Android emulator), so SELinux denials are logged but not enforced.
    // For real devices, the app must request the right SELinux permissions
    // or use a different approach (e.g., magisk, custom policy).
    let enforce_thread: Option<std::thread::JoinHandle<()>> = None;
    info!("[KR64] PARENT: SELinux permissive watchdog DISABLED (no permissive mode — works on unmodified devices)");

    // ── Pre-create /twrp-init.log in the rootfs ──────────────────────
    //
    // The child branch below redirects init's stdout/stderr to a log
    // file via `open(... O_CREAT|O_TRUNC, 0o644)` + dup2. In root mode
    // (use_namespaces=true) the parent has already pivot_root'd into
    // the rootfs, so `/twrp-init.log` resolves there and `open` works.
    //
    // In NON-root mode (use_namespaces=false) the child is NOT chrooted
    // — it inherits the host's filesystem namespace. The literal
    // `/twrp-init.log` would resolve to the HOST's root, which an
    // untrusted_app has no permission to write to (EROFS / EACCES), so
    // the open fails and we log:
    //   "[KR64 CHILD] TWRP: WARN could not open /twrp-init.log for redirect"
    // and init's stdout/stderr stay attached to kr64's stderr (the
    // inherited fd 1/2). That makes debugging init's actual output
    // impossible because it's interleaved with kr64's logs.
    //
    // FIX: Pre-create the file in the parent (which CAN use std::fs
    // safely) at the host-visible path
    // `{rootfs_prefix}/twrp-init.log`. In root mode rootfs_prefix is
    // "" so this is `/twrp-init.log` (chroot-relative). In non-root
    // mode rootfs_prefix is cfg.rootfs so this is the host path under
    // the app's private data dir, which is writable.
    //
    // We also build a CString of the same path here so the child can
    // reuse it without allocating (which is async-signal-unsafe)
    // between fork() and execve().
    use std::os::unix::fs::PermissionsExt;
    let twrp_log_path_str: String = format!("{}/twrp-init.log", rootfs_prefix);
    let twrp_log_path_cstr: CString =
        CString::new(twrp_log_path_str.as_str()).unwrap_or_else(|_| {
            // Path contained an interior NUL — extremely unlikely for
            // an app-private data dir, but fall back to the literal
            // so we don't panic in the parent. The child's open() will
            // then fail and the existing WARN branch will fire.
            CString::new("/twrp-init.log").unwrap()
        });
    match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&twrp_log_path_str)
    {
        Ok(_) => {
            // World-readable so init (which may run as a different
            // UID under TWRP's recovery policy) can still append.
            let _ = std::fs::set_permissions(
                &twrp_log_path_str,
                std::fs::Permissions::from_mode(0o666),
            );
            info!(
                "[KR64] PARENT: pre-created {} (mode 0666, truncated)",
                twrp_log_path_str
            );
        }
        Err(e) => {
            // Don't make this fatal — the child's open() will still
            // try and produce its own diagnostic. We just lose the
            // pre-creation guarantee.
            warning!(
                "[KR64] PARENT: failed to pre-create {}: {} — child redirect may fail",
                twrp_log_path_str,
                e
            );
        }
    }

    info!("[KR64] forking guest process");

    // ── Pre-create essential /dev files in rootfs ──────────────────────
    //
    // TWRP init expects certain files in /dev to exist (or be creatable):
    //   /dev/null, /dev/zero, /dev/urandom, /dev/random — real kernel
    //     devices. We create symlinks to the host's /dev/* so opens
    //     reach the real devices.
    //   /dev/console, /dev/ptmx — real kernel devices (symlinks).
    //   /dev/.booting — marker file init creates with O_CREAT|O_EXCL.
    //     Pre-create it so O_CREAT without O_EXCL succeeds. If init
    //     uses O_EXCL, it gets EEXIST (which init handles).
    //   /dev/__null__ — temp file init creates via mknod, then opens,
    //     then unlinks. Pre-create as a regular file so open succeeds
    //     even when mknod is seccomp-blocked.
    //
    // This is needed because translate_path now redirects /dev/* to
    // {rootfs}/dev/* (the host /dev is read-only for untrusted_app).
    {
        let dev_dir = format!("{}/dev", rootfs_prefix);
        let _ = std::fs::create_dir_all(&dev_dir);
        let _ = std::fs::create_dir_all(format!("{}/dev/pts", rootfs_prefix));
        let _ = std::fs::create_dir_all(format!("{}/dev/socket", rootfs_prefix));

        // Symlinks to host kernel devices (target is absolute, resolves
        // on the host filesystem because we're NOT chrooted in non-root
        // mode).
        let symlinks: &[(&str, &str)] = &[
            ("dev/null", "/dev/null"),
            ("dev/zero", "/dev/zero"),
            ("dev/urandom", "/dev/urandom"),
            ("dev/random", "/dev/random"),
            ("dev/console", "/dev/console"),
            ("dev/ptmx", "/dev/ptmx"),
            ("dev/tty", "/dev/tty"),
            ("dev/kmsg", "/dev/kmsg"),
        ];
        for (rel, target) in symlinks {
            let link_path = format!("{}/{}", rootfs_prefix, rel);
            // Remove existing file/symlink first (ignore errors — might
            // not exist, or might be a directory we shouldn't touch).
            let _ = std::fs::remove_file(&link_path);
            match symlink(*target, &link_path) {
                Ok(()) => {}
                Err(e) => {
                    // Not fatal — init may handle missing /dev/null etc.
                    warning!(
                        "[KR64] PARENT: failed to create {} -> {} symlink: {}",
                        link_path,
                        target,
                        e
                    );
                }
            }
        }

        // Regular files init expects to create/open.
        // Use OpenOptions (same pattern as /dev/__kmsg__ creation above)
        // and log success/failure for each file.
        use std::os::unix::fs::OpenOptionsExt;
        let regular_files: &[(&str, u32)] = &[("dev/.booting", 0o666), ("dev/__null__", 0o666)];
        for (rel, mode) in regular_files {
            let file_path = format!("{}/{}", rootfs_prefix, rel);
            // Remove existing file first (in case it was unlinked by a
            // previous init run's unlink() call).
            let _ = std::fs::remove_file(&file_path);
            match std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .mode(*mode)
                .open(&file_path)
            {
                Ok(_) => {
                    let _ = std::fs::set_permissions(
                        &file_path,
                        std::fs::Permissions::from_mode(*mode),
                    );
                    // Verify the file exists and is accessible
                    match std::fs::metadata(&file_path) {
                        Ok(m) => info!(
                            "[KR64] PARENT: pre-created {} (mode={:o}, size={})",
                            file_path,
                            std::os::unix::fs::PermissionsExt::mode(&m.permissions()),
                            m.len()
                        ),
                        Err(e) => warning!(
                            "[KR64] PARENT: pre-created {} but metadata check failed: {}",
                            file_path,
                            e
                        ),
                    }
                }
                Err(e) => warning!(
                    "[KR64] PARENT: FAILED to pre-create {} (mode={:o}): {} (errno={})",
                    file_path,
                    mode,
                    e,
                    e.raw_os_error().unwrap_or(0)
                ),
            }
        }

        info!(
            "[KR64] PARENT: pre-created essential /dev files in {} (null, zero, urandom, console, ptmx, .booting, __null__)",
            dev_dir
        );

        // Pre-create /proc/cmdline — init reads this to get kernel boot
        // parameters. The host's /proc/cmdline is not readable by
        // untrusted_app (EACCES from SELinux). We create a fake one in
        // rootfs with the essential Android boot parameters.
        let cmdline_path = format!("{}/twrp-cmdline", rootfs_prefix);
        // Minimal Android boot parameters that TWRP init expects.
        // androidboot.hardware is particularly important — init uses it
        // to find the right init.{hardware}.rc file.
        let cmdline_content = "androidboot.hardware=ranchu androidboot.hardware.gralloc=ranchu androidboot.hardware.vulkan=ranchu androidboot.serialno=twoyi androidboot.boot_devices=pci0000:00/0000:00:03.0 androidboot.verifiedbootstate=orange androidboot.flash.locked=0 androidboot.slot_suffix= androidboot.vbmeta.size=0 qemu=1 qemu.avd_name=twoyi_test\n";
        match std::fs::write(&cmdline_path, cmdline_content) {
            Ok(_) => {
                let _ =
                    std::fs::set_permissions(&cmdline_path, std::fs::Permissions::from_mode(0o444));
                info!(
                    "[KR64] PARENT: pre-created {} ({} bytes)",
                    cmdline_path,
                    cmdline_content.len()
                );
            }
            Err(e) => warning!(
                "[KR64] PARENT: FAILED to pre-create {}: {} (errno={})",
                cmdline_path,
                e,
                e.raw_os_error().unwrap_or(0)
            ),
        }
    }

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
            // ── PTRACE-BASED SYSCALL EMULATION (non-root TWRP boot) ──
            //
            // On unrooted devices, we can't chroot or unshare(CLONE_NEWPID).
            // TWRP's init is statically linked, so LD_PRELOAD doesn't work.
            //
            // The ONLY way to make init think it's PID 1 in a chrooted
            // filesystem is to use ptrace to intercept its syscalls:
            //   - getpid() → return 1
            //   - open("/init.rc") → translate to "{rootfs}/init.rc"
            //   - stat("/sbin/recovery") → translate to "{rootfs}/sbin/recovery"
            //
            // This is the same technique PROOT uses for rootless containers.
            // ptrace IS allowed by Android's seccomp policy for untrusted
            // apps on their own children.
            //
            // The child calls PTRACE_TRACEME + raises SIGSTOP so the parent
            // can attach before we exec init. The parent then runs the
            // ptrace interception loop (see ptrace_emu::run_ptrace_loop).
            unsafe {
                safe_write_err(b"[KR64 CHILD] enabling PTRACE_TRACEME for syscall emulation\n");
                let r = libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0);
                if r == -1 {
                    let e = std::io::Error::last_os_error();
                    safe_write_err(b"[KR64 CHILD] PTRACE_TRACEME failed: ");
                    safe_write_err_errno(b"", e.raw_os_error().unwrap_or(0));
                    safe_write_err(b"\n");
                    // Continue anyway — init will exit 31 but we'll get
                    // diagnostic output.
                } else {
                    safe_write_err(
                        b"[KR64 CHILD] PTRACE_TRACEME OK - raising SIGSTOP for parent\n",
                    );
                    libc::kill(libc::getpid(), libc::SIGSTOP);
                }
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

        // NON-ROOT MODE: The rootfs is on the app's data partition which
        // has noexec. execve() of {rootfs}/init fails with EACCES.
        // Copy the init binary to the app's cache dir (which IS executable)
        // and exec from there. The ptrace emulator translates all path
        // syscalls so init still "sees" the rootfs as "/".
        let exec_path = if !cfg.use_namespaces {
            // Copy init to cache dir.
            let cache_init = format!("{}/cache/twoyi_init", cfg.data_dir);
            match std::fs::read(&full_init_path) {
                Ok(init_bytes) => {
                    // Ensure cache dir exists.
                    let _ = std::fs::create_dir_all(format!("{}/cache", cfg.data_dir));
                    match std::fs::write(&cache_init, &init_bytes) {
                        Ok(_) => {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = std::fs::set_permissions(
                                &cache_init,
                                std::fs::Permissions::from_mode(0o755),
                            );
                            unsafe {
                                safe_write_err(b"[KR64 CHILD] copied init to ");
                                safe_write_err(cache_init.as_bytes());
                                safe_write_err(b" (");
                                safe_write_err(format!("{}", init_bytes.len()).as_bytes());
                                safe_write_err(b" bytes) for exec\n");
                            }
                            cache_init
                        }
                        Err(e) => unsafe {
                            safe_write_err(b"[KR64 CHILD] FATAL: failed to copy init to cache: ");
                            safe_write_err(e.to_string().as_bytes());
                            safe_write_err(b"\n");
                            libc::_exit(127);
                        },
                    }
                }
                Err(e) => unsafe {
                    safe_write_err(b"[KR64 CHILD] FATAL: failed to read init binary at ");
                    safe_write_err(full_init_path.as_bytes());
                    safe_write_err(b": ");
                    safe_write_err(e.to_string().as_bytes());
                    safe_write_err(b"\n");
                    libc::_exit(127);
                },
            }
        } else {
            full_init_path.clone()
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

        let init_cstr = match CString::new(exec_path.as_str()) {
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
        // (32-bit x86) hook library: `/sbin/libtwrp_fb_hook.so`. The
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
        // LD_PRELOAD for TWRP mode: load ONLY libtwrp_fb_hook.so (the i686
        // FB ioctl hook). The x86_64 libgetpid_hook.so and
        // libtwoyi_loader_shlib.so are NOT loaded because:
        //   - init is statically linked (ignores LD_PRELOAD)
        //   - recovery is i386 and its 32-bit linker can't load x86_64 libs
        let ld_preload_str = if cfg.boot_recovery {
            "LD_PRELOAD=/sbin/libtwrp_fb_hook.so".to_string()
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
        //
        // PATH RESOLUTION:
        //   - root mode (use_namespaces=true): the parent has pivot_root'd,
        //     so `/twrp-init.log` resolves inside the rootfs. The path
        //     string is `/twrp-init.log` (rootfs_prefix == "").
        //   - non-root mode (use_namespaces=false): the child is NOT
        //     chrooted, so `/twrp-init.log` would resolve to the HOST's
        //     root (not writable by an untrusted_app). We use the
        //     pre-built `twrp_log_path_cstr` instead, which is
        //     `{cfg.rootfs}/twrp-init.log` on the host filesystem.
        //
        // The parent already pre-created the file (see the "Pre-create
        // /twrp-init.log" block above), so the open() below should
        // succeed via O_CREAT even if the file was deleted between the
        // parent's pre-create and our open.
        if cfg.boot_recovery {
            // `twrp_log_path_cstr` is a CString built in the parent
            // BEFORE the fork — using its .as_ptr() here is async-
            // signal-safe (no allocation, no locks). The bytes are
            // NUL-terminated and live for the duration of the child
            // branch (the parent's stack frame is preserved across
            // fork()).
            let log_path_ptr = twrp_log_path_cstr.as_ptr();
            let fd = unsafe {
                libc::open(
                    log_path_ptr,
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
                // Capture errno BEFORE any other call (the safe_write_err
                // path goes through libc::write which can clobber it).
                let open_errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                unsafe {
                    safe_write_err(
                        b"[KR64 CHILD] TWRP: WARN could not open /twrp-init.log for redirect\n",
                    );
                    safe_write_err_errno(b"[KR64 CHILD] TWRP: open errno=", open_errno);
                    safe_write_err(b"[KR64 CHILD] TWRP: path=");
                    safe_write_err(twrp_log_path_cstr.to_bytes());
                    safe_write_err(b"\n");
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

    // ── PTRACE SYSCALL EMULATION (non-root TWRP boot) ──
    //
    // In non-root mode, the child called PTRACE_TRACEME + raise(SIGSTOP)
    // before exec'ing init. We need to:
    //   1. waitpid for the SIGSTOP
    //   2. Run the ptrace syscall interception loop (translates paths,
    //      fakes getpid → 1)
    //   3. The loop returns when the child exits
    //
    // In root mode (use_namespaces=true), the child did NOT call
    // PTRACE_TRACEME, so we skip the ptrace loop and use plain waitpid.
    if !cfg.use_namespaces {
        // Wait for the child to stop itself (SIGSTOP from PTRACE_TRACEME).
        let mut status: libc::c_int = 0;
        let waited = unsafe { libc::waitpid(pid, &mut status, 0) };
        if waited == -1 {
            let e = std::io::Error::last_os_error();
            error!("[KR64][parent] waitpid for SIGSTOP failed: {}", e);
        } else if libc::WIFSTOPPED(status) {
            let sig = libc::WSTOPSIG(status);
            info!(
                "[KR64][parent] child stopped with signal {} — starting ptrace emulation loop",
                sig
            );
            // Run the ptrace loop — this blocks until the child exits.
            let exit_code = ptrace_emu::run_ptrace_loop(pid, &cfg.rootfs);
            info!(
                "[KR64][parent] ptrace emulation loop ended — child exit code: {}",
                exit_code
            );
            // -------------------------------------------------------------
            // Copy diagnostic logs to external storage.
            //
            // The guest writes twrp-init.log, twrp-kmsg.log and
            // dev/__kmsg__ into the rootfs, which lives under the app's
            // private data dir (/data/user/0/io.twoyi/rootfs/...). On
            // release (non-debuggable) builds `adb shell run-as` is
            // rejected and `adb pull` cannot read from the app's private
            // dir, so the logs are effectively inaccessible.
            //
            // The external app-specific files dir
            // (/sdcard/Android/data/io.twoyi/files/) IS readable via
            // `adb pull` without root on release builds, so we mirror
            // the logs there once the child has exited.
            // -------------------------------------------------------------
            {
                let ext_files_dir = "/sdcard/Android/data/io.twoyi/files";
                let _ = std::fs::create_dir_all(ext_files_dir);
                // (rootfs-relative source, external-files-dst filename)
                let copies: &[(&str, &str)] = &[
                    ("twrp-init.log", "twrp-init.log"),
                    ("twrp-kmsg.log", "twrp-kmsg.log"),
                    ("dev/__kmsg__", "dev-__kmsg__"),
                ];
                for (src_rel, dst_name) in copies {
                    let src = format!("{}/{}", cfg.rootfs, src_rel);
                    let dst = format!("{}/{}", ext_files_dir, dst_name);
                    match std::fs::copy(&src, &dst) {
                        Ok(n) => info!(
                            "[KR64] copied diagnostic log {} -> {} ({} bytes)",
                            src, dst, n
                        ),
                        Err(e) => warning!("[KR64] failed to copy diagnostic log {}: {}", src, e),
                    }
                }
            }
            // The child has exited — we're done. Skip the normal waitpid
            // loop below (it would return ECHILD immediately).
            return if exit_code >= 0 { exit_code } else { 1 };
        } else {
            warning!(
                "[KR64][parent] child did not stop as expected (status=0x{:x}) — ptrace emulation skipped",
                status
            );
        }
    }

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
        // Use the standard Android ABI directory names (arm64-v8a, not
        // "arm64") that `apk_native_lib_candidates_in` scans for.
        let apk_root = tmp.join("~~random1==").join("io.twoyi-random2==");
        let x86_64_lib = apk_root.join("lib/x86_64/libgetpid_hook.so");
        let arm64_lib = apk_root.join("lib/arm64-v8a/libgetpid_hook.so");
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
            cands[1].ends_with("/lib/arm64-v8a/libgetpid_hook.so"),
            "arm64-v8a must be second: {}",
            cands[1]
        );

        // Cleanup.
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// `apk_native_lib_candidates_in` must ALSO find the library under
    /// the DIRECT APK install layout used by older Android and some
    /// custom ROMs: `/data/app/<pkg>-<n>/lib/<abi>/<lib>` (no `~~<rand>`
    /// bucket level). The original code only handled the bucket layout
    /// and silently returned 0 candidates on devices using the direct
    /// layout — this test guards against that regression.
    #[test]
    fn apk_native_lib_candidates_finds_lib_in_direct_layout() {
        let tmp = std::env::temp_dir().join("twoyi-apk-scan-direct-test");
        let _ = std::fs::remove_dir_all(&tmp);
        // Direct layout: /data/app/io.twoyi-1/lib/<abi>/<lib>
        // (NO ~~<rand> bucket level — the io.twoyi-* dir is directly
        // under the base dir.)
        let apk_root = tmp.join("io.twoyi-1");
        let x86_64_lib = apk_root.join("lib/x86_64/libtwrp_fb_hook.so");
        let arm64_lib = apk_root.join("lib/arm64-v8a/libtwrp_fb_hook.so");
        std::fs::create_dir_all(x86_64_lib.parent().unwrap()).unwrap();
        std::fs::create_dir_all(arm64_lib.parent().unwrap()).unwrap();
        std::fs::write(&x86_64_lib, b"fake i686 ELF in x86_64 dir").unwrap();
        std::fs::write(&arm64_lib, b"fake aarch64 ELF").unwrap();

        let cands = apk_native_lib_candidates_in(&tmp, "libtwrp_fb_hook.so");
        assert_eq!(
            cands.len(),
            2,
            "direct layout: expected 2 candidates (x86_64 + arm64-v8a), got: {:?}",
            cands
        );
        assert!(
            cands[0].ends_with("/lib/x86_64/libtwrp_fb_hook.so"),
            "x86_64 must be first: {}",
            cands[0]
        );
        assert!(
            cands[1].ends_with("/lib/arm64-v8a/libtwrp_fb_hook.so"),
            "arm64-v8a must be second: {}",
            cands[1]
        );

        // Cleanup.
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// `apk_native_lib_candidates_in` must find libs in BOTH layouts
    /// simultaneously (e.g. an old direct-layout install + a new
    /// bucket-layout install under the same /data/app/). The HONOR
    /// NTH-NX9 device layout (`/data/app/~~CUxx...==/io.twoyi-XXX==/`)
    /// is the bucket layout, but mixing both is the most defensive
    /// test case.
    #[test]
    fn apk_native_lib_candidates_finds_lib_in_mixed_layouts() {
        let tmp = std::env::temp_dir().join("twoyi-apk-scan-mixed-test");
        let _ = std::fs::remove_dir_all(&tmp);
        // Direct layout: /data/app/io.twoyi-1/lib/x86_64/<lib>
        let direct_root = tmp.join("io.twoyi-1");
        std::fs::create_dir_all(direct_root.join("lib/x86_64")).unwrap();
        std::fs::write(
            direct_root.join("lib/x86_64/libtwrp_fb_hook.so"),
            b"direct-layout i686",
        )
        .unwrap();
        // Bucket layout: /data/app/~~rand/io.twoyi-2/lib/arm64-v8a/<lib>
        let bucket_root = tmp.join("~~rand").join("io.twoyi-2");
        std::fs::create_dir_all(bucket_root.join("lib/arm64-v8a")).unwrap();
        std::fs::write(
            bucket_root.join("lib/arm64-v8a/libtwrp_fb_hook.so"),
            b"bucket-layout aarch64",
        )
        .unwrap();

        let cands = apk_native_lib_candidates_in(&tmp, "libtwrp_fb_hook.so");
        assert_eq!(
            cands.len(),
            2,
            "mixed layouts: expected 2 candidates (x86_64 + arm64-v8a), got: {:?}",
            cands
        );
        // The exact order depends on the order entries come back from
        // read_dir (filesystem-dependent); just verify both ABIs are
        // present.
        let has_x86_64 = cands
            .iter()
            .any(|p| p.ends_with("/lib/x86_64/libtwrp_fb_hook.so"));
        let has_arm64 = cands
            .iter()
            .any(|p| p.ends_with("/lib/arm64-v8a/libtwrp_fb_hook.so"));
        assert!(has_x86_64, "x86_64 candidate missing: {:?}", cands);
        assert!(has_arm64, "arm64-v8a candidate missing: {:?}", cands);

        // Cleanup.
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// `hook_library_candidates` must DEDUPLICATE candidates when
    /// `cfg.rootfs == format!("{}/rootfs", cfg.data_dir)` — the
    /// production case (core.rs:get_rootfs_dir() returns
    /// `{data_dir}/rootfs`). Without dedup, candidates #1/#4 and #2/#3
    /// are identical, producing 4 log lines for 2 unique paths
    /// (confirmed in HONOR NTH-NX9 kr64.log: 4 "checked:" lines, only
    /// 2 unique paths).
    #[test]
    fn hook_library_candidates_deduplicates_when_rootfs_is_app_rootfs() {
        // Mirror the production config: rootfs = {data_dir}/rootfs.
        let data_dir = "/data/user/11/io.twoyi".to_string();
        let rootfs = format!("{}/rootfs", data_dir); // matches core.rs
        let cfg = Config {
            rootfs,
            data_dir,
            native_lib_dir: None,
            ..Config::default()
        };
        let cands = hook_library_candidates(&cfg, "libtwrp_fb_hook.so");
        // The 4 documented candidates collapse to 2 unique paths after
        // dedup (rootfs/{lib} == data_dir/rootfs/{lib}, and
        // rootfs/system/lib64/{lib} == data_dir/rootfs/system/lib64/{lib}).
        // The APK dir scan returns 0 entries on Linux.
        assert_eq!(
            cands.len(),
            2,
            "expected 2 UNIQUE candidates after dedup (rootfs == {{data_dir}}/rootfs), got {}: {:?}",
            cands.len(),
            cands
        );
        // Verify both unique paths are present.
        let has_rootfs_root = cands
            .iter()
            .any(|p| p == "/data/user/11/io.twoyi/rootfs/libtwrp_fb_hook.so");
        let has_rootfs_lib64 = cands
            .iter()
            .any(|p| p == "/data/user/11/io.twoyi/rootfs/system/lib64/libtwrp_fb_hook.so");
        assert!(
            has_rootfs_root,
            "missing rootfs/<lib> candidate: {:?}",
            cands
        );
        assert!(
            has_rootfs_lib64,
            "missing rootfs/system/lib64/<lib> candidate: {:?}",
            cands
        );
    }

    /// `hook_library_candidates` must put `cfg.native_lib_dir/<lib>` as
    /// candidate #0 (highest priority) when `cfg.native_lib_dir` is
    /// `Some`. This is the Java-passed `nativeLibraryDir` and is the
    /// ONLY reliable APK-source candidate on real devices (where kr64
    /// runs unprivileged and can't listdir /data/app/).
    #[test]
    fn hook_library_candidates_native_lib_dir_is_candidate_zero() {
        let cfg = Config {
            rootfs: "/r".to_string(),
            data_dir: "/d".to_string(),
            native_lib_dir: Some("/data/app/~~rand/io.twoyi-rand/lib/x86_64".to_string()),
            ..Config::default()
        };
        let cands = hook_library_candidates(&cfg, "libtwrp_fb_hook.so");
        assert!(
            !cands.is_empty(),
            "expected at least one candidate, got empty list"
        );
        // Candidate #0 must be {native_lib_dir}/<lib>.
        assert_eq!(
            cands[0], "/data/app/~~rand/io.twoyi-rand/lib/x86_64/libtwrp_fb_hook.so",
            "native_lib_dir must be candidate #0 (highest priority): {:?}",
            cands
        );
        // The remaining candidates must be the 4 documented rootfs
        // paths (no duplicates since /r != /d/rootfs).
        assert!(
            cands.len() >= 5,
            "expected at least 5 candidates (1 native_lib_dir + 4 rootfs), got {}: {:?}",
            cands.len(),
            cands
        );
    }

    /// `hook_library_candidates` must NOT add the native_lib_dir
    /// candidate when `cfg.native_lib_dir` is `None` (the default).
    /// This is the case when kr64 is launched without the
    /// `TWOYI_NATIVE_LIB_DIR` env var (e.g. unit tests, or old Java
    /// side that doesn't pass it yet).
    #[test]
    fn hook_library_candidates_omits_native_lib_dir_when_none() {
        let cfg = Config {
            rootfs: "/r".to_string(),
            data_dir: "/d".to_string(),
            native_lib_dir: None,
            ..Config::default()
        };
        let cands = hook_library_candidates(&cfg, "libgetpid_hook.so");
        // All candidates must start with /r/ or /d/ (no separate
        // native_lib_dir prefix).
        assert!(
            cands
                .iter()
                .all(|p| p.starts_with("/r/") || p.starts_with("/d/")),
            "no native_lib_dir candidate expected when cfg.native_lib_dir is None: {:?}",
            cands
        );
    }

    /// `parse_args` must accept the new `--native-lib-dir <path>` flag
    /// and store it in `cfg.native_lib_dir`. This is the explicit
    /// override path (the env var path is tested separately).
    #[test]
    fn parse_args_native_lib_dir_flag_sets_field() {
        let cfg = parse_args(args(&[
            "--rootfs",
            "/r",
            "--data-dir",
            "/d",
            "--native-lib-dir",
            "/data/app/~~rand/io.twoyi-rand/lib/x86_64",
        ]))
        .unwrap();
        assert_eq!(
            cfg.native_lib_dir.as_deref(),
            Some("/data/app/~~rand/io.twoyi-rand/lib/x86_64")
        );
    }

    /// `parse_args` must reject an empty `--native-lib-dir` argument
    /// with a clear error (rather than silently storing an empty
    /// string that would later confuse `hook_library_candidates`).
    #[test]
    fn parse_args_native_lib_dir_rejects_empty() {
        let r = parse_args(args(&[
            "--rootfs",
            "/r",
            "--data-dir",
            "/d",
            "--native-lib-dir",
            "",
        ]));
        assert!(r.is_err(), "empty --native-lib-dir must error");
        let err = r.unwrap_err();
        assert!(
            err.contains("--native-lib-dir"),
            "error must mention --native-lib-dir: {}",
            err
        );
    }

    /// `--native-lib-dir` is mentioned in `--help` so users discover
    /// it (consistent with the other documented flags).
    #[test]
    fn parse_args_help_mentions_native_lib_dir() {
        let r = parse_args(args(&["--help"]));
        assert!(r.is_err());
        let err = r.unwrap_err();
        assert!(
            err.contains("--native-lib-dir"),
            "--help must mention --native-lib-dir: {}",
            err
        );
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
                "service recovery /sbin/recovery\n    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so"
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
            patched.contains("service recovery /sbin/recovery\n    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so\n    seclabel u:r:recovery:s0"),
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
            .matches("setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so")
            .count();
        assert_eq!(
            count, 1,
            "only the first recovery service should be patched"
        );
    }

    // ====================================================================
    // Tests for `patch_twrp_init_rc_recovery_service_in_rootfs` — the
    // orchestrator that scans multiple .rc files (init.rc, init.recovery.rc,
    // init.recovery.*.rc, system/etc/init/recovery.rc, plus imports) and
    // falls back to creating init.twoyi.rc if none contain the recovery
    // service. These tests use a tempdir to set up a realistic rootfs
    // skeleton so we can verify the file I/O behaviour end-to-end.
    // ====================================================================

    /// Helper: build a unique tempdir rootfs skeleton with the given init.rc
    /// content. Returns the tempdir path (kept alive for the test duration
    /// by the caller holding the TempDir).
    fn make_test_rootfs(init_rc_content: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "twoyi-kr64-test-rootfs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("init.rc"), init_rc_content).unwrap();
        dir
    }

    /// `patch_twrp_init_rc_recovery_service_in_rootfs` must patch init.rc
    /// when init.rc contains the `service recovery` line directly (the x86
    /// TWRP layout — the original code path).
    #[test]
    fn rootfs_patcher_patches_init_rc_when_service_recovery_is_in_init_rc() {
        let dir = make_test_rootfs("service recovery /sbin/recovery\n");
        let rootfs = dir.to_string_lossy().into_owned();
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs);
        let init_rc = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        assert!(
            init_rc.contains("    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so"),
            "init.rc should be patched. Got:\n{}",
            init_rc
        );
        // No fallback file should be created.
        assert!(
            !dir.join("init.twoyi.rc").exists(),
            "init.twoyi.rc should NOT be created when init.rc has the service"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `patch_twrp_init_rc_recovery_service_in_rootfs` must patch
    /// init.recovery.rc (NOT init.rc) when init.rc doesn't contain the
    /// recovery service but init.recovery.rc does. This is the arm64 TWRP
    /// regression scenario described in the task.
    #[test]
    fn rootfs_patcher_patches_init_recovery_rc_when_init_rc_lacks_service() {
        let dir = make_test_rootfs("service ueventd /sbin/ueventd\n");
        // arm64-style: recovery service lives in init.recovery.rc.
        std::fs::write(
            dir.join("init.recovery.rc"),
            "service recovery /sbin/recovery\n",
        )
        .unwrap();
        let rootfs = dir.to_string_lossy().into_owned();
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs);
        // init.rc should be UNTOUCHED (no service recovery line, no patch).
        let init_rc = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        assert!(
            !init_rc.contains("setenv LD_PRELOAD"),
            "init.rc should NOT be patched. Got:\n{}",
            init_rc
        );
        // init.recovery.rc SHOULD be patched.
        let init_recovery_rc = std::fs::read_to_string(dir.join("init.recovery.rc")).unwrap();
        assert!(
            init_recovery_rc.contains("    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so"),
            "init.recovery.rc should be patched. Got:\n{}",
            init_recovery_rc
        );
        // No fallback file should be created.
        assert!(
            !dir.join("init.twoyi.rc").exists(),
            "init.twoyi.rc should NOT be created when init.recovery.rc has the service"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `patch_twrp_init_rc_recovery_service_in_rootfs` must follow `import`
    /// directives in init.rc and patch an imported file when it contains
    /// the recovery service.
    #[test]
    fn rootfs_patcher_follows_import_directives() {
        let dir =
            make_test_rootfs("import /init.recovery.qcom.rc\nservice ueventd /sbin/ueventd\n");
        // Imported file lives at the chroot root (matching the import path).
        std::fs::write(
            dir.join("init.recovery.qcom.rc"),
            "service recovery /sbin/recovery\n",
        )
        .unwrap();
        let rootfs = dir.to_string_lossy().into_owned();
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs);
        // init.rc should be UNTOUCHED.
        let init_rc = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        assert!(
            !init_rc.contains("setenv LD_PRELOAD"),
            "init.rc should NOT be patched. Got:\n{}",
            init_rc
        );
        // The imported file SHOULD be patched.
        let imported = std::fs::read_to_string(dir.join("init.recovery.qcom.rc")).unwrap();
        assert!(
            imported.contains("    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so"),
            "imported init.recovery.qcom.rc should be patched. Got:\n{}",
            imported
        );
        // No fallback file should be created.
        assert!(
            !dir.join("init.twoyi.rc").exists(),
            "init.twoyi.rc should NOT be created when an import contains the service"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `patch_twrp_init_rc_recovery_service_in_rootfs` must skip shell-
    /// style variables in import paths (e.g. `import /init.recovery.${ro.hardware}.rc`)
    /// and rely on the glob step to pick up `init.recovery.*.rc` files.
    #[test]
    fn rootfs_patcher_skips_import_paths_with_unexpanded_vars_and_uses_glob() {
        let dir = make_test_rootfs(
            "import /init.recovery.${ro.hardware}.rc\nservice ueventd /sbin/ueventd\n",
        );
        // Glob-matched file (the import path itself can't be resolved
        // because ${ro.hardware} is unexpanded, but the glob step picks
        // up the file directly).
        std::fs::write(
            dir.join("init.recovery.qcom.rc"),
            "service recovery /sbin/recovery\n",
        )
        .unwrap();
        let rootfs = dir.to_string_lossy().into_owned();
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs);
        let imported = std::fs::read_to_string(dir.join("init.recovery.qcom.rc")).unwrap();
        assert!(
            imported.contains("    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so"),
            "glob-matched init.recovery.qcom.rc should be patched. Got:\n{}",
            imported
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FALLBACK: when NO .rc file (init.rc, init.recovery.rc,
    /// init.recovery.*.rc, system/etc/init/recovery.rc, or imports)
    /// contains the `service recovery` line, the orchestrator must create
    /// `{rootfs}/init.twoyi.rc` with a complete recovery service definition
    /// AND append `import /init.twoyi.rc` to init.rc.
    #[test]
    fn rootfs_patcher_falls_back_to_init_twoyi_rc_when_no_service_found() {
        let dir = make_test_rootfs("service ueventd /sbin/ueventd\n");
        let rootfs = dir.to_string_lossy().into_owned();
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs);

        // init.twoyi.rc should be created with the expected content.
        let twoyi_rc_path = dir.join("init.twoyi.rc");
        assert!(
            twoyi_rc_path.exists(),
            "init.twoyi.rc should be created as fallback"
        );
        let twoyi_rc = std::fs::read_to_string(&twoyi_rc_path).unwrap();
        assert!(
            twoyi_rc.contains("service recovery /sbin/recovery"),
            "init.twoyi.rc should define the recovery service. Got:\n{}",
            twoyi_rc
        );
        assert!(
            twoyi_rc.contains("setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so"),
            "init.twoyi.rc should set LD_PRELOAD. Got:\n{}",
            twoyi_rc
        );
        assert!(
            twoyi_rc.contains("seclabel u:r:recovery:s0"),
            "init.twoyi.rc should set seclabel. Got:\n{}",
            twoyi_rc
        );

        // init.rc should have the import line appended.
        let init_rc = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        assert!(
            init_rc.contains("import /init.twoyi.rc"),
            "init.rc should contain 'import /init.twoyi.rc'. Got:\n{}",
            init_rc
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// IDEMPOTENCE: running the orchestrator twice should not create
    /// duplicate patches or duplicate `import /init.twoyi.rc` lines.
    #[test]
    fn rootfs_patcher_is_idempotent_when_service_in_init_rc() {
        let dir = make_test_rootfs("service recovery /sbin/recovery\n");
        let rootfs = dir.to_string_lossy().into_owned();
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs);
        let init_rc_after_first = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs);
        let init_rc_after_second = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        assert_eq!(
            init_rc_after_first, init_rc_after_second,
            "second run should not modify init.rc (idempotent)"
        );
        let count = init_rc_after_second
            .matches("setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so")
            .count();
        assert_eq!(
            count, 1,
            "exactly one setenv line should be present after two runs"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// IDEMPOTENCE (fallback case): running the orchestrator twice when
    /// no .rc file has the service should not create a duplicate
    /// `import /init.twoyi.rc` line in init.rc.
    #[test]
    fn rootfs_patcher_fallback_is_idempotent_for_import_line() {
        let dir = make_test_rootfs("service ueventd /sbin/ueventd\n");
        let rootfs = dir.to_string_lossy().into_owned();
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs);
        let init_rc_after_first = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs);
        let init_rc_after_second = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        assert_eq!(
            init_rc_after_first, init_rc_after_second,
            "second run should not modify init.rc (idempotent fallback)"
        );
        let count = init_rc_after_second
            .matches("import /init.twoyi.rc")
            .count();
        assert_eq!(
            count, 1,
            "exactly one 'import /init.twoyi.rc' line should be present after two runs"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `collect_imported_rc_files` must handle relative import paths
    /// (relative to the importing file's parent directory), not just
    /// chroot-absolute paths.
    #[test]
    fn rootfs_patcher_handles_relative_import_paths() {
        // init.rc imports a relative path; we put the imported file in
        // the same directory as init.rc.
        let dir = make_test_rootfs("import extra.rc\nservice ueventd /sbin/ueventd\n");
        std::fs::write(dir.join("extra.rc"), "service recovery /sbin/recovery\n").unwrap();
        let rootfs = dir.to_string_lossy().into_owned();
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs);
        let extra = std::fs::read_to_string(dir.join("extra.rc")).unwrap();
        assert!(
            extra.contains("    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so"),
            "relative-imported extra.rc should be patched. Got:\n{}",
            extra
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Build a 34-byte klog_init instruction sequence matching the pattern
    /// that `patch_twrp_init_klog_init` searches for. The `jne_offset`
    /// parameter fills the wildcard byte after `0x75` (jne).
    ///
    /// Only built on non-aarch64 hosts — the pattern-matching tests below
    /// are x86/i386-specific (they verify the i386 klog_init byte pattern)
    /// and the function under test short-circuits to `Skipped` on aarch64.
    #[cfg(not(target_arch = "aarch64"))]
    fn build_klog_init_pattern(jne_offset: u8) -> Vec<u8> {
        let mut v = Vec::new();
        // mov DWORD PTR [esp+0x8], 0x10b
        v.extend_from_slice(&[0xc7, 0x44, 0x24, 0x08, 0x0b, 0x01, 0x00, 0x00]);
        // lea esi, [ebx-0x1502e]
        v.extend_from_slice(&[0x8d, 0xb3, 0xd2, 0xaf, 0xfe, 0xff]);
        // mov DWORD PTR [esp+0x4], 0x2180
        v.extend_from_slice(&[0xc7, 0x44, 0x24, 0x04, 0x80, 0x21, 0x00, 0x00]);
        // mov [esp], esi
        v.extend_from_slice(&[0x89, 0x34, 0x24]);
        // call mknod (relative call: e8 + 4-byte signed offset, here 0x7a 0x9f 0x00 0x00)
        v.extend_from_slice(&[0xe8, 0x7a, 0x9f, 0x00, 0x00]);
        // test eax, eax
        v.extend_from_slice(&[0x85, 0xc0]);
        // jne <offset>
        v.extend_from_slice(&[0x75, jne_offset]);
        assert_eq!(v.len(), 34);
        v
    }

    /// `patch_twrp_init_klog_init` must find the mknod-failure pattern in
    /// an unpatched binary and replace the `jne` (75 ??) with two NOPs
    /// (90 90).
    ///
    /// Skipped on aarch64: the function short-circuits to `Skipped` there
    /// (the i386 byte pattern is irrelevant), so the x86-specific assertions
    /// below would never hold on arm64.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_klog_init_applies_to_unpatched_binary() {
        let mut bytes = build_klog_init_pattern(0xd4);
        // The patch should succeed.
        assert_eq!(
            patch_twrp_init_klog_init(&mut bytes),
            KlogInitPatchResult::Applied,
            "patch should apply to unpatched binary"
        );
        // The jne at offset 32 should now be 90 90 (two NOPs).
        assert_eq!(bytes[32], 0x90, "jne byte 0 should be NOP'd");
        assert_eq!(bytes[33], 0x90, "jne byte 1 should be NOP'd");
    }

    /// `patch_twrp_init_klog_init` must be IDEMPOTENT: applying it twice
    /// yields the same result as applying it once. The second call must
    /// return `AlreadyApplied` (not `Applied`) and must NOT modify the
    /// bytes.
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_klog_init_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_klog_init_is_idempotent() {
        let mut bytes = build_klog_init_pattern(0xd4);
        assert_eq!(
            patch_twrp_init_klog_init(&mut bytes),
            KlogInitPatchResult::Applied,
            "first patch should be Applied"
        );
        let after_first = bytes.clone();
        assert_eq!(
            patch_twrp_init_klog_init(&mut bytes),
            KlogInitPatchResult::AlreadyApplied,
            "second patch should be AlreadyApplied (idempotent)"
        );
        assert_eq!(
            bytes, after_first,
            "second patch should not modify the binary"
        );
    }

    /// `patch_twrp_init_klog_init` must find the pattern even when it's
    /// embedded in a larger binary (with prefix and suffix bytes).
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_klog_init_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_klog_init_finds_pattern_in_context() {
        // 256 bytes of random-ish prefix, then the pattern, then 64 bytes
        // of suffix. The function should find the pattern and patch only
        // the jne.
        let mut bytes = Vec::new();
        // Prefix: 0x90 (NOP) sled to make sure we don't match by accident.
        bytes.extend(std::iter::repeat_n(0x90u8, 256));
        // The pattern.
        let pattern = build_klog_init_pattern(0x2a);
        bytes.extend_from_slice(&pattern);
        // Suffix.
        bytes.extend(std::iter::repeat_n(0xccu8, 64));

        let jne_file_offset = 256 + 32; // pattern_start + 32
        assert_eq!(bytes[jne_file_offset], 0x75, "jne should be unpatched");
        assert_eq!(bytes[jne_file_offset + 1], 0x2a, "jne offset byte");

        assert_eq!(
            patch_twrp_init_klog_init(&mut bytes),
            KlogInitPatchResult::Applied,
            "patch should apply"
        );
        assert_eq!(bytes[jne_file_offset], 0x90, "jne byte 0 should be NOP'd");
        assert_eq!(
            bytes[jne_file_offset + 1],
            0x90,
            "jne byte 1 should be NOP'd"
        );
        // Prefix and suffix should be untouched.
        for (i, b) in bytes.iter().enumerate() {
            if (256..256 + 32).contains(&i) {
                // Pattern prefix (before jne) — should be unchanged.
                assert_eq!(
                    *b,
                    pattern[i - 256],
                    "pattern byte at {} should be unchanged",
                    i
                );
            } else if i < 256 {
                // Prefix.
                assert_eq!(*b, 0x90, "prefix byte at {} should be unchanged", i);
            } else if (256 + 34..256 + 34 + 64).contains(&i) {
                // Suffix.
                assert_eq!(*b, 0xcc, "suffix byte at {} should be unchanged", i);
            }
        }
    }

    /// `patch_twrp_init_klog_init` must return `NotFound` if the pattern
    /// is not found (e.g. a different TWRP version with a different code
    /// layout). This is important — we must NOT silently corrupt an
    /// unknown binary.
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_klog_init_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_klog_init_returns_not_found_if_pattern_not_found() {
        // 256 bytes of 0x90 (NOP) — doesn't contain the pattern.
        let mut bytes = vec![0x90u8; 256];
        assert_eq!(
            patch_twrp_init_klog_init(&mut bytes),
            KlogInitPatchResult::NotFound,
            "should return NotFound if pattern is not found"
        );
        // Bytes should be unchanged.
        assert!(
            bytes.iter().all(|&b| b == 0x90),
            "bytes should be unchanged"
        );
    }

    /// `patch_twrp_init_klog_init` must NOT patch if the byte after `0x75`
    /// (jne) is not what we expect — i.e. if the pattern matched by
    /// coincidence but the next instruction isn't a jne. We must be
    /// conservative and not corrupt the binary.
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_klog_init_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_klog_init_does_not_patch_if_jne_byte_unexpected() {
        // Build a pattern that matches up to byte 31, then has `0x75` at
        // offset 32, but with a strange byte after it... actually our
        // pattern requires `0x75` at offset 32 and patches the byte at
        // offset 33. So if `0x75` is present, we always patch.
        //
        // Instead, test the case where the byte at offset 32 is NEITHER
        // 0x75 (jne) NOR 0x90 (already patched). We modify the pattern
        // so the byte at offset 32 is 0xeb (jmp) — which means the pattern
        // matched by coincidence but it's not a jne.
        let mut bytes = build_klog_init_pattern(0xd4);
        bytes[32] = 0xeb; // jmp instead of jne
                          // The function should detect this and NOT patch.
        assert_eq!(
            patch_twrp_init_klog_init(&mut bytes),
            KlogInitPatchResult::NotFound,
            "should return NotFound (NOT patch) if byte at jne location is unexpected"
        );
        // Byte at offset 32 should be unchanged.
        assert_eq!(bytes[32], 0xeb, "byte at jne location should be unchanged");
    }

    /// `patch_twrp_init_klog_init` must work on a real TWRP init binary
    /// extracted from `assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img`. This
    /// is a regression test: if the TWRP version changes, this test will
    /// fail (alerting us to update the pattern).
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_klog_init_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_klog_init_works_on_real_twrp_init_binary() {
        // The TWRP boot image is at `assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img`
        // (relative to the repo root). We need to extract the ramdisk,
        // decompress it, and read the /init file.
        //
        // This test is somewhat slow (it parses a 7 MB ramdisk) but it's
        // the most important regression test — it verifies the pattern
        // matches the actual TWRP init binary we ship.
        let boot_img_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img");
        if !boot_img_path.exists() {
            eprintln!(
                "skip: TWRP boot image not found at {} (this is OK in CI without assets)",
                boot_img_path.display()
            );
            return;
        }
        // Read the boot image, extract ramdisk, decompress, parse cpio,
        // and find the /init entry.
        let boot_bytes = match std::fs::read(&boot_img_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("skip: failed to read TWRP boot image: {}", e);
                return;
            }
        };
        if boot_bytes.len() < 0x1000 || &boot_bytes[..8] != b"ANDROID!" {
            eprintln!("skip: TWRP boot image is not a valid Android boot image");
            return;
        }
        // Parse Android boot image header (v0).
        let kernel_size =
            u32::from_le_bytes([boot_bytes[8], boot_bytes[9], boot_bytes[10], boot_bytes[11]])
                as usize;
        let ramdisk_size = u32::from_le_bytes([
            boot_bytes[16],
            boot_bytes[17],
            boot_bytes[18],
            boot_bytes[19],
        ]) as usize;
        let page_size = u32::from_le_bytes([
            boot_bytes[36],
            boot_bytes[37],
            boot_bytes[38],
            boot_bytes[39],
        ]) as usize;
        if page_size == 0 || kernel_size == 0 || ramdisk_size == 0 {
            eprintln!("skip: TWRP boot image has invalid header");
            return;
        }
        let ramdisk_off = page_size + kernel_size.div_ceil(page_size) * page_size;
        if ramdisk_off + ramdisk_size > boot_bytes.len() {
            eprintln!("skip: TWRP boot image ramdisk truncated");
            return;
        }
        let ramdisk_gz = &boot_bytes[ramdisk_off..ramdisk_off + ramdisk_size];
        // Verify gzip magic.
        if ramdisk_gz.len() < 2 || ramdisk_gz[0] != 0x1f || ramdisk_gz[1] != 0x8b {
            eprintln!("skip: TWRP ramdisk is not gzip-compressed");
            return;
        }
        // Decompress gzip using flate2 if available; otherwise skip.
        // We use std::io::Read with a Cursor to decompress.
        let ramdisk_cpio = match decompress_gzip(ramdisk_gz) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("skip: failed to decompress TWRP ramdisk: {}", e);
                return;
            }
        };
        // Parse cpio (newc format) and find the "init" entry.
        let init_bytes = match find_cpio_entry(&ramdisk_cpio, b"init") {
            Some(b) => b,
            None => {
                eprintln!("skip: /init not found in TWRP ramdisk cpio");
                return;
            }
        };
        // Verify the pattern is present (unpatched).
        let pattern_len = 34;
        assert!(
            init_bytes.len() >= pattern_len,
            "init binary too small: {} bytes",
            init_bytes.len()
        );
        // Find the pattern manually to verify it's there pre-patch.
        let mut found_unpatched = false;
        for i in 0..=(init_bytes.len() - pattern_len) {
            if init_bytes[i + 32] == 0x75 {
                let p = &init_bytes[i..i + 32];
                if p[0..8] == [0xc7, 0x44, 0x24, 0x08, 0x0b, 0x01, 0x00, 0x00]
                    && p[8..14] == [0x8d, 0xb3, 0xd2, 0xaf, 0xfe, 0xff]
                    && p[14..22] == [0xc7, 0x44, 0x24, 0x04, 0x80, 0x21, 0x00, 0x00]
                    && p[22..25] == [0x89, 0x34, 0x24]
                    && p[25] == 0xe8
                    && p[30..32] == [0x85, 0xc0]
                {
                    found_unpatched = true;
                    break;
                }
            }
        }
        assert!(
            found_unpatched,
            "klog_init mknod-failure pattern should be present in real TWRP init binary (TWRP version may have changed — update patch_twrp_init_klog_init pattern)"
        );
        // Apply the patch.
        let mut init_bytes_mut = init_bytes.clone();
        assert_eq!(
            patch_twrp_init_klog_init(&mut init_bytes_mut),
            KlogInitPatchResult::Applied,
            "patch should apply to real TWRP init binary"
        );
        // Apply again — should be idempotent (AlreadyApplied).
        assert_eq!(
            patch_twrp_init_klog_init(&mut init_bytes_mut),
            KlogInitPatchResult::AlreadyApplied,
            "patch should be idempotent on real TWRP init binary"
        );
    }

    /// Decompress a gzip-encoded byte slice using flate2.
    ///
    /// Only built on non-aarch64 hosts — this helper is used exclusively
    /// by `patch_twrp_init_klog_init_works_on_real_twrp_init_binary`,
    /// which is itself cfg-gated to non-aarch64 hosts (see that test for
    /// the rationale).
    #[cfg(not(target_arch = "aarch64"))]
    fn decompress_gzip(input: &[u8]) -> std::io::Result<Vec<u8>> {
        use std::io::Read;
        // flate2 is a dependency of kr64 (used elsewhere); we use it here
        // to decompress the TWRP ramdisk.
        let decoder = flate2::read::GzDecoder::new(input);
        let mut out = Vec::with_capacity(input.len() * 8);
        decoder.take(64 * 1024 * 1024).read_to_end(&mut out)?;
        Ok(out)
    }

    /// Find a regular file entry in a cpio newc archive by name.
    /// Returns the file's bytes, or None if not found.
    ///
    /// Only built on non-aarch64 hosts — see `decompress_gzip` above.
    #[cfg(not(target_arch = "aarch64"))]
    fn find_cpio_entry(cpio: &[u8], name: &[u8]) -> Option<Vec<u8>> {
        let mut pos = 0;
        while pos + 110 <= cpio.len() {
            if &cpio[pos..pos + 6] != b"070701" {
                return None;
            }
            pos += 6;
            // 13 8-char hex fields
            let mut fields = [0u32; 13];
            for i in 0..13 {
                let s = std::str::from_utf8(&cpio[pos + i * 8..pos + (i + 1) * 8]).ok()?;
                fields[i] = u32::from_str_radix(s, 16).ok()?;
            }
            pos += 13 * 8;
            let mode = fields[1];
            let filesize = fields[6] as usize;
            let namesize = fields[11] as usize;
            if pos + namesize > cpio.len() {
                return None;
            }
            let entry_name = &cpio[pos..pos + namesize.saturating_sub(1)];
            pos += namesize;
            pos = (pos + 3) & !3;
            if entry_name == b"TRAILER!!!" {
                return None;
            }
            if pos + filesize > cpio.len() {
                return None;
            }
            let data = &cpio[pos..pos + filesize];
            pos += filesize;
            pos = (pos + 3) & !3;
            // Regular file?
            if (mode >> 12) == 0o10 && entry_name == name {
                return Some(data.to_vec());
            }
        }
        None
    }
}
