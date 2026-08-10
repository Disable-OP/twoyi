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

### MAJOR MILESTONE: init second stage started!
### Timestamp UTC: 2026-08-10 ~01:15

Boot frontier progression:
1. ✅ init first stage started
2. ✅ first_stage_mount skipped (fstab blocked)
3. ✅ SELinux policy compiled (secilc)
4. ✅ SELinux policy loaded
5. ✅ file_contexts loaded
6. ✅ restorecon succeeded (packages.list accessible)
7. ✅ init second stage started! (FIRST TIME EVER!)
8. ✅ SELinux enforcing=1 set
9. ❌ SIGSEGV during early second-stage init (before zygote)

Current blocker: guest init gets signal 11 (SIGSEGV) during early
second-stage init. Need to investigate what second-stage init does
that causes the crash.

The io.twoyi host process is ALIVE (pid 6144) — the host app didn't crash.
The guest init crashed with SIGSEGV.

### Experiment: property_info on HOST (no chmod) — PARTIAL
### Timestamp UTC: 2026-08-10 ~08:10

**Change:** Create /dev/__properties__/property_info on HOST (no chmod)
**Expected:** WriteStringToFile's open() finds the file → succeeds
**Observed:** "created property_info on host" appears in logs (constructor runs), but WriteStringToFile STILL fails with ENOENT
**Conclusion:** The file is created by the constructor but WriteStringToFile can't find it. Possible causes:
1. O_NOFOLLOW flag in WriteStringToFile — if /dev/__properties__ is a symlink, open fails
2. The constructor runs in first-stage init but NOT in second-stage init (LD_PRELOAD not restored for second execv)
3. The file is created but deleted before second-stage init runs

**Next experiment:** Verify the constructor runs in second-stage init by checking twoyi-loader.log. Also check if /dev/__properties__ is a symlink on the emulator.
