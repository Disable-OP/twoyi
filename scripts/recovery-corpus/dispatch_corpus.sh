#!/usr/bin/env bash
# Dispatch ui-e2e-test-arm64.yml runs for every corpus manifest entry in a
# given tier (§31/§32). The manifest is data; this tool turns entries into
# workflow_dispatch runs. Recovery images in different concurrency groups
# run in parallel automatically (the workflow groups by recovery_name).
#
# Usage: dispatch_corpus.sh <pr|nightly|all|NAME> [boot_wait_seconds]
#
# Requires: ~/.git-credentials with https://Disable-OP:<TOKEN>@github.com
#           python3 with PyYAML (falls back to a naive parser if missing)
set -u
REPO="Disable-OP/twoyi"
WF="ui-e2e-test-arm64.yml"
SELECT="${1:?usage: dispatch_corpus.sh <pr|nightly|all|NAME> [boot_wait]}"
BOOT_WAIT="${2:-60}"

TOKEN=$(sed -n 's|https://Disable-OP:\([^@]*\)@github.com|\1|p' ~/.git-credentials)
[ -n "$TOKEN" ] || { echo "no token in ~/.git-credentials"; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MANIFEST="${SCRIPT_DIR}/../../corpus/manifest.yaml"
[ -f "$MANIFEST" ] || { echo "manifest not found: $MANIFEST"; exit 1; }

# ── parse manifest entries (name, url, referer, md5, tier) ──────────────
ENTRIES=$(python3 - "$MANIFEST" "$SELECT" <<'EOF'
import sys, json
path, select = sys.argv[1], sys.argv[2]
# 6-Z221: batch selector — "batch:N:M" dispatches entries [N, M) of the
# full manifest (0-based, half-open) so a long corpus can be verified in
# controlled waves (§28 resource budget) instead of one 592-run burst.
# "batch:N" dispatches entries [N, end). Interleaves families so every
# wave exercises TWRP + OrangeFox + Lineage together (failure clustering
# needs family diversity per wave, §36).
def interleave(items):
    by_fam = {}
    order = []
    for it in items:
        fam = it.get("family", "?")
        if fam not in by_fam:
            by_fam[fam] = []
            order.append(fam)
        by_fam[fam].append(it)
    out = []
    while any(by_fam[f] for f in order):
        for f in order:
            if by_fam[f]:
                out.append(by_fam[f].pop(0))
    return out
try:
    import yaml
    data = yaml.safe_load(open(path))
    images = (data or {}).get("images", [])
    if select.startswith("batch"):
        parts = select.split(":")
        lo = int(parts[1]) if len(parts) > 1 and parts[1] != "" else 0
        hi = int(parts[2]) if len(parts) > 2 else len(images)
        sel_imgs = interleave(images[lo:hi])
        out = sel_imgs
    else:
        out = []
        for img in images:
            if select in ("all", img.get("tier")) or select == img.get("name"):
                out.append(img)
        out = [{
            "name": i.get("name", ""),
            "url": i.get("url", ""),
            "referer": i.get("referer", "") or "",
            "md5": i.get("md5", "") or "",
        } for i in out]
    if select.startswith("batch"):
        out = [{
            "name": i.get("name", ""),
            "url": i.get("url", ""),
            "referer": i.get("referer", "") or "",
            "md5": i.get("md5", "") or "",
        } for i in out]
    print(json.dumps(out))
except ImportError:
    # naive fallback: parse the flat 2-space-indented list
    out, cur = [], {}
    for line in open(path):
        s = line.rstrip()
        if s.startswith("- name:"):
            if cur.get("name"):
                out.append(cur)
            cur = {"name": s.split("- name:", 1)[1].strip()}
        elif cur is not None and s.startswith("  ") and ":" in s:
            k, v = s.strip().split(":", 1)
            cur[k.strip()] = v.strip()
    if cur.get("name"):
        out.append(cur)
    sel = []
    for img in out:
        t = img.get("tier", "")
        if select in ("all", t) or select == img.get("name"):
            sel.append({"name": img.get("name", ""),
                        "url": img.get("url", ""),
                        "referer": img.get("referer", ""),
                        "md5": img.get("md", "")})
    print(json.dumps(sel))
EOF
)

COUNT=$(echo "$ENTRIES" | python3 -c "import json,sys; print(len(json.load(sys.stdin)))")
echo "dispatching $COUNT corpus run(s) for tier/selector '$SELECT'"

echo "$ENTRIES" | python3 -c "
import json, sys, subprocess, os
entries = json.load(sys.stdin)
token = sys.argv[1]
wf, repo, boot_wait = sys.argv[2], sys.argv[3], sys.argv[4]
ok = fail = 0
for e in entries:
    if not e['url']:
        print(f\"  SKIP {e['name']} (no url)\")
        continue
    inputs = {'recovery_name': e['name'], 'recovery_url': e['url'],
              'boot_wait_seconds': boot_wait}
    if e['referer']:
        inputs['recovery_referer'] = e['referer']
    if e['md5']:
        inputs['recovery_md5'] = e['md5']
    payload = json.dumps({'ref': 'main', 'inputs': inputs})
    r = subprocess.run(['curl', '-s', '-o', '/tmp/disp-one.txt', '-w',
                        '%{http_code}', '-X', 'POST',
                        '-H', 'Accept: application/vnd.github+json',
                        '-H', f'Authorization: token {token}',
                        f'https://api.github.com/repos/{repo}/actions/workflows/{wf}/dispatches',
                        '-d', payload], capture_output=True, text=True)
    code = r.stdout.strip()
    if code == '204':
        print(f\"  OK   {e['name']}\"); ok += 1
    else:
        print(f\"  FAIL {e['name']} (HTTP {code}): {open('/tmp/disp-one.txt').read()[:120]}\")
        fail += 1
print(f'dispatched={ok} failed={fail}')
" "$TOKEN" "$WF" "$REPO" "$BOOT_WAIT"
