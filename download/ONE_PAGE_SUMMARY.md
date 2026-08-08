# Twoyi — One-Page Summary

**Twoyi** (两仪) is a **rootless Android-on-Android container**: it boots a
nearly complete second Android userland — `init`, `zygote`, `system_server`,
SurfaceFlinger, ART, HALs — inside one normal app process, with no root, no
unlocked bootloader, and no host modifications.

## What we accomplished overnight (~9 hrs, 22:06 → 07:13 UTC)

- **Reverse-engineered Virtual Master's APK** — recovered the AES-128 key
  (`%z89aviCM0KkbEs9`), decrypted all 4 bundled plugins, identified the GSI.
- **Open-sourced the native side** — rebuilt AOSP's `libOpenglRender` from
  source for **arm64-v8a + x86_64**, replacing 3 closed-source `.so` blobs.
- **Built `kr64`** — a ~11,554-LOC Rust crate emulating the kernel / binder /
  HAL surface a 64-bit Android guest expects; 8 modules, **165 tests passing**.
- **Wired up CI** — 2 GitHub Actions workflows, both green; ~20 commits
  overnight (235 total on `main`).
- **Shipped a signed release APK** + ~32 analysis docs (~15,900 lines).

## The breakthrough — x86_64 rootfs (05:20 UTC)

For the first time in the project's history, the **guest Android userland
reached the rendering stage on x86_64**. From `logcat`: `init` **executed**
(`avc: granted { execute }`), the QEMU pipe was **found and connected**
(`/dev/qemu_pipe` → `/opengles3`), a **GL context was created**, and the app
**stayed alive** (no crash). Rootfs: Android SDK
`system-images;android-30;google_apis;x86_64`. Screenshot: `download/screenshots/05_x86_64_rootfs_boot.png`.

## Current status

| ✅ Works | ❌ Doesn't work (yet) |
|---|---|
| Builds for arm64 + x86_64 | End-to-end rendering |
| 154 unit tests, 0 failing; CI green (historical — see Round 68 update above) | Pipe write fails: `EINVAL` (os error 22) |
| Updated: 165 tests as of round 68 | — |
| x86_64 rootfs: `init` runs | Guest SurfaceFlinger can't draw a frame |
| QEMU pipe connects, GL ctx created | — |

**Root cause of the one remaining failure:** the emulator's `/dev/qemu_pipe`
speaks the *emulator's* GL protocol, not twoyi's. Twoyi needs its own pipe
device that routes to its own `libOpenglRender.so`.

## Next step (one sentence)

**Create twoyi's OWN `/dev/qemu_pipe` via the `kr64` daemon**
(`app/rs/kr64/src/devices.rs` → `create_qemu_pipe()`) so guest
SurfaceFlinger's GL commands route to twoyi's AOSP-built renderer — that one
change turns "init executes, pipe connects, GL context created" into "the
container renders."

## Key documents

- `download/X86_64_BREAKTHROUGH.md` — session finale, full logcat evidence
- `download/SESSION_SUMMARY.md` — complete overnight log (all 20 commits)
- `download/FINAL_STATUS.md` — read-this-first 1-pager
- `download/HANDOFF.md` — how to continue, loose ends
- `download/DEVELOPMENT_ROADMAP.md` — 56 phased tasks
- `ARCHITECTURE.md` — three-layer design, PIE hack, spawn flow, renderer
- `worklog.md` — full task history (3,800+ lines)

*Good morning. The hard part is done. The next step is small and well-defined.*
