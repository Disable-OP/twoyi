# Copyright Disclaimer: AI-Generated Content
# This file was created by GitHub Copilot, an AI coding assistant.
# AI-generated content is not subject to copyright protection and is provided
# without any warranty, express or implied, including warranties of merchantability,
# fitness for a particular purpose, or non-infringement.
# Use at your own risk.

# New OpenGL Renderer Library

This directory contains the source code for the new open-source `libOpenglRender.so` library, which is a complete replacement for the proprietary legacy library.

## Overview

The new library provides the same C API as the legacy `libOpenglRender.so` but uses a modern Rust implementation based on QEMU pipes and OpenGL ES protocol.

## Architecture

The library consists of:

1. **lib.rs** - Main entry point exposing the C API functions
2. **pipe.rs** - QEMU pipe communication layer
3. **opengles.rs** - OpenGL ES protocol implementation
4. **gralloc.rs** - Android graphics buffer (gralloc) management

## Exported API

The library exports the following C functions that match the legacy library:

- `int startOpenGLRenderer(void* win, int width, int height, int xdpi, int ydpi, int fps)`
- `int setNativeWindow(void* window)`
- `int resetSubWindow(void* p_window, int wx, int wy, int ww, int wh, int fbw, int fbh, float dpr, float zRot)`
- `int removeSubWindow(void* window)`
- `int destroyOpenGLSubwindow()`
- `void repaintOpenGLDisplay()`

## Building

To build the library:

```bash
./build.sh
```

Or manually:

```bash
cargo xdk -t arm64-v8a build --release
cp target/aarch64-linux-android/release/libOpenglRender.so ../../src/main/jniLibs/arm64-v8a/libOpenglRender_new.so
```

## Requirements

- Rust toolchain with `aarch64-linux-android` target
- cargo-xdk (Android cross-compilation tool)
- Android NDK

## Size Comparison

- Legacy library: 1.1 MB
- New library: 544 KB (50% smaller!)

## Verification

To verify the exported symbols:

```bash
nm -D libOpenglRender_new.so | grep -E "startOpenGLRenderer|destroyOpenGLSubwindow|repaintOpenGLDisplay|setNativeWindow|resetSubWindow|removeSubWindow"
```

## Usage

The new library can be used as a drop-in replacement for the legacy `libOpenglRender.so`. Simply rename it or update the code to load it instead of the legacy library.

## Implementation Details

The new library is based on the Anbox OpenGL renderer architecture:

1. **QEMU Pipe Protocol** - Uses `/dev/qemu_pipe` to communicate with the container
2. **OpenGL ES Commands** - Sends GL commands through the pipe (Initialize, SetWindowSize, SwapBuffers, etc.)
3. **Gralloc Integration** - Manages graphics buffers through Android's gralloc system
4. **Automatic Fallback** - Tries OpenGL ES 3 → 2 → 1 based on availability

## Debug Mode

The library supports debug mode which can be enabled to log extensive debugging information:

- QEMU pipe communication
- OpenGL ES commands
- Gralloc buffer operations

Debug logs are written to `/data/local/tmp/twoyi_renderer_debug/` by default.

## License

This library is licensed under the Mozilla Public License 2.0 (MPL 2.0), same as the rest of the Twoyi project.

## Contributing

Contributions are welcome! The code is written in Rust and follows standard Rust conventions.
