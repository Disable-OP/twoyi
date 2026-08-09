// getpid_hook.c — LD_PRELOAD library that makes getpid() return 1.
//
// Android's init binary checks getpid() == 1 and exits with code 31
// if it's not PID 1. Without unshare(CLONE_NEWPID), we can't make
// the child actually PID 1. Instead, we hook getpid() via LD_PRELOAD
// so init thinks it's PID 1.
//
// This is the same approach VM's libkrloader64.so uses (it hooks
// getpid at the linker level).

#include <unistd.h>
#include <sys/syscall.h>

// Override getpid() to always return 1.
// This makes Android's init think it's PID 1.
pid_t getpid(void) {
    return 1;
}

// Also override getppid() to return 0 (init's parent is 0/the kernel).
pid_t getppid(void) {
    return 0;
}
