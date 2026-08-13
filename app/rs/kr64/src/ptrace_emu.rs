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
//   32-bit (i386) child support:
//   - TWRP's init binary ships as a 32-bit static ELF, so on an x86_64
//     host the traced child runs in compat mode. The kernel exposes
//     its register state via a 68-byte user_regs_struct32 (returned
//     by PTRACE_GETREGSET with NT_PRSTATUS) instead of the 216-byte
//     user_regs_struct used by 64-bit children, and the child uses
//     the i386 syscall-number table (e.g. getpid is 20, not 39).
//   - We detect the child's bitness at the first syscall stop by
//     inspecting the `iov_len` returned by PTRACE_GETREGSET, then
//     pick the matching `ChildAbi` (ABI_X86_32 vs ABI_X86_64). All
//     syscall-number comparisons and register-index lookups go
//     through that ABI so the same loop body handles both cases.
//   - On aarch64 there is no 32-bit userspace, so we always use
//     ABI_AARCH64.
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
// iovec. On x86_64, we NOW ALSO use PTRACE_GETREGSET (we used to use
// PTRACE_GETREGS, but that does not expose `iov_len`, which we need
// in order to detect whether the child is a 32-bit i386 binary or a
// 64-bit x86_64 binary — see the "32-bit child support" note above).
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
// interface for the GETREGSET/SETREGSET requests on aarch64. We now do
// the same on x86_64 (the raw-syscall path costs nothing there and keeps
// the GET/SET code paths identical across architectures). We keep
// `libc::ptrace()` for the non-REGSET requests (PTRACE_SETOPTIONS,
// PTRACE_SYSCALL, PTRACE_PEEKDATA, ...) because those have been observed
// to work fine through bionic and the workaround adds nothing there.

// ── Register types ─────────────────────────────────────────────────

/// On x86_64, user_regs_struct has 27 fields (216 bytes). We access them
/// as u64 array elements via index constants stored in `ChildAbi`.
///
/// On aarch64, we use user_pt_regs which is u64[31] + sp + pc + pstate
/// (272 bytes). We access x0-x30 as array[0..30], and the syscall
/// number is in x8.
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

// Named constants for the generic PTRACE_*REGSET interface, now used on
// BOTH x86_64 and aarch64. (Previously x86_64 used PTRACE_GETREGS, but
// that does not expose `iov_len` so we cannot detect child bitness with
// it — and on x86_64 we need that detection to support i386 compat-mode
// children like TWRP's static init binary.) Using names instead of bare
// `33`/`34`/`1` makes the intent obvious and stops future readers from
// wondering whether they are syscall numbers, NT_* types, or something
// else entirely.
mod ptrace_regset {
    /// `PTRACE_GETREGSET` — Linux generic ptrace request number, see
    /// <linux/ptrace.h>. Reads a regset by NT_* type into a user `iovec`.
    pub const PTRACE_GETREGSET: libc::c_long = 33;
    /// `PTRACE_SETREGSET` — Linux generic ptrace request number, see
    /// <linux/ptrace.h>. Writes a regset by NT_* type from a user `iovec`.
    pub const PTRACE_SETREGSET: libc::c_long = 34;
    /// `NT_PRSTATUS` — general-purpose registers regset, see
    /// <linux/elf.h>. This is the regset that maps to `user_pt_regs`
    /// on aarch64, to `user_regs_struct` on x86_64, and to
    /// `user_regs_struct32` on i386 compat-mode children.
    pub const NT_PRSTATUS: libc::c_long = 1;
}

// ── Child ABI: runtime syscall numbers + register layout ───────────
//
// On x86_64 the traced child may be either a 64-bit ELF or a 32-bit
// (i386) ELF running in compat mode. The two have DIFFERENT syscall
// number tables (e.g. getpid is 39 on x86_64 but 20 on i386) and
// DIFFERENT syscall register conventions (x86_64: rdi/rsi/rdx/r10;
// i386: ebx/ecx/edx/esi). We therefore cannot use compile-time
// constants for the syscall numbers or the argument register indices
// — we have to pick the right set at runtime based on the child's
// actual bitness.
//
// We detect bitness at the first syscall stop by inspecting the
// `iov_len` returned by PTRACE_GETREGSET(NT_PRSTATUS):
//   - 68 bytes  → 32-bit child (user_regs_struct32, 17 * 4)
//   - 216 bytes → 64-bit child (user_regs_struct, 27 * 8)
// and then select ABI_X86_32 or ABI_X86_64 accordingly.
//
// On aarch64 there is no 32-bit userspace, so we always use ABI_AARCH64.

/// Runtime-detected syscall numbers and register layout for the traced
/// child. All fields are valid for the child's actual bitness — callers
/// do not need to know whether the child is 32-bit or 64-bit.
#[derive(Clone, Copy)]
struct ChildAbi {
    // Syscall numbers (the values the child puts in the syscall-number
    // register to request each syscall). -1 means "not present on this
    // architecture" (e.g. plain `open` does not exist on aarch64, which
    // only has `openat`).
    getpid: i64,
    getppid: i64,
    open: i64,
    openat: i64,
    openat2: i64,
    stat: i64,
    lstat: i64,
    newfstatat: i64,
    statx: i64,
    access: i64,
    faccessat: i64,
    rt_sigprocmask: i64,
    readlink: i64,
    readlinkat: i64,
    chdir: i64,
    // Syscalls we never actually emulate, but whose numbers we need for
    // the SIGSYS diagnostic logging (we look up the original syscall
    // number to print a human-readable name when seccomp blocks it).
    mount: i64,
    chroot: i64,
    mkdir: i64,
    chmod: i64,
    unshare: i64,
    // Register indices into the `Regs` buffer reinterpreted as a u64
    // array. On x86_64 these index into user_regs_struct; on aarch64
    // into user_pt_regs. On x86_64 running a 32-bit child, PTRACE_GETREGS
    // zero-extends the 32-bit register values into the corresponding
    // 64-bit slots, so we use the 64-bit struct indices that match the
    // i386 syscall-register convention (rbx/rcx/rdx/rsi, NOT rdi/rsi/
    // rdx/r10).
    reg_syscall: usize,
    reg_ret: usize,
    reg_arg1: usize,
    reg_arg2: usize,
    #[allow(dead_code)]
    reg_arg3: usize,
    #[allow(dead_code)]
    reg_arg4: usize,
}

// x86_64 user_regs_struct field order (as u64 array indices):
//   0:r15 1:r14 2:r13 3:r12 4:rbp 5:rbx 6:r11 7:r10 8:r9 9:r8
//   10:rax 11:rcx 12:rdx 13:rsi 14:rdi 15:orig_rax 16:rip 17:cs
//   18:eflags 19:rsp 20:ss 21:fs_base 22:gs_base 23:ds 24:es 25:fs 26:gs
//
// i386 syscall register convention: syscall number in orig_eax, return
// value in eax, args in ebx/ecx/edx/esi/edi/ebp. When the kernel
// reports a 32-bit child's registers to a 64-bit tracer via the
// 64-bit user_regs_struct view (PTRACE_GETREGS zero-extends), the
// 32-bit values appear in the corresponding 64-bit slots (rbx ← ebx,
// rcx ← ecx, rdx ← edx, rsi ← esi, rax ← eax, orig_rax ← orig_eax),
// so we use those slots' indices for the 32-bit ABI too.

#[cfg(target_arch = "x86_64")]
const ABI_X86_64: ChildAbi = ChildAbi {
    getpid: 39,
    getppid: 110,
    open: 2,
    openat: 257,
    openat2: 437,
    stat: 4,
    lstat: 6,
    newfstatat: 262,
    statx: 332,
    access: 21,
    faccessat: 48,
    rt_sigprocmask: 14,
    readlink: 89,
    readlinkat: 267,
    chdir: 80,
    mount: 165,
    chroot: 161,
    mkdir: 83,
    chmod: 90,
    unshare: 272,
    reg_syscall: 15, // orig_rax
    reg_ret: 10,     // rax
    reg_arg1: 14,    // rdi
    reg_arg2: 13,    // rsi
    reg_arg3: 12,    // rdx
    reg_arg4: 7,     // r10
};

#[cfg(target_arch = "x86_64")]
const ABI_X86_32: ChildAbi = ChildAbi {
    // i386 syscall numbers — see asm/unistd_32.h.
    getpid: 20,
    getppid: 64,
    open: 5,
    openat: 295,
    openat2: 437,
    stat: 106,
    lstat: 107,
    newfstatat: 300,
    statx: 383,
    access: 33,
    faccessat: 307,
    rt_sigprocmask: 14,
    readlink: 85,
    readlinkat: 303,
    chdir: 12,
    mount: 21,
    chroot: 61,
    mkdir: 39,
    chmod: 15,
    unshare: 310,
    // i386 syscall args are passed in ebx/ecx/edx/esi (not rdi/rsi/
    // rdx/r10). When reading these from a 32-bit child via the 64-bit
    // user_regs_struct view (PTRACE_GETREGS zero-extends), the values
    // land in the corresponding 64-bit slots: rbx ← ebx, rcx ← ecx,
    // rdx ← edx, rsi ← esi. orig_rax ← orig_eax and rax ← eax are the
    // same slots for both bitnesses.
    reg_syscall: 15, // orig_rax (same as 64-bit)
    reg_ret: 10,     // rax (same)
    reg_arg1: 5,     // rbx (NOT rdi which is 14)
    reg_arg2: 11,    // rcx (NOT rsi which is 13)
    reg_arg3: 12,    // rdx (same)
    reg_arg4: 13,    // rsi (NOT r10 which is 7)
};

#[cfg(target_arch = "aarch64")]
const ABI_AARCH64: ChildAbi = ChildAbi {
    getpid: 172,
    getppid: 173,
    open: -1, // aarch64 has no open(); only openat()
    openat: 56,
    openat2: 437,
    stat: -1,
    lstat: -1,
    newfstatat: 79,
    statx: 291,
    access: -1,
    faccessat: 48,
    rt_sigprocmask: 135,
    readlink: -1,
    readlinkat: 78,
    chdir: 49,
    mount: 165,
    chroot: 51,
    mkdir: 34,
    chmod: 53,
    unshare: 97,
    reg_syscall: 8, // x8 (syscall number)
    reg_ret: 0,     // x0 (return value)
    reg_arg1: 0,    // x0
    reg_arg2: 1,    // x1
    reg_arg3: 2,    // x2
    reg_arg4: 3,    // x3
};

/// Expected iov_len for a 32-bit (i386 compat) child's NT_PRSTATUS
/// regset — `user_regs_struct32` is 17 * 4 = 68 bytes. Used by
/// `pick_abi_from_iov_len` (on x86_64) and by the bitness-detection
/// log messages to label the child as 32-bit vs 64-bit.
///
/// On aarch64 this value is never meaningful (aarch64 has no 32-bit
/// userspace), but we still define it so the `cfg!()`-gated logging
/// code in `run_ptrace_loop` compiles uniformly across architectures —
/// the `cfg!(target_arch = "x86_64")` branch that compares against it
/// is dead on aarch64.
const USER_REGS32_LEN: usize = 68;

/// Pick the right `ChildAbi` based on the `iov_len` returned by
/// PTRACE_GETREGSET. On x86_64 this distinguishes 32-bit (68 bytes)
/// from 64-bit (216 bytes) children. On aarch64 there is no choice to
/// make — the child is always 64-bit.
fn pick_abi_from_iov_len(iov_len: usize) -> ChildAbi {
    #[cfg(target_arch = "x86_64")]
    {
        if iov_len == USER_REGS32_LEN {
            ABI_X86_32
        } else {
            // 216 (sizeof user_regs_struct) or any other size → default
            // to 64-bit. Defaulting is safer than panicking: if the
            // kernel ever reports an unexpected size we still try the
            // 64-bit ABI, which is the historical behaviour.
            ABI_X86_64
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        let _ = iov_len;
        ABI_AARCH64
    }
}

// ── Architecture-specific get/set registers ────────────────────────

/// Get the child's registers via PTRACE_GETREGSET with NT_PRSTATUS.
///
/// This used to call PTRACE_GETREGS on x86_64, but we switched to
/// PTRACE_GETREGSET on BOTH architectures so that we can inspect the
/// `iov_len` the kernel writes back — that length tells us whether the
/// child is a 32-bit (i386) or 64-bit ELF, which we need to pick the
/// correct syscall-number table and register-index mapping.
///
/// Returns the actual `iov_len` reported by the kernel on success.
/// Callers use this at the first syscall stop to lazily initialize
/// the per-child `ChildAbi`.
///
/// The function name `ptrace_getregs` is historical — neither arch now
/// uses PTRACE_GETREGS; both go through the generic PTRACE_GETREGSET
/// mechanism. Kept the name for parity with the call sites in
/// `run_ptrace_loop`.
fn ptrace_getregs(pid: libc::pid_t, regs: &mut Regs) -> std::io::Result<usize> {
    use ptrace_regset::{NT_PRSTATUS, PTRACE_GETREGSET};

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
    //
    // We use the raw-syscall path on BOTH architectures now: the
    // historical bionic variadic-ptrace EIO problem was observed on
    // aarch64, but going through libc::syscall uniformly costs nothing
    // on x86_64 and means we don't need a cfg-split here.
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
    Ok(iov.iov_len)
}

/// Set the child's registers via PTRACE_SETREGSET with NT_PRSTATUS.
///
/// Same switch as `ptrace_getregs`: we used to call PTRACE_SETREGS on
/// x86_64, but now use PTRACE_SETREGSET on both architectures so the
/// GET and SET paths are symmetric (and so we write back the same
/// regset size the kernel gave us at GETREGSET time, which matters for
/// 32-bit children — writing 216 bytes to a 32-bit child would confuse
/// the kernel's 32-bit regset handler).
///
/// `iov_len` is the regset size captured at the matching GETREGSET
/// call. Passing it through (instead of `sizeof(Regs)`) makes the
/// 32-bit and 64-bit paths symmetric.
///
/// Same bionic-ptrace-wrapper caveat as `ptrace_getregs`: we go through
/// the raw syscall on BOTH architectures because the libc::ptrace()
/// variadic wrapper has been observed to fail with EIO on real arm64
/// Android devices, and going through libc::syscall uniformly costs
/// nothing on x86_64.
fn ptrace_setregs(pid: libc::pid_t, regs: &Regs, iov_len: usize) -> std::io::Result<()> {
    use ptrace_regset::{NT_PRSTATUS, PTRACE_SETREGSET};

    // libc::iovec has `iov_base: *mut c_void`; for SETREGSET we only
    // need `*const c_void` (the kernel reads from us), but the struct
    // layout is identical and the cast is safe — the kernel does not
    // mutate the regset source buffer for SETREGSET.
    let iov = libc::iovec {
        iov_base: regs as *const _ as *mut libc::c_void,
        iov_len,
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

/// Get the syscall number from registers.
fn get_syscall_num(regs: &Regs, abi: &ChildAbi) -> i64 {
    let regs_ptr = regs as *const Regs as *const u64;
    unsafe { *regs_ptr.add(abi.reg_syscall) as i64 }
}

/// Get a syscall argument from registers.
fn get_syscall_arg(regs: &Regs, arg: usize) -> u64 {
    let regs_ptr = regs as *const Regs as *const u64;
    unsafe { *regs_ptr.add(arg) }
}

/// Set the return value of a syscall in registers.
///
/// On x86_64 this writes `rax` (the kernel's "syscall return value"
/// slot). On aarch64 this writes `x0`. On a 32-bit x86 child the same
/// 64-bit slot is used because the kernel zero-extends the 32-bit
/// `eax` into `rax` (and likewise on writeback the kernel takes the
/// low 32 bits of `rax` and stores them in `eax`).
fn set_syscall_ret(regs: &mut Regs, abi: &ChildAbi, val: i64) {
    let regs_ptr = regs as *mut Regs as *mut u64;
    unsafe {
        *regs_ptr.add(abi.reg_ret) = val as u64;
    }
}

/// Set the syscall number in registers.
///
/// On x86_64 this writes `orig_rax` (the kernel's "what syscall was
/// requested" slot, distinct from `rax` which holds the return value).
/// On aarch64 this writes `x8` (the syscall-number register). On a
/// 32-bit x86 child the same 64-bit `orig_rax` slot is used because
/// PTRACE_GETREGS zero-extends `orig_eax` into it.
///
/// Used by the SIGSYS handler to rewrite a seccomp-blocked syscall
/// into a harmless one (getpid) before resuming, so the kernel does
/// not re-evaluate the original (blocked) syscall number and re-raise
/// SIGSYS. The `getpid` number is taken from `abi` so it is correct
/// for both 32-bit (20) and 64-bit (39) children.
fn set_syscall_num(regs: &mut Regs, abi: &ChildAbi, val: i64) {
    let regs_ptr = regs as *mut Regs as *mut u64;
    unsafe {
        *regs_ptr.add(abi.reg_syscall) = val as u64;
    }
}

/// Map a raw syscall number to a human-readable name for log messages.
///
/// Takes the per-child `ChildAbi` so the match works regardless of
/// whether the child is 32-bit or 64-bit (the numbers differ — e.g.
/// `access` is 21 on x86_64 but 33 on i386, and `mount` is 165 on
/// x86_64 but 21 on i386, so a single static table would be
/// ambiguous).
fn syscall_name(nr: i64, abi: &ChildAbi) -> &'static str {
    if nr == abi.access {
        "access"
    } else if nr == abi.rt_sigprocmask {
        "rt_sigprocmask"
    } else if nr == abi.mount {
        "mount"
    } else if nr == abi.chroot {
        "chroot"
    } else if nr == abi.mkdir {
        "mkdir"
    } else if nr == abi.chmod {
        "chmod"
    } else if nr == abi.unshare {
        "unshare"
    } else {
        "unknown"
    }
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
    if result.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&result).into_owned())
    }
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
        word_bytes[..chunk_len]
            .copy_from_slice(&new_bytes[offset as usize..offset as usize + chunk_len]);
        let word = libc::c_long::from_ne_bytes(word_bytes);
        let r = unsafe {
            libc::ptrace(
                libc::PTRACE_POKEDATA,
                pid,
                addr as i64 + offset,
                word as libc::c_long,
            )
        };
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

    log(&format!(
        "ptrace loop started for pid {} (rootfs={})",
        pid, rootfs
    ));

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
    // Runtime-detected syscall/register layout for the child. `None`
    // until the first successful ptrace_getregs — at that point we
    // inspect the returned `iov_len` to pick ABI_X86_32 vs ABI_X86_64
    // (on x86_64) or unconditionally use ABI_AARCH64 (on aarch64).
    // Until then we have no registers to look at, so there is nothing
    // to dispatch on.
    let mut abi: Option<ChildAbi> = None;
    // Rolling log of the last N SIGSYS-intercepted syscall numbers.
    // Used on child exit to print "the last few syscalls seccomp
    // blocked" — this is the single most useful diagnostic when init
    // dies with a non-zero exit code, because it shows the syscall
    // that the rewrite-to-getpid strategy is masking. Cap is small
    // (32) to keep memory bounded if init triggers thousands of
    // SIGSYS in a tight loop.
    const RECENT_SIGSYS_CAP: usize = 32;
    // Rolling history of the last N SIGSYS-intercepted syscalls, stored
    // as human-readable descriptions (e.g. `access("/init.rc") nr=21`
    // or `nr=14 [rt_sigprocmask]`). Storing strings instead of bare
    // syscall numbers lets us include the path argument for access()
    // calls — the single most useful diagnostic when init dies after
    // a flurry of access() probes (it shows which file init was
    // looking for).
    let mut recent_sigsys: std::collections::VecDeque<String> =
        std::collections::VecDeque::with_capacity(RECENT_SIGSYS_CAP);
    // Signal to deliver to the child on the next PTRACE_SYSCALL resume.
    // 0 means "don't deliver any signal". Non-zero values are set by
    // the signal-forwarding branch below so that the SINGLE
    // PTRACE_SYSCALL at the loop top can inject the signal —
    // having two PTRACE_SYSCALL calls (one in the handler, one at the
    // loop top) caused the second to return ESRCH because the child
    // was already running, which then made us return -1 prematurely.
    let mut resume_signal: libc::c_int = 0;

    loop {
        // Continue the child to the next syscall entry/exit. This is
        // the ONLY PTRACE_SYSCALL in the loop — handlers below set
        // `resume_signal` (and `continue`) instead of resuming the
        // child themselves, so we never race the second ptrace call.
        let r =
            unsafe { libc::ptrace(libc::PTRACE_SYSCALL, pid, 0, resume_signal as libc::c_long) };
        // Reset for the next iteration — only set again if a
        // signal-delivery branch below populates it.
        resume_signal = 0;
        if r == -1 {
            let e = std::io::Error::last_os_error();
            // ESRCH = child already exited — not an error, just done.
            if e.raw_os_error() == Some(libc::ESRCH) {
                log("PTRACE_SYSCALL: child already exited (ESRCH)");
                // Print the rolling SIGSYS history before reaping — the
                // main WIFEXITED/WIFSIGNALED branches below won't run
                // on this path, so without this log we'd lose the
                // "last N intercepted syscalls" diagnostic when the
                // child dies between a syscall-exit-stop and our
                // next PTRACE_SYSCALL.
                if !recent_sigsys.is_empty() {
                    let collected: Vec<String> = recent_sigsys.iter().cloned().collect();
                    log(&format!(
                        "last {} SIGSYS-intercepted syscalls before ESRCH (oldest->newest): {:?}",
                        collected.len(),
                        collected
                    ));
                }
                // Try to reap the child to get its exit status.
                let mut status: libc::c_int = 0;
                let waited = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
                if waited == pid {
                    if libc::WIFEXITED(status) {
                        let code = libc::WEXITSTATUS(status);
                        log(&format!("ESRCH path: child exit code {}", code));
                        return code;
                    }
                    if libc::WIFSIGNALED(status) {
                        let sig = libc::WTERMSIG(status);
                        log(&format!("ESRCH path: child killed by signal {}", sig));
                        return -sig;
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
            log(&format!(
                "child exited with code {} (after {} iterations)",
                code, loop_count
            ));
            // Print the last few SIGSYS-intercepted syscalls so we can
            // identify what init was doing right before it died. This is
            // critical for diagnosing the "init exits with code 1 at
            // iteration 177" issue: the last few SIGSYS numbers tell us
            // which seccomp-blocked syscall (mount? chroot? unshare?)
            // init was retrying right before it gave up and exited.
            if recent_sigsys.is_empty() {
                log("no SIGSYS interceptions recorded during this run");
            } else {
                let collected: Vec<String> = recent_sigsys.iter().cloned().collect();
                log(&format!(
                    "last {} SIGSYS-intercepted syscalls (oldest->newest): {:?}",
                    collected.len(),
                    collected
                ));
            }
            return code;
        }
        if libc::WIFSIGNALED(status) {
            let sig = libc::WTERMSIG(status);
            log(&format!(
                "child killed by signal {} (after {} iterations)",
                sig, loop_count
            ));
            if !recent_sigsys.is_empty() {
                let collected: Vec<String> = recent_sigsys.iter().cloned().collect();
                log(&format!(
                    "last {} SIGSYS-intercepted syscalls before kill (oldest->newest): {:?}",
                    collected.len(),
                    collected
                ));
            }
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
                let iov_len = match ptrace_getregs(pid, &mut regs) {
                    Ok(len) => len,
                    Err(e) => {
                        log(&format!(
                            "ptrace_getregs failed: {} (iteration {})",
                            e, loop_count
                        ));
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
                };

                // Lazily initialize the per-child ABI on the first
                // successful register read. The iov_len tells us
                // whether the child is 32-bit (68) or 64-bit (216) on
                // x86_64; on aarch64 the choice is always ABI_AARCH64.
                if abi.is_none() {
                    let picked = pick_abi_from_iov_len(iov_len);
                    let bitness_label = if cfg!(target_arch = "x86_64") {
                        if iov_len == USER_REGS32_LEN {
                            "32-bit (i386 compat)"
                        } else {
                            "64-bit (x86_64)"
                        }
                    } else {
                        "64-bit (aarch64)"
                    };
                    log(&format!(
                        "detected child bitness: {} (iov_len={})",
                        bitness_label, iov_len
                    ));
                    abi = Some(picked);
                }
                // Safe to unwrap: we just set `abi` if it was None.
                let abi = abi.unwrap();

                let syscall_num = get_syscall_num(&regs, &abi);

                if !in_syscall {
                    // ── Syscall ENTRY ──
                    in_syscall = true;

                    // Log every syscall number on entry for the first 50
                    // iterations so we can see exactly what TWRP's init
                    // is calling (and in what order) before it dies or
                    // settles into its main loop. This is invaluable for
                    // diagnosing seccomp SIGSYS kills: the entry log
                    // shows the syscall number that was about to be
                    // attempted, and the very next log line is typically
                    // the SIGSYS intercept.
                    if loop_count <= 50 {
                        log(&format!("syscall entry: nr={}", syscall_num));
                    }

                    match syscall_num {
                        n if n == abi.getpid => {
                            pending_getpid = true;
                            if loop_count <= 20 {
                                log("intercepted getpid() -> will return 1");
                            }
                        }
                        n if n == abi.getppid => {
                            pending_getpid = true;
                            if loop_count <= 20 {
                                log("intercepted getppid() -> will return 1");
                            }
                        }
                        n if n == abi.open || n == abi.openat || n == abi.openat2 => {
                            let path_arg_index = if syscall_num == abi.open {
                                abi.reg_arg1
                            } else {
                                abi.reg_arg2
                            };
                            let path_addr = get_syscall_arg(&regs, path_arg_index);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path && loop_count <= 500 {
                                    log(&format!("intercepted open({}) -> {}", path, translated));
                                }
                                if translated != path {
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        n if n == abi.stat
                            || n == abi.lstat
                            || n == abi.newfstatat
                            || n == abi.statx =>
                        {
                            let path_arg_index =
                                if syscall_num == abi.stat || syscall_num == abi.lstat {
                                    abi.reg_arg1
                                } else {
                                    abi.reg_arg2
                                };
                            let path_addr = get_syscall_arg(&regs, path_arg_index);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path {
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        n if n == abi.access || n == abi.faccessat => {
                            let path_arg_index = if syscall_num == abi.access {
                                abi.reg_arg1
                            } else {
                                abi.reg_arg2
                            };
                            let path_addr = get_syscall_arg(&regs, path_arg_index);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path {
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        n if n == abi.readlink || n == abi.readlinkat => {
                            let path_arg_index = if syscall_num == abi.readlink {
                                abi.reg_arg1
                            } else {
                                abi.reg_arg2
                            };
                            let path_addr = get_syscall_arg(&regs, path_arg_index);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path {
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        n if n == abi.chdir => {
                            let path_addr = get_syscall_arg(&regs, abi.reg_arg1);
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
                        if let Ok(len) = ptrace_getregs(pid, &mut regs2) {
                            // `abi` was unwrapped above (it is the
                            // local `ChildAbi` for this syscall-stop)
                            // and is guaranteed Some because we only
                            // ever set pending_getpid on a syscall
                            // ENTRY stop, which can only happen after
                            // the ENTRY stop that initialized `abi`.
                            set_syscall_ret(&mut regs2, &abi, 1);
                            let _ = ptrace_setregs(pid, &regs2, len);
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
            } else if sig == libc::SIGSYS {
                // SIGSYS (signal 31) is raised by the kernel when the
                // child calls a syscall blocked by a SECCOMP_RET_TRAP
                // filter (e.g. mount, mkdir, chmod, chroot, unshare).
                // The default action for SIGSYS is to terminate the
                // process with a core dump — so if we forwarded it to
                // the child (as we do for other signals below), TWRP's
                // init would die the moment it tried to mount /tmp or
                // mkdir /dev/block.
                //
                // Instead we INTERCEPT the signal: rewrite the blocked
                // syscall into a harmless getpid and force a return
                // value, then resume WITHOUT delivering the signal.
                //
                // The return value depends on the original syscall:
                //   - access()      → 0 (success). The previous behaviour
                //     returned -ENOENT so init would treat the probed
                //     path as missing, but init apparently needs certain
                //     paths to "exist" (e.g. to decide which init.rc
                //     fragment to source) and the -ENOENT caused init
                //     to exit with code 1 after ~183 iterations. We now
                //     return 0 (the original behaviour before the
                //     -ENOENT "fix") and log the PATH argument so we can
                //     see exactly what init is probing.
                //   - rt_sigprocmask() → 0 (success). For a ptraced
                //     process the actual signal mask doesn't matter —
                //     bionic just needs the call to "succeed" so its
                //     init path continues. Skipping it (i.e. returning
                //     0 without actually setting the mask) is the
                //     correct emulation here.
                //   - All other blocked syscalls (mount, chroot, mkdir,
                //     chmod, unshare, …) → rewrite to getpid and return
                //     0 (success). This is the existing PROOT-style
                //     "always succeed" strategy for rootless containers.
                //     We log a WARNING because lying about these syscalls
                //     succeeding can cause init to operate on a half-
                //     initialised environment, but it is the best we
                //     can do without CAP_SYS_ADMIN.
                //
                // The getpid number used for the rewrite comes from the
                // child's ABI — 20 for a 32-bit child, 39 for a 64-bit
                // child. Using the wrong number (e.g. always 39) would
                // make the kernel re-evaluate a syscall number that does
                // not match the child's syscall table, which on a 32-bit
                // child would either re-raise SIGSYS (if 39 is blocked)
                // or invoke the wrong syscall entirely.
                //
                // We do NOT flip `in_syscall`: seccomp fires during
                // syscall entry, so the next stop will be the
                // syscall-exit-stop of the (now rewritten) syscall —
                // the same phase we were already heading to.
                let mut sigsys_regs: Regs = unsafe { std::mem::zeroed() };
                match ptrace_getregs(pid, &mut sigsys_regs) {
                    Ok(len) => {
                        // Initialize the ABI on the first successful
                        // register read — seccomp can fire on the very
                        // first syscall after execve, before any
                        // SIGTRAP|0x80 syscall-stop has had a chance
                        // to set it.
                        if abi.is_none() {
                            let picked = pick_abi_from_iov_len(len);
                            let bitness_label = if cfg!(target_arch = "x86_64") {
                                if len == USER_REGS32_LEN {
                                    "32-bit (i386 compat)"
                                } else {
                                    "64-bit (x86_64)"
                                }
                            } else {
                                "64-bit (aarch64)"
                            };
                            log(&format!(
                                "detected child bitness (SIGSYS path): {} (iov_len={})",
                                bitness_label, len
                            ));
                            abi = Some(picked);
                        }
                        let a = abi.unwrap();

                        // Read the ORIGINAL syscall number BEFORE rewriting
                        // it. This is the syscall that seccomp blocked —
                        // logging it is the only way to know which kernel
                        // facilities TWRP's init is asking for that we're
                        // silently masking (mount? chroot? unshare? ioctl
                        // on a specific fd?). Without this log we just see
                        // "intercepted SIGSYS" with no clue WHAT was
                        // intercepted.
                        let original_syscall = get_syscall_num(&sigsys_regs, &a);
                        let name = syscall_name(original_syscall, &a);

                        // For access(): read the PATH argument from the
                        // child's memory so we can log what init is
                        // probing. access() takes (const char *pathname,
                        // int mode), so the path pointer is in
                        // reg_arg1. On aarch64 `abi.access` is -1
                        // (aarch64 uses faccessat instead), so this
                        // branch is dead on that architecture — the
                        // comparison still compiles and is harmless.
                        // The same applies on x86_64: `abi.access` is
                        // 21 (64-bit) or 33 (32-bit), matched against
                        // the original syscall number from the child's
                        // own ABI so the comparison is correct for both
                        // bitnesses.
                        let access_path: Option<String> = if original_syscall == a.access {
                            let path_addr = get_syscall_arg(&sigsys_regs, a.reg_arg1);
                            read_child_string(pid, path_addr)
                        } else {
                            None
                        };

                        // Push a human-readable description into the rolling
                        // history so the exit handler can print "last N
                        // blocked syscalls". For access() we include the
                        // path being probed — that is the single most
                        // useful diagnostic when init dies after a flurry
                        // of access() calls (it tells us which file init
                        // was looking for and couldn't find).
                        let history_entry: String = if original_syscall == a.access {
                            match &access_path {
                                Some(p) => {
                                    format!("access({:?}) nr={}", p, original_syscall)
                                }
                                None => {
                                    format!("access(?) nr={} [{}]", original_syscall, name)
                                }
                            }
                        } else {
                            format!("nr={} [{}]", original_syscall, name)
                        };
                        if recent_sigsys.len() == RECENT_SIGSYS_CAP {
                            recent_sigsys.pop_front();
                        }
                        recent_sigsys.push_back(history_entry);

                        // Rewrite the syscall number to getpid (a
                        // harmless, always-allowed syscall) so the
                        // kernel does not re-evaluate the original
                        // blocked number and re-raise SIGSYS when we
                        // resume. This is done for ALL intercepted
                        // syscalls — the return value (set below) is
                        // what differs per-syscall. We use `a.getpid`
                        // so the rewrite uses the correct number for
                        // the child's bitness (20 on i386, 39 on
                        // x86_64, 172 on aarch64) — using the wrong
                        // number here would re-trip seccomp on a
                        // 32-bit child because 39 (x86_64 getpid)
                        // maps to a completely different i386 syscall.
                        set_syscall_num(&mut sigsys_regs, &a, a.getpid);

                        // Decide on the return value based on the original
                        // syscall. See the long comment above for the full
                        // rationale per-syscall.
                        //
                        // On aarch64 `a.access` is -1 (aarch64 uses
                        // faccessat instead), so the `access` branch is
                        // effectively dead on that architecture — the
                        // comparison still compiles and is harmless.
                        let ret_val: i64 = if original_syscall == a.access {
                            let path_display = access_path.as_deref().unwrap_or("?");
                            log(&format!(
                                "intercepted SIGSYS — access({}) nr={} → returning 0 (success)",
                                path_display, original_syscall
                            ));
                            0
                        } else if original_syscall == a.rt_sigprocmask {
                            log(&format!(
                                "intercepted SIGSYS — rt_sigprocmask() nr={} [{}] (rewriting to getpid nr={}, returning 0 — signal mask emulation)",
                                original_syscall, name, a.getpid
                            ));
                            0
                        } else {
                            log(&format!(
                                "intercepted SIGSYS — syscall nr={} [{}] (rewriting to getpid nr={}, returning 0) — WARNING: unexpected SIGSYS for this syscall, may cause issues",
                                original_syscall, name, a.getpid
                            ));
                            0
                        };
                        // Force the return value. The child will see the
                        // (blocked) syscall as having returned `ret_val`.
                        set_syscall_ret(&mut sigsys_regs, &a, ret_val);
                        let _ = ptrace_setregs(pid, &sigsys_regs, len);
                    }
                    Err(_) => {
                        // ptrace_getregs failed — we couldn't read the
                        // registers, so we can't log the original syscall
                        // number. Fall back to the old generic message so
                        // the count of SIGSYS events is still visible.
                        log("intercepted SIGSYS (seccomp-blocked syscall) — ptrace_getregs failed; skipping and returning 0");
                    }
                }

                // Do NOT call PTRACE_SYSCALL here — the loop top will
                // do it with resume_signal = 0 (already reset), which
                // resumes the child WITHOUT forwarding the signal.
                // Calling PTRACE_SYSCALL here would race the loop-top
                // call (child already running → ESRCH → premature exit).
                continue;
            } else {
                // The child stopped because of a real signal that was
                // NOT a syscall-stop, NOT a debugger trap, and NOT a
                // seccomp SIGSYS — e.g. SIGSEGV, SIGBUS, SIGFPE, or a
                // SIGCHLD-style signal delivered by the kernel. Forward
                // it to the child so its own signal handlers (or default
                // action) run.
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
                log(&format!("forwarding signal {} to child", sig));
                // Stash the signal so the SINGLE PTRACE_SYSCALL at the
                // loop top injects it on resume. We do NOT call
                // PTRACE_SYSCALL here — doing so would race the loop-top
                // call (child already running → ESRCH → premature exit).
                resume_signal = sig;
                continue;
            }
        }
    }
}
