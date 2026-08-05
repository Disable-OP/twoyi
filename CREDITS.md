# Credits & Acknowledgements

twoyi is a fork of a fork, and every layer of that lineage deserves credit.

## Lineage

### Original author

**[weishu](https://github.com/tiann)** — creator of twoyi, also known for
[Taichi](https://github.com/taichi-framework/Taichi),
[EdXposed](https://github.com/ElderDrivers/EdXposed), and
[KernelSU](https://github.com/tiann/KernelSU). twoyi's core architecture —
userspace Android container booting a guest ROM via a kernel-replacement
daemon (`libkr64.so`) plus the AOSP `emugl` QEMU-pipe renderer — is entirely
weishu's design. This fork builds directly on that foundation.

### Fork maintainer

**[cyanmint](https://github.com/cyanmint)** — maintained the only active
fork after the upstream was archived, kept the build working against modern
toolchains, and published the `rootfs.tar.gz` release this project still
ships as its default guest image. Without that stewardship twoyi would have
died with the original repository.

### Current fork

**[Disable-OP](https://github.com/Disable-OP)** — the fork where active
improvements are being made (branch `improvements/initial-cleanup`). See the
[contributors graph](https://github.com/Disable-OP/twoyi/graphs/contributors)
for the up-to-date human contributor list.

## Overnight contributors

A sequence of general-purpose sub-agents (automated coding assistants) worked
through the night of 2026-08-05 to advance the fork. They are not named
individuals, but their work is recorded in `worklog.md` under task IDs
`VM-ROM-1`, `VM-JAVA-1`, `VM-DISASM-1`, `VM-KR64-1`, `AOSP-BUILD-1`,
`PORT-1`, `KR64-IMPL-1`, `README-1`, `CHANGELOG-1`, `MIGRATION-1`, and the
`KEEP-WORKING-*` series. Their contributions span:

- Reverse engineering of the Virtual Master APK (Java, native, and ROM).
- Building `libOpenglRender.so` from AOSP `emugl` source for arm64/x86_64.
- Skeleton implementations of the kernel-replacement daemon in Rust.
- Documentation: README, ARCHITECTURE, CHANGELOG, migration guide, FAQ,
  security policy, ADR set, glossary, contributor ladder, project health,
  and this credits file.

All sub-agent output is unreviewed draft work awaiting human verification.

## Upstream projects

- **AOSP `emugl`** (Android Open Source Project) — the OpenGL ES renderer and
  `FrameBuffer` / `ColorBuffer` / `render_api` sources that twoyi's
  `libOpenglRender.so` is built from. Licensed under the Apache License 2.0.
- **Virtual Master** (`com.clone.android.dual.space`) — a commercial
  Android-in-Android app whose APK was used **as a reverse-engineering
  reference only**. No code, assets, or ROM images from Virtual Master are
  included in this repository; the analysis lives in `worklog.md` and the
  `vm-*` / `docs_vm_*` notes.
- **Anbox** — an earlier container-based Android-on-Linux project whose
  public design discussions informed the container approach used here.

## Tools used

- **GNU binutils** (`objdump`, `readelf`, `nm`, `ar`) — disassembly and symbol analysis of the native blobs.
- **[jadx](https://github.com/skylot/jadx)** — Java/Dex decompilation and StringFog deobfuscation of the VM APK.
- **[Playwright](https://playwright.dev/)** — automated screenshot capture and UI verification on emulators.
- **[GitHub Actions](https://github.com/features/actions)** — CI builds of the Android APK and the `kr64` Rust crate on every push.
- **Android NDK** — cross-compilation of native libraries for `arm64-v8a` and `x86_64`.
- **[Rust](https://www.rust-lang.org/)** — language for the new `libOpenglRender` / `libloader` / `kr64` replacements, built via `cargo-xdk`.

## License

twoyi is licensed under the Mozilla Public License, Version 2.0 (see
`LICENSE`). AOSP-derived code retains its Apache 2.0 license where noted.

---

Made with help from many people, many projects, and one long night of
automated agents.
