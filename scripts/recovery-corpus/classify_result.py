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
# 6-Z220: with the recovery service's `stdio_to_kmsg` option (emitted when
# the guest init supports it), the recovery process's stderr — hook
# diagnostics, linker errors, glog fatals — lands in /dev/__kmsg__ and is
# captured here. It is the ONLY place those diagnostics are visible for
# AOSP-layout services (init redirects non-console service stdio to
# /dev/null, so the tracer DIAG capture never sees them).
kmsg = read("kmsg-stub.txt", 8 * 1024 * 1024)
kr_and_kmsg = kr + "\n" + kmsg

# ── recovery instances + family banner ───────────────────────────────
# 6-Z224: TWO banner layouts, matching the real recovery print formats:
#   * TWRP/OrangeFox (legacy): "Starting TWRP <version>" newline
#     "(pid <n>)" — the pid line is separate.
#   * AOSP/lineage (minui-era, run 33279361259): "Starting recovery
#     (pid 1) on <date>" — the pid is on the SAME line. The old
#     same-line-only regex missed it and counted recovery_instances=0,
#     which misjudged a fully-booted recovery as BOOT_FAIL.
# 6-Z224b: OrangeFox R12's banner spells the pid WITHOUT a paren
#     ("...; pid 1)" — run 33282961791) — match that variant too.
banners = re.findall(
    r"Starting (TWRP|OrangeFox|SkyHawk|SHRP|recovery)[^\n]*\n\s*\(pid (\d+)\)",
    rec)
banners += [
    (fam, pid) for fam, pid in re.findall(
        r"Starting (TWRP|OrangeFox|SkyHawk|SHRP|recovery)[^\n]*\(pid (\d+)\)[^\n]*",
        rec)
    if (fam, pid) not in banners
]
banners += [
    (fam, pid) for fam, pid in re.findall(
        r"Starting (TWRP|OrangeFox|SkyHawk|SHRP|recovery)[^\n]*;\s*pid (\d+)\)[^\n]*",
        rec)
    if (fam, pid) not in banners
]
result["recovery_instances"] = len({pid for _, pid in banners})
if banners:
    result["markers"]["family_banner"] = sorted({f for f, _ in banners})

# ── boot: did the guest recovery process start at all? ──────────────
# 6-Z225: OrangeFox R12 prints "Welcome to OrangeFox Recovery!" +
# "Switching packages (OrangeFox)" instead of the TWRP-style
# "Starting ... (pid N)" banner (run 33284467693) — count those as a
# reached boot, with the family marker for the result record.
if banners or "Starting the UI" in rec:
    result["boot"] = "BOOT_OK"
elif "Welcome to OrangeFox Recovery!" in rec or "Switching packages (OrangeFox)" in rec:
    result["boot"] = "BOOT_OK"
    result["markers"]["family_banner"] = sorted(
        set(result["markers"].get("family_banner", [])) | {"OrangeFox"})
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
    # 6-Z219: Android 12+ libfstab aborts a slotselect fstab when the
    # slot suffix is empty (A/B recovery images on a non-A/B cmdline);
    # the recovery binary then glog-CHECKs on the empty fstab.
    ("Error updating for slotselect", "slotselect_fstab"),
    ("Check failed: !fstab\\.empty\\(\\)", "fstab_empty"),
]:
    if re.search(sig, kr_and_kmsg):
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

# ── 6-Z224b: AOSP-layout (minui-era) UI liveness ────────────────────
# Lineage-22.2-class recoveries have NO TWRP theme/page system — the
# legacy "Set page:" detector can never fire for them (run
# 33282962805: fully-alive UI thread, 495s, brightness dim cycle, but
# classified SPLASH_HANG). TWO hard, non-forgeable liveness signals:
#   (1) "framebuffer: <fd> (<W> x <H>)" — printed by AOSP minui's
#       fbdev Init ONLY after open+FSCREENINFO+VSCREENINFO+mmap all
#       succeeded;
#   (2) the UI THREAD's backlight dim cycle — "Brightness: <n> (%)"
#       stepping DOWN over time (127 -> 63 -> 0) — runs only while the
#       menu loop is actually alive (run 33282961791 proves the need
#       for (2): OrangeFox died at the ashmem abort AFTER printing
#       "framebuffer: 4 (720 x 1600)" but its log has ZERO dim steps;
#       the alive lineage run has 2). Require >= 2 distinct brightness
#       values (initial + at least one dim step).
fb_lines = re.findall(r"framebuffer: (\d+) \((\d+) x (\d+)\)", rec)
if fb_lines:
    result["markers"]["aosp_minui_fb"] = fb_lines[-1]

# ── presentation: did guest pixels actually reach the SurfaceView? ──
# The UI/page markers above prove the GUEST is alive internally; they do
# NOT prove the app is PRESENTING its framebuffer. The fb0 blit loop
# (core.rs twrp_fb_render_loop) logs one TWOYI_DIAG line per milestone:
#   "TWRP-FB first frame blitted to surface (...)"
#   "TWRP-FB frame #<n> blitted digest=<fnv1a> center=[r,g,b,a]"
# (the digest line is rate-capped to every ~20th changed frame).
# >= 2 DISTINCT digests with a rising frame counter = live, CHANGING
# guest frames reached the screen (page transitions included) — the
# acceptance signal for the OrangeFox display fix (run 33317227548:
# guest UI alive internally but ZERO blit lines, screen stayed on the
# loading texture forever).
# Evidence source: the docker-exec logcat window often MISSES the app
# process's own tags (the capture runs before/around the boot window),
# so ALSO scan the FileLogger archives (app-logs/log/logcat.log*) that
# the E2E pulls off /sdcard at teardown — verify run 33322760296: the
# "Recovery (loader path) flag: true" + blit lines were ONLY in the
# archives, the classifier then falsely read display_mode=gl.
app_logcat = ""
for _p in sorted(glob.glob(os.path.join(ART, "app-logs", "log", "logcat.log*"))):
    app_logcat += read(os.path.relpath(_p, ART), 32 * 1024 * 1024)
presentation_logcat = logcat + "\n" + app_logcat
blit_frames = re.findall(
    r"TWOYI_DIAG.*?TWRP-FB frame #(\d+) blitted digest=([0-9a-f]+)",
    presentation_logcat)
if blit_frames:
    last_n = max(int(n) for n, _ in blit_frames)
    digests = {d for _, d in blit_frames}
    result["markers"]["blit_frames_last"] = last_n
    result["markers"]["blit_digest_count"] = len(digests)
    if last_n >= 2 and len(digests) >= 2:
        result["markers"]["presentation"] = "FLOWING"
    elif last_n >= 1:
        result["markers"]["presentation"] = "SINGLE_FRAME"
    else:
        result["markers"]["presentation"] = "NONE"
else:
    result["markers"]["presentation"] = "NONE"
# Which display mode did the app select? (Render2Activity logs both.)
if re.search(r"Recovery \(loader path\) flag: true", presentation_logcat):
    result["markers"]["display_mode"] = "recovery_loader"
elif re.search(r"Boot Recovery \(TWRP\) flag: true", presentation_logcat):
    result["markers"]["display_mode"] = "twrp"
else:
    result["markers"]["display_mode"] = "gl"
brightness_vals = set(re.findall(r"Brightness: (\d+) \(", rec))
result["markers"]["brightness_steps"] = sorted(brightness_vals)
aosp_ui_live = bool(fb_lines) \
    and len(brightness_vals) >= 2 \
    and len({pid for _, pid in banners}) <= 1 \
    and "Rebooting..." not in rec.split("framebuffer:")[-1]
if any(p in menu_pages for p in pages):
    result["ui"] = "UI_READY"
elif aosp_ui_live:
    result["ui"] = "UI_READY"
    result["markers"]["ui_source"] = "aosp_minui"
elif "system_readonly" in rec and "Set page:" not in rec:
    result["ui"] = "UI_HANG"
elif banners:
    result["ui"] = "SPLASH_HANG"
else:
    result["ui"] = "NOT_REACHED"

# ── terminal ─────────────────────────────────────────────────────────
# 6-Z201: do NOT gate on one implementation-specific tracer string.
# 6-Z217c: after the 6-Z188 socketpair-pty switch the tracer log does
# NOT contain "pty" at all in WORKING runs (the fb hook's fd=2 pty
# diagnostics go to the TERMINAL SCREEN — the child's stdio is the pty
# slave — not to the app stderr capture; confirmed on run 33267265616
# where the screenshot shows a live ash prompt while every legacy
# signal was absent). The signal list therefore gains the §13/§20
# ground-truth form: a reached terminal page + UI_READY + the scripted
# term-07 screenshots is a live-shell verdict, because the nav script
# only scripts those screenshots when the terminal page actually
# opened, and the whyred frozen-splash false positive (33189885036)
# had ui != UI_READY so the UI_READY gate holds.
terminal_signals = [
    "pty master" in kr,
    "pty" in kr and ("socketpair" in kr or "slave" in kr),
    "can't access tty" in rec,
    "job control turned off" in rec,
]
term07_shots = [
    p for p in glob.glob(os.path.join(ART, "screenshot-*"))
    if "term-07" in os.path.basename(p)
]
if "terminalcommand" in rec or "Set page: 'terminal" in rec:
    result["terminal"] = "OK" if any(terminal_signals) else "FAIL"
    # 6-Z217c: the screenshot ground truth OVERRIDES the stale-signal
    # FAIL (master prompt §20: clear UI evidence beats a brittle
    # textual heuristic).
    if result["terminal"] == "FAIL" and result["ui"] == "UI_READY" and term07_shots:
        result["terminal"] = "OK"
        result["markers"]["terminal_evidence"] = "term-07 screenshots (6-Z217c)"
elif result["ui"] == "UI_READY" and term07_shots:
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
# 6-Z216 classifier note: a single crash produces EIGHT raw "SIGSEGV"
# grep-matches (details line + 2 regs + comm + threads + stack window +
# event lines). Count the DETAILS lines — one per real crash.
for sig in ("SIGSEGV", "SIGILL", "SIGBUS"):
    n = len(re.findall(rf"{sig} details:", kr))
    if n:
        result["markers"][f"{sig.lower()}_count"] = n
    elif kr.count(sig):
        result["markers"][f"{sig.lower()}_count"] = kr.count(sig)
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
