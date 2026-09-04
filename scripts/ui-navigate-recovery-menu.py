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

TAG = chr(91) + "menu-probe" + chr(93)
ART = "/tmp/ui-e2e-artifacts"
PROBE_JSON = os.path.join(ART, "menu-probe.json")


def guest_recovery_log():
    """Best-effort GUEST recovery log (the classifier's `rec` source — the
    minui 'framebuffer: <fd> (<W> x <H>)' marker lives HERE, not in host
    logcat: the guest's stderr goes to /dev/__kmsg__, not the logd
    buffer; run 33911283769's probe skipped itself for exactly this)."""
    for root in ("/data/user/0/io.twoyi.debug", "/data/data/io.twoyi.debug"):
        try:
            r = subprocess.run(
                ["sudo", "docker", "exec", "redroid", "sh", "-c",
                 f"tail -n 4000 {root}/rootfs/tmp/recovery.log"],
                capture_output=True, text=True, timeout=20)
            out = (r.stdout or "")
            if len(out) > 200:
                return out
        except Exception:
            continue
    return ""


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


def is_aosp_menu_candidate(recovery_log_text, logcat_text):
    """AOSP-layout recovery: minui fbdev marker present in the GUEST
    recovery log, no TWRP pages anywhere."""
    both = recovery_log_text + "\n" + logcat_text
    has_minui_fb = bool(re.search(r"framebuffer: \d+ \(\d+ x \d+\)", both))
    has_twrp_pages = bool(re.findall(r"Set page: '([^']+)'", both))
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



def adb_docker_out(cmd, timeout=25):
    """Run a shell command as CONTAINER ROOT via docker exec (adb shell
    is the shell user — its /proc reads of app processes return nothing;
    the workflow's keystore2-threads dump uses exactly this channel)."""
    try:
        r = subprocess.run(
            ["sudo", "docker", "exec", "redroid", "sh", "-c", cmd],
            capture_output=True, text=True, timeout=timeout)
        return ((r.stdout or "") + (r.stderr or "")).strip()
    except Exception as e:
        return f"<threw {e!r}>"


def adb_out(cmd, timeout=20):
    """Run a shell command via adb and return its combined output (the
    probe needs RAW results — `input tap` failures print usage/error text
    that the effect check alone would silently bury)."""
    try:
        r = subprocess.run(nav.ADB + ["shell", cmd],
                           capture_output=True, text=True, timeout=timeout)
        return ((r.stdout or "") + (r.stderr or "")).strip()
    except Exception as e:
        return f"<threw {e!r}>"

INPUT_READS_RE = re.compile(r"INPUT read\(fd=\d+\) -> \d+ bytes")


def count_input_reads():
    """Number of 'INPUT read' deliveries in the GUEST recovery log — the
    ground truth that a tap's events reached the guest's EventHub
    (run 33912338135: the probe's docker `input tap` channel silently
    no-oped late-run — ZERO new reads after +22s while ui-navigate's
    early taps had produced 7)."""
    grec = guest_recovery_log()
    return len(INPUT_READS_RE.findall(grec))


def tap_with_effect(x, y):
    """Tap via an EFFECT-VERIFIED channel ladder. After each attempt wait
    and re-count the guest's INPUT-read deliveries; a delivery is the
    proof the events crossed the bridge. Returns the channel name that
    worked (or None if no channel produced events)."""
    channels = [
        ("adb-input", lambda: adb_out(f"input tap {x} {y}")),
        ("broadcast", lambda: nav._broadcast_cmd(f"input tap {x} {y}")),
        ("sendevent", lambda: nav._sendevent_cmd(f"input tap {x} {y}")),
        ("ladder", lambda: nav.input_cmd(f"input tap {x} {y}")),
    ]
    for name, fn in channels:
        before = count_input_reads()
        try:
            out = fn()
        except Exception as e:
            print(TAG, "tap channel", name, "threw", repr(e))
            continue
        if out:
            print(TAG, "tap channel", name, "raw out:", repr(str(out)[:200]))
        for _ in range(6):  # up to ~3 s for the events to cross
            time.sleep(0.5)
            if count_input_reads() > before:
                print(TAG, "tap via", name, "- events REACHED the guest")
                return name
        print(TAG, "tap via", name, "- no guest events, next channel")
    return None


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
    grec = guest_recovery_log()
    if not is_aosp_menu_candidate(grec, lc):
        probe["reason"] = (
            "not an AOSP-layout recovery (minui fb marker missing or TWRP "
            "pages present) — probe skipped, ui-navigate owns TWRP runs")
        with open(PROBE_JSON, "w") as f:
            json.dump(probe, f, indent=2)
        print(TAG, "skip:", probe["reason"])
        return

    W, H = screen_size()
    probe["ran"] = True
    probe["screen"] = [W, H]
    print(TAG, f"AOSP menu candidate — screen {W}x{H}")

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

    # 6-Z289d: capture the INPUT/FOCUS ground truth for this exact moment.
    # Run 33914083301: adb was ALIVE at probe time (screenshots rode it),
    # system_server alive at teardown, yet adb `input tap`, am broadcast
    # and sendevent ALL failed to produce a single guest INPUT read.
    # These dumps name the focused window, the InputDispatcher's view of
    # the touch, and the resumed activity — no more guessing.
    for label, cmd in (
        ("focused-window", "dumpsys window windows | grep -E "
         "'mCurrentFocus|mFocusedWindow|mAwakened|mHasSurface' | head -12"),
        ("input-dispatch", "dumpsys input | grep -A 8 "
         "'InputStreamState|FocusedWindow|dispatch' | head -40"),
        ("resumed-activity", "dumpsys activity activities | grep -E "
         "'mResumedActivity|topResumedActivity' | head -6"),
    ):
        out = adb_out(cmd, timeout=25)
        path = os.path.join(ART, f"menu-probe-{label}.txt")
        try:
            with open(path, "w") as f:
                f.write(out or "<empty>")
        except OSError:
            pass
    print(TAG, label + ":", (out or "<empty>")[:200])

    # Safe tap ladder with REAL menu geometry, measured from the
    # lineage-22.2-sailfish final screenshot (720x1600): rows are
    #   Reboot system now  y = 555-690  (pre-highlighted — NEVER tapped)
    #   Apply update       y = 700-840  (center ~770 = 0.481H)
    #   Factory reset      y = 850-990  (center ~920 = 0.575H)
    #   Advanced           y = 1000-1140 (center ~1070 = 0.669H)
    # Tapping a NON-highlighted row moves the selection (a redraw);
    # tapping the highlighted row would ACTIVATE it — the destructive
    # case this probe must never hit.
    cx = W // 2
    ladder = [int(H * 0.481), int(H * 0.575), int(H * 0.481)]
    channels_used = []
    for i, y in enumerate(ladder):
        ch = tap_with_effect(cx, y)
        probe["taps"].append([cx, y, ch])
        if ch:
            channels_used.append(ch)
        nav.screenshot(f"menu-{i+1:02d}-after-tap{i+1}")
        time.sleep(2)
    probe["channels_used"] = channels_used

    frame_after = blit_frame_counter(logcat_dump())
    probe["frame_after"] = frame_after

    # 6-Z289g/h: the host chain is PROVEN healthy by the 6-Z289e/f
    # instrumentation (receiver fires, no drops, worker write succeeds).
    # v8's dump was empty (sed escaping through shell layers) and lineage
    # turned out to be EPOLL-based (zero poll-hook lines ever). This
    # diagnostic block answers the remaining questions with ROOT access:
    #   1. guest input-thread park state (epoll_wait? dead? futex?),
    #   2. the abstract-socket QUEUE states (ss -xp: a stuck Send-Q on
    #      the app's worker fd = worker blocked; data in the guest's
    #      Recv-Q = the guest never reads),
    #   3. which app fds still hold the two hook connections.
    app_pid = adb_docker_out("pgrep -f 'io.twoyi' | head -1", timeout=10).strip()
    # 6-Z293b: the tracer truncates guest comms to 15 chars — the recovery
    # shows up as "_system_bin_rec", so a "*recovery*" glob NEVER matched
    # and v8/v9's dump came back empty. Match the truncated shape + the
    # loader shlib's threads (which carry the hook's input work).
    dump = adb_docker_out(
        "for d in /proc/[0-9]*/task/[0-9]*; do "
        "c=$(cat $d/comm 2>/dev/null) || continue; "
        "case $c in *rec*|*recovery*|*twoyi*|*loader*) "
        "pp=${d#/proc/}; pid=${pp%%/*}; "
        "echo \"pid=$pid tid=${d##*/} comm=$c wchan=$(cat $d/wchan 2>/dev/null)\"; "
        "echo \"  syscall: $(cat $d/syscall 2>/dev/null | cut -c1-80)\";; "
        "esac; done",
        timeout=35)
    sock = adb_docker_out("ss -xp 2>/dev/null | grep -iE 'twoyi|touch' | head -12", timeout=15)
    fds = ""
    if app_pid:
        fds = adb_docker_out(
            f"ls -l /proc/{app_pid}/fd 2>/dev/null | grep -c socket; "
            f"ls -l /proc/{app_pid}/fd 2>/dev/null | tail -40 | grep -E '114|129' || true",
            timeout=15)
    try:
        with open(os.path.join(ART, "menu-probe-guest-threads.txt"), "w") as f:
            f.write("== threads ==\n" + (dump or "<empty>") +
                    "\n== sockets ==\n" + (sock or "<empty>") +
                    "\n== app fds (pid " + (app_pid or "?") + ") ==\n" + (fds or "<empty>"))
    except OSError:
        pass
    print(TAG, "guest threads lines:", len((dump or "").splitlines()),
          "| sock lines:", len((sock or "").splitlines()))

    c2 = screencap_bytes()
    probe["cap_hashes"].append(
        [hashlib.sha256(c2).hexdigest()[:16] if c2 else None])
    if c0 and c2:
        probe["cap_differs_after_taps"] = (c0 != c2)

    probe["frame_delta"] = (frame_after - frame_before
                            if frame_after is not None and
                            frame_before is not None else None)
    probe["interactive"] = bool(probe["frame_delta"] and probe["frame_delta"] > 0)
    # Verdict policy: events REACHING the guest (channels_used non-empty)
    # is the touch-delivery truth; the frame rise is the menu-redraw
    # truth. Both together = a fully interactive AOSP menu.

    with open(PROBE_JSON, "w") as f:
        json.dump(probe, f, indent=2)
    print(f"  [menu-probe] frame {frame_before} -> {frame_after} "
          f"(delta {probe['frame_delta']}), interactive={probe['interactive']}")


if __name__ == "__main__":
    main()
