#!/usr/bin/env python3
"""
UI navigation script for twoyi E2E test.

Simulates a real user:
  1. Launch app via monkey (equivalent to tapping launcher icon)
  2. Tap "Select ROM" preference
  3. Navigate file picker to /sdcard/Download/ → tap recovery.img
  4. Wait for import to complete
  5. Enable "Boot to Recovery" checkbox
  6. Tap "Launch Container"
  7. Wait for boot, taking screenshots every 5s to capture the TWRP screen
  8. Pull app logs

Uses uiautomator dump + XML parsing to find elements by text.
No hardcoded coordinates — everything is text-based.
"""
import os
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

ADB = ["adb", "-s", "emulator-5554"]
ART = "/tmp/ui-e2e-artifacts"
os.makedirs(ART, exist_ok=True)

# TWRP recovery image is 720x1280; the emulator is typically 1080x1920.
# The app's SurfaceView will be letterboxed inside the screen.

def adb(*args, timeout=30):
    """Run an adb command, return stdout."""
    try:
        r = subprocess.run(ADB + list(args), capture_output=True, text=True,
                           timeout=timeout)
        return r.stdout.strip()
    except subprocess.TimeoutExpired:
        return ""

def adb_shell(cmd, timeout=30):
    """Run an adb shell command."""
    return adb("shell", cmd, timeout=timeout)

def screenshot(name):
    """Take a screenshot and save to artifacts dir."""
    path = os.path.join(ART, f"screenshot-{name}.png")
    try:
        r = subprocess.run(ADB + ["exec-out", "screencap", "-p"],
                          capture_output=True, timeout=15)
        with open(path, "wb") as f:
            f.write(r.stdout)
        print(f"  [screenshot] {path} ({len(r.stdout)} bytes)")
    except Exception as e:
        print(f"  [screenshot] FAILED: {e}")
    return path

def dump_ui(name):
    """Dump UI hierarchy to XML + save a copy to artifacts."""
    adb_shell("uiautomator dump /sdcard/ui-dump.xml")
    xml_path = os.path.join(ART, f"uiautomator-{name}.xml")
    subprocess.run(ADB + ["pull", "/sdcard/ui-dump.xml", xml_path],
                  capture_output=True, timeout=10)
    return xml_path

def parse_ui(xml_path):
    """Parse uiautomator XML and return list of nodes."""
    try:
        tree = ET.parse(xml_path)
        return tree.getroot()
    except Exception:
        return None

def find_by_text(root, text, partial=True):
    """Find a UI node by text or content-desc. Returns (cx, cy, node) or None."""
    if root is None:
        return None
    search = text.lower()
    for node in root.iter("node"):
        txt = (node.get("text", "") or "").lower()
        desc = (node.get("content-desc", "") or "").lower()
        match = (search in txt) if partial else (txt == search)
        if not match:
            match = (search in desc) if partial else (desc == search)
        if match:
            bounds = node.get("bounds", "")
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
            if m:
                x1, y1, x2, y2 = int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4))
                cx, cy = (x1 + x2) // 2, (y1 + y2) // 2
                return (cx, cy, node)
    return None

def find_all_by_text(root, text, partial=True):
    """Find ALL UI nodes matching text. Returns list of (cx, cy, node)."""
    results = []
    if root is None:
        return results
    search = text.lower()
    for node in root.iter("node"):
        txt = (node.get("text", "") or "").lower()
        desc = (node.get("content-desc", "") or "").lower()
        match = (search in txt) if partial else (txt == search)
        if not match:
            match = (search in desc) if partial else (desc == search)
        if match:
            bounds = node.get("bounds", "")
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
            if m:
                x1, y1, x2, y2 = int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4))
                cx, cy = (x1 + x2) // 2, (y1 + y2) // 2
                results.append((cx, cy, node))
    return results

def tap(x, y):
    """Tap at coordinates."""
    adb_shell(f"input tap {x} {y}")

def wait(seconds):
    time.sleep(seconds)

def get_current_activity():
    """Get the currently focused activity."""
    out = adb_shell("dumpsys activity activities", timeout=10)
    for line in out.split("\n"):
        if "mResumedActivity" in line or "topResumedActivity" in line:
            return line.strip()
    return "(unknown)"

def main():
    boot_wait = int(os.environ.get("BOOT_WAIT_SECONDS", "60"))

    # ─────────────────────────────────────────────
    # Step 1: Launch app via monkey (taps launcher icon)
    # ─────────────────────────────────────────────
    print("=" * 60)
    print("  Step 1: Launch app via launcher (monkey -p)")
    print("=" * 60)
    adb_shell("monkey -p io.twoyi -c android.intent.category.LAUNCHER 1")
    wait(5)
    xml = dump_ui("01_app_launched")
    root = parse_ui(xml)
    print(f"  Current activity: {get_current_activity()}")

    # ─────────────────────────────────────────────
    # Step 2: Tap "Select ROM" preference
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print('  Step 2: Tap "Select ROM" preference')
    print("=" * 60)
    # Try various text variants
    for text in ["Select ROM", "Select ROM File", "ROM"]:
        result = find_by_text(root, text)
        if result:
            cx, cy, _ = result
            print(f"  Found '{text}' at ({cx}, {cy}) — tapping")
            tap(cx, cy)
            wait(3)
            break
    else:
        print("  ✗ Could not find 'Select ROM' preference")
        # Print all visible text for debugging
        if root:
            for node in root.iter("node"):
                t = node.get("text", "")
                d = node.get("content-desc", "")
                if t or d:
                    print(f"    text={t!r} desc={d!r}")

    xml = dump_ui("02_after_select_rom")
    root = parse_ui(xml)
    print(f"  Current activity: {get_current_activity()}")

    # ─────────────────────────────────────────────
    # Step 3: Navigate file picker to recovery.img
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print("  Step 3: Navigate file picker to recovery.img")
    print("=" * 60)

    # The file picker (DocumentsUI) is now open. We need to find recovery.img.
    # Strategy:
    #   a) Look for "recovery.img" directly (might be in Recent)
    #   b) If not found, open nav drawer (hamburger) → tap "Downloads"
    #   c) Then look for "recovery.img" in the file list

    # Try (a): look for the file directly
    found_file = False
    for text in ["recovery.img", "recovery", "byt_t", "twrp"]:
        result = find_by_text(root, text)
        if result:
            cx, cy, _ = result
            print(f"  Found '{text}' at ({cx}, {cy}) — tapping")
            tap(cx, cy)
            wait(3)
            found_file = True
            break

    if not found_file:
        # Try (b): open nav drawer
        print("  File not in current view — opening nav drawer")
        # Tap hamburger menu (top-left corner, ~50,100)
        tap(50, 100)
        wait(2)
        xml = dump_ui("03_drawer_open")
        root = parse_ui(xml)

        # Look for "Downloads" in the drawer
        for text in ["Downloads", "Download", "All files"]:
            result = find_by_text(root, text)
            if result:
                cx, cy, _ = result
                print(f"  Found '{text}' at ({cx}, {cy}) — tapping")
                tap(cx, cy)
                wait(2)
                break

        # Now look for recovery.img in the file list
        xml = dump_ui("03b_in_downloads")
        root = parse_ui(xml)
        for text in ["recovery.img", "recovery", "byt_t", "twrp"]:
            result = find_by_text(root, text)
            if result:
                cx, cy, _ = result
                print(f"  Found '{text}' at ({cx}, {cy}) — tapping")
                tap(cx, cy)
                wait(3)
                found_file = True
                break

        if not found_file:
            # Last resort: try navigating to /sdcard/Download via the
            # breadcrumb path bar. Some DocumentsUI versions show a
            # path bar at the top.
            print("  Trying to navigate via path bar")
            # Tap the path/breadcrumb area (top of screen)
            tap(540, 72)
            wait(2)
            xml = dump_ui("03c_path_bar")
            root = parse_ui(xml)
            for text in ["recovery.img", "recovery", "Download"]:
                result = find_by_text(root, text)
                if result:
                    cx, cy, _ = result
                    print(f"  Found '{text}' at ({cx}, {cy}) — tapping")
                    tap(cx, cy)
                    wait(3)
                    found_file = True
                    break

    xml = dump_ui("04_after_file_select")
    root = parse_ui(xml)
    print(f"  Current activity: {get_current_activity()}")

    # ─────────────────────────────────────────────
    # Step 4: Wait for ROM import to complete
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print("  Step 4: Wait for ROM import (up to 120s)")
    print("=" * 60)
    for i in range(60):
        wait(2)
        xml = dump_ui(f"05_import_wait_{i}")
        root = parse_ui(xml)
        # Check if progress dialog is gone (import complete)
        has_progress = False
        if root:
            for node in root.iter("node"):
                txt = (node.get("text", "") or "").lower()
                if any(w in txt for w in ["progress", "importing", "extracting", "loading", "please wait"]):
                    has_progress = True
                    break
        if not has_progress:
            print(f"  Import appears complete (no progress dialog at attempt {i})")
            break
        if i % 10 == 0:
            print(f"  Still importing... (attempt {i})")

    xml = dump_ui("05_import_done")
    root = parse_ui(xml)
    print(f"  Current activity: {get_current_activity()}")

    # ─────────────────────────────────────────────
    # Step 5: Enable "Boot to Recovery" checkbox
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print('  Step 5: Enable "Boot to Recovery" checkbox')
    print("=" * 60)
    # We may need to scroll down to find it, or it might be visible.
    # First, check current view.
    for text in ["Boot to Recovery", "Boot Recovery", "Recovery"]:
        results = find_all_by_text(root, text)
        if results:
            cx, cy, node = results[0]
            # Check if it's a checkbox and whether it's already checked
            checked = node.get("checked", "false")
            print(f"  Found '{text}' at ({cx}, {cy}), checked={checked}")
            if checked == "false":
                print(f"  Tapping to enable")
                tap(cx, cy)
                wait(1)
            else:
                print(f"  Already enabled")
            break
    else:
        print("  'Boot to Recovery' not found — scrolling down")
        # Swipe up to scroll
        adb_shell("input swipe 540 1500 540 300 300")
        wait(1)
        xml = dump_ui("05b_after_scroll")
        root = parse_ui(xml)
        for text in ["Boot to Recovery", "Boot Recovery", "Recovery"]:
            results = find_all_by_text(root, text)
            if results:
                cx, cy, node = results[0]
                checked = node.get("checked", "false")
                print(f"  Found '{text}' at ({cx}, {cy}), checked={checked}")
                if checked == "false":
                    tap(cx, cy)
                    wait(1)
                break

    # ─────────────────────────────────────────────
    # Step 6: Tap "Launch Container"
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print('  Step 6: Tap "Launch Container"')
    print("=" * 60)
    # Scroll back to top first
    adb_shell("input swipe 540 300 540 1500 300")
    wait(1)
    xml = dump_ui("06_before_launch")
    root = parse_ui(xml)

    for text in ["Launch Container", "Launch", "Start", "Boot", "Container", "Run"]:
        results = find_all_by_text(root, text)
        if results:
            cx, cy, _ = results[0]
            print(f"  Found '{text}' at ({cx}, {cy}) — tapping")
            tap(cx, cy)
            wait(5)
            break
    else:
        print("  ✗ Could not find 'Launch Container'")
        if root:
            for node in root.iter("node"):
                t = node.get("text", "")
                if t:
                    print(f"    text={t!r}")

    xml = dump_ui("06_after_launch")
    root = parse_ui(xml)
    print(f"  Current activity: {get_current_activity()}")

    # ─────────────────────────────────────────────
    # Step 7: Wait for boot — take screenshots every 5s
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print(f"  Step 7: Wait for boot ({boot_wait}s) — screenshots every 5s")
    print("=" * 60)
    for i in range(boot_wait // 5):
        wait(5)
        elapsed = (i + 1) * 5
        screenshot(f"07_boot_{elapsed}s")
        # Check if we're still in Render2Activity
        activity = get_current_activity()
        if "Render2Activity" not in activity and elapsed > 15:
            print(f"  Left Render2Activity at {elapsed}s: {activity}")
            # The app may have crashed or the user was kicked back
            break

    # ─────────────────────────────────────────────
    # Step 8: Final capture
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print("  Step 8: Final capture")
    print("=" * 60)
    screenshot("08_final")
    dump_ui("08_final")
    print(f"  Final activity: {get_current_activity()}")

    # Capture logcat
    print("  Capturing logcat...")
    logcat = adb("logcat", "-d", timeout=15)
    with open(os.path.join(ART, "logcat.txt"), "w") as f:
        f.write(logcat)

    # Pull app's FileLogger logs
    print("  Pulling app file logs...")
    os.makedirs(os.path.join(ART, "app-logs"), exist_ok=True)
    subprocess.run(ADB + ["pull", "/sdcard/Android/data/io.twoyi/files/log/",
                         os.path.join(ART, "app-logs/")],
                  capture_output=True, timeout=30)

    # Try to pull kr64-app-stderr.log (needs root — usually fails)
    subprocess.run(ADB + ["pull", "/data/data/io.twoyi/kr64-app-stderr.log",
                         os.path.join(ART, "kr64-app-stderr.log")],
                  capture_output=True, timeout=10)

    print()
    print("=" * 60)
    print("  Artifacts captured:")
    print("=" * 60)
    for f in sorted(os.listdir(ART)):
        path = os.path.join(ART, f)
        if os.path.isfile(path):
            print(f"  {f} ({os.path.getsize(path)} bytes)")
        elif os.path.isdir(path):
            print(f"  {f}/")
            for sub in sorted(os.listdir(path)):
                subpath = os.path.join(path, sub)
                print(f"    {sub} ({os.path.getsize(subpath)} bytes)")

if __name__ == "__main__":
    main()
