/* libbionic_compat.c — Twoyi bionic FORTIFY-compat shim (Task 6-Z236).
 *
 * WHY: Task 6-Z230 stages MISSING DT_NEEDED libraries (libcrypto.so,
 * libssl.so, ...) from the HOST Android runtime into the guest rootfs
 * sbin. The host runtime's libraries are built against the HOST's bionic
 * libc with FORTIFY_SOURCE enabled: their undefined-symbol set references
 * the FORTIFY wrapper family (__write_chk, __read_chk, __memmove_chk, ...)
 * which lives in the HOST libc.so. The GUEST recovery's own libc.so (an
 * older bionic generation) frequently does NOT export those symbols, so
 * the guest linker fails with:
 *
 *   CANNOT LINK EXECUTABLE: cannot locate symbol "__write_chk"
 *   referenced by ".../sbin/libcrypto.so"...
 *
 * (cherry, run 33306474686 recovery-ld-debug.txt). The guest cannot be
 * patched (no device-specific hacks, §22) and the host libc must never be
 * staged wholesale (private-ABI mismatch class, §9/§22).
 *
 * MECHANISM: this file builds a small -nostdlib shared object that
 * EXPORTS the common FORTIFY symbol family and implements each one
 * directly on raw syscalls / inline string operations — no libc
 * dependency at all. The parent stages it as {rootfs}/sbin/
 * libbionic_compat.so (guest-arch-matched build) and prepends it to the
 * recovery service's LD_PRELOAD chain. bionic loads LD_PRELOAD libraries
 * BEFORE the main executable's DT_NEEDED dependencies, so these symbols
 * satisfy later relocations of the host-staged libraries. The shim is
 * inert for guests that don't need it (extra exports are harmless).
 *
 * Deliberately NOT covered (documented in the worklog): the format-family
 * FORTIFY wrappers (__snprintf_chk etc.) — they need a full vsnprintf,
 * which a -nostdlib shim cannot realistically carry. If a corpus image
 * needs them, extend this file.
 *
 * Every implementation is bounded and follows the FORTIFY contract:
 * check the caller-supplied object size against the requested length and
 * trap (null write → SIGSEGV, the no-libc abort) on overflow, otherwise
 * forward to the raw syscall / plain loop.
 */

#include <stdint.h>
#include <stddef.h>

/* Freestanding type shims (no libc headers under -nostdlib). */
typedef long ssize_t_;      /* 32-bit on arm32/i386, 64-bit on arm64/x86_64 */
typedef unsigned int socklen_t_;

/* ── raw syscall shims per architecture ─────────────────────────────── */

#if defined(__aarch64__)

static long sys_call3(long nr, long a, long b, long c) {
    register long x8 __asm__("x8") = nr;
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    register long x2 __asm__("x2") = c;
    __asm__ volatile("svc #0"
                     : "=r"(x0)
                     : "r"(x8), "r"(x0), "r"(x1), "r"(x2)
                     : "memory", "cc");
    return x0;
}

static long sys_recvfrom6(long fd, void* buf, size_t n, long flags, void* src,
                          void* addrlen) {
    register long x8 __asm__("x8") = 207; /* __NR_recvfrom */
    register long x0 __asm__("x0") = fd;
    register long x1 __asm__("x1") = (long)buf;
    register long x2 __asm__("x2") = (long)n;
    register long x3 __asm__("x3") = flags;
    register long x4 __asm__("x4") = (long)src;
    register long x5 __asm__("x5") = (long)addrlen;
    __asm__ volatile("svc #0"
                     : "=r"(x0)
                     : "r"(x8), "r"(x0), "r"(x1), "r"(x2), "r"(x3), "r"(x4), "r"(x5)
                     : "memory", "cc");
    return x0;
}

static long sys_write3(long fd, const void* buf, size_t n) {
    return sys_call3(64, fd, (long)buf, (long)n);
}
static long sys_read3(long fd, void* buf, size_t n) {
    return sys_call3(63, fd, (long)buf, (long)n);
}

#elif defined(__arm__)

static long sys_call4(long nr, long a, long b, long c, long d) {
    register long r7 __asm__("r7") = nr;
    register long r0 __asm__("r0") = a;
    register long r1 __asm__("r1") = b;
    register long r2 __asm__("r2") = c;
    register long r3 __asm__("r3") = d;
    __asm__ volatile("svc #0"
                     : "=r"(r0)
                     : "r"(r7), "r"(r0), "r"(r1), "r"(r2), "r"(r3)
                     : "memory", "cc");
    return r0;
}

static long sys_call6(long nr, long a, long b, long c, long d, long e, long f) {
    register long r7 __asm__("r7") = nr;
    register long r0 __asm__("r0") = a;
    register long r1 __asm__("r1") = b;
    register long r2 __asm__("r2") = c;
    register long r3 __asm__("r3") = d;
    register long r4 __asm__("r4") = e;
    register long r5 __asm__("r5") = f;
    __asm__ volatile("svc #0"
                     : "=r"(r0)
                     : "r"(r7), "r"(r0), "r"(r1), "r"(r2), "r"(r3), "r"(r4), "r"(r5)
                     : "memory", "cc");
    return r0;
}

static long sys_write3(long fd, const void* buf, size_t n) {
    return sys_call4(4, fd, (long)buf, (long)n, 0);
}
static long sys_read3(long fd, void* buf, size_t n) {
    return sys_call4(3, fd, (long)buf, (long)n, 0);
}
static long sys_recvfrom6(long fd, void* buf, size_t n, long flags, void* src,
                          void* addrlen) {
    /* arm EABI: __NR_recvfrom = 291 */
    return sys_call6(291, fd, (long)buf, (long)n, flags, (long)src, (long)addrlen);
}

#elif defined(__x86_64__)

static long sys_call3(long nr, long a, long b, long c) {
    long ret;
    __asm__ volatile("syscall"
                     : "=a"(ret)
                     : "a"(nr), "D"(a), "S"(b), "d"(c)
                     : "rcx", "r11", "memory");
    return ret;
}

static long sys_recvfrom6(long fd, void* buf, size_t n, long flags, void* src,
                          void* addrlen) {
    long ret;
    register long r10 __asm__("r10") = flags;
    register long r8 __asm__("r8") = (long)src;
    register long r9 __asm__("r9") = (long)addrlen;
    __asm__ volatile("syscall"
                     : "=a"(ret)
                     : "a"(45), "D"(fd), "S"(buf), "d"((long)n), "r"(r10), "r"(r8),
                       "r"(r9)
                     : "rcx", "r11", "memory");
    return ret;
}

static long sys_write3(long fd, const void* buf, size_t n) {
    return sys_call3(1, fd, (long)buf, (long)n);
}
static long sys_read3(long fd, void* buf, size_t n) {
    return sys_call3(0, fd, (long)buf, (long)n);
}

#elif defined(__i386__)

/* i386 PIC: ebx holds the GOT, so it must be saved/restored around int 80. */
static long sys_call3(long nr, long a, long b, long c) {
    long ret;
    __asm__ volatile("pushl %%ebx\n\t"
                     "movl %2, %%ebx\n\t"
                     "int $0x80\n\t"
                     "popl %%ebx"
                     : "=a"(ret)
                     : "a"(nr), "m"(a), "c"(b), "d"(c)
                     : "memory", "cc");
    return ret;
}

static long sys_write3(long fd, const void* buf, size_t n) {
    return sys_call3(4, fd, (long)buf, (long)n);
}
static long sys_read3(long fd, void* buf, size_t n) {
    return sys_call3(3, fd, (long)buf, (long)n);
}

#else
#error "unsupported architecture for libbionic_compat"
#endif

/* FORTIFY semantics: on detected overflow the compiler-provided runtime
 * aborts. Without libc we trap via a null write (SIGSEGV). */
static void fortify_abort(void) {
    *(volatile long*)0 = 0;
    for (;;)
        ;
}

/* ── FORTIFY wrapper family ─────────────────────────────────────────── */

ssize_t_ __write_chk(int fd, const void* buf, size_t count, size_t buf_size) {
    if (count > buf_size) fortify_abort();
    return (ssize_t_)sys_write3(fd, buf, count);
}

ssize_t_ __read_chk(int fd, void* buf, size_t count, size_t buf_size) {
    if (count > buf_size) fortify_abort();
    return (ssize_t_)sys_read3(fd, buf, count);
}

#if defined(__aarch64__) || defined(__arm__) || defined(__x86_64__)
ssize_t_ __recvfrom_chk(int fd, void* buf, size_t len, long flags, void* src_addr,
                        socklen_t_* addrlen, size_t buf_size) {
    if (len > buf_size) fortify_abort();
    return (ssize_t_)sys_recvfrom6(fd, buf, len, flags, src_addr, (void*)addrlen);
}
#endif /* i386 recvfrom needs socketcall — omitted (rare in staged libs) */

void* __memmove_chk(void* dst, const void* src, size_t len, size_t dst_len) {
    unsigned char* d = (unsigned char*)dst;
    const unsigned char* s = (const unsigned char*)src;
    if (len > dst_len) fortify_abort();
    if (d == s || len == 0) return dst;
    if (d < s) {
        for (size_t i = 0; i < len; i++) d[i] = s[i];
    } else {
        for (size_t i = len; i > 0; i--) d[i - 1] = s[i - 1];
    }
    return dst;
}

void* __memcpy_chk(void* dst, const void* src, size_t len, size_t dst_len) {
    unsigned char* d = (unsigned char*)dst;
    const unsigned char* s = (const unsigned char*)src;
    if (len > dst_len) fortify_abort();
    for (size_t i = 0; i < len; i++) d[i] = s[i];
    return dst;
}

void* __memset_chk(void* dst, int ch, size_t len, size_t dst_len) {
    unsigned char* d = (unsigned char*)dst;
    if (len > dst_len) fortify_abort();
    for (size_t i = 0; i < len; i++) d[i] = (unsigned char)ch;
    return dst;
}

char* __strcpy_chk(char* dst, const char* src, size_t dst_len) {
    char* d = dst;
    const char* s = src;
    if (dst_len == 0) fortify_abort();
    while ((*d++ = *s++) != '\0') {
        if ((size_t)(d - dst) >= dst_len) fortify_abort();
    }
    return dst;
}

char* __strncpy_chk(char* dst, const char* src, size_t n, size_t dst_len) {
    char* d = dst;
    const char* s = src;
    if (n > dst_len) fortify_abort();
    while (n > 0 && (*d++ = *s++) != '\0') n--;
    while (n > 0) {
        *d++ = '\0';
        n--;
    }
    return dst;
}

char* __strcat_chk(char* dst, const char* src, size_t dst_len) {
    char* d = dst;
    const char* s = src;
    while (*d != '\0') {
        if ((size_t)(d - dst) >= dst_len) fortify_abort();
        d++;
    }
    while ((*d++ = *s++) != '\0') {
        if ((size_t)(d - dst) >= dst_len) fortify_abort();
    }
    return dst;
}

char* __strncat_chk(char* dst, const char* src, size_t n, size_t dst_len) {
    char* d = dst;
    const char* s = src;
    while (*d != '\0') {
        if ((size_t)(d - dst) >= dst_len) fortify_abort();
        d++;
    }
    while (n > 0) {
        if ((*d = *s++) == '\0') break;
        d++;
        n--;
        if ((size_t)(d - dst) >= dst_len) fortify_abort();
    }
    *d = '\0';
    return dst;
}

size_t __strlen_chk(const char* s, size_t s_len) {
    size_t n = 0;
    while (s[n] != '\0') {
        n++;
        if (n > s_len) fortify_abort();
    }
    return n;
}
