#!/bin/bash
# deadline_check.sh
#
# Usage: ./deadline_check.sh
# Prints ONLY "true" or "false" — nothing else, no time remaining, no
# deadline timestamp. This is deliberate: knowing how much time is left
# invites "I'm running low, let me wrap up" reasoning, which is exactly
# the early-stopping behavior this script exists to prevent. All it
# tells you is whether to continue right now.
#
# Exit code 0 + prints "true"  = continue.
# Exit code 1 + prints "false" = STOP immediately.
#
# Deadline: the NEXT 07:30 UTC from whenever this is first run. No date
# is hardcoded — if today's 07:30 UTC has already passed, it rolls to
# tomorrow's 07:30 automatically. Correct no matter when the session
# starts, and stays correct if it runs past midnight.

NOW_EPOCH=$(date -u +%s)
TODAY_DEADLINE=$(date -u -d "today 07:30:00" +%s 2>/dev/null || date -u -j -f "%Y-%m-%d %H:%M:%S" "$(date -u +%Y-%m-%d) 07:30:00" +%s)

if [ "$NOW_EPOCH" -lt "$TODAY_DEADLINE" ]; then
    DEADLINE_EPOCH=$TODAY_DEADLINE
else
    DEADLINE_EPOCH=$(date -u -d "tomorrow 07:30:00" +%s 2>/dev/null || date -u -j -v+1d -f "%Y-%m-%d %H:%M:%S" "$(date -u +%Y-%m-%d) 07:30:00" +%s)
fi

if [ "$NOW_EPOCH" -lt "$DEADLINE_EPOCH" ]; then
    echo "true"
    exit 0
else
    echo "false"
    exit 1
fi
