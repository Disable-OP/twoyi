// arch_regs.h — Architecture-specific register access for SIGSYS handler.
//
// This header provides a unified API for accessing syscall arguments and
// the return-value register in ucontext_t, across arm64-v8a and x86_64.
//
// Sources (primary):
//   - arm64: AOSP bionic libc/include/sys/ucontext.h (mcontext_t = struct sigcontext)
//            Linux kernel arch/arm64/include/uapi/asm/sigcontext.h
//            Chromium sandbox/linux/bpf_dsl/seccomp_macros.h
//   - x86_64: AOSP bionic libc/include/sys/ucontext.h (mcontext_t.gregs[REG_*])
//             Linux kernel arch/x86/include/uapi/asm/sigcontext.h
//             Chromium sandbox/linux/bpf_dsl/seccomp_macros.h
//
// Key difference between architectures:
//   arm64: syscall number is in x8 (regs[8]), return value in x0 (regs[0])
//          — DIFFERENT registers
//   x86_64: syscall number AND return value both in rax (gregs[REG_RAX]=13)
//          — SAME register (after syscall_rollback, rax = orig_rax = syscall nr)
//
// After seccomp SECCOMP_RET_TRAP:
//   arm64: kernel does syscall_rollback → regs[0] = orig_x0 (original arg1)
//          x8 still contains the syscall number
//   x86_64: kernel does syscall_rollback → rax = orig_rax (syscall number)
//          The handler must overwrite rax with the desired return value.

#ifndef TWOYI_LOADER_ARCH_REGS_H
#define TWOYI_LOADER_ARCH_REGS_H

#include <stdint.h>
#include <ucontext.h>

// Architecture identification for BPF filter
#if defined(__aarch64__)
  #define TWOYI_AUDIT_ARCH 0xC00000B7U  // AUDIT_ARCH_AARCH64
  #define TWOYI_ARCH_NAME "arm64-v8a"
#elif defined(__x86_64__)
  #define TWOYI_AUDIT_ARCH 0xC000003EU  // AUDIT_ARCH_X86_64
  #define TWOYI_ARCH_NAME "x86_64"
#else
  #error "Unsupported architecture"
#endif

// =========================================================================
// Unified register access API
// =========================================================================
// These macros provide architecture-independent access to the syscall
// number, arguments, and return-value register in a ucontext_t.

#if defined(__aarch64__)

// arm64: mcontext_t IS struct sigcontext (no gregs[] indirection)
// struct sigcontext { __u64 fault_address; __u64 regs[31]; __u64 sp; __u64 pc; ... };

// Read the syscall number from the ucontext.
// On arm64, the syscall number is in x8 (regs[8]).
static inline long twoyi_get_syscall_nr(ucontext_t *ctx) {
    return (long)ctx->uc_mcontext.regs[8];
}

// Read syscall arguments from the ucontext.
// On arm64, args are in x0-x5 (regs[0]-regs[5]).
// NOTE: After syscall_rollback, regs[0] = orig_x0 (the original first arg),
// which is what we want.
static inline unsigned long twoyi_get_arg(ucontext_t *ctx, int n) {
    return (unsigned long)ctx->uc_mcontext.regs[n];
}

// Write the return value into the ucontext.
// On arm64, the return value goes in x0 (regs[0]).
static inline void twoyi_set_return_value(ucontext_t *ctx, long value) {
    ctx->uc_mcontext.regs[0] = (uint64_t)value;
}

// Get the instruction pointer where the syscall was invoked.
static inline unsigned long twoyi_get_ip(ucontext_t *ctx) {
    return (unsigned long)ctx->uc_mcontext.pc;
}

#elif defined(__x86_64__)

// x86_64: mcontext_t has gregs[] array with REG_* indices
// enum { REG_R8=0, REG_R9, REG_R10, REG_R11, REG_R12, REG_R13, REG_R14,
//        REG_R15, REG_RDI, REG_RSI, REG_RBP, REG_RBX, REG_RDX, REG_RAX,
//        REG_RCX, REG_RSP, REG_RIP, REG_EFL, ... };

#include <sys/ucontext.h>

// REG_RAX = 13, REG_RDI = 8, REG_RSI = 9, REG_RDX = 12,
// REG_R10 = 2, REG_R8 = 0, REG_R9 = 1, REG_RIP = 16

// Read the syscall number from the ucontext.
// On x86_64, after syscall_rollback, rax (gregs[REG_RAX]) = orig_rax = syscall nr.
static inline long twoyi_get_syscall_nr(ucontext_t *ctx) {
    return (long)ctx->uc_mcontext.gregs[13]; // REG_RAX
}

// Read syscall arguments from the ucontext.
// On x86_64: arg1=rdi(8), arg2=rsi(9), arg3=rdx(12),
//            arg4=r10(2), arg5=r8(0), arg6=r9(1)
static inline unsigned long twoyi_get_arg(ucontext_t *ctx, int n) {
    switch (n) {
        case 0: return (unsigned long)ctx->uc_mcontext.gregs[8];  // REG_RDI
        case 1: return (unsigned long)ctx->uc_mcontext.gregs[9];  // REG_RSI
        case 2: return (unsigned long)ctx->uc_mcontext.gregs[12]; // REG_RDX
        case 3: return (unsigned long)ctx->uc_mcontext.gregs[2];  // REG_R10
        case 4: return (unsigned long)ctx->uc_mcontext.gregs[0];  // REG_R8
        case 5: return (unsigned long)ctx->uc_mcontext.gregs[1];  // REG_R9
        default: return 0;
    }
}

// Write the return value into the ucontext.
// On x86_64, the return value goes in rax (gregs[REG_RAX]).
// This OVERWRITES the syscall number (which was in rax after rollback).
static inline void twoyi_set_return_value(ucontext_t *ctx, long value) {
    ctx->uc_mcontext.gregs[13] = (long)value; // REG_RAX
}

// Get the instruction pointer where the syscall was invoked.
static inline unsigned long twoyi_get_ip(ucontext_t *ctx) {
    return (unsigned long)ctx->uc_mcontext.gregs[16]; // REG_RIP
}

#endif // arch

// =========================================================================
// Syscall number constants (for the BPF filter + handler dispatch)
// =========================================================================
// These are the syscall numbers that our BPF filter will trap.
// On arm64 and x86_64, most syscall numbers DIFFER — we must use
// architecture-specific values.

#if defined(__aarch64__)
  // arm64 syscall numbers (from linux/asm-generic/unistd.h)
  #define TWOYI_NR_mount    40
  #define TWOYI_NR_umount2  39
  #define TWOYI_NR_chroot   51
  #define TWOYI_NR_mknodat  33
  #define TWOYI_NR_openat   56
  #define TWOYI_NR_getpid   172
#elif defined(__x86_64__)
  // x86_64 syscall numbers (from linux/asm-x86/unistd_64.h)
  #define TWOYI_NR_mount    165
  #define TWOYI_NR_umount2  166
  #define TWOYI_NR_chroot   161
  #define TWOYI_NR_mknodat  259
  #define TWOYI_NR_openat   257
  #define TWOYI_NR_getpid   39
#endif

#endif // TWOYI_LOADER_ARCH_REGS_H
