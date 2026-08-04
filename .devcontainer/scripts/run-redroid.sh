#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Start an x86_64 redroid container for testing twoyi.
#
# Redroid is "Android In Container" — it runs Android directly in a
# Docker container using namespace isolation, NOT KVM. So it works in
# GitHub Codespaces even when /dev/kvm is unavailable.
#
# This script starts redroid:13.0.0 (Android 13, x86_64) and exposes
# ADB on port 5555. The companion test-twoyi.sh script then connects
# to it and installs the twoyi APK.

set -e

CONTAINER_NAME="${TWOYI_REDROID_NAME:-redroid-twoyi}"
ANDROID_VERSION="${TWOYI_REDROID_VERSION:-13.0.0}"
ADB_PORT="${TWOYI_REDROID_ADB_PORT:-5555}"

# If a codespace KVM check has been run, prefer it; otherwise default
# to "no KVM" (redroid doesn't need it anyway).
KVM_AVAILABLE="${KVM_AVAILABLE:-no}"
if [ -f /tmp/kvm-verdict.txt ]; then
    source /tmp/kvm-verdict.txt
fi

echo "================================================================"
echo "  Starting redroid container"
echo "    Name:    $CONTAINER_NAME"
echo "    Android: $ANDROID_VERSION (x86_64)"
echo "    ADB:     tcp:$ADB_PORT"
echo "    KVM:     $KVM_AVAILABLE  (redroid doesn't require KVM)"
echo "================================================================"

# Stop + remove any existing container with the same name
if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "→ Removing existing container..."
    docker rm -f "$CONTAINER_NAME" >/dev/null
fi

# Pull the redroid image if not present
echo "→ Pulling redroid:$ANDROID_VERSION image (this is ~500MB on first run)..."
docker pull "redroid/redroid:$ANDROID_VERSION-latest"

# redroid requires --privileged for binder/ashmem. It does NOT require
# /dev/kvm — that's the whole point of using redroid in a codespace.
echo "→ Starting container..."
docker run -d \
    --name "$CONTAINER_NAME" \
    --privileged \
    -p "$ADB_PORT:5555" \
    -v redroid-data:/data \
    "redroid/redroid:$ANDROID_VERSION-latest" \
    androidboot.redroid_gpu_mode=guest \
    androidboot.redroid_width=1080 \
    androidboot.redroid_height=1920 \
    androidboot.redroid_dpi=420 \
    androidboot.hardware=qemu \

echo ""
echo "✓ Container started. Waiting for Android to boot..."

# Wait for the ADB port to be reachable
MAX_WAIT=120
WAITED=0
while [ $WAITED -lt $MAX_WAIT ]; do
    if adb connect "localhost:$ADB_PORT" 2>/dev/null | grep -q "connected"; then
        break
    fi
    sleep 2
    WAITED=$((WAITED + 2))
    printf '.'
done
echo ''

if [ $WAITED -ge $MAX_WAIT ]; then
    echo "✗ Timed out after ${MAX_WAIT}s waiting for ADB. Container logs:"
    docker logs "$CONTAINER_NAME" | tail -30
    exit 1
fi

echo "✓ ADB connected. Waiting for boot_completed..."
MAX_WAIT=120
WAITED=0
while [ $WAITED -lt $MAX_WAIT ]; do
    BOOTED=$(adb -s "localhost:$ADB_PORT" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')
    if [ "$BOOTED" = "1" ]; then
        break
    fi
    sleep 2
    WAITED=$((WAITED + 2))
    printf '.'
done
echo ''

if [ $WAITED -ge $MAX_WAIT ]; then
    echo "✗ Android did not finish booting in ${MAX_WAIT}s. Container logs:"
    docker logs "$CONTAINER_NAME" | tail -30
    exit 1
fi

echo ""
echo "================================================================"
echo "  ✓  redroid is up and booted."
echo "================================================================"
echo ""
echo "Connect manually:"
echo "  adb connect localhost:$ADB_PORT"
echo "  adb -s localhost:$ADB_PORT shell"
echo ""
echo "Stop:"
echo "  docker stop $CONTAINER_NAME"
echo ""
echo "Next:"
echo "  ./.devcontainer/scripts/test-twoyi.sh"
