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

pub mod apex_extract;
pub mod audio;
pub mod battery;
pub mod binder;
pub mod compat_paths;
pub mod devices;
pub mod haptics;
pub mod hostbridge;
pub mod mount_mgr;
pub mod proc_emu;
pub mod ptrace_emu;
pub mod qemu_pipe;
pub mod seccomp;
pub mod sensors;
pub mod symlinks;
pub mod vfs;

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
//
// 6-Z131: EVERY line emitted through these macros (and the ptrace loop's
// `log` closure) is capped by `cap_log_line` below. The app-side
// FileLogger tee re-reads this process's stderr line-by-line; a
// multi-megabyte diagnostic line (guest children inherit kr64's stderr
// fd and can write huge blobs without a newline) OOM'd the app in run
// 32786386000 (145 MB single line -> OutOfMemoryError -> the whole app
// died mid-boot). Defense in depth: the app now bounds its reads too,
// but kr64 must never EMIT an unbounded line in the first place.
// ============================================================================

/// 6-Z131: hard cap (bytes) for a single emitted diagnostic log line.
pub(crate) const MAX_LOG_LINE: usize = 8192;

/// 6-Z131: cap `s` at `max` bytes, appending "...[log line truncated]"
/// when it actually truncates. Truncation is clamped to a UTF-8 char
/// boundary -- slicing a `str` at a non-boundary PANICS, and this runs
/// on the ptrace loop's hot path where a panic would kill the guest.
///
/// Returns a borrowed `Cow` (zero-copy) when the line is already short
/// enough, which is the overwhelmingly common case.
pub(crate) fn cap_log_line(s: &str, max: usize) -> std::borrow::Cow<'_, str> {
    if s.len() <= max {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    std::borrow::Cow::Owned(format!("{}...[log line truncated]", &s[..end]))
}

/// 6-Z260: wall-clock anchor for boot-time forensics. The first call
/// initializes it (effectively daemon start — the first log happens on
/// the daemon's startup path). Every emitted line carries `[+Nms]` so
/// any artifact can be turned into a boot-phase timeline without
/// relying on external timestamps (kr64's stderr has no clock of its
/// own, and the app-side logcat tee only covers a filtered slice).
pub(crate) static BOOT_CLOCK: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Milliseconds since the boot clock anchor (0 until initialized).
pub(crate) fn boot_elapsed_ms() -> u128 {
    match BOOT_CLOCK.get() {
        Some(t) => t.elapsed().as_millis(),
        None => 0,
    }
}

/// Initialize the boot clock (call once at daemon entry).
pub(crate) fn boot_clock_init() {
    let _ = BOOT_CLOCK.set(std::time::Instant::now());
}

// ── 6-Z267: buffered tracer-stderr line sink ─────────────────────────
//
// The ptrace loop's `log` used to pay ONE write(2) per line on the
// stderr pipe (std's Stderr is UNBUFFERED) — and the downstream chain
// (app-side FileLogger tee reading the pipe line-by-line → logcat →
// disk) is the exact chain 6-Z260 measured as a visible share of the
// phone's boot time ("kr64 blocks when the pipe fills, and the whole
// guest is frozen while the tracer waits on write()"). The pre-cap
// 6-Z210 broad-DIAG phase alone emits up to 20k lines per boot.
//
// The sink batches whole lines into a 16 KiB buffer (≈1 write per
// 150-300 lines instead of one per line) while keeping the emitted
// byte stream IDENTICAL: same line format, same order, newline-
// terminated lines (the tee's line reader is unaffected). Flush points:
//   * threshold reached inside the sink (the common case),
//   * an explicit flush() (RAII guard at run_ptrace_loop exit — every
//     early return reclaims the guard and drains the tail),
//   * Mutex-poisoned fallback writes directly (never lose a line to a
//     poisoned lock).
// The only unflushed-tail window is a hard SIGKILL mid-boot with a
// non-empty buffer (≤16 KiB) — the same window in which ANY stderr
// writer on the device loses data; the stop-ring dumps cover forensics.
static TRACE_LINE_BUF: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
const TRACE_LINE_FLUSH_BYTES: usize = 16 * 1024;

fn trace_line_flush_locked(buf: &mut String) {
    if !buf.is_empty() {
        let _ = std::io::Write::write_all(&mut std::io::stderr(), buf.as_bytes());
        buf.clear();
    }
}

/// Emit one tracer line through the buffer (format identical to the
/// old direct writeln!: `[KR64][ptrace][+<ms>] <line>`).
pub(crate) fn trace_log_line(msg: &str) {
    match TRACE_LINE_BUF.lock() {
        Ok(mut buf) => {
            buf.push_str(&format!(
                "[KR64][ptrace][+{}ms] {}\n",
                boot_elapsed_ms(),
                cap_log_line(msg, MAX_LOG_LINE)
            ));
            if buf.len() >= TRACE_LINE_FLUSH_BYTES {
                trace_line_flush_locked(&mut buf);
            }
        }
        Err(_) => {
            // Poisoned (a panic while the buffer was held): never drop
            // a line — write it through directly.
            let _ = std::io::Write::write_all(
                &mut std::io::stderr(),
                format!(
                    "[KR64][ptrace][+{}ms] {}\n",
                    boot_elapsed_ms(),
                    cap_log_line(msg, MAX_LOG_LINE)
                )
                .as_bytes(),
            );
        }
    }
}

/// Drain the buffer to stderr immediately (teardown paths).
pub(crate) fn trace_log_flush() {
    if let Ok(mut buf) = TRACE_LINE_BUF.lock() {
        trace_line_flush_locked(&mut buf);
    }
}

/// RAII guard: flushes the trace-line buffer when dropped. Held for the
/// lifetime of `run_ptrace_loop` so EVERY early return (waitpid
/// failure, all-children-gone, teardown) drains the tail exactly once,
/// with no per-path bookkeeping.
pub(crate) struct TraceLogFlushGuard;

impl TraceLogFlushGuard {
    pub(crate) fn new() -> Self {
        TraceLogFlushGuard
    }
}

impl Default for TraceLogFlushGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TraceLogFlushGuard {
    fn drop(&mut self) {
        trace_log_flush();
    }
}

/// 6-Z268: tagged variant of [`trace_log_line`] used by the `info!` /
/// `warning!` / `error!` macros. The macros previously expanded to
/// `eprintln!` — one UNBUFFERED write(2) per line straight onto the
/// stderr file, ~544 call sites including the entire pre-execve staging
/// segment (the segment 6-Z260 measured as disk-I/O-bound; every line
/// also woke the app-side tee). Byte-identical line format, now through
/// the shared 16 KiB buffered sink.
pub(crate) fn trace_log_line_tagged(level: &str, msg: &str) {
    match TRACE_LINE_BUF.lock() {
        Ok(mut buf) => {
            buf.push_str(&format!(
                "[KR64 {}][+{}ms] {}\n",
                level,
                boot_elapsed_ms(),
                cap_log_line(msg, MAX_LOG_LINE)
            ));
            if buf.len() >= TRACE_LINE_FLUSH_BYTES {
                trace_line_flush_locked(&mut buf);
            }
        }
        Err(_) => {
            let _ = std::io::Write::write_all(
                &mut std::io::stderr(),
                format!(
                    "[KR64 {}][+{}ms] {}\n",
                    level,
                    boot_elapsed_ms(),
                    cap_log_line(msg, MAX_LOG_LINE)
                )
                .as_bytes(),
            );
        }
    }
}

/// Log an info-level message to stderr.
/// 6-Z268: routed through the buffered sink (was eprintln! per line).
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::trace_log_line_tagged("INFO", &format!($($arg)*))
    };
}

/// Log a warning-level message to stderr.
///
/// NOTE: this macro is named `warning!` (not `warning!`) to avoid a name
/// conflict with Rust's built-in `#[warn(...)]` lint attribute, which
/// makes the bare name `warn` ambiguous in `pub(crate) use` exports.
/// 6-Z268: routed through the buffered sink (was eprintln! per line).
macro_rules! warning {
    ($($arg:tt)*) => {
        $crate::trace_log_line_tagged("WARN", &format!($($arg)*))
    };
}

/// Log an error-level message to stderr.
/// 6-Z268: routed through the buffered sink (was eprintln! per line).
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::trace_log_line_tagged("ERROR", &format!($($arg)*))
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
pub(crate) unsafe fn safe_write_err(msg: &[u8]) -> isize {
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
pub(crate) unsafe fn format_decimal(buf: &mut [u8; 12], n: i32) -> usize {
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
/// 6-Z226: candidate #4b — {data_dir}/files/<name> is where RomManager
/// extracts the APK assets — the 32-bit hook variants (lib*_arm32.so,
/// see detect_guest_recovery_bitness/guest_hook_lib_name) ship as assets
/// because the package manager extracts only the device's ABI jniLibs.
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
    // 6-Z226 candidate #4b: {data_dir}/files/<name> — where RomManager
    // extracts the APK ASSETS at app init (the libdl.so Option-D pattern).
    // The 32-bit hook variants (lib*_arm32.so) live here: Android's
    // package manager extracts only the DEVICE's ABI libs from jniLibs,
    // so the armeabi-v7a builds must ship as assets instead. Placed AFTER
    // the rootfs symlink candidates: for the regular 64-bit names the
    // rootfs paths are the historically-proven sources, and for the
    // _arm32 names the asset file is the only one that exists.
    // Candidates #1-#4: rootfs and app-level rootfs paths (RomManager's
    // ensureLibSymlink targets).
    out.push(format!("{}/{}", cfg.rootfs, lib_name));
    out.push(format!("{}/system/lib64/{}", cfg.rootfs, lib_name));
    out.push(format!("{}/rootfs/system/lib64/{}", cfg.data_dir, lib_name));
    out.push(format!("{}/rootfs/{}", cfg.data_dir, lib_name));
    out.push(format!("{}/files/{}", cfg.data_dir, lib_name));
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

/// 6-Z226: ELF bitness of the GUEST recovery binary. Reads the 6-byte
/// ELF header (magic + EI_CLASS) of the first candidate that opens.
/// Returns Some(true) for ELFCLASS64, Some(false) for ELFCLASS32, None
/// when no recovery binary exists/is readable (caller defaults to the
/// 64-bit chain — the historic behavior for every arm64 image).
/// Wave-1 corpus runs 251/265/266/267/263 (merlin/ali/athene/bacon/
/// a7xelte): the guest recovery is ELF32 EM_ARM and the staged aarch64
/// hook chain made bionic refuse the preload ("... is 64-bit instead of
/// 32-bit") → init exit 1 — the single largest wave-1 failure class.
pub fn detect_guest_recovery_bitness(rootfs_prefix: &str) -> Option<bool> {
    const CANDIDATES: &[&str] = &["sbin/recovery", "system/bin/recovery"];
    for c in CANDIDATES {
        let p = format!("{}/{}", rootfs_prefix, c);
        let mut hdr = [0u8; 6];
        match std::fs::File::open(&p).and_then(|mut f| {
            use std::io::Read;
            f.read_exact(&mut hdr)
        }) {
            Ok(()) if hdr[0..4] == *b"\x7fELF" => {
                return Some(hdr[4] == 2); // EI_CLASS: 1 = 32-bit, 2 = 64-bit
            }
            _ => continue,
        }
    }
    None
}

/// 6-Z226: the hook-library FILE NAME for the guest's bitness. The 32-bit
/// variants are built by app/cpp/build.sh (armeabi-v7a target) into the
/// APK (jniLibs/armeabi-v7a/) AND as APK assets extracted by RomManager
/// to {data_dir}/files/ at app init. Destination paths in the guest
/// (sbin/libtwrp_fb_hook.so, /dev/... in the LD_PRELOAD chain) are
/// UNCHANGED — only the staged CONTENT differs.
pub fn guest_hook_lib_name(base: &str, guest_is_64: bool) -> String {
    if guest_is_64 {
        base.to_string()
    } else {
        match base {
            "libtwrp_fb_hook.so" => "libtwrp_fb_hook_arm32.so".to_string(),
            "libtwoyi_loader_shlib.so" => "libtwoyi_loader_shlib_arm32.so".to_string(),
            "libgetpid_hook.so" => "libgetpid_hook_arm32.so".to_string(),
            other => {
                // Generic: insert _arm32 before the extension.
                match other.rsplit_once('.') {
                    Some((stem, ext)) => format!("{}_arm32.{}", stem, ext),
                    None => format!("{}_arm32", other),
                }
            }
        }
    }
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

// ============================================================
// 6-Z215: ELF-machine helpers + native-guest detection.
// ============================================================

/// ELF `e_machine` values kr64 can encounter for Android guests/hosts.
const EM_386: u16 = 3;
const EM_ARM: u16 = 40;
const EM_X86_64: u16 = 62;
const EM_AARCH64: u16 = 183;

/// Parse `e_machine` from an in-memory ELF image (handles ELF32/ELF64,
/// little- and big-endian). Returns `None` if the bytes don't look like
/// an ELF header.
///
/// Layout (Simplified/Combined ELF spec):
///   e_ident[0..4] = \x7fELF, e_ident[EI_CLASS=4] = 1 (32-bit) | 2 (64-bit),
///   e_ident[EI_DATA=5] = 1 (LE) | 2 (BE),
///   ELF64: e_type@16(2) e_machine@18(2), ELF32: e_type@16(2) e_machine@18(2)
///   — the e_machine field is at offset 18 in BOTH classes.
pub fn elf_machine_from_bytes(data: &[u8]) -> Option<u16> {
    if data.len() < 20 {
        return None;
    }
    if data[0..4] != [0x7f, b'E', b'L', b'F'] {
        return None;
    }
    let le = match data[5] {
        1 => true,
        2 => false,
        _ => return None,
    };
    // e_machine is a 16-bit field at offset 18 in both ELF32 and ELF64.
    let raw = [data[18], data[19]];
    Some(if le {
        u16::from_le_bytes(raw)
    } else {
        u16::from_be_bytes(raw)
    })
}

/// Parse `e_machine` from a file on the filesystem. `None` = unreadable
/// or not an ELF (missing file, permission, truncated, ...).
pub fn elf_machine(path: &str) -> Option<u16> {
    // Only the first 64 bytes are needed; a bounded read avoids pulling
    // megabytes of libc.so into memory on every boot.
    let mut f = std::fs::File::open(path).ok()?;
    use std::io::Read;
    let mut hdr = [0u8; 64];
    let n = f.read(&mut hdr).ok()?;
    elf_machine_from_bytes(&hdr[..n])
}

/// Map the compile-time host architecture to its ELF e_machine. Used as
/// a LAST-RESORT fallback when the host's bionic files cannot be read —
/// the kr64 binary itself runs on the host, so its arch IS the host arch.
fn host_arch_machine() -> u16 {
    match std::env::consts::ARCH {
        "x86" => EM_386,
        "x86_64" => EM_X86_64,
        "arm" => EM_ARM,
        "aarch64" => EM_AARCH64,
        _ => 0, // unknown — will never equal a guest machine value
    }
}

/// Determine the HOST's bionic e_machine: prefer the host's own
/// runtime-APEX libc.so (exactly the file the guest would otherwise
/// resolve through /apex/com.android.runtime/lib64/bionic), then the
/// host's /system/lib64/libc.so, then the compile-time arch.
fn host_bionic_machine() -> u16 {
    elf_machine("/apex/com.android.runtime/lib64/bionic/libc.so")
        .or_else(|| elf_machine("/system/lib64/libc.so"))
        .unwrap_or_else(host_arch_machine)
}

/// 6-Z215: TRUE when the guest rootfs ships its own libc.so whose ELF
/// machine matches the HOST's bionic machine — i.e. guest processes
/// execute natively (arm64-on-arm64, x86_64-on-x86_64) with NO binfmt
/// runner between them. In that mode the ROM's OWN bionic must take
/// priority over the host's (private-ABI mismatch otherwise).
///
/// FALSE when:
///   * the ROM does not ship {rootfs}/system/lib64/libc.so (nothing to
///     prefer — the host's trees remain the only provider), or
///   * the machines differ (x86_64-host + arm64-ROM = binfmt runner
///     mode — the runner must keep using its own host trees, the
///     6-Z93 narrowing stays in force), or
///   * the guest's libc.so is not a readable ELF (treat as runner mode:
///     conservative default that preserves pre-6-Z215 behavior).
pub fn guest_bionic_is_native(rootfs_prefix: &str) -> bool {
    let guest_libc = format!("{}/system/lib64/libc.so", rootfs_prefix);
    let guest = match elf_machine(&guest_libc) {
        Some(m) => m,
        None => return false,
    };
    let host = host_bionic_machine();
    guest == host && guest != 0
}

/// 6-Z218b: stage the ROM's own bionic into the bootstrap library
/// directories the guest linker hard-codes, when the guest lacks them.
///
/// ROOT CAUSE (r25 lineage-22.2-sailfish, run 33269272962): bionic's
/// linker resolves libc.so/libdl.so for init (bootstrap mode) from the
/// hard-coded path /system/lib64/bootstrap/libc.so WITHOUT consulting
/// LD_LIBRARY_PATH — the 6-Z215 /dev farm (LD_LIBRARY_PATH[0]=/dev)
/// never gets a chance for libc specifically. The lineage ramdisk
/// ships no system/lib64/bootstrap/ directory, so the lookup fell
/// through the VFS to the HOST's bootstrap libc (device 00:34), and
/// the Android-14 host libc inside an Android-15 guest init SIGSEGV'd
/// during early property-area init (si_addr=0x0, pc inside the host
/// libc.so mapping).
///
/// THE FIX: when the guest rootfs ships its own bionic (native-guest
/// mode, see [`guest_bionic_is_native`]) and lacks the bootstrap
/// directory, stage relative symlinks for the bionic set (libc.so,
/// libdl.so, libm.so, libdl_android.so — only names the ROM actually
/// ships as regular files) from the ROM's own system/lib64 into BOTH
/// bootstrap locations the linker is known to search:
///   * {rootfs}/system/lib64/bootstrap/
///   * {rootfs}/apex/com.android.runtime/lib64/bootstrap/
///
/// This mirrors exactly what real recovery ramdisks that DO ship a
/// bootstrap directory provide, and keeps every resolution inside the
/// guest rootfs (§23: host fallback only for resources the guest
/// lacks — a DIFFERENT ABI's libc is not such a resource).
///
/// NEVER overwrites an existing entry (idempotent, hooks/ROM wins).
/// Returns (staged, already_present).
pub fn stage_guest_bootstrap_bionic(rootfs_prefix: &str) -> (usize, usize) {
    const BIONIC_NAMES: [&str; 4] = ["libc.so", "libdl.so", "libm.so", "libdl_android.so"];
    // (guest-relative dir, .. segments needed to reach the rootfs root)
    const BOOTSTRAP_DIRS: [(&str, usize); 2] = [
        ("system/lib64/bootstrap", 3),
        ("apex/com.android.runtime/lib64/bootstrap", 4),
    ];
    // Only stage when the ROM's own bionic exists as a regular file.
    let rom_lib64 = format!("{}/system/lib64", rootfs_prefix);
    let mut staged = 0usize;
    let mut already = 0usize;
    for (dir, depth) in BOOTSTRAP_DIRS {
        let dst_dir = format!("{}/{}", rootfs_prefix, dir);
        if std::fs::create_dir_all(&dst_dir).is_err() {
            continue;
        }
        for name in BIONIC_NAMES {
            let src_path = format!("{}/{}", rom_lib64, name);
            match std::fs::symlink_metadata(&src_path) {
                Ok(md) if md.is_file() => {}
                _ => continue,
            }
            // Relative symlink target, e.g. from system/lib64/bootstrap:
            // ../../../system/lib64/libc.so
            let mut target = String::with_capacity(depth * 3 + 24);
            for _ in 0..depth {
                target.push_str("../");
            }
            target.push_str("system/lib64/");
            target.push_str(name);
            let dst = format!("{}/{}", dst_dir, name);
            match std::os::unix::fs::symlink(&target, &dst) {
                Ok(()) => staged += 1,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => already += 1,
                Err(_) => {}
            }
        }
    }
    (staged, already)
}

// ============================================================
// 6-Z230: missing DT_NEEDED library staging (host-runtime donor).
// ============================================================

/// 6-Z230: parse the DT_NEEDED library names from an in-memory ELF
/// image (ELF32 + ELF64, little-endian — Android is LE everywhere).
///
/// Uses the PROGRAM-header route (PT_DYNAMIC + DT_STRTAB resolved
/// through PT_LOAD vaddr→offset), NOT section headers: stripped
/// release binaries keep program headers but can lose sections.
///
/// Returns `None` when the image is not an ELF / has no PT_DYNAMIC /
/// the dynamic segment is malformed. An ELF with a PT_DYNAMIC but ZERO
/// DT_NEEDED entries returns Some(vec![]).
///
/// 6-Z268: file-windowed variant — reads ONLY the ELF header, the
/// program-header table, the PT_DYNAMIC segment (≤256 KiB cap) and the
/// DT_STRTAB window (≤1 MiB cap) via seek+read_at, instead of slurping
/// the WHOLE file. `stage_missing_dt_needed` seeds its parse queue with
/// every regular file in {rootfs}/sbin — including the 10–25 MB
/// recovery binary — so the old `fs::read` per queued file turned the
/// pre-execve staging step into tens of MB of heap-churned disk I/O
/// just to learn a handful of library names. Semantics are identical
/// to [`dt_needed_names_from_bytes`] (same header bounds, same
/// PT_LOAD/PT_DYNAMIC walk, same NUL-terminated string extraction).
pub fn dt_needed_names_from_file(path: &str) -> Option<Vec<String>> {
    use std::io::{Read, Seek, SeekFrom};
    const PT_LOAD: u32 = 1;
    const PT_DYNAMIC: u32 = 2;
    const DT_NULL: u64 = 0;
    const DT_NEEDED: u64 = 1;
    const DT_STRTAB: u64 = 5;
    const DT_STRSZ: u64 = 10;

    let mut f = std::fs::File::open(path).ok()?;
    // ELF header (both classes fit in 64 bytes).
    let mut ehdr = [0u8; 64];
    f.read_exact(&mut ehdr).ok()?;
    if ehdr[0..4] != [0x7f, b'E', b'L', b'F'] {
        return None;
    }
    let is64 = ehdr[4] == 2;
    if ehdr[5] != 1 {
        return None; // little-endian only (Android ABIs)
    }
    let (phoff, phentsize, phnum): (usize, usize, usize) = if is64 {
        (
            u64::from_le_bytes(ehdr[0x20..0x28].try_into().ok()?) as usize,
            u16::from_le_bytes(ehdr[0x36..0x38].try_into().ok()?) as usize,
            u16::from_le_bytes(ehdr[0x38..0x3A].try_into().ok()?) as usize,
        )
    } else {
        (
            u32::from_le_bytes(ehdr[0x1C..0x20].try_into().ok()?) as usize,
            u16::from_le_bytes(ehdr[0x2A..0x2C].try_into().ok()?) as usize,
            u16::from_le_bytes(ehdr[0x2C..0x2E].try_into().ok()?) as usize,
        )
    };
    if phnum == 0 || phnum > 64 {
        return None;
    }
    // Program-header table.
    let phdr_bytes = phnum.checked_mul(phentsize)?;
    if phdr_bytes > 64 * 128 {
        return None;
    }
    let mut phdrs = vec![0u8; phdr_bytes];
    f.seek(SeekFrom::Start(phoff as u64)).ok()?;
    f.read_exact(&mut phdrs).ok()?;

    struct Load {
        vaddr: u64,
        off: u64,
        filesz: u64,
    }
    let mut loads: Vec<Load> = Vec::with_capacity(phnum);
    let mut dyn_off: Option<u64> = None;
    let mut dyn_sz: usize = 0;
    for i in 0..phnum {
        let base = i.checked_mul(phentsize)?;
        if base + 40 > phdrs.len() {
            return None;
        }
        if is64 {
            let p_type = u32::from_le_bytes(phdrs[base..base + 4].try_into().ok()?);
            let p_offset = u64::from_le_bytes(phdrs[base + 8..base + 16].try_into().ok()?);
            let p_vaddr = u64::from_le_bytes(phdrs[base + 16..base + 24].try_into().ok()?);
            let p_filesz = u64::from_le_bytes(phdrs[base + 32..base + 40].try_into().ok()?);
            if p_type == PT_LOAD {
                loads.push(Load {
                    vaddr: p_vaddr,
                    off: p_offset,
                    filesz: p_filesz,
                });
            } else if p_type == PT_DYNAMIC {
                dyn_off = Some(p_offset);
                dyn_sz = p_filesz as usize;
            }
        } else {
            let p_type = u32::from_le_bytes(phdrs[base..base + 4].try_into().ok()?);
            let p_offset = u32::from_le_bytes(phdrs[base + 4..base + 8].try_into().ok()?) as u64;
            let p_vaddr = u32::from_le_bytes(phdrs[base + 8..base + 12].try_into().ok()?) as u64;
            let p_filesz = u32::from_le_bytes(phdrs[base + 16..base + 20].try_into().ok()?) as u64;
            if p_type == PT_LOAD {
                loads.push(Load {
                    vaddr: p_vaddr,
                    off: p_offset,
                    filesz: p_filesz,
                });
            } else if p_type == PT_DYNAMIC {
                dyn_off = Some(p_offset);
                dyn_sz = p_filesz as usize;
            }
        }
    }
    let dyn_off = dyn_off?;
    if dyn_sz == 0 {
        // Mirror dt_needed_names_from_bytes: PT_DYNAMIC present but
        // empty → zero DT_NEEDED entries.
        return Some(Vec::new());
    }
    if dyn_sz > 256 * 1024 {
        return None;
    }
    let mut dyn_bytes = vec![0u8; dyn_sz];
    f.seek(SeekFrom::Start(dyn_off)).ok()?;
    f.read_exact(&mut dyn_bytes).ok()?;

    // Walk the dynamic entries for DT_NEEDED values + the STRTAB pointer
    // and size.
    let dyn_ent = if is64 { 16usize } else { 8usize };
    let mut needed: Vec<u64> = Vec::new();
    let mut strtab_vaddr: Option<u64> = None;
    let mut strsz: usize = 0;
    let mut off = 0usize;
    while off + dyn_ent <= dyn_bytes.len() {
        let (tag, val): (u64, u64) = if is64 {
            (
                u64::from_le_bytes(dyn_bytes[off..off + 8].try_into().ok()?),
                u64::from_le_bytes(dyn_bytes[off + 8..off + 16].try_into().ok()?),
            )
        } else {
            (
                u32::from_le_bytes(dyn_bytes[off..off + 4].try_into().ok()?) as u64,
                u32::from_le_bytes(dyn_bytes[off + 4..off + 8].try_into().ok()?) as u64,
            )
        };
        if tag == DT_NULL {
            break;
        }
        match tag {
            DT_NEEDED => needed.push(val),
            DT_STRTAB => strtab_vaddr = Some(val),
            DT_STRSZ => strsz = val as usize,
            _ => {}
        }
        off += dyn_ent;
    }
    let strtab_vaddr = strtab_vaddr?;
    if strsz == 0 || strsz > 1024 * 1024 {
        return None;
    }
    // vaddr → file offset via PT_LOAD.
    let strtab_off = loads
        .iter()
        .find(|l| {
            strtab_vaddr >= l.vaddr
                && l.vaddr
                    .checked_add(l.filesz)
                    .map(|end| strtab_vaddr < end)
                    .unwrap_or(false)
        })
        .map(|l| (l.off + (strtab_vaddr - l.vaddr)) as u64)?;
    let mut strtab = vec![0u8; strsz];
    f.seek(SeekFrom::Start(strtab_off)).ok()?;
    // The strtab may extend past EOF on hand-crafted images — read what
    // is there and pad implicitly (read_exact would fail).
    let mut got = 0usize;
    while got < strsz {
        match f.read(&mut strtab[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(_) => break,
        }
    }
    let names = needed
        .into_iter()
        .filter_map(|v| {
            let v = v as usize;
            if v >= got {
                return None;
            }
            let end = strtab[v..got].iter().position(|&b| b == 0)? + v;
            Some(String::from_utf8_lossy(&strtab[v..end]).into_owned())
        })
        .collect();
    Some(names)
}

/// Returns `None` when the image is not an ELF / has no PT_DYNAMIC /
/// the dynamic segment is malformed. An ELF with a PT_DYNAMIC but ZERO
/// DT_NEEDED entries returns Some(vec![]).
pub fn dt_needed_names_from_bytes(data: &[u8]) -> Option<Vec<String>> {
    const PT_LOAD: u32 = 1;
    const PT_DYNAMIC: u32 = 2;
    const DT_NULL: u64 = 0;
    const DT_NEEDED: u64 = 1;
    const DT_STRTAB: u64 = 5;

    if data.len() < 52 || data[0..4] != [0x7f, b'E', b'L', b'F'] {
        return None;
    }
    let is64 = data[4] == 2;
    let le = data[5] == 1;
    if !le {
        // Android is little-endian on every ABI this tracer serves.
        return None;
    }

    // Header fields: e_phoff, e_phentsize, e_phnum
    let (phoff, phentsize, phnum): (usize, usize, usize) = if is64 {
        (
            u64::from_le_bytes(data[0x20..0x28].try_into().ok()?) as usize,
            u16::from_le_bytes(data[0x36..0x38].try_into().ok()?) as usize,
            u16::from_le_bytes(data[0x38..0x3A].try_into().ok()?) as usize,
        )
    } else {
        (
            u32::from_le_bytes(data[0x1C..0x20].try_into().ok()?) as usize,
            u16::from_le_bytes(data[0x2A..0x2C].try_into().ok()?) as usize,
            u16::from_le_bytes(data[0x2C..0x2E].try_into().ok()?) as usize,
        )
    };
    if phnum == 0 || phnum > 64 {
        return None;
    }

    // Walk the program headers once: collect PT_LOAD ranges and find
    // PT_DYNAMIC's (file offset, filesz).
    struct Load {
        vaddr: u64,
        off: u64,
        filesz: u64,
    }
    let mut loads: Vec<Load> = Vec::with_capacity(phnum);
    let mut dyn_off = None;
    let mut dyn_sz = 0usize;
    for i in 0..phnum {
        let base = phoff.checked_add(i.checked_mul(phentsize)?)?;
        if base + 32 > data.len() {
            return None;
        }
        if is64 {
            let p_type = u32::from_le_bytes(data[base..base + 4].try_into().ok()?);
            let p_offset = u64::from_le_bytes(data[base + 8..base + 16].try_into().ok()?);
            let p_vaddr = u64::from_le_bytes(data[base + 16..base + 24].try_into().ok()?);
            let p_filesz = u64::from_le_bytes(data[base + 32..base + 40].try_into().ok()?);
            if base + 40 > data.len() {
                return None;
            }
            if p_type == PT_LOAD {
                loads.push(Load {
                    vaddr: p_vaddr,
                    off: p_offset,
                    filesz: p_filesz,
                });
            } else if p_type == PT_DYNAMIC {
                dyn_off = Some(p_offset as usize);
                dyn_sz = p_filesz as usize;
            }
        } else {
            let p_type = u32::from_le_bytes(data[base..base + 4].try_into().ok()?);
            let p_offset = u32::from_le_bytes(data[base + 4..base + 8].try_into().ok()?);
            let p_vaddr = u32::from_le_bytes(data[base + 8..base + 12].try_into().ok()?);
            let p_filesz = u32::from_le_bytes(data[base + 16..base + 20].try_into().ok()?);
            if p_type == PT_LOAD {
                loads.push(Load {
                    vaddr: p_vaddr as u64,
                    off: p_offset as u64,
                    filesz: p_filesz as u64,
                });
            } else if p_type == PT_DYNAMIC {
                dyn_off = Some(p_offset as usize);
                dyn_sz = p_filesz as usize;
            }
        }
    }

    let dyn_off = dyn_off?;
    if dyn_off.checked_add(dyn_sz)? > data.len() || dyn_sz == 0 {
        return None;
    }

    // vaddr → file offset via the PT_LOAD ranges.
    let vaddr_to_off = |vaddr: u64| -> Option<usize> {
        for l in &loads {
            if vaddr >= l.vaddr && vaddr < l.vaddr.checked_add(l.filesz)? {
                return Some((l.off + (vaddr - l.vaddr)) as usize);
            }
        }
        None
    };

    // First dynamic pass: DT_STRTAB.
    let entsz = if is64 { 16 } else { 8 };
    let mut strtab_vaddr = None;
    let mut i = 0usize;
    while i + entsz <= dyn_sz {
        let (tag, val) = if is64 {
            (
                u64::from_le_bytes(data[dyn_off + i..dyn_off + i + 8].try_into().ok()?),
                u64::from_le_bytes(data[dyn_off + i + 8..dyn_off + i + 16].try_into().ok()?),
            )
        } else {
            (
                u32::from_le_bytes(data[dyn_off + i..dyn_off + i + 4].try_into().ok()?) as u64,
                u32::from_le_bytes(data[dyn_off + i + 4..dyn_off + i + 8].try_into().ok()?) as u64,
            )
        };
        if tag == DT_NULL {
            break;
        }
        if tag == DT_STRTAB {
            strtab_vaddr = Some(val);
        }
        i += entsz;
    }
    let strtab_vaddr = strtab_vaddr?;
    let strtab_off = vaddr_to_off(strtab_vaddr)?;

    // Second dynamic pass: collect DT_NEEDED names.
    let mut names = Vec::new();
    let mut i = 0usize;
    while i + entsz <= dyn_sz {
        let (tag, val) = if is64 {
            (
                u64::from_le_bytes(data[dyn_off + i..dyn_off + i + 8].try_into().ok()?),
                u64::from_le_bytes(data[dyn_off + i + 8..dyn_off + i + 16].try_into().ok()?),
            )
        } else {
            (
                u32::from_le_bytes(data[dyn_off + i..dyn_off + i + 4].try_into().ok()?) as u64,
                u32::from_le_bytes(data[dyn_off + i + 4..dyn_off + i + 8].try_into().ok()?) as u64,
            )
        };
        if tag == DT_NULL {
            break;
        }
        if tag == DT_NEEDED && val != 0 {
            let start = strtab_off.checked_add(val as usize)?;
            if start >= data.len() {
                continue;
            }
            let end = data[start..]
                .iter()
                .position(|&b| b == 0)
                .map(|p| start + p)
                .unwrap_or(data.len());
            if end > start {
                names.push(String::from_utf8_lossy(&data[start..end]).into_owned());
            }
        }
        i += entsz;
    }
    Some(names)
}

/// 6-Z230: stage HOST-runtime copies of DT_NEEDED libraries the guest
/// ramdisk does not ship, into `{rootfs}/sbin/` (already first on the
/// guest's LD_LIBRARY_PATH).
///
/// ROOT CAUSE (cherry, run 33286314950): TWRP 3.7 builds for devices
/// with ROM-provided recovery dependencies link `libcrypto.so` (FBE/
/// fscrypt decryption support) and EXPECT the ROM's /system/lib64 to
/// provide it. The guest searched LD_LIBRARY_PATH=/sbin:/system/lib:/
/// /system/lib64 — but the twoyi rootfs ships a MINIMAL /system tree
/// (no libcrypto) → "CANNOT LINK EXECUTABLE: library \"libcrypto.so\"
/// not found" → the recovery service exit-1 restart loop → BOOT_FAIL
/// with zero UI. On a real device the ROM's /system provides the lib.
///
/// THE FIX: scan the guest's own /sbin ELF binaries+libraries for
/// DT_NEEDED names, resolve each against the guest's own trees first
/// (sbin, system/lib{,64}, vendor/lib64, odm/lib64), and for names the
/// guest lacks, COPY the host runtime's copy (same ABI only — the
/// candidate's e_machine must match the guest's recovery binary) into
/// {rootfs}/sbin/<name>. COPY (not symlink): a rootfs-internal symlink
/// to an absolute host path would leak the host namespace (§6/§23) and
/// fail the sandbox backstop. Never overwrites an existing file;
/// idempotent.
///
/// Transitive: staged libs are themselves scanned (closure, depth-
/// bounded via the visited set + the file cap). Capped at 48 staged
/// libs and 96 parsed images per run — bounded cost, bounded blast
/// radius.
///
/// Returns (staged_count, missing_names_not_found_anywhere, staged_from_host).
/// `staged_from_host` is true when at least ONE host-runtime library was
/// copied — the signal for the 6-Z236 FORTIFY-shim staging (host bionic
/// libs reference __*_chk symbols the guest libc may not export).
pub fn stage_missing_dt_needed(rootfs_prefix: &str) -> (usize, Vec<String>, bool) {
    use std::os::unix::fs::PermissionsExt;
    const MAX_STAGED: usize = 48;
    const MAX_PARSED: usize = 96;
    const GUEST_LIB_DIRS: [&str; 6] = [
        "sbin",
        "system/lib64",
        "system/lib",
        "vendor/lib64",
        "vendor/lib",
        "odm/lib64",
    ];
    // Host runtime donor dirs, in priority order. Android 12+ keeps
    // libcrypto/libssl inside the conscrypt APEX, so both flat and
    // APEX candidates must be probed.
    let host_lib_dirs: Vec<String> = {
        let mut v = vec![
            "/system/lib64".to_string(),
            "/system/lib".to_string(),
            "/vendor/lib64".to_string(),
            "/vendor/lib".to_string(),
        ];
        if let Ok(rd) = std::fs::read_dir("/apex") {
            let mut apex_dirs: Vec<String> = rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .flat_map(|p| {
                    [
                        p.join("lib64").to_string_lossy().into_owned(),
                        p.join("lib64/boringssl").to_string_lossy().into_owned(),
                        p.join("lib").to_string_lossy().into_owned(),
                    ]
                })
                .collect();
            apex_dirs.sort();
            v.append(&mut apex_dirs);
        }
        v
    };

    // Guest ABI anchor: the recovery binary itself (falls back to the
    // guest's sbin libc). Host candidates whose e_machine differs are
    // rejected — staging a wrong-ABI lib would recreate the 6-Z226
    // "is 64-bit instead of 32-bit" class.
    let recovery_path = {
        let a = format!("{}/sbin/recovery", rootfs_prefix);
        let b = format!("{}/system/bin/recovery", rootfs_prefix);
        if std::path::Path::new(&a).exists() {
            a
        } else {
            b
        }
    };
    let guest_machine = elf_machine(&recovery_path)
        .or_else(|| elf_machine(&format!("{}/sbin/libc.so", rootfs_prefix)));
    let guest_machine = match guest_machine {
        Some(m) if m != 0 => m,
        _ => return (0, vec![], false),
    };

    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut parse_queue: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    let mut staged = 0usize;
    let mut staged_from_host = false;

    // Seed: the recovery binary + every regular ELF in sbin.
    if std::path::Path::new(&recovery_path).exists() {
        parse_queue.push(recovery_path.clone());
    }
    if let Ok(rd) = std::fs::read_dir(format!("{}/sbin", rootfs_prefix)) {
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            // Regular files ONLY: the busybox applet farm is hundreds of
            // symlinks to one staged binary — symlink_metadata().is_file()
            // is false for them and the parse queue stays meaningful.
            match std::fs::symlink_metadata(&p) {
                Ok(md) if md.is_file() => {
                    parse_queue.push(p.to_string_lossy().into_owned());
                }
                _ => {}
            }
        }
    }
    parse_queue.truncate(MAX_PARSED);

    while let Some(path) = parse_queue.pop() {
        if visited.contains(&path) || visited.len() >= MAX_PARSED {
            continue;
        }
        visited.insert(path.clone());
        // 6-Z268: windowed parse — header + phdrs + PT_DYNAMIC + strtab
        // only (see dt_needed_names_from_file). The old full-file
        // fs::read slurped 10–25 MB for the recovery binary alone,
        // per boot, on the pre-execve critical path.
        let names = match dt_needed_names_from_file(&path) {
            Some(n) => n,
            None => continue, // not an ELF or no PT_DYNAMIC
        };
        for name in names {
            if name.is_empty() || !name.starts_with("lib") || !name.ends_with(".so") {
                continue;
            }
            // Resolvable inside the guest already?
            let guest_hit = GUEST_LIB_DIRS.iter().any(|d| {
                std::path::Path::new(&format!("{}/{}/{}", rootfs_prefix, d, name)).exists()
            });
            if guest_hit {
                continue;
            }
            // Host runtime donor, same ABI only.
            let mut staged_path: Option<String> = None;
            for dir in &host_lib_dirs {
                let cand = format!("{}/{}", dir, name);
                if elf_machine(&cand) != Some(guest_machine) {
                    continue; // wrong ABI — would recreate the 6-Z226 class
                }
                let dst = format!("{}/sbin/{}", rootfs_prefix, name);
                match std::fs::copy(&cand, &dst) {
                    Ok(_) => {
                        let _ =
                            std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o755));
                        staged_path = Some(dst);
                        break;
                    }
                    Err(_) => continue,
                }
            }
            match staged_path {
                Some(dst) => {
                    staged += 1;
                    staged_from_host = true;
                    crate::info!(
                        "[KR64] 6-Z230: staged host-runtime lib {} -> {} (guest DT_NEEDED not shipped by ramdisk)",
                        name, dst
                    );
                    if staged < MAX_STAGED {
                        // Transitive closure: the staged lib may itself
                        // need libs the guest lacks.
                        parse_queue.push(dst);
                    }
                }
                None => {
                    if !missing.contains(&name) {
                        missing.push(name.clone());
                    }
                }
            }
        }
    }
    for m in &missing {
        crate::info!(
            "[KR64] 6-Z230: DT_NEEDED {} not shipped by the ramdisk and not found in any host runtime dir — the guest linker will report it if genuinely required",
            m
        );
    }
    (staged, missing, staged_from_host)
}

/// 6-Z236: stage the bionic FORTIFY-compat shim (libbionic_compat.so)
/// next to the host-runtime libraries staged by [`stage_missing_dt_needed`].
///
/// ROOT CAUSE (cherry, run 33306474686): the host runtime's libcrypto.so
/// references the FORTIFY wrapper family (__write_chk, __read_chk, ...)
/// exported by the HOST's bionic libc. The GUEST's own libc.so (an older
/// bionic generation) does not export them → "cannot locate symbol
/// \"__write_chk\" referenced by .../sbin/libcrypto.so" → CANNOT LINK →
/// the recovery service exit-1 restart loop.
///
/// THE FIX: the shim (app/cpp/twoyi_loader/src/bionic_compat.c) is a
/// -nostdlib shared object that implements the FORTIFY family on raw
/// syscalls and exports the symbols. It is staged into {rootfs}/sbin
/// (LD_LIBRARY_PATH[0] proximity) AND {rootfs}/dev (the AOSP-chain
/// /dev slot) and PREPENDED to the recovery service's LD_PRELOAD chain:
/// bionic loads LD_PRELOAD libraries BEFORE DT_NEEDED resolution, so the
/// shim's exports satisfy the host libs' relocations. Inert for guests
/// that don't need it. Never overwrites a guest-shipped
/// libbionic_compat.so (respect guest contents, §10/§22).
///
/// Source resolution is bitness-aware (§9): EM_ARM (40) guests read the
/// RomManager-extracted `libbionic_compat_arm32.so` asset; other ABIs
/// read the jniLibs `libbionic_compat.so` (x86_64 corpus additionally
/// has the `libbionic_compat_i686.so` slot).
/// 6-Z236: the shim SOURCE FILE NAME for a guest ABI. Pure decision core
/// (unit-locked): EM_ARM (40) guests read the RomManager-extracted
/// `libbionic_compat_arm32.so` asset; EM_386 (3) reads the i686 slot;
/// EM_AARCH64 (183) / EM_X86_64 (62) read `libbionic_compat.so`; any other
/// machine → None (never guess an ABI, §9/§22).
fn z236_compat_shim_source_name(guest_machine: u16) -> Option<&'static str> {
    const EM_386: u16 = 3;
    const EM_ARM: u16 = 40;
    const EM_X86_64: u16 = 62;
    const EM_AARCH64: u16 = 183;
    match guest_machine {
        EM_ARM => Some("libbionic_compat_arm32.so"),
        EM_386 => Some("libbionic_compat_i686.so"),
        EM_AARCH64 | EM_X86_64 => Some("libbionic_compat.so"),
        _ => None,
    }
}

fn stage_bionic_compat_shim(cfg: &Config, rootfs_prefix: &str, guest_machine: u16) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let source_name = match z236_compat_shim_source_name(guest_machine) {
        Some(n) => n,
        None => return false, // unknown ABI — never guess
    };
    // Respect guest content: if the ramdisk ships its own shim, leave it.
    let sbin_dst = format!("{}/sbin/libbionic_compat.so", rootfs_prefix);
    if std::path::Path::new(&sbin_dst).exists() {
        crate::info!(
            "[KR64] 6-Z236: guest ships its own sbin/libbionic_compat.so — not staging ours"
        );
        return true;
    }
    let candidates = hook_library_candidates(cfg, source_name);
    for src in &candidates {
        let Ok(content) = std::fs::read(src) else {
            continue;
        };
        if content.len() < 4 || content[0..4] != *b"\x7fELF" {
            continue; // placeholder/asset guard — stage real ELFs only
        }
        if elf_machine_from_bytes(&content) != Some(guest_machine) {
            continue; // wrong-ABI copy — would recreate the 6-Z226 class
        }
        let mut written = false;
        if std::fs::write(&sbin_dst, &content).is_ok() {
            let _ = std::fs::set_permissions(&sbin_dst, std::fs::Permissions::from_mode(0o755));
            crate::info!(
                "[KR64] 6-Z236: staged FORTIFY-compat shim {} ({} bytes from {}) -> {}",
                source_name,
                content.len(),
                src,
                sbin_dst
            );
            written = true;
        }
        // Also cover the /dev slot used by the AOSP-chain prepend.
        let dev_dst = format!("{}/dev/libbionic_compat.so", rootfs_prefix);
        if std::fs::write(&dev_dst, &content).is_ok() {
            let _ = std::fs::set_permissions(&dev_dst, std::fs::Permissions::from_mode(0o755));
        }
        return written;
    }
    crate::info!(
        "[KR64] 6-Z236: no usable {} shim source found in candidates (host libs staged WITHOUT the FORTIFY shim — if the guest linker reports __*_chk failures, extend the shim build)",
        source_name
    );
    false
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
            // Task 6-Z40: set mode 0755 (executable), NOT 0644. The
            // libtwrp_fb_hook.so is loaded via LD_PRELOAD by the 32-bit
            // bionic linker during execve of /sbin/recovery. The linker
            // checks the file's execute permission — if it's 0644 (not
            // executable), the kernel rejects the execve with EACCES
            // before the ptrace syscall-stop fires → exit 127.
            // PRE-FORK DIAG confirmed: hook was mode 0100644, exec=false.
            let _ = std::fs::set_permissions(dst, std::fs::Permissions::from_mode(0o755));
            info!(
                "[KR64] PARENT: wrote {} ({} bytes) {} -> {} (AFTER pivot_root, tmpfs, mode 0755) (Task 6-Z40: executable for LD_PRELOAD)",
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

/// 6-Z225: strip `capabilities` service options from every guest .rc
/// file. Mechanism: init's forked service child runs SetCapsForExec()
/// (init/service.cpp) for services whose .rc declares `capabilities` —
/// it calls capset() AND prctl(PR_CAPBSET_DROP) per bounding cap; in
/// the non-root sandbox some of those still reach the kernel for real
/// (tracer fakes cover capset, but the cap_drop_bound prctl EPERM'd —
/// OrangeFox run 33284467693: "cap_drop_bound(0) failed: Operation not
/// permitted" -> "cannot set capabilities for logd" LOG(FATAL) ->
/// InitFatalReboot soft reboot of the whole guest). Capabilities are
/// UNENFORCEABLE in the sandbox anyway: every service runs as the app's
/// uid, and the tracer fake-successes capset so the child cannot tell —
/// the option is dead weight whose only effect is the FATAL path.
/// Stripping the option at staging (BEFORE init parses the files)
/// removes the entire class: no caps requested -> no SetCapsForExec ->
/// no FATAL.
///
/// Keyword form verified against the actual OrangeFox R12.0 image
/// (init.recovery.logd.rc line 12: `    capabilities SYSLOG
/// AUDIT_CONTROL SETGID SETUID` — keyword, whitespace, cap names; also
/// accepts the `capabilities:` spelling seen in some vendor trees).
/// Only service-option lines are touched; anything else passes through.
/// Idempotent by construction (a stripped file no longer matches).
/// Returns the number of files modified.
pub fn strip_service_capabilities_options(rootfs_prefix: &str) -> usize {
    // Service .rc locations across Android generations/ramdisk layouts.
    // .rc files are matched by extension; size-capped (init.rc files
    // are tiny — the 1 MiB cap only guards against pathological trees).
    const SCAN_DIRS: &[&str] = &[
        "system/etc/init",
        "system/etc/init/hw",
        "system/system_ext/etc/init",
        "vendor/etc/init",
        "odm/etc/init",
        "", // ramdisk root: init.rc, init.recovery*.rc, init.<hw>.rc
    ];
    let base = if rootfs_prefix.is_empty() {
        "/"
    } else {
        rootfs_prefix
    };
    let mut modified = 0usize;
    for dir in SCAN_DIRS {
        let full = if dir.is_empty() {
            base.to_string()
        } else {
            format!("{}/{}", base, dir)
        };
        let rd = match std::fs::read_dir(&full) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for ent in rd.flatten() {
            let name = ent.file_name().to_string_lossy().to_string();
            if !name.ends_with(".rc") {
                continue;
            }
            let path = format!("{}/{}", full, name);
            let content = match std::fs::read_to_string(&path) {
                Ok(c) if c.len() <= 1 << 20 => c, // 1 MiB cap
                _ => continue,
            };
            let mut changed = false;
            let mut out = String::with_capacity(content.len());
            for line in content.lines() {
                let t = line.trim_start();
                let is_caps_opt = t == "capabilities"
                    || t.starts_with("capabilities ")
                    || t.starts_with("capabilities:");
                if is_caps_opt {
                    changed = true;
                    // Drop the line entirely — init tolerates a missing
                    // service option; keeping byte-identical spacing for
                    // everything else avoids parser edge cases.
                    continue;
                }
                out.push_str(line);
                out.push('\n');
            }
            if changed {
                match std::fs::write(&path, &out) {
                    Ok(()) => {
                        modified += 1;
                        info!(
                            "[KR64] PARENT: 6-Z225: stripped capabilities option(s) from {}",
                            path
                        );
                    }
                    Err(e) => warning!(
                        "[KR64] PARENT: 6-Z225: failed to rewrite {} (caps option kept): {}",
                        path,
                        e
                    ),
                }
            }
        }
    }
    modified
}

/// 6-Z272c: enable the image's OWN health HAL service so the recovery's
/// battery reader finds a live `android.hardware.health@2.x::IHealth/
/// default` in hwservicemanager.
///
/// Evidence chain (R12 lavender + TWRP-12.1 sources):
/// * The OF-R12 battery widget drew `battery_android_0.svg` (0%) while
///   the whole-run open trace showed ZERO /sys/class/power_supply reads
///   — the reader never touches sysfs directly.
/// * TWRP-12.1's battery reader is `recovery_utils/battery_utils.cpp`
///   `GetBatteryInfo()` → `get_health_service()` → HIDL hwservicemanager
///   lookup; on a null result it logs "No health implementation is
///   found; assuming defaults" and the capacity stays UNKNOWN (→ 0%).
/// * The image SHIPS `android.hardware.health@2.0/2.1-service` with rcs
///   marked `disabled`, and guest init never starts them (no exec in
///   any run) — hwservicemanager's registry stays empty.
/// * The service's own BatteryMonitor then reads /sys/class/power_supply
///   — which lands in OUR 6-Z272c-pinned tree (host-honest values from
///   the bridge) — so the values it publishes are the bridge's, no
///   fabrication anywhere.
///
/// Patch (same import-time family as 6-Z225 / the fstab sanitizer):
/// drop the `disabled` option line and append an `on late-init` start
/// trigger (TWRP init.rc processes late-init AFTER init, so
/// hwservicemanager — started `on init` — is up when the HAL registers).
/// The 2.1 service is preferred and also serves the 2.0 interface; the
/// 2.0 service is the fallback when only it exists. Idempotent via the
/// marker line.
pub fn enable_image_health_hal(rootfs_prefix: &str) -> usize {
    const MARKER: &str = "# 6-Z272c: health HAL started for the recovery battery reader";
    const CANDIDATES: &[(&str, &str, &str)] = &[
        (
            "system/etc/init/android.hardware.health@2.1-service.rc",
            "system/bin/android.hardware.health@2.1-service",
            "health-hal-2-1",
        ),
        (
            "vendor/etc/init/android.hardware.health@2.1-service.rc",
            "vendor/bin/hw/android.hardware.health@2.1-service",
            "health-hal-2-1",
        ),
        (
            "system/etc/init/android.hardware.health@2.0-service.rc",
            "system/bin/android.hardware.health@2.0-service",
            "health-hal-2-0",
        ),
    ];
    let base = if rootfs_prefix.is_empty() {
        "/"
    } else {
        rootfs_prefix
    };
    for (rc_rel, bin_rel, svc) in CANDIDATES {
        let rc_path = format!("{}/{}", base, rc_rel);
        let bin_path = format!("{}/{}", base, bin_rel);
        // The binary must exist — we start the IMAGE's own HAL, never a
        // synthetic one.
        if !std::path::Path::new(&bin_path).exists() {
            continue;
        }
        let content = match std::fs::read_to_string(&rc_path) {
            Ok(c) if c.len() <= 1 << 20 => c,
            _ => continue,
        };
        if content.contains(MARKER) {
            return 1; // already enabled
        }
        if !content.contains(svc) {
            continue; // not the rc for this service
        }
        let mut out = String::with_capacity(content.len() + 96);
        let mut had_disabled = false;
        for line in content.lines() {
            if line.trim() == "disabled" {
                had_disabled = true;
                continue; // drop the option line, init tolerates it
            }
            out.push_str(line);
            out.push('\n');
        }
        out.push_str(MARKER);
        out.push_str("\non late-init\n    start ");
        out.push_str(svc);
        out.push('\n');
        match std::fs::write(&rc_path, &out) {
            Ok(()) => {
                info!(
                    "[KR64] PARENT: 6-Z272c: enabled image health HAL ({} — disabled dropped: {}, on late-init start appended)",
                    rc_path, had_disabled
                );
                return 1;
            }
            Err(e) => {
                warning!(
                    "[KR64] PARENT: 6-Z272c: failed to rewrite {}: {}",
                    rc_path,
                    e
                );
                return 0;
            }
        }
    }
    0
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
/// Extra `setenv` lines the recovery service needs (6-Z175).
///
/// TWRP's init does NOT pass its own environ to forked services — the
/// service env is built from the init.rc `setenv` options (plus a few
/// globals). Run 33017901360's /proc evidence proved it: recovery's env
/// had LD_PRELOAD (an init.rc setenv line we patch in) but NOT
/// TWOYI_FB_WIDTH/TWOYI_FB_HEIGHT/TWOYI_ROOTFS (kr64's execve environ for
/// init). The twrp_fb_hook therefore fell back to 320x640 geometry and
/// ftruncate'd fb0 to 819200 regardless of the native resolution.
fn twrp_recovery_setenv_lines(fb_width: i32, fb_height: i32, rootfs: &str) -> String {
    let w = if fb_width > 0 { fb_width } else { 320 };
    let h = if fb_height > 0 { fb_height } else { 640 };
    format!(
        "\n    setenv TWOYI_FB_WIDTH {}\n    setenv TWOYI_FB_HEIGHT {}\n    setenv TWOYI_ROOTFS {}",
        w, h, rootfs
    )
}

// ─────────────────────────────────────────────────────────────────────
// 6-Z256: the proactive recovery child's environment — pure core
//
// ROOT CAUSE (cereus OrangeFox R11, run 33323583991): the 20-build
// aarch64 OrangeFox libc null-call class is
//   strlen(getenv("ANDROID_ROOT")) with getenv → NULL
// (crash pc = guest libc.so+0x1e320 = bionic's optimized strlen, x0=0,
// caller LR = recovery+0x45cb4 building a "/boot.img" path string).
// The 6-Z238/6-Z255 env scan named the losing stage DECISIVELY: the
// recovery execve envp had `std-vars 0/4 present [] total_entries=4`
// — and total_entries=4 matches Task 6-Z49's hardcoded envp EXACTLY
// (LD_PRELOAD, LD_LIBRARY_PATH, PATH, TWOYI_ROOTFS). init NEVER forks
// recovery for TWRP-family boots (Task 6-Z49 proactively forks it
// before the ptrace loop starts), so init.rc's `export ANDROID_ROOT
// /system` (verified present at line 34 of the extracted cereus
// ramdisk init.rc, `on init` section) NEVER reaches the recovery
// process. Every recovery that reads a standard Android env var
// without a NULL check crashes at startup — the single biggest
// remaining boot-failure family (20 OrangeFox builds + walleye, and
// the 7 no-decode property_area runs share the fail=property_area
// marker).
//
// THE FIX: give the proactive recovery child the environment a real
// device's init would have exported by the time it starts the
// recovery service (init.rc `on init` — AOSP rootdir/init.rc has
// exported these on EVERY generation from 5.0 through 12):
//   ANDROID_ROOT=/system, ANDROID_DATA=/data, EXTERNAL_STORAGE=/sdcard
// plus ANDROID_BOOTLOGO=1 (exported by many TWRP/OEM init.rcs; minui
// reads it for the splash path). GUEST OWNERSHIP (§10/§22): when the
// guest's own init.rc files export a variable, the GUEST'S value wins
// (we never override a guest-declared value with ours — the reverse
// only for the twoyi-owned LD_*/TWOYI_* keys below).
// ─────────────────────────────────────────────────────────────────────

/// The standard Android environment a real init exports in `on init`,
/// as (name, twoyi-default value) pairs. Applied to the proactive
/// recovery child when the guest's own rc files do not export the name.
pub const ANDROID_STD_ENV_DEFAULTS: [(&str, &str); 4] = [
    ("ANDROID_ROOT", "/system"),
    ("ANDROID_DATA", "/data"),
    ("EXTERNAL_STORAGE", "/sdcard"),
    ("ANDROID_BOOTLOGO", "1"),
];

/// 6-Z256: keys twoyi OWNS in the recovery child's envp — the guest's
/// rc `export` lines for these are never copied over the twoyi-staged
/// values (the virtualization stack depends on them; §22).
fn twoyi_owned_env_key(name: &str) -> bool {
    name == "LD_PRELOAD"
        || name == "LD_LIBRARY_PATH"
        || name == "PATH"
        || name.starts_with("TWOYI_")
}

/// 6-Z256: collect the guest's own init.rc `export <name> <value>`
/// directives (init.rc + its imports + init.recovery.*.rc — the same
/// candidate set the recovery-service patcher scans). These are the
/// environment variables the guest's init would have put into its own
/// environ (and into every service, via export/add_environment) by the
/// time the recovery service starts. Guest-owned truth, never guessed.
///
/// Grammar: `export <name> <value>` — value is the rest of the line,
/// trimmed (init.rc values are single tokens on every real recovery;
/// keeping the full remainder is strictly more faithful than taking
/// the first token). Comment/blank lines skipped. Lines inside service
/// blocks cannot start with `export` in any init generation (service
/// options are name-only), so no section tracking is needed.
fn parse_rc_export_lines(content: &str, out: &mut Vec<(String, String)>) {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("export ") {
            let rest = rest.trim();
            if let Some(sp) = rest.find(' ') {
                let name = &rest[..sp];
                let value = rest[sp + 1..].trim();
                if !name.is_empty() && !value.is_empty() {
                    out.push((name.to_string(), value.to_string()));
                }
            }
            // `export NAME` with no value — skip (not a real export).
        }
    }
}

/// 6-Z256: read the guest rc export set from disk. Best-effort: missing
/// files are skipped silently (the baseline std-env still applies).
fn collect_guest_rc_exports(rootfs_prefix: &str) -> Vec<(String, String)> {
    let mut exports: Vec<(String, String)> = Vec::new();
    let init_rc_path = format!("{}/init.rc", rootfs_prefix);
    let mut candidate_files: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    if seen.insert(init_rc_path.clone()) {
        candidate_files.push(init_rc_path.clone());
    }
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
    // init.recovery.*.rc glob (mirrors the recovery-service patcher step 4;
    // step 3's exact init.recovery.rc is covered by the glob too).
    let dir_path = if rootfs_prefix.is_empty() {
        "/".to_string()
    } else {
        rootfs_prefix.to_string()
    };
    if let Ok(entries) = std::fs::read_dir(&dir_path) {
        let mut glob_matches: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("init.recovery.") && name.ends_with(".rc") {
                    glob_matches.push(entry.path().to_string_lossy().into_owned());
                }
            }
        }
        glob_matches.sort();
        for m in glob_matches {
            if seen.insert(m.clone()) {
                candidate_files.push(m);
            }
        }
    }
    for path in &candidate_files {
        if let Ok(content) = std::fs::read_to_string(path) {
            parse_rc_export_lines(&content, &mut exports);
        }
    }
    exports
}

/// 6-Z256: build the proactive recovery child's envp (pure, unit-locked).
///
/// Layers, in order (first occurrence of a name wins — vec order is the
/// envp order the child sees):
///   1. twoyi's virtualization stack: LD_PRELOAD (compat shim prepended
///      when staged — the same chain the init.rc setenv patch writes),
///      LD_LIBRARY_PATH, PATH, TWOYI_ROOTFS (the fb_hook INPUT bridge).
///   2. guest rc exports (guest-owned truth) for names twoyi doesn't own.
///   3. the standard Android defaults for names still absent.
/// The result is deterministic and duplicate-free (envp duplicate names
/// are undefined behavior per execve(3) — glibc/bionic use the FIRST).
fn build_recovery_service_envp(
    rootfs: &str,
    compat_shim_staged: bool,
    rc_exports: &[(String, String)],
) -> Vec<String> {
    let ld_preload = if compat_shim_staged {
        format!(
            "{}/sbin/libbionic_compat.so:{}/sbin/libtwrp_fb_hook.so",
            rootfs, rootfs
        )
    } else {
        format!("{}/sbin/libtwrp_fb_hook.so", rootfs)
    };
    let ld_library_path = format!(
        "{}/sbin:{}/system/lib:{}/system/lib64",
        rootfs, rootfs, rootfs
    );
    let mut entries: Vec<String> = vec![
        format!("LD_PRELOAD={}", ld_preload),
        format!("LD_LIBRARY_PATH={}", ld_library_path),
        "PATH=/sbin:/system/bin".to_string(),
        format!("TWOYI_ROOTFS={}", rootfs),
    ];
    let has_key = |entries: &Vec<String>, name: &str| -> bool {
        let prefix = format!("{}=", name);
        entries.iter().any(|e| e.starts_with(&prefix))
    };
    // Layer 2: guest rc exports (skip twoyi-owned keys).
    for (name, value) in rc_exports {
        if twoyi_owned_env_key(name) {
            continue;
        }
        if has_key(&entries, name) {
            continue;
        }
        entries.push(format!("{}={}", name, value));
    }
    // Layer 3: standard Android defaults (only for still-absent names).
    for (name, default_value) in ANDROID_STD_ENV_DEFAULTS.iter() {
        if has_key(&entries, name) {
            continue;
        }
        entries.push(format!("{}={}", name, default_value));
    }
    entries
}

#[cfg(test)]
fn patch_twrp_init_rc_recovery_service(content: &str) -> Option<String> {
    patch_twrp_init_rc_recovery_service_with_env(content, "")
}

#[cfg_attr(not(test), allow(dead_code))]
fn patch_twrp_init_rc_recovery_service_with_env(
    content: &str,
    extra_setenv: &str,
) -> Option<String> {
    patch_twrp_init_rc_recovery_service_full(
        content,
        extra_setenv,
        "    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so",
        "",
    )
}

/// 6-Z220: mode-aware recovery-service patcher.
///
/// `preload_line`  — the full `    setenv LD_PRELOAD <chain>` option line
///                   to inject (legacy /sbin fb-hook for 32-bit TWRP, or
///                   the full AOSP chain for native-arch layouts).
/// `extra_options` — additional service option lines (e.g. `\n    stdio_to_kmsg`).
pub fn patch_twrp_init_rc_recovery_service_full(
    content: &str,
    extra_setenv: &str,
    preload_line: &str,
    extra_options: &str,
) -> Option<String> {
    // Find the "service recovery" line. It may be "service recovery /sbin/recovery"
    // or "service recovery /sbin/recovery\r" (CRLF). We match the prefix
    // "service recovery " at the start of a line.
    //
    // Task 6-Z76: collect the lines into a Vec ONCE so we can LOOK AHEAD
    // past the `service recovery` line — the recovery service block
    // sometimes ALREADY declares `seclabel u:r:recovery:s0`, and blindly
    // appending ours produced a DUPLICATED option line (init accepts a
    // duplicate seclabel — last one wins — but it is sloppy and broke the
    // insert-before-existing-options test).
    let all_lines: Vec<&str> = content.lines().collect();
    let mut result = String::with_capacity(content.len() + 96);
    let mut found = false;
    for (idx, line) in all_lines.iter().enumerate() {
        let trimmed = line.trim_start();
        // 6-Z220: INSIDE the recovery service block, drop any PREVIOUS
        // `setenv LD_PRELOAD` option (from an earlier boot's patch with a
        // different chain) and any previous stdio_to_kmsg — we re-emit
        // exactly one of each so a boot-mode flip can never leave BOTH
        // chains in the block (init's last-setenv-wins would hide which
        // chain actually applied; duplicate stdio_to_kmsg is sloppy).
        if found
            && !trimmed.is_empty()
            && (trimmed.starts_with("setenv LD_PRELOAD ")
                || trimmed.starts_with("setenv  LD_PRELOAD ")
                || trimmed == "stdio_to_kmsg")
        {
            continue;
        }
        result.push_str(line);
        // Check if this line starts the recovery service definition.
        // We check the trimmed start to handle leading whitespace (shouldn't
        // happen for service definitions, but be defensive).
        if !found && trimmed.starts_with("service recovery ") {
            // This is the recovery service line. Insert the setenv directive
            // as the next line (indented with 4 spaces, matching init.rc
            // convention for service options).
            //
            // NOTE: do NOT add `seclabel u:r:recovery:s0` here — see the
            // function-level doc comment above (Task ID 24). The host
            // kernel's SELinux policy doesn't have the recovery context,
            // so setexeccon returns EINVAL and aborts the service start.
            //
            // Task 6-Z76: scan THIS service's option block (the indented
            // option lines + blank lines that follow, terminated by the next
            // section header at column 0, e.g. `service ...` / `on ...`) for
            // an existing seclabel declaration. Only append ours when the
            // block doesn't already have one.
            let block_has_seclabel = all_lines[idx + 1..]
                .iter()
                .take_while(|l| l.starts_with(' ') || l.starts_with('\t') || l.trim().is_empty())
                .any(|l| l.trim_start().starts_with("seclabel"));
            result.push('\n');
            result.push_str(preload_line);
            // 6-Z175: the native-resolution + rootfs env lines (see
            // twrp_recovery_setenv_lines — TWRP init does not inherit
            // kr64's environ into services).
            result.push_str(extra_setenv);
            // Task 6-Z36: add LD_LIBRARY_PATH=/sbin so the 32-bit TWRP linker
            // searches /sbin/ for the recovery binary's 23 NEEDED libraries
            // (libaosprecovery.so, libblkid.so, libminuitwrp.so, etc.). Without
            // this, the linker only searches /system/lib/ (default) → the
            // libraries aren't found → the recovery binary exits 127 after
            // 1163 iterations (lazy load failure on first call to a missing lib).
            // 6-Z220: legacy 32-bit chain ONLY — for the native-arch AOSP
            // chain the service inherits init's 12-dir LD_LIBRARY_PATH
            // (which includes /dev, /apex/*, /system/lib64, ...); hard-
            // coding the 32-bit trio here would OVERRIDE that inherited
            // value and break native library resolution.
            if preload_line.starts_with("    setenv LD_PRELOAD /sbin/") {
                result.push_str("\n    setenv LD_LIBRARY_PATH /sbin:/system/lib:/system/lib64");
            }
            // 6-Z220: extra service options (stdio_to_kmsg when the guest
            // init supports it) — makes the recovery service's stderr
            // (hook + linker + glog diagnostics) visible in kmsg.
            result.push_str(extra_options);
            // Task 6-Z29: NOW adding seclabel u:r:recovery:s0 AGAIN. The
            // setexeccon EINVAL is handled by a NEW ptrace_emu fake: writes
            // to /proc/self/attr/exec (setexeccon's implementation) are
            // intercepted at the EXIT + faked to return success. This bypasses
            // both selabel_lookup (seclabel provides the context directly) AND
            // setexeccon (ptrace fakes the write return). init can then fork
            // the recovery service → it opens fb0 → TWRP renders.
            // (Task 6-Z76: skipped when the block already declares one.)
            if !block_has_seclabel {
                result.push_str("\n    seclabel u:r:recovery:s0");
            }
            found = true;
        }
        // Preserve the original line ending (lines() strips \n, so we add
        // it back). For the last line (no trailing \n), we don't add one.
        if idx + 1 < all_lines.len() {
            result.push('\n');
        }
    }
    if found {
        Some(result)
    } else {
        None
    }
}

/// 6-Z220: the legacy TWRP recovery-service preload option line. Kept as
/// a named constant for tests and for the None-branch of the patcher; the
/// idempotence check now keys on the CURRENT chain (6-Z220) rather than
/// this legacy marker, so a boot-mode flip re-patches instead of skipping.
#[cfg(test)]
#[allow(dead_code)]
const TWRP_LD_PRELOAD_PATCH_MARKER: &str = "    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so";

/// 6-Z220: the recovery-service preload chain for AOSP-layout (native
/// arch) boots, without the "LD_PRELOAD=" prefix. The chain matches
/// AOSP_LD_PRELOAD_ENV (same order — FB hook BEFORE the shlib, 6-Z218a).
/// Init.rc `setenv LD_PRELOAD <chain>` injects it directly into the
/// forked recovery service so the service does not depend on inheriting
/// kr64's exec env through init (stock AOSP init DOES inherit environ,
/// but any init variant or rc-level `setenv`/`unsetenv` churn in a
/// vendor tree can silently drop it — the service MUST carry the full
/// virtualization stack explicitly, §22).
pub const AOSP_SERVICE_PRELOAD_CHAIN: &str =
    "/dev/libgetpid_hook.so:/dev/libtwrp_fb_hook.so:/dev/libtwoyi_loader_shlib.so";

/// 6-Z220: returns true when the init binary at `{rootfs}/<path>`
/// contains `needle` anywhere in its raw bytes. Used as a SAFE binary
/// indicator for init.rc feature options (§10 pattern — probe the
/// actual guest binary, never guess from a version string). Currently
/// used for `stdio_to_kmsg` (Android 11+ service option): older init
/// parsers treat unknown service options as parse errors and would
/// DROP the recovery service, so we only emit the option when the
/// guest's own init binary literally contains the option name.
fn binary_contains_string(path: &str, needle: &str) -> bool {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    let needle = needle.as_bytes();
    if needle.is_empty() {
        return true;
    }
    // Streaming overlap search over fixed-size chunks (init binaries are
    // a few MB; keep memory bounded).
    let mut tail: Vec<u8> = Vec::with_capacity(needle.len() - 1);
    let mut buf = [0u8; 65536];
    loop {
        match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let window = tail
                    .iter()
                    .chain(buf[..n].iter())
                    .cloned()
                    .collect::<Vec<u8>>();
                if window.windows(needle.len()).any(|w| w == needle) {
                    return true;
                }
                tail = window[window.len().saturating_sub(needle.len() - 1)..].to_vec();
            }
            Err(_) => return false,
        }
    }
    false
}

/// 6-Z218a: LD_PRELOAD for non-TWRP (AOSP-layout) boots. The FB hook
/// MUST precede libtwoyi_loader_shlib.so: bionic resolves PLT entries
/// against LD_PRELOAD libs in order, and the shlib exports
/// open/openat/__open_2/__openat_2/close/ioctl — with the shlib first,
/// libminuitwrp's PLT entries resolved to the shlib, the FB hook was
/// fully shadowed and gr_init() crash-looped (run 33269270911: 26
/// gr_fb_width SIGSEGVs). All fb-hook hooks chain via
/// dlsym(RTLD_NEXT), so shlib behavior for every other fd is kept.
pub const AOSP_LD_PRELOAD_ENV: &str =
    "LD_PRELOAD=/dev/libgetpid_hook.so:/dev/libtwrp_fb_hook.so:/dev/libtwoyi_loader_shlib.so";

#[cfg(test)]
fn assert_aosp_preload_order() {
    // The FB hook must appear BEFORE the shlib so its open/ioctl
    // interposition wins PLT resolution (6-Z218a).
    let fb = AOSP_LD_PRELOAD_ENV.find("libtwrp_fb_hook.so");
    let shlib = AOSP_LD_PRELOAD_ENV.find("libtwoyi_loader_shlib.so");
    assert!(fb.is_some() && shlib.is_some(), "both hooks must be set");
    assert!(
        fb.unwrap() < shlib.unwrap(),
        "libtwrp_fb_hook.so must precede libtwoyi_loader_shlib.so in LD_PRELOAD (bionic resolves PLT entries in preload order)"
    );
    // The getpid hook stays first (cheap, affects only getpid).
    assert!(
        AOSP_LD_PRELOAD_ENV.starts_with("LD_PRELOAD=/dev/libgetpid_hook.so:"),
        "getpid hook must remain first"
    );
}

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
fn patch_twrp_init_rc_recovery_service_in_rootfs(
    rootfs_prefix: &str,
    fb_width: i32,
    fb_height: i32,
    service_preload_chain: Option<&str>,
) {
    let extra_setenv = twrp_recovery_setenv_lines(fb_width, fb_height, rootfs_prefix);
    // 6-Z220: the LD_PRELOAD chain written into the recovery service.
    //   None                      → legacy TWRP 32-bit hook (unchanged).
    //   Some(chain)               → full AOSP virtualization stack for
    //                               native-arch AOSP-layout recoveries.
    let preload_line = match service_preload_chain {
        Some(chain) => format!("    setenv LD_PRELOAD {}", chain),
        None => "    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so".to_string(),
    };
    // 6-Z220: route the recovery service's stdio to /dev/__kmsg__ so the
    // hook libraries' diagnostics (fd 2) and any guest linker/glog output
    // are CAPTURED in the run artifacts. OrangeFox run 33271278540: the
    // recovery service crash-looped at gr_fb_width() with ALL of its
    // stderr invisible (init redirects non-console service stdio to
    // /dev/null) — every [twrp_fb_hook] ioctl diagnostic was lost and the
    // root cause had to be inferred from maps alone. The option exists
    // since Android 11; older init parsers reject unknown service options
    // and would DROP the recovery service, so emit it ONLY when the
    // guest's own init binary contains the option name (binary indicator,
    // never a version guess — §10/§22).
    let stdio_line = if binary_contains_string(
        &format!("{}/system/bin/init", rootfs_prefix),
        "stdio_to_kmsg",
    ) {
        "\n    stdio_to_kmsg"
    } else {
        ""
    };
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
    // CURRENT patch marker (the exact preload chain we would write, plus
    // stdio_to_kmsg when applicable), we're done (a previous boot patched
    // it). We check ALL candidates (not just the first) because the marker
    // may have been written to a non-init.rc file by an earlier boot.
    // 6-Z220: a DIFFERENT chain (e.g. the legacy /sbin marker on a
    // boot-mode flip) does NOT satisfy idempotence — the stale setenv is
    // dropped by the patcher below and re-emitted with the current chain.
    // -----------------------------------------------------------------
    for path in &candidate_files {
        if let Ok(content) = std::fs::read_to_string(path) {
            if content.contains(&preload_line)
                && (stdio_line.is_empty() || content.contains("stdio_to_kmsg"))
            {
                info!(
                    "[KR64] PARENT: {} already patched with {:?} for recovery service (idempotent skip)",
                    path,
                    preload_line.trim()
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
        if let Some(patched) = patch_twrp_init_rc_recovery_service_full(
            &content,
            &extra_setenv,
            &preload_line,
            stdio_line,
        ) {
            match std::fs::write(path, &patched) {
                Ok(()) => info!(
                    "[KR64] PARENT: patched {} — added {:?} + TWOYI_FB_WIDTH/HEIGHT/ROOTFS env (native res {}x{}){} to recovery service",
                    path,
                    preload_line.trim(),
                    if fb_width > 0 { fb_width } else { 320 },
                    if fb_height > 0 { fb_height } else { 640 },
                    if stdio_line.is_empty() { "" } else { " + stdio_to_kmsg" }
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
    // 6-Z220: the authored service uses the CURRENT chain (legacy /sbin
    // fb-hook or the full AOSP chain) and, when the guest init supports
    // it, stdio_to_kmsg. For the AOSP chain the service binary is
    // /system/bin/recovery (the AOSP-layout path), not /sbin/recovery.
    let (svc_binary, svc_preload_line) = match service_preload_chain {
        Some(chain) => (
            "/system/bin/recovery",
            format!("    setenv LD_PRELOAD {}", chain),
        ),
        None => (
            "/sbin/recovery",
            "    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so".to_string(),
        ),
    };
    let twoyi_rc_content = format!(
        concat!(
            "service recovery {}\n",
            "{}{}\n",
            "{}\n    seclabel u:r:recovery:s0\n",
        ),
        svc_binary, svc_preload_line, extra_setenv, stdio_line
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

/// DELETE `/property_contexts` entirely so init's `open()` returns
/// `-ENOENT` → the caller (iterating the SEPolicy context table at
/// `0x80ce270`) skips this context file → the parser is never invoked →
/// no corrupted-context SIGSEGV (Task 6-O; supersedes 6-N's empty-file
/// approach — see DISPATCHER-FINAL-8).
///
/// # Root cause (DISPATCHER-FINAL-3 + 6-K + 6-M + 6-L + 6-N analysis)
///
/// After 6-J's pause-loop fix (commit a171d62), the guest progressed from
/// iteration 220 to iteration 338 before hitting a NEW SIGSEGV at
/// `rip=0x80a0b9e, si_addr=0x74616433` (ASCII "3dat"). Disassembly
/// (DISPATCHER-FINAL-3) showed the crash is inside `init` at `0x080a0aa0`,
/// in a config-file parser loop: `fstat` + `fgets` calls followed by
/// `movl $0x0, 0x4(%edx)` where `edx = 0x74616433` (a garbage pointer
/// built from bytes of file content). The `open()` immediately before the
/// parsing loop opens `/property_contexts`.
///
/// The `/property_contexts` file (shipped in the TWRP ramdisk) has, on its
/// FIRST LINE, a C preprocessor directive:
///
/// ```text
/// #line 1 "external/sepolicy/property_contexts"
/// ```
///
/// This is a leftover from the AOSP build process — it's emitted by the
/// `m4`/assembler-style toolchain that produces `property_contexts` from
/// a template in `external/sepolicy/`. AOSP's `init` reads
/// `property_contexts` with its OWN parser (NOT a C preprocessor), so it
/// doesn't understand the `#line` directive: it tries to parse the
/// directive as a property-context mapping, misreads the bytes of the
/// path string `"external/sepolicy/property_contexts"` (per the dispatcher's
/// hypothesis, bytes from "data" in the path are misinterpreted as a
/// pointer), and the resulting "pointer" stored into `edx` is `0x74616433`
/// (ASCII "3dat" in little-endian: `0x33 0x64 0x61 0x74`). The subsequent
/// `movl $0x0, 0x4(%edx)` dereferences this garbage pointer and triggers
/// the SIGSEGV.
///
/// # The whack-a-mole (DISPATCHER-FINAL-7)
///
/// 6-L (fc3bde5) removed just the `#line` directive — the crash persisted
/// (the parser's context field at offset `0x14` is corrupted even when
/// reading the real property-context entries that follow). 6-M (dada9c6)
/// NOPed the crash instruction at `0x80a0b9e` (7 bytes:
/// `c7 42 04 00 00 00 00` → `90 90 90 90 90 90 90`) — but the crash MOVED
/// to `0x80a0bd8` (iteration 342, was 338) with the SAME garbage pointer
/// `0x74616433`. The crash instruction at `0x80a0bd8` is
/// `mov 0x4(%ecx),%eax` — a READ from `[ecx+4]` where `ecx` is loaded from
/// `ctx->field_0x14` (the SAME corrupted context field). The parser has
/// MULTIPLE instructions that dereference the garbage `edx`/`ecx` from
/// the corrupted `ctx->field_at_0x14`. NOPing each one individually is
/// whack-a-mole — not sustainable.
///
/// # Why 6-N's "empty to a comment line" didn't work (DISPATCHER-FINAL-8)
///
/// 6-N (aaedbe6) replaced the ENTIRE `/property_contexts` file with a
/// single comment line. The hypothesis was: the parser's `fgets` loop
/// reads the comment line → skips it (init's parser treats `#`-prefixed
/// lines as comments) → next `fgets` returns NULL → parser exits cleanly
/// WITHOUT hitting the corrupted-context code path. 407 tests passed.
///
/// UI E2E on aaedbe6 CONFIRMED the file was emptied (the log says "was
/// 2920 bytes, now 173 bytes"). BUT the crash PERSISTED: same
/// `rip=0x80a0bd8`, `si_addr=0x74616433`, iteration 342. This means the
/// context field `0x14` is corrupted BEFORE the parser reads the file
/// content. Even with a single comment line, `fgets` returns that line →
/// the parser processes it → tries to read `ctx->field_0x14->field_at_4`
/// (garbage pointer `0x74616433`) → SIGSEGV. Emptying the file does NOT
/// help because the corruption is upstream, in the caller's context setup.
///
/// # The fix (Task 6-O)
///
/// This is a DATA fix: DELETE the `/property_contexts` file ENTIRELY. If
/// the file does not exist, init's `open()` returns `-ENOENT`. The caller
/// (which iterates the function pointer table at `0x80ce270`) is expected
/// to handle `-ENOENT` gracefully by skipping that context file. This
/// avoids the parser being invoked AT ALL → no corrupted-context crash.
///
/// `init` tolerates missing property contexts — it's not fatal for TWRP
/// boot (it just means SELinux property labeling won't work, which is OK
/// in the sandboxed environment).
///
/// Replaces 6-N's "empty to a comment line" (which still triggered the
/// parser on the comment line and crashed). 6-M's NOP patch (on the init
/// binary) is kept in the binary as belt-and-suspenders in case any
/// context file path slips through un-deleted; we don't need to undo it
/// here.
///
/// # Idempotence
///
/// If the file is already missing (e.g. a previous boot already deleted
/// it), this function is a no-op and logs an idempotent skip.
///
/// # Non-fatal
///
/// If the deletion fails (e.g. permission denied), we log and continue —
/// the guest may still crash later if init actually opens this file, but
/// the condition is surfaced in the log for diagnosis.
fn patch_property_contexts_delete(rootfs_prefix: &str) {
    let path = format!("{}/property_contexts", rootfs_prefix);
    // Step 1: check existence. If the file is already gone (a previous
    // boot deleted it), this is an idempotent no-op.
    if !std::path::Path::new(&path).exists() {
        info!(
            "[KR64] PARENT: /property_contexts already absent at {} (idempotent skip — caller will get -ENOENT on open)",
            path
        );
        return;
    }
    // Step 2: read the existing file's size + first line for diagnostic
    // logging only. We do NOT need the content for the patch itself (the
    // fix is to DELETE, not to rewrite) — but logging the prior size
    // helps confirm the deletion actually happened (matches 6-N's
    // "was N bytes, now 0 bytes" log shape for grep parity).
    let (prior_len, prior_first_line): (usize, String) = match std::fs::read_to_string(&path) {
        Ok(c) => (c.len(), c.lines().next().unwrap_or("").to_string()),
        // read_to_string can fail on non-UTF8 content; the file is
        // still DELETABLE in that case (remove_file works on bytes).
        Err(_) => (0, String::new()),
    };
    // Step 3: DELETE the file entirely. If init's open() returns -ENOENT,
    // the caller (iterating the SEPolicy context table at 0x80ce270)
    // skips this context file → the parser is never invoked → no crash.
    match std::fs::remove_file(&path) {
        Ok(()) => info!(
            "[KR64] PARENT: DELETED /property_contexts — was {} bytes (first line: {:?}), now absent. init's open() will return -ENOENT → caller skips this context file → parser never invoked → no corrupted-context crash (DISPATCHER-FINAL-8). SELinux property labeling disabled in sandbox (non-fatal for TWRP boot).",
            prior_len, prior_first_line
        ),
        Err(e) => warning!(
            "[KR64] PARENT: failed to DELETE /property_contexts at {}: {} (init may open it and SIGSEGV at rip=0x80a0b9e/0x80a0bd8 with si_addr=0x74616433 — corrupted context field at 0x14)",
            path, e
        ),
    }
}

/// DELETE `/file_contexts` (+ `/file_contexts.homedirs` + `/file_contexts.local`)
/// entirely, mirroring `patch_property_contexts_delete` (6-O).
///
/// # Root cause (Task 6-V disassembly + DISPATCHER-SESSION-3-UPDATE-3)
///
/// After 6-T's stat64 path-translation fix (commit 3c184e2) eliminated the
/// stat64-ENOENT polling loop, the recovery now reaches the `/file_contexts`
/// parser (opened at post-execve syscall #77). `/file_contexts` has the SAME
/// `#line 1 "external/sepolicy/file_contexts"` directive on line 1 that
/// crashed `/property_contexts` (6-L/6-M/6-N/6-O). The recovery's parser
/// processes the file, the `#line` directive (or a path like `/init` in the
/// content) overflows a fixed-size buffer, and the corrupted bytes propagate
/// into a `std::string` object pointer. The next `std::string::string(char
/// const*, ...)` constructor (at rip=0x8052f65, file vaddr 0xaf65) is called
/// with `this = 0x696e692f` (= "/ini" in ASCII — the first 4 bytes of
/// "/init"), dereferences the bad pointer → SIGSEGV (si_code=1 MAPERR,
/// exit code -11). Observed on the 476446f UI E2E run 32194676789 (826
/// iterations, crash at post-execve #356 after the recovery read
/// /file_contexts + variants 10s earlier).
///
/// # The fix
///
/// Same DATA fix as 6-O: DELETE the files ENTIRELY so the recovery's
/// `open()` returns `-ENOENT` → the caller skips the context file → the
/// parser is never invoked → no buffer overflow → no corrupted `this`
/// pointer → no SIGSEGV. The recovery tolerates missing file contexts
/// (SELinux file labeling disabled in sandbox — non-fatal for TWRP boot;
/// the KVM E2E path has real SELinux + parses /file_contexts without
/// crashing because the real kernel provides valid SELinux contexts).
///
/// Deletes all 3 variants the recovery opens (verified in the 476446f
/// logcat: /file_contexts + /file_contexts.homedirs + /file_contexts.local).
/// Idempotent (no-op if already missing). Non-fatal on failure (logs + continues).
fn patch_file_contexts_delete(rootfs_prefix: &str) {
    // Task 6-Z7: REPLACE /file_contexts with a MINIMAL version (no #line
    // directive) instead of DELETING it. The deletion (6-V) fixed the
    // SIGSEGV (the #line directive crashed the parser), but the ABSENCE
    // prevents init from looking up the SELinux context for /sbin/recovery
    // → "could not get context while starting 'recovery'" → init can't
    // start the recovery service → no TWRP UI (verified on 8857048 UI E2E
    // run 32220797873).
    //
    // The MINIMAL file_contexts provides the essential entries init needs
    // (especially `/sbin(/.*)? u:object_r:rootfs:s0` for the recovery
    // service) WITHOUT the `#line 1 "external/sepolicy/file_contexts"`
    // directive that crashes the parser. The parser processes the minimal
    // file successfully → init gets the context for /sbin/recovery → init
    // starts the recovery service → TWRP UI boots.
    //
    // The .homedirs + .local variants are still DELETED (they don't contain
    // essential entries + their absence is harmless).
    let minimal_file_contexts = "# Minimal file_contexts (Task 6-Z7: no #line directive)
/sbin(/.*)?             u:object_r:rootfs:s0
/sbin/recovery          u:object_r:rootfs:s0
/init                   u:object_r:rootfs:s0
/charger                u:object_r:rootfs:s0
/file_contexts          u:object_r:rootfs:s0
/property_contexts      u:object_r:rootfs:s0
/sepolicy               u:object_r:rootfs:s0
/system(/.*)?           u:object_r:system_file:s0
/vendor(/.*)?           u:object_r:system_file:s0
/data(/.*)?             u:object_r:system_data_file:s0
/dev(/.*)?              u:object_r:device:s0
/proc(/.*)?             u:object_r:proc:s0
/sys(/.*)?              u:object_r:sysfs:s0
";

    // Replace /file_contexts with the minimal version.
    let fc_path = format!("{}/file_contexts", rootfs_prefix);
    let (prior_len, prior_first_line): (usize, String) = match std::fs::read_to_string(&fc_path) {
        Ok(c) => (c.len(), c.lines().next().unwrap_or("").to_string()),
        Err(_) => (0, String::new()),
    };
    match std::fs::write(&fc_path, minimal_file_contexts) {
        Ok(()) => info!(
            "[KR64] PARENT: REPLACED /file_contexts with minimal version (was {} bytes, first line: {:?}, now {} bytes — no #line directive, provides /sbin context for recovery service start). Task 6-Z7: fixes 'could not get context while starting recovery' (8857048 UI E2E).",
            prior_len, prior_first_line, minimal_file_contexts.len()
        ),
        Err(e) => warning!(
            "[KR64] PARENT: failed to REPLACE /file_contexts at {}: {} (recovery may SIGSEGV at rip=0x8052f65 if the original #line directive is parsed, OR fail to start the recovery service if the file is absent)",
            fc_path, e
        ),
    }

    // Delete the .homedirs + .local variants (not essential).
    for fname in &["file_contexts.homedirs", "file_contexts.local"] {
        let path = format!("{}/{}", rootfs_prefix, fname);
        if !std::path::Path::new(&path).exists() {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => info!(
                "[KR64] PARENT: DELETED /{} (not essential — no #line directive issue, but not needed for TWRP boot). Task 6-Z7.",
                fname
            ),
            Err(e) => warning!(
                "[KR64] PARENT: failed to DELETE /{} at {}: {}",
                fname, path, e
            ),
        }
    }
}

/// Pre-create a "fake sysfs" in the guest rootfs (`{rootfs}/sys/`) so the
/// guest init's `open("/sys/...")` calls succeed against an empty, writable
/// tree instead of returning `-EACCES` against the host's real kernel sysfs.
///
/// # Root cause (Task 6-P, dispatcher's analysis of 56a5bd3 UI E2E)
///
/// After 6-O's `property_contexts` deletion (56a5bd3), the guest now
/// progresses to ptrace iteration **3059** (was 342 — ~9× improvement) before
/// exiting with code 1. The exit happens right after:
///
/// ```text
/// open("/sys/class")            -> -13 (-EACCES)
/// open("/sys/fs/selinux/enforce") -> -13 (-EACCES)   (or -ENOENT)
/// open("/sys/fs/selinux/load")    -> -13 (-EACCES)   (or -ENOENT)
/// ```
///
/// **Why EACCES:** `/sys` is the host's real kernel sysfs (a `selinuxfs`
/// and `sysfs` superblock owned by root). The guest runs as `untrusted_app`
/// (no `CAP_SYS_ADMIN`, no `CAP_DAC_OVERRIDE`), so `open("/sys/class")`
/// fails with `EACCES` on the directory itself and on most of its children.
/// `selinux_android_load_policy()` ALSO opens
/// `/sys/fs/selinux/{load,enforce,booleans}` — those don't exist in the
/// sandbox at all (no real selinuxfs was mounted; the
/// `mount("selinuxfs", ...)` syscall returns negative).
///
/// The EXIT handler IS correctly writing 0 for `mount`/`mknod`/`chmod`
/// (confirmed by readback logs) — those fixes from prior tasks work. The
/// `-EACCES` on `/sys/class` is a NEW, different blocker.
///
/// # The fix — fake sysfs + path translation
///
/// This function pre-creates in the rootfs:
///   * `{rootfs}/sys/class/`                 (empty directory, mode 0755)
///   * `{rootfs}/sys/fs/`                    (empty directory, mode 0755)
///   * `{rootfs}/sys/fs/selinux/`            (empty directory, mode 0755)
///   * `{rootfs}/sys/fs/selinux/enforce`     (empty file, mode 0666 — content "0")
///   * `{rootfs}/sys/fs/selinux/load`        (empty file, mode 0666)
///
/// Companion change in `ptrace_emu::translate_path`: `/sys/*` opens are
/// now redirected to `{rootfs}/sys/*` (previously they passed through to
/// the host's real sysfs — the same fix that was applied to `/dev/*` in
/// commit 9154e59 / the find_property binary patch removal). Without the
/// translation, pre-creating the rootfs files alone would be useless —
/// the guest's `open("/sys/class")` would still hit the host's `/sys/class`
/// and get `EACCES`.
///
/// # Effect on init
///
/// init's `open("/sys/class")` now returns a valid fd (or `-ENOENT` if the
/// directory wasn't pre-created for some reason — much better than `-EACCES`).
/// If init then `readdir()`s it, it sees an EMPTY directory — no sysfs
/// devices — and proceeds (init treats "no devices" as "no work to do").
/// init's `open("/sys/fs/selinux/enforce")` returns a valid fd → `read()`
/// returns 0 bytes (or the literal "0" we wrote) → init treats SELinux as
/// permissive. init's `open("/sys/fs/selinux/load")` returns a valid fd →
/// init's `write()` of the policy blob is silently dropped (the file is a
/// regular empty file, the write extends it but no kernel policy is actually
/// loaded — non-fatal for TWRP boot in the sandbox).
///
/// # Idempotence
///
/// `create_dir_all` + `OpenOptions::create(true).truncate(false)` mean a
/// prior call's pre-creation is preserved (we do NOT clobber the `enforce`
/// content with "0" on every call — only on first creation). Repeated calls
/// are no-ops.
///
/// # Non-fatal
///
/// All errors are logged at `warning!` and swallowed — the boot proceeds.
/// If pre-creation fails, the guest will hit the original `-EACCES` blocker
/// at iteration ~3059 and exit(1), but at least the failure mode is
/// diagnosable from the parent's log.
fn precreate_sysfs_stubs(rootfs_prefix: &str) {
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::fs::PermissionsExt;

    // Helper: create a directory (idempotent) with a given mode.
    let mkdir = |rel: &str, mode: u32| {
        let path = format!("{}/{}", rootfs_prefix, rel);
        if let Err(e) = std::fs::create_dir_all(&path) {
            warning!(
                "[KR64] PARENT: failed to pre-create {} (mode {:o}): {} (init's open(/sys/...) may hit EACCES on host's real sysfs — iteration ~3059 exit(1) will persist)",
                path, mode, e
            );
            return;
        }
        if let Err(e) = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)) {
            warning!(
                "[KR64] PARENT: created {} but chmod {:o} failed: {} (non-fatal — directory exists, mode may be wrong)",
                path, mode, e
            );
        }
        info!(
            "[KR64] PARENT: pre-created {} (dir, mode {:o}) — fake sysfs entry for guest init's open()",
            path, mode
        );
    };

    // Helper: create an empty (or near-empty) file (idempotent). Does NOT
    // truncate on subsequent calls — only writes the seed content when the
    // file does not yet exist.
    let touch = |rel: &str, mode: u32, seed: &str| {
        let path = format!("{}/{}", rootfs_prefix, rel);
        // Create-if-missing. We deliberately do NOT use .truncate(true) so
        // a prior boot's content (e.g. a policy blob the guest wrote to
        // /sys/fs/selinux/load) is preserved — only the FIRST pre-creation
        // writes the seed bytes.
        match std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .mode(mode)
            .open(&path)
        {
            Ok(mut f) => {
                // If the file is empty (just created), seed it with `seed`
                // so init's read() returns that content instead of 0 bytes.
                // For `enforce`, "0" means permissive — the safe default.
                // For `load`, "" is fine (init writes its own policy blob).
                if !seed.is_empty() {
                    if let Ok(meta) = std::fs::metadata(&path) {
                        if meta.len() == 0 {
                            use std::io::Write;
                            let _ = f.write_all(seed.as_bytes());
                        }
                    }
                }
                let _ =
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode));
                info!(
                    "[KR64] PARENT: pre-created {} (file, mode {:o}, seed {:?}) — fake sysfs entry for guest init's open()",
                    path, mode, seed
                );
            }
            Err(e) => warning!(
                "[KR64] PARENT: failed to pre-create {} (mode {:o}): {} (init's open(/sys/...) may hit EACCES on host's real sysfs — iteration ~3059 exit(1) will persist)",
                path, mode, e
            ),
        }
    };

    // Create the directory tree first (parents before children).
    // Note: `create_dir_all` is recursive so we could just call it on the
    // leaf dirs, but we explicitly create each level so each gets its own
    // log line + mode setting (diagnostic clarity).
    mkdir("sys", 0o755);
    mkdir("sys/class", 0o755);
    mkdir("sys/fs", 0o755);
    mkdir("sys/fs/selinux", 0o755);

    // Then create the empty files. `enforce` is seeded with "0" (permissive)
    // so init's read() interprets SELinux as off — safe default for TWRP in
    // the sandbox. `load` is empty (init writes its policy blob to it; the
    // write succeeds silently against the regular file — no kernel policy
    // is actually loaded). `null` (6-Z154) is the write-sink old TWRP
    // init's logging redirect opens FIRST — run 32961216041 (arm64 redroid)
    // traced open("/sys/fs/selinux/null") → ENOENT → fallback mknodat →
    // -EACCES → init exit(1) at post-execve syscall #59. Pre-creating the
    // stub makes the FIRST open succeed, so the mknod fallback (and its
    // EXIT-side 6-Z154 stub) never even fires on this path.
    touch("sys/fs/selinux/enforce", 0o666, "0");
    touch("sys/fs/selinux/load", 0o666, "");
    touch("sys/fs/selinux/null", 0o666, "");

    // 6-Z271: device-mapper /sys node. First-stage init's
    // BlockDevInitializer::InitMiscDevice() (AOSP system/core/init/
    // block_dev_initializer.cpp) REGENERATES the dm uevent by opening
    // /sys/devices/virtual/misc/device-mapper/uevent for WRITING and
    // writing "add\n", then reads the NETLINK uevent socket (the tracer's
    // 6-Z271 synthetic-uevent delivery answers that read). Without the
    // directory, RegenerateUeventsForPath fails at opendir and init falls
    // to uevent_listener_.Poll(10s) — the measured "Wait for
    // device-mapper returned after 10010ms" hole (run 33411932921).
    mkdir("sys/devices", 0o755);
    mkdir("sys/devices/virtual", 0o755);
    mkdir("sys/devices/virtual/misc", 0o755);
    mkdir("sys/devices/virtual/misc/device-mapper", 0o755);
    // Seed mirrors the real kernel's uevent file for a misc device;
    // init's HandleUevent re-creates /dev/device-mapper via the tracer's
    // mknod fake — the ioctl surface stays ENOTTY (unchanged failure
    // mode vs the post-timeout path the boot already survived).
    touch(
        "sys/devices/virtual/misc/device-mapper/uevent",
        0o644,
        "MAJOR=254\nMINOR=0\nDEVNAME=device-mapper\nDEVTYPE=misc\n",
    );

    info!(
        "[KR64] PARENT: pre-created fake sysfs in {}/sys (class/ + fs/selinux/{{enforce,load,null}}) — guest init's open('/sys/class') + open('/sys/fs/selinux/*') will succeed instead of -EACCES (Task 6-P; was the iter-3059 exit(1) blocker after 6-O's property_contexts deletion; 6-Z154 added selinux/null — was the arm64 redroid init exit(1))",
        rootfs_prefix
    );
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

/// NOP the conditional jump after `selinux_android_load_policy()` so init
/// never takes the failure path → the `while(1) pause();` loop becomes
/// UNREACHABLE from main().
///
/// DEFINITIVE root-cause fix per 6-I's disassembly (worklog entry 6-I).
///
/// # Background — the pause() loop root cause
///
/// TWRP init's pause() loop at vaddr `0x08049103`-`0x08049108` is literally:
///   ```text
///   08049103: e8 08 10 02 00   call 806a110 <pause>
///   08049108: eb f9            jmp  8049103 <main+0xb33>
///   ```
/// This is a TIGHT UNCONDITIONAL infinite spin: `while(1) pause();`. NO
/// condition check, NO return-value check, NO global flag test, NO
/// property read. The dispatcher's hypothesis ("init waits on an
/// in-process flag/condition variable") was INCORRECT — there is no flag.
///
/// How init reaches that loop: main() at `0x08048fff` calls
/// `selinux_android_load_policy()` (at `0x080a14f0`). That function's
/// VERY FIRST syscall is `mount("selinuxfs", "/sys/fs/selinux",
/// "selinuxfs", 0, NULL)`. In kr64's ptrace_emu sandbox the mount()
/// syscall returns negative (no selinuxfs is mounted). The function's
/// failure path checks errno (cmp `$0x13`/EINVAL, cmp `$0x2`/ENOENT); for
/// any OTHER errno it logs an error and returns `-1`. main's `js
/// 0x080490cf` at vaddr `0x08049006` (file offset `0x1006`) takes the
/// failure path:
///   ```text
///   080490cf: klog "<3>init: SELinux: Failed to load policy; rebooting into recovery mode"
///   080490fe: call android_reboot(0xDEAD0003 /*ANDROID_RB_RESTART2*/, 0, "recovery")
///   08049103: call pause      ← LOOP START
///   08049108: jmp  08049103   ← JMP BACK (infinite spin)
///   ```
/// The `android_reboot()` syscall is faked/intercepted by the sandbox —
/// it returns instead of actually rebooting. So init spins in pause()
/// forever, waiting for a reboot that never happens.
///
/// This explains why ALL prior fixes failed:
///   * Return-value tricks (6-D `-EINTR`, 6-E/6-G `-ENOSYS`, 6-G
///     `-ETIMEDOUT`): init NEVER READS pause's return value. The loop is
///     unconditional — even if pause() returns `0`/`-1`/`-EINTR`/etc.,
///     the `jmp` just calls pause() again.
///   * Property service socket stub (6-H): init is NOT waiting on the
///     property service socket — that hypothesis was wrong. The pause loop
///     is the post-reboot spin-wait and has nothing to do with the
///     property socket.
///   * 100ms sleep (6-F): reduced the spin rate but the loop is
///     unconditional, so it just kept spinning.
///
/// # The fix (6-I's Option A — 6-byte NOP patch)
///
/// File offset `0x1006` (vaddr `0x08049006`):
///   * Original: `0f 88 c3 00 00 00` (`js 0x080490cf` — jump to failure path)
///   * Patched:  `90 90 90 90 90 90` (6 × NOP — never take the failure path)
///
/// Displacement verification: `js` is `0F 88 + imm32`, where `imm32` is
/// the signed displacement added to the address of the NEXT instruction.
/// `js` at `0x08049006` is 6 bytes long → next instruction is at
/// `0x0804900c`. Target is `0x080490cf`. Displacement = `0x080490cf −
/// 0x0804900c` = `0xc3`. As 4-byte little-endian signed: `c3 00 00 00`.
/// So bytes are `0f 88 c3 00 00 00` ✓.
///
/// Effect: even if `selinux_android_load_policy()` returns negative, init
/// does NOT enter the failure path. Falls through to
/// `selinux_init_all_handles()` (may fail non-fatally) →
/// `__property_get("ro.boot.selinux")` (returns NULL → defaults to
/// enforcing) → `security_setenforce(esi)` (may fail non-fatally) →
/// `jmp main+0x317` → TWRP recovery boot path. The pause loop becomes
/// UNREACHABLE from main().
///
/// # Honest caveat — this is a WORKAROUND, not a "fix"
///
/// This is a WORKAROUND for the missing selinuxfs mount in the sandboxed
/// environment (NOT a crash suppression — the proper fix would be to
/// provide a fake selinuxfs at `/sys/fs/selinux/{load,enforce,booleans,
/// ...}` and intercept `mount("selinuxfs", ...)` in ptrace_emu to return
/// 0, a larger effort that may still hit later failures in
/// `selinux_android_load_policy()` when it tries to open
/// `/sys/fs/selinux/load`). There is also residual risk that
/// `selinux_init_all_handles()` or `security_setenforce()` aborts/crashes
/// when called without a real selinux mount — if so, switch to Option C
/// (skip the entire selinux block by converting the `jne` after
/// `selinux_is_disabled()` into an unconditional `jmp`, file offset
/// `0x0fe3`). Reference: 6-I's disassembly report (worklog entry 6-I).
///
/// # Pattern (8 bytes — 2 bytes of pre-context + 6 bytes of the jump)
///
/// ```text
/// 85 c0                test eax, eax      (pre-context, unchanged)
/// 0f 88 c3 00 00 00    js 0x080490cf      (PATCH SITE — 6 bytes)
/// ```
///
/// The pre-context (`test eax, eax` — the result of
/// `selinux_android_load_policy()` left in EAX) makes the 8-byte pattern
/// unique: a bare 6-byte `0f 88 c3 00 00 00` could occur by coincidence
/// elsewhere in the binary, but `85 c0` immediately preceding it is a
/// strong signal we're at the right site. We additionally verify the
/// match offset is `0x1004` (the expected file offset for vaddr
/// `0x08049006` minus the 2-byte pre-context) as a safety net against
/// coincidental matches in a different code path.
///
/// # Replacement (overwrite bytes 2..8 of the matched pattern with 6 NOPs)
///
/// ```text
/// 85 c0                test eax, eax      (unchanged pre-context)
/// 90 90 90 90 90 90    nop × 6            (patched)
/// ```
///
/// # Idempotency
///
/// A prior application replaces the 6 `js`-bytes with 6 NOPs. We detect
/// that by scanning for the patched signature (`85 c0 90 90 90 90 90 90`
/// — pre-context + 6 NOPs) and return [`AlreadyApplied`]. Same idempotency
/// scheme as [`patch_twrp_init_klog_init`] above and the inline
/// find_property patch in [`run`].
///
/// On aarch64 the i386 byte pattern is irrelevant, so the function short-
/// circuits to [`SelinuxLoadSkipPatchResult::Skipped`] (the same approach
/// as [`patch_twrp_init_klog_init`]).
///
/// # Returns
///
/// See [`SelinuxLoadSkipPatchResult`] for the per-variant semantics.
fn patch_twrp_init_selinux_load_skip(init_bytes: &mut [u8]) -> SelinuxLoadSkipPatchResult {
    // The byte pattern we match (see PATTERN below) is specific to the
    // i386 build of TWRP init. On aarch64 TWRP images, the binary uses
    // an entirely different instruction encoding (AArch64), so the
    // pattern will never match — and the selinux-load-failure code path
    // this patch addresses is specific to the i386 init's main() anyway
    // (aarch64 TWRP uses a different selinux setup flow). Skip the patch
    // entirely on aarch64 to avoid the misleading "TWRP version
    // mismatch?" warning that the caller would otherwise log on every
    // arm64 boot.
    #[cfg(target_arch = "aarch64")]
    {
        info!(
            "[KR64] selinux_load_skip patch is x86-only; skipped on arm64 (aarch64 TWRP uses a different selinux setup flow)"
        );
        // Mark `init_bytes` as intentionally unused on aarch64 to silence
        // the unused_variables lint without renaming the parameter (which
        // is shared with the non-aarch64 branch below).
        let _ = init_bytes;
        SelinuxLoadSkipPatchResult::Skipped
    }

    // On non-aarch64 hosts (x86, x86_64, etc.), perform the actual
    // i386-instruction-pattern match.
    #[cfg(not(target_arch = "aarch64"))]
    {
        // Pattern: 2 bytes of pre-context (`test eax, eax` — the result
        // of `selinux_android_load_policy()` left in EAX) + the 6-byte
        // `js` instruction whose signed 32-bit displacement (0x000000c3)
        // points at the failure path at vaddr 0x080490cf.
        const PATTERN: [u8; 8] = [
            // test eax, eax
            0x85, 0xc0, // js 0x080490cf (signed disp 0x000000c3, little-endian)
            0x0f, 0x88, 0xc3, 0x00, 0x00, 0x00,
        ];
        const PATCH_OFF: usize = 2; // index of the js opcode within PATTERN
        const PATCH_LEN: usize = 6; // length of the js instruction

        // Patched signature: `test eax, eax` (unchanged) + 6 × NOP. Used to
        // detect an already-applied patch so we skip the rewrite instead of
        // (mis)matching nothing and returning NotFound.
        const PATCHED_SIG: [u8; 8] = [
            // test eax, eax (unchanged pre-context)
            0x85, 0xc0, // 6 × NOP (patched)
            0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
        ];
        // Replacement bytes (6 NOPs).
        const NOP_PATCH: [u8; PATCH_LEN] = [0x90; PATCH_LEN];

        // Expected file offset where PATTERN should match. The `js`
        // instruction itself is at file offset 0x1006 (vaddr 0x08049006);
        // PATTERN starts 2 bytes earlier (at 0x1004) because of the
        // `test eax, eax` pre-context.
        const EXPECTED_MATCH_OFF: usize = 0x1004;

        if init_bytes.len() < PATTERN.len() {
            return SelinuxLoadSkipPatchResult::NotFound;
        }

        // 1. Idempotency check: scan for the patched signature. If present,
        //    the patch was already applied in a previous boot — skip.
        for i in 0..=(init_bytes.len() - PATCHED_SIG.len()) {
            if init_bytes[i..i + PATCHED_SIG.len()] == PATCHED_SIG {
                return SelinuxLoadSkipPatchResult::AlreadyApplied;
            }
        }

        // 2. Find the unpatched pattern. We take the FIRST match; the
        //    8-byte pattern (including the `test eax, eax` pre-context) is
        //    unique enough that there should only be one match in the entire
        //    binary.
        for i in 0..=(init_bytes.len() - PATTERN.len()) {
            if init_bytes[i..i + PATTERN.len()] == PATTERN {
                // Sanity check: the match must be at the expected file
                // offset (0x1004). If it isn't, refuse to patch — the
                // 8-byte pattern matched by coincidence in a different code
                // path, and patching there could brick the binary. Return
                // NotFound so the caller logs the "TWRP version mismatch?"
                // warning.
                if i != EXPECTED_MATCH_OFF {
                    return SelinuxLoadSkipPatchResult::NotFound;
                }
                // Apply the patch: overwrite the 6 js-bytes with 6 NOPs.
                // The pre-context bytes [0..PATCH_OFF] are preserved.
                init_bytes[i + PATCH_OFF..i + PATCH_OFF + PATCH_LEN].copy_from_slice(&NOP_PATCH);
                return SelinuxLoadSkipPatchResult::Applied;
            }
        }
        SelinuxLoadSkipPatchResult::NotFound
    }
}

/// Result of attempting to apply the selinux-load-failure NOP patch.
///
/// See [`patch_twrp_init_selinux_load_skip`] for the full root-cause
/// analysis and the per-variant semantics. Mirrors [`KlogInitPatchResult`]
/// above: `Applied` and `AlreadyApplied` are successes, `Skipped` is an
/// expected non-action (e.g. aarch64), and `NotFound` is a potential
/// problem (TWRP version mismatch — the patch couldn't be applied, so
/// init will still spin in the pause() loop forever after
/// `selinux_android_load_policy()` fails).
///
/// `#[allow(dead_code)]` is needed for the same reason as
/// [`KlogInitPatchResult`]: the variants are platform-conditional
/// (`Skipped` is only constructed on aarch64, the other three are only
/// constructed on non-aarch64 hosts). On any given host, the "other"
/// platform's variants would otherwise be flagged as dead code by the
/// compiler.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelinuxLoadSkipPatchResult {
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
    /// Without this patch, init will still spin in pause() forever after
    /// `selinux_android_load_policy()` fails.
    NotFound,
}

/// PRAGMATIC "make it not crash" patch — NOP the property_contexts parser
/// crash instruction at vaddr 0x080a0b9e (file offset 0x58b9e).
///
/// # HONEST LABEL — this is NOT a proper fix
///
/// **This is a pragmatic "make it not crash" patch, NOT a proper fix.**
///
/// The proper fix would trace the caller of init's property_contexts
/// parser (the function that iterates the SEPolicy context-table at
/// 0x80ce270 + calls the parser with a context struct) and initialize
/// the uninitialized field at offset 0x14 — but that's deep in AOSP 5.1
/// libselinux internals (DISPATCHER-FINAL-5 + DISPATCHER-FINAL-6).
/// That proper fix is out of scope for this session.
///
/// # Root cause (per 6-K + DISPATCHER-FINAL-3/4/5/6)
///
/// After 6-J's selinux-load-skip NOP patch (eliminating the pause() loop)
/// and 6-L's `#line`-directive strip, the guest progresses to iteration
/// 338 of the property_contexts parser and hits a SIGSEGV at:
///
/// ```text
///   rip    = 0x080a0b9e
///   si_addr= 0x74616433   (ASCII "3dat" — garbage, not a real pointer)
///   eax    = 0x4
///   edx    = 0x74616433   (the garbage pointer being written through)
///   insn   = c7 42 04 00 00 00 00
///            movl $0x0, 0x4(%edx)        ; <-- CRASH HERE
/// ```
///
/// DISPATCHER-FINAL-5 traced edx back to its source:
/// ```text
///   80a0adc: mov 0x14(%eax),%eax       # eax = ctx->field_at_0x14
///   80a0adf: mov %eax,-0xc98(%ebp)     # local_var = that pointer
///   ...later...
///   80a0b98: mov -0xc98(%ebp),%edx     # edx = local_var (garbage)
///   80a0b9e: movl $0x0,0x4(%edx)       # CRASH: write to [edx+4]
/// ```
///
/// So edx = `ctx->field_at_0x14` where `ctx` is the parser's FIRST
/// ARGUMENT (the parser context struct). The garbage 0x74616433 lives in
/// the CALLER's context struct — the CALLER (DISPATCHER-FINAL-6 found it
/// is the function iterating the SEPolicy context table at 0x80ce270)
/// passes a context whose field at offset 0x14 is uninitialized/garbage.
/// This is a libselinux internal bug — NOT a file-content bug (6-L's
/// `#line` removal was a wrong hypothesis; the file is fine).
///
/// # The patch
///
/// NOP the 7-byte crash instruction at file offset 0x58b9e:
///
/// ```text
///   Original: c7 42 04 00 00 00 00    (movl $0x0, 0x4(%edx))
///   Patched:  90 90 90 90 90 90 90    (7 × NOP)
/// ```
///
/// The file offset is computed from the vaddr + the ELF load base:
///   `0x080a0b9e - 0x08048000 = 0x58b9e`
///
/// # Effect (and honest caveats)
///
/// With the crash instruction NOP'd, the parser SKIPS the write to the
/// garbage pointer and continues execution. The parser may produce
/// wrong/missing results for the entry it was processing when it hit
/// the crash — but it won't crash, and init proceeds further in the
/// boot path.
///
/// **Honest caveats:**
///   * This is a "make it not crash" patch, NOT a proper fix.
///   * The parser's incorrect internal state MAY cause a LATER crash
///     elsewhere in libselinux or in code consuming the parser output.
///   * The only definitive proof that this unblocks the boot is a
///     `ui-e2e-test.yml` run + VLM screenshot analysis. Do NOT claim
///     "TWRP boots now" without that.
///
/// # Architecture notes
///
/// The byte pattern is specific to the **i386** build of TWRP init
/// (TWRP 3.7.0_9-0). On **aarch64** TWRP images, the binary uses an
/// entirely different instruction encoding (AArch64), so the crash
/// instruction at vaddr 0x080a0b9e (an x86 i386 absolute address) is
/// meaningless. We skip the patch entirely on aarch64 (returning
/// [`PropertyContextsCrashNopPatchResult::Skipped`]) — same approach
/// as [`patch_twrp_init_klog_init`] + [`patch_twrp_init_selinux_load_skip`].
///
/// # Idempotence
///
/// Direct offset-based check: if the 7 bytes at file offset 0x58b9e are
/// already `90 90 90 90 90 90 90`, the patch was applied in a previous
/// boot and we return [`PropertyContextsCrashNopPatchResult::AlreadyApplied`]
/// without modifying the bytes.
///
/// # Safety check
///
/// Unlike [`patch_twrp_init_selinux_load_skip`] (which scans the whole
/// binary for an 8-byte pattern), this patch directly checks the bytes
/// at the EXPECTED file offset 0x58b9e. The 7-byte pattern
/// `c7 42 04 00 00 00 00` (`movl $0, [edx+4]`) is a common instruction
/// and could legitimately appear elsewhere in the binary — scanning for
/// it would yield many coincidental matches. The direct-offset check
/// is therefore both safer and unambiguous: we ONLY touch the exact
/// crash site, never any other `movl $0, [edx+4]` instance in the binary.
///
/// # Arguments
///
/// * `init_bytes` - The init binary's bytes (read from `{rootfs}/init`).
///
/// # Returns
///
/// See [`PropertyContextsCrashNopPatchResult`] for the per-variant semantics.
fn patch_twrp_init_property_contexts_crash_nop(
    init_bytes: &mut [u8],
) -> PropertyContextsCrashNopPatchResult {
    // The byte pattern we match is specific to the i386 build of TWRP
    // init. On aarch64 TWRP images, the binary uses an entirely different
    // instruction encoding (AArch64), so the crash instruction at vaddr
    // 0x080a0b9e (an i386 absolute address) is meaningless — and the
    // libselinux internal bug this patch works around is specific to the
    // i386 build of the property_contexts parser anyway. Skip the patch
    // entirely on aarch64 to avoid the misleading "TWRP version
    // mismatch?" warning that the caller would otherwise log on every
    // arm64 boot.
    #[cfg(target_arch = "aarch64")]
    {
        info!(
            "[KR64] property_contexts crash-nop patch is x86-only; skipped on arm64 (aarch64 TWRP uses a different property_contexts parser)"
        );
        // Mark `init_bytes` as intentionally unused on aarch64 to silence
        // the unused_variables lint without renaming the parameter (which
        // is shared with the non-aarch64 branch below).
        let _ = init_bytes;
        PropertyContextsCrashNopPatchResult::Skipped
    }

    // On non-aarch64 hosts (x86, x86_64, etc.), perform the actual
    // i386-instruction-pattern match at the expected file offset.
    #[cfg(not(target_arch = "aarch64"))]
    {
        // Pattern: `movl $0x0, 0x4(%edx)` = `c7 42 04 00 00 00 00`
        //   c7            opcode (MOV r/m32, imm32)
        //   42            ModR/M (mod=01 disp8, reg=000, rm=010 edx)
        //   04            8-bit displacement: [edx + 0x4]
        //   00 00 00 00   imm32 = 0
        // Total: 7 bytes (matches the disassembly at vaddr 0x080a0b9e).
        const PATTERN: [u8; 7] = [0xc7, 0x42, 0x04, 0x00, 0x00, 0x00, 0x00];
        // Patched: 7 × NOP.
        const NOP_PATCH: [u8; 7] = [0x90; 7];
        // Expected file offset: 0x080a0b9e (vaddr) - 0x08048000 (ELF load
        // base for this i386 PIE) = 0x58b9e. This is the file offset at
        // which the crash instruction `movl $0x0, 0x4(%edx)` lives.
        const EXPECTED_MATCH_OFF: usize = 0x58b9e;

        if init_bytes.len() < EXPECTED_MATCH_OFF + PATTERN.len() {
            // Binary is too small to contain the patch site — either
            // not a TWRP init binary or a very different TWRP version.
            return PropertyContextsCrashNopPatchResult::NotFound;
        }

        let target: &mut [u8] =
            &mut init_bytes[EXPECTED_MATCH_OFF..EXPECTED_MATCH_OFF + PATTERN.len()];

        // 1. Idempotency check: if the bytes are already 7 × NOP, the
        //    patch was applied in a previous boot — skip.
        if target == NOP_PATCH {
            return PropertyContextsCrashNopPatchResult::AlreadyApplied;
        }

        // 2. Apply the patch: if the bytes at the expected offset match
        //    the unpatched pattern, overwrite them with 7 × NOP.
        if target == PATTERN {
            target.copy_from_slice(&NOP_PATCH);
            return PropertyContextsCrashNopPatchResult::Applied;
        }

        // 3. Neither the unpatched pattern nor the patched signature
        //    matched at the expected offset. Either TWRP version drift
        //    (the crash instruction moved) OR the binary was already
        //    modified in some other way. Either way, refuse to patch —
        //    the caller logs a "TWRP version mismatch?" warning.
        PropertyContextsCrashNopPatchResult::NotFound
    }
}

/// Result of attempting to apply the property_contexts parser crash-NOP
/// patch (the pragmatic "make it not crash" patch at file offset 0x58b9e).
///
/// See [`patch_twrp_init_property_contexts_crash_nop`] for the full
/// root-cause analysis and the per-variant semantics. Mirrors
/// [`KlogInitPatchResult`] + [`SelinuxLoadSkipPatchResult`]: `Applied`
/// and `AlreadyApplied` are successes, `Skipped` is an expected
/// non-action (e.g. aarch64), and `NotFound` is a potential problem
/// (TWRP version mismatch — the patch couldn't be applied, so init
/// may SIGSEGV at rip=0x80a0b9e later in the boot path).
///
/// `#[allow(dead_code)]` is needed for the same reason as
/// [`KlogInitPatchResult`] + [`SelinuxLoadSkipPatchResult`]: the
/// variants are platform-conditional (`Skipped` is only constructed
/// on aarch64, the other three are only constructed on non-aarch64
/// hosts). On any given host, the "other" platform's variants would
/// otherwise be flagged as dead code by the compiler.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PropertyContextsCrashNopPatchResult {
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
    /// Pattern was not found at the expected file offset — caller
    /// should log a warning because this likely indicates a TWRP
    /// version mismatch. Without this patch, init will SIGSEGV at
    /// rip=0x80a0b9e (si_addr=0x74616433) when its property_contexts
    /// parser dereferences the garbage `ctx->field_at_0x14` pointer.
    NotFound,
}

/// PRAGMATIC symptom-mask patch — NOP the read_file() *arg2 store at
/// vaddr 0x8052f65 (file offset 0xaf65).
///
/// # HONEST LABEL — this is NOT a proper fix
///
/// **This is a pragmatic symptom-mask patch, NOT a proper fix.**
///
/// The proper fix belongs in the SIGSYS handler's register-preservation
/// logic — the garbage value 0x696e692f (ASCII "/ini") leaked into
/// arg2 because the SIGSYS handler raced with read_file()'s ABI
/// register use. That proper fix is out of scope for this session.
///
/// # Root cause (per 6-U DIAG KLOG diagnostic + 6-V-pre disassembly)
///
/// 6-U's DIAG KLOG diagnostic captured init's own write() buffer
/// contents inline, exposing a latent SIGSEGV. UI E2E run 32194676789
/// analysis showed iter count DROP from 3635 to 826 (the PEEKDATA
/// timing exposed a latent crash), with exit code -11 (SIGSEGV).
///
/// SIGSEGV details:
///   * si_code = 1 (MAPERR unmapped)
///   * si_addr = 0x696e692f (ASCII "/ini" — first 4 bytes of
///     "/init.rc" rodata leaked by a SIGSYS-handler race)
///   * rip    = 0x8052f65
///   * rsp    = 0xff864a70
///
/// The crash instruction at vaddr 0x8052f65 (file offset 0xaf65) is:
///
/// ```text
///   0x8052f5b: movb $0x0, 0x1(%edx,%ecx,1)   ; buffer[readcount] = 0
///                                            ; (buffer is NUL-terminated)
///   0x8052f60: test %eax,%eax                 ; if (arg2 != NULL)
///   0x8052f60: je 8052f67                     ; (NULL-guard — SHOULD skip
///                                            ; the store when arg2==NULL)
///   0x8052f65: mov %ecx,(%eax)                ; *arg2 = readcount
///                                            ; <-- CRASH HERE (89 08)
/// ```
///
/// `eax` = arg2 (an optional `ssize_t*` out-param) SHOULD be NULL
/// when the caller doesn't want the size back. The NULL-guard at
/// 0x8052f60 (`je 8052f67`) is designed to skip the store when
/// arg2==NULL — but arg2 is non-NULL GARBAGE (0x696e692f, leaked by
/// a SIGSYS-handler race), so the guard falls through to the crashing
/// store.
///
/// # The patch
///
/// NOP the 2-byte store instruction at file offset 0xaf65:
///
/// ```text
///   Original: 89 08        (mov %ecx,(%eax))
///   Patched:  90 90        (2 × NOP)
/// ```
///
/// The file offset is computed from the vaddr + the ELF load base:
///   `0x8052f65 - 0x8048000 = 0xaf65`
///
/// # Effect (and honest caveats)
///
/// With the store NOP'd, read_file() SKIPS writing the read byte-count
/// to *arg2 — but the buffer is STILL null-terminated at 0x8052f5b
/// (the `movb $0x0, 0x1(%edx,%ecx,1)` BEFORE the crash site is NOT
/// touched), so callers that use the buffer as a C string still work.
/// Only callers that explicitly depend on the ssize_t* out-param
/// being written are affected — 13 call sites exist; none critically
/// depend on the out-size being written (the buffer is NUL-terminated,
/// so callers can strlen() it if they need the length).
///
/// **Honest caveats:**
///   * This is a symptom-mask patch, NOT a proper fix.
///   * The real fix belongs in the SIGSYS handler's register-
///     preservation logic — preventing the 0x696e692f leak in the
///     first place.
///   * The ONLY definitive proof that this unblocks the boot is a
///     `ui-e2e-test.yml` run + VLM screenshot analysis. Do NOT claim
///     "TWRP boots now" without that.
///
/// # Architecture notes
///
/// The byte pattern is specific to the **i386** build of TWRP init
/// (TWRP 3.7.0_9-0). On **aarch64** TWRP images, the binary uses an
/// entirely different instruction encoding (AArch64), so the crash
/// instruction at vaddr 0x8052f65 (an x86 i386 absolute address) is
/// meaningless. We skip the patch entirely on aarch64 (returning
/// [`ReadFileSigsegvPatchResult::Skipped`]) — same approach as
/// [`patch_twrp_init_klog_init`] + [`patch_twrp_init_selinux_load_skip`]
/// + [`patch_twrp_init_property_contexts_crash_nop`].
///
/// # Idempotence
///
/// Direct offset-based check: if the 2 bytes at file offset 0xaf65
/// are already `90 90`, the patch was applied in a previous boot and
/// we return [`ReadFileSigsegvPatchResult::AlreadyApplied`] without
/// modifying the bytes.
///
/// # Safety check
///
/// Like [`patch_twrp_init_property_contexts_crash_nop`], this patch
/// directly checks the bytes at the EXPECTED file offset 0xaf65. The
/// 2-byte pattern `89 08` (`mov %ecx,(%eax)`) is a common instruction
/// and could legitimately appear elsewhere in the binary — scanning
/// for it would yield many coincidental matches. The direct-offset
/// check is therefore both safer and unambiguous: we ONLY touch the
/// exact crash site, never any other `mov %ecx,(%eax)` instance.
///
/// # Arguments
///
/// * `init_bytes` - The init binary's bytes (read from `{rootfs}/init`).
///
/// # Returns
///
/// See [`ReadFileSigsegvPatchResult`] for the per-variant semantics.
fn patch_twrp_init_read_file_sigsegv(init_bytes: &mut [u8]) -> ReadFileSigsegvPatchResult {
    // The byte pattern we match is specific to the i386 build of TWRP
    // init. On aarch64 TWRP images, the binary uses an entirely
    // different instruction encoding (AArch64), so the crash
    // instruction at vaddr 0x8052f65 (an i386 absolute address) is
    // meaningless — and the SIGSYS-handler race this patch works around
    // is specific to the i386 build of read_file() anyway. Skip the
    // patch entirely on aarch64 to avoid the misleading "TWRP version
    // mismatch?" warning that the caller would otherwise log on every
    // arm64 boot.
    #[cfg(target_arch = "aarch64")]
    {
        info!(
            "[KR64] read_file() SIGSEGV patch is x86-only; skipped on arm64 (aarch64 TWRP uses a different read_file() implementation)"
        );
        // Mark `init_bytes` as intentionally unused on aarch64 to
        // silence the unused_variables lint without renaming the
        // parameter (which is shared with the non-aarch64 branch below).
        let _ = init_bytes;
        ReadFileSigsegvPatchResult::Skipped
    }

    // On non-aarch64 hosts (x86, x86_64, etc.), perform the actual
    // i386-instruction-pattern match at the expected file offset.
    #[cfg(not(target_arch = "aarch64"))]
    {
        // Pattern: `mov %ecx,(%eax)` = `89 08`
        //   89            opcode (MOV r/m32, r32)
        //   08            ModR/M (mod=00, reg=001 ecx, rm=000 eax)
        // Total: 2 bytes (matches the disassembly at vaddr 0x8052f65).
        const PATTERN: [u8; 2] = [0x89, 0x08];
        // Patched: 2 × NOP.
        const NOP_PATCH: [u8; 2] = [0x90; 2];
        // Expected file offset: 0x8052f65 (vaddr) - 0x8048000 (ELF load
        // base for this i386 PIE) = 0xaf65. This is the file offset at
        // which the crash instruction `mov %ecx,(%eax)` lives.
        const EXPECTED_MATCH_OFF: usize = 0xaf65;

        if init_bytes.len() < EXPECTED_MATCH_OFF + PATTERN.len() {
            // Binary is too small to contain the patch site — either
            // not a TWRP init binary or a very different TWRP version.
            return ReadFileSigsegvPatchResult::NotFound;
        }

        let target: &mut [u8] =
            &mut init_bytes[EXPECTED_MATCH_OFF..EXPECTED_MATCH_OFF + PATTERN.len()];

        // 1. Idempotency check: if the bytes are already 2 × NOP, the
        //    patch was applied in a previous boot — skip.
        if target == NOP_PATCH {
            return ReadFileSigsegvPatchResult::AlreadyApplied;
        }

        // 2. Apply the patch: if the bytes at the expected offset match
        //    the unpatched pattern, overwrite them with 2 × NOP.
        if target == PATTERN {
            target.copy_from_slice(&NOP_PATCH);
            return ReadFileSigsegvPatchResult::Applied;
        }

        // 3. Neither the unpatched pattern nor the patched signature
        //    matched at the expected offset. Either TWRP version drift
        //    (the crash instruction moved) OR the binary was already
        //    modified in some other way. Either way, refuse to patch —
        //    the caller logs a "TWRP version mismatch?" warning.
        ReadFileSigsegvPatchResult::NotFound
    }
}

/// Result of attempting to apply the read_file() SIGSEGV-NOP patch
/// (the pragmatic symptom-mask patch at file offset 0xaf65).
///
/// See [`patch_twrp_init_read_file_sigsegv`] for the full root-cause
/// analysis and the per-variant semantics. Mirrors
/// [`PropertyContextsCrashNopPatchResult`] + [`KlogInitPatchResult`] +
/// [`SelinuxLoadSkipPatchResult`]: `Applied` and `AlreadyApplied` are
/// successes, `Skipped` is an expected non-action (e.g. aarch64), and
/// `NotFound` is a potential problem (TWRP version mismatch — the patch
/// couldn't be applied, so init may SIGSEGV at rip=0x8052f65 later in
/// the boot path).
///
/// `#[allow(dead_code)]` is needed for the same reason as
/// [`PropertyContextsCrashNopPatchResult`]: the variants are
/// platform-conditional (`Skipped` is only constructed on aarch64, the
/// other three are only constructed on non-aarch64 hosts). On any
/// given host, the "other" platform's variants would otherwise be
/// flagged as dead code by the compiler.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadFileSigsegvPatchResult {
    /// Patch was applied to the bytes — caller should write them back.
    Applied,
    /// Patch was already applied in a previous boot — caller can skip
    /// the write (no modification needed, idempotent).
    AlreadyApplied,
    /// Patch was intentionally skipped (e.g. on aarch64, where the
    /// i386-only byte pattern is irrelevant). The skip reason has
    /// been logged; the caller should NOT log a "version mismatch"
    /// warning and should NOT write the bytes back.
    Skipped,
    /// Pattern was not found at the expected file offset — caller
    /// should log a warning because this likely indicates a TWRP
    /// version mismatch. Without this patch, init will SIGSEGV at
    /// rip=0x8052f65 (si_addr=0x696e692f) when read_file() writes the
    /// read byte-count to the garbage `*arg2` pointer leaked by the
    /// SIGSYS-handler race.
    NotFound,
}

/// Result of attempting to apply the poll-loop NOP patch (Task 6-Z19).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PollLoopNopPatchResult {
    Applied,
    AlreadyApplied,
    Skipped,
    NotFound,
}

/// Task 6-Z19: NOP the `call poll` + `test %eax,%eax` at the top of init's
/// main poll loop (vaddr 0x8048c59, file offset 0xc59) so the loop body
/// runs WITHOUT calling poll. This breaks the POLLERR busy-wait: init's
/// property_service poll loop spins at ~1000/sec because poll(fake-bound
/// socket) returns POLLERR=1 instantly. NOP-ing the call means eax keeps
/// its prior value (likely 0 from a preceding syscall) → the `jle` branch
/// is taken → init continues the loop body (which checks action queues,
/// property changes, etc.) WITHOUT the poll → no spin.
///
/// Verified via objdump on /tmp/twrp-rd/init (the i386 statically-linked
/// TWRP init from twrp-3.7.0_9-0-byt_t_crv2.img):
///   8048c59: e8 f2 17 02 00    call   806a450 <poll>   ; 5 bytes
///   8048c5e: 85 c0             test   %eax,%eax         ; 2 bytes
///   8048c60: 0f 8e 8a fe ff ff jle    8048af0           ; 6 bytes
/// File offset = 0x8048c59 - 0x8048000 = 0xc59. We NOP 7 bytes (the call +
/// the test), leaving the jle intact (it will branch on the stale eax,
/// which is 0 → jle taken → continue loop without re-polling).
///
/// CAVEAT: this is a pragmatic symptom-mask (like 6-V/6-M). The real fix
/// is to make the property_service socket actually functional. But this
/// unblocks the recovery to reach the framebuffer while the real socket
/// fix is developed. The poll at 0x8057e89 (ueventd_main) + 0x805ce19
/// (parent) are NOT patched (they're different loops — ueventd needs real
/// poll for device events, parent's poll is a different context).
#[allow(clippy::doc_lazy_continuation)]
#[allow(dead_code)] // Task 6-Z28: call site commented out (reverted 6-Z19 NOP), kept for reference
fn patch_twrp_init_poll_loop_nop(init_bytes: &mut [u8]) -> PollLoopNopPatchResult {
    #[cfg(target_arch = "aarch64")]
    {
        let _ = init_bytes;
        PollLoopNopPatchResult::Skipped
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        // Pattern: `call poll` (e8 f2 17 02 00) + `test %eax,%eax` (85 c0) = 7 bytes
        const PATTERN: [u8; 7] = [0xe8, 0xf2, 0x17, 0x02, 0x00, 0x85, 0xc0];
        const NOP_PATCH: [u8; 7] = [0x90; 7];
        const EXPECTED_MATCH_OFF: usize = 0xc59;

        if init_bytes.len() < EXPECTED_MATCH_OFF + PATTERN.len() {
            return PollLoopNopPatchResult::NotFound;
        }
        let target: &mut [u8] =
            &mut init_bytes[EXPECTED_MATCH_OFF..EXPECTED_MATCH_OFF + PATTERN.len()];
        if target == NOP_PATCH {
            return PollLoopNopPatchResult::AlreadyApplied;
        }
        if target == PATTERN {
            target.copy_from_slice(&NOP_PATCH);
            return PollLoopNopPatchResult::Applied;
        }
        PollLoopNopPatchResult::NotFound
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

/// 6-Z192: probe the guest's init binary for the Android 8+ property-area
/// format. Two needles, ANY of which implies the NEW subdirectory layout:
///   * `properties_serial` — present in inits that reference the serial
///     file directly (e.g. twrp-3.7.0_9-0-whyred's init).
///   * `property_info` — present in every AOSP 8+ system/core init
///     (property_service.cpp's `CreateSerializedPropertyInfo()` writes
///     kPropertyInfoPath). Empirically the ONLY needle in some builds:
///     OrangeFox R12.0's /system/bin/init contains `property_info` (2x)
///     but NOT `properties_serial` (the serial path literal lives in
///     bionic's libc, not init) — run 33157500559 misdetected it as
///     OLD format because the single-needle probe missed it.
/// Old-format inits (AOSP 5.1/6.0 bionic, e.g. the angler TWRP builds —
/// both 2.8.7.0 and 3.7.0_9-0) contain NEITHER literal (verified against
/// the real binaries). Unreadable init → `false` (legacy behavior).
fn probe_init_new_property_format(init_path: &str) -> bool {
    const NEEDLES: [&[u8]; 2] = [b"properties_serial", b"property_info"];
    match std::fs::read(init_path) {
        Ok(bytes) => {
            // Static init binaries are ≤ ~2 MB; a single linear scan at
            // boot is negligible. windows() over 2 MB ≈ single-digit ms.
            NEEDLES
                .iter()
                .any(|needle| bytes.windows(needle.len()).any(|w| w == *needle))
        }
        Err(_) => false,
    }
}

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
// 6-Z192: unit-test the probe against REAL artifacts recorded from the
// corpus: the whyred 3.7 init contains the literal, the angler 2.8/3.7
// inits do not. (Embedded synthetic binaries keep the test hermetic.)
#[cfg(test)]
#[test]
fn z192_property_format_probe_detects_new_format() {
    // An init carrying the NEW-format rodata string.
    let new_init = {
        let mut b = vec![0u8; 1024];
        b[100..117].copy_from_slice(b"properties_serial");
        b
    };
    // 6-Z196: the OrangeFox shape — `property_info` present,
    // `properties_serial` ABSENT (the serial literal lives in bionic's
    // libc for AOSP system inits; init itself only writes the index).
    let fox_init = {
        let mut b = vec![0u8; 1024];
        let path = b"/dev/__properties__/property_info";
        b[80..80 + path.len()].copy_from_slice(path);
        b
    };
    // An old-format init: references /dev/__properties__ only.
    let old_init = {
        let mut b = vec![0u8; 1024];
        b[100..119].copy_from_slice(b"/dev/__properties__");
        b
    };
    let dir = std::env::temp_dir().join(format!("kr64-z192-probe-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let p_new = dir.join("init_new");
    let p_fox = dir.join("init_fox");
    let p_old = dir.join("init_old");
    let p_missing = dir.join("init_missing");
    std::fs::write(&p_new, &new_init).unwrap();
    std::fs::write(&p_fox, &fox_init).unwrap();
    std::fs::write(&p_old, &old_init).unwrap();
    assert!(probe_init_new_property_format(p_new.to_str().unwrap()));
    assert!(probe_init_new_property_format(p_fox.to_str().unwrap()));
    assert!(!probe_init_new_property_format(p_old.to_str().unwrap()));
    assert!(!probe_init_new_property_format(p_missing.to_str().unwrap()));
    let _ = std::fs::remove_dir_all(&dir);
}

pub fn run<I: IntoIterator<Item = String>>(args: I) -> i32 {
    // 6-Z260: anchor the boot clock as the very first action so every
    // subsequent log line's [+Nms] prefix measures from daemon start.
    boot_clock_init();
    // ── 6-Z268: tracer scheduling priority ──────────────────────────
    // The tracer is a single thread emulating EVERY guest syscall; on a
    // phone it competes with the host app's render/GC threads and
    // system_server on the big cores. Android grants app UIDs
    // RLIMIT_NICE=40, so raising our own nice to -8 (THREAD_PRIORITY_
    // URGENT_DISPLAY equivalent) is permitted unprivileged; under the
    // root-mode CI it always succeeds. Best-effort: EACCES/EPERM are
    // tolerated silently (the old default priority remains correct).
    unsafe {
        if libc::setpriority(libc::PRIO_PROCESS, 0, -8) != 0 {
            let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if e == libc::EACCES || e == libc::EPERM {
                // Expected on some hardened kernels — keep going.
            }
        }
    }
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

    // Task 6-Z22: once-guard. The Activity's surfaceCreated callback
    // fires repeatedly (surface recreated every ~2 sec), and despite the
    // RENDERER_STARTED guard in core.rs, kr64::run() was being re-invoked
    // 11 times (verified on 9c91ce0 E2E: 11 "starting ptrace emulation
    // loop" entries, same PID). Each re-invocation forks a NEW init child
    // + kills the previous (6-Z13's old-kr64 kill) → the recovery
    // restarts from scratch every 2 sec, never reaching the framebuffer.
    // This static guard ensures run() executes ONLY ONCE per process —
    // subsequent calls log + return immediately, letting the first init
    // child run to completion (reach the framebuffer render + BOOT_COMPLETED).
    static KR64_RUN_STARTED: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);
    if KR64_RUN_STARTED
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::Acquire,
            std::sync::atomic::Ordering::Relaxed,
        )
        .is_err()
    {
        info!("[KR64] run() already called once this process — skipping re-init (Task 6-Z22: prevents the 2-sec re-fork cycle that restarts the recovery from scratch)");
        return 0;
    }

    info!("[KR64] starting daemon with config: {:?}", cfg);

    // ── 6-Z268: page-cache prefetch thread ──────────────────────────
    // The loader segment (app-start → guest execve, measured ~10.1 s)
    // is disk-I/O shaped on a cold page cache: the traced init then
    // re-reads the same tree (rc files, binaries, libs) through the
    // tracer. This thread walks the rootfs issuing
    // posix_fadvise(WILLNEED) — pure kernel readahead requests, no
    // data copied through userspace — so the blocks are warm by the
    // time the traced guest asks for them. Key boot files go first;
    // then a bounded breadth walk. Time-capped at 4 s and fully
    // best-effort (every error ignored); the thread is detached and
    // never touches tracer state, so it is fork-safe (pre-fork threads
    // must not hold locks at fork — this one holds no locks and only
    // touches open/read/close + fadvise; the worst case is a one-fd
    // leak into the child, which the 6-Z268 child_close_fds snapshot
    // closes anyway).
    {
        let rootfs_for_prefetch = std::path::PathBuf::from(&cfg.rootfs);
        let _ = std::thread::Builder::new()
            .name("kr64-prefetch".to_string())
            .spawn(move || {
                let start = std::time::Instant::now();
                let budget = std::time::Duration::from_secs(4);
                let fadvise = |path: &std::path::Path| {
                    if start.elapsed() > budget {
                        return;
                    }
                    if let Ok(md) = std::fs::metadata(path) {
                        if !md.is_file() || md.len() < 16 * 1024 {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Ok(cpath) = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()) {
                        unsafe {
                            let fd = libc::open(cpath.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC);
                            if fd >= 0 {
                                libc::posix_fadvise(fd, 0, 0, libc::POSIX_FADV_WILLNEED);
                                libc::close(fd);
                            }
                        }
                    }
                };
                // 1. Key boot files, in boot order.
                let key = [
                    "init",
                    "sbin/recovery",
                    "system/bin/init",
                    "system/bin/linker64",
                    "sbin/linker64",
                    "init.rc",
                    "init.recovery.rc",
                    "system/etc/sepolicy",
                    "sbin/busybox",
                ];
                for k in key {
                    if start.elapsed() > budget {
                        return;
                    }
                    fadvise(&rootfs_for_prefetch.join(k));
                }
                // 2. Bounded breadth walk of the ROM trees.
                let mut queue: std::collections::VecDeque<std::path::PathBuf> =
                    ["system", "sbin", "vendor", "apex", "odm", "etc"]
                        .iter()
                        .map(|d| rootfs_for_prefetch.join(d))
                        .collect();
                let mut visited: usize = 0;
                while let Some(dir) = queue.pop_front() {
                    if start.elapsed() > budget || visited > 20000 {
                        return;
                    }
                    visited += 1;
                    let Ok(rd) = std::fs::read_dir(&dir) else {
                        continue;
                    };
                    for entry in rd.flatten() {
                        let Ok(ft) = entry.file_type() else {
                            continue;
                        };
                        let p = entry.path();
                        if ft.is_dir() {
                            if queue.len() < 4096 {
                                queue.push_back(p);
                            }
                        } else {
                            fadvise(&p);
                        }
                    }
                }
            });
    }

    // Task 6-Z24: log the kr64 child's OWN pid so the FileLogger tee
    // surfaces it in logcat. This distinguishes "10 separate kr64
    // children (10 different PIDs = re-spawn)" from "1 kr64 child
    // re-execve'ing itself (same PID = re-execve)". On a864395 the
    // "same PID 3988" in logcat was the tee thread (app PID), not kr64's
    // PID — so we never knew kr64's actual PID. This fixes that.
    info!(
        "[KR64] run() entered, kr64 child pid={}, parent_pid={}",
        unsafe { libc::getpid() },
        unsafe { libc::getppid() }
    );

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
    // 6-Z270b: /dev/block/device-mapper defensive stubs. Run 33411932921
    // measured this init's poller checking BEYOND file existence (still
    // 10010 ms with the node present) — see the devices.rs caveat: kept
    // for access()/open()-style pollers, NOT a fix for this image's wait.
    if let Err(e) = devices::create_device_mapper_stubs(&cfg.rootfs) {
        warning!("[KR64] failed to create device-mapper stubs: {}", e);
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
    // Step 2.8b: virtual haptics + backlight sysfs (§13/§16).
    //
    // Legacy recoveries (TWRP 2.x/3.x era) poll
    //   /sys/class/timed_output/vibrator/enable
    // on EVERY page transition and probe
    //   /sys/class/leds/lcd-backlight/brightness + /sys/class/backlight/*
    // for the display brightness path. Missing surfaces cost a failed
    // open through the whole hook-retry + tracer ladder per attempt —
    // an observable syscall storm under old UIs. Materialising the
    // standard ABI files removes the storm generically. Failure is
    // non-fatal (warn only), mirroring the battery step above.
    // ---------------------------------------------------------------
    let _haptics_handle = match haptics::HapticsDevice::new(&cfg.rootfs).and_then(|dev| dev.spawn())
    {
        Ok(_h) => {
            info!(
                "[KR64] haptics + backlight sysfs materialised under {}/sys/class",
                cfg.rootfs
            );
            Some(_h)
        }
        Err(e) => {
            warning!(
                "[KR64] failed to start haptics/backlight sysfs: {} -- guest vibrates will no-op",
                e
            );
            None
        }
    };

    // ---------------------------------------------------------------
    // Step 2.9: create the property service stub socket + spawn the
    // accept thread (Task 6-H — see `spawn_property_service_thread` for
    // the full background).
    //
    // The short version: TWRP init (AOSP 5.1 bionic) loops on pause()
    // (i386 syscall 29) waiting for the property service to signal
    // readiness. ALL return-value tricks (-EINTR 6-D, -ENOSYS 6-E/6-G,
    // -ETIMEDOUT 6-G) FAILED because init fundamentally requires the
    // property SERVICE to exist + accept connections.
    //
    // This stub:
    //   * Creates /dev/socket/property_service Unix socket (mode 0666)
    //     in the rootfs. AOSP 5.1 bionic's `send_prop_msg` hard-codes
    //     this path.
    //   * Spawns an accept thread that reads a 128-byte prop_msg_t per
    //     connection + writes a 4-byte "0" (PROP_SUCCESS) response.
    //   * This MAY satisfy init's start_property_service() wait
    //     condition + break the pause() loop. If it doesn't, the next
    //     step is to populate /dev/__properties__ with a valid
    //     property_area header (already pre-created by 6-A's
    //     `vfs::make_old_format_property_area` call below — Step 5.6).
    //
    // Non-fatal: the guest can still attempt to boot without this (the
    // ptrace_emu's pause() handler will continue to return -ENOSYS /
    // -ETIMEDOUT per 6-G), but the pause loop is expected to persist.
    // ---------------------------------------------------------------
    spawn_property_service_thread(&cfg.rootfs);

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
    // 6-Z226: detect the GUEST recovery binary's bitness and use the
    // matching hook variants (see detect_guest_recovery_bitness). The
    // DEST paths are unchanged — only the staged content differs. If no
    // recovery binary is found (or it isn't ELF), default to the 64-bit
    // chain: the historic behavior for every arm64 image.
    // NOTE: cfg.rootfs (the host-side rootfs path) is used here —
    // rootfs_prefix ("" in root mode) is not computed until the sbin
    // staging block below, and cfg.rootfs always resolves to real files.
    let guest_is_64 = detect_guest_recovery_bitness(&cfg.rootfs).unwrap_or(true);
    if !guest_is_64 {
        info!(
            "[KR64] PARENT: 6-Z226: guest recovery is ELF32 — staging the armeabi-v7a hook chain"
        );
    }
    let hook_lib_getpid = if cfg.boot_recovery {
        info!("[KR64] TWRP boot: skipping libgetpid_hook.so read (init is statically linked)");
        None
    } else {
        find_and_read_hook_library(
            &cfg,
            &guest_hook_lib_name("libgetpid_hook.so", guest_is_64),
            "LD_PRELOAD will fail",
        )
    };
    let hook_lib_loader = if cfg.boot_recovery {
        info!(
            "[KR64] TWRP boot: skipping libtwoyi_loader_shlib.so read (recovery is i386; x86_64 loader cannot be loaded by the 32-bit bionic linker)"
        );
        None
    } else {
        find_and_read_hook_library(
            &cfg,
            &guest_hook_lib_name("libtwoyi_loader_shlib.so", guest_is_64),
            "seccomp virtualization disabled",
        )
    };
    // FB ioctl hook. Loaded for BOTH boot modes:
    //   * TWRP mode: written to /sbin/libtwrp_fb_hook.so and injected via
    //     the init.rc setenv patch (the recovery service's LD_PRELOAD).
    //   * AOSP-layout mode (6-Z216): written to /dev/libtwrp_fb_hook.so
    //     and appended to the env LD_PRELOAD (see the ld_preload_str
    //     build below). ROOT CAUSE (6-Z216, ver8e2 run 33267257605,
    //     OrangeFox R12.0 lavender): the AOSP-layout recovery binary is
    //     launched by init from /system/bin/recovery — NOT via the
    //     /sbin/recovery service the init.rc patch targets — so it
    //     inherited only libgetpid_hook.so + libtwoyi_loader_shlib.so,
    //     /dev/graphics/fb0 had no FB interposer, gr_init() failed,
    //     and the theme engine's gr_fb_width() deref'd the null
    //     framebuffer (libminuitwrp.so+0x2027c, si_addr=0x0) in a
    //     20-restart crash loop — UI never reached.
    //     The hook is inert for processes that never open
    //     /dev/graphics/fb0, so loading it unconditionally is safe.
    let hook_lib_twrp_fb = find_and_read_hook_library(
        &cfg,
        &guest_hook_lib_name("libtwrp_fb_hook.so", guest_is_64),
        "framebuffer virtualization disabled (recovery will crash in libminuitwrp.so gr_fb_width)",
    );

    // ---------------------------------------------------------------
    // Step 3.7: extract the REAL libdl.so from the APEX ext4 image
    // (BEFORE setup_mounts, while host paths are still accessible).
    //
    // 5-K's diagnosis (kr64-stderr.log line 81): the visible
    // /apex/com.android.runtime/lib64/bionic/libdl.so is a 5848-byte
    // bootstrap STUB — same size as /system/lib64/libdl.so. The hook
    // libraries' DT_NEEDED:libdl.so (LIBC version) is NOT satisfied
    // by this stub → linker64 segfault at offset 0xaf174 (faulting
    // address 0x86 = NULL soinfo).
    //
    // The REAL libdl.so lives INSIDE the APEX ext4 image at
    // /system/apex/com.android.runtime.apex (a ZIP file containing
    // apex_payload.img — the ext4 image). We extract it via:
    //   1. Detecting the .apex file at multiple candidate paths
    //      (rom_dir/system/apex/..., rootfs/system/apex/...,
    //       /system/apex/..., /apex/com.android.runtime.apex).
    //   2. Parsing the ZIP central directory to find apex_payload.img
    //      (STORED method only — DEFLATE entries are rejected because
    //      we don't have zlib; APEX payloads are typically STORED).
    //   3. Writing the ext4 image to `<apex_temp_dir>/twoyi-apex-payload.img`
    //      (parent's TMPDIR env var, fallback /data/data/io.twoyi/cache —
    //      NOT /tmp/, which does NOT exist in the parent's Android-app-
    //      sandbox context before setup_mounts; see apex_extract::apex_temp_dir).
    //   4. Loopback-mounting the ext4 image (via /dev/loop-control +
    //      LOOP_SET_FD + mount("ext4")) and reading lib64/bionic/libdl.so.
    //   5. Validating the bytes via is_real_libdl (ELF magic + > stub size).
    //
    // Fallback: scan /apex/com.android.runtime@*/lib64/bionic/libdl.so
    // for any that's larger than the stub.
    //
    // If everything fails, the bytes Option is None — the LD_LIBRARY_PATH
    // change (Step 5 below) still prepends /dev/ so a future fix can
    // drop a real libdl.so at /dev/libdl.so without code changes.
    //
    // TWRP BOOT: skip extraction entirely — TWRP's init is statically
    // linked and doesn't use LD_PRELOAD hooks (so it doesn't need
    // libdl.so at all). The recovery service does need libdl.so (via
    // LD_PRELOAD=/sbin/libtwrp_fb_hook.so), but the i686 libtwrp_fb_hook
    // is built against the 32-bit bionic which doesn't have the
    // stub-vs-real libdl.so problem on Android 11.
    //
    // ----------------------------------------------------------------
    // OPTION D (5-U's recommendation, PRIMARY path): try the APK
    // asset first. The real libdl.so is shipped as
    // app/src/main/assets/libdl.so (extracted by Java on app init to
    // {data_dir}/files/libdl.so via RomManager.extractLibdlAsset).
    // This bypasses the APEX loopback mount pipeline entirely, which
    // hit 4 sequential failure modes in 5-L/5-N/5-O/5-P/5-U (temp-write
    // ENOENT → loop_open ENOENT → mknod+fallback loop_open ENXIO for
    // all N 0..31 → kernel has no registered gendisk). Each fix exposed
    // the next layer; the loopback-mount approach depends on too many
    // kernel/permission prerequisites (CAP_MKNOD + CAP_SYS_ADMIN +
    // kernel loop driver + init.rc mknod + ext4 driver).
    //
    // Option D requires only: APK asset read (always works — Java
    // extracted it on init) + write to /dev/libdl.so on tmpfs (always
    // works, /dev is tmpfs after pivot_root).
    //
    // The `> 5848` byte-size guard in is_real_libdl catches accidentally
    // shipping the Android bootstrap stub as the asset. A placeholder
    // asset (small text file or zero-filled bytes) is also rejected
    // (size + ELF magic check both fail), so the code falls through
    // gracefully to find_real_libdl_so (APEX extraction).
    //
    // If Option D's read_libdl_asset returns None (asset missing,
    // placeholder, or stub-sized), fall back to the existing APEX
    // extraction pipeline (find_real_libdl_so). This is the path that
    // 5-L/5-N/5-O/5-P/5-U analyzed: still broken on the Android
    // emulator (open /dev/loopN returns ENXIO for all N 0..31 per 5-U),
    // but kept as a defensive fallback for future environments where
    // loop devices DO work.
    let real_libdl: Option<(String, Vec<u8>)> = if cfg.boot_recovery {
        info!("[KR64] TWRP boot: skipping APEX libdl.so extraction (init is statically linked, doesn't need libdl.so)");
        None
    } else {
        // Option D (PRIMARY, 5-U's recommendation): try the APK asset
        // first. The asset is shipped in the APK + extracted by Java to
        // {data_dir}/files/libdl.so on app init.
        if let Some((src, bytes)) = apex_extract::read_libdl_asset(&cfg) {
            info!(
                "[KR64] Option D: using APK asset libdl.so ({} bytes from {}) — bypasses APEX loopback mount pipeline",
                bytes.len(),
                src
            );
            Some((src, bytes))
        } else {
            // Option D unavailable (asset missing, placeholder, or
            // stub-sized) — fall back to the APEX extraction pipeline
            // (find_real_libdl_so). This is the existing 5-L/5-N/5-O/5-P
            // path: still broken on the Android emulator per 5-U, but
            // kept as a defensive fallback for future environments
            // where loop devices DO work.
            //
            // 6-Z268: RUNTIME SELF-HEAL CACHE. The shipped asset is a
            // 5848-byte placeholder (is_real_libdl always rejects it),
            // so find_real_libdl_so ran its FULL pipeline on every boot:
            // whole-.apex read (6.4 MB observed) + heap copy + temp
            // ext4 image write + loopback mount + extraction — all
            // pre-execve on the measured 10.1 s segment. A successful
            // extraction now persists the ~30 KB payload to
            // {data_dir}/cache/libdl.real.so and every subsequent boot
            // starts from a single validated file read instead. The
            // cache is validated with the same is_real_libdl gate and
            // lives OUTSIDE {data_dir}/files (Java's extractLibdlAsset
            // re-overwrites files/libdl.so from the placeholder on
            // every app start).
            let libdl_cache_path = format!("{}/cache/libdl.real.so", cfg.data_dir);
            if let Ok(cached) = std::fs::read(&libdl_cache_path) {
                if apex_extract::is_real_libdl(&cached) {
                    info!(
                        "[KR64] Option D+6-Z268: using cached real libdl.so from {} ({} bytes) — APEX extraction pipeline skipped",
                        libdl_cache_path,
                        cached.len()
                    );
                    Some((libdl_cache_path, cached))
                } else {
                    info!(
                        "[KR64] 6-Z268: cached libdl at {} failed the is_real_libdl gate — re-running extraction",
                        libdl_cache_path
                    );
                    let found = apex_extract::find_real_libdl_so(&cfg);
                    if let Some((_, ref bytes)) = found {
                        let _ = std::fs::create_dir_all(format!("{}/cache", cfg.data_dir));
                        let _ = std::fs::write(&libdl_cache_path, bytes);
                    }
                    found
                }
            } else {
                info!(
                    "[KR64] Option D unavailable (no real libdl.so APK asset) — falling back to APEX extraction (find_real_libdl_so)"
                );
                let found = apex_extract::find_real_libdl_so(&cfg);
                if let Some((_, ref bytes)) = found {
                    // 6-Z268: persist the win — next boot skips the whole
                    // pipeline.
                    let _ = std::fs::create_dir_all(format!("{}/cache", cfg.data_dir));
                    let _ = std::fs::write(&libdl_cache_path, bytes);
                    info!(
                        "[KR64] 6-Z268: extracted real libdl.so ({} bytes) cached to {} for subsequent boots",
                        bytes.len(),
                        libdl_cache_path
                    );
                }
                found
            }
        }
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

    // Write hook libraries to the GUEST's /dev so the child can use
    // LD_PRELOAD=/dev/libgetpid_hook.so.
    //
    // The library CONTENT was read into memory BEFORE setup_mounts
    // (Step 3.6 above), while host filesystem paths were still
    // accessible. The write TARGET depends on the mode (rootfs_prefix,
    // defined in Step 4.4 above):
    //
    //   * use_namespaces=true (root, pivot_root): rootfs_prefix == "" so
    //     the target is /dev/libgetpid_hook.so — the tmpfs mounted by
    //     setup_mounts, which survives pivot_root and is visible inside
    //     the jail at /dev/ (exactly where LD_PRELOAD expects). /dev/
    //     (tmpfs) is also critical for SELinux: init second stage forks
    //     subcontexts running as u:r:vendor_init:s0, which is DENIED
    //     search on app_data_file directories
    //     (avc denied { search } for name="io.twoyi"
    //      tcontext=u:object_r:app_data_file:s0), while tmpfs is
    //     accessible to ALL domains.
    //
    //   * use_namespaces=false (non-root, NO pivot_root): rootfs_prefix
    //     == cfg.rootfs, so the target is {cfg.rootfs}/dev/…. The bare
    //     "/dev/" would be the HOST's devtmpfs — unwritable for
    //     untrusted_app (EACCES, E2E run 32635971098) and never visible
    //     to the guest. The ptrace interceptor (ptrace_emu
    //     ::translate_path) maps every guest /dev/* access onto
    //     {rootfs}/dev/*, and the linker's openat()s are intercepted,
    //     so LD_PRELOAD=/dev/libgetpid_hook.so resolves to exactly this
    //     staged file (previously the hook went to the HOST /dev →
    //     EACCES, {rootfs}/dev stayed empty, and the translated init's
    //     linker died: "CANNOT LINK EXECUTABLE … library
    //     /dev/libgetpid_hook.so not found" → exit(1) after 427 ptrace
    //     iterations — the aosp3 blocker).
    //
    // If a library was not found in the pre-pivot read, the
    // corresponding Option is None and we skip the write (the error
    // was already logged by find_and_read_hook_library with the full
    // list of checked paths). LD_PRELOAD will still reference the path
    // — the child will log "libgetpid_hook.so NOT found at staged /dev
    // path" and init will crash, but with clear diagnostics.
    let dev_stage_dir = format!("{}/dev", rootfs_prefix);

    // ---------------------------------------------------------------
    // 6-Z215: native-guest detection (computed once, used by BOTH the
    // libdl staging below and the /dev library-symlink farm below).
    //
    // TRUE when the guest rootfs ships its own libc.so whose ELF
    // e_machine matches the HOST's bionic — i.e. the guest executes as
    // a REAL same-machine process (arm64-on-arm64) with no binfmt
    // runner in between. In that mode the ROM's OWN bionic must win
    // over the host's (the host's libc/libdl are private-ABI
    // incompatible with the ROM's linker even though the machine
    // matches — see the 6-Z215 block at the /dev farm for the full
    // root-cause analysis).
    // ---------------------------------------------------------------
    let native_guest =
        !cfg.boot_recovery && !cfg.use_namespaces && guest_bionic_is_native(&rootfs_prefix);
    if let Err(e) = std::fs::create_dir_all(&dev_stage_dir) {
        warning!(
            "[KR64] PARENT: failed to create dev dir {} for hook libraries: {} (errno={})",
            dev_stage_dir,
            e,
            e.raw_os_error().unwrap_or(0)
        );
    }
    if let Some((src, content)) = &hook_lib_getpid {
        write_hook_library_to_dev(
            "libgetpid_hook.so",
            src,
            content,
            &format!("{}/libgetpid_hook.so", dev_stage_dir),
        );
    }
    if let Some((src, content)) = &hook_lib_loader {
        write_hook_library_to_dev(
            "libtwoyi_loader_shlib.so",
            src,
            content,
            &format!("{}/libtwoyi_loader_shlib.so", dev_stage_dir),
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
        // (a) TWRP path: /sbin/libtwrp_fb_hook.so (the init.rc setenv
        //     LD_PRELOAD target — the ptrace emulator translates the
        //     path at runtime).
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
        // (b) 6-Z216 AOSP-layout path: ALSO stage it at /dev/ so the
        //     env LD_PRELOAD chain (getpid_hook:loader_shlib:fb_hook)
        //     resolves it for every guest process, including the
        //     /system/bin/recovery service that the init.rc patch never
        //     touches. write_hook_library_to_dev is idempotent (plain
        //     overwrite) so restaging on every boot is safe.
        let fb_hook_dev_dst = format!("{}/libtwrp_fb_hook.so", dev_stage_dir);
        write_hook_library_to_dev("libtwrp_fb_hook.so", src, content, &fb_hook_dev_dst);
    }

    // ---------------------------------------------------------------
    // Step 4.6.1: write the REAL libdl.so (extracted in Step 3.7 before
    // pivot_root) to /dev/libdl.so on the tmpfs.
    //
    // This is the CRITICAL fix for the linker64 segfault at offset
    // 0xaf174 (faulting address 0x86 = NULL soinfo). The hook
    // libraries' DT_NEEDED:libdl.so (LIBC version) was NOT satisfied
    // by the 5848-byte bootstrap stub at both
    // /system/lib64/libdl.so AND /apex/com.android.runtime/lib64/bionic/libdl.so.
    //
    // The LD_LIBRARY_PATH (built in Step 5 below) is modified to put
    // /dev/ FIRST — so the linker finds /dev/libdl.so (the real one
    // we extracted from the APEX ext4 image) before falling back to
    // the stub at /apex/com.android.runtime/lib64/bionic/libdl.so.
    //
    // The bytes were extracted by `apex_extract::find_real_libdl_so`
    // BEFORE setup_mounts (while host paths like
    // /system/apex/com.android.runtime.apex were accessible). Now,
    // AFTER setup_mounts (when /dev/ is the tmpfs), we write them to
    // /dev/libdl.so — exactly the same pattern as the hook libraries
    // above (write to /dev/ tmpfs which survives pivot_root).
    //
    // If extraction failed (e.g. loop device not available, .apex
    // missing, or ext4 driver doesn't support the image), `real_libdl`
    // is None and we skip the write. The LD_LIBRARY_PATH change (Step 5)
    // still prepends /dev/ — that's safe because if /dev/libdl.so
    // doesn't exist, the linker just falls through to the next entry
    // (/apex/com.android.runtime/lib64/bionic/libdl.so = the stub).
    //
    // 6-Z215: NATIVE-GUEST OVERRIDE — if the ROM ships its OWN libdl.so
    // ({rootfs}/system/lib64/libdl.so, e.g. Lineage 22.2 ships the real
    // Android-15 13,896-byte libdl) AND the guest is native-arch, stage
    // a RELATIVE symlink /dev/libdl.so -> ../system/lib64/libdl.so
    // instead of the host-extracted bytes. The host's libdl is built for
    // the HOST's bionic version; the ROM's linker64 must see ITS OWN
    // libdl (the linker implements dlopen/dlsym internally; libdl.so
    // only carries the symbol entries). Mixing the host's libdl with the
    // ROM's linker is the same private-ABI mismatch class as the libc.so
    // mismatch that killed lineage-22.2-sailfish (r25 run 33261943269).
    // The relative target keeps the resolution inside the rootfs (§8:
    // never an absolute host path).
    //
    // ── 6-Z233: GUEST-RAMDISK libdl preference (capricorn + merlin
    // class, CI runs 33305049683 / 33305055239) ──
    //
    // TWRP recoveries ship their OWN libdl.so in the ramdisk /sbin
    // (capricorn: 5968-byte aarch64 ELF; merlin: 10064-byte EM_ARM
    // ELF32). The /dev/libdl.so slot existed ONLY for the host-extracted
    // copy: the APK asset is still the 5848-byte "PLAC" PLACEHOLDER
    // (never replaced — magic PLAC, rejected by is_real_libdl) and the
    // APEX extraction pipeline needs CAP_MKNOD + CAP_SYS_ADMIN + the
    // loop driver (all unavailable on redroid, 5-L/5-N/5-O/5-P/5-U).
    // Result: "real libdl.so NOT extracted" → the guest linker resolved
    // DT_NEEDED:libdl.so against the 5848-byte bootstrap stub →
    // "EXPECT linker64 segfault at 0xaf174" → CANNOT LINK / SIGSEGV →
    // boot dead for BOTH devices.
    //
    // Generic fix (§9/§22: resolve from ACTUAL guest contents): when the
    // guest's own ramdisk ships a libdl.so whose e_machine MATCHES the
    // guest recovery binary's, COPY its bytes into /dev/libdl.so (the
    // same write path as the hook libs). The guest linker then sees ITS
    // OWN bionic generation's libdl — the private-ABI-safe choice (the
    // linker implements dlopen/dlsym internally; libdl only carries the
    // symbol table, and a same-bionic copy is correct by construction).
    // Bounded: only the /dev/libdl.so slot, only on an exact e_machine
    // match, COPY not symlink (§6: no namespace leaks).
    //
    // Priority order for the /dev/libdl.so slot becomes:
    //   1. 6-Z215 ROM /system/lib64 symlink (native-guest full-ROM mode)
    //   2. 6-Z233 guest ramdisk sbin/libdl.so (ABI-matched COPY)
    //   3. host APK asset (when the asset is the REAL library)
    //   4. APEX extraction (legacy fallback)
    let native_guest_libdl = format!("{}/system/lib64/libdl.so", rootfs_prefix);
    let native_guest_libdl_ok = native_guest && std::path::Path::new(&native_guest_libdl).is_file();
    // 6-Z233: set when the guest's OWN ramdisk libdl was staged into
    // /dev/libdl.so — the host-asset/APEX write below must NOT clobber it.
    let mut z233_staged = false;
    if !native_guest_libdl_ok {
        let guest_machine = elf_machine(&format!("{}/sbin/recovery", rootfs_prefix))
            .or_else(|| elf_machine(&format!("{}/sbin/libc.so", rootfs_prefix)));
        let candidates = [
            format!("{}/sbin/libdl.so", rootfs_prefix),
            format!("{}/system/lib64/libdl.so", rootfs_prefix),
            format!("{}/system/lib/libdl.so", rootfs_prefix),
        ];
        'z233: for cand in &candidates {
            if !std::path::Path::new(cand).is_file() {
                continue;
            }
            let cand_machine = elf_machine(cand);
            // ABI guard: an e_machine mismatch would recreate the 6-Z226
            // wrong-ABI class inside the guest's own library slot. When
            // the guest machine is unknown, accept the candidate only if
            // its own class parses cleanly (better than the stub either
            // way — the ramdisk copy IS the guest's own bionic era).
            let abi_ok = match (guest_machine, cand_machine) {
                (Some(g), Some(c)) => g == c,
                (Some(_), None) => false,
                (None, Some(_)) => true,
                (None, None) => false,
            };
            if !abi_ok {
                info!(
                    "[KR64] 6-Z233: skipping {} (e_machine {:?} != guest {:?})",
                    cand, cand_machine, guest_machine
                );
                continue;
            }
            match std::fs::read(cand) {
                Ok(bytes) => {
                    info!(
                        "[KR64] 6-Z233: staging guest's own {} ({} bytes, e_machine {:?}) to {}/libdl.so — real libdl from the guest ramdisk wins over the 5848-byte host stub",
                        cand,
                        bytes.len(),
                        cand_machine,
                        dev_stage_dir
                    );
                    write_hook_library_to_dev(
                        "libdl.so",
                        cand,
                        &bytes,
                        &format!("{}/libdl.so", dev_stage_dir),
                    );
                    z233_staged = true;
                    break 'z233;
                }
                Err(e) => {
                    warning!(
                        "[KR64] 6-Z233: failed to read {}: {} — trying next candidate",
                        cand,
                        e
                    );
                }
            }
        }
    }
    if native_guest_libdl_ok {
        let link_path = format!("{}/libdl.so", dev_stage_dir);
        let target = "../system/lib64/libdl.so";
        let staged = match std::fs::symlink_metadata(&link_path) {
            Ok(md) if md.file_type().is_symlink() => match std::fs::read_link(&link_path) {
                Ok(t) if t == std::path::Path::new(target) => true,
                // A stale /dev/libdl.so symlink (e.g. pointing at a
                // previous host-backed target) — replace it.
                _ => match std::fs::remove_file(&link_path) {
                    Ok(()) => std::os::unix::fs::symlink(target, &link_path).is_ok(),
                    Err(_) => false,
                },
            },
            // A real file (previous host-extracted libdl write) —
            // replace it with the ROM's own symlink.
            Ok(_) => match std::fs::remove_file(&link_path) {
                Ok(()) => std::os::unix::fs::symlink(target, &link_path).is_ok(),
                Err(_) => false,
            },
            Err(_) => std::os::unix::fs::symlink(target, &link_path).is_ok(),
        };
        if staged {
            info!(
                "[KR64] 6-Z215: staged {} -> {} (ROM's own libdl.so wins over the host-extracted copy — native-guest bionic-first)",
                link_path, native_guest_libdl
            );
        } else {
            warning!(
                "[KR64] 6-Z215: failed to stage {} -> {} — falling back to the host-extracted libdl.so",
                link_path,
                native_guest_libdl
            );
        }
    }
    let real_libdl_still_needed = if native_guest_libdl_ok || z233_staged {
        None
    } else {
        real_libdl.as_ref()
    };
    if let Some((src, content)) = real_libdl_still_needed {
        info!(
            "[KR64] PARENT: writing real libdl.so ({} bytes, source: {}) to {}/libdl.so (guest /dev staging dir)",
            content.len(),
            src,
            dev_stage_dir
        );
        write_hook_library_to_dev(
            "libdl.so",
            src,
            content,
            &format!("{}/libdl.so", dev_stage_dir),
        );
    } else {
        if !z233_staged {
            warning!(
                "[KR64] PARENT: real libdl.so NOT extracted and no ABI-matched guest ramdisk copy — guest init will use the 5848-byte stub at /apex/com.android.runtime/lib64/bionic/libdl.so and may crash at offset 0xaf174 in linker64 (5-K's diagnosis; 6-Z233)"
            );
        }
    }

    // ---------------------------------------------------------------
    // Step 4.6.2 (Task 6-Z92, narrowed by 6-Z93): stage RELATIVE
    // library symlinks {rootfs}/dev/<lib>.so ->
    // ../system/lib64/<lib>.so for every *.so in the ROM's
    // {rootfs}/system/lib64/ that the HOST cannot already provide
    // (plus {rootfs}/dev/<lib>.so -> ../system/lib/<lib>.so for the
    // 32-bit tree if the ROM ships one).
    //
    // ROOT CAUSE (aosp4, E2E run 32638925300, commit c7cf36a): the
    // translated arm64 init — running as the host's binfmt_misc
    // ndk_translation_program_runner — linked a 55-library / 2.4 MB
    // DT_NEEDED closure (220 anonymous-mmap content injections, hooks
    // ACTIVE) and then died exactly ONE library short:
    //
    //   CANNOT LINK EXECUTABLE
    //   "/system/bin/ndk_translation_program_runner_binfmt_misc_arm64":
    //   library "libandroidicu.so" not found: needed by
    //   /system/lib64/libharfbuzz_ng.so
    //
    // The ROM HAS libandroidicu.so at {rootfs}/system/lib64/ — but the
    // linker's search PREVIOUSLY resolved /system/** against the HOST
    // (translate_path passed /system through). Since the 6-Z185 sandbox
    // fix, /system/lib{,64}/** translate into the rootfs when the ROM
    // ships them (the ROM-copy branch) and only fall back to the host
    // for lib subtrees the ROM lacks — ndk_translation's runner keeps
    // finding its HOST API-30 arm64 libs through that fallback.
    //
    // THE FIX: LD_LIBRARY_PATH's FIRST entry is /dev (see the child env
    // build in Step 8), and the interceptor maps the guest's /dev/*
    // onto {rootfs}/dev/*. So we stage a RELATIVE symlink
    // {rootfs}/dev/<lib>.so -> ../system/lib64/<lib>.so for every ROM
    // library. The kernel resolves the relative target within the
    // rootfs at open time:
    //
    //   openat("/dev/libX.so")            (linker probe, LD_LIBRARY_PATH[0])
    //     → intercepted → {rootfs}/dev/libX.so
    //     → kernel follows the relative symlink
    //     → {rootfs}/system/lib64/libX.so
    //     → the mmap2 rewrite+inject machinery serves the ROM's REAL
    //       8.1 library content.
    //
    // Any host-ABSENT DT_NEEDED name the linker probes via
    // LD_LIBRARY_PATH[0]=/dev now resolves to the ROM's own copy —
    // including the missing libandroidicu.so. (RELATIVE targets are
    // mandatory: an absolute /system/lib64/... target would resolve on
    // the HOST filesystem — the exact bug class the binderfs /dev/binder
    // symlinks already avoid by using "binderfs/binder"-style relative
    // targets.)
    //
    // CONFLICT RULE: if {rootfs}/dev/<name> already exists it WINS and
    // is NEVER overwritten — the staged hooks (libgetpid_hook.so,
    // libtwoyi_loader_shlib.so, the real libdl.so written above) must
    // keep priority over the ROM's same-named libraries. If the
    // existing entry is already exactly the right symlink, we count it
    // as staged (restaging is idempotent).
    //
    // OVERREACH (Task 6-Z93, E2E run 32643008745): staging EVERY ROM
    // library backfired, because LD_LIBRARY_PATH[0]=/dev is probed
    // FIRST for EVERY DT_NEEDED name — including names the
    // ndk_translation RUNNER must resolve NATIVELY (it is a HOST
    // x86_64 bionic executable). At iteration 527 the runner's own
    // linker opened /dev/liblog.so → the relative symlink handed it
    // the ROM's ARM64 ELF, and host bionic does NOT skip
    // incompatible-ELF candidates — machine mismatch is FATAL:
    //
    //   CANNOT LINK EXECUTABLE
    //   "/system/bin/ndk_translation_program_runner_binfmt_misc_arm64":
    //   "…/rootfs/system/lib64/liblog.so" is for EM_AARCH64 (183)
    //   instead of EM_X86_64 (62)
    //
    // → exit(1) EARLIER than before the farm existed (iter 527 vs
    // 2,353 — it died at the FIRST probe). THE NARROWING: only stage a
    // name the HOST CANNOT provide itself. If /system/lib64/<name>,
    // /system/lib/<name> or /apex/com.android.runtime/lib64/<name>
    // exists on the HOST, the runner's linker resolves that name
    // through its own x86_64 trees (LD_LIBRARY_PATH[0] miss → next
    // entry → host /system + /apex) — exactly how it got 55 libraries
    // deep before. Only host-ABSENT names (libandroidicu.so) get the
    // /dev symlink to the ROM's ARM64 copy, which the mmap2
    // rewrite+inject machinery then translates. Path::exists is
    // best-effort: any error (perm, ENOENT, …) reads as "absent" →
    // stage the symlink (the safe default for a host-lacking name).
    //
    // GATING: normal (AOSP) boot only. TWRP (boot_recovery=true) does
    // not need it — its init is statically linked and its
    // LD_LIBRARY_PATH is /sbin:/system/lib. Root mode
    // (use_namespaces=true) does not need it either: after pivot_root
    // the guest's /system/lib64 IS the ROM's, so the linker already
    // finds the ROM's libs without any /dev indirection.
    //
    // ----------------------------------------------------------------
    // 6-Z215: GUEST-BIONIC-FIRST POLICY FOR NATIVE-ARCH GUESTS.
    //
    // The 6-Z93 host-presence filter above made sense ONLY for the
    // x86_64-host binfmt_misc runner mode: the runner is a HOST x86_64
    // bionic executable whose linker fatally rejects the ROM's ARM64
    // ELFs (EM_AARCH64 != EM_X86_64), so host-present names had to be
    // left to the host's own trees.
    //
    // On an arm64 host (redroid E2E, or a real arm64 device running the
    // guest natively) there IS no translation runner: the guest init is
    // a REAL arm64 process, and the host bionic is arm64 too — so the
    // host-presence filter instead hands the guest the HOST's bionic
    // (libc.so from the host's /apex/com.android.runtime) even when the
    // ROM ships its OWN bionic in {rootfs}/system/lib64/. Result (r25,
    // run 33261943269, lineage-22.2-sailfish): the guest's Android-15
    // linker64 + init linked against the HOST's Android-14 libc.so +
    // libdl.so (maps: device 00:34 host /apex) while libc++/liblog/
    // libbase came from the ROM (device 08:01 rootfs) → private-ABI
    // mismatch → SIGSEGV si_addr=0x0 inside libc at libc+0x5fb20 during
    // early property-area init. The ROM's own libc.so (1,114,608 bytes,
    // Android 15) was bypassed because LD_LIBRARY_PATH[1] is
    // /apex/com.android.runtime/lib64/bionic which appears BEFORE
    // /system/lib64 (position 7).
    //
    // THE FIX (generic, §22/§23/§37): when the guest rootfs ships its
    // own libc.so whose ELF e_machine matches the HOST's bionic e_machine
    // (i.e. guest processes execute natively against the host's trees —
    // no binfmt runner in between), stage EVERY ROM library (including
    // host-present names like libc.so/libm.so/libdl.so) into /dev via
    // relative symlinks, so LD_LIBRARY_PATH[0]=/dev resolves them to the
    // ROM's own copies BEFORE any /apex entry can hit the host. The
    // host's trees remain the fallback for names the ROM does not ship.
    // In runner mode (guest ELF machine != host bionic machine) the
    // 6-Z93 filter is kept unchanged, so the x86_64 full-Android mode
    // cannot regress.
    if native_guest {
        info!(
            "[KR64] 6-Z215: native-arch guest with its own bionic detected — /dev library staging will prefer the ROM's own libc.so/libm.so/libdl.so over the host's (guest-first, runner mode filter disabled)"
        );
    }
    if cfg.boot_recovery {
        info!("[KR64] TWRP boot: skipping /dev ROM library symlinks (TWRP init is statically linked; LD_LIBRARY_PATH=/sbin)");
    } else if cfg.use_namespaces {
        info!("[KR64] PARENT: root mode — skipping /dev ROM library symlinks (pivot_root already serves the ROM's /system/lib64 to the guest)");
    } else {
        let mut staged_lib64 = 0usize;
        let mut staged_lib32 = 0usize;
        let mut kept_existing = 0usize;
        let mut failures = 0usize;
        let mut rom_libs = 0usize;
        let mut host_present_skipped = 0usize;
        // Task 6-Z93 host-presence filter: the host trees the
        // ndk_translation runner resolves names from natively (its own
        // x86_64 bionic /system trees + the runtime APEX). Path::exists
        // is best-effort — any error counts as "absent", which stages
        // the symlink (safe default: the host lacks the name).
        let host_provides = |name: &str| {
            Path::new("/system/lib64").join(name).exists()
                || Path::new("/system/lib").join(name).exists()
                || Path::new("/apex/com.android.runtime/lib64")
                    .join(name)
                    .exists()
        };
        // (guest-relative source dir, is-the-64-bit-tree). The symlink
        // target is always "../<source dir>/<name>" so the kernel
        // resolves it inside the rootfs at open time.
        for (src_subdir, is_lib64) in [("system/lib64", true), ("system/lib", false)] {
            let src_dir = format!("{}/{}", rootfs_prefix, src_subdir);
            let entries = match std::fs::read_dir(&src_dir) {
                Ok(entries) => entries,
                Err(e) => {
                    // The 32-bit tree is commonly absent on 64-bit-only
                    // ROMs — silently skip it. A missing lib64 tree is
                    // worth a warning (the whole fix stages nothing then).
                    if is_lib64 {
                        warning!(
                            "[KR64] PARENT: read_dir {} failed: {} — no ROM library symlinks staged (linker /dev probes will fall through to the host trees)",
                            src_dir,
                            e
                        );
                    }
                    continue;
                }
            };
            for entry in entries.flatten() {
                let name = match entry.file_name().into_string() {
                    Ok(name) => name,
                    Err(_) => continue, // non-UTF-8 name — nothing to link
                };
                if !name.ends_with(".so") {
                    continue;
                }
                // Only REGULAR files: symlinks already inside the ROM
                // tree (e.g. vendor-redirected libs) are skipped so the
                // /dev farm never chains link→link.
                match entry.file_type() {
                    Ok(ft) if ft.is_file() => {}
                    _ => continue,
                }
                rom_libs += 1;
                // Task 6-Z93: names the HOST can resolve itself must NOT
                // be symlinked into /dev — see the OVERREACH note above
                // (the EM_AARCH64-vs-EM_X86_64 fatal on /dev/liblog.so).
                //
                // 6-Z215 EXCEPTION: in native-guest mode the guest is a
                // REAL same-machine process (no binfmt runner) and MUST
                // get the ROM's own bionic — the host's is ABI-incompatible
                // even though the machine matches (r25 lineage SIGSEGV).
                // So the host-presence filter is skipped entirely and
                // every ROM library wins at LD_LIBRARY_PATH[0]=/dev.
                if host_provides(&name) && !native_guest {
                    host_present_skipped += 1;
                    continue;
                }
                let link_path = format!("{}/{}", dev_stage_dir, name);
                let target = format!("../{}/{}", src_subdir, name);
                match std::os::unix::fs::symlink(&target, &link_path) {
                    Ok(()) => {
                        if is_lib64 {
                            staged_lib64 += 1;
                        } else {
                            staged_lib32 += 1;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        // NEVER overwrite: a real staged file (the
                        // hooks / real libdl.so) or a foreign symlink at
                        // {rootfs}/dev/<name> wins over the ROM's copy.
                        let existing_is_right_symlink = match std::fs::symlink_metadata(&link_path)
                        {
                            Ok(md) if md.file_type().is_symlink() => {
                                match std::fs::read_link(&link_path) {
                                    Ok(t) => t == Path::new(&target),
                                    Err(_) => false,
                                }
                            }
                            _ => false,
                        };
                        if existing_is_right_symlink {
                            // Idempotent restage — already exactly what
                            // we wanted to create.
                            if is_lib64 {
                                staged_lib64 += 1;
                            } else {
                                staged_lib32 += 1;
                            }
                        } else {
                            kept_existing += 1;
                        }
                    }
                    Err(e) => {
                        failures += 1;
                        // One warning for the first failure is enough —
                        // the farm is ~hundreds of links and per-link
                        // spam would drown the bootlog.
                        if failures == 1 {
                            warning!(
                                "[KR64] PARENT: symlink {} -> {} failed: {} (further symlink failures suppressed)",
                                link_path,
                                target,
                                e
                            );
                        }
                    }
                }
            }
        }
        let mut summary = format!(
            "[KR64] PARENT: staged {} library symlinks into {} -> ../system/lib64 (host-absent only, of {} ROM libs; LD_LIBRARY_PATH[0] resolution of the ROM's own libs)",
            staged_lib64, dev_stage_dir, rom_libs
        );
        if staged_lib32 > 0 {
            summary = format!("{}; +{} -> ../system/lib (32-bit)", summary, staged_lib32);
        }
        if host_present_skipped > 0 {
            summary = format!(
                "{} [{} host-present names left to the host's x86_64 trees]",
                summary, host_present_skipped
            );
        }
        if kept_existing > 0 || failures > 0 {
            summary = format!(
                "{} [{} pre-existing /dev entries kept — hooks win; {} failures]",
                summary, kept_existing, failures
            );
        }
        info!("{}", summary);

        // 6-Z218b: native-guest bootstrap-bionic staging. The bionic
        // linker's special libc lookup for init resolves libc.so from
        // the hard-coded /system/lib64/bootstrap/ path WITHOUT
        // consulting LD_LIBRARY_PATH (so the /dev farm above cannot
        // serve it). When the guest rootfs lacks a bootstrap dir, that
        // lookup leaks to the HOST's bootstrap libc (r25 lineage:
        // Android-14 host libc inside Android-15 init → SIGSEGV).
        // Stage the ROM's own bionic into the bootstrap dirs so the
        // lookup resolves inside the guest rootfs.
        let (boot_staged, boot_already) = stage_guest_bootstrap_bionic(&rootfs_prefix);
        if boot_staged > 0 || boot_already > 0 {
            info!(
                "[KR64] 6-Z218b: staged {} bootstrap-bionic symlinks ({} already present) into {{rootfs}}/system/lib64/bootstrap + {{rootfs}}/apex/com.android.runtime/lib64/bootstrap — the guest linker's bootstrap libc lookup now resolves to the ROM's own bionic",
                boot_staged, boot_already
            );
        }
    }

    // 6-Z230: stage host-runtime copies of DT_NEEDED libraries the guest
    // ramdisk doesn't ship (cherry: libcrypto.so — TWRP 3.7 builds that
    // expect the ROM's /system/lib64 to provide it). MUST run in the
    // PARENT before pivot_root (host paths are still readable), and
    // BEFORE the guest execs anything dynamic. Runs for every boot
    // mode: any dynamic guest benefits; static-only guests parse as
    // no-PT_DYNAMIC and the pass is a cheap no-op.
    //
    // 6-Z236: when ANY host-runtime lib was staged, ALSO stage the
    // bionic FORTIFY-compat shim (the host libs reference __*_chk
    // symbols the guest libc may not export — cherry evidence run
    // 33306474686) and PREPEND it to the recovery LD_PRELOAD chains
    // below (rc setenv + the init env string).
    let mut compat_shim_staged = false;
    {
        let (libs_staged, libs_missing, staged_from_host) = stage_missing_dt_needed(&rootfs_prefix);
        if libs_staged > 0 || !libs_missing.is_empty() {
            info!(
                "[KR64] 6-Z230: staged {} missing DT_NEEDED libs from the host runtime into {{rootfs}}/sbin ({} unresolvable: {:?})",
                libs_staged,
                libs_missing.len(),
                libs_missing
            );
        }
        if staged_from_host {
            // The shim source is bitness-matched against the same ABI
            // anchor the DT_NEEDED staging used (sbin/recovery, falling
            // back to sbin/libc.so — §9: resolve from actual guest
            // contents, never guess).
            let abi_anchor = elf_machine(&format!("{}/sbin/recovery", rootfs_prefix))
                .or_else(|| elf_machine(&format!("{}/sbin/libc.so", rootfs_prefix)));
            if let Some(machine) = abi_anchor {
                compat_shim_staged = stage_bionic_compat_shim(&cfg, &rootfs_prefix, machine);
            }
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
        // NOTE: this loop only runs when use_namespaces=true (see the
        // `if !cfg.use_namespaces` skip above), where rootfs_prefix == ""
        // and dev_stage_dir == "/dev" — i.e. exactly the staged paths.
        // Built from dev_stage_dir so the relabel targets can never
        // drift from the staging targets above.
        let staged_hook_libs = [
            format!("{}/libgetpid_hook.so", dev_stage_dir),
            format!("{}/libtwoyi_loader_shlib.so", dev_stage_dir),
            format!("{}/libdl.so", dev_stage_dir),
            format!("{}/sbin/libtwrp_fb_hook.so", rootfs_prefix),
        ];
        for lib_path in &staged_hook_libs {
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
        // Guest-visible /dev/twoyi-bin. use_namespaces=true → rootfs_prefix
        // == "" → the post-pivot_root tmpfs /dev/twoyi-bin (unchanged).
        // use_namespaces=false → {cfg.rootfs}/dev/twoyi-bin: the bare
        // "/dev/twoyi-bin" would be the HOST's /dev, which is unwritable
        // for untrusted_app (E2E run 32635971098: every /dev/twoyi-bin/*
        // service copy ENOENT'd because the dir was never created there).
        // The interceptor maps the guest's /dev/twoyi-bin/* execs onto
        // {rootfs}/dev/twoyi-bin/*, so staging under rootfs_prefix/dev
        // makes those exec-redirect targets resolvable.
        let dev_bin_dir = format!("{}/dev/twoyi-bin", rootfs_prefix);
        let _ = std::fs::create_dir_all(&dev_bin_dir);
        let _ = std::fs::set_permissions(&dev_bin_dir, std::fs::Permissions::from_mode(0o755));

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

                // 6-Z184 AUDIT FIX (agent 6): this fallback used to
                // symlink("/dev/binder" -> "/dev/binder") — with
                // rootfs_prefix == "" (the only mode this branch runs
                // in, root mode post-pivot_root) link_path and target
                // were IDENTICAL, producing a self-referential symlink
                // that made every guest open("/dev/binder") fail with
                // ELOOP. The host's /dev is unreachable by absolute
                // path after pivot_root anyway — remove the broken
                // fallback and say so; the binder proxy socket path
                // (binder.rs) is the working mechanism in all modes.
                info!("[KR64] PARENT: binderfs mount failed — the binder proxy socket (created separately) remains the guest's binder path");
            }
        }
    } // end if cfg.use_namespaces (binderfs mount)

    // Pre-create directories that init and services expect to exist.
    // These are created in the rootfs so init's mkdir commands succeed.
    //
    // 6-Z187 ("MAKE IT ONLY SHOW GUESTS ONLY AND NOTHING ELSE"): in TWRP
    // mode (boot_recovery) the Android-guest compatibility dirs are NO
    // LONGER created — they are visible in TWRP's File Manager root as
    // folders a real recovery would never have (metadata, linkerconfig,
    // data_mirror, mnt/secure|asec|obb|user|installer|androidwritable|
    // pass_through, acct/uid_*…), which the user correctly reads as host
    // noise. TWRP's own init creates whatever mountpoints it needs inside
    // the writable rootfs. Only the partition-probe dirs (cache +
    // dev/block/by-name, needed by fstab handling before init runs) stay.
    {
        use std::os::unix::fs::PermissionsExt;
        let dirs: &[&str] = if cfg.boot_recovery {
            &["cache", "dev/block", "dev/block/by-name", "dev/block/dm-5"]
        } else {
            &[
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
            ]
        };
        for dir in dirs {
            let path = format!("{}/{}", rootfs_prefix, dir);
            let _ = std::fs::create_dir_all(&path);
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o777));
        }
        info!(
            "[KR64] PARENT: pre-created boot directories in rootfs ({} dirs, {} mode)",
            dirs.len(),
            if cfg.boot_recovery {
                "TWRP guest-only"
            } else {
                "Android guest"
            }
        );
    }

    // ── 6-Z187: TWRP terminal shell + REAL guest symlinks ──────────────
    //
    // The RamdiskImporter stores cpio symlinks as `<name>.symlink` text
    // sidecars (Java cannot express them), so {rootfs}/sbin/sh did NOT
    // exist — TWRP's terminal `execl("/sbin/sh", "sh", NULL)` ENOENT'd →
    // "Child processes exited.", and the File Manager showed charger.symlink
    // noise instead of the guest tree. Materialize EVERY sidecar as a real
    // symlink, patch busybox's PT_INTERP to the rootfs linker64, pre-stage
    // busybox under the /sbin/busybox + /sbin/sh keys (the rootfs partition
    // is noexec; the cache staging dir is the executable place), and point
    // busybox-targeted links at the staged copy so even RAW kernel execs of
    // {rootfs}/sbin/sh succeed (belt-and-braces for the PEEK-blind execve
    // ENTRY class — see ptrace_emu's 6-Z170 +1 fallback).
    //
    // Run in BOTH modes: an Android ROM rootfs has the same sidecars. In
    // AOSP mode there is no busybox provisioning (staged_busybox=None).
    {
        let staged_busybox: Option<String> = if cfg.boot_recovery {
            match crate::symlinks::provision_terminal_shell(&cfg.rootfs, &cfg.data_dir) {
                Ok(p) => {
                    info!(
                        "[KR64][symlinks] TWRP terminal shell provisioned: busybox PT_INTERP patched + staged + /sbin/sh registered ({})",
                        p
                    );
                    Some(p)
                }
                Err(e) => {
                    warning!(
                        "[KR64][symlinks] terminal shell provisioning FAILED: {} — terminal may stay dead",
                        e
                    );
                    None
                }
            }
        } else {
            None
        };
        let stats =
            crate::symlinks::materialize_symlink_sidecars(&cfg.rootfs, staged_busybox.as_deref());
        // 6-Z187: legacy in-rootfs artifacts from older installs — remove
        // so the guest File Manager shows the guest tree only.
        let legacy_marker = format!("{}/{}", cfg.rootfs, crate::ptrace_emu::STAGED_EXE_MARKER);
        let _ = std::fs::remove_file(&legacy_marker);
        let legacy_geom = format!("{}/.twoyi-fb-geometry", cfg.rootfs);
        let _ = std::fs::remove_file(&legacy_geom);
        // 6-Z187b: the rootfs-path FILE — the fb_hook's env-independent
        // rootfs source. Run 33119446980: the UI recovery is exec'd by
        // init, whose service env does NOT carry TWOYI_ROOTFS — the hook's
        // via=1 (prefix) open-retry form was unavailable and via=2
        // (path+1, cwd-relative) failed 166x before the cwd fix. The hook
        // reads this file through the tracer-translated absolute path
        // "/dev/.twoyi-rootfs" and caches it, restoring the prefix form
        // for EVERY process regardless of its env.
        {
            let rootfs_file = format!("{}/dev/.twoyi-rootfs", cfg.rootfs);
            if let Err(e) = std::fs::write(&rootfs_file, format!("{}\n", cfg.rootfs)) {
                warning!(
                    "[KR64][symlinks] failed to write {}: {} (hook will rely on env/cwd fallbacks)",
                    rootfs_file,
                    e
                );
            }
        }
        info!(
            "[KR64][symlinks] materialized {} real symlinks from .symlink sidecars (removed {}, skipped {}, busybox-staged={})",
            stats.links_created,
            stats.sidecars_removed,
            stats.skipped,
            staged_busybox.is_some()
        );
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
    // point to /dev/null) with regular files of cfg.width*cfg.height*4
    // bytes. This makes open() succeed and mmap() work naturally, so
    // libminuitwrp's/lineage-minui's graphics_fbdev_init can proceed past
    // the open+mmap stage. The FB ioctls themselves are intercepted by
    // the `libtwrp_fb_hook.so` / twoyi_loader shlib (LD_PRELOAD'd into
    // the recovery process). See `devices::create_twrp_framebuffer` for
    // the full rationale.
    //
    // 6-Z224: NO LONGER gated on cfg.boot_recovery. Run 33279361259
    // (lineage-22.2-sailfish, boot_recovery=false — the per-design
    // RomManager auto-set): the geometry file {rootfs}/dev/.twoyi-fb-
    // geometry was never written (it lives inside create_twrp_framebuffer,
    // behind this gate), the fb hook fell back to 320x640, and minui's
    // mmap failed on the stub-sized fb0 — "cannot open any framebuffer"
    // at t=10.1s, headless boot, UI never reached. AOSP-layout recovery
    // boots (the 6-Z220 preload chain) need the same full-size fb0 +
    // geometry file as TWRP boots. The creation is idempotent and
    // harmless for full-Android guest boots (modern Android never opens
    // /dev/graphics/fb0 — its graphics go through the host renderer).
    if let Err(e) =
        devices::create_twrp_framebuffer(&rootfs_prefix, cfg.width as u32, cfg.height as u32)
    {
        warning!(
            "[KR64] PARENT: failed to create TWRP framebuffer: {} (recovery will crash in libminuitwrp.so)",
            e
        );
    }
    // 6-Z171c: /dev/ashmem + /dev/pmsg0 stand-ins (regular files) so
    // minui's opens succeed; the hook fakes the ASHMEM_* ioctls.
    //
    // 6-Z224b: NO LONGER gated on cfg.boot_recovery. Run 33282961791
    // (orangefox-R12.0-lavender, boot_recovery=false — per-design): with
    // the capset + fb0 fixes in, OrangeFox's libc++ reached
    // ashmem_create_region("/dev/ashmem") during code-cache setup and
    // got ENOENT — "[glog F/abort] Creating code cache, ashmem_create_region
    // failed" -> abort() -> splash "Loading resources..." was the last
    // sign of life. Every modern AOSP-layout recovery's libc++/art maps
    // ashmem for JIT/code-cache regions, exactly like a TWRP boot does.
    // The stand-ins are empty regular files (the ASHMEM_* ioctls are
    // faked by the hooks) — harmless in every boot mode, idempotent.
    if let Err(e) = devices::create_twrp_misc_devs(&rootfs_prefix) {
        warning!(
            "[KR64] PARENT: failed to create TWRP misc devs (/dev/ashmem,/dev/pmsg0): {}",
            e
        );
    }

    // RECOVERY INPUT: pre-create {rootfs}/dev/input/event0 + event1 as
    // EMPTY regular files (0644). minui's /dev/input scan (readdir +
    // fstatat probes) needs openable "event*" names to exist; when one
    // of them is subsequently OPENED, the fb hook's input bridge
    // intercepts the open and hands back the connected touch-events
    // socket instead — so the files themselves never need contents.
    //
    // This REPLACES the fb hook constructor's raw staging (mkdir_raw +
    // openat(O_CREAT) issued from inside the guest): those raw syscalls
    // passed through the tracer's interception path and corrupted the
    // resume state — KVM run 32649156523: recovery SIGSEGV'd
    // (si_code=128 SI_KERNEL, rip inside the hook's text) ~20s in,
    // BEFORE minui ran; all 600s stayed solid black. Staging here is
    // purely parent-side — no guest syscalls are involved.
    //
    // 6-Z251g: NO LONGER gated on cfg.boot_recovery. The LD_PRELOAD fb
    // hook (which owns the input bridge) loads in EVERY loader-path
    // recovery boot too (OrangeFox/Lineage/AOSP — boot_recovery=false
    // by design since 6-Z209b), and their EventHub found an EMPTY
    // /dev/input: no event* entries -> zero input devices registered ->
    // every tap from the host (real surface touch, debug broadcast,
    // adb input, sendevent) vanished without a trace. Verify runs
    // 33325977950/33327023870: the fox-nav Menu tap never produced a
    // page marker and the guest recovery log has no event0 open at all
    // — the touch pipeline was dead BEFORE the first tap. Same
    // rationale as the 6-Z224 fb0/6-Z224b ashmem un-gating: creation is
    // idempotent and harmless for full-Android guests (EventHub opens
    // the empty regular files, the EVIOCGBIT ioctls ENOTTY, the entry
    // is skipped; the GSI's input flows through the same touch device).
    {
        use std::os::unix::fs::PermissionsExt;
        let input_dir = format!("{}/dev/input", rootfs_prefix);
        if let Err(e) = std::fs::create_dir_all(&input_dir) {
            warning!(
                "[KR64] PARENT: failed to create {} for event probe files: {} (minui's /dev/input scan may find nothing)",
                input_dir, e
            );
        }
        for name in &["event0", "event1"] {
            let path = format!("{}/dev/input/{}", rootfs_prefix, name);
            match std::fs::write(&path, b"") {
                Ok(()) => {
                    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
                }
                Err(e) => {
                    warning!(
                        "[KR64] PARENT: failed to pre-create probe file {}: {} (minui's evdev scan will skip it)",
                        path, e
                    );
                }
            }
        }
        info!(
            "[KR64] PARENT: pre-created /dev/input/event0+event1 probe files (the fb hook's input bridge intercepts their open)"
        );
        // 6-Z96b: also write the ABSOLUTE host path of the touch socket
        // into {rootfs}/dev/.touch-sock — the fb hook's INPUT bridge reads
        // this file (its openat is intercepted + translated to the real
        // file) and connects to that absolute path. Replaces the
        // getenv-based resolution: the ancient bionic linker inside the
        // TWRP recovery binary leaves the hook's weak getenv PLT
        // unresolved (run 32654424163: ncands=1, only the RELATIVE
        // candidate was tried; relative sun_path resolves against the
        // exec'd child's HOST CWD — not the rootfs — so it ENOENT'd).
        {
            let touch_sock = format!("{}/dev/touch-events", cfg.data_dir);
            // 6-Z184 AUDIT FIX (agent 6): rootfs_prefix, not cfg.rootfs —
            // in root/KVM mode (post-pivot_root) the host-absolute
            // cfg.rootfs path does not exist inside the new root, the
            // write ENOENT'd and the fb hook's input bridge never found
            // the hint → no touch in root mode. Every sibling staging
            // step in this block already uses rootfs_prefix.
            let sock_hint = format!("{}/dev/.touch-sock", rootfs_prefix);
            if let Err(e) = std::fs::write(&sock_hint, touch_sock.as_bytes()) {
                warning!("[KR64] PARENT: failed to write {}: {}", sock_hint, e);
            } else {
                info!(
                    "[KR64] PARENT: wrote {} ({} bytes) — the fb hook's input bridge reads it for the absolute socket path",
                    sock_hint,
                    touch_sock.len()
                );
            }
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
    //
    // Task 6-Z98 (run 32663728329, head fb89bfba): this section now runs
    // for ALL boots — NOT just TWRP. The 6-Z97 translate_path mapping
    // open("/dev/kmsg") → {rootfs}/dev/__kmsg__ (ptrace_emu.rs) is
    // mode-independent, and Android 11's first-stage init calls
    // InitKernelLogging() → open("/dev/kmsg", O_WRONLY|O_CLOEXEC) within
    // its first ~20 post-execve syscalls. With the creation still gated
    // behind boot_recovery (aosp15, boot_recovery=false) the translated
    // open hit a file that was never created → ENOENT (-2, "DIAG KLOG fd
    // capture: open() returned -2") → the guest stayed completely mute:
    // no init.rc parse results, no service starts, no property sets in
    // kr64-app-stderr.log. The post-run "failed to copy diagnostic log
    // …/rootfs/dev/__kmsg__: No such file or directory" (×2) confirmed
    // the backing file never existed. Hoisting the gate gives EVERY boot
    // a kmsg backing file so BOTH TWRP init's log_init() and AOSP 11
    // init's InitKernelLogging() write KLOG output somewhere kr64 can
    // mirror + copy out.
    {
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
    // 6-Z220: the rc patcher now runs for BOTH boot modes.
    //   * TWRP mode (boot_recovery=true): legacy /sbin fb-hook chain for
    //     the 32-bit i386 recovery (unchanged behavior — angler guard).
    //   * AOSP-layout mode (boot_recovery=false): the recovery service is
    //     forked by the ROM's own init from /system/bin/recovery. Stock
    //     AOSP init inherits environ, but relying on inheritance through
    //     arbitrary vendor init trees is fragile (any rc-level
    //     setenv/unsetenv churn silently drops the chain). The service
    //     MUST carry the full virtualization stack explicitly: inject
    //     AOSP_SERVICE_PRELOAD_CHAIN via the same init.rc setenv patch.
    //     The stub rc in the x86 emulator rootfs has no `service recovery`
    //     line → the patcher is a no-op there, preserving x86 behavior.
    // 6-Z236: when host-runtime libs were staged (6-Z230), PREPEND the
    // FORTIFY-compat shim to whichever chain applies — bionic loads
    // LD_PRELOAD libs BEFORE DT_NEEDED resolution so the shim's exports
    // satisfy the host libs' __*_chk relocations (cherry class).
    let service_preload_chain: Option<String> = if cfg.boot_recovery {
        if compat_shim_staged {
            Some("/sbin/libbionic_compat.so:/sbin/libtwrp_fb_hook.so".to_string())
        } else {
            None // legacy 32-bit TWRP: /sbin/libtwrp_fb_hook.so
        }
    } else if compat_shim_staged {
        Some(format!(
            "/dev/libbionic_compat.so:{}",
            AOSP_SERVICE_PRELOAD_CHAIN
        ))
    } else {
        Some(AOSP_SERVICE_PRELOAD_CHAIN.to_string())
    };
    patch_twrp_init_rc_recovery_service_in_rootfs(
        &rootfs_prefix,
        cfg.width,
        cfg.height,
        service_preload_chain.as_deref(),
    );

    // 6-Z225: strip `capabilities` service options from every guest .rc
    // (see strip_service_capabilities_options() for the OrangeFox
    // cap_drop_bound/InitFatalReboot evidence chain). Runs in the same
    // staging phase as the recovery-service patch — BEFORE init parses
    // the files.
    let caps_stripped = strip_service_capabilities_options(&rootfs_prefix);
    if caps_stripped > 0 {
        info!(
            "[KR64] PARENT: 6-Z225: stripped capabilities options from {} .rc file(s)",
            caps_stripped
        );
    }

    // 6-Z272c: start the image's OWN health HAL (rc patch: drop `disabled`,
    // start `on late-init`) so the recovery's HIDL battery reader
    // (GetBatteryInfo → get_health_service) finds a live IHealth/default;
    // the HAL's BatteryMonitor then serves the 6-Z272c-pinned sysfs tree.
    let health_enabled = enable_image_health_hal(&rootfs_prefix);
    if health_enabled > 0 {
        info!(
            "[KR64] PARENT: 6-Z272c: image health HAL enabled ({} rc patched)",
            health_enabled
        );
    }

    if cfg.boot_recovery {
        // TWRP BOOT: DELETE /property_contexts ENTIRELY.
        // The TWRP ramdisk's /property_contexts file has `#line 1 "..."`
        // on line 1 (leftover from the AOSP build process). init's parser
        // doesn't understand the `#line` directive, mis-parses the
        // path-string bytes as a pointer (garbage ptr 0x74616433 = ASCII
        // "3dat"), and SIGSEGVs at rip=0x80a0b9e. 6-L's fix (removing
        // just the #line directive) was insufficient — the parser's
        // context field at offset 0x14 stays corrupted, so 6-M NOPed the
        // crash instruction at 0x80a0b9e, but the crash MOVED to 0x80a0bd8
        // (DISPATCHER-FINAL-7: whack-a-mole — MULTIPLE instructions deref
        // the garbage edx/ecx from ctx->field_0x14). 6-N then EMPTIED the
        // file to a single comment line, BUT DISPATCHER-FINAL-8 showed the
        // crash PERSISTS at 0x80a0bd8 even with the emptied file: the
        // context field 0x14 is corrupted BEFORE the parser reads the file
        // content, so even a comment line still triggers fgets → the
        // parser processes it → tries to read ctx->field_0x14->field_at_4
        // → SIGSEGV. The sustainable fix is a DATA fix: DELETE the file
        // ENTIRELY. init's open() returns -ENOENT → the caller (iterating
        // the SEPolicy context table at 0x80ce270) skips this context
        // file → the parser is never invoked → no corrupted-context crash.
        // init tolerates missing property contexts (SELinux property
        // labeling disabled in sandbox — non-fatal for TWRP boot). See
        // `patch_property_contexts_delete` for the full root-cause analysis
        // (DISPATCHER-FINAL-3/4/5/6/7/8). Must run AFTER setup_mounts (so
        // the property_contexts file is reachable on the post-pivot_root
        // root) and BEFORE the guest's init is exec'd in the child below.
        // Idempotent: a no-op if the file is already missing.
        //
        // 6-Z241 ARCH SCOPE (capricorn class, run 33310479551): the
        // deletion is correct ONLY for i386 guests — the parser-overflow
        // bug lives in the i386 TWRP init's libselinux (angler evidence).
        // Android 9 (pi)-based AARCH64 inits REQUIRE the file: their
        // property_init does mkdirat(/dev/__properties__) → openat(
        // /property_contexts) [serializes the contexts into
        // property_info] and LOG(FATAL)s "Failed to initialize property
        // area" when the read fails (trace: mkdirat SUCCESS → openat
        // ENOENT → writev FATAL with NO property_info open between —
        // CreateSerializedPropertyInfo died before serializing). Real
        // devices boot these images WITH the file, so the shipped content
        // is the real-device condition (§22: do not destroy guest content
        // the guest expects). Gate: delete for EM_386 guests only —
        // the i386 parser bug is an ABI-level init implementation
        // distinction, not a device-specific one.
        {
            let guest_init_machine = elf_machine(&format!("{}/init", rootfs_prefix))
                .or_else(|| elf_machine(&format!("{}/sbin/recovery", rootfs_prefix)))
                .or_else(|| elf_machine(&format!("{}/system/bin/recovery", rootfs_prefix)));
            let is_i386_guest = guest_init_machine == Some(EM_386);
            if is_i386_guest {
                patch_property_contexts_delete(&rootfs_prefix);
            } else {
                info!(
                    "[KR64] PARENT: 6-Z241: keeping /property_contexts (guest init is {:?}, not i386 — pi-based aarch64/arm32 inits FATAL 'Failed to initialize property area' when the file is missing; run 33310479551)",
                    guest_init_machine
                );
            }
        }

        // TWRP BOOT: DELETE /file_contexts (+ .homedirs + .local) ENTIRELY.
        // Same root-cause pattern as /property_contexts (6-O): the TWRP
        // ramdisk's /file_contexts has `#line 1 "external/sepolicy/file_contexts"`
        // on line 1. After 6-T's stat64 path-translation fix unmasked the
        // recovery's /file_contexts parser (previously hidden behind the
        // stat64-ENOENT polling loop), the parser's buffer overflow corrupts
        // a std::string this-pointer → SIGSEGV at rip=0x8052f65 (si_addr=
        // 0x696e692f = "/ini"). Deleting the files makes open() return -ENOENT
        // → parser never invoked → no overflow. See `patch_file_contexts_delete`
        // for the full disassembly-driven root-cause analysis (Task 6-V).
        // Must run AFTER setup_mounts + AFTER patch_property_contexts_delete
        // + BEFORE the guest's init is exec'd. Idempotent.
        patch_file_contexts_delete(&rootfs_prefix);
    }

    // TWRP BOOT: DELETE /init.firmware.rc from rootfs.
    //
    // Root cause (Task 6-V diagnostic): init reads /init.firmware.rc
    // (90 bytes: "on boot\n    start intel_fw_props\n\nservice
    // intel_fw_props /sbin/intel_fw_props\n    oneshot\n") right
    // before SIGSEGV at si_addr=0x696e692f (ASCII "ini/"), rip=0x8052f65
    // after 826 iterations. The file defines an Intel firmware property
    // service (/sbin/intel_fw_props) that doesn't exist in the emulator.
    // TWRP init's .rc parser crashes when processing this file — likely a
    // function-pointer/struct-field confusion with the embedded pathname
    // string. The KVM E2E (root+strace) doesn't hit this because the real
    // strace doesn't trigger the same code path. FIX: delete the file.
    // init tolerates missing .rc files (skips them). The intel_fw_props
    // service is Intel-specific and unnecessary in the emulator.
    if cfg.boot_recovery {
        let rc_path = format!("{}/init.firmware.rc", rootfs_prefix);
        if std::path::Path::new(&rc_path).exists() {
            match std::fs::remove_file(&rc_path) {
                Ok(()) => info!(
                    "[KR64] PARENT: DELETED /init.firmware.rc (Intel firmware service — unnecessary in emulator, causes init parser SIGSEGV at si_addr=0x696e692f). Task 6-V diagnostic."
                ),
                Err(e) => info!(
                    "[KR64] PARENT: /init.firmware.rc already absent or failed to delete: {}",
                    e
                ),
            }
        }

        // TWRP BOOT: /init.partlink.rc — Task 6-Z7 fix4: REPLACE with a
        // COMMENT-ONLY file (not empty, not a service line).
        //
        // The .rc parser crashes (SIGSEGV at rip=0x6f722f69) in 3 cases:
        // 1. Original file: parser crashes on "service partlink /sbin/partlink"
        // 2. Deleted file (1efd28c): ENOENT → read_file returns NULL → SIGSEGV
        // 3. Empty file (62566f1): read() returns 0 (EOF) → read_file returns
        //    NULL → "read_file: ERROR RETURNING NULL" KLOG → SIGSEGV
        //
        // FIX: replace with a COMMENT-ONLY file (one line: "# partlink
        // disabled (emulator)"). The parser reads 1 line, sees a comment,
        // skips it, returns non-NULL (no service to start, no crash). init
        // tolerates the missing service definition.
        let partlink_path = format!("{}/init.partlink.rc", rootfs_prefix);
        match std::fs::write(&partlink_path, "# partlink disabled (emulator) — Task 6-Z7 fix4\n") {
            Ok(()) => info!(
                "[KR64] PARENT: REPLACED /init.partlink.rc with COMMENT-ONLY file (Task 6-Z7 fix4: empty file → read_file returns NULL → SIGSEGV; comment-only → parser reads 1 line, skips comment, returns non-NULL, no crash)."
            ),
            Err(e) => warning!(
                "[KR64] PARENT: failed to REPLACE /init.partlink.rc at {}: {} (recovery may SIGSEGV at rip=0x6f722f69 if read_file returns NULL)",
                partlink_path, e
            ),
        }

        // Task 6-Z8: REMOVE the 'import /init.partlink.rc' + 'import /init.firmware.rc'
        // + 'start partlink' lines from /init.rc. Even with the comment-only
        // /init.partlink.rc (6-Z7 fix4), the parser STILL crashes (SIGSEGV at
        // rip=0x6f722f69) after reading the file. The root cause is the init
        // binary's .rc parser corrupting a pointer during the import processing.
        // Removing the import lines from /init.rc means init never tries to
        // parse /init.partlink.rc (or /init.firmware.rc) at all → no crash.
        // The 'start partlink' line in the 'on init' section is also removed
        // (it references the partlink service that no longer exists).
        let init_rc_path = format!("{}/init.rc", rootfs_prefix);
        match std::fs::read_to_string(&init_rc_path) {
            Ok(content) => {
                let mut new_content = String::new();
                let mut removed_count = 0u32;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed == "import /init.partlink.rc"
                        || trimmed == "import /init.firmware.rc"
                        || trimmed == "start partlink"
                    {
                        removed_count += 1;
                        // Skip this line (don't add to new_content).
                        continue;
                    }
                    new_content.push_str(line);
                    new_content.push('\n');
                }
                if removed_count > 0 {
                    match std::fs::write(&init_rc_path, &new_content) {
                        Ok(()) => info!(
                            "[KR64] PARENT: REMOVED {} import/start lines from /init.rc (Task 6-Z8: 'import /init.partlink.rc', 'import /init.firmware.rc', 'start partlink' — prevents the .rc parser SIGSEGV at rip=0x6f722f69 during import processing)",
                            removed_count
                        ),
                        Err(e) => warning!(
                            "[KR64] PARENT: failed to write patched /init.rc at {}: {} (recovery may SIGSEGV at rip=0x6f722f69 during import processing)",
                            init_rc_path, e
                        ),
                    }
                } else {
                    info!(
                        "[KR64] PARENT: /init.rc has no partlink/firmware import lines to remove (Task 6-Z8: idempotent skip)"
                    );
                }
            }
            Err(e) => warning!(
                "[KR64] PARENT: failed to read /init.rc for import removal: {} (recovery may SIGSEGV at rip=0x6f722f69 during import processing)",
                e
            ),
        }
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

        // ── find_property binary patch — WORKAROUND for missing property service ──
        //
        // WORKAROUND (NOT a "suppressed crash"):
        //
        // 5-Z's disassembly DEFINITIVELY identified the SIGSEGV at
        // rip=0x809255d in `find_property()`: the faulting instruction
        // reads `[esi + 0x10]` where `esi = __system_property_area__ +
        // 0x80`, and the global `__system_property_area__` (BSS at
        // 0x8104de0) is NULL → SIGSEGV at NULL + 0x90 (exact match with
        // si_addr=0x90). This reframes 1-A's original "suppressed crash"
        // F.1 flag — the patch was a NECESSARY workaround, not a crash
        // being papered over.
        //
        // 6-A's commit 3eb83d9 made REAL PROGRESS on the proper fix:
        // `/dev/__properties__` now exists + opens + mmaps successfully
        // (mmap returns 0xEE930000, a valid address — see worklog
        // DISPATCHER-UPDATE-6). BUT the global `__system_property_area__`
        // is STILL NULL despite the successful mmap, because
        // `__system_property_area_init()` validates the mapped header
        // and bails BEFORE setting the global — AOSP 5.1 bionic expects
        // a property SERVICE to have written initial property entries
        // via the property socket first, and the kr64 sandboxed
        // environment does NOT provide a property service. The proper
        // fix (a full property service) is a future, much larger effort.
        //
        // PRAGMATIC UNBLOCK: restore the find_property binary patch
        // (commits 9154e59 + 0a4be80 + 5d561cf, removed by f720934).
        // The patch overwrites find_property()'s first 3 bytes with
        // `31 c0 c3` (xor eax,eax; ret) so every property lookup
        // returns NULL (0) immediately. TWRP init tolerates NULL
        // property values (it checks for NULL and uses defaults), so
        // this is safe for early boot. 6-A's `/dev/__properties__`
        // file + vfs.rs OLD-format prop_area remain in place for when
        // a full property service is implemented.
        //
        // Pattern (first 18 bytes of find_property at file offset
        // 0x4a500, virtual address 0x08092500):
        //   55 89 e5 57 56 89 c6 53 8d 64 24 a4 89 55 c4 8b 55 0c
        //   push ebp; mov esp,ebp; push edi; push esi; mov eax,esi;
        //   push ebx; lea -0x5c(esp),esp; mov [ebp-0x3c],edx;
        //   mov edx,[ebp+0xc]
        //
        // Replacement (first 3 bytes only):
        //   31 c0 c3  xor eax,eax; ret
        //
        // IDEMPOTENT: a prior application replaces the first 3 bytes of
        // the pattern with `31 c0 c3`, so the unpatched pattern no
        // longer matches. We detect that case by scanning for the
        // patched signature (`31 c0 c3` + bytes 3..7 of the original
        // pattern, `57 56 89 c6`) and skip the rewrite. This is the
        // SAME idempotency scheme as `patch_twrp_init_klog_init` above.
        {
            let init_path = format!("{}/init", rootfs_prefix);
            match std::fs::read(&init_path) {
                Ok(mut bytes) => {
                    let pattern: &[u8] = &[
                        0x55, 0x89, 0xe5, 0x57, 0x56, 0x89, 0xc6, 0x53, 0x8d, 0x64, 0x24, 0xa4,
                        0x89, 0x55, 0xc4, 0x8b, 0x55, 0x0c,
                    ];
                    let patch: &[u8] = &[0x31, 0xc0, 0xc3]; // xor eax,eax; ret
                    // Patched signature: `31 c0 c3` (patch prologue) + the
                    // unchanged tail bytes 3..7 of the original pattern
                    // (`57 56 89 c6`). Used to detect an already-applied
                    // patch so we skip the rewrite instead of (mis)matching
                    // nothing and logging a false "TWRP version mismatch?".
                    let patched_sig: &[u8] = &[0x31, 0xc0, 0xc3, 0x57, 0x56, 0x89, 0xc6];

                    let already_patched = bytes
                        .windows(patched_sig.len())
                        .any(|w| w == patched_sig);

                    if already_patched {
                        info!(
                            "[KR64] PARENT: /init find_property() already patched (idempotent skip) — property lookups return NULL safely (workaround for missing property service)"
                        );
                    } else {
                        let found_off = bytes
                            .windows(pattern.len())
                            .position(|w| w == pattern);

                        if let Some(off) = found_off {
                            bytes[off] = patch[0];
                            bytes[off + 1] = patch[1];
                            bytes[off + 2] = patch[2];
                            match std::fs::write(&init_path, &bytes) {
                                Ok(()) => info!(
                                    "[KR64] PARENT: patched /init find_property() at file offset {:#x} — replaced first 3 bytes with 'xor eax,eax; ret' (workaround for missing property service; see worklog 5-Z + DISPATCHER-UPDATE-6)",
                                    off
                                ),
                                Err(e) => warning!(
                                    "[KR64] PARENT: patched find_property in memory but failed to write back: {} (init may crash with SIGSEGV at rip=0x809255d)",
                                    e
                                ),
                            }
                        } else {
                            warning!(
                                "[KR64] PARENT: could not find find_property pattern in /init (TWRP version mismatch?) — init may crash with SIGSEGV at rip=0x809255d when accessing properties"
                            );
                        }
                    }
                }
                Err(e) => warning!(
                    "[KR64] PARENT: failed to read /init for find_property patching: {} (init may crash with SIGSEGV at rip=0x809255d)",
                    e
                ),
            }
        }

        // ── selinux-load-failure NOP patch — DEFINITIVE root-cause fix
        // per 6-I's disassembly ──
        //
        // WORKAROUND (NOT a "suppressed crash"):
        //
        // 6-I's disassembly DEFINITIVELY identified the pause() loop root
        // cause. The loop is NOT a wait-for-property-service loop — it's
        // `while(1) pause();` reached after a FAILED SELinux policy load.
        // `selinux_android_load_policy()` fails (mount("selinuxfs")
        // returns negative in the ptrace_emu sandbox) → init takes the
        // failure path → calls `android_reboot(RESTART2, "recovery")` →
        // the reboot is faked/intercepted (returns instead of rebooting)
        // → init falls into `while(1) pause();` forever.
        //
        // This explains why ALL prior fixes failed:
        //   * Return-value tricks (6-D -EINTR, 6-E/6-G -ENOSYS, 6-G
        //     -ETIMEDOUT): init never checks pause's return value (the
        //     loop is unconditional).
        //   * Property service socket stub (6-H): init isn't waiting on
        //     a socket — that hypothesis was wrong.
        //   * 100ms sleep (6-F): reduced the spin rate but the loop is
        //     unconditional, so it just kept spinning.
        //
        // The fix (6-I's Option A): NOP the 6-byte conditional jump at
        // file offset 0x1006 (vaddr 0x08049006):
        //   Original: 0f 88 c3 00 00 00  (js 0x080490cf — failure path)
        //   Patched:  90 90 90 90 90 90  (6 × NOP — never take the
        //                                 failure path)
        //
        // Effect: Even if `selinux_android_load_policy()` returns
        // negative, init does NOT enter the failure path. Falls through
        // to `selinux_init_all_handles()` → `__property_get("ro.boot.
        // selinux")` → `security_setenforce()` (all may fail non-fatally)
        // → `jmp main+0x317` (TWRP recovery boot). The pause loop becomes
        // UNREACHABLE from main().
        //
        // This is a WORKAROUND for the missing selinuxfs mount in the
        // sandboxed environment (the proper fix would be to provide a
        // fake selinuxfs at /sys/fs/selinux/{load,enforce,booleans,
        // ...} — a larger effort that may still hit later failures).
        // Reference: 6-I's disassembly report (worklog entry 6-I).
        //
        // Pattern (8 bytes — 2 bytes of pre-context + 6 bytes of the
        // jump): see [`patch_twrp_init_selinux_load_skip`] above for the
        // full root-cause analysis and pattern details. The function is
        // IDEMPOTENT (skipped if already applied, returns a typed result
        // for NotFound / Skipped / Applied / AlreadyApplied).
        {
            let init_path = format!("{}/init", rootfs_prefix);
            match std::fs::read(&init_path) {
                Ok(mut bytes) => {
                    match patch_twrp_init_selinux_load_skip(&mut bytes) {
                        SelinuxLoadSkipPatchResult::Applied => {
                            match std::fs::write(&init_path, &bytes) {
                                Ok(()) => info!(
                                    "[KR64] PARENT: patched /init selinux-load-failure jump at file offset 0x1006 (vaddr 0x08049006) — replaced `js 0x080490cf` (6 bytes: 0f 88 c3 00 00 00) with 6 × NOP (90 90 90 90 90 90); failure path becomes unreachable, init proceeds to selinux_init_all_handles() → security_setenforce() → TWRP recovery boot (DEFINITIVE root-cause fix per 6-I disassembly)"
                                ),
                                Err(e) => warning!(
                                    "[KR64] PARENT: patched /init selinux-load-failure jump in memory but failed to write back: {} (init will still spin in pause() forever — see worklog 6-I)",
                                    e
                                ),
                            }
                        }
                        SelinuxLoadSkipPatchResult::AlreadyApplied => {
                            info!(
                                "[KR64] PARENT: /init selinux-load-failure jump already NOP'd (idempotent skip) — failure path unreachable (DEFINITIVE root-cause fix per 6-I disassembly)"
                            );
                        }
                        SelinuxLoadSkipPatchResult::Skipped => {
                            // Skip reason already logged inside
                            // `patch_twrp_init_selinux_load_skip`
                            // (currently fires only on aarch64, where the
                            // i386-only byte pattern is irrelevant). Do
                            // NOT log the "TWRP version mismatch?" warning
                            // here — it would be misleading on arm64,
                            // where the skip is expected and harmless.
                        }
                        SelinuxLoadSkipPatchResult::NotFound => {
                            warning!(
                                "[KR64] PARENT: could not find selinux-load-failure jump pattern in /init (TWRP version mismatch?) — init will spin in pause() forever after selinux_android_load_policy() fails (see worklog 6-I)"
                            );
                        }
                    }
                }
                Err(e) => warning!(
                    "[KR64] PARENT: failed to read /init for selinux-load-failure patching: {} (init will spin in pause() forever — see worklog 6-I)",
                    e
                ),
            }
        }

        // ── property_contexts parser crash-NOP patch — PRAGMATIC
        // "make it not crash" patch (Task 6-M) ──
        //
        // HONEST LABEL: This is NOT a proper fix — it's a pragmatic
        // "make it not crash" patch. The proper fix would trace the
        // caller of init's property_contexts parser (the function that
        // iterates the SEPolicy context-table at 0x80ce270) and
        // initialize the uninitialized context field at offset 0x14 —
        // but that's deep in AOSP 5.1 libselinux internals
        // (DISPATCHER-FINAL-5 + DISPATCHER-FINAL-6).
        //
        // ROOT CAUSE (6-K + DISPATCHER-FINAL-3/4/5/6):
        //   After 6-J (selinux-load-skip NOP) + 6-L (#line strip), the
        //   guest progresses to iteration 338 of the property_contexts
        //   parser and SIGSEGVs at rip=0x80a0b9e:
        //     movl $0x0, 0x4(%edx)  (insn: c7 42 04 00 00 00 00)
        //   where edx=0x74616433 (garbage from `ctx->field_at_0x14`,
        //   uninitialized by the caller — NOT from file content).
        //
        // THE PATCH:
        //   Original: c7 42 04 00 00 00 00  (movl $0x0, 0x4(%edx))
        //   Patched:  90 90 90 90 90 90 90  (7 × NOP)
        //   File offset: 0x080a0b9e - 0x08048000 = 0x58b9e
        //
        // EFFECT (honest caveats):
        //   * The parser SKIPS the write to the garbage pointer and
        //     continues. The parser MAY produce wrong results for the
        //     entry being processed, but it won't crash.
        //   * The parser's incorrect internal state MAY cause a LATER
        //     crash elsewhere — this is a "make it not crash" patch,
        //     NOT a proper fix.
        //   * The ONLY definitive proof is a ui-e2e-test.yml run + VLM
        //     screenshot analysis. Do NOT claim "TWRP boots now".
        //
        // Pattern + offset verified by 6-K's disassembly + DISPATCHER-
        // FINAL-3/4/5/6. See [`patch_twrp_init_property_contexts_crash_nop`]
        // for the full root-cause analysis. The function is IDEMPOTENT
        // (skipped if already applied) and is safe to apply on every boot.
        {
            let init_path = format!("{}/init", rootfs_prefix);
            match std::fs::read(&init_path) {
                Ok(mut bytes) => {
                    match patch_twrp_init_property_contexts_crash_nop(&mut bytes) {
                        PropertyContextsCrashNopPatchResult::Applied => {
                            match std::fs::write(&init_path, &bytes) {
                                Ok(()) => info!(
                                    "[KR64] PARENT: patched /init property_contexts parser crash instruction at file offset 0x58b9e (vaddr 0x80a0b9e) — replaced `movl $0x0, 0x4(%edx)` (7 bytes: c7 42 04 00 00 00 00) with 7 × NOP (90 90 90 90 90 90 90); parser will SKIP the write to the garbage `ctx->field_at_0x14` pointer + continue (PRAGMATIC 'make it not crash' patch per 6-M + DISPATCHER-FINAL-5/6 — NOT a proper fix; may hit a later crash if the parser's incorrect state propagates)"
                                ),
                                Err(e) => warning!(
                                    "[KR64] PARENT: patched /init property_contexts crash instruction in memory but failed to write back: {} (init may SIGSEGV at rip=0x80a0b9e with si_addr=0x74616433)",
                                    e
                                ),
                            }
                        }
                        PropertyContextsCrashNopPatchResult::AlreadyApplied => {
                            info!(
                                "[KR64] PARENT: /init property_contexts parser crash instruction already NOP'd (idempotent skip) — parser will skip the write to the garbage pointer + continue (PRAGMATIC 'make it not crash' patch per 6-M)"
                            );
                        }
                        PropertyContextsCrashNopPatchResult::Skipped => {
                            // Skip reason already logged inside
                            // `patch_twrp_init_property_contexts_crash_nop`
                            // (currently fires only on aarch64, where the
                            // i386-only byte pattern is irrelevant). Do
                            // NOT log the "TWRP version mismatch?" warning
                            // here — it would be misleading on arm64,
                            // where the skip is expected and harmless.
                        }
                        PropertyContextsCrashNopPatchResult::NotFound => {
                            warning!(
                                "[KR64] PARENT: could not find property_contexts parser crash instruction at file offset 0x58b9e in /init (TWRP version mismatch?) — init may SIGSEGV at rip=0x80a0b9e with si_addr=0x74616433 (garbage `ctx->field_at_0x14` from uninitialized context — see worklog 6-M + DISPATCHER-FINAL-5/6)"
                            );
                        }
                    }
                }
                Err(e) => warning!(
                    "[KR64] PARENT: failed to read /init for property_contexts crash-nop patching: {} (init may SIGSEGV at rip=0x80a0b9e — see worklog 6-M)",
                    e
                ),
            }
        }

        // ── read_file() SIGSEGV NOP patch — PRAGMATIC symptom-mask
        // patch (Task 6-V) ──
        //
        // HONEST LABEL: This is NOT a proper fix — it's a pragmatic
        // symptom-mask patch. The proper fix belongs in the SIGSYS
        // handler's register-preservation logic.
        //
        // ROOT CAUSE (6-U DIAG KLOG + 6-V-pre disassembly):
        //   After 6-U's DIAG KLOG diagnostic landed (5fc92b7), the UI
        //   E2E run 32194676789 analysis showed iter count DROP
        //   3635→826 (PEEKDATA timing exposed a latent SIGSEGV), exit
        //   code -11 (SIGSEGV):
        //     mov %ecx,(%eax)  (insn: 89 08)
        //   where eax=0x696e692f (ASCII "/ini" — first 4 bytes of
        //   "/init.rc" rodata leaked by a SIGSYS-handler race).
        //
        // THE PATCH:
        //   Original: 89 08  (mov %ecx,(%eax))
        //   Patched:  90 90  (2 × NOP)
        //   File offset: 0x8052f65 - 0x8048000 = 0xaf65
        //
        // EFFECT (honest caveats):
        //   * read_file() SKIPS writing the read byte-count to *arg2.
        //   * The buffer is STILL null-terminated at 0x8052f5b
        //     (movb $0x0, 0x1(%edx,%ecx,1) BEFORE the crash site is
        //     NOT touched), so callers that use the buffer as a C
        //     string still work.
        //   * Only callers that explicitly depend on the ssize_t*
        //     out-param being written are affected — 13 call sites
        //     exist; none critically depend on the out-size being
        //     written (the buffer is NUL-terminated).
        //   * The ONLY definitive proof is a ui-e2e-test.yml run + VLM
        //     screenshot analysis. Do NOT claim "TWRP boots now".
        //
        // Pattern + offset verified by 6-V-pre's disassembly + 6-U's
        // DIAG KLOG. See [`patch_twrp_init_read_file_sigsegv`] for the
        // full root-cause analysis. The function is IDEMPOTENT (skipped
        // if already applied) and is safe to apply on every boot.
        {
            let init_path = format!("{}/init", rootfs_prefix);
            match std::fs::read(&init_path) {
                Ok(mut bytes) => {
                    match patch_twrp_init_read_file_sigsegv(&mut bytes) {
                        ReadFileSigsegvPatchResult::Applied => {
                            match std::fs::write(&init_path, &bytes) {
                                Ok(()) => info!(
                                    "[KR64] PARENT: patched /init read_file() at file offset 0xaf65 (vaddr 0x8052f65) — replaced `mov %ecx,(%eax)` (2 bytes: 89 08) with 2× NOP (90 90); skips the *arg2=readcount store that SIGSEGV'd when arg2 held garbage pointer 0x696e692f ('/ini' rodata leak from SIGSYS-handler race). The buffer is still null-terminated at 0x8052f5b so string-using callers work; only the explicit size out-param is dropped. (Task 6-V; pragmatic symptom-mask per disassembly — the real fix belongs in the SIGSYS handler's register-preservation logic.)"
                                ),
                                Err(e) => warning!(
                                    "[KR64] PARENT: patched /init read_file() in memory but failed to write back: {} (init may SIGSEGV at rip=0x8052f65 with si_addr=0x696e692f)",
                                    e
                                ),
                            }
                        }
                        ReadFileSigsegvPatchResult::AlreadyApplied => {
                            info!(
                                "[KR64] PARENT: /init read_file() *arg2 store already NOP'd (idempotent skip) — store skipped (PRAGMATIC symptom-mask per Task 6-V)"
                            );
                        }
                        ReadFileSigsegvPatchResult::Skipped => {
                            // Skip reason already logged inside
                            // `patch_twrp_init_read_file_sigsegv`
                            // (currently fires only on aarch64, where
                            // the i386-only byte pattern is irrelevant).
                            // Do NOT log the "TWRP version mismatch?"
                            // warning here — it would be misleading on
                            // arm64, where the skip is expected and
                            // harmless.
                        }
                        ReadFileSigsegvPatchResult::NotFound => {
                            warning!(
                                "[KR64] PARENT: could not find read_file() *arg2 store instruction at file offset 0xaf65 in /init (TWRP version mismatch?) — init may SIGSEGV at rip=0x8052f65 with si_addr=0x696e692f (garbage `*arg2` from SIGSYS-handler race — see worklog 6-U + DISPATCHER-UPDATE-12)"
                            );
                        }
                    }
                }
                Err(e) => warning!(
                    "[KR64] PARENT: failed to read /init for read_file() SIGSEGV patching: {} (init may SIGSEGV at rip=0x8052f65 — see worklog 6-U + DISPATCHER-UPDATE-12)",
                    e
                ),
            }
        }

        // Task 6-Z38: NOP the `repz cmpsb` in is_selinux_enabled() that
        // crashes with NULL pointer deref. The forked recovery service child
        // calls is_selinux_enabled() → getcon() → getcon returns SUCCESS but
        // with a NULL context pointer (the 6-Z25 attr/current fake isn't
        // working correctly for the forked child). The `repz cmpsb` at
        // vaddr 0x809d7e9 (file offset 0x557e9) compares the NULL context
        // with a string literal → SIGSEGV at si_addr=0x0.
        // PATCH: replace `f3 a6` (repz cmpsb, 2 bytes) with `90 90` (2× NOP).
        // This makes the comparison always "equal" (ZF=1 from prior test) →
        // is_selinux_enabled returns 1 (enabled). Pragmatic symptom-mask per
        // the 6-V/6-M disassembly approach.
        {
            let init_path = format!("{}/init", rootfs_prefix);
            match std::fs::read(&init_path) {
                Ok(mut bytes) => {
                    let off = 0x557e9; // file offset = vaddr 0x809d7e9 - base 0x08048000
                    if off + 2 <= bytes.len() && bytes[off] == 0xf3 && bytes[off + 1] == 0xa6 {
                        bytes[off] = 0x90; // NOP
                        bytes[off + 1] = 0x90; // NOP
                        match std::fs::write(&init_path, &bytes) {
                            Ok(()) => info!(
                                "[KR64] PARENT: patched /init is_selinux_enabled() repz cmpsb at file offset 0x557e9 (vaddr 0x809d7e9) — replaced `repz cmpsb` (2 bytes: f3 a6) with 2× NOP (90 90); prevents NULL ptr deref crash when getcon() returns NULL context in forked child (Task 6-Z38)"
                            ),
                            Err(e) => warning!(
                                "[KR64] PARENT: patched is_selinux_enabled NOP in memory but failed to write: {} (recovery child may SIGSEGV at rip=0x809d7e9)",
                                e
                            ),
                        }
                    } else if off + 2 <= bytes.len() && bytes[off] == 0x90 && bytes[off + 1] == 0x90
                    {
                        info!(
                            "[KR64] PARENT: /init is_selinux_enabled repz cmpsb already NOP'd (idempotent skip) (Task 6-Z38)"
                        );
                    } else {
                        warning!(
                            "[KR64] PARENT: could not find repz cmpsb (f3 a6) at file offset 0x557e9 in /init — found {:02x} {:02x} instead (TWRP version mismatch?) (Task 6-Z38)",
                            if off + 1 <= bytes.len() { bytes[off] } else { 0 },
                            if off + 2 <= bytes.len() { bytes[off + 1] } else { 0 },
                        );
                    }
                }
                Err(e) => warning!(
                    "[KR64] PARENT: failed to read /init for is_selinux_enabled NOP patch: {}",
                    e
                ),
            }
        }
    }

    // Task 6-Z28: REVERTED the 6-Z19 poll-loop NOP. The NOP made init skip
    // poll() entirely → init's event loop busy-spun in userspace (no
    // syscalls, no sleep, no events). init processed all actions up to the
    // recovery service start (#457), failed ("could not get context"), and
    // then spun forever waiting for events that never came (poll was NOP'd).
    // NOW: poll() is called normally. The ptrace_emu intercepts poll() at
    // the EXIT + fakes the return to 0 (timeout, no events) + sleeps 100ms
    // at the ENTRY to prevent the POLLERR busy-spin. This gives init timer
    // events to process (retry service starts, etc.) without busy-spinning.
    // {
    //     let init_path = format!("{}/init", rootfs_prefix);
    //     match std::fs::read(&init_path) {
    //         Ok(mut bytes) => {
    //             match patch_twrp_init_poll_loop_nop(&mut bytes) {
    //                 PollLoopNopPatchResult::Applied => {
    //                     match std::fs::write(&init_path, &bytes) {
    //                         Ok(()) => info!(
    //                             "[KR64] PARENT: patched /init poll-loop NOP at file offset 0xc59 (vaddr 0x8048c59) — replaced `call poll` + `test %eax,%eax` (7 bytes: e8 f2 17 02 00 85 c0) with 7× NOP (90 90 90 90 90 90 90); breaks the POLLERR busy-wait by skipping the poll() call entirely (eax keeps its prior value → jle taken → loop body continues without re-polling). (Task 6-Z19; pragmatic symptom-mask per disassembly — the real fix is making the property_service socket functional.)"
    //                         ),
    //                         Err(e) => warning!(
    //                             "[KR64] PARENT: patched /init poll-loop NOP in memory but failed to write back: {} (init may keep POLLERR-spinning at ~1000/sec)",
    //                             e
    //                         ),
    //                     }
    //                 }
    //                 PollLoopNopPatchResult::AlreadyApplied => {
    //                     info!(
    //                         "[KR64] PARENT: /init poll-loop NOP already applied (idempotent skip) — poll call already NOP'd (Task 6-Z19)"
    //                     );
    //                 }
    //                 PollLoopNopPatchResult::Skipped => {}
    //                 PollLoopNopPatchResult::NotFound => {
    //                     warning!(
    //                         "[KR64] PARENT: could not find poll-loop `call poll`+`test` at file offset 0xc59 in /init (TWRP version mismatch?) — init may keep POLLERR-spinning"
    //                     );
    //                 }
    //             }
    //         }
    //         Err(e) => warning!(
    //             "[KR64] PARENT: failed to read /init for poll-loop NOP patching: {} (init may keep POLLERR-spinning)",
    //             e
    //         ),
    //     }
    // }

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
    // 6-Z99: UNCONDITIONAL /vendor/etc/fstab.ranchu stub. The existing
    // non-TWRP branch already wrote it, yet E2E 32666825601's init logged
    // "ReadFstabFromFile(): cannot open file: '/vendor/etc/fstab.ranchu':
    // No such file or directory" — make the write unconditional so the
    // file always exists pre-fork (empty would suffice — init's "Failed
    // to fstab for first stage mount" fell through non-fatally — but the
    // long-standing minimal stub content is strictly better for vold
    // later). TWRP ignores /vendor (reads /etc/recovery.fstab); creating
    // the dir in its ramdisk is harmless.
    //
    // 6-Z101 PART D: the stub content is now COMMENT-ONLY (0 entries).
    // The previous 3 /dev/null entries were active fstab entries that
    // Android 11 init parses in BOTH stages; active entries would send
    // first-stage mount AND second-stage `mount_all` at /dev/null
    // (ENOTBLK noise, potentially boot-fatal mount_all failures). An
    // empty (comment-only) fstab parses cleanly (fs_mgr treats comment
    // lines as no-ops → 0 entries, success) → FirstStageMount finds no
    // `first_stage_mount`-flagged entries → skips early mounts entirely
    // (same non-fatal shape as the aosp16-proven ENOENT fallthrough,
    // minus the error log). No early mounts is the twoyi model: the
    // ptrace path translation provides /system + /vendor, and the self-
    // execve now goes through the staged cache copy. vold keeps working
    // per the ORIGINAL pre-6-Z99 design comment ("ship a truly empty
    // fstab — vold proceeds with an empty fstab").
    {
        let fstab_path = format!("{}/vendor/etc/fstab.ranchu", rootfs_prefix);
        let fstab_content = "# Minimal fstab for twoyi virtualization (comment-only — 0 entries; FirstStageMount skips early mounts, vold proceeds with an empty fstab)\n";
        let _ = std::fs::create_dir_all(format!("{}/vendor/etc", rootfs_prefix));
        let _ = std::fs::write(&fstab_path, fstab_content);
        if cfg.boot_recovery {
            info!("[KR64] PARENT: wrote /vendor/etc/fstab.ranchu stub (TWRP boot — informational; TWRP reads /etc/recovery.fstab)");
        } else {
            info!("[KR64] PARENT: overwrote fstab.ranchu with comment-only stub (0 entries)");
        }
    }

    // ── 6-Z192: guest property-area FORMAT PROBE ──
    //
    // The pre-creation below used to hardcode "boot_recovery ⇒ OLD
    // single-file format" — true for the angler-era TWRP inits (AOSP
    // 5.1/6.0 bionic opens /dev/__properties__ as a FILE) but WRONG for
    // newer recovery builds whose init speaks the Android 8+ SUBDIRECTORY
    // format. Evidence (run 33151412680): twrp-3.7.0_9-0-whyred's init
    // contains the strings `properties_serial`, `property_info`, and
    // "Unable to write serialized property infos" — it tries to write
    // /dev/__properties__/properties_serial, hits our pre-created FILE,
    // gets ENOTDIR, logs "Failed to initialize property area", and
    // exits 127 before any UI. The angler 3.7.0_9 init (same TWRP
    // release, older device tree) has NONE of those strings — it is a
    // pure old-format consumer.
    //
    // So the format is a PER-GUEST property, decided by probing the
    // guest's OWN init binary for the `properties_serial` literal
    // (present iff the init speaks the new format). Recovery-agnostic:
    // any recovery family, any generation — the binary is the truth.
    let guest_new_prop_format: bool =
        probe_init_new_property_format(&format!("{}/init", rootfs_prefix));
    if cfg.boot_recovery {
        info!(
            "[KR64] PARENT: guest init property-format probe: {} ({} format)",
            if guest_new_prop_format {
                "NEW (Android 8+ subdirectory)"
            } else {
                "OLD (single file)"
            },
            "boot_recovery=true — 6-Z192"
        );
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
    // TWRP BOOT (cfg.boot_recovery=true): pre-create `/dev/__properties__`
    // as a SINGLE FILE with the OLD AOSP 5.1 prop_area header (128 KB).
    // TWRP's bionic opens `/dev/__properties__` directly (NOT the Android
    // 8+ `/dev/__properties__/properties_serial` subdirectory path that the
    // else-branch below prepares). Providing the OLD-format file here makes
    // `__system_property_area_init` succeed → `__system_property_area__`
    // is non-NULL → `find_property()` reads valid memory instead of
    // NULL+0x90 → no SIGSEGV at rip=0x809255d. This is the PROPER
    // root-cause fix per 5-Z's recommendation #2 (not the revert of
    // f720934). See worklog 5-Z's disassembly for the full chain.
    //
    // ANDROID BOOT (cfg.boot_recovery=false): pre-create the directory +
    // `property_info` + `properties_serial` (NEW Android 8+ format) on host
    // + rootfs, as before.
    {
        use std::os::unix::fs::PermissionsExt;
        if cfg.boot_recovery && !guest_new_prop_format {
            // Build the OLD-format prop_area bytes (128 KB) — magic=PROP,
            // version=0xfc6ed0ab, bytes_used=0, serial=0, then zero-padded
            // data area. init's __system_property_area_init will re-mmap + memset
            // the in-memory header on top of this file (per AOSP 5.1 source),
            // but providing the correct header bytes here makes the file
            // self-describing for diagnostics and future property-injection.
            let prop_bytes = vfs::make_old_format_property_area();
            // Pre-create {rootfs}/dev/__properties__ as a regular FILE.
            let rootfs_prop_file = format!("{}/dev/__properties__", rootfs_prefix);
            // Ensure {rootfs}/dev exists (it usually does by this point).
            let _ = std::fs::create_dir_all(format!("{}/dev", rootfs_prefix));
            // Remove any pre-existing DIRECTORY at this path (from a prior
            // Android-mode run on the same rootfs) — a directory would block
            // the file write with EISDIR.
            let rootfs_prop_md = std::fs::metadata(&rootfs_prop_file);
            if matches!(rootfs_prop_md, Ok(ref md) if md.is_dir()) {
                let _ = std::fs::remove_dir_all(&rootfs_prop_file);
                info!(
                    "[KR64] PARENT: removed stale dir at {} (switching to OLD-format file)",
                    rootfs_prop_file
                );
            }
            match std::fs::write(&rootfs_prop_file, &prop_bytes) {
                Ok(_) => {
                    let _ = std::fs::set_permissions(
                        &rootfs_prop_file,
                        std::fs::Permissions::from_mode(0o666),
                    );
                    info!(
                        "[KR64] PARENT: pre-created rootfs {} (OLD-format, {} bytes, mode 0666)",
                        rootfs_prop_file,
                        prop_bytes.len()
                    );
                }
                Err(e) => {
                    error!(
                        "[KR64] PARENT: failed to pre-create rootfs {}: {}",
                        rootfs_prop_file, e
                    );
                }
            }
            // Also pre-create on the host /dev/__properties__ (defensive —
            // the SIGSYS handler may translate paths through the host's /dev
            // before falling back to rootfs). Don't clobber an existing
            // directory (host's property service may have created it on a
            // real-Android host — we don't want to break that).
            let host_prop_file = "/dev/__properties__";
            let host_md = std::fs::metadata(host_prop_file);
            let host_exists_as_file = matches!(host_md, Ok(ref md) if md.is_file());
            if !host_exists_as_file {
                // Only write if the path doesn't already exist OR exists as a
                // non-directory. If a directory exists at /dev/__properties__
                // on the host (rare — only happens on real Android hosts where
                // the system's init already created the directory), skip the
                // write to avoid breaking the host's property service.
                let host_exists_as_dir = matches!(host_md, Ok(ref md) if md.is_dir());
                if !host_exists_as_dir {
                    match std::fs::write(host_prop_file, &prop_bytes) {
                        Ok(_) => {
                            let _ = std::fs::set_permissions(
                                host_prop_file,
                                std::fs::Permissions::from_mode(0o666),
                            );
                            info!(
                            "[KR64] PARENT: pre-created host {} (OLD-format, {} bytes, mode 0666)",
                            host_prop_file,
                            prop_bytes.len()
                        );
                        }
                        Err(e) => {
                            // Likely EACCES on the host /dev in non-root mode
                            // — not fatal, the rootfs copy is what init opens
                            // after path translation. Log and continue.
                            info!(
                            "[KR64] PARENT: did not pre-create host {} ({} — non-fatal, rootfs copy is what init opens)",
                            host_prop_file, e
                        );
                        }
                    }
                } else {
                    info!(
                    "[KR64] PARENT: host {} exists as a directory (real-Android host?) — leaving it untouched",
                    host_prop_file
                );
                }
            }
        } else if cfg.boot_recovery {
            // ----- 6-Z196: NEW-format RECOVERY (Android 8+ init) -----
            // The guest init OWNS the property area: it parses property
            // contexts, serializes the trie, writes property_info itself
            // (open O_CREAT|O_TRUNC) and — critically — opens
            // properties_serial with O_CREAT|O_EXCL (bionic
            // SystemProperties::area_init). ANY pre-existing file at
            // either path breaks the boot:
            //   * a stale properties_serial → open returns EEXIST →
            //     __system_property_area_init() == -1 → init
            //     LOG(FATAL) "Failed to initialize property area" →
            //     exit 127 (run 33157498271: the parent pre-created a
            //     0-byte properties_serial; property_info was written
            //     fine — 8508 bytes — proving the guest got that far).
            //   * a stale single FILE /dev/__properties__ (an old-format
            //     run, or the probe's OLD default) → every child open
            //     fails ENOTDIR → same FATAL.
            // So: give init the same clean slate a fresh tmpfs /dev has
            // on real hardware — remove stale artifacts, create ONLY
            // the (empty) directory, pre-create NOTHING.
            let rootfs_prop_dir = format!("{}/dev/__properties__", rootfs_prefix);
            // (1) stale single FILE from an old-format boot → remove.
            let rootfs_prop_md = std::fs::metadata(&rootfs_prop_dir);
            if matches!(rootfs_prop_md, Ok(ref md) if md.is_file()) {
                match std::fs::remove_file(&rootfs_prop_dir) {
                    Ok(()) => info!(
                        "[KR64] PARENT: removed stale OLD-format file {} (new-format recovery boot — 6-Z196)",
                        rootfs_prop_dir
                    ),
                    Err(e) => error!(
                        "[KR64] PARENT: failed to remove stale file {}: {}",
                        rootfs_prop_dir, e
                    ),
                }
            }
            // (2) stale pre-created / prior-run property files → remove.
            for fname in ["property_info", "properties_serial"] {
                let path = format!("{}/{}", rootfs_prop_dir, fname);
                if Path::new(&path).exists() {
                    match std::fs::remove_file(&path) {
                        Ok(()) => info!(
                            "[KR64] PARENT: removed stale {} (guest init re-creates it with O_EXCL semantics — 6-Z196)",
                            path
                        ),
                        Err(e) => error!(
                            "[KR64] PARENT: failed to remove stale {}: {}",
                            path, e
                        ),
                    }
                }
            }
            // (3) the clean directory itself.
            let _ = std::fs::create_dir_all(&rootfs_prop_dir);
            let _ =
                std::fs::set_permissions(&rootfs_prop_dir, std::fs::Permissions::from_mode(0o777));
            info!(
                "[KR64] PARENT: new-format recovery property area: clean dir at {}, NO files pre-created (guest owns them — 6-Z196)"
            , rootfs_prop_dir);
        } else {
            // ----- Android-guest (NEW Android 8+ subdirectory format) -----
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
            let _ =
                std::fs::set_permissions(&rootfs_prop_dir, std::fs::Permissions::from_mode(0o777));
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
                            let _ = std::fs::set_permissions(
                                &path,
                                std::fs::Permissions::from_mode(0o666),
                            );
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
    } // end of { use PermissionsExt; if cfg.boot_recovery { ... } else { ... } }

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
    // NOTE (Task 6-Z88): the CString is built unconditionally because the
    // child branch below references it inside its own `cfg.boot_recovery`
    // runtime check (construction is side-effect-free — just format! +
    // CString::new). Only the FILE pre-create + its logging is gated:
    // in normal (AOSP) mode there is no TWRP init to redirect, so the
    // pre-create + info!/warning! chatter was pure noise (run 32632668179).
    let twrp_log_path_cstr: CString =
        CString::new(twrp_log_path_str.as_str()).unwrap_or_else(|_| {
            // Path contained an interior NUL — extremely unlikely for
            // an app-private data dir, but fall back to the literal
            // so we don't panic in the parent. The child's open() will
            // then fail and the existing WARN branch will fire.
            CString::new("/twrp-init.log").unwrap()
        });
    if cfg.boot_recovery {
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
    }

    info!("[KR64] forking guest process");

    // Task 6-Z31 diag: verify /sbin/recovery + /sbin/linker exist + are
    // executable before forking. The recovery service child exits 127
    // (execve failure) — this diagnostic will reveal whether the files
    // are missing, not executable, or the interpreter is wrong.
    {
        let sbin_recovery = format!("{}/sbin/recovery", rootfs_prefix);
        let sbin_linker = format!("{}/sbin/linker", rootfs_prefix);
        let sbin_hook = format!("{}/sbin/libtwrp_fb_hook.so", rootfs_prefix);
        for (name, path) in [
            ("recovery", &sbin_recovery),
            ("linker", &sbin_linker),
            ("libtwrp_fb_hook.so", &sbin_hook),
        ] {
            match std::fs::metadata(&path) {
                Ok(meta) => {
                    let perms = meta.permissions();
                    let mode = perms.mode();
                    let is_exec = (mode & 0o100) != 0;
                    info!(
                        "[KR64] PRE-FORK DIAG: {} at {} — exists, size={}, mode=0{:o}, exec={}",
                        name,
                        path,
                        meta.len(),
                        mode,
                        is_exec
                    );
                    if !is_exec {
                        warning!(
                            "[KR64] PRE-FORK DIAG: {} at {} is NOT executable (mode=0{:o}) — execve will EACCES → exit 127 (Task 6-Z31: RamdiskImporter should have set exec)",
                            name, path, mode
                        );
                    }
                }
                Err(e) => {
                    warning!(
                        "[KR64] PRE-FORK DIAG: {} at {} MISSING: {} — execve will ENOENT → exit 127",
                        name, path, e
                    );
                }
            }
        }
    }

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
            // 6-Z159: urandom/random moved OUT of this table — see the
            // adaptive block below (symlink ELOOP on the arm64 redroid
            // runner, run 32976478078 kmsg: 'Failed to open /dev/urandom:
            // Too many symbolic links encountered').
            ("dev/console", "/dev/console"),
            ("dev/ptmx", "/dev/ptmx"),
            ("dev/tty", "/dev/tty"),
            ("dev/kmsg", "/dev/kmsg"),
            // Task 6-Z10: /dev/hw_random — init opens this for hardware
            // RNG entropy. Missing → ENOENT → SIGSEGV at rip=0x6f722f69
            // (verified on ceec1f2 UI E2E run 32231410279). Symlinks to
            // /dev/urandom or /dev/null cause ELOOP (errno 40) on Android
            // (verified on d2963a8 + 0e19c57 UI E2E). Fix: pre-create as
            // a regular empty file (like /dev/.booting, /dev/__null__).
            // init reads 0 bytes → no crash. The hw_random read returns
            // EOF → init treats it as "no hw RNG available" → continues.
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

        // ── 6-Z161: /dev/urandom + /dev/random — REGULAR FILES with real
        // entropy, ALWAYS ─────────────────────────────────────────────
        //
        // 6-Z159's "adaptive symlink" approach FAILED on the arm64
        // redroid runner (run 32983937665, SHA 3481022 — which INCLUDES
        // the 6-Z159 fix): init still logged 'Failed to open
        // /dev/urandom: Too many symbolic links encountered' TWICE.
        // The parent's post-symlink test-open ran in the PARENT's
        // context (no chroot/mount-ns) and succeeded, so the symlink
        // was kept — but the CHILD resolves the absolute symlink target
        // in a DIFFERENT context (the guest's jailed root), where
        // /dev/urandom points back into {rootfs}/dev/urandom → ELOOP.
        // A parent-side test-open can never prove what the CHILD will
        // see; stop trying. Regular files are context-proof: no
        // symlink traversal at all.
        //
        // Entropy: the parent reads 4096 bytes from the HOST's real
        // /dev/urandom (world-readable — works in both the KVM root
        // context and the redroid untrusted_app context) and writes
        // them into the file. TWRP init reads /dev/urandom ONCE to
        // seed its RAND (the kmsg shows it opens + reads + continues
        // after the current failure); 4 KiB of genuine entropy is
        // more than enough for that. If the parent's own urandom read
        // fails (paranoia), fall back to an empty file — EOF, the
        // proven hw_random precedent (init logs + continues).
        for rel in ["dev/urandom", "dev/random"] {
            let file_path = format!("{}/{}", rootfs_prefix, rel);
            let _ = std::fs::remove_file(&file_path);
            let entropy: Vec<u8> = std::fs::File::open("/dev/urandom")
                .and_then(|mut f| {
                    use std::io::Read;
                    let mut buf = vec![0u8; 4096];
                    f.read_exact(&mut buf).map(|_| buf)
                })
                .unwrap_or_default();
            let wrote = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .mode(0o666)
                .open(&file_path)
                .and_then(|mut f| std::io::Write::write_all(&mut f, &entropy));
            match wrote {
                Ok(_) => {
                    let _ = std::fs::set_permissions(
                        &file_path,
                        std::fs::Permissions::from_mode(0o666),
                    );
                    info!(
                        "[KR64] PARENT: 6-Z161: {} pre-created as a REGULAR FILE with {} bytes of real host entropy (no symlink — ELOOP-proof in every child context)",
                        file_path,
                        entropy.len()
                    );
                }
                Err(e) => warning!(
                    "[KR64] PARENT: 6-Z161: could not create {} : {}",
                    file_path,
                    e
                ),
            }
        }

        // Regular files init expects to create/open.
        // Use OpenOptions (same pattern as /dev/__kmsg__ creation above)
        // and log success/failure for each file.
        //
        // 6-Z242: /dev/hw_random is DELIBERATELY NOT in this list — it is
        // staged by devices::create_twrp_misc_devs with 512 bytes of real
        // entropy, and THIS loop used to run afterwards and truncate it to
        // zero ("pre-created ... size=0" in the logs), re-creating the
        // exact EOF failure 6-Z242 fixed: pi/older init generations HARD
        // FAIL their mix-hwrng-into-linux-rng action on EOF
        // ("Security failure; rebooting into recovery mode", capricorn
        // class). The old 6-Z10 "init reads 0 bytes and continues"
        // assumption only held for the angler-era x86 init.
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
        // CRITICAL (format, 6-Z223): TWO consumer families parse this
        // file and they split DIFFERENTLY — see join_hybrid_cmdline()
        // for the full dual-parser rationale:
        //   * OLD init (TWRP 2.8, Task 6-Y): NUL / C-string iteration —
        //     a pure-space file made item 1's VALUE swallow the rest
        //     (ro.hardware garbage → boot sequence failure).
        //   * MODERN libfstab (Android 12+, 6-Z223): SPACE splitting —
        //     a pure-NUL file made every key after the first invisible
        //     ("Error updating for slotselect", run 33275526098, even
        //     with 6-Z219's slot_suffix written).
        // The HYBRID "item\0 item\0 … item" layout satisfies both.
        // 6-Z159: derive androidboot.hardware from the ACTUAL recovery
        // image instead of hardcoding ranchu. The ranchu value matched
        // the x86 emulator rootfs but broke every real-device image:
        // TWRP angler (hardware=angler) looked for /fstab.ranchu +
        // /init.recovery.ranchu.rc (both absent — the ramdisk ships
        // init.recovery.angler.rc) and recovery exited 1 (run
        // 32976478078 kmsg stub: 'fs_mgr: Cannot open file
        // /fstab.ranchu' + "could not import file
        // '/init.recovery.ranchu.rc' from '/init.rc'").
        //
        // Detection: scan the rootfs top level for a
        // hardware-suffixed file whose suffix is NOT one of TWRP's
        // generic per-mode rc names (service/usb/nano/hlthchrg/logd/
        // ldconfig/mksh/vold_decrypt). fstab.<hw> / ueventd.<hw>.rc /
        // init.<hw>.rc / init.recovery.<hw>.rc all vote; a single
        // distinct candidate wins. No candidate (e.g. the x86
        // emulator rootfs, whose fstab.ranchu lives under /vendor/etc)
        // falls back to ranchu — preserving x86 behaviour exactly.
        let generic_rc_names: &[&str] = &[
            "service",
            "usb",
            "nano",
            "hlthchrg",
            "logd",
            "ldconfig",
            "mksh",
            "vold_decrypt",
            "crypto",
            "quiet",
            "verifier",
        ];
        // 6-Z159b: pinned by the 6-Z159a listing (run 32980068049): the
        // angler rootfs contains BOTH init.recovery.angler.rc AND
        // init.partlink.rc — the plain `init.<hw>.rc` pattern made
        // partlink a second candidate → len != 1 → None. But init.rc
        // PROVES the split: it literally imports the generic rc files
        // (logd/ldconfig/mksh/nano/usb/service/vold_decrypt) and imports
        // the hardware one via `import /init.recovery.${ro.hardware}.rc`,
        // while init.partlink.rc is not imported by init.rc AT ALL (a
        // module rc pulled in by other rc files). So: candidate patterns
        // are ONLY fstab.<hw> / ueventd.<hw>.rc / init.recovery.<hw>.rc
        // (the ${ro.hardware} convention — plain init.<x>.rc DROPPED),
        // any candidate whose file init.rc imports LITERALLY is excluded,
        // and exactly one remaining candidate wins (else ranchu fallback,
        // x86 emulator rootfs unchanged).
        let init_rc_imports: Vec<String> =
            std::fs::read_to_string(format!("{}/init.rc", rootfs_prefix))
                .unwrap_or_default()
                .lines()
                .filter_map(|l| l.trim().strip_prefix("import "))
                .map(|s| s.trim().rsplit('/').next().unwrap_or("").to_string())
                .collect();
        let detected_hw: Option<String> = std::fs::read_dir(&rootfs_prefix).ok().and_then(|rd| {
            let mut cands: Vec<String> = Vec::new();
            for ent in rd.flatten() {
                let name = ent.file_name().to_string_lossy().to_string();
                let hw: Option<&str> = if let Some(rest) = name.strip_prefix("fstab.") {
                    Some(rest)
                } else if let Some(rest) = name.strip_suffix(".rc") {
                    if let Some(mid) = rest.strip_prefix("ueventd.") {
                        Some(mid)
                    } else if let Some(mid) = rest.strip_prefix("init.recovery.") {
                        Some(mid)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(hw) = hw {
                    let generic = hw.is_empty()
                        || hw.contains('.')
                        || generic_rc_names.contains(&hw)
                        || init_rc_imports.iter().any(|imp| imp == &name);
                    if !generic {
                        cands.push(hw.to_string());
                    }
                }
            }
            cands.sort();
            cands.dedup();
            if cands.len() == 1 {
                Some(cands.remove(0))
            } else {
                None
            }
        });
        let hw = detected_hw.clone().unwrap_or_else(|| "ranchu".to_string());
        if detected_hw.is_some() {
            info!(
                "[KR64] PARENT: 6-Z159: androidboot.hardware detected from recovery image = {:?} (was hardcoded ranchu)",
                hw
            );
        } else {
            // 6-Z159a: the standalone logic provably finds 'angler' on the
            // extracted angler ramdisk, yet run 32978121405 logged the None
            // branch — dump the directory the detector actually saw + every
            // candidate it considered so ONE run pins the divergence.
            let listing: Vec<String> = std::fs::read_dir(&rootfs_prefix)
                .map(|rd| {
                    rd.flatten()
                        .take(60)
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect()
                })
                .unwrap_or_default();
            info!(
                "[KR64] PARENT: 6-Z159: no unique hardware suffix found in rootfs {:?} — keeping androidboot.hardware=ranchu. Dir listing (first 60): {:?}",
                rootfs_prefix, listing
            );
        }
        // 6-Z219: derive androidboot.slot_suffix from the guest's OWN
        // fstab content instead of hard-coding an empty value. A/B
        // recovery images (e.g. lineage-22.2-sailfish) carry `slotselect`
        // flags; Android 12+ libfstab aborts the fstab parse (and the
        // recovery binary then CHECK-aborts on the empty fstab) unless a
        // non-empty slot suffix is discoverable. See the long comment on
        // detect_guest_slot_suffix() for the full evidence chain.
        let slot_suffix = detect_guest_slot_suffix(&rootfs_prefix);
        if !slot_suffix.is_empty() {
            info!(
                "[KR64] PARENT: 6-Z219: guest fstab uses slotselect → androidboot.slot_suffix={:?} (A/B recovery image)",
                slot_suffix
            );
        }
        // 6-Z270: strip FBE/FDE fs_mgr flags from /data entries so the
        // guest recovery skips its decryption probe entirely (the probe
        // forks keystore_cli_v2 which burns a ~20 s binder service-wait
        // budget per boot in the container — see the function doc).
        // Runs AFTER 6-Z219's slotselect scan (slotselect lives on
        // system/vendor lines we never touch; ordering is belt-and-braces).
        sanitize_fstab_encryption_flags(&rootfs_prefix);
        // 6-Z271j: declare the in-proxy VIRTUAL AIDL HALs in the guest's
        // VINTF device manifest so keystore2's manifest-driven
        // enumerators (libvintf getAidlInstances — NOT the servicemanager)
        // can discover them. Without this, keystore2's connect_keymint
        // finds no AIDL keymint instance, falls back to the HIDL-compat
        // chain, and panics ("Failed to create service … IKeystoreSecurity")
        // because the HIDL keymaster HAL genuinely does not exist.
        augment_vintf_manifest_for_virtual_hals(&rootfs_prefix);
        // 6-Z219: guest-derived slot suffix ("" = A-only → key omitted —
        // an empty `androidboot.slot_suffix=` would poison
        // fs_mgr_get_boot_config: "found but empty" short-circuits the
        // DT/bootconfig fallbacks).
        // 6-Z223: build + join the cmdline in the HYBRID format — see
        // join_hybrid_cmdline() for the full dual-parser rationale (old
        // init splits the buffer on NUL, Android 12+ libfstab splits on
        // SPACE; the legacy pure-NUL file made slot_suffix invisible to
        // libfstab → "Error updating for slotselect", run 33275526098,
        // even with 6-Z219's value present).
        let cmdline_items = build_cmdline_items(&hw, &slot_suffix);
        let cmdline_content = join_hybrid_cmdline(&cmdline_items);
        match std::fs::write(&cmdline_path, &cmdline_content) {
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

    // ── Pre-create TWRP's expected mount-point dirs (bootfix FIX 2) ────
    //
    // TWRP's partition manager stats/opens these paths during
    // "Updating partition details...": `/cache/.` (statfs64 — the fatal
    // "E:Unable to statfs '/cache/.`" of run 32612016071), `/cache/recovery`
    // (its log/settings folder — "I:Recreating /cache/recovery folder" →
    // "E:Could not create /cache/recovery"), `/sdcard` (data/media
    // emulated storage — "I:Can not create '/sdcard' folder."),
    // `/external_sd` + `/usb-otg` ("Can not create '/external_sd' folder
    // (Read-only file system)." — the mkdir resolved against the HOST /,
    // not the rootfs). NONE of them existed in the rootfs that run
    // (DEATH_CHAIN §9), so every layer failed. Combined with the statfs
    // path translation (ptrace_emu.rs bootfix FIX 1), statfs64("/cache/.")
    // now resolves to {rootfs}/cache/. — a real dir on the app's own
    // ext4, giving sane f_type/f_bfree/f_blocks instead of host-EACCES.
    // All are 0755 dirs; already-exists is NOT an error (idempotent —
    // matches the precreate_sysfs_stubs pattern below).
    {
        let twrp_dirs: &[&str] = &[
            "cache",
            "cache/recovery",
            "sdcard",
            "external_sd",
            "usb-otg",
        ];
        for rel in twrp_dirs {
            let dir_path = format!("{}/{}", rootfs_prefix, rel);
            match std::fs::create_dir(&dir_path) {
                Ok(()) => {
                    let _ =
                        std::fs::set_permissions(&dir_path, std::fs::Permissions::from_mode(0o755));
                    info!(
                        "[KR64] PARENT: pre-created TWRP dir {} (mode 0755)",
                        dir_path
                    );
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let _ =
                        std::fs::set_permissions(&dir_path, std::fs::Permissions::from_mode(0o755));
                    info!(
                        "[KR64] PARENT: TWRP dir {} already exists — re-asserting mode 0755",
                        dir_path
                    );
                }
                Err(e) => {
                    warning!(
                        "[KR64] PARENT: FAILED to pre-create TWRP dir {}: {} (errno={}) — TWRP's statfs/mkdir on /{} may fail",
                        dir_path,
                        e,
                        e.raw_os_error().unwrap_or(0),
                        rel
                    );
                }
            }
        }
    }

    // ── TWRP block-device nodes — 6-Z87 FIX 1: mmcblk stubs REVERTED ──
    //
    // 6-Z86 pre-created {rootfs}/dev/block/{mmcblk0,mmcblk1,mmcblk1p1}
    // as EMPTY regular files, betting on "open ok → pread EOF →
    // 'no media' → TWRP skips the device fast". E2E run 32631901109
    // (z86 analysis) disproved that bet: with the stubs, open()
    // SUCCEEDS, so TWRP proceeds to ioctl(BLKGETSIZE64) on the "block
    // device" — a regular file answers that ioctl with ENOTTY — and
    // TWRP treats a failed size probe as RETRYABLE: "Can't probe
    // device /dev/block/mmcblk1p1" looped 202+ times per boot (~15×/
    // cycle) and TWRP never finished "Updating partition details..."
    // (stuck mid-partition-details for the whole 600s window).
    //
    // WITHOUT the stubs (the z83 behaviour), the open ENOENTs, TWRP
    // logs "Can't probe" ~5 times and MOVES ON — z83 actually reached
    // the backup-folder/settings pages past that point. ENOENT is the
    // correct, fast answer for absent removable media; a stub that
    // opens-but-can't-answer-ioctls is strictly worse.
    //
    // 6-Z87 therefore deletes the three mmcblk FILES. The /dev/block
    // DIRECTORY below stays (harmless, idempotent, and other code may
    // legitimately expect the directory to exist).
    {
        // {rootfs}/dev/block — create_dir_all-style: EEXIST is fine.
        let block_dir = format!("{}/dev/block", rootfs_prefix);
        match std::fs::create_dir(&block_dir) {
            Ok(()) => {
                let _ =
                    std::fs::set_permissions(&block_dir, std::fs::Permissions::from_mode(0o755));
                info!(
                    "[KR64] PARENT: pre-created TWRP block dir {} (mode 0755; 6-Z87: no mmcblk node stubs — ENOENT lets TWRP skip absent media fast, a stubbed node ENOTTYs BLKGETSIZE64 and retry-loops)",
                    block_dir
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => {
                warning!(
                    "[KR64] PARENT: FAILED to pre-create TWRP block dir {}: {} (errno={}) — TWRP's /dev/block probes will ENOENT",
                    block_dir,
                    e,
                    e.raw_os_error().unwrap_or(0)
                );
            }
        }
    }

    // ── Pre-create fake sysfs (/sys/class + /sys/fs/selinux/{enforce,load}) ──
    //
    // ROOT CAUSE (Task 6-P, dispatcher's analysis of 56a5bd3 UI E2E):
    //   After 6-O's property_contexts deletion (56a5bd3), the guest now
    //   progresses to iteration 3059 (was 342 — ~9× improvement) but
    //   exits(1) right after `open("/sys/class") -> -13 (-EACCES)`. init
    //   tries to enumerate sysfs devices + read SELinux sysfs files
    //   (/sys/fs/selinux/{enforce,load}), but /sys is the host's REAL
    //   kernel sysfs which untrusted_app can't read.
    //
    // FIX: pre-create these as empty dirs/files in the rootfs + redirect
    //   /sys/* opens to {rootfs}/sys/* in ptrace_emu's translate_path
    //   (companion change in ptrace_emu.rs — without it, the pre-creation
    //   is useless because the guest's open() still hits the host /sys).
    //   This gives init a "fake sysfs" with no devices — init reads it,
    //   sees nothing, + proceeds. SELinux enforcement is effectively
    //   disabled (non-fatal for TWRP boot in the sandbox).
    //
    // See `precreate_sysfs_stubs` for the full root-cause analysis +
    // the per-file rationale (enforce seeded with "0" for permissive;
    // load left empty so init's policy-blob write succeeds silently).
    // Idempotent + non-fatal — runs BEFORE fork() so the child sees
    // the pre-created tree immediately.
    precreate_sysfs_stubs(&rootfs_prefix);

    // Task 6-Z4: close stale inherited fds 13..1024 in the PARENT before
    // fork. The twoyi app (grandparent) may have inherited a stale
    // /dev/socket/property_service socket fd from a PREVIOUS kr64
    // invocation's init (which bound it, then exited — the fd leaked into
    // the app via fork). kr64 inherits it from the app. The child's close
    // loop (fds 3..1024) closes the CHILD's copy, but the PARENT's copy
    // keeps the socket bound → the new init's bind returns EADDRINUSE
    // (-98). The socketcall fake-success (6-Z3) masks the EADDRINUSE to
    // 0, but the socket ISN'T actually bound → the recovery polls on it
    // → POLLERR → tight spin loop (poll returns 1 every time, busy-wait).
    //
    // fds 3..12 are kr64's daemon sockets (qemu_pipe, touch, key, event,
    // gb, gb2, dm-user, binder, audio, sensors) — must NOT be closed (the
    // daemon threads need them). So we can't blindly close 3..1024.
    //
    // Approach: iterate fds 3..1024. For each, call getsockname() to check
    // if it's a Unix socket bound to /dev/socket/property_service. If it
    // matches, close ONLY that fd. This preserves the daemon sockets +
    // frees the stale property_service socket.
    let target_path = b"/dev/socket/property_service\0";
    let mut closed_count = 0u32;
    for fd in 3..1024i32 {
        // getsockname on a non-socket fd returns ENOTSOCK (harmless).
        // On a Unix socket, it fills the sockaddr_un with the bound path.
        let mut storage: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        let mut len = std::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t;
        let ret = unsafe {
            libc::getsockname(fd, &mut storage as *mut _ as *mut libc::sockaddr, &mut len)
        };
        if ret == 0 && storage.sun_family == libc::AF_UNIX as u16 {
            // Compare the sun_path with the target path.
            // sun_path is [c_char; 108] = [i8; 108] in libc. Cast to u8
            // for comparison with the target_path byte slice.
            let sun_path = &storage.sun_path;
            let sun_path_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(sun_path.as_ptr() as *const u8, sun_path.len())
            };
            let path_len = target_path.len(); // includes NUL
            if sun_path_bytes.len() >= path_len
                && &sun_path_bytes[..path_len] == target_path.as_slice()
            {
                unsafe {
                    libc::close(fd);
                }
                closed_count += 1;
                info!(
                    "[KR64] PARENT: closed stale property_service socket at fd={} (Task 6-Z4: getsockname matched /dev/socket/property_service — freed stale binding → new init's bind will succeed)",
                    fd
                );
            }
        }
    }
    if closed_count == 0 {
        info!(
            "[KR64] PARENT: no stale property_service socket found in fds 3..1024 (Task 6-Z4: getsockname scan — the EADDRINUSE may be from a kernel-level stale binding or a different source)"
        );
    }

    // 6-Z268: drain the buffered trace sink so no staged lines straddle
    // the fork (the child branch only uses async-signal-safe writes; the
    // parent's buffer keeps its own ordering clean).
    trace_log_flush();

    // 6-Z268: enumerate the parent's open fds HERE (allocation is safe
    // outside the fork window; the child inherits the list via COW).
    // The child previously burned one close() syscall per fd in
    // 3..RLIMIT_NOFILE — the Android soft limit is 32768, so ~32k
    // mostly-EBADF syscalls ran inside the measured pre-execve window
    // on every boot. The snapshot is exact: every fd open at fork time
    // is in the list (only the readdir handle itself comes and goes,
    // and closing it again in the child is harmless).
    let child_close_fds: Vec<i32> = std::fs::read_dir("/proc/self/fd")
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().to_str().and_then(|s| s.parse::<i32>().ok()))
                .filter(|&fd| fd >= 3)
                .collect()
        })
        .unwrap_or_default();

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
        // 6-Z184 AUDIT FIX (agent 7): close up to the REAL descriptor
        // limit — fds >= 1024 survived execve into the guest (the
        // comment says "everything >= 3"; now it means it too).
        let fd_limit = unsafe {
            let mut rl: libc::rlimit = std::mem::zeroed();
            if libc::getrlimit(libc::RLIMIT_NOFILE, &mut rl) == 0 {
                rl.rlim_cur
            } else {
                1024
            }
        };
        if child_close_fds.is_empty() {
            // Fallback (procfs unavailable): the old full-range loop.
            for fd in 3..(fd_limit as i32) {
                unsafe {
                    libc::close(fd);
                }
            }
        } else {
            // 6-Z268: close exactly the fds the parent had open at fork
            // time (snapshot taken pre-fork, see child_close_fds).
            for &fd in &child_close_fds {
                unsafe {
                    libc::close(fd);
                }
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
            if cfg.use_namespaces {
                // Root mode (pivot_root): child stays x86_64, seccomp
                // arch check matches. Install the filter for hardening.
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
            } else {
                // Non-root ptrace-emulation mode: seccomp is intentionally
                // SKIPPED. The BPF filter checks arch == AUDIT_ARCH_X86_64,
                // but the guest init is an i386 binary (uses int $0x80 with
                // AUDIT_ARCH_I386). The arch mismatch causes
                // SECCOMP_RET_KILL_PROCESS for every i386 syscall, making
                // mmap2 (nr=192) return ENOSYS → __system_property_area_init
                // fails → ALL property_set calls fail → init exits(1) in a
                // boot loop (7 iterations observed in 6-W UI E2E). The
                // ptrace emulator handles all syscall interception (mount,
                // chmod, mknod, path translation, etc.) — seccomp is redundant
                // and harmful here. (Task 6-X)
                unsafe {
                    safe_write_err(
                        b"[KR64 CHILD] skipping seccomp install in ptrace-emulation mode (i386 guest AUDIT_ARCH mismatch: mmap2 ENOSYS, property system failure)\n",
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
                            // 6-Z196: run the init under the GUEST'S OWN
                            // dynamic linker. The normal path re-stages
                            // this copy via 6-Z102 (which patches PT_INTERP
                            // itself), but the PEEK-blind raw-exec fallback
                            // exec's THIS file directly — its bare guest
                            // interp ("/system/bin/linker64") would load
                            // the HOST's linker on Android hosts (API-level
                            // mismatch → CANNOT LINK) or ENOENT elsewhere.
                            if let Some(new_interp) =
                                crate::symlinks::ensure_guest_interp(&cfg.rootfs, &cache_init)
                            {
                                unsafe {
                                    safe_write_err(b"[KR64 CHILD] 6-Z196: init PT_INTERP -> ");
                                    safe_write_err(new_interp.as_bytes());
                                    safe_write_err(b" (guest's own linker)\n");
                                }
                            }
                            // 6-Z101 PART B: read-modify-write the
                            // .twoyi-staged marker keyed by cfg.init_path
                            // (re-run replaces its line, preserves future
                            // entries; currently the one
                            // `/system/bin/init<TAB>{data_dir}/cache/twoyi_init`
                            // pair; TWRP boots record `/init`). The ptrace
                            // emulator reads this marker lazily at the
                            // first execve and consults it for every
                            // subsequent execve ENTRY (the staged-exe map
                            // — see ptrace_emu::load_staged_exes_map +
                            // staged_exe_for). Mirrors the in-loop writer
                            // (ptrace_emu::append_staged_marker) exactly so
                            // the two writers stay interchangeable.
                            {
                                // 6-Z187: the marker lives OUTSIDE the guest
                                // rootfs (in {data_dir}/cache) so TWRP's File
                                // Manager never shows it.
                                let marker_path =
                                    crate::ptrace_emu::staged_exes_marker_path(&cfg.data_dir);
                                let guest_key = cfg.init_path.clone();
                                let cache_path_for_marker = cache_init.clone();
                                let existing =
                                    std::fs::read_to_string(&marker_path).unwrap_or_default();
                                let new_line = format!("{}\t{}", guest_key, cache_path_for_marker);
                                let mut out: Vec<String> = Vec::new();
                                let mut replaced = false;
                                for line in existing.lines() {
                                    let trimmed = line.trim();
                                    if trimmed.is_empty() || trimmed.starts_with('#') {
                                        out.push(line.to_string());
                                        continue;
                                    }
                                    let key = trimmed.split('\t').next().unwrap_or("");
                                    if key == guest_key {
                                        out.push(new_line.clone());
                                        replaced = true;
                                    } else {
                                        out.push(line.to_string());
                                    }
                                }
                                if !replaced {
                                    out.push(new_line);
                                }
                                if let Err(e) = std::fs::write(&marker_path, out.join("\n") + "\n")
                                {
                                    unsafe {
                                        safe_write_err(b"[KR64 CHILD] 6-Z101: staged-exe marker write FAILED for ");
                                        safe_write_err(marker_path.as_bytes());
                                        safe_write_err(b": ");
                                        safe_write_err(e.to_string().as_bytes());
                                        safe_write_err(b"\n");
                                    }
                                } else {
                                    unsafe {
                                        safe_write_err(b"[KR64 CHILD] 6-Z101: staged-exe marker ");
                                        safe_write_err(marker_path.as_bytes());
                                        safe_write_err(b" <- ");
                                        safe_write_err(guest_key.as_bytes());
                                        safe_write_err(b" -> ");
                                        safe_write_err(cache_path_for_marker.as_bytes());
                                        safe_write_err(b"\n");
                                    }
                                }
                            }
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

        // Debug: check that libgetpid_hook.so exists where the PARENT
        // staged it (see the hook-staging block in run()):
        //   * use_namespaces=true (post-pivot_root): /dev/libgetpid_hook.so
        //     (the tmpfs mounted by setup_mounts)
        //   * use_namespaces=false (no pivot_root): {cfg.rootfs}/dev/
        //     libgetpid_hook.so — the bare /dev/libgetpid_hook.so is NOT
        //     on the host filesystem in this mode; the GUEST-relative
        //     LD_PRELOAD path only resolves because the ptrace
        //     interceptor maps guest /dev/* onto {rootfs}/dev/*. So we
        //     check the host-side staged path, not the guest path.
        //
        // format! + CString::new are safe here: single-threaded
        // post-fork code running BEFORE execve (the same pattern is
        // already used for TWOYI_ROOTFS below).
        let hook_staged_path = if cfg.use_namespaces {
            "/dev/libgetpid_hook.so".to_string()
        } else {
            format!("{}/dev/libgetpid_hook.so", cfg.rootfs)
        };
        let hook_exists = match CString::new(hook_staged_path.as_str()) {
            Ok(p) => unsafe { libc::access(p.as_ptr(), libc::F_OK) == 0 },
            Err(_) => false, // staged path contains NUL -- treat as missing
        };
        if hook_exists {
            unsafe {
                safe_write_err(b"[KR64 CHILD] libgetpid_hook.so found at staged /dev path\n");
            }
        } else {
            unsafe {
                safe_write_err(b"[KR64 CHILD] libgetpid_hook.so NOT found at staged /dev path -- LD_PRELOAD=/dev/libgetpid_hook.so will fail to link\n");
            }
        }

        // 5-L diagnostic: also check /dev/libdl.so (the REAL libdl.so we
        // extracted from the APEX ext4 image in Step 3.7). If present, the
        // linker will find it FIRST (because LD_LIBRARY_PATH prepends /dev/).
        // If absent, the linker falls through to /apex/.../bionic/libdl.so
        // (the 5848-byte stub) and likely crashes at offset 0xaf174.
        // Same host-side staging-path logic as the libgetpid_hook.so check
        // above (in non-root mode the file lives at {cfg.rootfs}/dev/).
        let libdl_staged_path = if cfg.use_namespaces {
            "/dev/libdl.so".to_string()
        } else {
            format!("{}/dev/libdl.so", cfg.rootfs)
        };
        let libdl_exists = match CString::new(libdl_staged_path.as_str()) {
            Ok(p) => unsafe { libc::access(p.as_ptr(), libc::F_OK) == 0 },
            Err(_) => false,
        };
        if libdl_exists {
            unsafe {
                safe_write_err(b"[KR64 CHILD] libdl.so (REAL, from APEX) found at /dev/libdl.so -- linker should resolve DT_NEEDED:libdl.so via /dev/ FIRST\n");
            }
        } else {
            unsafe {
                safe_write_err(b"[KR64 CHILD] libdl.so NOT found at /dev/libdl.so -- linker will fall through to /apex/.../bionic/libdl.so (the 5848-byte stub). EXPECT linker64 segfault at 0xaf174 (5-K's diagnosis).\n");
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
        // LD_PRELOAD path: ALWAYS use the GUEST-RELATIVE
        // /dev/libgetpid_hook.so and /dev/libtwoyi_loader_shlib.so
        // (never an absolute HOST path like {cfg.rootfs}/dev/...).
        //
        // WHY guest-relative works in BOTH modes: every syscall of the
        // traced child goes through the ptrace loop, and
        // ptrace_emu::translate_path maps guest /dev/* onto
        // {rootfs}/dev/* — including the linker's openat()s during
        // LD_PRELOAD resolution (E2E run 32635971098 syscall #152:
        // openat "/dev/libgetpid_hook.so" → intercepted →
        // {rootfs}/dev/libgetpid_hook.so). The parent now stages the
        // hooks at exactly that translated location
        // ({rootfs_prefix}/dev/), so the env path resolves in both
        // modes. (Before the aosp3 fix the parent wrote to the HOST
        // /dev — EACCES — while {rootfs}/dev stayed empty →
        // "CANNOT LINK EXECUTABLE … library /dev/libgetpid_hook.so not
        // found" → exit(1).)
        //
        // use_namespaces=true: pivot_root has happened, /dev/ is the
        // tmpfs mounted by setup_mounts — required for SELinux too:
        // vendor_init subcontexts are denied search on app_data_file,
        // while tmpfs is accessible to ALL domains.
        // use_namespaces=false: /dev/ resolves via the interceptor to
        // {cfg.rootfs}/dev/ where the parent staged the libraries.
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
            if compat_shim_staged {
                // 6-Z236: host libs staged → the FORTIFY shim rides the
                // init env chain too (inert for static init, satisfies
                // __*_chk for dynamic init + the forked recovery).
                "LD_PRELOAD=/sbin/libbionic_compat.so:/sbin/libtwrp_fb_hook.so".to_string()
            } else {
                "LD_PRELOAD=/sbin/libtwrp_fb_hook.so".to_string()
            }
        } else {
            // 6-Z216: append the FB hook so AOSP-layout recoveries
            // (launched by init from /system/bin/recovery, never touched
            // by the TWRP init.rc patch) get framebuffer virtualization.
            // Inert for processes that never open /dev/graphics/fb0.
            //
            // 6-Z218a ORDER FIX: the FB hook MUST come BEFORE
            // libtwoyi_loader_shlib.so. bionic resolves a caller's PLT
            // entry against the executable, then the LD_PRELOAD libs IN
            // ORDER, then DT_NEEDED. libtwoyi_loader_shlib.so exports
            // open/openat/__open_2/__openat_2/close/ioctl — with the old
            // order (shlib before fb hook) every libminuitwrp PLT entry
            // for those names resolved to the SHLIB, the FB hook was
            // completely shadowed, /dev/graphics/fb0 opens went
            // untracked, FBIOGET_VSCREENINFO failed, gr_init() failed
            // and the theme engine crash-looped in gr_fb_width()
            // (libminuitwrp.so file offset 0x2027c, si_addr=0x0 — 26
            // crashes in run 33269270911). With the FB hook first, its
            // hooks special-case ONLY fb0/input/ashmem fds and chain
            // everything else through dlsym(RTLD_NEXT) → the shlib, so
            // path translation and property-area virtualization are
            // preserved for every other fd. Order asserted by
            // assert_aosp_preload_order() + AOSP_LD_PRELOAD_ENV.
            if compat_shim_staged {
                // 6-Z236: shim FIRST so its FORTIFY exports are visible
                // to every later library (LD_PRELOAD load order = PLT
                // search order).
                format!(
                    "LD_PRELOAD=/dev/libbionic_compat.so:{}",
                    AOSP_LD_PRELOAD_ENV.trim_start_matches("LD_PRELOAD=")
                )
            } else {
                AOSP_LD_PRELOAD_ENV.to_string()
            }
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
                // 6-Z161: append :/system/lib64 — the angler recovery is
                // arm64 (PT_INTERP /sbin/linker64, ELF64) and its libs
                // resolve from /sbin first, but adbd + any other dynamic
                // service that inherits THIS env needs the 64-bit dir in
                // the search path as the fallback (run 32983937665:
                // "Service 'adbd' (pid 2620) exited with status 1" —
                // every instant exit-1 is a linker failure family
                // symptom; the x86-era "/sbin:/system/lib" value left
                // the arm64 service with NO 64-bit fallback dir).
                CString::new("LD_LIBRARY_PATH=/sbin:/system/lib:/system/lib64").unwrap(),
                // 6-Z171b: native-resolution passthrough. The TWRP child's
                // libtwrp_fb_hook.so reads TWOYI_FB_WIDTH/TWOYI_FB_HEIGHT at
                // first use and synthesizes FBIOGET_VSCREENINFO /
                // FBIOGET_FSCREENINFO geometry to MATCH — no compile-time
                // hardcode anywhere in the chain (Java auto-detect ->
                // renderer_init -> core.rs --width/--height -> kr64 cfg ->
                // here + create_twrp_framebuffer's file size). Values <= 0
                // (unset) pass redroid's own default panel (320x640) so the
                // hook's fallback and the fb0 file size always agree.
                CString::new(format!(
                    "TWOYI_FB_WIDTH={}",
                    if cfg.width > 0 { cfg.width } else { 320 }
                ))
                .unwrap(),
                CString::new(format!(
                    "TWOYI_FB_HEIGHT={}",
                    if cfg.height > 0 { cfg.height } else { 640 }
                ))
                .unwrap(),
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
                    //
                    // 5-K REFUTATION (kr64-stderr.log line 81): the above
                    //   assumption is WRONG —
                    //   /apex/com.android.runtime/lib64/bionic/libdl.so is
                    //   ALSO the 5848-byte bootstrap stub (not the real
                    //   one). Both paths return the SAME 5848-byte stub.
                    //   The REAL libdl.so lives INSIDE the APEX ext4 image
                    //   at /system/apex/com.android.runtime.apex (not
                    //   extracted by `tar cf apex/`).
                    //
                    // 5-L FIX: extract the real libdl.so from the APEX
                    //   ext4 image (see apex_extract.rs + Step 3.7 in
                    //   run()) and write it to /dev/libdl.so BEFORE
                    //   fork. Prepend /dev/ to LD_LIBRARY_PATH so the
                    //   linker finds /dev/libdl.so (the real one) before
                    //   falling back to the stub at
                    //   /apex/com.android.runtime/lib64/bionic/libdl.so.
                    //
                    //   If extraction failed (real_libdl is None) and
                    //   /dev/libdl.so doesn't exist, the linker just
                    //   falls through to the next LD_LIBRARY_PATH entry
                    //   (/apex/com.android.runtime/lib64/bionic = the
                    //   stub) — same behavior as before this fix.
                    "LD_LIBRARY_PATH=\
                /dev:\
                /apex/com.android.runtime/lib64/bionic:\
                /apex/com.android.runtime/lib64:\
                /apex/com.android.runtime/lib64/bootstrap:\
                /apex/com.android.art/lib64:\
                /apex/com.android.i18n/lib64:\
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
        //
        // 6-Z134: FILE-BASED TRIGGER — the app's env cannot be set from CI
        // (the app launches kr64 via ProcessBuilder with its own env), so
        // also check the marker file {data_dir}/.ld_debug whose CONTENT is
        // the LD_DEBUG value (e.g. "2"). The nav script creates it via
        // run-as before launching the container — the run's artifact then
        // carries the linker's own account of every library search/open/
        // read (the ground truth for the "only found 1 bytes" /
        // "file size 0 >= 0" link-failure class).
        let ld_debug_from_file: Option<String> = {
            let marker = format!("{}/.ld_debug", cfg.data_dir);
            std::fs::read_to_string(&marker)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        if ld_debug_from_file.is_some() {
            unsafe {
                safe_write_err(
                    b"[KR64 CHILD] .ld_debug marker present -- enabling LD_DEBUG for guest init (file trigger)\n",
                );
            }
        }
        let ld_debug_val = std::env::var("TWOYI_LD_DEBUG")
            .ok()
            .filter(|s| !s.is_empty())
            .or(ld_debug_from_file);
        if let Some(ld_debug_val) = ld_debug_val {
            let ld_debug_env = format!("LD_DEBUG={}", ld_debug_val);
            match CString::new(ld_debug_env) {
                Ok(c) => {
                    env_vars.push(c);
                    unsafe {
                        safe_write_err(b"[KR64 CHILD] LD_DEBUG enabled for guest init\n");
                    }
                }
                Err(_) => unsafe {
                    safe_write_err(b"[KR64 CHILD] WARN: LD_DEBUG value contains NUL byte -- skipping LD_DEBUG\n");
                },
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

        // 6-Z187b: set the GUEST INIT's working directory to the rootfs
        // BEFORE execve. Run 33119446980 proved the UI recovery (init's
        // service fork) inherits the APP's cwd (host "/") — every
        // cwd-relative fallback then resolved against the WRONG directory:
        // the fb_hook's via=2 retries failed 166x with ENOENT, the terminal
        // execl("/sbin/sh") +1 rewrite resolved to the host's /sbin/sh
        // (ENOENT → exit 127 → "Child processes exited."), and any
        // path+1 open fallback missed. With cwd == {rootfs}, init and every
        // process it forks resolve rootfs-relative paths CORRECTLY, and a
        // guest chdir("/") is TRANSLATED by the tracer back to {rootfs}
        // anyway — the invariant is self-healing.
        {
            let root_c = std::ffi::CString::new(cfg.rootfs.as_str()).unwrap_or_default();
            let cr = unsafe { libc::chdir(root_c.as_ptr()) };
            unsafe {
                if cr == 0 {
                    safe_write_err(
                        b"[KR64 CHILD] 6-Z187b: chdir(rootfs) OK - guest cwd is the sandbox root\n",
                    );
                } else {
                    let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                    safe_write_err_errno(b"[KR64 CHILD] 6-Z187b: chdir(rootfs) FAILED errno=", e);
                    safe_write_err(b" (cwd-relative fallbacks may miss)\n");
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

    // Spawn the touch device accept thread EARLY (before the ptrace loop
    // or the waitpid loop) so the guest's EventHub can probe
    // `/dev/input/touch` during TWRP init OR full-Android boot. The
    // thread runs `devices::make_touch_device` + the encode_touch_*
    // helpers added by 2-B (commit `370b8ee`) to send the DeviceInfo
    // header (896 bytes) on accept, then forward encoded InputEvents
    // from the host's `{data_dir}/dev/touch-events` IPC socket.
    // 6-Z94: TWRP mode — the fb hook's INPUT bridge is the sole client of
    // the host touch socket (last-accept-wins would starve it)
    if !cfg.boot_recovery {
        spawn_touch_accept_thread(device_set.touch, cfg.clone());
    }

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
            //
            // We construct the Vfs here. Task 6-Z88: TWRP boots use
            // new_twrp() (the AOSP 5.1 single-file /dev/__properties__
            // override); NORMAL (AOSP) boots use new_android(pid) — the
            // Android-guest entries (/dev/__properties__/properties_serial
            // + /proc/self/* Dynamic nodes) keyed to the ACTUAL guest pid
            // (init), which is what the 8.1 bionic linker + zygote will
            // look up. The ptrace_emu's open/openat ENTRY-stop handler
            // asks the Vfs to materialise synthetic files into rootfs
            // before the real kernel open() runs, replacing the
            // find_property binary patch (worklog 1-A F.1 + 1-B Task 3).
            let vfs = if cfg.boot_recovery && !guest_new_prop_format {
                vfs::Vfs::new_twrp()
            } else if cfg.boot_recovery {
                // 6-Z192: a recovery whose init speaks the NEW Android 8+
                // property-area format (properties_serial/property_info in
                // the init binary — e.g. twrp-3.7.0_9-0-whyred). Such an
                // init OWNS the property area: it writes property_info and
                // creates properties_serial itself. new_twrp()'s
                // old-format FILE would ENOTDIR it, and new_android()'s
                // Dynamic properties_serial node would CLOBBER the
                // freshly-written area on every open (materialize-on-open)
                // → the empty/garbage area fails LoadPath's size check →
                // "Failed to initialize property area" → init exit. The
                // new_recovery_new_format() VFS registers NO property
                // entries: plain rootfs files, guest-owned end to end.
                info!("[KR64] PARENT: recovery boot with NEW-format init — using Vfs::new_recovery_new_format(pid={}) (Task 6-Z192)", pid);
                vfs::Vfs::new_recovery_new_format(pid as u32)
            } else {
                info!("[KR64] PARENT: normal (AOSP) boot — using Vfs::new_android(pid={}) (Task 6-Z88)", pid);
                vfs::Vfs::new_android(pid as u32)
            };

            // Task 6-Z49: proactively fork the recovery child BEFORE the
            // ptrace loop starts. The re-spawn cycle (kr64 re-forks every
            // ~2s) kills kr64 before init reaches the recovery service
            // execve (at syscall #466). By forking the recovery child
            // directly, we don't need init to start the service.
            //
            // Task 6-Z88: TWRP-ONLY. The whole block (PT_INTERP patching
            // of {rootfs}/sbin/recovery + the fork itself) is gated behind
            // cfg.boot_recovery: in normal (AOSP) mode the guest boots via
            // init + zygote — there is no /sbin/recovery, and the fork
            // just execve-failed + exit(127)'d on EVERY attempt (a second
            // doomed child, pid 5993, in run 32632668179) while the
            // PT_INTERP patcher was mutating the Android rootfs's
            // binaries. Normal mode gets recovery_pid = None.
            let recovery_pid = if cfg.boot_recovery {
                let recovery_path = format!("{}/sbin/recovery", cfg.rootfs);

                // Task 6-Z50 (+ 6-Z157 ELF64 fix): read the recovery
                // binary's PT_INTERP and patch it to the GUEST rootfs's
                // own bionic linker. The recovery binary is dynamically
                // linked with PT_INTERP = /sbin/linker (32-bit builds)
                // or /sbin/linker64 (64-bit builds) — rootfs-relative
                // paths the HOST kernel cannot resolve at execve time.
                // We patch PT_INTERP to the absolute host path
                // {rootfs}/sbin/linker{,64} so the kernel finds the
                // linker SHIPPED IN THE SAME RAMDISK (matching API level
                // — using the host's own /system/bin/linker64 instead
                // would run e.g. an Android-14 linker against an
                // Android-6-era recovery: "CANNOT LINK" → exit 127,
                // observed on arm64 run 32973154137).
                //
                // 6-Z157: the parser previously used ELF32 header
                // offsets unconditionally (e_phoff from byte 28,
                // e_phentsize 42, e_phnum 44). On the aarch64 (ELF64)
                // TWRP recovery that read garbage → "no PT_INTERP
                // found" → the binary was left UNPATCHED → staged
                // execve failed with ENOENT/127. Now: EI_CLASS decides
                // the layout AND the linker variant:
                //   ELF32 (EI_CLASS=1): e_phoff@28(u32) e_phentsize@42
                //     e_phnum@44; phdr p_offset@+4(u32) p_filesz@+16(u32)
                //   ELF64 (EI_CLASS=2): e_phoff@32(u64) e_phentsize@54
                //     e_phnum@56; phdr p_offset@+8(u64) p_filesz@+32(u64)
                let patched_path = {
                    // Open the recovery binary for read+write to patch PT_INTERP
                    match std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&recovery_path)
                    {
                        Ok(mut file) => {
                            use std::io::{Read, Seek, Write};
                            let mut ehdr = [0u8; 64];
                            if file.read_exact(&mut ehdr).is_ok() && &ehdr[0..4] == b"\x7fELF" {
                                // 6-Z157: class-aware ELF header parse.
                                let is_elf64 = ehdr[4] == 2;
                                let (e_phoff, e_phentsize, e_phnum): (u64, usize, usize) =
                                    if is_elf64 {
                                        (
                                            u64::from_le_bytes([
                                                ehdr[32], ehdr[33], ehdr[34], ehdr[35], ehdr[36],
                                                ehdr[37], ehdr[38], ehdr[39],
                                            ]),
                                            u16::from_le_bytes([ehdr[54], ehdr[55]]) as usize,
                                            u16::from_le_bytes([ehdr[56], ehdr[57]]) as usize,
                                        )
                                    } else {
                                        (
                                            u32::from_le_bytes([
                                                ehdr[28], ehdr[29], ehdr[30], ehdr[31],
                                            ]) as u64,
                                            u16::from_le_bytes([ehdr[42], ehdr[43]]) as usize,
                                            u16::from_le_bytes([ehdr[44], ehdr[45]]) as usize,
                                        )
                                    };
                                // 6-Z157: phdr field offsets by class.
                                let (p_off_field, p_sz_field, sz_bytes): (u64, u64, usize) =
                                    if is_elf64 { (8, 32, 8) } else { (4, 16, 4) };
                                // 6-Z184 AUDIT FIX (agent 7): a corrupt or
                                // malicious recovery image could carry
                                // e_phentsize=0 (→ empty vec → index panic) or
                                // an e_phentsize*e_phnum product large enough
                                // to OOM-abort the tracer — which kills every
                                // traced child via PTRACE_O_EXITKILL. Validate
                                // the program-header table before allocating.
                                let min_phentsize: usize = if is_elf64 { 56 } else { 32 };
                                // file_len via metadata (the later file_len
                                // binding below is scoped to the interp
                                // rewrite and not visible here).
                                let file_len_hdr =
                                    file.metadata().map(|m| m.len()).unwrap_or(u64::MAX);
                                let table_ok = e_phnum > 0
                                    && e_phnum <= 65535
                                    && e_phentsize >= min_phentsize
                                    && e_phentsize <= 4096
                                    && !e_phoff
                                        .checked_add((e_phentsize * e_phnum) as u64)
                                        .map_or(true, |end| end > file_len_hdr);
                                // 6-Z184: hoisted so the append path below
                                // (which re-scans the phdr table) can use it;
                                // only populated when table_ok.
                                let mut phdrs_all: Vec<u8> = Vec::new();
                                let interp: Option<(u64, usize)> = if !table_ok {
                                    warning!(
                                        "[KR64] PT_INTERP scan: implausible phdr table (phoff={}, phentsize={}, phnum={}, file_len={}) — skipping patch for this binary",
                                        e_phoff,
                                        e_phentsize,
                                        e_phnum,
                                        file_len_hdr
                                    );
                                    None
                                } else {
                                    phdrs_all = vec![0u8; e_phentsize * e_phnum];
                                    let _ = file.seek(std::io::SeekFrom::Start(e_phoff));
                                    let _ = file.read_exact(&mut phdrs_all);
                                    let phdrs = &phdrs_all;
                                    let mut interp_offset = None;
                                    let mut interp_filesz = None;
                                    for i in 0..e_phnum {
                                        let off = i * e_phentsize;
                                        let Some(p_type) = phdrs
                                            .get(off..off + 4)
                                            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                                        else {
                                            break;
                                        };
                                        if p_type == 3 {
                                            let read_u =
                                                |base: usize, field: u64, width: usize| -> u64 {
                                                    let s = base + field as usize;
                                                    let mut v = 0u64;
                                                    for b in (0..width).rev() {
                                                        v = (v << 8) | phdrs[s + b] as u64;
                                                    }
                                                    v
                                                };
                                            interp_offset =
                                                Some(read_u(off, p_off_field, sz_bytes));
                                            interp_filesz =
                                                Some(read_u(off, p_sz_field, sz_bytes) as usize);
                                            break;
                                        }
                                    }
                                    match (interp_offset, interp_filesz) {
                                        (Some(o), Some(s)) => Some((o, s)),
                                        _ => None,
                                    }
                                };
                                if let Some((p_offset, p_filesz)) = interp {
                                    let _ = file.seek(std::io::SeekFrom::Start(p_offset));
                                    let mut interp_buf = vec![0u8; p_filesz];
                                    let _ = file.read_exact(&mut interp_buf);
                                    let interp_str = String::from_utf8_lossy(&interp_buf);
                                    info!("[KR64] Task 6-Z50: PT_INTERP offset={}, filesz={}, path={:?} (ELF{})",
                                        p_offset, p_filesz, interp_str.trim_end_matches('\0'),
                                        if is_elf64 { 64 } else { 32 });
                                    // 6-Z157: linker variant by ELF class —
                                    // 32-bit binaries use /sbin/linker,
                                    // 64-bit use /sbin/linker64. Both live
                                    // in the TWRP ramdisk (same cpio as the
                                    // recovery binary itself).
                                    let linker_name = if is_elf64 { "linker64" } else { "linker" };
                                    let guest_linker =
                                        format!("{}/sbin/{}", cfg.rootfs, linker_name);
                                    if !std::path::Path::new(&guest_linker).exists() {
                                        error!("[KR64] Task 6-Z157: guest linker {} NOT found in rootfs — PT_INTERP left unpatched (execve of the staged binary will ENOENT)",
                                            guest_linker);
                                    }
                                    let new_interp = format!("{}\0", guest_linker);
                                    let new_interp_path =
                                        new_interp.trim_end_matches('\0').to_string();
                                    if new_interp.len() <= p_filesz {
                                        // Fits in place — overwrite
                                        let _ = file.seek(std::io::SeekFrom::Start(p_offset));
                                        let mut nb = new_interp.into_bytes();
                                        while nb.len() < p_filesz {
                                            nb.push(0);
                                        }
                                        let _ = file.write_all(&nb);
                                        info!("[KR64] Task 6-Z50: patched PT_INTERP in-place to {} ({} bytes)",
                                            new_interp_path, p_filesz);
                                    } else {
                                        // Doesn't fit — APPEND the new interp at the end of the file
                                        // and update the PT_INTERP program header to point to it.
                                        //
                                        let file_len = {
                                            let mut end = 0u64;
                                            let _ = file.seek(std::io::SeekFrom::End(0));
                                            if let Ok(pos) = file.stream_position() {
                                                end = pos;
                                            }
                                            end
                                        };
                                        let new_offset = file_len;
                                        // pad to 8-byte alignment for safety
                                        let new_offset = (new_offset + 7) & !7u64;
                                        let new_filesz = new_interp.len();
                                        let _ = file.seek(std::io::SeekFrom::Start(new_offset));
                                        let w0 = file.write_all(new_interp.as_bytes());
                                        if w0.is_err() {
                                            error!("[KR64] Task 6-Z50: append write FAILED");
                                        } else {
                                            info!("[KR64] Task 6-Z50: appended new PT_INTERP at offset {} ({} bytes)",
                                        new_offset, new_filesz);
                                            // Update the PT_INTERP program header's p_offset and
                                            // p_filesz (6-Z157: class-aware field offsets + widths)
                                            let mut pt_interp_phdr_off = None;
                                            let phdrs = &phdrs_all;
                                            for i in 0..e_phnum {
                                                let off = i * e_phentsize;
                                                let Some(p_type) =
                                                    phdrs.get(off..off + 4).map(|b| {
                                                        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
                                                    })
                                                else {
                                                    break;
                                                };
                                                if p_type == 3 {
                                                    pt_interp_phdr_off = Some(e_phoff + off as u64);
                                                    break;
                                                }
                                            }
                                            if let Some(phdr_off) = pt_interp_phdr_off {
                                                let mut wr = |field_off: u64, width: usize, val: u64| -> std::io::Result<()> {
                                                    let _ = file.seek(std::io::SeekFrom::Start(phdr_off + field_off));
                                                    let bytes = val.to_le_bytes();
                                                    file.write_all(&bytes[..width])
                                                };
                                                // Write new p_offset + p_filesz
                                                let w1 = wr(p_off_field, sz_bytes, new_offset);
                                                let w2 =
                                                    wr(p_sz_field, sz_bytes, new_filesz as u64);
                                                if w1.is_err() || w2.is_err() {
                                                    error!("[KR64] Task 6-Z65: PT_INTERP phdr update FAILED (w1={:?}, w2={:?}) — kernel will use the OLD interpreter path", w1.err(), w2.err());
                                                } else {
                                                    info!("[KR64] Task 6-Z50: updated PT_INTERP phdr: p_offset={}, p_filesz={}",
                                                        new_offset, new_filesz);
                                                    // Task 6-Z65: READ-BACK verification — re-read the
                                                    // phdr + interp from disk (bypassing the File's buffer
                                                    // via a fresh open) and log exactly what a subsequent
                                                    // execve would see. (6-Z157: class-aware parse.)
                                                    // 6-Z268: WINDOWED read-back — the old
                                                    // `std::fs::read(&recovery_path)` slurped the ENTIRE
                                                    // multi-MB recovery binary through FUSE while the
                                                    // guest sat SIGSTOPped on the loop-entry critical
                                                    // path, only to parse the phdr table + one interp
                                                    // string. Two bounded reads (64-byte ehdr + phdr
                                                    // table, then the interp window) verify the same
                                                    // bytes at ~4 KiB of I/O.
                                                    let verify = (|| -> Option<(usize, usize, String, usize)> {
                                                        use std::io::{Read, Seek, SeekFrom};
                                                        let mut vf = std::fs::File::open(&recovery_path).ok()?;
                                                        let file_len = vf
                                                            .metadata()
                                                            .ok()
                                                            .map(|m| m.len() as usize)
                                                            .unwrap_or(0);
                                                        let mut vhdr = [0u8; 64];
                                                        vf.read_exact(&mut vhdr).ok()?;
                                                        let ve64 = vhdr[4] == 2;
                                                        let v_e_phoff: u64 = if ve64 {
                                                            let mut a = [0u8; 8]; a.copy_from_slice(&vhdr[32..40]); u64::from_le_bytes(a)
                                                        } else {
                                                            let mut a = [0u8; 4]; a.copy_from_slice(&vhdr[28..32]); u32::from_le_bytes(a) as u64
                                                        };
                                                        let v_e_phentsize = if ve64 {
                                                            u16::from_le_bytes([vhdr[54], vhdr[55]]) as usize
                                                        } else {
                                                            u16::from_le_bytes([vhdr[42], vhdr[43]]) as usize
                                                        };
                                                        let v_e_phnum = if ve64 {
                                                            u16::from_le_bytes([vhdr[56], vhdr[57]]) as usize
                                                        } else {
                                                            u16::from_le_bytes([vhdr[44], vhdr[45]]) as usize
                                                        };
                                                        if v_e_phnum == 0 || v_e_phnum > 64 || v_e_phentsize == 0 || v_e_phentsize > 128 {
                                                            return None;
                                                        }
                                                        let (v_off_f, v_sz_f, v_w): (u64, u64, usize) = if ve64 { (8, 32, 8) } else { (4, 16, 4) };
                                                        let phdr_bytes = v_e_phnum * v_e_phentsize;
                                                        let mut vphdrs = vec![0u8; phdr_bytes];
                                                        vf.seek(SeekFrom::Start(v_e_phoff)).ok()?;
                                                        vf.read_exact(&mut vphdrs).ok()?;
                                                        for i in 0..v_e_phnum {
                                                            let off = i * v_e_phentsize;
                                                            if off + 4 > vphdrs.len() { break; }
                                                            let p_type = u32::from_le_bytes([vphdrs[off], vphdrs[off + 1], vphdrs[off + 2], vphdrs[off + 3]]);
                                                            if p_type == 3 {
                                                                let base = off;
                                                                let rd = |field: u64, width: usize| -> usize {
                                                                    let s = base + field as usize;
                                                                    let mut v = 0usize;
                                                                    for b in (0..width).rev() { v = (v << 8) | (vphdrs[s + b] as usize); }
                                                                    v
                                                                };
                                                                let v_off = rd(v_off_f, v_w);
                                                                let v_sz = rd(v_sz_f, v_w);
                                                                let read_len = v_sz.min(4096);
                                                                let mut interp = vec![0u8; read_len];
                                                                vf.seek(SeekFrom::Start(v_off as u64)).ok()?;
                                                                vf.read_exact(&mut interp).ok()?;
                                                                return Some((v_off, v_sz, String::from_utf8_lossy(&interp).trim_end_matches('\0').to_string(), file_len));
                                                            }
                                                        }
                                                        None
                                                    })();
                                                    match verify {
                                                        Some((v_off, v_sz, v_str, v_len)) => {
                                                            if v_str == new_interp_path {
                                                                info!("[KR64] Task 6-Z65: READ-BACK VERIFIED — PT_INTERP @{} ({} bytes) = {:?} (file size {}) — execve will find the rootfs linker",
                                                                    v_off, v_sz, v_str, v_len);
                                                            } else {
                                                                error!("[KR64] Task 6-Z65: READ-BACK MISMATCH — PT_INTERP @{} ({} bytes) = {:?} (expected {:?}, file size {}) — the patch did NOT persist!",
                                                                    v_off, v_sz, v_str, new_interp_path, v_len);
                                                            }
                                                        }
                                                        None => error!("[KR64] Task 6-Z65: read-back verification FAILED — could not re-parse the patched ELF"),
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => error!("[KR64] Task 6-Z50: can't open recovery binary: {}", e),
                    }
                    recovery_path.clone()
                };

                // 6-Z256: the envp is now built by the pure builder —
                // the old hardcoded 4-entry envp (LD_PRELOAD,
                // LD_LIBRARY_PATH, PATH, TWOYI_ROOTFS) is exactly the
                // `total_entries=4, std-vars 0/4` the 6-Z238/6-Z255 env
                // scan observed on the 20-build OrangeFox strlen(NULL)
                // class (run 33323583991): init never forks recovery for
                // TWRP-family boots (this proactive fork IS the exec),
                // so init.rc's `export ANDROID_ROOT /system` never
                // reached the process. The builder layers the twoyi
                // virtualization stack, the guest's own rc `export`
                // lines (guest-owned truth), then the standard Android
                // defaults — see ANDROID_STD_ENV_DEFAULTS.
                let guest_rc_exports = collect_guest_rc_exports(&rootfs_prefix);
                let recovery_envp =
                    build_recovery_service_envp(&cfg.rootfs, compat_shim_staged, &guest_rc_exports);
                info!(
                    "[KR64] Task 6-Z256: recovery child envp ({} entries, {} rc exports): {:?}",
                    recovery_envp.len(),
                    guest_rc_exports.len(),
                    recovery_envp
                );
                info!(
                    "[KR64] Task 6-Z49: proactively forking recovery child at {}",
                    patched_path
                );

                let path_c = std::ffi::CString::new(recovery_path.as_str()).unwrap_or_default();
                let argv_c: Vec<std::ffi::CString> =
                    vec![std::ffi::CString::new("/sbin/recovery").unwrap_or_default()];
                let envp_c: Vec<std::ffi::CString> = recovery_envp
                    .iter()
                    .filter_map(|s| std::ffi::CString::new(s.as_str()).ok())
                    .collect();
                let mut argv_ptr: Vec<*const libc::c_char> =
                    argv_c.iter().map(|s| s.as_ptr()).collect();
                argv_ptr.push(std::ptr::null());
                let mut envp_ptr: Vec<*const libc::c_char> =
                    envp_c.iter().map(|s| s.as_ptr()).collect();
                envp_ptr.push(std::ptr::null());

                let new_pid = unsafe { libc::fork() };
                if new_pid == 0 {
                    // Child — 64-bit, async-signal-safe only.
                    // 6-Z184 AUDIT FIX (agent 7): sanitize fds BEFORE
                    // TRACEME/execve — the parent's binder proxy holds a
                    // non-CLOEXEC /dev/binder fd plus log/config fds; the
                    // first child closes 3..fd_limit for exactly this
                    // reason, this child used to leak them into the
                    // guest recovery process.
                    unsafe {
                        libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0);
                        for fd in 3..1024i32 {
                            libc::close(fd);
                        }
                    }
                    unsafe {
                        libc::raise(libc::SIGSTOP);
                    }
                    // 6-Z187b: cwd == rootfs for this child too (see the
                    // init-child chdir above — the via=2/+1 cwd-relative
                    // fallbacks depend on it).
                    unsafe {
                        let root_c =
                            std::ffi::CString::new(cfg.rootfs.as_str()).unwrap_or_default();
                        if libc::chdir(root_c.as_ptr()) != 0 {
                            let msg = "6-Z187b: chdir(rootfs) FAILED in recovery child\n";
                            libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
                        }
                    }
                    let execve_ret = unsafe {
                        libc::execve(path_c.as_ptr(), argv_ptr.as_ptr(), envp_ptr.as_ptr())
                    };
                    // execve returned → it FAILED. Log via write(2,...) — async-signal-safe.
                    let msg = format!(
                        "EXECVE FAILED: ret={}, path={}\n",
                        execve_ret, recovery_path
                    );
                    unsafe {
                        libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
                    }
                    // Check if the interpreter (PT_INTERP) exists on the HOST
                    for interp in &[
                        "/sbin/linker",
                        "/system/bin/linker",
                        "/data/user/0/io.twoyi/rootfs/sbin/linker",
                    ] {
                        let interp_c = std::ffi::CString::new(*interp).unwrap_or_default();
                        let access_ret = unsafe { libc::access(interp_c.as_ptr(), libc::F_OK) };
                        let msg2 = format!("INTERP check: {} exists={}\n", interp, access_ret == 0);
                        unsafe {
                            libc::write(2, msg2.as_ptr() as *const libc::c_void, msg2.len());
                        }
                    }
                    unsafe {
                        libc::_exit(127);
                    }
                } else if new_pid > 0 {
                    info!("[KR64] Task 6-Z49: forked recovery child PID={}", new_pid);
                    Some(new_pid)
                } else {
                    error!("[KR64] Task 6-Z49: fork FAILED for recovery child");
                    None
                }
            } else {
                // Task 6-Z88: normal (AOSP) mode — no /sbin/recovery, no
                // PT_INTERP patching, no doomed fork. recovery_pid = None.
                info!("[KR64] PARENT: boot_recovery=false — skipping TWRP recovery child fork + PT_INTERP patch (Task 6-Z88)");
                None
            };

            // 6-Z102 PART D: pass `&cfg.data_dir` so the ptrace loop's
            // generic-staging engine (stage_guest_executable) can copy guest
            // ROM binaries to {data_dir}/cache/twoyi_stage/ and exec them
            // from there (the rootfs is on the noexec app-data partition;
            // the app cache dir is the ONE executable place we own).
            let exit_code = ptrace_emu::run_ptrace_loop(
                pid,
                &cfg.rootfs,
                &cfg.data_dir,
                &vfs,
                recovery_pid,
                cfg.boot_recovery,
            );
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
            // Android 11 scoped storage further restricts the legacy
            // mirror target `/sdcard/Android/data/io.twoyi/files/` —
            // `adb pull` from that path is unreliable on release builds
            // (the path is owned by the app and not browsable from
            // outside). The public Downloads directory
            // `/sdcard/Download/twoyi-logs/` IS readable via `adb pull`
            // without root on all builds (it's a MediaProvider-
            // managed shared collection, not an app-specific external
            // dir), so we mirror the logs there once the child has
            // exited.
            //
            // For debuggable builds (task 3-A Part 3 adds a twoyiDebug
            // flavor) `adb shell run-as io.twoyi.debug cat
            // /data/user/0/io.twoyi.debug/rootfs/twrp-init.log` is the
            // canonical fallback for poking around the guest rootfs.
            // -------------------------------------------------------------
            {
                let ext_files_dir = "/sdcard/Download/twoyi-logs";
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
    //
    // NOTE: `device_set.touch` is no longer passed here — it was
    // consumed by `spawn_touch_accept_thread` above (which sends the
    // DeviceInfo header + streams encoded InputEvents, using the
    // helpers from 2-B's commit `370b8ee`). The other four devices
    // still use the generic stub.
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
    // 6-Z268: BLOCKING accept. The thread's lifetime is the process's
    // (there is no shutdown role — "we just loop forever"), so the old
    // O_NONBLOCK + 50 ms poll bought nothing but +50 ms of connect
    // latency per guest connect and 20 wakeups/s of scheduler noise on
    // a box whose tracer thread is latency-critical. Plain accept()
    // parks the thread in the kernel until a guest connects.

    std::thread::Builder::new()
        .name(format!("kr64-accept-{}", name))
        .spawn(move || {
            let fd = listener.as_raw_fd();
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
// Property service stub — Task 6-H.
//
// Background: TWRP init (AOSP 5.1 bionic) loops on pause() (i386 syscall 29)
// waiting for the property service to signal readiness. ALL return-value
// tricks tried in 6-D / 6-E / 6-F / 6-G FAILED to break this loop:
//   * 0 (pre-6-D)         — init thought "pause completed without a signal"
//                            → re-checked condition → looped.
//   * -EINTR (6-D)         — init thought "interrupted by signal" → re-checked
//                            condition → looped.
//   * -ENOSYS (6-E/6-G)   — init did NOT fall back to a non-pause wait (it
//                            just retried pause()).
//   * -ETIMEDOUT (6-G)    — same as -ENOSYS (still retried).
//   * + 100ms sleep (6-F) — reduced CPU spin from 659k/sec but did not
//                            break the loop.
//
// Root cause (per DISPATCHER-STATUS-FINAL): init fundamentally requires
// the property SERVICE to exist + accept connections, not just "pause
// returned". The find_property binary patch (5-Y / 6-B) handles property
// LOOKUPS (returns NULL), but init ALSO waits for the service to send a
// "ready" signal via the property socket/pipe.
//
// This stub:
//   1. Creates /dev/socket/property_service Unix socket (mode 0666) in
//      the rootfs. AOSP 5.1 bionic's `send_prop_msg` (system_properties.c)
//      hard-codes the socket path as `/dev/socket/property_service`.
//   2. Listens for connections (init's start_property_service() OR its
//      clients' __system_property_set() connect here).
//   3. Accepts connections, reads a 128-byte `prop_msg_t` (cmd:4 +
//      name[32] + value[92] — AOSP 5.1 bionic's send_prop_msg format),
//      and writes a 4-byte "0" (PROP_SUCCESS) response. This makes the
//      client's send_prop_msg() succeed (read() returns 4 bytes =
//      sizeof(int)).
//
// The stub does NOT actually store / lookup property values:
//   * `find_property` is binary-patched (5-Y / 6-B) to return NULL, so
//     in-process lookups never consult the socket.
//   * The 4-byte "0" response just satisfies send_prop_msg's contract
//     ("server received + acked") so init's setprop calls succeed.
//
// KEY UNCERTAINTY: the property service in AOSP 5.1 init runs in init's
// OWN process (as a thread/function), NOT as a separate fork. So the
// "ready" signal may be a simple flag or a condition variable, not a
// socket message. IF that is the case, this stub may NOT break the
// pause() loop — init would still be waiting on the in-process flag.
// The only way to verify is a ui-e2e-test.yml run + VLM log analysis
// (which the dispatcher must trigger separately — this task only
// implements + tests the stub).
//
// CONFLICT NOTE: in non-root ptrace_emu mode (UI E2E), init's own
// start_property_service() tries to bind() /dev/socket/property_service
// directly. The host kernel rejects this with EACCES (untrusted_app can't
// write to /dev/socket/ on the host). So init's bind FAILS — but our stub
// binds the rootfs-path successfully (kr64 has access to {rootfs}/dev/).
// In root mode (use_namespaces=true), the loader's bind() hook would
// redirect init's bind to {rootfs}/dev/socket/property_service — which
// would then fail with EADDRINUSE because our stub owns the kernel-level
// socket. THIS IS A KNOWN LIMITATION of this stub: it conflicts with
// init's own bind() in root mode. For the UI E2E (non-root, ptrace_emu),
// the stub is safe (init's bind fails with EACCES independently).
// ============================================================================

/// AOSP 5.1 bionic's `PROP_SERVICE_NAME` constant — the basename of the
/// socket file at `/dev/socket/property_service`. Verified against
/// `bionic/libc/include/sys/system_properties.h`:
///   ```c
///   #define PROP_SERVICE_NAME "property_service"
///   ```
/// TWRP 3.7.0_9 (the boot image at `assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img`)
/// uses AOSP 5.1 bionic which has this same constant.
const PROP_SERVICE_SOCKET_NAME: &str = "property_service";

/// `sizeof(prop_msg_t)` in AOSP 5.1 bionic. Layout
/// (from `bionic/libc/include/sys/system_properties.h`):
///   ```c
///   #define PROP_NAME_MAX  32
///   #define PROP_VALUE_MAX 92
///   struct prop_msg {
///       unsigned cmd;          //  4 bytes
///       char name[PROP_NAME_MAX];   // 32 bytes
///       char value[PROP_VALUE_MAX]; // 92 bytes
///   };
///   ```
/// Size of the AOSP 5.1 `prop_msg_t` struct (cmd:4 + name[32] + value[92]).
/// Total = 4 + 32 + 92 = 128 bytes. `send_prop_msg` in
/// `bionic/libc/bionic/system_properties.cpp` sends exactly this many
/// bytes per `__system_property_set` call.
///
/// Task 6-X: the property service stub's accept thread (which drained
/// PROP_MSG_SIZE bytes + acked with PROP_SUCCESS=0) was REMOVED — init now
/// owns the property service socket. This constant is retained for the
/// `prop_msg_size_is_128_bytes` contract-locking test + future use if a
/// property-service-related feature is re-added.
#[allow(dead_code)]
const PROP_MSG_SIZE: usize = 128;

/// Create the `{rootfs}/dev/socket/` directory so init can create + bind
/// the property_service socket itself. Task 6-X: the previous implementation
/// (6-H) pre-bound the socket + spawned an accept thread — that was HARMFUL
/// (init couldn't unlink + rebind the socket → "init startup failure" →
/// exit(1)). See the body of this function for the full root-cause analysis.
fn spawn_property_service_thread(rootfs: &str) {
    let path = format!("{}/dev/socket/{}", rootfs, PROP_SERVICE_SOCKET_NAME);

    // Make sure {rootfs}/dev/socket exists (mode 0755). The guest's
    // init may already create this dir during coldboot (mknod hook),
    // but kr64's pre-creation here avoids the race + matches the
    // existing pattern in `devices::bind_unix_socket` / `ensure_parent_dir`.
    if let Some(parent) = Path::new(&path).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            warning!(
                "[KR64][property-svc] failed to mkdir {}: {} -- guest setprop may fail",
                parent.display(),
                e
            );
            return;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o755));
        }
    }

    // Task 6-X: DO NOT bind the socket. The previous implementation (6-H)
    // pre-bound /dev/socket/property_service + spawned an accept thread that
    // ACKed property-set requests with PROP_SUCCESS=0. This was added to
    // break init's pause() loop (waiting for the property service to start).
    // BUT 6-D (pause returns -ENOSYS) + 6-I (NOP the selinux-load-failure
    // jump that led to the pause loop) have ELIMINATED the pause loop.
    //
    // The stub is now HARMFUL: init (PID 1) is SUPPOSED to own the property
    // service — it creates + binds /dev/socket/property_service itself during
    // start_property_service(). But kr64's pre-bound socket file blocks init:
    // init's `unlink("/dev/socket/property_service")` returns EACCES (kr64 owns
    // the file), init logs "Failed to unlink old socket 'property_service':
    // Permission denied", init's property service fails to start, init's
    // property-setting all fails ("Failed to set ro.build.X" × 162), init
    // logs "init: init startup failure" + exit_group(1).
    //
    // FIX: don't bind. Just create the directory (above) so init can create
    // the socket file there. init will bind + listen + accept itself. This
    // matches the KVM E2E path (where init owns the property service + TWRP
    // boots). Verified root cause: 5b4ef63 UI E2E run 32200030310, last
    // syscalls before exit: read→EAGAIN, wait4→ECHILD, poll→0, write
    // "init: init startup failure", exit_group(1). The "Failed to unlink
    // old socket" error is the trigger.
    info!(
        "[KR64][property-svc] directory {} created — NOT binding socket (Task 6-X: let init own the property service; 6-H's pre-bind caused 'Failed to unlink old socket: Permission denied' → init startup failure → exit(1))",
        path
    );
}

// ============================================================================
// Touch device dispatcher.
//
// `spawn_accept_thread` above is the generic "accept-and-close" loop used
// for the device sockets we don't yet implement (key, event, gb, gb2). The
// touch device is different: the guest's EventHub opens `/dev/input/touch`
// and expects:
//   1. One `DeviceInfo` struct (896 bytes) advertising the device's
//      multi-touch Type-B capabilities (BTN_TOUCH, BTN_TOOL_FINGER,
//      ABS_MT_SLOT/TRACKING_ID/POSITION_X/Y/PRESSURE).
//   2. A continuous stream of `InputEvent` records, grouped into frames
//      by `SYN_REPORT`.
//
// The DeviceInfo header + InputEvent encoders already exist in
// `devices.rs` (added by 2-B at commit `370b8ee`). This module wires
// them into the socket accept loop: it sends the `DeviceInfo` on
// `accept()`, then enters a read loop where `TouchMessage` records
// (raw action + pointer_id + x + y + pressure) from the host-side
// IPC socket are encoded via `devices::encode_touch_*` and forwarded
// to the guest fd.
//
// ── IPC contract (host → kr64) ──────────────────────────────────────
//
// The host's `app/rs/src/input.rs` is expected to:
//   1. Bind a `UnixListener` at `{data_dir}/dev/touch-events`.
//   2. For each `MotionEvent` received via JNI
//      (`Renderer.handleTouch`), write a 20-byte little-endian
//      `TouchMessage` record to the accepted connection.
//
// ── TODO (out-of-scope for task 3-A) ──────────────────────────────
//
// As of commit `370b8ee` (2-B), `app/rs/src/input.rs::touch_server`
// binds `{data_dir}/rootfs/dev/input/touch` — the SAME path kr64 binds
// — and writes ENCODED `InputEvent` records directly. For this kr64-
// side dispatcher to receive raw `MotionEvent` data, `input.rs` must
// be refactored to:
//   * STOP binding the guest-facing `/dev/input/touch` socket (kr64
//     owns it now).
//   * Bind `{data_dir}/dev/touch-events` instead.
//   * Send raw `TouchMessage` records (action + pointer_id + x + y +
//     pressure) instead of pre-encoded `InputEvent`s.
//
// That change is OUT OF SCOPE for task 3-A (only `lib.rs` may be
// modified). Until it lands, kr64's touch dispatcher will accept the
// guest's connection, send the correct `DeviceInfo` header (so the
// guest's EventHub probes the device's capabilities correctly), then
// block forever on the empty `{data_dir}/dev/touch-events` socket —
// the guest will see a correctly-advertised multi-touch device but
// receive no events.
// ============================================================================

/// Size of one `TouchMessage` record on the host→kr64 IPC socket
/// (`{data_dir}/dev/touch-events`), in bytes. All fields are 4-byte
/// little-endian, no padding:
/// ```text
///   offset  size  field
///   ------  ----  -----
///     0      4    action      (u32: 0=DOWN, 1=MOVE, 2=UP, 3=CANCEL)
///     4      4    pointer_id  (i32: slot index 0..MAX_POINTERS-1)
///     8      4    x           (i32: pixel x)
///    12      4    y           (i32: pixel y)
///    16      4    pressure    (i32: 0..255)
/// ```
const TOUCH_MESSAGE_SIZE: usize = 20;

/// `TouchMessage::action` values. These match the subset of Android's
/// `MotionAction` that the touch dispatcher cares about (see
/// `app/rs/src/input.rs::handle_touch` for the same set).
mod touch_action {
    /// A new finger touched the screen (MotionEvent.ACTION_DOWN /
    /// ACTION_POINTER_DOWN).
    pub const DOWN: u32 = 0;
    /// An existing finger moved (MotionEvent.ACTION_MOVE).
    pub const MOVE: u32 = 1;
    /// The last finger lifted (MotionEvent.ACTION_UP).
    pub const UP: u32 = 2;
    /// A non-last finger lifted or the gesture was cancelled
    /// (MotionEvent.ACTION_POINTER_UP / ACTION_CANCEL).
    pub const CANCEL: u32 = 3;
}

/// A parsed touch message from the host's input.rs dispatcher. See
/// `TOUCH_MESSAGE_SIZE` for the on-wire layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TouchMessage {
    action: u32,
    pointer_id: i32,
    x: i32,
    y: i32,
    pressure: i32,
}

impl TouchMessage {
    /// Parse a 20-byte little-endian record into a `TouchMessage`.
    /// Returns `None` if the buffer is the wrong size.
    fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < TOUCH_MESSAGE_SIZE {
            return None;
        }
        let action = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let pointer_id = i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let x = i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let y = i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
        let pressure = i32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
        Some(TouchMessage {
            action,
            pointer_id,
            x,
            y,
            pressure,
        })
    }

    /// Serialise a `TouchMessage` into its 20-byte little-endian
    /// on-wire form. Used by tests to construct expected byte buffers.
    #[cfg(test)]
    fn to_bytes(self) -> [u8; TOUCH_MESSAGE_SIZE] {
        let mut buf = [0u8; TOUCH_MESSAGE_SIZE];
        buf[0..4].copy_from_slice(&self.action.to_le_bytes());
        buf[4..8].copy_from_slice(&self.pointer_id.to_le_bytes());
        buf[8..12].copy_from_slice(&self.x.to_le_bytes());
        buf[12..16].copy_from_slice(&self.y.to_le_bytes());
        buf[16..20].copy_from_slice(&self.pressure.to_le_bytes());
        buf
    }
}

/// Encode a parsed `TouchMessage` into the `InputEvent` byte stream the
/// guest's `EventHub` expects, by dispatching to `devices::encode_touch_*`.
///
/// `next_tracking_id` is a caller-maintained monotonic counter — each
/// fresh DOWN gets a new tracking ID assigned, and the assigned ID is
/// stored in `tracking_ids[slot]` so subsequent MOVE/UP events for that
/// slot know the same ID (the kernel's Type-B protocol requires the
/// tracking ID to be stable across a touch lifecycle). The kernel
/// releases the slot when it sees `ABS_MT_TRACKING_ID = -1`.
///
/// Returns the encoded bytes (empty `Vec<u8>` if the message was
/// ignored — e.g. out-of-range pointer_id or unknown action).
fn encode_touch_message(
    msg: &TouchMessage,
    time: libc::timeval,
    next_tracking_id: &mut i32,
    tracking_ids: &mut [i32; devices::MAX_POINTERS],
) -> Vec<u8> {
    let slot = msg.pointer_id;
    if slot < 0 || (slot as usize) >= devices::MAX_POINTERS {
        warning!(
            "[KR64][touch] out-of-range pointer_id {} (max {}) — dropping",
            slot,
            devices::MAX_POINTERS - 1
        );
        return Vec::new();
    }
    let slot_idx = slot as usize;
    match msg.action {
        touch_action::DOWN => {
            // Assign a fresh, non-zero tracking ID. The kernel treats
            // 0 as "uninitialised" and -1 as "released", so we start
            // the counter at 1 and increment by 1 per DOWN.
            let tid = *next_tracking_id;
            *next_tracking_id = next_tracking_id.wrapping_add(1);
            // Guard against the (extremely unlikely) wrap-around to 0
            // or -1 — skip the message rather than emit a malformed
            // tracking ID.
            if tid == 0 || tid == -1 {
                warning!(
                    "[KR64][touch] tracking-id counter wrapped to {} — skipping DOWN",
                    tid
                );
                return Vec::new();
            }
            tracking_ids[slot_idx] = tid;
            devices::encode_touch_down(time, slot, tid, msg.x, msg.y, msg.pressure)
        }
        touch_action::MOVE => {
            if tracking_ids[slot_idx] == 0 {
                // MOVE without a preceding DOWN — skip (the guest
                // would treat it as a stale slot state).
                return Vec::new();
            }
            devices::encode_touch_move(time, slot, msg.x, msg.y, msg.pressure)
        }
        touch_action::UP | touch_action::CANCEL => {
            if tracking_ids[slot_idx] == 0 {
                // UP/CANCEL without DOWN — nothing to release.
                return Vec::new();
            }
            tracking_ids[slot_idx] = 0;
            devices::encode_touch_release(time, slot)
        }
        other => {
            warning!("[KR64][touch] unknown action {} — dropping", other);
            Vec::new()
        }
    }
}

/// Get the current time as a `libc::timeval` (CLOCK_REALTIME). Used to
/// stamp each `InputEvent` so the guest's `EventHub` can group events
/// into frames via `SYN_REPORT`.
fn current_timeval() -> libc::timeval {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => libc::timeval {
            tv_sec: d.as_secs() as libc::time_t,
            tv_usec: d.subsec_micros() as libc::suseconds_t,
        },
        Err(_) => libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
    }
}

/// Spawn the touch device accept thread.
///
/// On accept (guest's EventHub opens `/dev/input/touch`):
///   1. Send the `DeviceInfo` header (896 bytes) built via
///      `devices::make_touch_device(cfg.width, cfg.height, &socket_path)`.
///      This advertises the multi-touch Type-B protocol capabilities
///      (BTN_TOUCH/BTN_TOOL_FINGER/ABS_MT_*).
///   2. Spawn a per-connection worker thread that reads `TouchMessage`
///      records (20-byte LE: action + pointer_id + x + y + pressure)
///      from the host-side IPC socket at `{cfg.data_dir}/dev/touch-events`
///      and writes encoded `InputEvent` bytes to the guest.
///
/// The thread takes ownership of the underlying `UnixListener` (so
/// `DeviceSocket` is consumed). The thread runs forever — it is
/// detached and cleaned up implicitly when the daemon exits.
fn spawn_touch_accept_thread(mut dev: devices::DeviceSocket, cfg: Config) {
    let listener = match dev.take_listener() {
        Some(l) => l,
        None => {
            warning!("[KR64][touch] cannot spawn accept thread: listener already taken");
            return;
        }
    };
    // 6-Z268: BLOCKING accept (see spawn_accept_thread) — the 50 ms
    // O_NONBLOCK poll only added connect latency + wakeups.
    let width = cfg.width;
    let height = cfg.height;
    let socket_path = dev.path.clone();
    let touch_events_path = format!("{}/dev/touch-events", cfg.data_dir);

    std::thread::Builder::new()
        .name("kr64-accept-touch".to_string())
        .spawn(move || {
            let fd = listener.as_raw_fd();
            info!(
                "[KR64][touch] accept thread started (fd={}, device_info_path={}, host_events_path={})",
                fd, socket_path, touch_events_path
            );
            // Build the DeviceInfo ONCE — it doesn't change per
            // connection. `devices::make_touch_device` advertises the
            // full Type-B multi-touch capabilities (BTN_TOUCH,
            // BTN_TOOL_FINGER, ABS_MT_SLOT/TRACKING_ID/POSITION_X/Y/
            // PRESSURE) so the guest's EventHub can probe the device.
            let device_info = devices::make_touch_device(width, height, &socket_path);
            // SAFETY: we're reading `DeviceInfo::size()` bytes from a
            // valid `&DeviceInfo` that we own. The slice does not
            // outlive `device_info` — we copy the bytes into a Vec
            // immediately.
            let device_info_bytes: Vec<u8> = unsafe {
                std::slice::from_raw_parts(
                    &device_info as *const devices::DeviceInfo as *const u8,
                    devices::DeviceInfo::size(),
                )
                .to_vec()
            };
            assert_eq!(
                device_info_bytes.len(),
                896,
                "DeviceInfo must be 896 bytes (got {})",
                device_info_bytes.len()
            );

            loop {
                match listener.accept() {
                    Ok((stream, _addr)) => {
                        info!("[KR64][touch] guest connected");
                        // Spawn a per-connection worker thread so the
                        // accept loop can keep accepting new
                        // connections (the guest may reconnect after
                        // a suspend/resume).
                        let dev_bytes = device_info_bytes.clone();
                        let ev_path = touch_events_path.clone();
                        std::thread::Builder::new()
                            .name("kr64-touch-conn".to_string())
                            .spawn(move || {
                                touch_connection_loop(stream, dev_bytes, ev_path);
                            })
                            .ok();
                    }
                    Err(e) => {
                        warning!("[KR64][touch] accept error: {}", e);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        })
        .expect("spawn kr64 touch accept thread");
}

/// Per-connection touch worker: send the `DeviceInfo` header, then read
/// `TouchMessage` records from the host's `{data_dir}/dev/touch-events`
/// socket and write encoded `InputEvent` bytes to the guest fd.
///
/// This function blocks for the lifetime of one guest connection. When
/// either side closes the socket, it returns (the worker thread exits
/// and the OS cleans up the fd).
fn touch_connection_loop(
    guest: std::os::unix::net::UnixStream,
    device_info_bytes: Vec<u8>,
    touch_events_path: String,
) {
    use std::io::{Read, Write};
    let mut guest = guest;

    // Step 1: send the DeviceInfo header so the guest's EventHub can
    // probe the device's capabilities (BTN_TOUCH, BTN_TOOL_FINGER,
    // ABS_MT_* axes). Without this header the guest sees a 1-byte
    // short-read (the old `spawn_accept_thread` behaviour) and drops
    // the device from its input device list.
    if let Err(e) = guest.write_all(&device_info_bytes) {
        warning!(
            "[KR64][touch] failed to send DeviceInfo header ({} bytes): {}",
            device_info_bytes.len(),
            e
        );
        return;
    }
    info!(
        "[KR64][touch] sent DeviceInfo header ({} bytes) — device advertised",
        device_info_bytes.len()
    );

    // Step 2: connect to the host-side touch-events IPC socket. The
    // host's input.rs is expected to bind this socket and write
    // TouchMessage records to it. 6-Z268: exponential backoff 20 ms →
    // 200 ms (cap 150 attempts) — the first attempts now fire within
    // milliseconds of the guest connect instead of always paying a full
    // 200 ms tail; the 30 s give-up budget is unchanged.
    let mut host_stream: Option<std::os::unix::net::UnixStream> = None;
    for attempt in 0..150u32 {
        match std::os::unix::net::UnixStream::connect(&touch_events_path) {
            Ok(s) => {
                host_stream = Some(s);
                info!(
                    "[KR64][touch] connected to host touch-events socket at {} (attempt {})",
                    touch_events_path, attempt
                );
                break;
            }
            Err(_) => {
                // 6-Z268: exponential backoff — 20 ms for the first ten
                // attempts (the common ordering race), then the historic
                // 200 ms.
                let backoff_ms = if attempt < 10 { 20 } else { 200 };
                std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
            }
        }
    }
    let mut host = match host_stream {
        Some(s) => s,
        None => {
            warning!(
                "[KR64][touch] host touch-events socket at {} never appeared after 30s — \
                 the guest will see the device but receive no events. \
                 TODO: update app/rs/src/input.rs to bind this socket and send TouchMessage records",
                touch_events_path
            );
            return;
        }
    };

    // Step 3: per-connection state for the Type-B multi-touch
    // protocol. `next_tracking_id` is a monotonically-increasing
    // counter — each fresh DOWN gets a new ID. `tracking_ids[slot]`
    // caches the active ID per slot (0 = unused) so MOVE/UP know the
    // same ID.
    let mut next_tracking_id: i32 = 1;
    let mut tracking_ids = [0i32; devices::MAX_POINTERS];

    let mut buf = [0u8; TOUCH_MESSAGE_SIZE];
    loop {
        if let Err(e) = host.read_exact(&mut buf) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                info!("[KR64][touch] host touch-events socket closed");
            } else {
                warning!("[KR64][touch] read from host socket failed: {}", e);
            }
            return;
        }
        let msg = match TouchMessage::parse(&buf) {
            Some(m) => m,
            None => continue,
        };
        let time = current_timeval();
        let encoded = encode_touch_message(&msg, time, &mut next_tracking_id, &mut tracking_ids);
        if encoded.is_empty() {
            continue;
        }
        if let Err(e) = guest.write_all(&encoded) {
            warning!("[KR64][touch] write to guest failed: {}", e);
            return;
        }
    }
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
// 6-Z219: guest-derived androidboot.slot_suffix (A/B recovery images).
// ============================================================================
// Android 12+ libfstab (fs_mgr/libfstab/slotselect.cpp) makes a fstab
// containing `slotselect` flags FATAL to parse when the slot suffix
// resolves empty:
//
//     fs_mgr_update_for_slotselect(): if any entry has slot_select and
//     fs_mgr_get_slot_suffix() == ""  →  return false
//         → "Error updating for slotselect"
//         → ReadFstabFromFileCommon fails → empty fstab
//         → recovery binary: glog CHECK(!fstab.empty()) → abort
//
// Evidence chain (run 33271279412, lineage-22.2-sailfish, 2026-08-29):
//   "<3>init: [libfstab] Error updating for slotselect"
//   "<3>init: [libfstab] ReadFstabFromFileCommon(): failed to load fstab
//        from : '/etc/recovery.fstab'"
//   "[glog F/abort] Check failed: !fstab.empty()"   (strings(1) of the
//        guest system/bin/recovery confirms the CHECK lives there)
//
// Twoyi hard-coded `androidboot.slot_suffix=` (EMPTY) in the synthetic
// /proc/cmdline. On a real A/B device the bootloader passes
// androidboot.slot_suffix=_a; the empty value poisons every lookup:
// fs_mgr_get_boot_config("slot_suffix") finds the key in /proc/cmdline and
// returns true with an EMPTY value, and DT/bootconfig fallbacks never run.
//
// GENERIC FIX (§22 — keyed on image content, not device identity): the
// recovery image itself tells us whether its device is A/B. If any fstab
// file shipped in the guest rootfs uses the `slotselect` flag, the guest
// expects an A/B device, so we provide androidboot.slot_suffix=_a (the
// default boot slot on every A/B device). When no fstab uses slotselect
// we provide NO slot_suffix key at all — identical semantics for A-only
// images (angler, whyred, lavender, x86 ranchu) and no poisoning.
//
// The empty-string value is also deliberately NOT emitted: an empty
// `androidboot.slot_suffix=` is worse than an absent key because modern
// fs_mgr_get_boot_config() treats "found but empty" as authoritative.
pub fn detect_guest_slot_suffix(rootfs_prefix: &str) -> String {
    // fstab files can live in any of these locations across Android
    // generations and ramdisk layouts (legacy top-level, recovery
    // ramdisks, first_stage_ramdisk layouts, system-as-root):
    const SCAN_DIRS: &[&str] = &[
        "",
        "etc",
        "system/etc",
        "system/system_ext/etc",
        "vendor/etc",
        "odm/etc",
        "first_stage_ramdisk",
        "first_stage_ramdisk/etc",
    ];
    for dir in SCAN_DIRS {
        let full = if dir.is_empty() {
            rootfs_prefix.to_string()
        } else {
            format!("{}/{}", rootfs_prefix, dir)
        };
        let rd = match std::fs::read_dir(&full) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for ent in rd.flatten() {
            let name = ent.file_name().to_string_lossy().to_string();
            if !(name == "recovery.fstab" || name.starts_with("fstab")) {
                continue;
            }
            let path = format!("{}/{}", full, name);
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let has_slotselect = content.lines().any(|line| {
                let t = line.trim();
                if t.is_empty() || t.starts_with('#') {
                    return false;
                }
                // fs_mgr_flags are the last whitespace-separated field of
                // an fstab line; be lenient about which field carries the
                // flag but exact about the token itself so a hypothetical
                // block device named *slotselect* cannot trip this.
                t.split_whitespace()
                    .flat_map(|f| f.split(','))
                    .any(|flag| flag.trim() == "slotselect" || flag.trim() == "slotselect_other")
            });
            if has_slotselect {
                return "_a".to_string();
            }
        }
    }
    String::new()
}

/// 6-Z270: strip FBE/FDE fs_mgr flags from the guest's /data fstab entries.
///
/// WHY (run 33409396151, OrangeFox R12.0 lavender — boot-to-UI 47 s): the
/// device fstab's /data line carries
/// `fileencryption=aes-256-xts:aes-256-cts:v2+inlinecrypt_optimized,
/// metadata_encryption=…,keydirectory=/metadata/vold/metadata_encryption`.
/// TWRP's PartitionManager sees the FBE flags and runs its full decryption
/// probe: it forks /system/bin/keystore_cli_v2 (tracer +10.6 s) which
/// blocks on the keystore2 binder service — a service our container can
/// never satisfy (keystore2 has no working keymaster HAL behind it) —
/// until the cli's ~20 s service-wait budget expires. Measured cost on
/// the 6-Z269 run: +12.7 s → +31.3 s of wall clock with only 4,968
/// tracer stops in the window (316 stops/s — pure client-side sleep
/// polling, zero useful work). The /data mount itself fails either way
/// in the container (no block device behind it), so the decryption probe
/// is unconditionally wasted time on EVERY recovery boot.
///
/// FIX: parent-side, pre-fork, rewrite the guest's own fstab files with
/// the crypto tokens removed from /data entries. TWRP then classifies
/// /data as not-FBE and skips the keystore chain entirely (its data/media
/// fallback path handles the unmountable /data exactly as before — the
/// 6-Z269 log already shows the fallback running after the decrypt
/// failure). Everything except /data lines is preserved byte-for-byte;
/// untouched lines are preserved byte-for-byte (only rewritten lines are
/// re-joined, single-space separated — fstab parsing is
/// whitespace-agnostic).
pub fn sanitize_fstab_encryption_flags(rootfs_prefix: &str) {
    // Same location census as detect_guest_slot_suffix — fstab files
    // live in any of these across Android generations/ramdisk layouts.
    const SCAN_DIRS: &[&str] = &[
        "",
        "etc",
        "system/etc",
        "system/system_ext/etc",
        "vendor/etc",
        "odm/etc",
        "first_stage_ramdisk",
        "first_stage_ramdisk/etc",
    ];
    for dir in SCAN_DIRS {
        let full = if dir.is_empty() {
            rootfs_prefix.to_string()
        } else {
            format!("{}/{}", rootfs_prefix, dir)
        };
        let rd = match std::fs::read_dir(&full) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for ent in rd.flatten() {
            let name = ent.file_name().to_string_lossy().to_string();
            if !(name == "recovery.fstab" || name.starts_with("fstab")) {
                continue;
            }
            let path = format!("{}/{}", full, name);
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let mut changed = false;
            let mut out: Vec<String> = Vec::new();
            for line in content.split_inclusive('\n') {
                let t = line.trim();
                if t.is_empty() || t.starts_with('#') {
                    out.push(line.trim_end_matches('\n').to_string());
                    continue;
                }
                let fields: Vec<&str> = t.split_whitespace().collect();
                // fstab row: src mount_point type mnt_flags fs_mgr_flags —
                // only touch rows whose MOUNT POINT (2nd field) is /data.
                if fields.len() < 3 || fields[1] != "/data" {
                    out.push(line.trim_end_matches('\n').to_string());
                    continue;
                }
                let mut line_changed = false;
                let rebuilt: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        let kept: Vec<&str> = f
                            .split(',')
                            .filter(|tok| {
                                let drop = tok.starts_with("fileencryption=")
                                    || tok.starts_with("metadata_encryption=")
                                    || tok.starts_with("keydirectory=")
                                    || tok.starts_with("forceencrypt=")
                                    || tok.starts_with("encryptable=")
                                    || *tok == "fileencryption"
                                    || *tok == "metadata_encryption"
                                    || *tok == "forceencrypt"
                                    || *tok == "encryptable";
                                if drop {
                                    line_changed = true;
                                }
                                !drop
                            })
                            .collect();
                        kept.join(",")
                    })
                    .collect();
                if !line_changed {
                    out.push(line.trim_end_matches('\n').to_string());
                    continue;
                }
                changed = true;
                let mut new_fields = rebuilt;
                // A flags field that consisted ONLY of crypto tokens would
                // collapse to "" — keep the field count intact (fstab
                // parsers want >= 5 fields) with a harmless standard flag.
                for f in new_fields.iter_mut() {
                    if f.is_empty() {
                        *f = "wait".to_string();
                    }
                }
                out.push(new_fields.join(" "));
            }
            if changed {
                let mut new_content = out.join("\n");
                if content.ends_with('\n') && !new_content.ends_with('\n') {
                    new_content.push('\n');
                }
                match std::fs::write(&path, &new_content) {
                    Ok(_) => info!(
                        "[KR64] PARENT: 6-Z270: stripped FBE/FDE flags from /data entries in {} ({} bytes -> {} bytes)",
                        path,
                        content.len(),
                        new_content.len()
                    ),
                    Err(e) => warning!(
                        "[KR64] PARENT: 6-Z270: FAILED to rewrite {}: {}",
                        path, e
                    ),
                }
            }
        }
    }
}

/// 6-Z271j: declare the in-proxy VIRTUAL AIDL HALs in the guest's VINTF
/// device manifest.
///
/// WHY (run 33428365193 + AOSP android-13 keystore2 sources): keystore2's
/// `connect_keymint()` does NOT ask the servicemanager for AIDL keymint —
/// it enumerates instances with libvintf's `getAidlInstances(package,
/// version, interface)` (keystore2/src/vintf/vintf.cpp →
/// `VintfObject::GetDeviceHalManifest()`), and only falls back to the
/// HIDL `android.security.compat` chain when the manifest lists NO AIDL
/// keymint instance. A recovery ramdisk's manifest declares the legacy
/// HIDL `android.hardware.keymaster@4.0` and never the AIDL interfaces,
/// so the genuine lookup path found nothing, the compat fallback could
/// not wrap a non-existent HIDL HAL, and keystore2 PANICKED before ever
/// registering IKeystoreSecurity — the recovery-side client then burned
/// its full service-wait budget (the ~20 s hole class).
///
/// FIX: inject `<hal>` entries for the two VIRTUAL services the proxy
/// genuinely hosts (IKeyMintDevice/default, ISharedSecret/default) into
/// the guest's vendor manifest, BEFORE any guest process runs (libvintf
/// caches the device manifest per process). This is semantically correct
/// virtualization — the manifest then describes what this virtual device
/// actually provides — not a fake success path: the declared services
/// really answer on the bus.
///
/// Injection rules: first existing manifest in the scan order wins
/// (vendor before system — the merged device manifest prefers vendor);
/// idempotent (skips files already declaring the hal name); only rewrites
/// when a `</manifest>` close tag exists (never corrupt unknown XML);
/// creates a minimal vendor manifest when none of the scanned files
/// exists.
pub fn augment_vintf_manifest_for_virtual_hals(rootfs_prefix: &str) {
    // 6-Z271m FIX (run 33481635353): manifest <version> for format="aidl"
    // is parsed by libvintf's AidlVersionConverter → parseAidlVersion — a
    // SINGLE integer. Version RANGES ("1-3") are AidlVersionRangeConverter
    // / matrix-only syntax: a range in a MANIFEST fails the whole
    // <manifest> parse, VintfObject::GetDeviceHalManifest() returns
    // nullptr, and keystore2_vintf's vintf.cpp dereferences it unchecked
    // → SIGSEGV(0x0) in keystore2's negotiation thread ~170 ms after
    // exec, crash-looping via init's 5 s restart budget. The single
    // version 3 is correct on both ends: HalManifest::forEachInstanceOf-
    // Version matches with minorAtLeast, so a declared 3 answers
    // keystore2's connect_keymint count-down (V2 query first, then V1)
    // with hal_version=2, and 3 is the KeyMint version the virtual
    // device's getHardwareInfo actually reports.
    const KEYMINT_BLOCK: &str = concat!(
        "    <hal format=\"aidl\">\n",
        "        <name>android.hardware.security.keymint</name>\n",
        "        <version>3</version>\n",
        "        <interface>\n",
        "            <name>IKeyMintDevice</name>\n",
        "            <instance>default</instance>\n",
        "        </interface>\n",
        "    </hal>\n",
    );
    const SHAREDSECRET_BLOCK: &str = concat!(
        "    <hal format=\"aidl\">\n",
        "        <name>android.hardware.security.sharedsecret</name>\n",
        "        <version>1</version>\n",
        "        <interface>\n",
        "            <name>ISharedSecret</name>\n",
        "            <instance>default</instance>\n",
        "        </interface>\n",
        "    </hal>\n",
    );

    // Minimal-but-valid skeleton when the ramdisk ships no manifest at
    // all (type="device" = vendor side; target-level omitted — optional).
    const SKELETON_HEAD: &str = "<manifest version=\"6.0\" type=\"device\">\n";
    const SKELETON_TAIL: &str = "</manifest>\n";

    const SCAN_FILES: &[&str] = &[
        "vendor/etc/vintf/manifest.xml",
        "vendor/manifest.xml",
        "system/etc/vintf/manifest.xml",
    ];
    let inject = |path: &str| -> Option<()> {
        let content = std::fs::read_to_string(path).ok()?;
        let mut blocks = String::new();
        if !content.contains("android.hardware.security.keymint") {
            blocks.push_str(KEYMINT_BLOCK);
        }
        if !content.contains("android.hardware.security.sharedsecret") {
            blocks.push_str(SHAREDSECRET_BLOCK);
        }
        if blocks.is_empty() {
            // Already declaring everything — nothing to do.
            return Some(());
        }
        let close = content.rfind("</manifest>")?;
        let mut new_content = String::with_capacity(content.len() + blocks.len());
        new_content.push_str(&content[..close]);
        new_content.push_str(&blocks);
        new_content.push_str(&content[close..]);
        match std::fs::write(path, &new_content) {
            Ok(_) => {
                info!(
                    "[KR64] PARENT: 6-Z271j: injected virtual AIDL HALs into VINTF manifest {} ({} -> {} bytes)",
                    path,
                    content.len(),
                    new_content.len()
                );
                Some(())
            }
            Err(e) => {
                warning!("[KR64] PARENT: 6-Z271j: FAILED to write {}: {}", path, e);
                None
            }
        }
    };

    for rel in SCAN_FILES {
        let path = format!("{}/{}", rootfs_prefix, rel);
        if !std::path::Path::new(&path).exists() {
            continue;
        }
        if inject(&path).is_some() {
            return;
        }
    }
    // No manifest anywhere: create the canonical vendor one.
    let path = format!("{}/vendor/etc/vintf/manifest.xml", rootfs_prefix);
    if let Some(dir) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let fresh = format!(
        "{}{}{}{}",
        SKELETON_HEAD, KEYMINT_BLOCK, SHAREDSECRET_BLOCK, SKELETON_TAIL
    );
    match std::fs::write(&path, &fresh) {
        Ok(_) => info!(
            "[KR64] PARENT: 6-Z271j: created vendor VINTF manifest with virtual AIDL HALs at {} ({} bytes)",
            path,
            fresh.len()
        ),
        Err(e) => warning!(
            "[KR64] PARENT: 6-Z271j: FAILED to create {}: {}",
            path, e
        ),
    }
}

/// 6-Z223: build the ordered androidboot.* item list for the guest
/// /proc/cmdline. `hw` is the detected androidboot.hardware (6-Z159);
/// `slot_suffix` is 6-Z219's guest-derived value ("_a") or "" for A-only
/// images (the key is omitted entirely — an empty value would poison
/// fs_mgr_get_boot_config's DT/bootconfig fallbacks). Ranchu-only
/// emulator extras (gralloc/vulkan/qemu) are emitted only for hw ==
/// "ranchu"; they are meaningless for real-device recovery images.
///
/// ORDERING INVARIANT: slot_suffix, when present, is the FINAL item —
/// join_hybrid_cmdline() leaves the final item's value NUL-free, and
/// libfstab's A/B consumers compare the slot suffix with std::string
/// equality (slotselect.cpp other_suffix(): `slot_suffix == "_a"`) and
/// splice it into entry.logical_partition_name, which is matched against
/// super-partition metadata by std::string equality for dynamic-
/// partition devices. A trailing NUL on that value would fail both.
pub fn build_cmdline_items(hw: &str, slot_suffix: &str) -> Vec<String> {
    let mut items = vec![format!("androidboot.hardware={}", hw)];
    if hw == "ranchu" {
        items.push("androidboot.hardware.gralloc=ranchu".to_string());
        items.push("androidboot.hardware.vulkan=ranchu".to_string());
    }
    items.push("androidboot.serialno=twoyi".to_string());
    items.push("androidboot.boot_devices=pci0000:00/0000:00:03.0".to_string());
    items.push("androidboot.verifiedbootstate=orange".to_string());
    items.push("androidboot.flash.locked=0".to_string());
    items.push("androidboot.vbmeta.size=0".to_string());
    if hw == "ranchu" {
        items.push("qemu=1".to_string());
        items.push("qemu.avd_name=twoyi_test".to_string());
    }
    if !slot_suffix.is_empty() {
        items.push(format!("androidboot.slot_suffix={}", slot_suffix));
    }
    items
}

/// 6-Z223: HYBRID cmdline join — "item\0 item\0 … item" (space BETWEEN
/// items, NUL after every item EXCEPT the last). TWO consumer families
/// must both parse every key:
///   * OLD init (TWRP 2.8's static Android-5.x init): import_kernel_cmdline
///     reads the file, self-terminates the buffer (cmdline[n] = 0), then
///     walks SPACE-delimited pieces with C-string semantics — strchr stops
///     at the FIRST item's NUL, so item 1 (androidboot.hardware, the boot-
///     critical key for TWRP's init.<hw>.rc lookup) is imported exactly as
///     under the legacy pure-NUL format and the walk never reaches items
///     2+. Even a hypothetical NUL-iterating reader sees items 2+ only as
///     leading-space keys (invisible; cosmetic loss identical to the
///     legacy format hiding them from every space-splitting parser).
///   * MODERN init + libfstab (Android 12+; property_service.cpp's
///     ProcessKernelCmdline and fstab's fs_mgr_get_boot_config use
///     android::fs_mgr::ImportKernelCmdlineFromString, which splits on
///     SPACE and '"', never NUL). With the legacy pure-NUL format the
///     whole cmdline was ONE space-piece: only androidboot.hardware (the
///     first '=' pair) was discoverable and EVERY later key — including
///     6-Z219's androidboot.slot_suffix — was invisible → Android 12+
///     libfstab aborted the A/B recovery's fstab ("Error updating for
///     slotselect", run 33275526098) even with slot_suffix present in
///     the file. Space-splitting now yields one piece per key.
///
/// VALUE-NUL RULE: non-final items keep their NUL (C-string terminator
/// for old parsers; every modern consumer of those values is C-string-
/// based: property storage, mount(2)/open(2) paths, strtoull). The FINAL
/// item is emitted WITHOUT a trailing NUL because build_cmdline_items()
/// places slot_suffix last and libfstab compares that value with
/// std::string equality (other_suffix) and splices it into
/// entry.logical_partition_name for super-partition matching.
pub fn join_hybrid_cmdline(items: &[String]) -> String {
    let mut out = String::new();
    for (n, item) in items.iter().enumerate() {
        if n > 0 {
            out.push(' ');
        }
        out.push_str(item);
        if n + 1 < items.len() {
            out.push('\0');
        }
    }
    out
}

// ============================================================================
// Tests -- exercise arg parsing and config defaults.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── 6-Z272c: enable_image_health_hal ─────────────────────────────────
    #[test]
    fn z272c_health_hal_enable_drops_disabled_and_appends_late_init_start() {
        let dir = std::env::temp_dir().join(format!("kr64_6z272c_hal_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let init_dir = dir.join("system/etc/init");
        std::fs::create_dir_all(&init_dir).unwrap();
        let bin_dir = dir.join("system/bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        // The image's own rc + binary.
        std::fs::write(bin_dir.join("android.hardware.health@2.1-service"), b"ELF").unwrap();
        let rc = init_dir.join("android.hardware.health@2.1-service.rc");
        std::fs::write(
            &rc,
            "service health-hal-2-1 /system/bin/android.hardware.health@2.1-service\n    disabled\n    user root\n    group root\n    file /dev/kmsg w\n    seclabel u:r:recovery:s0\n",
        )
        .unwrap();

        let n = enable_image_health_hal(dir.to_str().unwrap());
        assert_eq!(n, 1, "the 2.1 candidate should be patched");
        let out = std::fs::read_to_string(&rc).unwrap();
        assert!(
            !out.lines().any(|l| l.trim() == "disabled"),
            "the disabled option must be dropped: {out}"
        );
        assert!(
            out.contains("# 6-Z272c:") && out.contains("on late-init\n    start health-hal-2-1"),
            "the late-init start block must be appended: {out}"
        );
        // Service definition preserved byte-for-byte.
        assert!(
            out.contains("service health-hal-2-1 /system/bin/android.hardware.health@2.1-service")
        );
        assert!(out.contains("seclabel u:r:recovery:s0"));

        // Idempotent: a second pass is a no-op returning 1.
        let n2 = enable_image_health_hal(dir.to_str().unwrap());
        assert_eq!(n2, 1, "second pass must detect the marker");
        let out2 = std::fs::read_to_string(&rc).unwrap();
        assert_eq!(out, out2, "second pass must not duplicate the block");

        // Missing binary → no patch (we never start a synthetic service).
        let dir2 = std::env::temp_dir().join(format!("kr64_6z272c_hal2_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir2);
        let init2 = dir2.join("system/etc/init");
        std::fs::create_dir_all(&init2).unwrap();
        std::fs::write(
            init2.join("android.hardware.health@2.1-service.rc"),
            "service health-hal-2-1 /system/bin/android.hardware.health@2.1-service\n    disabled\n",
        )
        .unwrap();
        let n3 = enable_image_health_hal(dir2.to_str().unwrap());
        assert_eq!(n3, 0, "no binary → no patch");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir2);
    }

    // ── 6-Z270: sanitize_fstab_encryption_flags ─────────────────────────
    #[test]
    fn fstab_encryption_sanitize_strips_fbe_flags_from_data_only_6z270() {
        let dir = std::env::temp_dir().join(format!("kr64_6z270_test_{}", std::process::id()));
        let etc = dir.join("etc");
        std::fs::create_dir_all(&etc).unwrap();
        let path = etc.join("recovery.fstab");
        std::fs::write(
            &path,
            "# comment with fileencryption=keep-me\n\
             /dev/block/by-name/system\t/system\text4\tro,barrier=1\twait,logical,first_stage_mount\n\
             /dev/block/bootdevice/by-name/userdata\t/data\tf2fs\tnosuid,nodev\twait,check,formattable,fileencryption=aes-256-xts:aes-256-cts:v2+inlinecrypt_optimized,metadata_encryption=aes-256-xts,keydirectory=/metadata/vold/metadata_encryption,reservedsize=128M,checkpoint=fs\n\
             /dev/block/by-name/cache\t/cache\text4\trw\twait\n",
        )
        .unwrap();
        sanitize_fstab_encryption_flags(dir.to_str().unwrap());
        let out = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        // comment preserved (even though it mentions the token)
        assert!(
            lines[0].starts_with("# comment"),
            "comment line rewritten: {out}"
        );
        // non-/data rows byte-preserved
        assert!(lines[1].contains('\t'), "non-data row must keep raw bytes");
        assert!(lines[1].contains("first_stage_mount"));
        assert!(!lines[1].starts_with("/dev/block/bootdevice"));
        // /data row: crypto tokens stripped, other flags kept, field count kept
        let data = lines[2];
        assert!(!data.contains("fileencryption"), "{data}");
        assert!(!data.contains("metadata_encryption"), "{data}");
        assert!(!data.contains("keydirectory"), "{data}");
        assert!(data.contains("wait,check,formattable"), "{data}");
        assert!(data.contains("reservedsize=128M"), "{data}");
        assert!(data.contains("checkpoint=fs"), "{data}");
        assert!(data.contains("/data"), "{data}");
        assert!(data.split_whitespace().count() >= 5, "{data}");
        // /cache row untouched
        assert_eq!(lines[3], "/dev/block/by-name/cache\t/cache\text4\trw\twait");
        // idempotent second run: no further change
        sanitize_fstab_encryption_flags(dir.to_str().unwrap());
        let out2 = std::fs::read_to_string(&path).unwrap();
        assert_eq!(out, out2, "second sanitize must be a no-op");
        std::fs::remove_dir_all(&dir).ok();
    }

    // ── 6-Z271j: VINTF manifest augmentation for the virtual AIDL HALs ──

    #[test]
    fn vintf_manifest_augmentation_injects_virtual_aidl_hals_6z271j() {
        let dir = std::env::temp_dir().join(format!("kr64_6z271j_test_{}", std::process::id()));
        let vdir = dir.join("vendor/etc/vintf");
        std::fs::create_dir_all(&vdir).unwrap();
        let path = vdir.join("manifest.xml");
        std::fs::write(
            &path,
            "<manifest version=\"6.0\" type=\"device\">\n\
             \x20   <hal format=\"hidl\">\n\
             \x20       <name>android.hardware.keymaster</name>\n\
             \x20       <version>4.0</version>\n\
             \x20   </hal>\n\
             </manifest>\n",
        )
        .unwrap();
        augment_vintf_manifest_for_virtual_hals(dir.to_str().unwrap());
        let out = std::fs::read_to_string(&path).unwrap();
        // HIDL keymaster entry preserved byte-for-byte (TWRP's version
        // detection depends on it — do not regress other readers).
        assert!(out.contains("android.hardware.keymaster"), "{out}");
        assert!(out.contains("4.0"), "{out}");
        // Both virtual AIDL HALs declared before </manifest>.
        assert!(out.contains("android.hardware.security.keymint"), "{out}");
        // 6-Z271m: SINGLE version only — ranges ("1-3") are matrix-only
        // libvintf syntax; in a manifest they fail the whole parse and
        // keystore2's VintfObject returns nullptr → SIGSEGV (run
        // 33481635353). HalManifest matching is minorAtLeast, so a single
        // 3 answers keystore2's V2-then-V1 count-down.
        assert!(out.contains("<version>3</version>"), "{out}");
        let km_start = out.find("android.hardware.security.keymint").unwrap_or(0);
        let km_end = out[km_start..]
            .find("</hal>")
            .map(|i| km_start + i + 6)
            .unwrap_or(out.len());
        let km_block = out[km_start..km_end].to_string();
        assert!(
            !km_block.contains('-'),
            "injected keymint block must not carry range syntax: {km_block}"
        );
        assert!(out.contains("IKeyMintDevice"), "{out}");
        assert!(out.contains("<instance>default</instance>"), "{out}");
        assert!(
            out.contains("android.hardware.security.sharedsecret"),
            "{out}"
        );
        assert!(out.contains("ISharedSecret"), "{out}");
        // Well-formed structure: injected blocks land inside the root.
        let close = out.rfind("</manifest>").unwrap();
        assert!(
            out[..close].contains("IKeyMintDevice"),
            "hal blocks must precede the close tag"
        );
        // Idempotent second run: no double injection.
        augment_vintf_manifest_for_virtual_hals(dir.to_str().unwrap());
        let out2 = std::fs::read_to_string(&path).unwrap();
        assert_eq!(out, out2, "second augmentation must be a no-op");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn vintf_manifest_augmentation_creates_skeleton_when_missing_6z271j() {
        let dir = std::env::temp_dir().join(format!("kr64_6z271j_b_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        augment_vintf_manifest_for_virtual_hals(dir.to_str().unwrap());
        let path = dir.join("vendor/etc/vintf/manifest.xml");
        let out = std::fs::read_to_string(&path).unwrap();
        assert!(out.starts_with("<manifest"), "{out}");
        assert!(out.contains("type=\"device\""), "{out}");
        assert!(out.contains("IKeyMintDevice"), "{out}");
        assert!(out.contains("ISharedSecret"), "{out}");
        assert!(out.trim_end().ends_with("</manifest>"), "{out}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn vintf_manifest_augmentation_skips_already_declared_6z271j() {
        let dir = std::env::temp_dir().join(format!("kr64_6z271j_c_test_{}", std::process::id()));
        let vdir = dir.join("vendor/etc/vintf");
        std::fs::create_dir_all(&vdir).unwrap();
        let path = vdir.join("manifest.xml");
        let original = "<manifest version=\"6.0\" type=\"device\">\n\
             \x20   <hal format=\"aidl\">\n\
             \x20       <name>android.hardware.security.keymint</name>\n\
             \x20   </hal>\n\
             </manifest>\n";
        std::fs::write(&path, original).unwrap();
        augment_vintf_manifest_for_virtual_hals(dir.to_str().unwrap());
        let out = std::fs::read_to_string(&path).unwrap();
        // keymint name already present → not duplicated…
        assert_eq!(
            out.matches("android.hardware.security.keymint").count(),
            1,
            "{out}"
        );
        // …but the missing sharedsecret entry IS added.
        assert!(
            out.contains("android.hardware.security.sharedsecret"),
            "{out}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    fn args(v: &[&str]) -> Vec<String> {
        std::iter::once("kr64".to_string())
            .chain(v.iter().map(|s| s.to_string()))
            .collect()
    }

    // ── 6-Z256: recovery-child envp builder ──────────────────────────
    //
    // Regression anchors: the 20-build aarch64 OrangeFox
    // strlen(getenv("ANDROID_ROOT")) null-call class (run 33323583991).
    // The old hardcoded 4-entry envp (LD_PRELOAD, LD_LIBRARY_PATH, PATH,
    // TWOYI_ROOTFS) had `std-vars 0/4 present` per the 6-Z238/6-Z255
    // scan — every standard-var getenv() returned NULL.

    fn env_names(entries: &[String]) -> Vec<&str> {
        entries
            .iter()
            .map(|e| e.split('=').next().unwrap_or(""))
            .collect()
    }

    fn env_get<'a>(entries: &'a [String], name: &str) -> Option<&'a str> {
        let prefix = format!("{}=", name);
        entries
            .iter()
            .find(|e| e.starts_with(&prefix))
            .map(|e| &e[prefix.len()..])
    }

    #[test]
    fn z256_envp_includes_standard_android_vars() {
        let envp = build_recovery_service_envp("/host/rootfs", false, &[]);
        let names = env_names(&envp);
        for std_name in [
            "ANDROID_ROOT",
            "ANDROID_DATA",
            "EXTERNAL_STORAGE",
            "ANDROID_BOOTLOGO",
        ] {
            assert!(
                names.contains(&std_name),
                "envp must carry {} (the 20-build OrangeFox strlen(NULL) class)",
                std_name
            );
        }
        assert_eq!(env_get(&envp, "ANDROID_ROOT"), Some("/system"));
        // twoyi's virtualization stack entries must survive unchanged.
        assert_eq!(
            env_get(&envp, "LD_PRELOAD"),
            Some("/host/rootfs/sbin/libtwrp_fb_hook.so")
        );
        assert_eq!(env_get(&envp, "TWOYI_ROOTFS"), Some("/host/rootfs"));
        // No duplicate keys (execve(3) envp duplicate names are UB).
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            names.len(),
            "duplicate env names: {:?}",
            names
        );
    }

    #[test]
    fn z256_envp_rc_exports_override_defaults_but_not_twoyi_keys() {
        let rc_exports: Vec<(String, String)> = vec![
            // Guest-owned truth: the guest's own export must WIN over the
            // twoyi default for a non-twoyi key...
            ("ANDROID_ROOT".to_string(), "/system2".to_string()),
            // ...but the virtualization stack keys are twoyi-owned: the
            // guest rc export must NOT clobber them (§22).
            ("LD_PRELOAD".to_string(), "/evil.so".to_string()),
            ("TWOYI_ROOTFS".to_string(), "/evil".to_string()),
            // Guest-specific extras are carried through (e.g. mksh's ENV).
            ("ENV".to_string(), "/etc/mkshrc".to_string()),
        ];
        let envp = build_recovery_service_envp("/host/rootfs", false, &rc_exports);
        assert_eq!(env_get(&envp, "ANDROID_ROOT"), Some("/system2"));
        assert_eq!(env_get(&envp, "ENV"), Some("/etc/mkshrc"));
        assert_eq!(
            env_get(&envp, "LD_PRELOAD"),
            Some("/host/rootfs/sbin/libtwrp_fb_hook.so"),
            "guest rc export must never override the twoyi preload chain"
        );
        assert_eq!(env_get(&envp, "TWOYI_ROOTFS"), Some("/host/rootfs"));
    }

    #[test]
    fn z256_envp_compat_shim_prepends_to_preload_chain() {
        let envp = build_recovery_service_envp("/host/rootfs", true, &[]);
        let ld_preload = env_get(&envp, "LD_PRELOAD").unwrap();
        let shim = ld_preload.find("libbionic_compat.so");
        let fb = ld_preload.find("libtwrp_fb_hook.so");
        assert!(shim.is_some() && fb.is_some());
        assert!(
            shim.unwrap() < fb.unwrap(),
            "the FORTIFY shim must precede the fb hook (6-Z236 order)"
        );
    }

    #[test]
    fn z256_parse_rc_export_lines_guest_truth() {
        let rc = concat!(
            "# comment — skipped\n",
            "\n",
            "on init\n",
            "    export PATH /sbin:/system/bin\n",
            "    export LD_LIBRARY_PATH /sbin\n",
            "\n",
            "    export ANDROID_ROOT /system\n",
            "    export EMPTY\n",
            "service recovery /sbin/recovery\n",
            "    seclabel u:r:recovery:s0\n",
        );
        let mut out: Vec<(String, String)> = Vec::new();
        parse_rc_export_lines(rc, &mut out);
        let got: Vec<(String, String)> = out;
        assert_eq!(
            got,
            vec![
                ("PATH".to_string(), "/sbin:/system/bin".to_string()),
                ("LD_LIBRARY_PATH".to_string(), "/sbin".to_string()),
                ("ANDROID_ROOT".to_string(), "/system".to_string()),
            ],
            "comments, blanks, value-less exports and service options must be skipped"
        );
    }

    #[test]
    fn z256_parse_rc_export_lines_keeps_multiword_value() {
        let rc = "    export LD_CONFIG_FILE /sbin/ld.config.txt.extra\n";
        let mut out: Vec<(String, String)> = Vec::new();
        parse_rc_export_lines(rc, &mut out);
        assert_eq!(
            out,
            vec![(
                "LD_CONFIG_FILE".to_string(),
                "/sbin/ld.config.txt.extra".to_string()
            )]
        );
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

    /// `hook_library_candidates` must START with the 5 documented
    /// candidate paths in priority order (6-Z226 added candidate #0b:
    /// {data_dir}/files/<name> — the RomManager asset-extraction path
    /// for the 32-bit hook variants; the app-level rootfs path that
    /// RomManager's `ensureLibSymlink` ACTUALLY uses, which is NOT the
    /// same as `cfg.rootfs` for per-profile rootfs). The list may be
    /// followed by APK native lib dir scan results (candidate #5+) if
    /// `/data/app/` exists -- on the Linux devcontainer test runner it
    /// doesn't, so the list is exactly 5 here.
    #[test]
    fn hook_library_candidates_starts_with_four_documented_paths() {
        let cfg = Config {
            rootfs: "/data/data/io.twoyi/profiles/default/rootfs".to_string(),
            data_dir: "/data/data/io.twoyi".to_string(),
            ..Config::default()
        };
        let cands = hook_library_candidates(&cfg, "libgetpid_hook.so");
        // The first 5 candidates are the documented paths (6-Z226 added
        // the files-dir asset candidate at #5). The APK dir scan
        // (candidate #6+) returns 0 entries on Linux.
        assert!(
            cands.len() >= 5,
            "expected at least 5 candidates, got {}: {:?}",
            cands.len(),
            cands
        );
        // 0. Direct rootfs (historical fallback).
        assert_eq!(
            cands[0],
            "/data/data/io.twoyi/profiles/default/rootfs/libgetpid_hook.so"
        );
        // 1. Profile rootfs system/lib64 (RomManager per-profile symlink).
        assert_eq!(
            cands[1],
            "/data/data/io.twoyi/profiles/default/rootfs/system/lib64/libgetpid_hook.so"
        );
        // 2. App-level rootfs system/lib64 -- the CONFIRMED working path
        //    from logcat (ensureLibSymlink target).
        assert_eq!(
            cands[2],
            "/data/data/io.twoyi/rootfs/system/lib64/libgetpid_hook.so"
        );
        // 3. App-level rootfs root (alternative).
        assert_eq!(cands[3], "/data/data/io.twoyi/rootfs/libgetpid_hook.so");
        // 4. 6-Z226: {data_dir}/files — RomManager asset extraction
        //    (the _arm32 hook variants live here).
        assert_eq!(cands[4], "/data/data/io.twoyi/files/libgetpid_hook.so");
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
        // The 5 documented candidates + the 6-Z226 files-dir candidate
        // collapse to 3 unique paths after dedup (rootfs/{lib} ==
        // data_dir/rootfs/{lib}, and rootfs/system/lib64/{lib} ==
        // data_dir/rootfs/system/lib64/{lib}; data_dir/files/<lib> is
        // always unique). The APK dir scan returns 0 entries on Linux.
        assert_eq!(
            cands.len(),
            3,
            "expected 3 UNIQUE candidates after dedup (rootfs == {{data_dir}}/rootfs + the 6-Z226 files-dir candidate), got {}: {:?}",
            cands.len(),
            cands
        );
        // Verify all unique paths are present (including 6-Z226's).
        let has_files_dir = cands
            .iter()
            .any(|p| p == "/data/user/11/io.twoyi/files/libtwrp_fb_hook.so");
        assert!(has_files_dir, "missing files-dir candidate: {:?}", cands);
        let has_rootfs_root = cands
            .iter()
            .any(|p| p == "/data/user/11/io.twoyi/rootfs/libtwrp_fb_hook.so");
        assert!(
            has_rootfs_root,
            "missing rootfs/<lib> candidate: {:?}",
            cands
        );
        let has_rootfs_lib64 = cands
            .iter()
            .any(|p| p == "/data/user/11/io.twoyi/rootfs/system/lib64/libtwrp_fb_hook.so");
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
    /// recovery has existing options (like seclabel) — the setenv lines
    /// (LD_PRELOAD from 6-Z29 + LD_LIBRARY_PATH from 6-Z36) are inserted
    /// BEFORE the existing options, and the pre-existing seclabel is
    /// preserved EXACTLY once (6-Z76: no duplicate).
    #[test]
    fn patch_twrp_init_rc_inserts_before_existing_options() {
        let input = "service recovery /sbin/recovery\n    seclabel u:r:recovery:s0\n";
        let patched = patch_twrp_init_rc_recovery_service(input).expect("should patch");
        assert!(
            patched.contains("service recovery /sbin/recovery\n    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so\n    setenv LD_LIBRARY_PATH /sbin:/system/lib:/system/lib64\n    seclabel u:r:recovery:s0"),
            "setenv lines should be inserted before seclabel. Patched:\n{}",
            patched
        );
        // Task 6-Z76: an existing seclabel option must NOT be duplicated.
        assert_eq!(
            patched.matches("seclabel").count(),
            1,
            "seclabel must appear exactly once. Patched:\n{}",
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
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs, 720, 1600, None);
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
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs, 720, 1600, None);
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
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs, 720, 1600, None);
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
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs, 720, 1600, None);
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
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs, 720, 1600, None);

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
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs, 720, 1600, None);
        let init_rc_after_first = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs, 720, 1600, None);
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
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs, 720, 1600, None);
        let init_rc_after_first = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs, 720, 1600, None);
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
        patch_twrp_init_rc_recovery_service_in_rootfs(&rootfs, 720, 1600, None);
        let extra = std::fs::read_to_string(dir.join("extra.rc")).unwrap();
        assert!(
            extra.contains("    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so"),
            "relative-imported extra.rc should be patched. Got:\n{}",
            extra
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── 6-Z220: AOSP-layout service patch + stdio_to_kmsg gating ────────

    #[test]
    fn binary_contains_string_finds_and_rejects() {
        let dir = std::env::temp_dir().join(format!("twoyi-6z220-bcs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("init");
        // spanning-boundary content: 'o' at end of one chunk, "stdio_to_kmsg"
        // split across the 64 KiB boundary is exercised via the overlap tail.
        let mut big = vec![b'x'; 64 * 1024];
        big.extend_from_slice(b"junkstdio_to_kmsgtail");
        std::fs::write(&p, &big).unwrap();
        assert!(binary_contains_string(p.to_str().unwrap(), "stdio_to_kmsg"));
        assert!(!binary_contains_string(
            p.to_str().unwrap(),
            "NOT_PRESENT_TOKEN"
        ));
        // missing file → false (no crash)
        assert!(!binary_contains_string(
            dir.join("absent").to_str().unwrap(),
            "stdio_to_kmsg"
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rootfs_patcher_aosp_chain_and_stdio_to_kmsg() {
        let dir = make_test_rootfs(
            "service recovery /system/bin/recovery\n    seclabel u:r:recovery:s0\n",
        );
        // Fake a modern init that understands stdio_to_kmsg.
        std::fs::create_dir_all(dir.join("system/bin")).unwrap();
        std::fs::write(
            dir.join("system/bin/init"),
            b"binary with stdio_to_kmsg option table",
        )
        .unwrap();
        let rootfs = dir.to_string_lossy().into_owned();
        patch_twrp_init_rc_recovery_service_in_rootfs(
            &rootfs,
            720,
            1600,
            Some(AOSP_SERVICE_PRELOAD_CHAIN),
        );
        let rc = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        assert!(
            rc.contains(&format!(
                "    setenv LD_PRELOAD {}",
                AOSP_SERVICE_PRELOAD_CHAIN
            )),
            "AOSP chain must be injected. Got:\n{}",
            rc
        );
        assert!(
            rc.contains("    stdio_to_kmsg"),
            "stdio_to_kmsg must be emitted for a modern init"
        );
        assert!(
            !rc.contains("setenv LD_LIBRARY_PATH /sbin"),
            "32-bit LD_LIBRARY_PATH must NOT be set for the native chain"
        );
        // FB hook precedes shlib inside the injected chain (6-Z218a order).
        let fb = rc.find("libtwrp_fb_hook.so").unwrap();
        let shlib = rc.find("libtwoyi_loader_shlib.so").unwrap();
        assert!(
            fb < shlib,
            "fb hook must precede shlib in the service chain"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rootfs_patcher_stale_legacy_chain_is_replaced_not_duplicated() {
        // A previous boot in TWRP mode left the /sbin chain; an AOSP-mode
        // boot must REPLACE it (exactly one setenv LD_PRELOAD survives).
        let dir = make_test_rootfs(
            "service recovery /system/bin/recovery\n    setenv LD_PRELOAD /sbin/libtwrp_fb_hook.so\n",
        );
        // init WITHOUT the stdio_to_kmsg literal (old init) → option skipped.
        std::fs::create_dir_all(dir.join("system/bin")).unwrap();
        std::fs::write(dir.join("system/bin/init"), b"old init binary").unwrap();
        let rootfs = dir.to_string_lossy().into_owned();
        patch_twrp_init_rc_recovery_service_in_rootfs(
            &rootfs,
            720,
            1600,
            Some(AOSP_SERVICE_PRELOAD_CHAIN),
        );
        let rc = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        let count = rc.matches("setenv LD_PRELOAD").count();
        assert_eq!(
            count, 1,
            "exactly one LD_PRELOAD setenv must survive. Got:\n{}",
            rc
        );
        assert!(rc.contains(&format!(
            "    setenv LD_PRELOAD {}",
            AOSP_SERVICE_PRELOAD_CHAIN
        )));
        assert!(
            !rc.contains("/sbin/libtwrp_fb_hook.so"),
            "stale legacy chain must be gone"
        );
        assert!(
            !rc.contains("stdio_to_kmsg"),
            "stdio_to_kmsg must be skipped for old init"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rootfs_patcher_aosp_idempotent_on_second_call() {
        let dir = make_test_rootfs("service recovery /system/bin/recovery\n");
        std::fs::create_dir_all(dir.join("system/bin")).unwrap();
        std::fs::write(dir.join("system/bin/init"), b"stdio_to_kmsg here").unwrap();
        let rootfs = dir.to_string_lossy().into_owned();
        patch_twrp_init_rc_recovery_service_in_rootfs(
            &rootfs,
            720,
            1600,
            Some(AOSP_SERVICE_PRELOAD_CHAIN),
        );
        let once = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        patch_twrp_init_rc_recovery_service_in_rootfs(
            &rootfs,
            720,
            1600,
            Some(AOSP_SERVICE_PRELOAD_CHAIN),
        );
        let twice = std::fs::read_to_string(dir.join("init.rc")).unwrap();
        assert_eq!(once, twice, "second AOSP-mode call must be a no-op");
        assert_eq!(twice.matches("setenv LD_PRELOAD").count(), 1);
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

    // ========================================================================
    // Tests for `patch_twrp_init_selinux_load_skip` — the 6-byte NOP patch
    // at file offset 0x1006 (vaddr 0x08049006) that makes init's
    // selinux-load-failure path UNREACHABLE, so the `while(1) pause();`
    // loop becomes unreachable from main() (DEFINITIVE root-cause fix per
    // 6-I's disassembly — see worklog entry 6-I).
    //
    // The test set mirrors `patch_twrp_init_klog_init_*`:
    //   * applies cleanly to an unpatched binary
    //   * is IDEMPOTENT (applying twice == applying once)
    //   * returns NotFound when the pattern is absent
    //   * refuses to patch if the pattern matches at an UNEXPECTED offset
    //     (safety check against coincidental matches in a different code
    //     path)
    //   * works on a real TWRP init binary extracted from the ramdisk
    //     (regression test for TWRP version drift)
    // ========================================================================

    /// Build a binary containing the selinux-load-failure pattern at the
    /// expected file offset 0x1004 (vaddr 0x08049004 — the `test eax, eax`
    /// pre-context, with the `js 0x080490cf` at file offset 0x1006).
    ///
    /// Only built on non-aarch64 hosts — the pattern-matching tests below
    /// are x86/i386-specific (they verify the i386 byte pattern) and the
    /// function under test short-circuits to `Skipped` on aarch64.
    #[cfg(not(target_arch = "aarch64"))]
    fn build_selinux_load_skip_pattern_at_expected_offset() -> Vec<u8> {
        // 8 KiB of NOP filler — a recognizable, neutral background that
        // cannot accidentally contain the 8-byte pattern.
        let mut v = vec![0x90u8; 8 * 1024];
        // Place the pattern at file offset 0x1004 (the expected match offset
        // for vaddr 0x08049006 = file offset 0x1006 for the `js` itself,
        // minus the 2-byte `test eax, eax` pre-context at 0x1004).
        let off: usize = 0x1004;
        v[off] = 0x85; // test
        v[off + 1] = 0xc0; // eax, eax
        v[off + 2] = 0x0f; // js opcode byte 1
        v[off + 3] = 0x88; // js opcode byte 2 (jump-if-sign)
        v[off + 4] = 0xc3; // signed disp byte 0 (LSB)
        v[off + 5] = 0x00; // signed disp byte 1
        v[off + 6] = 0x00; // signed disp byte 2
        v[off + 7] = 0x00; // signed disp byte 3 (MSB)
        v
    }

    /// `patch_twrp_init_selinux_load_skip` must find the pattern at the
    /// expected offset (0x1004) and replace the 6-byte `js` at offset
    /// 0x1006 with 6 NOPs.
    ///
    /// Skipped on aarch64: the function short-circuits to `Skipped` there
    /// (the i386 byte pattern is irrelevant), so the x86-specific assertions
    /// below would never hold on arm64.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_selinux_load_skip_applies_to_unpatched_binary() {
        let mut bytes = build_selinux_load_skip_pattern_at_expected_offset();
        assert_eq!(
            patch_twrp_init_selinux_load_skip(&mut bytes),
            SelinuxLoadSkipPatchResult::Applied,
            "patch should apply to unpatched binary"
        );
        // The 6 `js`-bytes at offset 0x1006 should now be 6 NOPs.
        let off: usize = 0x1006;
        assert_eq!(
            &bytes[off..off + 6],
            &[0x90; 6],
            "`js` bytes should be NOP'd"
        );
        // The 2 pre-context bytes at 0x1004-0x1005 (`test eax, eax`) must be
        // preserved unchanged.
        assert_eq!(
            &bytes[off - 2..off],
            &[0x85, 0xc0],
            "pre-context (`test eax, eax`) should be preserved"
        );
    }

    /// `patch_twrp_init_selinux_load_skip` must be IDEMPOTENT: applying it
    /// twice yields the same result as applying it once. The second call
    /// must return `AlreadyApplied` (not `Applied`) and must NOT modify the
    /// bytes.
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_selinux_load_skip_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_selinux_load_skip_is_idempotent() {
        let mut bytes = build_selinux_load_skip_pattern_at_expected_offset();
        assert_eq!(
            patch_twrp_init_selinux_load_skip(&mut bytes),
            SelinuxLoadSkipPatchResult::Applied,
            "first patch should be Applied"
        );
        let after_first = bytes.clone();
        assert_eq!(
            patch_twrp_init_selinux_load_skip(&mut bytes),
            SelinuxLoadSkipPatchResult::AlreadyApplied,
            "second patch should be AlreadyApplied (idempotent)"
        );
        assert_eq!(
            bytes, after_first,
            "second patch should not modify the binary"
        );
    }

    /// `patch_twrp_init_selinux_load_skip` must return `NotFound` if the
    /// pattern is absent (e.g., a different TWRP version where the binary
    /// layout has shifted, or a binary with no selinux-load code path at
    /// all).
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_selinux_load_skip_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_selinux_load_skip_returns_not_found_if_pattern_not_found() {
        // 8 KiB of NOP filler — no pattern present.
        let mut bytes = vec![0x90u8; 8 * 1024];
        assert_eq!(
            patch_twrp_init_selinux_load_skip(&mut bytes),
            SelinuxLoadSkipPatchResult::NotFound,
            "patch should return NotFound when pattern is absent"
        );
    }

    /// `patch_twrp_init_selinux_load_skip` must refuse to patch if the
    /// pattern matches at an UNEXPECTED offset (safety check against
    /// coincidental matches in a different code path). Without this check,
    /// a coincidental match in unrelated code could brick the binary.
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_selinux_load_skip_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_selinux_load_skip_refuses_unexpected_offset() {
        let mut bytes = vec![0x90u8; 8 * 1024];
        // Place the pattern at an UNEXPECTED offset (0x500 instead of the
        // expected 0x1004). The 8-byte pattern will match, but the offset
        // check should refuse to patch (a coincidental match in unrelated
        // code could brick the binary).
        let off: usize = 0x500;
        bytes[off] = 0x85;
        bytes[off + 1] = 0xc0;
        bytes[off + 2] = 0x0f;
        bytes[off + 3] = 0x88;
        bytes[off + 4] = 0xc3;
        bytes[off + 5] = 0x00;
        bytes[off + 6] = 0x00;
        bytes[off + 7] = 0x00;
        assert_eq!(
            patch_twrp_init_selinux_load_skip(&mut bytes),
            SelinuxLoadSkipPatchResult::NotFound,
            "patch should refuse to apply at unexpected offset (safety check)"
        );
        // The bytes should be UNCHANGED (no patch applied).
        assert_eq!(
            &bytes[off..off + 8],
            &[0x85, 0xc0, 0x0f, 0x88, 0xc3, 0x00, 0x00, 0x00],
            "bytes should be unchanged when offset check refuses the patch"
        );
    }

    /// `patch_twrp_init_selinux_load_skip` must work on a real TWRP init
    /// binary extracted from `assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img`.
    /// This is a regression test: if the TWRP version changes (and the
    /// `js 0x080490cf` byte pattern is no longer at file offset 0x1006),
    /// this test will fail (alerting us to update the pattern / offset).
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_selinux_load_skip_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_selinux_load_skip_works_on_real_twrp_init_binary() {
        // The TWRP boot image is at `assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img`
        // (relative to the repo root). We need to extract the ramdisk,
        // decompress it, and read the /init file. This is the SAME extraction
        // pipeline as `patch_twrp_init_klog_init_works_on_real_twrp_init_binary`
        // above — we reuse the same `decompress_gzip` + `find_cpio_entry`
        // helpers.
        let boot_img_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img");
        if !boot_img_path.exists() {
            eprintln!(
                "skip: TWRP boot image not found at {} (this is OK in CI without assets)",
                boot_img_path.display()
            );
            return;
        }
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
        if ramdisk_gz.len() < 2 || ramdisk_gz[0] != 0x1f || ramdisk_gz[1] != 0x8b {
            eprintln!("skip: TWRP ramdisk is not gzip-compressed");
            return;
        }
        let ramdisk_cpio = match decompress_gzip(ramdisk_gz) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("skip: failed to decompress TWRP ramdisk: {}", e);
                return;
            }
        };
        let init_bytes = match find_cpio_entry(&ramdisk_cpio, b"init") {
            Some(b) => b,
            None => {
                eprintln!("skip: /init not found in TWRP ramdisk cpio");
                return;
            }
        };

        // Sanity-check the binary is large enough to contain the patch site.
        assert!(
            init_bytes.len() >= 0x1006 + 6,
            "init binary too small for selinux-load-failure patch site: {} bytes (need at least {})",
            init_bytes.len(),
            0x1006 + 6
        );

        // Verify the UNPATCHED pattern is present at file offset 0x1004.
        // (`test eax, eax` at 0x1004-0x1005; `js 0x080490cf` at 0x1006-0x100b.)
        assert_eq!(
            &init_bytes[0x1004..0x1004 + 8],
            &[0x85, 0xc0, 0x0f, 0x88, 0xc3, 0x00, 0x00, 0x00],
            "selinux-load-failure pattern (`test eax, eax; js 0x080490cf`) should be present at file offset 0x1004 in real TWRP init binary (TWRP version may have changed — update patch_twrp_init_selinux_load_skip pattern / offset)"
        );

        // Apply the patch.
        let mut init_bytes_mut = init_bytes.clone();
        assert_eq!(
            patch_twrp_init_selinux_load_skip(&mut init_bytes_mut),
            SelinuxLoadSkipPatchResult::Applied,
            "patch should apply to real TWRP init binary"
        );
        // Verify the 6 `js`-bytes at offset 0x1006 are now 6 NOPs.
        assert_eq!(
            &init_bytes_mut[0x1006..0x1006 + 6],
            &[0x90; 6],
            "js bytes at offset 0x1006 should be NOP'd after patch"
        );
        // Verify the 2 pre-context bytes at 0x1004-0x1005 are unchanged.
        assert_eq!(
            &init_bytes_mut[0x1004..0x1006],
            &[0x85, 0xc0],
            "pre-context (`test eax, eax`) should be preserved at offset 0x1004"
        );
        // Apply again — should be idempotent (AlreadyApplied).
        assert_eq!(
            patch_twrp_init_selinux_load_skip(&mut init_bytes_mut),
            SelinuxLoadSkipPatchResult::AlreadyApplied,
            "patch should be idempotent on real TWRP init binary"
        );
    }

    // ========================================================================
    // Tests for `patch_twrp_init_property_contexts_crash_nop` — the 7-byte
    // NOP patch at file offset 0x58b9e (vaddr 0x080a0b9e) that makes init's
    // property_contexts parser SKIP the write through the garbage
    // `ctx->field_at_0x14` pointer instead of SIGSEGV'ing (PRAGMATIC
    // "make it not crash" patch per Task 6-M + DISPATCHER-FINAL-5/6).
    //
    // The test set mirrors `patch_twrp_init_selinux_load_skip_*`:
    //   * applies cleanly to an unpatched binary at the expected offset
    //   * is IDEMPOTENT (applying twice == applying once)
    //   * returns NotFound when the binary is too small / pattern absent
    //     at the expected offset
    //   * refuses to apply when the bytes at the expected offset are some
    //     UNEXPECTED value (safety check against version drift / an
    //     unrelated patch having been applied there)
    //   * works on a real TWRP init binary extracted from the ramdisk
    //     (regression test for TWRP version drift)
    // ========================================================================

    /// Build a binary containing the property_contexts crash instruction
    /// (`movl $0x0, 0x4(%edx)` = `c7 42 04 00 00 00 00`) at the expected
    /// file offset 0x58b9e (vaddr 0x080a0b9e).
    ///
    /// Only built on non-aarch64 hosts — the pattern-matching tests below
    /// are x86/i386-specific (they verify the i386 byte pattern) and the
    /// function under test short-circuits to `Skipped` on aarch64.
    #[cfg(not(target_arch = "aarch64"))]
    fn build_property_contexts_crash_nop_pattern_at_expected_offset() -> Vec<u8> {
        // 384 KiB of NOP filler — large enough to contain the expected
        // match offset 0x58b9e (≈ 363 KiB) + 7 bytes of pattern. The
        // filler is a recognizable, neutral background that cannot
        // accidentally contain the 7-byte pattern.
        let mut v = vec![0x90u8; 384 * 1024];
        // Place the 7-byte `movl $0x0, 0x4(%edx)` instruction at the
        // expected file offset 0x58b9e (vaddr 0x080a0b9e — the crash
        // site identified by 6-K + DISPATCHER-FINAL-5).
        let off: usize = 0x58b9e;
        v[off] = 0xc7; // opcode (MOV r/m32, imm32)
        v[off + 1] = 0x42; // ModR/M (mod=01 disp8, reg=000, rm=010 edx)
        v[off + 2] = 0x04; // 8-bit displacement: [edx + 0x4]
        v[off + 3] = 0x00; // imm32 byte 0 (LSB)
        v[off + 4] = 0x00; // imm32 byte 1
        v[off + 5] = 0x00; // imm32 byte 2
        v[off + 6] = 0x00; // imm32 byte 3 (MSB)
        v
    }

    /// `patch_twrp_init_property_contexts_crash_nop` must find the pattern
    /// at the expected offset (0x58b9e) and replace the 7-byte
    /// `movl $0x0, 0x4(%edx)` instruction with 7 NOPs.
    ///
    /// Skipped on aarch64: the function short-circuits to `Skipped` there
    /// (the i386 byte pattern is irrelevant), so the x86-specific
    /// assertions below would never hold on arm64.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_property_contexts_crash_nop_applies_to_unpatched_binary() {
        let mut bytes = build_property_contexts_crash_nop_pattern_at_expected_offset();
        assert_eq!(
            patch_twrp_init_property_contexts_crash_nop(&mut bytes),
            PropertyContextsCrashNopPatchResult::Applied,
            "patch should apply to unpatched binary"
        );
        // The 7 bytes at offset 0x58b9e should now be 7 NOPs.
        let off: usize = 0x58b9e;
        assert_eq!(
            &bytes[off..off + 7],
            &[0x90; 7],
            "`movl $0x0, 0x4(%edx)` bytes should be NOP'd"
        );
    }

    /// `patch_twrp_init_property_contexts_crash_nop` must be IDEMPOTENT:
    /// applying it twice yields the same result as applying it once. The
    /// second call must return `AlreadyApplied` (not `Applied`) and must
    /// NOT modify the bytes.
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_property_contexts_crash_nop_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_property_contexts_crash_nop_is_idempotent() {
        let mut bytes = build_property_contexts_crash_nop_pattern_at_expected_offset();
        assert_eq!(
            patch_twrp_init_property_contexts_crash_nop(&mut bytes),
            PropertyContextsCrashNopPatchResult::Applied,
            "first patch should be Applied"
        );
        let after_first = bytes.clone();
        assert_eq!(
            patch_twrp_init_property_contexts_crash_nop(&mut bytes),
            PropertyContextsCrashNopPatchResult::AlreadyApplied,
            "second patch should be AlreadyApplied (idempotent)"
        );
        assert_eq!(
            bytes, after_first,
            "second patch should not modify the binary"
        );
    }

    /// `patch_twrp_init_property_contexts_crash_nop` must return
    /// `NotFound` if the binary is too small to contain the patch site
    /// at file offset 0x58b9e (e.g. a tiny test binary, or — in production
    /// — a different binary that isn't TWRP init).
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_property_contexts_crash_nop_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_property_contexts_crash_nop_returns_not_found_when_binary_too_small() {
        // 4 KiB of NOP filler — too small to contain the patch site at
        // file offset 0x58b9e (≈ 363 KiB).
        let mut bytes = vec![0x90u8; 4 * 1024];
        assert_eq!(
            patch_twrp_init_property_contexts_crash_nop(&mut bytes),
            PropertyContextsCrashNopPatchResult::NotFound,
            "patch should return NotFound when binary is too small"
        );
    }

    /// `patch_twrp_init_property_contexts_crash_nop` must return
    /// `NotFound` when the 7 bytes at file offset 0x58b9e are neither
    /// the unpatched pattern (`c7 42 04 00 00 00 00`) nor the already-
    /// patched signature (`90 90 90 90 90 90 90`). This is the safety
    /// check against TWRP version drift (the crash instruction moved to
    /// a different offset) OR an unrelated patch having been applied at
    /// the same offset.
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_property_contexts_crash_nop_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_property_contexts_crash_nop_refuses_unexpected_pattern_at_offset() {
        let mut bytes = vec![0x90u8; 384 * 1024];
        // Place an UNEXPECTED pattern at offset 0x58b9e (a `nop; int3`
        // sequence: `90 cc 90 cc 90 cc 90`). The patch should refuse to
        // apply (NotFound) — it doesn't know whether the bytes are an
        // intentional unrelated patch or TWRP version drift.
        let off: usize = 0x58b9e;
        bytes[off] = 0x90;
        bytes[off + 1] = 0xcc;
        bytes[off + 2] = 0x90;
        bytes[off + 3] = 0xcc;
        bytes[off + 4] = 0x90;
        bytes[off + 5] = 0xcc;
        bytes[off + 6] = 0x90;
        assert_eq!(
            patch_twrp_init_property_contexts_crash_nop(&mut bytes),
            PropertyContextsCrashNopPatchResult::NotFound,
            "patch should refuse to apply when bytes at offset are unexpected"
        );
        // The bytes at offset 0x58b9e should be UNCHANGED.
        assert_eq!(
            &bytes[off..off + 7],
            &[0x90, 0xcc, 0x90, 0xcc, 0x90, 0xcc, 0x90],
            "bytes should be unchanged when offset check refuses the patch"
        );
    }

    /// `patch_twrp_init_property_contexts_crash_nop` must work on a real
    /// TWRP init binary extracted from
    /// `assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img`. This is a regression
    /// test: if the TWRP version changes (and the `movl $0x0, 0x4(%edx)`
    /// byte pattern is no longer at file offset 0x58b9e), this test
    /// will fail (alerting us to update the offset).
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_property_contexts_crash_nop_applies_to_unpatched_binary` above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_property_contexts_crash_nop_works_on_real_twrp_init_binary() {
        // The TWRP boot image is at `assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img`
        // (relative to the repo root). We extract the ramdisk, decompress
        // it, and read the /init file. This is the SAME extraction
        // pipeline as `patch_twrp_init_selinux_load_skip_works_on_real_twrp_init_binary`
        // above — we reuse the same `decompress_gzip` + `find_cpio_entry`
        // helpers.
        let boot_img_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img");
        if !boot_img_path.exists() {
            eprintln!(
                "skip: TWRP boot image not found at {} (this is OK in CI without assets)",
                boot_img_path.display()
            );
            return;
        }
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
        if ramdisk_gz.len() < 2 || ramdisk_gz[0] != 0x1f || ramdisk_gz[1] != 0x8b {
            eprintln!("skip: TWRP ramdisk is not gzip-compressed");
            return;
        }
        let ramdisk_cpio = match decompress_gzip(ramdisk_gz) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("skip: failed to decompress TWRP ramdisk: {}", e);
                return;
            }
        };
        let init_bytes = match find_cpio_entry(&ramdisk_cpio, b"init") {
            Some(b) => b,
            None => {
                eprintln!("skip: /init not found in TWRP ramdisk cpio");
                return;
            }
        };

        // Sanity-check the binary is large enough to contain the patch site
        // at file offset 0x58b9e (≈ 363 KiB + 7 bytes).
        assert!(
            init_bytes.len() >= 0x58b9e + 7,
            "init binary too small for property_contexts crash-nop patch site: {} bytes (need at least {})",
            init_bytes.len(),
            0x58b9e + 7
        );

        // Verify the UNPATCHED pattern is present at file offset 0x58b9e
        // (`movl $0x0, 0x4(%edx)` = `c7 42 04 00 00 00 00`).
        assert_eq!(
            &init_bytes[0x58b9e..0x58b9e + 7],
            &[0xc7, 0x42, 0x04, 0x00, 0x00, 0x00, 0x00],
            "property_contexts crash instruction (`movl $0x0, 0x4(%edx)`) should be present at file offset 0x58b9e in real TWRP init binary (TWRP version may have changed — update patch_twrp_init_property_contexts_crash_nop offset)"
        );

        // Apply the patch.
        let mut init_bytes_mut = init_bytes.clone();
        assert_eq!(
            patch_twrp_init_property_contexts_crash_nop(&mut init_bytes_mut),
            PropertyContextsCrashNopPatchResult::Applied,
            "patch should apply to real TWRP init binary"
        );
        // Verify the 7 bytes at offset 0x58b9e are now 7 NOPs.
        assert_eq!(
            &init_bytes_mut[0x58b9e..0x58b9e + 7],
            &[0x90; 7],
            "`movl $0x0, 0x4(%edx)` bytes at offset 0x58b9e should be NOP'd after patch"
        );
        // Apply again — should be idempotent (AlreadyApplied).
        assert_eq!(
            patch_twrp_init_property_contexts_crash_nop(&mut init_bytes_mut),
            PropertyContextsCrashNopPatchResult::AlreadyApplied,
            "patch should be idempotent on real TWRP init binary"
        );
    }

    // ========================================================================
    // Tests for `patch_twrp_init_read_file_sigsegv` — the 2-byte NOP
    // patch at file offset 0xaf65 (vaddr 0x8052f65) that makes init's
    // read_file() SKIP the *arg2 = readcount store through the garbage
    // pointer (0x696e692f, ASCII "/ini" leaked by a SIGSYS-handler race)
    // instead of SIGSEGV'ing (PRAGMATIC symptom-mask patch per Task 6-V
    // + 6-U DIAG KLOG + DISPATCHER-UPDATE-12).
    //
    // The test set mirrors `patch_twrp_init_property_contexts_crash_nop_*`:
    //   * applies cleanly to an unpatched binary at the expected offset
    //   * is IDEMPOTENT (applying twice == applying once)
    //   * returns NotFound when the binary is too small / pattern absent
    //     at the expected offset
    //   * refuses to apply when the bytes at the expected offset are some
    //     UNEXPECTED value (safety check against version drift / an
    //     unrelated patch having been applied there)
    // ========================================================================

    /// Build a binary containing the read_file() crash instruction
    /// (`mov %ecx,(%eax)` = `89 08`) at the expected file offset 0xaf65
    /// (vaddr 0x8052f65).
    ///
    /// Only built on non-aarch64 hosts — the pattern-matching tests
    /// below are x86/i386-specific (they verify the i386 byte pattern)
    /// and the function under test short-circuits to `Skipped` on
    /// aarch64.
    #[cfg(not(target_arch = "aarch64"))]
    fn build_read_file_sigsegv_pattern_at_expected_offset() -> Vec<u8> {
        // 48 KiB of NOP filler — large enough to contain the expected
        // match offset 0xaf65 (≈ 45 KiB) + 2 bytes of pattern. The
        // filler is a recognizable, neutral background that cannot
        // accidentally contain the 2-byte pattern (a sequence of all
        // 0x90 bytes has no 0x89).
        let mut v = vec![0x90u8; 48 * 1024];
        // Place the 2-byte `mov %ecx,(%eax)` instruction at the
        // expected file offset 0xaf65 (vaddr 0x8052f65 — the crash
        // site identified by 6-U DIAG KLOG + 6-V-pre disassembly).
        let off: usize = 0xaf65;
        v[off] = 0x89; // opcode (MOV r/m32, r32)
        v[off + 1] = 0x08; // ModR/M (mod=00, reg=001 ecx, rm=000 eax)
        v
    }

    /// `patch_twrp_init_read_file_sigsegv` must find the pattern at the
    /// expected offset (0xaf65) and replace the 2-byte `mov %ecx,(%eax)`
    /// instruction with 2 NOPs.
    ///
    /// Skipped on aarch64: the function short-circuits to `Skipped`
    /// there (the i386 byte pattern is irrelevant), so the x86-specific
    /// assertions below would never hold on arm64.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_read_file_sigsegv_applies_to_unpatched_binary() {
        let mut bytes = build_read_file_sigsegv_pattern_at_expected_offset();
        assert_eq!(
            patch_twrp_init_read_file_sigsegv(&mut bytes),
            ReadFileSigsegvPatchResult::Applied,
            "patch should apply to unpatched binary"
        );
        // The 2 bytes at offset 0xaf65 should now be 2 NOPs.
        let off: usize = 0xaf65;
        assert_eq!(
            &bytes[off..off + 2],
            &[0x90; 2],
            "`mov %ecx,(%eax)` bytes should be NOP'd"
        );
    }

    /// `patch_twrp_init_read_file_sigsegv` must be IDEMPOTENT: applying
    /// it twice yields the same result as applying it once. The second
    /// call must return `AlreadyApplied` (not `Applied`) and must NOT
    /// modify the bytes.
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_read_file_sigsegv_applies_to_unpatched_binary`
    /// above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_read_file_sigsegv_is_idempotent() {
        let mut bytes = build_read_file_sigsegv_pattern_at_expected_offset();
        assert_eq!(
            patch_twrp_init_read_file_sigsegv(&mut bytes),
            ReadFileSigsegvPatchResult::Applied,
            "first patch should be Applied"
        );
        let after_first = bytes.clone();
        assert_eq!(
            patch_twrp_init_read_file_sigsegv(&mut bytes),
            ReadFileSigsegvPatchResult::AlreadyApplied,
            "second patch should be AlreadyApplied (idempotent)"
        );
        assert_eq!(
            bytes, after_first,
            "second patch should not modify the binary"
        );
    }

    /// `patch_twrp_init_read_file_sigsegv` must return `NotFound` if
    /// the binary is too small to contain the patch site at file offset
    /// 0xaf65 (e.g. a tiny test binary, or — in production — a
    /// different binary that isn't TWRP init).
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_read_file_sigsegv_applies_to_unpatched_binary`
    /// above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_read_file_sigsegv_returns_not_found_when_binary_too_small() {
        // 4 KiB of NOP filler — too small to contain the patch site at
        // file offset 0xaf65 (≈ 45 KiB).
        let mut bytes = vec![0x90u8; 4 * 1024];
        assert_eq!(
            patch_twrp_init_read_file_sigsegv(&mut bytes),
            ReadFileSigsegvPatchResult::NotFound,
            "patch should return NotFound when binary is too small"
        );
    }

    /// `patch_twrp_init_read_file_sigsegv` must return `NotFound` when
    /// the 2 bytes at file offset 0xaf65 are neither the unpatched
    /// pattern (`89 08`) nor the already-patched signature (`90 90`).
    /// This is the safety check against TWRP version drift (the crash
    /// instruction moved to a different offset) OR an unrelated patch
    /// having been applied at the same offset.
    ///
    /// Skipped on aarch64 — see
    /// `patch_twrp_init_read_file_sigsegv_applies_to_unpatched_binary`
    /// above.
    #[test]
    #[cfg(not(target_arch = "aarch64"))]
    fn patch_twrp_init_read_file_sigsegv_refuses_unexpected_pattern_at_offset() {
        let mut bytes = vec![0x90u8; 48 * 1024];
        // Place an UNEXPECTED pattern at offset 0xaf65 (a `nop; int3`
        // sequence: `90 cc`). The patch should refuse to apply
        // (NotFound) — it doesn't know whether the bytes are an
        // intentional unrelated patch or TWRP version drift.
        let off: usize = 0xaf65;
        bytes[off] = 0x90;
        bytes[off + 1] = 0xcc;
        assert_eq!(
            patch_twrp_init_read_file_sigsegv(&mut bytes),
            ReadFileSigsegvPatchResult::NotFound,
            "patch should refuse to apply when bytes at offset are unexpected"
        );
        // The bytes at offset 0xaf65 should be UNCHANGED.
        assert_eq!(
            &bytes[off..off + 2],
            &[0x90, 0xcc],
            "bytes should be unchanged when offset check refuses the patch"
        );
    }

    // ========================================================================
    // Touch dispatcher tests (Task 3-A, Part 1).
    //
    // These tests cover the message-parse → encode-touch-* flow that
    // `spawn_touch_accept_thread` runs in production:
    //   * `TouchMessage::parse` roundtrips through `to_bytes` byte-for-byte.
    //   * `encode_touch_message` dispatches DOWN/MOVE/UP/CANCEL to the
    //     correct `devices::encode_touch_*` helper.
    //   * Per-slot tracking IDs are stable across a touch lifecycle
    //     (DOWN assigns, MOVE preserves, UP clears).
    //   * Out-of-range pointer_id and unknown action are dropped silently.
    //   * The full DOWN→MOVE→UP cycle produces the concatenation of the
    //     three encoded frames (verifies the wire-level integration).
    // ========================================================================

    /// Helper: build a `TouchMessage` (avoids repeating the field list
    /// in every test below).
    fn touch_msg(action: u32, pointer_id: i32, x: i32, y: i32, pressure: i32) -> TouchMessage {
        TouchMessage {
            action,
            pointer_id,
            x,
            y,
            pressure,
        }
    }

    /// Helper: zero timeval (so the encoded bytes are deterministic
    /// and tests can hand-compute expected values).
    fn zero_timeval() -> libc::timeval {
        libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        }
    }

    /// `TOUCH_MESSAGE_SIZE` must be 20 bytes (4×5 little-endian fields).
    /// The host's `input.rs` IPC writer will pack the same layout — a
    /// size mismatch would silently misparse the stream.
    #[test]
    fn touch_message_size_is_20_bytes() {
        assert_eq!(TOUCH_MESSAGE_SIZE, 20);
    }

    /// `TouchMessage::parse` roundtrips through `to_bytes` byte-for-byte.
    #[test]
    fn touch_message_parse_roundtrip() {
        let cases = [
            touch_msg(touch_action::DOWN, 0, 100, 200, 128),
            touch_msg(touch_action::MOVE, 1, 500, 750, 200),
            touch_msg(touch_action::UP, 4, 0, 0, 0),
            touch_msg(touch_action::CANCEL, 2, -10, -20, -30),
        ];
        for msg in cases {
            let bytes = msg.to_bytes();
            assert_eq!(bytes.len(), TOUCH_MESSAGE_SIZE);
            let parsed =
                TouchMessage::parse(&bytes).expect("parse should succeed for a 20-byte buffer");
            assert_eq!(parsed, msg, "roundtrip failed for {:?}", msg);
        }
    }

    /// `TouchMessage::parse` returns `None` for buffers shorter than
    /// `TOUCH_MESSAGE_SIZE`. This protects against a short-read on the
    /// IPC socket silently producing a garbled `TouchMessage`.
    #[test]
    fn touch_message_parse_rejects_short_buffer() {
        assert!(TouchMessage::parse(&[]).is_none());
        assert!(TouchMessage::parse(&[0u8; 19]).is_none());
        // Exactly 20 bytes parses OK.
        let msg = touch_msg(touch_action::DOWN, 0, 1, 2, 3);
        assert!(TouchMessage::parse(&msg.to_bytes()).is_some());
        // Extra bytes are ignored (we read the first 20) — but in
        // practice `read_exact` always produces exactly 20 bytes.
        let mut buf = msg.to_bytes().to_vec();
        buf.push(0xff);
        assert!(TouchMessage::parse(&buf).is_some());
    }

    /// Verify the on-wire byte layout of a `TouchMessage` (little-endian,
    /// fields at the documented offsets). This catches a struct-layout
    /// drift that would break inter-process IPC with the host's
    /// `input.rs`.
    #[test]
    fn touch_message_byte_layout() {
        let msg = touch_msg(touch_action::MOVE, 3, 0x12345678, -5, 255);
        let b = msg.to_bytes();
        assert_eq!(
            u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            touch_action::MOVE
        );
        assert_eq!(
            i32::from_le_bytes([b[4], b[5], b[6], b[7]]),
            3,
            "pointer_id at offset 4"
        );
        assert_eq!(
            i32::from_le_bytes([b[8], b[9], b[10], b[11]]),
            0x12345678,
            "x at offset 8"
        );
        assert_eq!(
            i32::from_le_bytes([b[12], b[13], b[14], b[15]]),
            -5,
            "y at offset 12"
        );
        assert_eq!(
            i32::from_le_bytes([b[16], b[17], b[18], b[19]]),
            255,
            "pressure at offset 16"
        );
    }

    /// DOWN must emit the full 8-event multi-touch frame and assign a
    /// fresh tracking ID to the slot. Verifies the
    /// `devices::encode_touch_down` helper is called with the right
    /// arguments.
    #[test]
    fn encode_touch_message_down_emits_full_frame_and_assigns_tracking_id() {
        let mut next_tid = 1i32;
        let mut tracking = [0i32; devices::MAX_POINTERS];
        let msg = touch_msg(touch_action::DOWN, 0, 100, 200, 128);
        let out = encode_touch_message(&msg, zero_timeval(), &mut next_tid, &mut tracking);

        assert_eq!(
            out.len(),
            8 * devices::InputEvent::size(),
            "DOWN frame must be 8 events"
        );
        assert_eq!(next_tid, 2, "DOWN must increment next_tracking_id");
        assert_eq!(tracking[0], 1, "DOWN must cache the assigned tracking ID");
    }

    /// MOVE without a preceding DOWN must be silently dropped (the
    /// kernel would treat a slot state change without a tracking ID as
    /// a stale slot — better to skip than confuse the InputReader).
    #[test]
    fn encode_touch_message_move_without_down_is_dropped() {
        let mut next_tid = 1i32;
        let mut tracking = [0i32; devices::MAX_POINTERS];
        let msg = touch_msg(touch_action::MOVE, 0, 100, 200, 128);
        let out = encode_touch_message(&msg, zero_timeval(), &mut next_tid, &mut tracking);
        assert!(out.is_empty(), "MOVE without DOWN must produce no bytes");
        assert_eq!(next_tid, 1, "MOVE must not bump the tracking-id counter");
    }

    /// MOVE after a DOWN must emit the 5-event move frame and preserve
    /// the slot's tracking ID (so the next UP uses the same ID).
    #[test]
    fn encode_touch_message_move_after_down_preserves_tracking_id() {
        let mut next_tid = 1i32;
        let mut tracking = [0i32; devices::MAX_POINTERS];

        // DOWN first.
        let down = touch_msg(touch_action::DOWN, 0, 100, 200, 128);
        let _ = encode_touch_message(&down, zero_timeval(), &mut next_tid, &mut tracking);
        let tid_after_down = tracking[0];
        assert_eq!(tid_after_down, 1);

        // MOVE on the same slot.
        let mv = touch_msg(touch_action::MOVE, 0, 150, 250, 200);
        let out = encode_touch_message(&mv, zero_timeval(), &mut next_tid, &mut tracking);
        assert_eq!(
            out.len(),
            5 * devices::InputEvent::size(),
            "MOVE frame must be 5 events"
        );
        assert_eq!(
            tracking[0], tid_after_down,
            "MOVE must preserve tracking ID"
        );
        assert_eq!(next_tid, 2, "MOVE must not bump the tracking-id counter");
    }

    /// UP after DOWN must emit the 5-event release frame and CLEAR the
    /// slot's tracking ID (so a subsequent MOVE on the same slot is
    /// dropped — the touch lifecycle is over).
    #[test]
    fn encode_touch_message_up_after_down_clears_tracking_id() {
        let mut next_tid = 1i32;
        let mut tracking = [0i32; devices::MAX_POINTERS];

        let down = touch_msg(touch_action::DOWN, 2, 100, 200, 128);
        let _ = encode_touch_message(&down, zero_timeval(), &mut next_tid, &mut tracking);
        assert_eq!(tracking[2], 1);

        let up = touch_msg(touch_action::UP, 2, 0, 0, 0);
        let out = encode_touch_message(&up, zero_timeval(), &mut next_tid, &mut tracking);
        assert_eq!(
            out.len(),
            5 * devices::InputEvent::size(),
            "UP frame must be 5 events"
        );
        assert_eq!(tracking[2], 0, "UP must clear the tracking ID");
    }

    /// CANCEL is treated identically to UP (single-slot release).
    /// This matches the host's `app/rs/src/input.rs::handle_touch`
    /// ACTION_CANCEL path, which releases the slot + the BTN keys.
    #[test]
    fn encode_touch_message_cancel_treated_as_up() {
        let mut next_tid = 1i32;
        let mut tracking = [0i32; devices::MAX_POINTERS];

        let down = touch_msg(touch_action::DOWN, 1, 10, 20, 30);
        let _ = encode_touch_message(&down, zero_timeval(), &mut next_tid, &mut tracking);

        let cancel = touch_msg(touch_action::CANCEL, 1, 0, 0, 0);
        let out = encode_touch_message(&cancel, zero_timeval(), &mut next_tid, &mut tracking);
        assert_eq!(out.len(), 5 * devices::InputEvent::size());
        assert_eq!(tracking[1], 0, "CANCEL must clear the tracking ID");
    }

    /// UP without a preceding DOWN must be silently dropped (no
    /// tracking ID to release).
    #[test]
    fn encode_touch_message_up_without_down_is_dropped() {
        let mut next_tid = 1i32;
        let mut tracking = [0i32; devices::MAX_POINTERS];
        let up = touch_msg(touch_action::UP, 0, 0, 0, 0);
        let out = encode_touch_message(&up, zero_timeval(), &mut next_tid, &mut tracking);
        assert!(out.is_empty(), "UP without DOWN must produce no bytes");
    }

    /// Out-of-range pointer_id (negative OR >= MAX_POINTERS) must be
    /// dropped silently. Without this, a malformed `TouchMessage` would
    /// panic on array indexing in `tracking_ids[slot_idx]`.
    #[test]
    fn encode_touch_message_drops_out_of_range_pointer_id() {
        let mut next_tid = 1i32;
        let mut tracking = [0i32; devices::MAX_POINTERS];

        // Negative pointer_id.
        let neg = touch_msg(touch_action::DOWN, -1, 0, 0, 0);
        assert!(
            encode_touch_message(&neg, zero_timeval(), &mut next_tid, &mut tracking).is_empty()
        );

        // pointer_id == MAX_POINTERS (one past the end).
        let over = touch_msg(touch_action::DOWN, devices::MAX_POINTERS as i32, 0, 0, 0);
        assert!(
            encode_touch_message(&over, zero_timeval(), &mut next_tid, &mut tracking).is_empty()
        );

        // pointer_id == MAX_POINTERS - 1 (last valid slot) — succeeds.
        let last = touch_msg(
            touch_action::DOWN,
            (devices::MAX_POINTERS - 1) as i32,
            0,
            0,
            0,
        );
        assert!(
            !encode_touch_message(&last, zero_timeval(), &mut next_tid, &mut tracking).is_empty()
        );
    }

    /// Unknown action values (anything outside 0..=3) must be dropped
    /// silently rather than produce garbage `InputEvent`s.
    #[test]
    fn encode_touch_message_drops_unknown_action() {
        let mut next_tid = 1i32;
        let mut tracking = [0i32; devices::MAX_POINTERS];
        let unknown = touch_msg(99, 0, 0, 0, 0);
        let out = encode_touch_message(&unknown, zero_timeval(), &mut next_tid, &mut tracking);
        assert!(out.is_empty(), "unknown action must produce no bytes");
        assert_eq!(next_tid, 1, "unknown action must not bump the counter");
        assert_eq!(
            tracking[0], 0,
            "unknown action must not assign a tracking ID"
        );
    }

    /// Full touch lifecycle (DOWN → MOVE → UP) on slot 0 must produce
    /// a stream that concatenates cleanly into 8 + 5 + 5 = 18 events
    /// AND the second DOWN (slot 0 reused) must get a NEW tracking ID
    /// (so the guest's InputReader treats it as a fresh touch, not a
    /// stale-slot state change).
    #[test]
    fn encode_touch_message_full_lifecycle_concatenates() {
        let mut next_tid = 1i32;
        let mut tracking = [0i32; devices::MAX_POINTERS];

        let mut stream = Vec::new();
        stream.extend(encode_touch_message(
            &touch_msg(touch_action::DOWN, 0, 100, 200, 128),
            zero_timeval(),
            &mut next_tid,
            &mut tracking,
        ));
        stream.extend(encode_touch_message(
            &touch_msg(touch_action::MOVE, 0, 150, 250, 200),
            zero_timeval(),
            &mut next_tid,
            &mut tracking,
        ));
        stream.extend(encode_touch_message(
            &touch_msg(touch_action::UP, 0, 0, 0, 0),
            zero_timeval(),
            &mut next_tid,
            &mut tracking,
        ));

        assert_eq!(stream.len(), 18 * devices::InputEvent::size());

        // A second DOWN on the same slot must get a fresh tracking ID
        // (the kernel requires this to distinguish two touches that
        // happen on the same slot at different times).
        let first_tid = tracking[0]; // 0 (cleared by UP)
        assert_eq!(first_tid, 0);
        let down2 = encode_touch_message(
            &touch_msg(touch_action::DOWN, 0, 50, 60, 70),
            zero_timeval(),
            &mut next_tid,
            &mut tracking,
        );
        assert_eq!(tracking[0], 2, "second DOWN must get a new tracking ID");
        assert_eq!(next_tid, 3);
        assert!(!down2.is_empty());
    }

    /// Multi-touch: two simultaneous fingers (slots 0 and 1) must
    /// have INDEPENDENT tracking IDs and INDEPENDENT slot state.
    /// This catches a regression where the slot index is ignored and
    /// both touches get collapsed to the same slot.
    #[test]
    fn encode_touch_message_multi_touch_independent_slots() {
        let mut next_tid = 1i32;
        let mut tracking = [0i32; devices::MAX_POINTERS];

        // Finger 1 down on slot 0.
        encode_touch_message(
            &touch_msg(touch_action::DOWN, 0, 100, 200, 128),
            zero_timeval(),
            &mut next_tid,
            &mut tracking,
        );
        // Finger 2 down on slot 1.
        encode_touch_message(
            &touch_msg(touch_action::DOWN, 1, 300, 400, 200),
            zero_timeval(),
            &mut next_tid,
            &mut tracking,
        );

        // Each slot has its OWN tracking ID.
        assert_eq!(tracking[0], 1, "slot 0 must have tracking ID 1");
        assert_eq!(tracking[1], 2, "slot 1 must have tracking ID 2");
        assert_eq!(next_tid, 3);

        // Releasing slot 0 must not affect slot 1.
        encode_touch_message(
            &touch_msg(touch_action::UP, 0, 0, 0, 0),
            zero_timeval(),
            &mut next_tid,
            &mut tracking,
        );
        assert_eq!(tracking[0], 0, "slot 0 released");
        assert_eq!(
            tracking[1], 2,
            "slot 1 must be unaffected by slot 0 release"
        );
    }

    /// `current_timeval` must produce a timeval with non-zero `tv_sec`
    /// when called after the UNIX epoch (i.e. always, in practice).
    /// A zero timeval would cause all events to look simultaneous,
    /// breaking the guest's InputReader frame grouping.
    #[test]
    fn current_timeval_is_nonzero() {
        let tv = current_timeval();
        // We can't assert an exact value, but tv_sec must be > 0
        // (it's been 50+ years since the UNIX epoch).
        assert!(
            tv.tv_sec > 0,
            "current_timeval tv_sec must be non-zero, got {}",
            tv.tv_sec
        );
        // tv_usec is in [0, 1_000_000).
        assert!(
            tv.tv_usec >= 0 && tv.tv_usec < 1_000_000,
            "tv_usec out of range: {}",
            tv.tv_usec
        );
    }

    /// Verify the `DeviceInfo` built by `devices::make_touch_device`
    /// (called from `spawn_touch_accept_thread`) advertises all the
    /// capabilities the guest's EventHub needs. This is an integration
    /// sanity check — the per-field details are tested in
    /// `devices::tests::make_touch_device_advertises_full_capabilities`.
    #[test]
    fn touch_dispatcher_uses_make_touch_device_with_full_capabilities() {
        let info = devices::make_touch_device(720, 1280, "/dev/input/touch");
        assert_eq!(devices::DeviceInfo::size(), 896);
        assert_eq!(&info.name[..6], b"vtouch");

        // ABS_MT_* axes advertised.
        for &axis in &[
            devices::abs::ABS_MT_SLOT,
            devices::abs::ABS_MT_TRACKING_ID,
            devices::abs::ABS_MT_POSITION_X,
            devices::abs::ABS_MT_POSITION_Y,
            devices::abs::ABS_MT_PRESSURE,
        ] {
            let byte = (axis / 8) as usize;
            let bit = axis % 8;
            assert!(
                info.abs_bitmask[byte] & (1 << bit) != 0,
                "abs_bitmask should advertise axis 0x{:x}",
                axis
            );
        }

        // BTN_TOUCH + BTN_TOOL_FINGER advertised.
        for &key in &[devices::btn::BTN_TOUCH, devices::btn::BTN_TOOL_FINGER] {
            let byte = (key / 8) as usize;
            let bit = key % 8;
            assert!(
                info.key_bitmask[byte] & (1 << bit) != 0,
                "key_bitmask should advertise key 0x{:x}",
                key
            );
        }
    }

    // ====================================================================
    // Property service stub tests (Task 6-H).
    //
    // These tests exercise the stub's wire-protocol contract with AOSP
    // 5.1 bionic's `send_prop_msg`:
    //   1. The stub creates /dev/socket/property_service (mode 0666).
    //   2. A client connects, sends 128-byte prop_msg_t, reads 4-byte
    //      PROP_SUCCESS (0) response.
    //   3. The stub is idempotent — calling it twice on the same rootfs
    //      unlinks the stale socket + rebinds.
    //
    // The tests spawn forever-threads (no shutdown signal in the stub).
    // cargo test reaps them at process exit. Each test uses a unique
    // tempdir so parallel test runs don't collide on the socket path.
    // ====================================================================

    /// Helper: create a unique temp directory under /tmp for property
    /// service tests. Avoids collisions when tests run in parallel.
    fn property_svc_tempdir(tag: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let pid = std::process::id();
        let dir =
            std::env::temp_dir().join(format!("kr64-property-svc-test-{}-{}-{}", tag, pid, nanos));
        std::fs::create_dir_all(&dir).expect("create temp dir for property svc test");
        dir.to_string_lossy().into_owned()
    }

    /// Verify that `spawn_property_service_thread` creates the
    /// `{rootfs}/dev/socket/` directory (mode 0755) so init can create
    /// + bind the property_service socket itself.
    ///
    /// Task 6-X: the function NO LONGER binds the socket (init owns it —
    /// 6-H's pre-bind caused 'Failed to unlink old socket: Permission denied'
    /// → init startup failure → exit(1)).
    #[test]
    fn spawn_property_service_thread_creates_dev_socket_dir() {
        let tmp = property_svc_tempdir("mode");
        spawn_property_service_thread(&tmp);
        let dir_path = format!("{}/dev/socket", tmp);
        let meta =
            std::fs::metadata(&dir_path).expect("dev/socket directory should exist after spawn");
        assert!(
            meta.is_dir(),
            "dev/socket should be a directory, got something else at {}",
            dir_path
        );
        // The socket FILE must NOT exist (init creates it itself).
        let socket_path = format!("{}/dev/socket/property_service", tmp);
        assert!(
            std::fs::metadata(&socket_path).is_err(),
            "property_service socket file should NOT exist (Task 6-X: init owns the socket, not kr64)"
        );
        // Directory mode 0755 (init needs to create the socket file in it).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = meta.permissions().mode();
            assert_eq!(
                mode & 0o777,
                0o755,
                "dev/socket dir should be mode 0755 (init-writable), got {:o}",
                mode & 0o777
            );
        }
    }

    /// Verify that `spawn_property_service_thread` is idempotent — calling
    /// it twice on the same rootfs (e.g. across a daemon restart) does NOT
    /// fail + still does NOT bind the socket. Task 6-X: the function only
    /// creates the directory now; idempotence is trivial (create_dir_all).
    #[test]
    fn spawn_property_service_thread_is_idempotent() {
        let tmp = property_svc_tempdir("idem");
        spawn_property_service_thread(&tmp);
        let socket_path = format!("{}/dev/socket/property_service", tmp);
        assert!(
            std::fs::metadata(&socket_path).is_err(),
            "socket file should NOT exist after the first call (Task 6-X)"
        );
        // Second call must succeed (create_dir_all is idempotent).
        spawn_property_service_thread(&tmp);
        assert!(
            std::fs::metadata(&socket_path).is_err(),
            "socket file should still NOT exist after the second call (Task 6-X)"
        );
        // Directory should still exist.
        let dir_path = format!("{}/dev/socket", tmp);
        assert!(
            std::fs::metadata(&dir_path).is_ok(),
            "dev/socket dir should still exist after the second call"
        );
    }

    /// Verify the `PROP_SERVICE_SOCKET_NAME` constant is the AOSP 5.1
    /// bionic value (`"property_service"`). Locks the contract so a
    /// future rename is caught.
    #[test]
    fn prop_service_socket_name_is_property_service() {
        assert_eq!(PROP_SERVICE_SOCKET_NAME, "property_service");
    }

    /// Verify `PROP_MSG_SIZE` is 128 bytes — sizeof(prop_msg_t) in AOSP
    /// 5.1 bionic (cmd:4 + name[32] + value[92]). Locks the contract.
    #[test]
    fn prop_msg_size_is_128_bytes() {
        assert_eq!(PROP_MSG_SIZE, 128);
        assert_eq!(
            PROP_MSG_SIZE,
            4 + 32 + 92,
            "PROP_MSG_SIZE must be sizeof(prop_msg_t) = sizeof(unsigned) + PROP_NAME_MAX + PROP_VALUE_MAX"
        );
    }

    // ====================================================================
    // Tests for `patch_property_contexts_delete` (Task 6-O; supersedes
    // 6-N's `patch_property_contexts_empty`).
    //
    // The TWRP ramdisk's /property_contexts has a C preprocessor `#line`
    // directive on line 1 — a leftover from the AOSP build process. init's
    // property_contexts parser doesn't understand the directive and crashes
    // (garbage ptr 0x74616433 = ASCII "3dat"). 6-L's fix (removing just the
    // #line directive) was insufficient: the parser's context field at
    // offset 0x14 stays corrupted, so 6-M NOPed the crash instruction at
    // 0x80a0b9e, but the crash MOVED to 0x80a0bd8 (DISPATCHER-FINAL-7:
    // whack-a-mole — MULTIPLE instructions deref the garbage edx/ecx from
    // ctx->field_0x14). 6-N then EMPTIED the file to a single comment line,
    // BUT DISPATCHER-FINAL-8 showed the crash PERSISTS at 0x80a0bd8 even
    // with the emptied file: the context field 0x14 is corrupted BEFORE
    // the parser reads the file content, so even a comment line still
    // triggers fgets → the parser processes it → tries to read
    // ctx->field_0x14->field_at_4 → SIGSEGV. The sustainable fix is a DATA
    // fix: DELETE the file ENTIRELY. init's open() returns -ENOENT → the
    // caller skips this context file → the parser is never invoked → no
    // corrupted-context crash. These tests verify the patcher DELETES the
    // file, is idempotent (a no-op when the file is already missing), and
    // handles missing/edge-case content gracefully.
    // ====================================================================

    /// Helper: build a temp "rootfs" directory for property_contexts tests.
    fn make_property_contexts_temp_rootfs() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "twoyi-kr64-test-propctx-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The patcher must DELETE the entire /property_contexts file. This is
    /// the EXACT file shape shipped in the TWRP ramdisk (verified by Step 1
    /// of Task 6-L against the extracted ramdisk at
    /// /tmp/twrp-ramdisk-extract/property_contexts): the `#line` directive
    /// on line 1, followed by the real property-context entries. After
    /// the patch runs, the file MUST NOT EXIST (so init's open() returns
    /// -ENOENT → the caller skips this context file → parser never invoked).
    #[test]
    fn property_contexts_patcher_deletes_file() {
        let dir = make_property_contexts_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        // The exact file content shipped in the TWRP ramdisk: the `#line`
        // directive on line 1, followed by the real property-context entries.
        let original = "#line 1 \"external/sepolicy/property_contexts\"\n\
                        ##########################\n\
                        # property service keys\n\
                        net.rmnet               u:object_r:net_radio_prop:s0\n";
        std::fs::write(dir.join("property_contexts"), original).unwrap();
        assert!(dir.join("property_contexts").exists());
        patch_property_contexts_delete(&rootfs);
        // The file MUST be GONE — this is the whole point of the patch.
        // If the file is absent, init's open() returns -ENOENT → the caller
        // skips this context file → the parser is never invoked → no crash.
        assert!(
            !dir.join("property_contexts").exists(),
            "file should be DELETED after patch runs. Still exists at: {:?}",
            dir.join("property_contexts")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// IDEMPOTENCE: running the patcher twice must produce the same result
    /// as running it once. The second run must be a no-op because the first
    /// run already deleted the file (the file is already absent → the
    /// existence check returns false → log + return).
    #[test]
    fn property_contexts_patcher_is_idempotent() {
        let dir = make_property_contexts_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        let original = "#line 1 \"external/sepolicy/property_contexts\"\n\
                        ##########################\n\
                        net.rmnet               u:object_r:net_radio_prop:s0\n";
        std::fs::write(dir.join("property_contexts"), original).unwrap();
        patch_property_contexts_delete(&rootfs);
        let after_first_exists = dir.join("property_contexts").exists();
        patch_property_contexts_delete(&rootfs);
        let after_second_exists = dir.join("property_contexts").exists();
        assert!(!after_first_exists, "file should be absent after first run");
        assert!(
            !after_second_exists,
            "file should STILL be absent after second run (idempotent — already-missing is a no-op)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The patcher must be a no-op when the file is ALREADY MISSING (e.g.
    /// a previous boot already deleted it). This is the idempotent-skip
    /// path for re-runs: existence check returns false → log + return
    /// without attempting remove_file (which would error on a missing
    /// file).
    #[test]
    fn property_contexts_patcher_skips_when_already_absent() {
        let dir = make_property_contexts_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        // Don't create the file — patcher must not panic and must not
        // create it. The caller will get -ENOENT on open either way.
        patch_property_contexts_delete(&rootfs);
        assert!(
            !dir.join("property_contexts").exists(),
            "already-missing file should remain missing (idempotent skip — no spurious creation)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The patcher must handle a missing /property_contexts gracefully —
    /// log + return without panicking AND without creating the file. Some
    /// TWRP variants or full-Android boots don't ship this file; if init's
    /// open() fails, the parser isn't invoked at all (also a clean exit).
    /// This is identical in behavior to the "already-absent" idempotent
    /// skip above, but explicitly tests the missing-file-as-no-op contract.
    #[test]
    fn property_contexts_patcher_handles_missing_file_gracefully() {
        let dir = make_property_contexts_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        // Don't create the file — patcher must not panic.
        patch_property_contexts_delete(&rootfs);
        // File should still not exist (we deliberately don't create it —
        // if init's open() fails, the parser isn't invoked at all).
        assert!(
            !dir.join("property_contexts").exists(),
            "missing file should remain missing (no spurious creation)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Edge case: a file containing ONLY the `#line` directive (no newline,
    /// no body). The patcher must DELETE it (not panic on the edge-case
    /// content) — the file's content shape doesn't matter for the delete
    /// path, only its existence.
    #[test]
    fn property_contexts_patcher_deletes_file_with_only_line_directive_no_newline() {
        let dir = make_property_contexts_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        // A file with only the directive and NO trailing newline.
        let original = "#line 1 \"external/sepolicy/property_contexts\"";
        std::fs::write(dir.join("property_contexts"), original).unwrap();
        patch_property_contexts_delete(&rootfs);
        assert!(
            !dir.join("property_contexts").exists(),
            "file with only the #line directive should be DELETED. Still exists at: {:?}",
            dir.join("property_contexts")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Edge case: a file with the `#line` directive followed by a newline
    /// but no body (empty rest). The patcher must DELETE it (no special
    /// handling needed for the empty-body edge case).
    #[test]
    fn property_contexts_patcher_deletes_file_with_directive_and_empty_body() {
        let dir = make_property_contexts_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        let original = "#line 1 \"external/sepolicy/property_contexts\"\n";
        std::fs::write(dir.join("property_contexts"), original).unwrap();
        patch_property_contexts_delete(&rootfs);
        assert!(
            !dir.join("property_contexts").exists(),
            "file with directive + empty body should be DELETED. Still exists at: {:?}",
            dir.join("property_contexts")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Robustness: even a file WITHOUT the `#line` directive (e.g. a future
    /// TWRP version that removed it, or arbitrary content) must be DELETED.
    /// This is the KEY behavioral contract from 6-N (which ALWAYS emptied
    /// non-already-emptied files) carried into 6-O: the parser's context
    /// field at offset 0x14 is corrupted regardless of whether the #line
    /// directive is present (DISPATCHER-FINAL-7), so the file MUST be
    /// deleted regardless of its content.
    #[test]
    fn property_contexts_patcher_deletes_file_without_line_directive() {
        let dir = make_property_contexts_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        // A file without the #line directive (e.g. a future TWRP version
        // that removed it). 6-O must STILL delete it — the parser's context
        // field at offset 0x14 is corrupted regardless of the directive.
        let content = "##########################\n\
                        # property service keys\n\
                        net.rmnet               u:object_r:net_radio_prop:s0\n";
        std::fs::write(dir.join("property_contexts"), content).unwrap();
        patch_property_contexts_delete(&rootfs);
        assert!(
            !dir.join("property_contexts").exists(),
            "file without #line directive should STILL be DELETED (6-O behavior). Still exists at: {:?}",
            dir.join("property_contexts")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Non-UTF8 content robustness: the patcher reads the file for
    /// diagnostic logging only (size + first line), but read_to_string
    /// fails on non-UTF8 bytes. The patcher must STILL delete the file in
    /// that case (remove_file works on bytes, not strings). This guards
    /// against a hypothetical TWRP variant whose /property_contexts is
    /// mis-encoded.
    #[test]
    fn property_contexts_patcher_deletes_non_utf8_file() {
        let dir = make_property_contexts_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        // Non-UTF8 bytes (invalid UTF-8 sequence 0xFF 0xFE 0xFD).
        let original: [u8; 3] = [0xFF, 0xFE, 0xFD];
        std::fs::write(dir.join("property_contexts"), original).unwrap();
        // Must not panic on read_to_string failing.
        patch_property_contexts_delete(&rootfs);
        assert!(
            !dir.join("property_contexts").exists(),
            "non-UTF8 file should be DELETED (remove_file works on bytes, not strings). Still exists at: {:?}",
            dir.join("property_contexts")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ──────────────────────────────────────────────────────────────────
    // Tests for `precreate_sysfs_stubs` (Task 6-P).
    //
    // Verifies the fake sysfs is materialised in the rootfs with the
    // expected paths + modes + (for `enforce`) the permissive "0" seed
    // content. The companion `translate_path` tests in ptrace_emu.rs
    // verify that `/sys/*` opens are redirected to `{rootfs}/sys/*` —
    // together they ensure init's `open("/sys/class")` +
    // `open("/sys/fs/selinux/{enforce,load}")` succeed instead of
    // returning -EACCES (the iter-3059 exit(1) blocker).
    // ──────────────────────────────────────────────────────────────────

    fn make_sysfs_temp_rootfs() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "twoyi-kr64-test-sysfs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn precreate_sysfs_stubs_creates_all_expected_paths() {
        let dir = make_sysfs_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        precreate_sysfs_stubs(&rootfs);
        // Directories that init expects to exist + readdir.
        for rel in &["sys", "sys/class", "sys/fs", "sys/fs/selinux"] {
            let p = dir.join(rel);
            assert!(
                p.is_dir(),
                "expected {} to be a directory after precreate_sysfs_stubs",
                p.display()
            );
        }
        // Empty files init opens (SELinux sysfs). 6-Z154: `null` joins the
        // set — arm64 TWRP init's logging sink (see precreate_sysfs_stubs).
        for rel in &[
            "sys/fs/selinux/enforce",
            "sys/fs/selinux/load",
            "sys/fs/selinux/null",
        ] {
            let p = dir.join(rel);
            assert!(
                p.is_file(),
                "expected {} to be a regular file after precreate_sysfs_stubs",
                p.display()
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn precreate_sysfs_stubs_seeds_enforce_with_zero() {
        let dir = make_sysfs_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        precreate_sysfs_stubs(&rootfs);
        // `enforce` is seeded with "0" (permissive) — init's read() returns
        // that content + treats SELinux as off. Safe default for TWRP in
        // the sandbox.
        let enforce = dir.join("sys/fs/selinux/enforce");
        let content = std::fs::read_to_string(&enforce).unwrap();
        assert_eq!(
            content,
            "0",
            "enforce should be seeded with '0' (permissive). Got: {:?} at {}",
            content,
            enforce.display()
        );
        // `load` is empty — init writes its own policy blob to it (the
        // write succeeds silently against a regular empty file).
        let load = dir.join("sys/fs/selinux/load");
        let load_content = std::fs::read_to_string(&load).unwrap();
        assert_eq!(
            load_content,
            "",
            "load should be empty (init writes its own policy blob). Got: {:?} at {}",
            load_content,
            load.display()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn precreate_sysfs_stubs_is_idempotent() {
        let dir = make_sysfs_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        // First call materialises the fake sysfs.
        precreate_sysfs_stubs(&rootfs);
        // Capture the enforce content after the first call (should be "0").
        let enforce = dir.join("sys/fs/selinux/enforce");
        let after_first = std::fs::read_to_string(&enforce).unwrap();
        assert_eq!(after_first, "0");
        // Write something else to `load` (simulating init writing a policy
        // blob on a prior boot). Idempotent pre-creation must NOT clobber
        // it — only the FIRST pre-creation writes the (empty) seed.
        let load = dir.join("sys/fs/selinux/load");
        std::fs::write(&load, b"<fake-policy-blob-from-prior-boot>").unwrap();
        // Second call should be a no-op on the existing files (no truncation,
        // no re-seed of enforce).
        precreate_sysfs_stubs(&rootfs);
        let after_second_enforce = std::fs::read_to_string(&enforce).unwrap();
        assert_eq!(
            after_second_enforce, "0",
            "idempotent pre-creation must NOT re-seed enforce (it was already created with '0')"
        );
        let after_second_load = std::fs::read_to_string(&load).unwrap();
        assert_eq!(
            after_second_load, "<fake-policy-blob-from-prior-boot>",
            "idempotent pre-creation must NOT truncate an existing load file (init may have written a policy blob on a prior boot)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn precreate_sysfs_stubs_sets_modes_correctly() {
        use std::os::unix::fs::PermissionsExt;
        let dir = make_sysfs_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        precreate_sysfs_stubs(&rootfs);
        // Dirs are 0755 (rwxr-xr-x — init can read/exec, only owner can write).
        for rel in &["sys", "sys/class", "sys/fs", "sys/fs/selinux"] {
            let p = dir.join(rel);
            let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
            assert_eq!(
                mode,
                0o755,
                "directory {} should be mode 0755, got {:o}",
                p.display(),
                mode
            );
        }
        // Files are 0666 (rw-rw-rw- — init can read + write, regardless of UID).
        for rel in &["sys/fs/selinux/enforce", "sys/fs/selinux/load"] {
            let p = dir.join(rel);
            let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
            assert_eq!(
                mode, 0o666,
                "file {} should be mode 0666 (init may run as different UID under TWRP's recovery policy), got {:o}",
                p.display(),
                mode
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn precreate_sysfs_stubs_creates_sys_root_when_missing() {
        // If {rootfs}/sys does NOT exist at all (fresh extraction, no prior
        // pre-creation), precreate_sysfs_stubs must create the whole tree
        // (create_dir_all handles the recursive creation).
        let dir = make_sysfs_temp_rootfs();
        let rootfs = dir.to_string_lossy().into_owned();
        // Sanity: no sys/ at all yet.
        assert!(!dir.join("sys").exists());
        precreate_sysfs_stubs(&rootfs);
        assert!(dir.join("sys").is_dir());
        assert!(dir.join("sys/class").is_dir());
        assert!(dir.join("sys/fs/selinux/enforce").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }
    // ============================================================
    // 6-Z215: ELF-machine helpers + native-guest detection tests.
    // ============================================================

    /// Build a minimal ELF header image with the given EI_CLASS (1=32,
    /// 2=64), EI_DATA (1=LE, 2=BE) and e_machine value. The header is
    /// long enough for offset-18/19 reads in both classes.
    fn make_elf_hdr(class_: u8, data: u8, machine: u16) -> Vec<u8> {
        let mut v = vec![0u8; 64];
        v[0] = 0x7f;
        v[1] = b'E';
        v[2] = b'L';
        v[3] = b'F';
        v[4] = class_;
        v[5] = data;
        let raw = if data == 1 {
            machine.to_le_bytes()
        } else {
            machine.to_be_bytes()
        };
        v[18] = raw[0];
        v[19] = raw[1];
        v
    }

    #[test]
    fn elf_machine_from_bytes_parses_all_android_machines() {
        // ELF64 LE (the only layout Android ships in practice).
        assert_eq!(
            elf_machine_from_bytes(&make_elf_hdr(2, 1, EM_AARCH64)),
            Some(EM_AARCH64)
        );
        assert_eq!(
            elf_machine_from_bytes(&make_elf_hdr(2, 1, EM_X86_64)),
            Some(EM_X86_64)
        );
        // ELF32 LE (legacy 32-bit TWRP images).
        assert_eq!(
            elf_machine_from_bytes(&make_elf_hdr(1, 1, EM_ARM)),
            Some(EM_ARM)
        );
        assert_eq!(
            elf_machine_from_bytes(&make_elf_hdr(1, 1, EM_386)),
            Some(EM_386)
        );
        // Big-endian ELF32 (exotic but must not misparse).
        assert_eq!(
            elf_machine_from_bytes(&make_elf_hdr(1, 2, EM_ARM)),
            Some(EM_ARM)
        );
    }

    #[test]
    fn elf_machine_from_bytes_rejects_non_elf_and_short_input() {
        assert_eq!(elf_machine_from_bytes(&[]), None);
        assert_eq!(elf_machine_from_bytes(&[0u8; 19]), None);
        // 20 bytes but wrong magic.
        let mut not_elf = vec![0u8; 64];
        not_elf[0] = b'P';
        not_elf[1] = b'K';
        assert_eq!(elf_machine_from_bytes(&not_elf), None);
        // Valid magic but invalid EI_DATA.
        let mut bad_data = make_elf_hdr(2, 1, EM_AARCH64);
        bad_data[5] = 9;
        assert_eq!(elf_machine_from_bytes(&bad_data), None);
    }

    #[test]
    fn elf_machine_file_missing_returns_none() {
        assert_eq!(elf_machine("/nonexistent/twoyi-test/nope.so"), None);
    }

    // ── 6-Z230: DT_NEEDED parser tests ──────────────────────────────

    /// Build a minimal but structurally-valid ELF64 LE image with a
    /// PT_DYNAMIC segment whose DT_NEEDED entries point into a DT_STRTAB.
    fn make_elf64_with_dt_needed(names: &[&str]) -> Vec<u8> {
        // Layout: [ehdr 64][phdr2 2*56][pad][dyn entries][strtab]
        let mut img: Vec<u8> = Vec::new();
        // --- ELF header ---
        img.extend_from_slice(b"\x7fELF");
        img.push(2); // ELFCLASS64
        img.push(1); // little-endian
        img.push(1); // EV_CURRENT
        img.push(0); // ELFOSABI_NONE
        img.extend_from_slice(&[0u8; 8]); // padding
        img.extend_from_slice(&3u16.to_le_bytes()); // e_type = ET_DYN
        img.extend_from_slice(&183u16.to_le_bytes()); // e_machine = EM_AARCH64
        img.extend_from_slice(&1u32.to_le_bytes()); // e_version
        img.extend_from_slice(&0u64.to_le_bytes()); // e_entry
        let phoff = 64u64;
        img.extend_from_slice(&phoff.to_le_bytes()); // e_phoff
        img.extend_from_slice(&0u64.to_le_bytes()); // e_shoff
        img.extend_from_slice(&0u32.to_le_bytes()); // e_flags
        img.extend_from_slice(&64u16.to_le_bytes()); // e_ehsize
        img.extend_from_slice(&56u16.to_le_bytes()); // e_phentsize
        img.extend_from_slice(&2u16.to_le_bytes()); // e_phnum
        img.extend_from_slice(&64u16.to_le_bytes()); // e_shentsize
        img.extend_from_slice(&0u16.to_le_bytes()); // e_shnum
        img.extend_from_slice(&0u16.to_le_bytes()); // e_shstrndx
        assert_eq!(img.len(), 64);
        // --- Program headers: PT_LOAD + PT_DYNAMIC ---
        // dyn must start AFTER both phdrs (64 + 2*56 = 176):
        let dyn_vaddr: u64 = 176; // identity-mapped (vaddr == file offset)
        let dyn_off: u64 = 176;
        // strtab offset == dyn_off + dyn entries bytes (computed below);
        // strtab VADDR is identity-mapped to the same value so the
        // parser's PT_LOAD vaddr->offset walk resolves it.
        let n_entries = names.len() as u64 + 2; // needed*N + strtab + null
        let strtab_off: u64 = dyn_off + n_entries * 16;
        let strtab_vaddr: u64 = strtab_off;
        // PT_LOAD: [0, strtab_off+strsz) — covers everything.
        // strtab[0] is the mandatory empty string (real-ELF convention —
        // that's WHY DT_NEEDED offsets are never 0); names start at 1.
        let strsz: u64 = 1 + names.iter().map(|n| n.len() as u64 + 1).sum::<u64>();
        let load_filesz = strtab_off + strsz;
        // p_type=1 (PT_LOAD), p_flags, p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, align
        img.extend_from_slice(&1u32.to_le_bytes());
        img.extend_from_slice(&5u32.to_le_bytes());
        img.extend_from_slice(&0u64.to_le_bytes()); // offset 0
        img.extend_from_slice(&0u64.to_le_bytes()); // vaddr 0
        img.extend_from_slice(&0u64.to_le_bytes()); // paddr
        img.extend_from_slice(&load_filesz.to_le_bytes());
        img.extend_from_slice(&load_filesz.to_le_bytes());
        img.extend_from_slice(&0x1000u64.to_le_bytes());
        // PT_DYNAMIC: p_type=2, offset=dyn_off, vaddr=dyn_vaddr, filesz=n_entries*16
        img.extend_from_slice(&2u32.to_le_bytes());
        img.extend_from_slice(&6u32.to_le_bytes());
        img.extend_from_slice(&dyn_off.to_le_bytes());
        img.extend_from_slice(&dyn_vaddr.to_le_bytes());
        img.extend_from_slice(&dyn_vaddr.to_le_bytes());
        img.extend_from_slice(&(n_entries * 16).to_le_bytes());
        img.extend_from_slice(&(n_entries * 16).to_le_bytes());
        img.extend_from_slice(&8u64.to_le_bytes());
        assert_eq!(img.len(), 64 + 112);
        // pad to dyn_off
        img.resize(dyn_off as usize, 0);
        // --- dynamic entries (each entry = tag u64 + val u64) ---
        let mut dyn_img: Vec<u8> = Vec::new();
        let mut cur = 1u64; // strtab[0] = the mandatory empty string
        for n in names {
            dyn_img.extend_from_slice(&1u64.to_le_bytes()); // DT_NEEDED tag
            dyn_img.extend_from_slice(&cur.to_le_bytes()); // strtab offset
            cur += n.len() as u64 + 1;
        }
        dyn_img.extend_from_slice(&5u64.to_le_bytes()); // DT_STRTAB tag
        dyn_img.extend_from_slice(&strtab_vaddr.to_le_bytes());
        dyn_img.extend_from_slice(&0u64.to_le_bytes()); // DT_NULL tag
        dyn_img.extend_from_slice(&0u64.to_le_bytes()); // DT_NULL val
        assert_eq!(dyn_img.len() as u64, n_entries * 16);
        img.extend_from_slice(&dyn_img);
        assert_eq!(img.len(), strtab_off as usize);
        // --- strtab (offset 0 = the mandatory empty string) ---
        img.push(0);
        for n in names {
            img.extend_from_slice(n.as_bytes());
            img.push(0);
        }
        img
    }

    #[test]
    fn dt_needed_parser_6z230_extracts_names() {
        let img = make_elf64_with_dt_needed(&["libcrypto.so", "libssl.so", "libc.so"]);
        let names = dt_needed_names_from_bytes(&img).expect("parse");
        assert_eq!(names, vec!["libcrypto.so", "libssl.so", "libc.so"]);
    }

    #[test]
    fn dt_needed_parser_6z230_empty_needed_ok() {
        // A PT_DYNAMIC with ONLY DT_STRTAB + DT_NULL → Some(vec![]).
        let img = make_elf64_with_dt_needed(&[]);
        assert_eq!(dt_needed_names_from_bytes(&img), Some(vec![]));
    }

    #[test]
    fn dt_needed_parser_6z230_rejects_non_elf_and_truncated() {
        assert_eq!(dt_needed_names_from_bytes(b"not an elf at all....."), None);
        assert_eq!(dt_needed_names_from_bytes(&[0u8; 16]), None);
        // ELF magic but no PT_DYNAMIC (phnum=0):
        let mut img = vec![0u8; 64];
        img[0..4].copy_from_slice(b"\x7fELF");
        img[4] = 2;
        img[5] = 1;
        assert_eq!(dt_needed_names_from_bytes(&img), None);
    }

    #[test]
    fn dt_needed_parser_6z230_real_artifact_binaries() {
        // Real-artifact check: our own NDK-built test libs (written by
        // the 6-Z227 local build). If they exist, parse them — the v7a
        // shlib links -lc -ldl (2 DT_NEEDED), the fb_hook is nostdlib
        // (0 or None). Tolerate absence (artifact envs without /tmp).
        for (path, expect_dynamic) in [
            ("/tmp/shlib_v7a.so", true),
            ("/tmp/shlib_a64.so", true),
            ("/tmp/fbhook_v7a.so", false),
        ] {
            if let Ok(data) = std::fs::read(path) {
                let names = dt_needed_names_from_bytes(&data);
                if expect_dynamic {
                    let names = names.expect("dynamic ELF must parse");
                    assert!(
                        names.iter().any(|n| n == "libc.so"),
                        "{} should DT_NEEDED libc.so, got {:?}",
                        path,
                        names
                    );
                } else {
                    assert!(
                        names.map_or(true, |n| n.is_empty()),
                        "nostdlib fb_hook should have no DT_NEEDED"
                    );
                }
            }
        }
    }

    #[test]
    fn z236_compat_shim_source_name_per_machine() {
        // Pure decision core: the shim source file must match the guest's
        // ABI exactly (the 6-Z226 wrong-arch class must not reappear via
        // the shim), and unknown machines must NEVER guess (§9).
        assert_eq!(
            z236_compat_shim_source_name(40),
            Some("libbionic_compat_arm32.so")
        );
        assert_eq!(
            z236_compat_shim_source_name(183),
            Some("libbionic_compat.so")
        );
        assert_eq!(
            z236_compat_shim_source_name(62),
            Some("libbionic_compat.so")
        );
        assert_eq!(
            z236_compat_shim_source_name(3),
            Some("libbionic_compat_i686.so")
        );
        assert_eq!(z236_compat_shim_source_name(0), None);
        assert_eq!(z236_compat_shim_source_name(8), None); // MIPS — not served
        assert_eq!(z236_compat_shim_source_name(243), None); // RISCV — not served
    }

    #[test]
    fn z236_stage_missing_dt_needed_returns_host_flag_on_no_guest() {
        // An empty temp rootfs: no recovery binary + no sbin libc → the
        // ABI anchor is unknown → (0, [], false) without any staging.
        let dir = std::env::temp_dir().join(format!("twoyi-6z236-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sbin")).unwrap();
        let (staged, missing, from_host) = stage_missing_dt_needed(dir.to_str().unwrap());
        assert_eq!(staged, 0);
        assert!(missing.is_empty());
        assert!(!from_host);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn guest_bionic_is_native_false_when_guest_libc_missing() {
        // An empty temp dir: no {rootfs}/system/lib64/libc.so → the
        // detection MUST return false (conservative: keep the 6-Z93
        // runner-mode behavior).
        let dir = std::env::temp_dir().join(format!("twoyi-6z215-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!guest_bionic_is_native(dir.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn guest_bionic_is_native_false_when_guest_libc_not_elf() {
        // A text file at the libc path (corrupt import) must NOT trip
        // the native detection.
        let dir = std::env::temp_dir().join(format!("twoyi-6z215-text-{}", std::process::id()));
        let libc_dir = dir.join("system/lib64");
        std::fs::create_dir_all(&libc_dir).unwrap();
        std::fs::write(libc_dir.join("libc.so"), b"definitely not an elf").unwrap();
        assert!(!guest_bionic_is_native(dir.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn guest_bionic_is_native_matches_runner_and_native_modes() {
        // Host bionic machine as resolved in THIS test environment
        // (the CI/test host has no Android libc, so this falls back to
        // the compile-time arch — deterministic on any runner).
        let host_m = host_bionic_machine();
        assert_ne!(host_m, 0, "host machine must resolve on any runner");

        // Case 1: guest libc with the SAME machine as the host → native
        // mode (arm64 guest on arm64 host / x86_64 on x86_64).
        let dir = std::env::temp_dir().join(format!("twoyi-6z215-same-{}", std::process::id()));
        let libc_dir = dir.join("system/lib64");
        std::fs::create_dir_all(&libc_dir).unwrap();
        std::fs::write(libc_dir.join("libc.so"), make_elf_hdr(2, 1, host_m)).unwrap();
        assert!(guest_bionic_is_native(dir.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&dir);

        // Case 2: guest libc with a DIFFERENT machine than the host →
        // binfmt runner mode (x86_64 host + arm64 ROM) → NOT native,
        // 6-Z93 filter stays in force.
        let other_m = if host_m == EM_AARCH64 {
            EM_X86_64
        } else {
            EM_AARCH64
        };
        let dir = std::env::temp_dir().join(format!("twoyi-6z215-diff-{}", std::process::id()));
        let libc_dir = dir.join("system/lib64");
        std::fs::create_dir_all(&libc_dir).unwrap();
        std::fs::write(libc_dir.join("libc.so"), make_elf_hdr(2, 1, other_m)).unwrap();
        assert!(!guest_bionic_is_native(dir.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // -----------------------------------------------------------------
    // 6-Z218b: bootstrap-bionic staging tests
    // -----------------------------------------------------------------

    #[test]
    fn aosp_preload_order_fb_hook_before_shlib() {
        // 6-Z218a regression guard: the FB hook must precede the shlib
        // or libminuitwrp's open/ioctl PLT entries resolve to the
        // shlib and framebuffer virtualization never fires.
        assert_aosp_preload_order();
    }

    // ── 6-Z219: detect_guest_slot_suffix ────────────────────────────────

    /// Writes `content` to `{root}/{sub}` creating parents as needed.
    fn write_fstab(root: &std::path::Path, sub: &str, content: &str) {
        let p = root.join(sub);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    // ── 6-Z225: capabilities-option stripping ────────────────────────

    // ── 6-Z226: guest bitness detection + hook name selection ─────────

    #[test]
    fn guest_hook_lib_name_maps_all_three_hooks_6z226() {
        // 64-bit guests keep the canonical names.
        assert_eq!(
            guest_hook_lib_name("libtwrp_fb_hook.so", true),
            "libtwrp_fb_hook.so"
        );
        assert_eq!(
            guest_hook_lib_name("libgetpid_hook.so", true),
            "libgetpid_hook.so"
        );
        assert_eq!(
            guest_hook_lib_name("libtwoyi_loader_shlib.so", true),
            "libtwoyi_loader_shlib.so"
        );
        // 32-bit guests get the _arm32 asset names (built by
        // app/cpp/build.sh's armeabi-v7a section; extracted by RomManager
        // to {data_dir}/files/).
        assert_eq!(
            guest_hook_lib_name("libtwrp_fb_hook.so", false),
            "libtwrp_fb_hook_arm32.so"
        );
        assert_eq!(
            guest_hook_lib_name("libgetpid_hook.so", false),
            "libgetpid_hook_arm32.so"
        );
        assert_eq!(
            guest_hook_lib_name("libtwoyi_loader_shlib.so", false),
            "libtwoyi_loader_shlib_arm32.so"
        );
        // Generic fallback inserts _arm32 before the extension.
        assert_eq!(guest_hook_lib_name("other.so", false), "other_arm32.so");
    }

    #[test]
    fn detect_guest_recovery_bitness_reads_elf_class_6z226() {
        let dir = std::env::temp_dir().join(format!("twoyi-6z226-elf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sbin")).unwrap();
        // 32-bit ARM recovery: ELF magic + EI_CLASS=1.
        let mut elf32 = b"\x7fELF".to_vec();
        elf32.push(1);
        elf32.extend_from_slice(&[1, 1, 0]);
        std::fs::write(dir.join("sbin/recovery"), &elf32).unwrap();
        assert_eq!(
            detect_guest_recovery_bitness(dir.to_str().unwrap()),
            Some(false)
        );
        // 64-bit: EI_CLASS=2 (system/bin/recovery candidate path).
        let dir64 = std::env::temp_dir().join(format!("twoyi-6z226-elf64-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir64);
        std::fs::create_dir_all(dir64.join("system/bin")).unwrap();
        let mut elf64 = b"\x7fELF".to_vec();
        elf64.push(2);
        elf64.extend_from_slice(&[1, 1, 0]);
        std::fs::write(dir64.join("system/bin/recovery"), &elf64).unwrap();
        assert_eq!(
            detect_guest_recovery_bitness(dir64.to_str().unwrap()),
            Some(true)
        );
        // Missing binary -> None (caller defaults to the 64-bit chain).
        let empty = std::env::temp_dir().join(format!("twoyi-6z226-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&empty);
        std::fs::create_dir_all(&empty).unwrap();
        assert_eq!(detect_guest_recovery_bitness(empty.to_str().unwrap()), None);
        // Non-ELF file -> skipped, falls through to None.
        std::fs::write(dir.join("sbin/recovery"), b"#!/system/bin/sh\n").unwrap();
        assert_eq!(detect_guest_recovery_bitness(dir.to_str().unwrap()), None);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir64);
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn strip_caps_options_strips_all_spellings_and_keeps_rest_6z225() {
        let dir = std::env::temp_dir().join(format!("twoyi-6z225-strip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // logd.rc — the exact OrangeFox init.recovery.logd.rc shape
        // (keyword + space + caps, indented 4) that FATAL'd the guest.
        std::fs::create_dir_all(dir.join("init.recovery.logd.rc.parent")).unwrap();
        std::fs::write(
            dir.join("init.recovery.logd.rc"),
            "service logd /system/bin/logd\n    class core\n    socket logd stream 0666 logd logd\n    capabilities SYSLOG AUDIT_CONTROL SETGID SETUID\n\nservice other /system/bin/other\n    class late_start\n    capabilities: WAKE_ALARM\n    seclabel u:r:logd:s0\n",
        )
        .unwrap();
        // An .rc file WITHOUT the option must not be rewritten (mtime
        // content check) and non-.rc files must be ignored.
        std::fs::write(
            dir.join("init.plain.rc"),
            "service a /bin/a\n    class core\n",
        )
        .unwrap();
        std::fs::write(dir.join("capabilities.txt"), "capabilities FAKE\n").unwrap();
        // A comment line mentioning capabilities must be KEPT.
        std::fs::write(
            dir.join("init.comment.rc"),
            "# capabilities are declared per service\nservice b /bin/b\n",
        )
        .unwrap();

        let n = strip_service_capabilities_options(dir.to_str().unwrap());
        assert_eq!(n, 1, "only the logd rc has the option");

        let logd = std::fs::read_to_string(dir.join("init.recovery.logd.rc")).unwrap();
        assert!(!logd.contains("capabilities"), "both spellings stripped");
        assert!(
            logd.contains("socket logd stream 0666 logd logd"),
            "kept lines intact"
        );
        assert!(
            logd.contains("seclabel u:r:logd:s0"),
            "later options intact"
        );
        assert!(logd.contains("class late_start"), "other service intact");
        // Comment lines mentioning the keyword survive (trim_start only
        // matches the KEYWORD at line start).
        let comment = std::fs::read_to_string(dir.join("init.comment.rc")).unwrap();
        assert!(comment.contains("# capabilities are declared per service"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn strip_caps_options_is_idempotent_and_tolerates_missing_dirs_6z225() {
        let dir = std::env::temp_dir().join(format!("twoyi-6z225-idem-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("vendor/etc/init")).unwrap();
        std::fs::write(
            dir.join("vendor/etc/init/svc.rc"),
            "service hwservicemanager /system/bin/hwservicemanager\n    class core\n    capabilities WAKE_ALARM\n",
        )
        .unwrap();
        let first = strip_service_capabilities_options(dir.to_str().unwrap());
        assert_eq!(first, 1);
        assert!(!std::fs::read_to_string(dir.join("vendor/etc/init/svc.rc"))
            .unwrap()
            .contains("capabilities"));
        // Second pass: nothing left to strip.
        let second = strip_service_capabilities_options(dir.to_str().unwrap());
        assert_eq!(second, 0, "idempotent");
        // Missing rootfs is safe.
        assert_eq!(
            strip_service_capabilities_options(&format!(
                "{}/does-not-exist-6z225",
                dir.to_str().unwrap()
            )),
            0
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_slot_suffix_recovery_fstab_slotselect() {
        // lineage-22.2-sailfish layout: /etc/recovery.fstab (materialized
        // from the etc→/system/etc symlink) with slotselect entries.
        let dir = std::env::temp_dir().join(format!("twoyi-6z219-etc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_fstab(
            &dir,
            "etc/recovery.fstab",
            "# comment\n/dev/block/by-name/system /system ext4 ro,barrier=1 wait,slotselect,verify,first_stage_mount\n/dev/block/by-name/userdata /data ext4 errors=panic wait\n",
        );
        assert_eq!(detect_guest_slot_suffix(dir.to_str().unwrap()), "_a");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_slot_suffix_first_stage_ramdisk_fstab() {
        let dir = std::env::temp_dir().join(format!("twoyi-6z219-fsr-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_fstab(
            &dir,
            "first_stage_ramdisk/fstab.sailfish",
            "/dev/block/by-name/vendor /vendor ext4 ro wait,slotselect,first_stage_mount\n",
        );
        assert_eq!(detect_guest_slot_suffix(dir.to_str().unwrap()), "_a");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_slot_suffix_comment_or_aonly_is_empty() {
        let dir = std::env::temp_dir().join(format!("twoyi-6z219-aonly-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Only a COMMENT mentions slotselect — must NOT count.
        write_fstab(
            &dir,
            "etc/recovery.fstab",
            "# wait,slotselect,verify\n/dev/block/mmcblk0p1 /system ext4 ro wait,verify\n",
        );
        assert_eq!(detect_guest_slot_suffix(dir.to_str().unwrap()), "");
        // A/B comment token must also not match as a bare substring of a
        // longer flag (e.g. "slotselectfoo" is not slotselect).
        write_fstab(
            &dir,
            "etc/recovery.fstab",
            "/dev/x /mnt ext4 ro wait,slotselectfoo\n",
        );
        assert_eq!(detect_guest_slot_suffix(dir.to_str().unwrap()), "");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_slot_suffix_no_fstab_and_missing_rootfs() {
        let dir = std::env::temp_dir().join(format!("twoyi-6z219-none-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(detect_guest_slot_suffix(dir.to_str().unwrap()), "");
        let _ = std::fs::remove_dir_all(&dir);
        // Entirely missing rootfs must be safe (x86 ranchu pre-rootfs call).
        assert_eq!(
            detect_guest_slot_suffix(&format!("{}/does-not-exist-6z219", dir.to_str().unwrap())),
            ""
        );
    }

    #[test]
    fn detect_slot_suffix_uses_default_slot_a_not_b() {
        // The returned suffix must be the DEFAULT boot slot "_a" —
        // other_suffix("_a") = "_b" is only for slot_select_other lookups.
        let dir = std::env::temp_dir().join(format!("twoyi-6z219-odm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_fstab(
            &dir,
            "odm/etc/fstab.default",
            "/dev/block/by-name/system /system ext4 ro wait,slotselect_other\n",
        );
        assert_eq!(detect_guest_slot_suffix(dir.to_str().unwrap()), "_a");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── 6-Z223: hybrid cmdline format regression tests ────────────────
    //
    // Run 33275526098 proved the pure-NUL legacy format is invisible to
    // Android 12+ libfstab (space-splitting ImportKernelCmdlineFromString):
    // the host wrote slot_suffix=_a (6-Z219 log line present) yet the
    // guest still aborted with "Error updating for slotselect". These
    // tests simulate BOTH consumer families byte-exactly.

    #[test]
    fn cmdline_items_contents_and_gating() {
        // A/B device image (e.g. lineage-22.2-sailfish): slot key present.
        let ab = build_cmdline_items("sailfish", "_a");
        assert_eq!(ab[0], "androidboot.hardware=sailfish");
        assert!(ab.iter().any(|i| i == "androidboot.slot_suffix=_a"));
        // ORDERING INVARIANT: the slot suffix is the FINAL item —
        // join_hybrid_cmdline() leaves the final item NUL-free, and
        // libfstab compares that value via std::string equality
        // (other_suffix) and splices it into logical_partition_name
        // (super-partition match for dynamic-partition devices).
        assert_eq!(
            ab.last().map(String::as_str),
            Some("androidboot.slot_suffix=_a")
        );
        // A-only image: the key must be ABSENT, not empty-valued.
        let a_only = build_cmdline_items("angler", "");
        assert!(!a_only
            .iter()
            .any(|i| i.starts_with("androidboot.slot_suffix")));
        // Real-device images must NOT carry ranchu emulator extras.
        for it in build_cmdline_items("sailfish", "_a") {
            assert!(!it.starts_with("androidboot.hardware.gralloc"));
            assert!(!it.starts_with("androidboot.hardware.vulkan"));
            assert!(!it.starts_with("qemu"));
        }
        // ranchu keeps its emulator extras.
        let ranchu = build_cmdline_items("ranchu", "");
        assert!(ranchu
            .iter()
            .any(|i| i == "androidboot.hardware.gralloc=ranchu"));
        assert!(ranchu
            .iter()
            .any(|i| i == "androidboot.hardware.vulkan=ranchu"));
        assert!(ranchu.iter().any(|i| i == "qemu=1"));
        assert!(ranchu.iter().any(|i| i == "qemu.avd_name=twoyi_test"));
    }

    #[test]
    fn hybrid_cmdline_nul_parser_sees_clean_first_item() {
        // OLD init (TWRP 2.8's static Android-5.x init) reads /proc/cmdline,
        // SELF-TERMINATES its buffer (cmdline[n] = 0 in
        // import_kernel_cmdline), then walks space-delimited pieces with
        // C-string semantics — the first strchr(' ') operates on the C
        // string that ends at item 1's NUL, so the boot-critical
        // androidboot.hardware is imported byte-identically to the legacy
        // pure-NUL format and the walk stops there.
        let items = build_cmdline_items("angler", "");
        let content = join_hybrid_cmdline(&items);
        let first = content.split('\0').next().unwrap();
        assert_eq!(first, "androidboot.hardware=angler");
        // The buffer does NOT end with NUL (the final item stays NUL-free
        // for libfstab's std::string consumers) — safe because old init
        // self-terminates its read buffer and never reads past item 1.
        assert!(!content.ends_with('\0'));
        // Even a NUL-iterating reader on a self-terminated buffer stays
        // well-formed: every piece is a key=value item (items 2+ carry a
        // leading space — invisible to them by design), none empty.
        let terminated = format!("{}\0", content);
        for piece in terminated.split_terminator('\0') {
            assert!(!piece.is_empty());
            assert!(piece.contains('='), "piece {:?} is not key=value", piece);
        }
    }

    #[test]
    fn hybrid_cmdline_space_parser_discovers_every_key() {
        // MODERN libfstab (AOSP 15 boot_config.cpp): GetKernelCmdline does
        // Trim(cmdline) then ImportKernelCmdlineFromString — split on ' '
        // (quote spans; none emitted), split each piece at the first '='.
        // The legacy pure-NUL format collapsed everything into ONE piece
        // (only the first '=' pair discoverable); the hybrid format must
        // make EVERY key visible.
        //
        // Downstream consumers (fs_mgr_get_boot_config):
        //   * slotselect.cpp other_suffix(): `slot_suffix == "_a"` is a
        //     std::string comparison, and the suffix is spliced into
        //     entry.logical_partition_name for super-partition matching —
        //     the FINAL item's value must be NUL-FREE.
        //   * Everything else consumes values via C-string semantics
        //     (property storage, mount(2)/open(2), strtoull) — a trailing
        //     NUL on non-final items is harmless and required so old
        //     C-string parsers always terminate correctly.
        let items = build_cmdline_items("sailfish", "_a");
        let content = join_hybrid_cmdline(&items);
        let mut found: Vec<(String, String)> = Vec::new();
        for entry in content.split(' ') {
            let (k, v) = entry
                .split_once('=')
                .unwrap_or_else(|| panic!("piece {:?} has no '='", entry));
            // Key must be NUL-free for libfstab's config_key == key match.
            assert!(!k.contains('\0'), "key {} contains NUL", k);
            let clean = v.trim_end_matches('\0');
            assert!(!clean.contains('\0'), "value of {} has an embedded NUL", k);
            assert!(
                v == clean || (v.len() == clean.len() + 1 && v.ends_with('\0')),
                "value of {} must be clean or carry exactly one trailing NUL: {:?}",
                k,
                v
            );
            found.push((k.to_string(), clean.to_string()));
        }
        let get =
            |key: &str| -> Option<&String> { found.iter().find(|(k, _)| k == key).map(|(_, v)| v) };
        assert_eq!(
            get("androidboot.hardware").map(String::as_str),
            Some("sailfish")
        );
        assert_eq!(
            get("androidboot.slot_suffix").map(String::as_str),
            Some("_a")
        );
        assert_eq!(
            get("androidboot.boot_devices").map(String::as_str),
            Some("pci0000:00/0000:00:03.0")
        );
        assert_eq!(
            get("androidboot.serialno").map(String::as_str),
            Some("twoyi")
        );
        // The slot suffix piece must be byte-identical to the literal the
        // A/B machinery compares against (std::string ==, not C string):
        // last position AND NUL-free.
        assert_eq!(
            content.split(' ').next_back(),
            Some("androidboot.slot_suffix=_a")
        );
    }

    #[test]
    fn hybrid_cmdline_legacy_purenul_vs_hybrid_diff() {
        // Documents the actual bug: with the LEGACY format, a space-split
        // parser finds only ONE key. Keep this test as the regression
        // guard that the hybrid format never collapses back.
        let items = build_cmdline_items("sailfish", "_a");
        let legacy = items.join("\0"); // pre-6-Z223 format
        let legacy_keys: Vec<&str> = legacy
            .split(' ')
            .filter_map(|e| e.split('=').next())
            .collect();
        assert_eq!(legacy_keys.len(), 1, "legacy pure-NUL collapses to 1 piece");
        let hybrid_content = join_hybrid_cmdline(&items);
        let hybrid_keys: Vec<&str> = hybrid_content
            .split(' ')
            .filter_map(|e| e.split('=').next())
            .collect();
        assert_eq!(hybrid_keys.len(), items.len(), "hybrid exposes every key");
    }

    /// Fake ROM bionic: system/lib64/{libc,libdl,libm,libdl_android}.so
    /// as regular files (content irrelevant — staging never reads it).
    fn make_rom_libs(dir: &std::path::Path) {
        let libc_dir = dir.join("system/lib64");
        std::fs::create_dir_all(&libc_dir).unwrap();
        for name in ["libc.so", "libdl.so", "libm.so", "libdl_android.so"] {
            std::fs::write(libc_dir.join(name), b"fakelib").unwrap();
        }
    }

    #[test]
    fn stage_guest_bootstrap_bionic_stages_all_trio_into_both_dirs() {
        let dir = std::env::temp_dir().join(format!("twoyi-6z218-stages-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        make_rom_libs(&dir);

        let (staged, already) = stage_guest_bootstrap_bionic(dir.to_str().unwrap());
        // 4 names × 2 bootstrap dirs = 8 staged, 0 already-present.
        assert_eq!((staged, already), (8, 0));

        // Both bootstrap dirs exist and each symlink resolves inside
        // the rootfs to the ROM's own file.
        let sys_boot = dir.join("system/lib64/bootstrap/libc.so");
        let apex_boot = dir.join("apex/com.android.runtime/lib64/bootstrap/libdl.so");
        assert!(sys_boot.is_file(), "system bootstrap libc must resolve");
        assert!(apex_boot.is_file(), "apex bootstrap libdl must resolve");
        assert_eq!(
            std::fs::read_link(&sys_boot).unwrap().to_str().unwrap(),
            "../../../system/lib64/libc.so"
        );
        assert_eq!(
            std::fs::read_link(&apex_boot).unwrap().to_str().unwrap(),
            "../../../../system/lib64/libdl.so"
        );

        // Names the ROM does NOT ship must not appear (no libm in the
        // ROM → no symlink anywhere for it).
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stage_guest_bootstrap_bionic_skips_missing_rom_names() {
        let dir = std::env::temp_dir().join(format!("twoyi-6z218-partial-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let libc_dir = dir.join("system/lib64");
        std::fs::create_dir_all(&libc_dir).unwrap();
        // ROM ships ONLY libc.so (no libdl/libm/libdl_android).
        std::fs::write(libc_dir.join("libc.so"), b"fakelib").unwrap();

        let (staged, already) = stage_guest_bootstrap_bionic(dir.to_str().unwrap());
        assert_eq!((staged, already), (2, 0), "only libc.so into 2 dirs");

        // libdl must NOT be staged anywhere.
        assert!(!dir.join("system/lib64/bootstrap/libdl.so").exists());
        assert!(!dir
            .join("apex/com.android.runtime/lib64/bootstrap/libdl.so")
            .exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stage_guest_bootstrap_bionic_is_idempotent_and_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("twoyi-6z218-idem-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        make_rom_libs(&dir);

        // First pass stages 8; second pass finds all 8 already present.
        let (s1, a1) = stage_guest_bootstrap_bionic(dir.to_str().unwrap());
        assert_eq!((s1, a1), (8, 0));
        let (s2, a2) = stage_guest_bootstrap_bionic(dir.to_str().unwrap());
        assert_eq!((s2, a2), (0, 8), "idempotent: nothing new staged");

        // A PRE-EXISTING real file (not our symlink) must be preserved
        // — never overwritten.
        let existing = dir.join("system/lib64/bootstrap/libm.so");
        std::fs::write(&existing, b"guest-owns-this").unwrap();
        let (_s3, a3) = stage_guest_bootstrap_bionic(dir.to_str().unwrap());
        assert_eq!(a3, 8);
        assert_eq!(
            std::fs::read(&existing).unwrap(),
            b"guest-owns-this",
            "existing file must win over staging"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stage_guest_bootstrap_bionic_skips_symlinked_rom_sources() {
        let dir = std::env::temp_dir().join(format!("twoyi-6z218-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let libc_dir = dir.join("system/lib64");
        std::fs::create_dir_all(&libc_dir).unwrap();
        // ROM's libc.so is itself a SYMLINK (vendor-redirected) — the
        // farm rule (regular files only) must apply here too, so the
        // bootstrap dir never chains link→link→….
        std::os::unix::fs::symlink("../libc_real.so", libc_dir.join("libc.so")).unwrap();
        std::fs::write(libc_dir.join("libc_real.so"), b"real").unwrap();

        let (staged, already) = stage_guest_bootstrap_bionic(dir.to_str().unwrap());
        assert_eq!(
            (staged, already),
            (0, 0),
            "symlinked ROM source must not be staged"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
