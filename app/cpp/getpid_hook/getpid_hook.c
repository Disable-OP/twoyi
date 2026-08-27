// getpid_hook.c — minimal LD_PRELOAD library that makes guest init
// think it is PID 1 by hooking getpid()/getppid().
//
// 6-Z184 AUDIT FIX (agent 28): this file used to carry `return 0`
// diagnostic stubs for mount/umount/umount2/chroot/unshare/pivot_root/
// mknod/mknodat/mkdir/mkdirat/setgroups. In the production AOSP boot
// path kr64 sets
//   LD_PRELOAD=/dev/libgetpid_hook.so:/dev/libtwoyi_loader_shlib.so
// (lib.rs) — bionic resolves global symbols from the FIRST preload, so
// those stubs SHADOWED the real path-translating implementations in
// twoyi_loader_shlib.c (mkdir/mkdirat/mknod/mknodat/mount/...): every
// guest mkdir("/dev/__properties__") etc. silently no-opped and the
// shlib's virtualization for those symbols was dead code.
//
// The historical "why fake stubs are not a virtualization layer" essay
// that lived here is preserved in docs/reference/ (ROOTLESS
//VIRTUALIZATION_ARCHITECTURE) and the repo worklog — the stubs
// themselves are GONE. This library now exports exactly what its name
// promises: getpid/getppid.
//
// Note: LD_PRELOAD is only honored by bionic when !getauxval(AT_SECURE),
// i.e. in the KVM/redroid test environments where kr64 is launched via
// adb shell / docker exec — see lib.rs for the full boot-path matrix.

#include <unistd.h>
#include <sys/types.h>

pid_t getpid(void) {
    return 1;
}

pid_t getppid(void) {
    return 0;
}
