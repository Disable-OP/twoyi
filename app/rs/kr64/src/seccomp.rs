// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! Seccomp filter + SIGSYS handler — the "kernel replacement" syscall
//! interception layer.
//!
//! This mirrors what VM's `libkr64.so` does (see `VM_KR64_ANALYSIS.md`
//! §12 and §11):
//!
//! 1. `prctl(PR_SET_NO_NEW_PRIVS, 1)` — required before installing a
//!    seccomp filter without `CAP_SYS_ADMIN`.
//! 2. `sigaction(SIGSYS, …)` — install a handler that catches trapped
//!    syscalls. The string `__NR_rt_sigaction SIGSYS` was decoded from
//!    `libkr64.11.so`'s `.data` (key 0x1a).
//! 3. `seccomp(SECCOMP_SET_MODE_FILTER, …)` — install a BPF program
//!    that traps forbidden syscalls. The BPF program returns
//!    `SECCOMP_RET_TRAP` for "intercepted" syscalls (mount, umount,
//!    reboot, kexec_load, init_module, etc.) and `SECCOMP_RET_ALLOW`
//!    for everything else.
//! 4. The SIGSYS handler logs `BLOCKED.SYSCALL.FAILED: <nr>` (string
//!    decoded from libkr64.so with key 0xc9 / 0xc2) and either:
//!    - Emulates the syscall (sets the return value to 0 / fake-success
//!      and advances the PC past the syscall instruction), OR
//!    - Kills the guest (via `_exit`).
//!
//! # Why `SECCOMP_RET_TRAP`, not `SECCOMP_RET_TRACE`
//!
//! The task spec mentions `SECCOMP_RET_TRACE`, but `SECCOMP_RET_TRACE`
//! requires a ptrace tracer to be attached to the process — without
//! one, the kernel falls back to `SECCOMP_RET_KILL`. The mechanism
//! that actually delivers `SIGSYS` (and the one VM uses, per the
//! analysis) is `SECCOMP_RET_TRAP`. We expose `SECCOMP_RET_TRACE` as
//! a constant for callers that want to switch to ptrace-mode tracing
//! later (e.g. for debugging the guest), but the default filter uses
//! `SECCOMP_RET_TRAP`.
//!
//! # Why we don't trap EVERY syscall
//!
//! VM's filter traps a small set of dangerous syscalls (mount, reboot,
//! kexec, init_module, ptrace, etc.) and lets everything else go
//! through. This is the right tradeoff — trapping every syscall (and
//! emulating each one in the handler) would be 100× slower and would
//! require a complete kernel emulation layer. We only trap syscalls
//! that:
//!   (a) would let the guest escape its sandbox, OR
//!   (b) need to be redirected to a userspace implementation
//!       (mount → bind mount, umount → unbind, swapon → no-op, etc.)

use libc::{c_int, c_void, sigaction, siginfo_t, sigset_t, ucontext_t};
use std::collections::HashSet;
use std::sync::OnceLock;

// Crate-local logging macros (defined in lib.rs) — no external `log` crate.
use crate::{error, info};

// ============================================================================
// BPF / seccomp constants (not all exposed by the `libc` crate on every
// target, so we define them ourselves to be safe).
// ============================================================================

// BPF instruction classes (low 3 bits of `code`).
const BPF_LD: u16 = 0x00;
const BPF_JMP: u16 = 0x05;
const BPF_RET: u16 = 0x06;

// BPF size modifiers (next 2 bits).
const BPF_W: u16 = 0x00;

// BPF mode modifiers for BPF_LD (high 3 bits).
const BPF_ABS: u16 = 0x20;

// BPF jump operations (next 4 bits).
const BPF_JEQ: u16 = 0x10;

// BPF source operand (high 1 bit).
const BPF_K: u16 = 0x00;

// Seccomp return values (top 16 bits of the return word).
#[allow(dead_code)]
const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
#[allow(dead_code)]
const SECCOMP_RET_KILL_THREAD: u32 = 0x0000_0000;
const SECCOMP_RET_TRAP: u32 = 0x0003_0000;
#[allow(dead_code)]
const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;
#[allow(dead_code)]
const SECCOMP_RET_TRACE: u32 = 0x7ff0_0000;
#[allow(dead_code)]
const SECCOMP_RET_LOG: u32 = 0x7ffc_0000;
const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;

// prctl(2) operations used by seccomp.
const PR_SET_NO_NEW_PRIVS: c_int = 38;

// seccomp(2) operations.
const SECCOMP_SET_MODE_FILTER: c_int = 1;

// seccomp(2) flags.
const SECCOMP_FILTER_FLAG_TSYNC: u32 = 1;

// Audit architectures — used to gate the BPF program on the right ABI.
// These come from <linux/audit.h>.
#[cfg(target_arch = "aarch64")]
const AUDIT_ARCH_EXPECTED: u32 = 0xC000_00B7; // EM_AARCH64 | __AUDIT_ARCH_64BIT | __AUDIT_ARCH_LE
#[cfg(target_arch = "x86_64")]
const AUDIT_ARCH_EXPECTED: u32 = 0xC000_003E; // EM_X86_64  | __AUDIT_ARCH_64BIT | __AUDIT_ARCH_LE

// struct seccomp_data field offsets (see <linux/seccomp.h>).
const OFF_NR: u32 = 0; // int   nr
const OFF_ARCH: u32 = 4; // __u32 arch

// ============================================================================
// BPF instruction builder — thin wrapper around `sock_filter`.
// ============================================================================

/// A single BPF instruction. Layout matches `struct sock_filter`:
/// `__u16 code; __u8 jt; __u8 jf; __u32 k;`
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SockFilter {
    pub code: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}

/// Build a `BPF_LD | BPF_W | BPF_ABS` instruction — load a 32-bit word
/// from offset `k` of `struct seccomp_data` into the accumulator.
const fn bpf_ld_abs(k: u32) -> SockFilter {
    SockFilter {
        code: BPF_LD | BPF_W | BPF_ABS,
        jt: 0,
        jf: 0,
        k,
    }
}

/// Build a `BPF_JMP | BPF_JEQ | BPF_K` instruction — if accumulator
/// equals `k`, jump `jt` instructions forward, else jump `jf`.
const fn bpf_jeq(k: u32, jt: u8, jf: u8) -> SockFilter {
    SockFilter {
        code: BPF_JMP | BPF_JEQ | BPF_K,
        jt,
        jf,
        k,
    }
}

/// Build a `BPF_RET | BPF_K` instruction — return `k` from the filter.
const fn bpf_ret(k: u32) -> SockFilter {
    SockFilter {
        code: BPF_RET | BPF_K,
        jt: 0,
        jf: 0,
        k,
    }
}

// ============================================================================
// Syscall classification — what to allow, what to trap, what to kill.
// ============================================================================

/// Helper: insert `nr` into the set. We use a small macro instead of a
/// function so that callers can mix `libc::SYS_*` (which is `c_long`)
/// with raw `i32` literals without type-inference issues.
macro_rules! add_nr {
    ($set:expr, $nr:expr) => {
        $set.insert($nr as i32);
    };
}

/// Syscalls the guest is allowed to call directly (no interception).
///
/// This list is the union of:
///   - the task spec's allow-list (read, write, open, openat, close,
///     mmap, mprotect, munmap, brk, rt_sigaction, rt_sigprocmask,
///     ioctl, pread64, pwrite64, readv, writev, access, pipe, pipe2,
///     select, poll, ppoll, getsockopt, socket, connect, bind, listen,
///     accept, accept4, getsockname, getpeername, sendto, recvfrom,
///     sendmsg, recvmsg, shutdown, setsockopt, dup, dup2, fcntl, stat,
///     fstat, lstat, newfstatat, clone, uname, getcwd)
///   - syscalls added by follow-up tasks (wait4, waitid, fork, vfork,
///     exit, exit_group, rt_sigreturn, sigaltstack, gettid, getpid,
///     getppid, getuid, geteuid, getgid, getegid, setuid, setgid,
///     setpgid, setsid, getpgid, getpgrp, set_tid_address, clock_*,
///     timer_create, etc.)
///
/// We use `libc::SYS_*` so the numbers resolve correctly per target
/// (aarch64 vs x86_64).
fn allowed_syscalls() -> HashSet<i32> {
    use libc::*;
    let mut s: HashSet<i32> = HashSet::new();

    // --- file I/O (present on both aarch64 and x86_64) ---
    for &nr in &[
        SYS_read,
        SYS_write,
        SYS_readv,
        SYS_writev,
        SYS_pread64,
        SYS_pwrite64,
        SYS_openat,
        SYS_close,
        SYS_dup,
        SYS_dup3,
        SYS_fcntl,
        SYS_lseek,
        SYS_readlinkat,
        SYS_getcwd,
    ] {
        add_nr!(s, nr);
    }
    // SYS_newfstatat (x86_64's "stat relative to dirfd") and SYS_ftruncate
    // are NOT exposed by the libc crate for android-aarch64 — the crate
    // simply omits those `SYS_*` constants for the bionic aarch64 target.
    // (The underlying syscalls DO exist on aarch64 — newfstatat=79,
    // ftruncate=46 — but referencing them needs raw numbers; tracked as a
    // follow-up for full aarch64 stat-syscall support.) Gate x86_64-only
    // so the crate compiles for aarch64-linux-android.
    #[cfg(target_arch = "x86_64")]
    {
        add_nr!(s, SYS_newfstatat);
        add_nr!(s, SYS_ftruncate);
    }

    // --- file I/O (x86_64-only: open, access, dup2, stat, lstat, fstat,
    //     fchmod, fchown, chroot, unlink, rmdir, chmod, chown, lchown).
    //     On aarch64 these are replaced by the *at variants.
    #[cfg(target_arch = "x86_64")]
    for &nr in &[
        SYS_open,
        SYS_access,
        SYS_dup2,
        SYS_stat,
        SYS_fstat,
        SYS_lstat,
        SYS_chmod,
        SYS_fchmod,
        SYS_lchown,
        SYS_chown,
        SYS_fchown,
        SYS_chroot,
        SYS_unlink,
        SYS_rmdir,
        SYS_mkdir,
        SYS_readlink,
        SYS_truncate,
    ] {
        add_nr!(s, nr);
    }
    // (The previous aarch64-only block referencing SYS_fstat / SYS_fstatat
    // has been removed — neither constant is defined by the libc crate for
    // android-aarch64, and SYS_fstatat is not defined on any target.)

    // --- memory ---
    for &nr in &[
        SYS_mmap,
        SYS_munmap,
        SYS_mprotect,
        SYS_mremap,
        SYS_msync,
        SYS_madvise,
        SYS_brk,
    ] {
        add_nr!(s, nr);
    }
    // mincore exists on both — add if defined.
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    add_nr!(s, SYS_mincore);

    // --- signals ---
    for &nr in &[
        SYS_rt_sigaction,
        SYS_rt_sigprocmask,
        SYS_rt_sigpending,
        SYS_rt_sigtimedwait,
        SYS_rt_sigreturn,
        SYS_sigaltstack,
        SYS_rt_sigsuspend,
        SYS_kill,
        SYS_tgkill,
        SYS_tkill,
    ] {
        add_nr!(s, nr);
    }

    // --- signals: the signalfd / pidfd family (Task 6-Z109) ---
    //
    // Android 11 init (system/core/init/init.cpp, signal_handler_init):
    //   sigprocmask(SIG_BLOCK, {SIGCHLD});          // allowed above
    //   fd = signalfd4(-1, {SIGCHLD}, 8, SFD_NONBLOCK|SFD_CLOEXEC);
    //   epoll_ctl(epfd, EPOLL_CTL_ADD, fd, {EPOLLIN});
    // …then the main epoll loop reads child-death signalfd_siginfo records
    // from that fd. kill(62)/tgkill(234)/tkill(200) are init's service-
    // restart hammers (already allowed above); pidfd_open(434)/
    // pidfd_send_signal(424) are the modern equivalents newer init
    // releases prefer. ALL of these are process-scoped and
    // permission-checked per-signal — safe for an untrusted app uid:
    // signalfd4 can only observe the CALLER'S OWN pending signals, and
    // kill/tgkill/pidfd_send_signal still go through the kernel's
    // per-process permission checks (same-uid or CAP_KILL).
    //
    // Numbers (libc crate, verified against the kernel UAPI tables):
    //   x86_64:  signalfd=282 signalfd4=289 pidfd_send_signal=424
    //            pidfd_open=434 kill=62 tkill=200 tgkill=234
    //   i386:    signalfd=321 signalfd4=327 pidfd_send_signal=424
    //            pidfd_open=434 kill=37  tkill=238 tgkill=270
    //   aarch64: signalfd=(dropped — asm-generic kept only signalfd4=74)
    //            pidfd_send_signal=424 pidfd_open=434 kill=129
    //            tkill=130 tgkill=131
    //
    // NOTE (deployment truth): in the ptrace-emulation mode that actually
    // boots the guest, this filter is NOT installed (lib.rs skips
    // seccomp::install for the i386 guest — the wrong-arch arm would
    // KILL_PROCESS every int $0x80 syscall), so the HOST app-uid filter
    // is what the guest really hits; host-side blocks are handled by the
    // tracer's 6-Z109 ENOSYS fallbacks (ptrace_emu.rs — signalfd4 fake-fd
    // + tracer-forwarded kill). These entries harden the ROOT mode
    // (filter installed, x86_64 guest) and LOCK the intent against a
    // future deny-by-default tightening — the same belt-and-suspenders
    // rationale as the 6-Z108 rt_sigaction entry.
    add_nr!(s, SYS_signalfd4);
    add_nr!(s, SYS_pidfd_open);
    add_nr!(s, SYS_pidfd_send_signal);
    // SYS_signalfd (the pre-signalfd4 variant) exists only on x86 —
    // asm-generic dropped it (aarch64 callers use signalfd4 directly).
    // The libc crate therefore omits the constant for android-aarch64;
    // gate x86_64-only so the crate compiles everywhere (same pattern
    // as SYS_newfstatat above).
    #[cfg(target_arch = "x86_64")]
    add_nr!(s, SYS_signalfd);

    // --- sockets / IPC (present on both) ---
    for &nr in &[
        SYS_socket,
        SYS_socketpair,
        SYS_connect,
        SYS_bind,
        SYS_listen,
        SYS_accept4,
        SYS_getsockname,
        SYS_getpeername,
        SYS_sendto,
        SYS_recvfrom,
        SYS_sendmsg,
        SYS_recvmsg,
        SYS_shutdown,
        SYS_setsockopt,
        SYS_getsockopt,
        SYS_pipe2,
        SYS_pselect6,
        SYS_ppoll,
        SYS_epoll_create1,
        SYS_epoll_ctl,
        SYS_eventfd2,
        SYS_timerfd_create,
        SYS_timerfd_settime,
        SYS_inotify_init1,
        SYS_inotify_add_watch,
        SYS_inotify_rm_watch,
    ] {
        add_nr!(s, nr);
    }
    // SYS_poll and SYS_epoll_wait are x86_64-only — aarch64's asm-generic
    // syscall table replaced them with ppoll (already allowed above) and
    // epoll_pwait respectively. (Adding SYS_epoll_pwait for aarch64 would
    // keep the guest's epoll-wait working; left as a follow-up.)
    #[cfg(target_arch = "x86_64")]
    {
        add_nr!(s, SYS_poll);
        add_nr!(s, SYS_epoll_wait);
    }

    // aarch64-only legacy socket syscalls (recv / send). On the unified
    // asm-generic syscall table used by aarch64/riscv64 these don't exist
    // (callers use recvfrom/sendto with a NULL address), so we skip
    // them. They DO exist on x86_64 but we already allow sendto/recvfrom
    // above, which is sufficient for the bionic/libc++ socket layer.

    // accept / select — present on x86_64, removed on aarch64 (replaced
    // by accept4 / pselect6).
    #[cfg(target_arch = "x86_64")]
    for &nr in &[SYS_accept, SYS_select] {
        add_nr!(s, nr);
    }

    // --- process / thread ---
    for &nr in &[
        SYS_clone,
        SYS_execve,
        SYS_execveat,
        SYS_exit,
        SYS_exit_group,
        SYS_wait4,
        SYS_waitid,
        SYS_set_tid_address,
        SYS_setpgid,
        SYS_setsid,
        SYS_getpgid,
        SYS_getpid,
        SYS_getppid,
        SYS_gettid,
        SYS_getuid,
        SYS_geteuid,
        SYS_getgid,
        SYS_getegid,
        SYS_getresuid,
        SYS_getresgid,
        SYS_prctl, // NOT filtered — guest can set its own name, etc.
        SYS_uname,
        SYS_sethostname,
        SYS_setdomainname,
        SYS_getrlimit,
        SYS_setrlimit,
        SYS_prlimit64,
        SYS_getrandom,
        SYS_clock_gettime,
        SYS_clock_getres,
        SYS_clock_settime,
        SYS_gettimeofday,
        SYS_nanosleep,
    ] {
        add_nr!(s, nr);
    }
    // SYS_time is x86_64-only — removed from the asm-generic syscall table
    // used by aarch64 (callers use gettimeofday, already allowed above).
    #[cfg(target_arch = "x86_64")]
    add_nr!(s, SYS_time);

    // fork / vfork — present on x86_64, removed on aarch64.
    #[cfg(target_arch = "x86_64")]
    for &nr in &[SYS_fork, SYS_vfork] {
        add_nr!(s, nr);
    }

    // setuid / setgid — present on x86_64, removed on aarch64.
    #[cfg(target_arch = "x86_64")]
    for &nr in &[
        SYS_setuid,
        SYS_setgid,
        SYS_setreuid,
        SYS_setregid,
        SYS_settimeofday,
        SYS_getpgrp,
    ] {
        add_nr!(s, nr);
    }

    // --- filesystem (read-only; writes go through mount_mgr) ---
    // NOTE: Linux exposes these as `fadvise64` and `fallocate` (not the
    // POSIX names `posix_fadvise` / `posix_fallocate`). The libc crate
    // follows the kernel's `__NR_*` naming.
    //
    // SECURITY NOTE (6-Z185): SYS_getdents64 stays in the ALLOW bucket —
    // it must EXECUTE natively (the tracer does not marshal dirent
    // structs; that would slow every directory read). The sandbox does
    // NOT rely on this allowlist: in non-root ptrace mode every
    // open/openat is translated into the rootfs by
    // vfs::SandboxPolicy::translate_guest, and the tracer's entry-side
    // backstop (ptrace_emu::sandbox_backstop_at_entry) additionally
    // verifies each getdents64 fd's ORIGIN via /proc/<tid>/fd/<n> and
    // denies (fake -EACCES) any fd resolving outside the sandbox. The
    // allowlist entry is therefore safe-by-construction, not trusted.
    for &nr in &[
        SYS_getdents64,
        SYS_fallocate,
        SYS_flock,
        SYS_sync,
        SYS_fsync,
        SYS_fdatasync,
        SYS_renameat,
        SYS_renameat2,
        SYS_linkat,
        SYS_symlinkat,
        SYS_unlinkat,
        SYS_mkdirat,
        SYS_fchmod,
        SYS_fchmodat,
        SYS_fchown,
        SYS_fchownat,
        SYS_umask,
        SYS_chdir,
        SYS_fchdir,
    ] {
        add_nr!(s, nr);
    }
    // SYS_fadvise64 is not exposed by the libc crate for android-aarch64
    // (the syscall exists as __NR_fadvise64=223 on aarch64, but the crate
    // omits the constant). Gate x86_64-only to keep the crate compiling.
    #[cfg(target_arch = "x86_64")]
    add_nr!(s, SYS_fadvise64);
    // readahead exists on both.
    add_nr!(s, SYS_readahead);

    // --- ioctl / device ---
    add_nr!(s, SYS_ioctl);

    // --- misc ---
    add_nr!(s, SYS_memfd_create);
    #[cfg(target_arch = "x86_64")]
    add_nr!(s, SYS_getdents);
    #[cfg(target_arch = "x86_64")]
    add_nr!(s, SYS_getpgrp);

    s
}

/// Per-VM "intercept" set — syscalls the guest IS allowed to call, but
/// that we trap with SIGSYS so the handler can:
///   (a) log them,
///   (b) redirect them to a userspace implementation, or
///   (c) fake success.
///
/// This is the `SECCOMP_RET_TRAP` set. Examples:
///   - `mount`   → redirect to `mount_mgr::bind_mount()`
///   - `umount2` → redirect to `mount_mgr::unbind()`
///   - `swapon`  → fake success (no swap in the container)
///   - `swapoff` → fake success
///   - `acct`    → fake success (no process accounting)
fn trapped_syscalls() -> HashSet<i32> {
    use libc::*;
    let mut s: HashSet<i32> = HashSet::new();
    add_nr!(s, SYS_mount);
    add_nr!(s, SYS_umount2);
    add_nr!(s, SYS_swapon);
    add_nr!(s, SYS_swapoff);
    add_nr!(s, SYS_acct);
    add_nr!(s, SYS_reboot);
    s
}

/// Per-VM "kill" set — syscalls that immediately kill the guest
/// process. These are syscalls that would let the guest escape its
/// sandbox or destabilise the host.
fn killed_syscalls() -> HashSet<i32> {
    use libc::*;
    let mut s: HashSet<i32> = HashSet::new();
    add_nr!(s, SYS_kexec_load);
    // SYS_kexec_file_load is not exposed by the libc crate for
    // android-aarch64 (only for x86_64 / glibc-aarch64). Gate x86_64-only.
    #[cfg(target_arch = "x86_64")]
    add_nr!(s, SYS_kexec_file_load);
    add_nr!(s, SYS_init_module);
    add_nr!(s, SYS_finit_module);
    add_nr!(s, SYS_delete_module);
    // SYS_iopl and SYS_ioperm are x86/x86_64-only syscalls.
    // They don't exist on aarch64 (asm-generic syscall table).
    #[cfg(target_arch = "x86_64")]
    {
        add_nr!(s, SYS_iopl);
        add_nr!(s, SYS_ioperm);
    }
    add_nr!(s, SYS_kcmp);
    add_nr!(s, SYS_ptrace);
    add_nr!(s, SYS_pivot_root);
    s
}

// ============================================================================
// Build the BPF program.
// ============================================================================

/// Compile the allow/trap/kill classification into a BPF program.
///
/// The program structure is:
/// ```text
///   1. ld arch                         // load audit arch
///   2. jeq EXPECTED_ARCH, jt=1, jf=0   // right arch: skip the next insn;
///                                      // wrong arch: fall into the kill
///   3. ret KILL_PROCESS                // wrong arch dies IMMEDIATELY
///   4. ld nr                           // load syscall number
///   5. for each allowed syscall:
///        jeq nr, jt=0, jf=1            // if match: fall to ret ALLOW
///        ret ALLOW                      // else: skip to next jeq
///   6. for each trapped syscall:
///        jeq nr, jt=0, jf=1
///        ret TRAP
///   7. for each killed syscall:
///        jeq nr, jt=0, jf=1
///        ret KILL_PROCESS
///   8. ret ALLOW                       // default: allow
/// ```
///
/// EVERY jump in this program is 0 or 1 instructions long, so the
/// 8-bit jt/jf fields can never overflow, regardless of how many
/// syscalls are in each set. (An earlier layout kept the wrong-arch
/// KILL at the very end and patched the arch jeq's jf to jump the
/// whole program — an offset of ~260-337 instructions for the real
/// sets, which silently truncated to 8 bits and landed the wrong-arch
/// path in the middle of the allow chain. Killing inline — the
/// pattern the kernel's own selftests use — makes that whole class of
/// bug unrepresentable.)
pub fn build_filter() -> Vec<SockFilter> {
    let allowed = allowed_syscalls();
    let trapped = trapped_syscalls();
    let killed = killed_syscalls();

    // (1) Load arch.
    // Sequential BPF emit — the first insn seeds the vec, the rest push.
    let mut prog: Vec<SockFilter> = vec![bpf_ld_abs(OFF_ARCH)];
    // (2-3) Arch gate: if the arch does not match, fall through into
    //     an IMMEDIATE KILL_PROCESS. The jump distances here are 1
    //     and 0 — always representable in the 8-bit jt/jf fields, no
    //     matter how long the rest of the program is.
    prog.push(bpf_jeq(AUDIT_ARCH_EXPECTED, 1, 0)); // match: skip kill
    prog.push(bpf_ret(SECCOMP_RET_KILL_PROCESS)); // wrong arch: die

    // (4) Load syscall number.
    prog.push(bpf_ld_abs(OFF_NR));

    // (5) Allowed syscalls: jeq + ret ALLOW.
    let mut sorted_allowed: Vec<i32> = allowed
        .iter()
        .copied()
        .filter(|nr| !trapped.contains(nr) && !killed.contains(nr))
        .collect();
    sorted_allowed.sort_unstable();
    sorted_allowed.dedup();
    for nr in sorted_allowed {
        prog.push(bpf_jeq(nr as u32, 0, 1)); // match: fall to next; else skip 1
        prog.push(bpf_ret(SECCOMP_RET_ALLOW));
    }

    // (6) Trapped syscalls: jeq + ret TRAP.
    let mut sorted_trapped: Vec<i32> = trapped
        .iter()
        .copied()
        .filter(|nr| !killed.contains(nr))
        .collect();
    sorted_trapped.sort_unstable();
    sorted_trapped.dedup();
    for nr in sorted_trapped {
        prog.push(bpf_jeq(nr as u32, 0, 1));
        prog.push(bpf_ret(SECCOMP_RET_TRAP));
    }

    // (7) Killed syscalls: jeq + ret KILL.
    let mut sorted_killed: Vec<i32> = killed.iter().copied().collect();
    sorted_killed.sort_unstable();
    sorted_killed.dedup();
    for nr in sorted_killed {
        prog.push(bpf_jeq(nr as u32, 0, 1));
        prog.push(bpf_ret(SECCOMP_RET_KILL_PROCESS));
    }

    // (8) Default: allow.
    prog.push(bpf_ret(SECCOMP_RET_ALLOW));

    // Every jt/jf emitted above is <= 1; assert that invariant so a
    // future edit cannot reintroduce a long (u8-truncating) jump.
    for (i, insn) in prog.iter().enumerate() {
        debug_assert!(
            insn.jt <= 1 && insn.jf <= 1,
            "insn {i} has a long jump (jt={}, jf={}) — BPF jt/jf are 8-bit",
            insn.jt,
            insn.jf
        );
    }

    info!(
        "[KR64][seccomp] BPF filter built: {} instructions",
        prog.len()
    );
    prog
}

// ============================================================================
// Install the filter + SIGSYS handler.
// ============================================================================

/// Snapshotted syscall classification tables, read by the SIGSYS handler.
///
/// These are populated by [`install()`] BEFORE the handler is installed,
/// and are immutable afterwards. Reading an already-initialised
/// `OnceLock` (via `get()` or `get_or_init`'s fast path) is a single
/// atomic load — fully async-signal-safe — and `HashSet::contains` is a
/// read-only traversal, also signal-safe.
///
/// **Why no `Mutex`?** The previous version wrapped these in
/// `Mutex<HashSet<i32>>` and called `.lock()` from `classify()`, which
/// runs inside the SIGSYS handler. `Mutex::lock` is NOT async-signal-safe
/// (it performs a futex syscall and can deadlock if the thread already
/// holds the lock). Combined with `SA_NODEFER` (set so a recursive trap
/// in the handler is visible rather than silently dropped), a nested
/// SIGSYS would deadlock on the mutex, hanging the guest permanently.
/// Dropping the `Mutex` makes the handler lock-free and reentrant-safe.
///
/// The `get_or_init` fallback in `classify()` only ever runs its closure
/// in unit tests (where `classify()` is called without `install()`); in
/// production `install()` populates the sets first, so the closure is
/// never executed from a signal context.
static TRAPPED_SET: OnceLock<HashSet<i32>> = OnceLock::new();
static KILLED_SET: OnceLock<HashSet<i32>> = OnceLock::new();

/// Install the seccomp filter on the calling thread (and all threads
/// in the calling process, via `SECCOMP_FILTER_FLAG_TSYNC`).
///
/// This is the entry point called from `main.rs` after all devices
/// are set up but before the guest init is exec'd.
///
/// Returns `Ok(())` on success, or `Err(errno)` on failure.
pub fn install() -> std::io::Result<()> {
    info!("[KR64][seccomp] installing seccomp filter");

    // 0. Snapshot the trapped/killed syscall sets into the static
    //    OnceLocks BEFORE installing the SIGSYS handler. The handler
    //    reads these via `get()` (a lock-free atomic load) — see the
    //    comment on `TRAPPED_SET`/`KILLED_SET` for why this must happen
    //    before `sigaction(SIGSYS, …)` and must not involve a `Mutex`.
    //    `set()` returns Err if already set (e.g. a prior `install()`
    //    call, or a unit test that touched `classify()`); that's fine,
    //    we just keep the existing snapshot.
    let _ = KILLED_SET.set(killed_syscalls());
    let _ = TRAPPED_SET.set(trapped_syscalls());

    // 1. PR_SET_NO_NEW_PRIVS — required before seccomp(2) without
    //    CAP_SYS_ADMIN. Has to be set on every thread; we use the
    //    TSYNC flag below to propagate to all threads in the process.
    let r = unsafe { libc::prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    if r != 0 {
        let e = std::io::Error::last_os_error();
        error!("[KR64][seccomp] PR_SET_NO_NEW_PRIVS failed: {}", e);
        return Err(e);
    }
    info!("[KR64][seccomp] PR_SET_NO_NEW_PRIVS set");

    // 2. Install the SIGSYS handler. We use SA_SIGINFO so we get the
    //    siginfo_t (with si_syscall) and the ucontext_t (so we can
    //    modify the return value and PC).
    install_sigsys_handler()?;

    // 3. Build the BPF program.
    let prog = build_filter();

    // 4. seccomp(SECCOMP_SET_MODE_FILTER, TSYNC, &prog).
    //    We use the raw `syscall()` wrapper because libc doesn't wrap
    //    `seccomp(2)` on every target.
    #[repr(C)]
    struct Fprog {
        len: u16,
        filter: *const SockFilter,
    }
    let fprog = Fprog {
        len: prog.len() as u16,
        filter: prog.as_ptr(),
    };

    // seccomp(2) is syscall 277 on aarch64, 317 on x86_64. Use libc's
    // SYS_seccomp constant (defined on all Android targets we support).
    let r = unsafe {
        libc::syscall(
            libc::SYS_seccomp,
            SECCOMP_SET_MODE_FILTER as c_int,
            SECCOMP_FILTER_FLAG_TSYNC,
            &fprog as *const Fprog,
        )
    };
    if r != 0 {
        let e = std::io::Error::last_os_error();
        error!("[KR64][seccomp] SECCOMP_SET_MODE_FILTER failed: {}", e);
        return Err(e);
    }

    info!(
        "[KR64][seccomp] seccomp filter installed ({} insns, TSYNC)",
        prog.len()
    );
    Ok(())
}

/// Install the SIGSYS handler that catches `SECCOMP_RET_TRAP` events.
fn install_sigsys_handler() -> std::io::Result<()> {
    let mut sa: sigaction = unsafe { std::mem::zeroed() };
    // sa_sigaction is a union with sa_handler; on Linux we set it to
    // the address of our trampoline. Cast through a raw pointer first
    // to satisfy the `function_casts_as_integer` lint (and to match
    // the `sighandler_t` typedef on glibc).
    sa.sa_sigaction = sigsys_handler as *const () as usize;
    // SA_SIGINFO — pass siginfo + ucontext to the handler.
    // SA_NODEFER  — don't block SIGSYS while in the handler (so a
    //               recursive trap in the handler is visible).
    // SA_RESTART is deliberately NOT set: interrupted syscalls must
    // surface EINTR to the guest, not be transparently restarted —
    // the SIGSYS handler performs its own PC/return-value fixup.
    sa.sa_flags = libc::SA_SIGINFO | libc::SA_NODEFER;
    // Don't block any other signals during the handler.
    unsafe { libc::sigemptyset(&mut sa.sa_mask as *mut sigset_t) };

    let r = unsafe { libc::sigaction(libc::SIGSYS, &sa, std::ptr::null_mut()) };
    if r != 0 {
        let e = std::io::Error::last_os_error();
        error!("[KR64][seccomp] sigaction(SIGSYS) failed: {}", e);
        return Err(e);
    }
    info!(
        "[KR64][seccomp] SIGSYS handler installed at {:p}",
        sigsys_handler as *const c_void
    );
    Ok(())
}

// ============================================================================
// The SIGSYS handler itself.
// ============================================================================

/// Kernel-side layout of `siginfo_t` when `si_signo == SIGSYS` (i.e. the
/// `SYS_SECCOMP` / `si_code == 1` path). See Linux's
/// `include/uapi/asm-generic/siginfo.h::__sifields.__sigsys`.
///
/// The libc crate's `siginfo_t` exposes accessors like `si_pid()` /
/// `si_uid()` / `si_addr()` / `si_value()` but does NOT expose
/// `si_syscall`, so we re-interpret the raw bytes through this layout.
/// This is safe because the kernel-userland `siginfo` ABI is fixed and
/// architecture-independent (the `_pad` field after `si_code` exists to
/// align the 64-bit `si_call_addr` on 64-bit platforms; on 32-bit it
/// collapses but is harmless).
#[repr(C)]
struct SigsysSiginfo {
    si_signo: i32,
    /// For seccomp traps this holds the seccomp return value
    /// (e.g. `SECCOMP_RET_TRAP`).
    si_errno: i32,
    /// `SYS_SECCOMP` (= 1).
    si_code: i32,
    /// Alignment padding so `si_call_addr` is 8-byte aligned.
    _pad: i32,
    /// Address of the syscall instruction that was trapped.
    si_call_addr: *mut c_void,
    /// The trapped syscall number (what we care about).
    si_syscall: i32,
    /// The audit architecture of the trapped syscall
    /// (e.g. `AUDIT_ARCH_AARCH64`).
    si_arch: u32,
}

/// SIGSYS handler — called by the kernel when a `SECCOMP_RET_TRAP` is
/// triggered (i.e. when the guest calls one of the "trapped" syscalls).
///
/// The handler:
///   1. Reads `si_syscall` to learn which syscall was trapped.
///   2. Looks the syscall up in the trapped/killed sets.
///   3. If killed: log and call `_exit(1)` (no return).
///   4. If trapped: emulate the syscall by:
///      a. Setting the return value register to 0 (success) — or
///      `-ENOSYS` if we don't have a real emulation yet.
///      b. Advancing the program counter past the syscall instruction
///      so the kernel doesn't retry it.
///   5. Returns; the kernel restores the (modified) context and
///      continues execution at the new PC.
///
/// # Architecture-specific notes
///
/// On aarch64:
///   - Return value register: `mcontext.regs[0]` (x0)
///   - PC register:           `mcontext.pc`
///   - Syscall instruction:   4 bytes (`svc #0` = `0xd4000001`)
///
/// On x86_64:
///   - Return value register: `mcontext.gregs[REG_RAX]` = `gregs[13]`
///   - PC register:           `mcontext.gregs[REG_RIP]` = `gregs[16]`
///   - Syscall instruction:   2 bytes (`syscall` = `0x0f 0x05`)
extern "C" fn sigsys_handler(_sig: c_int, info: *mut siginfo_t, ctx: *mut c_void) {
    // Reinterpret the siginfo_t pointer as our SIGSYS-specific layout.
    // This is safe because the kernel always uses this layout when
    // delivering SIGSYS from seccomp, regardless of how the libc crate
    // chooses to expose the union.
    let si = unsafe { &*(info as *const SigsysSiginfo) };
    let uc = unsafe { &mut *(ctx as *mut ucontext_t) };

    // Read the syscall number.
    let syscall_nr = si.si_syscall;

    // Decide what to do.
    let action = classify(syscall_nr);

    // 6-Z184 AUDIT FIX (agent 62): this handler runs in SIGNAL context —
    // error!/warning! expand to format! (malloc) + eprintln! (stdio lock),
    // which are NOT async-signal-safe: with TSYNC every guest thread
    // carries the filter, and a SIGSYS landing while another thread holds
    // the allocator/stdio lock deadlocks or corrupts the heap. Use the
    // crate's signal-safe write helpers instead (write(2) only).
    let mut nbuf = [0u8; 12];
    match action {
        Action::Kill => unsafe {
            crate::safe_write_err(
                b"[KR64][seccomp] BLOCKED.SYSCALL.FAILED: killed guest for syscall ",
            );
            let n = crate::format_decimal(&mut nbuf, syscall_nr);
            crate::safe_write_err(&nbuf[..n]);
            crate::safe_write_err(b"\n");
            libc::_exit(1);
        },
        Action::Emulate { retval } => {
            // Emulate: set return value, advance PC past syscall instr.
            unsafe {
                crate::safe_write_err(b"[KR64][seccomp] trapped syscall ");
                let n = crate::format_decimal(&mut nbuf, syscall_nr);
                crate::safe_write_err(&nbuf[..n]);
                crate::safe_write_err(b" -> emulated (retval=");
                let r = crate::format_decimal(&mut nbuf, retval as i32);
                crate::safe_write_err(&nbuf[..r]);
                crate::safe_write_err(b")\n");
            }
            set_return_value(uc, retval);
            advance_pc(uc);
        }
        Action::Passthrough => {
            // This shouldn't happen — only trapped syscalls reach the
            // handler. But if it does, note it signal-safely and let the
            // kernel retry (don't advance PC) so the syscall executes.
            unsafe {
                crate::safe_write_err(b"[KR64][seccomp] SIGSYS for non-trapped syscall ");
                let n = crate::format_decimal(&mut nbuf, syscall_nr);
                crate::safe_write_err(&nbuf[..n]);
                crate::safe_write_err(b" - passthrough\n");
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Kill,
    Emulate { retval: i64 },
    Passthrough,
}

/// Look the syscall up in the trapped/killed tables and decide what to
/// do. The mapping is:
///   - killed set → Action::Kill
///   - trapped set → Action::Emulate { retval: 0 } (or syscall-specific)
///   - anything else → Action::Passthrough (shouldn't happen)
fn classify(syscall_nr: i32) -> Action {
    // Read the snapshotted syscall sets. In production `install()` has
    // already populated both OnceLocks before the SIGSYS handler was
    // installed, so `get_or_init` takes the fast path (a single atomic
    // load) and never runs its closure. The closure only runs in unit
    // tests that call `classify()` directly without `install()`.
    //
    // This is async-signal-safe because:
    //   - `OnceLock::get_or_init` on an already-initialised lock is just
    //     an atomic load (no syscall, no allocation).
    //   - `HashSet::contains` is a read-only hash lookup.
    // The previous version wrapped the sets in `Mutex` and called
    // `.lock()` here, which is NOT async-signal-safe and could deadlock
    // under `SA_NODEFER` if a SIGSYS nested.
    let killed = KILLED_SET.get_or_init(killed_syscalls);
    if killed.contains(&syscall_nr) {
        return Action::Kill;
    }
    let trapped = TRAPPED_SET.get_or_init(trapped_syscalls);
    if trapped.contains(&syscall_nr) {
        // Per-syscall emulation. For now everything trapped is
        // emulated as success (retval = 0) — the production
        // version will dispatch to per-syscall handlers in
        // mount_mgr / netlink / etc.
        return Action::Emulate {
            retval: emulate_syscall(syscall_nr),
        };
    }
    Action::Passthrough
}

/// Per-syscall emulation — return a fake return value.
///
/// The dispatch below implements the documented intent of the original
/// `TODO: dispatch to per-syscall handlers` block. The cases that don't
/// need to inspect the syscall arguments (which would require reading
/// the saved register context — arch-specific and unsafe in the
/// SIGSYS handler) are implemented now:
///   - `swapon` / `swapoff` / `acct` → return 0 (no-op success — the
///     guest believes the operation worked, but the host is untouched).
///   - `reboot` → return `-EPERM` so the guest's `init/shutdown` code
///     sees a clean "permission denied" instead of a fake success.
///     Returning 0 here (the previous behaviour) made the guest think
///     the reboot succeeded, after which it would proceed to stop
///     services, sync, and finally call `reboot(RB_POWER_OFF)` again
///     — a confusing no-op loop that polluted logs and could trip
///     watchdogs. `-EPERM` matches what the kernel would return if
///     the caller lacked `CAP_SYS_BOOT`, which is exactly the
///     semantics we want inside the sandbox.
///
/// `mount` and `umount2` still return 0 — a real implementation needs
/// to read `args[0..4]` out of the saved `ucontext_t`, decode the
/// `char *` pointers in the guest's address space, and forward to
/// `mount_mgr::handle_mount` / `handle_umount`. That work is deferred
/// to the `MOUNT-2` task; for now the guest's `mount(2)` calls
/// silently succeed without affecting the host mount table, which
/// is the same effective behaviour as VM's skeleton.
fn emulate_syscall(syscall_nr: i32) -> i64 {
    // `libc::SYS_reboot` is a compile-time `c_long` constant that
    // resolves to the correct syscall number per target (aarch64 = 142,
    // x86_64 = 169), so the comparison is a single integer compare —
    // no allocation, no locking, safe to call from the async-signal
    // SIGSYS handler.
    if syscall_nr == libc::SYS_reboot as i32 {
        // EPERM = 1 on Linux; syscalls return -errno on failure.
        return -(libc::EPERM as i64);
    }
    // swapon, swapoff, acct, mount, umount2 — fake success.
    // See method doc for why each case is (intentionally) a no-op.
    0
}

/// Set the syscall return value in the saved context.
///
/// On aarch64: x0 (regs[0]) is the return value register.
/// On x86_64: rax (gregs[REG_RAX]) is the return value register.
#[cfg(target_arch = "aarch64")]
fn set_return_value(uc: &mut ucontext_t, retval: i64) {
    uc.uc_mcontext.regs[0] = retval as u64;
}

#[cfg(target_arch = "x86_64")]
fn set_return_value(uc: &mut ucontext_t, retval: i64) {
    // REG_RAX = 13 (from <sys/ucontext.h> on Linux x86_64).
    const REG_RAX: usize = 13;
    uc.uc_mcontext.gregs[REG_RAX] = retval;
}

/// Advance the program counter past the syscall instruction.
///
/// On aarch64: every instruction is 4 bytes (svc #0 = 0xd4000001).
/// On x86_64:  the syscall instruction is 2 bytes (0x0f 0x05).
#[cfg(target_arch = "aarch64")]
fn advance_pc(uc: &mut ucontext_t) {
    uc.uc_mcontext.pc = uc.uc_mcontext.pc.wrapping_add(4);
}

#[cfg(target_arch = "x86_64")]
fn advance_pc(uc: &mut ucontext_t) {
    // REG_RIP = 16 (from <sys/ucontext.h> on Linux x86_64).
    const REG_RIP: usize = 16;
    let rip = uc.uc_mcontext.gregs[REG_RIP];
    uc.uc_mcontext.gregs[REG_RIP] = rip.wrapping_add(2);
}

// ============================================================================
// Tests.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_built_without_panic() {
        let prog = build_filter();
        // Sanity: the program has at least the arch check + a default ret.
        assert!(prog.len() >= 4, "filter too short: {:?}", prog);
        // First instruction must be the arch load.
        assert_eq!(prog[0].code, BPF_LD | BPF_W | BPF_ABS);
        assert_eq!(prog[0].k, OFF_ARCH);
        // The arch gate must kill INLINE (insn 2 = jeq with jt=1,
        // insn 3 = KILL_PROCESS) — never a long patched jump, which
        // would truncate into the 8-bit jf field and let a wrong-arch
        // syscall land inside the allow chain (regression lock).
        assert_eq!(prog[1].code, BPF_JMP | BPF_JEQ | BPF_K);
        assert_eq!(prog[1].jt, 1);
        assert_eq!(prog[1].jf, 0);
        assert_eq!(prog[2].code, BPF_RET | BPF_K);
        assert_eq!(prog[2].k, SECCOMP_RET_KILL_PROCESS);
        // Last instruction is the default: ALLOW.
        let last = *prog.last().unwrap();
        assert_eq!(last.code, BPF_RET | BPF_K);
        assert_eq!(last.k, SECCOMP_RET_ALLOW);
        // No jump anywhere in the program may exceed 1 instruction.
        for insn in &prog {
            assert!(insn.jt <= 1 && insn.jf <= 1, "long jump: {insn:?}");
        }
    }

    #[test]
    fn allowed_set_contains_read_write() {
        let allowed = allowed_syscalls();
        assert!(allowed.contains(&(libc::SYS_read as i32)));
        assert!(allowed.contains(&(libc::SYS_write as i32)));
        assert!(allowed.contains(&(libc::SYS_openat as i32)));
        assert!(allowed.contains(&(libc::SYS_close as i32)));
        assert!(allowed.contains(&(libc::SYS_mmap as i32)));
        assert!(allowed.contains(&(libc::SYS_ioctl as i32)));
    }

    #[test]
    fn z108_rt_sigaction_allowed_not_trapped_not_killed() {
        // Task 6-Z108 (SIGCHLD depth): rt_sigaction must EXECUTE with
        // REAL kernel semantics for the guest — never trapped (SIGSYS
        // emulation) and never killed. This is load-bearing for
        // SIGCHLD: a guest that registers SIG_IGN for SIGCHLD relies
        // on the KERNEL auto-reaping its children (POSIX SIG_IGN
        // semantics — no zombies, no wait4). If this syscall were
        // faked instead of executed, the real disposition would never
        // be set and the children would zombify forever — the aosp16
        // "livelock Z" shape. The default-ALLOW fallthrough would
        // cover it too, but the explicit allow-list entry (plus this
        // lock) documents the intent and guards against a future
        // deny-by-default tightening. ptrace_emu.rs mirrors this on
        // its side: nr 13/67/174 are NOT in
        // compute_exit_return_value (test-locked there), so the only
        // fake that can ever touch rt_sigaction is the i386
        // never-executed ENOSYS fallback, which RECORDS the
        // registration for diagnostics.
        #[cfg(target_arch = "x86_64")]
        assert_eq!(
            libc::SYS_rt_sigaction,
            13,
            "x86_64 rt_sigaction must be nr 13"
        );
        let allowed = allowed_syscalls();
        assert!(
            allowed.contains(&(libc::SYS_rt_sigaction as i32)),
            "SYS_rt_sigaction must be in the allow-list (6-Z108: real sigaction semantics, incl. SIGCHLD auto-reap under SIG_IGN)"
        );
        let trapped = trapped_syscalls();
        assert!(!trapped.contains(&(libc::SYS_rt_sigaction as i32)));
        let killed = killed_syscalls();
        assert!(!killed.contains(&(libc::SYS_rt_sigaction as i32)));
    }

    #[test]
    fn z109_signalfd_pidfd_kill_family_allowed_not_trapped_not_killed() {
        // Task 6-Z109 (signalfd + rt_sigprocmask depth): the whole
        // signal-DISPATCH path Android 11 init depends on must EXECUTE
        // with real kernel semantics — never trapped (SIGSYS emulation)
        // and never killed:
        //   - rt_sigprocmask: init blocks SIGCHLD before creating the
        //     signalfd (signalfd semantics REQUIRE the signal blocked).
        //     If the mask syscall were faked, the kernel would keep
        //     DELIVERING SIGCHLD while init waits on a signalfd that
        //     never becomes readable — events lost both ways.
        //   - signalfd4: init's signal_handler_init FATALs on a failed
        //     signalfd ("failed to create signalfd"); allowing it is
        //     process-scoped + safe (the fd can only observe the
        //     caller's OWN blocked signals).
        //   - kill/tgkill/tkill: init's service-restart hammer. The
        //     brief's item 4: "kill/tgkill FROM the guest to itself
        //     must execute for real" — the ptrace reaper (6-Z106/108)
        //     hands statuses back; the guest's kill must reach the
        //     kernel or service stop/restart loops spin.
        //   - pidfd_open/pidfd_send_signal: same class (process-scoped,
        //     per-signal permission-checked); Android 11 init does not
        //     use them yet, newer releases do — allow-listed for the
        //     same reason as signalfd4.
        // The filter's DEFAULT is ALLOW, so these entries are intent
        // documentation + a guard against a future deny-by-default
        // tightening — the same belt-and-suspenders lock as 6-Z108's
        // rt_sigaction entry.
        #[cfg(target_arch = "x86_64")]
        {
            assert_eq!(libc::SYS_signalfd4, 289, "x86_64 signalfd4 must be nr 289");
            assert_eq!(libc::SYS_signalfd, 282, "x86_64 signalfd must be nr 282");
            assert_eq!(libc::SYS_kill, 62, "x86_64 kill must be nr 62");
            assert_eq!(libc::SYS_tgkill, 234, "x86_64 tgkill must be nr 234");
            assert_eq!(libc::SYS_pidfd_send_signal, 424);
            assert_eq!(libc::SYS_pidfd_open, 434);
            assert_eq!(
                libc::SYS_rt_sigprocmask,
                14,
                "x86_64 rt_sigprocmask must be nr 14"
            );
        }
        let allowed = allowed_syscalls();
        let trapped = trapped_syscalls();
        let killed = killed_syscalls();
        for nr in [
            libc::SYS_rt_sigprocmask,
            libc::SYS_rt_sigpending,
            libc::SYS_rt_sigtimedwait,
            libc::SYS_kill,
            libc::SYS_tgkill,
            libc::SYS_tkill,
            libc::SYS_signalfd4,
            libc::SYS_pidfd_open,
            libc::SYS_pidfd_send_signal,
        ] {
            let nr = nr as i32;
            assert!(
                allowed.contains(&nr),
                "SYS nr {} must be in the allow-list (6-Z109: init's signal path executes for real)",
                nr
            );
            assert!(!trapped.contains(&nr), "SYS nr {} must NOT be trapped", nr);
            assert!(!killed.contains(&nr), "SYS nr {} must NOT be killed", nr);
        }
        // The x86-only legacy variant, where the libc crate defines it.
        #[cfg(target_arch = "x86_64")]
        {
            assert!(allowed.contains(&(libc::SYS_signalfd as i32)));
            assert!(!trapped.contains(&(libc::SYS_signalfd as i32)));
            assert!(!killed.contains(&(libc::SYS_signalfd as i32)));
        }
    }

    #[test]
    fn trapped_set_contains_mount_umount() {
        let trapped = trapped_syscalls();
        assert!(trapped.contains(&(libc::SYS_mount as i32)));
        assert!(trapped.contains(&(libc::SYS_umount2 as i32)));
    }

    #[test]
    fn killed_set_contains_ptrace_kexec() {
        let killed = killed_syscalls();
        assert!(killed.contains(&(libc::SYS_ptrace as i32)));
        assert!(killed.contains(&(libc::SYS_kexec_load as i32)));
    }

    #[test]
    fn classify_mount_is_emulated() {
        let action = classify(libc::SYS_mount as i32);
        match action {
            Action::Emulate { retval } => assert_eq!(retval, 0),
            other => panic!("expected Emulate, got {:?}", other),
        }
    }

    #[test]
    fn classify_reboot_returns_eperm() {
        // reboot(2) is trapped (not killed) so the guest's `init`
        // shutdown path can complete cleanly — but the emulated return
        // value must be -EPERM, NOT 0, otherwise the guest thinks the
        // host actually rebooted and starts its shutdown sequence
        // (stopping services, syncing, then calling reboot() again in
        // a loop). -EPERM matches the kernel's behaviour when the
        // caller lacks CAP_SYS_BOOT.
        let action = classify(libc::SYS_reboot as i32);
        match action {
            Action::Emulate { retval } => {
                assert_eq!(retval, -(libc::EPERM as i64));
            }
            other => panic!("expected Emulate, got {:?}", other),
        }
    }

    #[test]
    fn classify_swapon_returns_zero() {
        // swapon(2) is trapped and emulated as a no-op success — the
        // guest believes it added swap, but the host's swap is
        // untouched. Other no-op-emulated syscalls (swapoff, acct,
        // umount2) share the same retval; we only assert swapon here
        // to keep the test suite focused.
        let action = classify(libc::SYS_swapon as i32);
        match action {
            Action::Emulate { retval } => assert_eq!(retval, 0),
            other => panic!("expected Emulate, got {:?}", other),
        }
    }

    #[test]
    fn classify_ptrace_is_killed() {
        let action = classify(libc::SYS_ptrace as i32);
        match action {
            Action::Kill => {}
            other => panic!("expected Kill, got {:?}", other),
        }
    }

    #[test]
    fn classify_read_is_passthrough() {
        // read is allowed (not trapped), so SIGSYS for it is a "shouldn't happen".
        let action = classify(libc::SYS_read as i32);
        match action {
            Action::Passthrough => {}
            other => panic!("expected Passthrough, got {:?}", other),
        }
    }
}
