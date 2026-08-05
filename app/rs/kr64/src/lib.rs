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
//! * [`devices`]    — virtual `/dev` device creation (qemu_pipe,
//!                    touch, key, event, gb, gb2).
//! * [`seccomp`]    — BPF seccomp filter + SIGSYS handler.
//! * [`proc_emu`]   — synthesised `/proc` tree (version, cpuinfo,
//!                    meminfo, cmdline, self/, sys/).
//! * [`mount_mgr`]  — `unshare(CLONE_NEWNS)` + `pivot_root` + tmpfs
//!                    mounts for /dev, /proc, /sys, /system, /vendor.
//!
//! # Dependencies
//!
//! Per the task spec ("Use only std + libc, no external crates for
//! now"), this crate depends on **only** `libc` — no `log`, no
//! `once_cell`, no `nix`. Logging is done via the `info!` / `warning!` /
//! `error!` macros defined below (which expand to `eprintln!`), and
//! lazy statics use `std::sync::OnceLock` (stabilised in Rust 1.70).

pub mod devices;
pub mod seccomp;
pub mod proc_emu;
pub mod mount_mgr;

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
pub(crate) use info;
pub(crate) use warning;
pub(crate) use error;

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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            vmid:             0,
            data_dir:         String::new(),
            rootfs:           String::new(),
            rom_dir:          None,
            init_path:        "/system/bin/init".to_string(),
            width:            720,
            height:           1280,
            dpi:              320,
            use_namespaces:   true,
            read_only_rom:    true,
            install_seccomp:  true,
            log_level:        3,
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
                return Err(format!(
                    "twoyi kr64 — kernel-replacement daemon\n\
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
                       --no-seccomp              Skip seccomp filter installation\n"
                ));
            }
            "--rootfs" => {
                cfg.rootfs = iter.next().ok_or("--rootfs requires a path".to_string())?;
            }
            "--data-dir" => {
                cfg.data_dir = iter.next().ok_or("--data-dir requires a path".to_string())?;
            }
            "--rom-dir" => {
                cfg.rom_dir = Some(iter.next().ok_or("--rom-dir requires a path".to_string())?);
            }
            "--init" => {
                cfg.init_path = iter.next().ok_or("--init requires a path".to_string())?;
            }
            "--vmid" => {
                cfg.vmid = iter.next().ok_or("--vmid requires a value".to_string())?
                    .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            "--width" => {
                cfg.width = iter.next().ok_or("--width requires a value".to_string())?
                    .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            "--height" => {
                cfg.height = iter.next().ok_or("--height requires a value".to_string())?
                    .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            "--dpi" => {
                cfg.dpi = iter.next().ok_or("--dpi requires a value".to_string())?
                    .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            "--log-level" => {
                cfg.log_level = iter.next().ok_or("--log-level requires a value".to_string())?
                    .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            "--no-namespaces" => cfg.use_namespaces = false,
            "--rw-rom"        => cfg.read_only_rom = false,
            "--no-seccomp"    => cfg.install_seccomp = false,
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
    // Step 5: fork + exec the guest.
    // ---------------------------------------------------------------
    info!("[KR64] forking guest process");
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        let e = std::io::Error::last_os_error();
        error!("[KR64] fork failed: {}", e);
        return 1;
    }

    if pid == 0 {
        // ----- CHILD (will become the guest init) -----
        info!("[KR64][child] setting up mounts");
        let mount_cfg = mount_mgr::MountConfig {
            rootfs:           cfg.rootfs.clone(),
            rom_dir:          cfg.rom_dir.clone(),
            use_namespaces:   cfg.use_namespaces,
            read_only_rom:    cfg.read_only_rom,
        };
        if let Err(e) = mount_mgr::setup_mounts(&mount_cfg) {
            error!("[KR64][child] mount setup failed: {}", e);
            unsafe { libc::_exit(1) };
        }

        // Install seccomp filter on the guest process. This MUST be
        // done after mount setup (otherwise the mount syscalls would
        // be trapped and we couldn't set up the rootfs).
        if cfg.install_seccomp {
            info!("[KR64][child] installing seccomp filter");
            if let Err(e) = seccomp::install() {
                error!("[KR64][child] seccomp install failed: {}", e);
                // Continue anyway — the guest will see the host's
                // unfiltered syscalls, which is insecure but lets
                // debugging proceed.
            }
        }

        // Exec the guest init.
        info!("[KR64][child] exec'ing guest init at {}", cfg.init_path);
        let init_cstr = CString::new(cfg.init_path.as_str()).unwrap();
        let argv0 = CString::new(cfg.init_path.as_str()).unwrap();
        let argv: [*const libc::c_char; 2] = [argv0.as_ptr(), std::ptr::null()];
        let envp: [*const libc::c_char; 1] = [std::ptr::null()];

        // Set a few environment variables the guest init may look for.
        // (execve replaces envp entirely, so we have to construct it.)
        // For the MVP we just pass an empty envp — the guest's init
        // builds its own environment from /proc/cmdline + properties.
        let r = unsafe { libc::execve(init_cstr.as_ptr(), argv.as_ptr(), envp.as_ptr()) };
        let e = std::io::Error::last_os_error();
        error!("[KR64][child] execve({}) failed (r={}): {}", cfg.init_path, r, e);
        unsafe { libc::_exit(127) };
    }

    // ----- PARENT (the daemon) -----
    info!("[KR64][parent] guest pid = {}", pid);

    // Spawn one accept thread per device socket. For the MVP each
    // thread just accepts connections and immediately closes them
    // (echoing a single byte so the guest sees SOME response). The
    // production version will dispatch to per-device handlers:
    //   qemu_pipe → openglrenderer::pipe
    //   touch     → input::touch_server
    //   key       → input::key_server
    //   event     → TwoyiSocketServer (event IPC)
    //   gb/gb2    → openglrenderer::gralloc
    spawn_accept_thread(device_set.qemu_pipe, "qemu_pipe");
    spawn_accept_thread(device_set.touch,     "touch");
    spawn_accept_thread(device_set.key,       "key");
    spawn_accept_thread(device_set.event,     "event");
    spawn_accept_thread(device_set.gb.gb,     "gb");
    spawn_accept_thread(device_set.gb.gb2,    "gb2");

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

    warning!("[KR64][parent] guest waitpid returned unexpected status {}", status);
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
            warning!("[KR64] cannot spawn accept thread for {}: listener already taken", name);
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
#[no_mangle]
pub extern "C" fn kr64_main(argc: libc::c_int, argv: *const *const libc::c_char) -> libc::c_int {
    // Convert C argv → Rust Vec<String>.
    let args: Vec<String> = if argc <= 0 || argv.is_null() {
        Vec::new()
    } else {
        (0..argc as isize)
            .map(|i| unsafe {
                let p = *argv.offset(i);
                if p.is_null() { String::new() }
                else { std::ffi::CStr::from_ptr(p).to_string_lossy().to_string() }
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
static INTERP_REF: &'static [u8; 0] = unsafe { &INTERP };

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
            "--rootfs", "/r",
            "--data-dir", "/d",
            "--rom-dir", "/rom",
            "--init", "/sbin/init",
            "--vmid", "3",
            "--width", "1080",
            "--height", "1920",
            "--dpi", "480",
            "--log-level", "1",
            "--no-namespaces",
            "--rw-rom",
            "--no-seccomp",
        ])).unwrap();
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
}
