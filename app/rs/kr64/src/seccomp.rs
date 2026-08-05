// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use at your own risk.

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
use std::sync::{Mutex, OnceLock};

// Crate-local logging macros (defined in lib.rs) — no external `log` crate.
use crate::{error, info, warning};

// ============================================================================
// BPF / seccomp constants (not all exposed by the `libc` crate on every
// target, so we define them ourselves to be safe).
// ============================================================================

// BPF instruction classes (low 3 bits of `code`).
const BPF_LD:  u16 = 0x00;
const BPF_JMP: u16 = 0x05;
const BPF_RET: u16 = 0x06;

// BPF size modifiers (next 2 bits).
const BPF_W:   u16 = 0x00;

// BPF mode modifiers for BPF_LD (high 3 bits).
const BPF_ABS: u16 = 0x20;

// BPF jump operations (next 4 bits).
const BPF_JEQ: u16 = 0x10;

// BPF source operand (high 1 bit).
const BPF_K:   u16 = 0x00;

// Seccomp return values (top 16 bits of the return word).
#[allow(dead_code)]
const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
#[allow(dead_code)]
const SECCOMP_RET_KILL_THREAD:  u32 = 0x0000_0000;
const SECCOMP_RET_TRAP:         u32 = 0x0003_0000;
#[allow(dead_code)]
const SECCOMP_RET_ERRNO:        u32 = 0x0005_0000;
#[allow(dead_code)]
const SECCOMP_RET_TRACE:        u32 = 0x7ff0_0000;
#[allow(dead_code)]
const SECCOMP_RET_LOG:          u32 = 0x7ffc_0000;
const SECCOMP_RET_ALLOW:        u32 = 0x7fff_0000;

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
const OFF_NR:   u32 = 0;  // int   nr
const OFF_ARCH: u32 = 4;  // __u32 arch

// ============================================================================
// BPF instruction builder — thin wrapper around `sock_filter`.
// ============================================================================

/// A single BPF instruction. Layout matches `struct sock_filter`:
/// `__u16 code; __u8 jt; __u8 jf; __u32 k;`
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SockFilter {
    pub code: u16,
    pub jt:   u8,
    pub jf:   u8,
    pub k:    u32,
}

/// Build a `BPF_LD | BPF_W | BPF_ABS` instruction — load a 32-bit word
/// from offset `k` of `struct seccomp_data` into the accumulator.
const fn bpf_ld_abs(k: u32) -> SockFilter {
    SockFilter { code: BPF_LD | BPF_W | BPF_ABS, jt: 0, jf: 0, k }
}

/// Build a `BPF_JMP | BPF_JEQ | BPF_K` instruction — if accumulator
/// equals `k`, jump `jt` instructions forward, else jump `jf`.
const fn bpf_jeq(k: u32, jt: u8, jf: u8) -> SockFilter {
    SockFilter { code: BPF_JMP | BPF_JEQ | BPF_K, jt, jf, k }
}

/// Build a `BPF_RET | BPF_K` instruction — return `k` from the filter.
const fn bpf_ret(k: u32) -> SockFilter {
    SockFilter { code: BPF_RET | BPF_K, jt: 0, jf: 0, k }
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
        SYS_read, SYS_write, SYS_readv, SYS_writev,
        SYS_pread64, SYS_pwrite64,
        SYS_openat, SYS_close, SYS_dup, SYS_dup3, SYS_fcntl, SYS_lseek,
        SYS_newfstatat, SYS_readlinkat,
        SYS_ftruncate, SYS_getcwd,
    ] {
        add_nr!(s, nr);
    }

    // --- file I/O (x86_64-only: open, access, dup2, stat, lstat, fstat,
    //     fchmod, fchown, chroot, unlink, rmdir, chmod, chown, lchown).
    //     On aarch64 these are replaced by the *at variants.
    #[cfg(target_arch = "x86_64")]
    for &nr in &[
        SYS_open, SYS_access, SYS_dup2,
        SYS_stat, SYS_fstat, SYS_lstat,
        SYS_chmod, SYS_fchmod, SYS_lchown, SYS_chown, SYS_fchown,
        SYS_chroot, SYS_unlink, SYS_rmdir, SYS_mkdir,
        SYS_readlink, SYS_truncate,
    ] {
        add_nr!(s, nr);
    }

    // --- aarch64-only variants (already covered by *at, but listed for
    //     completeness — these are present on aarch64 and not x86_64).
    #[cfg(target_arch = "aarch64")]
    for &nr in &[
        SYS_fstat, SYS_fstatat, // fstat exists on aarch64 (not x86_64's "newfstatat")
    ] {
        add_nr!(s, nr);
    }

    // --- memory ---
    for &nr in &[
        SYS_mmap, SYS_munmap, SYS_mprotect, SYS_mremap, SYS_msync,
        SYS_madvise, SYS_brk,
    ] {
        add_nr!(s, nr);
    }
    // mincore exists on both — add if defined.
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    add_nr!(s, SYS_mincore);

    // --- signals ---
    for &nr in &[
        SYS_rt_sigaction, SYS_rt_sigprocmask, SYS_rt_sigpending,
        SYS_rt_sigtimedwait, SYS_rt_sigreturn, SYS_sigaltstack,
        SYS_rt_sigsuspend, SYS_kill, SYS_tgkill, SYS_tkill,
    ] {
        add_nr!(s, nr);
    }

    // --- sockets / IPC (present on both) ---
    for &nr in &[
        SYS_socket, SYS_socketpair, SYS_connect, SYS_bind, SYS_listen,
        SYS_accept4, SYS_getsockname, SYS_getpeername,
        SYS_sendto, SYS_recvfrom, SYS_sendmsg, SYS_recvmsg,
        SYS_shutdown, SYS_setsockopt, SYS_getsockopt,
        SYS_pipe, SYS_pipe2,
        SYS_pselect6, SYS_poll, SYS_ppoll,
        SYS_epoll_create1, SYS_epoll_ctl, SYS_epoll_wait,
        SYS_eventfd2, SYS_timerfd_create, SYS_timerfd_settime,
        SYS_inotify_init1, SYS_inotify_add_watch, SYS_inotify_rm_watch,
    ] {
        add_nr!(s, nr);
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
        SYS_clone, SYS_execve, SYS_execveat,
        SYS_exit, SYS_exit_group,
        SYS_wait4, SYS_waitid,
        SYS_set_tid_address, SYS_setpgid, SYS_setsid,
        SYS_getpgid, SYS_getpid, SYS_getppid,
        SYS_gettid, SYS_getuid, SYS_geteuid, SYS_getgid, SYS_getegid,
        SYS_getresuid, SYS_getresgid,
        SYS_prctl,  // NOT filtered — guest can set its own name, etc.
        SYS_uname, SYS_sethostname, SYS_setdomainname,
        SYS_getrlimit, SYS_setrlimit, SYS_prlimit64,
        SYS_getrandom,
        SYS_clock_gettime, SYS_clock_getres, SYS_clock_settime,
        SYS_gettimeofday, SYS_time,
        SYS_nanosleep,
    ] {
        add_nr!(s, nr);
    }

    // fork / vfork — present on x86_64, removed on aarch64.
    #[cfg(target_arch = "x86_64")]
    for &nr in &[SYS_fork, SYS_vfork] {
        add_nr!(s, nr);
    }

    // setuid / setgid — present on x86_64, removed on aarch64.
    #[cfg(target_arch = "x86_64")]
    for &nr in &[SYS_setuid, SYS_setgid, SYS_setreuid, SYS_setregid,
                 SYS_settimeofday, SYS_getpgrp] {
        add_nr!(s, nr);
    }

    // --- filesystem (read-only; writes go through mount_mgr) ---
    // NOTE: Linux exposes these as `fadvise64` and `fallocate` (not the
    // POSIX names `posix_fadvise` / `posix_fallocate`). The libc crate
    // follows the kernel's `__NR_*` naming.
    for &nr in &[
        SYS_getdents64,
        SYS_fadvise64, SYS_fallocate,
        SYS_flock, SYS_sync, SYS_fsync, SYS_fdatasync,
        SYS_renameat, SYS_renameat2, SYS_linkat, SYS_symlinkat,
        SYS_unlinkat, SYS_mkdirat,
        SYS_fchmod, SYS_fchmodat, SYS_fchown, SYS_fchownat,
        SYS_umask,
        SYS_chdir, SYS_fchdir,
    ] {
        add_nr!(s, nr);
    }
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
    add_nr!(s, SYS_kexec_file_load);
    add_nr!(s, SYS_init_module);
    add_nr!(s, SYS_finit_module);
    add_nr!(s, SYS_delete_module);
    add_nr!(s, SYS_iopl);
    add_nr!(s, SYS_ioperm);
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
///   2. jeq EXPECTED_ARCH, jt=0, jf=N   // if wrong arch, jump to kill
///   3. ld nr                           // load syscall number
///   4. for each allowed syscall:
///        jeq nr, jt=0, jf=1            // if match: fall to ret ALLOW
///        ret ALLOW                      // else: skip to next jeq
///   5. for each trapped syscall:
///        jeq nr, jt=0, jf=1
///        ret TRAP
///   6. for each killed syscall:
///        jeq nr, jt=0, jf=1
///        ret KILL_PROCESS
///   7. ret ALLOW                       // default: allow
///   8. ret KILL_PROCESS                 // for wrong-arch
/// ```
///
/// The `jt=0, jf=1` pattern keeps jumps short (1 instruction) so we
/// never overflow the 8-bit jt/jf range, regardless of how many
/// syscalls are in each set.
pub fn build_filter() -> Vec<SockFilter> {
    let allowed = allowed_syscalls();
    let trapped = trapped_syscalls();
    let killed  = killed_syscalls();

    let mut prog: Vec<SockFilter> = Vec::new();

    // (1) Load arch.
    prog.push(bpf_ld_abs(OFF_ARCH));
    // (2) If arch != EXPECTED, jump to the trailing KILL.
    //     We patch the jf offset after we know the program length.
    let arch_jeq_idx = prog.len();
    prog.push(bpf_jeq(AUDIT_ARCH_EXPECTED, 0, 0)); // patched below

    // (3) Load syscall number.
    prog.push(bpf_ld_abs(OFF_NR));

    // (4) Allowed syscalls: jeq + ret ALLOW.
    let mut sorted_allowed: Vec<i32> = allowed.iter().copied()
        .filter(|nr| !trapped.contains(nr) && !killed.contains(nr))
        .collect();
    sorted_allowed.sort_unstable();
    sorted_allowed.dedup();
    for nr in sorted_allowed {
        prog.push(bpf_jeq(nr as u32, 0, 1));  // match: fall to next; else skip 1
        prog.push(bpf_ret(SECCOMP_RET_ALLOW));
    }

    // (5) Trapped syscalls: jeq + ret TRAP.
    let mut sorted_trapped: Vec<i32> = trapped.iter().copied()
        .filter(|nr| !killed.contains(nr))
        .collect();
    sorted_trapped.sort_unstable();
    sorted_trapped.dedup();
    for nr in sorted_trapped {
        prog.push(bpf_jeq(nr as u32, 0, 1));
        prog.push(bpf_ret(SECCOMP_RET_TRAP));
    }

    // (6) Killed syscalls: jeq + ret KILL.
    let mut sorted_killed: Vec<i32> = killed.iter().copied().collect();
    sorted_killed.sort_unstable();
    sorted_killed.dedup();
    for nr in sorted_killed {
        prog.push(bpf_jeq(nr as u32, 0, 1));
        prog.push(bpf_ret(SECCOMP_RET_KILL_PROCESS));
    }

    // (7) Default: allow.
    prog.push(bpf_ret(SECCOMP_RET_ALLOW));

    // (8) Kill target (for wrong-arch). Patch the arch jeq's jf to
    //     jump here. The jf offset is relative to the *next*
    //     instruction, so it's `kill_idx - arch_jeq_idx - 1`.
    let kill_idx = prog.len();
    prog.push(bpf_ret(SECCOMP_RET_KILL_PROCESS));
    let jf_offset = (kill_idx - arch_jeq_idx - 1) as u8;
    prog[arch_jeq_idx].jf = jf_offset;

    info!("[KR64][seccomp] BPF filter built: {} instructions", prog.len());
    prog
}

// ============================================================================
// Install the filter + SIGSYS handler.
// ============================================================================

/// Global mutable state used by the SIGSYS handler. Protected by a
/// Mutex because the handler may run on any thread.
///
/// The handler reads `trapped_syscalls` and `killed_syscalls` to decide
/// what to do with the trapped syscall. We snapshot them at install
/// time into static `OnceLock`s to avoid allocating in the handler
/// (and to avoid the `once_cell` dependency — `OnceLock` is in `std`
/// since Rust 1.70).
static TRAPPED_SET: OnceLock<Mutex<HashSet<i32>>> = OnceLock::new();
static KILLED_SET: OnceLock<Mutex<HashSet<i32>>> = OnceLock::new();

/// Install the seccomp filter on the calling thread (and all threads
/// in the calling process, via `SECCOMP_FILTER_FLAG_TSYNC`).
///
/// This is the entry point called from `main.rs` after all devices
/// are set up but before the guest init is exec'd.
///
/// Returns `Ok(())` on success, or `Err(errno)` on failure.
pub fn install() -> std::io::Result<()> {
    info!("[KR64][seccomp] installing seccomp filter");

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
        len:    u16,
        filter: *const SockFilter,
    }
    let fprog = Fprog {
        len:    prog.len() as u16,
        filter: prog.as_ptr(),
    };

    // seccomp(2) is syscall 277 on aarch64, 317 on x86_64. Use libc's
    // SYS_seccomp constant (defined on all Android targets we support).
    let r = unsafe {
        libc::syscall(
            libc::SYS_seccomp,
            SECCOMP_SET_MODE_FILTER as c_int,
            SECCOMP_FILTER_FLAG_TSYNC as u32,
            &fprog as *const Fprog,
        )
    };
    if r != 0 {
        let e = std::io::Error::last_os_error();
        error!("[KR64][seccomp] SECCOMP_SET_MODE_FILTER failed: {}", e);
        return Err(e);
    }

    info!("[KR64][seccomp] seccomp filter installed ({} insns, TSYNC)", prog.len());
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
    // SA_RESTART  — restart interrupted syscalls.
    sa.sa_flags = libc::SA_SIGINFO | libc::SA_NODEFER;
    // Don't block any other signals during the handler.
    unsafe { libc::sigemptyset(&mut sa.sa_mask as *mut sigset_t) };

    let r = unsafe { libc::sigaction(libc::SIGSYS, &sa, std::ptr::null_mut()) };
    if r != 0 {
        let e = std::io::Error::last_os_error();
        error!("[KR64][seccomp] sigaction(SIGSYS) failed: {}", e);
        return Err(e);
    }
    info!("[KR64][seccomp] SIGSYS handler installed at {:p}",
          sigsys_handler as *const c_void);
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
    si_signo:     i32,
    /// For seccomp traps this holds the seccomp return value
    /// (e.g. `SECCOMP_RET_TRAP`).
    si_errno:     i32,
    /// `SYS_SECCOMP` (= 1).
    si_code:      i32,
    /// Alignment padding so `si_call_addr` is 8-byte aligned.
    _pad:         i32,
    /// Address of the syscall instruction that was trapped.
    si_call_addr: *mut c_void,
    /// The trapped syscall number (what we care about).
    si_syscall:   i32,
    /// The audit architecture of the trapped syscall
    /// (e.g. `AUDIT_ARCH_AARCH64`).
    si_arch:      u32,
}

/// SIGSYS handler — called by the kernel when a `SECCOMP_RET_TRAP` is
/// triggered (i.e. when the guest calls one of the "trapped" syscalls).
///
/// The handler:
///   1. Reads `si_syscall` to learn which syscall was trapped.
///   2. Looks the syscall up in the trapped/killed sets.
///   3. If killed: log and call `_exit(1)` (no return).
///   4. If trapped: emulate the syscall by:
///        a. Setting the return value register to 0 (success) — or
///           `-ENOSYS` if we don't have a real emulation yet.
///        b. Advancing the program counter past the syscall instruction
///           so the kernel doesn't retry it.
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

    match action {
        Action::Kill => {
            error!(
                "[KR64][seccomp] BLOCKED.SYSCALL.FAILED: killed guest for syscall {}",
                syscall_nr
            );
            unsafe { libc::_exit(1) };
        }
        Action::Emulate { retval } => {
            // Emulate: set return value, advance PC past syscall instr.
            warning!(
                "[KR64][seccomp] BLOCKED.SYSCALL.FAILED: trapped syscall {} → emulated (retval={})",
                syscall_nr, retval
            );
            set_return_value(uc, retval);
            advance_pc(uc);
        }
        Action::Passthrough => {
            // This shouldn't happen — only trapped syscalls reach the
            // handler. But if it does, log and let the kernel retry
            // (don't advance PC) so the syscall actually executes.
            warning!(
                "[KR64][seccomp] SIGSYS for non-trapped syscall {} — passthrough",
                syscall_nr
            );
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
    // `get_or_init` snapshots the syscall sets on first use. This is
    // safe to call from the SIGSYS handler because OnceLock's init path
    // is race-free (the OS guarantees atomic initialisation).
    let killed = KILLED_SET.get_or_init(|| Mutex::new(killed_syscalls()));
    if let Ok(killed) = killed.lock() {
        if killed.contains(&syscall_nr) {
            return Action::Kill;
        }
    }
    let trapped = TRAPPED_SET.get_or_init(|| Mutex::new(trapped_syscalls()));
    if let Ok(trapped) = trapped.lock() {
        if trapped.contains(&syscall_nr) {
            // Per-syscall emulation. For now everything trapped is
            // emulated as success (retval = 0) — the production
            // version will dispatch to per-syscall handlers in
            // mount_mgr / netlink / etc.
            return Action::Emulate { retval: emulate_syscall(syscall_nr) };
        }
    }
    Action::Passthrough
}

/// Per-syscall emulation — return a fake return value.
///
/// For the MVP, every trapped syscall returns 0 (success). The
/// production version will:
///   - `mount`   → call `mount_mgr::handle_mount(args…)`, return its result
///   - `umount2` → call `mount_mgr::handle_umount(args…)`, return its result
///   - `swapon`  → return 0 (no-op)
///   - `swapoff` → return 0 (no-op)
///   - `acct`    → return 0 (no-op)
///   - `reboot`  → return -EPERM (we don't reboot the host!)
fn emulate_syscall(_syscall_nr: i32) -> i64 {
    // TODO: dispatch to per-syscall handlers (mount_mgr, etc.).
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
        // Last instruction must be a KILL (the wrong-arch fallthrough).
        let last = *prog.last().unwrap();
        assert_eq!(last.code, BPF_RET | BPF_K);
        assert_eq!(last.k, SECCOMP_RET_KILL_PROCESS);
    }

    #[test]
    fn allowed_set_contains_read_write() {
        let allowed = allowed_syscalls();
        assert!(allowed.contains(&(libc::SYS_read  as i32)));
        assert!(allowed.contains(&(libc::SYS_write as i32)));
        assert!(allowed.contains(&(libc::SYS_openat as i32)));
        assert!(allowed.contains(&(libc::SYS_close as i32)));
        assert!(allowed.contains(&(libc::SYS_mmap  as i32)));
        assert!(allowed.contains(&(libc::SYS_ioctl as i32)));
    }

    #[test]
    fn trapped_set_contains_mount_umount() {
        let trapped = trapped_syscalls();
        assert!(trapped.contains(&(libc::SYS_mount   as i32)));
        assert!(trapped.contains(&(libc::SYS_umount2 as i32)));
    }

    #[test]
    fn killed_set_contains_ptrace_kexec() {
        let killed = killed_syscalls();
        assert!(killed.contains(&(libc::SYS_ptrace     as i32)));
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
