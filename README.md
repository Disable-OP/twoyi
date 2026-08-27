<div align="center">
  <h1>Twoyi</h1>
  <p><b>A rootless Android-on-Android container — actively developed fork</b></p>

  <p>
    <img alt="Status: active development"
         src="https://img.shields.io/badge/status-active%20development-success" />
    <img alt="License: MPL-2.0"
         src="https://img.shields.io/badge/license-MPL--2.0-blue.svg" />
    <img alt="ABIs: arm64-v8a + x86_64"
         src="https://img.shields.io/badge/ABIs-arm64--v8a%20%7C%20x86__64-orange" />
    <img alt="TWRP arm64 E2E: PASSING"
         src="https://img.shields.io/badge/TWRP%20arm64%20E2E-passing-brightgreen" />
    <img alt="Rust toolchain: stable"
         src="https://img.shields.io/badge/Rust-stable-orange?logo=rust" />
    <img alt="Java: 17"
         src="https://img.shields.io/badge/Java-17-orange?logo=openjdk" />
    <a href="https://github.com/Disable-OP/twoyi/actions/workflows/build.yml">
      <img alt="GitHub Actions CI"
           src="https://github.com/Disable-OP/twoyi/actions/workflows/build.yml/badge.svg?branch=main" />
    </a>
  </p>

  <p>
    <sub>
      Originally created by
      <a href="https://github.com/tiann">weishu</a> · Active fork maintained by
      <a href="https://github.com/Disable-OP">Disable-OP</a> and contributors
    </sub>
  </p>
</div>

---

## What is Twoyi?

Twoyi (Chinese: 两仪, *"two-yi"*) is a **rootless Android-on-Android container**.
It runs a nearly complete second Android userland — `init`, `zygote`,
`system_server`, framework JARs, ART runtime, HALs — **inside one normal Android
app process**, with no root, no unlocked bootloader, and no host modifications.

The trick: Android is, at the bottom, just a Linux process tree. Twoyi ships a
complete Android userland as a folder inside the app's private data directory,
then `exec`s `./init` from that folder the same way real Android does. The
guest shares the host kernel but has its own `system_server`, package manager,
SurfaceFlinger, and even its own `adbd`. Graphics are transported over a
QEMU-pipe-style Unix socket to an in-process OpenGL renderer.

**This fork adds a second act: booting real recovery images (TWRP) inside the
same jail** — on both `x86_64` and, as of the current HEAD, **`arm64`**, with
the TWRP menu rendered at native resolution and **working touch input**, all
verified end-to-end inside GitHub Actions.

---

## Current State (what actually works at HEAD)

| Path | Arch | Status | Evidence |
|---|---|---|---|
| **TWRP boot → menu, with touch, displayed in the app** | **arm64-v8a** | ✅ **WORKING E2E in CI** | `ui-e2e-test-arm64.yml` — boots TWRP 3.7.0-9 (angler) in a redroid container on `ubuntu-24.04-arm`, renders at native 720×1600, **displays the menu inside the app window with correct colors**, and drives it with real gestures through the input bridge (`Set page: clear_vars → main2` transitions in recovery.log). Proof — the app's own display as captured by framework screencap: [`screenshots/twrp-arm64-app-display-720x1600.png`](screenshots/twrp-arm64-app-display-720x1600.png) |
| **TWRP boot → menu, with touch, displayed in the app** | x86_64 | ✅ WORKING E2E in CI | `ui-e2e-test.yml` — the original byt_t_crv2 image path on the x86_64 emulator (KVM); TWRP rendered in the app window with the correct blue theme ([proof](screenshots/twrp-x86-app-display.png)) and driven by real gestures (`Set page: clear_vars → main2` in the `run-as` recovery.log pull) |
| **Full Android (AOSP ROM) guest** | x86_64 | ✅ WORKING (inherited baseline) | The original twoyi use case; see `docs/reference/ARCHITECTURE.md`. |
| **Full Android (AOSP ROM) guest** | arm64-v8a | 🟡 IN PROGRESS | Guest init boots through second-stage + services under the arm64 tracer; not yet a full framework boot. Tracked in [`worklog.md`](worklog.md) (6-series). |
| Host device requirement | arm64 phones | ✅ runs natively | The APK builds `arm64-v8a` + `x86_64`; on a real arm64 device the TWRP path is the same code CI exercises. |

### The arm64 TWRP boot chain (all green at HEAD)

App launch → ROM import → *Boot to Recovery* → **Launch Container** →
`kr64` jail (mount + PID namespaces, `pivot_root` with chroot fallback) →
TWRP `init` parses `init.rc` → `recovery` service starts with the
`libtwrp_fb_hook.so` preload → theme load → `gr_init` → **PixelFlinger renders
at native 720×1600 into the virtual fb0** → the app's render loop blits the
frame into the `Render2Activity` surface (RGBA_8888, live-window tracked —
the menu appears in the app window with correct colors within ~0.5 s of the
first frame).
Input: screen gestures → app `onTouch` → **abstract-namespace touch socket**
(`\0io.twoyi.touch`, chroot-proof) → hook bridge → synthesized evdev stream →
TWRP page navigation. CI proves every hop with artifacts: framework screencaps
of the app's own display, TWRP's `recovery.log` (page transitions = touch
proof), and the full tracer log. Heavy per-syscall tracer logging is OFF by
default (`TWOYI_TRACE_SYSCALLS=1` opts back in) so the container runs at full
speed.

### Known gaps (non-blocking)

- The container's own `adbd` dies shortly after the TWRP container starts.
  All CI evidence collection is `docker exec`-based now, so nothing is lost,
  but the root cause is still open.
- The arm64 AOSP-ROM guest (non-TWRP) is not yet a full boot — see the 6-series
  worklog entries for exactly where it stands and what is queued next.
- TWRP's internal actions that need real block devices (flashing, mounts of
  `/data`, `/system`) are answered with honest `-ENODEV`/synthetic nodes —
  deliberate: this is a container, not a device.

---

## This Fork

This is an **active fork** of the original
[`twoyi/twoyi`](https://github.com/twoyi/twoyi) repository, which was archived
in April 2023 with the maintainer's note *"Due to the complexity of the project
and lack of any revenue, the project has been discontinued."* The fork carries
the project forward: the closed-source reverse-engineered blobs
(`libvm.so`, `libkr64.so`) are reimplemented as the open-source Rust tracer
**`kr64`** (see `app/rs/kr64/`), the closed renderer is replaced by the
**AOSP `libOpenglRender.so`** build (`app/cpp/emugl/`), and the whole thing is
exercised by a redroid-based E2E matrix in GitHub Actions.

> 📖 Deep write-ups live in [`docs/reference/`](docs/reference/) —
> `ARCHITECTURE.md` (the code-level architecture map),
> `ARCHITECTURE_DECISIONS.md` (ADRs), the `docs_vm_*.md` reverse-engineering
> analyses of the original Virtual Master APK, and `TESTING_GUIDE.md`.
> The session-by-session engineering log is [`worklog.md`](worklog.md).

---

## How the TWRP mode works (short version)

1. **`kr64`** (`app/rs/kr64`, Rust, ~23k lines) forks the guest and becomes its
   `ptrace` tracer. It translates every syscall of the jailed child: paths are
   mapped into the rootfs, mounts/mknods are virtualized, identity syscalls are
   faked, `bind()` AF_UNIX sockaddrs are rewritten to per-pid paths under the
   rootfs (so two property-service hosts never `EADDRINUSE` each other), and a
   growing family of "honest lies" (root-owned stats, `/proc` synthesizers,
   capability masks) keeps old Android userlands happy — all without root.
2. **`libtwrp_fb_hook.so`** (`app/cpp/twoyi_loader/src/twrp_fb_hook.c`) is
   `LD_PRELOAD`ed into the recovery binary. It answers `FBIOGET_{V,F}SCREENINFO`
   with the real display geometry (native resolution, resolved via env →
   geometry file → fallback), fakes the evdev capability probes, and bridges
   touch: host gestures arrive on the abstract socket and are re-encoded as a
   `struct input_event` stream (correct per-arch layout) that minui consumes.
3. **The app** (`app/rs`, Kotlin + Rust) owns the surface: it reads the virtual
   `fb0` file the guest renders into, blits it into the `Render2Activity`
   surface, forwards `MotionEvent`s into the guest, and runs the in-process
   OpenGL renderer for the full-Android mode.

---

## Build & Run

```bash
# Android app (debuggable, both ABIs)
./gradlew assembleDebug

# Native only (host unit tests, fmt + clippy gates)
cd app/rs/kr64 && cargo test && cargo clippy --all-targets -- -D warnings
```

Install the APK (`app/build/outputs/apk/...`), open the app, import a ROM or a
recovery image, enable *Advanced → Boot to Recovery*, and tap
**Launch Container**. On an arm64 device the TWRP path is exactly what CI runs;
the menu appears in the app window and touch works.

## CI / Test Matrix

| Workflow | What it proves |
|---|---|
| `build.yml` | APK builds for `arm64-v8a` + `x86_64` on every push |
| `kr64-tests.yml` | `cargo fmt` + 560 unit tests + clippy `-D warnings` |
| `ui-e2e-test.yml` | x86_64 TWRP boots to its menu **displayed in the app** on the KVM emulator; pulls TWRP's `recovery.log` via `run-as` for the touch-page proof |
| `ui-e2e-test-arm64.yml` | **arm64 TWRP boots to its menu with touch on redroid** (`ubuntu-24.04-arm`); adb-independent evidence pipeline (docker-exec taps: app-display screencaps, TWRP recovery.log, tracer stderr) |
| `ui-e2e-aosp(-arm64).yml` | Full-Android guest E2E (x86_64 green; arm64 WIP) |

The arm64 TWRP workflow accepts inputs (`recovery_url`, `redroid_tag`,
`redroid_resolution`, `boot_wait_seconds`, `twrp_no_input` for input-bridge
A/B probes) so alternative recoveries (OrangeFox / SHRP / PitchBlack) can be
tested without code changes.

---

## Repository Layout

```
app/                    Android app (Kotlin) + native Rust/C++
  cpp/twoyi_loader/     the TWRP fb/input hook (twrp_fb_hook.c)
  cpp/emugl/            AOSP emugl renderer (open-source libOpenglRender)
  rs/kr64/              the ptrace syscall-translation jail (Rust)
  rs/src/               app-native core (renderer bindings, input, touch IPC)
.github/workflows/      build + test + the E2E matrix
docs/reference/         architecture docs + reverse-engineering analyses
screenshots/            E2E proof captures (incl. the arm64 TWRP menu)
worklog.md              the engineering log — start at the bottom
```

## Contributing & Security

See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`SECURITY.md`](SECURITY.md).
The coding conventions are in
[`docs/reference/CODE_STYLE_GUIDE.md`](docs/reference/CODE_STYLE_GUIDE.md).

## Credits

- **[weishu](https://github.com/tiann)** — the original twoyi (两仪).
- The Android Open Source Project — the emugl renderer and parts of the
  guest toolchain.
- Everyone who documented the original internals; the reverse-engineering
  notes in `docs/reference/docs_vm_*.md` credit the specific analyses.

## License

MPL-2.0 (see [LICENSE](LICENSE)). The fork's reimplementation work is
licensed like the original; individual files keep their upstream headers.
