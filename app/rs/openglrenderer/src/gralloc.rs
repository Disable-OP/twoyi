// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of merchantability,
// fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Gralloc buffer management for the new renderer
//!
//! This module provides graphics buffer allocation and management using
//! Android's ANativeWindow API, which interfaces with the gralloc HAL.

use log::{debug, error, info};
use std::ffi::c_void;
use std::io;
use ndk_sys::{
    ANativeWindow, ANativeWindow_Buffer, ANativeWindow_acquire,
    ANativeWindow_release, ANativeWindow_setBuffersGeometry,
    ANativeWindow_lock, ANativeWindow_unlockAndPost,
};

/// Gralloc buffer manager
///
/// This structure manages graphics buffers using Android's native window
/// and gralloc system for proper buffer allocation and rendering.
pub struct GrallocManager {
    window: *mut ANativeWindow,
    width: i32,
    height: i32,
    format: i32,
}

// ANativeWindow is thread-safe when properly reference counted
unsafe impl Send for GrallocManager {}
unsafe impl Sync for GrallocManager {}

impl GrallocManager {
    /// Create a new gralloc manager with the given native window
    pub fn new(window: *mut c_void, width: i32, height: i32) -> io::Result<Self> {
        if window.is_null() {
            error!("[NEW_RENDERER][GRALLOC] Window pointer is null");
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Null window pointer",
            ));
        }

        let native_window = window as *mut ANativeWindow;
        
        info!("[NEW_RENDERER][GRALLOC] Creating gralloc manager");
        info!("[NEW_RENDERER][GRALLOC] Window: {:?}, Size: {}x{}", native_window, width, height);
        
        // Acquire the window to increase reference count
        unsafe {
            debug!("[NEW_RENDERER][GRALLOC] Acquiring native window reference");
            ANativeWindow_acquire(native_window);
        }
        
        // WINDOW_FORMAT_RGBA_8888 = 1
        let format = 1;
        
        let manager = GrallocManager {
            window: native_window,
            width,
            height,
            format,
        };
        
        // Configure buffer geometry
        manager.configure_buffers()?;
        
        info!("[NEW_RENDERER][GRALLOC] Gralloc manager created successfully");
        Ok(manager)
    }
    
    /// Configure buffer geometry for the native window
    fn configure_buffers(&self) -> io::Result<()> {
        debug!("[NEW_RENDERER][GRALLOC] Configuring buffer geometry");
        debug!("[NEW_RENDERER][GRALLOC] Size: {}x{}, Format: {}", self.width, self.height, self.format);
        
        let result = unsafe {
            ANativeWindow_setBuffersGeometry(
                self.window,
                self.width,
                self.height,
                self.format,
            )
        };
        
        if result != 0 {
            error!("[NEW_RENDERER][GRALLOC] Failed to set buffer geometry: {}", result);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("ANativeWindow_setBuffersGeometry failed: {}", result),
            ));
        }
        
        info!("[NEW_RENDERER][GRALLOC] Buffer geometry configured successfully");
        Ok(())
    }
    
    /// Update window size and reconfigure buffers
    pub fn set_size(&mut self, width: i32, height: i32) -> io::Result<()> {
        info!("[NEW_RENDERER][GRALLOC] Updating window size to {}x{}", width, height);
        
        self.width = width;
        self.height = height;
        
        self.configure_buffers()?;
        
        info!("[NEW_RENDERER][GRALLOC] Window size updated successfully");
        Ok(())
    }
    
    /// Lock a buffer for CPU access
    ///
    /// This acquires a buffer from the gralloc system that can be written to.
    /// The buffer must be unlocked with `unlock_and_post` when done.
    pub fn lock_buffer(&self) -> io::Result<ANativeWindow_Buffer> {
        debug!("[NEW_RENDERER][GRALLOC] Locking buffer for CPU access");
        
        let mut buffer: ANativeWindow_Buffer = unsafe { std::mem::zeroed() };
        
        let result = unsafe {
            ANativeWindow_lock(self.window, &mut buffer as *mut _, std::ptr::null_mut())
        };
        
        if result != 0 {
            error!("[NEW_RENDERER][GRALLOC] Failed to lock buffer: {}", result);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("ANativeWindow_lock failed: {}", result),
            ));
        }
        
        debug!("[NEW_RENDERER][GRALLOC] Buffer locked successfully");
        debug!("[NEW_RENDERER][GRALLOC] Buffer: {}x{}, stride: {}, format: {}",
               buffer.width, buffer.height, buffer.stride, buffer.format);
        
        // Dump buffer info to debug log
        if super::is_debug_mode() {
            self.dump_buffer_info("lock", &buffer);
        }
        
        Ok(buffer)
    }
    
    /// Unlock the buffer and post it for display
    ///
    /// This releases the buffer back to the gralloc system and queues it
    /// for display by the compositor.
    pub fn unlock_and_post(&self) -> io::Result<()> {
        debug!("[NEW_RENDERER][GRALLOC] Unlocking buffer and posting for display");
        
        // Dump unlock event to debug log
        if super::is_debug_mode() {
            self.dump_gralloc_event("unlock_and_post");
        }
        
        let result = unsafe {
            ANativeWindow_unlockAndPost(self.window)
        };
        
        if result != 0 {
            error!("[NEW_RENDERER][GRALLOC] Failed to unlock and post buffer: {}", result);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("ANativeWindow_unlockAndPost failed: {}", result),
            ));
        }
        
        debug!("[NEW_RENDERER][GRALLOC] Buffer unlocked and posted successfully");
        Ok(())
    }
    
    /// Get the current window dimensions
    pub fn get_size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
    
    /// Get the buffer format
    #[allow(dead_code)]
    pub fn get_format(&self) -> i32 {
        self.format
    }
    
    /// Dump buffer info to debug log
    fn dump_buffer_info(&self, event: &str, buffer: &ANativeWindow_Buffer) {
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        let log_dir = super::get_debug_log_dir();
        let _ = std::fs::create_dir_all(&log_dir);
        let log_path = format!("{}/gralloc_buffers.log", log_dir);
        
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = writeln!(file, "[{}] BUFFER_{}: width={}, height={}, stride={}, format={}", 
                            timestamp, event.to_uppercase(), buffer.width, buffer.height, buffer.stride, buffer.format);
        }
    }
    
    /// Dump gralloc event to debug log
    fn dump_gralloc_event(&self, event: &str) {
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        let log_dir = super::get_debug_log_dir();
        let _ = std::fs::create_dir_all(&log_dir);
        let log_path = format!("{}/gralloc_events.log", log_dir);
        
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = writeln!(file, "[{}] {}", timestamp, event.to_uppercase());
        }
    }
}

impl Drop for GrallocManager {
    fn drop(&mut self) {
        info!("[NEW_RENDERER][GRALLOC] Dropping gralloc manager");
        
        if !self.window.is_null() {
            unsafe {
                debug!("[NEW_RENDERER][GRALLOC] Releasing native window reference");
                ANativeWindow_release(self.window);
            }
        }
        
        info!("[NEW_RENDERER][GRALLOC] Gralloc manager dropped");
    }
}
