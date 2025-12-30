fn main() {
    println!("cargo:rustc-link-search=native=../src/main/jniLibs/arm64-v8a");
    
    // Note: We do NOT set a custom entry point because it causes segfaults
    // Users should use the wrapper script (twoyi) which invokes via linker64
    // Direct execution of .so files requires proper ELF initialization
}
