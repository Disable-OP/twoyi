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
    """Tap at coordinates. PRIMARY method is `input tap X Y` — the
    simplest approach, and the one most likely to trigger Android's
    RecyclerView OnItemTouchListener (which is what the SAF file
    picker uses). The previous implementation used `input touchscreen
    swipe X Y X Y 100` as a workaround for a 'short-duration event'
    issue, but that workaround may have prevented the tap from
    registering as a real tap on RecyclerView rows.

    Falls back to `input touchscreen swipe X Y X Y 100` (a real
    touchscreen event, source = SOURCE_TOUCHSCREEN) if `input tap`
    returns an error, and finally to `input swipe X Y X Y 100` if
    the touchscreen source isn't recognized on this device.

    For file-picker-row taps that need to try MULTIPLE methods in
    sequence (with picker-closed checks between each), use
    `tap_picker_row_with_fallbacks()` instead."""
    r = subprocess.run(ADB + ["shell", f"input tap {x} {y}"],
                       capture_output=True, text=True, timeout=30)
    combined = (r.stdout + r.stderr).lower()
    if any(s in combined for s in ("usage", "unknown", "error", "not found", "invalid")):
        # input tap not recognized — fall back to touchscreen swipe
        r2 = subprocess.run(ADB + ["shell", f"input touchscreen swipe {x} {y} {x} {y} 100"],
                            capture_output=True, text=True, timeout=30)
        combined2 = (r2.stdout + r2.stderr).lower()
        if any(s in combined2 for s in ("usage", "unknown", "error", "not found", "invalid")):
            adb_shell(f"input swipe {x} {y} {x} {y} 100")

def touchscreen_tap(x, y):
    """Tap at coordinates using 'input touchscreen tap' — a real touchscreen
    event (source = SOURCE_TOUCHSCREEN), NOT a trackball/keyboard event like
    'input tap'. This is the most reliable way to trigger the RecyclerView's
    OnItemTouchListener in the Android SAF file picker, which sometimes
    drops short 'input tap' events. Falls back to 'input tap' if the
    'touchscreen' source isn't supported on this device."""
    r = subprocess.run(ADB + ["shell", f"input touchscreen tap {x} {y}"],
                       capture_output=True, text=True, timeout=30)
    combined = (r.stdout + r.stderr).lower()
    if any(s in combined for s in ("usage", "unknown", "error", "not found", "invalid")):
        adb_shell(f"input tap {x} {y}")


def tap_picker_row_with_fallbacks(x, y):
    """Tap a file picker row trying MULTIPLE tap methods in sequence,
    checking after each whether the picker closed (file was selected).
    Returns True if the picker closed after one of the methods, False
    if the picker is still open after all methods have been tried.

    This is the lowest-level touch-input approach: it tries `input tap`
    first (the simplest and most reliable for RecyclerView), then falls
    back through increasingly low-level methods. The SAF file picker's
    RecyclerView OnItemTouchListener should respond to at least one of
    these.

    Methods tried, in order:
      1. `input tap X Y` — the simplest approach; a real tap event.
         This is the most likely to trigger the RecyclerView's
         OnItemTouchListener (which `input swipe X Y X Y 100` does
         not always do).
      2. `input touchscreen swipe X Y X Y 100` — a real touchscreen
         event (source = SOURCE_TOUCHSCREEN), 100ms duration.
      3. `input swipe X Y X+1 Y 50` — a tiny 1-pixel horizontal
         movement, 50ms duration; registers as a tap (not a long
         press) while sending a slightly different motion-event
         sequence than (1) or (2).
      4. `input swipe X Y X Y 500` (LONG-PRESS) — a 500ms hold.
         Some RecyclerView OnItemTouchListeners only fire
         onLongPress, not onSingleTapUp; this also tests whether
         the picker responds differently to a long touch.
      5. `input motionevent DOWN/MOVE/UP` sequence — the lowest-
         level touch input API; sends raw MotionEvent actions.
         Available on Android 7+. This bypasses the `input` tool's
         tap/swipe abstractions and injects MotionEvent objects
         directly, which should trigger ANY OnItemTouchListener.
      6. `input trackball roll X Y` + `input trackball press` — moves
         the trackball cursor to the file position (rolling by the
         X/Y deltas) and then "clicks" it with a trackball press.
         Trackball events have a different input source
         (SOURCE_TRACKBALL) than touch events; some RecyclerView
         OnItemTouchListeners that drop touch events may still
         respond to a trackball press.
    """
    # Guard: only run if we're on the file picker. If we've drifted
    # off (e.g., to Google Photos), tapping would do the wrong thing.
    current = get_current_activity()
    if not is_on_file_picker(current):
        print(f"    tap_picker_row: not on file picker (activity={current!r}) — skipping")
        return False

    def picker_closed():
        return not is_on_file_picker(get_current_activity())

    def try_method(name, cmd, is_sequence=False):
        """Run a tap method and check if picker closed. Returns True if
        picker closed, False otherwise (or if method not supported)."""
        print(f"    tap_picker_row: trying {name}")
        if is_sequence:
            # cmd is a list of shell commands to run in sequence
            for sub in cmd:
                r = subprocess.run(ADB + ["shell", sub],
                                   capture_output=True, text=True, timeout=30)
                combined = (r.stdout + r.stderr).lower()
                if any(s in combined for s in ("usage", "unknown", "error", "not found", "invalid")):
                    print(f"    tap_picker_row: {name} not supported on device — skipping")
                    return False
        else:
            r = subprocess.run(ADB + ["shell", cmd],
                               capture_output=True, text=True, timeout=30)
            combined = (r.stdout + r.stderr).lower()
            if any(s in combined for s in ("usage", "unknown", "error", "not found", "invalid")):
                print(f"    tap_picker_row: {name} not supported on device — skipping")
                return False
        wait(1.5)  # give picker time to close (or not)
        if picker_closed():
            print(f"    tap_picker_row: ✓ picker closed after {name}")
            return True
        print(f"    tap_picker_row: picker still open after {name}")
        return False

    # Method 1: input tap
    if try_method(f"input tap {x} {y}", f"input tap {x} {y}"):
        return True
    # Method 2: input touchscreen swipe (same start/end, 100ms)
    if try_method(f"touchscreen swipe {x} {y} {x} {y} 100",
                  f"input touchscreen swipe {x} {y} {x} {y} 100"):
        return True
    # Method 3: tiny horizontal swipe (1px, 50ms)
    if try_method(f"tiny swipe {x} {y} {x+1} {y} 50",
                  f"input swipe {x} {y} {x+1} {y} 50"):
        return True
    # Method 4: long-press (500ms hold)
    if try_method(f"long-press {x} {y} {x} {y} 500",
                  f"input swipe {x} {y} {x} {y} 500"):
        return True
    # Method 5: input motionevent DOWN/MOVE/UP sequence (lowest-level)
    if try_method("motionevent DOWN/MOVE/UP",
                  [f"input motionevent DOWN {x} {y}",
                   f"input motionevent MOVE {x+1} {y}",
                   f"input motionevent UP {x+1} {y}"],
                  is_sequence=True):
        return True
    # Method 6: input trackball roll + press — move the trackball cursor
    # to the file position (rolling by the X/Y deltas from the origin)
    # and then "click" it with a trackball press. Trackball events use a
    # different input source (SOURCE_TRACKBALL) than touch events; some
    # RecyclerView OnItemTouchListeners that drop touch events may still
    # respond to a trackball press.
    if try_method("trackball roll + press",
                  [f"input trackball roll {x} {y}",
                   "input trackball press"],
                  is_sequence=True):
        return True
    print(f"    tap_picker_row: ✗ all 6 tap methods failed to close the picker")
    return False


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

    CRITICAL GUARD: the previous version fell through to an "assume
    imported" fallback when it could not find the 'Select ROM' preference.
    That fallback MASKED real failures — e.g. when the file picker closed
    without a real selection and we ended up on Google Photos (or any
    other non-io.twoyi app), the function returned True and the test
    continued, producing misleading "No ROM Installed" screenshots. The
    fallback is removed. Now:
      - If the current foreground activity is NOT io.twoyi, return False
        immediately (we are not even looking at the Settings screen).
      - If we can't find the 'Select ROM' preference, return False.
    """
    # GUARD: must be on io.twoyi to trust any UI signal. If we're on
    # Google Photos (or anything else), we are NOT on the Settings
    # screen — return False so the test aborts instead of pretending a
    # ROM was imported.
    activity = get_current_activity()
    if "io.twoyi" not in (activity or "").lower():
        print(f"  ✗ ROM import verification failed: not on io.twoyi (activity={activity!r})")
        return False
    if root is None:
        print("  ✗ ROM import verification failed: no UI hierarchy available")
        return False
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
    # Could not find the Select ROM preference at all. Previously this
    # returned True ("assume imported") which masked real failures where
    # the picker closed without a real selection and we ended up on an
    # unrelated screen. Return False so the test aborts early instead of
    # producing misleading "No ROM Installed" screenshots later.
    print("  ✗ Could not find 'Select ROM' preference in UI — returning False (was 'assume imported')")
    return False


def escape_google_photos():
    """Detect if we've accidentally navigated into Google Photos (which
    can happen if the SAF picker's drawer was opened and the wrong root
    was tapped instead of recovery.img) and send BACK multiple times to
    return to the picker.

    Returns True if Google Photos was detected (regardless of whether the
    BACK keys succeeded in escaping), False if we were not on Google
    Photos in the first place. The caller should re-dump the UI and
    retry the picker strategy after this returns True.
    """
    activity = get_current_activity()
    if not activity:
        return False
    a = activity.lower()
    if "photos" not in a and "google.android.apps.photos" not in a:
        return False
    print(f"  ⚠ Detected Google Photos (activity={activity!r}) — sending BACK keys to return to picker")
    for i in range(5):
        adb_shell("input keyevent KEYCODE_BACK")
        wait(1)
        a = (get_current_activity() or "").lower()
        if "photos" not in a and "google.android.apps.photos" not in a:
            print(f"  ✓ Escaped Google Photos after {i+1} BACK key(s) — now on {get_current_activity()!r}")
            return True
    print(f"  ⚠ Still on Google Photos after 5 BACK keys — activity={get_current_activity()!r}")
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
    #   - Opens on the "RECENT FILES" view by default — recovery.img is
    #     ALREADY VISIBLE there if it was recently placed in /sdcard/Download.
    #   - Has a hamburger menu (top-left) → drawer with "Downloads", etc.
    #   - Or has a breadcrumb path bar at the top
    #   - Or shows "Internal storage" / "SD card" options
    #
    # Strategy order — ALL are tried before aborting. The DPAD strategy
    # is PRIMARY because it was previously the most reliable. Trying it
    # FIRST means we usually select recovery.img before any drawer
    # navigation can drift us into Google Photos.
    #   a) COMBINED coordinate-tap + DPAD navigation (PRIMARY).
    #      FIRST: take a fresh uiautomator dump, find recovery.img's
    #      bounds, compute the center of the LEFT 30% of the row, and
    #      `input touchscreen tap X Y` — a real touchscreen event (NOT a
    #      trackball/keyboard event like `input tap`), which reliably
    #      triggers the SAF picker's RecyclerView OnItemTouchListener. If
    #      that closes the picker, done. FALL BACK to DPAD: tap
    #      recovery.img's exact coords to give the row focus, then
    #      DPAD_DOWN + ENTER. Retry with increasing DPAD_DOWN counts
    #      (1 through 10 — a broad range so we land on recovery.img
    #      regardless of how many list items precede it). If the picker
    #      closes but no ROM was imported, re-open it and retry with more
    #      DOWN presses. Dismiss any share sheet before each attempt.
    #   b) Direct tap on recovery.img in RECENT FILES (left-30% of the
    #      row, away from the preview icon). If the tap doesn't close
    #      the picker (the RecyclerView's OnItemTouchListener didn't
    #      fire), fall through to (c) — do NOT retry the same
    #      unresponsive tap.
    #   c) Drawer → Downloads → file. If not in Downloads, try Internal
    #      storage → Download folder → file. Can drift into Google
    #      Photos if the wrong root is tapped — escape and fall through.
    #   d) Coordinate-based tap from a fresh uiautomator dump.
    #
    # CRITICAL: after EACH strategy fails, call escape_google_photos()
    # and attempt to return to the picker (ensure_on_picker) before
    # trying the next strategy. We do NOT break out of the cascade —
    # every strategy gets a chance. Only abort after ALL fail.
    #
    # A tap is only a SUCCESS if the picker closed AND we are back on
    # io.twoyi. Being on Google Photos (or any other non-picker app) is
    # NOT a success. The old code accepted "not on picker" as success,
    # which incorrectly treated accidental Google Photos navigation as
    # a successful file selection.
    #
    # Throughout: if we accidentally navigate into Google Photos (which
    # can happen if the drawer's wrong root is tapped, or a thumbnail
    # row is tapped instead of recovery.img), escape_google_photos()
    # sends BACK multiple times to return to the picker.
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print("  Step 3: Navigate file picker to recovery.img")
    print("=" * 60)

    # Give the picker time to fully load its RecyclerView.
    print("  Waiting 3s for picker to fully load...")
    wait(3)

    # Helper: take a fresh dump, dismiss any share sheet, return (xml, root, activity).
    def fresh_picker_state(tag):
        xml = dump_ui(tag)
        root = parse_ui(xml)
        activity = get_current_activity()
        return xml, root, activity

    # Helper: classify the result of a "selection tap". A tap is only a
    # SUCCESS if the picker closed AND we are back on io.twoyi — being on
    # Google Photos (or any other non-picker app) is NOT a success.
    # Returns one of: "success", "photos", "picker", "other".
    def classify_tap_result(activity_str):
        a = (activity_str or "").lower()
        if "io.twoyi" in a:
            return "success"
        if "photos" in a or "google.android.apps.photos" in a:
            return "photos"
        if is_on_file_picker(activity_str):
            return "picker"
        return "other"

    # Helper: ensure we are on the SAF file picker before running a
    # strategy. Called between strategies after the previous one failed.
    # Escapes Google Photos if needed, then — if we drifted off the
    # picker entirely (e.g. BACK from Photos landed on io.twoyi settings
    # or the home screen) — attempts to recover back to the picker.
    # Returns True if we are on the picker now, False otherwise.
    #
    # Recovery logic (no `am start` — UI taps only):
    #   - If on Google Photos: escape_google_photos() (sends BACK keys).
    #   - If on io.twoyi (picker closed without a real selection): the
    #     picker is gone. Re-open it by re-tapping "Select ROM" via
    #     scroll_to_find (a UI tap, NOT an activity launch).
    #   - If elsewhere (home screen etc.): try a few BACK keys, then
    #     re-open via Select ROM if BACK lands on io.twoyi.
    def ensure_on_picker(tag):
        if escape_google_photos():
            _xml, _root, _act = fresh_picker_state(tag + "_post_escape")
            print(f"  After Google-Photos escape: {_act}")
        activity = get_current_activity()
        if is_on_file_picker(activity):
            return True
        a = (activity or "").lower()
        # If on io.twoyi settings, the picker closed without a selection.
        # Re-open it by re-tapping "Select ROM" (a UI tap, not am start).
        if "io.twoyi" in a:
            print(f"  Picker closed (on {activity!r}) — re-opening by re-tapping 'Select ROM'")
            result = scroll_to_find("Select ROM", max_scrolls=8, exact=True)
            if result:
                cx, cy, _ = result
                print(f"  Re-tapping 'Select ROM' at ({cx}, {cy}) to reopen picker")
                tap(cx, cy)
                wait(4)
                activity = get_current_activity()
                if is_on_file_picker(activity):
                    print(f"  ✓ Picker re-opened (activity={activity!r})")
                    return True
                print(f"  ⚠ Picker did not re-open after re-tap (activity={activity!r})")
            else:
                print(f"  ⚠ Could not find 'Select ROM' to re-open picker")
            return False
        # Somewhere else (home screen, etc.) — try BACK keys to recover.
        for i in range(3):
            print(f"  Not on picker ({activity!r}) — sending BACK to recover (attempt {i+1}/3)")
            adb_shell("input keyevent KEYCODE_BACK")
            wait(1)
            activity = get_current_activity()
            if is_on_file_picker(activity):
                print(f"  ✓ Back on picker after {i+1} BACK key(s)")
                return True
            if "io.twoyi" in (activity or "").lower():
                # BACK landed on io.twoyi — re-open picker via Select ROM.
                print(f"  BACK landed on io.twoyi — re-opening picker")
                result = scroll_to_find("Select ROM", max_scrolls=8, exact=True)
                if result:
                    cx, cy, _ = result
                    tap(cx, cy)
                    wait(4)
                    activity = get_current_activity()
                    if is_on_file_picker(activity):
                        print(f"  ✓ Picker re-opened (activity={activity!r})")
                        return True
                break
        print(f"  ⚠ Could not return to picker — on {activity!r}")
        return False

    # Helper: after the picker has just closed (we're back on io.twoyi),
    # determine whether a ROM was actually imported. The RECENT FILES list
    # contains ui-dump.xml (1st item) and recovery.img (2nd item).
    # Selecting ui-dump.xml closes the picker WITHOUT importing a ROM (it
    # isn't a valid ROM file). Selecting recovery.img starts an import —
    # an import progress dialog appears and the 'Select ROM' summary
    # eventually changes from the default 'Import rootfs ...' prompt.
    #
    # We wait up to ~12s for EITHER signal:
    #   - an import progress dialog (importing/extracting/loading/...) →
    #     import started, OR
    #   - verify_rom_imported() returning True (summary changed) →
    #     import finished (fast or already done).
    # If neither appears, no ROM was imported — re-open the picker (scroll
    # to + tap 'Select ROM') so the next DPAD attempt can retry with more
    # DOWN presses.
    # Returns True if a ROM was (being) imported (caller stops), False
    # otherwise (caller continues to the next attempt; picker re-opened).
    def verify_import_or_reopen(tag):
        progress_words = ["progress", "importing", "extracting", "loading",
                          "please wait", "extract"]
        for i in range(6):  # up to ~12s
            _xml, _root, _act = fresh_picker_state(f"{tag}_{i}")
            if _root is not None:
                for node in _root.iter("node"):
                    txt = (node.get("text", "") or "").lower()
                    if any(w in txt for w in progress_words):
                        print(f"  ✓ Import progress dialog detected — ROM import started")
                        return True
            wait(2)
        # No progress dialog seen — do a final summary check.
        _xml, _root, _act = fresh_picker_state(f"{tag}_final")
        if verify_rom_imported(_root):
            print(f"  ✓ Select ROM summary changed — ROM imported")
            return True
        print(f"  ⚠ No import progress and summary unchanged — no ROM imported")
        print(f"     (likely selected ui-dump.xml, the 1st RECENT item, not recovery.img)")
        print(f"     Re-opening picker to retry with more DPAD_DOWN presses")
        ensure_on_picker(tag + "_reopen")
        return False

    found_file = False

    # Initial dump + dismiss any stray share sheet from the previous step.
    xml, root, activity = fresh_picker_state("03_picker_open")
    print(f"  Current activity: {activity}")
    # In case we somehow already drifted into Google Photos.
    if escape_google_photos():
        xml, root, activity = fresh_picker_state("03_after_initial_escape")
        print(f"  After initial Google-Photos escape: {activity}")
    if dismiss_share_sheet(root):
        xml, root, activity = fresh_picker_state("03_after_initial_dismiss")
        print(f"  After initial share-sheet dismiss: {activity}")

    # ═══════════════════════════════════════════════════════════════
    # (a) DPAD-only navigation — PRIMARY strategy.
    #
    # The RECENT FILES list contains TWO items: ui-dump.xml (1st) and
    # recovery.img (2nd). We press DPAD_DOWN directly from whatever the
    # default focus position is — we do NOT tap the file list first.
    #
    # A previous version tapped the list area (recovery.img's exact
    # coordinates, or (20%, 55%) as a fallback) to "give the row focus"
    # before DPAD navigation, but that tap was putting focus on the
    # WRONG item (ui-dump.xml at position 1 instead of recovery.img at
    # position 2), and then 3x/4x/5x DPAD_DOWN moved focus PAST
    # recovery.img.
    #
    # We try DPAD_DOWN counts of 1, 2, 3 to cover the possible default
    # focus positions:
    #   - 1x DOWN: focus starts on item 1 → lands on item 2 (recovery.img)
    #   - 2x DOWN: focus starts on the header → lands on item 2 (recovery.img)
    #   - 3x DOWN: focus starts one position above the header → wraps or
    #     otherwise lands on item 2 (recovery.img)
    # After each DPAD_DOWN sequence, press ENTER. If the picker closed,
    # verify a ROM was actually imported (not ui-dump.xml, which closes
    # the picker without importing). If no ROM was imported, re-open
    # the picker (via verify_import_or_reopen) and try the next DPAD
    # count.
    #
    # DPAD_ENTER can land on the per-row 'Preview' icon (focusable=true)
    # and trigger a share sheet instead of selecting the file. Mitigate
    # by dismissing any share sheet BEFORE each attempt, and after each
    # ENTER. If we drift to Google Photos, escape_google_photos() sends
    # BACK keys to return to the picker.
    # ═══════════════════════════════════════════════════════════════
    if not found_file:
        ensure_on_picker("03a_pre_dpad")
    if not found_file and is_on_file_picker(get_current_activity()):
        print("  (a) PRIMARY: DPAD-only navigation (no tap before DPAD)")

        for attempt in range(3):
            # If a previous attempt already closed the picker and imported
            # a ROM, skip the remaining DPAD attempts entirely.
            if found_file or not is_on_file_picker(get_current_activity()):
                break
            downs = attempt + 1  # 1, 2, 3
            print(f"  DPAD attempt {attempt+1}/3: dismiss-sheet + {downs}x DPAD_DOWN + ENTER (no pre-tap)")

            # Take a fresh dump and dismiss any share sheet BEFORE the DPAD.
            xml, root, activity = fresh_picker_state(f"03a_dpad_{attempt}_pre")
            if dismiss_share_sheet(root):
                xml, root, activity = fresh_picker_state(f"03a_dpad_{attempt}_post_dismiss")
                print(f"  After pre-DPAD share-sheet dismiss: {activity}")
                # If dismissing the sheet closed the picker too, verify a
                # ROM was actually imported (not a false success from
                # selecting ui-dump.xml). Re-open and retry if not.
                status = classify_tap_result(activity)
                if status == "success":
                    if verify_import_or_reopen(f"03a_dpad_{attempt}_preverify"):
                        found_file = True
                        print(f"  ✓ ROM imported after pre-DPAD share-sheet dismiss")
                        break
                    else:
                        continue  # picker re-opened; try next attempt

            # NO TAP before DPAD — press DPAD_DOWN directly from whatever
            # the default focus position is. A previous version tapped the
            # file list (recovery.img's exact coords, or (20%, 55%) as a
            # fallback) to "give the row focus" before DPAD navigation,
            # but that tap was putting focus on the WRONG item (ui-dump.xml
            # at position 1 instead of recovery.img at position 2), and
            # then 3x/4x/5x DPAD_DOWN moved focus PAST recovery.img.
            for _ in range(downs):
                adb_shell("input keyevent KEYCODE_DPAD_DOWN")
                wait(0.5)
            adb_shell("input keyevent KEYCODE_ENTER")
            wait(3)

            # Verify the picker actually closed AND we're back on io.twoyi.
            a = get_current_activity()
            print(f"  After DPAD attempt {attempt+1}: {a}")
            status = classify_tap_result(a)
            if status == "success":
                # Picker closed — verify a ROM was actually imported. With
                # too few DPAD_DOWNs we may have selected ui-dump.xml (1st
                # RECENT item), which closes the picker without importing.
                if verify_import_or_reopen(f"03a_dpad_{attempt}_verify"):
                    found_file = True
                    print(f"  ✓ ROM import verified after DPAD attempt {attempt+1}")
                    break
                else:
                    # Picker closed but no ROM imported — verify_import_or_reopen
                    # has re-opened the picker. Try next attempt (more DOWNs).
                    continue
            else:
                print(f"  ⚠ Picker still open or drifted (status={status}) after DPAD attempt {attempt+1}")
                if status == "photos":
                    escape_google_photos()
                # A share sheet may have appeared — dismiss before next attempt.
                xml, root, activity = fresh_picker_state(f"03a_dpad_{attempt}_post")
                if dismiss_share_sheet(root):
                    xml, root, activity = fresh_picker_state(f"03a_dpad_{attempt}_post2")
                    if classify_tap_result(activity) == "success":
                        if verify_import_or_reopen(f"03a_dpad_{attempt}_postverify"):
                            found_file = True
                            print(f"  ✓ ROM imported after post-DPAD share-sheet dismiss")
                            break
                        else:
                            continue  # picker re-opened; try next attempt
        # End of DPAD attempts — fall through to (b) regardless of outcome.

    # ═══════════════════════════════════════════════════════════════
    # (b) Direct tap on recovery.img in RECENT FILES. The picker opens
    # with recovery.img already visible here. Retry the dump a few times
    # to give the picker's RecyclerView time to populate. If the tap
    # doesn't close the picker (the RecyclerView's OnItemTouchListener
    # didn't fire), fall through to (c) drawer navigation — do NOT retry
    # the same unresponsive tap.
    # ═══════════════════════════════════════════════════════════════
    if not found_file:
        ensure_on_picker("03b_pre_tap")
    if not found_file and is_on_file_picker(get_current_activity()):
        print("  (b) Looking for recovery.img in current view (RECENT FILES) — direct tap")
        for dump_attempt in range(3):
            if dump_attempt > 0:
                print(f"  recovery.img not visible yet — waiting 2s and re-dumping (attempt {dump_attempt+1}/3)")
                wait(2)
                xml, root, activity = fresh_picker_state(f"03b_recent_retry_{dump_attempt}")
            tap_outcome = None  # "success" | "photos" | "picker_or_other" | "not_found"
            for text in ["recovery.img", "recovery", "byt_t", "twrp"]:
                result = safe_row_tap_target(root, text, exact=False)
                if not result:
                    continue
                cx, cy, _ = result
                print(f"  Found '{text}' — safe row tap at ({cx}, {cy}) — tapping (tap_picker_row_with_fallbacks)")
                tap_picker_row_with_fallbacks(cx, cy)
                wait(3)
                a = get_current_activity()
                status = classify_tap_result(a)
                print(f"  Tap result: {status} (activity={a!r})")
                if status == "success":
                    found_file = True
                    print(f"  ✓ Picker closed and back on io.twoyi — file selected")
                    tap_outcome = "success"
                    break
                elif status == "photos":
                    # Tap accidentally opened Google Photos instead of selecting
                    # the file — escape and retry the direct tap.
                    print(f"  ⚠ Tap opened Google Photos instead of selecting file — escaping")
                    escape_google_photos()
                    xml, root, activity = fresh_picker_state(f"03b_after_photos_escape_{dump_attempt}")
                    tap_outcome = "photos"
                    break  # break inner loop; outer loop will re-dump and retry
                else:
                    # "picker" (still on picker — tap missed) or "other".
                    print(f"  ⚠ Picker still open or drifted (status={status}) after direct tap")
                    if dismiss_share_sheet(root):
                        xml, root, activity = fresh_picker_state("03b_after_tap_dismiss")
                    tap_outcome = "picker_or_other"
                    break  # break inner loop; fall through to (c)
            if found_file:
                break
            if tap_outcome == "picker_or_other":
                # Tap was sent but picker didn't close — the RecyclerView
                # isn't responding to taps. Fall through to (c) drawer
                # navigation instead of retrying the same unresponsive tap.
                break
            # tap_outcome == "not_found" or "photos": retry the dump
            # (outer loop) to give the picker more time to load OR to
            # retry after escaping Photos.
        # End of direct-tap attempts — fall through to (c).

    # ═══════════════════════════════════════════════════════════════
    # (c) Drawer → Downloads → file. If not in Downloads, try Internal
    # storage → Download folder → file. Can drift into Google Photos if
    # the wrong root is tapped — escape and fall through to (d).
    # ═══════════════════════════════════════════════════════════════
    if not found_file:
        ensure_on_picker("03c_pre_drawer")
    if not found_file and is_on_file_picker(get_current_activity()):
        print("  (c) Opening nav drawer → Downloads → recovery.img")
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

        xml, root, activity = fresh_picker_state("03c_drawer_open")
        print(f"  Current activity: {activity}")
        print("  Visible text after drawer tap:")
        print_all_text(root, prefix="    ")

        # If the drawer tap accidentally launched Google Photos, escape.
        if escape_google_photos():
            xml, root, activity = fresh_picker_state("03c_drawer_after_photos_escape")
            print(f"  After drawer Google-Photos escape: {activity}")

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
        xml, root, activity = fresh_picker_state("03c_in_downloads")
        print("  Visible text in Downloads:")
        print_all_text(root, prefix="    ")

        for text in ["recovery.img", "recovery", "byt_t", "twrp"]:
            result = safe_row_tap_target(root, text, exact=False)
            if not result:
                continue
            cx, cy, _ = result
            print(f"  Found '{text}' — safe row tap at ({cx}, {cy}) — tapping (tap_picker_row_with_fallbacks)")
            tap_picker_row_with_fallbacks(cx, cy)
            wait(3)
            a = get_current_activity()
            status = classify_tap_result(a)
            print(f"  Tap result: {status} (activity={a!r})")
            if status == "success":
                found_file = True
                print(f"  ✓ Picker closed after Downloads tap — file selected")
                break
            elif status == "photos":
                print(f"  ⚠ Tap opened Google Photos — escaping")
                escape_google_photos()
                xml, root, activity = fresh_picker_state("03c_after_photos_escape")
                break
            else:
                print(f"  ⚠ Picker still open after Downloads tap (status={status})")
                if dismiss_share_sheet(root):
                    xml, root, activity = fresh_picker_state("03c_after_dismiss")
                break

        # If still not found, try "Internal storage" → Download folder
        if not found_file and is_on_file_picker(get_current_activity()):
            print("  (c-cont) recovery.img not in Downloads — trying internal storage")
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

            xml, root, activity = fresh_picker_state("03c_in_download_folder")
            for text in ["recovery.img", "recovery", "byt_t", "twrp"]:
                result = safe_row_tap_target(root, text, exact=False)
                if not result:
                    continue
                cx, cy, _ = result
                print(f"  Found '{text}' — safe row tap at ({cx}, {cy}) — tapping (tap_picker_row_with_fallbacks)")
                tap_picker_row_with_fallbacks(cx, cy)
                wait(3)
                a = get_current_activity()
                status = classify_tap_result(a)
                print(f"  Tap result: {status} (activity={a!r})")
                if status == "success":
                    found_file = True
                    print(f"  ✓ Picker closed after internal-storage tap — file selected")
                    break
                elif status == "photos":
                    print(f"  ⚠ Tap opened Google Photos — escaping")
                    escape_google_photos()
                    xml, root, activity = fresh_picker_state("03c_internal_after_photos_escape")
                    break
                else:
                    if dismiss_share_sheet(root):
                        xml, root, activity = fresh_picker_state("03c_internal_after_dismiss")
                    break
        # End of drawer strategy — fall through to (d).

    # ═══════════════════════════════════════════════════════════════
    # (d) Final fallback: coordinate-based tap from a fresh uiautomator
    # dump. If DPAD and taps all failed (e.g., focus kept landing on the
    # preview icon), the file is still visible in the picker — take a
    # fresh dump and tap the row's left-30% coordinate directly, which
    # is the most reliable way to trigger the RecyclerView's
    # OnItemTouchListener.
    # ═══════════════════════════════════════════════════════════════
    if not found_file:
        ensure_on_picker("03d_pre_final")
    if not found_file and is_on_file_picker(get_current_activity()):
        print("  (d) Coordinate-based tap from fresh uiautomator dump")
        xml, root, activity = fresh_picker_state("03d_final_dump")
        for text in ["recovery.img", "recovery", "byt_t", "twrp"]:
            result = safe_row_tap_target(root, text, exact=False)
            if not result:
                continue
            cx, cy, _ = result
            print(f"  Final attempt: tapping '{text}' at ({cx}, {cy}) (tap_picker_row_with_fallbacks)")
            tap_picker_row_with_fallbacks(cx, cy)
            wait(3)
            a = get_current_activity()
            status = classify_tap_result(a)
            print(f"  Tap result: {status} (activity={a!r})")
            if status == "success":
                found_file = True
                print(f"  ✓ Picker closed after final coordinate tap")
                break
            else:
                print(f"  ⚠ Picker still open or drifted (status={status}) after final coordinate tap")
                if status == "photos":
                    escape_google_photos()
                if dismiss_share_sheet(root):
                    xml, root, activity = fresh_picker_state("03d_final_after_dismiss")
                    if classify_tap_result(activity) == "success":
                        found_file = True
                        print(f"  ✓ Picker closed after final share-sheet dismiss")
                        break
        # End of coordinate strategy — all strategies exhausted.

    # Final status report for Step 3.
    xml = dump_ui("04_after_file_select")
    root = parse_ui(xml)
    activity = get_current_activity()
    print(f"  Step 3 final activity: {activity}")
    if is_on_file_picker(activity):
        print("  ✗✗✗ FILE PICKER STILL OPEN — could not select recovery.img")
        print("  All strategies tried: (a) DPAD, (b) direct tap, (c) drawer, (d) coordinate.")
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
