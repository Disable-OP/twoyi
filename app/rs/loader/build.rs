// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of merchantability,
// fitness for a particular purpose, or non-infringement.
// Use at your own risk.

fn main() {
    println!("cargo:rerun-if-changed=src");
    
    // Configure for PIE executable with main entry point
    println!("cargo:rustc-cdylib-link-arg=-Wl,-e,main");
    println!("cargo:rustc-cdylib-link-arg=-Wl,--dynamic-linker=/system/bin/linker64");
    println!("cargo:rustc-cdylib-link-arg=-pie");
}
