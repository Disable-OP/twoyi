#!/bin/bash
# Build libtwoyi.so for both arm64-v8a and x86_64 ABIs.
#
# This script is designed to be run in the GitHub Codespace which has
# the Android NDK and Rust toolchain installed.
#
# Usage:
#   ./build_libtwoyi.sh [--debug]
#
# Output:
#   app/src/main/jniLibs/arm64-v8a/libtwoyi.so
#   app/src/main/jniLibs/x86_64/libtwoyi.so

set -e

cd "$(dirname "$0")/.."

RELEASE_FLAG="--release"
if [[ "$1" == "--debug" ]]; then
    RELEASE_FLAG=""
fi

echo "=========================================="
echo "Building libtwoyi.so for both ABIs"
echo "=========================================="

# Check NDK
if [[ -z "$ANDROID_NDK_HOME" && -z "$NDK_HOME" ]]; then
    echo "ERROR: ANDROID_NDK_HOME not set"
    echo "Install NDK: sdkmanager 'ndk;25.2.9519653'"
    exit 1
fi
NDK="${ANDROID_NDK_HOME:-$NDK_HOME}"
echo "Using NDK: $NDK"

# Check Rust
if ! command -v cargo &>/dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Add Android targets
echo "Adding Android targets to Rust..."
rustup target add aarch64-linux-android || true
rustup target add x86_64-linux-android || true

# Configure environment for arm64
echo ""
echo "=========================================="
echo "Building for arm64-v8a (aarch64-linux-android)"
echo "=========================================="
export TOOLCHAIN_ARM="$NDK/toolchains/llvm/prebuilt/linux-x86_64"
export CC_aarch64_linux_android="$TOOLCHAIN_ARM/bin/aarch64-linux-android24-clang"
export CXX_aarch64_linux_android="$TOOLCHAIN_ARM/bin/aarch64-linux-android24-clang++"
export AR_aarch64_linux_android="$TOOLCHAIN_ARM/bin/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"

cd app/rs
cargo build $RELEASE_FLAG --target aarch64-linux-android

# Copy to jniLibs
SRC_PATH="target/aarch64-linux-android/${RELEASE_FLAG:+release}/libtwoyi.so"
if [[ -z "$RELEASE_FLAG" ]]; then
    SRC_PATH="target/aarch64-linux-android/debug/libtwoyi.so"
fi
cp "$SRC_PATH" ../src/main/jniLibs/arm64-v8a/libtwoyi.so
echo "Copied to app/src/main/jniLibs/arm64-v8a/libtwoyi.so"
ls -la ../src/main/jniLibs/arm64-v8a/libtwoyi.so

# Configure environment for x86_64
echo ""
echo "=========================================="
echo "Building for x86_64 (x86_64-linux-android)"
echo "=========================================="
export CC_x86_64_linux_android="$TOOLCHAIN_ARM/bin/x86_64-linux-android24-clang"
export CXX_x86_64_linux_android="$TOOLCHAIN_ARM/bin/x86_64-linux-android24-clang++"
export AR_x86_64_linux_android="$TOOLCHAIN_ARM/bin/llvm-ar"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$CC_x86_64_linux_android"

cargo build $RELEASE_FLAG --target x86_64-linux-android

SRC_PATH="target/x86_64-linux-android/${RELEASE_FLAG:+release}/libtwoyi.so"
if [[ -z "$RELEASE_FLAG" ]]; then
    SRC_PATH="target/x86_64-linux-android/debug/libtwoyi.so"
fi
cp "$SRC_PATH" ../src/main/jniLibs/x86_64/libtwoyi.so
echo "Copied to app/src/main/jniLibs/x86_64/libtwoyi.so"
ls -la ../src/main/jniLibs/x86_64/libtwoyi.so

echo ""
echo "=========================================="
echo "BUILD COMPLETE"
echo "=========================================="
echo "Both ABIs built successfully."
