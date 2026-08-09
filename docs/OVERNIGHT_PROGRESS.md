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


### Experiment 2: Recursive mkdir fix — PARTIAL
### Timestamp UTC: 2026-08-10 ~22:35

**Change:** Added mkdir_p() recursive mkdir + open() hook auto-create for selinuxfs
**Expected:** checkreqprot file created, init passes SELinux setup
**Observed:** "selinuxfs virtual files created" logged, but init still gets ENOENT on checkreqprot
**Conclusion:** Directory creation works, but file open still fails. The PLT hook for open() may not be intercepting init's open() call. Init might use a direct syscall or a different libc function.

**Next experiment:** Add path logging to open()/openat() hooks to verify which paths init actually opens. Also check if init uses a different open variant (like __open_2 or openat).
