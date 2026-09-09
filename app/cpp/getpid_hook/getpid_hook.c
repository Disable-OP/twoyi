// getpid_hook.c — LD_PRELOAD getpid()/getppid() interposer.
//
// 6-Z305i (run 34005543435 decode): this hook used to return 1
// UNCONDITIONALLY for every process that inherited it (init forks and
// execs children with the same LD_PRELOAD, so EVERY guest process saw
// getpid()==1). That broke AOSP init's own aborter contract
// (platform/system/core init/util.cpp, android-11):
//
//     static void InitAborter(const char* abort_message) {
//         // When init forks, it continues to use this aborter for
//         // LOG(FATAL), but we want children to simply abort instead
//         // of trying to reboot the system.
//         if (getpid() != 1) {
//             android::base::DefaultAborter(abort_message);
//             return;
//         }
//         InitFatalReboot(SIGABRT);
//     }
//
// With the blanket fake, every child's LOG(FATAL) took the
// InitFatalReboot path: "InitFatalReboot: signal 6" → the reboot
// machinery's multi-second service-shutdown wait (the 5 s nanosleep
// loop the death-window tracer captured on the vendor_init subcontext)
// → "Reboot ending, jumping to kernel" → __reboot() fails in the
// container (EPERM) → abort() retry → _exit(127) — and the socketpair
// peer (init main, blocked in Subcontext::TransmitMessage) EOF'd and
// quietly exited(0) right after. The whole boot died from ONE child
// LOG(FATAL) taking init's impossible-reboot path.
//
// The hook now defers to the tracer's fake: kr64's ptrace EXIT arm
// rewrites the getpid SYSCALL result to 1 for the INIT anchor pid only
// (6-Z305i, ptrace_emu.rs pending_getpid consumption). Every other
// guest process gets its REAL host pid — which is exactly what a real
// kernel reports and what InitAborter expects. This hook keeps one
// belt: if TWOYI_INIT_PID is set (a future launch path may pin the
// anchor before ptrace starts), the matching process still sees 1.
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
// shlib's virtualization for those symbols was dead code. The stubs
// are gone; this library exports exactly what its name promises.
//
// Note: LD_PRELOAD is only honored by bionic when !getauxval(AT_SECURE),
// i.e. in the KVM/redroid test environments where kr64 is launched via
// adb shell / docker exec — see lib.rs for the full boot-path matrix.

#include <unistd.h>
#include <sys/types.h>
#include <stdlib.h>
#include <sys/syscall.h>

pid_t getpid(void) {
    pid_t real = (pid_t)syscall(SYS_getpid);

    static pid_t anchor = -2;
    if (anchor == -2) {
        const char *s = getenv("TWOYI_INIT_PID");
        anchor = (s != NULL && *s != '\0') ? (pid_t)atoi(s) : -1;
    }
    // No anchor pinned (the normal ptrace path — the tracer fakes the
    // syscall result itself) → pass the real value through untouched.
    return (anchor > 0 && real == anchor) ? 1 : real;
}

pid_t getppid(void) {
    return (pid_t)syscall(SYS_getppid);
}

// 6-Z305t-72: gettid() must NEVER read bionic's cached tid. The cache
// (pthread_internal_t.tid) is seeded once at libc init from the raw
// getpid syscall — bionic guards its PID cache against a 0 value but
// the TID cache only against -1 — so a single poisoned getpid return
// permanently pins gettid() to 0 for that process. That was the
// ladder-#142 zygote wall: the tracer's 6-Z147 prctl rewrite matched
// the cross-ABI literal union (157|172|167), which on aarch64 ALSO
// matched nr=172 (= getpid) and overwrote every arm64 getpid's return
// with emulated_prctl_ret(0,0)==0; ART's first recursive-mutex lock
// taken with Thread::Current()==nullptr then took the IsExclusiveHeld
// fast-path (fresh-mutex owner 0 == GetTid() 0), skipped the futex CAS,
// and the first unlock FATALed "Unexpected state_ in unlock 0 for
// CumulativeLoggerLock…" every generation. The raw syscall can never
// return 0 for a live thread, so this interposer is immune to the
// entire cache-poison class. ART calls the public gettid() symbol
// (libartbase utils.cc GetTid()), so this preload export (first in
// LD_PRELOAD order) covers every guest library including libart.
pid_t gettid(void) {
    return (pid_t)syscall(SYS_gettid);
}
