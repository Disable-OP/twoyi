#!/usr/bin/env bash
# scripts/pack-logs.sh — compress logs into a single .tar.xz for upload.
#
# Why this exists: GitHub Actions storage was at 0.49 GB used (0.01 GB
# free) until we deleted 1217 old artifacts (4.61 GB). Going forward
# we upload ONLY logs — no APKs, no screenshots — and we compress them
# with xz (ratio ~10:1 on text logs) before handing off to
# actions/upload-artifact@v4. A typical KVM-e2e run's logcat +
# twoyi-loader.log + kr64-stderr.log goes from ~6 MB raw to ~0.5 MB
# compressed, so the 0.5 GB free tier now holds ~1000 runs instead of
# ~80.
#
# Usage:
#   scripts/pack-logs.sh <output.tar.xz> <path> [path...]
#
#   <output.tar.xz>  Path where the compressed archive will be written.
#                    Parent directory must exist.
#   <path>           One or more files, directories, or globs. Files
#                    that don't exist are skipped silently (mirrors
#                    actions/upload-artifact's `if-no-files-found: ignore`).
#
# Exit codes:
#   0  archive created (even if empty — caller can check size)
#   1  usage error
#
# What to do with the archive in a workflow:
#   - name: Pack logs
#     run: scripts/pack-logs.sh /tmp/logs.tar.xz /tmp/ci-artifacts/ app/build/outputs/logs/
#   - uses: actions/upload-artifact@v4
#     with:
#       name: twoyi-logs
#       path: /tmp/logs.tar.xz
#       retention-days: 7
#       if-no-files-found: error

set -u

if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <output.tar.xz> <path> [path...]" >&2
    exit 1
fi

OUT="$1"
shift
PATHS=( "$@" )

# Expand globs, filter missing entries.
EXPANDED=()
for p in "${PATHS[@]}"; do
    if [ -e "$p" ]; then
        EXPANDED+=( "$p" )
    else
        echo "  (skipping $p — not found)" >&2
    fi
done

if [ "${#EXPANDED[@]}" -eq 0 ]; then
    echo "⚠  no files matched any of ${PATHS[*]}" >&2
    # Create an empty marker archive so the upload step has something
    # to attach (with if-no-files-found: warn this won't fail the build).
    mkdir -p "$(dirname "$OUT")"
    echo "no logs were produced by this run" > /tmp/NO_LOGS.txt
    tar -cJf "$OUT" -C /tmp NO_LOGS.txt
    rm -f /tmp/NO_LOGS.txt
    exit 0
fi

# xz compression: -6 is the default and gives good ratio/speed tradeoff
# for text logs. -T 0 uses all cores. --owner=0 --group=0 strips the
# runner user's uid so the tarball is reproducible.
echo "→ packing ${#EXPANDED[@]} entries into $OUT" >&2
tar -cJf "$OUT" \
    --owner=0 --group=0 --numeric-owner \
    "${EXPANDED[@]}" 2>&1 || {
    echo "✗ tar failed" >&2
    exit 0  # don't fail the build over a packing error
}

SIZE=$(stat -c %s "$OUT" 2>/dev/null || stat -f %z "$OUT" 2>/dev/null || echo 0)
echo "✓ $OUT created ($((SIZE / 1024)) KiB)" >&2
