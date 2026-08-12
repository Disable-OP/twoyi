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

# Build twrp_fb_hook.so — LD_PRELOAD library for TWRP framebuffer
# virtualization. Built for BOTH i686 (32-bit x86, for x86 TWRP images
# like the bundled byt_t_crv2.img) AND aarch64 (for arm64 TWRP images
# that real arm64 devices use).
#
# This is a SEPARATE build from the main loader because:
#   - TWRP's recovery binary may be 32-bit (i386) or 64-bit (aarch64),
#     and the LD_PRELOAD library MUST match the recovery binary's arch
#     (bionic linker refuses to load cross-arch .so files).
#   - We only need FB ioctl interception for TWRP (no seccomp, no path
#     translation, no exec hooks), so a minimal hook file is cleaner than
#     making the main loader build for every TWRP arch.
#
# For i686: NDK r27c dropped i686 sysroot libs, so we build with -nostdlib.
# The resulting .so leaves libc/libdl symbols unresolved; bionic resolves
# them at load time from the recovery binary's own libc/libdl.
#
# For aarch64: we can use the full sysroot (NDK r27c has aarch64 libs),
# so we link normally with -lc.
#
# CRITICAL for i686: We MUST pass -Wl,--hash-style=sysv. By default
# clang/ld.lld emit only DT_GNU_HASH (no DT_HASH). TWRP's old bionic
# linker (AOSP 5.1, Android L) does NOT understand DT_GNU_HASH — it
# errors out "CANNOT LINK EXECUTABLE: empty/missing DT_HASH in
# twrp_fb_hook.so". --hash-style=sysv makes ld emit DT_HASH (the classic
# SysV hash table) and omit DT_GNU_HASH, satisfying old bionic.
# (aarch64 TWRP images use modern bionic that understands DT_GNU_HASH,
#  so we don't need this flag for aarch64 — but we pass it anyway for
#  consistency; it's harmless on modern linkers.)

# i686 build → jniLibs/x86_64/twrp_fb_hook.so
echo '=========================================='
echo 'Building twrp_fb_hook.so (i686, for x86 TWRP images)'
echo '=========================================='
TWRP_HOOK_BUILD_DIR_I686=$SCRIPT_DIR/build/twrp_fb_hook/i686
rm -rf "$TWRP_HOOK_BUILD_DIR_I686"
mkdir -p "$TWRP_HOOK_BUILD_DIR_I686"
TWRP_HOOK_JNILIBS_DIR_X86=$SCRIPT_DIR/../src/main/jniLibs/x86_64
mkdir -p "$TWRP_HOOK_JNILIBS_DIR_X86"
$NDK_BUILD_DIR/toolchains/llvm/prebuilt/linux-x86_64/bin/clang \
    -target i686-linux-android24 \
    --sysroot=$NDK_BUILD_DIR/toolchains/llvm/prebuilt/linux-x86_64/sysroot \
    -nostdlib -shared -fPIC -O2 -g \
    -fno-builtin -fno-builtin-memset -fno-builtin-strcmp -fno-builtin-strlen \
    -Wl,--hash-style=sysv \
    -Wl,--exclude-libs,ALL \
    -D_GNU_SOURCE \
    -I$SCRIPT_DIR/twoyi_loader/include -I$SCRIPT_DIR/twoyi_loader/src \
    -o $TWRP_HOOK_BUILD_DIR_I686/twrp_fb_hook.so \
    $SCRIPT_DIR/twoyi_loader/src/twrp_fb_hook.c 2>&1 \
    || { echo "  ✗ twrp_fb_hook (i686) build failed" >&2; exit 1; }
if [ -f "$TWRP_HOOK_BUILD_DIR_I686/twrp_fb_hook.so" ]; then
    cp -v $TWRP_HOOK_BUILD_DIR_I686/twrp_fb_hook.so $TWRP_HOOK_JNILIBS_DIR_X86/twrp_fb_hook.so
    echo "  ✓ twrp_fb_hook.so (i686): $(file -b $TWRP_HOOK_BUILD_DIR_I686/twrp_fb_hook.so)"
else
    echo "  ✗ twrp_fb_hook.so (i686) not produced" >&2
    exit 1
fi

# aarch64 build → jniLibs/arm64-v8a/twrp_fb_hook.so
echo '=========================================='
echo 'Building twrp_fb_hook.so (aarch64, for arm64 TWRP images)'
echo '=========================================='
TWRP_HOOK_BUILD_DIR_ARM64=$SCRIPT_DIR/build/twrp_fb_hook/aarch64
rm -rf "$TWRP_HOOK_BUILD_DIR_ARM64"
mkdir -p "$TWRP_HOOK_BUILD_DIR_ARM64"
TWRP_HOOK_JNILIBS_DIR_ARM64=$SCRIPT_DIR/../src/main/jniLibs/arm64-v8a
mkdir -p "$TWRP_HOOK_JNILIBS_DIR_ARM64"
$NDK_BUILD_DIR/toolchains/llvm/prebuilt/linux-x86_64/bin/clang \
    -target aarch64-linux-android24 \
    --sysroot=$NDK_BUILD_DIR/toolchains/llvm/prebuilt/linux-x86_64/sysroot \
    -nostdlib -shared -fPIC -O2 -g \
    -fno-builtin -fno-builtin-memset -fno-builtin-strcmp -fno-builtin-strlen \
    -Wl,--hash-style=sysv \
    -Wl,--exclude-libs,ALL \
    -D_GNU_SOURCE \
    -I$SCRIPT_DIR/twoyi_loader/include -I$SCRIPT_DIR/twoyi_loader/src \
    -o $TWRP_HOOK_BUILD_DIR_ARM64/twrp_fb_hook.so \
    $SCRIPT_DIR/twoyi_loader/src/twrp_fb_hook.c 2>&1 \
    || { echo "  ✗ twrp_fb_hook (aarch64) build failed" >&2; exit 1; }
if [ -f "$TWRP_HOOK_BUILD_DIR_ARM64/twrp_fb_hook.so" ]; then
    cp -v $TWRP_HOOK_BUILD_DIR_ARM64/twrp_fb_hook.so $TWRP_HOOK_JNILIBS_DIR_ARM64/twrp_fb_hook.so
    echo "  ✓ twrp_fb_hook.so (aarch64): $(file -b $TWRP_HOOK_BUILD_DIR_ARM64/twrp_fb_hook.so)"
else
    echo "  ✗ twrp_fb_hook.so (aarch64) not produced" >&2
    exit 1
fi

echo '=========================================='
echo 'Build complete!'
echo '=========================================='
