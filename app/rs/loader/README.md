# Copyright Disclaimer: AI-Generated Content
# This file was created by GitHub Copilot, an AI coding assistant.
# AI-generated content is not subject to copyright protection and is provided
# without any warranty, express or implied, including warranties of merchantability,
# fitness for a particular purpose, or non-infringement.
# Use at your own risk.

# Open-Source Dynamic Library Loader

This directory contains the source code for an open-source dynamic library loader (`libloader.so` / `loader64`), which is a complete replacement for the proprietary legacy loader.

## Overview

The new loader is a simple but powerful dynamic library loader written in Rust. It provides the same functionality as the legacy libloader.so but with full source code available under the MPL 2.0 license.

## Architecture

The loader consists of a single Rust module that:

1. **lib.rs** - Main loader implementation with dlopen/dlsym wrappers

## Key Features

- **Dynamic Library Loading**: Load shared libraries at runtime using `dlopen`
- **Symbol Resolution**: Find and execute functions within loaded libraries using `dlsym`
- **Direct Execution**: Can be executed directly as `loader64` thanks to PIE executable configuration
- **C API**: Provides C-compatible functions for integration with existing code
- **Error Handling**: Proper error reporting using `dlerror`

## Exported API

The library exports the following functions:

### C API Functions

```c
// Initialize the loader (optional)
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

### Rust API Functions

```rust
// Load a library with custom flags
pub unsafe fn load_library(path: &str, flags: c_int) -> Result<*mut c_void, String>

// Find a symbol in a library
pub unsafe fn find_symbol(handle: *mut c_void, symbol: &str) -> Result<*mut c_void, String>

// Close a library
pub unsafe fn close_library(handle: *mut c_void) -> Result<(), String>
```

## Building

To build the loader:

```bash
./build.sh
```

Or manually:

```bash
cargo xdk -t arm64-v8a build --release
cp target/aarch64-linux-android/release/libloader.so ../../src/main/jniLibs/arm64-v8a/libloader_new.so
chmod +x ../../src/main/jniLibs/arm64-v8a/libloader_new.so
```

## Requirements

- Rust toolchain with `aarch64-linux-android` target
- cargo-xdk (Android cross-compilation tool)
- Android NDK

## Size Comparison

- Legacy library: 50 KB (stripped, proprietary)
- New library: 455 KB (not stripped, includes Rust std library)

The new library is larger because it includes Rust's standard library and is not stripped. Stripping would reduce the size significantly, but we keep symbols for debugging.

## Usage

### As a Shared Library

Load the library in your application:

```c
#include <dlfcn.h>

// Load a library
void* handle = loader_load("/path/to/library.so");

// Find a symbol
void* symbol = loader_symbol(handle, "function_name");

// Use the symbol...

// Close the library
loader_close(handle);
```

### As a Direct Executable (loader64)

Execute the loader directly to load and run a library:

```bash
# Via symlink (as used in Twoyi)
ln -s libloader_new.so loader64
./loader64 /path/to/library.so [args...]

# Or with LD_PRELOAD
LD_PRELOAD=/path/to/library.so ./loader64
```

## Implementation Details

The loader is built as a PIE (Position Independent Executable) which allows it to be:

1. **Loaded as a library**: Can be loaded by other programs via `System.loadLibrary()` or `dlopen()`
2. **Executed directly**: Can be run as a standalone executable

Key implementation details:

- Uses `/system/bin/linker64` as the dynamic linker (configured in build.rs)
- Entry point is set to `main` function
- Uses `RTLD_NOW | RTLD_GLOBAL` flags for dlopen by default
- Proper error handling with dlerror()

## Integration with Twoyi

The loader is used in Twoyi via:

1. **Symlink Creation**: `RomManager.java` creates a symlink from `loader64` to `libloader.so`
2. **Environment Variable**: Set via `TYLOADER` environment variable in container init
3. **Bootstrap**: Used to bootstrap the container environment

## Verification

Check the exported symbols:

```bash
nm -D libloader_new.so | grep -E "main|loader_"
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
readelf -h libloader_new.so | grep "Entry point"
```

Expected: Entry point address should not be 0x0

## Troubleshooting

### Library fails to load

Check if the target architecture matches:

```bash
file libloader_new.so
```

Should show: `ELF 64-bit LSB shared object, ARM aarch64`

### Symbol not found

Make sure the library is built in release mode and symbols are exported.

### Execution fails

Verify the entry point is set:

```bash
readelf -h libloader_new.so | grep "Entry"
```

## License

This loader is licensed under the Mozilla Public License 2.0 (MPL 2.0), same as the rest of the Twoyi project.

## Contributing

Contributions are welcome! The code is written in Rust and follows standard Rust conventions.
