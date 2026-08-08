#!/bin/sh
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# build.sh — build libkr64.so (the kernel-replacement daemon) for one
# or more Android ABIs.
#
# IMPORTANT (2026-08-09 overnight debug): We previously built the cdylib
# target (libkr64.so) and used the PIE hack to make it directly
# executable. But the cdylib's entry point (kr64_main) was called WITHOUT
# the C runtime initialization that _start (from crt1.o) normally does —
# no TLS setup, no stdio init, no __libc_start_main. The very first
# eprintln! call crashed with SIGSEGV at rip=0x7 (NULL function pointer
# via uninitialized TLS).
#
# The fix: build the BIN target (`kr64`) instead. The bin target has a
# proper _start from crt1.o that initializes the C runtime before calling
# main(). We then copy the bin as `libkr64.so` (the lib*.so naming is
# required so that Android's PackageManager extracts it from the APK into
# the app's nativeLibraryDir). The resulting file is a regular PIE
# executable, just named with the .so convention.
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
echo "Building libkr64.so (bin target) for ABIs=[$ABIS]"
echo "=========================================="

cd "$(dirname "$0")"

for ABI in $ABIS; do
    echo ""
    echo "------------------------------------------"
    echo "  Building kr64 (bin) for $ABI"
    echo "------------------------------------------"

    # cargo-xdk takes the Android ABI name (arm64-v8a / x86_64); it maps
    # internally to the right Rust target triple. We build the BIN target
    # (--bin kr64) which produces a regular PIE executable with a proper
    # _start from crt1.o — NOT the cdylib with the PIE hack.
    cargo xdk -t "$ABI" build --release --bin kr64

    # cargo-xdk's target dir uses the Rust triple, not the Android ABI name
    case "$ABI" in
        arm64-v8a) RUST_TARGET="aarch64-linux-android" ;;
        x86_64)    RUST_TARGET="x86_64-linux-android"  ;;
    esac

    # The bin target produces a file named `kr64` (no lib prefix, no .so suffix).
    SRC="target/$RUST_TARGET/release/kr64"
    DST_DIR="../../src/main/jniLibs/$ABI"
    DST="$DST_DIR/libkr64.so"

    if [ ! -f "$SRC" ]; then
        echo "  ✗ build failed: $SRC not found"
        echo "    (did cargo xdk produce the bin target?)"
        exit 1
    fi

    mkdir -p "$DST_DIR"
    cp -v "$SRC" "$DST"
    chmod +x "$DST"

    echo "  -> $DST"

    if command -v readelf >/dev/null 2>&1; then
        echo ""
        echo "Entry point / ELF header check ($ABI):"
        readelf -h "$DST" | grep -E "Entry point|Type:" || true
        # Verify it's an EXEC-type (PIE) binary, not a shared lib (DYN-only).
        # A PIE executable shows "Type: DYN (Shared object file)" — same as a
        # shared lib, but it has an entry point set to a real function (not 0).
        # The entry point should be non-zero for a proper executable.
        echo ""
        echo "PT_INTERP check (should show /system/bin/linker64):"
        readelf -l "$DST" 2>/dev/null | grep -A1 INTERP || true
    fi
done

echo ""
echo "=========================================="
echo "libkr64.so build complete for all ABIs."
echo "(bin target, packaged as libkr64.so for APK extraction)"
echo "=========================================="
