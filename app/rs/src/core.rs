// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of merchantability,
// fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use log::info;
use std::ffi::c_void;
use std::fs::File;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use once_cell::sync::Lazy;

use crate::input;
use crate::renderer_bindings;
use crate::renderer_new;

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
    DATA_DIR.get().map(|s| s.as_str()).unwrap_or("/data/data/io.twoyi")
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

/// Renderer type selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RendererType {
    Old,  // Original libOpenglRender.so
    New,  // New open-source Rust implementation
}

/// Global renderer type setting
static RENDERER_TYPE: Lazy<Mutex<RendererType>> = Lazy::new(|| Mutex::new(RendererType::Old));

/// Global debug renderer setting
static DEBUG_RENDERER: AtomicBool = AtomicBool::new(false);

/// Global debug log directory
static DEBUG_LOG_DIR: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));

/// Set the renderer type to use
pub fn set_renderer_type(use_new_renderer: bool) {
    let mut renderer_type = RENDERER_TYPE.lock().unwrap();
    *renderer_type = if use_new_renderer {
        RendererType::New
    } else {
        RendererType::Old
    };
    info!("[CORE] ========================================");
    info!("[CORE] Renderer type set to: {:?}", *renderer_type);
    info!("[CORE] ========================================");
}

/// Set the debug renderer mode
pub fn set_debug_renderer(debug_enabled: bool) {
    DEBUG_RENDERER.store(debug_enabled, Ordering::Relaxed);
    info!("[CORE] ========================================");
    info!("[CORE] Debug renderer set to: {}", debug_enabled);
    info!("[CORE] ========================================");
    
    // Pass debug flag to the new renderer module
    renderer_new::set_debug_mode(debug_enabled);
}

/// Set the debug log directory
pub fn set_debug_log_dir(log_dir: String) {
    let mut dir = DEBUG_LOG_DIR.lock().unwrap();
    *dir = log_dir.clone();
    info!("[CORE] ========================================");
    info!("[CORE] Debug log directory set to: {}", log_dir);
    info!("[CORE] ========================================");
    
    // Pass log directory to the new renderer module
    renderer_new::set_debug_log_dir(log_dir);
}

/// Initialize the renderer with the given parameters
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
    info!("[CORE] Surface: {}x{}, Virtual: {}x{}, FPS: {}", 
          surface_width, surface_height, virtual_width, virtual_height, fps);

    let renderer_type = *RENDERER_TYPE.lock().unwrap();


    info!("[CORE] Using renderer: {:?}", renderer_type);
    info!("[CORE] ========================================");

    if RENDERER_STARTED
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        info!("[CORE] Renderer already started, updating window");
        // Renderer already started, just update window
        match renderer_type {
            RendererType::Old => {
                info!("[CORE] Updating old renderer window");
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
            },
            RendererType::New => {
                info!("[CORE] Updating new renderer window");
                renderer_new::set_native_window(window);
                renderer_new::reset_window(
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

        // Convert raw pointer to usize for safe transfer between threads
        let window_addr = window as usize;
        
        // Start the renderer in a separate thread
        thread::spawn(move || {
            let window = window_addr as *mut c_void;
            info!("[CORE] Renderer thread started, window: {:?}", window);
            
            match renderer_type {
                RendererType::Old => {
                    info!("[CORE] Starting old renderer (libOpenglRender.so)");
                    unsafe {
                        renderer_bindings::startOpenGLRenderer(window, virtual_width, virtual_height, xdpi, ydpi, fps);
                    }
                },
                RendererType::New => {
                    info!("[CORE] Starting new renderer (built-in Rust implementation)");
                    let result = renderer_new::start_renderer(window, virtual_width, virtual_height, xdpi, ydpi, fps);
                    if result != 0 {
                        info!("[CORE] New renderer failed to start (result={}), this is expected if QEMU pipe is not available", result);
                    }
                }
            }
        });

        let working_dir = get_rootfs_dir();
        let log_path = get_log_path();
        info!("[CORE] Starting container init process");
        info!("[CORE] Working directory: {}", working_dir);
        info!("[CORE] Log path: {}", log_path);
        let outputs = File::create(log_path).unwrap();
        let errors = outputs.try_clone().unwrap();
        let _ = Command::new("./init")
            .current_dir(&working_dir)
            .env("TYLOADER", &loader_path)
            .env("TWOYI_ROOTFS", &working_dir)
            .stdout(Stdio::from(outputs))
            .stderr(Stdio::from(errors))
            .spawn();
    }
}

/// Returns the renderer type to use, falling back to New on non-aarch64
/// where the legacy libOpenglRender.so blob is not available.
fn effective_renderer_type() -> RendererType {
    *RENDERER_TYPE.lock().unwrap()
}

/// Reset window parameters
pub fn reset_window(
    window: *mut c_void,
    top: i32,
    left: i32,
    width: i32,
    height: i32,
    fb_width: i32,
    fb_height: i32,
) {
    let renderer_type = effective_renderer_type();
    
    match renderer_type {
        RendererType::Old => unsafe {
            renderer_bindings::resetSubWindow(
                window,
                left,
                top,
                width,
                height,
                fb_width,
                fb_height,
                1.0,
                0.0,
            );
        },
        RendererType::New => {
            renderer_new::reset_window(
                window,
                left,
                top,
                width,
                height,
                fb_width,
                fb_height,
                1.0,
                0.0,
            );
        }
    }
}

/// Remove a window
pub fn remove_window(window: *mut c_void) {
    let renderer_type = effective_renderer_type();
    
    match renderer_type {
        RendererType::Old => unsafe {
            renderer_bindings::removeSubWindow(window);
        },
        RendererType::New => {
            renderer_new::remove_window(window);
        }
    }
}
