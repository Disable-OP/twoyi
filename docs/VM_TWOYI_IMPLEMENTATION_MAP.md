# Virtual Master → Twoyi Implementation Map

## Detailed Reverse-Engineering Report

**Status:** Research complete. No implementation code written.
**Date:** 2026-08-09
**Method:** Binary analysis of VM libraries + cross-reference with AOSP source, kernel docs, Chromium sandbox, shadowhook docs, original Twoyi/Nogitsune source.

---

## Executive Summary

Virtual Master achieves rootless Android-on-Android virtualization using a **seccomp-BPF + SIGSYS handler** as the PRIMARY mechanism, with shadowhook inline hooks as a SECONDARY mechanism for specific libc functions. The custom ELF interpreter (`libkrloader64.so`) is a rebuilt AOSP bionic linker that installs the seccomp filter before the guest `main()` runs.

**KEY CORRECTION from previous assumptions:** The zygote's seccomp filter uses `SECCOMP_RET_TRAP` (NOT `SECCOMP_RET_KILL_PROCESS`) for all blacklisted syscalls (`mount`, `chroot`, `mknod`, `unshare`, etc.). This means a SIGSYS handler CAN intercept and emulate these syscalls. The previous belief that inline hooks were the ONLY way to intercept `mount`/`chroot` was WRONG.

---

## 1. Startup Path (guest ELF → guest main)

### 1.1 The Custom ELF Interpreter (`libkrloader64.so`)

**VERIFIED** — `libkrloader64.so` is a rebuilt AOSP bionic dynamic linker.

| Property | Value | Evidence |
|----------|-------|----------|
| Size | 217,456 bytes | file stat |
| Arch | AArch64, ET_DYN | `readelf -h` |
| Entry point | `0x2cd0` (`_start`) | `readelf -h` |
| PT_INTERP | `/system/bin/linker64` | `readelf -l` (it itself uses the system linker) |
| DT_NEEDED | `libc++.so`, `libdl.so`, `libc.so`, `libm.so` | `readelf -d` |
| Imported functions | ONLY `malloc`, `free`, `calloc`, `realloc` | `readelf --dyn-syms` — everything else is statically linked |
| Build path | `out_gp/target/product/marlin/obj/EXECUTABLES/krloader_intermediates/LINKED/libkrloader64.so` | `.comment` section — PROVES it was built in the AOSP build system |
| Compiler | Obfuscator-LLVM clang 5.0.2 + Android clang 3.8.256229 | `.comment` section |

**Startup path disassembly (VERIFIED):**

```
_start @ 0x2cd0:
    mov x0, sp           ; pass raw KernelArgumentBlock
    bl  0x40b0           ; call __linker_init
    br  x0               ; jump to guest entry point

__linker_init @ 0x40b0:
    ; Parse argc/argv/envp/auxv from stack
    ; Call getauxval(AT_BASE=7) → get linker's own load address
    ; Call getauxval(AT_ENTRY=9) → get guest's entry point
    ; Read own ELF header (e_phoff at +0x20, e_phnum at +0x38) for self-relocation
    ; Install seccomp BPF filter (see §3.1)
    ; Load + relocate guest ELF (mprotect + __builtin___clear_cache pattern)
    ; Return guest entry point in x0
```

**This EXACTLY matches AOSP `bionic/linker/arch/arm64/begin.S` and `bionic/linker/linker_main.cpp` `__linker_init()`.**

Sources:
- AOSP `bionic/linker/linker_main.cpp`: https://android.googlesource.com/platform/bionic/+/android-11.0.0_r1/linker/linker_main.cpp
- AOSP `bionic/linker/arch/arm64/begin.S`

### 1.2 How the Guest Gets exec'd

**VERIFIED** — via original Twoyi source + Nogitsune source.

The host Java app calls:
```java
// Nogitsune BootHelper.kt
ProcessBuilder("./init")
    .directory(rootfs)                    // cwd = /data/data/.../rootfs
    .redirectOutput(log)
    .environment()["TYLOADER"] = loader_path  // path to libloader.so
    .start()
```

This triggers `fork()` + `execve("./init", argv, envp)` with cwd=rootfs. The kernel:
1. Loads `./init` (the guest ELF)
2. Reads its `PT_INTERP` = `./loader64` (relative path)
3. Resolves `./loader64` against cwd → `{rootfs}/loader64`
4. `execve`s `{rootfs}/loader64` as the ELF interpreter
5. `{rootfs}/loader64` is a copy of `libkrloader64.so` (staged by `stageGuestLoader`)

The loader runs, loads the guest's DT_NEEDED dependencies, installs hooks, and jumps to guest `main()`.

**Sources:**
- Twoyi `app/rs/src/lib.rs`: `Command::new("./init").current_dir(rootfs).env("TYLOADER", loader_path)`
- Nogitsune `BootHelper.kt` `spawnInitTwoyiStyle()`: `ProcessBuilder("./init").directory(cwd).environment()["TYLOADER"]=...`

---

## 2. Hook Installation

### 2.1 Seccomp BPF Filter (PRIMARY mechanism)

**VERIFIED** — installed by `libkrloader64.so` before guest `main()`.

**Installation sequence (at `0x3384` in libkrloader64.so):**

```
1. rt_sigprocmask(SIG_BLOCK, {SIGSYS=bit 30})     @ 0x3964
   ; Block SIGSYS before installing the filter to prevent races

2. prctl(PR_SET_NO_NEW_PRIVS=0x26, 1, 0, 0, 0)    @ 0x3a28
   ; Mandatory prerequisite for seccomp

3. prctl(PR_SET_SECCOMP=0x16, SECCOMP_MODE_FILTER=2, &fprog)  @ 0x3c00
   ; Install the BPF filter
```

**BPF filter structure (decoded from stack construction):**

```
[0] BPF_LD_W_ABS k=0x4                    ; A = seccomp_data.arch
[1] BPF_JEQ_K jt=1 jf=0 k=0xC00000B7      ; if (A == AUDIT_ARCH_AARCH64) skip 1
[2] BPF_RET k=0x7FFF0000                  ; return SECCOMP_RET_ALLOW (non-AARCH64)
[3] BPF_LD_W_ABS k=0xC                    ; A = seccomp_data.nr (syscall number)
[4+] BPF_JEQ_K for each trapped syscall   ; if matched → SECCOMP_RET_TRAP
    default → SECCOMP_RET_ALLOW
```

**Syscalls trapped (from `libkr64.11.so` BPF construction at `0x111d84`):**

| Syscall | Nr (arm64) | Action |
|---------|-----------|--------|
| `mount` | 40 | TRAP → emulate |
| `umount2` | 39 | TRAP → emulate |
| `unshare` | 97 | TRAP → emulate |
| `ptrace` | 117 | TRAP → emulate |
| `setgid` | 144 | TRAP → emulate |
| `setuid` | 146 | TRAP → emulate |
| `setgroups` | 159 | TRAP → emulate |
| `setresuid` | 147 | TRAP → emulate |
| `setresgid` | 149 | TRAP → emulate |
| `getgroups` | 158 | TRAP → emulate |
| `capset` | 90 | TRAP → emulate |
| `capget` | 91 | TRAP → emulate |
| `setpriority` | 140 | TRAP → emulate |
| `getrusage` | 164 | TRAP → emulate |
| `getcpu` | 167 | TRAP → emulate |
| `adjtimex` | 170 | TRAP → emulate |
| `recvmsg` | 208 | TRAP → emulate |
| `shutdown` | 209 | TRAP → emulate |
| 6000 | (custom) | TRAP → VM hypercall |
| 6001 | (custom) | TRAP → VM hypercall |

Plus many more with dedicated handlers (see §3.2): `mknodat(33)`, `mkdirat(34)`, `unlinkat(35)`, `openat(56)`, `newfstatat(79)`, `chroot(51)`, `clone(212)`, `execve(221)`, `bind(199)`, etc.

**Sources:**
- Kernel seccomp docs: https://www.kernel.org/doc/html/v5.0/userspace-api/seccomp_filter.html
- AOSP `bionic/libc/seccomp/seccomp_policy.cpp`: https://android.googlesource.com/platform/bionic/+/android-11.0.0_r1/libc/seccomp/seccomp_policy.cpp
- AOSP `bionic/libc/SECCOMP_BLACKLIST_APP.TXT`: https://android.googlesource.com/platform/bionic/+/android-11.0.0_r1/libc/SECCOMP_BLACKLIST_APP.TXT

### 2.2 SIGSYS Handler

**VERIFIED** — installed by `libkr64.so` (the runtime, loaded as DT_NEEDED of the loader or via LD_PRELOAD).

**Installation (at `0x116120` in libkr64.so):**

```asm
; Build struct sigaction on stack:
;   sa_sigaction = 0x115f04  (the SIGSYS handler)
;   sa_flags = SA_SIGINFO (4)
;   sa_mask = 0

0x116134:  adrp x9, 0x115000; add x9, x9, #0xf04   ; x9 = 0x115f04 (handler addr)
0x11613c:  mov  w10, #4                          ; SA_SIGINFO
0x116140:  mov  w0, #0x1f                        ; SIGSYS (signal 31)
0x116144:  mov  x2, xzr                          ; old_act = NULL
0x116154:  bl   0x115ca4                         ; syscall(__NR_rt_sigaction=134, ...)
```

**SIGSYS handler (at `0x115f04`):**

```asm
0x115f2c:  mov  x19, x2        ; x19 = ucontext_t*
0x115f40:  mov  x20, x1        ; x20 = siginfo_t*
0x115f3c:  ldr  x0, [x19, #0xb8]   ; x0 = ucontext->uc_mcontext.regs[0] (return value reg)
0x115f8c:  ldrsw x1, [x20, #0x18]  ; x1 = siginfo->si_syscall (trapped syscall nr)
; ... logging ...
0x115fd0:  bl   0x1131f8       ; call syscall emulation dispatcher
0x115fd4:  str  x0, [x19, #0xb8]   ; write emulated return value to ucontext->regs[0]
```

**This EXACTLY matches the Chromium `sandbox/linux/seccomp-bpf/trap.cc` `PutValueInUcontext` pattern** — the handler reads `si_syscall` from `siginfo_t`, dispatches to an emulator, and writes the result to `ucontext_t->uc_mcontext.regs[0]`.

**Sources:**
- Chromium trap.cc: https://chromium.googlesource.com/chromium/src/+/lkgr/sandbox/linux/seccomp-bpf/trap.cc
- Chromium seccomp_macros.h (aarch64): `SECCOMP_RESULT(ctx) = SECCOMP_REG(ctx, 0)` = `regs[0]`

### 2.3 Shadowhook Inline Hooks (SECONDARY mechanism)

**VERIFIED** — shadowhook v1.0.8 is embedded in `libkr64.so`.

However, shadowhook is used for ONLY 5 libc functions:

| Hooked function | Purpose |
|----------------|---------|
| `open` | File path redirection (complement to openat SIGSYS handler) |
| `connect` | SOCKS5 proxy redirection |
| `socket` | Socket namespace isolation |
| `dlopen` | Intercept guest's dynamic library loading |
| `dlsym` | Symbol resolution interception |

**`mount`, `chroot`, `pivot_root`, `openat`, `mknodat`, `getpid`, `fork`, etc. are NOT hooked via shadowhook** — they are handled exclusively via the seccomp/SIGSYS path.

**Source:** ByteDance shadowhook: https://github.com/bytedance/android-inline-hook

---

## 3. Syscall Interception Details

### 3.1 The Emulation Dispatcher

**VERIFIED** — jump table at `.rodata:0x15f0a8` in libkr64.so.

```asm
; Dispatcher at 0x1131f8:
0x113260:  ldr  x20, [x19, #0x58]      ; x20 = syscall number from struct
0x113264:  cmp  x20, #0x176f(5999)     ; check upper bound
0x113268:  b.hi 0x113580               ; >5999 → special handler
0x113270:  sub  x10, x20, #5           ; index = nr - 5
0x113278:  cmp  x10, #0x114(276)       ; valid range: 5..281
0x11327c:  b.hi default
0x113288:  ldrsw x10, [x11, x10, lsl #2]  ; load 32-bit offset from jump table
0x11328c:  add  x10, x10, x11          ; target = table_base + offset
           br   x10                    ; jump to handler
```

- **Jump table base:** `0x15f0a8`
- **Index:** `syscall_nr - 5` (syscall 5 = index 0)
- **Entry size:** 4 bytes (signed offset from base)
- **Entries:** 277 (syscalls 5–281)

### 3.2 Syscall Handling Categories

**VERIFIED** — from disassembly of each handler.

#### Category A: Fully Emulated (return fake result, no real syscall)

| Syscall | Handler | Emulation | Evidence |
|---------|---------|-----------|----------|
| `mount(40)` | `0x113380` → `0x13d1f8` → `0x8618` | Virtual mount table; `/dev`, `/mnt`, `/storage` special-cased; always returns 0 | `mount_mgr:` strings, handler always returns 0 |
| `chroot(51)` | `0x113e68` → `0x11c928` | **NO-OP**: `mov w0, wzr; ret` — always returns 0, does nothing | Disassembly of `0x11c928` is 2 instructions |
| `mknodat(33)` | `0x1139e4` → `0x11d598` | Creates a regular file + AF_UNIX socket instead of device node | `openat(O_CREAT)` instead of `mknodat` |
| `openat(56)` | `0x114124` → `0x118320` | Path translation: prepend rootfs prefix; `/proc/` special handling | `strncmp("/proc/")` at `0x119080` |
| `newfstatat(79)` | `0x114248` → `0x11f644` | Path translation (same as openat) | Same path translation function |
| `mkdirat(34)` | `0x113a54` | Path translation | — |
| `unlinkat(35)` | `0x113b0c` | Path translation | — |
| `clone(212)` | `0x11533c` → `0x1358b8` | Clone emulation (tracks child PIDs) | INFERRED from function structure |
| `execve(221)` | `0x1153c8` → `0x1230f8` | Execve emulation (translates path, sets up guest env) | INFERRED from function structure |
| `setuid(146)`, `setgid(144)`, `setgroups(159)`, `setresuid(147)`, `setresgid(149)` | various | Return 0 (fake success, don't actually change IDs) | — |
| `rt_sigaction(134)` | `0x114650` | If signal==SIGSYS: return 0 (prevent guest from overriding handler). Otherwise: passthrough. | `cmp w10, #0x1f` (SIGSYS=31) |

#### Category B: Passthrough (re-execute real syscall via `syscall@plt`)

**VERIFIED** — DEFAULT handler at `0x114664`.

Syscalls using passthrough: `pivot_root(41)`, `getpid(171)`, `getppid(172)`, `gettid(177)`, `wait4(260)`, `close(57)`, `read(63)`, `write(64)`, and most others.

```asm
; DEFAULT handler at 0x114664:
; Load all 7 syscall args from struct
; mov x0, syscall_nr
; bl syscall@plt  →  re-execute the real syscall
```

**`getpid()` returns the REAL host PID** — there is NO PID number virtualization at the syscall level.

#### Category C: Path Translation (modify path arg, then passthrough)

`openat`, `newfstatat`, `mkdirat`, `unlinkat`, `symlinkat`, `linkat`, `renameat`, `faccessat`, `fchmodat`, `fchownat`, `readlinkat`, `truncate`, `chdir`, `inotify_add_watch`, and xattr variants.

---

## 4. Filesystem/Path Virtualization

### 4.1 Rootfs Prefix

**VERIFIED** — rootfs path is `/data/data/com.clone.android.dual.space/vm/vm%d/fs`.

| String | .data Address | XOR Key |
|--------|--------------|---------|
| `/data/data/com.clone.android.dual.space` | `0x170fd0` | `0x15` |
| `/data/data/com.clone.android.dual.space/vm/vm%d/fs` | `0x1704a0` | `0xb0` |
| `/data/data/com.clone.android.dual.space/vm/vm%d%s` | `0x1725c0` | `0x99` |

**Twoyi equivalent:** `/data/data/io.twoyi/rootfs` (set via `TWOYI_ROOTFS` env var).

### 4.2 /proc Virtualization

**VERIFIED** — extensive /proc path translation.

The `openat` handler at `0x118320` calls path translator at `0x119080`:
1. `strncmp(path, "/proc/", 6)` — check if path starts with `/proc/`
2. If yes: special `/proc` handling
3. If no: prepend rootfs prefix

**Per-VM /proc files (all XOR'd in .data):**

| Path | Purpose |
|------|---------|
| `/proc/self/maps` | Virtual memory maps |
| `/proc/self/exe` | Executable path |
| `/proc/self/status` | Process status |
| `/proc/self/mounts` | Mount info |
| `/proc/self/fd/%d` | File descriptors |
| `/proc/%d/cmdline` | Per-PID cmdline |
| `/proc/%d/status` | Per-PID status |
| `/proc/%d/maps` | Per-PID maps |
| `/proc/%d/mounts` | Per-PID mounts |
| `/proc/1` | Init process (special case) |
| `/proc/cmdline` | Global cmdline |
| `/proc/version` | Kernel version |
| `/proc/mounts` | Mount info |
| `%s/proc/mounts_%d_%d` | Per-VM mount file |
| `%s/proc/maps_%d_%d` | Per-VM maps file |
| `%s/proc/status_%d_%d` | Per-VM status file |

### 4.3 /dev Virtualization

**VERIFIED** — `mknodat` emulation creates regular files + AF_UNIX sockets instead of device nodes.

The `mknodat(33)` handler at `0x11d598`:
1. Translate path (prepend rootfs prefix)
2. Check if mode is `S_IFCHR` or `S_IFBLK` (device node)
3. If device node: create a **regular file** via `openat(AT_FDCWD, path, O_RDWR|O_CREAT, 0666)` instead
4. Set up AF_UNIX socket associated with the file

**Virtual /dev devices (all XOR'd in .data):**

| Path | Purpose |
|------|---------|
| `/dev/qemu_pipe` | Goldfish GL command transport |
| `/dev/input/touch` | Touch input |
| `/dev/socket/logdw` | Log daemon write |
| `/dev/socket/logdr` | Log daemon read |
| `/dev/socket/process_pid` | PID query socket |
| `/dev/__properties__` | Android property area |
| `/dev/__kmsg__` | Kernel messages |
| `/dev/__kmsg2__` | Kernel messages v2 |
| `/dev/__krlog__` | KR log |
| `/dev/vmproc` | Virtual process info |
| `/dev/vmproc/%d` | Per-PID process info |
| `/dev/ashmem` | Ashmem device |
| `/dev/ashmemsim` | Simulated ashmem |
| `/dev/tmpfs` | Tmpfs device |
| `/dev/.magisk` | Magisk detection (fake) |
| `/dev/.busybox` | Busybox detection (fake) |
| `/dev/.coldboot_done` | Coldboot flag |
| `/dev/gb2` | Graphics buffer (gralloc) |

### 4.4 /sys Virtualization

**INFERRED** — `/sys` is likely handled via path translation (same as /proc), but specific handlers were not traced in detail. The `mount_mgr` skips `/sys` as special.

### 4.5 Mount Manager

**VERIFIED** — function at `0x8618` in libkr64.so.

The mount manager maintains a **virtual mount table** and handles:
- `mount(source, target, fstype, flags, data)` — adds to virtual table, returns 0
- `umount2(target, flags)` — removes from virtual table, returns 0
- Special paths: `/dev`, `/mnt`, `/storage` are skipped (no-op)
- Bind mount loop detection
- Propagation type tracking (`ms.bind`, `ms.unbindable`, `ms.remount`)

**Key strings (all XOR'd):**
- `mount_mgr: /dev is special, skip`
- `mount_mgr: /mnt is special, skip`
- `mount_mgr: /storage is special, skip`
- `mount_mgr: %s -> %s -> %s`
- `mount_mgr: mount arg source %s is bad`
- `mount_mgr: unsupported filesystemtype %s`
- `mount_mgr: bind loop detected %s`

---

## 5. PID Virtualization

### 5.1 getpid / getppid / gettid

**VERIFIED** — these use the DEFAULT handler (passthrough), returning the REAL host PID.

There is **NO PID number virtualization** at the syscall level. `getpid()` returns the real host PID of the guest process.

### 5.2 /proc-based PID Virtualization

**STRONGLY SUPPORTED** — PID virtualization is done at the `/proc` filesystem level.

- The guest's init PID is stored in:
  - `VMINIT_PID` env var
  - `vminit.pid` property
  - `HVMINIT_PID=%d` format string
- When the guest opens `/proc/<pid>/*`, the handler:
  1. Extracts the PID from the path
  2. Translates it to the VM's virtual PID space
  3. Redirects to per-VM files (`/proc/maps_%d_%d`, `/proc/status_%d_%d`, etc.)
- `/proc/1` is specially handled (redirects to the VM's init process info)
- `/dev/vmproc/%d` provides per-PID virtual process data

### 5.3 clone / fork

**INFERRED** — `clone(212)` has a dedicated handler at `0x1358b8` that likely tracks child PIDs for the virtual PID space. The handler was not fully traced, but the existence of a dedicated (non-passthrough) handler indicates clone is emulated.

### 5.4 execve

**INFERRED** — `execve(221)` has a dedicated handler at `0x1230f8` that likely:
1. Translates the executable path (prepend rootfs prefix)
2. Sets up the guest environment (TYLOADER, TWOYI_ROOTFS, etc.)
3. Calls the real `execve` with translated args

The handler was not fully traced, but the existence of a dedicated handler indicates execve is emulated.

---

## 6. Child Process / execve Inheritance

### 6.1 How Child Processes Inherit Virtualization

**STRONGLY SUPPORTED** — the seccomp filter is inherited across `fork()` and `clone()`.

The seccomp BPF filter is installed once (by the loader before guest `main()`). Linux kernel semantics guarantee that:
- `fork()` — child inherits the parent's seccomp filter
- `clone()` — child inherits the parent's seccomp filter (unless `CLONE_NEWPID` + new seccomp, which untrusted_app cannot do)
- `execve()` — seccomp filter persists across exec (this is the KEY property that makes the whole scheme work)

So when the guest's init forks zygote, and zygote forks apps, ALL descendant processes inherit the seccomp filter. Every `mount()`/`chroot()`/`openat()` etc. in ANY descendant process is trapped and emulated.

**Source:** Kernel seccomp docs — "Filters installed by a process are inherited by all of its descendant processes."

### 6.2 How execve Handles Guest ELFs

**INFERRED** — the `execve(221)` handler at `0x1230f8` translates the path and calls real `execve`.

When the guest calls `execve("/system/bin/sh", ...)`:
1. The SIGSYS handler traps `execve`
2. The handler translates the path: `/system/bin/sh` → `{rootfs}/system/bin/sh`
3. The handler calls the real `execve("{rootfs}/system/bin/sh", ...)`
4. The kernel loads `{rootfs}/system/bin/sh`
5. The kernel reads its `PT_INTERP` → `./loader64` (relative to cwd)
6. The kernel exec's `{cwd}/loader64` (the custom linker)
7. The loader runs, re-installs hooks (or they persist from seccomp), and jumps to `sh`'s `main()`

**The seccomp filter persists across execve**, so the new process (`sh`) is still under the seccomp filter. The SIGSYS handler also persists (it's a process-level setting that survives execve).

---

## 7. VM → Twoyi Implementation Map

### Confidence Levels
- **VERIFIED**: Confirmed by binary disassembly + primary source cross-reference
- **STRONGLY SUPPORTED**: Confirmed by binary disassembly, but some details inferred
- **INFERRED**: Not fully traced, but strongly suggested by available evidence
- **UNKNOWN**: Could not determine from available evidence

| VM Mechanism | Evidence | Required Semantics | Twoyi Equivalent | Confidence |
|--------------|----------|---------------------|------------------|------------|
| **Custom ELF interpreter** (libkrloader64.so) | `_start` pattern matches AOSP `begin.S`; `getauxval(AT_BASE/AT_ENTRY)`; build path `EXECUTABLES/krloader_intermediates`; bionic libc strings | Load guest ELF, install seccomp filter before main() | Build `libloader.so` from AOSP `bionic/linker/` source; guest init has `PT_INTERP=./loader64` | **VERIFIED** |
| **Seccomp BPF filter** | `prctl(PR_SET_NO_NEW_PRIVS)` + `prctl(PR_SET_SECCOMP, &fprog)` at `0x3384`; BPF constants `AUDIT_ARCH_AARCH64` + `SECCOMP_RET_ALLOW` | Trap blacklisted syscalls (mount, chroot, mknod, etc.) with `SECCOMP_RET_TRAP` | Install same BPF filter in libloader.so before guest main() | **VERIFIED** |
| **SIGSYS handler** | `rt_sigaction(SIGSYS=31, handler=0x115f04, SA_SIGINFO)` at `0x116120`; handler reads `si_syscall`, writes return to `ucontext->regs[0]` | Receive trapped syscalls, dispatch to emulators, forge return value | Implement SIGSYS handler in libloader.so (Chromium trap.cc pattern) | **VERIFIED** |
| **Syscall emulation dispatcher** | Jump table at `0x15f0a8`, 277 entries (syscalls 5-281); dispatcher at `0x1131f8` | Route each trapped syscall to its emulator | Implement syscall dispatch table in libloader.so | **VERIFIED** |
| **mount emulation** | Handler at `0x113380` → `0x8618` (mount_mgr); virtual mount table; `/dev`/`/mnt`/`/storage` special; always returns 0 | Maintain virtual mount table; fake success | Implement mount_mgr in Twoyi's runtime | **VERIFIED** |
| **chroot emulation** | Handler at `0x113e68` → `0x11c928` = `mov w0, wzr; ret` (NO-OP) | Return 0 without doing anything; chroot effect via path translation | Implement chroot as no-op in SIGSYS handler | **VERIFIED** |
| **openat path translation** | Handler at `0x114124` → `0x118320` → `0x119080`; `strncmp("/proc/")`; rootfs prefix strings | Prepend rootfs prefix; special /proc handling | Implement path translator with TWOYI_ROOTFS prefix | **VERIFIED** |
| **mknodat emulation** | Handler at `0x1139e4` → `0x11d598`; creates regular file + AF_UNIX socket instead of device node | Convert device node creation to file+socket | Implement mknodat emulator (kr64 already creates AF_UNIX sockets) | **VERIFIED** |
| **/proc virtualization** | Extensive /proc path strings; `/proc/self/maps`, `/proc/%d/cmdline`, per-VM files | Translate /proc paths to per-VM virtual files | Implement /proc virtualization layer | **VERIFIED** |
| **/dev virtualization** | `mknodat` creates files+sockets; extensive /dev path strings; `/dev/qemu_pipe`, `/dev/__properties__`, etc. | Create AF_UNIX sockets at /dev paths | kr64 already does this (create_all_devices) | **VERIFIED** |
| **PID virtualization** | getpid uses DEFAULT (passthrough, returns real PID); /proc paths translated per-VM; VMINIT_PID env var | Virtualize /proc, not getpid itself | Implement /proc-based PID virtualization | **STRONGLY SUPPORTED** |
| **clone emulation** | Dedicated handler at `0x11533c` → `0x1358b8` (not passthrough) | Track child PIDs for virtual PID space | Implement clone emulator | **INFERRED** |
| **execve emulation** | Dedicated handler at `0x1153c8` → `0x1230f8` (not passthrough) | Translate path, set up guest env, call real execve | Implement execve emulator | **INFERRED** |
| **shadowhook inline hooks** | shadowhook v1.0.8 strings; only 5 functions hooked (open, connect, socket, dlopen, dlsym) | Hook libc functions that bypass seccomp (e.g., open vs openat) | Integrate shadowhook for open/connect/socket/dlopen | **VERIFIED** |
| **rt_sigaction guard** | Handler at `0x114650`: if signal==SIGSYS, return 0 (prevent guest override) | Prevent guest from replacing SIGSYS handler | Add SIGSYS guard to rt_sigaction handler | **VERIFIED** |
| **Property virtualization** | `/dev/__properties__` string; `scrubBadGuestPropertyArea` in Nogitsune | Virtualize Android property service | Implement property area virtualization | **STRONGLY SUPPORTED** |
| **Patched goldfish ROM** | Nogitsune sets `ro.hardware=goldfish`; Twoyi uses goldfish rootfs | Guest ROM expects emulator-style devices | Use goldfish rootfs (already done in Twoyi) | **VERIFIED** |
| **Custom syscalls 6000/6001** | BPF filter traps syscalls 6000 and 6001 | VM-internal hypercalls for guest→host communication | UNKNOWN if Twoyi needs this | **INFERRED** |
| **Netlink emulation** | `netlink_server`, `netlink_client/netdevice_%d_%d` strings | Emulate netlink for network stack | UNKNOWN if Twoyi needs this | **INFERRED** |
| **SOCKS5 proxy** | `socks5 server v6`, `__connect` hook strings | Redirect TCP through SOCKS5 proxy | Optional feature, not needed for boot | **VERIFIED** (exists in VM, not needed for Twoyi boot) |

---

## 8. Critical Findings (Corrections to Previous Assumptions)

### 8.1 The seccomp action is TRAP, not KILL

**CRITICAL CORRECTION** — The AOSP `bionic/libc/seccomp/seccomp_policy.cpp` (Android 11) shows:

```cpp
inline void Disallow(filter& f) {
    f.push_back(BPF_STMT(BPF_RET|BPF_K, SECCOMP_RET_TRAP));  // NOT KILL
}
```

ALL blacklisted syscalls (`mount`, `chroot`, `mknod`, `unshare`, `setgroups`, etc.) use `SECCOMP_RET_TRAP`, NOT `SECCOMP_RET_KILL_PROCESS`.

**This means the SIGSYS handler CAN intercept and emulate these syscalls.** The previous belief that "inline hooks are the ONLY way to intercept mount/chroot" was WRONG.

**Source:** https://android.googlesource.com/platform/bionic/+/android-11.0.0_r1/libc/seccomp/seccomp_policy.cpp

### 8.2 LD_PRELOAD CAN work for apps

**CORRECTION** — The AOSP linker source shows:

```cpp
if (!getauxval(AT_SECURE)) {
    ldpreload_env = getenv("LD_PRELOAD");
}
```

A child of a normal untrusted_app **does NOT have AT_SECURE set** (real uid == effective uid, no file capabilities). So `LD_PRELOAD` IS honored for app processes.

However, PLT interposition (LD_PRELOAD) only catches calls through libc wrappers. Direct `svc` instructions bypass it. VM uses seccomp/SIGSYS as the primary mechanism (catches ALL paths), with shadowhook inline hooks as a secondary mechanism for specific functions.

### 8.3 init's CHECKCALL collects errors, doesn't abort immediately

**KEY FINDING** — AOSP `system/core/init/first_stage_init.cpp`:

```cpp
#define CHECKCALL(x) \
    if ((x) != 0) errors.emplace_back(#x " failed", errno);

// ... all mount/mkdir/mknod calls ...

InitKernelLogging(argv);  // needs /dev/kmsg

if (!errors.empty()) {
    LOG(FATAL) << "Init encountered errors starting first stage, aborting";
}
```

Init collects all errors and only aborts AFTER `InitKernelLogging` runs. If the SIGSYS handler fakes success (return 0) for all mount/mknod calls, the `errors` vector stays empty, and init proceeds past FirstStageMain.

**This is the entire legal basis for VM's approach.**

---

## 9. What Twoyi Needs to Implement

Based on this research, Twoyi needs:

### 9.1 Critical (REQUIRED for boot)

1. **Custom ELF interpreter** (`libloader.so`)
   - Build from AOSP `bionic/linker/` source
   - Guest init must have `PT_INTERP=./loader64`
   - Install seccomp BPF filter before guest main()

2. **Seccomp BPF filter**
   - Trap: mount, umount2, chroot, mknodat, unshare, setuid/gid/groups, openat, newfstatat, mkdirat, unlinkat, clone, execve, etc.
   - Use `SECCOMP_RET_TRAP` (not KILL)

3. **SIGSYS handler**
   - Install via `rt_sigaction(SIGSYS, handler, SA_SIGINFO)`
   - Read `si_syscall` from `siginfo_t`
   - Write return value to `ucontext->uc_mcontext.regs[0]`
   - Guard: prevent guest from overriding SIGSYS handler (intercept `rt_sigaction(SIGSYS)`)

4. **Syscall emulation dispatcher**
   - Jump table indexed by syscall number
   - Per-syscall handlers (mount, chroot, openat, mknodat, etc.)

5. **Path translation**
   - Prepend `{TWOYI_ROOTFS}` to absolute paths
   - Special handling for `/proc/`, `/sys/`, `/dev/`
   - Virtual `/proc/cmdline`, `/proc/self/maps`, etc.

6. **Virtual /dev devices**
   - mknodat emulation: create AF_UNIX sockets instead of device nodes
   - kr64 already does this — just needs to be in the SIGSYS handler

7. **Patched goldfish ROM**
   - `ro.hardware=goldfish`
   - Skip `first_stage_mount`
   - Permissive SELinux

### 9.2 Important (for full functionality)

8. **clone/execve emulation**
   - Track child PIDs
   - Translate execve paths

9. **/proc virtualization**
   - Per-VM /proc files
   - PID translation in /proc paths

10. **Property virtualization**
    - `/dev/__properties__` per-VM area

### 9.3 Optional (VM has these, Twoyi may not need)

11. shadowhook inline hooks (open, connect, socket, dlopen, dlsym)
12. SOCKS5 proxy support
13. Netlink emulation
14. Samsung GameSDK integration
15. Custom syscalls 6000/6001 (hypercalls)

---

## 10. Sources

### AOSP (primary)
- `bionic/linker/linker_main.cpp`: https://android.googlesource.com/platform/bionic/+/android-11.0.0_r1/linker/linker_main.cpp
- `bionic/libc/seccomp/seccomp_policy.cpp`: https://android.googlesource.com/platform/bionic/+/android-11.0.0_r1/libc/seccomp/seccomp_policy.cpp
- `bionic/libc/SECCOMP_BLACKLIST_APP.TXT`: https://android.googlesource.com/platform/bionic/+/android-11.0.0_r1/libc/SECCOMP_BLACKLIST_APP.TXT
- `system/core/init/first_stage_init.cpp`: https://android.googlesource.com/platform/system/core/+/android-11.0.0_r1/init/first_stage_init.cpp

### Kernel / seccomp
- https://www.kernel.org/doc/html/v5.0/userspace-api/seccomp_filter.html
- https://man7.org/linux/man-pages/man2/seccomp.2.html

### Reference implementations
- Chromium `sandbox/linux/seccomp-bpf/trap.cc`: https://chromium.googlesource.com/chromium/src/+/lkgr/sandbox/linux/seccomp-bpf/trap.cc
- Chromium `sandbox/linux/bpf_dsl/seccomp_macros.h` (aarch64 register macros)
- ByteDance shadowhook: https://github.com/bytedance/android-inline-hook

### Project sources
- Twoyi: https://github.com/twoyi/twoyi (app/rs/src/lib.rs, RomManager.java)
- Nogitsune: https://github.com/cyanmint/Nogitsune (BootHelper.kt)

### Binary analysis
- `libkrloader64.so`: AArch64 ELF, 217KB, entry 0x2cd0, rebuilt AOSP bionic linker
- `libkr64.so`: AArch64 ELF, 1.5MB, OLLVM-obfuscated, 277-entry syscall emulation jump table
- `libkr64.11.so`: AArch64 ELF, 2MB, Android 11 variant with additional features
