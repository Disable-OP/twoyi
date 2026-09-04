#!/usr/bin/env python3
"""AOSP-recovery menu interactivity probe (6-Z289).

The corpus classifier marks AOSP-layout recoveries (Lineage / AOSP /
minui-era — no TWRP theme system) SPLASH_HANG even when the full menu is
rendered and waiting for input (run 33900850051: the menu was fully
drawn — "Reboot system now" / "Apply update" / "Factory reset" /
"Advanced" — and the main thread sat in WaitInputEvent; the TWRP-centric
"Set page:" detector can never fire for these images).

This probe supplies the MISSING ground truth: does the rendered menu
REACT to touch?  It taps SAFE menu rows (never the pre-highlighted first
row — on lineage that is "Reboot system now"; activating it would reboot
the guest) and records:

  * the twoyi fb0 blit-loop frame counter before/after the taps
    ("TWOYI_DIAG ... TWRP-FB frame #<n> blitted ..." in logcat) — the
    highlight move forces minui to redraw, so a rising frame counter
    after the taps PROVES guest pixels changed in response to host
    input: end-to-end touch delivery into the AOSP recovery;
  * screencap byte hashes before/after (secondary signal).

Everything lands in /tmp/ui-e2e-artifacts/menu-probe.json +
screenshot-menu-*.png for classify_result.py.

Gate: the probe runs ONLY when logcat shows the AOSP minui framebuffer
marker and NO TWRP pages — for TWRP-family runs ui-navigate.py already
owns navigation and the probe must not disturb it.

Import note: helpers are reused from ui-navigate.py (same input channel
ladder — adb → docker exec → app broadcast → sendevent — same
screenshot ladder), which keeps ONE battle-tested input path.
"""

import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
import time

_ART_DIR = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "ui_navigate", os.path.join(_ART_DIR, "ui-navigate.py"))
nav = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(nav)  # module main() is __main__-guarded

ART = "/tmp/ui-e2e-artifacts"
PROBE_JSON = os.path.join(ART, "menu-probe.json")


def logcat_dump():
    """Best-effort logcat snapshot text (adb first, docker exec fallback)."""
    out = ""
    try:
        r = subprocess.run(nav.ADB + ["shell", "logcat", "-d", "-v", "time"],
                           capture_output=True, text=True, timeout=20)
        out = (r.stdout or "") + (r.stderr or "")
        if "Beginning of" in out or len(out) > 200:
            return out
    except Exception:
        pass
    try:
        r = subprocess.run(
            ["sudo", "docker", "exec", "redroid", "sh", "-c",
             "/system/bin/logcat -d -v time"],
            capture_output=True, text=True, timeout=25)
        return (r.stdout or "") + (r.stderr or "")
    except Exception:
        return out


def blit_frame_counter(text):
    """Highest 'TWRP-FB frame #<n> blitted' counter in a logcat text."""
    frames = re.findall(r"TWRP-FB frame #(\d+) blitted", text)
    return max((int(n) for n in frames), default=0)


def is_aosp_menu_candidate(logcat_text):
    """AOSP-layout recovery: minui fbdev marker present, no TWRP pages."""
    has_minui_fb = bool(re.search(r"framebuffer: \d+ \(\d+ x \d+\)", logcat_text))
    has_twrp_pages = bool(re.findall(r"Set page: '([^']+)'", logcat_text))
    return has_minui_fb and not has_twrp_pages


def screen_size():
    try:
        out = nav.adb_shell("wm size", timeout=10)
        if hasattr(out, "stdout"):
            out = out.stdout
        m = re.search(r"(\d+)x(\d+)", out or "")
        if m:
            return int(m.group(1)), int(m.group(2))
    except Exception:
        pass
    try:
        r = subprocess.run(
            ["sudo", "docker", "exec", "redroid", "sh", "-c", "wm size"],
            capture_output=True, text=True, timeout=10)
        m = re.search(r"(\d+)x(\d+)", (r.stdout or ""))
        if m:
            return int(m.group(1)), int(m.group(2))
    except Exception:
        pass
    return 720, 1600


def screencap_bytes():
    """Raw PNG bytes via the ui-navigate screenshot ladder (no file)."""
    data = nav._screencap_adb()
    if not data:
        data = nav._screencap_docker()
    return data


def tap(x, y):
    nav.input_cmd(f"input tap {x} {y}")


def main():
    os.makedirs(ART, exist_ok=True)
    probe = {
        "schema": "twoyi.menu-probe/1",
        "ran": False,
        "reason": "",
        "frame_before": None,
        "frame_after": None,
        "taps": [],
        "cap_hashes": [],
        "cap_differs_after_taps": None,
    }

    lc = logcat_dump()
    if not is_aosp_menu_candidate(lc):
        probe["reason"] = (
            "not an AOSP-layout recovery (minui fb marker missing or TWRP "
            "pages present) — probe skipped, ui-navigate owns TWRP runs")
        with open(PROBE_JSON, "w") as f:
            json.dump(probe, f, indent=2)
        print(f"  [menu-probe] skip: {probe['reason']}")
        return

    W, H = screen_size()
    probe["ran"] = True
    probe["screen"] = [W, H]
    print(f"  [menu-probe] AOSP menu candidate — screen {W}x{H}")

    # Stability baseline: two captures ~4s apart of the untouched menu.
    c0 = screencap_bytes()
    time.sleep(4)
    c1 = screencap_bytes()
    probe["cap_hashes"].append([
        hashlib.sha256(c0).hexdigest()[:16] if c0 else None,
        hashlib.sha256(c1).hexdigest()[:16] if c1 else None,
    ])
    probe["baseline_stable"] = bool(c0 and c1 and c0 == c1)
    nav.screenshot("menu-00-baseline")

    frame_before = blit_frame_counter(logcat_dump())
    probe["frame_before"] = frame_before

    # Safe tap ladder. AOSP recovery lays menu rows out below the header
    # + battery block; rows sit roughly at y ≈ 0.32H with ≈0.07H pitch,
    # horizontally centered. Row 1 ("Reboot system now" on lineage) is
    # PRE-HIGHLIGHTED — activating a highlighted row is destructive, so
    # the ladder only ever taps NON-highlighted rows, moving the
    # selection down and back up. Each tap forces a minui redraw of the
    # changed highlight.
    row_y = [int(H * f) for f in (0.33, 0.40, 0.47)]
    cx = W // 2
    ladder = [row_y[1], row_y[2], row_y[1]]  # row2 -> row3 -> row2
    for i, y in enumerate(ladder):
        tap(cx, y)
        probe["taps"].append([cx, y])
        nav.screenshot(f"menu-{i+1:02d}-after-tap{i+1}")
        time.sleep(3)

    frame_after = blit_frame_counter(logcat_dump())
    probe["frame_after"] = frame_after

    c2 = screencap_bytes()
    probe["cap_hashes"].append(
        [hashlib.sha256(c2).hexdigest()[:16] if c2 else None])
    if c0 and c2:
        probe["cap_differs_after_taps"] = (c0 != c2)

    probe["frame_delta"] = (frame_after - frame_before
                            if frame_after is not None and
                            frame_before is not None else None)
    probe["interactive"] = bool(probe["frame_delta"] and probe["frame_delta"] > 0)

    with open(PROBE_JSON, "w") as f:
        json.dump(probe, f, indent=2)
    print(f"  [menu-probe] frame {frame_before} -> {frame_after} "
          f"(delta {probe['frame_delta']}), interactive={probe['interactive']}")


if __name__ == "__main__":
    main()
