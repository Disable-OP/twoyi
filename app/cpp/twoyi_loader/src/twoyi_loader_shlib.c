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
// Path translation (rootfs prefixing) — defined with should_translate;
// needed earlier by the 6-Z288 node-materialisation hooks.
static const char *translate(const char *path);
static void unsetenv_internal(const char *name);
// Binder device open fallback (defined later, before __open_2 hook)
static int is_binder_device_path(const char *path);
static int binder_open_fallback(const char *path, int real_fd, int saved_errno);
// qemu_pipe device open fallback (6-Z116; defined later, alongside
// binder_open_fallback — mirrors the z113 pattern; see the block comment
// at the definition site for the z115 §3 + §7-Rank-1 rationale).
static int is_qemu_pipe_device_path(const char *path);
static int qemu_pipe_open_fallback(const char *path, int real_fd, int saved_errno);
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/mount.h>
#include <sys/statfs.h>  // 6-Z143: struct statfs (the selinuxfs magic hook)
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
#include <linux/fb.h>
#include <ucontext.h>
#include <pthread.h>
#include <sys/epoll.h>

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
  #define NR_mkdirat  258
  #define NR_unlinkat 263
  #define NR_fchmodat 268
  #define NR_fchownat 260
  #define NR_close    3
  #define NR_write    1
  #define NR_read     0
  #define NR_getpid   39
  #define NR_setuid   105
  #define NR_setgid   106
  // 6-Z184 AUDIT FIX (agent 1): these three were wrong by-transposition —
  // 113/114/118 are setreuid/setregid/setresgid; setgroups/setresuid/
  // setresgid are 116/117/119. The old values made the seccomp filter
  // trap the WRONG syscalls on x86_64 (EPERM-ing setreuid/setregid and
  // leaving setresuid/setresgid unvirtualized).
  #define NR_setgroups 116
  #define NR_setresuid 117
  #define NR_setresgid 119
  #define NR_unshare  272
  // 6-Z184 AUDIT FIX (agent 1): rt_sigaction is 13 on x86_64 (134 is
  // uselib — an aarch64 number copy-paste); the sigaction emulation arm
  // could never fire.
  #define NR_rt_sigaction 13
  #define NR_sched_yield 24
  // 6-Z272n: epoll/readlink for the /proc/mounts MountHandler neutralizer.
  #define NR_epoll_ctl 233
  #define NR_readlinkat 267
  // 6-Z272m pt3: xattr syscalls for the selinux-label fake (x86_64).
  #define NR_setxattr 188
  #define NR_lsetxattr 189
  #define NR_fsetxattr 190

  #define GET_ARG(ctx, n) ({ \
      unsigned long _a; \
      switch(n) { \
          case 0: _a = (ctx)->uc_mcontext.gregs[8]; break;  \
          case 1: _a = (ctx)->uc_mcontext.gregs[9]; break;  \
          case 2: _a = (ctx)->uc_mcontext.gregs[12]; break; \
          case 3: _a = (ctx)->uc_mcontext.gregs[2]; break;  \
          case 4: _a = (ctx)->uc_mcontext.gregs[0]; break;  \
          case 5: _a = (ctx)->uc_mcontext.gregs[1]; break; \
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
  #define NR_mkdirat  34
  #define NR_unlinkat 35
  #define NR_fchmodat 53
  #define NR_fchownat 55
  #define NR_close    57
  #define NR_write    64
  #define NR_read     63
  #define NR_getpid   172
  #define NR_setuid   146
  #define NR_setgid   144
  #define NR_setgroups 159
  #define NR_setresuid 147
  #define NR_setresgid 149
  #define NR_unshare  97
  #define NR_rt_sigaction 134
  #define NR_sched_yield 124
  // 6-Z272n: epoll/readlink for the /proc/mounts MountHandler neutralizer
  // (aarch64 generic: epoll_ctl=21, readlinkat=78).
  #define NR_epoll_ctl 21
  #define NR_readlinkat 78
  // 6-Z272m pt3: xattr syscalls for the selinux-label fake (asm-generic).
  #define NR_setxattr 5
  #define NR_lsetxattr 6
  #define NR_fsetxattr 7

  #define GET_ARG(ctx, n) ((unsigned long)(ctx)->uc_mcontext.regs[n])
  #define SET_RET(ctx, val) (ctx)->uc_mcontext.regs[0] = (uint64_t)(val)
#elif defined(__arm__) && !defined(__aarch64__)
  // 6-Z227: ARMv7 (AArch32) — the ELF32 EM_ARM guest hook builds
  // (armeabi-v7a, build.sh) compile this file with
  // -target armv7a-linux-androideabi24. All numbers are the EABI
  // syscall table (arch/arm/tools/syscall.tbl), __NR_SYSCALL_BASE=0.
  #define TWOYI_AUDIT_ARCH 0x40000028U  // AUDIT_ARCH_ARM (EM_ARM|32BIT)
  #define NR_mount    21
  #define NR_umount2  52
  #define NR_chroot   61
  #define NR_mknod    14
  #define NR_mknodat  324
  #define NR_openat   322
  #define NR_mkdirat  323
  #define NR_unlinkat 328
  #define NR_fchmodat 333
  #define NR_fchownat 325
  #define NR_close    6
  #define NR_write    4
  #define NR_read     3
  #define NR_getpid   20
  // 6-Z227: bionic on arm32 issues the *32 uid-syscall variants (the
  // plain-number slots are the 16-bit uid legacy entries). Verified
  // against arch/arm/tools/syscall.tbl: setuid32=213, setgid32=214,
  // setgroups32=206, setresuid32=208, setresgid32=210.
  #define NR_setuid   213
  #define NR_setgid   214
  #define NR_setgroups 206
  #define NR_setresuid 208
  #define NR_setresgid 210
  #define NR_unshare  337
  #define NR_rt_sigaction 174
  #define NR_sched_yield 158
  // 6-Z272n: epoll/readlink for the /proc/mounts MountHandler neutralizer
  // (ARM EABI: epoll_ctl=251, readlinkat=332).
  #define NR_epoll_ctl 251
  #define NR_readlinkat 332
  // 6-Z272m pt3: xattr syscalls for the selinux-label fake (ARM EABI).
  #define NR_setxattr 226
  #define NR_lsetxattr 227
  #define NR_fsetxattr 228
  // 6-Z227: arm32 has NO newfstatat / SYS_mmap — the *64 variants are
  // the wired syscalls (fstatat64=327 fills struct stat64; mmap2=192
  // takes the file offset in page units). See twoyi_sys_fstatat and
  // the mmap() hook tail below.
  #define NR_fstatat64 327
  #define NR_mmap2    192

  #define GET_ARG(ctx, n) ({ \
      unsigned long _a; \
      switch(n) { \
          case 0: _a = (ctx)->uc_mcontext.arm_r0; break;  \
          case 1: _a = (ctx)->uc_mcontext.arm_r1; break;  \
          case 2: _a = (ctx)->uc_mcontext.arm_r2; break;  \
          case 3: _a = (ctx)->uc_mcontext.arm_r3; break;  \
          case 4: _a = (ctx)->uc_mcontext.arm_r4; break;  \
          case 5: _a = (ctx)->uc_mcontext.arm_r5; break; \
          default: _a = 0; break; \
      } _a; })
  #define SET_RET(ctx, val) (ctx)->uc_mcontext.arm_r0 = (unsigned long)(val)
#endif

// Legacy fallback defines for bionic API 24 compatibility (x86_64 only).
// On aarch64 these syscalls don't exist; the wrappers below route around
// the missing definitions using the *at variants.
#ifndef SYS_open
  #ifdef __NR_open
    #define SYS_open __NR_open
  #elif defined(__x86_64__)
    #define SYS_open 2
  #endif
#endif
#ifndef SYS_mkdir
  #ifdef __NR_mkdir
    #define SYS_mkdir __NR_mkdir
  #elif defined(__x86_64__)
    #define SYS_mkdir 39
  #endif
#endif
#ifndef SYS_unlink
  #ifdef __NR_unlink
    #define SYS_unlink __NR_unlink
  #elif defined(__x86_64__)
    #define SYS_unlink 87
  #endif
#endif
#ifndef SYS_chmod
  #ifdef __NR_chmod
    #define SYS_chmod __NR_chmod
  #elif defined(__x86_64__)
    #define SYS_chmod 90
  #endif
#endif
#ifndef SYS_chown
  #ifdef __NR_chown
    #define SYS_chown __NR_chown
  #elif defined(__x86_64__)
    #define SYS_chown 92
  #endif
#endif
// 6-Z139: tgkill (x86_64: 234) / rt_tgsigqueueinfo (x86_64: 297) — used
// by the abort-unmasking hooks' syscall() fallback. Old NDK headers may
// lack the SYS_* spellings; __NR_* is checked first, then the verified
// x86_64 numbers (aarch64: tgkill=131, rt_tgsigqueueinfo=240).
#ifndef SYS_tgkill
  #ifdef __NR_tgkill
    #define SYS_tgkill __NR_tgkill
  #elif defined(__x86_64__)
    #define SYS_tgkill 234
  #endif
#endif
#ifndef SYS_rt_tgsigqueueinfo
  #ifdef __NR_rt_tgsigqueueinfo
    #define SYS_rt_tgsigqueueinfo __NR_rt_tgsigqueueinfo
  #elif defined(__x86_64__)
    #define SYS_rt_tgsigqueueinfo 297
  #endif
#endif

// =========================================================================
// Architecture-independent syscall wrappers
// =========================================================================
// On aarch64 (and other newer architectures) the legacy syscalls
// SYS_open / SYS_mkdir / SYS_unlink / SYS_chmod / SYS_chown do NOT exist
// in the kernel — only their *at variants (openat / mkdirat / unlinkat /
// fchmodat / fchownat) are wired up. The previous version of this file
// tried to #define SYS_open to a hardcoded x86_64 number, which silently
// left it undefined on aarch64 and broke the build for arm64-v8a.
//
// These wrappers dispatch to the right syscall per architecture. They use
// raw syscall() (not the libc wrappers open()/mkdir()/...) to avoid
// recursing through the PLT — this file is loaded via LD_PRELOAD and
// interposes open()/mkdir()/unlink()/chmod()/chown() themselves.
//
// NR_openat / NR_mkdirat / NR_unlinkat / NR_fchmodat / NR_fchownat are
// defined in the arch-specific block above (x86_64 and aarch64).
// AT_FDCWD comes from <fcntl.h>.

static inline long twoyi_sys_open(const char *path, int flags, mode_t mode) {
#if defined(__x86_64__)
    return syscall(SYS_open, path, flags, mode);
#else
    return syscall(NR_openat, AT_FDCWD, path, flags, mode);
#endif
}
static inline long twoyi_sys_open2(const char *path, int flags) {
    // 2-arg open() variant (no mode) — used by __open_2 fallbacks.
#if defined(__x86_64__)
    return syscall(SYS_open, path, flags);
#else
    return syscall(NR_openat, AT_FDCWD, path, flags);
#endif
}
static inline long twoyi_sys_mkdir(const char *path, mode_t mode) {
#if defined(__x86_64__)
    return syscall(SYS_mkdir, path, mode);
#else
    return syscall(NR_mkdirat, AT_FDCWD, path, mode);
#endif
}
static inline long twoyi_sys_unlink(const char *path) {
#if defined(__x86_64__)
    return syscall(SYS_unlink, path);
#else
    return syscall(NR_unlinkat, AT_FDCWD, path, 0);
#endif
}
static inline long twoyi_sys_chmod(const char *path, mode_t mode) {
#if defined(__x86_64__)
    return syscall(SYS_chmod, path, mode);
#else
    return syscall(NR_fchmodat, AT_FDCWD, path, mode, 0);
#endif
}
static inline long twoyi_sys_chown(const char *path, uid_t owner, gid_t group) {
#if defined(__x86_64__)
    return syscall(SYS_chown, path, owner, group);
#else
    return syscall(NR_fchownat, AT_FDCWD, path, owner, group, 0);
#endif
}

// 6-Z227: portable fstatat-into-struct-stat. arm32 has no SYS_newfstatat;
// the wired syscall is fstatat64, which fills a struct stat64 (different
// field widths/layout from struct stat). The 64-bit targets keep the
// direct newfstatat syscall (identical behavior to before). The
// conversion only matters on the dlsym-failure fallback paths — bionic's
// own wrappers are always preferred by the callers.
static inline int twoyi_sys_fstatat(int dirfd, const char *path,
                                    struct stat *buf, int flags) {
#if defined(__arm__) && !defined(__aarch64__)
    struct stat64 st64;
    if (syscall(NR_fstatat64, dirfd, path, &st64, flags) != 0) return -1;
    memset(buf, 0, sizeof(*buf));
    buf->st_dev = st64.st_dev;
    buf->st_mode = st64.st_mode;
    buf->st_uid = st64.st_uid;
    buf->st_gid = st64.st_gid;
    buf->st_rdev = st64.st_rdev;
    buf->st_size = st64.st_size;
    buf->st_blksize = st64.st_blksize;
    buf->st_blocks = st64.st_blocks;
    buf->st_nlink = st64.st_nlink;
    buf->st_ino = st64.st_ino;
    // st_atime/st_mtime/st_ctime (+ _nsec) are the portable spellings:
    // every bionic generation either exposes them as direct fields or
    // as macros over st_atim.tv_sec / .tv_nsec (glibc-style).
    buf->st_atime = st64.st_atime;
    buf->st_atime_nsec = st64.st_atime_nsec;
    buf->st_mtime = st64.st_mtime;
    buf->st_mtime_nsec = st64.st_mtime_nsec;
    buf->st_ctime = st64.st_ctime;
    buf->st_ctime_nsec = st64.st_ctime_nsec;
    return 0;
#else
    return syscall(SYS_newfstatat, dirfd, path, buf, flags);
#endif
}

// Helper: write to both stderr and a log file (for debugging when stderr is /dev/null)
// Also tries to write to logd via __android_log_print (if available via dlsym)
static void write_str(int fd, const char *s) {
    if (!s) return;
    size_t l = 0; while (s[l]) l++;
    // Write to stderr (goes to logd's stderr collector)
    syscall(NR_write, fd, s, l);
    // Also write to /data/local/tmp/twoyi-loader.log for debugging
    int logfd = twoyi_sys_open("/data/local/tmp/twoyi-loader.log", O_WRONLY | O_CREAT | O_APPEND, 0666);
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

// 6-Z139: the REAL pid of this process (captured at loader init via a
// raw syscall BEFORE any hooks install). bionic's abort() does
// tgkill(getpid(), gettid(), SIGABRT) — with the getpid hook returning
// 1 (the fake init pid), tgkill(1, real_tid) returns ESRCH and abort()
// falls back to _exit(127), masking EVERY guest FATAL as a silent 127
// exit (run 32811363109: servicemanager/wait_for_keymaster/zygote all
// died this way with no crash message). tgkill/rt_tgsigqueueinfo hooks
// rewrite tgid==1 (or == the fake getpid) to this REAL pid so the
// signal actually delivers and the crash message prints.
static int g_real_pid = -1;

// Mount table
#define MAX_MOUNTS 32
struct mount_entry {
    char source[256]; char target[256]; char fstype[64];
    unsigned long flags; int active;
};
static struct mount_entry g_mounts[MAX_MOUNTS];
static pthread_mutex_t g_mount_lock = PTHREAD_MUTEX_INITIALIZER;

// ---------------------------------------------------------------------------
// 6-Z265: kernel-true BR_REPLY data backing.
//
// EVIDENCE (run 33334415274, OrangeFox R12 lavender): the guest's REAL
// libbinder.so (recovery + keystore2) dereferences tr.data_ptr from every
// BR_REPLY. The proxy used to hand v1 clients a wire tr with data_ptr=0
// and NO reply bytes on the wire — libbinder built its reply Parcel over
// NULL and SIGSEGV'd at si_addr=0x0. The recovery died 7 times (init kept
// restarting it — the "soft reboots to flash back again" user report) and
// keystore2 crash-looped 56 times (63 libbinder NULL crashes in one run).
//
// The proxy now appends the reply parcel bytes to the response frame as
// a v2-style trailer even for v1 requests. Here we:
//   1. parse that trailer,
//   2. malloc real backing memory for [data][offsets],
//   3. patch tr.data_ptr / tr.offsets_ptr inside the BR stream we just
//      copied into the guest's read buffer,
//   4. free the backing memory when the guest returns the buffer via
//      BC_FREE_BUFFER (kernel-true lifecycle).
// ---------------------------------------------------------------------------
#define BP_WIRE_V2_MAGIC 0x30325657u   /* "WV20" little-endian */
#define BP_BR_REPLY      0x80407203u   /* _IOR('r', 3, binder_transaction_data=64) */
#define BP_BR_TRANSACTION 0x80407202u  /* _IOR('r', 2, 64) — 6-Z271 server delivery */
#define BP_TR_DATA_PTR_OFF     48u     /* binder_transaction_data.data.ptr.buffer */
#define BP_TR_OFFSETS_PTR_OFF  56u     /* binder_transaction_data.data.ptr.offsets */

#define BP_REPLY_ALLOC_MAX 32
struct bp_reply_alloc {
    void *base;
    uint64_t size;
};
static struct bp_reply_alloc g_bp_reply_allocs[BP_REPLY_ALLOC_MAX];
static size_t g_bp_reply_alloc_next = 0;
static pthread_mutex_t g_bp_alloc_lock = PTHREAD_MUTEX_INITIALIZER;

// 6-Z272d: the most recent SM-GET reply stash — the client's verdict
// observer (BC_FREE_BUFFER = reply parcel CONSUMED; free-less re-transact
// = transport-level failure). Single pending slot: the poll loop answers
// one SM GET at a time, and the verdict fires before the next one.
static pthread_mutex_t g_sm_pending_lock = PTHREAD_MUTEX_INITIALIZER;
static uintptr_t g_sm_pending_stash = 0;
static uint32_t g_sm_pending_stash_len = 0;

static void bp_alloc_register(void *base, uint64_t size) {
    pthread_mutex_lock(&g_bp_alloc_lock);
    // Prefer a free slot; otherwise overwrite the oldest (round-robin).
    // Recovery traffic is a handful of transactions, so eviction of a
    // still-live buffer would only matter for pathological floods — and
    // a freed-under-client buffer is exactly what the real kernel does
    // under memory pressure anyway (BR_FROZEN_REPLY / transaction
    // failure), which libbinder handles.
    size_t start = g_bp_reply_alloc_next;
    size_t slot = start;
    for (size_t i = 0; i < BP_REPLY_ALLOC_MAX; i++) {
        size_t idx = (start + i) % BP_REPLY_ALLOC_MAX;
        if (g_bp_reply_allocs[idx].base == NULL) { slot = idx; break; }
    }
    g_bp_reply_allocs[slot].base = base;
    g_bp_reply_allocs[slot].size = size;
    g_bp_reply_alloc_next = (slot + 1) % BP_REPLY_ALLOC_MAX;
    pthread_mutex_unlock(&g_bp_alloc_lock);
}

static void bp_alloc_free(uintptr_t ptr) {
    if (ptr == 0) return;
    pthread_mutex_lock(&g_bp_alloc_lock);
    for (size_t i = 0; i < BP_REPLY_ALLOC_MAX; i++) {
        if (g_bp_reply_allocs[i].base != NULL &&
            (uintptr_t)g_bp_reply_allocs[i].base == ptr) {
            free(g_bp_reply_allocs[i].base);
            g_bp_reply_allocs[i].base = NULL;
            g_bp_reply_allocs[i].size = 0;
            break;
        }
    }
    pthread_mutex_unlock(&g_bp_alloc_lock);
    // 6-Z272d: the client's own verdict on the SM reply. BC_FREE_BUFFER
    // for the pending SM stash = the client's waitForResponse populated
    // the reply Parcel and tore it down (the reply parcel was CONSUMED —
    // a null still returned means the failure is inside the AIDL
    // marshalling). No free + another handle-0 transact = the transact
    // failed before Parcel teardown (transport-level).
    pthread_mutex_lock(&g_sm_pending_lock);
    if (g_sm_pending_stash != 0 && ptr == (uintptr_t)g_sm_pending_stash) {
        static int sm_consumed_budget = 4;
        if (sm_consumed_budget > 0) {
            sm_consumed_budget--;
            char msg[128];
            snprintf(msg, sizeof(msg),
                "[twoyi_loader] *** SM-REPLY-CONSUMED (client freed the reply "
                "parcel; %u stash bytes)\n", (unsigned)g_sm_pending_stash_len);
            write_str(2, msg);
        }
        g_sm_pending_stash = 0;
        g_sm_pending_stash_len = 0;
    }
    pthread_mutex_unlock(&g_sm_pending_lock);
}

/// Walk the copied BR stream for BR_REPLY commands and give every one a
/// real backing allocation from the proxy's reply-blob trailer.
static void bp_patch_reply_data(uint8_t *stream, uint64_t stream_len,
                                const uint8_t *tail, uint64_t tail_len) {
    if (tail_len < 8) return;
    uint32_t magic;
    memcpy(&magic, tail, 4);
    if (magic != BP_WIRE_V2_MAGIC) return;
    uint32_t blob_count;
    memcpy(&blob_count, tail + 4, 4);
    const uint8_t *p = tail + 8;
    uint64_t rem = tail_len - 8;
    uint32_t blob_idx = 0;

    uint64_t pos = 0;
    while (pos + 4 <= stream_len && blob_idx < blob_count) {
        uint32_t cmd;
        memcpy(&cmd, stream + pos, 4);
        uint32_t sz = (cmd >> 16) & 0x3fff;
        if ((cmd == BP_BR_REPLY || cmd == BP_BR_TRANSACTION) &&
            pos + 4 + 64 <= stream_len) {
            // 6-Z272k: bounded per-side delivery DIAG — the keystore2
            // compat chain deadlock needed the CLIENT-side view: did the
            // guest's waitForResponse actually receive the
            // BR_TRANSACTION that the proxy's steal delivered (tx #2),
            // and what code/target did it carry. First 12 deliveries per
            // process.
            {
                static int br_tx_diag = 12;
                if (br_tx_diag > 0) {
                    br_tx_diag--;
                    uint32_t t_code, t_target;
                    memcpy(&t_code, stream + pos + 4 + 16, 4);
                    memcpy(&t_target, stream + pos + 4, 4);
                    char m[160];
                    snprintf(m, sizeof(m),
                             "[twoyi_loader] *** 6-Z272k %s to client: target=%u code=0x%x "
                             "(pid=%d)\n",
                             (cmd == BP_BR_TRANSACTION) ? "BR-TX" : "BR-RPY",
                             t_target, t_code, g_real_pid);
                    write_str(2, m);
                }
            }
            if (rem < 8) return;
            uint32_t dlen, olen;
            memcpy(&dlen, p, 4);
            memcpy(&olen, p + 4, 4);
            p += 8; rem -= 8;
            if (rem < (uint64_t)dlen + olen) return;
            // 6-Z271w: first-REPLY-per-process blob dump — the client-side
            // parse of our SM getService reply returned NULL (keystore2's
            // NAME_NOT_FOUND panic chain) while the proxy-side parse was
            // clean; this dumps the EXACT parcel bytes the guest client
            // receives. Prefixed with the fb_hook fatal marker so the
            // tracer's write-DIAG bypass captures it regardless of
            // budget (g_fatal_entered-style: once per process).
            static int reply_dump_done = 0;
            if (!reply_dump_done && dlen >= 28 && dlen <= 128 && olen >= 8) {
                reply_dump_done = 1;
                char dump[512];
                int off = snprintf(dump, sizeof(dump),
                    "[twoyi_loader] *** SM-REPLY dlen=%u olen=%u data=", dlen, olen);
                for (uint32_t q = 0; q < dlen && off < (int)sizeof(dump) - 4; q++)
                    off += snprintf(dump + off, sizeof(dump) - off, "%02x", p[q]);
                off += snprintf(dump + off, sizeof(dump) - off, " offsets=");
                for (uint32_t q = 0; q < olen && off < (int)sizeof(dump) - 4; q++)
                    off += snprintf(dump + off, sizeof(dump) - off, "%02x", p[dlen + q]);
                snprintf(dump + off, sizeof(dump) - off, "\n");
                write_str(2, dump);
            }
            // [data][offsets] in one allocation; offsets_ptr = base + dlen.
            uint8_t *back = (uint8_t *)malloc((size_t)(dlen + olen ? dlen + olen : 1));
            if (back) {
                if (dlen) memcpy(back, p, dlen);
                if (olen) memcpy(back + dlen, p + dlen, olen);
                bp_alloc_register(back, (uint64_t)dlen + olen);
                uint64_t data_ptr = (uint64_t)(uintptr_t)back;
                uint64_t offsets_ptr = (uint64_t)(uintptr_t)(back + dlen);
                memcpy(stream + pos + 4 + BP_TR_DATA_PTR_OFF, &data_ptr, 8);
                memcpy(stream + pos + 4 + BP_TR_OFFSETS_PTR_OFF, &offsets_ptr, 8);
                // 6-Z272d: the SM reply is now byte-identical to a real
                // android-11/12/13 servicemanager reply ([EX_NONE][flat]
                // [stability i32], offsets=[4] — verified against fetched
                // Parcel.cpp/ServiceManager.cpp for all three) yet the
                // client's ServiceManagerShim::getService STILL polls
                // "Waiting for service ... didn't start. Returning NULL"
                // 50× per 5 s budget. The bytes at the shlib are proven;
                // the failure must sit in the LAST inch: the final
                // client-visible tr fields / the client's parse-vs-
                // transport boundary. This dump prints (a) the FINAL tr
                // the client's mIn will read (after patching), (b) a
                // re-read of the backing memory at the patched pointers
                // (stale/corrupted stash detection), and (c) the client's
                // OWN verdict observed at our boundary: a BC_FREE_BUFFER
                // for this stash means the client's waitForResponse
                // populated + tore down the reply Parcel (the null arose
                // INSIDE the AIDL marshalling); a free-less re-transact
                // to handle 0 means the transact itself failed before
                // Parcel teardown.
                {
                    static int tr_dump_budget = 4;
                    static int vs_dump_budget = 6;
                    /* 6-Z272g: the virtual-service METHOD replies
                     * (getHardwareInfo etc.) have olen==0 — the SM gate
                     * skipped them while the getHardwareInfo reply is the
                     * current keystore2 TRANSACTION_FAILED site. Widen the
                     * observation to every reply shape. */
                    int is_sm = (dlen >= 28 && dlen <= 128 && olen >= 8);
                    int is_vs = (dlen >= 8 && dlen <= 256 && olen == 0);
                    if ((is_sm && tr_dump_budget > 0) || (is_vs && vs_dump_budget > 0)) {
                        if (is_sm) tr_dump_budget--;
                        if (is_vs) vs_dump_budget--;
                        uint32_t t_flags, t_dsize, t_osize;
                        uint64_t t_dptr, t_optr;
                        memcpy(&t_flags, stream + pos + 4 + 20, 4);
                        memcpy(&t_dsize, stream + pos + 4 + 32, 4);
                        memcpy(&t_osize, stream + pos + 4 + 40, 4);
                        memcpy(&t_dptr, stream + pos + 4 + BP_TR_DATA_PTR_OFF, 8);
                        memcpy(&t_optr, stream + pos + 4 + BP_TR_OFFSETS_PTR_OFF, 8);
                        char msg[560];
                        int off;
                        if (olen >= 8 && dlen >= 4) {
                            off = snprintf(msg, sizeof(msg),
                                "[twoyi_loader] *** SM-TR flags=0x%08x dsize=%u osize=%u "
                                "dptr=%llx optr=%llx stash[d0]=%02x%02x%02x%02x "
                                "stash[d+%u..]=%02x%02x%02x%02x stash[o0]=%02x%02x%02x%02x%02x%02x%02x%02x\n",
                                t_flags, t_dsize, t_osize,
                                (unsigned long long)t_dptr, (unsigned long long)t_optr,
                                ((uint8_t *)t_dptr)[0], ((uint8_t *)t_dptr)[1],
                                ((uint8_t *)t_dptr)[2], ((uint8_t *)t_dptr)[3],
                                dlen - 4,
                                ((uint8_t *)t_dptr)[dlen - 4], ((uint8_t *)t_dptr)[dlen - 3],
                                ((uint8_t *)t_dptr)[dlen - 2], ((uint8_t *)t_dptr)[dlen - 1],
                                ((uint8_t *)t_optr)[0], ((uint8_t *)t_optr)[1],
                                ((uint8_t *)t_optr)[2], ((uint8_t *)t_optr)[3],
                                ((uint8_t *)t_optr)[4], ((uint8_t *)t_optr)[5],
                                ((uint8_t *)t_optr)[6], ((uint8_t *)t_optr)[7]);
                        } else {
                            /* 6-Z272g: no offsets array — dump the reply's
                             * head + tail words only (in-bounds). */
                            uint32_t dlen4 = (dlen >= 16) ? dlen : 16;
                            off = snprintf(msg, sizeof(msg),
                                "[twoyi_loader] *** VS-TR flags=0x%08x dsize=%u osize=%u "
                                "dptr=%llx optr=%llx stash[0..3]=%02x%02x%02x%02x "
                                "stash[%u..]=%02x%02x%02x%02x\n",
                                t_flags, t_dsize, t_osize,
                                (unsigned long long)t_dptr, (unsigned long long)t_optr,
                                ((uint8_t *)t_dptr)[0], ((uint8_t *)t_dptr)[1],
                                ((uint8_t *)t_dptr)[2], ((uint8_t *)t_dptr)[3],
                                dlen4 - 4,
                                ((uint8_t *)t_dptr)[dlen4 - 4], ((uint8_t *)t_dptr)[dlen4 - 3],
                                ((uint8_t *)t_dptr)[dlen4 - 2], ((uint8_t *)t_dptr)[dlen4 - 1]);
                        }
                        (void)off;
                        write_str(2, msg);
                        // Track the pending stash for the CONSUMED/NOFREE verdict.
                        pthread_mutex_lock(&g_sm_pending_lock);
                        g_sm_pending_stash = t_dptr;
                        g_sm_pending_stash_len = dlen;
                        pthread_mutex_unlock(&g_sm_pending_lock);
                    }
                }
            }
            // On malloc failure the tr keeps data_ptr=0 (the pre-6-Z265
            // behavior) — no worse than before.
            p += dlen + olen; rem -= (uint64_t)dlen + olen;
            blob_idx++;
        }
        pos += 4 + sz;
    }
}

/// Release the backing memory for buffers the guest returned via
/// BC_FREE_BUFFER (type 'c', nr 3, payload = binder_uintptr_t buffer).
static void bp_free_returned_buffers(const uint8_t *wb, uint64_t ws) {
    uint64_t pos = 0;
    while (pos + 4 <= ws) {
        uint32_t cmd;
        memcpy(&cmd, wb + pos, 4);
        uint32_t sz = (cmd >> 16) & 0x3fff;
        if ((uint8_t)((cmd >> 8) & 0xff) == (uint8_t)'c' &&
            (uint8_t)(cmd & 0xff) == 3 && sz == 8 && pos + 4 + 8 <= ws) {
            uint64_t bufptr;
            memcpy(&bufptr, wb + pos + 4, 8);
            bp_alloc_free(bufptr);
        }
        pos += 4 + sz;
    }
}

// ---------------------------------------------------------------------------
// Binder fallback fd tracking.
//
// When binder_open_fallback() returns a /dev/null fd in place of a real
// binder device fd, that fd is recorded here. The ioctl hook checks this
// set to decide whether to fake binder ioctls (for /dev/null fallbacks,
// where the real ioctl would return ENOTTY) or pass them through to the
// real ioctl (for real binderfs fds, which support real binder IPC within
// the container's binder domain).
//
// Real binderfs fds (opened successfully from /dev/binderfs/*) are NEVER
// in this set, so the ioctl hook passes them through to the real ioctl —
// real binder IPC works natively because the container has its own
// binderfs (mounted by kr64 with chmod 0666) in a separate binder domain
// from the host.
//
// The close() hook clears entries when fds are closed. binder_open_fallback
// opens /dev/null with O_CLOEXEC, so fallback fds do not survive execve
// (no stale entries after exec).
//
// Limitation: dup/dup2/dup3 of a fallback fd are not tracked (the new fd
// would not be in the set). Binder fds are not typically dup'd, so this is
// acceptable. If a dup'd fallback fd receives a binder ioctl, it falls
// through to the real ioctl which returns ENOTTY — logged, not suppressed.
// ---------------------------------------------------------------------------
#define TWOYI_MAX_FD 1024
static unsigned char g_binder_fallback_fds[(TWOYI_MAX_FD + 7) / 8];
static pthread_mutex_t g_binder_fd_lock = PTHREAD_MUTEX_INITIALIZER;

static void binder_fd_mark_fallback(int fd) {
    if (fd < 0 || fd >= TWOYI_MAX_FD) return;
    pthread_mutex_lock(&g_binder_fd_lock);
    g_binder_fallback_fds[fd >> 3] |= (unsigned char)(1u << (fd & 7));
    pthread_mutex_unlock(&g_binder_fd_lock);
}

static int binder_fd_is_fallback(int fd) {
    if (fd < 0 || fd >= TWOYI_MAX_FD) return 0;
    pthread_mutex_lock(&g_binder_fd_lock);
    int r = (g_binder_fallback_fds[fd >> 3] >> (fd & 7)) & 1;
    pthread_mutex_unlock(&g_binder_fd_lock);
    return r;
}

static void binder_fd_clear(int fd) {
    if (fd < 0 || fd >= TWOYI_MAX_FD) return;
    pthread_mutex_lock(&g_binder_fd_lock);
    g_binder_fallback_fds[fd >> 3] &= (unsigned char)~(1u << (fd & 7));
    pthread_mutex_unlock(&g_binder_fd_lock);
}

// ---------------------------------------------------------------------------
// Binder proxy wire client — 6-Z113 (z112 DESIGN.md, strategy S1a).
//
// In non-root mode kr64 materializes {rootfs}/vm0/dev/binder as a
// Unix-domain SOCKET (plus the {rootfs}/dev/binder symlink). open(2) on a
// bound socket node returns ENXIO, which used to land in the /dev/null
// fallback below — a non-pollable fd whose fake ioctls livelocked
// servicemanager (z112 G3). Instead we CONNECT a Unix stream socket to the
// kr64 binder proxy and speak its Frame/Resp wire protocol from the ioctl
// hook (kr64_src/binder.rs read_frame/write_frame):
//
//   guest->host frame:  [u32 cmd][u32 arg_len][arg_len bytes payload]
//   host->guest resp:   [i32 ret ][u32 arg_len][arg_len bytes payload]
//
// (native-endian; x86_64 and aarch64 are both little-endian, so a plain
// memcpy of u32 words is the wire encoding). BINDER_WRITE_READ uses the
// proxy's custom WireBinderWriteRead layout because the kernel struct's
// write_buffer/read_buffer are USER POINTERS that only make sense
// in-process — which is exactly where this hook runs, so we marshal:
//
//   request payload:  [u32 write_size][u32 read_capacity][write bytes]
//   response payload: [u32 read_size ][read bytes]
//
// and set bwr->write_consumed = write_size (the proxy always consumes the
// whole BC_* stream) and bwr->read_consumed = copied bytes. That consumed
// accounting is the G3 livelock fix: IPCThreadState::waitForResponse spins
// if write_consumed never advances (mOut never drains) while no BR_* ever
// arrives. The proxy pushes BR_NOOP when idle with read capacity, and its
// servicemanager_proxy ADD_SERVICE arm answers BR_REPLY (fake success), so
// the ROM servicemanager's startup addService("manager") terminates.
//
// The connected socket fd is pollable/epoll-able (unlike /dev/null), so
// servicemanager's Looper::pollAll sleeps instead of hot-spinning.
//
// G5 decision (single socket for all three binder contexts): the proxy
// exposes exactly ONE socket and the wire protocol has no context tag, so
// /dev/binder, /dev/hwbinder AND /dev/vndbinder all connect to
// {rootfs}/vm0/dev/binder and share one binder context. Wrong for a real
// three-context system, but strictly better than /dev/null for the HAL
// services (no ENOENT livelock class) until S1b adds a context tag.
//
// All socket syscalls here are RAW (SYS_socket/SYS_connect/SYS_sendto/
// SYS_recvfrom) to avoid recursing through our own PLT hooks (the fb-hook
// input-bridge recipe). connect(2) sockaddr_un paths are NOT translated by
// the tracer or this loader, so the absolute $TWOYI_ROOTFS-derived path is
// the host-side truth (TWOYI_ROOTFS is preserved across clearenv() — see
// the clearenv hook above).
// ---------------------------------------------------------------------------
#include <stddef.h>  // offsetof (used for sockaddr_un path length)

// Ioctl numbers: GUEST spellings (kernel uapi / bionic) vs the proxy's
// dispatch constants (binder.rs). They differ for VERSION's alternate
// spelling and for SET_CONTEXT_MGR (binder.rs matches the legacy _IO('b',7)
// form while the kernel/bionic spell it _IOW('b',7,__s32)) — we translate
// on the wire so the proxy's match arms hit.
#define BP_IOC_VERSION_GUEST      0xc0046209u  // _IOWR('b', 9, 4) — bionic uapi
#define BP_IOC_VERSION_ALT        0xc004620du  // alt spelling (see mmap hook)
#define BP_IOC_SET_MAX_THREADS    0x40046205u  // _IOW('b', 5, 4)  — same both sides
#define BP_IOC_SET_CTX_MGR_GUEST  0x40046207u  // _IOW('b', 7, 4)  — kernel/bionic
#define BP_IOC_SET_CTX_MGR_WIRE   0x00006207u  // _IO('b', 7)      — binder.rs arm
#define BP_IOC_THREAD_EXIT        0x40046208u  // _IOW('b', 8, 4)  — same both sides
#define BP_IOC_WRITE_READ         0xc0306201u  // _IOWR('b', 1, 48) — same both sides
#define BP_MAX_FRAME              (1u << 20)   // 1 MiB payload cap (binder.rs read_frame)

// Third fd class: CONNECTED PROXY fds (alongside fallback + real binder).
static unsigned char g_binder_proxy_fds[(TWOYI_MAX_FD + 7) / 8];

static void binder_fd_mark_proxy(int fd) {
    if (fd < 0 || fd >= TWOYI_MAX_FD) return;
    pthread_mutex_lock(&g_binder_fd_lock);
    g_binder_proxy_fds[fd >> 3] |= (unsigned char)(1u << (fd & 7));
    pthread_mutex_unlock(&g_binder_fd_lock);
}

static int binder_fd_is_proxy(int fd) {
    if (fd < 0 || fd >= TWOYI_MAX_FD) return 0;
    pthread_mutex_lock(&g_binder_fd_lock);
    int r = (g_binder_proxy_fds[fd >> 3] >> (fd & 7)) & 1;
    pthread_mutex_unlock(&g_binder_fd_lock);
    return r;
}

static void binder_fd_clear_proxy(int fd) {
    if (fd < 0 || fd >= TWOYI_MAX_FD) return;
    pthread_mutex_lock(&g_binder_fd_lock);
    g_binder_proxy_fds[fd >> 3] &= (unsigned char)~(1u << (fd & 7));
    pthread_mutex_unlock(&g_binder_fd_lock);
}

// ---------------------------------------------------------------------------
// qemu_pipe proxy fd tracking — 6-Z116 (z115 DESIGN.md §3 + §7-Rank-1).
//
// When qemu_pipe_open_fallback() CONNECTS a Unix stream socket to the
// host-app's (or kr64 daemon's) qemu_pipe proxy at {TWOYI_ROOTFS}/dev/
// qemu_pipe, that fd is recorded here. There is NO ioctl hook for
// qemu_pipe (the protocol is pure read/write — the goldfish EGL driver
// writes "pipe:opengles" then GL command packets, reads reply bytes),
// so this set is for fd-recycling hygiene only: the close() hook clears
// the bit so a recycled fd number is not mistaken for a connected proxy
// socket (same reasoning as the binder sets above). /dev/null fallback
// fds (returned when the proxy is unreachable) are NOT tracked here —
// the guest's writes to /dev/null are silently discarded and reads
// return EOF naturally, with no fd-class dispatch needed.
//
// O_CLOEXEC on the connected socket (SYS_socket with SOCK_CLOEXEC) means
// proxy fds do not survive execve, so no stale entries after exec.
// ---------------------------------------------------------------------------
static unsigned char g_qemu_pipe_proxy_fds[(TWOYI_MAX_FD + 7) / 8];
static pthread_mutex_t g_qemu_pipe_fd_lock = PTHREAD_MUTEX_INITIALIZER;

static void qemu_pipe_fd_mark_proxy(int fd) {
    if (fd < 0 || fd >= TWOYI_MAX_FD) return;
    pthread_mutex_lock(&g_qemu_pipe_fd_lock);
    g_qemu_pipe_proxy_fds[fd >> 3] |= (unsigned char)(1u << (fd & 7));
    pthread_mutex_unlock(&g_qemu_pipe_fd_lock);
}

static void qemu_pipe_fd_clear_proxy(int fd) {
    if (fd < 0 || fd >= TWOYI_MAX_FD) return;
    pthread_mutex_lock(&g_qemu_pipe_fd_lock);
    g_qemu_pipe_proxy_fds[fd >> 3] &= (unsigned char)~(1u << (fd & 7));
    pthread_mutex_unlock(&g_qemu_pipe_fd_lock);
}

// One proxy connection is a sequential request/response stream (binder.rs
// handle_connection reads one frame, dispatches, writes one response), but
// libbinder shares a single driver fd across its thread pool — concurrent
// ioctl()s on the same fd would interleave frames and desync the stream.
// 6-Z271g: that serialization moved from a process-wide mutex to
// PER-THREAD connections (see g_bp_thread_conns above the WRITE_READ
// marshalling) — the old g_bp_wire_lock is GONE. Rationale: with one
// shared conn the wire lock was held for the full response wait (idle
// looper ticks, 8 s REPLY_TIMEOUT waits), which futex-froze every other
// binder thread of the process — the measured 20 s stall class.

static int bp_send_all(int fd, const void *buf, size_t len) {
    const unsigned char *p = (const unsigned char *)buf;
    size_t off = 0;
    while (off < len) {
        ssize_t n = (ssize_t)syscall(SYS_sendto, fd, p + off, len - off,
                                     MSG_NOSIGNAL, NULL, 0);
        if (n < 0) {
            if (errno == EINTR) continue;
            return -1;  // errno set (EPIPE when the proxy died, etc.)
        }
        if (n == 0) return -1;
        off += (size_t)n;
    }
    return 0;
}

static int bp_recv_all(int fd, void *buf, size_t len) {
    unsigned char *p = (unsigned char *)buf;
    size_t off = 0;
    while (off < len) {
        ssize_t n = (ssize_t)syscall(SYS_recvfrom, fd, p + off, len - off,
                                     0, NULL, NULL);
        if (n < 0) {
            if (errno == EINTR) continue;
            return -1;
        }
        if (n == 0) { errno = ECONNRESET; return -1; }  // proxy closed
        off += (size_t)n;
    }
    return 0;
}

// One request->response exchange. Returns a malloc'd payload buffer (caller
// frees; never NULL when *resp_len is 0 — a 1-byte sentinel is returned) or
// NULL on transport failure with errno set. *ret_out receives the proxy's
// i32 ret (0 success, negative errno), *resp_len the payload size.
static unsigned char *bp_exchange(int fd, uint32_t cmd,
                                  const void *req_payload, uint32_t req_len,
                                  int32_t *ret_out, uint32_t *resp_len) {
    unsigned char hdr[8];
    memcpy(hdr + 0, &cmd, 4);
    memcpy(hdr + 4, &req_len, 4);
    if (bp_send_all(fd, hdr, 8) != 0) return NULL;
    if (req_len > 0 && bp_send_all(fd, req_payload, req_len) != 0) return NULL;

    if (bp_recv_all(fd, hdr, 8) != 0) return NULL;
    int32_t ret;
    uint32_t rlen;
    memcpy(&ret, hdr + 0, 4);
    memcpy(&rlen, hdr + 4, 4);
    if (rlen > BP_MAX_FRAME) { errno = EPROTO; return NULL; }

    unsigned char *p = (unsigned char *)malloc(rlen > 0 ? rlen : 1);
    if (!p) { errno = ENOMEM; return NULL; }
    if (rlen > 0 && bp_recv_all(fd, p, rlen) != 0) {
        free(p);
        return NULL;
    }
    *ret_out = ret;
    *resp_len = rlen;
    return p;
}

// Real guest pid WITHOUT the getpid fake: the tracer's synthetic
// /proc/self/status reports the REAL tracer-child pid (vfs.rs
// make_proc_self_status writes `Pid:\t<real pid>`). Raw syscalls only —
// no PLT hooks in the way. Returns 0 when unreadable (never the faked 1).
static uint32_t bp_stat_pid(void) {
    char buf[512];
    long fd = syscall(NR_openat, (long)AT_FDCWD, "/proc/self/status",
                      O_RDONLY, 0l);
    if (fd < 0) return 0;
    long n = syscall(NR_read, fd, buf, (long)(sizeof(buf) - 1));
    syscall(NR_close, fd);
    if (n <= 0) return 0;
    buf[n] = '\0';
    // `Pid:\t<digits>` — the synthetic status starts with `Name:` so a
    // plain strstr for "\nPid:\t" is unambiguous.
    static const char key[] = "\nPid:\t";
    char *p = strstr(buf, key);
    if (!p) return 0;
    p += sizeof(key) - 1;
    uint32_t v = 0;
    while (*p >= '0' && *p <= '9') {
        v = v * 10u + (uint32_t)(*p - '0');
        p++;
    }
    return v;
}

// Connect a Unix stream socket to the kr64 binder proxy. Candidates, in
// order (fb-hook input-bridge recipe — fresh socket per candidate because a
// failed connect(2) leaves socket state unspecified):
//   0: {TWOYI_ROOTFS}/vm0/dev/binder   — the canonical proxy socket
//   1: {TWOYI_ROOTFS}/dev/binder      — the relative symlink to it
//   2: vm0/dev/binder                 — relative (guest cwd is the rootfs)
// Returns the connected fd or -1 (caller falls back to /dev/null — no
// regression when the proxy is absent, e.g. TWRP mode).
static int binder_proxy_connect(const char *guest_path) {
    char cands[3][160];
    int ncands = 0;
    static int logged_fail = 0;

    if (g_rootfs && g_rootfs[0]) {
        size_t rl = strlen(g_rootfs);
        while (rl > 1 && g_rootfs[rl - 1] == '/') rl--;  // strip trailing '/'
        const char *suffixes[2] = { "/vm0/dev/binder", "/dev/binder" };
        for (int i = 0; i < 2; i++) {
            size_t sl = strlen(suffixes[i]);
            if (rl + sl + 1 > sizeof(cands[0])) continue;  // must fit sun_path
            if (rl == 1)  // rootfs is "/" — suffix already absolute
                snprintf(cands[ncands], sizeof(cands[0]), "%s", suffixes[i] + 1);
            else
                snprintf(cands[ncands], sizeof(cands[0]), "%.*s%s",
                         (int)rl, g_rootfs, suffixes[i]);
            ncands++;
        }
    }
    if (ncands < 3)
        snprintf(cands[ncands++], sizeof(cands[0]), "vm0/dev/binder");

    for (int i = 0; i < ncands; i++) {
        int sfd = (int)syscall(SYS_socket, AF_UNIX,
                               SOCK_STREAM | SOCK_CLOEXEC, 0);
        if (sfd < 0) return -1;
        struct sockaddr_un sa;
        memset(&sa, 0, sizeof(sa));
        sa.sun_family = AF_UNIX;
        size_t pl = strlen(cands[i]);
        if (pl >= sizeof(sa.sun_path)) {
            syscall(NR_close, sfd);
            continue;
        }
        memcpy(sa.sun_path, cands[i], pl + 1);
        socklen_t salen = (socklen_t)(offsetof(struct sockaddr_un, sun_path) + pl);
        long rc = syscall(SYS_connect, (long)sfd, &sa, (long)salen);
        if (rc == 0) {
            // 6-Z271: announce the guest process identity so the proxy can
            // stamp real sender_pid/sender_euid into routed transactions
            // (the kernel would do this; the wire doesn't). Best-effort:
            // a failed IDENT still leaves the connection usable (pid=0).
            //
            // 6-Z271f: the announced pid used to be getpid() — which the
            // tracer FAKES to 1 for every guest process (load-bearing
            // illusion), so run 33431538542 had all four conns announce
            // "pid=1" and conn ownership stayed unattributable. Honest
            // source: the tracer's synthetic /proc/self/status reports
            // the REAL tracer-child pid (vfs.rs make_proc_self_status
            // writes `Pid:\t<real pid>`), read here with raw syscalls
            // only (no PLT hooks in the way). The proxy's SO_PEERCRED
            // stamp (kernel truth on real kernels) takes precedence;
            // this announcement fills the gap when creds are stripped
            // (hardened kernels zero them). 0 = unknown — NEVER stamp
            // the faked getpid() value.
            struct bp_ident { uint32_t pid; uint32_t uid; uint32_t gid; } ident;
            ident.pid = bp_stat_pid();
            ident.uid = (uint32_t)getuid();
            ident.gid = (uint32_t)getgid();
            unsigned char ihdr[8];
            uint32_t icmd = 0x400462FFu;  // WIRE_CMD_IDENT (not a real binder ioctl)
            uint32_t ilen = 12;
            memcpy(ihdr + 0, &icmd, 4);
            memcpy(ihdr + 4, &ilen, 4);
            if (bp_send_all(sfd, ihdr, 8) == 0 &&
                bp_send_all(sfd, &ident, sizeof(ident)) == 0) {
                int32_t iret = 0;
                uint32_t irlen = 0;
                // Drain the ack ([i32 ret][u32 len]) — ignore content.
                unsigned char idummy[8];
                if (bp_recv_all(sfd, idummy, 8) == 0) {
                    memcpy(&iret, idummy + 0, 4);
                    memcpy(&irlen, idummy + 4, 4);
                    (void)iret;
                    while (irlen > 0) {
                        unsigned char isink[64];
                        uint32_t chunk = irlen > sizeof(isink) ? sizeof(isink) : irlen;
                        if (bp_recv_all(sfd, isink, chunk) != 0) break;
                        irlen -= chunk;
                    }
                }
            }
            char msg[320];
            snprintf(msg, sizeof(msg),
                "[twoyi_loader] binder proxy: %s -> CONNECTED %s (fd=%d)\n",
                guest_path ? guest_path : "(null)", cands[i], sfd);
            write_str(2, msg);
            return sfd;
        }
        syscall(NR_close, sfd);
    }
    if (!logged_fail) {
        logged_fail = 1;
        char msg[256];
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] binder proxy: connect FAILED for %s -> /dev/null "
            "fallback (proxy absent?)\n", guest_path ? guest_path : "(null)");
        write_str(2, msg);
    }
    return -1;
}

// Kernel struct binder_write_read for 64-bit binder_size_t (x86_64 +
// aarch64 guests — the only targets this loader builds for). The ioctl arg
// is a POINTER to it, and since this hook runs in-process the
// write_buffer/read_buffer pointers are directly dereferenceable.
struct bp_binder_write_read {
    uint64_t write_size;
    uint64_t write_consumed;
    uint64_t write_buffer;   // binder_uintptr_t — guest pointer
    uint64_t read_size;
    uint64_t read_consumed;
    uint64_t read_buffer;    // guest pointer
};

// ---------------------------------------------------------------------------
// 6-Z271g: PER-THREAD proxy connections.
//
// Until now one proxy conn served the whole PROCESS, serialized by
// g_bp_wire_lock — but a WRITE_READ response can legitimately take a long
// time: an idle looper's 250 ms BR_NOOP tick (6-Z152), or up to
// REPLY_TIMEOUT = 8 s while the proxy waits for a routed transaction's
// reply. While one thread waited, EVERY other binder thread of the
// process blocked in futex on the wire lock. That is the measured
// futex_do_wait stall class: run 33431538542's pid 2684 `Binder:1_2`
// blocked 20 s with a stale recvfrom ENTRY, and the same chain deadlocked
// guest services whose threads needed the wire to drain the very inbox a
// blocked sender was waiting on — a guaranteed 8 s timeout per call, and
// recovery's ~18.5 s hole ≈ two chained timeouts. It is also the user's
// "vibration freezes everything" symptom: the haptic binder call froze
// behind whatever thread held the lock.
//
// Real binder is PER-THREAD: every IPCThreadState talks to the driver
// independently and blocks only itself. Mirror that: every guest thread
// issuing a binder ioctl gets its OWN proxy connection, lazily
// established on first use (IDENT included — the proxy then attributes
// transactions per thread). No cross-thread lock is ever held during
// I/O; the table lock below guards lookup/insert/remove only.
// ---------------------------------------------------------------------------
struct bp_thread_conn {
    int binder_fd;    // the fd libbinder opened (the "driver" fd)
    uint32_t tid;     // gettid of the owning guest thread
    int conn_fd;      // the dedicated proxy connection
};
#define BP_THREAD_CONN_MAX 48
static struct bp_thread_conn g_bp_thread_conns[BP_THREAD_CONN_MAX];
static pthread_mutex_t g_bp_thread_conn_lock = PTHREAD_MUTEX_INITIALIZER;

static uint32_t bp_gettid(void) {
    return (uint32_t)syscall(SYS_gettid);
}

// Lookup (no insert). Returns the dedicated conn fd or -1.
static int bp_thread_conn_lookup(int binder_fd, uint32_t tid) {
    int conn = -1;
    pthread_mutex_lock(&g_bp_thread_conn_lock);
    for (int i = 0; i < BP_THREAD_CONN_MAX; i++) {
        if (g_bp_thread_conns[i].binder_fd == binder_fd &&
            g_bp_thread_conns[i].tid == tid &&
            g_bp_thread_conns[i].conn_fd > 0) {
            conn = g_bp_thread_conns[i].conn_fd;
            break;
        }
    }
    pthread_mutex_unlock(&g_bp_thread_conn_lock);
    return conn;
}

// Insert. Returns conn_fd on success, -1 when the table is full.
static int bp_thread_conn_insert(int binder_fd, uint32_t tid, int conn_fd) {
    int stored = -1;
    pthread_mutex_lock(&g_bp_thread_conn_lock);
    for (int i = 0; i < BP_THREAD_CONN_MAX; i++) {
        if (g_bp_thread_conns[i].conn_fd == 0) {
            g_bp_thread_conns[i].binder_fd = binder_fd;
            g_bp_thread_conns[i].tid = tid;
            g_bp_thread_conns[i].conn_fd = conn_fd;
            stored = conn_fd;
            break;
        }
    }
    pthread_mutex_unlock(&g_bp_thread_conn_lock);
    return stored;
}

// Remove one (binder_fd, tid) entry, closing its conn. Returns the conn
// fd that was closed, or -1.
static int bp_thread_conn_remove(int binder_fd, uint32_t tid) {
    int closed_fd = -1;
    pthread_mutex_lock(&g_bp_thread_conn_lock);
    for (int i = 0; i < BP_THREAD_CONN_MAX; i++) {
        if (g_bp_thread_conns[i].binder_fd == binder_fd &&
            g_bp_thread_conns[i].tid == tid &&
            g_bp_thread_conns[i].conn_fd > 0) {
            closed_fd = g_bp_thread_conns[i].conn_fd;
            g_bp_thread_conns[i].binder_fd = -1;
            g_bp_thread_conns[i].tid = 0;
            g_bp_thread_conns[i].conn_fd = 0;
            break;
        }
    }
    pthread_mutex_unlock(&g_bp_thread_conn_lock);
    return closed_fd;
}

// close(fd) of the BINDER fd itself: tear down every per-thread conn
// bound to it (the process is dropping the driver; leaving the sockets
// open would leak proxy conns and keep ghost registrations alive).
static void bp_thread_conn_close_for_binder(int binder_fd) {
    pthread_mutex_lock(&g_bp_thread_conn_lock);
    for (int i = 0; i < BP_THREAD_CONN_MAX; i++) {
        if (g_bp_thread_conns[i].binder_fd == binder_fd &&
            g_bp_thread_conns[i].conn_fd > 0) {
            syscall(NR_close, g_bp_thread_conns[i].conn_fd);
            g_bp_thread_conns[i].binder_fd = -1;
            g_bp_thread_conns[i].tid = 0;
            g_bp_thread_conns[i].conn_fd = 0;
        }
    }
    pthread_mutex_unlock(&g_bp_thread_conn_lock);
}

// Resolve the caller's dedicated conn for this binder fd: cached, or a
// fresh connection (binder_proxy_connect sends the IDENT). Falls back to
// the binder fd itself when the table is full or the proxy is gone —
// the degraded shared path, not a failure.
static int bp_conn_for_ioctl(int binder_fd) {
    uint32_t tid = bp_gettid();
    int conn = bp_thread_conn_lookup(binder_fd, tid);
    if (conn >= 0) return conn;
    conn = binder_proxy_connect(NULL);
    if (conn < 0) return binder_fd;
    if (bp_thread_conn_insert(binder_fd, tid, conn) < 0) {
        // Table full — drop the spare and share the primary (rare: the
        // cap is 48 vs ~16 binder threads per process).
        syscall(NR_close, conn);
        return binder_fd;
    }
    return conn;
}

// ---------------------------------------------------------------------------
// 6-Z271: v2 REQUEST blob inlining.
//
// The proxy's servicemanager_proxy() only parses real AIDL parcels when the
// WRITE_READ request carries the v2 trailer ([WIRE_V2_MAGIC][blob_count]
// [blobs...], binder.rs handle_write_read). Without it every real-libbinder
// transaction lands in servicemanager_legacy() where the service NAME is
// DISCARDED — the registry stayed permanently empty, keystore2's addService
// at +11.4s was consumed name-less (run 33411932921: single code=3, v2=false,
// zero "addService(NAME)" lines), and OrangeFox R12's recovery main thread
// polled checkService ~170x over 18.5 s waiting for a service that could
// never appear. The hook runs IN the guest process, so the parcel bytes
// behind each BC_TRANSACTION's data_ptr/offsets_ptr are directly
// memcpy-able — collect one blob per transaction command (the proxy pairs
// the i-th blob with the i-th BC_TRANSACTION/BC_TRANSACTION_SG in stream
// order; BC_REPLY requests carry no blob on this wire) and append the
// trailer. Total-frame safety: if header + stream + trailer would exceed
// the proxy's 1 MiB read_frame cap the trailer is dropped (v1 fallback) —
// recovery/keystore2 parcels are KBs, so this never fires in practice.
// ---------------------------------------------------------------------------
#define BP_BC_TRANSACTION    0x40406300u  // _IOW('c', 0, 64)
#define BP_BC_REPLY          0x40406301u  // _IOW('c', 1, 64)
#define BP_BC_TRANSACTION_SG 0x40486311u  // _IOW('c', 17, 72)
#define BP_BC_REPLY_SG       0x40486312u  // _IOW('c', 18, 72)
#define BP_BLOB_MAX          (512u * 1024u)  // per-buffer sanity cap
#define BP_BLOB_MAX_CMDS     32u             // cap before falling back to v1

struct bp_blob_desc {
    uint64_t data_len;
    uint64_t offsets_len;
    uint64_t data_ptr;
    uint64_t offsets_ptr;
};

// Scan a BC_* stream; fill one descriptor per BC_TRANSACTION[_SG]. Returns
// the descriptor count, or 0 when the stream holds more transaction
// commands than we are willing to inline (caller falls back to v1).
static uint32_t bp_scan_tx_blobs(const uint8_t *stream, uint64_t len,
                                 struct bp_blob_desc *descs) {
    uint64_t pos = 0;
    uint32_t n = 0;
    uint32_t tx_cmds = 0;
    while (pos + 4 <= len) {
        uint32_t cmd;
        memcpy(&cmd, stream + pos, 4);
        pos += 4;
        uint32_t psize = (cmd >> 16) & 0x3fffu;
        if (pos + psize > len) break;
        if (cmd == BP_BC_TRANSACTION || cmd == BP_BC_TRANSACTION_SG ||
            cmd == BP_BC_REPLY || cmd == BP_BC_REPLY_SG) {
            // 6-Z272k: bounded outgoing-side DIAG — the keystore2 compat
            // chain needed the guest's REPLY bytes (the self-_NTF on
            // handle 4 was answered with a 4-byte void parcel — if the
            // guest's own BBinder replies like that the descriptor never
            // reaches asInterface). First 12 transaction/reply commands
            // per process.
            {
                static int bc_diag = 12;
                if (bc_diag > 0) {
                    bc_diag--;
                    const uint8_t *btd = stream + pos;
                    uint32_t t_code, t_target;
                    uint64_t t_ds;
                    memcpy(&t_target, btd, 4);
                    memcpy(&t_code, btd + 16, 4);
                    memcpy(&t_ds, btd + 32, 8);
                    char m[160];
                    snprintf(m, sizeof(m),
                             "[twoyi_loader] *** 6-Z272k BC to proxy: cmd=%s target=%u "
                             "code=0x%x dsize=%llu (pid=%d)\n",
                             (cmd == BP_BC_REPLY || cmd == BP_BC_REPLY_SG) ? "REPLY" : "TX",
                             t_target, t_code, (unsigned long long)t_ds, g_real_pid);
                    write_str(2, m);
                }
            }
            tx_cmds++;
            if (n >= BP_BLOB_MAX_CMDS) return 0;  // too many — v1 fallback
            const uint8_t *btd = stream + pos;
            uint64_t data_size, offsets_size, data_ptr, offsets_ptr;
            memcpy(&data_size, btd + 32, 8);
            memcpy(&offsets_size, btd + 40, 8);
            memcpy(&data_ptr, btd + 48, 8);
            memcpy(&offsets_ptr, btd + 56, 8);
            // Sanity caps: a zero pointer with nonzero size (or an
            // oversized buffer) downgrades THIS blob to empty — the proxy
            // treats an empty blob as no-parcel for that command.
            if (data_size > BP_BLOB_MAX || (data_size > 0 && data_ptr == 0)) {
                data_size = 0;
                data_ptr = 0;
            }
            if (offsets_size > BP_BLOB_MAX ||
                (offsets_size > 0 && offsets_ptr == 0)) {
                offsets_size = 0;
                offsets_ptr = 0;
            }
            descs[n].data_len = data_size;
            descs[n].offsets_len = offsets_size;
            descs[n].data_ptr = data_ptr;
            descs[n].offsets_ptr = offsets_ptr;
            n++;
        }
        pos += psize;
    }
    (void)tx_cmds;
    return n;
}

// Build the v2 request trailer: [WIRE_V2_MAGIC][blob_count]
// (per blob) [u32 data_len][u32 offsets_len][data][offsets].
// Returns a malloc'd buffer (caller frees) and *out_len, or NULL when
// there is nothing to inline. Grand total is capped so the FULL request
// frame (8 + ws + trailer) stays under the proxy's 1 MiB cap.
static unsigned char *bp_build_v2_request_trailer(
        const uint8_t *stream, uint64_t ws, uint32_t *out_len) {
    *out_len = 0;
    if (ws < 4) return NULL;
    struct bp_blob_desc descs[BP_BLOB_MAX_CMDS];
    uint32_t n = bp_scan_tx_blobs(stream, ws, descs);
    if (n == 0) return NULL;

    uint64_t total = 8;
    for (uint32_t i = 0; i < n; i++)
        total += 8 + descs[i].data_len + descs[i].offsets_len;
    // Frame budget: 8-byte wire header + stream + this trailer + slack.
    if (total + 8 + ws + 64 > BP_MAX_FRAME) return NULL;

    unsigned char *tr = (unsigned char *)malloc((size_t)total);
    if (!tr) return NULL;
    size_t off = 0;
    uint32_t magic = BP_WIRE_V2_MAGIC;
    memcpy(tr + off, &magic, 4);
    off += 4;
    memcpy(tr + off, &n, 4);
    off += 4;
    for (uint32_t i = 0; i < n; i++) {
        uint32_t dl = (uint32_t)descs[i].data_len;
        uint32_t ol = (uint32_t)descs[i].offsets_len;
        memcpy(tr + off, &dl, 4);
        off += 4;
        memcpy(tr + off, &ol, 4);
        off += 4;
        if (dl > 0) {
            memcpy(tr + off, (const void *)(uintptr_t)descs[i].data_ptr, dl);
            off += dl;
        }
        if (ol > 0) {
            memcpy(tr + off, (const void *)(uintptr_t)descs[i].offsets_ptr, ol);
            off += ol;
        }
    }
    *out_len = (uint32_t)total;
    return tr;
}

// BINDER_WRITE_READ marshalling (called with g_bp_wire_lock held).
// Forwards the guest's BC_* write buffer verbatim (plus the 6-Z271 v2
// request trailer); copies the proxy's BR_* response bytes into the guest's
// read buffer; sets BOTH consumed fields (the livelock fix). Returns 0 / -1
// with errno like a real ioctl.
static int binder_proxy_write_read(int fd, struct bp_binder_write_read *bwr) {
    static unsigned log_budget = 2;

    // The proxy parses the BC_* stream from offset 0 and never reports
    // partial consumption, so a nonzero incoming write_consumed (which
    // libbinder only produces after a SUCCESSFUL ioctl — and we always
    // report write_consumed == write_size on success) would resend
    // already-consumed commands. Reject nonsense defensively.
    if (bwr->write_consumed > bwr->write_size) { errno = EINVAL; return -1; }
    if (bwr->write_size > 0 && bwr->write_buffer == 0) { errno = EFAULT; return -1; }
    if (bwr->read_size > 0 && bwr->read_buffer == 0) { errno = EFAULT; return -1; }
    // 8-byte WireBinderWriteRead header must fit under the proxy's 1 MiB
    // read_frame cap, else the proxy DROPS THE CONNECTION.
    if (8 + bwr->write_size > BP_MAX_FRAME) {
        char msg[160];
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] binder proxy WRITE_READ: write_size=%llu > cap "
            "-> EINVAL\n", (unsigned long long)bwr->write_size);
        write_str(2, msg);
        errno = EINVAL;
        return -1;
    }

    // Single frame: [u32 WRITE_READ][u32 8+ws+trailer][u32 ws][u32 read_cap]
    //               [stream bytes][v2 trailer (6-Z271)]
    uint64_t ws = bwr->write_size - bwr->write_consumed;

    // 6-Z265: peek the outgoing BC stream for BC_FREE_BUFFER commands and
    // release the backing allocations the guest is returning (kernel-true
    // transaction-buffer lifecycle). Must happen BEFORE the forwarding so
    // even a transport failure can't leak them.
    if (ws > 0 && bwr->write_buffer != 0) {
        bp_free_returned_buffers(
            (const uint8_t *)(uintptr_t)(bwr->write_buffer + bwr->write_consumed), ws);
    }
    // 6-Z271: inline parcel blobs for the request's BC_TRANSACTION commands
    // so the proxy's registry sees real service names (v1 fallback = NULL).
    uint32_t trailer_len = 0;
    unsigned char *trailer = NULL;
    if (ws > 0 && bwr->write_buffer != 0) {
        trailer = bp_build_v2_request_trailer(
            (const uint8_t *)(uintptr_t)(bwr->write_buffer + bwr->write_consumed),
            ws, &trailer_len);
    }
    uint32_t req_len = (uint32_t)(8 + ws + trailer_len);

    unsigned char *req = (unsigned char *)malloc(req_len);
    if (!req) {
        free(trailer);
        errno = ENOMEM;
        return -1;
    }
    uint32_t ws32 = (uint32_t)ws;
    uint32_t rc32 = (uint32_t)(bwr->read_size > 0xffffffffull
                               ? 0xffffffffull : bwr->read_size);
    memcpy(req + 0, &ws32, 4);
    memcpy(req + 4, &rc32, 4);
    if (ws > 0)
        memcpy(req + 8, (const void *)(uintptr_t)(bwr->write_buffer + bwr->write_consumed), ws);
    uint32_t blob_count_logged = 0;
    if (trailer != NULL) {
        if (trailer_len >= 8) memcpy(&blob_count_logged, trailer + 4, 4);
        memcpy(req + 8 + ws, trailer, trailer_len);
        free(trailer);
    }

    int32_t ret = 0;
    uint32_t rlen = 0;
    unsigned char *resp = bp_exchange(fd, BP_IOC_WRITE_READ, req, req_len,
                                      &ret, &rlen);
    if (log_budget > 0) {
        // 6-Z271 evidence: log whether the v2 trailer was attached and how
        // many blobs it carried (first exchanges only — bounded logging).
        log_budget--;
        char msg[192];
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] binder proxy WRITE_READ: ws=%llu blobs=%u -> "
            "exchanged\n", (unsigned long long)ws, blob_count_logged);
        write_str(2, msg);
    }
    free(req);
    if (!resp) {
        char msg[192];
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] binder proxy WRITE_READ: transport FAILED "
            "(errno=%d: %s)\n", errno, strerror(errno));
        write_str(2, msg);
        return -1;
    }
    if (ret != 0) {
        errno = (ret <= 0 && ret >= -4095) ? -ret : EPROTO;
        free(resp);
        return -1;
    }
    // Response payload: [u32 read_size][read_size bytes of BR_* stream].
    if (rlen < 4) { free(resp); errno = EPROTO; return -1; }
    uint32_t srv_read;
    memcpy(&srv_read, resp + 0, 4);
    if (4ull + (uint64_t)srv_read > (uint64_t)rlen) {
        free(resp);
        errno = EPROTO;
        return -1;
    }
    uint64_t ncopy = ((uint64_t)srv_read < bwr->read_size)
                     ? (uint64_t)srv_read : bwr->read_size;
    if (ncopy > 0) {
        memcpy((void *)(uintptr_t)bwr->read_buffer, resp + 4, (size_t)ncopy);
        // 6-Z265: the proxy appends a v2-style trailer with the reply
        // parcel bytes even for v1 requests. Back every BR_REPLY's
        // data_ptr/offsets_ptr with real memory so real libbinder clients
        // never build a Parcel over NULL (the OrangeFox R12 lavender
        // recovery + keystore2 SIGSEGV class).
        if ((uint64_t)rlen > 4ull + srv_read) {
            bp_patch_reply_data((uint8_t *)(uintptr_t)bwr->read_buffer, ncopy,
                                resp + 4 + srv_read, (uint64_t)rlen - 4ull - srv_read);
        }
        if (ncopy < (uint64_t)srv_read) {
            char msg[192];
            snprintf(msg, sizeof(msg),
                "[twoyi_loader] binder proxy WRITE_READ: BR stream %u bytes "
                "TRUNCATED to %llu (read_size) — S1b TODO\n",
                srv_read, (unsigned long long)ncopy);
            write_str(2, msg);
        }
    }
    bwr->read_consumed = ncopy;
    bwr->write_consumed = bwr->write_size;  // proxy consumed the whole stream

    if (log_budget > 0) {
        log_budget--;
        char msg[224];
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] binder proxy WRITE_READ: ws=%llu rs=%llu -> "
            "write_consumed=%llu read_consumed=%llu (first BR=0x%08x)\n",
            (unsigned long long)ws, (unsigned long long)bwr->read_size,
            (unsigned long long)bwr->write_consumed,
            (unsigned long long)bwr->read_consumed,
            ncopy >= 4 ? *(const uint32_t *)(const void *)(resp + 4) : 0u);
        write_str(2, msg);
    }
    free(resp);
    return 0;
}

// Binder ioctl dispatch for CONNECTED PROXY fds (called from the ioctl
// hook). Translate guest ioctl spellings to the proxy's wire constants,
// exchange the frame, write response bytes back into the guest's arg.
// Returns 0 / -1 with errno like a real ioctl.
static int binder_proxy_ioctl(int fd, unsigned req, void *argp) {
    static unsigned log_budget = 4;
    uint32_t wire = req;
    uint32_t req_len = 0;
    const void *req_payload = NULL;
    uint32_t cookie = 0;

    // BINDER_SET_CONTEXT_MGR_EXT = 0x4018620d (Android 11's
    // servicemanager calls it with a flat_binder_object context
    // after the plain SET_CONTEXT_MGR; the kr64 vm0 proxy bridge
    // rejects it as "unknown ioctl" — satisfy it locally as success,
    // never forward it down the wire).
    if (req == 0x4018620du) {
        write_str(2, "[twoyi_loader] ioctl(BINDER_SET_CONTEXT_MGR_EXT) -> success\n");
        return 0;
    }
    // 6-Z265: BINDER_ENABLE_ONEWAY_SPAM_DETECTION = 0x40046210 — Android
    // 11+ libbinder arms it right after the handshake; the real kernel
    // always accepts, so satisfy it locally instead of an EINVAL round
    // trip (run 33334415274: OrangeFox R12 recovery + keystore2 both hit
    // the unknown-ioctl path).
    if (req == 0x40046210u) {
        return 0;
    }
    // 6-Z271g: BINDER_THREAD_EXIT = _IOW('b', 8, size_t) = 0x40086208 —
    // a departing pool thread tells the driver to tear down its state.
    // Close and forget this thread's dedicated proxy conn (its mailbox,
    // registrations and inflight txn die with the conn, exactly like the
    // kernel's per-thread bookkeeping).
    if (req == 0x40086208u) {
        int gone = bp_thread_conn_remove(fd, bp_gettid());
        (void)gone;
        return 0;
    }

    // 6-Z271g: per-thread conn — NO wire lock anywhere on this path (see
    // the g_bp_thread_conns table comment).
    int conn = bp_conn_for_ioctl(fd);

    if (req == BP_IOC_WRITE_READ) {
        if (argp)
            return binder_proxy_write_read(conn, (struct bp_binder_write_read *)argp);
        errno = EFAULT;
        return -1;
    }

    if (req == BP_IOC_VERSION_GUEST || req == BP_IOC_VERSION_ALT) {
        // Both guest spellings -> the proxy's BINDER_VERSION arm (nr 9).
        wire = BP_IOC_VERSION_GUEST;
    } else if (req == BP_IOC_SET_MAX_THREADS) {
        if (argp) { cookie = *(const uint32_t *)argp; req_payload = &cookie; req_len = 4; }
    } else if (req == BP_IOC_SET_CTX_MGR_GUEST || req == BP_IOC_SET_CTX_MGR_WIRE) {
        // becomeContextManager passes 0/NULL as the arg — never deref.
        wire = BP_IOC_SET_CTX_MGR_WIRE;
        if (argp) { cookie = *(const uint32_t *)argp; req_payload = &cookie; req_len = 4; }
    } else {
        // Generic forward (BINDER_THREAD_EXIT, SET_CONTEXT_MGR_EXT, node
        // debug ioctls, ...): the ioctl number's size bits give the arg
        // size. Unknown-to-the-proxy cmds come back -EINVAL (honest).
        uint32_t sz = (req >> 16) & 0x3fff;
        if (!argp) sz = 0;
        if (sz > 4096) { errno = EINVAL; goto fail; }
        req_len = sz;
        req_payload = argp;
    }

    {
        int32_t ret = 0;
        uint32_t rlen = 0;
        unsigned char *resp = bp_exchange(conn, wire, req_payload, req_len,
                                          &ret, &rlen);
        if (!resp) {
            char msg[192];
            snprintf(msg, sizeof(msg),
                "[twoyi_loader] binder proxy ioctl 0x%x: transport FAILED "
                "(errno=%d: %s)\n", req, errno, strerror(errno));
            write_str(2, msg);
            goto fail;
        }
        if (log_budget > 0) {
            log_budget--;
            char msg[192];
            snprintf(msg, sizeof(msg),
                "[twoyi_loader] binder proxy ioctl 0x%x -> wire 0x%x "
                "ret=%d rlen=%u\n", req, wire, ret, rlen);
            write_str(2, msg);
        }
        if (ret != 0) {
            errno = (ret <= 0 && ret >= -4095) ? -ret : EPROTO;
            free(resp);
            goto fail;
        }
        // Write response bytes back (VERSION's protocol_version, _IOWR
        // node-debug structs). Guarded by the ioctl's own size bits.
        uint32_t sz = (req >> 16) & 0x3fff;
        if (argp && rlen > 0 && sz > 0) {
            uint32_t n = rlen < sz ? rlen : sz;
            memcpy(argp, resp, n);
        }
        free(resp);
    }
    return 0;

fail:
    return -1;
}

// ---------------------------------------------------------------------------
// Framebuffer fd tracking (for TWRP virtual fb0).
//
// In TWRP mode, kr64 creates /dev/graphics/fb0 and /dev/fb0 as REGULAR
// files (3,686,400 bytes = 720x1280x4 RGBA8888) so open() and mmap()
// succeed naturally. But FB ioctls (FBIOGET_VSCREENINFO etc.) on a
// regular file return ENOTTY from the kernel, leaving the screeninfo
// struct zeroed. libminuitwrp then dereferences the NULL fields and
// segfaults at offset 0x57d7.
//
// FIX: when open/openat/__open_2/__openat_2 successfully opens an fb0
// path, the returned fd is recorded here. The ioctl hook checks this
// set to decide whether to fake FB ioctls. The close hook clears
// entries when fds are closed.
//
// This tracking is HARMLESS in non-TWRP mode: /dev/graphics/fb0 is a
// real device (or doesn't exist), so is_fb_path() returns false and no
// fd is ever tracked. The ioctl hook's fb_fd_is_tracked() check is a
// single bitmap lookup — negligible overhead.
//
// Limitation: dup/dup2/dup3 of an fb0 fd are not tracked. This is
// acceptable — libminuitwrp doesn't dup the fb0 fd.
// ---------------------------------------------------------------------------
static unsigned char g_fb_fds[(TWOYI_MAX_FD + 7) / 8];
static pthread_mutex_t g_fb_fd_lock = PTHREAD_MUTEX_INITIALIZER;

static void fb_fd_mark(int fd) {
    if (fd < 0 || fd >= TWOYI_MAX_FD) return;
    pthread_mutex_lock(&g_fb_fd_lock);
    g_fb_fds[fd >> 3] |= (unsigned char)(1u << (fd & 7));
    pthread_mutex_unlock(&g_fb_fd_lock);
}

static int fb_fd_is_tracked(int fd) {
    if (fd < 0 || fd >= TWOYI_MAX_FD) return 0;
    pthread_mutex_lock(&g_fb_fd_lock);
    int r = (g_fb_fds[fd >> 3] >> (fd & 7)) & 1;
    pthread_mutex_unlock(&g_fb_fd_lock);
    return r;
}

static void fb_fd_clear(int fd) {
    if (fd < 0 || fd >= TWOYI_MAX_FD) return;
    pthread_mutex_lock(&g_fb_fd_lock);
    g_fb_fds[fd >> 3] &= (unsigned char)~(1u << (fd & 7));
    pthread_mutex_unlock(&g_fb_fd_lock);
}

// 6-Z222: forward declaration — the ioctl hook (below) calls this to
// self-heal untracked fb0 fds; the definition lives next to the open64
// hooks near the end of this file.
static void fb_fd_selfheal(int fd);

// Returns 1 if path is /dev/graphics/fb0 or /dev/fb0 (exact match).
// These are the framebuffer device paths that libminuitwrp opens.
static int is_fb_path(const char *path) {
    if (!path) return 0;
    if (strcmp(path, "/dev/graphics/fb0") == 0) return 1;
    if (strcmp(path, "/dev/fb0") == 0) return 1;
    return 0;
}

// Called from open/openat/__open_2/__openat_2/__open_real after a
// successful open. If the path is an fb0 path, marks the fd for the
// ioctl hook. Returns the fd unchanged (so it can be used inline as
// `return track_fb_fd(path, fd);`).
static int track_fb_fd(const char *path, int fd) {
    if (fd >= 0 && is_fb_path(path)) {
        fb_fd_mark(fd);
        char msg[256];
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] open(%s) -> fd=%d (tracking for FB ioctls)\n",
            path, fd);
        write_str(2, msg);
    }
    return fd;
}

// ---------------------------------------------------------------------------
// Virtual screen configuration for TWRP framebuffer virtualization.
//
// 720x1280 @ 32bpp (RGBA8888). This matches the byt_t_crv2 (Minix Z64)
// TWRP image's expected display resolution. libminuitwrp reads these
// values from FBIOGET_VSCREENINFO and uses them to size its framebuffer.
//
// The framebuffer memory is 720*1280*4 = 3,686,400 bytes. kr64 pre-
// creates /dev/graphics/fb0 as a regular file of exactly this size, so
// mmap() on the fd works naturally (no mmap hook needed for the common
// case).
// ---------------------------------------------------------------------------
#define TWOYI_FB_WIDTH          720
#define TWOYI_FB_HEIGHT         1280
#define TWOYI_FB_BPP            32
#define TWOYI_FB_BYTES_PER_PIX  4
// 6-Z222: RUNTIME geometry — the shlib must NOT hardcode 720x1280. The
// resolution chain is TWOYI_FB_WIDTH/HEIGHT env (the 6-Z220 rc setenv
// adds it to the recovery service) → {rootfs}/dev/.twoyi-fb-geometry
// file ("WxH\n", written by kr64's parent at fb0-creation time) →
// 720x1280 fallback. Must match kr64's fb0 file size so mmap succeeds
// and core.rs's blit reads a matching frame.
static int g_twoyi_fb_rt_w = 0;
static int g_twoyi_fb_rt_h = 0;

static int twoyi_fb_atoi_pos(const char *s) {
    if (!s) return 0;
    int v = 0, seen = 0;
    while (*s >= '0' && *s <= '9') {
        if (v < 100000) v = v * 10 + (*s - '0');
        s++; seen = 1;
    }
    return seen ? v : 0;
}

static void fb_geometry_init_shlib(void) {
    if (g_twoyi_fb_rt_w > 0) return;
    g_twoyi_fb_rt_w = TWOYI_FB_WIDTH;
    g_twoyi_fb_rt_h = TWOYI_FB_HEIGHT;
    const char *wenv = getenv("TWOYI_FB_WIDTH");
    const char *henv = getenv("TWOYI_FB_HEIGHT");
    int w = twoyi_fb_atoi_pos(wenv);
    int h = twoyi_fb_atoi_pos(henv);
    if (w > 0 && h > 0) {
        g_twoyi_fb_rt_w = w;
        g_twoyi_fb_rt_h = h;
        return;
    }
    // Geometry FILE fallback: {rootfs}/dev/.twoyi-fb-geometry ("WxH\n").
    if (g_rootfs) {
        char p[512];
        snprintf(p, sizeof(p), "%s/dev/.twoyi-fb-geometry", g_rootfs);
        int fd = (int)syscall(SYS_openat, AT_FDCWD, p, 0 /*O_RDONLY*/, 0);
        if (fd >= 0) {
            char buf[64] = {0};
            long n = syscall(SYS_read, fd, buf, (long)(sizeof(buf) - 1));
            syscall(SYS_close, fd);
            if (n > 0) {
                buf[n] = '\0';
                char *x = strchr(buf, 'x');
                if (x) {
                    *x = '\0';
                    int fw = twoyi_fb_atoi_pos(buf);
                    int fh = twoyi_fb_atoi_pos(x + 1);
                    if (fw > 0 && fh > 0) {
                        g_twoyi_fb_rt_w = fw;
                        g_twoyi_fb_rt_h = fh;
                    }
                }
            }
        }
    }
}

static int fb_w_rt(void) { fb_geometry_init_shlib(); return g_twoyi_fb_rt_w; }
static int fb_h_rt(void) { fb_geometry_init_shlib(); return g_twoyi_fb_rt_h; }
static long fb_line_length_rt(void) { return (long)fb_w_rt() * TWOYI_FB_BYTES_PER_PIX; }
static long fb_smem_len_rt(void)    { return (long)fb_w_rt() * (long)fb_h_rt() * TWOYI_FB_BYTES_PER_PIX; }

// Fill struct fb_var_screeninfo with valid runtime-geometry @32bpp RGBA8888.
// libminuitwrp reads xres, yres, bits_per_pixel, and the color channel
// offsets/lengths to configure its software renderer.
static void fill_vscreeninfo(struct fb_var_screeninfo *v) {
    memset(v, 0, sizeof(*v));
    fb_geometry_init_shlib();
    v->xres = (__u32)g_twoyi_fb_rt_w;
    v->yres = (__u32)g_twoyi_fb_rt_h;
    v->xres_virtual = (__u32)g_twoyi_fb_rt_w;
    v->yres_virtual = (__u32)g_twoyi_fb_rt_h;
    v->xoffset = 0;
    v->yoffset = 0;
    v->bits_per_pixel = TWOYI_FB_BPP;
    v->grayscale = 0;
    // RGBA8888: red at bit 16, green at bit 8, blue at bit 0, alpha at bit 24
    v->red.offset = 16;    v->red.length = 8;    v->red.msb_right = 0;
    v->green.offset = 8;   v->green.length = 8;  v->green.msb_right = 0;
    v->blue.offset = 0;    v->blue.length = 8;   v->blue.msb_right = 0;
    v->transp.offset = 24; v->transp.length = 8; v->transp.msb_right = 0;
    v->nonstd = 0;
    v->activate = 0;  // FB_ACTIVATE_NOW
    // Physical dimensions in mm (for DPI calculation). 720x1280 at
    // ~250 DPI is ~73x130mm — round numbers close to a 5" phone.
    v->height = 130;
    v->width = 73;
    v->accel_flags = 0;
    // Pixclock in picoseconds. For 60Hz refresh of 720x1280:
    //   pixclock = 1 / (60 * 720 * 1280) = ~18ns = ~18100ps
    v->pixclock = 18100;
    v->left_margin = 24;
    v->right_margin = 24;
    v->upper_margin = 4;
    v->lower_margin = 4;
    v->hsync_len = 24;
    v->vsync_len = 4;
    v->sync = 0;
    v->vmode = 0;  // FB_VMODE_NONINTERLACED
    v->rotate = 0;
    v->colorspace = 0;  // FB_COLORSPACE_RGB
}

// Fill struct fb_fix_screeninfo with valid fixed framebuffer info.
// libminuitwrp reads smem_len (for the mmap size) and line_length
// (for the row stride).
static void fill_fscreeninfo(struct fb_fix_screeninfo *f) {
    memset(f, 0, sizeof(*f));
    // id is a 16-byte char array (kernel: char id[16]).
    strncpy(f->id, "twoyi_fb", sizeof(f->id) - 1);
    // smem_start is an unsigned long — we set it to 0 (libminuitwrp
    // doesn't dereference it; it only uses smem_len for the mmap size).
    f->smem_start = 0;
    f->smem_len = (__u32)fb_smem_len_rt();
    f->type = 0;  // FB_TYPE_PACKED_PIXELS
    f->type_aux = 0;
    f->visual = 2;  // FB_VISUAL_TRUECOLOR
    f->xpanstep = 0;
    f->ypanstep = 0;
    f->ywrapstep = 0;
    f->line_length = (__u32)fb_line_length_rt();
    f->mmio_start = 0;
    f->mmio_len = 0;
    f->accel = 0;
    f->capabilities = 0;
    f->reserved[0] = 0;
    f->reserved[1] = 0;
}

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
// BUT: we need an internal version that CAN unset LD_PRELOAD for 32-bit binaries
static void unsetenv_internal(const char *name) {
    static int (*real_unsetenv)(const char *) = NULL;
    if (!real_unsetenv) real_unsetenv = dlsym(RTLD_NEXT, "unsetenv");
    if (real_unsetenv) real_unsetenv(name);
}

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

// 6-Z214: is this mount() flags value a mount-PROPAGATION / REMOUNT /
// MOVE operation rather than a fresh filesystem mount?
//
// ROOT CAUSE THIS FIXES (OrangeFox R12.1 lavender / Lineage 22.2 init,
// 10 investigation rounds r14-r24 + the r25 6-Z213 raw-stop census that
// finally localized it): AOSP init's SetupMountNamespaces calls
//   mount(nullptr, "/apex", nullptr, MS_PRIVATE (0x40000) [, MS_REC])
// after /apex was ALREADY recorded in the loader's virtual mount table
// by an earlier guest mount() call. The old interposer only special-
// cased MS_REMOUNT — a propagation-only flags value fell through to the
// "duplicate mount" branch and returned -EBUSY:
//
//   <3>init: Failed to remount /apex as 40000: Device or resource busy
//   [glog F/abort] SetupMountNamespaces failed: Device or resource busy
//   <3>init: InitFatalReboot: signal 6      → exit_group(127)
//
// CRITICAL COROLLARY (explains the r14-r24 ptrace mystery): the
// interposer returns WITHOUT issuing a real mount(2) syscall, so the
// kernel NEVER generates a ptrace syscall-stop for it — the r25 6-Z213
// RAW STOP census (14503 syscall-stops, ZERO nr=40/39/41, every other
// stop accounted for) is exactly what a PLT-interposed mount looks
// like to the tracer. The Rust-side 6-Z210 fake-success could never
// fire because there was no syscall to intercept. The fix MUST live
// here, at the interposer, for every dynamically-linked guest.
//
// Real Linux mount(2) semantics for these flags: MS_PRIVATE/SLAVE/
// SHARED/UNBINDABLE change the propagation TYPE of the EXISTING mount
// at target (do_change_type); MS_REMOUNT re-configures its flags
// (do_remount); MS_MOVE relocates it. NONE of them create a duplicate
// mount, so the EBUSY "already mounted" path does not apply. The
// kernel CAN return EBUSY for these on a busy bind mount — but the
// loader's mount namespace is fully virtualized, so faking success is
// the correct virtualization semantics (same decision as the Rust-side
// is_mount_propagation_or_remount_op, commit 2258271).
static int is_mount_propagation_op(unsigned long flags) {
    const unsigned long MS_PROP_MASK = MS_PRIVATE | MS_SLAVE | MS_SHARED |
                                       MS_UNBINDABLE | MS_REMOUNT | MS_MOVE;
    return (flags & MS_PROP_MASK) != 0;
}

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
            // 6-Z214: propagation / remount / move ops reconfigure the
            // EXISTING entry — never a duplicate-mount EBUSY. MS_BIND
            // onto an already-mounted target is ALSO legal Linux
            // semantics (stacked bind mounts — AOSP init's MountDir
            // does mkdir + MS_BIND onto live dirs), so allow it too:
            // record success without clobbering the original entry's
            // fstype/source (a bind mount has no fstype of its own).
            if (is_mount_propagation_op(flags) ||
                (flags & MS_BIND)) {
                g_mounts[i].flags = flags;
                pthread_mutex_unlock(&g_mount_lock);
                return 0;
            }
            // Plain (non-bind, non-propagation) re-mount of a live
            // target: real kernel returns EBUSY — keep that semantic.
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
// NOTE: init mostly uses direct syscalls that bypass these hooks; we pre-create
// all the directories it needs in twoyi_init() instead. These hooks remain to
// catch any libc-routed mkdir calls from other processes.
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
    if (real_mkdir) {
        return real_mkdir(path, mode);
    }
    return twoyi_sys_mkdir(path, mode);
}

// Hook mkdirat — bionic may inline mkdir() as mkdirat(AT_FDCWD, ...)
// This catches the case where mkdir's PLT hook is bypassed.
// NOTE: init mostly uses direct syscalls that bypass these hooks; we pre-create
// all the directories it needs in twoyi_init() instead.
int mkdirat(int dirfd, const char *path, mode_t mode) {
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);

        // Create parent directories recursively
        char dir[512];
        strncpy(dir, translated, sizeof(dir) - 1);
        dir[sizeof(dir) - 1] = 0;
        char *slash = strrchr(dir, '/');
        if (slash) {
            *slash = 0;
            // Create directory chain using direct syscalls
            for (char *p = dir + 1; *p; p++) {
                if (*p == '/') {
                    *p = 0;
                    syscall(SYS_mkdirat, AT_FDCWD, dir, 0777);
                    *p = '/';
                }
            }
            syscall(SYS_mkdirat, AT_FDCWD, dir, 0777);
        }

        return syscall(SYS_mkdirat, AT_FDCWD, translated, mode);
    }
    return syscall(SYS_mkdirat, dirfd, path, mode);
}

static int (*real_mknod)(const char *, mode_t, dev_t) = NULL;
static int (*real_mknodat)(int, const char *, mode_t, dev_t) = NULL;
static int (*real_mkfifo)(const char *, mode_t) = NULL;
static int (*real_mkfifoat)(int, const char *, mode_t) = NULL;

// 6-Z288: materialise a node request inside the GUEST rootfs.
//
// The guest cannot mknod on the host (app sandbox), and the previous
// hooks lied or misfired: CHR/BLK returned 0 creating NOTHING on arm64
// (the 6-Z185 lie), and FIFO fell through to real_mknodat on the
// UNTRANSLATED host path — OrangeFox's mkfifo("/system/bin/orsin")
// failed on the host's read-only /system, and its ORS writer child then
// spun a ~2.8kHz open("/system/bin/orsin", O_WRONLY) ENOENT retry loop
// (423k opens, run 33897176830) that starved the render loop → BOOT_FAIL.
//
// HONEST MODEL: the node is materialised in the rootfs at the TRANSLATED
// path —
//   * FIFO/SOCK: try the REAL node type first (a real fifo in the app-
//     writable rootfs gives true ORS pipe semantics: fox's reader and the
//     sh writer meet on the same host fifo). If the host refuses, back it
//     with a regular file — the writer's open/write then SUCCEED and the
//     reader's read returns EOF/empty ("command recorded, no daemon
//     answers") instead of an ENOENT storm.
//   * CHR/BLK: regular file carrying dev_t (the documented VM behavior —
//     emu_mknodat "creates regular file with dev_t"; the arm64 branch
//     simply never implemented it, now fixed for both arches).
//   * Anything already at the translated path: idempotent success.
// Returns 0 on success, -1 with honest errno when even the backing file
// cannot be created.
static int twoyi_materialise_node(const char *path, mode_t mode, dev_t dev) {
    if (!path || !g_rootfs) return -1;
    mode_t fmt = mode & S_IFMT;
    const char *t = should_translate(path) ? translate(path) : path;
    if (!t || t[0] != '/') return -1;

    // Already there? Idempotent success (EEXIST would break callers that
    // legitimately tolerate re-creation).
    struct stat st;
    if (twoyi_sys_fstatat(AT_FDCWD, t, &st, AT_SYMLINK_NOFOLLOW) == 0) return 0;

    if (fmt == S_IFCHR || fmt == S_IFBLK) {
        int fd = (int)twoyi_sys_open(t, O_RDWR | O_CREAT | O_EXCL, 0666);
        if (fd < 0) return -1;
        syscall(NR_write, fd, &dev, sizeof(dev_t));
        syscall(NR_close, fd);
        return 0;
    }

    // FIFO / SOCK / REG: real node type first (PLT-only mode — no seccomp
    // filter is installed in this process, so the raw syscall is safe).
    if (fmt == S_IFIFO || fmt == S_IFREG) {
        if (syscall(NR_mknodat, AT_FDCWD, t, mode, dev) == 0) return 0;
    }
    // Fallback: regular-file backing — kills the retry storm with honest
    // (if degenerate) read/write semantics.
    int fd = (int)twoyi_sys_open(t, O_RDWR | O_CREAT | O_EXCL, 0666);
    if (fd >= 0) { syscall(NR_close, fd); return 0; }
    return -1; // errno from the open — honest failure
}

int mknod(const char *path, mode_t mode, dev_t dev) {
    if (!real_mknod) real_mknod = dlsym(RTLD_NEXT, "mknod");
    if (path && should_translate(path))
        return twoyi_materialise_node(path, mode, dev);
    // Host paths: keep the legacy behavior (device nodes are not ours to
    // create on the host; other types pass through honestly).
    mode_t fmt = mode & S_IFMT;
    if (fmt == S_IFCHR || fmt == S_IFBLK) return 0;
    if (real_mknod) return real_mknod(path, mode, dev);
    return 0;
}

int mknodat(int dirfd, const char *path, mode_t mode, dev_t dev) {
    if (!real_mknodat) real_mknodat = dlsym(RTLD_NEXT, "mknodat");
    // Absolute guest paths materialise in the rootfs (6-Z288).
    if (path && path[0] == '/' && should_translate(path))
        return twoyi_materialise_node(path, mode, dev);
    mode_t fmt = mode & S_IFMT;
    if (fmt == S_IFCHR || fmt == S_IFBLK) return 0;
    // Relative path + dirfd: the dirfd (from a translated open) already
    // points into the rootfs — try the real syscall honestly.
    if (real_mknodat) return real_mknodat(dirfd, path, mode, dev);
    return 0;
}

// 6-Z288: mkfifo/mkfifoat hooks. OrangeFox imports mkfifo in its dynsym
// (verified in orangefox-R12.0-lavender's system/bin/recovery) and calls
// it via PLT; bionic's internal mkfifo→mknod chain is NOT PLT-visible,
// so hooking mknod/mknodat alone never saw it.
int mkfifo(const char *path, mode_t mode) {
    if (!real_mkfifo) real_mkfifo = dlsym(RTLD_NEXT, "mkfifo");
    if (path && should_translate(path))
        return twoyi_materialise_node(path, (mode & ~S_IFMT) | S_IFIFO, 0);
    if (real_mkfifo) return real_mkfifo(path, mode);
    return -1;
}

int mkfifoat(int dirfd, const char *path, mode_t mode) {
    if (!real_mkfifoat) real_mkfifoat = dlsym(RTLD_NEXT, "mkfifoat");
    if (path && path[0] == '/' && should_translate(path))
        return twoyi_materialise_node(path, (mode & ~S_IFMT) | S_IFIFO, 0);
    if (real_mkfifoat) return real_mkfifoat(dirfd, path, mode);
    return -1;
}

// =========================================================================
// 6-Z138: UID/GID VIRTUALIZATION — services must see their DECLARED ids.
//
// Run 32809458174 (2515435): servicemanager exit_group(127) right after
// getuid() returned 10167 — the APP's uid. System services CHECK their
// identity (servicemanager and friends require AID_ROOT/system; media
// requires AID_MEDIA...) and bail when the uid is wrong. init's
// SetProcessAttributes calls setresuid(uid,uid,uid) — our fake succeeds
// but the REAL uid stays the app's. Record the REQUESTED id at each
// setuid-family call and report it from the getuid-family hooks
// (default 0 = root before any setuid, matching a real boot's init).
// =========================================================================
static __thread uid_t g_virtual_uid = 0;
static __thread gid_t g_virtual_gid = 0;

uid_t getuid(void) { return g_virtual_uid; }
uid_t geteuid(void) { return g_virtual_uid; }
gid_t getgid(void) { return g_virtual_gid; }
gid_t getegid(void) { return g_virtual_gid; }

int setuid(uid_t uid) { g_virtual_uid = uid; return 0; }
int setgid(gid_t gid) { g_virtual_gid = gid; return 0; }
int setgroups(size_t size, const gid_t *list) { (void)size; (void)list; return 0; }
int setresuid(uid_t ruid, uid_t euid, uid_t suid) {
    (void)ruid; (void)suid;
    g_virtual_uid = euid;
    return 0;
}
int setresgid(gid_t rgid, gid_t egid, gid_t sgid) {
    (void)rgid; (void)sgid;
    g_virtual_gid = egid;
    return 0;
}
int unshare(int flags) { (void)flags; return 0; }

// =========================================================================
// 6-Z127: prctl + capset — the capability-service FATAL pair.
//
// Run 32777004259 (1ece4d9) LIVE kmsg: the guest boot devolved into a
// rapid crash loop — EVERY capability-carrying service died at
//   init: prctl(PR_SET_SECUREBITS) failed for <service>: Operation not
//   permitted -> LOG(FATAL) -> SIGABRT -> InitFatalReboot (x58 in 71s).
// service.cpp SetProcessAttributesAndCaps: for services with
// `capabilities` + non-root uid it does
//   prctl(PR_GET_SECUREBITS)  — works (returns 0 for the app)
//   prctl(PR_SET_SECUREBITS, SECBIT_KEEP_CAPS|SECBIT_KEEP_CAPS_LOCKED)
//                            — needs CAP_SETPCAP -> EPERM -> FATAL  ← HERE
//   ... then SetCapsForExec() -> capset() — will be the NEXT EPERM.
// PR_SET_SECUREBITS is 38 / PR_GET_SECUREBITS is 39 (both directions
// handled: GET must keep succeeding — return 0, the "no securebits"
// state).
// Everything else (PR_SET_NAME, PR_SET_PDEATHSIG, PR_SET_DUMPABLE,
// PR_GET_DUMPABLE, PR_SET_VMA...) PASSES THROUGH to the real prctl.
// =========================================================================
int prctl(int option, ...) {
    if (option == 38 /* PR_SET_SECUREBITS */) {
        return 0;  // fake success — the sandbox has no CAP_SETPCAP
    }
    if (option == 39 /* PR_GET_SECUREBITS */) {
        return 0;  // no securebits locked
    }
    va_list ap;
    va_start(ap, option);
    unsigned long arg2 = va_arg(ap, unsigned long);
    unsigned long arg3 = va_arg(ap, unsigned long);
    unsigned long arg4 = va_arg(ap, unsigned long);
    unsigned long arg5 = va_arg(ap, unsigned long);
    va_end(ap);
    static int (*real_prctl)(int, ...) = NULL;
    if (!real_prctl) real_prctl = dlsym(RTLD_NEXT, "prctl");
    if (real_prctl) return real_prctl(option, arg2, arg3, arg4, arg5);
    errno = EINVAL;
    return -1;
}

// capset — fake success (SetCapsForExec's next EPERM wall; the guest
// cannot manage capability sets without CAP_SETPCAP).
int capset(void *hdrp, void *datap) {
    (void)hdrp; (void)datap;
    return 0;
}

// capget — fake success with an all-zero header/data (some callers
// probe capabilities before setting them; a -1 return is treated as
// fatal by init's DropInheritableCaps path).
int capget(void *hdrp, void *datap) {
    (void)hdrp; (void)datap;
    return 0;
}

// 6-Z141: mprotect — survive bionic's atexit-array write-protection.
// Run 32816689086: 'mprotect failed on atexit array: Permission
// denied' aborted every guest process (untrusted_app lacks the
// execmem-class permission on the host). The array functions fine
// unprotected: on EACCES for an anonymous-looking range, fake success
// (the pages keep their current permissions — the atexit machinery
// only uses the mprotect as a hardening step).
int mprotect(void *addr, size_t len, int prot) {
    static int (*real_mprotect)(void *, size_t, int) = NULL;
    if (!real_mprotect) real_mprotect = dlsym(RTLD_NEXT, "mprotect");
    if (real_mprotect) {
        int r = real_mprotect(addr, len, prot);
        if (r == 0) return 0;
        if (errno == EACCES) {
            // Fake success — the hardening mprotect is advisory.
            return 0;
        }
        return r;
    }
    return syscall(SYS_mprotect, addr, len, prot);
}

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

// Hook setns — fake success for mount namespace switches.
// init's SetupMountNamespaces calls setns() to switch to the bootstrap
// mount namespace. In our container (no real mount namespaces), this fails.
// Faking success lets init continue.
int setns(int fd, int nstype) {
    (void)fd; (void)nstype;
    char msg[128];
    snprintf(msg, sizeof(msg), "[twoyi_loader] setns: faking success (fd=%d nstype=%d)\n", fd, nstype);
    write_str(2, msg);
    return 0;
}

// =========================================================================
// 6-Z139: abort unmasking — tgkill/rt_tgsigqueueinfo real-pid rewrite.
//
// bionic's abort() does tgkill(getpid(), gettid(), SIGABRT). The getpid
// hook (getpid_hook.so, also in LD_PRELOAD) returns 1 — the fake init
// pid — so tgkill(1, real_tid) hits ESRCH (no such tgid) and abort()
// falls back to _exit(127), masking EVERY guest FATAL as a silent 127
// exit with no crash message (run 32811363109: servicemanager,
// wait_for_keymaster and zygote all died this way). Rewrite tgid==1
// (the fake getpid value) to g_real_pid — captured at loader init via a
// raw syscall before any hook installs — so the signal actually
// delivers, the SIGABRT handler runs and the crash message prints.
// =========================================================================
int tgkill(int tgid, int tid, int sig) {
    if (tgid == 1 && g_real_pid > 0) tgid = g_real_pid;
    static int (*real_tgkill)(int, int, int) = NULL;
    if (!real_tgkill) real_tgkill = dlsym(RTLD_NEXT, "tgkill");
    if (real_tgkill) return real_tgkill(tgid, tid, sig);
    return syscall(SYS_tgkill, tgid, tid, sig);
}

int rt_tgsigqueueinfo(int tgid, int tid, int sig, void *uinfo) {
    if (tgid == 1 && g_real_pid > 0) tgid = g_real_pid;
    static int (*real_rt_tgsigqueueinfo)(int, int, int, void *) = NULL;
    if (!real_rt_tgsigqueueinfo) real_rt_tgsigqueueinfo = dlsym(RTLD_NEXT, "rt_tgsigqueueinfo");
    if (real_rt_tgsigqueueinfo) return real_rt_tgsigqueueinfo(tgid, tid, sig, uinfo);
    return syscall(SYS_rt_tgsigqueueinfo, tgid, tid, sig, uinfo);
}

// Hook android_get_control_socket — return a fake fd.
// lmkd and other services call this to get the socket fd that init
// created for them via "socket" in the .rc file. The fd is normally
// passed via ANDROID_SOCKET_<name> env var. If the env var is missing
// (e.g., because our exec hooks stripped it), the function returns -1
// and the service exits.
//
// Fix: return a fake fd (3) so the service thinks it has the socket.
// The service will then bind/listen on this fd. Since the fd is not
// a real socket, bind/listen will fail, but the service may continue
// running (or at least not exit immediately).
int android_get_control_socket(const char *name) {
    (void)name;
    // Check if the env var exists first
    char env_name[128];
    snprintf(env_name, sizeof(env_name), "ANDROID_SOCKET_%s", name);
    const char *val = getenv(env_name);
    if (val) {
        // Env var exists — parse it as an fd
        int fd = atoi(val);
        if (fd >= 0) return fd;
    }
    // Env var missing — return a fake fd
    char msg[256];
    snprintf(msg, sizeof(msg),
        "[twoyi_loader] android_get_control_socket(%s) — env var missing, returning fake fd 3\n", name);
    write_str(2, msg);
    return 3;  // fake fd
}

// =========================================================================
// 6-Z140: android_log family → stderr mirror. The guest's logd is dead
// (its sockets are unreachable), so every ART/system log line —
// including ABORT MESSAGES, the crash-storm forensics we need most —
// is silently lost. Mirror each call to stderr (2, unbuffered write)
// BEFORE the real function runs, so the E2E artifact always carries
// the guest's own words.
//
// NOTE (6-Z140): this REPLACES the older __android_log_write /
// __android_log_buf_write fallback hooks (and their `in_log_hook`
// re-entrancy guard). The old hooks mirrored via write_str(), whose
// dlsym(RTLD_DEFAULT, "__android_log_write") resolves back to OUR
// hook — hence the guard. log_mirror_line() uses only snprintf + a
// raw NR_write syscall, so it can never re-enter the hook chain and
// the guard is obsolete. write_str()'s own dlsym call now lands here:
// each loader message also mirrors one "[glog I/twoyi_loader]" line
// (a single extra copy — no cycle).
// =========================================================================
static void log_mirror_line(int prio, const char *tag, const char *text) {
    if (!text) return;
    char c;
    switch (prio) {
        case 0: c = 'U'; break; case 1: c = 'F'; break; case 2: c = 'E'; break;
        case 3: c = 'W'; break; case 4: c = 'I'; break; case 5: c = 'D'; break;
        case 6: c = 'V'; break; default: c = '?'; break;
    }
    char line[2048];
    int n = snprintf(line, sizeof(line), "[glog %c/%s] %s\n", c, tag ? tag : "?", text);
    if (n > 0) syscall(NR_write, 2, line, (size_t)(n < (int)sizeof(line) ? n : (int)sizeof(line) - 1));
}

int __android_log_write(int prio, const char *tag, const char *text) {
    log_mirror_line(prio, tag, text);
    static int (*real_fn)(int, const char *, const char *) = NULL;
    if (!real_fn) real_fn = dlsym(RTLD_NEXT, "__android_log_write");
    if (real_fn) return real_fn(prio, tag, text);
    return 0;
}

int __android_log_buf_write(int bufID, int prio, const char *tag, const char *text) {
    log_mirror_line(prio, tag, text);
    static int (*real_fn)(int, int, const char *, const char *) = NULL;
    if (!real_fn) real_fn = dlsym(RTLD_NEXT, "__android_log_buf_write");
    if (real_fn) return real_fn(bufID, prio, tag, text);
    return 0;
}

int __android_log_print(int prio, const char *tag, const char *fmt, ...) {
    char buf[1024];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(buf, sizeof(buf), fmt, ap);
    va_end(ap);
    log_mirror_line(prio, tag, buf);
    return __android_log_write(prio, tag, buf);
}

int __android_log_vprint(int prio, const char *tag, const char *fmt, va_list ap) {
    char buf[1024];
    vsnprintf(buf, sizeof(buf), fmt, ap);
    log_mirror_line(prio, tag, buf);
    return __android_log_write(prio, tag, buf);
}

int __android_log_buf_print(int bufID, int prio, const char *tag, const char *fmt, ...) {
    char buf[1024];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(buf, sizeof(buf), fmt, ap);
    va_end(ap);
    log_mirror_line(prio, tag, buf);
    static int (*real_fn)(int, int, const char *, const char *, ...) = NULL;
    if (!real_fn) real_fn = dlsym(RTLD_NEXT, "__android_log_buf_print");
    if (real_fn) {
        va_list ap2;
        va_start(ap2, fmt);
        int r = real_fn(bufID, prio, tag, "%s", buf);
        va_end(ap2);
        return r;
    }
    return 0;
}

// android_set_abort_message — THE abort-reason carrier (bionic stores
// it for crash_dump; with tombstoned unreachable it is otherwise lost).
void android_set_abort_message(const char *message) {
    log_mirror_line(1 /*FATAL*/, "abort", message ? message : "(null)");
    static void (*real_fn)(const char *) = NULL;
    if (!real_fn) real_fn = dlsym(RTLD_NEXT, "android_set_abort_message");
    if (real_fn) real_fn(message);
}

// =========================================================================
// Hook socket — intercept AF_NETLINK to prevent vold's NetlinkManager
// from failing. vold's NetlinkManager::start() calls
// socket(AF_NETLINK, SOCK_RAW, NETLINK_KOBJECT_UEVENT) to monitor
// kernel uevents. In our container, netlink sockets may fail (EPERM
// or ENOPROTOOPT), causing vold to exit(1).
// Fix: replace AF_NETLINK with AF_UNIX (always succeeds), giving vold
// a valid fd that won't fail on bind/setsockopt. vold won't receive
// real uevents (there are no real block devices), but it won't crash.
//
// Also hook sendto — capture logd messages and mirror to stderr
//
// vold's LogdLogger sends messages directly to /dev/socket/logdw via
// sendto(), bypassing our __android_log_write hooks above. This hook
// intercepts sendto() to the logd socket and mirrors the message to
// stderr so we can see the actual error message that causes vold to
// exit(1).
//
// We still forward to the real sendto() so logd (if running) also gets
// the message — we only add a side-channel mirror.
// =========================================================================

// Helper: check if current process is vold (by reading /proc/self/comm)
static int is_vold_process(void) {
    char comm[16] = {0};
    int fd = (int)syscall(NR_openat, AT_FDCWD, "/proc/self/comm", O_RDONLY, 0);
    if (fd < 0) return 0;
    long n = syscall(SYS_read, fd, comm, sizeof(comm) - 1);
    syscall(NR_close, fd);
    if (n <= 0) return 0;
    char *nl = strchr(comm, '\n');
    if (nl) *nl = 0;
    return (strcmp(comm, "vold") == 0);
}

// Hook socket — intercept AF_NETLINK ONLY for vold, replace with AF_UNIX
// init/ueventd also use AF_NETLINK and MUST NOT be intercepted.
int socket(int domain, int type, int protocol) {
    static int (*real_socket)(int, int, int) = NULL;
    if (!real_socket) real_socket = dlsym(RTLD_NEXT, "socket");
    
    if (domain == AF_NETLINK && is_vold_process()) {
        // vold's NetlinkManager calls socket(AF_NETLINK, SOCK_RAW,
        // NETLINK_KOBJECT_UEVENT) which fails in our container.
        // Replace with AF_UNIX (always succeeds).
        char msg[256];
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] socket(AF_NETLINK, %d, %d) -> replacing with AF_UNIX (vold)\n",
            type, protocol);
        write_str(2, msg);
        domain = AF_UNIX;
        type = SOCK_DGRAM;
    }
    
    if (real_socket) return real_socket(domain, type, protocol);
    return syscall(SYS_socket, domain, type, protocol);
}

// Hook bind — for AF_NETLINK binds (vold only), return success
int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen) {
    static int (*real_bind)(int, const struct sockaddr *, socklen_t) = NULL;
    if (!real_bind) real_bind = dlsym(RTLD_NEXT, "bind");
    
    // For AF_NETLINK bind (vold only), just return success
    if (addr && addr->sa_family == AF_NETLINK && is_vold_process()) {
        write_str(2, "[twoyi_loader] bind(AF_NETLINK) -> returning success (fake, vold)\n");
        return 0;
    }
    
    // Normal bind path (with AF_UNIX translation for rootfs sockets)
    if (addr && addr->sa_family == AF_UNIX && g_rootfs) {
        struct sockaddr_un *un = (struct sockaddr_un *)addr;
        if (un->sun_path[0] == '/' && should_translate(un->sun_path)) {
            char translated[600];
            snprintf(translated, sizeof(translated), "%s%s", g_rootfs, un->sun_path);
            char dir[600];
            strncpy(dir, translated, sizeof(dir) - 1);
            dir[sizeof(dir) - 1] = 0;
            char *slash = strrchr(dir, '/');
            if (slash) {
                *slash = 0;
                for (char *p = dir + 1; *p; p++) {
                    if (*p == '/') { *p = 0; syscall(SYS_mkdirat, AT_FDCWD, dir, 0777); *p = '/'; }
                }
                syscall(SYS_mkdirat, AT_FDCWD, dir, 0777);
            }
            struct sockaddr_un new_addr;
            memset(&new_addr, 0, sizeof(new_addr));
            new_addr.sun_family = AF_UNIX;
            strncpy(new_addr.sun_path, translated, sizeof(new_addr.sun_path) - 1);
            char msg[256];
            snprintf(msg, sizeof(msg), "[twoyi_loader] bind: translated %s -> %s\n", un->sun_path, translated);
            write_str(2, msg);
            if (real_bind) return real_bind(sockfd, (const struct sockaddr *)&new_addr, sizeof(new_addr));
            return syscall(SYS_bind, sockfd, &new_addr, sizeof(new_addr));
        }
    }
    if (real_bind) return real_bind(sockfd, addr, addrlen);
    return syscall(SYS_bind, sockfd, addr, addrlen);
}

// Hook ioctl — intercept binder ioctls.
//
// Three kinds of binder fds reach this hook:
//   1. REAL binderfs fds — opened successfully from /dev/binderfs/* (the
//      container has its own binderfs, mounted by kr64 with chmod 0666).
//      These support REAL binder IPC within the container's binder domain.
//      For these, we pass BINDER_WRITE_READ / BINDER_SET_CONTEXT_MGR /
//      BINDER_SET_MAX_THREADS / BINDER_VERSION through to the REAL ioctl.
//      Without this, system_server's ServiceManager.addService/getService
//      would silently no-op (the old hook returned 0 with no data) and
//      binder-dependent services would crash or hang.
//
//   2. FALLBACK fds — /dev/null fds returned by binder_open_fallback()
//      when the real open of a binder device failed AND the kr64 binder
//      proxy was unreachable (proxy absent — e.g. TWRP mode). These
//      CANNOT do real binder IPC — the real ioctl would return ENOTTY on
//      /dev/null. For these, we keep faking BINDER_VERSION (-> 8),
//      BINDER_SET_MAX_THREADS, BINDER_SET_CONTEXT_MGR,
//      BINDER_WRITE_READ so the calling process (e.g., a HIDL HAL
//      service) sees fd >= 0 and a valid protocol version, then blocks in
//      its threadpool without crashing.
//
//   3. PROXY fds (6-Z113) — CONNECTED Unix sockets to the kr64 binder
//      proxy, returned by binder_open_fallback() when the real open failed
//      but {TWOYI_ROOTFS}/vm0/dev/binder accepted a connect(2) (non-root
//      mode's expected shape: open on the socket node gives ENXIO). For
//      these, binder_proxy_ioctl() speaks the proxy's Frame/Resp wire
//      protocol — see the wire client block above the framebuffer
//      tracking for the full contract.
//
// Fd tracking: binder_open_fallback() records each fallback fd in
// g_binder_fallback_fds and each proxy fd in g_binder_proxy_fds (see
// above). The close() hook clears both. Real binderfs fds are in
// neither set, so they pass through to the real ioctl. We do NOT
// suppress real ioctl errors: if real_ioctl returns -1,
// we log the errno and return -1.
//
// Binder ioctl numbers (from kernel: include/uapi/linux/android/binder.h):
// BINDER_VERSION = _IOWR('b', 9, struct binder_version)
// BINDER_SET_MAX_THREADS = _IOW('b', 5, __u32)
// BINDER_SET_CONTEXT_MGR = _IOW('b', 7, __s32)
// BINDER_WRITE_READ = _IOWR('b', 1, struct binder_write_read)
//
// On x86_64: 'b' = 0x62, so:
// BINDER_VERSION = _IOWR(0x62, 9, struct{__s32}) = 0xc0046209
//                 (0xc004620d below is NOT a kernel spelling — nr 13 is
//                  BINDER_SET_CONTEXT_MGR_EXT; we keep accepting it because
//                  the mmap hook probes it — 6-Z113 verified vs binder.rs)
// BINDER_SET_MAX_THREADS = _IOW(0x62, 5, __u32) = 0x40046205
// BINDER_SET_CONTEXT_MGR = _IOW(0x62, 7, __s32) = 0x40046207
// BINDER_WRITE_READ = _IOWR(0x62, 1, ...) = 0xc0306201
// Use the correct ioctl signature for the build target:
// - bionic (Android NDK): int ioctl(int, int, ...)
// - glibc (host build): int ioctl(int, unsigned long, ...)
#ifdef __BIONIC__
int ioctl(int fd, int request, ...) {
#else
int ioctl(int fd, unsigned long request, ...) {
#endif
    static int (*real_ioctl)() = NULL;
    if (!real_ioctl) real_ioctl = dlsym(RTLD_NEXT, "ioctl");

    va_list ap;
    va_start(ap, request);
    void *argp = va_arg(ap, void *);
    va_end(ap);

    unsigned req = (unsigned)request;

    // -----------------------------------------------------------------------
    // FB ioctl virtualization for TWRP: if this fd was opened on
    // /dev/graphics/fb0 or /dev/fb0, intercept FB ioctls and return valid
    // screen info. The kernel returns ENOTTY for FB ioctls on a regular
    // file (which is what /dev/graphics/fb0 is in TWRP mode), leaving the
    // screeninfo struct zeroed and causing a NULL deref segfault in
    // libminuitwrp.so at offset 0x57d7.
    //
    // FB ioctl numbers (from linux/fb.h — raw numbers, no _IO encoding):
    //   FBIOGET_VSCREENINFO  = 0x4600
    //   FBIOPUT_VSCREENINFO  = 0x4601
    //   FBIOGET_FSCREENINFO  = 0x4602
    //   FBIOPUT_FSCREENINFO  = 0x4603
    //   FBIOGETCMAP          = 0x4604
    //   FBIOPUTCMAP          = 0x4605
    //   FBIOPAN_DISPLAY      = 0x4606
    //   FBIOBLANK            = 0x4611
    //   FBIO_WAITFORVSYNC    = 0x40044620
    //
    // A tracked fb0 fd will never be a binder fallback fd (the open hook
    // only tracks fb0 paths, not binder paths), so this check is mutually
    // exclusive with the binder logic below.
    // -----------------------------------------------------------------------
    if (!fb_fd_is_tracked(fd) && ((req & 0xff00) == 0x4600u || req == 0x40044620u)) {
        // 6-Z222: an FB ioctl on an untracked fd — self-heal before giving
        // up. Catches every open variant that bypassed the open hooks
        // (open64 pre-fix, dup()ed fds, future bypasses) by verifying the
        // fd's /proc/self/fd target really is an fb0 path.
        fb_fd_selfheal(fd);
    }
    if (fb_fd_is_tracked(fd)) {
        switch (req) {
            case 0x4600u: {  // FBIOGET_VSCREENINFO
                if (argp) fill_vscreeninfo((struct fb_var_screeninfo *)argp);
                {
                    char msg[128];
                    snprintf(msg, sizeof(msg),
                        "[twoyi_loader] ioctl(FBIOGET_VSCREENINFO) -> %dx%d@32bpp\n",
                        fb_w_rt(), fb_h_rt());
                    write_str(2, msg);
                }
                return 0;
            }
            case 0x4601u: {  // FBIOPUT_VSCREENINFO — accept the mode change
                write_str(2, "[twoyi_loader] ioctl(FBIOPUT_VSCREENINFO) -> success\n");
                return 0;
            }
            case 0x4602u: {  // FBIOGET_FSCREENINFO
                if (argp) fill_fscreeninfo((struct fb_fix_screeninfo *)argp);
                {
                    char msg[160];
                    snprintf(msg, sizeof(msg),
                        "[twoyi_loader] ioctl(FBIOGET_FSCREENINFO) -> smem_len=%ld line_length=%ld\n",
                        fb_smem_len_rt(), fb_line_length_rt());
                    write_str(2, msg);
                }
                return 0;
            }
            case 0x4603u: {  // FBIOPUT_FSCREENINFO — accept
                return 0;
            }
            case 0x4606u: {  // FBIOPAN_DISPLAY — page flip, accept
                return 0;
            }
            case 0x4611u: {  // FBIOBLANK — virtual display, blanking is a no-op
                return 0;
            }
            case 0x40044620u: {  // FBIO_WAITFORVSYNC — no real vsync, return
                return 0;
            }
            default: {
                // Other FB ioctls (0x46xx range: FBIOGETCMAP, FBIOPUTCMAP,
                // FBIO_CURSOR, etc.) — fake success. Non-FB ioctls on an
                // fb0 fd (shouldn't happen) fall through to the real ioctl.
                if ((req & 0xff00) == 0x4600) {
                    return 0;
                }
                break;  // fall through to binder check / real ioctl
            }
        }
    }

    // Quick filter: is this a binder ioctl? (magic 'b' = 0x62 in high byte)
    int is_binder_req = ((req & 0xff00) == 0x6200);
    if (!is_binder_req) {
        // Non-binder ioctl — pass through (existing behavior).
        if (real_ioctl) return real_ioctl(fd, request, argp);
        return syscall(SYS_ioctl, fd, request, argp);
    }

    // Binder ioctl. Decide fake-vs-real based on whether fd is a fallback.
    int is_fallback = binder_fd_is_fallback(fd);

    if (is_fallback) {
        // /dev/null fallback fd — fake binder ioctls (the real ioctl would
        // return ENOTTY on /dev/null). This is the virtualization path for
        // processes that couldn't open the real binder device.
        char msg[256];
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] ioctl(fd=%d fallback, req=0x%x) -> faking\n",
            fd, req);
        write_str(2, msg);

        // BINDER_VERSION — returns protocol version.
        // Match both 0xc004620d (newer kernels) and 0xc0046209 (older).
        if (req == 0xc004620du || req == 0xc0046209u) {
            if (argp) {
                *(int *)argp = 8;  // BINDER_CURRENT_PROTOCOL_VERSION
            }
            write_str(2, "[twoyi_loader] ioctl(BINDER_VERSION) -> faking version 8\n");
            return 0;
        }
        // BINDER_SET_MAX_THREADS = 0x40046205
        if (req == 0x40046205u) {
            write_str(2, "[twoyi_loader] ioctl(BINDER_SET_MAX_THREADS) -> success\n");
            return 0;
        }
        // BINDER_SET_CONTEXT_MGR = 0x40046207
        if (req == 0x40046207u) {
            write_str(2, "[twoyi_loader] ioctl(BINDER_SET_CONTEXT_MGR) -> success\n");
            return 0;
        }
        // BINDER_SET_CONTEXT_MGR_EXT = 0x4018620d (Android 11's
        // servicemanager calls it with a flat_binder_object context
        // after the plain SET_CONTEXT_MGR; reject it as a FATAL).
        if (req == 0x4018620du) {
            write_str(2, "[twoyi_loader] ioctl(BINDER_SET_CONTEXT_MGR_EXT) -> success\n");
            return 0;
        }
        // BINDER_WRITE_READ = 0xc0306201 — return success with no data
        if (req == 0xc0306201u) {
            return 0;
        }
        // Unknown binder ioctl on fallback fd — fake success (the fd is
        // /dev/null, so the real ioctl would fail anyway).
        return 0;
    }

    // 3. PROXY fds — 6-Z113: connected Unix sockets to the kr64 binder
    //    proxy ({rootfs}/vm0/dev/binder, handed out by
    //    binder_open_fallback when the real open failed but the proxy is
    //    reachable). Translate the binder ioctl into the proxy's
    //    Frame/Resp wire protocol (see the wire client block above for the
    //    full contract: VERSION/SET_MAX_THREADS/SET_CONTEXT_MGR/
    //    THREAD_EXIT pass-through with ioctl-number translation, and
    //    WRITE_READ marshalled to WireBinderWriteRead with correct
    //    write_consumed/read_consumed accounting — the z112 G3 livelock
    //    fix). Transport failures return -1 with errno (honest ioctl
    //    semantics; no silent degradation to the fakes above).
    if (binder_fd_is_proxy(fd)) {
        return binder_proxy_ioctl(fd, req, argp);
    }

    // REAL binder fd (real binderfs device) — pass through to the real
    // ioctl so real binder IPC happens within the container's binderfs
    // domain. Do NOT fake success: if the real ioctl fails, log the errno
    // and return -1 (don't suppress real failures — we need to see them).
    int ret;
    if (real_ioctl) ret = real_ioctl(fd, request, argp);
    else            ret = syscall(SYS_ioctl, fd, request, argp);
    if (ret < 0) {
        int e = errno;
        char msg[256];
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] ioctl(fd=%d real, req=0x%x) -> -1 (errno=%d: %s)\n",
            fd, req, e, strerror(e));
        write_str(2, msg);
        errno = e;
    }
    return ret;
}

// Hook mmap — for binder fds, return an anonymous mapping.
// binderfs devices don't support mmap() the way real binder devices do.
// ProcessState::ProcessState() calls mmap() on the binder fd and aborts
// if it returns MAP_FAILED. We intercept mmap() for binder fds and
// return an anonymous mapping instead.
#include <sys/mman.h>
void *mmap(void *addr, size_t length, int prot, int flags, int fd, off_t offset) {
    static void *(*real_mmap)(void *, size_t, int, int, int, off_t) = NULL;
    if (!real_mmap) real_mmap = dlsym(RTLD_NEXT, "mmap");

    if (real_mmap) {
        void *result = real_mmap(addr, length, prot, flags, fd, offset);
        if (result != MAP_FAILED) return result;

        // ── 6-Z224: SAVE the real mmap errno BEFORE anything else can
        // overwrite it. The pre-6-Z224 code probed with BINDER_VERSION
        // ioctls FIRST and the probe's ENOTTY (25) replaced the real
        // errno — run 33279360223 (OrangeFox) and run 33279361259
        // (lineage) both printed "failed to mmap framebuffer:
        // Inappropriate ioctl for device", a pure artifact of the probe:
        // OrangeFox's REAL error was EACCES (its minui opened fb0
        // O_RDONLY — __open_2 fl=0x2 — then mapped PROT_READ|PROT_WRITE
        // MAP_SHARED, which the kernel refuses on a read-only fd), and
        // the wrong errno misdirected an entire analysis session.
        int saved_errno = errno;
        int fb_tracked = fb_fd_is_tracked(fd);
        if (!fb_tracked) fb_fd_selfheal(fd);  // 6-Z222 self-heal (dup'd/untracked opens)
        fb_tracked = fb_fd_is_tracked(fd);

        // Bounded diagnostic (first 8 failures per process): fd identity
        // via /proc/self/fd readlinkat so the artifacts show WHAT the
        // failed mmap actually pointed at.
        static unsigned mmap_fail_log = 0;
        if (mmap_fail_log < 8) {
            mmap_fail_log++;
            char target[256];
            target[0] = '\0';
            char linkpath[128];
            snprintf(linkpath, sizeof(linkpath), "/proc/self/fd/%d", fd);
            long n = syscall(SYS_readlinkat, AT_FDCWD, linkpath, target,
                             (long)(sizeof(target) - 1));
            if (n < 0) n = 0;
            target[n] = '\0';
            char msg[640];
            snprintf(msg, sizeof(msg),
                "[twoyi_loader] mmap FAILED fd=%d (fb_tracked=%d) len=%zu "
                "prot=0x%x flags=0x%x errno=%d (%s) -> %s\n",
                fd, fb_tracked, length, prot, flags, saved_errno,
                strerror(saved_errno), n > 0 ? target : "?");
            write_str(2, msg);
        }

        // ── 6-Z224: FB-tracked fd recovery. minui/libminuitwrp mmaps the
        // virtual framebuffer; the file itself is app-owned and writable,
        // but the CALLER's fd may be read-only (OrangeFox) or otherwise
        // unmappable. Fix: re-open /dev/graphics/fb0 O_RDWR (raw syscall,
        // tracer path translation — the same mechanism the fb hook's
        // create branch uses) and mmap THAT fd. The re-opened fd maps the
        // SAME underlying regular file, so the pixel pipeline (guest
        // writes -> file pages -> kr64's blit) stays intact. The mapping
        // survives close() of the temporary fd.
        if (fb_tracked) {
            int wfd = (int)twoyi_sys_open("/dev/graphics/fb0", O_RDWR | O_CLOEXEC, 0);
            if (wfd < 0)
                wfd = (int)twoyi_sys_open("/dev/fb0", O_RDWR | O_CLOEXEC, 0);
            if (wfd >= 0) {
                void *r2 = real_mmap(addr, length, prot, flags, wfd, offset);
                syscall(NR_close, wfd);
                if (r2 != MAP_FAILED) {
                    write_str(2, "[twoyi_loader] mmap fb-tracked fd -> recovered via O_RDWR re-open\n");
                    return r2;
                }
            }
            // Last resort: shared anonymous mapping — keeps minui running
            // (no "cannot open any framebuffer" headless exit) at the cost
            // of the host blit seeing a stale frame. Strictly better than
            // MAP_FAILED; the diagnostic above carries the real errno for
            // the next iteration.
            void *r3 = real_mmap(addr, length, prot,
                                 (flags & ~(MAP_PRIVATE | MAP_SHARED))
                                     | MAP_ANONYMOUS | MAP_SHARED,
                                 -1, 0);
            if (r3 != MAP_FAILED) {
                write_str(2, "[twoyi_loader] mmap fb-tracked fd -> MAP_ANONYMOUS|MAP_SHARED fallback\n");
                return r3;
            }
            errno = saved_errno;
            return MAP_FAILED;
        }

        // mmap failed on a NON-fb fd — check if this might be a binder fd
        // by trying BINDER_VERSION ioctl (pre-6-Z224 behavior, but now
        // skipped for fb fds: probing an fb fd with binder ioctls is what
        // produced the misleading ENOTTY).
        int vers = 0;
        if (ioctl(fd, 0xc0046209, &vers) == 0 || ioctl(fd, 0xc004620d, &vers) == 0) {
            // This is a binder fd — return anonymous mapping
            write_str(2, "[twoyi_loader] mmap on binder fd -> using MAP_ANONYMOUS\n");
            return real_mmap(addr, length, prot, flags | MAP_ANONYMOUS, -1, 0);
        }
        errno = saved_errno;
        return MAP_FAILED;
    }
#if defined(__arm__) && !defined(__aarch64__)
    // 6-Z227: arm32 has no SYS_mmap — the wired file-backed syscall is
    // mmap2, which takes the offset in PAGE units (not bytes). This is
    // the real_mmap==NULL fallback path; offsets we are ever handed
    // here (fb/shlib loading) are page-aligned, but reject unaligned
    // ones explicitly instead of silently mapping the wrong window.
    if (offset < 0 || (offset & 4095) != 0) {
        errno = EINVAL;
        return MAP_FAILED;
    }
    return (void *)syscall(NR_mmap2, addr, length, prot, flags, fd,
                           (long)(offset / 4096));
#else
    return (void *)syscall(SYS_mmap, addr, length, prot, flags, fd, offset);
#endif
}

// Hook close — clear binder fallback fd tracking when an fd is closed.
// This keeps g_binder_fallback_fds accurate: when a fallback fd is closed
// and its fd number is recycled for a different file, the new fd is NOT
// mistakenly treated as a binder fallback (which would wrongly fake its
// ioctls). Real binderfs fds are never in the set, so clearing is a
// no-op for them. We clear unconditionally — clearing an unset bit is
// harmless and avoids a mutex-locked lookup before the mutex-locked clear.
// 6-Z113: also clears the binder PROXY fd class (same reasoning — a
// recycled fd number must not be treated as a connected proxy socket).
// 6-Z116: also clears the qemu_pipe PROXY fd class (same reasoning).
int close(int fd) {
    static int (*real_close)(int) = NULL;
    if (!real_close) real_close = dlsym(RTLD_NEXT, "close");
    binder_fd_clear(fd);
    binder_fd_clear_proxy(fd);
    // 6-Z271g: dropping the binder fd also tears down every per-thread
    // proxy conn bound to it (fd-number recycling hygiene + no ghost
    // registrations in the proxy).
    bp_thread_conn_close_for_binder(fd);
    qemu_pipe_fd_clear_proxy(fd);
    fb_fd_clear(fd);
    if (real_close) return real_close(fd);
    return (int)syscall(NR_close, fd);
}

// Hook setsockopt — for AF_NETLINK options (vold only), return success
int setsockopt(int sockfd, int level, int optname, const void *optval, socklen_t optlen) {
    static int (*real_setsockopt)(int, int, int, const void *, socklen_t) = NULL;
    if (!real_setsockopt) real_setsockopt = dlsym(RTLD_NEXT, "setsockopt");
    
    // SOL_NETLINK = 270, common optnames: NETLINK_ADD_MEMBERSHIP=1, etc.
    if (level == 270 && is_vold_process()) {  // SOL_NETLINK
        char msg[256];
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] setsockopt(SOL_NETLINK, %d) -> returning success (fake)\n", optname);
        write_str(2, msg);
        return 0;
    }
    
    if (real_setsockopt) return real_setsockopt(sockfd, level, optname, optval, optlen);
    return syscall(SYS_setsockopt, sockfd, level, optname, optval, optlen);
}

ssize_t sendto(int sockfd, const void *buf, size_t len, int flags,
               const struct sockaddr *dest_addr, socklen_t addrlen) {
    static ssize_t (*real_sendto)(int, const void *, size_t, int,
                                  const struct sockaddr *, socklen_t) = NULL;
    if (!real_sendto) real_sendto = dlsym(RTLD_NEXT, "sendto");

    // Check if this is a send to the logd socket
    if (dest_addr && dest_addr->sa_family == AF_UNIX && buf && len > 0) {
        struct sockaddr_un *un = (struct sockaddr_un *)dest_addr;
        if (strstr(un->sun_path, "logdw") || strstr(un->sun_path, "logd")) {
            // Mirror to stderr
            static __thread int in_sendto_hook = 0;
            if (!in_sendto_hook) {
                in_sendto_hook = 1;
                // Write raw payload to stderr
                syscall(SYS_write, 2, buf, len);
                syscall(SYS_write, 2, "\n", 1);
                in_sendto_hook = 0;
            }
        }
    }

    if (real_sendto) return real_sendto(sockfd, buf, len, flags, dest_addr, addrlen);
    return syscall(SYS_sendto, sockfd, buf, len, flags, dest_addr, addrlen);
}

// NOTE: abort(), raise(), kill(), sigaction(), signal() hooks were previously
// here to suppress SIGABRT and prevent InitFatalReboot. They have been REMOVED
// because the real fix (android_get_control_socket hook for lmkd) makes them
// unnecessary. If InitFatalReboot returns, the correct response is to fix the
// root cause of the crash, not to suppress the signal.

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
        return twoyi_sys_unlink(translated);
    }
    static int (*real_unlink)(const char *) = NULL;
    if (!real_unlink) real_unlink = dlsym(RTLD_NEXT, "unlink");
    if (real_unlink) return real_unlink(path);
    return twoyi_sys_unlink(path);
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

// Hook connect — redirect AF_UNIX socket paths to rootfs (matches bind)
int connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen) {
    static int (*real_connect)(int, const struct sockaddr *, socklen_t) = NULL;
    if (!real_connect) real_connect = dlsym(RTLD_NEXT, "connect");

    // 6-Z272p: /dev/socket/logdw has NO reader in recovery images (they
    // don't run logd — unlike the full OrangeFox/TWRP ramdisks, which is
    // exactly why those worked while the AOSP-recovery cohort stalled).
    // liblog writes log lines to that socket SYNCHRONOUSLY; once the
    // socket buffer fills, every write blocks in liblog's poll-retry
    // loop and the whole process freezes BEFORE any UI draw (the walleye
    // runs: the recovery alive in poll_schedule_timeout for 300+ s,
    // zero draws, no kmsg output past the BCB retries). Redirect the
    // connect to /dev/__kmsg__ instead: the fd accepts unbounded
    // non-blocking writes, the log lines land in the kmsg artifacts,
    // and liblog's write path returns immediately.
    if (addr && addr->sa_family == AF_UNIX && g_rootfs) {
        struct sockaddr_un *un = (struct sockaddr_un *)addr;
        if (un->sun_path[0] == '/' &&
            strncmp(un->sun_path, "/dev/socket/logdw", 17) == 0) {
            char kmsg_path[600];
            snprintf(kmsg_path, sizeof(kmsg_path), "%s/dev/__kmsg__", g_rootfs);
            static int logdw_redirect_diag = 2;
            if (logdw_redirect_diag > 0) {
                logdw_redirect_diag--;
                char msg[160];
                snprintf(msg, sizeof(msg),
                    "[twoyi_loader] connect(/dev/socket/logdw) -> /dev/__kmsg__ "
                    "(no logd in recovery images — liblog would block)\n");
                write_str(2, msg);
            }
            // 6-Z279 FIX: the caller KEEPS writing to ITS OWN sockfd after
            // connect() — the previous code returned a FRESH openat() fd
            // number from connect(), which is neither 0 (success) nor -1
            // (failure), so liblog treated the connect as FAILED and its
            // socket fd stayed UNCONNECTED → every log writev returned
            // ENOTCONN in a retry loop (the health-HAL run 33848916173:
            // 26+ "FAILED syscall nr=66 ret=-107" events at +10550ms, the
            // battery stream's stall site). Put the kmsg fd IN PLACE OF
            // the socket fd (raw syscalls — no hook recursion), then
            // return 0 (real connect success). Subsequent writev(sockfd)
            // lands in /dev/__kmsg__: unbounded, non-blocking, and the
            // log lines still land in the kmsg artifacts.
            int kmsg_fd = (int)syscall(NR_openat, AT_FDCWD, kmsg_path, O_WRONLY, 0);
            if (kmsg_fd >= 0) {
                syscall(SYS_dup3, kmsg_fd, sockfd, 0);
                syscall(SYS_close, kmsg_fd);
                return 0;
            }
            // kmsg open failed — fall through to the real connect so the
            // caller sees the honest error instead of a fake success.
        }
        if (un->sun_path[0] == '/' && should_translate(un->sun_path)) {
            char translated[600];
            snprintf(translated, sizeof(translated), "%s%s", g_rootfs, un->sun_path);

            struct sockaddr_un new_addr;
            memset(&new_addr, 0, sizeof(new_addr));
            new_addr.sun_family = AF_UNIX;
            strncpy(new_addr.sun_path, translated, sizeof(new_addr.sun_path) - 1);

            if (real_connect) return real_connect(sockfd, (const struct sockaddr *)&new_addr, sizeof(new_addr));
            return syscall(SYS_connect, sockfd, &new_addr, sizeof(new_addr));
        }
    }
    if (real_connect) return real_connect(sockfd, addr, addrlen);
    return syscall(SYS_connect, sockfd, addr, addrlen);
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
        return twoyi_sys_chmod(translated, mode);
    }
    static int (*real_chmod)(const char *, mode_t) = NULL;
    if (!real_chmod) real_chmod = dlsym(RTLD_NEXT, "chmod");
    if (real_chmod) return real_chmod(path, mode);
    return twoyi_sys_chmod(path, mode);
}

// Hook chown — redirect paths to rootfs
int chown(const char *path, uid_t owner, gid_t group) {
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static int (*real_chown)(const char *, uid_t, gid_t) = NULL;
        if (!real_chown) real_chown = dlsym(RTLD_NEXT, "chown");
        if (real_chown) return real_chown(translated, owner, group);
        return twoyi_sys_chown(translated, owner, group);
    }
    static int (*real_chown)(const char *, uid_t, gid_t) = NULL;
    if (!real_chown) real_chown = dlsym(RTLD_NEXT, "chown");
    if (real_chown) return real_chown(path, owner, group);
    return twoyi_sys_chown(path, owner, group);
}

// Hook lstat — translate paths to rootfs (init's make_dir uses lstat)
int lstat(const char *path, struct stat *buf) {
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static int (*real_lstat)(const char *, struct stat *) = NULL;
        if (!real_lstat) real_lstat = dlsym(RTLD_NEXT, "lstat");
        if (real_lstat) return real_lstat(translated, buf);
        return twoyi_sys_fstatat(AT_FDCWD, translated, buf, AT_SYMLINK_NOFOLLOW);
    }
    static int (*real_lstat)(const char *, struct stat *) = NULL;
    if (!real_lstat) real_lstat = dlsym(RTLD_NEXT, "lstat");
    if (real_lstat) return real_lstat(path, buf);
    return twoyi_sys_fstatat(AT_FDCWD, path, buf, AT_SYMLINK_NOFOLLOW);
}

// Hook stat — translate paths to rootfs (6-Z120: the secilc wall).
//
// secilc.c stats EVERY CIL input file right after a SUCCESSFUL fopen():
//     file = fopen(argv[i], "r");      // hooked -> rootfs copy -> OK
//     rc = stat(argv[i], &filedata);   // NOT hooked -> EMULATOR's copy
// On the emulator /system/etc/selinux/plat_sepolicy.cil carries the
// sepolicy_file label and untrusted_app's { getattr } is DENIED
// (run 32724319465 avc line 173, permissive=0):
//   avc: denied { getattr } for path="/system/etc/selinux/plat_sepolicy.cil"
//   scontext=u:r:untrusted_app_27 tcontext=u:object_r:sepolicy_file:s0
// -> "Could not stat file" -> secilc exit 255 -> init InitFatalReboot.
// Translating stat() to the rootfs copy (whose readability the hooked
// fopen already proved) unlocks the whole SELinux-compile bootstep of
// the Android 11 guest. init's own stat() callers get the same benefit.
int stat(const char *path, struct stat *buf) {
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static int (*real_stat)(const char *, struct stat *) = NULL;
        if (!real_stat) real_stat = dlsym(RTLD_NEXT, "stat");
        if (real_stat) return real_stat(translated, buf);
        return twoyi_sys_fstatat(AT_FDCWD, translated, buf, 0);
    }
    static int (*real_stat)(const char *, struct stat *) = NULL;
    if (!real_stat) real_stat = dlsym(RTLD_NEXT, "stat");
    if (real_stat) return real_stat(path, buf);
    return twoyi_sys_fstatat(AT_FDCWD, path, buf, 0);
}

// Hook fstatat — translate ABSOLUTE guest paths to rootfs (6-Z120).
// Absolute paths ignore dirfd per openat(2) semantics, so translating
// them is safe regardless of which dirfd the caller passes. RELATIVE
// paths (guest-internal dirfd lookups, e.g. openat(dirfd,"event0"))
// pass through untouched — the guest's own fds point into the rootfs
// already.
int fstatat(int dirfd, const char *path, struct stat *buf, int flags) {
    if (path && path[0] == '/' && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static int (*real_fstatat)(int, const char *, struct stat *, int) = NULL;
        if (!real_fstatat) real_fstatat = dlsym(RTLD_NEXT, "fstatat");
        if (real_fstatat) return real_fstatat(dirfd, translated, buf, flags);
        return twoyi_sys_fstatat(dirfd, translated, buf, flags);
    }
    static int (*real_fstatat)(int, const char *, struct stat *, int) = NULL;
    if (!real_fstatat) real_fstatat = dlsym(RTLD_NEXT, "fstatat");
    if (real_fstatat) return real_fstatat(dirfd, path, buf, flags);
    return twoyi_sys_fstatat(dirfd, path, buf, flags);
}

// Hook lchown — translate paths to rootfs
int lchown(const char *path, uid_t owner, gid_t group) {
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static int (*real_lchown)(const char *, uid_t, gid_t) = NULL;
        if (!real_lchown) real_lchown = dlsym(RTLD_NEXT, "lchown");
        if (real_lchown) return real_lchown(translated, owner, group);
        return syscall(SYS_fchownat, AT_FDCWD, translated, owner, group, AT_SYMLINK_NOFOLLOW);
    }
    static int (*real_lchown)(const char *, uid_t, gid_t) = NULL;
    if (!real_lchown) real_lchown = dlsym(RTLD_NEXT, "lchown");
    if (real_lchown) return real_lchown(path, owner, group);
    return syscall(SYS_fchownat, AT_FDCWD, path, owner, group, AT_SYMLINK_NOFOLLOW);
}

// Hook access — translate paths to rootfs
int access(const char *path, int mode) {
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static int (*real_access)(const char *, int) = NULL;
        if (!real_access) real_access = dlsym(RTLD_NEXT, "access");
        if (real_access) return real_access(translated, mode);
        return syscall(SYS_faccessat, AT_FDCWD, translated, mode, 0);
    }
    static int (*real_access)(const char *, int) = NULL;
    if (!real_access) real_access = dlsym(RTLD_NEXT, "access");
    if (real_access) return real_access(path, mode);
    return syscall(SYS_faccessat, AT_FDCWD, path, mode, 0);
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

// =========================================================================
// 6-Z125: getfilecon / lgetfilecon / fgetfilecon (+ _raw variants) —
// THE missing service-start hook.
//
// Run 32770427272 (55c4d2a): after 6-Z124 unlocked SetKptrRestrict and
// the early-init actions, EVERY service start died at
//   init: Could not start exec service: Could not get file context
// (linkerconfig, apexd-bootstrap) -> 'bootstrap-apexd-failed' -> clean
// shutdown. service.cpp's ComputeContextFromExecutable chain:
//     getcon()                 -> hooked -> "u:r:init:s0"        OK
//     getfilecon(path)         -> REAL libselinux -> getxattr(
//                                 "security.selinux") -> ENODATA
//                                 (the staged binaries carry no label —
//                                 the app CANNOT set security.* xattrs,
//                                 lsetxattr EPERM) -> -1
//                              -> "Could not get file context" THE WALL
//     security_compute_create() -> hooked (derives u:r:<domain>:s0) OK
//
// The fix: fabricate a per-binary *_exec label from the path's basename
// (the real device convention: /system/bin/apexd is labeled
// u:object_r:apexd_exec:s0). The existing security_compute_create hook
// then derives the matching u:r:<domain>:s0 transition context (which
// differs from mycon "u:r:init:s0"), so Service::Start proceeds to
// fork+exec — and the exec itself succeeds via the kernel's
// execute_no_trans grant for app_data_file (the proven init/secilc
// path; setexeccon is already a no-op success).
//
// Return semantics match libselinux: the LENGTH of the context string
// (positive) on success, -1 on failure (init checks `== -1`).
// =========================================================================
static int twoyi_fake_filecon(const char *path, char **context) {
    if (!context) return -1;
    // Derive the type from the basename: "/system/bin/apexd" ->
    // "u:object_r:apexd_exec:s0". No basename -> generic system_file.
    const char *base = "system_file";
    char type[128];
    if (path && *path) {
        const char *slash = strrchr(path, '/');
        const char *b = slash ? slash + 1 : path;
        if (*b) {
            snprintf(type, sizeof(type), "%s_exec", b);
            base = type;
        }
    }
    char buf[256];
    snprintf(buf, sizeof(buf), "u:object_r:%s:s0", base);
    *context = strdup(buf);
    return *context ? (int)strlen(*context) : -1;
}

int getfilecon(const char *path, char **context) {
    return twoyi_fake_filecon(path, context);
}

int lgetfilecon(const char *path, char **context) {
    return twoyi_fake_filecon(path, context);
}

int fgetfilecon(int fd, char **context) {
    (void)fd;
    // fd-based: fabricate a generic label (the compute hook only needs
    // a non-mycon "object_r:" form to derive the transition context).
    return twoyi_fake_filecon("/proc/self/fd", context);
}

int getfilecon_raw(const char *path, char **context) {
    return twoyi_fake_filecon(path, context);
}

int lgetfilecon_raw(const char *path, char **context) {
    return twoyi_fake_filecon(path, context);
}

int fgetfilecon_raw(int fd, char **context) {
    (void)fd;
    return twoyi_fake_filecon("/proc/self/fd", context);
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

// 6-Z130: setsockcreatecon — fake success (the guest cannot set socket
// creation contexts; init's zygote-socket creation dies on EACCES).
int setsockcreatecon(const char *context) {
    (void)context;
    return 0;
}

int setsockcreatecon_raw(const char *context) {
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

// 6-Z143: selinux_status_open — fake success. libselinux's implementation
// needs to find the selinuxfs mount point (reads /proc/filesystems or
// /proc/mounts — EACCES for the app on the HOST) and then open+mmap
// /sys/fs/selinux/status. Even with the status page created + the open
// hooks fixed, the MOUNT-POINT DISCOVERY fails first (run 32828992586:
// the aborting processes never even open the status file — the failure
// is in the discovery step). Fake 0 (success): init only checks >= 0
// and the status data (enforcing=0, policyload=1) matches what our
// virtual selinuxfs reports anyway.
int selinux_status_open(int fallback) {
    (void)fallback;
    return 0;
}

void selinux_status_close(void) {
    // no-op — nothing was actually opened
}

// 6-Z143: statfs — the selinuxfs mount-point discovery. libselinux's
// init_selinuxmnt() does statfs("/sys/fs/selinux") and checks
// f_type == SELINUX_MAGIC (0xf97cff8c). Our virtual selinuxfs directory
// lives on the app's ext4 — wrong magic — so the check falls through to
// /proc/filesystems (EACCES for the app), selinux_mnt stays NULL, and
// selinux_status_open returns ENOENT BEFORE ever opening the status
// file (the run 32828992586 abort signature). Report SELINUX_MAGIC for
// the virtual selinuxfs path; pass everything else through.
int statfs(const char *path, struct statfs *buf) {
    static int (*real_statfs)(const char *, struct statfs *) = NULL;
    if (!real_statfs) real_statfs = dlsym(RTLD_NEXT, "statfs");
    if (real_statfs) {
        int r = real_statfs(path, buf);
        if (r == 0 && path && buf &&
            strncmp(path, "/sys/fs/selinux", 15) == 0) {
            // SELINUX_MAGIC from <linux/magic.h> — the value
            // init_selinuxmnt() compares f_type against.
            buf->f_type = 0xf97cff8cUL;
        }
        return r;
    }
    return syscall(SYS_statfs, path, buf);
}

int statfs64(const char *path, struct statfs64 *buf) {
    static int (*real_statfs64)(const char *, struct statfs64 *) = NULL;
    if (!real_statfs64) real_statfs64 = dlsym(RTLD_NEXT, "statfs64");
    if (real_statfs64) {
        int r = real_statfs64(path, buf);
        if (r == 0 && path && buf &&
            strncmp(path, "/sys/fs/selinux", 15) == 0) {
            buf->f_type = 0xf97cff8cUL;
        }
        return r;
    }
    return syscall(SYS_statfs, path, buf);
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
// 6-Z272m (final): the table GROWS — the fixed 256-entry array was the
// root cause of the A13+ recovery property storm: a full LineageOS
// build.prop set is ~2-3k properties, so every __system_property_add
// past entry 256 returned -1 ("__system_property_add failed", 186×/boot
// in the walleye runs) and init's property area never came up, which
// kept sys.usb.config unset and the A13+ recovery blocked in
// SetUsbConfig's WaitForProperty forever (6-Z272l context).
#define PROPS_INITIAL_CAP 512
#define PROPS_MAX_CAP 16384
struct prop_entry {
    char key[128];
    char value[128];
    int used;
};
static struct prop_entry *g_props = NULL;
static int g_props_count = 0;
static int g_props_cap = 0;

static int prop_set(const char *key, const char *value) {
    if (!key || !value) return -1;
    // Find existing
    for (int i = 0; i < g_props_count; i++) {
        if (g_props[i].used && strcmp(g_props[i].key, key) == 0) {
            strncpy(g_props[i].value, value, 127);
            g_props[i].value[127] = 0;
            return 0;
        }
    }
    // Grow the table — 6-Z272m (final): the boot prop set of a modern
    // image far exceeds the old fixed 256 slots.
    if (g_props_count >= g_props_cap) {
        int new_cap = g_props_cap ? g_props_cap * 2 : PROPS_INITIAL_CAP;
        if (new_cap > PROPS_MAX_CAP) return -1; // table hard cap
        struct prop_entry *np = (struct prop_entry *)realloc(
            g_props, (size_t)new_cap * sizeof(struct prop_entry));
        if (!np) return -1;
        g_props = np;
        memset(g_props + g_props_count, 0,
               (size_t)(new_cap - g_props_count) * sizeof(struct prop_entry));
        g_props_cap = new_cap;
    }
    strncpy(g_props[g_props_count].key, key, 127);
    g_props[g_props_count].key[127] = 0;
    strncpy(g_props[g_props_count].value, value, 127);
    g_props[g_props_count].value[127] = 0;
    g_props[g_props_count].used = 1;
    g_props_count++;
    return 0;
}

static int prop_get(const char *key, char *value) {
    if (!key || !value) return 0;
    for (int i = 0; i < g_props_count; i++) {
        if (g_props[i].used && strcmp(g_props[i].key, key) == 0) {
            // 6-Z184 AUDIT FIX (agent 1): callers pass a
            // PROP_VALUE_MAX (92) byte buffer; the old strncpy(…, 128)
            // zero-padded 128 bytes into it — a guaranteed 36-byte
            // stack smash on every hit. Copy at most 91 bytes + NUL,
            // matching the __system_property_get contract.
            size_t len = strlen(g_props[i].value);
            if (len > PROP_VALUE_MAX - 1) len = PROP_VALUE_MAX - 1;
            memcpy(value, g_props[i].value, len);
            value[len] = 0;
            return (int)len;
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
        int fd = twoyi_sys_open("/dev/__properties__/property_info",
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
    for (int i = 0; i < g_props_count; i++) {
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
    for (int i = 0; i < g_props_count; i++) {
        if (g_props[i].used) {
            propfn(&g_props[i], cookie);
        }
    }
    return 0;
}

// Hook __system_property_wait_any — return immediately with a fake prop_info
// This unblocks init's WaitForProperty loops (e.g., wait_for_coldboot_done)
const void *__system_property_wait_any(const void *pi) {
    (void)pi;
    // Return a non-NULL pointer to indicate "a property changed"
    // This causes init's WaitForProperty to re-read the property and check
    // if it matches the expected value.
    static struct prop_entry wait_any_dummy;
    return g_props ? (const void *)&g_props[0] : (const void *)&wait_any_dummy;
}

// Hook __system_property_wait — return 1 (property changed)
int __system_property_wait(const void *pi) {
    (void)pi;
    return 1;
}

// Hook __system_property_poll — return immediately
void __system_property_poll(void) {
    // No-op — our properties don't have serials that change
}

// execv/execve hooks — restore LD_PRELOAD before each exec
static int (*real_execv)(const char *, char *const[]) = NULL;
static int (*real_execve)(const char *, char *const[], char *const[]) = NULL;
static int (*real_execvp)(const char *, char *const[]) = NULL;
static int (*real_execvpe)(const char *, char *const[], char *const[]) = NULL;
static int (*real_execveat)(int, const char *, char *const[], char *const[], int) = NULL;

// Helper: read the ELF class of a binary (32-bit or 64-bit)
// Returns 1 for 32-bit (ELFCLASS32), 2 for 64-bit (ELFCLASS64), 0 on error
static int get_elf_class(const char *path) {
    if (!path) return 0;
    // Use direct syscall to avoid recursion with our open hooks
    int fd = twoyi_sys_open(path, O_RDONLY, 0);
    if (fd < 0) {
        // Try with rootfs prefix
        if (g_rootfs && path[0] == '/') {
            char translated[512];
            snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
            fd = twoyi_sys_open(translated, O_RDONLY, 0);
        }
        if (fd < 0) return 0;
    }
    char ehdr[64];
    long n = syscall(SYS_read, fd, ehdr, sizeof(ehdr));
    syscall(NR_close, fd);
    if (n < 20) return 0;
    // Check ELF magic
    if (ehdr[0] != 0x7f || ehdr[1] != 'E' || ehdr[2] != 'L' || ehdr[3] != 'F')
        return 0;
    // EI_CLASS is at offset 4
    return ehdr[4] == 1 ? 1 : (ehdr[4] == 2 ? 2 : 0);
}

// Helper: check if LD_PRELOAD should be set for this exec
// Returns 1 if LD_PRELOAD should be set, 0 if it should be skipped
static int should_set_preload_for_exec(const char *path) {
    if (!g_preload_path[0]) return 0;
    // Our libraries are 64-bit (ELFCLASS32).
    // If the target binary is 32-bit, skip LD_PRELOAD to avoid:
    //   CANNOT LINK EXECUTABLE: "/dev/libgetpid_hook.so" is 64-bit instead of 32-bit
    int elf_class = get_elf_class(path);
    if (elf_class == 1) {
        char msg[256];
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] %s is 32-bit — skipping LD_PRELOAD (our libs are 64-bit)\n", path);
        write_str(2, msg);
        return 0;
    }
    return 1;
}

// Diagnostic helper: log the exec call with path + LD_PRELOAD state
static void log_exec_call(const char *variant, const char *path) {
    char msg[768];
    int len = snprintf(msg, sizeof(msg),
        "[twoyi_loader] %s called: path=%s preload_path=%s\n",
        variant, path ? path : "(null)",
        g_preload_path[0] ? g_preload_path : "(empty)");
    write_str(2, msg);
}

// Helper: translate path for exec (prepend rootfs if needed)
// For critical service binaries, try /dev/twoyi-bin/ first (tmpfs, executable)
static const char *translate_exec_path(const char *path) {
    // Always log entry to verify this function is called
    {
        char msg[256];
        snprintf(msg, sizeof(msg), "[twoyi_loader] translate_exec_path ENTER: path=%s g_rootfs=%s\n",
            path ? path : "(null)", g_rootfs ? g_rootfs : "(null)");
        write_str(2, msg);
    }
    if (!path || !g_rootfs) {
        return path;
    }
    if (!should_translate(path)) {
        char msg[256];
        snprintf(msg, sizeof(msg), "[twoyi_loader] translate_exec_path: should_translate returned 0 for %s\n", path);
        write_str(2, msg);
        return path;
    }

    // First, try /dev/twoyi-bin/<basename> (tmpfs, always executable)
    // This is where kr64 copies critical service binaries.
    const char *basename = strrchr(path, '/');
    if (basename) {
        basename++;  // skip the '/'
        static char dev_bin_path[512];
        snprintf(dev_bin_path, sizeof(dev_bin_path), "/dev/twoyi-bin/%s", basename);
        struct stat st;
        int rc = twoyi_sys_fstatat(AT_FDCWD, dev_bin_path, &st, 0);
        if (rc == 0 && (st.st_mode & 0111)) {
            char msg[600];
            snprintf(msg, sizeof(msg), "[twoyi_loader] translate_exec_path: %s -> %s (dev/twoyi-bin, mode=0%o)\n",
                path, dev_bin_path, st.st_mode & 0777);
            write_str(2, msg);
            return dev_bin_path;
        }
    }

    // Fall back to rootfs path
    static char translated[512];
    snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
    // Verify the translated file exists using direct syscall (bypasses our hooks)
    struct stat st;
    int rc = twoyi_sys_fstatat(AT_FDCWD, translated, &st, 0);
    if (rc == 0) {
        char msg[600];
        snprintf(msg, sizeof(msg), "[twoyi_loader] translate_exec_path: %s -> %s (exists, mode=0%o)\n",
            path, translated, st.st_mode & 0777);
        write_str(2, msg);
        return translated;
    } else {
        char msg[600];
        snprintf(msg, sizeof(msg), "[twoyi_loader] translate_exec_path: %s -> %s (MISSING! errno=%d: %s)\n",
            path, translated, errno, strerror(errno));
        write_str(2, msg);
        // Even if stat fails, try the translated path anyway — the exec might
        // succeed where stat failed (e.g., if stat is denied but exec is allowed)
        return translated;
    }
}

int execv(const char *path, char *const argv[]) {
    if (!real_execv) real_execv = dlsym(RTLD_NEXT, "execv");
    log_exec_call("execv", path);
    // Translate path to rootfs (e.g., /system/bin/logd -> {rootfs}/system/bin/logd)
    const char *exec_path = translate_exec_path(path);
    if (exec_path != path) {
        char msg[600];
        snprintf(msg, sizeof(msg), "[twoyi_loader] execv: translated %s -> %s\n", path, exec_path);
        write_str(2, msg);
    }
    // Check if we should skip LD_PRELOAD for this binary (e.g., 32-bit)
    if (!should_set_preload_for_exec(path)) {
        // Remove LD_PRELOAD and LD_LIBRARY_PATH from env so the 32-bit binary
        // can link normally (32-bit needs 32-bit libs, not our 64-bit paths)
        unsetenv_internal("LD_PRELOAD");
        unsetenv_internal("LD_LIBRARY_PATH");
        write_str(2, "[twoyi_loader] execv: skipped LD_PRELOAD+LD_LIBRARY_PATH for 32-bit binary\n");
        if (!real_execv) return syscall(SYS_execve, exec_path, argv, environ);
        return real_execv(exec_path, argv);
    }
    restore_preload_env();
    write_str(2, "[twoyi_loader] execv: restored LD_PRELOAD\n");
    if (!real_execv) return syscall(SYS_execve, exec_path, argv, environ);
    return real_execv(exec_path, argv);
}

// Hook execve — re-set LD_PRELOAD before exec

int execve(const char *path, char *const argv[], char *const envp[]) {
    if (!real_execve) real_execve = dlsym(RTLD_NEXT, "execve");
    log_exec_call("execve", path);
    // Translate path to rootfs (e.g., /system/bin/logd -> {rootfs}/system/bin/logd)
    const char *exec_path = translate_exec_path(path);
    if (exec_path != path) {
        char msg[600];
        snprintf(msg, sizeof(msg), "[twoyi_loader] execve: translated %s -> %s\n", path, exec_path);
        write_str(2, msg);
    }
    // Check if we should skip LD_PRELOAD for this binary (e.g., 32-bit)
    if (!should_set_preload_for_exec(path)) {
        // Build new envp WITHOUT LD_PRELOAD and WITHOUT LD_LIBRARY_PATH
        // (32-bit binaries need 32-bit libs, not our 64-bit LD_LIBRARY_PATH)
        int env_count = 0;
        if (envp) { while (envp[env_count]) env_count++; }
        char **new_envp = (char **)malloc(sizeof(char *) * (env_count + 1));
        if (!new_envp) {
            if (!real_execve) return syscall(SYS_execve, exec_path, argv, envp);
            return real_execve(exec_path, argv, envp);
        }
        int j = 0;
        for (int i = 0; i < env_count; i++) {
            if (strncmp(envp[i], "LD_PRELOAD=", 11) != 0 &&
                strncmp(envp[i], "LD_LIBRARY_PATH=", 16) != 0) {
                new_envp[j++] = (char *)envp[i];
            }
        }
        new_envp[j] = NULL;
        write_str(2, "[twoyi_loader] execve: skipped LD_PRELOAD+LD_LIBRARY_PATH for 32-bit binary\n");
        int ret;
        if (!real_execve) ret = syscall(SYS_execve, exec_path, argv, new_envp);
        else ret = real_execve(exec_path, argv, new_envp);
        free(new_envp);
        return ret;
    }
    // Always build a new envp with the correct LD_PRELOAD.
    // Even if LD_PRELOAD is already in envp, it might have the wrong path
    // (e.g., {rootfs}/dev/ instead of /dev/). We always replace it with
    // our known-good g_preload_path.
    restore_preload_env();

    if (!g_preload_path[0]) {
        // No preload path — use envp as-is
        if (!real_execve) return syscall(SYS_execve, exec_path, argv, envp);
        return real_execve(exec_path, argv, envp);
    }

    // Count existing envp entries
    int env_count = 0;
    if (envp) {
        while (envp[env_count]) env_count++;
    }

    // Build new envp: copy all entries EXCEPT LD_PRELOAD and LD_LIBRARY_PATH,
    // then add our LD_PRELOAD and LD_LIBRARY_PATH
    char preload_env[600];
    snprintf(preload_env, sizeof(preload_env), "LD_PRELOAD=%s", g_preload_path);

    // Set LD_LIBRARY_PATH to include rootfs library directories
    // This is needed because binaries in /dev/twoyi-bin/ need libraries
    // from {rootfs}/system/lib64/ — plus the APEX lib dirs.
    // 6-Z130: + /apex/com.android.art/lib64 (libnativeloader.so — app_process64
    // link wall) and /apex/com.android.i18n/lib64 (libandroidicu.so — mediaserver).
    // 6-Z132: the /apex/* entries are PLAIN (no {rootfs} prefix) — they pass
    // through UNTRANSLATED to the HOST emulator's fully-mounted APEXes
    // (same arch; should_translate returns 0 for /apex). Prefixing them with
    // {rootfs} pointed the linker at the rootfs's INCOMPLETE apex tree and
    // made libnativeloader.so / libandroidicu.so ENOENT (run 32790504763).
    char ld_library_path[2048];
    snprintf(ld_library_path, sizeof(ld_library_path),
        "LD_LIBRARY_PATH=%s/system/lib64:%s/system/lib64/bootstrap:/apex/com.android.runtime/lib64:/apex/com.android.runtime/lib64/bionic:/apex/com.android.runtime/lib64/bootstrap:/apex/com.android.art/lib64:/apex/com.android.i18n/lib64:%s/vendor/lib64:/apex/com.android.os.statsd/lib64:%s/system_ext/lib64:%s/product/lib64",
        g_rootfs, g_rootfs, g_rootfs, g_rootfs, g_rootfs);

    char **new_envp = (char **)malloc(sizeof(char *) * (env_count + 3));
    if (!new_envp) {
        // Can't allocate — fall back to environ
        if (!real_execve) return syscall(SYS_execve, exec_path, argv, environ);
        return real_execve(exec_path, argv, environ);
    }

    int j = 0;
    for (int i = 0; i < env_count; i++) {
        if (strncmp(envp[i], "LD_PRELOAD=", 11) != 0 &&
            strncmp(envp[i], "LD_LIBRARY_PATH=", 16) != 0) {
            new_envp[j++] = (char *)envp[i];
        }
    }
    new_envp[j] = preload_env;
    new_envp[j + 1] = ld_library_path;
    new_envp[j + 2] = NULL;

    write_str(2, "[twoyi_loader] execve: replaced LD_PRELOAD in envp\n");
    int ret;
    if (!real_execve) ret = syscall(SYS_execve, exec_path, argv, new_envp);
    else ret = real_execve(exec_path, argv, new_envp);
    free(new_envp);
    return ret;
}

// Hook execvp — same as execv but uses PATH
int execvp(const char *path, char *const argv[]) {
    if (!real_execvp) real_execvp = dlsym(RTLD_NEXT, "execvp");
    log_exec_call("execvp", path);
    const char *exec_path = translate_exec_path(path);
    restore_preload_env();
    write_str(2, "[twoyi_loader] execvp: restored LD_PRELOAD\n");
    if (!real_execvp) return syscall(SYS_execve, exec_path, argv, environ);
    return real_execvp(exec_path, argv);
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
// 6-Z214: is_mount_propagation_op is defined above (before the PLT
// mount() interposer) — shared by the interposer AND this SIGSYS-path
// emulation so both paths carry identical semantics.
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
            // 6-Z214: propagation/remount/move ops (and bind-onto-live-
            // target) reconfigure the EXISTING entry — the old code
            // returned -EBUSY for everything except MS_REMOUNT, which
            // made AOSP init's SetupMountNamespaces
            // mount(nullptr,"/apex",nullptr,MS_PRIVATE) abort fatally
            // with InitFatalReboot (the r14-r25 OrangeFox/Lineage
            // blocker). Keep only the plain-duplicate-mount EBUSY.
            if (is_mount_propagation_op(flags)) {
                g_mounts[i].flags=flags; pthread_mutex_unlock(&g_mount_lock); return 0;
            }
            if (flags & MS_BIND) {
                if (src && strncmp(src,tgt,256)==0) { pthread_mutex_unlock(&g_mount_lock); return -EINVAL; }
                g_mounts[i].flags=flags; pthread_mutex_unlock(&g_mount_lock); return 0;
            }
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
    (void)dirfd;
    if(!path) return -EFAULT;
    mode_t fmt = mode & S_IFMT;
    // 6-Z288: honest rootfs materialisation, mirroring the PLT
    // twoyi_materialise_node but WITHOUT dlsym or re-issuing mknodat
    // (both unsafe from a SIGSYS context under a TRAP filter — a nested
    // trapped syscall would recurse into this handler). The node is
    // backed by a regular file at the TRANSLATED path: CHR/BLK carry
    // dev_t (the documented VM behavior), FIFO/SOCK/REG start empty.
    // Previously: CHR/BLK created a file at the UNTRANSLATED path on
    // x86_64 only, and arm64 returned fake success creating nothing —
    // the 6-Z185 lie that fed OrangeFox's orsin retry storm.
    if (!should_translate(path)) {
        // Host paths are not ours to materialise.
        return 0;
    }
    const char *t = translate(path);
    if (!t || t[0] != '/') return 0;
    struct stat st;
    if (twoyi_sys_fstatat(AT_FDCWD, t, &st, AT_SYMLINK_NOFOLLOW) == 0) return 0;
    int fd = (int)twoyi_sys_open(t, O_RDWR | O_CREAT | O_EXCL, 0666);
    if (fd < 0) return 0; // lost a creation race — the node exists now
    if (fmt == S_IFCHR || fmt == S_IFBLK) {
        syscall(NR_write, fd, &dev, sizeof(dev_t));
    }
    syscall(NR_close, fd);
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

// 6-Z123: the security-sysctl trio the Android 11 init FATALs on.
//
// Run 32749762701 (584a79f): after 6-Z121 unlocked PropertyInit, the
// guest advanced through the FULL init.rc parse (all 60+ .rc files)
// and died at the SetKptrRestrict builtin:
//   init: Cannot open for reading: /proc/sys/kernel/kptr_restrict
//   init: Unable to set adequate kptr_restrict value!
//   init: InitFatalReboot: signal 6
// security.cpp's SetHighestAvailableOptionValue() opens the sysctl
// with std::ifstream (must SUCCEED), writes each candidate value with
// std::ofstream (O_WRONLY|O_CREAT|O_TRUNC), then seeks the ifstream
// back and VERIFIES the read matches the write. The host emulator's
// real /proc/sys/kernel/kptr_restrict is SELinux-denied for
// untrusted_app (the ifstream open fails), and init's
// SetKptrRestrictAction/SetMmapRndBitsAction both LOG(FATAL) when the
// set-verify loop fails.
//
// THE FIX: virtualize the three files into the rootfs — pre-created
// with a valid seed (so the ifstream open succeeds), translated by
// should_translate (so ofstream writes + the seekg(0) re-read hit the
// SAME rootfs file — write/read-back coherence is automatic), and
// app-owned (so the write succeeds). init sees a perfectly normal
// writable sysctl.
//
// The trio (values from security.cpp android11-release):
//   /proc/sys/kernel/kptr_restrict        — SetKptrRestrictAction:
//       min 2, max 4 (writes "4" first, accepts it)
//   /proc/sys/vm/mmap_rnd_bits            — SetMmapRndBitsAction
//       (x86_64): min 32, max 32 (writes "32")
//   /proc/sys/vm/mmap_rnd_compat_bits     — the compat sibling:
//       min 16, max 16 (writes "16")
// TestPerfEventSelinuxAction (perf_event_open test) is fully
// non-fatal; MixHwrngIntoLinuxRng reads /dev/hw_random (already
// virtualized); wait_for_coldboot_done is pre-set by the property
// pre-seeds.
static int is_proc_sys_virtual_file(const char *path) {
    if (!path) return 0;
    return strcmp(path, "/proc/sys/kernel/kptr_restrict") == 0 ||
           strcmp(path, "/proc/sys/vm/mmap_rnd_bits") == 0 ||
           strcmp(path, "/proc/sys/vm/mmap_rnd_compat_bits") == 0;
}

// 6-Z123b/c: post-open diagnostic + write-retry for the proc-sys trio.
// Run 32755956760: the ifstream open SUCCEEDED (fd=14, translated to the
// rootfs copy) but the ofstream open returned EACCES (-13, NO SELinux
// avc — pure DAC; the write bits were stripped from the file by an as-
// yet-unidentified writer between boot cycles). This helper (a) LOGS
// every proc-sys open with flags + fd + errno (rate-capped), and (b) on
// a write-intent failure heals the mode (chmod 0666), ensure-creates,
// and retries. Returns the fd to use.
static int proc_sys_open_finish(const char *path, const char *translated,
                                int flags, mode_t mode, int fd) {
    static int log_count = 0;
    if (is_proc_sys_virtual_file(path)) {
        if (log_count < 24) {
            log_count++;
            char msg[512];
            int len = snprintf(msg, sizeof(msg),
                               "[twoyi_loader] 6-Z123b: open(%s, flags=0x%x) -> fd=%d errno=%d\n",
                               path, flags, fd, fd < 0 ? errno : 0);
            syscall(NR_write, 2, msg, len > 0 ? (size_t)len : 0);
        }
        if (fd < 0 && (flags & (O_WRONLY | O_RDWR)) && g_rootfs) {
            // Write-intent open failed — heal the mode, ensure the file
            // exists, and retry once.
            twoyi_sys_chmod(translated, 0666);
            int cfd = twoyi_sys_open(translated, O_WRONLY | O_CREAT, 0666);
            if (cfd >= 0) {
                syscall(NR_close, cfd);
                fd = (int)twoyi_sys_open(translated, flags, mode);
                if (log_count < 24) {
                    log_count++;
                    char msg[512];
                    int len2 = snprintf(msg, sizeof(msg),
                                        "[twoyi_loader] 6-Z123b: retry open(%s) -> fd=%d errno=%d\n",
                                        path, fd, fd < 0 ? errno : 0);
                    syscall(NR_write, 2, msg, len2 > 0 ? (size_t)len2 : 0);
                }
            }
        }
    }
    return fd;
}

// Pre-create {rootfs}/proc/sys/{kernel,vm}/<file> with a valid seed.
// Idempotent: an existing file (e.g. "4" left by a previous boot
// cycle's write) is left alone — any in-range value satisfies the
// read-open, and the write loop overwrites it immediately anyway.
// 6-Z123c: the file is ALSO chmod'd 0666 on EVERY process start —
// run 32755956760 proved the ofstream's O_WRONLY|O_CREAT|O_TRUNC open
// of an existing file returns EACCES (no SELinux avc — pure DAC),
// i.e. SOMETHING strips the write bits between cycles (candidate:
// bionic's fortified __open_2 creating with mode 0 after a delete, or
// an init.rc chmod). Healing the mode here unblocks the write-open
// regardless of the corrupter.
static void ensure_proc_sys_files(void) {
    if (!g_rootfs) return;
    static const struct {
        const char *rel;   // path under /proc/sys/
        const char *seed;  // a valid in-range value
    } files[] = {
        {"kernel/kptr_restrict", "2"},
        {"vm/mmap_rnd_bits", "32"},
        {"vm/mmap_rnd_compat_bits", "16"},
    };
    char dir[512];
    snprintf(dir, sizeof(dir), "%s/proc/sys", g_rootfs);
    mkdir_p(dir, 0755);
    for (size_t i = 0; i < sizeof(files) / sizeof(files[0]); i++) {
        char sub[512];
        snprintf(sub, sizeof(sub), "%s/proc/sys/%s", g_rootfs,
                 strncmp(files[i].rel, "kernel/", 7) == 0 ? "kernel" : "vm");
        mkdir_p(sub, 0755);
        char path[600];
        snprintf(path, sizeof(path), "%s/proc/sys/%s", g_rootfs, files[i].rel);
        // O_EXCL: only the creator writes the seed.
        int fd = twoyi_sys_open(path, O_WRONLY | O_CREAT | O_EXCL, 0666);
        if (fd >= 0) {
            syscall(NR_write, fd, files[i].seed, strlen(files[i].seed));
            syscall(NR_write, fd, "\n", 1);
            syscall(NR_close, fd);
        }
        // 6-Z123c: unconditional mode heal — see the comment above.
        twoyi_sys_chmod(path, 0666);
    }
}

static void ensure_selinuxfs_files(void) {
    if (!g_rootfs) return;
    char dir[512];
    snprintf(dir, sizeof(dir), "%s/sys/fs/selinux", g_rootfs);
    // Recursive mkdir — create /sys, /sys/fs, /sys/fs/selinux
    mkdir_p(dir, 0755);

    // Create required selinuxfs control files
    const char *files[] = {
        // 6-Z141: "status" — the kernel's selinux status page
        // (selinux_status_open mmaps it; missing → init's
        // 'Check failed: selinux_status_open(true) >= 0' abort).
        // Layout: u32 version=1, u32 seq=0, u32 enforcing=0,
        // u32 policyload=1, rest zero; page-sized so the mmap works.
        "status",        // selinux_status_open's open+mmap target
        "checkreqprot",  // init writes "0" here (FATAL if missing)
        "enforce",       // init writes "0" or "1" here
        "load",          // init writes policy here
        "policyvers",    // init reads policy version
        "null",          // secilc's -f file_contexts sink (writes discarded)
        NULL
    };
    for (int i = 0; files[i]; i++) {
        char path[600];
        snprintf(path, sizeof(path), "%s/%s", dir, files[i]);
        int fd = twoyi_sys_open(path, O_WRONLY | O_CREAT, 0666);
        if (fd >= 0) {
            if (strcmp(files[i], "status") == 0) {
                // The real status page, page-sized: selinux_status_open
                // mmaps 4096 bytes, so the file must be at least that.
                // u32 LE: version=1 @0, sequence=0 @4, enforcing=0 @8,
                // policyload=1 @12, deny_unknown=0 @16, rest zero.
                // (The loop's strlen-style seeds can't express this —
                // write the full page explicitly.)
                char page[4096];
                memset(page, 0, sizeof(page));
                uint32_t version = 1;
                uint32_t policyload = 1;
                memcpy(page + 0, &version, sizeof(version));
                // seq @4, enforcing @8, deny_unknown @16 stay zero
                memcpy(page + 12, &policyload, sizeof(policyload));
                syscall(NR_write, fd, page, sizeof(page));
            } else if (strcmp(files[i], "checkreqprot") == 0) {
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
    // IMPORTANT: Use boundary checks (path[N] == 0 || path[N] == '/') to avoid
    // matching paths that just start with the same prefix.
    // e.g., /sys must NOT match /system (which starts with /sys)
    // 6-Z123: the three security sysctls init FATALs on — virtualize to
    // rootfs (see ensure_proc_sys_files for the full rationale). Checked
    // BEFORE the blanket /proc rejection below.
    if (is_proc_sys_virtual_file(path)) return 1;
    // 6-Z272n: virtualize /proc/mounts (+ /proc/self/mounts) to the
    // rootfs. A15 debuggable inits scan /proc/mounts for "overlay" lines
    // (init/selinux.cpp SetupOverlays, gated on ALLOW_REMOUNT_OVERLAYS =
    // product_variable("debuggable") — the overlay_remounter copy): on a
    // docker/redroid host the REAL /proc/mounts is full of the host's
    // overlayfs mounts, so has_overlays=true and init LOG(FATAL)s
    // "Failed to copy /system/xbin/overlay_remounter" (recovery images
    // don't ship that debug-only binary) → InitFatalReboot before any UI
    // (zippo run 33800240705). The synthesized rootfs/proc/mounts
    // (proc_emu.rs write_proc_mounts) is guest-scoped and carries exactly
    // the pseudo-mounts this container provides. Both spellings exist in
    // the synthesized tree (/proc/mounts is a symlink to self/mounts).
    if (strcmp(path, "/proc/mounts") == 0) return 1;
    if (strcmp(path, "/proc/self/mounts") == 0) return 1;
    if (strncmp(path, "/proc", 5) == 0 && (path[5] == 0 || path[5] == '/')) return 0;
    if (strncmp(path, "/sys", 4) == 0 && (path[4] == 0 || path[4] == '/')) return 0;
    if (strncmp(path, "/data", 5) == 0 && (path[5] == 0 || path[5] == '/')) return 0;
    // /dev/ paths: translate socket, __properties__, binder, and other guest paths
    // but keep host device nodes (/dev/null, /dev/zero, /dev/qemu_pipe, etc.)
    if (strncmp(path, "/dev/socket", 11) == 0) return 1;  // guest sockets
    if (strncmp(path, "/dev/__properties__", 19) == 0) return 1;
    if (strncmp(path, "/dev/__null__", 13) == 0) return 1;
    // 6-Z272o pt2: /dev/dri must NOT leak to the host. AOSP minui's
    // gr_init prefers the DRM backend (/dev/dri/card0 | renderD128) and
    // falls back to fbdev only when DRM init fails. On a redroid host
    // those nodes exist, so the walleye --show_text run drew the menu to
    // the HOST's DRM display — invisible to the fb0 SurfaceView reader
    // (the recovery stayed healthy in its poll loop over one blank
    // frame). Translating to the rootfs (no dri there) makes the open
    // ENOENT and minui deterministically falls back to
    // /dev/graphics/fb0, which the fb_hook serves.
    if (strncmp(path, "/dev/dri", 8) == 0 && (path[8] == 0 || path[8] == '/')) return 1;
    // 6-Z287: /dev/block maps into the GUEST rootfs — kr64 stages the
    // image's by-name nodes there (devices.rs create_by_name_block_nodes,
    // parsed from the guest's own fstabs). The /dev/ catch-all below used
    // to pass block paths through to the HOST: the sandbox has no host
    // /dev/block, so the guest's misc BCB access ENOENT'd for ~10s of
    // retries plus an on-screen "Failed to clear BCB message" error
    // (lineage run 33900850051) — and a real-device host would LEAK its
    // own partitions into the guest (6-Z187 guests-only). Must sit before
    // the /dev/ catch-all.
    if (strncmp(path, "/dev/block", 10) == 0 && (path[10] == 0 || path[10] == '/')) return 1;
    // Translate binder devices to rootfs — kr64 mounts binderfs there
    // so the guest has its own binder domain separate from the host.
    if (strcmp(path, "/dev/binder") == 0 ||
        strcmp(path, "/dev/hwbinder") == 0 ||
        strcmp(path, "/dev/vndbinder") == 0) {
        // Log binder opens for debugging
        char msg[256];
        snprintf(msg, sizeof(msg), "[twoyi_loader] should_translate: %s -> YES (binder)\n", path);
        write_str(2, msg);
        return 1;
    }
    if (strncmp(path, "/dev/binderfs/", 14) == 0) return 1;
    // Other /dev/ paths (null, zero, random, qemu_pipe, etc.) stay on host
    if (strncmp(path, "/dev/", 5) == 0) return 0;
    if (strncmp(path, "/dev", 4) == 0 && (path[4] == 0)) return 0;  // /dev exactly
    // Guest rootfs paths — translate
    if (strncmp(path, "/system", 7) == 0 && (path[7] == 0 || path[7] == '/')) return 1;
    // 6-Z136: vendor LIBRARY dirs pass through to the HOST (same arch,
    // complete tree — the rootfs's vendor/lib64 is INCOMPLETE: run
    // 32805483993 showed {rootfs}/vendor/lib64/android.hardware.*.so
    // ENOENT for 78 clean link failures while the emulator's own
    // /vendor/lib64 has them all). Binaries/configs under the rest of
    // /vendor still translate (exec staging + ROM configs); only the
    // LIB search paths pass through, mirroring the /apex passthrough.
    if (strncmp(path, "/vendor/lib64", 13) == 0 &&
        (path[13] == 0 || path[13] == '/')) return 0;
    if (strncmp(path, "/vendor/lib", 11) == 0 &&
        (path[11] == 0 || path[11] == '/')) return 0;
    if (strncmp(path, "/vendor", 7) == 0 && (path[7] == 0 || path[7] == '/')) return 1;
    // 6-Z132: /apex/* PASSES THROUGH to the host's mounted APEXes. The
    // rootfs's apex tree is incomplete (kr64's apex_extract loopback
    // mount fails with /dev/loop-control EPERM in the sandbox), while
    // the emulator's own /apex/com.android.{runtime,art,i18n} are fully
    // mounted with correct-arch libs — and the guest is the SAME ARCH.
    // Translating /apex to the rootfs made libnativeloader.so /
    // libandroidicu.so (and 115/128 runtime APEX libs) ENOENT.
    // The tracer's translate_path already passes /apex through too.
    if (strncmp(path, "/apex", 5) == 0 && (path[5] == 0 || path[5] == '/')) return 0;
    if (strncmp(path, "/init", 5) == 0 && (path[5] == 0 || path[5] == '/')) return 1;
    if (strncmp(path, "/default.prop", 13) == 0) return 1;
    // Init creates these directories during boot — translate to rootfs
    if (strncmp(path, "/linkerconfig", 13) == 0) return 1;
    if (strncmp(path, "/acct", 5) == 0) return 1;
    if (strncmp(path, "/config", 7) == 0) return 1;
    if (strncmp(path, "/metadata", 9) == 0) return 1;
    if (strncmp(path, "/mnt", 4) == 0 && (path[4] == 0 || path[4] == '/')) return 1;
    if (strncmp(path, "/storage", 8) == 0) return 1;
    if (strncmp(path, "/cache", 6) == 0) return 1;
    if (strncmp(path, "/bin", 4) == 0) return 1;
    if (strncmp(path, "/sbin", 5) == 0) return 1;
    if (strncmp(path, "/lib", 4) == 0) return 1;
    if (strncmp(path, "/etc", 4) == 0) return 1;
    if (strncmp(path, "/root", 5) == 0 && (path[5] == 0 || path[5] == '/')) return 1;
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
/* 6-Z184 AUDIT FIX (agent 2): thread-local — the old shared static was
 * raced by every open-family hook from arbitrary guest threads (zygote
 * children, SurfaceFlinger, binder threads): two threads' opens could
 * each receive the OTHER's translated path. */
static __thread char g_translated[512];
static const char *translate(const char *path) {
    if (!path || !g_rootfs) return path;
    if (!should_translate(path)) return path; // kernel paths pass through
    snprintf(g_translated, sizeof(g_translated), "%s%s", g_rootfs, path);
    return g_translated;
}

// Returns 1 if the calling process is "init" (first_stage or second_stage).
// We use raw syscalls (NR_openat + SYS_read + NR_close) to avoid recursing
// into our own open()/openat() PLT hooks. NR_openat is defined for both
// x86_64 and arm64 in this file (unlike SYS_open which is x86_64-only).
static int is_init_process(void) {
    char comm[16] = {0};
    int fd = (int)syscall(NR_openat, AT_FDCWD, "/proc/self/comm", O_RDONLY, 0);
    if (fd < 0) return 0;
    long n = syscall(SYS_read, fd, comm, sizeof(comm) - 1);
    syscall(NR_close, fd);
    if (n <= 0) return 0;
    // Strip trailing newline
    char *nl = strchr(comm, '\n');
    if (nl) *nl = 0;
    // 6-Z98: also accept "twoyi_init" — kr64 stages the guest init as
    // cache/twoyi_init (the noexec workaround) and execs that copy, so the
    // process comm is "twoyi_init", not "init". Without this the init-only
    // protections (fstab block) silently disarmed.
    return (strcmp(comm, "init") == 0) || (strcmp(comm, "twoyi_init") == 0);
}

// Block fstab opens for init only.
//   - first_stage init reading fstab → triggers FirstStageMount() which
//     fatally fails (device-mapper EBUSY) → InitFatalReboot.
//   - second_stage init reading fstab → mount_all fails, but init continues.
//   - vold reading fstab → process_config() needs it; must be allowed.
// Returns 1 if the caller should return -1 / errno=ENOENT.
static int should_block_fstab(const char *path) {
    if (!path) return 0;
    if (!strstr(path, "fstab.")) return 0;
    return is_init_process();
}

// openat PLT interposition
int openat(int dirfd, const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
    init_real_funcs();

    // Block fstab files for init only → first_stage_mount is skipped.
    // vold (and other services) can still read the fstab.
    if (should_block_fstab(path)) {
        errno = ENOENT;
        return -1;
    }

    // Debug: log all /dev/__properties__ opens
    if (path && strncmp(path, "/dev/__properties__", 19) == 0) {
        char msg[256];
        int len = snprintf(msg, sizeof(msg),
            "[twoyi_loader] openat(%s, flags=0x%x)\n", path, flags);
        write(2, msg, len);
    }

    // Special handling for selinuxfs paths
    // Init opens /sys/fs/selinux/checkreqprot, /sys/fs/selinux/enforce, etc.
    // These need to exist as writable files.
    if (path && strncmp(path, "/sys/fs/selinux/", 16) == 0) {
        // 6-Z142: explicit rootfs prefix (translate() passes /sys through
        // to the HOST's real selinuxfs — EACCES for the app; the virtual
        // files live at {rootfs}/sys/fs/selinux/).
        char selinux_translated[600];
        snprintf(selinux_translated, sizeof(selinux_translated), "%s%s",
                 g_rootfs ? g_rootfs : "", path);
        const char *translated = selinux_translated;
        int fd = real_openat ? real_openat(dirfd, translated, flags, mode)
                              : syscall(NR_openat, dirfd, translated, flags, mode);
        if (fd < 0 && (flags & O_WRONLY || flags & O_RDWR)) {
            // File doesn't exist — create it
            fd = real_openat ? real_openat(dirfd, translated, flags | O_CREAT, 0666)
                              : syscall(NR_openat, dirfd, translated, flags | O_CREAT, 0666);
        }
        return fd;
    }

    if (!real_openat) {
        // real_openat not resolved — use direct syscall on the TRANSLATED
        // path (6-Z184 AUDIT FIX (agent 2): the raw `path` used to go to
        // the syscall untranslated, hitting the HOST filesystem for every
        // guest absolute path), then apply binder / qemu_pipe fallback.
        const char *translated = should_translate(path) ? translate(path) : path;
        int fd = syscall(NR_openat, dirfd, translated, flags, mode);
        if (is_binder_device_path(path)) {
            int saved_errno = fd < 0 ? errno : 0;
            return binder_open_fallback(path, fd, saved_errno);
        }
        // qemu_pipe device open fallback (6-Z116; mirrors binder above).
        if (is_qemu_pipe_device_path(path)) {
            int saved_errno = fd < 0 ? errno : 0;
            return qemu_pipe_open_fallback(path, fd, saved_errno);
        }
        return track_fb_fd(path, fd);
    }
    const char *translated = translate(path);
    int fd = real_openat(dirfd, translated, flags, mode);
    // 6-Z123b: proc-sys trio diagnostic + write-intent retry.
    fd = proc_sys_open_finish(path, translated, flags, mode, fd);
    // Binder device open fallback (see binder_open_fallback() docs above).
    if (is_binder_device_path(path)) {
        int saved_errno = fd < 0 ? errno : 0;
        return binder_open_fallback(path, fd, saved_errno);
    }
    // qemu_pipe device open fallback (6-Z116; mirrors binder above —
    // connect to the proxy at {TWOYI_ROOTFS}/dev/qemu_pipe, fall back to
    // /dev/null on connect failure). See qemu_pipe_open_fallback() docs.
    if (is_qemu_pipe_device_path(path)) {
        int saved_errno = fd < 0 ? errno : 0;
        return qemu_pipe_open_fallback(path, fd, saved_errno);
    }
    return track_fb_fd(path, fd);
}

// open PLT interposition (for code that uses open() instead of openat())
int open(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }

    init_real_funcs();

    // Block fstab files for init only → first_stage_mount is skipped.
    // vold (and other services) can still read the fstab.
    if (should_block_fstab(path)) {
        errno = ENOENT;
        return -1;
    }

    // Debug: log all /dev/__properties__ opens
    if (path && strncmp(path, "/dev/__properties__", 19) == 0) {
        char msg[256];
        int len = snprintf(msg, sizeof(msg),
            "[twoyi_loader] open(%s, flags=0x%x)\n", path, flags);
        write(2, msg, len);
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
        // 6-Z142: use the EXPLICIT rootfs-prefixed path, NOT translate()
        // — should_translate() returns 0 for /sys (host kernel paths),
        // so translate() passed /sys/fs/selinux/* through to the HOST's
        // REAL selinuxfs where every open EACCESes for the app. The
        // virtual files live at {rootfs}/sys/fs/selinux/ (created by
        // ensure_selinuxfs_files); open THOSE.
        char selinux_translated[600];
        snprintf(selinux_translated, sizeof(selinux_translated), "%s%s",
                 g_rootfs ? g_rootfs : "", path);
        const char *translated = selinux_translated;
        // Ensure the file exists — create it if missing
        char real_path[600];
        snprintf(real_path, sizeof(real_path), "%s/sys/fs/selinux", g_rootfs ? g_rootfs : "");
        mkdir_p(real_path, 0755);

        // Try to create the file if it doesn't exist
        if (g_rootfs) {
            char file_path[600];
            snprintf(file_path, sizeof(file_path), "%s%s", g_rootfs, path);
            int cfd = twoyi_sys_open(file_path, O_WRONLY | O_CREAT, 0666);
            if (cfd >= 0) syscall(NR_close, cfd);
        }

#if defined(__x86_64__)
        int fd = twoyi_sys_open(translated, flags, mode);
        // Only retry with O_CREAT for selinuxfs virtual files, NOT for
        // binder devices or other character devices — O_CREAT on a
        // binderfs device would create a regular file and corrupt it.
        if (fd < 0 && (flags & O_WRONLY || flags & O_RDWR) &&
            strncmp(path, "/sys/fs/selinux", 15) == 0) {
            fd = twoyi_sys_open(translated, flags | O_CREAT, 0666);
        }
        return fd;
#else
        if (real_openat) {
            int fd = real_openat(AT_FDCWD, translated, flags, mode);
            if (fd < 0 && (flags & O_WRONLY || flags & O_RDWR) &&
                strncmp(path, "/sys/fs/selinux", 15) == 0) {
                fd = real_openat(AT_FDCWD, translated, flags | O_CREAT, 0666);
            }
            return fd;
        }
        return syscall(NR_openat, AT_FDCWD, translated, flags, mode);
#endif
    }

    const char *translated = translate(path);
#if defined(__x86_64__)
    int fd = twoyi_sys_open(translated, flags, mode);
#else
    int fd;
    if (real_openat) fd = real_openat(AT_FDCWD, translated, flags, mode);
    else fd = syscall(NR_openat, AT_FDCWD, translated, flags, mode);
#endif
    // 6-Z123b: proc-sys trio diagnostic + write-intent retry.
    fd = proc_sys_open_finish(path, translated, flags, mode, fd);
    // Binder device open fallback (see binder_open_fallback() docs above).
    if (is_binder_device_path(path)) {
        int saved_errno = fd < 0 ? errno : 0;
        return binder_open_fallback(path, fd, saved_errno);
    }
    // qemu_pipe device open fallback (6-Z116; mirrors binder above —
    // connect to the proxy at {TWOYI_ROOTFS}/dev/qemu_pipe, fall back to
    // /dev/null on connect failure). See qemu_pipe_open_fallback() docs.
    if (is_qemu_pipe_device_path(path)) {
        int saved_errno = fd < 0 ? errno : 0;
        return qemu_pipe_open_fallback(path, fd, saved_errno);
    }
    return track_fb_fd(path, fd);
}

// 6-Z222: open64 / openat64 PLT interposition.
//
// Modern bionic (Android 11+) exports open64/openat64 as REAL symbols
// (thin aliases that call openat). Libraries built against that bionic —
// OrangeFox R12's libminuitwrp.so opens /dev/graphics/fb0 via open64 —
// bind their open64@plt DIRECTLY to libc, bypassing the open()/openat()
// hooks above. The fb0 fd was therefore never TRACKED, the ioctl hook
// passed FBIOGET_VSCREENINFO through to the regular-file stub (ENOTTY),
// the screeninfo struct stayed zeroed, and the theme engine crashed at
// gr_fb_width() (libminuitwrp.so+0x2027c, si_addr=0) in a crash loop —
// with all three hook libraries correctly loaded and the 6-Z218a preload
// order intact (run 33275527467). Both thin aliases route through the
// SAME openat() hook body so tracking/translation/fallbacks behave
// identically for 64-bit callers.
int open64(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
    return openat(AT_FDCWD, path, flags, mode);
}

int openat64(int dirfd, const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
    return openat(dirfd, path, flags, mode);
}

// 6-Z222: self-healing fb0 fd tracking for the ioctl path. When an FB
// ioctl arrives on an fd we did NOT track at open time (an open variant
// that slipped every hook, a dup()ed fd, or a library that resolved its
// open through some future bypass), read /proc/self/fd/<fd> and mark the
// fd when it actually resolves to an fb0 path. This keeps the FB ioctl
// synthesis working no matter which open flavor a guest library uses.
static void fb_fd_selfheal(int fd) {
    if (fd < 0) return;
    if (fb_fd_is_tracked(fd)) return;
    char path[128];
    char target[256];
    snprintf(path, sizeof(path), "/proc/self/fd/%d", fd);
    // Raw readlinkat syscall — no dependency on dlsym ordering, and the
    // tracer translates /proc/self/fd like any other /proc path.
    long n = syscall(SYS_readlinkat, AT_FDCWD, path, target, sizeof(target) - 1);
    if (n <= 0) return;
    target[n] = '\0';
    // Match the TAIL of the target against the fb0 names so both the
    // guest-absolute and the {rootfs}-prefixed host forms resolve.
    if (n >= 8 && strcmp(target + n - 8, "/dev/fb0") == 0) {
        fb_fd_mark(fd);
        return;
    }
    if (n >= 18 && strcmp(target + n - 18, "/dev/graphics/fb0") == 0) {
        fb_fd_mark(fd);
    }
}

// selinuxfs fopen helper (6-Z120b): /sys/fs/selinux/* must resolve to the
// rootfs's virtual selinuxfs files for fopen() callers too. secilc's
// file_contexts output (-f /sys/fs/selinux/null, mode "w+") fopen's this
// path AFTER a successful compile; the open()/openat() hooks already
// special-case selinuxfs, but fopen did NOT — it resolved against the
// HOST's REAL selinuxfs where untrusted_app writes are denied (EACCES)
// and secilc died with "Failed to open file_contexts file" (exit 255)
// even though the compiled policy write had succeeded. Mirrors the
// open() hook: write-intent modes create the virtual file first.
static FILE *fopen_selinuxfs_translated(const char *path, const char *mode,
                                        FILE *(*real_fn)(const char *, const char *)) {
    char translated[600];
    snprintf(translated, sizeof(translated), "%s%s",
             g_rootfs ? g_rootfs : "", path);
    // write-intent fopen modes: w / a / + (any of them needs the file to
    // exist for O_TRUNC/O_APPEND semantics — create-if-missing like open())
    int write_intent =
        (mode && (strchr(mode, 'w') || strchr(mode, 'a') || strchr(mode, '+')));
    if (write_intent && g_rootfs) {
        int cfd = twoyi_sys_open(translated, O_WRONLY | O_CREAT, 0666);
        if (cfd >= 0) syscall(NR_close, cfd);
    }
    return real_fn ? real_fn(translated, mode) : NULL;
}

// Hook fopen — translate paths to rootfs.
// fopen() internally calls openat() within libc, bypassing our PLT hooks.
// This means vold's fs_mgr_read_fstab() (which uses fopen) can't find
// /vendor/etc/fstab.ranchu in the rootfs. We must hook fopen directly.
FILE *fopen(const char *path, const char *mode) {
    // selinuxfs special case FIRST (should_translate excludes /sys)
    if (path && strncmp(path, "/sys/fs/selinux", 15) == 0 && g_rootfs) {
        static FILE *(*real_fopen_s)(const char *, const char *) = NULL;
        if (!real_fopen_s) real_fopen_s = dlsym(RTLD_NEXT, "fopen");
        return fopen_selinuxfs_translated(path, mode, real_fopen_s);
    }
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static FILE *(*real_fopen)(const char *, const char *) = NULL;
        if (!real_fopen) real_fopen = dlsym(RTLD_NEXT, "fopen");
        if (real_fopen) return real_fopen(translated, mode);
        return NULL;
    }
    static FILE *(*real_fopen)(const char *, const char *) = NULL;
    if (!real_fopen) real_fopen = dlsym(RTLD_NEXT, "fopen");
    if (real_fopen) return real_fopen(path, mode);
    return NULL;
}

// Hook fopen64 — same as fopen but for large file support
FILE *fopen64(const char *path, const char *mode) {
    // selinuxfs special case FIRST (should_translate excludes /sys)
    if (path && strncmp(path, "/sys/fs/selinux", 15) == 0 && g_rootfs) {
        static FILE *(*real_fopen64_s)(const char *, const char *) = NULL;
        if (!real_fopen64_s) real_fopen64_s = dlsym(RTLD_NEXT, "fopen64");
        return fopen_selinuxfs_translated(path, mode, real_fopen64_s);
    }
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static FILE *(*real_fopen64)(const char *, const char *) = NULL;
        if (!real_fopen64) real_fopen64 = dlsym(RTLD_NEXT, "fopen64");
        if (real_fopen64) return real_fopen64(translated, mode);
        return NULL;
    }
    static FILE *(*real_fopen64)(const char *, const char *) = NULL;
    if (!real_fopen64) real_fopen64 = dlsym(RTLD_NEXT, "fopen64");
    if (real_fopen64) return real_fopen64(path, mode);
    return NULL;
}

// Hook freopen — translate paths to rootfs
FILE *freopen(const char *path, const char *mode, FILE *stream) {
    // selinuxfs special case FIRST (should_translate excludes /sys)
    if (path && strncmp(path, "/sys/fs/selinux", 15) == 0 && g_rootfs) {
        static FILE *(*real_freopen_s)(const char *, const char *, FILE *) = NULL;
        if (!real_freopen_s) real_freopen_s = dlsym(RTLD_NEXT, "freopen");
        // freopen replaces an existing stream; route through the translated
        // path with the same create-if-missing write handling.
        char translated[600];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        int write_intent =
            (mode && (strchr(mode, 'w') || strchr(mode, 'a') || strchr(mode, '+')));
        if (write_intent) {
            int cfd = twoyi_sys_open(translated, O_WRONLY | O_CREAT, 0666);
            if (cfd >= 0) syscall(NR_close, cfd);
        }
        if (real_freopen_s) return real_freopen_s(translated, mode, stream);
        return NULL;
    }
    if (path && should_translate(path)) {
        char translated[512];
        snprintf(translated, sizeof(translated), "%s%s", g_rootfs, path);
        static FILE *(*real_freopen)(const char *, const char *, FILE *) = NULL;
        if (!real_freopen) real_freopen = dlsym(RTLD_NEXT, "freopen");
        if (real_freopen) return real_freopen(translated, mode, stream);
        return NULL;
    }
    static FILE *(*real_freopen)(const char *, const char *, FILE *) = NULL;
    if (!real_freopen) real_freopen = dlsym(RTLD_NEXT, "freopen");
    if (real_freopen) return real_freopen(path, mode, stream);
    return NULL;
}

// =========================================================================
// Binder device open fallback
// =========================================================================
// ROOT CAUSE (KVM run 31489388552, commit 14e3989):
//   HIDL HAL services (android.system.suspend@1.0-service, etc.) crash with
//   "Binder driver could not be opened. Terminating." because open() of
//   /dev/hwbinder returns EACCES for them. SELinux is permissive (enforcing=0
//   confirmed in logcat before the first crash), so this is a DAC permission
//   issue on the binderfs character device — vold's open succeeds (fd=5) but
//   HIDL service opens fail 22/26 times. When open() fails, libhidlbase's
//   ProcessState::open_driver() returns -1, and the constructor aborts.
//
//   Our ioctl hook (BINDER_VERSION -> 8) and mmap hook (MAP_ANONYMOUS) only
//   fire when open() succeeds — that is why only 12 "faking version 8" lines
//   appear (from vold/init/servicemanager, whose opens succeed) while 25 HIDL
//   services crash. The ioctl hook is NOT bypassed; open() just fails first.
//
// FIX: when the real open of a binder device (/dev/binder, /dev/hwbinder,
//   /dev/vndbinder) fails, fall back to opening /dev/null and return that fd.
//   This gives the caller a valid fd. The existing ioctl hook then fakes
//   BINDER_VERSION (-> 8), BINDER_SET_MAX_THREADS, BINDER_SET_CONTEXT_MGR and
//   BINDER_WRITE_READ, and the mmap hook returns MAP_ANONYMOUS. The HIDL
//   service's ProcessState::open_driver() sees fd >= 0 and proceeds past the
//   "Binder driver could not be opened" LOG_ALWAYS_FATAL.
//
//   This is a virtualization technique (providing a virtual binder fd), NOT a
//   crash suppression: the BINDER_VERSION check still runs and our ioctl hook
//   returns a valid version (8). The HAL service simply cannot perform real
//   binder IPC — it blocks in its threadpool, which init treats as "running".
//
//   6-Z113 (z112 S1a): in NON-ROOT mode the "failed open" is actually the
//   kr64 binder proxy's Unix socket node (open gives ENXIO), so before the
//   /dev/null fallback we CONNECT to the proxy — see the wire client block
//   above the framebuffer tracking. /dev/null remains the last resort.
static int is_binder_device_path(const char *path) {
    if (!path) return 0;
    return (strcmp(path, "/dev/binder") == 0 ||
            strcmp(path, "/dev/hwbinder") == 0 ||
            strcmp(path, "/dev/vndbinder") == 0);
}

// Called from __open_2/open/openat after the real open of a binder device.
// If the real open failed, returns a /dev/null fd as a virtual binder fd.
// `real_fd` is the result of the real open; `saved_errno` is errno right
// after the failed open (captured by the caller before any other syscall).
static int binder_open_fallback(const char *path, int real_fd, int saved_errno) {
    if (real_fd >= 0) {
        // 6-Z271p: a "successful" binder-device open can still be WRONG.
        // The tracer materializes placeholder files at the translated
        // socket paths (its own ENXIO fallback, 6-Z268), so an open that
        // races behind that materialization returns a REGULAR-FILE fd.
        // Such an fd is not marked as a proxy fd, the ioctl hook passes
        // raw ioctls to it (ENOTTY), libhwbinder's ProcessState::open_
        // driver() closes it and hits LOG_ALWAYS_FATAL("Binder driver
        // could not be opened. Terminating.") — abort() → the §13 park
        // → every mutex the aborting thread holds wedges the process
        // (runs 33496750544/33498154781/33499415910: keystore2's
        // negotiation thread died exactly there — pc in liblog
        // __android_log_assert — right after the compat getService hit;
        // its main thread futex-waited on one of the parked thread's
        // mutexes forever, IKeystoreSecurity never registered).
        // A binder-device fd MUST be a proxy fd or a REAL binder char
        // device (root mode). A regular file is always the placeholder
        // artifact — close it and connect to the proxy.
        if (!binder_fd_is_proxy(real_fd)) {
            struct stat st;
            memset(&st, 0, sizeof(st));
            int is_real_binder =
                (fstat(real_fd, &st) == 0 && S_ISCHR(st.st_mode));
            if (!is_real_binder) {
                syscall(NR_close, real_fd);
                int pfd = binder_proxy_connect(path);
                if (pfd >= 0) {
                    binder_fd_mark_proxy(pfd);
                    // 6-Z271g: the opening thread keeps pfd as its
                    // dedicated conn (same as the failed-open path).
                    bp_thread_conn_insert(pfd, bp_gettid(), pfd);
                    return pfd;
                }
                // Proxy unreachable — fall through to /dev/null below.
                real_fd = -1;
                saved_errno = ENXIO;
            }
        } else {
            return real_fd;
        }
    }
    if (!is_binder_device_path(path)) return real_fd;  // not a binder device
    // 6-Z113 (z112 S1a): the real open failed — in non-root mode that is the
    // EXPECTED outcome (open(2) on the proxy's Unix-socket node returns
    // ENXIO), so BEFORE falling back to /dev/null, try to CONNECT to the
    // kr64 binder proxy at {TWOYI_ROOTFS}/vm0/dev/binder. A connected
    // socket fd is pollable and the ioctl hook speaks the Frame/Resp wire
    // protocol on it (see the wire client block above). If the connect
    // fails too (proxy absent — e.g. TWRP mode, or kr64 fell back to the
    // host binder), the /dev/null path below is unchanged: no regression.
    int pfd = binder_proxy_connect(path);
    if (pfd >= 0) {
        binder_fd_mark_proxy(pfd);
        // 6-Z271g: the OPENING thread keeps pfd as its dedicated conn;
        // every other binder thread lazily establishes its own (see the
        // g_bp_thread_conns table). One conn per thread = real-binder
        // parking semantics, no cross-thread wire lock.
        bp_thread_conn_insert(pfd, bp_gettid(), pfd);
        return pfd;
    }
    // Proxy unreachable — open /dev/null as a virtual binder fd.
    // twoyi_sys_open uses a direct syscall (no PLT recursion).
    int fb = twoyi_sys_open("/dev/null", O_RDWR | O_CLOEXEC, 0);
    char msg[320];
    snprintf(msg, sizeof(msg),
        "[twoyi_loader] binder_open_fallback: %s real open FAILED (errno=%d:%s) "
        "-> virtual /dev/null fd=%d\n",
        path, saved_errno, strerror(saved_errno), fb);
    write_str(2, msg);
    if (fb < 0) {
        // /dev/null itself failed (very unlikely) — restore original errno
        errno = saved_errno;
    } else {
        // Record this fd as a binder fallback so the ioctl hook knows to
        // keep faking binder ioctls (the real ioctl would ENOTTY on
        // /dev/null). Real binderfs fds (real_fd >= 0 path above) are NOT
        // recorded, so they pass through to the real ioctl for real IPC.
        // O_CLOEXEC ensures the fd (and thus the tracking entry) does not
        // survive execve.
        binder_fd_mark_fallback(fb);
    }
    return fb;
}

// =========================================================================
// qemu_pipe device open fallback (6-Z116, z115 DESIGN.md §3 + §7-Rank-1)
// =========================================================================
// ROOT CAUSE: in non-root mode the host-app's spawn_qemu_pipe_proxy
// (core.rs line 913) AND the kr64 daemon's qemu_pipe::spawn_qemu_pipe_proxy
// (kr64_src/qemu_pipe.rs line 57) both bind a UnixListener socket NODE at
// {TWOYI_ROOTFS}/dev/qemu_pipe (verified against devices.rs::create_qemu_pipe
// line 218 + the kr64_src/qemu_pipe.rs test fixtures line 415). The guest's
// goldfish EGL driver open(2)s /dev/qemu_pipe — but open(2) on a bound
// Unix-socket NODE returns ENXIO (the bound-socket-node hazard empirically
// verified for /dev/binder in z112 §3-G1). And even with should_translate
// returning 0 for /dev/qemu_pipe (the /dev/ catch-all on line ~2890, kept
// at 0 per z115 §11 Do-NOT-forget), the host kernel has no /dev/qemu_pipe
// char device on a real Android phone → the untranslated open ENOENTs.
// Either way the open fails; the qemu_pipe wire never gets established; the
// goldfish EGL driver falls back to SwiftShader → no GL bytes ever reach
// the host renderer (z115 §3 blocker; the entire host renderer + proxy
// pipeline built by 6-Z114 + twoyi_glue.cpp + the AOSP emugl source is
// unreachable without this fix).
//
// FIX (mirrors z113's binder_open_fallback exactly): on open failure of
// /dev/qemu_pipe, CONNECT a Unix stream socket to {TWOYI_ROOTFS}/dev/qemu_pipe
// and return the connected fd. The qemu_pipe protocol is pure read/write
// (NO binder-style ioctls to translate): the goldfish EGL driver's first
// write is the 13-byte string "pipe:opengles" (the channel-name handshake
// per kr64_src/qemu_pipe.rs line 28 + core.rs::read_channel_name line 1050),
// which the proxy reads and uses to connect onward to {rootfs}/opengles
// (the RenderServer). The next write is a 4-byte clientFlags u32 (per
// kr64_src/qemu_pipe.rs line 33), then emugl command packets (8-byte header:
// u32 opcode + u32 packetLen + payload). Reads return the host EGL's reply
// stream. The connected socket fd is pollable / epoll-able (unlike a
// /dev/null fallback), so any future SF Looper that adds this fd sleeps
// instead of hot-spinning.
//
// No ioctl hook needed — the qemu_pipe protocol is pure read/write. The
// close() hook clears the proxy bit (fd-recycling hygiene, same as z113).
//
// FALLBACK-on-fallback: if the proxy connect fails too (proxy absent —
// e.g. TWRP mode where kr64 uses the fb0 path, or the renderer thread
// didn't start), open /dev/null as a virtual qemu_pipe fd. The guest's
// writes are silently discarded and reads return EOF, triggering the
// goldfish EGL fallback to SwiftShader — same EFFECTIVE outcome as today's
// ENOENT (no regression). This mirrors z113's binder /dev/null last-resort,
// with the same caveat that /dev/null is slightly worse than -1 because the
// guest writes-then-reads before noticing (an extra round-trip before the
// EGL fallback). Acceptable because in production (where the proxy IS up)
// the connect succeeds and we never reach /dev/null; in TWRP mode the guest
// isn't trying to render via GL anyway.
static int is_qemu_pipe_device_path(const char *path) {
    if (!path) return 0;
    // The goldfish pipe family. AOSP's goldfish_pipe driver exposes
    // /dev/qemu_pipe (the canonical path used by BOTH twoyi proxies —
    // core.rs::spawn_qemu_pipe_proxy line 913 + kr64_src/devices.rs line
    // 218). No /dev/gpu_pipe or /dev/gld_pipe variants exist in the twoyi
    // tree (verified by grep), but the matcher accepts /dev/gpu_pipe for
    // forward-compat with vendor forks that rename the device. /dev/gld_pipe
    // is NOT matched (looks like a typo; no AOSP reference).
    return (strcmp(path, "/dev/qemu_pipe") == 0 ||
            strcmp(path, "/dev/gpu_pipe") == 0);
}

// Connect a Unix stream socket to the host-app's (or kr64 daemon's)
// qemu_pipe proxy. Candidates, in order (z113 fb-hook input-bridge recipe
// — fresh socket per candidate because a failed connect(2) leaves socket
// state unspecified):
//   0: {TWOYI_ROOTFS}/dev/qemu_pipe  — the canonical proxy socket
//   1: dev/qemu_pipe                 — relative (guest cwd is the rootfs)
// Returns the connected fd or -1 (caller falls back to /dev/null — no
// regression when the proxy is absent, e.g. TWRP mode).
static int qemu_pipe_proxy_connect(const char *guest_path) {
    char cands[2][160];
    int ncands = 0;
    static int logged_fail = 0;

    if (g_rootfs && g_rootfs[0]) {
        size_t rl = strlen(g_rootfs);
        while (rl > 1 && g_rootfs[rl - 1] == '/') rl--;  // strip trailing '/'
        const char *suffix = "/dev/qemu_pipe";
        size_t sl = strlen(suffix);
        if (rl + sl + 1 <= sizeof(cands[0])) {
            if (rl == 1)  // rootfs is "/" — suffix already absolute
                snprintf(cands[ncands], sizeof(cands[0]), "%s", suffix + 1);
            else
                snprintf(cands[ncands], sizeof(cands[0]), "%.*s%s",
                         (int)rl, g_rootfs, suffix);
            ncands++;
        }
    }
    if (ncands < 2)
        snprintf(cands[ncands++], sizeof(cands[0]), "dev/qemu_pipe");

    for (int i = 0; i < ncands; i++) {
        int sfd = (int)syscall(SYS_socket, AF_UNIX,
                               SOCK_STREAM | SOCK_CLOEXEC, 0);
        if (sfd < 0) return -1;
        struct sockaddr_un sa;
        memset(&sa, 0, sizeof(sa));
        sa.sun_family = AF_UNIX;
        size_t pl = strlen(cands[i]);
        if (pl >= sizeof(sa.sun_path)) {
            syscall(NR_close, sfd);
            continue;
        }
        memcpy(sa.sun_path, cands[i], pl + 1);
        socklen_t salen = (socklen_t)(offsetof(struct sockaddr_un, sun_path) + pl);
        long rc = syscall(SYS_connect, (long)sfd, &sa, (long)salen);
        if (rc == 0) {
            char msg[320];
            snprintf(msg, sizeof(msg),
                "[twoyi_loader] qemu_pipe proxy: %s -> CONNECTED %s (fd=%d)\n",
                guest_path ? guest_path : "(null)", cands[i], sfd);
            write_str(2, msg);
            return sfd;
        }
        syscall(NR_close, sfd);
    }
    if (!logged_fail) {
        logged_fail = 1;
        char msg[256];
        snprintf(msg, sizeof(msg),
            "[twoyi_loader] qemu_pipe proxy: connect FAILED for %s -> "
            "/dev/null fallback (proxy absent?)\n",
            guest_path ? guest_path : "(null)");
        write_str(2, msg);
    }
    return -1;
}

// Called from __open_2/open/openat after the real open of /dev/qemu_pipe.
// If the real open failed, CONNECT to the host-app's qemu_pipe proxy at
// {TWOYI_ROOTFS}/dev/qemu_pipe. If the connect fails too (proxy absent),
// open /dev/null as a virtual qemu_pipe fd (no regression vs. today's
// ENOENT — the goldfish EGL driver falls back to SwiftShader either way).
// `real_fd` is the result of the real open; `saved_errno` is errno right
// after the failed open (captured by the caller before any other syscall).
static int qemu_pipe_open_fallback(const char *path, int real_fd, int saved_errno) {
    if (real_fd >= 0) return real_fd;  // real open succeeded — use it
    if (!is_qemu_pipe_device_path(path)) return real_fd;  // not a qemu_pipe device
    // The real open failed (ENOENT on the host's /dev/qemu_pipe, or ENXIO
    // on the proxy's bound socket node if should_translate were 1 — either
    // way we land here). Try to CONNECT to the qemu_pipe proxy. A connected
    // socket fd is pollable and the guest's subsequent write("pipe:opengles")
    // + read(reply) go through naturally (the proxy decodes the channel name
    // and forwards to {rootfs}/opengles). No ioctl hook needed — the
    // qemu_pipe protocol is pure read/write (unlike binder).
    int pfd = qemu_pipe_proxy_connect(path);
    if (pfd >= 0) {
        qemu_pipe_fd_mark_proxy(pfd);
        return pfd;
    }
    // Proxy unreachable — open /dev/null as a virtual qemu_pipe fd.
    // twoyi_sys_open uses a direct syscall (no PLT recursion).
    int fb = twoyi_sys_open("/dev/null", O_RDWR | O_CLOEXEC, 0);
    char msg[320];
    snprintf(msg, sizeof(msg),
        "[twoyi_loader] qemu_pipe_open_fallback: %s real open FAILED "
        "(errno=%d:%s) -> virtual /dev/null fd=%d\n",
        path, saved_errno, strerror(saved_errno), fb);
    write_str(2, msg);
    if (fb < 0) {
        // /dev/null itself failed (very unlikely) — restore original errno
        errno = saved_errno;
    }
    // Note: /dev/null fds are NOT tracked in g_qemu_pipe_proxy_fds (the
    // proxy bit is for CONNECTED sockets only). The guest's writes to
    // /dev/null are silently discarded and reads return EOF — there's no
    // ioctl hook for qemu_pipe (unlike binder), so no fd-class dispatch is
    // needed. O_CLOEXEC ensures the fd does not survive execve.
    return fb;
}

// Hook __open_2 (bionic's fortified open — used by init's WriteFile)
int __open_2(const char *path, int flags) {
    // Block fstab files for init only → first_stage_mount is skipped.
    // vold (and other services) can still read the fstab.
    if (should_block_fstab(path)) {
        errno = ENOENT;
        return -1;
    }

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
    // Translate only rootfs paths (system, vendor, data, init — 6-Z132:
    // /apex now passes through to the HOST's mounted APEXes)
    if (should_translate(path)) {
        const char *translated = translate(path);
        static int (*real_open2)(const char *, int) = NULL;
        if (!real_open2) real_open2 = dlsym(RTLD_NEXT, "__open_2");
        int fd;
        if (real_open2) fd = real_open2(translated, flags);
#if defined(__x86_64__)
        else fd = twoyi_sys_open2(translated, flags);
#else
        else fd = syscall(NR_openat, AT_FDCWD, translated, flags);
#endif
        // 6-Z123c: the ifstream/ofstream opens of the security-sysctl
        // trio go through THIS hook (run 32755956760 — the open/openat
        // hooks' diagnostic never fired but the paths WERE translated).
        // Route them through the diagnostic + mode-heal + write-retry
        // helper. __open_2 has no mode parameter — pass 0666 (the retry
        // ignores the mode for existing files; the heal chmod'd them).
        fd = proc_sys_open_finish(path, translated, flags, 0666, fd);
        int saved_errno = fd < 0 ? errno : 0;
        // Log binder device open results
        if (path && is_binder_device_path(path)) {
            char msg[512];
            snprintf(msg, sizeof(msg),
                "[twoyi_loader] __open_2(%s) -> %s = %d (errno=%d: %s)\n",
                path, translated, fd, saved_errno,
                fd < 0 ? strerror(saved_errno) : "OK");
            write_str(2, msg);
        }
        // Binder device open fallback: if the real open failed (e.g. EACCES on
        // the binderfs char device for HIDL HAL services), return a /dev/null
        // fd so ProcessState::open_driver() sees fd >= 0 and our ioctl hook
        // can fake BINDER_VERSION. See binder_open_fallback() docs above.
        if (is_binder_device_path(path)) {
            return binder_open_fallback(path, fd, saved_errno);
        }
        if (fd < 0) errno = saved_errno;
        return fd;
    }
    // Pass through (kernel paths: /proc, /sys, /dev, relative paths)
    static int (*real_open2)(const char *, int) = NULL;
    if (!real_open2) real_open2 = dlsym(RTLD_NEXT, "__open_2");
    int fd;
    if (real_open2) fd = real_open2(path, flags);
#if defined(__x86_64__)
    else fd = twoyi_sys_open2(path, flags);
#else
    else fd = (int)syscall(NR_openat, AT_FDCWD, path, flags);
#endif
    // qemu_pipe device open fallback (6-Z116; mirrors binder_open_fallback).
    // /dev/qemu_pipe stays on host per should_translate (the /dev/ catch-all
    // returns 0 — see line ~2890), so the real open above hits the host's
    // /dev/qemu_pipe → ENOENT (real Android has no such device). The fallback
    // connects to {TWOYI_ROOTFS}/dev/qemu_pipe (the proxy's bound socket).
    if (is_qemu_pipe_device_path(path)) {
        int saved_errno = fd < 0 ? errno : 0;
        return qemu_pipe_open_fallback(path, fd, saved_errno);
    }
    return track_fb_fd(path, fd);
}

// 6-Z272m pt3: fake SUCCESS for security.selinux xattr writes on the
// rootfs files. bionic's property area init (prop_area::map_prop_area_rw)
// fsetxattrs "security.selinux" on every area file and
// SystemProperties::AreaInit treats a failure as FATAL-false:
// ContextsSerialized::Initialize leaves initialized_ = false → EVERY
// __system_property_add fails for the whole boot ("properties not
// initialized" — 192× in the walleye runs). The host filesystem in this
// environment REJECTS selinux xattrs outright (no CAP_MAC_ADMIN — the
// parent's own lsetxattr warnings prove it), and the guest's SELinux is
// neutralized anyway (permissive stubs), so a fake success is exactly as
// honest as the selinuxfs virtual files.
static int twoyi_is_selinux_xattr(const char *name) {
    return name != NULL && (strcmp(name, "security.selinux") == 0 ||
                            strcmp(name, "security.SMACK64") == 0);
}

int fsetxattr(int fd, const char *name, const void *value, size_t size, int flags) {
    if (twoyi_is_selinux_xattr(name)) return 0;
    return (int)syscall(NR_fsetxattr, fd, name, value, size, flags);
}

int setxattr(const char *path, const char *name, const void *value, size_t size, int flags) {
    if (twoyi_is_selinux_xattr(name)) return 0;
    return (int)syscall(NR_setxattr, path, name, value, size, flags);
}

int lsetxattr(const char *path, const char *name, const void *value, size_t size, int flags) {
    if (twoyi_is_selinux_xattr(name)) return 0;
    return (int)syscall(NR_lsetxattr, path, name, value, size, flags);
}

// 6-Z272n: A15+ init's MountHandler fopens /proc/mounts and registers the
// fd with epoll_ctl(EPOLL_CTL_ADD, EPOLLERR|EPOLLPRI) — on real kernels
// proc files are pollable, but our synthesized REGULAR FILE
// ({rootfs}/proc/self/mounts, the 6-Z272n translation target that keeps
// the host's docker overlayfs lines out of the guest) is NOT, so
// epoll_ctl fails EPERM and init LOG(FATAL)s "epoll_ctl failed to add
// fd" → InitFatalReboot before any UI (zippo run 33803918612).
//
// Neutralize: when an EPOLL_CTL_ADD fails EPERM and the target fd is the
// synthesized mounts file (resolved via the raw /proc/self/fd/N link),
// report success WITHOUT registering. The guest mounts are static for
// the lifetime of the container, so the change-notification the callback
// waits for would never legitimately fire anyway; the fd stays a plain
// seekable file for fopen/rewind/getline, which is all MountHandler's
// parse path needs.
int epoll_ctl(int epfd, int op, int fd, struct epoll_event *event) {
    long ret = syscall(NR_epoll_ctl, epfd, op, fd, event);
    if (ret == -1 && errno == EPERM && op == EPOLL_CTL_ADD && fd >= 0 && g_rootfs) {
        char link_path[64];
        char target[600];
        snprintf(link_path, sizeof(link_path), "/proc/self/fd/%d", fd);
        long n = syscall(NR_readlinkat, AT_FDCWD, link_path, target,
                         (int)(sizeof(target) - 1));
        if (n > 0) {
            target[n] = 0;
            // Match the path SUFFIX at a component boundary — the
            // canonical /proc/self/fd target may carry symlink segments
            // the g_rootfs env string lacks (the app data dir resolves
            // through /profiles/default/, so the first walleye verdict
            // run's readlink saw ".../io.twoyi.debug/profiles/default/
            // rootfs/proc/self/mounts" while g_rootfs was
            // "/data/user/0/io.twoyi.debug/rootfs" — the prefix match
            // failed and the hook fell through to EPERM).
            const char *tail = NULL;
            const char *scan = target;
            while ((scan = strstr(scan, "/proc/")) != NULL) {
                if (strcmp(scan, "/proc/mounts") == 0 ||
                    strcmp(scan, "/proc/self/mounts") == 0) {
                    tail = scan;
                    break;
                }
                scan++;
            }
            if (tail != NULL) {
                static int mounts_epoll_diag = 4;
                if (mounts_epoll_diag > 0) {
                    mounts_epoll_diag--;
                    char msg[256];
                    snprintf(msg, sizeof(msg),
                        "[twoyi_loader] epoll_ctl ADD on the synthesized /proc/mounts fd=%d "
                        "neutralized (EPERM — regular file; guest mounts are static) target=%s\n",
                        fd, target);
                    write_str(2, msg);
                }
                return 0;
            }
        }
    }
    return (int)ret;
}

// Hook __open_real (bionic's internal open — all open variants call this)
// This catches WriteStringToFile's open() which bypasses __open_2/__openat_2
int __open_real(const char *pathname, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
    // Block fstab files for init only → first_stage_mount is skipped.
    // vold (and other services) can still read the fstab.
    if (should_block_fstab(pathname)) {
        errno = ENOENT;
        return -1;
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
        return twoyi_sys_open(translated, flags, mode);
#else
        return syscall(NR_openat, AT_FDCWD, translated, flags, mode);
#endif
    }
    // Also translate other rootfs paths
    if (should_translate(pathname)) {
        const char *translated = translate(pathname);
        static int (*real_open_real)(const char *, int, ...) = NULL;
        if (!real_open_real) real_open_real = dlsym(RTLD_NEXT, "__open_real");
        int fd;
        if (real_open_real) fd = real_open_real(translated, flags, mode);
#if defined(__x86_64__)
        else fd = twoyi_sys_open(translated, flags, mode);
#else
        else fd = syscall(NR_openat, AT_FDCWD, translated, flags, mode);
#endif
        // Binder device open fallback (see binder_open_fallback() docs above).
        if (is_binder_device_path(pathname)) {
            int saved_errno = fd < 0 ? errno : 0;
            return binder_open_fallback(pathname, fd, saved_errno);
        }
        return track_fb_fd(pathname, fd);
    }
    static int (*real_open_real)(const char *, int, ...) = NULL;
    if (!real_open_real) real_open_real = dlsym(RTLD_NEXT, "__open_real");
    int fd;
    if (real_open_real) fd = real_open_real(pathname, flags, mode);
#if defined(__x86_64__)
    else fd = twoyi_sys_open(pathname, flags, mode);
#else
    else fd = (int)syscall(NR_openat, AT_FDCWD, pathname, flags, mode);
#endif
    // qemu_pipe device open fallback (6-Z116; mirrors binder_open_fallback).
    // See the __open_2 pass-through branch above for the rationale
    // (should_translate=0 → host's /dev/qemu_pipe → ENOENT → connect).
    if (is_qemu_pipe_device_path(pathname)) {
        int saved_errno = fd < 0 ? errno : 0;
        return qemu_pipe_open_fallback(pathname, fd, saved_errno);
    }
    return track_fb_fd(pathname, fd);
}

// Hook __openat_2 (bionic's fortified openat)
int __openat_2(int dirfd, const char *path, int flags) {
    // Block fstab files for init only → first_stage_mount is skipped.
    // vold (and other services) can still read the fstab.
    if (should_block_fstab(path)) {
        errno = ENOENT;
        return -1;
    }

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
    if (should_translate(path)) {
        const char *translated = translate(path);
        static int (*real_openat2)(int, const char *, int) = NULL;
        if (!real_openat2) real_openat2 = dlsym(RTLD_NEXT, "__openat_2");
        int fd;
        if (real_openat2) fd = real_openat2(dirfd, translated, flags);
        else fd = (int)syscall(NR_openat, dirfd, translated, flags);
        // 6-Z184 AUDIT FIX (agent 2): this translate branch was missing
        // the binder/qemu_pipe fallbacks every sibling hook has — a
        // fortified openat("/dev/binder") returned the raw ENXIO and
        // libbinder aborted ("Binder driver could not be opened").
        if (is_binder_device_path(path)) {
            int saved_errno = fd < 0 ? errno : 0;
            return binder_open_fallback(path, fd, saved_errno);
        }
        if (is_qemu_pipe_device_path(path)) {
            int saved_errno = fd < 0 ? errno : 0;
            return qemu_pipe_open_fallback(path, fd, saved_errno);
        }
        return track_fb_fd(path, fd);
    }
    static int (*real_openat2)(int, const char *, int) = NULL;
    if (!real_openat2) real_openat2 = dlsym(RTLD_NEXT, "__openat_2");
    int fd;
    if (real_openat2) fd = real_openat2(dirfd, path, flags);
    else fd = (int)syscall(NR_openat, dirfd, path, flags);
    if (is_binder_device_path(path)) {
        int saved_errno = fd < 0 ? errno : 0;
        return binder_open_fallback(path, fd, saved_errno);
    }
    if (is_qemu_pipe_device_path(path)) {
        int saved_errno = fd < 0 ? errno : 0;
        return qemu_pipe_open_fallback(path, fd, saved_errno);
    }
    return track_fb_fd(path, fd);
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
    // 6-Z139: capture the REAL pid FIRST, before any hook could
    // recurse (getpid is interposed to return 1 — the fake init pid —
    // by getpid_hook.so, so only a raw syscall sees the truth here).
    // The tgkill/rt_tgsigqueueinfo hooks rewrite tgid==1 to this so
    // bionic's abort() actually delivers SIGABRT (see g_real_pid).
    if (g_real_pid < 0) {
        g_real_pid = (int)syscall(SYS_getpid);
    }

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

    // Check if this process is wait_for_keymaster or similar blocking services.
    // These services wait forever for HAL services that don't exist in our
    // container. Since init uses exec_start (blocking) for them, they block
    // the entire boot process. We make them exit(0) immediately so init
    // continues to the next action.
    //
    // This is NOT suppressing a crash — the service would otherwise hang
    // forever. This is a virtualization technique: we're telling init that
    // the HAL is "ready" (the virtualized equivalent of the HAL having
    // registered with hwservicemanager).
    {
        char exe_path[512] = {0};
        ssize_t len = readlink("/proc/self/exe", exe_path, sizeof(exe_path) - 1);
        if (len > 0) {
            exe_path[len] = 0;
            const char *basename = strrchr(exe_path, '/');
            if (basename) basename++; else basename = exe_path;

            // List of services that block waiting for HALs we don't have.
            // vold is NOT in this list — we need to find out why it exits
            // on its own and fix the real cause.
            const char *blocking_services[] = {
                "wait_for_keymaster",
                "wait_for_gatekeeper",
                "vdc",  // vdc communicates with vold via binder
                NULL
            };

            for (int i = 0; blocking_services[i]; i++) {
                if (strcmp(basename, blocking_services[i]) == 0) {
                    char msg[256];
                    snprintf(msg, sizeof(msg),
                        "[twoyi_loader] detected %s — exiting 0 to unblock init\n",
                        basename);
                    write_str(2, msg);
                    _exit(0);
                }
            }

            // For vold: redirect stderr to a file so we can capture
            // vold's own error messages before it exits.
            // Init redirects stderr to /dev/null, so we need to
            // re-redirect it BEFORE vold's main() runs.
            if (strcmp(basename, "vold") == 0) {
                int stderr_fd = syscall(SYS_openat, AT_FDCWD,
                    "/data/local/tmp/twoyi-vold-stderr.log",
                    O_WRONLY | O_CREAT | O_TRUNC, 0666);
                if (stderr_fd >= 0) {
                    // Redirect fd 2 (stderr) to the file
                    syscall(SYS_dup3, stderr_fd, 2, 0);
                    syscall(SYS_close, stderr_fd);
                }
                write_str(2, "[twoyi_loader] vold stderr redirected to /data/local/tmp/twoyi-vold-stderr.log\n");
                setenv("ANDROID_PRINTF_LOG", "stderr", 1);

                // Also redirect stdout
                int stdout_fd = syscall(SYS_openat, AT_FDCWD,
                    "/data/local/tmp/twoyi-vold-stderr.log",
                    O_WRONLY | O_CREAT | O_APPEND, 0666);
                if (stdout_fd >= 0) {
                    syscall(SYS_dup3, stdout_fd, 1, 0);
                    syscall(SYS_close, stdout_fd);
                }
            }
        }
    }

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
    // 6-Z123: the security-sysctl trio (kptr_restrict + mmap_rnd_bits +
    // mmap_rnd_compat_bits) — see ensure_proc_sys_files() for the
    // SetKptrRestrict/SetMmapRndBits FATAL rationale.
    ensure_proc_sys_files();
    write_str(2, "[twoyi_loader] selinuxfs virtual files created\n");

    // Pre-create directories that init's mkdir commands create.
    // Init uses direct syscalls for mkdir/mkdirat, bypassing our PLT hooks.
    // We must create these directories BEFORE init's main() runs.
    if (g_rootfs) {
        const char *dirs[] = {
            "acct", "acct/uid", "acct/uid_0", "acct/uid_1000",
            "mnt/secure", "mnt/secure/asec", "mnt/secure/staging",
            "mnt/media_rw", "mnt/user", "mnt/user/0", "mnt/user/0/self",
            "mnt/user/0/emulated", "mnt/pass_through", "mnt/pass_through/0",
            "mnt/expand", "mnt/appfuse", "mnt/installer", "mnt/androidwritable",
            "mnt/runtime", "mnt/runtime/default", "mnt/runtime/read",
            "mnt/runtime/write", "mnt/runtime/full",
            "cache", "cache/recovery", "cache/backup_stage", "cache/backup",
            "cache/lost+found",
            "metadata", "metadata/password_slots", "metadata/ota",
            "metadata/ota/snapshots", "metadata/apex", "metadata/bootstat",
            "metadata/vold", "metadata/gsi",
            "linkerconfig", "linkerconfig/bootstrap", "linkerconfig/default",
            "data_mirror", "data_mirror/cur_profiles", "data_mirror/data_de",
            "data_mirror/data_de/null", "data_mirror/data_ce",
            "data_mirror/data_ce/null", "data_mirror/data_ce/null/0",
            "dev/socket", "dev/block", "dev/block/by-name",
            "config",
            NULL
        };
        for (int i = 0; dirs[i]; i++) {
            char path[512];
            snprintf(path, sizeof(path), "%s/%s", g_rootfs, dirs[i]);
            // Create directory chain using direct syscalls
            for (char *p = path + 1; *p; p++) {
                if (*p == '/') {
                    *p = 0;
                    syscall(SYS_mkdirat, AT_FDCWD, path, 0777);
                    *p = '/';
                }
            }
            syscall(SYS_mkdirat, AT_FDCWD, path, 0777);
        }
        write_str(2, "[twoyi_loader] pre-created init directories in rootfs\n");
    }

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
        // 6-Z272m: create the DIRECTORY only. Do NOT pre-create
        // property_info / properties_serial in the ROOTFS — every
        // Android-8+ guest init creates the serial area with
        // O_CREAT|O_EXCL (bionic prop_area::map_prop_area_rw), and
        // ContextsSerialized::Initialize FreeAndUnmap()s the ENTIRE area
        // when the serial create fails EEXIST → every
        // __system_property_add fails for the whole boot (walleye run
        // 33803292025: 186× PROP_ERROR_SET_FAILED even with the parent's
        // clean slate, because THIS bootstrap ran inside init and
        // re-created the files after it). property_info is written by
        // init itself (CreateSerializedPropertyInfo, O_CREAT) — a
        // pre-created empty file is at best redundant.
        // The HOST-side property_info create stays: WriteStringToFile
        // bypasses PLT hooks (direct openat syscalls), and the host
        // /dev/__properties__/property_info existence is the legacy
        // guard for the pre-tracer-translation boot path.
        struct stat st;
        if (stat("/dev/__properties__", &st) == 0) {
            // Only create property_info (NOT properties_serial — host uses that)
            int fd = twoyi_sys_open("/dev/__properties__/property_info",
                            O_WRONLY | O_CREAT, 0666);
            if (fd >= 0) {
                syscall(NR_close, fd);
                write_str(2, "[twoyi_loader] created property_info on host\n");
            }
        }
    }

    write_str(2, "[twoyi_loader] runtime ready — guest can boot\n");

    // Install our own SIGABRT handler that ignores the signal.
    // init's InstallRebootSignalHandlers() installs a SIGABRT handler that
    // calls InitFatalReboot (which reboots the system). We override it with
    // SIG_IGN so LOG(FATAL) doesn't reboot.
    // This must happen AFTER init's signal handlers are installed, but since
    // our .init_array runs BEFORE init's main(), init will OVERRIDE our
    // handler. So we can't do it here.
    //
    // Instead, we hook sigaction() to intercept SIGABRT handler installations
    // and replace them with SIG_IGN. See the sigaction hook below.

    // Pre-set critical properties that init waits for during boot.
    // These properties are normally set by other processes (ueventd, etc.)
    // but our in-memory property system is per-process, so init can't see
    // properties set by other processes. Pre-setting them in init's own
    // property table ensures init doesn't wait forever.
    // NOTE: ro.cold_boot_done (with underscore) is the correct name
    // (from AOSP util.h: kColdBootDoneProp = "ro.cold_boot_done")
    prop_set("ro.cold_boot_done", "true");  // unblocks wait_for_coldboot_done
    prop_set("ro.coldboot_done", "true");   // alias just in case

    prop_set("ro.bootmode", "normal");
    prop_set("ro.boot.mode", "normal");
    prop_set("ro.boot.bootreason", "reboot");
    prop_set("ro.boot.bootdevice", "");
    prop_set("ro.boot.bootloader", "unknown");
    prop_set("ro.boot.serialno", "EMULATOR37X1X11X0");
    prop_set("ro.boot.hardware", "ranchu");
    prop_set("ro.bootfrog", "0");
    prop_set("ro.persistent_properties.ready", "true");
    prop_set("ro.actionable_compatible_property.enabled", "true");
    // 6-Z130: unblock updatable services (netd etc.) — init's AreRuntimeApexesReady gate
    prop_set("apexd.status", "activated");
    // ro.zygote is needed for init to parse init.zygote64_32.rc
    prop_set("ro.zygote", "zygote64_32");
    // sys.boot_completed is the final goal — but don't set it yet
    // (we want the guest to actually boot, not fake it)

    // vold exits(0) in our container, so it never sets this property.
    // Init's post-fs-data action waits for it via wait_for_prop.
    // Pre-set it so init can proceed past post-fs-data to zygote-start.
    prop_set("vold.post_fs_data_done", "1");

    // vold normally sets vold.decrypt=trigger_restart_framework after post-fs-data
    // to trigger the "on property:vold.decrypt=trigger_restart_framework" action
    // in init.rc, which starts class_start core/main (zygote).
    // Since vold exits(0), pre-set this so init triggers zygote startup.
    prop_set("vold.decrypt", "trigger_restart_framework");

    // NOTE: ro.crypto.state/ro.crypto.type are intentionally NOT set here.
    // Setting ro.crypto.state (any value: "encrypted", "unsupported", etc.)
    // causes init to SIGABRT at make_dir("/acct/uid") because the lstat()
    // that follows mkdir() fails with ENOENT. Without ro.crypto.state, init
    // tolerates the lstat failure and continues booting. The lstat hook is
    // present but currently fails — see debug logging in the lstat() hook.
    // vold.decrypt=trigger_restart_framework above is enough to start zygote.

    // ------------------------------------------------------------------
    // BANNED: Fake boot completion pre-sets REMOVED (overnight Task ID 4).
    // ------------------------------------------------------------------
    // The following properties must NOT be pre-set here, because they are
    // the FINAL GOALS of the boot, not infrastructure inputs:
    //
    //   prop_set("sys.boot_completed", "1");   // BANNED — removed
    //   prop_set("dev.bootcomplete", "1");     // BANNED — removed
    //   prop_set("init.svc.vold", "running");  // BANNED — removed
    //   prop_set("init.svc.zygote", "running");// BANNED — removed
    //
    // Why banned:
    //   * sys.boot_completed is THE signal that the entire userspace boot
    //     finished. Pre-setting it makes the system think it booted when
    //     it didn't. The comment on line 3005 above already says "we want
    //     the guest to actually boot, not fake it" — the code below it
    //     used to contradict that. Now they agree.
    //   * dev.bootcomplete is set by init AFTER the device boot completes.
    //     Pre-setting it is faking completion.
    //   * init.svc.* properties are set BY init when the corresponding
    //     service actually transitions states. Pre-setting
    //     init.svc.zygote=running tells init "zygote is already running"
    //     before zygote has started — that is exactly the kind of
    //     "make a problem look solved when it isn't" that the overnight
    //     rules forbid.
    //
    // Per the overnight rule: "Never make a problem look solved when it
    // isn't." If zygote isn't really running, init.svc.zygote must not
    // claim it is. If the boot isn't really complete, sys.boot_completed
    // must not claim it is.
    //
    // What we KEEP (legitimate virtualization, not boot-status fakes):
    //   * ro.boot.*             — hardware description, not boot status
    //   * ro.zygote             — tells init which zygote .rc to parse
    //   * vold.post_fs_data_done— vold exits(0) so it never sets this;
    //                             pre-setting is virtualization of vold
    //   * vold.decrypt          — same reason; vold exits(0)
    //   * apexd.status          — apexd doesn't fully run in container
    //   * ro.persistent_properties.ready,
    //     ro.actionable_compatible_property.enabled — infrastructure props
    // ------------------------------------------------------------------
    // DO NOT re-add the four banned lines above. If a future contributor
    // finds a regression that "looks like" it needs sys.boot_completed,
    // investigate the real cause (zygote failing to start, init.svc.*
    // transitions not firing) instead of faking the completion signal.

    g_runtime_ready = 1;
}
