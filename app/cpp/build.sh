#!/bin/sh
# Build libOpenglRender.so from AOSP source for arm64-v8a and x86_64
set -e

SCRIPT_DIR=$(cd $(dirname $0) && pwd)
NDK_BUILD_DIR=${ANDROID_NDK_HOME:-$ANDROID_NDK}

if [ -z "$NDK_BUILD_DIR" ]; then
    echo 'ERROR: ANDROID_NDK_HOME not set'
    exit 1
fi

for ABI in arm64-v8a x86_64; do
    echo '=========================================='
    echo "Building libOpenglRender.so for $ABI"
    echo '=========================================='

    BUILD_DIR=$SCRIPT_DIR/build/$ABI
    # Wipe any stale CMake cache before reconfiguring. CMake embeds the
    # absolute source path in CMakeCache.txt and refuses to proceed if
    # the current source path doesn't match — which breaks builds when
    # the source tree is moved (e.g. codespace -> CI runner, or just a
    # fresh clone at a different path). Wiping here guarantees a clean
    # reconfigure every run; the cost is ~5s of reconfigure time, which
    # is negligible compared to the actual compile time.
    rm -rf "$BUILD_DIR"
    mkdir -p "$BUILD_DIR"

    cmake -S $SCRIPT_DIR/emugl -B $BUILD_DIR \
        -DCMAKE_TOOLCHAIN_FILE=$NDK_BUILD_DIR/build/cmake/android.toolchain.cmake \
        -DANDROID_ABI=$ABI \
        -DANDROID_PLATFORM=android-24 \
        -DCMAKE_BUILD_TYPE=Release

    cmake --build $BUILD_DIR -j$(nproc)

    # Copy to jniLibs (use SCRIPT_DIR-relative path so the script works
    # regardless of the caller's CWD).
    JNILIBS_DIR=$SCRIPT_DIR/../src/main/jniLibs/$ABI
    mkdir -p $JNILIBS_DIR
    cp -v $BUILD_DIR/libOpenglRender.so $JNILIBS_DIR/libOpenglRender.so

    # Build getpid_hook.so (LD_PRELOAD library that makes init think
    # it's PID 1 by hooking getpid() to return 1)
    HOOK_BUILD_DIR=$SCRIPT_DIR/build/getpid_hook/$ABI
    rm -rf "$HOOK_BUILD_DIR"
    mkdir -p "$HOOK_BUILD_DIR"
    cmake -S $SCRIPT_DIR/getpid_hook -B $HOOK_BUILD_DIR \
        -DCMAKE_TOOLCHAIN_FILE=$NDK_BUILD_DIR/build/cmake/android.toolchain.cmake \
        -DANDROID_ABI=$ABI \
        -DANDROID_PLATFORM=android-24 \
        -DCMAKE_BUILD_TYPE=Release
    cmake --build $HOOK_BUILD_DIR -j$(nproc)
    cp -v $HOOK_BUILD_DIR/libgetpid_hook.so $JNILIBS_DIR/libgetpid_hook.so

    # Build libtwoyi_loader_shlib.so (the seccomp/SIGSYS virtualization library)
    # This is loaded via LD_PRELOAD to install seccomp + SIGSYS handler
    # before the guest init's main() runs.
    LOADER_BUILD_DIR=$SCRIPT_DIR/build/twoyi_loader/$ABI
    rm -rf "$LOADER_BUILD_DIR"
    mkdir -p "$LOADER_BUILD_DIR"
    # Compile directly (not CMake — it's a single file)
    $NDK_BUILD_DIR/toolchains/llvm/prebuilt/linux-x86_64/bin/clang \
        -target $ABI-linux-android24 \
        --sysroot=$NDK_BUILD_DIR/toolchains/llvm/prebuilt/linux-x86_64/sysroot \
        -shared -fPIC -O2 -g \
        -D_GNU_SOURCE \
        -o $LOADER_BUILD_DIR/libtwoyi_loader_shlib.so \
        $SCRIPT_DIR/twoyi_loader/src/twoyi_loader_shlib.c \
        -lc -ldl 2>&1 || { echo "  ✗ twoyi_loader_shlib build failed for $ABI" >&2; exit 1; }
    if [ -f "$LOADER_BUILD_DIR/libtwoyi_loader_shlib.so" ]; then
        cp -v $LOADER_BUILD_DIR/libtwoyi_loader_shlib.so $JNILIBS_DIR/libtwoyi_loader_shlib.so
    fi
done

# Build twrp_fb_hook.so — a MINIMAL i686 (32-bit x86) LD_PRELOAD library
# for TWRP framebuffer virtualization. This is a SEPARATE build from the
# main loader because:
#   - TWRP's recovery binary is i386 (32-bit), so its bionic linker can't
#     load the 64-bit libtwoyi_loader_shlib.so.
#   - We only need FB ioctl interception for TWRP (no seccomp, no path
#     translation, no exec hooks), so a minimal hook file is cleaner than
#     making the main loader build for i686 (which would require adding
#     i686 syscall number defines throughout the file).
#
# NDK r27c dropped i686 sysroot libs, so we build with -nostdlib. The
# resulting .so leaves libc/libdl symbols (syscall, dlsym, strcmp) unresol-
# ved; bionic resolves them at load time from the recovery binary's own
# libc/libdl. This produces a valid 32-bit ELF that bionic can load.
#
# CRITICAL: We MUST pass -Wl,--hash-style=sysv. By default clang/ld.lld
# emit only DT_GNU_HASH (no DT_HASH). TWRP's bionic linker (AOSP 5.1,
# Android L) does NOT understand DT_GNU_HASH — it warns
# "unused DT entry: type 0x6ffffef5" (DT_GNU_HASH) and then errors out
# "CANNOT LINK EXECUTABLE: empty/missing DT_HASH in twrp_fb_hook.so",
# causing /sbin/recovery to exit(1) before main() runs (strace-confirmed
# in KVM run 31562902048). --hash-style=sysv makes ld emit DT_HASH (the
# classic SysV hash table) and omit DT_GNU_HASH, satisfying old bionic.
#
# The .so is placed in jniLibs/x86_64/ (despite being i686) so the APK
# packaging includes it. Android's PackageManager extracts whatever is in
# lib/<abi>/ without validating the ELF architecture, and the KVM test
# script extracts it via `unzip` (not via PackageManager) and pushes it
# to the device's rootfs where kr64 reads it. The library is NEVER loaded
# by the Android app itself — it's just a file in the APK that the KVM
# test pulls out and pushes to the guest.
echo '=========================================='
echo 'Building twrp_fb_hook.so (i686, TWRP framebuffer virtualization)'
echo '=========================================='
TWRP_HOOK_BUILD_DIR=$SCRIPT_DIR/build/twrp_fb_hook
rm -rf "$TWRP_HOOK_BUILD_DIR"
mkdir -p "$TWRP_HOOK_BUILD_DIR"
TWRP_HOOK_JNILIBS_DIR=$SCRIPT_DIR/../src/main/jniLibs/x86_64
mkdir -p "$TWRP_HOOK_JNILIBS_DIR"
$NDK_BUILD_DIR/toolchains/llvm/prebuilt/linux-x86_64/bin/clang \
    -target i686-linux-android24 \
    --sysroot=$NDK_BUILD_DIR/toolchains/llvm/prebuilt/linux-x86_64/sysroot \
    -nostdlib -shared -fPIC -O2 -g \
    -Wl,--hash-style=sysv \
    -D_GNU_SOURCE \
    -I$SCRIPT_DIR/twoyi_loader/include -I$SCRIPT_DIR/twoyi_loader/src \
    -o $TWRP_HOOK_BUILD_DIR/twrp_fb_hook.so \
    $SCRIPT_DIR/twoyi_loader/src/twrp_fb_hook.c 2>&1 \
    || { echo "  ✗ twrp_fb_hook build failed" >&2; exit 1; }
if [ -f "$TWRP_HOOK_BUILD_DIR/twrp_fb_hook.so" ]; then
    cp -v $TWRP_HOOK_BUILD_DIR/twrp_fb_hook.so $TWRP_HOOK_JNILIBS_DIR/twrp_fb_hook.so
    echo "  ✓ twrp_fb_hook.so: $(file -b $TWRP_HOOK_BUILD_DIR/twrp_fb_hook.so)"
else
    echo "  ✗ twrp_fb_hook.so not produced" >&2
    exit 1
fi

echo '=========================================='
echo 'Build complete!'
echo '=========================================='
