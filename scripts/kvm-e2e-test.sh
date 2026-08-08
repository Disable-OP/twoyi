#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# scripts/kvm-e2e-test.sh — boots an x86_64 Android emulator with KVM,
# installs the twoyi APK, imports an x86_64 rootfs, launches the
# container, and captures logcat + screenshots for offline inspection.
#
# Invoked by .github/workflows/kvm-e2e-test.yml. Also runnable locally
# on any Linux box with KVM + Android SDK installed (set ANDROID_HOME).
#
# Usage:
#   scripts/kvm-e2e-test.sh \
#       --rootfs-source emulator|sdk_image|cyanmint \
#       --boot-wait 60 \
#       --artifact-dir /tmp/ci-artifacts
#
# The script NEVER exits non-zero on a twoyi boot failure. The whole
# point is to capture artifacts for diagnosis, so the workflow run
# succeeds (and uploads artifacts) regardless of whether the guest
# container actually booted. The verdict is written to
# ${ARTIFACT_DIR}/boot-verdict.txt — the workflow step after this
# script just `cat`s it for visibility in the GHA log.
#
# Why this script exists: the project has NEVER successfully booted a
# guest container end-to-end. The qemu_pipe GL command proxy was
# implemented in commit 8dc63f4 (Phase 1 of QEMU_PIPE_DISPATCHER_PLAN.md),
# but Phase 0 — rebuilding the full AOSP emugl renderer pipeline that
# would actually execute those GL commands — is still a stub. So as of
# today the test will likely FAIL to fully boot the container. The
# workflow is in place so that when Phase 0 lands, this test catches
# the first successful boot automatically.
#
# Reference docs:
#   - X86_64_BREAKTHROUGH.md   — the manual version of this flow that
#                                produced the first-ever x86_64 init exec
#   - TESTING_GUIDE.md §3-5    — emulator setup + boot verification
#   - download/QEMU_PIPE_DISPATCHER_PLAN.md — renderer roadmap (Phase 0)
#   - .devcontainer/scripts/run-redroid.sh — sibling approach (redroid,
#                                no KVM needed, used in codespaces)

set -u

# ---------------------------------------------------------------------------
# Defaults / arg parsing
# ---------------------------------------------------------------------------
ROOTFS_SOURCE="emulator"
BOOT_WAIT_SECONDS=60
ARTIFACT_DIR="/tmp/ci-artifacts"
AVD_NAME="twoyi_test"

while [ $# -gt 0 ]; do
    case "$1" in
        --rootfs-source)
            ROOTFS_SOURCE="$2"; shift 2 ;;
        --boot-wait)
            BOOT_WAIT_SECONDS="$2"; shift 2 ;;
        --artifact-dir)
            ARTIFACT_DIR="$2"; shift 2 ;;
        --help|-h)
            sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
            exit 0 ;;
        *)
            echo "Unknown argument: $1" >&2
            exit 2 ;;
    esac
done

mkdir -p "$ARTIFACT_DIR"
# Always start with a fresh verdict file so a stale run can't fool the
# workflow's "Print boot verdict" step.
echo "twoyi E2E test started at $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    > "$ARTIFACT_DIR/boot-verdict.txt"
echo "rootfs_source=$ROOTFS_SOURCE" >> "$ARTIFACT_DIR/boot-verdict.txt"
echo "boot_wait_seconds=$BOOT_WAIT_SECONDS" >> "$ARTIFACT_DIR/boot-verdict.txt"
echo "" >> "$ARTIFACT_DIR/boot-verdict.txt"

# ---------------------------------------------------------------------------
# Locate the Android SDK. CI installs at /usr/local/lib/android/sdk;
# locally, fall back to ANDROID_HOME / ANDROID_SDK_ROOT.
# ---------------------------------------------------------------------------
ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-/usr/local/lib/android/sdk}}"
EMULATOR_BIN="$ANDROID_HOME/emulator/emulator"
AVDMANAGER_BIN="$ANDROID_HOME/cmdline-tools/latest/bin/avdmanager"
ADB_BIN="$ANDROID_HOME/platform-tools/adb"

if [ ! -x "$ADB_BIN" ]; then
    # Try PATH as a fallback (e.g. codespaces already has adb on PATH)
    if command -v adb >/dev/null 2>&1; then
        ADB_BIN="$(command -v adb)"
    else
        echo "✗ adb not found at $ADB_BIN and not on PATH" >&2
        echo "  Set ANDROID_HOME to your Android SDK root." >&2
        exit 1
    fi
fi
if [ ! -x "$EMULATOR_BIN" ]; then
    if command -v emulator >/dev/null 2>&1; then
        EMULATOR_BIN="$(command -v emulator)"
    else
        echo "✗ emulator not found at $EMULATOR_BIN and not on PATH" >&2
        exit 1
    fi
fi

export ANDROID_HOME
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"

echo "── Environment ──"
echo "  ANDROID_HOME:    $ANDROID_HOME"
echo "  emulator:        $EMULATOR_BIN"
echo "  adb:             $ADB_BIN"
echo "  AVD name:        $AVD_NAME"
echo "  rootfs_source:   $ROOTFS_SOURCE"
echo "  boot_wait:       ${BOOT_WAIT_SECONDS}s"
echo "  artifact_dir:    $ARTIFACT_DIR"
echo ""

# ---------------------------------------------------------------------------
# Step 1: Start the emulator headless with KVM + swiftshader.
# ---------------------------------------------------------------------------
echo "── Step 1/6: start emulator ──"
# -no-window -no-audio           : headless CI
# -no-snapshot                   : cold boot (we re-extract rootfs each run)
# -no-boot-anim                  : skip the boot animation (faster boot)
# -gpu swiftshader_indirect      : software GL renderer (works headless)
# -qemu -enable-kvm              : enable hardware virtualization
# -read-only                     : the SDK system image is read-only; we
#                                 extract rootfs to the data partition
# -partition-size 4096           : 4 GB data partition (room for rootfs)
# -ports 5554,5555               : console on 5554, adb on 5555
#                                 (must specify BOTH ports, else emulator
#                                  dies with "Failed to parse option: |5554|")
"$EMULATOR_BIN" -avd "$AVD_NAME" \
    -no-window \
    -no-audio \
    -no-snapshot \
    -no-boot-anim \
    -gpu swiftshader_indirect \
    -partition-size 4096 \
    -read-only \
    -ports 5554,5555 \
    -qemu -enable-kvm \
    > "$ARTIFACT_DIR/emulator-stdout.log" \
    2> "$ARTIFACT_DIR/emulator-stderr.log" &
EMULATOR_PID=$!
echo "  emulator PID: $EMULATOR_PID"
echo "  (stdout → $ARTIFACT_DIR/emulator-stdout.log)"
echo "  (stderr → $ARTIFACT_DIR/emulator-stderr.log)"

# Wait for the device to come online. `adb wait-for-device` blocks
# until `getprop sys.boot_completed` returns 1 OR the timeout fires.
# On a KVM-accelerated ubuntu-latest runner, the SDK Android 30 image
# boots in ~60-90 s; allow 240 s for cold-cache runs.
BOOT_TIMEOUT=240
echo "  waiting for boot_completed (timeout ${BOOT_TIMEOUT}s)..."
WAITED=0
while [ "$WAITED" -lt "$BOOT_TIMEOUT" ]; do
    if ! kill -0 "$EMULATOR_PID" 2>/dev/null; then
        echo "  ✗ emulator process died before booting" >&2
        echo "  last 30 lines of stderr:" >&2
        tail -30 "$ARTIFACT_DIR/emulator-stderr.log" >&2 || true
        exit 1
    fi
    BOOTED=$("$ADB_BIN" -s emulator-5554 shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)
    if [ "$BOOTED" = "1" ]; then
        echo "  ✓ boot_completed after ${WAITED}s"
        break
    fi
    sleep 2
    WAITED=$((WAITED + 2))
    printf '.'
done
echo ''

if [ "$WAITED" -ge "$BOOT_TIMEOUT" ]; then
    echo "  ✗ emulator did not finish booting in ${BOOT_TIMEOUT}s" >&2
    echo "  last 30 lines of stderr:" >&2
    tail -30 "$ARTIFACT_DIR/emulator-stderr.log" >&2 || true
    exit 1
fi

# Make sure adb is pointed at the emulator
"$ADB_BIN" -s emulator-5554 wait-for-device
echo "  ✓ emulator is up"
echo ""

# ---------------------------------------------------------------------------
# Step 2: Install the twoyi APK.
# ---------------------------------------------------------------------------
echo "── Step 2/6: install APK ──"
APK=$(ls app/build/outputs/apk/release/*.apk 2>/dev/null | head -1 || true)
if [ -z "$APK" ]; then
    echo "  ✗ No APK found in app/build/outputs/apk/release/" >&2
    echo "    Run ./gradlew assembleRelease -Pabis=x86_64 first." >&2
    exit 1
fi
echo "  APK: $APK ($(stat -c%s "$APK") bytes)"

# -r : reinstall, keeping data
# -t : allow test packages (twoyi isn't debuggable by default, but -t
#      lets us install over a debug-signed variant if needed)
# -g : grant all runtime permissions automatically
if ! "$ADB_BIN" -s emulator-5554 install -r -t -g "$APK"; then
    echo "  ✗ Install failed" >&2
    echo "    If the error is INSTALL_FAILED_NO_MATCHING_ABIS, your APK" >&2
    echo "    doesn't include x86_64. Rebuild with:" >&2
    echo "      ./gradlew assembleRelease -Pabis=x86_64" >&2
    exit 1
fi
echo "  ✓ APK installed"
echo ""

# ---------------------------------------------------------------------------
# Step 3: Source the rootfs.
#   - emulator  : the X86_64_BREAKTHROUGH.md trick — boot the emulator,
#                 then `tar` its system/ + init + default.prop out via
#                 `adb root`. Produces a working x86_64 rootfs.
#   - sdk_image : run scripts/extract-rootfs.sh on the SDK system image
#                 (system.img + ramdisk.img → rootfs.tar). More fragile
#                 (needs sudo + simg2img + mount loop).
#   - cyanmint  : download cyanmint's arm64 rootfs. WON'T BOOT on x86_64
#                 — used only as a smoke test that twoyi starts at all
#                 (renderer initializes, init spawns) without the full
#                 boot.
# ---------------------------------------------------------------------------
echo "── Step 3/6: source rootfs ($ROOTFS_SOURCE) ──"
ROOTFS_TAR="$ARTIFACT_DIR/rootfs.tar"

case "$ROOTFS_SOURCE" in
    emulator)
        # Per X86_64_BREAKTHROUGH.md §"How to reproduce". The emulator's
        # /system is the SDK system image (Android 11, x86_64). Tarring
        # it out gives us a working rootfs with init, linker, libc, etc.
        echo "  → adb root (restarts adbd as root)"
        "$ADB_BIN" -s emulator-5554 root
        sleep 2  # adbd takes a moment to come back up as root
        "$ADB_BIN" -s emulator-5554 wait-for-device

        echo "  → tar system/ init* default.prop from the booted emulator"
        # `adb shell tar` writes to the device's /data/local/tmp; we then
        # pull it. This is faster than `adb shell tar | tar xf -` over
        # the adb pipe (which has high per-packet overhead).
        "$ADB_BIN" -s emulator-5554 shell \
            'cd / && tar cf /data/local/tmp/rootfs.tar system/ init* default.prop' \
            2>&1 | tee -a "$ARTIFACT_DIR/rootfs-extract.log"
        echo "  → pull /data/local/tmp/rootfs.tar"
        "$ADB_BIN" -s emulator-5554 pull /data/local/tmp/rootfs.tar "$ROOTFS_TAR"
        echo "  ✓ rootfs.tar: $(stat -c%s "$ROOTFS_TAR") bytes"
        ;;

    sdk_image)
        SDK_SYSIMG="$ANDROID_HOME/system-images/android-30/google_apis/x86_64"
        if [ ! -d "$SDK_SYSIMG" ]; then
            echo "  ✗ SDK system image not found at $SDK_SYSIMG" >&2
            echo "    Install it with:" >&2
            echo "      sdkmanager 'system-images;android-30;google_apis;x86_64'" >&2
            exit 1
        fi
        echo "  → running scripts/extract-rootfs.sh on $SDK_SYSIMG"
        if [ ! -x scripts/extract-rootfs.sh ]; then
            echo "  ✗ scripts/extract-rootfs.sh not found or not executable" >&2
            exit 1
        fi
        scripts/extract-rootfs.sh \
            --system-img "$SDK_SYSIMG/system.img" \
            --ramdisk-img "$SDK_SYSIMG/ramdisk.img" \
            --output "$ROOTFS_TAR" \
            2>&1 | tee -a "$ARTIFACT_DIR/rootfs-extract.log"
        if [ ! -s "$ROOTFS_TAR" ]; then
            echo "  ✗ extract-rootfs.sh did not produce $ROOTFS_TAR" >&2
            exit 1
        fi
        echo "  ✓ rootfs.tar: $(stat -c%s "$ROOTFS_TAR") bytes"
        ;;

    cyanmint)
        echo "  → downloading cyanmint's arm64 rootfs (WON'T BOOT on x86_64 — smoke test only)"
        echo "    This rootfs is for ARM64 hosts; on this x86_64 emulator it will" >> "$ARTIFACT_DIR/boot-verdict.txt"
        echo "    fail to start the guest init binary. The test still verifies" >> "$ARTIFACT_DIR/boot-verdict.txt"
        echo "    that twoyi's host-side Java UI + renderer init code runs." >> "$ARTIFACT_DIR/boot-verdict.txt"
        echo "" >> "$ARTIFACT_DIR/boot-verdict.txt"
        curl -sL -o "$ROOTFS_TAR.gz" \
            https://github.com/cyanmint/twoyi/releases/download/original/rootfs.tar.gz
        # The cyanmint release is .tar.gz — twoyi's importer accepts both,
        # but for consistency with the other paths, decompress to .tar.
        gunzip -c "$ROOTFS_TAR.gz" > "$ROOTFS_TAR"
        rm -f "$ROOTFS_TAR.gz"
        echo "  ✓ rootfs.tar: $(stat -c%s "$ROOTFS_TAR") bytes (decompressed from cyanmint rootfs.tar.gz)"
        ;;

    *)
        echo "  ✗ unknown --rootfs-source: $ROOTFS_SOURCE" >&2
        echo "    valid options: emulator, sdk_image, cyanmint" >&2
        exit 2
        ;;
esac
echo ""

# ---------------------------------------------------------------------------
# Step 4: Push the rootfs into twoyi's data dir + fix init.
# The X86_64_BREAKTHROUGH.md trick: replace the symlinked init with the
# actual binary from system/bin/init, since twoyi's chroot needs a real
# ELF binary at /init.
# ---------------------------------------------------------------------------
echo "── Step 4/6: install rootfs into twoyi data dir ──"
TWOYI_DATA=/data/data/io.twoyi
TWOYI_PROFILE="$TWOYI_DATA/profiles/default/rootfs"

# Stop twoyi if it's running (it shouldn't be — fresh boot — but be safe)
"$ADB_BIN" -s emulator-5554 shell am force-stop io.twoyi 2>/dev/null || true

# Push the tarball to a temp location, then extract it into twoyi's
# data dir. We can't `adb push` directly to /data/data/io.twoyi/.../
# because that dir is created on first launch; create it first.
"$ADB_BIN" -s emulator-5554 shell "run-as io.twoyi mkdir -p profiles/default/rootfs" 2>/dev/null \
    || "$ADB_BIN" -s emulator-5554 shell "mkdir -p $TWOYI_PROFILE" 2>/dev/null \
    || true

# As root (we already `adb root`'d above for the emulator source; do it
# again for the other sources), push + extract.
"$ADB_BIN" -s emulator-5554 root 2>/dev/null || true
sleep 1
"$ADB_BIN" -s emulator-5554 wait-for-device

echo "  → push rootfs.tar to /data/local/tmp/"
"$ADB_BIN" -s emulator-5554 push "$ROOTFS_TAR" /data/local/tmp/rootfs.tar

echo "  → extract into $TWOYI_PROFILE"
"$ADB_BIN" -s emulator-5554 shell "mkdir -p $TWOYI_PROFILE"
"$ADB_BIN" -s emulator-5554 shell \
    "cd $TWOYI_PROFILE && tar xf /data/local/tmp/rootfs.tar" \
    2>&1 | tee -a "$ARTIFACT_DIR/rootfs-extract.log" || true

# Fix init: X86_64_BREAKTHROUGH.md §4 — replace the symlinked /init
# with the actual binary from /system/bin/init. This is required
# because twoyi's chroot execs /init directly (not via /system/bin/init).
"$ADB_BIN" -s emulator-5554 shell \
    "if [ -L $TWOYI_PROFILE/init ]; then \
         rm $TWOYI_PROFILE/init && \
         cp $TWOYI_PROFILE/system/bin/init $TWOYI_PROFILE/init; \
     fi" 2>&1 | tee -a "$ARTIFACT_DIR/rootfs-extract.log" || true

# SELinux permissive for first-boot debugging (X86_64_BREAKTHROUGH.md §5)
"$ADB_BIN" -s emulator-5554 shell setenforce 0 2>/dev/null || true

# Sanity-check the rootfs landed
"$ADB_BIN" -s emulator-5554 shell "ls -la $TWOYI_PROFILE/ | head -20" \
    2>&1 | tee -a "$ARTIFACT_DIR/rootfs-extract.log" || true
echo "  ✓ rootfs installed"
echo ""

# ---------------------------------------------------------------------------
# Step 5: Launch twoyi + capture logcat + screenshots.
# Coordinates for "Launch Container" tap come from
# .devcontainer/scripts/test-twoyi.sh (heuristic; pixel_5 is 1080x1920).
# The settings list starts ~y=400 and items are ~120px apart, so
# y=700 is a reasonable guess for the first non-header preference.
# ---------------------------------------------------------------------------
echo "── Step 5/6: launch twoyi + capture ──"

# Clear logcat so we only capture the container-launch session
"$ADB_BIN" -s emulator-5554 logcat -c

# Start logcat capture in the background; we'll kill it after the
# boot-wait window. -v time prepends a timestamp to each line.
"$ADB_BIN" -s emulator-5554 logcat -v time '*:V' \
    > "$ARTIFACT_DIR/logcat.txt" 2>&1 &
LOGCAT_PID=$!
echo "  logcat capture started (PID $LOGCAT_PID → $ARTIFACT_DIR/logcat.txt)"

echo "  → launching io.twoyi/.ui.SettingsActivity"
"$ADB_BIN" -s emulator-5554 shell am start -n io.twoyi/.ui.SettingsActivity
sleep 3

# Take the pre-launch screenshot so we can see what the settings list
# looks like (and confirm the app actually opened).
"$ADB_BIN" -s emulator-5554 exec-out screencap -p \
    > "$ARTIFACT_DIR/screenshot-00_settings.png"
echo "  ✓ screenshot-00_settings.png"

# Tap "Launch Container". Coordinates are heuristic; see note in
# .devcontainer/scripts/test-twoyi.sh — adjust if the VLM analysis
# of the settings screenshot says otherwise.
echo "  → tap 'Launch Container' (heuristic coords 540, 700)"
"$ADB_BIN" -s emulator-5554 shell input tap 540 700

# Screenshot sequence: 5s, 15s, 30s, 60s (or however long boot_wait is)
# after the tap. We use a cumulative sleep so the intervals are
# measured from launch, not from each screenshot.
PREV=0
for i in 5 15 30 60; do
    if [ "$i" -gt "$BOOT_WAIT_SECONDS" ]; then
        break
    fi
    SLEEP=$((i - PREV))
    sleep "$SLEEP"
    "$ADB_BIN" -s emulator-5554 exec-out screencap -p \
        > "$ARTIFACT_DIR/screenshot-${i}s.png"
    SIZE=$(stat -c%s "$ARTIFACT_DIR/screenshot-${i}s.png" 2>/dev/null || echo 0)
    echo "  ✓ screenshot-${i}s.png (${SIZE} bytes)"
    PREV=$i
done

# If boot_wait > 60, sleep the remainder and take a final screenshot.
if [ "$BOOT_WAIT_SECONDS" -gt 60 ]; then
    REMAINDER=$((BOOT_WAIT_SECONDS - 60))
    sleep "$REMAINDER"
    "$ADB_BIN" -s emulator-5554 exec-out screencap -p \
        > "$ARTIFACT_DIR/screenshot-${BOOT_WAIT_SECONDS}s.png"
    echo "  ✓ screenshot-${BOOT_WAIT_SECONDS}s.png"
fi

# Stop logcat
kill "$LOGCAT_PID" 2>/dev/null || true
wait "$LOGCAT_PID" 2>/dev/null || true
echo "  ✓ logcat capture stopped ($(stat -c%s "$ARTIFACT_DIR/logcat.txt") bytes)"
echo ""

# ---------------------------------------------------------------------------
# Step 6: Verdict — did the container boot?
# We grep the captured logcat for the milestones from TESTING_GUIDE.md §5.1.
# This is informational only — the workflow never fails on boot failure
# (yet) because Phase 0 (full AOSP emugl renderer) isn't shipped.
# ---------------------------------------------------------------------------
echo "── Step 6/6: verdict ──"

# Save a filtered logcat for quick scanning — the full logcat can be
# 100k+ lines on a booted emulator.
grep -E 'KR64 INFO|KR64 WARN|KR64 ERROR|CORE|NEW_RENDERER|CLIENT_EGL|SOCKET_MONITOR|BOOT_COMPLETED' \
    "$ARTIFACT_DIR/logcat.txt" 2>/dev/null \
    > "$ARTIFACT_DIR/logcat-filtered.txt" || true

# Look for the milestones from TESTING_GUIDE.md §5.1
KR64_START=$(grep -c 'KR64 INFO.*kr64 daemon starting' "$ARTIFACT_DIR/logcat-filtered.txt" || true)
QEMU_PIPE_CREATED=$(grep -c 'created device /dev/qemu_pipe' "$ARTIFACT_DIR/logcat-filtered.txt" || true)
PIPE_AVAIL=$(grep -c 'QEMU pipe device.*availability: true' "$ARTIFACT_DIR/logcat-filtered.txt" || true)
PIPE_CONN=$(grep -c 'Successfully connected to QEMU pipe' "$ARTIFACT_DIR/logcat-filtered.txt" || true)
GL_CTX=$(grep -c 'GL context created successfully' "$ARTIFACT_DIR/logcat-filtered.txt" || true)
BOOT_COMPLETED=$(grep -c 'BOOT_COMPLETED' "$ARTIFACT_DIR/logcat-filtered.txt" || true)

# Is the twoyi process still alive?
TWOYI_PID=$("$ADB_BIN" -s emulator-5554 shell pidof io.twoyi 2>/dev/null | tr -d '\r' || true)

# Tombstones (crash dumps) created during the run?
TOMBSTONE_COUNT=$("$ADB_BIN" -s emulator-5554 shell \
    'find /data/tombstones -name tombstone_* -newer /data/local/tmp/rootfs.tar 2>/dev/null | wc -l' \
    2>/dev/null | tr -d '\r' || echo 0)

# Pull any tombstones for offline inspection
if [ "$TOMBSTONE_COUNT" -gt 0 ]; then
    mkdir -p "$ARTIFACT_DIR/tombstones"
    "$ADB_BIN" -s emulator-5554 shell 'find /data/tombstones -name tombstone_* -newer /data/local/tmp/rootfs.tar 2>/dev/null' \
        | while read -r ts; do
            "$ADB_BIN" -s emulator-5554 pull "$ts" "$ARTIFACT_DIR/tombstones/" 2>/dev/null || true
        done
fi

# Pull dropbox + ANR entries too
mkdir -p "$ARTIFACT_DIR/dropbox" "$ARTIFACT_DIR/anr"
"$ADB_BIN" -s emulator-5554 shell 'ls /data/system/dropbox/ 2>/dev/null' \
    | while read -r f; do
        case "$f" in
            *io.twoyi*|*crash*|*anr*)
                "$ADB_BIN" -s emulator-5554 pull "/data/system/dropbox/$f" \
                    "$ARTIFACT_DIR/dropbox/" 2>/dev/null || true
                ;;
        esac
    done

# Write the verdict
{
    echo "── Boot milestone checklist ──"
    echo "  KR64 daemon started:           $([ "$KR64_START" -gt 0 ] && echo "✓ ($KR64_START lines)" || echo "✗")"
    echo "  /dev/qemu_pipe created:        $([ "$QEMU_PIPE_CREATED" -gt 0 ] && echo "✓ ($QEMU_PIPE_CREATED lines)" || echo "✗")"
    echo "  Pipe availability: true:       $([ "$PIPE_AVAIL" -gt 0 ] && echo "✓ ($PIPE_AVAIL lines)" || echo "✗")"
    echo "  Pipe connected:                $([ "$PIPE_CONN" -gt 0 ] && echo "✓ ($PIPE_CONN lines)" || echo "✗")"
    echo "  GL context created:            $([ "$GL_CTX" -gt 0 ] && echo "✓ ($GL_CTX lines)" || echo "✗")"
    echo "  BOOT_COMPLETED signal:         $([ "$BOOT_COMPLETED" -gt 0 ] && echo "✓ ($BOOT_COMPLETED lines)" || echo "✗")"
    echo ""
    echo "── Runtime state ──"
    if [ -n "$TWOYI_PID" ]; then
        echo "  io.twoyi process: ALIVE (pid $TWOYI_PID)"
    else
        echo "  io.twoyi process: NOT RUNNING (crashed or never started)"
    fi
    echo "  tombstones during run:         $TOMBSTONE_COUNT"
    echo ""
    echo "── Overall verdict ──"
    if [ "$BOOT_COMPLETED" -gt 0 ]; then
        echo "  ✓✓✓ CONTAINER BOOTED — BOOT_COMPLETED signal received."
        echo "  This is the holy grail. Phase 0 has landed!"
    elif [ -n "$TWOYI_PID" ] && [ "$GL_CTX" -gt 0 ]; then
        echo "  ◐ PARTIAL — twoyi alive + GL context created, but no BOOT_COMPLETED."
        echo "  Likely cause: full AOSP emugl renderer (Phase 0) not yet shipped."
        echo "  See download/QEMU_PIPE_DISPATCHER_PLAN.md."
    elif [ -n "$TWOYI_PID" ]; then
        echo "  ◐ PARTIAL — twoyi process is alive but no GL context."
        echo "  Likely cause: renderer init failed before reaching GL context creation."
    elif [ "$ROOTFS_SOURCE" = "cyanmint" ]; then
        echo "  ◐ EXPECTED FAILURE — cyanmint rootfs is arm64, won't boot on x86_64."
        echo "  This was a smoke test only. Use --rootfs-source emulator for a real test."
    else
        echo "  ✗ twoyi crashed or never started."
        echo "  Inspect $ARTIFACT_DIR/logcat.txt + tombstones/ for details."
    fi
    echo ""
    echo "── Artifacts ──"
    echo "  APK:                app/build/outputs/apk/release/*.apk"
    echo "  Full logcat:        $ARTIFACT_DIR/logcat.txt"
    echo "  Filtered logcat:    $ARTIFACT_DIR/logcat-filtered.txt"
    echo "  Screenshots:        $ARTIFACT_DIR/screenshot-*.png"
    echo "  Tombstones:         $ARTIFACT_DIR/tombstones/"
    echo "  Rootfs extract log: $ARTIFACT_DIR/rootfs-extract.log"
    echo "  Emulator stdout:    $ARTIFACT_DIR/emulator-stdout.log"
    echo "  Emulator stderr:    $ARTIFACT_DIR/emulator-stderr.log"
} >> "$ARTIFACT_DIR/boot-verdict.txt"

cat "$ARTIFACT_DIR/boot-verdict.txt"

# Clean up the emulator
echo ""
echo "── Cleanup ──"
"$ADB_BIN" -s emulator-5554 emu kill 2>/dev/null || true
kill "$EMULATOR_PID" 2>/dev/null || true
wait "$EMULATOR_PID" 2>/dev/null || true
echo "  ✓ emulator killed"

# Always exit 0 — the workflow step after this just prints the verdict
# and uploads artifacts. A non-zero exit here would skip the upload
# steps, hiding the very evidence we just collected.
exit 0
