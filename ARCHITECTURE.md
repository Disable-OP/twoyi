# Twoyi — Architecture & Source Map

> **What this document is.** A deep, code-level architecture write-up of the
> **`cyanmint/twoyi`** fork (the only currently-active continuation of the
> archived `twoyi/twoyi` project) as it stands **plus the improvements in
> this branch** (`improvements/initial-cleanup`).
>
> **Date of analysis:** 2026-08-05 (last revised 2026-08-06)
> **Base commit:** `25ef89c` ("rom manifest", 2026-05-09, upstream)
> **Branch tip:** `a021b25` ("docs: final comprehensive MEMORY.md update + any
> remaining fixes") on `improvements/initial-cleanup`. Adds: x86_64 ABI,
> AOSP-source `libOpenglRender.so`, Rust `libloader.so`, dynamic data dir
> (work-profile support), x86_64 SIGABRT fix, `kr64` kernel-replacement
> skeleton, CI, devcontainer, input handling, and APK signing. See §9–§10
> for the reverse-engineering and roadmap sections added in this revision.
>
> **Post-architecture cleanup rounds (2026-08-06):** subsequent commits
> (`69a9741` → `a021b25`) did not change the *architecture* but did add
> production-readiness hardening: `res/xml/network_security_config.xml`
> (cleartext forbidden by default, loopback exception for ADB), full
> 4-locale i18n (en/zh-rCN/zh-rTW/ja), 0 clippy warnings / 0 lint errors
> / 145-145 Rust tests passing, `printStackTrace()` → `Log.e()` sweep,
> AppCenter key extracted to `BuildConfig`, and CI gating on
> `cargo clippy -- -D warnings` + `./gradlew lintRelease`. See
> `worklog.md` §Round 19 for the full rundown.

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

### 4.7 Work Profile Support — Dynamic Data Directory

**Commit:** `9c4b907` (`feat: dynamic data directory for work profile support`)

Earlier twoyi hardcoded the app's data path as `/data/data/io.twoyi/` in
**eight** places across the Rust crate (`core.rs`, `input.rs`,
`socket_monitor.rs`). This broke when the app was installed in an Android
**work profile** (managed profile / Android for Work), where the data
directory is `/data/user/<uid>/io.twoyi/` instead of `/data/data/io.twoyi/`.
A work-profile install is the common case for "clone a second instance of
twoyi inside a corporate container", so the hardcodes were a real blocker.

The fix replaces all eight hardcodes with a runtime-resolved path. The
Java side asks Android for the real data dir, hands it to Rust once via
JNI, and every subsequent path computation derives from that.

#### 4.7.1 The Rust API

`app/rs/src/core.rs` exposes a thread-safe single-assignment cell holding
the data dir:

```rust
static DATA_DIR: OnceLock<String> = OnceLock::new();

pub fn set_data_dir(dir: String) {
    let _ = DATA_DIR.set(dir);
}

pub fn get_data_dir() -> &'static str {
    DATA_DIR.get().map(|s| s.as_str()).unwrap_or("/data/data/io.twoyi")
}
```

The `unwrap_or` fallback preserves backwards compatibility with older
Java builds that don't call `setDataDir` — they continue to work exactly
as before.

Derived path helpers are now functions, not constants:

| Helper | Returns |
|---|---|
| `get_rootfs_dir()` | `{data_dir}/rootfs` |
| `get_log_path()` | `{data_dir}/log.txt` |
| `get_touch_path()` | `{data_dir}/rootfs/dev/input/touch` |
| `get_key_path()` | `{data_dir}/rootfs/dev/input/key0` |
| `get_opengles_paths()` | `Vec<String>` of `{rootfs}/opengles{,2,3}` |

`input.rs` replaced its `const TOUCH_PATH` / `const KEY_PATH` constants
with calls into `core::get_touch_path()` / `core::get_key_path()`.
`socket_monitor.rs` removed three hardcoded `opengles*` paths from the
static `SOCKET_PATHS` array and rebuilds the list lazily from
`core::get_opengles_paths()`.

#### 4.7.2 The JNI bridge

`Renderer.java` declares the new native method:

```java
public static native void setDataDir(String dataDir);
```

`Render2Activity.surfaceCreated()` calls it **before** `Renderer.init()`:

```java
Renderer.setDataDir(getDataDir().getAbsolutePath());
Renderer.init(surface, loaderPath, vw, vh, xdpi, ydpi, fps);
```

`lib.rs` registers the method on `io/twoyi/Renderer` as
`setDataDir(Ljava/lang/String;)V` and forwards the argument to
`core::set_data_dir()`. There is no reset path — `OnceLock` is
single-assignment by design; the data dir does not change for the
lifetime of the process.

#### 4.7.3 How `libOpenglRender.so` follows suit

The AOSP-built renderer (§5.4) honors the same convention through an
**environment variable** rather than a function call: `UnixStream.cpp`'s
`make_unix_path()` reads `TWOYI_ROOTFS` (defaulting to
`/data/data/io.twoyi/rootfs`) and builds the three `opengles{,2,3}`
socket paths from it. `core.rs::init_renderer()` exports
`TWOYI_ROOTFS=<data_dir>/rootfs` into the child environment when it
spawns the guest's `./init`, so the renderer and the guest agree on the
socket locations regardless of where the OS actually put the app's data.

---

## 5. The native layer (Rust + AOSP C++)

The native side is **three Cargo crates plus one AOSP C++ build**:

| Crate | Source | Output | Purpose |
|---|---|---|---|
| `twoyi` | `app/rs/` | `libtwoyi.so` | JNI entry, renderer dispatch, input system, guest spawn |
| `loader` | `app/rs/loader/` | `libloader.so` (+ `_new`) | Open-source replacement for legacy `libloader.so` |
| `openglrenderer` | `app/rs/openglrenderer/` | `libOpenglRender.so` (Rust impl, `_new` variant) | Open-source Rust renderer (alternative to the AOSP build) |
| `kr64` | `app/rs/kr64/` | `libkr64.so` (+ `kr64` binary) | **Skeleton** kernel-replacement daemon (see §5.5) |
| *AOSP emugl* | `download/aosp-built/` (rebuilt from `platform/sdk` commit `7a712acc`) | `libOpenglRender.so` (final, shipped) | AOSP-source renderer with ported legacy pieces (see §5.4) |

> The AOSP-source `libOpenglRender.so` (§5.4) is the **shipped** renderer;
> the Rust `openglrenderer` crate (§5.3) is an alternative open-source
> implementation kept around for experimentation and as the default on
> x86_64 (where the legacy arm64-only blob cannot run).

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

### 5.3 `libOpenglRender.so` — open-source Rust renderer (alternative)

A standalone Rust crate exposing the same six-function C ABI as the
legacy blob. Architecture mirrors `renderer_new/` inside `libtwoyi.so`:
`pipe.rs` (QEMU pipe), `opengles.rs` (GL protocol), `gralloc.rs`
(ANativeWindow buffer management).

This is the **fallback / x86_64-default** renderer. On arm64 the AOSP
build in §5.4 is preferred (it is more complete — it has the
`startGBServer` graphics-buffer proxy the Rust impl doesn't yet
implement). On x86_64 the Rust impl is forced on by
`core.rs::effective_renderer_type()` because the legacy blob is arm64-
only and the AOSP build of `startGBServer` depends on
`AHardwareBuffer_recvHandleFromUnixSocket` which historically wasn't
wired up on x86_64 emulator images.

---

### 5.4 Open-Source `libOpenglRender.so` — AOSP rebuild

**Commits:** `47f8335` (initial build) + `eb13449` (port missing legacy
pieces). **Source:** AOSP `platform/sdk` at commit
`7a712acc02282985dcd32feb81284e1f2b19ec7e`, Apache-2.0 licensed.
**Artifacts:** `download/aosp-built/libOpenglRender_aosp_{arm64,x86_64}.so`.

The legacy closed-source `libOpenglRender.so` shipped by upstream twoyi
is a 1,059,128-byte (1.06 MB) arm64-only blob. Disassembly
(`download/TWOYI_DISASSEMBLY_ANALYSIS.md`) proved via demangled symbol
table cross-reference that it's a lightly-modified build of AOSP
`emugl` — so we rebuilt it from source. The result is a 605–611 KB
`.so` for both ABIs (an ~43% size reduction) that links the device's
real system `libEGL.so` / `libGLESv1_CM.so` / `libGLESv2.so`
dynamically instead of bundling desktop-GL translator libraries.

#### 5.4.1 Build pipeline

1. **Sparse-checkout the emugl tree** from `platform/sdk`:
   `emulator/opengl/{shared,host/libs/{GLESv1_dec,GLESv2_dec,Translator,
   renderControl_dec},host/tools/emugen,system}`.
2. **Build the `emugen` host tool** (AOSP's wire-protocol code generator)
   from 5 source files with host `g++ -std=c++11 -O2 -D_GNU_SOURCE
   -include unistd.h` — the `-include unistd.h` is needed because
   `main.cpp` calls `getopt()` without including the header on glibc ≥ 11.
3. **Generate the decoder sources** by running `emugen -D` three times:
   `renderControl`, `gl`, `gl2`. Each invocation produces six files
   (`<base>_dec.{cpp,h}`, `<base>_opcodes.h`,
   `<base>_server_context.{cpp,h}`, `<base>_server_proc.h`).
4. **Write the Android compat shim layer** under `compat/`. The AOSP
   emugl source includes several Android platform-private headers that
   the NDK doesn't ship. Each is reimplemented with POSIX primitives:

   | Header | Implementation |
   |---|---|
   | `cutils/threads.h` | `pthread_key_t` + `pthread_mutex_t` |
   | `cutils/atomic.h` | `__atomic_*` GCC/Clang builtins |
   | `cutils/log.h` | `__android_log_print` from `liblog` |
   | `cutils/sockets.h` | raw `AF_UNIX`/`AF_INET` sockets |
   | `utils/{threads,Errors,Vector,List,String8,KeyedVector,RefBase}.h` | `pthread` + `std::vector` / `std::list` / `std::string` / `std::map` wrappers |

5. **Apply the twoyi-specific patches** (see §5.4.2 below).
6. **Build with CMake + NDK r27c** (`clang 18.0.3`, `cmake 3.22.1`):
   `-DANDROID -DHAVE_ANDROID_OS=1 -DWITH_GLES2 -include assert.h -O2
   -fno-rtti -std=c++11`. **No `-fvisibility=hidden`** — the legacy blob
   exports all symbols, and we need the C-ABI entry points visible.
   Stripped with `llvm-strip -x` (keep dynamic symbols only).

#### 5.4.2 Modifications applied to the AOSP source

| # | File | Modification |
|---:|---|---|
| 1 | `render_api_platform_types.h` | Added `__ANDROID__` branch using `void*` for `FBNative{Display,Window}Type` (no X11 on Android) |
| 2 | `EGLDispatch.cpp` | `libEGL_translator.so` → `libEGL.so` (use the device's real EGL) |
| 3 | `GLDispatch.cpp` | `libGLES_CM_translator.so` → `libGLESv1_CM.so` |
| 4 | `GL2Dispatch.cpp` | `libGLES_V2_translator.so` → `libGLESv2.so` |
| 5 | `UnixStream.cpp` | Rewrote `make_unix_path()` to produce `$TWOYI_ROOTFS/opengles{,2,3}` (defaults to `/data/data/io.twoyi/rootfs/opengles`) |
| 6 | `NativeLinuxSubWindow.cpp` (and `NativeMacSubWindow.m` / `NativeWindowsSubWindow.cpp`) | Not in `CMakeLists.txt`'s `EMUGL_SOURCES` (platform-specific X11 / Win32 / Carbon code; not applicable on Android). Kept in the tree as part of the reference AOSP source. |
| 7 | `twoyi_api.cpp` (**new**) | Implements the six twoyi-required entry points (`startOpenGLRenderer`, `setNativeWindow`, `resetSubWindow`, `removeSubWindow`, `destroyOpenGLSubwindow`, `repaintOpenGLDisplay`) plus the four `dl*_ex` wrappers. **This is the only file actually compiled into the shipping `libOpenglRender.so`** (see `CMakeLists.txt`); it talks to the system EGL / GLESv2 directly (`eglGetDisplay` / `eglInitialize` / `eglChooseConfig` / `eglCreateContext` / `eglCreateWindowSurface` / `eglSwapBuffers`) and runs a background render thread that owns the EGL context for its entire lifetime. The original "compose the AOSP `FrameBuffer` / `RenderServer` API" approach (rows 1–6, 8) is preserved in the tree as reference source for a possible future re-enablement of the full AOSP emugl pipeline. |
| 8 | `render_api.cpp` | Removed `static` from `s_renderThread` so the now-deleted reference `twoyi/twoyi_api.cpp` could `extern` it. Not required by the active build; the patch is preserved on the reference source in `libOpenglRender/render_api.cpp` for historical consistency. |
| 9 | `CMakeLists.txt` (**new**) | Builds **only `twoyi_api.cpp`** into `libOpenglRender.so`. Links `libEGL`, `libGLESv2`, `libandroid`, `liblog`, `libdl`. (The 33-source-file build described in earlier revisions of this document was the abandoned "full AOSP pipeline" approach.) |

**Function renaming.** The twoyi-required entry-point names —
`startOpenGLRenderer` with the twoyi-specific signature
`(win, w, h, xdpi, ydpi, fps)`, `setNativeWindow`, `resetSubWindow`,
`removeSubWindow`, `destroyOpenGLSubwindow`, `repaintOpenGLDisplay` —
are exported by `twoyi_api.cpp` to match what `renderer_bindings.rs`
declares as `extern "C"`. (The original AOSP names `initOpenGLRenderer`
and `createOpenGLSubwindow` are not used by the active build; they
were the names the abandoned "compose the AOSP API" approach renamed.)

#### 5.4.3 The ported legacy pieces (commit `eb13449`)

The first AOSP build was missing three pieces the legacy blob had.
The function-level comparison (`download/FUNCTION_LEVEL_COMPARISON.md`)
found them; the PORT-1 task reverse-engineered and re-implemented them
in C++ on top of the AOSP source:

| File | Lines | Purpose | Size vs. legacy |
|---|---:|---|---|
| `dl_ex.cpp` | 339 | Android-7+-aware `dlopen_ex` / `dlsym_ex` / `dlclose_ex` / `dlerror_ex` with `/proc/self/maps` scanner + 5 hardcoded system library paths + an ELF `.dynsym` parser. Works around Android 7+'s library-namespace restrictions (needed to resolve non-exported symbols like `android::AHardwareBuffer_to_ANativeWindowBuffer`). | `dlclose_ex` is **byte-for-byte the same size** (208 B). Net `dl*_ex` + `startGBServer` is 24 B smaller than legacy. |
| `GraphicBuffer.h` + `GraphicBuffer.cpp` | 74 + 153 | Opens `$TWOYI_ROOTFS/opengles3` Unix socket via `socket_local_server()`, `accept()` loop calls `AHardwareBuffer_recvHandleFromUnixSocket` then `AHardwareBuffer_to_ANativeWindowBuffer`. | Merges legacy's `GraphicBuffer` + `GraphicBufferHandler` (1,072 B) into one class (948 B) |
| `startGBServer.cpp` | 137 | Entry point: `GraphicBuffer::create()` → `dlopen_ex("libandroid.so")` → `dlsym_ex` for both `AHardwareBuffer_*` symbols → cache in globals → `gb->start()`. | 372 B vs. legacy's 220 B (added a singleton guard — legacy would crash if called twice) |

**`RenderWindow` deliberately NOT ported** — the function-level
comparison confirmed it's a thin wrapper around `FrameBuffer`; the
AOSP build's flat `startOpenGLRenderer → FrameBuffer` architecture is
behaviorally equivalent. Porting would add ~2.5 KB of dead indirection.

#### 5.4.4 Size and symbol comparison with the legacy blob

| Build | Size | Notes |
|---|---:|---|
| Legacy arm64 (closed-source) | 1,059,128 B | Statically links GL translators + libc++ + libgcc |
| AOSP arm64 (initial) | 603,296 B | All 6 twoyi symbols, but missing `startGBServer` / `GraphicBuffer` / `dl*_ex` |
| **AOSP arm64 (after port, shipped)** | **610,720 B** | +7,424 B from port; functionally complete |
| AOSP x86_64 (initial) | 597,632 B | Same feature set as initial arm64 |
| **AOSP x86_64 (after port, shipped)** | **605,152 B** | +7,520 B from port |

**Why the AOSP build is smaller:** the legacy blob statically links the
desktop-GL translator libraries (`libEGL_translator.so`,
`libGLES_CM_translator.so`, `libGLES_V2_translator.so`) which translate
GLES 1/2 commands to desktop OpenGL — totaling ~290 KB. It also
statically links libc++ locale support (~30 KB), libc++abi (~5 KB), and
libgcc unwinder (~2 KB). The AOSP build dynamically links the system
`libEGL.so` / `libGLESv1_CM.so` / `libGLESv2.so` (using the device's
actual GPU driver — architecturally superior) and uses NDK's minimal
`c++_static` STL.

**Symbol verification** (after port) — all 11 twoyi-required C-ABI
symbols exported on both ABIs:

| Symbol | Legacy arm64 | AOSP arm64 | AOSP x86_64 |
|---|:---:|:---:|:---:|
| `startOpenGLRenderer` | ✓ | ✓ | ✓ |
| `destroyOpenGLSubwindow` | ✓ | ✓ | ✓ |
| `repaintOpenGLDisplay` | ✓ | ✓ | ✓ |
| `setNativeWindow` | ✓ | ✓ | ✓ |
| `resetSubWindow` | ✓ | ✓ | ✓ |
| `removeSubWindow` | ✓ | ✓ | ✓ |
| `startGBServer` | ✓ | ✓ (port) | ✓ (port) |
| `dlopen_ex` / `dlsym_ex` / `dlclose_ex` / `dlerror_ex` | ✓ | ✓ (port) | ✓ (port) |
| `GraphicBuffer::*` + vtable | ✓ | ✓ (port) | ✓ (port) |

Full per-function size deltas and the full 35-row twoyi-vs-VM-vs-AOSP
comparison table are in `download/PROJECT_SUMMARY.md` §4.4 and §6.

#### 5.4.5 Honest status

The AOSP build has been compiled and is shipped in
`app/src/main/jniLibs/{arm64-v8a,x86_64}/libOpenglRender.so`. It has
**not** yet been verified end-to-end on a real device to actually render
the guest's GL output. The closest verified result is on x86_64, where
the *Rust* renderer (§5.3) initialises its GL context successfully but
fails at the QEMU-pipe step (the standard Android emulator doesn't
expose `/dev/qemu_pipe`). See `download/TWOYI_HONEST_STATUS.md` for the
full honest test matrix.

---

### 5.5 Kernel Replacement Daemon (`kr64`) — skeleton

**Commit:** `570e95e` (`feat(kr64): kernel replacement daemon skeleton`).
**Source:** `app/rs/kr64/` (3,084 lines across 9 files).
**Design doc:** `download/KR64_SKELETON.md`.

This is the **Rust port of Virtual Master's `libkr64.so`** — the most
architecturally significant piece of VM's container model (see §9 for
the full reverse-engineering). In VM, `libkr64.so` is a standalone ELF
executable disguised as a `.so` (with `.interp` pointing at a custom
`libkrloader64.so`); the kernel exec's it, and it materialises a
per-VM virtual `/dev` tree, installs a seccomp filter, emulates `/proc`,
sets up the mount namespace, then `execve`s the guest's
`/system/bin/init`. Without an equivalent daemon, twoyi can only boot
its ancient custom 8.1 ROM — it cannot boot a Treble GSI (see §10).

The current `kr64` crate is a **skeleton**: it compiles, has 26 passing
unit tests, and a working end-to-end smoke test on Linux. It is **not**
integrated into the build yet (it is not a member of the parent
workspace; `build_rs.sh` does not yet build it).

#### 5.5.1 Files

| File | Lines | Purpose |
|------|-------|---------|
| `Cargo.toml` | 39 | `crate-type = ["cdylib", "rlib"]` + `[[bin]]`. Deps: `libc` only (no `log`, `once_cell`, `nix` — per task spec). Build-dep: `cc` for `interp.c`. |
| `build.rs` | 88 | Compiles `interp.c`; emits PIE linker flags (`-Wl,-e,kr64_main`, `-Wl,--undefined=interp`). Android-only: `--dynamic-linker=/system/bin/linker64`. |
| `interp.c` | 40 | Forces a `.interp` section (PT_INTERP) so `libkr64.so` is directly executable via the PIE-as-cdylib trick. On Android: `/system/bin/linker64`. On Linux host: no override (so `cargo test` runs). |
| `src/main.rs` | 38 | Binary entry point. Thin wrapper around `kr64::run(args)`. |
| `src/lib.rs` | 652 | Crate root. `Config` struct, `parse_args()`, `run()` daemon entry point, `kr64_main` cdylib entry, `info!` / `warning!` / `error!` macros (eprintln-based, no `log` crate). |
| `src/devices.rs` | 405 | Virtual `/dev` tree: `qemu_pipe`, `touch`, `key0`, `event`, `gb`, `gb2` via `UnixListener::bind`. |
| `src/seccomp.rs` | 831 | BPF seccomp filter + SIGSYS handler. Allow ~80 syscalls, trap `mount` / `umount2` / `swapon` / `reboot` / etc., kill on `ptrace` / `kexec_load` / `init_module` / `pivot_root`. |
| `src/proc_emu.rs` | 534 | Synthesises `/proc/version`, `/proc/cpuinfo`, `/proc/meminfo`, `/proc/cmdline`, `/proc/mounts`, `/proc/self/`, `/proc/sys/kernel/*`, `/proc/sys/vm/*`. |
| `src/mount_mgr.rs` | 457 | `unshare(CLONE_NEWNS)` → bind-mount ROM partitions → tmpfs on `/dev` / `/proc` / `/sys` / `/tmp` / `/apex` / `/mnt` → `pivot_root` → `umount2(old_root)`. Falls back to `chroot` on EPERM. |

#### 5.5.2 What it does at runtime

```
kr64 --rootfs <fs_dir> --data-dir <data_dir> [--vmid N]
     [--width W --height H --dpi D] [--init /system/bin/init]
     [--no-seccomp] [--no-namespaces] [--rw-rom] [--log-level debug]
```

1. `parse_args()` builds a `Config`.
2. `devices::create_all_devices()` binds the six MVP Unix sockets:
   `{rootfs}/dev/qemu_pipe`, `{rootfs}/dev/input/touch`,
   `{rootfs}/dev/input/key0`, `{data_dir}/dev/event`, `{rootfs}/dev/gb`,
   `{rootfs}/dev/gb2`.
3. `proc_emu::populate_proc()` writes synthesised static files into
   `{rootfs}/proc/`.
4. `run()` `fork()`s. The child calls `mount_mgr::setup_mounts()` →
   `seccomp::install()` → `execve(init_path)`. The parent runs the
   device-accept threads and `waitpid()`s on the child.

#### 5.5.3 The PIE-as-cdylib trick (matches VM)

`libkr64.so` is **directly executable** via the same trick as
`libtwoyi.so` (§5.1.2) — `interp.c` puts `/system/bin/linker64` in the
`.interp` section, and `build.rs` sets `-Wl,-e,kr64_main` so the linker
entry point is `kr64_main` (the cdylib entry exported by `lib.rs`).
This matches VM's approach exactly, except twoyi uses the **system**
`linker64` instead of a custom `libkrloader64.so` — simpler, but loses
VM's elevated-privilege early bootstrap.

On the Linux host (for `cargo test`), `interp.c`'s `#ifdef __ANDROID__`
gate emits a plain `.rodata` symbol instead of overriding `.interp`,
so the test binary runs under the default `/lib64/ld-linux-x86-64.so.2`.

#### 5.5.4 Seccomp + SIGSYS handler

The BPF program is structured as:

```
  1. ld arch                  // load audit arch
  2. jeq EXPECTED_ARCH, jt=0, jf=N     // wrong arch → kill
  3. ld nr                    // load syscall number
  4. for each allowed syscall: jeq + ret ALLOW
  5. for each trapped syscall: jeq + ret TRAP
  6. for each killed  syscall: jeq + ret KILL_PROCESS
  7. ret ALLOW                // default: allow
  8. ret KILL_PROCESS         // wrong-arch target
```

Uses `jt=0, jf=1` (fall-through on match, skip 1 on miss) so the 8-bit
`jt` / `jf` offsets never overflow regardless of set size.

The SIGSYS handler reinterprets the `*mut siginfo_t` through a
`#[repr(C)] struct SigsysSiginfo` (mirroring the kernel's
`__sifields.__sigsys` layout — `signo`, `errno`, `code`, `_pad`,
`call_addr`, `syscall`, `arch`) to read `si_syscall` (the `libc` crate
doesn't expose it as a method). It then classifies the syscall, sets
the return-value register (`x0` on aarch64, `rax` on x86_64), and
advances PC past the syscall instruction (4 bytes on aarch64, 2 bytes
on x86_64). Trapped syscalls currently return 0 (success); the
production version needs to dispatch `mount` → `mount_mgr::bind_mount()`,
`umount2` → unbind, `reboot` → `-EPERM`, etc.

#### 5.5.5 What's NOT here yet (follow-ups)

1. **Full device inventory** — VM creates 20+ devices (`/dev/vmproc`,
   `/dev/__kmsg__`, `/dev/__properties__`, `/dev/ashmem`,
   `/dev/socket/*`, `/dev/block/vdc`, `/dev/fuse`, netlink sockets, …).
   Skeleton has 6.
2. **Binder virtualisation** — per-VM `/dev/binder` + Java-side
   `IActivityManager` proxy. Not started. (Hardest piece — see §10.2.)
3. **`/proc` dynamic files** — `/proc/self/maps`, `/proc/self/status`,
   `/proc/<pid>/…` require shadowhook interception of `open` / `openat`.
   Skeleton uses static files only.
4. **Per-syscall emulation** — `seccomp::emulate_syscall()` returns 0
   for all trapped syscalls.
5. **`mknodat`-based socket creation** — skeleton uses
   `UnixListener::bind()` (creates the socket file as a side effect);
   VM uses `mknodat(S_IFSOCK)` + `bind()` (requires `CAP_MKNOD`).
6. **GSI ROM extractor** — the daemon expects the rootfs to already
   contain `/system`, `/vendor`, etc.
7. **Workspace integration** — `kr64` is not yet a member of the
   parent `twoyi` Cargo workspace; `build_rs.sh` doesn't build it yet.

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
    ├── src/                           ← the `twoyi` crate (libtwoyi.so)
    │   ├── lib.rs                     ← JNI_OnLoad + native method regs
    │   ├── core.rs                    ← renderer dispatch + guest spawn + DATA_DIR (§4.7)
    │   ├── input.rs                   ← UPDATED: keycode mapping + bitmask + dynamic paths
    │   ├── renderer_bindings.rs       ← FFI to AOSP libOpenglRender.so (§5.4)
    │   ├── renderer_new/              ← open-source Rust renderer (§5.3)
    │   └── interp.c                   ← .interp segment for PIE
    ├── loader/                        ← the `loader` crate (libloader.so)
    │   ├── build.sh                   ← UPDATED: multi-ABI support
    │   └── src/lib.rs
    ├── openglrenderer/                ← the `openglrenderer` crate (Rust alt renderer, §5.3)
    │   ├── build.sh                   ← UPDATED: multi-ABI support
    │   └── src/
    └── kr64/                          ← NEW (§5.5): kernel-replacement daemon skeleton
        ├── Cargo.toml                 ← crate-type = ["cdylib", "rlib"] + [[bin]]
        ├── build.rs                   ← PIE flags; Android-only --dynamic-linker
        ├── interp.c                   ← .interp segment for PIE
        └── src/
            ├── main.rs                ← bin entry point
            ├── lib.rs                 ← crate root, Config, parse_args, run
            ├── devices.rs             ← virtual /dev tree (6 MVP sockets)
            ├── seccomp.rs             ← BPF filter + SIGSYS handler
            ├── proc_emu.rs            ← synthesised /proc tree
            └── mount_mgr.rs           ← unshare + pivot_root + tmpfs mounts
```

The repo also carries a `download/` directory of analysis reports and
built artifacts (not part of the shipped APK):

```
download/
├── aosp-built/                       ← AOSP-rebuilt libOpenglRender.so (§5.4)
│   ├── libOpenglRender_aosp_arm64.so     (610,720 B)
│   └── libOpenglRender_aosp_x86_64.so    (605,152 B)
├── port_files/                       ← C++ sources ported from legacy (§5.4.3)
│   ├── dl_ex.cpp
│   ├── GraphicBuffer.{h,cpp}
│   ├── startGBServer.cpp
│   ├── CMakeLists.txt
│   └── patch_twoyi_api.py
├── GSI_BOOT_PLAN.md                  ← the 997-line GSI boot roadmap (§10)
├── KR64_SKELETON.md                  ← design doc for the kr64 crate (§5.5)
├── PROJECT_SUMMARY.md                ← definitive project state write-up
├── VM_JAVA_ANALYSIS.md               ← VM Java reverse-engineering (§9)
├── VM_KR64_ANALYSIS.md               ← VM libkr64.so reverse-engineering (§9)
├── VM_DEEP_DISASSEMBLY.md            ← VM libvm.so deep disassembly (§9)
├── VM_ROM_ANALYSIS.md                ← VM ROM catalog / decryption (§9)
├── FUNCTION_LEVEL_COMPARISON.md      ← legacy vs AOSP renderer comparison
├── AOSP_BUILD_RESULTS.md             ← AOSP rebuild process (§5.4)
├── AOSP_VS_LEGACY_COMPARISON.md
├── PORT_RESULTS.md                   ← port of dl*_ex / GraphicBuffer / startGBServer
├── TWOYI_DISASSEMBLY_ANALYSIS.md     ← legacy libOpenglRender.so disassembly
├── TWOYI_HONEST_STATUS.md            ← what actually works today (§5.4.5)
├── VIRTUAL_MASTER_ANALYSIS.md        ← early VM overview (superseded by VM_*)
└── VIRTUAL_MASTER_FULL_ANALYSIS.md
```

---

## 9. Virtual Master — reverse-engineering comparison

**APK analyzed:** `com.clone.android.dual.space` v3.2.53
(`Virtual Master` — a competing closed-source rootless
Android-on-Android container, much more advanced than twoyi).

Six reverse-engineering reports (totalling ~4,000 lines) live under
`download/`: `VM_ROM_ANALYSIS.md`, `VM_JAVA_ANALYSIS.md`,
`VM_DEEP_DISASSEMBLY.md`, `VM_KR64_ANALYSIS.md`,
`VIRTUAL_MASTER_ANALYSIS.md`, `VIRTUAL_MASTER_FULL_ANALYSIS.md`.
This section is the executive summary.

### 9.1 What VM does that twoyi doesn't

| Capability | Twoyi (this branch) | Virtual Master v3.2.53 |
|---|---|---|
| **Kernel-replacement daemon** | ❌ Skeleton only (§5.5) | ✅ `libkr64.so` — standalone ELF disguised as `.so`, custom `libkrloader64.so` interpreter, 20+ virtual devices, mount namespaces, seccomp+SIGSYS, `/proc` emulation, shadowhook v1.0.8 (ByteDance) |
| **Binder virtualization** | ❌ Uses host binder | ✅ Per-VM `/vm%d/dev/binder` + Java `IActivityManager` proxy via `BinderService.setupBinder()` JNI |
| **Multi-VM** | ❌ Single VM | ✅ Up to 4 concurrent (`VMStartActivity0..3` with `.vm0..3` taskAffinity) |
| **Renderer pattern** | Global singleton (`FrameBuffer::s_theFrameBuffer`) | **Per-VM renderer handle** (`DisplayService.nativeAddSurface(ptr, surfaceId, surface, w, h, rot)`) — `jlong ptr` is a per-VM handle |
| **HAL services** | ❌ | ✅ Display, Input, Audio, Camera (Camera1 proxy), Sensor (12 types), Location, WiFi scan, Phone (TelephonyManager proxy), Battery, Network (tun0) — each in its own HandlerThread |
| **`/proc` emulation** | ❌ (uses host `/proc`) | ✅ Intercepts `open("/proc/…")` via shadowhook — synthesises `cmdline`, `version`, `self/maps`, `self/status`, `self/mounts`, `self/exe`, `net/if_inet6/`, `sys/kernel/kptr_restrict`, `sys/vm/mmap_rnd_bits` |
| **State machine** | Implicit (boot log lines) | ✅ Explicit 11-state machine in `VMInstance.f8940WWoWWo` (`-5..7`) with EventBus `VMStatusEvent` broadcasts |
| **GSI support** | ❌ (custom 8.1 ROM only) | ✅ Treble GSIs for Android 9 + 11 (VINTF manifest, `/system/product/`, `/system/system_ext/`, `/vendor/`) |
| **ROM distribution** | Bundled `rootfs.7z` in APK | Downloaded from server at runtime, 6 versions (4.2.2 / 5.1.1 / 7.1.2 32-bit / 7.1.2 64-bit / 9.0.0 / 11.0.0), 66–351 MiB each, AES-128-ECB or XOR decrypted on the fly via `CipherOutputStream` (key `%z89aviCM0KkbEs9`) |
| **Inline hooking** | ❌ | ✅ shadowhook v1.0.8 — hooks `do_dlopen`, `open`, `mount`, `__system_property_get` |
| **FreeReflection bypass** | ❌ | ✅ base64-encoded dex with `me.weishu.reflection.BootstrapClass.exemptAll()` |
| **String obfuscation** | None | ✅ OLLVM control-flow flattening + StringFog (Vigenère-XOR with per-block keys); `.symtab` stripped; 77 `.datadiv_decode*` exported string-decoder thunks |

### 9.2 Where twoyi is ahead

| Capability | Twoyi (this branch) | Virtual Master v3.2.53 |
|---|---|---|
| **ABIs supported** | `arm64-v8a` + `x86_64` | `arm64-v8a` + `armeabi-v7a` only |
| **Open-source renderer** | ✅ AOSP-source rebuild (§5.4) + Rust alt (§5.3) | ❌ All closed |
| **Renderer size** | 605–611 KB | 7.7 MB (`libvm.so`, OLLVM-bloated) |
| **Debuggability** | ✅ Full debug symbols, no obfuscation | ❌ Stripped `.symtab`; OLLVM flattening |
| **License** | MPL-2.0 (renderer Apache-2.0) | Proprietary |
| **Loader** | Rust `libloader.so` (51 KB) | `libkrloader64.so` (217 KB, custom ELF interpreter) |

### 9.3 The 20+ virtual devices VM materialises

Decoded from `libkr64.so`'s XOR-obfuscated `.data` section (per-string
keys, recovered via brute-force single-byte XOR scan). Each socket path
lives under `<vmDataDir>/vm/vm%d/dev/`:

| Device | Path | Purpose |
|---|---|---|
| Process-info procfs | `/dev/vmproc` | Per-VM `/proc` emulation backend |
| Kernel log | `/dev/__kmsg__`, `/dev/__kmsg2__` | `printk` capture |
| Daemon log | `/dev/__krlog__` | `libkr64`'s own log stream |
| Property area | `/dev/__properties__` | mmap'd system properties |
| Shared memory | `/dev/ashmem`, `/dev/ashmemsim` | Binder transactions, SurfaceFlinger |
| Mount markers | `/dev/.busybox`, `/dev/.coldboot_done` | Boot-progress sentinels |
| Process-pid socket | `/dev/socket/process_pid` | Per-VM PID coordination |
| Logger sockets | `/dev/socket/logdw`, `/dev/socket/logdr` | `logd` wire interface |
| Touch input | `/dev/input/touch` | Same path as twoyi |
| QEMU pipe | `/dev/qemu_pipe` (Android 7), `/dev/goldfish_pipe` (Android 11) | GL transport |
| Graphics buffer | `/dev/gb`, `/dev/gb2` (Android 11 only) | `AHardwareBuffer` FD-passing proxy |
| Block device | `/dev/block/vdc` (Android 11) | Virtual disk |
| FUSE | `/dev/fuse` (Android 11) | Filesystem-in-userspace |
| HAL power supply | `/dev/hal/power_supply%s` (Android 11) | Battery HAL shim |
| Binder | `/vm%d/dev/binder` | Per-VM binder driver proxy |
| Netlink sockets | `/vm%d/dev/netlink_server`, `/vm%d/dev/netlink_client/nl_dhcp_%d_%d`, `/vm%d/dev/netlink_client/netdevice_%d_%d` | Network HAL |

Twoyi's `kr64` skeleton (§5.5) currently materialises 6 of these
(`qemu_pipe`, `touch`, `key0`, `event`, `gb`, `gb2`). The full
inventory is a follow-up task.

### 9.4 The VM boot state machine

`com.android.vmcore.VMInstance.f8940WWoWWo` — 11 states:

```
-5 = STOPPING            0 = STOPPED          4 = BOOTING
-4 = BOOT_FAILED         1 = CHECKING_ENV     5 = RUNNING
-3 = SVC_FAILED           2 = INSTALLING       6 = BOOT_COMPLETED
-2 = INSTALL_FAILED      3 = STARTING_SVC     7 = SHUTDOWN
-1 = ENV_FAILED
```

Each transition fires an EventBus `VMStatusEvent`. The two task
pipelines:

- **SetupTasks** (state 2): `PrepareFs → InstallFs → FixFs → CleanFs →
  ChmodFs → CleanCache → FixCPUArch → LoadVMProp`.
- **StartupTasks** (state 4): `ApplyOverlays → Bug1..Bug8 → CleanLog →
  Superuser → Xposed → GooglePlay → Magisk → BuildTmpfs → BuildVMProp →
  BuildExecPath`.

Then `startOS(vmId, dpi, kernelPath)` JNI call (kernelPath = `dataDir +
"/lib64"`).

> Twoyi's implicit boot-log-line approach (see §4.5) is technically
> sufficient but provides no programmatic state to the UI. Adopting
> VM's explicit state machine is recommended (see §11.4 below).

### 9.5 The three IPC channels to the guest

| Channel | Mechanism | Used for |
|---|---|---|
| A | Unix domain socket at `<vmDataDir>/dev/event` (Java `LocalServerSocket`) | 25+ event types (`BOOT_COMPLETED`, `SHUTDOWN`, `START_INSTALL_APP`, `CLIPBOARD_DATA`, `SEND_KEY_EVENT`, `EXECUTE_COMMAND`, …). Messages are UTF-8 strings of form `eventName` + backtick + `payload`. |
| B | Binder virtualization via `BinderService.setupBinder(vmId, ...)` JNI | Creates per-VM `/vm%d/dev/binder`, proxies host's `android.app.IActivityManager` IBinder through a Java `Proxy` so the guest's `servicemanager` thinks it's talking to a real OS. |
| C | `/dev/qemu_pipe` (native-only) | GL transport — same mechanism as twoyi. |

Twoyi has channel C and a stripped-down channel A (only `BOOT_COMPLETED`,
`SWITCH_HOST`, `SETTINGS`). Twoyi does not have channel B at all.

### 9.6 Key correction to earlier analysis

The earlier `VIRTUAL_MASTER_ANALYSIS.md` claimed VM uses `NativeActivity`
and `TextureView`. The deeper `VM_JAVA_ANALYSIS.md` corrected this: VM
uses **`VMDisplayActivity extends BaseActivity`** (a regular
`AppCompatActivity`) and **`SurfaceView`**. The earlier claim was based
on the presence of `ANativeActivity_onCreate` in `libvm.so`'s exports —
but that symbol is just NDK app-glue boilerplate that doesn't get
called.

---

## 10. GSI Boot Roadmap

**Definitive plan:** `download/GSI_BOOT_PLAN.md` (997 lines, 9
sub-projects with file paths, acceptance criteria, and a recommended
milestone order). This section is the executive summary.

### 10.1 What a GSI is

A **GSI (Generic System Image)** is an Android `system.img` conforming
to the Treble HAL interface contract (introduced in Android 8.0,
Project Treble). It ships `system.img` + `product.img` +
`system_ext.img` (and a `boot.img` for kernel+ramdisk). The `vendor.img`
must be supplied by the device. In a container, we reuse the host
kernel — but we still must synthesise everything else a Treble boot
expects (`/dev/binder`, `/dev/hwbinder`, `/dev/vndbinder`,
`/dev/ashmem` or `/dev/dm-user`, `/dev/__properties__`, `init`,
`servicemanager`, `surfaceflinger`, a gralloc HAL, the HALs declared in
`/vendor/etc/vintf/manifest/*.xml`, and a procfs/sysfs that look real).

### 10.2 The 9 sub-projects

| § | Sub-project | Twoyi location | Status |
|---|---|---|---|
| 3.1 | **Kernel-replacement daemon** | `app/rs/kr64/` (§5.5) | 🟡 Skeleton done (6 of 20+ devices) |
| 3.2 | **Binder virtualization** | `app/rs/kr64/src/binder_proxy.rs` + Java `BinderService.java` (new) | 🔴 Not started (hardest piece) |
| 3.3 | **Graphics buffer management** (`/dev/gb` + `/dev/gb2`) | `app/rs/kr64/src/gb.rs` (new) + the ported `startGBServer` in `libOpenglRender.so` (§5.4.3) | 🟡 Skeleton done in renderer; not wired to `FrameBuffer::createColorBuffer` yet |
| 3.4 | **Seccomp filter** | `app/rs/kr64/src/seccomp.rs` | 🟡 Skeleton done (filter + SIGSYS handler exist; `emulate_syscall()` returns 0) |
| 3.5 | **`/proc` emulator** | `app/rs/kr64/src/proc_emu.rs` | 🟡 Skeleton done (static files only; no shadowhook interception) |
| 3.6 | **Inline hooking** | `app/rs/kr64/src/hooks.rs` (new) — LD_PRELOAD for MVP, simpler than shadowhook | 🔴 Not started |
| 3.7 | **GSI-aware ROM extraction** | `GsiExtractor.java` + `app/rs/gsi_extractor/` Rust crate (new) | 🔴 Not started |
| 3.8 | **Init configuration** | `GsiInitPatcher.java` (new) — patches `build.prop`, `init.rc`, `vendor/etc/init/*.rc` | 🔴 Not started |
| 3.9 | **HAL virtualization** (12 HALs) | Various Java + Rust modules | 🔴 Not started |

### 10.3 HAL priority (GSI_BOOT_PLAN §3.9)

| Priority | HALs |
|---|---|
| **Critical** (MVP blockers) | graphics allocator, graphics mapper, graphics composer |
| **High** | audio, keymaster, gatekeeper |
| **Medium** | health, power, vibrator |
| **Low** (stubs OK for MVP) | sensors, camera, gps, wifi, telephony, bluetooth |

### 10.4 Recommended milestone order

| Weeks | Milestone |
|---|---|
| 1–2 | Device tree creation (`/dev/qemu_pipe`, `/dev/input/touch`, `/dev/event` socket) — foundational (extends §5.5) |
| 2–3 | GSI extractor + GSI init patcher |
| 3–4 | Graphics HAL (allocator / mapper / composer) |
| 4–5 | `/dev/gb` + `/dev/gb2` (wire §5.4.3's `GraphicBuffer::Main` into `FrameBuffer::createColorBuffer`) |
| 5–6 | Stub HALs → boot to launcher |
| 6–8 | `/proc` emulator + seccomp |
| 8–12 | Binder virtualization (the hardest piece) |
| 12+ | Audio / camera / sensors / gps / wifi / telephony / bluetooth HAL proxies |

**Total estimate:** 8–12 weeks for an MVP that boots to launcher; 16–24
weeks for full VM parity.

### 10.5 x86_64 path

All infrastructure is in place on x86_64:

- Codespace has KVM (AMD EPYC 7763, EastUs, seccomp:0).
- AOSP x86_64 renderer is built (605 KB — §5.4).
- Rust crates already build for x86_64.
- x86_64 GSIs are downloadable from `ci.android.com`.

The x86_64 boot flow is the same as arm64 except the GSI must be x86_64
(there is no binary translation in the container path — twoyi shares the
host kernel).

> A separate **KVM alternative** is documented in GSI_BOOT_PLAN §5.5:
use `crosvm` or QEMU to boot the GSI in a real VM. Much simpler
conceptually but requires an Android-common kernel and a different
overall architecture. Out of scope for the container path.

### 10.6 What's verifiably done vs. what's not

| Item | Status | Evidence |
|---|---|---|
| `libOpenglRender.so` builds from AOSP source | ✅ | `download/aosp-built/libOpenglRender_aosp_{arm64,x86_64}.so` |
| `kr64` daemon compiles + passes 26 unit tests | ✅ | `app/rs/kr64/` + `download/KR64_SKELETON.md` |
| `kr64` smoke test on Linux creates all 6 devices | ✅ | `KR64_SKELETON.md` §4 |
| `kr64` is built by `build_rs.sh` and shipped in `jniLibs/` | ❌ | Not yet a workspace member |
| `libOpenglRender.so` renders guest GL output on a real device | ❌ | Not verified end-to-end (see `download/TWOYI_HONEST_STATUS.md`) |
| A GSI boots inside twoyi | ❌ | Requires §10.2 items 2, 6, 7, 8 — none started |

---


## 11. Improvement opportunities (still open)

Things this branch does NOT fix, but should be done next:

### 11.1 Build & dependency modernization (low risk, high payoff)

- Bump AGP from 7.1.1 to 8.x.
- Bump `targetSdkVersion` from 28 to 34+.
- Migrate `cargo-xdk` to `cargo-ndk`.
- Bump all AndroidX libraries.
- Bump Rust crates (`jni:0.19` → `0.21`, `ndk:0.6` → `0.9`, etc.).
- Add `kr64` to the parent Cargo workspace and to `build_rs.sh` so it
  ships inside the APK (currently standalone — see §5.5.5 item 7).

### 11.2 Replace closed-source blobs (medium risk, very high payoff)

| Blob | Status | Replacement |
|---|---|---|
| `libloader.so` | ✅ Built from source (Rust) | `app/rs/loader/` (§5.2). Delete the legacy `.so` and update `RomManager.LOADER_FILE` to point at `libloader_new.so`. |
| `libOpenglRender.so` | ✅ Built from source (AOSP) | `download/aosp-built/` (§5.4). All 11 twoyi-required C-ABI symbols exported. Pending: end-to-end device verification (see §5.4.5). |
| `libadb.so` | ❌ Still closed | Replace with an open-source Java ADB client (e.g., `adblib` from AOSP, or `github.com/MuntashirAkon/adb-android`). |

### 11.3 Architecture cleanups

- Deduplicate `renderer_new/` (inside `libtwoyi.so`) and the standalone
  `openglrenderer/` crate — they are nearly identical code.
- Convert `Renderer.java` to Kotlin.
- Replace `jdeferred` with `CompletableFuture`.
- Replace `com.cleveroad:androidmanimation:0.9.1` with `android.animation.*`.
- Replace `com.github.clans:fab:1.6.4` with Material `ExtendedFloatingActionButton`.

### 11.4 Runtime behavior improvements (informed by §9 — VM comparison)

- Add an explicit boot state machine to `TwoyiStatusManager` mirroring
  VM's 11-state machine (§9.4). Today twoyi uses implicit boot-log-lines
  which gives the UI no programmatic state.
- Refactor `libOpenglRender.so` to take a per-instance handle matching
  VM's `DisplayService.nativeAddSurface(ptr, surfaceId, surface, w, h,
  rot)` pattern (§9.1). This unblocks multi-VM and multi-surface.
- The 60-second boot timeout doesn't differentiate between "still
  booting" and "stuck". Add a heartbeat mechanism.
- The dalvik-cache clear is synchronous on the UI thread. Move to a
  coroutine.
- `patchServicesJarForPackageInstaller` should be moved to install time.

### 11.5 GSI boot (the headline outstanding project — see §10)

- Wire `kr64` into the build (`build_rs.sh` + parent `Cargo.toml`
  workspace) so `libkr64.so` ships in `jniLibs/`.
- Extend the `kr64` device inventory from 6 to the full 20+ (§5.5.5
  item 1).
- Implement per-syscall `emulate_syscall()` dispatch (§5.5.5 item 4).
- Implement binder virtualization (§10.2 item 2 — the hardest piece).
- Write the GSI extractor + GSI init patcher (§10.2 items 7, 8).
- Implement the graphics HAL (allocator / mapper / composer) so
  SurfaceFlinger can composite (§10.3).
- Boot to launcher on x86_64 (estimated 8–12 weeks of work).

### 11.6 Missing features

- Android 10 / 12 / 13 guest support.
- Multi-window / multi-VM mode (requires §11.4 per-instance renderer
  refactor first).
- Build the legacy 8.1 ROM from source via `default.xml`. The manifest
  exists; what's missing is build orchestration in CI.

### 11.7 Quality-of-life improvements

- Add instrumented tests.
- Replace Microsoft AppCenter (being retired) with Sentry or self-hosted.
- Remove `moe.feng:AlipayZeroSdk:1.1` (donation SDK).
- Refresh `README_CN.md` to match the rewritten `README.md`
  (the Chinese translation still references the old discontinued-project
  README).

---

## 12. Setting up a development environment

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

## 13. References

- **Active fork source**: https://github.com/cyanmint/twoyi
- **This branch's fork**: https://github.com/Disable-OP/twoyi
- **Original archived repo**: https://github.com/twoyi/twoyi
- **cyanmint's Nogitsune (twoyi v2 rewrite)**: https://github.com/cyanmint/Nogitsune
- **Project website**: https://twoyi.app
- **Original author**: weishu — https://github.com/tiann
- **cargo-xdk**: https://github.com/tiann/cargo-xdk
- **Anbox** (graphics/pipe design twoyi's renderer follows): https://github.com/anbox/anbox
- **AOSP `platform/sdk` source** (rebuilt into `libOpenglRender.so`, §5.4):
  https://android.googlesource.com/platform/sdk/+/7a712acc02282985dcd32feb81284e1f2b19ec7e
- **AOSP android-8.1.0_r81 reference** (the `default.xml` manifest):
  https://android.googlesource.com/platform/manifest/+/refs/tags/android-8.1.0_r81
- **DEX bytecode reference**:
  https://source.android.com/docs/core/runtime/dex-format
- **Codespaces KVM**:
  https://github.com/devcontainers/images/issues/884
  https://github.com/dotnet/runtime/issues/77851
- **Redroid (Android-in-container)**: https://github.com/remote-android/redroid-doc
- **Project Treble / GSI** (§10): https://source.android.com/docs/core/architecture/halse
  and https://source.android.com/docs/core/ota/gsi
- **shadowhook** (ByteDance inline-hook lib used by VM's `libkr64.so`):
  https://github.com/bytedance/android-inline-hook

**Analysis reports under `download/`** (see §8 file map for the full
list):
- `GSI_BOOT_PLAN.md` — the 997-line GSI boot roadmap (§10).
- `KR64_SKELETON.md` — design doc for the `kr64` crate (§5.5).
- `PROJECT_SUMMARY.md` — the definitive state-of-the-project write-up.
- `VM_JAVA_ANALYSIS.md`, `VM_KR64_ANALYSIS.md`, `VM_DEEP_DISASSEMBLY.md`,
  `VM_ROM_ANALYSIS.md` — Virtual Master reverse-engineering (§9).
- `AOSP_BUILD_RESULTS.md`, `FUNCTION_LEVEL_COMPARISON.md`,
  `PORT_RESULTS.md`, `AOSP_VS_LEGACY_COMPARISON.md`,
  `TWOYI_DISASSEMBLY_ANALYSIS.md` — the open-source-renderer rebuild
  chain (§5.4).
- `TWOYI_HONEST_STATUS.md` — what actually works today (§5.4.5).

---

*This document was produced by reading every source file in the
`cyanmint/twoyi` repository at commit `25ef89c` plus the improvements in
this branch, the 13 analysis reports under `download/`, and the full
worklog. It is intended as a living reference — if you improve the
project, please update the corresponding section.*
