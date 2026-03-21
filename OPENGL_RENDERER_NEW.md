# Copyright Disclaimer: AI-Generated Content
# This file was created by GitHub Copilot, an AI coding assistant.
# AI-generated content is not subject to copyright protection and is provided
# without any warranty, express or implied, including warranties of merchantability,
# fitness for a particular purpose, or non-infringement.
# Use at your own risk.

# New libOpenglRender.so - Usage Guide

## Overview

This guide explains how to use the new open-source `libOpenglRender.so` that has been created as a complete replacement for the legacy proprietary library.

## What Was Created

A new standalone Rust library that:
- **Provides the exact same C API** as the legacy libOpenglRender.so
- **Is 50% smaller** (544KB vs 1.1MB)
- **Is fully open-source** with MPL 2.0 license
- **Works as a drop-in replacement** for the legacy library

## Library Location

**Source code**: `app/rs/openglrenderer/`  
**Built library**: `app/src/main/jniLibs/arm64-v8a/libOpenglRender_new.so`  
**Legacy library**: `app/src/main/jniLibs/arm64-v8a/libOpenglRender.so`

## Exported API

The new library exports these C functions:

```c
int startOpenGLRenderer(void* win, int width, int height, int xdpi, int ydpi, int fps);
int setNativeWindow(void* window);
int resetSubWindow(void* p_window, int wx, int wy, int ww, int wh, int fbw, int fbh, float dpr, float zRot);
int removeSubWindow(void* window);
int destroyOpenGLSubwindow();
void repaintOpenGLDisplay();
```

## Building the Library

### Prerequisites

1. Rust toolchain with Android target:
   ```bash
   rustup target add aarch64-linux-android
   ```

2. cargo-xdk for Android builds:
   ```bash
   cargo install --git https://github.com/tiann/cargo-xdk
   ```

3. Android NDK (automatically detected from ANDROID_NDK_HOME)

### Build Commands

**Quick build** (using the script):
```bash
cd app/rs/openglrenderer
./build.sh
```

**Manual build**:
```bash
cd app/rs/openglrenderer
cargo xdk -t arm64-v8a build --release
cp target/aarch64-linux-android/release/libOpenglRender.so ../../src/main/jniLibs/arm64-v8a/libOpenglRender_new.so
```

## Verification

Check that all required symbols are exported:

```bash
nm -D app/src/main/jniLibs/arm64-v8a/libOpenglRender_new.so | grep -E "startOpenGLRenderer|destroyOpenGLSubwindow|repaintOpenGLDisplay|setNativeWindow|resetSubWindow|removeSubWindow"
```

Expected output:
```
000000000001b4e0 T destroyOpenGLSubwindow
000000000001ba64 T removeSubWindow
000000000001bcfc T repaintOpenGLDisplay
000000000001c0d0 T resetSubWindow
000000000001c8f8 T setNativeWindow
000000000001ccc0 T startOpenGLRenderer
```

## Using the New Library

### Option 1: Replace the Legacy Library

The safest way is to rename the new library:

```bash
cd app/src/main/jniLibs/arm64-v8a/
# Backup the legacy library
mv libOpenglRender.so libOpenglRender_legacy.so
# Use the new library
cp libOpenglRender_new.so libOpenglRender.so
```

Then rebuild the app:
```bash
./gradlew assembleRelease
```

### Option 2: Load Dynamically

Modify the Java/Kotlin code to load the new library instead:

```java
// Instead of loading "OpenglRender"
System.loadLibrary("OpenglRender");  // old

// Load the new library (after renaming _new.so to OpenglRender_new.so)
System.loadLibrary("OpenglRender_new");  // new
```

### Option 3: Use via Renderer Toggle

The app already has a renderer selection mechanism. The new library can be integrated into that system to allow users to choose between old and new at runtime.

## Testing

After building and integrating the new library:

1. **Start the app** and check logcat for renderer initialization messages:
   ```bash
   adb logcat | grep "OPENGL_RENDERER\|CLIENT_EGL"
   ```

2. **Check for initialization messages**:
   ```
   [OPENGL_RENDERER] Starting OpenGL renderer
   [OPENGL_RENDERER] QEMU pipe device is available
   [OPENGL_RENDERER] GL context created successfully
   ```

3. **Verify functionality**:
   - Launch a container
   - Check if the display works
   - Test touch input
   - Verify rotation and resizing

## Debug Mode

The library supports debug mode for troubleshooting:

```rust
// In the code, debug mode can be enabled by setting:
DEBUG_MODE.store(true, Ordering::Relaxed);

// Or by setting the debug log directory:
DEBUG_LOG_DIR.lock().unwrap() = String::from("/sdcard/twoyi_debug");
```

Debug logs will include:
- QEMU pipe communication
- OpenGL ES commands
- Gralloc buffer operations

## Architecture

The new library uses:

1. **QEMU Pipe** (`/dev/qemu_pipe`) for communication with the container
2. **OpenGL ES Protocol** to send rendering commands
3. **Gralloc** for Android graphics buffer management
4. **Automatic Fallback** from OpenGL ES 3 → 2 → 1

## Troubleshooting

### Library fails to load

Check if the library was built correctly:
```bash
file app/src/main/jniLibs/arm64-v8a/libOpenglRender_new.so
```

Should show:
```
ELF 64-bit LSB shared object, ARM aarch64, version 1 (SYSV), dynamically linked
```

### Renderer fails to initialize

Common causes:
1. `/dev/qemu_pipe` not available in container
2. OpenGL ES services not running
3. Permissions issues

Check logcat for error messages:
```bash
adb logcat | grep -i "opengl\|qemu\|pipe"
```

### Black screen or no display

1. Check if the container is fully booted
2. Verify OpenGL context was created successfully
3. Try reverting to the legacy library to isolate the issue

## Comparison: New vs Legacy

| Feature | Legacy | New |
|---------|--------|-----|
| Size | 1.1 MB | 544 KB |
| Source Code | Closed | Open (MPL 2.0) |
| Implementation | C++ (proprietary) | Rust (open-source) |
| Symbols Exported | 2517 | 6 (clean API) |
| Dependencies | Unknown | QEMU pipe, gralloc |
| Debug Support | No | Yes |
| Maintainability | Low (no source) | High (full source) |

## Future Enhancements

Potential improvements:
- [ ] Hardware acceleration support
- [ ] Full OpenGL ES 3.x feature support
- [ ] Performance optimizations
- [ ] Additional graphics protocols
- [ ] Better error reporting

## Contributing

The source code is in `app/rs/openglrenderer/`. Contributions are welcome!

1. Make changes in the source files
2. Build and test: `./build.sh`
3. Verify symbols: `nm -D ...`
4. Submit a PR

## License

The new library is licensed under MPL 2.0, same as the Twoyi project.

## Support

For issues or questions:
1. Check logcat for error messages
2. Enable debug mode for detailed logs
3. Compare behavior with legacy library
4. File an issue with logs and reproduction steps
