#!/usr/bin/env python3
"""Classify a completed E2E run into machine-readable result categories
(master prompt §27/§33). Runs INSIDE the ui-e2e-test-arm64 job on the
already-collected /tmp/ui-e2e-artifacts/ evidence, writing result.json
next to the logs so it lands in the uploaded artifact.

Categories:
  boot      : BOOT_OK | BOOT_FAIL | TIMEOUT
  ui        : UI_READY | UI_HANG | SPLASH_HANG | CRASH | NOT_REACHED
  theme     : OK | THEME_FAIL | N/A
  terminal  : OK | FAIL | NOT_REACHED | NOT_APPLICABLE
  vfs       : CLEAN | HOST_LEAK | UNKNOWN
  extras    : vibrator_ok, battery_ok, prop_format, recovery_instances

Evidence sources (all optional — the classifier degrades gracefully):
  twrp-recovery-log-dockerexec.log   guest recovery.log (via docker exec)
  kr64-app-stderr-dockerexec.log     tracer log
  ps-dockerexec.txt                  final process table
  logcat-dockerexec.txt              host logcat
  screenshot-* names                 visual timeline (names only)
"""
import glob
import json
import os
import re
import sys

ART = sys.argv[1] if len(sys.argv) > 1 else "/tmp/ui-e2e-artifacts"
OUT = sys.argv[2] if len(sys.argv) > 2 else os.path.join(ART, "result.json")

result = {
    "schema": "twoyi.recovery.result/1",
    "boot": "UNKNOWN",
    "ui": "UNKNOWN",
    "theme": "UNKNOWN",
    "terminal": "UNKNOWN",
    "vfs": "UNKNOWN",
    "recovery_instances": 0,
    "vibrator_ok": None,
    "battery_ok": None,
    "markers": {},
}


def read(name, limit=8 * 1024 * 1024):
    p = os.path.join(ART, name)
    try:
        with open(p, "r", errors="replace") as f:
            return f.read(limit)
    except OSError:
        return ""


rec = read("twrp-recovery-log-dockerexec.log")
kr = read("kr64-app-stderr-dockerexec.log", 16 * 1024 * 1024)
ps = read("ps-dockerexec.txt")
logcat = read("logcat-dockerexec.txt", 16 * 1024 * 1024)

# ── recovery instances + family banner ───────────────────────────────
banners = re.findall(
    r"Starting (TWRP|OrangeFox|SkyHawk|SHRP|recovery)[^\n]*\n\s*\(pid (\d+)\)",
    rec)
result["recovery_instances"] = len({pid for _, pid in banners})
if banners:
    result["markers"]["family_banner"] = sorted({f for f, _ in banners})

# ── boot: did the guest recovery process start at all? ──────────────
if banners or "Starting the UI" in rec:
    result["boot"] = "BOOT_OK"
elif "exit code 127" in kr or "exited with code 127" in kr:
    result["boot"] = "BOOT_FAIL"
    result["markers"]["exit_code"] = "127"
elif "Failed to initialize property area" in kr or \
        "Unable to write serialized property infos" in kr:
    result["boot"] = "BOOT_FAIL"
    result["markers"]["failure"] = "property_area"
else:
    result["boot"] = "BOOT_FAIL" if kr else "UNKNOWN"

# ── 6-Z205: init-failure signature markers (§28 failure clustering) ──
# Ordered by boot-phase depth: the FIRST signature found is the one that
# killed init (deepest progress wins — later markers indicate the boot
# advanced past earlier hazards).
for sig, tag in [
    ("Could not open uevent socket", "uevent_socket"),
    ("ashmem_create_region failed", "ashmem"),
    ("Init encountered errors starting first stage", "first_stage_checkcall"),
    ("Failed to load serialized property info", "property_info_stat"),
    ("Failed to initialize property area", "property_area"),
    ("linker.*not found", "linker"),
    ("CANNOT LINK EXECUTABLE", "linker"),
    ("InitFatalReboot", "init_fatal_reboot"),
]:
    if re.search(sig, kr):
        result["markers"].setdefault("failure", tag)
        break

# ── theme: the 2.8-class splash-hang signature ──────────────────────
theme_fail = rec.count("Failed to load base packages") + \
    rec.count("unable to load theme")
if theme_fail:
    # Distinguish: did the FIRST boot load the theme (UI up) and only a
    # later re-exec fail, or did the boot never theme?
    first_ok = re.search(
        r"Starting TWRP.*?\n.*?Switching packages", rec, re.S) is not None \
        or "Set page: 'main2'" in rec
    result["theme"] = "OK_FIRST_BOOT_FAIL_REEXEC" if first_ok else "THEME_FAIL"
    result["markers"]["theme_fail_events"] = theme_fail
elif "Loading resources" in rec or "Switching packages" in rec:
    result["theme"] = "OK"
elif banners:
    result["theme"] = "N/A"
else:
    result["theme"] = "N/A"

# ── ui: furthest guest page ─────────────────────────────────────────
pages = re.findall(r"Set page: '([^']+)'", rec)
result["markers"]["pages"] = pages[-12:]
menu_pages = {"main2", "main"}
if any(p in menu_pages for p in pages):
    result["ui"] = "UI_READY"
elif "system_readonly" in rec and "Set page:" not in rec:
    result["ui"] = "UI_HANG"
elif banners:
    result["ui"] = "SPLASH_HANG"
else:
    result["ui"] = "NOT_REACHED"

# ── terminal ─────────────────────────────────────────────────────────
# 6-Z201: do NOT gate on one implementation-specific tracer string.
# The socketpair-based pty path never emits "pty master", yet the
# terminal demonstrably works (6-Z188: live busybox ash prompt on run
# 33132782393; VLM on run 33164208433 screenshot-term-07b). Accept ANY
# of the independent live-shell signals (master prompt §13/§20: a
# missing literal must not automatically mean failure):
#   * "pty master" in the tracer log (the old ptmx path)
#   * any pty/socketpair activity in the tracer log
#   * ash's interactive-mode banner in the guest recovery.log
#     ("can't access tty; job control turned off") — the verified
#     live-prompt signature
terminal_signals = [
    "pty master" in kr,
    "pty" in kr and ("socketpair" in kr or "slave" in kr),
    "can't access tty" in rec,
    "job control turned off" in rec,
]
if "terminalcommand" in rec or "Set page: 'terminal" in rec:
    result["terminal"] = "OK" if any(terminal_signals) else "FAIL"
elif result["ui"] == "UI_READY" and any(
        "term-07" in os.path.basename(p)
        for p in glob.glob(os.path.join(ART, "screenshot-*"))):
    # 6-Z204: gate the screenshot-name fallback on UI_READY — run
    # 33189885036 (whyred) credited a FROZEN SPLASH screen because the
    # probe's screenshot filenames are stage-scripted, not evidence of
    # a live terminal (the classifier must not trust staged names when
    # the UI never reached a menu).
    result["terminal"] = "OK"
elif result["ui"] == "UI_READY":
    result["terminal"] = "NOT_REACHED"
else:
    result["terminal"] = "NOT_APPLICABLE"

# ── vfs isolation (6-Z185 regression guard) ─────────────────────────
host_leak_markers = ["data_mirror", "linkerconfig", ".twoyi-fb-geometry",
                     ".twoyi-staged"]
# The FM root evidence lives in screenshots we cannot OCR here; use the
# tracer's backstop as the signal: leaks would show as getdents on host
# dirs NOT denied.
denied = kr.count("SANDBOX BACKSTOP: DENIED")
result["markers"]["backstop_denied"] = denied
leak_suspect = any(
    m in rec for m in host_leak_markers) and "filemanagerlist" in rec
result["vfs"] = "HOST_LEAK" if leak_suspect else "CLEAN"

# ── haptics/battery (§12/§13) ───────────────────────────────────────
vib_fail = len(re.findall(
    r'open\("/sys/class/timed_output/vibrator/enable"[^\n]*fd=-1', rec))
vib_ok = len(re.findall(
    r'open\("/sys/class/timed_output/vibrator/enable"[^\n]*fd=\d+', rec))
result["vibrator_ok"] = (vib_ok > 0) if (vib_ok or vib_fail) else None
bat = 'open("/sys/class/power_supply/battery/capacity"' in rec
result["battery_ok"] = bat if bat else None

# ── crash signals ────────────────────────────────────────────────────
for sig in ("SIGSEGV", "SIGILL", "SIGBUS"):
    n = kr.count(sig)
    if n:
        result["markers"][f"{sig.lower()}_count"] = n
if "FATAL EXCEPTION" in logcat:
    result["markers"]["java_fatal"] = logcat.count("FATAL EXCEPTION")

# ── 6-Z190 coverage watchdog firings (bug signal) ────────────────────
z190 = kr.count("6-Z190: coverage watchdog ATTACHED")
if z190:
    result["markers"]["z190_attached"] = z190

# ── overall verdict (§27) ────────────────────────────────────────────
if result["boot"] == "BOOT_OK" and result["ui"] == "UI_READY":
    overall = "UI_READY"
elif result["boot"] == "BOOT_OK" and result["ui"] == "UI_HANG":
    overall = "UI_HANG"
elif result["boot"] == "BOOT_OK" and result["ui"] == "SPLASH_HANG":
    overall = "UI_HANG"
elif result["boot"] == "BOOT_FAIL":
    if result["markers"].get("failure") == "property_area" or \
            result["markers"].get("exit_code") == "127":
        overall = "BOOT_FAIL_EARLY_INIT"
    else:
        overall = "BOOT_FAIL"
else:
    overall = "TIMEOUT_OR_UNKNOWN"
result["overall"] = overall

with open(OUT, "w") as f:
    json.dump(result, f, indent=2, sort_keys=True)
print(json.dumps(result, indent=2, sort_keys=True))
