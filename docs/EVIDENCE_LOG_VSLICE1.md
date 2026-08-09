# Evidence Log: Vertical Slice 1

## Source-Validation Audit

This document records the evidence backing each technical claim in the
vertical slice implementation. Every claim is classified as:

- **SOURCE-VERIFIED**: Verified from actual system headers/binaries on this machine
- **PRIMARY-SOURCE-VERIFIED**: Verified from official AOSP/kernel source (web-fetched)
- **BINARY-VERIFIED**: Verified from VM binary disassembly
- **TEST-VERIFIED**: Verified by observable test behavior
- **INFERRED**: Plausible but not directly verified
- **UNKNOWN**: Not verified

---

## 1. Seccomp BPF Constants

| Constant | Value | Evidence | Classification |
|----------|-------|----------|----------------|
| `SECCOMP_RET_TRAP` | `0x00030000U` | `/usr/include/linux/seccomp.h` | SOURCE-VERIFIED |
| `SECCOMP_RET_ALLOW` | `0x7fff0000U` | `/usr/include/linux/seccomp.h` | SOURCE-VERIFIED |
| `SECCOMP_RET_KILL_PROCESS` | `0x80000000U` | `/usr/include/linux/seccomp.h` | SOURCE-VERIFIED |
| `SECCOMP_RET_DATA` | `0x0000ffffU` | `/usr/include/linux/seccomp.h` | SOURCE-VERIFIED |
| `SECCOMP_SET_MODE_FILTER` | `1` | `/usr/include/linux/seccomp.h` | SOURCE-VERIFIED |
| `BPF_LD` | `0x00` | `/usr/include/linux/bpf_common.h` | SOURCE-VERIFIED |
| `BPF_W` | `0x00` | `/usr/include/linux/bpf_common.h` | SOURCE-VERIFIED |
| `BPF_ABS` | `0x20` | `/usr/include/linux/bpf_common.h` | SOURCE-VERIFIED |
| `BPF_JMP` | `0x05` | `/usr/include/linux/bpf_common.h` | SOURCE-VERIFIED |
| `BPF_JEQ` | `0x10` | `/usr/include/linux/bpf_common.h` | SOURCE-VERIFIED |
| `BPF_RET` | `0x06` | `/usr/include/linux/bpf_common.h` | SOURCE-VERIFIED |
| `BPF_K` | `0x00` | `/usr/include/linux/bpf_common.h` | SOURCE-VERIFIED |

## 2. seccomp_data Structure

| Field | Offset | Evidence | Classification |
|-------|--------|----------|----------------|
| `nr` (syscall number) | 0 | `/usr/include/linux/seccomp.h` `struct seccomp_data` | SOURCE-VERIFIED |
| `arch` (AUDIT_ARCH) | 4 | `/usr/include/linux/seccomp.h` `struct seccomp_data` | SOURCE-VERIFIED |
| `instruction_pointer` | 8 | `/usr/include/linux/seccomp.h` `struct seccomp_data` | SOURCE-VERIFIED |
| `args[0..5]` | 16..56 | `/usr/include/linux/seccomp.h` `struct seccomp_data` | SOURCE-VERIFIED |

## 3. AUDIT_ARCH Values

| Constant | Value | Evidence | Classification |
|----------|-------|----------|----------------|
| `AUDIT_ARCH_X86_64` | `0xC000003E` | `/usr/include/linux/audit.h` + `/usr/include/linux/elf-em.h` | SOURCE-VERIFIED |
| `AUDIT_ARCH_AARCH64` | `0xC00000B7` | `/usr/include/linux/audit.h` + `/usr/include/linux/elf-em.h` | SOURCE-VERIFIED |

Computation verified:
- `EM_X86_64 = 62` (from `/usr/include/linux/elf-em.h`)
- `__AUDIT_ARCH_64BIT = 0x80000000`
- `__AUDIT_ARCH_LE = 0x40000000`
- `62 | 0x80000000 | 0x40000000 = 0xC000003E` ✓

## 4. Syscall Numbers (x86_64)

| Syscall | Number | Evidence | Classification |
|---------|--------|----------|----------------|
| `mount` | 165 | `/usr/include/x86_64-linux-gnu/asm/unistd_64.h` | SOURCE-VERIFIED |
| `umount2` | 166 | `/usr/include/x86_64-linux-gnu/asm/unistd_64.h` | SOURCE-VERIFIED |
| `chroot` | 161 | `/usr/include/x86_64-linux-gnu/asm/unistd_64.h` | SOURCE-VERIFIED |
| `openat` | 257 | `/usr/include/x86_64-linux-gnu/asm/unistd_64.h` | SOURCE-VERIFIED |
| `mknodat` | 259 | `/usr/include/x86_64-linux-gnu/asm/unistd_64.h` | SOURCE-VERIFIED |
| `getpid` | 39 | `/usr/include/x86_64-linux-gnu/asm/unistd_64.h` | SOURCE-VERIFIED |

## 5. Syscall Numbers (arm64-v8a)

| Syscall | Number | Evidence | Classification |
|---------|--------|----------|----------------|
| `mount` | 40 | AOSP `bionic/libc/kernel/uapi/asm-generic/unistd.h` | PRIMARY-SOURCE-VERIFIED |
| `umount2` | 39 | AOSP `bionic/libc/kernel/uapi/asm-generic/unistd.h` | PRIMARY-SOURCE-VERIFIED |
| `chroot` | 51 | AOSP `bionic/libc/kernel/uapi/asm-generic/unistd.h` | PRIMARY-SOURCE-VERIFIED |
| `openat` | 56 | AOSP `bionic/libc/kernel/uapi/asm-generic/unistd.h` | PRIMARY-SOURCE-VERIFIED |
| `mknodat` | 33 | AOSP `bionic/libc/kernel/uapi/asm-generic/unistd.h` | PRIMARY-SOURCE-VERIFIED |
| `getpid` | 172 | AOSP `bionic/libc/kernel/uapi/asm-generic/unistd.h` | PRIMARY-SOURCE-VERIFIED |

**NOTE:** arm64 syscall numbers are NOT verified from a local system header
(no arm64 cross-compiler headers installed). They are from AOSP source.
Needs verification on an actual arm64 system or via cross-compiler headers.

## 6. ucontext_t Register Access (x86_64)

| Register | Access | Index | Evidence | Classification |
|----------|--------|-------|----------|----------------|
| RAX (return value) | `uc_mcontext.gregs[REG_RAX]` | 13 | `/usr/include/x86_64-linux-gnu/sys/ucontext.h` | SOURCE-VERIFIED |
| RDI (arg1) | `uc_mcontext.gregs[REG_RDI]` | 8 | `/usr/include/x86_64-linux-gnu/sys/ucontext.h` | SOURCE-VERIFIED |
| RSI (arg2) | `uc_mcontext.gregs[REG_RSI]` | 9 | `/usr/include/x86_64-linux-gnu/sys/ucontext.h` | SOURCE-VERIFIED |
| RDX (arg3) | `uc_mcontext.gregs[REG_RDX]` | 12 | `/usr/include/x86_64-linux-gnu/sys/ucontext.h` | SOURCE-VERIFIED |
| R10 (arg4) | `uc_mcontext.gregs[REG_R10]` | 2 | `/usr/include/x86_64-linux-gnu/sys/ucontext.h` | SOURCE-VERIFIED |
| R8 (arg5) | `uc_mcontext.gregs[REG_R8]` | 0 | `/usr/include/x86_64-linux-gnu/sys/ucontext.h` | SOURCE-VERIFIED |
| R9 (arg6) | `uc_mcontext.gregs[REG_R9]` | 1 | `/usr/include/x86_64-linux-gnu/sys/ucontext.h` | SOURCE-VERIFIED |
| RIP (instruction ptr) | `uc_mcontext.gregs[REG_RIP]` | 16 | `/usr/include/x86_64-linux-gnu/sys/ucontext.h` | SOURCE-VERIFIED |

**Key fact verified:** On x86_64, the syscall number AND return value are
BOTH in RAX (gregs[13]). After `syscall_rollback`, RAX = original syscall
number. The handler overwrites RAX with the return value.
Source: Linux kernel `kernel/seccomp.c` `syscall_rollback()`.

## 7. ucontext_t Register Access (arm64-v8a)

| Register | Access | Evidence | Classification |
|----------|--------|----------|----------------|
| x0 (return value / arg1) | `uc_mcontext.regs[0]` | AOSP bionic `libc/include/sys/ucontext.h` | PRIMARY-SOURCE-VERIFIED |
| x8 (syscall number) | `uc_mcontext.regs[8]` | AOSP bionic `libc/include/sys/ucontext.h` | PRIMARY-SOURCE-VERIFIED |
| x1-x5 (args 2-6) | `uc_mcontext.regs[1..5]` | AOSP bionic `libc/include/sys/ucontext.h` | PRIMARY-SOURCE-VERIFIED |
| PC | `uc_mcontext.pc` | AOSP bionic `libc/include/sys/ucontext.h` | PRIMARY-SOURCE-VERIFIED |

**Key fact verified:** On arm64, syscall number (x8) and return value (x0)
are DIFFERENT registers. This is the critical arch difference.
Source: Linux kernel `arch/arm64/include/asm/syscall.h` `syscall_rollback()`
restores `orig_x0 → regs[0]`, but x8 (syscall nr) is unchanged.

**NOTE:** arm64 ucontext layout is NOT verified from a local system header.
Needs verification on an actual arm64 system.

## 8. siginfo_t for SIGSYS

| Field | Evidence | Classification |
|-------|----------|----------------|
| `si_syscall` (syscall number) | `/usr/include/x86_64-linux-gnu/bits/types/siginfo_t.h` | SOURCE-VERIFIED |
| `si_arch` (AUDIT_ARCH) | `/usr/include/x86_64-linux-gnu/bits/types/siginfo_t.h` | SOURCE-VERIFIED |
| `si_call_addr` (instruction ptr) | `/usr/include/x86_64-linux-gnu/bits/types/siginfo_t.h` | SOURCE-VERIFIED |
| `si_code = SYS_SECCOMP (1)` for seccomp | `/usr/include/asm-generic/siginfo.h` | SOURCE-VERIFIED |
| `si_errno = SECCOMP_RET_DATA` | Linux kernel `kernel/signal.c` `force_sig_seccomp()` | PRIMARY-SOURCE-VERIFIED |

## 9. SIGSYS Handler Flags

| Flag | Value | Evidence | Classification |
|------|-------|----------|----------------|
| `SA_SIGINFO` | `0x00000004` | `/usr/include/x86_64-linux-gnu/bits/sigaction.h` | SOURCE-VERIFIED |
| `SA_NODEFER` | `0x40000000` | `/usr/include/x86_64-linux-gnu/bits/sigaction.h` | SOURCE-VERIFIED |

**SA_NODEFER rationale:** Chromium `sandbox/linux/seccomp-bpf/trap.cc` uses
`SA_NODEFER` to allow nested SIGSYS delivery (in case the handler itself
triggers a trapped syscall). This is a best practice, not a kernel requirement.

## 10. Virtual Mount Table Semantics

| Behavior | Evidence | Classification |
|----------|----------|----------------|
| VM maintains a virtual mount table | VM `libkr64.so` strings: `mount_mgr: %s -> %s -> %s` | BINARY-VERIFIED |
| VM special-cases `/dev`, `/mnt`, `/storage` | VM strings: `mount_mgr: /dev is special, skip`, etc. | BINARY-VERIFIED |
| VM returns 0 for mount (success) | VM `mount_mgr` always returns 0 (handler at `0x113380` → `0x13d1f8`) | BINARY-VERIFIED |
| Duplicate mount returns EBUSY | Linux `mount(2) man page` — NOT verified from VM binary | INFERRED |
| MS_REMOUNT updates existing entry | Linux `mount(2) man page` — NOT verified from VM binary | INFERRED |

**GAP:** I implemented EBUSY for duplicate mounts based on Linux man page
semantics, but I did NOT verify that VM actually returns EBUSY. VM might
return 0 for duplicate mounts (since it's a virtual table). The test
passes because MY implementation returns EBUSY, but this may not match
VM's actual behavior.

## 11. chroot Emulation

| Claim | Evidence | Classification |
|-------|----------|----------------|
| VM's chroot handler returns 0 | VM `libkr64.so` at `0x11c928`–`0x11c984`: complex OLLVM code ending in `mov w0, wzr; ret` | BINARY-VERIFIED |
| VM's chroot is a simple no-op | **WRONG** — the sub-agent claimed 2 instructions, but actual disassembly shows ~20 instructions of OLLVM-flattened checks before `mov w0, wzr; ret` | CORRECTED |
| My implementation (return 0) matches VM | Observable behavior matches (both return 0). Internal logic differs (VM does checks, I don't). | INFERRED |

**GAP:** VM's chroot handler does obfuscated checks before returning 0.
I don't know what those checks do. They might set internal state that
later operations depend on. My implementation (immediate return 0) may
miss required side effects.

## 12. BPF Filter Correctness

| Claim | Evidence | Classification |
|----------|----------|----------------|
| Filter traps mount/umount2/chroot | Test 1 passes (mount returns 0, not EPERM) | TEST-VERIFIED |
| Filter allows getpid | Test 7 passes (getpid returns real PID) | TEST-VERIFIED |
| Filter allows read/write | Test 8 passes (pipe read/write works) | TEST-VERIFIED |
| Filter allows stat | Test 9 passes (stat /proc/self works) | TEST-VERIFIED |
| Filter inherits across fork | Test 10 passes (child's mount is trapped) | TEST-VERIFIED |

## 13. Architecture Verification

| Architecture | Build | Test | Classification |
|--------------|-------|------|----------------|
| x86_64 | ✅ builds | ✅ 12/12 tests pass | TEST-VERIFIED |
| arm64-v8a | ❌ not built | ❌ not tested | UNKNOWN |

**GAP:** arm64 is NOT verified. The assembly stub (`arm64_start.S`) is
written but not compiled or tested. The ucontext layout for arm64 is
from AOSP source (web-fetched), not from a local system header.

## 14. Remaining Gaps

1. **arm64 not tested** — Need cross-compiler + arm64 system or emulator
2. **VM mount table EBUSY behavior** — Not verified from VM binary
3. **VM chroot internal checks** — Not traced (OLLVM obfuscation)
4. **Real PT_INTERP mode** — Current test uses combined binary, not
   separate loader + guest with PT_INTERP
5. **Shared memory mount table** — fork() copies the table; production
   needs MAP_SHARED mmap for parent/child to share state
