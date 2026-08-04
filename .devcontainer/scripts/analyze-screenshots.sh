#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Send each screenshot in $SCREENSHOT_DIR to a vision LLM and ask it
# to (a) describe what's on screen and (b) give tap coordinates for
# the next action.
#
# Uses the z-ai-web-dev-sdk VLM skill (see /skills/VLM). Defaults to
# GLM-4.6V; pass an env override to use a newer model if available.
#
#   TWOYI_VLM_MODEL=glm-5-vision-turbo ./.devcontainer/scripts/analyze-screenshots.sh

set -e

SCREENSHOT_DIR="${TWOYI_SCREENSHOTS:-/tmp/twoyi-screenshots}"
VLM_MODEL="${TWOYI_VLM_MODEL:-glm-4.6v}"
ANALYSIS_DIR="${TWOYI_ANALYSIS:-/tmp/twoyi-analysis}"
mkdir -p "$ANALYSIS_DIR"

if [ ! -d "$SCREENSHOT_DIR" ] || [ -z "$(ls -A "$SCREENSHOT_DIR" 2>/dev/null)" ]; then
    echo "✗ No screenshots in $SCREENSHOT_DIR."
    echo "  Run ./.devcontainer/scripts/test-twoyi.sh first."
    exit 1
fi

echo "================================================================"
echo "  Analyzing screenshots with VLM model: $VLM_MODEL"
echo "  Screenshots: $SCREENSHOT_DIR"
echo "  Analysis:    $ANALYSIS_DIR"
echo "================================================================"
echo ""

# The z-ai CLI is the simplest way to call the VLM. We send each
# screenshot with a structured prompt and capture the response.
PROMPT='You are analyzing a screenshot of the Twoyi Android container app
running inside a redroid x86_64 emulator.

Describe:
  1. What UI elements are visible (buttons, text fields, lists, dialogs).
  2. The current state (booting? settings screen? error? container home?).
  3. If a "Launch Container" button is visible, give its approximate
     tap coordinates as: TAP: <x>,<y>
  4. Any error messages or anomalies.

Be concise (under 150 words).'

for screenshot in "$SCREENSHOT_DIR"/*.png; do
    name=$(basename "$screenshot" .png)
    out="$ANALYSIS_DIR/${name}.txt"

    echo "── Analyzing $name ──"
    echo ""

    # z-ai function vlm_analyze expects a base64 image and a prompt.
    # We base64 the image and pass via --args.
    IMG_B64=$(base64 -w0 "$screenshot")

    z-ai function -n vlm_analyze \
        -a "{\"image\":\"$IMG_B64\",\"prompt\":\"$PROMPT\",\"model\":\"$VLM_MODEL\"}" \
        -o "$out" 2>&1 | tail -3 || true

    if [ -f "$out" ]; then
        echo "Response:"
        cat "$out"
        echo ""
    else
        echo "(no response — see error above)"
    fi
    echo ""
done

echo "================================================================"
echo "  ✓  Analysis complete. Results in $ANALYSIS_DIR"
echo "================================================================"
