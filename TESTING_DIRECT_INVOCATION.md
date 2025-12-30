# Testing libtwoyi.so Direct Invocation

This document provides instructions for testing the direct invocation of libtwoyi.so via linker64 on an Android device.

## Prerequisites

- Android device with ARM64 architecture (aarch64)
- ADB installed and device connected
- Root access or access to /data/local/tmp (adb shell has access by default)

## Build the Library

First, build the library with the entry point configured:

```bash
cd app/rs
sh build_rs.sh --release
```

This will create `app/src/main/jniLibs/arm64-v8a/libtwoyi.so` with the entry point set to `main`.

## Verify Library Structure (Optional)

Run the test script to verify the library has the correct structure:

```bash
./test_libtwoyi.sh
```

Expected output:
```
Testing libtwoyi.so structure...
================================

✓ Library exists at app/src/main/jniLibs/arm64-v8a/libtwoyi.so
✓ Entry point is set (non-zero)
✓ main symbol found at address: 0x000000000003a3a8
✓ JNI_OnLoad symbol found
✓ Found 3 exported twoyi_* functions
```

## Test on Android Device

### Test 1: Help Message

Push the library to the device and invoke it with --help:

```bash
# Push library to device
adb push app/src/main/jniLibs/arm64-v8a/libtwoyi.so /data/local/tmp/

# Push dependencies if needed
adb push app/src/main/jniLibs/arm64-v8a/libloader.so /data/local/tmp/

# Test help message
adb shell LD_LIBRARY_PATH=/data/local/tmp /system/bin/linker64 /data/local/tmp/libtwoyi.so --help
```

Expected output:
```
Twoyi Renderer - Standalone Mode
Arguments received: 2
Arguments:
  [0]: /data/local/tmp/libtwoyi.so
  [1]: --help

Usage: twoyi [OPTIONS] or linker64 libtwoyi.so [OPTIONS]
Options:
  --help                Show this help message
  --width <width>       Set virtual display width (default: 720)
  --height <height>     Set virtual display height (default: 1280)
  --loader <path>       Set loader path
  --start-input         Start input system only
...
```

### Test 2: With --loader Argument (As in Problem Statement)

Test the exact command from the problem statement:

```bash
adb shell "cd /data/local/tmp && LD_LIBRARY_PATH=. /system/bin/linker64 \$(realpath libtwoyi.so) --loader \$(realpath libloader.so) --help"
```

This should work without segmentation fault and display the help message.

### Test 3: Start Input System

If you have appropriate permissions:

```bash
adb shell LD_LIBRARY_PATH=/data/local/tmp /system/bin/linker64 /data/local/tmp/libtwoyi.so --start-input --width 720 --height 1280
```

Note: This requires appropriate permissions to access the input system.

## Test JNI Mode (App)

To verify that JNI mode still works:

1. Build the APK:
```bash
./gradlew assembleRelease
```

2. Install on device:
```bash
adb install app/build/outputs/apk/release/*.apk
```

3. Launch the Twoyi app and verify it works normally:
   - App should start without crashes
   - Virtual Android system should boot
   - Touch and keyboard input should work
   - All features should function as before

## Troubleshooting

### "Segmentation fault" error

If you still get a segmentation fault, verify:
1. The library was built with the entry point: `readelf -h libtwoyi.so | grep Entry`
   - Should show a non-zero address like `0x3a3a8`
2. The main symbol exists: `nm -D libtwoyi.so | grep " T main"`
   - Should show the main function address

### "cannot execute binary file"

This is expected if you try to execute the .so directly: `./libtwoyi.so`

Always use linker64: `/system/bin/linker64 libtwoyi.so`

### "library not found" errors

Make sure LD_LIBRARY_PATH is set if the library has dependencies:
```bash
LD_LIBRARY_PATH=/data/local/tmp /system/bin/linker64 /data/local/tmp/libtwoyi.so
```

## Success Criteria

✅ The library can be invoked via linker64 without segmentation fault
✅ Help message is displayed correctly
✅ Arguments are parsed correctly (--loader, --width, --height, etc.)
✅ JNI mode (app) still works without any issues
