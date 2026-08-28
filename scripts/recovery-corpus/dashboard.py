#!/usr/bin/env python3
"""Twoyi Recovery Compatibility Dashboard (§35/§36).

Aggregates result.json files (produced by classify_result.py inside each
E2E run) into:
  * corpus/results/summary.json  — machine-readable totals + per-image
  * corpus/results/dashboard.md  — the human dashboard + failure
    clustering by subsystem (§36)

Sources (in priority order):
  1. --results-dir DIR     — local dir of result.json files
  2. --from-github HOURS   — pull result.json out of the recent GitHub
                             Actions artifacts of ui-e2e-test-arm64 runs
                             (needs ~/.git-credentials token)

Dedup: one result per (image name, head_sha) — the newest wins; a
re-run of the same image on the same commit replaces the old verdict.
"""
import argparse
import io
import json
import os
import re
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from collections import defaultdict
from datetime import datetime, timedelta, timezone

REPO = "Disable-OP/twoyi"


def token():
    try:
        s = open(os.path.expanduser("~/.git-credentials")).read()
        m = re.search(r"https://Disable-OP:([^@]+)@github.com", s)
        return m.group(1) if m else None
    except OSError:
        return None


def gh(path):
    t = token()
    r = subprocess.run(
        ["curl", "-s", "-m", "60", "-H", f"Authorization: token {t}",
         f"https://api.github.com/repos/{REPO}/{path}"],
        capture_output=True, text=True)
    return json.loads(r.stdout) if r.stdout else {}


def collect_from_github(hours):
    out = []
    since = datetime.now(timezone.utc) - timedelta(hours=hours)
    runs = gh(f"actions/runs?per_page=100")
    for run in runs.get("workflow_runs", []):
        if "UI E2E Test (ARM64" not in run.get("name", ""):
            continue
        created = datetime.strptime(
            run["created_at"], "%Y-%m-%dT%H:%M:%SZ").replace(
            tzinfo=timezone.utc)
        if created < since:
            continue
        if run.get("status") != "completed":
            continue
        name = None
        # The recovery under test is recoverable from the run's jobs'
        # input echo... simplest: the artifact name is always the same,
        # so tag the result with run id + head sha; the image name comes
        # from inside result.json? No — result.json has no image name
        # (the classifier never sees it). Use the run's display title /
        # inputs via the run's json (inputs are only on the dispatch).
        # Fallback: jobs API -> job name is constant; so use the
        # workflow run's `display_title` heuristic: dispatches carry the
        # recovery name in… they don't. Practical approach: match by
        # created-time against known dispatches is fragile — instead we
        # read the recovery name from the downloaded logs (the probe
        # prints it) — see below.
        arts = gh(f"actions/runs/{run['id']}/artifacts")
        for a in arts.get("artifacts", []):
            if a["name"] != "ui-e2e-arm64-logs":
                continue
            r = subprocess.run(
                ["curl", "-s", "-m", "180", "-L",
                 "-H", f"Authorization: token {token()}",
                 a["archive_download_url"], "-o", "/tmp/dash-art.zip"],
                capture_output=True, text=True)
            try:
                with zipfile.ZipFile("/tmp/dash-art.zip") as z:
                    z.extractall("/tmp/dash-art")
            except (OSError, zipfile.BadZipFile):
                continue
            tar = "/tmp/dash-art/ui-e2e-logs.tar.xz"
            if not os.path.exists(tar):
                continue
            with tempfile.TemporaryDirectory() as td:
                subprocess.run(["tar", "-xJf", tar, "-C", td],
                               capture_output=True)
                rj = os.path.join(td, "tmp/ui-e2e-artifacts/result.json")
                data = {}
                if os.path.exists(rj):
                    try:
                        data = json.load(open(rj))
                    except json.JSONDecodeError:
                        data = {}
                # Recover the image name from the job log echo (the
                # workflow prints RECOVERY_NAME) or the recovery.log
                # banner.
                rl = os.path.join(
                    td, "tmp/ui-e2e-artifacts/twrp-recovery-log-dockerexec.log")
                banner = ""
                try:
                    txt = open(rl, errors="replace").read(4096)
                    m = re.search(r"Starting (TWRP \S+|OrangeFox\S*)", txt)
                    banner = m.group(1) if m else ""
                except OSError:
                    pass
                data.setdefault("image", banner or f"run-{run['id']}")
                data["run_id"] = run["id"]
                data["head_sha"] = run["head_sha"][:12]
                data["created_at"] = run["created_at"]
                data["conclusion"] = run.get("conclusion")
                out.append(data)
            break
    return out


def collect_local(d):
    out = []
    for root, _, files in os.walk(d):
        if "result.json" in files:
            try:
                data = json.load(open(os.path.join(root, "result.json")))
                data.setdefault("image", os.path.basename(root))
                out.append(data)
            except (OSError, json.JSONDecodeError):
                pass
    return out


# §36 failure clustering
def cluster(results):
    buckets = defaultdict(list)
    for r in results:
        ov = r.get("overall", "UNKNOWN")
        if ov in ("UI_READY",):
            continue
        boot = r.get("boot", "UNKNOWN")
        theme = r.get("theme", "UNKNOWN")
        if boot == "BOOT_FAIL":
            sub = "BOOT/" + r.get("markers", {}).get("failure", "init")
        elif theme == "THEME_FAIL":
            sub = "VFS/theme"
        elif theme == "OK_FIRST_BOOT_FAIL_REEXEC":
            sub = "re-exec coverage"
        elif r.get("terminal") == "FAIL":
            sub = "TERMINAL"
        else:
            sub = ov
        buckets[sub].append(r.get("image", "?"))
    return dict(buckets)


def render(results, out_md, out_json):
    total = len(results)
    by = lambda k, v: sum(1 for r in results if r.get(k) == v)
    lines = []
    lines.append("# Twoyi Recovery Compatibility Dashboard\n")
    lines.append(f"_generated {datetime.now(timezone.utc).isoformat()}_\n")
    lines.append("## Totals\n")
    lines.append("```")
    lines.append(f"Tested:        {total}")
    lines.append(f"Booted:        {by('boot', 'BOOT_OK')}")
    lines.append(f"UI Ready:      {by('ui', 'UI_READY')}")
    lines.append(f"Terminal OK:   {by('terminal', 'OK')}")
    lines.append(f"VFS Clean:     {by('vfs', 'CLEAN')}")
    lines.append(f"Failed:        {sum(1 for r in results if r.get('overall') not in ('UI_READY',))}")
    lines.append("```\n")
    lines.append("## Per-image\n")
    lines.append("| image | overall | boot | ui | theme | terminal | run |")
    lines.append("|---|---|---|---|---|---|---|")
    for r in sorted(results, key=lambda x: x.get("image", "")):
        lines.append(
            f"| {r.get('image','?')} | {r.get('overall','?')} "
            f"| {r.get('boot','?')} | {r.get('ui','?')} "
            f"| {r.get('theme','?')} | {r.get('terminal','?')} "
            f"| {r.get('run_id','local')} |")
    cl = cluster(results)
    if cl:
        lines.append("\n## Failure clusters (§36)\n")
        for sub, imgs in sorted(cl.items(), key=lambda kv: -len(kv[1])):
            lines.append(f"* **{sub}** ({len(imgs)}): {', '.join(sorted(imgs))}")
    md = "\n".join(lines) + "\n"
    with open(out_md, "w") as f:
        f.write(md)
    with open(out_json, "w") as f:
        json.dump({"generated": datetime.now(timezone.utc).isoformat(),
                   "total": total, "results": results}, f, indent=2)
    print(md)
    print(f"wrote {out_md} + {out_json}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--results-dir")
    ap.add_argument("--from-github", type=int, metavar="HOURS")
    ap.add_argument("--out-md", default="corpus/results/dashboard.md")
    ap.add_argument("--out-json", default="corpus/results/summary.json")
    a = ap.parse_args()
    results = []
    if a.results_dir:
        results += collect_local(a.results_dir)
    if a.from_github:
        results += collect_from_github(a.from_github)
    # dedupe: newest per (image, head_sha)
    best = {}
    for r in results:
        key = (r.get("image"), r.get("head_sha", "local"))
        prev = best.get(key)
        if not prev or str(r.get("created_at", "")) > str(
                prev.get("created_at", "")):
            best[key] = r
    os.makedirs(os.path.dirname(a.out_md), exist_ok=True)
    render(list(best.values()), a.out_md, a.out_json)


if __name__ == "__main__":
    main()
