fn main() {
    // Pick the right jniLibs subdirectory based on the target architecture.
    // CARGO_CFG_TARGET_ARCH is set by Cargo at build time:
    //   "aarch64"  → arm64-v8a
    //   "x86_64"   → x86_64
    // The legacy closed-source blobs (libOpenglRender.so, libloader.so)
    // only ship in arm64-v8a/. On x86_64 we link against the new
    // open-source libOpenglRender_new.so from app/rs/openglrenderer/
    // instead (see build.sh in that crate).
    //
    // For x86_64 the link line below is a no-op when no x86_64 blobs exist,
    // which is fine because the renderer_bindings.rs `#[link(name="OpenglRender")]`
    // attribute is what actually drives the link, and the linker will only
    // complain if it can't find the symbol at link time. To make x86_64 builds
    // work we need to either:
    //   (a) skip the link entirely on x86_64 (which means the new Rust
    //       renderer must be used), or
    //   (b) provide an x86_64 stub libOpenglRender.so.
    //
    // We go with (a): on x86_64, we don't add the link-search path at all,
    // and the renderer_bindings module is compiled out via a cfg attribute.
    // See lib.rs and renderer_bindings.rs for the cfg gate.
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let abi = match arch.as_str() {
        "aarch64" => "arm64-v8a",
        "x86_64"  => "x86_64",
        _         => "arm64-v8a", // fallback; warn at link time if wrong
    };

    // Only add the link-search path if the directory actually exists.
    // On x86_64, the legacy closed-source libOpenglRender.so blob is not
    // present (only the open-source libOpenglRender_new.so is), so we
    // must NOT add the link path — otherwise the linker tries to link
    // the arm64 blob into an x86_64 binary and fails with
    // "libOpenglRender.so is incompatible with elf_x86_64".
    let jni_libs_dir = format!("../src/main/jniLibs/{}", abi);
    if std::path::Path::new(&jni_libs_dir).exists() {
        // Check whether libOpenglRender.so (the legacy blob) exists in
        // this directory. Only add the link-search path if it does.
        let legacy_blob = format!("{}/libOpenglRender.so", jni_libs_dir);
        if std::path::Path::new(&legacy_blob).exists() {
            println!("cargo:rustc-link-search=native={}", jni_libs_dir);
            println!("cargo:rustc-link-lib=dylib=OpenglRender");
        } else {
            println!("cargo:warning=twoyi build.rs: no legacy libOpenglRender.so in {}; new renderer only", jni_libs_dir);
        }
    } else {
        println!("cargo:warning=twoyi build.rs: jniLibs dir {} does not exist", jni_libs_dir);
    }

    // Compile interp.c to add INTERP segment for direct execution.
    // interp.c is architecture-independent (just a string constant in a
    // section), so it's safe to compile for both aarch64 and x86_64.
    cc::Build::new()
        .file("src/interp.c")
        .compile("interp");

    // The entry point is set via RUSTFLAGS in build_rs.sh: -Wl,-e,main
    // The interp.c file adds the .interp section needed for direct execution
    // This makes the library a PIE executable that can still be loaded by JNI

    // Tell Cargo to re-run this script if the jniLibs dir changes.
    println!("cargo:rerun-if-changed=../src/main/jniLibs");
}
