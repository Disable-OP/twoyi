// build.rs — twoyi native (Rust) build script.
//
// After task AOSP-VENDOR-1 the only native dependency is
// `libOpenglRender.so`, which is now built from the vendored AOSP
// emugl source under `app/cpp/emugl` by `app/cpp/build.sh`
// (invoked via the `cmakeBuild` Gradle task). By the time `cargo
// build` runs, that .so is already sitting in
// `app/src/main/jniLibs/<abi>/libOpenglRender.so`, so all we have to
// do here is point the linker at it.
//
// There is no longer any per-arch fallback / conditional logic —
// both arm64-v8a and x86_64 ship the AOSP-from-source .so.

fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let abi = match arch.as_str() {
        "aarch64" => "arm64-v8a",
        "x86_64" => "x86_64",
        _ => "arm64-v8a",
    };
    let jni_libs_dir = format!("../src/main/jniLibs/{}", abi);
    let renderer_lib = format!("{}/libOpenglRender.so", jni_libs_dir);
    if !std::path::Path::new(&renderer_lib).exists() {
        panic!(
            "libOpenglRender.so not found at {} — run `app/cpp/build.sh` \
             (or the Gradle `cmakeBuild` task) before building the Rust crate.",
            renderer_lib
        );
    }
    println!("cargo:rustc-link-search=native={}", jni_libs_dir);
    println!("cargo:rustc-link-lib=dylib=OpenglRender");

    cc::Build::new().file("src/interp.c").compile("interp");
    println!("cargo:rerun-if-changed=../src/main/jniLibs");
    println!("cargo:rerun-if-changed=../cpp/emugl");
}
