# Twoyi — Testing Guide

> How to test twoyi at every level: host unit tests, CI, Android
> emulator (with KVM), real arm64 devices, end-to-end boot
> verification (logcat + screenshot + VLM), performance, and where to
> find or build the test data.
>
> **Related:** [`QUICK_START.md`](QUICK_START.md),
> [`ARCHITECTURE.md`](../ARCHITECTURE.md),
> [`CODE_STYLE_GUIDE.md`](CODE_STYLE_GUIDE.md),
> [`TWOYI_HONEST_STATUS.md`](TWOYI_HONEST_STATUS.md) (verified vs.
> theoretical — read before trusting any "it works" claim).

---

## 1. Unit tests — the `kr64` crate

The kernel-replacement daemon (`app/rs/kr64/`) is the only Rust crate
that is fully host-testable: its sole dependency is `libc`, and its
`build.rs` (which compiles `interp.c`) works on any platform with a C
compiler. **144 tests across 8 submodules plus the lib root** run on
plain Linux/x86_64 — no NDK, no device.

```bash
cd app/rs/kr64
cargo test --no-fail-fast     # CI form — reports every failure
cargo test binder             # one module only
cargo test -- --nocapture     # show eprintln! from passing tests
./gradlew test                # Java JVM unit tests (app/src/test/java/io/twoyi/)
./gradlew connectedAndroidTest # instrumented (app/src/androidTest/)
```

### Per-module breakdown

| Module | # | What it covers |
|---|---:|---|
| `lib.rs` | 7 | `Config::default`, `parse_args` (minimal/full/missing `--rootfs`/missing `--data-dir`/unknown/`--help`) |
| `devices.rs` | 3 | `create_qemu_pipe` socket + FD, `create_all_devices` creates all 6 sockets, marker files |
| `binder.rs` | 12 | ioctl/`BC_*`/`BR_*` constants match kernel, struct sizes (`binder_write_read`=48 B, `binder_transaction_data`=64 B, `flat_binder_object`=24 B), `HandleTable`, per-VM binder device, `BINDER_VERSION` roundtrip, `BINDER_WRITE_READ`→`BR_NOOP`, `ThreadPool` |
| `audio.rs` | 27 | `AudioHeader` 16-byte layout + magic/direction, playback/capture roundtrips, `create_audio_device` socket + stale replacement, spawn accepts/rejects, `Handle::shutdown` joins, `ThreadPool`, `read_exact` EOF |
| `sensors.rs` | 60 | `SensorEvent` 24-byte + 12-sensor index/type mapping matching VM wire format, `SensorState` bitflags, `SensorControl` 12-byte, spawn + per-cmd dispatch, multi-connection, `ConnState` cache, `ThreadPool` |
| `battery.rs` | 19 | `BatteryStatus` roundtrip + Linux ABI bytes, `BatteryDevice::new` creates 7 files in nested `/sys/class/power_supply/battery/`, idempotency, `update_*` clamps, `refresh` from JNI stubs, `spawn` background refresh |
| `seccomp.rs` | 7 | BPF builds without panic, `allowed`/`trapped`/`killed` syscall sets, `classify` returns `Emulate{retval:0}`/`Kill`/`Passthrough` correctly |
| `proc_emu.rs` | 5 | `populate_proc` creates all files, `/proc/version`/`cmdline`/`meminfo`/`cpuinfo` content |
| `mount_mgr.rs` | 4 | `MountSpec::default`, `unshare(CLONE_NEWUSER)` works, `list_mounts`, `pivot_root` wrapper |
| **Total** | **144** | |

Tests use a per-process `AtomicU64`-indexed tmpdir helper (e.g.
`devices.rs:360-366`) so they run in parallel without colliding.

---

## 2. CI tests — GitHub Actions

Two workflows in [`.github/workflows/`](../.github/workflows/).

### 2.1 `build.yml` — APK build

- **Triggers:** push to `main`/`develop`/`improvements/**`, PRs to
  `main`/`develop`, manual `workflow_dispatch` with `abis`
  (`arm64-v8a`/`x86_64`/`all`) and `include_rootfs` boolean inputs.
- **Toolchain:** JDK 17 (temurin), Rust stable with both Android
  targets, NDK r27c, `cargo-xdk` (cached).
- **Rootfs:** placeholder assets by default. Tick `include_rootfs` to
  fetch the real ~275 MB `rootfs.tar.gz` from the `cyanmint/twoyi`
  `original` release into `app/src/main/assets/`.
- **Build:** `./gradlew assembleRelease -Pabis=<abis>`.
- **Artifacts:** APK (30-day); build logs on failure (7-day).

### 2.2 `kr64-tests.yml` — host unit tests

- **Triggers:** push to `improvements/**`, any PR, manual dispatch.
  Concurrency group `kr64-tests-${{ github.ref }}` with
  `cancel-in-progress: true`.
- **Toolchain:** Rust stable + `rustfmt` + `clippy`, host linux/x86_64
  (no Android targets needed).
- **Run:** `cd app/rs/kr64 && cargo test --no-fail-fast 2>&1 | tee cargo-test.log`
  — `--no-fail-fast` surfaces every failure; GHA's `set -eo pipefail`
  propagates the exit code through the pipe.
- **Artifacts:** `kr64-test-results` with `cargo-test.log`,
  `.fingerprint/`, and built `kr64*` binaries (14-day, `if: always()`).

---

## 3. Emulator testing — codespace with KVM

Use a GitHub Codespace on `standardLinux32gb` (EastUs, AMD EPYC 7763,
`/dev/kvm` accessible — see `TWOYI_HONEST_STATUS.md` §2).

```bash
# 3.1 One-time setup (devcontainer pre-installs SDK + NDK + Rust):
avdmanager create avd -n twoyi_test -k "system-images;android-30;google_apis;x86_64" -d pixel_5
./gradlew assembleRelease -Pabis=all
adb install -r app/build/outputs/apk/release/app-release.apk

# 3.2 Boot the emulator headless:
$ANDROID_HOME/emulator/emulator -avd twoyi_test -no-window -no-audio -no-snapshot &
adb wait-for-device

# 3.3 Extract an x86_64 rootfs from the running emulator (trick from
#     download/X86_64_BREAKTHROUGH.md — the emulator's system.img
#     becomes twoyi's rootfs):
adb root
adb shell 'cd / && tar cf /data/local/tmp/rootfs.tar system/ init* default.prop'
adb pull /data/local/tmp/rootfs.tar /tmp/rootfs-x86_64.tar
adb shell 'mkdir -p /data/data/io.twoyi/profiles/default/rootfs'
adb shell 'cd /data/data/io.twoyi/profiles/default/rootfs && tar xf /data/local/tmp/rootfs.tar'
adb shell 'rm /data/data/io.twoyi/rootfs/init && \
           cp /data/data/io.twoyi/rootfs/system/bin/init /data/data/io.twoyi/rootfs/init'
adb shell setenforce 0   # SELinux permissive for first-boot debugging

# 3.4 Launch and observe:
adb shell am start -n io.twoyi/.ui.SettingsActivity   # then tap Launch Container
adb logcat | grep -E "KR64|CORE|NEW_RENDERER|CLIENT_EGL"
```

**Known limitation:** the Android emulator's `/dev/qemu_pipe` is
wired to the emulator's own GL renderer, not twoyi's. The x86_64
rootfs proves `init` executes and the pipe connects, but full
compositing needs the `kr64` daemon to create twoyi's own pipe device
(see `X86_64_BREAKTHROUGH.md`).

---

## 4. Device testing — real arm64 hardware

A physical arm64 device is the **highest-fidelity** test target and is
required for the headline verification of the AOSP renderer (Roadmap
task 1.1).

```bash
./gradlew assembleRelease -Pabis=arm64-v8a
adb install -r app/build/outputs/apk/release/app-release.apk
adb shell am start -n io.twoyi/.ui.SettingsActivity   # tap Launch Container
adb logcat -c && adb logcat | grep -E "KR64|CORE|NEW_RENDERER|CLIENT_EGL|BOOT_COMPLETED"
```

Pass criteria (Roadmap 1.1): APK installs and boots; guest GL output
renders; no `dlopen` failures in logcat; tombstone count = 0 over a
5-minute session (`adb shell ls /data/tombstones/`).

### Direct invocation of `libtwoyi.so` via `linker64`

For debugging the renderer without the full Java UI, see
[`TESTING_DIRECT_INVOCATION.md`](../TESTING_DIRECT_INVOCATION.md):

```bash
adb push app/src/main/jniLibs/arm64-v8a/libtwoyi.so  /data/local/tmp/
adb push app/src/main/jniLibs/arm64-v8a/libloader.so /data/local/tmp/
adb shell LD_LIBRARY_PATH=/data/local/tmp \
  /system/bin/linker64 /data/local/tmp/libtwoyi.so --help
```

Verify ELF structure first with `./test_libtwoyi.sh` (checks entry
point is non-zero, `main`/`JNI_OnLoad` exported, `twoyi_*` FFI
symbols present).

---

## 5. Integration testing — does the container actually boot?

### 5.1 logcat analysis

The boot leaves a distinctive trace. Capture and filter:

```bash
adb logcat -c
adb shell am start -n io.twoyi/.ui.SettingsActivity   # then tap Launch
adb logcat -d > twoyi-boot.log
grep -E "KR64 INFO|KR64 WARN|KR64 ERROR|CORE|NEW_RENDERER|CLIENT_EGL|SOCKET_MONITOR|BOOT_COMPLETED" twoyi-boot.log
```

Milestones to look for, in order:
1. `[KR64 INFO] kr64 daemon starting`
2. `[KR64 INFO] created device /dev/qemu_pipe` (then `touch`, `key0`, `event`, `gb`, `gb2`)
3. `[NEW_RENDERER] QEMU pipe device /dev/qemu_pipe availability: true`
4. `[NEW_RENDERER] Successfully connected to QEMU pipe: /opengles3`
5. `[NEW_RENDERER] GL context created successfully`
6. `BOOT_COMPLETED` (from `TwoyiStatusManager`)

If step 6 doesn't fire within ~60 s, `Render2Activity` times out and
returns to `SettingsActivity`. Enable per-pipe debug log files via
Settings → Advanced → Debug Renderer → Send Log (produces a
`bugreport.zip` with `renderer_debug/pipe_*_write.log` — see
`DEBUG_RENDERER_TESTING.md`).

### 5.2 Screenshot verification

```bash
adb exec-out screencap -p > /tmp/twoyi_state.png
```

Reference screenshots live in
[`download/screenshots/`](screenshots/) (`01_twoyi_settings.png`,
`02_boot_log_3s.png`, `05_x86_64_rootfs_boot.png`, …) and in
[`screenshots/`](../screenshots/) at the repo root.

### 5.3 VLM analysis (vision-language model)

Two helper scripts call the `glm-4.6v` model via `z-ai-web-dev-sdk`:

```bash
node scripts/vlm_analyze.js /tmp/twoyi_state.png \
  "Is this the twoyi boot log, the Android launcher, or a crash? Give tap coordinates."

python3 scripts/analyze_screenshot.py /tmp/twoyi_state.png   # same thing via z-ai CLI
```

> ⚠️ **Honesty warning.** `TWOYI_HONEST_STATUS.md` documents a prior
> false positive: a VLM reported the container had booted, but the
> screenshot was actually the Android emulator's own launcher
> (NexusLauncher + pink/purple wallpaper) — twoyi had crashed with
> SIGABRT. **Never trust VLM output alone.** Cross-check against
> (a) the logcat milestones in §5.1 and (b) `adb shell ps -A | grep
> io.twoyi` confirming the guest `init` is alive.

---

## 6. Performance testing

### 6.1 Boot time

```bash
adb logcat -c
START=$(date +%s)
adb shell am start -n io.twoyi/.ui.SettingsActivity   # then tap Launch
adb logcat | grep -m1 "BOOT_COMPLETED"
END=$(date +%s); echo "Boot time: $((END - START)) s"
```

For finer granularity: `adb logcat -v threadtime | grep "KR64 INFO"`.
Target: under 30 s on a flagship arm64 device; under 60 s in the
codespace emulator.

### 6.2 Rendering FPS

```bash
adb shell dumpsys SurfaceFlinger --latency io.twoyi/io.twoyi.Render2Activity
adb shell dumpsys gfxinfo io.twoyi framestats
```

`BootLogTexture.java` also logs frame timing when debug rendering is on.

### 6.3 Memory / CPU / I/O

```bash
adb shell dumpsys meminfo io.twoyi | head -20
# PSS over time (every 5 s for 2 min):
for i in $(seq 1 24); do adb shell dumpsys meminfo io.twoyi | grep "TOTAL PSS:"; sleep 5; done
adb shell top -m 10 -d 1 -p $(adb shell pidof io.twoyi)   # CPU%
adb shell iotop -p $(adb shell pidof io.twoyi)            # disk I/O
```

Watch for: PSS growth during boot (expected), leaks during steady-state
render (regression), and the `Native` heap column (the Rust side).

---

## 7. Test data — rootfs images and how to make more

### 7.1 Pre-built rootfs

| Source | Arch | Size | Notes |
|---|---|---|---|
| `https://github.com/cyanmint/twoyi/releases/download/original/rootfs.tar.gz` | arm64 | ~275 MB | The "official" rootfs bundled with upstream VM. Drop into `app/src/main/assets/rootfs.tar.gz` (or let CI fetch it via `include_rootfs`). |
| Android SDK `system-images;android-30;google_apis;x86_64` | x86_64 | ~600 MB | Extract from a running emulator — see §3.3. |
| Android GSI images (`https://source.android.com/docs/core/ota/gsi`) | arm64/x86_64 | ~1 GB | Treble GSIs for Android 11+. Requires a synthetic `vendor.img` stub — see `GSI_BOOT_PLAN.md` §1.4. |

### 7.2 Build an x86_64 rootfs from AOSP source

The definitive guide is
[`download/X86_64_ROOTFS_BUILD_GUIDE.md`](X86_64_ROOTFS_BUILD_GUIDE.md),
offering three paths:

- **Path A — Build from AOSP source** (2–6 h one-time, ~30 min
  incremental): pins `android-8.1.0_r81` from `default.xml`. Required
  if you'll later patch `init`, `surfaceflinger`, or `libui`.
- **Path B — Convert a pre-built GSI** (20–60 min): download a Treble
  `system.img` and convert to twoyi's flat `rootfs/` layout. Lower
  fidelity (Android 11+ vs. 8.1).
- **Path C — Extract from the Android SDK emulator** (5 min): the
  fastest smoke-test path — see §3.3.

### 7.3 Synthetic vendor stub

A GSI ships no `vendor.img`. twoyi needs at least a stub with empty
VINTF manifests so `init` doesn't fail to start HAL-dependent services.
See `GSI_BOOT_PLAN.md` §5.7 for the file layout
(`/vendor/etc/vintf/manifest.xml`, `/vendor/build.prop`, …).

### 7.4 Test fixtures inside the repo

- `download/twoyi_container_booted.png` — reference "booted" screenshot.
- `download/screenshots/` — captured boot sequence (3 s, 8 s, 20 s).
- `app/src/test/java/io/twoyi/ExampleUnitTest.java` — Java unit-test skeleton.
- `app/src/androidTest/java/io/twoyi/ExampleInstrumentedTest.java` — instrumented skeleton.

---

*If a test stops working, open a PR against `main` and update the command + file:line ref.*
