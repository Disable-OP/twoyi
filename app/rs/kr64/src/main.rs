// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! Binary entry point for `kr64`.
//!
//! This is the regular Rust `bin` target — a thin wrapper around
//! [`kr64::run`] that reads `std::env::args()` and exits with the
//! returned status code.
//!
//! The cdylib (`libkr64.so`) has its own entry point (`kr64_main`,
//! defined in `lib.rs`) that's invoked when the .so is exec'd
//! directly via the PIE hack. Both code paths call the same `run()`
//! function, so behaviour is identical regardless of how the daemon
//! is invoked.
//!
//! # Logging
//!
//! The crate uses a built-in `eprintln!`-based logger (see the
//! `info!` / `warn!` / `error!` macros in `lib.rs`) — no external
//! `log` crate dependency. All log lines go to stderr in the format
//! `[KR64 <LEVEL>] <msg>`. A production version would plug into
//! Android's `__android_log_write` or `logd` socket, but for the
//! MVP stderr is sufficient (and works both on-device via
//! `adb logcat` and on the host during `cargo test`).

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let exit_code = kr64::run(args);
    std::process::exit(exit_code);
}
