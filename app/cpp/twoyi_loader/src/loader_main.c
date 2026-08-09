// loader_main.c — Custom ELF interpreter entry point.
//
// In the real Twoyi system, this is the C implementation of the custom
// ELF interpreter that the kernel loads as PT_INTERP for guest binaries.
// It parses auxv to find AT_ENTRY, installs seccomp + SIGSYS, and returns
// the guest entry point for the _start stub to jump to.
//
// For the vertical slice test, we also support a "combined" mode where
// the loader and test guest are in the same binary. In this mode,
// twoyi_loader_main() installs seccomp + SIGSYS and then calls main()
// directly (instead of returning an entry point to jump to).
//
// Source: AOSP bionic/linker/linker_main.cpp (auxv parsing pattern),
//         Linux kernel fs/binfmt_elf.c (stack layout),
//         include/uapi/linux/auxvec.h (AT_* constants).

#include "twoyi_loader.h"
#include "sigsys_handler.h"
#include "seccomp_filter.h"
#include "mount_table.h"

#include <stdint.h>
#include <unistd.h>
#include <string.h>
#include <stdio.h>

// Auxiliary vector types (from include/uapi/linux/auxvec.h).
#define AT_NULL   0
#define AT_PHDR   3
#define AT_PHNUM  5
#define AT_BASE   7
#define AT_ENTRY  9

// Parse the kernel argument block to find an auxv entry.
static uint64_t find_auxv(uint64_t *raw_stack, uint64_t type) {
    if (!raw_stack) return 0;

    uint64_t argc = *raw_stack;
    uint64_t *p = raw_stack + 1;

    // Skip argv
    p += argc;
    p++; // skip argv NULL terminator

    // Skip envp (scan for NULL)
    while (*p != 0) {
        p++;
    }
    p++; // skip envp NULL terminator

    // Scan auxv for the requested type
    while (1) {
        uint64_t a_type = *p;
        uint64_t a_val = *(p + 1);
        if (a_type == AT_NULL) {
            break;
        }
        if (a_type == type) {
            return a_val;
        }
        p += 2;
    }

    return 0;
}

// Write a string to a file descriptor (async-signal-safe).
static void write_str(int fd, const char *s) {
    if (s) {
        size_t len = 0;
        while (s[len]) len++;
        write(fd, s, len);
    }
}

// In combined test mode, main() is provided by the test guest.
// We declare it here so the loader can call it.
extern int main(int argc, char **argv, char **envp);

uint64_t twoyi_loader_main(uint64_t *raw_stack) {
    write_str(2, "[twoyi_loader] starting\n");

    // Parse the stack to get argc, argv, envp (for calling main()).
    uint64_t argc = raw_stack ? *raw_stack : 0;
    char **argv = (char **)(raw_stack + 1);
    char **envp = argv + argc + 1; // skip argv + NULL terminator

    // Find AT_ENTRY from auxv (for the real PT_INTERP mode).
    uint64_t guest_entry = find_auxv(raw_stack, AT_ENTRY);
    {
        char buf[32];
        const char hex[] = "0123456789abcdef";
        char *p = buf;
        *p++ = '0';
        *p++ = 'x';
        for (int i = 60; i >= 0; i -= 4) {
            *p++ = hex[(guest_entry >> i) & 0xf];
        }
        *p++ = '\n';
        write(2, buf, p - buf);
    }

    // Initialize the virtual mount table.
    twoyi_mount_table_init();
    write_str(2, "[twoyi_loader] mount table initialized\n");

    // Install the SIGSYS handler BEFORE the seccomp filter.
    if (twoyi_sigsys_handler_install() != 0) {
        write_str(2, "[twoyi_loader] FATAL: failed to install SIGSYS handler\n");
        _exit(1);
    }
    write_str(2, "[twoyi_loader] SIGSYS handler installed\n");

    // Install the seccomp BPF filter.
    if (twoyi_seccomp_install() != 0) {
        write_str(2, "[twoyi_loader] FATAL: failed to install seccomp filter\n");
        _exit(1);
    }
    write_str(2, "[twoyi_loader] seccomp filter installed\n");

    // In combined test mode: call main() directly.
    // In real PT_INTERP mode: return guest_entry for _start to jump to.
    write_str(2, "[twoyi_loader] calling main()\n");
    int ret = main((int)argc, argv, envp);
    // Flush stdout before _exit (which doesn't flush stdio buffers).
    fflush(NULL);
    _exit(ret);

    return guest_entry; // never reached in combined mode
}
