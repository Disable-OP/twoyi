#!/bin/bash
#
# Test script for libtwoyi.so in redroid container
# This script tests the three requirements:
# 1. Direct ./libtwoyi.so execution (via linker64)
# 2. Auto-seek libOpenglRender.so without LD_LIBRARY_PATH
# 3. Proper argument reception

set -e

echo "=== Testing libtwoyi.so in redroid ==="
echo ""

# Check if redroid container is running
if ! docker ps | grep -q redroid-test; then
    echo "Starting redroid container..."
    docker run -d --privileged --name redroid-test -p 5555:5555 redroid/redroid:12.0.0-latest
    echo "Waiting for container to start..."
    sleep 15
fi

echo "Container architecture:"
docker exec redroid-test uname -m

echo ""
echo "Note: redroid is x86_64, but libtwoyi.so is built for arm64-v8a"
echo "For proper testing, we need either:"
echo "  1. Build libtwoyi.so for x86_64, or"
echo "  2. Use arm64 redroid image, or"
echo "  3. Use qemu-aarch64 in the container"
echo ""

# For now, let's document the test commands that should work on arm64:
echo "=== Test commands for ARM64 device/emulator ==="
echo ""

echo "Test 1: Invoke via linker64 (should work without LD_LIBRARY_PATH)"
echo "  docker cp app/src/main/jniLibs/arm64-v8a/libtwoyi.so redroid-test:/data/local/tmp/"
echo "  docker cp app/src/main/jniLibs/arm64-v8a/libloader.so redroid-test:/data/local/tmp/"
echo "  docker cp app/src/main/jniLibs/arm64-v8a/libOpenglRender.so redroid-test:/data/local/tmp/"
echo "  docker exec redroid-test sh -c 'cd /data/local/tmp && /system/bin/linker64 ./libtwoyi.so --help'"
echo ""

echo "Test 2: Verify RUNPATH is working (libraries found without LD_LIBRARY_PATH)"
echo "  docker exec redroid-test sh -c 'cd /data/local/tmp && /system/bin/linker64 ./libtwoyi.so --loader ./libloader.so --help'"
echo ""

echo "Test 3: Verify arguments are received"
echo "  docker exec redroid-test sh -c 'cd /data/local/tmp && /system/bin/linker64 ./libtwoyi.so --width 1920 --height 1080'"
echo "  (should show: argc: 5, with all arguments listed)"
echo ""

echo "=== Cleanup ==="
echo "docker stop redroid-test && docker rm redroid-test"
