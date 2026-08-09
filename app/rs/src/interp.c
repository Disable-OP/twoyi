// Force INTERP segment for direct execution
const char interp[] __attribute__((section(".interp"))) = "/system/bin/linker64";
