#!/usr/bin/env python3
"""Universal recovery corpus scraper — broadens coverage toward the
900/1000 boot-rate target (master prompt §17/§18).

Scrapes TWRP's device catalog (dl.twrp.me lists every device + version
side-by-side, ~500 devices × multiple versions) and OrangeFox's release
feed (api.orangefox.download exposes a JSON release index). Each scraped
entry becomes a manifest.yaml image entry with tier=nightly (the PR tier
stays the curated small representative set).

TWO OUTPUT MODES
  --list          Just print "<name>\t<url>\t<referer>\t<family>" lines
                  (no yaml emitted). Useful for preview + manual triage.
  --yaml          Emit ready-to-append yaml entries (the default).

USAGE
  scrape_corpus.py --yaml >> corpus/manifest.yaml
  scrape_corpus.py --list | head -50
  scrape_corpus.py --vendor Google --yaml    # only one vendor

WHAT IT SCRAPES
  TWRP:   twrp.me/Devices/ lists vendors (Asus/Google/LG/...);
          each vendor page links to per-device pages on twrp.me/<vendor>/
          <device>.html; each device page contains a "Download Links:"
          section that links to dl.twrp.me/<codename>/ (the codename
          often differs from the device page name — e.g. "googlepixel"
          page → dl.twrp.me/sailfish/). The dl.twrp.me/<codename>/
          index lists all the .img.html files; we pick the LATEST per
          device (one entry per device, not per-version — keeps the
          corpus to ~500 entries rather than 2000+, the nightly tier
          already takes hours).
  OrangeFox:  api.orangefox.download/v2/releases?list=releases
          returns a JSON array of release slugs; each slug maps to a
          /v2/releases/<slug> JSON object with a direct download URL.

The script is idempotent: re-running it appends NEW entries only (it
skips any (name) that already appears in the manifest). It is also
DATA-ONLY (master prompt §32): no recovery-specific logic, no per-image
branching — just a list of (name, url, referer, md5, family, device,
arch, generation, tier, notes) rows.
"""
import argparse
import html
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

USER_AGENT = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 TwoyiCorpus/1.0"
DEFAULT_TIMEOUT = 20


def http_get(url, referer=None, timeout=DEFAULT_TIMEOUT, max_retries=3):
    """HTTP GET that follows redirects + sets Referer (dl.twrp.me anti-leech)."""
    req = urllib.request.Request(url)
    req.add_header("User-Agent", USER_AGENT)
    if referer:
        req.add_header("Referer", referer)
    last_err = None
    for attempt in range(max_retries):
        try:
            with urllib.request.urlopen(req, timeout=timeout) as r:
                return r.read().decode("utf-8", "replace")
        except urllib.error.HTTPError as e:
            last_err = e
            if e.code == 404:
                return None
            if attempt + 1 < max_retries:
                time.sleep(1.0 * (attempt + 1))
                continue
            return None
        except urllib.error.URLError as e:
            last_err = e
            if attempt + 1 < max_retries:
                time.sleep(1.0 * (attempt + 1))
                continue
            return None
    return None


# ───────────────────────────── TWRP scraper ──────────────────────────

def twrp_vendor_pages():
    """Return list of vendor names from twrp.me/Devices/."""
    html_text = http_get("https://twrp.me/Devices/")
    if not html_text:
        return []
    seen = set()
    # The page links to /Devices/<Vendor>/ pages — pick those hrefs.
    for m in re.finditer(r'href="/Devices/([^/]+)/?"', html_text):
        name = html.unescape(m.group(1)).strip()
        if name and re.match(r"^[A-Z][a-zA-Z& ]+$", name):
            seen.add(name)
    return sorted(seen)


def twrp_device_pages(vendor):
    """Return list of (device_codename, device_page_url) for a vendor."""
    url = f"https://twrp.me/Devices/{urllib.parse.quote(vendor)}/"
    html_text = http_get(url)
    if not html_text:
        return []
    out = []
    seen = set()
    # twrp.me/<vendor>/<device>.html — pick those hrefs.
    pat = re.compile(
        r'href="/([a-zA-Z0-9_]+)/([a-zA-Z0-9_]+)\.html"',
    )
    for m in pat.finditer(html_text):
        v, dev = m.group(1).lower(), m.group(2).lower()
        # Skip non-vendor matches (e.g., "about", "FAQ", "Devices").
        if v in ("about", "faq", "contactus", "devices", "search"):
            continue
        if dev and dev not in seen:
            seen.add(dev)
            out.append((dev, f"https://twrp.me/{v}/{dev}.html"))
    return out


def twrp_device_codename(device_page_html):
    """Find the dl.twrp.me/<codename>/ link on a device page."""
    # The page has a "Download Links:" section that links to
    # dl.twrp.me/<codename> — the codename may differ from the page name.
    matches = set()
    for m in re.finditer(r'dl\.twrp\.me/([a-z0-9_]+)', device_page_html, re.I):
        codename = m.group(1)
        # Skip generic slugs.
        if codename not in ("twrpapp",):
            matches.add(codename)
    # Prefer the FIRST match (usually the canonical device codename).
    if not matches:
        return None
    # Heuristic: pick the longest codename (the real one is usually
    # longer than generic placeholders).
    return sorted(matches, key=lambda c: (-len(c), c))[0]


def twrp_latest_img(device_codename, dl_index_html):
    """Pick the latest .img file from a dl.twrp.me/<device>/ index page."""
    # The page lists .img.html files. Pick the one with the highest
    # version number (parse 3.7.0_9-0 → tuple (3, 7, 0, 9, 0)).
    best_version = None
    best_url = None
    best_referer = None
    # Pattern: /<codename>/twrp-<version>-<release>-<codename>.img.html
    # Example: /sailfish/twrp-3.7.0_9-0-sailfish.img.html
    pat = re.compile(
        r'href="(/' + re.escape(device_codename)
        + r'/twrp-([0-9][0-9._]*?)-([0-9]+)-'
        + re.escape(device_codename) + r'\.img\.html)"',
        re.IGNORECASE,
    )
    for m in pat.finditer(dl_index_html):
        page = m.group(1)
        ver_str = m.group(2)
        rel_str = m.group(3)
        # Build a sortable tuple: 3.7.0_9 → (3, 7, 0, 9); release: 0 → 0
        try:
            ver_parts = [int(p) for p in re.split(r"[._]", ver_str) if p.isdigit()]
            rel_num = int(rel_str) if rel_str.isdigit() else 0
            ver_tuple = tuple(ver_parts + [rel_num])
        except ValueError:
            continue
        if best_version is None or ver_tuple > best_version:
            best_version = ver_tuple
            best_referer = f"https://dl.twrp.me{page}"
            best_url = page
    if best_url is None:
        return None, None, None
    # The .html page contains the direct .img link.
    page_url = f"https://dl.twrp.me{best_url}"
    page_html = http_get(page_url)
    if not page_html:
        return None, None, None
    # Direct img link: href="/<device>/twrp-...-<device>.img"
    m2 = re.search(
        r'href="(/' + re.escape(device_codename)
        + r'/twrp-[0-9][0-9._]*?-[0-9]+-' + re.escape(device_codename)
        + r'\.img)"',
        page_html,
    )
    if m2:
        img_url = f"https://dl.twrp.me{m2.group(1)}"
        ver_str = ".".join(str(x) for x in best_version[:-1]) \
            + f"-{best_version[-1]}"
        return img_url, best_referer, ver_str
    return None, None, None


def twrp_scrape(vendor_filter=None, max_devices=None):
    """Scrape TWRP — yield (name, url, referer, family, device, version)."""
    vendors = twrp_vendor_pages()
    if vendor_filter:
        vendors = [v for v in vendors if v.lower() == vendor_filter.lower()]
    count = 0
    for vendor in vendors:
        try:
            devices = twrp_device_pages(vendor)
        except Exception as e:
            print(f"# skip vendor {vendor}: {e}", file=sys.stderr)
            continue
        for dev_page_name, page_url in devices:
            if max_devices is not None and count >= max_devices:
                return
            try:
                page_html = http_get(page_url)
                if not page_html:
                    continue
                codename = twrp_device_codename(page_html)
                if not codename:
                    continue
                idx_html = http_get(f"https://dl.twrp.me/{codename}/")
                if not idx_html:
                    continue
                img_url, referer, version = twrp_latest_img(codename, idx_html)
                if not img_url:
                    continue
                name = f"twrp-{version}-{codename}"
                yield {
                    "name": name,
                    "url": img_url,
                    "referer": referer or "",
                    "family": "TWRP",
                    "device": codename,
                    "arch": "arm64",
                    "generation": "11",
                    "tier": "nightly",
                    "notes": f"Scraped from TWRP catalog (vendor: {vendor}, "
                             f"device page: {dev_page_name}).",
                }
                count += 1
            except Exception as e:
                print(f"# skip device {dev_page_name}: {e}", file=sys.stderr)
                continue


# ─────────────────────────── OrangeFox scraper ──────────────────────

def orangefox_scrape(max_devices=None):
    """Scrape OrangeFox's release feed — yield per-device latest stable."""
    count = 0
    try:
        idx = http_get(
            "https://api.orangefox.download/v2/releases?list=releases",
            timeout=60,
        )
        if not idx:
            return
        try:
            slugs = json.loads(idx)
        except json.JSONDecodeError:
            print("# orangefox: bad JSON from releases index", file=sys.stderr)
            return
        for slug in slugs:
            if max_devices is not None and count >= max_devices:
                return
            try:
                rel = http_get(
                    f"https://api.orangefox.download/v2/releases/{slug}",
                    timeout=30,
                )
                if not rel:
                    continue
                rel_json = json.loads(rel)
                url = None
                md5 = None
                device = rel_json.get("device", slug)
                for v in rel_json.get("variants", []):
                    if v.get("variant") == "Stable" or v.get("type") == "stable":
                        url = v.get("download_url") or v.get("url")
                        md5 = v.get("md5")
                        break
                if not url:
                    if rel_json.get("variants"):
                        v = rel_json["variants"][0]
                        url = v.get("download_url") or v.get("url")
                        md5 = v.get("md5")
                if not url:
                    continue
                if not url.startswith("http"):
                    url = f"https://api.orangefox.download/release/{slug}/dl"
                name = f"orangefox-{slug}"
                yield {
                    "name": name,
                    "url": url,
                    "referer": "",
                    "family": "OrangeFox",
                    "device": device,
                    "arch": "arm64",
                    "generation": "11",
                    "tier": "nightly",
                    "notes": f"OrangeFox stable for {device}.",
                }
                count += 1
            except Exception as e:
                print(f"# skip orangefox {slug}: {e}", file=sys.stderr)
                continue
    except Exception as e:
        print(f"# orangefox scrape failed: {e}", file=sys.stderr)


# ─────────────────────────── Manifest YAML emit ─────────────────────

def entry_to_yaml(entry):
    """Format an entry dict as a yaml image: block."""
    notes = entry.get("notes", "").replace('"', '\\"')
    out = [
        f"  - name: {entry['name']}",
        f"    family: {entry['family']}",
        f"    device: {entry['device']}",
        f"    arch: {entry['arch']}",
        f'    generation: "{entry["generation"]}"',
        f"    tier: {entry['tier']}",
        f"    url: {entry['url']}",
        f"    referer: \"{entry.get('referer', '') or ''}\"",
        f"    md5: \"{entry.get('md5', '') or ''}\"",
        f"    sha256: \"\"",
        f"    notes: >-",
        f"      {notes}",
    ]
    return "\n".join(out)


def existing_manifest_names(manifest_path):
    """Return the set of `name:` values already in the manifest."""
    seen = set()
    try:
        with open(manifest_path, "r") as f:
            for line in f:
                m = re.match(r"^\s*-\s*name:\s*(\S+)", line)
                if m:
                    seen.add(m.group(1))
    except FileNotFoundError:
        pass
    return seen


# ─────────────────────────────── main ────────────────────────────────

def twrp_scrape_codenames(codenames):
    """Fast-path scraper: given a list of dl.twrp.me codenames,
    fetch each /<codename>/ page directly + emit one entry per codename."""
    for codename in codenames:
        codename = codename.strip()
        if not codename or codename.startswith("#"):
            continue
        try:
            idx_html = http_get(f"https://dl.twrp.me/{codename}/")
            if not idx_html:
                continue
            img_url, referer, version = twrp_latest_img(codename, idx_html)
            if not img_url:
                continue
            name = f"twrp-{version}-{codename}"
            yield {
                "name": name,
                "url": img_url,
                "referer": referer or "",
                "family": "TWRP",
                "device": codename,
                "arch": "arm64",
                "generation": "11",
                "tier": "nightly",
                "notes": f"TWRP catalog device {codename}.",
            }
        except Exception as e:
            print(f"# skip codename {codename}: {e}", file=sys.stderr)
            continue


def main():
    p = argparse.ArgumentParser(description="Twoyi corpus scraper")
    p.add_argument("--yaml", action="store_true",
                   help="Emit yaml entries ready to append to manifest")
    p.add_argument("--list", action="store_true",
                   help="Print tab-separated list (preview)")
    p.add_argument("--vendor", default=None,
                   help="Restrict TWRP scrape to one vendor (e.g., Google)")
    p.add_argument("--codenames", default=None,
                   help="File with TWRP codenames (one per line) "
                        "— fast path skipping twrp.me/Devices/ walk")
    p.add_argument("--max", type=int, default=None,
                   help="Maximum number of entries to emit per source")
    p.add_argument("--manifest", default="corpus/manifest.yaml",
                   help="Manifest path (for dedup on --yaml)")
    p.add_argument("--source", choices=["twrp", "orangefox", "all"],
                   default="all")
    args = p.parse_args()

    if not args.yaml and not args.list:
        args.yaml = True  # default

    seen = existing_manifest_names(args.manifest) if args.yaml else set()
    out_count = 0

    sources = []
    if args.codenames:
        try:
            with open(args.codenames) as f:
                codenames = [line.strip() for line in f]
        except FileNotFoundError:
            sys.exit(f"codenames file not found: {args.codenames}")
        sources.append(("twrp-codenames", twrp_scrape_codenames(codenames)))
    elif args.source in ("twrp", "all"):
        sources.append(("twrp", twrp_scrape(
            vendor_filter=args.vendor, max_devices=args.max)))
    if args.source in ("orangefox", "all") and not args.codenames:
        sources.append(("orangefox", orangefox_scrape(max_devices=args.max)))

    for src_name, gen in sources:
        for entry in gen:
            if entry["name"] in seen:
                continue
            seen.add(entry["name"])
            if args.list:
                print(f"{entry['name']}\t{entry['url']}\t"
                      f"{entry.get('referer', '')}\t{entry['family']}")
            else:
                print(entry_to_yaml(entry))
                print()  # blank line between entries for readability
            out_count += 1
    print(f"# scraped {out_count} new entries from "
          f"{args.source} (deduped against {args.manifest})",
          file=sys.stderr)


if __name__ == "__main__":
    main()
