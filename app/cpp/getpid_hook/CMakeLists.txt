cmake_minimum_required(VERSION 3.22.1)
project(getpid_hook)

# Build a comprehensive LD_PRELOAD library for rootless Android container
# boot. Hooks getpid(), mount(), mknod(), chroot(), unshare(), etc. to
# return fake success, allowing init to boot in the HOST filesystem
# namespace without needing root privileges.
#
# IMPORTANT: Do NOT link against any shared libraries (no `log`, no
# `c`). The hook only uses `syscall()` which is provided by the
# dynamic linker itself. Linking against liblog.so causes the Android
# linker to load liblog.so as a DT_NEEDED dependency, and if there's
# any version mismatch between the NDK's stub and the emulator's
# liblog.so, the linker crashes with a NULL pointer dereference at
# address 0x86 (signal 11 SIGSEGV in linker64).

add_library(getpid_hook SHARED getpid_hook.c)
# No target_link_libraries — the hook is self-contained.
# Set visibility to default so all hooks are exported.
set_target_properties(getpid_hook PROPERTIES
    C_VISIBILITY_PRESET default
)
