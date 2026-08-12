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
//     (720x1280 @ 32bpp, RGBA8888). This is the FIX for the libminuitwrp
//     segfault at offset 0x57d7 (NULL deref after FBIOGET_VSCREENINFO
//     returned ENOTTY on /dev/null and the struct stayed zeroed).
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
// referenced by 'twrp_fb_hook.so'" (strace-confirmed in KVM runs 31574428304
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
// referenced by \"twrp_fb_hook.so\"..."). Using inline asm eliminates the
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

#else
#error "twrp_fb_hook.c: unsupported architecture (need __i386__ or __aarch64__)"
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
// Virtual screen configuration.
//
// 720x1280 @ 32bpp (RGBA8888). This matches the byt_t_crv2 (Minix Z64)
// TWRP image's expected display resolution. libminuitwrp reads these
// values from FBIOGET_VSCREENINFO and uses them to size its framebuffer.
//
// The framebuffer memory is 720*1280*4 = 3,686,400 bytes. kr64 pre-
// creates /dev/graphics/fb0 as a regular file of exactly this size, so
// mmap() on the fd works naturally via bionic's native mmap() — no
// mmap hook needed (see the "mmap() — NOT HOOKED" comment at the
// bottom of this file for why the previous safety-net hook was removed).
// ---------------------------------------------------------------------------
#define TWRP_FB_WIDTH          720
#define TWRP_FB_HEIGHT         1280
#define TWRP_FB_BPP            32
#define TWRP_FB_BYTES_PER_PIX  4
#define TWRP_FB_LINE_LENGTH    (TWRP_FB_WIDTH * TWRP_FB_BYTES_PER_PIX)  /* 2880 */
#define TWRP_FB_SMEM_LEN       (TWRP_FB_WIDTH * TWRP_FB_HEIGHT * TWRP_FB_BYTES_PER_PIX)  /* 3686400 */

// FB_ACTIVATE_NOW = 0 (see linux/fb.h)
#define TWRP_FB_ACTIVATE_NOW   0
// FB_TYPE_PACKED_PIXELS = 0
#define TWRP_FB_TYPE_PACKED    0
// FB_VISUAL_TRUECOLOR = 2
#define TWRP_FB_VISUAL_TRUECOLOR 2

static void fill_vscreeninfo(struct fb_var_screeninfo *v) {
    my_memset(v, 0, sizeof(*v));
    v->xres = TWRP_FB_WIDTH;
    v->yres = TWRP_FB_HEIGHT;
    v->xres_virtual = TWRP_FB_WIDTH;
    v->yres_virtual = TWRP_FB_HEIGHT;
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
    // Physical dimensions in mm (for DPI calculation). 720x1280 at
    // ~250 DPI is ~73x130mm — we use round numbers close to a 5" phone.
    v->height = 130;
    v->width = 73;
    v->accel_flags = 0;
    // Pixclock in picoseconds. For 60Hz refresh of 720x1280:
    //   pixclock = 1 / (60 * 720 * 1280) = ~18ns = ~18100ps
    // libminuitwrp doesn't use this for the software renderer, but we
    // provide a sane value anyway.
    v->pixclock = 18100;
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
    f->smem_len = TWRP_FB_SMEM_LEN;
    f->type = TWRP_FB_TYPE_PACKED;
    f->type_aux = 0;
    f->visual = TWRP_FB_VISUAL_TRUECOLOR;
    f->xpanstep = 0;
    f->ypanstep = 0;
    f->ywrapstep = 0;
    f->line_length = TWRP_FB_LINE_LENGTH;
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

__attribute__((constructor))
static void twrp_fb_hook_init(void) {
    write_str(2, "[twrp_fb_hook] loaded (i686 LD_PRELOAD for /dev/graphics/fb0)\n");
    // Log hook function addresses to confirm they're defined and to
    // correlate with any future PLT-resolution diagnostics. These are
    // the addresses of OUR definitions; if bionic's linker resolves
    // libminuitwrp's `open` PLT entry to a DIFFERENT address, that
    // would explain why our hook isn't being called.
    write_str(2, "[twrp_fb_hook] addrs: open@"); write_hex(2, (unsigned int)(uintptr_t)&open);
    write_str(2, " openat@"); write_hex(2, (unsigned int)(uintptr_t)&openat);
    write_str(2, " __open_2@"); write_hex(2, (unsigned int)(uintptr_t)&__open_2);
    write_str(2, " __openat_2@"); write_hex(2, (unsigned int)(uintptr_t)&__openat_2);
    write_str(2, " close@"); write_hex(2, (unsigned int)(uintptr_t)&close);
    write_str(2, " ioctl@"); write_hex(2, (unsigned int)(uintptr_t)(int(*)(int,int,...))&ioctl);
    write_str(2, "\n");
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
    if (!real_open   && dlsym) real_open   = (int (*)(const char *, int, ...))dlsym(RTLD_NEXT, "open");
    if (!real_openat && dlsym) real_openat = (int (*)(int, const char *, int, ...))dlsym(RTLD_NEXT, "openat");
}

int open(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
    init_real_funcs();
    int fd = real_open ? real_open(path, flags, mode)
                       : (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)path, flags, mode);
    // If opening /dev/graphics/fb0 or /dev/fb0 fails with ENOENT, create
    // the virtual framebuffer file and re-open it. TWRP init may re-mount
    // /dev tmpfs, wiping kr64's pre-created fb0 file.
    if (fd < 0 && is_fb_path(path)) {
        // Create /dev/graphics/ directory if needed
        mkdir_raw("/dev/graphics", 0755);
        // Create the fb0 file with the right size (720*1280*4 = 3686400)
        int create_fd = (int)raw_syscall4(SYS_openat, AT_FDCWD,
            (long)(my_strcmp(path, "/dev/fb0") == 0 ? "/dev/fb0" : "/dev/graphics/fb0"),
            O_CREAT | O_RDWR, 0644);
        if (create_fd >= 0) {
            // Truncate to framebuffer size
            raw_syscall3(SYS_ftruncate, create_fd, 3686400, 0);
            raw_syscall1(SYS_close, create_fd);
            // Re-open with the original flags
            fd = real_open ? real_open(path, flags, mode)
                           : (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)path, flags, mode);
            write_str(2, "[twrp_fb_hook] created virtual fb0 file, re-opened -> fd=");
            write_num(2, fd);
            write_str(2, "\n");
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
    write_str(2, "\n");
    return fd;
}

int openat(int dirfd, const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
    init_real_funcs();
    int fd = real_openat ? real_openat(dirfd, path, flags, mode)
                         : (int)raw_syscall4(SYS_openat, dirfd, (long)path, flags, mode);
    // If opening /dev/graphics/fb0 or /dev/fb0 fails with ENOENT, create
    // the virtual framebuffer file and re-open it.
    if (fd < 0 && is_fb_path(path)) {
        mkdir_raw("/dev/graphics", 0755);
        int create_fd = (int)raw_syscall4(SYS_openat, AT_FDCWD,
            (long)(my_strcmp(path, "/dev/fb0") == 0 ? "/dev/fb0" : "/dev/graphics/fb0"),
            O_CREAT | O_RDWR, 0644);
        if (create_fd >= 0) {
            raw_syscall3(SYS_ftruncate, create_fd, 3686400, 0);
            raw_syscall1(SYS_close, create_fd);
            fd = real_openat ? real_openat(dirfd, path, flags, mode)
                             : (int)raw_syscall4(SYS_openat, dirfd, (long)path, flags, mode);
            write_str(2, "[twrp_fb_hook] created virtual fb0 file, re-opened -> fd=");
            write_num(2, fd);
            write_str(2, "\n");
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
    write_str(2, "\n");
    return fd;
}

// bionic's fortified open variants. These are called by code compiled with
// -D_FORTIFY_SOURCE (most of AOSP). They have the same path-tracking logic.
int __open_2(const char *path, int flags) {
    init_real_funcs();
    static int (*real_open2)(const char *, int) = NULL;
    if (!real_open2 && dlsym) real_open2 = (int (*)(const char *, int))dlsym(RTLD_NEXT, "__open_2");
    int fd;
    if (real_open2) fd = real_open2(path, flags);
    else            fd = (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)path, flags, 0);
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
    write_str(2, "\n");
    return fd;
}

int __openat_2(int dirfd, const char *path, int flags) {
    init_real_funcs();
    static int (*real_openat2)(int, const char *, int) = NULL;
    if (!real_openat2 && dlsym) real_openat2 = (int (*)(int, const char *, int))dlsym(RTLD_NEXT, "__openat_2");
    int fd;
    if (real_openat2) fd = real_openat2(dirfd, path, flags);
    else              fd = (int)raw_syscall4(SYS_openat, dirfd, (long)path, flags, 0);
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
    write_str(2, "\n");
    return fd;
}

// ---------------------------------------------------------------------------
// close() PLT interposition — clear fd tracking when an fb0 fd is closed.
// ---------------------------------------------------------------------------
int close(int fd) {
    if (fb_fd_is_tracked(fd)) {
        fb_fd_clear(fd);
        write_str(2, "[twrp_fb_hook] close(fd=");
        write_num(2, fd);
        write_str(2, ") (was tracked fb0 fd)\n");
    }
    static int (*real_close)(int) = NULL;
    if (!real_close && dlsym) real_close = (int (*)(int))dlsym(RTLD_NEXT, "close");
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
int ioctl(int fd, int request, ...) {
    va_list ap;
    va_start(ap, request);
    void *argp = va_arg(ap, void *);
    va_end(ap);

    unsigned req = (unsigned)request;

    // DIAGNOSTIC (Task 31): log EVERY ioctl() call to verify our hook is
    // being invoked. Key ioctl numbers to watch for:
    //   FBIOGET_VSCREENINFO = 0x4600  (libminuitwrp reads screen size)
    //   FBIOGET_FSCREENINFO = 0x4602  (libminuitwrp reads smem_len for mmap)
    //   FBIOPUT_VSCREENINFO = 0x4601
    //   FBIOPAN_DISPLAY     = 0x4606
    //   FBIOBLANK            = 0x4611
    // If our hook IS being called but recovery still segfaults, the issue
    // is in our ioctl handling (wrong struct size, wrong values, etc.).
    // If our hook is NOT being called, the issue is PLT interception.
    {
        int tracked = fb_fd_is_tracked(fd);
        write_str(2, "[twrp_fb_hook] ioctl(fd="); write_num(2, fd);
        write_str(2, ", req=0x"); write_hex(2, req);
        write_str(2, ") [trk="); write_num(2, tracked); write_str(2, "]\n");
    }

    // Fast path: not an fb0 fd, pass through.
    if (!fb_fd_is_tracked(fd)) {
        static int (*real_ioctl)(int, int, ...) = NULL;
        if (!real_ioctl && dlsym) real_ioctl = (int (*)(int, int, ...))dlsym(RTLD_NEXT, "ioctl");
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
            write_str(2, "[twrp_fb_hook] ioctl(FBIOGET_VSCREENINFO) -> 720x1280@32bpp\n");
            return 0;
        }
        case 0x4601u: {  // FBIOPUT_VSCREENINFO — accept the mode change
            write_str(2, "[twrp_fb_hook] ioctl(FBIOPUT_VSCREENINFO) -> success\n");
            return 0;
        }
        case 0x4602u: {  // FBIOGET_FSCREENINFO
            if (argp) fill_fscreeninfo((struct fb_fix_screeninfo *)argp);
            write_str(2, "[twrp_fb_hook] ioctl(FBIOGET_FSCREENINFO) -> smem_len=3686400 line_length=2880\n");
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
            if (!real_ioctl && dlsym) real_ioctl = (int (*)(int, int, ...))dlsym(RTLD_NEXT, "ioctl");
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
