// arm64_start.S — Entry point for the custom ELF interpreter (arm64-v8a).
//
// This is the first code that runs when the kernel loads our interpreter
// as PT_INTERP of a guest binary. The kernel sets up the stack with
// argc, argv, envp, auxv, then jumps to our entry point (_start).
//
// The _start stub:
//   1. Passes the stack pointer to twoyi_loader_main() as the first arg
//   2. Calls twoyi_loader_main(), which returns the guest entry point
//   3. Jumps to the guest entry point
//
// This matches the AOSP bionic linker pattern:
//   bionic/linker/arch/arm64/begin.S:
//     mov x0, sp
//     bl __linker_init
//     br x0
//
// Source: https://android.googlesource.com/platform/bionic/+/refs/heads/main/linker/arch/arm64/begin.S

    .text
    .globl _start
    .type _start, %function
_start:
    // The kernel passes the stack pointer in sp.
    // Stack layout at entry:
    //   [sp]      argc
    //   [sp+8]    argv[0]
    //   ...
    //   [sp+8*argc]  NULL
    //   [sp+...]  envp[0]
    //   ...
    //   NULL
    //   auxv...

    // Pass sp as the first argument (in x0, per AArch64 AAPCS).
    mov x0, sp

    // Call twoyi_loader_main(uint64_t *raw_stack).
    // Returns the guest entry point in x0.
    bl twoyi_loader_main

    // Jump to the guest entry point.
    // This is a tail-call (br, not bl) — we never return.
    br x0

    .size _start, .-_start
