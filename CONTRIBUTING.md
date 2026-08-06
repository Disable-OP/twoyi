# Contributing to Twoyi

Thanks for your interest in improving Twoyi! This document covers everything
you need to get a build going, write code that fits in, and land it upstream.

> **Project status.** This is an **active fork** of the archived
> `twoyi/twoyi` project, maintained on the `improvements/initial-cleanup`
> branch of [`Disable-OP/twoyi`](https://github.com/Disable-OP/twoyi). Read
> [`README.md`](README.md) and [`ARCHITECTURE.md`](ARCHITECTURE.md) first if
> you haven't already.

---

## 1. How to contribute

The standard GitHub flow:

1. **Fork** `Disable-OP/twoyi` to your own account.
2. **Clone** your fork and add the upstream as a second remote:
   ```bash
   git clone https://github.com/<you>/twoyi.git
   cd twoyi
   git remote add upstream https://github.com/Disable-OP/twoyi.git
   git fetch upstream
   git checkout -b my-feature improvements/initial-cleanup
   ```
3. **Branch** from `improvements/initial-cleanup` (the active development
   branch). Use a descriptive branch name:
   - `feat/kr64-binder-proxy`
   - `fix/renderer-reset-window-race`
   - `docs/readme-arm64-build`
4. **Commit** with [Conventional Commits](https://www.conventionalcommits.org/)
   prefixes — `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `ci:`,
   `build:`, `chore:`. Scope is encouraged, e.g. `feat(kr64): …`,
   `fix(renderer): …`, `ci(build): …`.
5. **Push** to your fork and open a pull request against
   `Disable-OP/twoyi:improvements/initial-cleanup`.
6. **Address review feedback** by pushing additional commits to the same
   branch (do **not** squash unless asked — reviewers want to see the
   iteration history).

### Issue first

For anything bigger than a typo or a one-line fix, **open an issue first** so
we can discuss scope and approach before you sink time into code. The
[roadmap](README.md#roadmap) lists the open milestones — if you want to pick
one up, comment on the relevant issue (or open one) so we don't double up.

---

## 2. Development environment

### Option A: GitHub Codespace (recommended, fastest setup)

The repo ships a full devcontainer (`.devcontainer/`). Create a codespace:

1. Click the green **Code** button on the repo → **Codespaces** tab →
   **Create codespace on improvements/initial-cleanup**.
2. Pick the **`standardLinux32gb`** machine (4 cores / 16 GB / 32 GB disk).
   You need this size for the Android SDK + NDK + Rust toolchain.
3. The `postCreateCommand` (`.devcontainer/scripts/setup.sh`) installs:
   - OpenJDK 17, Rust (stable) with both Android targets, `cargo-xdk`
   - Android SDK (platform-tools, API 31, build-tools 30.0.3, emulator,
     `system-images;android-30;google_apis;x86_64`)
   - Android NDK r27c
   - QEMU/KVM, Docker, and `mknod /dev/kvm` if the codespace runs
     `--privileged` (which the devcontainer requests).

Verify the codespace is healthy:

```bash
.devcontainer/scripts/check-kvm.sh    # KVM available?
./gradlew cargoBuild                   # Rust crates build?
```

Then jump to **[Building](README.md#building)** in the README.

### Option B: Local setup

Install manually:

| Tool | Version | Notes |
|---|---|---|
| JDK | 17 | Temurin or OpenJDK. |
| Android SDK | API 31, build-tools 30.0.3 | `sdkmanager "platform-tools" "platforms;android-31" "build-tools;30.0.3"`. |
| Android NDK | r27c | Matches CI exactly. |
| Rust | stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh`. |
| Rust targets | `aarch64-linux-android`, `x86_64-linux-android` | `rustup target add …`. |
| `cargo-xdk` | latest | `cargo install cargo-xdk`. |
| (optional) Android Studio | recent | For the visual layout editor. |

Set the standard env vars:

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/27.2.12479018"  # adjust to installed version
export PATH="$PATH:$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/latest/bin"
```

Verify:

```bash
./gradlew assembleRelease -Pabis=arm64-v8a    # ~5 min cold
```

### Dev tools that help

- **`rust-analyzer`** VS Code extension — the devcontainer pre-installs it
  with `linkedProjects` pointing at each Cargo.toml.
- **`vadimcn.vscode-lldb`** — for debugging the Rust side.
- **`tamasfe.even-better-toml`** — for `Cargo.toml` editing.
- **`scrcpy`** — for interacting with a real device over ADB without keeping
  the device screen on.

---

## 3. Code style

### Rust (`app/rs/`, `app/rs/loader/`, `app/rs/openglrenderer/`, `app/rs/kr64/`)

- **Format** with the default `rustfmt` configuration. Run `cargo fmt` before
  committing. Do not introduce `#![rustfmt::skip]` without justification.
- **Lint** with `cargo clippy --all-targets -- -D warnings`. CI does not yet
  enforce this, but reviewers will ask you to fix anything `clippy` flags.
- **Edition**: 2021. Don't bump it unilaterally across crates.
- **Dependencies**: prefer `std` + `libc` only where possible. The `kr64`
  crate is intentionally `libc`-only (no `log`, no `once_cell`, no `nix`) so
  it can be statically analyzed and audited. The main `twoyi` crate is allowed
  to use `log`, `jni`, `ndk`, `once_cell`, etc. — match the crate's existing
  convention.
- **Logging**: use the `log` crate macros (`info!`, `warn!`, `error!`,
  `debug!`) in the main `twoyi` crate. In `kr64` use the crate-local
  `info!`/`warning!`/`error!` macros (named `warning!` to avoid clashing with
  Rust's `#[warn]` lint attribute).
- **Unsafe**: keep `unsafe` blocks small and add a `// SAFETY:` comment
  explaining why each one is sound. Cross-language `extern "C"` boundaries
  are inherently unsafe — document the contract.
- **Tests**: every new module should ship unit tests under `#[cfg(test)]`.
  Tests must pass on the **Linux host** (`cargo test`), not just on Android —
  the `kr64` skeleton's `build.rs` gates Android-specific linker flags on
  `target_os = "android"` so host tests work. Follow that pattern for new
  crates.
- **No `println!` in libraries** — use the logging macros.

### Java (`app/src/main/java/io/twoyi/`)

- **Style**: AOSP Java style, 4-space indent, 100-column limit.
- **Imports**: explicit, no wildcards. Sort in IDE-standard order
  (static last).
- **Logging**: use `android.util.Log` with the existing tag conventions
  (`LogEvents`, `Log.d(TAG, …)`). Don't introduce a new logging framework.
- **JNI**: declare `native` methods on `Renderer.java` (or a sibling class)
  with the `extern "C"` counterpart in Rust. Use `Renderer.setDataDir()`,
  `Renderer.init()`, etc. as templates — keep the Java side thin.
- **Reflection**: the codebase uses standard `java.lang.reflect` to call
  a handful of hidden platform APIs (e.g. `android.os.FileUtils.setPermissions`,
  `ApplicationInfo.primaryCpuAbi`). This works because `targetSdkVersion`
  is pinned to 28, so Android's hidden-API blocklist is not enforced.
  If you raise `targetSdkVersion` ≥ 28 you will need to either drop the
  reflective calls or reintroduce a hidden-API bypass (e.g.
  [FreeReflection](https://github.com/tiann/FreeReflection)).
- **No new dependencies** without an issue discussion — the dependency list
  in `app/build.gradle` is intentionally short.

### C / C++ (`app/rs/src/interp.c`, AOSP-derived renderer sources)

- Only C is used for the PIE `.interp` trick (`interp.c`). Match existing
  style (K&R braces, 4-space indent).
- The AOSP-derived `libOpenglRender.so` source is **not** checked into this
  repo — it's built from `platform/sdk` commit `7a712acc` with a patch series
  documented in `download/AOSP_BUILD_RESULTS.md` and `download/port_files/`.
  If you need to modify the renderer's C++ source, open an issue first;
  rebuild instructions are in those reports.

### Build scripts (`*.sh`, `build.rs`, `build.gradle`)

- **Shell**: POSIX `sh`-compatible (the repo's `build_rs.sh` deliberately
  avoids bash arrays so it works under dash on Ubuntu CI). Use `set -e`.
- **`build.rs`**: gate Android-only behaviour on
  `CARGO_CFG_TARGET_OS == "android"` so the same crate compiles on the Linux
  host for tests (see `app/rs/kr64/build.rs`).
- **Gradle**: keep `build.gradle` in Groovy (don't switch to Kotlin DSL
  unilaterally — match what's already there).

---

## 4. Testing

### What to test

| Layer | What to test | How |
|---|---|---|
| Rust crates | Every public function with non-trivial logic. Argument parsing, syscall classification, device creation, `/proc` synthesis. | `cargo test` (runs on Linux host). |
| `kr64` daemon | End-to-end smoke test with `--no-seccomp --no-namespaces` on a tmpdir. | `cargo run -- --rootfs /tmp/rfs --data-dir /tmp/data --no-seccomp --no-namespaces` (expect status 1 — `mount` fails with EPERM on host, which is correct). |
| Java side | At minimum, don't break the existing boot flow. Add JUnit tests under `app/src/test/` for pure-Java logic; instrumented tests under `app/src/androidTest/` for anything touching Android framework classes. | `./gradlew test`, `./gradlew connectedAndroidTest`. |
| Renderer | Hard to unit-test. Verify by booting on a real arm64 device and confirming GL output. | Manual — see [Testing](README.md#testing) in the README. |
| End-to-end | APK builds, installs, and launches without crashing on both ABIs. | `.devcontainer/scripts/test-twoyi.sh` in the codespace, or manual `adb install` on a real device. |

### Running tests

```bash
# Rust unit tests (host):
cd app/rs/kr64 && cargo test
cd app/rs      && cargo test    # main crate, if applicable

# Java unit tests:
./gradlew test

# Java instrumented tests (needs a connected device or emulator):
./gradlew connectedAndroidTest

# Smoke-build everything:
./gradlew assembleRelease -Pabis=all
```

### Honest test reporting

When you open a PR, **state explicitly what you tested and what you didn't**.
For example:

> Tested: `cargo test` in `app/rs/kr64` (26 passing), `./gradlew
> assembleRelease -Pabis=all` succeeds, APK installs on redroid x86_64.
> Not tested: real arm64 device boot (don't have hardware on hand).

This project has a history of overclaims (see `download/TWOYI_HONEST_STATUS.md`).
Be conservative — say "inferred from symbol matching" rather than "verified
working" unless you actually ran it.

---

## 5. Pull request process

### Before you open the PR

- [ ] Branch is based on `improvements/initial-cleanup` (not `main`).
- [ ] `cargo fmt` and `cargo clippy --all-targets -- -D warnings` are clean
      for any Rust crate you touched.
- [ ] `cargo test` passes in every crate you touched.
- [ ] `./gradlew assembleRelease -Pabis=all` succeeds.
- [ ] Commit messages follow Conventional Commits.
- [ ] No large binary blobs committed unless explicitly justified (the repo
      already has too many — see `download/`). Use Git LFS or link to a release
      asset instead.
- [ ] No secrets, keystore passwords, or tokens. The committed
      `twoyi-release.keystore` is intentionally a public test key — don't
      commit a real one.

### PR description template

```markdown
## What
One-paragraph summary of the change.

## Why
Link to issue / roadmap item / motivation.

## How
Brief implementation notes — what files, what approach, any non-obvious
design decisions.

## Testing
- [ ] cargo test (which crate, how many tests)
- [ ] ./gradlew assembleRelease -Pabis=…
- [ ] On-device / emulator / redroid test (describe what you saw)

## Not tested
Be honest about what you didn't verify.

## Checklist
- [ ] Conventional Commits
- [ ] cargo fmt + clippy clean
- [ ] No new dependencies without prior discussion
- [ ] Documentation updated (README / ARCHITECTURE / inline comments)
```

### Review criteria

Reviewers will look for:

1. **Correctness** — does it do what it claims? Does it break anything else?
2. **Honesty** — does the "Testing" section match what was actually run?
3. **Scope** — is the PR the smallest reasonable change, or is it bundling
   unrelated work?
4. **Style** — matches the surrounding code; no surprise dependencies.
5. **Documentation** — non-trivial changes update `README.md`,
   `ARCHITECTURE.md`, or inline doc comments as appropriate.

### CI requirements

The `.github/workflows/build.yml` workflow runs on every push to
`improvements/**` and every PR against `main` / `develop`. It builds the APK
with `assembleRelease -Pabis=all`. **Your PR must produce a green build on
CI** before it can be merged.

If CI fails for a reason unrelated to your change (e.g. an upstream SDK
outage), call it out in the PR description.

### Approval and merge

- One approval from a maintainer is required for non-architectural changes.
- Architectural changes (new Rust crate, new AIDL surface, changes to the
  JNI boundary, anything touching `app/rs/kr64/src/seccomp.rs` or
  `app/rs/src/interp.c`) need **two** approvals.
- Maintainers squash-merge small PRs and merge-commit larger ones with
  meaningful intermediate history. Tell us your preference in the PR if it
  matters to you.

---

## 6. Areas needing help

The [Roadmap](README.md#roadmap) lists every open milestone; the items below
are the ones with the highest leverage and the lowest barrier to entry for a
new contributor. Each one is a good first PR size if scoped tightly.

### Good first issues

1. **Open-source `libadb.so`** (Roadmap #10). Replace the 4.46 MB
   closed-source `adb` blob with a build from `packages/modules/adb`
   (Apache-2.0). Build steps are documented in
   `download/TWOYI_DISASSEMBLY_ANALYSIS.md` Phase 3. Estimated 1 week for
   someone familiar with the AOSP build system.

2. **Extend `GraphicBuffer::Main` to register buffers** (Roadmap #5
   sub-task). The ported `GraphicBuffer::Main` accept loop in the AOSP
   renderer receives `AHardwareBuffer` file descriptors but doesn't yet
   register them with `FrameBuffer` for compositing. Reverse-engineer the
   legacy `GraphicBufferHandler::main` (136 B + 5 sibling methods, 296 B
   total) and re-implement the buffer-id registration protocol.
   `download/FUNCTION_LEVEL_COMPARISON.md` §4.7–4.9 has the starting
   analysis.

3. **Full device inventory in `kr64`** (Roadmap #1 follow-up). The skeleton
   creates 6 MVP devices (`qemu_pipe`, `touch`, `key0`, `event`, `gb`,
   `gb2`). Virtual Master creates 20+ (see `download/VM_KR64_ANALYSIS.md`
   §6 — `/dev/vmproc`, `/dev/__kmsg__`, `/dev/__properties__`,
   `/dev/ashmem`, `/dev/socket/*`, `/dev/block/vdc`, `/dev/fuse`, netlink
   sockets). Add them one at a time, each with a unit test.

4. **`mknodat`-based socket creation** in `kr64`. The skeleton uses
   `UnixListener::bind` (creates the socket file as a side effect). VM uses
   `mknodat(S_IFSOCK)` + `bind()` which requires `CAP_MKNOD`. Add a
   capability check and switch to the `mknodat` path when the capability is
   available. See `download/KR64_SKELETON.md` §5 item 5.

5. **Per-syscall emulation in `kr64`'s SIGSYS handler**. Currently
   `seccomp::emulate_syscall()` returns 0 for all trapped syscalls.
   Production needs to dispatch `mount` → `mount_mgr::bind_mount()`,
   `umount2` → unbind, `reboot` → `-EPERM`, etc. See
   `app/rs/kr64/src/seccomp.rs`.

### Medium-effort projects

6. **x86_64 rootfs from AOSP** (Roadmap #2). Unblocks all x86_64 end-to-end
   testing. Use the recovered `default.xml` manifest (commit `25ef89c`) or
   `repo init -u https://android.googlesource.com/platform/manifest -b
   android-8.1.0_r81`. Build user-space only (init, zygote,
   SurfaceFlinger, servicemanager). Package as `rootfs.tar.gz` matching
   the existing `RomManager` extraction format.

7. **GSI extractor** (Roadmap #3). Implement sparse-ext4 → raw ext4
   conversion, ext4 extraction, `boot.img` ramdisk extraction, and
   minimal `vendor.img` synthesis. Files to create:
   `app/src/main/java/io/twoyi/utils/GsiExtractor.java` and a new Rust
   crate at `app/rs/gsi_extractor/`. See `download/GSI_BOOT_PLAN.md` §3.7
   for the full spec.

8. **GSI init patcher** (Roadmap #4). Patch `/system/build.prop`,
   `/system/etc/init/hw/init.rc`, `/vendor/etc/init/*.rc`,
   `/system/etc/prop.default` so the guest talks to twoyi's virtual
   devices. File to create: `app/src/main/java/io/twoyi/utils/GsiInitPatcher.java`.
   See `download/GSI_BOOT_PLAN.md` §3.8.

### Hard problems (need design discussion first)

9. **Binder virtualization** (Roadmap #8). Per-VM `/vm%d/dev/binder` plus a
   Java-side `IActivityManager` proxy. This is the hardest single piece of
   the GSI boot plan. **Open an issue to discuss approach before
   starting** — the MVP workaround (patching `system_server` to skip
   `publishService`) may be a better first step.

10. **Graphics HAL** (Roadmap #5). `/dev/gb` + `/dev/gb2` char devices with
    gralloc allocator/mapper/composer ioctls, routed through the existing
    `libOpenglRender_aosp.so` `ColorBuffer` infrastructure.

### Non-code contributions

- **Documentation** — `ARCHITECTURE.md`, `README.md`, and the `download/`
  analysis reports always need tightening. Typos, dead links, and unclear
  explanations are fair game.
- **Bug reproduction** — install the latest APK from CI on a real device,
  try to boot, and file detailed issues with `adb logcat` output and
  tombstones.
- **Translation** — `README_CN.md` is out of date with the new README. A
  fresh Chinese translation would be welcome.

---

## Questions?

- Open a [GitHub Discussion](https://github.com/Disable-OP/twoyi/discussions)
  for general questions.
- Open a [GitHub Issue](https://github.com/Disable-OP/twoyi/issues) for
  bugs and feature requests.
- For security-sensitive issues, see `SECURITY.md` (or email the maintainer
  directly if no SECURITY.md exists yet).

Thanks for helping make Twoyi better!
