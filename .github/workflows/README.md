# Copyright Disclaimer: AI-Generated Content
# This file was created by GitHub Copilot, an AI coding assistant.
# AI-generated content is not subject to copyright protection and is provided
# without any warranty, express or implied, including warranties of merchantability,
# fitness for a particular purpose, or non-infringement.
# Use at your own risk.

# GitHub Actions Build Workflow

This repository includes a GitHub Actions workflow that automatically builds the Twoyi APK.

## Prerequisites

Before the workflow can successfully build the APK, you need to:

1. **Add the rootfs.7z file** to `app/src/main/assets/`
   - Download from: https://github.com/cyanmint/twoyi/releases/download/original/rootfs.7z
   - Or extract it from an official release APK
   - Place it in `app/src/main/assets/rootfs.7z`

2. **Ensure rom.ini exists** in `app/src/main/assets/`
   - This file should be included in the rootfs.7z archive
   - Or copy it from an official release

## Workflow Features

- **Automatic builds** on push to main/develop branches
- **Pull request builds** for validation
- **Manual trigger** via workflow_dispatch
- **Artifact uploads** - APKs are stored for 30 days
- **Cargo-xdk caching** - Speeds up builds by caching the Rust toolchain helper
- **Android target** - Automatically installs aarch64-linux-android Rust target

## Build Configuration

- Uses NDK r22b as required by the project
- compileSdk set to 31 (compatible with build tools 30.0.3)
- targetSdk remains 28 as per project requirements
- Uses mavenCentral for dependency resolution (JCenter is deprecated)
- libp7zip replaced with jitpack alternative (com.github.hzy3774:AndroidP7zip)

## Running Locally

To test the build locally before committing:

1. Install Rust and Cargo: https://www.rust-lang.org/tools/install
2. Install Rust Android target: `rustup target add aarch64-linux-android`
3. Install cargo-xdk: `cargo install cargo-xdk`
4. Install Android NDK r22b (or set ANDROID_NDK_HOME to compatible version)
5. Add rootfs.7z to assets
6. Run: `./gradlew assembleRelease`

## Notes

- The workflow uses NDK r22b as required by the project
- Build artifacts are automatically uploaded after successful builds
- The workflow validates YAML syntax with yamllint
- Rust target aarch64-linux-android is installed automatically
