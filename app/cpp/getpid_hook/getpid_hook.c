// getpid_hook.c — LD_PRELOAD library for rootless Android container boot.
//
// Twoyi is designed as a ROOTLESS virtualizer — no root, no ADB, no
// custom ROM. The app runs as untrusted_app, and the zygote's seccomp
// filter blocks mount(), chroot(), unshare(), mknod() for untrusted_app.
//
// This library hooks the syscalls that Android's init binary calls during
// FirstStageMain and SecondStageMain, making them return fake success
// without actually performing the operation. This allows init to boot
// in the HOST filesystem namespace without needing root privileges.
//
// Hooked syscalls:
//   getpid()    -> return 1 (init thinks it's PID 1)
//   getppid()   -> return 0 (init's parent is "the kernel")
//   mount()     -> return 0 (don't actually mount; host already has /proc, /sys, /dev)
//   umount()    -> return 0
//   umount2()   -> return 0
//   chroot()    -> return 0 (don't actually chroot)
//   unshare()   -> return 0 (don't actually unshare)
//   mknod()     -> return 0 (don't create device nodes; kr64 provides sockets)
//   mknodat()   -> return 0
//   pivot_root() -> return 0 (don't actually pivot)
//
// Init's FirstStageMain does:
//   mount("tmpfs", "/dev", "tmpfs", MS_NOSUID, "mode=0755")
//   mkdir("/dev/pts", 0755)
//   mkdir("/dev/socket", 0755)
//   mount("devpts", "/dev/pts", "devpts", 0, NULL)
//   mount("proc", "/proc", "proc", 0, "hidepid=2,gid=...")
//   mount("sysfs", "/sys", "sysfs", 0, NULL)
//   mount("selinuxfs", "/sys/fs/selinux", "selinuxfs", 0, NULL)
//   mknod("/dev/kmsg", S_IFCHR | 0600, makedev(1, 11))
//   mknod("/dev/random", S_IFCHR | 0666, makedev(1, 8))
//   mknod("/dev/urandom", S_IFCHR | 0666, makedev(1, 9))
//   mknod("/dev/ptmx", S_IFCHR | 0666, makedev(5, 2))
//   mknod("/dev/null", S_IFCHR | 0666, makedev(1, 3))
//
// Without these hooks, mount() returns EBUSY (host already has /proc,
// /sys mounted) and init aborts with "Init encountered errors starting
// first stage, aborting".
//
// With these hooks, init thinks all mounts/mknods succeeded and continues
// to SecondStageMain (property service, zygote spawn, etc.).

#include <unistd.h>
#include <sys/syscall.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <fcntl.h>
#include <errno.h>
#include <string.h>

// =========================================================================
// PID hooks — make init think it's PID 1
// =========================================================================

pid_t getpid(void) {
    return 1;
}

pid_t getppid(void) {
    return 0;
}

// =========================================================================
// Mount hooks — fake success without actually mounting
// =========================================================================
// Init's FirstStageMain mounts tmpfs/proc/sysfs/devpts/selinuxfs.
// In the rootless mode, the HOST already has these mounted. If we let
// the real mount() run, it returns EBUSY and init aborts. So we hook
// mount() to return 0 (success) without doing anything.

int mount(const char *source, const char *target,
          const char *filesystemtype, unsigned long mountflags,
          const void *data) {
    // Return success without actually mounting. The host's existing
    // mounts (/proc, /sys, /dev) are already available.
    return 0;
}

int umount(const char *target) {
    return 0;
}

int umount2(const char *target, int flags) {
    return 0;
}

// =========================================================================
// chroot/unshare/pivot_root hooks — fake success
// =========================================================================
// Init doesn't call chroot directly, but some init scripts might.
// unshare() is called by init's subcontext setup. pivot_root is not
// called by init directly but might be called by init scripts.

int chroot(const char *path) {
    // Return success without actually chrooting. Init runs in the host
    // filesystem with TWOYI_ROOTFS pointing at the rootfs.
    return 0;
}

int unshare(int flags) {
    // Return success without actually unsharing. Init's PID namespace
    // and mount namespace setup will be faked by the other hooks.
    return 0;
}

int pivot_root(const char *new_root, const char *put_old) {
    return 0;
}

// =========================================================================
// mknod/mknodat hooks — fake success without creating device nodes
// =========================================================================
// Init's FirstStageMain creates device nodes via mknod:
//   /dev/kmsg, /dev/random, /dev/urandom, /dev/ptmx, /dev/null
//
// In rootless mode, we can't create real device nodes (requires root).
// But the HOST already has /dev/null, /dev/random, /dev/urandom, /dev/ptmx
// as real device nodes. And /dev/kmsg exists on the host too.
//
// So we hook mknod to return 0 (success) without creating anything.
// Init will then open("/dev/kmsg", ...) which opens the HOST's /dev/kmsg.
// This is fine for debugging — init's log messages go to the host's
// kernel log.

int mknod(const char *pathname, mode_t mode, dev_t dev) {
    // If it's a regular file (not a device node), let the real mknod run.
    if ((mode & S_IFMT) == 0) {
        return syscall(SYS_mknod, pathname, mode, dev);
    }
    // For device nodes, return success without creating anything.
    // The host already has /dev/null, /dev/random, etc.
    return 0;
}

int mknodat(int dirfd, const char *pathname, mode_t mode, dev_t dev) {
    if ((mode & S_IFMT) == 0) {
        return syscall(SYS_mknodat, dirfd, pathname, mode, dev);
    }
    return 0;
}

// =========================================================================
// setgroups hook — fake success
// =========================================================================
// Init calls setgroups() to set supplementary groups. In rootless mode,
// this might fail with EPERM. Hook it to return 0.

int setgroups(size_t size, const gid_t *list) {
    return 0;
}

// =========================================================================
// setenv/clearenv pass-through (no hook needed, just for documentation)
// =========================================================================
// Init calls clearenv() and setenv("PATH", ...). These work fine in
// the host environment, no hook needed.
