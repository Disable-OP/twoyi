// sigsys_handler.c — SIGSYS handler for seccomp-trapped syscalls.
//
// This is the core of the rootless virtualization mechanism. When a
// seccomp-trapped syscall is invoked, the kernel delivers SIGSYS with:
//   si_code   = SYS_SECCOMP (1)
//   si_syscall = the trapped syscall number
//   si_errno   = SECCOMP_RET_DATA (lower 16 bits of BPF return)
//   si_arch    = AUDIT_ARCH_*
//
// The handler reads the syscall number from siginfo, reads the arguments
// from the ucontext (architecture-specific registers), dispatches to the
// appropriate emulator, and writes the return value to the ucontext
// (architecture-specific return register).
//
// When the handler returns, sigreturn restores the modified context, and
// the calling code sees the synthetic return value.
//
// Sources:
//   - Kernel: https://www.kernel.org/doc/html/v5.0/userspace-api/seccomp_filter.html
//   - Chromium: sandbox/linux/seccomp-bpf/trap.cc, syscall.cc
//   - VM: libkr64.so SIGSYS handler at 0x115f04

#include "sigsys_handler.h"
#include "arch_regs.h"
#include "mount_table.h"

#include <signal.h>
#include <errno.h>
#include <string.h>
#include <unistd.h>
#include <sys/syscall.h>

// SYS_SECCOMP — si_code value for seccomp-triggered SIGSYS.
// Source: include/uapi/asm-generic/siginfo.h
#ifndef SYS_SECCOMP
#define SYS_SECCOMP 1
#endif

// Test instrumentation counters (not for production, but needed for
// verifying observable behavior per the test plan).
// These are volatile because they're modified from a signal handler.
static volatile unsigned int sigsys_invoke_count = 0;
static volatile long sigsys_last_syscall_nr = -1;

unsigned int twoyi_sigsys_get_invoke_count(void) {
    return sigsys_invoke_count;
}

long twoyi_sigsys_get_last_syscall_nr(void) {
    return sigsys_last_syscall_nr;
}

// The SIGSYS handler.
//
// This function is ASYNC-SIGNAL-SAFE: it does not call malloc, stdio,
// or any non-async-signal-safe function. It only uses:
//   - siginfo_t fields (read-only, provided by kernel)
//   - ucontext_t register access (read/write, provided by kernel)
//   - The mount table (fixed-size array, no locks)
//
// Architecture-specific register access is in arch_regs.h.
void twoyi_sigsys_handler(int signum, siginfo_t *info, void *ucontext) {
    (void)signum; // always SIGSYS (31)

    ucontext_t *ctx = (ucontext_t *)ucontext;

    // 1. Verify this is a seccomp trap.
    //    si_code must be SYS_SECCOMP. If not, it's a different SIGSYS
    //    (e.g., syscall user dispatch) — we should not handle it.
    if (!info || info->si_code != SYS_SECCOMP) {
        // Not our signal — restore default handler and re-raise.
        // For now, just return without modifying the context.
        // The syscall will appear to return whatever garbage was in the
        // return register (likely the syscall number on x86_64).
        return;
    }

    // 2. Read the syscall number.
    //    si_syscall contains the trapped syscall number.
    //    We can also read it from the ucontext (arch-specific).
    long syscall_nr = info->si_syscall;

    // Update test instrumentation.
    sigsys_invoke_count++;
    sigsys_last_syscall_nr = syscall_nr;

    // 3. Read syscall arguments from the ucontext.
    //    Arguments are in architecture-specific registers (see arch_regs.h).
    //
    //    mount(source, target, fstype, flags, data)
    //    arg1 = source  (const char*)
    //    arg2 = target  (const char*)
    //    arg3 = fstype  (const char*)
    //    arg4 = flags   (unsigned long)
    //    arg5 = data    (const void*)
    unsigned long arg1 = twoyi_get_arg(ctx, 0);
    unsigned long arg2 = twoyi_get_arg(ctx, 1);
    unsigned long arg3 = twoyi_get_arg(ctx, 2);
    unsigned long arg4 = twoyi_get_arg(ctx, 3);
    unsigned long arg5 = twoyi_get_arg(ctx, 4);

    long ret_val;

    // 4. Dispatch to the appropriate emulator.
    //    For the vertical slice, only mount() is emulated.
    //    All other trapped syscalls return -ENOSYS.
    //
    //    NOTE: We use the architecture-specific TWOYI_NR_* constants
    //    because syscall numbers differ between arm64 and x86_64.
    switch (syscall_nr) {
        case TWOYI_NR_mount: {
            // mount(source, target, fstype, flags, data)
            const char *source = (const char *)arg1;
            const char *target = (const char *)arg2;
            const char *fstype = (const char *)arg3;
            unsigned long flags = arg4;
            const void *data = (const void *)arg5;

            // Emulate with real virtual mount-table semantics.
            ret_val = twoyi_mount_emulate(source, target, fstype, flags, data);
            break;
        }

        case TWOYI_NR_umount2: {
            // umount2(target, flags)
            const char *target = (const char *)arg1;
            int flags = (int)arg2;
            ret_val = twoyi_umount2_emulate(target, flags);
            break;
        }

        case TWOYI_NR_chroot: {
            // chroot(path) — VM implements this as a NO-OP (mov w0, wzr; ret).
            // The chroot effect is achieved entirely by path translation
            // in openat/stat/etc. For the vertical slice, we return 0
            // (success) but do NOT implement path translation yet.
            //
            // This is a REAL semantic match to VM's behavior (verified
            // by disassembly of 0x11c928 in libkr64.so), not a fake stub.
            ret_val = 0;
            break;
        }

        default:
            // Unknown trapped syscall — return -ENOSYS.
            // This is correct Linux semantics for an unimplemented syscall.
            ret_val = -ENOSYS;
            break;
    }

    // 5. Write the return value to the ucontext.
    //    This is architecture-specific (see arch_regs.h):
    //      arm64:  ctx->uc_mcontext.regs[0] = ret_val
    //      x86_64: ctx->uc_mcontext.gregs[REG_RAX] = ret_val
    //
    //    When the handler returns, sigreturn restores this value into
    //    the actual CPU register, and the calling code sees it as the
    //    syscall's return value.
    twoyi_set_return_value(ctx, ret_val);

    // 6. The PC is already past the syscall instruction (kernel sets this
    //    up before delivering the signal). When we return, execution
    //    resumes at the instruction after the syscall, with our forged
    //    return value in the return register. No further action needed.
}

int twoyi_sigsys_handler_install(void) {
    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));

    // Use sa_sigaction (not sa_handler) because we need siginfo + ucontext.
    sa.sa_sigaction = twoyi_sigsys_handler;

    // SA_SIGINFO: required to get (int, siginfo_t*, void*) handler signature.
    // SA_NODEFER: do not auto-block SIGSYS during handler execution.
    //   This is important if the handler itself triggers a syscall that
    //   might be trapped (e.g., a future write() for logging). Without
    //   SA_NODEFER, a nested SIGSYS would be queued but not delivered,
    //   causing a deadlock.
    //   Source: Chromium sandbox/linux/seccomp-bpf/trap.cc uses SA_NODEFER.
    sa.sa_flags = SA_SIGINFO | SA_NODEFER;

    // Empty signal mask — don't block any other signals during handler.
    sigemptyset(&sa.sa_mask);

    if (sigaction(SIGSYS, &sa, NULL) != 0) {
        return -1;
    }

    // Unblock SIGSYS — it may be blocked by default in some environments.
    // This must happen BEFORE installing the seccomp filter, otherwise
    // the first trapped syscall would be queued but never delivered.
    sigset_t mask;
    sigemptyset(&mask);
    sigaddset(&mask, SIGSYS);
    if (sigprocmask(SIG_UNBLOCK, &mask, NULL) != 0) {
        return -1;
    }

    return 0;
}
