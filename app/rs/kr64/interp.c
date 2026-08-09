// Force a PT_INTERP program-header entry so the resulting cdylib
// (libkr64.so) is directly executable: `./libkr64.so arg1 arg2`.
//
// When the kernel exec's libkr64.so, it reads PT_INTERP and exec's the
// dynamic linker (linker64) which loads libkr64.so, resolves its
// DT_NEEDED deps (libc, libc++, libm, libdl, liblog — the standard
// Android system libs), and jumps to the entry point set by
// `-Wl,-e,kr64_main` (see .cargo/config.toml).
//
// This is the same PIE-as-cdylib trick used by the parent twoyi crate
// (see app/rs/src/interp.c) and by Virtual Master's libkr64.so (see
// VM_KR64_ANALYSIS.md §1: ".interp = libkrloader64.so"). The only
// difference is VM uses a custom-built interpreter (libkrloader64.so)
// instead of the system linker — they did this so they could embed a
// static bionic libc and pre-install shadowhook before any guest code
// runs. Twoyi doesn't need a custom interpreter because we don't ship
// our own bionic or shadowhook (yet).
//
// Architecture-independent: the string is just bytes in a section.
//
// On non-Android targets (e.g. Linux host builds for `cargo test`) we
// emit an empty .interp section. This is because:
//   1. The `/system/bin/linker64` path doesn't exist on Linux, so
//      forcing it would make the test binary un-runnable.
//   2. The Linux default interpreter (/lib64/ld-linux-x86-64.so.2) is
//      what we want for host-side testing.
//   3. The .interp section still needs to exist (even if empty) so the
//      `--undefined=interp` linker flag in build.rs resolves.
#ifdef __ANDROID__
const char interp[] __attribute__((section(".interp"))) =
    "/system/bin/linker64";
#else
// On Linux host: emit the `interp` symbol as a regular global constant
// in `.rodata` (no special section). This satisfies the
// `--undefined=interp` linker flag in build.rs without producing a
// `.interp` section (which would override the interpreter). We avoid
// `__attribute__((section(".comment")))` here because GCC/Clang emit
// a warning about incorrect section attributes for `.comment`.
const char interp[] = "kr64: host build (no .interp override)";
#endif
