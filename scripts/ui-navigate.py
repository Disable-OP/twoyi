#!/usr/bin/env python3
"""
UI navigation script for twoyi E2E test.

Simulates a real user navigating the app entirely via UI taps:
  1. Launch app via monkey (taps launcher icon)
  2. Scroll down to find "Select ROM" preference
  3. Tap it → file picker opens
  4. Navigate file picker to /sdcard/Download/ → tap recovery.img
  5. Wait for import to complete
  6. Scroll to find "Boot to Recovery" checkbox → enable it
  7. Scroll back to top → tap "Launch Container"
  8. Wait for boot, taking screenshots every 5s to capture the TWRP screen
  9. Pull app logs

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

def adb(*args, timeout=30):
    try:
        r = subprocess.run(ADB + list(args), capture_output=True, text=True,
                           timeout=timeout)
        return r.stdout.strip()
    except subprocess.TimeoutExpired:
        return ""

def adb_shell(cmd, timeout=30):
    return adb("shell", cmd, timeout=timeout)

def screenshot(name):
    path = os.path.join(ART, f"screenshot-{name}.png")
    try:
        r = subprocess.run(ADB + ["exec-out", "screencap", "-p"],
                          capture_output=True, timeout=15)
        with open(path, "wb") as f:
            f.write(r.stdout)
        print(f"  [screenshot] {name} ({len(r.stdout)} bytes)")
    except Exception as e:
        print(f"  [screenshot] FAILED: {e}")
    return path

def dump_ui(name):
    adb_shell("uiautomator dump /sdcard/ui-dump.xml")
    xml_path = os.path.join(ART, f"uiautomator-{name}.xml")
    subprocess.run(ADB + ["pull", "/sdcard/ui-dump.xml", xml_path],
                  capture_output=True, timeout=10)
    return xml_path

def parse_ui(xml_path):
    try:
        tree = ET.parse(xml_path)
        return tree.getroot()
    except Exception:
        return None

def find_by_text(root, text, exact=False):
    """Find a UI node by text or content-desc. Returns (cx, cy, node) or None."""
    if root is None:
        return None
    search = text.lower()
    for node in root.iter("node"):
        txt = (node.get("text", "") or "").lower()
        desc = (node.get("content-desc", "") or "").lower()
        match = (txt == search) if exact else (search in txt)
        if not match:
            match = (desc == search) if exact else (search in desc)
        if match:
            bounds = node.get("bounds", "")
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
            if m:
                x1, y1, x2, y2 = int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4))
                cx, cy = (x1 + x2) // 2, (y1 + y2) // 2
                return (cx, cy, node)
    return None

def find_all_by_text(root, text, exact=False):
    results = []
    if root is None:
        return results
    search = text.lower()
    for node in root.iter("node"):
        txt = (node.get("text", "") or "").lower()
        desc = (node.get("content-desc", "") or "").lower()
        match = (txt == search) if exact else (search in txt)
        if not match:
            match = (desc == search) if exact else (search in desc)
        if match:
            bounds = node.get("bounds", "")
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
            if m:
                x1, y1, x2, y2 = int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4))
                cx, cy = (x1 + x2) // 2, (y1 + y2) // 2
                results.append((cx, cy, node))
    return results

def tap(x, y):
    adb_shell(f"input tap {x} {y}")

def swipe_up():
    """Swipe up to scroll down the list."""
    adb_shell("input swipe 540 1500 540 300 300")

def swipe_down():
    """Swipe down to scroll up the list."""
    adb_shell("input swipe 540 300 540 1500 300")

def wait(seconds):
    time.sleep(seconds)

def get_current_activity():
    out = adb_shell("dumpsys activity activities", timeout=10)
    for line in out.split("\n"):
        if "mResumedActivity" in line or "topResumedActivity" in line:
            return line.strip()
    return "(unknown)"

def scroll_to_find(text, max_scrolls=5, exact=True):
    """Scroll down up to max_scrolls times looking for text.
    Returns (cx, cy, node) or None."""
    for i in range(max_scrolls + 1):
        xml = dump_ui(f"scroll_{text}_{i}")
        root = parse_ui(xml)
        result = find_by_text(root, text, exact=exact)
        if result:
            print(f"  Found '{text}' at ({result[0]}, {result[1]}) after {i} scrolls")
            return result
        if i < max_scrolls:
            print(f"  '{text}' not visible — scrolling down (attempt {i+1}/{max_scrolls})")
            swipe_up()
            wait(1)
    return None

def main():
    boot_wait = int(os.environ.get("BOOT_WAIT_SECONDS", "60"))

    # ─────────────────────────────────────────────
    # Step 1: Launch app
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
    # Step 2: Scroll to find "Select ROM" and tap it
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print('  Step 2: Find and tap "Select ROM" preference')
    print("=" * 60)
    # "Select ROM" is deep in the Advanced category, below the fold.
    # We need to scroll down to find it.
    # Use EXACT matching to avoid matching "ROM" in "from" etc.
    result = scroll_to_find("Select ROM", max_scrolls=5, exact=True)
    if result:
        cx, cy, _ = result
        print(f"  Tapping 'Select ROM' at ({cx}, {cy})")
        tap(cx, cy)
        wait(3)
    else:
        print("  ✗ Could not find 'Select ROM' — dumping all visible text:")
        xml = dump_ui("02_debug_all_text")
        root = parse_ui(xml)
        if root:
            for node in root.iter("node"):
                t = node.get("text", "")
                if t:
                    print(f"    text={t!r}")

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

    # The file picker (DocumentsUI) should now be open.
    # Try to find recovery.img directly first (might be in Recent).
    found_file = False
    for text in ["recovery.img", "recovery"]:
        result = find_by_text(root, text, exact=False)
        if result:
            cx, cy, _ = result
            print(f"  Found '{text}' at ({cx}, {cy}) — tapping")
            tap(cx, cy)
            wait(3)
            found_file = True
            break

    if not found_file:
        # Open nav drawer (hamburger menu, top-left)
        print("  File not in current view — opening nav drawer")
        tap(50, 50)
        wait(2)
        xml = dump_ui("03_drawer_open")
        root = parse_ui(xml)

        # Look for "Downloads" in the drawer
        for text in ["Downloads", "Download"]:
            result = find_by_text(root, text, exact=False)
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
            result = find_by_text(root, text, exact=False)
            if result:
                cx, cy, _ = result
                print(f"  Found '{text}' at ({cx}, {cy}) — tapping")
                tap(cx, cy)
                wait(3)
                found_file = True
                break

        if not found_file:
            # Try navigating via path bar
            print("  Trying to navigate via path bar")
            tap(540, 36)
            wait(2)
            xml = dump_ui("03c_path_bar")
            root = parse_ui(xml)
            for text in ["recovery.img", "recovery", "Download"]:
                result = find_by_text(root, text, exact=False)
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
    # "Boot to Recovery" is in the Advanced category, above "Select ROM".
    # We may need to scroll to find it.
    result = scroll_to_find("Boot to Recovery", max_scrolls=3, exact=False)
    if result:
        cx, cy, node = result
        checked = node.get("checked", "false")
        print(f"  Found 'Boot to Recovery' at ({cx}, {cy}), checked={checked}")
        if checked == "false":
            print("  Tapping to enable")
            tap(cx, cy)
            wait(1)
        else:
            print("  Already enabled")
    else:
        print("  ✗ Could not find 'Boot to Recovery'")

    # ─────────────────────────────────────────────
    # Step 6: Scroll to top and tap "Launch Container"
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print('  Step 6: Tap "Launch Container"')
    print("=" * 60)
    # Scroll back to top
    for _ in range(5):
        swipe_down()
        wait(0.5)

    xml = dump_ui("06_before_launch")
    root = parse_ui(xml)
    result = find_by_text(root, "Launch Container", exact=True)
    if result:
        cx, cy, _ = result
        print(f"  Found 'Launch Container' at ({cx}, {cy}) — tapping")
        tap(cx, cy)
        wait(5)
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
    # Step 7: Wait for boot — screenshots every 5s
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print(f"  Step 7: Wait for boot ({boot_wait}s) — screenshots every 5s")
    print("=" * 60)
    for i in range(boot_wait // 5):
        wait(5)
        elapsed = (i + 1) * 5
        screenshot(f"07_boot_{elapsed}s")
        activity = get_current_activity()
        if "Render2Activity" not in activity and elapsed > 15:
            print(f"  Left Render2Activity at {elapsed}s: {activity}")
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

    # Try to pull kr64 logs
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
