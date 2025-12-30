#! /bin/bash

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

# Set entry point to 'main' so the library can be executed directly via linker64
# Append to existing RUSTFLAGS to avoid overwriting any existing flags
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C link-arg=-Wl,-e,main"
cargo xdk -t arm64-v8a -o ../src/main/jniLibs build $1

# Copy wrapper script and make it executable
cp twoyi.sh ../src/main/jniLibs/arm64-v8a/twoyi 2>/dev/null || true
chmod +x ../src/main/jniLibs/arm64-v8a/twoyi 2>/dev/null || true
