#!/bin/sh
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# build.sh — build libloader.so (open-source replacement for the legacy
# closed-source libloader.so) for one or more Android ABIs.
#
# After task AOSP-VENDOR-1 the legacy libloader.so blob has been
# removed and the output of this build is shipped *as* libloader.so
# (no more libloader_new.so).
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
echo "Building libloader.so for ABIs=[$ABIS]"
echo "=========================================="

cd "$(dirname "$0")"

for ABI in $ABIS; do
    echo ""
    echo "------------------------------------------"
    echo "  Building for $ABI"
    echo "------------------------------------------"

    cargo xdk -t "$ABI" build --release

    # cargo-xdk's target dir uses the Rust triple, not the Android ABI name
    case "$ABI" in
        arm64-v8a) RUST_TARGET="aarch64-linux-android" ;;
        x86_64)    RUST_TARGET="x86_64-linux-android"  ;;
    esac

    SRC="target/$RUST_TARGET/release/libloader.so"
    DST_DIR="../../src/main/jniLibs/$ABI"
    DST="$DST_DIR/libloader.so"

    mkdir -p "$DST_DIR"
    cp -v "$SRC" "$DST"
    chmod +x "$DST"

    echo "  -> $DST"

    if command -v readelf >/dev/null 2>&1; then
        echo ""
        echo "Entry point check ($ABI):"
        readelf -h "$DST" | grep "Entry point" || true
        echo "Dynamic interpreter ($ABI):"
        readelf -l "$DST" | grep interpreter || echo "  (no interpreter found)"
    fi
done

echo ""
echo "=========================================="
echo "Build complete!"
echo "=========================================="
