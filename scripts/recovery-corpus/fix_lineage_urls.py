#!/usr/bin/env python3
"""6-Z272b: pick the best LineageOS artifact per device.

The 2026-09-03 sweep decoded two corpus-level defects:
  1. For legacy devices (violet/a21s/…) the sweep used boot.img — a NORMAL
     boot ramdisk (11 entries: init + fstab only). Booting it as a recovery
     makes A15 init run FirstStageMount against nonexistent block devices →
     BOOT_FAIL_EARLY_INIT. Lineage publishes the real recovery ramdisk as
     recovery.img for those devices.
  2. For dynamic-partition devices (alioth/barbet/…) recovery.img does not
     exist at all — recovery lives in vendor_boot.img.

This script probes, per manifest device+date, recovery.img →
vendor_boot.img → boot.img and rewrites the manifest URL to the first
artifact that exists (HTTP 200/302). Manifest is data; no workflow edits.

6-Z334 (2026-09-14): the 2026-09-13 nightly rotation DELETED whole dated
builds (92/296 manifest URLs → 404; the lineage-22.2-sailfish pr-gate child
died on `curl: (22) 404` in run 34820533069). New fallback: when EVERY
artifact of (device, date) is gone, query the official builds API
(https://download.lineageos.org/api/v2/devices/{dev}/builds) and redate the
entry to the newest build that still ships one of the artifacts, taking the
authoritative sha256 from the API. Pinned md5/sha256 that no longer match
the rewritten artifact are cleared ("empty = computed at CI") so the
download gate stays honest instead of failing pre-boot.

Usage: fix_lineage_urls.py corpus/manifest.yaml
"""
import concurrent.futures
import json
import re
import sys
import urllib.request

MIRROR = "https://mirrorbits.lineageos.org/full/{dev}/{date}/{art}"
BUILDS_API = "https://download.lineageos.org/api/v2/devices/{dev}/builds"
ORDER = ["recovery", "vendor_boot", "boot"]
URL_RE = re.compile(
    r"^\s*url:\s*https://mirrorbits\.lineageos\.org"
    r"/full/([a-z0-9_]+)/([0-9]{8})/(recovery|vendor_boot|boot)\.img\s*$"
)
NAME_RE = re.compile(r"^  - name:")


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


_opener = urllib.request.build_opener(NoRedirect)


def exists(url: str) -> bool:
    # mirrorbits answers 302 for artifacts that exist and 404 for those that
    # don't. Do NOT follow the redirect — the CDN targets hang on HEAD.
    # 6-Z272l: mirrorbits answers HEAD with 404 for EVERYTHING (even
    # artifacts it happily serves — verified: HEAD boot.img → 404, ranged
    # GET boot.img → 302 for the same URL the sweep downloaded). The probe
    # therefore uses a no-redirect GET: 302 = exists. Mirrorbits returns
    # the 302 immediately without a body, so no transfer happens.
    req = urllib.request.Request(url, method="GET")
    try:
        resp = _opener.open(req, timeout=20)
        resp.close()
        return True
    except urllib.request.HTTPError as e:
        return e.code in (200, 302)
    except Exception:
        return False


def probe_artifact(dev: str, date: str, art: str) -> bool:
    return exists(MIRROR.format(dev=dev, date=date, art=art) + ".img")


_api_cache: dict[str, list | None] = {}


def builds_for(dev: str) -> list | None:
    """Official builds list, newest first. None = API unreachable/empty."""
    if dev in _api_cache:
        return _api_cache[dev]
    try:
        with urllib.request.urlopen(BUILDS_API.format(dev=dev), timeout=30) as r:
            data = json.load(r)
        _api_cache[dev] = data if isinstance(data, list) and data else None
    except Exception:
        _api_cache[dev] = None
    return _api_cache[dev]


def resolve(dev: str, date: str, art: str) -> dict:
    """Decide what to do with one (dev, date, art) URL.

    Returns {"action": "keep" | "swap" | "redate" | "dead", ...}:
      keep   — current URL is alive.
      swap   — another artifact of the SAME date is alive (art=…).
      redate — whole dated build deleted; newest build with a usable
               artifact found via the builds API (date/art/sha256 set).
      dead   — device has no usable artifact anywhere (warn, leave as-is).
    """
    if probe_artifact(dev, date, art):
        return {"action": "keep"}
    # Same-date artifact swap (6-Z272b behavior, unchanged).
    for alt in ORDER:
        if alt != art and probe_artifact(dev, date, alt):
            return {"action": "swap", "art": alt}
    # 6-Z334: whole dated build deleted by the nightly rotation — redate
    # from the builds API (newest build first, artifact ORDER within it).
    for build in builds_for(dev) or []:
        files = {f.get("filename", ""): f for f in build.get("files", [])}
        for a in ORDER:
            f = files.get(a + ".img")
            if f:
                return {
                    "action": "redate",
                    "art": a,
                    "date": build.get("date", "").replace("-", ""),
                    "sha256": f.get("sha256", ""),
                }
    return {"action": "dead"}


def main() -> int:
    path = sys.argv[1] if len(sys.argv) > 1 else "corpus/manifest.yaml"
    lines = open(path).readlines()

    # ── collect entries: (url_idx, sha256_idx, md5_idx, match) ──────────
    entries: list[tuple[int, int | None, int | None, re.Match]] = []
    cur: tuple[int, int | None, int | None, re.Match] | None = None
    for i, line in enumerate(lines):
        if NAME_RE.match(line):
            cur = None  # a new entry starts; url/sha/md5 lines re-bind below
        m = URL_RE.match(line.rstrip("\n"))
        if m:
            cur = (i, None, None, m)
            entries.append(cur)
        elif cur is not None:
            if re.match(r"^\s*sha256:", line) and cur[1] is None:
                cur = (cur[0], i, cur[2], cur[3])
                entries[-1] = cur
            elif re.match(r"^\s*md5:", line) and cur[2] is None:
                cur = (cur[0], cur[1], i, cur[3])
                entries[-1] = cur

    urls = sorted(set((m.group(1), m.group(2), m.group(3)) for *_, m in entries))
    print(f"{len(urls)} LineageOS mirror URLs in manifest")

    # Parallel probe+resolve (6-Z272b: 32 workers; probe GETs return
    # immediately; the API fallback fires at most once per dead device).
    with concurrent.futures.ThreadPoolExecutor(max_workers=32) as pool:
        results = dict(zip(urls, pool.map(lambda t: resolve(*t), urls)))

    swapped = redated = dead = 0
    for dev, date, art in urls:
        r = results[(dev, date, art)]
        mine = [e for e in entries
                if e[3].group(1) == dev and e[3].group(2) == date
                and e[3].group(3) == art]
        if r["action"] == "keep":
            print(f"{dev}/{date}: keeping {art}.img")
        elif r["action"] == "swap":
            old = f"/full/{dev}/{date}/{art}.img"
            new = f"/full/{dev}/{date}/{r['art']}.img"
            for ui, si, mi, _ in mine:
                lines[ui] = lines[ui].replace(old, new)
                # Stale pins would fail the CI checksum gate — clear them.
                if si is not None and 'sha256: ""' not in lines[si]:
                    lines[si] = re.sub(r"(sha256:\s*).*", r'\1""', lines[si])
                if mi is not None and 'md5: ""' not in lines[mi]:
                    lines[mi] = re.sub(r"(md5:\s*).*", r'\1""', lines[mi])
            swapped += 1
            print(f"{dev}: {art}.img -> {r['art']}.img (same date, pins cleared)")
        elif r["action"] == "redate":
            for ui, si, mi, _ in mine:
                lines[ui] = lines[ui].replace(
                    f"/full/{dev}/{date}/", f"/full/{dev}/{r['date']}/"
                ).replace(f"/{art}.img", f"/{r['art']}.img")
                if si is not None:
                    # lambda repl: a hex sha256 would be parsed as a group
                    # reference when interpolated into the template (\1205…).
                    lines[si] = re.sub(
                        r"(sha256:\s*).*",
                        lambda m, s=r["sha256"]: m.group(1) + s,
                        lines[si],
                    )
                if mi is not None:
                    lines[mi] = re.sub(r"(md5:\s*).*", r'\1""', lines[mi])
            redated += 1
            print(
                f"{dev}: {date}/{art}.img -> {r['date']}/{r['art']}.img "
                f"(build rotated; sha256 {r['sha256'][:12]}… from builds API)"
            )
        else:
            dead += 1
            print(f"{dev}/{date}: DEAD and no newer build — leaving as-is (WARN)")

    open(path, "w").writelines(lines)
    print(f"done: {swapped} artifact swap(s), {redated} redate(s), {dead} unfixable")
    return 0


if __name__ == "__main__":
    sys.exit(main())
