# Changelog

All notable changes to the **twoyi fork** (active development on the
`improvements/initial-cleanup` branch) are documented in this file.

The format is based on [Keep a Changelog v1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
for the Rust crates (`kr64`, `twoyi-rs`) and to Android's own versioning for the
APK.

Commit hashes below are linked to the `Disable-OP/twoyi` mirror.

## [Unreleased]

This release is the first batch of work on the active fork. It supersedes the
upstream `cyanmint/twoyi` project (which had been declared discontinued and
required NDK r22 or older) with a modernised build toolchain (NDK r27c /
Rust stable), an open-source OpenGL renderer, a kernel-replacement daemon
skeleton, work-profile support, and a functioning CI/devcontainer story.

30 commits since `main` (15 in the initial batch listed below; 15 more in follow-on rounds — see `worklog.md` and `MEMORY.md` for the full per-commit log):

```
ce29754 docs: update ARCHITECTURE.md with all new findings (664→1324 lines)
9249147 docs: rewrite README + add CONTRIBUTING for active fork
570e95e feat(kr64): kernel replacement daemon skeleton
eb13449 feat: rebuilt AOSP libOpenglRender.so with startGBServer + dl*_ex
47f8335 feat: add AOSP-built libOpenglRender.so for arm64 + x86_64
9c4b907 feat: dynamic data directory for work profile support
7664c66 fix(renderer): default to new renderer on x86_64 to prevent SIGABRT
a6e6dbb fix(devcontainer): add sshd feature for gh codespace ssh access
ff1cc37 feat(build): sign release APKs with a test keystore
3628519 fix(devcontainer): use Dockerfile instead of features for Ubuntu base
f8368e9 fix(ci): use correct rootfs URL (rootfs.tar.gz, not rootfs.7z)
719a0db fix(socket): disambiguate EXECUTOR.submit(this::start0) for JDK 17
2085938 fix(build): don't link legacy libOpenglRender.so on x86_64
7858bce fix(input): make copy_to_cstr generic over array element type
d2cfb8d fix(build): make build scripts POSIX-sh compatible
```

### Added

- **AOSP-built `libOpenglRender.so` for `arm64-v8a` and `x86_64`** — the first
  ever open-source build of twoyi's OpenGL renderer, built from AOSP
  `platform/sdk` commit `7a712ac` (Apache-2.0 `emugl/renderer`) using NDK r27c
  / clang 18 / cmake 3.22. All 6 twoyi-required C-ABI functions are exported
  and verified on both ABIs: `startOpenGLRenderer` (renamed from
  `initOpenGLRenderer`), `destroyOpenGLSubwindow`, `repaintOpenGLDisplay`,
  `setNativeWindow` (twoyi-specific), `resetSubWindow` (renamed from
  `createOpenGLSubwindow`), and `removeSubWindow` (twoyi-specific). Replaces
  the legacy 1,059,128-byte closed-source blob with a smaller open-source
  build (~603 KB arm64 / ~597 KB x86_64). ([`47f8335`])

- **`startGBServer`, `dl*_ex` wrappers, and `GraphicBuffer` class** added to
  the AOSP `libOpenglRender.so` build, porting the three missing pieces
  identified by the function-level comparison against the legacy blob.
  `startGBServer` (372 B) is the Graphics Buffer server that receives
  `AHardwareBuffer` FDs from the guest over the `opengles3` Unix socket —
  needed for SurfaceFlinger compositing in GSI boot. The `dl*_ex` family
  (`dlopen_ex`, `dlsym_ex`, `dlclose_ex`, `dlerror_ex`) provides
  Android-7+-aware dynamic-library wrappers with a `/proc/self/maps` scanner
  and 5 hardcoded system library paths for the library-namespace workaround.
  `dlclose_ex` is byte-for-byte identical in size to the legacy blob.
  `RenderWindow` was deliberately *not* ported (it is just a thin wrapper
  around `FrameBuffer` in the legacy blob; AOSP's flat architecture is
  behaviourally equivalent). ([`eb13449`])

- **`kr64` kernel-replacement daemon skeleton** (`app/rs/kr64/`) — reverse
  engineered from Virtual Master's `libkr64.so` and re-implemented from
  scratch in Rust under `MPL-2.0`. The daemon materialises the per-VM
  virtual `/dev/` tree (`qemu_pipe`, `touch`, `key0`, `gb`, `gb2`, `event`
  via `UnixListener::bind`), installs a seccomp filter (~60 syscalls
  allowed, ~15 dangerous ones blocked) with a `SIGSYS` emulation handler,
  emulates `/proc` (`version`, `cpuinfo`, `meminfo`, `self/`), sets up the
  mount namespace (`pivot_root` + tmpfs mounts), and `exec`s the guest
  `init`. Builds as both a `cdylib` (`libkr64.so`, directly executable via a
  `.interp` PIE trick) and an `rlib`+`bin` (`kr64`). Status: compiles with
  zero warnings, **26 unit tests passing**, depends on `libc` only, 3,084
  lines total. Follow-ups tracked in `download/KR64_SKELETON.md`.
  ([`570e95e`])

- **Dynamic data directory** — replaces 8 hardcoded `/data/data/io.twoyi`
  paths with a runtime-resolved data directory obtained from
  `Context.getDataDir()` via the new `setDataDir` JNI function. This makes
  the app work inside a work profile (`Android for Work` / managed profile),
  where the data directory is `/data/user/<uid>/io.twoyi` instead of
  `/data/data/io.twoyi`. Touched files: `core.rs` (added `DATA_DIR`
  `OnceLock<String>`, `set_data_dir`, `get_data_dir`, `get_rootfs_dir`,
  `get_log_path`, `get_touch_path`, `get_key_path`, `get_opengles_paths`),
  `input.rs` (replaced `const TOUCH_PATH`/`KEY_PATH` with `touch_path()` /
  `key_path()`), `socket_monitor.rs` (removed 3 hardcoded rootfs paths from
  `SOCKET_PATHS`), `lib.rs` (registered `setDataDir` JNI), `Renderer.java`,
  `Render2Activity.java`, and `TwoyiDocumentsProvider.java`. Backwards
  compatible: `get_data_dir()` falls back to `/data/data/io.twoyi` if
  `setDataDir` was never called. ([`9c4b907`])

- **Test keystore for signing release APKs** — a self-signed RSA 2048-bit
  keystore (validity 10000 days) wired into the release `signingConfig` so
  CI and codespace builds produce installable APKs out of the box. Without
  signing, Android refuses to install the APK
  (`INSTALL_PARSE_FAILED_NO_CERTIFICATES`). The keystore is intentionally
  committed because it is a test key (trivially replaceable) and it makes
  CI/codespace builds work without requiring GitHub Actions secrets;
  production distributors should substitute their own key. ([`ff1cc37`])

### Changed

- **`README.md` rewritten** (310 lines) — replaces the obsolete upstream
  README (which declared the project "discontinued" and required "NDK v22
  or lower") with a current description of the active fork: CI / license /
  ABI / Rust / Java badges, a 10-row improvement table with commit hashes,
  quick-start instructions (codespace + local), architecture overview
  (links to `ARCHITECTURE.md`), NDK-r27c build instructions for both ABIs,
  testing guide (codespace KVM / emulator / device), an 11-item 8–12-week
  MVP roadmap, and credits (`weishu`, `cyanmint`, `Disable-OP`).
  ([`9249147`])

- **`CONTRIBUTING.md` added** (404 lines) — fork/branch/PR process,
  dev-environment setup (codespace + local), per-language code style
  (Rust / Java / C/C++ / shell), testing guidelines with honest reporting,
  a 10-item pre-PR checklist, and a 10-area "needs help" list grouped into
  three priority tiers. ([`9249147`])

- **`ARCHITECTURE.md` expanded** (664 → 1324 lines) — adds 5 new sections
  (work profile support, open-source `libOpenglRender.so`, the kr64 daemon
  skeleton, the Virtual Master reverse-engineering comparison, and a GSI
  boot roadmap) and updates the crate table, file map, improvement
  opportunities (the `libOpenglRender` row flipped to ✅ built), and
  references (AOSP source, Treble/GSI docs, shadowhook). ([`ce29754`])

### Fixed

- **SIGABRT on `x86_64` when the default renderer was "old"** — the legacy
  `libOpenglRender.so` blob is arm64-only and not shipped for `x86_64`, so
  `renderer_bindings.rs` provided panic stubs that called `abort()`.
  `ProfileSettings.useNewRenderer()` defaulted to `false`, so on x86_64 the
  app would select the old renderer, hit `renderer_reset_window` → the
  panic stub, and SIGABRT in `Render2Activity$1.surfaceChanged`. Fix has
  two layers: (1) `ProfileSettings.useNewRenderer()` now defaults to `true`
  when the device's primary ABI is not `arm64-v8a`; (2) `core.rs` adds
  `effective_renderer_type()` which forces `RendererType::New` on
  non-`aarch64` targets as defence-in-depth, regardless of what the Java
  side requests. ([`7664c66`])

- **`gh codespace ssh` failure** — the custom devcontainer Dockerfile did
  not include an SSH server (the default Codespaces image has one built in,
  but our Ubuntu base did not). Added the
  `ghcr.io/devcontainers/features/sshd:1` feature, which installs and
  configures `openssh-server` automatically. ([`a6e6dbb`])

- **Devcontainer silently falling back to Alpine/musl** — the previous
  `devcontainer.json` used the features approach with
  `mcr.microsoft.com/devcontainers/base:ubuntu-22.04` as the base image, but
  the features build failed silently and GitHub fell back to its default
  codespace image (Alpine Linux with musl libc). This broke everything
  downstream: the Android emulator binary is compiled for glibc and refused
  to run on musl (`posix_fallocate64: symbol not found`); `setup.sh` used
  `apt-get` which does not exist on Alpine; `cargo-xdk` needed `rustup`
  which Alpine does not bundle. Fix: replaced `devcontainer.json` with a
  Dockerfile that explicitly installs all dependencies on Ubuntu 22.04
  (glibc). `setup.sh` now also creates `/dev/kvm` via `mknod` if the kvm
  module is loaded but the device node is missing, and pre-installs the
  Android emulator + `system-images;android-30;google_apis;x86_64` plus all
  the X11/Qt/PulseAudio shared libraries the emulator binary needs.
  ([`3628519`])

- **CI downloading a 9-byte "Not Found" rootfs** — the previous rootfs URL
  pointed at `rootfs.7z`, but the `cyanmint/twoyi` `original` release hosts
  `rootfs.tar.gz` (~275 MB). The 404 response was written to
  `app/src/main/assets/rootfs.7z` as a 9-byte `Not Found` placeholder that
  then got bundled into the APK. Corrected to the real URL.
  ([`f8368e9`])

- **`EXECUTOR.submit(this::start0)` ambiguous under JDK 17** — JDK 17's
  stricter method-resolution sees `start0()` and `start0(int)` as both
  matching `Executor.submit(Runnable)` and `Executor.submit(Consumer<Integer>)`,
  so it cannot pick one (`reference to submit is ambiguous` /
  `cannot infer type-variable(s) T`). The original code (JDK 8/11) did not
  have this problem because the overload-resolution rules were more
  relaxed. Fix: cast the method reference to `Runnable` explicitly so the
  compiler picks `submit(Runnable)` unambiguously. No behaviour change.
  ([`719a0db`])

- **Linker error: ARM64 `libOpenglRender.so` incompatible with `elf_x86_64`**
  — `build.rs` hardcoded the link-search path to `arm64-v8a/`, so when
  building `libtwoyi.so` for `x86_64-linux-android` the linker tried to link
  the ARM64-only legacy blob into an x86_64 binary (`ld.lld: error:
  ../src/main/jniLibs/arm64-v8a/libOpenglRender.so is incompatible with
  elf_x86_64`). Fix: `build.rs` now picks the `jniLibs` subdir based on
  `CARGO_CFG_TARGET_ARCH` (`aarch64` → `arm64-v8a`, `x86_64` → `x86_64`)
  and only adds the link-search path and `-lOpenglRender` directive if the
  legacy blob actually exists in that directory. `renderer_bindings.rs` is
  now `cfg`-gated: on `aarch64` it declares the `extern "C"` block with
  `#[link(name="OpenglRender")]` as before; on non-`aarch64` it provides
  stub functions that panic at runtime with a clear message. Result:
  `libtwoyi.so` builds cleanly for both ABIs. ([`2085938`])

- **`copy_to_cstr` type mismatch on `aarch64`** — declared as
  `fn copy_to_cstr<const COUNT: usize>(data: &str, arr: &mut [u8; COUNT])`
  but `device_info.name` is `[c_char; 80]`, and on
  `aarch64-linux-android` `c_char == i8`, so `&mut [i8; 80]` does not match
  `&mut [u8; 80]`. This was latent (the old jniLibs blob was built with a
  less strict toolchain) but the modern Rust stable toolchain used in CI
  rejected it. Fix: made `copy_to_cstr` generic over the element type `T`
  with a bounded `unsafe` pointer cast inside the function body. The cast
  is sound because `[u8; N]` and `[i8; N]` have identical memory layout
  (both are `N` one-byte elements) and we never write past `len <= COUNT`.
  ([`7858bce`])

- **Build scripts used bash arrays but were invoked with `sh`** —
  `app/build.gradle`'s `cargoBuild` task runs `sh build_rs.sh`, but on
  Ubuntu (which is what GitHub Actions runners and the devcontainer use)
  `sh` is `dash`, and `dash` does not support bash arrays
  (`build_rs.sh: 30: Syntax error: "(" unexpected`). Rewrote all three
  build scripts (`build_rs.sh`, `loader/build.sh`,
  `openglrenderer/build.sh`) to use space-separated strings instead of
  arrays. Verified with `sh -n` (syntax check) and three argument-parsing
  test cases (`--release`, `--release all`, `--release arm64-v8a x86_64`).
  Also fixed a latent logic bug in the original array version: when
  `--release` was passed alongside explicit ABIs, the `--release` case
  would clobber the ABI list with the default if the list was still empty
  at that point (which it always was, since `--release` typically comes
  first). The fixed version defers the default-ABIs fallback to after the
  argument loop. ([`d2cfb8d`])

### Removed

- Nothing user-visible has been removed in this release. (Internally, the
  obsolete upstream `README.md` content was replaced wholesale in
  [`9249147`]; the old `cyanmint`-era `CHANGES.md` is being superseded by
  this file.)

### Security

- A **test-only release keystore** is now committed to the repository
  ([`ff1cc37`]). This is intentional for fork usability — it lets CI and
  codespace builds produce installable APKs without requiring GitHub
  Actions secrets. The key is **not** a production key; downstream
  distributors **MUST** replace it before publishing a release. Instructions
  for swapping in a real key are documented inline in `app/build.gradle` →
  `signingConfigs.release`:

  ```sh
  keytool -genkey -v -keystore app/my-release.keystore \
    -alias my-key -keyalg RSA -keysize 2048 -validity 10000
  ```

  then update `storeFile` / `storePassword` / `keyAlias` / `keyPassword`.

- The `kr64` seccomp filter blocks ~15 dangerous syscalls (`ptrace`,
  `perf_event_open`, `kexec_load`, `swap*`, etc.) and traps them to a
  `SIGSYS` handler for emulation ([`570e95e`]). This is a
  skeleton-level mitigation that reduces the guest's syscall attack
  surface; full per-syscall emulation in the `SIGSYS` handler is a tracked
  follow-up task (`download/KR64_SKELETON.md`).

[`47f8335`]: https://github.com/Disable-OP/twoyi/commit/47f833582c4188bcfe3c6504d444046b4daf985d
[`eb13449`]: https://github.com/Disable-OP/twoyi/commit/eb13449db3eecfe4d96a7f5b6e0095a3ac232c26
[`570e95e`]: https://github.com/Disable-OP/twoyi/commit/570e95e8e08a1a64063045975157d1a679288959
[`9c4b907`]: https://github.com/Disable-OP/twoyi/commit/9c4b907ac535bc7b9badc025748d6dbee1f34ccf
[`ff1cc37`]: https://github.com/Disable-OP/twoyi/commit/ff1cc37aaed0f442a2ffe6be215df06d425bfc68
[`9249147`]: https://github.com/Disable-OP/twoyi/commit/9249147f00ae747f49e73cad1c34513f1768e1cb
[`ce29754`]: https://github.com/Disable-OP/twoyi/commit/ce29754468ba2975d022e8943eb65a232a8f0ed5
[`7664c66`]: https://github.com/Disable-OP/twoyi/commit/7664c660106dfeee9a8576ca3962cb7bdce230ac
[`a6e6dbb`]: https://github.com/Disable-OP/twoyi/commit/a6e6dbb3b920298bd85609e0ebf61eb5b5829186
[`3628519`]: https://github.com/Disable-OP/twoyi/commit/362851942828b10cccddc48833e83ca86c7f6cc5
[`f8368e9`]: https://github.com/Disable-OP/twoyi/commit/f8368e90640cae692e17323bb33f0da06fc8fd7e
[`719a0db`]: https://github.com/Disable-OP/twoyi/commit/719a0db55b7634655f7904bc915fa87d7800db4f
[`2085938`]: https://github.com/Disable-OP/twoyi/commit/2085938042bfc540b33a479045d0e2450a4f04e9
[`7858bce`]: https://github.com/Disable-OP/twoyi/commit/7858bce56697665727972b2a3f2c6d6e1e9b8879
[`d2cfb8d`]: https://github.com/Disable-OP/twoyi/commit/d2cfb8dbb7eef1b963293b8d3e061cfe954779c2
