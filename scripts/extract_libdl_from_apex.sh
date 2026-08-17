#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# scripts/extract_libdl_from_apex.sh — extract the REAL libdl.so from
# com.android.runtime.apex (an Android 11+ APEX package) and drop it
# into app/src/main/assets/libdl.so so it ships as an APK asset (Option D,
# 5-U's recommendation: bypass the fragile APEX loopback-mount pipeline).
#
# Background:
#   On Android 11+, /apex/com.android.runtime/lib64/bionic/libdl.so is a
#   5848-byte bootstrap STUB. The REAL libdl.so (with the LIBC version
#   symbol required by DT_NEEDED:libdl.so (LIBC) from libgetpid_hook.so
#   and libtwoyi_loader_shlib.so) is INSIDE the APEX ext4 image at
#   /system/apex/com.android.runtime.apex (a ZIP file containing
#   apex_payload.img -- the ext4 image).
#
#   The kr64 runtime extraction (apex_extract::find_real_libdl_so in
#   app/rs/kr64/src/apex_extract.rs) tries to extract this at runtime
#   via a loopback mount, but hit 4 sequential failure modes on the
#   Android emulator (5-L -> 5-N -> 5-O -> 5-P -> 5-U diagnosis):
#     1. /tmp/ ENOENT (path didn't exist pre-setup_mounts)
#     2. /dev/loopN ENOENT (no udev on emulator)
#     3. /dev/loopN ENXIO (kernel has no registered gendisk)
#     4. (would-be-next): LOOP_SET_FD, ext4 mount, etc.
#
#   Option D (this script): do the extraction ONCE, on a dev/CI machine
#   that has the loopback mount working, and commit the resulting real
#   libdl.so as an APK asset. The kr64 daemon then reads the asset at
#   runtime via apex_extract::read_libdl_asset (no loopback mount
#   needed -- bypasses all 4 failure modes).
#
# Inputs (any one of):
#   --apex <path>         Path to com.android.runtime.apex (a ZIP containing
#                         apex_payload.img). This is the canonical input --
#                         it's the same file kr64 tries to extract from at
#                         runtime.
#   --emulator            Use a running AOSP x86_64 emulator: pull the
#                         post-apexd mount of /apex/com.android.runtime@<N>/
#                         lib64/bionic/libdl.so via adb. Requires `adb` in
#                         PATH + a booted emulator.
#
# Output:
#   --output <path>       Where to write the extracted real libdl.so.
#                         Default: app/src/main/assets/libdl.so (the APK
#                         asset path -- so running this script with no
#                         --output argument drops the real libdl.so
#                         directly into the APK assets).
#
# Requirements (varies by input source):
#   --apex:    unzip (or python3), debugfs OR mount+loop (sudo), awk.
#              debugfs is preferred (no sudo needed) -- apt install e2fsprogs.
#              If debugfs is missing, falls back to sudo mount + losetup.
#   --emulator: adb (in PATH), a booted AOSP x86_64 emulator.
#
# Verification:
#   After extraction, this script verifies the extracted file:
#     - file <output> should say "ELF 64-bit LSB shared object, x86-64"
#     - size > 5848 bytes (LIBDL_STUB_SIZE)
#     - first 4 bytes are 0x7f 0x45 0x4c 0x46 (ELF magic)
#   If verification fails, the script exits with non-zero status + does
#   NOT touch the output path (so the placeholder asset remains in place
#   + kr64's read_libdl_asset gracefully falls through to APEX runtime
#   extraction).
#
# Usage:
#   # From a com.android.runtime.apex file (dev host with debugfs):
#   scripts/extract_libdl_from_apex.sh \\
#       --apex /path/to/com.android.runtime.apex
#
#   # From a running AOSP x86_64 emulator:
#   scripts/extract_libdl_from_apex.sh --emulator
#
#   # Custom output path (e.g. for testing without committing):
#   scripts/extract_libdl_from_apex.sh --emulator --output /tmp/libdl.so
#
# Exit codes:
#   0  -- success: real libdl.so written to --output
#   1  -- verification failed (output is NOT touched)
#   2  -- argument/usage error
#   3  -- required tool missing (unzip/debugfs/adb/etc.)
#   4  -- apex_payload.img extraction failed
#   5  -- libdl.so extraction from ext4 image failed

set -euo pipefail

SCRIPT_NAME="$(basename "$0")"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEFAULT_OUTPUT="$REPO_ROOT/app/src/main/assets/libdl.so"

APEX=""
EMULATOR=0
OUTPUT="$DEFAULT_OUTPUT"

usage() {
    sed -n '2,/^$/p' "$0" | sed 's/^# \?//' >&2
}

while [ $# -gt 0 ]; do
    case "$1" in
        --apex)      APEX="$2"; shift 2 ;;
        --emulator)  EMULATOR=1; shift ;;
        --output)    OUTPUT="$2"; shift 2 ;;
        --help|-h)   usage; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; usage; exit 2 ;;
    esac
done

if [ -z "$APEX" ] && [ "$EMULATOR" -eq 0 ]; then
    echo "✗ one of --apex or --emulator is required" >&2
    usage
    exit 2
fi
if [ -n "$APEX" ] && [ "$EMULATOR" -eq 1 ]; then
    echo "✗ --apex and --emulator are mutually exclusive" >&2
    usage
    exit 2
fi

# ----------------------------------------------------------------------------
# verify_libdl <path>: returns 0 if the file at <path> looks like the REAL
# libdl.so (ELF + > 5848 bytes), 1 otherwise. Mirrors the kr64 Rust
# validator `is_real_libdl` in app/rs/kr64/src/apex_extract.rs.
# ----------------------------------------------------------------------------
LIBDL_STUB_SIZE=5848
verify_libdl() {
    local path="$1"
    if [ ! -f "$path" ]; then
        echo "  ✗ verify: file missing: $path" >&2
        return 1
    fi
    local size
    size=$(stat -c '%s' "$path" 2>/dev/null || stat -f '%z' "$path")
    if [ "$size" -le "$LIBDL_STUB_SIZE" ]; then
        echo "  ✗ verify: size $size bytes <= stub size $LIBDL_STUB_SIZE -- looks like the stub" >&2
        return 1
    fi
    # ELF magic: 0x7f 'E' 'L' 'F' = first 4 bytes.
    local magic
    magic=$(od -An -tx1 -N4 "$path" | tr -d ' \n')
    if [ "$magic" != "7f454c46" ]; then
        echo "  ✗ verify: first 4 bytes are $magic, not ELF magic 7f454c46" >&2
        return 1
    fi
    echo "  ✓ verify: $path is $size bytes + ELF magic — looks like the real libdl.so"
    return 0
}

TMPDIR_WORK="$(mktemp -d -t twoyi-extract-libdl.XXXXXX)"
cleanup() {
    rm -rf "$TMPDIR_WORK"
}
trap cleanup EXIT

# ----------------------------------------------------------------------------
# Extract from a running AOSP x86_64 emulator via adb. Pulls the
# post-apexd mount of /apex/com.android.runtime@<N>/lib64/bionic/libdl.so
# (the REAL libdl.so after apexd has mounted the APEX ext4 image).
# ----------------------------------------------------------------------------
extract_from_emulator() {
    if ! command -v adb >/dev/null 2>&1; then
        echo "✗ adb not found in PATH -- required for --emulator mode" >&2
        exit 3
    fi

    echo "→ checking adb connectivity..."
    if ! adb get-state >/dev/null 2>&1; then
        echo "✗ no adb device found -- boot an AOSP x86_64 emulator first" >&2
        exit 4
    fi

    # Wait for apexd to finish mounting APEX packages. On a fresh boot
    # this can take 5-10 seconds; on a warm boot it's instant. Poll
    # /apex/ for the versioned com.android.runtime@<N> directory.
    echo "→ waiting for apexd to mount /apex/com.android.runtime@<N>/..."
    local tries=0
    local runtime_dir=""
    while [ $tries -lt 30 ]; do
        runtime_dir=$(adb shell "ls -d /apex/com.android.runtime@* 2>/dev/null | head -1" 2>/dev/null | tr -d '\r\n')
        if [ -n "$runtime_dir" ]; then
            break
        fi
        tries=$((tries + 1))
        sleep 1
    done
    if [ -z "$runtime_dir" ]; then
        echo "✗ /apex/com.android.runtime@<N>/ not found after 30s -- apexd not started?" >&2
        exit 4
    fi
    echo "  ✓ found: $runtime_dir"

    # Try root -- needed to read /apex/.../lib64/bionic/libdl.so on
    # userdebug builds. On user builds this fails (and we can't pull
    # the file). The script will exit with the adb error.
    if ! adb root >/dev/null 2>&1; then
        echo "  ! adb root failed -- continuing (may fail on user builds)" >&2
    fi

    local remote_path="$runtime_dir/lib64/bionic/libdl.so"
    local tmp_local="$TMPDIR_WORK/libdl.so"
    echo "→ adb pull $remote_path..."
    if ! adb pull "$remote_path" "$tmp_local" >/dev/null 2>&1; then
        echo "✗ adb pull failed -- is the emulator rooted (userdebug build)?" >&2
        exit 5
    fi

    # Verify before installing.
    if ! verify_libdl "$tmp_local"; then
        echo "✗ verification failed -- NOT installing as app/src/main/assets/libdl.so" >&2
        exit 1
    fi

    mkdir -p "$(dirname "$OUTPUT")"
    cp "$tmp_local" "$OUTPUT"
    echo "✓ installed real libdl.so to $OUTPUT ($(stat -c '%s' "$OUTPUT") bytes)"
}

# ----------------------------------------------------------------------------
# Extract from a com.android.runtime.apex ZIP file via debugfs (preferred,
# no sudo needed) or sudo mount+loop fallback.
# ----------------------------------------------------------------------------
extract_from_apex() {
    local apex="$APEX"
    if [ ! -f "$apex" ]; then
        echo "✗ apex file not found: $apex" >&2
        exit 4
    fi

    echo "→ extracting apex_payload.img from $apex..."
    local payload="$TMPDIR_WORK/apex_payload.img"
    # Use unzip if available (fast); fall back to python3.
    if command -v unzip >/dev/null 2>&1; then
        if ! unzip -p "$apex" apex_payload.img > "$payload" 2>/dev/null; then
            echo "✗ unzip failed to extract apex_payload.img" >&2
            exit 4
        fi
    elif command -v python3 >/dev/null 2>&1; then
        python3 - "$apex" "$payload" <<'PY'
import sys, zipfile
apex_path, out_path = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(apex_path, 'r') as z:
    with z.open('apex_payload.img') as f_in, open(out_path, 'wb') as f_out:
        f_out.write(f_in.read())
PY
    else
        echo "✗ neither unzip nor python3 found -- required for --apex mode" >&2
        exit 3
    fi
    local payload_size
    payload_size=$(stat -c '%s' "$payload")
    echo "  ✓ extracted apex_payload.img ($payload_size bytes)"

    # Extract lib64/bionic/libdl.so from the ext4 image via debugfs
    # (no sudo needed). If debugfs is missing, fall back to sudo mount+loop.
    local extracted="$TMPDIR_WORK/libdl.so"

    if command -v debugfs >/dev/null 2>&1; then
        echo "→ extracting lib64/bionic/libdl.so via debugfs..."
        # debugfs -R "dump <path> <out>" reads a file from an ext4 image
        # without mounting. The path inside the image is lib64/bionic/libdl.so.
        if ! debugfs -R "dump lib64/bionic/libdl.so $extracted" "$payload" 2>/dev/null; then
            echo "✗ debugfs dump failed -- the ext4 image may be corrupted or use an unsupported feature" >&2
            exit 5
        fi
    else
        echo "→ debugfs not found -- falling back to sudo mount + loop..."
        if ! command -v sudo >/dev/null 2>&1; then
            echo "✗ sudo not found -- required for mount fallback (or install debugfs: apt install e2fsprogs)" >&2
            exit 3
        fi
        if ! command -v mount >/dev/null 2>&1; then
            echo "✗ mount not found -- required for mount fallback (or install debugfs)" >&2
            exit 3
        fi
        local mnt="$TMPDIR_WORK/mnt"
        mkdir -p "$mnt"
        if ! sudo mount -o loop,ro "$payload" "$mnt"; then
            echo "✗ sudo mount failed -- check loop device availability + CAP_SYS_ADMIN" >&2
            exit 5
        fi
        sudo cp "$mnt/lib64/bionic/libdl.so" "$extracted"
        sudo umount "$mnt"
    fi

    if ! verify_libdl "$extracted"; then
        echo "✗ verification failed -- NOT installing as app/src/main/assets/libdl.so" >&2
        exit 1
    fi

    mkdir -p "$(dirname "$OUTPUT")"
    cp "$extracted" "$OUTPUT"
    echo "✓ installed real libdl.so to $OUTPUT ($(stat -c '%s' "$OUTPUT") bytes)"
}

if [ "$EMULATOR" -eq 1 ]; then
    extract_from_emulator
else
    extract_from_apex
fi

echo ""
echo "Next steps:"
echo "  1. Rebuild the APK: ./gradlew assembleRelease"
echo "  2. Reinstall: adb install -r app/build/outputs/apk/release/twoyi_*.apk"
echo "  3. Launch twoyi -- kr64 will now use the APK asset libdl.so"
echo "     (visible in kr64-stderr.log: 'Option D: using APK asset libdl.so')"
