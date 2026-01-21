// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of merchantability,
// fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! OpenGL ES protocol implementation
//! 
//! This module handles the OpenGL ES command protocol communication
//! with the container's graphics backend.

use log::{debug, info, warn};
use super::pipe::PipeConnection;
use std::io;

/// OpenGL ES command types
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum GLCommand {
    Initialize = 0x1000,
    SetWindowSize = 0x1001,
    SwapBuffers = 0x1002,
    MakeCurrent = 0x1003,
    Destroy = 0x1004,
    Repaint = 0x1005,
}

/// OpenGL ES context state
pub struct GLContext {
    pipe: PipeConnection,
    width: i32,
    height: i32,
    initialized: bool,
}

impl GLContext {
    /// Create a new OpenGL ES context
    pub fn new() -> io::Result<Self> {
        info!("[NEW_RENDERER] Creating new GL context");
        debug!("[NEW_RENDERER] Establishing pipe connection...");
        let pipe = match super::pipe::create_opengles_connection() {
            Ok(p) => {
                info!("[NEW_RENDERER] Pipe connection established successfully");
                p
            },
            Err(e) => {
                warn!("[NEW_RENDERER] Failed to create pipe connection: {}", e);
                return Err(e);
            }
        };
        
        info!("[NEW_RENDERER] GL context created successfully");
        Ok(GLContext {
            pipe,
            width: 0,
            height: 0,
            initialized: false,
        })
    }
    
    /// Dump command info to debug log if debug mode is enabled
    fn dump_command(&self, cmd: GLCommand, params: &str) {
        if super::is_debug_mode() {
            use std::fs::OpenOptions;
            use std::io::Write;
            use std::time::{SystemTime, UNIX_EPOCH};
            
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
            
            let log_dir = super::get_debug_log_dir();
            let _ = std::fs::create_dir_all(&log_dir);
            let log_path = format!("{}/opengles_commands.log", log_dir);
            
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
            {
                let _ = writeln!(file, "[{}] {:?} (0x{:04x}): {}", 
                                timestamp, cmd, cmd as u32, params);
            }
        }
    }
    
    /// Initialize the OpenGL ES context
    pub fn initialize(&mut self, width: i32, height: i32, xdpi: i32, ydpi: i32, fps: i32) -> io::Result<()> {
        info!("[NEW_RENDERER] Initializing GL context: {}x{}, DPI: {}x{}, FPS: {}", 
              width, height, xdpi, ydpi, fps);
        
        // Dump command to debug log
        self.dump_command(GLCommand::Initialize, 
                         &format!("width={}, height={}, xdpi={}, ydpi={}, fps={}", width, height, xdpi, ydpi, fps));
        
        // Send initialization command
        let cmd = GLCommand::Initialize as u32;
        debug!("[NEW_RENDERER] Sending Initialize command: 0x{:04x}", cmd);
        
        let data = [
            cmd.to_le_bytes(),
            width.to_le_bytes(),
            height.to_le_bytes(),
            xdpi.to_le_bytes(),
            ydpi.to_le_bytes(),
            fps.to_le_bytes(),
        ];
        
        debug!("[NEW_RENDERER] Sending {} parameter sets", data.len());
        for (i, bytes) in data.iter().enumerate() {
            debug!("[NEW_RENDERER] Writing param {}: {:?}", i, bytes);
            self.pipe.write_all(bytes)?;
        }
        
        debug!("[NEW_RENDERER] Flushing initialization data...");
        self.pipe.flush()?;
        
        self.width = width;
        self.height = height;
        self.initialized = true;
        
        info!("[NEW_RENDERER] GL context initialized successfully");
        Ok(())
    }
    
    /// Set or update window size
    pub fn set_window_size(&mut self, width: i32, height: i32, fb_width: i32, fb_height: i32) -> io::Result<()> {
        info!("[NEW_RENDERER] Setting window size: surface={}x{}, framebuffer={}x{}", 
              width, height, fb_width, fb_height);
        
        // Dump command to debug log
        self.dump_command(GLCommand::SetWindowSize, 
                         &format!("width={}, height={}, fb_width={}, fb_height={}", width, height, fb_width, fb_height));
        
        let cmd = GLCommand::SetWindowSize as u32;
        debug!("[NEW_RENDERER] Sending SetWindowSize command: 0x{:04x}", cmd);
        
        let data = [
            cmd.to_le_bytes(),
            width.to_le_bytes(),
            height.to_le_bytes(),
            fb_width.to_le_bytes(),
            fb_height.to_le_bytes(),
        ];
        
        for (i, bytes) in data.iter().enumerate() {
            debug!("[NEW_RENDERER] Writing param {}: {:?}", i, bytes);
            self.pipe.write_all(bytes)?;
        }
        self.pipe.flush()?;
        
        self.width = fb_width;
        self.height = fb_height;
        
        info!("[NEW_RENDERER] Window size updated successfully");
        Ok(())
    }
    
    /// Swap buffers to display rendered content
    #[allow(dead_code)]
    pub fn swap_buffers(&mut self) -> io::Result<()> {
        debug!("[NEW_RENDERER] Swapping buffers");
        self.dump_command(GLCommand::SwapBuffers, "");
        let cmd = GLCommand::SwapBuffers as u32;
        self.pipe.write_all(&cmd.to_le_bytes())?;
        self.pipe.flush()?;
        debug!("[NEW_RENDERER] Buffers swapped");
        Ok(())
    }
    
    /// Repaint the display
    #[allow(dead_code)]
    pub fn repaint(&mut self) -> io::Result<()> {
        debug!("[NEW_RENDERER] Repainting display");
        self.dump_command(GLCommand::Repaint, "");
        let cmd = GLCommand::Repaint as u32;
        self.pipe.write_all(&cmd.to_le_bytes())?;
        self.pipe.flush()?;
        debug!("[NEW_RENDERER] Display repainted");
        Ok(())
    }
    
    /// Destroy the GL context
    pub fn destroy(&mut self) -> io::Result<()> {
        info!("[NEW_RENDERER] Destroying GL context");
        
        self.dump_command(GLCommand::Destroy, "");
        
        let cmd = GLCommand::Destroy as u32;
        debug!("[NEW_RENDERER] Sending Destroy command: 0x{:04x}", cmd);
        self.pipe.write_all(&cmd.to_le_bytes())?;
        self.pipe.flush()?;
        
        self.initialized = false;
        info!("[NEW_RENDERER] GL context destroyed");
        Ok(())
    }
    
    /// Check if context is initialized
    #[allow(dead_code)]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl Drop for GLContext {
    fn drop(&mut self) {
        if self.initialized {
            let _ = self.destroy();
        }
    }
}
