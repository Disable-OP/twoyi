<div align="center">
  <h1>Twoyi</h1>
  <p><b>A rootless Android-on-Android container — actively developed fork</b></p>

  <!-- Badges -->
  <p>
    <img alt="Status: active development"
         src="https://img.shields.io/badge/status-active%20development-success" />
    <img alt="License: MPL-2.0"
         src="https://img.shields.io/badge/license-MPL--2.0-blue.svg" />
    <img alt="ABIs: arm64-v8a + x86_64"
         src="https://img.shields.io/badge/ABIs-arm64--v8a%20%7C%20x86__64-orange" />
    <img alt="Rust toolchain: stable"
         src="https://img.shields.io/badge/Rust-stable-orange?logo=rust" />
    <img alt="Java: 17"
         src="https://img.shields.io/badge/Java-17-orange?logo=openjdk" />
    <a href="https://github.com/Disable-OP/twoyi/actions/workflows/build.yml">
      <img alt="GitHub Actions CI"
           src="https://github.com/Disable-OP/twoyi/actions/workflows/build.yml/badge.svg?branch=improvements%2Finitial-cleanup" />
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

> 📖 **For the deep architectural write-up** (the three-layer architecture, the
> PIE hack in `app/rs/src/interp.c`, the guest spawn flow, the renderer
> pipeline), see **[ARCHITECTURE.md](ARCHITECTURE.md)**.

---

## This Fork

This is an **active fork** of the original
[`twoyi/twoyi`](https://github.com/twoyi/twoyi) repository, which was archived
in April 2023 with the maintainer's note *"Due to the complexity of the project
and lack of any revenue, the project has been discontinued."* The fork carries
the project forward and replaces several closed-source components with
open-source rebuilds.

Development happens on the **`improvements/initial-cleanup`** branch. Notable
improvements over the archived upstream:

| Improvement | What it does | Headline commit |
|---|---|---|
| **x86_64 ABI support** | Adds `x86_64` to `abiFilters` so the same APK runs in emulators and redroid containers, not just on arm64 hardware. | `84ece58` |
| **Open-source `libOpenglRender.so`** | Replaces the 1.06 MB closed-source arm64-only blob with a 605–611 KB build **compiled from AOSP emugl source** (`platform/sdk` commit `7a712acc`, Apache-2.0), for both `arm64-v8a` and `x86_64`. All 6 twoyi-required C-ABI symbols exported; `startGBServer`, `GraphicBuffer`, and `dl*_ex` ported back from the legacy blob. | `47f8335`, `eb13449` |
| **Open-source `libloader.so`** | Replaces the 51 KB closed-source loader blob with a Rust crate at `app/rs/loader/`. | `a33e8c5` |
| **Work profile support** | Replaces 8 hardcoded `/data/data/io.twoyi` paths with a runtime-resolved data dir (`Context.getDataDir()`) so the app works inside a work profile / managed profile. New `TWOYI_ROOTFS` env var. | `9c4b907` |
| **x86_64 SIGABRT fix** | Defaults to the new Rust renderer on non-aarch64 hosts, preventing the `surfaceChanged → renderer_reset_window → SIGABRT` tombstone that previously crashed the x86_64 build. | `7664c66` |
| **Kernel replacement daemon (`kr64`)** | Skeleton Rust port of Virtual Master's `libkr64.so` at `app/rs/kr64/`. Creates 6 virtual devices, installs a seccomp filter with a SIGSYS handler, emulates `/proc`, sets up a mount namespace, and exec's the guest `init`. 26 unit tests passing. | `570e95e` |
| **GitHub Actions CI** | Matrix workflow builds both `arm64-v8a` and `x86_64` APKs with `workflow_dispatch` ABI/rootfs inputs. | `93f5f1c` |
| **Codespace devcontainer** | Custom Ubuntu 22.04 Dockerfile + sshd feature. Pre-installs JDK 17, Rust + Android targets, NDK r27c, Android SDK, QEMU/KVM, Docker. Creates `/dev/kvm` via `mknod` when the codespace runs `--privileged`. | `3628519`, `a6e6dbb` |
| **Input handling** | `send_key_code()` now honours its keycode argument (was hardcoded to `KEY_BACK`) and advertises all supported keys; added `android_keycode_to_linux()` mapping for HOME/BACK/VOLUME_*/POWER/MENU/SEARCH/APP_SWITCH. | `7dc6093` |
| **Signed release APKs** | Self-signed RSA-2048 test keystore wired into Gradle so CI and codespace builds produce installable APKs. | `ff1cc37` |

The full 207-commit history is preserved:

```bash
cd /home/z/my-project && git log --oneline improvements/initial-cleanup
```

---

## Quick Start

The fastest path to a running APK is the GitHub Codespace — KVM-enabled, all
toolchains pre-installed.

```bash
# 1. Create a codespace from this repo (pick "standardLinux32gb" — 4c/16GB).
#    The devcontainer's postCreateCommand installs the SDK, NDK, and Rust targets.

# 2. Inside the codespace, build both ABIs:
./gradlew assembleRelease -Pabis=all

# 3. The signed APK lands at:
ls -lh app/build/outputs/apk/release/*.apk

# 4. Start a redroid x86_64 container and install the APK:
.devcontainer/scripts/run-redroid.sh
.devcontainer/scripts/test-twoyi.sh
```

For real arm64 hardware, see **[Building](#building)** below and copy the APK
to a physical device.

> ⚠️ **ROM note.** The APK ships a placeholder asset. To actually boot the
> guest you need a `rootfs.tar.gz` (cyanmint's `original` release works) —
> either drop it into `app/src/main/assets/` before building, or trigger the
> CI workflow with `include_rootfs: true`, or push it to the device and use
> the app's import flow. Booting a real Android **Treble GSI** is the next
> major milestone — see **[Roadmap](#roadmap)**.

---

## Architecture

Twoyi is a three-layer system:

```
┌──────────────────────────────────────────────────────────┐
│  Java app (io.twoyi)                                     │
│  Render2Activity · RomManager · ProfileManager ·         │
│  TwoyiSocketServer · TwoyiStatusManager                  │
└───────────────┬──────────────────────────────────────────┘
                │  JNI   (Renderer.java, setDataDir, init, …)
┌───────────────▼──────────────────────────────────────────┐
│  libtwoyi.so  (Rust, app/rs/)                            │
│  core.rs · input.rs · renderer_bindings.rs ·             │
│  renderer_new/ · interp.c (PIE hack)                     │
└───────────────┬──────────────────────────────────────────┘
                │  C ABI  (startOpenGLRenderer, setNativeWindow, …)
┌───────────────▼──────────────────────────────────────────┐
│  libOpenglRender.so  (C++, built from AOSP emugl source) │
│  FrameBuffer · ColorBuffer · GraphicBuffer ·             │
│  GLESv1/v2 decoders · renderControl · startGBServer      │
└──────────────────────────────────────────────────────────┘
                │  /dev/qemu_pipe  (Unix socket)
┌───────────────▼──────────────────────────────────────────┐
│  Guest Android userland  (rootfs/)                       │
│  init · zygote · system_server · SurfaceFlinger ·        │
│  servicemanager · adbd                                   │
└──────────────────────────────────────────────────────────┘
```

The Java app is the UI host. `libtwoyi.so` is a Rust crate compiled as a PIE
`cdylib` (directly executable AND JNI-loadable thanks to the `.interp` trick in
`app/rs/src/interp.c`). `libOpenglRender.so` is rebuilt from AOSP emugl source
and renders GLES commands received over the QEMU pipe into the host
`SurfaceView`.

> 📖 Full code-level walkthrough: **[ARCHITECTURE.md](ARCHITECTURE.md)**

---

## Building

### Prerequisites

| Tool | Version | Notes |
|---|---|---|
| JDK | 17 | Temurin or OpenJDK. |
| Android SDK | API 31, build-tools 30.0.3 | `compileSdkVersion 31`, `targetSdk 28`. |
| Android NDK | **r27c** | Matches CI. Newer may also work. |
| Rust | stable | With `aarch64-linux-android` and `x86_64-linux-android` targets. |
| `cargo-xdk` | latest | `cargo install cargo-xdk`. Wraps `cargo build` for Android ABIs. |
| Android Studio | any recent | Optional — Gradle from the CLI works fine. |

> The original README's "use NDK v22 or lower" warning is **obsolete** — this
> fork's AOSP-source `libOpenglRender.so` is built with NDK r27c / clang 18 and
> the Rust crates target the same NDK.

### Build both ABIs (default for CI)

```bash
./gradlew assembleRelease -Pabis=all
# → app/build/outputs/apk/release/twoyi_<version>.apk  (fat APK, both ABIs)
```

### Build a single ABI (faster local iteration)

```bash
./gradlew assembleRelease -Pabis=arm64-v8a   # real devices
./gradlew assembleRelease -Pabis=x86_64      # emulators / redroid
```

### Build the Rust side only

`./gradlew cargoBuild` invokes `app/rs/build_rs.sh`, which calls
`cargo xdk` for each requested ABI and drops `libtwoyi.so` into
`app/src/main/jniLibs/<abi>/`. You can run it directly:

```bash
cd app/rs
sh build_rs.sh --release              # arm64-v8a only (default)
sh build_rs.sh --release all          # both ABIs
sh build_rs.sh --release arm64-v8a x86_64
```

### Build the `kr64` kernel-replacement daemon

```bash
cd app/rs/kr64
cargo test                  # 26 unit tests, runs on Linux host
cargo build --release       # host binary for development
# For Android:
cargo build --target aarch64-linux-android --release
cargo build --target x86_64-linux-android --release
```

### Bundling a rootfs

The repo ships a placeholder asset. To get a bootable APK, drop a real rootfs
into the assets directory before building:

```bash
curl -L -o app/src/main/assets/rootfs.tar.gz \
  https://github.com/cyanmint/twoyi/releases/download/original/rootfs.tar.gz
```

Or push it to a running device and use the app's "Import ROM" flow.

### Replacing the test signing key

The committed `app/twoyi-release.keystore` is a **self-signed RSA-2048 test
key** (password `twoyi-release`). Replace it with your own before publishing:

```bash
keytool -genkeypair -v -keystore app/twoyi-release.keystore \
  -alias twoyi-release -keyalg RSA -keysize 2048 -validity 10000
```

---

## Testing

### Codespace with KVM (recommended for x86_64)

The devcontainer configures a `--privileged` Ubuntu 22.04 container with KVM
exposed. On GitHub's AMD EPYC VMs (EastUs region), nested virtualization works
end-to-end.

```bash
# Verify KVM is available:
.devcontainer/scripts/check-kvm.sh

# Start a redroid x86_64 container:
.devcontainer/scripts/run-redroid.sh

# Build + install + screenshot the APK in one shot:
.devcontainer/scripts/test-twoyi.sh
```

The screenshot harness deposits PNGs in `/tmp/twoyi-screenshots/` for visual
inspection or VLM analysis.

### Local emulator

```bash
# Create an x86_64 emulator (the devcontainer's setup.sh already does this):
sdkmanager "system-images;android-30;google_apis;x86_64"
avdmanager create avd -n twoyi_x86_64 -k "system-images;android-30;google_apis;x86_64" -d pixel_5
emulator -avd twoyi_x86_64 -no-window -no-audio &
adb wait-for-device
adb install -r -t app/build/outputs/apk/release/*.apk
adb shell am start -n io.twoyi/.ui.SettingsActivity
```

### Real arm64 device

```bash
adb install -r app/build/outputs/apk/release/*.apk
adb shell am start -n io.twoyi/.ui.SettingsActivity
# Tap "Launch Container" — the legacy arm64 rootfs + libOpenglRender.so path
# is the most complete end-to-end test path.
```

### Rust unit tests

```bash
cd app/rs/kr64 && cargo test           # 26 tests
cd app/rs      && cargo build          # smoke-build the host crate
```

> ℹ️ **Honest status of x86_64 boot.** The x86_64 build no longer SIGABRT-crashes
> and the new Rust renderer initializes (`GL context created successfully`), but
> the guest `init` in the bundled rootfs is arm64 — it can't execute on x86_64
> without a matching x86_64 rootfs. The codespace is therefore ideal for
> **building, signing, and installing** the APK and for testing the Rust crates,
> but end-to-end guest boot on x86_64 is blocked on building an x86_64 rootfs
> (Roadmap item 2). See `download/TWOYI_HONEST_STATUS.md` for the full
> verified-vs-theoretical breakdown.

---

## Roadmap

The headline goal is **booting a real Android Treble GSI** inside the
container, with per-VM isolation comparable to Virtual Master. The full
file-and-function-level plan lives in
[`download/GSI_BOOT_PLAN.md`](download/GSI_BOOT_PLAN.md); the
[`kr64` skeleton](app/rs/kr64/) is the first concrete step.

| # | Milestone | Status | Estimate |
|---|---|---|---|
| 1 | **Kernel replacement daemon (`kr64`)** — virtual `/dev` tree, seccomp filter, `/proc` emulator, mount namespace | 🟡 Skeleton done (6 devices, 26 tests); full implementation pending | weeks 1–2 remaining |
| 2 | **x86_64 rootfs from AOSP** — unblocks end-to-end x86_64 testing in the codespace | 🔴 Not started | 1–2 weeks |
| 3 | **GSI extractor** — sparse-ext4 → raw ext4 → directory tree, boot.img ramdisk extraction, `vendor.img` synthesis | 🔴 Not started | weeks 2–3 |
| 4 | **GSI init patcher** — patch `init.rc`, `build.prop`, VINTF manifests so the guest talks to virtual devices | 🔴 Not started | weeks 2–3 |
| 5 | **Graphics HAL** — `/dev/gb` + `/dev/gb2`, gralloc allocator/mapper/composer, extend `GraphicBuffer::Main` to register buffers with `FrameBuffer` | 🔴 Stub only | weeks 3–5 |
| 6 | **Stub HALs → boot to launcher** — keymaster, health, power, vibrator stubs sufficient for `init` to complete | 🔴 Not started | weeks 5–6 |
| 7 | **`/proc` dynamic files + seccomp emulation** — intercept `open("/proc/self/maps")` etc., dispatch `mount`/`umount2`/`reboot` from the SIGSYS handler | 🟡 Skeleton only | weeks 6–8 |
| 8 | **Binder virtualization** — per-VM `/vm%d/dev/binder` + Java `IActivityManager` proxy. **Hardest piece.** MVP workaround: patch `system_server` to skip `publishService`. | 🔴 Not started | weeks 8–12 |
| 9 | **Full HAL proxies** — audio, camera, sensor, location, phone, network, bluetooth | 🔴 Not started | weeks 12+ |
| 10 | **Open-source `libadb.so`** — replace the 4.46 MB closed-source `adb` blob with a build from `packages/modules/adb` (Apache-2.0) | 🔴 Not started | 1 week |
| 11 | **Multi-VM support** — adopt Virtual Master's per-VM renderer pointer pattern (`DisplayService.nativeAddSurface(ptr, …)`) | 🔴 Not started | post-MVP |

**Rough estimates:** 8–12 weeks for an MVP that boots to launcher, 16–24 weeks
for full Virtual Master parity.

---

## Contributing

Contributions are welcome — see **[CONTRIBUTING.md](CONTRIBUTING.md)** for
development setup, code style, test expectations, the PR process, and a list of
areas that especially need help (most of the roadmap above is unstaffed).

If you want to dive in, the highest-leverage starting points are:

- **Roadmap item #2** — building an x86_64 rootfs from AOSP (unblocks all
  x86_64 end-to-end testing).
- **Roadmap item #5** — extending `GraphicBuffer::Main` in the AOSP-built
  renderer to register received buffers with `FrameBuffer` (unblocks
  SurfaceFlinger compositing).
- **Roadmap item #1 follow-ups** — full device inventory, binder
  virtualization, dynamic `/proc` files, `mknodat`-based socket creation.

---

## License

This Source Code Form is subject to the terms of the **Mozilla Public License,
v. 2.0**. If a copy of the MPL was not distributed with this file, you can
obtain one at <https://mozilla.org/MPL/2.0/>.

See [`LICENSE`](LICENSE) for the full text.

The AOSP-derived `libOpenglRender.so` source is licensed under
**Apache-2.0** (see `download/AOSP_BUILD_RESULTS.md` for provenance and the
upstream AOSP commit).

---

## Credits

- **[weishu](https://github.com/tiann)** — original author of Twoyi (and of
  Taichi / EdXposed). The original `twoyi/twoyi` repo is archived; this fork
  builds directly on weishu's design and codebase.
- **[cyanmint](https://github.com/cyanmint)** — maintained the only active
  continuation of twoyi for the 8 months before this fork; contributed the
  open-source Rust loader, profile manager, ROM manifest, and many boot-fix
  patches that this branch is built on top of.
- **[Disable-OP](https://github.com/Disable-OP)** — current fork maintainer.
- **Contributors** — see the
  [contributors graph](https://github.com/Disable-OP/twoyi/graphs/contributors).

### Analysis credit

The reverse-engineering of Virtual Master and the AOSP-source rebuild of
`libOpenglRender.so` that informed this fork's direction are documented in the
`download/` directory (13 analysis reports, ~4,000 lines total). See
[`download/PROJECT_SUMMARY.md`](download/PROJECT_SUMMARY.md) for the definitive
state-of-the-project write-up, and [`worklog.md`](worklog.md) for the
sub-agent task history.
