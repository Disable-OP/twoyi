#!/bin/bash
# deadline_check.sh
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
