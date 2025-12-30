#!/bin/bash
#
# Copyright Disclaimer: AI-Generated Content
# This file was created by GitHub Copilot, an AI coding assistant.
# AI-generated content is not subject to copyright protection and is provided
# without any warranty, express or implied, including warranties of merchantability,
# fitness for a particular purpose, or non-infringement.
# Use at your own risk.
#
# Test script to verify libtwoyi.so structure and symbols

set -e

LIB_PATH="app/src/main/jniLibs/arm64-v8a/libtwoyi.so"

echo "Testing libtwoyi.so structure..."
echo "================================"
echo ""

# Check if library exists
if [ ! -f "$LIB_PATH" ]; then
    echo "ERROR: Library not found at $LIB_PATH"
    echo "Please build the library first: cd app/rs && sh build_rs.sh --release"
    exit 1
fi

echo "✓ Library exists at $LIB_PATH"
echo ""

# Check file type
echo "File type:"
file "$LIB_PATH"
echo ""

# Check ELF header and entry point
echo "ELF Header:"
readelf -h "$LIB_PATH" | grep -E "(Type|Entry)"
echo ""

# Get entry point address (get the last field which is the hex address)
ENTRY_POINT=$(readelf -h "$LIB_PATH" | awk '/Entry point address:/ {print $NF}')
echo "Entry point address: $ENTRY_POINT"

if [ "$ENTRY_POINT" = "0x0" ]; then
    echo "ERROR: Entry point is 0x0 - library cannot be executed via linker64"
    exit 1
fi
echo "✓ Entry point is set (non-zero)"
echo ""

# Check for main symbol
echo "Checking for 'main' symbol:"
if ! MAIN_ADDR=$(nm -D "$LIB_PATH" 2>/dev/null | grep " T main$" | awk '{print $1}'); then
    echo "ERROR: 'main' symbol not found or nm failed"
    echo "This could mean the library is stripped or nm is not available"
    exit 1
fi
if [ -z "$MAIN_ADDR" ]; then
    echo "ERROR: 'main' symbol not found in dynamic symbol table"
    exit 1
fi
echo "✓ main symbol found at address: 0x$MAIN_ADDR"
echo ""

# Verify entry point matches main address
if [ "$ENTRY_POINT" = "0x$MAIN_ADDR" ]; then
    echo "✓ Entry point matches 'main' function address"
else
    echo "WARNING: Entry point ($ENTRY_POINT) does not match main address (0x$MAIN_ADDR)"
    echo "This may still work, but the entry point should ideally point to main"
fi
echo ""

# Check for JNI_OnLoad
echo "Checking for 'JNI_OnLoad' symbol:"
if nm -D "$LIB_PATH" | grep -q " T JNI_OnLoad"; then
    echo "✓ JNI_OnLoad symbol found"
else
    echo "ERROR: JNI_OnLoad symbol not found - JNI mode will not work"
    exit 1
fi
echo ""

# Check for exported twoyi functions
echo "Checking for exported twoyi_* functions:"
TWOYI_FUNCS=$(nm -D "$LIB_PATH" | grep " T twoyi_" | wc -l)
echo "✓ Found $TWOYI_FUNCS exported twoyi_* functions:"
nm -D "$LIB_PATH" | grep " T twoyi_"
echo ""

echo "================================"
echo "All tests passed! ✓"
echo ""
echo "The library should be invokable via:"
echo "  /system/bin/linker64 $LIB_PATH --help"
echo ""
echo "On Android device:"
echo "  adb push $LIB_PATH /data/local/tmp/"
echo "  adb shell LD_LIBRARY_PATH=/data/local/tmp /system/bin/linker64 /data/local/tmp/libtwoyi.so --help"
