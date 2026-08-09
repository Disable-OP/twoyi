cmake_minimum_required(VERSION 3.22.1)
project(getpid_hook)

# Build a tiny LD_PRELOAD library that hooks getpid() to return 1.
# This makes Android's init think it's PID 1, bypassing the exit 31
# check without needing unshare(CLONE_NEWPID) (which fails on the
# emulator even as root).

add_library(getpid_hook SHARED getpid_hook.c)
target_link_libraries(getpid_hook log)
