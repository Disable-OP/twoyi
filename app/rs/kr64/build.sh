#!/bin/sh
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# build.sh — build libkr64.so (the kernel-replacement daemon) for one
# or more Android ABIs.
#
# libkr64.so is a cdylib that uses the same PIE hack as libtwoyi.so
# (see interp.c + build.rs): it's both dlopen-able AND directly
# executable. The Java side (RomManager) symlinks it into the guest
# rootfs so the guest's init can exec it.
#
# Usage:
#   ./build.sh                       # default: arm64-v8a only
#   ./build.sh arm64-v8a x86_64
#   ./build.sh all
#
# POSIX-sh compatible (no bash arrays) so it works under `sh build.sh`.
# Supported ABIs:  arm64-v8a, x86_64

set -e

# Ensure cargo / cargo-xdk are on PATH (Gradle exec does not source ~/.bashrc)
export PATH="$HOME/.cargo/bin:$PATH"

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

echo "=========================================="
echo "Building libkr64.so for ABIs=[$ABIS]"
echo "=========================================="

cd "$(dirname "$0")"

for ABI in $ABIS; do
    echo ""
    echo "------------------------------------------"
    echo "  Building libkr64.so for $ABI"
    echo "------------------------------------------"

    # cargo-xdk takes the Android ABI name (arm64-v8a / x86_64); it maps
    # internally to the right Rust target triple. The kr64 crate's
    # build.rs emits the PIE linker flags that make the cdylib directly
    # executable (same trick as libtwoyi.so).
    cargo xdk -t "$ABI" build --release

    # cargo-xdk's target dir uses the Rust triple, not the Android ABI name
    case "$ABI" in
        arm64-v8a) RUST_TARGET="aarch64-linux-android" ;;
        x86_64)    RUST_TARGET="x86_64-linux-android"  ;;
    esac

    SRC="target/$RUST_TARGET/release/libkr64.so"
    DST_DIR="../../src/main/jniLibs/$ABI"
    DST="$DST_DIR/libkr64.so"

    if [ ! -f "$SRC" ]; then
        echo "  ✗ build failed: $SRC not found"
        echo "    (did cargo xdk produce the cdylib?)"
        exit 1
    fi

    mkdir -p "$DST_DIR"
    cp -v "$SRC" "$DST"
    chmod +x "$DST"

    echo "  -> $DST"

    if command -v readelf >/dev/null 2>&1; then
        echo ""
        echo "Entry point check ($ABI):"
        readelf -h "$DST" | grep "Entry point" || true
    fi
done

echo ""
echo "=========================================="
echo "libkr64.so build complete for all ABIs."
echo "=========================================="
