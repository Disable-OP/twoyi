// arm64_stub.c — Stub symbols needed by libc.a when linking with -nostartfiles
// and -no-pie on AArch64.
//
// dl-reloc-static-pie.o references _DYNAMIC and _dl_relocate_static_pie,
// which are normally provided by the dynamic linker or by a PIE binary.
// Since we're building a non-PIE static binary with our own _start,
// we provide weak stub definitions so the link succeeds.
//
// These stubs are NOT used at runtime — _dl_relocate_static_pie is only
// called during static PIE initialization, which we skip by using
// -no-pie and providing our own _start.

// Weak empty definition of _DYNAMIC (just needs to exist for linking)
__attribute__((weak))
const char _DYNAMIC[] = {0};

// Weak stub for _dl_relocate_static_pie (called by libc-start for PIE)
__attribute__((weak))
void _dl_relocate_static_pie(void) {
    // No-op: we're not using PIE, so no relocation needed.
}

// Weak stubs for _init and _fini (referenced by libc-start)
__attribute__((weak))
void _init(void) {}

__attribute__((weak))
void _fini(void) {}
