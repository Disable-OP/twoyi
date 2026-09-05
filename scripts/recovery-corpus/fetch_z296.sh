#!/usr/bin/env bash
# 6-Z296: fetch a completed ui-e2e-test-arm64 run's artifact and extract
# the input-read caller probe evidence (6-Z296 lines) plus the standard
# verdict markers, formatted for fast reading.
#
# Usage: fetch_z296.sh <run_id> [outdir]
# Requires: GITHUB_TOKEN (or /home/z/.twoyi-creds/token)
set -u
RUN="${1:?usage: fetch_z296.sh <run_id> [outdir]}"
OUT="${2:-/tmp/z296-${RUN}}"
REPO="Disable-OP/twoyi"
TOKEN="${GITHUB_TOKEN:-}"
[ -n "$TOKEN" ] || TOKEN="$(cat /home/z/.twoyi-creds/token 2>/dev/null)"
[ -n "$TOKEN" ] || { echo "no GITHUB_TOKEN"; exit 1; }

mkdir -p "$OUT"
# 1. artifacts list
curl -s -H "Accept: application/vnd.github+json" -H "Authorization: Bearer $TOKEN" \
  "https://api.github.com/repos/${REPO}/actions/runs/${RUN}/artifacts" > "$OUT/arts.json"
python3 - "$OUT/arts.json" << 'EOF' > "$OUT/art_urls.txt"
import json, sys
d = json.load(open(sys.argv[1]))
for a in d.get("artifacts", []):
    print(a["id"], a["name"], a["archive_download_url"], a["expired"])
EOF
echo "artifacts:"; cat "$OUT/art_urls.txt" | sed 's| https://[^ ]*||'
# 2. download each artifact zip
while read -r id name url expired; do
  [ "$expired" = "False" ] || continue
  curl -sL -H "Authorization: Bearer $TOKEN" -o "$OUT/${name}.zip" "$url"
  mkdir -p "$OUT/${name}"
  unzip -o -q "$OUT/${name}.zip" -d "$OUT/${name}" 2>/dev/null || true
done < "$OUT/art_urls.txt"
# 3. extract the probe evidence
echo "=== 6-Z296 probe evidence ==="
grep -rh "6-Z296" "$OUT" --include="*.log" --include="*.txt" 2>/dev/null | head -400
echo
echo "=== verdict markers ==="
grep -rh -E "BOOT_OK|BOOT_FAIL|UI_READY|SPLASH_HANG|frame_delta|interactive|UI=" "$OUT" \
  --include="result.json" --include="*.log" 2>/dev/null | tail -30
