# Copyright Disclaimer: AI-Generated Content
# This file was created by GitHub Copilot, an AI coding assistant.
# AI-generated content is not subject to copyright protection and is provided
# without any warranty, express or implied, including warranties of merchantability,
# fitness for a particular purpose, or non-infringement.
# Use at your own risk.

# Open Source OpenGL Renderer

This document describes the new open-source OpenGL renderer implementation in Rust.

## Overview

The new renderer is a fully open-source implementation written in Rust that communicates with the container's OpenGL ES endpoints via QEMU pipes. This implementation is inspired by the [Anbox project](https://github.com/anbox/anbox) and provides an alternative to the proprietary `libOpenglRender.so`.

## Architecture

The new renderer consists of three main components:

### 1. QEMU Pipe Connection (`pipe.rs`)

Handles low-level communication with the container through the QEMU pipe device (`/dev/qemu_pipe`). The pipe connection:

- Connects to OpenGL ES services in the container (`/opengles`, `/opengles2`, `/opengles3`)
- Automatically falls back from OpenGL ES 3 → 2 → 1 based on availability
- Provides read/write operations for command and data transfer

### 2. OpenGL ES Protocol (`opengles.rs`)

Implements the OpenGL ES command protocol for communication with the container's graphics backend:

- **Initialize**: Set up the GL context with display dimensions and DPI
- **SetWindowSize**: Update surface and framebuffer dimensions
- **SwapBuffers**: Display rendered frames
- **Repaint**: Refresh the display
- **Destroy**: Clean up GL resources

### 3. Renderer Interface (`renderer.rs`)

Provides a high-level API that mimics the old renderer's interface:

- `start_renderer()`: Initialize the OpenGL renderer
- `set_native_window()`: Set the Android native window
- `reset_window()`: Update window parameters
- `remove_window()`: Remove a window
- `destroy_subwindow()`: Destroy the GL context
- `repaint_display()`: Trigger a display refresh

## Renderer Selection

Users can choose between the old and new renderer via the app settings:

1. Go to **Settings** → **Advanced**
2. Toggle **Use New Renderer** checkbox
3. Reboot the container for changes to take effect

The setting is profile-specific, so different profiles can use different renderers.

## Debug Renderer Mode

For advanced debugging and diagnostics, the app now includes a **Debug Renderer** option that captures extensive logging when enabled.

### Enabling Debug Mode

1. Go to **Settings** → **Advanced**
2. Toggle **Debug Renderer** checkbox
3. Reboot the container for changes to take effect

**⚠️ WARNING**: Debug mode produces **very large log files** and should only be enabled when troubleshooting renderer issues. The default is OFF.

### What Gets Logged

When debug renderer mode is enabled, the following data is dumped to `/sdcard/twoyi_renderer_debug/`:

1. **QEMU Pipe Communication** (`pipe_*.log`)
   - All data written to OpenGL ES pipes (`/opengles`, `/opengles2`, `/opengles3`)
   - All data read from pipes
   - Data is logged in hex, ASCII, and as integers

2. **OpenGL ES Commands** (`opengles_commands.log`)
   - All GL commands sent (Initialize, SetWindowSize, SwapBuffers, etc.)
   - Command parameters and timestamps

3. **Gralloc Buffer Operations** (`gralloc_*.log`)
   - Buffer lock/unlock operations
   - Buffer dimensions, stride, and format
   - Buffer posting events

4. **Container Socket Communication** (`socket_*.log`)
   - Input sockets: `/dev/input/key0`, `/dev/input/touch`
   - Service sockets: property_service, vold, cryptd, netd, dnsproxyd, mdns, fwmarkd, zygote, webview_zygote
   - Binder sockets: vbinder, vndbinder, hwbinder (bcs, bhs, bis)
   - Debug socket: `/data/system/ndebugsocket`
   - All socket data in hex and ASCII

### Log File Location

All debug logs are written to: `<app_data_dir>/files/twoyi_renderer_debug/`

This is the app's private files directory and is automatically included in the **Send Log** bugreport when debug renderer is enabled.

You can access these logs:
1. Using the **Send Log** feature in Settings (creates bugreport.zip with logs)
2. Via `adb` if the device is rooted
3. Via Android backup/restore mechanisms

### Performance Impact

Debug mode has significant performance impact due to extensive file I/O operations. It should **only** be used for debugging purposes and disabled during normal use.

## Comparison: Old vs New Renderer

| Feature | Old Renderer | New Renderer |
|---------|--------------|--------------|
| Implementation | Proprietary C++ library | Open-source Rust |
| Source Code | Closed source | Fully open source |
| License | Unknown | MPL 2.0 |
| Communication | Unknown protocol | QEMU pipe (documented) |
| Fallback | No automatic fallback | Auto-fallback ES 3→2→1 |
| Dependencies | libOpenglRender.so | None (pure Rust) |

## Technical Details

### QEMU Pipe Protocol

The QEMU pipe is a virtual device that provides communication between the guest (container) and host (Android app). The protocol is simple:

1. Open `/dev/qemu_pipe` device
2. Write service name (e.g., `/opengles2`) to establish connection
3. Exchange data using standard read/write operations

### OpenGL ES Command Protocol

Commands are sent as 32-bit integers followed by parameters:

```rust
// Example: Initialize GL context
Command: 0x1000 (Initialize)
Params:
  - width: i32
  - height: i32
  - xdpi: i32
  - ydpi: i32
  - fps: i32
```

### Thread Safety

The renderer uses Rust's `Mutex` and atomic operations to ensure thread-safe access to shared state. The GL context is managed globally and can be safely accessed from multiple threads.

## Limitations

The new renderer is **experimental** and may have the following limitations:

1. **QEMU Pipe Availability**: Requires `/dev/qemu_pipe` device to be available in the container. If not available, the renderer automatically falls back to the old implementation.

2. **Protocol Compatibility**: The OpenGL ES protocol implementation is based on Anbox's approach and may need adjustments for specific container configurations.

3. **Performance**: As this is a new implementation, performance characteristics may differ from the old renderer.

## Troubleshooting

### Renderer Fails to Start

If the new renderer fails to start:

1. Check if `/dev/qemu_pipe` exists in the container
2. Verify OpenGL ES services are running in the container
3. Check logcat for error messages (tag: `CLIENT_EGL`)
4. Switch back to the old renderer if issues persist

### Black Screen or No Display

If you see a black screen:

1. Ensure the container is fully booted
2. Try toggling back to the old renderer
3. Check if the container's graphics stack is properly configured

## Future Improvements

Potential enhancements for the new renderer:

- [ ] Full OpenGL ES 3.x feature support
- [ ] Hardware acceleration integration
- [ ] Performance optimizations
- [ ] Better error handling and diagnostics
- [x] Detailed logging for debugging (Debug Renderer mode)
- [ ] Support for additional graphics protocols

## References

- [Anbox Graphics Implementation](https://github.com/anbox/anbox/tree/master/src/anbox/graphics)
- [QEMU Pipe Connection Creator](https://github.com/anbox/anbox/blob/master/src/anbox/qemu/pipe_connection_creator.h)
- [Android QEMU Pipe Documentation](https://android.googlesource.com/platform/external/qemu/+/master/docs/ANDROID-QEMU-PIPE.TXT)

## Contributing

Contributions to improve the new renderer are welcome! Areas where help is needed:

- Testing on different devices and Android versions
- Performance benchmarking and optimization
- Protocol implementation improvements
- Documentation and examples
- Bug fixes and error handling

## License

The new renderer is licensed under the Mozilla Public License 2.0 (MPL 2.0), the same as the rest of the Twoyi project.
