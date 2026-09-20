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


def guest_subtree_names(rows):
    """names of the io.twoyi.debug subtree for one ps snapshot (any format)."""
    procs = {pid: (ppid, name) for pid, ppid, name in rows}
    app_pid = next((pid for pid, (_ppid, name) in procs.items()
                    if name.rstrip("]").endswith("io.twoyi.debug")), None)
    if app_pid is None:
        return []
    guests = {app_pid}
    changed = True
    while changed:
        changed = False
        for pid, (ppid, _name) in procs.items():
            if ppid in guests and pid not in guests:
                guests.add(pid)
                changed = True
    return [name for pid, (_ppid, name) in sorted(procs.items())
            if pid in guests]


def parse_watch_samples(watch_text):
    """[(offset_s, ps_rows)] per '===== liveness @Ns =====' block.

    The watch sampler (6-Z367) appends filtered `ps -A -o PID,PPID,NAME`
    rows — 3-column rows, a DIFFERENT shape from the final full `ps -A`
    (parse_ps_rows expects >=8 columns and would skip every watch row).
    Rows are grep-filtered by the sampler, so a row joins the guest
    subtree only when its own parent chain (app -> libkr64 -> init ->
    zygote -> ...) was ALSO matched by the filter — the workflow keeps
    that chain complete (6-Z374 added systemui/launcher to it).
    """
    samples = []
    offset = None
    rows = []
    for line in watch_text.splitlines():
        m = re.match(r"===== liveness @(\d+)s", line)
        if m:
            if offset is not None and rows:
                samples.append((offset, rows))
            offset, rows = int(m.group(1)), []
            continue
        if offset is None:
            continue
        parts = line.split()
        if len(parts) < 3:
            continue
        try:
            pid_i, ppid_i = int(parts[0]), int(parts[1])
        except ValueError:
            continue
        rows.append((pid_i, ppid_i, " ".join(parts[2:])))
    if offset is not None and rows:
        samples.append((offset, rows))
    return samples


def guest_process_view(ps_text):
    """(guest_names_text, guest_name_list, app_pid, guest_dead)."""
    rows = parse_ps_rows(ps_text)
    procs = {pid: (ppid, name) for pid, ppid, name in rows}
    app_pid = next((pid for pid, (_ppid, name) in procs.items()
                    if name.rstrip("]").endswith("io.twoyi.debug")), None)
    if app_pid is None:
        return "", [], None, False
    names = guest_subtree_names(rows)
    dead = any("kr64" in name and name.startswith("[")
               for pid, (ppid, name) in procs.items()
               if ppid == app_pid)
    return "\n".join(names), names, app_pid, dead


def main(art, out_path):
    evidence = {}

    kr = read(os.path.join(art, "kr64-app-stderr-dockerexec.log"))
    kr_kr = read(os.path.join(art, "kr64-dockerexec.log"))
    # 6-Z367 (rn318 decode): the docker-exec push of the app stderr is a
    # `tail -n 20000` of a very chatty file — in rn318 it only covered
    # +60.6s..+111.6s, so the early-boot KLOG-TIMELINE lines
    # (servicemanager +2.4s, zygote +9.7s, surfaceflinger +24.7s) were
    # truncated OUT of the classifier's input while the run actually
    # reached rung 7 territory. The app's FileLogger copy of kr64.log is
    # pulled via adb WITHOUT a tail bound — full-history, and
    # guest-attributed by construction (it IS the tracer's own log).
    kr_fl = read(os.path.join(art, "app-logs", "log", "kr64.log"))
    kr_all = kr + "\n" + kr_kr + "\n" + kr_fl
    ps = read(os.path.join(art, "ps-dockerexec.txt"))
    props = read(os.path.join(art, "property-area-state.txt"))
    # 6-Z374 (rn327/328 verifier gap): the FINAL ps is one snapshot — if
    # SystemUI/launcher lived and died (watchdog abort) or the guest
    # advanced between the last sample and the final dump, the mid-watch
    # evidence was invisible to the classifier. The liveness watcher
    # samples the container ps every 30s; UNION the guest subtree across
    # ALL samples (each labeled with its @offset for the timeline) so a
    # rung once reached stays claimed. guest-logcat.txt is the GUEST's own
    # logd tail (staged by twoyi-logdrain.rc INSIDE the guest rootfs —
    # host redroid content cannot appear in it), so its zygote/AMS
    # "Start proc PID:pkg/uid" lines are guest-attributed by
    # construction; the line shape is verified framework format (host
    # logcat ground truth rn328: 'Start proc 668:com.android.systemui/
    # u0a64 for service {...}').
    watch = read(os.path.join(art, "docker-exec-watch.log"))
    watch_rows = []
    for offset, rows in parse_watch_samples(watch):
        for name in guest_subtree_names(rows):
            watch_rows.append(f"@{offset}s {name}")
    watch_names_text = "\n".join(watch_rows)
    guest_logcat = read(os.path.join(art, "guest-logcat.txt"))

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
    # 4..8 — GUEST SUBTREE ONLY, with 6-Z367 klog fallbacks.
    # The ps subtree disappears whenever the BENCH dies after the guest
    # made progress (rn318: host runtime restart killed the app tree),
    # and the stderr tail may have truncated the early KLOG lines. The
    # tracer's own "init: starting service 'X'" bridge lines are
    # GUEST-attributed evidence (the HOST has no such init service
    # messages inside the kr64 trace), so they are a legitimate fallback.
    # VERIFIED-PATTERNS-ONLY policy: rn318 ground truth shows the exact
    # lines for servicemanager (+2.4s), zygote (+9.7s), surfaceflinger
    # (+24.7s). system_server is forked BY zygote, not spawned by init,
    # so no klog pattern is asserted for rung 6 (ps-only until a run
    # captures the real line — no speculative patterns).
    if hit("CORE_DAEMONS", r"\bservicemanager\b|\bvold\b|\bkeystore2\b|\blogd\b", guest_names, "guest-ps") or \
       hit("CORE_DAEMONS", r"\bservicemanager\b|\bvold\b|\bkeystore2\b|\blogd\b", watch_names_text, "watch-ps") or \
       hit("CORE_DAEMONS", r"starting service '(?:servicemanager|logd|vold|keystore2|hwservicemanager)'", kr_all, "kr64-klog"):
        rung, stage = 4, "CORE_DAEMONS"
    if hit("ZYGOTE", r"zygote", guest_names, "guest-ps") or \
       hit("ZYGOTE", r"zygote", watch_names_text, "watch-ps") or \
       hit("ZYGOTE", r"starting service 'zygote'", kr_all, "kr64-klog"):
        rung, stage = 5, "ZYGOTE"
    if hit("SYSTEM_SERVER", r"system_server", guest_names, "guest-ps") or \
       hit("SYSTEM_SERVER", r"system_server", watch_names_text, "watch-ps"):
        rung, stage = 6, "SYSTEM_SERVER"
    if hit("SURFACEFLINGER", r"surfaceflinger", guest_names, "guest-ps") or \
       hit("SURFACEFLINGER", r"surfaceflinger", watch_names_text, "watch-ps") or \
       hit("SURFACEFLINGER", r"starting service 'surfaceflinger'", kr_all, "kr64-klog"):
        rung, stage = 7, "SURFACEFLINGER"
    # 6-Z494 (rn469 decode): the app processes' ps NAME column is the
    # kernel comm (prctl PR_SET_NAME), truncated to 15 chars —
    # "com.android.systemui" appears as "com.android.sys" and
    # "com.android.launcher3" as "com.android.lau"; the full-name regex
    # could never match. The truncated forms are unique in the A11 fleet
    # (settings="com.android.set", phone="com.android.pho" don't
    # collide), and the raw name rides the evidence line for the audit.
    if hit("SYSTEMUI_LAUNCHER", r"com\.android\.systemui|com\.android\.sys|launcher|com\.android\.lau", guest_names, "guest-ps") or \
       hit("SYSTEMUI_LAUNCHER", r"com\.android\.systemui|com\.android\.sys|launcher|com\.android\.lau", watch_names_text, "watch-ps") or \
       hit("SYSTEMUI_LAUNCHER", r"Start proc \d+:[^ ]*(?:systemui|launcher)", guest_logcat, "guest-logcat"):
        rung, stage = 8, "SYSTEMUI_LAUNCHER"
    # 9 BOOT_COMPLETED — the ONLY honest source: the kr64 bridge line
    if hit("BOOT_COMPLETED", r"BOOT_COMPLETED sent to @", kr_all, "kr64"):
        rung, stage = 9, "BOOT_COMPLETED"

    # 6-Z367: bench-death discriminator. rn318's final `ps -A` had NO
    # io.twoyi.debug at all (host runtime restart killed the app tree),
    # which made rungs 4-8 structurally invisible even though the guest
    # trace was alive to +111.6s. Distinguish "guest died" (init reboot /
    # zombie kr64) from "the bench around it died" (app subtree vanished
    # while the trace still shows life). The annotation NEVER lowers the
    # rung — the klog evidence stays real guest progress.
    bench_death = app_pid is None and bool(kr_all.strip())

    post_mortem = ""
    if bench_death and not guest_dead:
        post_mortem = ("bench-death: io.twoyi.debug subtree absent from final "
                       "host ps while the kr64 trace shows guest life — "
                       "host-side restart/teardown class (see "
                       "host-restart-forensics.txt); rung NOT lowered")
        stage = f"{stage} [{post_mortem}]"
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
                       "redroid processes are excluded) — final full ps UNION the "
                       "per-30s liveness-watch samples (6-Z374, @offset-labeled) — "
                       "or, rung 8 only, the guest's OWN logd 'Start proc PID:pkg' "
                       "line via twoyi-logdrain.rc (guest-attributed by "
                       "construction).",
    }
    with open(out_path, "w") as f:
        json.dump(result, f, indent=2)
    print(json.dumps({"rung": rung, "stage": stage, "post_mortem": post_mortem},
                     indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2]))
