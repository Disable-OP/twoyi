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
use std::thread;

use crate::input;
use crate::renderer_bindings;

static RENDERER_STARTED: AtomicBool = AtomicBool::new(false);

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
    info!(
        "init_renderer surface: {}x{}, virtual: {}x{}, fps: {}",
        surface_width, surface_height, virtual_width, virtual_height, fps
    );

    if RENDERER_STARTED
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
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
    } else {
        input::start_input_system(virtual_width, virtual_height);

        // Convert raw pointer to usize for safe transfer between threads
        let window_addr = window as usize;
        thread::spawn(move || {
            let window = window_addr as *mut c_void;
            info!("Starting OpenGL renderer with window: {:#?}", window);
            unsafe {
                renderer_bindings::startOpenGLRenderer(window, virtual_width, virtual_height, xdpi, ydpi, fps);
            }
        });

        let working_dir = "/data/data/io.twoyi/rootfs";
        let log_path = "/data/data/io.twoyi/log.txt";
        let outputs = File::create(log_path).unwrap();
        let errors = outputs.try_clone().unwrap();
        let _ = Command::new("./init")
            .current_dir(working_dir)
            .env("TYLOADER", loader_path)
            .stdout(Stdio::from(outputs))
            .stderr(Stdio::from(errors))
            .spawn();
    }
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
    unsafe {
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
    }
}

/// Remove a window
pub fn remove_window(window: *mut c_void) {
    unsafe {
        renderer_bindings::removeSubWindow(window);
    }
}
