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

        // Start the renderer in a separate thread
        thread::spawn(move || {
            let window = window_addr as *mut c_void;
            info!("[CORE] Renderer thread started, window: {:?}", window);
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
        });

        let working_dir = get_rootfs_dir();
        let log_path = get_log_path();
        info!("[CORE] Starting container init process");
        info!("[CORE] Working directory: {}", working_dir);
        info!("[CORE] Log path: {}", log_path);

        // -----------------------------------------------------------------
        // Init spawn strategy (post-2026-08 fix):
        //
        // The naive `Command::new("./init")` approach fails because init's
        // INTERP segment is `/system/bin/bootstrap/linker64`, which resolves
        // to the HOST linker. The host linker then loads init's NEEDED
        // libraries (libc.so, libbase, ...) from the HOST /system/lib64/,
        // producing a zombie init that can't do PID 1 operations.
        //
        // The fix: exec the ROOTFS linker directly, with init as its
        // argument. The rootfs linker is a static PIE (it is its own
        // interpreter), so the kernel doesn't read init's INTERP at all.
        // We pass --library-path so the linker resolves init's deps from
        // the rootfs's /system/lib64/, not the host's.
        //
        // This works WITHOUT SELinux permissive because:
        //   - The rootfs linker file is in the app's data dir (app_data_file
        //     context), which the app can execute.
        //   - We never touch /system/bin/linker64 on the host.
        //   - The kernel only needs exec permission on the linker binary.
        //
        // If the rootfs linker is missing (e.g. older Android 8 rootfs),
        // fall back to the loader64 (libloader.so) approach, which dlopens
        // init — but this loads HOST libs and is expected to fail.
        // -----------------------------------------------------------------
        let init_path = format!("{}/init", working_dir);

        // Android 10+ uses /system/bin/bootstrap/linker64 (Treble split).
        // Android 8/9 uses /system/bin/linker64.
        let bootstrap_linker = format!("{}/system/bin/bootstrap/linker64", working_dir);
        let legacy_linker = format!("{}/system/bin/linker64", working_dir);

        let linker_path = if Path::new(&bootstrap_linker).exists() {
            info!("[CORE] Using rootfs bootstrap linker: {}", bootstrap_linker);
            bootstrap_linker
        } else if Path::new(&legacy_linker).exists() {
            info!("[CORE] Using rootfs legacy linker: {}", legacy_linker);
            legacy_linker
        } else {
            info!("[CORE] No rootfs linker found, falling back to loader64 (host libs — may fail)");
            loader_path.clone()
        };

        // LD_LIBRARY_PATH: where the linker looks for init's NEEDED libs.
        // We point it at the rootfs's lib64 dirs so init gets ROOTFS libs.
        let ld_library_path = format!(
            "{root}/system/lib64:{root}/system/lib64/bootstrap:{root}/system/lib64/vndk-sp-29:{root}/system/lib64/vndk-29:{root}/system/lib64/apex:{root}/system/lib",
            root = working_dir
        );

        info!("[CORE] Init path: {}", init_path);
        info!("[CORE] Linker: {}", linker_path);
        info!("[CORE] LD_LIBRARY_PATH: {}", ld_library_path);

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

        // Build the command. If we're using the rootfs linker, invoke it as:
        //   <linker> --library-path <libs> <init>
        // Otherwise (loader64 fallback): <loader64> <init>
        let mut cmd = Command::new(&linker_path);
        cmd.current_dir(&working_dir);

        if linker_path == loader_path {
            // loader64 fallback: just pass init as arg
            cmd.arg(&init_path);
        } else {
            // rootfs linker: pass --library-path then init
            cmd.arg("--library-path").arg(&ld_library_path);
            cmd.arg(&init_path);
        }

        cmd.env("LD_LIBRARY_PATH", &ld_library_path);
        cmd.env_remove("LD_PRELOAD");
        cmd.env("TWOYI_ROOTFS", &working_dir);
        cmd.env("TYLOADER", &loader_path);
        cmd.env("ANDROID_BOOTLOGO", "1");
        // Point ANDROID_ROOT/ANDROID_DATA at the rootfs, not the host filesystem
        cmd.env("ANDROID_ROOT", format!("{}/system", working_dir));
        cmd.env("ANDROID_DATA", format!("{}/data", working_dir));
        // Remove these so init/zygote computes them from the rootfs config
        cmd.env_remove("BOOTCLASSPATH");
        cmd.env_remove("SYSTEMSERVERCLASSPATH");
        cmd.stdout(Stdio::from(outputs));
        cmd.stderr(Stdio::from(errors));

        match cmd.spawn() {
            Ok(child) => {
                info!("[CORE] Container init spawned, PID={}", child.id());
            }
            Err(e) => {
                log::error!("[CORE] FAILED to spawn container init: {}", e);
                log::error!(
                    "[CORE]   linker_path: {} (exists: {})",
                    linker_path,
                    Path::new(&linker_path).exists()
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
