#!/system/bin/sh
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Wrapper script to execute libtwoyi.so from command line
# This script loads the library and calls its main function

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LIB_PATH="$SCRIPT_DIR/libtwoyi.so"

# Check if library exists
if [ ! -f "$LIB_PATH" ]; then
    echo "Error: libtwoyi.so not found at $LIB_PATH"
    exit 1
fi

# Execute the library using the linker directly
# This allows the .so to be executed as if it were an executable
exec /system/bin/linker64 "$LIB_PATH" "$@"
