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

// WEAK getenv declaration — used to locate the host touch-events socket
// via $TWOYI_ROOTFS (kr64 puts TWOYI_ROOTFS=<absolute host rootfs path> in
// the guest child env; {data_dir} = dirname(rootfs), so the app's socket
// is at $TWOYI_ROOTFS/../dev/touch-events). Like dlsym, WEAK so a bionic
// that can't resolve it leaves it NULL instead of failing the whole
// LD_PRELOAD load (the guest cwd fallback covers that case: the guest's
// cwd IS the rootfs, so "../dev/touch-events" resolves to the same file).
extern char *getenv(const char *name) __attribute__((weak));

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

// Virtual touch panel extents — MUST MATCH TWRP_FB_WIDTH/HEIGHT (320x640;
// the host SurfaceView feeds the same coordinate space, so we only CLAMP).
#define INBR_MAX_X               319
#define INBR_MAX_Y               639

// Kernel errno values (arch-independent) — we build with -nostdlib and
// must not depend on the host errno.h having been included consistently.
#define INBR_EAGAIN              11
#define INBR_EINTR                4

// i386 struct input_event = 16 bytes:
//   struct timeval time (u32 tv_sec + u32 tv_usec on 32-bit);
//   u16 type; u16 code; s32 value;
#define INBR_EV_SIZE             16
#define INBR_MSG_SIZE            20
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
    p[0] = 0; p[1] = 0; p[2] = 0; p[3] = 0;             /* tv_sec  = 0   */
    p[4] = 0; p[5] = 0; p[6] = 0; p[7] = 0;             /* tv_usec = 0   */
    p[8]  = (unsigned char)(type & 0xff);
    p[9]  = (unsigned char)((type >> 8) & 0xff);
    p[10] = (unsigned char)(code & 0xff);
    p[11] = (unsigned char)((code >> 8) & 0xff);
    p[12] = (unsigned char)((unsigned)value & 0xff);
    p[13] = (unsigned char)(((unsigned)value >> 8) & 0xff);
    p[14] = (unsigned char)(((unsigned)value >> 16) & 0xff);
    p[15] = (unsigned char)(((unsigned)value >> 24) & 0xff);
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
    char *cands[3];
    unsigned cand_lens[2];
    int ncands = 0;
    long fd, i;

    *init_len = 0;
    *out_slot = 0;

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
            if (pr > 0 && pr < 128) {
                long k;
                pbuf[pr] = 0;
                for (k = pr - 1; k >= 0; k--) {
                    if (pbuf[k] == '\n' || pbuf[k] == '\r' || pbuf[k] == ' ') pbuf[k] = 0;
                    else break;
                }
                for (k = 0; pbuf[k]; k++) rootbuf[k] = pbuf[k];
                rootbuf[k] = 0;
                if (k > 0 && rootbuf[0] == '/') {
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
            if (n > 0) { cands[ncands] = rootbuf; cand_lens[ncands] = n; ncands++; }
        }
    }
    /* candidate 1: relative ../dev/touch-events (guest cwd == rootfs) */
    {
        const char *rel = "../dev/touch-events";
        unsigned n = my_strlen(rel);
        unsigned j;
        my_memset(relbuf, 0, sizeof(relbuf));
        for (j = 0; j < n; j++) relbuf[j] = rel[j];
        cands[ncands] = relbuf; cand_lens[ncands] = n; ncands++;
    }

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
            for (j = 0; j < cand_lens[i] && j < 107; j++) {
                addr.sun_path[j] = cands[i][j];
            }
        }
        (void)inbr_unix_connect((int)fd, &addr, 2 + cand_lens[i]);
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
            real_read = (ssize_t (*)(int, void *, size_t))dlsym(RTLD_NEXT, "read");
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
            real_poll = (int (*)(struct pollfd *, unsigned long, int))dlsym(RTLD_NEXT, "poll");
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
            if (g_inbr_log_poll < 8) {
                g_inbr_log_poll++;
                write_str(2, "[twrp_fb_hook] INPUT poll -> ");
                write_num(2, ready);
                write_str(2, " ready (userspace, no raw poll syscall)\n");
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
        unsigned need = 8;                       /* bytes actually set */
        if (cap > 64) cap = 64;                  /* our bitmap budget */
        if (size > 4096) return -2;
        my_memset(argp, 0, size);
        if (ev == 0) {                           /* event-type bits */
            need = (0x1f /*EV_MAX*/ + 7) / 8;    /* = 4 */
            inbr_set_bit((unsigned char *)argp, cap, INBR_EV_SYN);
            inbr_set_bit((unsigned char *)argp, cap, INBR_EV_KEY);
            inbr_set_bit((unsigned char *)argp, cap, INBR_EV_ABS);
        } else if (ev == INBR_EV_KEY) {
            need = (0x2ff /*KEY_MAX*/ + 7) / 8;  /* = 96 */
            inbr_set_bit((unsigned char *)argp, cap, INBR_BTN_TOUCH);
            inbr_set_bit((unsigned char *)argp, cap, INBR_BTN_TOOL_FINGER);
        } else if (ev == INBR_EV_ABS) {
            need = (0x3f /*ABS_MAX*/ + 7) / 8;   /* = 8 */
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
        return (int)(size < need ? size : need);
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
#define TWRP_FB_WIDTH          320
#define TWRP_FB_HEIGHT         640
#define TWRP_FB_BPP            32
#define TWRP_FB_BYTES_PER_PIX  4
#define TWRP_FB_LINE_LENGTH    (TWRP_FB_WIDTH * TWRP_FB_BYTES_PER_PIX)  /* 1280 */
#define TWRP_FB_SMEM_LEN       (TWRP_FB_WIDTH * TWRP_FB_HEIGHT * TWRP_FB_BYTES_PER_PIX)  /* 819200 */

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
    int i;
    write_str(2, "[twrp_fb_hook] loaded (i686 LD_PRELOAD for /dev/graphics/fb0)\n");

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
    if (fd < 0 && is_fb_path(path)) {
        // Create /dev/graphics/ directory if needed
        mkdir_raw("/dev/graphics", 0755);
        // Create the fb0 file with the right size (320*640*4 = 819200)
        int create_fd = (int)raw_syscall4(SYS_openat, AT_FDCWD,
            (long)(my_strcmp(path, "/dev/fb0") == 0 ? "/dev/fb0" : "/dev/graphics/fb0"),
            O_CREAT | O_RDWR, 0644);
        if (create_fd >= 0) {
            // Truncate to framebuffer size
            raw_syscall3(SYS_ftruncate, create_fd, TWRP_FB_SMEM_LEN, 0);
            raw_syscall1(SYS_close, create_fd);
            // Re-open with the original flags
            fd = real_open ? real_open(path, flags, mode)
                           : (int)raw_syscall4(SYS_openat, AT_FDCWD, (long)path, flags, mode);
            write_str(2, "[twrp_fb_hook] created virtual fb0 file, re-opened -> fd=");
            write_num(2, fd);
            write_str(2, "\n");
        }
    }
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
            raw_syscall3(SYS_ftruncate, trunc_fd, TWRP_FB_SMEM_LEN, 0);
            raw_syscall1(SYS_close, trunc_fd);
            write_str(2, "[twrp_fb_hook] ftruncated existing fb0 to TWRP_FB_SMEM_LEN\n");
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
    {
        int in_fd = try_open_input_bridge(path);
        if (in_fd != -2) return in_fd;
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
            raw_syscall3(SYS_ftruncate, create_fd, TWRP_FB_SMEM_LEN, 0);
            raw_syscall1(SYS_close, create_fd);
            fd = real_openat ? real_openat(dirfd, path, flags, mode)
                             : (int)raw_syscall4(SYS_openat, dirfd, (long)path, flags, mode);
            write_str(2, "[twrp_fb_hook] created virtual fb0 file, re-opened -> fd=");
            write_num(2, fd);
            write_str(2, "\n");
        }
    }
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
            raw_syscall3(SYS_ftruncate, trunc_fd, TWRP_FB_SMEM_LEN, 0);
            raw_syscall1(SYS_close, trunc_fd);
            write_str(2, "[twrp_fb_hook] ftruncated existing fb0 to TWRP_FB_SMEM_LEN\n");
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
    {
        int in_fd = try_open_input_bridge(path);
        if (in_fd != -2) return in_fd;
    }
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
    {
        int in_fd = try_open_input_bridge(path);
        if (in_fd != -2) return in_fd;
    }
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
        if (!real_ioctl_in && dlsym) real_ioctl_in = (int (*)(int, int, ...))dlsym(RTLD_NEXT, "ioctl");
        if (real_ioctl_in) return real_ioctl_in(fd, request, argp);
        return (int)raw_syscall3(SYS_ioctl, fd, request, (long)argp);
    }

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
            // 6-Z84: log the ACTUAL configured values (the old hardcoded
            // "720x1280" string survived the 6-Z64 geometry fix and
            // misdirected run analysis for hours — TWRP's own prints had
            // the truth: 320 x 640).
            write_str(2, "[twrp_fb_hook] ioctl(FBIOGET_VSCREENINFO) -> ");
            write_num(2, TWRP_FB_WIDTH); write_str(2, "x"); write_num(2, TWRP_FB_HEIGHT);
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
            write_num(2, TWRP_FB_SMEM_LEN);
            write_str(2, " line_length=");
            write_num(2, TWRP_FB_LINE_LENGTH);
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
