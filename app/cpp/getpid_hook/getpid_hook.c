// getpid_hook.c — EXPERIMENTAL/DIAGNOSTIC LD_PRELOAD library.
//
// ⚠️  EXPERIMENTAL — NOT THE ARCHITECTURAL SOLUTION  ⚠️
//
// This file is a DIAGNOSTIC TOOL, not the real virtualization layer.
// It was created to isolate WHERE the init boot fails, not to make
// init "boot" correctly. A process that returns 0 from every syscall
// is not a booted Android system — it's a process that thinks it
// booted but has no actual virtualization.
//
// WHY THIS EXISTS (diagnostic value only):
//   - Confirms whether the linker can load init at all (yes, in
//     no-namespaces mode the HOST linker loads init fine).
//   - Confirms WHICH syscalls init calls during FirstStageMain
//     (mount, mkdir, mknod, setgroups — all observed failing).
//   - Confirms that fake-success makes init progress PAST
//     FirstStageMain to SecondStageMain (the next failure point).
//
// WHY THIS IS NOT THE SOLUTION:
//
// 1. LD_PRELOAD is IGNORED for Android apps. Bionic's linker only
//    honors LD_PRELOAD when !getauxval(AT_SECURE). App processes
//    run with AT_SECURE set, so LD_PRELOAD is silently dropped.
//    Source: bionic/linker/linker_main.cpp (the `if (!getauxval(AT_SECURE))`
//    block around line 315). This library only works in the KVM test
//    environment where we launch kr64 as root via `adb shell`, NOT
//    in the production app context.
//
// 2. PLT interposition only catches calls through libc wrappers.
//    Direct `svc` instructions (static code, JIT, hand-coded asm)
//    bypass these hooks entirely. VM's libkr64.so uses shadowhook
//    (inline hooks) + a seccomp/SIGSYS backstop to catch ALL paths.
//
// 3. The zygote's seccomp filter KILLS mount() and chroot() with
//    SECCOMP_RET_KILL_PROCESS. SECCOMP_RET_TRAP from our own filter
//    CANNOT override KILL_PROCESS (precedence: KILL > TRAP > ERRNO).
//    Source: kernel docs userspace-api/seccomp_filter.html, and
//    bionic/libc/SECCOMP_BLACKLIST_APP.TXT (mount/chroot blacklisted).
//    So these syscalls MUST be intercepted at the inline-hook layer
//    (before the svc fires), not at the seccomp layer.
//
// 4. Fake-success hides required failures. Init's FirstStageMain
//    expects mount() to create a FRESH tmpfs on /dev, then mkdir's
//    /dev/pts, /dev/socket into it. If mount() returns 0 but does
//    nothing, the mkdir's operate on the HOST's /dev (which already
//    has these dirs). Init's /proc/cmdline read returns the HOST's
//    cmdline (wrong androidboot.* args). Init's /dev/kmsg open
//    opens the HOST's /dev/kmsg (wrong log context). Each fake
//    success compounds into wrong state that breaks later boot
//    stages in subtle, hard-to-debug ways.
//
// THE REAL ARCHITECTURAL SOLUTION (per research):
//
// Twoyi/VM/Nogitsune all use the SAME architecture:
//
//   1. Custom ELF interpreter (libloader.so / libkrloader64.so)
//      - Built from AOSP bionic/linker/ source
//      - Guest binaries have PT_INTERP → this custom linker
//      - Runs BEFORE guest main(), so it can install hooks
//      - Source: bionic/linker/linker_main.cpp
//
//   2. Inline hooks (shadowhook v1.0.8 — ByteDance's library)
//      - Hooks libc syscall wrappers (mount, chroot, pivot_root,
//        mknod, openat, stat, etc.) at the PLT/inline level
//      - The svc instruction NEVER fires for hooked syscalls
//      - This is the ONLY way to intercept mount/chroot/pivot_root
//        (which are KILL_PROCESS in the zygote's seccomp filter)
//      - Source: https://github.com/bytedance/android-inline-hook
//
//   3. Seccomp-BPF filter + SIGSYS handler (backstop)
//      - Catches syscalls that bypass libc (direct svc, JIT, etc.)
//      - Only works for syscalls the zygote ALLOWs (mknodat, unshare)
//      - Cannot catch mount/chroot (KILL_PROCESS shadows TRAP)
//      - Handler reads args from ucontext_t, emulates, writes
//        return value via PutValueInUcontext
//      - Source: Chromium sandbox/linux/seccomp-bpf/trap.cc
//      - Kernel docs: userspace-api/seccomp_filter.html
//
//   4. Path translation (virtual rootfs)
//      - openat("/dev/foo") → openat("{TWOYI_ROOTFS}/dev/foo")
//      - stat("/proc/cmdline") → return guest's cmdline
//      - This provides actual filesystem isolation, not fake mounts
//
//   5. Patched goldfish init (guest ROM)
//      - Stock AOSP init cannot be driven by userspace shims alone
//      - The guest ROM must be a patched goldfish (emulator) image
//        that skips first_stage_mount and real SELinux load
//      - twoyi ships this as the closed-source "ROM" component
//
// This file will be REPLACED by the custom linker + shadowhook
// implementation. It is kept ONLY as a diagnostic tool for the KVM
// test environment (where we can run as root and LD_PRELOAD works).
//
// DO NOT build production features on top of these fake stubs.
// DO NOT confuse "init stops crashing" with "the system boots".
//
// See: docs/ROOTLESS_VIRTUALIZATION_ARCHITECTURE.md (to be written)
// Research sources: see worklog.md task "rootless-ld-preload-breakthrough"

#include <unistd.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/types.h>

// =========================================================================
// EXPERIMENTAL DIAGNOSTIC HOOKS — see warning above
// =========================================================================
// These hooks make init PROGRESS past FirstStageMain so we can observe
// the NEXT failure point. They do NOT provide real virtualization.

pid_t getpid(void) {
    return 1;
}

pid_t getppid(void) {
    return 0;
}

int mount(const char *source, const char *target,
          const char *filesystemtype, unsigned long mountflags,
          const void *data) {
    // DIAGNOSTIC: fake success. Real implementation needs inline hook
    // (shadowhook) that either no-ops or redirects to path translation.
    return 0;
}

int umount(const char *target) { return 0; }
int umount2(const char *target, int flags) { return 0; }

int chroot(const char *path) {
    // DIAGNOSTIC: fake success. Real implementation needs path-prefix
    // virtualization (translate "/foo" → "{TWOYI_ROOTFS}/foo").
    return 0;
}

int unshare(int flags) {
    // DIAGNOSTIC: fake success. Real implementation needs PID namespace
    // emulation (getpid → 1 is already hooked, but waitpid/kill also
    // need virtualization).
    return 0;
}

int pivot_root(const char *new_root, const char *put_old) { return 0; }

int mknod(const char *pathname, mode_t mode, dev_t dev) {
    // DIAGNOSTIC: fake success. Real implementation should create
    // AF_UNIX sockets at these paths (like VM's mknodat+bind pattern)
    // so the guest HAL can talk to the host's device emulators.
    return 0;
}

int mknodat(int dirfd, const char *pathname, mode_t mode, dev_t dev) {
    return 0;
}

int mkdir(const char *pathname, mode_t mode) {
    // DIAGNOSTIC: fake success (host dirs already exist).
    // Real implementation needs path translation.
    return 0;
}

int mkdirat(int dirfd, const char *pathname, mode_t mode) { return 0; }

int setgroups(size_t size, const gid_t *list) {
    // DIAGNOSTIC: fake success (EPERM without CAP_SETGID).
    return 0;
}
