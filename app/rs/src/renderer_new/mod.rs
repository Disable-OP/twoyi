// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of merchantability,
// fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! New open-source OpenGL renderer module
//! 
//! This module implements an open-source OpenGL ES renderer that communicates
//! with the container via QEMU pipes, similar to the Anbox implementation.
//! It also integrates with Android's gralloc system for proper graphics
//! buffer management.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use once_cell::sync::Lazy;

/// Global debug mode flag for renderer
pub static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

/// Global debug log directory
pub static DEBUG_LOG_DIR: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::from("twoyi_renderer_debug")));

pub mod pipe;
pub mod opengles;
pub mod gralloc;
pub mod renderer;
pub mod socket_monitor;

pub use renderer::{
    start_renderer,
    reset_window,
    remove_window,
    set_native_window,
};

/// Set the debug mode for the renderer
pub fn set_debug_mode(enabled: bool) {
    DEBUG_MODE.store(enabled, Ordering::Relaxed);
    log::info!("[NEW_RENDERER] ========================================");
    log::info!("[NEW_RENDERER] Debug mode set to: {}", enabled);
    log::info!("[NEW_RENDERER] Current log directory: {}", get_debug_log_dir());
    log::info!("[NEW_RENDERER] ========================================");
    
    // Start socket monitoring if debug mode is enabled
    if enabled {
        log::info!("[NEW_RENDERER] Starting socket monitoring for debug mode");
        socket_monitor::start_socket_monitoring();
    }
}

/// Set the debug log directory
pub fn set_debug_log_dir(log_dir: String) {
    let mut dir = DEBUG_LOG_DIR.lock().unwrap();
    *dir = log_dir.clone();
    log::info!("[NEW_RENDERER] Debug log directory set to: {}", log_dir);
}

/// Check if debug mode is enabled
pub fn is_debug_mode() -> bool {
    DEBUG_MODE.load(Ordering::Relaxed)
}

/// Get the debug log directory
pub fn get_debug_log_dir() -> String {
    DEBUG_LOG_DIR.lock().unwrap().clone()
}
