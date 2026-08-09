// twoyi_loader.c — Custom ELF interpreter for rootless Android virtualization.
//
// This is the REAL custom dynamic linker that the kernel loads as PT_INTERP
// for guest binaries. It:
//   1. Installs the SIGSYS handler (before any guest code runs)
//   2. Installs the seccomp BPF filter (traps mount, chroot, mknod, etc.)
//   3. Reads AT_ENTRY from auxv to find the guest's entry point
//   4. Jumps to the guest's entry point
//
// The guest then runs with seccomp active — any trapped syscall triggers
// SIGSYS, which is handled by our emulator.
//
// Architecture: x86_64 (arm64-v8a to follow)
//
// Source: AOSP bionic/linker/arch/x86_64/begin.S (_start pattern),
//         AOSP bionic/linker/linker_main.cpp (auxv parsing),
//         VM libkrloader64.so (custom interpreter pattern).
//
// BUILD: gcc -nostartfiles -shared -fPIC -Wl,-e,_start -o libtwoyi_loader.so \
//          twoyi_loader.c sigsys_handler.c seccomp_filter.c mount_table.c \
//          path_translation.c -lc
//
// INSTALL: Copy as /data/data/io.twoyi/rootfs/loader64
//          Set guest init's PT_INTERP to "./loader64" (via patchelf or
//          linker flags at rootfs build time)

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
#include <linux/audit.h>
#include <ucontext.h>

// =========================================================================
// Architecture-specific constants
// =========================================================================

#if defined(__x86_64__)
  #define TWOYI_AUDIT_ARCH 0xC000003EU  // AUDIT_ARCH_X86_64
  #define TWOYI_NR_mount    165
  #define TWOYI_NR_umount2  166
  #define TWOYI_NR_chroot   161
  #define TWOYI_NR_mknod    133
  #define TWOYI_NR_mknodat  259
  #define TWOYI_NR_openat   257
  #define TWOYI_NR_getpid   39
#elif defined(__aarch64__)
  #define TWOYI_AUDIT_ARCH 0xC00000B7U  // AUDIT_ARCH_AARCH64
  #define TWOYI_NR_mount    40
  #define TWOYI_NR_umount2  39
  #define TWOYI_NR_chroot   51
  #define TWOYI_NR_mknod    14
  #define TWOYI_NR_mknodat  33
  #define TWOYI_NR_openat   56
  #define TWOYI_NR_getpid   172
#endif

// =========================================================================
// Virtual mount table (real semantics, not return 0)
// =========================================================================

#define MAX_MOUNTS 32
struct mount_entry {
    char source[256];
    char target[256];
    char fstype[64];
    unsigned long flags;
    int active;
};
static struct mount_entry mount_table[MAX_MOUNTS];

static long emulate_mount(const char *source, const char *target,
                          const char *fstype, unsigned long flags, const void *data) {
    if (!target) return -EFAULT;

    // Special paths: /dev, /mnt, /storage — skip (no-op, like VM)
    if (strncmp(target, "/dev", 4) == 0 && (target[4] == '\0' || target[4] == '/')) return 0;
    if (strncmp(target, "/mnt", 4) == 0 && (target[4] == '\0' || target[4] == '/')) return 0;
    if (strncmp(target, "/storage", 8) == 0 && (target[8] == '\0' || target[8] == '/')) return 0;

    // Check if already mounted
    for (int i = 0; i < MAX_MOUNTS; i++) {
        if (mount_table[i].active && strncmp(mount_table[i].target, target, 256) == 0) {
            if (flags & 0x20) { // MS_REMOUNT
                mount_table[i].flags = flags;
                return 0;
            }
            return -EBUSY;
        }
    }

    // Find free slot
    for (int i = 0; i < MAX_MOUNTS; i++) {
        if (!mount_table[i].active) {
            if (source) strncpy(mount_table[i].source, source, 255);
            else mount_table[i].source[0] = '\0';
            strncpy(mount_table[i].target, target, 255);
            if (fstype) strncpy(mount_table[i].fstype, fstype, 63);
            else mount_table[i].fstype[0] = '\0';
            mount_table[i].flags = flags;
            mount_table[i].active = 1;
            return 0;
        }
    }
    return -ENOMEM;
}

static long emulate_umount2(const char *target, int flags) {
    if (!target) return -EFAULT;
    for (int i = 0; i < MAX_MOUNTS; i++) {
        if (mount_table[i].active && strncmp(mount_table[i].target, target, 256) == 0) {
            mount_table[i].active = 0;
            return 0;
        }
    }
    return -EINVAL;
}

// =========================================================================
// SIGSYS handler — receives trapped syscalls, emulates them
// =========================================================================

static volatile int sigsys_count = 0;

static void sigsys_handler(int sig, siginfo_t *info, void *ucontext) {
    (void)sig;
    ucontext_t *ctx = (ucontext_t *)ucontext;

    if (!info || info->si_code != 1) return; // SYS_SECCOMP

    long nr = info->si_syscall;
    sigsys_count++;

#if defined(__x86_64__)
    // x86_64: args in rdi, rsi, rdx, r10, r8, r9
    //         return value in rax (gregs[REG_RAX=13])
    #define GET_ARG(n) ({ \
        unsigned long _a; \
        switch(n) { \
            case 0: _a = ctx->uc_mcontext.gregs[8]; break;  /* REG_RDI */ \
            case 1: _a = ctx->uc_mcontext.gregs[9]; break;  /* REG_RSI */ \
            case 2: _a = ctx->uc_mcontext.gregs[12]; break; /* REG_RDX */ \
            case 3: _a = ctx->uc_mcontext.gregs[2]; break;  /* REG_R10 */ \
            case 4: _a = ctx->uc_mcontext.gregs[0]; break;  /* REG_R8 */ \
            case 5: _a = ctx->uc_mcontext.gregs[1]; break;  /* REG_R9 */ \
            default: _a = 0; break; \
        } _a; })
    #define SET_RET(val) ctx->uc_mcontext.gregs[13] = (long)(val) // REG_RAX
#elif defined(__aarch64__)
    // arm64: args in x0-x5 (regs[0]-regs[5])
    //        return value in x0 (regs[0])
    //        syscall nr in x8 (regs[8])
    #define GET_ARG(n) ((unsigned long)ctx->uc_mcontext.regs[n])
    #define SET_RET(val) ctx->uc_mcontext.regs[0] = (uint64_t)(val)
#endif

    long ret;

    switch (nr) {
        case TWOYI_NR_mount: {
            const char *source = (const char *)GET_ARG(0);
            const char *target = (const char *)GET_ARG(1);
            const char *fstype = (const char *)GET_ARG(2);
            unsigned long flags = GET_ARG(3);
            const void *data = (const void *)GET_ARG(4);
            ret = emulate_mount(source, target, fstype, flags, data);
            break;
        }
        case TWOYI_NR_umount2: {
            const char *target = (const char *)GET_ARG(0);
            int flags = (int)GET_ARG(1);
            ret = emulate_umount2(target, flags);
            break;
        }
        case TWOYI_NR_chroot:
            // VM implements this as a no-op (returns 0).
            // The chroot effect is achieved by path translation in openat.
            ret = 0;
            break;
        case TWOYI_NR_mknod:
        case TWOYI_NR_mknodat:
            // Init creates /dev/kmsg, /dev/random, etc.
            // Return 0 (success) — the host already has these devices.
            // Future: create AF_UNIX sockets for virtual devices.
            ret = 0;
            break;
        default:
            ret = -ENOSYS;
            break;
    }

    SET_RET(ret);
}

// =========================================================================
// Seccomp BPF filter installation
// =========================================================================

static int install_seccomp(void) {
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0)
        return -1;

    struct sock_filter filter[] = {
        // Check architecture
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, 4), // offset 4 = arch
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_AUDIT_ARCH, 1, 0),
        BPF_STMT(BPF_RET | BPF_K, 0x80000000), // KILL_PROCESS (wrong arch)

        // Check syscall number
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, 0), // offset 0 = nr

        // Trap mount
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_mount, 6, 0),
        // Trap umount2
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_umount2, 5, 0),
        // Trap chroot
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_chroot, 4, 0),
        // Trap mknod
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_mknod, 3, 0),
        // Trap mknodat
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_mknodat, 2, 0),

        // Default: allow
        BPF_STMT(BPF_RET | BPF_K, 0x7fff0000), // SECCOMP_RET_ALLOW

        // Trap targets
        BPF_STMT(BPF_RET | BPF_K, 0x00030000), // SECCOMP_RET_TRAP (mount)
        BPF_STMT(BPF_RET | BPF_K, 0x00030000), // SECCOMP_RET_TRAP (umount2)
        BPF_STMT(BPF_RET | BPF_K, 0x00030000), // SECCOMP_RET_TRAP (chroot)
        BPF_STMT(BPF_RET | BPF_K, 0x00030000), // SECCOMP_RET_TRAP (mknod)
        BPF_STMT(BPF_RET | BPF_K, 0x00030000), // SECCOMP_RET_TRAP (mknodat)
    };

    struct sock_fprog prog = {
        .len = sizeof(filter) / sizeof(filter[0]),
        .filter = filter,
    };

    // Try seccomp(2) first, then prctl(PR_SET_SECCOMP) fallback
    if (syscall(SYS_seccomp, SECCOMP_SET_MODE_FILTER, 0, &prog) != 0) {
        if (prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &prog, 0, 0) != 0)
            return -1;
    }
    return 0;
}

// =========================================================================
// SIGSYS handler installation
// =========================================================================

static int install_sigsys_handler(void) {
    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_sigaction = sigsys_handler;
    sa.sa_flags = SA_SIGINFO | SA_NODEFER;
    sigemptyset(&sa.sa_mask);
    if (sigaction(SIGSYS, &sa, NULL) != 0)
        return -1;

    // Unblock SIGSYS
    sigset_t mask;
    sigemptyset(&mask);
    sigaddset(&mask, SIGSYS);
    if (sigprocmask(SIG_UNBLOCK, &mask, NULL) != 0)
        return -1;

    return 0;
}

// =========================================================================
// Entry point — called from assembly _start
// =========================================================================

// AT_* constants (from include/uapi/linux/auxvec.h)
#define AT_NULL   0
#define AT_ENTRY  9

// Find a value in the auxiliary vector.
// The stack layout is: argc, argv[], NULL, envp[], NULL, auxv[].
static uint64_t find_auxv(uint64_t *stack, uint64_t type) {
    if (!stack) return 0;
    uint64_t argc = *stack;
    uint64_t *p = stack + 1 + argc + 1; // skip argc + argv + NULL
    while (*p) p++; // skip envp
    p++; // skip envp NULL
    while (1) {
        if (*p == AT_NULL) break;
        if (*p == type) return *(p + 1);
        p += 2;
    }
    return 0;
}

static void write_str(const char *s) {
    write(2, s, strlen(s));
}

uint64_t twoyi_loader_main(uint64_t *raw_stack) {
    write_str("[twoyi_loader] starting\n");

    // 1. Install SIGSYS handler (before seccomp filter)
    if (install_sigsys_handler() != 0) {
        write_str("[twoyi_loader] FATAL: sigsys handler install failed\n");
        _exit(1);
    }
    write_str("[twoyi_loader] SIGSYS handler installed\n");

    // 2. Install seccomp BPF filter
    if (install_seccomp() != 0) {
        write_str("[twoyi_loader] FATAL: seccomp install failed\n");
        _exit(1);
    }
    write_str("[twoyi_loader] seccomp filter installed\n");

    // 3. Find guest entry point from auxv
    uint64_t guest_entry = find_auxv(raw_stack, AT_ENTRY);
    if (guest_entry == 0) {
        write_str("[twoyi_loader] FATAL: AT_ENTRY not found\n");
        _exit(1);
    }

    write_str("[twoyi_loader] jumping to guest entry\n");
    return guest_entry;
}
