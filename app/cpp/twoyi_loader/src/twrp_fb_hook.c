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
//   - mmap() — for fb0 fds, fall back to MAP_ANONYMOUS if the real mmap
//     fails (some callers use MAP_SHARED which requires the file to be
//     writable; our regular file is writable so this usually isn't
//     needed, but it's a safety net).
//
// WHAT THIS DOES NOT HOOK:
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
#include <dlfcn.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <linux/fb.h>

// ---------------------------------------------------------------------------
// RAW i386 SYSCALL HELPERS — we use inline `int $0x80` instead of calling
// libc's `syscall()` function. This is CRITICAL: TWRP's bionic linker
// (AOSP 5.1) fails to resolve the `syscall` symbol from our LD_PRELOAD
// library even though libc.so exports it (strace-confirmed in KVM run
// 31572816370: "CANNOT LINK EXECUTABLE DEPENDENCIES: cannot locate
// symbol \"syscall\" referenced by \"twrp_fb_hook.so\"..."). Using inline
// asm eliminates the undefined `syscall` symbol from our .so's dynsym,
// so bionic can load us.
//
// i386 syscall convention (kernel sigreturn ABI):
//   eax = syscall number
//   ebx = arg1, ecx = arg2, edx = arg3
//   esi = arg4, edi = arg5, ebp = arg6
//   int $0x80
//   eax = return value (negative errno on error)
//
// We only need 1/3/4-arg variants (no 6-arg syscalls after removing the
// dead SYS_mmap2 fallback in mmap()). For the 6-arg case ebp would have
// to be saved/restored (it's the frame pointer); we avoid that entirely.
// ---------------------------------------------------------------------------
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
    if (strcmp(path, "/dev/graphics/fb0") == 0) return 1;
    if (strcmp(path, "/dev/fb0") == 0) return 1;
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
// mmap() on the fd works naturally (no mmap hook needed for the common
// case). The mmap hook below is a safety net for callers that use
// MAP_SHARED with unusual flags.
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
    memset(v, 0, sizeof(*v));
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
    memset(f, 0, sizeof(*f));
    // id is a 16-byte char array (kernel: char id[16]).
    // Use a short null-terminated string; the rest stays zeroed.
    strncpy(f->id, "twoyi_fb", sizeof(f->id) - 1);
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
// ---------------------------------------------------------------------------
__attribute__((constructor))
static void twrp_fb_hook_init(void) {
    write_str(2, "[twrp_fb_hook] loaded (i686 LD_PRELOAD for /dev/graphics/fb0)\n");
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
    if (!real_open)    real_open    = dlsym(RTLD_NEXT, "open");
    if (!real_openat)  real_openat  = dlsym(RTLD_NEXT, "openat");
}

int open(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
    init_real_funcs();
    int fd = real_open ? real_open(path, flags, mode)
                       : (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)path, flags, mode);
    if (fd >= 0 && is_fb_path(path)) {
        fb_fd_mark(fd);
        write_str(2, "[twrp_fb_hook] open(");
        write_str(2, path);
        write_str(2, ") -> fd=");
        write_num(2, fd);
        write_str(2, " (tracking for FB ioctls)\n");
    }
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
    if (fd >= 0 && is_fb_path(path)) {
        fb_fd_mark(fd);
        write_str(2, "[twrp_fb_hook] openat(");
        write_str(2, path);
        write_str(2, ") -> fd=");
        write_num(2, fd);
        write_str(2, " (tracking for FB ioctls)\n");
    }
    return fd;
}

// bionic's fortified open variants. These are called by code compiled with
// -D_FORTIFY_SOURCE (most of AOSP). They have the same path-tracking logic.
int __open_2(const char *path, int flags) {
    init_real_funcs();
    static int (*real_open2)(const char *, int) = NULL;
    if (!real_open2) real_open2 = dlsym(RTLD_NEXT, "__open_2");
    int fd;
    if (real_open2) fd = real_open2(path, flags);
    else            fd = (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)path, flags, 0);
    if (fd >= 0 && is_fb_path(path)) {
        fb_fd_mark(fd);
        write_str(2, "[twrp_fb_hook] __open_2(");
        write_str(2, path);
        write_str(2, ") -> fd=");
        write_num(2, fd);
        write_str(2, " (tracking for FB ioctls)\n");
    }
    return fd;
}

int __openat_2(int dirfd, const char *path, int flags) {
    init_real_funcs();
    static int (*real_openat2)(int, const char *, int) = NULL;
    if (!real_openat2) real_openat2 = dlsym(RTLD_NEXT, "__openat_2");
    int fd;
    if (real_openat2) fd = real_openat2(dirfd, path, flags);
    else              fd = (int)raw_syscall4(SYS_openat, dirfd, (long)path, flags, 0);
    if (fd >= 0 && is_fb_path(path)) {
        fb_fd_mark(fd);
        write_str(2, "[twrp_fb_hook] __openat_2(");
        write_str(2, path);
        write_str(2, ") -> fd=");
        write_num(2, fd);
        write_str(2, " (tracking for FB ioctls)\n");
    }
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
    if (!real_close) real_close = dlsym(RTLD_NEXT, "close");
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
// bionic: int ioctl(int, int, ...)
// glibc:  int ioctl(int, unsigned long, ...)
// We match bionic's signature since this runs in the recovery process.
int ioctl(int fd, int request, ...) {
    va_list ap;
    va_start(ap, request);
    void *argp = va_arg(ap, void *);
    va_end(ap);

    unsigned req = (unsigned)request;

    // Fast path: not an fb0 fd, pass through.
    if (!fb_fd_is_tracked(fd)) {
        static int (*real_ioctl)(int, int, ...) = NULL;
        if (!real_ioctl) real_ioctl = dlsym(RTLD_NEXT, "ioctl");
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
            if (!real_ioctl) real_ioctl = dlsym(RTLD_NEXT, "ioctl");
            if (real_ioctl) return real_ioctl(fd, request, argp);
            return (int)raw_syscall3(SYS_ioctl, fd, request, (long)argp);
        }
    }
}

// ---------------------------------------------------------------------------
// mmap() PLT interposition — safety net for fb0 fds.
//
// libminuitwrp does:
//   bits = mmap(0, fi.smem_len, PROT_READ|PROT_WRITE, MAP_SHARED, fd, 0)
//
// On a regular file of size 3,686,400 bytes, this should succeed. But if
// for some reason it fails (e.g. the file wasn't pre-allocated to the
// right size, or the caller used unusual flags), we fall back to
// MAP_ANONYMOUS so the caller gets a writable mapping and doesn't crash.
// The mapping won't be backed by the file, but for TWRP's software
// renderer the framebuffer memory just needs to be writable — the pixels
// are never displayed on a real screen.
// ---------------------------------------------------------------------------
void *mmap(void *addr, size_t length, int prot, int flags, int fd, off_t offset) {
    static void *(*real_mmap)(void *, size_t, int, int, int, off_t) = NULL;
    if (!real_mmap) real_mmap = dlsym(RTLD_NEXT, "mmap");

    // Try the real mmap first.
    if (real_mmap) {
        void *result = real_mmap(addr, length, prot, flags, fd, offset);
        if (result != MAP_FAILED) return result;
    }

    // mmap failed — if this is an fb0 fd, fall back to MAP_ANONYMOUS.
    if (fb_fd_is_tracked(fd)) {
        write_str(2, "[twrp_fb_hook] mmap on fb0 fd failed -> MAP_ANONYMOUS fallback\n");
        if (real_mmap) {
            return real_mmap(addr, length, prot, flags | MAP_ANONYMOUS, -1, 0);
        }
        // real_mmap is NULL (dlsym failed — shouldn't happen, bionic always
        // has mmap in libc.so). We can't do a raw 6-arg SYS_mmap2 syscall
        // here without saving/restoring ebp (the frame pointer on i386).
        // Rather than risk corrupting the caller's frame, return MAP_FAILED
        // and let the caller handle the error.
    }

    return MAP_FAILED;
}
