#!/usr/bin/env python3
"""Analyze a screenshot using the z-ai-web-dev-sdk VLM function."""
import json
import sys
import base64
import subprocess

def analyze_image(image_path, prompt, model="glm-4.6v"):
    with open(image_path, 'rb') as f:
        img_b64 = base64.b64encode(f.read()).decode('ascii')

    args = json.dumps({
        "image": img_b64,
        "prompt": prompt,
        "model": model
    })

    result = subprocess.run(
        ['z-ai', 'function', '-n', 'vlm_analyze', '-a', args],
        capture_output=True, text=True, timeout=120
    )
    if result.returncode != 0:
        print(f"Error: {result.stderr}", file=sys.stderr)
        return None
    return result.stdout

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: analyze_screenshot.py <image.png> [prompt] [model]")
        sys.exit(1)

    image_path = sys.argv[1]
    prompt = sys.argv[2] if len(sys.argv) > 2 else "Describe what you see in this Android screenshot. What app is this? What UI elements are visible? What state is the app in? Give tap coordinates for any buttons. Be concise."
    model = sys.argv[3] if len(sys.argv) > 3 else "glm-4.6v"

    print(f"Analyzing {image_path} with model={model}...")
    result = analyze_image(image_path, prompt, model)
    if result:
        print(result)
