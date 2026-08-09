// loader_main.c — Bridge between _start and the test's main().
//
// The real loader logic (seccomp, SIGSYS, mount emulation) is in
// twoyi_loader.c. For the combined test binary, we need to:
//   1. Call twoyi_loader_main() to install seccomp + SIGSYS
//   2. Then call the test's main()
//
// But twoyi_loader_main() in twoyi_loader.c expects to jump to AT_ENTRY
// (the real PT_INTERP flow). For the combined test, we override it here
// to call main() instead.

#include <stdint.h>
#include <unistd.h>
#include <string.h>
#include <stdio.h>
#include <errno.h>
#include <signal.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <linux/seccomp.h>
#include <linux/filter.h>

// Forward declarations of functions from twoyi_loader.c
extern int install_sigsys_handler(void);
extern int install_seccomp(void);
extern void init_rootfs_prefix(void);
extern volatile int g_runtime_ready;

// The test's main()
extern int main(int argc, char **argv, char **envp);

// Helper: write a string to a file descriptor
static void write_str(int fd, const char *s) {
    if (s) {
        size_t len = 0;
        while (s[len]) len++;
        write(fd, s, len);
    }
}

// This is called from _start. For the combined test, we install
// seccomp + SIGSYS, then call main() directly.
uint64_t twoyi_loader_main(uint64_t *raw_stack) {
    write_str(2, "[twoyi_loader] starting (combined test mode)\n");

    // Initialize rootfs prefix
    init_rootfs_prefix();

    // Install SIGSYS handler
    if (install_sigsys_handler() != 0) {
        write_str(2, "[twoyi_loader] FATAL: sigsys handler install failed\n");
        _exit(1);
    }
    write_str(2, "[twoyi_loader] SIGSYS handler installed\n");

    // Install seccomp filter
    if (install_seccomp() != 0) {
        write_str(2, "[twoyi_loader] FATAL: seccomp install failed\n");
        _exit(1);
    }
    write_str(2, "[twoyi_loader] seccomp filter installed\n");

    // Mark runtime as ready
    g_runtime_ready = 1;
    write_str(2, "[twoyi_loader] runtime ready\n");

    // Parse argc/argv/envp from stack
    uint64_t argc = raw_stack ? *raw_stack : 0;
    char **argv = (char **)(raw_stack + 1);
    char **envp = argv + argc + 1;

    // Call main()
    write_str(2, "[twoyi_loader] calling main()\n");
    int ret = main((int)argc, argv, envp);
    fflush(NULL);
    _exit(ret);
    return 0;
}
