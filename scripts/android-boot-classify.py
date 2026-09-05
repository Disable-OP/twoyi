#!/usr/bin/env python3
"""scripts/android-boot-classify.py — Android boot-ladder classifier.

Mission 6-Z305: the primary objective is BOOTING A REAL ARM64 ANDROID
SYSTEM inside twoyi (pure unmodified stock Android 11 first, GSIs next).
"init started" is NOT "Android booted" — this classifier maps the raw
run evidence onto the mission's boot milestone ladder and reports the
HIGHEST rung reached, with the observed blockers at that rung.

Ladder (each rung implies all below it):
  0 IMPORT_FAIL      — rootfs never materialized
  1 ROOTFS_READY     — rootfs/init exists in the app profile
  2 INIT_STARTED     — kr64 exec'd the guest init (trace evidence)
  3 PROPERTY_SERVICE — property area created / prop sets observed
  4 CORE_DAEMONS     — logd / servicemanager / vold / keystore2 spawn
  5 ZYGOTE           — zygote64 spawned
  6 SYSTEM_SERVER    — system_server spawned
  7 SURFACEFLINGER   — surfaceflinger spawned
  8 SYSTEMUI_LAUNCHER— SystemUI / launcher process up
  9 BOOT_COMPLETED   — REAL sys.boot_completed=1 bridge fired

Evidence: the artifacts directory produced by the E2E workflow
(kr64-app-stderr log, docker-exec ps, logcat, property-area state,
rootfs listings, screenshots). Every verdict cites its source line.

Usage:
  python3 scripts/android-boot-classify.py <artifacts_dir> <out_result_json>
"""
import json
import os
import re
import sys

LADDER = [
    (1, "ROOTFS_READY", "rootfs/init materialized"),
    (2, "INIT_STARTED", "kr64 exec'd guest init"),
    (3, "PROPERTY_SERVICE", "property service up"),
    (4, "CORE_DAEMONS", "core daemons spawned"),
    (5, "ZYGOTE", "zygote spawned"),
    (6, "SYSTEM_SERVER", "system_server spawned"),
    (7, "SURFACEFLINGER", "surfaceflinger spawned"),
    (8, "SYSTEMUI_LAUNCHER", "SystemUI/launcher up"),
    (9, "BOOT_COMPLETED", "real sys.boot_completed=1"),
]


def read(path, limit_mb=24):
    try:
        with open(path, "r", errors="replace") as f:
            return f.read(limit_mb * 1024 * 1024)
    except OSError:
        return ""


def main(art, out_path):
    evidence = {}

    kr = read(os.path.join(art, "kr64-app-stderr-dockerexec.log"))
    kr_kr = read(os.path.join(art, "kr64-dockerexec.log"))
    kr_all = kr + "\n" + kr_kr
    ps = read(os.path.join(art, "ps-dockerexec.txt"))
    logcat = read(os.path.join(art, "logcat-dockerexec.txt"), 48)
    props = read(os.path.join(art, "property-area-state.txt"))
    sockets = read(os.path.join(art, "rootfs-socket-dir.txt"))

    def hit(name, pattern, text, source):
        m = re.search(pattern, text, re.I)
        if m:
            line = text[:m.end()].strip().splitlines()[-1][:200]
            evidence.setdefault(name, []).append({"source": source, "line": line})
        return bool(m)

    rung = 0
    stage = "IMPORT_FAIL"

    # 1 ROOTFS_READY
    if hit("ROOTFS_READY", r"rootfs/init", ps, "ps") or \
       hit("ROOTFS_READY", r"/rootfs/dev/__properties__", props, "property-area"):
        rung, stage = 1, "ROOTFS_READY"
    # 2 INIT_STARTED — kr64 exec evidence or init in the process list
    if hit("INIT_STARTED", r"exec(?:ve)? .{0,40}/init\b", kr_all, "kr64") or \
       hit("INIT_STARTED", r"\binit\b.*\bexec\b|exec.*\b(/)?init\b", kr, "kr64") or \
       hit("INIT_STARTED", r"^ *(?:\d+ +\S+ +\S+ +\S+.*|)init$", ps, "ps"):
        rung, stage = 2, "INIT_STARTED"
    # 3 PROPERTY_SERVICE
    if hit("PROPERTY_SERVICE", r"__properties__|prop_msg|property.*set|sys\.\w+ =", kr_all, "kr64") or \
       hit("PROPERTY_SERVICE", r"property_info +\d+|properties_serial +\d+", props, "property-area"):
        rung, stage = 3, "PROPERTY_SERVICE"
    # 4 CORE_DAEMONS
    if hit("CORE_DAEMONS", r"\bservicemanager\b", ps, "ps") or \
       hit("CORE_DAEMONS", r"\bvold\b|\bkeystore2\b|\blogd\b", ps, "ps"):
        rung, stage = 4, "CORE_DAEMONS"
    # 5 ZYGOTE
    if hit("ZYGOTE", r"zygote", ps, "ps"):
        rung, stage = 5, "ZYGOTE"
    # 6 SYSTEM_SERVER
    if hit("SYSTEM_SERVER", r"system_server", ps, "ps"):
        rung, stage = 6, "SYSTEM_SERVER"
    # 7 SURFACEFLINGER
    if hit("SURFACEFLINGER", r"surfaceflinger", ps, "ps"):
        rung, stage = 7, "SURFACEFLINGER"
    # 8 SYSTEMUI_LAUNCHER
    if hit("SYSTEMUI_LAUNCHER", r"com\.android\.systemui", ps, "ps") or \
       hit("SYSTEMUI_LAUNCHER", r"launcher|com\.android\.launcher", ps, "ps"):
        rung, stage = 8, "SYSTEMUI_LAUNCHER"
    # 9 BOOT_COMPLETED — the ONLY honest source: the kr64 bridge line
    if hit("BOOT_COMPLETED", r"BOOT_COMPLETED sent to @", kr_all, "kr64"):
        rung, stage = 9, "BOOT_COMPLETED"

    # blocker forensics: first error-class lines from the kr64 trace
    blockers = []
    for pat, label in [
        (r"(?:SIGSEGV|SIGABRT|SIGBUS|Fatal signal)", "signal"),
        (r"(?:abort|Aborted)", "abort"),
        (r"(?:EACCES|EPERM|permission denied)", "perm"),
        (r"(?:ENOENT|No such file)", "missing-path"),
        (r"(?:cannot |unable to |failed to )", "failure-msg"),
        (r"(?:mount.*failed|umount.*failed)", "mount"),
    ]:
        for m in list(re.finditer(pat, kr_all, re.I))[:6]:
            line = kr_all[max(0, m.start() - 120):m.end() + 160].strip().splitlines()
            snippet = (line[-1] if line else "")[:240]
            blockers.append({"kind": label, "line": snippet})

    # process census of the guest (evidence of who came up)
    guest_procs = sorted(set(re.findall(
        r"^(?:[a-z]|\d).{0,120}?\b(init|servicemanager|hwservicemanager|vold|keystore2|zygote64?|app_process64|system_server|surfaceflinger|logd|installd|netd|cameraserver|audioserver)\b",
        ps, re.I | re.M)))

    result = {
        "suite": "android-boot-ladder",
        "rom": "pure-stock-android11-aosp-arm64 (RSR1.210722.013.A4)",
        "rung": rung,
        "stage": stage,
        "stage_meaning": next((m for r, s, m in LADDER if r == rung),
                              "rootfs never materialized"),
        "ladder": [{"rung": r, "stage": s, "meaning": m} for r, s, m in LADDER],
        "evidence": evidence,
        "guest_processes": guest_procs,
        "blockers_sample": blockers[:20],
        "honest_note": "BOOT_COMPLETED rung fires ONLY on the kr64 bridge for a "
                       "REAL sys.boot_completed=1 property write observed on the "
                       "emulated property wire — never synthesized.",
    }
    with open(out_path, "w") as f:
        json.dump(result, f, indent=2)
    print(json.dumps({"rung": rung, "stage": stage}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2]))
