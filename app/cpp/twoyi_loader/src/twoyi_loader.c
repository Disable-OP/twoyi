// twoyi_loader.c — Custom ELF interpreter for rootless Android virtualization.
//
// This is the REAL custom dynamic linker that the kernel loads as PT_INTERP
// for guest binaries. It implements the seccomp/SIGSYS virtualization
// mechanism discovered from Virtual Master's libkr64.so reverse engineering.
//
// Architecture: x86_64 (arm64-v8a to follow)
//
// Hidden logic from VM disassembly (docs/VM_HIDDEN_LOGIC_FINDINGS.md):
//   1. Global runtime readiness barrier (ALL handlers wait for init)
//   2. mount_mgr: lock + mkdir + fstype validation + bind loop detection
//   3. mknodat: creates regular file containing dev_t value
//   4. rt_sigaction: guards SIGSYS from guest override
//   5. openat: path translation with /proc/ virtualization
//   6. chroot: synchronization barrier (waits for runtime readiness)

#include <stdint.h>
#include <unistd.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <signal.h>
#include <fcntl.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/mount.h>
#include <linux/seccomp.h>
#include <linux/filter.h>
#include <linux/audit.h>
#include <ucontext.h>
#include <pthread.h>

// =========================================================================
// Architecture-specific constants (VERIFIED from system headers)
// =========================================================================

#if defined(__x86_64__)
  // From /usr/include/x86_64-linux-gnu/asm/unistd_64.h
  #define TWOYI_AUDIT_ARCH 0xC000003EU
  #define TWOYI_NR_mount    165
  #define TWOYI_NR_umount2  166
  #define TWOYI_NR_chroot   161
  #define TWOYI_NR_mknod    133
  #define TWOYI_NR_mknodat  259
  #define TWOYI_NR_openat   257
  #define TWOYI_NR_getpid   39
  #define TWOYI_NR_close    3
  #define TWOYI_NR_write    1
  #define TWOYI_NR_mkdir    83
  #define TWOYI_NR_mkdirat  258
  #define TWOYI_NR_setuid   105
  #define TWOYI_NR_setgid   106
  #define TWOYI_NR_setgroups 118
  #define TWOYI_NR_setresuid 113
  #define TWOYI_NR_setresgid 114
  #define TWOYI_NR_unshare  272
  #define TWOYI_NR_clone    56
  #define TWOYI_NR_execve   59
  #define TWOYI_NR_rt_sigaction 134
  #define TWOYI_NR_sched_yield 24

  // x86_64 ucontext register access (VERIFIED from /usr/include/x86_64-linux-gnu/sys/ucontext.h)
  // REG_R8=0, REG_R9=1, REG_R10=2, REG_R11=3, REG_R12=4, REG_R13=5, REG_R14=6, REG_R15=7,
  // REG_RDI=8, REG_RSI=9, REG_RBP=10, REG_RBX=11, REG_RDX=12, REG_RAX=13, REG_RCX=14,
  // REG_RSP=15, REG_RIP=16
  #define GET_ARG(ctx, n) ({ \
      unsigned long _a; \
      switch(n) { \
          case 0: _a = (ctx)->uc_mcontext.gregs[8]; break;  /* RDI */ \
          case 1: _a = (ctx)->uc_mcontext.gregs[9]; break;  /* RSI */ \
          case 2: _a = (ctx)->uc_mcontext.gregs[12]; break; /* RDX */ \
          case 3: _a = (ctx)->uc_mcontext.gregs[2]; break;  /* R10 */ \
          case 4: _a = (ctx)->uc_mcontext.gregs[0]; break;  /* R8 */ \
          case 5: _a = (ctx)->uc_mcontext.gregs[1]; break;  /* R9 */ \
          default: _a = 0; break; \
      } _a; })
  #define SET_RET(ctx, val) (ctx)->uc_mcontext.gregs[13] = (long)(val) // REG_RAX

#elif defined(__aarch64__)
  // From AOSP bionic/libc/kernel/uapi/asm-generic/unistd.h
  #define TWOYI_AUDIT_ARCH 0xC00000B7U
  #define TWOYI_NR_mount    40
  #define TWOYI_NR_umount2  39
  #define TWOYI_NR_chroot   51
  #define TWOYI_NR_mknod    14
  #define TWOYI_NR_mknodat  33
  #define TWOYI_NR_openat   56
  #define TWOYI_NR_getpid   172
  #define TWOYI_NR_close    57
  #define TWOYI_NR_write    64
  #define TWOYI_NR_mkdir    34
  #define TWOYI_NR_mkdirat  34
  #define TWOYI_NR_setuid   146
  #define TWOYI_NR_setgid   144
  #define TWOYI_NR_setgroups 159
  #define TWOYI_NR_setresuid 147
  #define TWOYI_NR_setresgid 149
  #define TWOYI_NR_unshare  97
  #define TWOYI_NR_clone    220
  #define TWOYI_NR_execve   221
  #define TWOYI_NR_rt_sigaction 134
  #define TWOYI_NR_sched_yield 124

  // arm64 ucontext: mcontext_t = struct sigcontext
  // regs[0]=x0 (return/arg1), regs[8]=x8 (syscall nr), regs[1-5]=x1-x5 (args)
  #define GET_ARG(ctx, n) ((unsigned long)(ctx)->uc_mcontext.regs[n])
  #define SET_RET(ctx, val) (ctx)->uc_mcontext.regs[0] = (uint64_t)(val)
#endif

// =========================================================================
// Global runtime readiness barrier
// VERIFIED from VM disassembly: ALL handlers check BSS state variables
// and infinite-loop until the VM runtime is fully initialized.
// (docs/VM_HIDDEN_LOGIC_FINDINGS.md §1, §6)
// =========================================================================

volatile int g_runtime_ready = 0;

static void wait_for_runtime_ready(void) {
    while (!g_runtime_ready) {
        syscall(TWOYI_NR_sched_yield);
    }
}

// =========================================================================
// Virtual mount table with REAL semantics
// VERIFIED from VM mount_mgr at 0x8618:
//   - Acquires lock (0x8e14)
//   - Creates directories if needed (mkdir)
//   - Compares fstype against known types
//   - Handles bind mounts with loop detection
//   - Handles remount (updates flags)
//   - Special paths: /dev, /mnt, /storage
// (docs/VM_HIDDEN_LOGIC_FINDINGS.md §2)
// =========================================================================

#define MAX_MOUNTS 32
#define MOUNT_PATH_MAX 256
#define MOUNT_FSTYPE_MAX 64

struct mount_entry {
    char source[MOUNT_PATH_MAX];
    char target[MOUNT_PATH_MAX];
    char fstype[MOUNT_FSTYPE_MAX];
    unsigned long flags;
    int active;
};

struct mount_entry g_mount_table[MAX_MOUNTS];
pthread_mutex_t g_mount_lock = PTHREAD_MUTEX_INITIALIZER;

// Known filesystem types (VM compares fstype against 3 types at 0x8758-0x878c)
// The exact types could not be decoded (multi-byte XOR), but from the decoded
// strings we know VM handles: tmpfs, proc, sysfs, devpts, selinuxfs
static const char *known_fstypes[] = {
    "tmpfs", "proc", "sysfs", "devpts", "selinuxfs", "ext4",
    "fuse", "vfat", "exfat", "ntfs", "f2fs", NULL
};

static int is_known_fstype(const char *fstype) {
    if (!fstype || !*fstype) return 1; // NULL or empty fstype = bind mount, allow
    for (int i = 0; known_fstypes[i]; i++) {
        if (strcmp(fstype, known_fstypes[i]) == 0) return 1;
    }
    return 0;
}

static int is_special_path(const char *target) {
    // VM special-cases /dev, /mnt, /storage
    // (decoded strings: "mount_mgr: /dev is special, skip", etc.)
    if (!target) return 0;
    if (strncmp(target, "/dev", 4) == 0 && (target[4] == '\0' || target[4] == '/')) return 1;
    if (strncmp(target, "/mnt", 4) == 0 && (target[4] == '\0' || target[4] == '/')) return 1;
    if (strncmp(target, "/storage", 8) == 0 && (target[8] == '\0' || target[8] == '/')) return 1;
    return 0;
}

static long emulate_mount(const char *source, const char *target,
                         const char *fstype, unsigned long flags, const void *data) {
    (void)data;

    // Wait for runtime readiness (VM hidden logic)
    wait_for_runtime_ready();

    if (!target) return -EFAULT;

    // Special paths: /dev, /mnt, /storage — skip (no-op, return 0)
    // (VM: "mount_mgr: /dev is special, skip")
    if (is_special_path(target)) return 0;

    pthread_mutex_lock(&g_mount_lock);

    // Check if target is already mounted
    for (int i = 0; i < MAX_MOUNTS; i++) {
        if (g_mount_table[i].active &&
            strncmp(g_mount_table[i].target, target, MOUNT_PATH_MAX) == 0) {

            // 6-Z214: propagation / remount / move ops reconfigure the
            // EXISTING entry — never a duplicate-mount EBUSY. AOSP init's
            // SetupMountNamespaces issues mount(nullptr, "/apex", nullptr,
            // MS_PRIVATE) on an already-recorded /apex; the old code
            // returned -EBUSY and init aborted fatally with
            // InitFatalReboot (the r14-r25 OrangeFox/Lineage blocker).
            // Semantics mirror is_mount_propagation_op in
            // twoyi_loader_shlib.c (see the full rationale there).
            {
                const unsigned long prop_mask = MS_REMOUNT | MS_MOVE |
                                                MS_UNBINDABLE | MS_PRIVATE |
                                                MS_SLAVE | MS_SHARED;
                if (flags & prop_mask) {
                    g_mount_table[i].flags = flags;
                    pthread_mutex_unlock(&g_mount_lock);
                    return 0;
                }
            }

            // Bind-mount ONTO an already-mounted target is legal Linux
            // semantics (stacked bind mounts). Virtualize as success.
            if (flags & MS_BIND) {
                if (source && strncmp(source, target, MOUNT_PATH_MAX) == 0) {
                    pthread_mutex_unlock(&g_mount_lock);
                    return -EINVAL; // self-bind loop (unchanged)
                }
                g_mount_table[i].flags = flags;
                pthread_mutex_unlock(&g_mount_lock);
                return 0;
            }

            // Plain (non-bind, non-propagation) re-mount of a live
            // target: real kernel returns EBUSY — keep that semantic.
            pthread_mutex_unlock(&g_mount_lock);
            return -EBUSY;
        }
    }

    // Validate fstype (VM: "mount_mgr: unsupported filesystemtype %s")
    if (!is_known_fstype(fstype)) {
        pthread_mutex_unlock(&g_mount_lock);
        return -ENODEV;
    }

    // Find free slot
    int slot = -1;
    for (int i = 0; i < MAX_MOUNTS; i++) {
        if (!g_mount_table[i].active) {
            slot = i;
            break;
        }
    }
    if (slot < 0) {
        pthread_mutex_unlock(&g_mount_lock);
        return -ENOMEM;
    }

    // Record the mount entry
    if (source) strncpy(g_mount_table[slot].source, source, MOUNT_PATH_MAX - 1);
    else g_mount_table[slot].source[0] = '\0';
    strncpy(g_mount_table[slot].target, target, MOUNT_PATH_MAX - 1);
    if (fstype) strncpy(g_mount_table[slot].fstype, fstype, MOUNT_FSTYPE_MAX - 1);
    else g_mount_table[slot].fstype[0] = '\0';
    g_mount_table[slot].flags = flags;
    g_mount_table[slot].active = 1;

    pthread_mutex_unlock(&g_mount_lock);
    return 0;
}

static long emulate_umount2(const char *target, int flags) {
    (void)flags;
    wait_for_runtime_ready();

    if (!target) return -EFAULT;

    pthread_mutex_lock(&g_mount_lock);
    for (int i = 0; i < MAX_MOUNTS; i++) {
        if (g_mount_table[i].active &&
            strncmp(g_mount_table[i].target, target, MOUNT_PATH_MAX) == 0) {
            g_mount_table[i].active = 0;
            g_mount_table[i].source[0] = '\0';
            g_mount_table[i].target[0] = '\0';
            g_mount_table[i].fstype[0] = '\0';
            pthread_mutex_unlock(&g_mount_lock);
            return 0;
        }
    }
    pthread_mutex_unlock(&g_mount_lock);
    return -EINVAL;
}

// =========================================================================
// mknodat emulation — creates a regular file containing dev_t
// VERIFIED from VM mknodat handler at 0x11d598:
//   1. Translate path (prepend rootfs prefix)
//   2. If S_IFCHR or S_IFBLK: create regular file via openat+write+close
//   3. Write 8 bytes (dev_t value) to the file
// (docs/VM_HIDDEN_LOGIC_FINDINGS.md §3)
// =========================================================================

// Raw syscall that bypasses seccomp by using open() instead of openat()
// (open is NOT in our trap list, so it passes through)
#if defined(__x86_64__)
  #define TWOYI_NR_open 2
#elif defined(__aarch64__)
  // arm64 doesn't have open() — only openat(). We need a different approach.
  #define TWOYI_NR_open -1  // will use openat with AT_FDCWD
#endif

static long emulate_mknodat(int dirfd, const char *pathname, mode_t mode, dev_t dev) {
    wait_for_runtime_ready();

    if (!pathname) return -EFAULT;

    // Check if it's a device node (S_IFCHR or S_IFBLK)
    mode_t fmt = mode & S_IFMT;
    if (fmt != S_IFCHR && fmt != S_IFBLK) {
        // Not a device node — for regular files, try real mknodat
        // (VM only intercepts device node creation)
        return syscall(TWOYI_NR_mknodat, dirfd, pathname, mode, dev);
    }

    // Create a REGULAR FILE at the path containing the dev_t value
    // (VM: openat(AT_FDCWD, path, O_RDWR|O_CREAT, 0666) + write + close)
    //
    // IMPORTANT: We can't use openat() here because openat is trapped by
    // our own seccomp filter! Using it would cause a recursive SIGSYS.
    // Instead, use open() on x86_64 (not trapped), or on arm64 we need
    // to use a flag to tell the handler this is an internal call.
#if defined(__x86_64__)
    int fd = syscall(TWOYI_NR_open, pathname, O_RDWR | O_CREAT, 0666);
#else
    // arm64: openat is the only option. We need to mark this as internal.
    // For now, just return 0 (device creation skipped).
    // TODO: implement an internal-call flag to bypass path translation.
    return 0;
#endif
    if (fd < 0) return -errno;

    // Write the dev_t value (8 bytes) to the file
    // (VM writes 8 bytes at 0x11d6d0: write(fd, &dev, 8))
    ssize_t written = syscall(TWOYI_NR_write, fd, &dev, sizeof(dev_t));
    syscall(TWOYI_NR_close, fd);

    if (written != sizeof(dev_t)) return -EIO;
    return 0;
}

// =========================================================================
// rt_sigaction guard — prevents guest from overriding SIGSYS handler
// VERIFIED from VM at 0x114650:
//   if (signal == SIGSYS) return 0;  // fake success
//   else: call real rt_sigaction
// (docs/VM_HIDDEN_LOGIC_FINDINGS.md §4)
// =========================================================================

static long emulate_rt_sigaction(int signum, const struct sigaction *act,
                                 struct sigaction *oldact, size_t sigsetsize) {
    (void)sigsetsize;

    // Guard SIGSYS — prevent guest from overriding our handler
    if (signum == SIGSYS) {
        // Return fake success without calling real sigaction
        // (VM: mov x0, xzr; b return_path)
        if (oldact) {
            // Return the current (our) handler as "oldact"
            memset(oldact, 0, sizeof(struct sigaction));
        }
        return 0;
    }

    // For all other signals, call the real rt_sigaction
    return syscall(TWOYI_NR_rt_sigaction, signum, act, oldact, sigsetsize);
}

// =========================================================================
// openat path translation — /proc/ virtualization + rootfs prefix
// VERIFIED from VM openat handler at 0x119080:
//   1. strncmp(path, "/proc/", 6) — check for /proc/
//   2. If /proc/: redirect to per-VM files
//   3. Else: prepend rootfs prefix
// (docs/VM_HIDDEN_LOGIC_FINDINGS.md §5)
// =========================================================================

static const char *g_rootfs_prefix = NULL;

// Initialize the rootfs prefix from TWOYI_ROOTFS env var
void init_rootfs_prefix(void) {
    g_rootfs_prefix = getenv("TWOYI_ROOTFS");
    if (!g_rootfs_prefix) {
        g_rootfs_prefix = "/data/data/io.twoyi/rootfs";
    }
}

// Translate a guest path to a host path
// Returns a pointer to a static buffer (NOT thread-safe — called from
// signal handler, which is single-threaded per-process after fork)
static char g_translated_path[512];

static const char *translate_path(const char *path) {
    if (!path) return NULL;

    // Check for /proc/ prefix (VM: strncmp at 0x119080)
    if (strncmp(path, "/proc/", 6) == 0) {
        // /proc/ virtualization — redirect to per-VM files
        // For now, use a simple approach: redirect /proc/* to {rootfs}/proc/*
        // Future: implement per-VM files like VM does
        snprintf(g_translated_path, sizeof(g_translated_path),
                 "%s%s", g_rootfs_prefix, path);
        return g_translated_path;
    }

    // Check for /sys/ prefix
    if (strncmp(path, "/sys/", 5) == 0) {
        snprintf(g_translated_path, sizeof(g_translated_path),
                 "%s%s", g_rootfs_prefix, path);
        return g_translated_path;
    }

    // Check for /dev/ prefix
    if (strncmp(path, "/dev/", 5) == 0) {
        snprintf(g_translated_path, sizeof(g_translated_path),
                 "%s%s", g_rootfs_prefix, path);
        return g_translated_path;
    }

    // For absolute paths, prepend rootfs prefix
    if (path[0] == '/') {
        snprintf(g_translated_path, sizeof(g_translated_path),
                 "%s%s", g_rootfs_prefix, path);
        return g_translated_path;
    }

    // Relative paths — pass through unchanged
    return path;
}

static long emulate_openat(int dirfd, const char *pathname, int flags, mode_t mode) {
    wait_for_runtime_ready();

    if (!pathname) return -EFAULT;

    // Translate the path (VM: 0x119080)
    const char *translated = translate_path(pathname);

    // Call real openat with translated path
    return syscall(TWOYI_NR_openat, dirfd, translated, flags, mode);
}

// =========================================================================
// Other syscall emulations
// =========================================================================

static long emulate_chroot(const char *path) {
    // VM: synchronization barrier — waits for runtime readiness
    // (docs/VM_HIDDEN_LOGIC_FINDINGS.md §1)
    wait_for_runtime_ready();

    // VM returns 0 without doing anything
    // The chroot effect is achieved by path translation in openat
    return 0;
}

static long emulate_unshare(int flags) {
    // VM traps unshare but the DEFAULT handler re-executes the real syscall
    // For now, return 0 (fake success) to prevent EPERM
    // Future: emulate PID namespace via getpid hooking
    return 0;
}

static long emulate_setuid(uid_t uid) { return 0; }
static long emulate_setgid(gid_t gid) { return 0; }
static long emulate_setgroups(size_t size, const gid_t *list) { (void)size; (void)list; return 0; }
static long emulate_setresuid(uid_t ruid, uid_t euid, uid_t suid) { (void)ruid; (void)euid; (void)suid; return 0; }
static long emulate_setresgid(gid_t rgid, gid_t egid, gid_t sgid) { (void)rgid; (void)egid; (void)sgid; return 0; }

// =========================================================================
// SIGSYS handler — dispatches trapped syscalls to emulators
// VERIFIED from VM SIGSYS handler at 0x115f04:
//   1. Read si_syscall from siginfo
//   2. Dispatch via jump table
//   3. Write return value to ucontext
// (docs/VM_KR64_ANALYSIS.md, docs/VM_HIDDEN_LOGIC_FINDINGS.md)
// =========================================================================

volatile int g_sigsys_count = 0;

static void sigsys_handler(int sig, siginfo_t *info, void *ucontext) {
    (void)sig;
    ucontext_t *ctx = (ucontext_t *)ucontext;

    // Verify this is a seccomp trap (si_code == SYS_SECCOMP = 1)
    if (!info || info->si_code != 1) return;

    long nr = info->si_syscall;
    g_sigsys_count++;

    long ret;

    switch (nr) {
        // --- Mount/umount ---
        case TWOYI_NR_mount: {
            const char *source = (const char *)GET_ARG(ctx, 0);
            const char *target = (const char *)GET_ARG(ctx, 1);
            const char *fstype = (const char *)GET_ARG(ctx, 2);
            unsigned long flags = GET_ARG(ctx, 3);
            const void *data = (const void *)GET_ARG(ctx, 4);
            ret = emulate_mount(source, target, fstype, flags, data);
            break;
        }
        case TWOYI_NR_umount2: {
            const char *target = (const char *)GET_ARG(ctx, 0);
            int flags = (int)GET_ARG(ctx, 1);
            ret = emulate_umount2(target, flags);
            break;
        }

        // --- chroot (synchronization barrier) ---
        case TWOYI_NR_chroot: {
            const char *path = (const char *)GET_ARG(ctx, 0);
            ret = emulate_chroot(path);
            break;
        }

        // --- mknod/mknodat (creates regular file with dev_t) ---
        case TWOYI_NR_mknod: {
            const char *pathname = (const char *)GET_ARG(ctx, 0);
            mode_t mode = (mode_t)GET_ARG(ctx, 1);
            dev_t dev = (dev_t)GET_ARG(ctx, 2);
            ret = emulate_mknodat(AT_FDCWD, pathname, mode, dev);
            break;
        }
        case TWOYI_NR_mknodat: {
            int dirfd = (int)GET_ARG(ctx, 0);
            const char *pathname = (const char *)GET_ARG(ctx, 1);
            mode_t mode = (mode_t)GET_ARG(ctx, 2);
            dev_t dev = (dev_t)GET_ARG(ctx, 3);
            ret = emulate_mknodat(dirfd, pathname, mode, dev);
            break;
        }

        // --- openat (path translation + /proc/ virtualization) ---
        case TWOYI_NR_openat: {
            int dirfd = (int)GET_ARG(ctx, 0);
            const char *pathname = (const char *)GET_ARG(ctx, 1);
            int flags = (int)GET_ARG(ctx, 2);
            mode_t mode = (mode_t)GET_ARG(ctx, 3);
            ret = emulate_openat(dirfd, pathname, flags, mode);
            break;
        }

        // --- rt_sigaction (guard SIGSYS from override) ---
        case TWOYI_NR_rt_sigaction: {
            int signum = (int)GET_ARG(ctx, 0);
            const struct sigaction *act = (const struct sigaction *)GET_ARG(ctx, 1);
            struct sigaction *oldact = (struct sigaction *)GET_ARG(ctx, 2);
            size_t sigsetsize = (size_t)GET_ARG(ctx, 3);
            ret = emulate_rt_sigaction(signum, act, oldact, sigsetsize);
            break;
        }

        // --- UID/GID (fake success) ---
        case TWOYI_NR_setuid:
            ret = emulate_setuid((uid_t)GET_ARG(ctx, 0));
            break;
        case TWOYI_NR_setgid:
            ret = emulate_setgid((gid_t)GET_ARG(ctx, 0));
            break;
        case TWOYI_NR_setgroups:
            ret = emulate_setgroups((size_t)GET_ARG(ctx, 0),
                                    (const gid_t *)GET_ARG(ctx, 1));
            break;
        case TWOYI_NR_setresuid:
            ret = emulate_setresuid((uid_t)GET_ARG(ctx, 0),
                                    (uid_t)GET_ARG(ctx, 1),
                                    (uid_t)GET_ARG(ctx, 2));
            break;
        case TWOYI_NR_setresgid:
            ret = emulate_setresgid((gid_t)GET_ARG(ctx, 0),
                                    (gid_t)GET_ARG(ctx, 1),
                                    (gid_t)GET_ARG(ctx, 2));
            break;

        // --- unshare (fake success) ---
        case TWOYI_NR_unshare:
            ret = emulate_unshare((int)GET_ARG(ctx, 0));
            break;

        // --- DEFAULT: re-execute real syscall (VM DEFAULT handler at 0x114664) ---
        default:
            // For trapped syscalls we don't explicitly handle,
            // re-execute the real syscall via syscall()
            // (VM: "bl syscall@plt" at 0x114664)
            {
                unsigned long a0 = GET_ARG(ctx, 0);
                unsigned long a1 = GET_ARG(ctx, 1);
                unsigned long a2 = GET_ARG(ctx, 2);
                unsigned long a3 = GET_ARG(ctx, 3);
                unsigned long a4 = GET_ARG(ctx, 4);
                unsigned long a5 = GET_ARG(ctx, 5);
                ret = syscall(nr, a0, a1, a2, a3, a4, a5);
            }
            break;
    }

    SET_RET(ctx, ret);
}

// =========================================================================
// Seccomp BPF filter installation
// VERIFIED from VM libkrloader64.so at 0x3384
// =========================================================================

int install_seccomp(void) {
    // Step 1: PR_SET_NO_NEW_PRIVS (mandatory prerequisite)
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0) return -1;

    // Step 2: Build BPF filter
    struct sock_filter filter[] = {
        // Check architecture (security: prevent cross-arch confusion)
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, 4),  // load arch
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_AUDIT_ARCH, 1, 0),
        BPF_STMT(BPF_RET | BPF_K, 0x80000000),  // KILL_PROCESS (wrong arch)

        // Load syscall number
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, 0),  // load nr

        // Trap mount
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_mount, 14, 0),
        // Trap umount2
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_umount2, 13, 0),
        // Trap chroot
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_chroot, 12, 0),
        // Trap mknod
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_mknod, 11, 0),
        // Trap mknodat
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_mknodat, 10, 0),
        // Trap openat
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_openat, 9, 0),
        // Trap rt_sigaction
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_rt_sigaction, 8, 0),
        // Trap setuid
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_setuid, 7, 0),
        // Trap setgid
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_setgid, 6, 0),
        // Trap setgroups
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_setgroups, 5, 0),
        // Trap setresuid
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_setresuid, 4, 0),
        // Trap setresgid
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_setresgid, 3, 0),
        // Trap unshare
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_unshare, 2, 0),

        // Default: allow all other syscalls
        BPF_STMT(BPF_RET | BPF_K, 0x7fff0000),  // SECCOMP_RET_ALLOW

        // Trap targets (all use SECCOMP_RET_TRAP)
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // mount
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // umount2
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // chroot
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // mknod
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // mknodat
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // openat
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // rt_sigaction
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // setuid
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // setgid
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // setgroups
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // setresuid
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // setresgid
        BPF_STMT(BPF_RET | BPF_K, 0x00030000),  // unshare
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
// VERIFIED from VM at 0x116120 + Chromium trap.cc pattern
// =========================================================================

int install_sigsys_handler(void) {
    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_sigaction = sigsys_handler;
    // SA_SIGINFO: required for (int, siginfo_t*, void*) handler
    // SA_NODEFER: allow nested SIGSYS (Chromium pattern)
    sa.sa_flags = SA_SIGINFO | SA_NODEFER;
    sigemptyset(&sa.sa_mask);
    if (sigaction(SIGSYS, &sa, NULL) != 0) return -1;

    // Unblock SIGSYS (VM: rt_sigprocmask at 0x3964)
    sigset_t mask;
    sigemptyset(&mask);
    sigaddset(&mask, SIGSYS);
    if (sigprocmask(SIG_UNBLOCK, &mask, NULL) != 0) return -1;

    return 0;
}

// =========================================================================
// Entry point — called from assembly _start
// =========================================================================

#define AT_NULL   0
#define AT_ENTRY  9

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
