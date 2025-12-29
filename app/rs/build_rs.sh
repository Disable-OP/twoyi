#! /bin/bash

#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#

cargo xdk -t arm64-v8a -o ../src/main/jniLibs build $1

# Make the .so executable so it can be run from shell
chmod +x ../src/main/jniLibs/arm64-v8a/libtwoyi.so 2>/dev/null || true

# Copy wrapper script
cp twoyi.sh ../src/main/jniLibs/arm64-v8a/twoyi 2>/dev/null || true
chmod +x ../src/main/jniLibs/arm64-v8a/twoyi 2>/dev/null || true
