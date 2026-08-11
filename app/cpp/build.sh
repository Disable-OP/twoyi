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

echo '=========================================='
echo 'Build complete!'
echo '=========================================='
