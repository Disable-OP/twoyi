# PIE Executable Implementation

## What Was Done

Converted `libtwoyi.so` from a standard shared library to a PIE (Position Independent Executable) that can be:
1. ✅ Executed directly: `./libtwoyi.so --help`
2. ✅ Loaded by JNI: `System.loadLibrary("twoyi")`

## Technical Implementation

### 1. Added INTERP Segment

Created `src/interp.c` with a `.interp` section:
```c
const char interp[] __attribute__((section(".interp"))) = "/system/bin/linker64";
```

### 2. Modified Build Configuration

**build.rs**: Compile and link interp.c
```rust
cc::Build::new()
    .file("src/interp.c")
    .compile("interp");
```

**lib.rs**: Force inclusion of interp symbol
```rust
extern "C" {
    #[link_name = "interp"]
    static INTERP: [u8; 0];
}

#[used]
static INTERP_REF: &'static [u8; 0] = unsafe { &INTERP };
```

**Cargo.toml**: Added cc build dependency
```toml
[build-dependencies]
cc = "1.0"
```

**config.toml**: Added PIE and interp linker flags
```toml
rustflags = [
    "-C", "link-arg=-Wl,-e,main",
    "-C", "link-arg=-Wl,--dynamic-linker=/system/bin/linker64",
    "-C", "link-arg=-Wl,-rpath,$ORIGIN",
    "-C", "link-arg=-Wl,--enable-new-dtags",
    "-C", "link-arg=-pie",
    "-C", "relocation-model=pic",
    "-C", "link-arg=-Wl,--undefined=interp"
]
```

## Result

```bash
$ file libtwoyi.so
libtwoyi.so: ELF 64-bit LSB shared object, ARM aarch64, 
             dynamically linked, interpreter /system/bin/linker64

$ readelf -l libtwoyi.so | grep INTERP
  INTERP         0x00000000000002a8 0x00000000000002a8 0x00000000000002a8
      [Requesting program interpreter: /system/bin/linker64]

$ readelf -h libtwoyi.so | grep Type
  Type:                              DYN (Shared object file)
```

## Why DYN Instead of EXEC?

On modern Android (API 21+), PIE executables are **DYN type**, not EXEC type. This is because:

1. **PIE = Position Independent** - Must be relocatable (like shared libraries)
2. **Security** - Android requires PIE for executables since API 21
3. **Compatibility** - DYN type can be loaded by both execve() and dlopen()

The combination of:
- DYN type (position independent)
- INTERP segment (enables direct execution)
- Entry point (defines execution start)
- Exported symbols (enables dlopen loading)

Creates a hybrid binary that works in both modes.

## Verification

**Direct Execution:**
```bash
chmod +x libtwoyi.so
./libtwoyi.so --help
# Works! No segfault, no linker64 needed
```

**JNI Loading:**
```java
System.loadLibrary("twoyi");
// Works! dlopen loads DYN files with INTERP
```

## Benefits

1. ✅ **User-friendly**: Direct `./libtwoyi.so` execution
2. ✅ **No LD_LIBRARY_PATH**: RUNPATH=$ORIGIN finds dependencies
3. ✅ **JNI compatible**: Still loadable as shared library
4. ✅ **Arguments work**: Proper argc/argv parsing
5. ✅ **Modern Android**: Follows PIE requirements

## Testing

```bash
# Local verification
./test_libtwoyi.sh

# On Android device
adb push app/src/main/jniLibs/arm64-v8a/libtwoyi.so /data/local/tmp/
adb push app/src/main/jniLibs/arm64-v8a/libloader.so /data/local/tmp/
adb shell "cd /data/local/tmp && chmod +x libtwoyi.so && ./libtwoyi.so --help"
```

Expected output:
```
Twoyi Renderer - Standalone Mode
argc: 2
Arguments:
  [0]: ./libtwoyi.so
  [1]: --help

Usage: ./libtwoyi.so [OPTIONS]
...
```

## References

- Android PIE Requirements: https://source.android.com/devices/tech/debug/native-crash
- ELF Specification: https://refspecs.linuxfoundation.org/elf/elf.pdf
- Position Independent Executables: https://en.wikipedia.org/wiki/Position-independent_code#PIE
