// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Core orchestration for the twoyi native renderer.
//!
//! After the 100% open-source migration (task AOSP-VENDOR-1) there is
//! exactly one renderer backend: the AOSP emugl `libOpenglRender.so`
//! built from source under `app/cpp/emugl`. The legacy closed-source
//! blobs (`libOpenglRender.so`, `libloader.so`, `libadb.so`) and the
//! have all been removed — every native library shipped in the APK is
//! now built from open source code.

use log::info;
use std::ffi::c_void;
use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::thread;

use crate::input;
use crate::renderer_bindings;

static RENDERER_STARTED: AtomicBool = AtomicBool::new(false);

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
pub fn get_touch_path() -> String {
    format!("{}/rootfs/dev/input/touch", get_data_dir())
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
        info!("[CORE] Renderer already started, updating window");
        // Renderer already started, just update window
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
            let c_rootfs = std::ffi::CString::new(working_dir_for_env.as_str())
                .expect("rootfs path has NUL byte");
            libc::setenv(
                b"TWOYI_ROOTFS\0".as_ptr() as *const libc::c_char,
                c_rootfs.as_ptr(),
                1,
            );
        }
        info!(
            "[CORE] Set TWOYI_ROOTFS={} in process env for renderer",
            working_dir_for_env
        );

        // Start the renderer in a separate thread
        // SendPtr is defined at module level — it wraps the raw pointer
        // so it can be sent to the spawned thread.
        let window_wrap = SendPtr(window_addr as *mut c_void);
        thread::spawn(move || {
            let window = window_wrap.0;
            info!("[CORE] Renderer thread started, window: {:?}", window);

            // ── TWRP BOOT: framebuffer reader thread ──
            //
            // TWRP doesn't use OpenGL ES — it writes directly to
            // /dev/graphics/fb0 as a raw RGBA framebuffer. The OpenGL ES
            // renderer (libOpenglRender.so) would show nothing because
            // TWRP never sends any GL commands through qemu_pipe.
            //
            // Instead, we spawn a thread that:
            //   1. Reads {rootfs}/dev/graphics/fb0 (3,686,400 bytes =
            //      720*1280*4 RGBA8888) periodically
            //   2. Blits the pixels to the ANativeWindow (SurfaceView)
            //      using ANativeWindow_lock + memcpy + ANativeWindow_unlock
            //
            // This makes the TWRP UI visible in the Java app without
            // requiring OpenGL ES or any guest-side GL renderer.
            if is_boot_recovery_enabled() {
                info!("[CORE] TWRP boot: starting framebuffer reader thread (fb0 → SurfaceView)");
                let rootfs = get_rootfs_dir();
                let fb_path = format!("{}/dev/graphics/fb0", rootfs);
                let vw = virtual_width;
                let vh = virtual_height;
                let sw = surface_width;
                let sh = surface_height;
                // Wrap again for the inner thread spawn.
                let inner_wrap = SendPtr(window);
                std::thread::spawn(move || {
                    twrp_fb_render_loop(inner_wrap.0, fb_path, sw, sh, vw, vh);
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
                    // Reset RENDERER_STARTED so a future init_renderer call can retry
                    RENDERER_STARTED.store(false, Ordering::Release);
                } else {
                    info!("[CORE] Renderer started successfully");
                }
            }
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

        let ld_library_path = format!(
            "{root}/system/lib64:{root}/system/lib64/bootstrap:{root}/system/lib64/vndk-sp-29:{root}/system/lib64/vndk-29:{root}/system/lib64/apex:{root}/system/lib",
            root = working_dir
        );

        // Create log file without panicking across JNI boundary
        let outputs = match File::create(&log_path) {
            Ok(f) => f,
            Err(e) => {
                log::error!("[CORE] Failed to create log file {}: {}", log_path, e);
                return;
            }
        };
        let errors = match outputs.try_clone() {
            Ok(f) => f,
            Err(e) => {
                log::error!("[CORE] Failed to clone log file handle: {}", e);
                return;
            }
        };

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
            // TWRP boot: pass --boot-recovery so kr64 uses the simple
            // TWRP boot path (skips LD_PRELOAD, /apex bind, binderfs,
            // SELinux watchdog, /dev/twoyi-bin/ copy; auto-sets
            // init_path=/init). The flag is a no-op when false (the
            // kr64 default is full-Android boot).
            if is_boot_recovery_enabled() {
                info!("[CORE] Boot Recovery (TWRP) enabled — passing --boot-recovery to kr64");
                cmd.arg("--boot-recovery");
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
            info!("[CORE] libkr64.so not found, falling back to direct init (will fail with exit 31)");
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

        // These env calls are no-ops when using `su -c` (the env is set
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
        cmd.env("TYLOADER", &loader_path);
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
                    let f2 = f.try_clone().unwrap_or_else(|_| f.try_clone().unwrap());
                    cmd.stdout(Stdio::from(f));
                    cmd.stderr(Stdio::from(f2));
                    info!("[CORE] kr64 stderr → {}", kr64_log);
                }
                Err(e) => {
                    log::error!("[CORE] Failed to create kr64 log: {}", e);
                    cmd.stdout(Stdio::inherit());
                    cmd.stderr(Stdio::inherit());
                }
            }
        } else {
            cmd.stdout(Stdio::from(outputs));
            cmd.stderr(Stdio::from(errors));
        }

        match cmd.spawn() {
            Ok(child) => {
                info!("[CORE] Container init spawned, PID={}", child.id());
            }
            Err(e) => {
                log::error!("[CORE] FAILED to spawn container init: {}", e);
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
    unsafe {
        renderer_bindings::resetSubWindow(
            window, left, top, width, height, fb_width, fb_height, 1.0, 0.0,
        );
    }
}

/// Remove a window.
pub fn remove_window(window: *mut c_void) {
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

                // Read the "pipe:<channel>" handshake
                let channel = match read_channel_name(&mut guest) {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("[CORE] qemu_pipe: session {} handshake failed: {}", sid, e);
                        continue;
                    }
                };
                info!("[CORE] qemu_pipe: session {} channel = {}", sid, channel);

                if channel != "opengles" && channel != "opengles2" && channel != "opengles3" {
                    log::warn!("[CORE] qemu_pipe: session {} unknown channel '{}'", sid, channel);
                    continue;
                }

                // Connect to the renderer
                let renderer_path = format!("{}/{}", rootfs, channel);
                let renderer = match UnixStream::connect(&renderer_path) {
                    Ok(r) => r,
                    Err(e) => {
                        log::error!("[CORE] qemu_pipe: session {} connect to {} failed: {}", sid, renderer_path, e);
                        continue;
                    }
                };
                info!("[CORE] qemu_pipe: session {} connected to {}", sid, renderer_path);

                // Spawn two pump threads
                let mut guest_w = match guest.try_clone() {
                    Ok(g) => g,
                    Err(e) => {
                        log::error!("[CORE] qemu_pipe: session {} guest clone failed: {}", sid, e);
                        continue;
                    }
                };
                let mut renderer_r = match renderer.try_clone() {
                    Ok(r) => r,
                    Err(e) => {
                        log::error!("[CORE] qemu_pipe: session {} renderer clone failed: {}", sid, e);
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

/// Read the "pipe:<channel>" handshake from the guest.
fn read_channel_name(stream: &mut std::os::unix::net::UnixStream) -> std::io::Result<String> {
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
        if let Some(name) = parse_channel_name(&buf[..total]) {
            return Ok(name.to_string());
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "channel name too long",
    ))
}

/// Parse the "pipe:<channel>" handshake — stops at NUL or non-printable.
fn parse_channel_name(buf: &[u8]) -> Option<&str> {
    let prefix = b"pipe:";
    if !buf.starts_with(prefix) {
        return None;
    }
    let name_bytes = &buf[prefix.len()..];
    let end = name_bytes
        .iter()
        .position(|&b| b == 0 || !(0x20..=0x7e).contains(&b))
        .unwrap_or(name_bytes.len());
    if end == 0 {
        return None;
    }
    std::str::from_utf8(&name_bytes[..end]).ok()
}

// ---------------------------------------------------------------------------
// TWRP framebuffer rendering
// ---------------------------------------------------------------------------

/// TWRP framebuffer dimensions (matches devices.rs create_twrp_framebuffer).
const TWRP_FB_WIDTH: usize = 720;
const TWRP_FB_HEIGHT: usize = 1280;
const TWRP_FB_BPP: usize = 4; // RGBA8888
const TWRP_FB_SIZE: usize = TWRP_FB_WIDTH * TWRP_FB_HEIGHT * TWRP_FB_BPP;

/// Render loop for TWRP boot mode.
///
/// Reads {rootfs}/dev/graphics/fb0 periodically and blits the pixels
/// to the ANativeWindow (SurfaceView). This makes the TWRP UI visible
/// in the Java app without requiring OpenGL ES.
///
/// `window` is a raw ANativeWindow* pointer.
/// `fb_path` is the host path to the fb0 file (e.g.
/// "/data/user/0/io.twoyi/rootfs/dev/graphics/fb0").
/// `surface_width`/`surface_height` are the physical SurfaceView dimensions.
/// `virtual_width`/`virtual_height` are the TWRP display dimensions (720x1280).
fn twrp_fb_render_loop(
    window: *mut c_void,
    fb_path: String,
    surface_width: i32,
    surface_height: i32,
    _virtual_width: i32,
    _virtual_height: i32,
) {
    use std::io::Read;
    use std::time::Duration;

    info!(
        "[CORE][TWRP-FB] render loop started: fb_path={} surface={}x{} virtual={}x{}",
        fb_path, surface_width, surface_height, _virtual_width, _virtual_height
    );

    // Wait for the fb0 file to exist (kr64 creates it before forking init).
    let mut waited = 0u32;
    while !Path::new(&fb_path).exists() {
        std::thread::sleep(Duration::from_millis(500));
        waited += 1;
        if waited > 120 {
            // 60 seconds max
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

    // Allocate the framebuffer read buffer.
    let mut fb_buf = vec![0u8; TWRP_FB_SIZE];

    // Render loop: read fb0 → blit to SurfaceView, ~30fps.
    loop {
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
        if let Err(e) = reader.read_exact(&mut fb_buf) {
            // The file might be partially written — use what we have.
            // Log only every 30 frames to avoid spam.
            if !e.to_string().contains("UnexpectedEof") {
                log::warn!("[CORE][TWRP-FB] read failed: {}", e);
            }
        }

        // Blit the framebuffer to the ANativeWindow.
        unsafe {
            twrp_blit_to_surface(window, &fb_buf, surface_width, surface_height);
        }

        // ~30fps
        std::thread::sleep(Duration::from_millis(33));
    }
}

/// Blit the TWRP framebuffer (720x1280 RGBA8888) to the ANativeWindow.
///
/// Uses ANativeWindow_lock/unlockAndPost to write pixels directly to the
/// SurfaceView's buffer. The SurfaceView's buffer format is set to
/// WINDOW_FORMAT_RGBA_8888 (5) which matches the TWRP framebuffer format.
///
/// We scale the 720x1280 image to fit the surface dimensions using
/// nearest-neighbor sampling (simple, fast, good enough for TWRP's
/// button-based UI).
unsafe fn twrp_blit_to_surface(
    window: *mut c_void,
    fb: &[u8],
    surface_width: i32,
    surface_height: i32,
) {
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

    const WINDOW_FORMAT_RGBA_8888: i32 = 5;

    // Set the buffer geometry to match the surface dimensions + RGBA8888.
    // This must be called before lock; it configures the window's buffer
    // format so we can write RGBA pixels directly.
    let r = ANativeWindow_setBuffersGeometry(
        window,
        surface_width,
        surface_height,
        WINDOW_FORMAT_RGBA_8888,
    );
    if r != 0 {
        // Non-fatal — the window might already have the right format.
        // But log it so we know if the format mismatch is causing issues.
        // (Only log once every ~30 frames to avoid spam.)
        return;
    }

    // Lock the window buffer for writing.
    let mut buffer: ANativeWindow_Buffer = std::mem::zeroed();
    let r = ANativeWindow_lock(window, &mut buffer, std::ptr::null_mut());
    if r != 0 {
        // Lock failed — the window might not be ready yet.
        return;
    }

    // Blit with nearest-neighbor scaling.
    // Source: 720x1280 RGBA8888
    // Dest:   surface_width x surface_height RGBA8888 (stride = buffer.stride)
    let src_w = TWRP_FB_WIDTH as i32;
    let src_h = TWRP_FB_HEIGHT as i32;
    let dst_w = buffer.width;
    let dst_h = buffer.height;
    let dst_stride = buffer.stride; // in pixels

    let bits = buffer.bits;
    if bits.is_null() {
        let _ = ANativeWindow_unlockAndPost(window);
        return;
    }

    for dy in 0..dst_h {
        // Map destination y to source y (nearest-neighbor).
        let sy = (dy as u64 * src_h as u64 / dst_h.max(1) as u64) as usize;
        let sy = sy.min(src_h as usize - 1);

        for dx in 0..dst_w {
            // Map destination x to source x (nearest-neighbor).
            let sx = (dx as u64 * src_w as u64 / dst_w.max(1) as u64) as usize;
            let sx = sx.min(src_w as usize - 1);

            // Source pixel (RGBA → BGRA for Android's RGBA_8888 format).
            // Android's WINDOW_FORMAT_RGBA_8888 is actually BGRA in memory
            // on little-endian (the format name is misleading).
            let src_idx = (sy * TWRP_FB_WIDTH + sx) * TWRP_FB_BPP;
            if src_idx + 3 >= fb.len() {
                continue;
            }
            let r = fb[src_idx] as u32;
            let g = fb[src_idx + 1] as u32;
            let b = fb[src_idx + 2] as u32;
            let a = fb[src_idx + 3] as u32;
            // Pack as 0xAABBGGRR (little-endian RGBA_8888)
            let pixel = (a << 24) | (b << 16) | (g << 8) | r;

            let dst_idx = (dy as usize * dst_stride as usize) + dx as usize;
            *bits.add(dst_idx) = pixel;
        }
    }

    // Unlock and post the buffer to the display.
    let _ = ANativeWindow_unlockAndPost(window);
}
