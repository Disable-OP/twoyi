// twrp_fb_hook.c — Minimal i686 LD_PRELOAD library for TWRP framebuffer
// virtualization.
//
// WHY THIS FILE EXISTS:
//   TWRP's recovery binary is i386 (32-bit x86). The main
//   libtwoyi_loader_shlib.so is x86_64 (64-bit). The 32-bit bionic linker
//   in TWRP's recovery process CANNOT load a 64-bit LD_PRELOAD library
//   ("CANNOT LINK EXECUTABLE: ... is 64-bit instead of 32-bit"). So we
//   need a SEPARATE 32-bit hook library specifically for TWRP mode.
//
//   We build this with:
//     clang -target i686-linux-android24 -nostdlib -shared -fPIC
//   The -nostdlib leaves libc/libdl symbols unresolved; bionic resolves
//   them at load time from the recovery binary's own libc/libdl. This
//   works because NDK r27c dropped i686 sysroot libs, but the resulting
//   .so is still a valid 32-bit ELF that bionic can load.
//
// WHAT THIS HOOKS:
//   - open() / openat() / __open_2() / __openat_2() — track opens of
//     /dev/graphics/fb0 and /dev/fb0 (record returned fd).
//   - close() — clear fd tracking.
//   - ioctl() — respond to FB ioctls with valid screen info
//     (320x640 @ 32bpp, RGBA8888). This is the FIX for the libminuitwrp
//     segfault at offset 0x57d7 (NULL deref after FBIOGET_VSCREENINFO
//     returned ENOTTY on /dev/null and the struct stayed zeroed).
//
// INPUT BRIDGE (6-Z93) — synthesize the evdev stream TWRP's minui needs:
//   - open/openat/__open_2/__openat_2 of /dev/input/event0|event1|event2|
//     touch: connect a Unix stream socket to the HOST touch-events socket
//     ({data_dir}/dev/touch-events, where {data_dir} = dirname of
//     $TWOYI_ROOTFS, with a relative "../dev/touch-events" fallback since
//     the guest's cwd IS the rootfs) and return the CONNECTED SOCKET FD as
//     the "evdev fd". Falls back to the real open (a pre-created regular
//     file — harmless EOF) if the connect cannot be verified.
//   - ioctl() on input fds: fake EVIOCGBIT/EVIOCGVERSION/EVIOCGID/
//     EVIOCGNAME/EVIOCGABS so minui's ev_init capability probe accepts
//     the device (EV_SYN+EV_KEY+EV_ABS; ABS_X/Y + ABS_MT_POSITION_X/Y +
//     ABS_MT_TRACKING_ID + ABS_MT_PRESSURE; BTN_TOUCH).
//   - poll() on input fds: checked ENTIRELY IN USERSPACE (raw recv-peek is
//     unnecessary — we drain with plain read(2), which the ptrace tracer
//     never rewrites). This is REQUIRED because kr64's tracer fakes every
//     positive raw poll() return to 0 and zeroes revents (6-Z5) — minui
//     would never see readability through a real poll syscall.
//   - read() on input fds: drain 20-byte TouchMessage records from the
//     socket (action+pointer_id+x+y+pressure, all LE s32/u32) and
//     synthesize 16-byte i386 struct input_event frames (EV_ABS
//     ABS_MT_* + ABS_X/Y, EV_KEY BTN_TOUCH, EV_SYN SYN_REPORT).
//   - /dev/input/event0|event1 are pre-created as regular files by
//     kr64 itself (devices phase, parent-side) so minui's /dev/input
//     scan finds openable "event*" names in the first place.
//
// WHAT THIS DOES NOT HOOK:
//   - mmap() — REMOVED. kr64 pre-creates /dev/graphics/fb0 as a regular
//     file of exactly 3,686,400 bytes, so bionic's native mmap() works
//     without intervention. (The old mmap hook required dlsym, which is
//     unavailable on TWRP's old bionic — see the weak-dlsym comment below.)
//   - mount/mkdir/etc — TWRP's init is statically linked and doesn't
//     need path translation or mount emulation.
//   - execv/execve — TWRP's recovery doesn't fork+exec other binaries
//     that need LD_PRELOAD propagation.
//   - seccomp/SIGSYS — not needed for TWRP (init is static, no syscall
//     virtualization required).
//
// ARCHITECTURE: i386 (32-bit x86). Struct layouts match the i386 ABI:
//   - sizeof(fb_var_screeninfo) = 160 (no longs/pointers, same on all arches)
//   - sizeof(fb_fix_screeninfo) = 68 on i386 (smem_start + mmio_start are
//     4-byte unsigned long; on x86_64 they'd be 8 bytes → 80 bytes)
//   - FB ioctl numbers (0x4600, 0x4601, 0x4602, ...) do NOT encode size,
//     so they're identical on i386 and x86_64.

#include <stdarg.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <linux/fb.h>

// ---------------------------------------------------------------------------
// WEAK dlsym DECLARATION — we deliberately do NOT include <dlfcn.h> because
// it declares dlsym as a strong extern, which bionic's old linker (AOSP 5.1,
// Android L) refuses to leave unresolved: it errors out with
// "CANNOT LINK EXECUTABLE DEPENDENCIES: cannot locate symbol 'dlsym'
// referenced by 'libtwrp_fb_hook.so'" (strace-confirmed in KVM runs 31574428304
// and 31576126359 — the recovery binary doesn't link against libdl.so, so
// dlsym is unresolvable from the LD_PRELOAD).
//
// By declaring dlsym as a WEAK symbol, bionic's linker will leave it as NULL
// when it can't be resolved, instead of erroring out. At runtime we check
// `if (dlsym)` before calling it, and fall back to raw syscalls when it's
// NULL. This lets the SAME .so load on both:
//   - modern bionic (libdl loaded) — uses dlsym to find real_* functions
//   - old bionic (libdl not loaded) — uses raw_syscall* fallbacks
//
// RTLD_NEXT is normally defined in <dlfcn.h> as ((void *)-1L); we redefine
// it here since we're not including the header.
// ---------------------------------------------------------------------------
extern void *dlsym(void *handle, const char *symbol) __attribute__((weak));
#define RTLD_NEXT ((void *)-1L)

// ── 6-Z246: RTLD_NEXT self-resolution guard ──────────────────────────
//
// Old arm32 bionic linkers (the Android M/N-era soinfo traversal that
// old-platform TWRP ramdisks ship: lux/m8/osprey/surnia/thea/titan/
// seed/victara/xt1032/woods — the whole TWRP 3.7.0_9 arm32 class) can
// resolve dlsym(RTLD_NEXT, X) FROM AN LD_PRELOADed LIBRARY to the
// PRELOAD'S OWN EXPORT of X. Evidence (run 33317153489, osprey, decoded
// with the 6-Z243 compat forensics): the arm32 __open_2's
// `if (real_open2) blx r2` call site re-entered the hook's own
// __open_2 endlessly — 0x28-byte frames with the same path/flags
// arguments, si_addr = sp-0x24 at a helper's prologue push, [stack]
// grown to its full 16 MB — the guest died of main-stack exhaustion
// before the first open ever completed. The crashing register x2 even
// HELD the address of the hook's own __open_2.
//
// The generic defense: after every dlsym(RTLD_NEXT), REJECT any result
// that lies inside the hook's own executable segment and fall back to
// the raw-syscall path exactly as if dlsym had failed (the pre-6-Z246
// contract for weak-dlsym-unresolved). The segment range is derived at
// first use from the interposers' own addresses (they all live in that
// one segment), so no linker-specific APIs are needed.
static unsigned long hook_own_lo = 0;
static unsigned long hook_own_hi = 0;
static int hook_own_diag = 0;

/* forward declarations — the guard sits before these definitions */
static void write_str(int fd, const char *s);
ssize_t read(int fd, void *buf, size_t count);
void __assert2(const char *file, int line, const char *expr);

static void *hook_de_self(void *p, const char *what) {
    unsigned long v = (unsigned long)p;
    if (p == 0) return 0;
    if (hook_own_lo == 0) {
        /* one-time: bracket the hook's own exec segment with two
         * interposer addresses guaranteed to be its first and last
         * exported code (read is .text's first export, __assert2 sits
         * at its tail). Round to page granularity so internal helpers
         * between them are covered too. */
        unsigned long a = (unsigned long)(void *)&read;
        unsigned long b = (unsigned long)(void *)&__assert2;
        if (a > b) { unsigned long t = a; a = b; b = t; }
        hook_own_lo = a & ~0xFFFUL;
        hook_own_hi = (b + 0xFFFUL) & ~0xFFFUL;
    }
    if (v >= hook_own_lo && v < hook_own_hi) {
        if (hook_own_diag < 8) {
            hook_own_diag++;
            write_str(2, "[twrp_fb_hook] 6-Z246: dlsym(RTLD_NEXT, \"");
            write_str(2, what);
            write_str(2, "\") resolved INSIDE the hook itself — old-bionic "
                         "RTLD_NEXT quirk, using raw-syscall fallback\n");
        }
        return 0;
    }
    return p;
}

// WEAK getenv declaration — used to locate the host touch-events socket
// via $TWOYI_ROOTFS (kr64 puts TWOYI_ROOTFS=<absolute host rootfs path> in
// the guest child env; {data_dir} = dirname(rootfs), so the app's socket
// is at $TWOYI_ROOTFS/../dev/touch-events). Like dlsym, WEAK so a bionic
// that can't resolve it leaves it NULL instead of failing the whole
// LD_PRELOAD load (the guest cwd fallback covers that case: the guest's
// cwd IS the rootfs, so "../dev/touch-events" resolves to the same file).
extern char *getenv(const char *name) __attribute__((weak));

// WEAK environ — for execv()'s default environment (see the 6-Z187c
// exec interposition below).
extern char **environ __attribute__((weak));

// ---------------------------------------------------------------------------
// CUSTOM LIBC FUNCTIONS — we build with -nostdlib, so we must provide our
// own implementations of memset, strcmp, and strlen. Without these, the
// compiler generates PLT calls to these symbols, which bionic tries to
// resolve from the recovery binary's libc. But loading libc triggers a
// cascade of symbol resolution that ultimately fails with "cannot locate
// symbol 'syscall'".
//
// We use -fno-builtin-memset etc. in the build flags to prevent the
// compiler from using its built-in implementations (which would generate
// PLT calls). Then we provide our own static implementations.
// ---------------------------------------------------------------------------
static void *my_memset(void *s, int c, unsigned int n) {
    unsigned char *p = (unsigned char *)s;
    while (n--) *p++ = (unsigned char)c;
    return s;
}

static int my_strcmp(const char *a, const char *b) {
    while (*a && *a == *b) { a++; b++; }
    return (int)(unsigned char)*a - (int)(unsigned char)*b;
}

static unsigned int my_strlen(const char *s) {
    unsigned int n = 0;
    while (s[n]) n++;
    return n;
}

// ---------------------------------------------------------------------------
// RAW SYSCALL HELPERS — we use inline asm instead of calling libc's
// `syscall()` function. This is CRITICAL: TWRP's bionic linker (AOSP 5.1)
// fails to resolve the `syscall` symbol from our LD_PRELOAD library even
// though libc.so exports it (strace-confirmed in KVM run 31572816370:
// "CANNOT LINK EXECUTABLE DEPENDENCIES: cannot locate symbol \"syscall\"
// referenced by \"libtwrp_fb_hook.so\"..."). Using inline asm eliminates the
// undefined `syscall` symbol from our .so's dynsym, so bionic can load us.
//
// We support TWO architectures:
//   - i386 (32-bit x86): int $0x80, eax=num, ebx/cx/dx/si/di/bp=args
//   - aarch64 (64-bit ARM): svc #0, x8=num, x0-x5=args
//
// i386 syscall convention:
//   eax = syscall number
//   ebx = arg1, ecx = arg2, edx = arg3
//   esi = arg4, edi = arg5, ebp = arg6
//   int $0x80
//   eax = return value (negative errno on error)
//
// aarch64 syscall convention:
//   x8 = syscall number
//   x0 = arg1, x1 = arg2, x2 = arg3
//   x3 = arg4, x4 = arg5, x5 = arg6
//   svc #0
//   x0 = return value (negative errno on error)
//
// We only need 1/3/4-arg variants (no 6-arg syscalls now that the
// mmap hook is removed — see the mmap comment at the bottom). For the
// 6-arg case on i386 ebp would have to be saved/restored (it's the frame
// pointer); we avoid that entirely.
// ---------------------------------------------------------------------------
#if defined(__i386__)

static long raw_syscall1(long num, long a) {
    long ret;
    __asm__ volatile (
        "int $0x80"
        : "=a"(ret)
        : "a"(num), "b"(a)
        : "memory"
    );
    return ret;
}

static long raw_syscall2(long num, long a, long b) {
    long ret;
    __asm__ volatile (
        "int $0x80"
        : "=a"(ret)
        : "a"(num), "b"(a), "c"(b)
        : "memory"
    );
    return ret;
}

static long raw_syscall3(long num, long a, long b, long c) {
    long ret;
    __asm__ volatile (
        "int $0x80"
        : "=a"(ret)
        : "a"(num), "b"(a), "c"(b), "d"(c)
        : "memory"
    );
    return ret;
}

static long raw_syscall4(long num, long a, long b, long c, long d) {
    long ret;
    __asm__ volatile (
        "int $0x80"
        : "=a"(ret)
        : "a"(num), "b"(a), "c"(b), "d"(c), "S"(d)
        : "memory"
    );
    return ret;
}

/* 6-Z187c: no marker channel on i386 (the register file is fully used by
 * syscall args); the marked wrappers degrade to the plain ones. */
#define raw_syscall4_marked raw_syscall4
#define raw_syscall3_marked raw_syscall3

#elif defined(__aarch64__)

static long raw_syscall1(long num, long a) {
    register long x8 __asm__("x8") = num;
    register long x0 __asm__("x0") = a;
    __asm__ volatile (
        "svc #0"
        : "+r"(x0)
        : "r"(x8)
        : "memory"
    );
    return x0;
}

static long raw_syscall2(long num, long a, long b) {
    register long x8 __asm__("x8") = num;
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    __asm__ volatile (
        "svc #0"
        : "+r"(x0)
        : "r"(x8), "r"(x1)
        : "memory"
    );
    return x0;
}

static long raw_syscall3(long num, long a, long b, long c) {
    register long x8 __asm__("x8") = num;
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    register long x2 __asm__("x2") = c;
    __asm__ volatile (
        "svc #0"
        : "+r"(x0)
        : "r"(x8), "r"(x1), "r"(x2)
        : "memory"
    );
    return x0;
}

static long raw_syscall4(long num, long a, long b, long c, long d) {
    register long x8 __asm__("x8") = num;
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    register long x2 __asm__("x2") = c;
    register long x3 __asm__("x3") = d;
    __asm__ volatile (
        "svc #0"
        : "+r"(x0)
        : "r"(x8), "r"(x1), "r"(x2), "r"(x3)
        : "memory"
    );
    return x0;
}

/* ── 6-Z187c: the HOOK→TRACER syscall MARKER ────────────────────────────
 *
 * Run 33120905168: the tracer is PEEK/pvm/proc-mem BLIND on the pages
 * the hook (and the linker-mapped guest image) live on, so every path
 * the hook passes from its OWN buffers gets the tracer's +1 cwd-relative
 * fallback applied — which STRIPS the leading '/' off already-host-valid
 * {rootfs}-prefixed retry paths → ENOENT (the via=1 prefix retries were
 * being corrupted by the very fallback meant to save them).
 *
 * THE MARKER: for syscalls the hook issues with a KNOWN-GOOD HOST path,
 * set x6 (unused by openat/execve arg slots) to TWOYI_SYSCALL_MARK. The
 * tracer sees the marker and leaves the syscall COMPLETELY untouched —
 * no translation, no +1, no backstop fail-closed. The hook only uses
 * marked calls for paths it built itself as {rootfs}-prefixed (or
 * cwd-relative under a cwd it knows is the rootfs). */
#define TWOYI_SYSCALL_MARK 0x74776f7969313233ULL /* "twoyi123" */

static long raw_syscall4_marked(long num, long a, long b, long c, long d) {
    register long x8 __asm__("x8") = num;
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    register long x2 __asm__("x2") = c;
    register long x3 __asm__("x3") = d;
    register long x6 __asm__("x6") = (long)TWOYI_SYSCALL_MARK;
    __asm__ volatile (
        "svc #0"
        : "+r"(x0)
        : "r"(x8), "r"(x1), "r"(x2), "r"(x3), "r"(x6)
        : "memory"
    );
    return x0;
}

static long raw_syscall3_marked(long num, long a, long b, long c) {
    return raw_syscall4_marked(num, a, b, c, 0);
}

/* 6-Z188j: 5-arg MARKED syscall (getsockopt) — same marker contract. */
static long raw_syscall5_marked(long num, long a, long b, long c, long d, long e) {
    register long x8 __asm__("x8") = num;
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    register long x2 __asm__("x2") = c;
    register long x3 __asm__("x3") = d;
    register long x4 __asm__("x4") = e;
    register long x6 __asm__("x6") = (long)TWOYI_SYSCALL_MARK;
    __asm__ volatile (
        "svc #0"
        : "+r"(x0)
        : "r"(x8), "r"(x1), "r"(x2), "r"(x3), "r"(x4), "r"(x6)
        : "memory"
    );
    return x0;
}

#elif defined(__arm__) && !defined(__aarch64__)

/* 6-Z227: ARMv7 (AArch32) raw syscall variants — EABI convention:
 * syscall number in r7, args in r0-r5, return in r0. The kernel
 * preserves every register except r0 across svc, so r6 stays a free
 * marker channel (same contract as aarch64 x6).
 *
 * BUILD REQUIREMENT: compile with -marm -fomit-frame-pointer. In Thumb
 * mode r7 is the frame pointer, so the compiler must be told to leave
 * it free (build.sh passes both flags for the armv7a hook build). */
static long raw_syscall1(long num, long a) {
    register long r0 __asm__("r0") = a;
    register long r7 __asm__("r7") = num;
    __asm__ volatile (
        "svc #0"
        : "+r"(r0)
        : "r"(r7)
        : "memory"
    );
    return r0;
}

static long raw_syscall2(long num, long a, long b) {
    register long r0 __asm__("r0") = a;
    register long r1 __asm__("r1") = b;
    register long r7 __asm__("r7") = num;
    __asm__ volatile (
        "svc #0"
        : "+r"(r0)
        : "r"(r1), "r"(r7)
        : "memory"
    );
    return r0;
}

static long raw_syscall3(long num, long a, long b, long c) {
    register long r0 __asm__("r0") = a;
    register long r1 __asm__("r1") = b;
    register long r2 __asm__("r2") = c;
    register long r7 __asm__("r7") = num;
    __asm__ volatile (
        "svc #0"
        : "+r"(r0)
        : "r"(r1), "r"(r2), "r"(r7)
        : "memory"
    );
    return r0;
}

static long raw_syscall4(long num, long a, long b, long c, long d) {
    register long r0 __asm__("r0") = a;
    register long r1 __asm__("r1") = b;
    register long r2 __asm__("r2") = c;
    register long r3 __asm__("r3") = d;
    register long r7 __asm__("r7") = num;
    __asm__ volatile (
        "svc #0"
        : "+r"(r0)
        : "r"(r1), "r"(r2), "r"(r3), "r"(r7)
        : "memory"
    );
    return r0;
}

/* 6-Z227: the marker channel on arm32. r6 is preserved across svc and
 * unused by the 0-4 arg syscall slots, so the marked wrappers keep the
 * full 6-Z187c contract. The 64-bit marker constant ("twoyi123",
 * TWOYI_SYSCALL_MARK) does not fit in a 32-bit register, so arm32 uses
 * its low 32 bits ("i123") — the tracer's arm32 ABI must compare
 * against THIS value (see kr64 ABI_ARM32 reg_marker handling). */
#define TWOYI_SYSCALL_MARK_ARM32 0x69313233L /* low 32 of "twoyi123" */

static long raw_syscall4_marked(long num, long a, long b, long c, long d) {
    register long r0 __asm__("r0") = a;
    register long r1 __asm__("r1") = b;
    register long r2 __asm__("r2") = c;
    register long r3 __asm__("r3") = d;
    register long r6 __asm__("r6") = TWOYI_SYSCALL_MARK_ARM32;
    register long r7 __asm__("r7") = num;
    __asm__ volatile (
        "svc #0"
        : "+r"(r0)
        : "r"(r1), "r"(r2), "r"(r3), "r"(r6), "r"(r7)
        : "memory"
    );
    return r0;
}

static long raw_syscall3_marked(long num, long a, long b, long c) {
    return raw_syscall4_marked(num, a, b, c, 0);
}

/* 6-Z188j: 5-arg MARKED syscall (getsockopt) — r0-r4 args, r6 marker,
 * r7 number; r5 unused but not needed. */
static long raw_syscall5_marked(long num, long a, long b, long c, long d, long e) {
    register long r0 __asm__("r0") = a;
    register long r1 __asm__("r1") = b;
    register long r2 __asm__("r2") = c;
    register long r3 __asm__("r3") = d;
    register long r4 __asm__("r4") = e;
    register long r6 __asm__("r6") = TWOYI_SYSCALL_MARK_ARM32;
    register long r7 __asm__("r7") = num;
    __asm__ volatile (
        "svc #0"
        : "+r"(r0)
        : "r"(r1), "r"(r2), "r"(r3), "r"(r4), "r"(r6), "r"(r7)
        : "memory"
    );
    return r0;
}

#else
#error "twrp_fb_hook.c: unsupported architecture (need __i386__, __arm__, or __aarch64__)"
#endif

// ── 6-Z227: per-arch raw syscall NUMBERS for the calls below ──────────
// These were hardcoded aarch64 values (209/199/24/25) which are wrong
// on every other architecture — arm32 and ia32 have completely
// different tables. Verified against the kernel tables:
//   arch/arm/tools/syscall.tbl, arch/x86/entry/syscalls/syscall_32.tbl.
#if defined(__aarch64__)
  #define TWOYI_NR_GETSOCKOPT 209
  #define TWOYI_NR_SOCKETPAIR 199
  #define TWOYI_NR_DUP        24   /* dup3(oldfd, newfd, flags) */
  #define TWOYI_NR_FCNTL      25
#elif defined(__arm__) && !defined(__aarch64__)
  #define TWOYI_NR_GETSOCKOPT 295
  #define TWOYI_NR_SOCKETPAIR 288
  #define TWOYI_NR_DUP        358  /* dup3 */
  #define TWOYI_NR_FCNTL      55
#elif defined(__i386__)
  #define TWOYI_NR_GETSOCKOPT 365
  #define TWOYI_NR_SOCKETPAIR 360
  #define TWOYI_NR_DUP        330  /* dup3 */
  #define TWOYI_NR_FCNTL      55
#endif

// aarch64 has no SYS_mkdir — only SYS_mkdirat. Provide a portable wrapper.
#if defined(__aarch64__)
  #define SYS_mkdir_portable SYS_mkdirat
  #define mkdir_raw(path, mode) raw_syscall4(SYS_mkdirat, AT_FDCWD, (long)(path), (mode), 0)
#else
  #define SYS_mkdir_portable SYS_mkdir
  #define mkdir_raw(path, mode) raw_syscall3(SYS_mkdir, (long)(path), (mode), 0)
#endif

// ---------------------------------------------------------------------------
// Async-signal-safe write to stderr (for diagnostics from the constructor
// and hooks). We use raw SYS_write via inline asm to avoid recursing into
// our own hooks AND to avoid the `syscall` symbol dependency (see above).
// ---------------------------------------------------------------------------
static void write_str(int fd, const char *s) {
    size_t n = 0;
    while (s[n]) n++;
    (void)raw_syscall3(SYS_write, fd, (long)s, (long)n);
}

// Hex formatter — used for logging ioctl request numbers and function
// addresses in diagnostics. Writes "0x" + up to 8 hex digits.
static void write_hex(int fd, unsigned int val) {
    char buf[11];
    int i = (int)sizeof(buf);
    buf[--i] = '\0';
    if (val == 0) {
        buf[--i] = '0';
    } else {
        const char *hexd = "0123456789abcdef";
        while (val) { buf[--i] = hexd[val & 0xf]; val >>= 4; }
    }
    buf[--i] = 'x';
    buf[--i] = '0';
    write_str(fd, &buf[i]);
}

// 64-bit hex formatter — aarch64 code addresses exceed 32 bits, so the
// abort()/__assert2 interposition (6-Z171) needs this to print caller PCs.
static void write_hex64(int fd, unsigned long long val) {
    char buf[19];
    int i = (int)sizeof(buf);
    buf[--i] = '\0';
    if (val == 0) {
        buf[--i] = '0';
    } else {
        const char *hexd = "0123456789abcdef";
        while (val) { buf[--i] = hexd[val & 0xf]; val >>= 4; }
    }
    buf[--i] = 'x';
    buf[--i] = '0';
    write_str(fd, &buf[i]);
}

static void write_num(int fd, int v) {
    char buf[16];
    int i = (int)sizeof(buf);
    if (v == 0) {
        buf[--i] = '0';
    } else {
        int neg = 0;
        unsigned u;
        if (v < 0) { neg = 1; u = (unsigned)(-v); } else u = (unsigned)v;
        while (u) { buf[--i] = (char)('0' + (u % 10)); u /= 10; }
        if (neg) buf[--i] = '-';
    }
    (void)raw_syscall3(SYS_write, fd, (long)&buf[i], (long)(sizeof(buf) - (size_t)i));
}

// ---------------------------------------------------------------------------
// 6-Z171b: RUNTIME screen geometry (native-resolution support).
//
// The compile-time 320x640 hardcode forced the TWRP container to a fixed
// size no matter what screen the host Android actually has. The resolution
// chain is now:
//
//   Java ProfileSettings (auto-detect via DisplayMetrics, or the user's
//   per-profile override) → renderer_init(width,height) → core.rs
//   --width/--height → kr64 cfg → {rootfs}/dev/graphics/fb0 file size AND
//   the TWOYI_FB_WIDTH/TWOYI_FB_HEIGHT env vars on the TWRP child → THIS
//   hook reads them at first use and reports matching FBIOGET_VSCREENINFO /
//   FBIOGET_FSCREENINFO geometry.
//
// Fallback (env missing, e.g. very old kr64): 320x640 — redroid's own
// default panel, so the old behavior is preserved exactly.
// ---------------------------------------------------------------------------
#define TWRP_FB_WIDTH          320   /* fallback only — see fb_geometry_init */
#define TWRP_FB_HEIGHT         640   /* fallback only — see fb_geometry_init */
#define TWRP_FB_BPP            32
#define TWRP_FB_BYTES_PER_PIX  4
static int g_fb_rt_w = 0;
static int g_fb_rt_h = 0;

static int my_atoi_pos(const char *s) {
    if (!s) return 0;
    int v = 0;
    int seen = 0;
    while (*s >= '0' && *s <= '9') {
        if (v < 100000) v = v * 10 + (*s - '0');
        s++;
        seen = 1;
    }
    return seen ? v : 0;
}

static void fb_geometry_init(void) {
    if (g_fb_rt_w > 0) return; /* already initialized */
    g_fb_rt_w = TWRP_FB_WIDTH;
    g_fb_rt_h = TWRP_FB_HEIGHT;
    if (getenv) {
        int w = my_atoi_pos(getenv("TWOYI_FB_WIDTH"));
        int h = my_atoi_pos(getenv("TWOYI_FB_HEIGHT"));
        if (w > 0 && h > 0) {
            g_fb_rt_w = w;
            g_fb_rt_h = h;
            write_str(2, "[twrp_fb_hook] geometry from env: ");
            write_num(2, w); write_str(2, "x"); write_num(2, h);
            write_str(2, "\n");
            return;
        }
    }
    /* 6-Z176: geometry FILE fallback — {rootfs}/dev/.twoyi-fb-geometry,
     * written by kr64's parent at fb0-creation time ("WxH\n"). This
     * removes ALL env-plumbing dependencies (TWRP's old init builds the
     * service env from init.rc setenv options only — run 33018901591
     * proved TWOYI_FB_* still did not reach the hook even after the
     * 6-Z175 init.rc patch, so the hook kept shrinking fb0 to the
     * 320x640 fallback). The file is opened via the same
     * jail-resolvable forms the hook already uses (rootfs-prefixed,
     * cwd-relative fallback — the guest cwd IS the rootfs).
     * 6-Z187: the file moved from the rootfs ROOT into {rootfs}/dev/
     * so TWRP's File Manager shows the guest tree only — all three
     * candidate forms below updated in lockstep. */
    {
        static const char *rel = "dev/.twoyi-fb-geometry";
        char pathbuf[560];
        const char *gp = NULL;
        if (getenv) {
            const char *root = getenv("TWOYI_ROOTFS");
            if (root && root[0] == '/') {
                int i = 0;
                while (root[i] && i < 500) { pathbuf[i] = root[i]; i++; }
                pathbuf[i++] = '/';
                int j = 0;
                while (rel[j] && i < 558) { pathbuf[i] = rel[j]; i++; j++; }
                pathbuf[i] = '\0';
                gp = pathbuf;
            }
        }
        int gfd = -1;
        if (gp) gfd = (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)gp, 0 /*O_RDONLY*/, 0);
        if (gfd < 0)
            gfd = (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)rel, 0, 0);
        /* 6-Z182: the CHROOT-ABSOLUTE candidate "/dev/.twoyi-fb-geometry".
         * Run 33061152563: TWOYI_ROOTFS env reached the hook but its
         * value is a HOST path that does not exist INSIDE the jail
         * (openat translated it to {rootfs}<host-path> -> ENOENT), and
         * the cwd-relative open also missed (recovery's cwd is not the
         * rootfs root). The ABSOLUTE "/dev/.twoyi-fb-geometry" is mapped
         * by the tracer to {rootfs}/dev/.twoyi-fb-geometry — exactly
         * where kr64's create_twrp_framebuffer wrote it (6-Z187) — and is
         * correct in every jail mode (chroot, pivot_root, no-namespace).
         * LEGACY: the pre-6-Z187 "/.twoyi-fb-geometry" root candidate is
         * kept as a LAST resort so an old rootfs still boots. */
        if (gfd < 0)
            gfd = (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)"/dev/.twoyi-fb-geometry", 0, 0);
        if (gfd < 0)
            gfd = (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)"/.twoyi-fb-geometry", 0, 0);
        if (gfd >= 0) {
            char gbuf[32];
            long n = raw_syscall3(SYS_read, gfd, (long)gbuf, (long)(sizeof(gbuf) - 1));
            raw_syscall1(SYS_close, gfd);
            if (n > 0) {
                gbuf[n] = '\0';
                int w = 0, h = 0;
                const char *p = gbuf;
                while (*p >= '0' && *p <= '9') { w = w * 10 + (*p - '0'); p++; }
                if (*p == 'x' || *p == 'X') {
                    p++;
                    while (*p >= '0' && *p <= '9') { h = h * 10 + (*p - '0'); p++; }
                }
                if (w > 0 && h > 0) {
                    g_fb_rt_w = w;
                    g_fb_rt_h = h;
                    write_str(2, "[twrp_fb_hook] geometry from file: ");
                    write_num(2, w); write_str(2, "x"); write_num(2, h);
                    write_str(2, "\n");
                    return;
                }
            }
        }
    }
    write_str(2, "[twrp_fb_hook] geometry: env+file missing -> fallback ");
    write_num(2, g_fb_rt_w); write_str(2, "x"); write_num(2, g_fb_rt_h);
    write_str(2, "\n");
}

static int fb_w(void)  { fb_geometry_init(); return g_fb_rt_w; }
static int fb_h(void)  { fb_geometry_init(); return g_fb_rt_h; }
static long fb_line_length(void) { return (long)fb_w() * TWRP_FB_BYTES_PER_PIX; }
static long fb_smem_len(void)    { return (long)fb_w() * (long)fb_h() * TWRP_FB_BYTES_PER_PIX; }

// ---------------------------------------------------------------------------
// Framebuffer fd tracking.
//
// When open/openat/__open_2/__openat_2 successfully opens /dev/graphics/fb0
// or /dev/fb0, the returned fd is recorded here. The ioctl hook checks this
// set to decide whether to fake FB ioctls (the regular file would return
// ENOTTY from the kernel for FBIOGET_VSCREENINFO). The close hook clears
// entries when fds are closed.
//
// Limitation: dup/dup2/dup3 of an fb0 fd are not tracked. This is acceptable
// for TWRP — libminuitwrp doesn't dup the fb0 fd.
// ---------------------------------------------------------------------------
#define TWRP_FB_MAX_FD 1024
static unsigned char g_fb_fds[(TWRP_FB_MAX_FD + 7) / 8];

static void fb_fd_mark(int fd) {
    if (fd < 0 || fd >= TWRP_FB_MAX_FD) return;
    g_fb_fds[fd >> 3] |= (unsigned char)(1u << (fd & 7));
}

static int fb_fd_is_tracked(int fd) {
    if (fd < 0 || fd >= TWRP_FB_MAX_FD) return 0;
    return (g_fb_fds[fd >> 3] >> (fd & 7)) & 1;
}

static void fb_fd_clear(int fd) {
    if (fd < 0 || fd >= TWRP_FB_MAX_FD) return;
    g_fb_fds[fd >> 3] &= (unsigned char)~(1u << (fd & 7));
}

// Returns 1 if path is /dev/graphics/fb0 or /dev/fb0 (exact match).
static int is_fb_path(const char *path) {
    if (!path) return 0;
    if (my_strcmp(path, "/dev/graphics/fb0") == 0) return 1;
    if (my_strcmp(path, "/dev/fb0") == 0) return 1;
    return 0;
}

// ---------------------------------------------------------------------------
// 6-Z171c: /dev/ashmem support (regular file + faked ioctls).
//
// Run 33010273952 (arm64): right after the splash asset loads, recovery
// opened "/dev/ashmem" -> fd=-1 and "/dev/pmsg0" -> fd=-1, then the child
// SELF-ABORTED (tgkill SIGABRT) inside minui gr_init. Many TWRP builds'
// graphics backends allocate the backbuffer via ashmem; the ENOENT on
// /dev/ashmem is a prime abort suspect. kr64 now pre-creates BOTH files
// as app-owned regular files; we mark successful ashmem opens and fake
// the ASHMEM_* ioctl protocol on them:
//   - ASHMEM_SET_SIZE  -> ftruncate the backing file to the size (so the
//     caller's later mmap(len, MAP_SHARED, fd) has a big-enough file) + 0
//   - ASHMEM_GET_SIZE  -> the last SET_SIZE value
//   - SET_NAME / SET_PROT_MASK / PIN / UNPIN -> 0
// The mmap itself is NOT touched: on aarch64 the tracer does not rewrite
// file-backed MAP_SHARED (that rewrite is i386-only), so the mapping is
// a real file-backed one and shared-memory semantics "just work" for a
// single-process user (minui).
//
// ioctl numbers: bionic's ashmem.h builds them with _IOW(0x77, nr, T),
// so the size field differs between 32-bit (i386: 0x400877xx) and 64-bit
// (aarch64: 0x401077xx for pointer/long-sized args). We accept BOTH.
// ---------------------------------------------------------------------------
#define ASHMEM_NAME        0x7701
#define ASHMEM_SET_NAME    0x7702
#define ASHMEM_SET_SIZE    0x7703
#define ASHMEM_GET_SIZE    0x7704
#define ASHMEM_SET_PROT_MASK 0x7705
static unsigned char g_ash_fds[(TWRP_FB_MAX_FD + 7) / 8];
static long g_ash_size[TWRP_FB_MAX_FD]; /* last SET_SIZE per fd (page-rounded file len) */

static void ash_fd_mark(int fd) {
    if (fd < 0 || fd >= TWRP_FB_MAX_FD) return;
    g_ash_fds[fd >> 3] |= (unsigned char)(1u << (fd & 7));
    g_ash_size[fd] = 0;
}
static int ash_fd_is_tracked(int fd) {
    if (fd < 0 || fd >= TWRP_FB_MAX_FD) return 0;
    return (g_ash_fds[fd >> 3] >> (fd & 7)) & 1;
}
static void ash_fd_clear(int fd) {
    if (fd < 0 || fd >= TWRP_FB_MAX_FD) return;
    g_ash_fds[fd >> 3] &= (unsigned char)~(1u << (fd & 7));
}

static int is_ashmem_path(const char *path) {
    if (!path) return 0;
    if (my_strcmp(path, "/dev/ashmem") == 0) return 1;
    return 0;
}

// Strip the _IOC size/dir bits: keep type (0x77) + nr. Returns 0 if the
// request is not an ashmem-family ioctl at all.
static unsigned ash_req_nr(unsigned req) {
    if (((req >> 8) & 0xff) != 0x77) return 0;
    return req & 0xff;
}

// Handle one ioctl on a tracked ashmem fd. Returns the ioctl return value,
// or -2 when the request is not ashmem-family (caller passes through).
static int ashmem_ioctl(int fd, unsigned req, unsigned long arg) {
    unsigned nr = ash_req_nr(req);
    if (!nr) return -2;
    switch (nr) {
        case 0x03: { /* ASHMEM_SET_SIZE: arg IS the size (by value) */
            long size = (long)arg;
            if (size < 0) size = 0;
            long fl = (long)raw_syscall3(SYS_fcntl, fd, 4 /*F_SETFL*/, 0);
            (void)fl;
            long r = raw_syscall3(SYS_ftruncate, fd, size, 0);
            g_ash_size[fd >= 0 && fd < TWRP_FB_MAX_FD ? fd : 0] = size;
            write_str(2, "[twrp_fb_hook] ashmem SET_SIZE(");
            write_num(2, (int)size);
            write_str(2, ") ftruncate -> ");
            write_num(2, (int)r);
            write_str(2, "\n");
            return 0;
        }
        case 0x04: /* ASHMEM_GET_SIZE */
            return (fd >= 0 && fd < TWRP_FB_MAX_FD) ? (int)g_ash_size[fd] : 0;
        case 0x01: case 0x02: case 0x05: /* NAME / SET_NAME / SET_PROT_MASK */
        case 0x06: case 0x07: case 0x08: case 0x09: case 0x0a: case 0x0b:
        case 0x0c: case 0x0d: /* PIN / UNPIN / GET_PIN / PURGE / ... */
            return 0;
        default:
            return 0;
    }
}

// ---------------------------------------------------------------------------
// ── 6-Z188: SOCKETPAIR-BACKED PTY EMULATION ─────────────────────────────
//
// Run 33122549751 proved the terminal chain now reaches the shell:
// the pty child forked, execl("/sbin/sh") SUCCEEDED via the 6-Z187
// provisioning (+1 cwd-relative, ret=0) — and the shell immediately
// exit_group(1)'d. Root cause: there is no real /dev/ptmx or /dev/pts/N
// in the sandbox. Recovery terminal emulators do:
//
//     fdMaster = getpt();                    // open("/dev/ptmx")
//     unlockpt(fdMaster);                    // ioctl TIOCSPTLCK 0
//     pid = fork();
//     child: fdSlave = open(ptsname(fdMaster));   // "/dev/pts/N" -> ENOENT
//           dup2(fdSlave, 0/1/2); close(fdSlave);
//           setsid(); ioctl(0, TIOCSCTTY, 1);
//           execl("/sbin/sh", "sh", NULL);
//
// With fdSlave = -1 the dup2s fail and the shell runs with dead stdio
// and exits 1. THE GENERIC FIX (no tracer changes, recovery-agnostic —
// works for ANY recovery using the standard pty API): back the "pty"
// with an AF_UNIX socketpair created at getpt() time:
//
//   open("/dev/ptmx")        -> socketpair(); return sv[0] as master,
//                                stash sv[1] in slot i (pty index i)
//   ioctl(TIOCSPTLCK)        -> 0        (unlockpt succeeds)
//   ioctl(TIOCGPTN)          -> slot i   (ptsname builds /dev/pts/i)
//   ptsname(fd)              -> "/dev/pts/<i>" (directly interposed too)
//   ioctl(TIOCSCTTY)         -> 0        (session-ctty "grant")
//   open("/dev/pts/<i>")     -> dup(stashed sv[1]) — the child inherited
//                                the socket end at fork; reads/writes on
//                                stdio reach the GUI's master fd directly.
//
// aarch64 only (i386 socketpair needs socketcall plumbing; x86 TWRP
// images don't exercise the terminal in CI).
// ---------------------------------------------------------------------------
#if defined(__aarch64__)
#define TWOYI_PTY_SLOTS 4
static int  g_pty_master_fd[TWOYI_PTY_SLOTS];   /* sv[0] handed to caller */
static int  g_pty_slave_fd[TWOYI_PTY_SLOTS];    /* sv[1] stashed          */
static int  g_pty_backup_fd[TWOYI_PTY_SLOTS];   /* 6-Z188g: extra dup of the
                                                   slave — survives a
                                                   targeted close of the
                                                   original number      */
static unsigned char g_pty_slave_fds[(1024 + 7) / 8]; /* every fd that is a
                                                   live slave socket end */

static void pty_mark_slave_fd(int fd) {
    if (fd < 0 || fd >= 1024) return;
    g_pty_slave_fds[fd >> 3] |= (unsigned char)(1u << (fd & 7));
}

static int pty_is_slave_fd(int fd) {
    if (fd < 0 || fd >= 1024) return 0;
    return (g_pty_slave_fds[fd >> 3] >> (fd & 7)) & 1;
}

/* 6-Z188j: STATELESS slave detection — is `fd` a connected unix stream
 * socket? After execve("/sbin/sh") the hook state RESETS (fresh
 * constructor, empty bitmaps — run 33130609939: "ioctl(fd=0, 0x5401)
 * [trk=0]" in the sh process, isatty failed again). But the fds
 * THEMSELVES survive exec: a raw getsockopt(SO_TYPE) succeeds exactly
 * on sockets, and inside the recovery sandbox the only stream sockets
 * on stdio are our pty ends. No state, exec-proof, generic. */
static int pty_fd_is_stream_socket(int fd) {
    long val = 0;
    long len = 4;
    long r = raw_syscall5_marked(TWOYI_NR_GETSOCKOPT,
                                 (long)fd, 1 /*SOL_SOCKET*/, 3 /*SO_TYPE*/,
                                 (long)&val, (long)&len);
    return (r == 0 && val == 1 /*SOCK_STREAM*/) ? 1 : 0;
}

static void pty_init_slots(void) {
    static int inited = 0;
    if (!inited) {
        for (int i = 0; i < TWOYI_PTY_SLOTS; i++) {
            g_pty_master_fd[i] = -1;
            g_pty_slave_fd[i] = -1;
        }
        inited = 1;
    }
}

static int is_ptmx_path(const char *path) {
    if (!path) return 0;
    if (my_strcmp(path, "/dev/ptmx") == 0) return 1;
    if (my_strcmp(path, "/dev/pts/ptmx") == 0) return 1;
    return 0;
}

/* open("/dev/ptmx") -> master socket end. -1 when no slot/kernel fail. */
static int pty_open_master(int flags) {
    (void)flags;
    pty_init_slots();
    int slot = -1;
    for (int i = 0; i < TWOYI_PTY_SLOTS; i++) {
        if (g_pty_master_fd[i] < 0) { slot = i; break; }
    }
    if (slot < 0) return -1;
    /* 6-Z188e: THE root cause of every BAD dump (runs 33126808683 +
     * 33127733712: sv=[-1 N -1 -1]): socketpair's out-param is
     * `int sv[2]` — TWO 32-BIT INTS, 8 BYTES TOTAL. My buffer was
     * `long[]`, so BOTH fds landed inside the FIRST long element
     * (fd0 | fd1<<32): the (int) log prints showed only fd0 and the
     * <1024 sanity check on the huge long failed. Use the real type. */
    for (int attempt = 0; attempt < 3; attempt++) {
        int sv_fds[2] = { -1, -1 };
        long r = raw_syscall4_marked(TWOYI_NR_SOCKETPAIR,
                                     1 /*AF_UNIX*/, 1 /*SOCK_STREAM*/, 0,
                                     (long)sv_fds);
        if (r == 0 && sv_fds[0] >= 0 && sv_fds[0] < 1024
                   && sv_fds[1] >= 0 && sv_fds[1] < 1024
                   && sv_fds[0] != sv_fds[1]) {
            g_pty_master_fd[slot] = sv_fds[0];
            g_pty_slave_fd[slot]  = sv_fds[1];
            /* 6-Z188g: a BACKUP dup of the slave — run 33129187020 saw
             * the child's dup(slave) return -EBADF while the master fd
             * lived: something closed the slave number alone. A second
             * copy (whatever fd the kernel picks) survives that. */
            long b = raw_syscall3_marked(TWOYI_NR_DUP, (long)sv_fds[1], 0, 0);
            g_pty_backup_fd[slot] = (b >= 0 && b < 1024) ? (int)b : -1;
            pty_mark_slave_fd(sv_fds[1]);
            if (g_pty_backup_fd[slot] >= 0) pty_mark_slave_fd(g_pty_backup_fd[slot]);
            write_str(2, "[twrp_fb_hook] pty master fd=");
            write_num(2, sv_fds[0]);
            write_str(2, " (slot ");
            write_num(2, slot);
            write_str(2, ", slave fd=");
            write_num(2, sv_fds[1]);
            write_str(2, ")\n");
            return sv_fds[0];
        }
        write_str(2, "[twrp_fb_hook] pty socketpair BAD ret=");
        write_num(2, (int)r);
        write_str(2, " sv=[");
        write_num(2, sv_fds[0]);
        write_str(2, " ");
        write_num(2, sv_fds[1]);
        write_str(2, "] (attempt ");
        write_num(2, attempt);
        write_str(2, ")\n");
        if (sv_fds[0] >= 0 && sv_fds[0] < 1024) raw_syscall1(SYS_close, sv_fds[0]);
        if (sv_fds[1] >= 0 && sv_fds[1] < 1024) raw_syscall1(SYS_close, sv_fds[1]);
    }
    return -1;
}

/* 6-Z188c: shared dispatch for ALL FOUR open wrappers (open/openat/
 * __open_2/__openat_2 — bionic's fortified variants are what TWRP
 * actually calls at some sites; run 33125938988's slave open went
 * through __openat_2 and missed the pty checks entirely). Returns -2
 * when the path is not a pty path (caller falls through). */
static int pty_open_slave(const char *path); /* fwd (defined below) */

static int pty_open_dispatch(const char *path, int flags) {
    if (is_ptmx_path(path)) {
        int pfd = pty_open_master(flags);
        if (pfd >= 0) return pfd;
        /* 6-Z188d: a /dev/ptmx STUB FILE may exist in the rootfs — never
         * fall through to it (run 33126808683: the stub open SUCCEEDED,
         * fd=18, and unlockpt/TIOCGPTN then ENOTTY'd downstream). A
         * failed pty allocation must look like "no ptmx device": ENOENT.
         */
        write_str(2, "[twrp_fb_hook] ptmx allocation failed -> ENOENT (not falling through to any stub file)\n");
        return -1;
    }
    int sfd = pty_open_slave(path);
    if (sfd != -2) return sfd;
    return -2;
}

static int pty_slot_of_master(int fd); /* fwd — used by pty_slave_ioctl */

static int pty_slot_of_master(int fd) {
    pty_init_slots();
    for (int i = 0; i < TWOYI_PTY_SLOTS; i++)
        if (g_pty_master_fd[i] == fd) return i;
    return -1;
}

/* open("/dev/pts/<n>") — 6-Z188f: n is the SLAVE FD NUMBER itself
 * (see ptsname): the fork child INHERITED that exact fd, so dup(n)
 * works with ZERO per-process hook state. Processes holding the slot
 * table (the creator) resolve identically via the table. */
static int pty_open_slave(const char *path) {
    if (!path) return -2;
    static const char pfx[] = "/dev/pts/";
    for (int j = 0; j < 9; j++) {
        if (path[j] != pfx[j]) {
            if (path[j] == '\0') return -2;
            break;
        }
    }
    int n = 0; int any = 0;
    for (const char *p = path + 9; *p >= '0' && *p <= '9'; p++) {
        n = n * 10 + (*p - '0'); any = 1;
    }
    /* No digits after the prefix (or a trailing non-digit like
     * "/dev/pts/ptmx") = not a slave-address form we serve: -2 lets the
     * caller fall through to the real open (never shadow foreign paths). */
    if (!any) return -2;
    if (n < 0 || n >= 1024) return -1;
    /* 6-Z188g: MARKED dup — the tracer's socket-fd bookkeeping never
     * saw the marked socketpair, and run 33129187020's plain dup(17)
     * came back -EBADF while fcntl said live (a faked dup). A marked
     * dup is left untouched — the KERNEL decides. */
    long d = raw_syscall3_marked(TWOYI_NR_DUP, (long)n, 0, 0);
    /* 6-Z188k: NEVER return fd 0/1/2. Run 33131312944: the dup landed
     * on fd 0 (freed earlier in the child) and TWRP's runSlave then did
     * dup2(slave,0/1/2); close(slave) — closing fd 0 == closing STDIN
     * — ash read EOF and died silently. Run 33132103848: the plain
     * re-dup ALSO landed in 0..2 (the child had closed fd1/2 as well)
     * so the escape never fired. fcntl(d, F_DUPFD, 3) is deterministic:
     * the lowest free fd >= 3, always. */
    if (d >= 0 && d <= 2) {
        long d2 = raw_syscall3_marked(TWOYI_NR_FCNTL, d,
                                      0 /* F_DUPFD */, 3 /* min fd */);
        if (d2 >= 3) {
            raw_syscall1(SYS_close, (int)d);
            d = d2;
        }
        write_str(2, "[twrp_fb_hook] pty slave stdio-range escape -> fd=");
        write_num(2, (int)d);
        write_str(2, "\n");
    }
    if (d < 0) {
        /* Backup fd from the fork-inherited slot table (the direct fork
         * child HAS the table — ptsname worked there in every run). */
        pty_init_slots();
        for (int i = 0; i < TWOYI_PTY_SLOTS; i++) {
            if (g_pty_slave_fd[i] == n && g_pty_backup_fd[i] >= 0) {
                long d2 = raw_syscall3_marked(TWOYI_NR_DUP, (long)g_pty_backup_fd[i], 0, 0);
                if (d2 >= 0) {
                    write_str(2, "[twrp_fb_hook] pty slave open RECOVERED via backup ");
                    write_num(2, g_pty_backup_fd[i]);
                    write_str(2, " -> fd=");
                    write_num(2, (int)d2);
                    write_str(2, "\n");
                    return (int)d2;
                }
            }
        }
    }
    write_str(2, "[twrp_fb_hook] pty slave open /dev/pts/");
    write_num(2, n);
    write_str(2, " -> fd=");
    write_num(2, (int)d);
    write_str(2, "\n");
    if (d >= 0) pty_mark_slave_fd((int)d);
    return d >= 0 ? (int)d : -1;
}

/* ── 6-Z188i: the TTYS protocol on slave fds. Run 33129910056: the
 * slave open finally SUCCEEDED (fd=0) but the socket is not a tty —
 * isatty() fails and busybox ash runs NON-interactive (no prompt,
 * bare cursor). Answer TCGETS with a sane cooked termios (isatty==1
 * => interactive ash => prompt) and accept the TCSETS family. ── */
static long pty_slave_ioctl(int fd, unsigned req, unsigned long arg) {
    /* 6-Z188j: bitmap first (pre-exec processes), then the STATELESS
     * socket check (post-exec processes like the shell itself). */
    if (!pty_is_slave_fd(fd)
        && pty_slot_of_master(fd) < 0
        && !pty_fd_is_stream_socket(fd)) {
        return -2;
    }
    switch (req) {
        case 0x5401u: { /* TCGETS */
            if (arg) {
                unsigned int *t = (unsigned int *)(long)arg;
                t[0] = 0x500u;   /* c_iflag: ICRNL|IXON */
                t[1] = 0x5u;     /* c_oflag: OPOST|ONLCR */
                t[2] = 0xBFu;    /* c_cflag: B38400|CS8|CREAD */
                t[3] = 0x800Bu;  /* c_lflag: ISIG|ICANON|ECHO|IEXTEN */
                unsigned char *cc = (unsigned char *)(long)(arg + 16);
                for (int k = 0; k < 20; k++) cc[k] = 0;
                cc[6] = 1;       /* VMIN = 1 */
            }
            return 0;
        }
        case 0x5402u: case 0x5403u: case 0x5404u: /* TCSETS* — accept */
            return 0;
        case 0x5413u: { /* TIOCGWINSZ */
            if (arg) {
                unsigned short *ws = (unsigned short *)(long)arg;
                ws[0] = 24; ws[1] = 80; ws[2] = 80 * 8; ws[3] = 24 * 16;
            }
            return 0;
        }
        case 0x5414u:
            return 0;
        case 0x540Eu: /* TIOCSCTTY */
            return 0;
        default:
            return -2;
    }
}

/* ioctl on a tracked MASTER fd: the ptmx protocol. -2 = not ours. */
static long pty_master_ioctl(int fd, unsigned req, unsigned long arg) {
    int slot = pty_slot_of_master(fd);
    if (slot < 0) return -2;
    switch (req) {
        case 0x40045431u: /* TIOCSPTLCK — unlockpt */
            return 0;
        case 0x80045431u: /* TIOCGPTLCK */
            if (arg) *(int *)(long)arg = 0;
            return 0;
        case 0x80045430u: { /* TIOCGPTN — pty number: the SLAVE FD
                             * NUMBER (6-Z188f — matches ptsname). */
            int slot = pty_slot_of_master(fd);
            if (slot < 0) return -2;
            if (arg)
                *(unsigned int *)(long)arg =
                    (unsigned int)g_pty_slave_fd[slot];
            return 0;
        }
        case 0x540Eu:     /* TIOCSCTTY (asm-generic) */
            return 0;
        case 0x5413u: {   /* TIOCGWINSZ (asm-generic) */
            if (arg) {
                unsigned short *ws = (unsigned short *)(long)arg;
                ws[0] = 24; ws[1] = 80; ws[2] = 80 * 8; ws[3] = 24 * 16;
            }
            return 0;
        }
        case 0x5414u:     /* TIOCSWINSZ */
            return 0;
        default:
            return -2;
    }
}
#endif /* __aarch64__ */

// ---------------------------------------------------------------------------
// 6-Z165/6-Z166: absolute-open failure → rootfs-resolvable retry.
//
// Run 33002423676 (arm64 jail): the 6-Z164 tracer DIAG named the mechanism
// — read_child_string PEEK-fails on SOME child addresses (the dri/fb0 loop
// buffer 0xffffc35f6810, 11+ occurrences), so those openat ENTRYs ran
// UNTRANSLATED against the host: /dev/graphics/fb0 and /etc/recovery.fstab
// → host ENOENT → TWRP "Failing out of recovery due to problem with fstab".
// Meanwhile {rootfs}/dev/graphics/fb0 IS pre-created (3686400 bytes) and
// {rootfs}/etc/recovery.fstab exists — the files were always THERE.
//
// The retry uses TWO path forms that need NO tracer translation:
//   1. "{TWOYI_ROOTFS}{path}" — {rootfs} lives under /data/…, which the
//      tracer's translate_path PASSES THROUGH untranslated, so the
//      prefixed path is correct for the kernel in EVERY case.
//   2. path+1 (relative) — the guest's cwd IS the rootfs (fallback when
//      the TWOYI_ROOTFS env is unavailable to the weak getenv).
// ---------------------------------------------------------------------------

// Best jail-resolvable form for an absolute guest path: the
// {TWOYI_ROOTFS}-prefixed absolute path when the env is readable,
// otherwise path+1 (relative; cwd == rootfs). Returns NULL when the path
// is not absolute. The result may point at a STATIC BUFFER (overwritten
// per call) — callers must consume it before the next call.
//
// 6-Z187b: SECOND source — the {rootfs}/dev/.twoyi-rootfs file. Run
// 33119446980: the UI recovery is exec'd by INIT, whose service env does
// NOT carry TWOYI_ROOTFS (getenv returned NULL — the hook's via=1 prefix
// form was unavailable and via=2 failed 166x). kr64's parent writes the
// absolute rootfs path into /dev/.twoyi-rootfs at boot; the raw openat
// below uses the ABSOLUTE guest path "/dev/.twoyi-rootfs", which the
// tracer translates to {rootfs}/dev/.twoyi-rootfs in every mode (and
// resolves natively in chroot/pivot_root modes). The value is read ONCE
// and cached — the file lives on the pseudo-tmpfs {rootfs}/dev the
// tracer materialized, so it is stable for the process lifetime.
static char *rootfs_path_form(const char *path) {
    static char buf[512];
    static char root_cache[256];
    static int root_cache_state = 0; /* 0=untried, 1=loaded, 2=unavailable */
    if (!path || path[0] != '/') return NULL;
    const char *root = NULL;
    if (getenv) {
        root = getenv("TWOYI_ROOTFS");
    }
    if (!root || root[0] != '/') {
        /* 6-Z187b: file-based fallback (see the comment above). */
        if (root_cache_state == 0) {
            root_cache_state = 2;
            int rfd = (int)raw_syscall4(SYS_openat, AT_FDCWD,
                                        (long)"/dev/.twoyi-rootfs", 0 /*O_RDONLY*/, 0);
            if (rfd >= 0) {
                char rbuf[256];
                long n = raw_syscall3(SYS_read, rfd, (long)rbuf,
                                      (long)(sizeof(rbuf) - 1));
                raw_syscall1(SYS_close, rfd);
                if (n > 0) {
                    rbuf[n] = '\0';
                    /* strip trailing newline / whitespace */
                    long end = n;
                    while (end > 0 && (rbuf[end-1] == '\n' || rbuf[end-1] == '\r' ||
                                       rbuf[end-1] == ' ' || rbuf[end-1] == '\t'))
                        end--;
                    rbuf[end] = '\0';
                    if (end > 0 && rbuf[0] == '/' && end < (long)sizeof(root_cache)) {
                        for (long k = 0; k <= end; k++) root_cache[k] = rbuf[k];
                        root_cache_state = 1;
                    }
                }
            }
        }
        if (root_cache_state == 1)
            root = root_cache;
    }
    if (root && root[0] == '/') {
        int i = 0;
        while (root[i] && i < 400) { buf[i] = root[i]; i++; }
        int j = 0;
        while (path[j] && i < 510) { buf[i] = (char)path[j]; i++; j++; }
        buf[i] = '\0';
        return buf;
    }
    return (char *)(path + 1);
}

// Shared retry: ABSOLUTE open FAILED → (a) probe the same path once WITHOUT
// O_CREAT (no side effects) so the log shows the raw -errno, then (b) retry
// with the rootfs-resolvable form (prefix first, relative as fallback).
// Returns the working fd, or the original abs_fd when every attempt fails.
// Only raw returns are logged — no libc errno dependency (-nostdlib: the
// hook must not read/write the TLS errno slot).
static int rootfs_retry_open(const char *path, int abs_fd, int flags, int mode) {
    if (abs_fd >= 0 || !path || path[0] != '/') return abs_fd;
    long probe = raw_syscall4(SYS_openat, AT_FDCWD, (long)path,
                              flags & ~O_CREAT, 0);
    if (probe >= 0) {
        // Probe leaked an fd (the absolute path IS openable without
        // O_CREAT — e.g. an existing file after the create failed).
        raw_syscall1(SYS_close, (int)probe);
    }
    int rfd = -1;
    int via = 0;
    char *alt = rootfs_path_form(path);
    if (alt) {
        /* 6-Z187c: MARKED — the prefix form is a host-valid path built by
         * the hook; the tracer must not +1-corrupt it (it cannot read the
         * hook's static buffer — that is exactly how the via=1 retries
         * died in run 33120905168). */
        rfd = (int)raw_syscall4_marked(SYS_openat, AT_FDCWD, (long)alt, flags, mode);
        via = 1;
    }
    if (rfd < 0 && alt && alt != path + 1) {
        // Prefix form failed (or env missing) — last resort: relative.
        // 6-Z187c: also MARKED — path+1 is cwd-relative and the guest cwd
        // IS the rootfs (6-Z187b chdir); translation is a no-op for
        // relative paths, but the +1-of-+1 corruption must not happen.
        rfd = (int)raw_syscall4_marked(SYS_openat, AT_FDCWD, (long)(path + 1),
                                flags, mode);
        via = 2;
    }
    write_str(2, "[twrp_fb_hook] abs open fd=");
    write_num(2, abs_fd);
    write_str(2, " probe_raw=");
    write_num(2, (int)probe);
    write_str(2, " rootfs retry via=");
    write_num(2, via);
    write_str(2, " -> fd=");
    write_num(2, rfd);
    write_str(2, "\n");
    return rfd >= 0 ? rfd : abs_fd;
}

// ---------------------------------------------------------------------------
// INPUT BRIDGE (6-Z93) — host touch events -> guest evdev input_event.
//
// WHY: TWRP's minui is a plain evdev reader. It scans /dev/input for
// "event*" names, opens them, probes capabilities with EVIOCGBIT, then
// poll()+read()s 16-byte struct input_event records. The kr64 touch
// bridge socket ({rootfs}/dev/input/touch) speaks the Android EventHub
// protocol (896-byte DeviceInfo header + pre-encoded InputEvents), which
// minui cannot consume — and no guest ever connected to it anyway. The
// APP, however, binds {data_dir}/dev/touch-events and WRITES 20-byte LE
// TouchMessage records to every accepted client (Render2Activity.onTouch
// -> Renderer.handleTouch -> socket). So we become a second CLIENT of
// that socket and re-encode the records into the evdev stream minui
// expects.
//
// PROTOCOL (lib.rs TouchMessage, 20 bytes, little-endian):
//   offset 0  u32 action     (0=DOWN, 1=MOVE, 2=UP, 3=CANCEL)
//   offset 4  i32 pointer_id (slot; we implement single-finger: only id 0)
//   offset 8  i32 x          (pixels)
//   offset 12 i32 y          (pixels)
//   offset 16 i32 pressure   (0..255)
//
// TRACER INTERACTIONS (ptrace_emu.rs — these shape the design):
//   - 6-Z3: any FAILING i386 socketcall (nr=102) gets its return faked to
//     0. Our connect() failure would look like SUCCESS — so we VERIFY the
//     connection with a plain read(2) on the socket (read is NOT
//     socketcall, never faked): -EAGAIN = connected+idle, -ENOTCONN =
//     the connect actually failed.
//   - 6-Z5: any POSITIVE raw poll() return is faked to 0 + revents are
//     zeroed (and every poll costs the tracer a 10ms entry sleep). A real
//     poll syscall on our socket fd can therefore NEVER report readiness
//     to minui — our poll() interposition decides readability purely in
//     userspace (drain socket -> pending buffer -> revents=POLLIN).
//   - Guest paths (/dev/input/event0) are translated by the tracer to
//     {rootfs}/dev/input/event0 — which is why kr64 pre-creates the
//     probe targets in its devices phase (parent-side, no guest
//     syscalls), and why the connect fallback path is RELATIVE
//     ("../dev/touch-events"):
//     sockaddr_un paths are NOT translated, and the guest's cwd is the
//     rootfs (kr64 spawns the guest with cwd=working_dir; nothing chdirs
//     before recovery's minui runs).
// ---------------------------------------------------------------------------

// evdev constants (uapi linux/input.h + input-event-codes.h)
#define INBR_EV_SYN              0x00
#define INBR_EV_KEY              0x01
#define INBR_EV_ABS              0x03
#define INBR_SYN_REPORT          0x00
#define INBR_ABS_X               0x00
#define INBR_ABS_Y               0x01
#define INBR_ABS_MT_SLOT         47
#define INBR_ABS_MT_POSITION_X   53
#define INBR_ABS_MT_POSITION_Y   54
#define INBR_ABS_MT_TRACKING_ID  57
#define INBR_ABS_MT_PRESSURE     58
#define INBR_BTN_TOOL_FINGER     0x145  /* 325 */
#define INBR_BTN_TOUCH           0x14a  /* 330 */

// Virtual touch panel extents — RUNTIME (6-Z171b): must match the fb
// geometry (fb_w()-1 / fb_h()-1) so minui's EVIOCGABS abs_max and the
// incoming host coordinates live in the SAME native-resolution space as
// the rendered framebuffer. (Was a hardcoded 319/639.)
#define INBR_MAX_X               (fb_w() - 1)
#define INBR_MAX_Y               (fb_h() - 1)

// struct input_event layout is ARCH-DEPENDENT (6-Z171c fix):
//   i386:    timeval = 2×u32 (8B) + type u16 + code u16 + value s32 = 16B
//   aarch64: timeval = 2×s64 (16B) + type u16 + code u16 + value s32 = 24B
//   armv7:   timeval = 2×u32 (8B) — SAME 16B layout as i386 (6-Z227)
// minui read()s sizeof(struct input_event) per event — feeding an arm64
// child 16-byte i386 frames misaligns EVERY type/code/value (garbage
// events, no touches recognized). The header (timeval) is always zeroed
// by us, so the only layout difference that matters is the TOTAL size
// and the offset of type/code/value (always at INBR_EV_SIZE-8/-6/-4).
#if defined(__aarch64__)
  #define INBR_EV_SIZE           24
#elif defined(__arm__) && !defined(__aarch64__)
  #define INBR_EV_SIZE           16
#elif defined(__i386__)
  #define INBR_EV_SIZE           16
#else
  #error "twrp_fb_hook.c: no struct input_event layout for this arch"
#endif
#define INBR_MSG_SIZE            20

// Kernel errno values (arch-independent) — we build with -nostdlib and
// must not depend on the host errno.h having been included consistently.
#define INBR_EAGAIN              11
#define INBR_EINTR                4
#define INBR_RING_EVENTS         64     /* 64 * 16 = 1 KiB pending buffer */
#define INBR_MAX_SLOTS           4
#define INBR_DRAIN_BUF           1024

// Per-input-fd state. Only INBR_MAX_SLOTS fds get full state (minui opens
// one evdev fd per accepted /dev/input/eventN node — at most a handful).
struct inbr_slot {
    int fd;                     /* the socket fd; -1 = free slot */
    int connected;              /* 0 once the host socket EOFs/errors */
    int next_tid;               /* monotonic Type-B tracking-id counter */
    int active_tid;             /* 0 = finger up */
    unsigned ring_len;          /* synthesized input_event bytes pending */
    unsigned stage_len;         /* partial TouchMessage bytes buffered */
    unsigned dropped_msgs;
    unsigned char ring[INBR_RING_EVENTS * INBR_EV_SIZE];
    unsigned char stage[INBR_MSG_SIZE];
};

static struct inbr_slot g_inbr[INBR_MAX_SLOTS];
static unsigned char g_in_fds[(TWRP_FB_MAX_FD + 7) / 8];

// Terse logging: first N events per hook point, then silence.
static unsigned g_inbr_log_open;
static unsigned g_inbr_log_ioctl;
static unsigned g_inbr_log_read;
static unsigned g_inbr_log_poll;
static unsigned g_inbr_log_drop;

static void in_fd_mark(int fd) {
    if (fd < 0 || fd >= TWRP_FB_MAX_FD) return;
    g_in_fds[fd >> 3] |= (unsigned char)(1u << (fd & 7));
}

static int in_fd_is_tracked(int fd) {
    if (fd < 0 || fd >= TWRP_FB_MAX_FD) return 0;
    return (g_in_fds[fd >> 3] >> (fd & 7)) & 1;
}

static void in_fd_clear(int fd) {
    if (fd < 0 || fd >= TWRP_FB_MAX_FD) return;
    g_in_fds[fd >> 3] &= (unsigned char)~(1u << (fd & 7));
}

static struct inbr_slot *inbr_slot_for(int fd) {
    int i;
    if (!in_fd_is_tracked(fd)) return 0;
    for (i = 0; i < INBR_MAX_SLOTS; i++) {
        if (g_inbr[i].fd == fd) return &g_inbr[i];
    }
    return 0;
}

static unsigned inbr_ld32(const unsigned char *p) {
    return (unsigned)p[0] | ((unsigned)p[1] << 8) |
           ((unsigned)p[2] << 16) | ((unsigned)p[3] << 24);
}

// small memmove (overlapping, dst <= src) for ring compaction
static void inbr_shift(unsigned char *dst, const unsigned char *src, unsigned n) {
    unsigned i;
    for (i = 0; i < n; i++) dst[i] = src[i];
}

// Append one synthesized input_event to the ring. If the ring is full,
// drop the OLDEST events (the reader is polling at ~100Hz; losing stale
// motion beats losing a fresh DOWN/UP).
static void inbr_emit(struct inbr_slot *s, unsigned short type,
                      unsigned short code, int value) {
    unsigned char *p;
    if (s->ring_len + INBR_EV_SIZE > sizeof(s->ring)) {
        unsigned need = INBR_EV_SIZE;
        inbr_shift(s->ring, s->ring + need, sizeof(s->ring) - need);
        s->ring_len -= need;
        s->dropped_msgs++;
        if (g_inbr_log_drop < 4) {
            g_inbr_log_drop++;
            write_str(2, "[twrp_fb_hook] INPUT ring full — dropped oldest event (slow reader?)\n");
        }
    }
    p = s->ring + s->ring_len;
    /* timeval header: zeroed (minui ignores timestamps) — INBR_EV_SIZE-8
     * bytes: 8 on i386 (2×u32), 16 on aarch64 (2×s64). */
    {
        unsigned ti;
        for (ti = 0; ti < (unsigned)(INBR_EV_SIZE - 8); ti++) p[ti] = 0;
    }
    {
        unsigned char *q = s->ring + s->ring_len + (INBR_EV_SIZE - 8);
        q[0] = (unsigned char)(type & 0xff);
        q[1] = (unsigned char)((type >> 8) & 0xff);
        q[2] = (unsigned char)(code & 0xff);
        q[3] = (unsigned char)((code >> 8) & 0xff);
        q[4] = (unsigned char)((unsigned)value & 0xff);
        q[5] = (unsigned char)(((unsigned)value >> 8) & 0xff);
        q[6] = (unsigned char)(((unsigned)value >> 16) & 0xff);
        q[7] = (unsigned char)(((unsigned)value >> 24) & 0xff);
    }
    s->ring_len += INBR_EV_SIZE;
}

// Encode one 20-byte TouchMessage (in s->stage) into evdev events.
// Single-finger Type-B-ish + legacy ABS_X/Y, mirroring kr64's
// encode_touch_message() semantics (lib.rs) but in minui's format.
static void inbr_encode_msg(struct inbr_slot *s) {
    const unsigned char *p = s->stage;
    unsigned action = inbr_ld32(p + 0);
    int pointer_id = (int)inbr_ld32(p + 4);
    int x = (int)inbr_ld32(p + 8);
    int y = (int)inbr_ld32(p + 12);
    int pressure = (int)inbr_ld32(p + 16);

    if (pointer_id != 0) return;            /* single-finger: slot 0 only */
    if (x < 0) x = 0;
    if (x > INBR_MAX_X) x = INBR_MAX_X;
    if (y < 0) y = 0;
    if (y > INBR_MAX_Y) y = INBR_MAX_Y;

    switch (action) {
        case 0:  /* DOWN */
            s->next_tid++;
            if (s->next_tid <= 0) s->next_tid = 1;  /* wrap guard */
            s->active_tid = s->next_tid;
            inbr_emit(s, INBR_EV_ABS, INBR_ABS_MT_TRACKING_ID, s->active_tid);
            inbr_emit(s, INBR_EV_ABS, INBR_ABS_MT_POSITION_X, x);
            inbr_emit(s, INBR_EV_ABS, INBR_ABS_MT_POSITION_Y, y);
            inbr_emit(s, INBR_EV_ABS, INBR_ABS_MT_PRESSURE, pressure);
            inbr_emit(s, INBR_EV_KEY, INBR_BTN_TOUCH, 1);
            inbr_emit(s, INBR_EV_ABS, INBR_ABS_X, x);
            inbr_emit(s, INBR_EV_ABS, INBR_ABS_Y, y);
            inbr_emit(s, INBR_EV_SYN, INBR_SYN_REPORT, 0);
            break;
        case 1:  /* MOVE */
            if (!s->active_tid) return;     /* MOVE without DOWN — skip */
            inbr_emit(s, INBR_EV_ABS, INBR_ABS_MT_POSITION_X, x);
            inbr_emit(s, INBR_EV_ABS, INBR_ABS_MT_POSITION_Y, y);
            inbr_emit(s, INBR_EV_ABS, INBR_ABS_MT_PRESSURE, pressure);
            inbr_emit(s, INBR_EV_ABS, INBR_ABS_X, x);
            inbr_emit(s, INBR_EV_ABS, INBR_ABS_Y, y);
            inbr_emit(s, INBR_EV_SYN, INBR_SYN_REPORT, 0);
            break;
        case 2:  /* UP */
        case 3:  /* CANCEL */
            if (!s->active_tid) return;     /* UP without DOWN — skip */
            inbr_emit(s, INBR_EV_ABS, INBR_ABS_MT_TRACKING_ID, -1);
            inbr_emit(s, INBR_EV_KEY, INBR_BTN_TOUCH, 0);
            inbr_emit(s, INBR_EV_SYN, INBR_SYN_REPORT, 0);
            s->active_tid = 0;
            break;
        default:
            return;                          /* unknown action — drop */
    }
}

// Drain the (non-blocking) socket: parse 20-byte TouchMessages and
// synthesize events into the ring. Uses PLAIN read(2) — not socketcall —
// so the tracer's 6-Z3 socketcall fake-success can never mask a real
// error here. Returns bytes consumed from the socket.
static int inbr_drain(struct inbr_slot *s) {
    unsigned char buf[INBR_DRAIN_BUF];
    int total = 0;
    if (!s->connected) return 0;
    for (;;) {
        long r = raw_syscall3(SYS_read, s->fd, (long)buf, (long)sizeof(buf));
        if (r < 0) {
            if (r != -INBR_EAGAIN && r != -INBR_EINTR) {
                /* real error (-ENOTCONN / -EBADF / ...) — stop reporting */
                s->connected = 0;
            }
            break;
        }
        if (r == 0) {                        /* host socket closed (EOF) */
            s->connected = 0;
            break;
        }
        {
            long off = 0;
            while (off < r) {
                if (s->stage_len < INBR_MSG_SIZE) {
                    s->stage[s->stage_len++] = buf[off++];
                }
                if (s->stage_len == INBR_MSG_SIZE) {
                    inbr_encode_msg(s);
                    s->stage_len = 0;
                }
            }
        }
        total += (int)r;
        if (r < (long)sizeof(buf)) break;    /* kernel buffer drained */
    }
    return total;
}

// ---- raw Unix socket helpers (i386 socketcall / aarch64 direct) --------
#if defined(__i386__)
#define INBR_NR_SOCKETCALL 102   /* __NR_socketcall */
#define INBR_SUB_SOCKET    1
#define INBR_SUB_CONNECT   3
#endif

struct inbr_sockaddr_un {
    unsigned short sun_family;    /* AF_UNIX = 1 */
    char sun_path[108];
};

static long inbr_unix_socket(void) {
#if defined(__i386__)
    unsigned long a[3];
    a[0] = 1 /*AF_UNIX*/; a[1] = 1 /*SOCK_STREAM*/; a[2] = 0;
    return raw_syscall3(INBR_NR_SOCKETCALL, INBR_SUB_SOCKET, (long)a, 0);
#elif defined(__aarch64__)
    return raw_syscall3(SYS_socket, 1 /*AF_UNIX*/, 1 /*SOCK_STREAM*/, 0);
#endif
}

static long inbr_unix_connect(int fd, const struct inbr_sockaddr_un *addr,
                              unsigned addrlen) {
#if defined(__i386__)
    unsigned long a[3];
    a[0] = (unsigned long)fd; a[1] = (unsigned long)addr; a[2] = addrlen;
    return raw_syscall3(INBR_NR_SOCKETCALL, INBR_SUB_CONNECT, (long)a, 0);
#elif defined(__aarch64__)
    return raw_syscall3(SYS_connect, fd, (long)addr, (long)addrlen);
#endif
}

// Build "{root}/../dev/touch-events" or copy the relative fallback.
// Returns path length, or 0 if it does not fit sun_path (108 bytes; we
// keep a margin so 2 + len stays a valid sockaddr_un length).
static unsigned inbr_build_path(char *out, unsigned outsz, const char *suffix_root) {
    const char *tail = "/../dev/touch-events";
    unsigned n = my_strlen(suffix_root);
    unsigned t = my_strlen(tail);
    if (n == 0 || n + t > 100) return 0;
    my_memset(out, 0, outsz);
    {
        unsigned i;
        for (i = 0; i < n; i++) out[i] = suffix_root[i];
        for (i = 0; i < t; i++) out[n + i] = tail[i];
    }
    return n + t;
}

// Connect to the host touch-events socket. Returns the connected socket
// fd (non-blocking) or -1. On success any initial pending bytes are
// staged into the returned slot.
static int inbr_connect(struct inbr_slot **out_slot, unsigned char *init_bytes,
                        unsigned *init_len) {
    struct inbr_sockaddr_un addr;
    char rootbuf[128];
    char relbuf[24];
    char absbuf[24];
    char *cands[4];
    /* 6-Z180 FIX: cand_lens was [2] while multiple candidates can be
     * staged (abstract + /dev/.touch-sock + $TWOYI_ROOTFS + relative).
     * With all present, `cand_lens[ncands]` wrote PAST the array onto
     * this function's stack — a deterministic stack corruption inside
     * the exact window every arm64 crash run died in (runs
     * 33021261552/33021972679: socket -> fcntl -> fcntl -> SIGSEGV with
     * a garbage sp). The arrays now hold all four candidates and every
     * insert is bounds-guarded. */
    unsigned cand_lens[4];
    /* 6-Z182: abstract-name lengths — cand_lens[i]==0 marks candidate i
     * as ABSTRACT (name bytes in cands[i], length here). */
    unsigned cand_name_lens[4];
    int ncands = 0;
    long fd, i;

    (void)my_memset(cand_lens, 0, sizeof(cand_lens));
    (void)my_memset(cand_name_lens, 0, sizeof(cand_name_lens));
    (void)my_memset(cands, 0, sizeof(cands));

    *init_len = 0;
    *out_slot = 0;

    /* 6-Z182 candidate 0 (ABSTRACT, tried FIRST): \0io.twoyi.touch —
     * the chroot-proof listener the app binds (input.rs
     * touch_server_abstract). Abstract names bypass the filesystem
     * entirely, so the jail's root cannot hide it. */
    {
        static const char abs_name[] = "io.twoyi.touch";
        unsigned n = (unsigned)(sizeof(abs_name) - 1); /* 14, no NUL */
        unsigned j;
        my_memset(absbuf, 0, sizeof(absbuf));
        for (j = 0; j < n && j < sizeof(absbuf) - 1; j++) absbuf[j] = abs_name[j];
        cands[ncands] = absbuf; cand_lens[ncands] = 0; cand_name_lens[ncands] = n; ncands++;
    }

    /* 6-Z96b candidate 0: /dev/.touch-sock — a file kr64 writes with the
     * ABSOLUTE host path of the touch socket. Reading it via openat is
     * intercepted + translated by the tracer to the real file, so this
     * needs NO getenv (whose weak PLT the recovery binary's ancient
     * bionic linker leaves unresolved — run 32654424163 tried only the
     * relative candidate and ENOENT'd). */
    {
        int tf = (int)raw_syscall4(SYS_openat, -100 /*AT_FDCWD*/,
                                   (long)"/dev/.touch-sock", 0 /*O_RDONLY*/, 0);
        if (tf >= 0) {
            char pbuf[136];
            long pr = raw_syscall3(SYS_read, tf, (long)pbuf, (long)(sizeof(pbuf) - 1));
            (void)raw_syscall1(SYS_close, tf);
            /* 6-Z184 AUDIT FIX (agent 2): k must fit sun_path[108]
             * INCLUDING the NUL (<=107), else the later copy truncates
             * silently and inbr_unix_connect is handed an addrlen past
             * the 110-byte sockaddr_un (kernel OOB read). Mirror
             * inbr_build_path's conservative cap. */
            if (pr > 0 && pr < 128) {
                long k;
                pbuf[pr] = 0;
                for (k = pr - 1; k >= 0; k--) {
                    if (pbuf[k] == '\n' || pbuf[k] == '\r' || pbuf[k] == ' ') pbuf[k] = 0;
                    else break;
                }
                for (k = 0; pbuf[k]; k++) rootbuf[k] = pbuf[k];
                rootbuf[k] = 0;
                if (k > 0 && k <= 105 && rootbuf[0] == '/' && ncands < 4) {
                    cands[ncands] = rootbuf; cand_lens[ncands] = (unsigned)k; ncands++;
                }
            }
        }
    }
    /* candidate 1: $TWOYI_ROOTFS/../dev/touch-events (absolute host path) */
    if (getenv) {
        const char *root = getenv("TWOYI_ROOTFS");
        if (root && root[0] && my_strcmp(root, "/") != 0) {
            unsigned n = inbr_build_path(rootbuf, sizeof(rootbuf), root);
            if (n > 0 && ncands < 4) { cands[ncands] = rootbuf; cand_lens[ncands] = n; ncands++; }
        }
    }
    /* candidate 1: relative ../dev/touch-events (guest cwd == rootfs) */
    {
        const char *rel = "../dev/touch-events";
        unsigned n = my_strlen(rel);
        unsigned j;
        my_memset(relbuf, 0, sizeof(relbuf));
        for (j = 0; j < n; j++) relbuf[j] = rel[j];
        if (ncands < 4) { cands[ncands] = relbuf; cand_lens[ncands] = n; ncands++; }
    }
    if (ncands == 0) return -1; /* unreachable (relative insert is
                                 * unconditional) — kept for clarity */

    for (i = 0; i < ncands; i++) {
        long v;
        unsigned char peek[64];
        /* FRESH socket per candidate: after a failed connect() the socket
         * state is unspecified (POSIX) — do not reuse the fd. */
        fd = inbr_unix_socket();
        if (fd < 0) return -1;
        {
            long fl = raw_syscall3(SYS_fcntl, fd, 3 /*F_GETFL*/, 0);
            if (fl >= 0) {
                (void)raw_syscall3(SYS_fcntl, fd, 4 /*F_SETFL*/, fl | 0x800);
            }
        }
        my_memset(&addr, 0, sizeof(addr));
        addr.sun_family = 1 /*AF_UNIX*/;
        {
            unsigned j;
            if (cand_lens[i] == 0) {
                /* 6-Z182: ABSTRACT candidate (cand_lens == 0 marks it).
                 * sun_path[0] stays NUL (abstract marker); the name bytes
                 * were pre-staged into cands[i]. Abstract AF_UNIX names
                 * resolve in the NETWORK namespace — immune to the jail's
                 * chroot/pivot_root (run 33061152563: every filesystem
                 * candidate ENOENT'd INSIDE the jail because the absolute
                 * host path does not exist under the rootfs). The app's
                 * touch_server_abstract() binds \0io.twoyi.touch and
                 * feeds the same 20-byte TouchMessage stream. */
                addr.sun_path[0] = '\0';
                for (j = 0; j < cand_name_lens[i] && j < 105; j++) {
                    addr.sun_path[1 + j] = cands[i][j];
                }
                (void)inbr_unix_connect((int)fd, &addr, 2 + 1 + cand_name_lens[i]);
            } else {
                for (j = 0; j < cand_lens[i] && j < 107; j++) {
                    addr.sun_path[j] = cands[i][j];
                }
                (void)inbr_unix_connect((int)fd, &addr, 2 + cand_lens[i]);
            }
        }
        /* 6-Z3: a REAL connect failure was faked to 0 by the tracer —
         * verify with plain read(2) (never faked):
         *   >0        connected, initial data pending (stage it)
         *   -EAGAIN   connected + idle  -> GOOD
         *   -EINTR    connected + idle  -> GOOD
         *   anything else (e.g. -ENOTCONN/-ENOENT) -> connect failed   */
        v = raw_syscall3(SYS_read, fd, (long)peek, (long)sizeof(peek));
        if (v == -INBR_EAGAIN || v == -INBR_EINTR) {
            return (int)fd;
        }
        if (v > 0) {
            unsigned j;
            for (j = 0; j < (unsigned)v && *init_len < sizeof(peek); j++) {
                init_bytes[(*init_len)++] = peek[j];
            }
            return (int)fd;
        }
        /* connect verification failed on this candidate — close and try
         * the next path */
        if (g_inbr_log_open < 8) {
            g_inbr_log_open++;
            write_str(2, "[twrp_fb_hook] INPUT bridge: connect candidate FAILED: ");
            write_str(2, cands[i] != 0 ? cands[i] : "(null)");
            write_str(2, " (read verify=");
            write_num(2, (int)v);
            write_str(2, ")\n");
        }
        (void)raw_syscall1(SYS_close, fd);
        fd = -1;
    }

    if (g_inbr_log_open < 8) {
        g_inbr_log_open++;
        write_str(2, "[twrp_fb_hook] INPUT bridge: ALL candidates failed (ncands=");
        write_num(2, ncands);
        write_str(2, ") — TWOYI_ROOTFS env set? ");
        write_str(2, (getenv && getenv("TWOYI_ROOTFS")) ? "yes" : "NO");
        write_str(2, "\n");
    }

    return -1;
}

// Returns 1 if path is one of the evdev nodes we bridge.
static int is_input_path(const char *path) {
    if (!path) return 0;
    if (my_strcmp(path, "/dev/input/event0") == 0) return 1;
    if (my_strcmp(path, "/dev/input/event1") == 0) return 1;
    if (my_strcmp(path, "/dev/input/event2") == 0) return 1;
    if (my_strcmp(path, "/dev/input/touch") == 0) return 1;
    // Run 32651067502: minui opens the evdev node RELATIVE via
    // openat(dirfd, "event0") — the absolute matches above never fired,
    // the hook handed back the pre-created probe FILE (fd=4) and the
    // bridge never connected. Also match the bare base names (minui
    // chdir's to /dev/input or passes the last component).
    if (my_strcmp(path, "event0") == 0) return 1;
    if (my_strcmp(path, "event1") == 0) return 1;
    if (my_strcmp(path, "event2") == 0) return 1;
    if (my_strcmp(path, "touch") == 0) return 1;
    return 0;
}

// 6-Z185: the 6-Z180 no-input gate was REMOVED. It was an A/B
// diagnostic probe (disable the input bridge when a marker file
// exists) for the input-tail SIGSEGV; the user vetoed input gating
// ("fix the actual problem") and the real fix shipped separately
// (the 6-Z183 window-tracking + RGBA path). Touch stays ALWAYS ON.

// Attempt the input bridge for this open(). Returns the fd to hand to the
// caller, or -2 to fall through to a real open().
static int try_open_input_bridge(const char *path) {
    unsigned char init_bytes[64];
    unsigned init_len = 0;
    struct inbr_slot *slot = 0;
    int fd;
    int i;

    if (!is_input_path(path)) return -2;

    fd = inbr_connect(&slot, init_bytes, &init_len);
    if (fd < 0) {
        if (g_inbr_log_open < 8) {
            g_inbr_log_open++;
            write_str(2, "[twrp_fb_hook] INPUT bridge connect FAILED for \"");
            write_str(2, path);
            write_str(2, "\" — falling back to real open\n");
        }
        return -2;
    }

    /* claim a state slot */
    slot = 0;
    for (i = 0; i < INBR_MAX_SLOTS; i++) {
        if (g_inbr[i].fd < 0) { slot = &g_inbr[i]; break; }
    }
    if (!slot) {
        /* too many input fds (minui never does this) — keep the socket
         * as a plain untracked fd */
        if (g_inbr_log_open < 8) {
            g_inbr_log_open++;
            write_str(2, "[twrp_fb_hook] INPUT bridge: no free slot — fd left untracked\n");
        }
        return fd;
    }
    my_memset(slot, 0, sizeof(*slot));
    slot->fd = fd;
    slot->connected = 1;
    slot->next_tid = 0;
    {
        unsigned j;
        for (j = 0; j < init_len; j++) {
            if (slot->stage_len < INBR_MSG_SIZE) {
                slot->stage[slot->stage_len++] = init_bytes[j];
            }
            if (slot->stage_len == INBR_MSG_SIZE) {
                inbr_encode_msg(slot);
                slot->stage_len = 0;
            }
        }
    }
    in_fd_mark(fd);
    if (g_inbr_log_open < 8) {
        g_inbr_log_open++;
        write_str(2, "[twrp_fb_hook] INPUT bridge: open(\"");
        write_str(2, path);
        write_str(2, "\") -> touch-events socket fd=");
        write_num(2, fd);
        write_str(2, "\n");
    }
    return fd;
}

// ---------------------------------------------------------------------------
// read() interposition — synthesized evdev stream for input fds, real
// read for everything else. The pass-through uses the dlsym'd libc read
// when available (errno handled by libc); the raw-syscall fallback returns
// the raw kernel value like the existing open()/ioctl() pass-throughs do
// (callers check n < 0, which still works; errno is left stale — same
// trade-off the existing hooks already make under -nostdlib).
// ---------------------------------------------------------------------------
ssize_t read(int fd, void *buf, size_t count) {
    struct inbr_slot *s = inbr_slot_for(fd);
    static ssize_t (*real_read)(int, void *, size_t) = NULL;
    long n;

    if (!s) {
        if (!real_read && dlsym) {
            real_read = (ssize_t (*)(int, void *, size_t))hook_de_self(
                (void *)dlsym(RTLD_NEXT, "read"), "read"); /* 6-Z246 */
        }
        if (real_read) return real_read(fd, buf, count);
        return (ssize_t)raw_syscall3(SYS_read, fd, (long)buf, (long)count);
    }

    /* input fd: serve complete 16-byte events from the ring */
    if (s->ring_len == 0) inbr_drain(s);
    if (s->ring_len == 0) {
        if (g_inbr_log_read < 8) {
            g_inbr_log_read++;
            write_str(2, "[twrp_fb_hook] INPUT read(fd=");
            write_num(2, fd);
            write_str(2, ") with empty ring (spurious poll) -> EAGAIN\n");
        }
        return (ssize_t)-INBR_EAGAIN;   /* raw-style -errno, like open() */
    }
    n = (long)count;
    if (n > (long)s->ring_len) n = (long)s->ring_len;
    n = n / INBR_EV_SIZE * INBR_EV_SIZE;    /* complete events only */
    if (n <= 0) return (ssize_t)-22 /* -EINVAL: buffer smaller than 1 event */;
    {
        long i;
        unsigned char *dst = (unsigned char *)buf;
        for (i = 0; i < n; i++) dst[i] = s->ring[i];
    }
    inbr_shift(s->ring, s->ring + n, s->ring_len - (unsigned)n);
    s->ring_len -= (unsigned)n;
    if (g_inbr_log_read < 8) {
        g_inbr_log_read++;
        write_str(2, "[twrp_fb_hook] INPUT read(fd=");
        write_num(2, fd);
        write_str(2, ") -> ");
        write_num(2, (int)n);
        write_str(2, " bytes of input_events\n");
    }
    return (ssize_t)n;
}

// ---------------------------------------------------------------------------
// poll() interposition.
//
// If NO input fd is in the set: pass through unchanged (same syscall the
// caller would have made anyway — the tracer's 6-Z5 entry-sleep + fake
// apply exactly as before, so behaviour for every other poll is IDENTICAL
// to the pre-hook state).
//
// If an input fd IS in the set: decide readability purely in userspace —
// drain the socket, then report POLLIN iff synthesized events are
// pending. NEVER issue the raw poll syscall in this case: the tracer
// fakes positive poll returns to 0 and zeroes revents (6-Z5), so minui
// could never see the readiness through the kernel. revents of the OTHER
// fds in the set are zeroed while an input fd is present (their events
// are currently invisible to the guest anyway via 6-Z5; minui re-polls
// at ~100Hz).
// ---------------------------------------------------------------------------
struct pollfd {
    int fd;
    short events;
    short revents;
};

#define INBR_POLLIN   0x001
#define INBR_SLICE_MS 15

int poll(struct pollfd *fds, unsigned long nfds, int timeout_ms) {
    static int (*real_poll)(struct pollfd *, unsigned long, int) = NULL;
    unsigned long i;
    int has_input = 0;
    unsigned waited = 0;

    for (i = 0; i < nfds; i++) {
        if (in_fd_is_tracked(fds[i].fd)) { has_input = 1; break; }
    }
    if (!has_input) {
        if (!real_poll && dlsym) {
            real_poll = (int (*)(struct pollfd *, unsigned long, int))hook_de_self(
                (void *)dlsym(RTLD_NEXT, "poll"), "poll"); /* 6-Z246 */
        }
        if (real_poll) return real_poll(fds, nfds, timeout_ms);
#if defined(__i386__)
        return (int)raw_syscall3(SYS_poll, (long)fds, (long)nfds, timeout_ms);
#else
        // aarch64 has no poll syscall (ppoll only) — and the input bridge
        // is i386-only (TWRP). On other arches without a real_poll symbol
        // report a plain timeout; harmless for the fb-only build.
        (void)fds; (void)nfds;
        return 0;
#endif
    }

    for (;;) {
        int ready = 0;
        for (i = 0; i < nfds; i++) fds[i].revents = 0;
        for (i = 0; i < nfds; i++) {
            struct inbr_slot *s = inbr_slot_for(fds[i].fd);
            if (!s) continue;
            (void)inbr_drain(s);
            if (s->ring_len > 0) {
                fds[i].revents = (short)INBR_POLLIN;
                ready++;
            }
            /* disconnected socket: report NOTHING (device goes silent,
             * same as "no events") — reporting POLLERR would just make
             * minui busy-read -EAGAIN forever. */
        }
        if (ready) {
            /* 6-Z289g: heartbeat — the first-8 cap made the poll state
             * invisible exactly when it mattered (runs 33914…33921: the
             * guest stops reading input after ~+34 s and the artifacts
             * could not tell a dead poll loop from a starved one). Log
             * the first 8 AND every 256th delivery thereafter. */
            g_inbr_log_poll++;
            if (g_inbr_log_poll <= 8 || (g_inbr_log_poll % 256) == 0) {
                write_str(2, "[twrp_fb_hook] INPUT poll -> ");
                write_num(2, ready);
                write_str(2, " ready (userspace, no raw poll syscall) #");
                write_num(2, g_inbr_log_poll);
                write_str(2, "\n");
            }
            return ready;
        }
        if (timeout_ms == 0) return 0;
        {
            /* 15ms slice sleep — keeps input latency low without any
             * kernel poll (which the tracer would fake + throttle). */
            long ts[4]; /* s32 sec, s32 nsec, s32 rem_sec, s32 rem_nsec */
            ts[0] = 0; ts[1] = INBR_SLICE_MS * 1000L * 1000L;
            ts[2] = 0; ts[3] = 0;
            (void)raw_syscall3(SYS_nanosleep, (long)ts, (long)(ts + 2), 0);
        }
        waited += INBR_SLICE_MS;
        if (timeout_ms > 0 && waited >= (unsigned)timeout_ms) return 0;
        /* timeout_ms < 0: loop until input arrives */
    }
}

// bionic's FORTIFIED variants (selected when the caller is compiled with
// -D_FORTIFY_SOURCE — which AOSP platform code, i.e. minui, IS). The
// existing hook already interposes __open_2/__openat_2 for the same
// reason; these two cover the input path:
//   ssize_t __read_chk(int fd, void* buf, size_t count, size_t buf_size);
//   int __poll_chk(struct pollfd* fds, nfds_t nfds, int timeout,
//                  size_t fds_size);
// The extra size argument is only for the fortify bounds-check, which our
// own buffers already respect — ignore it and forward. DEFINING these in
// the .so is free (no DT_NEEDED resolution involved).
ssize_t __read_chk(int fd, void *buf, size_t count, size_t buf_size) {
    (void)buf_size;
    return read(fd, buf, count);
}

int __poll_chk(struct pollfd *fds, unsigned long nfds, int timeout_ms,
               size_t fds_size) {
    (void)fds_size;
    return poll(fds, nfds, timeout_ms);
}

// ---------------------------------------------------------------------------
// ioctl() handling for INPUT fds — fake the evdev capability probe so
// minui's ev_init accepts the device:
//   EVIOCGBIT(0, len)            -> EV_SYN|EV_KEY|EV_ABS type bits
//   EVIOCGBIT(EV_KEY, len)       -> BTN_TOUCH|BTN_TOOL_FINGER
//   EVIOCGBIT(EV_ABS, len)       -> ABS_X/Y + ABS_MT_SLOT/POSITION_X/Y/
//                                   TRACKING_ID/PRESSURE
//   EVIOCGVERSION                -> 0x010001
//   EVIOCGID                     -> zeroed input_id
//   EVIOCGNAME/EVIOCGPHYS/EVIOCGUNIQ -> "twoyi_virtual_touch"
//   EVIOCGABS(abs)               -> input_absinfo (0..319 / 0..639)
// Anything else is passed through (a socket fd returns ENOTTY, which
// minui ignores for ioctls it doesn't care about).
// ---------------------------------------------------------------------------
static void inbr_set_bit(unsigned char *bm, unsigned cap, unsigned code) {
    if (code / 8 < cap) bm[code / 8] |= (unsigned char)(1u << (code & 7));
}

static int input_ioctl(int fd, unsigned req, void *argp) {
    unsigned type = (req >> 8) & 0xffu;
    unsigned nr = req & 0xffu;
    unsigned size = (req >> 16) & 0x3fffu;

    (void)fd;
    if (type != 0x45u /* 'E' */ || !argp) return -2;   /* not ours */

    if (nr == 0x01u && size >= 4) {              /* EVIOCGVERSION */
        *(int *)argp = 0x010001;
        return 0;
    }
    if (nr == 0x02u && size >= 8) {              /* EVIOCGID */
        my_memset(argp, 0, 8);
        return 0;
    }
    if (nr == 0x06u || nr == 0x07u || nr == 0x08u) { /* EVIOCGNAME/PHYS/UNIQ */
        static const char name[] = "twoyi_virtual_touch";
        unsigned i;
        unsigned cap = size < sizeof(name) ? size : sizeof(name);
        if (size > 256) return -2;               /* implausible size */
        my_memset(argp, 0, size);
        for (i = 0; i + 1 < cap; i++) ((char *)argp)[i] = name[i];
        return 0;
    }
    if (nr >= 0x20u && nr <= 0x3fu) {            /* EVIOCGBIT(ev, len) */
        unsigned ev = nr - 0x20u;
        unsigned cap = size;
        unsigned need = 1;                       /* kernel bitmap size for this ev */
        if (cap > 64) cap = 64;                  /* our bitmap budget */
        if (size > 4096) return -2;
        /* Kernel-faithful WRITE BOUND (task 6-Z284): the kernel's
         * bits_to_user() writes at most (maxbit+7)/8 bytes for the
         * requested bitmap, whatever len the caller passed. New-gen minui
         * (AOSP 12+ / LOS 20+) calls EVIOCGBIT(EV_KEY, KEY_MAX) with a
         * 96-byte stack buffer — ev_iterate_available_keys passes
         * len=KEY_MAX=767 but allocates only BITS_TO_LONGS(KEY_MAX)*8=96
         * bytes. Memsetting the full len zeroed ~575 bytes of the
         * caller's stack ABOVE the buffer, which nulled the std::function
         * object living in RecoveryUI::Init's frame — its __f_ pointer
         * then read as NULL on the first key_detected() callback ->
         * "__throw_bad_function_call" -> abort() -> recovery parked
         * BEFORE UI init (lineage-22.2 SINGLE_FRAME, ui=NOT_REACHED).
         * TWRP-era callers pass len=sizeof(bits)=96 and never hit it. */
        if (ev == 0u) {                          /* EV_SYN bitmap: EV_MAX=0x1f */
            need = (0x1fu + 7) / 8;              /* = 4  */
        } else if (ev == INBR_EV_KEY) {
            need = (0x2ffu /*KEY_MAX*/ + 7) / 8; /* = 96 */
        } else if (ev == INBR_EV_ABS) {
            need = (0x3fu /*ABS_MAX*/ + 7) / 8;  /* = 8  */
        } else if (ev == 2u) {                   /* EV_REL: REL_MAX=0x0f */
            need = (0x0fu + 7) / 8;              /* = 2  */
        } else if (ev == 4u) {                   /* EV_MSC: MSC_MAX=0x07 */
            need = (0x07u + 7) / 8;              /* = 1  */
        } else if (ev == 5u) {                   /* EV_SW: SW_MAX=0x07 */
            need = (0x07u + 7) / 8;              /* = 1  */
        } else if (ev == 0x11u) {                /* EV_LED: LED_MAX=0x0f */
            need = (0x0fu + 7) / 8;              /* = 2  */
        } else if (ev == 0x12u) {                /* EV_SND: SND_MAX=0x07 */
            need = (0x07u + 7) / 8;              /* = 1  */
        } else if (ev == 0x14u) {                /* EV_REP: REP_MAX=0x01 */
            need = (0x01u + 7) / 8;              /* = 1  */
        } else if (ev == 0x15u) {                /* EV_FF: FF_MAX=0x7f */
            need = (0x7fu + 7) / 8;              /* = 16 */
        }
        if (need > size) need = size;            /* kernel: min(len, bitmap) */
        my_memset(argp, 0, need);                /* touch ONLY what the kernel would */
        if (ev == 0) {                           /* event-type bits */
            inbr_set_bit((unsigned char *)argp, cap, INBR_EV_SYN);
            inbr_set_bit((unsigned char *)argp, cap, INBR_EV_KEY);
            inbr_set_bit((unsigned char *)argp, cap, INBR_EV_ABS);
        } else if (ev == INBR_EV_KEY) {
            inbr_set_bit((unsigned char *)argp, cap, INBR_BTN_TOUCH);
            inbr_set_bit((unsigned char *)argp, cap, INBR_BTN_TOOL_FINGER);
        } else if (ev == INBR_EV_ABS) {
            inbr_set_bit((unsigned char *)argp, cap, INBR_ABS_X);
            inbr_set_bit((unsigned char *)argp, cap, INBR_ABS_Y);
            inbr_set_bit((unsigned char *)argp, cap, INBR_ABS_MT_SLOT);
            inbr_set_bit((unsigned char *)argp, cap, INBR_ABS_MT_POSITION_X);
            inbr_set_bit((unsigned char *)argp, cap, INBR_ABS_MT_POSITION_Y);
            inbr_set_bit((unsigned char *)argp, cap, INBR_ABS_MT_TRACKING_ID);
            inbr_set_bit((unsigned char *)argp, cap, INBR_ABS_MT_PRESSURE);
        }
        /* Kernel-faithful: EVIOCGBIT returns the number of bytes copied
         * (min(len, bits-map size)), not 0 — callers that check > 0 (the
         * documented kernel contract) must see a positive count. */
        return (int)need;
    }
    if (nr >= 0x40u && nr <= 0x7fu && size >= 24) { /* EVIOCGABS(abs) */
        unsigned abs = nr - 0x40u;
        int *v = (int *)argp;                    /* 6 x s32 */
        v[0] = 0;                                /* value */
        v[1] = 0;                                /* minimum */
        v[2] = 255;                              /* maximum */
        v[3] = 0;                                /* fuzz */
        v[4] = 0;                                /* flat */
        v[5] = 0;                                /* resolution */
        if (abs == INBR_ABS_X || abs == INBR_ABS_MT_POSITION_X) v[2] = INBR_MAX_X;
        if (abs == INBR_ABS_Y || abs == INBR_ABS_MT_POSITION_Y) v[2] = INBR_MAX_Y;
        if (abs == INBR_ABS_MT_TRACKING_ID) { v[1] = -1; v[2] = 0x7fffffff; }
        if (abs == INBR_ABS_MT_SLOT) v[2] = 9;
        return 0;
    }
    return -2;                                    /* unknown — pass through */
}

// ---------------------------------------------------------------------------
// Virtual screen configuration.
//
// 320x640 @ 32bpp (BGRA8888 in memory). MUST MATCH the E2E profile's
// virtual display dims AND kr64's devices::create_twrp_framebuffer()
// file size (320*640*4 = 819200) AND the app reader's read size
// (core.rs twrp_fb_render_loop reads virtual_width*virtual_height*4).
// The OLD 720x1280/3686400 hardcode ftruncated the fb0 file to 3686400
// while the app still read only the first 819200 bytes = the top 320
// rows of a 720-wide frame, reinterpreted as a 320x640 image → garbage
// (Task 6-Z64: found by Agent E, Wave 1).
//
// libminuitwrp reads these values from FBIOGET_VSCREENINFO and uses
// them to size its framebuffer. Channel offsets (red=16/green=8/blue=0)
// declare in-memory [B,G,R,A] — exactly what the real byt_t_crv2 Bay
// Trail device reports and what this TWRP image was built for; the app
// side swaps R/B when blitting (core.rs, Task 6-Z64).
// ---------------------------------------------------------------------------
#define TWRP_FB_ACTIVATE_NOW   0
// FB_TYPE_PACKED_PIXELS = 0
#define TWRP_FB_TYPE_PACKED    0
// FB_VISUAL_TRUECOLOR = 2
#define TWRP_FB_VISUAL_TRUECOLOR 2

static void fill_vscreeninfo(struct fb_var_screeninfo *v) {
    my_memset(v, 0, sizeof(*v));
    fb_geometry_init();
    v->xres = (__u32)g_fb_rt_w;
    v->yres = (__u32)g_fb_rt_h;
    v->xres_virtual = (__u32)g_fb_rt_w;
    v->yres_virtual = (__u32)g_fb_rt_h;
    v->xoffset = 0;
    v->yoffset = 0;
    v->bits_per_pixel = TWRP_FB_BPP;
    v->grayscale = 0;
    // RGBA8888: red at bit 16, green at bit 8, blue at bit 0, alpha at bit 24
    v->red.offset = 16;    v->red.length = 8;    v->red.msb_right = 0;
    v->green.offset = 8;   v->green.length = 8;  v->green.msb_right = 0;
    v->blue.offset = 0;    v->blue.length = 8;   v->blue.msb_right = 0;
    v->transp.offset = 24; v->transp.length = 8; v->transp.msb_right = 0;
    v->nonstd = 0;
    v->activate = TWRP_FB_ACTIVATE_NOW;
    // Physical dimensions in mm (for DPI calculation). 320x640 at
    // ~250 DPI is ~32x65mm — a phone-sized portrait panel.
    v->height = 65;
    v->width = 32;
    v->accel_flags = 0;
    // Pixclock in picoseconds. For 60Hz refresh of 320x640:
    //   pixclock = 1 / (60 * 320 * 640) = ~82ns = ~82000ps
    // libminuitwrp doesn't use this for the software renderer, but we
    // provide a sane value anyway.
    v->pixclock = 82000;
    v->left_margin = 24;
    v->right_margin = 24;
    v->upper_margin = 4;
    v->lower_margin = 4;
    v->hsync_len = 24;
    v->vsync_len = 4;
    v->sync = 0;
    v->vmode = 0;  // FB_VMODE_NONINTERLACED = 0
    v->rotate = 0;
    v->colorspace = 0;  // FB_COLORSPACE_RGB
}

static void fill_fscreeninfo(struct fb_fix_screeninfo *f) {
    my_memset(f, 0, sizeof(*f));
    // id is a 16-byte char array (kernel: char id[16]).
    // Use a short null-terminated string; the rest stays zeroed.
    // We avoid strncpy() here because it would generate a PLT call to
    // libc's strncpy (we build with -nostdlib), which bionic's old
    // linker can't resolve cleanly. Manual byte copy is safe because
    // my_memset already zeroed all 16 bytes of f->id above.
    {
        static const char id_str[] = "twoyi_fb";
        unsigned int i;
        for (i = 0; i < (unsigned int)sizeof(f->id) - 1 && id_str[i]; i++) {
            f->id[i] = id_str[i];
        }
    }
    // smem_start is an unsigned long — on i386 it's 4 bytes. We set it
    // to a non-zero placeholder (libminuitwrp doesn't dereference it;
    // it only uses smem_len for the mmap size).
    f->smem_start = 0;
    f->smem_len = (__u32)fb_smem_len();
    f->type = TWRP_FB_TYPE_PACKED;
    f->type_aux = 0;
    f->visual = TWRP_FB_VISUAL_TRUECOLOR;
    f->xpanstep = 0;
    f->ypanstep = 0;
    f->ywrapstep = 0;
    f->line_length = (__u32)fb_line_length();
    f->mmio_start = 0;
    f->mmio_len = 0;
    f->accel = 0;
    f->capabilities = 0;
    f->reserved[0] = 0;
    f->reserved[1] = 0;
}

// ---------------------------------------------------------------------------
// Constructor (.init_array) — runs when the LD_PRELOAD library is loaded
// by the bionic linker, BEFORE recovery's main() starts. Logs that we
// loaded so we can verify in the KVM logs that LD_PRELOAD is working.
//
// DIAGNOSTIC (Task 31, KVM run 31578527978): the previous run showed that
// our hook's constructor runs ([twrp_fb_hook] loaded appears 16×) but
// NONE of our open/ioctl/close hooks are ever called by recovery — the
// "tracking for FB ioctls" message NEVER appears, and recovery segfaults
// at "I:Checking resolution..." because FBIOGET_VSCREENINFO returns
// ENOTTY (fb0 is a regular file) and libminuitwrp derefs the zeroed vi.
// To diagnose whether our hook's functions are actually exported in the
// .dynsym table (and thus reachable by bionic's PLT resolution), we log
// their addresses here. If bionic's linker can find these symbols via
// DT_HASH lookup, open@<addr> should be the address that gets called
// when libminuitwrp's PLT entry for `open` is resolved.
// ---------------------------------------------------------------------------
// Forward declarations so the constructor can take their addresses. open,
// openat, close, ioctl are declared in standard headers (fcntl.h, unistd.h,
// sys/ioctl.h), but __open_2 and __openat_2 are bionic-internal fortified
// variants not in any public header — we must declare them ourselves.
int __open_2(const char *path, int flags);
int __openat_2(int dirfd, const char *path, int flags);

// 6-Z176: defined near the bottom (with the other fatal machinery);
// forward-declared so the constructor can snapshot /proc/self/maps at
// load time (see the comment block in the constructor).
static void fatal_dump_maps(void);

__attribute__((constructor))
static void twrp_fb_hook_init(void) {
    int i;
    write_str(2, "[twrp_fb_hook] loaded ("
#if defined(__aarch64__)
                 "aarch64"
#elif defined(__i386__)
                 "i686"
#else
                 "unknown-arch"
#endif
                 " LD_PRELOAD for /dev/graphics/fb0)\n");

    // INPUT BRIDGE: initialize the slot table (bss-zeroed fd==0 would
    // otherwise alias stdin!). NO raw staging of /dev/input here:
    //
    //   the probe files are pre-created by kr64 (devices phase) — raw
    //   staging here crashed the guest (run 32649156523)
    //
    // Run 32649156523 showed the constructor's mkdir_raw("/dev/input") +
    // openat(O_CREAT) loop passing through the tracer's interception
    // path and corrupting the resume state (recovery SIGSEGV'd,
    // si_code=128 SI_KERNEL, rip inside this hook's text, ~20s in and
    // BEFORE minui ran). kr64 now pre-creates {rootfs}/dev/input/
    // event0+event1 parent-side before the guest is even forked; when
    // minui OPENS one of them, our open() hook swaps the fd for the
    // touch-events socket.
    for (i = 0; i < INBR_MAX_SLOTS; i++) g_inbr[i].fd = -1;
    // Log hook function addresses to confirm they're defined and to
    // correlate with any future PLT-resolution diagnostics. These are
    // the addresses of OUR definitions; if bionic's linker resolves
    // libminuitwrp's `open` PLT entry to a DIFFERENT address, that
    // would explain why our hook isn't being called.
    /* 6-Z180: full 64-bit addresses + the true arch label — run
     * 33021972679's "addrs: open@0x8347270c" looked like a 32-bit
     * hook inside an arm64 process because the banner said i686 and
     * write_hex truncated to 32 bits. write_hex64 prints the real
     * value (same digits on i386). */
    write_str(2, "[twrp_fb_hook] addrs: open@"); write_hex64(2, (unsigned long long)(uintptr_t)&open);
    write_str(2, " openat@"); write_hex64(2, (unsigned long long)(uintptr_t)&openat);
    write_str(2, " __open_2@"); write_hex64(2, (unsigned long long)(uintptr_t)&__open_2);
    write_str(2, " __openat_2@"); write_hex64(2, (unsigned long long)(uintptr_t)&__openat_2);
    write_str(2, " close@"); write_hex64(2, (unsigned long long)(uintptr_t)&close);
    write_str(2, " ioctl@"); write_hex64(2, (unsigned long long)(uintptr_t)(int(*)(int,int,...))&ioctl);
    write_str(2, "\n");

    // 6-Z176: dump /proc/self/maps ONCE AT LOAD. Run 33018901591: the
    // recovery child SIGSEGV'd (si_addr 0xffff00000013, rip
    // 0xffffedbc8470) right after libpixelflinger generated its first
    // scanlines — but the rip could NOT be symbolized because module
    // bases are unknown at crash time (tracer-side /proc/<pid>/maps is
    // ENOENT — the 6-Z167 pid-namespace finding; bionic's debuggerd
    // client also failed: "Unable to open connection to debuggerd").
    // At CONSTRUCTOR time ALL DT_NEEDED libraries (libminuitwrp,
    // libpixelflinger, ...) are already mapped and their bases never
    // move — this snapshot makes ANY later crash rip symbolizable
    // offline. Runs BEFORE the tracer's per-syscall spam, so the dump
    // lands clean (no byte interleaving). One dump per process load
    // (~8KB × the crash-restart cycle count — cheap evidence).
    //
    // 6-Z270: GATE the constructor dump to the boot-critical binaries.
    // Every guest binary (sh, chmod, toybox applets, the OrangeFox
    // startup-script pipeline — ~30 processes in the first 40 s of an
    // R12 boot) was dumping its full module list to the shared stderr
    // pipe at load; the tracer DIAG-logs every stderr write chunk with
    // content escaping, so this was hundreds of 2 KiB copies through
    // the single-threaded ptrace loop per boot — measurable boot tax
    // and log flood (each dump also interleaves into the merged guest
    // log, obscuring the real evidence). Keep the dump ONLY for the
    // crash-prone boot daemons (init / recovery / keystore2): their
    // module bases anchor every later process (same rootfs, same lib
    // set), and crashes in OTHER processes still dump at the FATAL
    // site (fatal_evidence_once → fatal_dump_maps, un-gated).
    //
    // The gate reads /proc/self/cmdline (raw syscalls — /proc passes
    // through the tracer untranslated per the 6-Z170 finding). At
    // constructor time the path is the STAGED exec target
    // (/data/…/twoyi_stage/_system_bin_<name>_<hash>), so the basenames
    // we match carry the _system_bin/_sbin prefix; plain "init" and
    // "twoyi_init" cover the non-staged early loads.
    {
        volatile char cbuf[96];
        int dump_it = 0;
        {
            static const char csrc[] = "/proc/self/cmdline";
            volatile char cpath[sizeof(csrc)];
            unsigned k;
            for (k = 0; k < sizeof(csrc); k++) cpath[k] = csrc[k];
            int cfd = (int)raw_syscall4(SYS_openat, (long)-100 /*AT_FDCWD*/,
                                        (long)(const char *)cpath, 0, 0);
            if (cfd >= 0) {
                long n = raw_syscall3(SYS_read, cfd, (long)(char *)cbuf,
                                      (long)(sizeof(cbuf) - 1));
                raw_syscall1(SYS_close, cfd);
                if (n > 0) {
                    char *p = (char *)cbuf;
                    char *base = p;
                    long i;
                    for (i = 0; i < n; i++) {
                        if (p[i] == '\0') { p[i] = '\0'; break; }
                        if (p[i] == '/') base = &p[i + 1];
                    }
                    p[n] = '\0';
                    /* staged basenames: _system_bin_<name>_<hash> /
                     * _sbin_<name>_<hash>; match the daemon prefixes. */
                    if (strncmp(base, "_system_bin_init", 16) == 0 ||
                        strncmp(base, "_system_bin_recovery", 20) == 0 ||
                        strncmp(base, "_system_bin_keystore2", 21) == 0 ||
                        strncmp(base, "_sbin_recovery", 14) == 0 ||
                        strcmp(base, "init") == 0 ||
                        strcmp(base, "recovery") == 0 ||
                        strcmp(base, "twoyi_init") == 0 ||
                        strcmp(base, "keystore2") == 0) {
                        dump_it = 1;
                    }
                }
            }
        }
        if (dump_it) {
            fatal_dump_maps();
        }
    }
}

// ---------------------------------------------------------------------------
// open() PLT interposition.
//
// TWRP's libminuitwrp calls open("/dev/graphics/fb0", O_RDWR) to get the
// framebuffer fd. We intercept this (and the openat / __open_2 / __openat_2
// variants) to track the returned fd in g_fb_fds.
//
// We do NOT translate paths or do any of the other open-hook work that
// the main loader does — TWRP's init is statically linked and doesn't
// need path translation. We ONLY track fb0 opens.
// ---------------------------------------------------------------------------
static int (*real_open)(const char *, int, ...) = NULL;
static int (*real_openat)(int, const char *, int, ...) = NULL;

static void init_real_funcs(void) {
    // dlsym may be NULL (weak unresolved on old bionic without libdl).
    // In that case real_open / real_openat stay NULL and callers fall
    // back to raw_syscall4(SYS_openat, ...).
    if (!real_open   && dlsym) real_open   = (int (*)(const char *, int, ...))hook_de_self(
                (void *)dlsym(RTLD_NEXT, "open"), "open"); /* 6-Z246 */
    if (!real_openat && dlsym) real_openat = (int (*)(int, const char *, int, ...))hook_de_self(
                (void *)dlsym(RTLD_NEXT, "openat"), "openat"); /* 6-Z246 */
}

int open(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
#if defined(__aarch64__)
    /* 6-Z188: PTY master/slave (see pty_open_dispatch). */
    {
        int pfd = pty_open_dispatch(path, flags);
        if (pfd != -2) return pfd;
    }
#endif
    {
        int in_fd = try_open_input_bridge(path);
        if (in_fd != -2) return in_fd;
    }
    init_real_funcs();
    int fd = real_open ? real_open(path, flags, mode)
                       : (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)path, flags, mode);
    // If opening /dev/graphics/fb0 or /dev/fb0 fails with ENOENT, create
    // the virtual framebuffer file and re-open it. TWRP init may re-mount
    // /dev tmpfs, wiping kr64's pre-created fb0 file.
    // 6-Z166: the create-side mkdir/open ALSO use the rootfs-resolvable
    // form — the hook's .rodata string literals hit the same tracer
    // PEEK failure as the loop buffers, and the prefixed form resolves
    // into the rootfs regardless of tracer translation.
    if (fd < 0 && is_fb_path(path)) {
        // Create /dev/graphics/ directory if needed
        char *gdir = rootfs_path_form("/dev/graphics");
        if (gdir) mkdir_raw(gdir, 0755);
        // Create the fb0 file with the right size (320*640*4 = 819200)
        char *fbp = rootfs_path_form(
            my_strcmp(path, "/dev/fb0") == 0 ? "/dev/fb0" : "/dev/graphics/fb0");
        int create_fd = fbp
            ? (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)fbp,
                                O_CREAT | O_RDWR, 0644)
            : -1;
        if (create_fd >= 0) {
            // Truncate to framebuffer size (runtime native geometry)
            raw_syscall3(SYS_ftruncate, create_fd, fb_smem_len(), 0);
            raw_syscall1(SYS_close, create_fd);
            // Re-open with the original flags
            fd = real_open ? real_open(path, flags, mode)
                           : (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)path, flags, mode);
            write_str(2, "[twrp_fb_hook] created virtual fb0 file, re-opened -> fd=");
            write_num(2, fd);
            write_str(2, "\n");
        }
    }
    // 6-Z166: absolute-open failure → rootfs-resolvable retry (see the
    // rootfs_retry_open comment block above for the full rationale).
    fd = rootfs_retry_open(path, fd, flags, mode);
    // ALWAYS ftruncate the existing fb0 file to TWRP_FB_SMEM_LEN — even
    // when open() SUCCEEDED. Root cause (6-Q's definitive diff of KVM
    // strace vs UI E2E logcat):
    //   - kr64 creates /dev/graphics/fb0 at cfg.width*cfg.height*4 =
    //     320*640*4 = 819200 bytes (auto-detected screen size).
    //   - But fill_fscreeninfo() hardcodes smem_len = 720*1280*4 =
    //     3686400 (TWRP_FB_SMEM_LEN) in FBIOGET_FSCREENINFO.
    //   - In KVM E2E (root): init's mount(tmpfs,/dev) REALLY wipes fb0 →
    //     recovery's open returns ENOENT → the create branch above fires
    //     + ftruncates to 3686400 (matching smem_len). OK.
    //   - In UI E2E (ptrace_emu): the mount is fake-successed (no real
    //     mount) → fb0 SURVIVES at 819200 bytes → open SUCCEEDS → the
    //     create branch is SKIPPED → file stays at the wrong size.
    //   - recovery then does mmap2(fb_fd, smem_len=3686400, ...) on the
    //     819200-byte file. mmap succeeds, but writes past byte 819200
    //     (libminuitwrp clears the whole framebuffer on init) → SIGBUS
    //     → recovery crashes → init exits(1) at iter 3233.
    // Fix: ftruncate to TWRP_FB_SMEM_LEN so the file size always matches
    // the hardcoded smem_len that FBIOGET_FSCREENINFO returns. This
    // makes both KVM + UI E2E paths converge on the correct file size.
    // (We open a SEPARATE O_RDWR fd because the caller's fd may have
    // been opened O_RDONLY, which forbids ftruncate on Linux.)
    if (is_fb_path(path) && fd >= 0) {
        int trunc_fd = (int)raw_syscall4(SYS_openat, AT_FDCWD,
            (long)path, O_RDWR, 0);
        if (trunc_fd >= 0) {
            raw_syscall3(SYS_ftruncate, trunc_fd, fb_smem_len(), 0);
            raw_syscall1(SYS_close, trunc_fd);
            write_str(2, "[twrp_fb_hook] ftruncated existing fb0 to runtime smem_len\n");
        }
    }
    write_str(2, "[twrp_fb_hook] open(\"");
    write_str(2, path ? path : "(null)");
    write_str(2, "\", fl=0x"); write_hex(2, (unsigned int)flags);
    write_str(2, ") -> fd="); write_num(2, fd);
    if (fd >= 0 && is_fb_path(path)) {
        fb_fd_mark(fd);
        write_str(2, " [FB0 TRACKED]");
    }
    if (fd >= 0 && is_ashmem_path(path)) {
        ash_fd_mark(fd);
        write_str(2, " [ASHMEM TRACKED]");
    }
    write_str(2, "\n");
    return fd;
}

int openat(int dirfd, const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
#if defined(__aarch64__)
    /* 6-Z188: PTY master/slave (see pty_open_dispatch). */
    {
        int pfd = pty_open_dispatch(path, flags);
        if (pfd != -2) return pfd;
    }
#endif
    {
        int in_fd = try_open_input_bridge(path);
        if (in_fd != -2) return in_fd;
    }
    init_real_funcs();
    int fd = real_openat ? real_openat(dirfd, path, flags, mode)
                         : (int)raw_syscall4(SYS_openat, dirfd, (long)path, flags, mode);
    // If opening /dev/graphics/fb0 or /dev/fb0 fails with ENOENT, create
    // the virtual framebuffer file and re-open it.
    // 6-Z166: rootfs-resolvable forms for the create side (see open()).
    if (fd < 0 && is_fb_path(path)) {
        char *gdir = rootfs_path_form("/dev/graphics");
        if (gdir) mkdir_raw(gdir, 0755);
        char *fbp = rootfs_path_form(
            my_strcmp(path, "/dev/fb0") == 0 ? "/dev/fb0" : "/dev/graphics/fb0");
        int create_fd = fbp
            ? (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)fbp,
                                O_CREAT | O_RDWR, 0644)
            : -1;
        if (create_fd >= 0) {
            raw_syscall3(SYS_ftruncate, create_fd, fb_smem_len(), 0);
            raw_syscall1(SYS_close, create_fd);
            fd = real_openat ? real_openat(dirfd, path, flags, mode)
                             : (int)raw_syscall4(SYS_openat, dirfd, (long)path, flags, mode);
            write_str(2, "[twrp_fb_hook] created virtual fb0 file, re-opened -> fd=");
            write_num(2, fd);
            write_str(2, "\n");
        }
    }
    // 6-Z166: absolute-open failure → rootfs-resolvable retry. Only
    // absolute paths engage, so the AT_FDCWD retry inside is valid
    // regardless of the caller's dirfd.
    fd = rootfs_retry_open(path, fd, flags, mode);
    // ALWAYS ftruncate the existing fb0 file to TWRP_FB_SMEM_LEN — even
    // when openat() SUCCEEDED. Root cause (6-Q's definitive diff of KVM
    // strace vs UI E2E logcat):
    //   - kr64 creates /dev/graphics/fb0 at cfg.width*cfg.height*4 =
    //     320*640*4 = 819200 bytes (auto-detected screen size).
    //   - But fill_fscreeninfo() hardcodes smem_len = 720*1280*4 =
    //     3686400 (TWRP_FB_SMEM_LEN) in FBIOGET_FSCREENINFO.
    //   - In KVM E2E (root): init's mount(tmpfs,/dev) REALLY wipes fb0 →
    //     recovery's openat returns ENOENT → the create branch above
    //     fires + ftruncates to 3686400 (matching smem_len). OK.
    //   - In UI E2E (ptrace_emu): the mount is fake-successed (no real
    //     mount) → fb0 SURVIVES at 819200 bytes → openat SUCCEEDS → the
    //     create branch is SKIPPED → file stays at the wrong size.
    //   - recovery then does mmap2(fb_fd, smem_len=3686400, ...) on the
    //     819200-byte file. mmap succeeds, but writes past byte 819200
    //     (libminuitwrp clears the whole framebuffer on init) → SIGBUS
    //     → recovery crashes → init exits(1) at iter 3233.
    // Fix: ftruncate to TWRP_FB_SMEM_LEN so the file size always matches
    // the hardcoded smem_len that FBIOGET_FSCREENINFO returns. This
    // makes both KVM + UI E2E paths converge on the correct file size.
    // (We open a SEPARATE O_RDWR fd because the caller's fd may have
    // been opened O_RDONLY, which forbids ftruncate on Linux. The path
    // is absolute — /dev/graphics/fb0 or /dev/fb0 — so AT_FDCWD is
    // correct regardless of the caller's dirfd.)
    if (is_fb_path(path) && fd >= 0) {
        int trunc_fd = (int)raw_syscall4(SYS_openat, AT_FDCWD,
            (long)path, O_RDWR, 0);
        if (trunc_fd >= 0) {
            raw_syscall3(SYS_ftruncate, trunc_fd, fb_smem_len(), 0);
            raw_syscall1(SYS_close, trunc_fd);
            write_str(2, "[twrp_fb_hook] ftruncated existing fb0 to runtime smem_len\n");
        }
    }
    write_str(2, "[twrp_fb_hook] openat(df="); write_num(2, dirfd);
    write_str(2, ", \"");
    write_str(2, path ? path : "(null)");
    write_str(2, "\", fl=0x"); write_hex(2, (unsigned int)flags);
    write_str(2, ") -> fd="); write_num(2, fd);
    if (fd >= 0 && is_fb_path(path)) {
        fb_fd_mark(fd);
        write_str(2, " [FB0 TRACKED]");
    }
    if (fd >= 0 && is_ashmem_path(path)) {
        ash_fd_mark(fd);
        write_str(2, " [ASHMEM TRACKED]");
    }
    write_str(2, "\n");
    return fd;
}

// 6-Z222: open64 / openat64 — modern bionic (Android 11+) exports these
// as REAL symbols, so libraries built against it (e.g. a 64-bit TWRP
// image's libminuitwrp) bind their open64@plt directly to libc and
// bypass the open/openat hooks above. The fb0 fd then never lands in
// g_fb_fds and every FB ioctl passes through as ENOTTY (zeroed
// screeninfo → gr_fb_width NULL crash). These thin aliases route through
// the SAME openat() hook body — identical pty/input/fb0/ashmem handling.
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

// bionic's fortified open variants. These are called by code compiled with
// -D_FORTIFY_SOURCE (most of AOSP). They have the same path-tracking logic.
int __open_2(const char *path, int flags) {
#if defined(__aarch64__)
    /* 6-Z188c: fortified open must serve the pty too (run 33125938988:
     * the terminal slave open went through a __open* variant and
     * bypassed the pty checks). */
    {
        int pfd = pty_open_dispatch(path, flags);
        if (pfd != -2) return pfd;
    }
#endif
    {
        int in_fd = try_open_input_bridge(path);
        if (in_fd != -2) return in_fd;
    }
    init_real_funcs();
    static int (*real_open2)(const char *, int) = NULL;
    if (!real_open2 && dlsym) real_open2 = (int (*)(const char *, int))hook_de_self(
                (void *)dlsym(RTLD_NEXT, "__open_2"), "__open_2"); /* 6-Z246 */
    int fd;
    if (real_open2) fd = real_open2(path, flags);
    else            fd = (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)path, flags, 0);
    // 6-Z166: absolute-open failure → rootfs-resolvable retry.
    fd = rootfs_retry_open(path, fd, flags, 0);
    // DIAGNOSTIC (Task 31): log EVERY __open_2() call (bionic's fortified
    // open variant — selected by -D_FORTIFY_SOURCE). If libminuitwrp was
    // built with _FORTIFY_SOURCE, this is the variant that gets called
    // instead of open().
    write_str(2, "[twrp_fb_hook] __open_2(\"");
    write_str(2, path ? path : "(null)");
    write_str(2, "\", fl=0x"); write_hex(2, (unsigned int)flags);
    write_str(2, ") -> fd="); write_num(2, fd);
    if (fd >= 0 && is_fb_path(path)) {
        fb_fd_mark(fd);
        write_str(2, " [FB0 TRACKED]");
    }
    if (fd >= 0 && is_ashmem_path(path)) {
        ash_fd_mark(fd);
        write_str(2, " [ASHMEM TRACKED]");
    }
    write_str(2, "\n");
    return fd;
}

int __openat_2(int dirfd, const char *path, int flags) {
#if defined(__aarch64__)
    /* 6-Z188c: fortified openat serves the pty too (see __open_2). */
    {
        int pfd = pty_open_dispatch(path, flags);
        if (pfd != -2) return pfd;
    }
#endif
    {
        int in_fd = try_open_input_bridge(path);
        if (in_fd != -2) return in_fd;
    }
    init_real_funcs();
    static int (*real_openat2)(int, const char *, int) = NULL;
    if (!real_openat2 && dlsym) real_openat2 = (int (*)(int, const char *, int))hook_de_self(
                (void *)dlsym(RTLD_NEXT, "__openat_2"), "__openat_2"); /* 6-Z246 */
    int fd;
    if (real_openat2) fd = real_openat2(dirfd, path, flags);
    else              fd = (int)raw_syscall4(SYS_openat, dirfd, (long)path, flags, 0);
    // 6-Z166: absolute-open failure → rootfs-resolvable retry (absolute
    // paths ignore dirfd, AT_FDCWD retry is always valid).
    fd = rootfs_retry_open(path, fd, flags, 0);
    // DIAGNOSTIC (Task 31): log EVERY __openat_2() call.
    write_str(2, "[twrp_fb_hook] __openat_2(df="); write_num(2, dirfd);
    write_str(2, ", \"");
    write_str(2, path ? path : "(null)");
    write_str(2, "\", fl=0x"); write_hex(2, (unsigned int)flags);
    write_str(2, ") -> fd="); write_num(2, fd);
    if (fd >= 0 && is_fb_path(path)) {
        fb_fd_mark(fd);
        write_str(2, " [FB0 TRACKED]");
    }
    if (fd >= 0 && is_ashmem_path(path)) {
        ash_fd_mark(fd);
        write_str(2, " [ASHMEM TRACKED]");
    }
    write_str(2, "\n");
    return fd;
}

// ---------------------------------------------------------------------------
// close() PLT interposition — clear fd tracking when an fb0 fd is closed.
// ---------------------------------------------------------------------------
int close(int fd) {
    if (in_fd_is_tracked(fd)) {
        struct inbr_slot *s = inbr_slot_for(fd);
        if (s) {
            s->fd = -1;
            s->connected = 0;
            s->ring_len = 0;
            s->stage_len = 0;
        }
        in_fd_clear(fd);
        write_str(2, "[twrp_fb_hook] close(fd=");
        write_num(2, fd);
        write_str(2, ") (was INPUT bridge fd)\n");
    }
    if (fb_fd_is_tracked(fd)) {
        fb_fd_clear(fd);
        write_str(2, "[twrp_fb_hook] close(fd=");
        write_num(2, fd);
        write_str(2, ") (was tracked fb0 fd)\n");
    }
    if (ash_fd_is_tracked(fd)) {
        ash_fd_clear(fd);
        write_str(2, "[twrp_fb_hook] close(fd=");
        write_num(2, fd);
        write_str(2, ") (was tracked ashmem fd)\n");
    }
    static int (*real_close)(int) = NULL;
    if (!real_close && dlsym) real_close = (int (*)(int))hook_de_self(
                (void *)dlsym(RTLD_NEXT, "close"), "close"); /* 6-Z246 */
    if (real_close) return real_close(fd);
    return (int)raw_syscall1(SYS_close, fd);
}

// ---------------------------------------------------------------------------
// ioctl() PLT interposition — the actual FIX for the libminuitwrp crash.
//
// libminuitwrp calls:
//   ioctl(fd, FBIOGET_VSCREENINFO, &vi)  // 0x4600
//   ioctl(fd, FBIOGET_FSCREENINFO, &fi)  // 0x4602
//   ioctl(fd, FBIOPUT_VSCREENINFO, &vi)  // 0x4601 (set mode)
//   ioctl(fd, FBIOPAN_DISPLAY, &vi)      // 0x4606 (page flip)
//   ioctl(fd, FBIOBLANK, ...)            // 0x4611 (blank/unblank)
//
// On a regular file, the kernel returns ENOTTY for all of these. Without
// our hook, libminuitwrp ignores the ENOTTY, leaves vi/fi zeroed, then
// dereferences the NULL pointer in vi (e.g. vi.xres is read and used as
// a size — but it's 0, leading to NULL deref or zero-sized allocation).
//
// Our hook:
//   - For tracked fb0 fds: respond to FB ioctls with valid data, return 0.
//   - For all other fds: pass through to the real ioctl (do NOT suppress
//     real ioctl errors — we only fake for fb0).
// ---------------------------------------------------------------------------
// bionic: int ioctl(int, int, ...)         (bits/ioctl.h on Android)
// glibc:  int ioctl(int, unsigned long, ...) (sys/ioctl.h on Linux)
//
// We MUST match bionic's signature EXACTLY (int, not unsigned long) AND
// must NOT add __attribute__((overloadable)). Bionic's <bits/ioctl.h>
// declares `int ioctl(int __fd, int __op, ...);` UNMARKED — i.e. without
// the overloadable attribute. clang enforces: ALL overloads of a given
// name must consistently have (or not have) the overloadable attribute.
//   - If we declare `int ioctl(int, unsigned long, ...)` (different
//     signature, both unmarked) → clang errors:
//       "at most one overload for a given name may lack the 'overloadable'
//        attribute"
//     (KVM run 31575531674 hit this after commit ea3a484 changed the
//     request param to unsigned long for spurious glibc-compat reasons.)
//   - If we declare `int ioctl(int, int, ...) __attribute__((overloadable))`
//     (same signature, but we mark ours while bionic's is unmarked) →
//     clang errors:
//       "redeclaration of 'ioctl' must not have the 'overloadable' attribute"
//     (KVM run 31577950499 hit this after commit d73a848 added overloadable
//     on top of an already-matching signature — the attribute was redundant
//     and conflicting.)
//
// The correct fix: match bionic's signature EXACTLY (int request) and do
// NOT add overloadable. Two unmarked declarations with identical
// signatures are the SAME function (not an overload), which is what we
// want for LD_PRELOAD interposition — bionic's dynamic linker resolves
// the first definition found in the link order, and LD_PRELOAD .so
// entries come before the executable's own libs.
// ---------------------------------------------------------------------------
// ── 6-Z187c: EXEC INTERPOSITION ─────────────────────────────────────────
//
// TWRP's terminal (gui/terminal.cpp runSlave) does
//   execl("/sbin/sh", "sh", NULL);
//   _exit(127);
// and run 33120905168 showed the tracer's +1 cwd-relative fallback for
// the PEEK-blind execl path STILL ended in exit(127) — the terminal
// prints "Child processes exited.". The HOOK can read its own address
// space (the same pages the tracer cannot PEEK), so give exec the same
// treatment as open: try the {rootfs}-prefix form FIRST via a MARKED
// raw syscall (untouchable by the tracer — no translation, no +1),
// then the raw form (the tracer translates it when it CAN read it),
// then the cwd-relative form (cwd == rootfs per 6-Z187b). With the
// 6-Z187 provisioning, {rootfs}/sbin/sh is a REAL symlink to the
// STAGED busybox on the exec-able cache partition — the prefix form
// succeeds and the terminal gets its shell.
// ---------------------------------------------------------------------------

static long hook_exec_common(const char *path, char *const argv[], char *const envp[]) {
    if (!path) return -22; /* EINVAL */
    write_str(2, "[twrp_fb_hook] exec path=\"");
    write_str(2, path);
    write_str(2, "\"\n");
    if (path[0] == '/') {
        char *alt = rootfs_path_form(path);
        if (alt && alt != path + 1) {
            long r = raw_syscall4_marked(SYS_execve, (long)alt,
                                         (long)argv, (long)envp, 0);
            write_str(2, "[twrp_fb_hook] exec prefix-form ret=");
            write_num(2, (int)r);
            write_str(2, "\n");
            if (r == 0) return 0; /* never reached on success */
        }
        long r2 = raw_syscall4(SYS_execve, (long)path, (long)argv,
                               (long)envp, 0);
        write_str(2, "[twrp_fb_hook] exec raw-form ret=");
        write_num(2, (int)r2);
        write_str(2, "\n");
        if (r2 < 0 && path[1] != '\0') {
            long r3 = raw_syscall4_marked(SYS_execve, (long)(path + 1),
                                          (long)argv, (long)envp, 0);
            write_str(2, "[twrp_fb_hook] exec cwd-relative-form ret=");
            write_num(2, (int)r3);
            write_str(2, "\n");
            return r3;
        }
        return r2;
    }
    return raw_syscall4(SYS_execve, (long)path, (long)argv,
                         (long)envp, 0);
}

int execve(const char *path, char *const argv[], char *const envp[]) {
    return (int)hook_exec_common(path, argv, envp);
}

int execv(const char *path, char *const argv[]) {
    char *const empty_envp[1] = { 0 };
    char **env = (environ && *environ) ? environ : (char **)empty_envp;
    return (int)hook_exec_common(path, argv, env);
}

int execl(const char *path, const char *arg0, ...) {
    /* Build argv from the varargs (arg0 .. NULL). TWRP's terminal calls
     * execl("/sbin/sh", "sh", NULL) — tiny argv, stack buffer is fine. */
    const char *argv_stack[33];
    int n = 0;
    va_list ap;
    va_start(ap, arg0);
    argv_stack[n++] = arg0;
    while (n < 32) {
        const char *a = va_arg(ap, const char *);
        if (!a) break;
        argv_stack[n++] = a;
    }
    va_end(ap);
    argv_stack[n] = 0;
    char *const empty_envp[1] = { 0 };
    char **env = (environ && *environ) ? environ : (char **)empty_envp;
    return (int)hook_exec_common(path, (char *const *)argv_stack, env);
}

#if defined(__aarch64__)
// ── 6-Z188: ptsname/grantpt interposition (see the PTY block above).
// bionic's ptsname() runs TIOCGPTN + builds "/dev/pts/%d" — both sides
// are already virtualized, but interposing ptsname directly is the
// robust path (some builds read /proc/self/fd/N instead). grantpt is a
// no-op (permissions are ours). ──
char *ptsname(int fd) {
    static char name_buf[16];
    int slot = pty_slot_of_master(fd);
    if (slot < 0) return NULL;
    /* 6-Z188f: the name carries the SLAVE FD NUMBER — the fork child
     * inherited that exact fd, so its open("/dev/pts/<n>") dups it
     * with zero per-process state (run 33128456506: the slot-table
     * lookup MISSed in the child and the dup got faked -22). */
    int slave = g_pty_slave_fd[slot];
    if (slave < 0) return NULL;
    char *p = name_buf;
    const char *s = "/dev/pts/";
    while (*s) *p++ = *s++;
    if (slave >= 100) *p++ = '0' + (slave / 100) % 10;
    if (slave >= 10)  *p++ = '0' + (slave / 10) % 10;
    *p++ = '0' + slave % 10;
    *p = '\0';
    write_str(2, "[twrp_fb_hook] ptsname(fd=");
    write_num(2, fd);
    write_str(2, ") -> ");
    write_str(2, name_buf);
    write_str(2, "\n");
    return name_buf;
}

int ptsname_r(int fd, char *buf, size_t buflen) {
    char *n = ptsname(fd);
    if (!n) return 25; /* ENOTTY */
    size_t l = 0;
    while (n[l]) l++;
    if (l + 1 > buflen) return 34; /* ERANGE */
    for (size_t i = 0; i <= l; i++) buf[i] = n[i];
    return 0;
}

int grantpt(int fd) {
    return pty_slot_of_master(fd) >= 0 ? 0 : -1;
}
#endif

int ioctl(int fd, int request, ...) {
    va_list ap;
    va_start(ap, request);
    void *argp = va_arg(ap, void *);
    va_end(ap);

    unsigned req = (unsigned)request;

    // INPUT-fd ioctls come FIRST (before the fb fast-path below) — the
    // evdev capability probe must be answered for our socket fds.
    if (in_fd_is_tracked(fd)) {
        int r = input_ioctl(fd, req, argp);
        if (r != -2) {
            if (g_inbr_log_ioctl < 8) {
                g_inbr_log_ioctl++;
                write_str(2, "[twrp_fb_hook] INPUT ioctl(fd=");
                write_num(2, fd);
                write_str(2, ", req=0x");
                write_hex(2, req);
                write_str(2, ") -> ");
                write_num(2, r);
                write_str(2, "\n");
            }
            return r;
        }
        /* unknown evdev ioctl — pass through to the socket (ENOTTY) */
        static int (*real_ioctl_in)(int, int, ...) = NULL;
        if (!real_ioctl_in && dlsym) real_ioctl_in = (int (*)(int, int, ...))hook_de_self(
                (void *)dlsym(RTLD_NEXT, "ioctl"), "ioctl"); /* 6-Z246 */
        if (real_ioctl_in) return real_ioctl_in(fd, request, argp);
        return (int)raw_syscall3(SYS_ioctl, fd, request, (long)argp);
    }

    // 6-Z171c: ashmem-fd ioctls — fake the ASHMEM_* protocol on the
    // pre-created regular-file /dev/ashmem (see the ashmem block above).
    if (ash_fd_is_tracked(fd)) {
        int r = ashmem_ioctl(fd, req, (unsigned long)argp);
        if (r != -2) return r;
    }

#if defined(__aarch64__)
    // ── 6-Z188: PTY ioctls — MASTER protocol (TIOCSPTLCK/TIOCGPTN/...)
    // first, then the SLAVE tty protocol (TCGETS so isatty() succeeds —
    // 6-Z188i). Both must run BEFORE the generic handlers (these fds
    // are sockets/files; the real ioctl would ENOTTY). ──
    {
        long pr = pty_master_ioctl(fd, req, (unsigned long)argp);
        if (pr != -2) return (int)pr;
    }
    {
        long sr = pty_slave_ioctl(fd, req, (unsigned long)argp);
        if (sr != -2) return (int)sr;
    }
#endif

    // 6-Z186: TIOCGWINSZ — answer with a standard 80x24 winsize when the
    // real fd can't (what a terminal emulator does) so recovery terminal
    // logic can proceed instead of looping on the failure.
    // 6-Z188: ARCH-CORRECT codes. The old check was 0x540e ONLY — that is
    // the x86 TIOCGWINSZ but the aarch64 (asm-generic) TIOCSCTTY! On
    // arm64, ash's TIOCGWINSZ(0x5413) went unanswered while SCTTY got a
    // bogus 8-byte winsize write. Handle BOTH arches' WINSZ codes, and
    // answer SCTTY(0x540E asm-generic) with 0 on aarch64.
#if defined(__aarch64__)
    if (req == 0x5413u) {
#else
    if (req == 0x540eu || req == 0x5413u) {
#endif
        static int (*real_winsz)(int, int, ...) = NULL;
        if (!real_winsz && dlsym)
            real_winsz = (int (*)(int, int, ...))hook_de_self(
                (void *)dlsym(RTLD_NEXT, "ioctl"), "ioctl"); /* 6-Z246 */
        int r = real_winsz ? real_winsz(fd, request, argp)
                           : (int)raw_syscall3(SYS_ioctl, fd, request, (long)argp);
        if (r == 0) return 0;
        if (argp) {
            // struct winsize { u16 ws_row, ws_col, ws_xpixel, ws_ypixel; }
            unsigned short *ws = (unsigned short *)argp;
            ws[0] = 24; ws[1] = 80; ws[2] = 80 * 8; ws[3] = 24 * 16;
        }
        return 0;
    }
#if defined(__aarch64__)
    // 6-Z188: TIOCSCTTY (asm-generic 0x540E) on ANY fd — the terminal
    // slave child's ioctl(0, TIOCSCTTY, 1) must "succeed" (there is no
    // real controlling tty; ash degrades gracefully either way).
    if (req == 0x540Eu) {
        return 0;
    }
#endif

    // DIAGNOSTIC (Task 31): log ioctl() calls to verify our hook is being
    // invoked. Key ioctl numbers to watch for:
    //   FBIOGET_VSCREENINFO = 0x4600  (libminuitwrp reads screen size)
    //   FBIOGET_FSCREENINFO = 0x4602  (libminuitwrp reads smem_len for mmap)
    //   FBIOPUT_VSCREENINFO = 0x4601
    //   FBIOPAN_DISPLAY     = 0x4606
    //   FBIOBLANK            = 0x4611
    // 6-Z186: RATE-LIMITED. The old unconditional log flooded the TWRP
    // terminal screen with an "error" line per ioctl (the terminal page
    // prints child stderr). FB-family requests (0x46xx) and tracked fds
    // stay unlimited (diagnostic value); everything else is capped.
    {
        int tracked = fb_fd_is_tracked(fd);
        static unsigned g_ioctl_diag_dropped;
        if (tracked || (req >> 8) == 0x46u || g_ioctl_diag_dropped < 16) {
            if (!tracked && (req >> 8) != 0x46u) g_ioctl_diag_dropped++;
            write_str(2, "[twrp_fb_hook] ioctl(fd="); write_num(2, fd);
            write_str(2, ", req=0x"); write_hex(2, req);
            write_str(2, ") [trk="); write_num(2, tracked); write_str(2, "]\n");
        }
    }

    // Fast path: not an fb0 fd, pass through.
    if (!fb_fd_is_tracked(fd)) {
        static int (*real_ioctl)(int, int, ...) = NULL;
        if (!real_ioctl && dlsym) real_ioctl = (int (*)(int, int, ...))hook_de_self(
                (void *)dlsym(RTLD_NEXT, "ioctl"), "ioctl"); /* 6-Z246 */
        if (real_ioctl) return real_ioctl(fd, request, argp);
        return (int)raw_syscall3(SYS_ioctl, fd, request, (long)argp);
    }

    // fb0 fd — respond to FB ioctls.
    // FB ioctl numbers (linux/fb.h, no size encoding, same on all arches):
    //   FBIOGET_VSCREENINFO  = 0x4600
    //   FBIOPUT_VSCREENINFO  = 0x4601
    //   FBIOGET_FSCREENINFO  = 0x4602
    //   FBIOPAN_DISPLAY      = 0x4606
    //   FBIOBLANK            = 0x4611
    //   FBIO_WAITFORVSYNC    = 0x40044620
    switch (req) {
        case 0x4600u: {  // FBIOGET_VSCREENINFO
            if (argp) fill_vscreeninfo((struct fb_var_screeninfo *)argp);
            // 6-Z84: log the ACTUAL configured values (the old hardcoded
            // "720x1280" string survived the 6-Z64 geometry fix and
            // misdirected run analysis for hours — TWRP's own prints had
            // the truth: 320 x 640).
            write_str(2, "[twrp_fb_hook] ioctl(FBIOGET_VSCREENINFO) -> ");
            write_num(2, fb_w()); write_str(2, "x"); write_num(2, fb_h());
            write_str(2, "@"); write_num(2, TWRP_FB_BPP); write_str(2, "bpp\n");
            return 0;
        }
        case 0x4601u: {  // FBIOPUT_VSCREENINFO — accept the mode change
            write_str(2, "[twrp_fb_hook] ioctl(FBIOPUT_VSCREENINFO) -> success\n");
            return 0;
        }
        case 0x4602u: {  // FBIOGET_FSCREENINFO
            if (argp) fill_fscreeninfo((struct fb_fix_screeninfo *)argp);
            write_str(2, "[twrp_fb_hook] ioctl(FBIOGET_FSCREENINFO) -> smem_len=");
            write_num(2, (int)fb_smem_len());
            write_str(2, " line_length=");
            write_num(2, (int)fb_line_length());
            write_str(2, "\n");
            return 0;
        }
        case 0x4606u: {  // FBIOPAN_DISPLAY — page flip, accept
            return 0;
        }
        case 0x4611u: {  // FBIOBLANK — arg is the blank level (0=unblank, 1-4=blank)
            // Always return success (we're a virtual display, blanking is a no-op).
            return 0;
        }
        case 0x40044620u: {  // FBIO_WAITFORVSYNC
            // No real vsync to wait for — return immediately.
            return 0;
        }
        default: {
            // Other FB ioctls (FBIOGETCMAP, FBIOPUTCMAP, FBIO_CURSOR, etc.)
            // — fake success for 0x46xx range, ENOTTY for anything else.
            if ((req & 0xff00) == 0x4600) {
                return 0;
            }
            // Non-FB ioctl on an fb0 fd — shouldn't happen, but pass through.
            static int (*real_ioctl)(int, int, ...) = NULL;
            if (!real_ioctl && dlsym) real_ioctl = (int (*)(int, int, ...))hook_de_self(
                (void *)dlsym(RTLD_NEXT, "ioctl"), "ioctl"); /* 6-Z246 */
            if (real_ioctl) return real_ioctl(fd, request, argp);
            return (int)raw_syscall3(SYS_ioctl, fd, request, (long)argp);
        }
    }
}

// ---------------------------------------------------------------------------
// mmap() — NOT HOOKED.
//
// The previous version of this file intercepted mmap() to provide a
// MAP_ANONYMOUS fallback if the real mmap failed on an fb0 fd. But that
// hook required dlsym(RTLD_NEXT, "mmap") to find the real mmap, and on
// old bionic (AOSP 5.1, TWRP) dlsym is unavailable (weak unresolved —
// see the comment above the dlsym declaration). Without real_mmap, the
// hook returned MAP_FAILED unconditionally, which would crash TWRP's
// libminuitwrp at the mmap() call.
//
// Removing the mmap hook entirely is the correct fix:
//   - kr64 pre-creates /dev/graphics/fb0 as a REGULAR FILE of exactly
//     3,686,400 bytes (720×1280×4). mmap() on a regular file works
//     natively via bionic's libc → kernel mmap2 syscall. No hook needed.
//   - bionic's mmap sets errno correctly (our raw_syscall6 fallback
//     would not, since i386's 6th syscall arg clobbers ebp).
//   - The LD_PRELOAD .so no longer needs to export an mmap symbol, so
//     there's no risk of recursing into our own hook.
//
// If a future TWRP build requires mmap interception (e.g. for a real
// device file that can't be a regular file), we'd need to implement
// raw_syscall6 with explicit ebp save/restore — see the i386 syscall
// convention comment above raw_syscall1.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 6-Z171a: abort()/__assert2()/__cxa_pure_virtual() interposition — NAME
// the fatal site before dying.
//
// Run 33010273952 (arm64): the recovery child SELF-ABORTS via
// tgkill(self, SIGABRT) inside minui gr_init right after the splash asset
// loads, with NO abort message in any channel (not in the shared stderr,
// not in logcat; debuggerd does not attach to traced children). We
// intercept the PLT-visible fatal entries, print the caller PC (plus
// file:line:expr for bionic asserts), and then re-raise SIGABRT to the
// kernel so the observable behavior (signal death) is IDENTICAL — the
// only delta is one stderr line of evidence.
//
// Coverage note: calls to abort() from WITHIN bionic's libc.so (internal
// aliases) do not cross the PLT and are not interposed; but TWRP's own
// code (recovery binary, libminuitwrp, libc++ std::terminate → abort)
// DOES cross the PLT and will be caught.
//
// PC capture: __builtin_return_address(0) is read at function entry
// (before any of our calls can clobber the link register). On aarch64
// this is x30; on i386 the return address slot at [esp]. With -O2 clang
// preserves it for level-0 return addresses.
// ---------------------------------------------------------------------------

// ONE-SHOT guard: TWRP's crash handler catches SIGABRT and may re-abort
// (or init restarts recovery into another abort). Without the guard the
// maps dump + marker lines re-enter at full speed — run 33014296538: the
// whole framework wedged seconds after launch with every adb channel
// dead (the abort spam loop is a prime suspect for the CPU meltdown).
// First entry prints evidence ONCE; every later entry goes straight to
// the raw tgkill.
static volatile int g_fatal_entered = 0;

// Dump up to ~32 KiB of /proc/self/maps to stderr — gives the exact load
// bases needed to symbolize the printed caller PCs (the binaries are
// stripped; module+offset + disasm context identifies the call site).
// Uses only raw syscalls; runs INSIDE the child, where /proc/self/maps is
// always readable (the tracer's /proc/<pid>/maps view was ENOENT — the
// 6-Z167 finding — but the child's own /proc/self never is).
static void fatal_dump_maps(void) {
    write_str(2, "[twrp_fb_hook] --- /proc/self/maps at fatal ---\n");
    /* 6-Z176b: build the path in a VOLATILE STACK buffer. A .rodata
     * string literal is in the tracer's PEEK-blind class (EIO) — the
     * 6-Z170 +1 rewrite then resolves "proc/self/maps" RELATIVE to the
     * rootfs cwd → ENOENT (run 33019847021: "(open /proc/self/maps
     * failed)"). A volatile buffer physically lives on the stack: the
     * tracer reads stack addresses fine, translate_path passes /proc paths
     * through untranslated, and the kernel opens the REAL maps file. */
    volatile char mpath[17]; /* "/proc/self/maps" + NUL */
    {
        static const char src[] = "/proc/self/maps";
        unsigned k;
        for (k = 0; k < sizeof(src); k++) mpath[k] = src[k];
    }
    int mfd = (int)raw_syscall4(SYS_openat, AT_FDCWD,
                                (long)(const char *)mpath, 0 /*O_RDONLY*/, 0);
    if (mfd < 0) {
        write_str(2, "(open /proc/self/maps failed)\n");
        return;
    }
    static char buf[2048];
    long total = 0;
    for (;;) {
        long n = raw_syscall3(SYS_read, mfd, (long)buf, (long)sizeof(buf));
        if (n <= 0) break;
        raw_syscall3(SYS_write, 2, (long)buf, n);
        total += n;
        /* 6-Z269: 32 KiB truncated before the aborting module's r-xp
         * segment on OrangeFox R12 (the PC ended up un-symbolizable);
         * 128 KiB covers the full module list without drowning the
         * stderr pipe. */
        if (total > 131072) break;
    }
    raw_syscall1(SYS_close, mfd);
    write_str(2, "[twrp_fb_hook] --- end maps ---\n");
}

// Print the fatal evidence exactly once (see g_fatal_entered).
static void fatal_evidence_once(const char *kind, void *pc) {
    if (g_fatal_entered) return;
    g_fatal_entered = 1;
    write_str(2, "[twrp_fb_hook] *** ");
    write_str(2, kind);
    write_str(2, " INTERCEPTED *** caller_pc=0x");
    write_hex64(2, (unsigned long long)(unsigned long)pc);
    write_str(2, "\n");
    fatal_dump_maps();
}

// 6-Z269: park forever instead of re-raising, with EXPLICIT NULL args.
//
// WHY THIS CHANGED (the 31.9M-stop storm, CI run 33353128469): the old
// loop was
//     for (;;) { tgkill(pid, tid, SIGABRT); raw_syscall2(SYS_ppoll, 0, 0); }
// with TWO independent bugs that turned every guest abort() into a
// tracer-melting busy-spin:
//
//   1. raw_syscall2(SYS_ppoll, 0, 0) sets ONLY x0/x1 (fds=NULL, nfds=0).
//      On aarch64/x86_64 the inline asm leaves x2 (timespec*) and x3
//      (sigmask*) holding whatever the caller frame left there — at the
//      abort site that is garbage (saved PCs, clobbered temporaries).
//      The kernel then copies the timespec from a wild pointer →
//      ppoll returns -EFAULT IMMEDIATELY instead of blocking. The
//      "park" call never parked: the loop ran at full speed,
//      tgkill+ppoll pairs at ~60k/s, tracer loop pegged at 100% CPU —
//      measured 31,961,601 loop stops over one boot (nr=73 EXIT -14 +
//      nr=131 EXIT -3 alternating in every kr64 log line of the storm
//      window), the single biggest component of the user-reported
//      "~1 minute to boot".
//
//   2. raw_syscall1(SYS_getpid) returns the TRACER-FAKED pid 1 (the
//      seccomp/SIGSYS getpid fake), so tgkill(1, tid, SIGABRT) fails
//      with -ESRCH in the real kernel — the signal was never delivered
//      to anyone, the abort never completed, and (with tracer 6-Z266
//      now translating fake-pid-1 kill-family arguments to the real
//      tgid) a raise here WOULD actually deliver SIGABRT and kill the
//      whole process.
//
// WHY PARK (default) INSTEAD OF DIE: the aborting site on OrangeFox R12
// lavender is a glog "bad_function_call was thrown in -fno-exceptions
// mode" FATAL in the recovery main thread ~10 s into boot. On a physical
// device abort() kills the process and init restarts it. Under twoyi a
// full process death means: recovery gone → no framebuffer producer →
// the boot-to-UI run is over (and an init restart loop would re-abort
// every cycle). The pre-6-Z266 behavior — ESRCH spin — accidentally kept
// the process alive while the OTHER threads (minui render, input,
// fb_hook splash) kept producing frames. The park preserves exactly that
// observable behavior (aborting thread gone from the scheduler, process
// alive) at ZERO CPU: ppoll(NULL, 0, NULL, NULL) with all-NULL args
// blocks in the kernel forever (nfds=0, no timeout, no sigmask), the
// tracee sleeps, the tracer waits at its EXIT stop, nothing runs.
//
// TWOYI_ABORT_RERAISE=1 (opt-in, read once) restores the old
// raise-then-park semantics for debugging, with the ppoll args FIXED
// (all-NULL) so the park between re-raises actually parks.
static volatile int g_abort_reraise_mode = -1; /* -1 = unresolved */

static void fatal_reraise(void) __attribute__((noreturn));
static void fatal_reraise(void) {
    if (g_abort_reraise_mode < 0) {
        /* Raw getenv-free probe: reading an env var through libc getenv
         * is safe here (no TLS/errno hazards — environ is plain data). */
        const char *m = getenv("TWOYI_ABORT_RERAISE");
        g_abort_reraise_mode = (m != NULL && m[0] == '1') ? 1 : 0;
    }
    if (g_abort_reraise_mode) {
        long pid = raw_syscall1(SYS_getpid, 0);
        long tid = raw_syscall1(SYS_gettid, 0);
        for (;;) {
            raw_syscall4(SYS_tgkill, pid, tid, 6 /*SIGABRT*/, 0);
            /* 6-Z269 FIX: was raw_syscall2(SYS_ppoll, 0, 0) — x2/x3
             * garbage → EFAULT → busy-spin. All-NULL ppoll blocks. */
            raw_syscall4(SYS_ppoll, 0, 0, 0, 0);
        }
    }
    /* Default: park this thread forever at zero cost. ppoll with
     * nfds=0, NULL timeout and NULL sigmask has nothing to copy from
     * user memory — no EFAULT path — and blocks until a signal (which
     * for an aborting thread never meaningfully arrives). */
    for (;;) {
        raw_syscall4(SYS_ppoll, 0, 0, 0, 0);
    }
}

void abort(void) __attribute__((noreturn));
void abort(void) {
    void *pc = NULL;
    pc = __builtin_return_address(0);
    fatal_evidence_once("abort()", pc);
    fatal_reraise();
}

// bionic assert(3): void __assert2(const char *file, int line, const char *expr)
void __assert2(const char *file, int line, const char *expr) __attribute__((noreturn));
void __assert2(const char *file, int line, const char *expr) {
    void *pc = NULL;
    pc = __builtin_return_address(0);
    if (!g_fatal_entered) {
        g_fatal_entered = 1;
        write_str(2, "[twrp_fb_hook] *** __assert2 INTERCEPTED *** ");
        write_str(2, file ? file : "(null)");
        write_str(2, ":");
        write_num(2, line);
        write_str(2, ": ");
        write_str(2, expr ? expr : "(null)");
        write_str(2, " caller_pc=0x");
        write_hex64(2, (unsigned long long)(unsigned long)pc);
        write_str(2, "\n");
        fatal_dump_maps();
    }
    fatal_reraise();
}

// C++ pure-virtual call → std::terminate → abort. Interposed directly so
// we see it even if terminate is inlined in the caller's binary.
void __cxa_pure_virtual(void) __attribute__((noreturn));
void __cxa_pure_virtual(void) {
    void *pc = NULL;
    pc = __builtin_return_address(0);
    fatal_evidence_once("__cxa_pure_virtual", pc);
    fatal_reraise();
}
