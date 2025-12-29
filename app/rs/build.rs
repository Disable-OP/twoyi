fn main() {
    println!("cargo:rustc-link-search=native=../src/main/jniLibs/arm64-v8a");
    
    // Add linker flags to make the .so executable
    // -e main sets the entry point to our main function
    println!("cargo:rustc-cdylib-link-arg=-Wl,-e,main");
}
