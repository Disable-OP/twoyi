// seccomp_filter.h — Seccomp BPF filter installation for syscall trapping.
//
// Installs a seccomp-BPF filter that traps specific syscalls with
// SECCOMP_RET_TRAP, causing them to trigger SIGSYS (handled by
// sigsys_handler.c).
//
// Architecture: arm64-v8a and x86_64.
//
// Source: Linux kernel seccomp docs, AOSP bionic seccomp_policy.cpp,
//         VM libkrloader64.so seccomp installation at 0x3384.

#ifndef TWOYI_LOADER_SECCOMP_FILTER_H
#define TWOYI_LOADER_SECCOMP_FILTER_H

// Install the seccomp BPF filter that traps mount, umount2, and chroot.
//
// Prerequisites:
//   1. SIGSYS handler must be installed (twoyi_sigsys_handler_install).
//   2. SIGSYS must be unblocked (done by the handler install).
//   3. PR_SET_NO_NEW_PRIVS must be set (done by this function).
//
// Returns 0 on success, -1 on error (errno set).
int twoyi_seccomp_install(void);

#endif // TWOYI_LOADER_SECCOMP_FILTER_H
