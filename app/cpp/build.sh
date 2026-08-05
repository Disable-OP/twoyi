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
    mkdir -p $BUILD_DIR

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
done

echo '=========================================='
echo 'Build complete!'
echo '=========================================='
