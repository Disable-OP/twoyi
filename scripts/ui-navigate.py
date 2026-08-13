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
Detects actual screen size from the hierarchy root bounds and scales
all swipe/tap coordinates accordingly.
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

# Screen dimensions — detected from the first uiautomator dump
SCREEN_W = 320
SCREEN_H = 640

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

def detect_screen_size(root):
    """Detect screen size from the root hierarchy node bounds."""
    global SCREEN_W, SCREEN_H
    if root is not None:
        bounds = root.get("bounds", "")
        m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
        if m:
            SCREEN_W = int(m.group(3))
            SCREEN_H = int(m.group(4))
            print(f"  [screen] {SCREEN_W}x{SCREEN_H}")

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

def find_tap_target_for_text(root, text, exact=False):
    """Find a UI node by text, then walk up the parent chain to find the
    widest ancestor (the row/card container). Return the center of that
    container — this is where to tap to actually trigger a click, because
    the text node itself is often NOT clickable (clickable=false in the
    uiautomator dump) even though the row container IS clickable at the
    RecyclerView level.

    This is critical for the Android SAF file picker: the file name text
    is NOT clickable, but the row containing it IS (via the RecyclerView's
    ViewHolder click handler). Tapping the text center does nothing; we
    need to tap the row center."""
    if root is None:
        return None
    # Build parent map for walking up
    parent_map = {c: p for p in root.iter() for c in p}
    search = text.lower()
    for node in root.iter("node"):
        txt = (node.get("text", "") or "").lower()
        desc = (node.get("content-desc", "") or "").lower()
        match = (txt == search) if exact else (search in txt)
        if not match:
            match = (desc == search) if exact else (search in desc)
        if match:
            # Walk up the parent chain to find the widest ancestor
            widest_node = node
            widest_width = 0
            cur = node
            depth = 0
            while cur in parent_map and depth < 15:
                parent = parent_map[cur]
                pbounds = parent.get("bounds", "")
                m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", pbounds)
                if m:
                    x1, y1, x2, y2 = int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4))
                    w = x2 - x1
                    if w > widest_width:
                        widest_width = w
                        widest_node = parent
                cur = parent
                depth += 1
            # Return center of the widest ancestor
            bounds = widest_node.get("bounds", "")
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
            if m:
                x1, y1, x2, y2 = int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4))
                cx, cy = (x1 + x2) // 2, (y1 + y2) // 2
                return (cx, cy, widest_node)
            # Fallback: return the text node's own center
            bounds = node.get("bounds", "")
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
            if m:
                x1, y1, x2, y2 = int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4))
                cx, cy = (x1 + x2) // 2, (y1 + y2) // 2
                return (cx, cy, node)
    return None

def tap(x, y):
    """Tap at coordinates. Use 'input swipe' with same start/end point and
    100ms duration instead of 'input tap' — the short-duration 'input tap'
    sometimes doesn't register on RecyclerView items that use
    OnItemTouchListener (like the Android SAF file picker). The swipe
    variant sends a proper DOWN→UP sequence with sufficient duration for
    the GestureDetector to register it as a click."""
    adb_shell(f"input swipe {x} {y} {x} {y} 100")

def swipe_up():
    """Swipe up to scroll down the list. Uses screen-relative coordinates."""
    cx = SCREEN_W // 2
    y1 = int(SCREEN_H * 0.7)
    y2 = int(SCREEN_H * 0.3)
    adb_shell(f"input swipe {cx} {y1} {cx} {y2} 300")

def swipe_down():
    """Swipe down to scroll up the list. Uses screen-relative coordinates."""
    cx = SCREEN_W // 2
    y1 = int(SCREEN_H * 0.3)
    y2 = int(SCREEN_H * 0.7)
    adb_shell(f"input swipe {cx} {y1} {cx} {y2} 300")

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
        if i == 0:
            detect_screen_size(root)
        result = find_by_text(root, text, exact=exact)
        if result:
            print(f"  Found '{text}' at ({result[0]}, {result[1]}) after {i} scrolls")
            return result
        if i < max_scrolls:
            print(f"  '{text}' not visible — scrolling down (attempt {i+1}/{max_scrolls})")
            swipe_up()
            wait(1)
    return None

def print_all_text(root, prefix="    "):
    """Print all non-empty text/content-desc for debugging."""
    if root is None:
        return
    for node in root.iter("node"):
        t = node.get("text", "")
        d = node.get("content-desc", "")
        if t:
            print(f"{prefix}text={t!r}")
        if d:
            print(f"{prefix}desc={d!r}")

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
    detect_screen_size(root)
    print(f"  Current activity: {get_current_activity()}")

    # ─────────────────────────────────────────────
    # Step 2: Scroll to find "Select ROM" and tap it
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print('  Step 2: Find and tap "Select ROM" preference')
    print("=" * 60)
    # "Select ROM" is deep in the Advanced category, below the fold.
    # Use EXACT matching to avoid matching "ROM" in "from" etc.
    result = scroll_to_find("Select ROM", max_scrolls=8, exact=True)
    if result:
        cx, cy, _ = result
        print(f"  Tapping 'Select ROM' at ({cx}, {cy})")
        tap(cx, cy)
        wait(4)
    else:
        print("  ✗ Could not find 'Select ROM' — dumping all visible text:")
        xml = dump_ui("02_debug_all_text")
        root = parse_ui(xml)
        print_all_text(root)

    xml = dump_ui("02_after_select_rom")
    root = parse_ui(xml)
    activity = get_current_activity()
    print(f"  Current activity: {activity}")

    # Check if the file picker actually opened
    if "SettingsActivity" in activity:
        print("  ⚠ Still on SettingsActivity — file picker did not open!")
        # Maybe the tap hit the wrong row. Try scrolling more.
        for extra_scroll in range(3):
            swipe_up()
            wait(1)
            xml = dump_ui(f"02b_extra_scroll_{extra_scroll}")
            root = parse_ui(xml)
            result = find_by_text(root, "Select ROM", exact=True)
            if result:
                cx, cy, _ = result
                print(f"  Found 'Select ROM' on extra scroll {extra_scroll} at ({cx}, {cy}) — tapping")
                tap(cx, cy)
                wait(4)
                xml = dump_ui("02c_after_retry_tap")
                root = parse_ui(xml)
                activity = get_current_activity()
                print(f"  Current activity: {activity}")
                if "SettingsActivity" not in activity:
                    break

    # ─────────────────────────────────────────────
    # Step 3: Navigate file picker to recovery.img
    #
    # The Android SAF (Storage Access Framework) file picker on API 30:
    #   - Opens with "Recent" view by default
    #   - Has a hamburger menu (top-left) → drawer with "Downloads", etc.
    #   - Or has a breadcrumb path bar at the top
    #   - Or shows "Internal storage" / "SD card" options
    #
    # Strategy (in order of preference):
    #   a) Look for recovery.img directly (might be in Recent if we're lucky)
    #   b) Open the hamburger menu / drawer → tap "Downloads"
    #   c) Look for recovery.img in Downloads
    #   d) If still not found, try "Show internal storage" → navigate tree
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print("  Step 3: Navigate file picker to recovery.img")
    print("=" * 60)

    found_file = False

    # (a) Try to find recovery.img directly in the current view.
    # CRITICAL: use find_tap_target_for_text() instead of find_by_text()
    # because the file name text node is NOT clickable — we need to tap
    # the center of the file ROW (the widest ancestor), which is the
    # clickable RecyclerView item.
    for text in ["recovery.img", "recovery"]:
        result = find_tap_target_for_text(root, text, exact=False)
        if result:
            cx, cy, _ = result
            print(f"  Found '{text}' — row center at ({cx}, {cy}) — tapping")
            tap(cx, cy)
            wait(3)
            found_file = True
            break

    if not found_file:
        # (b) Open the hamburger menu / drawer
        print("  recovery.img not in current view — opening nav drawer")

        # Look for the "Show Roots" / "Navigate up" button
        drawer_opened = False
        for desc in ["Show roots", "Navigate up", "Show list", "More options"]:
            result = find_by_text(root, desc, exact=False)
            if result:
                cx, cy, _ = result
                print(f"  Found '{desc}' at ({cx}, {cy}) — tapping to open drawer")
                tap(cx, cy)
                wait(2)
                drawer_opened = True
                break

        if not drawer_opened:
            hamburger_x = int(SCREEN_W * 0.08)
            hamburger_y = int(SCREEN_H * 0.08)
            print(f"  No drawer button found — tapping top-left ({hamburger_x}, {hamburger_y})")
            tap(hamburger_x, hamburger_y)
            wait(2)

        xml = dump_ui("03_drawer_open")
        root = parse_ui(xml)
        print(f"  Current activity: {get_current_activity()}")
        print("  Visible text after drawer tap:")
        print_all_text(root, prefix="    ")

        # (c) Look for "Downloads" in the drawer
        for text in ["Downloads", "Download"]:
            result = find_tap_target_for_text(root, text, exact=False)
            if result:
                cx, cy, _ = result
                print(f"  Found '{text}' — row center at ({cx}, {cy}) — tapping")
                tap(cx, cy)
                wait(3)
                break

        # Now look for recovery.img in the file list
        xml = dump_ui("03b_in_downloads")
        root = parse_ui(xml)
        print("  Visible text in Downloads:")
        print_all_text(root, prefix="    ")

        for text in ["recovery.img", "recovery", "byt_t", "twrp"]:
            result = find_tap_target_for_text(root, text, exact=False)
            if result:
                cx, cy, _ = result
                print(f"  Found '{text}' — row center at ({cx}, {cy}) — tapping")
                tap(cx, cy)
                wait(3)
                found_file = True
                break

        if not found_file:
            # (d) Try "Show internal storage" / "SD card" / phone storage
            print("  recovery.img not in Downloads — trying internal storage")
            for text in ["Internal storage", "Phone", "SD card", "Storage"]:
                result = find_tap_target_for_text(root, text, exact=False)
                if result:
                    cx, cy, _ = result
                    print(f"  Found '{text}' — row center at ({cx}, {cy}) — tapping")
                    tap(cx, cy)
                    wait(2)
                    break

            xml = dump_ui("03c_internal_storage")
            root = parse_ui(xml)
            for text in ["Download", "Downloads"]:
                result = find_tap_target_for_text(root, text, exact=False)
                if result:
                    cx, cy, _ = result
                    print(f"  Found '{text}' — row center at ({cx}, {cy}) — tapping")
                    tap(cx, cy)
                    wait(2)
                    break

            xml = dump_ui("03d_in_download_folder")
            root = parse_ui(xml)
            for text in ["recovery.img", "recovery", "byt_t", "twrp"]:
                result = find_tap_target_for_text(root, text, exact=False)
                if result:
                    cx, cy, _ = result
                    print(f"  Found '{text}' — row center at ({cx}, {cy}) — tapping")
                    tap(cx, cy)
                    wait(3)
                    found_file = True
                    break

    # After tapping the file, check if we're back on SettingsActivity
    # (file picker closed = file was selected). If still on PickActivity,
    # the tap didn't work — try double-tapping or tapping a different spot.
    xml = dump_ui("04_after_file_select")
    root = parse_ui(xml)
    activity = get_current_activity()
    print(f"  Current activity: {activity}")

    if "documentsui" in activity.lower() or "picker" in activity.lower():
        print("  ⚠ Still on file picker — file was not selected!")
        print("  Visible text:")
        print_all_text(root, prefix="    ")
        # Try double-tapping recovery.img
        print("  Trying double-tap on recovery.img...")
        result = find_tap_target_for_text(root, "recovery.img", exact=False)
        if result:
            cx, cy, _ = result
            tap(cx, cy)
            wait(0.1)
            tap(cx, cy)
            wait(3)
        xml = dump_ui("04b_after_doubletap")
        root = parse_ui(xml)
        activity = get_current_activity()
        print(f"  After double-tap: {activity}")

        if "documentsui" in activity.lower() or "picker" in activity.lower():
            print("  Still on file picker — trying to tap the 'Preview' icon instead...")
            # The "Preview the file recovery.img" FrameLayout IS clickable
            for desc in ["Preview the file recovery.img", "Preview"]:
                result = find_by_text(root, desc, exact=False)
                if result:
                    cx, cy, _ = result
                    print(f"  Found '{desc}' at ({cx}, {cy}) — tapping")
                    tap(cx, cy)
                    wait(3)
                    break
            xml = dump_ui("04c_after_preview_tap")
            root = parse_ui(xml)
            activity = get_current_activity()
            print(f"  After preview tap: {activity}")

            if "documentsui" in activity.lower() or "picker" in activity.lower():
                print("  Still on file picker — trying DPAD navigation...")
                # Use keyboard navigation: tap the first file to give focus,
                # then use DPAD to navigate to recovery.img and ENTER to select.
                # First, tap on the file list area to give it focus
                tap(SCREEN_W // 2, int(SCREEN_H * 0.6))
                wait(1)
                # Navigate down with DPAD — recovery.img is the 2nd file
                for _ in range(3):
                    adb_shell("input keyevent KEYCODE_DPAD_DOWN")
                    wait(0.5)
                adb_shell("input keyevent KEYCODE_ENTER")
                wait(3)
                xml = dump_ui("04d_after_dpad")
                root = parse_ui(xml)
                activity = get_current_activity()
                print(f"  After DPAD: {activity}")

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
                if any(w in txt for w in ["progress", "importing", "extracting", "loading", "please wait", "extract"]):
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
    # We should be back on SettingsActivity now. Scroll to find it.
    result = scroll_to_find("Boot to Recovery", max_scrolls=5, exact=False)
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
    # Scroll back to top — swipe down multiple times
    for _ in range(8):
        swipe_down()
        wait(0.5)
    wait(1)

    xml = dump_ui("06_before_launch")
    root = parse_ui(xml)
    result = find_by_text(root, "Launch Container", exact=True)
    if result:
        cx, cy, _ = result
        print(f"  Found 'Launch Container' at ({cx}, {cy}) — tapping")
        tap(cx, cy)
        wait(5)
    else:
        print("  ✗ Could not find 'Launch Container' — visible text:")
        print_all_text(root)

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
        # Don't break on "left Render2Activity" — the TWRP screen might
        # be visible even if the activity changed. Keep taking screenshots.
        if "Render2Activity" not in activity and elapsed > 20:
            print(f"  Note: not in Render2Activity at {elapsed}s: {activity}")

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

    # Pull the TWRP init log from the app's rootfs directory. This log
    # is written by the ptrace emulator (kr64) and captures the
    # SIGSYS interceptions, syscall entries, and child exit code —
    # it is the single most useful artifact for diagnosing init boot
    # failures (e.g. "init exits with code 1 after 183 iterations").
    subprocess.run(ADB + ["pull", "/data/user/0/io.twoyi/rootfs/twrp-init.log",
                         os.path.join(ART, "twrp-init.log")],
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
