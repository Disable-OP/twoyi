# Rootless Android Virtualization Architecture

## Research-First Engineering Document

**Status:** Research complete, implementation pending
**Date:** 2026-08-09

---

## 1. The Problem

Twoyi is a **rootless** Android virtualizer: it runs a guest Android system
inside one app process, without root, ADB, or custom ROM. The app runs as
`untrusted_app`, which means:

- **No `CAP_SYS_ADMIN`** — cannot `mount()`, `chroot()`, `unshare(CLONE_NEWNS)`, `pivot_root()`
- **No `CAP_MKNOD`** — cannot `mknod()` device nodes
- **Zygote seccomp filter** — `mount()` and `chroot()` are `SECCOMP_RET_KILL_PROCESS`
- **`AT_SECURE` set** — bionic linker **ignores `LD_PRELOAD`** for app processes

Source: `bionic/libc/SECCOMP_BLACKLIST_APP.TXT` (Android 9+), `bionic/linker/linker_main.cpp`

---

## 2. How Existing Solutions Work (Primary Sources)

### 2.1 Virtual Master (com.clone.android.dual.space)

Confirmed by disassembly of `libkr64.so` + `libkrloader64.so`:

1. **Custom ELF interpreter** — `libkrloader64.so` is a rebuilt bionic linker (built from AOSP `bionic/linker/` source). Guest binaries have `PT_INTERP` → `libkrloader64.so`.

2. **Inline hooks (shadowhook v1.0.8)** — Hooks libc syscall wrappers (`mount`, `chroot`, `pivot_root`, `mknod`, `openat`, `stat`) at the PLT/inline level. The `svc` instruction NEVER fires.

3. **Seccomp-BPF filter + SIGSYS handler** — backstop for syscalls that bypass libc. Only works for syscalls the zygote `ALLOW`s.

4. **Virtual `/dev` devices** — `mknodat` + AF_UNIX `bind` creates socket files.

### 2.2 Original Twoyi (github.com/twoyi/twoyi)

1. **`fork()` + `execve("./init")`** — guest init runs as ordinary child of untrusted_app.
2. **Custom dynamic linker** — `libloader.so` (staged as `loader64`), via `TYLOADER` env var.
3. **Emugl renderer** — `libOpenglRender.so` (AOSP Android-emulator OpenGL renderer).
4. **Patched goldfish ROM** — guest is a patched Android-emulator image.

### 2.3 Nogitsune (cyanmint's reimplementation)

- Stages `libloader.so` → `<rootfs>/loader64`
- `spawnInitTwoyiStyle()`: `ProcessBuilder("./init").environment()["TYLOADER"]=…`
- Sets `ro.hardware=goldfish`

---

## 3. The Real Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Host Android (untrusted_app)                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Twoyi App (Java + Rust)                              │  │
│  │  - Renderer (libOpenglRender.so — Emugl)              │  │
│  │  - qemu_pipe proxy (AF_UNIX → renderer)               │  │
│  │  - Input bridge (touch/key → AF_UNIX sockets)         │  │
│  │  - Boot helper (fork+exec guest init)                 │  │
│  └───────────────────────┬───────────────────────────────┘  │
│                          │ fork() + execve("./init")         │
│                          ▼                                   │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Guest Process (same UID as app)                      │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │  Custom ELF Interpreter (libloader.so)          │  │  │
│  │  │  - Built from AOSP bionic/linker/               │  │  │
│  │  │  - PT_INTERP → this linker                      │  │  │
│  │  │  - Runs BEFORE guest main()                     │  │  │
│  │  │  - Installs shadowhook inline hooks             │  │  │
│  │  │  - Installs seccomp-BPF + SIGSYS handler        │  │  │
│  │  └───────────────────────┬─────────────────────────┘  │  │
│  │                          ▼                              │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │  Guest init (patched goldfish)                   │  │  │
│  │  │  - FirstStageMain (mount/mkdir/mknod → hooked)   │  │  │
│  │  │  - SecondStageMain (property service, zygote)    │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. Why LD_PRELOAD Stubs Don't Work

The current `getpid_hook.c` is an EXPERIMENTAL diagnostic tool, not the architectural solution:

1. **`LD_PRELOAD` is ignored for apps** — bionic only honors it when `!getauxval(AT_SECURE)`. App processes have `AT_SECURE` set.
2. **PLT interposition misses direct `svc`** — static code, JIT, and hand-coded asm bypass libc wrappers.
3. **Seccomp KILL shadows TRAP** — `mount()`/`chroot()` are `SECCOMP_RET_KILL_PROCESS` in the zygote filter. `SECCOMP_RET_TRAP` from our filter cannot override `KILL_PROCESS` (precedence: KILL > TRAP > ERRNO).
4. **Fake success hides required failures** — init expects `mount("tmpfs","/dev")` to create a FRESH tmpfs. Returning 0 without doing anything means subsequent `mkdir("/dev/pts")` operates on the HOST's `/dev`.

---

## 5. Component Breakdown

### 5.1 Custom ELF Interpreter (libloader.so)
- Built from AOSP `bionic/linker/`
- Guest binaries have `PT_INTERP` → this linker
- Runs BEFORE guest `main()`
- Installs hooks before calling guest `_start`

### 5.2 Inline Hooks (shadowhook)
- ByteDance shadowhook v1.0.8 (https://github.com/bytedance/android-inline-hook)
- Hooks libc syscall wrappers so `svc` never fires
- Must hook: `mount`, `chroot`, `pivot_root` (KILL_PROCESS in seccomp)
- Path translation: `openat`/`stat`/`access` → prefix with `{TWOYI_ROOTFS}`

### 5.3 Seccomp-BPF + SIGSYS Handler (backstop)
- Catches syscalls that bypass libc (direct `svc`, JIT)
- Only for syscalls zygote ALLOWs (`mknodat`, `unshare`)
- Handler reads args from `ucontext_t`, emulates, writes return value
- Model: Chromium `sandbox/linux/seccomp-bpf/trap.cc`

### 5.4 Path Translation
- `openat("/dev/foo")` → `openat("{TWOYI_ROOTFS}/dev/foo")`
- `/proc/cmdline` → virtual file with guest's cmdline
- Provides filesystem isolation without chroot

### 5.5 Patched Goldfish ROM
- Skip `first_stage_mount`
- Permissive SELinux
- `ro.hardware=goldfish`

---

## 6. What init Actually Needs

From `system/core/init/first_stage_init.cpp` `FirstStageMain()`:

| Call | What init expects | Real implementation |
|------|-------------------|---------------------|
| `mount("tmpfs","/dev",...)` | Fresh empty tmpfs | Inline hook: no-op (rootfs `/dev` already populated) |
| `mkdir("/dev/pts",0755)` | Create subdir | Inline hook: create under `{TWOYI_ROOTFS}/dev/pts` |
| `mount("proc","/proc",...)` | Fresh procfs | Inline hook: no-op + virtualize `/proc/cmdline` |
| `mknod("/dev/kmsg",...)` | Char device | Inline hook: create ring buffer file |
| `setgroups(...)` | Drop groups | Inline hook: return 0 (EPERM without CAP_SETGID) |

**Critical:** `InitKernelLogging(argv)` requires `/dev/kmsg` to be openable after the mount block.

---

## 7. Implementation Plan

### Phase 1: Custom ELF Interpreter (libloader.so)
- Build from AOSP `bionic/linker/`
- Add `TWOYI_ROOTFS` env var support
- Add hook installation before `main()`

### Phase 2: Inline Hooks (shadowhook)
- Integrate shadowhook v1.0.8
- Hook `mount`/`chroot`/`pivot_root`
- Hook `openat`/`stat`/`access` (path translation)
- Hook `getpid`/`getppid`

### Phase 3: Seccomp/SIGSYS Backstop
- Install BPF filter for `mknodat`/`unshare`
- Implement SIGSYS handler (Chromium trap.cc pattern)

### Phase 4: Virtual Device Layer
- AF_UNIX socket creation at `/dev/<name>`
- `/dev/kmsg` ring buffer

### Phase 5: Patched Goldfish ROM
- Skip `first_stage_mount`
- Permissive SELinux

---

## 8. Current Status (Honest)

- ✅ Renderer (libOpenglRender.so) — working
- ✅ qemu_pipe proxy — working
- ✅ Device sockets — working
- ✅ Rootfs extraction — working
- ⚠️ getpid_hook.c — EXPERIMENTAL/DIAGNOSTIC only
- ❌ Custom ELF interpreter — NOT implemented (critical gap)
- ❌ Inline hooks (shadowhook) — NOT integrated
- ❌ Seccomp/SIGSYS handler — NOT implemented
- ❌ Path translation — NOT implemented
- ❌ Patched goldfish ROM — NOT available

**The container CANNOT boot without the custom ELF interpreter + inline hooks.**

---

## 9. Sources

### AOSP
- `system/core/init/first_stage_init.cpp`: https://android.googlesource.com/platform/system/core/+/refs/heads/main/init/first_stage_init.cpp
- `bionic/linker/linker_main.cpp`: https://android.googlesource.com/platform/bionic/+/refs/heads/main/linker/linker_main.cpp
- `bionic/libc/SECCOMP_BLACKLIST_APP.TXT`: https://android.googlesource.com/platform/bionic/+/android-9.0.0_r33/libc/SECCOMP_BLACKLIST_APP.TXT

### Kernel / seccomp
- https://www.kernel.org/doc/html/v5.0/userspace-api/seccomp_filter.html
- https://man7.org/linux/man-pages/man2/seccomp.2.html

### Reference implementations
- Chromium: https://chromium.googlesource.com/chromium/src/+/lkgr/sandbox/linux/seccomp-bpf/trap.cc
- shadowhook: https://github.com/bytedance/android-inline-hook
- proot: https://proot-me.github.io

### Projects
- twoyi: https://github.com/twoyi/twoyi
- Nogitsune: https://github.com/cyanmint/Nogitsune
