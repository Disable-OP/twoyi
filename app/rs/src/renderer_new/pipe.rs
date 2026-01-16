// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of merchantability,
// fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! QEMU pipe connection module
//! 
//! This module implements the QEMU pipe protocol for communication with the
//! container's OpenGL ES endpoints, similar to Anbox's pipe_connection_creator.

use log::{debug, info, warn};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;

/// QEMU pipe service names for OpenGL ES
pub const OPENGLES_PIPE: &str = "/opengles";
pub const OPENGLES2_PIPE: &str = "/opengles2";
pub const OPENGLES3_PIPE: &str = "/opengles3";

/// Default pipe device path
const PIPE_DEVICE: &str = "/dev/qemu_pipe";

/// Represents a connection to a QEMU pipe
pub struct PipeConnection {
    file: File,
    service_name: String,
}

impl PipeConnection {
    /// Create a new pipe connection to the specified service
    /// 
    /// # Arguments
    /// * `service_name` - The name of the service to connect to (e.g., "/opengles2")
    pub fn new(service_name: &str) -> io::Result<Self> {
        info!("[NEW_RENDERER] Opening QEMU pipe connection to: {}", service_name);
        debug!("[NEW_RENDERER] Pipe device path: {}", PIPE_DEVICE);
        
        // Try to open the pipe device
        debug!("[NEW_RENDERER] Attempting to open pipe device...");
        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .open(PIPE_DEVICE) {
            Ok(f) => {
                info!("[NEW_RENDERER] Successfully opened pipe device");
                f
            },
            Err(e) => {
                warn!("[NEW_RENDERER] Failed to open pipe device {}: {}", PIPE_DEVICE, e);
                return Err(e);
            }
        };
        
        let mut connection = PipeConnection {
            file,
            service_name: service_name.to_string(),
        };
        
        // Send the service name to establish connection
        debug!("[NEW_RENDERER] Establishing connection by sending service name...");
        connection.connect()?;
        
        info!("[NEW_RENDERER] Successfully connected to QEMU pipe: {}", service_name);
        Ok(connection)
    }
    
    /// Establish the pipe connection by writing the service name
    fn connect(&mut self) -> io::Result<()> {
        // Write service name to pipe to establish connection
        let service_bytes = self.service_name.as_bytes();
        debug!("[NEW_RENDERER] Writing service name ({} bytes): {:?}", 
               service_bytes.len(), self.service_name);
        
        match self.file.write_all(service_bytes) {
            Ok(_) => debug!("[NEW_RENDERER] Successfully wrote service name"),
            Err(e) => {
                warn!("[NEW_RENDERER] Failed to write service name: {}", e);
                return Err(e);
            }
        }
        
        match self.file.flush() {
            Ok(_) => debug!("[NEW_RENDERER] Successfully flushed pipe"),
            Err(e) => {
                warn!("[NEW_RENDERER] Failed to flush pipe: {}", e);
                return Err(e);
            }
        }
        
        debug!("[NEW_RENDERER] Connection established for: {}", self.service_name);
        Ok(())
    }
    
    /// Write data to the pipe
    #[allow(dead_code)]
    pub fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.file.write(data)
    }
    
    /// Write all data to the pipe
    pub fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        debug!("[NEW_RENDERER] Writing {} bytes to pipe", data.len());
        match self.file.write_all(data) {
            Ok(_) => {
                debug!("[NEW_RENDERER] Successfully wrote data to pipe");
                Ok(())
            },
            Err(e) => {
                warn!("[NEW_RENDERER] Failed to write to pipe: {}", e);
                Err(e)
            }
        }
    }
    
    /// Read data from the pipe
    #[allow(dead_code)]
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
    
    /// Read exact amount of data from the pipe
    #[allow(dead_code)]
    pub fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.file.read_exact(buf)
    }
    
    /// Flush the pipe
    pub fn flush(&mut self) -> io::Result<()> {
        debug!("[NEW_RENDERER] Flushing pipe");
        self.file.flush()
    }
    
    /// Get the raw file descriptor
    #[allow(dead_code)]
    pub fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

/// Check if QEMU pipe device is available
pub fn is_pipe_available() -> bool {
    let available = Path::new(PIPE_DEVICE).exists();
    info!("[NEW_RENDERER] QEMU pipe device {} availability: {}", PIPE_DEVICE, available);
    available
}

/// Try to create a connection with automatic fallback
/// 
/// Tries to connect to OpenGL ES 3, then 2, then 1 if earlier versions fail
pub fn create_opengles_connection() -> io::Result<PipeConnection> {
    info!("[NEW_RENDERER] Attempting to create OpenGL ES connection with fallback");
    
    // Try OpenGL ES 3 first
    debug!("[NEW_RENDERER] Trying OpenGL ES 3.x");
    if let Ok(conn) = PipeConnection::new(OPENGLES3_PIPE) {
        info!("[NEW_RENDERER] Using OpenGL ES 3.x");
        return Ok(conn);
    }
    debug!("[NEW_RENDERER] OpenGL ES 3.x not available, trying ES 2.x");
    
    // Fall back to OpenGL ES 2
    debug!("[NEW_RENDERER] Trying OpenGL ES 2.x");
    if let Ok(conn) = PipeConnection::new(OPENGLES2_PIPE) {
        info!("[NEW_RENDERER] Using OpenGL ES 2.x");
        return Ok(conn);
    }
    debug!("[NEW_RENDERER] OpenGL ES 2.x not available, trying ES 1.x");
    
    // Fall back to OpenGL ES 1
    debug!("[NEW_RENDERER] Trying OpenGL ES 1.x");
    if let Ok(conn) = PipeConnection::new(OPENGLES_PIPE) {
        info!("[NEW_RENDERER] Using OpenGL ES 1.x");
        return Ok(conn);
    }
    
    warn!("[NEW_RENDERER] Failed to connect to any OpenGL ES pipe service");
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Failed to connect to any OpenGL ES pipe service"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pipe_availability() {
        // This test just checks if the function runs without panic
        let _ = is_pipe_available();
    }
}
