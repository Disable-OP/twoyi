# Code Style Guide

Patterns used in twoyi, with file + line references. Where existing
code disagrees, the existing code wins — fix both. For one-line
conventions see `CONTRIBUTING.md` §3; this guide documents the
**patterns**.

---

## 1. Rust patterns

### 1.1 Module structure

- **`app/rs/`** — host renderer + input (`libtwoyi.so`). Entry points
  `JNI_OnLoad` (`app/rs/src/lib.rs:241`) and PIE `main` (`lib.rs:308`).
  Sub-modules at `lib.rs:15-18`; new renderer is a directory module
  (`app/rs/src/renderer_new/mod.rs:29-33`).
- **`app/rs/kr64/`** — kernel-replacement daemon (`libkr64.so` + `kr64`
  bin). Crate root `lib.rs` declares all eight feature modules at lines
  67-74; `src/main.rs` is a 38-line bin wrapper calling `kr64::run()`.
- One concern per file. Every module starts with a `//!` block
  explaining *what* it mirrors in Virtual Master and *what's stubbed* —
  `audio.rs:12-130` is canonical (wire protocol as ASCII table). File
  headers carry MPL-2.0 + AI-generated disclaimer; match
  `app/rs/kr64/src/lib.rs:1-10`.

### 1.2 Error handling

- **`std::io::Result<T>`** for fs / socket / syscall code —
  `bind_unix_socket` (`app/rs/kr64/src/devices.rs:136`).
- **`Result<T, String>`** for arg parsing — `parse_args` (`lib.rs:207`)
  so `--help` text rides back as `Err` (caller dispatches at
  `lib.rs:315-326`).
- Use `?` everywhere. Convert non-`io` errors with
  `.map_err(|e: std::num::ParseIntError| e.to_string())?` (`lib.rs:254`).
- **Non-fatal failures are logged, not propagated**: `match { Ok(h) =>
  Some(h), Err(e) => { warning!(...); None } }` — audio handle
  (`lib.rs:390-406`). Guest can boot without sound.
- **Fatal failures in the forked child** use `libc::_exit(1)` / `_exit(127)`
  — never `panic!` (would unwind across the fork). `lib.rs:516`, `lib.rs:546`.
- `Drop` impls never return errors — `let _ = fs::remove_file(...)` and
  move on (`DeviceSocket::drop`, `devices.rs:107-115`).

### 1.3 Testing patterns

- Every module ends with `#[cfg(test)] mod tests { use super::*; ... }`
  placed **inside** the module so it sees private items (`devices.rs:350`,
  `binder.rs:1771`).
- Tests run on the **Linux host** (`cargo test`). `build.rs` gates
  `/system/bin/linker64` PT_INTERP on `target_os == "android"`
  (`app/rs/kr64/build.rs:78`).
- **Parallel-safe tmpdirs**: every fs-touching test calls a `tmpdir()`
  helper backed by an `AtomicU64` counter (`devices.rs:356-366`) to
  avoid `EADDRINUSE` under parallel `cargo test`. Tests clean up with
  `let _ = fs::remove_dir_all(&rootfs);`; `Drop` impls also unlink
  socket files so panicking tests leave no state.
- **End-to-end socket tests** allowed: bind → `spawn()` →
  `UnixStream::connect` → write → read → `drop(handle)`. See
  `binder_proxy_responds_to_version_ioctl` (`binder.rs:1859-1895`).
  Sleep 50 ms after `spawn()` for the accept thread to start.
- **Test naming**: `snake_case` with an outcome verb —
  `parse_args_missing_rootfs_errors` (`lib.rs:751`),
  `classify_mount_is_emulated` (`seccomp.rs:805`). Avoid `test_foo`.
- **Size assertions** pin `#[repr(C)]` ABIs:
  `assert_eq!(std::mem::size_of::<FlatBinderObject>(), 24);`
  (`binder.rs:1804`). Add one whenever a struct crosses a kernel ABI.

### 1.4 Device creation pattern

Most-repeated shape — `devices`, `binder`, `audio`, `sensors`,
`battery` all identical. Copy `audio.rs` as the template; do not invent
a new pattern.

1. **Free fn** `create_<thing>(rootfs, ...) -> io::Result<Thing>`:
   ensure parent dir, remove stale socket (`NotFound` logged silently,
   `devices.rs:142-146`), `UnixListener::bind(path)?`, `chmod 0666` so
   the guest uid can `connect`. See `bind_unix_socket`
   (`devices.rs:136-162`), `create_audio_device` (`audio.rs:453`).
2. **`Thing` struct**: `Option<UnixListener>` (Option so `spawn` can
   `take()` it) + `path: String` + `shutdown: Arc<AtomicBool>`. See
   `AudioDevice` (`audio.rs:516`), `BinderProxy` (`binder.rs:874`).
3. **`Thing::spawn(self) -> io::Result<ThingHandle>`**: consumes `self`,
   takes the listener, spawns `thread::Builder::new().name(format!(
   "kr64-<thing>-{}", vm_id)).spawn(move || { ... })`. The `name` is
   mandatory — `logcat` and `/proc/<pid>/comm` need it. See
   `BinderProxy::spawn` (`binder.rs:930-996`).
4. **`ThingHandle` + `Drop`**: sets `shutdown` (Release ordering), joins
   the accept thread, unlinks the socket (`binder.rs:1001-1029`).
5. Accept thread is **non-blocking** (`fcntl(F_SETFL, O_NONBLOCK)` in
   `BinderProxy::new`, `binder.rs:906`). On `WouldBlock` sleep 25 ms; on
   real error sleep 50 ms and retry (`binder.rs:974-984`).

### 1.5 Thread pool pattern

`kr64` is `libc`-only (no `rayon`/`crossbeam`), so each device module
rolls its own minimal pool — the classic Rust-book design (`audio.rs:885-965`):

- `type Job = Box<dyn FnOnce() + Send + 'static>;`
- `enum Message { Job(Job), Terminate }`
- `struct Worker { thread: Option<JoinHandle<()>> }` blocks on
  `receiver.lock().unwrap().recv()`, exits on `Terminate`/`Err`.
- `ThreadPool::new(size)` asserts `size > 0`, builds `size` workers
  sharing one `Arc<Mutex<mpsc::Receiver<Message>>>`.
- `execute<F: FnOnce() + Send + 'static>` boxes `F`, sends `Job`; `Drop`
  sends `Terminate` once per worker then joins each.

Duplicated verbatim in `binder.rs:1066-1118`. Keep the copies in sync;
the duplication is deliberate (each module is self-contained).

### 1.6 JNI stub pattern

The `kr64` HAL modules need to up-call into Java for real AudioTrack /
SensorManager / BatteryManager access, but the skeleton can't pull in
the `jni` crate. Pattern: declare a `JniObject = *mut c_void` type
alias + a set of `fn jni_<verb>(...)` stubs returning null/0/empty, and
call them from the pump loop. When the real Java side is wired in, only
the stubs change. See `app/rs/kr64/src/audio.rs:827-873` for the six
stubs (`jni_acquire_audio_track`, `jni_write_audio_data`, etc.). The
pump calls these unconditionally — null return closes the connection
gracefully. Same shape in `battery.rs` and `sensors.rs`.

The **parent `twoyi` crate** does the opposite — real JNI, not stubs.
`#[no_mangle] pub fn renderer_init(env: JNIEnv, _clz: jclass, ...)`
(`app/rs/src/lib.rs:44`) is the template. Registration in `JNI_OnLoad`
(`lib.rs:241`) via the `jni_method!` macro (`lib.rs:34-42`) builds a
`NativeMethod` table (`lib.rs:252-270`). **Three files must agree** when
adding a JNI method: `Renderer.java` (declare), `lib.rs` table
(register), `lib.rs` `#[no_mangle] fn` (implement).

### 1.7 Logging macros

- **`kr64` crate** defines its own `info!` / `warning!` / `error!`
  macros expanding to `eprintln!("[KR64 <LEVEL>] {}", ...)`
  (`app/rs/kr64/src/lib.rs:91-118`). Exported `pub(crate) use` so
  sub-modules do `use crate::{info, warning};` (`devices.rs:66`). Named
  `warning!` (not `warn!`) to avoid clashing with `#[warn]` lint
  (`lib.rs:99-101`). Lines go to stderr so they're visible via
  `adb logcat *:S KR64:V` **and** during `cargo test`.
- **`twoyi` parent crate** uses the real `log` crate + `android_logger`,
  initialized in `JNI_OnLoad` with tag `CLIENT_EGL` (`lib.rs:243-247`).
  Sub-modules `use log::{debug, error, info};` (`core.rs:12`, `input.rs:17`).
- **Tag convention**: `[KR64][<module>] <msg>` for kr64, `[CORE]` /
  `[NEW_RENDERER]` for the parent crate. Put the bracket tag inside the
  format string, not as a log filter — keeps `logcat` greppable.

---

## 2. Java patterns

### 2.1 Activity lifecycle

`Render2Activity` (`app/src/main/java/io/twoyi/Render2Activity.java`)
is the template.

- Extends `Activity` (not `AppCompatActivity`) — line 66.
- `SurfaceHolder.Callback` is anonymous field `mSurfaceCallback`
  (line 90). `surfaceCreated` runs `Renderer.setDataDir()` →
  `setRendererType()` → `setDebugRenderer()` → `Renderer.init()` **in
  that order** (lines 99-140). Data-dir must come first because the
  Rust side resolves every path relative to it.
- `AtomicBoolean mIsExtracting` (line 88) guards re-entrant extraction.
  Use `AtomicBoolean.compareAndSet` for one-shot start guards —
  `TwoyiSocketServer.start()` (`TwoyiSocketServer.java:75`).
- `runOnUiThread(...)` for any UI mutation from a worker (line 363).
  Background work via `new Thread(..., "waiting-boot").start()` (line
  367) or `UIHelper.defer().when(...)` (line 457) — **name every thread**.
- `TwoyiApplication.attachBaseContext` (line 42) does early init
  (`ProfileManager.initializeProfiles` → `RomManager.ensureBootFiles` →
  `TwoyiSocketServer.start`) before any Activity, so the host-side
  socket server is listening before the guest boots.

### 2.2 JNI method registration

- Java declares `public static native` methods on a thin class
  (`Renderer.java:34-69`).
- `static { System.loadLibrary("twoyi"); }` loads `libtwoyi.so` on class
  init (`Renderer.java:30-32`).
- Rust registers them in `JNI_OnLoad` via the `jni_method!` macro —
  `app/rs/src/lib.rs:251-270`. Each entry is
  `jni_method!(javaName, rustFn, "(sig)Lret;V")`.
- The Java name **must** match the `native` declaration exactly. Adding
  a JNI method touches three places: `Renderer.java` (declare), `lib.rs`
  table (register), `lib.rs` `#[no_mangle] pub fn` (implement).
- Errors at the JNI boundary: prefer `error!` log + early return over
  throwing a Java exception. See `renderer_init` (`lib.rs:59-65`).

### 2.3 Shell command execution

- All shell access goes through `ShellUtil.newSh()`
  (`app/src/main/java/io/twoyi/utils/ShellUtil.java:21-25`), a
  **non-root** `Shell` via `topjohnwu/Assets`.
- Usage: `ShellUtil.newSh().newJob().add("rm -rf '" + path + "'").exec()`
  (`RomManager.java:294`). `.exec()` blocks; check
  `Shell.Result.getCode()` / `getOut()`.
- For multi-line commands, chain `.add(...)` calls (`RomManager.java:303-306`).
- **Never** interpolate user-controlled paths without sanitising — the
  ROM-import path (`Render2Activity.java:482-485`) rejects paths
  containing `;` or `&`.
- For tar/unzip, prefer `ProcessBuilder` over shell
  (`Render2Activity.java:488-493`).

### 2.4 Settings management

- Two layers. **Global** app flags in `AppKV` — one
  `SharedPreferences` file `"app_kv"` (`AppKV.java:20`). **Per-profile**
  flags in `ProfileSettings` — one file per profile
  `"profile_settings_<name>"` (`ProfileSettings.java:26`).
- Both use the same pattern: `static boolean getBoolean(Context, key,
  default)` / `@SuppressLint("ApplySharedPref") static void
  setBoolean(Context, key, value)`. Uses `.commit()` (synchronous) not
  `.apply()` (async) — `ProfileSettings.java:65`. `@SuppressLint` is
  mandatory because lint prefers `apply`.
- Setting keys are `public static final String` in `SCREAMING_SNAKE_CASE`
  — `ProfileSettings.java:29-34`.
- Defaults can be **architecture-dependent**: `useNewRenderer` defaults
  to `false` on `arm64-v8a` (legacy blob available) and `true` elsewhere
  — `ProfileSettings.java:173-176`. Match this when a feature's
  availability depends on the ABI.

---

## 3. Build patterns

### 3.1 Cargo.toml structure

Four crates, each self-contained (no Cargo workspace — each built by
`cargo xdk`):

- `app/rs/Cargo.toml` — `name = "twoyi"`, `crate-type = ["cdylib"]`,
  full dep list (`log`, `android_logger`, `ndk`, `jni`, `uinput-sys`,
  `unix_socket`). Patches `uinput-sys` to a forked git repo (lines 34-35).
- `app/rs/kr64/Cargo.toml` — `name = "kr64"`, **dual target**:
  `crate-type = ["cdylib", "rlib"]` + `[[bin]]` at `src/main.rs` (lines
  27-33). Deps are `libc` only — rationale at `lib.rs:60-66`.
- `app/rs/loader/Cargo.toml` and `app/rs/openglrenderer/Cargo.toml` —
  `crate-type = ["cdylib"]` for direct use as `libloader.so` /
  `libOpenglRender.so`.

Every `Cargo.toml` carries the AI-generated copyright disclaimer as the
file header (lines 1-6 of each). `edition = "2021"` everywhere.

### 3.2 build.rs usage

`build.rs` is for **link-time configuration only** — never codegen.

1. **Compile the `.interp` C shim**:
   `cc::Build::new().file("interp.c").compile("interp");` —
   `app/rs/kr64/build.rs:33-35` and `app/rs/build.rs:54-56`. Puts the
   PT_INTERP string into the final `.so`.
2. **Emit PIE linker flags** via `cargo:rustc-cdylib-link-arg=`. Flag
   set documented inline at `app/rs/kr64/build.rs:37-76`. Key flags:
   `-Wl,-e,kr64_main` (entry),
   `-Wl,--dynamic-linker=/system/bin/linker64` (PT_INTERP, **Android
   only**), `-Wl,-rpath,$ORIGIN` (sibling `.so`s),
   `-Wl,--undefined=interp` (retain the `.interp` section).
3. **Detect the target ABI** and gate Android-only behaviour on
   `CARGO_CFG_TARGET_OS == "android"` (`build.rs:63-64`) so `cargo
   test` on the Linux host still links and runs. The parent crate's
   `build.rs` (`app/rs/build.rs:23-49`) gates the legacy-blob link
   path on whether `libOpenglRender.so` actually exists in
   `jniLibs/<abi>/`.

`println!("cargo:rerun-if-changed=interp.c");` is mandatory — without
it, `build.rs` won't re-run when the C file changes.

### 3.3 PIE executable pattern

The "PIE hack" lets a `.so` be `exec`'d directly: `./libkr64.so arg1 arg2`.
This is how the Java side exec's the kr64 daemon without a separate
binary. Three pieces must agree:

1. **`interp.c`** — one line: `const char interp[]
   __attribute__((section(".interp"))) = "/system/bin/linker64";`
   (`app/rs/src/interp.c` / `app/rs/kr64/interp.c`, identical).
2. **Linker flags** in `build.rs` — `-Wl,-e,<entry>` (sets ELF entry to
   a Rust `#[no_mangle] pub extern "C" fn` instead of `_start`), `-pie`,
   and the PT_INTERP override. See §3.2.
3. **The Rust entry fn** — `#[no_mangle] pub extern "C" fn kr64_main(
   argc: c_int, argv: *const *const c_char) -> c_int`
   (`app/rs/kr64/src/lib.rs:665-681`). Converts C `argv` → `Vec<String>`
   and calls `run(args)`. Parent crate uses the same shape but names the
   entry `main` (`app/rs/src/lib.rs:308`).

`#[used] static INTERP_REF` (`lib.rs:692-693`) works around linker GC —
without it, LTO may strip the `.interp` section as unused.

### 3.4 POSIX sh scripts

- `build_rs.sh` is deliberately **POSIX `sh`-compatible** (no bash
  arrays) so it runs under `dash` on Ubuntu CI. Comment at
  `app/rs/build_rs.sh:18-23`. Use space-separated strings for lists
  (lines 47-80) and `for ABI in $ABIS; do ... done` for iteration.
- `set -e` at the top of every script (`build_rs.sh:24`).
- ABI → target-triple mapping is a `case` statement, not an associative
  array — `abi_to_target()` (`build_rs.sh:31-37`).
- `twoyi.sh` is the **on-device** wrapper — `#!/system/bin/sh` (not
  `#!/usr/bin/env bash`), 30 lines, `exec /system/bin/linker64
  "$LIB_PATH" "$@"` (`app/rs/twoyi.sh`). Copied into
  `jniLibs/<abi>/twoyi` by `build_rs.sh:110`.

---

## 4. Naming conventions

### 4.1 File naming

- **Rust modules**: `snake_case.rs`. Module directories use
  `snake_case/mod.rs` (`renderer_new/mod.rs`). One module per file.
- **Rust crates**: lowercase, no hyphens (`name = "kr64"`). Output is
  `lib<name>.so`.
- **Java files**: `PascalCase.java`, one top-level class per file.
- **C files**: `lowercase.c` (`interp.c`).
- **Shell scripts**: `snake_case.sh` (`build_rs.sh`, `twoyi.sh`).
- **Docs**: `SCREAMING_SNAKE_CASE.md` — matches existing `download/*.md`.

### 4.2 Function naming

- **Rust public API**: `snake_case` (`create_qemu_pipe`, `parse_args`,
  `bind_unix_socket`). Constructors `Thing::new`, lifecycle methods
  `Thing::spawn` / `ThingHandle::shutdown`.
- **Rust FFI**: `#[no_mangle] pub extern "C" fn` named after the Java
  method they implement — `renderer_init`, `set_data_dir`, `handle_touch`
  (`app/rs/src/lib.rs`). The `kr64_main` PIE entry is the only exception
  (matches the link flag).
- **Rust private helpers**: `snake_case`, often subsystem-prefixed
  (`bind_unix_socket`, `ensure_parent_dir`, `spawn_accept_thread`).
- **Java methods**: `camelCase`. JNI declarations on `Renderer.java`
  match the C side verbatim (`init`, `resetWindow`, `setRendererType`).
- **Test functions**: `<unit>_<condition>_<outcome>` —
  `parse_args_missing_rootfs_errors`, `classify_mount_is_emulated`,
  `binder_proxy_responds_to_version_ioctl`. The trailing verb states the
  expected result.

### 4.3 Struct naming

- **Rust structs**: `PascalCase`. Plain-data holders like `Config`
  (`lib.rs:132`), `DeviceSet` (`devices.rs:276`) use `pub` fields.
  Stateful owners like `BinderProxy` (`binder.rs:874`), `AudioDevice`
  (`audio.rs:516`) use private fields + accessor methods.
- The **Handle** suffix is reserved for the thing returned by `spawn()`
  — `BinderProxyHandle`, `AudioDeviceHandle`, `BatteryDeviceHandle`. The
  **Device** suffix is for the pre-spawn state. Don't mix them.
- **`#[repr(C)]`** structs mirroring kernel ABIs (`BinderWriteRead`,
  `FlatBinderObject`, `device_info`) use the kernel's C name verbatim so
  docs match `<uapi/linux/…>`. `#[allow(dead_code)]` because we use only
  a subset of fields.
- **Java classes**: `PascalCase`. Rust enums use `PascalCase` variants
  (`RendererType::Old`, `RendererType::New` at `app/rs/src/core.rs:82-85`).

### 4.4 Constants

- **Rust**: `SCREAMING_SNAKE_CASE`. Module-private `const`
  (`BINDER_IOC_TYPE`, `binder.rs:159`); magic numbers get a named const
  even if used once (`AUDIO_HEADER_MAGIC`).
- **Java**: `SCREAMING_SNAKE_CASE` and `public static final`
  (`ProfileSettings.DISPLAY_WIDTH`, `AppKV.FORCE_ROM_BE_RE_INSTALL`).
- **Rust `static`** (vs `const`) is reserved for things that must have
  an address — `INTERP_REF` (`lib.rs:692`) and the `AtomicBool` /
  `OnceLock` globals in `core.rs:25-94`. Match the existing pattern:
  `OnceLock<String>` for one-time-set globals (`core.rs:34`),
  `Lazy<Mutex<T>>` for mutable globals (`core.rs:88`).
