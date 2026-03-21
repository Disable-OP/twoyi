#!/bin/bash

# Copyright Disclaimer: AI-Generated Content
# This file was created by GitHub Copilot, an AI coding assistant.
# AI-generated content is not subject to copyright protection and is provided
# without any warranty, express or implied, including warranties of merchantability,
# fitness for a particular purpose, or non-infringement.
# Use at your own risk.

# Exit on error
set -e

echo "=========================================="
echo "Building new libloader.so (loader64)"
echo "=========================================="

# Build the library using cargo-xdk for Android
cd "$(dirname "$0")"

# Build for ARM64
echo "Building for arm64-v8a..."
cargo xdk -t arm64-v8a build --release

# Copy the output to jniLibs
echo "Copying library to jniLibs..."
cp -v target/aarch64-linux-android/release/libloader.so ../../src/main/jniLibs/arm64-v8a/libloader_new.so

# Make it executable (for direct execution as loader64)
chmod +x ../../src/main/jniLibs/arm64-v8a/libloader_new.so

echo "=========================================="
echo "Build complete!"
echo "New library: app/src/main/jniLibs/arm64-v8a/libloader_new.so"
echo "=========================================="

# Verify the file
echo ""
echo "File info:"
file ../../src/main/jniLibs/arm64-v8a/libloader_new.so

echo ""
echo "Library size comparison:"
echo "Legacy: $(ls -lh ../../src/main/jniLibs/arm64-v8a/libloader.so 2>/dev/null | awk '{print $5}' || echo 'N/A')"
echo "New:    $(ls -lh ../../src/main/jniLibs/arm64-v8a/libloader_new.so 2>/dev/null | awk '{print $5}' || echo 'N/A')"

echo ""
echo "Entry point check:"
readelf -h ../../src/main/jniLibs/arm64-v8a/libloader_new.so | grep "Entry point"

echo ""
echo "Dynamic interpreter:"
readelf -l ../../src/main/jniLibs/arm64-v8a/libloader_new.so | grep interpreter || echo "No interpreter found"
