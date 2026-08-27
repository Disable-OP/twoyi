# Contributing to Twoyi

Thanks for your interest in improving Twoyi! This fork turns the original
[twoyi](https://github.com/cyanmint/twoyi) Android-in-Android container into a
maintained project that boots **TWRP recovery** (and full Android ROMs) on both
x86_64 and arm64-v8a devices.

## Getting started

1. **Install the toolchain**
   - Android Studio (or the command-line SDK tools) + NDK r27c or newer
   - Rust (`rustup`) with the `aarch64-linux-android` and
     `x86_64-linux-android` targets
   - Python 3 (for the E2E navigation scripts)

2. **Get the rootfs asset** (only needed to actually RUN the app)
   - Download `rootfs.tar.gz` from the
     [cyanmint/twoyi `original` release](https://github.com/cyanmint/twoyi/releases/download/original/rootfs.tar.gz)
   - Place it in `app/src/main/assets/` (CI can fetch it automatically via the
     `include_rootfs` input of the build workflow)

3. **Build**
   ```bash
   ./gradlew assembleDebug          # debug APK, both ABIs
   ./gradlew assembleRelease -Pabis=all
   ```
   See [`.github/workflows/README.md`](.github/workflows/README.md) for the CI
   equivalent and [scripts/build_libtwoyi.sh](scripts/build_libtwoyi.sh) for
   building just the Rust `libtwoyi.so`.

## Before you open a PR

- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must pass for
  **both** Rust crates (`app/rs`, `app/rs/kr64`) — this is what
  `kr64-tests.yml` enforces in CI.
- `cargo test` in `app/rs/kr64` must pass.
- If you touch anything in the boot path (tracer, loader hook, renderer,
  input bridge), run the relevant E2E workflow and attach the conclusion to
  your PR description:
  - TWRP: `ui-e2e-test.yml` (x86_64 emulator) and/or
    `ui-e2e-test-arm64.yml` (redroid on `ubuntu-24.04-arm`)
  - Full Android: `ui-e2e-aosp.yml` / `ui-e2e-aosp-arm64.yml`
- Keep performance in mind: the tracer runs on real devices. Do NOT add
  unguarded per-syscall logging — heavy tracing belongs behind the
  `TWOYI_TRACE_SYSCALLS=1` opt-in (see `app/rs/kr64/src/ptrace_emu.rs`).

## Code layout

| Path | What lives there |
|---|---|
| `app/rs/kr64/` | The Rust ptrace tracer that emulates the guest kernel |
| `app/rs/src/` | `libtwoyi.so` — JNI bridge, renderer, input, container launch |
| `app/cpp/twoyi_loader/` | Guest-side loader + the TWRP framebuffer/input hook |
| `app/cpp/emugl/` | AOSP emugl renderer (libOpenglRender.so) for full-Android mode |
| `app/src/main/java/io/twoyi/` | The Android app (UI, profile manager, import flow) |
| `scripts/` | E2E navigation + build helpers |
| `docs/reference/` | Architecture docs and reverse-engineering analyses |

Code style and naming conventions for the Rust code are documented in
[docs/reference/CODE_STYLE_GUIDE.md](docs/reference/CODE_STYLE_GUIDE.md).

## Notes on claims and evidence

This project previously accumulated overclaims; every "works" statement in the
README is backed by a CI run and (where visual) a screenshot under
`screenshots/`. Please keep it that way: if you add a feature, add the proof.
Historical analyses live in [docs/reference/](docs/reference/) — when you cite
one, link the file directly instead of restating its conclusions.
