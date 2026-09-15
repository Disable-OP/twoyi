#!/system/bin/sh
# scripts/twoyi-cpu-policy.sh — keep the HOST responsive while the twoyi
# guest fleet hogs CPU.
#
# 6-Z365 (rn318 decode): the guest fleet (io.twoyi.debug + libkr64 + the
# ptrace'd guest init/zygote/SF fleet) were the TOP CPU CONSUMERS of the
# whole system — the HOST's own AMS watchdog dump named them one by one
# ("Skipping next CPU consuming process, not a java proc: 2027 2647 27xx…").
# HOST system_server starved → Watchdog WAITED_HALF 07:10:22 → killed at
# ~07:10:52 → runtime restart (new zygote 3482/3484, system_server 3534)
# → the restart SIGKILLed every app process → the ENTIRE guest subtree
# (app 2027 → libkr64 2647 → guest init 2679 → guest zygote64 2864 →
# fleet) died at ~07:11:2x, ~111s into the guest boot.
#
# This is HOST TESTBENCH policy, NOT guest/ROM logic — same class as the
# 6-Z305t-71 phantom-killer neutralization. It renices the whole
# io.twoyi.debug subtree (BFS over ps PPID chains) to +15 so the HOST's
# watchdog checkers keep their scheduling slice while the guest fleet
# still gets all idle CPU. Called in a loop from the UI-e2e workflow;
# every invocation prints ONE line so the loop log stays bounded.
#
# NOTE: runs INSIDE the redroid container as root (docker exec), so
# renice carries CAP_SYS_NICE even though the app runs as u0_a87.

app=$(pidof io.twoyi.debug 2>/dev/null || true)
if [ -z "$app" ]; then
    echo "cpu-policy: app not running"
    exit 0
fi

# BFS the descendant closure of the app over the ps PPID graph.
# Toybox ps/awk: header line skipped via NR>1; ~200 processes max, so the
# O(n^2) closure is fine at 10s cadence.
PIDS=$(ps -A -o PID,PPID 2>/dev/null | awk -v app="$app" '
    NR > 1 { pp[$1] = $2 }
    END {
        q[0] = app; n = 1; seen[app] = 1
        for (i = 0; i < n; i++) {
            p = q[i]
            for (c in pp) if (pp[c] == p && !(c in seen)) { seen[c] = 1; q[n++] = c }
        }
        out = ""
        for (c in seen) out = out " " c
        print out
    }')

if [ -z "$PIDS" ]; then
    echo "cpu-policy: BFS produced no pids for app=$app"
    exit 0
fi

if command -v renice >/dev/null 2>&1; then
    if renice -n 15 -p $PIDS >/dev/null 2>&1; then
        echo "cpu-policy: renice +15 ok on $(echo $PIDS | wc -w) pids (app=$app)"
    else
        echo "cpu-policy: renice FAILED on app=$app"
    fi
else
    echo "cpu-policy: renice not available in container"
fi
