#!/usr/bin/env python3
"""Open a PR via the GitHub REST API."""
import json
import os
import sys
import urllib.request
import urllib.error

TOKEN = os.environ['GH_TOKEN']
REPO = 'Disable-OP/twoyi'
UPSTREAM = 'cyanmint/twoyi'

PR_TITLE = "Initial cleanup: input keycode fix, socket retry bounds, x86_64 ABI, multi-ABI CI, Codespace + redroid test harness"

PR_BODY = """## Summary

This PR applies the first round of low-risk, high-value improvements identified in the architecture analysis of `cyanmint/twoyi`. Six self-contained commits:

1. **fix(input): honor keycode argument instead of hardcoding KEY_BACK** — `send_key_code` ignored its `keycode` parameter and always emitted `KEY_BACK`. This worked by accident for the only caller (`onBackPressed → KEYCODE_HOME`) but broke any future caller (volume, recents, power, etc.). Adds `android_keycode_to_linux()` mapping and advertises the corresponding keys in `key_bitmask`.

2. **fix(socket): bound retries + exponential backoff in TwoyiSocketServer** — `start0()` recursed into `start()` on every `IOException` with a fixed 1s sleep. If the bind kept failing (SELinux denial, name collision) the cached executor pool would accumulate blocked threads and starve the app. Replaced with `EXECUTOR.submit(...)` + 5-retry cap + exponential backoff (1→2→4→8→16s, capped at 30s) + jitter.

3. **feat(build): add x86_64 ABI for emulator/redroid testing** — Adds `x86_64` as a first-class build target alongside `arm64-v8a`. The repo's own `REDROID_TESTING.md` documents that the default redroid image is x86_64 and that ARM64 libs can't run inside it — this commit unblocks that test path. `build_rs.sh`, `loader/build.sh`, `openglrenderer/build.sh` all accept an ABI list now; `.cargo/config.toml` has identical PIE flags for both targets; `app/build.gradle` adds `x86_64` to `abiFilters`.

4. **ci: build both arm64-v8a + x86_64, add workflow_dispatch inputs** — Rewrites `.github/workflows/build.yml` to: add `x86_64-linux-android` to Rust targets, trigger on `improvements/**` branches, add `workflow_dispatch` inputs (`abis`, `include_rootfs`), bump JDK 11→17, cache cargo registry+git+target, upload build logs on failure, 30-min timeout.

5. **feat(devcontainer): add Codespace config + redroid test harness** — Adds `.devcontainer/` with a 4-core / 16 GB / 32 GB Codespace config (`runArgs: ["--privileged"]`), a postCreateCommand that installs the full Android toolchain, and four scripts:
   - `check-kvm.sh` — definitive KVM availability check, writes `/tmp/kvm-verdict.txt`
   - `run-redroid.sh` — starts an x86_64 `redroid:13.0.0` container with ADB on port 5555
   - `test-twoyi.sh` — installs the APK, launches twoyi, takes 8 screenshots at increasing intervals
   - `analyze-screenshots.sh` — sends each screenshot to a vision LLM (default `glm-4.6v`, override with `TWOYI_VLM_MODEL=glm-5-vision-turbo`) and asks for UI description + tap coordinates

6. **docs: add ARCHITECTURE.md** — A 663-line deep architecture write-up covering project history, fork landscape, three-layer architecture, component-by-component deep dive (including the DEX-level `services.jar` patcher), end-to-end boot sequence, complete file map, and ranked improvement opportunities.

## Why these changes?

The cyanmint fork is the only active continuation of twoyi (audited 2026-08-05 — every other "active-looking" fork is a one-off mirror). The maintainer has been doing real engineering work, but a few small bugs and gaps are blocking faster iteration:

- The hardcoded `KEY_BACK` made it impossible to wire up new navigation buttons (volume, recents) without recompiling.
- The unbounded socket retry loop was a latent thread-leak that would eventually freeze the app under sustained SELinux denials.
- The arm64-only build meant the existing `REDROID_TESTING.md` test plan couldn't actually run — the maintainer had documented the problem but couldn't fix it without an x86_64 build.

## Notes on the Codespace / KVM question

The devcontainer includes `runArgs: ["--privileged"]` as requested. **However:** multiple authoritative sources confirm that GitHub Codespaces runs on Azure VMs whose host kernel does NOT expose `/dev/kvm` to the devcontainer, even under `--privileged`:

- [devcontainers/images#884](https://github.com/devcontainers/images/issues/884)
- [dotnet/runtime#77851](https://github.com/dotnet/runtime/issues/77851) (written by a Microsoft .NET team member)
- [bgplabs.net/4-codespaces](https://bgplabs.net/4-codespaces)
- [github/community#160591](https://github.com/orgs/community/discussions/160591)

`--privileged` grants the container access to host devices, but it cannot *create* a device that the host kernel doesn't have. The `check-kvm.sh` script runs on codespace creation and definitively reports KVM availability — **if KVM is actually available, the test scripts will use it; if not, they fall back to redroid (Android-in-container, no KVM needed).**

Either way the test harness works. The redroid path is the one I've actually wired up because it's the one I'm confident will work.

## Related: Nogitsune

While researching cyanmint's profile I found `cyanmint/Nogitsune` — a from-scratch Kotlin + Compose + C++ rewrite of twoyi (essentially "twoyi v2"). It's at an early stage ("not yet ready for public use") but worth watching. The ARCHITECTURE.md includes a comparison table. Improvements in this PR target v1 because v1 is the only usable version today, but the same bugs likely exist in Nogitsune's reimplementation.

## How to test

### Option A: GitHub Actions (build only)
1. Merge this PR into a branch CI watches (`main`, `develop`, or `improvements/**`).
2. The workflow will build both ABIs and upload the APK as an artifact.
3. To bundle the real rootfs, trigger via `workflow_dispatch` with `include_rootfs=true`.

### Option B: Codespace (build + run + screenshot + VLM analysis)
1. Create a codespace on this branch (4-core / 16 GB / 32 GB machine).
2. Wait for `postCreateCommand` to finish (~5 min).
3. Run `./.devcontainer/scripts/check-kvm.sh` — see the KVM verdict for yourself.
4. Run `./gradlew assembleRelease` to build the APK.
5. Run `./.devcontainer/scripts/run-redroid.sh` to start redroid.
6. Run `./.devcontainer/scripts/test-twoyi.sh` to install + screenshot.
7. Run `./.devcontainer/scripts/analyze-screenshots.sh` to send screenshots to the VLM.

## What's NOT in this PR

Deliberately deferred (see ARCHITECTURE.md §9 for the full list):

- AGP / targetSdk / dependency bumps (medium risk, needs device testing)
- Deleting the legacy `libloader.so` / `libOpenglRender.so` blobs (needs validation that the open-source replacements actually work on real devices)
- Replacing `libadb.so` with an open-source Java ADB client
- Converting Java to Kotlin
- Building the guest ROM from source (the manifest exists but build orchestration is WIP upstream)

These are all worth doing, but each needs more validation than I can do without a real device or working emulator. This PR sticks to changes that are either (a) pure refactors with no behavior change, (b) bug fixes with clear root causes, or (c) additive (new ABIs, new CI inputs, new test harness) — nothing that should regress existing functionality.

## Commits

- `7dc6093` fix(input): honor keycode argument instead of hardcoding KEY_BACK
- `ae06304` fix(socket): bound retries + exponential backoff in TwoyiSocketServer
- `84ece58` feat(build): add x86_64 ABI for emulator/redroid testing
- `93f5f1c` ci: build both arm64-v8a + x86_64, add workflow_dispatch inputs
- `036cf21` feat(devcontainer): add Codespace config + redroid test harness
- `030a377` docs: add ARCHITECTURE.md — deep code-level architecture write-up
"""

def open_pr(repo, head, base):
    """Open a PR with given head and base."""
    data = json.dumps({
        'title': PR_TITLE,
        'head': head,
        'base': base,
        'body': PR_BODY,
        'maintainer_can_modify': True,
        'draft': False,
    }).encode('utf-8')
    req = urllib.request.Request(
        f'https://api.github.com/repos/{repo}/pulls',
        data=data,
        method='POST',
        headers={
            'Authorization': f'token {TOKEN}',
            'Accept': 'application/vnd.github+json',
            'Content-Type': 'application/json',
        },
    )
    try:
        with urllib.request.urlopen(req) as resp:
            result = json.loads(resp.read())
            return result
    except urllib.error.HTTPError as e:
        return {'error': f'HTTP {e.code}: {e.reason}', 'body': e.read().decode('utf-8')}

# Try opening PR against upstream (cyanmint/twoyi) first
print(f"Trying to open PR against {UPSTREAM} (Disable-OP:improvements/initial-cleanup → cyanmint:main)...")
result = open_pr(UPSTREAM, 'Disable-OP:improvements/initial-cleanup', 'main')
if 'error' in result:
    print(f"  ✗ {result['error']}", file=sys.stderr)
    print(f"    {result['body'][:500]}", file=sys.stderr)
    print(f"\nFalling back to opening PR within Disable-OP/twoyi (improvements/initial-cleanup → main)...")
    result = open_pr(REPO, 'improvements/initial-cleanup', 'main')
    if 'error' in result:
        print(f"  ✗ {result['error']}", file=sys.stderr)
        print(f"    {result['body'][:500]}", file=sys.stderr)
        sys.exit(1)

print(f"\n✓ PR opened: {result['html_url']}")
print(f"  Number: #{result['number']}")
print(f"  State: {result['state']}")
print(f"  Head: {result['head']['ref']}")
print(f"  Base: {result['base']['ref']}")
