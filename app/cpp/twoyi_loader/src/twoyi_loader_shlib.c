// twoyi_loader_shlib.c — Shared library version of the Twoyi loader.
//
// This is loaded via LD_PRELOAD (in the KVM test environment) or as
// DT_NEEDED of a custom PT_INTERP linker (in production).
//
// The .init_array constructor installs seccomp + SIGSYS handler BEFORE
// the guest's main() runs. This is the same pattern VM uses:
// libkr64.so's .init_array installs the filter before init's main().
//
// Key difference from twoyi_loader.c:
//   - No _start stub (the system linker provides it)
//   - .init_array constructor does the installation
//   - openat is NOT trapped by seccomp (avoids recursion)
//   - openat path translation is done via PLT interposition (LD_PRELOAD)
//
// Architecture: x86_64 (arm64-v8a to follow)

#include <stdint.h>
#include <unistd.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <signal.h>
#include <fcntl.h>
#include <stdarg.h>
#include <dlfcn.h>
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

// _GNU_SOURCE needed for RTLD_NEXT
#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif

// =========================================================================
// Architecture-specific constants (VERIFIED from system headers)
// =========================================================================

#if defined(__x86_64__)
  #define TWOYI_AUDIT_ARCH 0xC000003EU
  #define NR_mount    165
  #define NR_umount2  166
  #define NR_chroot   161
  #define NR_mknod    133
  #define NR_mknodat  259
  #define NR_openat   257
  #define NR_open     2
  #define NR_close    3
  #define NR_write    1
  #define NR_getpid   39
  #define NR_setuid   105
  #define NR_setgid   106
  #define NR_setgroups 118
  #define NR_setresuid 113
  #define NR_setresgid 114
  #define NR_unshare  272
  #define NR_rt_sigaction 134
  #define NR_sched_yield 24

  #define GET_ARG(ctx, n) ({ \
      unsigned long _a; \
      switch(n) { \
          case 0: _a = (ctx)->uc_mcontext.gregs[8]; break;  \
          case 1: _a = (ctx)->uc_mcontext.gregs[9]; break;  \
          case 2: _a = (ctx)->uc_mcontext.gregs[12]; break; \
          case 3: _a = (ctx)->uc_mcontext.gregs[2]; break;  \
          case 4: _a = (ctx)->uc_mcontext.gregs[0]; break;  \
          case 5: _a = (ctx)->uc_mcontext.gregs[1]; break;  \
          default: _a = 0; break; \
      } _a; })
  #define SET_RET(ctx, val) (ctx)->uc_mcontext.gregs[13] = (long)(val)
#elif defined(__aarch64__)
  #define TWOYI_AUDIT_ARCH 0xC00000B7U
  #define NR_mount    40
  #define NR_umount2  39
  #define NR_chroot   51
  #define NR_mknod    14
  #define NR_mknodat  33
  #define NR_openat   56
  #define NR_close    57
  #define NR_write    64
  #define NR_getpid   172
  #define NR_setuid   146
  #define NR_setgid   144
  #define NR_setgroups 159
  #define NR_setresuid 147
  #define NR_setresgid 149
  #define NR_unshare  97
  #define NR_rt_sigaction 134
  #define NR_sched_yield 124

  #define GET_ARG(ctx, n) ((unsigned long)(ctx)->uc_mcontext.regs[n])
  #define SET_RET(ctx, val) (ctx)->uc_mcontext.regs[0] = (uint64_t)(val)
#endif

// Helper
static void write_str(int fd, const char *s) {
    if (s) { size_t l=0; while(s[l])l++; write(fd,s,l); }
}

// =========================================================================
// Global state
// =========================================================================

volatile int g_runtime_ready = 0;
volatile int g_sigsys_count = 0;
static const char *g_rootfs = NULL;

// Mount table
#define MAX_MOUNTS 32
struct mount_entry {
    char source[256]; char target[256]; char fstype[64];
    unsigned long flags; int active;
};
static struct mount_entry g_mounts[MAX_MOUNTS];
static pthread_mutex_t g_mount_lock = PTHREAD_MUTEX_INITIALIZER;

// =========================================================================
// Runtime readiness barrier (VM hidden logic: BSS state vars + infinite loop)
// =========================================================================
static void wait_ready(void) {
    while (!g_runtime_ready) syscall(NR_sched_yield);
}

// =========================================================================
// Mount emulation (VM mount_mgr at 0x8618)
// =========================================================================
static long emu_mount(const char *src, const char *tgt, const char *fs,
                      unsigned long flags, const void *data) {
    (void)data; wait_ready();
    if (!tgt) return -EFAULT;
    // Special paths
    if ((strncmp(tgt,"/dev",4)==0 && (tgt[4]==0||tgt[4]=='/')) ||
        (strncmp(tgt,"/mnt",4)==0 && (tgt[4]==0||tgt[4]=='/')) ||
        (strncmp(tgt,"/storage",8)==0 && (tgt[8]==0||tgt[8]=='/')))
        return 0;
    pthread_mutex_lock(&g_mount_lock);
    for (int i=0;i<MAX_MOUNTS;i++) {
        if (g_mounts[i].active && strncmp(g_mounts[i].target,tgt,256)==0) {
            if (flags & MS_REMOUNT) { g_mounts[i].flags=flags; pthread_mutex_unlock(&g_mount_lock); return 0; }
            if ((flags & MS_BIND) && src && strncmp(src,tgt,256)==0) { pthread_mutex_unlock(&g_mount_lock); return -EINVAL; }
            pthread_mutex_unlock(&g_mount_lock); return -EBUSY;
        }
    }
    int slot=-1;
    for (int i=0;i<MAX_MOUNTS;i++) if(!g_mounts[i].active){slot=i;break;}
    if (slot<0) { pthread_mutex_unlock(&g_mount_lock); return -ENOMEM; }
    if(src) strncpy(g_mounts[slot].source,src,255); else g_mounts[slot].source[0]=0;
    strncpy(g_mounts[slot].target,tgt,255);
    if(fs) strncpy(g_mounts[slot].fstype,fs,63); else g_mounts[slot].fstype[0]=0;
    g_mounts[slot].flags=flags; g_mounts[slot].active=1;
    pthread_mutex_unlock(&g_mount_lock);
    return 0;
}

static long emu_umount2(const char *tgt, int flags) {
    (void)flags; wait_ready();
    if(!tgt) return -EFAULT;
    pthread_mutex_lock(&g_mount_lock);
    for(int i=0;i<MAX_MOUNTS;i++) {
        if(g_mounts[i].active && strncmp(g_mounts[i].target,tgt,256)==0) {
            g_mounts[i].active=0; pthread_mutex_unlock(&g_mount_lock); return 0;
        }
    }
    pthread_mutex_unlock(&g_mount_lock); return -EINVAL;
}

// =========================================================================
// mknodat emulation (VM at 0x11d598: creates regular file with dev_t)
// =========================================================================
static long emu_mknodat(int dirfd, const char *path, mode_t mode, dev_t dev) {
    wait_ready();
    if(!path) return -EFAULT;
    mode_t fmt = mode & S_IFMT;
    if (fmt != S_IFCHR && fmt != S_IFBLK)
        return syscall(NR_mknodat, dirfd, path, mode, dev);
    // Create regular file containing dev_t (use open() not openat() to avoid seccomp recursion)
#if defined(__x86_64__)
    int fd = syscall(NR_open, path, O_RDWR|O_CREAT, 0666);
    if(fd<0) return -errno;
    syscall(NR_write, fd, &dev, sizeof(dev_t));
    syscall(NR_close, fd);
#else
    // arm64: no open() syscall, return 0 for now (TODO: use internal flag)
    (void)dev;
#endif
    return 0;
}

// =========================================================================
// rt_sigaction guard (VM at 0x114650: prevent SIGSYS override)
// =========================================================================
static long emu_rt_sigaction(int sig, const struct sigaction *act,
                             struct sigaction *old, size_t sz) {
    (void)sz;
    if (sig == SIGSYS) { if(old) memset(old,0,sizeof(*old)); return 0; }
    return syscall(NR_rt_sigaction, sig, act, old, sz);
}

// =========================================================================
// openat PLT interposition (path translation)
// VM uses shadowhook for openat; we use LD_PRELOAD PLT interposition.
// This is NOT in the seccomp trap list — it's a libc wrapper hook.
// =========================================================================

// Real openat pointer (resolved via dlsym)
static int (*real_openat)(int, const char *, int, ...) = NULL;

static void init_real_funcs(void) {
    if (!real_openat) {
        real_openat = dlsym(RTLD_NEXT, "openat");
    }
}

// Path translation: prepend rootfs prefix for absolute paths
static char g_translated[512];
static const char *translate(const char *path) {
    if (!path || !g_rootfs) return path;
    if (path[0] != '/') return path; // relative
    // Don't translate paths that are already under rootfs
    if (strncmp(path, g_rootfs, strlen(g_rootfs)) == 0) return path;
    snprintf(g_translated, sizeof(g_translated), "%s%s", g_rootfs, path);
    return g_translated;
}

// openat PLT interposition
int openat(int dirfd, const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
    init_real_funcs();
    if (!real_openat) return syscall(NR_openat, dirfd, path, flags, mode);
    const char *translated = translate(path);
    return real_openat(dirfd, translated, flags, mode);
}

// open PLT interposition (for code that uses open() instead of openat())
int open(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
    init_real_funcs();
    const char *translated = translate(path);
#if defined(__x86_64__)
    return syscall(NR_open, translated, flags, mode);
#else
    if (real_openat) return real_openat(AT_FDCWD, translated, flags, mode);
    return syscall(NR_openat, AT_FDCWD, translated, flags, mode);
#endif
}

// =========================================================================
// SIGSYS handler
// =========================================================================
static void sigsys_handler(int sig, siginfo_t *info, void *uc) {
    (void)sig;
    ucontext_t *ctx = (ucontext_t *)uc;
    if (!info || info->si_code != 1) return;
    long nr = info->si_syscall;
    g_sigsys_count++;
    long ret;
    switch (nr) {
        case NR_mount: {
            ret = emu_mount((const char*)GET_ARG(ctx,0), (const char*)GET_ARG(ctx,1),
                           (const char*)GET_ARG(ctx,2), GET_ARG(ctx,3), (const void*)GET_ARG(ctx,4));
            break;
        }
        case NR_umount2: ret = emu_umount2((const char*)GET_ARG(ctx,0), (int)GET_ARG(ctx,1)); break;
        case NR_chroot: wait_ready(); ret = 0; break;
        case NR_mknod: ret = emu_mknodat(AT_FDCWD, (const char*)GET_ARG(ctx,0), (mode_t)GET_ARG(ctx,1), (dev_t)GET_ARG(ctx,2)); break;
        case NR_mknodat: ret = emu_mknodat((int)GET_ARG(ctx,0), (const char*)GET_ARG(ctx,1), (mode_t)GET_ARG(ctx,2), (dev_t)GET_ARG(ctx,3)); break;
        case NR_rt_sigaction: ret = emu_rt_sigaction((int)GET_ARG(ctx,0), (const struct sigaction*)GET_ARG(ctx,1), (struct sigaction*)GET_ARG(ctx,2), (size_t)GET_ARG(ctx,3)); break;
        case NR_setuid: case NR_setgid: case NR_setgroups:
        case NR_setresuid: case NR_setresgid: case NR_unshare:
            ret = 0; break;
        case NR_getpid:
            // init requires getpid() == 1 (exit 31 otherwise)
            // The LD_PRELOAD getpid_hook catches libc calls, but init might
            // use a direct syscall. Trap it here to be sure.
            ret = 1; break;
        default:
            ret = syscall(nr, GET_ARG(ctx,0), GET_ARG(ctx,1), GET_ARG(ctx,2),
                         GET_ARG(ctx,3), GET_ARG(ctx,4), GET_ARG(ctx,5));
            break;
    }
    SET_RET(ctx, ret);
}

// =========================================================================
// Seccomp BPF filter (traps ONLY privileged syscalls, NOT openat)
// openat is handled via PLT interposition above
// =========================================================================
static int install_seccomp(void) {
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0) return -1;
    struct sock_filter filter[] = {
        BPF_STMT(BPF_LD|BPF_W|BPF_ABS, 4),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K, TWOYI_AUDIT_ARCH, 1, 0),
        BPF_STMT(BPF_RET|BPF_K, 0x80000000),
        BPF_STMT(BPF_LD|BPF_W|BPF_ABS, 0),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K, NR_mount, 11, 0),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K, NR_umount2, 10, 0),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K, NR_chroot, 9, 0),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K, NR_mknod, 8, 0),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K, NR_mknodat, 7, 0),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K, NR_rt_sigaction, 6, 0),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K, NR_setuid, 5, 0),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K, NR_setgid, 4, 0),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K, NR_setgroups, 3, 0),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K, NR_unshare, 2, 0),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K, NR_getpid, 1, 0),
        BPF_STMT(BPF_RET|BPF_K, 0x7fff0000), // ALLOW
        BPF_STMT(BPF_RET|BPF_K, 0x00030000), // TRAP: mount
        BPF_STMT(BPF_RET|BPF_K, 0x00030000), // TRAP: umount2
        BPF_STMT(BPF_RET|BPF_K, 0x00030000), // TRAP: chroot
        BPF_STMT(BPF_RET|BPF_K, 0x00030000), // TRAP: mknod
        BPF_STMT(BPF_RET|BPF_K, 0x00030000), // TRAP: mknodat
        BPF_STMT(BPF_RET|BPF_K, 0x00030000), // TRAP: rt_sigaction
        BPF_STMT(BPF_RET|BPF_K, 0x00030000), // TRAP: setuid
        BPF_STMT(BPF_RET|BPF_K, 0x00030000), // TRAP: setgid
        BPF_STMT(BPF_RET|BPF_K, 0x00030000), // TRAP: setgroups
        BPF_STMT(BPF_RET|BPF_K, 0x00030000), // TRAP: unshare
        BPF_STMT(BPF_RET|BPF_K, 0x00030000), // TRAP: getpid
    };
    struct sock_fprog prog = { .len = sizeof(filter)/sizeof(filter[0]), .filter = filter };
    if (syscall(SYS_seccomp, SECCOMP_SET_MODE_FILTER, 0, &prog) != 0)
        if (prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &prog, 0, 0) != 0) return -1;
    return 0;
}

static int install_sigsys(void) {
    struct sigaction sa = {};
    sa.sa_sigaction = sigsys_handler;
    sa.sa_flags = SA_SIGINFO | SA_NODEFER;
    sigemptyset(&sa.sa_mask);
    if (sigaction(SIGSYS, &sa, NULL) != 0) return -1;
    sigset_t mask; sigemptyset(&mask); sigaddset(&mask, SIGSYS);
    if (sigprocmask(SIG_UNBLOCK, &mask, NULL) != 0) return -1;
    return 0;
}

// =========================================================================
// .init_array constructor — runs before main()
// This is the key: when loaded via LD_PRELOAD, this runs before init's main()
// =========================================================================
__attribute__((constructor(101)))
static void twoyi_init(void) {
    // Get rootfs path from env
    g_rootfs = getenv("TWOYI_ROOTFS");
    if (!g_rootfs) g_rootfs = "/data/data/io.twoyi/rootfs";

    write_str(2, "[twoyi_loader] init: installing virtualization\n");

    // Initialize real function pointers
    init_real_funcs();

    // Install SIGSYS handler
    if (install_sigsys() != 0) {
        write_str(2, "[twoyi_loader] FATAL: sigsys handler failed\n");
        return;
    }
    write_str(2, "[twoyi_loader] SIGSYS handler installed\n");

    // Install seccomp filter
    if (install_seccomp() != 0) {
        write_str(2, "[twoyi_loader] FATAL: seccomp failed\n");
        return;
    }
    write_str(2, "[twoyi_loader] seccomp filter installed\n");

    // Mark runtime as ready
    g_runtime_ready = 1;
    write_str(2, "[twoyi_loader] runtime ready — guest can boot\n");
}
