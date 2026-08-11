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

        echo "  → tar system/ init* default.prop apex/ from the booted emulator"
        # IMPORTANT: We also tar apex/ because on Android 11+,
        # /system/bin/linker64 and /system/lib64/libc.so are symlinks
        # into /apex/com.android.runtime/. Without /apex in the rootfs,
        # these symlinks are dangling and the dynamic linker crashes
        # with SIGSEGV at address 0x86 (NULL pointer dereference in
        # linker64's soinfo handling).
        "$ADB_BIN" -s emulator-5554 shell \
            'cd / && tar cf /data/local/tmp/rootfs.tar system/ vendor/ init* default.prop apex/' \
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
# ProfileManager.initializeProfiles expects the rootfs at
# <dataDir>/profiles/default/rootfs and creates a symlink at
# <dataDir>/rootfs -> <dataDir>/profiles/default/rootfs. RomManager reads
# via <dataDir>/rootfs (following the symlink). Extract directly to the
# profile path so ProfileManager doesn't need to migrate anything — it
# just creates the symlink. This avoids the DirectoryNotEmptyException
# flake that occurred when a stale <dataDir>/rootfs directory blocked
# Files.move() during migration (see cleanup block below).
TWOYI_PROFILE="$TWOYI_DATA/profiles/default/rootfs"

# Stop twoyi if it's running (it shouldn't be — fresh boot — but be safe)
"$ADB_BIN" -s emulator-5554 shell am force-stop io.twoyi 2>/dev/null || true

# Clean up stale dev/event directory from previous runs (kr64 fails if
# /data/user/0/io.twoyi/dev/event exists as a directory or stale socket)
"$ADB_BIN" -s emulator-5554 root 2>/dev/null || true
sleep 2
"$ADB_BIN" -s emulator-5554 wait-for-device 2>/dev/null || true
# Use both paths — /data/user/0/io.twoyi is a symlink to /data/data/io.twoyi
# but some operations don't follow symlinks
"$ADB_BIN" -s emulator-5554 shell "rm -rf /data/user/0/io.twoyi/dev /data/data/io.twoyi/dev" 2>/dev/null || true
# Also remove the kr64-stderr.log from previous runs
"$ADB_BIN" -s emulator-5554 shell "rm -f /data/user/0/io.twoyi/kr64-stderr.log /data/data/io.twoyi/kr64-stderr.log" 2>/dev/null || true
# Verify the cleanup worked
DEV_EXISTS=$("$ADB_BIN" -s emulator-5554 shell "ls -la /data/user/0/io.twoyi/dev 2>&1" 2>/dev/null || true)
if echo "$DEV_EXISTS" | grep -q "No such file"; then
    echo "  ✓ cleaned up stale dev/ directory"
else
    echo "  ⚠ dev/ directory still exists: $DEV_EXISTS"
    # Force remove with chmod
    "$ADB_BIN" -s emulator-5554 shell "chmod -R 777 /data/user/0/io.twoyi/dev 2>/dev/null; rm -rf /data/user/0/io.twoyi/dev" 2>/dev/null || true
fi

# As root (we already `adb root`'d above for the emulator source; do it
# again for the other sources) before the cleanup + extract below.
"$ADB_BIN" -s emulator-5554 root 2>/dev/null || true
sleep 1
"$ADB_BIN" -s emulator-5554 wait-for-device

# Fix CI flake: clean up stale state from previous runs.
#
# ProfileManager.initializeProfiles does:
#   1. If <dataDir>/rootfs exists as a real directory (not a symlink),
#      Files.move() it to <dataDir>/profiles/default/rootfs.
#   2. Calls updateRootfsSymlink(), which does Files.deleteIfExists()
#      on <dataDir>/rootfs — this throws DirectoryNotEmptyException if
#      <dataDir>/rootfs is a non-empty directory (e.g. because the
#      Files.move above failed when <dataDir>/profiles/default/rootfs
#      already existed and was non-empty).
#
# The previous fix (commit 131c89b) only removed <dataDir>/rootfs before
# extraction — but the extraction recreated it, AND a stale
# <dataDir>/profiles/default/rootfs from a prior failed run still
# blocked the migration. Both paths have to be clean.
#
# Now that we extract directly to <dataDir>/profiles/default/rootfs
# (see TWOYI_PROFILE above), ProfileManager doesn't need to migrate.
# We just need to ensure BOTH <dataDir>/rootfs (so no migration is
# attempted) and <dataDir>/profiles (so the new extraction lands in a
# clean path) are absent before extraction. Remove them from both
# /data/data and /data/user/0 (the latter is a symlink to the former
# but some operations don't follow symlinks).
"$ADB_BIN" -s emulator-5554 shell "
    rm -rf /data/user/0/io.twoyi/rootfs /data/data/io.twoyi/rootfs
    rm -rf /data/user/0/io.twoyi/profiles /data/data/io.twoyi/profiles
    echo 'cleaned stale rootfs + profiles directories'
" 2>/dev/null || true

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

# Ensure /system/bin/init exists in the rootfs.
# On some emulator images, /system/bin/init is a symlink that doesn't
# survive tar extraction. Create it as a copy of /init if missing.
"$ADB_BIN" -s emulator-5554 shell \
    "if [ ! -e $TWOYI_PROFILE/system/bin/init ]; then \
         cp $TWOYI_PROFILE/init $TWOYI_PROFILE/system/bin/init; \
     fi" 2>&1 | tee -a "$ARTIFACT_DIR/rootfs-extract.log" || true
echo "  ✓ ensured /system/bin/init exists in rootfs"

# Fix permissions: the rootfs was extracted as root (via adb root), so
# all files/dirs are owned by root. The twoyi app runs as u0_a167 and
# can't create symlinks (ensureLibSymlink for libkr64.so, libOpenglRender.so)
# or write to rootfs/system/lib64/. Chown the entire rootfs to the app's uid.
# On Android, the app uid is determined at install time; we look it up
# via `pm list packages -U io.twoyi` and extract the uid.
APP_UID=$("$ADB_BIN" -s emulator-5554 shell "pm list packages -U io.twoyi" 2>/dev/null | sed 's/.*uid://' | tr -d '\r\n ')
if [ -n "$APP_UID" ]; then
    echo "  → chown rootfs to app uid $APP_UID"
    "$ADB_BIN" -s emulator-5554 shell "chown -R $APP_UID:$APP_UID $TWOYI_PROFILE" 2>&1 | tee -a "$ARTIFACT_DIR/rootfs-extract.log" || true
    "$ADB_BIN" -s emulator-5554 shell "chmod -R 0755 $TWOYI_PROFILE" 2>&1 | tee -a "$ARTIFACT_DIR/rootfs-extract.log" || true
else
    echo "  ⚠ could not determine app uid — skipping chown (ensureLibSymlink may fail)"
fi

# SELinux permissive for first-boot debugging (X86_64_BREAKTHROUGH.md §5)
"$ADB_BIN" -s emulator-5554 shell setenforce 0 2>/dev/null || true

# Sanity-check the rootfs landed
"$ADB_BIN" -s emulator-5554 shell "ls -la $TWOYI_PROFILE/ | head -20" \
    2>&1 | tee -a "$ARTIFACT_DIR/rootfs-extract.log" || true
echo "  ✓ rootfs installed"
echo ""

# ---------------------------------------------------------------------------
# Step 5: Pre-launch kr64 as ROOT + launch twoyi + capture logcat + screenshots.
#
# The app's own kr64 (launched by core.rs) CANNOT chroot() or mount() —
# the zygote's seccomp filter blocks these syscalls and kills the process
# with SIGSYS (signal 31). Only a ROOT-launched kr64 can do these ops.
#
# So we pre-launch kr64 as root via `adb shell` BEFORE starting the app.
# The app's core.rs detects that /dev/qemu_pipe already exists and skips
# its own kr64 launch + qemu_pipe proxy. The app's role is JUST to
# provide the renderer (libOpenglRender.so) and the UI.
#
# The root kr64:
#   1. Creates all /dev devices (qemu_pipe, touch, key, event, gb, etc.)
#   2. unshare(CLONE_NEWNS) + mount tmpfs + pivot_root (or chroot fallback)
#   3. unshare(CLONE_NEWPID) — makes init actually PID 1 (no LD_PRELOAD hack needed)
#   4. fork() + exec init
# ---------------------------------------------------------------------------
echo "── Step 5/6: pre-launch root kr64 + launch twoyi + capture ──"

# Clear logcat so we only capture the container-launch session
"$ADB_BIN" -s emulator-5554 logcat -c

# Start logcat capture in the background; we'll kill it after the
# boot-wait window. -v time prepends a timestamp to each line.
"$ADB_BIN" -s emulator-5554 logcat -v time '*:V' \
    > "$ARTIFACT_DIR/logcat.txt" 2>&1 &
LOGCAT_PID=$!
echo "  logcat capture started (PID $LOGCAT_PID → $ARTIFACT_DIR/logcat.txt)"

# --- Extract libkr64.so + libgetpid_hook.so from the APK ---
# The root kr64 needs to be at a path accessible to root. We extract it
# from the APK and push it to /data/local/tmp/.
# The script is run from the repo root, so the APK is at
# app/build/outputs/apk/release/*.apk (relative to cwd).
APK_PATH=$(ls app/build/outputs/apk/release/*.apk 2>/dev/null | head -1)
if [ -z "$APK_PATH" ] && [ -n "${GITHUB_WORKSPACE:-}" ]; then
    APK_PATH=$(ls "$GITHUB_WORKSPACE"/app/build/outputs/apk/release/*.apk 2>/dev/null | head -1)
fi
if [ -n "$APK_PATH" ] && [ -f "$APK_PATH" ]; then
    # Convert to absolute path BEFORE any subshell cd — unzip runs inside
    # a `(cd "$EXTRACT_DIR" && unzip ...)` subshell, and a relative
    # APK_PATH would not resolve from /tmp/apk-extract/. This was the
    # root cause of "unzip failed: 9" in every previous KVM run: the
    # glob matched the APK (so APK_PATH was set), but the subshell's cd
    # made the relative path unresolvable.
    APK_ABS=$(readlink -f "$APK_PATH" 2>/dev/null || echo "$APK_PATH")
    echo "  → extracting libkr64.so + libgetpid_hook.so from APK ($APK_ABS)"
    EXTRACT_DIR=/tmp/apk-extract
    rm -rf "$EXTRACT_DIR" && mkdir -p "$EXTRACT_DIR"
    # Use unzip -d to extract into EXTRACT_DIR without cd-ing (avoids
    # the relative-path-after-cd bug entirely).
    unzip -o "$APK_ABS" "lib/x86_64/libkr64.so" "lib/x86_64/libgetpid_hook.so" -d "$EXTRACT_DIR" || echo "  ⚠ unzip failed: $?"
    echo "  → EXTRACT_DIR contents:"
    ls -la "$EXTRACT_DIR/lib/x86_64/" 2>/dev/null || echo "    (lib/x86_64/ not found in extraction)"
    if [ -f "$EXTRACT_DIR/lib/x86_64/libkr64.so" ]; then
        "$ADB_BIN" -s emulator-5554 push "$EXTRACT_DIR/lib/x86_64/libkr64.so" /data/local/tmp/kr64
        "$ADB_BIN" -s emulator-5554 shell chmod 755 /data/local/tmp/kr64
        echo "  ✓ pushed kr64 to /data/local/tmp/kr64"
    else
        echo "  ⚠ libkr64.so not found in APK extraction — trying installed APK"
        # Try to copy from the installed APK's native lib dir
        "$ADB_BIN" -s emulator-5554 root 2>/dev/null || true
        sleep 1
        "$ADB_BIN" -s emulator-5554 wait-for-device 2>/dev/null || true
        "$ADB_BIN" -s emulator-5554 shell "
            APK_DIR=\$(dirname \$(pm path io.twoyi | head -1 | sed 's/package://'))
            if [ -f \"\$APK_DIR/lib/x86_64/libkr64.so\" ]; then
                cp \"\$APK_DIR/lib/x86_64/libkr64.so\" /data/local/tmp/kr64
                chmod 755 /data/local/tmp/kr64
                echo 'copied kr64 from installed APK'
            else
                echo 'kr64 not found in installed APK either'
                # Last resort: look in the extracted rootfs (the
                # libkr64.so symlink is created later by RomManager on
                # app launch, but the rootfs.tar may already contain
                # one from a previous extraction).
                if [ -e $TWOYI_PROFILE/system/lib64/libkr64.so ]; then
                    cp -L $TWOYI_PROFILE/system/lib64/libkr64.so /data/local/tmp/kr64
                    chmod 755 /data/local/tmp/kr64
                    echo 'copied kr64 from extracted rootfs'
                fi
            fi
        " 2>&1 | tail -5
    fi
    if [ -f "$EXTRACT_DIR/lib/x86_64/libgetpid_hook.so" ]; then
        "$ADB_BIN" -s emulator-5554 push "$EXTRACT_DIR/lib/x86_64/libgetpid_hook.so" /data/local/tmp/libgetpid_hook.so
        echo "  ✓ pushed libgetpid_hook.so to /data/local/tmp/"
    else
        echo "  ⚠ libgetpid_hook.so not found in APK extraction — trying installed APK"
        "$ADB_BIN" -s emulator-5554 shell "
            APK_DIR=\$(dirname \$(pm path io.twoyi | head -1 | sed 's/package://'))
            if [ -f \"\$APK_DIR/lib/x86_64/libgetpid_hook.so\" ]; then
                cp \"\$APK_DIR/lib/x86_64/libgetpid_hook.so\" /data/local/tmp/libgetpid_hook.so
                echo 'copied libgetpid_hook from installed APK'
            fi
        " 2>&1 | tail -3
    fi
else
    echo "  ⚠ APK not found — cannot extract libkr64.so"
    echo "    CWD: $(pwd)"
    ls -la app/build/outputs/apk/release/ 2>/dev/null || echo "    APK dir does not exist"
    EXTRACT_DIR=/tmp/apk-extract  # ensure variable is set for the push logic below
    mkdir -p "$EXTRACT_DIR"
fi

# --- Push libgetpid_hook.so to ROOT of rootfs ---
# Use the file from the CI runner's EXTRACT_DIR (not from the device's
# /data/local/tmp/, which may not have been populated).
if [ -f "$EXTRACT_DIR/lib/x86_64/libgetpid_hook.so" ]; then
    "$ADB_BIN" -s emulator-5554 push "$EXTRACT_DIR/lib/x86_64/libgetpid_hook.so" "$TWOYI_PROFILE/libgetpid_hook.so" 2>&1 | tail -2
    echo "  ✓ pushed libgetpid_hook.so to root of rootfs (from EXTRACT_DIR)"
elif [ -f "$EXTRACT_DIR/lib/x86_64/libkr64.so" ]; then
    # libgetpid_hook.so wasn't extracted, but libkr64.so was — try extracting
    # libgetpid_hook.so separately. Use absolute APK path + unzip -d (no cd).
    APK_ABS2=$(readlink -f "$APK_PATH" 2>/dev/null || echo "$APK_PATH")
    unzip -o "$APK_ABS2" "lib/x86_64/libgetpid_hook.so" -d "$EXTRACT_DIR" 2>/dev/null || true
    if [ -f "$EXTRACT_DIR/lib/x86_64/libgetpid_hook.so" ]; then
        "$ADB_BIN" -s emulator-5554 push "$EXTRACT_DIR/lib/x86_64/libgetpid_hook.so" "$TWOYI_PROFILE/libgetpid_hook.so" 2>&1 | tail -2
        echo "  ✓ pushed libgetpid_hook.so to root of rootfs (extracted separately)"
    else
        echo "  ⚠ libgetpid_hook.so not found in APK extraction"
    fi
else
    echo "  ⚠ EXTRACT_DIR not populated — trying device /data/local/tmp/"
    "$ADB_BIN" -s emulator-5554 push /data/local/tmp/libgetpid_hook.so "$TWOYI_PROFILE/libgetpid_hook.so" 2>&1 | tail -2 || echo "  ⚠ push from /data/local/tmp/ failed"
fi
"$ADB_BIN" -s emulator-5554 shell "chmod 644 $TWOYI_PROFILE/libgetpid_hook.so && ls -la $TWOYI_PROFILE/libgetpid_hook.so" 2>&1 | tail -2

# --- Push libtwoyi_loader_shlib.so (seccomp/SIGSYS virtualization) ---
# This is the REAL virtualization library. Loaded via LD_PRELOAD.
if [ -f "$EXTRACT_DIR/lib/x86_64/libtwoyi_loader_shlib.so" ]; then
    "$ADB_BIN" -s emulator-5554 push "$EXTRACT_DIR/lib/x86_64/libtwoyi_loader_shlib.so" "$TWOYI_PROFILE/libtwoyi_loader_shlib.so" 2>&1 | tail -2
    "$ADB_BIN" -s emulator-5554 shell "chmod 644 $TWOYI_PROFILE/libtwoyi_loader_shlib.so" 2>&1
    echo "  ✓ pushed libtwoyi_loader_shlib.so to rootfs"
elif [ -f "$EXTRACT_DIR/lib/x86_64/libkr64.so" ]; then
    APK_ABS3=$(readlink -f "$APK_PATH" 2>/dev/null || echo "$APK_PATH")
    unzip -o "$APK_ABS3" "lib/x86_64/libtwoyi_loader_shlib.so" -d "$EXTRACT_DIR" 2>/dev/null || true
    if [ -f "$EXTRACT_DIR/lib/x86_64/libtwoyi_loader_shlib.so" ]; then
        "$ADB_BIN" -s emulator-5554 push "$EXTRACT_DIR/lib/x86_64/libtwoyi_loader_shlib.so" "$TWOYI_PROFILE/libtwoyi_loader_shlib.so" 2>&1 | tail -2
        "$ADB_BIN" -s emulator-5554 shell "chmod 644 $TWOYI_PROFILE/libtwoyi_loader_shlib.so" 2>&1
        echo "  ✓ pushed libtwoyi_loader_shlib.so to rootfs (extracted separately)"
    else
        echo "  ⚠ libtwoyi_loader_shlib.so not found in APK — seccomp virtualization disabled"
    fi
else
    echo "  ⚠ EXTRACT_DIR not populated — libtwoyi_loader_shlib.so not available"
fi

# Also create the libkr64.so symlink in the rootfs (RomManager does this
# when the app starts, but we want it ready before the app launches).
"$ADB_BIN" -s emulator-5554 shell "
    if [ ! -e $TWOYI_PROFILE/system/lib64/libkr64.so ]; then
        ln -sf /data/local/tmp/kr64 $TWOYI_PROFILE/system/lib64/libkr64.so
    fi
" 2>/dev/null || true

# --- Pre-launch kr64 as ROOT ---
# Run kr64 with namespaces enabled (root can do unshare/pivot_root).
# Skip seccomp — the filter's SIGSYS handler crashes init with SIGSEGV.
#
# TWOYI_SKIP_PRELOAD: when set, kr64 skips LD_PRELOAD (getpid_hook.so).
# This is a diagnostic mode: init will exit 31 (getpid != 1), but if it
# exits 31 instead of SIGSEGV, we know the linker can load init without
# the hook. Set TWOYI_SKIP_PRELOAD=1 in the CI env to test.
#
# TWOYI_LD_DEBUG: when set, kr64 propagates it as LD_DEBUG to the guest
# init. This enables bionic linker debug output (which library is being
# loaded when the crash happens). The output goes to stderr (captured in
# kr64-stderr.log). Default is "2" (libs level — library load/unload).
# Set TWOYI_LD_DEBUG=0 in the CI env to disable.
SKIP_PRELOAD_ENV="${TWOYI_SKIP_PRELOAD:-}"
INIT_PATH_OVERRIDE="${TWOYI_INIT_PATH:-}"
NO_NAMESPACES_ENV="${TWOYI_NO_NAMESPACES:-}"
LD_DEBUG_ENV="${TWOYI_LD_DEBUG:-2}"
if [ -n "$NO_NAMESPACES_ENV" ]; then
    echo "  → pre-launching kr64 as root (TWOYI_NO_NAMESPACES=1, no pivot_root/chroot)"
elif [ -n "$SKIP_PRELOAD_ENV" ]; then
    echo "  → pre-launching kr64 as root (TWOYI_SKIP_PRELOAD=1, no LD_PRELOAD)"
elif [ -n "$INIT_PATH_OVERRIDE" ]; then
    echo "  → pre-launching kr64 as root (TWOYI_INIT_PATH=$INIT_PATH_OVERRIDE)"
else
    echo "  → pre-launching kr64 as root (with namespaces, no seccomp)"
fi
"$ADB_BIN" -s emulator-5554 shell "
    export LD_LIBRARY_PATH=/system/lib64:/vendor/lib64
    ${SKIP_PRELOAD_ENV:+export TWOYI_SKIP_PRELOAD=1}
    export TWOYI_LD_DEBUG=$LD_DEBUG_ENV
    /data/local/tmp/kr64 \
        --rootfs $TWOYI_PROFILE \
        --data-dir /data/user/0/io.twoyi \
        --vmid 0 \
        --no-seccomp \
        ${NO_NAMESPACES_ENV:+--no-namespaces} \
        ${INIT_PATH_OVERRIDE:+--init $INIT_PATH_OVERRIDE} \
        > /data/user/0/io.twoyi/kr64-stderr.log 2>&1 &
    echo \$! > /data/local/tmp/kr64.pid
    echo 'kr64 launched'
" 2>&1 | tail -5

# Wait for /dev/qemu_pipe to be created (kr64 is setting up)
echo "  → waiting for kr64 to create /dev/qemu_pipe..."
for i in $(seq 1 15); do
    if "$ADB_BIN" -s emulator-5554 shell "test -S $TWOYI_PROFILE/dev/qemu_pipe" 2>/dev/null; then
        echo "  ✓ /dev/qemu_pipe created (after ${i}s)"
        break
    fi
    sleep 1
done
if ! "$ADB_BIN" -s emulator-5554 shell "test -S $TWOYI_PROFILE/dev/qemu_pipe" 2>/dev/null; then
    echo "  ⚠ /dev/qemu_pipe not created after 15s — kr64 may have failed"
    echo "  → pulling kr64-stderr.log for diagnosis"
    "$ADB_BIN" -s emulator-5554 pull /data/user/0/io.twoyi/kr64-stderr.log "$ARTIFACT_DIR/kr64-stderr.log" 2>/dev/null || true
    if [ -f "$ARTIFACT_DIR/kr64-stderr.log" ]; then
        echo "  === kr64-stderr.log ==="
        cat "$ARTIFACT_DIR/kr64-stderr.log"
    fi
fi

echo "  → launching io.twoyi/.ui.SettingsActivity (to trigger rootfs detection)"
"$ADB_BIN" -s emulator-5554 shell am start -n io.twoyi/.ui.SettingsActivity
sleep 3

# Take the pre-launch screenshot so we can see what the settings list
# looks like (and confirm the app actually opened).
"$ADB_BIN" -s emulator-5554 exec-out screencap -p \
    > "$ARTIFACT_DIR/screenshot-00_settings.png"
echo "  ✓ screenshot-00_settings.png"

# Give SettingsActivity time to initialize before launching Render2Activity
sleep 5

# Launch the container DIRECTLY via Render2Activity instead of tapping
# the settings preference. Tapping at fixed coordinates is fragile
# (different screen densities, layout changes). Render2Activity is the
# activity that SettingsActivity's "Launch Container" preference starts,
# so launching it directly is equivalent and 100% reliable.
echo "  → launching io.twoyi/.Render2Activity (direct container launch)"
"$ADB_BIN" -s emulator-5554 shell am start -n io.twoyi/.Render2Activity
sleep 2

# Verify Render2Activity is in the foreground. If it crashed back to
# settings or the home screen, log it but continue (screenshots will
# show what happened).
CURRENT_FOCUS=$("$ADB_BIN" -s emulator-5554 shell dumpsys activity activities 2>/dev/null \
    | grep -E 'mResumedActivity|topResumedActivity' | head -1 || true)
echo "  current focus: $CURRENT_FOCUS"
if echo "$CURRENT_FOCUS" | grep -q "Render2Activity"; then
    echo "  ✓ Render2Activity is foreground"
elif echo "$CURRENT_FOCUS" | grep -q "SettingsActivity"; then
    echo "  ⚠ Render2Activity fell back to SettingsActivity (crash?)"
    # Fallback: try the tap approach in case Render2Activity needs the
    # preference click path to set up state first.
    echo "  → fallback: tap 'Launch Container' (heuristic coords 540, 700)"
    "$ADB_BIN" -s emulator-5554 shell input tap 540 700
    sleep 2
else
    echo "  ⚠ unexpected focus — container may have crashed"
fi

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

# Pull the twoyi log file (kr64's stderr/stdout is redirected here by
# core.rs). This is CRITICAL for debugging — without it, we can't see
# kr64's [KR64 INFO] / [KR64 ERROR] messages, only the tombstones.
"$ADB_BIN" -s emulator-5554 root 2>/dev/null || true
sleep 1
"$ADB_BIN" -s emulator-5554 wait-for-device

# Pull the pre-launched kr64 log (final pull — may have more output than
# the initial pull before the app started). Overwrites the earlier copy.
"$ADB_BIN" -s emulator-5554 pull /data/local/tmp/kr64.log "$ARTIFACT_DIR/kr64-prelaunch.log" 2>/dev/null || true
if [ -f "$ARTIFACT_DIR/kr64-prelaunch.log" ]; then
    LOG_SIZE=$(stat -c%s "$ARTIFACT_DIR/kr64-prelaunch.log")
    echo "  ✓ pulled kr64-prelaunch.log (final, $LOG_SIZE bytes)"
    if [ "$LOG_SIZE" -gt 0 ]; then
        echo "  ── kr64 log (last 40 lines) ──"
        tail -40 "$ARTIFACT_DIR/kr64-prelaunch.log" | sed 's/^/    /'
    fi
else
    echo "  ⚠ could not pull /data/local/tmp/kr64.log"
fi

# Check if kr64 is still alive at the end
KR64_PID_END=$("$ADB_BIN" -s emulator-5554 shell "pidof libkr64.so" 2>/dev/null | tr -d '\r ' || true)
if [ -n "$KR64_PID_END" ]; then
    echo "  ✓ kr64 process still alive at end (pid $KR64_PID_END)"
else
    echo "  ⚠ kr64 process not found at end — may have crashed or exited"
fi

# Check if init is still alive at the end
INIT_PID_END=$("$ADB_BIN" -s emulator-5554 shell "pidof init" 2>/dev/null | tr -d '\r ' || true)
if [ -n "$INIT_PID_END" ]; then
    echo "  ✓ init process alive at end (pid $INIT_PID_END)"
else
    echo "  ⚠ init process not found at end — may have exited"
fi

"$ADB_BIN" -s emulator-5554 pull /data/data/io.twoyi/log.txt "$ARTIFACT_DIR/twoyi-log.txt" 2>/dev/null || true
if [ -f "$ARTIFACT_DIR/twoyi-log.txt" ]; then
    echo "  ✓ pulled twoyi-log.txt ($(stat -c%s "$ARTIFACT_DIR/twoyi-log.txt") bytes)"
else
    echo "  ⚠ could not pull /data/data/io.twoyi/log.txt (file may not exist yet)"
fi

# Pull the kr64 daemon's stderr log (separate from the app's log.txt)
# Try both the root kr64's log and the app's kr64 log
"$ADB_BIN" -s emulator-5554 pull /data/user/0/io.twoyi/kr64-stderr.log "$ARTIFACT_DIR/kr64-stderr.log" 2>/dev/null || true
"$ADB_BIN" -s emulator-5554 pull /data/user/0/io.twoyi/kr64-app-stderr.log "$ARTIFACT_DIR/kr64-app-stderr.log" 2>/dev/null || true
for logf in kr64-stderr.log kr64-app-stderr.log; do
    if [ -f "$ARTIFACT_DIR/$logf" ]; then
        echo "  ✓ pulled $logf ($(stat -c%s "$ARTIFACT_DIR/$logf") bytes)"
        echo "  === $logf ==="
        cat "$ARTIFACT_DIR/$logf"
    fi
done

# Pull the twoyi-loader debug log (written by libtwoyi_loader_shlib.so)
"$ADB_BIN" -s emulator-5554 pull /data/local/tmp/twoyi-loader.log "$ARTIFACT_DIR/twoyi-loader.log" 2>/dev/null || true
if [ -f "$ARTIFACT_DIR/twoyi-loader.log" ]; then
    echo "  ✓ pulled twoyi-loader.log ($(stat -c%s "$ARTIFACT_DIR/twoyi-loader.log") bytes)"
    echo "  === twoyi-loader.log ==="
    cat "$ARTIFACT_DIR/twoyi-loader.log"
fi

# Pull the vold stderr log (vold's own error output, redirected by our loader)
"$ADB_BIN" -s emulator-5554 pull /data/local/tmp/twoyi-vold-stderr.log "$ARTIFACT_DIR/twoyi-vold-stderr.log" 2>/dev/null || true
if [ -f "$ARTIFACT_DIR/twoyi-vold-stderr.log" ]; then
    echo "  ✓ pulled twoyi-vold-stderr.log ($(stat -c%s "$ARTIFACT_DIR/twoyi-vold-stderr.log") bytes)"
    echo "  === twoyi-vold-stderr.log ==="
    cat "$ARTIFACT_DIR/twoyi-vold-stderr.log"
fi

# Save a filtered logcat for quick scanning — the full logcat can be
# 100k+ lines on a booted emulator.
grep -E 'KR64 INFO|KR64 WARN|KR64 ERROR|CORE|NEW_RENDERER|CLIENT_EGL|SOCKET_MONITOR|BOOT_COMPLETED|TWOYI_RENDERER|emugl' \
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
