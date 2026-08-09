# Implementation Plan: Vertical Slice 1

## Custom ELF Interpreter → Seccomp → SIGSYS → Mount Emulation

**Status:** Implementation in progress
**Date:** 2026-08-10
**Goal:** Prove the end-to-end seccomp/SIGSYS virtualization path with ONE real syscall (mount)

---

## Dependencies (in order)

```
1. Architecture-specific _start stub (arm64.S, x86_64.S)
   ↓ (provides stack pointer to loader_main)
2. loader_main (parse auxv, find AT_ENTRY)
   ↓ (knows where to jump after setup)
3. SIGSYS handler installation (sigaction + SA_SIGINFO|SA_NODEFER)
   ↓ (handler ready before filter installed)
4. Unblock SIGSYS (sigprocmask)
   ↓ (prevent deadlock if handler triggers another trap)
5. PR_SET_NO_NEW_PRIVS (prctl)
   ↓ (mandatory prerequisite for seccomp)
6. Seccomp BPF filter (trap mount syscall)
   ↓ (mount() now triggers SIGSYS)
7. Jump to AT_ENTRY (guest main runs)
   ↓ (guest calls mount())
8. SIGSYS handler fires (reads si_syscall, ucontext args)
   ↓ (identifies mount, extracts source/target/fstype/flags)
9. Virtual mount table (real semantics: add entry, check EBUSY)
   ↓ (updates observable state)
10. Write return value to ucontext (arch-specific register)
    ↓ (guest sees mount() returned 0)
11. Guest continues execution normally
```

---

## Architecture-Specific Details (from research)

### arm64-v8a

| Register | ucontext access | Role |
|----------|----------------|------|
| x0 | `uc_mcontext.regs[0]` | Return value / arg1 |
| x8 | `uc_mcontext.regs[8]` | Syscall number |
| x1-x5 | `uc_mcontext.regs[1..5]` | args 2-6 |
| pc | `uc_mcontext.pc` | Instruction pointer |
| AUDIT_ARCH | `0xC00000B7` | BPF arch check |

### x86_64

| Register | ucontext access | Role |
|----------|----------------|------|
| rax | `uc_mcontext.gregs[REG_RAX]` (13) | Return value AND syscall nr |
| rdi | `uc_mcontext.gregs[REG_RDI]` (8) | arg1 |
| rsi | `uc_mcontext.gregs[REG_RSI]` (9) | arg2 |
| rdx | `uc_mcontext.gregs[REG_RDX]` (12) | arg3 |
| r10 | `uc_mcontext.gregs[REG_R10]` (2) | arg4 |
| r8 | `uc_mcontext.gregs[REG_R8]` (0) | arg5 |
| r9 | `uc_mcontext.gregs[REG_R9]` (1) | arg6 |
| rip | `uc_mcontext.gregs[REG_RIP]` (16) | Instruction pointer |
| AUDIT_ARCH | `0xC000003E` | BPF arch check |

**Key difference:** On arm64, syscall nr (x8) and return value (x0) are DIFFERENT registers. On x86_64, they're the SAME register (rax) — after `syscall_rollback`, rax contains the original syscall number, and the handler overwrites it with the return value.

---

## Virtual Mount Table Semantics (REAL, not return 0)

Based on VM binary evidence (`mount_mgr` at `0x8618` in libkr64.so):

```c
struct mount_entry {
    char source[256];   // e.g., "tmpfs", "proc", "sysfs"
    char target[256];   // e.g., "/dev", "/proc", "/sys"
    char fstype[64];    // e.g., "tmpfs", "proc", "sysfs"
    unsigned long flags; // MS_NOSUID, MS_BIND, etc.
    bool active;
};

// Global mount table (fixed-size for simplicity, no malloc in signal handler)
static struct mount_entry mount_table[32];
static int mount_count = 0;

// mount() emulation:
// 1. Check if target is already mounted -> EBUSY
// 2. Check special paths (/dev, /mnt, /storage) -> skip (no-op, return 0)
// 3. Add entry to table
// 4. Return 0 (success)
```

**Observable behavior:**
- First `mount("tmpfs", "/dev", "tmpfs", ...)` → returns 0, adds entry
- Second `mount("tmpfs", "/dev", "tmpfs", ...)` → returns -1, errno=EBUSY
- `mount(NULL, "/proc", "proc", ...)` → returns 0, adds entry
- Future: `/proc/mounts` can read the virtual mount table

---

## Test Plan

### test_guest.c (the guest ELF that gets exec'd)

```c
int main() {
    // Test 1: trapped syscall reaches handler and returns 0
    int ret = mount("tmpfs", "/test_dev", "tmpfs", MS_NOSUID, "mode=0755");
    assert(ret == 0);

    // Test 2: arguments decoded correctly (check mount table state)
    assert(mount_table_contains("/test_dev", "tmpfs"));

    // Test 3: EBUSY on duplicate mount
    ret = mount("tmpfs", "/test_dev", "tmpfs", 0, NULL);
    assert(ret == -1 && errno == EBUSY);

    // Test 4: non-trapped syscall works normally
    pid_t pid = getpid();
    assert(pid > 0);

    // Test 5: child process inherits seccomp filter
    pid_t child = fork();
    if (child == 0) {
        // Child: mount should still be trapped
        ret = mount("proc", "/test_proc", "proc", 0, NULL);
        assert(ret == 0);
        _exit(0);
    }
    waitpid(child, &ret, 0);
    assert(WIFEXITED(ret) && WEXITSTATUS(ret) == 0);

    printf("ALL TESTS PASSED\n");
    return 0;
}
```

### Test verification points:

1. ✅ Trapped syscall reaches handler (mount returns 0, not EPERM)
2. ✅ Arguments decoded correctly (mount table has correct source/target/fstype)
3. ✅ Emulated state is updated (mount table entry exists)
4. ✅ Subsequent operations observe that state (second mount returns EBUSY)
5. ✅ Return values match Linux semantics (0=success, -1+errno=error)
6. ✅ Non-trapped syscalls continue normally (getpid works)
7. ✅ Child processes preserve virtualization (fork+mount in child still trapped)

---

## File Structure

```
app/cpp/twoyi_loader/
├── CMakeLists.txt
├── arch/
│   ├── arm64_start.S        # _start for arm64
│   └── x86_64_start.S       # _start for x86_64
├── include/
│   ├── twoyi_loader.h       # public API
│   ├── sigsys_handler.h     # SIGSYS handler interface
│   ├── mount_table.h        # virtual mount table interface
│   └── arch_regs.h          # arch-specific register access macros
├── src/
│   ├── loader_main.c        # entry point: parse auxv, install seccomp, jump
│   ├── sigsys_handler.c     # SIGSYS handler implementation
│   ├── seccomp_filter.c     # BPF filter construction + installation
│   └── mount_table.c        # virtual mount table (real semantics)
└── tests/
    ├── test_guest.c         # guest program that calls mount()
    ├── test_runner.sh       # builds + runs the test
    └── CMakeLists.txt
```

---

## Build & Test Strategy

### Phase 1: x86_64 Linux (fast iteration)
- Build loader as static PIE with custom _start
- Build guest as static binary with PT_INTERP=./twoyi_loader
- Run guest directly on Linux
- Verify all 7 test points pass

### Phase 2: arm64 Android (via KVM test)
- Cross-compile loader for arm64
- Cross-compile guest for arm64
- Push to emulator, run, verify
- Verify arch-specific ucontext access works

---

## Target Android Version

**Android 11 (API 30)** — matches the emulator system image used in KVM tests.
- arm64-v8a: primary target (real devices)
- x86_64: secondary target (emulator/CI testing)

Both ABIs must be supported with correct arch-specific register access.
