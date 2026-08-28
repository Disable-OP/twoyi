#!/usr/bin/env bash
# Dispatch one or more manifest entries by name.
# Usage: dispatch_by_name.sh <name1> [name2] [name3] ...
# Reads ~/.git-credentials for the token. Manifest must be in corpus/manifest.yaml.
set -u
REPO="Disable-OP/twoyi"
WF="ui-e2e-test-arm64.yml"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MANIFEST="${SCRIPT_DIR}/../../corpus/manifest.yaml"

TOKEN=$(sed -n 's|https://Disable-OP:\([^@]*\)@github.com|\1|p' ~/.git-credentials 2>/dev/null)
if [ -z "$TOKEN" ]; then
  # Try the PAT from the environment (used by the dev sandbox).
  TOKEN="${GITHUB_TOKEN:-}"
fi
[ -n "$TOKEN" ] || { echo "no token"; exit 1; }

# Read entries from the manifest, parse out (name, url, referer, md5) for each.
ENTRIES_JSON=$(python3 - "$MANIFEST" "$@" <<'EOF'
import sys, json, yaml
path, *names = sys.argv[1], *sys.argv[2:]
try:
    data = yaml.safe_load(open(path))
except Exception as e:
    # Naive fallback parser (no PyYAML).
    sys.exit("yaml parse failed: {}".format(e))
images = (data or {}).get("images", [])
out = []
for img in images:
    name = img.get("name", "")
    if not names or name in names:
        out.append({
            "name": name,
            "url": img.get("url", ""),
            "referer": img.get("referer", "") or "",
            "md5": img.get("md5", "") or "",
        })
print(json.dumps(out))
EOF
)

[ -n "$ENTRIES_JSON" ] || { echo "no entries matched"; exit 1; }

echo "$ENTRIES_JSON" | python3 -c "
import json, sys, subprocess, os
entries = json.load(sys.stdin)
token = sys.argv[1]
wf, repo = sys.argv[2], sys.argv[3]
boot_wait = sys.argv[4] if len(sys.argv) > 4 else '60'
ok = fail = 0
for e in entries:
    if not e['url']:
        print('  SKIP {} (no url)'.format(e['name']))
        continue
    inputs = {'recovery_name': e['name'], 'recovery_url': e['url'],
              'boot_wait_seconds': boot_wait}
    if e['referer']:
        inputs['recovery_referer'] = e['referer']
    if e['md5']:
        inputs['recovery_md5'] = e['md5']
    payload = json.dumps({'ref': 'main', 'inputs': inputs})
    r = subprocess.run(['curl', '-s', '-o', '/tmp/disp.txt', '-w',
                        '%{http_code}', '-X', 'POST',
                        '-H', 'Accept: application/vnd.github+json',
                        '-H', 'Authorization: token ' + token,
                        'https://api.github.com/repos/{}/actions/workflows/{}/dispatches'.format(repo, wf),
                        '-d', payload], capture_output=True, text=True)
    code = r.stdout.strip()
    if code == '204':
        print('  OK   {}'.format(e['name'])); ok += 1
    else:
        msg = open('/tmp/disp.txt').read()[:120]
        print('  FAIL {} (HTTP {}): {}'.format(e['name'], code, msg)); fail += 1
print('dispatched={} failed={}'.format(ok, fail))
" "$TOKEN" "$WF" "$REPO" "60"
