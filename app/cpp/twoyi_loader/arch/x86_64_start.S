// x86_64_start.S — Entry point for the custom ELF interpreter (x86_64).
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
//   arch/arm64/begin.S: mov x0, sp; bl __linker_init; br x0
//   (for x86_64 it would be: mov %rsp, %rdi; call __linker_init; jmp *%rax)
//
// We use raw assembly (not C) because:
//   - No C runtime is available (no crt0, no __libc_start_main)
//   - We need the raw stack pointer before any prologue
//   - The function must not return (it jumps, not calls)

    .text
    .globl _start
    .type _start, @function
_start:
    // The kernel passes the stack pointer in %rsp.
    // Stack layout at entry:
    //   [rsp]     argc
    //   [rsp+8]   argv[0]
    //   ...
    //   [rsp+8*argc]  NULL
    //   [rsp+...] envp[0]
    //   ...
    //   NULL
    //   auxv...

    // Pass %rsp as the first argument (in %rdi, per x86_64 SysV ABI).
    movq %rsp, %rdi

    // Call twoyi_loader_main(uint64_t *raw_stack).
    // Returns the guest entry point in %rax.
    call twoyi_loader_main

    // Jump to the guest entry point.
    // This is a tail-call (jmp, not call) — we never return.
    jmp *%rax

    // Mark the function as not returning (for the assembler).
    .size _start, .-_start
