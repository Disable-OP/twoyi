#!/usr/bin/env python3
"""
UI navigation script for twoyi E2E test.

Simulates a real user navigating the app entirely via UI taps:
  1. Launch app via monkey (taps launcher icon)
  2. Scroll down to find "Select ROM" preference
  3. Tap it → file picker opens
  4. Navigate file picker to /sdcard/Download/ → tap recovery.img
  5. Wait for import to complete
  6. Scroll to find "Boot to Recovery" checkbox → VERIFY its state (auto-set by app — 6-Z209b: do NOT force-enable for non-TWRP recoveries)
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

PACKAGE = os.environ.get("TWOYI_PACKAGE", "io.twoyi")

# ADB serial is configurable via env var so the same script works for:
#   - x86_64 Android emulator (serial "emulator-5554")
#   - redroid Docker container on arm64 runner (serial "localhost:5555")
# Default keeps the x86_64 path working without any env changes.
ADB = ["adb", "-s", os.environ.get("ADB_SERIAL", "emulator-5554")]
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

def input_cmd(cmd):
    """6-Z186 iter-8: run an `input ...` shell command on the device.

    Channel ladder:
      1. adb (reviving it once with `adb connect` if dead — redroid's
         adbd usually just needs a reconnect).
      2. `sudo docker exec redroid sh -c '<cmd>'` (works most runs, but
         silently no-op'd for a whole run in 33108190303 — hence 3).
      3. sendevent to the touchscreen evdev node (kernel-level; cannot
         silently fail — it writes the exact protocol-B event sequence
         EventHub injects).

    Supported by the sendevent path: `input tap X Y`,
    `input swipe X1 Y1 X2 Y2 DUR` (duration honored by interpolating
    moves), `input keyevent CODE` (via the keyboard node if present,
    else falls back to `input`)."""
    global _ADB_DEAD, _TOUCH_EVDEV, _TOUCH_RANGE, _KEY_EVDEV, _FORCE_SENDEVENT
    if _FORCE_SENDEVENT:
        # Escalated mode (effect-based: an input round showed zero
        # effect). docker `input` is presumed dead-silent — go straight
        # to the app-level broadcast channel, then sendevent.
        r = _broadcast_cmd(cmd)
        if r is not None:
            return r
        return _sendevent_cmd(cmd)
    if not _ADB_DEAD:
        try:
            st = subprocess.run(ADB + ["get-state"], capture_output=True,
                                text=True, timeout=5)
            alive = "device" in (st.stdout + st.stderr)
        except Exception:
            alive = False
        if alive:
            return adb_shell(cmd, timeout=20) or ""
        _ADB_DEAD = True
        print("  [input] adb dead (get-state) — attempting adb connect revival")
        # Revival attempt: adbd inside redroid usually restarts; the
        # serial is host:port so `adb connect` is the whole fix.
        serial = os.environ.get("ADB_SERIAL", "emulator-5554")
        if ":" in serial:
            try:
                r = subprocess.run(["adb", "connect", serial],
                                   capture_output=True, text=True, timeout=10)
                if "connected" in (r.stdout + r.stderr):
                    st2 = subprocess.run(ADB + ["get-state"],
                                         capture_output=True, text=True,
                                         timeout=5)
                    if "device" in (st2.stdout + st2.stderr):
                        _ADB_DEAD = False
                        print("  [input] adb REVIVED via connect")
                        return adb_shell(cmd, timeout=20) or ""
            except Exception:
                pass
        print("  [input] routing input via docker exec")
    # Channel 2: docker exec `input`.
    try:
        r = subprocess.run(["sudo", "docker", "exec", "redroid", "sh", "-c", cmd],
                           capture_output=True, text=True, timeout=20)
        out = (r.stdout or "") + (r.stderr or "")
        if r.returncode == 0 and "rror" not in out and "sage" not in out:
            return out
        print(f"  [input] docker `input` failed (rc={r.returncode}, "
              f"out={out.strip()[:120]!r}) — trying sendevent")
    except Exception as e:
        print(f"  [input] docker exec threw {e!r} — trying sendevent")
    # Channel 2b: app-level broadcast (see _broadcast_cmd).
    r = _broadcast_cmd(cmd)
    if r is not None:
        return r
    # Channel 3: sendevent.
    return _sendevent_cmd(cmd)


def _broadcast_cmd(cmd):
    """Channel 2b: `am broadcast` -> Render2Activity's 6-Z186 debug
    touch receiver -> the production onTouch path. InputManager-
    independent (ActivityManager service); the only channel that works
    when docker `input` silently no-ops AND no evdev nodes exist for
    sendevent (the exact 33110255428 situation). Returns the shell
    output when the command shape was handled, None otherwise."""
    m = re.match(r"input (?:tap|swipe) (\d+) (\d+)(?: (\d+) (\d+))?(?: \d+)?", cmd)
    if not m:
        return None
    try:
        x1, y1 = m.group(1), m.group(2)
        is_swipe = m.group(3) is not None
        if is_swipe and (int(x1) != int(m.group(3)) or int(y1) != int(m.group(4))):
            amcmd = (f"am broadcast -a io.twoyi.debug.TOUCH "
                     f"--es action swipe --ei x {x1} --ei y {y1} "
                     f"--ei x2 {m.group(3)} --ei y2 {m.group(4)} "
                     f"--ei steps 8")
        else:
            amcmd = (f"am broadcast -a io.twoyi.debug.TOUCH "
                     f"--es action tap --ei x {x1} --ei y {y1}")
        r = subprocess.run(
            ["sudo", "docker", "exec", "redroid", "sh", "-c", amcmd],
            capture_output=True, text=True, timeout=20)
        # Delivery is fire-and-forget; the probe verifies the effect
        # via its marker/frame checks after each gesture.
        return (r.stdout or "") + (r.stderr or "")
    except Exception:
        # Broadcast channel unavailable — empty string still means
        # "shape handled, nothing to report"; sendevent follows.
        return ""


def _sendevent_discover():
    """Find the touchscreen evdev node + ABS_MT ranges inside redroid."""
    global _TOUCH_EVDEV, _TOUCH_RANGE, _KEY_EVDEV
    if _TOUCH_EVDEV is not None:
        return
    _TOUCH_EVDEV = ""
    _TOUCH_RANGE = (0, 0, 0, 0)
    _KEY_EVDEV = ""
    try:
        r = subprocess.run(
            ["sudo", "docker", "exec", "redroid", "sh", "-c", "getevent -pl 2>/dev/null"],
            capture_output=True, text=True, timeout=15)
        text = r.stdout or ""
    except Exception:
        return
    cur = None
    for line in text.splitlines():
        if "add device" in line and "name:" in line:
            nm = line.split("name:")[-1].strip().strip('"')
            cur = nm
            continue
        if "ABS_MT_POSITION_X" in line and cur is not None:
            mx = re.search(r"ABS_MT_POSITION_X.*?max\s+(\d+)", line)
            cur_ev = _first_evdev_for(text, cur)
            if cur_ev:
                _TOUCH_EVDEV = cur_ev
                _TOUCH_RANGE = (0, int(mx.group(1)) if mx else 0, 0, 0)
                cur = None
                continue
        if "KEY_BACK" in line and _KEY_EVDEV == "" and cur is not None:
            ev = _first_evdev_for(text, cur)
            if ev:
                _KEY_EVDEV = ev
    if _TOUCH_EVDEV:
        # Fill Y max from the same device block.
        try:
            r = subprocess.run(
                ["sudo", "docker", "exec", "redroid", "sh", "-c",
                 f"getevent -pl 2>/dev/null | grep -A30 '{_TOUCH_EVDEV}' | "
                 f"grep ABS_MT_POSITION_Y"],
                capture_output=True, text=True, timeout=15)
            my = re.search(r"max\s+(\d+)", r.stdout or "")
            _, xm, _, _ = _TOUCH_RANGE
            _TOUCH_RANGE = (0, xm, 0, int(my.group(1)) if my else 0)
        except Exception:
            pass
        print(f"  [input] sendevent touchscreen: {_TOUCH_EVDEV} "
              f"range x<= {_TOUCH_RANGE[1]} y<= {_TOUCH_RANGE[3]}")


def _first_evdev_for(getevent_pl_text, dev_name):
    """Map a device name from `getevent -pl` back to its event node by
    re-running getevent -lp and correlating blocks (name precedes the
    handlers line)."""
    try:
        r = subprocess.run(
            ["sudo", "docker", "exec", "redroid", "sh", "-c",
             "getevent -lp 2>/dev/null"],
            capture_output=True, text=True, timeout=15)
        for block in (r.stdout or "").split("add device"):
            if dev_name in block:
                m = re.search(r"/dev/input/event\d+", block)
                if m:
                    return m.group(0)
    except Exception:
        pass
    return ""


def _sendevent_cmd(cmd):
    """Execute tap/swipe/keyevent via raw sendevent (protocol B)."""
    global _TOUCH_EVDEV, _TOUCH_RANGE, _KEY_EVDEV
    _sendevent_discover()
    m = re.match(r"input tap (\d+) (\d+)", cmd)
    if m and _TOUCH_EVDEV:
        return _sendevent_tap(int(m.group(1)), int(m.group(2)))
    m = re.match(r"input swipe (\d+) (\d+) (\d+) (\d+) (\d+)", cmd)
    if m and _TOUCH_EVDEV:
        return _sendevent_swipe(int(m.group(1)), int(m.group(2)),
                                int(m.group(3)), int(m.group(4)),
                                int(m.group(5)))
    m = re.match(r"input keyevent (\S+)", cmd)
    if m and _KEY_EVDEV:
        code = m.group(1)
        key = {"4": "158", "KEYCODE_BACK": "158"}.get(code, None)
        if key:
            for val in ("1", "0"):
                subprocess.run(["sudo", "docker", "exec", "redroid", "sendevent",
                                _KEY_EVDEV, "1", key, val],
                               capture_output=True, timeout=10)
                subprocess.run(["sudo", "docker", "exec", "redroid", "sendevent",
                                _KEY_EVDEV, "0", "0", "0"],
                               capture_output=True, timeout=10)
            return ""
    # Unknown shape or no nodes — last resort: docker `input` anyway.
    try:
        r = subprocess.run(["sudo", "docker", "exec", "redroid", "sh", "-c", cmd],
                           capture_output=True, text=True, timeout=20)
        return (r.stdout or "") + (r.stderr or "")
    except Exception:
        return ""


def _se(*args):
    subprocess.run(["sudo", "docker", "exec", "redroid", "sendevent"] +
                   list(args), capture_output=True, timeout=10)


def _sendevent_tap(x, y):
    ev, xm, ym = _TOUCH_EVDEV, _TOUCH_RANGE[1], _TOUCH_RANGE[3]
    sx = int(x * (xm / SCREEN_W)) if xm else x
    sy = int(y * (ym / SCREEN_H)) if ym else y
    _se(ev, "3", "57", "100")       # ABS_MT_TRACKING_ID
    _se(ev, "3", "53", str(sx))     # ABS_MT_POSITION_X
    _se(ev, "3", "54", str(sy))     # ABS_MT_POSITION_Y
    _se(ev, "3", "48", "5")         # ABS_MT_TOUCH_MAJOR
    _se(ev, "1", "330", "1")        # BTN_TOUCH down
    _se(ev, "0", "0", "0")          # SYN
    _se(ev, "3", "57", "-1")
    _se(ev, "1", "330", "0")
    _se(ev, "0", "0", "0")
    return ""


def _sendevent_swipe(x1, y1, x2, y2, dur_ms):
    ev, xm, ym = _TOUCH_EVDEV, _TOUCH_RANGE[1], _TOUCH_RANGE[3]
    sx1 = int(x1 * (xm / SCREEN_W)) if xm else x1
    sy1 = int(y1 * (ym / SCREEN_H)) if ym else y1
    sx2 = int(x2 * (xm / SCREEN_W)) if xm else x2
    sy2 = int(y2 * (ym / SCREEN_H)) if ym else y2
    steps = max(2, dur_ms // 40)
    _se(ev, "3", "57", "101")
    _se(ev, "3", "53", str(sx1))
    _se(ev, "3", "54", str(sy1))
    _se(ev, "1", "330", "1")
    _se(ev, "0", "0", "0")
    for i in range(1, steps + 1):
        nx = sx1 + (sx2 - sx1) * i // steps
        ny = sy1 + (sy2 - sy1) * i // steps
        _se(ev, "3", "53", str(nx))
        _se(ev, "3", "54", str(ny))
        _se(ev, "0", "0", "0")
        time.sleep(dur_ms / 1000.0 / steps)
    _se(ev, "3", "57", "-1")
    _se(ev, "1", "330", "0")
    _se(ev, "0", "0", "0")
    return ""

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

def _screencap_adb():
    """screencap via adb; returns raw PNG bytes or b''."""
    try:
        r = subprocess.run(ADB + ["exec-out", "screencap", "-p"],
                           capture_output=True, timeout=8)
        data = r.stdout or b""
        return data if data[:8] == b"\x89PNG\r\n\x1a\n" else b""
    except Exception:
        return b""

def _screencap_docker():
    """screencap via a single `docker exec` — works when adb is DEAD but
    the container + SurfaceFlinger live (run 33015609499: the guest jail
    unlinked the container's real /dev/socket/adbd; every adb channel died
    while the framework kept running). Returns raw PNG bytes or b''.

    6-Z183 FIX (the 0-byte screenshots): this used to `docker exec
    redroid screencap ...` with the BINARY name directly. docker exec
    resolves the binary through the image config's PATH — which does NOT
    include /system/bin on redroid — so every call died with "executable
    file not found" and ALL 21 navigation screenshots came out 0 bytes
    (run 33062718661), while the workflow's own `sh -c "screencap ..."`
    variant worked. The fix mirrors that proven form: run the ABSOLUTE
    path under sh -c and stream the PNG back on stdout (no docker cp,
    no second exec round-trip, PNG-magic validated)."""
    try:
        r = subprocess.run(
            ["sudo", "docker", "exec", "redroid", "sh", "-c",
             "/system/bin/screencap -p /data/local/tmp/_e2e_cap.png "
             "&& cat /data/local/tmp/_e2e_cap.png"],
            capture_output=True, timeout=20)
        data = r.stdout or b""
        # Locate the PNG signature in case the shell prefixes any noise.
        idx = data.find(b"\x89PNG\r\n\x1a\n")
        if idx < 0:
            return b""
        return data[idx:]
    except Exception:
        return b""

# 6-Z183: adbd inside redroid dies when the TWRP container starts (known
# open issue). Every screenshot then burned up to ~90 s in a timeout
# ladder: 2x adb exec-out (20 s each) + adb connect (10 s) + docker exec
# (25 s) + docker cp (15 s) — the 12 screenshots of the 60 s boot window
# stretched the run by many minutes and stole CPU from the ptrace
# emulation. After ADB_DEAD_THRESHOLD consecutive empty adb attempts the
# adb path is skipped entirely (one final reconnect, then docker-only).
_ADB_DEAD = False
_ADB_EMPTY_STREAK = 0
_ADB_DEAD_THRESHOLD = 3
# 6-Z186 iter-8: sendevent-fallback state (touchscreen node + ranges).
_TOUCH_EVDEV = None
_TOUCH_RANGE = (0, 0, 0, 0)
_KEY_EVDEV = ""
# 6-Z186 iter-10: docker `input` sometimes silently no-ops for a whole
# run (rc=0, no error text — 33108190303, 33110255428). Effect-based
# escalation: the probe flips this after an input round shows ZERO
# effect, after which input_cmd goes straight to sendevent.
_FORCE_SENDEVENT = False

def screenshot(name):
    global _ADB_DEAD, _ADB_EMPTY_STREAK
    path = os.path.join(ART, f"screenshot-{name}.png")
    data = b""
    source = "none"
    if not _ADB_DEAD:
        data = _screencap_adb()
        if data:
            source = "adb"
            # 6-Z184: reset the empty-streak on every SUCCESS — the old
            # counter only ever incremented (reset only in the reconnect
            # branch), so three NON-adjacent empty frames across steps
            # 6b/7/8 spuriously declared adb dead mid-run.
            _ADB_EMPTY_STREAK = 0
        else:
            _ADB_EMPTY_STREAK += 1
            if _ADB_EMPTY_STREAK >= _ADB_DEAD_THRESHOLD:
                # One last reconnect attempt, then declare adb dead for the
                # rest of the run (it does not come back once the TWRP
                # container starts — verified across 33014296538…33062718661).
                # adb connect expects host:port — meaningless for a local emulator serial.
                if ":" in ADB[-1]:
                    subprocess.run(["adb", "connect", ADB[-1]], capture_output=True, timeout=10)
                data = _screencap_adb()
                if data:
                    source = "adb"
                    _ADB_EMPTY_STREAK = 0
                else:
                    _ADB_DEAD = True
                    print("  [screenshot] adb declared DEAD — docker-only from here")
    if not data:
        data = _screencap_docker()
        if data:
            source = "docker"
    with open(path, "wb") as f:
        f.write(data)
    print(f"  [screenshot] {name} ({len(data)} bytes, {source})")
    return path

def dump_ui(name):
    """6-Z186 iter-5: adb FIRST (the proven channel for the early
    phases), docker-exec ONLY when adb is dead. Iter-4's docker-first
    ordering froze every dump to the first frame: a failed docker
    `uiautomator dump` left the previous /sdcard/ui-dump.xml in place
    and the `cat` served the STALE file — all 24 XMLs in run 33105112068
    were byte-identical while the real screen scrolled on. Now the
    remote file is DELETED before each dump attempt so a failure yields
    EMPTY (parse_ui -> None, callers fall back cleanly) instead of a
    frozen viewport."""
    xml_path = os.path.join(ART, f"uiautomator-{name}.xml")
    if os.path.exists(xml_path):
        os.remove(xml_path)  # never serve a previous same-named dump
    # adb branch (alive): rm old file -> dump -> pull.
    global _ADB_DEAD
    alive = False
    if not _ADB_DEAD:
        try:
            st = subprocess.run(ADB + ["get-state"], capture_output=True,
                                text=True, timeout=5)
            alive = "device" in (st.stdout + st.stderr)
        except Exception:
            alive = False
    if alive:
        adb_shell("rm -f /sdcard/ui-dump.xml; uiautomator dump /sdcard/ui-dump.xml",
                  timeout=45)
        subprocess.run(ADB + ["pull", "/sdcard/ui-dump.xml", xml_path],
                       capture_output=True, timeout=10)
        if os.path.exists(xml_path) and os.path.getsize(xml_path) > 100:
            return xml_path
        # dump failed (stale deleted + nothing new) — fall through to
        # the docker attempt below, which likewise cannot serve stale.
    try:
        r = subprocess.run(
            ["sudo", "docker", "exec", "redroid", "sh", "-c",
             "rm -f /sdcard/ui-dump.xml; "
             "uiautomator dump /sdcard/ui-dump.xml >/dev/null 2>&1; "
             "cat /sdcard/ui-dump.xml 2>/dev/null"],
            capture_output=True, timeout=45)
        if r.stdout and b"<hierarchy" in r.stdout:
            with open(xml_path, "wb") as f:
                f.write(r.stdout)
        elif os.path.exists(xml_path):
            os.remove(xml_path)  # never leave a stale local copy
    except Exception:
        pass
    return xml_path

def parse_ui(xml_path):
    try:
        tree = ET.parse(xml_path)
        return tree.getroot()
    except Exception:
        return None

def detect_screen_size(root):
    """Detect screen size from the root hierarchy node bounds.

    Cross-checked against `wm size` (the SAME source of truth the app's
    DisplayMetrics auto-detect uses — 6-Z171b native-resolution chain).
    Any mismatch between the two is loud evidence, not a silent default.
    """
    global SCREEN_W, SCREEN_H
    wm = adb_shell("wm size")
    m = re.search(r"(\d+)x(\d+)", wm or "")
    if m:
        wm_w, wm_h = int(m.group(1)), int(m.group(2))
        print(f"  [screen] wm size: {wm_w}x{wm_h}")
    else:
        wm_w = wm_h = 0
    if root is not None:
        bounds = root.get("bounds", "")
        m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
        if m:
            SCREEN_W = int(m.group(3))
            SCREEN_H = int(m.group(4))
            print(f"  [screen] uiautomator bounds: {SCREEN_W}x{SCREEN_H}")
    if wm_w and wm_h and (wm_w != SCREEN_W or wm_h != SCREEN_H):
        print(f"  [screen] NOTE: wm size ({wm_w}x{wm_h}) != uiautomator bounds "
              f"({SCREEN_W}x{SCREEN_H}) — using wm size (app-side truth)")
        SCREEN_W, SCREEN_H = wm_w, wm_h
    with open(os.path.join(ART, "screen-size.txt"), "w") as f:
        f.write(f"wm: {wm}\nuiautomator: {SCREEN_W}x{SCREEN_H}\nfinal: {SCREEN_W}x{SCREEN_H}\n")

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
    # 6-Z186 iter-4: routed through input_cmd (adb -> docker-exec
    # fallback with liveness probing) so the tap survives redroid's
    # mid-run adbd deaths.
    combined = input_cmd(f"input swipe {x} {y} {x} {y} 100").lower()
    if any(s in combined for s in ("usage", "unknown", "error", "not found", "invalid")):
        # input swipe not recognized — fall back to input tap
        input_cmd(f"input tap {x} {y}")

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
    input_cmd(f"input swipe {cx} {y1} {cx} {y2} 300")

def dismiss_blocking_popups(tag="popup", rounds=8):
    """Dismiss the popups that block Render2Activity from ever booting.

    FORENSIC GROUND TRUTH (run 32952695067 artifacts — uiautomator XML
    dumps + VLM analysis of the screenshots, NOT coordinate guesses):

      Popup 1 — SystemUI ImmersiveModeConfirmation, package="android":
        text "Viewing full screen" / "To exit, swipe down from the top."
        button node: text="GOT IT", resource-id="android:id/ok",
        class="android.widget.Button", bounds=[464,353][640,449] on the
        720x1280 redroid display (i.e. NOT at any fixed coordinate — we
        parse the bounds from the live XML dump and tap its center).
        Triggered because Render2Activity sets IMMERSIVE_STICKY fullscreen
        flags; it covers the whole screen and hides the app dialog from
        uiautomator's top-window dump (the app dialog is still visible in
        the composite screenshot, poking out beneath it).

      Popup 2 — the app's own Android-12 AlertDialog chain (legacy APKs
        only; REMOVED at the source in v5.3 — Render2Activity now calls
        bootSystem() directly):
        "Attention — You are running on Android 12, please follow the
        guide to use twoyi." with buttons "Read it" / "Don't show again",
        then a second dialog "I confirm it". We tap "Don't show again"
        (NEVER "Read it" — that opens a browser and finishes the
        activity), then "I confirm it". Button labels may render ALL-CAPS
        in the dump, so matching is case-insensitive substring.

    Strategy per round (up to `rounds`, ~2s apart):
      0. One-time belt: `settings put global immersive_mode_confirmations
         confirmed` — kills popup 1 system-wide for every future launch.
      1. Dump UI → if a "GOT IT"-style confirm button exists → tap its
         parsed center. Repeat (SystemUI can re-show it once).
      2. Else if "don't show again" exists → tap it, then on the next
         round tap "confirm".
      3. Stop when a round finds neither (screen is clear → bootSystem
         is free to run).

    Returns the number of taps performed. Every round's XML dump and a
    before/after screenshot land in the artifacts for evidence.
    """
    # Belt: make sure SystemUI never shows the immersive confirmation
    # again on this device (idempotent, harmless on emulators that ship
    # with it pre-confirmed).
    adb_shell("settings put global immersive_mode_confirmations confirmed")

    taps_done = 0
    for r in range(rounds):
        xml = dump_ui(f"{tag}_round{r}")
        root = parse_ui(xml)
        if root is None:
            wait(1)
            continue

        # 1) SystemUI fullscreen confirmation (any casing of "got it").
        got_it = None
        for node in root.iter("node"):
            txt = (node.get("text", "") or "").strip().lower()
            rid = node.get("resource-id", "") or ""
            if txt in ("got it", "ok", "okay") or (
                    rid == "android:id/ok" and txt):
                got_it = node
                break
        if got_it is not None:
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]",
                         got_it.get("bounds", ""))
            if m:
                cx = (int(m.group(1)) + int(m.group(3))) // 2
                cy = (int(m.group(2)) + int(m.group(4))) // 2
                print(f"  popup-dismiss[{r}]: tapping 'GOT IT' at "
                      f"({cx},{cy}) from XML bounds {got_it.get('bounds')}")
                tap(cx, cy)
                taps_done += 1
                wait(2)
                continue

        # 2) Legacy app Android-12 dialog chain (case-insensitive).
        dont_show = None
        for node in root.iter("node"):
            txt = (node.get("text", "") or "").strip().lower()
            if "don't show again" in txt or "dont show again" in txt \
                    or "don’t show again" in txt or "不再提示" in txt:
                dont_show = node
                break
        if dont_show is not None:
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]",
                         dont_show.get("bounds", ""))
            if m:
                cx = (int(m.group(1)) + int(m.group(3))) // 2
                cy = (int(m.group(2)) + int(m.group(4))) // 2
                print(f"  popup-dismiss[{r}]: tapping 'Don't show again' at "
                      f"({cx},{cy}) from XML bounds")
                tap(cx, cy)
                taps_done += 1
                wait(2)
                continue

        confirm = None
        for node in root.iter("node"):
            txt = (node.get("text", "") or "").strip().lower()
            if "i confirm" in txt or "我已确认" in txt:
                confirm = node
                break
        if confirm is not None:
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]",
                         confirm.get("bounds", ""))
            if m:
                cx = (int(m.group(1)) + int(m.group(3))) // 2
                cy = (int(m.group(2)) + int(m.group(4))) // 2
                print(f"  popup-dismiss[{r}]: tapping 'I confirm it' at "
                      f"({cx},{cy}) from XML bounds")
                tap(cx, cy)
                taps_done += 1
                wait(2)
                continue

        # Nothing blockable found — screen is clear.
        print(f"  popup-dismiss[{r}]: no blocking popup visible — clear")
        break
    return taps_done

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
      - If we can't find the 'Select ROM' preference, SCROLL DOWN through
        the preference list (up to 6 swipes — on high-res screens like
        redroid's 720x1280, 'Select ROM' sits several viewport-heights
        down inside the Advanced category) before returning False.
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

    def check_dump(r):
        """Return (found, imported) for a dumped hierarchy.
        found=False  -> 'Select ROM' not in this viewport.
        found=True   -> title found; imported reflects the summary check."""
        for node in r.iter("node"):
            txt = (node.get("text", "") or "").lower()
            desc = (node.get("content-desc", "") or "").lower()
            for marker in ["no rom installed", "no rom", "no roms"]:
                if marker in txt or marker in desc:
                    print(f"  ✗ ROM import verification failed: found '{marker}' (text={txt!r}, desc={desc!r})")
                    return (True, False)
        parent_map = {c: p for p in r.iter() for c in p}
        for node in r.iter("node"):
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
                        return (True, False)
                    print("  ✓ Select ROM summary changed from default import prompt — ROM appears imported")
                    return (True, True)
        return (False, None)

    # Pass 1: check the caller-provided dump (current viewport).
    found, imported = check_dump(root)
    if found:
        return imported

    # Pass 2: scroll down through the preference list, re-dumping each
    # viewport, until 'Select ROM' is visible or we run out of list.
    for i in range(6):
        print(f"  'Select ROM' not in viewport — scrolling down to find it (attempt {i+1}/6)")
        swipe_up()
        wait(1)
        xml = dump_ui(f"05_verify_scroll_{i}")
        r = parse_ui(xml)
        if r is None:
            continue
        found, imported = check_dump(r)
        if found:
            return imported

    # Could not find the Select ROM preference even after scrolling.
    # Previously this returned True ("assume imported") which masked
    # real failures. Return False so the test aborts early instead of
    # producing misleading "No ROM Installed" screenshots later.
    print("  ✗ Could not find 'Select ROM' preference in UI after scrolling — returning False (was 'assume imported')")
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
    # v5.3: 30s default (was 600s). The old 600s wait burned 10 minutes
    # staring at two undismissed popups (run 32952695067) — the popups are
    # now removed at the source AND actively dismissed (step 6b), so the
    # wait is just for the TWRP render itself. 30s is generous: redroid
    # arm64 is native and TWRP is a ramdisk-only guest.
    boot_wait = int(os.environ.get("BOOT_WAIT_SECONDS", "30"))

    # ─────────────────────────────────────────────
    # Step 1: Launch app
    # ─────────────────────────────────────────────
    print("=" * 60)
    print("  Step 1: Launch app via launcher (monkey -p)")
    print("=" * 60)
    adb_shell(f"monkey -p {PACKAGE} -c android.intent.category.LAUNCHER 1")
    wait(5)
    # Launch robustness (redroid arm64): on fast cold starts the activity
    # may not have resumed yet when we check — and on some launches monkey
    # silently fails to foreground the app (v5 run 32951494158: step 2
    # scrolled the LAUNCHER for 8 attempts because the app never came up).
    # Retry monkey once, then fall back to an explicit component start
    # (SettingsActivity IS the launcher activity — singleTask, so this is
    # exactly the same activity a launcher tap would bring up).
    activity = get_current_activity() or ""
    if PACKAGE not in activity.lower():
        print(f"  App not foregrounded after monkey (activity={activity!r}) — retrying monkey")
        adb_shell(f"monkey -p {PACKAGE} -c android.intent.category.LAUNCHER 1")
        wait(4)
        activity = get_current_activity() or ""
        if PACKAGE not in activity.lower():
            print(f"  Still not foregrounded (activity={activity!r}) — am start fallback")
            adb_shell(f"am start -n {PACKAGE}/io.twoyi.ui.SettingsActivity")
            wait(4)
            activity = get_current_activity() or ""
            if PACKAGE not in activity.lower():
                print(f"  ⚠ App STILL not foregrounded (activity={activity!r}) — continuing; Step 3 am start will retry")
            else:
                print(f"  ✓ App foregrounded via am start fallback (activity={activity!r})")
        else:
            print(f"  ✓ App foregrounded on monkey retry (activity={activity!r})")
    else:
        print(f"  ✓ App foregrounded (activity={activity!r})")
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
    # Step 5: VERIFY "Boot to Recovery" checkbox state (do NOT force)
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print('  Step 5: Verify "Boot to Recovery" checkbox state (auto-set by app)')
    print("=" * 60)
    # 6-Z209b: the Twoyi app auto-sets boot_recovery based on the
    # imported recovery's layout (RomManager.autoSetBootRecovery
    # called from SettingsActivity.importRomForActiveProfile right
    # after the staging→live rename). TWRP-style (/init regular file
    # OR /sbin/recovery present) → boot_recovery=true; AOSP-style
    # (/init symlink + /system/bin/recovery) → boot_recovery=false.
    #
    # Previously this step FORCE-ENABLED the checkbox for every
    # recovery, which broke AOSP-style recoveries (OrangeFox R12
    # lavender run 33206081307): kr64 with --boot-recovery tried to
    # stage /sbin/recovery → ENOENT (OrangeFox ships /system/bin/
    # recovery instead) → child exited 127 → recovery UI never
    # rendered.
    #
    # Now we only READ the checkbox state to confirm the app's
    # auto-detection worked. If the user (or the app) had it set
    # the other way, we DO NOT override — the app's auto-detection
    # is authoritative for the imported recovery's layout.
    result = scroll_to_find("Boot to Recovery", max_scrolls=5, exact=False)
    if result:
        cx, cy, node = result
        checked = node.get("checked", "false")
        print(f"  Found 'Boot to Recovery' at ({cx}, {cy}), checked={checked} (auto-set by app — not overriding)")
    else:
        print("  ⚠ Could not find 'Boot to Recovery' (may have scrolled off — the app's auto-set still applies)")

    # ─────────────────────────────────────────────
    # Step 6: Scroll to top and tap "Launch Container"
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print('  Step 6: Tap "Launch Container"')
    print("=" * 60)

    # 6-Z185: the 6-Z180 no-input A/B probe (TWOYI_NO_INPUT) was
    # REMOVED — the hook no longer reads gate files (touch is always
    # on; the user vetoed input gating). The workflow input was
    # removed with it.

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
    activity_after_launch = get_current_activity()
    print(f"  Current activity: {activity_after_launch}")

    # ─────────────────────────────────────────────
    # Step 6b: Dismiss the popups that block bootSystem()
    # ─────────────────────────────────────────────
    # Run 32952695067: BOTH the SystemUI "Viewing full screen / GOT IT"
    # confirmation AND the app's Android-12 "Attention" dialog sat on
    # screen for the ENTIRE 600s wait — bootSystem() never ran, TWRP never
    # rendered, and the workflow still passed (false green). The app-side
    # dialog is removed in v5.3 (Render2Activity boots unconditionally);
    # this step kills the SystemUI confirmation via settings and taps any
    # residual popup button by its PARSED uiautomator bounds — never a
    # guessed coordinate. Screenshot evidence before/after.
    print()
    print("=" * 60)
    print("  Step 6b: Dismiss blocking popups (GOT IT / Android-12 gate)")
    print("=" * 60)
    screenshot("06b_popups_before")
    n_popups = dismiss_blocking_popups(tag="06b_popup")
    screenshot("06b_popups_after")
    print(f"  Step 6b done: {n_popups} popup button(s) tapped")

    # ── ARM64-T1: catch the silent "app crashed back to launcher" case ──
    # Run 32886902337 (arm64 via native bridge) produced a false-green
    # workflow because the script took screenshots for 180s after the
    # Launch Container tap and exited 0 — but the app had actually
    # CRASHED back to the launcher within 5s of the tap (the arm64
    # ptrace path couldn't decode x86_64 register layout returned by
    # the host kernel). The 180s of "launcher" screenshots were then
    # misread as "TWRP booted".
    #
    # Hard-abort here if the resumed activity is the system launcher —
    # the twoyi app process is gone, and no amount of waiting will
    # bring TWRP up. The error includes the diagnostic captures
    # (logcat, app-logs, run-as cat of FileLogger) so the next
    # debugging session has the crash trace.
    if activity_after_launch and "launcher" in activity_after_launch.lower():
        print()
        print("=" * 60)
        print("  ✗✗✗ ABORTING: twoyi app crashed back to launcher after Launch Container tap")
        print("=" * 60)
        print(f"  Current activity: {activity_after_launch}")
        print("  The app process died immediately after the ptrace launch.")
        print("  This is the run-32886902337 failure pattern (arm64 native bridge")
        print("  couldn't decode x86_64 register layout from host kernel).")
        print()
        print("  Capturing diagnostic artifacts before exiting...")
        screenshot("08_abort_launcher_crash")
        logcat = adb("logcat", "-d", timeout=15)
        with open(os.path.join(ART, "logcat.txt"), "w") as f:
            f.write(logcat)
        try:
            os.makedirs(os.path.join(ART, "app-logs"), exist_ok=True)
            subprocess.run(ADB + ["pull", f"/sdcard/Android/data/{PACKAGE}/files/log/",
                                 os.path.join(ART, "app-logs/")],
                          capture_output=True, timeout=30)
        except Exception:
            pass
        # Try run-as cat of the FileLogger crash log too.
        for remote, local in [
            ("cache/log/app.log", "app.log"),
            ("cache/log/boot.log", "boot.log"),
            ("cache/log/crash.log", "crash.log"),
            ("kr64-app-stderr.log", "kr64-app-stderr.log"),
        ]:
            out = adb_shell(f"run-as {PACKAGE} cat {remote}", timeout=60)
            if out:
                with open(os.path.join(ART, local), "w", errors="replace") as f:
                    f.write(out)
                print(f"  pulled via run-as: {remote} -> {local}")
        print("  Diagnostic artifacts captured. Exiting with code 1.")
        sys.exit(1)

    # ─────────────────────────────────────────────
    # Step 7: Wait for boot — screenshots every 5s
    # ─────────────────────────────────────────────
    print()
    print("=" * 60)
    print(f"  Step 7: Wait for boot ({boot_wait}s) — screenshots every 5s")
    print("=" * 60)
    # ── History: TWRP renders its welcome gate LIVE (run 32640227105):
    # "Unmounted System Partition — Keep Read Only?" with buttons
    # Keep Read Only / Select Language / Swipe to Allow Modifications.
    # A plain center tap does NOT dismiss this gate — it needs a BUTTON
    # TAP or a SLIDER SWIPE, hence the rotation below.
    #
    # ── v5.3: gate dismissal, screen-relative, 10s cadence ──
    # The TWRP welcome gate ("Unmounted System Partition — Keep Read Only?"
    # with Keep Read Only / Select Language / Swipe to Allow Modifications)
    # still needs a BUTTON TAP or SLIDER SWIPE. The OLD rotation used
    # hardcoded 320x640 coordinates ("input tap 160 320" etc.) — on the
    # 720x1280 redroid display those landed in empty space, which is why
    # run 32952695067 sat frozen: neither popup nor gate was ever touched.
    # All gestures are now computed from SCREEN_W/SCREEN_H (detected in
    # step 1 from the hierarchy root bounds):
    #   0: center tap — generic, harmless everywhere
    #   1: bottom-left button — 'Keep Read Only' (0.25W, 0.89H)
    #   2: bottom slider swipe — 'Swipe to Allow Modifications'
    #      (0.19W → 0.88W at 0.89H)
    #   3: bottom-center tap — the slider area
    # A gesture fires every 10s (so a 30s wait = up to 3 gestures, enough
    # for the gate + one retry). BACK is deliberately NOT in the first
    # rotation: on the gate it does nothing; it stays available via the
    # safe rotation below.
    #
    # After 120s (long waits re-enabled via BOOT_WAIT_SECONDS for local
    # debugging) the rotation switches to SAFE-ONLY (center taps + BACK)
    # so a stray gesture can never cross a TWRP menu button (the old
    # hardcoded swipe crossed "OTG" and dead-ended the run for ~440s).
    # 6-Z186 iter-4: the ImmersiveModeConfirmation ("Viewing full
    # screen" / GOT IT, android:id/ok) MUST be dismissed before any
    # TWRP gate gesture can land — it intercepts every touch. VLM on
    # run 33102864178's frame + the run-32952695067 XML forensics both
    # pin the button at ~(552,400) FIXED PIXELS (anchored near the top
    # on both 720x1280 and 720x1600 displays). Rotation: GOT IT first,
    # then gate slider, then Keep Read Only. All through input_cmd
    # (docker-exec fallback) so a mid-run adbd death can't starve the
    # guest of input (run 33104264666 lost every gesture that way).
    gestures = [
        f"input tap {int(SCREEN_W * 0.77)} {int(SCREEN_H * 0.25)}",   # 0: GOT IT
        f"input tap {int(SCREEN_W * 0.72)} {int(SCREEN_H * 0.22)}",   # 1: GOT IT variant
        f"input swipe {int(SCREEN_W * 0.19)} {int(SCREEN_H * 0.89)} "
        f"{int(SCREEN_W * 0.88)} {int(SCREEN_H * 0.89)} 400",         # 2: gate slider
        f"input tap {int(SCREEN_W * 0.25)} {int(SCREEN_H * 0.89)}",   # 3: Keep Read Only
    ]
    gestures_safe = [
        f"input tap {SCREEN_W // 2} {SCREEN_H // 2}",               # 0: center
        "input keyevent 4",                                         # 1: BACK
        f"input tap {SCREEN_W // 2} {SCREEN_H // 2}",               # 2: center
        f"input tap {SCREEN_W // 2} {SCREEN_H // 2}",               # 3: center
    ]
    gesture_index = 0
    for i in range(max(1, boot_wait // 5)):
        wait(5)
        elapsed = (i + 1) * 5
        if elapsed % 10 == 0:
            g = gesture_index % 4
            seq = gestures if elapsed <= 120 else gestures_safe
            input_cmd(seq[g])
            mode = "gate" if elapsed <= 120 else "safe"
            print(f"  feeding gesture {g}: {seq[g]} (at {elapsed}s, {mode})")
            gesture_index += 1
            # One-off mid-cycle capture 1 s after the gesture so a
            # transition (dialog → main menu) is caught even if the
            # 5 s cadence would have missed a 0.5 s frame.
            wait(1)
            screenshot(f"07_boot_{elapsed}s_postg")
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

    # ─────────────────────────────────────────────
    # Step 8b (6-Z185): sandbox escape repro probe.
    #
    # Drives TWRP's File Manager to the /system listing and screenshots
    # every step. The security bar: the /system listing must show the
    # GUEST rootfs's own contents (the TWRP ramdisk ships only
    # system/usr/share/zoneinfo/tzdata — so a listing of just "usr"),
    # NOT the host device's real /system (dozens of app/lib/etc dirs).
    # Before the 6-Z185 fix this exact path listed the HOST's real
    # /system/app (observed live on a physical Honor Magic UI device).
    #
    # minui is not uiautomator-visible, so taps are fractional-screen
    # candidates with a screenshot after every step; the verdict comes
    # from analyzing the captures + the pulled rootfs/system listing
    # (deterministic) + the tracer log's SANDBOX BACKSTOP lines.
    # The CI rootfs is disposable per-run; every tapped target here is
    # a navigation action, never a destructive confirm slider.
    # ─────────────────────────────────────────────
    if os.environ.get("TWOYI_PROBE_SYSTEM_APP", "0") == "1":
        print()
        print("=" * 60)
        print("  Step 8c: sandbox-escape repro — EXACT physical-device chain")
        print("  (GOT IT overlay -> gate swipe -> Advanced -> File Manager")
        print("   -> bottom-right File -> Open Terminal -> BACK -> reopen FM)")
        print("=" * 60)

        def _read_recovery_log():
            # 6-Z186 iter-3: on arm64 redroid, `run-as io.twoyi.debug cat`
            # silently produces NOTHING (works on the x86 emulator, dead
            # on redroid — every run-as pull in run 33102864178 was empty,
            # so the gate/pages were invisible to the probe and the whole
            # chain ran blind). docker exec reads the same file from the
            # container FS as root — the SAME channel the workflow's own
            # post-run pull uses (twrp-recovery-log-dockerexec.log), which
            # demonstrably works. Try run-as first (no sudo needed on
            # x86), then docker exec for each candidate path.
            _diag_shown = getattr(_read_recovery_log, "_diag", False)
            for p in (
                f"profiles/default/rootfs/tmp/recovery.log",
                "rootfs/tmp/recovery.log",
            ):
                out = adb_shell(f"run-as {PACKAGE} cat {p}", timeout=10)
                if out and len(out) > 200:
                    return out
                if out and not _diag_shown:
                    _read_recovery_log._diag = True
                    print(f"  [recovery.log] run-as returned unusable output "
                          f"({len(out)} bytes, first 120: {out[:120]!r}) — "
                          f"falling back to docker exec")
            for p in (
                f"/data/user/0/{PACKAGE}/profiles/default/rootfs/tmp/recovery.log",
                f"/data/user/0/{PACKAGE}/rootfs/tmp/recovery.log",
            ):
                try:
                    r = subprocess.run(
                        ["sudo", "docker", "exec", "redroid", "sh", "-c",
                         f"cat '{p}' 2>/dev/null"],
                        capture_output=True, timeout=15)
                    out = r.stdout.decode(errors="replace")
                except Exception:
                    out = ""
                if out and len(out) > 200:
                    return out
            return ""

        def _pages():
            """(char_offset, page) for every 'Set page:' marker in the
            guest recovery.log. Offsets (not line numbers) make
            'marker X must appear AFTER marker Y' checks trivial."""
            txt = _read_recovery_log() or ""
            return [(m.start(), m.group(1))
                    for m in re.finditer(r"Set page: '([A-Za-z0-9_]+)'", txt)]

        def dismiss_fullscreen_overlay(tag="ovl", blind=False):
            """6-Z186: the AOSP 'Viewing full screen' immersive-training
            overlay is a FRAMEWORK window (not app, not OEM-only) that
            sits above the app and eats EVERY tap until its GOT IT
            button is pressed. VLM analysis of run 33102864178's gate
            frame pinned it: title ~(0.5, 0.13), GOT IT ~(0.77, 0.25).

            uiautomator dump is the precise channel (system window) but
            it FAILS while TWRP renders (45 fps SurfaceView = never
            idle). So: dump+bounds first; when the dump is unavailable,
            blind=True taps the VLM-derived candidate positions (all in
            the TOP quarter — far above every TWRP gate/menu control at
            y >= 0.6, so they are harmless when the overlay is absent).
            blind is only used BEFORE the main menu is confirmed."""
            dismissed_any = False
            for attempt in range(4):
                path = dump_ui(f"{tag}-{attempt}")
                root = parse_ui(path)
                hit = None
                for needle in ("got it", "got_it", "understood"):
                    hit = find_by_text(root, needle) if root is not None else None
                    if hit:
                        break
                if hit is None and root is not None:
                    marker = find_by_text(root, "full screen") or \
                        find_by_text(root, "viewing")
                    if marker is not None and attempt == 0:
                        print("  [overlay] 'Viewing full screen' overlay text "
                              "present (no GOT IT match?) — dumping for review")
                if hit is not None:
                    x, y, _ = hit
                    print(f"  [overlay] GOT IT button at ({x},{y}) — tapping")
                    tap(x, y)
                    dismissed_any = True
                    wait(1.2)
                elif attempt == 0 and (root is None or hit is None):
                    # No dump (parse failed / empty) — blind candidates.
                    if blind:
                        for bfx, bfy in ((0.77, 0.25), (0.72, 0.22),
                                         (0.82, 0.28), (0.5, 0.20)):
                            bx, by = int(SCREEN_W * bfx), int(SCREEN_H * bfy)
                            print(f"  [overlay] blind GOT IT candidate "
                                  f"({bx},{by}) — tapping")
                            tap(bx, by)
                            wait(0.8)
                        dismissed_any = True
                    break
                else:
                    break
            return dismissed_any

        # ── 0) Kill the full-screen overlay FIRST (before ANY guest tap) ──
        dismiss_fullscreen_overlay("pre-gate", blind=True)

        # ── 1) Wait for the read-only gate page (system_readonly). ──
        # NOTE (physical-device lesson): TWRP's main menu only FLASHES
        # (<1 ms) at boot before the read-only gate takes over, because
        # the menu checks the read-only system state. Seeing 'main2' in
        # recovery.log therefore does NOT mean the main menu is up. The
        # gate is what's actually on screen; the swipe bar on it is how
        # you reach the REAL main menu.
        gate_pos = None
        deadline = time.time() + 150
        while time.time() < deadline:
            pages = _pages()
            gate_positions = [pos for pos, p in pages if p == "system_readonly"]
            if gate_positions:
                gate_pos = max(gate_positions)
                print("  gate page 'system_readonly' is UP")
                break
            dismiss_fullscreen_overlay("gate-wait", blind=True)
            wait(5)
        if gate_pos is None:
            print("  WARNING: gate never appeared in recovery.log — "
                  "proceeding anyway (gate may pre-date the log pull)")
            gate_pos = 0
        screenshot("term-00-gate")

        # ── 2) Swipe the read-only slider -> REAL main menu (stays up) ──
        # VLM on the run-33102864178 gate frame: slider center y=0.88,
        # horizontal span x 0.06..0.94. 6-Z186 iter-7: the gate->menu
        # transition has MULTI-SECOND latency — a single marker check
        # per swipe raced it in run 33107230132 (the "Advanced" tap hit
        # the still-present gate; the next candidate then opened WIPE
        # from the main menu). Now: keep swiping (up to 6) and then
        # SETTLE-WAIT (up to 40 s) until main2-after-gate has appeared
        # AND two screenshots 3 s apart are identical AND differ from
        # the gate frame — the menu is then rendered and stable.
        DANGER_PAGES = ("wipe", "formatdata", "install", "restore",
                        "restoreconfirm", "reboot",
                        # 6-Z188: the FM Confirm-Action page (move/copy
                        # sliders) — reached accidentally when a file is
                        # selected (run 33123690562: the probe's entry taps
                        # selected service_contexts, then every File-button
                        # tap landed here and the terminal chain died).
                        "filemanagerconfirm")

        def _shot_bytes(name):
            p = screenshot(name)
            try:
                with open(p, "rb") as f:
                    return f.read()
            except OSError:
                return b""

        gate_bytes = _shot_bytes("term-00-gate")
        menu_confirmed = False
        global _FORCE_SENDEVENT
        for i in range(6):
            y = int(SCREEN_H * 0.88)
            input_cmd(f"input swipe {int(SCREEN_W * 0.10)} {y} "
                      f"{int(SCREEN_W * 0.90)} {y} 350")
            wait(3.0)
            dismiss_fullscreen_overlay("post-swipe")
            pages = _pages()
            if any(p == "main2" and pos > gate_pos for pos, p in pages):
                menu_confirmed = True
                break
            if i == 1:
                # iter-10: TWO swipe rounds produced no new page marker.
                # Either the gate needs a different gesture or docker
                # `input` is silently dead (rc=0 no-op — runs 33108190303,
                # 33110255428). Escalate to kernel-level sendevent NOW.
                cur = _shot_bytes("term-00b-swipe-effect")
                if cur == gate_bytes:
                    print("  [input] swipe rounds show ZERO effect — "
                          "escalating to sendevent-only input")
                    _FORCE_SENDEVENT = True
        # Settle-wait: markers may lag; require visual stability.
        prev = b""
        for i in range(13):
            cur = _shot_bytes(f"term-01-settle-{i}") if i < 3 else b"x"
            if i < 3:
                if prev and cur == prev and cur != gate_bytes:
                    menu_confirmed = True
                    print("  main menu visually STABLE (2 identical non-gate "
                          "frames)")
                    break
                prev = cur
            pages = _pages()
            if any(p == "main2" and pos > gate_pos for pos, p in pages):
                menu_confirmed = True
                if i >= 3:
                    print("  main2 marker present; proceeding")
                break
            wait(3)
        if not menu_confirmed:
            print("  WARNING: main menu not confirmed — aborting the chain "
                  "BEFORE any menu taps (safety; evidence captured)")
            screenshot("term-01-abort-no-menu")
        screenshot("term-01-main-menu")

        def _last_page_after(pos):
            tail = [p for p_pos, p in _pages() if p_pos > pos]
            return tail[-1] if tail else None

        def _back_off_danger(pos):
            last = _last_page_after(pos)
            if last in DANGER_PAGES:
                print(f"  [safety] danger page '{last}' open — pressing BACK")
                input_cmd("input keyevent 4")
                wait(1.5)
                return True
            return False

        def guest_back():
            """BACK that works in escalated input mode too: keyevent
            first, then taps on the guest TWRP navbar's back button
            (bottom-left, y~1560; the exact pixel from VLM on the
            console frame, corrected for its x-scale error)."""
            marker_before = _last_page_after(0)
            input_cmd("input keyevent 4")
            wait(1.2)
            if _last_page_after(0) != marker_before:
                return True
            for bx, by in ((166, 1560), (120, 1560), (220, 1560), (166, 1500)):
                tap(bx, by)
                wait(1.2)
                if _last_page_after(0) != marker_before:
                    print(f"  [back] navbar tap ({bx},{by}) worked")
                    return True
            return False


        # ── 3) Advanced — DETERMINISTIC geometry from the layout
        # analyzer on run 33106119333's main-menu frame: 2x4 grid,
        # columns x=192/528, row centers y=414/708/1002/1294.
        # Advanced = row 4 LEFT. (The old fractional guesses opened
        # WIPE — (360,480) lands past Install's rect edge.)
        # iter-7: only tap when the menu was confirmed; verify by the
        # 'advanced' page marker specifically; back off danger pages.
        pages = _pages()
        main_pos = max([pos for pos, p in pages if p == "main2"], default=0)
        adv_open = False
        if menu_confirmed:
            for i, (x, y) in enumerate([(192, 1294), (192, 1294)]):
                print(f"  tap Advanced candidate {i}: ({x},{y})")
                dismiss_fullscreen_overlay("adv")
                tap(x, y)
                wait(2.5)
                screenshot(f"term-02-advanced-{i}")
                _back_off_danger(main_pos)
                tail = [p for p_pos, p in _pages() if p_pos > main_pos]
                if any(p == "advanced" for p in tail):
                    print("  'advanced' page marker seen — Advanced IS open")
                    adv_open = True
                    break
                if tail:
                    print(f"  page markers after tap: {tail[-4:]} (no 'advanced' yet)")
        pages = _pages()
        adv_pos = max([pos for pos, _ in pages], default=0)

        # ── 4) File Manager — DETERMINISTIC (iter-9): the Advanced page
        # is ALSO a 4-button grid (analyzer + VLM on run 33109264796's
        # term-02-advanced-1 frame): row1 y=414 = Copy Log | ADB
        # Sideload, row2 y=708 = Terminal | File Manager. Columns
        # x=192/528. FM = (528,708); Terminal = (192,708) is the
        # iter-9 FALLBACK entry for the terminal episode.
        fm_open = False
        adv_frame = _shot_bytes("term-02-advanced-frame")
        for i, (x, y) in enumerate([(528, 708), (528, 708), (360, 708)]):
            if _back_off_danger(adv_pos):
                continue
            print(f"  tap File Manager candidate {i}: ({x},{y})")
            dismiss_fullscreen_overlay("fm")
            tap(x, y)
            wait(2.0)
            cur = _shot_bytes(f"term-03-fm-{i}")
            if _back_off_danger(adv_pos):
                continue
            tail = [p for p_pos, p in _pages() if p_pos > adv_pos]
            if any(p in ("filemanager", "filemanagerlist", "filelist")
                   for p in tail):
                print(f"  file-manager page marker seen: {tail[-4:]}")
                fm_open = True
                break
            if cur != adv_frame and cur:
                print("  frame CHANGED after the FM tap (no marker — the FM "
                      "page does not log a Set page) — treating as FM open")
                fm_open = True
                break
        pages = _pages()
        fm_pos = max([pos for pos, _ in pages], default=0)
        wait(1.0)
        # BEFORE the terminal: the FM must show the sandboxed root.
        # 6-Z187 "ALWAYS SHOW GUEST'S ROOTFS": the first-entry listing must
        # be NON-EMPTY (was: blank until the terminal episode opened it).
        # Capture EARLY (right after page-entry), SETTLED (+2.5s) and
        # SCROLLED so the analysis can prove first-open correctness.
        screenshot("term-04-fm-root-BEFORE-terminal")
        wait(2.5)
        screenshot("term-04c-fm-root-BEFORE-settled")
        swipe_up()
        wait(1.0)
        screenshot("term-04d-fm-root-BEFORE-scrolled")
        # 6-Z187: press the `system` folder BEFORE the terminal as well —
        # its listing must be GUEST content in BOTH episodes (the user's
        # physical-device repro saw the HOST /system here).
        for i, sy in enumerate([1170, 1050, 930, 810]):
            print(f"  tap system-folder candidate BEFORE-{i}: (360,{sy})")
            tap(360, sy)
            wait(1.5)
            if _back_off_danger(fm_pos):
                continue
            screenshot(f"term-04e-system-BEFORE-{i}")
            input_cmd("input keyevent 4")
            wait(1.0)
        # 6-Z188b: return to the FM list MARKER-BASED (not frame bytes —
        # the status-bar CLOCK makes every screenshot byte-different, and
        # run 33125036396's byte-compare over-BACKed past the FM to the
        # main menu, where the remaining taps hit Install/Restore).
        # 'filemanagerlist' = folder navigated; 'filemanageroptions' = an
        # action popup opened then dismissed (selection cleared) — both
        # mean we are visually on the FM list.
        for bi in range(3):
            if _back_off_danger(fm_pos):
                continue
            last = _last_page_after(fm_pos)
            if last in ("filemanagerlist", "filemanageroptions"):
                print(f"  [clean-fm] on the FM list (marker '{last}', try {bi})")
                break
            guest_back()
            wait(1.5)
        else:
            print("  [clean-fm] marker not confirmed — danger watchdog armed")
        screenshot("term-04f-fm-clean")

        # ── 5) THE TERMINAL EPISODE (the escape trigger) ──
        # User's exact chain: FM bottom-right File button -> "Choose
        # Action in current folder" -> Open Terminal. The FM bottom
        # button row sits in the y≈1176..1300 band (analyzer on other
        # pages' bottom bars).
        # FALLBACK (iter-9): the Advanced page has a direct Terminal
        # button at (192,708) — same terminal page, same fork/exit
        # storm. Danger watchdog guards every tap.
        terminal_open = False
        if fm_open:
            fm_frame = _shot_bytes("term-04b-fm-frame")
            for i, (bx, by) in enumerate([
                (624, 1372), (624, 1390), (672, 1372), (592, 1372),
                (620, 1228), (560, 1300),
            ]):
                print(f"  tap bottom-right File button candidate {i}: ({bx},{by})")
                tap(bx, by)
                wait(1.5)
                cur = _shot_bytes(f"term-05-filebtn-{i}")
                if _back_off_danger(fm_pos):
                    continue
                if not cur or cur == fm_frame:
                    continue  # no popup appeared — next candidate
                # A popup appeared (frame changed) — tap its rows.
                for ty in (640, 520, 760, 880):
                    print(f"    tap Open Terminal row candidate: (360,{ty})")
                    tap(360, ty)
                    wait(3.0)
                    screenshot(f"term-06-terminal-try-{i}-{ty}")
                    if _back_off_danger(fm_pos):
                        break
                    new = [p for pos, p in _pages() if pos > fm_pos]
                    if any("terminal" in p.lower() for p in new):
                        print("    terminal page marker seen")
                        terminal_open = True
                        break
                if terminal_open:
                    break
                # popup row missed — dismiss popup, next File candidate
                input_cmd("input keyevent 4")
                wait(1.0)
        else:
            print("  [chain] FM page never confirmed — will use the "
                  "Advanced-page Terminal fallback")
        if not terminal_open:
            # FALLBACK: Advanced -> Terminal button (192,708).
            # 6-Z188b: MARKER-GATED (was frame-compare, which the
            # status-bar clock defeated — run 33125036396 tapped the
            # main menu's Backup/Restore instead).
            print("  [terminal fallback] reaching Advanced, then Terminal "
                  "button (192,708)")
            guest_back()
            wait(1.0)
            on_advanced = False
            for _ in range(3):
                if _back_off_danger(adv_pos):
                    continue
                last = _last_page_after(adv_pos)
                if last == "advanced":
                    on_advanced = True
                    break
                if last in ("main2", "main", "clear_vars"):
                    tap(192, 1294)
                    wait(2.5)
                    continue
                guest_back()
                wait(1.0)
            if on_advanced or not fm_open:
                if _last_page_after(adv_pos) != "advanced" and fm_open:
                    print("  [terminal fallback] not on Advanced — skipping "
                          "the tap (safety)")
                else:
                    print("  [terminal fallback] on Advanced — tapping Terminal")
                    tap(192, 708)
                    wait(3.0)
                    screenshot("term-06f-terminal-fallback")
                    new = [p for p_pos, p in _pages() if p_pos > fm_pos]
                    print(f"  [terminal fallback] pages after tap: {new[-5:]}")
                    if any("terminal" in p.lower() for p in new):
                        terminal_open = True
                        print("  TERMINAL OPENED via fallback — chain complete")
            else:
                print("  [terminal fallback] could not confirm Advanced — "
                      "skipping the tap (safety)")

        # Let the terminal children settle. 6-Z187: with the terminal fix
        # the pty child execs the STAGED busybox ash — give it 8s to draw
        # a PROMPT (and NOT print "Child processes exited.").
        wait(8.0)
        screenshot("term-07-terminal-settled")
        wait(4.0)
        screenshot("term-07b-terminal-prompt")

        # ── 6) BACK (bottom nav bar) -> returns to the Advanced page ──
        guest_back()
        wait(2.0)
        screenshot("term-08-after-back")

        # ── 7) Reopen File Manager — THE MONEY SHOT ──
        # On the physical device this listing showed the ENTIRE host FS
        # (/data restricted) after the failed terminal. Compare against
        # term-04-fm-root-BEFORE-terminal: identical view + this step is
        # the ONLY difference.
        if fm_open:
            # BACK from the terminal returns to the FM (or Advanced);
            # navigate to the FM again via its Advanced-page button.
            # 6-Z188b: MARKER-GATED — only tap FM/Terminal grid buttons
            # when the last Set page is 'advanced'; if we drifted to the
            # main menu (run 33125036396), enter Advanced first via its
            # main-menu button (192,1294).
            def _ensure_advanced(tag):
                for _ in range(3):
                    if _back_off_danger(adv_pos):
                        continue
                    last = _last_page_after(adv_pos)
                    if last == "advanced":
                        return True
                    if last in ("main2", "main", "clear_vars"):
                        tap(192, 1294)
                        wait(2.5)
                        continue
                    guest_back()
                    wait(1.5)
                return _last_page_after(adv_pos) == "advanced"

            if not _ensure_advanced("reopen"):
                print("  [reopen] could not reach Advanced — skipping FM reopen taps")
            for (x, y) in [(528, 708), (528, 708)]:
                if _last_page_after(adv_pos) != "advanced":
                    break
                print(f"  tap File Manager reopen candidate: ({x},{y})")
                tap(x, y)
                wait(2.5)
                if _back_off_danger(adv_pos):
                    continue
                break
            screenshot("term-09-fm-AFTER-terminal")
            # 6-Z187: press `system` FIRST (unscrolled position — the
            # user's complaint was that scrolling made the row unreachable).
            for i, sy in enumerate([1170, 1050, 930, 810]):
                print(f"  tap system-folder candidate AFTER-{i}: (360,{sy})")
                tap(360, sy)
                wait(1.5)
                if _back_off_danger(fm_pos):
                    continue
                screenshot(f"term-11-system-{i}")
                input_cmd("input keyevent 4")
                wait(1.0)
            for i in range(3):
                swipe_up()
                wait(1.0)
                screenshot(f"term-10-scroll-{i}")
            screenshot("term-12-final")
        else:
            screenshot("term-09-fm-AFTER-terminal-SKIPPED")

        # Back out of whatever page we ended on.
        for _ in range(6):
            guest_back()
            wait(0.5)
        screenshot("term-13-after-back-all")

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

    # ── 6-Z158: pull the GUEST rootfs internals via run-as ──
    # After the boot wait the staged rootfs holds the guest's own
    # runtime evidence: TWRP recovery's own log (/tmp/recovery.log —
    # written as soon as recovery main() runs), the __kmsg__ stub (all
    # guest kernel-log writes incl. init/recovery fatals that never
    # reach logcat), and the selinux stubs the init dance wrote. These
    # pin down post-restorecon blockers (recovery exit 127 family)
    # without another dispatch cycle.
    # Rootfs top-level listing — settles hardware-detection questions
    # (6-Z159a) in one run: exactly what the kr64 parent saw.
    listing = adb_shell(f"run-as {PACKAGE} ls -la rootfs/", timeout=30)
    if listing:
        with open(os.path.join(ART, "rootfs-listing.txt"), "w", errors="replace") as f:
            f.write(listing)
        print(f"  pulled rootfs listing ({len(listing)} bytes)")
    # 6-Z185 sandbox evidence: the GUEST's /system tree. The File
    # Manager probe screenshots are compared against THIS listing — if
    # the FM shows entries that are not here, the guest is seeing the
    # HOST's /system (escape). For the TWRP ramdisk this listing is
    # expected to be just usr/ (+ tzdata under it). The app's rootfs
    # dir may BE a symlink to the active profile's rootfs — probe both
    # locations so the evidence is never empty by accident.
    seen = set()
    for base in ("rootfs", "profiles/default/rootfs"):
        for leaf in ("system", "system/app", "vendor"):
            remote = f"{base}/{leaf}"
            if remote in seen:
                continue
            seen.add(remote)
            out = adb_shell(f"run-as {PACKAGE} ls -la {remote}", timeout=30)
            if out:
                local = "rootfs-" + leaf.replace("/", "-") + "-listing.txt"
                with open(os.path.join(ART, local), "w", errors="replace") as f:
                    f.write(f"# {remote}\n{out}\n")
                print(f"  pulled sandbox evidence: {remote} ({len(out)} bytes)")
    for remote, local in [
        ("rootfs/tmp/recovery.log", "twrp-recovery.log"),
        ("rootfs/dev/__kmsg__", "kmsg-stub.txt"),
        ("rootfs/sys/fs/selinux/null", "selinux-null-stub.txt"),
        ("rootfs/sys/fs/selinux/enforce", "selinux-enforce-stub.txt"),
        # 6-Z162 FIX: the staged-exe marker lives INSIDE the rootfs
        # (rootfs/.twoyi-staged — see ptrace_emu::staged_exes_marker_path).
        # The pre-6-Z162 path (.twoyi-staged, relative to the data dir)
        # NEVER existed, which is why staged-exes.txt was silently
        # missing from every artifact since 6-Z158.
        ("rootfs/.twoyi-staged", "staged-exes.txt"),
        # 6-Z162: the app-side tee of the tracer log + the ramdisk's own
        # init-diag file (62 bytes on run 32983937665, never pulled).
        ("rootfs/twrp-init.log", "twrp-init-rootfs.txt"),
        ("rootfs/twrp-cmdline", "twrp-cmdline.txt"),
    ]:
        out = adb_shell(f"run-as {PACKAGE} cat {remote}", timeout=60)
        if out:
            with open(os.path.join(ART, local), "w", errors="replace") as f:
                f.write(out)
            print(f"  pulled rootfs evidence: {remote} -> {local} ({len(out)} bytes)")

    # ── 6-Z162: directory listings that settle linker questions ──────
    # Run 32983937665: "Service 'recovery' (pid 2619/2641) exited with
    # status 1" + "Service 'adbd' ... exited with status 1" — the instant
    # exit-1 family is a dynamic-linker failure symptom, but we could
    # never SEE /sbin's contents (does linker64 exist? libc.so? which
    # libs did the angler ramdisk ship?). These listings answer that in
    # one run. All failure-tolerant: a missing dir just yields an error
    # string in the file (still evidence).
    for remote, local in [
        ("ls -la rootfs/sbin/", "sbin-contents.txt"),
        ("ls -la rootfs/tmp/ rootfs/lib64/ rootfs/system/lib/ 2>&1", "tmp-lib-dirs.txt"),
        ("ls -la rootfs/system/lib64/ 2>&1 | head -60", "system-lib64.txt"),
        ("ls -la rootfs/etc/ 2>&1 | head -40", "etc-contents.txt"),
        # 6-Z169: run 33007215600 — the importer extracted ALL 3217 cpio
        # entries (app log: sawTrailer=true) yet PageManager's loads of
        # /twres/splash.xml + /twres/languages/en.xml ENOENT'd (TWRP:
        # "E:Package splash failed to load" / "E:Unable to load
        # '/twres/languages/en.xml'"). Recursive listing + the four key
        # theme files' sizes settle whether the files exist at PULL time
        # (vs. runtime — the tracer-side 6-Z169 dir dump covers runtime).
        ("ls -laR rootfs/twres/ 2>&1 | head -120", "twres-contents.txt"),
        ("wc -c rootfs/twres/splash.xml rootfs/twres/ui.xml rootfs/twres/portrait.xml rootfs/twres/languages/en.xml 2>&1", "twres-keyfiles.txt"),
    ]:
        out = adb_shell(f"run-as {PACKAGE} sh -c '{remote}'", timeout=60)
        if out:
            with open(os.path.join(ART, local), "w", errors="replace") as f:
                f.write(out)
            print(f"  pulled listing: {remote} -> {local} ({len(out)} bytes)")

    # ── 6-Z162: manual service-exec probes — the definitive ──────────
    # exit-1 diagnosis. The guest's services die inside the jail where
    # their stderr is invisible — but the tracer's STAGED copies in
    # cache/twoyi_stage/ are exec-allowed AND their PT_INTERP is already
    # rewritten to the HOST-absolute rootfs linker64 (Task 6-Z50/6-Z157),
    # so run-as can exec them DIRECTLY. The bionic linker then prints
    # "CANNOT LINK"/"library 'X' not found"/"cannot locate symbol"
    # straight to OUR captured stderr — the exact lines the jail swallows.
    # NOTE: a fresh `cp` of rootfs/sbin/<bin> would NOT work — its
    # PT_INTERP is still the GUEST-absolute /sbin/linker64, which does
    # not exist outside the jail (execve → ENOENT). Only the marker's
    # staged entries are patched.
    # toybox `timeout` bounds each probe so a probe that actually RUNS
    # (recovery may sit in its event loop for its whole lifetime) cannot
    # hang the step.
    staged_marker = adb_shell(f"run-as {PACKAGE} cat rootfs/.twoyi-staged", timeout=30) or ""
    staged_pairs = []  # [(guest_path, cache_path)]
    for line in staged_marker.splitlines():
        parts = line.strip().split("\t")
        if len(parts) == 2 and parts[0].startswith("/") and parts[1].startswith("/"):
            staged_pairs.append((parts[0], parts[1]))
    with open(os.path.join(ART, "service-exec-probe-MARKER.txt"), "w", errors="replace") as f:
        f.write(staged_marker or "(marker missing/empty)\n")

    base_env = (
        "LD_LIBRARY_PATH=/data/user/0/{pkg}/rootfs/sbin:"
        "/data/user/0/{pkg}/rootfs/system/lib:"
        "/data/user/0/{pkg}/rootfs/system/lib64"
    ).format(pkg=PACKAGE)
    preload_env = (
        "LD_PRELOAD=/data/user/0/" + PACKAGE + "/rootfs/sbin/libtwrp_fb_hook.so " + base_env
    )
    # 6-Z163d FIX (the big one): POSIX sh does NOT export bare `VAR=x;`
    # assignments to child processes (verified: `sh -c 'A=1; env' | grep A`
    # prints NOTHING on dash/mksh/toybox). The pre-6-Z163d probes joined
    # the env with `;` — LD_PRELOAD/LD_LIBRARY_PATH never reached the
    # staged binary, and the linker's "libaosprecovery.so not found" was
    # just the NO-ENV fallback (/system/lib64 has no TWRP libs). The env
    # strings above are now PREFIX ASSIGNMENTS on the actual command
    # (space-separated, no semicolons) so they are guaranteed to reach
    # the exec'd binary through timeout's environ.
    # 6-Z163 fixup: `timeout` must be invoked by its HOST-absolute path —
    # the probe env deliberately points PATH at the ROOTFS dirs (so the
    # probe resolves tools the way the jailed service would), which hid
    # redroid's own /system/bin/timeout on run 32988644183 ("sh: timeout:
    # inaccessible or not found" → PROBE_EXIT_CODE:127 on every probe).
    TIMEOUT = "/system/bin/timeout"
    # Probe targets: every staged exe whose guest path matches the dying
    # services (recovery, adbd, ueventd...). The nopreload differential
    # variant rules the fb-hook in/out as the loader failure source.
    wanted = [p for p in staged_pairs if p[0].endswith(
        ("/sbin/recovery", "/sbin/adbd", "/sbin/ueventd", "/sbin/linker64"))]
    for guest_path, cache_path in wanted:
        base = guest_path.rsplit("/", 1)[-1]
        for variant, env_str in [("", preload_env), ("-nopreload", base_env)]:
            label = f"{base}{variant}"
            cmd = (
                f"run-as {PACKAGE} sh -c 'ls -la {cache_path}; "
                f"{TIMEOUT} 8 env TWOYI_ROOTFS=/data/user/0/{PACKAGE}/rootfs {env_str} {cache_path} 2>&1; "
                f"echo PROBE_EXIT_CODE:$?'"
            )
            out = adb_shell(cmd, timeout=30)
            with open(os.path.join(ART, f"service-exec-probe-{label}.txt"), "w",
                      errors="replace") as f:
                f.write(f"$ {cmd}\n{out or '(no output)'}\n")
            print(f"  probe {label}: captured ({len(out or '')} bytes)")

    # ── 6-Z163b: WHY did the linker say "libaosprecovery.so not found"
    # when /sbin/libaosprecovery.so EXISTS (34544 bytes, mode 0755)?
    # Either the file is corrupt (bad ELF magic — the ramdisk extraction
    # may have produced garbage) or the old linker's search failed for a
    # subtler reason. Hexdump the first 32 bytes of every DT_NEEDED
    # candidate the probe env points at: a VALID arm64 shared lib starts
    # 7f 45 4c 46 02 01 01 00 (ELF magic, ELFCLASS64, LSByte, current).
    magic_out = adb_shell(
        f"run-as {PACKAGE} sh -c 'for f in libaosprecovery.so libc.so "
        f"libcrecovery.so liblog.so libminuitwrp.so linker64 recovery adbd; do "
        f"echo \"== $f ==\"; ls -la rootfs/sbin/$f 2>&1; "
        f"dd if=rootfs/sbin/$f bs=1 count=32 2>/dev/null | od -An -tx1; done'",
        timeout=60)
    if magic_out:
        with open(os.path.join(ART, "sbin-lib-magic.txt"), "w", errors="replace") as f:
            f.write(magic_out)
        print(f"  pulled sbin lib magic dump ({len(magic_out)} bytes)")

    # LD_DEBUG=1 variant for recovery — the OLD bionic linker prints its
    # library search decisions (which dirs it tried, what failed) when
    # LD_DEBUG is set. Harmless if unsupported (env is just ignored).
    ld_debug_out = adb_shell(
        f"run-as {PACKAGE} sh -c '{TIMEOUT} 8 env LD_DEBUG=1 TWOYI_ROOTFS=/data/user/0/{PACKAGE}/rootfs {preload_env} "
        f"/data/user/0/{PACKAGE}/cache/twoyi_stage/_sbin_recovery_* 2>&1 | head -60; "
        f"echo LDDEBUG_EXIT:$?'",
        timeout=30)
    if ld_debug_out:
        with open(os.path.join(ART, "recovery-ld-debug.txt"), "w", errors="replace") as f:
            f.write(ld_debug_out)
        print(f"  pulled recovery LD_DEBUG output ({len(ld_debug_out)} bytes)")

    # ── 6-Z163c: ldd-mode trace — LD_TRACE_LOADED_OBJECTS=1 makes even the
    # OLD bionic linker print the full DT_NEEDED walk (every lib + the
    # path it resolved to, or the not-found line) WITHOUT executing main.
    # This settles WHERE the old linker searches when it claims
    # libaosprecovery.so "not found" despite the file existing in the
    # very first LD_LIBRARY_PATH dir. Plus: run the linker64 DIRECTLY
    # with the staged binary as its argv (old linkers support this) —
    # a second, independent view of the same walk.
    staged_recovery = adb_shell(
        f"run-as {PACKAGE} sh -c 'ls /data/user/0/{PACKAGE}/cache/twoyi_stage/_sbin_recovery_*'",
        timeout=30) or ""
    staged_recovery = staged_recovery.splitlines()[0].strip() if staged_recovery.strip() else ""
    if staged_recovery:
        ldd_out = adb_shell(
            f"run-as {PACKAGE} sh -c 'env LD_TRACE_LOADED_OBJECTS=1 TWOYI_ROOTFS=/data/user/0/{PACKAGE}/rootfs {base_env} "
            f"{staged_recovery} 2>&1; echo LDD_EXIT:$?'",
            timeout=30)
        if ldd_out:
            with open(os.path.join(ART, "recovery-ldd.txt"), "w", errors="replace") as f:
                f.write(ldd_out)
            print(f"  pulled recovery ldd trace ({len(ldd_out)} bytes)")
        # The old linker ignores LD_TRACE_LOADED_OBJECTS and EXECUTES the
        # binary — so the "ldd" trace above is actually TWRP recovery's
        # own boot log, and TWRP also mirrors it to /tmp/recovery.log
        # (redroid's /tmp is writable from the probe context). Pull that
        # too — it is LONGER than stdout (LOGE-only lines go there).
        reclog = adb_shell(f"run-as {PACKAGE} sh -c 'cat /tmp/recovery.log 2>&1; rm -f /tmp/recovery.log'", timeout=30)
        if reclog:
            with open(os.path.join(ART, "recovery-probe-tmp-log.txt"), "w",
                      errors="replace") as f:
                f.write(reclog)
            print(f"  pulled /tmp/recovery.log from probe run ({len(reclog)} bytes)")
        direct_out = adb_shell(
            f"run-as {PACKAGE} sh -c '{TIMEOUT} 8 env TWOYI_ROOTFS=/data/user/0/{PACKAGE}/rootfs {base_env} "
            f"/data/user/0/{PACKAGE}/rootfs/sbin/linker64 {staged_recovery} 2>&1 | head -40; "
            f"echo DIRECT_LINKER_EXIT:$?'",
            timeout=30)
        if direct_out:
            with open(os.path.join(ART, "recovery-direct-linker.txt"), "w",
                      errors="replace") as f:
                f.write(direct_out)
            print(f"  pulled direct linker64 trace ({len(direct_out)} bytes)")

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
    # 6-Z184: kr64 mirrors these to /sdcard/Download/twoyi-logs/ (public,
    # adb-pullable on every build — see kr64 lib.rs ext_files_dir), and the
    # run-as fallback must use the ACTIVE package (io.twoyi.debug in CI),
    # not the hardcoded release id.
    pull_with_fallback(
        "/sdcard/Download/twoyi-logs/twrp-init.log",
        f"/data/user/0/{PACKAGE}/rootfs/twrp-init.log",
        os.path.join(ART, "twrp-init.log"))
    pull_with_fallback(
        "/sdcard/Download/twoyi-logs/twrp-kmsg.log",
        f"/data/user/0/{PACKAGE}/rootfs/twrp-kmsg.log",
        os.path.join(ART, "twrp-kmsg.log"))
    pull_with_fallback(
        "/sdcard/Download/twoyi-logs/dev-__kmsg__",
        f"/data/user/0/{PACKAGE}/rootfs/dev/__kmsg__",
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

    # ── ARM64-T2: final launcher-state assertion ──
    # The post-Launch-Container check at step 6 catches the IMMEDIATE
    # crash, but a slower failure (app boots ptrace, runs for 30-200s,
    # then crashes mid-way) would slip past step 6 and produce the
    # same false-green workflow that 32886902337 did.
    #
    # Re-check the final activity here: if we're back on the system
    # launcher at the end of the boot wait, TWRP never came up —
    # FAIL the workflow with exit 1 so the false-green pattern stops.
    final_activity = get_current_activity() or ""
    print()
    print("=" * 60)
    print("  Final launcher-state check")
    print("=" * 60)
    print(f"  Final activity: {final_activity}")
    final_activity_lc = final_activity.lower()
    is_launcher = (
        "launcher" in final_activity_lc
        or "nexuslauncher" in final_activity_lc
        or final_activity_lc.strip() == ""
    )
    # Render2Activity is twoyi's render surface — being on it OR on any
    # non-launcher activity means the app is still alive (boot succeeded
    # OR is still in progress). Only the launcher means a crash.
    if is_launcher:
        print()
        print("  ✗✗✗ FINAL STATE IS LAUNCHER — TWRP NEVER BOOTED")
        print("  The twoyi app is no longer the resumed activity after")
        print(f"  {boot_wait}s of boot wait. This is the false-green")
        print("  pattern from run 32886902337 — failing the workflow now.")
        print()
        print("  Diagnostic artifacts captured above. Exiting with code 1.")
        sys.exit(1)
    print(f"  ✓ Final activity is non-launcher ({final_activity}) — app still "
          "alive; inspect 07_boot/08_final screenshots for the TWRP frame "
          "(the frame sequence is the render evidence).")

if __name__ == "__main__":
    main()
