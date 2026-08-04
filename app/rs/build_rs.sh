#! /bin/bash

# Exit on error
set -e

#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#

# =============================================================================
# build_rs.sh — build libtwoyi.so for one or more Android ABIs.
#
# Usage:
#   ./build_rs.sh [--release] [abi ...]
#
# Examples:
#   ./build_rs.sh --release                 # default: arm64-v8a only
#   ./build_rs.sh --release arm64-v8a x86_64
#   ./build_rs.sh arm64-v8a                 # debug build, arm64 only
#
# If no abi is specified, defaults to arm64-v8a only (the historical
# behaviour). Pass "all" to build every supported ABI.
#
# Supported ABIs:  arm64-v8a, x86_64
# =============================================================================

# Default ABIs if none are specified on the command line
DEFAULT_ABIS=("arm64-v8a")
ALL_ABIS=("arm64-v8a" "x86_64")

# Map Android ABI → Rust target triple + dynamic linker path
abi_to_target() {
    case "$1" in
        arm64-v8a) echo "aarch64-linux-android" ;;
        x86_64)    echo "x86_64-linux-android"  ;;
        *) echo "UNKNOWN_TARGET_FOR_$1"; return 1 ;;
    esac
}

abi_to_linker() {
    case "$1" in
        arm64-v8a) echo "/system/bin/linker64"   ;;
        x86_64)    echo "/system/bin/linker64"   ;;
        *) echo "/system/bin/linker64" ;;
    esac
}

# Parse args: separate --release / --debug flags from ABI names
PROFILE_ARG=""
ABIS=()
for arg in "$@"; do
    case "$arg" in
        --release)  PROFILE_ARG="--release"; ABIS=("${DEFAULT_ABIS[@]}") ;;
        --debug)    PROFILE_ARG="" ;;
        all)        ABIS=("${ALL_ABIS[@]}") ;;
        arm64-v8a|x86_64) ABIS+=("$arg") ;;
        *)
            # Pass through unknown args (e.g. --features=foo) to cargo.
            # If the user previously set ABIS=() and we see an unknown flag,
            # fall back to the default ABI list so legacy callers still work.
            if [ ${#ABIS[@]} -eq 0 ]; then
                ABIS=("${DEFAULT_ABIS[@]}")
            fi
            ;;
    esac
done

# If only --release was given with no ABIs, use defaults
if [ ${#ABIS[@]} -eq 0 ]; then
    ABIS=("${DEFAULT_ABIS[@]}")
fi

echo "build_rs.sh: building ABIs=[${ABIS[*]}] profile=${PROFILE_ARG:-debug}"

# Configure as PIE executable for direct execution with JNI compatibility.
# PIE with INTERP segment allows direct execution: ./libtwoyi.so
# These flags work for both aarch64 and x86_64.
for ABI in "${ABIS[@]}"; do
    TARGET=$(abi_to_target "$ABI")
    LINKER=$(abi_to_linker "$ABI")

    if [ "$TARGET" = "UNKNOWN_TARGET_FOR_$ABI" ]; then
        echo "build_rs.sh: skipping unknown ABI '$ABI'"
        continue
    fi

    echo ""
    echo "============================================================"
    echo "  Building libtwoyi.so  —  ABI=$ABI  target=$TARGET"
    echo "============================================================"

    export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C link-arg=-Wl,-e,main -C link-arg=-Wl,--dynamic-linker=$LINKER -C link-arg=-Wl,-rpath,\$ORIGIN -C link-arg=-Wl,--enable-new-dtags -C link-arg=-pie -C relocation-model=pic -C link-arg=-Wl,--undefined=interp"

    # cargo-xdk takes the Android ABI name (arm64-v8a / x86_64); it maps
    # internally to the right Rust target triple.
    cargo xdk -t "$ABI" -o ../src/main/jniLibs build $PROFILE_ARG

    # Copy wrapper script and make it executable. The wrapper is the same
    # for both ABIs (it just exec's linker64 with the .so path).
    mkdir -p "../src/main/jniLibs/$ABI"
    cp twoyi.sh "../src/main/jniLibs/$ABI/twoyi" 2>/dev/null || true
    chmod +x "../src/main/jniLibs/$ABI/twoyi" 2>/dev/null || true

    echo "build_rs.sh: $ABI done → ../src/main/jniLibs/$ABI/libtwoyi.so"
done

echo ""
echo "build_rs.sh: all ABIs done."
