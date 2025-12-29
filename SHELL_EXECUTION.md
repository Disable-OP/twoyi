# libtwoyi.so - Dual-Mode Execution Guide

## Overview

The `libtwoyi.so` library has been refactored to support two modes of execution:

1. **JNI Library Mode**: Traditional usage via `System.loadLibrary("twoyi")` in the Android app
2. **Shell Executable Mode**: Direct execution from the command line

## Building

The library is built automatically as part of the app build process:

```bash
./gradlew assembleRelease
```

Or build just the Rust library:

```bash
cd app/rs
sh build_rs.sh --release
```

The compiled files will be at: 
- `app/src/main/jniLibs/arm64-v8a/libtwoyi.so` - The shared library
- `app/src/main/jniLibs/arm64-v8a/twoyi` - Wrapper script for shell execution

## Usage

### 1. JNI Library Mode (Default - Used by App)

This is the traditional way the app uses the library. The app loads it via:

```java
System.loadLibrary("twoyi");
```

The JNI entry point `JNI_OnLoad` registers native methods that can be called from Java. All renderer and input functionality is available through the JNI interface.

### 2. Shell Executable Mode

The library can be executed from the shell in two ways:

#### Method A: Using the Wrapper Script (Recommended)

```bash
# Copy to device
adb push app/src/main/jniLibs/arm64-v8a/twoyi /data/local/tmp/
adb push app/src/main/jniLibs/arm64-v8a/libtwoyi.so /data/local/tmp/

# Execute
adb shell /data/local/tmp/twoyi --help
adb shell /data/local/tmp/twoyi --start-input --width 1080 --height 1920
```

The wrapper script uses Android's linker to properly load and execute the library.

#### Method B: Direct Execution (Advanced)

The library has a custom entry point and can be executed directly:

```bash
# Copy to device
adb push app/src/main/jniLibs/arm64-v8a/libtwoyi.so /data/local/tmp/

# Execute using linker64 directly
adb shell /system/bin/linker64 /data/local/tmp/libtwoyi.so --help
```

**Note**: Direct execution may cause segfaults if the Android linker doesn't properly initialize the Rust runtime. Use Method A (wrapper script) for reliable execution.

### Available Command-Line Options

```
Usage: twoyi [OPTIONS] or ./libtwoyi.so [OPTIONS]
Options:
  --help                Show this help message
  --width <width>       Set virtual display width (default: 720)
  --height <height>     Set virtual display height (default: 1280)
  --loader <path>       Set loader path  
  --start-input         Start input system only
```

### Example: Starting Input System Standalone

```bash
# Start the input system with custom dimensions
adb shell /data/local/tmp/twoyi --start-input --width 1080 --height 1920
```

## Exported C Functions

The library exports C functions that provide the same functionality as the JNI interface, but can be called from shell tools, other native code, or via `dlopen`/`dlsym`:

- `twoyi_start_input_system(width, height)` - Start the input system
- `twoyi_print_help()` - Display help information
- `twoyi_send_keycode(keycode)` - Send a keycode event

These functions can be called from other native code by linking against `libtwoyi.so`.

## Implementation Details

### Entry Points

The library has multiple entry points:

1. **`JNI_OnLoad`**: Called when loaded via `System.loadLibrary()` from Java
2. **`main`**: Called when executed directly or via linker
3. **`twoyi_*`**: Exported C functions for programmatic access

All entry points can be verified with:

```bash
nm -D libtwoyi.so | grep -E "(main|JNI_OnLoad|twoyi_)"
```

### Linker Configuration

The library is configured with:
- Built as a `cdylib` (C-compatible dynamic library)
- Entry point set to `main` function via linker flag: `-Wl,-e,main`
- All symbols exported via: `-Wl,--export-dynamic`
- Executable permissions set by build script

### Wrapper Script

The `twoyi` wrapper script invokes the Android linker to properly load and execute the library:

```sh
#!/system/bin/sh
exec /system/bin/linker64 "$SCRIPT_DIR/libtwoyi.so" "$@"
```

This ensures proper runtime initialization that might not happen with direct execution.

### Target SDK

The app targets SDK 28 to bypass W^X (Write XOR Execute) restrictions, allowing execution in `/data/user/0/`.

## Architecture

The code is organized into modules:

- **`lib.rs`**: JNI entry points, main() function, and exported C functions
- **`core.rs`**: Shared renderer functionality used by both JNI and shell modes
- **`input.rs`**: Input system for touch and keyboard events (same in both modes)
- **`renderer_bindings.rs`**: FFI bindings to OpenGL renderer (same in both modes)

## Same Functions in Both Modes

**Both JNI mode and shell execution mode have access to the same core functionality:**

| Function | JNI Mode (App) | Shell Mode (CLI) |
|----------|---------------|------------------|
| Start Input System | `Renderer.init()` → `renderer_init()` → `core::init_renderer()` | `twoyi --start-input` → `main()` → `twoyi_start_input_system()` |
| Send Keycode | `Renderer.sendKeycode()` → `send_key_code()` | `twoyi_send_keycode()` (via dlopen) |
| Reset Window | `Renderer.resetWindow()` → `renderer_reset_window()` | `core::reset_window()` (via dlopen) |

All functions ultimately call the same underlying implementations in the `core` and `input` modules.

## Testing

### Test JNI Mode (App)
Build and install the APK, then launch the app. The library should load normally and all features should work.

```bash
./gradlew assembleRelease
adb install app/build/outputs/apk/release/*.apk
# Launch the app
```

### Test Shell Execution Mode

```bash
# Push files to device
adb push app/src/main/jniLibs/arm64-v8a/twoyi /data/local/tmp/
adb push app/src/main/jniLibs/arm64-v8a/libtwoyi.so /data/local/tmp/

# Test help
adb shell /data/local/tmp/twoyi --help

# Test input system (requires proper permissions)
adb shell /data/local/tmp/twoyi --start-input --width 720 --height 1280
```

Expected output:
```
Twoyi Renderer - Standalone Mode
argc: 2
Arguments:
  [0]: /data/local/tmp/libtwoyi.so
  [1]: --help

Usage: ./libtwoyi.so [OPTIONS]
...
```

## Security Considerations

- The library maintains all existing security properties in both modes
- Shell execution mode has the same functionality as JNI mode
- Input system requires appropriate permissions (same in both modes)
- Renderer functionality requires proper Android context (Surface) which is only available through JNI mode

## Compatibility

- **Target SDK**: 28 (maintains W^X bypass)
- **Min SDK**: 27  
- **Architecture**: arm64-v8a only
- **NDK Version**: v22 or lower recommended
- **Shell**: Requires `/system/bin/sh` and `/system/bin/linker64`

## Troubleshooting

### Segmentation Fault When Running ./libtwoyi.so

**Problem**: Direct execution causes segfault due to incomplete runtime initialization.

**Solution**: Use the wrapper script instead:
```bash
adb shell /data/local/tmp/twoyi --help
```

Or use linker64 directly:
```bash
adb shell /system/bin/linker64 /data/local/tmp/libtwoyi.so --help
```

### Library not executable
```bash
adb shell chmod +x /path/to/libtwoyi.so
adb shell chmod +x /path/to/twoyi
```

### Permission denied
Ensure the library is in a directory with execute permissions (e.g., `/data/local/tmp/` works with adb).

### Symbol not found
Verify entry points and exported functions exist:
```bash
nm -D libtwoyi.so | grep main
nm -D libtwoyi.so | grep JNI_OnLoad
nm -D libtwoyi.so | grep twoyi_
```

## Future Enhancements

Potential improvements:
- Add more CLI commands for diagnostics and testing
- Standalone testing framework for renderer components
- Enhanced logging and debugging output in CLI mode
- Interactive shell mode for live testing
- Integration with Android debugging tools
