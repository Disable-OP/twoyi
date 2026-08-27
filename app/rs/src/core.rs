// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Core orchestration for the twoyi native renderer.
//!
//! There are two render paths:
//! * TWRP/recovery mode — this module's fb0 reader thread blits the
//!   guest framebuffer straight to the SurfaceView (no GL involved).
//! * Android mode — the AOSP emugl `libOpenglRender.so` built from
//!   source under `app/cpp/emugl` renders the guest's GLES stream.
//!
//! After the 100% open-source migration (task AOSP-VENDOR-1) every
//! native library shipped in the APK is built from source in this
//! repo: libtwoyi.so (app/rs), libloader.so (app/rs/loader),
//! libkr64.so + kr64 (app/rs/kr64), libOpenglRender.so (app/cpp/emugl),
//! libtwoyi_loader_shlib.so + libtwrp_fb_hook.so + libgetpid_hook.so
//! (app/cpp/twoyi_loader, app/cpp/getpid_hook). libadb.so is still the
//! prebuilt ADB binary (upstream code, built by scripts/build_libtwoyi.sh).

use log::info;
use std::ffi::c_void;
use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::thread;

use crate::input;
use crate::renderer_bindings;

static RENDERER_STARTED: AtomicBool = AtomicBool::new(false);

// ──────────────────────────────────────────────────────────────────────────
// Task 6-Z24: direct `__android_log_print` diagnostics + file-based guard.
//
// PROBLEM (verified on a864395 E2E): kr64 re-spawns every ~2 sec (10
// "starting daemon with config" entries in 60s). Each re-spawn kills the
// previous kr64 child (via the pgrep/SIGKILL below) → the recovery
// restarts from scratch every 2 sec, never reaching the framebuffer
// render. The `RENDERER_STARTED` AtomicBool guard SHOULD prevent
// re-spawn, but it demonstrably doesn't (10 spawns observed) — and we
// CAN'T see why because libtwoyi.so's `info!` (android_logger, tag
// "CLIENT_EGL") is INVISIBLE in logcat on release builds (0 CLIENT_EGL
// lines in the a864395 logcat), and the 6-Z23 `eprintln!` diagnostic
// goes to stderr which is also not captured for release apps.
//
// FIX (two-pronged):
//   (1) `alog!` — direct `__android_log_print` FFI call that BYPASSES
//       the `log` crate + android_logger entirely, going straight to
//       logd with a distinct tag ("TWOYI_DIAG"). This GUARANTEES the
//       init_renderer call count + guard state are visible in the next
//       E2E logcat, so we can finally see whether init_renderer is
//       called 10× (guard failed) or 1× (different spawn path).
//   (2) File-based PID-checked guard at the TOP of init_renderer. The
//       `RENDERER_STARTED` Rust static resets if libtwoyi.so is reloaded
//       (the only remaining explanation for the guard not holding). A
//       FILE persists across library reload — so this guard is
//       bulletproof. It stores the app PID; if the lock exists with OUR
//       PID, we skip the kr64 spawn entirely (just update the window).
//       This stops the pgrep-kill + re-spawn cycle so the FIRST kr64
//       child runs uninterrupted for the full boot window → the recovery
//       can finally progress to the framebuffer render.
// ──────────────────────────────────────────────────────────────────────────

extern "C" {
    // Direct FFI to Android's logd. `__android_log_print` is in liblog.so,
    // which is always loaded for Android apps (android_logger already
    // links it). Declaring it extern "C" (no #[link]) resolves at runtime.
    fn __android_log_print(
        prio: libc::c_int,
        tag: *const libc::c_char,
        fmt: *const libc::c_char,
        ...
    ) -> libc::c_int;
}

/// ANDROID_LOG_INFO priority constant for `__android_log_print`.
const ALOG_PRIO_INFO: libc::c_int = 4;
/// ANDROID_LOG_ERROR priority constant for `__android_log_print`.
const ALOG_PRIO_ERROR: libc::c_int = 6;

/// Log a message DIRECTLY to Android logcat (logd), bypassing the `log`
/// crate + `android_logger` (which is invisible on release builds).
///
/// Tag: `TWOYI_DIAG`. Priority: INFO. This is the ONLY reliable way to
/// get a diagnostic from libtwoyi.so into logcat on a release APK —
/// `eprintln!` goes to stderr (not captured for release) and `info!`
/// via android_logger with tag "CLIENT_EGL" produced 0 lines in the
/// a864395 E2E logcat (android_logger init_once appears to not take).
///
/// # Safety
/// Safe wrapper — constructs CStrings + calls the FFI.
fn alog(msg: &str) {
    use std::ffi::CString;
    let tag = match CString::new("TWOYI_DIAG") {
        Ok(t) => t,
        Err(_) => return,
    };
    // Replace any NUL bytes (CString::new would fail on them) so the
    // log line is never silently dropped.
    let sanitized: String = msg.replace('\u{0000}', "\\0");
    let msg_c = match CString::new(sanitized) {
        Ok(c) => c,
        Err(_) => return,
    };
    // "%s" → the message string (no user-controlled format specifiers).
    let fmt = match CString::new("%s") {
        Ok(f) => f,
        Err(_) => return,
    };
    unsafe {
        __android_log_print(ALOG_PRIO_INFO, tag.as_ptr(), fmt.as_ptr(), msg_c.as_ptr());
    }
}

/// Log an ERROR-priority message directly to Android logcat. Same as
/// [`alog`] but at ANDROID_LOG_ERROR priority (visible as `E` in logcat
/// and flagged by logcat filters). Used for spawn-failure diagnostics.
fn alog_error(msg: &str) {
    use std::ffi::CString;
    let tag = match CString::new("TWOYI_DIAG") {
        Ok(t) => t,
        Err(_) => return,
    };
    let sanitized: String = msg.replace('\u{0000}', "\\0");
    let msg_c = match CString::new(sanitized) {
        Ok(c) => c,
        Err(_) => return,
    };
    let fmt = match CString::new("%s") {
        Ok(f) => f,
        Err(_) => return,
    };
    unsafe {
        __android_log_print(ALOG_PRIO_ERROR, tag.as_ptr(), fmt.as_ptr(), msg_c.as_ptr());
    }
}

/// Path to the file-based renderer-init lock. Lives in the app's private
/// data dir (same place as kr64-app-stderr.log). The lock stores the PID
/// of the app process that initialized the renderer.
fn renderer_lock_path() -> String {
    format!("{}/.renderer_init.lock", get_data_dir())
}

/// Check whether THIS app process has already initialized the renderer.
///
/// Returns `true` if the lock file exists AND contains the current
/// process's PID (meaning init_renderer already ran to completion for
/// this process). A stale lock (different, now-dead PID) is removed.
///
/// This is the bulletproof guard that survives libtwoyi.so reload (which
/// resets the `RENDERER_STARTED` Rust static but NOT a file on disk).
///
/// 6-Z184: the lock stores "<pid> <starttime>" — /proc/<pid>/stat field
/// 22, the process's clock ticks at exec. Comparing BOTH kills the PID-
/// reuse false positive: if the OS recycles the exact PID of a dead
/// previous instance, the new process has a different starttime, the
/// guard correctly treats the lock as stale, and kr64 spawns normally
/// (previously the session showed a permanent black screen).
fn renderer_init_done_for_this_process() -> bool {
    let lock = renderer_lock_path();
    let my_pid = unsafe { libc::getpid() };
    if let Ok(content) = std::fs::read_to_string(&lock) {
        let mut parts = content.trim().split_whitespace();
        let lock_pid = parts.next().and_then(|p| p.parse::<i32>().ok());
        let lock_start = parts.next().and_then(|s| s.parse::<u64>().ok());
        if let Some(lock_pid) = lock_pid {
            if lock_pid == my_pid && lock_start == process_start_time(my_pid) {
                return true; // same process already initialized
            }
            // Different PID (or recycled PID with a new start time) —
            // the process that wrote the lock is dead. Remove the stale
            // lock so this process can initialize.
            let _ = std::fs::remove_file(&lock);
        }
    }
    false
}

/// Read field 22 (starttime) of /proc/<pid>/stat — the process's
/// start time in clock ticks since boot. Two processes with the same
/// PID but different start times are different processes.
fn process_start_time(pid: i32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    // Field 2 (comm) can contain spaces inside parens — everything after
    // the LAST ')' is fields 3..; starttime is field 22, i.e. index 19.
    let tail = stat.rsplit(')').next()?;
    tail.split_whitespace().nth(19).and_then(|s| s.parse().ok())
}

/// Mark the renderer as initialized for THIS process (write the PID and
/// start time to the lock file). Called once, right after the kr64 child
/// is spawned.
fn mark_renderer_initialized_for_this_process() {
    let lock = renderer_lock_path();
    let my_pid = unsafe { libc::getpid() };
    let start = process_start_time(my_pid).unwrap_or(0);
    let _ = std::fs::write(&lock, format!("{} {}", my_pid, start));
}

/// When true, the next container launch passes `--boot-recovery` to
/// kr64, booting a TWRP recovery image instead of full Android. Set
/// from Java via `Renderer.setBootRecovery(boolean)` before
/// `Renderer.init()`. Defaults to false (full Android boot).
///
/// TWRP boot skips LD_PRELOAD, /apex bind mount, binderfs mount,
/// SELinux permissive watchdog, /dev/twoyi-bin/ copy, and
/// /dev/__properties__ pre-creation — TWRP's init.rc handles all of
/// those itself. See `kr64::Config::boot_recovery` for the full
/// list of what changes in TWRP mode.
static BOOT_RECOVERY: AtomicBool = AtomicBool::new(false);

/// Current ANativeWindow* target for the TWRP framebuffer blit loop.
///
/// 6-Z183 FIX (black app surface): the render loop used to capture the
/// window pointer ONCE at spawn and blit to it forever. Android can
/// recreate the SurfaceView's buffer queue at any time (immersive-mode
/// transitions, window insets animations, activity resume) — after that
/// the OLD ANativeWindow is dead, every ANativeWindow_lock fails
/// silently, and the app shows a BLACK void while the container keeps
/// rendering (run 33062718661: framework screencap = system overlay +
/// black, while fb0 held a perfect TWRP menu). The loop now re-reads
/// this atomic every tick; `reset_window`/`remove_window` swap it with
/// proper acquire/release bookkeeping, so the blit always targets the
/// LIVE surface and a recreation just forces one re-blit.
static TWRP_WINDOW: AtomicUsize = AtomicUsize::new(0);

extern "C" {
    fn ANativeWindow_acquire(window: *mut c_void);
    fn ANativeWindow_release(window: *mut c_void);
    fn ANativeWindow_getWidth(window: *mut c_void) -> i32;
    fn ANativeWindow_getHeight(window: *mut c_void) -> i32;
}

/// Publish `window` as the TWRP blit target. Acquires a reference on
/// the new window and releases the replaced one, so ownership stays
/// balanced no matter how many times the surface is recreated.
/// `window` may be null to just detach (e.g. surface destroyed).
pub fn twrp_set_window(window: *mut c_void) {
    let new = window as usize;
    let old = TWRP_WINDOW.swap(new, Ordering::SeqCst);
    if new != 0 {
        unsafe { ANativeWindow_acquire(window) };
    }
    if old != 0 {
        unsafe { ANativeWindow_release(old as *mut c_void) };
    }
    alog(&format!(
        "twrp_set_window: {:p} (was {:p})",
        window, old as *mut c_void
    ));
}

/// Set the Boot Recovery (TWRP) flag. Called from JNI
/// (`set_boot_recovery` in lib.rs) before `init_renderer`.
pub fn set_boot_recovery(enabled: bool) {
    BOOT_RECOVERY.store(enabled, Ordering::SeqCst);
    info!("[CORE] Boot Recovery (TWRP) flag set to: {}", enabled);
}

/// Read the Boot Recovery (TWRP) flag. Used by `init_renderer` to
/// decide whether to pass `--boot-recovery` to kr64.
pub fn is_boot_recovery_enabled() -> bool {
    BOOT_RECOVERY.load(Ordering::SeqCst)
}

/// The app's data directory, set from Java via `set_data_dir()`.
///
/// This replaces the previously hardcoded `/data/data/io.twoyi` path.
/// In a work profile the path is `/data/user/<uid>/io.twoyi` instead,
/// so the old hardcoded path would break. The Java side calls
/// `Renderer.setDataDir(context.getDataDir().getAbsolutePath())` before
/// `Renderer.init()`, and all Rust paths are derived from this value.
static DATA_DIR: OnceLock<String> = OnceLock::new();

/// Set the data directory. Called from JNI (`set_data_dir` in lib.rs)
/// before any rendering or input initialization.
pub fn set_data_dir(dir: String) {
    let _ = DATA_DIR.set(dir);
    info!("[CORE] Data directory set to: {:?}", DATA_DIR.get());
}

/// Get the data directory. Falls back to the hardcoded path if
/// `set_data_dir` was never called (backwards compatibility with
/// older Java code that doesn't call `setDataDir`).
pub fn get_data_dir() -> &'static str {
    DATA_DIR
        .get()
        .map(|s| s.as_str())
        .unwrap_or("/data/data/io.twoyi")
}

/// Get the rootfs directory path.
pub fn get_rootfs_dir() -> String {
    format!("{}/rootfs", get_data_dir())
}

/// Get the log file path.
pub fn get_log_path() -> String {
    format!("{}/log.txt", get_data_dir())
}

/// Get the touch device socket path.
///
/// This is the GUEST-FACING touch device socket — the path the guest's
/// `EventHub` opens via `connect()`. As of commit `c67c498` (task 3-A),
/// **kr64 owns this socket** (it binds it via `devices::create_touch_device`
/// and dispatches `InputEvent`s from the IPC socket below). The host's
/// `input.rs` MUST NOT bind this path — doing so conflicts with kr64.
pub fn get_touch_path() -> String {
    format!("{}/rootfs/dev/input/touch", get_data_dir())
}

/// Get the touch-events IPC socket path — the host-side socket the
/// libtwoyi daemon (`app/rs/src/input.rs::touch_server`) binds, and
/// kr64's `spawn_touch_accept_thread` connects to as a client.
///
/// Path: `{data_dir}/dev/touch-events` (NOT under `rootfs/` — this is a
/// host-side IPC channel, not a guest-facing device node).
///
/// This socket carries 20-byte little-endian `TouchMessage` records
/// (action + pointer_id + x + y + pressure) from the host's
/// `handle_touch` JNI callback to kr64's per-connection touch worker,
/// which re-encodes them into the guest's `InputEvent` format via
/// `devices::encode_touch_*`. See commit `c67c498` (kr64 side) and the
/// matching `app/rs/src/input.rs` refactor (host side) for the full
/// IPC contract.
pub fn get_touch_events_path() -> String {
    format!("{}/dev/touch-events", get_data_dir())
}

/// Get the key device socket path.
pub fn get_key_path() -> String {
    format!("{}/rootfs/dev/input/key0", get_data_dir())
}

/// Get the OpenGL ES pipe paths (for socket monitoring).
pub fn get_opengles_paths() -> Vec<String> {
    let rootfs = get_rootfs_dir();
    vec![
        format!("{}/opengles", rootfs),
        format!("{}/opengles2", rootfs),
        format!("{}/opengles3", rootfs),
    ]
}

/// Initialize the renderer with the given parameters.
///
/// Spawns the AOSP emugl renderer thread (which opens the QEMU pipe
/// under `$TWOYI_ROOTFS/opengles*` and serves the guest's EGL/GLES
/// calls), then launches the container's `./init` process.
///
/// Wrapper for raw pointers to make them Send — the pointer is only
/// used from the spawned thread, but Rust doesn't know raw pointers
/// are safe to send. This wrapper asserts Send safety.
struct SendPtr(*mut c_void);
unsafe impl Send for SendPtr {}

/// Renderer thread main function. Called from `thread::spawn` in
/// `init_renderer`. Takes a `SendPtr` (not a raw pointer) so the
/// closure doesn't capture `*mut c_void` (which is not Send).
fn renderer_thread_main(
    window_sp: SendPtr,
    surface_width: i32,
    surface_height: i32,
    virtual_width: i32,
    virtual_height: i32,
    xdpi: i32,
    ydpi: i32,
    fps: i32,
) {
    let window = window_sp.0;
    info!("[CORE] Renderer thread started, window: {:?}", window);

    if is_boot_recovery_enabled() {
        info!("[CORE] TWRP boot: starting framebuffer reader thread (fb0 → SurfaceView)");
        alog("renderer_thread_main: TWRP mode — fb0 reader thread starting");
        let rootfs = get_rootfs_dir();
        let fb_path = format!("{}/dev/graphics/fb0", rootfs);
        let vw = virtual_width;
        let vh = virtual_height;
        // 6-Z183: publish the window through TWRP_WINDOW (the loop re-reads
        // it every tick — see the static's doc). twrp_set_window acquires
        // its OWN reference and releases the previous one, so ownership
        // is fully managed there (the fromSurface reference is dropped
        // when the NativeWindow wrapper in lib.rs goes out of scope —
        // Java's Surface keeps the window alive until surfaceDestroyed).
        twrp_set_window(window);
        std::thread::spawn(move || {
            twrp_fb_render_loop(fb_path, vw, vh);
        });
    } else {
        info!("[CORE] Starting AOSP libOpenglRender.so");
        let result = unsafe {
            renderer_bindings::startOpenGLRenderer(
                window,
                virtual_width,
                virtual_height,
                xdpi,
                ydpi,
                fps,
            )
        };
        if result != 0 {
            log::error!(
                "[CORE] startOpenGLRenderer returned {} (non-zero = failure)",
                result
            );
            // Task 6-Z23: do NOT reset RENDERER_STARTED here. Resetting
            // causes the next surfaceCreated to re-spawn kr64 (the 2-sec
            // re-fork cycle). The OpenGL renderer failure is non-fatal —
            // we keep the guard true so subsequent calls just update the
            // window. (Previously: RENDERER_STARTED.store(false, ...);)
        } else {
            info!("[CORE] Renderer started successfully");
        }
    }
}

/// On subsequent calls (e.g. when the Surface is recreated) the
/// renderer thread is *not* restarted — only the subwindow is reset.
#[allow(clippy::too_many_arguments)]
pub fn init_renderer(
    window: *mut c_void,
    loader_path: String,
    surface_width: i32,
    surface_height: i32,
    virtual_width: i32,
    virtual_height: i32,
    xdpi: i32,
    ydpi: i32,
    fps: i32,
) {
    // Task 6-Z24: file-based PID-checked guard. This is the BULLETPROOF
    // guard that survives libtwoyi.so reload (which resets the
    // RENDERER_STARTED Rust static — the only remaining explanation for
    // the 10 re-spawns observed on a864395). A file on disk persists
    // across library reload. If THIS process already initialized the
    // renderer, skip the kr64 spawn entirely — just refresh the window.
    // This stops the pgrep-kill + re-spawn cycle so the FIRST kr64 child
    // runs uninterrupted for the full boot window.
    let app_pid = unsafe { libc::getpid() };
    alog(&format!(
        "init_renderer called, app_pid={}, RENDERER_STARTED={}",
        app_pid,
        RENDERER_STARTED.load(Ordering::SeqCst)
    ));
    if renderer_init_done_for_this_process() {
        alog("init_renderer: file-guard HOLDS for this PID — refreshing window only, NO kr64 spawn (Task 6-Z24)");
        // The first kr64 child is still running (it's blocked in the
        // ptrace loop). Just update the window so the surface is fresh.
        // 6-Z184: in TWRP mode the emugl renderer was never started —
        // resetSubWindow would poke uninitialized emugl state and the
        // blit loop would keep the STALE window. Route to the TWRP
        // window swap instead (mirrors reset_window below).
        if is_boot_recovery_enabled() {
            twrp_set_window(window);
        } else {
            unsafe {
                renderer_bindings::setNativeWindow(window);
                renderer_bindings::resetSubWindow(
                    window,
                    0,
                    0,
                    surface_width,
                    surface_height,
                    virtual_width,
                    virtual_height,
                    1.0,
                    0.0,
                );
            }
        }
        return;
    }
    alog("init_renderer: file-guard does NOT hold — proceeding with full init (first call for this PID)");
    info!("[CORE] ========================================");
    info!("[CORE] init_renderer called");
    info!(
        "[CORE] Surface: {}x{}, Virtual: {}x{}, FPS: {}",
        surface_width, surface_height, virtual_width, virtual_height, fps
    );
    info!("[CORE] Using AOSP emugl libOpenglRender.so (100% open source)");
    info!("[CORE] ========================================");

    if RENDERER_STARTED
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        eprintln!("[CORE] Renderer already started — updating window only (no kr64 re-spawn)");
        info!("[CORE] Renderer already started, updating window");
        // Renderer already started, just update window (TWRP-mode branch
        // mirrors the file-guard path above).
        if is_boot_recovery_enabled() {
            twrp_set_window(window);
        } else {
            unsafe {
                renderer_bindings::setNativeWindow(window);
                renderer_bindings::resetSubWindow(
                    window,
                    0,
                    0,
                    surface_width,
                    surface_height,
                    virtual_width,
                    virtual_height,
                    1.0,
                    0.0,
                );
            }
        }
    } else {
        info!("[CORE] First time initialization");
        // First time initialization
        input::start_input_system(virtual_width, virtual_height);

        // Diagnostic: log the OpenGL ES pipe paths the renderer will
        // open inside libOpenglRender.so. This is the only call site
        // for get_opengles_paths() — without it, the helper is dead
        // code and rustc emits a `dead_code` warning on every build.
        // Surfacing the paths up front also makes it obvious from the
        // logs whether the rootfs layout is what we expect.
        let opengles_paths = get_opengles_paths();
        info!(
            "[CORE] Renderer will open OpenGL ES pipes: {:?}",
            opengles_paths
        );

        // Convert raw pointer to usize for safe transfer between threads
        let window_addr = window as usize;

        // Set TWOYI_ROOTFS env var in the CURRENT process so that
        // libOpenglRender.so's UnixStream::listen() can find the rootfs
        // path to create the /opengles socket. This must happen BEFORE
        // startOpenGLRenderer is called, because the renderer's
        // RenderServer::create() calls UnixStream::listen() which calls
        // getenv("TWOYI_ROOTFS") in the same process (not the child).
        let working_dir_for_env = get_rootfs_dir();
        unsafe {
            // A Java-supplied data dir could theoretically contain a NUL
            // byte; CString::new would fail and .expect() would unwind
            // across the JNI boundary. Sanitize instead: strip NULs (the
            // env var is best-effort for the renderer's socket path).
            let sanitized: String = working_dir_for_env
                .chars()
                .filter(|c| *c != '\0')
                .collect();
            match std::ffi::CString::new(sanitized.as_str()) {
                Ok(c_rootfs) => {
                    libc::setenv(
                        b"TWOYI_ROOTFS\0".as_ptr() as *const libc::c_char,
                        c_rootfs.as_ptr(),
                        1,
                    );
                }
                Err(e) => {
                    log::error!("[CORE] TWOYI_ROOTFS path not representable as C string: {:?}", e);
                }
            }
        }
        info!(
            "[CORE] Set TWOYI_ROOTFS={} in process env for renderer",
            working_dir_for_env
        );

        // Start the renderer in a separate thread
        // SendPtr is defined at module level — it wraps the raw pointer
        // so it can be sent to the spawned thread. We keep the SendPtr
        // intact throughout the closure (never extract .0 until passing
        // to a function) so Rust's Send checker is satisfied.
        let window_wrap = SendPtr(window_addr as *mut c_void);
        thread::spawn(move || {
            // Pass the SendPtr to a non-closure helper that extracts .0
            // and does the actual work. This avoids the closure capturing
            // the raw *mut c_void field (which is not Send).
            renderer_thread_main(
                window_wrap,
                surface_width,
                surface_height,
                virtual_width,
                virtual_height,
                xdpi,
                ydpi,
                fps,
            );
        });

        let working_dir = get_rootfs_dir();
        let log_path = get_log_path();
        info!("[CORE] Starting container init process");
        info!("[CORE] Working directory: {}", working_dir);
        info!("[CORE] Log path: {}", log_path);

        // -----------------------------------------------------------------
        // Check if a root-launched kr64 has already set up the guest.
        //
        // In the KVM test environment, the test script pre-launches kr64
        // as root (via `adb shell`) BEFORE starting the app. The root
        // kr64 can chroot() and unshare(CLONE_NEWPID) — operations the
        // app's own kr64 can't do because the zygote's seccomp filter
        // blocks them (SIGSYS / signal 31).
        //
        // The root kr64 creates /dev/qemu_pipe (and all other /dev
        // devices) in the rootfs. If we detect that /dev/qemu_pipe
        // already exists, we skip:
        //   1. Our own qemu_pipe proxy (would conflict with root kr64's)
        //   2. Our own kr64 launch (root kr64 is already handling the guest)
        //
        // The app's role in this mode is JUST to provide the renderer
        // (libOpenglRender.so) and the UI — the guest container is
        // managed by the root kr64.
        // -----------------------------------------------------------------
        let qemu_pipe_path = format!("{}/dev/qemu_pipe", working_dir);
        let root_kr64_running = Path::new(&qemu_pipe_path).exists();
        if root_kr64_running {
            info!("[CORE] /dev/qemu_pipe already exists — root kr64 is running");
            info!("[CORE] Skipping app's kr64 launch + qemu_pipe proxy");
            info!("[CORE] App will only provide the renderer (root kr64 handles the guest)");
        } else {
            // -----------------------------------------------------------------
            // Create /dev/qemu_pipe socket + GL proxy BEFORE spawning init.
            //
            // The renderer's RenderServer is listening on $TWOYI_ROOTFS/opengles.
            // The guest's SurfaceFlinger opens /dev/qemu_pipe and writes
            // "pipe:opengles" to request a GL connection. We need a proxy
            // that:
            //   1. Listens on {rootfs}/dev/qemu_pipe
            //   2. Reads the "pipe:opengles" handshake from the guest
            //   3. Connects to {rootfs}/opengles (the renderer)
            //   4. Pumps bytes bidirectionally
            //
            // Without this proxy, the guest can't send GL commands to the
            // renderer and the screen stays black.
            // -----------------------------------------------------------------
            let rootfs_for_pipe = working_dir.clone();
            std::thread::Builder::new()
                .name("twoyi-qemu-pipe-setup".into())
                .spawn(move || {
                    spawn_qemu_pipe_proxy(&rootfs_for_pipe);
                })
                .ok();
            info!("[CORE] qemu_pipe proxy thread spawned");
        }

        // -----------------------------------------------------------------
        // Init spawn strategy (post-2026-08-09 fix):
        //
        // Android's init binary requires PID 1 status and a properly set up
        // environment (property service, /dev/__properties__, seccomp, etc.)
        // If we exec init directly, it exits with code 31 because it's not
        // PID 1 and can't set up the property service.
        //
        // The fix: exec libkr64.so (the kr64 daemon) instead. The kr64
        // daemon sets up the virtual /dev tree, creates /dev/__properties__,
        // installs a seccomp filter, and then execs init in a child process
        // where init believes it's PID 1. The kr64 daemon acts as a
        // "kernel replacement" that provides the environment init expects.
        //
        // libkr64.so is a PIE cdylib — it can be exec'd directly because
        // its ELF entry point is set to kr64_main via the -Wl,-e linker
        // flag. The kernel loads it via the dynamic linker and jumps to
        // kr64_main, which parses args and calls run().
        //
        // If libkr64.so doesn't exist (e.g. not yet built), fall back to
        // the rootfs linker + init approach (which will fail with exit 31
        // but at least we'll see the error).
        //
        // SKIP ENTIRELY if root kr64 is already running (detected above).
        // The root kr64 handles chroot + PID namespace + exec init —
        // things the app's kr64 can't do because the zygote's seccomp
        // filter blocks chroot/mount/unshare with SIGSYS.
        // -----------------------------------------------------------------
        if root_kr64_running {
            info!("[CORE] Root kr64 is running — skipping app's kr64 launch entirely");
            // The root kr64 is already handling the guest. We just need
            // to make sure the renderer is started (done above) and the
            // app's UI is shown. Nothing else to do here.
            return;
        }

        let init_path = format!("{}/init", working_dir);

        // libkr64.so is in the app's nativeLibraryDir (symlinked into rootfs
        // by RomManager.ensureLibSymlink). The loader_path passed from Java
        // is the path to libloader.so in the same dir.
        let kr64_path = loader_path.replace("libloader.so", "libkr64.so");

        // Derive the app's nativeLibraryDir from loader_path. This is the
        // directory where Android's package manager extracted the APK's
        // lib/<abi>/*.so files at install time (extractNativeLibs=true).
        // We pass it to kr64 via the TWOYI_NATIVE_LIB_DIR env var so kr64
        // can directly read libtwrp_fb_hook.so (and other hook libs) from
        // <nativeLibraryDir>/<lib> WITHOUT scanning /data/app/.
        //
        // Why this matters: kr64's apk_native_lib_candidates_in() does
        // read_dir("/data/app/") to find the APK install dir, but
        // /data/app/ is mode 0771 (rwxrwx--x) — untrusted_app CANNOT
        // listdir it (only traverse). So on real devices where kr64
        // runs unprivileged (the ptrace-emulation path), the scan
        // returns 0 candidates and the hook library is "not found".
        // Passing the path from Java (where ApplicationInfo.nativeLibraryDir
        // is known and the app CAN read its own APK lib dir) sidesteps
        // the permission issue entirely. See lib.rs::hook_library_candidates
        // candidate #0 for the consumer of this env var.
        let native_lib_dir = loader_path
            .rsplitn(2, '/')
            .nth(1)
            .map(|s| s.to_string())
            .unwrap_or_default();
        if !native_lib_dir.is_empty() {
            info!(
                "[CORE] Passing TWOYI_NATIVE_LIB_DIR={} to kr64",
                native_lib_dir
            );
        } else {
            log::warn!(
                "[CORE] Could not derive nativeLibraryDir from loader_path='{}' — \
                 kr64 will fall back to the /data/app/ scan (which fails on real devices)",
                loader_path
            );
        }
        let ld_library_path = format!(
            "{root}/system/lib64:{root}/system/lib64/bootstrap:{root}/system/lib64/vndk-sp-29:{root}/system/lib64/vndk-29:{root}/system/lib64/apex:{root}/system/lib",
            root = working_dir
        );

        // NOTE: the guest-init log file (log.txt) is created ONLY on the
        // fallback branch below (the only branch that pipes into it).
        // Creating it unconditionally used to TRUNCATE the previous
        // boot's guest-init log on every init_renderer call — including
        // TWRP/kr64 boots, which never write to it at all (they use
        // kr64-app-stderr.log).

        let mut cmd;
        if Path::new(&kr64_path).exists() {
            // Use the kr64 binary directly. It's a regular PIE executable
            // built with --bin kr64 (proper _start from crt1.o).
            //
            // kr64 will fork() + exec init in a child. The child gets
            // a fresh PID. kr64 also sets up /dev, properties, etc.
            //
            // If kr64 fails (exit 1), the fallback below (direct init
            // via rootfs linker) will be tried on the next boot attempt.
            info!("[CORE] Using kr64 binary: {}", kr64_path);
            cmd = Command::new(&kr64_path);
            cmd.current_dir(&working_dir);
            cmd.arg("--rootfs").arg(&working_dir);
            cmd.arg("--data-dir").arg(get_data_dir());
            cmd.arg("--vmid").arg("0");
            cmd.arg("--no-namespaces");
            cmd.arg("--no-seccomp");
            // Pass the profile's virtual display dimensions to kr64 so
            // it creates the fb0 file with the correct size (instead of
            // hardcoded 720x1280). These come from ProfileSettings which
            // auto-detects the physical screen resolution.
            cmd.arg("--width").arg(virtual_width.to_string());
            cmd.arg("--height").arg(virtual_height.to_string());
            // TWRP boot: pass --boot-recovery so kr64 uses the simple
            // TWRP boot path (skips LD_PRELOAD, /apex bind, binderfs,
            // SELinux watchdog, /dev/twoyi-bin/ copy; auto-sets
            // init_path=/init). The flag is a no-op when false (the
            // kr64 default is full-Android boot).
            if is_boot_recovery_enabled() {
                info!("[CORE] Boot Recovery (TWRP) enabled — passing --boot-recovery to kr64");
                cmd.arg("--boot-recovery");
            } else {
                // Task 6-Z88: NORMAL (AOSP) mode — pass the init path
                // explicitly. The cyanmint 8.1 profile is a ramdisk-style
                // rootfs whose init lives at {rootfs}/init; there is NO
                // /system/bin/init (kr64's Config::default()). Without
                // this, the guest child's first fs::read of
                // "{rootfs}/system/bin/init" gets ENOENT and _exit(127)'s
                // on every attempt (244× in E2E run 32632668179 — zero
                // guest instructions ever executed).
                info!("[CORE] NORMAL (AOSP) boot — passing --init /init to kr64 (ramdisk-style profile)");
                cmd.arg("--init").arg("/init");
            }
            // Use the HOST system lib64 for kr64's own dependencies
            // (libc.so, libdl.so, etc.) — NOT the rootfs's versions.
            // The rootfs's libc.so may be incompatible with the host
            // linker, causing the C runtime to crash before main().
            cmd.env("LD_LIBRARY_PATH", "/system/lib64:/vendor/lib64");
        } else {
            // Fallback: exec rootfs linker + init directly.
            // This will fail with exit 31 (init not PID 1) but at least
            // we get diagnostic output.
            info!(
                "[CORE] libkr64.so not found, falling back to direct init (will fail with exit 31)"
            );
            let bootstrap_linker = format!("{}/system/bin/bootstrap/linker64", working_dir);
            let legacy_linker = format!("{}/system/bin/linker64", working_dir);
            let linker_path = if Path::new(&bootstrap_linker).exists() {
                bootstrap_linker
            } else {
                legacy_linker
            };
            info!("[CORE] Using rootfs linker: {}", linker_path);
            info!("[CORE] Init path: {}", init_path);
            cmd = Command::new(&linker_path);
            cmd.current_dir(&working_dir);
            cmd.arg(&init_path);
        }

        // These env calls only affect the spawned child (kr64 or the
        // fallback linker+init); the current process env is untouched.
        // Set LD_LIBRARY_PATH. For the kr64 path, we already set it to
        // the HOST system lib64 (kr64 needs host libs, not rootfs libs).
        // For the fallback path, use the rootfs lib64 paths so init
        // gets rootfs libs.
        if !Path::new(&kr64_path).exists() {
            // Fallback path — use rootfs libs for init
            cmd.env("LD_LIBRARY_PATH", &ld_library_path);
        }
        // kr64 path already set LD_LIBRARY_PATH above
        cmd.env_remove("LD_PRELOAD");
        cmd.env("TYLD_PRELOAD", "");
        cmd.env("TWOYI_ROOTFS", &working_dir);
        // Task 6-Z88: pass the app's data dir so kr64's helpers (e.g.
        // apex_extract's temp-dir resolution) can derive package-correct
        // paths (io.twoyi.debug, work profiles) instead of falling back
        // to the hardcoded /data/data/io.twoyi/cache (Permission denied
        // for any other package — run 32632668179).
        cmd.env("TWOYI_DATA_DIR", get_data_dir());
        cmd.env("TYLOADER", &loader_path);
        // Pass the app's nativeLibraryDir (derived from loader_path above)
        // so kr64 can find hook libraries (libtwrp_fb_hook.so, libgetpid_hook.so,
        // libtwoyi_loader_shlib.so) without scanning /data/app/ — which is
        // mode 0771 and unreadable for untrusted_app. See lib.rs's
        // hook_library_candidates() candidate #0 for the consumer.
        if !native_lib_dir.is_empty() {
            cmd.env("TWOYI_NATIVE_LIB_DIR", &native_lib_dir);
        }
        cmd.env("ANDROID_BOOTLOGO", "1");
        cmd.env("ANDROID_ROOT", format!("{}/system", working_dir));
        cmd.env("ANDROID_DATA", format!("{}/data", working_dir));
        cmd.env_remove("BOOTCLASSPATH");
        cmd.env_remove("SYSTEMSERVERCLASSPATH");
        // For the kr64 path, redirect stderr to a separate log file
        // that we can pull via adb. Android's release builds redirect
        // stderr to /dev/null, so Stdio::inherit() doesn't work.
        // For the fallback path, use the app's log file.
        if Path::new(&kr64_path).exists() {
            let kr64_log = format!("{}/kr64-app-stderr.log", get_data_dir());
            match File::create(&kr64_log) {
                Ok(f) => {
                    // Clone for stderr without a panic path: if try_clone
                    // fails (fd exhaustion), fall back to inheriting for
                    // stderr rather than unwinding across the JNI boundary
                    // (this function is called from Java).
                    match f.try_clone() {
                        Ok(f2) => {
                            cmd.stdout(Stdio::from(f));
                            cmd.stderr(Stdio::from(f2));
                        }
                        Err(e) => {
                            log::error!("[CORE] could not clone kr64 log fd: {}", e);
                            cmd.stdout(Stdio::from(f));
                            cmd.stderr(Stdio::inherit());
                        }
                    }
                    info!("[CORE] kr64 stderr → {}", kr64_log);
                }
                Err(e) => {
                    log::error!("[CORE] Failed to create kr64 log: {}", e);
                    cmd.stdout(Stdio::inherit());
                    cmd.stderr(Stdio::inherit());
                }
            }
        } else {
            // Fallback branch — create (truncate) the guest-init log here
            // and now only here, right before it is used.
            let piped = match File::create(&log_path) {
                Ok(f) => match f.try_clone() {
                    Ok(e) => Some((f, e)),
                    Err(err) => {
                        log::error!("[CORE] Failed to clone log file handle: {}", err);
                        None
                    }
                },
                Err(e) => {
                    log::error!("[CORE] Failed to create log file {}: {}", log_path, e);
                    None
                }
            };
            match piped {
                Some((o, e)) => {
                    cmd.stdout(Stdio::from(o));
                    cmd.stderr(Stdio::from(e));
                }
                None => {
                    cmd.stdout(Stdio::inherit());
                    cmd.stderr(Stdio::inherit());
                }
            }
        }

        // Task 6-Z13: Kill any existing kr64 process before spawning a new one.
        // When Android kills + restarts the Render2Activity (OOM, ANR, config
        // change), the old kr64 process is NOT cleaned up — it keeps running
        // (it's a detached process). The new Activity calls init_renderer()
        // again, which spawns a NEW kr64. The old kr64 holds the
        // property_service socket (EADDRINUSE), /dev devices, etc. — the new
        // kr64 conflicts with it.
        //
        // Fix: before spawning the new kr64, find and SIGKILL any existing
        // kr64 process. We use `pgrep -f libkr64.so` to find it (the process
        // name is the .so path because kr64 is exec'd as a PIE cdylib).
        // This is safe because:
        //   - We're about to spawn a NEW kr64 anyway
        //   - The old kr64's child (init/guest) will be orphaned + reparented
        //     to init (PID 1), which will reap it
        //   - The old kr64's daemon threads (qemu_pipe, touch, etc.) will be
        //     killed when the process dies
        {
            let pgrep = std::process::Command::new("pgrep")
                .arg("-f")
                .arg("libkr64.so")
                .output();
            if let Ok(output) = pgrep {
                let pids_str = String::from_utf8_lossy(&output.stdout);
                for pid_str in pids_str.split_whitespace() {
                    if let Ok(old_pid) = pid_str.parse::<i32>() {
                        // Don't kill ourselves
                        let my_pid = unsafe { libc::getpid() };
                        if old_pid != my_pid {
                            info!(
                                "[CORE] Killing existing kr64 process (PID={}) before spawning new one",
                                old_pid
                            );
                            unsafe {
                                libc::kill(old_pid, libc::SIGKILL);
                            }
                            // Wait briefly for the process to die + release
                            // its resources (sockets, fds, etc.)
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                }
            }
        }

        match cmd.spawn() {
            Ok(child) => {
                info!("[CORE] Container init spawned, PID={}", child.id());
                alog(&format!(
                    "kr64 child SPAWNED, child_pid={} (Task 6-Z24: this should appear EXACTLY ONCE per app session — if it appears >1×, the file-guard is also failing)",
                    child.id()
                ));
                // Task 6-Z24: write the PID lock so subsequent init_renderer
                // calls (surface recreation) skip the spawn + pgrep-kill.
                mark_renderer_initialized_for_this_process();
            }
            Err(e) => {
                log::error!("[CORE] FAILED to spawn container init: {}", e);
                alog_error(&format!(
                    "kr64 child spawn FAILED: {} (errno path; will NOT write lock so a retry can happen)",
                    e
                ));
                log::error!(
                    "[CORE]   kr64_path: {} (exists: {})",
                    kr64_path,
                    Path::new(&kr64_path).exists()
                );
                log::error!(
                    "[CORE]   init_path: {} (exists: {})",
                    init_path,
                    Path::new(&init_path).exists()
                );
                log::error!(
                    "[CORE]   working_dir: {} (exists: {})",
                    working_dir,
                    Path::new(&working_dir).exists()
                );
            }
        }
    }
}

/// Reset window parameters.
pub fn reset_window(
    window: *mut c_void,
    top: i32,
    left: i32,
    width: i32,
    height: i32,
    fb_width: i32,
    fb_height: i32,
) {
    // 6-Z183: in TWRP mode the emugl subwindow is not running — the fb0
    // blit loop owns the display path. Route the (possibly RECREATED)
    // surface to it so blits always target the live window; the loop
    // sees the swap and force-re-blits the current frame.
    if is_boot_recovery_enabled() {
        twrp_set_window(window);
        return;
    }
    unsafe {
        renderer_bindings::resetSubWindow(
            window, left, top, width, height, fb_width, fb_height, 1.0, 0.0,
        );
    }
}

/// Remove a window.
pub fn remove_window(window: *mut c_void) {
    // 6-Z183: TWRP mode — detaching the blit target pauses rendering
    // (the loop idles at 200 ms ticks with zero fb reads until a fresh
    // surfaceCreated/reset_window publishes a new window).
    if is_boot_recovery_enabled() {
        twrp_set_window(std::ptr::null_mut());
        return;
    }
    unsafe {
        renderer_bindings::removeSubWindow(window);
    }
}

// ---------------------------------------------------------------------------
// qemu_pipe GL proxy — creates /dev/qemu_pipe in the guest rootfs and
// forwards guest GL connections to the renderer's /opengles socket.
// ---------------------------------------------------------------------------

/// Create /dev/qemu_pipe and run a proxy that forwards guest connections
/// to the renderer's RenderServer listening on {rootfs}/opengles.
///
/// The AOSP qemu_pipe protocol:
///   1. Guest opens /dev/qemu_pipe
///   2. Guest writes "pipe:opengles" (the channel name)
///   3. Host reads the channel name, connects to {rootfs}/opengles
///   4. Bytes flow bidirectionally (guest GL commands → renderer)
fn spawn_qemu_pipe_proxy(rootfs: &str) {
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};

    let pipe_path = format!("{}/dev/qemu_pipe", rootfs);
    let dev_dir = format!("{}/dev", rootfs);

    // Ensure /dev exists
    let _ = std::fs::create_dir_all(&dev_dir);

    // Remove stale socket
    let _ = std::fs::remove_file(&pipe_path);

    // Bind the listener
    let listener = match UnixListener::bind(&pipe_path) {
        Ok(l) => {
            info!("[CORE] qemu_pipe proxy listening at {}", pipe_path);
            l
        }
        Err(e) => {
            log::error!("[CORE] Failed to bind qemu_pipe at {}: {}", pipe_path, e);
            return;
        }
    };

    // chmod 0666 so the guest (which may run as a different uid in the chroot) can connect
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&pipe_path, std::fs::Permissions::from_mode(0o666));

    let mut session_id: u64 = 0;
    loop {
        match listener.accept() {
            Ok((mut guest, _addr)) => {
                let sid = session_id;
                session_id += 1;
                info!("[CORE] qemu_pipe: guest connected (session={})", sid);

                // Read the "pipe:<channel>" handshake (plus any early
                // payload bytes that arrived with it).
                let (channel, leftover) = match read_channel_name(&mut guest) {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("[CORE] qemu_pipe: session {} handshake failed: {}", sid, e);
                        continue;
                    }
                };
                info!("[CORE] qemu_pipe: session {} channel = {}", sid, channel);

                if !KNOWN_PIPE_CHANNELS.contains(&channel.as_str()) {
                    log::warn!(
                        "[CORE] qemu_pipe: session {} unknown channel '{}'",
                        sid,
                        channel
                    );
                    continue;
                }

                // Connect to the renderer
                let renderer_path = format!("{}/{}", rootfs, channel);
                let mut renderer = match UnixStream::connect(&renderer_path) {
                    Ok(r) => r,
                    Err(e) => {
                        log::error!(
                            "[CORE] qemu_pipe: session {} connect to {} failed: {}",
                            sid,
                            renderer_path,
                            e
                        );
                        continue;
                    }
                };
                info!(
                    "[CORE] qemu_pipe: session {} connected to {}",
                    sid, renderer_path
                );

                // Forward handshake-tail bytes (previously DROPPED —
                // desyncing the emugl wire protocol when the guest
                // coalesced the channel name with the first payload).
                if !leftover.is_empty() {
                    use std::io::Write;
                    log::info!(
                        "[CORE] qemu_pipe: session {} forwarding {} handshake-tail bytes",
                        sid,
                        leftover.len()
                    );
                    if renderer.write_all(&leftover).is_err() {
                        continue;
                    }
                }

                // Spawn two pump threads
                let mut guest_w = match guest.try_clone() {
                    Ok(g) => g,
                    Err(e) => {
                        log::error!(
                            "[CORE] qemu_pipe: session {} guest clone failed: {}",
                            sid,
                            e
                        );
                        continue;
                    }
                };
                let mut renderer_r = match renderer.try_clone() {
                    Ok(r) => r,
                    Err(e) => {
                        log::error!(
                            "[CORE] qemu_pipe: session {} renderer clone failed: {}",
                            sid,
                            e
                        );
                        continue;
                    }
                };

                let mut guest_r = guest;
                let mut renderer_w = renderer;

                let sid_g2r = sid;
                std::thread::Builder::new()
                    .name(format!("pipe-g2r-{}", sid_g2r))
                    .spawn(move || {
                        let mut buf = [0u8; 16 * 1024];
                        loop {
                            match guest_r.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    if renderer_w.write_all(&buf[..n]).is_err() {
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                        let _ = renderer_w.shutdown(std::net::Shutdown::Both);
                    })
                    .ok();

                let sid_r2g = sid;
                std::thread::Builder::new()
                    .name(format!("pipe-r2g-{}", sid_r2g))
                    .spawn(move || {
                        let mut buf = [0u8; 16 * 1024];
                        loop {
                            match renderer_r.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    if guest_w.write_all(&buf[..n]).is_err() {
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                        let _ = guest_w.shutdown(std::net::Shutdown::Both);
                    })
                    .ok();
            }
            Err(e) => {
                log::warn!("[CORE] qemu_pipe: accept error: {}", e);
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}

/// Read the "pipe:<channel>" handshake from the guest. Returns
/// `(name, leftover)` — leftover holds any bytes that arrived in the
/// same packet after the channel name (e.g. the first clientFlags
/// word) and must be forwarded to the renderer by the caller.
/// Only a TERMINATED name (non-printable byte, or exactly a known
/// channel) is accepted — end-of-buffer alone must NOT terminate, or
/// the split delivery "pipe:open"+"gles" would parse as "open".
fn read_channel_name(
    stream: &mut std::os::unix::net::UnixStream,
) -> std::io::Result<(String, Vec<u8>)> {
    use std::io::Read;
    let mut buf = [0u8; 256];
    let mut total = 0;
    while total < buf.len() {
        let n = stream.read(&mut buf[total..])?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "guest closed before sending channel name",
            ));
        }
        total += n;
        if let Some((name, consumed)) = parse_channel_name(&buf[..total]) {
            return Ok((name.to_string(), buf[consumed..total].to_vec()));
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "channel name too long",
    ))
}

/// Channel names the qemu_pipe dispatcher accepts.
const KNOWN_PIPE_CHANNELS: [&str; 3] = ["opengles", "opengles2", "opengles3"];

/// Parse the "pipe:<channel>" handshake — stops at NUL or non-printable.
/// Returns `(name, consumed)` where consumed spans buf[..] through the
/// END of the name; leftover bytes after that belong to the payload.
fn parse_channel_name(buf: &[u8]) -> Option<(&str, usize)> {
    const PREFIX: usize = 5; // "pipe:".len()
    let prefix = b"pipe:";
    if !buf.starts_with(prefix) {
        return None;
    }
    let name_bytes = &buf[PREFIX..];
    let end = name_bytes
        .iter()
        .position(|&b| b == 0 || !(0x20..=0x7e).contains(&b));
    let end = match end {
        Some(i) => i,
        // No terminator: only accept an exact known channel (AOSP
        // writes exactly "pipe:opengles" and keeps the stream open).
        None => {
            let candidate = std::str::from_utf8(name_bytes).ok()?;
            if KNOWN_PIPE_CHANNELS.contains(&candidate) {
                return Some((candidate, buf.len()));
            }
            return None;
        }
    };
    if end == 0 {
        return None;
    }
    let name = std::str::from_utf8(&name_bytes[..end]).ok()?;
    Some((name, PREFIX + end))
}

// ---------------------------------------------------------------------------
// TWRP framebuffer rendering
// ---------------------------------------------------------------------------

/// Default color depth (bits per pixel) for the TWRP framebuffer.
/// NOTE: fb0's byte order is BGRA (see twrp_blit_to_surface); the value
/// 4 here is BYTES per pixel (32bpp), not a format claim.
const DEFAULT_FB_BPP: usize = 4;

/// Render loop for TWRP boot mode.
///
/// Reads {rootfs}/dev/graphics/fb0 periodically and blits the pixels
/// to the LIVE ANativeWindow (SurfaceView — re-read from TWRP_WINDOW
/// every tick, 6-Z183). This makes the TWRP UI visible in the Java app
/// without requiring OpenGL ES.
///
/// `fb_path` is the host path to the fb0 file (e.g.
/// "/data/user/0/io.twoyi/rootfs/dev/graphics/fb0").
/// `virtual_width`/`virtual_height` are the TWRP display dimensions (from
/// the profile settings — NOT hardcoded).
fn twrp_fb_render_loop(fb_path: String, virtual_width: i32, virtual_height: i32) {
    use std::io::Read;
    use std::time::Duration;

    // Use the profile's virtual display dimensions (NOT hardcoded 720x1280).
    // The fb0 file is created by kr64's devices::create_twrp_framebuffer()
    // using the SAME virtual_width x virtual_height, so they match.
    let fb_w = virtual_width as usize;
    let fb_h = virtual_height as usize;
    let fb_bpp = DEFAULT_FB_BPP;
    let fb_size = fb_w * fb_h * fb_bpp;

    info!(
        "[CORE][TWRP-FB] render loop started: fb_path={} virtual={}x{} fb_size={}",
        fb_path, virtual_width, virtual_height, fb_size
    );
    alog(&format!(
        "TWRP-FB render loop started: fb={} virtual={}x{}",
        fb_path, virtual_width, virtual_height
    ));

    // 6-Z183: the window is re-read every tick from TWRP_WINDOW; when it
    // changes (surface recreated) force one unconditional re-blit so the
    // fresh blank surface gets pixels immediately.
    let mut last_window: usize = 0;
    let mut blit_fail_logged_for: usize = 0;
    let mut first_blit_logged = false;

    // Wait for the fb0 file to exist (kr64 creates it before forking init).
    let mut waited = 0u32;
    while !Path::new(&fb_path).exists() {
        std::thread::sleep(Duration::from_millis(500));
        waited += 1;
        if waited > 120 {
            log::error!(
                "[CORE][TWRP-FB] fb0 file not found after 60s: {} — giving up",
                fb_path
            );
            return;
        }
    }
    info!(
        "[CORE][TWRP-FB] fb0 file found after {}s — starting render loop",
        waited / 2
    );

    // Allocate the framebuffer read buffer + a "last blitted" copy for the
    // 6-Z172 dirty-check throttle. Without it the loop reads fb_size bytes
    // AND pushes a full ANativeWindow frame at ~30fps even when TWRP shows
    // a static menu — at native resolutions (e.g. 720x1600 = 4.6 MiB/frame)
    // that is ~276 MiB/s of memory traffic plus a SurfaceFlinger composite
    // per frame in redroid's SOFTWARE guest GPU mode, which saturated all
    // runner cores and wedged the whole framework (run 33014296538: every
    // adb channel — screencap/dumpsys/logcat — dead within 5 s of launch).
    // TWRP screens are mostly STATIC (menus): compare-then-blit makes the
    // idle cost ~one fb read per tick and a blit ONLY on real changes, with
    // an adaptive backoff (33 ms → 250 ms) while nothing changes.
    let mut fb_buf = vec![0u8; fb_size];
    let mut last_blit: Vec<u8> = Vec::with_capacity(fb_size);
    let mut idle_ticks: u32 = 0;
    let mut short_read_logged: u32 = 0;

    // Render loop: read fb0 → (dirty-check) → blit to the LIVE window.
    loop {
        // 6-Z183: no live surface (app backgrounded / surface destroyed)?
        // Idle cheaply — no fb reads, no blits, no spin.
        let window = TWRP_WINDOW.load(Ordering::SeqCst) as *mut c_void;
        if window.is_null() {
            std::thread::sleep(Duration::from_millis(200));
            continue;
        }

        // Read the framebuffer file.
        let file = match std::fs::File::open(&fb_path) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("[CORE][TWRP-FB] open({}) failed: {} — retrying", fb_path, e);
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
        };
        let mut reader = std::io::BufReader::new(file);
        // Tolerant read (6-Z172): a short fb0 file must not wedge the loop
        // in a warn-storm — read whatever is there, zero-fill the rest so
        // the dirty check sees a coherent frame, and log the shortfall
        // (with the actual file size) only for the first few occurrences.
        {
            let mut total = 0usize;
            while total < fb_size {
                use std::io::Read as _;
                match reader.read(&mut fb_buf[total..]) {
                    Ok(0) => break,
                    Ok(n) => total += n,
                    Err(_) => break,
                }
            }
            if total < fb_size {
                fb_buf[total..].fill(0);
                if short_read_logged < 5 {
                    short_read_logged += 1;
                    let fsize = std::fs::metadata(&fb_path).map(|m| m.len()).unwrap_or(0);
                    log::warn!(
                        "[CORE][TWRP-FB] short read: got {}/{} bytes (fb0 file len={}) — rendering partial frame",
                        total,
                        fb_size,
                        fsize
                    );
                }
            }
        }

        // Dirty check (u64 chunks; both buffers are fb_size long). Skip the
        // blit entirely when the framebuffer content did not change.
        let window_changed = last_window != window as usize;
        let changed = window_changed || {
            let prev: &[u8] = &last_blit;
            prev.len() != fb_buf.len()
                || prev
                    .chunks_exact(8)
                    .zip(fb_buf.chunks_exact(8))
                    .any(|(a, b)| a != b)
                || prev[prev.len() & !7..] != fb_buf[fb_buf.len() & !7..]
        };
        if changed {
            // Blit the framebuffer to the ANativeWindow.
            let ok = unsafe { twrp_blit_to_surface(window, &fb_buf, fb_w, fb_h, fb_bpp) };
            if ok {
                last_window = window as usize;
                if !first_blit_logged {
                    first_blit_logged = true;
                    alog(&format!(
                        "TWRP-FB first frame blitted to surface ({}, {}x{})",
                        fb_path, fb_w, fb_h
                    ));
                }
                // Update the dirty-check baseline ONLY on success: on
                // failure the next tick must still see the frame as
                // "changed" so the blit is genuinely retried (a single
                // transient ANativeWindow_lock failure — busy
                // SurfaceFlinger, mid-recreation window — used to freeze
                // the display on stale pixels until fb0 content changed).
                if last_blit.is_empty() {
                    log::info!(
                        "[CORE][TWRP-FB] first non-blank frame blitted ({}x{})",
                        fb_w,
                        fb_h
                    );
                }
                last_blit.clear();
                last_blit.extend_from_slice(&fb_buf);
                idle_ticks = 0;
            } else {
                // Rate-limit: log a lock/geometry failure ONCE per window
                // instance (a dead surface would otherwise spam every tick).
                // last_window is NOT advanced on failure — the blit
                // retries next tick via the still-stale baseline above.
                if blit_fail_logged_for != window as usize {
                    blit_fail_logged_for = window as usize;
                    alog_error("TWRP-FB blit FAILED (setBuffersGeometry/lock) — surface shows stale pixels");
                }
            }
            std::thread::sleep(Duration::from_millis(33));
        } else {
            idle_ticks = idle_ticks.saturating_add(1);
            // 33ms while fresh, backing off to 250ms when static.
            let delay_ms = match idle_ticks {
                0..=3 => 33,
                4..=10 => 66,
                _ => 250,
            };
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
    }
}

/// Blit the TWRP framebuffer to the ANativeWindow.
///
/// Uses ANativeWindow_lock/unlockAndPost to write pixels directly to the
/// SurfaceView's buffer. The surface buffer format is requested as
/// WINDOW_FORMAT_RGBA_8888 (**1**).
///
/// 6-Z183 COLOR FIX: this used to request format **5**, which the comment
/// claimed was WINDOW_FORMAT_RGBA_8888 — but 5 is HAL_PIXEL_FORMAT_BGRA_8888
/// (WINDOW_FORMAT_RGBA_8888 is 1; the WINDOW_FORMAT_* enum aliases the
/// HAL_PIXEL_FORMAT_* values). The blit below packs pixels in RGBA byte
/// order, so a BGRA buffer composited them with R and B swapped — TWRP's
/// blue theme (#0090CA) rendered ORANGE on the app display (the "colors
/// seem off" report). Requesting format 1 makes the RGBA packing correct.
/// On grallocs that don't honor the request the returned buffer.format is
/// honoured below (BGRA → pack BGR order) so the colors stay right either
/// way.
///
/// We scale the source framebuffer to the window's current dimensions
/// using nearest-neighbor sampling.
///
/// Returns true when a frame was actually posted.
///
/// Parameters:
/// - `fb`: raw framebuffer bytes (BGRA byte order in memory, i.e.
///   [B,G,R,A] per pixel — see the hook's FBIOGET_VSCREENINFO)
/// - `fb_w`/`fb_h`: source framebuffer dimensions (from profile settings)
/// - `fb_bpp`: bytes per pixel (4)
unsafe fn twrp_blit_to_surface(
    window: *mut c_void,
    fb: &[u8],
    fb_w: usize,
    fb_h: usize,
    fb_bpp: usize,
) -> bool {
    // ANativeWindow functions from libandroid.so (linked via ndk).
    extern "C" {
        fn ANativeWindow_lock(
            window: *mut c_void,
            out_buffer: *mut ANativeWindow_Buffer,
            inOutDirtyBounds: *mut c_void,
        ) -> i32;
        fn ANativeWindow_unlockAndPost(window: *mut c_void) -> i32;
        fn ANativeWindow_setBuffersGeometry(
            window: *mut c_void,
            width: i32,
            height: i32,
            format: i32,
        ) -> i32;
    }

    // ANativeWindow_Buffer struct (from android/native_window.h)
    #[repr(C)]
    struct ANativeWindow_Buffer {
        width: i32,
        height: i32,
        stride: i32, // in pixels, NOT bytes
        format: i32,
        bits: *mut u32, // pointer to the buffer
        reserved: [u32; 6],
    }

    const WINDOW_FORMAT_RGBA_8888: i32 = 1;
    const HAL_PIXEL_FORMAT_BGRA_8888: i32 = 5;

    // Query the window's own size (6-Z183): a recreated/resized surface
    // is handled without trusting the size captured at loop spawn.
    let surface_width = ANativeWindow_getWidth(window);
    let surface_height = ANativeWindow_getHeight(window);
    if surface_width <= 0 || surface_height <= 0 {
        return false;
    }

    // Set the buffer geometry to match the surface dimensions + RGBA8888.
    let r = ANativeWindow_setBuffersGeometry(
        window,
        surface_width,
        surface_height,
        WINDOW_FORMAT_RGBA_8888,
    );
    if r != 0 {
        return false;
    }

    // Lock the window buffer for writing.
    let mut buffer: ANativeWindow_Buffer = std::mem::zeroed();
    let r = ANativeWindow_lock(window, &mut buffer, std::ptr::null_mut());
    if r != 0 {
        return false;
    }

    // Some gralloc implementations keep their preferred format instead of
    // the requested one — honour whatever they actually gave us so the
    // channel packing is always right (6-Z183 color fix, belt + braces).
    let buffer_is_bgra = buffer.format == HAL_PIXEL_FORMAT_BGRA_8888;

    // Blit with nearest-neighbor scaling.
    let src_w = fb_w as i32;
    let src_h = fb_h as i32;
    let dst_w = buffer.width;
    let dst_h = buffer.height;
    let dst_stride = buffer.stride; // in pixels

    let bits = buffer.bits;
    if bits.is_null() {
        let _ = ANativeWindow_unlockAndPost(window);
        return false;
    }

    for dy in 0..dst_h {
        // Map destination y to source y (nearest-neighbor).
        let sy = (dy as u64 * src_h as u64 / dst_h.max(1) as u64) as usize;
        let sy = sy.min(fb_h.saturating_sub(1));

        for dx in 0..dst_w {
            // Map destination x to source x (nearest-neighbor).
            let sx = (dx as u64 * src_w as u64 / dst_w.max(1) as u64) as usize;
            let sx = sx.min(fb_w.saturating_sub(1));

            // Source pixel — fb0 is written by TWRP as in-memory
            // [B,G,R,A]: the hook's FBIOGET_VSCREENINFO declares
            // red.offset=16 / green.offset=8 / blue.offset=0 (exactly
            // what the real byt_t_crv2 Bay Trail panel reports and what
            // this TWRP image renders for).
            //
            // 6-Z183: the destination packing follows the buffer's ACTUAL
            // format — RGBA_8888 wants in-memory [R,G,B,A], BGRA_8888
            // wants [B,G,R,A] (which is exactly fb0's own byte order, so
            // a BGRA buffer takes a verbatim copy).
            let src_idx = (sy * fb_w + sx) * fb_bpp;
            if src_idx + 3 >= fb.len() {
                continue;
            }
            let fb_b = fb[src_idx] as u32;
            let fb_g = fb[src_idx + 1] as u32;
            let fb_r = fb[src_idx + 2] as u32;
            let a = fb[src_idx + 3] as u32;
            let pixel = if buffer_is_bgra {
                // little-endian BGRA_8888: bytes [B,G,R,A]
                (a << 24) | (fb_r << 16) | (fb_g << 8) | fb_b
            } else {
                // little-endian RGBA_8888: bytes [R,G,B,A]
                (a << 24) | (fb_b << 16) | (fb_g << 8) | fb_r
            };

            let dst_idx = (dy as usize * dst_stride as usize) + dx as usize;
            *bits.add(dst_idx) = pixel;
        }
    }

    // Unlock and post the buffer to the display.
    let _ = ANativeWindow_unlockAndPost(window);
    true
}
