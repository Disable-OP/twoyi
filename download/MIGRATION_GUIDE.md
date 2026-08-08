# Migration Guide: Original twoyi → Disable-OP fork

This guide walks users of the original **[`twoyi/twoyi`](https://github.com/twoyi/twoyi)**
repository (archived April 2023 with the maintainer's note *"the project has
been discontinued"*) through migrating to the actively maintained fork at
**[`Disable-OP/twoyi`](https://github.com/Disable-OP/twoyi)** (branch `main` —
the only branch; the historical `improvements/initial-cleanup` branch has
been merged in and deleted).

The fork preserves the original project's design and history (all 207
upstream commits are intact) and adds 80+ new commits on top that
modernise the toolchain, replace closed-source blobs with open-source
rebuilds, and lay the groundwork for Virtual-Master-parity features.

> ℹ️ **Scope.** This is a **build-and-install migration guide**, not a
> feature-parity claim. End-to-end guest boot on x86_64 is still blocked
> on an x86_64 rootfs (see §6 Troubleshooting). On arm64 hardware the
> fork is a near drop-in replacement for the original.

---

## 1. Why migrate?

| Reason | What you get | Original twoyi |
|---|---|---|
| **Active development** | Improvements, bug fixes, PRs merged. | Archived since April 2023. |
| **x86_64 ABI support** | Same APK runs in emulators, redroid containers, and Intel Chromebooks — not just arm64 phones. | arm64-v8a only. |
| **Open-source renderer** | `libOpenglRender.so` rebuilt from AOSP `emugl` source (Apache-2.0), ~603 KB per ABI. The 1.06 MB closed-source arm64-only blob is gone. | Closed-source blob, arm64-only. |
| **Open-source loader** | `libloader.so` replaced by the Rust crate at `app/rs/loader/`. | 51 KB closed-source blob. |
| **Work profile support** | App works inside a managed/work profile — no more hardcoded `/data/data/io.twoyi` paths. | Broken inside work profiles. |
| **`kr64` daemon** | Skeleton Rust port of Virtual Master's `libkr64.so`: virtual `/dev` tree, seccomp filter with `SIGSYS` handler, `/proc` emulator, mount-namespace setup. 26+ unit tests passing. | No equivalent — guest runs with whatever the host kernel exposes. |
| **x86_64 SIGABRT fix** | The new Rust renderer is auto-selected on non-aarch64 hosts, killing the `surfaceChanged → renderer_reset_window → SIGABRT` tombstone. | x86_64 build crashed on startup. |
| **CI / devcontainer** | Matrix GitHub Actions workflow builds both ABIs; one-click GitHub Codespace with KVM, all toolchains pre-installed. | Manual build only. |
| **Signed APKs out of the box** | A test keystore is wired in so `./gradlew assembleRelease` produces an installable APK immediately. | Unsigned; you had to bring your own keystore. |
| **Input handling fixes** | `send_key_code()` now honours its keycode argument (was hardcoded to `KEY_BACK`) and maps HOME/BACK/VOLUME_*/POWER/MENU/SEARCH/APP_SWITCH. | Volume/Home/Power keys didn't work. |
| **Documentation** | `README.md`, `ARCHITECTURE.md` (1,324 lines), `CONTRIBUTING.md`, `CHANGELOG.md`, 13 analysis reports under `download/`. | Sparse; declared discontinued. |

The full list of changes is in [`CHANGELOG.md`](../CHANGELOG.md).

---

## 2. Prerequisites

| Tool | Version | Notes |
|---|---|---|
| **JDK** | **17** | Temurin or OpenJDK. JDK 8/11 will *not* work — the build uses JDK 17 method-resolution rules. |
| **Android SDK** | API 31, build-tools 30.0.3 | `compileSdkVersion 31`, `targetSdk 28`. |
| **Android NDK** | **r27c** (not r22!) | The original README's *"use NDK v22 or lower"* warning is **obsolete** — the fork's AOSP-source `libOpenglRender.so` is built with NDK r27c / clang 18 and the Rust crates target the same NDK. |
| **Rust** | stable | With `aarch64-linux-android` and `x86_64-linux-android` targets (`rustup target add …`). |
| **`cargo-xdk`** | latest | `cargo install cargo-xdk`. Wraps `cargo build` for Android ABIs. |
| **Android Studio** | any recent | Optional — Gradle from the CLI works fine. |
| **Device** | arm64 phone, or x86_64 emulator/redroid | For end-to-end testing. |

### Easiest path: GitHub Codespace

Skip the local install entirely. Open the repo on GitHub → click **Code →
Codespaces → Create**. The devcontainer (`.devcontainer/Dockerfile`)
pre-installs JDK 17, Rust + Android targets, NDK r27c, the Android SDK,
QEMU/KVM, and Docker, on Ubuntu 22.04 glibc. Pick the `standardLinux32gb`
machine size.

---

## 3. Migration steps

### 3.1 Clone the fork

```bash
git clone https://github.com/Disable-OP/twoyi.git
cd twoyi
git checkout main   # the only branch (round 68: improvements/initial-cleanup merged in and deleted)
```

> Round 68 note: `main` is now both the dev branch AND the release branch.
The historical `improvements/initial-cleanup` branch has been merged in
and deleted from origin.

### 3.2 Build the APK

#### Option A — Both ABIs (fat APK, matches CI)

```bash
./gradlew assembleRelease -Pabis=all
# → app/build/outputs/apk/release/twoyi_<version>.apk
```

#### Option B — Single ABI (faster local iteration)

```bash
./gradlew assembleRelease -Pabis=arm64-v8a   # real devices
./gradlew assembleRelease -Pabis=x86_64      # emulators / redroid
```

#### Option C — Rust side only

```bash
cd app/rs
sh build_rs.sh --release              # arm64-v8a (default)
sh build_rs.sh --release all          # both ABIs
sh build_rs.sh --release arm64-v8a x86_64
```

This drops `libtwoyi.so` into `app/src/main/jniLibs/<abi>/`, after which
a normal `./gradlew assembleRelease` will pick it up.

### 3.3 Install on device

```bash
adb install -r app/build/outputs/apk/release/*.apk
adb shell am start -n io.twoyi/.ui.SettingsActivity
```

The committed `app/twoyi-release.keystore` is a **self-signed RSA-2048
test key** (password `twoyi-release`), so the APK installs without any
keystore setup. If you are distributing a real release, replace it
first:

```bash
keytool -genkeypair -v -keystore app/twoyi-release.keystore \
  -alias twoyi-release -keyalg RSA -keysize 2048 -validity 10000
```

then update `storePassword` / `keyPassword` in `app/build.gradle` →
`signingConfigs.release`.

### 3.4 Import an existing rootfs

The APK ships a **placeholder asset** — to actually boot the guest you
need a real `rootfs.tar.gz`. You have two options:

#### Option 1 — Bundle into the APK at build time

```bash
curl -L -o app/src/main/assets/rootfs.tar.gz \
  https://github.com/cyanmint/twoyi/releases/download/original/rootfs.tar.gz
./gradlew assembleRelease -Pabis=arm64-v8a
```

This is the recommended path if you're distributing the APK to others.

#### Option 2 — Push to a running device and use the in-app "Import ROM" flow

1. Boot the freshly-installed APK on the device.
2. Open **Settings → Select ROM** (or the equivalent import entry).
3. The app opens a system file picker. Choose your `rootfs.tar.gz`.
4. The app copies the file to its cache, then runs
   `tar -xf rootfs_import.tar -C <rootfsDir>` to extract it into the
   active profile's rootfs directory (`<getDataDir()>/rootfs/`).
5. On success you'll see *"ROM imported successfully. Please reboot."*
6. Reboot the container (kill the app and relaunch, or tap
   **Launch Container** again).

> ⚠️ **Work-profile users.** Because the data directory is now resolved
> at runtime via `Context.getDataDir()`, the rootfs lands at
> `/data/user/<uid>/io.twoyi/rootfs/` inside a work profile instead of
> the legacy `/data/data/io.twoyi/rootfs/`. This is the headline
> behaviour change — see §4.1.

#### Reusing your existing rootfs

If you previously ran the original twoyi and have a working rootfs at
`/data/data/io.twoyi/rootfs/`, you can:

1. `adb shell` to the device, `su` if rooted, then `tar` up the existing
   rootfs directory:
   ```bash
   adb shell "cd /data/data/io.twoyi && tar -czf /sdcard/rootfs.tar.gz rootfs"
   adb pull /sdcard/rootfs.tar.gz
   ```
2. Use Option 2 above to import it into the fork.

The fork is **binary-compatible** with the original cyanmint rootfs —
no reformatting is needed. The rootfs layout (`init` at root, `system/`,
`vendor/`, `data/`, `dev/`, `sdcard/`, `rom.ini`) is unchanged.

---

## 4. What's different

Behavioural changes you'll notice when switching from the original twoyi.

### 4.1 Dynamic data directory (work profile support)

The biggest user-visible change. The original twoyi hardcoded
`/data/data/io.twoyi` in 8 places across `core.rs`, `input.rs`,
`socket_monitor.rs`, and three Java files. The fork replaces all of them
with a runtime-resolved directory obtained from `Context.getDataDir()`
via a new `setDataDir(String)` JNI function.

What this means in practice:

| Scenario | Original twoyi | Fork |
|---|---|---|
| Normal install | `/data/data/io.twoyi/rootfs/` | `/data/data/io.twoyi/rootfs/` (same) |
| Work profile install | **Broken** — app tries to read `/data/data/io.twoyi/...` which doesn't exist in the profile's namespace. | `/data/user/<work_uid>/io.twoyi/rootfs/` (works). |
| `TWOYI_ROOTFS` env var | Not set. | Exported into the guest's environment by `core.rs::init_renderer()`, so the AOSP-built `libOpenglRender.so` can find the `opengles{,2,3}` Unix sockets inside the active profile's rootfs. |

The fallback path (`get_data_dir()` returns `/data/data/io.twoyi` when
`setDataDir` was never called) preserves backwards compatibility, so
older integrations that don't call `setDataDir` keep working.

### 4.2 New renderer default on x86_64

On `aarch64` hosts the renderer selection is unchanged (defaults to the
legacy `libOpenglRender.so` blob unless you toggle "Use new renderer" in
Settings). On **non-`aarch64`** hosts (x86_64 emulators, redroid, Intel
hardware) the new Rust renderer is now the default — both at the Java
level (`ProfileSettings.useNewRenderer()` returns `true`) and as
defence-in-depth in Rust (`core.rs::effective_renderer_type()` forces
`RendererType::New`).

If you were previously overriding the renderer in your profile
preferences, the override is still respected on arm64; on x86_64 the
override is silently ignored (the new renderer is the only option
that doesn't `abort()`).

### 4.3 APK signing with a test keystore

The original twoyi shipped **unsigned** release builds — you had to
bring your own keystore or the APK wouldn't install
(`INSTALL_PARSE_FAILED_NO_CERTIFICATES`).

The fork commits a self-signed RSA-2048 test keystore
(`app/twoyi-release.keystore`, password `twoyi-release`) and wires it
into `signingConfigs.release`. Consequences:

- **CI and codespace builds produce installable APKs out of the box.**
- **The signing key is the same for everyone who builds from the public
  repo.** This is fine for testing but means anyone can sign a malicious
  APK that will install *over* yours. **Replace the keystore before
  publishing a release** (see §3.3).
- Android will refuse to install the fork's APK *over* an APK signed
  with a different key (e.g. the original twoyi). **Uninstall the
  original first** — your data won't survive the uninstall, so back up
  your rootfs first (see §3.4).

### 4.4 CI/CD integration

The original twoyi had no CI. The fork ships:

- **`.github/workflows/build.yml`** — matrix workflow that builds
  `arm64-v8a` and `x86_64` APKs on every push to `main` and
  on PRs. Has `workflow_dispatch` inputs for ABI selection and
  `include_rootfs`.
- **`.github/workflows/kr64-tests.yml`** — runs `cargo test` in
  `app/rs/kr64/` on every push to `main`. Uploads the test
  log and binaries as a 14-day-retention artifact.

If you fork the repo, both workflows run automatically on your fork.
There is no need to configure any GitHub Actions secrets — the test
keystore is committed.

### 4.5 NDK version

The original README warned *"use NDK v22 or lower"*. That warning is
**obsolete** — it existed because the closed-source `libOpenglRender.so`
blob was built against an old NDK ABI. The fork's open-source rebuild
is built with **NDK r27c / clang 18**, and the Rust crates target the
same NDK. Do not downgrade your NDK.

---

## 5. What's new

Features the original twoyi never had.

### 5.1 `kr64` kernel-replacement daemon

A Rust reimplementation of Virtual Master's `libkr64.so`, located at
`app/rs/kr64/`. It is the foundation for GSI boot (Roadmap item #1).

What the skeleton does today:

- **Virtual `/dev` tree** — materialises `qemu_pipe`, `touch`, `key0`,
  `gb`, `gb2`, `event` via `UnixListener::bind` in the guest's
  namespace. These are the devices twoyi's host side expects to find.
- **Seccomp filter** — ~60 syscalls allowed, ~15 dangerous ones
  blocked (`ptrace`, `perf_event_open`, `kexec_load`, `swap*`, etc.).
  A `SIGSYS` handler traps blocked calls for future emulation.
- **`/proc` emulator** — `version`, `cpuinfo`, `meminfo`, `self/`
  populated with values matching what the guest kernel would have
  reported.
- **Mount namespace** — `pivot_root` + tmpfs mounts isolate the guest's
  filesystem view.
- **Guest exec** — `exec`s the guest `init` from the rootfs.

Builds as both a `cdylib` (`libkr64.so`, directly executable via a
`.interp` PIE trick) and an `rlib`+`bin` (`kr64`) for host testing.
26+ unit tests passing. Follow-ups tracked in
[`download/KR64_SKELETON.md`](KR64_SKELETON.md).

### 5.2 AOSP-built `libOpenglRender.so`

The first-ever open-source build of twoyi's OpenGL renderer. Built from
AOSP `platform/sdk` commit `7a712ac` (Apache-2.0 `emugl/renderer`)
using NDK r27c / clang 18 / cmake 3.22.

- All 6 twoyi-required C-ABI functions exported and verified on both
  ABIs: `startOpenGLRenderer`, `destroyOpenGLSubwindow`,
  `repaintOpenGLDisplay`, `setNativeWindow`, `resetSubWindow`,
  `removeSubWindow`.
- `startGBServer`, `GraphicBuffer`, and the `dl*_ex` family ported back
  from the legacy blob (the three pieces the function-level comparison
  identified as missing from a stock AOSP build).
- Replaces the 1,059,128-byte closed-source blob with a ~603 KB
  arm64 / ~597 KB x86_64 build.
- Build process and provenance documented in
  [`download/AOSP_BUILD_RESULTS.md`](AOSP_BUILD_RESULTS.md).

### 5.3 Audio / sensor / battery HAL skeletons

Three new HAL modules in `app/rs/kr64/src/`, each following the same
`XxxDevice` + `XxxHandle` + `Drop` pattern with a `ThreadPool` for
connection handling. Each ships with comprehensive unit tests
(`tmpdir()` test helper, `JniObject = *mut c_void` type alias).

| HAL | Module | Tests | Wire protocol |
|---|---|---|---|
| Audio | `audio.rs` | 19 | Socket (`/dev/audio`-style), `AudioTrack`/`AudioRecord` up-calls to Java. |
| Sensor | `sensors.rs` | 60 | Socket (`/dev/sensors`), 12-entry guest-idx → `Sensor.TYPE_*` mapping. |
| Battery | `battery.rs` | 19 | File-based `/sys/class/power_supply/battery/` tree (guest polls; no socket). |

The Java side is **not yet implemented** — the Rust skeletons are ready
to receive a `HALManager.java` dispatcher with per-VM `nativePtr`s and
~30 JNI callback methods (modelled on VM's `HALManager`). See
[`download/HAL_VIRTUALIZATION_ANALYSIS.md`](HAL_VIRTUALIZATION_ANALYSIS.md)
for the full plan.

### 5.4 Binder virtualisation skeleton

The "hardest piece" of the GSI boot plan
([`download/GSI_BOOT_PLAN.md`](GSI_BOOT_PLAN.md) §3.2). Lives at
`app/rs/kr64/src/binder.rs` and ships the protocol constants and
plumbing needed by the next task (`BINDER-3`) to fill in parcel
parsing and handle translation.

- `_IOC` / `_IO` / `_IOR` / `_IOW` / `_IOWR` as `const fn`, matching
  `<asm-generic/ioctl.h>`.
- Kernel ABI structs (`BinderWriteRead`, `BinderPtrCookie`,
  `BinderTransactionData`, `FlatBinderObject`, etc.) — all
  `#[repr(C)]`, sizes verified by tests.
- All `BINDER_*` ioctl numbers (`BINDER_WRITE_READ`, `SET_MAX_THREADS`,
  `SET_CONTEXT_MGR`, `THREAD_EXIT`, `VERSION`, …).
- Device creation (`/dev/binder`, `/dev/vndbinder`, `/dev/hwbinder`)
  via the existing `devices.rs::bind_unix_socket` template.

See [`download/BINDER_SKELETON.md`](BINDER_SKELETON.md) for the full
design and the remaining work items.

---

## 6. Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `INSTALL_PARSE_FAILED_NO_CERTIFICATES` | You're trying to install an unsigned APK from a non-fork build. | Use `./gradlew assembleRelease` — the test keystore is wired in. |
| `INSTALL_FAILED_UPDATE_INCOMPATIBLE` | The fork's APK is signed with a different key than the original twoyi you have installed. | `adb uninstall io.twoyi` first (after backing up your rootfs — see §3.4). |
| `INSTALL_FAILED_VERIFICATION_FAILURE` | Play Protect blocking the test-signed APK. | Disable Play Protect for the install, or replace the keystore with your own. |
| App crashes immediately on x86_64 with `SIGABRT` in `surfaceChanged`. | You're running an old build of the fork before commit `7664c66`. | Rebuild from `main` — the new renderer is now the default on x86_64. |
| Boot stalls at *"Failed to write to pipe: Invalid argument (os error 22)"*. | The bundled rootfs is arm64-only; on x86_64 the guest `init` is an aarch64 ELF and can't execute. | Build or vendor an x86_64 rootfs — see [`X86_64_ROOTFS_BUILD_GUIDE.md`](X86_64_ROOTFS_BUILD_GUIDE.md). On arm64 this error means the rootfs is missing — see §3.4. |
| `adb shell am start -n io.twoyi/.ui.SettingsActivity` does nothing. | The activity moved or the APK didn't install. | `adb shell pm list packages \| grep twoyi` to confirm; if absent, `adb install -r` again. |
| Work-profile install: app says "ROM not found" after importing. | The `TWOYI_ROOTFS` env var or `setDataDir` JNI wasn't called. | Make sure you're on a build from `9c4b907` or later. `Render2Activity` calls `Renderer.setDataDir(getDataDir())` before `Renderer.init()`. |
| `cargo xdk` fails with `linker not found`. | NDK r27c not on PATH, or `cargo-xdk` not installed. | `cargo install cargo-xdk`; ensure `$ANDROID_NDK_HOME` points at r27c; `rustup target add aarch64-linux-android x86_64-linux-android`. |
| Build error: `libOpenglRender.so is incompatible with elf_x86_64`. | Stale `app/src/main/jniLibs/arm64-v8a/libOpenglRender.so` left over from an arm64-only build. | `rm -rf app/src/main/jniLibs/` and rebuild with `-Pabis=all` (commit `2085938` fixes this in CI). |
| `build_rs.sh: Syntax error: "(" unexpected`. | You're running an old checkout where the build scripts used bash arrays but were invoked with `sh`. | Pull `d2cfb8d` or later — scripts are now POSIX-sh compatible. |
| `reference to submit is ambiguous` (JDK 17 compile error). | Old checkout before `719a0db`. | Pull latest `main`. |
| Original twoyi ran fine; fork's arm64 build also runs fine but rootfs is empty. | The fork doesn't bundle the rootfs in the APK by default — only a placeholder. | Drop a real `rootfs.tar.gz` into `app/src/main/assets/` before building, or use the in-app Import ROM flow (§3.4). |
| `adb -s localhost:22122` timeout after boot. | Guest `adbd` not running, or the rootfs doesn't include the twoyi `init` wrapper that starts `adbd`. | Confirm `init` is at the rootfs root (not `/system/bin/init`); see [`X86_64_ROOTFS_BUILD_GUIDE.md`](X86_64_ROOTFS_BUILD_GUIDE.md) §3.8.1. |

For deeper status (what's verified working vs. theoretical), see
[`TWOYI_HONEST_STATUS.md`](TWOYI_HONEST_STATUS.md).

---

## 7. Rollback

Need to go back to the original twoyi? The fork doesn't modify any
shared state outside the app's own data directory, so rollback is
straightforward.

### 7.1 Back up your rootfs first

```bash
# On the device, with the fork still installed:
adb shell "cd /data/data/io.twoyi 2>/dev/null && tar -czf /sdcard/rootfs-backup.tar.gz rootfs" \
  || adb shell "cd /data/user/10/io.twoyi 2>/dev/null && tar -czf /sdcard/rootfs-backup.tar.gz rootfs"
adb pull /sdcard/rootfs-backup.tar.gz
```

(The `10` is the work-profile user ID; adjust if your profile is a
different number.)

### 7.2 Uninstall the fork

```bash
adb uninstall io.twoyi
```

This wipes the app's data directory, including the rootfs — that's why
§7.1 comes first.

### 7.3 Install the original twoyi

Build from the archived `twoyi/twoyi` repo, or download a pre-built
APK from its releases. The original requires **NDK r22 or lower** to
build — if you no longer have r22 around, the easiest path is to find
a pre-built release APK.

```bash
adb install original-twoyi.apk
adb shell am start -n io.twoyi/.ui.SettingsActivity
```

### 7.4 Restore your rootfs

```bash
adb push rootfs-backup.tar.gz /sdcard/
# Then use the original twoyi's ROM-import flow, or:
adb shell "su -c 'cd /data/data/io.twoyi && tar -xzf /sdcard/rootfs-backup.tar.gz'"
```

The rootfs layout is identical between the original and the fork — no
reformatting is needed.

### 7.5 Known rollback caveats

- **Signing key.** The original twoyi was either unsigned or signed
  with whoever built it. You can't install the original over the fork
  (or vice versa) without uninstalling first.
- **Renderer.** If you toggled "Use new renderer" in the fork's
  settings, that preference is wiped on uninstall — the original
  always uses the legacy blob.
- **Profiles.** The fork's profile manager (`ProfileManager`) is
  compatible with the original's profile layout, but if you created
  profiles only the fork supports (e.g. work-profile-rooted profiles),
  they won't survive rollback. Back up `app_kv` if you care:
  ```bash
  adb shell "cat /data/data/io.twoyi/shared_prefs/*.xml"
  ```
- **`kr64` daemon.** The original twoyi has no `kr64` — it relies
  entirely on the host kernel. Any guest behaviour that depended on
  the seccomp filter, `/proc` emulation, or the virtual `/dev` tree
  will behave differently (often worse) on the original.

If rollback was due to a regression in the fork, please
[file an issue](https://github.com/Disable-OP/twoyi/issues) with the
exact symptom, the fork's commit hash (`git rev-parse HEAD`), and the
device/emulator details. Most known regressions are already listed in
[`CHANGELOG.md`](../CHANGELOG.md) under **Fixed**.

---

## See also

- [`README.md`](../README.md) — project overview, quick start, build.
- [`ARCHITECTURE.md`](../ARCHITECTURE.md) — 1,324-line deep dive into
  the three-layer architecture.
- [`CONTRIBUTING.md`](../CONTRIBUTING.md) — dev setup, code style,
  PR process.
- [`CHANGELOG.md`](../CHANGELOG.md) — every commit on the fork,
  grouped by Added / Changed / Fixed / Removed / Security.
- [`TWOYI_HONEST_STATUS.md`](TWOYI_HONEST_STATUS.md) — what's verified
  working vs. theoretical, especially for x86_64.
- [`X86_64_ROOTFS_BUILD_GUIDE.md`](X86_64_ROOTFS_BUILD_GUIDE.md) — how
  to build an x86_64 rootfs from AOSP source or from a pre-built GSI.
- [`KR64_SKELETON.md`](KR64_SKELETON.md) — `kr64` daemon design and
  follow-up work items.
- [`BINDER_SKELETON.md`](BINDER_SKELETON.md) — binder virtualisation
  design.
- [`HAL_VIRTUALIZATION_ANALYSIS.md`](HAL_VIRTUALIZATION_ANALYSIS.md) —
  the 10-HAL porting plan modelled on Virtual Master.
