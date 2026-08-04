#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Runs on codespace creation. Sets up the full Android dev toolchain
# (SDK, NDK, cargo-xdk) and verifies whether KVM is available.
#
# This script intentionally does NOT auto-build the APK — that's what
# GitHub Actions is for. The codespace is for: running tests, debugging,
# and driving the redroid test environment.

set -e

echo "================================================================"
echo "  twoyi devcontainer setup"
echo "================================================================"

# ---------------------------------------------------------------------------
# 1. KVM availability check
# ---------------------------------------------------------------------------
# GitHub Codespaces run on Azure VMs that historically do NOT expose
# /dev/kvm to the devcontainer, even with --privileged in runArgs.
# We check definitively here so the test scripts know which path to take.
# ---------------------------------------------------------------------------

bash /workspaces/twoyi/.devcontainer/scripts/check-kvm.sh || true

# ---------------------------------------------------------------------------
# 2. Android SDK command-line tools
# ---------------------------------------------------------------------------
echo ""
echo "── Installing Android SDK command-line tools ──"
ANDROID_SDK_ROOT="${ANDROID_HOME:-/workspaces/twoyi/.android-sdk}"
mkdir -p "$ANDROID_SDK_ROOT/cmdline-tools"
if [ ! -d "$ANDROID_SDK_ROOT/cmdline-tools/latest" ]; then
    cd /tmp
    wget -q https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip -O cmdline-tools.zip
    unzip -q cmdline-tools.zip -d "$ANDROID_SDK_ROOT/cmdline-tools"
    mv "$ANDROID_SDK_ROOT/cmdline-tools/cmdline-tools" "$ANDROID_SDK_ROOT/cmdline-tools/latest"
    rm cmdline-tools.zip
fi
export PATH="$ANDROID_SDK_ROOT/cmdline-tools/latest/bin:$ANDROID_SDK_ROOT/platform-tools:$PATH"

yes | sdkmanager --licenses >/dev/null 2>&1 || true
sdkmanager --update
sdkmanager \
    "platform-tools" \
    "platforms;android-31" \
    "build-tools;30.0.3"

echo "✓ Android SDK installed at $ANDROID_SDK_ROOT"

# ---------------------------------------------------------------------------
# 3. Android NDK r27c (matches CI)
# ---------------------------------------------------------------------------
echo ""
echo "── Installing Android NDK r27c ──"
ANDROID_NDK_ROOT="${ANDROID_NDK_HOME:-/workspaces/twoyi/.android-ndk}"
if [ ! -d "$ANDROID_NDK_ROOT" ]; then
    cd /tmp
    wget -q https://dl.google.com/android/repository/android-ndk-r27c-linux.zip -O ndk.zip
    unzip -q ndk.zip -d /tmp/ndk-extract
    mv /tmp/ndk-extract/android-ndk-r27c "$ANDROID_NDK_ROOT"
    rm ndk.zip
    rm -rf /tmp/ndk-extract
fi
echo "✓ Android NDK installed at $ANDROID_NDK_ROOT"

# ---------------------------------------------------------------------------
# 4. Rust Android targets + cargo-xdk
# ---------------------------------------------------------------------------
echo ""
echo "── Adding Rust Android targets ──"
rustup target add aarch64-linux-android
rustup target add x86_64-linux-android

echo ""
echo "── Installing cargo-xdk ──"
if ! command -v cargo-xdk >/dev/null 2>&1; then
    cargo install cargo-xdk
fi
echo "✓ cargo-xdk installed"

# ---------------------------------------------------------------------------
# 5. Docker (for redroid-based testing)
# ---------------------------------------------------------------------------
echo ""
echo "── Verifying Docker access (for redroid testing) ──"
if docker info >/dev/null 2>&1; then
    echo "✓ Docker is accessible"
else
    echo "⚠ Docker is not accessible. redroid-based tests will not work."
    echo "  Try: sudo usermod -aG docker \$USER && newgrp docker"
fi

# ---------------------------------------------------------------------------
# 6. Done
# ---------------------------------------------------------------------------
echo ""
echo "================================================================"
echo "  ✅  twoyi devcontainer setup complete"
echo "================================================================"
echo ""
echo "Next steps:"
echo "  cd /workspaces/twoyi"
echo "  ./gradlew assembleRelease        # build the APK (arm64 + x86_64)"
echo "  ./.devcontainer/scripts/run-redroid.sh   # start an x86_64 redroid"
echo "  ./.devcontainer/scripts/test-twoyi.sh    # install APK + screenshot"
echo ""
echo "KVM status (re-run anytime):"
echo "  ./.devcontainer/scripts/check-kvm.sh"
