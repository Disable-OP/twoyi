// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of merchantability,
// fitness for a particular purpose, or non-infringement.
// Use at your own risk.

fn main() {
    println!("cargo:rerun-if-changed=src");
    
    // Configure library settings
    println!("cargo:rustc-cdylib-link-arg=-Wl,--version-script=libOpenglRender.map");
    
    // Export only the public API symbols
    println!("cargo:rustc-cdylib-link-arg=-Wl,--exclude-libs,ALL");
}
