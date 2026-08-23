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
import os

PACKAGE = os.environ.get("TWOYI_PACKAGE", "io.twoyi")

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

def pull_via_run_as(remote_path, local_path, timeout=10):
    """Read a file from the app's private data dir via `adb shell run-as`.

    On non-rooted devices, `adb pull` cannot read files in
    /data/user/0/io.twoyi/ because they're owned by the app's UID
    (untrusted_app). `run-as io.twoyi` switches to the app's UID, so
    `cat <path>` can read them. The stdout (file contents) is captured
    and written to `local_path` verbatim.

    Returns True if the file was read successfully (non-empty stdout,
    exit code 0), False otherwise (file missing, run-as denied, etc.).
    Like the `adb pull` calls it replaces, failures are silent — these
    logs are diagnostic aids, not required artifacts.
    """
    try:
        r = subprocess.run(ADB + ["shell", "run-as", PACKAGE,
                                  "cat", remote_path],
                           capture_output=True, timeout=timeout)
        if r.returncode == 0 and r.stdout:
            with open(local_path, "wb") as f:
                f.write(r.stdout)
            return True
    except subprocess.TimeoutExpired:
        pass
    return False

def pull_with_fallback(external_path, internal_path, local_path, timeout=10):
    """Pull a diagnostic log via `adb pull` (external mirror), falling
    back to `pull_via_run_as` (app private dir) on failure.

    kr64 mirrors twrp-init.log, twrp-kmsg.log and dev/__kmsg__ to
    /sdcard/Android/data/io.twoyi/files/ once the guest exits. That
    directory is readable via `adb pull` on release builds (where
    `run-as io.twoyi` is rejected and the app's private data dir is
    inaccessible), so we try the external copy first.

    If the external pull fails for any reason (file not yet mirrored,
    partial write, timeout, non-zero exit), any empty/partial local
    file is removed and we fall back to `pull_via_run_as`, which reads
    the original copy under /data/user/0/io.twoyi/rootfs/ via
    `run-as {PACKAGE} cat`. This keeps the script working on debuggable
    builds where run-as still functions, and on devices where the
    external mirror hasn't landed yet.

    Returns True if the file was fetched successfully, False otherwise.
    Like the `adb pull` calls it wraps, failures are silent — these
    logs are diagnostic aids, not required artifacts.
    """
    try:
        r = subprocess.run(ADB + ["pull", external_path, local_path],
                           capture_output=True, timeout=timeout)
        if (r.returncode == 0 and os.path.exists(local_path)
                and os.path.getsize(local_path) > 0):
            return True
    except subprocess.TimeoutExpired:
        pass
    # External pull failed — remove any empty/partial file so the
    # run-as fallback starts from a clean slate.
    if os.path.exists(local_path):
        try:
            os.remove(local_path)
        except OSError:
            pass
    return pull_via_run_as(internal_path, local_path, timeout=timeout)

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
    """Tap at coordinates. PRIMARY method is `input swipe X Y X Y 100`
    — a zero-distance swipe with a 100ms duration. This is the form
    that worked reliably on the Android 11 emulator: `input tap X Y`
    was found to NOT register as a real tap on certain UI elements
    (e.g. preference rows), causing the app to fall through to the
    home screen instead of opening the file picker.

    Falls back to `input tap X Y` only if the swipe returns an error
    (e.g. on devices where `input swipe` is not recognized).

    For file-picker-row taps that need to try MULTIPLE methods in
    sequence (with picker-closed checks between each), use
    `tap_picker_row_with_fallbacks()` instead."""
    r = subprocess.run(ADB + ["shell", f"input swipe {x} {y} {x} {y} 100"],
                       capture_output=True, text=True, timeout=30)
    combined = (r.stdout + r.stderr).lower()
    if any(s in combined for s in ("usage", "unknown", "error", "not found", "invalid")):
        # input swipe not recognized — fall back to input tap
        adb_shell(f"input tap {x} {y}")

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
      6. `input trackball roll 0 1` + `input trackball press` scan —
         IMPORTANT: `input trackball roll X Y` takes RELATIVE deltas,
         not absolute coordinates, so rolling by (x, y) from the origin
         would overshoot the target wildly. Instead we perform an
         incremental scan: roll DOWN by 1 pixel, press the trackball,
         check whether the picker closed, and repeat up to 20 times
         (covering the full 320x640 screen height). Trackball events
         use a different input source (SOURCE_TRACKBALL) than touch
         events; some RecyclerView OnItemTouchListeners that drop
         touch events may still respond to a trackball press.
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
    # Method 6: input trackball scan. NOTE: `input trackball roll X Y`
    # takes RELATIVE deltas, not absolute coordinates, so a single roll
    # to (x, y) from the origin would move the cursor x pixels right and
    # y pixels down — way past the target. Instead, roll DOWN by 1 pixel
    # at a time (`input trackball roll 0 1`), press the trackball after
    # each roll, and check whether the picker has closed. Up to 20 rolls
    # covers the full 320x640 screen height. This is effectively a
    # trackball-based scan: move down one line, click, check if picker
    # closed, repeat.
    print("    tap_picker_row: trying trackball scan")
    probe = subprocess.run(ADB + ["shell", "input trackball roll 0 0"],
                           capture_output=True, text=True, timeout=30)
    probe_out = (probe.stdout + probe.stderr).lower()
    if any(s in probe_out for s in ("usage", "unknown", "error", "not found", "invalid")):
        print("    tap_picker_row: trackball not supported on device — skipping")
    else:
        closed = False
        for i in range(1, 21):
            subprocess.run(ADB + ["shell", "input trackball roll 0 1"],
                           capture_output=True, text=True, timeout=30)
            subprocess.run(ADB + ["shell", "input trackball press"],
                           capture_output=True, text=True, timeout=30)
            wait(1.0)  # give picker time to close (or not)
            if picker_closed():
                print(f"    tap_picker_row: ✓ picker closed after trackball roll #{i}")
                closed = True
                break
        if closed:
            return True
        print("    tap_picker_row: picker still open after 20 trackball rolls")
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
    if PACKAGE not in (activity or "").lower():
        print(f"  ✗ ROM import verification failed: not on {PACKAGE} (activity={activity!r})")
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
    adb_shell(f"monkey -p {PACKAGE} -c android.intent.category.LAUNCHER 1")
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
    # Step 3: Open recovery.img directly with `am start` (bypasses SAF picker)
    #
    # PREVIOUS approach: navigate the Android SAF (Storage Access Framework)
    # file picker (DocumentsUI) via uiautomator dump + DPAD/tap/trackball.
    # This was extremely unreliable across emulator API levels:
    #   - The picker's RecyclerView uses an OnItemTouchListener that does
    #     NOT fire on synthetic `input tap` events (gesture detector requires
    #     a real touch-screen event with proper down/up timing).
    #   - DPAD focus frequently lands on the per-row 'Preview' icon
    #     (focusable=true) and KEYCODE_ENTER triggers a share/open-with
    #     sheet instead of selecting the file.
    #   - The drawer's left-side root list sometimes launches Google Photos
    #     when the wrong root is tapped, requiring BACK-key escape sequences.
    #   - Search-then-DPAD, DPAD-only escalation, direct tap, drawer
    #     navigation, and final coordinate tap were all tried with extensive
    #     retry/escalation logic — see git history (commits b1cd7ac, 0eb432c,
    #     819895e, 372072b, 204d876) for the long tail of failed attempts.
    #
    # NEW approach: bypass the picker UI entirely with `am start -a VIEW`,
    # which is EXACTLY what a file manager does when a user taps a file.
    # SettingsActivity now has an intent filter for ACTION_VIEW with file://
    # and content:// URIs ending in .img/.tar/.cpio/.zip. Its onCreate and
    # onNewIntent (launchMode="singleTask") forward the URI to the SAME
    # importRomForActiveProfile(Uri) method that onActivityResult calls —
    # functionally identical to picking the file in the SAF picker, but
    # without the picker UI getting in the way.
    #
    # If Step 2 (tapping 'Select ROM') happened to open the picker before
    # this runs, the `am start` brings the SettingsActivity task back to
    # the foreground (singleTask), clearing the picker, and onNewIntent is
    # called on the existing activity instance. Either way, the import
    # starts immediately and Step 4 waits for it to complete.
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print("  Step 3: Open recovery.img via `am start` (bypasses SAF picker)")
    print("=" * 60)

    # `am start -a android.intent.action.VIEW -d "file:///sdcard/Download/recovery.img"
    #           -t "*/*" -n io.twoyi/.ui.SettingsActivity`
    #
    # -a VIEW          : the action the SettingsActivity intent filter matches.
    # -d file://...    : the file URI to import (URI is forwarded to
    #                    RamdiskImporter.importRamdisk which opens it via
    #                    ContentResolver.openInputStream — works for both
    #                    file:// and content:// schemes).
    # -t "*/*"         : MIME type — matches the intent filter's mimeType="*/*".
    #                    (Android sometimes infers image/* for .img files,
    #                    which would NOT match — passing "*/*" explicitly
    #                    avoids that.)
    # -n io.twoyi/.ui.SettingsActivity : explicitly target SettingsActivity
    #                    so Android does NOT show a chooser even if other
    #                    apps also match the filter. This makes the test
    #                    deterministic.
    am_cmd = ('am start -a android.intent.action.VIEW '
              '-d "file:///sdcard/Download/recovery.img" '
              '-t "*/*" '
              f'-n {PACKAGE}/io.twoyi.ui.SettingsActivity')
    print(f"  $ adb shell {am_cmd}")
    out = adb_shell(am_cmd)
    if out:
        print(f"  am start output: {out!r}")

    # Give SettingsActivity time to receive the intent (via onCreate for
    # cold-start, or onNewIntent for warm-start), attach the SettingsFragment,
    # and kick off the import (which shows a progress dialog). 5s has been
    # enough in practice; the import itself can take up to 120s and is
    # waited for in Step 4.
    wait(5)

    # Confirm we landed back on SettingsActivity. If Step 2 opened the
    # picker, singleTask should have cleared it. If we're still on the
    # picker (am start failed to match the filter, or the activity didn't
    # come to the foreground), Step 4's verify_rom_imported() will catch
    # it and abort the test.
    xml = dump_ui("04_after_am_start")
    root = parse_ui(xml)
    activity = get_current_activity()
    print(f"  Step 3 current activity: {activity}")

    found_file = PACKAGE in (activity or "").lower()
    if found_file:
        print("  ✓ am start delivered recovery.img to SettingsActivity — import should be running")
    else:
        print(f"  ⚠ Not on {PACKAGE} after am start (activity={activity!r})")
        print(f"     Step 4 will verify whether a ROM was actually imported and abort if not.")

    # Step 4 will wait for the import to complete (up to 120s) and verify
    # the 'Select ROM' summary changed from the default 'Import rootfs ...'
    # prompt to the imported file name (set by importRomForActiveProfile
    # after a successful import).
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
    # continuing to Step 5/6/7. If the `am start -a VIEW` in Step 3 failed
    # to deliver the file URI to SettingsActivity (e.g. intent filter
    # mismatch, SettingsActivity not in foreground after am start, or
    # RamdiskImporter.importRamdisk threw an exception), the SettingsActivity
    # 'Select ROM' summary will still show the default "Import rootfs..."
    # prompt — and Steps 5-7 will produce misleading "No ROM Installed"
    # screenshots that waste CI time. Abort early with a clear error instead.
    print()
    print("  Verifying ROM was actually imported...")
    rom_imported = verify_rom_imported(root)
    if not rom_imported:
        print()
        print("=" * 60)
        print("  ✗✗✗ ABORTING TEST EARLY: No ROM was imported")
        print("=" * 60)
        print("  The `am start -a VIEW` in Step 3 likely failed to deliver")
        print("  recovery.img to SettingsActivity's importRomForActiveProfile.")
        print("  Continuing to Step 5/6/7 would only produce misleading 'No ROM")
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
            subprocess.run(ADB + ["pull", f"/sdcard/Android/data/{PACKAGE}/files/log/",
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
    # ── 6-Z87 FIX 4: feed TWRP input during the wait ──
    # TWRP's UI thread (PagesManager → ev loop) may need INPUT events to
    # advance from the splash to the main package — with zero input events
    # the package switch can stall waiting on its event pipeline. Every
    # 30 s (every 6th iteration of this 5 s loop) tap the screen center
    # (160,320 on the 320x640 AVD). A periodic center tap is harmless on
    # any screen: on the splash it merely wakes the input path, on the
    # TWRP grid the center cell is an empty label, on the app's own UI a
    # stray tap just re-focuses. tap_count lets us log "feeding input
    # tap" only once per minute (every 2nd tap) to keep the console
    # readable; the tap itself still fires every 30 s.
    tap_count = 0
    for i in range(boot_wait // 5):
        wait(5)
        elapsed = (i + 1) * 5
        if elapsed % 30 == 0:
            adb_shell("input tap 160 320")
            tap_count += 1
            if tap_count % 2 == 1:
                print(f"  feeding input tap 160 320 (#{tap_count} at {elapsed}s; next log at +60s)")
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
    subprocess.run(ADB + ["pull", f"/sdcard/Android/data/{PACKAGE}/files/log/",
                         os.path.join(ART, "app-logs/")],
                  capture_output=True, timeout=30)

    # Try to pull kr64 logs
    subprocess.run(ADB + ["pull", f"/data/data/{PACKAGE}/kr64-app-stderr.log",
                         os.path.join(ART, "kr64-app-stderr.log")],
                  capture_output=True, timeout=10)

    # ── 6-Z85: pull the app's FileLogger logs + kr64 stderr via run-as ──
    # These hold the APP-SIDE boot story (waitBoot polling, markCompleted
    # receipts, the TWRP-FB render-loop lifecycle) that logcat never sees
    # on release builds. Works when the debuggable variant is installed
    # (TWOYI_PACKAGE=io.twoyi.debug); silently no-ops otherwise.
    for remote, local in [
        ("cache/log/app.log", "app.log"),
        ("cache/log/boot.log", "boot.log"),
        ("cache/log/crash.log", "crash.log"),
        ("cache/log/logcat.log", "logcat-guest.log"),
        ("kr64-app-stderr.log", "kr64-app-stderr.log"),
    ]:
        out = adb_shell(f"run-as {PACKAGE} cat {remote}", timeout=60)
        if out:
            with open(os.path.join(ART, local), "w", errors="replace") as f:
                f.write(out)
            print(f"  pulled via run-as: {remote} -> {local}")

    # Pull the TWRP diagnostic logs.
    #
    # kr64 mirrors these three files to /sdcard/Android/data/io.twoyi/
    # files/ once the guest exits (see kr64 src/lib.rs around the
    # ptrace emulation loop). That directory is readable via `adb pull`
    # on release builds — where `adb shell run-as io.twoyi` is rejected
    # and the app's private data dir (/data/user/0/io.twoyi/rootfs/)
    # is inaccessible — so we pull from the external mirror first and
    # only fall back to `pull_via_run_as` (run-as cat from the private
    # dir) if the external copy is missing or the pull fails.
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
    #     may still be on disk and contain KLOG output. Mirrored to
    #     the external dir as `dev-__kmsg__` (flat name); missing file
    #     is not an error.
    pull_with_fallback(
        "/sdcard/Android/data/io.twoyi/files/twrp-init.log",
        "/data/user/0/io.twoyi/rootfs/twrp-init.log",
        os.path.join(ART, "twrp-init.log"))
    pull_with_fallback(
        "/sdcard/Android/data/io.twoyi/files/twrp-kmsg.log",
        "/data/user/0/io.twoyi/rootfs/twrp-kmsg.log",
        os.path.join(ART, "twrp-kmsg.log"))
    pull_with_fallback(
        "/sdcard/Android/data/io.twoyi/files/dev-__kmsg__",
        "/data/user/0/io.twoyi/rootfs/dev/__kmsg__",
        os.path.join(ART, "dev-__kmsg__"))

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
