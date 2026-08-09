#!/usr/bin/env bash
set -euo pipefail

now="$(date -u +%s)"

today_target="$(date -u -d 'today 07:15:00' +%s)"

if (( now < today_target )); then
    target="$today_target"
else
    target="$(date -u -d 'tomorrow 07:15:00' +%s)"
fi

if (( now < target )); then
    printf 'true\n'
else
    printf 'false\n'
fi
