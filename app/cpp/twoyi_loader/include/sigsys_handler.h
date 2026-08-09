// sigsys_handler.h — SIGSYS handler for seccomp-trapped syscalls.
//
// This handler receives SIGSYS signals when a seccomp-trapped syscall is
// invoked. It reads the syscall number from siginfo_t, dispatches to the
// appropriate emulator, and writes the return value to the ucontext.
//
// Architecture: arm64-v8a and x86_64 (see arch_regs.h for register access).
//
// Source: Chromium sandbox/linux/seccomp-bpf/trap.cc (handler pattern),
//         VM libkr64.so SIGSYS handler at 0x115f04.

#ifndef TWOYI_LOADER_SIGSYS_HANDLER_H
#define TWOYI_LOADER_SIGSYS_HANDLER_H

#include <signal.h>

// Install the SIGSYS handler with SA_SIGINFO | SA_NODEFER.
// Must be called BEFORE installing the seccomp BPF filter.
//
// Returns 0 on success, -1 on error (errno set).
int twoyi_sigsys_handler_install(void);

// The actual SIGSYS handler function.
// Signature matches sa_sigaction: void(int, siginfo_t*, void*).
void twoyi_sigsys_handler(int signum, siginfo_t *info, void *ucontext);

// Get a count of how many times the handler has been invoked.
// Used by tests to verify the handler was reached.
unsigned int twoyi_sigsys_get_invoke_count(void);

// Get the last syscall number that was trapped.
// Used by tests to verify the correct syscall was identified.
long twoyi_sigsys_get_last_syscall_nr(void);

#endif // TWOYI_LOADER_SIGSYS_HANDLER_H
