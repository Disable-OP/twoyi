#!/usr/bin/env python3
"""scripts/android-boot-classify.py — Android boot-ladder classifier.

Mission 6-Z305: the primary objective is BOOTING A REAL ARM64 ANDROID
SYSTEM inside twoyi (pure unmodified stock Android 11 first, GSIs next).
"init started" is NOT "Android booted" — this classifier maps the raw
run evidence onto the mission's boot milestone ladder and reports the
HIGHEST rung reached, with the observed blockers at that rung.

HOST-CONTAMINATION GUARD (run 33987046990 lesson): the E2E runs inside
redroid, whose OWN Android userland has zygote64/system_server/
surfaceflinger/com.android.systemui. A naive `grep zygote` over the
container's `ps -A` classified the HOST framework as the guest and
reported a false SYSTEMUI_LAUNCHER rung 8 for a guest that had already
exited. Therefore:
  * guest processes = the process SUBTREE rooted at io.twoyi.debug;
  * init/property rungs come from the kr64 TRACE ONLY (the host also
    runs an init — ps alone can never prove the guest one);
  * a dead guest (kr64 child zombie / init reboot message) caps the
    verdict with an explicit post-mortem note.

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


def parse_ps_rows(ps_text):
    rows = []
    for line in ps_text.splitlines():
        parts = line.split()
        if len(parts) < 8:
            continue
        try:
            pid_i, ppid_i = int(parts[1]), int(parts[2])
        except ValueError:
            continue  # header line
        rows.append((pid_i, ppid_i, parts[-1]))
    return rows


def guest_process_view(ps_text):
    """(guest_names_text, guest_name_list, app_pid, guest_dead)."""
    rows = parse_ps_rows(ps_text)
    procs = {pid: (ppid, name) for pid, ppid, name in rows}
    app_pid = next((pid for pid, (_ppid, name) in procs.items()
                    if name.rstrip("]").endswith("io.twoyi.debug")), None)
    if app_pid is None:
        return "", [], None, False
    guests = {app_pid}
    changed = True
    while changed:
        changed = False
        for pid, (ppid, _name) in procs.items():
            if ppid in guests and pid not in guests:
                guests.add(pid)
                changed = True
    names = [name for pid, (_ppid, name) in sorted(procs.items())
             if pid in guests]
    dead = any("kr64" in name and name.startswith("[")
               for pid, (ppid, name) in procs.items()
               if ppid == app_pid)
    return "\n".join(names), names, app_pid, dead


def main(art, out_path):
    evidence = {}

    kr = read(os.path.join(art, "kr64-app-stderr-dockerexec.log"))
    kr_kr = read(os.path.join(art, "kr64-dockerexec.log"))
    kr_all = kr + "\n" + kr_kr
    ps = read(os.path.join(art, "ps-dockerexec.txt"))
    props = read(os.path.join(art, "property-area-state.txt"))

    guest_names, guest_name_list, app_pid, ps_dead = guest_process_view(ps)
    # guest death: init's reboot path observed in the trace (kr64 bridges
    # init's stderr through the WRITEV samples) or a zombie kr64 child.
    init_reboot = bool(re.search(r"Reboot ending, jumping to kernel", kr_all))
    zombie = bool(re.search(r"Z\s+\[?libkr64", ps))
    guest_dead = init_reboot or zombie or ps_dead

    def hit(name, pattern, text, source):
        m = re.search(pattern, text, re.I)
        if m:
            line = text[:m.end()].strip().splitlines()[-1][:200]
            evidence.setdefault(name, []).append({"source": source, "line": line})
        return bool(m)

    rung = 0
    stage = "IMPORT_FAIL"

    # 1 ROOTFS_READY — app-profile tree listing evidence
    if hit("ROOTFS_READY", r"rootfs/init", ps, "ps") or \
       hit("ROOTFS_READY", r"/rootfs/dev/__properties__", props, "property-area"):
        rung, stage = 1, "ROOTFS_READY"
    # 2 INIT_STARTED — kr64 trace ONLY (host also has an init; ps is void)
    if hit("INIT_STARTED", r"init first stage started!", kr_all, "kr64") or \
       hit("INIT_STARTED", r"exec(?:ve)?.{0,60}/init\b", kr_all, "kr64"):
        rung, stage = 2, "INIT_STARTED"
    # 3 PROPERTY_SERVICE — prop wire / property area evidence
    if hit("PROPERTY_SERVICE", r"prop_msg|property.*set|__properties__|z111_apply_property_set", kr_all, "kr64") or \
       hit("PROPERTY_SERVICE", r"property_info +\d+|properties_serial +\d+", props, "property-area"):
        rung, stage = 3, "PROPERTY_SERVICE"
    # 4..8 — GUEST SUBTREE ONLY
    if hit("CORE_DAEMONS", r"\bservicemanager\b|\bvold\b|\bkeystore2\b|\blogd\b", guest_names, "guest-ps"):
        rung, stage = 4, "CORE_DAEMONS"
    if hit("ZYGOTE", r"zygote", guest_names, "guest-ps"):
        rung, stage = 5, "ZYGOTE"
    if hit("SYSTEM_SERVER", r"system_server", guest_names, "guest-ps"):
        rung, stage = 6, "SYSTEM_SERVER"
    if hit("SURFACEFLINGER", r"surfaceflinger", guest_names, "guest-ps"):
        rung, stage = 7, "SURFACEFLINGER"
    if hit("SYSTEMUI_LAUNCHER", r"com\.android\.systemui|launcher", guest_names, "guest-ps"):
        rung, stage = 8, "SYSTEMUI_LAUNCHER"
    # 9 BOOT_COMPLETED — the ONLY honest source: the kr64 bridge line
    if hit("BOOT_COMPLETED", r"BOOT_COMPLETED sent to @", kr_all, "kr64"):
        rung, stage = 9, "BOOT_COMPLETED"

    post_mortem = ""
    if guest_dead and rung < 9:
        post_mortem = ("guest exited post-mortem: "
                       + ("init reboot path (Reboot ending, jumping to kernel)" if init_reboot else "")
                       + ("; " if init_reboot and zombie else "")
                       + ("kr64 child zombie in ps" if zombie else ""))
        stage = f"{stage} [{post_mortem}]"

    # blocker forensics: the guest's OWN stderr lines bridged by kr64
    # (WRITEV samples carry init/daemon messages) + error-class lines
    blockers = []
    seen_msgs = set()
    for m in re.finditer(r'iov0\[[^\]]*\]="([^"]{8,240})"', kr_all):
        msg = m.group(1).replace("\\n", "")
        if re.search(r"init:|selinux:|reboot|FATAL|cannot|failed|Unable", msg, re.I):
            key = msg[:80]
            if key not in seen_msgs:
                seen_msgs.add(key)
                blockers.append({"kind": "guest-stderr", "line": msg[:240]})
    for pat, label in [
        (r"(?:SIGSEGV|SIGABRT|SIGBUS|Fatal signal)", "signal"),
        (r"InitFatalReboot[^\"]{0,80}", "init-fatal"),
        (r"(?:EACCES|EPERM|permission denied)", "perm"),
        (r"(?:ENOENT|No such file)", "missing-path"),
        (r"(?:mount.*failed|umount.*failed)", "mount"),
    ]:
        for m in list(re.finditer(pat, kr_all, re.I))[:6]:
            line = kr_all[max(0, m.start() - 100):m.end() + 140].strip().splitlines()
            snippet = (line[-1] if line else "")[:240]
            blockers.append({"kind": label, "line": snippet})

    result = {
        "suite": "android-boot-ladder",
        "rom": "pure-stock-android11-aosp-arm64 (RSR1.210722.013.A4)",
        "rung": rung,
        "stage": stage,
        "stage_meaning": next((m for r, s, m in LADDER if r == rung),
                              "rootfs never materialized"),
        "ladder": [{"rung": r, "stage": s, "meaning": m} for r, s, m in LADDER],
        "evidence": evidence,
        "app_pid": app_pid,
        "guest_processes": guest_name_list,
        "post_mortem": post_mortem,
        "blockers_sample": blockers[:24],
        "honest_note": "BOOT_COMPLETED rung fires ONLY on the kr64 bridge for a "
                       "REAL sys.boot_completed=1 property write observed on the "
                       "emulated property wire — never synthesized. Guest process "
                       "rungs require the io.twoyi.debug process subtree (host "
                       "redroid processes are excluded).",
    }
    with open(out_path, "w") as f:
        json.dump(result, f, indent=2)
    print(json.dumps({"rung": rung, "stage": stage, "post_mortem": post_mortem},
                     indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2]))
