#!/bin/bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# build.sh — build libOpenglRender.so from the vendored AOSP emugl
# source (task AOSP-VENDOR-1) for one or more Android ABIs.
#
# This script lives at app/cpp/build.sh and drives the CMake build in
# app/cpp/emugl/. It is invoked by the Gradle `cmakeBuild` task
# (see app/build.gradle).
#
# Usage:
#   ./build.sh                       # default: arm64-v8a only
#   ./build.sh arm64-v8a x86_64
#   ./build.sh all
#
# Output is written to ../../src/main/jniLibs/<abi>/libOpenglRender.so
# (replacing the legacy closed-source blob).

set -e

ALL_ABIS="arm64-v8a x86_64"

# Parse args
ABIS=""
for arg in "$@"; do
    case "$arg" in
        all) ABIS="$ALL_ABIS" ;;
        arm64-v8a|x86_64)
            if [ -z "$ABIS" ]; then
                ABIS="$arg"
            else
                ABIS="$ABIS $arg"
            fi
            ;;
        *) echo "Unknown arg: $arg"; exit 1 ;;
    esac
done
if [ -z "$ABIS" ]; then
    ABIS="arm64-v8a"
fi

cd "$(dirname "$0")/emugl"

# Locate the NDK. Prefer $ANDROID_NDK_HOME / $ANDROID_NDK_ROOT, then
# fall back to the codespace-default install.
NDK="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-/workspaces/twoyi/.android-ndk}}"
if [ ! -d "$NDK" ]; then
    echo "ERROR: Android NDK not found."
    echo "       Set ANDROID_NDK_HOME or install at /workspaces/twoyi/.android-ndk"
    exit 1
fi
TOOLCHAIN="$NDK/build/cmake/android.toolchain.cmake"
STRIP="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip"

echo "=========================================="
echo "Building libOpenglRender.so (AOSP source) for ABIs=[$ABIS]"
echo "  NDK      : $NDK"
echo "=========================================="

for ABI in $ABIS; do
    echo ""
    echo "------------------------------------------"
    echo "  Building for $ABI"
    echo "------------------------------------------"

    BUILD_DIR="build-$ABI"
    mkdir -p "$BUILD_DIR"

    cmake -G "Unix Makefiles" \
        -DCMAKE_TOOLCHAIN_FILE="$TOOLCHAIN" \
        -DANDROID_ABI="$ABI" \
        -DANDROID_PLATFORM=android-24 \
        -DANDROID_STL=c++_static \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_STRIP="$STRIP" \
        -B "$BUILD_DIR" \
        -S .

    cmake --build "$BUILD_DIR" -- -j"$(nproc)"

    DST_DIR="../../src/main/jniLibs/$ABI"
    DST="$DST_DIR/libOpenglRender.so"
    mkdir -p "$DST_DIR"
    cp -v "$BUILD_DIR/libOpenglRender.so" "$DST"

    echo "  -> $DST"

    # Symbol verification — all 6 twoyi-required C-ABI entry points
    # must be present in the dynamic symbol table.
    if command -v nm >/dev/null 2>&1; then
        echo ""
        echo "Exported symbols ($ABI):"
        nm -D --defined-only "$DST" \
            | grep -E "startOpenGLRenderer|destroyOpenGLSubwindow|repaintOpenGLDisplay|setNativeWindow|resetSubWindow|removeSubWindow" \
            || echo "  WARNING: missing symbols!"
    fi
done

echo ""
echo "=========================================="
echo "Build complete!"
echo "=========================================="
