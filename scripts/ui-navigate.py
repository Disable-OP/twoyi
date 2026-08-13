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

def is_on_file_picker(activity_str):
    """Return True if the given activity string indicates we're still on
    the SAF file picker (DocumentsUI) or any other picker activity."""
    if not activity_str:
        return False
    a = activity_str.lower()
    return "documentsui" in a or "picker" in a

def dismiss_share_sheet(root):
    """Detect Android's share sheet / 'Complete action using' dialog / 'Open with'
    dialog and dismiss it with BACK. Returns True if a dialog was detected
    and dismissed, False otherwise.

    This is critical: the SAF file picker's per-row 'Preview' icon is
    focusable, and a DPAD_ENTER on it opens a share/preview sheet instead
    of selecting the file. If we don't dismiss the sheet before retrying,
    subsequent DPAD keyevents go to the sheet (not the picker) and the
    test spirals into unrelated UI.

    IMPORTANT: the markers are intentionally SPECIFIC. We do NOT match
    on generic words like 'share' or 'preview' alone — those would
    false-match the picker's own 'Preview the file X' content-desc
    (a normal picker UI element, not a share sheet) and cause us to
    press BACK on the picker itself, closing it. We only match phrases
    that appear EXCLUSIVELY on a share/open-with sheet."""
    if root is None:
        return False
    markers = [
        "complete action using",
        "open with",
        "share via",
        "share to",
        "send to",
        "just once",
        "share & send",
    ]
    for node in root.iter("node"):
        txt = (node.get("text", "") or "").lower()
        desc = (node.get("content-desc", "") or "").lower()
        for marker in markers:
            if marker in txt or marker in desc:
                print(f"  Detected share/open-with sheet (marker='{marker}') — pressing BACK")
                adb_shell("input keyevent KEYCODE_BACK")
                wait(1)
                return True
    return False

def safe_row_tap_target(root, text, exact=False):
    """Like find_tap_target_for_text(), but returns a tap coordinate that
    is intentionally offset to the LEFT 30% of the row — well away from
    the per-row 'Preview' icon that sits at the right edge of every SAF
    picker row. Tapping the preview icon opens a share/preview sheet
    instead of selecting the file; tapping the left/center of the row
    triggers the RecyclerView's OnItemTouchListener which selects the file.

    Returns (cx, cy, node) or None.
    """
    if root is None:
        return None
    parent_map = {c: p for p in root.iter() for c in p}
    search = text.lower()
    for node in root.iter("node"):
        txt = (node.get("text", "") or "").lower()
        desc = (node.get("content-desc", "") or "").lower()
        match = (txt == search) if exact else (search in txt)
        if not match:
            match = (desc == search) if exact else (search in desc)
        if match:
            # Walk up parent chain to find the widest ancestor (the row container)
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
            bounds = widest_node.get("bounds", "")
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
            if not m:
                # Fallback to node's own bounds
                bounds = node.get("bounds", "")
                m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
                if not m:
                    continue
            x1, y1, x2, y2 = int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4))
            # Use LEFT 30% of the row for the x-coordinate, vertical center.
            # This avoids the preview_icon at the right edge (~last 25%).
            safe_x = x1 + int((x2 - x1) * 0.30)
            safe_y = (y1 + y2) // 2
            # Clamp to screen
            safe_x = max(20, min(safe_x, SCREEN_W - 20))
            safe_y = max(20, min(safe_y, SCREEN_H - 20))
            return (safe_x, safe_y, widest_node)
    return None

def verify_rom_imported(root):
    """Check whether a ROM appears to have been imported by inspecting the
    SettingsActivity UI hierarchy. Returns True if a ROM appears imported,
    False otherwise.

    Two signals indicate NO ROM was imported:
      1. The 'Select ROM' preference's summary still contains the import
         prompt text ('Import rootfs', 'Import a', etc.) — once a ROM is
         imported, the summary changes to show the file name.
      2. Any node contains 'No ROM Installed' or similar text.
    """
    if root is None:
        # Conservative: assume imported (let the test continue) — but this
        # should not normally happen because we always dump_ui first.
        return True
    # Check for explicit "No ROM" markers
    for node in root.iter("node"):
        txt = (node.get("text", "") or "").lower()
        desc = (node.get("content-desc", "") or "").lower()
        for marker in ["no rom installed", "no rom", "no roms"]:
            if marker in txt or marker in desc:
                print(f"  ✗ ROM import verification failed: found '{marker}' (text={txt!r}, desc={desc!r})")
                return False
    # Check the 'Select ROM' preference summary — if it still says
    # 'Import rootfs ...', no ROM was imported.
    parent_map = {c: p for p in root.iter() for c in p}
    for node in root.iter("node"):
        if node.get("resource-id", "") != "android:id/title":
            continue
        txt = (node.get("text", "") or "").lower()
        if "select rom" not in txt:
            continue
        # Found the 'Select ROM' title node — look at sibling summary
        parent = parent_map.get(node)
        if parent is None:
            continue
        for sib in parent:
            if sib.get("resource-id", "") == "android:id/summary":
                summary = (sib.get("text", "") or "").lower()
                print(f"  'Select ROM' summary: {summary!r}")
                # If the summary still mentions importing rootfs, no ROM is imported
                if "import rootfs" in summary or "import a " in summary or "import a." in summary:
                    print("  ✗ Select ROM summary still shows import prompt — NO ROM imported")
                    return False
                print("  ✓ Select ROM summary changed from default import prompt — ROM appears imported")
                return True
    # If we can't find the Select ROM preference at all, be conservative
    print("  ⚠ Could not find 'Select ROM' preference in UI — assuming ROM imported (no signal either way)")
    return True

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

    # CRITICAL FIX (round-76): the previous run failed because:
    #   1. The SAF picker's "Recent" view sometimes shows "No items" briefly
    #      before populating, so a tap fired too early hits nothing.
    #   2. The DPAD fallback's KEYCODE_ENTER landed on the per-row 'Preview'
    #      icon (focusable=true) instead of the row itself, which triggered
    #      Android's share/open-with sheet — closing the picker without
    #      selecting a file. The test then continued thinking a ROM was
    #      imported, but SettingsActivity still showed "No ROM Installed".
    #
    # Robustness fixes applied below:
    #   - 3s initial wait (was 2s) so the picker's RecyclerView fully loads.
    #   - All per-row taps use safe_row_tap_target() which targets the LEFT
    #     30% of the row, avoiding the 'Preview' icon at the right edge.
    #   - Share/open-with sheets are detected and dismissed with BACK before
    #     every DPAD attempt and between attempts.
    #   - DPAD retry loop: 3 attempts with INCREASING DPAD_DOWN counts
    #     (1, 2, 3). After each ENTER we verify the picker actually closed.
    #   - Coordinate-based fallback: if DPAD fails, take a fresh uiautomator
    #     dump and tap the file row's left-30% coordinate directly.
    #   - found_file is only set TRUE when the picker actually closes
    #     (verified via is_on_file_picker()), not just when a tap is sent.

    # Give the picker time to fully load its RecyclerView.
    print("  Waiting 3s for picker to fully load...")
    wait(3)

    # Helper: take a fresh dump, dismiss any share sheet, return (xml, root, activity).
    def fresh_picker_state(tag):
        xml = dump_ui(tag)
        root = parse_ui(xml)
        activity = get_current_activity()
        return xml, root, activity

    # Initial dump + dismiss any stray share sheet from the previous step.
    xml, root, activity = fresh_picker_state("03_picker_open")
    print(f"  Current activity: {activity}")
    if dismiss_share_sheet(root):
        xml, root, activity = fresh_picker_state("03_after_initial_dismiss")
        print(f"  After initial share-sheet dismiss: {activity}")

    found_file = False

    # (a) Try to find recovery.img directly in the current view (Recent).
    # Use safe_row_tap_target() to avoid the per-row 'Preview' icon.
    print("  (a) Looking for recovery.img in current view (Recent)")
    for text in ["recovery.img", "recovery"]:
        result = safe_row_tap_target(root, text, exact=False)
        if result:
            cx, cy, _ = result
            print(f"  Found '{text}' — safe row tap at ({cx}, {cy}) — tapping")
            tap(cx, cy)
            wait(3)
            # VERIFY the picker actually closed before trusting the tap.
            a = get_current_activity()
            if not is_on_file_picker(a):
                found_file = True
                print(f"  ✓ Picker closed after direct tap — file selected")
                break
            else:
                print(f"  ⚠ Picker still open after direct tap — trying next strategy")
                # A share/preview sheet may have appeared — dismiss and re-dump.
                if dismiss_share_sheet(root):
                    pass
                xml, root, activity = fresh_picker_state("03_after_tap_retry")
                if dismiss_share_sheet(root):
                    xml, root, activity = fresh_picker_state("03_after_tap_dismiss")
                break  # fall through to drawer navigation

    # (b) If direct tap didn't work, open the hamburger drawer → Downloads.
    if not found_file and is_on_file_picker(get_current_activity()):
        print("  (b) recovery.img not in current view — opening nav drawer")
        # Look for the "Show Roots" / "Navigate up" button
        drawer_opened = False
        for desc in ["Show roots", "Navigate up", "Show list", "More options"]:
            result = find_by_text(root, desc, exact=False)
            if result:
                cx, cy, _ = result
                print(f"  Found '{desc}' at ({cx}, {cy}) — tapping to open drawer")
                tap(cx, cy)
                wait(3)  # was 2s — increased to 3s for drawer animation
                drawer_opened = True
                break

        if not drawer_opened:
            hamburger_x = int(SCREEN_W * 0.08)
            hamburger_y = int(SCREEN_H * 0.08)
            print(f"  No drawer button found — tapping top-left ({hamburger_x}, {hamburger_y})")
            tap(hamburger_x, hamburger_y)
            wait(3)  # was 2s

        xml, root, activity = fresh_picker_state("03_drawer_open")
        print(f"  Current activity: {activity}")
        print("  Visible text after drawer tap:")
        print_all_text(root, prefix="    ")

        # Look for "Downloads" in the drawer
        for text in ["Downloads", "Download"]:
            result = safe_row_tap_target(root, text, exact=False)
            if result:
                cx, cy, _ = result
                print(f"  Found '{text}' — safe row tap at ({cx}, {cy}) — tapping")
                tap(cx, cy)
                wait(3)
                break

        # Now look for recovery.img in the Downloads file list
        xml, root, activity = fresh_picker_state("03b_in_downloads")
        print("  Visible text in Downloads:")
        print_all_text(root, prefix="    ")

        for text in ["recovery.img", "recovery", "byt_t", "twrp"]:
            result = safe_row_tap_target(root, text, exact=False)
            if result:
                cx, cy, _ = result
                print(f"  Found '{text}' — safe row tap at ({cx}, {cy}) — tapping")
                tap(cx, cy)
                wait(3)
                a = get_current_activity()
                if not is_on_file_picker(a):
                    found_file = True
                    print(f"  ✓ Picker closed after Downloads tap — file selected")
                    break
                else:
                    print(f"  ⚠ Picker still open after Downloads tap")
                    if dismiss_share_sheet(root):
                        xml, root, activity = fresh_picker_state("03b_after_dismiss")
                    break

        # (c) If still not found, try "Internal storage" → Download folder
        if not found_file and is_on_file_picker(get_current_activity()):
            print("  (c) recovery.img not in Downloads — trying internal storage")
            for text in ["Internal storage", "Phone", "SD card", "Storage"]:
                result = safe_row_tap_target(root, text, exact=False)
                if result:
                    cx, cy, _ = result
                    print(f"  Found '{text}' — safe row tap at ({cx}, {cy}) — tapping")
                    tap(cx, cy)
                    wait(3)  # was 2s
                    break

            xml, root, activity = fresh_picker_state("03c_internal_storage")
            for text in ["Download", "Downloads"]:
                result = safe_row_tap_target(root, text, exact=False)
                if result:
                    cx, cy, _ = result
                    print(f"  Found '{text}' — safe row tap at ({cx}, {cy}) — tapping")
                    tap(cx, cy)
                    wait(3)  # was 2s
                    break

            xml, root, activity = fresh_picker_state("03d_in_download_folder")
            for text in ["recovery.img", "recovery", "byt_t", "twrp"]:
                result = safe_row_tap_target(root, text, exact=False)
                if result:
                    cx, cy, _ = result
                    print(f"  Found '{text}' — safe row tap at ({cx}, {cy}) — tapping")
                    tap(cx, cy)
                    wait(3)
                    a = get_current_activity()
                    if not is_on_file_picker(a):
                        found_file = True
                        print(f"  ✓ Picker closed after internal-storage tap — file selected")
                        break
                    else:
                        if dismiss_share_sheet(root):
                            xml, root, activity = fresh_picker_state("03d_after_dismiss")
                        break

    # (d) DPAD fallback with retry loop — only if picker is still open.
    # This is the most fragile strategy: DPAD_ENTER can land on the
    # per-row 'Preview' icon (focusable=true) and trigger a share sheet
    # instead of selecting the file. We mitigate by:
    #   - Dismissing any share sheet BEFORE each DPAD attempt.
    #   - Tapping the LEFT side of the file list (away from preview icons)
    #     to give the row focus, not the preview icon.
    #   - Using INCREASING DPAD_DOWN counts (1, 2, 3) across 3 attempts.
    #   - Verifying the picker closed after each ENTER; if a share sheet
    #     appeared instead, dismiss it and try the next attempt.
    if not found_file and is_on_file_picker(get_current_activity()):
        print("  (d) Direct taps failed — trying DPAD navigation with retry loop")
        for attempt in range(3):
            downs = attempt + 1  # 1, 2, 3
            print(f"  DPAD attempt {attempt+1}/3: dismiss-sheet + tap-list + {downs}x DPAD_DOWN + ENTER")

            # Take a fresh dump and dismiss any share sheet BEFORE the DPAD.
            xml, root, activity = fresh_picker_state(f"03_dpad_{attempt}_pre")
            if dismiss_share_sheet(root):
                xml, root, activity = fresh_picker_state(f"03_dpad_{attempt}_post_dismiss")
                print(f"  After pre-DPAD share-sheet dismiss: {activity}")
                # If dismissing the sheet closed the picker too, we're done.
                if not is_on_file_picker(activity):
                    found_file = True
                    print(f"  ✓ Picker closed after pre-DPAD share-sheet dismiss")
                    break

            # Tap the LEFT side of the file list area to give the row focus.
            # CRITICAL: do NOT tap the center/right — the 'Preview' icon lives
            # at the right edge and would steal focus.
            list_tap_x = int(SCREEN_W * 0.20)
            list_tap_y = int(SCREEN_H * 0.55)
            print(f"  Tapping file list left side ({list_tap_x}, {list_tap_y}) to give row focus")
            tap(list_tap_x, list_tap_y)
            wait(1)

            # DPAD navigation
            for _ in range(downs):
                adb_shell("input keyevent KEYCODE_DPAD_DOWN")
                wait(0.5)
            adb_shell("input keyevent KEYCODE_ENTER")
            wait(3)

            # Verify the picker actually closed.
            a = get_current_activity()
            print(f"  After DPAD attempt {attempt+1}: {a}")
            if not is_on_file_picker(a):
                found_file = True
                print(f"  ✓ Picker closed after DPAD attempt {attempt+1}")
                break
            else:
                print(f"  ⚠ Picker still open after DPAD attempt {attempt+1}")
                # A share sheet may have appeared — dismiss before next attempt.
                xml, root, activity = fresh_picker_state(f"03_dpad_{attempt}_post")
                if dismiss_share_sheet(root):
                    xml, root, activity = fresh_picker_state(f"03_dpad_{attempt}_post2")
                    if not is_on_file_picker(activity):
                        found_file = True
                        print(f"  ✓ Picker closed after post-DPAD share-sheet dismiss")
                        break

    # (e) Final fallback: coordinate-based tap from a fresh uiautomator dump.
    # If DPAD failed (e.g., focus kept landing on the preview icon), the
    # file is still visible in the picker — take a fresh dump and tap the
    # row's left-30% coordinate directly, which is the most reliable way
    # to trigger the RecyclerView's OnItemTouchListener.
    if not found_file and is_on_file_picker(get_current_activity()):
        print("  (e) DPAD failed — trying coordinate-based tap from fresh uiautomator dump")
        xml, root, activity = fresh_picker_state("03_final_dump")
        for text in ["recovery.img", "recovery", "byt_t", "twrp"]:
            result = safe_row_tap_target(root, text, exact=False)
            if result:
                cx, cy, _ = result
                print(f"  Final attempt: tapping '{text}' at ({cx}, {cy})")
                tap(cx, cy)
                wait(3)
                a = get_current_activity()
                if not is_on_file_picker(a):
                    found_file = True
                    print(f"  ✓ Picker closed after final coordinate tap")
                    break
                else:
                    print(f"  ⚠ Picker still open after final coordinate tap")
                    if dismiss_share_sheet(root):
                        xml, root, activity = fresh_picker_state("03_final_after_dismiss")
                        if not is_on_file_picker(activity):
                            found_file = True
                            print(f"  ✓ Picker closed after final share-sheet dismiss")
                            break

    # Final status report for Step 3.
    xml = dump_ui("04_after_file_select")
    root = parse_ui(xml)
    activity = get_current_activity()
    print(f"  Step 3 final activity: {activity}")
    if is_on_file_picker(activity):
        print("  ✗✗✗ FILE PICKER STILL OPEN — could not select recovery.img")
        print("  Step 4 import verification will catch this and abort the test.")
        print("  Visible text on picker:")
        print_all_text(root, prefix="    ")
    elif found_file:
        print("  ✓ File selection succeeded — picker closed.")
    else:
        print("  ⚠ Picker closed but selection was not explicitly confirmed (no found_file flag).")
        print("  Step 4 will verify whether a ROM was actually imported.")

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

    # CRITICAL FIX (round-76): verify a ROM was actually imported before
    # continuing to Step 5/6/7. If the file picker navigation failed
    # silently (picker closed via BACK or share-sheet dismissal instead
    # of a real selection), SettingsActivity will still show "No ROM
    # Installed" / the default "Import rootfs..." prompt — and Steps 5-7
    # will produce misleading "No ROM Installed" screenshots that waste
    # CI time. Abort early with a clear error instead.
    print()
    print("  Verifying ROM was actually imported...")
    rom_imported = verify_rom_imported(root)
    if not rom_imported:
        print()
        print("=" * 60)
        print("  ✗✗✗ ABORTING TEST EARLY: No ROM was imported")
        print("=" * 60)
        print("  The file picker navigation in Step 3 likely failed silently")
        print("  (picker closed without a real file selection). Continuing")
        print("  to Step 5/6/7 would only produce misleading 'No ROM")
        print("  Installed' screenshots — aborting instead.")
        print()
        # Capture diagnostic artifacts before exiting.
        screenshot("08_abort_no_rom")
        logcat = adb("logcat", "-d", timeout=15)
        with open(os.path.join(ART, "logcat.txt"), "w") as f:
            f.write(logcat)
        # Pull app logs too — they may show why the import didn't start.
        try:
            os.makedirs(os.path.join(ART, "app-logs"), exist_ok=True)
            subprocess.run(ADB + ["pull", "/sdcard/Android/data/io.twoyi/files/log/",
                                 os.path.join(ART, "app-logs/")],
                          capture_output=True, timeout=30)
        except Exception:
            pass
        print("  Diagnostic artifacts captured. Exiting with code 1.")
        sys.exit(1)
    print("  ✓ ROM import verified — continuing to Step 5.")

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

    # Pull the TWRP diagnostic logs from the app's rootfs directory.
    #
    # All three pulls use `check=False` (the default for subprocess.run),
    # which is the Python equivalent of `adb pull ... || true` — if the
    # source file does not exist (e.g. kr64 never created it, or TWRP
    # init's unlink() already removed it by the time we pull), adb
    # returns a non-zero exit code that is captured by capture_output
    # and silently swallowed. This is intentional: these logs are
    # diagnostic aids, not required artifacts.
    #
    #   - twrp-init.log: written by the ptrace emulator (kr64). Captures
    #     SIGSYS interceptions, syscall entries, and child exit code.
    #     The single most useful artifact for diagnosing init boot
    #     failures (e.g. "init exits with code 1 after 183 iterations").
    #
    #   - twrp-kmsg.log: captures kernel-side KLOG messages written by
    #     TWRP init via the /dev/__kmsg__ -> /twrp-kmsg.log symlink
    #     (root mode) or directly to /dev/__kmsg__ (non-root mode).
    #     Complements twrp-init.log by giving the kernel-side view of
    #     what init was doing right before it died.
    #
    #   - dev/__kmsg__: in non-root mode kr64 creates this as a regular
    #     file (empty, mode 0666) because the host /dev is read-only.
    #     TWRP init's log_init() opens it and writes KLOG messages
    #     here. TWRP init later unlink()s the file, but the open fd
    #     keeps the inode alive — if we pull before kr64 SIGKILLs init
    #     (or if the unlink is intercepted by the loader), the file
    #     may still be on disk and contain KLOG output. Pulled best-
    #     effort; missing file is not an error.
    subprocess.run(ADB + ["pull", "/data/user/0/io.twoyi/rootfs/twrp-init.log",
                         os.path.join(ART, "twrp-init.log")],
                  capture_output=True, timeout=10)
    subprocess.run(ADB + ["pull", "/data/user/0/io.twoyi/rootfs/twrp-kmsg.log",
                         os.path.join(ART, "twrp-kmsg.log")],
                  capture_output=True, timeout=10)
    subprocess.run(ADB + ["pull", "/data/user/0/io.twoyi/rootfs/dev/__kmsg__",
                         os.path.join(ART, "dev-__kmsg__")],
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
