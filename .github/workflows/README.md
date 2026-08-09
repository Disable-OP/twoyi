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

1. **Add the rootfs.tar.gz file** to `app/src/main/assets/`
   - Download from: https://github.com/cyanmint/twoyi/releases/download/original/rootfs.tar.gz
   - Or extract it from an official release APK
   - Place it in `app/src/main/assets/rootfs.tar.gz`
   - The build workflow can also fetch it automatically — see the
     `include_rootfs` input on the manual (`workflow_dispatch`) trigger.

2. **Ensure rom.ini exists** in `app/src/main/assets/`
   - This file should be included in the rootfs.tar.gz archive
   - Or copy it from an official release

## Workflow Features

- **Automatic builds** on push to main/develop/`improvements/**` branches
- **Pull request builds** for validation
- **Manual trigger** via workflow_dispatch (with `abis` and `include_rootfs` inputs)
- **Concurrency control** — superseded runs on the same ref are cancelled
  automatically, so rapid pushes don't queue redundant ~15-minute builds
- **Artifact uploads** - APKs are stored for 30 days
- **Cargo-xdk caching** - Speeds up builds by caching the Rust toolchain helper
- **Android target** - Automatically installs aarch64-linux-android and
  x86_64-linux-android Rust targets

## Build Configuration

- Uses NDK r27c (upgraded from r22b for Rust compatibility)
- compileSdk set to 31 (compatible with build tools 30.0.3)
- targetSdk remains 28 as per project requirements
- Uses mavenCentral + jitpack for dependency resolution (JCenter is deprecated
  and only kept for a handful of legacy transitive deps)
- The rootfs is shipped as `rootfs.tar.gz` (not `.7z`) — RomManager and the
  import flow in SettingsActivity extract it by shelling out to the system
  `tar -xf` binary (via libsu), so no 7-Zip native library is bundled.

## Running Locally

To test the build locally before committing:

1. Install Rust and Cargo: https://www.rust-lang.org/tools/install
2. Install Rust Android target: `rustup target add aarch64-linux-android`
3. Install cargo-xdk: `cargo install cargo-xdk`
4. Install Android NDK r27c (or compatible version 26+)
5. Add rootfs.tar.gz to `app/src/main/assets/` (without it the APK still
   builds but won't run — the import flow expects to find a rootfs there
   or to be given one via Settings → Advanced → Import Rootfs)
6. Run: `./gradlew assembleRelease -Pabis=all`

## Notes

- The workflow uses NDK r27c (r22b has compatibility issues with modern Rust)
- Build artifacts are automatically uploaded after successful builds
- Rust targets aarch64-linux-android and x86_64-linux-android are installed
  automatically
- Placeholder assets are not bundled — CI builds produce a "shell" APK that
  compiles but isn't functional. Flip the `include_rootfs` workflow_dispatch
  input to fetch the real ~275 MB rootfs.tar.gz from the cyanmint/twoyi
  `original` release and bundle it into the APK.
- The companion `kr64-tests.yml` workflow runs `cargo fmt --check`,
  `cargo clippy -D warnings`, and `cargo test` on the kr64 crate to gate CI
  on the project's "0 clippy warnings / 145 tests pass" baseline
