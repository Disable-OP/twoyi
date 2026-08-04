#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Definitively checks whether KVM is available in this codespace.
#
# Background: GitHub Codespaces run on Azure VMs. The devcontainer.json
# `runArgs: ["--privileged"]` gives the container extended privileges,
# but it cannot create /dev/kvm if the host kernel doesn't expose it.
# Multiple authoritative sources (see README in this directory) confirm
# that /dev/kvm is NOT available in Codespaces as of 2026.
#
# This script writes a verdict to /tmp/kvm-status.txt so other scripts
# (run-redroid.sh, test-twoyi.sh) can branch on the result.

set -u

STATUS_FILE="/tmp/kvm-status.txt"
echo "================================================================" | tee "$STATUS_FILE"
echo "  KVM availability check" | tee -a "$STATUS_FILE"
echo "================================================================" | tee -a "$STATUS_FILE"
echo "" | tee -a "$STATUS_FILE"

# Test 1: does /dev/kvm exist?
echo "Test 1: /dev/kvm existence" | tee -a "$STATUS_FILE"
if [ -e /dev/kvm ]; then
    echo "  ✓ /dev/kvm exists" | tee -a "$STATUS_FILE"
    DEV_KVM=yes
else
    echo "  ✗ /dev/kvm does NOT exist" | tee -a "$STATUS_FILE"
    DEV_KVM=no
fi
echo "" | tee -a "$STATUS_FILE"

# Test 2: can we read/write it?
if [ "$DEV_KVM" = "yes" ]; then
    echo "Test 2: /dev/kvm read/write access" | tee -a "$STATUS_FILE"
    if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
        echo "  ✓ /dev/kvm is readable and writable" | tee -a "$STATUS_FILE"
        KVM_RW=yes
    else
        echo "  ✗ /dev/kvm is NOT accessible (permissions)" | tee -a "$STATUS_FILE"
        KVM_RW=no
    fi
    echo "" | tee -a "$STATUS_FILE"
else
    KVM_RW=no
fi

# Test 3: CPU virtualization extensions
echo "Test 3: CPU virtualization extensions (vmx/svm)" | tee -a "$STATUS_FILE"
VIRT_EXTENSIONS=$(grep -c -E '(vmx|svm)' /proc/cpuinfo 2>/dev/null || echo 0)
if [ "$VIRT_EXTENSIONS" -gt 0 ]; then
    echo "  ✓ Found $VIRT_EXTENSIONS CPU(s) with vmx/svm extensions" | tee -a "$STATUS_FILE"
else
    echo "  ✗ No vmx/svm CPU extensions found" | tee -a "$STATUS_FILE"
fi
echo "" | tee -a "$STATUS_FILE"

# Test 4: can we load the kvm kernel module?
echo "Test 4: kvm kernel module" | tee -a "$STATUS_FILE"
if lsmod 2>/dev/null | grep -q '^kvm '; then
    echo "  ✓ kvm module is already loaded" | tee -a "$STATUS_FILE"
    KVM_MODULE=yes
elif sudo modprobe kvm 2>/dev/null; then
    echo "  ✓ kvm module loaded successfully" | tee -a "$STATUS_FILE"
    KVM_MODULE=yes
else
    echo "  ✗ kvm module cannot be loaded" | tee -a "$STATUS_FILE"
    KVM_MODULE=no
fi
echo "" | tee -a "$STATUS_FILE"

# Test 5: kvm-ok if available
echo "Test 5: kvm-ok (if installed)" | tee -a "$STATUS_FILE"
if command -v kvm-ok >/dev/null 2>&1; then
    if kvm-ok 2>&1 | tee -a "$STATUS_FILE"; then
        :
    fi
else
    echo "  (kvm-ok not installed — skipping)" | tee -a "$STATUS_FILE"
fi
echo "" | tee -a "$STATUS_FILE"

# Verdict
echo "================================================================" | tee -a "$STATUS_FILE"
if [ "$DEV_KVM" = "yes" ] && [ "$KVM_RW" = "yes" ]; then
    echo "  VERDICT: KVM IS AVAILABLE — hardware-accelerated emulators will work." | tee -a "$STATUS_FILE"
    echo "  KVM_AVAILABLE=yes" > /tmp/kvm-verdict.txt
else
    echo "  VERDICT: KVM IS NOT AVAILABLE." | tee -a "$STATUS_FILE"
    echo "  Falling back to redroid (Android-in-container, no KVM needed)." | tee -a "$STATUS_FILE"
    echo "  See: https://github.com/devcontainers/images/issues/884" | tee -a "$STATUS_FILE"
    echo "  KVM_AVAILABLE=no" > /tmp/kvm-verdict.txt
fi
echo "================================================================" | tee -a "$STATUS_FILE"

cat /tmp/kvm-verdict.txt
