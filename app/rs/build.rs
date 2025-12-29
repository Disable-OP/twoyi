fn main() {
    println!("cargo:rustc-link-search=native=../src/main/jniLibs/arm64-v8a");
    
    // Make the shared library executable by setting entry point
    println!("cargo:rustc-cdylib-link-arg=-Wl,-e,main");
    println!("cargo:rustc-cdylib-link-arg=-Wl,--export-dynamic");
}
