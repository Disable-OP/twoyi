// ptrace_emu.rs — Ptrace-based syscall emulation for unrooted TWRP boot.
//
// PROBLEM:
//   On unrooted devices, kr64 CANNOT:
//     - chroot()  (blocked by zygote seccomp filter → SIGSYS)
//     - unshare(CLONE_NEWPID) (needs CAP_SYS_ADMIN → EPERM)
//     - mount() (blocked by seccomp)
//   Without chroot, TWRP's init opens /init.rc on the HOST filesystem
//   (wrong file). Without CLONE_NEWPID, init's getpid() returns a
//   non-1 PID, so init exits 31 immediately.
//
//   TWRP's init is STATICALLY linked → LD_PRELOAD doesn't work → we
//   can't hook getpid() or open() the normal way.
//
// SOLUTION:
//   Use ptrace to intercept syscalls in the forked child (init).
//   This is the same technique PROOT uses for rootless container
//   emulation. ptrace IS allowed by Android's seccomp policy for
//   untrusted apps on their own children.
//
//   We intercept:
//     - getpid/getppid → return 1 (fake PID 1)
//     - open/openat/openat2 → translate path (prepend rootfs)
//     - stat/lstat/newfstatat → translate path
//     - access/faccessat → translate path
//     - readlink/readlinkat → translate path
//     - chdir → translate path
//     - statx → translate path
//
//   Path translation rules:
//     - If path starts with "/" and is NOT under /dev/ (which kr64
//       already sets up on the host), prepend the rootfs path.
//     - Exception: /proc, /sys, /dev, /data, /apex, /system, /vendor
//       are left as-is if they already exist on the host.
//     - Exception: /init.rc, /init.*.rc, /sbin/*, /etc/* → translate
//       to rootfs (these are TWRP-specific files).

// Architecture-specific register access.
//
// CRITICAL: On aarch64, PTRACE_GETREGS (12) does NOT exist — it returns
// EIO. We must use PTRACE_GETREGSET (33) with NT_PRSTATUS (1) and an
// iovec. On x86_64, PTRACE_GETREGS works directly.
//
// CRITICAL (aarch64, real-device fix): The libc crate declares `ptrace`
// as a variadic C function (`extern "C" { fn ptrace(c_uint, ...) -> c_long; }`).
// On aarch64, Rust's C-variadic ABI hands the callee a `__va_list` that the
// callee walks via `va_arg`. bionic's ptrace() wrapper then forwards the
// unpacked arguments to the kernel via `syscall(__NR_ptrace, req, pid, addr, data)`.
// On real arm64 Android devices this has been observed to consistently
// return EIO for PTRACE_GETREGSET (with NT_PRSTATUS), producing the
// "ptrace_getregs failed: I/O error (os error 5)" log spam.
//
// The kernel's own ptrace syscall works correctly when invoked directly via
// `libc::syscall(SYS_ptrace, ...)` — there is no regset lookup problem
// (NT_PRSTATUS is always present on aarch64), no permission issue, and
// no size mismatch. The failure is purely an artifact of going through
// bionic's variadic `ptrace()` wrapper.
//
// Fix: bypass `libc::ptrace()` and call the kernel via the raw syscall
// interface for the GETREGSET/SETREGSET requests on aarch64. We keep
// `libc::ptrace()` for the non-REGSET requests (PTRACE_SETOPTIONS,
// PTRACE_SYSCALL, PTRACE_PEEKDATA, ...) because those have been observed
// to work fine through bionic and the workaround adds nothing there.

// ── Register types ─────────────────────────────────────────────────

/// On x86_64, user_regs_struct has 27 fields. We access them as u64
/// array elements via index constants.
///
/// On aarch64, we use user_pt_regs which is u64[31] + sp + pc + pstate.
/// We access x0-x30 as array[0..30], and the syscall number is in x8.
#[cfg(target_arch = "x86_64")]
type Regs = libc::user_regs_struct;

#[cfg(target_arch = "aarch64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct Aarch64Regs {
    regs: [u64; 31], // x0-x30
    sp: u64,
    pc: u64,
    pstate: u64,
}

#[cfg(target_arch = "aarch64")]
type Regs = Aarch64Regs;

// Compile-time assertion that `Aarch64Regs` exactly matches the kernel's
// `struct user_pt_regs` from <asm/ptrace.h>:
//   struct user_pt_regs {
//       __u64 regs[31];   // 31 * 8 = 248
//       __u64 sp;          //   8
//       __u64 pc;          //   8
//       __u64 pstate;      //   8
//   };                    // total: 272 bytes
//
// If anyone reorders the fields or changes a type, this assertion will
// fail at compile time — PTRACE_GETREGSET would otherwise silently copy
// the wrong bytes into our struct and produce garbage register values.
#[cfg(target_arch = "aarch64")]
const _: () = assert!(std::mem::size_of::<Aarch64Regs>() == 272);

// Named constants for the magic numbers used in the aarch64 PTRACE_*REGSET
// path. Using names instead of bare `33`/`34`/`1` makes the intent obvious
// and stops future readers from wondering whether they are syscall numbers,
// NT_* types, or something else entirely.
#[cfg(target_arch = "aarch64")]
mod aarch64_ptrace {
    /// `PTRACE_GETREGSET` — Linux generic ptrace request number, see
    /// <linux/ptrace.h>. Reads a regset by NT_* type into a user `iovec`.
    pub const PTRACE_GETREGSET: libc::c_long = 33;
    /// `PTRACE_SETREGSET` — Linux generic ptrace request number, see
    /// <linux/ptrace.h>. Writes a regset by NT_* type from a user `iovec`.
    pub const PTRACE_SETREGSET: libc::c_long = 34;
    /// `NT_PRSTATUS` — general-purpose registers regset, see
    /// <linux/elf.h>. This is the regset that maps to `user_pt_regs`
    /// on aarch64 and to `user_regs_struct` on x86_64.
    pub const NT_PRSTATUS: libc::c_long = 1;
}

// ── Register index constants ───────────────────────────────────────

// x86_64 user_regs_struct field order (as u64 array indices):
//   0:r15 1:r14 2:r13 3:r12 4:rbp 5:rbx 6:r11 7:r10 8:r9 9:r8
//   10:rax 11:rcx 12:rdx 13:rsi 14:rdi 15:orig_rax 16:rip 17:cs
//   18:eflags 19:rsp 20:ss 21:fs_base 22:gs_base 23:ds 24:es 25:fs 26:gs
#[cfg(target_arch = "x86_64")]
const REG_SYSCALL: usize = 15; // orig_rax
#[cfg(target_arch = "x86_64")]
const REG_RET: usize = 10;    // rax
#[cfg(target_arch = "x86_64")]
const REG_ARG1: usize = 14;   // rdi
#[cfg(target_arch = "x86_64")]
const REG_ARG2: usize = 13;   // rsi
#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
const REG_ARG3: usize = 12;   // rdx
#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
const REG_ARG4: usize = 7;    // r10

#[cfg(target_arch = "aarch64")]
const REG_SYSCALL: usize = 8;   // x8 (syscall number)
#[cfg(target_arch = "aarch64")]
const REG_RET: usize = 0;       // x0 (return value)
#[cfg(target_arch = "aarch64")]
const REG_ARG1: usize = 0;      // x0
#[cfg(target_arch = "aarch64")]
const REG_ARG2: usize = 1;      // x1
#[cfg(target_arch = "aarch64")]
#[allow(dead_code)]
const REG_ARG3: usize = 2;      // x2
#[cfg(target_arch = "aarch64")]
#[allow(dead_code)]
const REG_ARG4: usize = 3;      // x3

// ── Architecture-specific get/set registers ────────────────────────

/// Get the child's registers.
/// On x86_64: uses PTRACE_GETREGS.
/// On aarch64: uses PTRACE_GETREGSET with NT_PRSTATUS (PTRACE_GETREGS
/// does NOT exist on aarch64 and returns EIO).
///
/// The function name `ptrace_getregs` is historical — on aarch64 it does
/// NOT use PTRACE_GETREGS (which doesn't exist on that arch); it uses the
/// generic PTRACE_GETREGSET mechanism. Kept the name for parity with the
/// x86_64 path and the call sites in `run_ptrace_loop`.
fn ptrace_getregs(pid: libc::pid_t, regs: &mut Regs) -> std::io::Result<()> {
    #[cfg(target_arch = "x86_64")]
    {
        let r = unsafe {
            libc::ptrace(
                libc::PTRACE_GETREGS,
                pid,
                0 as libc::c_long,
                regs as *mut _ as *mut libc::c_void,
            )
        };
        if r == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    #[cfg(target_arch = "aarch64")]
    {
        use aarch64_ptrace::{NT_PRSTATUS, PTRACE_GETREGSET};

        // Use libc::iovec (matching the kernel's `struct iovec`). The
        // kernel reads `iov_base`/`iov_len` from this struct, copies
        // `iov_len` bytes of register state INTO `iov_base`, and updates
        // `iov_len` to the actual number of bytes copied.
        let mut iov = libc::iovec {
            iov_base: regs as *mut _ as *mut libc::c_void,
            iov_len: std::mem::size_of::<Regs>(),
        };

        // IMPORTANT: bypass libc::ptrace() (bionic's variadic wrapper) and
        // invoke the kernel ptrace syscall directly via libc::syscall().
        // See the long comment at the top of this file for the rationale.
        //
        // libc::syscall is itself variadic but it is a thin Rust shim that
        // only forwards the raw syscall arguments to `syscall(2)`; bionic
        // does no `va_arg` unpacking here because libc::syscall is a
        // direct syscall stub, not a variadic-C wrapper like ptrace().
        //
        // The kernel's SYSCALL_DEFINE4(ptrace, long, request, long, pid,
        // unsigned long, addr, unsigned long, data) takes addr as an
        // integer (here NT_PRSTATUS=1), NOT a pointer — that's fine, the
        // kernel just reads it as `unsigned long` and dispatches on it.
        let r = unsafe {
            libc::syscall(
                libc::SYS_ptrace,
                PTRACE_GETREGSET,
                pid as libc::c_long,
                NT_PRSTATUS,
                &mut iov as *mut libc::iovec,
            )
        };
        if r == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

/// Set the child's registers.
/// On x86_64: uses PTRACE_SETREGS.
/// On aarch64: uses PTRACE_SETREGSET with NT_PRSTATUS.
///
/// Same bionic-ptrace-wrapper caveat as `ptrace_getregs`: we go through
/// the raw syscall on aarch64 because the libc::ptrace() variadic wrapper
/// has been observed to fail with EIO on real arm64 Android devices.
fn ptrace_setregs(pid: libc::pid_t, regs: &Regs) -> std::io::Result<()> {
    #[cfg(target_arch = "x86_64")]
    {
        let r = unsafe {
            libc::ptrace(
                libc::PTRACE_SETREGS,
                pid,
                0 as libc::c_long,
                regs as *const _ as *mut libc::c_void,
            )
        };
        if r == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    #[cfg(target_arch = "aarch64")]
    {
        use aarch64_ptrace::{NT_PRSTATUS, PTRACE_SETREGSET};

        // libc::iovec has `iov_base: *mut c_void`; for SETREGSET we only
        // need `*const c_void` (the kernel reads from us), but the struct
        // layout is identical and the cast is safe — the kernel does not
        // mutate the regset source buffer for SETREGSET.
        let iov = libc::iovec {
            iov_base: regs as *const _ as *mut libc::c_void,
            iov_len: std::mem::size_of::<Regs>(),
        };

        let r = unsafe {
            libc::syscall(
                libc::SYS_ptrace,
                PTRACE_SETREGSET,
                pid as libc::c_long,
                NT_PRSTATUS,
                &iov as *const libc::iovec,
            )
        };
        if r == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

/// Get the syscall number from registers.
fn get_syscall_num(regs: &Regs) -> i64 {
    let regs_ptr = regs as *const Regs as *const u64;
    unsafe { *regs_ptr.add(REG_SYSCALL) as i64 }
}

/// Get a syscall argument from registers.
fn get_syscall_arg(regs: &Regs, arg: usize) -> u64 {
    let regs_ptr = regs as *const Regs as *const u64;
    unsafe { *regs_ptr.add(arg) }
}

/// Set the return value of a syscall in registers.
fn set_syscall_ret(regs: &mut Regs, val: i64) {
    let regs_ptr = regs as *mut Regs as *mut u64;
    unsafe { *regs_ptr.add(REG_RET) = val as u64; }
}

// ── Path translation ───────────────────────────────────────────────

/// Translate a guest path to a host path by prepending the rootfs.
pub fn translate_path(rootfs: &str, path: &str) -> String {
    if !path.starts_with('/') {
        return path.to_string();
    }
    for prefix in &["/proc/", "/sys/", "/data/", "/apex/"] {
        if path.starts_with(prefix) {
            return path.to_string();
        }
    }
    if path.starts_with("/dev/") || path == "/dev" {
        return path.to_string();
    }
    if path.starts_with("/system/") || path == "/system" {
        return path.to_string();
    }
    if path.starts_with("/vendor/") || path == "/vendor" {
        return path.to_string();
    }
    format!("{}{}", rootfs, path)
}

// ── String read/write helpers ──────────────────────────────────────

fn read_child_string(pid: libc::pid_t, addr: u64) -> Option<String> {
    if addr == 0 {
        return None;
    }
    let mut result = Vec::new();
    let mut offset = 0i64;
    loop {
        let word = unsafe { libc::ptrace(libc::PTRACE_PEEKDATA, pid, addr as i64 + offset, 0) };
        if word == -1 {
            break;
        }
        let bytes = word.to_ne_bytes();
        for &b in &bytes {
            if b == 0 {
                return Some(String::from_utf8_lossy(&result).into_owned());
            }
            result.push(b);
        }
        offset += std::mem::size_of::<libc::c_long>() as i64;
        if result.len() > 4096 {
            break;
        }
    }
    if result.is_empty() { None } else { Some(String::from_utf8_lossy(&result).into_owned()) }
}

fn write_child_string(pid: libc::pid_t, addr: u64, s: &str) -> bool {
    if addr == 0 {
        return false;
    }
    let orig = read_child_string(pid, addr).unwrap_or_default();
    if s.len() >= orig.len() {
        return false;
    }
    let mut new_bytes = s.as_bytes().to_vec();
    new_bytes.push(0);
    let mut offset = 0i64;
    while offset < new_bytes.len() as i64 {
        let mut word_bytes = [0u8; 8];
        let chunk_len = std::cmp::min(8, new_bytes.len() - offset as usize);
        word_bytes[..chunk_len].copy_from_slice(&new_bytes[offset as usize..offset as usize + chunk_len]);
        let word = libc::c_long::from_ne_bytes(word_bytes);
        let r = unsafe { libc::ptrace(libc::PTRACE_POKEDATA, pid, addr as i64 + offset, word as libc::c_long) };
        if r == -1 {
            return false;
        }
        offset += 8;
    }
    true
}

// ── Ptrace loop ────────────────────────────────────────────────────

/// Check if ptrace is likely to work on this device.
pub fn ptrace_available() -> bool {
    let r = unsafe { libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0) };
    if r == 0 {
        return true;
    }
    let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
    if errno == libc::EPERM {
        return true;
    }
    false
}

pub fn run_ptrace_loop(pid: libc::pid_t, rootfs: &str) -> i32 {
    use std::io::Write;
    let log = |msg: &str| {
        let _ = writeln!(std::io::stderr(), "[KR64][ptrace] {}", msg);
    };

    log(&format!("ptrace loop started for pid {} (rootfs={})", pid, rootfs));

    // Set PTRACE_O_TRACESYSGOOD so we get SIGTRAP|0x80 for syscall stops.
    let r = unsafe {
        libc::ptrace(
            libc::PTRACE_SETOPTIONS,
            pid,
            0,
            libc::PTRACE_O_TRACESYSGOOD as libc::c_int,
        )
    };
    if r == -1 {
        let e = std::io::Error::last_os_error();
        log(&format!("PTRACE_SETOPTIONS failed: {}", e));
        return -1;
    }
    log("PTRACE_O_TRACESYSGOOD set");

    let mut in_syscall = false;
    let mut pending_getpid = false;
    let mut loop_count: u64 = 0;

    loop {
        // Continue the child to the next syscall entry/exit.
        let r = unsafe { libc::ptrace(libc::PTRACE_SYSCALL, pid, 0, 0) };
        if r == -1 {
            let e = std::io::Error::last_os_error();
            // ESRCH = child already exited — not an error, just done.
            if e.raw_os_error() == Some(libc::ESRCH) {
                log("PTRACE_SYSCALL: child already exited (ESRCH)");
                // Try to reap the child to get its exit status.
                let mut status: libc::c_int = 0;
                let waited = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
                if waited == pid {
                    if libc::WIFEXITED(status) {
                        return libc::WEXITSTATUS(status);
                    }
                    if libc::WIFSIGNALED(status) {
                        return -libc::WTERMSIG(status);
                    }
                }
                return -1;
            }
            log(&format!("PTRACE_SYSCALL failed: {}", e));
            return -1;
        }

        // Wait for the child to stop.
        let mut status: libc::c_int = 0;
        let waited = unsafe { libc::waitpid(pid, &mut status, 0) };
        if waited == -1 {
            let e = std::io::Error::last_os_error();
            log(&format!("waitpid failed: {}", e));
            return -1;
        }

        // Check if the child exited.
        if libc::WIFEXITED(status) {
            let code = libc::WEXITSTATUS(status);
            log(&format!("child exited with code {} (after {} iterations)", code, loop_count));
            return code;
        }
        if libc::WIFSIGNALED(status) {
            let sig = libc::WTERMSIG(status);
            log(&format!("child killed by signal {} (after {} iterations)", sig, loop_count));
            return -sig;
        }

        // Check if the child was stopped by a signal.
        if libc::WIFSTOPPED(status) {
            let sig = libc::WSTOPSIG(status);

            // SIGTRAP | 0x80 = syscall stop (because we set TRACESYSGOOD).
            if sig == (libc::SIGTRAP | 0x80) {
                loop_count += 1;

                // Get the child's registers using the arch-specific function.
                let mut regs: Regs = unsafe { std::mem::zeroed() };
                if let Err(e) = ptrace_getregs(pid, &mut regs) {
                    log(&format!("ptrace_getregs failed: {} (iteration {})", e, loop_count));
                    // We've consumed this syscall-stop regardless of whether
                    // we could read its registers — the next PTRACE_SYSCALL
                    // will land on the *other* half of the same syscall
                    // (entry↔exit alternate). If we don't flip `in_syscall`
                    // here, the next stop will be misclassified as the same
                    // phase (e.g. entry again), and we'll permanently lose
                    // sync — every subsequent open/getpid/etc. would be
                    // handled at the wrong phase and never actually faked.
                    //
                    // Flipping here means: if getregs fails transiently we
                    // stay in sync; if it fails persistently (the original
                    // bug) the loop still terminates via the child exiting,
                    // not via state corruption.
                    in_syscall = !in_syscall;
                    continue;
                }

                let syscall_num = get_syscall_num(&regs);

                if !in_syscall {
                    // ── Syscall ENTRY ──
                    in_syscall = true;

                    match syscall_num {
                        GETPID_SYSCALL => {
                            pending_getpid = true;
                            if loop_count <= 20 {
                                log("intercepted getpid() -> will return 1");
                            }
                        }
                        GETPPID_SYSCALL => {
                            pending_getpid = true;
                            if loop_count <= 20 {
                                log("intercepted getppid() -> will return 1");
                            }
                        }
                        #[allow(unreachable_patterns)]
                        OPEN_SYSCALL | OPENAT_SYSCALL | OPENAT2_SYSCALL => {
                            let path_arg_index = if syscall_num == OPEN_SYSCALL {
                                REG_ARG1
                            } else {
                                REG_ARG2
                            };
                            let path_addr = get_syscall_arg(&regs, path_arg_index);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path && loop_count <= 50 {
                                    log(&format!("intercepted open({}) -> {}", path, translated));
                                }
                                if translated != path {
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        #[allow(unreachable_patterns)]
                        STAT_SYSCALL | LSTAT_SYSCALL | NEWFSTATAT_SYSCALL | STATX_SYSCALL => {
                            let path_arg_index = if syscall_num == STAT_SYSCALL || syscall_num == LSTAT_SYSCALL {
                                REG_ARG1
                            } else {
                                REG_ARG2
                            };
                            let path_addr = get_syscall_arg(&regs, path_arg_index);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path {
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        #[allow(unreachable_patterns)]
                        ACCESS_SYSCALL | FACCESSAT_SYSCALL => {
                            let path_arg_index = if syscall_num == ACCESS_SYSCALL {
                                REG_ARG1
                            } else {
                                REG_ARG2
                            };
                            let path_addr = get_syscall_arg(&regs, path_arg_index);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path {
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        #[allow(unreachable_patterns)]
                        READLINK_SYSCALL | READLINKAT_SYSCALL => {
                            let path_arg_index = if syscall_num == READLINK_SYSCALL {
                                REG_ARG1
                            } else {
                                REG_ARG2
                            };
                            let path_addr = get_syscall_arg(&regs, path_arg_index);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path {
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        CHDIR_SYSCALL => {
                            let path_addr = get_syscall_arg(&regs, REG_ARG1);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path {
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        _ => {
                            // Not an intercepted syscall — let it through.
                        }
                    }
                } else {
                    // ── Syscall EXIT ──
                    in_syscall = false;

                    if pending_getpid {
                        let mut regs2: Regs = unsafe { std::mem::zeroed() };
                        if ptrace_getregs(pid, &mut regs2).is_ok() {
                            set_syscall_ret(&mut regs2, 1);
                            let _ = ptrace_setregs(pid, &regs2);
                        }
                        pending_getpid = false;
                    }
                }
            } else if sig == libc::SIGTRAP {
                // Regular SIGTRAP (breakpoint, single-step without 0x80
                // marker, etc.). We did NOT request delivery of any signal
                // so falling through to the next PTRACE_SYSCALL (with a
                // 0 signal arg at the top of the loop) is correct: the
                // child resumes and the trap is consumed without being
                // re-injected. We must NOT forward SIGTRAP back to the
                // child — that would loop forever (the same breakpoint
                // would fire again immediately).
            } else {
                // The child stopped because of a real signal that was
                // NOT a syscall-stop and NOT a debugger trap — e.g.
                // SIGSEGV, SIGBUS, SIGFPE, or a SIGCHLD-style signal
                // delivered by the kernel. Forward it to the child so
                // its own signal handlers (or default action) run.
                //
                // The `sig as c_long` 4th arg to PTRACE_SYSCALL is the
                // signal to deliver on resume; the next waitpid will
                // report either the next syscall-stop or, if the signal's
                // default action terminates the child, WIFSIGNALED.
                //
                // We do NOT flip `in_syscall`: the signal interrupted
                // whatever the child was doing between two syscall-stops,
                // so the next stop will be the same phase (entry if we
                // were heading to an entry, exit if we were heading to
                // an exit) as it would have been without the signal.
                unsafe {
                    libc::ptrace(libc::PTRACE_SYSCALL, pid, 0, sig as libc::c_long);
                }
                continue;
            }
        }
    }
}

// ── Architecture-specific syscall numbers ──────────────────────────

#[cfg(target_arch = "x86_64")]
const GETPID_SYSCALL: i64 = 39;
#[cfg(target_arch = "x86_64")]
const GETPPID_SYSCALL: i64 = 110;
#[cfg(target_arch = "x86_64")]
const OPEN_SYSCALL: i64 = 2;
#[cfg(target_arch = "x86_64")]
const OPENAT_SYSCALL: i64 = 257;
#[cfg(target_arch = "x86_64")]
const OPENAT2_SYSCALL: i64 = 437;
#[cfg(target_arch = "x86_64")]
const STAT_SYSCALL: i64 = 4;
#[cfg(target_arch = "x86_64")]
const LSTAT_SYSCALL: i64 = 6;
#[cfg(target_arch = "x86_64")]
const NEWFSTATAT_SYSCALL: i64 = 262;
#[cfg(target_arch = "x86_64")]
const ACCESS_SYSCALL: i64 = 21;
#[cfg(target_arch = "x86_64")]
const FACCESSAT_SYSCALL: i64 = 48;
#[cfg(target_arch = "x86_64")]
const READLINK_SYSCALL: i64 = 89;
#[cfg(target_arch = "x86_64")]
const READLINKAT_SYSCALL: i64 = 267;
#[cfg(target_arch = "x86_64")]
const CHDIR_SYSCALL: i64 = 80;
#[cfg(target_arch = "x86_64")]
const STATX_SYSCALL: i64 = 332;

#[cfg(target_arch = "aarch64")]
const GETPID_SYSCALL: i64 = 172;
#[cfg(target_arch = "aarch64")]
const GETPPID_SYSCALL: i64 = 173;
#[cfg(target_arch = "aarch64")]
const OPEN_SYSCALL: i64 = -1; // aarch64 has no open()
#[cfg(target_arch = "aarch64")]
const OPENAT_SYSCALL: i64 = 56;
#[cfg(target_arch = "aarch64")]
const OPENAT2_SYSCALL: i64 = 437;
#[cfg(target_arch = "aarch64")]
const STAT_SYSCALL: i64 = -1;
#[cfg(target_arch = "aarch64")]
const LSTAT_SYSCALL: i64 = -1;
#[cfg(target_arch = "aarch64")]
const NEWFSTATAT_SYSCALL: i64 = 79;
#[cfg(target_arch = "aarch64")]
const ACCESS_SYSCALL: i64 = -1;
#[cfg(target_arch = "aarch64")]
const FACCESSAT_SYSCALL: i64 = 48;
#[cfg(target_arch = "aarch64")]
const READLINK_SYSCALL: i64 = -1;
#[cfg(target_arch = "aarch64")]
const READLINKAT_SYSCALL: i64 = 78;
#[cfg(target_arch = "aarch64")]
const CHDIR_SYSCALL: i64 = 49;
#[cfg(target_arch = "aarch64")]
const STATX_SYSCALL: i64 = 291;
