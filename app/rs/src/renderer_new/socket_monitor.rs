// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of merchantability,
// fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Socket monitoring module for debug renderer
//! 
//! This module monitors and dumps data from various container sockets when
//! debug mode is enabled, including input sockets, service sockets, binder
//! sockets, and OpenGL ES sockets.

use log::{debug, error, info, warn};
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// List of sockets to monitor in the container
const SOCKET_PATHS: &[&str] = &[
    // Input sockets
    "/dev/input/key0",
    "/dev/input/touch",
    
    // Service sockets
    "/dev/socket/property_service",
    "/dev/socket/vold",
    "/dev/socket/cryptd",
    "/dev/socket/netd",
    "/dev/socket/dnsproxyd",
    "/dev/socket/mdns",
    "/dev/socket/fwmarkd",
    "/dev/socket/zygote",
    "/dev/socket/webview_zygote",
    
    // Binder sockets
    "/dev/vbinder/bcs",
    "/dev/vbinder/bhs",
    "/dev/vbinder/bis",
    "/dev/vndbinder/bcs",
    "/dev/vndbinder/bhs",
    "/dev/vndbinder/bis",
    "/dev/hwbinder/bcs",
    "/dev/hwbinder/bhs",
    "/dev/hwbinder/bis",
    
    // OpenGL ES sockets (at root level - from container)
    "/opengles",
    "/opengles2",
    "/opengles3",
    
    // OpenGL ES sockets (full paths - from legacy renderer)
    "/data/data/io.twoyi/rootfs/opengles",
    "/data/data/io.twoyi/rootfs/opengles2",
    "/data/data/io.twoyi/rootfs/opengles3",
    
    // Debug socket
    "/data/system/ndebugsocket",
];

/// Start monitoring all container sockets
pub fn start_socket_monitoring() {
    if !super::is_debug_mode() {
        debug!("[SOCKET_MONITOR] Debug mode not enabled, skipping socket monitoring");
        return;
    }
    
    info!("[SOCKET_MONITOR] Starting socket monitoring for debug renderer");
    
    // Create debug log directory
    let log_dir = super::get_debug_log_dir();
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        error!("[SOCKET_MONITOR] Failed to create log directory: {}", e);
        return;
    }
    
    // Spawn monitoring threads for each socket
    // Note: Threads will naturally terminate when sockets close or fail to connect.
    // They use timeouts and non-blocking I/O to avoid hanging indefinitely.
    // The OS will clean up all threads when the process terminates.
    for socket_path in SOCKET_PATHS {
        let path = socket_path.to_string();
        thread::spawn(move || {
            monitor_socket(&path);
        });
    }
    
    info!("[SOCKET_MONITOR] Socket monitoring threads started");
}

/// Monitor a single socket and dump its data
fn monitor_socket(socket_path: &str) {
    debug!("[SOCKET_MONITOR] Starting monitor for socket: {}", socket_path);
    
    // Check if socket exists
    if !Path::new(socket_path).exists() {
        debug!("[SOCKET_MONITOR] Socket does not exist: {}", socket_path);
        return;
    }
    
    // Try to connect to the socket
    match UnixStream::connect(socket_path) {
        Ok(mut stream) => {
            info!("[SOCKET_MONITOR] Connected to socket: {}", socket_path);
            
            // Set non-blocking mode with timeout
            if let Err(e) = stream.set_read_timeout(Some(Duration::from_millis(100))) {
                warn!("[SOCKET_MONITOR] Failed to set read timeout: {}", e);
            }
            
            let socket_name = socket_path.replace("/", "_");
            let log_dir = super::get_debug_log_dir();
            let log_path = format!("{}/socket_{}.log", log_dir, socket_name);
            
            // Read and dump data from socket
            let mut buffer = vec![0u8; 4096];
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) => {
                        debug!("[SOCKET_MONITOR] Socket closed: {}", socket_path);
                        break;
                    }
                    Ok(n) => {
                        dump_socket_data(socket_path, &log_path, "READ", &buffer[..n]);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        // Timeout, continue monitoring
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                    Err(e) => {
                        warn!("[SOCKET_MONITOR] Error reading from socket {}: {}", socket_path, e);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            debug!("[SOCKET_MONITOR] Failed to connect to socket {}: {}", socket_path, e);
        }
    }
}

/// Dump socket data to a log file
fn dump_socket_data(socket_path: &str, log_path: &str, direction: &str, data: &[u8]) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        let _ = writeln!(file, "\n[{}] {} {} bytes from {}:", 
                        timestamp, direction, data.len(), socket_path);
        let _ = writeln!(file, "Hex: {:02x?}", data);
        
        // Try to interpret as ASCII
        let ascii: String = data.iter()
            .map(|&b| if b.is_ascii_graphic() || b == b' ' || b == b'\n' || b == b'\r' { 
                b as char 
            } else { 
                '.' 
            })
            .collect();
        let _ = writeln!(file, "ASCII: {}", ascii);
        
        // If data is 4-byte aligned, also show as integers
        if data.len() >= 4 && data.len() % 4 == 0 {
            let _ = write!(file, "i32: [");
            for chunk in data.chunks_exact(4) {
                let value = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let _ = write!(file, "{}, ", value);
            }
            let _ = writeln!(file, "]");
        }
    }
}

/// Stop socket monitoring (currently monitors run until socket closes or process ends)
#[allow(dead_code)]
pub fn stop_socket_monitoring() {
    info!("[SOCKET_MONITOR] Socket monitoring stop requested");
    // Individual monitor threads will terminate when sockets close
}
