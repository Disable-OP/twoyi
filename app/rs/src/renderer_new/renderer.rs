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
    info!("Starting new OpenGL renderer: {}x{} @ {} fps", width, height, fps);
    
    // Check if pipe is available
    if !super::pipe::is_pipe_available() {
        error!("QEMU pipe device not available - falling back to old renderer");
        return -1;
    }
    
    // Create GL context
    let mut gl_context = match GLContext::new() {
        Ok(ctx) => ctx,
        Err(e) => {
            error!("Failed to create GL context: {}", e);
            return -1;
        }
    };
    
    // Initialize the context
    if let Err(e) = gl_context.initialize(width, height, xdpi, ydpi, fps) {
        error!("Failed to initialize GL context: {}", e);
        return -1;
    }
    
    // Store the renderer state
    let state = RendererState {
        gl_context,
        window,
        width,
        height,
    };
    
    let mut renderer = RENDERER.lock().unwrap();
    *renderer = Some(state);
    
    info!("New OpenGL renderer started successfully");
    0
}

/// Set or update the native window
/// 
/// This function mimics the old `setNativeWindow` API
pub fn set_native_window(window: *mut c_void) -> i32 {
    debug!("Setting native window");
    
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(state) = renderer.as_mut() {
        state.window = window;
        0
    } else {
        warn!("Renderer not initialized");
        -1
    }
}

/// Reset subwindow parameters
/// 
/// This function mimics the old `resetSubWindow` API
pub fn reset_window(
    window: *mut c_void,
    wx: i32,
    wy: i32,
    ww: i32,
    wh: i32,
    fbw: i32,
    fbh: i32,
    dpr: f32,
    z_rot: f32,
) -> i32 {
    debug!("Resetting window: surface={}x{}, framebuffer={}x{}", ww, wh, fbw, fbh);
    
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(state) = renderer.as_mut() {
        state.window = window;
        state.width = fbw;
        state.height = fbh;
        
        if let Err(e) = state.gl_context.set_window_size(ww, wh, fbw, fbh) {
            error!("Failed to set window size: {}", e);
            return -1;
        }
        
        0
    } else {
        warn!("Renderer not initialized");
        -1
    }
}

/// Remove subwindow
/// 
/// This function mimics the old `removeSubWindow` API
pub fn remove_window(window: *mut c_void) -> i32 {
    debug!("Removing window");
    
    let mut renderer = RENDERER.lock().unwrap();
    if renderer.is_some() {
        // Keep the renderer alive but acknowledge the window removal
        0
    } else {
        warn!("Renderer not initialized");
        -1
    }
}

/// Destroy the OpenGL subwindow
/// 
/// This function mimics the old `destroyOpenGLSubwindow` API
pub fn destroy_subwindow() -> i32 {
    info!("Destroying OpenGL subwindow");
    
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(mut state) = renderer.take() {
        if let Err(e) = state.gl_context.destroy() {
            error!("Failed to destroy GL context: {}", e);
            return -1;
        }
        0
    } else {
        warn!("Renderer not initialized");
        -1
    }
}

/// Repaint the OpenGL display
/// 
/// This function mimics the old `repaintOpenGLDisplay` API
pub fn repaint_display() {
    debug!("Repainting display");
    
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(state) = renderer.as_mut() {
        if let Err(e) = state.gl_context.repaint() {
            error!("Failed to repaint display: {}", e);
        }
    } else {
        warn!("Renderer not initialized - cannot repaint");
    }
}
