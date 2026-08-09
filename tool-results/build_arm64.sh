#!/bin/bash
# Build libOpenglRender.so for arm64-v8a using NDK r27c
set -e

NDK=/workspaces/twoyi/.android-ndk
TOOLCHAIN=$NDK/build/cmake/android.toolchain.cmake
BUILD=/tmp/build_opengl

mkdir -p $BUILD/build-arm64
cd $BUILD/build-arm64

cmake -G "Unix Makefiles" \
    -DCMAKE_TOOLCHAIN_FILE=$TOOLCHAIN \
    -DANDROID_ABI=arm64-v8a \
    -DANDROID_PLATFORM=android-24 \
    -DANDROID_STL=c++_static \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_STRIP=$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip \
    $BUILD 2>&1

echo "=== CONFIGURE DONE, BUILDING ==="

make -j$(nproc) OpenglRender 2>&1

echo "=== BUILD DONE ==="
ls -la $BUILD/build-arm64/libOpenglRender.so
