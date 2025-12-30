fn main() {
    println!("cargo:rustc-link-search=native=../src/main/jniLibs/arm64-v8a");
    
    // Compile interp.c to add INTERP segment for direct execution
    cc::Build::new()
        .file("src/interp.c")
        .compile("interp");
    
    // The entry point is set via RUSTFLAGS in build_rs.sh: -Wl,-e,main
    // The interp.c file adds the .interp section needed for direct execution
    // This makes the library a PIE executable that can still be loaded by JNI
}
