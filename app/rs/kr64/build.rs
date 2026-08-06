// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// Build script for the kr64 crate.
//
// Responsibilities:
//   1. Compile `interp.c` into a static archive so the `.interp` section
//      ends up in the final libkr64.so. This makes the cdylib directly
//      executable (see interp.c for details).
//   2. Emit the PIE linker flags that turn the cdylib into a Position
//      Independent Executable. These flags are emitted ONLY for the
//      cdylib target (via `cargo:rustc-cdylib-link-arg=`), so the `bin`
//      target remains a regular Rust binary that does not need any of
//      this trickery — Rust already produces a PIE for `bin` targets on
//      Android by default.
//
// The flags are the same ones used by the parent twoyi crate (see
// app/rs/.cargo/config.toml) — they are known to work for both
// aarch64-linux-android and x86_64-linux-android.

fn main() {
    // Re-run if interp.c changes.
    println!("cargo:rerun-if-changed=interp.c");

    // Compile interp.c — produces a static archive `libinterp.a` which
    // the linker will pull the `.interp` section out of. The
    // `--undefined=interp` link flag below forces the section to be
    // retained in the final binary (otherwise the linker may GC it).
    cc::Build::new().file("interp.c").compile("interp");

    // PIE flags for the cdylib. See app/rs/.cargo/config.toml for the
    // canonical version (these flags are duplicated here so the kr64
    // crate is self-contained and can be built standalone without
    // inheriting the workspace config).
    //
    // -Wl,-e,kr64_main          : set `kr64_main` (defined in src/lib.rs)
    //                              as the ELF entry point.
    // -Wl,--dynamic-linker=...  : PT_INTERP path for the dynamic linker.
    // -Wl,-rpath,$ORIGIN        : look for shared libs next to the .so
    //                              first (so libkr64.so can find libc.so
    //                              etc. in the same jniLibs/<abi>/ dir).
    // -Wl,--enable-new-dtags    : use DT_RUNPATH (newer) instead of
    //                              DT_RPATH (older) — needed for the
    //                              $ORIGIN lookup above to work right.
    // -pie                      : produce a PIE (not a static exec).
    // -Wl,--undefined=interp    : force the .interp symbol from interp.c
    //                              to be retained in the final binary.
    //
    // These flags are ONLY emitted when targeting Android — on a Linux
    // host build (e.g. `cargo test` during development) the
    // `/system/bin/linker64` interpreter doesn't exist, and forcing it
    // would make the resulting binary un-runnable on the host. The
    // `interp.c` symbol reference is still emitted on all targets so
    // the `.interp` section is retained (it's harmless on Linux — it
    // just overrides the interpreter to `/system/bin/linker64`, which
    // we DON'T want on Linux, so we skip that flag too).
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let is_android = target_os == "android";

    // The `--undefined=interp` flag forces the linker to retain the
    // `.interp` section from interp.c. We emit this on all targets so
    // the symbol reference in lib.rs (`extern "C" { static INTERP }`)
    // resolves. On non-Android targets we DON'T emit the
    // `/system/bin/linker64` PT_INTERP override — the default Linux
    // interpreter is used instead, which lets `cargo test` run.
    println!("cargo:rustc-cdylib-link-arg=-Wl,--undefined=interp");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-e,kr64_main");
    println!("cargo:rustc-cdylib-link-arg=-pie");
    println!("cargo:rustc-cdylib-link-arg=-Wl,--enable-new-dtags");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-rpath,$ORIGIN");

    if is_android {
        // Android: use the system linker64 as the PT_INTERP. This is
        // what makes `./libkr64.so arg1 arg2` directly executable on
        // Android (the kernel reads PT_INTERP and exec's linker64,
        // which loads libkr64.so and jumps to kr64_main).
        println!("cargo:rustc-cdylib-link-arg=-Wl,--dynamic-linker=/system/bin/linker64");
    }
    // On Linux host builds we omit --dynamic-linker so the default
    // /lib64/ld-linux-x86-64.so.2 is used — this lets `cargo test`
    // actually run the test binary.
}
