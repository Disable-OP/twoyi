# libtwoyi.so - Dual-Mode Execution Guide

## Overview

The `libtwoyi.so` library has been refactored to support two modes of execution:

1. **JNI Library Mode**: Traditional usage via `System.loadLibrary("twoyi")` in the Android app
2. **Shell Executable Mode**: Direct execution from the command line via `./libtwoyi.so`

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

The compiled library will be at: `app/src/main/jniLibs/arm64-v8a/libtwoyi.so`

## Usage

### 1. JNI Library Mode (Default)

This is the traditional way the app uses the library. The app loads it via:

```java
System.loadLibrary("twoyi");
```

The JNI entry point `JNI_OnLoad` registers native methods that can be called from Java.

### 2. Shell Executable Mode

The library can now be executed directly from the shell (adb shell or termux):

```bash
# Copy to device (example)
adb push app/src/main/jniLibs/arm64-v8a/libtwoyi.so /data/local/tmp/

# Make executable (already set by build script)
adb shell chmod +x /data/local/tmp/libtwoyi.so

# Execute with help
adb shell /data/local/tmp/libtwoyi.so --help

# Execute to show usage
adb shell /data/local/tmp/libtwoyi.so
```

### Available Command-Line Options

```
Usage: ./libtwoyi.so [OPTIONS]
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
adb shell /data/local/tmp/libtwoyi.so --start-input --width 1080 --height 1920
```

## Implementation Details

### Entry Points

The library has two entry points:

1. **`JNI_OnLoad`**: Called when loaded via `System.loadLibrary()` from Java
2. **`main`**: Called when executed directly from shell

Both entry points are exported and can be verified with:

```bash
nm -D libtwoyi.so | grep -E "(main|JNI_OnLoad)"
```

### Linker Configuration

The library is configured with:
- Built as a `cdylib` (C-compatible dynamic library)
- Entry point set to `main` function via linker flag: `-Wl,-e,main`
- Executable permissions set by build script

### Target SDK

The app targets SDK 28 to bypass W^X (Write XOR Execute) restrictions, allowing execution in `/data/user/0/`.

## Architecture

The code is organized into modules:

- **`lib.rs`**: JNI entry points and main() function for shell execution
- **`core.rs`**: Shared renderer functionality used by both modes
- **`input.rs`**: Input system for touch and keyboard events
- **`renderer_bindings.rs`**: FFI bindings to OpenGL renderer

## Testing

### Test JNI Mode
Build and install the APK, then launch the app. The library should load normally.

### Test Shell Execution Mode
```bash
# Push library to device
adb push app/src/main/jniLibs/arm64-v8a/libtwoyi.so /data/local/tmp/

# Execute
adb shell /data/local/tmp/libtwoyi.so --help
```

Expected output:
```
Twoyi Renderer - Standalone Mode
argc: 2
Arguments:
  [0]: "/data/local/tmp/libtwoyi.so"
  [1]: "--help"

Usage: ./libtwoyi.so [OPTIONS]
...
```

## Security Considerations

- The library maintains all existing security properties
- Shell execution mode has limited functionality (primarily diagnostic/utility)
- Full renderer functionality requires proper Android context (Surface, etc.)
- Input system can be started independently for testing

## Compatibility

- **Target SDK**: 28 (maintains W^X bypass)
- **Min SDK**: 27
- **Architecture**: arm64-v8a only
- **NDK Version**: v22 or lower recommended

## Troubleshooting

### Library not executable
```bash
adb shell chmod +x /path/to/libtwoyi.so
```

### Permission denied
Ensure the library is in a directory with execute permissions (e.g., `/data/local/tmp/` works with adb).

### Symbol not found
Verify entry points exist:
```bash
nm -D libtwoyi.so | grep main
nm -D libtwoyi.so | grep JNI_OnLoad
```

## Future Enhancements

Potential improvements:
- Add more CLI commands for diagnostics
- Support for standalone testing of renderer components
- Enhanced logging and debugging output in CLI mode
- Integration with testing frameworks
