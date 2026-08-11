#!/usr/bin/env bash
# scripts/upload-to-0x0.sh — replace actions/upload-artifact with 0x0.st URLs.
#
# GitHub Actions storage is scarce (0.5 GB free tier; this repo was down to
# 0.01 GB). Instead of storing artifacts on GitHub, this script uploads each
# artifact to https://0x0.st (a free, anonymous, curl-friendly file host) and
# prints the returned URL to:
#
#   1. stdout                          — visible in the live GHA log
#   2. $GITHUB_STEP_SUMMARY            — rendered as markdown on the run page
#   3. /tmp/0x0-uploads.txt            — aggregated list for the final
#                                        "all upload links" step
#
# Usage:
#   scripts/upload-to-0x0.sh <label> <path> [path...]
#
#   <label>  Human-readable name shown in the summary. No shell escaping
#            needed — it's only ever interpolated into single-quoted markdown.
#   <path>   One or more files, directories, or globs. Behaviour:
#              - exactly 1 file → upload as-is (preserves filename extension)
#              - multiple files → tar+gzip them together as <label>.tar.gz
#              - directory      → tar+gzip the directory contents
#              - glob with 0 matches → print warning, exit 0 (mirrors
#                actions/upload-artifact's `if-no-files-found: warn`)
#
# Exit codes:
#   0  upload succeeded, OR file was missing, OR upload failed but we
#      chose not to fail the workflow (0x0.st is best-effort — losing
#      an artifact URL is annoying, failing the build over it is worse)
#   1  usage error (wrong arg count)
#
# 0x0.st behaviour reference (https://0x0.st):
#   - POST multipart/form-data with field name `file=@localpath`
#   - Response body: plain-text URL ending in `\n`, e.g. `https://0x0.st/XxYy.apk`
#   - Max size: 512 MiB. Larger uploads are rejected with HTTP 413.
#   - Retention: 365 days for <100 KiB, scaling down to 30 days for ~100 MiB,
#     and shorter for larger files. We don't warn on retention — the user
#     can re-run the workflow if a link has expired.
#   - The site occasionally rate-limits (HTTP 429). We retry up to 3 times
#     with exponential backoff (2s, 4s, 8s).
#
# Why this lives in scripts/ and not as an inline `run:` block:
#   - Every workflow needs the same logic (compress multi-file artifacts,
#     retry on 429, write to summary + aggregate file). Duplicating ~60
#     lines of bash across 4 workflows would be a maintenance hazard.
#   - Keeping it as a versioned script means a future switch to a different
#     host (e.g. file.io, transfer.sh, a self-hosted nginx) only requires
#     editing one file.

set -u

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <label> <path> [path...]" >&2
    exit 1
fi

LABEL="$1"
shift
PATHS=( "$@" )

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------
ENDPOINT="https://0x0.st"
MAX_BYTES=$((512 * 1024 * 1024))   # 512 MiB — 0x0.st hard limit
MAX_RETRIES=3
AGGREGATE_FILE="/tmp/0x0-uploads.txt"

# Sanitize the label into a filename-safe slug for the .tar.gz case.
# We only use [a-zA-Z0-9._-]; everything else collapses to '-'.
slug() {
    # shellcheck disable=SC2001
    echo "$1" | sed -E 's/[^a-zA-Z0-9._-]+/-/g; s/^-+//; s/-+$//'
}

# ---------------------------------------------------------------------------
# Expand globs into a concrete list of files.
# bash globs that don't match expand to the literal pattern; we filter
# those out with `[ -e "$p" ]`.
# ---------------------------------------------------------------------------
EXPANDED=()
for p in "${PATHS[@]}"; do
    if [ -e "$p" ]; then
        EXPANDED+=( "$p" )
    fi
done

if [ "${#EXPANDED[@]}" -eq 0 ]; then
    echo "⚠  [$LABEL] no files matched '${PATHS[*]}' — skipping upload" >&2
    exit 0
fi

# ---------------------------------------------------------------------------
# Decide what to upload:
#   - 1 file           → upload directly
#   - >1 file OR dir   → tar+gzip into /tmp/<slug>.tar.gz, upload that
# ---------------------------------------------------------------------------
UPLOAD_PATH=""
UPLOAD_NAME=""
CLEANUP=""

if [ "${#EXPANDED[@]}" -eq 1 ] && [ -f "${EXPANDED[0]}" ]; then
    UPLOAD_PATH="${EXPANDED[0]}"
    UPLOAD_NAME="$(basename "$UPLOAD_PATH")"
else
    SLUG="$(slug "$LABEL")"
    [ -z "$SLUG" ] && SLUG="artifact"
    UPLOAD_PATH="/tmp/${SLUG}.tar.gz"
    UPLOAD_NAME="${SLUG}.tar.gz"
    CLEANUP="$UPLOAD_PATH"

    echo "→ [$LABEL] packing ${#EXPANDED[@]} entries into $UPLOAD_NAME" >&2
    # `-h` follows symlinks (the kvm-e2e workflow sometimes globs into
    # /tmp/ci-artifacts which is itself fine, but screenshots may be
    # symlinked). `--owner=0 --group=0 --numeric-owner` strips the runner
    # user's uid so the tarball is reproducible.
    tar -czf "$UPLOAD_PATH" \
        --owner=0 --group=0 --numeric-owner \
        "${EXPANDED[@]}" >&2 2>&1 || {
        echo "✗ [$LABEL] tar failed" >&2
        exit 0
    }
fi

# ---------------------------------------------------------------------------
# Size check — 0x0.st rejects >512 MiB with HTTP 413.
# ---------------------------------------------------------------------------
SIZE=$(stat -c %s "$UPLOAD_PATH" 2>/dev/null || stat -f %z "$UPLOAD_PATH" 2>/dev/null || echo 0)
if [ "$SIZE" -gt "$MAX_BYTES" ]; then
    echo "✗ [$LABEL] $UPLOAD_NAME is $((SIZE / 1024 / 1024)) MiB — exceeds 0x0.st 512 MiB limit, skipping" >&2
    [ -n "$CLEANUP" ] && rm -f "$CLEANUP"
    exit 0
fi
echo "→ [$LABEL] uploading $UPLOAD_NAME ($((SIZE / 1024 / 1024)) MiB) to 0x0.st" >&2

# ---------------------------------------------------------------------------
# Upload with retry. 0x0.st returns the URL in the response body (plain text,
# trailing newline). curl's `-w '%{http_code}'` lets us distinguish a
# transient 429/5xx from a real success.
# ---------------------------------------------------------------------------
URL=""
for attempt in $(seq 1 "$MAX_RETRIES"); do
    RESPONSE_FILE="$(mktemp)"
    HTTP_CODE=$(curl -sS \
        -o "$RESPONSE_FILE" \
        -w '%{http_code}' \
        -F "file=@${UPLOAD_PATH}" \
        -F "expires=90" \
        "$ENDPOINT" 2>/dev/null || echo "000")

    BODY="$(cat "$RESPONSE_FILE" 2>/dev/null || true)"
    rm -f "$RESPONSE_FILE"

    if [ "$HTTP_CODE" = "200" ] && [ -n "$BODY" ]; then
        # Strip trailing whitespace/newlines.
        URL="$(printf '%s' "$BODY" | tr -d '[:space:]')"
        break
    fi

    echo "  attempt $attempt/$MAX_RETRIES: HTTP $HTTP_CODE — $BODY" >&2
    if [ "$attempt" -lt "$MAX_RETRIES" ]; then
        BACKOFF=$((2 ** attempt))
        echo "  retrying in ${BACKOFF}s..." >&2
        sleep "$BACKOFF"
    fi
done

[ -n "$CLEANUP" ] && rm -f "$CLEANUP"

if [ -z "$URL" ]; then
    echo "✗ [$LABEL] upload failed after $MAX_RETRIES attempts" >&2
    exit 0
fi

# ---------------------------------------------------------------------------
# Surface the URL in 3 places: stdout, GITHUB_STEP_SUMMARY, aggregate file.
# ---------------------------------------------------------------------------
echo "✓ [$LABEL] $URL"

# 1. stdout (already done above)

# 2. GHA step summary — markdown table row. We append; the workflow's
#    final "All upload links" step can re-print the aggregate file as
#    a nice table. Using >> here so multiple upload steps in the same
#    job accumulate.
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    {
        printf '| %s | [`%s`](%s) |\n' "$LABEL" "$UPLOAD_NAME" "$URL"
    } >> "$GITHUB_STEP_SUMMARY"
fi

# 3. Aggregate file — for the final summary step to consume.
{
    printf '%s\t%s\t%s\n' "$LABEL" "$UPLOAD_NAME" "$URL"
} >> "$AGGREGATE_FILE"

exit 0
