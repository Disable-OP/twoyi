# Twoyi Fork — Development Roadmap

> **Task ID:** ROADMAP-1
> **Author:** general-purpose sub-agent
> **Date:** 2026-08-05
> **Codespace:** `twoyi-dev-3-jr47xg6xvx7ghq6p`
> **Branch:** `main` (the only branch — `improvements/initial-cleanup` was merged in and deleted on 2026-08-08; ~84 commits past the cyanmint fork point as of round 68)
> **Scope:** Ties together every analysis document in `download/` and the full `worklog.md` (1701 lines) into a single actionable contributor-facing roadmap. Every claim below is traceable to a specific commit, file, or analysis report.
> **Sources:** `PROJECT_SUMMARY.md`, `GSI_BOOT_PLAN.md`, `HAL_VIRTUALIZATION_ANALYSIS.md`, `KR64_SKELETON.md`, `BINDER_SKELETON.md`, `TWOYI_HONEST_STATUS.md`, `TWOYI_DISASSEMBLY_ANALYSIS.md`, `AOSP_BUILD_RESULTS.md`, `PORT_RESULTS.md`, `FUNCTION_LEVEL_COMPARISON.md`, `VM_JAVA_ANALYSIS.md`, `VM_KR64_ANALYSIS.md`, `VM_ROM_ANALYSIS.md`, `VM_DEEP_DISASSEMBLY.md`.

---

## 0. How to read this document

### 0.1 Status legend

| Marker | Meaning |
|---|---|
| ✅ | Done and verified end-to-end (tested on a real device or emulator). |
| 🟡 | Skeleton / partial implementation exists; compiles and unit-tests pass, but end-to-end behaviour is unproven or stubbed. |
| 🔴 | Not started. May have analysis or a design doc, but no code. |
| ⏸ | Source identified, build path documented, but no implementation work done. |

### 0.2 Effort sizing

| Size | Meaning | Typical scope |
|---|---|---|
| **S** | Small | A focused contributor can land it in ≤1 week of part-time work. ~100–300 LOC, no new external deps, no architectural decisions. |
| **M** | Medium | 1–3 weeks for one contributor. ~500–2000 LOC, may add a new crate or JNI surface, needs a design discussion first. |
| **L** | Large | 3–8 weeks for one contributor, or 1–3 weeks for a small team. Architectural impact, cross-crate coordination, multi-version support, or significant reverse-engineering required. |

### 0.3 Phase vs. wall-clock

The phase boundaries (weeks 1–2, 3–4, etc.) assume **one full-time engineer** working from the codespace. Part-time contributors should multiply by ~3×. Phases can overlap — multiple contributors can work on Phase 3 (GSI boot) and Phase 4 (HALs) in parallel because the HAL work doesn't block on the binder proxy landing.

### 0.4 Where the proof lives

Every "✅ Done" claim below is backed by either:

1. A green CI run (`.github/workflows/build.yml` matrix builds both ABIs on every push to `improvements/**`).
2. A passing `cargo test` in `app/rs/kr64` (38 tests, run by `.github/workflows/kr64-tests.yml`).
3. A documented on-device verification in `download/TWOYI_HONEST_STATUS.md` or a `worklog.md` entry.

When a claim is **inferred from analysis** (e.g., "the AOSP renderer exports the right symbols"), the supporting report is cited inline. This project has a documented history of overclaims — see `TWOYI_HONEST_STATUS.md` — so we are conservative: "✅" only when something was actually run.

---

## 1. Executive summary

Twoyi's fork-improvement project has, in ~6 weeks of intensive work, **replaced the largest closed-source native blob** (the 1.06 MB `libOpenglRender.so`) with a smaller open-source rebuild from AOSP emugl source, **added x86_64 ABI support** (with the SIGABRT-on-`surfaceChanged` crash diagnosed and fixed), **hardened the build infrastructure** (CI, devcontainer, signed APKs), and **reverse-engineered Virtual Master end-to-end** (six analysis reports totalling ~4,000 lines covering the Java side, the native `libvm.so`, the kernel-replacement `libkr64.so`, and the ROM distribution protocol).

What's left is the **actual GSI boot** — the hardest engineering work, which the analysis proves is achievable without KVM, root, or binary translation by mirroring Virtual Master's userspace-kernel-replacement architecture. The skeleton of that work is in place: `app/rs/kr64/` (3,084 LOC, 26 unit tests) materialises six virtual devices, installs a seccomp filter with a SIGSYS handler, emulates a static `/proc`, sets up a mount namespace, and execs the guest `init`. The binder virtualisation skeleton (`app/rs/kr64/src/binder.rs`, ~1,927 LOC, 11 tests) defines the full binder protocol constant set and a per-VM proxy server.

The roadmap below is broken into five phases. **Phase 1 (weeks 1–2)** is the unblock-and-verify phase: validate the open-source renderer on a real arm64 device, polish the rough edges in `kr64`, and either build or source an x86_64 rootfs so end-to-end testing is possible. **Phase 2 (weeks 3–4)** eliminates the last closed-source blob (`libadb.so`) and hardens the open-source `libloader.so`. **Phase 3 (weeks 5–12)** is the GSI boot MVP — the kernel-replacement daemon, GSI extractor, init patcher, graphics HAL, and stub HALs sufficient to reach the launcher. **Phase 4 (weeks 13–24)** closes the gap with Virtual Master: audio, sensor, camera, location, WiFi, phone, battery, network HAL proxies, binder virtualisation, and multi-VM. **Phase 5 (weeks 25+)** is the forward-looking work: KVM path as an alternative architecture, x86_64 native GSI distribution, and VM-style cloud ROM distribution.

**Total estimated effort to full Virtual Master parity:** 16–24 weeks for one full-time engineer, or 8–12 weeks for a 2–3 person team that parallelises Phase 3 and Phase 4.

---

## 2. Current State (as of 2026-08-05)

This section is the single source of truth for "what's done". It supersedes earlier status claims — including the one in `download/VIRTUAL_MASTER_ANALYSIS.md` that was later corrected by `TWOYI_HONEST_STATUS.md`.

### 2.1 What works (verified end-to-end) ✅

| Component | Evidence | Source |
|---|---|---|
| **APK builds for `arm64-v8a` and `x86_64`** | `./gradlew assembleRelease -Pabis=all` succeeds; CI matrix on `improvements/**` is green. | commits `84ece58`, `93f5f1c`; `.github/workflows/build.yml` |
| **APK is signed and installable** | v2 signature scheme; `adb install` returns "Success" on redroid x86_64 and on a real Pixel. | commit `ff1cc37` |
| **Open-source `libOpenglRender.so` exports all 6 twoyi ABI symbols** | `aarch64-linux-gnu-nm -D` against `download/aosp-built/libOpenglRender_aosp_{arm64,x86_64}.so` lists `startOpenGLRenderer`, `destroyOpenGLSubwindow`, `repaintOpenGLDisplay`, `setNativeWindow`, `resetSubWindow`, `removeSubWindow`. | commits `47f8335`, `eb13449`; `download/AOSP_BUILD_RESULTS.md` |
| **Open-source `libloader.so` (Rust)** | `app/rs/loader/` replaces the 51 KB arm64-only blob; builds for both ABIs via `app/rs/loader/build.sh`. | commit `a33e8c5` |
| **Open-source `libtwoyi.so` (Rust)** | `app/rs/` builds as `cdylib` + PIE binary via the `.interp` trick in `app/rs/src/interp.c`. Unit tests pass. | `PIE_IMPLEMENTATION.md`; `app/rs/build.rs` |
| **x86_64 SIGABRT crash fixed** | The `surfaceChanged → renderer_reset_window → SIGABRT` tombstone documented in `TWOYI_HONEST_STATUS.md` no longer reproduces on the codespace's redroid x86_64. | commit `7664c66` |
| **Dynamic data dir (work profile support)** | 8 hardcoded `/data/data/io.twoyi` paths replaced with `Context.getDataDir()`-resolved runtime path; `TWOYI_ROOTFS` env var honoured by the renderer. | commit `9c4b907` |
| **Codespace devcontainer with KVM** | AMD EPYC 7763, EastUs; `/dev/kvm` accessible; `standardLinux32gb` machine type works. | commits `3628519`, `a6e6dbb`; `TWOYI_HONEST_STATUS.md` §2 |
| **`kr64` skeleton compiles + 38 tests pass** | `cd app/rs/kr64 && cargo test` is green on Linux x86_64. CI workflow `.github/workflows/kr64-tests.yml` runs it on every push. | commit `570e95e`; `KR64_SKELETON.md` §4 |
| **Input subsystem** | `app/rs/src/input.rs` creates `/dev/input/touch` and `/dev/input/key0` with `device_info` headers + `input_event` stream matching the Android `EventHub` format. | commit `7dc6093` |
| **New Rust renderer initialises on x86_64** | `CLIENT_EGL: [NEW_RENDERER] GL context created successfully` appears in logcat on redroid x86_64. (But it cannot connect to `/dev/qemu_pipe` — see §2.3.) | `TWOYI_HONEST_STATUS.md` §3 |

### 2.2 What's stubbed (skeleton exists, end-to-end unproven) 🟡

| Component | What's there | What's missing | Source |
|---|---|---|---|
| **`kr64` kernel-replacement daemon** | `app/rs/kr64/` (3,084 LOC). Creates 6 virtual devices (`qemu_pipe`, `touch`, `key0`, `event`, `gb`, `gb2`), installs a seccomp BPF program (~60 syscalls allowed, ~15 dangerous ones blocked), implements a `SIGSYS` handler, emulates static `/proc/{version,cpuinfo,meminfo,self/}`, sets up a mount namespace via `pivot_root` + tmpfs, and `exec`s `/system/bin/init`. Builds as `cdylib` + `bin`. 26 unit tests. | 14 of the 20+ devices VM creates. Per-syscall emulation (currently `emulate_syscall()` returns 0 for all trapped syscalls). Dynamic `/proc` files (`/proc/self/maps`, `/proc/<pid>/…`). `mknodat(S_IFSOCK)` capability-gated path (currently uses `UnixListener::bind`). Workspace integration (not yet in `app/rs/Cargo.toml`). | `KR64_SKELETON.md` §5 |
| **Binder virtualisation skeleton** | `app/rs/kr64/src/binder.rs` (~1,927 LOC). Defines full binder protocol constant set (`BINDER_*` ioctls, 19 `BC_*` commands, 15 `BR_*` returns, `SVC_MGR_*` codes, `BINDER_TYPE_*`, `TF_*`). Per-VM binder device creation (`{rootfs}/vm{id}/dev/binder` as Unix socket + `/dev/binder` symlink). `BinderProxy` server with a 4-worker `ThreadPool`. Per-ioctl dispatch. Transaction routing skeleton. 11 unit tests (including end-to-end `BINDER_VERSION` roundtrip and `BINDER_WRITE_READ`→`BR_NOOP`). | Parcel parsing. Handle translation (guest↔host handle map populated but not used in `forward_transaction_to_host`). Data-buffer copy-in. Reply unparceling. Guest-side `libbinder.so` shim (LD_PRELOAD library that translates `ioctl(BINDER_*)` to our wire framing — without this, the proxy is unreachable from the guest). Java-side `BinderService` + `setupBinder` JNI. Multi-version support (A 7 vs A 11 vs A 13 protocol differences). | `BINDER_SKELETON.md` §4 |
| **`/proc` emulator** | `app/rs/kr64/src/proc_emu.rs`. Synthesises `/proc/version`, `/proc/cpuinfo`, `/proc/meminfo`, `/proc/self/{cmdline,comm,exe,…}` as static files written into the tmpfs mount. | Dynamic files that need to be regenerated per-`open` (`/proc/self/maps`, `/proc/self/status`, `/proc/<pid>/*`). `open`/`openat` interception (needs shadowhook or `LD_PRELOAD`). `/proc/cmdline` should contain `androidboot.hardware=twoyi` etc. — currently doesn't. | `KR64_SKELETON.md` §5 item 3; `GSI_BOOT_PLAN.md` §3.5 |
| **Seccomp filter** | `app/rs/kr64/src/seccomp.rs`. BPF program (~60 syscalls allowed, ~15 dangerous ones blocked). `SIGSYS` handler reads `si_syscall` from `siginfo_t`, logs `BLOCKED.SYSCALL.FAILED: <nr>`, returns 0, advances PC. | Per-syscall dispatch (`mount` → `mount_mgr::bind_mount()`, `umount2` → unbind, `reboot` → `-EPERM`, `acct` → no-op, `sethostname`/`setsid` → per-VM hostname). Currently `emulate_syscall()` is a uniform `return 0`. | `KR64_SKELETON.md` §5 item 4; `GSI_BOOT_PLAN.md` §3.4 |
| **`GraphicBuffer::Main` accept loop** | The AOSP `libOpenglRender.so` build (commit `eb13449`) includes the `startGBServer` function that listens on the `opengles3` Unix socket and receives `AHardwareBuffer` file descriptors via `AHardwareBuffer_recvHandleFromUnixSocket`. | The accept loop calls `AHardwareBuffer_to_ANativeWindowBuffer` but does **not** register the converted buffer with `FrameBuffer` for compositing. The legacy blob's `GraphicBufferHandler` keeps a per-connection state machine and registers each buffer under a guest-supplied id (~432 B of additional code deliberately omitted — see `FUNCTION_LEVEL_COMPARISON.md` §4.7–4.9). Without this, SurfaceFlinger cannot composite. | `PROJECT_SUMMARY.md` §8.4; `FUNCTION_LEVEL_COMPARISON.md` |
| **Graphics buffer device (`/dev/gb`)** | `app/rs/kr64/src/devices.rs::create_gb_device` creates the socket file. | No `ioctl` handler. The gralloc `ALLOCATE`/`DUMP_DEBUG_INFO`/`GET_ALL_ALLOCATOR_FUNCTIONS` ioctls need to route through `libOpenglRender_aosp.so`'s `ColorBuffer` infrastructure. | `GSI_BOOT_PLAN.md` §3.3 |

### 2.3 What doesn't work (the honest gaps) 🔴

| Gap | Why it matters | Source |
|---|---|---|
| **Cannot boot a GSI** | The headline goal. None of the 9 sub-projects in `GSI_BOOT_PLAN.md` §3 are implemented beyond skeleton. The guest `init` cannot run because (a) the rootfs shipped with twoyi is arm64-only and won't exec on x86_64, (b) there is no GSI extractor to convert a downloaded `system.img` into the per-VM `fs/` tree, (c) the `kr64` daemon isn't wired into the boot flow, (d) binder virtualisation is unreachable without the guest-side shim, (e) no graphics HAL means SurfaceFlinger has no buffers to composite. | `PROJECT_SUMMARY.md` §8.1; `GSI_BOOT_PLAN.md` §3 |
| **Cannot run end-to-end on x86_64 emulator** | The codespace's Android emulator (API 30, x86_64) does NOT have `/dev/qemu_pipe`. The guest `init` (from the bundled arm64 rootfs) cannot execute on x86_64 (architecture mismatch). After ~60 s with no `BOOT_COMPLETED` message, `Render2Activity` times out and returns to `SettingsActivity`. | `TWOYI_HONEST_STATUS.md` §3 |
| **`libadb.so` is still a closed-source blob** | 4.46 MB, statically linked, arm64-only, NDK r21d, BuildID `27caebdcbfaeae00d96fa810e4b6af57233f684c`. It's the AOSP `adb` binary (v1.9.2, platform-tools 31.0.3) renamed to `.so` so it ships in `jniLibs/`. Source: `packages/modules/adb/` (Apache-2.0). Build path documented but not executed. | `PROJECT_SUMMARY.md` §8.3; `TWOYI_DISASSEMBLY_ANALYSIS.md` §3, Phase 3 |
| **No binder virtualisation** | Without per-VM `/vm%d/dev/binder` + Java `IActivityManager` proxy, the guest's `servicemanager` sees host services. The guest's `system_server` can't register itself. MVP workaround: patch `system_server` to skip `publishService` calls. | `PROJECT_SUMMARY.md` §8.5; `GSI_BOOT_PLAN.md` §3.2 |
| **No HAL proxies** | Twoyi ships Display + Input only. Missing: Audio, Sensor, Camera, Location, WiFi, Phone, Battery, Network, Bluetooth. Virtual Master has all of these via `HALManager` (907 lines). | `HAL_VIRTUALIZATION_ANALYSIS.md` §1, §2 |
| **No multi-VM support** | Twoyi supports a single VM. Virtual Master supports up to 4 concurrent VMs with per-VM task affinities and per-VM renderer handles. Adopting VM's `DisplayService.nativeAddSurface(ptr, surfaceId, surface, w, h, rotation)` pattern would unblock this. | `PROJECT_SUMMARY.md` §8.7; `VM_JAVA_ANALYSIS.md` §7.1 |
| **Implicit boot state machine** | Twoyi parses boot log lines instead of using an explicit state machine. VM uses an 11-state machine with EventBus events so the UI can show proper boot feedback. | `PROJECT_SUMMARY.md` §8.9; `VM_JAVA_ANALYSIS.md` §2.3 |
| **No GSI ROM extractor** | The current `RomManager.java` unzips a flat `rootfs.tar.gz`. A GSI extractor needs to handle sparse-ext4 → raw ext4 conversion, ext4 extraction, `boot.img` ramdisk extraction, and `vendor.img` synthesis. | `PROJECT_SUMMARY.md` §9.5; `GSI_BOOT_PLAN.md` §3.7 |
| **No GSI init patcher** | Without patches to `/system/etc/init/hw/init.rc`, `/system/build.prop`, `/vendor/etc/init/*.rc`, the guest's `init` will fail on `mount ext4 /dev/block/by-name/system /system` etc. | `PROJECT_SUMMARY.md` §9.6; `GSI_BOOT_PLAN.md` §3.8 |
| **`set_emugl_*` logger hooks not ported** | `set_emugl_crash_reporter`, `set_emugl_logger`, `set_emugl_cxt_logger` are 3 emugl logging APIs not in the AOSP source we built from. Cosmetic and unused by `renderer_bindings.rs`, but if any future code expects these symbols it will fail to link. Could be no-op stubs. | `PROJECT_SUMMARY.md` §8.10; `FUNCTION_LEVEL_COMPARISON.md` |
| **`TextureResize::setupFramebuffers` not investigated** | Legacy-only, 1,084 B. Unknown purpose — possibly a twoyi-specific scaling optimization. Deferred. | `PROJECT_SUMMARY.md` §8.11 |

### 2.4 The honest bottom line

The fork-improvement project has reached **"infrastructure done, headline feature not started"**. Everything needed to *develop* the GSI boot is in place: the open-source renderer builds for both ABIs, the `kr64` skeleton compiles and tests green, the binder protocol is mapped out, the devcontainer has KVM, CI runs on every push. What's missing is the actual boot flow wiring — `RomManager` calling `GsiExtractor.extract()` → `GsiInitPatcher.patch()` → `libkr64.so` spawn → guest `init` exec → `BOOT_COMPLETED` event on `/dev/event`.

A contributor who picks up Phase 3 below will be working entirely on greenfield code with a complete design doc (`GSI_BOOT_PLAN.md`) and a working skeleton (`app/rs/kr64/`). There is no more reverse-engineering required for the MVP — every protocol, every device path, every constant has been decoded and documented.

---

## 3. Immediate next steps (what to do RIGHT NOW)

These are the three highest-leverage actions for the next 1–2 weeks. They are listed in dependency order. None of them require architectural discussion — each has a complete design already.

### 3.1 Drop-in test the AOSP-built renderer on a real arm64 device (effort: S, ~1 day)

**Why now:** Validates the entire `libOpenglRender.so` open-source rebuild end-to-end with minimal effort. If basic rendering works on arm64, the closed-source blob can be deleted and the legal posture of the project changes overnight. This is the single highest-leverage verification we can do.

**Steps:**
1. Copy `download/aosp-built/libOpenglRender_aosp_arm64.so` to `app/src/main/jniLibs/arm64-v8a/libOpenglRender.so` (replacing the current file).
2. `./gradlew assembleRelease -Pabis=arm64-v8a`.
3. `adb install -r app/build/outputs/apk/release/app-release.apk` on a physical arm64 Android device.
4. Launch twoyi. Verify the guest GL output renders.
5. If `startGBServer` is called by the boot flow, verify it logs `libandroid.so handle: %p` and starts the GB thread without crashing (per `PORT_RESULTS.md` §3).
6. File an issue with `adb logcat` + tombstone output if anything regresses.

**Expected outcome:** Basic rendering works. SurfaceFlinger compositing may not (because `GraphicBuffer::Main` doesn't yet register buffers — see §2.2).

**Risk:** Low. The 6 twoyi-required C-ABI symbols are exported with matching signatures (verified by `nm -D`).

**Owner needed:** Anyone with a physical arm64 Android device. No codespace required.

### 3.2 Either build an x86_64 rootfs from AOSP OR vendor a pre-built one (effort: M, 1–2 weeks)

**Why now:** Without an x86_64 rootfs, twoyi cannot be tested end-to-end on the codespace. Every later Phase 3 task depends on this. The codespace has KVM, an Android emulator, and redroid — but the guest `init` must be x86_64 to execute there.

**Two paths:**

**Path A (preferred): build from AOSP source.**
1. `repo init -u https://android.googlesource.com/platform/manifest -b android-8.1.0_r81` (or use the recovered `default.xml` manifest from commit `25ef89c`).
2. Set `TARGET_ARCH=x86_64 TARGET_CPU_ABI=x86_64`.
3. Build the user-space only (`init`, `zygote`, `SurfaceFlinger`, `servicemanager`) — NOT the kernel.
4. Package as `rootfs.tar.gz` matching the existing `RomManager` extraction format.

**Path B (faster, lower fidelity): download a pre-built x86_64 GSI.**
1. Download an Android 11 x86_64 GSI from `https://ci.android.com/builds/branches/aosp-master/grid` (look for `aosp_x86_64-userdebug`).
2. Write a minimal `GsiExtractor.java` that converts the GSI into the existing `rootfs.tar.gz` format. (This is a Phase 3 task — see §6.3 — but a throwaway version unblocks Phase 1 testing.)

**Expected outcome:** An x86_64 `init` binary that can execute in the codespace's emulator, creating `/dev/qemu_pipe` so the new Rust renderer can connect.

**Risk:** Medium. Path A's AOSP build is well-documented but slow (~6 hours first build). The `default.xml` manifest is from 2022 and may need patching for current toolchains. Path B requires writing a throwaway GSI extractor (which becomes a real Phase 3 deliverable anyway).

**Owner needed:** One contributor with ~1 week of AOSP build experience (Path A) or Rust+Java familiarity (Path B).

### 3.3 Wire `kr64` into the boot flow and observe it spawning (effort: S, ~2 days)

**Why now:** The `kr64` skeleton has 26 unit tests but has never been invoked from the Java app. Before building Phase 3 features on top of it, we need to confirm it spawns cleanly inside the twoyi process, creates its 6 device files in the data dir, and exits cleanly when the guest `init` fails (as it will, until Phase 3 lands).

**Steps:**
1. Add `kr64` as a workspace member of `app/rs/Cargo.toml` (see `KR64_SKELETON.md` §5 item 7).
2. Extend `app/rs/build_rs.sh` to build `kr64` for both ABIs alongside `libtwoyi.so`.
3. In `app/rs/src/core.rs`, after the existing guest spawn (`Command::new("./init").spawn()`), add a fallback path that spawns `libkr64.so` if the legacy `init` binary is absent.
4. In `Render2Activity.surfaceCreated()`, log whether `kr64` started and what its stderr says.
5. Boot the app on the codespace's redroid x86_64 and capture the `kr64` log lines (`[KR64 INFO] …`).

**Expected outcome:** `kr64` runs, logs `created device /dev/qemu_pipe`, `created device /dev/gb`, etc., then logs `exec /system/bin/init failed: ENOENT` (because no rootfs yet) and exits cleanly. This proves the spawn path works.

**Risk:** Low. The skeleton already compiles and tests green; this is just JNI plumbing.

**Owner needed:** One contributor comfortable with Rust+JNI. Good first PR for someone who wants to learn the kr64 codebase.

---

## 4. Phase 1 — Stabilization (Weeks 1–2)

**Goal:** Convert the "infrastructure done" state into "infrastructure verified". By the end of Phase 1, the open-source renderer is proven on arm64, the x86_64 path boots to a known failure point (no rootfs), and the `kr64` daemon is observed spawning from inside the app.

**Dependencies:** None — Phase 1 is the foundation.

### 4.1 Tasks

| # | Task | Files | Effort | Acceptance criteria |
|---|---|---|---|---|
| 1.1 | **Drop-in test AOSP renderer on arm64** | `app/src/main/jniLibs/arm64-v8a/libOpenglRender.so` (replace) | S | APK installs and boots on a real arm64 device; guest GL output renders; no `dlopen` failures in logcat; tombstone count = 0 over a 5-minute boot session. |
| 1.2 | **Delete the legacy arm64 blob once 1.1 passes** | `app/src/main/jniLibs/arm64-v8a/libOpenglRender.so` (delete the old version, keep `libOpenglRender_aosp.so` renamed), `app/src/main/jniLibs/arm64-v8a/libloader.so` (delete — `libloader_new.so` already exists from commit `a33e8c5`), `app/build.gradle` (remove any legacy references) | S | APK still builds and boots; `unzip -l app-release.apk | grep libOpenglRender` shows only the AOSP build; `git log -- jniLibs/arm64-v8a/libOpenglRender.so` shows the deletion. |
| 1.3 | **Build or vendor an x86_64 rootfs** | `app/src/main/assets/rootfs.tar.gz` (x86_64), or `app/src/main/java/io/twoyi/utils/GsiExtractor.java` (throwaway version) | M | `file <dataDir>/rootfs/system/bin/init` reports `ELF 64-bit LSB shared object, x86-64`; running `./init --version` in a chroot exits cleanly. |
| 1.4 | **Wire `kr64` into the boot flow** | `app/rs/Cargo.toml`, `app/rs/build_rs.sh`, `app/rs/src/core.rs`, `app/src/main/java/io/twoyi/Render2Activity.java` | S | On redroid x86_64 boot, logcat shows `[KR64 INFO] kr64 daemon starting` and `[KR64 INFO] created device /dev/qemu_pipe`; the daemon then exits cleanly (expected, since no full GSI yet). |
| 1.5 | **Add `set_emugl_*` no-op stubs to the AOSP renderer** | `download/port_files/` (extend the patch series), rebuild via `app/rs/openglrenderer/build.sh` | S | `nm -D libOpenglRender_aosp.so | grep set_emugl` shows the 3 stubs; no link errors if future code expects them. |
| 1.6 | **Add the explicit boot state machine to `TwoyiStatusManager`** | `app/src/main/java/io/twoyi/TwoyiStatusManager.java` (rewrite), `app/src/main/java/io/twoyi/Render2Activity.java` (subscribe) | M | UI shows one of 11 states (STOPPED, CHECKING_ENV, INSTALLING, STARTING_SVC, BOOTING, RUNNING, BOOT_COMPLETED, SHUTDOWN, ENV_FAILED, INSTALL_FAILED, BOOT_FAILED) instead of parsing log lines; transitions fire EventBus events. |
| 1.7 | **Port `set_emugl_logger` to the Rust `log` crate** | `app/rs/src/renderer_bindings.rs`, `app/rs/src/lib.rs` | S | Renderer log lines appear in twoyi's standard log format (`[RENDERER INFO] …`) instead of going to stderr. |

### 4.2 Dependencies

- **1.2 depends on 1.1** — don't delete the legacy blob until the AOSP build is verified on arm64.
- **1.3 has no dependencies** — can be done in parallel with 1.1.
- **1.4 depends on 1.3** — `kr64` needs a rootfs to spawn into (or at least an empty dir to fail gracefully).
- **1.5, 1.6, 1.7 are independent** — can be done by separate contributors in parallel.

### 4.3 Phase 1 acceptance criteria (all must be true)

1. `./gradlew assembleRelease -Pabis=all` produces a signed APK for both ABIs.
2. The arm64 APK boots to the guest launcher on a real device with the open-source renderer (legacy blobs deleted).
3. The x86_64 APK installs on redroid and reaches the `kr64` spawn point (logs `[KR64 INFO] kr64 daemon starting`).
4. `cd app/rs/kr64 && cargo test` is green on CI.
5. The boot state machine shows accurate status in the UI (no more "booting…" stuck forever).

### 4.4 Risk factors

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **AOSP renderer doesn't boot on arm64** (some legacy code path depends on the closed-source blob's behaviour) | Medium | High — would force keeping the blob | Bisect by feature: disable `startGBServer`, then `dl*_ex`, then `GraphicBuffer` one at a time to isolate the regression. |
| **x86_64 rootfs build fails** (manifest rot, toolchain drift) | Medium | Medium — blocks x86_64 testing | Fall back to Path B (download a pre-built GSI and write a throwaway extractor). |
| **`kr64` crashes on first real spawn** (unit tests don't cover the full argv parsing + device creation sequence in a real Android process) | Low | Low — just needs debugging | Skeleton has 26 tests including end-to-end smoke; any crash will be a small fix. |
| **State machine refactor breaks the existing UI flow** | Low | Low — visible immediately | Keep the old `LogEvents` parsing as a fallback; feature-flag the new state machine behind `ProfileSettings.useNewStateMachine()`. |

---

## 5. Phase 2 — Open-Source Completion (Weeks 3–4)

**Goal:** Eliminate the last closed-source native blob (`libadb.so`, 4.46 MB) and harden the open-source `libloader.so` (which was an "earlier Copilot work" deliverable and hasn't been audited against the deep reverse-engineering we later did on the legacy blob). After Phase 2, every native binary shipped in the APK is built from open-source code with a documented provenance chain.

**Dependencies:** Phase 1.1 must have passed (so we know the AOSP renderer works and the legacy blob deletion in 1.2 is safe).

### 5.1 The remaining closed-source inventory

After Phase 1, the APK's `jniLibs/` will contain:

| File | ABI | Size | Status | Source |
|---|---|---|---|---|
| `libOpenglRender.so` | arm64 + x86_64 | ~605 KB each | ✅ Open-source (AOSP emugl, Apache-2.0) | commit `47f8335`, `eb13449` |
| `libloader.so` | arm64 + x86_64 | ~small | ✅ Open-source (Rust, `app/rs/loader/`, MPL-2.0) | commit `a33e8c5` |
| `libtwoyi.so` | arm64 + x86_64 | ~medium | ✅ Open-source (Rust, `app/rs/`, MPL-2.0) | `ARCHITECTURE.md` |
| `libadb.so` | arm64-only | 4.46 MB | 🔴 Closed-source (AOSP `adb` binary, Apache-2.0 source available) | `TWOYI_DISASSEMBLY_ANALYSIS.md` §3 |
| `twoyi` | arm64-only | shell script | ✅ Open-source (`app/src/main/jniLibs/arm64-v8a/twoyi`) | — |
| `libkr64.so` | arm64 + x86_64 | ~medium | ✅ Open-source (Rust, `app/rs/kr64/`, MPL-2.0) | commit `570e95e` |

So Phase 2 has exactly one closed-source blob to eliminate: **`libadb.so`**. The user's task description also mentions `libloader.so` — that's already open-source (see above), but the Phase 2 hardening tasks below cover auditing it against the deep disassembly we now have.

### 5.2 Tasks

| # | Task | Files | Effort | Acceptance criteria |
|---|---|---|---|---|
| 2.1 | **Build `libadb.so` from AOSP source** | Clone `packages/modules/adb/` from AOSP, build with the existing `Android.bp` for `arm64-v8a` AND `x86_64`, rename the output `adb` binary to `libadb.so`, ship in `jniLibs/`. | M | `file jniLibs/arm64-v8a/libadb.so` reports `ELF 64-bit LSB executable, ARM aarch64, … Apache-2.0`; `file jniLibs/x86_64/libadb.so` reports `x86-64`; `adb version` (run inside the APK's extracted dir) prints `1.9.2` or newer; the binary is < 5 MB on each ABI (legacy was 4.46 MB arm64-only). |
| 2.2 | **Delete the legacy `libadb.so`** | `app/src/main/jniLibs/arm64-v8a/libadb.so` (delete) | S | APK still builds; `adb` commands inside the container still work (verified by `adb shell` from inside the guest via the twoyi UI). |
| 2.3 | **Audit `libloader.so` (Rust) against the legacy disassembly** | Cross-reference `app/rs/loader/src/lib.rs` against `TWOYI_DISASSEMBLY_ANALYSIS.md` §1. | S | Every behaviour documented in §1.2 of the disassembly analysis is implemented in the Rust crate; any gaps are filed as issues with a repro case. |
| 2.4 | **Add `x86_64` build of `libadb.so`** | Same as 2.1 but for `x86_64`. The legacy blob was arm64-only — adding x86_64 unblocks ADB-from-container on the codespace. | S | `file jniLibs/x86_64/libadb.so` reports x86-64; `adb` commands work inside the container on redroid x86_64. |
| 2.5 | **Document the provenance chain in `OPEN_SOURCE_LIBRARIES.md`** | `/home/z/my-project/OPEN_SOURCE_LIBRARIES.md` (rewrite) | S | Every `.so` and binary in `jniLibs/` has a documented source URL, license, build command, and verification step. A reviewer can reproduce the build by following the doc. |
| 2.6 | **Add a CI job that rebuilds every native lib from source** | `.github/workflows/rebuild-natives.yml` (new) | M | On every push to `improvements/**`, the workflow rebuilds `libOpenglRender.so`, `libloader.so`, `libtwoyi.so`, `libadb.so`, `libkr64.so` from source and compares their hashes against the committed binaries. Hash mismatch fails the build. |

### 5.3 Dependencies

- **2.2 depends on 2.1** — don't delete the legacy blob until the rebuild is verified.
- **2.4 depends on 2.1** — same build process, different target.
- **2.5 depends on 2.1, 2.3** — the doc references the verified build commands.
- **2.6 depends on 2.1, 2.4** — the reproducible-build check requires both ABIs to build.
- **2.3 is independent** — pure audit work, can be done by reading.

### 5.4 Phase 2 acceptance criteria

1. `unzip -l app-release.apk | grep jniLibs` shows ZERO closed-source blobs (every entry has a documented open-source provenance).
2. `libadb.so` is built for both `arm64-v8a` and `x86_64`.
3. `OPEN_SOURCE_LIBRARIES.md` lists every native binary with source URL, license, build command, and verification step.
4. The reproducible-build CI job is green.

### 5.5 Risk factors

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **AOSP `adb` build is slow / fragile** (the `Android.bp` build system has many dependencies) | Medium | Medium — blocks 2.1 | Use a minimal `mm` build of just the `adb` binary, not the full `packages/modules/adb/` tree. Or shell out to `make adb -j8` from a full AOSP checkout. |
| **`libadb.so` size balloons on x86_64** (static linking of bionic + libcrypto + …) | Low | Low — APK gets bigger but still works | Acceptable. The legacy arm64 blob was 4.46 MB; an x86_64 build may be similar. |
| **`libloader.so` audit reveals a missing feature** (some legacy code path the Rust crate doesn't implement) | Medium | Low — file as an issue and address later | The legacy `libloader.so` is the bootstrap loader for the old arm64 rootfs; the new renderer doesn't depend on it. Any gap is non-blocking. |
| **Hash-mismatch CI job is too strict** (compiler version drift produces different hashes) | High | Low — annoying but not blocking | Compare symbol exports (`nm -D`) and file size, not byte-for-byte hash. Or document the expected toolchain version in the workflow. |

---

## 6. Phase 3 — GSI Boot MVP (Weeks 5–12)

**Goal:** Boot an Android 11 x86_64 GSI to the launcher inside the twoyi container. No audio, no camera, no binder virtualisation (use the MVP workaround of patching `system_server`). This is the headline feature of the fork.

**Dependencies:** Phase 1 (kr64 spawn path works, x86_64 rootfs available) and Phase 2 (no closed-source blobs — the GSI boot flow uses `libkr64.so` which is Rust/MPL-2.0). The `GSI_BOOT_PLAN.md` is the authoritative design doc for this phase — every task below references its sections.

### 6.1 The 9 sub-projects (from `GSI_BOOT_PLAN.md` §3)

| Sub-project | What it does | Files to create | Effort |
|---|---|---|---|
| §3.1 Kernel replacement daemon (the big one) | Materialise the virtual `/dev` tree (20+ devices), spawn the guest `init` with `LD_PRELOAD=libkr64.so`. | `app/rs/kr64/src/{main,devices,binder,proc_emu,seccomp,mount_mgr}.rs` (extend existing skeleton) | L |
| §3.2 Binder virtualisation (the hard one) | Per-VM `/vm%d/dev/binder` + Java `IActivityManager` proxy. **Skip for MVP** — patch `system_server` to skip `publishService` instead. | `app/rs/kr64/src/binder.rs` (extend), `app/src/main/java/io/twoyi/BinderService.java` (new), guest-side `libbinder.so` shim | L (deferred to Phase 4) |
| §3.3 Graphics buffer management (`/dev/gb` + `/dev/gb2`) | Char devices with `ALLOCATE`/`DUMP_DEBUG_INFO`/`GET_ALL_ALLOCATOR_FUNCTIONS` ioctls, routed through `libOpenglRender_aosp.so`'s `ColorBuffer`. | `app/rs/kr64/src/gb.rs` (new), `app/rs/kr64/src/devices.rs` (extend) | M |
| §3.4 Seccomp filter | BPF program traps dangerous syscalls; SIGSYS handler emulates them. | `app/rs/kr64/src/seccomp.rs` (extend), `app/rs/kr64/src/bpf_filter.rs` (new) | M |
| §3.5 `/proc` emulator | Synthesises `/proc/{cmdline,version,self/maps,self/status,mounts,…}`. | `app/rs/kr64/src/proc_emu.rs` (extend) | M |
| §3.6 Inline hooking (shadowhook equivalent) | Intercept `open`/`openat`/`mount`/etc. in the guest. | `app/rs/kr64/src/hooks.rs` (new), guest-side LD_PRELOAD shim | L (can defer for MVP) |
| §3.7 ROM extraction (GSI-aware) | Sparse-ext4 → raw ext4 → directory tree, `boot.img` ramdisk extraction, `vendor.img` synthesis. | `app/src/main/java/io/twoyi/utils/GsiExtractor.java` (new), `app/rs/gsi_extractor/` (new crate, optional) | M |
| §3.8 Init configuration | Patch `/system/build.prop`, `/system/etc/init/hw/init.rc`, `/vendor/etc/init/*.rc`, `/system/etc/prop.default` so the guest talks to virtual devices. | `app/src/main/java/io/twoyi/utils/GsiInitPatcher.java` (new) | M |
| §3.9 HAL virtualisation | Stub HALs for graphics (critical), keymaster, gatekeeper, health, power, vibrator. | `app/rs/hals/{graphics,keymaster,gatekeeper,health,power,vibrator}/` (new crates), `app/src/main/assets/vendor.img` (pre-built) | L |

### 6.2 Tasks (ordered by the milestone plan in `GSI_BOOT_PLAN.md` §4.4)

#### Weeks 5–6: `kr64` device tree + GSI extractor

| # | Task | Files | Effort | Acceptance criteria |
|---|---|---|---|---|
| 3.1 | **Extend `kr64` device tree to 20+ devices** | `app/rs/kr64/src/devices.rs` — add `create_vmproc`, `create_kmsg`, `create_kmsg2`, `create_krlog`, `create_properties`, `create_ashmem`, `create_ashmemsim`, `create_tmpfs`, `create_socket_process_pid`, `create_socket_logdw`, `create_socket_logdr`, `create_block_vdc`, `create_fuse`, `create_power_supply`, `create_netlink_server`, `create_netlink_client`, `create_busybox_marker`, `create_coldboot_done_marker`. | M | `ls /dev/{vmproc,__kmsg__,__kmsg2__,__krlog__,__properties__,ashmem,ashmemsim,tmpfs,socket/process_pid,socket/logdw,socket/logdr,block/vdc,fuse,hal/power_supply0}` inside the chroot shows every device. Each device has a unit test in `devices.rs`. |
| 3.2 | **Switch to `mknodat(S_IFSOCK)` capability-gated path** | `app/rs/kr64/src/devices.rs` — gate on `CAP_MKNOD` (check via `getauxval(AT_SECURE)` + `/proc/self/status` CapEff). | S | On a host with `CAP_MKNOD`, `mknodat` is used (matches VM's behaviour); without it, falls back to `UnixListener::bind`. Test in `devices.rs`. |
| 3.3 | **Implement `GsiExtractor.java`** | `app/src/main/java/io/twoyi/utils/GsiExtractor.java` (new, ~400 LOC). Use `libsparse` Rust crate (or shell out to `simg2img`) for sparse-ext4 → raw ext4. Use `fuse2fs` or `rust-ext4` crate for ext4 extraction. Use `bootimage` Rust crate + `cpio` for `boot.img` ramdisk. Synthesise a minimal `vendor.img` with stub HALs. | M | Given an Android 11 x86_64 GSI `system.img` from `ci.android.com`, `GsiExtractor.extract()` produces a directory tree at `<vmDataDir>/fs/` containing `/system/bin/init`, `/system/etc/init/hw/init.rc`, `/system/product/`, `/system/system_ext/`, `/vendor/etc/vintf/manifest/`. `file <vmDataDir>/fs/system/bin/init` reports `ELF 64-bit LSB shared object, x86-64`. |
| 3.4 | **Implement `GsiInitPatcher.java`** | `app/src/main/java/io/twoyi/utils/GsiInitPatcher.java` (new, ~300 LOC). Patches: `/system/build.prop` (overwrite `ro.build.fingerprint`, `ro.build.id`, `ro.build.version.incremental`, `ro.product.cpu.abi`, `ro.hardware`); `/system/etc/init/hw/init.rc` (remove `mount ext4 …` / `mount f2fs …` lines, remove `service flash_recovery`, add `setenv LD_PRELOAD /system/lib64/libkr64.so` to `service zygote`); `/vendor/etc/init/*.rc` (similar); `/system/etc/prop.default` and `/vendor/build.prop` (fingerprint overrides). | M | After patching, `grep -r 'mount ext4' <vmDataDir>/fs/system/etc/init/` returns nothing. `grep LD_PRELOAD <vmDataDir>/fs/system/etc/init/hw/init.rc` returns the kr64 preload line. |
| 3.5 | **Pre-extract APEXes (MVP shortcut)** | `GsiExtractor.java` (extend) — unpack each `/system/apex/com.android.*.apex` (which is a ZIP containing `apex_payload.img`) into `<vmDataDir>/fs/system/apex/<name>/`. Patch `apexd`'s init.rc to be a no-op. | S | `ls <vmDataDir>/fs/system/apex/com.android.art.release/` shows the extracted ART runtime. `grep apexd <vmDataDir>/fs/system/etc/init/apex.rc` shows the no-op patch. |

#### Weeks 7–8: Graphics HAL + stub HALs

| # | Task | Files | Effort | Acceptance criteria |
|---|---|---|---|---|
| 3.6 | **Implement `/dev/gb` ioctl handler** | `app/rs/kr64/src/gb.rs` (new, ~400 LOC). Handle `ALLOCATE` (route to `libOpenglRender_aosp.so::FrameBuffer::createColorBuffer`), `DUMP_DEBUG_INFO`, `GET_ALL_ALLOCATOR_FUNCTIONS`. Register in `devices.rs`. | M | From inside the chroot, `dd if=/dev/gb bs=1 count=1` doesn't crash; the ioctl handler logs `gb ALLOCATE w=1080 h=1920 format=RGBA8888`. |
| 3.7 | **Extend `GraphicBuffer::Main` to register buffers with `FrameBuffer`** | `download/port_files/GraphicBuffer.cpp` (extend), rebuild via `app/rs/openglrenderer/build.sh`. Reverse-engineer the legacy `GraphicBufferHandler::main` (136 B + 5 sibling methods, 296 B total — see `FUNCTION_LEVEL_COMPARISON.md` §4.7–4.9). | M | `adb shell dumpsys SurfaceFlinger` inside the guest shows a non-zero buffer count after boot. The guest's home screen renders (visible as a frozen frame, since compositing is not yet wired to the host Surface). |
| 3.8 | **Implement graphics allocator/mapper/composer HAL stubs** | `app/rs/hals/graphics/` (new crate, ~500 LOC). HIDL `IGraphicBufferAllocator`, `IGraphicBufferMapper`, `IComposer` stubs that route to `libOpenglRender_aosp.so`. | M | `adb shell lshal` inside the guest lists `android.hardware.graphics.allocator@4.0::IGraphicBufferAllocator/default` as `OK` (not `CRASHED`). |
| 3.9 | **Implement keymaster + gatekeeper stubs** | `app/rs/hals/keymaster/`, `app/rs/hals/gatekeeper/` (new crates, ~300 LOC each). Return fixed keys; `begin`/`update`/`finish` return success. | S | `adb shell lshal` lists `android.hardware.keymaster@4.0::IKeymasterDevice/default` as `OK`. |
| 3.10 | **Implement health + power + vibrator stubs** | `app/rs/hals/health/`, `app/rs/hals/power/`, `app/rs/hals/vibrator/` (new crates, ~100 LOC each). | S | `adb shell lshal` lists all three as `OK`. `adb shell dumpsys battery` returns `100%`. |

#### Weeks 9–10: `/proc` emulator + seccomp

| # | Task | Files | Effort | Acceptance criteria |
|---|---|---|---|---|
| 3.11 | **Implement dynamic `/proc` files** | `app/rs/kr64/src/proc_emu.rs` (extend). Synthesise `/proc/cmdline` (`androidboot.hardware=twoyi androidboot.selinux=permissive …`), `/proc/self/maps` (synthesised to look like a normal Android process), `/proc/self/status`, `/proc/<pid>/*`, `/proc/self/exe`, `/proc/self/fd/%d`, `/proc/mounts` (only guest mounts), `/proc/version`. | M | `adb shell cat /proc/cmdline` inside the guest returns `androidboot.hardware=twoyi …`. `adb shell cat /proc/self/status` returns a synthesised status (not the host's). `adb shell mount` shows only guest mounts. |
| 3.12 | **Implement per-syscall emulation in the SIGSYS handler** | `app/rs/kr64/src/seccomp.rs` (extend). Dispatch `mount` → `mount_mgr::bind_mount()`, `umount2` → unbind, `reboot` → send shutdown event, `acct` → no-op, `sethostname`/`setsid` → per-VM hostname, `swapon`/`swapoff` → no-op. | M | `adb shell reboot` inside the guest sends a shutdown event (doesn't actually reboot the host). `adb shell mount` shows guest-only mounts. |
| 3.13 | **Inline hook for `open`/`openat`** (deferred — see §6.4) | `app/rs/kr64/src/hooks.rs` (new). Use shadowhook or LD_PRELOAD shim to intercept `open`/`openat` and redirect `/proc/…` paths to `/dev/vmproc`. | L (can defer) | (Acceptance criteria same as 3.11 — the inline hook is an implementation detail of the proc emulator.) |

#### Weeks 11–12: Wire it all together + reach launcher

| # | Task | Files | Effort | Acceptance criteria |
|---|---|---|---|---|
| 3.14 | **Wire `GsiExtractor` + `GsiInitPatcher` into `RomManager`** | `app/src/main/java/io/twoyi/utils/RomManager.java` (extend). If the ROM file is a GSI (detected by `.img` extension or magic bytes), call `GsiExtractor` + `GsiInitPatcher` instead of the legacy `unzip` path. | S | Selecting a GSI file in the UI triggers extraction + patching; the data dir contains a valid `fs/` tree after extraction. |
| 3.15 | **Spawn `libkr64.so` from `core.rs`** | `app/rs/src/core.rs` (extend). After `RomManager` extracts the GSI, spawn `libkr64.so` with the 7 argv args: `vmid`, `data_dir`, `rom_dir`, `kernel_path`, `config_path`, `log_level`, `socket_fd`. | S | The `kr64` daemon starts, creates the device tree, sets up the mount namespace, and execs `/system/bin/init`. Logcat shows the `init` boot sequence. |
| 3.16 | **MVP binder workaround: patch `system_server` to skip `publishService`** | `GsiInitPatcher.java` (extend). Patch `/system/framework/services.jar` (or the equivalent `system_server` init code) to skip the `publishService` calls that would fail without binder virtualisation. | M | `adb shell am start` inside the guest starts an activity (using host's ActivityManager as a fallback). The guest reaches the launcher. |
| 3.17 | **Verify boot to launcher** | Manual test on the codespace's redroid x86_64 + Android 11 GSI. | S | The guest's launcher is visible on the twoyi SurfaceView. `adb shell dumpsys activity activities` inside the guest shows the launcher activity. `BOOT_COMPLETED` event fires on `/dev/event`. |

### 6.3 Dependencies

The dependency graph for Phase 3 (simplified):

```
3.1 (device tree) ──┐
                    ├─► 3.15 (spawn kr64) ──► 3.17 (boot to launcher)
3.3 (GsiExtractor) ─┤                              ▲
                    ├─► 3.14 (wire RomManager) ────┤
3.4 (InitPatcher) ──┘                              │
                                                   │
3.5 (APEX pre-extract) ────────────────────────────┤
3.6 (gb ioctl) ──► 3.7 (GraphicBuffer register) ───┤
3.8 (graphics HAL) ────────────────────────────────┤
3.9 (keymaster/gatekeeper stubs) ──────────────────┤
3.10 (health/power/vibrator stubs) ────────────────┤
3.11 (proc emulator) ──────────────────────────────┤
3.12 (per-syscall emulation) ──────────────────────┤
3.16 (system_server patch) ────────────────────────┘
```

Critical path: **3.3 → 3.4 → 3.14 → 3.15 → 3.17**. Everything else can be parallelised.

### 6.4 What's deferred to Phase 4 (the MVP shortcuts)

The following are **explicitly skipped** for the MVP, per `GSI_BOOT_PLAN.md` §4.2. Each one is a Phase 4 task:

- **Binder virtualisation (§3.2)** — use the MVP workaround of patching `system_server` (task 3.16). The guest boots but every `getSystemService()` returns host services.
- **Seccomp filter (§3.4)** — install the filter but don't dispatch per-syscall (task 3.12 is deferred to Phase 4). The guest sees the host's `/proc` etc., which is wrong but won't crash immediately.
- **Full `/proc` emulator (§3.5)** — implement only `/proc/cmdline` and `/proc/version`. The rest can be the host's (until we hit a boot failure caused by it).
- **Inline hooking (§3.6)** — skip. The guest's `dlopen` will load from the host's `/system/lib64/`. May cause ABI mismatches (host is Android 11, guest GSI is Android 11 — should be OK if versions match).
- **Audio/camera/sensors/gps/wifi/telephony/bluetooth HALs (§3.9)** — all stubs for MVP.
- **APEX support (§2.6)** — pre-extract for MVP (task 3.5).

### 6.5 Phase 3 acceptance criteria (the MVP definition of done)

1. **The guest launcher is visible** on the twoyi SurfaceView, rendered through the open-source AOSP renderer.
2. **`BOOT_COMPLETED` fires** on the `/dev/event` socket within 60 seconds of `kr64` spawn.
3. **`adb shell` works** from inside the guest (via twoyi's in-container ADB).
4. **`adb shell lshal`** lists all stub HALs as `OK`.
5. **`adb shell cat /proc/cmdline`** returns `androidboot.hardware=twoyi …`.
6. **No closed-source blobs** in the APK (Phase 2 prerequisite holds).
7. **The whole boot sequence is reproducible** by: (a) downloading an Android 11 x86_64 GSI, (b) placing it in the twoyi data dir, (c) launching twoyi, (d) waiting ≤60 s.
8. **The boot state machine** in `TwoyiStatusManager` shows `BOOTING → BOOT_COMPLETED` (or `BOOT_FAILED` with a useful diagnostic).

### 6.6 Risk factors

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **Graphics HAL is harder than expected** (gralloc + SurfaceFlinger compositing is fiddly) | High | High — blocks the visible boot | Defer SurfaceFlinger compositing; boot to a "frozen first frame" first, then iterate. |
| **`system_server` patch is brittle** (each Android version's `services.jar` is different) | Medium | Medium — workaround may not work on all GSI versions | Pin the MVP to Android 11 x86_64 GSI only; document the patch as Android-11-specific. |
| **`/proc/self/maps` synthesis is wrong** (guest's `linker` sees impossible mappings and crashes) | Medium | Medium — silent boot failure | Start by passing through the host's `/proc/self/maps` unmodified; synthesise only when a specific failure is observed. |
| **APEX pre-extraction is incomplete** (some APEXes have nested payloads) | Medium | Low — boot hangs at `apexd` | Patch `apexd` to be a no-op for the MVP (task 3.5). |
| **Binder workaround breaks apps** (any app that calls `getSystemService(ACTIVITY_SERVICE)` and expects the guest's `ActivityManager`) | High | Medium — launcher works but apps don't | Accept for MVP. Document as a known limitation. Phase 4 fixes it properly. |
| **x86_64 GSI from `ci.android.com` is too new** (Android 13+ has Treble changes our stubs don't handle) | Medium | Medium — boot fails | Pin to Android 11 GSI for MVP. Phase 5 adds multi-version support. |

---

## 7. Phase 4 — Feature Parity with Virtual Master (Weeks 13–24)

**Goal:** Close the gap with Virtual Master. Implement binder virtualisation (the hardest piece), the full HAL suite (audio, sensor, camera, location, WiFi, phone, battery, network), and multi-VM support. After Phase 4, twoyi matches VM feature-for-feature on x86_64.

**Dependencies:** Phase 3 must have reached the launcher. The HAL work can proceed in parallel with binder virtualisation (only Phone + Network depend on binder — see `HAL_VIRTUALIZATION_ANALYSIS.md` §5.5).

### 7.1 Tasks

#### Weeks 13–16: Binder virtualisation + Display refactor

| # | Task | Files | Effort | Acceptance criteria |
|---|---|---|---|---|
| 4.1 | **Implement parcel parsing in `binder.rs`** | `app/rs/kr64/src/binder.rs` (extend). Parse the wire payload of `BC_TRANSACTION` (the `binder_transaction_data` struct's `data_ptr` and `offsets` fields). Implement `Parcel` reader/writer matching `frameworks/native/libs/binder/Parcel.cpp`. | L | `cargo test` in `kr64` includes a roundtrip test: encode a `Parcel` with `writeInt(42)` + `writeString("hello")`, decode it, assert the values match. |
| 4.2 | **Implement handle translation** | `app/rs/kr64/src/binder.rs` (extend). The `HandleTable` (already implemented) maps guest handles → host handles. In `forward_transaction_to_host`, translate the `target.handle` field and every `flat_binder_object` in the data buffer. | M | A guest `getService("activity")` call returns a usable host `IActivityManager` binder. |
| 4.3 | **Implement guest-side `libbinder.so` shim** | `app/rs/kr64/src/libbinder_shim/` (new crate, ~1000 LOC). LD_PRELOAD library that intercepts `ioctl(fd, BINDER_*, arg)` and translates to our wire framing on `/dev/binder` (which is a Unix socket, not a real char device). | L | Guest's `servicemanager` connects to our proxy. `adb shell service list` inside the guest lists services registered through our proxy. |
| 4.4 | **Implement Java-side `BinderService` + `setupBinder` JNI** | `app/src/main/java/io/twoyi/BinderService.java` (new, ~400 LOC). Mirror VM's `com.android.vmcore.service.BinderService`: reflect `IActivityManager`, install `java.lang.reflect.Proxy`, call native `setupBinder(vmId, …)`, `bindService(BinderService.class)`. | M | `TwoyiApplication.onCreate` calls `BinderService.setupBinder(0, …)`; the per-VM `/vm0/dev/binder` is created; guest's `servicemanager` registers with it. |
| 4.5 | **Remove the `system_server` patch** | `GsiInitPatcher.java` (revert task 3.16). | S | Guest's `system_server` registers its services normally; `adb shell am start` starts the guest's activity (not the host's). |
| 4.6 | **Refactor `Renderer.java` → `DisplayService.java`** | `app/src/main/java/io/twoyi/DisplayService.java` (new, ~200 LOC). Adopt VM's per-VM pointer pattern: `nativeAddSurface(long ptr, int surfaceId, Surface, w, h, rot)`. Rust side: `core.rs` + `renderer_bindings.rs` take a `ptr` arg. | M | Multiple `DisplayService` instances can coexist (one per VM). `DisplayService.nativeGetFPS()` returns per-instance FPS. |

#### Weeks 17–20: HAL proxies (audio, sensor, battery, location, WiFi, camera)

| # | Task | Files | Effort | Acceptance criteria |
|---|---|---|---|---|
| 4.7 | **Implement `HALManager` dispatcher** | `app/src/main/java/io/twoyi/HALManager.java` (new, ~500 LOC). Single class with `long mNativePtr` and ~30 private methods called back from native via JNI (`AudioStart`, `AudioStop`, `EnableSensors`, `DisableSensors`, `CameraConnect`, …). Rust side: `nativeSetup` returns `*mut HalDispatcher`. | M | A test HAL call (e.g., `nativeSetup(0)` → `AudioStart(44100, 2)` → `AudioStop()`) round-trips successfully. |
| 4.8 | **Audio HAL** | `app/rs/src/audio.rs` (new, ~200 LOC), `app/rs/kr64/src/devices.rs` (extend with `create_audio_device`), `app/src/main/java/io/twoyi/hal/AudioService.java` (new, ~300 LOC). Port of VM's `com.android.vmcore.hal.AudioService`. | M | `adb shell dumpsys audio` inside the guest returns without crashing. The guest can play an audio file (audible on the host speaker). |
| 4.9 | **Sensor HAL (12 types)** | `app/rs/src/sensor.rs` (new, ~150 LOC), `app/rs/kr64/src/devices.rs` (extend with `create_sensor_device`), `app/src/main/java/io/twoyi/hal/SensorService.java` (new, ~400 LOC). Port of VM's. | M | `adb shell dumpsys sensorservice` inside the guest lists 12 sensors. Rotating the host device rotates the guest's display (if an accelerometer app is running). |
| 4.10 | **Battery HAL** | `app/rs/src/battery.rs` (new, ~100 LOC), `app/src/main/java/io/twoyi/hal/BatteryService.java` (new, ~150 LOC). File-based: write stats to `/sys/class/power_supply/battery/uevent`. | S | `adb shell dumpsys battery` returns the host's battery level. |
| 4.11 | **Location HAL** | `app/rs/src/location.rs` (new, ~100 LOC), `app/rs/kr64/src/devices.rs` (extend with `create_location_socket`), `app/src/main/java/io/twoyi/hal/LocationService.java` (new, ~300 LOC). | M | `adb shell dumpsys location` inside the guest shows the host's GPS location. |
| 4.12 | **WiFi HAL** | `app/rs/src/wifi.rs` (new, ~150 LOC), `app/rs/kr64/src/devices.rs` (extend with `create_wifi_socket`), `app/src/main/java/io/twoyi/hal/WiFiService.java` (new, ~250 LOC). wpa_supplicant control protocol. | M | `adb shell dumpsys wifi` inside the guest shows the host's WiFi scan results. |
| 4.13 | **Camera HAL** | `app/rs/src/camera.rs` (new, ~250 LOC), `app/rs/kr64/src/devices.rs` (extend with `create_camera_device`), `app/src/main/java/io/twoyi/hal/CameraService.java` (new, ~600 LOC). Camera1 API proxy. | L | A camera app inside the guest shows the host's camera preview. |

#### Weeks 21–24: Phone, Network, multi-VM

| # | Task | Files | Effort | Acceptance criteria |
|---|---|---|---|---|
| 4.14 | **Phone HAL (SIM/SMS/Call)** | `app/rs/src/phone.rs` (new, ~300 LOC), `app/rs/kr64/src/devices.rs` (extend with `create_rild_sockets`), `app/src/main/java/io/twoyi/hal/PhoneService.java` + `phone/` sub-package (new, ~2000 LOC). Standard Android RIL protocol. | L | A dialer app inside the guest can place a call (routed through the host's TelephonyManager). SMS send/receive works. **Depends on 4.4 (binder virtualisation).** |
| 4.15 | **Network HAL (tun0)** | `app/rs/src/netlink.rs` (new, ~400 LOC), `app/rs/kr64/src/devices.rs` (extend with `create_netlink_socket`), `app/src/main/java/io/twoyi/hal/NetlinkManager.java` (new, ~200 LOC). RTNETLINK emulation. | L | The guest has its own IP address (different from the host's). `adb shell ip addr` inside the guest shows `tun0`. **Depends on 4.4.** |
| 4.16 | **Multi-VM support** | `app/src/main/java/io/twoyi/VMManager.java` (new, ~400 LOC), `app/src/main/java/io/twoyi/VMInstance.java` (new, ~800 LOC), per-VM data dirs (`vm/vm0/`, `vm/vm1/`, …, `vm/vm3/`), per-VM `SharedPreferences`, per-VM task affinities (`VMStartActivity0..3`). | L | Up to 4 concurrent VMs can boot. Each has its own launcher, its own app data, its own IP (if 4.15 is done). Switching between VMs via the host UI preserves each VM's state. **Depends on 4.6 (per-VM renderer pointer).** |

### 7.2 Dependencies

- **4.3 (libbinder shim) depends on 4.1, 4.2** — the shim needs the parcel parser and handle translator.
- **4.4 (Java BinderService) depends on 4.3** — the Java side calls `setupBinder` which assumes the shim is loaded.
- **4.5 (remove system_server patch) depends on 4.4** — only safe once binder virtualisation actually works.
- **4.14 (Phone) and 4.15 (Network) depend on 4.4** — these HALs need the guest's `servicemanager` to be per-VM.
- **4.16 (multi-VM) depends on 4.6 (per-VM renderer pointer)** — without per-VM pointers, the renderer is a global singleton.
- **4.6 through 4.13 are independent** — can be done in parallel by different contributors.

### 7.3 Phase 4 acceptance criteria

1. **Binder virtualisation works**: `adb shell am start` inside the guest starts the guest's activity (not the host's). `adb shell service list` lists guest-registered services.
2. **All 10 HALs work**: audio, sensor, camera, location, WiFi, phone, battery, network, display, input. Each has a working demo (e.g., play audio, take a photo, get GPS coordinates).
3. **Multi-VM works**: 4 concurrent VMs boot, each with its own launcher and app data.
4. **No MVP shortcuts remain**: the `system_server` patch is gone, seccomp dispatches per-syscall, `/proc` is fully synthesised, APEX support is real (not pre-extracted).
5. **Feature parity with Virtual Master**: every feature listed in `VM_JAVA_ANALYSIS.md` §5.4 is implemented.

### 7.4 Risk factors

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **Binder virtualisation is harder than estimated** (parcel format, handle translation, FD passing, death notifications) | High | High — blocks 4.5, 4.14, 4.15 | Budget extra time. If 4 weeks isn't enough, ship Phase 4 without Phone/Network (defer to Phase 5). |
| **Guest-side `libbinder.so` shim is fragile** (LD_PRELOAD on bionic is tricky) | Medium | High — the whole binder virtualisation depends on it | Use `dlopen` + `dlsym(RTLD_NEXT, "ioctl")` instead of LD_PRELOAD. Or patch `libbinder.so` directly (binary patch the `ioctl` call sites). |
| **Camera frame format negotiation fails** (guest asks for a YUV format the host camera doesn't emit) | High | Medium — camera doesn't work | Support only NV21 + JPEG initially; convert others in Java (per `HAL_VIRTUALIZATION_ANALYSIS.md` §3.5). |
| **Multi-VM resource contention** (4 VMs × 1 GB RAM each = 4 GB; many host devices can't handle it) | Medium | Medium — multi-VM crashes on low-RAM devices | Default to 1 VM; let the user opt into multi-VM in Settings. Document the RAM requirement. |
| **Phase 4 takes longer than 12 weeks** | High | Medium — schedule slip | Phase 4 is parallelisable. Recruit contributors for the independent tasks (4.6–4.13). |

---

## 8. Phase 5 — Advanced Features (Weeks 25+)

**Goal:** Forward-looking work that takes twoyi beyond Virtual Master parity. These are research-grade projects with significant uncertainty.

**Dependencies:** Phase 4 complete (or at least the relevant sub-component — e.g., KVM path doesn't depend on multi-VM).

### 8.1 Tasks

| # | Task | Files | Effort | Acceptance criteria |
|---|---|---|---|---|
| 5.1 | **KVM path (alternative architecture)** | `app/rs/kvm/` (new crate). Use `crosvm` (Rust, https://crosvm.dev/) or QEMU to boot a minimal Linux kernel + the GSI's ramdisk in a real VM. Display: `crosvm`'s GPU passthrough or `virglrenderer`. Input: `crosvm`'s virtual input devices. Network: `crosvm`'s virtual network. Binder: native (the guest has its own kernel binder). | L (research) | The codespace's KVM (AMD EPYC 7763, `/dev/kvm` accessible) boots an Android 11 GSI via crosvm. The guest reaches the launcher. Display renders to a host Surface. **This is an alternative to the container path, not a replacement.** |
| 5.2 | **x86_64 native GSI distribution** | `app/src/main/java/io/twoyi/utils/GsiDownloader.java` (new). Download pre-built x86_64 GSIs from `ci.android.com` (or a mirror) with on-the-fly progress + integrity check. Cache by build ID. | M | The twoyi UI offers "Download Android 11 x86_64 GSI" and "Download Android 13 x86_64 GSI" options. The download completes (or fails with a useful error) and the GSI is automatically extracted + booted. |
| 5.3 | **Cloud ROM distribution (VM-style)** | `app/src/main/java/io/twoyi/utils/RomCatalog.java` (new), `app/src/main/java/io/twoyi/utils/AesInputStream.java` (new). Mirror VM's `RomConfig` JSON schema + `api.virtualmaster.app`-style server. On-the-fly AES-128-ECB decryption with a published key. | L | A user can choose from multiple ROM versions (Android 9, 11, 13) in the UI. The ROM downloads, decrypts, and boots. Document the server-side API contract so anyone can host a ROM catalog. |
| 5.4 | **ARM binary translation on x86_64** (research) | Investigate `libhoudini` (closed-source), `libndk_translation` (Google's, closed-source), or build a custom ARM-to-x86_64 translator using `unicorn-engine` (Rust bindings). | L (research) | An arm64 GSI boots on an x86_64 host via binary translation. Performance is acceptable (>50% of native for compute-bound apps). |
| 5.5 | **Multi-version GSI support** | Extend `GsiExtractor` + `GsiInitPatcher` to handle Android 9, 11, 13, 14 GSIs. Each version has different init.rc syntax, VINTF manifest format, APEX layout, binder protocol version. | L | Twoyi boots GSIs from Android 9 through Android 14. The user picks the version in the UI. |
| 5.6 | **Real APEX support** | `app/rs/kr64/src/apex.rs` (new). Implement loop-mount of `apex_payload.img` (or bind-mount of pre-extracted APEX dirs). Patch `apexd` to use our loop-mount instead of the kernel's. | M | `adb shell ls /apex/com.android.art.release/` inside the guest shows the mounted APEX contents (not a pre-extracted directory). |
| 5.7 | **Performance: GPU passthrough** | Investigate whether `libOpenglRender_aosp.so` can use the host's hardware GL driver directly (instead of going through the emugl decoder). | L (research) | A GLBenchmark run inside the guest achieves >50% of native host GL performance. |
| 5.8 | **Cloud sync / VM backup** | `app/src/main/java/io/twoyi/utils/VmBackup.java` (new). Backup a VM's `fs/` + `data/` to a cloud provider (Google Drive, Dropbox). Restore on another device. | M | A user can back up VM #1 on device A, restore on device B, and continue using the same guest apps with their data. |
| 5.9 | **Magisk / Xposed / GApps plugin support** | `app/src/main/java/io/twoyi/utils/PluginInstaller.java` (new). Mirror VM's `play.zip` / `magisk.zip` / `xposed.zip` / `superuser.zip` plugin system. AES-128-ECB decryption with key `%z89aviCM0KkbEs9` (documented in `VM_ROM_ANALYSIS.md` §2). | M | A user can install GApps, Magisk, Xposed, or Superuser into a VM from the UI. |
| 5.10 | **SELinux enforcing mode** | Currently the MVP runs SELinux permissive. Implement the SELinux policy patches needed to run enforcing. | L (research) | `adb shell getenforce` inside the guest returns `Enforcing`. No AVC denials in `dmesg`. |

### 8.2 Phase 5 acceptance criteria

Each task has its own acceptance criteria above. Phase 5 as a whole is "done" when at least 3 of the tasks are landed and the project has a published v1.0 release.

### 8.3 Risk factors

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **KVM path doesn't work on most consumer devices** (no `/dev/kvm` on non-rooted phones) | High | High — KVM path is codespace/desktop-only | Document clearly. The container path (Phases 3–4) is the primary architecture. |
| **ARM binary translation is too slow / too legally complex** | High | Medium — Phase 5 task 5.4 is research-only | Don't commit to it. If `libhoudini`/`libndk_translation` aren't available, drop 5.4. |
| **Cloud ROM distribution has legal issues** (distributing Google's GApps, etc.) | Medium | Medium — 5.3 + 5.9 may need legal review | Only distribute AOSP-derived images. Document the licensing clearly. Don't ship GApps in the default catalog. |
| **Multi-version GSI support explodes test matrix** | High | Medium — CI cost | Test only Android 11 + Android 13 in CI. Other versions are best-effort. |

---

## 9. Key Architectural Decisions

This section documents the choices we've made and why, so contributors understand the design philosophy and don't re-litigate settled questions.

### 9.1 Container path over KVM path

**Decision:** Twoyi's primary architecture is the **container path** — the guest shares the host kernel and is isolated via `chroot` + `seccomp` + mount namespaces + a userspace "kernel replacement" daemon (`libkr64.so`). The KVM path (Phase 5 task 5.1) is an alternative, not a replacement.

**Why:** This is the architectural direction twoyi was already on (the original `cyanmint/twoyi` worked this way for arm64). Virtual Master proves it can boot a Treble GSI without KVM and without root. The container path is also strictly more portable — it works on any unprivileged Android app process, including non-rooted consumer devices. KVM is only available on rooted devices or Linux desktops.

**Trade-off:** The container path is harder to implement (binder virtualisation, `/proc` emulation, seccomp filter) but easier to deploy. The KVM path is easier to implement (the guest kernel does everything) but harder to deploy (requires `/dev/kvm`).

**Source:** `GSI_BOOT_PLAN.md` §0, §5.5; `VM_KR64_ANALYSIS.md` (full analysis of VM's container path).

### 9.2 Rust + JNI over C++ throughout

**Decision:** All new native code is written in **Rust** (MPL-2.0). The Java side calls into Rust via JNI. The only C++ in the project is the AOSP-derived `libOpenglRender.so` (Apache-2.0, kept as-is because it's a faithful rebuild of upstream code).

**Why:**
- **Memory safety:** The kernel-replacement daemon handles untrusted guest input (binder transactions, `/proc` reads, ioctl payloads). Rust's ownership model eliminates whole classes of bugs (use-after-free, buffer overflow) that would be security-critical in this context.
- **Cross-platform unit testing:** Rust crates compile and test on Linux x86_64 host, so we can run `cargo test` in CI without an Android device. The `kr64` crate's `build.rs` gates Android-specific linker flags on `target_os = "android"` so the same code works on host and target (see `KR64_SKELETON.md` §3.1).
- **No external deps in `kr64`:** The `kr64` crate intentionally depends on `libc` only (no `log`, no `once_cell`, no `nix`) so it can be statically analysed and audited. This matches the security-critical nature of the kernel-replacement daemon.
- **Modern toolchain:** Rust stable + NDK r27c + clang 18 is the current state of the art; the original twoyi required NDK r22 or older (which is no longer supported).

**Trade-off:** Rust has a steeper learning curve than C++ for contributors unfamiliar with it. We mitigate this with the per-crate convention: the main `twoyi` crate is allowed to use `log`, `jni`, `ndk`, `once_cell`, etc. (matching what contributors expect), while `kr64` is `libc`-only (documented in `CONTRIBUTING.md` §3).

**Source:** `CONTRIBUTING.md` §3; `KR64_SKELETON.md` §3.1; `ARCHITECTURE.md`.

### 9.3 PIE-as-cdylib (directly-executable `.so`)

**Decision:** `libtwoyi.so` and `libkr64.so` are built as `cdylib` (shared library) AND are directly executable as PIE binaries, thanks to a `.interp` section injected by `app/rs/src/interp.c` (for `libtwoyi.so`) and `app/rs/kr64/interp.c` (for `libkr64.so`).

**Why:** Android doesn't allow apps to ship arbitrary executables in `jniLibs/` (only `.so` files are extracted). But twoyi needs to `exec` the loader (`libloader.so`) and the kernel-replacement daemon (`libkr64.so`) as separate processes. The `.interp` trick makes a `.so` also a valid ELF executable — Android's package installer sees a `.so` and extracts it; `execve` sees a PIE binary and runs it.

**Trade-off:** The trick is non-obvious and requires a custom `build.rs` to inject the `.interp` section. But it's proven (twoyi's original `libloader.so` used it) and well-documented in `PIE_IMPLEMENTATION.md`.

**Source:** `PIE_IMPLEMENTATION.md`; `app/rs/src/interp.c`; `app/rs/kr64/interp.c`; `KR64_SKELETON.md` §3.2.

### 9.4 Open-source everything (no closed-source blobs)

**Decision:** Every native binary shipped in the APK must be built from open-source code with a documented provenance chain. The project will not accept new closed-source blobs.

**Why:**
- **Legal:** Distributing closed-source binaries without source is a copyright issue (the original twoyi blobs were derived from AOSP Apache-2.0 code but stripped of attribution). The fork's policy is "if we can't rebuild it from source, we don't ship it."
- **Practical:** Closed-source blobs can't be patched (e.g., to add x86_64 support), can't be audited for security, and can't be debugged when they crash. The x86_64 SIGABRT crash (commit `7664c66`) was caused by the legacy `libOpenglRender.so` being arm64-only — once we rebuilt from AOSP source, x86_64 worked.
- **Reproducibility:** The reproducible-build CI job (Phase 2 task 2.6) ensures every binary can be rebuilt bit-for-bit (or symbol-for-symbol) from source.

**Trade-off:** Building from source is slower than shipping a pre-built blob. The AOSP `libOpenglRender.so` build took ~6 hours to set up the first time (per `AOSP_BUILD_RESULTS.md`). But the build is now automated in `app/rs/openglrenderer/build.sh` and runs in CI.

**Source:** `CONTRIBUTING.md` §3 (C/C++ section); `AOSP_BUILD_RESULTS.md`; `TWOYI_DISASSEMBLY_ANALYSIS.md` Phase 4.

### 9.5 Honest status reporting (no overclaims)

**Decision:** Every claim of "✅ done" must be backed by a test, a CI run, or a documented on-device verification. "Inferred from analysis" is explicitly distinguished from "verified working."

**Why:** The project has a documented history of overclaims — `TWOYI_HONEST_STATUS.md` corrected an earlier report that claimed the container had booted based on a VLM screenshot analysis (it was actually the Android emulator's own launcher). This wasted contributor time and eroded trust.

**Trade-off:** Being conservative makes the project look less complete than it is. But it's better than the alternative — contributors discovering that "done" actually means "we think it should work."

**Source:** `TWOYI_HONEST_STATUS.md`; `CONTRIBUTING.md` §4 ("Honest test reporting"); `PROJECT_SUMMARY.md` Appendix A (Verified vs Theoretical).

### 9.6 Defer binder virtualisation for the MVP

**Decision:** The GSI boot MVP (Phase 3) explicitly skips binder virtualisation. The workaround is to patch `system_server` to skip `publishService` calls. Binder virtualisation is implemented properly in Phase 4.

**Why:** Binder virtualisation is the single hardest piece of the GSI boot plan (per `GSI_BOOT_PLAN.md` §4.3). The parcel format, handle translation, FD passing, death notifications, and per-Android-version protocol differences are each a multi-week project. If we block the MVP on it, we never reach the launcher.

The MVP workaround (patching `system_server`) is hacky but lets us verify the rest of the stack (kernel replacement, graphics HAL, `/proc` emulator, stub HALs) works end-to-end. Once that's proven, binder virtualisation becomes a focused Phase 4 task with a working test bed.

**Trade-off:** The MVP's `getSystemService()` calls return host services, not guest services. Apps that depend on the guest's `ActivityManager` / `PackageManager` won't work. But the launcher will boot, and we'll have a visible demonstration that the GSI boot works.

**Source:** `GSI_BOOT_PLAN.md` §4.2; `BINDER_SKELETON.md` §0; `PROJECT_SUMMARY.md` §8.5.

### 9.7 `kr64` skeleton mirrors VM's `libkr64.so` architecture (not its code)

**Decision:** The `kr64` Rust crate is a clean-room re-implementation of VM's `libkr64.so` architecture, NOT a port of VM's code. We use the decoded strings, the device inventory, the protocol constants, and the architectural patterns from `VM_KR64_ANALYSIS.md`, but every line of Rust is original.

**Why:**
- **Licensing:** VM's `libkr64.so` is closed-source. Even decompiled code is a derivative work. We can't ship it.
- **Clarity:** VM's binary is heavily OLLVM-obfuscated (control-flow flattening, XOR'd strings, stripped symbols). A clean-room implementation in idiomatic Rust is far more maintainable.
- **Testability:** The Rust skeleton has 26 unit tests; VM's binary has zero.

**Trade-off:** We lose any subtle behaviour VM's binary has that isn't documented in `VM_KR64_ANALYSIS.md`. The risk is that some guest path depends on undocumented VM behaviour. Mitigation: the `KR64_SKELETON.md` §5 "What's NOT here yet" list explicitly tracks every known gap.

**Source:** `KR64_SKELETON.md` §0, §2; `VM_KR64_ANALYSIS.md`; `CONTRIBUTING.md` §3 (Rust section).

### 9.8 Per-VM data layout (mirrors VM)

**Decision:** Twoyi will adopt VM's per-VM data layout: `<dataDir>/vm/vmN/fs/` (extracted ROM), `<dataDir>/vm/vmN/dev/event` (IPC socket), `<dataDir>/vm/vmN/dev/binder` (virtual binder), `<dataDir>/vm/vmN/dev/qemu_pipe` (GL transport), `<dataDir>/lib64/` (native libs), `shared_prefs/vm_config_N.xml`.

**Why:** This layout supports multi-VM (up to 4 concurrent VMs, matching VM's `VMStartActivity0..3` with `taskAffinity=.vm0..3`). It also makes the data dir self-describing — every VM has its own `fs/`, `dev/`, and config, so backup/restore is a simple file copy.

**Trade-off:** The current twoyi layout is flat (`<dataDir>/rootfs/`, `<dataDir>/dev/`). Migrating requires a one-time data migration for existing users. Phase 4 task 4.16 implements this.

**Source:** `VM_JAVA_ANALYSIS.md` §6 (point 9); `GSI_BOOT_PLAN.md` §2.4 (device paths).

### 9.9 The boot state machine (11 states)

**Decision:** Twoyi will adopt VM's explicit 11-state boot state machine in `TwoyiStatusManager`: STOPPED, CHECKING_ENV, INSTALLING, STARTING_SVC, BOOTING, RUNNING, BOOT_COMPLETED, SHUTDOWN, ENV_FAILED, INSTALL_FAILED, BOOT_FAILED. Each transition fires an EventBus `VMStatusEvent`.

**Why:** The current twoyi boot flow parses log lines to determine state, which is brittle (log format changes break the parser) and gives the UI no useful feedback during a long boot. An explicit state machine makes the UI responsive and makes failures diagnosable.

**Trade-off:** Refactoring `TwoyiStatusManager` is a Medium-sized task (Phase 1 task 1.6) that touches the UI. But it's a one-time cost with permanent benefit.

**Source:** `VM_JAVA_ANALYSIS.md` §2.3; `PROJECT_SUMMARY.md` §8.9.

---

## 10. How to Contribute

### 10.1 Start here

Read these in order:

1. **[`README.md`](../README.md)** — project overview, quick start, build instructions.
2. **[`ARCHITECTURE.md`](../ARCHITECTURE.md)** — the 3-layer architecture (Java app → `libtwoyi.so` → `libOpenglRender.so` → guest), the PIE hack, the boot flow.
3. **[`CONTRIBUTING.md`](../CONTRIBUTING.md)** — development environment, code style, testing, PR process.
4. **This document** — what to work on next.
5. **[`GSI_BOOT_PLAN.md`](GSI_BOOT_PLAN.md)** — the file-and-function-level plan for Phase 3.

### 10.2 Good first issues (effort: S, no architectural discussion required)

These are scoped tightly enough for a new contributor to land in a week. Each has a complete design doc — no need to "open an issue first."

1. **Drop-in test the AOSP renderer on a real arm64 device** (Phase 1 task 1.1). Just install the APK on a phone and verify it boots. ~1 day. Requires a physical arm64 device.

2. **Add `set_emugl_*` no-op stubs to the AOSP renderer** (Phase 1 task 1.5). Extend the patch series in `download/port_files/` and rebuild via `app/rs/openglrenderer/build.sh`. ~1 day.

3. **Port `set_emugl_logger` to the Rust `log` crate** (Phase 1 task 1.7). Wire the emugl logger callback to twoyi's existing `log` macros. ~1 day.

4. **Wire `kr64` into the boot flow** (Phase 1 task 1.4). Add `kr64` as a workspace member of `app/rs/Cargo.toml`, extend `build_rs.sh`, add the spawn call in `core.rs`. ~2 days. Good for learning the kr64 codebase.

5. **Extend `kr64` device tree to 20+ devices** (Phase 3 task 3.1). Each device is ~20 lines following the existing `bind_unix_socket` helper. Can be done one device at a time. ~3 days total, but each device is ~30 minutes.

6. **Add `mknodat(S_IFSOCK)` capability-gated path** (Phase 3 task 3.2). Add a `CAP_MKNOD` check + switch the device-creation path. ~1 day.

7. **Implement the battery HAL** (Phase 4 task 4.10). File-based, no real-time complexity. ~1 day. Good first HAL.

8. **Open-source `libadb.so`** (Phase 2 task 2.1). Build `packages/modules/adb` from AOSP source, rename to `libadb.so`, ship in `jniLibs/`. ~1 week. Documented in `TWOYI_DISASSEMBLY_ANALYSIS.md` Phase 3.

9. **Audit `libloader.so` (Rust) against the legacy disassembly** (Phase 2 task 2.3). Read `TWOYI_DISASSEMBLY_ANALYSIS.md` §1 and compare against `app/rs/loader/src/lib.rs`. ~2 days. Pure reading + issue filing.

10. **Document the provenance chain in `OPEN_SOURCE_LIBRARIES.md`** (Phase 2 task 2.5). Rewrite the file to list every native binary with source URL, license, build command, and verification step. ~2 days. Good for a contributor who wants to understand the build system.

### 10.3 Medium-effort projects (effort: M, may need design discussion)

These are listed in `CONTRIBUTING.md` §6 and reproduced here with roadmap cross-references:

- **Build an x86_64 rootfs from AOSP** (Phase 1 task 1.3, Roadmap item #2). Unblocks all x86_64 end-to-end testing.
- **GSI extractor** (Phase 3 task 3.3, Roadmap item #3). Implement sparse-ext4 → raw ext4 → directory tree.
- **GSI init patcher** (Phase 3 task 3.4, Roadmap item #4). Patch `init.rc`, `build.prop`, VINTF manifests.
- **Extend `GraphicBuffer::Main` to register buffers** (Phase 3 task 3.7, Roadmap item #5). Reverse-engineer the legacy `GraphicBufferHandler::main`.

### 10.4 Hard problems (need design discussion first)

Open an issue to discuss approach before starting:

- **Binder virtualisation** (Phase 4 tasks 4.1–4.4, Roadmap item #8). The hardest single piece of the GSI boot plan.
- **Graphics HAL** (Phase 3 tasks 3.6, 3.8, Roadmap item #5). `/dev/gb` + gralloc allocator/mapper/composer.
- **Multi-VM support** (Phase 4 task 4.16, Roadmap item #11). Architectural refactor of the renderer + data layout.
- **KVM path** (Phase 5 task 5.1). Alternative architecture — discuss whether to pursue before committing.

### 10.5 Non-code contributions

- **Documentation** — `ARCHITECTURE.md`, `README.md`, and the `download/` analysis reports always need tightening. Typos, dead links, and unclear explanations are fair game.
- **Bug reproduction** — install the latest APK from CI on a real device, try to boot, and file detailed issues with `adb logcat` output and tombstones.
- **Translation** — `README_CN.md` is out of date with the new README. A fresh Chinese translation would be welcome.
- **ROM testing** — try booting different Android versions (9, 11, 13) and file compatibility reports.

### 10.6 Communication

- **[GitHub Discussions](https://github.com/Disable-OP/twoyi/discussions)** — general questions.
- **[GitHub Issues](https://github.com/Disable-OP/twoyi/issues)** — bugs and feature requests.
- **Pull requests** — against `Disable-OP/twoyi:main`. See `CONTRIBUTING.md` §5 for the PR process.

---

## 11. Glossary

| Term | Meaning |
|---|---|
| **AOSP** | Android Open Source Project. The source tree at `https://android.googlesource.com/`. |
| **APEX** | Android Pony EXpress. A ZIP containing an `apex_payload.img` (ext4 or sparse-ext4). Mountable mini-image introduced in Android 10. |
| **Binder** | Android's IPC mechanism. The kernel driver at `/dev/binder`. Treble additionally uses `/dev/hwbinder` (HIDL HALs) and `/dev/vndbinder` (vendor binder). |
| **Container path** | Twoyi's primary architecture: guest shares the host kernel, isolated via `chroot` + `seccomp` + mount namespaces + `libkr64.so`. Opposite of the KVM path. |
| **Crosvm** | Rust-based VMM (Virtual Machine Monitor) from ChromiumOS. The KVM-path alternative (Phase 5 task 5.1). |
| **Emugl** | AOSP's "emulator OpenGL" renderer. The `libOpenglRender.so` source is at `platform/sdk` commit `7a712acc`. |
| **GSI** | Generic System Image. A Treble `system.img` that conforms to the HAL interface contract. Boots on any device whose `vendor` partition implements the matching VINTF manifest. |
| **HAL** | Hardware Abstraction Layer. Android's mechanism for talking to hardware. Each HAL is declared in a VINTF manifest XML. |
| **HIDL** | HAL Interface Definition Language. The Treble HAL IPC format (replaced by AIDL in Android 11+). |
| **KVM** | Kernel-based Virtual Machine. Linux's hypervisor. Available on the codespace (AMD EPYC 7763) but not on most consumer Android devices. |
| **`libkr64.so`** | Virtual Master's kernel-replacement daemon. Standalone ELF executable disguised as `.so`, launched by `libkrloader64.so` (custom dynamic linker). Twoyi's reimplementation is `app/rs/kr64/`. |
| **`libvm.so`** | Virtual Master's single native library. Contains JNI bindings, OpenGL render server, binder virtualisation. Heavily OLLVM-obfuscated. |
| **OLLVM** | Obfuscating LLVM. The tool that VM used to obfuscate `libvm.so` and `libkr64.so` (control-flow flattening, XOR'd strings, stripped symbols). |
| **PIE** | Position-Independent Executable. Twoyi uses a `.interp` trick to make `.so` files also directly executable. |
| **QEMU pipe** | The `/dev/qemu_pipe` Unix socket. SurfaceFlinger writes GL commands here; the host `libOpenglRender.so` reads them. |
| **Seccomp** | Linux's secure computing mode. A BPF filter that traps or blocks syscalls. Twoyi uses it with a `SIGSYS` handler to emulate blocked syscalls. |
| **Shadowhook** | ByteDance's inline-hooking library (https://github.com/bytedance/android-inline-hook). VM uses it to intercept `open`/`openat`/`mount` in the guest. |
| **Treble** | Android 8.0's Project Treble. Split `system.img` into framework + vendor + product + system_ext partitions, with HALs declared in VINTF manifests. |
| **VINTF** | Vendor Interface. The XML manifest at `/vendor/etc/vintf/manifest/*.xml` that declares which HALs a device implements. |
| **VM** | Virtual Master. The competing closed-source Android-in-Android app (`com.clone.android.dual.space`). Reverse-engineered in six analysis reports in `download/`. |

---

## 12. References

### 12.1 Twoyi project files

- [`README.md`](../README.md) — project overview, quick start, build instructions.
- [`ARCHITECTURE.md`](../ARCHITECTURE.md) — 3-layer architecture, PIE hack, boot flow.
- [`CONTRIBUTING.md`](../CONTRIBUTING.md) — development setup, code style, PR process.
- [`CHANGELOG.md`](../CHANGELOG.md) — semantic versioning + commit log.
- [`PIE_IMPLEMENTATION.md`](../PIE_IMPLEMENTATION.md) — how `libtwoyi.so` was made into a PIE executable.
- [`app/rs/kr64/`](../app/rs/kr64/) — kernel-replacement daemon skeleton (Rust).
- [`app/rs/src/core.rs`](../app/rs/src/core.rs) — current guest spawn.
- [`app/rs/src/input.rs`](../app/rs/src/input.rs) — current input system.
- [`app/rs/openglrenderer/src/pipe.rs`](../app/rs/openglrenderer/src/pipe.rs) — current `/dev/qemu_pipe` server.
- [`app/src/main/jniLibs/x86_64/libOpenglRender_aosp.so`](../app/src/main/jniLibs/x86_64/libOpenglRender_aosp.so) — AOSP-built x86_64 renderer.

### 12.2 Analysis reports in `download/`

| File | What it covers |
|---|---|
| [`PROJECT_SUMMARY.md`](PROJECT_SUMMARY.md) | The definitive state-of-the-project write-up (968 lines). Every claim traceable to a commit, file, or analysis report. |
| [`GSI_BOOT_PLAN.md`](GSI_BOOT_PLAN.md) | 9-section roadmap for booting a Treble GSI inside twoyi. The authoritative design doc for Phase 3. |
| [`HAL_VIRTUALIZATION_ANALYSIS.md`](HAL_VIRTUALIZATION_ANALYSIS.md) | Technical analysis of every HAL VM virtualizes, with a build plan for twoyi. The authoritative design doc for Phase 4. |
| [`KR64_SKELETON.md`](KR64_SKELETON.md) | Design doc + follow-up task list for the `kr64` skeleton. |
| [`BINDER_SKELETON.md`](BINDER_SKELETON.md) | Design doc + follow-up task list for the binder virtualisation skeleton. |
| [`TWOYI_HONEST_STATUS.md`](TWOYI_HONEST_STATUS.md) | Honest assessment of x86_64 emulator testing. Corrects an earlier overclaim. |
| [`TWOYI_DISASSEMBLY_ANALYSIS.md`](TWOYI_DISASSEMBLY_ANALYSIS.md) | Disassembly of legacy twoyi native blobs. 4-phase implementation plan for open-source replacement. |
| [`TWOYI_FINAL_REPORT.md`](TWOYI_FINAL_REPORT.md) | Earlier summary report (some claims superseded by `PROJECT_SUMMARY.md`). |
| [`AOSP_BUILD_RESULTS.md`](AOSP_BUILD_RESULTS.md) | Full report of the AOSP-source build of `libOpenglRender.so`. |
| [`AOSP_VS_LEGACY_COMPARISON.md`](AOSP_VS_LEGACY_COMPARISON.md) | Symbol-level comparison between AOSP emugl source and legacy twoyi blob. |
| [`FUNCTION_LEVEL_COMPARISON.md`](FUNCTION_LEVEL_COMPARISON.md) | Function-logic comparison. Found 7 categories of logic differences. |
| [`PORT_RESULTS.md`](PORT_RESULTS.md) | Report of porting `startGBServer` + `dl*_ex` + `GraphicBuffer` to the AOSP build. |
| [`VM_JAVA_ANALYSIS.md`](VM_JAVA_ANALYSIS.md) | Decompile of VM's Java code. Documents the 11-state boot machine, two-stage task pipeline, three IPC channels, 12 HAL services. |
| [`VM_KR64_ANALYSIS.md`](VM_KR64_ANALYSIS.md) | Deep analysis of VM's `libkr64.so`. 187 imports, 24 init_array constructors, seccomp filter, `/proc` emulation, 20+ virtual devices. |
| [`VM_ROM_ANALYSIS.md`](VM_ROM_ANALYSIS.md) | Analysis of VM's APK assets. AES-128-ECB key, ROM catalog, Treble paths. |
| [`VM_DEEP_DISASSEMBLY.md`](VM_DEEP_DISASSEMBLY.md) | Deep disassembly of VM's `libvm.so`. Locates `startGBServer`-equivalent and `nativeAddSurface`. |
| [`VIRTUAL_MASTER_FULL_ANALYSIS.md`](VIRTUAL_MASTER_FULL_ANALYSIS.md) | Breakthrough report — decoded XOR-obfuscated strings in `libvm.so`'s `.data` section. |

### 12.3 AOSP / external references

- Treble architecture: https://source.android.com/docs/core/architecture/halse
- GSI: https://source.android.com/docs/core/ota/gsi
- GSI download: https://ci.android.com/builds/branches/aosp-master/grid
- Binder: https://source.android.com/docs/core/architecture/hidl/binder-ipc
- VINTF: https://source.android.com/docs/core/architecture/hals/vintf-manifest
- APEX: https://source.android.com/docs/core/ota/apex
- Seccomp: https://source.android.com/docs/core/permissions/seccomp
- crosvm (KVM alternative): https://crosvm.dev/
- shadowhook: https://github.com/bytedance/android-inline-hook
- FreeReflection: https://github.com/tiann/FreeReflection
- ext4 Rust crate: https://crates.io/crates/ext4
- sparse image Rust crate: https://crates.io/crates/libsparse

---

## 13. Conclusion

Twoyi's fork-improvement project has done the hard reverse-engineering and infrastructure work. The open-source renderer builds for both ABIs, the `kr64` skeleton compiles and tests green, the binder protocol is mapped out, the devcontainer has KVM, CI runs on every push, and six analysis reports totalling ~4,000 lines document every architectural decision Virtual Master made.

What's left is the engineering: wiring the pieces together, implementing the GSI extractor and init patcher, building the graphics HAL, and reaching the launcher. The plan above breaks that work into 5 phases with concrete file paths, acceptance criteria, and risk factors for each task.

**The single highest-leverage action right now is Phase 1 task 1.1: drop-in test the AOSP renderer on a real arm64 device.** If that passes, the closed-source blob can be deleted, the legal posture of the project changes, and we have a verified foundation for Phase 3.

If you want to contribute, start with the [good first issues](#102-good-first-issues-effort-s-no-architectural-discussion-required) in §10.2, read [`CONTRIBUTING.md`](../CONTRIBUTING.md), and open a pull request against `main`.

— End of roadmap —
