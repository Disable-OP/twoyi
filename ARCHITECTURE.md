# Twoyi — Architecture & Source Map

> **What this document is.** A deep, code-level architecture write-up of the
> **`cyanmint/twoyi`** fork (the only currently-active continuation of the
> archived `twoyi/twoyi` project) as it stands **plus the improvements in
> this branch** (`improvements/initial-cleanup`).
>
> **Date of analysis:** 2026-08-05
> **Base commit:** `25ef89c` ("rom manifest", 2026-05-09, upstream)
> **Branch tip:** see `git log --oneline` — adds input, socket, build, CI,
> and devcontainer improvements.

---

## 1. Project context — why this fork?

### 1.1 The original Twoyi

Twoyi (Chinese: 两仪, pronounced *"two-yi"*) is a **rootless Android-on-Android
container** created by **weishu** (the author of Taichi/EdXposed). Instead of
running Android in a virtual machine (like Waydroid does on Linux, or QEMU on
desktop), Twoyi runs an entire second Android userland **inside one normal
Android app process**. The host device does **not** need to be rooted, unlocked,
or modified in any way — the container runs purely as a `targetSdk=28` APK that
you install from a normal `.apk` file.

The trick works because Android is, at the bottom, just a Linux process tree
sitting on top of a kernel. Twoyi ships a complete Android 8.1 userland
(`init`, `zygote`, `system_server`, framework JARs, ART runtime, HALs) as a
folder inside the app's private data directory, then exec's `./init` from
that folder the same way the real Android boot sequence does. The result is a
*second* Android instance that shares the host kernel but has its own
`system_server`, package manager, surface flinger, and even its own `adbd`.

The original repository is **`github.com/twoyi/twoyi`** — archived in April
2023 with the maintainer's note *"Due to the complexity of the project and
lack of any revenue, the project has been discontinued."* At the time of
archiving it had **1,911 stars** and ~100 forks.

### 1.2 Fork landscape (audited 2026-08-05)

I queried the GitHub fork API for every fork of `twoyi/twoyi` and sorted by
last-push date. The result is sobering — most forks are dead mirrors pushed
exactly once on the day they were created (2023-04-20, the archive date).

| Fork | Last push | Stars | Status |
|---|---|---|---|
| **`cyanmint/twoyi`** | **2026-07-16** | **18** | **Actively developed, primary fork** |
| `blank948555/twoyi` | 2026-07-06 | 0 | One-off |
| `Entersjkhdfkjdhfksjf/twoyi` | 2026-05-15 | 0 | One-off |
| `roro2239/twoyi` | 2026-04-05 | 0 | One-off |
| `ColdWindScholar/twoyi` | 2026-03-21 | 0 | One-off |
| `Faris-0/twoyi` | 2026-03-03 | 1 | Low activity |
| `adnan-core/twoyi` | 2025-12-26 | 2 | Low activity |
| `hyowe/twoyi2` | 2023-11-19 | 1 | Stale (2.7 years ago) |

**`cyanmint/twoyi`** is the only continuation worth contributing to. Its
commit history shows real engineering work across the last 8 months:

- **Open-source `libOpenglRender.so` and `libloader.so` replacements** in Rust
- **`services.jar` DEX patcher** for fixing an Android 8.0 ENOENT bug
- **Profile manager** for multiple rootfs instances
- **`rom manifest`** (May 2026) — AOSP build manifest for the guest ROM

### 1.3 Nogitsune — cyanmint's from-scratch rewrite

While researching cyanmint's profile, I found a second repo: **`cyanmint/Nogitsune`**
(created 2026-06-03, only 3 commits, originally from `Kitsuri-Studios/Nogitsune`).
Nogitsune is **twoyi v2** — a from-scratch rewrite in a modern stack:

| Aspect | Twoyi (current) | Nogitsune (rewrite) |
|---|---|---|
| Language (app) | Java | **Kotlin** |
| Language (native) | Rust | **C++** |
| UI | XML layouts | **Jetpack Compose** |
| Build | Groovy Gradle | **Kotlin DSL + version catalog** |
| Storage | SharedPreferences | **Room DB** (multi-instance from day 1) |
| Application ID | `io.twoyi` | `io.twoyi` (kept same for compatibility!) |
| Namespace | `io.twoyi` | `io.kitsuri.nogitsune` |
| License | MPL-2.0 | Apache-2.0 |
| Status | Active, mature | "still under active development, not yet ready for public use" |

The architecture mirrors twoyi 1:1 (BootHelper, BootStatus, NogitsuneApp,
NogitsuneMessenger, NogitsuneSocketServer, RootfsExtractor, ShellUtil,
Renderer) — same concepts, modern implementation. **This is twoyi v2.**
Improvements we make to v1 may or may not be portable to Nogitsune, but
fixing v1's bugs is still valuable because (a) v1 is the only usable version
today and (b) the same bugs likely exist in Nogitsune's reimplementation.

---

## 2. Twoyi at a glance

| Property | Value |
|---|---|
| **Package name** | `io.twoyi` |
| **Host `minSdk` / `targetSdk` / `compileSdk`** | 27 / 28 / 31 |
| **Guest Android version** | 8.1 (upstream says 8.1–12; only 8.1 ROM exists) |
| **ABIs (this branch)** | `arm64-v8a` + `x86_64` (was arm64-v8a only) |
| **License** | MPL-2.0 |
| **Native languages** | Rust (modern) + Java (UI) + tiny C shims |
| **Original author** | weishu (`github.com/tiann`) |
| **Active fork maintainer** | cyanmint (with `copilot-swe-agent` assistance) |

---

## 3. Three-layer architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│  GUEST LAYER  (rootfs, unpacked to /data/data/io.twoyi/rootfs/)      │
│                                                                      │
│   ./init  ─►  zygote  ─►  system_server  ─►  system services          │
│                                                                      │
│   - SurfaceFlinger renders to a virtual framebuffer                  │
│   - OpenGL ES encoder writes GL commands to /dev/qemu_pipe           │
│   - EventHub reads input from /dev/input/touch + /dev/input/key0     │
│   - installd + PackageManager manage in-container APKs               │
│   - adbd listens on TCP 22122 (connected to by host's libadb.so)     │
└──────────────────────────────────────────────────────────────────────┘
                                  ▲ ▼
                          (sockets + pipes)
                                  ▲ ▼
┌──────────────────────────────────────────────────────────────────────┐
│  NATIVE LAYER  (libtwoyi.so + libloader.so + libOpenglRender.so)     │
│                                                                      │
│   JNI entry (lib.rs) ─► core.rs dispatches to:                       │
│     • input.rs           — virtual uinput touch + key devices         │
│     • renderer_bindings  — FFI to legacy libOpenglRender.so           │
│     • renderer_new/      — open-source Rust renderer (QEMU pipe)     │
│   Spawns the guest: `./init` with TYLOADER env                       │
└──────────────────────────────────────────────────────────────────────┘
                                  ▲ ▼
                              JNI boundary
                                  ▲ ▼
┌──────────────────────────────────────────────────────────────────────┐
│  APP LAYER  (Java, package io.twoyi)                                 │
│                                                                      │
│   TwoyiApplication  →  ProfileManager + RomManager.ensureBootFiles   │
│                     →  TwoyiSocketServer (abstract LocalSocket)      │
│                                                                      │
│   SettingsActivity (launcher) ──► Render2Activity                    │
│       │                                                                  │
│       ├─ SurfaceHolder.Callback → Renderer.init / resetWindow         │
│       ├─ View.OnTouchListener   → Renderer.handleTouch                │
│       └─ onBackPressed          → Renderer.sendKeycode                │
│                                                                      │
│   Installer       — runs libadb.so → adb connect localhost:22122     │
│   TwoyiDocumentsProvider — Storage Access Framework for file share   │
│   TwoyiStatusManager / TwoyiMessenger — boot latch + IPC             │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 4. The app layer (Java)

### 4.1 Entry point — `TwoyiApplication.java`

Every Android app's real entry point is `Application.attachBaseContext()`,
which runs before any `Activity` is created. Twoyi uses this to do five
things in strict order:

1. `ProfileManager.initializeProfiles()` — make sure `profiles/default/rootfs/`
   exists and that `<datadir>/rootfs` is a symlink pointing to the active
   profile's rootfs.
2. `RomManager.ensureBootFiles()` — the **boot hygiene** routine. It runs
   four sub-steps that fix three real-world bugs: orphan container processes,
   missing `/data/local/tmp`, and a DEX-level bug in Android 8.0's
   `PackageInstallerSession`.
3. `TwoyiSocketServer.getInstance(base).start()` — opens an abstract
   SEQPACKET `LocalSocket` named `TWOYI_SOCK` and waits for the guest to
   send control messages.
4. `AppCenter.start(...)` — Microsoft AppCenter crash analytics.
5. Compute and cache the status-bar height.

### 4.2 The launcher — `SettingsActivity.java`

In this fork the **Settings activity is the LAUNCHER intent**. Users land
in a settings screen and must explicitly tap "Launch Container" to start
`Render2Activity`. This lets users configure profiles, display size, DPI,
and renderer choice *before* committing to a boot.

### 4.3 The container host — `Render2Activity.java`

`Render2Activity` is where the magic happens. Its lifecycle:

1. **`onCreate`** — checks `TwoyiStatusManager.isStarted()`. If the guest
   is already running, it calls `RomManager.reboot()` to avoid a double-boot.
2. **`bootSystem()`** — checks `RomManager.romExist()`. If missing, prompts
   the user to pick a `.tar` file via `ACTION_GET_CONTENT`. If present,
   calls `RomManager.clearDalvikCacheIfNeeded()` and `addView(mSurfaceView)`.
3. **`SurfaceHolder.Callback.surfaceCreated`** — sets renderer type and
   debug mode from profile settings, computes scaled DPI, calls
   `Renderer.init(surface, loaderPath, vw, vh, xdpi, ydpi, fps)`.
4. **`surfaceChanged`** — `Renderer.resetWindow(...)`.
5. **`onTouch`** — applies scale matrix to transform surface-space
   coordinates into virtual-display-space, then `Renderer.handleTouch(...)`.
6. **`onBackPressed`** — `Renderer.sendKeycode(KEYCODE_HOME)`.
7. **`showBootingProcedure()`** — waits up to 60s for `BOOT_COMPLETED`.

### 4.4 ROM management — `RomManager.java`

The densest Java file (746 lines) contains the most clever hack: a
**DEX-level binary patcher** for the guest's `services.jar`.

#### 4.4.1 `ensureBootFiles()` — the boot hygiene routine

```java
killOrphanProcess();            // ps -ef | awk '$3==1' | xargs kill -9
ensureDataLocalTmp(context);    // mkdir -p + chmod 777 on rootfs/data/local/tmp
patchServicesJarForPackageInstaller(context);  // see §4.4.2
// create dev/input, dev/socket, dev/maps dirs in rootfs
createLoaderSymlink(context);   // <datadir>/loader64 → libloader.so
saveLastKmsg(context);
```

#### 4.4.2 `patchServicesJarForPackageInstaller()` — DEX surgery

The bug: Android 8.0 (SDK 26) `PackageInstallerSession.openWriteInternal()`
calls `target.delete()` then `Os.stat(target)`. When `offsetBytes == 0` (a
fresh install), the file was just deleted, so `Os.stat()` throws
`ErrnoException(ENOENT)`. SDK 26 doesn't handle `ENOENT` and re-throws as
`IOException("stat failed: ENOENT...")`. This breaks every in-container
APK install. AOSP fixed this in SDK 27.

The patch: **rewrites two bytes inside `classes.dex`** to convert the
`throw v_ioe` instruction into `const/4 v_stat, 0`, mirroring the SDK 27
behavior. Both instructions are exactly 2 bytes:

| Original | Patched |
|---|---|
| `throw vN` = `0x27 0xNN` | `const/4 vM, 0` = `0x12 0x0M` |

The algorithm finds the string `"stat failed: "` in the DEX string pool,
locates the `throw` instruction via the `move-exception` catch handler,
patches it in place, recomputes DEX checksums, and writes the patched
`classes.dex` back into `services.jar`. The patch is idempotent — a flag
file records the patched JAR's mod-time so it doesn't re-patch unless
the JAR has been replaced.

### 4.5 IPC — `TwoyiSocketServer` + `TwoyiMessenger`

Host and guest communicate over an **abstract SEQPACKET LocalSocket**
named `TWOYI_SOCK`. Three message prefixes:

| Message | Action |
|---|---|
| `BOOT_COMPLETED` | `TwoyiStatusManager.markStarted()` — releases the boot latch |
| `SWITCH_HOST` | toggle between host home and guest home |
| `SETTINGS` | open SettingsActivity from inside the guest |

> **Bug fixed in this branch:** the original `start0()` recursed into
> `start()` on every `IOException` with a fixed 1-second sleep. If the
> bind kept failing (e.g. SELinux denial), the cached executor pool
> would accumulate one blocked thread per retry and eventually starve
> the app. This branch replaces the recursion with
> `EXECUTOR.submit(() -> start0(attempt+1))`, caps retries at 5, and
> uses exponential backoff (1s → 2s → 4s → 8s → 16s, capped at 30s)
> plus 0-50% jitter.

### 4.6 App installation — `Installer.java`

To install an APK *inside the guest*, Twoyi runs its own bundled
`libadb.so` (a stripped Android `adb` binary) and uses `adb connect` +
`adb install`. The flow:

1. Spawn `libadb.so -P 9563 nodaemon server` in the background.
2. `adb -P 9563 connect localhost:22122`.
3. `adb -P 9563 -s localhost:22122 install -t -r --no-streaming <apk>`.
   The `--no-streaming` flag is critical: streaming install pipes the
   APK through `abb_exec` to a staging directory that the container's
   `installd` may not have created yet.
4. Parse stdout for `Success` or `Failure [...]`.

---

## 5. The native layer (Rust)

The native side is **three separate Cargo crates**:

| Crate | Source | Output | Purpose |
|---|---|---|---|
| `twoyi` | `app/rs/` | `libtwoyi.so` | JNI entry, renderer dispatch, input system, guest spawn |
| `loader` | `app/rs/loader/` | `libloader.so` (+ `_new`) | Open-source replacement for legacy `libloader.so` |
| `openglrenderer` | `app/rs/openglrenderer/` | `libOpenglRender.so` (+ `_new`) | Open-source replacement for legacy `libOpenglRender.so` |

### 5.1 `libtwoyi.so` — `app/rs/`

`JNI_OnLoad` registers 8 native methods on class `io/twoyi/Renderer`:
`init`, `resetWindow`, `removeWindow`, `handleTouch`, `sendKeycode`,
`setRendererType`, `setDebugRenderer`, `setDebugLogDir`.

`core::init_renderer()` is the heart of the native side:

1. Acquires the `ANativeWindow` from the Java `Surface`.
2. Uses `RENDERER_STARTED.compare_exchange(false, true)` as a one-shot
   guard. On first call:
   - `input::start_input_system(virtual_width, virtual_height)` — spawns
     the touch and key device threads.
   - `thread::spawn` — starts the renderer (Old blob or New Rust impl).
   - `Command::new("./init").current_dir("/data/data/io.twoyi/rootfs").env("TYLOADER", loader_path).spawn()`
     — **this is where the guest actually boots**.
3. On subsequent calls: just calls `resetSubWindow`/`reset_window` to
   update the window pointer without re-booting.

### 5.1.1 Input system — `input.rs`

Two virtual input devices are created as **unix domain sockets** that the
guest's `EventHub` connects to:

- `/data/data/io.twoyi/rootfs/dev/input/touch` — multi-touch device
  named `vtouch` with proper Type-B protocol (ABS_MT_SLOT, TRACKING_ID, etc.).
- `/data/data/io.twoyi/rootfs/dev/input/key0` — keyboard device
  named `vkey`.

> **Bug fixed in this branch:** `send_key_code` previously ignored its
> `keycode` argument and always emitted `KEY_BACK`, regardless of what
> Java passed in. This happened to work for the only caller
> (`onBackPressed → KEYCODE_HOME`) because Android falls back when
> `KEY_HOME` is not in the keyboard's capability bitmap, but it broke
> any future caller (volume, recents, power, etc.).
>
> This branch adds `android_keycode_to_linux()` mapping for the
> navigation-relevant Android `KeyEvent.KEYCODE_*` constants to their
> Linux `KEY_*` equivalents, and `generate_key_device()` now advertises
> every supported key in `key_bitmask` via the new `set_key_bit()`
> helper. The legacy `info.key_bitmask[14] = 0x1C` is removed.

### 5.1.2 The PIE hack

Three pieces cooperate to make `libtwoyi.so` simultaneously loadable as
a JNI library and executable as a standalone binary:

1. `src/interp.c` — places `/system/bin/linker64` in the `.interp` ELF section.
2. `build.rs` — compiles `interp.c` via the `cc` crate.
3. `build_rs.sh` — sets `RUSTFLAGS` to `-C link-arg=-pie` etc. and runs
   `cargo xdk -t <abi> -o ../src/main/jniLibs build --release`.

The result: `./libtwoyi.so --help` works from a shell, AND
`System.loadLibrary("twoyi")` works from Java.

### 5.2 `libloader.so` — open-source replacement

A thin wrapper around `dlopen`/`dlsym`/`dlclose`. The build script
(`build.sh`) builds it for `arm64-v8a` and (since this branch) `x86_64`,
copies the output to `libloader_new.so`, and `chmod +x` so it can be
executed directly as `loader64`.

### 5.3 `libOpenglRender.so` — open-source renderer

A standalone crate exposing the same six-function C ABI as the legacy
blob. Architecture mirrors `renderer_new/` inside `libtwoyi.so`:
`pipe.rs` (QEMU pipe), `opengles.rs` (GL protocol), `gralloc.rs`
(ANativeWindow buffer management).

---

## 6. The guest layer

### 6.1 ROM layout

Once unpacked, `rootfs/` is a fairly standard Android system tree:

```
rootfs/
├── init                    ← the binary core.rs exec's
├── init.rc                 ← boot script
├── rom.ini                 ← ROM metadata (author, version, md5, code)
├── system/framework/services.jar   ← patched by RomManager (§4.4.2)
├── vendor/default.prop     ← host locale + timezone + DPI
├── data/local/tmp/         ← created by ensureDataLocalTmp (chmod 777)
├── data/dalvik-cache/      ← wiped on host OTA (clearDalvikCacheIfNeeded)
├── dev/input/{touch,key0}  ← unix sockets (input.rs)
└── sdcard/                 ← exposed via TwoyiDocumentsProvider
```

### 6.2 AOSP manifest — `default.xml`

Pinned at `refs/tags/android-8.1.0_r81`. The standard AOSP project set
plus twoyi-specific forks in the `twoyi/*` GitHub organization
(`Magisk`, `Superuser`, `platform_art`, `system_vold`,
`hardware_libhardware_legacy`, etc.). As of May 2026 the manifest is
committed but the build infrastructure to turn it into a flashable
`rootfs.7z` is **not yet complete** — the README still says "Build the
ROM: WIP". For now, you must extract the rootfs from a prebuilt APK.

### 6.3 Boot sequence (end-to-end)

```
T+0ms    Application.attachBaseContext
         ├─ ProfileManager.initializeProfiles
         ├─ RomManager.ensureBootFiles
         ├─ TwoyiSocketServer.start (async)
T+100ms  SettingsActivity.onCreate (user sees settings)
T+???    User taps "Launch Container"
T+0      Render2Activity.onCreate
         ├─ reset TwoyiStatusManager
         ├─ load profile display settings (W×H×DPI)
         ├─ setupSurfaceViewLayout (letterboxing)
         └─ bootSystem
             ├─ RomManager.romExist? yes → continue
             ├─ clearDalvikCacheIfNeeded (~100ms if cache exists)
             └─ mRootView.addView(mSurfaceView, 0)
T+50ms   surfaceCreated
         └─ Renderer.init(...)  → JNI → core::init_renderer
             ├─ input::start_input_system (touch + key threads)
             ├─ thread::spawn (renderer thread)
             └─ Command::new("./init").spawn()  ← GUEST BOOTS
T+200ms  Guest init starts: parse init.rc, mount tmpfs, start ueventd,
         start zygote, fork system_server, start core services...
T+3-30s  Guest sends "BOOT_COMPLETED" over TWOYI_SOCK
         → boot latch released → user sees guest home screen
```

---

## 7. Build system (this branch)

### 7.1 Gradle

- Root `build.gradle` uses AGP 7.1.1.
- `app/build.gradle` declares `compileSdkVersion 31`, `targetSdkVersion 28`,
  `minSdkVersion 27`.
- **New in this branch:** `abiFilters` now includes both `arm64-v8a` and
  `x86_64`. The `cargoBuild` task passes `all` to `build_rs.sh` by default
  and can be overridden with `-Pabis=arm64-v8a` for fast local iteration.

### 7.2 Rust toolchain

- Uses `cargo-xdk` (weishu's tool, `github.com/tiann/cargo-xdk`).
- NDK r27c (was r22 originally — bumped because r22 has libunwind issues
  with modern Rust).
- **New in this branch:** `build_rs.sh`, `loader/build.sh`, and
  `openglrenderer/build.sh` all accept an ABI list on the command line
  (`./build_rs.sh --release arm64-v8a x86_64` or `./build_rs.sh --release all`)
  and loop over each ABI.
- **New in this branch:** `.cargo/config.toml` now has identical PIE
  flags for both `aarch64-linux-android` and `x86_64-linux-android`.

### 7.3 GitHub Actions — `.github/workflows/build.yml`

**New in this branch:** the workflow:

- Adds `x86_64-linux-android` to the Rust target list.
- Triggers on `improvements/**` branches (in addition to main/develop).
- Adds `workflow_dispatch` inputs:
  - `abis`: `all` | `arm64-v8a` | `x86_64` (default `all`).
  - `include_rootfs`: boolean (default false). When true, downloads the
    real `rootfs.7z` from the cyanmint release and bundles it into the APK.
- Bumps JDK from 11 to 17.
- Caches cargo registry + git + per-crate target dirs.
- Uploads build logs + reports on failure.
- 30-minute timeout (was unbounded).

### 7.4 Codespace — `.devcontainer/`

**New in this branch:** a complete Codespace configuration:

- `devcontainer.json` — 4-core / 16 GB / 32 GB machine (user-selectable
  in GitHub UI), `runArgs: ["--privileged", "--init"]`, Docker-outside-of-Docker,
  JDK 17, Rust, NDK r27c, cargo-xdk.
- `scripts/setup.sh` — runs on `postCreateCommand`; installs the full
  Android toolchain.
- `scripts/check-kvm.sh` — definitive KVM availability check. Writes
  `/tmp/kvm-verdict.txt` so other scripts can branch on the result.
- `scripts/run-redroid.sh` — starts an x86_64 `redroid:13.0.0` container
  with ADB on port 5555.
- `scripts/test-twoyi.sh` — installs the APK, launches twoyi, takes 8
  screenshots at increasing intervals.
- `scripts/analyze-screenshots.sh` — sends each screenshot to a vision
  LLM (default `glm-4.6v`, override with `TWOYI_VLM_MODEL`) and asks
  for UI description + tap coordinates for the next action.

#### Why redroid instead of KVM

GitHub Codespaces run on Azure VMs that do NOT expose `/dev/kvm` to the
devcontainer, even with `--privileged` in `runArgs`. Multiple authoritative
sources confirm this:

- [devcontainers/images#884](https://github.com/devcontainers/images/issues/884)
- [dotnet/runtime#77851](https://github.com/dotnet/runtime/issues/77851)
- [bgplabs.net/4-codespaces](https://bgplabs.net/4-codespaces)
- [github/community#160591](https://github.com/orgs/community/discussions/160591)

`check-kvm.sh` runs on codespace creation to verify this empirically —
if `/dev/kvm` is unavailable, the test scripts use redroid (Android-in-
container, no KVM needed) instead.

---

## 8. Complete file map (this branch)

```
cyanmint/twoyi (+ improvements/initial-cleanup branch)/
├── README.md / README_CN.md           Project overview (EN/CN)
├── LICENSE                            MPL-2.0
├── ARCHITECTURE.md                    ← NEW: this document
├── default.xml                        AOSP manifest (android-8.1.0_r81)
├── build.gradle                       Root Gradle config (AGP 7.1.1)
├── settings.gradle                    Single-module project (':app')
├── gradle.properties                  AndroidX + Jetifier enabled
├── gradlew / gradlew.bat              Gradle wrapper
│
├── .github/
│   └── workflows/build.yml            ← UPDATED: builds arm64 + x86_64
│
├── .devcontainer/                     ← NEW: Codespace config
│   ├── devcontainer.json
│   └── scripts/
│       ├── setup.sh                   — runs on postCreateCommand
│       ├── check-kvm.sh               — definitive KVM check
│       ├── run-redroid.sh             — start x86_64 redroid
│       ├── test-twoyi.sh              — install APK + screenshot
│       └── analyze-screenshots.sh     — VLM-based UI analysis
│
├── app/
│   ├── build.gradle                   ← UPDATED: abiFilters now arm64 + x86_64
│   └── src/
│       ├── main/
│       │   ├── AndroidManifest.xml
│       │   ├── jniLibs/arm64-v8a/     ← legacy blobs + new Rust variants
│       │   ├── jniLibs/x86_64/        ← NEW: only the new Rust variants
│       │   ├── java/io/twoyi/         ← Java sources (mostly unchanged)
│       │   │   ├── TwoyiApplication.java
│       │   │   ├── Render2Activity.java
│       │   │   ├── Renderer.java
│       │   │   ├── TwoyiSocketServer.java    ← UPDATED: bounded retries
│       │   │   └── ...
│       │   └── res/                   ← layouts, strings, themes
│       └── test/
│
└── app/rs/                            ← Rust source
    ├── Cargo.toml
    ├── build.rs
    ├── build_rs.sh                    ← UPDATED: multi-ABI support
    ├── .cargo/config.toml             ← UPDATED: x86_64 PIE flags
    ├── src/
    │   ├── lib.rs                     ← JNI_OnLoad + native method regs
    │   ├── core.rs                    ← renderer dispatch + guest spawn
    │   ├── input.rs                   ← UPDATED: keycode mapping + bitmask
    │   ├── renderer_bindings.rs       ← FFI to legacy libOpenglRender.so
    │   ├── renderer_new/              ← open-source Rust renderer
    │   └── interp.c                   ← .interp segment for PIE
    ├── loader/
    │   ├── build.sh                   ← UPDATED: multi-ABI support
    │   └── src/lib.rs
    └── openglrenderer/
        ├── build.sh                   ← UPDATED: multi-ABI support
        └── src/
```

---

## 9. Improvement opportunities (still open)

Things this branch does NOT fix, but should be done next:

### 9.1 Build & dependency modernization (low risk, high payoff)

- Bump AGP from 7.1.1 to 8.x.
- Bump `targetSdkVersion` from 28 to 34+.
- Migrate `cargo-xdk` to `cargo-ndk`.
- Bump all AndroidX libraries.
- Bump Rust crates (`jni:0.19` → `0.21`, `ndk:0.6` → `0.9`, etc.).

### 9.2 Replace closed-source blobs (medium risk, very high payoff)

| Blob | Status | Replacement |
|---|---|---|
| `libloader.so` | ✅ Replaced | `app/rs/loader/` (Rust) — already builds. Delete the legacy `.so` and update `RomManager.LOADER_FILE` to point at `libloader_new.so`. |
| `libOpenglRender.so` | ⚠️ Partial | `app/rs/openglrenderer/` exists but the new renderer is behind `use_new_renderer=false` default. Validate it actually renders correctly across devices, then flip the default. |
| `libadb.so` | ❌ Still closed | Replace with an open-source Java ADB client (e.g., `adblib` from AOSP, or `github.com/MuntashirAkon/adb-android`). |

### 9.3 Architecture cleanups

- Deduplicate `renderer_new/` and `openglrenderer/` (nearly identical code).
- Convert `Renderer.java` to Kotlin.
- Replace `jdeferred` with `CompletableFuture`.
- Replace `com.cleveroad:androidmanimation:0.9.1` with `android.animation.*`.
- Replace `com.github.clans:fab:1.6.4` with Material `ExtendedFloatingActionButton`.

### 9.4 Runtime behavior improvements

- The 60-second boot timeout doesn't differentiate between "still booting"
  and "stuck". Add a heartbeat mechanism.
- The dalvik-cache clear is synchronous on the UI thread. Move to a coroutine.
- `patchServicesJarForPackageInstaller` should be moved to install time.

### 9.5 Missing features

- Build the ROM from source. The manifest exists; what's missing is build
  orchestration in CI.
- Android 10/12/13 guest support.
- Multi-window mode.

### 9.6 Quality-of-life improvements

- Add instrumented tests.
- Replace Microsoft AppCenter (being retired) with Sentry or self-hosted.
- Remove `moe.feng:AlipayZeroSdk:1.1` (donation SDK).
- Add a `CONTRIBUTING.md` (referenced but missing).

---

## 10. Setting up a development environment

```bash
# 1. Clone the fork
git clone https://github.com/Disable-OP/twoyi.git
cd twoyi

# 2. EITHER set up locally:
curl https://sh.rustup.rs -sSf | sh
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-xdk
# Install Android Studio with SDK Platform 31 + NDK r27c
# Download rootfs.7z from cyanmint releases → app/src/main/assets/rootfs.7z
./gradlew assembleRelease

# 2. OR spin up a Codespace:
#    - On GitHub: Code → Codespaces → Create codespace on main (or this branch)
#    - Select 4-core / 16 GB / 32 GB machine
#    - Wait ~5 min for postCreateCommand to finish
#    - Run: ./.devcontainer/scripts/check-kvm.sh
#    - Run: ./.devcontainer/scripts/run-redroid.sh
#    - Run: ./.devcontainer/scripts/test-twoyi.sh
#    - Run: ./.devcontainer/scripts/analyze-screenshots.sh

# 3. (Optional) Build only one ABI for faster iteration:
./gradlew assembleRelease -Pabis=arm64-v8a

# 4. (Optional) Rebuild only the Rust side:
cd app/rs && ./build_rs.sh --release arm64-v8a x86_64
```

---

## 11. References

- **Active fork source**: https://github.com/cyanmint/twoyi
- **This branch's fork**: https://github.com/Disable-OP/twoyi
- **Original archived repo**: https://github.com/twoyi/twoyi
- **cyanmint's Nogitsune (twoyi v2 rewrite)**: https://github.com/cyanmint/Nogitsune
- **Project website**: https://twoyi.app
- **Original author**: weishu — https://github.com/tiann
- **cargo-xdk**: https://github.com/tiann/cargo-xdk
- **Anbox** (graphics/pipe design twoyi's renderer follows): https://github.com/anbox/anbox
- **AOSP android-8.1.0_r81 reference**:
  https://android.googlesource.com/platform/manifest/+/refs/tags/android-8.1.0_r81
- **DEX bytecode reference**:
  https://source.android.com/docs/core/runtime/dex-format
- **Codespaces KVM**:
  https://github.com/devcontainers/images/issues/884
  https://github.com/dotnet/runtime/issues/77851
- **Redroid (Android-in-container)**: https://github.com/remote-android/redroid-doc

---

*This document was produced by reading every source file in the
`cyanmint/twoyi` repository at commit `25ef89c` plus the improvements
in this branch. It is intended as a living reference — if you improve
the project, please update the corresponding section.*
