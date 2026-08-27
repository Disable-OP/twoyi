#!/usr/bin/env python3
"""
Find all <string> entries in values/strings.xml (English — the default)
that are missing from any of the 3 translation files:
  - values-zh-rCN/strings.xml
  - values-zh-rTW/strings.xml
  - values-ja/strings.xml

Also reports strings marked translatable="false" (which are correctly
excluded from translations — those don't need to be added).

Output: a list of (key, missing_in_locales) tuples.
"""

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
RES_DIR = REPO_ROOT / "app/src/main/res"

DEFAULT_FILE = RES_DIR / "values" / "strings.xml"
LOCALE_FILES = {
    "zh-CN": RES_DIR / "values-zh-rCN" / "strings.xml",
    "zh-TW": RES_DIR / "values-zh-rTW" / "strings.xml",
    "ja":    RES_DIR / "values-ja" / "strings.xml",
}

# Match <string name="key"> or <string name="key" translatable="false">
STRING_RE = re.compile(r'<string\s+name="([^"]+)"([^>]*)>')


def parse_strings(xml_path: Path) -> dict[str, bool]:
    """
    Return {key: is_translatable} for every <string> in the file.
    is_translatable is False if the element has translatable="false",
    True otherwise.
    """
    text = xml_path.read_text(encoding="utf-8")
    result: dict[str, bool] = {}
    for m in STRING_RE.finditer(text):
        key = m.group(1)
        attrs = m.group(2)
        is_translatable = 'translatable="false"' not in attrs
        result[key] = is_translatable
    return result


def main() -> int:
    default_strings = parse_strings(DEFAULT_FILE)
    print(f"Default file has {len(default_strings)} strings "
          f"({sum(1 for v in default_strings.values() if v)} translatable, "
          f"{sum(1 for v in default_strings.values() if not v)} non-translatable)")

    # Load each locale's strings
    locale_strings: dict[str, set[str]] = {}
    for locale, path in LOCALE_FILES.items():
        strings = parse_strings(path)
        locale_strings[locale] = set(strings.keys())
        print(f"  {locale} file has {len(strings)} strings")

    print()
    print("=== Missing translations (translatable strings in default "
          "but missing from a locale) ===")
    missing: list[tuple[str, list[str]]] = []
    for key, is_translatable in default_strings.items():
        if not is_translatable:
            continue
        missing_in = [
            locale for locale, keys in locale_strings.items()
            if key not in keys
        ]
        if missing_in:
            missing.append((key, missing_in))
            print(f"  {key}: missing from {', '.join(missing_in)}")

    print()
    print(f"=== TOTAL: {len(missing)} strings missing from at least one locale ===")

    # Also report strings in locale files that AREN'T in the default
    # (extra/orphan translations — usually stale leftovers from removed
    # English strings).
    print()
    print("=== Orphan translations (in locale file but not in default) ===")
    default_keys = set(default_strings.keys())
    for locale, keys in locale_strings.items():
        orphans = keys - default_keys
        for o in sorted(orphans):
            print(f"  {locale}: {o}")
        if not orphans:
            print(f"  {locale}: (none)")

    return 0 if not missing else 1


if __name__ == "__main__":
    sys.exit(main())
