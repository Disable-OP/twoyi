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

use log::{debug, info};
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
        let pipe = super::pipe::create_opengles_connection()?;
        
        Ok(GLContext {
            pipe,
            width: 0,
            height: 0,
            initialized: false,
        })
    }
    
    /// Initialize the OpenGL ES context
    pub fn initialize(&mut self, width: i32, height: i32, xdpi: i32, ydpi: i32, fps: i32) -> io::Result<()> {
        info!("Initializing GL context: {}x{} @ {} fps", width, height, fps);
        
        // Send initialization command
        let cmd = GLCommand::Initialize as u32;
        let data = [
            cmd.to_le_bytes(),
            width.to_le_bytes(),
            height.to_le_bytes(),
            xdpi.to_le_bytes(),
            ydpi.to_le_bytes(),
            fps.to_le_bytes(),
        ];
        
        for bytes in &data {
            self.pipe.write_all(bytes)?;
        }
        self.pipe.flush()?;
        
        self.width = width;
        self.height = height;
        self.initialized = true;
        
        debug!("GL context initialized successfully");
        Ok(())
    }
    
    /// Set or update window size
    pub fn set_window_size(&mut self, width: i32, height: i32, fb_width: i32, fb_height: i32) -> io::Result<()> {
        debug!("Setting window size: surface={}x{}, framebuffer={}x{}", width, height, fb_width, fb_height);
        
        let cmd = GLCommand::SetWindowSize as u32;
        let data = [
            cmd.to_le_bytes(),
            width.to_le_bytes(),
            height.to_le_bytes(),
            fb_width.to_le_bytes(),
            fb_height.to_le_bytes(),
        ];
        
        for bytes in &data {
            self.pipe.write_all(bytes)?;
        }
        self.pipe.flush()?;
        
        self.width = fb_width;
        self.height = fb_height;
        
        Ok(())
    }
    
    /// Swap buffers to display rendered content
    #[allow(dead_code)]
    pub fn swap_buffers(&mut self) -> io::Result<()> {
        let cmd = GLCommand::SwapBuffers as u32;
        self.pipe.write_all(&cmd.to_le_bytes())?;
        self.pipe.flush()?;
        Ok(())
    }
    
    /// Repaint the display
    #[allow(dead_code)]
    pub fn repaint(&mut self) -> io::Result<()> {
        let cmd = GLCommand::Repaint as u32;
        self.pipe.write_all(&cmd.to_le_bytes())?;
        self.pipe.flush()?;
        Ok(())
    }
    
    /// Destroy the GL context
    pub fn destroy(&mut self) -> io::Result<()> {
        info!("Destroying GL context");
        
        let cmd = GLCommand::Destroy as u32;
        self.pipe.write_all(&cmd.to_le_bytes())?;
        self.pipe.flush()?;
        
        self.initialized = false;
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
