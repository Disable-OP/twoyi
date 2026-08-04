#!/bin/bash

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# =============================================================================
# build.sh — build libOpenglRender.so (open-source replacement for the
# legacy closed-source libOpenglRender.so) for one or more Android ABIs.
#
# Usage:
#   ./build.sh [abi ...]
#
# Examples:
#   ./build.sh                       # default: arm64-v8a only
#   ./build.sh arm64-v8a x86_64
#   ./build.sh all
#
# Supported ABIs:  arm64-v8a, x86_64
# =============================================================================

set -e

ALL_ABIS=("arm64-v8a" "x86_64")

# Parse args
ABIS=()
for arg in "$@"; do
    case "$arg" in
        all) ABIS=("${ALL_ABIS[@]}") ;;
        arm64-v8a|x86_64) ABIS+=("$arg") ;;
        *) echo "Unknown arg: $arg"; exit 1 ;;
    esac
done
if [ ${#ABIS[@]} -eq 0 ]; then
    ABIS=("arm64-v8a")
fi

echo "=========================================="
echo "Building libOpenglRender.so for ABIs=[${ABIS[*]}]"
echo "=========================================="

cd "$(dirname "$0")"

for ABI in "${ABIS[@]}"; do
    echo ""
    echo "------------------------------------------"
    echo "  Building for $ABI"
    echo "------------------------------------------"

    cargo xdk -t "$ABI" build --release

    case "$ABI" in
        arm64-v8a) RUST_TARGET="aarch64-linux-android" ;;
        x86_64)    RUST_TARGET="x86_64-linux-android"  ;;
    esac

    SRC="target/$RUST_TARGET/release/libOpenglRender.so"
    DST_DIR="../../src/main/jniLibs/$ABI"
    DST="$DST_DIR/libOpenglRender_new.so"

    mkdir -p "$DST_DIR"
    cp -v "$SRC" "$DST"

    echo "  → $DST"

    if command -v nm >/dev/null 2>&1; then
        echo ""
        echo "Exported symbols ($ABI):"
        nm -D "$DST" | grep -E "startOpenGLRenderer|destroyOpenGLSubwindow|repaintOpenGLDisplay|setNativeWindow|resetSubWindow|removeSubWindow" || true
    fi

    echo ""
    echo "Library size comparison ($ABI):"
    echo "  Legacy: $(ls -lh "$DST_DIR/libOpenglRender.so" 2>/dev/null | awk '{print $5}' || echo 'N/A')"
    echo "  New:    $(ls -lh "$DST" 2>/dev/null | awk '{print $5}' || echo 'N/A')"
done

echo ""
echo "=========================================="
echo "Build complete!"
echo "=========================================="
