fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let abi = match arch.as_str() {
        "aarch64" => "arm64-v8a",
        "x86_64"  => "x86_64",
        _         => "arm64-v8a",
    };
    let jni_libs_dir = format!("../src/main/jniLibs/{}", abi);
    if std::path::Path::new(&jni_libs_dir).exists() {
        let renderer_lib = format!("{}/libOpenglRender.so", jni_libs_dir);
        if std::path::Path::new(&renderer_lib).exists() {
            println!("cargo:rustc-link-search=native={}", jni_libs_dir);
            println!("cargo:rustc-link-lib=dylib=OpenglRender");
        }
    }
    cc::Build::new().file("src/interp.c").compile("interp");
    println!("cargo:rerun-if-changed=../src/main/jniLibs");
}
