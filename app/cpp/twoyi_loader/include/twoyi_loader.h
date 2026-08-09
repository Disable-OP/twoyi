// twoyi_loader.h — Custom ELF interpreter (PT_INTERP) for Twoyi.
//
// This is the custom dynamic linker that the kernel loads as the ELF
// interpreter for guest binaries. It:
//   1. Parses the kernel-provided auxv to find the guest's entry point
//   2. Installs the SIGSYS handler
//   3. Installs the seccomp BPF filter (traps mount/umount2/chroot)
//   4. Jumps to the guest's entry point
//
// The guest then runs with seccomp active — any trapped syscall triggers
// SIGSYS, which is handled by our emulator.
//
// Architecture: arm64-v8a and x86_64.
//
// Source: AOSP bionic/linker/arch/arm64/begin.S (_start pattern),
//         AOSP bionic/linker/linker_main.cpp (auxv parsing),
//         VM libkrloader64.so (custom interpreter pattern).

#ifndef TWOYI_LOADER_H
#define TWOYI_LOADER_H

#include <stdint.h>

// Loader entry point called by the assembly _start stub.
//
// The _start stub passes the raw stack pointer (which points to the
// kernel-provided argument block: argc, argv, envp, auxv).
//
// This function:
//   1. Parses the stack to find AT_ENTRY in auxv
//   2. Installs the SIGSYS handler
//   3. Installs the seccomp BPF filter
//   4. Returns the guest's entry point address
//
// The _start stub then jumps to the returned address.
//
// Parameters:
//   raw_stack — pointer to the top of the stack (argc is at *raw_stack)
//
// Returns: address of the guest's entry point (AT_ENTRY from auxv).
uint64_t twoyi_loader_main(uint64_t *raw_stack);

#endif // TWOYI_LOADER_H
