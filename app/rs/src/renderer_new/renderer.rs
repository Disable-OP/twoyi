// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of merchantability,
// fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! New open-source OpenGL renderer implementation
//! 
//! This module provides the main renderer interface that mimics the old
//! renderer API while using the new open-source implementation.

use log::{debug, error, info, warn};
use std::ffi::c_void;
use std::sync::Mutex;
use once_cell::sync::Lazy;

use super::opengles::GLContext;

/// Global renderer state
static RENDERER: Lazy<Mutex<Option<RendererState>>> = Lazy::new(|| Mutex::new(None));

struct RendererState {
    gl_context: GLContext,
    window: *mut c_void,
    width: i32,
    height: i32,
}

// Mark RendererState as Send since we're controlling access via Mutex
unsafe impl Send for RendererState {}

/// Start the OpenGL renderer
/// 
/// This function mimics the old `startOpenGLRenderer` API
pub fn start_renderer(
    window: *mut c_void,
    width: i32,
    height: i32,
    xdpi: i32,
    ydpi: i32,
    fps: i32,
) -> i32 {
    info!("[NEW_RENDERER] ========================================");
    info!("[NEW_RENDERER] Starting new OpenGL renderer");
    info!("[NEW_RENDERER] Window: {:?}, Dimensions: {}x{}", window, width, height);
    info!("[NEW_RENDERER] DPI: {}x{}, FPS: {}", xdpi, ydpi, fps);
    info!("[NEW_RENDERER] ========================================");
    
    // Check if pipe is available
    debug!("[NEW_RENDERER] Checking QEMU pipe availability...");
    if !super::pipe::is_pipe_available() {
        error!("[NEW_RENDERER] QEMU pipe device not available");
        error!("[NEW_RENDERER] Falling back to old renderer");
        return -1;
    }
    info!("[NEW_RENDERER] QEMU pipe device is available");
    
    // Create GL context
    debug!("[NEW_RENDERER] Creating GL context...");
    let mut gl_context = match GLContext::new() {
        Ok(ctx) => {
            info!("[NEW_RENDERER] GL context created successfully");
            ctx
        },
        Err(e) => {
            error!("[NEW_RENDERER] Failed to create GL context: {}", e);
            error!("[NEW_RENDERER] Falling back to old renderer");
            return -1;
        }
    };
    
    // Initialize the context
    debug!("[NEW_RENDERER] Initializing GL context...");
    if let Err(e) = gl_context.initialize(width, height, xdpi, ydpi, fps) {
        error!("[NEW_RENDERER] Failed to initialize GL context: {}", e);
        error!("[NEW_RENDERER] Falling back to old renderer");
        return -1;
    }
    info!("[NEW_RENDERER] GL context initialized successfully");
    
    // Store the renderer state
    debug!("[NEW_RENDERER] Storing renderer state...");
    let state = RendererState {
        gl_context,
        window,
        width,
        height,
    };
    
    let mut renderer = RENDERER.lock().unwrap();
    *renderer = Some(state);
    
    info!("[NEW_RENDERER] ========================================");
    info!("[NEW_RENDERER] New OpenGL renderer started successfully!");
    info!("[NEW_RENDERER] ========================================");
    0
}

/// Set or update the native window
/// 
/// This function mimics the old `setNativeWindow` API
pub fn set_native_window(window: *mut c_void) -> i32 {
    info!("[NEW_RENDERER] Setting native window: {:?}", window);
    
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(state) = renderer.as_mut() {
        debug!("[NEW_RENDERER] Updating window pointer in renderer state");
        state.window = window;
        info!("[NEW_RENDERER] Native window updated successfully");
        0
    } else {
        warn!("[NEW_RENDERER] Renderer not initialized, cannot set window");
        -1
    }
}

/// Reset subwindow parameters
/// 
/// This function mimics the old `resetSubWindow` API
pub fn reset_window(
    window: *mut c_void,
    _wx: i32,
    _wy: i32,
    ww: i32,
    wh: i32,
    fbw: i32,
    fbh: i32,
    _dpr: f32,
    _z_rot: f32,
) -> i32 {
    info!("[NEW_RENDERER] Resetting window");
    info!("[NEW_RENDERER] Window: {:?}", window);
    info!("[NEW_RENDERER] Surface: {}x{}, Framebuffer: {}x{}", ww, wh, fbw, fbh);
    
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(state) = renderer.as_mut() {
        debug!("[NEW_RENDERER] Updating window pointer");
        state.window = window;
        state.width = fbw;
        state.height = fbh;
        
        debug!("[NEW_RENDERER] Setting window size in GL context...");
        if let Err(e) = state.gl_context.set_window_size(ww, wh, fbw, fbh) {
            error!("[NEW_RENDERER] Failed to set window size: {}", e);
            return -1;
        }
        
        info!("[NEW_RENDERER] Window reset successfully");
        0
    } else {
        warn!("[NEW_RENDERER] Renderer not initialized");
        -1
    }
}

/// Remove subwindow
/// 
/// This function mimics the old `removeSubWindow` API
pub fn remove_window(_window: *mut c_void) -> i32 {
    info!("[NEW_RENDERER] Removing window: {:?}", _window);
    
    let renderer = RENDERER.lock().unwrap();
    if renderer.is_some() {
        debug!("[NEW_RENDERER] Renderer still active, window removal acknowledged");
        // Keep the renderer alive but acknowledge the window removal
        0
    } else {
        warn!("[NEW_RENDERER] Renderer not initialized");
        -1
    }
}

/// Destroy the OpenGL subwindow
/// 
/// This function mimics the old `destroyOpenGLSubwindow` API
#[allow(dead_code)]
pub fn destroy_subwindow() -> i32 {
    info!("[NEW_RENDERER] Destroying OpenGL subwindow");
    
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(mut state) = renderer.take() {
        debug!("[NEW_RENDERER] Destroying GL context...");
        if let Err(e) = state.gl_context.destroy() {
            error!("[NEW_RENDERER] Failed to destroy GL context: {}", e);
            return -1;
        }
        info!("[NEW_RENDERER] OpenGL subwindow destroyed successfully");
        0
    } else {
        warn!("[NEW_RENDERER] Renderer not initialized");
        -1
    }
}

/// Repaint the OpenGL display
/// 
/// This function mimics the old `repaintOpenGLDisplay` API
#[allow(dead_code)]
pub fn repaint_display() {
    debug!("[NEW_RENDERER] Repainting display");
    
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(state) = renderer.as_mut() {
        if let Err(e) = state.gl_context.repaint() {
            error!("[NEW_RENDERER] Failed to repaint display: {}", e);
        } else {
            debug!("[NEW_RENDERER] Display repainted successfully");
        }
    } else {
        warn!("[NEW_RENDERER] Renderer not initialized - cannot repaint");
    }
}
