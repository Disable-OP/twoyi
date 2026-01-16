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

pub mod pipe;
pub mod opengles;
pub mod renderer;

pub use renderer::{
    start_renderer,
    reset_window,
    remove_window,
    set_native_window,
};
