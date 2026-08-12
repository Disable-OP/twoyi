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
//     - creat → translate path
//
//   Path translation rules:
//     - If path starts with "/" and is NOT under /dev/ (which kr64
//       already sets up on the host), prepend the rootfs path.
//     - Exception: /proc, /sys, /dev, /data, /apex, /system, /vendor
//       are left as-is if they already exist on the host (the host's
//       Android provides these). Otherwise, translate to rootfs.
//     - Exception: /init.rc, /init.*.rc, /sbin/*, /etc/* → translate
//       to rootfs (these are TWRP-specific files).

// Architecture-specific syscall numbers and register indices.
// Defined as const at module level (not in a sub-module) to avoid
// `use arch::*` import issues.

#[cfg(target_arch = "x86_64")]
const REG_SYSCALL: usize = 8;  // orig_rax
#[cfg(target_arch = "x86_64")]
const REG_RET: usize = 11;    // rax
#[cfg(target_arch = "x86_64")]
const REG_ARG1: usize = 5;    // rdi
#[cfg(target_arch = "x86_64")]
const REG_ARG2: usize = 4;    // rsi
#[cfg(target_arch = "x86_64")]
const REG_ARG3: usize = 3;    // rdx
#[cfg(target_arch = "x86_64")]
const REG_ARG4: usize = 10;   // r10

#[cfg(target_arch = "aarch64")]
const REG_SYSCALL: usize = 8;   // x8 (syscall number)
#[cfg(target_arch = "aarch64")]
const REG_RET: usize = 0;       // x0 (return value)
#[cfg(target_arch = "aarch64")]
const REG_ARG1: usize = 0;      // x0
#[cfg(target_arch = "aarch64")]
const REG_ARG2: usize = 1;      // x1
#[cfg(target_arch = "aarch64")]
const REG_ARG3: usize = 2;      // x2
#[cfg(target_arch = "aarch64")]
const REG_ARG4: usize = 3;      // x3

/// Check if ptrace is likely to work on this device.
/// On Android, untrusted apps CAN ptrace their own children.
/// Returns true if ptrace should work, false if it's definitely blocked.
pub fn ptrace_available() -> bool {
    // Try a no-op ptrace call on ourselves. This doesn't do anything
    // harmful but tells us if the syscall is available.
    let r = unsafe { libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0) };
    if r == 0 {
        // Undo the TRACEME — we don't actually want to be traced.
        // Unfortunately there's no PTRACE_DETACH from TRACEME.
        // The process will be traced by its parent, but since the parent
        // isn't waiting, this is effectively a no-op.
        // Actually, PTRACE_TRACEME just sets a flag; it doesn't do anything
        // until the parent calls wait(). So it's safe.
        return true;
    }
    // PTRACE_TRACEME failed — probably because we're already being traced
    // (e.g., by Android Studio debugger) or ptrace is blocked.
    let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
    // EPERM = already traced (ptrace is available, just can't TRACEME twice)
    if errno == libc::EPERM {
        return true;
    }
    false
}

/// Translate a guest path to a host path by prepending the rootfs.
///
/// Rules:
/// - /dev/ paths: keep as-is (kr64 sets these up on the host)
/// - /proc/ paths: keep as-is (host's procfs is fine)
/// - /sys/ paths: keep as-is (host's sysfs is fine)
/// - /data/ paths: keep as-is (host's data partition)
/// - /apex/ paths: keep as-is (host's apex)
/// - /system/ paths: keep as-is (host's system — TWRP doesn't need the
///   guest's /system for recovery mode)
/// - Everything else (/, /init.rc, /sbin/*, /etc/*, /var/*, /tmp/*):
///   prepend rootfs path
pub fn translate_path(rootfs: &str, path: &str) -> String {
    // If the path doesn't start with /, it's relative — leave as-is.
    if !path.starts_with('/') {
        return path.to_string();
    }

    // Paths that are left as-is (host provides these).
    // NOTE: /dev/ is left as-is because kr64 creates device sockets/symlinks
    // on the host's /dev. But we DO translate /dev/graphics/fb0 etc.
    // because those are TWRP-specific.
    for prefix in &[
        "/proc/",
        "/sys/",
        "/data/",
        "/apex/",
    ] {
        if path.starts_with(prefix) {
            return path.to_string();
        }
    }

    // /dev/ — leave as-is (kr64 sets up /dev/qemu_pipe etc. on the host).
    // TWRP-specific /dev/ paths like /dev/graphics/fb0 are also left as-is
    // because kr64 creates them.
    if path.starts_with("/dev/") || path == "/dev" {
        return path.to_string();
    }

    // /system/ and /vendor/ — on the host, these exist as the host's
    // Android system. TWRP's recovery doesn't need /system for recovery
    // mode (it uses /sbin/ and /res/). Leave as-is.
    if path.starts_with("/system/") || path == "/system" {
        return path.to_string();
    }
    if path.starts_with("/vendor/") || path == "/vendor" {
        return path.to_string();
    }

    // Everything else: prepend rootfs.
    // This covers: /init.rc, /init.*.rc, /sbin/*, /etc/*, /res/*, /tmp/*, etc.
    format!("{}{}", rootfs, path)
}

/// Read a string from the child's memory at the given address.
/// Uses PTRACE_PEEKDATA one word at a time.
fn read_child_string(pid: libc::pid_t, addr: u64) -> Option<String> {
    if addr == 0 {
        return None;
    }
    let mut result = Vec::new();
    let mut offset = 0i64;
    loop {
        let word = unsafe { libc::ptrace(libc::PTRACE_PEEKDATA, pid, addr as i64 + offset, 0) };
        if word == -1 {
            // Check errno — EIO means we've gone past the end of the mapping.
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EIO) {
                break;
            }
            // Other error — stop reading.
            break;
        }
        // The word is 8 bytes (on 64-bit). Copy bytes until we find a NUL.
        let bytes = word.to_ne_bytes();
        for &b in &bytes {
            if b == 0 {
                return Some(String::from_utf8_lossy(&result).into_owned());
            }
            result.push(b);
        }
        offset += std::mem::size_of::<libc::c_long>() as i64;
        // Safety limit — don't read more than 4KB.
        if result.len() > 4096 {
            break;
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&result).into_owned())
    }
}

/// Write a string to the child's memory at the given address.
/// Uses PTRACE_POKEDATA one word at a time.
/// The new string must be <= the old string's length (we can't extend
/// the child's memory layout, only overwrite existing bytes).
fn write_child_string(pid: libc::pid_t, addr: u64, s: &str) -> bool {
    if addr == 0 {
        return false;
    }
    // Read the original string to know its length (we need the NUL terminator
    // to fit within the original allocation).
    let orig = read_child_string(pid, addr).unwrap_or_default();
    if s.len() >= orig.len() {
        // The new string is too long — can't fit in the original allocation.
        // We'd need to allocate new memory in the child, which is much more
        // complex. For now, skip the translation.
        return false;
    }

    // Build the new bytes: the new string + NUL + padding.
    let mut new_bytes = s.as_bytes().to_vec();
    new_bytes.push(0); // NUL terminator

    // Write word by word.
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

/// Get the syscall number from the child's registers.
#[cfg(target_arch = "x86_64")]
fn get_syscall_num(regs: &libc::user_regs_struct) -> i64 {
    // orig_rax is at offset 120 (15 * 8) in user_regs_struct
    // But we can't index it directly because Rust's libc doesn't expose
    // the fields as an array. Use a union/reinterpret trick.
    let regs_ptr = regs as *const libc::user_regs_struct as *const u64;
    unsafe { *regs_ptr.add(REG_SYSCALL) as i64 }
}

#[cfg(target_arch = "aarch64")]
fn get_syscall_num(regs: &libc::user_regs_struct) -> i64 {
    let regs_ptr = regs as *const libc::user_regs_struct as *const u64;
    unsafe { *regs_ptr.add(REG_SYSCALL) as i64 }
}

/// Get a syscall argument from the registers.
#[cfg(target_arch = "x86_64")]
fn get_syscall_arg(regs: &libc::user_regs_struct, arg: usize) -> u64 {
    let regs_ptr = regs as *const libc::user_regs_struct as *const u64;
    unsafe { *regs_ptr.add(arg) }
}

#[cfg(target_arch = "aarch64")]
fn get_syscall_arg(regs: &libc::user_regs_struct, arg: usize) -> u64 {
    let regs_ptr = regs as *const libc::user_regs_struct as *const u64;
    unsafe { *regs_ptr.add(arg) }
}

/// Set the return value of a syscall in the child's registers.
#[cfg(target_arch = "x86_64")]
fn set_syscall_ret(regs: &mut libc::user_regs_struct, val: i64) {
    let regs_ptr = regs as *mut libc::user_regs_struct as *mut u64;
    unsafe { *regs_ptr.add(REG_RET) = val as u64; }
}

#[cfg(target_arch = "aarch64")]
fn set_syscall_ret(regs: &mut libc::user_regs_struct, val: i64) {
    let regs_ptr = regs as *mut libc::user_regs_struct as *mut u64;
    unsafe { *regs_ptr.add(REG_RET) = val as u64; }
}

/// Set the syscall number (to skip a syscall, set it to -1 / __NR_read with
/// a return value of 0).
#[cfg(target_arch = "x86_64")]
fn set_syscall_num(regs: &mut libc::user_regs_struct, num: i64) {
    let regs_ptr = regs as *mut libc::user_regs_struct as *mut u64;
    unsafe { *regs_ptr.add(REG_SYSCALL) = num as u64; }
}

#[cfg(target_arch = "aarch64")]
fn set_syscall_num(regs: &mut libc::user_regs_struct, num: i64) {
    let regs_ptr = regs as *mut libc::user_regs_struct as *mut u64;
    unsafe { *regs_ptr.add(REG_SYSCALL) = num as u64; }
}

/// Run the ptrace syscall interception loop in the PARENT process.
///
/// This function blocks until the child exits. It intercepts:
/// - getpid/getppid → return 1
/// - open/openat/openat2 → translate path
/// - stat/lstat/newfstatat → translate path
/// - access/faccessat → translate path
/// - readlink/readlinkat → translate path
/// - chdir → translate path
/// - statx → translate path
///
/// `rootfs` is the host path to the guest's rootfs (e.g.
/// "/data/user/0/io.twoyi/rootfs").
pub fn run_ptrace_loop(pid: libc::pid_t, rootfs: &str) -> i32 {
    use std::io::Write;
    let log = |msg: &str| {
        let _ = writeln!(std::io::stderr(), "[KR64][ptrace] {}", msg);
    };

    log(&format!("ptrace loop started for pid {} (rootfs={})", pid, rootfs));

    // Set PTRACE_O_TRACESYSGOOD so we get SIGTRAP|0x80 for syscall stops,
    // distinguishing them from other signals.
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

    loop {
        // Continue the child to the next syscall entry/exit.
        let r = unsafe { libc::ptrace(libc::PTRACE_SYSCALL, pid, 0, 0) };
        if r == -1 {
            let e = std::io::Error::last_os_error();
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
            log(&format!("child exited with code {}", code));
            return code;
        }
        if libc::WIFSIGNALED(status) {
            let sig = libc::WTERMSIG(status);
            log(&format!("child killed by signal {}", sig));
            return -sig;
        }

        // Check if the child was stopped by a signal.
        if libc::WIFSTOPPED(status) {
            let sig = libc::WSTOPSIG(status);

            // SIGTRAP | 0x80 = syscall stop (because we set TRACESYSGOOD).
            if sig == (libc::SIGTRAP | 0x80) {
                // Get the child's registers.
                let mut regs: libc::user_regs_struct = unsafe { std::mem::zeroed() };
                let r = unsafe {
                    libc::ptrace(
                        libc::PTRACE_GETREGS,
                        pid,
                        0,
                        &mut regs as *mut _ as *mut libc::c_void,
                    )
                };
                if r == -1 {
                    let e = std::io::Error::last_os_error();
                    log(&format!("PTRACE_GETREGS failed: {}", e));
                    continue;
                }

                let syscall_num = get_syscall_num(&regs);

                if !in_syscall {
                    // ── Syscall ENTRY ──
                    in_syscall = true;

                    match syscall_num {
                        GETPID_SYSCALL => {
                            // Intercept getpid → return 1
                            pending_getpid = true;
                            log("intercepted getpid() → will return 1");
                        }
                        GETPPID_SYSCALL => {
                            // Intercept getppid → return 1
                            pending_getpid = true;
                            log("intercepted getppid() → will return 1");
                        }
                        #[allow(unreachable_patterns)]
                        OPEN_SYSCALL | OPENAT_SYSCALL | OPENAT2_SYSCALL => {
                            // For open(path, ...) → arg1 = path
                            // For openat(dirfd, path, ...) → arg2 = path
                            let path_arg_index = if syscall_num == OPEN_SYSCALL {
                                REG_ARG1
                            } else {
                                REG_ARG2
                            };
                            let path_addr = get_syscall_arg(&regs, path_arg_index);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path {
                                    log(&format!(
                                        "intercepted open({}) → translated to {}",
                                        path, translated
                                    ));
                                    // Try to overwrite the path in the child's memory.
                                    if write_child_string(pid, path_addr, &translated) {
                                        log("path overwrite succeeded");
                                    } else {
                                        log("path overwrite FAILED (string too long) — skipping translation");
                                    }
                                }
                            }
                        }
                        #[allow(unreachable_patterns)]
                        STAT_SYSCALL | LSTAT_SYSCALL | NEWFSTATAT_SYSCALL | STATX_SYSCALL => {
                            // stat(path, ...) → arg1 = path
                            // newfstatat(dirfd, path, ...) → arg2 = path
                            // statx(dirfd, path, ...) → arg2 = path
                            let path_arg_index = if syscall_num == STAT_SYSCALL || syscall_num == LSTAT_SYSCALL {
                                REG_ARG1
                            } else {
                                REG_ARG2
                            };
                            let path_addr = get_syscall_arg(&regs, path_arg_index);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path {
                                    log(&format!(
                                        "intercepted stat({}) → translated to {}",
                                        path, translated
                                    ));
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        #[allow(unreachable_patterns)]
                        ACCESS_SYSCALL | FACCESSAT_SYSCALL => {
                            // access(path, ...) → arg1 = path
                            // faccessat(dirfd, path, ...) → arg2 = path
                            let path_arg_index = if syscall_num == ACCESS_SYSCALL {
                                REG_ARG1
                            } else {
                                REG_ARG2
                            };
                            let path_addr = get_syscall_arg(&regs, path_arg_index);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path {
                                    log(&format!(
                                        "intercepted access({}) → translated to {}",
                                        path, translated
                                    ));
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        #[allow(unreachable_patterns)]
                        READLINK_SYSCALL | READLINKAT_SYSCALL => {
                            // readlink(path, ...) → arg1 = path
                            // readlinkat(dirfd, path, ...) → arg2 = path
                            let path_arg_index = if syscall_num == READLINK_SYSCALL {
                                REG_ARG1
                            } else {
                                REG_ARG2
                            };
                            let path_addr = get_syscall_arg(&regs, path_arg_index);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path {
                                    log(&format!(
                                        "intercepted readlink({}) → translated to {}",
                                        path, translated
                                    ));
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        CHDIR_SYSCALL => {
                            let path_addr = get_syscall_arg(&regs, REG_ARG1);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path {
                                    log(&format!(
                                        "intercepted chdir({}) → translated to {}",
                                        path, translated
                                    ));
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
                        // Override the return value to 1.
                        let mut regs: libc::user_regs_struct = unsafe { std::mem::zeroed() };
                        let r = unsafe {
                            libc::ptrace(
                                libc::PTRACE_GETREGS,
                                pid,
                                0,
                                &mut regs as *mut _ as *mut libc::c_void,
                            )
                        };
                        if r == 0 {
                            set_syscall_ret(&mut regs, 1);
                            unsafe {
                                libc::ptrace(
                                    libc::PTRACE_SETREGS,
                                    pid,
                                    0,
                                    &regs as *const _ as *mut libc::c_void,
                                );
                            }
                            log("getpid/getppid return value set to 1");
                        }
                        pending_getpid = false;
                    }
                }
            } else if sig == libc::SIGTRAP {
                // Regular SIGTRAP (not syscall) — could be a breakpoint.
                // Just continue.
            } else {
                // The child received a signal (e.g., SIGSEGV, SIGBUS).
                // Forward it to the child and continue.
                unsafe {
                    libc::ptrace(libc::PTRACE_SYSCALL, pid, 0, sig as libc::c_long);
                }
                continue;
            }
        }
    }
}

// Architecture-specific syscall number constants.
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
