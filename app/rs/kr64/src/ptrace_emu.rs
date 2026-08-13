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
//     reading /proc/<pid>/exe and inspecting the ELF header
//     (EI_CLASS at byte 4, e_machine at bytes 18-19), then pick
//     the matching `ChildAbi` (ABI_X86_32 vs ABI_X86_64). All
//     syscall-number comparisons and register-index lookups go
//     through that ABI so the same loop body handles both cases.
//     We previously detected bitness by inspecting the `iov_len`
//     returned by PTRACE_GETREGSET, but on the x86_64 Android
//     emulator PTRACE_GETREGSET returns EIO (forcing us onto the
//     PTRACE_GETREGS fallback, which has no iov_len) so we now use
//     the ELF header as the single source of truth for bitness.
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
// iovec. PTRACE_GETREGSET works reliably on real aarch64 hardware.
//
// CRITICAL (x86_64, emulator fix): PTRACE_GETREGSET returns EIO on the
// x86_64 Android emulator. On x86_64 we therefore TRY PTRACE_GETREGSET
// first (so the aarch64 and x86_64 paths share the same primary code)
// and FALL BACK to the legacy PTRACE_GETREGS (request 12) on EIO.
// PTRACE_GETREGS works on x86_64 for both 64-bit children and 32-bit
// (i386 compat) children — the kernel zero-extends each 32-bit register
// value into the corresponding 64-bit slot of user_regs_struct, so the
// ABI_X86_32 register indices (5=rbx, 11=rcx, 12=rdx, 13=rsi) work
// correctly against that 64-bit view.
//
// Because the GETREGS fallback path has no iov_len, child-bitness
// detection is now done independently by reading /proc/<pid>/exe and
// inspecting the ELF header — see `detect_child_is_64bit`.
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
// interface for the GETREGSET/SETREGSET requests on aarch64. We do the
// same on x86_64 (the raw-syscall path costs nothing there and keeps
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

// Named constants for the generic PTRACE_*REGSET interface, used as
// the PRIMARY register-fetch path on BOTH x86_64 and aarch64. On
// x86_64 we fall back to the legacy PTRACE_GETREGS/SETREGS (numbers
// 12/13) when GETREGSET returns EIO — see `ptrace_getregs_legacy`
// and `ptrace_setregs_legacy`. The constants below cover only the
// REGSET requests because the legacy request numbers are small
// integers that read more clearly as `12`/`13` at the fallback call
// sites (with a comment pointing at <linux/ptrace.h>). Using names
// instead of bare `33`/`34`/`1` makes the intent obvious and stops
// future readers from wondering whether they are syscall numbers,
// NT_* types, or something else entirely.
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
// We detect bitness at the first syscall stop by reading
// /proc/<pid>/exe and inspecting the ELF header (see
// `detect_child_is_64bit`): EI_CLASS=1 → 32-bit i386 child,
// EI_CLASS=2 → 64-bit x86_64 child. We then select ABI_X86_32 or
// ABI_X86_64 accordingly. (We used to inspect the `iov_len` returned
// by PTRACE_GETREGSET, but on the x86_64 Android emulator
// PTRACE_GETREGSET returns EIO and the GETREGS fallback has no
// iov_len — so the ELF header is now the single source of truth.)
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
    // Stack-pointer register index — used to reserve a scratch area
    // BELOW the child's current stack pointer for translated paths.
    // Translated paths are always longer than the originals (e.g.
    // `/init.rc` → `/data/user/0/io.twoyi/rootfs/init.rc`), so we
    // cannot overwrite the original string in the child's memory —
    // it may live in read-only .rodata, and even when writable the
    // longer translation would clobber adjacent bytes.
    //
    // Instead of allocating a scratch page via mmap (which is itself
    // blocked by seccomp on the host, just like chroot/mount/...),
    // we reserve a 4 KiB region below the child's current stack
    // pointer. Linux guarantees at least 128 bytes of stack redzone
    // below `rsp`/`sp`, and we only need a few hundred bytes for
    // short path strings — the kernel will not touch this region
    // until the child actually pushes a new stack frame, by which
    // time we have already consumed the translated path and the
    // syscall has returned.
    //
    // On x86_64 user_regs_struct this is index 19 (rsp). On aarch64
    // user_pt_regs the `sp` field follows the 31-entry `regs` array
    // (x0..x30), so it is index 31. On a 32-bit (i386 compat) child
    // the kernel zero-extends `esp` into the 64-bit `rsp` slot when
    // reporting registers via PTRACE_GETREGS, so we use index 19
    // there too.
    reg_sp: usize,
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
    reg_sp: 19,      // rsp
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
    // On a 32-bit child the kernel zero-extends `esp` into the 64-bit
    // `rsp` slot when reporting registers via PTRACE_GETREGS, so we
    // use the same index as the 64-bit ABI (19).
    reg_sp: 19, // rsp (zero-extended esp)
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
    // Aarch64 user_pt_regs is `u64 regs[31]` (x0..x30) followed by
    // `sp`, `pc`, `pstate`. The `sp` field is therefore at index 31
    // when reinterpreted as a flat `u64` array.
    reg_sp: 31, // sp
};

/// Detect whether the traced child is a 32-bit (i386) or 64-bit (x86_64)
/// ELF by reading its `/proc/<pid>/exe` symlink target and parsing the
/// ELF header.
///
/// We do NOT rely on the `iov_len` returned by PTRACE_GETREGSET for
/// bitness detection anymore. On the x86_64 Android emulator (and
/// possibly other bionic/kernel combinations) PTRACE_GETREGSET returns
/// EIO, which forces `ptrace_getregs` to fall back to PTRACE_GETREGS —
/// and PTRACE_GETREGS does not expose `iov_len` at all. Reading the
/// child's own ELF header is also a more reliable bitness signal than
/// the regset size the kernel happened to report: it tells us the
/// executable's actual architecture.
///
/// # What we read
///
/// The ELF header's first 20 bytes contain everything we need:
///   - bytes 0-3   : ELFMAG ("\x7fELF")
///   - byte  4     : `EI_CLASS` (1=ELFCLASS32, 2=ELFCLASS64)
///   - byte  5     : `EI_DATA`  (1=LSB, 2=MSB) — assumed LSB for x86
///   - bytes 18-19 : `e_machine` (little-endian u16; EM_386=3, EM_X86_64=62)
///
/// `EI_CLASS` is the primary bitness signal — we cross-check `e_machine`
/// for robustness (in case of a malformed ELF header). On disagreement
/// `EI_CLASS` wins.
///
/// # Return value
///
/// - `Some(true)`  → 64-bit (x86_64) child
/// - `Some(false)` → 32-bit (i386 compat) child
/// - `None`        → detection failed (file missing, not an ELF, I/O
///   error, …). Callers fall back to the 64-bit ABI on `None`, which
///   matches the historical behaviour.
///
/// On aarch64 this function does not exist — aarch64 has no 32-bit
/// userspace, so the child is unconditionally 64-bit and the loop uses
/// `ABI_AARCH64` directly without any detection step.
#[cfg(target_arch = "x86_64")]
fn detect_child_is_64bit(pid: libc::pid_t) -> Option<bool> {
    use std::io::Read;

    // /proc/<pid>/exe is a symlink to the child's executable. We use
    // std::fs::File::open, which follows symlinks automatically, so we
    // read the actual ELF image rather than the symlink target string.
    let path = format!("/proc/{}/exe", pid);
    let mut f = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return None,
    };

    let mut hdr = [0u8; 20];
    if f.read_exact(&mut hdr).is_err() {
        return None;
    }

    classify_elf_bitness(&hdr)
}

/// Pure byte-level ELF header classifier — the testable core of
/// [`detect_child_is_64bit`]. Takes the first 20 bytes of a file's
/// ELF header and returns:
///   - `Some(true)`  → 64-bit (ELFCLASS64 / EM_X86_64)
///   - `Some(false)` → 32-bit (ELFCLASS32 / EM_386)
///   - `None`        → not a recognizable x86 ELF (bad magic, unknown
///     EI_CLASS, unknown e_machine).
///
/// `EI_CLASS` (byte 4) takes precedence over `e_machine` (bytes 18-19)
/// because every valid i386/x86_64 ELF has EI_CLASS set correctly;
/// `e_machine` is only consulted as a tiebreaker for malformed headers
/// where EI_CLASS is neither 1 nor 2.
///
/// Defined on all architectures so the unit tests run on the host
/// (typically x86_64-linux), even when the production caller
/// (`detect_child_is_64bit`) is x86_64-only. On aarch64 the function
/// has no production caller, so we silence the dead-code lint — the
/// tests still exercise the byte-parsing logic.
#[cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]
fn classify_elf_bitness(hdr: &[u8; 20]) -> Option<bool> {
    // Validate the ELF magic. If this isn't an ELF we have no way to
    // detect bitness — return None so the caller falls back to 64-bit.
    if &hdr[0..4] != b"\x7fELF" {
        return None;
    }

    let ei_class = hdr[4];
    // e_machine is a little-endian u16 at offset 18. i386 and x86_64
    // are both little-endian, so `from_le_bytes` is always correct
    // here.
    const EM_386: u16 = 3;
    const EM_X86_64: u16 = 62;
    let e_machine = u16::from_le_bytes([hdr[18], hdr[19]]);

    // EI_CLASS takes precedence over e_machine: a valid i386 or x86_64
    // ELF always has EI_CLASS == 1 or 2 respectively, and e_machine is
    // only consulted as a tiebreaker for malformed headers.
    match (ei_class, e_machine) {
        (1, _) => Some(false), // ELFCLASS32 — i386 child
        (2, _) => Some(true),  // ELFCLASS64 — x86_64 child
        (_, EM_386) => Some(false),
        (_, EM_X86_64) => Some(true),
        _ => None,
    }
}

// ── Architecture-specific get/set registers ────────────────────────

/// Get the child's registers.
///
/// On **aarch64** we always use `PTRACE_GETREGSET` with `NT_PRSTATUS`
/// (the only regset-fetching ptrace request the aarch64 kernel
/// supports — `PTRACE_GETREGS` returns `EIO` there).
///
/// On **x86_64** we try `PTRACE_GETREGSET` first (because it exposes
/// `iov_len`, which historically we used for child-bitness detection),
/// but on the x86_64 Android emulator `PTRACE_GETREGSET` returns
/// `EIO`. In that case we fall back to the legacy `PTRACE_GETREGS`
/// request (number 12), which works on x86_64 — both for 64-bit
/// children and for 32-bit (i386 compat) children, where the kernel
/// zero-extends each 32-bit register value into the corresponding
/// 64-bit slot of `user_regs_struct`.
///
/// Because the fallback path has no `iov_len`, child bitness is now
/// detected separately by [`detect_child_is_64bit`] (which reads the
/// child's ELF header via `/proc/<pid>/exe`). The `usize` we return
/// is therefore only used as the `iov_len` argument to the matching
/// [`ptrace_setregs`] call — on the GETREGS fallback path it is just
/// `sizeof(Regs)` (216 on x86_64), which the SETREGS fallback path
/// ignores.
///
/// The function name `ptrace_getregs` is historical — on x86_64 it
/// may now actually use `PTRACE_GETREGS` (the legacy request), and on
/// aarch64 it always uses `PTRACE_GETREGSET`. Kept the name for parity
/// with the call sites in `run_ptrace_loop`.
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
    // We use the raw-syscall path on BOTH architectures: the
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
        let e = std::io::Error::last_os_error();
        // On x86_64, PTRACE_GETREGSET returns EIO on the Android
        // emulator (the kernel exposes PTRACE_GETREGSET only for
        // ptrace requests that the kernel actually implements for the
        // traced task's architecture; the emulator's bionic/kernel
        // combination evidently does not). Fall back to the legacy
        // PTRACE_GETREGS, which works on x86_64 for both 64-bit and
        // 32-bit (compat) children. On aarch64 PTRACE_GETREGS does
        // not exist at all, so we DON'T fall back there — the
        // aarch64 path MUST go through PTRACE_GETREGSET (which works
        // on real aarch64 hardware).
        #[cfg(target_arch = "x86_64")]
        if e.raw_os_error() == Some(libc::EIO) {
            return ptrace_getregs_legacy(pid, regs);
        }
        return Err(e);
    }
    Ok(iov.iov_len)
}

/// x86_64-only fallback for [`ptrace_getregs`]: read registers via the
/// legacy `PTRACE_GETREGS` request (number 12).
///
/// `PTRACE_GETREGS` is the historical x86_64 register-fetch request.
/// It does NOT expose `iov_len` — the kernel always writes
/// `sizeof(user_regs_struct)` (= 216) bytes into the data pointer —
/// so we return `sizeof(Regs)` as the "iov_len" and rely on
/// [`detect_child_is_64bit`] for bitness detection.
///
/// For a 32-bit (i386 compat) child the kernel zero-extends each
/// 32-bit register value into the corresponding 64-bit slot of
/// `user_regs_struct` (rbx ← ebx, rcx ← ecx, …), so the indices in
/// [`ABI_X86_32`] work correctly against this 64-bit view.
///
/// We invoke the kernel via `libc::syscall(SYS_ptrace, …)` for the
/// same reason [`ptrace_getregs`] does — to bypass bionic's variadic
/// `ptrace()` wrapper.
#[cfg(target_arch = "x86_64")]
fn ptrace_getregs_legacy(pid: libc::pid_t, regs: &mut Regs) -> std::io::Result<usize> {
    // PTRACE_GETREGS = 12 (see <linux/ptrace.h>). Not exposed by the
    // `libc` crate on every target, so use the literal value with a
    // named binding for clarity.
    const PTRACE_GETREGS: libc::c_long = 12;

    // PTRACE_GETREGS signature:
    //   ptrace(PTRACE_GETREGS, pid, /*addr*/ 0, /*data*/ void *)
    // The kernel writes sizeof(user_regs_struct) bytes into `data`.
    // We pass `regs` directly — `Regs` IS `user_regs_struct` on
    // x86_64 (see the type alias near the top of this file).
    let r = unsafe {
        libc::syscall(
            libc::SYS_ptrace,
            PTRACE_GETREGS,
            pid as libc::c_long,
            0,
            regs as *mut Regs as *mut libc::c_void,
        )
    };
    if r == -1 {
        return Err(std::io::Error::last_os_error());
    }
    // No iov_len on this path — return sizeof(Regs) (= 216 on x86_64)
    // so the matching `ptrace_setregs` call has a sane value to
    // forward. The SETREGS fallback path ignores this argument
    // anyway.
    Ok(std::mem::size_of::<Regs>())
}

/// Set the child's registers.
///
/// Mirrors [`ptrace_getregs`]:
///   - On **aarch64** we always use `PTRACE_SETREGSET` with `NT_PRSTATUS`.
///   - On **x86_64** we try `PTRACE_SETREGSET` first, and on `EIO` (the
///     same failure that triggers the GETREGS fallback in `ptrace_getregs`)
///     we fall back to the legacy `PTRACE_SETREGS` request.
///
/// `iov_len` is the value returned by the matching `ptrace_getregs`
/// call. On the SETREGS fallback path it is ignored (the kernel always
/// reads exactly `sizeof(user_regs_struct)` bytes from the data
/// pointer); on the SETREGSET path it is forwarded to the kernel as
/// the iovec length.
///
/// The GET and SET paths are intentionally symmetric: if GETREGSET
/// returned EIO for this child then SETREGSET almost certainly will
/// too, so we fall back to SETREGS in lockstep.
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
        let e = std::io::Error::last_os_error();
        // Same fallback as in `ptrace_getregs`: on x86_64 the
        // PTRACE_*REGSET requests return EIO on the Android emulator,
        // so use the legacy PTRACE_SETREGS (request 13) which writes
        // sizeof(user_regs_struct) bytes from the data pointer. We
        // MUST stay symmetric with `ptrace_getregs` — if we read via
        // GETREGS we must write via SETREGS, otherwise the kernel
        // would reject the write (and we'd silently drop register
        // updates like the SIGSYS "rewrite to getpid" return value).
        #[cfg(target_arch = "x86_64")]
        if e.raw_os_error() == Some(libc::EIO) {
            return ptrace_setregs_legacy(pid, regs);
        }
        return Err(e);
    }
    Ok(())
}

/// x86_64-only fallback for [`ptrace_setregs`]: write registers via
/// the legacy `PTRACE_SETREGS` request (number 13).
///
/// Symmetric with [`ptrace_getregs_legacy`] — see the comment on that
/// function for why we go through `libc::syscall` directly. The kernel
/// reads `sizeof(user_regs_struct)` (= 216) bytes from `regs` and
/// writes them to the child; for a 32-bit compat child the kernel
/// truncates each 64-bit slot to its low 32 bits when populating the
/// child's `user_regs_struct32`.
#[cfg(target_arch = "x86_64")]
fn ptrace_setregs_legacy(pid: libc::pid_t, regs: &Regs) -> std::io::Result<()> {
    // PTRACE_SETREGS = 13 (see <linux/ptrace.h>).
    const PTRACE_SETREGS: libc::c_long = 13;

    let r = unsafe {
        libc::syscall(
            libc::SYS_ptrace,
            PTRACE_SETREGS,
            pid as libc::c_long,
            0,
            regs as *const Regs as *mut libc::c_void,
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

/// Set a syscall argument register to `val`.
///
/// `arg` is a register index into the `Regs` buffer reinterpreted as a
/// `u64` array — typically one of `abi.reg_arg1` .. `abi.reg_arg4`. Used
/// by the path-translation code to rewrite the path-argument register
/// to point at the translated path inside the scratch area (instead of
/// trying to overwrite the original, possibly read-only, possibly too-
/// short path string in the child's memory).
///
/// On a 32-bit x86 child the kernel takes the low 32 bits of the slot
/// on writeback, which is correct — a 32-bit pointer fits in 32 bits.
fn set_syscall_arg(regs: &mut Regs, arg: usize, val: u64) {
    let regs_ptr = regs as *mut Regs as *mut u64;
    unsafe {
        *regs_ptr.add(arg) = val;
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

/// Write a NUL-terminated string `s` to the child's memory at `addr`,
/// WITHOUT the "new string must be shorter than the existing string"
/// guard that [`write_child_string`] enforces.
///
/// This is used to write translated paths into the dedicated scratch
/// area we reserve below the child's stack pointer (see
/// `run_ptrace_loop`). The scratch area is entirely under our control,
/// so there is no pre-existing string to overflow and no adjacent data
/// to clobber — the length guard is both unnecessary and harmful,
/// because translated paths (e.g. `/init.rc` →
/// `/data/user/0/io.twoyi/rootfs/init.rc`) are ALWAYS longer than the
/// originals.
///
/// Writes one `c_long` (8 bytes on 64-bit hosts, 4 on 32-bit) at a
/// time via `PTRACE_POKEDATA`. The final partial word is zero-padded,
/// which means a few bytes past the NUL terminator are also written —
/// that is fine because we always lay out paths 8-byte-aligned inside
/// the scratch area (see `write_translated_path`) and read each path
/// back as a NUL-terminated C string (so we stop at the first NUL).
///
/// Returns `true` on success, `false` on a `PTRACE_POKEDATA` failure
/// (which typically means `addr` is not a valid mapped address in the
/// child).
fn write_child_string_unchecked(pid: libc::pid_t, addr: u64, s: &str) -> bool {
    if addr == 0 {
        return false;
    }
    let mut new_bytes = s.as_bytes().to_vec();
    new_bytes.push(0); // NUL terminator
    let word_size = std::mem::size_of::<libc::c_long>();
    let mut offset = 0i64;
    while offset < new_bytes.len() as i64 {
        let mut word_bytes = [0u8; 8];
        let chunk_len = std::cmp::min(word_size, new_bytes.len() - offset as usize);
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
        offset += word_size as i64;
    }
    true
}

/// Write a translated path into the child's scratch page and rewrite the
/// path-argument register to point at it.
///
/// This is the core of the path-translation fix. The previous code
/// called [`write_child_string`] with the ORIGINAL path address, which
/// has a length guard (`s.len() >= orig.len() → return false`) that
/// rejects every translated path — translated paths are always longer
/// than the originals. As a result path translation was a complete
/// no-op and init's `openat("/init.rc")` hit the HOST's `/init.rc`
/// (which doesn't exist) → ENOENT → `exit(1)`.
///
/// This helper instead writes the translated path into the dedicated
/// scratch area (reserved below the child's stack pointer — see
/// `run_ptrace_loop`), then updates the path-argument register
/// (`path_arg_index`) to point at the freshly-written string. The
/// child's syscall then proceeds with the translated path.
///
/// # Arguments
/// - `pid` — traced child PID.
/// - `regs` — the child's current register snapshot (taken at the
///   syscall ENTRY stop). Modified in place: the path-arg register is
///   updated, then the whole register set is written back via
///   `ptrace_setregs`.
/// - `iov_len` — the `iovec.iov_len` returned by the matching
///   `ptrace_getregs` call (forwarded to `ptrace_setregs`).
/// - `path_arg_index` — which register slot holds the path pointer
///   (differs per syscall: arg1 for `open`/`stat`/`chdir`, arg2 for
///   `openat`/`newfstatat`/`faccessat`/`readlinkat`).
/// - `scratch_addr` — base address of the scratch area in the child
///   (0 means "not yet allocated").
/// - `scratch_offset` — rotating write cursor within the area; advanced
///   by the path length (8-byte aligned) and wrapped to 0 near the end.
/// - `translated` — the translated path string (no NUL — we add it).
///
/// Returns `true` if the path was written and the arg register updated,
/// `false` if the scratch area is not yet allocated (caller should fall
/// back to the legacy in-place overwrite, which will typically fail
/// silently for longer strings — but at least we don't crash).
fn write_translated_path(
    pid: libc::pid_t,
    regs: &mut Regs,
    iov_len: usize,
    path_arg_index: usize,
    scratch_addr: u64,
    scratch_offset: &mut usize,
    translated: &str,
) -> bool {
    if scratch_addr == 0 {
        return false; // scratch page not yet allocated
    }
    let new_addr = scratch_addr + *scratch_offset as u64;
    if !write_child_string_unchecked(pid, new_addr, translated) {
        return false;
    }
    // Update the path-argument register to point at the scratch copy.
    set_syscall_arg(regs, path_arg_index, new_addr);
    if ptrace_setregs(pid, regs, iov_len).is_err() {
        return false;
    }
    // Advance the rotating cursor: round up the path length (including
    // the NUL terminator) to 8-byte alignment so the next path starts
    // on a clean word boundary. Wrap to 0 when fewer than 256 bytes
    // remain — that is plenty of room for any realistic path and avoids
    // a path straddling the end of the page.
    let path_len = translated.len() + 1; // include NUL
    *scratch_offset += (path_len + 7) & !7;
    if *scratch_offset + 256 > 4096 {
        *scratch_offset = 0;
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
    // read /proc/<pid>/exe and inspect the ELF header to pick
    // ABI_X86_32 vs ABI_X86_64 (on x86_64), or unconditionally use
    // ABI_AARCH64 (on aarch64). Until then we have no registers to
    // look at, so there is nothing to dispatch on.
    let mut abi: Option<ChildAbi> = None;
    // ── Scratch area for translated paths ───────────────────────────
    //
    // Translated paths (e.g. `/init.rc` →
    // `/data/user/0/io.twoyi/rootfs/init.rc`) are ALWAYS longer than
    // the original, so we cannot overwrite the original string in the
    // child's memory — it may live in read-only .rodata, and even when
    // writable the longer translation would clobber adjacent bytes.
    //
    // We previously tried to `mmap` a dedicated scratch page in the
    // child by hijacking the first getpid() syscall, but mmap is ALSO
    // blocked by seccomp on the host (the same filter that blocks
    // chroot/mount/...). The hijacked getpid returned the mmap syscall
    // number itself (0x9 on x86_64) instead of a real address, and
    // every subsequent path translation wrote to invalid memory.
    //
    // Instead we reserve a scratch area BELOW the child's current
    // stack pointer. At the first syscall ENTRY stop we read the
    // stack pointer via `abi.reg_sp` and set `scratch_addr =
    // sp - 4096` (a 4 KiB region). Linux guarantees at least 128
    // bytes of stack redzone below `rsp`/`sp`, and we only need a
    // few hundred bytes for short path strings — the kernel will not
    // touch this region until the child actually pushes a new stack
    // frame, by which time we have already consumed the translated
    // path and the syscall has returned.
    //
    // No syscall is required: we just write to existing memory in
    // the child's address space via PTRACE_POKEDATA.
    //
    // `scratch_addr` is 0 until the first syscall ENTRY stop reads
    // the stack pointer; `scratch_offset` is the rotating write
    // cursor within the 4 KiB scratch area (paths are laid out
    // end-to-end, 8-byte aligned, and the cursor wraps to 0 when
    // the area is nearly full).
    let mut scratch_addr: u64 = 0;
    let mut scratch_offset: usize = 0;
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
                // The returned `iov_len` is no longer used here for bitness
                // detection — we read /proc/<pid>/exe instead (see
                // `detect_child_is_64bit`), because the PTRACE_GETREGS
                // fallback path on x86_64 has no real iov_len to inspect.
                // We DO still need iov_len itself, though: it is the
                // `iovec.iov_len` value we must hand back to
                // `ptrace_setregs` when rewriting registers (e.g. when
                // pointing the path argument register at the scratch
                // area). On the PTRACE_GETREGS fallback path it is just
                // sizeof(Regs), which the SETREGS fallback ignores — so
                // forwarding it is always correct.
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
                // successful register read. On x86_64 we read the
                // child's ELF header via /proc/<pid>/exe to detect
                // bitness (PTRACE_GETREGS, which is the fallback path
                // on the x86_64 Android emulator, does not expose
                // iov_len — so iov_len-based detection no longer
                // works there). On aarch64 the child is always 64-bit
                // so we use ABI_AARCH64 unconditionally.
                if abi.is_none() {
                    #[cfg(target_arch = "x86_64")]
                    let (picked, bitness_label) = match detect_child_is_64bit(pid) {
                        Some(true) => (ABI_X86_64, "64-bit (x86_64)"),
                        Some(false) => (ABI_X86_32, "32-bit (i386 compat)"),
                        None => (
                            ABI_X86_64,
                            "unknown (ELF detection failed — defaulting to 64-bit)",
                        ),
                    };
                    #[cfg(target_arch = "aarch64")]
                    let (picked, bitness_label) = (ABI_AARCH64, "64-bit (aarch64)");
                    log(&format!("detected child bitness: {}", bitness_label));
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

                    // ── Lazy scratch-area reservation ──────────────────
                    //
                    // Translated paths are always longer than the originals
                    // (e.g. `/init.rc` → `/data/user/0/io.twoyi/rootfs/init.rc`),
                    // so we cannot overwrite the original string in the
                    // child's memory. Instead we reserve a 4 KiB scratch
                    // area BELOW the child's current stack pointer and
                    // point the path-argument register at translated paths
                    // inside it.
                    //
                    // We reserve the area LAZILY at the first syscall
                    // ENTRY stop (any syscall — we do not need to hijack
                    // a getpid here): we read the stack pointer via
                    // `abi.reg_sp`, set `scratch_addr = sp - 4096`, and
                    // log it. No syscall is required — we are just
                    // picking a writable address inside the child's
                    // existing stack mapping. Linux guarantees at least
                    // 128 bytes of stack redzone below `rsp`/`sp`, and
                    // we only need a few hundred bytes for short path
                    // strings, so this is safe.
                    if scratch_addr == 0 {
                        let sp = get_syscall_arg(&regs, abi.reg_sp);
                        // Reserve 4 KiB below the current stack pointer.
                        // 8-byte align defensively in case the child's sp
                        // happened to be unaligned at this stop.
                        scratch_addr = (sp - 4096) & !7u64;
                        log(&format!(
                            "scratch area at {:#x} (below stack pointer {:#x})",
                            scratch_addr, sp
                        ));
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
                                if translated != path
                                    && !write_translated_path(
                                        pid,
                                        &mut regs,
                                        iov_len,
                                        path_arg_index,
                                        scratch_addr,
                                        &mut scratch_offset,
                                        &translated,
                                    )
                                {
                                    // Scratch area not yet allocated — fall
                                    // back to the legacy in-place overwrite
                                    // (will likely fail for longer strings
                                    // but does not crash). In practice this
                                    // branch is dead because we reserve the
                                    // scratch area at the very first syscall
                                    // ENTRY stop (before any path-bearing
                                    // syscall can be intercepted), but the
                                    // fallback is harmless and keeps the
                                    // `write_translated_path` failure mode
                                    // well-defined.
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
                                if translated != path
                                    && !write_translated_path(
                                        pid,
                                        &mut regs,
                                        iov_len,
                                        path_arg_index,
                                        scratch_addr,
                                        &mut scratch_offset,
                                        &translated,
                                    )
                                {
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
                                if translated != path
                                    && !write_translated_path(
                                        pid,
                                        &mut regs,
                                        iov_len,
                                        path_arg_index,
                                        scratch_addr,
                                        &mut scratch_offset,
                                        &translated,
                                    )
                                {
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
                                if translated != path
                                    && !write_translated_path(
                                        pid,
                                        &mut regs,
                                        iov_len,
                                        path_arg_index,
                                        scratch_addr,
                                        &mut scratch_offset,
                                        &translated,
                                    )
                                {
                                    write_child_string(pid, path_addr, &translated);
                                }
                            }
                        }
                        n if n == abi.chdir => {
                            let path_addr = get_syscall_arg(&regs, abi.reg_arg1);
                            if let Some(path) = read_child_string(pid, path_addr) {
                                let translated = translate_path(rootfs, &path);
                                if translated != path
                                    && !write_translated_path(
                                        pid,
                                        &mut regs,
                                        iov_len,
                                        abi.reg_arg1,
                                        scratch_addr,
                                        &mut scratch_offset,
                                        &translated,
                                    )
                                {
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

                            // Fake the getpid/getppid return value to 1
                            // (the intended fake-PID-1 behaviour).
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
                //   - rt_sigprocmask() → 0 (success), AND the syscall
                //     number is NOT rewritten (see the EXCEPTION comment
                //     at the `set_syscall_num` call below for why).
                //     For a ptraced process the actual signal mask
                //     doesn't matter — bionic just needs the call to
                //     "succeed" so its init path continues. Returning 0
                //     without actually setting the mask (and without
                //     rewriting to getpid) is the correct emulation
                //     here: the kernel already aborted the syscall
                //     when seccomp fired, so it skips straight to
                //     syscall-exit using our return value of 0.
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
                        // to set it. We use the same ELF-based
                        // detection as the SIGTRAP|0x80 path so the
                        // two paths agree on the child's bitness even
                        // when PTRACE_GETREGSET returns EIO (and we
                        // silently fell through to PTRACE_GETREGS).
                        if abi.is_none() {
                            #[cfg(target_arch = "x86_64")]
                            let (picked, bitness_label) = match detect_child_is_64bit(pid) {
                                Some(true) => (ABI_X86_64, "64-bit (x86_64)"),
                                Some(false) => (ABI_X86_32, "32-bit (i386 compat)"),
                                None => (
                                    ABI_X86_64,
                                    "unknown (ELF detection failed — defaulting to 64-bit)",
                                ),
                            };
                            #[cfg(target_arch = "aarch64")]
                            let (picked, bitness_label) = (ABI_AARCH64, "64-bit (aarch64)");
                            log(&format!(
                                "detected child bitness (SIGSYS path): {}",
                                bitness_label
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
                        // syscalls EXCEPT rt_sigprocmask — the return
                        // value (set below) is what differs per-syscall.
                        // We use `a.getpid` so the rewrite uses the
                        // correct number for the child's bitness (20 on
                        // i386, 39 on x86_64, 172 on aarch64) — using
                        // the wrong number here would re-trip seccomp
                        // on a 32-bit child because 39 (x86_64 getpid)
                        // maps to a completely different i386 syscall.
                        //
                        // EXCEPTION — rt_sigprocmask: when seccomp traps
                        // a syscall, the kernel ABORTS it (it does NOT
                        // execute the syscall handler) and delivers
                        // SIGSYS. After the SIGSYS handler runs and we
                        // resume, the kernel proceeds to syscall-exit
                        // with whatever return value we set via
                        // PTRACE_SETREGS — it does NOT re-execute the
                        // (possibly rewritten) syscall. HOWEVER: on
                        // some kernels (observed on Android's
                        // aarch64 5.x line) rewriting the syscall
                        // number causes the kernel to actually EXECUTE
                        // the new syscall (getpid) before returning to
                        // syscall-exit, which OVERWRITES our return
                        // value with getpid's real return value (the
                        // child's PID, a positive integer). Bionic's
                        // linker treats rt_sigprocmask's return value
                        // as either 0 (success) or -errno (failure);
                        // seeing a positive PID confuses bionic's
                        // signal-mask initialization and init exits
                        // with code 1. The fix: for rt_sigprocmask,
                        // DON'T rewrite the syscall number — leave
                        // orig_rax as the original (blocked) syscall.
                        // The kernel has already aborted the syscall
                        // (seccomp fired), so it will skip straight to
                        // syscall-exit using our return value of 0,
                        // which is exactly what bionic expects.
                        if original_syscall != a.rt_sigprocmask {
                            set_syscall_num(&mut sigsys_regs, &a, a.getpid);
                        }

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
                            // NO rewrite — see the EXCEPTION comment above.
                            // The kernel already aborted the syscall
                            // (seccomp fired), so we just set the return
                            // value to 0 (success) and resume. Bionic's
                            // linker needs rt_sigprocmask to "succeed"
                            // during its init path; the actual signal
                            // mask doesn't matter for a ptraced process
                            // (the parent controls signal delivery).
                            log(&format!(
                                "intercepted SIGSYS — rt_sigprocmask() nr={} [{}] (NOT rewriting — seccomp already aborted the syscall, returning 0 — signal mask emulation)",
                                original_syscall, name
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic 20-byte ELF header with the given EI_CLASS
    /// and e_machine values. All other bytes are zero — they're not
    /// consulted by `classify_elf_bitness`, so this is sufficient.
    fn elf_hdr(ei_class: u8, e_machine: u16) -> [u8; 20] {
        let mut hdr = [0u8; 20];
        hdr[0..4].copy_from_slice(b"\x7fELF");
        hdr[4] = ei_class;
        // EI_DATA = 1 (little-endian) — what we assume for x86.
        hdr[5] = 1;
        hdr[18..20].copy_from_slice(&e_machine.to_le_bytes());
        hdr
    }

    #[test]
    fn classify_elf_bitness_detects_x86_64() {
        // Real x86_64 ELF: EI_CLASS=2, e_machine=EM_X86_64 (62).
        let hdr = elf_hdr(2, 62);
        assert_eq!(classify_elf_bitness(&hdr), Some(true));
    }

    #[test]
    fn classify_elf_bitness_detects_i386() {
        // Real i386 ELF: EI_CLASS=1, e_machine=EM_386 (3).
        let hdr = elf_hdr(1, 3);
        assert_eq!(classify_elf_bitness(&hdr), Some(false));
    }

    #[test]
    fn classify_elf_bitness_rejects_non_elf() {
        // Not an ELF — random bytes with no ELFMAG.
        let hdr = [0u8; 20];
        assert_eq!(classify_elf_bitness(&hdr), None);
    }

    #[test]
    fn classify_elf_bitness_e_machine_tiebreaker_for_unknown_class() {
        // EI_CLASS=0 (invalid) but e_machine=EM_X86_64 — should fall
        // through to the e_machine tiebreaker and classify as 64-bit.
        let hdr = elf_hdr(0, 62);
        assert_eq!(classify_elf_bitness(&hdr), Some(true));
    }

    #[test]
    fn classify_elf_bitness_e_machine_tiebreaker_for_em_386() {
        // EI_CLASS=0 (invalid) but e_machine=EM_386 — tiebreaker
        // classifies as 32-bit.
        let hdr = elf_hdr(0, 3);
        assert_eq!(classify_elf_bitness(&hdr), Some(false));
    }

    #[test]
    fn classify_elf_bitness_ei_class_wins_on_disagreement() {
        // EI_CLASS=2 (64-bit) but e_machine=EM_386 (32-bit) — EI_CLASS
        // takes precedence, so this is classified as 64-bit. This case
        // doesn't occur for real ELFs but documents the precedence
        // rule.
        let hdr = elf_hdr(2, 3);
        assert_eq!(classify_elf_bitness(&hdr), Some(true));
    }

    /// Verify the classifier against a REAL ELF on disk (the test
    /// binary itself). This is a smoke test that the byte offsets and
    /// endianness assumptions in `classify_elf_bitness` match what
    /// real-world compilers produce.
    #[test]
    fn classify_elf_bitness_against_real_elf() {
        use std::io::Read;

        // The test binary is the current executable. Read the first
        // 20 bytes of /proc/self/exe and classify. On x86_64 hosts
        // this should be a 64-bit ELF; on aarch64 hosts the
        // classification should also be 64-bit (because aarch64 ELFs
        // have EI_CLASS=2 too, and `classify_elf_bitness` falls into
        // the `(2, _) => Some(true)` arm regardless of e_machine).
        let mut f = match std::fs::File::open("/proc/self/exe") {
            Ok(f) => f,
            Err(_) => {
                // /proc/self/exe isn't available on every platform
                // (e.g. some sandboxes). Skip rather than fail.
                return;
            }
        };
        let mut hdr = [0u8; 20];
        if f.read_exact(&mut hdr).is_err() {
            return;
        }
        let classified = classify_elf_bitness(&hdr);
        // The test binary must be a valid ELF — `None` would indicate
        // a regression in either the magic check or the host
        // environment.
        assert!(
            classified == Some(true) || classified == Some(false),
            "classify_elf_bitness returned {:?} for a real ELF",
            classified
        );

        // On x86_64 hosts the test binary IS a 64-bit x86_64 ELF, so
        // the classifier must say so. (We don't assert anything on
        // aarch64 because the function is x86_64-only in production
        // — the aarch64 path never calls it.)
        if cfg!(target_arch = "x86_64") {
            assert_eq!(
                classified,
                Some(true),
                "x86_64 test binary should be classified as 64-bit"
            );
        }
    }

    /// `detect_child_is_64bit` (the production caller of
    /// `classify_elf_bitness`) reads `/proc/<pid>/exe`. Verify that
    /// passing our own PID correctly identifies the host binary's
    /// bitness. This exercises the file-reading code path on top of
    /// the byte-level classifier.
    #[cfg(target_arch = "x86_64")]
    #[test]
    fn detect_child_is_64bit_against_self() {
        let self_pid = unsafe { libc::getpid() };
        let classified = detect_child_is_64bit(self_pid);
        assert_eq!(
            classified,
            Some(true),
            "the test binary itself is a 64-bit x86_64 ELF"
        );
    }

    // ── translate_path tests ────────────────────────────────────────
    //
    // These verify the path-translation rules that the scratch-area
    // mechanism exists to support — making sure a translated path is
    // indeed longer than the original (which is the whole reason we
    // need the scratch area in the first place).

    #[test]
    fn translate_path_prepends_rootfs_for_init_rc() {
        let t = translate_path("/data/user/0/io.twoyi/rootfs", "/init.rc");
        assert_eq!(t, "/data/user/0/io.twoyi/rootfs/init.rc");
        // The translated path MUST be longer — this is the exact
        // situation that broke the old `write_child_string` length
        // guard and motivated the scratch-area fix.
        assert!(t.len() > "/init.rc".len());
    }

    #[test]
    fn translate_path_leaves_proc_sys_dev_untouched() {
        let rootfs = "/data/user/0/io.twoyi/rootfs";
        for p in &[
            "/proc/self/status",
            "/sys/class/net",
            "/dev/null",
            "/data/data",
        ] {
            assert_eq!(
                translate_path(rootfs, p),
                *p,
                "path {} should not be translated",
                p
            );
        }
    }

    #[test]
    fn translate_path_leaves_relative_untouched() {
        // Relative paths are returned as-is (no rootfs prefix).
        assert_eq!(translate_path("/r", "relative/path"), "relative/path");
    }
}
