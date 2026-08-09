cmake_minimum_required(VERSION 3.22.1)
project(getpid_hook)

# Build a tiny LD_PRELOAD library that hooks getpid() to return 1.
# This makes Android's init think it's PID 1, bypassing the exit 31
# check without needing unshare(CLONE_NEWPID) (which fails on the
# emulator even as root).
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
