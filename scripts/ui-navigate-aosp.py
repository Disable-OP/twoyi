#!/usr/bin/env python3
"""UI navigation for the AOSP (normal Android container) E2E test.

v2 (fixes from run 32628825953 — the run that never launched the container):
  - The app's LAUNCHER activity is SettingsActivity (a PreferenceScreen),
    NOT SelectAppActivity with app rows. The container boots by tapping
    the "Launch Container" preference (found by text, with scroll).
  - The `am start` fallback now ASSERTS the activity switched to
    Render2Activity and aborts early (the old script discarded the
    output and produced a green run with zero boot).
  - Log pulls fixed: FileLogger writes to /sdcard/Android/data/<pkg>/
    files/log/ (EXTERNAL — adb-pullable directly, no run-as needed),
    and kr64-app-stderr.log rotates to kr64.log. Full `adb logcat -d`
    is captured before the emulator dies.
  - The KR64-line count uses a real regex (not the logcat headers).
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

ADB = ["adb", "-s", "emulator-5554"]


def adb(*args, timeout=30):
    return subprocess.run(ADB + list(args), capture_output=True, text=True,
                          timeout=timeout)


def adb_shell(cmd, timeout=30):
    return adb("shell", cmd, timeout=timeout)


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


def find_by_text(root, text, exact=False):
    if root is None:
        return None
    for node in root.iter("node"):
        label = node.get("text", "") or node.get("content-desc", "")
        ok = (label == text) if exact else (text.lower() in label.lower())
        if ok:
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.get("bounds", ""))
            if m:
                x1, y1, x2, y2 = map(int, m.groups())
                return ((x1 + x2) // 2, (y1 + y2) // 2)
    return None


def scroll_down():
    adb_shell("input swipe 160 500 160 150 300", timeout=15)


def scroll_to_find(text, max_scrolls=6, exact=False):
    for _ in range(max_scrolls):
        path = dump_ui("scroll_probe")
        root = parse_ui(path)
        pos = find_by_text(root, text, exact)
        if pos:
            return pos
        scroll_down()
        time.sleep(1)
    return None


def tap(x, y):
    adb_shell(f"input tap {x} {y}", timeout=15)


def get_current_activity():
    out = adb_shell(
        "dumpsys activity activities | grep -E 'ResumedActivity|mResumedActivity'",
        timeout=20).stdout
    m = re.search(r"(\S+\.\S+/\S+)", out)
    return m.group(1) if m else "<unknown>"


def capture_logs():
    print("\n  Capturing logs...")
    r = adb_shell("logcat -d", timeout=60)
    with open(f"{ART}/logcat-full.txt", "w", errors="replace") as f:
        f.write(r.stdout)
    print(f"    logcat-full.txt: {len(r.stdout)} bytes")
    # 6-Z123c: post-boot ground truth for the security-sysctl trio —
    # the actual file modes after the guest ran (the EACCES hunt).
    for d in ("proc/sys/kernel", "proc/sys/vm", "dev/__properties__"):
        rr = adb_shell(f"run-as {PACKAGE} ls -la profiles/default/rootfs/{d}",
                       timeout=30)
        with open(f"{ART}/rootfs-{d.replace('/', '-')}.txt", "w",
                  errors="replace") as f:
            f.write(rr.stdout or "")
        print(f"    rootfs/{d}: {len(rr.stdout or '')} bytes")
    adb("pull", f"/sdcard/Android/data/{PACKAGE}/files/log",
        f"{ART}/app-logs", timeout=60)
    for remote, local in [
        ("cache/log/app.log", "app-internal.log"),
        ("cache/log/boot.log", "boot-internal.log"),
        ("cache/log/crash.log", "crash.log"),
        ("kr64-app-stderr.log", "kr64-app-stderr.log"),
        ("cache/kr64.log", "kr64.log"),
        ("files/kr64.log", "kr64-ext.log"),
    ]:
        rr = adb_shell(f"run-as {PACKAGE} cat {remote}", timeout=60)
        if rr.stdout:
            with open(f"{ART}/{local}", "w", errors="replace") as f:
                f.write(rr.stdout)
            print(f"    run-as {remote} -> {local} ({len(rr.stdout)} bytes)")
    os.makedirs(f"{ART}/twoyi-logs", exist_ok=True)
    adb_shell("cp -r /sdcard/Download/twoyi-logs /sdcard/twoyi-logs-copy 2>/dev/null; "
              "chmod -R 777 /sdcard/twoyi-logs-copy 2>/dev/null", timeout=15)
    adb("pull", "/sdcard/twoyi-logs-copy", f"{ART}/twoyi-logs", timeout=60)
    adb_shell("rm -rf /sdcard/twoyi-logs-copy", timeout=10)


def main():
    os.makedirs(ART, exist_ok=True)
    print("=" * 60)
    print(f"  AOSP E2E navigation v2 (package {PACKAGE})")
    print("=" * 60)

    print("\n  Step 1: Launch app via launcher")
    adb_shell(f"monkey -p {PACKAGE} -c android.intent.category.LAUNCHER 1")
    time.sleep(6)
    activity = get_current_activity()
    print(f"  Current activity: {activity}")
    dump_ui("01_app_launched")

    print("\n  Step 2: Tap 'Launch Container'")
    pos = scroll_to_find("Launch Container", max_scrolls=6)
    launched = False
    if pos:
        print(f"  Tapping 'Launch Container' at {pos}")
        tap(*pos)
        time.sleep(5)
        activity = get_current_activity()
        print(f"  Current activity: {activity}")
        launched = "Render2Activity" in activity
    else:
        print("  ✗ 'Launch Container' not found after scrolling")

    if not launched:
        print("  Fallback: am start Render2Activity (output asserted)")
        r = adb_shell(f"am start -n {PACKAGE}/io.twoyi.Render2Activity", timeout=20)
        print(f"  am start stdout: {r.stdout.strip()!r}")
        print(f"  am start stderr: {r.stderr.strip()!r}")
        time.sleep(6)
        activity = get_current_activity()
        print(f"  Current activity: {activity}")

    if "Render2Activity" not in activity:
        print("\n  ✗✗✗ CONTAINER NEVER LAUNCHED — activity is still "
              f"{activity!r}. Aborting early.")
        dump_ui("02_launch_failed")
        screenshot("08_launch_failed")
        capture_logs()
        sys.exit(1)

    dump_ui("02_after_launch_tap")
    print("  ✓ Render2Activity is foreground — the container is booting")

    print(f"\n  Step 3: Waiting {BOOT_WAIT}s for guest boot "
          f"(screenshots every {SCREENSHOT_EVERY}s)")
    t0 = time.time()
    shot = 0
    md5s = {}
    while time.time() - t0 < BOOT_WAIT:
        shot += 1
        p = screenshot(f"07_boot_{shot * SCREENSHOT_EVERY}s")
        h = md5(p)
        md5s[h] = md5s.get(h, 0) + 1
        if shot % (60 // SCREENSHOT_EVERY) == 0:
            print(f"    t={int(time.time() - t0)}s "
                  f"activity={get_current_activity()} shots={shot}")
            dump_ui(f"08_progress_{int(time.time() - t0)}s")
        time.sleep(SCREENSHOT_EVERY)

    dump_ui("09_final")
    capture_logs()

    print("\n" + "=" * 60)
    print("  GROUND TRUTH SUMMARY")
    print("=" * 60)
    print(f"  screenshot md5 distribution: {md5s}")

    kr64_count = 0
    try:
        with open(f"{ART}/logcat-full.txt", errors="replace") as f:
            kr64_count = sum(1 for line in f if re.search(r"\bKR64\b", line))
    except OSError:
        pass
    print(f"  KR64 lines in full logcat: {kr64_count}")
    if kr64_count == 0:
        print("  ✗✗✗ ZERO KR64 lines — the container daemon never ran. FAILED run.")
        sys.exit(1)

    for logf in ("kr64.log", "kr64-app-stderr.log", "boot-internal.log",
                 "app-internal.log"):
        try:
            content = open(f"{ART}/{logf}", errors="replace").read()
            n = content.count("BOOT_COMPLETED")
            print(f"  BOOT_COMPLETED mentions in {logf}: {n}")
        except OSError:
            pass
    print("\n  Done.")


if __name__ == "__main__":
    main()
