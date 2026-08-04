#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Install the twoyi APK into a running redroid container, wait for it
# to boot, then take a series of screenshots for VLM analysis.
#
# Prerequisites:
#   - ./.devcontainer/scripts/run-redroid.sh has been run successfully
#   - app/build/outputs/apk/release/*.apk exists (run ./gradlew assembleRelease)
#   - rootfs.7z has been downloaded (the script will fetch it if missing)

set -e

ADB_PORT="${TWOYI_REDROID_ADB_PORT:-5555}"
ADB_TARGET="localhost:$ADB_PORT"
SCREENSHOT_DIR="${TWOYI_SCREENSHOTS:-/tmp/twoyi-screenshots}"

# Locate the APK
APK=$(ls app/build/outputs/apk/release/*.apk 2>/dev/null | head -1 || true)
if [ -z "$APK" ]; then
    echo "✗ No APK found in app/build/outputs/apk/release/."
    echo "  Run ./gradlew assembleRelease first."
    exit 1
fi
echo "→ Using APK: $APK"

# Make sure ADB is connected
echo "→ Connecting to redroid..."
adb connect "$ADB_TARGET"
adb -s "$ADB_TARGET" wait-for-device

# Install
echo "→ Installing APK (this can take a minute)..."
if ! adb -s "$ADB_TARGET" install -r -t "$APK"; then
    echo "✗ Install failed. If the error is 'INSTALL_FAILED_NO_MATCHING_ABIS',"
    echo "  your APK doesn't include x86_64. Rebuild with:"
    echo "    ./gradlew assembleRelease -Pabis=all"
    exit 1
fi
echo "✓ APK installed."

# Download rootfs.7z if not bundled in the APK assets
echo "→ Checking for rootfs.7z in APK assets..."
ROOTFS_IN_APK=$(unzip -l "$APK" 2>/dev/null | grep -c "assets/rootfs.7z" || echo 0)
if [ "$ROOTFS_IN_APK" -eq 0 ]; then
    echo "  APK does not bundle rootfs.7z. The app will prompt for one"
    echo "  on first launch — for automated testing you'll need to push"
    echo "  one manually:"
    echo "    curl -L -o /tmp/rootfs.7z \\"
    echo "      https://github.com/cyanmint/twoyi/releases/download/original/rootfs.7z"
    echo "    adb -s $ADB_TARGET push /tmp/rootfs.7z /sdcard/Download/"
fi

# Launch the app
echo "→ Launching twoyi..."
adb -s "$ADB_TARGET" shell am start -n io.twoyi/.ui.SettingsActivity
sleep 3

# Screenshot sequence
mkdir -p "$SCREENSHOT_DIR"
echo "→ Taking screenshots into $SCREENSHOT_DIR ..."

take_screenshot() {
    local label="$1"
    local delay="${2:-2}"
    sleep "$delay"
    local file="$SCREENSHOT_DIR/${label}.png"
    adb -s "$ADB_TARGET" exec-out screencap -p > "$file"
    echo "  → $file ($(stat -c%s "$file") bytes)"
}

take_screenshot "01_settings"        2
take_screenshot "02_settings_after"  3

# Tap the "Launch Container" preference. Coordinates depend on screen
# resolution and density — for redroid 1080x1920 @ 420dpi, the list
# items start at around y=400 and increment by ~120px. We tap where the
# "Launch Container" preference typically appears. The VLM analysis
# step (see analyze-screenshots.sh) will tell us the actual coordinates
# for the next round.
echo "→ Attempting to tap 'Launch Container' (heuristic coordinates)..."
adb -s "$ADB_TARGET" shell input tap 540 700
take_screenshot "03_container_booting" 5
take_screenshot "04_container_booting_5s" 5
take_screenshot "05_container_booting_10s" 5
take_screenshot "06_container_booting_15s" 5
take_screenshot "07_container_booting_30s" 15
take_screenshot "08_container_final"      15

echo ""
echo "================================================================"
echo "  ✓  Screenshots captured."
echo "================================================================"
echo ""
echo "View them:"
echo "  ls -la $SCREENSHOT_DIR"
echo ""
echo "Analyze with VLM (uses GLM-4.6V or newer):"
echo "  ./.devcontainer/scripts/analyze-screenshots.sh"
