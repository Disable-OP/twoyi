# Redroid Testing Notes

## Architecture Mismatch

The default `redroid/redroid:12.0.0-latest` image is x86_64, but `libtwoyi.so` is built for ARM64 (aarch64).

When attempting to run the ARM64 library in x86_64 redroid:
```bash
docker exec redroid-test /system/bin/linker64 /data/local/tmp/libtwoyi.so --help
# Error: "/data/local/tmp/libtwoyi.so" is for EM_AARCH64 (183) instead of EM_X86_64 (62)
```

## Solutions for Testing

### Option 1: Use ARM64 Redroid (Recommended)
Use an ARM64-compatible system or check if redroid provides arm64 images.

### Option 2: Build for x86_64
Build the library for x86_64-linux-android:
```bash
rustup target add x86_64-linux-android
cd app/rs
# Modify build_rs.sh to build for x86_64
cargo xdk -t x86_64 build --release
```

Note: There may be compilation issues in the input module that need to be fixed for x86_64.

### Option 3: Use Real ARM64 Device
Test on an actual ARM64 Android device using adb.

## Test Commands (for ARM64 Environment)

Once you have an ARM64 environment:

```bash
# Start redroid (ARM64 version)
docker run -d --privileged --name redroid-arm64 -p 5555:5555 <arm64-redroid-image>

# Copy files
docker cp app/src/main/jniLibs/arm64-v8a/libtwoyi.so redroid-arm64:/data/local/tmp/
docker cp app/src/main/jniLibs/arm64-v8a/libloader.so redroid-arm64:/data/local/tmp/
docker cp app/src/main/jniLibs/arm64-v8a/libOpenglRender.so redroid-arm64:/data/local/tmp/

# Test 1: Basic invocation (RUNPATH should find libraries in same directory)
docker exec redroid-arm64 sh -c 'cd /data/local/tmp && /system/bin/linker64 /data/local/tmp/libtwoyi.so --help'

# Test 2: With arguments (verify argc/argv parsing)
docker exec redroid-arm64 sh -c '/system/bin/linker64 /data/local/tmp/libtwoyi.so --loader /data/local/tmp/libloader.so --width 1920 --height 1080'

# Test 3: Verify no LD_LIBRARY_PATH needed (thanks to RUNPATH=$ORIGIN)
docker exec redroid-arm64 sh -c 'cd /data/local/tmp && /system/bin/linker64 /data/local/tmp/libtwoyi.so --help'
# Should work without setting LD_LIBRARY_PATH
```

## Current Implementation Status

✅ **RUNPATH configured**: Set to `$ORIGIN` so libraries are found in current directory
✅ **Entry point set**: Points to `main` function for execution  
✅ **Argument parsing**: Fixed to properly read argc/argv using `libc::c_char`
❌ **Direct ./libtwoyi.so execution**: Not possible - shared libraries can't have INTERP segment
✅ **No LD_LIBRARY_PATH needed**: RUNPATH finds dependencies automatically

## Workaround for Direct Execution

Since shared libraries cannot have an INTERP segment, direct execution (`./libtwoyi.so`) is not possible. The library must be invoked via linker64:

```bash
/system/bin/linker64 /path/to/libtwoyi.so --help
```

However, with RUNPATH set to `$ORIGIN`, you no longer need to set `LD_LIBRARY_PATH`.
