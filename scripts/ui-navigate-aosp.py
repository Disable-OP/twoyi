#!/usr/bin/env python3
"""UI navigation for the AOSP (normal Android container) E2E test.

Differences from the TWRP variant (ui-navigate.py):
  - The guest rootfs is PRELOADED by the workflow (extracted from
    cyanmint/twoyi release profile_default_export.tar.xz into
    profiles/default/rootfs via run-as on the debuggable variant,
    io.twoyi.debug) — there is NO in-app ROM import step.
  - The container boots in NORMAL mode (Boot to Recovery OFF): the
    guest is a full Android 8.1 x86 system; rendering goes through
    the emugl OpenGL renderer (NOT the fb0 file reader).
  - The app's launcher activity (SelectAppActivity) lists host apps
    that can boot inside the container. We tap the FIRST enabled row
    (the "normal phone user" flow). Fallback: am start Render2Activity.

Steps:
  1. Launch app via monkey -p io.twoyi.debug
  2. Dump UI; find + tap the first app row (or fallback am start)
  3. Wait for boot, screenshot every 5s (BOOT_WAIT_SECONDS, default 600)
  4. Pull ALL app logs via run-as (the debuggable variant makes this
     work — the TWRP E2E's app-logs were always empty because the
     release build rejects run-as):
       - cache/log/{app,boot,crash,logcat}.log
       - dataDir/kr64-app-stderr.log
       - /sdcard/Download/twoyi-logs/ (kr64's post-mortem mirror)
  5. Print a ground-truth summary (SurfaceView visibility, boot
     completion markers, screenshot md5 variation).
"""

import hashlib
import os
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

PACKAGE = os.environ.get("AOSP_PACKAGE", "io.twoyi.debug")
ART = "/tmp/ui-e2e-artifacts"
BOOT_WAIT = int(os.environ.get("BOOT_WAIT_SECONDS", "600"))
SCREENSHOT_EVERY = 5


def adb(*args, timeout=30):
    return subprocess.run(
        ["adb", "-s", "emulator-5554"] + list(args),
        capture_output=True, text=True, timeout=timeout)


def adb_shell(cmd, timeout=30):
    return adb("shell", cmd, timeout=timeout)


def run_as(cmd, timeout=30):
    """Run a command as the app via run-as (debuggable variant only)."""
    return adb_shell(f"run-as {PACKAGE} {cmd}", timeout=timeout)


def screenshot(name):
    path = f"{ART}/{name}.png"
    adb_shell(f"screencap -p /sdcard/{name}.png", timeout=15)
    adb("pull", f"/sdcard/{name}.png", path, timeout=15)
    adb_shell(f"rm /sdcard/{name}.png", timeout=10)
    return path


def md5(path):
    try:
        with open(path, "rb") as f:
            return hashlib.md5(f.read()).hexdigest()
    except OSError:
        return "<missing>"


def dump_ui(name):
    path = f"{ART}/{name}.xml"
    adb_shell("uiautomator dump /sdcard/ui.xml", timeout=20)
    adb("pull", "/sdcard/ui.xml", path, timeout=15)
    adb_shell("rm /sdcard/ui.xml", timeout=10)
    return path


def parse_ui(xml_path):
    try:
        tree = ET.parse(xml_path)
        return tree.getroot()
    except (ET.ParseError, OSError):
        return None


def find_tappable_rows(root):
    """Find clickable node bounds in document order (the app list rows)."""
    rows = []
    if root is None:
        return rows
    for node in root.iter("node"):
        if node.get("clickable") == "true":
            bounds = node.get("bounds", "")
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
            if m:
                x1, y1, x2, y2 = map(int, m.groups())
                w, h = x2 - x1, y2 - y1
                # Skip tiny controls (checkboxes etc.) — want list rows
                if h >= 60 and w >= 200:
                    rows.append(((x1 + x2) // 2, (y1 + y2) // 2,
                                 node.get("text", "") or node.get("content-desc", "")))
    return rows


def tap(x, y):
    adb_shell(f"input tap {x} {y}", timeout=15)


def get_current_activity():
    out = adb_shell(
        "dumpsys activity activities | grep -E 'ResumedActivity|mResumedActivity'",
        timeout=20).stdout
    m = re.search(r"(\S+\.\S+/\S+)", out)
    return m.group(1) if m else "<unknown>"


def pull_run_as(remote, local):
    """Pull a file from the app's private dir via run-as cat."""
    r = adb_shell(f"run-as {PACKAGE} cat {remote}", timeout=60)
    if r.returncode == 0 and r.stdout:
        with open(local, "w", errors="replace") as f:
            f.write(r.stdout)
        return True
    return False


def main():
    os.makedirs(ART, exist_ok=True)
    print("=" * 60)
    print(f"  AOSP E2E navigation (package {PACKAGE})")
    print("=" * 60)

    # ── Step 1: Launch the app ─────────────────────────────────────
    print("\n  Step 1: Launch app via launcher")
    adb_shell(f"monkey -p {PACKAGE} -c android.intent.category.LAUNCHER 1")
    time.sleep(6)
    activity = get_current_activity()
    print(f"  Current activity: {activity}")
    dump_ui("01_app_launched")

    # ── Step 2: Boot the container ────────────────────────────────
    # Normal flow: SelectAppActivity lists host apps; tapping one boots
    # the container. Tap the first substantial row.
    root = parse_ui(f"{ART}/01_app_launched.xml")
    rows = find_tappable_rows(root)
    booted_via = None
    if rows:
        cx, cy, label = rows[0]
        print(f"\n  Step 2: Tapping first app row at ({cx},{cy}) label={label!r}")
        tap(cx, cy)
        time.sleep(5)
        booted_via = "ui-row-tap"
    else:
        print("\n  Step 2: No app rows found — falling back to am start Render2Activity")
        adb_shell(f"am start -n {PACKAGE}/io.twoyi.Render2Activity")
        time.sleep(5)
        booted_via = "am-start-fallback"

    activity = get_current_activity()
    print(f"  Current activity: {activity} (boot via {booted_via})")
    dump_ui("02_after_boot_tap")

    # ── Step 3: Boot wait + screenshots ───────────────────────────
    print(f"\n  Step 3: Waiting {BOOT_WAIT}s for guest boot (screenshots every {SCREENSHOT_EVERY}s)")
    t0 = time.time()
    shot = 0
    md5s = {}
    while time.time() - t0 < BOOT_WAIT:
        shot += 1
        p = screenshot(f"07_boot_{shot * SCREENSHOT_EVERY}s")
        h = md5(p)
        md5s[h] = md5s.get(h, 0) + 1
        # every 60s: note the activity + grab a UI dump
        if shot % (60 // SCREENSHOT_EVERY) == 0:
            print(f"    t={int(time.time() - t0)}s activity={get_current_activity()} shots={shot}")
            dump_ui(f"08_progress_{int(time.time() - t0)}s")
        time.sleep(SCREENSHOT_EVERY)

    dump_ui("09_final")

    # ── Step 4: Pull logs via run-as ──────────────────────────────
    print("\n  Step 4: Pulling app logs via run-as")
    pulls = [
        ("cache/log/app.log", "app.log"),
        ("cache/log/boot.log", "boot.log"),
        ("cache/log/crash.log", "crash.log"),
        ("cache/log/logcat.log", "logcat-guest.log"),
        ("kr64-app-stderr.log", "kr64-app-stderr.log"),
    ]
    for remote, local in pulls:
        if pull_run_as(remote, f"{ART}/{local}"):
            print(f"    pulled {remote} -> {local} ({os.path.getsize(f'{ART}/{local}')} bytes)")
        else:
            print(f"    (no {remote})")

    # kr64's post-mortem mirror (public dir — works on any variant)
    adb_shell("ls -la /sdcard/Download/twoyi-logs/ 2>/dev/null", timeout=10)
    os.makedirs(f"{ART}/twoyi-logs", exist_ok=True)
    adb_shell("cp -r /sdcard/Download/twoyi-logs /sdcard/twoyi-logs-copy 2>/dev/null; chmod -R 777 /sdcard/twoyi-logs-copy 2>/dev/null", timeout=15)
    adb("pull", "/sdcard/twoyi-logs-copy", f"{ART}/twoyi-logs", timeout=60)
    adb_shell("rm -rf /sdcard/twoyi-logs-copy", timeout=10)

    # ── Step 5: Ground truth summary ──────────────────────────────
    print("\n" + "=" * 60)
    print("  GROUND TRUTH SUMMARY")
    print("=" * 60)
    print(f"  screenshot md5 distribution: {md5s}")
    identical = len(md5s) == 1
    print(f"  PIXELS {'FROZEN (all identical — nothing rendered!)' if identical else 'CHANGED over time!'}")

    # SurfaceView / UI checks in the final dump
    try:
        final = open(f"{ART}/09_final.xml", errors="replace").read()
        print(f"  SurfaceView in final UI dump: {'SurfaceView' in final}")
        print(f"  loadingLayout in final UI dump: {'loadingLayout' in final}")
    except OSError:
        print("  (no final UI dump)")

    # Boot completion markers from the pulled logs
    for logf in ("kr64-app-stderr.log", "boot.log", "app.log"):
        try:
            content = open(f"{ART}/{logf}", errors="replace").read()
            n = content.count("BOOT_COMPLETED")
            print(f"  BOOT_COMPLETED mentions in {logf}: {n}")
        except OSError:
            pass

    logcat = adb_shell("logcat -d -s KR64:I 2>/dev/null", timeout=30).stdout
    print(f"  KR64 lines in live logcat: {len(logcat.splitlines())}")
    with open(f"{ART}/kr64-logcat-tail.txt", "w") as f:
        f.write(logcat[-200000:])

    print("\n  Done.")


if __name__ == "__main__":
    main()
