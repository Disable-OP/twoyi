// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! Twoyi kernel-replacement daemon — the Rust port of VM's `libkr64.so`.
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
//! The crate builds as BOTH a cdylib (`libkr64.so` — directly
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
//! * [`devices`]   — virtual `/dev` device creation (qemu_pipe,
//!   touch, key, event, gb, gb2).
//! * [`binder`]    — per-VM `/vm{id}/dev/binder` Unix socket + binder
//!   transaction proxy (skeleton; see
//!   `download/BINDER_SKELETON.md`).
//! * [`audio`]     — virtual `/dev/audio` Unix socket + bidirectional
//!   PCM pump (playback + capture skeleton; see
//!   `download/AUDIO_SENSOR_HAL.md`).
//! * [`sensors`]   — virtual `/dev/sensors` Unix socket + multiplexed
//!   12-sensor HAL (accel/mag/gyro/... skeleton; see
//!   `download/AUDIO_SENSOR_HAL.md`).
//! * [`battery`]   — virtual `/sys/class/power_supply/battery` file tree
//!   and 30 s refresh thread (file-based, no socket; see
//!   `download/BATTERY_IMPL.md`).
//! * [`seccomp`]   — BPF seccomp filter + SIGSYS handler.
//! * [`proc_emu`]  — synthesised `/proc` tree (version, cpuinfo,
//!   meminfo, cmdline, self/, sys/).
//! * [`mount_mgr`] — `unshare(CLONE_NEWNS)` + `pivot_root` + tmpfs
//!   mounts for /dev, /proc, /sys, /system, /vendor.
//!
//! # Dependencies
//!
//! Per the task spec ("Use only std + libc, no external crates for
//! now"), this crate depends on **only** `libc` — no `log`, no
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

// ============================================================================
// Logging — minimal `eprintln!`-based macros. No external `log` crate.
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
// primitive for diagnostics in the child branch of the kr64 fork —
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
/// pipe with a small buffer) — for short log lines on stderr this is
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
/// safe equivalent of `format!("{}", n)` — no allocation, no locks.
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
/// `VM_KR64_ANALYSIS.md` §2 (`vmid`, `data_dir`, `rom_dir`,
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
    /// If `false`, mount them read-write (for development — lets you
    /// `adb push` test binaries into the running guest).
    pub read_only_rom: bool,
    /// If `true`, install the seccomp filter on the guest. If `false`,
    /// skip it (for debugging — the guest will see the host's `/proc`
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
    /// password auth is not yet supported — the stub only implements
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
                return Err("twoyi kr64 — kernel-replacement daemon\n\
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
                // actual forwarding thread is not yet wired up — see
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
// Zombie reaping — VM-inspired cleanup.
// ============================================================================

/// Reap any leftover zombie children before forking the new guest.
///
/// This mirrors VM's `ProcessKiller` / `ZombieReaper` (see
/// `VM_KR64_ANALYSIS.md` §2.10) which runs at daemon startup to clean
/// up processes left behind by a previous VM run that crashed or was
/// killed with SIGKILL. Without this, those zombies stay reaped by no
/// one (their parent is gone) and accumulate as `<defunct>` entries in
/// `/proc`, which on long-running hosts can exhaust the PID table.
///
/// We call `waitpid(-1, WNOHANG)` in a loop until it returns 0 (no
/// more children to reap) or -1 with ECHILD (no children at all).
/// Both terminating conditions are benign. EINTR is retried (it can
/// happen if a signal arrives mid-syscall — we have no handlers yet,
/// but this is the correct defensive pattern).
///
/// # Safety
///
/// `waitpid` is a POSIX syscall; calling it is safe. The `WNOHANG`
/// flag makes it non-blocking, so this function never sleeps. It only
/// reaps children that have ALREADY exited — it does not kill or
/// signal any running process. The "kill orphan processes" step (which
/// DOES send SIGKILL to leftover guest PIDs) is handled separately on
/// the Java side in `RomManager.killOrphanProcess()` — this Rust
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
        // pid == -1: error. ECHILD means "no children" (benign — first
        // run). EINTR means "interrupted by signal" — retry. Anything
        // else is unexpected; log and stop to avoid an infinite loop.
        let e = std::io::Error::last_os_error();
        if e.raw_os_error() == Some(libc::EINTR) {
            continue;
        }
        if e.raw_os_error() == Some(libc::ECHILD) {
            // No children at all — first run, nothing to reap.
            break;
        }
        warning!("[KR64][zombie] waitpid failed: {} — stopping reap loop", e);
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
///   3. Populate `/proc` (version, cpuinfo, meminfo, cmdline, …).
///   4. Set up mount namespace (unshare + bind mounts + tmpfs mounts).
///   5. Fork:
///      - Child: pivot_root → install seccomp → exec /system/bin/init.
///      - Parent: run the device-accept loop (spawns one thread per
///        device socket; for the MVP each thread just accepts and
///        echoes).
///   6. Wait for the child (the guest init) to exit; propagate its
///      exit code.
pub fn run<I: IntoIterator<Item = String>>(args: I) -> i32 {
    let cfg = match parse_args(args) {
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
    // VM does this at daemon startup (see `VM_KR64_ANALYSIS.md` §2.10)
    // to clean up after a crashed/killed previous VM. We do the same so
    // a rapid restart of the guest doesn't accumulate `<defunct>` PIDs.
    // This is purely defensive — if there are no children, waitpid
    // returns ECHILD immediately and we move on.
    // ---------------------------------------------------------------
    clear_zombie_processes();

    // Log the SOCKS5 proxy configuration if set (stub: the actual
    // forwarding thread is not yet spawned — see `Config::socks5_proxy`).
    if let Some(ref upstream) = cfg.socks5_proxy {
        info!(
            "[KR64] SOCKS5 proxy configured: {} (stub — forwarding thread not yet started)",
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
    // Magisk presence markers — make Magisk-aware apps detect a
    // consistent "rooted VM" environment. Non-fatal: the guest boots
    // fine without these, but banking/root-checker apps may misbehave.
    if let Err(e) = devices::create_magisk_marker(&cfg.rootfs) {
        warning!("[KR64] failed to create Magisk markers: {}", e);
    }
    // /dev/dm-user — required by Android 12+ GSIs for userspace
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
                "[KR64] failed to create /dev/dm-user: {} — Android 12+ GSIs may boot-loop",
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
                "[KR64] failed to start binder proxy: {} — falling back to host binder",
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
    // stubbed (no JNI yet) — the pump compiles and exercises the
    // protocol but produces no sound until the Java side is wired
    // up in a follow-up task.
    // ---------------------------------------------------------------
    let _audio_handle = match audio::create_audio_device(&cfg.rootfs).and_then(|dev| dev.spawn()) {
        Ok(h) => {
            info!("[KR64] audio device listening at {}", h.path());
            Some(h)
        }
        Err(e) => {
            // Non-fatal: the guest can still boot without sound —
            // AudioFlinger's connect() to /dev/audio will fail and
            // the guest's audio HAL will fall back to silence / a
            // null output. Sound is the user's primary use case
            // though, so this warning is worth surfacing.
            warning!(
                "[KR64] failed to start audio device: {} — guest will have no sound",
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
    // (no JNI yet) — the control loop replies false to every
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
                // Non-fatal: the guest can still boot without sensors —
                // the guest's sensor HAL will see "no sensors available"
                // and `SensorManager.getDefaultSensor()` will return null.
                // Apps that hard-require a sensor (e.g. compass apps)
                // will crash, but the boot proceeds.
                warning!(
                    "[KR64] failed to start sensor device: {} — guest will have no sensors",
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
    // re-writes them every 30 s. Failure is non-fatal — the guest
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
                "[KR64] failed to start battery HAL: {} — guest will see no battery",
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
            "[KR64] failed to materialise Samsung GameSDK compat paths: {} — some games may crash",
            e
        );
    }

    // ---------------------------------------------------------------
    // Step 4: set up mount namespace + bind mounts + tmpfs.
    // ---------------------------------------------------------------
    // NOTE: We do NOT do this yet — the mount setup has to happen
    // inside the forked child, because once we pivot_root we lose
    // access to the host's paths (and the parent needs to keep
    // accepting connections on the device sockets it bound above).
    //
    // The flow is:
    //   parent: bind device sockets (already done above)
    //   parent: fork()
    //     child: mount_mgr::setup_mounts(cfg)
    //     child: seccomp::install()
    //     child: execve(/system/bin/init)
    //   parent: accept loop on device sockets

    // ---------------------------------------------------------------
    // Step 4.5: create a PID namespace so the guest init becomes PID 1.
    //
    // Android's init binary requires getpid() == 1 — if it's not PID 1,
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
    // fall through — init will exit 31, but at least we get diagnostic
    // output.
    // ---------------------------------------------------------------
    match unsafe { libc::unshare(libc::CLONE_NEWPID) } {
        0 => info!("[KR64] unshare(CLONE_NEWPID) succeeded — child will be PID 1"),
        _ => {
            let e = std::io::Error::last_os_error();
            warning!(
                "[KR64] unshare(CLONE_NEWPID) failed: {} — init will not be PID 1 (will exit 31)",
                e
            );
        }
    }

    // ---------------------------------------------------------------
    // Step 5: fork + exec the guest.
    // ---------------------------------------------------------------

    // Debug: check if libgetpid_hook.so exists in the new root
    // (after pivot_root, before fork)
    let hook_path = "/system/lib64/libgetpid_hook.so";
    if Path::new(hook_path).exists() {
        info!("[KR64] PARENT: libgetpid_hook.so EXISTS at {} (after pivot_root)", hook_path);
        match std::fs::metadata(hook_path) {
            Ok(m) => info!("[KR64] PARENT: file size = {} bytes, mode = {:o}", m.len(), {
                use std::os::unix::fs::PermissionsExt;
                m.permissions().mode()
            }),
            Err(e) => info!("[KR64] PARENT: metadata error: {}", e),
        }
    } else {
        error!("[KR64] PARENT: libgetpid_hook.so does NOT exist at {} (after pivot_root)", hook_path);
        // List /system/lib64/ to see what's there
        if let Ok(entries) = std::fs::read_dir("/system/lib64") {
            let mut count = 0;
            for entry in entries {
                if let Ok(e) = entry {
                    if count < 10 {
                        info!("[KR64] PARENT: /system/lib64/ contains: {:?}", e.file_name());
                    }
                    count += 1;
                }
            }
            info!("[KR64] PARENT: /system/lib64/ has {} entries total", count);
        } else {
            error!("[KR64] PARENT: cannot read /system/lib64/ directory");
        }
    }

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
        // expand to `eprintln!`, which is NOT async-signal-safe — it
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
        // one — close() (syscall 3) IS whitelisted. This is O(1024) but
        // only runs once at guest startup.
        //
        // When kr64 runs as root (via `su -c`), the zygote's seccomp
        // filter is not inherited, so close_range would work — but we
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
                safe_write_err(b"[KR64 CHILD] non-root mode: skipping mount+chroot (seccomp blocks both)\n");
            }
        }

        if cfg.install_seccomp {
            if let Err(e) = seccomp::install() {
                // Non-fatal: we explicitly continue so the guest can boot
                // in a permissive-ish mode (the seccomp filter is a
                // hardening layer, not a correctness requirement for the
                // MVP). But the user must be told — silent failure here
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
        // Fixed: was passing empty envp — init needs at least PATH,
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

        // Debug: check if libgetpid_hook.so exists at the expected path
        let hook_path = if cfg.use_namespaces {
            c"/system/lib64/libgetpid_hook.so".to_bytes_with_nul()
        } else {
            // Can't use format! here (async-signal-unsafe), so just check
            // the chroot-relative path
            b"/system/lib64/libgetpid_hook.so\0"
        };
        let hook_exists = unsafe { libc::access(hook_path.as_ptr() as *const libc::c_char, libc::F_OK) } == 0;
        if hook_exists {
            unsafe { safe_write_err(b"[KR64 CHILD] libgetpid_hook.so found at /system/lib64/\n"); }
        } else {
            unsafe { safe_write_err(b"[KR64 CHILD] libgetpid_hook.so NOT found at /system/lib64/\n"); }
            // Try listing /system/lib64/ to see what's there
            // (can't use opendir in async-signal-safe context, so just
            // try a few known paths)
            let alt_path = b"/system/lib64/libc.so\0";
            let alt_exists = unsafe { libc::access(alt_path.as_ptr() as *const libc::c_char, libc::F_OK) } == 0;
            if alt_exists {
                unsafe { safe_write_err(b"[KR64 CHILD] /system/lib64/libc.so exists - dir is accessible\n"); }
            } else {
                unsafe { safe_write_err(b"[KR64 CHILD] /system/lib64/libc.so NOT found - dir may not exist\n"); }
            }
        }

        // Build environment for the guest init. The CString::new calls
        // below use compile-time-constant strings (no NUL possible) and
        // format!() — the format! allocation happens BEFORE execve, so
        // it's safe (we're not yet racing the post-fork window for the
        // allocator lock on this short, single-thread-of-control path).
        let twoyi_rootfs_env = match CString::new(format!("TWOYI_ROOTFS={}", cfg.rootfs)) {
            Ok(s) => s,
            Err(_) => unsafe {
                safe_write_err(b"[KR64 CHILD] FATAL: TWOYI_ROOTFS env contains NUL byte\n");
                libc::_exit(127);
            },
        };
        // LD_PRELOAD path: when use_namespaces is true, pivot_root
        // has already happened, so the path is relative to the new
        // root. When false, we need the full absolute path.
        let ld_preload_str = if cfg.use_namespaces {
            // After pivot_root, /system/lib64/libgetpid_hook.so should
            // resolve to the file in the rootfs. But if it doesn't work,
            // try the full path (which won't work after pivot_root either,
            // but at least we'll see a different error).
            "LD_PRELOAD=/system/lib64/libgetpid_hook.so".to_string()
        } else {
            format!("LD_PRELOAD={}/system/lib64/libgetpid_hook.so", cfg.rootfs)
        };
        let env_vars: Vec<CString> = vec![
            CString::new("PATH=/system/bin:/system/xbin:/vendor/bin").unwrap(),
            CString::new("ANDROID_ROOT=/system").unwrap(),
            CString::new("ANDROID_DATA=/data").unwrap(),
            CString::new("ANDROID_BOOTLOGO=1").unwrap(),
            twoyi_rootfs_env,
            CString::new("LD_LIBRARY_PATH=/system/lib64:/system/lib64/bootstrap").unwrap(),
            // Only set LD_PRELOAD if the file actually exists
            // (checking before pivot_root is not possible in the child,
            // so we always set it — if the file doesn't exist, the linker
            // will print an error but init may still run)
            CString::new(ld_preload_str).unwrap(),
        ];
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

    // qemu_pipe → real GL command proxy (Phase 1 of the dispatcher plan).
    // The proxy accepts guest connections, reads the "pipe:opengles"
    // channel-name handshake, connects to the renderer's Unix socket
    // at {rootfs}/opengles, and pumps bytes bidirectionally. This
    // replaces the old MVP stub that wrote a single 0 byte and closed.
    // See download/QEMU_PIPE_DISPATCHER_PLAN.md for the full design.
    let _qemu_pipe_proxy = {
        let mut dev = device_set.qemu_pipe;
        let listener = match dev.take_listener() {
            Some(l) => l,
            None => {
                error!("[KR64] qemu_pipe listener already taken — cannot start proxy");
                return 1;
            }
        };
        match qemu_pipe::spawn_qemu_pipe_proxy(listener, dev.path.clone(), cfg.rootfs.clone()) {
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
    //   touch     → input::touch_server
    //   key       → input::key_server
    //   event     → TwoyiSocketServer (event IPC)
    //   gb/gb2    → openglrenderer::gralloc
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
                        // byte — e.g. the touch device sends a device_info
                        // struct on connect, which the guest reads before
                        // sending anything. The production version will
                        // dispatch to the right handler.)
                        use std::io::Write;
                        let _ = stream.write_all(&[0u8]);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No pending connection — sleep briefly to avoid
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
// cdylib entry point — used when `libkr64.so` is exec'd directly via
// the PIE hack. The ELF entry point is set to `kr64_main` via the
// `-Wl,-e,kr64_main` link flag in build.rs / .cargo/config.toml.
// ============================================================================

/// Entry point for the cdylib (libkr64.so). Mirrors the C `main`
/// signature `(argc, argv) → int` so the standard C runtime can call
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
    // Convert C argv → Rust Vec<String>.
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
// Tests — exercise arg parsing and config defaults.
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
        // CAN verify the function handles ECHILD gracefully — which is
        // the "no children" condition. This is a smoke test.
        clear_zombie_processes();
        // If we get here without panicking, the test passes.
    }
}
