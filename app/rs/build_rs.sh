#! /bin/bash

# Exit on error
set -e

#
# Copyright Disclaimer: AI-Generated Content
# This file was created by GitHub Copilot, an AI coding assistant.
# AI-generated content is not subject to copyright protection and is provided
# without any warranty, express or implied, including warranties of merchantability,
# fitness for a particular purpose, or non-infringement.
# Use at your own risk.
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#

# Configure as PIE executable for direct execution with JNI compatibility
# PIE with INTERP segment allows direct execution: ./libtwoyi.so
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C link-arg=-Wl,-e,main -C link-arg=-Wl,--dynamic-linker=/system/bin/linker64 -C link-arg=-Wl,-rpath,\$ORIGIN -C link-arg=-Wl,--enable-new-dtags -C link-arg=-pie -C relocation-model=pic -C link-arg=-Wl,--undefined=interp"
cargo xdk -t arm64-v8a -o ../src/main/jniLibs build $1

# Copy wrapper script and make it executable
cp twoyi.sh ../src/main/jniLibs/arm64-v8a/twoyi 2>/dev/null || true
chmod +x ../src/main/jniLibs/arm64-v8a/twoyi 2>/dev/null || true
