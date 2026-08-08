# Quick Start — Twoyi for New Contributors

> **Goal:** get you from `git clone` to a working build and a picked task in
> **5 minutes**. Twoyi is a rootless Android-on-Android container — it boots
> a second Android userland inside one normal app process, with no root and
> no kernel module. Active development happens on the `main` branch of
> [`Disable-OP/twoyi`](https://github.com/Disable-OP/twoyi) (the historical
> `improvements/initial-cleanup` branch has been merged in and deleted).
>
> If you want the *why* before the *how*, read
> [`download/TECHNICAL_BRIEFING.md`](TECHNICAL_BRIEFING.md) (~15 min read).

---

## 1. Clone + Build (3 commands)

```bash
git clone https://github.com/Disable-OP/twoyi.git && cd twoyi
git checkout main   # main is the only branch (round 68)
./gradlew assembleRelease -Pabis=arm64-v8a
```

The signed APK lands at `app/build/outputs/apk/release/`. For a fat APK that
also works in x86_64 emulators and redroid, swap the last line for
`./gradlew assembleRelease -Pabis=all`.

> **Fastest path:** skip the local install — open the repo in a GitHub
> Codespace (pick `standardLinux32gb`). The devcontainer pre-installs JDK 17,
> Rust + both Android targets, NDK r27c, the SDK, and QEMU/KVM. Then just
> run the `./gradlew` line above.

**Prerequisites (local only):** JDK 17 · Android NDK r27c · Rust stable with
`aarch64-linux-android` and `x86_64-linux-android` targets · `cargo-xdk`.

---

## 2. Run Tests (2 commands)

```bash
cd app/rs/kr64 && cargo test        # Rust unit tests — 165 tests on Linux host
./gradlew test                      # Java unit tests
```

Instrumented tests on a device/emulator: `./gradlew connectedAndroidTest`.
End-to-end smoke test in the codespace: `.devcontainer/scripts/test-twoyi.sh`.

---

## 3. Start Coding — where to look first

Twoyi is three layers. Read these files **in this order** to get oriented:

| Read this | To understand |
|---|---|
| [`ARCHITECTURE.md`](../ARCHITECTURE.md) | The 3-layer design, the PIE `.interp` hack, the boot flow. Read this first. |
| `app/rs/src/core.rs` | The guest spawn: forks `./init` from the rootfs inside a data-dir chroot. |
| `app/rs/kr64/src/lib.rs` (lines 39–80, 556–565) | The kernel-replacement daemon: 6 virtual devices, seccomp BPF, `/proc` emulator, mount namespace. |
| `app/rs/src/renderer_bindings.rs` | The 6-symbol FFI contract between the Rust side and `libOpenglRender.so`. |
| `app/rs/src/renderer_new/pipe.rs` | How the host opens `/dev/qemu_pipe` and decodes the GL stream. |
| `app/src/main/java/io/twoyi/Render2Activity.java` | The Java UI host (SurfaceView + JNI entry points). |
| `app/src/main/java/io/twoyi/utils/RomManager.java` | How the rootfs asset is extracted at first boot. |

**Module map** (what each Rust crate is for):

- `app/rs/` — the main `libtwoyi.so` cdylib (also a PIE binary). Boot loader
  + input sockets + renderer pipe client.
- `app/rs/kr64/` — the kernel-replacement daemon. Modules:
  `devices.rs` (virtual `/dev` tree), `binder.rs` (binder proxy),
  `seccomp.rs` (BPF filter + SIGSYS handler), `proc_emu.rs` (`/proc` synthesiser),
  `mount_mgr.rs` (bind-mount + tmpfs), `audio.rs` / `battery.rs` / `sensors.rs`
  (HAL shims). ~9,500 LOC, 165 tests.
- `app/rs/loader/` — open-source replacement for the legacy `libloader.so` blob.
- `app/rs/openglrenderer/` — Rust scaffolding around the AOSP-built
  `libOpenglRender.so` (which is built from AOSP emugl source, not in this repo).

**Java side** (`app/src/main/java/io/twoyi/`): `Render2Activity` (UI + JNI),
`RomManager` (rootfs extract), `TwoyiStatusManager` (boot state),
`TwoyiSocketServer` (IPC to guest), `Renderer.java` (JNI declarations).

---

## 4. Pick a Task

The full plan, with file paths, acceptance criteria, and effort estimates,
is in **[`DEVELOPMENT_ROADMAP.md`](DEVELOPMENT_ROADMAP.md)**. It breaks the
work into 5 phases (Stabilization → Open-Source Completion → GSI Boot MVP →
Feature Parity → Advanced). The "good first issues" list is in §10.2.

**Three good first issues** (each is effort **S** — ≤1 week, no architectural
discussion needed, complete design doc already exists):

1. **Drop-in test the AOSP renderer on a real arm64 device**
   (Phase 1 task 1.1, ~1 day). Copy
   `download/aosp-built/libOpenglRender_aosp_arm64.so` to
   `app/src/main/jniLibs/arm64-v8a/libOpenglRender.so`, rebuild, install on
   a phone, verify guest GL renders. Highest-leverage verification in the
   project. Requires a physical arm64 device.

2. **Wire `kr64` into the boot flow**
   (Phase 1 task 1.4, ~2 days). Add `kr64` as a workspace member of
   `app/rs/Cargo.toml`, extend `app/rs/build_rs.sh`, add the spawn call in
   `app/rs/src/core.rs`. Goal: see `[KR64 INFO] created device /dev/qemu_pipe`
   in logcat on redroid x86_64. Great for learning the kr64 codebase.

3. **Extend `kr64` device tree to 20+ devices**
   (Phase 3 task 3.1, ~3 days total, ~30 min per device). The skeleton
   creates 6 devices; VM creates 20+. Each new device is ~20 lines following
   the existing `bind_unix_socket` helper in `app/rs/kr64/src/devices.rs`.
   See `download/VM_KR64_ANALYSIS.md` §6 for the full inventory.

For anything bigger than a typo, **open an issue first** so we can confirm
scope. Branch from `main`, use Conventional Commits
(`feat:`, `fix:`, `docs:` …), and open a PR against the same branch.

---

## 5. Get Help

**Documentation (read in this order):**

1. [`README.md`](../README.md) — project overview, build/test instructions, roadmap summary.
2. [`ARCHITECTURE.md`](../ARCHITECTURE.md) — 3-layer architecture, PIE hack, boot flow.
3. [`CONTRIBUTING.md`](../CONTRIBUTING.md) — dev environment, code style, PR process.
4. [`download/DEVELOPMENT_ROADMAP.md`](DEVELOPMENT_ROADMAP.md) — what to work on next, with file paths.
5. [`download/TECHNICAL_BRIEFING.md`](TECHNICAL_BRIEFING.md) — 15-minute architectural briefing.
6. [`download/PROJECT_SUMMARY.md`](PROJECT_SUMMARY.md) — definitive state-of-the-project write-up (~970 lines).
7. [`download/GSI_BOOT_PLAN.md`](GSI_BOOT_PLAN.md) — file-level plan for the headline GSI boot goal.
8. [`download/TWOYI_HONEST_STATUS.md`](TWOYI_HONEST_STATUS.md) — verified vs. theoretical status
   (this project has a documented history of overclaims — read this before
   trusting any "it works" claim).

**Ask questions:**

- 💬 **[GitHub Discussions](https://github.com/Disable-OP/twoyi/discussions)** — general questions, design ideas.
- 🐛 **[GitHub Issues](https://github.com/Disable-OP/twoyi/issues)** — bugs, feature requests, claiming a roadmap item.
- 🔒 **Security-sensitive** — see `SECURITY.md` or email the maintainer directly.
- 📝 **PR review** — one approval for normal changes, two for architectural
  changes (new Rust crate, AIDL surface, JNI boundary, `kr64/src/seccomp.rs`,
  `app/rs/src/interp.c`).

**Conventions:** be **honest** in PRs about what you tested — say "inferred
from symbol matching" rather than "verified working" unless you actually ran
it. Rust crates: `cargo fmt` + `cargo clippy --all-targets -- -D warnings`
before committing; the `kr64` crate is `libc`-only. Java: AOSP style, 4-space
indent, 100-col limit. Shell scripts must be POSIX `sh`-compatible (CI runs `dash`).

Welcome aboard — and thanks for helping make Twoyi better!
