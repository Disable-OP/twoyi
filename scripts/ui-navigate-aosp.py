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

# 6-Z127: a CompletedProcess stand-in for timed-out/failed adb calls.
# Run 32777004259: the guest's ptrace storm starved the emulator so
# hard that a 15s `screencap` TIMED OUT — and the raised
# TimeoutExpired killed the WHOLE navigation script at t=71s, losing
# the remaining 530s of the boot window and all post-run captures.
# Every adb call below is now failure-tolerant: a timeout logs a note
# and yields an empty result instead of crashing the run.
class AdbTimeout:
    def __init__(self):
        self.stdout = ""
        self.stderr = "<adb timeout>"
        self.returncode = -1


def adb(*args, timeout=30):
    try:
        return subprocess.run(ADB + list(args), capture_output=True,
                              text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        print(f"    [adb-timeout] {' '.join(args)[:120]} "
              f"({timeout}s) — emulator starved; continuing")
        return AdbTimeout()
    except OSError as e:
        print(f"    [adb-error] {e} — continuing")
        return AdbTimeout()


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
    # 6-Z126: guest-binary INTEGRITY — md5 + first bytes of the key
    # runtime binaries/libs on the rootfs vs the emulator's originals.
    # If the extraction/symlink-rewrite corrupted a library, the guest's
    # linker/ART parse garbage and scudo aborts (run 32773072503's
    # zygote-child SIGABRT) — this pins it in one shot.
    checks = [
        "system/lib/libc++.so", "system/lib64/libc++.so",
        "system/lib64/libc.so", "system/lib64/libart.so",
        "system/lib64/libartbase.so", "system/bin/app_process64",
        "system/bin/app_process32", "system/bin/linker64",
        "system/lib/libdl.so", "system/lib64/libdl.so",
        "system/lib64/libbase.so", "system/lib64/libm.so",
        "system/lib64/libunwindstack.so",
        "apex/com.android.art/lib64/libnativeloader.so",
        # 6-Z137: the quota-truncation victims + their kin.
        "system/lib64/libutils.so", "system/lib64/libnblog.so",
        "system/lib64/libaudioclient.so", "system/lib64/libbinder.so",
        "system/lib64/liblog.so", "system/lib64/libz.so",
    ]
    lines = []
    for rel in checks:
        guest = adb_shell(
            f"run-as {PACKAGE} sh -c 'md5sum profiles/default/rootfs/{rel}; "
            f"od -An -tx1 -N16 profiles/default/rootfs/{rel}' "
            f"2>&1 | head -3", timeout=30)
        host = adb_shell(
            f"sh -c 'md5sum /{rel}; od -An -tx1 -N16 /{rel}' "
            f"2>&1 | head -3", timeout=30)
        lines.append(f"== {rel} ==")
        lines.append("guest(rootfs): " + (guest.stdout or "").strip())
        lines.append("host(emulator): " + (host.stdout or "").strip())
        match = (guest.stdout or "").split()[:1] == (host.stdout or "").split()[:1] \
            and bool((guest.stdout or "").strip())
        lines.append(f"md5 match: {'YES' if match else 'NO'}")
    with open(f"{ART}/rootfs-integrity.txt", "w", errors="replace") as f:
        f.write("\n".join(lines) + "\n")
    print("    rootfs-integrity.txt written")
    # 6-Z134: READ-PROBE the exact files the guest linker failed on —
    # how many bytes can the APP UID actually read, and what's on disk?
    # (The "only found 1 bytes"/"file size 0" link failures: this pins
    # whether the FILE is broken or the READ path is.)
    probes = [
        # (label, shell command — runs via adb shell, app uid via run-as)
        ("guest /dev staged libs",
         f"run-as {PACKAGE} sh -c 'ls -la rootfs/dev/*.so 2>&1; "
         f"wc -c rootfs/dev/libgetpid_hook.so rootfs/dev/libtwoyi_loader_shlib.so "
         f"rootfs/dev/libdl.so 2>&1'"),
        ("guest /dev libgetpid_hook 64-byte read",
         f"run-as {PACKAGE} sh -c 'dd if=rootfs/dev/libgetpid_hook.so bs=64 count=1 "
         f"2>/dev/null | od -An -tx1 | head -1'"),
        ("host /apex runtime bionic (app view)",
         "ls -la /apex/com.android.runtime/lib64/bionic/libdl.so "
         "/apex/com.android.runtime/lib64/bionic/libc.so 2>&1"),
        ("host /apex libdl 64-byte read (shell uid)",
         "dd if=/apex/com.android.runtime/lib64/bionic/libdl.so bs=64 count=1 "
         "2>/dev/null | od -An -tx1 | head -1"),
        ("host /apex art+i18n dirs",
         "ls /apex/com.android.art/lib64/libnativeloader.so "
         "/apex/com.android.i18n/lib64/libandroidicu.so 2>&1"),
        ("rootfs apex stub tree",
         f"run-as {PACKAGE} sh -c 'ls -la rootfs/apex/com.android.runtime/lib64/bionic/ "
         "2>&1 | head -12'"),
    ]
    probed = []
    for label, cmd in probes:
        rr = adb_shell(cmd, timeout=30)
        probed.append(f"== {label} ==\n{rr.stdout or rr.stderr or ''}")
    with open(f"{ART}/file-read-probes.txt", "w", errors="replace") as f:
        f.write("\n".join(probed) + "\n")
    print("    file-read-probes.txt written")
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
    # 6-Z134: enable the bionic linker's own debug log for the guest
    # (LD_DEBUG=2 "libs") via the marker file kr64 checks — the linker's
    # account of every library search/open/read is the ground truth for
    # the "only found 1 bytes" / "file size 0 >= 0" link-failure class.
    #
    # 6-Z154: DISABLED by default — the LD_DEBUG output generates a write
    # syscall PER symbol lookup, and the tracer intercepts + logs each
    # one. With 2-3M syscalls per service boot, this overloaded the
    # tracer (kr64 log = 81 MB / 1.14M lines in Z153 run) and froze the
    # emulator at 65s (21 adb-timeouts, REGRESSION vs Z152 which ran
    # 481s). Set ENABLE_LD_DEBUG=1 in the CI env to re-enable for
    # specific link-failure debugging — the diagnostic value is preserved
    # for when it's needed again.
    if os.environ.get("ENABLE_LD_DEBUG", "0") == "1":
        adb_shell(f"run-as {PACKAGE} sh -c 'echo 2 > .ld_debug' 2>/dev/null", timeout=15)
        print("  [ENABLE_LD_DEBUG=1] LD_DEBUG marker written")
    else:
        # Remove any stale marker from previous runs
        adb_shell(f"run-as {PACKAGE} sh -c 'rm -f .ld_debug' 2>/dev/null", timeout=15)
        print("  [ENABLE_LD_DEBUG=0] LD_DEBUG disabled (6-Z154 default)")
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
            # 6-Z126: LIVE kmsg snapshot — the guest init's own messages
            # are only copied to /sdcard at kr64 TEARDOWN, which never
            # runs when the guest wedges instead of exiting (run
            # 32773072503 lost ALL guest init messages that way). Pull
            # the file directly every 60 s so a wedged boot still tells
            # us exactly where init stalled / which service is looping.
            ts = int(time.time() - t0)
            rr = adb_shell(
                f"run-as {PACKAGE} cat rootfs/dev/__kmsg__ 2>/dev/null "
                f"| tail -c 200000", timeout=30)
            if rr.stdout:
                with open(f"{ART}/kmsg-live-{ts}s.txt", "w",
                          errors="replace") as f:
                    f.write(rr.stdout)
                # The last 12 lines tell us the current frontier.
                tail = "\n".join(rr.stdout.rstrip().splitlines()[-12:])
                print(f"    [kmsg@{ts}s] last lines:\n      "
                      + "\n      ".join(tail.splitlines()[-6:]))
            # Guest process census — which guest processes are alive.
            ps = adb_shell(
                "ps -A | grep -E 'twoyi|app_process|zygote|ueventd|"
                "servicemanager|vold|surfaceflinger|init '", timeout=20)
            if ps.stdout:
                with open(f"{ART}/guest-ps-{ts}s.txt", "w",
                          errors="replace") as f:
                    f.write(ps.stdout)
        time.sleep(SCREENSHOT_EVERY)

    dump_ui("09_final")
    capture_logs()

    print("\n" + "=" * 60)
    print("  GROUND TRUTH SUMMARY")
    print("=" * 60)
    print(f"  screenshot md5 distribution: {md5s}")

    # ── ARM64-A1: final launcher-state assertion ──
    # The post-Launch-Container check at step 2 catches the IMMEDIATE
    # crash, but a slower failure (app starts ptrace, runs for 30-200s,
    # then crashes mid-way) would slip past step 2 and produce a
    # false-green workflow. Re-check the final activity here: if we're
    # back on the system launcher at the end of the boot wait, the
    # container never finished booting — FAIL the workflow.
    final_activity = get_current_activity() or ""
    print(f"  Final activity: {final_activity}")
    final_activity_lc = final_activity.lower()
    is_launcher = (
        "launcher" in final_activity_lc
        or "nexuslauncher" in final_activity_lc
        or final_activity_lc.strip() == ""
    )
    if is_launcher:
        print()
        print("  ✗✗✗ FINAL STATE IS LAUNCHER — AOSP CONTAINER NEVER BOOTED")
        print(f"  The twoyi app is no longer the resumed activity after")
        print(f"  {BOOT_WAIT}s of boot wait. This is the false-green")
        print("  pattern — failing the workflow now.")
        sys.exit(1)
    print(f"  ✓ Final activity is non-launcher ({final_activity}) — container booted.")

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
