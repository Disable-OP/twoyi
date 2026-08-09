# Architecture Decision Records (ADRs)

> Project: **twoyi** — rootless Android-on-Android container (`io.twoyi`)
> Fork: `Disable-OP/twoyi` branch `main` (was `cyanmint/twoyi` branch `improvements/initial-cleanup`, consolidated 2026-08-08)
> Format: **Status**, **Context**, **Decision**, **Consequences**.
> Author: general-purpose sub-agent (Task KEEP-WORKING-8) · 2026-08-05

The eight most consequential architectural decisions made during the
twoyi improvement project. Each reflects a real fork in the road where
an equally-plausible option was rejected for documented reasons. To
supersede a record, add a new ADR marking the prior one as
**Superseded**; do not delete the old record.

---

## ADR-001: Use Rust for the `kr64` daemon

**Status:** Accepted

### Context

VM's `libkr64.so` is a 1.5 MB stripped, OLLVM-obfuscated ARM64 binary
that must materialise a 20+ entry per-VM `/dev` tree, install a seccomp
BPF filter with a SIGSYS handler, synthesise `/proc`, set up the mount
namespace via `unshare` + `pivot_root`, and `execve` the guest's
`/system/bin/init` (`download/VM_KR64_ANALYSIS.md`). Twoyi needed a
clean-room reimplementation. Candidates: **C++** (matches VM and AOSP
`emugl`); **C** (minimal deps); **Rust** (already used by
`libtwoyi.so`/`libloader.so`).

### Decision

Write `kr64` in **Rust**, depending only on `std` + `libc` (no `nix`,
`once_cell`, or `log`). Replace `log` with crate-local `info!`/
`warning!`/`error!` macros → `eprintln!`. Replace `once_cell::Lazy`
with `std::sync::OnceLock`. Use `libc::*` directly for syscalls.
Commit `570e95e`.

### Consequences

- **+** Shares the cargo-xdk / NDK r27c toolchain with the rest of the
  Rust side — no second build system. Compile-time memory safety around
  `mmap`/`fork`/`execve`. Zero external runtime deps → the `.so` is
  self-contained (the daemon is `execve`'d as a standalone PIE binary
  via a custom `.interp` section).
- **−** The `libc` crate does not expose `si_syscall` on `siginfo_t`;
  the SIGSYS handler reinterprets the raw pointer through a
  `#[repr(C)] struct SigsysSiginfo` — a contained `unsafe` block.
  `cargo test` cannot exercise the Android-only
  `--dynamic-linker=/system/bin/linker64` path; `interp.c` carries an
  `#ifdef __ANDROID__` gate so the Linux-host test binary runs under
  the default `/lib64/ld-linux-x86-64.so.2`.
- **Follow-up:** `kr64` is not yet a member of the parent Cargo
  workspace and is not built by `build_rs.sh` (`ARCHITECTURE.md`
  §5.5.5 item 7).

---

## ADR-002: Build `libOpenglRender.so` from AOSP source

**Status:** Accepted

### Context

Upstream twoyi shipped a 1,059,128-byte arm64-only closed-source
`libOpenglRender.so` blob — disassembly proved it was a lightly-modified
build of AOSP `emugl`. Problems: (1) **x86_64 impossible** — no source;
(2) statically linked ~290 KB of desktop-GL translator `.so`s, bypassing
the device's real GPU driver; (3) closed-source blobs cannot be audited,
patched, or debugged. Options: keep the blob (arm64-only forever); write
a renderer from scratch in Rust; rebuild AOSP `emugl` from source.

### Decision

Rebuild `libOpenglRender.so` **from AOSP `platform/sdk` at commit
`7a712acc`** (Apache-2.0). Sparse-checkout the `emugl` tree, build the
`emugen` code generator, write a POSIX-compat shim for Android-private
headers, and apply nine targeted patches: `void*` for `FBNative*Type`
(no X11); link the device's real `libEGL.so`/`libGLESv1_CM.so`/
`libGLESv2.so`; rewrite `UnixStream.cpp::make_unix_path()` to read
`$TWOYI_ROOTFS`; add `NativeAndroidSubWindow.cpp`; write new
`twoyi_api.cpp` with the six twoyi entry points + four `dl*_ex`
wrappers; drop `static` from `s_renderThread`; new `CMakeLists.txt`.
Then port the three legacy-only pieces (`dl_ex.cpp`,
`GraphicBuffer.{h,cpp}`, `startGBServer.cpp`) on top. Commits
`47f8335` (initial) + `eb13449` (port).

### Consequences

- **+** Ship a 605–611 KB `.so` for **both** `arm64-v8a` and `x86_64`,
  vs. a single 1.06 MB arm64-only blob — 43% smaller and a whole new
  ABI. Links the device's real GPU driver instead of a desktop-GL
  translator — architecturally superior, matches what VM's `libvm.so`
  does. All 11 twoyi-required C-ABI symbols verified exported on both
  ABIs (`ARCHITECTURE.md` §5.4.4).
- **−** **Not yet verified end-to-end on a real device** — compiles,
  links, exports the right symbols, but SurfaceFlinger-driven GL
  streaming has not been visually confirmed
  (`download/TWOYI_HONEST_STATUS.md`). Future AOSP `emugl` upstream
  changes require re-sparse-checkout + re-patch; the nine patches are
  not upstream candidates because twoyi's `startOpenGLRenderer`
  signature diverges from AOSP's.
- The Rust `openglrenderer` crate is retained as the x86_64-default
  fallback (ADR-004).

---

## ADR-003: Dynamic data directory (work profile support)

**Status:** Accepted

### Context

Earlier twoyi hardcoded `/data/data/io.twoyi/` in **eight** places
across the Rust crate (`core.rs`, `input.rs`, `socket_monitor.rs`).
This broke when the app was installed inside an Android **work
profile**, where the data dir is `/data/user/<uid>/io.twoyi/` instead.
Work-profile installs are the common case for "clone a second twoyi
inside a corporate container", so the hardcodes were a real adoption
blocker. Options: keep hardcodes and document the limitation; `#ifdef`
per install location; resolve once at runtime and thread through.

### Decision

Resolve the data dir **once** from Java via
`Context.getDataDir().getAbsolutePath()`, hand it to Rust through a new
JNI method `Renderer.setDataDir(String)`, store it in a
`std::sync::OnceLock<String>` (single-assignment by design), and derive
every hardcoded path via helper functions (`get_rootfs_dir()`,
`get_log_path()`, `get_touch_path()`, `get_key_path()`,
`get_opengles_paths()`). The AOSP-built `libOpenglRender.so` follows
suit via an **environment variable**: `core.rs::init_renderer()` exports
`TWOYI_ROOTFS=<data_dir>/rootfs` when spawning the guest's `./init`;
`UnixStream.cpp::make_unix_path()` reads `TWOYI_ROOTFS` (defaulting to
`/data/data/io.twoyi/rootfs`). Commit `9c4b907`.

### Consequences

- **+** Work-profile installs work. The `unwrap_or("/data/data/io.twoyi")`
  fallback in `get_data_dir()` preserves backwards compatibility with
  older Java builds that don't call `setDataDir`. The env-var convention
  for the C++ renderer means future native modules don't need a JNI
  round-trip to learn the rootfs path.
- **−** The `OnceLock` is genuinely single-assignment — no reset path.
  If the data dir ever needs to change at runtime (multi-window with
  two profiles in one process), this breaks and a `RwLock<String>`
  refactor is required. Every Rust file that previously `const`-declared
  a path now calls a function — marginal runtime cost and slightly
  noisier call sites.

---

## ADR-004: Default to the new renderer on x86_64

**Status:** Accepted

### Context

The original twoyi shipped one renderer — the legacy arm64-only blob.
The fork added a second, open-source Rust renderer. Java's
`ProfileSettings.useNewRenderer()` defaulted to `false`, so on x86_64
(newly added in this fork) the app selected the old renderer — which is
**not shipped** for that ABI. `renderer_bindings.rs` provided panic-stubs
for non-aarch64 that called `abort()`:

```
signal 6 (SIGABRT)  #11 renderer_reset_window+204  #14 Render2Activity$1.surfaceChanged
```

i.e. `surfaceChanged → renderer_reset_window → panic stub → SIGABRT`.
Options: ship the legacy blob for x86_64 (impossible — no source);
build the AOSP `libOpenglRender.so` for x86_64 (ADR-002, done, but its
`startGBServer` depends on `AHardwareBuffer_recvHandleFromUnixSocket`
which historically wasn't wired up on x86_64 emulator images); force
the Rust renderer on non-aarch64 hosts.

### Decision

Force the **new Rust renderer** on non-aarch64 hosts, in two layers
(defense in depth): (1) Java `ProfileSettings.useNewRenderer()` defaults
to `true` when the device's primary ABI is not `arm64-v8a`; (2) Rust
`core.rs::effective_renderer_type()` forces `RendererType::New` on
non-aarch64 even if Java requests `Old`. Commit `7664c66`.

### Consequences

- **+** The SIGABRT no longer reproduces on the codespace's redroid
  x86_64. The app stays alive long enough to log its real state, which
  is how we discovered the next layer of limitation (QEMU pipe
  unavailable — `download/X86_64_BREAKTHROUGH.md`). The Rust renderer
  successfully initialises a GL context and connects to the QEMU pipe
  on x86_64.
- **−** The Rust renderer is **less complete** than the AOSP C++ build
  — it does not implement the `startGBServer` graphics-buffer proxy. So
  x86_64 users get a renderer that boots but cannot yet composite
  SurfaceFlinger output. Known gap.
- **−** The two-layer override is subtle — a future contributor reading
  `ProfileSettings.useNewRenderer()` may not realise the Rust side
  silently re-overrides their `false` return on x86_64. The
  `effective_renderer_type()` doc comment must stay loud about this.
- **Follow-up:** when the AOSP build's `startGBServer` works on x86_64,
  this default may be superseded by a per-ABI preference table.

---

## ADR-005: POSIX `sh` compatibility for build scripts

**Status:** Accepted

### Context

Build scripts (`build_rs.sh`, `loader/build.sh`,
`openglrenderer/build.sh`, `twoyi.sh`) run from both the dev shell and
GitHub Actions CI. On Ubuntu CI, `/bin/sh` is **dash**, not bash. The
first iteration of `build_rs.sh` used bash arrays
(`ABIS=("arm64-v8a" "x86_64"); for abi in "${ABIS[@]}"`) — dash treats
`(` as a syntax error → CI fails with `Syntax error: "(" unexpected`.
The script worked on macOS (where `/bin/sh` is bash) and devcontainers,
so the failure was CI-only and confusing. Options: `#!/usr/bin/env bash`
everywhere; rewrite in Python; rewrite in POSIX `sh`.

### Decision

Rewrite all build scripts in **POSIX `sh`** with `#!/bin/sh` and
`set -e`. Replace bash arrays with whitespace-separated strings iterated
via `for abi in $ABIS; do ...` (unquoted `$ABIS` deliberately triggers
word-splitting). Replace `[[ ... ]]` with `[ ... ]`, `function name {`
with `name() {`. Document in `CONTRIBUTING.md`: "Shell: POSIX
`sh`-compatible (the repo's `build_rs.sh` deliberately avoids bash
arrays so it works under dash on Ubuntu CI). Use `set -e`." Commit
`d2cfb8d`.

### Consequences

- **+** Every script runs identically under dash, bash, ksh, mksh, and
  ash (BusyBox). CI no longer depends on bash being installed. Scripts
  are shorter — POSIX `sh`'s limitations push toward simpler constructs.
- **−** Lost bash features: arrays, `[[ =~ ]]` regex, `${var,,}`
  lowercasing, `mapfile`, process substitution `<(...)`. Future
  contributors must find POSIX workarounds.
- **−** The unquoted `$ABIS` word-splitting idiom looks like a bug to
  anyone with `shellcheck` muscle memory — `shellcheck` flags it as
  SC2086. We accept the lint warning as intentional and document why in
  the script header.
- **Alternative rejected:** `#!/usr/bin/env bash` adds a hidden runtime
  dependency that bites contributors on minimal containers (Alpine,
  distroless) where bash is not installed by default.

---

## ADR-006: Kernel replacement over KVM

**Status:** Accepted

### Context

Goal: boot a second Android userland *inside one normal Android app
process* on an **unrooted** host device. Two approaches: **KVM** — boot
the guest in a real VM, requires `/dev/kvm` exposed to the app process
(Pixel 6+ pKVM only; does not work on the vast majority of phones in
users' hands); **container / kernel replacement** — share the host
kernel, synthesise everything else the guest expects (per-VM `/dev`
tree, `/proc`, mount namespace, seccomp, binder). This is what VM does
with `libkr64.so`. KVM is conceptually simpler (a real kernel handles
syscalls/mounts/devices) but requires reimplementing slices of kernel
behaviour in userspace. The deployment reality is decisive:
targetSdk=28 APKs cannot depend on `/dev/kvm`.

### Decision

Pursue the **kernel-replacement container path** as primary. Build the
`kr64` Rust daemon (ADR-001): `fork`s, sets up the mount namespace via
`unshare(CLONE_NEWNS)` + `pivot_root`, installs a seccomp BPF filter
with a SIGSYS handler that traps `mount`/`umount2`/`swapon`/`reboot`
and kills on `ptrace`/`kexec_load`/`init_module`/`pivot_root`,
materialises 6 MVP virtual devices, synthesises `/proc`, and `execve`s
the guest's `/system/bin/init`. KVM is documented as a separate,
out-of-scope alternative in `GSI_BOOT_PLAN.md` §5.5. Commit `570e95e`
(skeleton).

### Consequences

- **+** Works on any unrooted Android phone with `minSdk 27` — the
  entire install base, not just Pixel 6+ pKVM devices. Shares the host
  kernel — no kernel image to ship, no per-SoC kernel build matrix.
- **−** The `kr64` daemon must reimplement kernel behaviour (mount
  trapping, `/proc` synthesis, device-tree materialisation) in
  userspace. This is the bulk of the ~11,554-LOC Rust skeleton and the
  8–12 week MVP estimate.
- **−** **Binder virtualisation is the hardest piece** and is not yet
  started. Without it the guest's `servicemanager` cannot register
  `IActivityManager`, so the guest can boot to `init` but cannot run a
  full system_server. MVP workaround: patch `system_server` to skip
  `publishService` calls.
- **−** `unshare(CLONE_NEWNS)` requires `CAP_SYS_ADMIN`, which the twoyi
  app process does not have. The skeleton detects `EPERM` and falls back
  to `chroot` — degraded isolation but functional for development.
- **Follow-up:** if Android's pKVM becomes widely available, the KVM
  path becomes viable and this ADR may be **Superseded** by a hybrid
  (KVM where available, container otherwise).

---

## ADR-007: Per-block XOR string deobfuscation

**Status:** Accepted

### Context

VM's `libkr64.so` is OLLVM-obfuscated: `.symtab` stripped, control-flow
flattening, and `.data` strings are XOR-encrypted with **per-string byte
keys** (not a single global key). `.rodata` is plaintext (key 0) and
contains shadowhook v1.0.8 — easy. The interesting strings — 20+
virtual device paths (`/dev/vmproc`, `/vm/vm%d/dev/qemu_pipe`, etc.) —
live in `.data` under per-string keys. A standard single-key XOR
brute-force failed: a 4 KB `.data` page typically contains 8–12 strings
each with a different key, so any single XOR pass produces noise from
the other strings on the same page — thousands of false-positive "hits"
at every key. Options: dynamic runtime dump under `qemu-user -strace`
or `frida`; symbolic execution of each `.datadiv_decode*` thunk (77 of
them); per-block XOR brute-force keyed on the NUL terminator pattern.

### Decision

Implement **per-block XOR brute-force** in `kr64-analysis/xor_brute.py`
and `xor_scan_text.py`. For each of 256 candidate keys `k`: walk `.data`
byte-by-byte; at each byte XOR a forward run of up-to-256 bytes with
`k`; treat the run as a candidate string if it contains only printable
ASCII + NUL terminator after XOR; score against ~150 known substrings
(`/dev/binder`, `qemu_pipe`, `servicemanager`, `linker64`,
`BINDER_WRITE_READ`, `/proc/self/maps`, etc.). A hit at offset `o` with
key `k` means **that specific 4–32 byte block** is XOR'd with `k`.
Record `(offset, key, decoded)` and move on — the next block on the
same page may use a different key.

### Consequences

- **+** Recovered **50+ virtual device paths** and the full per-VM
  socket-path table (`/vm/vm%d/dev/qemu_pipe` key `0xba`,
  `/vm/vm%d/dev/touch` key `0x03`, `/vm/vm%d/dev/gb` key `0xe0`,
  `/vm/vm%d/dev/gb2` key `0x0c`, `/vm/vm%d/dev/netlink_server` key
  `0x2c`, etc. — full table in `ARCHITECTURE.md` §9.3). Decoded the
  `mount_mgr` error strings (key `0x1a`) which directly informed the
  design of `kr64/src/mount_mgr.rs`. **Deterministic and reproducible**
  — no emulator or runtime instrumentation needed.
- **−** Depends on the candidate substring list. Strings that don't
  match any known prefix (e.g. log format strings with no `/dev` or
  `/proc`) are missed. The decoded catalog is therefore a **lower
  bound**, not a complete recovery.
- **−** Per-block scoring is O(256 × page_size × num_substrings) —
  fast enough on a 1.5 MB binary (~30s) but would not scale to a 100 MB
  binary without segmentation.
- **Alternative that may supersede:** dynamic runtime string-dump under
  `frida` would recover **all** decoded strings regardless of substring
  match, at the cost of needing a working ARM64 emulator + the binary's
  init path (Action 1 in the VM-KR64-1 worklog entry).

---

## ADR-008: Two-process architecture (`libvm.so` in-process + `libkr64.so` separate process)

**Status:** Accepted

### Context

VM splits its native code across two libraries. **`libvm.so`** (7.7 MB)
is loaded in-process via `System.loadLibrary("vm")` and handles
everything that needs JNI: display, input, audio, HAL, binder
virtualisation (via a Java `Proxy` of `IActivityManager`), OS boot
trigger, and the GL renderer. **`libkr64.so`** (1.5 MB) is a standalone
ELF executable disguised as a `.so` (`.interp` → custom
`libkrloader64.so`) — the kernel exec's it, not loaded as a JNI library.
It handles everything that needs to run as the guest's effective "PID 1":
`unshare` + `pivot_root`, seccomp install, `/dev` tree materialisation,
`/proc` synthesis, and `execve(/system/bin/init)`. Three pressures force
the split: (1) **seccomp is irreversible per-process** — installing the
guest's filter would break host-side JNI calls in a shared process;
(2) **mount namespaces are per-process** — `pivot_root` would isolate
the host Android framework's file accesses; (3) **the guest's `init`
expects to be PID 1** — running it as a child of `libkr64.so` satisfies
this without contorting `init`'s assumptions. Options for twoyi:
one-process (accept seccomp/mount conflicts); two-process matching VM;
three-process (split GL renderer out too).

### Decision

Adopt the **two-process architecture** matching Virtual Master.
**In-process** (`libtwoyi.so`, Rust): JNI entry, renderer dispatch,
input system, guest spawn trigger — no seccomp, no pivot_root.
**Separate process** (`libkr64.so`, Rust, PIE-as-cdylib via `interp.c`
+ `build.rs`): `fork` from `libtwoyi.so`, install seccomp + mount
namespace, `execve` the guest's `/system/bin/init` as effective PID 1.
The two communicate over the per-VM Unix sockets that `kr64`
materialises (`/dev/qemu_pipe`, `/dev/input/touch`, `/dev/event`, etc.)
and over the host's binder (until binder virtualisation lands). Commit
`570e95e` (skeleton).

### Consequences

- **+** The in-process side keeps full JNI access to the host Android
  framework — `Renderer.init(surface, …)`, `Renderer.handleTouch(…)`,
  `Renderer.setDataDir(…)` all work as normal JNI calls. The separate
  `kr64` process can install a strict seccomp filter, `pivot_root` into
  the guest rootfs, and `execve` `init` without affecting the host
  app's other threads. Matches VM's proven architecture — every
  surprise VM hit during reverse-engineering, twoyi is likely to hit
  too, and having the same process split makes the lessons transferable.
- **−** The PIE-as-cdylib trick is fragile — it relies on Android's
  `linker64` accepting a `.so` as the main executable. VM sidesteps
  this with a custom `libkrloader64.so`; twoyi uses the system linker
  (simpler, but loses VM's elevated-privilege early bootstrap).
  Two-process split makes debugging harder — `logcat` shows the `kr64`
  process's logs interleaved with the host app's. The skeleton uses
  `eprintln!("[KR64 <LEVEL>] …")` to make its lines greppable.
- **−** `libkr64.so` must be `chmod +x` and `execve`'d by the host app,
  which requires the app's data directory to be executable. Works on
  standard Android but breaks on some hardened SELinux policies.
- **Follow-up:** three-process split (renderer in its own process) is
  **not** adopted — the renderer needs the `ANativeWindow` from the
  Java `Surface`, most cheaply passed in-process. If multi-VM support
  lands (per-VM renderer handle per VM's
  `DisplayService.nativeAddSurface(ptr, …)`), this decision may be
  revisited.

---

*Living reference. When a decision is superseded, do not delete the
record — add a new ADR with `Status: Superseded by ADR-NNN` and a
one-line rationale, then write the replacement ADR below it. Commit
references: 001/006/008 → `570e95e`; 002 → `47f8335`+`eb13449`;
003 → `9c4b907`; 004 → `7664c66`; 005 → `d2cfb8d`; 007 is analysis-only.*
