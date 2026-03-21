// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of merchantability,
// fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Open-source OpenGL Renderer Library
//! 
//! This library provides a complete open-source replacement for the legacy
//! libOpenglRender.so. It implements the same C API but uses a modern Rust
//! implementation based on QEMU pipes and OpenGL ES protocol.

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;
use log::{info, warn, error};

mod pipe;
mod opengles;
mod gralloc;

use opengles::GLContext;
use gralloc::GrallocManager;

/// Global debug mode flag for renderer
static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

/// Global debug log directory
static DEBUG_LOG_DIR: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::from("/data/local/tmp/twoyi_renderer_debug")));

/// Check if debug mode is enabled
pub fn is_debug_mode() -> bool {
    DEBUG_MODE.load(Ordering::Relaxed)
}

/// Get the debug log directory
pub fn get_debug_log_dir() -> String {
    DEBUG_LOG_DIR.lock().unwrap().clone()
}

/// Global renderer state
static RENDERER: Lazy<Mutex<Option<RendererState>>> = Lazy::new(|| Mutex::new(None));

struct RendererState {
    gl_context: GLContext,
    gralloc_manager: Option<GrallocManager>,
    window: *mut c_void,
    width: i32,
    height: i32,
}

// Mark RendererState as Send since we're controlling access via Mutex
unsafe impl Send for RendererState {}

/// Start the OpenGL renderer
/// 
/// This matches the signature of the legacy startOpenGLRenderer function
#[no_mangle]
pub extern "C" fn startOpenGLRenderer(
    win: *mut c_void,
    width: i32,
    height: i32,
    xdpi: i32,
    ydpi: i32,
    fps: i32,
) -> i32 {
    info!("[OPENGL_RENDERER] ========================================");
    info!("[OPENGL_RENDERER] Starting OpenGL renderer");
    info!("[OPENGL_RENDERER] Window: {:?}, Dimensions: {}x{}", win, width, height);
    info!("[OPENGL_RENDERER] DPI: {}x{}, FPS: {}", xdpi, ydpi, fps);
    info!("[OPENGL_RENDERER] ========================================");
    
    // Check if pipe is available
    if !pipe::is_pipe_available() {
        error!("[OPENGL_RENDERER] QEMU pipe device not available");
        return -1;
    }
    info!("[OPENGL_RENDERER] QEMU pipe device is available");
    
    // Create GL context
    let mut gl_context = match GLContext::new() {
        Ok(ctx) => {
            info!("[OPENGL_RENDERER] GL context created successfully");
            ctx
        },
        Err(e) => {
            error!("[OPENGL_RENDERER] Failed to create GL context: {}", e);
            return -1;
        }
    };
    
    // Initialize the context
    if let Err(e) = gl_context.initialize(width, height, xdpi, ydpi, fps) {
        error!("[OPENGL_RENDERER] Failed to initialize GL context: {}", e);
        return -1;
    }
    info!("[OPENGL_RENDERER] GL context initialized successfully");
    
    // Initialize gralloc manager for buffer management
    let gralloc_manager = match GrallocManager::new(win, width, height) {
        Ok(manager) => {
            info!("[OPENGL_RENDERER] Gralloc manager initialized successfully");
            Some(manager)
        },
        Err(e) => {
            warn!("[OPENGL_RENDERER] Failed to initialize gralloc manager: {}", e);
            warn!("[OPENGL_RENDERER] Continuing without gralloc buffer management");
            None
        }
    };
    
    // Store the renderer state
    let state = RendererState {
        gl_context,
        gralloc_manager,
        window: win,
        width,
        height,
    };
    
    let mut renderer = RENDERER.lock().unwrap();
    *renderer = Some(state);
    
    info!("[OPENGL_RENDERER] OpenGL renderer started successfully!");
    0
}

/// Set or update the native window
#[no_mangle]
pub extern "C" fn setNativeWindow(arg1: *mut c_void) -> i32 {
    info!("[OPENGL_RENDERER] Setting native window: {:?}", arg1);
    
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(state) = renderer.as_mut() {
        state.window = arg1;
        
        if let Some(ref gralloc) = state.gralloc_manager {
            info!("[OPENGL_RENDERER] Buffer size: {}x{}", gralloc.get_size().0, gralloc.get_size().1);
        }
        
        info!("[OPENGL_RENDERER] Native window updated successfully");
        0
    } else {
        warn!("[OPENGL_RENDERER] Renderer not initialized");
        -1
    }
}

/// Reset subwindow parameters
#[no_mangle]
pub extern "C" fn resetSubWindow(
    p_window: *mut c_void,
    wx: i32,
    wy: i32,
    ww: i32,
    wh: i32,
    fbw: i32,
    fbh: i32,
    dpr: f32,
    z_rot: f32,
) -> i32 {
    info!("[OPENGL_RENDERER] Resetting window");
    info!("[OPENGL_RENDERER] Window: {:?}, pos=({},{})", p_window, wx, wy);
    info!("[OPENGL_RENDERER] Surface: {}x{}, Framebuffer: {}x{}", ww, wh, fbw, fbh);
    info!("[OPENGL_RENDERER] DPR: {}, Z-Rotation: {}", dpr, z_rot);
    
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(state) = renderer.as_mut() {
        state.window = p_window;
        state.width = fbw;
        state.height = fbh;
        
        // Update gralloc manager buffer size if available
        if let Some(ref mut gralloc) = state.gralloc_manager {
            if let Err(e) = gralloc.set_size(fbw, fbh) {
                error!("[OPENGL_RENDERER] Failed to update gralloc buffer size: {}", e);
            } else {
                info!("[OPENGL_RENDERER] Gralloc buffer size updated successfully");
            }
        }
        
        if let Err(e) = state.gl_context.set_window_size(ww, wh, fbw, fbh) {
            error!("[OPENGL_RENDERER] Failed to set window size: {}", e);
            return -1;
        }
        
        info!("[OPENGL_RENDERER] Window reset successfully");
        0
    } else {
        warn!("[OPENGL_RENDERER] Renderer not initialized");
        -1
    }
}

/// Remove subwindow
#[no_mangle]
pub extern "C" fn removeSubWindow(arg1: *mut c_void) -> i32 {
    info!("[OPENGL_RENDERER] Removing window: {:?}", arg1);
    
    let renderer = RENDERER.lock().unwrap();
    if renderer.is_some() {
        // Keep the renderer alive but acknowledge the window removal
        0
    } else {
        warn!("[OPENGL_RENDERER] Renderer not initialized");
        -1
    }
}

/// Destroy the OpenGL subwindow
#[no_mangle]
pub extern "C" fn destroyOpenGLSubwindow() -> i32 {
    info!("[OPENGL_RENDERER] Destroying OpenGL subwindow");
    
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(mut state) = renderer.take() {
        if let Err(e) = state.gl_context.destroy() {
            error!("[OPENGL_RENDERER] Failed to destroy GL context: {}", e);
            return -1;
        }
        info!("[OPENGL_RENDERER] OpenGL subwindow destroyed successfully");
        0
    } else {
        warn!("[OPENGL_RENDERER] Renderer not initialized");
        -1
    }
}

/// Repaint the OpenGL display
#[no_mangle]
pub extern "C" fn repaintOpenGLDisplay() {
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(state) = renderer.as_mut() {
        // Use gralloc buffer management for repaint if available
        if let Some(ref gralloc) = state.gralloc_manager {
            if let Ok(buffer) = gralloc.lock_buffer() {
                // Buffer locked successfully
                let _ = gralloc.unlock_and_post();
            }
        }
        
        // Perform the actual GL repaint
        if let Err(e) = state.gl_context.repaint() {
            error!("[OPENGL_RENDERER] Failed to repaint display: {}", e);
        }
    } else {
        warn!("[OPENGL_RENDERER] Renderer not initialized - cannot repaint");
    }
}
