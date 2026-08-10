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
#include <malloc.h>
#include <unistd.h> // for environ on some systems
extern char **environ;

// Forward declarations
static int mkdir_p(const char *path, mode_t mode);
static int should_translate(const char *path);
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/mount.h>
#include <sys/socket.h>
#include <sys/un.h>
// Android system property constants (from sys/system_properties.h)
// These are Android-specific and not available on the host build system.
// We define them here so the loader compiles on the host.
#define PROP_NAME_MAX   32
#define PROP_VALUE_MAX  92
typedef struct prop_info prop_info;
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

// Helper: write to both stderr and a log file (for debugging when stderr is /dev/null)
// Also tries to write to logd via __android_log_print (if available via dlsym)
static void write_str(int fd, const char *s) {
    if (!s) return;
    size_t l = 0; while (s[l]) l++;
    // Write to stderr (goes to logd's stderr collector)
    syscall(NR_write, fd, s, l);
    // Also write to /data/local/tmp/twoyi-loader.log for debugging
    int logfd = syscall(NR_open, "/data/local/tmp/twoyi-loader.log", O_WRONLY | O_CREAT | O_APPEND, 0666);
    if (logfd >= 0) { syscall(NR_write, logfd, s, l); syscall(NR_close, logfd); }
    // Also try __android_log_write via dlsym (goes directly to logd socket)
    // This is critical for processes where stderr is closed/redirected (e.g., after execv)
    static int (*android_log_write_p)(int, const char *, const char *) = NULL;
    static int android_log_checked = 0;
    if (!android_log_checked) {
        android_log_write_p = (int (*)(int, const char *, const char *))dlsym(RTLD_DEFAULT, "__android_log_write");
        android_log_checked = 1;
    }
    if (android_log_write_p) {
        // ANDROID_LOG_INFO = 4, tag = "twoyi_loader"
        android_log_write_p(4, "twoyi_loader", s);
    }
}

// =========================================================================
// Global state (must be before PLT hooks that use it)
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
// LD_PRELOAD path — stored at init time, re-set before each exec
// ROOT CAUSE: Android init's FirstStageMain calls clearenv() which wipes
// LD_PRELOAD, then execv() which resets signal handlers to SIG_DFL.
// The seccomp filter survives execve, but the SIGSYS handler is gone.
// Fix: hook execv/execve to re-set LD_PRELOAD before each exec, so the
// .init_array constructor re-installs the handler in the new process.
// =========================================================================
static char g_preload_path[512] = {0};
static char g_rootfs_env[512] = {0};

static void set_preload_path(void) {
    const char *preload = getenv("LD_PRELOAD");
    if (preload) {
        // LD_PRELOAD may contain multiple paths separated by ':'
        // We need to preserve ALL of them
        strncpy(g_preload_path, preload, sizeof(g_preload_path) - 1);
    }
    // Also save TWOYI_ROOTFS so we can restore it after clearenv
    const char *rootfs = getenv("TWOYI_ROOTFS");
    if (rootfs) {
        strncpy(g_rootfs_env, rootfs, sizeof(g_rootfs_env) - 1);
    }
}

static void restore_preload_env(void) {
    if (g_preload_path[0]) {
        setenv("LD_PRELOAD", g_preload_path, 1);
    }
    if (g_rootfs_env[0]) {
        setenv("TWOYI_ROOTFS", g_rootfs_env, 1);
    }
}

// Hook clearenv — preserve LD_PRELOAD and TWOYI_ROOTFS across clearenv().
// Android init's FirstStageMain calls clearenv() which wipes ALL env vars.
// This means LD_PRELOAD is gone, and subsequent execv's don't load our loader.
// Our execv hook restores LD_PRELOAD, but if init uses an exec variant we
// don't hook (or a direct syscall), LD_PRELOAD stays missing.
// By preserving LD_PRELOAD across clearenv, we ensure it's always in environ
// when execv is called, regardless of which exec variant is used.
int clearenv(void) {
    // Save LD_PRELOAD and TWOYI_ROOTFS before clearing
    char saved_preload[512] = {0};
    char saved_rootfs[512] = {0};
    const char *preload = getenv("LD_PRELOAD");
    const char *rootfs = getenv("TWOYI_ROOTFS");
    if (preload) strncpy(saved_preload, preload, sizeof(saved_preload) - 1);
    if (rootfs) strncpy(saved_rootfs, rootfs, sizeof(saved_rootfs) - 1);
    
    // Call real clearenv
    static int (*real_clearenv)(void) = NULL;
    if (!real_clearenv) real_clearenv = dlsym(RTLD_NEXT, "clearenv");
    int ret = 0;
    if (real_clearenv) {
        ret = real_clearenv();
    } else {
        // Fallback: set environ to NULL manually
        extern char **environ;
        environ = NULL;
    }
    
    // Restore LD_PRELOAD and TWOYI_ROOTFS
    if (saved_preload[0]) {
        setenv("LD_PRELOAD", saved_preload, 1);
    }
    if (saved_rootfs[0]) {
        setenv("TWOYI_ROOTFS", saved_rootfs, 1);
    }
    
    // Update g_preload_path and g_rootfs_env in case they weren't set
    if (!g_preload_path[0] && saved_preload[0]) {
        strncpy(g_preload_path, saved_preload, sizeof(g_preload_path) - 1);
    }
    if (!g_rootfs_env[0] && saved_rootfs[0]) {
        strncpy(g_rootfs_env, saved_rootfs, sizeof(g_rootfs_env) - 1);
    }
    
    write_str(2, "[twoyi_loader] clearenv: preserved LD_PRELOAD + TWOYI_ROOTFS\n");
    return ret;
}

// Hook unsetenv — prevent LD_PRELOAD from being unset
int unsetenv(const char *name) {
    if (name && (strcmp(name, "LD_PRELOAD") == 0 || strcmp(name, "TWOYI_ROOTFS") == 0)) {
        char msg[256];
        int len = snprintf(msg, sizeof(msg),
            "[twoyi_loader] unsetenv(%s) — BLOCKED to preserve loader\n", name);
        write_str(2, msg);
        return 0;
    }
    static int (*real_unsetenv)(const char *) = NULL;
    if (!real_unsetenv) real_unsetenv = dlsym(RTLD_NEXT, "unsetenv");
    if (real_unsetenv) return real_unsetenv(name);
    return 0;
}

// =========================================================================
// PLT interposition for mount/mknod/chroot/etc
// These replace the seccomp/SIGSYS approach (which doesn't survive execv).
// PLT hooks survive execv because LD_PRELOAD is restored by our execv hook.
// =========================================================================

static int (*real_mount)(const char *, const char *, const char *,
                         unsigned long, const void *) = NULL;

int mount(const char *source, const char *target, const char *fstype,
          unsigned long flags, const void *data) {
    if (!real_mount) real_mount = dlsym(RTLD_NEXT, "mount");

    // Special paths: /dev, /mnt, /storage — skip (return 0)
    if (target) {
        if ((strncmp(target, "/dev", 4) == 0 && (target[4] == 0 || target[4] == '/')) ||
            (strncmp(target, "/mnt", 4) == 0 && (target[4] == 0 || target[4] == '/')) ||
            (strncmp(target, "/storage", 8) == 0 && (target[8] == 0 || target[8] == '/')))
            return 0;
    }

    // Record in mount table
    pthread_mutex_lock(&g_mount_lock);
    for (int i = 0; i < MAX_MOUNTS; i++) {
        if (g_mounts[i].active && target &&
            strncmp(g_mounts[i].target, target, 256) == 0) {
            if (flags & MS_REMOUNT) {
                g_mounts[i].flags = flags;
                pthread_mutex_unlock(&g_mount_lock);
                return 0;
            }
            pthread_mutex_unlock(&g_mount_lock);
            errno = EBUSY;
            return -1;
        }
    }
    int slot = -1;
    for (int i = 0; i < MAX_MOUNTS; i++) {
        if (!g_mounts[i].active) { slot = i; break; }
    }
    if (slot >= 0 && target) {
        if (source) strncpy(g_mounts[slot].source, source, 255);
        else g_mounts[slot].source[0] = 0;
        strncpy(g_mounts[slot].target, target, 255);
        if (fstype) strncpy(g_mounts[slot].fstype, fstype, 63);
        else g_mounts[slot].fstype[0] = 0;
        g_mounts[slot].flags = flags;
        g_mounts[slot].active = 1;
    }
    pthread_mutex_unlock(&g_mount_lock);

    // Return 0 (success) — don't actually call real mount
    return 0;
}

static int (*real_umount2)(const char *, int) = NULL;

int umount2(const char *target, int flags) {
    (void)flags;
    if (!real_umount2) real_umount2 = dlsym(RTLD_NEXT, "umount2");
    if (!target) { errno = EFAULT; return -1; }
    pthread_mutex_lock(&g_mount_lock);
    for (int i = 0; i < MAX_MOUNTS; i++) {
        if (g_mounts[i].active && strncmp(g_mounts[i].target, target, 256) == 0) {
            g_mounts[i].active = 0;
            pthread_mutex_unlock(&g_mount_lock);
            return 0;
        }
    }
    pthread_mutex_unlock(&g_mount_lock);
    errno = EINVAL;
    return -1;
}

int chroot(const char *path) {
    (void)path;
    // Return 0 (success) — path translation handles chroot effect
    return 0;
}

// Hook mkdir — redirect rootfs paths to {rootfs}/...
// This catches init's mkdir calls for /linkerconfig, /acct, /config, etc.
int mkdir(const char *path, mode_t mode) {
    // Redirect /dev/__properties__ and its subdirectories to rootfs
    if (path && g_rootfs && (
        strcmp(path, "/dev/__properties__") == 0 ||
        strncmp(path, "/dev/__properties__/", 20) == 0
    )) {
        char real_path[512];
        snprintf(real_path, sizeof(real_path), "%s%s", g_rootfs, path);
        mkdir_p(real_path, mode);
        // Also create on host as a symlink target (if it doesn't exist)
        struct stat st;
        if (lstat(path, &st) != 0) {
            symlink(real_path, path);
        }
        return 0;
    }
    // Redirect other rootfs paths (linkerconfig, acct, config, etc.)
    // to {rootfs}/... so init can create them
    if (path && should_translate(path)) {
        char real_path[512];
        snprintf(real_path, sizeof(real_path), "%s%s", g_rootfs, path);
        mkdir_p(real_path, mode);
        return 0;
    }
    // For other paths (host paths like /dev, /proc, /sys, /data), call real mkdir
    static int (*real_mkdir)(const char *, mode_t) = NULL;
    if (!real_mkdir) real_mkdir = dlsym(RTLD_NEXT, "mkdir");
    if (real_mkdir) return real_mkdir(path, mode);
    return syscall(SYS_mkdir, path, mode);
}

static int (*real_mknod)(const char *, mode_t, dev_t) = NULL;
static int (*real_mknodat)(int, const char *, mode_t, dev_t) = NULL;

int mknod(const char *path, mode_t mode, dev_t dev) {
    if (!real_mknod) real_mknod = dlsym(RTLD_NEXT, "mknod");
    // For device nodes, create a regular file containing dev_t
    mode_t fmt = mode & S_IFMT;
    if (fmt == S_IFCHR || fmt == S_IFBLK) {
#if defined(__x86_64__)
        int fd = syscall(NR_open, path, O_RDWR|O_CREAT, 0666);
        if (fd >= 0) {
            syscall(NR_write, fd, &dev, sizeof(dev_t));
            syscall(NR_close, fd);
        }
#endif
        return 0;
    }
    // For non-device nodes, call real mknod
    if (real_mknod) return real_mknod(path, mode, dev);
    return 0;
}

int mknodat(int dirfd, const char *path, mode_t mode, dev_t dev) {
    if (!real_mknodat) real_mknodat = dlsym(RTLD_NEXT, "mknodat");
    mode_t fmt = mode & S_IFMT;
    if (fmt == S_IFCHR || fmt == S_IFBLK) {
        // Create regular file
        return 0;
    }
    if (real_mknodat) return real_mknodat(dirfd, path, mode, dev);
    return 0;
}

int setuid(uid_t uid) { (void)uid; return 0; }
int setgid(gid_t gid) { (void)gid; return 0; }
int setgroups(size_t size, const gid_t *list) { (void)size; (void)list; return 0; }
int setresuid(uid_t ruid, uid_t euid, uid_t suid) { (void)ruid; (void)euid; (void)suid; return 0; }
int setresgid(gid_t rgid, gid_t egid, gid_t sgid) { (void)rgid; (void)egid; (void)sgid; return 0; }
int unshare(int flags) { (void)flags; return 0; }

// Hook setpgid — fake success.
// ROOT CAUSE: init calls setpgid(0, 0) when forking services (ueventd, etc.).
// In our container (PID namespace without proper session setup), this fails
// with EPERM, causing init to LOG(FATAL) and abort (signal 6).
// Fix: return 0 (fake success) so init continues.
int setpgid(pid_t pid, pid_t pgid) { (void)pid; (void)pgid; return 0; }

// Hook setsid — fake success (return current PID as the new session ID).
// init may call setsid() for some services. In our container, this might
// fail. Return 1 (fake session ID) to avoid crashes.
pid_t setsid(void) { return 1; }

// Hook unlink/unlinkat — redirect /dev/socket/ paths to rootfs.
// init calls unlink("/dev/socket/property_service") before bind(). Without
// this hook, it would delete the HOST's property_service socket, breaking
// the host's property service.
int unlink(const char *path) {
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static int (*real_unlink)(const char *) = NULL;
        if (!real_unlink) real_unlink = dlsym(RTLD_NEXT, "unlink");
        if (real_unlink) return real_unlink(translated);
        return syscall(SYS_unlink, translated);
    }
    static int (*real_unlink)(const char *) = NULL;
    if (!real_unlink) real_unlink = dlsym(RTLD_NEXT, "unlink");
    if (real_unlink) return real_unlink(path);
    return syscall(SYS_unlink, path);
}

int unlinkat(int dirfd, const char *path, int flags) {
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static int (*real_unlinkat)(int, const char *, int) = NULL;
        if (!real_unlinkat) real_unlinkat = dlsym(RTLD_NEXT, "unlinkat");
        if (real_unlinkat) return real_unlinkat(dirfd, translated, flags);
        return syscall(SYS_unlinkat, dirfd, translated, flags);
    }
    static int (*real_unlinkat)(int, const char *, int) = NULL;
    if (!real_unlinkat) real_unlinkat = dlsym(RTLD_NEXT, "unlinkat");
    if (real_unlinkat) return real_unlinkat(dirfd, path, flags);
    return syscall(SYS_unlinkat, dirfd, path, flags);
}

// Hook bind — redirect AF_UNIX socket paths to rootfs.
// init creates sockets at /dev/socket/property_service, /dev/socket/..., etc.
// Without this hook, these sockets would be created on the HOST, conflicting
// with host services. We translate the socket path to {rootfs}/dev/socket/...
int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen) {
    static int (*real_bind)(int, const struct sockaddr *, socklen_t) = NULL;
    if (!real_bind) real_bind = dlsym(RTLD_NEXT, "bind");

    if (addr && addr->sa_family == AF_UNIX && g_rootfs) {
        struct sockaddr_un *un = (struct sockaddr_un *)addr;
        if (un->sun_path[0] == '/' && should_translate(un->sun_path)) {
            // Build translated path
            char translated[600];
            snprintf(translated, sizeof(translated), "%s%s", g_rootfs, un->sun_path);

            // Ensure parent directory exists using DIRECT SYSCALLS
            // (bypass our mkdir hook to avoid recursion/translation issues)
            char dir[600];
            strncpy(dir, translated, sizeof(dir) - 1);
            dir[sizeof(dir) - 1] = 0;
            char *slash = strrchr(dir, '/');
            if (slash) {
                *slash = 0;
                // Create directory chain using direct syscalls
                for (char *p = dir + 1; *p; p++) {
                    if (*p == '/') {
                        *p = 0;
                        syscall(SYS_mkdir, dir, 0777);
                        *p = '/';
                    }
                }
                syscall(SYS_mkdir, dir, 0777);
            }

            // Create a copy of the sockaddr with the translated path
            struct sockaddr_un new_addr;
            memset(&new_addr, 0, sizeof(new_addr));
            new_addr.sun_family = AF_UNIX;
            strncpy(new_addr.sun_path, translated, sizeof(new_addr.sun_path) - 1);

            char msg[256];
            snprintf(msg, sizeof(msg),
                "[twoyi_loader] bind: translated %s -> %s\n", un->sun_path, translated);
            write_str(2, msg);

            if (real_bind) return real_bind(sockfd, (const struct sockaddr *)&new_addr, sizeof(new_addr));
            return syscall(SYS_bind, sockfd, &new_addr, sizeof(new_addr));
        }
    }
    if (real_bind) return real_bind(sockfd, addr, addrlen);
    return syscall(SYS_bind, sockfd, addr, addrlen);
}

// Hook fchmodat — redirect paths to rootfs (matches bind translation)
int fchmodat(int dirfd, const char *path, mode_t mode, int flags) {
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static int (*real_fchmodat)(int, const char *, mode_t, int) = NULL;
        if (!real_fchmodat) real_fchmodat = dlsym(RTLD_NEXT, "fchmodat");
        if (real_fchmodat) return real_fchmodat(dirfd, translated, mode, flags);
        return syscall(SYS_fchmodat, dirfd, translated, mode, flags);
    }
    static int (*real_fchmodat)(int, const char *, mode_t, int) = NULL;
    if (!real_fchmodat) real_fchmodat = dlsym(RTLD_NEXT, "fchmodat");
    if (real_fchmodat) return real_fchmodat(dirfd, path, mode, flags);
    return syscall(SYS_fchmodat, dirfd, path, mode, flags);
}

// Hook chmod — redirect paths to rootfs
int chmod(const char *path, mode_t mode) {
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static int (*real_chmod)(const char *, mode_t) = NULL;
        if (!real_chmod) real_chmod = dlsym(RTLD_NEXT, "chmod");
        if (real_chmod) return real_chmod(translated, mode);
        return syscall(SYS_chmod, translated, mode);
    }
    static int (*real_chmod)(const char *, mode_t) = NULL;
    if (!real_chmod) real_chmod = dlsym(RTLD_NEXT, "chmod");
    if (real_chmod) return real_chmod(path, mode);
    return syscall(SYS_chmod, path, mode);
}

// Hook chown — redirect paths to rootfs
int chown(const char *path, uid_t owner, gid_t group) {
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static int (*real_chown)(const char *, uid_t, gid_t) = NULL;
        if (!real_chown) real_chown = dlsym(RTLD_NEXT, "chown");
        if (real_chown) return real_chown(translated, owner, group);
        return syscall(SYS_chown, translated, owner, group);
    }
    static int (*real_chown)(const char *, uid_t, gid_t) = NULL;
    if (!real_chown) real_chown = dlsym(RTLD_NEXT, "chown");
    if (real_chown) return real_chown(path, owner, group);
    return syscall(SYS_chown, path, owner, group);
}

// =========================================================================
// SELinux context hooks
// Init checks if it can transition from its current context (u:r:su:s0)
// to a service's exec context (u:r:apexd_exec:s0, etc.). The guest's
// SELinux policy doesn't have these transition rules for `su` domain,
// so init fails to start services.
//
// Fix: hook the SELinux context functions to fake success:
// - getcon(): return "u:r:init:s0" (so init thinks it's in init domain)
// - setexeccon(): fake success (don't actually set context)
// - security_compute_create(): return "u:r:init:s0" (allow transition)
// - selinux_check_access(): return 0 (allow all)
// =========================================================================

// Fake SELinux context strings
static const char *FAKE_CONTEXT = "u:r:init:s0";

// Define security_class_t if not available (older systems)
#ifndef security_class_t
typedef unsigned short security_class_t;
#endif

int getcon(char **context) {
    if (context) {
        *context = strdup(FAKE_CONTEXT);
        if (*context) return 0;
        return -1;
    }
    return -1;
}

int getprevcon(char **context) {
    if (context) {
        *context = strdup(FAKE_CONTEXT);
        if (*context) return 0;
        return -1;
    }
    return -1;
}

int getpidcon(pid_t pid, char **context) {
    (void)pid;
    if (context) {
        *context = strdup(FAKE_CONTEXT);
        if (*context) return 0;
        return -1;
    }
    return -1;
}

int getexeccon(char **context) {
    if (context) {
        *context = strdup(FAKE_CONTEXT);
        if (*context) return 0;
        return -1;
    }
    return -1;
}

int setexeccon(const char *context) {
    (void)context;
    // Fake success — don't actually set the exec context
    return 0;
}

int setexeccon_raw(const char *context) {
    (void)context;
    return 0;
}

int security_compute_create(const char *scon, const char *tcon,
                            security_class_t tclass, char **newcon) {
    (void)scon; (void)tclass;
    if (newcon) {
        // Return a context DIFFERENT from scon to indicate a domain transition.
        // Init checks: if (newcon == mycon) → "no domain transition" → fail.
        // So we must return something different from "u:r:init:s0".
        //
        // Strategy: derive a context from tcon (the file's context).
        // tcon is like "u:object_r:apexd_exec:s0" — we convert it to
        // "u:r:apexd:s0" (the process domain) by replacing "object_r:"
        // with "r:" and removing the "_exec" suffix.
        if (tcon) {
            // tcon format: u:object_r:<type>:s0
            // We want:    u:r:<domain>:s0
            // where <domain> is <type> without "_exec" suffix
            char result[256];
            const char *p = tcon;
            // Copy user (e.g., "u:")
            int i = 0;
            while (*p && *p != ':' && i < 200) result[i++] = *p++;
            if (*p == ':') { result[i++] = ':'; p++; }
            // Skip "object_r:" — replace with "r:"
            if (strncmp(p, "object_r:", 9) == 0) {
                result[i++] = 'r'; result[i++] = ':';
                p += 9;
            } else if (strncmp(p, "r:", 2) == 0) {
                result[i++] = 'r'; result[i++] = ':';
                p += 2;
            }
            // Copy type (e.g., "apexd_exec"), removing "_exec" suffix
            char type_buf[128];
            int ti = 0;
            while (*p && *p != ':' && ti < 120) type_buf[ti++] = *p++;
            type_buf[ti] = 0;
            // Remove "_exec" suffix if present
            int tlen = ti;
            if (tlen >= 5 && strcmp(type_buf + tlen - 5, "_exec") == 0) {
                type_buf[tlen - 5] = 0;
            }
            // Append type to result
            for (int j = 0; type_buf[j] && i < 240; j++) result[i++] = type_buf[j];
            // Copy remaining (e.g., ":s0")
            while (*p && i < 250) result[i++] = *p++;
            result[i] = 0;
            *newcon = strdup(result);
            if (*newcon) return 0;
        }
        // Fallback: return a generic context
        *newcon = strdup("u:r:init:s0");
        if (*newcon) return 0;
        return -1;
    }
    return -1;
}

int security_compute_create_raw(const char *scon, const char *tcon,
                                security_class_t tclass, char **newcon) {
    return security_compute_create(scon, tcon, tclass, newcon);
}

// security_compute_create_name — same as security_compute_create but with name
int security_compute_create_name(const char *scon, const char *tcon,
                                  security_class_t tclass, const char *objname,
                                  char **newcon) {
    (void)objname;
    return security_compute_create(scon, tcon, tclass, newcon);
}

int security_compute_create_name_raw(const char *scon, const char *tcon,
                                      security_class_t tclass, const char *objname,
                                      char **newcon) {
    (void)objname;
    return security_compute_create(scon, tcon, tclass, newcon);
}

// selinux_check_access — allow all
int selinux_check_access(const char *scon, const char *tcon,
                         const char *class, const char *perm, void *aux) {
    (void)scon; (void)tcon; (void)class; (void)perm; (void)aux;
    return 0;  // allow all
}

// selinux_check_security_context — all contexts are valid
int selinux_check_security_context(const char *con) {
    (void)con;
    return 0;
}

// selinux_cmpcon — contexts match (avoid context comparison failures)
int selinux_context_cmp(const char *a, const char *b) {
    (void)a; (void)b;
    return 0;  // equal
}

// selinux_check_context — all contexts are valid
int selinux_check_context(const char *con) {
    (void)con;
    return 0;
}

int selinux_android_restorecon(const char *pathname, unsigned int flags) {
    (void)pathname; (void)flags;
    return 0;  // fake success
}

int selinux_android_restorecon_pkgdir(const char *pkgdir, const char *seinfo,
                                       uid_t uid, unsigned int flags) {
    (void)pkgdir; (void)seinfo; (void)uid; (void)flags;
    return 0;
}

int selinux_android_setfilecon(const char *path, const char *seinfo,
                                uid_t uid) {
    (void)path; (void)seinfo; (void)uid;
    return 0;
}

int selinux_android_context_type(const char *type) {
    (void)type;
    return 0;
}

// setfscreatecon — fake success
int setfscreatecon(const char *context) {
    (void)context;
    return 0;
}

int setfscreatecon_raw(const char *context) {
    (void)context;
    return 0;
}

int getfscreatecon(char **context) {
    if (context) {
        *context = strdup(FAKE_CONTEXT);
        if (*context) return 0;
        return -1;
    }
    return -1;
}

// fsetfilecon — fake success
int fsetfilecon(int fd, const char *context) {
    (void)fd; (void)context;
    return 0;
}

int setfilecon(const char *path, const char *context) {
    (void)path; (void)context;
    return 0;
}

// lsetfilecon — fake success
int lsetfilecon(const char *path, const char *context) {
    (void)path; (void)context;
    return 0;
}

// freecon — no-op (we used strdup, but freecon is supposed to free)
void freecon(char *context) {
    // free(context);  // free what we strdup'd
    // Actually, safer to no-op since some callers pass non-alloc'd contexts
    (void)context;
}

// is_selinux_enabled — return 1 (enabled) so init's selinux code paths run
int is_selinux_enabled(void) {
    return 1;
}

// security_getenforce — return 0 (permissive) so init doesn't try to enforce
int security_getenforce(void) {
    return 0;  // permissive
}

// security_setenforce — fake success
int security_setenforce(int value) {
    (void)value;
    return 0;
}

// Hook keyctl — init calls keyctl_get_keyring_ID(KEY_SPEC_SESSION_KEYRING, 1)
// This might fail or cause issues in our container. Return 0 (fake success).
long keyctl(int cmd, ...) {
    (void)cmd;
    return 0;  // fake keyring ID
}

// Minimal in-memory property system
// We can't use the real bionic property area (it corrupts the host).
// Instead, we fake all property functions with a simple key-value store.
#define MAX_PROPS 256
struct prop_entry {
    char key[128];
    char value[128];
    int used;
};
static struct prop_entry g_props[MAX_PROPS];

static int prop_set(const char *key, const char *value) {
    if (!key || !value) return -1;
    // Find existing
    for (int i = 0; i < MAX_PROPS; i++) {
        if (g_props[i].used && strcmp(g_props[i].key, key) == 0) {
            strncpy(g_props[i].value, value, 127);
            g_props[i].value[127] = 0;
            return 0;
        }
    }
    // Find free slot
    for (int i = 0; i < MAX_PROPS; i++) {
        if (!g_props[i].used) {
            strncpy(g_props[i].key, key, 127);
            g_props[i].key[127] = 0;
            strncpy(g_props[i].value, value, 127);
            g_props[i].value[127] = 0;
            g_props[i].used = 1;
            return 0;
        }
    }
    return -1; // table full
}

static int prop_get(const char *key, char *value) {
    if (!key || !value) return 0;
    for (int i = 0; i < MAX_PROPS; i++) {
        if (g_props[i].used && strcmp(g_props[i].key, key) == 0) {
            strncpy(value, g_props[i].value, 128);
            return strlen(g_props[i].value);
        }
    }
    value[0] = 0;
    return 0;
}

// Hook __system_property_area_init — create property_info file on HOST
// and return 0 (success) without creating the real property area.
// The in-memory property system handles all get/set operations.
int __system_property_area_init(void) {
    // Create /dev/__properties__/property_info on the HOST so that
    // WriteStringToFile (which uses direct openat syscall) can write to it.
    // This is called from PropertyInit() BEFORE CreateSerializedPropertyInfo().
    struct stat st;
    if (stat("/dev/__properties__", &st) == 0) {
        int fd = syscall(NR_open, "/dev/__properties__/property_info",
                        O_WRONLY | O_CREAT, 0666);
        if (fd >= 0) {
            syscall(NR_close, fd);
        }
    }
    return 0;
}

// Hook __system_property_set — store in our in-memory table
int __system_property_set(const char *key, const char *value) {
    return prop_set(key, value);
}

// Hook __system_property_add — init uses this to add properties during boot
// Returns 0 on success, -1 on failure
int __system_property_add(const char *name, unsigned int namelen,
                          const char *value, unsigned int valuelen) {
    (void)namelen; (void)valuelen;
    return prop_set(name, value);
}

// Hook __system_property_update — update an existing property
int __system_property_update(prop_info *pi, const char *value, unsigned int len) {
    (void)len;
    if (!pi) return -1;
    // pi is a pointer to our prop_entry (from __system_property_find)
    struct prop_entry *entry = (struct prop_entry *)pi;
    if (entry->used) {
        strncpy(entry->value, value, 127);
        entry->value[127] = 0;
        return 0;
    }
    return -1;
}

// Hook __system_property_read — read property value
int __system_property_read(const prop_info *pi, char *name, char *value) {
    if (!pi) return 0;
    const struct prop_entry *entry = (const struct prop_entry *)pi;
    if (entry->used) {
        if (name) {
            strncpy(name, entry->key, PROP_NAME_MAX - 1);
            name[PROP_NAME_MAX - 1] = 0;
        }
        if (value) {
            strncpy(value, entry->value, PROP_VALUE_MAX - 1);
            value[PROP_VALUE_MAX - 1] = 0;
        }
        return strlen(entry->value);
    }
    return 0;
}

// Hook __system_property_get — read from our in-memory table
int __system_property_get(const char *name, char *value) {
    return prop_get(name, value);
}

// Hook __system_property_find — return a fake pointer (non-NULL if found)
// We use the prop_entry address as the "prop_info" pointer
static char g_dummy_prop_info[1] = {0};
const void *__system_property_find(const char *name) {
    if (!name) return NULL;
    for (int i = 0; i < MAX_PROPS; i++) {
        if (g_props[i].used && strcmp(g_props[i].key, name) == 0) {
            return &g_props[i]; // return pointer to entry as fake prop_info
        }
    }
    return NULL;
}

// Hook __system_property_read_callback — call callback with our value
void __system_property_read_callback(const void *pi,
    void (*callback)(void *cookie, const char *name, const char *value, uint32_t serial),
    void *cookie) {
    if (!pi || !callback) return;
    const struct prop_entry *entry = (const struct prop_entry *)pi;
    callback(cookie, entry->key, entry->value, 0);
}

// Hook __system_property_serial — return 0
uint32_t __system_property_serial(const void *pi) {
    (void)pi;
    return 0;
}

// Hook __system_property_foreach — iterate our table
int __system_property_foreach(void (*propfn)(const void *pi, void *cookie), void *cookie) {
    if (!propfn) return 0;
    for (int i = 0; i < MAX_PROPS; i++) {
        if (g_props[i].used) {
            propfn(&g_props[i], cookie);
        }
    }
    return 0;
}

// Hook __system_property_wait_any — return immediately
const void *__system_property_wait_any(const void *pi) {
    (void)pi;
    return NULL;
}

// execv/execve hooks — restore LD_PRELOAD before each exec
static int (*real_execv)(const char *, char *const[]) = NULL;
static int (*real_execve)(const char *, char *const[], char *const[]) = NULL;
static int (*real_execvp)(const char *, char *const[]) = NULL;
static int (*real_execvpe)(const char *, char *const[], char *const[]) = NULL;
static int (*real_execveat)(int, const char *, char *const[], char *const[], int) = NULL;

// Diagnostic helper: log the exec call with path + LD_PRELOAD state
static void log_exec_call(const char *variant, const char *path) {
    char msg[768];
    int len = snprintf(msg, sizeof(msg),
        "[twoyi_loader] %s called: path=%s preload_path=%s\n",
        variant, path ? path : "(null)",
        g_preload_path[0] ? g_preload_path : "(empty)");
    write_str(2, msg);
}

int execv(const char *path, char *const argv[]) {
    if (!real_execv) real_execv = dlsym(RTLD_NEXT, "execv");
    log_exec_call("execv", path);
    restore_preload_env();
    write_str(2, "[twoyi_loader] execv: restored LD_PRELOAD\n");
    if (!real_execv) return syscall(SYS_execve, path, argv, environ);
    return real_execv(path, argv);
}

// Hook execve — re-set LD_PRELOAD before exec

int execve(const char *path, char *const argv[], char *const envp[]) {
    if (!real_execve) real_execve = dlsym(RTLD_NEXT, "execve");
    log_exec_call("execve", path);
    // For execve, we need to modify the envp array to include LD_PRELOAD
    // But envp might not have LD_PRELOAD. We need to add it.
    // Simplest approach: use setenv() to set it in the current env,
    // then pass environ instead of envp.
    // But execve uses the passed envp, not environ.
    // So we need to build a new envp with LD_PRELOAD added.
    restore_preload_env();

    // Count existing envp entries
    int env_count = 0;
    if (envp) {
        while (envp[env_count]) env_count++;
    }

    // Check if LD_PRELOAD is already in envp
    int has_preload = 0;
    for (int i = 0; i < env_count; i++) {
        if (strncmp(envp[i], "LD_PRELOAD=", 11) == 0) {
            has_preload = 1;
            break;
        }
    }

    if (has_preload || !g_preload_path[0]) {
        // Already has LD_PRELOAD or we don't have a path to set
        if (!real_execve) return syscall(SYS_execve, path, argv, envp);
        return real_execve(path, argv, envp);
    }

    // Build new envp with LD_PRELOAD added
    char preload_env[600];
    snprintf(preload_env, sizeof(preload_env), "LD_PRELOAD=%s", g_preload_path);

    char **new_envp = (char **)malloc(sizeof(char *) * (env_count + 2));
    if (!new_envp) {
        // Can't allocate — fall back to environ
        if (!real_execve) return syscall(SYS_execve, path, argv, environ);
        return real_execve(path, argv, environ);
    }

    for (int i = 0; i < env_count; i++) {
        new_envp[i] = (char *)envp[i];
    }
    new_envp[env_count] = preload_env;
    new_envp[env_count + 1] = NULL;

    write_str(2, "[twoyi_loader] execve: added LD_PRELOAD to envp\n");
    int ret;
    if (!real_execve) ret = syscall(SYS_execve, path, argv, new_envp);
    else ret = real_execve(path, argv, new_envp);
    free(new_envp);
    return ret;
}

// Hook execvp — same as execv but uses PATH
int execvp(const char *path, char *const argv[]) {
    if (!real_execvp) real_execvp = dlsym(RTLD_NEXT, "execvp");
    log_exec_call("execvp", path);
    restore_preload_env();
    write_str(2, "[twoyi_loader] execvp: restored LD_PRELOAD\n");
    if (!real_execvp) return syscall(SYS_execve, path, argv, environ);
    return real_execvp(path, argv);
}

// Hook execvpe — same as execve but uses PATH
int execvpe(const char *path, char *const argv[], char *const envp[]) {
    if (!real_execvpe) real_execvpe = dlsym(RTLD_NEXT, "execvpe");
    log_exec_call("execvpe", path);
    // Same envp manipulation as execve
    restore_preload_env();

    int env_count = 0;
    if (envp) {
        while (envp[env_count]) env_count++;
    }

    int has_preload = 0;
    for (int i = 0; i < env_count; i++) {
        if (strncmp(envp[i], "LD_PRELOAD=", 11) == 0) {
            has_preload = 1;
            break;
        }
    }

    if (has_preload || !g_preload_path[0]) {
        if (!real_execvpe) return syscall(SYS_execve, path, argv, envp);
        return real_execvpe(path, argv, envp);
    }

    char preload_env[600];
    snprintf(preload_env, sizeof(preload_env), "LD_PRELOAD=%s", g_preload_path);

    char **new_envp = (char **)malloc(sizeof(char *) * (env_count + 2));
    if (!new_envp) {
        if (!real_execvpe) return syscall(SYS_execve, path, argv, environ);
        return real_execvpe(path, argv, environ);
    }

    for (int i = 0; i < env_count; i++) {
        new_envp[i] = (char *)envp[i];
    }
    new_envp[env_count] = preload_env;
    new_envp[env_count + 1] = NULL;

    write_str(2, "[twoyi_loader] execvpe: added LD_PRELOAD to envp\n");
    int ret;
    if (!real_execvpe) ret = syscall(SYS_execve, path, argv, new_envp);
    else ret = real_execvpe(path, argv, new_envp);
    free(new_envp);
    return ret;
}

// Hook execveat — execve relative to a dirfd
int execveat(int dirfd, const char *path, char *const argv[],
             char *const envp[], int flags) {
    if (!real_execveat) real_execveat = dlsym(RTLD_NEXT, "execveat");
    log_exec_call("execveat", path);
    restore_preload_env();

    int env_count = 0;
    if (envp) {
        while (envp[env_count]) env_count++;
    }

    int has_preload = 0;
    for (int i = 0; i < env_count; i++) {
        if (strncmp(envp[i], "LD_PRELOAD=", 11) == 0) {
            has_preload = 1;
            break;
        }
    }

    if (has_preload || !g_preload_path[0]) {
        // No LD_PRELOAD needed — call real execveat
        if (!real_execveat) {
            // Fall back to syscall
#ifdef SYS_execveat
            return syscall(SYS_execveat, dirfd, path, argv, envp, flags);
#else
            errno = ENOSYS;
            return -1;
#endif
        }
        return real_execveat(dirfd, path, argv, envp, flags);
    }

    // Build new envp with LD_PRELOAD added
    char preload_env[600];
    snprintf(preload_env, sizeof(preload_env), "LD_PRELOAD=%s", g_preload_path);

    char **new_envp = (char **)malloc(sizeof(char *) * (env_count + 2));
    if (!new_envp) {
        if (!real_execveat) {
#ifdef SYS_execveat
            return syscall(SYS_execveat, dirfd, path, argv, environ, flags);
#else
            errno = ENOSYS;
            return -1;
#endif
        }
        return real_execveat(dirfd, path, argv, environ, flags);
    }

    for (int i = 0; i < env_count; i++) {
        new_envp[i] = (char *)envp[i];
    }
    new_envp[env_count] = preload_env;
    new_envp[env_count + 1] = NULL;

    write_str(2, "[twoyi_loader] execveat: added LD_PRELOAD to envp\n");
    int ret;
    if (!real_execveat) {
#ifdef SYS_execveat
        ret = syscall(SYS_execveat, dirfd, path, argv, new_envp, flags);
#else
        errno = ENOSYS;
        ret = -1;
#endif
    } else {
        ret = real_execveat(dirfd, path, argv, new_envp, flags);
    }
    free(new_envp);
    return ret;
}

// =========================================================================
// Global state
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
    (void)data; // wait_ready() removed — runtime is always ready when handler runs
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
    (void)flags; // wait_ready() removed — runtime is always ready when handler runs
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
    // wait_ready() removed — runtime is always ready when handler runs
    if(!path) return -EFAULT;
    mode_t fmt = mode & S_IFMT;
    if (fmt != S_IFCHR && fmt != S_IFBLK) {
        // Not a device node — return 0 (fake success).
        // Can't call real mknodat (trapped by seccomp, would recurse).
        return 0;
    }
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
    // For non-SIGSYS signals, we can't call syscall(rt_sigaction) because
    // rt_sigaction is trapped by our own BPF filter (would recurse).
    // Instead, use the libc sigaction() wrapper which goes through PLT
    // (not a direct syscall). The PLT call doesn't trigger seccomp.
    // But wait — we're in a signal handler, which is async-signal-unsafe.
    // The safest approach: just return 0 (fake success for all sigaction calls).
    // This means the guest can't install ANY signal handlers, but init
    // doesn't need signal handlers during FirstStageMain.
    return 0;
}

// =========================================================================
// SELinuxFS virtualization
// Init needs /sys/fs/selinux/* files during SELinux setup.
// We create virtual files in the rootfs's /sys/fs/selinux/ directory.
// =========================================================================
// Recursive mkdir (like mkdir -p)
static int mkdir_p(const char *path, mode_t mode) {
    char tmp[512];
    strncpy(tmp, path, sizeof(tmp) - 1);
    tmp[sizeof(tmp) - 1] = 0;
    int len = strlen(tmp);
    if (len > 0 && tmp[len - 1] == '/') tmp[len - 1] = 0;
    for (char *p = tmp + 1; *p; p++) {
        if (*p == '/') {
            *p = 0;
            mkdir(tmp, mode);
            *p = '/';
        }
    }
    return mkdir(tmp, mode);
}

static void ensure_selinuxfs_files(void) {
    if (!g_rootfs) return;
    char dir[512];
    snprintf(dir, sizeof(dir), "%s/sys/fs/selinux", g_rootfs);
    // Recursive mkdir — create /sys, /sys/fs, /sys/fs/selinux
    mkdir_p(dir, 0755);

    // Create required selinuxfs control files
    const char *files[] = {
        "checkreqprot",  // init writes "0" here (FATAL if missing)
        "enforce",       // init writes "0" or "1" here
        "load",          // init writes policy here
        "policyvers",    // init reads policy version
        NULL
    };
    for (int i = 0; files[i]; i++) {
        char path[600];
        snprintf(path, sizeof(path), "%s/%s", dir, files[i]);
        int fd = syscall(NR_open, path, O_WRONLY | O_CREAT, 0666);
        if (fd >= 0) {
            if (strcmp(files[i], "checkreqprot") == 0) {
                syscall(NR_write, fd, "0", 1);
            } else if (strcmp(files[i], "enforce") == 0) {
                syscall(NR_write, fd, "0", 1);
            } else if (strcmp(files[i], "policyvers") == 0) {
                syscall(NR_write, fd, "33", 2);
            }
            syscall(NR_close, fd);
        }
    }
}

// Check if a path should be translated to rootfs
static int should_translate(const char *path) {
    if (!path || !g_rootfs || path[0] != '/') return 0;
    if (strncmp(path, g_rootfs, strlen(g_rootfs)) == 0) return 0;
    // Host-only paths — do NOT translate (these are virtual or host-specific)
    if (strncmp(path, "/proc", 5) == 0) return 0;
    if (strncmp(path, "/sys", 4) == 0) return 0;
    if (strncmp(path, "/data", 5) == 0) return 0;  // host data
    // /dev/ paths: translate socket, __properties__, and other guest paths
    // but keep host device nodes (/dev/null, /dev/zero, /dev/qemu_pipe, etc.)
    if (strncmp(path, "/dev/socket", 11) == 0) return 1;  // guest sockets
    if (strncmp(path, "/dev/__properties__", 19) == 0) return 1;
    if (strncmp(path, "/dev/__null__", 13) == 0) return 1;
    // Other /dev/ paths (null, zero, random, qemu_pipe, binder, etc.) stay on host
    if (strncmp(path, "/dev/", 5) == 0) return 0;
    if (strncmp(path, "/dev", 4) == 0 && (path[4] == 0)) return 0;  // /dev exactly
    // Guest rootfs paths — translate
    if (strncmp(path, "/system", 7) == 0) return 1;
    if (strncmp(path, "/vendor", 7) == 0) return 1;
    if (strncmp(path, "/apex", 5) == 0) return 1;
    if (strncmp(path, "/init", 5) == 0) return 1;
    if (strncmp(path, "/default.prop", 13) == 0) return 1;
    // Init creates these directories during boot — translate to rootfs
    if (strncmp(path, "/linkerconfig", 13) == 0) return 1;
    if (strncmp(path, "/acct", 5) == 0) return 1;
    if (strncmp(path, "/config", 7) == 0) return 1;
    if (strncmp(path, "/metadata", 9) == 0) return 1;
    if (strncmp(path, "/mnt", 4) == 0) return 1;
    if (strncmp(path, "/storage", 8) == 0) return 1;
    if (strncmp(path, "/cache", 6) == 0) return 1;
    if (strncmp(path, "/bin", 4) == 0) return 1;
    if (strncmp(path, "/sbin", 5) == 0) return 1;
    if (strncmp(path, "/lib", 4) == 0) return 1;
    if (strncmp(path, "/etc", 4) == 0) return 1;
    if (strncmp(path, "/root", 5) == 0) return 1;
    if (strncmp(path, "/tmp", 4) == 0) return 1;
    if (strncmp(path, "/var", 4) == 0) return 1;
    if (strncmp(path, "/odm", 4) == 0) return 1;
    if (strncmp(path, "/product", 8) == 0) return 1;
    if (strncmp(path, "/system_ext", 11) == 0) return 1;
    // For any other path starting with /, translate it (safer to redirect
    // to rootfs than to write to the host filesystem)
    return 1;
}

// =========================================================================
// openat PLT interposition (path translation)
// VM uses shadowhook for openat; we use LD_PRELOAD PLT interposition.
// =========================================================================

static int (*real_openat)(int, const char *, int, ...) = NULL;

static void init_real_funcs(void) {
    if (!real_openat) {
        real_openat = dlsym(RTLD_NEXT, "openat");
    }
}

// Path translation: prepend rootfs prefix for rootfs paths only
static char g_translated[512];
static const char *translate(const char *path) {
    if (!path || !g_rootfs) return path;
    if (!should_translate(path)) return path; // kernel paths pass through
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

    // Debug: log all /dev/__properties__ opens
    if (path && strncmp(path, "/dev/__properties__", 19) == 0) {
        char msg[256];
        int len = snprintf(msg, sizeof(msg),
            "[twoyi_loader] openat(%s, flags=0x%x)\n", path, flags);
        write(2, msg, len);
    }

    // Block fstab files → init skips first_stage_mount
    if (path && strstr(path, "fstab.")) {
        errno = ENOENT;
        return -1;
    }

    // Special handling for selinuxfs paths
    // Init opens /sys/fs/selinux/checkreqprot, /sys/fs/selinux/enforce, etc.
    // These need to exist as writable files.
    if (path && strncmp(path, "/sys/fs/selinux/", 16) == 0) {
        const char *translated = translate(path);
        int fd = real_openat ? real_openat(dirfd, translated, flags, mode)
                              : syscall(NR_openat, dirfd, translated, flags, mode);
        if (fd < 0 && (flags & O_WRONLY || flags & O_RDWR)) {
            // File doesn't exist — create it
            fd = real_openat ? real_openat(dirfd, translated, flags | O_CREAT, 0666)
                              : syscall(NR_openat, dirfd, translated, flags | O_CREAT, 0666);
        }
        return fd;
    }

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

    // Debug: log all /dev/__properties__ opens
    if (path && strncmp(path, "/dev/__properties__", 19) == 0) {
        char msg[256];
        int len = snprintf(msg, sizeof(msg),
            "[twoyi_loader] open(%s, flags=0x%x)\n", path, flags);
        write(2, msg, len);
    }

    // Block fstab files → init skips first_stage_mount
    if (path && strstr(path, "fstab.")) {
        errno = ENOENT;
        return -1;
    }

    // Log selinuxfs opens for debugging
    if (path && strncmp(path, "/sys/fs/selinux", 15) == 0) {
        char msg[256];
        int len = snprintf(msg, sizeof(msg),
            "[twoyi_loader] open(%s, flags=0x%x)\n", path, flags);
        write(2, msg, len);
    }

    // Special handling for selinuxfs paths — same as openat()
    if (path && strncmp(path, "/sys/fs/selinux", 15) == 0) {
        const char *translated = translate(path);
        // Ensure the file exists — create it if missing
        char real_path[600];
        snprintf(real_path, sizeof(real_path), "%s/sys/fs/selinux", g_rootfs ? g_rootfs : "");
        mkdir_p(real_path, 0755);

        // Try to create the file if it doesn't exist
        if (g_rootfs) {
            char file_path[600];
            snprintf(file_path, sizeof(file_path), "%s%s", g_rootfs, path);
            int cfd = syscall(NR_open, file_path, O_WRONLY | O_CREAT, 0666);
            if (cfd >= 0) syscall(NR_close, cfd);
        }

#if defined(__x86_64__)
        int fd = syscall(NR_open, translated, flags, mode);
        if (fd < 0 && (flags & O_WRONLY || flags & O_RDWR)) {
            fd = syscall(NR_open, translated, flags | O_CREAT, 0666);
        }
        return fd;
#else
        if (real_openat) {
            int fd = real_openat(AT_FDCWD, translated, flags, mode);
            if (fd < 0 && (flags & O_WRONLY || flags & O_RDWR)) {
                fd = real_openat(AT_FDCWD, translated, flags | O_CREAT, 0666);
            }
            return fd;
        }
        return syscall(NR_openat, AT_FDCWD, translated, flags, mode);
#endif
    }

    const char *translated = translate(path);
#if defined(__x86_64__)
    return syscall(NR_open, translated, flags, mode);
#else
    if (real_openat) return real_openat(AT_FDCWD, translated, flags, mode);
    return syscall(NR_openat, AT_FDCWD, translated, flags, mode);
#endif
}


// Hook __open_2 (bionic's fortified open — used by init's WriteFile)
int __open_2(const char *path, int flags) {
    // Debug: log all /dev/__properties__ opens
    if (path && strncmp(path, "/dev/__properties__", 19) == 0) {
        char msg[256];
        int len = snprintf(msg, sizeof(msg),
            "[twoyi_loader] __open_2(%s, flags=0x%x)\n", path, flags);
        write(2, msg, len);
    }
    // SELinuxFS: intercept and auto-create
    if (path && strncmp(path, "/sys/fs/selinux", 15) == 0) {
        return open(path, flags);  // our hook (translate + create)
    }
    // Fstab files: return ENOENT so init skips first_stage_mount
    if (path && strstr(path, "fstab.")) {
        errno = ENOENT;
        return -1;
    }
    // Translate only rootfs paths (system, vendor, apex, data, init)
    if (should_translate(path)) {
        const char *translated = translate(path);
        static int (*real_open2)(const char *, int) = NULL;
        if (!real_open2) real_open2 = dlsym(RTLD_NEXT, "__open_2");
        if (real_open2) return real_open2(translated, flags);
#if defined(__x86_64__)
        return syscall(NR_open, translated, flags);
#else
        return syscall(NR_openat, AT_FDCWD, translated, flags);
#endif
    }
    // Pass through (kernel paths: /proc, /sys, /dev, relative paths)
    static int (*real_open2)(const char *, int) = NULL;
    if (!real_open2) real_open2 = dlsym(RTLD_NEXT, "__open_2");
    if (real_open2) return real_open2(path, flags);
#if defined(__x86_64__)
    return syscall(NR_open, path, flags);
#else
    return syscall(NR_openat, AT_FDCWD, path, flags);
#endif
}

// Hook __open_real (bionic's internal open — all open variants call this)
// This catches WriteStringToFile's open() which bypasses __open_2/__openat_2
int __open_real(const char *pathname, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
    // Translate /dev/__properties__ paths
    if (pathname && strncmp(pathname, "/dev/__properties__", 19) == 0 && g_rootfs) {
        char translated[600];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, pathname);
        // Ensure parent dir exists
        char dir[600];
        strncpy(dir, translated, sizeof(dir) - 1);
        dir[sizeof(dir) - 1] = 0;
        char *slash = strrchr(dir, '/');
        if (slash) { *slash = 0; mkdir_p(dir, 0777); }
        static int (*real_open_real)(const char *, int, ...) = NULL;
        if (!real_open_real) real_open_real = dlsym(RTLD_NEXT, "__open_real");
        if (real_open_real) return real_open_real(translated, flags, mode);
#if defined(__x86_64__)
        return syscall(NR_open, translated, flags, mode);
#else
        return syscall(NR_openat, AT_FDCWD, translated, flags, mode);
#endif
    }
    // Also translate other rootfs paths
    if (should_translate(pathname)) {
        const char *translated = translate(pathname);
        static int (*real_open_real)(const char *, int, ...) = NULL;
        if (!real_open_real) real_open_real = dlsym(RTLD_NEXT, "__open_real");
        if (real_open_real) return real_open_real(translated, flags, mode);
#if defined(__x86_64__)
        return syscall(NR_open, translated, flags, mode);
#else
        return syscall(NR_openat, AT_FDCWD, translated, flags, mode);
#endif
    }
    static int (*real_open_real)(const char *, int, ...) = NULL;
    if (!real_open_real) real_open_real = dlsym(RTLD_NEXT, "__open_real");
    if (real_open_real) return real_open_real(pathname, flags, mode);
#if defined(__x86_64__)
    return syscall(NR_open, pathname, flags, mode);
#else
    return syscall(NR_openat, AT_FDCWD, pathname, flags, mode);
#endif
}

// Hook __openat_2 (bionic's fortified openat)
int __openat_2(int dirfd, const char *path, int flags) {
    // Debug: log all /dev/__properties__ opens
    if (path && strncmp(path, "/dev/__properties__", 19) == 0) {
        char msg[256];
        int len = snprintf(msg, sizeof(msg),
            "[twoyi_loader] __openat_2(%s, flags=0x%x)\n", path, flags);
        write(2, msg, len);
    }
    if (path && strncmp(path, "/sys/fs/selinux", 15) == 0) {
        return openat(dirfd, path, flags);
    }
    if (path && strstr(path, "fstab.")) {
        errno = ENOENT;
        return -1;
    }
    if (should_translate(path)) {
        const char *translated = translate(path);
        static int (*real_openat2)(int, const char *, int) = NULL;
        if (!real_openat2) real_openat2 = dlsym(RTLD_NEXT, "__openat_2");
        if (real_openat2) return real_openat2(dirfd, translated, flags);
        return syscall(NR_openat, dirfd, translated, flags);
    }
    static int (*real_openat2)(int, const char *, int) = NULL;
    if (!real_openat2) real_openat2 = dlsym(RTLD_NEXT, "__openat_2");
    if (real_openat2) return real_openat2(dirfd, path, flags);
    return syscall(NR_openat, dirfd, path, flags);
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

    // Log: use ONLY async-signal-safe functions (write, not snprintf)
    // Format: "[twoyi_loader] SIGSYS #N nr=M\n"
    {
        char msg[64];
        char *p = msg;
        const char *prefix = "[twoyi_loader] SIGSYS #";
        while (*prefix) *p++ = *prefix++;
        // Write count (decimal)
        int c = g_sigsys_count;
        char tmp[16]; int t = 0;
        if (c == 0) tmp[t++] = '0';
        while (c > 0) { tmp[t++] = '0' + (c % 10); c /= 10; }
        while (t > 0) *p++ = tmp[--t];
        *p++ = ' '; *p++ = 'n'; *p++ = 'r'; *p++ = '=';
        // Write syscall number (decimal)
        long n = nr;
        if (n == 0) tmp[t++] = '0';
        else { t = 0; while (n > 0) { tmp[t++] = '0' + (n % 10); n /= 10; } }
        while (t > 0) *p++ = tmp[--t];
        *p++ = '\n';
        write(2, msg, p - msg);
    }
    long ret;
    switch (nr) {
        case NR_mount: {
            ret = emu_mount((const char*)GET_ARG(ctx,0), (const char*)GET_ARG(ctx,1),
                           (const char*)GET_ARG(ctx,2), GET_ARG(ctx,3), (const void*)GET_ARG(ctx,4));
            break;
        }
        case NR_umount2: ret = emu_umount2((const char*)GET_ARG(ctx,0), (int)GET_ARG(ctx,1)); break;
        case NR_chroot: // wait_ready() removed — runtime is always ready when handler runs ret = 0; break;
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
            // Handle openat (trapped by seccomp) — translate path
            if (nr == NR_openat) {
                int dirfd = (int)GET_ARG(ctx, 0);
                const char *path = (const char *)GET_ARG(ctx, 1);
                int flags = (int)GET_ARG(ctx, 2);
                mode_t mode = (mode_t)GET_ARG(ctx, 3);
                if (path && should_translate(path)) {
                    const char *translated = translate(path);
                    ret = syscall(NR_openat, dirfd, translated, flags, mode);
                } else if (path && strncmp(path, "/dev/__properties__", 19) == 0) {
                    const char *translated = translate(path);
                    ret = syscall(NR_openat, dirfd, translated, flags, mode);
                } else {
                    ret = syscall(NR_openat, dirfd, path, flags, mode);
                }
            } else {
                ret = -ENOSYS;
            }
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
        // Use SECCOMP_RET_ERRNO(EPERM) instead of SECCOMP_RET_TRAP
        // This returns -1 + errno=EPERM for trapped syscalls WITHOUT
        // sending SIGSYS. This avoids the signal handler entirely.
        //
        // WHY: Android init's execv() resets signal handlers to SIG_DFL.
        // The seccomp filter survives execve, but the SIGSYS handler is gone.
        // Using ERRNO instead of TRAP means we don't need a handler.
        //
        // SECCOMP_RET_ERRNO = 0x00050000 | errno
        // EPERM = 1
        // So: 0x00050001
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
        // ERRNO(EPERM) for all trapped syscalls
        // 0x00050000 = SECCOMP_RET_ERRNO, | 1 = EPERM
        BPF_STMT(BPF_RET|BPF_K, 0x00050001), // ERRNO(EPERM): mount
        BPF_STMT(BPF_RET|BPF_K, 0x00050001), // ERRNO(EPERM): umount2
        BPF_STMT(BPF_RET|BPF_K, 0x00050001), // ERRNO(EPERM): chroot
        BPF_STMT(BPF_RET|BPF_K, 0x00050001), // ERRNO(EPERM): mknod
        BPF_STMT(BPF_RET|BPF_K, 0x00050001), // ERRNO(EPERM): mknodat
        BPF_STMT(BPF_RET|BPF_K, 0x00050001), // ERRNO(EPERM): rt_sigaction
        BPF_STMT(BPF_RET|BPF_K, 0x00050001), // ERRNO(EPERM): setuid
        BPF_STMT(BPF_RET|BPF_K, 0x00050001), // ERRNO(EPERM): setgid
        BPF_STMT(BPF_RET|BPF_K, 0x00050001), // ERRNO(EPERM): setgroups
        BPF_STMT(BPF_RET|BPF_K, 0x00050001), // ERRNO(EPERM): unshare
        // getpid: return 1 via ERRNO? No — ERRNO returns -1.
        // We need getpid to return 1 (success, PID=1).
        // With ERRNO, getpid returns -1 + errno=EPERM, which init treats as failure.
        // But init's getpid check: if (getpid() != 1) → exit(31)
        // With ERRNO, getpid returns -1, which != 1, so init exits 31.
        // We CANNOT use ERRNO for getpid.
        // Instead, trap getpid with TRAP and have the handler return 1.
        // But after execv, the handler is gone...
        // SOLUTION: Don't trap getpid at all. Use LD_PRELOAD getpid_hook
        // for the libc path. For direct syscalls, init won't use getpid
        // directly (it uses the libc wrapper).
        // Remove getpid from the trap list.
        BPF_STMT(BPF_RET|BPF_K, 0x00030000), // TRAP: getpid (only works pre-execv)
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

    // Save LD_PRELOAD path so we can restore it before execv/execve
    set_preload_path();

    // DIAGNOSTIC: log the LD_PRELOAD value and TWOYI_ROOTFS so we can
    // verify the loader is being loaded with the right env in every
    // process (init first stage, selinux_setup, secilc, second_stage).
    {
        char msg[1024];
        const char *preload = getenv("LD_PRELOAD");
        const char *rootfs = getenv("TWOYI_ROOTFS");
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] init: LD_PRELOAD=%s TWOYI_ROOTFS=%s g_rootfs=%s\n",
            preload ? preload : "(null)",
            rootfs ? rootfs : "(null)",
            g_rootfs ? g_rootfs : "(null)");
        write_str(2, msg);
        
        // Also verify that the LD_PRELOAD files exist and are accessible
        if (preload) {
            char *preload_copy = strdup(preload);
            if (preload_copy) {
                char *tok = strtok(preload_copy, ":");
                while (tok) {
                    struct stat st;
                    if (stat(tok, &st) == 0) {
                        snprintf(msg, sizeof(msg),
                            "[twoyi_loader] init: LD_PRELOAD file OK: %s (size=%ld mode=0%o)\n",
                            tok, (long)st.st_size, st.st_mode & 0777);
                    } else {
                        snprintf(msg, sizeof(msg),
                            "[twoyi_loader] init: LD_PRELOAD file MISSING: %s (errno=%d: %s)\n",
                            tok, errno, strerror(errno));
                    }
                    write_str(2, msg);
                    tok = strtok(NULL, ":");
                }
                free(preload_copy);
            }
        }
    }

    write_str(2, "[twoyi_loader] init: installing virtualization (PLT-only mode)\n");

    // Initialize real function pointers
    init_real_funcs();

    // NOTE: We do NOT install seccomp BPF filter anymore.
    // ROOT CAUSE: Android init calls execv() which resets signal handlers.
    // Seccomp filters survive execve but SIGSYS handlers don't.
    // After execv, any SECCOMP_RET_TRAP → SIGSYS → SIG_DFL → process killed.
    //
    // Instead, we use PLT interposition (LD_PRELOAD hooks) for ALL
    // virtualized syscalls. PLT hooks survive execv because:
    // 1. Our execv/execve hooks restore LD_PRELOAD in the environment
    // 2. The new process loads our .so via LD_PRELOAD
    // 3. The .init_array constructor runs again, re-installing PLT hooks
    //
    // The getpid_hook.so (also in LD_PRELOAD) handles getpid() → return 1.

    // NOTE: Do NOT install seccomp BPF filter for openat.
    // ROOT CAUSE: Android init calls execv() which resets signal handlers.
    // Seccomp filters survive execve but SIGSYS handlers don't.
    // After execv, any SECCOMP_RET_TRAP → SIGSYS → SIG_DFL → process killed.
    // The linker's own openat calls during library loading get trapped → SIGSYS → kill.
    //
    // Instead, use PLT interposition + pre-create property files in rootfs.

    write_str(2, "[twoyi_loader] PLT hooks installed\n");

    // Create SELinuxFS virtual files (init needs /sys/fs/selinux/checkreqprot etc.)
    ensure_selinuxfs_files();
    write_str(2, "[twoyi_loader] selinuxfs virtual files created\n");

    // Eagerly create /dev/__properties__/ in the rootfs with property files
    // (init's WriteStringToFile bypasses PLT hooks, but our open/__open_2
    // hooks translate /dev/__properties__ → {rootfs}/dev/__properties__)
    // Create property files in ROOTFS AND on HOST
    // WriteStringToFile uses a direct openat syscall that bypasses ALL PLT hooks.
    // It opens /dev/__properties__/property_info on the HOST path.
    // We need the file to exist on the HOST so the open succeeds.
    // The HOST's /dev/__properties__/ directory already exists (created by host init).
    // We just create the file (no chmod on the directory).
    // The host's property service reads properties_serial, not property_info,
    // so creating property_info should be safe.
    if (g_rootfs) {
        char prop_dir[512];
        snprintf(prop_dir, sizeof(prop_dir), "%s/dev/__properties__", g_rootfs);
        mkdir_p(prop_dir, 0777);
        // Pre-create in rootfs
        const char *files[] = {"property_info", "properties_serial", NULL};
        for (int i = 0; files[i]; i++) {
            char fpath[600];
            snprintf(fpath, sizeof(fpath), "%s/%s", prop_dir, files[i]);
            int fd = syscall(NR_open, fpath, O_WRONLY | O_CREAT, 0666);
            if (fd >= 0) syscall(NR_close, fd);
        }
        // Also create on HOST (WriteStringToFile bypasses PLT hooks)
        struct stat st;
        if (stat("/dev/__properties__", &st) == 0) {
            // Only create property_info (NOT properties_serial — host uses that)
            int fd = syscall(NR_open, "/dev/__properties__/property_info",
                            O_WRONLY | O_CREAT, 0666);
            if (fd >= 0) {
                syscall(NR_close, fd);
                write_str(2, "[twoyi_loader] created property_info on host\n");
            }
        }
    }

    write_str(2, "[twoyi_loader] runtime ready — guest can boot\n");

    g_runtime_ready = 1;
}
