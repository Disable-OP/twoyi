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
# When TWRP_MODE=1, the script boots the TWRP recovery image instead
# of a full Android rootfs. Set via the --twrp flag (or the TWOYI_TWRP
# env var for the workflow). See the big TWRP block in Step 3 / Step 4
# / Step 5 for what changes.
TWRP_MODE=0
# Path to the TWRP boot image (relative to repo root). Override via
# --twrp-img for testing with a different TWRP image. The default is
# the TWRP 3.7.0 image shipped in assets/twrp/.
TWRP_IMG_DEFAULT="assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img"

while [ $# -gt 0 ]; do
    case "$1" in
        --rootfs-source)
            ROOTFS_SOURCE="$2"; shift 2 ;;
        --boot-wait)
            BOOT_WAIT_SECONDS="$2"; shift 2 ;;
        --artifact-dir)
            ARTIFACT_DIR="$2"; shift 2 ;;
        --twrp)
            TWRP_MODE=1; shift ;;
        --twrp-img)
            TWRP_IMG_DEFAULT="$2"; shift 2 ;;
        --help|-h)
            sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
            exit 0 ;;
        *)
            echo "Unknown argument: $1" >&2
            exit 2 ;;
    esac
done

# TWOYI_TWRP env var also flips TWRP_MODE (used by the workflow's
# `twrp` input). The --twrp CLI flag wins over the env var if both
# are set (the CLI flag is more explicit).
if [ "${TWOYI_TWRP:-0}" = "1" ] || [ "${TWOYI_TWRP:-}" = "true" ]; then
    TWRP_MODE=1
fi

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
echo "  twrp_mode:       $TWRP_MODE"
echo "  twrp_img:        $TWRP_IMG_DEFAULT"
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

# Uninstall any previous version first to clear ALL app data (including
# the cached rootfs). This is CRITICAL: previous runs may have modified
# the rootfs (e.g., the bionic library copy replaced /system/lib64/
# symlinks with real files from the host). Without a clean uninstall,
# the stale rootfs persists and causes version mismatch crashes
# (linker SIGSEGV at 0x86).
echo "  Uninstalling previous version (clear all app data)..."
"$ADB_BIN" -s emulator-5554 uninstall io.twoyi 2>/dev/null || true

# -t : allow test packages (twoyi isn't debuggable by default, but -t
#      lets us install over a debug-signed variant if needed)
# -g : grant all runtime permissions automatically
# (no -r: we already uninstalled, so this is a fresh install)
if ! "$ADB_BIN" -s emulator-5554 install -t -g "$APK"; then
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

if [ "$TWRP_MODE" = "1" ]; then
    # -----------------------------------------------------------------------
    # TWRP recovery ramdisk extraction.
    #
    # We use scripts/extract-twrp-ramdisk.py to:
    #   1. Parse the Android boot image header (magic 'ANDROID!',
    #      page_size 2048, kernel_size 7.4MB, ramdisk_size 7.4MB).
    #   2. Read the gzip-compressed ramdisk at offset 7,473,152.
    #   3. Decompress it (gzip → 20.4 MB cpio).
    #   4. Parse the SVR4 cpio archive (magic 070701) and emit a tar.
    #
    # The tar is what gets pushed into twoyi's data dir (same path as the
    # Android rootfs.tar from the emulator/sdk_image/cyanmint branches).
    #
    # The TWRP ramdisk contains:
    #   - /init             statically linked i386 init binary (578 KB)
    #   - /sbin/recovery    dynamically linked i386 (interp /sbin/linker)
    #   - /sbin/linker      32-bit Android dynamic linker
    #   - 3,107 total entries (66 dirs, 2,797 files, 244 symlinks)
    #   - init.rc with the very simple TWRP boot script
    #     (start ueventd, mount tmpfs on /tmp, class_start default+core,
    #      service recovery /sbin/recovery)
    #
    # kr64 launches with --boot-recovery, which:
    #   - skips LD_PRELOAD (init is statically linked, no hooks needed)
    #   - skips /apex bind mount (TWRP doesn't use APEX packages)
    #   - skips binderfs mount (TWRP doesn't use binder)
    #   - skips SELinux permissive watchdog (TWRP handles SELinux in init.rc)
    #   - skips /dev/twoyi-bin/ copy (TWRP only needs /sbin/*)
    #   - auto-sets init_path=/init (TWRP's init is at the root, not
    #     /system/bin/init)
    # -----------------------------------------------------------------------
    echo "  → TWRP mode: extracting ramdisk from $TWRP_IMG_DEFAULT"
    if [ ! -f "$TWRP_IMG_DEFAULT" ]; then
        echo "  ✗ TWRP boot image not found at $TWRP_IMG_DEFAULT" >&2
        echo "    (override with --twrp-img <path>)" >&2
        exit 1
    fi
    if [ ! -x scripts/extract-twrp-ramdisk.py ]; then
        echo "  ✗ scripts/extract-twrp-ramdisk.py not found or not executable" >&2
        exit 1
    fi
    # python3 is available on ubuntu-latest runners + devcontainers.
    # The extractor outputs the tar to --output-tar; we then push that
    # to the device in Step 4.
    if ! python3 scripts/extract-twrp-ramdisk.py \
            --boot-img "$TWRP_IMG_DEFAULT" \
            --output-tar "$ROOTFS_TAR" \
            2>&1 | tee -a "$ARTIFACT_DIR/rootfs-extract.log"; then
        echo "  ✗ TWRP ramdisk extraction failed" >&2
        exit 1
    fi
    if [ ! -s "$ROOTFS_TAR" ]; then
        echo "  ✗ TWRP extraction produced empty $ROOTFS_TAR" >&2
        exit 1
    fi
    echo "  ✓ TWRP rootfs.tar: $(stat -c%s "$ROOTFS_TAR") bytes"
    echo "    twrp_mode=1" >> "$ARTIFACT_DIR/boot-verdict.txt"
else
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
fi  # end of TWRP_MODE branch
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
#
# TWRP MODE: skip this entire block. TWRP's /init is already a real
# statically-linked ELF binary at the root of the ramdisk (NOT a
# symlink to /system/bin/init), so the symlink dance is a no-op at
# best and actively harmful at worst (it would copy an Android init
# binary over TWRP's init).
if [ "$TWRP_MODE" = "1" ]; then
    echo "  → TWRP mode: skipping /init symlink dance (TWRP init is real)"
    # Verify TWRP's /init exists and is a regular file (not a symlink)
    "$ADB_BIN" -s emulator-5554 shell "
        if [ -f $TWOYI_PROFILE/init ] && [ ! -L $TWOYI_PROFILE/init ]; then
            ls -la $TWOYI_PROFILE/init
            echo '✓ TWRP init is a regular file'
        else
            echo '✗ TWRP init is MISSING or is a symlink — TWRP ramdisk extraction may have failed'
        fi
    " 2>&1 | tee -a "$ARTIFACT_DIR/rootfs-extract.log" || true
else
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
fi  # end of TWRP_MODE init-fix branch

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
#
# TWRP MODE: skip this push. TWRP's init is statically linked, so
# kr64's --boot-recovery mode doesn't read or write libgetpid_hook.so
# at all. Pushing it would just leave a stale file in the rootfs.
if [ "$TWRP_MODE" = "1" ]; then
    echo "  → TWRP mode: skipping libgetpid_hook.so push (no LD_PRELOAD)"
else
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
fi  # end of TWRP_MODE libgetpid_hook push branch

# --- Push libtwoyi_loader_shlib.so (seccomp/SIGSYS virtualization) ---
# This is the REAL virtualization library. Loaded via LD_PRELOAD.
#
# In NON-TWRP mode only: provides seccomp/SIGSYS virtualization + binder
# ioctl hooks + path translation for the full Android boot. The guest
# init's bionic linker loads it via LD_PRELOAD=/dev/libtwoyi_loader_shlib.so.
#
# TWRP MODE: skip this push. TWRP's recovery binary is i386 (32-bit x86)
# and its 32-bit bionic linker CANNOT load the 64-bit libtwoyi_loader_shlib.so
# ("CANNOT LINK EXECUTABLE: ... is 64-bit instead of 32-bit"). Instead, we
# push the i686 twrp_fb_hook.so below, which is the architecturally correct
# 32-bit hook library for the i386 recovery. Task ID 17 incorrectly pushed
# the x86_64 libtwoyi_loader_shlib.so in TWRP mode and set LD_PRELOAD to
# it; the linker aborted recovery on the arch mismatch, making recovery
# invisible in `ps` (Task ID 18 / KVM run 31536016997).
if [ "$TWRP_MODE" = "1" ]; then
    echo "  → TWRP mode: skipping libtwoyi_loader_shlib.so push (recovery is i386; x86_64 loader cannot be loaded by the 32-bit bionic linker)"
else
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
        echo "  ⚠ libtwoyi_loader_shlib.so not found in APK — virtualization disabled"
    fi
else
    echo "  ⚠ EXTRACT_DIR not populated — libtwoyi_loader_shlib.so not available"
fi
fi  # end of TWRP_MODE libtwoyi_loader_shlib push branch

# --- Push twrp_fb_hook.so (TWRP mode only — i686 FB ioctl hook) ---
# This is the i686 (32-bit x86) LD_PRELOAD library for TWRP framebuffer
# virtualization. TWRP's recovery binary is i386 and loads libminuitwrp.so
# which crashes at offset 0x57d7 (NULL deref after FBIOGET_VSCREENINFO
# returns ENOTTY on the /dev/graphics/fb0 regular-file stub).
#
# The i686 hook intercepts FB ioctls and returns valid 720x1280@32bpp
# screen info, fixing the segfault. The .so is placed in jniLibs/x86_64/
# (despite being i686) so the APK packaging includes it; PackageManager
# doesn't validate per-file ELF architecture. The KVM test extracts it
# via `unzip` and pushes it to the device rootfs where kr64 reads it.
#
# Task ID 18 (KVM run 31536016997): reverted from the x86_64
# libtwoyi_loader_shlib.so (which the i386 recovery's 32-bit bionic linker
# cannot load) back to the i686 twrp_fb_hook.so — the architecturally
# correct choice for the i386 recovery binary.
if [ "$TWRP_MODE" = "1" ]; then
    if [ -f "$EXTRACT_DIR/lib/x86_64/twrp_fb_hook.so" ]; then
        "$ADB_BIN" -s emulator-5554 push "$EXTRACT_DIR/lib/x86_64/twrp_fb_hook.so" "$TWOYI_PROFILE/twrp_fb_hook.so" 2>&1 | tail -2
        "$ADB_BIN" -s emulator-5554 shell "chmod 644 $TWOYI_PROFILE/twrp_fb_hook.so" 2>&1
        echo "  ✓ pushed twrp_fb_hook.so to rootfs"
    else
        # Try extracting from the APK directly.
        APK_ABS4=$(readlink -f "$APK_PATH" 2>/dev/null || echo "$APK_PATH")
        unzip -o "$APK_ABS4" "lib/x86_64/twrp_fb_hook.so" -d "$EXTRACT_DIR" 2>/dev/null || true
        if [ -f "$EXTRACT_DIR/lib/x86_64/twrp_fb_hook.so" ]; then
            "$ADB_BIN" -s emulator-5554 push "$EXTRACT_DIR/lib/x86_64/twrp_fb_hook.so" "$TWOYI_PROFILE/twrp_fb_hook.so" 2>&1 | tail -2
            "$ADB_BIN" -s emulator-5554 shell "chmod 644 $TWOYI_PROFILE/twrp_fb_hook.so" 2>&1
            echo "  ✓ pushed twrp_fb_hook.so to rootfs (extracted separately)"
        else
            echo "  ⚠ twrp_fb_hook.so not found in APK — TWRP framebuffer virtualization disabled (recovery will crash in libminuitwrp.so)"
        fi
    fi
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
#
# TWRP MODE: pass --boot-recovery to kr64. This:
#   - loads twrp_fb_hook.so (i686) and sets
#     LD_PRELOAD=/sbin/twrp_fb_hook.so (the statically-linked
#     init ignores LD_PRELOAD, but the dynamically-linked i386 recovery
#     binary loads it → FB ioctls are intercepted → no libminuitwrp crash).
#     The i686 hook is required because the 32-bit bionic linker in TWRP's
#     i386 recovery process CANNOT load the 64-bit libtwoyi_loader_shlib.so
#     ("CANNOT LINK EXECUTABLE: ... is 64-bit instead of 32-bit"). See
#     Task ID 18 / KVM run 31536016997 for the regression Task ID 17 caused.
#   - skips the x86_64 libgetpid_hook.so (init is statically linked,
#     doesn't need PID 1 faking; recovery doesn't need it either)
#   - skips the x86_64 libtwoyi_loader_shlib.so (recovery is i386;
#     the 32-bit bionic linker cannot load a 64-bit library)
#   - skips /apex bind mount (TWRP doesn't use APEX packages)
#   - skips binderfs mount (TWRP doesn't use binder)
#   - skips SELinux permissive watchdog (TWRP handles SELinux in init.rc)
#   - skips /dev/twoyi-bin/ copy (TWRP only needs /sbin/*)
#   - auto-sets init_path=/init (TWRP's init is at the root, not
#     /system/bin/init)
# The TWOYI_INIT_PATH env var is IGNORED in TWRP mode (--boot-recovery
# auto-sets /init, and an explicit --init would override the auto-set
# but we don't want that in TWRP mode unless the user explicitly asks).
SKIP_PRELOAD_ENV="${TWOYI_SKIP_PRELOAD:-}"
INIT_PATH_OVERRIDE="${TWOYI_INIT_PATH:-}"
NO_NAMESPACES_ENV="${TWOYI_NO_NAMESPACES:-}"
LD_DEBUG_ENV="${TWOYI_LD_DEBUG:-2}"
if [ "$TWRP_MODE" = "1" ]; then
    echo "  → pre-launching kr64 as root (TWRP mode: --boot-recovery, LD_PRELOAD=libtwoyi_loader_shlib.so for recovery FB ioctl hook)"
elif [ -n "$NO_NAMESPACES_ENV" ]; then
    echo "  → pre-launching kr64 as root (TWOYI_NO_NAMESPACES=1, no pivot_root/chroot)"
elif [ -n "$SKIP_PRELOAD_ENV" ]; then
    echo "  → pre-launching kr64 as root (TWOYI_SKIP_PRELOAD=1, no LD_PRELOAD)"
elif [ -n "$INIT_PATH_OVERRIDE" ]; then
    echo "  → pre-launching kr64 as root (TWOYI_INIT_PATH=$INIT_PATH_OVERRIDE)"
else
    echo "  → pre-launching kr64 as root (with namespaces, no seccomp)"
fi
# TWRP mode flag for the kr64 command line (empty in non-TWRP mode).
TWRP_FLAG=""
if [ "$TWRP_MODE" = "1" ]; then
    TWRP_FLAG="--boot-recovery"
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
        ${TWRP_FLAG} \
        ${INIT_PATH_OVERRIDE:+--init $INIT_PATH_OVERRIDE} \
        > /data/user/0/io.twoyi/kr64-stderr.log 2>&1 &
    echo \$! > /data/local/tmp/kr64.pid
    echo 'kr64 launched'
" 2>&1 | tee -a "$ARTIFACT_DIR/rootfs-extract.log" | tail -5

# Give kr64 2 seconds to start (or crash) before checking for qemu_pipe
sleep 2

# ALWAYS pull kr64-stderr.log right after launch — even if kr64 crashed
# immediately, this captures the crash output for diagnosis.
echo "  → pulling kr64-stderr.log (early capture)..."
"$ADB_BIN" -s emulator-5554 pull /data/user/0/io.twoyi/kr64-stderr.log "$ARTIFACT_DIR/kr64-stderr.log" 2>/dev/null || true
if [ -f "$ARTIFACT_DIR/kr64-stderr.log" ] && [ -s "$ARTIFACT_DIR/kr64-stderr.log" ]; then
    echo "  ✓ kr64-stderr.log captured ($(stat -c%s "$ARTIFACT_DIR/kr64-stderr.log") bytes)"
    echo "  === kr64-stderr.log (first 30 lines) ==="
    head -30 "$ARTIFACT_DIR/kr64-stderr.log"
fi

# Wait for /dev/qemu_pipe to be created (kr64 is setting up)
echo "  → waiting for kr64 to create /dev/qemu_pipe..."
for i in $(seq 1 15); do
    if "$ADB_BIN" -s emulator-5554 shell "test -S $TWOYI_PROFILE/dev/qemu_pipe" 2>/dev/null; then
        echo "  ✓ /dev/qemu_pipe created (after ${i}s)"
        break
    fi
    sleep 1
done

# TWRP MODE: attach strace to the guest init to capture KLOG writes.
# TWRP init writes its log messages to /dev/__kmsg__ (a char device that
# writes to the kernel kmsg ring buffer). Those messages are mixed with
# the host's init messages in dmesg, making them impossible to find.
# strace captures the write() calls directly, showing us exactly what
# TWRP init is logging.
if [ "$TWRP_MODE" = "1" ]; then
    # Pull kr64-stderr.log to get the guest PID
    "$ADB_BIN" -s emulator-5554 pull /data/user/0/io.twoyi/kr64-stderr.log "$ARTIFACT_DIR/kr64-stderr.log" 2>/dev/null || true
    GUEST_PID=$(grep -oE 'guest pid = [0-9]+' "$ARTIFACT_DIR/kr64-stderr.log" 2>/dev/null | tail -1 | awk '{print $NF}')
    if [ -n "$GUEST_PID" ]; then
        echo "  → TWRP: attaching strace to guest init (pid $GUEST_PID)..."
        # Start strace in background, capture write + execve calls.
        #
        # IMPORTANT (Task ID 25, KVM run 31559261755): the previous
        # version of this block used `strace ... 2>/dev/null &` — only
        # strace's stderr was redirected. strace's stdout was still
        # attached to the adb shell pipe, and adb shell waits for ALL
        # child processes to close their stdout before returning. Since
        # strace runs forever (until the traced init exits), adb shell
        # never returned — the script hung for 8m48s until the 600s
        # workflow timeout killed it. NONE of the downstream diagnostic
        # capture (twrp-ps-pre-kill.log, twrp-guest-tree.log, dmesg.log,
        # twrp-strace.log itself) ran.
        #
        # Fix: redirect ALL THREE std streams (stdin, stdout, stderr)
        # away from the adb pipe. `nohup` makes strace survive adb
        # shell's SIGHUP. `timeout 10` on the adb call is a safety net
        # in case the redirection ever regresses. The strace child is
        # backgrounded with `&`, so adb shell can return immediately.
        timeout 10 "$ADB_BIN" -s emulator-5554 shell "
            nohup strace -e trace=write,execve -f -p $GUEST_PID -o /data/local/tmp/twrp-strace.log </dev/null >/dev/null 2>&1 &
            echo \$! > /data/local/tmp/strace.pid
            echo 'strace attached'
        " 2>&1 | tail -3
        sleep 2
        # Check if strace is running. We verify by checking that the
        # PID in strace.pid is still alive (kill -0) — this catches
        # both "strace not installed" and "strace exited immediately
        # (e.g. permission denied, PID not found)".
        STRACE_PID=$("$ADB_BIN" -s emulator-5554 shell "cat /data/local/tmp/strace.pid 2>/dev/null" 2>/dev/null | tr -d '\r\n ')
        if [ -n "$STRACE_PID" ]; then
            STRACE_ALIVE=$("$ADB_BIN" -s emulator-5554 shell "kill -0 $STRACE_PID 2>/dev/null && echo yes || echo no" 2>/dev/null | tr -d '\r\n ')
            if [ "$STRACE_ALIVE" = "yes" ]; then
                echo "  ✓ strace running (pid $STRACE_PID)"
            else
                echo "  ⚠ strace pid $STRACE_PID is not alive — strace may have failed to attach"
                echo "    (check if strace is installed: adb shell which strace)"
            fi
        else
            echo "  ⚠ strace failed to start — strace may not be available on this device"
            echo "    (run 'adb shell which strace' to verify)"
        fi
    fi
fi
if ! "$ADB_BIN" -s emulator-5554 shell "test -S $TWOYI_PROFILE/dev/qemu_pipe" 2>/dev/null; then
    echo "  ⚠ /dev/qemu_pipe not created after 15s — kr64 may have failed"
    echo "  → pulling kr64-stderr.log for diagnosis"
    "$ADB_BIN" -s emulator-5554 pull /data/user/0/io.twoyi/kr64-stderr.log "$ARTIFACT_DIR/kr64-stderr.log" 2>/dev/null || true
    if [ -f "$ARTIFACT_DIR/kr64-stderr.log" ]; then
        echo "  === kr64-stderr.log ==="
        cat "$ARTIFACT_DIR/kr64-stderr.log"
    fi
fi

# IMPORTANT (Task ID 20, KVM run 31550762716): the previous version of
# this block had NO timeouts on `am start`, `screencap`, `dumpsys`, or
# `input tap`. When the TWRP container's guest init started running, it
# made the host emulator's system server / SurfaceFlinger unresponsive,
# causing `screencap` (line 870) to hang indefinitely. The script was
# killed by the 480s workflow timeout before reaching the TWRP
# diagnostic capture block (Step A at line ~957) — so NONE of the
# twrp-*.log files were captured. This fix wraps every adb command in
# `timeout` + `|| true` so a hang can't block the diagnostic capture.
echo "  → launching io.twoyi/.ui.SettingsActivity (to trigger rootfs detection)"
timeout 30 "$ADB_BIN" -s emulator-5554 shell am start -n io.twoyi/.ui.SettingsActivity \
    2>/dev/null || echo "  ⚠ am start SettingsActivity timed out or failed"
sleep 3

# Take the pre-launch screenshot so we can see what the settings list
# looks like (and confirm the app actually opened).
timeout 15 "$ADB_BIN" -s emulator-5554 exec-out screencap -p \
    > "$ARTIFACT_DIR/screenshot-00_settings.png" 2>/dev/null || true
echo "  ✓ screenshot-00_settings.png ($(stat -c%s "$ARTIFACT_DIR/screenshot-00_settings.png" 2>/dev/null || echo 0) bytes)"

# Give SettingsActivity time to initialize before launching Render2Activity
sleep 5

# Launch the container DIRECTLY via Render2Activity instead of tapping
# the settings preference. Tapping at fixed coordinates is fragile
# (different screen densities, layout changes). Render2Activity is the
# activity that SettingsActivity's "Launch Container" preference starts,
# so launching it directly is equivalent and 100% reliable.
echo "  → launching io.twoyi/.Render2Activity (direct container launch)"
timeout 30 "$ADB_BIN" -s emulator-5554 shell am start -n io.twoyi/.Render2Activity \
    2>/dev/null || echo "  ⚠ am start Render2Activity timed out or failed"
sleep 2

# Verify Render2Activity is in the foreground. If it crashed back to
# settings or the home screen, log it but continue (screenshots will
# show what happened).
CURRENT_FOCUS=$(timeout 15 "$ADB_BIN" -s emulator-5554 shell dumpsys activity activities 2>/dev/null \
    | grep -E 'mResumedActivity|topResumedActivity' | head -1 || true)
echo "  current focus: $CURRENT_FOCUS"
if echo "$CURRENT_FOCUS" | grep -q "Render2Activity"; then
    echo "  ✓ Render2Activity is foreground"
elif echo "$CURRENT_FOCUS" | grep -q "SettingsActivity"; then
    echo "  ⚠ Render2Activity fell back to SettingsActivity (crash?)"
    # Fallback: try the tap approach in case Render2Activity needs the
    # preference click path to set up state first.
    echo "  → fallback: tap 'Launch Container' (heuristic coords 540, 700)"
    timeout 10 "$ADB_BIN" -s emulator-5554 shell input tap 540 700 \
        2>/dev/null || true
    sleep 2
else
    echo "  ⚠ unexpected focus — container may have crashed (or dumpsys timed out)"
fi

# ── EARLY TWRP diagnostic snapshot (Task ID 20) ──
# In TWRP mode, capture /twrp-init.log + /twrp-kmsg.log + kr64-stderr.log
# RIGHT NOW (before the boot_wait / screenshot sequence). This gives us a
# snapshot of the TWRP init's output from the first ~10 seconds — even if
# the later screenshot loop or Step A capture hangs due to system load.
# The files are on ext4 (not tmpfs), so they survive even if kr64 dies.
# The FULL diagnostic capture (Step A at line ~957) still runs later for
# the post-boot-wait state.
if [ "$TWRP_MODE" = "1" ]; then
    echo "  → TWRP early snapshot: pulling /twrp-init.log + /twrp-kmsg.log + kr64-stderr.log..."
    timeout 10 "$ADB_BIN" -s emulator-5554 pull /data/user/0/io.twoyi/kr64-stderr.log \
        "$ARTIFACT_DIR/kr64-stderr-early.log" 2>/dev/null || true
    for EARLY_LOG_PATH in \
        "$TWOYI_PROFILE/twrp-init.log" \
        "/data/data/io.twoyi/profiles/default/rootfs/twrp-init.log" \
        "/data/user/0/io.twoyi/rootfs/twrp-init.log"; do
        timeout 10 "$ADB_BIN" -s emulator-5554 pull "$EARLY_LOG_PATH" \
            "$ARTIFACT_DIR/twrp-init-early.log" 2>/dev/null && break
    done
    for EARLY_KMSG_PATH in \
        "$TWOYI_PROFILE/twrp-kmsg.log" \
        "/data/data/io.twoyi/profiles/default/rootfs/twrp-kmsg.log" \
        "/data/user/0/io.twoyi/rootfs/twrp-kmsg.log"; do
        timeout 10 "$ADB_BIN" -s emulator-5554 pull "$EARLY_KMSG_PATH" \
            "$ARTIFACT_DIR/twrp-kmsg-early.log" 2>/dev/null && break
    done
    echo "    kr64-stderr-early.log: $(stat -c%s "$ARTIFACT_DIR/kr64-stderr-early.log" 2>/dev/null || echo 0) bytes"
    echo "    twrp-init-early.log:   $(stat -c%s "$ARTIFACT_DIR/twrp-init-early.log" 2>/dev/null || echo 0) bytes"
    echo "    twrp-kmsg-early.log:   $(stat -c%s "$ARTIFACT_DIR/twrp-kmsg-early.log" 2>/dev/null || echo 0) bytes"
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
    timeout 10 "$ADB_BIN" -s emulator-5554 exec-out screencap -p \
        > "$ARTIFACT_DIR/screenshot-${i}s.png" 2>/dev/null || true
    SIZE=$(stat -c%s "$ARTIFACT_DIR/screenshot-${i}s.png" 2>/dev/null || echo 0)
    echo "  ✓ screenshot-${i}s.png (${SIZE} bytes)"
    PREV=$i
done

# If boot_wait > 60, sleep the remainder and take a final screenshot.
if [ "$BOOT_WAIT_SECONDS" -gt 60 ]; then
    REMAINDER=$((BOOT_WAIT_SECONDS - 60))
    sleep "$REMAINDER"
    timeout 10 "$ADB_BIN" -s emulator-5554 exec-out screencap -p \
        > "$ARTIFACT_DIR/screenshot-${BOOT_WAIT_SECONDS}s.png" 2>/dev/null || true
    echo "  ✓ screenshot-${BOOT_WAIT_SECONDS}s.png"
fi

# TWRP MODE: kill kr64 after the boot wait so the script can continue
# to capture logs. In TWRP mode, kr64 + the guest init run indefinitely
# (TWRP's recovery service never exits), which would cause the script
# to hang forever waiting for kr64 to exit.
#
# We send SIGTERM (not SIGKILL) so kr64's SIGTERM handler can do a
# graceful shutdown: it does a final waitpid on the guest init and logs
# the exit status (crash, exit, or still-running-then-SIGKILLed). This
# is critical — without it we can't tell whether init crashed, exited,
# or was still running.
#
# ────────────────────────────────────────────────────────────────────
# IMPORTANT: we capture the TWRP container's processes + framebuffer
# BEFORE killing kr64. The previous logic ran `ps -A` AFTER kill, which
# captured the HOST emulator's processes (init, surfaceflinger, etc.)
# and not the TWRP container's processes — because the guest init was
# already SIGKILLed.
#
# The TWRP guest init is NOT PID 1 (kr64's unshare(CLONE_NEWPID) fails
# with EINVAL on the KVM runner). kr64 forks the guest init, the guest
# init gets a normal PID (logged by kr64 as "guest pid = NNNN" in
# kr64-stderr.log). So `ps -A` shows BOTH the host's init (PID 1) AND
# the TWRP init (some other PID). We need to find the guest PID and
# walk its process tree to see what services it spawned (ueventd,
# recovery, etc.).
# ────────────────────────────────────────────────────────────────────
if [ "$TWRP_MODE" = "1" ]; then
    # ── Step A: capture TWRP container's processes BEFORE killing kr64 ──
    # Pull the latest kr64-stderr.log so we can parse the guest PID.
    # The earlier pull (right after launch) may be stale if kr64 logged
    # the guest pid later in its lifecycle.
    "$ADB_BIN" -s emulator-5554 pull /data/user/0/io.twoyi/kr64-stderr.log "$ARTIFACT_DIR/kr64-stderr.log" 2>/dev/null || true

    GUEST_PID=$(grep -oE 'guest pid = [0-9]+' "$ARTIFACT_DIR/kr64-stderr.log" 2>/dev/null | tail -1 | awk '{print $NF}')
    if [ -n "$GUEST_PID" ]; then
        echo "  → TWRP guest init PID: $GUEST_PID (from kr64-stderr.log)"
        echo "  → capturing TWRP process tree BEFORE kill (so we see live processes)..."

        # Capture full process list with parent PIDs — needed for tree walking.
        # `ps -A -o PID,PPID,STAT,NAME` is the portable syntax; some Android
        # toolboxes support `ps -ef` too, so we capture both for redundancy.
        timeout 5 "$ADB_BIN" -s emulator-5554 shell "ps -A -o PID,PPID,STAT,NAME 2>/dev/null" \
            > "$ARTIFACT_DIR/twrp-ps-pre-kill.log" 2>/dev/null || true
        timeout 5 "$ADB_BIN" -s emulator-5554 shell "ps -ef 2>/dev/null" \
            > "$ARTIFACT_DIR/twrp-ps-ef.log" 2>/dev/null || true

        # Verify the guest init is still alive + capture its identity.
        # /proc/<pid>/cmdline is NUL-separated; we convert to spaces.
        timeout 5 "$ADB_BIN" -s emulator-5554 shell "cat /proc/$GUEST_PID/cmdline 2>/dev/null | tr '\0' ' '; echo" \
            > "$ARTIFACT_DIR/twrp-init-cmdline.log" 2>/dev/null || true
        timeout 5 "$ADB_BIN" -s emulator-5554 shell "cat /proc/$GUEST_PID/status 2>/dev/null" \
            > "$ARTIFACT_DIR/twrp-init-status.log" 2>/dev/null || true
        timeout 5 "$ADB_BIN" -s emulator-5554 shell "ls /proc/$GUEST_PID/task 2>/dev/null" \
            > "$ARTIFACT_DIR/twrp-init-threads.log" 2>/dev/null || true

        echo "    guest init cmdline: $(tr -d '\r\n' < "$ARTIFACT_DIR/twrp-init-cmdline.log" 2>/dev/null)"
        echo "    guest init state:   $(grep '^State:' "$ARTIFACT_DIR/twrp-init-status.log" 2>/dev/null | tr -d '\r\n')"
        echo "    guest init threads: $(tr '\r\n' ' ' < "$ARTIFACT_DIR/twrp-init-threads.log" 2>/dev/null)"

        # Verify /dev/kmsg AND /dev/__kmsg__ are the symlinks we expect.
        # kr64 creates BOTH as symlinks → /twrp-kmsg.log so TWRP init's
        # KLOG writes go to a retrievable file instead of the host's
        # flooded dmesg ring buffer.
        #
        # IMPORTANT (Task ID 21): TWRP init (AOSP 5.1-based) uses
        # /dev/__kmsg__ (NOT /dev/kmsg) for its log_init(). Without the
        # /dev/__kmsg__ symlink, twrp-kmsg.log will be EMPTY — confirmed
        # in KVM run 31552072308 where fd 3 -> /dev/__kmsg__ (deleted)
        # but twrp-kmsg.log had 0 bytes.
        KR64_PID_FOR_KMSG=$(awk '/^PPid:/ {print $2}' "$ARTIFACT_DIR/twrp-init-status.log" 2>/dev/null | tr -d '\r\n')
        if [ -n "$KR64_PID_FOR_KMSG" ]; then
            timeout 5 "$ADB_BIN" -s emulator-5554 shell "ls -la /proc/$KR64_PID_FOR_KMSG/root/dev/kmsg 2>/dev/null; ls -la /proc/$KR64_PID_FOR_KMSG/root/dev/__kmsg__ 2>/dev/null; ls -la /proc/$KR64_PID_FOR_KMSG/root/twrp-kmsg.log 2>/dev/null; wc -c /proc/$KR64_PID_FOR_KMSG/root/twrp-kmsg.log 2>/dev/null" \
                > "$ARTIFACT_DIR/twrp-kmsg-symlink-check.log" 2>/dev/null || true
            echo "    /dev/kmsg + /dev/__kmsg__ + /twrp-kmsg.log status (in kr64 mount ns):"
            sed 's/^/      /' "$ARTIFACT_DIR/twrp-kmsg-symlink-check.log" 2>/dev/null | head -8
        fi

        # Also dump the TWRP init's open file descriptors — this shows
        # whether init has /dev/kmsg open (and whether it's a symlink to
        # /twrp-kmsg.log as expected). If /dev/kmsg is NOT in init's fd
        # table, init never opened it (and we won't see KLOG output).
        timeout 5 "$ADB_BIN" -s emulator-5554 shell "ls -la /proc/$GUEST_PID/fd 2>/dev/null" \
            > "$ARTIFACT_DIR/twrp-init-fds.log" 2>/dev/null || true
        if [ -s "$ARTIFACT_DIR/twrp-init-fds.log" ]; then
            KMSG_FD=$(grep -E 'kmsg|/twrp-kmsg' "$ARTIFACT_DIR/twrp-init-fds.log" 2>/dev/null | head -3 || true)
            if [ -n "$KMSG_FD" ]; then
                echo "    init has /dev/kmsg open (KLOG should be captured):"
                echo "$KMSG_FD" | sed 's/^/      /'
            else
                echo "    ⚠ init does NOT have /dev/kmsg open — KLOG messages are being dropped"
                echo "      (init's open fds, first 10:)"
                head -10 "$ARTIFACT_DIR/twrp-init-fds.log" | sed 's/^/        /'
            fi
        fi

        # Build the guest's process tree: all processes whose PPID chain
        # leads back to $GUEST_PID. We awk through twrp-ps-pre-kill.log
        # (PID,PPID,STAT,NAME format), record each pid's ppid, then for
        # each pid walk up the parent chain looking for $GUEST_PID.
        if [ -s "$ARTIFACT_DIR/twrp-ps-pre-kill.log" ]; then
            awk -v root="$GUEST_PID" '
                NR == 1 { next }     # skip header
                NF >= 4 {
                    ppid[$1] = $2
                    stat[$1] = $3
                    # NAME may contain spaces (rare), so join fields 4..NF
                    name[$1] = ""
                    for (i = 4; i <= NF; i++) name[$1] = (name[$1] " " $i)
                    sub(/^ /, "", name[$1])
                }
                END {
                    # Print the root (guest init) first.
                    if (ppid[root] != "" || name[root] != "") {
                        printf "  root  PID=%s PPID=%s STAT=%s NAME=%s\n", root, ppid[root], stat[root], name[root]
                    }
                    # Walk each process parent chain up to 20 hops deep.
                    for (pid in ppid) {
                        if (pid == root) continue
                        cur = ppid[pid]
                        depth = 0
                        while (cur != "" && cur != "0" && cur != "1" && cur != "2" && depth < 20) {
                            if (cur == root) {
                                printf "  child PID=%s PPID=%s STAT=%s NAME=%s\n", pid, ppid[pid], stat[pid], name[pid]
                                break
                            }
                            cur = ppid[cur]
                            depth++
                        }
                    }
                }
            ' "$ARTIFACT_DIR/twrp-ps-pre-kill.log" > "$ARTIFACT_DIR/twrp-guest-tree.log" 2>/dev/null
        fi

        if [ -s "$ARTIFACT_DIR/twrp-guest-tree.log" ]; then
            TREE_LINES=$(wc -l < "$ARTIFACT_DIR/twrp-guest-tree.log")
            echo "  ✓ TWRP guest process tree ($TREE_LINES entries):"
            sed 's/^/    /' "$ARTIFACT_DIR/twrp-guest-tree.log"
        else
            echo "  ⚠ no TWRP processes found in tree walk — guest init may have exited before capture"
        fi
    else
        echo "  ⚠ could not parse 'guest pid = NNNN' from kr64-stderr.log"
        echo "    → kr64 may have crashed before forking the guest; check kr64-stderr.log"
    fi

    # ── Step B: capture the TWRP virtual framebuffer BEFORE killing kr64 ──
    # TWRP renders to /dev/graphics/fb0 inside the pivot_root jail, which
    # kr64 creates as a REGULAR FILE (3686400 bytes = 720×1280×4 RGBA8888).
    # The file lives on the tmpfs that kr64 mounted on {rootfs}/dev BEFORE
    # pivot_root — so it's INSIDE kr64's mount namespace, NOT visible from
    # the host's normal /data/data/... path (which still shows the original
    # ext4 /dev directory with the /dev/null symlink).
    #
    # To access kr64's mount namespace from the host, we use the magic
    # /proc/<kr64_pid>/root/ path — the kernel exposes any process's
    # rootfs via /proc/<pid>/root/, and that path DOES cross mount
    # namespace boundaries (it follows the link to the process's mount
    # namespace's root). So /proc/<kr64_pid>/root/dev/graphics/fb0 is
    # the FB file inside the pivot_root jail.
    #
    # We MUST do this BEFORE killing kr64 — once kr64 dies, its mount
    # namespace is destroyed and the tmpfs (with the FB file) is unmounted.
    if [ -n "$GUEST_PID" ] || [ "$TWRP_MODE" = "1" ]; then
        echo "  → capturing TWRP virtual framebuffer (RGBA8888 → PNG)..."
        # Find kr64's PID (parent of the guest init) — needed for the
        # /proc/<kr64_pid>/root/ path. Fall back to the kr64.pid file
        # (written by the launch step) if /proc/<guest_pid>/status doesn't
        # give us a PPID.
        KR64_PID_HOST=$(awk '/^PPid:/ {print $2}' "$ARTIFACT_DIR/twrp-init-status.log" 2>/dev/null | tr -d '\r\n')
        if [ -z "$KR64_PID_HOST" ]; then
            KR64_PID_HOST=$("$ADB_BIN" -s emulator-5554 shell "cat /data/local/tmp/kr64.pid 2>/dev/null" 2>/dev/null | tr -d '\r\n' || true)
        fi
        echo "    kr64 host PID: ${KR64_PID_HOST:-unknown}"
        FB_PULLED=0
        # Try in priority order:
        #   1. /proc/<kr64_pid>/root/dev/graphics/fb0 — kr64's mount namespace (preferred)
        #   2. /data/data/io.twoyi/profiles/default/rootfs/dev/graphics/fb0 — ext4 fallback
        #   3. /data/user/0/io.twoyi/rootfs/dev/graphics/fb0 — alternate symlink path
        for FB_PATH in \
            "/proc/$KR64_PID_HOST/root/dev/graphics/fb0" \
            "/proc/$KR64_PID_HOST/root/dev/fb0" \
            "$TWOYI_PROFILE/dev/graphics/fb0" \
            "/data/data/io.twoyi/profiles/default/rootfs/dev/graphics/fb0" \
            "/data/user/0/io.twoyi/rootfs/dev/graphics/fb0"; do
            if [ -z "$FB_PATH" ]; then continue; fi
            if "$ADB_BIN" -s emulator-5554 pull "$FB_PATH" "$ARTIFACT_DIR/twrp-fb-rgba.bin" 2>/dev/null; then
                FB_PULLED=1
                echo "  ✓ pulled $FB_PATH → twrp-fb-rgba.bin ($(stat -c%s "$ARTIFACT_DIR/twrp-fb-rgba.bin" 2>/dev/null || echo 0) bytes)"
                break
            fi
        done
        if [ "$FB_PULLED" = "1" ] && [ -s "$ARTIFACT_DIR/twrp-fb-rgba.bin" ]; then
            # Convert RGBA8888 (4 bytes/pixel, R+G+B+A) to PNG (RGB, 3 bytes/pixel).
            # We avoid the Pillow dependency — pure stdlib zlib + struct.
            # TWRP's fb0 is 720x1280 RGBA8888 (per kr64's create_twrp_framebuffer).
            python3 - <<'PY' "$ARTIFACT_DIR/twrp-fb-rgba.bin" "$ARTIFACT_DIR/twrp-fb.png" 2>&1 | tail -5
import struct, zlib, sys
src, dst = sys.argv[1], sys.argv[2]
with open(src, 'rb') as f:
    data = f.read()
W, H = 720, 1280
expected = W * H * 4
if len(data) < expected:
    print(f"  ⚠ FB file too small: {len(data)} bytes (expected {expected}) — TWRP init may not have written to it")
    # Still produce a PNG so the user can see what (if anything) was written.
    # Pad with zeros so struct.unpack doesn't fail.
    data = data + b'\x00' * (expected - len(data))
# Drop alpha, keep RGB.
rgb = bytearray(W * H * 3)
for i in range(W * H):
    r, g, b = data[i*4], data[i*4+1], data[i*4+2]
    rgb[i*3], rgb[i*3+1], rgb[i*3+2] = r, g, b
# Count non-zero pixels (TWRP UI has buttons + text on a colored bg).
nonzero_px = sum(1 for i in range(W * H) if rgb[i*3] or rgb[i*3+1] or rgb[i*3+2])
def chunk(typ, payload):
    c = typ + payload
    return struct.pack('>I', len(payload)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
png = b'\x89PNG\r\n\x1a\n'
png += chunk(b'IHDR', struct.pack('>IIBBBBB', W, H, 8, 2, 0, 0, 0))  # 8-bit RGB
raw = bytearray()
row_stride = W * 3
for y in range(H):
    raw.append(0)  # filter byte (None)
    raw.extend(rgb[y*row_stride:(y+1)*row_stride])
png += chunk(b'IDAT', zlib.compress(bytes(raw), 9))
png += chunk(b'IEND', b'')
with open(dst, 'wb') as f:
    f.write(png)
print(f"  ✓ twrp-fb.png ({len(png)} bytes) — {nonzero_px}/{W*H} non-zero pixels ({100*nonzero_px//(W*H)}%)")
PY
        else
            echo "  ⚠ could not pull TWRP virtual framebuffer (fb0 not found on device)"
        fi
    fi

    # ── Step C: graceful shutdown of kr64 (existing logic) ──
    echo "  → TWRP mode: sending SIGTERM to kr64 (graceful shutdown)..."
    timeout 8 "$ADB_BIN" -s emulator-5554 shell "kill \$(cat /data/local/tmp/kr64.pid 2>/dev/null) 2>/dev/null" 2>/dev/null || true
    sleep 2
    timeout 5 "$ADB_BIN" -s emulator-5554 shell "kill -9 \$(cat /data/local/tmp/kr64.pid 2>/dev/null) 2>/dev/null; pkill -9 -f '/data/local/tmp/kr64' 2>/dev/null" 2>/dev/null || true
    sleep 1

    # Capture kernel log (dmesg) — the guest shares the host emulator's
    # kernel, so any segfault from TWRP init appears here. TWRP init also
    # tries to log to /dev/kmsg (which kr64 symlinks to /twrp-kmsg.log
    # in TWRP mode); the symlinked messages do NOT appear in dmesg (they
    # go to the file instead), but kernel-level segfaults still do.
    echo "  → capturing dmesg (kernel log)..."
    timeout 10 "$ADB_BIN" -s emulator-5554 shell dmesg > "$ARTIFACT_DIR/dmesg.log" 2>/dev/null || true
    if [ -s "$ARTIFACT_DIR/dmesg.log" ]; then
        echo "  ✓ dmesg.log: $(stat -c%s "$ARTIFACT_DIR/dmesg.log") bytes"
        # Surface any segfault or init-related kernel messages for quick diagnosis.
        echo "  → kernel messages mentioning init/segfault/twrp:"
        grep -iE 'segfault|init\[|twrp|recovery' "$ARTIFACT_DIR/dmesg.log" 2>/dev/null | tail -20 || echo "    (none found)"
    else
        echo "  ⚠ dmesg.log empty or not captured"
    fi

    # Check if any TWRP processes are still alive AFTER kr64 died.
    # kr64's SIGKILL of the guest init (in the handler) should have killed
    # it, but if init forked daemons (ueventd, partlink, recovery) they
    # may survive as orphans reparented to the host's init (PID 1).
    echo "  → checking for surviving TWRP processes (post-kill)..."
    timeout 5 "$ADB_BIN" -s emulator-5554 shell "ps -A 2>/dev/null" > "$ARTIFACT_DIR/twrp-ps-post-kill.log" 2>/dev/null || true
    # Also keep the legacy twrp-ps.log name (back-compat with old tooling).
    cp "$ARTIFACT_DIR/twrp-ps-post-kill.log" "$ARTIFACT_DIR/twrp-ps.log" 2>/dev/null || true
    if [ -s "$ARTIFACT_DIR/twrp-ps-post-kill.log" ]; then
        SURVIVORS=$(grep -iE 'ueventd|partlink|recovery' "$ARTIFACT_DIR/twrp-ps-post-kill.log" 2>/dev/null || true)
        INIT_PROCS=$(awk '/[i]nit/ && $2 != 1' "$ARTIFACT_DIR/twrp-ps-post-kill.log" 2>/dev/null || true)
        if [ -n "$SURVIVORS" ] || [ -n "$INIT_PROCS" ]; then
            echo "  ⚠ TWRP-related processes still alive after kr64 died:"
            [ -n "$SURVIVORS" ] && echo "$SURVIVORS"
            [ -n "$INIT_PROCS" ] && echo "$INIT_PROCS"
        else
            echo "  ✓ no TWRP daemons survived (init was killed or exited)"
        fi
    else
        echo "  ⚠ could not capture ps output"
    fi
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
# WARNING: in TWRP mode, logcat is the HOST emulator's logcat, NOT the
# TWRP container's. The "BOOT_COMPLETED" line in logcat is from the host's
# ActivityManager, not from TWRP init. So in TWRP mode we MUST NOT treat
# this as a container-boot signal — see the TWRP-specific verdict below.
BOOT_COMPLETED=$(grep -c 'BOOT_COMPLETED' "$ARTIFACT_DIR/logcat-filtered.txt" || true)

# ── TWRP-mode milestone extraction ──
# In TWRP mode, the meaningful boot signal is "init: starting service
# 'recovery'" in twrp-kmsg.log (TWRP init's KLOG, captured via the
# /dev/kmsg → /twrp-kmsg.log symlink that kr64 creates in TWRP mode).
# We also check twrp-guest-tree.log (children of guest init PID) to see
# whether the recovery process actually started, and twrp-fb.png's
# non-zero pixel count to see whether TWRP actually rendered a UI.
TWRP_KMSG_EXISTS=0
TWRP_RECOVERY_STARTED=0
TWRP_UEVENTD_STARTED=0
TWRP_RECOVERY_PROC=0
TWRP_FB_NONZERO_PCT=0
TWRP_GUEST_PID_FOUND=0
if [ "$TWRP_MODE" = "1" ]; then
    if [ -f "$ARTIFACT_DIR/twrp-kmsg.log" ] && [ -s "$ARTIFACT_DIR/twrp-kmsg.log" ]; then
        TWRP_KMSG_EXISTS=1
        # TWRP init's KLOG_INFO writes look like "init: starting service 'recovery'"
        # (matching AOSP 5.1.1 init's service-start log format).
        TWRP_RECOVERY_STARTED=$(grep -cE "init: starting service 'recovery'|starting service 'recovery'" "$ARTIFACT_DIR/twrp-kmsg.log" 2>/dev/null || echo 0)
        TWRP_UEVENTD_STARTED=$(grep -cE "init: starting service 'ueventd'|starting service 'ueventd'" "$ARTIFACT_DIR/twrp-kmsg.log" 2>/dev/null || echo 0)
    fi
    if [ -s "$ARTIFACT_DIR/twrp-guest-tree.log" ]; then
        TWRP_GUEST_PID_FOUND=1
        # Look for a "recovery" entry in the guest's process tree.
        if grep -qiE 'NAME=.*recovery' "$ARTIFACT_DIR/twrp-guest-tree.log" 2>/dev/null; then
            TWRP_RECOVERY_PROC=1
        fi
    fi
    # Check the framebuffer non-zero pixel percentage (from the python
    # script's output, captured to FB_INFO). We re-derive it from the
    # bin file size: if it's exactly 3686400 bytes (720*1280*4), the FB
    # was created and may have data; if 0 bytes, TWRP never wrote to it.
    if [ -f "$ARTIFACT_DIR/twrp-fb-rgba.bin" ]; then
        FB_SIZE=$(stat -c%s "$ARTIFACT_DIR/twrp-fb-rgba.bin" 2>/dev/null || echo 0)
        if [ "$FB_SIZE" -ge 3686400 ]; then
            # Count non-zero bytes (cheap heuristic for "did TWRP render anything").
            TWRP_FB_NONZERO_PCT=$(python3 -c "
import sys
with open('$ARTIFACT_DIR/twrp-fb-rgba.bin', 'rb') as f:
    data = f.read(3686400)
nz = sum(1 for b in data if b != 0)
print(100 * nz // len(data) if data else 0)
" 2>/dev/null || echo 0)
        fi
    fi
fi

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
    if [ "$TWRP_MODE" = "1" ]; then
        # TWRP-specific checklist — uses twrp-kmsg.log + twrp-guest-tree.log
        # + twrp-fb.png, NOT the host emulator's logcat.
        echo "  (TWRP mode — logcat milestones below are HOST's, not TWRP's)"
        echo "  KR64 daemon started:           $([ "$KR64_START" -gt 0 ] && echo "✓ ($KR64_START lines)" || echo "✗")"
        echo "  TWRP init KMSG captured:       $([ "$TWRP_KMSG_EXISTS" = "1" ] && echo "✓" || echo "✗")"
        echo "  TWRP ueventd started:          $([ "$TWRP_UEVENTD_STARTED" -gt 0 ] && echo "✓ ($TWRP_UEVENTD_STARTED lines)" || echo "✗")"
        echo "  TWRP 'recovery' svc started:   $([ "$TWRP_RECOVERY_STARTED" -gt 0 ] && echo "✓ ($TWRP_RECOVERY_STARTED lines)" || echo "✗")"
        echo "  recovery proc in guest tree:   $([ "$TWRP_RECOVERY_PROC" = "1" ] && echo "✓" || echo "✗")"
        echo "  guest init PID found:          $([ "$TWRP_GUEST_PID_FOUND" = "1" ] && echo "✓" || echo "✗")"
        echo "  TWRP framebuffer non-zero:     ${TWRP_FB_NONZERO_PCT}% (100% = empty/zero; <100% = rendered)"
        echo "  (host's BOOT_COMPLETED line:   $([ "$BOOT_COMPLETED" -gt 0 ] && echo "present (IGNORED — host's, not TWRP's)" || echo "absent"))"
    else
        echo "  KR64 daemon started:           $([ "$KR64_START" -gt 0 ] && echo "✓ ($KR64_START lines)" || echo "✗")"
        echo "  /dev/qemu_pipe created:        $([ "$QEMU_PIPE_CREATED" -gt 0 ] && echo "✓ ($QEMU_PIPE_CREATED lines)" || echo "✗")"
        echo "  Pipe availability: true:       $([ "$PIPE_AVAIL" -gt 0 ] && echo "✓ ($PIPE_AVAIL lines)" || echo "✗")"
        echo "  Pipe connected:                $([ "$PIPE_CONN" -gt 0 ] && echo "✓ ($PIPE_CONN lines)" || echo "✗")"
        echo "  GL context created:            $([ "$GL_CTX" -gt 0 ] && echo "✓ ($GL_CTX lines)" || echo "✗")"
        echo "  BOOT_COMPLETED signal:         $([ "$BOOT_COMPLETED" -gt 0 ] && echo "✓ ($BOOT_COMPLETED lines)" || echo "✗")"
    fi
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
    if [ "$TWRP_MODE" = "1" ]; then
        # TWRP verdict: did TWRP init start the recovery service AND
        # render something to the framebuffer?
        if [ "$TWRP_RECOVERY_STARTED" -gt 0 ] && [ "$TWRP_RECOVERY_PROC" = "1" ] && [ "$TWRP_FB_NONZERO_PCT" -gt 0 ] && [ "$TWRP_FB_NONZERO_PCT" -lt 100 ]; then
            echo "  ✓✓✓ TWRP BOOTED — recovery service started + framebuffer rendered."
            echo "  Inspect twrp-fb.png to confirm the TWRP menu is visible."
        elif [ "$TWRP_RECOVERY_STARTED" -gt 0 ] && [ "$TWRP_RECOVERY_PROC" = "1" ]; then
            echo "  ◐ PARTIAL — recovery service started but framebuffer is empty."
            echo "  Likely cause: recovery binary crashed before calling fb_update,"
            echo "  OR our LD_PRELOAD FB hook isn't being loaded by the i386 recovery."
            echo "  Inspect twrp-kmsg.log + twrp-guest-tree.log for the next error."
        elif [ "$TWRP_KMSG_EXISTS" = "1" ]; then
            echo "  ◐ PARTIAL — TWRP init ran (KLOG captured) but didn't start recovery."
            echo "  Likely cause: init.rc parse error, missing service binary, or"
            echo "  init crashed before reaching the 'on boot' trigger."
            echo "  Inspect twrp-kmsg.log for the last KLOG_ERROR / KLOG_WARNING line."
        elif [ "$TWRP_GUEST_PID_FOUND" = "1" ]; then
            echo "  ◐ PARTIAL — guest init ran but produced no KLOG output."
            echo "  Likely cause: kr64's /dev/__kmsg__ → /twrp-kmsg.log symlink wasn't"
            echo "  created (older kr64 binary), OR TWRP init crashed before opening /dev/__kmsg__."
            echo "  NOTE: TWRP init (AOSP 5.1) uses /dev/__kmsg__, NOT /dev/kmsg, for KLOG."
            echo "  Inspect kr64-stderr.log for 'created /dev/__kmsg__' + dmesg.log for segfaults."
        else
            echo "  ✗ TWRP init did not run or crashed immediately."
            echo "  Inspect kr64-stderr.log + dmesg.log for the failure."
        fi
    elif [ "$BOOT_COMPLETED" -gt 0 ]; then
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
    if [ "$TWRP_MODE" = "1" ]; then
        echo "  TWRP framebuffer:   $ARTIFACT_DIR/twrp-fb.png (RGBA8888 → PNG)"
        echo "  TWRP FB raw:        $ARTIFACT_DIR/twrp-fb-rgba.bin (3.5 MB)"
        echo "  TWRP KMSG log:      $ARTIFACT_DIR/twrp-kmsg.log (TWRP init's KLOG)"
        echo "  TWRP init log:      $ARTIFACT_DIR/twrp-init.log (stdout/stderr)"
        echo "  TWRP ps pre-kill:   $ARTIFACT_DIR/twrp-ps-pre-kill.log (live container procs)"
        echo "  TWRP ps-ef:         $ARTIFACT_DIR/twrp-ps-ef.log (full process tree)"
        echo "  TWRP guest tree:    $ARTIFACT_DIR/twrp-guest-tree.log (children of guest init)"
        echo "  TWRP init cmdline:  $ARTIFACT_DIR/twrp-init-cmdline.log"
        echo "  TWRP init status:   $ARTIFACT_DIR/twrp-init-status.log"
        echo "  TWRP init threads:  $ARTIFACT_DIR/twrp-init-threads.log"
        echo "  TWRP ps post-kill:  $ARTIFACT_DIR/twrp-ps-post-kill.log (survivors after kill)"
    fi
    echo "  Tombstones:         $ARTIFACT_DIR/tombstones/"
    echo "  Rootfs extract log: $ARTIFACT_DIR/rootfs-extract.log"
    echo "  Emulator stdout:    $ARTIFACT_DIR/emulator-stdout.log"
    echo "  Emulator stderr:    $ARTIFACT_DIR/emulator-stderr.log"
} >> "$ARTIFACT_DIR/boot-verdict.txt"

cat "$ARTIFACT_DIR/boot-verdict.txt"

# Final pull of kr64-stderr.log — always attempt, even if the script
# finished normally. This catches cases where kr64 ran for a while and
# produced output that wasn't captured by the early pull.
echo ""
echo "── Final kr64-stderr.log capture ──"
"$ADB_BIN" -s emulator-5554 pull /data/user/0/io.twoyi/kr64-stderr.log "$ARTIFACT_DIR/kr64-stderr.log" 2>/dev/null || true
if [ -f "$ARTIFACT_DIR/kr64-stderr.log" ] && [ -s "$ARTIFACT_DIR/kr64-stderr.log" ]; then
    echo "  ✓ kr64-stderr.log: $(stat -c%s "$ARTIFACT_DIR/kr64-stderr.log") bytes"
else
    echo "  ⚠ kr64-stderr.log not found or empty — kr64 may have never started"
    # Try alternate path
    "$ADB_BIN" -s emulator-5554 pull /data/data/io.twoyi/kr64-stderr.log "$ARTIFACT_DIR/kr64-stderr.log" 2>/dev/null || true
fi

# TWRP MODE: pull the TWRP init's stdout/stderr log from /twrp-init.log
# (kr64 redirects the guest init's output there in TWRP boot mode).
# The file is at the ROOT of the rootfs (NOT /tmp/ which is tmpfs and
# gets unmounted when kr64 dies).
if [ "$TWRP_MODE" = "1" ]; then
    echo ""
    echo "── TWRP init log capture ──"
    # The init runs inside the pivot_root jail, so /twrp-init.log is
    # at {rootfs}/twrp-init.log from the host's perspective.
    # Try multiple paths — the rootfs might be at different locations.
    for TWRP_LOG_PATH in \
        "$TWOYI_PROFILE/twrp-init.log" \
        "/data/data/io.twoyi/profiles/default/rootfs/twrp-init.log" \
        "/data/user/0/io.twoyi/rootfs/twrp-init.log"; do
        "$ADB_BIN" -s emulator-5554 pull "$TWRP_LOG_PATH" "$ARTIFACT_DIR/twrp-init.log" 2>/dev/null && break
    done
    if [ -f "$ARTIFACT_DIR/twrp-init.log" ] && [ -s "$ARTIFACT_DIR/twrp-init.log" ]; then
        echo "  ✓ twrp-init.log: $(stat -c%s "$ARTIFACT_DIR/twrp-init.log") bytes"
        echo "  === twrp-init.log (first 80 lines) ==="
        head -80 "$ARTIFACT_DIR/twrp-init.log"
    else
        echo "  ⚠ twrp-init.log not found — TWRP init may not have started, or wrote to /dev/kmsg instead"
        # Also try to find it via adb shell find
        echo "  → searching for twrp-init.log on device..."
        timeout 10 "$ADB_BIN" -s emulator-5554 shell "find /data/data/io.twoyi -name 'twrp-init.log' 2>/dev/null" 2>/dev/null || true
    fi

    # Pull /twrp-kmsg.log — TWRP init's KLOG messages (captured via the
    # /dev/kmsg → /twrp-kmsg.log symlink that kr64 creates in TWRP mode).
    # AOSP 5.1.1 init writes ALL its log messages via KLOG_INFO/KLOG_ERROR
    # to /dev/kmsg. Without this capture, we can't see "starting service
    # 'recovery'" or any error messages from TWRP init. The host's dmesg
    # ring buffer is flooded by the outer Android init's subcontext loop,
    # so even if TWRP init wrote to a real /dev/kmsg char device, the
    # messages would be pushed out within ~12s.
    echo ""
    echo "── TWRP init KMSG log capture ──"
    for TWRP_KMSG_PATH in \
        "$TWOYI_PROFILE/twrp-kmsg.log" \
        "/data/data/io.twoyi/profiles/default/rootfs/twrp-kmsg.log" \
        "/data/user/0/io.twoyi/rootfs/twrp-kmsg.log"; do
        "$ADB_BIN" -s emulator-5554 pull "$TWRP_KMSG_PATH" "$ARTIFACT_DIR/twrp-kmsg.log" 2>/dev/null && break
    done
    if [ -f "$ARTIFACT_DIR/twrp-kmsg.log" ] && [ -s "$ARTIFACT_DIR/twrp-kmsg.log" ]; then
        echo "  ✓ twrp-kmsg.log: $(stat -c%s "$ARTIFACT_DIR/twrp-kmsg.log") bytes"
        echo "  === twrp-kmsg.log (first 120 lines) ==="
        head -120 "$ARTIFACT_DIR/twrp-kmsg.log"
        # Surface any recovery-related lines for quick diagnosis.
        echo "  → lines mentioning recovery/service/segfault:"
        grep -iE "recovery|starting service|signal|segfault|execve|exec|setenv" "$ARTIFACT_DIR/twrp-kmsg.log" 2>/dev/null | tail -30 || echo "    (none found)"
    else
        echo "  ⚠ twrp-kmsg.log not found or empty — TWRP init may not have written KLOG messages"
        echo "  → searching for twrp-kmsg.log on device..."
        timeout 10 "$ADB_BIN" -s emulator-5554 shell "find /data/data/io.twoyi -name 'twrp-kmsg.log' 2>/dev/null" 2>/dev/null || true
    fi

    # Pull strace log — captures TWRP init's write() calls (KLOG messages)
    echo ""
    echo "── TWRP strace log capture ──"
    "$ADB_BIN" -s emulator-5554 pull /data/local/tmp/twrp-strace.log "$ARTIFACT_DIR/twrp-strace.log" 2>/dev/null || true
    if [ -f "$ARTIFACT_DIR/twrp-strace.log" ] && [ -s "$ARTIFACT_DIR/twrp-strace.log" ]; then
        echo "  ✓ twrp-strace.log: $(stat -c%s "$ARTIFACT_DIR/twrp-strace.log") bytes"
        echo "  === twrp-strace.log (write calls to fd 3 = kmsg, first 80 lines) ==="
        grep "write(3," "$ARTIFACT_DIR/twrp-strace.log" 2>/dev/null | head -80 || echo "    (no write(3,) calls found)"
        echo ""
        echo "  === execve calls ==="
        grep "execve(" "$ARTIFACT_DIR/twrp-strace.log" 2>/dev/null | head -20 || echo "    (no execve calls found)"
    else
        echo "  ⚠ twrp-strace.log not found — strace may not be available on the device"
    fi
fi

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
