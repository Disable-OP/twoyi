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

