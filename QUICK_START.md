# Quick Start: Testing the Fix

## What Was Fixed

The segmentation fault when invoking `libtwoyi.so` via `linker64` has been fixed by setting the ELF entry point to the `main` function.

## How to Test (Local Build Verification)

```bash
# 1. Build the library (if not already built)
cd app/rs
sh build_rs.sh --release
cd ../..

# 2. Run the test script
./test_libtwoyi.sh
```

Expected output:
```
✓ Library exists
✓ Entry point is set (0x3a3a8)
✓ Entry point matches 'main' function address
✓ JNI_OnLoad symbol found
✓ All twoyi_* functions exported
All tests passed! ✓
```

## How to Test (On Android Device)

```bash
# 1. Build the library (if not already done)
cd app/rs && sh build_rs.sh --release && cd ../..

# 2. Push files to device
adb push app/src/main/jniLibs/arm64-v8a/libtwoyi.so /data/local/tmp/
adb push app/src/main/jniLibs/arm64-v8a/libloader.so /data/local/tmp/

# 3. Test the exact command from the problem statement
adb shell "cd /data/local/tmp && LD_LIBRARY_PATH=. /system/bin/linker64 ./libtwoyi.so --loader ./libloader.so --help"
```

**Expected:** Help message should display without segmentation fault.

## What Changed

### Files Modified
- `app/rs/.cargo/config.toml` - New: Rust linker configuration
- `app/rs/build_rs.sh` - Modified: Sets RUSTFLAGS to configure entry point
- `app/rs/build.rs` - Modified: Updated comments
- `SHELL_EXECUTION.md` - Modified: Updated documentation

### Files Added
- `test_libtwoyi.sh` - Test script to validate library structure
- `TESTING_DIRECT_INVOCATION.md` - Complete testing guide
- `JNI_VERIFICATION.md` - JNI compatibility explanation
- `FIX_SUMMARY.md` - Detailed fix summary
- `QUICK_START.md` - This file

## Next Steps

1. **Verify Local Build**: Run `./test_libtwoyi.sh` to ensure the library was built correctly
2. **Test on Device**: Follow the Android device testing steps above
3. **Verify JNI Mode**: Build and test the APK to ensure app functionality is not affected

## For More Information

- See `FIX_SUMMARY.md` for complete technical details
- See `TESTING_DIRECT_INVOCATION.md` for comprehensive testing instructions
- See `JNI_VERIFICATION.md` to understand JNI compatibility
