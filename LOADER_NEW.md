# Copyright Disclaimer: AI-Generated Content
# This file was created by GitHub Copilot, an AI coding assistant.
# AI-generated content is not subject to copyright protection and is provided
# without any warranty, express or implied, including warranties of merchantability,
# fitness for a particular purpose, or non-infringement.
# Use at your own risk.

# New libloader.so (loader64) - Usage Guide

## Overview

This guide explains how to use the new open-source `libloader.so` (also known as `loader64`) that has been created as a complete replacement for the legacy proprietary loader.

## What Was Created

A new standalone Rust library that:
- **Provides dynamic library loading** via dlopen/dlsym
- **Can be executed directly** as loader64 thanks to PIE configuration
- **Is fully open-source** with MPL 2.0 license
- **Matches the legacy loader's functionality** while being more maintainable

## Library Location

**Source code**: `app/rs/loader/`  
**Built library**: `app/src/main/jniLibs/arm64-v8a/libloader_new.so`  
**Legacy library**: `app/src/main/jniLibs/arm64-v8a/libloader.so`

## Exported API

The new loader exports these functions:

### C API

```c
// Initialize the loader
void loader_init();

// Load a library by path
void* loader_load(const char* path);

// Find a symbol in a loaded library
void* loader_symbol(void* handle, const char* symbol);

// Close a loaded library
int loader_close(void* handle);

// Main entry point for direct execution
int main(int argc, char** argv);
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
cd app/rs/loader
./build.sh
```

**Manual build**:
```bash
cd app/rs/loader
cargo xdk -t arm64-v8a build --release
cp target/aarch64-linux-android/release/libloader.so ../../src/main/jniLibs/arm64-v8a/libloader_new.so
chmod +x ../../src/main/jniLibs/arm64-v8a/libloader_new.so
```

## Verification

Check that all required symbols are exported:

```bash
nm -D app/src/main/jniLibs/arm64-v8a/libloader_new.so | grep -E "main|loader_"
```

Expected output:
```
0000000000014e20 T loader_close
0000000000014e98 T loader_init
0000000000014e9c T loader_load
0000000000014f34 T loader_symbol
0000000000014fe0 T main
```

Check the entry point:
```bash
readelf -h app/src/main/jniLibs/arm64-v8a/libloader_new.so | grep "Entry point"
```

Expected: Entry point address should not be 0x0

## Using the New Library

### Option 1: Replace the Legacy Library

The safest way is to rename the new library:

```bash
cd app/src/main/jniLibs/arm64-v8a/
# Backup the legacy library
mv libloader.so libloader_legacy.so
# Use the new library
cp libloader_new.so libloader.so
```

Then rebuild the app:
```bash
./gradlew assembleRelease
```

### Option 2: Update Symlink Code

Modify `RomManager.java` to use the new library:

```java
// In createLoaderSymlink() method
private static final String LOADER_FILE = "libloader_new.so"; // Changed from libloader.so
```

### Option 3: Symlink Configuration

The loader is used via a symlink created at runtime:

```java
// From RomManager.java
Path loaderSymlink = new File(context.getDataDir(), "loader64").toPath();
String loaderPath = getLoaderPath(context); // Points to libloader.so
Files.createSymbolicLink(loaderSymlink, Paths.get(loaderPath));
```

The symlink `loader64` is then used in the container's environment via the `TYLOADER` environment variable.

## Testing

After building and integrating the new library:

1. **Check the file**:
   ```bash
   file app/src/main/jniLibs/arm64-v8a/libloader_new.so
   ```
   
   Should show:
   ```
   ELF 64-bit LSB shared object, ARM aarch64, version 1 (SYSV), dynamically linked
   ```

2. **Verify symbols**:
   ```bash
   nm -D app/src/main/jniLibs/arm64-v8a/libloader_new.so | grep main
   ```

3. **Test in app**:
   - Build the app with the new loader
   - Launch a container
   - Check logcat for loader messages
   - Verify the container boots successfully

## Architecture

The new loader uses:

1. **dlopen/dlsym**: Standard POSIX dynamic loading APIs
2. **PIE Executable**: Position Independent Executable configuration
3. **Main Entry Point**: Can be executed directly via the linker
4. **C API**: Compatible with existing code expecting a C interface

## Troubleshooting

### Library fails to load

Check if the library was built correctly:
```bash
file app/src/main/jniLibs/arm64-v8a/libloader_new.so
```

Should show:
```
ELF 64-bit LSB shared object, ARM aarch64, version 1 (SYSV), dynamically linked
```

### Loader fails to execute

Check the entry point:
```bash
readelf -h app/src/main/jniLibs/arm64-v8a/libloader_new.so | grep "Entry point"
```

Should show a non-zero address.

### Symbol not found

Verify all required symbols are exported:
```bash
nm -D app/src/main/jniLibs/arm64-v8a/libloader_new.so | grep -E "loader_|main"
```

### Container fails to boot

Check logcat for error messages:
```bash
adb logcat | grep -i "loader\|TYLOADER"
```

## Comparison: New vs Legacy

| Feature | Legacy | New |
|---------|--------|-----|
| Size | 50 KB | 455 KB |
| Source Code | Closed | Open (MPL 2.0) |
| Implementation | Unknown (stripped) | Rust (documented) |
| Symbols | Stripped | Available for debugging |
| Dependencies | Unknown | Rust std library |
| Entry Point | 0xce0 | 0x14fe0 |
| Maintainability | Low (no source) | High (full source) |

## Future Enhancements

Potential improvements:
- [ ] Strip symbols to reduce size
- [ ] Add more error reporting
- [ ] Support for additional dlopen flags
- [ ] Environment variable configuration
- [ ] Logging and debugging features

## Integration Notes

The loader is used in Twoyi to:

1. **Bootstrap the container**: Acts as the initial loader for the container environment
2. **Dynamic library loading**: Loads libraries needed by the init process
3. **Symlink target**: Created as `loader64` symlink in the app's data directory
4. **Environment configuration**: Set via `TYLOADER` environment variable

## Contributing

The source code is in `app/rs/loader/`. Contributions are welcome!

1. Make changes in the source files
2. Build and test: `./build.sh`
3. Verify symbols and functionality
4. Submit a PR

## License

The new loader is licensed under MPL 2.0, same as the Twoyi project.

## Support

For issues or questions:
1. Check logcat for error messages
2. Verify the library is built correctly
3. Compare with legacy library behavior
4. File an issue with logs and reproduction steps
