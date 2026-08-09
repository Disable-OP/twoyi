# Overnight Progress Log

## Timestamp UTC: 2026-08-10 ~20:55
## Goal: Fix SIGSYS handler not firing in Android init context

### Current Evidence
- .init_array runs: "SIGSYS handler installed", "seccomp filter installed"
- init runs: "init first stage started!", fstab errors, "First stage mount skipped"
- guest killed by signal 31 (SIGSYS)
- NO handler diagnostic output when SIGSYS fires

### Key Question
Why is the SIGSYS handler installed but not catching signals when init triggers a trapped syscall?

### Hypotheses to test:
A. BPF filter returns KILL not TRAP
B. Inherited seccomp filter kills process
C. SIGSYS disposition changed after constructor
D. SIGSYS is masked
E. Handler runs but fails before logging
F. Handler invokes trapped syscall (recursion)
G. Different thread triggers failure
H. Architecture/syscall matching incorrect

### Experiment 1: SECCOMP_RET_ERRNO diagnostic
Change TRAP to ERRNO(EPERM) for ALL trapped syscalls. If init survives longer, the issue is signal delivery. If init still dies, the issue is BPF filter or inherited filter.


### Experiment 1: ROOT CAUSE FOUND — execv() resets signal handlers
### Timestamp UTC: 2026-08-10 ~21:00

**Hypothesis confirmed:** Android init's FirstStageMain calls `clearenv()` (wipes LD_PRELOAD) then `execv("/system/bin/init", "selinux_setup")` which resets ALL signal handlers to SIG_DFL. The seccomp filter survives execve, but the SIGSYS handler is gone.

**Evidence:**
- AOSP first_stage_init.cpp line 347: `clearenv()` wipes LD_PRELOAD
- AOSP first_stage_init.cpp line 565: `execv()` resets signal handlers to SIG_DFL
- Seccomp filters survive execve (kernel semantics)
- After execv: filter active, handler gone → SIGSYS → default kill

**Fix:** Hook execv/execve to re-set LD_PRELOAD before each exec, so the .init_array re-installs the handler in the new process image.

**Also found:** Bionic's debuggerd_register_handlers installs a SIGSYS handler BEFORE constructors run (step 4, before step 8b). Our constructor overrides it. But if sigaction() is intercepted by ART's libsigchain, it might silently no-op. Fix: use raw syscall for rt_sigaction.

