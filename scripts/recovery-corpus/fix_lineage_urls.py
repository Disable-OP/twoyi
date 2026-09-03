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

Usage: fix_lineage_urls.py corpus/manifest.yaml [--probe-limit N]
"""
import concurrent.futures
import re
import sys
import urllib.request

MIRROR = "https://mirrorbits.lineageos.org/full/{dev}/{date}/{art}"
ORDER = ["recovery", "vendor_boot", "boot"]
URL_RE = re.compile(
    r"https://mirrorbits\.lineageos\.org/full/([a-z0-9_]+)/([0-9]{8})/(recovery|vendor_boot|boot)"
)


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


def main() -> int:
    path = sys.argv[1] if len(sys.argv) > 1 else "corpus/manifest.yaml"
    src = open(path).read()

    urls = sorted(set(URL_RE.findall(src)))
    print(f"{len(urls)} LineageOS mirror URLs in manifest")

    # 6-Z272b: probe in parallel (284 devices x sequential HEADs exceeded the
    # interactive time budget; 32 workers finish in ~2 min).
    probe_targets = [(dev, date, art) for dev, date, _ in urls for art in ORDER]
    with concurrent.futures.ThreadPoolExecutor(max_workers=32) as pool:
        probe_ok = set(
            pool.map(
                lambda t: (t[0], t[1], t[2])
                # 6-Z272l: the probe URL must carry the .img extension —
                # mirrorbits 404s extension-less paths (the original probe
                # silently answered False for EVERY artifact because of the
                # missing suffix, so no URL was ever rewritten).
                if exists(MIRROR.format(dev=t[0], date=t[1], art=t[2]) + ".img")
                else (t[0], t[1], None),
                probe_targets,
            )
        )
    available: dict[tuple[str, str], str] = {}
    for dev, date, art in probe_ok:
        if art is not None:
            available.setdefault((dev, date), art)  # ORDER order preserved

    swapped = 0
    for dev, date, current in urls:
        chosen = available.get((dev, date), current)
        if chosen != current:
            old = f"/full/{dev}/{date}/{current}.img"
            new = f"/full/{dev}/{date}/{chosen}.img"
            src = src.replace(old, new)
            swapped += 1
            print(f"{dev}: {current}.img -> {chosen}.img")
        else:
            print(f"{dev}: keeping {current}.img")

    open(path, "w").write(src)
    print(f"done: {swapped} URL(s) rewritten")
    return 0


if __name__ == "__main__":
    sys.exit(main())
