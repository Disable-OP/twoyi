#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# scripts/extract-rootfs.sh — extract an x86_64 rootfs.tar from an
# Android SDK system image (system.img + ramdisk.img → rootfs.tar).
#
# The SDK ships system.img as an Android sparse image (simg). To get
# a usable rootfs, we:
#   1. Convert the sparse image to raw ext4 with `simg2img`.
#   2. Mount the raw image at a temp dir (requires sudo + loop device).
#   3. Also extract the ramdisk (gzipped cpio) into the mount dir so
#      /init, /init.rc, /default.prop etc. are present.
#   4. `tar` the whole thing into rootfs.tar.
#
# Requires:
#   - simg2img  (apt: android-sdk-libsparse-utils)
#   - mount + losetup (sudo)
#   - cpio, gzip (always present)
#
# Alternative (recommended if you have a booted emulator): use
# `--rootfs-source emulator` in scripts/kvm-e2e-test.sh instead — it
# extracts the rootfs from a running emulator via `adb root` and
# avoids the mount/loop dance entirely.
#
# Usage:
#   scripts/extract-rootfs.sh \
#       --system-img /path/to/system.img \
#       --ramdisk-img /path/to/ramdisk.img \
#       --output /tmp/rootfs.tar

set -euo pipefail

SYSTEM_IMG=""
RAMDISK_IMG=""
OUTPUT=""

usage() {
    sed -n '2,/^$/p' "$0" | sed 's/^# \?//' >&2
}

while [ $# -gt 0 ]; do
    case "$1" in
        --system-img)  SYSTEM_IMG="$2";  shift 2 ;;
        --ramdisk-img) RAMDISK_IMG="$2"; shift 2 ;;
        --output)      OUTPUT="$2";      shift 2 ;;
        --help|-h)     usage; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; usage; exit 2 ;;
    esac
done

if [ -z "$SYSTEM_IMG" ] || [ -z "$RAMDISK_IMG" ] || [ -z "$OUTPUT" ]; then
    echo "✗ --system-img, --ramdisk-img, and --output are all required" >&2
    usage
    exit 2
fi

if [ ! -f "$SYSTEM_IMG" ]; then
    echo "✗ system image not found: $SYSTEM_IMG" >&2
    exit 1
fi
if [ ! -f "$RAMDISK_IMG" ]; then
    echo "✗ ramdisk image not found: $RAMDISK_IMG" >&2
    exit 1
fi

# Locate simg2img
if ! command -v simg2img >/dev/null 2>&1; then
    echo "✗ simg2img not found on PATH" >&2
    echo "  Install it with one of:" >&2
    echo "    sudo apt install android-sdk-libsparse-utils" >&2
    echo "    sudo apt install libsimg2img  # older Ubuntu" >&2
    echo "  Or use the 'emulator' rootfs source instead of 'sdk_image'." >&2
    exit 1
fi

WORKDIR=$(mktemp -d -t twoyi-rootfs-XXXXXX)
MOUNT="$WORKDIR/mnt"
RAW_IMG="$WORKDIR/system.raw.img"
RAMDISK_DIR="$WORKDIR/ramdisk"

cleanup() {
    rc=$?
    sudo umount "$MOUNT" 2>/dev/null || true
    sudo losetup -d "$LOOP_DEV" 2>/dev/null || true
    rm -rf "$WORKDIR"
    exit "$rc"
}
trap cleanup EXIT INT TERM

mkdir -p "$MOUNT" "$RAMDISK_DIR"

echo "── Step 1/4: convert sparse system.img → raw ext4 ──"
echo "  input:  $SYSTEM_IMG ($(stat -c%s "$SYSTEM_IMG") bytes)"
# simg2img prints "Total of ... blocks" on success; quiet unless it fails.
if ! simg2img "$SYSTEM_IMG" "$RAW_IMG" 2>&1; then
    # If the image isn't sparse, simg2img fails. Fall back to a plain
    # copy (the file is already raw ext4).
    echo "  simg2img failed — image may already be raw. Copying as-is."
    cp "$SYSTEM_IMG" "$RAW_IMG"
fi
echo "  output: $RAW_IMG ($(stat -c%s "$RAW_IMG") bytes)"

echo "── Step 2/4: extract ramdisk.img (gzipped cpio) ──"
# ramdisk.img is gzip-compressed cpio. Newer SDK images may use lz4;
# try gzip first, fall back to lz4.
if zcat "$RAMDISK_IMG" 2>/dev/null | (cd "$RAMDISK_DIR" && cpio -idm --quiet); then
    echo "  ✓ ramdisk extracted (gzip)"
elif command -v lz4 >/dev/null 2>&1 \
     && lz4 -d "$RAMDISK_IMG" - 2>/dev/null | (cd "$RAMDISK_DIR" && cpio -idm --quiet); then
    echo "  ✓ ramdisk extracted (lz4)"
else
    echo "  ⚠ could not extract ramdisk (tried gzip + lz4) — continuing without it"
    echo "    The rootfs will be missing /init, /init.rc, /default.prop."
    echo "    Use --rootfs-source emulator for a complete rootfs."
fi
ls "$RAMDISK_DIR" 2>/dev/null | head -5 || true

echo "── Step 3/4: mount raw system image + overlay ramdisk ──"
# Set up a loop device and mount read-only. We use sudo because mounting
# ext4 images requires CAP_SYS_ADMIN (loop mounts are not user-namespaced
# for ext4 — only for FUSE filesystems).
LOOP_DEV=$(sudo losetup --show -f "$RAW_IMG")
echo "  loop device: $LOOP_DEV"
sudo mount -t ext4 -o ro "$LOOP_DEV" "$MOUNT"
echo "  ✓ mounted at $MOUNT"

# Overlay the ramdisk contents onto the mounted system image (copy, since
# we can't write to the read-only mount — we copy the relevant files into
# a writable overlay dir then add that to the tar).
OVERLAY="$WORKDIR/overlay"
mkdir -p "$OVERLAY"
if [ -d "$RAMDISK_DIR" ]; then
    cp -a "$RAMDISK_DIR/." "$OVERLAY/" 2>/dev/null || true
fi

echo "── Step 4/4: tar it all up → $OUTPUT ──"
# Tar the mounted system + the ramdisk overlay. We use --transform to
# strip leading "/" from absolute paths in the ramdisk (cpio extracts
# with absolute paths by default with -d, but our tar should have
# relative paths).
# Use --xattrs to preserve SELinux contexts (twoyi's chroot respects them).
# If --xattrs isn't supported, fall back to a plain tar.
cd "$MOUNT"
TAR_OPTS=(
    --create
    --file "$OUTPUT"
    --xattrs
    --xattrs-include='*'
    --transform='s,^/,,'  # strip leading / from absolute paths
)

if ! sudo tar "${TAR_OPTS[@]}" . 2>/dev/null; then
    echo "  ⚠ tar with --xattrs failed; retrying without"
    sudo tar --create --file "$OUTPUT" \
        --transform='s,^/,,' \
        . 2>/dev/null
fi

# Append ramdisk overlay files (init, default.prop, etc.) to the tar.
if [ -d "$OVERLAY" ] && [ -n "$(ls -A "$OVERLAY" 2>/dev/null)" ]; then
    cd "$OVERLAY"
    sudo tar --append --file "$OUTPUT" \
        --transform='s,^/,,' \
        . 2>/dev/null || true
fi

echo "  ✓ rootfs.tar: $OUTPUT ($(stat -c%s "$OUTPUT") bytes)"
echo ""
echo "── Contents (top level) ──"
tar -tf "$OUTPUT" 2>/dev/null | head -20 | sed 's/^/  /' || true
echo ""
echo "✓ Done. Push to device with:"
echo "  adb push $OUTPUT /data/local/tmp/rootfs.tar"
