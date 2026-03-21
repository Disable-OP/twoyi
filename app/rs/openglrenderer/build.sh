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
echo "Building new libOpenglRender.so"
echo "=========================================="

# Build the library using cargo-xdk for Android
cd "$(dirname "$0")"

# Build for ARM64
echo "Building for arm64-v8a..."
cargo xdk -t arm64-v8a build --release

# Copy the output to jniLibs
echo "Copying library to jniLibs..."
cp -v target/aarch64-linux-android/release/libOpenglRender.so ../../src/main/jniLibs/arm64-v8a/libOpenglRender_new.so

echo "=========================================="
echo "Build complete!"
echo "New library: app/src/main/jniLibs/arm64-v8a/libOpenglRender_new.so"
echo "=========================================="

# Verify the exported symbols
echo ""
echo "Exported symbols:"
nm -D ../../src/main/jniLibs/arm64-v8a/libOpenglRender_new.so | grep -E "startOpenGLRenderer|destroyOpenGLSubwindow|repaintOpenGLDisplay|setNativeWindow|resetSubWindow|removeSubWindow" || true

echo ""
echo "Library size comparison:"
echo "Legacy: $(ls -lh ../../src/main/jniLibs/arm64-v8a/libOpenglRender.so 2>/dev/null | awk '{print $5}' || echo 'N/A')"
echo "New:    $(ls -lh ../../src/main/jniLibs/arm64-v8a/libOpenglRender_new.so 2>/dev/null | awk '{print $5}' || echo 'N/A')"
