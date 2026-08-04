#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Runs on codespace creation. Sets up the Android SDK + NDK and
# creates /dev/kvm if the kernel supports it but the device node
# wasn't auto-created.

set -e

echo "================================================================"
echo "  twoyi devcontainer setup (Ubuntu 22.04)"
echo "================================================================"

# ---------------------------------------------------------------------------
# 1. Create /dev/kvm if the kernel module is loaded but the device node
#    doesn't exist. On GitHub Codespaces with --privileged, the kvm module
#    is often loaded but udev doesn't create /dev/kvm inside the container.
#    We create it manually with mknod and set mode 666 so all users can
#    access it. (The seccomp profile may still block KVM_RUN on some
#    VM series — run check-kvm.sh to verify.)
# ---------------------------------------------------------------------------
echo ""
echo "── KVM setup ──"
if [ ! -e /dev/kvm ]; then
    if lsmod 2>/dev/null | grep -q '^kvm '; then
        echo "  kvm module is loaded but /dev/kvm doesn't exist — creating it..."
        sudo mknod /dev/kvm c 10 232
        sudo chmod 666 /dev/kvm
        echo "  ✓ /dev/kvm created (mode 666)"
    else
        echo "  kvm module not loaded — trying modprobe..."
        sudo modprobe kvm 2>/dev/null || true
        sudo modprobe kvm_intel 2>/dev/null || true
        sudo modprobe kvm_amd 2>/dev/null || true
        if [ -c /dev/kvm ] || sudo mknod /dev/kvm c 10 232 2>/dev/null; then
            sudo chmod 666 /dev/kvm 2>/dev/null || true
            echo "  ✓ /dev/kvm created after modprobe"
        else
            echo "  ✗ KVM not available on this VM"
        fi
    fi
else
    echo "  ✓ /dev/kvm already exists"
fi

# Add vscode user to the kvm group (needed by some tools)
sudo gpasswd -a vscode kvm 2>/dev/null || true

# Run the definitive KVM check
bash /workspaces/twoyi/.devcontainer/scripts/check-kvm.sh || true

# ---------------------------------------------------------------------------
# 2. Android SDK command-line tools
# ---------------------------------------------------------------------------
echo ""
echo "── Installing Android SDK command-line tools ──"
ANDROID_SDK_ROOT="${ANDROID_HOME:-/workspaces/twoyi/.android-sdk}"
if [ ! -d "$ANDROID_SDK_ROOT/cmdline-tools/latest" ]; then
    mkdir -p "$ANDROID_SDK_ROOT/cmdline-tools"
    cd /tmp
    wget -q https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip -O cmdline-tools.zip
    unzip -q cmdline-tools.zip -d "$ANDROID_SDK_ROOT/cmdline-tools"
    mv "$ANDROID_SDK_ROOT/cmdline-tools/cmdline-tools" "$ANDROID_SDK_ROOT/cmdline-tools/latest"
    rm cmdline-tools.zip
fi
export PATH="$ANDROID_SDK_ROOT/cmdline-tools/latest/bin:$ANDROID_SDK_ROOT/platform-tools:$ANDROID_SDK_ROOT/emulator:$PATH"

yes | sdkmanager --licenses >/dev/null 2>&1 || true
sdkmanager --update
sdkmanager \
    "platform-tools" \
    "platforms;android-31" \
    "build-tools;30.0.3" \
    "emulator" \
    "system-images;android-30;google_apis;x86_64"
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
# 4. Rust targets + cargo-xdk (already installed in Dockerfile, but verify)
# ---------------------------------------------------------------------------
echo ""
echo "── Verifying Rust toolchain ──"
source "$HOME/.cargo/env" 2>/dev/null || true
rustup target add aarch64-linux-android x86_64-linux-android 2>/dev/null || true
echo "✓ rustc: $(rustc --version)"
echo "✓ cargo: $(cargo --version)"
echo "✓ cargo-xdk: $(cargo xdk --version 2>&1 || echo 'not found')"

# ---------------------------------------------------------------------------
# 5. Docker (for redroid-based testing if KVM is unavailable)
# ---------------------------------------------------------------------------
echo ""
echo "── Verifying Docker access ──"
if docker info >/dev/null 2>&1; then
    echo "✓ Docker is accessible"
else
    echo "⚠ Docker is not accessible — redroid tests won't work"
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
echo "  ./gradlew assembleRelease -Pabis=all   # build the APK"
echo "  ./.devcontainer/scripts/check-kvm.sh   # verify KVM"
echo "  ./.devcontainer/scripts/run-redroid.sh # start Android container"
echo ""
