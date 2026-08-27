fn main() {
    println!("cargo:rerun-if-changed=src");

    // Configure for PIE executable with main entry point
    println!("cargo:rustc-cdylib-link-arg=-Wl,-e,main");
    println!("cargo:rustc-cdylib-link-arg=-Wl,--dynamic-linker=/system/bin/linker64");
    println!("cargo:rustc-cdylib-link-arg=-pie");
}
