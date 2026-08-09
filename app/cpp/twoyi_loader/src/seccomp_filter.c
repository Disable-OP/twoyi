// seccomp_filter.c — Seccomp BPF filter installation.
//
// Builds and installs a BPF filter that traps specific syscalls with
// SECCOMP_RET_TRAP. When a trapped syscall is invoked, the kernel:
//   1. Does NOT execute the syscall
//   2. Delivers SIGSYS to the thread
//   3. Sets si_code = SYS_SECCOMP, si_syscall = syscall number
//   4. The SIGSYS handler emulates the syscall and writes the return value
//
// The filter structure (from VM analysis + kernel docs):
//   [0] Load seccomp_data.arch
//   [1] If arch == AUDIT_ARCH_*, skip to [3] (correct arch)
//   [2] Return SECCOMP_RET_KILL_PROCESS (wrong arch — security)
//   [3] Load seccomp_data.nr (syscall number)
//   [4] If nr == mount, return SECCOMP_RET_TRAP | data=1
//   [5] If nr == umount2, return SECCOMP_RET_TRAP | data=2
//   [6] If nr == chroot, return SECCOMP_RET_TRAP | data=3
//   [7] Default: return SECCOMP_RET_ALLOW (all other syscalls pass through)
//
// Sources:
//   - Kernel: https://www.kernel.org/doc/html/v5.0/userspace-api/seccomp_filter.html
//   - seccomp_data struct: include/uapi/linux/seccomp.h
//   - BPF encoding: include/uapi/linux/bpf_common.h, include/uapi/linux/filter.h
//   - VM: libkrloader64.so BPF construction at 0x3384

#include "seccomp_filter.h"
#include "arch_regs.h"

#include <linux/seccomp.h>
#include <linux/filter.h>
#include <linux/audit.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <errno.h>
#include <unistd.h>
#include <stddef.h>

// SECCOMP_RET_TRAP value (from linux/seccomp.h).
// We define it here in case the system headers don't have it.
#ifndef SECCOMP_RET_TRAP
#define SECCOMP_RET_TRAP 0x00030000U
#endif

#ifndef SECCOMP_RET_ALLOW
#define SECCOMP_RET_ALLOW 0x7fff0000U
#endif

#ifndef SECCOMP_RET_KILL_PROCESS
#define SECCOMP_RET_KILL_PROCESS 0x80000000U
#endif

// SECCOMP_RET_DATA mask (lower 16 bits of the return value).
// We use this to pass a "reason code" to the SIGSYS handler via si_errno.
// data=1 → mount, data=2 → umount2, data=3 → chroot
#define TRAP_MOUNT   (SECCOMP_RET_TRAP | 1)
#define TRAP_UMOUNT2 (SECCOMP_RET_TRAP | 2)
#define TRAP_CHROOT  (SECCOMP_RET_TRAP | 3)

// seccomp_data field offsets (from include/uapi/linux/seccomp.h):
//   struct seccomp_data { int nr; __u32 arch; __u64 instruction_pointer; __u64 args[6]; };
#define OFF_nr    0  // offset of seccomp_data.nr
#define OFF_arch  4  // offset of seccomp_data.arch

// BPF instruction helpers (from linux/filter.h):
//   BPF_STMT(code, k)    — { code, 0, 0, k }
//   BPF_JUMP(code, k, jt, jf) — { code, jt, jf, k }
//
// BPF opcodes we use:
//   BPF_LD | BPF_W | BPF_ABS  = 0x20 — load 32-bit from seccomp_data at offset k
//   BPF_JMP | BPF_JEQ | BPF_K = 0x15 — if A == k, skip jt instructions, else jf
//   BPF_RET | BPF_K           = 0x06 — return k

int twoyi_seccomp_install(void) {
    // Step 1: Set PR_SET_NO_NEW_PRIVS.
    // This is mandatory before installing a seccomp filter (unless the
    // process has CAP_SYS_ADMIN). It prevents the filter from being
    // bypassed via execve of setuid binaries.
    // Source: man seccomp(2), kernel seccomp.c
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0) {
        return -1;
    }

    // Step 2: Build the BPF filter program.
    //
    // The filter checks the architecture first (security: prevent
    // cross-arch syscall number confusion), then checks the syscall
    // number against our trap list.
    struct sock_filter filter[] = {
        // [0] Load seccomp_data.arch into accumulator A.
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, OFF_arch),

        // [1] If arch matches our architecture, skip to [3] (jt=1, jf=0).
        //     TWOYI_AUDIT_ARCH is 0xC00000B7 (arm64) or 0xC000003E (x86_64).
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_AUDIT_ARCH, 1, 0),

        // [2] Wrong architecture — kill the process (security).
        //     This prevents a 32-bit syscall from bypassing the filter
        //     on a 64-bit kernel (syscall number confusion attack).
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS),

        // [3] Load seccomp_data.nr (syscall number) into accumulator A.
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, OFF_nr),

        // [4] If syscall == mount, trap with data=1.
        //     TWOYI_NR_mount is 40 (arm64) or 165 (x86_64).
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_mount, 4, 0),

        // [5] If syscall == umount2, trap with data=2.
        //     TWOYI_NR_umount2 is 39 (arm64) or 166 (x86_64).
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_umount2, 3, 0),

        // [6] If syscall == chroot, trap with data=3.
        //     TWOYI_NR_chroot is 51 (arm64) or 161 (x86_64).
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, TWOYI_NR_chroot, 2, 0),

        // [7] Default: allow all other syscalls.
        //     This is critical — we only trap mount/umount2/chroot.
        //     All other syscalls (getpid, read, write, fork, etc.) pass
        //     through to the kernel normally.
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),

        // [8] Trap mount (reached via jt=4 from [4]).
        BPF_STMT(BPF_RET | BPF_K, TRAP_MOUNT),

        // [9] Trap umount2 (reached via jt=3 from [5]).
        BPF_STMT(BPF_RET | BPF_K, TRAP_UMOUNT2),

        // [10] Trap chroot (reached via jt=2 from [6]).
        BPF_STMT(BPF_RET | BPF_K, TRAP_CHROOT),
    };

    struct sock_fprog prog = {
        .len = sizeof(filter) / sizeof(filter[0]),
        .filter = filter,
    };

    // Step 3: Install the filter via seccomp(2) syscall.
    // SECCOMP_SET_MODE_FILTER installs the filter for the current thread.
    // (We use seccomp(2) instead of prctl(PR_SET_SECCOMP, ...) because
    //  seccomp(2) is the modern API and supports flags.)
    //
    // Note: We do NOT use SECCOMP_FILTER_FLAG_TSYNC here because the
    // filter is installed before any threads are created. TSYNC is only
    // needed if there are already other threads running.
    if (syscall(SYS_seccomp, SECCOMP_SET_MODE_FILTER, 0, &prog) != 0) {
        // If seccomp(2) is not available (old kernel), fall back to
        // prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &prog).
        if (prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &prog, 0, 0) != 0) {
            return -1;
        }
    }

    return 0;
}
