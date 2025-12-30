fn main() {
    println!("cargo:rustc-link-search=native=../src/main/jniLibs/arm64-v8a");
    
    // The entry point is set via RUSTFLAGS in build_rs.sh: -Wl,-e,main
    // This allows the library to be executed directly via linker64
    // Without this, the entry point would be 0x0 causing segmentation faults
}
