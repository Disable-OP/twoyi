# JNI Mode Verification

## Why Setting Entry Point Doesn't Break JNI

When an Android app loads a native library using `System.loadLibrary("twoyi")`, the following happens:

1. The Android runtime uses `dlopen()` to load the shared library
2. `dlopen()` does NOT use the ELF entry point - it only loads the library into memory
3. After loading, the runtime looks for and calls the `JNI_OnLoad` function if it exists
4. The entry point is only used when the library is executed directly (e.g., via linker64)

Therefore, setting the entry point to `main` via `-Wl,-e,main` does not affect JNI mode at all.

## Verification

The library exports both entry points:
- **Entry point address**: 0x3a3a8 (points to `main`) - used by linker64
- **JNI_OnLoad symbol**: 0x38c84 - used by System.loadLibrary()

Both mechanisms work independently:
- JNI mode: `System.loadLibrary("twoyi")` → `dlopen()` → `JNI_OnLoad()`
- Shell mode: `/system/bin/linker64 libtwoyi.so` → entry point → `main()`

## Testing

To fully verify JNI mode works:
1. Build the APK: `./gradlew assembleRelease`
2. Install on device: `adb install app/build/outputs/apk/release/*.apk`
3. Launch the app and verify all features work normally
4. The library should load without errors and all native methods should work

Since we don't have a device in this environment, we've verified:
- ✓ Entry point is set correctly
- ✓ JNI_OnLoad symbol is exported
- ✓ All expected symbols are present
- ✓ ELF structure is valid
