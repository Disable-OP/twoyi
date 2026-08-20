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
//     - fchown / fchmod / capget / ioprio_get / ioprio_set /
//       mount / rt_sigprocmask / mknod → fake success (return 0)
//       TWRP init calls these early in startup; as untrusted_app they
//       all return EPERM (or are seccomp-blocked with SIGSYS) and init
//       gives up with exit(1). We intercept them at syscall EXIT and
//       force the return value to 0 so init proceeds. They are ALSO
//       handled in the SIGSYS path in case some devices' seccomp filter
//       blocks them outright.
//       (ioprio_set was MISSING from this set until Task 5-S — see the
//       comment on `ioprio_set` in `ChildAbi` for the verified
//       per-ABI numbers and the dispatcher's misdiagnosis it corrected.)
//       (mount + rt_sigprocmask were MISSING from the EXIT-handler's
//       fake-success set until Task 5-T — see the doc on
//       `compute_exit_return_value` for the verified per-ABI numbers
//       and the dispatcher's i386-rt_sigprocmask-number misdiagnosis it
//       corrected. This was the REAL root cause of the UI E2E TWRP init
//       exit(1) at iter 189: mount(nr=21) returned 21 — the syscall
//       NUMBER, not 0 — four times in a row, then init exit(1)'d.)
//       (mknod was MISSING from this set until Task 5-X — see the doc
//       on `compute_exit_return_value` for the verified per-ABI
//       numbers. 5-T's mount fix advanced mount from "returns 21" to
//       "returns 0", surfacing mknod (i386 syscall 14) as the next
//       blocker per 5-W's VLM-verified UI E2E analysis. mknod was
//       ALSO given a rootfs-level empty-file stub in the SIGSYS
//       handler so the guest's subsequent open() of /dev/null etc.
//       succeeds — the only faked-success syscall with an fs op that
//       creates a non-directory file.)
//       NOTE: the original diagnostic log reported "fchown (nr=91)"
//       but nr=91 on x86_64 is actually fchmod (real fchown is 93).
//       We intercept BOTH — the field named `fchown` uses the correct
//       fchown numbers (93/95/55), and the field named `fchmod` uses
//       the correct fchmod numbers (91/94/52) which matches the
//       diagnostic's nr=91. Either way the bug is fixed.
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
    // stat64 / lstat64 / fstat64 — the i386 64-bit-struct variants of
    // stat/lstat/fstat. Modern i386 bionic (including TWRP's recovery)
    // uses stat64 (nr=195) + lstat64 (nr=196) INSTEAD of the old stat
    // (nr=106) + lstat (nr=107), because the old struct stat can't
    // represent large files/inodes. Task 6-T: these were MISSING from
    // ChildAbi, so the path-translation condition (which matches on
    // abi.stat || abi.lstat) did NOT cover stat64/lstat64 → the recovery's
    // stat64("/some/rootfs/path") checked the HOST filesystem (not the
    // rootfs) → ENOENT for rootfs-only files → infinite polling loop
    // (clock_gettime → stat64 → ENOENT → nanosleep → repeat ~3500× →
    // recovery gives up → wait4 → exit_group(1); observed on the 3a77faf
    // UI E2E run 32191877530 with the 5000-cap logging from Task 6-S).
    //
    // stat64 + lstat64 take a PATH (arg1) → need path translation (same
    // as stat/lstat). fstat64 takes an fd (arg1) → NO path translation,
    // but we add the field for syscall_name logging.
    //
    // Verified against /usr/include/x86_64-linux-gnu/asm/unistd_32.h:
    //   i386:   __NR_stat64 195, __NR_lstat64 196, __NR_fstat64 197
    //   x86_64: (no stat64/lstat64/fstat64 — stat/lstat/fstat are already 64-bit)
    //   aarch64: (asm-generic has no stat64/lstat64/fstat64 — uses statx/newfstatat)
    stat64: i64,
    lstat64: i64,
    fstat64: i64,
    access: i64,
    faccessat: i64,
    rt_sigprocmask: i64,
    readlink: i64,
    readlinkat: i64,
    chdir: i64,
    // unlink / unlinkat — path-taking file-deletion syscalls. Task 6-Y:
    // these were MISSING from ChildAbi, so the path-translation match
    // arm did NOT cover them → init's unlink("/dev/socket/property_service")
    // hit the HOST /dev/socket (not the rootfs) → EACCES → init logged
    // "Failed to unlink old socket 'property_service': Permission denied"
    // → init's property service failed to start → ALL property-setting
    // failed → "init: init startup failure" → exit_group(1).
    //
    // Verified against /usr/include/x86_64-linux-gnu/asm/unistd_32.h +
    // unistd_64.h + asm-generic/unistd.h:
    //   i386:   unlink=10, unlinkat=301
    //   x86_64: unlink=87, unlinkat=263
    //   aarch64 (asm-generic): unlink=-1 (dropped; uses unlinkat=35)
    // unlink takes a PATH (arg1) → needs path translation. unlinkat takes
    // a dirfd (arg1) + a PATH (arg2) → arg2 is the path (same as openat).
    unlink: i64,
    unlinkat: i64,
    // Syscalls that TWRP init calls early in startup which return EPERM
    // as untrusted_app (capget — no capabilities; fchown/fchmod — can't
    // change ownership/permissions of fds; ioprio_get / ioprio_set —
    // need CAP_SYS_ADMIN for some classes). Init sees the failures and
    // exits with code 1. We intercept these at syscall EXIT and force
    // the return value to 0 (success) so init proceeds. They are also
    // handled in the SIGSYS path (return 0, no rewrite to getpid — same
    // as rt_sigprocmask) in case some devices' seccomp filter blocks
    // them.
    //
    // NOTE on fchown vs fchmod: the diagnostic log that motivated this
    // fix reported "fchown (nr=91)" but nr=91 on x86_64 is actually
    // fchmod (real fchown is 93). The aarch64 number reported (55) IS
    // correct for fchown. We carry BOTH `fchown` (with the actually-
    // correct fchown numbers) AND `fchmod` (with the actually-correct
    // fchmod numbers, matching the diagnostic's nr=91) so the bug is
    // fixed regardless of which syscall init was really calling.
    //
    // NOTE on ioprio_set: this field was MISSING until Task 5-S — only
    // `ioprio_get` was present. TWRP init DOES call ioprio_set during
    // early boot (it sets the I/O priority of background threads /
    // services), and an EPERM there can trip init's "fatal config
    // error" path. The numbers per the kernel's own UAPI headers
    // (verified in Task 5-S against /usr/lib/linux/uapi/x86/asm/
    // unist_32.h, /usr/include/x86_64-linux-gnu/asm/unistd_64.h, and
    // /usr/include/asm-generic/unistd.h) are:
    //   i386:   ioprio_set=289, ioprio_get=290
    //   x86_64: ioprio_set=251, ioprio_get=252
    //   aarch64: ioprio_set=30,  ioprio_get=31
    // NOTE: the dispatcher's task spec for 5-S claimed i386
    // ioprio_set=252 / ioprio_get=251 — that was WRONG. 252 on i386 is
    // `exit_group` (NOT ioprio_set!), 251 is UNUSED in the i386 table
    // (the table jumps from fadvise64=250 straight to exit_group=252),
    // and 290 is `ioprio_get` (NOT epoll_create1 — epoll_create1 is
    // 329 on i386). Setting i386 ioprio_set=252 would have collided
    // with exit_group: every exit_group call would have been
    // mislabelled "ioprio_set" in the syscall_name() diagnostic AND
    // would have entered the fake-success branch (return Some(0)),
    // making future debugging of init's exit path much harder.
    fchown: i64,
    fchmod: i64,
    capget: i64,
    ioprio_get: i64,
    ioprio_set: i64,
    // ── chmod / lchown / chown / fchmodat / fchownat ─────────────
    //
    // These are the path-taking siblings of fchmod/fchown. TWRP's
    // init calls `chmod("/proc/cmdline", ...)` TWICE in a row right
    // before parsing /proc/cmdline (verified in the dbcac85 / 4-E
    // E2E log). If the chmod return value is not 0 (success), init's
    // error-handling path corrupts a pointer that leads to a SIGSEGV
    // at rip=0x809255d (NULL+0x90 deref) immediately after reading
    // /proc/cmdline.
    //
    // The kernel leaves rax holding the syscall number (15 on i386,
    // 90 on x86_64) at the syscall-EXIT stop when seccomp
    // SECCOMP_RET_TRAP fires on i386 compat — NOT -ENOSYS (-38) as
    // the kernel docs imply. Without forcing rax=0 here, init sees
    // `chmod returned 15`, takes the error path, and crashes. See the
    // "Decoded crash" block in the worklog entry for Task 5-A for the
    // full sequence.
    //
    // We force rax=0 for ALL of these siblings at syscall-EXIT (see
    // `compute_exit_return_value`). On i386 compat the fchmodat /
    // fchownat numbers are large (306 / 298); on x86_64 they're 268
    // / 260; on aarch64 only fchmodat (53) and fchownat (54) exist
    // (asm-generic has no plain `chmod` / `lchown` / `chown` —
    // bionic's `chmod(path, mode)` shim issues `fchmodat(AT_FDCWD,
    // path, mode, 0)`, and similarly for chown).
    //
    // Pre-existing data note: the existing ABI_AARCH64.chmod field is
    // set to 53 — that value IS fchmodat in asm-generic, so the
    // SIGSYS handler has always routed aarch64 chmod through the
    // fchmodat slot. We keep ABI_AARCH64.chmod=53 unchanged for
    // backwards compatibility and ALSO expose fchmodat=53 explicitly
    // so the EXIT handler matches both names. The net effect is that
    // syscall 53 on aarch64 is faked-success at EXIT (the desired
    // behaviour).
    chmod: i64,
    lchown: i64,
    chown: i64,
    fchmodat: i64,
    fchownat: i64,
    // execve syscall number for this ABI. Used to detect when the child
    // replaces its image (kr64 → TWRP init, or TWRP init → recovery), so
    // we can reset the lazily-detected ABI and re-read /proc/<pid>/exe
    // to pick up the new binary's bitness. Without this, the first
    // bitness detection (which runs at the FIRST syscall stop, BEFORE
    // execve) would permanently lock in the kr64 binary's bitness
    // (x86_64) even after the child exec's a 32-bit i386 init — causing
    // every subsequent syscall number and register index to be wrong.
    execve: i64,
    // Syscalls we never actually emulate, but whose numbers we need for
    // the SIGSYS diagnostic logging (we look up the original syscall
    // number to print a human-readable name when seccomp blocks it).
    //
    // NOTE: `mount`, `mkdir`, AND `mknod` ALSO get a real filesystem
    // operation performed in the rootfs by the SIGSYS handler's
    // "mount/mkdir/chmod/chroot/unshare" block (mkdir for mount/mkdir,
    // empty-file-creation for mknod — so the guest's subsequent open()
    // of the device node succeeds). chmod/chroot/unshare do NOT get an
    // fs op (they are pure fake-success). See the SIGSYS handler for
    // the per-syscall branches.
    //
    // NOTE on `mknod` (Task 5-X): the per-ABI numbers are:
    //   i386:   mknod = 14
    //   x86_64: mknod = 133
    //   aarch64: mknod = -1 (sentinel "not present on this ABI"; the
    //     asm-generic/unistd.h table dropped plain `mknod`, only
    //     `mknodat = 33` survives — bionic's mknod() libc wrapper on
    //     aarch64 issues mknodat(AT_FDCWD, ...) under the hood. A
    //     future aarch64-specific fix would need to add a dedicated
    //     mknodat field. This mirrors the existing pattern for
    //     ABI_AARCH64.open / access / lchown / chown, which are also
    //     set to -1 for the same "asm-generic dropped it" reason.)
    // Verified directly against /usr/include/x86_64-linux-gnu/asm/
    // unistd_32.h (__NR_mknod 14), unistd_64.h (__NR_mknod 133), and
    // /usr/include/asm-generic/unistd.h (no __NR_mknod — only
    // __NR_mknodat 33) in Task 5-X.
    mount: i64,
    chroot: i64,
    mkdir: i64,
    unshare: i64,
    // mknod(pathname, mode, dev) — TWRP init calls this for /dev/null,
    // /dev/zero, /dev/urandom etc. during early boot. As untrusted_app
    // it returns EPERM (no CAP_MKNOD), and init's fatal-config-error
    // path triggers exit(1) on non-zero non-EPERM return values. See
    // the doc on `compute_exit_return_value` for why mknod was added
    // in Task 5-X (the immediate next blocker after 5-T's mount fix).
    mknod: i64,
    // Extended-attribute SET syscalls (setxattr / lsetxattr / fsetxattr) —
    // TWRP recovery calls lsetxattr() to set security.selinux labels on
    // files during its SELinux-restorecon phase. As untrusted_app the
    // kernel returns -EPERM (no CAP_FSETID/CAP_DAC_OVERRIDE for
    // security.* xattrs), and recovery treats EPERM as retryable →
    // infinite spin → death → kr64 2s relaunch loop (Task 6-R;
    // observed on e04dab6 UI E2E run 32181613036 as syscalls #123 +
    // #135 both nr=227 -> -EPERM, with the parent relaunching every 2s
    // from 20:29:56 to 20:30:10).
    // We fake success (return 0) at syscall-EXIT — same pattern as
    // chmod/mknod/mount. SELinux labeling is not enforced in the
    // sandbox (non-fatal for TWRP boot).
    //
    // Verified against /usr/include/x86_64-linux-gnu/asm/unistd_32.h +
    // unistd_64.h:
    //   i386:   setxattr=226, lsetxattr=227, fsetxattr=228
    //   x86_64: setxattr=188, lsetxattr=189, fsetxattr=190
    //   aarch64 (upstream Linux kernel asm-generic, NOT this sandbox's
    //     non-standard /usr/include/asm-generic/unistd.h which wrongly
    //     lists 5/6/7): setxattr=188, lsetxattr=189, fsetxattr=190
    //     — matching x86_64. (This sandbox's asm-generic/unistd.h has
    //     setxattr=5/lsetxattr=6/fsetxattr=7, which are io_setup/
    //     io_destroy/io_getevents in upstream Linux — the sandbox
    //     header is non-standard. Real Android aarch64 bionic uses
    //     188/189/190.)
    setxattr: i64,
    lsetxattr: i64,
    fsetxattr: i64,
    // SysV shared-memory syscalls (shmget / shmat / shmctl). TWRP
    // init calls shmget() during early boot — Android's
    // __system_property_area_init uses a SysV shared memory segment
    // for the property file. The host's seccomp filter blocks these
    // (not in the untrusted_app allow list), raising SIGSYS.
    //
    // Returning 0 (fake success) for shmget causes init to loop
    // forever: shmid=0 is not a valid shmid, so init retries shmget
    // — observed in the 0a4be80 E2E run as a 13k-iteration SIGSYS
    // loop that OOM'd the Java FileLogger-Kr64Tee thread.
    //
    // Returning -ENOSYS (-38) tells init "this kernel does not
    // implement SysV shared memory", which makes bionic fall back to
    // a non-shared-memory property area initialization path (the
    // same fallback used on kernels built without CONFIG_SYSVIPC).
    // This is the correct emulation for an unprivileged rootless
    // container.
    shmget: i64,
    shmat: i64,
    shmctl: i64,
    // pause() — TWRP init's __system_property_area_init code calls
    // pause() in a loop while waiting for the property service to
    // signal that it has set up /dev/__properties__. On the host's
    // untrusted_app seccomp filter pause() (i386 syscall 29) is
    // blocked, raising SIGSYS. The kernel's own pause() can ONLY ever
    // return -EINTR (errno 4) — there is no "successful" return from
    // pause(); it blocks until interrupted by a signal.
    //
    // Returning 0 for pause() (the SIGSYS handler's pre-6-D default
    // "NOT rewriting orig_rax, returning 0" branch) is WRONG: init
    // interprets return value 0 as "pause completed WITHOUT a
    // signal" → re-checks its condition (property service still not
    // ready) → calls pause() again → INFINITE LOOP. This is the
    // post-6-C UI E2E blocker: the guest now loops on pause() (i386
    // syscall 29) 1,048,000+ times instead of looping on shmget.
    //
    // 6-D (commit 2b073f8) tried returning -EINTR (-4): this made
    // init think "interrupted by a signal" → check the condition
    // (property service not ready) → call pause() again → INFINITE
    // LOOP. The UI E2E test on 2b073f8 shows the pause loop is STILL
    // there (992,000+ repeats) — -EINTR did NOT break the loop. The
    // property service will NEVER signal readiness because kr64 has
    // NO property service (5-Y's find_property binary patch makes
    // lookups return NULL, but there's no actual service to send the
    // "ready" signal).
    //
    // Task 6-E: return -ENOSYS (-38) instead. This tells init "this
    // kernel does not implement pause()" → init falls back to a
    // non-pause wait mechanism (or skips the wait entirely). This
    // mirrors how 6-C's shmget -ENOSYS made init fall back to non-
    // shared-memory property init (which WORKED — the shmget loop
    // stopped). The same fallback pattern should break the pause
    // loop here.
    //
    // pause() is NOT added to compute_exit_return_value's fake-
    // success list — it returns -ENOSYS, not 0, via a dedicated branch
    // in the SIGSYS handler. Historically (6-C) this meant
    // `should_skip_sigsys_setregs` did NOT skip the SIGSYS handler's
    // setregs for pause (the skip fired only for syscalls in the fake-
    // success list — pause isn't in it) → the SIGSYS handler's setregs
    // fired to write -ENOSYS. Under 6-W, `should_skip_sigsys_setregs`
    // ALWAYS returns false (never skip) — so the setregs fires for
    // pause unconditionally now (see the function's doc comment for
    // the 5-J → 6-C → 6-W evolution).
    //
    // The per-ABI numbers (verified against the kernel's UAPI headers
    // in Task 6-D):
    //   i386:   pause = 29   (per asm/unistd_32.h: __NR_pause 29)
    //   x86_64: pause = 34   (per asm/unistd_64.h: __NR_pause 34)
    //   aarch64: pause = -1  (SENTINEL — no __NR_pause in
    //     asm-generic/unistd.h; pause was REMOVED in the asm-generic
    //     table — aarch64 callers use ppoll/nanosleep instead.
    //     bionic's pause() libc wrapper on aarch64 issues ppoll(NULL,
    //     0, NULL, NULL) under the hood — a future aarch64-specific
    //     fix would need a dedicated ppoll field. Mirrors the existing
    //     pattern for ABI_AARCH64.open / access / lchown / chown /
    //     mknod, which are also set to -1 for the same reason. The host
    //     is x86_64 running an i386 child, so this aarch64 path is
    //     currently dead code at runtime — the sentinel keeps the
    //     compile happy and documents the aarch64 behaviour.)
    pause: i64,
    // Task 6-S: fork / clone / vfork / wait4 / exit_group syscall numbers.
    // Used ONLY by the dedicated always-log diagnostic blocks in the ENTRY
    // + EXIT handlers of `run_ptrace_loop` (search for "Task-6-S ENTRY"
    // + "Task-6-S EXIT" log strings). These 5 syscalls are the critical
    // signals for diagnosing the post-6-R recovery exit(1):
    //   - The recovery (post-6-R's lsetxattr fix) runs 3281 iterations
    //     before exit(1), but the post-execve logging was capped at 150
    //     — hiding the middle phase (iters 151-3271) where fork/clone
    //     attempts + the exit trigger live.
    //   - The bceac63 diagnostic showed ZERO fork/clone/vfork calls in
    //     the entire visible logcat (neither i386 nr=2/120/190 nor x86_64
    //     nr=56/57/58). The recovery's last-10 ALL syscalls include
    //     wait4 (init waiting for a child) and exit_group. The child
    //     is absent or dead → wait4 likely returns -ECHILD → recovery
    //     exit_group(1). But we cannot tell whether recovery attempts
    //     fork/clone in the middle phase because the cap hid it.
    //   - The dedicated always-log block (NOT gated by the 5000 cap) for
    //     these 5 syscalls ensures we ALWAYS see them, even past iter
    //     5000 — so we can determine whether the guest ever creates
    //     children + what wait4 returns.
    //
    // Verified directly against the kernel's UAPI headers in Task 6-S
    // (NOT just taken from the task spec):
    //   i386 (per /usr/include/x86_64-linux-gnu/asm/unistd_32.h):
    //     __NR_fork 2, __NR_clone 120, __NR_vfork 190,
    //     __NR_wait4 114, __NR_exit_group 252.
    //   x86_64 (per /usr/include/x86_64-linux-gnu/asm/unistd_64.h):
    //     __NR_fork 57, __NR_clone 56, __NR_vfork 58,
    //     __NR_wait4 61, __NR_exit_group 231.
    //   aarch64 (per /usr/include/asm-generic/unistd.h):
    //     __NR_clone 220, __NR_wait4 260, __NR_exit_group 94.
    //     NOTE: __NR_exit_group is 94 (NOT 93 — 93 is plain `exit`,
    //     which exits just the calling thread; exit_group exits all
    //     threads in the process and is what bionic's exit_group() /
    //     _Exit() wrappers call). The task spec for 6-S said aarch64
    //     exit_group=93 — that was WRONG. Verified: 94.
    //     aarch64 (asm-generic) DROPPED plain `fork` and `vfork` —
    //     bionic's fork() libc wrapper on aarch64 issues clone() under
    //     the hood. fork_nr=-1 + vfork_nr=-1 (sentinels "not present
    //     on this ABI"). Mirrors the existing pattern for
    //     ABI_AARCH64.open / access / lchown / chown / mknod / pause.
    clone_nr: i64,
    fork_nr: i64,
    vfork_nr: i64,
    wait4_nr: i64,
    exit_group_nr: i64,
    // write(fd, buf, count) — NOT intercepted or emulated; carried
    // purely so the syscall-EXIT diagnostic (Task 6-U) can recognise
    // a write() return + capture the buffer contents. TWRP init writes
    // its KLOG via write(fd=3, "<N>init: ...\n", len) to /dev/__kmsg__.
    // kr64 copies that file to /sdcard but the test harness never
    // `adb pull`s it, so init's own diagnostic messages (WHY it bails
    // before parsing init.rc) are stranded. The 6-U diagnostic captures
    // the buffer contents inline in the logcat so the KLOG is visible
    // without pulling /sdcard. Per-ABI numbers (verified against the
    // kernel's UAPI headers — same source as `pause`):
    //   i386:   write = 4   (per asm/unistd_32.h: __NR_write 4)
    //   x86_64: write = 1   (per asm/unistd_64.h: __NR_write 1)
    //   aarch64: write = 64 (per asm-generic/unistd.h: __NR_write 64)
    // The host is x86_64 running an i386 child, so the i386 number (4)
    // is the one that fires at runtime; the x86_64/aarch64 numbers are
    // locked in for ABI completeness + so the EXIT handler's
    // `== abi.write` comparison is correct if a future x86_64/aarch64
    // guest is ever supported. Crucially the comparison is
    // `syscall_num == abi.write` (NOT `matches!(syscall_num, 4 | 1)`)
    // to avoid cross-ABI confusion: x86_64 nr=4 is `stat` and i386
    // nr=1 is `exit`, so a naive `4 | 1` match would fire spuriously
    // on the wrong ABI.
    write: i64,
    // read(fd, buf, count) — NOT intercepted or emulated; carried
    // purely so the syscall-EXIT diagnostic (Task 6-V) can recognise
    // a read() return + capture the buffer contents. TWRP recovery reads
    // two files shortly before SIGSEGV (72 + 90 bytes); this diagnostic
    // surfaces WHAT was read so we can identify the files. Per-ABI numbers
    // (verified against the kernel's UAPI headers — same source as `write`):
    //   i386:   read = 3   (per asm/unistd_32.h: __NR_read 3)
    //   x86_64: read = 0   (per asm/unistd_64.h: __NR_read 0)
    //   aarch64: read = 63  (per asm-generic/unistd.h: __NR_read 63)
    // The host is x86_64 running an i386 child, so the i386 number (3)
    // is the one that fires at runtime. The ABI-aware comparison
    // `syscall_num == abi.read` avoids cross-ABI confusion (x86_64 nr=3
    // is `close`, i386 nr=0 is `restart_syscall`).
    read: i64,
    // mmap / mmap2 — Task 6-Y. TWRP init (i386) calls
    //   mmap2(NULL, 0x20000, PROT_READ|PROT_WRITE, MAP_SHARED, fd, 0)
    // on /dev/__properties__ to set up the property area. This returns
    // -ENOSYS (-38): the Android zygote's seccomp filter (inherited by
    // untrusted_app, can't be removed) blocks file-backed MAP_SHARED
    // mmap2 for i386 compat syscalls. kr64's OWN seccomp is already
    // skipped (install_seccomp=false, Task 6-X) — the zygote filter is
    // the blocker. Anonymous mmap2 SUCCEEDS; only file-backed MAP_SHARED
    // fails. Result: property area not mapped → all 383
    // __system_property_set calls fail → init bails at iter 927 →
    // exit(1). TWRP never boots.
    //
    // Fix: in the syscall-ENTRY handler, intercept mmap (x86_64 nr=9,
    // aarch64 nr=222) and mmap2 (i386 nr=192). When the mmap is file-
    // backed (MAP_SHARED, fd >= 0) AND the fd was opened as
    // /dev/__properties__ (tracked via the loop-local `properties_fd`
    // from open() EXIT), rewrite the mmap args to be anonymous:
    //   flags: (flags & !MAP_SHARED) | MAP_ANONYMOUS | MAP_PRIVATE
    //   fd:    -1
    //   offset: 0
    // The kernel then performs an anonymous mmap (which succeeds).
    // init's property_init writes the property area header to this
    // anonymous region. Since init is the only process in the sandbox
    // (no fork), the lack of file-backing/sharing is fine.
    //
    // Per-ABI numbers (verified against the kernel's UAPI headers in
    // Task 6-Y):
    //   i386:   mmap2 = 192 (per asm/unistd_32.h: __NR_mmap2 192)
    //           mmap  =  90 (per asm/unistd_32.h: __NR_mmap 90 — but
    //           modern i386 bionic uses mmap2 EXCLUSIVELY; this is set
    //           to -1 in ABI_X86_32 to mark it "not used at runtime"
    //           so the ENTRY match arm only fires for mmap2=192).
    //   x86_64: mmap  =   9 (per asm/unistd_64.h: __NR_mmap 9)
    //           mmap2 =  -1 (x86_64 has no mmap2 — sentinel)
    //   aarch64: mmap = 222 (per asm-generic/unistd.h: __NR_mmap 222)
    //           mmap2 =  -1 (asm-generic has no mmap2 — sentinel)
    // The host is x86_64 running an i386 child, so ABI_X86_32.mmap2
    // (192) is the value that fires at runtime. ABI_X86_64.mmap (9) and
    // ABI_AARCH64.mmap (222) are locked in for ABI completeness + so
    // the ENTRY match arm works correctly if a future x86_64/aarch64
    // guest is ever supported.
    //
    // NOTE on the -1 sentinels: when abi.mmap or abi.mmap2 is -1, the
    // ENTRY match arm `n if n == abi.mmap || n == abi.mmap2` reduces to
    // `n if n == <other>`, since real syscall numbers are never -1.
    // This is the SAME pattern ABI_AARCH64 uses for `open`/`access`/
    // `lchown`/`chown`/`mknod`/`pause` (all -1 on aarch64).
    mmap: i64,
    mmap2: i64,
    // socketcall — Task 6-Z3. i386 multiplexed socket syscall
    // (nr=102). arg1 = sub-call number (1=socket, 2=bind, 3=connect,
    // 4=listen, 5=accept, ...); arg2 = pointer to an array of the
    // sub-call's args (NOT the args themselves — the args are read
    // indirectly via the pointer). x86_64 + aarch64 have NO
    // socketcall — those ABIs use the direct socket/bind/listen/
    // connect/accept syscalls (numbers 41/49/50/42/43 on x86_64,
    // 198/200/201/202/204 on aarch64). Set to -1 (sentinel) on
    // x86_64 + aarch64, mirroring the existing ABI_AARCH64.open /
    // .access / .lchown / .chown / .mknod / .pause precedent.
    //
    // ROOT CAUSE — why this field is needed: TWRP init (an i386
    // binary) opens a Unix socket + binds it to the path
    // /dev/socket/property_service during its property service
    // startup. On the b492c65 UI E2E (run 32212585042), this bind
    // returned -98 (EADDRINUSE — "Address already in use") even
    // though the preceding unlink("/dev/socket/property_service")
    // returned -2 (ENOENT — file doesn't exist). The unlink removes
    // the FILESYSTEM ENTRY, but the socket itself is still bound in
    // the kernel because a STALE socket fd from a PREVIOUS relaunch
    // cycle is still open in the parent (the twoyi app) — the fd was
    // inherited by kr64 from the twoyi app via fork, and the parent
    // does NOT close it before forking the guest. The child's close
    // loop (fds 3..1024) closes the CHILD's fds, but not the parent's.
    // The parent's stale fd keeps the address bound → bind returns
    // EADDRINUSE → "Failed to bind socket 'property_service': Address
    // already in use" → "init: init startup failure" → exit_group(1).
    //
    // FIX — the SIMPLEST pragmatic approach: at the syscall-EXIT
    // stop, if `syscall_num == abi.socketcall_nr` AND the return
    // value is negative (error), fake the return to 0 (success).
    // This catches:
    //   - bind EADDRINUSE (-98) — the immediate blocker (init stops
    //     calling bind a failure + proceeds).
    //   - listen failure (if bind was faked, the socket isn't really
    //     bound, so listen would fail too — we fake it).
    //   - any other failing socketcall sub-call (connect/accept/...).
    // It does NOT fake socket() itself: socket() returns a positive
    // fd on success (which is the normal case — the zygote allows
    // it), and only negative values trigger the fake. The property
    // service doesn't actually need to WORK in the sandbox — init
    // just needs to THINK it started (the property AREA is mapped
    // separately via the mmap2 REWRITE from Task 6-Y, and ro.*
    // properties are written directly to the area by init's
    // property_init, NOT via the socket). Non-ro property SET goes
    // via the socket, but TWRP doesn't depend on non-ro sets for
    // boot — ro.* (set directly to the area) is sufficient.
    //
    // Verified against /usr/include/x86_64-linux-gnu/asm/unistd_32.h:
    //   #define __NR_socketcall 102
    // (only present on i386 — x86_64 and asm-generic have no
    // socketcall). Verified directly in Task 6-Z3.
    socketcall_nr: i64,
    // poll — Task 6-Z5. The legacy `poll` syscall (single syscall, not
    // ppoll). TWRP's recovery (an i386 binary) calls poll() in its
    // property_service startup loop after the bind has been faked to
    // 0 (Task 6-Z3). The faked bind hid the EADDRINUSE but the socket
    // is NOT actually bound (concurrent kr64 invocations — the twoyi
    // app relaunches kr64 every 2s without killing the old one → the
    // old init's socket is still bound → new init's bind fails → faked
    // to 0 → the socket isn't bound → poll returns POLLERR=1 every
    // call → busy-wait). Verified on a76b677 UI E2E run 32218145762:
    // poll (i386 nr=168) × 101 at syscalls #4900-5000, each returns 1
    // (a fd is ALWAYS ready with POLLERR). The recovery is alive but
    // stuck in a TIGHT POLL SPIN — it never proceeds to open
    // /dev/graphics/fb0 (no framebuffer render, no TWRP UI).
    //
    // FIX (PRAGMATIC): at the syscall-EXIT stop, if
    // syscall_num == abi.poll_nr AND the return value is POSITIVE
    // (N fds ready), fake the return to 0 (no fds ready — equivalent
    // to a timeout). This stops the busy-wait: the recovery thinks
    // no events are pending + either sleeps (if the poll timeout is
    // non-zero) or retries less aggressively.
    //
    // CAVEAT: the TWRP main UI loop ALSO uses poll (for input events).
    // Faking ALL poll returns to 0 would prevent the TWRP UI from
    // processing input. BUT the recovery is currently stuck BEFORE
    // the TWRP UI loop (in a setup/property-service poll spin).
    // Faking poll to 0 should let it proceed PAST the setup loop.
    // Once the recovery reaches the framebuffer phase, the poll
    // behaviour might differ. If this fix causes a regression (the
    // TWRP UI can't process input), a follow-up fix can make the fake
    // conditional (only during the setup phase, not the main loop).
    //
    // Per-ABI numbers (verified against /usr/include/x86_64-linux-gnu/
    // asm/unistd_32.h + unistd_64.h + asm-generic/unistd.h in 6-Z5):
    //   i386:   poll = 168 (per asm/unistd_32.h: __NR_poll 168 —
    //     THIS is the value that fires at runtime; TWRP recovery is
    //     an i386 binary that issues poll() as i386 syscall 168).
    //   x86_64: poll =   7 (per asm/unistd_64.h: __NR_poll 7 — does
    //     NOT currently fire at runtime; the host runs an i386 child.
    //     Locked in for ABI completeness + so the EXIT handler's
    //     `== abi.poll_nr` comparison is correct if a future x86_64
    //     guest is ever supported).
    //   aarch64: poll = -1 (SENTINEL — asm-generic/unistd.h has NO
    //     __NR_poll; aarch64 callers use ppoll/nanosleep instead.
    //     bionic's poll() libc wrapper on aarch64 issues ppoll under
    //     the hood. A future aarch64-specific fix would need a
    //     dedicated ppoll field. Mirrors the existing pattern for
    //     ABI_AARCH64.open / access / lchown / chown / mknod / pause
    //     / socketcall, which are all set to -1 for the same "asm-
    //     generic dropped it" reason.)
    poll_nr: i64,
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
    // Task 6-Y: arg5 + arg6 register indices — used ONLY by the mmap /
    // mmap2 ENTRY handler to read the fd (arg5) + offset (arg6) args so
    // we can rewrite them when the mmap is file-backed MAP_SHARED on
    // /dev/__properties__. No other syscall in this file uses 6+ args
    // (open/openat use 3-4, stat/lstat use 2, etc.), so these are
    // dead-code-allowed everywhere except the mmap ENTRY arm.
    #[allow(dead_code)]
    reg_arg5: usize,
    #[allow(dead_code)]
    reg_arg6: usize,
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
    // x86_64 stat/lstat/fstat are already 64-bit (struct stat carries
    // 64-bit st_size + st_ino); there are NO stat64/lstat64/fstat64
    // syscall numbers on x86_64 (verified: /usr/include/x86_64-linux-gnu/
    // asm/unistd_64.h has NO __NR_stat64 / __NR_lstat64 / __NR_fstat64
    // entries — they were a 32-bit-only workaround for the old small-
    // field struct stat). Set to -1 (sentinels "not present on this
    // ABI"), mirroring the existing ABI_AARCH64.open / ABI_X86_32.stat64
    // precedent. Task 6-T.
    stat64: -1,
    lstat64: -1,
    fstat64: -1,
    access: 21,
    faccessat: 48,
    rt_sigprocmask: 14,
    readlink: 89,
    readlinkat: 267,
    chdir: 80,
    // x86_64 unlink=87, unlinkat=263 (Task 6-Y; verified against
    // /usr/include/x86_64-linux-gnu/asm/unistd_64.h). Path-translated so
    // init's unlink("/dev/socket/property_service") hits the rootfs.
    unlink: 87,
    unlinkat: 263,
    // TWRP-init EPERM workaround — see the long comment on these
    // fields in `ChildAbi`. Real fchown on x86_64 is 93 (NOT 91, which
    // is fchmod — the diagnostic log that motivated this fix reported
    // "fchown (nr=91)" but nr=91 is actually fchmod; we carry both
    // `fchown` and `fchmod` so the fix works either way).
    fchown: 93,
    fchmod: 91,
    capget: 125,
    ioprio_get: 252,
    // ioprio_set on x86_64 = 251 (per asm/unistd_64.h, verified in
    // Task 5-S against /usr/include/x86_64-linux-gnu/asm/unistd_64.h).
    // Pre-5-S this field was MISSING — added defensively because TWRP
    // init DOES call ioprio_set during early boot and an EPERM there
    // can trip init's fatal-config-error path.
    ioprio_set: 251,
    // chmod / lchown / chown / fchmodat / fchownat (x86_64 numbers
    // per asm/unistd_64.h):
    //   chmod=90, lchown=94, chown=182, fchmodat=268, fchownat=260.
    // NOTE: 257 is `openat` on x86_64 — Task 5-A's commit ee93ac0 had a
    //   1-char typo here (`fchownat: 257`) which made every openat()
    //   call fake-success returning stdin fd (0); fixed by Task 5-H.
    // We ALSO fake success (return 0) for these at syscall-EXIT —
    // see the comment on `chmod` in `ChildAbi` for why.
    chmod: 90,
    lchown: 94,
    chown: 182,
    fchmodat: 268,
    fchownat: 260,
    execve: 59, // SYS_execve (x86_64)
    mount: 165,
    chroot: 161,
    mkdir: 83,
    unshare: 272,
    // x86_64 mknod = 133 (per /usr/include/x86_64-linux-gnu/asm/
    // unistd_64.h, verified directly against the kernel's UAPI
    // header in Task 5-X). Pre-5-X this field was MISSING — added
    // defensively because TWRP init DOES call mknod for /dev/null,
    // /dev/zero, /dev/urandom during early boot and an EPERM there
    // can trip init's fatal-config-error path. TWRP's init binary is
    // i386, so this x86_64 number doesn't currently fire at runtime,
    // but the EXIT handler's if-chain is ABI-aware so we lock the
    // x86_64 number in too (cheap insurance).
    mknod: 133,
    // x86_64 setxattr / lsetxattr / fsetxattr (per /usr/include/x86_64-
    // linux-gnu/asm/unistd_64.h: __NR_setxattr 188, __NR_lsetxattr 189,
    // __NR_fsetxattr 190, verified directly against the kernel's UAPI
    // header in Task 6-R). The host is x86_64 running an i386 child,
    // so these x86_64 numbers do NOT currently fire at runtime (the
    // guest uses i386 syscall 226/227/228). Locked in for ABI
    // completeness + to keep the EXIT handler's ABI-aware if-chain
    // correct if a future x86_64 guest is ever supported. See the doc
    // on these fields in `ChildAbi` for the TWRP-restorecon EPERM
    // retry-loop blocker (Task 6-R).
    setxattr: 188,
    lsetxattr: 189,
    fsetxattr: 190,
    // SysV shared-memory syscalls — see the comment on these fields
    // in `ChildAbi`. x86_64: shmget=29, shmat=30, shmctl=31
    // (asm/unistd_64.h).
    shmget: 29,
    shmat: 30,
    shmctl: 31,
    // x86_64 pause = 34 (per /usr/include/x86_64-linux-gnu/asm/unistd_64.h:
    // __NR_pause 34, verified directly against the kernel's UAPI header
    // in Task 6-D). The host is x86_64 running an i386 child, so this
    // x86_64 number does NOT currently fire at runtime (the guest uses
    // i386 syscall 29 for pause). It is locked in for ABI completeness
    // and to keep the EXIT handler's ABI-aware if-chain correct if a
    // future x86_64 guest is ever supported. See the doc on `pause` in
    // `ChildAbi` for why we return -ENOSYS (not 0, not -EINTR) for
    // pause (Task 6-E: -ENOSYS makes init fall back to a non-pause
    // wait instead of looping on -EINTR + re-checking the never-ready
    // property service).
    pause: 34,
    // Task 6-S: fork / clone / vfork / wait4 / exit_group syscall numbers
    // (per /usr/include/x86_64-linux-gnu/asm/unistd_64.h, verified
    // directly against the kernel's UAPI header in Task 6-S). Used by the
    // dedicated always-log ENTRY/EXIT diagnostic block in run_ptrace_loop
    // for these 5 critical syscalls (NOT gated by the 5000 post-execve
    // cap, so we never miss them even past iter 5000). The host is x86_64
    // running an i386 child, so these x86_64 numbers do NOT currently
    // fire at runtime (the guest uses i386 syscall 2/120/190/114/252).
    // Locked in for ABI completeness + so the always-log block compiles
    // + works correctly if a future x86_64 guest is ever supported
    // (mirrors the mknod: 133 / setxattr: 188 / pause: 34 precedent).
    clone_nr: 56,
    fork_nr: 57,
    vfork_nr: 58,
    wait4_nr: 61,
    exit_group_nr: 231,
    // x86_64 write = 1 (per /usr/include/x86_64-linux-gnu/asm/unistd_64.h:
    // __NR_write 1). NOT intercepted — carried so the syscall-EXIT
    // diagnostic (Task 6-U) can match `syscall_num == abi.write` for
    // write() return-value + buffer capture. See the doc on `write` in
    // `ChildAbi` for why the comparison must be ABI-aware (i386 nr=1 is
    // `exit`, so a naive `matches!(syscall_num, 1 | 4)` would misfire
    // cross-ABI). The host is x86_64 running an i386 child, so this
    // x86_64 number does NOT currently fire at runtime (the guest uses
    // i386 syscall 4). Locked in for ABI completeness.
    write: 1,
    // x86_64 read = 0 (per asm/unistd_64.h: __NR_read 0).
    // NOT intercepted — carried for ABI completeness so the 6-V
    // syscall-EXIT diagnostic's `syscall_num == abi.read` comparison
    // is correct if a future x86_64 guest is ever supported. The host
    // is x86_64 running an i386 child, so this number does NOT
    // currently fire at runtime.
    read: 0,
    // x86_64 mmap = 9 (per /usr/include/x86_64-linux-gnu/asm/unistd_64.h:
    // __NR_mmap 9, verified directly against the kernel's UAPI header
    // in Task 6-Y). x86_64 has NO mmap2 (the modern x86_64 mmap takes
    // offset in BYTES directly, not in 4096-byte pages — there is no
    // need for the mmap2 page-shift workaround that i386 needs because
    // i386's orig_eax is only 32 bits and cannot pass a 64-bit byte
    // offset). The host is x86_64 running an i386 child, so this x86_64
    // number does NOT currently fire at runtime (the guest uses i386
    // syscall 192). Locked in for ABI completeness + so the mmap ENTRY
    // handler's `== abi.mmap || == abi.mmap2` comparison is correct if
    // a future x86_64 guest is ever supported. See the doc on `mmap` /
    // `mmap2` in `ChildAbi` for why we rewrite file-backed MAP_SHARED
    // mmap of /dev/__properties__ to anonymous (Task 6-Y).
    mmap: 9,
    mmap2: -1, // x86_64 has no mmap2 — sentinel.
    // x86_64 socketcall = -1 (SENTINEL — x86_64 has no socketcall;
    // x86_64 uses the direct socket/bind/listen/connect/accept
    // syscalls instead, per /usr/include/x86_64-linux-gnu/asm/
    // unistd_64.h: __NR_socket 41, __NR_bind 49, __NR_listen 50,
    // __NR_connect 42, __NR_accept 43). The host is x86_64 running
    // an i386 child, so this x86_64 number does NOT currently fire at
    // runtime (the guest uses i386 syscall 102). Locked in for ABI
    // completeness + so the EXIT handler's
    // `== abi.socketcall_nr` comparison is correct if a future x86_64
    // guest is ever supported. Mirrors the existing
    // ABI_X86_64.mmap2 = -1 precedent. Task 6-Z3.
    socketcall_nr: -1,
    // x86_64 poll = 7 (per /usr/include/x86_64-linux-gnu/asm/unistd_64.h:
    // __NR_poll 7, verified directly against the kernel's UAPI header
    // in Task 6-Z5). The host is x86_64 running an i386 child, so this
    // x86_64 number does NOT currently fire at runtime (the guest uses
    // i386 syscall 168). Locked in for ABI completeness + so the EXIT
    // handler's `== abi.poll_nr` comparison is correct if a future
    // x86_64 guest is ever supported. See the doc on `poll_nr` in
    // `ChildAbi` for the full root-cause analysis (Task 6-Z5).
    poll_nr: 7,
    reg_syscall: 15, // orig_rax
    reg_ret: 10,     // rax
    reg_arg1: 14,    // rdi
    reg_arg2: 13,    // rsi
    reg_arg3: 12,    // rdx
    reg_arg4: 7,     // r10
    // Task 6-Y: arg5 + arg6 register indices — used by the mmap ENTRY
    // handler to read the fd (arg5=r8) + offset (arg6=r9) args so we
    // can rewrite them when the mmap is file-backed MAP_SHARED on
    // /dev/__properties__. x86_64 user_regs_struct field order:
    //   0:r15 1:r14 2:r13 3:r12 4:rbp 5:rbx 6:r11 7:r10 8:r9 9:r8
    //   10:rax 11:rcx 12:rdx 13:rsi 14:rdi 15:orig_rax ...
    // so r8=9, r9=8.
    #[allow(dead_code)]
    reg_arg5: 9, // r8
    #[allow(dead_code)]
    reg_arg6: 8, // r9
    reg_sp: 19, // rsp
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
    // Task 6-T: i386 stat64/lstat64/fstat64. Verified directly against
    // /usr/include/x86_64-linux-gnu/asm/unistd_32.h:
    //   __NR_stat64  195
    //   __NR_lstat64 196
    //   __NR_fstat64 197
    // THESE are the values that ACTUALLY fire at runtime — modern i386
    // bionic (incl. TWRP's recovery) uses stat64/lstat64 INSTEAD of the
    // old stat(106)/lstat(107) because the old struct stat can't carry
    // 64-bit st_size/st_ino. Pre-6-T, the path-translation condition
    // (which matched abi.stat || abi.lstat) did NOT cover stat64/lstat64
    // → the recovery's stat64("/some/rootfs/path") hit the HOST fs
    // (where rootfs files don't exist) → ENOENT → infinite polling loop
    // (clock_gettime → stat64 → ENOENT → nanosleep → repeat ~3500×) →
    // recovery gives up → wait4 → exit_group(1). Observed on 3a77faf UI
    // E2E run 32191877530 (5000-cap logging from Task 6-S revealed the
    // full polling loop: post-execve syscalls #294+ all nr=195 → -2
    // ENOENT, repeating with nr=265 clock_gettime + nr=162 nanosleep).
    // fstat64(197) takes an fd (NOT a path) → no path translation needed
    // but added to ChildAbi for syscall_name diagnostic logging.
    stat64: 195,
    lstat64: 196,
    fstat64: 197,
    access: 33,
    faccessat: 307,
    // i386 rt_sigprocmask = 175 (per /usr/include/x86_64-linux-gnu/
    // asm/unistd_32.h, verified directly against the kernel's UAPI
    // header in Task 5-T). Pre-5-T this was 14 — WRONG: i386 syscall 14
    // is `mknod`, NOT `rt_sigprocmask`. The kr64 SIGSYS-handler
    // diagnostic was therefore mislabelling every mknod SIGSYS as
    // "rt_sigprocmask() nr=14" — see the worklog entry for Task 5-T
    // for the full root-cause analysis. The x86_64 number (14 below)
    // IS correctly rt_sigprocmask on x86_64; this correction is i386-
    // only because the i386 and x86_64 syscall tables diverge here.
    rt_sigprocmask: 175,
    readlink: 85,
    readlinkat: 303,
    chdir: 12,
    // i386 unlink=10, unlinkat=301 (Task 6-Y; verified against
    // /usr/include/x86_64-linux-gnu/asm/unistd_32.h). Path-translated so
    // init's unlink("/dev/socket/property_service") hits the rootfs, not
    // the HOST /dev/socket (which gave EACCES → init startup failure).
    unlink: 10,
    unlinkat: 301,
    // TWRP-init EPERM workaround — see the long comment on these
    // fields in `ChildAbi`. i386 fchown=95, fchmod=94, capget=184,
    // ioprio_get=290, ioprio_set=289 (per /usr/lib/linux/uapi/x86/
    // asm/unistd_32.h — verified directly against the kernel's UAPI
    // headers in Task 5-S; the dispatcher's task spec for 5-S claimed
    // ioprio_get should be 251 / ioprio_set should be 252, which was
    // WRONG: 251 is UNUSED in the i386 table and 252 is `exit_group`).
    fchown: 95,
    fchmod: 94,
    capget: 184,
    ioprio_get: 290,
    ioprio_set: 289,
    // chmod / lchown / chown / fchmodat / fchownat (i386 numbers
    // per asm-i386/unistd_32.h):
    //   chmod=15, lchown=16, chown=182, fchmodat=306, fchownat=298.
    //
    // CRITICAL: i386 lchown is 16, NOT 94 (94 is fchmod on i386).
    // The task spec mistakenly listed lchown=96 for i386 — verified
    // against asm-i386/unistd_32.h: the real value is 16.
    //
    // We ALSO fake success (return 0) for these at syscall-EXIT —
    // see the comment on `chmod` in `ChildAbi` for why.
    chmod: 15,
    lchown: 16,
    chown: 182,
    fchmodat: 306,
    fchownat: 298,
    execve: 11, // SYS_execve (i386)
    mount: 21,
    chroot: 61,
    mkdir: 39,
    unshare: 310,
    // i386 mknod = 14 (per /usr/include/x86_64-linux-gnu/asm/
    // unistd_32.h: __NR_mknod 14, verified directly against the
    // kernel's UAPI header in Task 5-X). Pre-5-X this field was
    // MISSING — added because TWRP init calls mknod() for /dev/null,
    // /dev/zero, /dev/urandom during early boot and the kernel's
    // syscall-number-leak value (rax = 14) at the EXIT stop caused
    // init's fatal-config-error path to fire exit(1) at iter 189.
    //
    // This is the IMMEDIATE NEXT BLOCKER after 5-T's mount fix — see
    // the worklog entry for 5-W (VLM-verified analysis) and 5-X
    // (this fix). 5-T's i386 rt_sigprocmask number correction
    // (14 → 175) cleared the way for this: pre-5-T, the SIGSYS
    // handler was matching syscall 14 against ABI_X86_32.
    // rt_sigprocmask=14 (WRONG) and mislabelling it; post-5-T the
    // diagnostic label correctly says "[unknown]" for syscall 14
    // — and post-5-X it correctly says "mknod" (this addition).
    mknod: 14,
    // i386 setxattr / lsetxattr / fsetxattr (per /usr/include/x86_64-
    // linux-gnu/asm/unistd_32.h: __NR_setxattr 226, __NR_lsetxattr 227,
    // __NR_fsetxattr 228, verified directly against the kernel's UAPI
    // header in Task 6-R). THIS is the value that fires at runtime —
    // TWRP recovery (an i386 binary) issues lsetxattr(path,
    // "security.selinux", ctx, 44, 0) during its SELinux-restorecon
    // phase. As untrusted_app the kernel returns -EPERM (no CAP for
    // security.* xattrs), and recovery treats EPERM as retryable →
    // infinite spin → death → kr64 2s relaunch loop (observed on
    // e04dab6 UI E2E run 32181613036 as syscalls #123 + #135 both
    // nr=227 -> -EPERM). See the doc on these fields in `ChildAbi`
    // (Task 6-R) for the full root-cause analysis.
    setxattr: 226,
    lsetxattr: 227,
    fsetxattr: 228,
    // SysV shared-memory syscalls — see the comment on these fields
    // in `ChildAbi`. i386 (verified against
    // /usr/include/x86_64-linux-gnu/asm/unistd_32.h in Task 6-C):
    //   __NR_shmget = 395
    //   __NR_shmat  = 397
    //   __NR_shmctl = 396
    // NOTE: the order is shmget=395, shmctl=396, shmat=397 — shmat is
    // 397, NOT 396. The previous values (29/30/31) were copy-pasted
    // from ABI_X86_64 and were WRONG: i386 syscall 29 is `pause` (not
    // shmget), 30 is `utime` (not shmat), 31 is `stty` (not shmctl).
    // That copy-paste caused the post-e6d85e1 UI E2E blocker where
    // init's real shmget() calls (nr=395) were never intercepted by
    // the SIGSYS handler, while `pause()` (nr=29) was misidentified
    // as shmget and had -ENOSYS returned — yielding an infinite
    // shmget-retry loop (790k+ calls/sec). Task 6-C.
    shmget: 395,
    shmat: 397,
    shmctl: 396,
    // i386 pause = 29 (per /usr/include/x86_64-linux-gnu/asm/unistd_32.h:
    // __NR_pause 29, verified directly against the kernel's UAPI header
    // in Task 6-D). This is the value the guest's TWRP init uses when
    // its __system_property_area_init code calls pause() in a loop
    // waiting for the property service to signal readiness. Pre-6-D
    // the SIGSYS handler returned 0 for pause (the default
    // "NOT rewriting orig_rax, returning 0" branch) → init interpreted
    // return value 0 as "pause completed without a signal" → re-checked
    // its condition (property service not ready) → retried pause →
    // INFINITE LOOP (1,048,000+ calls observed on commit 368f59b). 6-D
    // (commit 2b073f8) tried returning -EINTR (-4) but the UI E2E test
    // on 2b073f8 shows the pause loop is STILL there (992,000+
    // repeats) — -EINTR makes init think "interrupted by a signal" →
    // check the condition (property service not ready) → call pause()
    // again → INFINITE LOOP, because the property service will NEVER
    // signal readiness (kr64 has no property service). Task 6-E: return
    // -ENOSYS (-38) instead — tells init "this kernel does not
    // implement pause()" → init falls back to a non-pause wait (mirrors
    // how 6-C's shmget -ENOSYS made init fall back to non-shared-memory
    // property init, which worked — the shmget loop stopped).
    //
    // NOTE: this is the SAME number that the pre-6-C kr64 mistakenly
    // used for ABI_X86_32.shmget (because the i386 shm numbers were
    // copy-pasted from ABI_X86_64, where shmget IS 29). 6-C moved
    // shmget to 395 (the real i386 number), which left syscall 29
    // "unintercepted" by the shmget branch and falling through to the
    // default "returning 0" branch — exposing the pause() loop bug.
    pause: 29,
    // Task 6-S: fork / clone / vfork / wait4 / exit_group syscall numbers
    // (per /usr/include/x86_64-linux-gnu/asm/unistd_32.h, verified
    // directly against the kernel's UAPI header in Task 6-S). THIS is
    // the value set that fires at runtime — TWRP init + recovery are
    // i386 binaries. The dedicated always-log ENTRY/EXIT block for these
    // 5 syscalls (NOT gated by the 5000 post-execve cap) is what will
    // reveal whether the post-6-R recovery actually calls fork/clone/
    // vfork in the middle phase (iters 151-3271) that the pre-6-S 150-
    // cap was hiding. The bceac63 diagnostic showed ZERO such calls in
    // the last-10 buffer + the entire visible logcat — but that's
    // exactly the range that was hidden by the cap. With the always-log
    // block in place + the cap raised to 5000, the next UI E2E run
    // will tell us definitively whether recovery ever forks a service.
    clone_nr: 120,
    fork_nr: 2,
    vfork_nr: 190,
    wait4_nr: 114,
    exit_group_nr: 252,
    // i386 write = 4 (per /usr/include/x86_64-linux-gnu/asm/unistd_32.h:
    // __NR_write 4). THIS is the value that fires at runtime — TWRP
    // init (an i386 binary) issues write(fd, buf, count) as i386
    // syscall 4 to push KLOG lines to /dev/__kmsg__. The 6-U syscall-
    // EXIT diagnostic matches `syscall_num == abi.write` and captures
    // `min(ret, 256)` bytes from the buffer pointer (arg2 = ecx on
    // i386, which the kernel preserves across the syscall). See the
    // doc on `write` in `ChildAbi` for the full rationale.
    write: 4,
    // i386 read = 3 (per asm/unistd_32.h: __NR_read 3). THIS is the
    // value that fires at runtime — TWRP recovery (an i386 binary)
    // issues read(fd, buf, count) as i386 syscall 3. The 6-V syscall-
    // EXIT diagnostic matches `syscall_num == abi.read` and captures
    // `min(ret, 256)` bytes from the buffer pointer (arg2 = ecx on
    // i386). See the doc on `read` in `ChildAbi` for the full rationale.
    read: 3,
    // i386 mmap2 = 192 (per /usr/include/x86_64-linux-gnu/asm/unistd_32.h:
    // __NR_mmap2 192, verified directly against the kernel's UAPI
    // header in Task 6-Y). i386 has BOTH `mmap` (old, nr=90, takes a
    // pointer to a struct mmap_arg_struct) and `mmap2` (nr=192, the
    // modern 6-arg direct call with offset in 4096-byte pages).
    // Modern i386 bionic (incl. TWRP init) uses mmap2 EXCLUSIVELY —
    // it never issues plain mmap(nr=90). THIS is the value that fires
    // at runtime: init's property_init calls
    //   mmap2(NULL, 0x20000, PROT_READ|PROT_WRITE, MAP_SHARED, fd, 0)
    // on /dev/__properties__ to set up the property area.
    //
    // The Android zygote's seccomp filter (inherited by untrusted_app,
    // can't be removed) blocks file-backed MAP_SHARED mmap2 for i386
    // compat syscalls → returns -ENOSYS (-38). kr64's OWN seccomp is
    // already skipped (install_seccomp=false, Task 6-X) — the zygote
    // filter is the blocker. Anonymous mmap2 SUCCEEDS (the existing
    // allow list permits it). The 6-Y fix: at mmap2 ENTRY, if fd is
    // /dev/__properties__ AND flags & MAP_SHARED, rewrite the args to
    // be anonymous (MAP_ANONYMOUS|MAP_PRIVATE, fd=-1, offset=0) so the
    // kernel performs an anonymous mmap that succeeds.
    // See the doc on `mmap` / `mmap2` in `ChildAbi` for the full
    // root-cause analysis.
    mmap: -1,   // i386 plain mmap (nr=90) — unused by modern bionic.
    mmap2: 192, // i386 mmap2 — the value that fires at runtime.
    // i386 socketcall = 102 (per /usr/include/x86_64-linux-gnu/asm/
    // unistd_32.h: __NR_socketcall 102, verified directly against the
    // kernel's UAPI header in Task 6-Z3). THIS is the value that fires
    // at runtime — TWRP init (an i386 binary) issues socketcall(2, fd,
    // sockaddr, addrlen) to bind the property_service socket during
    // its property service startup. The socketcall multiplexes ALL
    // socket sub-calls (1=socket, 2=bind, 3=connect, 4=listen,
    // 5=accept, ...); arg1 = sub-call number, arg2 = pointer to the
    // array of the sub-call's args.
    //
    // The bind returns EADDRINUSE (-98) because a stale socket fd
    // from a previous relaunch cycle is still bound in the parent
    // (the twoyi app), inherited via fork and NOT closed before
    // forking the guest. The child's close loop (fds 3..1024) closes
    // the CHILD's fds, not the parent's → the parent's stale fd keeps
    // the address bound → EADDRINUSE → "init startup failure" →
    // exit_group(1). The Task 6-Z3 fix: at the syscall-EXIT stop, if
    // syscall_num == abi.socketcall_nr AND the return is negative
    // (error), fake the return to 0 (success). socket() returns a
    // positive fd (success), so it's NOT faked. See the doc on
    // `socketcall_nr` in `ChildAbi` for the full root-cause analysis.
    socketcall_nr: 102,
    // i386 poll = 168 (per /usr/include/x86_64-linux-gnu/asm/unistd_32.h:
    // __NR_poll 168, verified directly against the kernel's UAPI header
    // in Task 6-Z5). THIS is the value that fires at runtime — TWRP
    // recovery (an i386 binary) issues poll() as i386 syscall 168 in
    // its property_service startup loop after the bind has been faked
    // to 0 (Task 6-Z3). The faked bind hid the EADDRINUSE but the
    // socket is NOT actually bound → poll returns POLLERR=1 every call
    // → busy-wait. The Task 6-Z5 fix: at the syscall-EXIT stop, if
    // syscall_num == abi.poll_nr AND the return is positive (N fds
    // ready), fake the return to 0 (no fds ready). This stops the
    // busy-wait. See the doc on `poll_nr` in `ChildAbi` for the full
    // root-cause analysis (Task 6-Z5).
    poll_nr: 168,
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
    // Task 6-Y: arg5 + arg6 register indices — used by the mmap2
    // ENTRY handler to read the fd (arg5=edi) + offset (arg6=ebp)
    // args so we can rewrite them when the mmap is file-backed
    // MAP_SHARED on /dev/__properties__. On a 32-bit child the kernel
    // zero-extends edi into the 64-bit rdi slot (index 14) and ebp
    // into the 64-bit rbp slot (index 4) when reporting registers
    // via PTRACE_GETREGS — so we use those slot indices.
    #[allow(dead_code)]
    reg_arg5: 14, // rdi (zero-extended edi)
    #[allow(dead_code)]
    reg_arg6: 4, // rbp (zero-extended ebp)
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
    // asm-generic (aarch64) uses statx + newfstatat exclusively; there
    // are NO stat64/lstat64/fstat64 syscall numbers on aarch64. Verified
    // against /usr/include/asm-generic/unistd.h: __NR_stat64 /
    // __NR_lstat64 / __NR_fstat64 are only defined when
    // __ARCH_WANT_STAT64 is set, which aarch64 does NOT set (aarch64
    // came after the stat64 era + mandates the statx/newfstatat API).
    // Set to -1 (sentinels "not present on this ABI"), mirroring the
    // existing ABI_AARCH64.open / ABI_AARCH64.access precedent.
    // Task 6-T.
    stat64: -1,
    lstat64: -1,
    fstat64: -1,
    access: -1,
    faccessat: 48,
    rt_sigprocmask: 135,
    readlink: -1,
    readlinkat: 78,
    chdir: 49,
    // aarch64 (asm-generic): unlink=-1 (dropped; uses unlinkat=35).
    // Task 6-Y; verified against /usr/include/asm-generic/unistd.h.
    unlink: -1,
    unlinkat: 35,
    // TWRP-init EPERM workaround — see the long comment on these
    // fields in `ChildAbi`. aarch64 uses asm-generic/unistd.h, where
    // fchown=55, fchmod=52, capget=90, ioprio_get=31, ioprio_set=30
    // (verified in Task 5-S against /usr/include/asm-generic/unistd.h).
    fchown: 55,
    fchmod: 52,
    capget: 90,
    ioprio_get: 31,
    ioprio_set: 30,
    // chmod / lchown / chown / fchmodat / fchownat (aarch64 numbers
    // per asm-generic/unistd.h):
    //   fchmodat=53, fchownat=54.
    // asm-generic has NO plain `chmod` / `lchown` / `chown` — bionic's
    // `chmod(path, mode)` shim issues `fchmodat(AT_FDCWD, path, mode, 0)`
    // (syscall 53), and similarly for chown (fchownat, syscall 54).
    //
    // The existing ABI_AARCH64.chmod field is set to 53 (the
    // fchmodat number) for historical reasons — pre-5-A code routed
    // the SIGSYS handler's "chmod" branch through syscall 53. We keep
    // chmod=53 unchanged AND set fchmodat=53 so both names match.
    // (Net effect: syscall 53 is faked-success at EXIT.)
    //
    // lchown / chown are set to -1 ("does not exist on this ABI") so
    // the EXIT-handler comparisons simply never match them — they
    // are reached only via fchownat (54) at runtime.
    chmod: 53,
    lchown: -1,
    chown: -1,
    fchmodat: 53,
    fchownat: 54,
    execve: 221, // SYS_execve (aarch64)
    // aarch64 mount = 40 (per /usr/include/asm-generic/unistd.h,
    // verified directly against the kernel's UAPI header in Task 5-T).
    // Pre-5-T this was 165 — WRONG: aarch64 syscall 165 is `getrusage`,
    // NOT `mount` (the 165 value was copy-pasted from ABI_X86_64 where
    // it IS correct). With the wrong number the SIGSYS handler's
    // `mount` branch would never match a real mount() call on aarch64
    // (and worse, would have spurious-matched any getrusage SIGSYS).
    mount: 40,
    chroot: 51,
    mkdir: 34,
    unshare: 97,
    // aarch64 mknod = -1 (SENTINEL "not present on this ABI"). The
    // asm-generic/unistd.h table (used by aarch64) has NO plain
    // `mknod` — only `mknodat = 33` (verified directly against
    // /usr/include/asm-generic/unistd.h in Task 5-X). bionic's
    // mknod(pathname, mode, dev) libc wrapper on aarch64 issues
    // mknodat(AT_FDCWD, pathname, mode, dev) under the hood, so the
    // syscall that actually hits the kernel is mknodat (33), not
    // mknod. With ABI_AARCH64.mknod = -1:
    //   - syscall_name(-1, &ABI_AARCH64) falls through to "unknown"
    //     (the mknod branch never matches — no real syscall is -1).
    //   - compute_exit_return_value(-1, &ABI_AARCH64) would match the
    //     `|| syscall_nr == abi.mknod` clause (`-1 == -1`) and return
    //     Some(0) — but no real caller ever passes -1, so this is
    //     harmless. (If you wanted to be strictly correct you could
    //     special-case -1 in the if-chain, but the existing pattern
    //     for ABI_AARCH64.open / access / lchown / chown does NOT
    //     special-case -1 either — open/access aren't in the if-chain
    //     at all, lchown/chown ARE. For lchown/chown the same
    //     "harmless if no real syscall is -1" reasoning applies.)
    //   - A future aarch64-specific fix would add a dedicated
    //     `mknodat: i64` field (= 33) instead of aliasing mknod to 33.
    //     Aliasing mknod to 33 would mislabel mknodat SIGSYS as "mknod"
    //     in syscall_name() (mislabeled but harmless) AND would
    //     intercept a real mknodat in compute_exit_return_value
    //     (acceptable, but conflates two different syscalls in one
    //     field — confusing for future maintainers).
    // The host is x86_64 running an i386 child, so this aarch64 path
    // is currently dead code at runtime — the sentinel keeps the
    // compile happy and documents the aarch64 behaviour.
    mknod: -1,
    // aarch64 setxattr / lsetxattr / fsetxattr. Real Android aarch64
    // bionic uses the upstream Linux asm-generic numbers: setxattr=188,
    // lsetxattr=189, fsetxattr=190 (matching x86_64). This sandbox's
    // /usr/include/asm-generic/unistd.h NON-STANDARDLY lists these as
    // 5/6/7, but those numbers are io_setup / io_destroy /
    // io_getevents in upstream Linux — the sandbox header is wrong
    // (verified directly in Task 6-R; see the doc on these fields in
    // `ChildAbi` for the full discrepancy analysis). We use 188/189/190
    // so that a real aarch64 TWRP recovery calling lsetxattr() will be
    // correctly fake-succeeded.
    setxattr: 188,
    lsetxattr: 189,
    fsetxattr: 190,
    // SysV shared-memory syscalls — see the comment on these fields
    // in `ChildAbi`. aarch64 uses asm-generic/unistd.h, where
    // shmget=194, shmctl=195, shmat=196.
    shmget: 194,
    shmat: 196,
    shmctl: 195,
    // aarch64 pause = -1 (SENTINEL "not present on this ABI"). The
    // asm-generic/unistd.h table (used by aarch64) has NO __NR_pause
    // — pause() was REMOVED in the asm-generic table (verified
    // directly against /usr/include/asm-generic/unistd.h in Task 6-D
    // — `grep __NR_pause` returned nothing). aarch64 callers use
    // ppoll(NULL, 0, NULL, NULL) or nanosleep instead; bionic's
    // pause() libc wrapper on aarch64 issues ppoll under the hood.
    // With ABI_AARCH64.pause = -1:
    //   - syscall_name(-1, &ABI_AARCH64) falls through to "unknown"
    //     (the pause branch never matches — no real syscall is -1).
    //   - compute_exit_return_value(-1, &ABI_AARCH64) returns None
    //     (pause is not in the fake-success if-chain at all).
    //   - The SIGSYS handler's pause branch (`original_syscall ==
    //     a.pause`) never matches -1 — no real caller ever passes -1.
    //   - A future aarch64-specific fix would add a dedicated
    //     `ppoll: i64` field (= 73 in asm-generic) instead of aliasing
    //     pause to it — aliasing would mislabel ppoll SIGSYS as
    //     "pause" in syscall_name() (mislabeled but harmless) AND
    //     would intercept a real ppoll in the SIGSYS handler (would
    //     force -ENOSYS for any ppoll the guest makes, even legitimate
    //     ones that should return 0 — too risky). Mirrors the existing
    //     pattern for ABI_AARCH64.open / access / lchown / chown /
    //     mknod, which are also set to -1 for the same "asm-generic
    //     dropped it" reason.
    // The host is x86_64 running an i386 child, so this aarch64 path
    // is currently dead code at runtime — the sentinel keeps the
    // compile happy and documents the aarch64 behaviour.
    pause: -1,
    // Task 6-S: fork / clone / vfork / wait4 / exit_group syscall numbers
    // (per /usr/include/asm-generic/unistd.h, verified directly against
    // the kernel's UAPI header in Task 6-S). aarch64 uses asm-generic,
    // which DROPPED plain `fork` and `vfork` (bionic's fork() libc
    // wrapper on aarch64 issues clone() under the hood — see the
    // existing pattern for ABI_AARCH64.open / access / lchown / chown /
    // mknod / pause). So fork_nr=-1 and vfork_nr=-1.
    // NOTE: __NR_exit_group is 94 (NOT 93 — 93 is plain `exit`, which
    // exits just the calling thread; exit_group exits all threads in the
    // process and is what bionic's exit_group() / _Exit() wrappers call).
    // The task spec for 6-S said aarch64 exit_group=93 — that was WRONG;
    // verified: 94. __NR_clone is 220, __NR_wait4 is 260.
    // The host is x86_64 running an i386 child, so this aarch64 path is
    // currently dead code at runtime — locked in for ABI completeness
    // + so the always-log block compiles + works correctly if a future
    // aarch64 host is ever used.
    clone_nr: 220,
    fork_nr: -1,
    vfork_nr: -1,
    wait4_nr: 260,
    exit_group_nr: 94,
    // aarch64 write = 64 (per /usr/include/asm-generic/unistd.h:
    // __NR_write 64). NOT intercepted — carried for ABI completeness
    // so the 6-U syscall-EXIT diagnostic's `syscall_num == abi.write`
    // comparison is correct if a future aarch64 guest is ever
    // supported. The host is x86_64 running an i386 child, so this
    // aarch64 number does NOT currently fire at runtime.
    write: 64,
    // aarch64 read = 63 (per asm-generic/unistd.h: __NR_read 63).
    // NOT intercepted — carried for ABI completeness so the 6-V
    // syscall-EXIT diagnostic's `syscall_num == abi.read` comparison
    // is correct if a future aarch64 guest is ever supported. The host
    // is x86_64 running an i386 child, so this aarch64 number does NOT
    // currently fire at runtime.
    read: 63,
    // aarch64 mmap = 222 (per /usr/include/asm-generic/unistd.h:
    // __NR_mmap 222, verified directly against the kernel's UAPI
    // header in Task 6-Y). asm-generic has NO mmap2 (the modern
    // asm-generic mmap takes offset in BYTES directly, not in 4096-
    // byte pages — there is no need for the mmap2 page-shift
    // workaround that i386 needs). The host is x86_64 running an i386
    // child, so this aarch64 number does NOT currently fire at runtime.
    // Locked in for ABI completeness + so the mmap ENTRY handler's
    // `== abi.mmap || == abi.mmap2` comparison is correct if a future
    // aarch64 host is ever used. See the doc on `mmap` / `mmap2` in
    // `ChildAbi` for the full root-cause analysis (Task 6-Y).
    mmap: 222,
    mmap2: -1, // asm-generic has no mmap2 — sentinel.
    // aarch64 socketcall = -1 (SENTINEL — asm-generic has NO
    // socketcall; aarch64 uses the direct socket/bind/listen/connect/
    // accept syscalls instead, per /usr/include/asm-generic/unistd.h:
    // __NR_socket 198, __NR_bind 200, __NR_listen 201, __NR_connect
    // 203, __NR_accept 202). The host is x86_64 running an i386 child,
    // so this aarch64 number does NOT currently fire at runtime (the
    // guest uses i386 syscall 102). Locked in for ABI completeness +
    // so the EXIT handler's `== abi.socketcall_nr` comparison is
    // correct if a future aarch64 guest is ever supported. Mirrors
    // the existing ABI_AARCH64.mmap2 = -1 precedent. Task 6-Z3.
    socketcall_nr: -1,
    // aarch64 poll = -1 (SENTINEL — asm-generic/unistd.h has NO
    // __NR_poll; aarch64 callers use ppoll/nanosleep instead, per
    // /usr/include/asm-generic/unistd.h. bionic's poll() libc wrapper
    // on aarch64 issues ppoll(NULL, n, timeout, NULL) under the hood.
    // A future aarch64-specific fix would need a dedicated `ppoll: i64`
    // field (= 73 in asm-generic) instead of aliasing poll to it —
    // aliasing would mislabel ppoll SIGSYS as "poll" in
    // syscall_name() (mislabeled but harmless) AND would intercept a
    // real ppoll in the EXIT handler (would force return to 0 for any
    // ppoll the guest makes, even legitimate ones that should return
    // N>0 — too risky). Mirrors the existing pattern for
    // ABI_AARCH64.mmap2 / .open / .access / .lchown / .chown / .mknod /
    // .pause / .socketcall, which are all set to -1 for the same
    // "asm-generic dropped it" reason. The host is x86_64 running an
    // i386 child, so this aarch64 path is currently dead code at
    // runtime — the sentinel keeps the compile happy + documents the
    // aarch64 behaviour. Task 6-Z5.
    poll_nr: -1,
    reg_syscall: 8, // x8 (syscall number)
    reg_ret: 0,     // x0 (return value)
    reg_arg1: 0,    // x0
    reg_arg2: 1,    // x1
    reg_arg3: 2,    // x2
    reg_arg4: 3,    // x3
    // Task 6-Y: arg5 + arg6 register indices — used by the mmap ENTRY
    // handler to read the fd (arg5=x4) + offset (arg6=x5) args so we
    // can rewrite them when the mmap is file-backed MAP_SHARED on
    // /dev/__properties__. aarch64 user_pt_regs is `u64 regs[31]`
    // (x0..x30), so x4=4, x5=5.
    #[allow(dead_code)]
    reg_arg5: 4, // x4
    #[allow(dead_code)]
    reg_arg6: 5, // x5
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
/// Historically used by the SIGSYS handler to rewrite a seccomp-blocked
/// syscall into a harmless one (getpid) before resuming. This rewrite
/// was REMOVED in the "never rewrite orig_rax" fix — see the SIGSYS
/// handler for the rationale. The function is now used by Task 6-Z9's
/// ENTRY-side xattr-SET → getpid rewrite (a DIFFERENT code path that
/// rewrites orig_rax BEFORE the kernel executes the syscall, which is
/// safe — unlike the SIGSYS case where the kernel had already aborted
/// the syscall).
fn set_syscall_num(regs: &mut Regs, abi: &ChildAbi, val: i64) {
    let regs_ptr = regs as *mut Regs as *mut u64;
    unsafe {
        *regs_ptr.add(abi.reg_syscall) = val as u64;
    }
}

/// Decide whether a given syscall number should have its return value
/// forced to 0 (success) at syscall-EXIT, and if so return `Some(0)`.
///
/// This is the central definition of "TWRP init treats a non-zero return
/// here as a fatal error" — both the EXIT handler (for non-seccomp-blocked
/// syscalls that return EPERM) and the SIGSYS handler (for
/// seccomp-blocked syscalls) consult this list so the two paths agree.
/// Without this single source of truth, the two handlers drifted:
/// historically the EXIT handler covered only `fchown / fchmod / capget /
/// ioprio_get` (commit f279552) and the SIGSYS handler covered only those
/// four PLUS `chmod` (via the `mount/mkdir/chmod/chroot/unshare` block).
/// The path-taking siblings `lchown / chown / fchmodat / fchownat` were
/// MISSING from BOTH — when init called `chmod("/proc/cmdline", ...)` and
/// the kernel left rax = 15 (the syscall number, NOT 0, NOT -ENOSYS, NOT
/// -EPERM) at the syscall-EXIT stop on i386 compat, init's chmod-error
/// path dereferenced a NULL+0x90 pointer and SIGSEGV'd.
///
/// `ioprio_set` was ALSO MISSING until Task 5-S — only `ioprio_get` was
/// in the historical list. TWRP init calls ioprio_set during early boot
/// (to set the I/O priority of background services), and an EPERM there
/// can trip init's fatal-config-error path. The number set is verified
/// against the kernel's UAPI headers in Task 5-S — see the comment on
/// `ioprio_set` in `ChildAbi` for the per-ABI values.
///
/// `mount` and `rt_sigprocmask` were ALSO MISSING until Task 5-T. These
/// are the REAL root cause of the UI E2E TWRP init exit(1) at iter 189
/// (the ioprio_set hypothesis from 5-S was a misdiagnosis — see
/// DISPATCHER-CORRECTION-3 in the worklog: nr=252 in the logcat was
/// `exit_group`, the SYMPTOM of init deciding to exit, not the cause).
/// The 3b571fe UI E2E logcat (re-read after 5-S caught the ioprio
/// misdiagnosis) shows:
/// ```text
/// #26: mount(nr=21)         → returns 21  ← BUG! syscall NUMBER, not 0
/// #29: mount(nr=21)         → returns 21
/// #30: mount(nr=21)         → returns 21
/// #31: mount(nr=21)         → returns 21
/// #34: rt_sigprocmask(nr=14) → returns 14  ← BUG! (see NOTE below)
/// ... then exit_group(1)
/// ```
///
/// The mount SIGSYS handler already returns 0 via the
/// `mount/mkdir/chmod/chroot/unshare` block — but in DESYNC mode
/// (5-J's fix) the SIGSYS handler SKIPS its `ptrace_setregs` call, so
/// the EXIT handler's write is the only one. Without `mount` in this
/// list, the EXIT handler leaves rax = the kernel's syscall-number-leak
/// value (21 on i386) → init sees "mount returned 21" four times in a
/// row → init's mount-sequence-failed path → exit_group(1).
///
/// NOTE on the i386 rt_sigprocmask number: the diagnostic label
/// "rt_sigprocmask() nr=14" in the 3b571fe logcat is itself a misnomer
/// caused by a SECOND bug fixed in this same commit: ABI_X86_32.
/// rt_sigprocmask was previously 14 — but i386 syscall 14 is actually
/// `mknod` (per /usr/include/x86_64-linux-gnu/asm/unistd_32.h, verified
/// in Task 5-T). The real i386 rt_sigprocmask number is 175. So the
/// child was EITHER calling mknod (syscall 14 — TWRP init does call
/// mknod for /dev nodes during early boot) OR calling rt_sigprocmask
/// (syscall 175 — also called by bionic's signal-mask init); either way
/// the kr64 SIGSYS handler was matching syscall 14 against the (wrong)
/// ABI_X86_32.rt_sigprocmask=14 and labelling it "rt_sigprocmask".
/// After this commit the i386 rt_sigprocmask number is corrected to
/// 175 AND both mount and rt_sigprocmask are in the fake-success list,
/// so the EXIT handler writes rax=0 for whichever of the two the child
/// actually calls. (If the child was actually calling mknod, this
/// commit does NOT add mknod to the fake-success list — that is left
/// for a follow-up; see the worklog entry for 5-T.)
///
/// The per-ABI rt_sigprocmask + mount numbers (verified against the
/// kernel's UAPI headers in Task 5-T):
///   i386:   mount=21,  rt_sigprocmask=175   (was WRONG: rt_sigprocmask=14)
///   x86_64: mount=165, rt_sigprocmask=14    (both already correct)
///   aarch64: mount=40, rt_sigprocmask=135   (mount was WRONG: 165)
///
/// `mknod` was ALSO MISSING until Task 5-X. After 5-T's mount fix
/// advanced mount from "returns 21" to "returns 0", the next blocker
/// surfaced in 5-W's VLM-verified UI E2E analysis: the post-5-T
/// logcat showed
/// ```text
/// post-execve syscall #34: nr=14 [unknown]
/// post-execve return  #34: unknown nr=14 -> 14   ← NON-ZERO, NOT faked
/// ... then exit_group(1) at iter 189 (unchanged from 3b571fe)
/// ```
/// i386 syscall 14 is `mknod` (per /usr/include/x86_64-linux-gnu/asm/
/// unistd_32.h, verified directly in Task 5-X). 5-T's i386
/// rt_sigprocmask number correction (14 → 175) had CLEARED the
/// diagnostic-label misnomer — pre-5-T the kr64 SIGSYS handler was
/// mislabelling every mknod SIGSYS as "rt_sigprocmask() nr=14";
/// post-5-T it correctly says "[unknown]" for syscall 14. 5-X adds
/// the mknod field (so syscall_name() says "mknod", not "[unknown]")
/// AND adds mknod to the fake-success list (so the EXIT handler
/// writes rax=0 instead of leaving the kernel's leaked 14).
///
/// 5-W's CRITICAL FOLLOW-UP: the mknod fix fakes `rax=0` BUT does NOT
/// actually create a device node in the rootfs (unlike the
/// mount/mkdir/chmod/chroot/unshare block which creates real
/// directories). The guest's NEXT `open(/dev/null)` may fail. 5-X
/// therefore ALSO extends the SIGSYS handler's mknod branch to create
/// a matching EMPTY regular file at `{rootfs}<path>` so guest open()
/// succeeds (mirroring what the mount/mkdir block already does for
/// directories). This is a best-effort stub — empty-file creation is
/// sufficient for /dev/null (writes succeed as no-op, reads return
/// EOF) but gives wrong read-content for /dev/zero and /dev/urandom
/// (reads return 0 bytes instead of \0-bytes / random bytes). Good
/// enough to get init past the mknod failure + subsequent open; if a
/// later TWRP code path actually reads from /dev/zero or /dev/urandom
/// expecting real content, that's the next-next blocker.
///
/// The per-ABI mknod numbers (verified against the kernel's UAPI
/// headers in Task 5-X):
///   i386:   mknod = 14   (per asm/unistd_32.h)
///   x86_64: mknod = 133  (per asm/unistd_64.h)
///   aarch64: mknod = -1  (SENTINEL — no plain mknod in asm-generic/
///     unistd.h, only mknodat=33. bionic's mknod() libc wrapper on
///     aarch64 issues mknodat(AT_FDCWD, ...) under the hood. A future
///     aarch64-specific fix would need a dedicated mknodat field.)
///
/// `setxattr` / `lsetxattr` / `fsetxattr` were ALSO MISSING until Task
/// 6-R. After 6-Q's PTRACE_O_TRACEFORK machinery successfully traces
/// the recovery child (post-6-Q UI E2E on e04dab6: recovery runs ~135
/// post-execve syscalls, no longer untraced), the NEW blocker was
/// recovery entering a retry loop at syscalls #123 + #135 — both
/// nr=227 (lsetxattr on i386). TWRP recovery calls lsetxattr(path,
/// "security.selinux", ctx, 44, 0) during its SELinux-restorecon
/// phase; as untrusted_app the kernel returns -EPERM, recovery treats
/// EPERM as retryable → infinite spin → death → kr64 relaunches every
/// 2s (20:29:56 to 20:30:10). With these in the fake-success list,
/// the EXIT handler writes rax=0 and recovery sees "lsetxattr
/// returned 0 (success)". SELinux labeling is not enforced in the
/// sandbox (same pragmatic pattern as chmod/mknod/mount).
///
/// The per-ABI setxattr/lsetxattr/fsetxattr numbers (verified against
/// the kernel's UAPI headers in Task 6-R):
///   i386:   setxattr=226, lsetxattr=227, fsetxattr=228
///           (per /usr/include/x86_64-linux-gnu/asm/unistd_32.h)
///   x86_64: setxattr=188, lsetxattr=189, fsetxattr=190
///           (per /usr/include/x86_64-linux-gnu/asm/unistd_64.h)
///   aarch64: setxattr=188, lsetxattr=189, fsetxattr=190
///           (upstream Linux asm-generic — matches x86_64. NOTE:
///           this sandbox's /usr/include/asm-generic/unistd.h
///           NON-STANDARDLY lists these as 5/6/7, which are
///           io_setup/io_destroy/io_getevents in upstream Linux —
///           the sandbox header is wrong. Real Android aarch64
///           bionic uses 188/189/190.)
///
/// Returns `Some(0)` for the faked-success syscalls, `None` for syscalls
/// whose return value the caller should leave untouched. The value (0) is
/// hard-coded because every faked-success syscall uses the same return
/// value; if a future case needs a different value (e.g. -ENOSYS for a
/// "this kernel does not implement X" emulation), it must be handled
/// separately (as `shmget` is in the SIGSYS handler — see the
/// `-(libc::ENOSYS as i64)` branch).
fn compute_exit_return_value(syscall_nr: i64, abi: &ChildAbi) -> Option<i64> {
    // Order the comparisons by expected frequency-of-occurrence during
    // TWRP init's early boot: chmod and the *at siblings are called
    // multiple times before the SIGSEGV that motivated this fix.
    //
    // Task 5-T added mount + rt_sigprocmask to this set: in DESYNC mode
    // (5-J) the SIGSYS handler skips setregs, so the EXIT handler is
    // the only writer — and these two were missing, so the kernel's
    // syscall-number-leak value (21 for mount, 14/175 for rt_sigprocmask)
    // was the final value the child saw → init exit(1).
    //
    // Task 5-X added mknod to this set: 5-W's VLM-verified UI E2E
    // analysis showed that AFTER 5-T's mount fix, mknod (i386 syscall
    // 14) became the next blocker — the post-5-T logcat showed
    // "post-execve return #34: unknown nr=14 -> 14" (NON-ZERO, NOT
    // faked) → init treats it as a fatal config error → exit(1) at
    // iter 189 (UNCHANGED from 3b571fe — 5-T's mount fix advanced
    // mount but not mknod). With mknod in this list, the EXIT handler
    // writes rax=0 and init sees "mknod returned 0 (success)".
    //
    // Task 6-R added setxattr / lsetxattr / fsetxattr to this set:
    // AFTER 6-Q's PTRACE_O_TRACEFORK machinery successfully traces
    // the recovery child (post-6-Q UI E2E on e04dab6: recovery runs
    // ~135 post-execve syscalls, no longer untraced), the NEW blocker
    // was recovery entering a retry loop at syscalls #123 + #135 —
    // both nr=227 (lsetxattr on i386, verified against
    // /usr/include/x86_64-linux-gnu/asm/unistd_32.h). TWRP recovery
    // calls lsetxattr(path, "security.selinux", ctx, 44, 0) during its
    // SELinux-restorecon phase. As untrusted_app the kernel returns
    // -EPERM (no CAP for security.* xattrs), recovery treats EPERM as
    // retryable → infinite spin → death → kr64 relaunches every 2s
    // (parent setup logs repeat 20:29:56 to 20:30:10). With these in
    // the list, the EXIT handler writes rax=0 and recovery sees
    // "lsetxattr returned 0 (success)". SELinux labeling is not
    // enforced in the sandbox (same pragmatic fake-success pattern as
    // chmod/mknod/mount — non-fatal for TWRP boot).
    if syscall_nr == abi.chmod
        || syscall_nr == abi.fchmod
        || syscall_nr == abi.fchown
        || syscall_nr == abi.lchown
        || syscall_nr == abi.chown
        || syscall_nr == abi.fchmodat
        || syscall_nr == abi.fchownat
        || syscall_nr == abi.capget
        || syscall_nr == abi.ioprio_get
        || syscall_nr == abi.ioprio_set
        || syscall_nr == abi.mount
        || syscall_nr == abi.rt_sigprocmask
        || syscall_nr == abi.mknod
        || syscall_nr == abi.setxattr
        || syscall_nr == abi.lsetxattr
        || syscall_nr == abi.fsetxattr
    {
        Some(0)
    } else {
        None
    }
}

/// Decide whether the SIGSYS handler should skip its `ptrace_setregs`
/// call. **Task 6-W: ALWAYS returns `false` (never skip).**
///
/// # Background — the historical DESYNC register-writeback race
///
/// On i386 compat (and on some kernels for x86_64 too), the kernel
/// delivers the ptrace stops for a seccomp-trapped syscall in this
/// order:
///
/// 1. syscall-ENTRY-stop (WSTOPSIG = SIGTRAP|0x80)
/// 2. syscall-EXIT-stop  (WSTOPSIG = SIGTRAP|0x80)
/// 3. SIGSYS signal-delivery-stop (WSTOPSIG = SIGSYS)
///
/// (The kernel's `exit_to_user_mode_prepare` calls `trace_sys_exit`
/// BEFORE `do_signal`, so the EXIT stop is delivered BEFORE the
/// SIGSYS signal-delivery-stop — this is the order 5-H's log
/// evidence shows for chmod nr=15 on i386 compat.)
///
/// Both the EXIT handler (step 2) and the SIGSYS handler (step 3)
/// call `ptrace_setregs` to write rax=0 for the faked-success syscalls
/// in `compute_exit_return_value`. The two writebacks are *intended*
/// to be redundant (belt-and-suspenders). 5-J observed that in the
/// DESYNC case the SIGSYS handler's whole-struct `setregs` COULD race
/// with the kernel's signal-delivery-stop register snapshotting: if
/// the kernel re-snapshotted rax from `syscall_rollback`
/// (rax=orig_rax=syscall number), the child could resume with rax=15
/// (i386 chmod) instead of rax=0 → SIGSEGV at rip=0x809255d.
///
/// # The 5-J fix (now REVERTED by 6-W)
///
/// 5-J's fix: in DESYNC mode (`in_syscall == false` at SIGSYS entry),
/// the SIGSYS handler SKIPPED its `ptrace_setregs` call entirely. The
/// EXIT handler's rax=0 was meant to be the final value the child
/// sees on resume. 6-C refined this: the skip fired ONLY for syscalls
/// in `compute_exit_return_value`'s fake-success list (so shmget's
/// -ENOSYS writeback still executed).
///
/// # Why 6-W reverts the skip — the rodata-leak SIGSEGV
///
/// The skip left the signal frame's registers in whatever state the
/// kernel's signal-delivery setup left them. In the iter-826 UI E2E
/// run (post-6-V's NOP at 0xaf65), the SIGSEGV immediately
/// re-manifested at `rip=0x6f722f69` (ASCII "i/ro" — a rodata
/// pointer leaked into a control-flow register). 6-V's NOP patched
/// ONE crash site (the `*arg2 = readcount` store at 0x8052f65), but
/// the SAME root cause — garbage rodata bytes leaking into a register
/// used as a jump/call target — immediately re-appeared at a
/// DIFFERENT rip. Masking one site does NOT fix the underlying
/// register-corruption race.
///
/// # The 6-W fix — ALWAYS do a fresh getregs + setregs
///
/// 6-W: **never skip**. In DESYNC mode the SIGSYS handler now does a
/// FRESH `ptrace_getregs` (re-reading the CURRENT post-signal-
/// delivery register state — NOT the stale pre-EXIT values that
/// motivated 5-J's skip), then `set_syscall_ret(rax=ret_val)`, then
/// `ptrace_setregs`. The fresh getregs reads the live register state
/// (the child is stopped, so registers are stable), avoiding the
/// stale-value race. The setregs writes rax=ret_val to the signal
/// frame so sigreturn restores it correctly. It ALSO re-writes the
/// OTHER registers with their current values, preventing the
/// rodata-leak SIGSEGV that the skip was causing.
///
/// In NORMAL mode (`in_syscall == true` at SIGSYS entry), the
/// existing flow is unchanged: the SIGSYS-entry `ptrace_getregs`
/// (line ~4667) already read the current state, `set_syscall_ret`
/// set rax, and `ptrace_setregs` writes it back. No fresh getregs
/// is needed in NORMAL mode (the SIGSYS-entry read IS already
/// current).
///
/// # Why this function still exists (instead of being deleted)
///
/// The function is kept (always returning `false`) so the existing
/// call site and unit tests compile. The signature is preserved so
/// future investigation can re-enable a conditional skip if a NEW
/// race is discovered (the function would then return `true` again
/// under whatever new condition is identified). The historical
/// doc above is retained so the next investigator understands the
/// full 5-J → 6-C → 6-W evolution.
#[allow(dead_code)] // Kept as a testable contract — see the doc comment.
fn should_skip_sigsys_setregs(
    _in_syscall_at_sigsys: bool,
    _syscall_nr: i64,
    _abi: &ChildAbi,
) -> bool {
    // 6-W: NEVER skip the SIGSYS handler's ptrace_setregs. The 5-J/6-C
    // skip caused the iter-826 SIGSEGV at rip=0x6f722f69 ("i/ro" rodata
    // leak) — see the function's doc comment for the full root-cause
    // analysis. The SIGSYS handler now ALWAYS does a fresh
    // ptrace_getregs → set_syscall_ret(rax=ret_val) → ptrace_setregs
    // in DESYNC mode (handled at the call site, NOT here).
    //
    // This function is kept (always returning false) so the unit tests
    // can pin the 6-W contract ("never skip") as a regression guard.
    // The SIGSYS handler no longer calls this function — it
    // unconditionally does the fresh getregs + setregs in DESYNC mode.
    // If a future change re-introduces a conditional skip, restore the
    // call site AND update this function's body (and the tests).
    false
}

/// The return value the SIGSYS handler writes for pause() (Task 6-E).
///
/// Returns `-ENOSYS` (-38) — NOT `-EINTR`, NOT 0. See the dedicated
/// branch in the SIGSYS handler for the full rationale. Extracted as a
/// named function so the contract is unit-testable (the SIGSYS handler
/// itself is inline ptrace code and not directly callable from tests).
///
/// # Why -ENOSYS (Task 6-E), not -EINTR (6-D's attempt) or 0 (pre-6-D)
///
/// 6-D (commit 2b073f8) returned -EINTR (-4): this made init think
/// "interrupted by a signal" → check the condition (property service
/// not ready) → call pause() again → INFINITE LOOP. The UI E2E test on
/// 2b073f8 showed the pause loop was STILL there (992,000+ repeats) —
/// -EINTR did NOT break the loop. The property service will NEVER
/// signal readiness because kr64 has NO property service (5-Y's
/// find_property binary patch makes lookups return NULL, but there's
/// no actual service to send the "ready" signal).
///
/// Returning 0 (the pre-6-D default) is ALSO wrong: init interprets 0
/// as "pause completed WITHOUT a signal" → re-checks its condition →
/// calls pause() again → INFINITE LOOP (the post-6-C UI E2E blocker,
/// 1,048,000+ repeats on commit 368f59b).
///
/// -ENOSYS (-38) tells init "this kernel does not implement pause()"
/// → init falls back to a non-pause wait mechanism (or skips the wait
/// entirely). This mirrors how 6-C's shmget -ENOSYS made init fall
/// back to non-shared-memory property init (which WORKED — the shmget
/// loop stopped). The same fallback pattern should break the pause
/// loop here.
fn sigsys_ret_for_pause() -> i64 {
    -(libc::ENOSYS as i64)
}

/// Threshold for the pause() timeout (Task 6-G).
///
/// After this many CONSECUTIVE pause() SIGSYS calls, the SIGSYS handler
/// returns `-ETIMEDOUT` instead of `-ENOSYS`. With Task 6-F's 100ms sleep,
/// 50 pauses ≈ 5 seconds — a reasonable "give up" deadline for the
/// missing property service's "ready" signal (which never arrives in
/// kr64's sandboxed environment — kr64 has NO property service; 5-Y's
/// find_property binary patch makes LOOKUPS return NULL, but there's no
/// actual service to send the "ready" signal init's pause() loop waits
/// for).
///
/// -ENOSYS (Task 6-E) was the right idea ("kernel doesn't implement
/// pause → fall back to a non-pause wait") but the UI E2E test on
/// 6e51920 showed init kept calling pause regardless of -ENOSYS (833
/// pauses over 90s — reduced from 659k/sec by 6-F's sleep, but NOT
/// broken). The deeper root cause is the missing property service — init
/// doesn't care WHAT pause returns, it just keeps calling pause until
/// the property service signals readiness. So -ENOSYS alone can't break
/// the loop.
///
/// -ETIMEDOUT (-110) is a different signal: it tells init "the wait
/// TIMED OUT" (not "the syscall is unimplemented"). Many init
/// implementations treat timeout as "the dependency didn't start in
/// time" and proceed with defaults instead of looping forever — which
/// is exactly the behaviour we need to break the pause loop.
///
/// If -ETIMEDOUT doesn't break the loop either (init may treat it as
/// retryable, like -EINTR), the next attempt should be -EIO (-5,
/// "I/O error") or a direct property-service stub implementation (see
/// the worklog DISPATCHER-FINAL-ASSESSMENT for the full analysis).
const PAUSE_TIMEOUT_THRESHOLD: u32 = 50;

/// The return value the SIGSYS handler writes for pause() AFTER
/// `pause_count` consecutive pause() SIGSYS calls (Task 6-G).
///
/// Returns:
///   - `-ETIMEDOUT` (-110) if `pause_count > PAUSE_TIMEOUT_THRESHOLD` —
///     signals "the wait timed out", which should make init give up
///     waiting for the missing property service and proceed (many init
///     implementations treat timeout as non-fatal + proceed with
///     defaults). This is the FALLBACK when -ENOSYS (Task 6-E) didn't
///     break the loop.
///   - `-ENOSYS` (-38) otherwise — the 6-E default, makes init fall
///     back to a non-pause wait (mirrors 6-C's shmget -ENOSYS fallback).
///
/// Extracted as a named function so the threshold contract is
/// unit-testable (the SIGSYS handler itself is inline ptrace code and
/// not directly callable from tests — see the `pause_ret_after_*` tests
/// below).
fn pause_ret_after(pause_count: u32) -> i64 {
    if pause_count > PAUSE_TIMEOUT_THRESHOLD {
        -(libc::ETIMEDOUT as i64)
    } else {
        sigsys_ret_for_pause()
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
    } else if nr == abi.mknod {
        // Added in Task 5-X. Pre-5-X, syscall 14 on i386 was labelled
        // "[unknown]" (because no field matched it — the i386
        // rt_sigprocmask number was corrected to 175 in Task 5-T, so
        // syscall 14 no longer matched that branch either). The
        // post-5-T logcat's "post-execve syscall #34: nr=14 [unknown]"
        // made 5-W's VLM-verified UI E2E analysis immediate — but the
        // "[unknown]" label was still misleading for any reader who
        // didn't cross-reference against the kernel's UAPI header.
        // With this entry, syscall 14 on i386 is correctly labelled
        // "mknod" in the SIGSYS diagnostic log.
        "mknod"
    } else if nr == abi.chroot {
        "chroot"
    } else if nr == abi.mkdir {
        "mkdir"
    } else if nr == abi.chmod {
        "chmod"
    } else if nr == abi.unshare {
        "unshare"
    } else if nr == abi.shmget {
        "shmget"
    } else if nr == abi.shmat {
        "shmat"
    } else if nr == abi.shmctl {
        "shmctl"
    } else if nr == abi.pause {
        // Added in Task 6-D. Pre-6-D, i386 syscall 29 was labelled
        // "[unknown]" (after 6-C moved shmget from 29 to 395, syscall
        // 29 was no longer matched by any branch). The post-6-C logcat
        // showed "post-execve syscall #92: nr=29 [unknown]" + "NOTE:
        // unexpected SIGSYS for this syscall" 1,048,000+ times — that
        // is the infinite pause() retry loop. With this entry the
        // diagnostic label correctly says "pause" so the next person
        // debugging the loop can immediately identify it from the
        // SIGSYS log without cross-referencing against the kernel's
        // UAPI header.
        "pause"
    } else if nr == abi.fchown {
        "fchown"
    } else if nr == abi.fchmod {
        "fchmod"
    } else if nr == abi.lchown {
        "lchown"
    } else if nr == abi.chown {
        "chown"
    } else if nr == abi.fchmodat {
        "fchmodat"
    } else if nr == abi.fchownat {
        "fchownat"
    } else if nr == abi.capget {
        "capget"
    } else if nr == abi.ioprio_get {
        "ioprio_get"
    } else if nr == abi.ioprio_set {
        "ioprio_set"
    } else if nr == abi.setxattr {
        // Added in Task 6-R. Pre-6-R, i386 syscall 226 was labelled
        // "[unknown]" because no field matched it. With this entry the
        // diagnostic label correctly says "setxattr" so the next person
        // debugging the recovery SELinux-restorecon phase can
        // immediately identify it from the SIGSYS log without cross-
        // referencing against /usr/include/x86_64-linux-gnu/asm/
        // unistd_32.h.
        "setxattr"
    } else if nr == abi.lsetxattr {
        // Added in Task 6-R. Pre-6-R, i386 syscall 227 was labelled
        // "[unknown]" — this is THE syscall that caused the post-6-Q
        // UI E2E blocker (recovery retry loop at syscalls #123 + #135,
        // both nr=227 -> -EPERM, kr64 relaunching every 2s). With this
        // entry the diagnostic label correctly says "lsetxattr".
        "lsetxattr"
    } else if nr == abi.fsetxattr {
        // Added in Task 6-R (companion to setxattr/lsetxattr — same
        // SELinux-restorecon code path, just the fd-based variant).
        "fsetxattr"
    } else if nr == abi.stat64 {
        // Task 6-T: i386 64-bit-struct variant of stat (nr=195). Pre-6-T
        // these were labelled "[unknown]" in the diagnostic log — the
        // post-6-S logcat showed hundreds of "nr=195 -> -2 ENOENT"
        // lines that, without this label, required cross-referencing
        // against /usr/include/x86_64-linux-gnu/asm/unistd_32.h to
        // identify as stat64. This label makes the polling-loop root
        // cause immediately readable.
        "stat64"
    } else if nr == abi.lstat64 {
        // Task 6-T: i386 64-bit-struct variant of lstat (nr=196).
        // Companion to stat64 above.
        "lstat64"
    } else if nr == abi.fstat64 {
        // Task 6-T: i386 64-bit-struct variant of fstat (nr=197).
        // fstat64 takes an fd (not a path) so it is NOT in the path-
        // translation match arm — but we add the label here for
        // diagnostic logging.
        "fstat64"
    } else if nr == abi.execve {
        "execve"
    } else if nr == abi.unlink {
        // Task 6-Y: path-taking file-deletion syscall. Pre-6-Y these were
        // labelled "[unknown]" in the diagnostic log.
        "unlink"
    } else if nr == abi.unlinkat {
        "unlinkat"
    } else if nr == abi.socketcall_nr {
        // Task 6-Z3: i386 multiplexed socket syscall (nr=102).
        // Pre-6-Z3 this was labelled "[unknown]" in the diagnostic log
        // because no field matched it. With this entry the diagnostic
        // label correctly says "socketcall" so the next person
        // debugging the property-service bind EADDRINUSE can
        // immediately identify it from the SIGSYS log without cross-
        // referencing against /usr/include/x86_64-linux-gnu/asm/
        // unistd_32.h. NOTE: on x86_64 + aarch64 socketcall_nr is -1
        // (sentinel) so no real syscall ever matches this branch on
        // those ABIs.
        "socketcall"
    } else if nr == abi.poll_nr {
        // Task 6-Z5: legacy poll syscall (i386 nr=168, x86_64 nr=7).
        // Pre-6-Z5 this was labelled "[unknown]" because no field
        // matched it. With this entry the diagnostic label correctly
        // says "poll" so the next person debugging the property-service
        // POLLERR busy-wait (after the 6-Z3 bind fake-success masked
        // the EADDRINUSE but the socket isn't actually bound) can
        // immediately identify it from the EXIT log without cross-
        // referencing against /usr/include/x86_64-linux-gnu/asm/
        // unistd_32.h. NOTE: on aarch64 poll_nr is -1 (sentinel) so no
        // real syscall ever matches this branch on that ABI; the EXIT
        // handler's `== abi.poll_nr` comparison is also gated by
        // `abi.poll_nr != -1` so the branch is fully skipped on
        // aarch64.
        "poll"
    } else {
        "unknown"
    }
}

/// Human-readable label for a `ChildAbi` (used in log messages when
/// reporting the ABI transition around execve).
#[allow(unused_variables)]
fn abi_label(abi: ChildAbi) -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        if abi.execve == 59 {
            "64-bit (x86_64)"
        } else if abi.execve == 11 {
            "32-bit (i386 compat)"
        } else {
            "unknown-x86-abi"
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        // On aarch64 there is no 32-bit userspace, so the child is
        // always 64-bit. The `abi` parameter is unused on this target.
        "64-bit (aarch64)"
    }
}

/// Format the rolling "all syscalls" buffer (a `VecDeque<i64>` of raw
/// syscall numbers) as a comma-separated list of `nr=N [name]` entries
/// (oldest first). Used by the exit handlers in `run_ptrace_loop` to
/// print the last few syscalls the child made before dying.
///
/// `abi` is the (lazily-initialized) child ABI — if `None` (the child
/// died before any syscall-stop populated the ABI) we fall back to bare
/// `nr=N` formatting. `ChildAbi` is `Copy`, so the caller may pass
/// `abi` (an `Option<ChildAbi>`) by value without consuming the
/// caller's variable.
fn format_syscall_buffer(list: &std::collections::VecDeque<i64>, abi: Option<ChildAbi>) -> String {
    list.iter()
        .map(|&nr| match abi {
            Some(a) => {
                let name = syscall_name(nr, &a);
                if name == "unknown" {
                    format!("nr={}", nr)
                } else {
                    format!("nr={} [{}]", nr, name)
                }
            }
            None => format!("nr={}", nr),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

// ── Path translation ───────────────────────────────────────────────

/// Translate a guest path to a host path by prepending the rootfs.
pub fn translate_path(rootfs: &str, path: &str) -> String {
    if !path.starts_with('/') {
        return path.to_string();
    }
    // /proc/cmdline — translate to rootfs/twrp-cmdline so init can read
    // the (fake) kernel command line. The host's /proc/cmdline is not
    // readable by untrusted_app (EACCES from SELinux proc_cmdline label).
    // We pre-create {rootfs}/twrp-cmdline with appropriate content.
    // NOTE: we use {rootfs}/twrp-cmdline (NOT {rootfs}/proc/cmdline)
    // because the /proc directory in rootfs may not be writable (it's
    // created by the TWRP ramdisk extraction with restrictive perms).
    if path == "/proc/cmdline" {
        return format!("{}/twrp-cmdline", rootfs);
    }
    // /proc/, /data/, /apex/ — leave untranslated. The host's /proc
    // is partially readable by untrusted_app (so /proc/self/* works);
    // /data + /apex are either app-private (already accessible) or
    // not needed by TWRP init.
    //
    // NOTE (Task 6-P): /sys/ used to be in this list (left untranslated
    // → guest's open("/sys/...") hit the host's REAL kernel sysfs →
    // EACCES for untrusted_app → init exit(1) at ptrace iteration
    // ~3059). /sys/ was REMOVED from the untranslated list + moved to
    // a dedicated translated branch below (mirror of /dev/* handling).
    // The companion pre-creation in lib.rs::precreate_sysfs_stubs
    // materialises {rootfs}/sys/class/ + {rootfs}/sys/fs/selinux/{
    // enforce,load} so the translated opens succeed against an empty
    // fake sysfs instead of the host's real one.
    for prefix in &["/proc/", "/data/", "/apex/"] {
        if path.starts_with(prefix) {
            return path.to_string();
        }
    }
    // /dev/* — translate to rootfs/dev/* so init finds the pre-created
    // device stubs and files (e.g., /dev/.booting, /dev/__null__).
    // The host's /dev is read-only for untrusted_app, so opens of
    // /dev/* on the host fail with EACCES. By translating to rootfs/dev/,
    // init operates on the writable rootfs copy.
    //
    // Essential device files (null, urandom, zero, etc.) are pre-created
    // as symlinks to the host's /dev/* by the parent setup, so opens of
    // these still reach the real kernel devices.
    //
    // /dev/__properties__/* is now ALSO translated to rootfs (and
    // materialised there by the VFS layer — see vfs.rs::Vfs::materialize).
    // Previously this path was left untranslated so init's open() hit
    // the host's /dev/__properties__ (mode 0711, owned by root →
    // EACCES for untrusted_app), which caused a SIGSEGV in find_property
    // from an uninitialized property-area pointer. The SIGSEGV was being
    // suppressed by the find_property binary patch (lib.rs:3404-3485,
    // commits 9154e59+0a4be80+5d561cf) — that patch is removed in step 3
    // of the VFS rollout. With a valid property area materialised at
    // {rootfs}/dev/__properties__/properties_serial, find_property()
    // iterates over 0 properties and returns NULL naturally — no binary
    // mutation needed.
    if path.starts_with("/dev/") || path == "/dev" {
        return format!("{}{}", rootfs, path);
    }
    // /sys/* — translate to rootfs/sys/* so init's sysfs enumeration +
    // SELinux sysfs reads hit the pre-created FAKE sysfs (empty dirs +
    // empty /sys/fs/selinux/{enforce,load} files materialised by
    // lib.rs::precreate_sysfs_stubs). Without this translation, init's
    // open("/sys/class") + open("/sys/fs/selinux/{enforce,load}") hit
    // the host's REAL kernel sysfs, which untrusted_app can't read →
    // -EACCES → init exit(1) at ptrace iteration ~3059 (the NEW
    // post-56a5bd3 UI E2E blocker — Task 6-P).
    //
    // The fake sysfs is intentionally SPARSE — we only pre-create the
    // paths init is known to open (/sys/class, /sys/fs/selinux/*).
    // Opens of OTHER /sys/* paths will translate to {rootfs}/sys/* +
    // return -ENOENT (acceptable — init treats unknown sysfs entries
    // as "no such device" + proceeds, much better than -EACCES).
    //
    // SELinux note: this gives init a writable /sys/fs/selinux/load it
    // can write its policy blob to (the write succeeds silently against
    // a regular empty file — no kernel policy is actually loaded).
    // /sys/fs/selinux/enforce is pre-seeded with "0" (permissive) by
    // lib.rs::precreate_sysfs_stubs. Together these make SELinux appear
    // permissive to init — non-fatal for TWRP boot in the sandbox.
    if path.starts_with("/sys/") || path == "/sys" {
        return format!("{}{}", rootfs, path);
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

/// Read exactly `len` bytes from the traced child's memory starting at
/// `addr`, using `PTRACE_PEEKDATA` in word-sized chunks. Returns `None`
/// if the very first PEEK fails (EIO / unmapped address); returns a
/// possibly-shorter `Vec<u8>` if a later word fails (the partial read
/// up to the faulting address is still useful for diagnostics).
///
/// This is the N-byte variant of [`read_child_string`] — used by the
/// Task 6-U syscall-EXIT diagnostic to capture `write()` buffer
/// contents. KLOG lines (e.g. `<3>init: failed to parse init.rc\n`)
/// are NOT NUL-terminated within the write buffer (they end with
/// `\n`), so `read_child_string` would overshoot past the buffer's
/// intended length into adjacent memory. This helper reads exactly
/// the byte count the kernel reported in the write's return value.
///
/// The write() buffer lives in the child's WRITABLE memory (it's the
/// data the child just wrote), so PTRACE_PEEKDATA will succeed.
fn read_child_bytes(pid: libc::pid_t, addr: u64, len: usize) -> Option<Vec<u8>> {
    if addr == 0 || len == 0 {
        return Some(Vec::new());
    }
    let mut result = Vec::with_capacity(len);
    let mut offset = 0i64;
    while offset < len as i64 {
        let word = unsafe { libc::ptrace(libc::PTRACE_PEEKDATA, pid, addr as i64 + offset, 0) };
        if word == -1 {
            // First-PEEK failure → no data at all → treat as unreadable
            // (matches the existing `read_child_string` behaviour where
            // the loop breaks on the first -1). Later failures yield a
            // partial read (the bytes we DID get are still useful for
            // the diagnostic).
            if result.is_empty() {
                return None;
            }
            break;
        }
        let bytes = word.to_ne_bytes();
        let remaining = len - offset as usize;
        let take = std::cmp::min(bytes.len(), remaining);
        result.extend_from_slice(&bytes[..take]);
        offset += std::mem::size_of::<libc::c_long>() as i64;
    }
    Some(result)
}

/// Zero the `revents` field of every `struct pollfd` in the child's pollfd
/// array, so the caller sees "no events" in BOTH the syscall return value
/// (faked to 0 separately) AND the pollfd struct's revents field.
///
/// `struct pollfd { int fd; short events; short revents; }` is 8 bytes on
/// i386/x86_64 (sizeof = 8). `revents` is at offset 6 (2 bytes). The kernel
/// writes POLLERR (0x0008) into revents for the fake-bound property_service
/// socket BEFORE the EXIT stop fires, so even when we fake the return value
/// to 0 (Task 6-Z5), init's property_service poll loop sees POLLERR in
/// revents and retries immediately → busy-wait (verified on 5e0f157 E2E:
/// poll returns 1, faked to 0, but init keeps spinning — it checks revents).
///
/// We read-modify-write each 8-byte pollfd word: PEEKDATA the word, mask
/// bytes 6-7 (revents) to 0, POKEDATA it back. Capped at 32 fds to avoid
/// runaway (TWRP init's property_service poll uses 1 fd).
///
/// Returns the number of pollfd entries zeroed (for logging).
fn zero_pollfd_revents(pid: libc::pid_t, pollfd_ptr: u64, nfds: u64) -> usize {
    if pollfd_ptr == 0 || nfds == 0 {
        return 0;
    }
    let cap = std::cmp::min(nfds, 32) as usize;
    let mut zeroed = 0usize;
    for i in 0..cap {
        let addr = pollfd_ptr.wrapping_add((i as u64) * 8);
        let word = unsafe { libc::ptrace(libc::PTRACE_PEEKDATA, pid, addr as i64, 0) };
        if word == -1 {
            // PEEKDATA failed (bad addr or end of mapping) — stop.
            break;
        }
        // Little-endian: bytes 6-7 are the high 2 bytes of the 8-byte word.
        // Mask = 0x0000FFFFFFFFFFFF clears bits 48-63 (revents field).
        let masked = word & 0x0000FFFFFFFFFFFFi64;
        if masked != word {
            let r = unsafe {
                libc::ptrace(
                    libc::PTRACE_POKEDATA,
                    pid,
                    addr as i64,
                    masked as libc::c_long,
                )
            };
            if r == -1 {
                // POKEDATA failed — stop (don't keep trying a bad region).
                break;
            }
            zeroed += 1;
        }
    }
    zeroed
}

/// Classify whether an opened path is a kernel-message log destination
/// whose writes should be tagged `DIAG KLOG` (vs the generic `DIAG
/// write`) by the Task 6-U syscall-EXIT diagnostic. Matches:
///   - `/dev/__kmsg__` (TWRP init's KLOG destination — opened as fd 3
///     per the worklog 5-C twrp-init-fds.log analysis)
///   - `/dev/kmsg`     (the standard Linux kernel log destination)
///   - any ABSOLUTE path whose final component is `__kmsg__` (covers
///     path-translated variants like `{rootfs}/dev/__kmsg__` once
///     [`translate_path`] rewrites it, and the `__kmsg__` symlink that
///     gets "(deleted)" after tmpfs mount on /dev)
///
/// Pulled out as a free function so it is unit-testable independently
/// of the ptrace loop. Requires the path to be absolute (start with
/// '/') because real open() calls in TWRP init always use absolute
/// paths — relative paths are not produced by the open() ENTRY
/// handler's `read_child_string` in practice, and requiring absolute
/// keeps the matcher from spuriously matching unrelated relative
/// strings that happen to end in `__kmsg__`.
fn is_kmsg_path(path: &str) -> bool {
    if !path.starts_with('/') {
        return false;
    }
    if path == "/dev/__kmsg__" || path == "/dev/kmsg" {
        return true;
    }
    // Final path component == "__kmsg__" (covers {rootfs}/dev/__kmsg__
    // after translate_path rewrites /dev/__kmsg__, and the orphaned
    // "(deleted)" variants).
    path.rsplit('/').next() == Some("__kmsg__")
}

/// Classify an open() path as the Android property-area file
/// `/dev/__properties__`.
///
/// Matches:
///   - `/dev/__properties__` — the canonical path init opens.
///   - `{rootfs}/dev/__properties__` — after translate_path rewrites
///     `/dev/__properties__` to the host-side rootfs path.
///   - any path whose final component is `__properties__` (defensive —
///     covers the orphaned "(deleted)" variants + future translate_path
///     forms).
///
/// Rejects relative paths + paths whose final component is NOT exactly
/// `__properties__` (e.g. `/dev/__properties__foo`, `/dev/null`,
/// `/init.rc`). Mirrors `is_kmsg_path`'s structure for symmetry.
fn is_properties_path(path: &str) -> bool {
    if !path.starts_with('/') {
        return false;
    }
    if path == "/dev/__properties__" {
        return true;
    }
    // Final path component == "__properties__" (covers {rootfs}/dev/
    // __properties__ after translate_path rewrites /dev/__properties__,
    // and the orphaned "(deleted)" variants).
    path.rsplit('/').next() == Some("__properties__")
}

/// Pure, testable core of the Task 6-Y mmap2 MAP_SHARED → MAP_ANONYMOUS
/// flag rewrite.
///
/// TWRP init (i386) calls
///   `mmap2(NULL, 0x20000, PROT_READ|PROT_WRITE, MAP_SHARED, fd, 0)`
/// on `/dev/__properties__` to set up the property area. The Android
/// zygote's seccomp filter (inherited by untrusted_app) blocks file-
/// backed MAP_SHARED mmap2 for i386 compat → `-ENOSYS(-38)`. To get
/// the kernel to perform an anonymous mmap (which SUCCEEDS under the
/// zygote's filter), we rewrite `flags` to clear MAP_SHARED and set
/// MAP_ANONYMOUS|MAP_PRIVATE, then rewrite `fd` to -1 and `offset` to
/// 0. This helper does ONLY the flags portion — `fd` and `offset` are
/// constants handled inline at the call site.
///
/// Contract:
///   - The MAP_SHARED bit (0x01) is CLEARED.
///   - The MAP_ANONYMOUS bit (0x20) is SET.
///   - The MAP_PRIVATE bit (0x02) is SET.
///   - All OTHER bits the caller passed (e.g. MAP_FIXED, MAP_LOCKED,
///     PROT-readability carried via upper bits — though prot is a
///     separate arg) are PRESERVED.
///
/// Constants (per `<sys/mman.h>` + verified in libc):
///   MAP_SHARED     = 0x01
///   MAP_PRIVATE    = 0x02
///   MAP_ANONYMOUS  = 0x20  (also exposed as MAP_ANON — same value)
fn rewrite_mmap_flags_shared_to_anonymous(flags: i32) -> i32 {
    (flags & !libc::MAP_SHARED) | libc::MAP_ANONYMOUS | libc::MAP_PRIVATE
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
    // Task 6-Z14: check for overflow BEFORE writing. The scratch area is
    // 4096 bytes. If the translated path (including NUL) would extend past
    // the end, wrap to 0 first. This prevents the path from overflowing
    // into the child's stack frame (return addresses, saved registers),
    // which caused the SIGSEGV at rip=0x6f722f69 ('i/ro' from the rootfs
    // path '/data/user/0/io.twoyi/rootfs' leaking into a code pointer).
    let path_len = translated.len() + 1; // include NUL
    let aligned_len = (path_len + 7) & !7; // 8-byte aligned
    if *scratch_offset + aligned_len > 4096 {
        *scratch_offset = 0;
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

/// Write `0xFFFFFFFF` to all three fields of a `struct __user_cap_data_struct`
/// (effective, permitted, inheritable) at `addr` in the child's memory.
///
/// The struct is 12 bytes (3 × `u32`, see `<linux/capability.h>`):
/// ```c
/// struct __user_cap_data_struct {
///     __u32 effective;    // offset 0
///     __u32 permitted;    // offset 4
///     __u32 inheritable;  // offset 8
/// };
/// ```
///
/// We write `0xFFFFFFFF` to all three fields so init sees "all capabilities
/// granted" when it inspects the buffer after we forced capget to return 0.
/// Without this, the buffer stays zeroed (the kernel never wrote to it
/// because either seccomp aborted the syscall or the syscall returned
/// EPERM before writing) and init interprets "success but no caps" as a
/// fatal condition — exactly the bug we are fixing.
///
/// The host is always 64-bit (`x86_64` or `aarch64`), so `c_long` is 8
/// bytes and `PTRACE_POKEDATA` writes 8 bytes at a time. We need to
/// write 12 bytes, so:
///   - Word 1 (offset 0): effective + permitted (both `0xFFFFFFFF`) =
///     `0xFFFFFFFFFFFFFFFF` (= `-1` in two's complement).
///   - Word 2 (offset 8): inheritable (`0xFFFFFFFF`) at bytes 0-3, plus
///     the EXISTING bytes 4-7 preserved via read-modify-write to avoid
///     clobbering adjacent stack memory (which may be another local
///     variable, not just padding).
///
/// If `PTRACE_PEEKDATA` fails for word 2 (returns `-1` with `errno != 0`),
/// we fall back to writing `0xFFFFFFFFFFFFFFFF` directly. This clobbers
/// 4 bytes of adjacent memory — typically safe because the 4 bytes after
/// a 12-byte struct on a 16-byte-aligned stack are padding, but the
/// read-modify-write path is preferred for robustness.
///
/// `PTRACE_PEEKDATA` returns `long`, and `-1` is also a valid word value,
/// so we follow the standard `ptrace(2)` pattern: clear `errno` first,
/// then check `errno` after the call to disambiguate.
///
/// Returns `true` on success, `false` if `addr` is null or any
/// `PTRACE_POKEDATA` fails (which typically means `addr` is not a valid
/// mapped address in the child).
///
/// NOTE: this function is currently UNUSED. It was previously called from
/// the capget EXIT intercept to populate the `cap_user_data_t` buffer, but
/// the 8-byte `PTRACE_POKEDATA` write corrupted the child's stack and
/// caused SIGSEGV (signal 11). The call site was removed — we now just
/// fake return 0 and leave the buffer untouched. The function is retained
/// for reference / potential future use (e.g. a safer `process_vm_writev`
/// based implementation) but is intentionally not invoked.
#[allow(dead_code)]
fn poke_capget_data(pid: libc::pid_t, addr: u64) -> bool {
    if addr == 0 {
        return false;
    }
    // Word 1: effective (0xFFFFFFFF) + permitted (0xFFFFFFFF).
    // 0xFFFFFFFFFFFFFFFF in two's complement is just -1.
    let all_ones: libc::c_long = -1;
    let r1 = unsafe {
        libc::ptrace(
            libc::PTRACE_POKEDATA,
            pid,
            addr as i64,
            all_ones as libc::c_long,
        )
    };
    if r1 == -1 {
        return false;
    }
    // Word 2: inheritable (0xFFFFFFFF) at bytes 0-3, plus existing bytes
    // 4-7 preserved via read-modify-write. PTRACE_PEEKDATA returns -1 on
    // error, but -1 is also a valid word value — clear errno first, then
    // check errno after the call to disambiguate (see ptrace(2)).
    // On Android, __errno_location is not available via libc crate; use
    // std::io::Error::last_os_error() instead.
    let _ = std::io::Error::last_os_error(); // clear errno
    let existing = unsafe { libc::ptrace(libc::PTRACE_PEEKDATA, pid, addr as i64 + 8, 0) };
    let peek_err = std::io::Error::last_os_error();
    let peek_errno = peek_err.raw_os_error().unwrap_or(0);
    let word2: libc::c_long = if existing == -1 && peek_errno != 0 {
        // PEEKDATA genuinely failed — fall back to writing all-ones
        // directly. This clobbers 4 bytes of adjacent memory (typically
        // stack padding), which is safe in practice.
        all_ones
    } else {
        // Preserve existing high 4 bytes, set low 4 bytes to 0xFFFFFFFF.
        let mut bytes = existing.to_ne_bytes();
        bytes[0..4].copy_from_slice(&0xFFFFFFFFu32.to_ne_bytes());
        libc::c_long::from_ne_bytes(bytes)
    };
    let r2 = unsafe {
        libc::ptrace(
            libc::PTRACE_POKEDATA,
            pid,
            addr as i64 + 8,
            word2 as libc::c_long,
        )
    };
    if r2 == -1 {
        return false;
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

pub fn run_ptrace_loop(pid: libc::pid_t, rootfs: &str, vfs: &crate::vfs::Vfs) -> i32 {
    use std::io::Write;
    let log = |msg: &str| {
        let _ = writeln!(std::io::stderr(), "[KR64][ptrace] {}", msg);
    };

    log(&format!(
        "ptrace loop started for pid {} (rootfs={})",
        pid, rootfs
    ));

    // ── PTRACE_SETOPTIONS: trace forks + good-syscall-stops ──────────
    //
    // We set FOUR options here:
    //
    //   - PTRACE_O_TRACESYSGOOD (the only option set before Task 6-S):
    //     makes syscall-stops arrive as SIGTRAP|0x80 (so we can
    //     distinguish them from regular SIGTRAPs in `WSTOPSIG`).
    //
    //   - PTRACE_O_TRACEFORK | TRACECLONE | TRACEVFORK (Task 6-S): makes
    //     the kernel auto-attach us to every forked/clone'd/vfork'd
    //     child of `pid`. Each new child starts traced + stopped at its
    //     first instruction (a SIGSTOP). The PARENT is reported with a
    //     PTRACE_EVENT_FORK/CLONE/VFORK stop at the moment of the fork
    //     (status >> 16 == event number); we use PTRACE_GETEVENTMSG on
    //     the parent to read the new child's PID. Without these options
    //     the new child runs UNTRACED — which was the FUNDAMENTAL
    //     architectural gap fixed by Task 6-S: TWRP's init forks the
    //     recovery service during boot, and the recovery service is
    //     STATICALLY LINKED (LD_PRELOAD hook doesn't load), so its
    //     syscalls (open /dev/graphics/fb0, mmap, ioctl) went directly
    //     to the host kernel with NO interception → -ENOENT → recovery
    //     crashes → init exits(1) at iter 3605.
    //
    //   - PTRACE_O_EXITKILL (Task 6-S): if kr64 (the tracer) dies for
    //     any reason, the kernel kills every traced child. Without this
    //     a forked child (e.g. recovery) could outlive kr64 and run
    //     untraced — leaking a process that touches the host /dev tree.
    //     EXITKILL ensures clean teardown on kr64 crash.
    //
    //   - PTRACE_O_TRACEEXEC (Task 6-T): makes the kernel report a
    //     PTRACE_EVENT_EXEC stop when ANY traced child calls execve.
    //     This is a SEPARATE stop, distinct from the syscall-entry/exit
    //     stops of the execve syscall itself, and is delivered even in
    //     compat-mode (i386 child on x86_64 host) where syscall-stops
    //     for fork-family syscalls can be missed (DISPATCHER-FINAL-15).
    //     This may help catch the recovery service's execve of
    //     /sbin/recovery that is currently invisible.
    //
    //   - PTRACE_O_TRACEVFORKDONE (Task 6-T): makes the kernel report a
    //     PTRACE_EVENT_VFORK_DONE stop on the PARENT when a vforked
    //     child releases the parent (i.e. after the child execve's OR
    //     exits — the parent SUSPENDS until then). Without this option
    //     the parent's vfork return is just a regular syscall-exit-stop
    //     (or, in compat mode, may be missed entirely). This may help
    //     catch the vfork completion that's currently invisible
    //     (DISPATCHER-FINAL-15).
    //
    // The new child's ABI is the SAME as the parent's ABI at fork time
    // (init is i386 after its first execve → recovery is also i386).
    // If the child later calls execve (e.g. init forks, then the child
    // execve's /sbin/recovery), the existing execve-detection logic
    // (saw_execve / reset_abi_next at the top of run_ptrace_loop)
    // re-detects the ABI from /proc/<pid>/exe. So we do NOT need a
    // per-child ABI map — the single shared `abi: Option<ChildAbi>`
    // local correctly tracks whichever child is currently stopped,
    // because only one child makes syscalls at a time (init BLOCKS in
    // waitpid while the recovery service runs).
    let ptrace_opts: libc::c_int = (libc::PTRACE_O_TRACESYSGOOD
        | libc::PTRACE_O_TRACEFORK
        | libc::PTRACE_O_TRACECLONE
        | libc::PTRACE_O_TRACEVFORK
        | libc::PTRACE_O_TRACEVFORKDONE
        | libc::PTRACE_O_TRACEEXEC
        | libc::PTRACE_O_EXITKILL) as libc::c_int;
    let r = unsafe { libc::ptrace(libc::PTRACE_SETOPTIONS, pid, 0, ptrace_opts) };
    if r == -1 {
        let e = std::io::Error::last_os_error();
        log(&format!("PTRACE_SETOPTIONS failed: {}", e));
        return -1;
    }
    log("PTRACE_O_TRACESYSGOOD | TRACEFORK | TRACECLONE | TRACEVFORK | TRACEVFORKDONE | TRACEEXEC | EXITKILL set");

    let mut in_syscall = false;
    let mut pending_getpid = false;
    let mut loop_count: u64 = 0;

    // ── Task 6-Z9: xattr-SET syscall-rewrite state ───────────────────
    //
    // `pending_xattr_fake`: set at the ENTRY stop for setxattr /
    //   lsetxattr / fsetxattr (the *xattr SET family) when we rewrite
    //   the syscall number to `getpid` so the kernel executes a
    //   HARMLESS getpid instead of the xattr SET. As untrusted_app the
    //   kernel returns -EPERM / -EACCES / -EOPNOTSUPP for these xattr
    //   SETs (no CAP for security.* xattrs / filesystem doesn't
    //   support xattrs). The post-6-R EXIT-side fake
    //   (`compute_exit_return_value`) was SUPPOSED to overwrite rax=0
    //   at the EXIT stop, but the dispatcher's evidence on b712639 UI
    //   E2E run 32227786881 was inconclusive (the "faking success"
    //   log is gated by `loop_count <= 200`, which is long past by the
    //   time lsetxattr fires in the restorecon phase; the readback log
    //   is gated by `loop_count <= 300` — also past). The recovery
    //   was stuck in the restorecon loop (lstat64 → lsetxattr →
    //   gettid → open attr/current → read → close → repeat).
    //
    //   Task 6-Z9 takes a MORE DIRECT approach (the "syscall rewrite"
    //   pattern, mirroring how the SIGSYS handler conceptually rewrites
    //   blocked syscalls to getpid): at the ENTRY stop we rewrite
    //   orig_eax to `abi.getpid` BEFORE the kernel executes the syscall.
    //   The kernel then executes getpid (always succeeds, returns the
    //   PID) instead of the xattr SET (which would return EPERM/etc.).
    //   At the matching EXIT stop, `pending_xattr_fake` is consumed:
    //   we fake the return to 0 (success) regardless of what getpid
    //   returned. This is belt-and-suspenders with the existing
    //   `compute_exit_return_value` EXIT-side fake — if the ENTRY
    //   rewrite succeeds, `syscall_num` at EXIT is `getpid` (not the
    //   xattr number), so `compute_exit_return_value(getpid, abi)`
    //   returns None and the existing block is a no-op. If the ENTRY
    //   rewrite FAILS (ptrace_setregs error), `syscall_num` at EXIT is
    //   still the original xattr number, `compute_exit_return_value`
    //   returns Some(0), AND `pending_xattr_fake` ALSO fires — both
    //   try to set rax=0 (redundant but harmless — both write 0 to
    //   rax via fresh ptrace_getregs + set_syscall_ret + ptrace_setregs).
    //   The legacy `compute_exit_return_value` block's "faking success"
    //   log is gated by `loop_count <= 200` (long past by the time
    //   xattr fires), so in practice NO double-logging occurs.
    let mut pending_xattr_fake: bool = false;
    // Task 6-Z28: pending flag for poll() return fake. Set at the ENTRY
    // stop (when init calls poll), consumed at the EXIT stop (fake return 0).
    let mut pending_poll_fake: bool = false;

    // ── Task 6-U diagnostic state ────────────────────────────────────
    //
    // TWRP init writes 339 write() calls (5820 bytes) to /dev/__kmsg__
    // (its KLOG). kr64 copies that file to /sdcard but the test
    // harness never `adb pull`s it, so init's own diagnostic messages
    // (WHY it bails before parsing init.rc / forking recovery) are
    // invisible. The 6-U diagnostic captures the write() buffer
    // contents inline in the logcat so the KLOG is visible without
    // pulling /sdcard.
    //
    // `kmsg_fd`: the file descriptor the open() syscall returned for
    //   /dev/__kmsg__ (or /dev/kmsg). When a subsequent write() uses
    //   this fd, we prefix the log line with "DIAG KLOG" instead of
    //   "DIAG write" so init's KLOG lines are trivially greppable.
    //   Loop-local shared across all traced children — per the brief.
    //   The fd is process-local (different children have different fd
    //   tables), so if recovery also opens __kmsg__ the tracked fd
    //   will be overwritten with recovery's fd. This is an accepted
    //   limitation of the shared-state architecture (see worklog
    //   6-S/6-S3 — `current_pid` shadows init's pid, no per-pid map).
    //   The PRIMARY subject (init's KLOG) is captured before recovery
    //   runs, since init opens __kmsg__ early in its startup (well
    //   before forking recovery).
    // `pending_kmsg_open`: set at open()/openat()/openat2() ENTRY
    //   when the (translated) path matches is_kmsg_path(). Consumed
    //   + cleared at the matching open EXIT to record the returned
    //   fd in `kmsg_fd`. Mirrors the existing `pending_getpid` pattern
    //   (ENTRY-flag, EXIT-consume).
    // `post_execve_write_count`: gate counter for the write() EXIT
    //   diagnostic — only the first 800 post-execve writes are logged,
    //   to bound log volume if init spins. (Init does ~339 writes total
    //   per the strace, so 800 is a comfortable 2.4× headroom.)
    let mut kmsg_fd: Option<i32> = None;
    let mut pending_kmsg_open: bool = false;
    let mut post_execve_write_count: u64 = 0;

    // ── Task 6-V diagnostic state ────────────────────────────────────
    //
    // `post_execve_read_count`: gate counter for the read() EXIT
    //   diagnostic — only the first 800 post-execve reads are logged,
    //   to bound log volume. (Recovery does ~N reads before SIGSEGV,
    //   800 is a comfortable headroom.)
    // `open_fd_paths`: fd→(translated path) map for ALL open() calls.
    //   Set at open EXIT (ret > 0 → insert), used in the read() EXIT
    //   diagnostic to annotate which file was read.
    // `pending_open_translated_path`: set at open/openat/openat2 ENTRY
    //   with the translated path; consumed at the matching EXIT to
    //   insert into `open_fd_paths`. Mirrors the existing
    //   `pending_kmsg_open` pattern (ENTRY-flag, EXIT-consume).
    let mut post_execve_read_count: u64 = 0;
    let mut open_fd_paths: std::collections::HashMap<i32, String> =
        std::collections::HashMap::new();
    let mut pending_open_translated_path: Option<String> = None;

    // ── Task 6-Y: __properties__ fd tracking state ────────────────────
    //
    // TWRP init's property_init opens `/dev/__properties__` and mmaps
    // it with MAP_SHARED to set up the shared property area. The
    // Android zygote's seccomp filter (inherited by untrusted_app,
    // can't be removed) blocks file-backed MAP_SHARED mmap2 for i386
    // compat syscalls → -ENOSYS (-38). Anonymous mmap2 SUCCEEDS; only
    // the file-backed MAP_SHARED variant fails. Without the mmap
    // succeeding, the property area is not mapped → all 383
    // __system_property_set calls fail → init bails at iter 927 →
    // exit(1). TWRP never boots.
    //
    // `properties_fd`: the file descriptor open() returned for
    //   /dev/__properties__. Set at open EXIT when the translated path
    //   matches is_properties_path(). Consumed at the next mmap/mmap2
    //   ENTRY whose fd argument equals this value AND whose flags
    //   include MAP_SHARED — at that point we rewrite the mmap args to
    //   be anonymous (MAP_ANONYMOUS|MAP_PRIVATE, fd=-1, offset=0) so
    //   the kernel performs an anonymous mmap that succeeds. Mirrors
    //   the existing `kmsg_fd` pattern from Task 6-U (set at open EXIT,
    //   used by a later syscall-ENTRY arm). Init is the only process
    //   in the sandbox (no fork), so the single-Option-per-loop
    //   trade-off (overwriting if init closes + reopens the file) is
    //   acceptable — the latest fd is always the one the next mmap
    //   will use. See the doc on `mmap` / `mmap2` in `ChildAbi` for
    //   the full root-cause analysis.
    let mut properties_fd: Option<i32> = None;

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
    // Task 6-Z16: `scratch_offset` is re-zeroed at every syscall ENTRY
    // (the ENTRY block re-reserves the scratch area from the current sp).
    // The initial `0` here is therefore a dead write (overwritten at the
    // first ENTRY before any read), but Rust requires the binding be
    // initialized. `#[allow(unused_assignments)]` silences the clippy
    // lint for this known-dead init.
    #[allow(unused_assignments)]
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
    // ── SIGSYS log rate-limiting ──────────────────────────────────────
    //
    // When init hits a SIGSYS loop (e.g. the 0a4be80 E2E run where
    // shmget returned 0 → init retried because shmid=0 is invalid),
    // the SIGSYS handler emits 2+ log lines per iteration. At 3000+
    // iterations per second this floods logcat and OOMs the Java
    // FileLogger-Kr64Tee thread (which accumulates the tee'd output
    // into a single String before flushing — a single 155MB
    // String.getBytes() allocation was observed in that run).
    //
    // To prevent the OOM we rate-limit the per-SIGSYS log output:
    //   - Track the last SIGSYS syscall number and how many times it
    //     has repeated consecutively.
    //   - For the first 5 repetitions, log normally (so the operator
    //     still sees the diagnostic for any NEW syscall that starts
    //     looping — the first few iterations carry the useful
    //     "what changed?" context).
    //   - After 5 repetitions of the SAME syscall number, suppress
    //     ALL per-SIGSYS log output (the in_syscall DESYNC log, the
    //     access() raw-args log, and the per-syscall "intercepted
    //     SIGSYS" log).
    //   - Every 100 suppressed iterations, emit ONE summary log line
    //     so the operator can still see that the loop is ongoing and
    //     track its progress.
    //
    // The actual SIGSYS handling (return-value forcing, history
    // buffer push, ptrace_setregs) is NOT affected by suppression —
    // only the log() calls are gated. This ensures the emulated
    // return value (e.g. -ENOSYS for shmget) is still applied to the
    // child's registers on every iteration, so the loop terminates
    // once init acts on the -ENOSYS.
    let mut last_sigsys_nr: i64 = -1;
    let mut sigsys_repeat_count: u64 = 0;
    let mut sigsys_suppressed_total: u64 = 0;
    // ── Pause() consecutive-call counter (Task 6-G) ───────────────────
    //
    // Tracks CONSECUTIVE pause() SIGSYS calls so the SIGSYS handler can
    // return -ETIMEDOUT after PAUSE_TIMEOUT_THRESHOLD (50) retries
    // instead of -ENOSYS, to make init give up waiting for the missing
    // property service.
    //
    // Reset to 0 on every NON-pause SIGSYS (so the counter only counts
    // consecutive pause SIGSYS events). NOT reset on SIGTRAP|0x80 stops
    // — pause is ALWAYS seccomp-blocked, so a SIGTRAP|0x80 between two
    // pause SIGSYS events means init made real progress via an
    // unblocked syscall (e.g. openat, read, write). Letting the
    // counter carry over in that case is the right behaviour: if init
    // re-enters the pause loop after that forward progress, we still
    // want the timeout to fire eventually (the property service is
    // STILL not running). Resetting on every SIGTRAP|0x80 would let
    // init make trivial progress (e.g. a single getpid) between
    // batches of pauses and NEVER hit the timeout — defeating the fix.
    let mut pause_count: u32 = 0;
    // Rolling log of the last N syscall numbers — captures BOTH
    // seccomp-intercepted syscalls (recorded in the SIGSYS handler
    // below, because seccomp-blocked syscalls do NOT produce a
    // syscall-entry stop) AND unintercepted syscalls (recorded at the
    // syscall ENTRY stop). Used on child exit to print "the last few
    // syscalls the child made before dying", which complements
    // `recent_sigsys` (which only captures intercepted ones).
    //
    // When init dies with exit code 1 after a flurry of access()
    // probes, the last few UNintercepted syscalls are usually the
    // ones that returned the error init is reacting to (an openat()
    // that returned -ENOENT, an fstat() that returned -EBADF, …), so
    // logging them is the single most useful next diagnostic. The
    // cap (50) is sized to show the recovery's full cleanup/exit path
    // (Task 6-S: was 10 — bumped to 50 because the post-6-R recovery
    // runs 3281 iterations before exit(1), and the last 10 was too
    // short to see the context that led to exit_group(1). 50 captures
    // roughly the final mprotect/munmap/wait4 sequence + the execve-
    // time setup that preceded it, while still keeping the log line
    // readable — `format_syscall_buffer` joins with ", " so a 50-elem
    // buffer is one ~600-char line, well within logcat's 4 KiB cap).
    const RECENT_ALL_SYSCALLS_CAP: usize = 50;
    let mut recent_all_syscalls: std::collections::VecDeque<i64> =
        std::collections::VecDeque::with_capacity(RECENT_ALL_SYSCALLS_CAP);
    // Signal to deliver to the child on the next PTRACE_SYSCALL resume.
    // 0 means "don't deliver any signal". Non-zero values are set by
    // the signal-forwarding branch below so that the SINGLE
    // PTRACE_SYSCALL at the loop top can inject the signal —
    // having two PTRACE_SYSCALL calls (one in the handler, one at the
    // loop top) caused the second to return ESRCH because the child
    // was already running, which then made us return -1 prematurely.
    let mut resume_signal: libc::c_int = 0;

    // ── execve tracking for ABI re-detection ──────────────────────────
    //
    // The child ABI (`abi` below) is lazily detected by reading
    // /proc/<pid>/exe at the FIRST syscall stop. But the first syscall
    // stop happens BEFORE the child has called execve — the child is
    // still running kr64's own code (copying the init binary, writing
    // status messages, etc.). So /proc/<pid>/exe points to kr64 (the
    // x86_64 host binary), and the ABI is permanently locked to x86_64
    // — even after the child exec's a 32-bit i386 TWRP init binary.
    //
    // This causes every subsequent syscall number to be misinterpreted
    // (i386 mount=21 read as x86_64 access=21) and every register index
    // to be wrong (x86_64 rdi=14 used to read arg1, but on an i386
    // compat child rdi actually holds edi, the 5th arg — so the ptrace
    // emulator reads "mode=0755" instead of the mount source path).
    //
    // FIX: track execve explicitly. When we see an execve ENTRY, set
    // `saw_execve`. At the execve EXIT, set `reset_abi_next`. At the
    // top of the next loop iteration (before any handler runs), reset
    // `abi = None` so the NEXT syscall stop re-reads /proc/<pid>/exe
    // — which now points to the new binary (i386 init).
    //
    // We check the execve number for ALL ABIs the child could be at
    // that point (x86_64=59, i386=11, aarch64=221) so we catch execve
    // regardless of the child's current bitness.
    let mut saw_execve: bool = false;
    let mut reset_abi_next: bool = false;
    // True once we've processed the first execve's ABI reset. Used to
    // log the first N post-execve syscalls (which are the new binary's
    // own syscalls — the most useful diagnostic for "what does TWRP
    // init do after execve?").
    let mut past_first_execve: bool = false;
    // Task 6-Z42: flag set when we inject a 64-bit execve via the
    // scratch area. When the ptrace loop catches the next syscall
    // ENTRY (the 64-bit execve from the scratch area), we clear this
    // flag and let the 64-bit execve execute normally (it's not blocked
    // by the zygote's seccomp filter because the filter whitelists
    // x86_64 execve nr=59 but blocks i386 execve nr=11).
    let mut pending_64bit_execve: bool = false;
    let mut post_execve_syscall_count: u64 = 0;

    // ── Multi-child PID tracking (Task 6-S) ───────────────────────────
    //
    // `init_pid` is the PID of the ORIGINAL traced child (the `pid`
    // function parameter — kr64's forked init). When init exits we
    // return its exit code from `run_ptrace_loop`. When a FORKED child
    // (recovery, ueventd, thermald, …) exits, we log + continue the
    // loop — init is still running and we must keep tracing it until
    // it too exits.
    //
    // `current_pid` is the PID of whichever child we most recently
    // received a ptrace stop from. The loop-top PTRACE_SYSCALL resumes
    // THIS child (not always init), and the loop's `let pid = current_pid`
    // shadow makes the existing handler code (which uses `pid` for
    // ptrace_getregs / read_child_string / etc.) operate on the
    // currently-stopped child instead of always on init. `current_pid`
    // is updated every iteration by the `waitpid(-1)` call, which
    // receives stops from ANY traced child (init + every forked
    // descendant, thanks to PTRACE_O_TRACEFORK set above).
    let init_pid: libc::pid_t = pid;
    let mut current_pid: libc::pid_t = pid;

    loop {
        // ── Deferred ABI reset (after execve EXIT) ──
        //
        // If the previous iteration was an execve EXIT, reset `abi`
        // here so the CURRENT iteration's handler re-detects the
        // child's bitness from /proc/<pid>/exe (which now points to
        // the new binary). This runs BEFORE waitpid, so both the
        // SIGTRAP|0x80 path and the SIGSYS path will see abi=None
        // and re-detect.
        if reset_abi_next {
            if abi.is_some() {
                let prev_label = abi_label(abi.unwrap());
                log(&format!(
                    "execve completed — resetting ABI (was {}) to re-detect child bitness from /proc/{}/exe",
                    prev_label, pid
                ));
                abi = None;
            }
            reset_abi_next = false;
            past_first_execve = true;
            post_execve_syscall_count = 0;
            // CRITICAL: reset the scratch area. The scratch area was
            // allocated BELOW the child's stack pointer BEFORE execve
            // (when the child was kr64, x86_64). After execve, the
            // child becomes i386 (32-bit) and its stack pointer moves
            // to a 32-bit address. The old 64-bit scratch address
            // (e.g. 0x7fffc9356038) is OUTSIDE the i386 child's
            // 32-bit address space, so PTRACE_POKEDATA to that address
            // fails with EIO. This caused write_translated_path to
            // fail for EVERY post-execve open, making path translation
            // a no-op — init opened the UNTRANSLATED host paths
            // (/dev/.booting on host → EACCES, /dev/__null__ on host
            // → ENOENT).
            //
            // Task 6-Z16: scratch_addr + scratch_offset are now
            // re-reserved at EVERY syscall ENTRY (see the ENTRY block
            // below), so the post-execve reset is no longer needed for
            // correctness — the very next ENTRY stop will re-read the
            // (now 32-bit) sp and re-reserve. We keep the diagnostic
            // log only (no assignments — clippy would flag them as
            // dead writes, overwritten at the next ENTRY before read).
            if scratch_addr != 0 {
                log(&format!(
                    "execve completed — old scratch area {:#x} now stale (64-bit, pre-execve); will re-reserve at the next ENTRY stop from the new 32-bit sp",
                    scratch_addr
                ));
            }
        }

        // Continue the child to the next syscall entry/exit. This is
        // the ONLY PTRACE_SYSCALL in the loop — handlers below set
        // `resume_signal` (and `continue`) instead of resuming the
        // child themselves, so we never race the second ptrace call.
        //
        // Task 6-S: resume `current_pid` (the last-stopped child), NOT
        // always `pid` (init). After PTRACE_O_TRACEFORK was set, the
        // kernel auto-attaches us to forked children; the previous
        // iteration's `waitpid(-1)` may have returned a NON-init child
        // (e.g. the recovery service), and we must resume THAT child —
        // resuming init instead would leave the recovery service stuck
        // in its SIGSTOP forever, and init would never receive the
        // child-stop event that breaks it out of its own waitpid.
        let r = unsafe {
            libc::ptrace(
                libc::PTRACE_SYSCALL,
                current_pid,
                0,
                resume_signal as libc::c_long,
            )
        };
        // Reset for the next iteration — only set again if a
        // signal-delivery branch below populates it.
        resume_signal = 0;
        if r == -1 {
            let e = std::io::Error::last_os_error();
            // ESRCH = child already exited — not an error, just done.
            if e.raw_os_error() == Some(libc::ESRCH) {
                log(&format!(
                    "PTRACE_SYSCALL: child {} already exited (ESRCH)",
                    current_pid
                ));
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
                // Print the rolling ALL-syscalls history before reaping.
                // This complements `recent_sigsys` by also showing the
                // UNintercepted syscalls that ran in between the
                // SIGSYS interceptions — typically the openat()/stat()
                // call whose -ENOENT return value is what made init
                // decide to exit(1). Without this log we only ever
                // see the SIGSYS side of the picture.
                if !recent_all_syscalls.is_empty() {
                    log(&format!(
                        "last {} ALL syscalls before ESRCH (intercepted + unintercepted, oldest->newest): {}",
                        recent_all_syscalls.len(),
                        format_syscall_buffer(&recent_all_syscalls, abi)
                    ));
                } else {
                    log("no syscalls recorded in all-syscalls buffer before ESRCH");
                }
                // Try to reap the child to get its exit status.
                // Task 6-S: reap `current_pid` (the child we tried to
                // resume), NOT always `pid` (init). If a forked child
                // exited between iterations, this is what reaps it.
                let mut status: libc::c_int = 0;
                let waited = unsafe { libc::waitpid(current_pid, &mut status, libc::WNOHANG) };
                if waited == current_pid {
                    if libc::WIFEXITED(status) {
                        let code = libc::WEXITSTATUS(status);
                        log(&format!(
                            "ESRCH path: child {} exit code {}",
                            current_pid, code
                        ));
                        // Task 6-S: only init's exit terminates the
                        // loop. A forked child exiting via ESRCH is
                        // just reaped — init is still running.
                        if current_pid == init_pid {
                            return code;
                        }
                        log(&format!(
                            "ESRCH path: forked child {} reaped (code {}); init {} still running — continuing",
                            current_pid, code, init_pid
                        ));
                        // Re-sync current_pid back to init so the next
                        // loop iteration resumes init, not the just-
                        // reaped child (which would ESRCH again).
                        current_pid = init_pid;
                        continue;
                    }
                    if libc::WIFSIGNALED(status) {
                        let sig = libc::WTERMSIG(status);
                        log(&format!(
                            "ESRCH path: child {} killed by signal {}",
                            current_pid, sig
                        ));
                        if current_pid == init_pid {
                            return -sig;
                        }
                        log(&format!(
                            "ESRCH path: forked child {} reaped (signal {}); init {} still running — continuing",
                            current_pid, sig, init_pid
                        ));
                        current_pid = init_pid;
                        continue;
                    }
                }
                // Could not reap — if it's init, give up; otherwise
                // fall back to init and continue.
                if current_pid == init_pid {
                    return -1;
                }
                log(&format!(
                    "ESRCH path: forked child {} not reaped; falling back to init {} — continuing",
                    current_pid, init_pid
                ));
                current_pid = init_pid;
                continue;
            }
            log(&format!("PTRACE_SYSCALL failed: {}", e));
            return -1;
        }

        // Wait for ANY traced child to stop. Task 6-S: previously this
        // was `waitpid(pid, ...)` which only received stops from init.
        // With PTRACE_O_TRACEFORK the kernel auto-attaches us to every
        // forked child, and each new child's stops (SIGSTOP at attach,
        // then syscall-stops, then exit) are reported via `waitpid(-1)`.
        // The returned PID is the child that actually stopped — which
        // may differ from `current_pid` (the child we resumed) when a
        // NEW forked child stops before the resumed child does. We
        // update `current_pid` to the actually-waited child below.
        let mut status: libc::c_int = 0;
        let waited = unsafe { libc::waitpid(-1, &mut status, 0) };
        if waited == -1 {
            let e = std::io::Error::last_os_error();
            log(&format!("waitpid failed: {}", e));
            return -1;
        }
        // Update `current_pid` to the child that actually stopped. The
        // shadow `let pid = current_pid` below makes the existing
        // handler code (which uses `pid` for ptrace_getregs /
        // read_child_string / ptrace_setregs) operate on THIS child.
        current_pid = waited;
        // Shadow the function-parameter `pid` (init's PID) with
        // `current_pid` for the rest of this iteration. All the handler
        // code below — ptrace_getregs(pid, …), read_child_string(pid, …),
        // ptrace_setregs(pid, …), the SIGSEGV siginfo read, etc. —
        // uses `pid` and now correctly operates on the currently-stopped
        // child (init OR a forked child like the recovery service).
        // `init_pid` retains the original init PID for the exit-check
        // below ("is this init exiting, or a forked child?").
        let pid: libc::pid_t = current_pid;

        // Check if the child exited.
        if libc::WIFEXITED(status) {
            let code = libc::WEXITSTATUS(status);
            log(&format!(
                "child {} exited with code {} (after {} iterations)",
                pid, code, loop_count
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
            // Print the rolling ALL-syscalls history so we can see the
            // last few UNintercepted syscalls init made before exiting.
            // This is the single most useful diagnostic for narrowing
            // down WHY init gave up: the last unintercepted syscall
            // before exit(1) is typically the one whose error return
            // (e.g. openat() → -ENOENT) triggered the exit.
            if !recent_all_syscalls.is_empty() {
                log(&format!(
                    "last {} ALL syscalls before exit (intercepted + unintercepted, oldest->newest): {}",
                    recent_all_syscalls.len(),
                    format_syscall_buffer(&recent_all_syscalls, abi)
                ));
            } else {
                log("no syscalls recorded in all-syscalls buffer before exit");
            }
            // Task 6-S: only init's exit terminates `run_ptrace_loop`.
            // A forked child (recovery, ueventd, thermald, …) exiting is
            // expected behaviour — init is still running and may be
            // blocked in waitpid waiting for the just-exited child. Log
            // the exit and continue the loop so we can resume init
            // (which will receive the waitpid return + run its own exit
            // path, eventually terminating the loop). Re-sync
            // `current_pid` to init so the next iteration resumes init.
            if pid == init_pid {
                return code;
            }
            log(&format!(
                "forked child {} exited with code {} — init {} still running, continuing loop",
                pid, code, init_pid
            ));
            current_pid = init_pid;
            continue;
        }
        if libc::WIFSIGNALED(status) {
            let sig = libc::WTERMSIG(status);
            log(&format!(
                "child {} killed by signal {} (after {} iterations)",
                pid, sig, loop_count
            ));
            if !recent_sigsys.is_empty() {
                let collected: Vec<String> = recent_sigsys.iter().cloned().collect();
                log(&format!(
                    "last {} SIGSYS-intercepted syscalls before kill (oldest->newest): {:?}",
                    collected.len(),
                    collected
                ));
            }
            // Print the rolling ALL-syscalls history on signal death
            // too — same rationale as the WIFEXITED branch.
            if !recent_all_syscalls.is_empty() {
                log(&format!(
                    "last {} ALL syscalls before kill (intercepted + unintercepted, oldest->newest): {}",
                    recent_all_syscalls.len(),
                    format_syscall_buffer(&recent_all_syscalls, abi)
                ));
            } else {
                log("no syscalls recorded in all-syscalls buffer before kill");
            }
            // Task 6-S: same forked-child handling as WIFEXITED above.
            if pid == init_pid {
                return -sig;
            }
            log(&format!(
                "forked child {} killed by signal {} — init {} still running, continuing loop",
                pid, sig, init_pid
            ));
            current_pid = init_pid;
            continue;
        }

        // Check if the child was stopped by a signal.
        if libc::WIFSTOPPED(status) {
            let sig = libc::WSTOPSIG(status);

            // ── Task 6-S: ptrace event stops (fork/clone/vfork/exec/exit) ──
            //
            // When PTRACE_O_TRACEFORK | TRACECLONE | TRACEVFORK is set
            // (see the PTRACE_SETOPTIONS call at the top of
            // `run_ptrace_loop`), the kernel reports fork-family events
            // as a special kind of SIGTRAP stop: WSTOPSIG returns
            // SIGTRAP, and the upper bits of `status` carry the event
            // number (status >> 16). For syscall-stops WSTOPSIG
            // returns SIGTRAP|0x80, so the `sig == (SIGTRAP|0x80)`
            // branch below already filters those out. The branch below
            // catches the remaining SIGTRAP-family stops (event stops
            // AND regular SIGTRAPs from breakpoints/single-step).
            //
            // We must handle the event stops BEFORE the regular
            // SIGTRAP branch, because both have WSTOPSIG == SIGTRAP.
            // Without this branch a fork event would be treated as a
            // "regular SIGTRAP" and silently swallowed — the new child
            // would still be auto-attached (PTRACE_O_TRACEFORK does
            // that independently of the tracer's response), but we
            // would have no diagnostic logging + no PTRACE_GETEVENTMSG
            // call to surface the new child's PID.
            //
            // status layout for an event stop:
            //   bits 0-6   : 0x7f (WIFSTOPPED marker)
            //   bits 8-15  : SIGTRAP (the "signal" — always SIGTRAP for
            //                event stops)
            //   bits 16-31 : ptrace event number (1=FORK, 2=VFORK,
            //                3=CLONE, 4=EXEC, 5=VFORK_DONE, 6=EXIT)
            // We extract the event number and dispatch on it. For
            // FORK/CLONE/VFORK we use PTRACE_GETEVENTMSG on the parent
            // to read the new child's PID — purely diagnostic, since
            // the kernel auto-attaches us to the new child regardless.
            // DIAGNOSTIC (6-R): log every SIGTRAP-family stop (with and without 0x80)
            // to diagnose why PTRACE_EVENT_FORK is never observed.
            if sig == libc::SIGTRAP {
                log(&format!(
                    "SIGTRAP stop (no 0x80) on pid={}: status=0x{:08x}, ptrace_event={}",
                    pid,
                    status as u32,
                    (status as u32) >> 16
                ));
            }

            let ptrace_event: u32 = ((status as u32) >> 16) & 0xFFFF;
            if ptrace_event != 0 {
                match ptrace_event {
                    ev if ev == libc::PTRACE_EVENT_FORK as u32
                        || ev == libc::PTRACE_EVENT_VFORK as u32
                        || ev == libc::PTRACE_EVENT_CLONE as u32 =>
                    {
                        // Parent (== `pid` here, the currently-stopped
                        // child) just forked/clone'd/vfork'd. Read the
                        // new child's PID via PTRACE_GETEVENTMSG for
                        // diagnostic logging. The kernel has ALREADY
                        // auto-attached us to the new child — we will
                        // receive its first stop (a SIGSTOP) via a
                        // future `waitpid(-1)` call, and the existing
                        // SIGSTOP / SIGTRAP|0x80 / SIGSYS handling will
                        // process its syscalls with the SAME per-child
                        // state vars (in_syscall, abi, scratch_addr,
                        // etc.). This works because only ONE child
                        // makes syscalls at a time: after init forks
                        // the recovery service, init BLOCKS in its own
                        // waitpid waiting for recovery to exit, so the
                        // shared state is never raced between two
                        // running children.
                        let mut new_child_id: libc::c_long = 0;
                        let getevent_r = unsafe {
                            libc::ptrace(
                                libc::PTRACE_GETEVENTMSG,
                                pid,
                                0,
                                &mut new_child_id as *mut _ as libc::c_long,
                            )
                        };
                        let event_name = if ev == libc::PTRACE_EVENT_FORK as u32 {
                            "FORK"
                        } else if ev == libc::PTRACE_EVENT_VFORK as u32 {
                            "VFORK"
                        } else {
                            "CLONE"
                        };
                        if getevent_r == 0 {
                            log(&format!(
                                "PTRACE_EVENT_{}: parent {} forked — new child PID {} (auto-attached by kernel; will receive its stops via waitpid(-1))",
                                event_name, pid, new_child_id
                            ));
                        } else {
                            log(&format!(
                                "PTRACE_EVENT_{}: parent {} forked — PTRACE_GETEVENTMSG failed: {} (new child is still auto-attached; will receive its stops via waitpid(-1))",
                                event_name,
                                pid,
                                std::io::Error::last_os_error()
                            ));
                        }
                        // Continue the parent — it should proceed to
                        // its waitpid() (or whatever it does after the
                        // fork) so the new child can run. resume_signal
                        // is 0 (no signal to deliver). The loop-top
                        // PTRACE_SYSCALL will resume `current_pid` (the
                        // parent) on the next iteration.
                        continue;
                    }
                    ev if ev == libc::PTRACE_EVENT_EXEC as u32 => {
                        // ── Task 6-T: PTRACE_EVENT_EXEC (event 4) ──
                        //
                        // A traced child just called execve (the new
                        // program is now loaded into the child's image).
                        // This is a SEPARATE ptrace stop, distinct from
                        // the syscall-entry/exit stops of the execve
                        // syscall itself — it's delivered after the
                        // execve has fully completed (the old image is
                        // gone, the new image is mapped). It's delivered
                        // even in compat-mode (i386 child on x86_64 host)
                        // where syscall-stops for fork-family + exec
                        // syscalls can be missed (DISPATCHER-FINAL-15).
                        //
                        // We use this for TWO purposes:
                        //   1. Diagnostic — log that the child execve'd
                        //      (helps trace the recovery service's
                        //      execve of /sbin/recovery that's currently
                        //      invisible).
                        //   2. Defensive ABI reset — even if our normal
                        //      saw_execve / reset_abi_next path already
                        //      handled the execve EXIT stop, set the
                        //      reset flag here too in case the syscall-
                        //      EXIT stop was skipped (which is exactly
                        //      the DISPATCHER-FINAL-15 hypothesis). This
                        //      guarantees we re-detect the ABI from
                        //      /proc/<pid>/exe on the next syscall-stop.
                        //
                        // CRITICAL: reset `in_syscall = false` here —
                        // the execve syscall-exit stop will NOT fire
                        // (the EXEC event replaces it). If we left
                        // `in_syscall = true`, the NEXT syscall stop
                        // would be misinterpreted as an EXIT instead of
                        // an ENTRY, completely breaking syscall
                        // emulation for the new image.
                        //
                        // PTRACE_GETEVENTMSG on EXEC returns the tracee's
                        // PID prior to the execve — for a non-vfork
                        // execve this is the same PID; for a vfork+execve
                        // in a child it's the parent's PID (the new child
                        // took over the parent's address space until
                        // vfork_done). Logged for diagnostics.
                        let mut prev_pid: libc::c_long = 0;
                        let getevent_r = unsafe {
                            libc::ptrace(
                                libc::PTRACE_GETEVENTMSG,
                                pid,
                                0,
                                &mut prev_pid as *mut _ as libc::c_long,
                            )
                        };
                        if getevent_r == 0 {
                            log(&format!(
                                "PTRACE_EVENT_EXEC: child {} execve'd (prev PID {}) — new image loaded; resetting in_syscall + ABI",
                                pid, prev_pid
                            ));
                        } else {
                            log(&format!(
                                "PTRACE_EVENT_EXEC: child {} execve'd (PTRACE_GETEVENTMSG failed) — new image loaded; resetting in_syscall + ABI",
                                pid
                            ));
                        }
                        // Defensive: arm the deferred ABI reset so the
                        // next syscall-stop re-detects bitness from
                        // /proc/<pid>/exe (which now points to the new
                        // binary). If the normal execve-EXIT path already
                        // armed it, this is a harmless no-op.
                        reset_abi_next = true;
                        // CRITICAL: the execve syscall-exit stop is
                        // suppressed by the EXEC event, so we MUST
                        // clear `in_syscall` or the next stop is
                        // misclassified as a syscall-exit.
                        in_syscall = false;
                        continue;
                    }
                    ev if ev == libc::PTRACE_EVENT_VFORK_DONE as u32 => {
                        // ── Task 6-T: PTRACE_EVENT_VFORK_DONE (event 5) ──
                        //
                        // The PARENT (== `pid` here) just RESUMED after a
                        // vforked child released it (the vforked child
                        // either execve'd OR exited). Without
                        // PTRACE_O_TRACEVFORKDONE this stop is NOT
                        // delivered — the parent's vfork syscall-exit
                        // would be just a regular syscall-exit-stop, BUT
                        // in compat mode (i386 child on x86_64 host) that
                        // exit-stop can be missed (DISPATCHER-FINAL-15),
                        // leaving the parent SUSPENDED forever (and the
                        // vforked child runs untraced until it execve's
                        // or exits). With TRACEVFORKDONE we get this
                        // explicit "vfork done" stop, which we use for:
                        //   1. Diagnostic — log that the vfork completed
                        //      (helps surface the currently-invisible
                        //      vfork path).
                        //   2. Defensive `in_syscall = false` reset —
                        //      the vfork syscall-exit stop will NOT fire
                        //      (the VFORK_DONE event replaces it). If we
                        //      left `in_syscall = true`, the NEXT syscall
                        //      stop would be misinterpreted as an EXIT
                        //      instead of an ENTRY.
                        //
                        // No PTRACE_GETEVENTMSG payload is documented for
                        // VFORK_DONE (the new child PID was already
                        // reported at the preceding PTRACE_EVENT_VFORK
                        // stop). We just log + continue the parent.
                        log(&format!(
                            "PTRACE_EVENT_VFORK_DONE: parent {} resumed after vforked child released it (child execve'd or exited) — resetting in_syscall",
                            pid
                        ));
                        // CRITICAL: the vfork syscall-exit stop is
                        // suppressed by the VFORK_DONE event, so we
                        // MUST clear `in_syscall` or the next stop is
                        // misclassified as a syscall-exit.
                        in_syscall = false;
                        continue;
                    }
                    ev if ev == libc::PTRACE_EVENT_EXIT as u32 => {
                        // The child is about to exit (PTRACE_O_TRACEEXIT
                        // would be needed to receive the actual exit
                        // event stop, but PTRACE_O_EXITKILL alone gives
                        // us this PTRACE_EVENT_EXIT report too on most
                        // kernels). Read the pending exit status for
                        // diagnostic logging. The actual WIFEXITED/
                        // WIFSIGNALED report comes from the next
                        // waitpid — this is just an early heads-up.
                        let mut exit_status: libc::c_long = 0;
                        let getevent_r = unsafe {
                            libc::ptrace(
                                libc::PTRACE_GETEVENTMSG,
                                pid,
                                0,
                                &mut exit_status as *mut _ as libc::c_long,
                            )
                        };
                        if getevent_r == 0 {
                            log(&format!(
                                "PTRACE_EVENT_EXIT: child {} about to exit (pending status 0x{:x})",
                                pid, exit_status
                            ));
                        } else {
                            log(&format!(
                                "PTRACE_EVENT_EXIT: child {} about to exit (PTRACE_GETEVENTMSG failed)",
                                pid
                            ));
                        }
                        continue;
                    }
                    ev => {
                        // Unknown ptrace event — log + continue. Don't
                        // deliver a signal (would corrupt the child).
                        log(&format!(
                            "unknown PTRACE_EVENT {} on child {} — continuing without signal delivery",
                            ev, pid
                        ));
                        continue;
                    }
                }
            }

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

                    // Record the syscall number in the rolling "all
                    // syscalls" buffer. This captures UNintercepted
                    // syscalls — seccomp-blocked syscalls are recorded
                    // separately in the SIGSYS handler below (they do
                    // NOT produce a syscall-entry stop, only a SIGSYS
                    // stop followed by the syscall-exit stop of the
                    // rewritten getpid), so without the SIGSYS-side
                    // recording they'd be missing from this buffer.
                    // We push at ENTRY (not EXIT) so the buffer reflects
                    // what init ASKED for, in the order init asked.
                    if recent_all_syscalls.len() == RECENT_ALL_SYSCALLS_CAP {
                        recent_all_syscalls.pop_front();
                    }
                    recent_all_syscalls.push_back(syscall_num);

                    // ── DIAGNOSTIC (6-S3): unconditional fork-family + wait4 logging ──
                    //
                    // bceac63 showed ZERO PTRACE_EVENT_FORK/CLONE/VFORK despite
                    // the options being set. init's last-10 buffer has wait4
                    // (nr=114 i386) — so init DID wait4. This answers: does
                    // init actually CALL fork/clone/vfork/clone3, or does it
                    // skip forking entirely (→ wait4 -ECHILD → exit(1))?
                    //
                    // Raw numbers for BOTH i386 + x86_64 ABIs (init is i386
                    // post-execve; the twoyi-app restart path is x86_64
                    // pre-execve). UNCONDITIONAL — NOT gated by loop_count.
                    let is_fork_family = matches!(syscall_num, 2 | 57 | 120 | 56 | 190 | 58 | 435);
                    let is_wait4 = matches!(syscall_num, 114 | 61 | 247 | 290);
                    if is_fork_family {
                        log(&format!(
                            "DIAG fork-family ENTRY: nr={} (pid={}), loop_count={}, in_syscall_was={}",
                            syscall_num, pid, loop_count, in_syscall
                        ));
                    }
                    if is_wait4 {
                        let wait_pid = get_syscall_arg(&regs, abi.reg_arg1) as i64;
                        log(&format!(
                            "DIAG wait4 ENTRY: nr={}, wait_pid={} (0=any, -1=any-block, >0=specific), loop_count={}",
                            syscall_num, wait_pid, loop_count
                        ));
                    }

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

                    // ── execve detection (ABI re-detection trigger) ──
                    //
                    // When the child calls execve, its memory image is
                    // about to be replaced. /proc/<pid>/exe currently
                    // still points to the OLD binary, but at execve EXIT
                    // it will point to the NEW binary. We set `saw_execve`
                    // here so the EXIT handler can schedule an ABI reset.
                    //
                    // We compare against `abi.execve` (the current ABI's
                    // execve number). This is correct because:
                    //   - Before the first execve, the child is kr64
                    //     (same bitness as host), so abi.execve matches
                    //     the host's native execve number.
                    //   - After the first execve (and ABI re-detection),
                    //     abi.execve matches the new binary's execve
                    //     number, so a second execve (e.g. init →
                    //     recovery) is also caught.
                    if syscall_num == abi.execve {
                        saw_execve = true;
                        log(&format!(
                            "execve ENTRY (nr={}) — will reset ABI after EXIT to re-detect child bitness",
                            syscall_num
                        ));

                        // Task 6-Z42: inject a 64-bit execve to bypass seccomp.
                        // The zygote's seccomp filter blocks i386 execve (nr=11)
                        // → returns -38 (ENOSYS). But it ALLOWS x86_64 execve
                        // (nr=59) — init's own first execve (nr=59) succeeds.
                        //
                        // FIX: at the i386 execve ENTRY, skip the i386 syscall
                        // (set orig_rax=-1), write a `syscall` instruction
                        // (0x0f 0x05) to the scratch area, set the x86_64
                        // registers (rax=59, rdi=path, rsi=argv, rdx=envp),
                        // set rip to the scratch area, and resume. The child
                        // executes the `syscall` instruction → 64-bit execve
                        // (nr=59) → not blocked by seccomp → SUCCESS.
                        //
                        // The `syscall` instruction (0x0f 0x05) always uses
                        // the x86_64 syscall table on a 64-bit kernel,
                        // regardless of the current CS register. So even though
                        // the child is in 32-bit compat mode, the `syscall`
                        // instruction bypasses the i386 seccomp filter.
                        if abi.execve == 11 && scratch_addr != 0 {
                            // Read path, argv, envp from i386 regs
                            let path_addr_i386 = get_syscall_arg(&regs, abi.reg_arg1);
                            let argv_addr_i386 = get_syscall_arg(&regs, abi.reg_arg2);
                            let envp_addr_i386 = get_syscall_arg(&regs, abi.reg_arg3);

                            // Translate the path
                            let orig_path = if path_addr_i386 != 0 {
                                read_child_string(pid, path_addr_i386)
                            } else {
                                None
                            };

                            if let Some(ref orig) = orig_path {
                                let translated = translate_path(rootfs, orig);

                                // Write the translated path to the scratch area
                                // (NUL-terminated)
                                let path_bytes = format!("{}\0", translated);
                                let path_len = path_bytes.len();
                                // 8-byte align
                                let aligned_len = (path_len + 7) & !7;
                                let code_offset = aligned_len;

                                // Write path + padding + syscall instruction + int3
                                let mut scratch_content = path_bytes.into_bytes();
                                // Pad to 8-byte alignment
                                while scratch_content.len() < code_offset {
                                    scratch_content.push(0);
                                }
                                // syscall instruction (0x0f 0x05) + int3 (0xcc)
                                scratch_content.push(0x0f);
                                scratch_content.push(0x05);
                                scratch_content.push(0xcc);

                                // Write to child's memory
                                let mut written = false;
                                for (offset, &byte) in scratch_content.iter().enumerate() {
                                    let word = byte as libc::c_long;
                                    let r = unsafe {
                                        libc::ptrace(
                                            libc::PTRACE_POKEDATA,
                                            pid,
                                            (scratch_addr + offset as u64) as i64,
                                            word,
                                        )
                                    };
                                    if r == -1 {
                                        log(&format!(
                                            "DIAG 64-bit execve: POKEDATA failed at offset {} (Task 6-Z42)",
                                            offset
                                        ));
                                        break;
                                    }
                                    written = offset == scratch_content.len() - 1;
                                }

                                if written {
                                    // Task 6-Z42 fix: the syscall instruction
                                    // was written to the scratch area (stack)
                                    // which is NX (non-executable) → SIGSEGV.
                                    // FIX: write the syscall instruction (0x0f 0x05)
                                    // to the init binary's .text section at
                                    // vaddr 0x08048c59 (the 6-Z19 NOP area,
                                    // which is r-xp executable + unused).
                                    // The translated PATH stays on the stack
                                    // (readable, not executable — that's fine).
                                    let code_addr: u64 = 0x08048c59;
                                    let syscall_word: libc::c_long = 0x00cc050f; // LE bytes: 0f 05 cc 00 (syscall + int3 + pad)
                                    let r2 = unsafe {
                                        libc::ptrace(libc::PTRACE_POKEDATA, pid, code_addr as i64, syscall_word)
                                    };
                                    if r2 == -1 {
                                        log("DIAG 64-bit execve: POKEDATA to .text FAILED (Task 6-Z42)");
                                    } else {
                                        let mut regs2: Regs = unsafe { std::mem::zeroed() };
                                        match ptrace_getregs(pid, &mut regs2) {
                                            Ok(len2) => {
                                                // x86_64 user_regs_struct indices:
                                                // 10=rax, 12=rdx, 13=rsi, 14=rdi,
                                                // 15=orig_rax, 16=rip, 17=cs, 20=ss
                                                set_syscall_arg(&mut regs2, 14, scratch_addr);          // rdi = path (on stack, readable)
                                                set_syscall_arg(&mut regs2, 13, argv_addr_i386);       // rsi = argv
                                                set_syscall_arg(&mut regs2, 12, envp_addr_i386);       // rdx = envp
                                                set_syscall_arg(&mut regs2, 10, 59);                   // rax = 59 (x86_64 execve)
                                                set_syscall_arg(&mut regs2, 16, code_addr);            // rip = syscall instruction in .text
                                                set_syscall_arg(&mut regs2, 15, (-1i64) as u64);      // orig_rax = -1 (skip i386)
                                                set_syscall_arg(&mut regs2, 17, 0x33);                 // cs = 0x33 (64-bit code segment — switches to 64-bit mode so 'syscall' uses x86_64 table, bypassing i386 seccomp)
                                                set_syscall_arg(&mut regs2, 20, 0x2b);                 // ss = 0x2b (64-bit data segment)
                                                match ptrace_setregs(pid, &regs2, len2) {
                                                    Ok(()) => {
                                                        pending_64bit_execve = true;
                                                        log(&format!(
                                                            "DIAG 64-bit execve INJECTED: path='{}' → '{}' (path at {:#x}, syscall at {:#x}) — bypassing i386 seccomp block (Task 6-Z42)",
                                                            orig, translated, scratch_addr, code_addr
                                                        ));
                                                    }
                                                    Err(e) => log(&format!(
                                                        "DIAG 64-bit execve: ptrace_setregs FAILED: {} (Task 6-Z42)", e
                                                    )),
                                                }
                                            }
                                            Err(e) => log(&format!(
                                                "DIAG 64-bit execve: ptrace_getregs FAILED: {} (Task 6-Z42)", e
                                            )),
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Task 6-Z42: when the 64-bit execve from the scratch
                    // area fires (after our injection), clear the pending
                    // flag and let it execute without path translation
                    // (the path was already translated when we injected it).
                    if pending_64bit_execve {
                        pending_64bit_execve = false;
                        log("DIAG 64-bit execve: ENTRY caught from scratch area — letting it execute (Task 6-Z42)");
                    }

                    // Log the first 50 post-execve syscalls so we can
                    // see exactly what the new binary (TWRP init) does
                    // after execve. This is critical because loop_count
                    // may already be large (kr64's pre-execve syscalls
                    // inflate it), so the existing "loop_count <= 50"
                    // log gate would suppress these.
                    //
                    // Task 6-S: increased from 150 to 5000 to capture the
                    // full recovery phase (recovery runs ~3281 iterations
                    // before exit(1); 150 hid the middle phase where
                    // fork/clone attempts + the exit trigger live).
                    if past_first_execve {
                        post_execve_syscall_count = post_execve_syscall_count.saturating_add(1);
                        // Task 6-Z26: raised from 200 to 500 to see what init
                        // does after the SELinux/restorecon phase. The file
                        // was static at 86540 bytes (#200) for 16+ sec — init
                        // is BLOCKED on something past #200 that we can't see.
                        // 500 shows syscalls #201-#500 (~200KB file, manageable
                        // for the tee). This will reveal what init is blocked on
                        // (poll/futex/read on a socket/FIFO).
                        if post_execve_syscall_count <= 500 {
                            log(&format!(
                                "post-execve syscall #{}: nr={} [{}]",
                                post_execve_syscall_count,
                                syscall_num,
                                syscall_name(syscall_num, &abi)
                            ));
                        }
                    }

                    // Task 6-S: always log fork/clone/vfork/wait4/exit_group
                    // (critical for diagnosing the recovery exit(1) — these
                    // reveal whether the guest attempts to fork services +
                    // what wait4 returns). Not gated by the 5000 post-execve
                    // cap so we never miss them even past iteration 5000.
                    // The bceac63 diagnostic showed ZERO fork/clone/vfork
                    // calls in the entire visible logcat (neither i386
                    // nr=2/120/190 nor x86_64 nr=56/57/58) — but the post-
                    // execve logging was capped at 150, hiding the middle
                    // phase (iters 151-3271) where these attempts would
                    // have lived. This block ensures we ALWAYS see them.
                    //
                    // NOTE: this fires on EVERY ENTRY stop (not just post-
                    // execve), so kr64's own pre-execve fork/clone calls
                    // (if any) are also captured. The post-6-R recovery's
                    // exit(1) at iter 3281 with last-10 ALL syscalls
                    // including wait4 + exit_group but ZERO fork/clone/
                    // vfork is the immediate target of this log.
                    {
                        let nr = syscall_num;
                        if nr == abi.clone_nr
                            || nr == abi.fork_nr
                            || nr == abi.vfork_nr
                            || nr == abi.wait4_nr
                            || nr == abi.exit_group_nr
                        {
                            let name = if nr == abi.clone_nr {
                                "clone"
                            } else if nr == abi.fork_nr {
                                "fork"
                            } else if nr == abi.vfork_nr {
                                "vfork"
                            } else if nr == abi.wait4_nr {
                                "wait4"
                            } else {
                                "exit_group"
                            };
                            log(&format!(
                                "Task-6-S ENTRY: pid={} {} nr={} args=0x{:x},0x{:x},0x{:x},0x{:x}",
                                pid,
                                name,
                                nr,
                                get_syscall_arg(&regs, abi.reg_arg1),
                                get_syscall_arg(&regs, abi.reg_arg2),
                                get_syscall_arg(&regs, abi.reg_arg3),
                                get_syscall_arg(&regs, abi.reg_arg4),
                            ));
                        }
                    }

                    // ── Post-execve PATH logging ──────────────────────
                    //
                    // The existing "intercepted open({}) -> {}" log below
                    // is gated by `loop_count <= 500`, but kr64's own
                    // pre-execve setup (copying init, patching init.rc,
                    // creating /dev sockets, etc.) inflates `loop_count`
                    // well past 500 by the time TWRP init runs. As a
                    // result, NO post-execve open paths are logged, and we
                    // cannot see what init is opening — only the bare
                    // syscall numbers above.
                    //
                    // This dedicated block bypasses the `loop_count` gate
                    // and logs the path argument (arg1, or arg2 for the
                    // *at syscalls) for EVERY path-bearing syscall during
                    // the first 150 post-execve syscalls. It also covers
                    // UNtranslated paths (/dev/*, /proc/*, /sys/*, which
                    // `translate_path` leaves untouched and therefore
                    // never produce an "intercepted open" log), so we see
                    // those opens too.
                    //
                    // For mount(source, target, fstype, flags, data) we
                    // additionally log arg2 (target) and arg3 (fstype) so
                    // we can see exactly what init is mounting where.
                    // Task 6-Z26: raised from 200 to 500 (see ENTRY gate above).
                    if past_first_execve && post_execve_syscall_count <= 500 {
                        let path_idx = match syscall_num {
                            n if n == abi.open => Some(abi.reg_arg1),
                            n if n == abi.openat || n == abi.openat2 => Some(abi.reg_arg2),
                            n if n == abi.stat || n == abi.lstat => Some(abi.reg_arg1),
                            n if n == abi.newfstatat || n == abi.statx => Some(abi.reg_arg2),
                            n if n == abi.access => Some(abi.reg_arg1),
                            n if n == abi.faccessat => Some(abi.reg_arg2),
                            n if n == abi.mkdir => Some(abi.reg_arg1),
                            n if n == abi.chdir => Some(abi.reg_arg1),
                            n if n == abi.readlink => Some(abi.reg_arg1),
                            n if n == abi.readlinkat => Some(abi.reg_arg2),
                            n if n == abi.chmod => Some(abi.reg_arg1),
                            n if n == abi.chroot => Some(abi.reg_arg1),
                            n if n == abi.execve => Some(abi.reg_arg1),
                            // Task 6-Y: unlink takes path in arg1; unlinkat
                            // takes dirfd in arg1 + path in arg2 (like openat).
                            n if n == abi.unlink => Some(abi.reg_arg1),
                            n if n == abi.unlinkat => Some(abi.reg_arg2),
                            // mount's arg1 is the SOURCE path (may be NULL
                            // for bind/virtual mounts) — logged below; we
                            // do NOT set path_idx here so the generic
                            // "post-execve path" log is skipped for mount
                            // (mount gets its own structured 3-arg log).
                            _ => None,
                        };
                        if let Some(idx) = path_idx {
                            let path_addr = get_syscall_arg(&regs, idx);
                            if path_addr != 0 {
                                if let Some(path) = read_child_string(pid, path_addr) {
                                    log(&format!(
                                        "post-execve path: {} -> {:?}",
                                        syscall_name(syscall_num, &abi),
                                        path
                                    ));
                                }
                            }
                        }
                        // mount(source, target, fstype, flags, data):
                        //   arg1=source, arg2=target, arg3=fstype.
                        // Log all three non-NULL string args.
                        if syscall_num == abi.mount {
                            let src_addr = get_syscall_arg(&regs, abi.reg_arg1);
                            let tgt_addr = get_syscall_arg(&regs, abi.reg_arg2);
                            let fs_addr = get_syscall_arg(&regs, abi.reg_arg3);
                            let src = if src_addr != 0 {
                                read_child_string(pid, src_addr).unwrap_or_else(|| "<null>".into())
                            } else {
                                "<null>".into()
                            };
                            let tgt = if tgt_addr != 0 {
                                read_child_string(pid, tgt_addr).unwrap_or_else(|| "<null>".into())
                            } else {
                                "<null>".into()
                            };
                            let fs = if fs_addr != 0 {
                                read_child_string(pid, fs_addr).unwrap_or_else(|| "<null>".into())
                            } else {
                                "<null>".into()
                            };
                            log(&format!(
                                "post-execve mount: source={:?} target={:?} fstype={:?}",
                                src, tgt, fs
                            ));
                        }
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
                    // We re-reserve the area at EVERY syscall ENTRY
                    // stop (not lazily once). The child's stack grows
                    // DOWNWARD between syscalls (userspace function
                    // calls push frames, decrementing sp). If we
                    // cached the scratch area at the FIRST entry, the
                    // child's subsequent stack growth would move sp
                    // below the cached `sp - 4096`, putting the stale
                    // scratch area in the ACTIVE stack region — the
                    // next `write_translated_path` would then overwrite
                    // the child's new stack frames (return addresses,
                    // saved registers) with rootfs path bytes → SIGSEGV
                    // at rip=0x6f722f69 ('i/ro' from
                    // '/data/user/0/io.twoyi/rootfs'). This was the
                    // 6-Z14 failure mode (cached sp-4096, SIGSEGV at
                    // iter 786 after the child's stack grew into the
                    // stale scratch page).
                    //
                    // Task 6-Z15 tried to dodge this by using sp + 4096
                    // (ABOVE the stack pointer), reasoning that "stack
                    // grows down, so above sp is unused". That reasoning
                    // was BACKWARDS: on a downward-growing stack the
                    // region ABOVE sp holds the LIVE caller frames
                    // (return addresses, saved registers, locals), not
                    // unused space. Worse, when sp is near the TOP of
                    // the stack mapping (as it is early in init's boot),
                    // sp + 4096 lands in an UNMAPPED page beyond the
                    // mapping → PTRACE_POKEDATA fails (EIO) → path
                    // translation becomes a no-op → open("/dev/__null__")
                    // sees the untranslated host path → ENOENT → init
                    // exit(1) at iter 187. This was the 6-Z15 failure
                    // mode (verified on f973d7e UI E2E run 32252514166:
                    // "WARNING: write_translated_path FAILED for
                    // /dev/__null__ (scratch_addr=0xffc7a740, offset=0)
                    // — falling back to in-place overwrite").
                    //
                    // Task 6-Z16 (THE REAL FIX): re-reserve at EVERY
                    // syscall ENTRY, using sp - 4096 (BELOW the current
                    // stack pointer, in the auto-growable region). Each
                    // ENTRY reads the CURRENT sp, so the scratch area is
                    // always in the FRESH region below the child's
                    // current stack usage. The child is in the KERNEL
                    // between ENTRY and EXIT (no userspace function
                    // calls, no stack growth), so the scratch area is
                    // stable + untouched during the path's lifetime
                    // (ENTRY write → kernel reads path → EXIT). Between
                    // EXIT and the next ENTRY the child runs userspace
                    // and may grow its stack, but by then the path is
                    // already consumed and the NEXT ENTRY will re-reserve
                    // a fresh area at the new (lower) sp. sp - 4096 is in
                    // the MAP_GROWSDOWN stack VMA, so PTRACE_POKEDATA
                    // triggers a minor page fault + the kernel maps the
                    // page on demand — the write succeeds. 8-byte-align
                    // the address so POKEDATA word writes never straddle
                    // an unmapped boundary.
                    {
                        let sp = get_syscall_arg(&regs, abi.reg_sp);
                        scratch_addr = (sp.wrapping_sub(4096)) & !7u64;
                        scratch_offset = 0;
                        if loop_count <= 30 {
                            log(&format!(
                                "scratch area at {:#x} (below stack pointer {:#x}, re-reserved this ENTRY)",
                                scratch_addr, sp
                            ));
                        }
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
                                // ── VFS materialization ─────────────────
                                //
                                // BEFORE calling translate_path, ask the VFS
                                // to materialise any synthetic / dynamic node
                                // for this path into the host filesystem at
                                // `{rootfs}{path}`. For TWRP boot this is
                                // what writes the minimal valid
                                // __system_property_area__ at
                                // {rootfs}/dev/__properties__/properties_serial
                                // so init's find_property() iterates over 0
                                // properties and returns NULL naturally
                                // (replacing the old binary patch — see
                                // worklog 1-A F.1 + 1-B Task 3).
                                //
                                // translate_path() below will then rewrite
                                // the open's path argument to that exact
                                // `{rootfs}{path}` location (for /dev/* paths),
                                // so the real kernel open() finds the freshly
                                // materialised file.
                                //
                                // Materialization is a no-op for paths the VFS
                                // does not know about, so non-synthetic opens
                                // (e.g. /init.rc) proceed unchanged.
                                if vfs.is_synthetic(&path) {
                                    // Task 6-Z: if /dev/__properties__ already
                                    // exists as a regular file (pre-created by
                                    // the parent in TWRP mode before the ptrace
                                    // loop), SKIP VFS materialization to avoid
                                    // (a) clobbering the file (which would
                                    // destroy init's runtime
                                    // ftruncate/mmap modifications to the
                                    // property area header), and (b) any risk
                                    // of the host's real-Android
                                    // /dev/__properties__ directory (Android
                                    // 11+) shadowing the file. TWRP's AOSP 5.1
                                    // bionic requires /dev/__properties__ to be
                                    // a FILE (the OLD single-file property
                                    // area), NOT the NEW Android 8+ directory
                                    // format. materialize() also has this check
                                    // internally as a safety net, but we log it
                                    // here for runtime visibility.
                                    //
                                    // Without this skip, init's
                                    // open("/dev/__properties__") could hit a
                                    // directory → -EISDIR → properties_fd never
                                    // recorded → the mmap2 MAP_SHARED →
                                    // MAP_ANONYMOUS rewrite (Task 6-Y) never
                                    // fires → -38 persists → init exit(1).
                                    let skip_for_properties = is_properties_path(&path)
                                        && matches!(
                                            std::fs::metadata(format!("{}{}", rootfs, path)),
                                            Ok(ref md) if md.is_file()
                                        );
                                    if skip_for_properties {
                                        log("VFS: /dev/__properties__ already exists as a regular file (pre-created OLD-format) — skipping directory materialization (TWRP mode requires FILE, not directory)");
                                    } else if let Err(e) = vfs.materialize(&path, rootfs) {
                                        // Don't fail the open — log and let the
                                        // kernel's open() report its own error
                                        // (e.g. ENOENT) so we see both the
                                        // materialization failure AND the open
                                        // failure in the log.
                                        log(&format!(
                                            "VFS materialize FAILED for {}: {} — open() will see kernel's errno",
                                            path, e
                                        ));
                                    } else if loop_count <= 500 {
                                        log(&format!(
                                            "VFS materialized synthetic node for {} into {}{}",
                                            path, rootfs, path
                                        ));
                                    }
                                }
                                let translated = translate_path(rootfs, &path);
                                // ── Task 6-U: KLOG fd tracking (ENTRY side) ──
                                //
                                // If this open()'s path (original OR
                                // post-translation form) is a kernel-
                                // message log destination, set the pending
                                // flag so the matching open EXIT captures
                                // the returned fd into `kmsg_fd`. Subsequent
                                // write()s to that fd are then tagged
                                // "DIAG KLOG" by the EXIT-side write
                                // diagnostic below.
                                //
                                // Checked against BOTH the original `path`
                                // (covers untranslated /dev/__kmsg__ paths)
                                // AND `translated` (covers
                                // {rootfs}/dev/__kmsg__ after translate_path
                                // rewrites /dev/__kmsg__). The check is
                                // cheap and idempotent — `pending_kmsg_open`
                                // is cleared at the matching open EXIT.
                                if is_kmsg_path(&path) || is_kmsg_path(&translated) {
                                    pending_kmsg_open = true;
                                }
                                // Task 6-V: save translated path for fd
                                // tracking at the matching open EXIT.
                                pending_open_translated_path = Some(translated.clone());
                                if translated != path && loop_count <= 500 {
                                    log(&format!("intercepted open({}) -> {}", path, translated));
                                }
                                if translated != path {
                                    let wtp_ok = write_translated_path(
                                        pid,
                                        &mut regs,
                                        iov_len,
                                        path_arg_index,
                                        scratch_addr,
                                        &mut scratch_offset,
                                        &translated,
                                    );
                                    if !wtp_ok {
                                        log(&format!(
                                            "WARNING: write_translated_path FAILED for {} -> {} (scratch_addr={:#x}, offset={}) — falling back to in-place overwrite (will likely fail for longer paths)",
                                            path, translated, scratch_addr, scratch_offset
                                        ));
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
                        }
                        // Task 6-T: stat64 + lstat64 added to the
                        // path-translation match arm — these are the
                        // 64-bit-struct variants that modern i386 bionic
                        // (incl. TWRP recovery) actually uses INSTEAD of
                        // old stat/lstat. Same arg1-as-path semantics as
                        // stat/lstat. fstat64 takes an fd (not a path)
                        // and is intentionally NOT in this arm.
                        n if n == abi.stat
                            || n == abi.lstat
                            || n == abi.stat64
                            || n == abi.lstat64
                            || n == abi.newfstatat
                            || n == abi.statx =>
                        {
                            // Task 6-T: stat64/lstat64 use arg1 as the
                            // path (same as old stat/lstat); newfstatat
                            // + statx use arg2 (the dirfd is arg1).
                            let path_arg_index = if syscall_num == abi.stat
                                || syscall_num == abi.lstat
                                || syscall_num == abi.stat64
                                || syscall_num == abi.lstat64
                            {
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
                        // Task 6-Y: mmap (x86_64 nr=9, aarch64 nr=222) /
                        // mmap2 (i386 nr=192) — rewrite file-backed
                        // MAP_SHARED mmap of /dev/__properties__ to
                        // anonymous so the zygote's seccomp filter
                        // (inherited by untrusted_app, can't be removed)
                        // does not block it with -ENOSYS (-38).
                        //
                        // i386 mmap2 layout (per kernel UAPI):
                        //   arg1=addr (ebx), arg2=length (ecx),
                        //   arg3=prot  (edx), arg4=flags (esi),
                        //   arg5=fd    (edi), arg6=offset (ebp)
                        // x86_64 mmap layout (per kernel UAPI):
                        //   arg1=addr (rdi), arg2=length (rsi),
                        //   arg3=prot  (rdx), arg4=flags (r10),
                        //   arg5=fd    (r8),  arg6=offset (r9)
                        // aarch64 mmap layout (per kernel UAPI):
                        //   arg1=addr (x0), arg2=length (x1),
                        //   arg3=prot  (x2), arg4=flags (x3),
                        //   arg5=fd    (x4), arg6=offset (x5)
                        //
                        // We read arg4 (flags) + arg5 (fd) via the
                        // ABI-aware register indices (reg_arg4 / reg_arg5
                        // set per-ABI above). If fd matches the recorded
                        // `properties_fd` (set by the open EXIT handler
                        // when init opened /dev/__properties__) AND
                        // flags & MAP_SHARED != 0, rewrite:
                        //   flags: (flags & !MAP_SHARED) | MAP_ANONYMOUS
                        //                                            | MAP_PRIVATE
                        //   fd:    -1   (0xFFFFFFFF as i32 → sign-extended)
                        //   offset: 0
                        // Then ptrace_setregs writes the modified regs
                        // back so the kernel sees an anonymous mmap when
                        // it resumes the child. The kernel performs the
                        // anonymous mmap (which succeeds — anonymous mmap2
                        // is in the zygote's allow list). init's
                        // property_init then writes the property area
                        // header to this anonymous region and uses it as
                        // the property area. Since init is the only
                        // process in the sandbox (no fork), the lack of
                        // file-backing/sharing is fine.
                        //
                        // We DO NOT rewrite:
                        //   - anonymous mmaps (flags & MAP_SHARED == 0):
                        //     they already succeed.
                        //   - mmaps of OTHER files (sepolicy, etc.):
                        //     only /dev/__properties__ is rewritten. Other
                        //     MAP_SHARED mmaps either succeed (the zygote
                        //     allows them) or fail with -ENOSYS (which we
                        //     let propagate — the property area is the
                        //     ONLY file-backed MAP_SHARED mmap init
                        //     performs during early boot, per the strace).
                        //   - mmaps where properties_fd has not yet been
                        //     set (init hasn't opened /dev/__properties__
                        //     yet): no fd to match against, so we let the
                        //     kernel handle it normally.
                        n if n == abi.mmap || n == abi.mmap2 => {
                            let flags = get_syscall_arg(&regs, abi.reg_arg4) as i32;
                            let fd = get_syscall_arg(&regs, abi.reg_arg5) as i32;
                            // Task 6-Z2: ALWAYS log mmap2 args so we can see
                            // what the early calls (that return -38 ENOSYS) are.
                            log(&format!(
                                "DIAG mmap2 ENTRY: nr={} flags=0x{:x} (MAP_SHARED={},MAP_PRIVATE={},MAP_ANONYMOUS={}) fd={} prop_fd={:?}",
                                syscall_num,
                                flags,
                                (flags & libc::MAP_SHARED) != 0,
                                (flags & libc::MAP_PRIVATE) != 0,
                                (flags & libc::MAP_ANONYMOUS) != 0,
                                fd,
                                properties_fd
                            ));
                            // Task 6-Z2: rewrite ALL file-backed mmap2 (both
                            // MAP_SHARED AND MAP_PRIVATE) to MAP_ANONYMOUS.
                            // The zygote's seccomp blocks ALL file-backed mmap2
                            // for i386 compat (not just MAP_SHARED). Verified
                            // on 2f58da3 UI E2E: init's mmap2 of
                            // /dev/__properties__ uses flags=0x2 (MAP_PRIVATE,
                            // file-backed) -> -38. The old MAP_SHARED-only
                            // check didn't catch it. Since init is statically
                            // linked, the only file-backed mmap is
                            // /dev/__properties__ — safe to rewrite all
                            // file-backed to anonymous.
                            if (flags & libc::MAP_ANONYMOUS) == 0 {
                                let new_flags =
                                    rewrite_mmap_flags_shared_to_anonymous(flags) as u64;
                                set_syscall_arg(&mut regs, abi.reg_arg4, new_flags);
                                set_syscall_arg(&mut regs, abi.reg_arg5, (-1i32) as i64 as u64);
                                set_syscall_arg(&mut regs, abi.reg_arg6, 0);
                                match ptrace_setregs(pid, &regs, iov_len) {
                                    Ok(()) => log(&format!(
                                        "DIAG mmap2 REWRITE: fd={} flags=0x{:x} (file-backed) → MAP_ANONYMOUS|MAP_PRIVATE fd=-1 (zygote seccomp blocks ALL file-backed mmap2 for i386 compat)",
                                        fd,
                                        flags
                                    )),
                                    Err(e) => log(&format!(
                                        "DIAG mmap2 REWRITE FAILED: ptrace_setregs for nr={} fd={}: {} — child will see -ENOSYS",
                                        syscall_num, fd, e
                                    )),
                                }
                            }
                        }
                        // Task 6-Y: unlink + unlinkat need the ACTUAL
                        // translate_path + write-back (not just the logging
                        // match above). Pre-6-Y-fix, unlink was in the path-idx
                        // logging match but NOT in this write-back match → the
                        // path was logged but NOT translated → init's unlink hit
                        // the HOST /dev/socket → EACCES → 'init startup failure'.
                        // unlink: path in arg1. unlinkat: dirfd in arg1, path in
                        // arg2 (like openat).
                        n if n == abi.unlink || n == abi.unlinkat => {
                            let path_arg_index = if syscall_num == abi.unlink {
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
                        // Task 6-Z6: poll — at the ENTRY stop, if the
                        // timeout (arg3) is 0 (non-blocking) or negative
                        // (infinite), set it to 100ms so the kernel SLEEPS
                        // instead of returning immediately. This stops the
                        // poll spin: the recovery retries every 100ms
                        // instead of every microsecond. Combined with the
                        // EXIT-side poll fake-success (6-Z5, which fakes
                        // the return to 0), this lets the recovery proceed
                        // past the setup poll loop.
                        //
                        // i386 poll(2): arg1=pollfd*, arg2=nfds, arg3=timeout(ms).
                        // The timeout is a signed int: 0=non-blocking, -1=infinite,
                        // >0=wait N ms. We set it to 100 (100ms) to reduce the
                        // spin rate from ~1531 polls/s to ~10 polls/s.
                        //
                        // CAVEAT: the TWRP main UI loop also uses poll for input.
                        // Forcing a 100ms timeout there would add 100ms input
                        // latency. BUT the recovery is stuck BEFORE the UI loop
                        // (in a setup poll spin). Once it reaches the framebuffer,
                        // the poll behavior might differ. If this causes a
                        // regression (input lag), a follow-up can make the timeout
                        // conditional (only during setup).
                        n if abi.poll_nr != -1 && n == abi.poll_nr => {
                            let timeout = get_syscall_arg(&regs, abi.reg_arg3) as i32;
                            // Task 6-Z17 DIAG: log the poll timeout value +
                            // whether the timeout-set fires. The 5e0f157
                            // E2E showed 0 "DIAG poll timeout-set" logs but
                            // the polls returned instantly (POLLERR busy-wait)
                            // — need to know if the block is entered + what
                            // the timeout is.
                            if loop_count <= 100 {
                                log(&format!(
                                    "DIAG poll ENTRY: timeout={} (arg3) — {}",
                                    timeout,
                                    if timeout <= 0 {
                                        "WILL set to 100ms"
                                    } else {
                                        "timeout already >0, no set"
                                    }
                                ));
                            }
                            if timeout <= 0 {
                                set_syscall_arg(&mut regs, abi.reg_arg3, 100);
                                if let Err(e) = ptrace_setregs(pid, &regs, iov_len) {
                                    log(&format!(
                                        "DIAG poll timeout-set FAILED: ptrace_setregs: {} — child will spin",
                                        e
                                    ));
                                }
                            }
                        }
                        // ── Task 6-Z9: xattr-SET ENTRY → rewrite to getpid ──
                        //
                        // setxattr / lsetxattr / fsetxattr (the *xattr SET
                        // family) are called by TWRP recovery during its
                        // SELinux-restorecon phase:
                        //   lsetxattr(path, "security.selinux", ctx, 44, 0)
                        // As untrusted_app the kernel returns -EPERM / -EACCES
                        // / -EOPNOTSUPP (no CAP for security.* xattrs, or the
                        // filesystem doesn't support xattrs). The post-6-R
                        // EXIT-side fake (compute_exit_return_value) was
                        // SUPPOSED to overwrite rax=0 at the EXIT stop, but
                        // the dispatcher's evidence on b712639 UI E2E run
                        // 32227786881 was inconclusive — the "faking success"
                        // log is gated by `loop_count <= 200` (long past by
                        // the time lsetxattr fires in the restorecon phase),
                        // and the readback log is gated by
                        // `loop_count <= 300` (also past). The recovery was
                        // stuck in the restorecon loop.
                        //
                        // Task 6-Z9 takes a MORE DIRECT approach: at the
                        // ENTRY stop, REWRITE orig_eax to `abi.getpid` BEFORE
                        // the kernel executes the syscall. The kernel then
                        // executes getpid (always succeeds, returns the PID)
                        // instead of the xattr SET (which would return
                        // EPERM/EACCES/EOPNOTSUPP). At the matching EXIT
                        // stop, `pending_xattr_fake` is consumed: we fake
                        // the return to 0 (success) regardless of what getpid
                        // returned.
                        //
                        // This is SAFE (unlike the SIGSYS-handler's old
                        // "rewrite orig_rax" which was reverted in the
                        // "never rewrite orig_rax" fix) because here the
                        // kernel has NOT yet executed the syscall — we are
                        // at the ENTRY stop, and rewriting orig_rax BEFORE
                        // resuming causes the kernel to execute the NEW
                        // syscall (getpid) instead of the original. The
                        // SIGSYS case was different: the kernel had ALREADY
                        // aborted the syscall (seccomp fired), so rewriting
                        // orig_rax there caused the kernel to RE-EXECUTE the
                        // new syscall (getpid) and overwrite our faked return
                        // value with getpid's real PID.
                        //
                        // We set `pending_xattr_fake = true` BEFORE the
                        // ptrace_setregs call so that even if the rewrite
                        // FAILS (ptrace_setregs error → the kernel still
                        // executes the original xattr syscall), the EXIT
                        // handler still fakes the return to 0 via the
                        // `pending_xattr_fake` branch (belt-and-suspenders
                        // with `compute_exit_return_value`, which would
                        // ALSO match the original xattr number at EXIT in
                        // the rewrite-failed case — both set rax=0,
                        // redundant but harmless).
                        //
                        // UN-GATED log: the restorecon loop is the CURRENT
                        // blocker, so we need to see EVERY xattr ENTRY
                        // rewrite (not just the first N) to confirm the
                        // fix is applied. The volume is bounded by the
                        // number of files recovery tries to relabel (a few
                        // hundred at most before it proceeds past the
                        // restorecon phase).
                        n if n == abi.setxattr || n == abi.lsetxattr || n == abi.fsetxattr => {
                            pending_xattr_fake = true;
                            set_syscall_num(&mut regs, &abi, abi.getpid);
                            match ptrace_setregs(pid, &regs, iov_len) {
                                Ok(()) => log(&format!(
                                    "DIAG xattr ENTRY: {} nr={} → rewritten to getpid nr={} (kernel will execute getpid; EXIT will fake return 0) — avoids kernel EPERM/EACCES/EOPNOTSUPP for security.* xattr SET as untrusted_app",
                                    syscall_name(syscall_num, &abi),
                                    syscall_num,
                                    abi.getpid
                                )),
                                Err(e) => log(&format!(
                                    "DIAG xattr ENTRY REWRITE FAILED: {} nr={} → getpid: ptrace_setregs: {} — kernel will execute the xattr syscall (EXIT will still fake return 0 via pending_xattr_fake)",
                                    syscall_name(syscall_num, &abi),
                                    syscall_num,
                                    e
                                )),
                            }
                        }
                        _ => {
                            // Not an intercepted syscall — let it through.
                        }
                    }

                    // Task 6-Z28: poll() interception. init's main event
                    // loop calls poll() which returns POLLERR immediately
                    // (fake property_service socket). The 6-Z19 NOP prevented
                    // the call entirely but caused a userspace busy-spin (no
                    // syscalls, no sleep, no events → init stuck at #457).
                    // NOW: let poll execute (returns POLLERR), sleep 100ms to
                    // prevent busy-spin, set pending flag for EXIT fake.
                    if syscall_num == abi.poll_nr {
                        pending_poll_fake = true;
                        // Sleep 100ms to give init timer-event processing time
                        // + prevent the POLLERR busy-spin. The child is stopped
                        // at the ENTRY (before poll executes) — the sleep doesn't
                        // block the child, only the ptrace parent.
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        if post_execve_syscall_count <= 500 {
                            log(&format!(
                                "DIAG poll ENTRY: nr={} — sleeping 100ms + will fake return 0 at EXIT (Task 6-Z28: prevents POLLERR busy-spin, gives init timer events)",
                                syscall_num
                            ));
                        }
                    }
                } else {
                    // ── Syscall EXIT ──
                    in_syscall = false;

                    // ── DIAGNOSTIC (6-S3): unconditional fork-family EXIT ──
                    //
                    // Logs the kernel return value for every fork-family
                    // syscall (nr=2/57/120/56/190/58/435), UNCONDITIONALLY
                    // (not gated). 0=child, >0=parent's-child-pid, <0=error.
                    if matches!(syscall_num, 2 | 57 | 120 | 56 | 190 | 58 | 435) {
                        let ret = get_syscall_arg(&regs, abi.reg_ret) as i64;
                        log(&format!(
                            "DIAG fork-family EXIT: nr={} returned {} (0=child, >0=parent's-child-pid, <0=error)",
                            syscall_num, ret
                        ));
                    }

                    // ── DIAGNOSTIC (6-U): KLOG fd tracking + write() ──
                    //
                    // Surface TWRP init's stranded KLOG inline in the
                    // logcat. init writes 339 write() calls (5820 bytes)
                    // to /dev/__kmsg__ (its KLOG) — kr64 copies that
                    // file to /sdcard but the test harness never pulls
                    // it, so init's own diagnostic messages (WHY it
                    // bails before parsing init.rc) are invisible. This
                    // block captures the write() buffer contents inline
                    // so the KLOG is visible without pulling /sdcard.
                    //
                    // Part A — open()/openat()/openat2() EXIT: consume
                    // the `pending_kmsg_open` flag set at ENTRY. If the
                    // kernel returned a valid fd (ret > 0), record it
                    // in `kmsg_fd`. Subsequent write()s to that fd are
                    // tagged "DIAG KLOG" by Part B below. The i386
                    // syscall convention preserves arg registers
                    // (ebx/ecx/edx) across the syscall, so the EXIT
                    // snapshot's abi.reg_ret holds the new fd.
                    if pending_kmsg_open
                        && (syscall_num == abi.open
                            || syscall_num == abi.openat
                            || syscall_num == abi.openat2)
                    {
                        let ret = get_syscall_arg(&regs, abi.reg_ret) as i64;
                        if ret > 0 {
                            kmsg_fd = Some(ret as i32);
                            log(&format!(
                                "DIAG KLOG fd captured: open() returned fd={} — subsequent write()s to this fd will be tagged 'DIAG KLOG'",
                                ret
                            ));
                        } else {
                            // open() failed (ret < 0 = -errno, or 0).
                            // Keep the previous kmsg_fd (if any) — this
                            // open might be a retry that failed; the
                            // prior successful open's fd is still the
                            // canonical KLOG fd.
                            log(&format!(
                                "DIAG KLOG fd capture: open() returned {} (failure) — kmsg_fd remains {:?}",
                                ret, kmsg_fd
                            ));
                        }
                        pending_kmsg_open = false;
                    }
                    // Task 6-V Part A2 — open()/openat()/openat2() EXIT:
                    // record the returned fd → translated-path mapping into
                    // `open_fd_paths` (used by the read() diagnostic below
                    // to annotate which file was read).
                    //
                    // Task 6-Y extension: ALSO record the fd in
                    // `properties_fd` when the translated path matches
                    // is_properties_path() (i.e. /dev/__properties__). The
                    // subsequent mmap2 ENTRY handler uses this fd to
                    // recognise the file-backed MAP_SHARED mmap of the
                    // property area and rewrite it to anonymous so the
                    // zygote seccomp filter does not -ENOSYS it. Mirrors
                    // the existing `pending_kmsg_open` → `kmsg_fd`
                    // pattern from Task 6-U (set at open EXIT, consumed by
                    // a later syscall-ENTRY arm).
                    if syscall_num == abi.open
                        || syscall_num == abi.openat
                        || syscall_num == abi.openat2
                    {
                        let ret = get_syscall_arg(&regs, abi.reg_ret) as i64;
                        if ret > 0 {
                            if let Some(ref p) = pending_open_translated_path {
                                open_fd_paths.insert(ret as i32, p.clone());
                                // Task 6-Y: track __properties__ fd for
                                // the mmap2 MAP_SHARED → MAP_ANONYMOUS
                                // rewrite. The translated path covers both
                                // the raw `/dev/__properties__` (when
                                // translate_path leaves it untouched) AND
                                // `{rootfs}/dev/__properties__` (when
                                // translate_path rewrites it) —
                                // is_properties_path() matches the final
                                // component `__properties__` in both cases.
                                if is_properties_path(p) {
                                    properties_fd = Some(ret as i32);
                                    log(&format!(
                                        "DIAG properties fd captured: open() returned fd={} for {} — subsequent mmap2 with MAP_SHARED on this fd will be rewritten to MAP_ANONYMOUS|MAP_PRIVATE",
                                        ret, p
                                    ));
                                }
                            }
                        } else if let Some(ref p) = pending_open_translated_path {
                            // Task 6-Y fix 2: when open(/dev/__properties__)
                            // fails (ret <= 0), fake a successful return
                            // (fd=42) so init gets a valid fd. The
                            // subsequent mmap2 ENTRY handler will
                            // rewrite the fd=42 MAP_SHARED mmap to
                            // anonymous anyway, so the fd never needs
                            // to be a real kernel fd.
                            //
                            // Root cause: the zygote's seccomp filter
                            // blocks i386 open() on some paths →
                            // ENOSYS (-38) or EEXIST (-17) when init
                            // uses O_CREAT|O_EXCL on the pre-created
                            // file. Without this fake, properties_fd
                            // is never set → the mmap2 rewrite never
                            // fires → property area not mapped →
                            // all property_set calls fail → init
                            // exits(1).
                            if is_properties_path(p) {
                                let fake_fd: i64 = 42;
                                set_syscall_ret(&mut regs, &abi, fake_fd);
                                properties_fd = Some(fake_fd as i32);
                                if let Err(e) = ptrace_setregs(pid, &regs, iov_len) {
                                    log(&format!(
                                        "DIAG properties fd FAKE FAILED: ptrace_setregs for fd=42: {} — open returned {}, init will see the error",
                                        e, ret
                                    ));
                                } else {
                                    log(&format!(
                                        "DIAG properties fd FAKE: open() returned {} for {} — faked fd=42 (zygote seccomp blocks i386 open on this path); mmap2 with MAP_SHARED on fd=42 will be rewritten to MAP_ANONYMOUS|MAP_PRIVATE",
                                        ret, p
                                    ));
                                }
                            }
                        }
                        pending_open_translated_path = None;
                    }
                    // Part B — write(fd, buf, count) EXIT: capture
                    // the buffer contents and log them. Gated to the
                    // first 800 post-execve writes (init does ~339
                    // total per the strace, so 800 is a 2.4× headroom
                    // that bounds log volume if init spins). Only fires
                    // when `past_first_execve` (the pre-execve kr64
                    // writes are uninteresting setup noise).
                    if past_first_execve && syscall_num == abi.write {
                        let ret = get_syscall_arg(&regs, abi.reg_ret) as i64;
                        // Only capture successful writes with a
                        // reasonable size (0 < ret <= 512). ret > 512
                        // is almost certainly a non-KLOG bulk write
                        // (e.g. property-area sync); ret <= 0 is a
                        // failed write (nothing to capture).
                        if ret > 0 && ret <= 512 {
                            post_execve_write_count = post_execve_write_count.saturating_add(1);
                            if post_execve_write_count <= 800 {
                                // fd = arg1 (ebx on i386 / rdi on x86_64),
                                // buf = arg2 (ecx on i386 / rsi on x86_64).
                                // Both are preserved across the syscall
                                // by the i386 + x86_64 calling convention
                                // for syscalls (the kernel does NOT
                                // clobber ebx/rcx/rdi/rsi in syscall
                                // entry — only rax is overwritten with
                                // the return value).
                                let fd = get_syscall_arg(&regs, abi.reg_arg1) as i32;
                                let buf_addr = get_syscall_arg(&regs, abi.reg_arg2);
                                let to_read = std::cmp::min(ret as usize, 256);
                                let captured = read_child_bytes(pid, buf_addr, to_read);
                                let is_klog = kmsg_fd == Some(fd);
                                let prefix = if is_klog { "DIAG KLOG" } else { "DIAG write" };
                                match captured {
                                    Some(bytes) => {
                                        let captured_str = String::from_utf8_lossy(&bytes);
                                        log(&format!(
                                            "{}(fd={}, ret={}): {:?}",
                                            prefix, fd, ret, captured_str
                                        ));
                                    }
                                    None => {
                                        // PTRACE_PEEKDATA failed on the
                                        // very first word (EIO / unmapped
                                        // address). Log the failure
                                        // explicitly so it is greppable
                                        // but do NOT crash the loop.
                                        log(&format!(
                                            "{}(fd={}, ret={}): <buffer read failed: EIO>",
                                            prefix, fd, ret
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    // Task 6-Z29: fake setexeccon (write to /proc/self/attr/exec).
                    // init's Service::Start() calls setexeccon(seclabel) which is
                    // implemented as write(fd, context, len) to /proc/self/attr/exec.
                    // The kernel rejects the write (EINVAL — context not in loaded
                    // policy, or EPERM — untrusted_app lacks MAC_ADMIN). This
                    // aborts the service start → recovery service never forks.
                    // FIX: if the write fd was opened from an attr/exec path AND
                    // the write failed (ret < 0), fake the return to the write
                    // count (success). init thinks setexeccon succeeded → forks
                    // the recovery service → TWRP renders.
                    if past_first_execve && syscall_num == abi.write {
                        let ret = get_syscall_arg(&regs, abi.reg_ret) as i64;
                        if ret < 0 {
                            let fd = get_syscall_arg(&regs, abi.reg_arg1) as i32;
                            if let Some(path) = open_fd_paths.get(&fd) {
                                if path.contains("attr/exec") {
                                    let count = get_syscall_arg(&regs, abi.reg_arg3) as i64;
                                    let fake_ret = if count > 0 { count } else { 0 };
                                    let mut regs2: Regs = unsafe { std::mem::zeroed() };
                                    match ptrace_getregs(pid, &mut regs2) {
                                        Ok(len) => {
                                            set_syscall_ret(&mut regs2, &abi, fake_ret);
                                            match ptrace_setregs(pid, &regs2, len) {
                                                Ok(()) => log(&format!(
                                                    "DIAG attr/exec: faked setexeccon write success (ret {}->{}) — init thinks context was set, will fork recovery service (Task 6-Z29)",
                                                    ret, fake_ret
                                                )),
                                                Err(e) => log(&format!(
                                                    "DIAG attr/exec: ptrace_setregs FAILED: {} — setexeccon fails, service start aborts",
                                                    e
                                                )),
                                            }
                                        }
                                        Err(e) => log(&format!(
                                            "DIAG attr/exec: ptrace_getregs FAILED: {}",
                                            e
                                        )),
                                    }
                                }
                            }
                        }
                    }

                    // Task 6-V Part C — read(fd, buf, count) EXIT:
                    // capture the buffer contents and log them. Mirrors
                    // the 6-U write() diagnostic above. Gated to the
                    // first 800 post-execve reads. Only fires for
                    // read() syscalls (syscall_num == abi.read) to
                    // avoid capturing buffers from unrelated syscalls.
                    if past_first_execve && syscall_num == abi.read {
                        let ret = get_syscall_arg(&regs, abi.reg_ret) as i64;
                        // Only capture successful reads with a
                        // reasonable size (0 < ret <= 256). ret <= 0
                        // is a failed/EOF read (nothing to capture).
                        if ret > 0 && ret <= 256 {
                            post_execve_read_count = post_execve_read_count.saturating_add(1);
                            if post_execve_read_count <= 800 {
                                let fd = get_syscall_arg(&regs, abi.reg_arg1) as i32;
                                let buf_addr = get_syscall_arg(&regs, abi.reg_arg2);
                                let to_read = std::cmp::min(ret as usize, 256);
                                let captured = read_child_bytes(pid, buf_addr, to_read);
                                // Look up the path from open fd tracking
                                let path_info = match open_fd_paths.get(&fd) {
                                    Some(p) => format!(", path=\"{}\"", p),
                                    None => String::new(),
                                };
                                match captured {
                                    Some(bytes) => {
                                        let captured_str = String::from_utf8_lossy(&bytes);
                                        log(&format!(
                                            "DIAG read(fd={}, ret={}{}): {:?}",
                                            fd, ret, path_info, captured_str
                                        ));
                                    }
                                    None => {
                                        log(&format!(
                                            "DIAG read(fd={}, ret={}{}): <buffer read failed: EIO>",
                                            fd, ret, path_info
                                        ));
                                    }
                                }

                                // Task 6-Z25: fake the SELinux context read for
                                // /proc/self/task/*/attr/current. PROBLEM (verified on
                                // 4fdd2d5 E2E): init reads its OWN thread context
                                // (/proc/self/task/<tid>/attr/current) + sees
                                // "u:r:untrusted_app_27:s0:c167,c256,c512,c768" (the
                                // REAL untrusted_app context). It expects a privileged
                                // recovery/init context. It then tries lsetxattr
                                // (faked return 0 but doesn't actually apply) +
                                // re-reads (STILL untrusted_app_27) → retries forever
                                // (the #87→#141 attr/current open loop, ~9 iterations,
                                // blocking the recovery at the SELinux phase, never
                                // reaching service-start + framebuffer render). FIX:
                                // overwrite the read buffer with "u:r:recovery:s0" so
                                // init thinks it's running as the privileged recovery
                                // context → stops the retry loop → proceeds.
                                if path_info.contains("attr/current") {
                                    let fake_ctx = "u:r:recovery:s0";
                                    // read() count includes the trailing NUL (the
                                    // original returned 44 = 43 chars + NUL), so the
                                    // faked return is len + 1 (NUL).
                                    let fake_len = (fake_ctx.len() as i64) + 1;
                                    if write_child_string_unchecked(pid, buf_addr, fake_ctx) {
                                        let mut regs2: Regs = unsafe { std::mem::zeroed() };
                                        match ptrace_getregs(pid, &mut regs2) {
                                            Ok(len) => {
                                                set_syscall_ret(&mut regs2, &abi, fake_len);
                                                match ptrace_setregs(pid, &regs2, len) {
                                                    Ok(()) => log(&format!(
                                                        "DIAG attr/current: faked context -> \"{}\" (ret {}->{}) — init sees privileged recovery context, retry loop should stop (Task 6-Z25)",
                                                        fake_ctx, ret, fake_len
                                                    )),
                                                    Err(e) => log(&format!(
                                                        "DIAG attr/current: ptrace_setregs FAILED: {} — init still sees untrusted_app_27, retry loop continues",
                                                        e
                                                    )),
                                                }
                                            }
                                            Err(e) => log(&format!(
                                                "DIAG attr/current: ptrace_getregs FAILED: {} — cannot fake return",
                                                e
                                            )),
                                        }
                                    } else {
                                        log("DIAG attr/current: write_child_string_unchecked FAILED — buffer not overwritten, init still sees untrusted_app_27");
                                    }
                                }
                            }
                        }
                    }

                    // ── Post-execve RETURN-VALUE logging ─────────────
                    //
                    // Logs the kernel's return value for every post-execve
                    // syscall (first 5000), so we can see EXACTLY which
                    // open/mount/mkdir/mknod fails and with what errno.
                    // This is read from the `regs` snapshot taken at the
                    // TOP of this SIGTRAP|0x80 stop (BEFORE the
                    // pending_getpid / EPERM-workaround rewrites below
                    // overwrite the return register), so it reflects the
                    // REAL kernel return value — e.g. -ENOENT (-2),
                    // -EPERM (-1), -EEXIST (-17), or 0 (success).
                    //
                    // For seccomp-trapped syscalls (mount, mknod) the
                    // SIGSYS handler runs BEFORE this EXIT stop and sets
                    // the return to 0 (faked success); the value logged
                    // here will be that faked 0, which is still useful
                    // (confirms the fake was applied) — the ENTRY-side
                    // path log above tells us WHAT was attempted.
                    //
                    // Task 6-S: increased from 150 to 5000 to capture the
                    // full recovery phase (recovery runs ~3281 iterations
                    // before exit(1); 150 hid the middle phase where
                    // fork/clone attempts + the exit trigger live).
                    // Task 6-Z26: raised from 200 to 500 (see ENTRY gate above).
                    if past_first_execve && post_execve_syscall_count <= 500 {
                        let ret = get_syscall_arg(&regs, abi.reg_ret) as i64;
                        let ret_desc: String = if ret < 0 && ret > -4096 {
                            format!("{} (-errno {})", ret, -ret)
                        } else {
                            format!("{}", ret)
                        };
                        log(&format!(
                            "post-execve return #{}: {} nr={} -> {}",
                            post_execve_syscall_count,
                            syscall_name(syscall_num, &abi),
                            syscall_num,
                            ret_desc
                        ));
                    }

                    // Task 6-S: log the return value for
                    // fork/clone/vfork/wait4/exit_group (NOT gated by the
                    // 5000 post-execve cap). This is the EXIT-side
                    // companion to the ENTRY-side always-log above. For
                    // fork/clone/vfork a positive return value is the
                    // child PID (in the parent) or 0 (in the child); a
                    // negative value is -errno (e.g. -ENOMEM, -EAGAIN).
                    // For wait4 it is the reaped child's PID, or -ECHILD
                    // (no children to wait for — the post-6-R recovery's
                    // suspected failure mode), or -EINTR.
                    // For exit_group the call never returns to the caller
                    // (the process exits) — but if we somehow see this
                    // fire it confirms exit_group was the syscall that
                    // terminated the child. The pre-6-S 150-cap hid
                    // whether recovery ever attempted these in the middle
                    // phase (iters 151-3271) — this block ensures we see
                    // the return values even past 5000 iterations.
                    {
                        let nr = syscall_num;
                        if nr == abi.clone_nr
                            || nr == abi.fork_nr
                            || nr == abi.vfork_nr
                            || nr == abi.wait4_nr
                            || nr == abi.exit_group_nr
                        {
                            let name = if nr == abi.clone_nr {
                                "clone"
                            } else if nr == abi.fork_nr {
                                "fork"
                            } else if nr == abi.vfork_nr {
                                "vfork"
                            } else if nr == abi.wait4_nr {
                                "wait4"
                            } else {
                                "exit_group"
                            };
                            let ret = get_syscall_arg(&regs, abi.reg_ret) as i64;
                            log(&format!(
                                "Task-6-S EXIT: pid={} {} nr={} -> {} (0x{:x})",
                                pid, name, nr, ret, ret as u64,
                            ));
                        }
                    }

                    // ── execve EXIT: schedule ABI reset ──
                    //
                    // If the ENTRY for this syscall was an execve (flag
                    // set above), the child's image has now been
                    // replaced. /proc/<pid>/exe points to the new
                    // binary. We set `reset_abi_next` so the TOP of the
                    // next loop iteration resets `abi = None`, forcing
                    // a fresh bitness detection at the next syscall
                    // stop. This is what actually fixes the
                    // "permanently locked to x86_64" bug.
                    //
                    // We do the reset at the TOP of the next iteration
                    // (not here) so the SIGSYS handler — which also
                    // checks `abi.is_none()` and re-detects — sees the
                    // reset state if a SIGSYS fires before the next
                    // SIGTRAP|0x80 stop.
                    if saw_execve {
                        saw_execve = false;
                        reset_abi_next = true;
                        log("execve EXIT — will reset ABI at next stop");
                    }

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

                    // ── Task 6-Z9: xattr-SET EXIT → fake return 0 ──────
                    //
                    // This consumes the `pending_xattr_fake` flag set at the
                    // ENTRY stop for setxattr / lsetxattr / fsetxattr (where
                    // we rewrote orig_eax to `getpid`). At this EXIT stop,
                    // the kernel has just executed getpid (NOT the original
                    // xattr SET) and rax holds getpid's return value (the
                    // child's PID, a positive integer). We fake rax=0 so
                    // recovery sees "lsetxattr returned 0 (success)" and
                    // proceeds past the restorecon retry loop.
                    //
                    // This is the PRIMARY fake path for xattr SET syscalls
                    // post-6-Z9. The legacy `compute_exit_return_value`
                    // block below is belt-and-suspenders: it ONLY fires if
                    // the ENTRY rewrite FAILED (so syscall_num at EXIT is
                    // still the original xattr number, e.g. 227 for i386
                    // lsetxattr). In that case BOTH this block AND the
                    // `compute_exit_return_value` block set rax=0
                    // (redundant but harmless — both write 0 to rax via
                    // fresh ptrace_getregs + set_syscall_ret + ptrace_setregs).
                    //
                    // DIAGNOSTIC: the pre-6-Z9 dispatcher evidence on
                    // b712639 UI E2E run 32227786881 was INCONCLUSIVE about
                    // whether the legacy EXIT-side fake (compute_exit_return_value)
                    // was actually being applied for lsetxattr — the
                    // "intercepted ... faking success" log was gated by
                    // `loop_count <= 200` (long past by the time lsetxattr
                    // fires in the restorecon phase), and the 5-J readback
                    // log was gated by `loop_count <= 300` (also past). The
                    // 6-Z9 ENTRY-side rewrite makes the fake OBSERVABLE via
                    // the UN-GATED "DIAG xattr ENTRY" + "DIAG xattr EXIT"
                    // logs (which fire regardless of loop_count), so the
                    // next UI E2E run will definitively show whether the
                    // fake is applied.
                    //
                    // RE-READ syscall number at EXIT: `syscall_num` was
                    // computed at the top of this SIGTRAP|0x80 block from
                    // the EXIT-stop `regs` snapshot. If the ENTRY rewrite
                    // succeeded, `syscall_num` here is `getpid` (NOT the
                    // original xattr number) — the diagnostic logs both
                    // so we can confirm the rewrite took effect.
                    if pending_xattr_fake {
                        pending_xattr_fake = false;
                        let exit_syscall_num = syscall_num;
                        let mut regs2: Regs = unsafe { std::mem::zeroed() };
                        match ptrace_getregs(pid, &mut regs2) {
                            Ok(len) => {
                                let original_ret = get_syscall_arg(&regs2, abi.reg_ret) as i64;
                                set_syscall_ret(&mut regs2, &abi, 0);
                                match ptrace_setregs(pid, &regs2, len) {
                                    Ok(()) => log(&format!(
                                        "DIAG xattr EXIT: faked return 0 (getpid returned {}; exit-syscall_num={} [{}] — if [getpid], the ENTRY rewrite succeeded) — child sees xattr-SET success",
                                        original_ret,
                                        exit_syscall_num,
                                        syscall_name(exit_syscall_num, &abi)
                                    )),
                                    Err(e) => log(&format!(
                                        "DIAG xattr EXIT FAKE FAILED: ptrace_setregs: {} — child will see getpid's return {} (NOT 0) for the xattr SET; recovery may spin",
                                        e, original_ret
                                    )),
                                }
                            }
                            Err(e) => {
                                // The pre-6-Z9 code had NO else branch here
                                // — a silent ptrace_getregs failure would
                                // skip the fake with NO log, leaving the
                                // dispatcher unable to tell whether the
                                // fake ran. 6-Z9 surfaces the failure.
                                log(&format!(
                                    "DIAG xattr EXIT: ptrace_getregs FAILED: {} — cannot fake return 0; child will see getpid's return for the xattr SET",
                                    e
                                ));
                            }
                        }
                    }

                    // Task 6-Z28: poll() EXIT fake. The kernel's poll returned
                    // POLLERR (1) because the property_service socket is fake.
                    // Fake the return to 0 (timeout, no events) so init's event
                    // loop processes the timeout + retries actions (instead of
                    // seeing POLLERR + busy-spinning). Combined with the 100ms
                    // sleep at the ENTRY, this gives init ~10 timer events/sec.
                    if pending_poll_fake {
                        pending_poll_fake = false;
                        let mut regs2: Regs = unsafe { std::mem::zeroed() };
                        match ptrace_getregs(pid, &mut regs2) {
                            Ok(len) => {
                                let original_ret = get_syscall_arg(&regs2, abi.reg_ret) as i64;
                                set_syscall_ret(&mut regs2, &abi, 0);
                                match ptrace_setregs(pid, &regs2, len) {
                                    Ok(()) => {
                                        if post_execve_syscall_count <= 500 {
                                            log(&format!(
                                                "DIAG poll EXIT: faked return 0 (was {} — POLLERR) — init sees timeout, processes timer events (Task 6-Z28)",
                                                original_ret
                                            ));
                                        }
                                    }
                                    Err(e) => log(&format!(
                                        "DIAG poll EXIT FAKE FAILED: ptrace_setregs: {} — init sees POLLERR, may busy-spin",
                                        e
                                    )),
                                }
                            }
                            Err(e) => log(&format!(
                                "DIAG poll EXIT: ptrace_getregs FAILED: {} — cannot fake return",
                                e
                            )),
                        }
                    }

                    // ── TWRP-init EPERM / syscall-number-leak workaround ───
                    //
                    // chmod / fchmod / fchown / lchown / chown / fchmodat /
                    // fchownat / capget / ioprio_get / ioprio_set / mount /
                    // rt_sigprocmask / mknod all return EPERM (or are
                    // seccomp-blocked with SIGSYS) as untrusted_app (no
                    // CAP_CHOWN / CAP_FOWNER / CAP_SYS_ADMIN / CAP_SYS_RESOURCE
                    // / CAP_SYS_NICE / CAP_MKNOD; mount/rt_sigprocmask hit
                    // the seccomp filter for sys_admin-related operations;
                    // mknod needs CAP_MKNOD which untrusted_app lacks).
                    // TWRP's init calls these early in its startup and
                    // EITHER gives up with exit(1) (capget / ioprio_get /
                    // ioprio_set / fchown / fchmod — the Round-78/79 blocker
                    // fixed by commit f279552; ioprio_set added by Task 5-S;
                    // mount + rt_sigprocmask added by Task 5-T — these were
                    // the REAL root cause of the UI E2E TWRP init exit(1)
                    // at iter 189, misdiagnosed earlier as ioprio_set=252
                    // which is actually exit_group; mknod added by Task 5-X
                    // — the next blocker after 5-T's mount fix per 5-W's
                    // VLM-verified UI E2E analysis: "post-execve return #34:
                    // unknown nr=14 -> 14" (non-zero, NOT faked) → init
                    // exit(1) at iter 189 unchanged)
                    // OR takes an error-handling path that ends up
                    // dereferencing NULL+0x90 (chmod/lchown/chown/fchmodat/
                    // fchownat — the Round-80/81 blocker fixed here, see the
                    // worklog entry for Task 5-A).
                    //
                    // These syscalls are NOT blocked by Android's seccomp
                    // filter (no SIGSYS) on most devices — they execute
                    // normally and the kernel returns -EPERM. So we cannot
                    // rely on the SIGSYS handler below to mask them; we have
                    // to intercept them HERE at the syscall-exit stop and
                    // overwrite the return value with 0 (success).
                    //
                    // THERE IS A SECOND, MORE SUBTLE CASE: on i386 compat
                    // children the kernel does NOT always call
                    // `syscall_set_return_value(-ENOSYS)` for
                    // seccomp-aborted syscalls. The 4-E E2E log
                    // (dbcac85) shows that at the syscall-EXIT stop for
                    // `chmod("/proc/cmdline")` (i386 nr=15), rax = 15 (the
                    // syscall number), NOT -ENOSYS (-38), NOT 0, NOT -EPERM.
                    // The SIGSYS handler later sets rax = 0 — but only AFTER
                    // this EXIT log fires, and only IF the kernel delivers
                    // the SIGSYS signal-delivery-stop with the right
                    // timing. Forcing rax = 0 HERE at the EXIT stop is a
                    // belt-and-suspenders fix: it guarantees that even if the
                    // SIGSYS handler's setregs doesn't take effect (for
                    // whatever reason — kernel quirk, ptrace_setregs race,
                    // signal-delivery ordering), the userspace sees rax = 0.
                    //
                    // `syscall_num` was computed at the top of the
                    // SIGTRAP|0x80 block from the SAME `regs` snapshot
                    // we are about to refresh — on every supported
                    // architecture the syscall-number register
                    // (orig_rax on x86_64, orig_eax on i386 compat, x8
                    // on aarch64) is preserved across the syscall, so
                    // `syscall_num` at the EXIT stop still names the
                    // syscall that just returned. The same property is
                    // what lets `pending_getpid` work without having to
                    // re-read the syscall number from registers.
                    //
                    // We do NOT log this branch on every iteration
                    // (only for the first 200) to avoid log spam if
                    // init calls fchown/fchmod in a hot loop.
                    //
                    // Task 6-Z9 diagnostic: UN-GATED log for the xattr
                    // SET family (setxattr/lsetxattr/fsetxattr) so we
                    // can DEFINITIVELY confirm whether the legacy
                    // `compute_exit_return_value` path is reached +
                    // returns Some(0) for these syscalls. This fires
                    // ONLY when the 6-Z9 ENTRY-side rewrite FAILED
                    // (so syscall_num at EXIT is still the original
                    // xattr number); when the rewrite succeeds,
                    // syscall_num at EXIT is `getpid` and this block
                    // is a no-op (compute_exit_return_value(getpid)=None).
                    let _forced_ret_opt = compute_exit_return_value(syscall_num, &abi);
                    if syscall_num == abi.setxattr
                        || syscall_num == abi.lsetxattr
                        || syscall_num == abi.fsetxattr
                    {
                        log(&format!(
                            "DIAG compute_exit_return_value for xattr nr={} [{}] → {:?} (loop_count={}, in_syscall_was_exit=true) — this fires ONLY if the 6-Z9 ENTRY rewrite FAILED (otherwise syscall_num at EXIT would be getpid, not the xattr number)",
                            syscall_num,
                            syscall_name(syscall_num, &abi),
                            _forced_ret_opt,
                            loop_count
                        ));
                    }
                    if let Some(_forced_ret) = _forced_ret_opt {
                        let mut regs2: Regs = unsafe { std::mem::zeroed() };
                        match ptrace_getregs(pid, &mut regs2) {
                            Ok(len) => {
                                let name = syscall_name(syscall_num, &abi);
                                // Task 6-Z9 diagnostic: re-read the
                                // syscall number from the FRESH EXIT-stop
                                // regs2 + compare with `syscall_num`
                                // (computed from the top-of-block `regs`
                                // snapshot). A mismatch means the snapshot
                                // diverged — e.g. a signal-delivery-stop
                                // between ENTRY and EXIT re-snapshotted
                                // regs, or the kernel clobbered orig_rax.
                                let exit_stop_syscall_num = get_syscall_num(&regs2, &abi);
                                if exit_stop_syscall_num != syscall_num {
                                    log(&format!(
                                        "DIAG EXIT syscall_num mismatch: top-of-block syscall_num={} [{}] vs fresh-regs2 syscall_num={} [{}] — snapshot diverged (signal delivery / kernel clobber between ENTRY and EXIT?)",
                                        syscall_num,
                                        syscall_name(syscall_num, &abi),
                                        exit_stop_syscall_num,
                                        syscall_name(exit_stop_syscall_num, &abi)
                                    ));
                                }
                                if loop_count <= 200 {
                                    log(&format!(
                                        "intercepted {}() nr={} at EXIT → faking success (return 0) — original return was EPERM as untrusted_app OR rax=nr leak on i386 compat seccomp-abort",
                                        name, syscall_num
                                    ));
                                }
                                // ── capget: do NOT write to the data buffer.
                                //
                                // We previously tried to populate the
                                // `cap_user_data_t` buffer in the child with
                                // 0xFFFFFFFF via PTRACE_POKEDATA so init
                                // would see "all caps granted". That 8-byte
                                // poke corrupted the child's stack and
                                // caused a SIGSEGV (signal 11). The buffer
                                // pointer passed by init may not actually be
                                // a writable mapped address we can safely
                                // poke (alignment / stack layout
                                // assumptions do not hold in practice).
                                //
                                // Instead we just fake success (return 0)
                                // and leave the buffer untouched. The child
                                // sees "success but no capabilities". This
                                // may cause init to exit, but it will not
                                // crash the process with SIGSEGV — which is
                                // the strictly better failure mode.
                                if syscall_num == abi.capget && loop_count <= 200 {
                                    log("capget: faking success (return 0) without writing data buffer — avoids stack-corrupting PTRACE_POKEDATA");
                                }
                                set_syscall_ret(&mut regs2, &abi, 0);
                                let setregs_result = ptrace_setregs(pid, &regs2, len);
                                if let Err(e) = setregs_result {
                                    // 5-J diagnostic: surface silent setregs failures
                                    // (previously discarded with `let _ =` — a
                                    // failed setregs here leaves rax = the kernel's
                                    // syscall-number leak value, e.g. 15 for i386
                                    // chmod, and init takes the chmod-error path
                                    // → SIGSEGV at rip=0x809255d).
                                    log(&format!(
                                        "EXIT handler: ptrace_setregs FAILED for {} (nr={}): {} — child will see kernel's leaked rax, not our faked 0",
                                        name, syscall_num, e
                                    ));
                                }
                                // ── 5-J diagnostic readback ──
                                //
                                // Re-read rax IMMEDIATELY after our setregs to
                                // confirm the write stuck. If the kernel clobbers
                                // rax between our setregs and this readback (e.g.
                                // because the SIGSYS signal-delivery-stop is
                                // already pending and the kernel re-snapshots
                                // regs from `syscall_rollback` which sets
                                // rax = orig_rax = the syscall number), we'll see
                                // a non-zero value here. This is the smoking gun
                                // 5-H asked the next investigation agent to look
                                // for: "Add a log AFTER set_syscall_ret(...) and
                                // after ptrace_setregs to confirm the writeback
                                // happened".
                                //
                                // Gated by `loop_count <= 300` so we capture the
                                // chmod(/proc/cmdline) at post-execve syscall #50
                                // (iter ~216 per 5-H's log) without flooding
                                // logcat for the later fchown/fchmod hot loop.
                                //
                                // Task 6-Z9: UN-GATED for the xattr SET family
                                // (the current restorecon-loop blocker) so the
                                // readback fires regardless of loop_count.
                                if loop_count <= 300
                                    || syscall_num == abi.setxattr
                                    || syscall_num == abi.lsetxattr
                                    || syscall_num == abi.fsetxattr
                                {
                                    let mut readback: Regs = unsafe { std::mem::zeroed() };
                                    if ptrace_getregs(pid, &mut readback).is_ok() {
                                        let readback_rax =
                                            get_syscall_arg(&readback, abi.reg_ret) as i64;
                                        if loop_count <= 200 {
                                            log(&format!(
                                                "[KR64][ptrace] EXIT handler wrote rax=0 for {} (nr={}), readback rax={}",
                                                name, syscall_num, readback_rax
                                            ));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                // Task 6-Z9: surface the previously-SILENT
                                // ptrace_getregs failure. Pre-6-Z9, if this
                                // fresh getregs failed, the fake was skipped
                                // with NO log — leaving the dispatcher unable
                                // to tell whether the fake ran. This is the
                                // most likely root cause of the pre-6-Z9
                                // "fake not applied" symptom (if the legacy
                                // path was the only fake): a transient
                                // ptrace_getregs failure (e.g. the child was
                                // briefly in an unrecoverable state) would
                                // silently drop the fake, the child would
                                // see the kernel's raw EPERM/EACCES/EOPNOTSUPP,
                                // and the restorecon loop would spin.
                                let name = syscall_name(syscall_num, &abi);
                                log(&format!(
                                    "DIAG compute_exit_return_value: ptrace_getregs FAILED for {} (nr={}): {} — fake SKIPPED, child will see kernel's raw return (EPERM/EACCES/EOPNOTSUPP for xattr SET)",
                                    name, syscall_num, e
                                ));
                            }
                        }
                    }

                    // ── Task 6-Z3: socketcall error-return fake-success ─────
                    //
                    // i386 socketcall (nr=102) is a multiplexed socket
                    // syscall: arg1 = sub-call number (1=socket, 2=bind,
                    // 3=connect, 4=listen, 5=accept, ...); arg2 = pointer
                    // to an array of the sub-call's args. TWRP init (an
                    // i386 binary) calls socketcall(2=bind, fd, sockaddr,
                    // addrlen) to bind the property_service socket during
                    // its property service startup.
                    //
                    // ROOT CAUSE: the bind returns EADDRINUSE (-98) because
                    // a stale socket fd from a PREVIOUS relaunch cycle is
                    // still bound in the parent (the twoyi app), inherited
                    // via fork and NOT closed before forking the guest.
                    // The child's close loop (fds 3..1024) closes the
                    // CHILD's fds, not the parent's → the parent's stale
                    // fd keeps the address bound → EADDRINUSE → "Failed
                    // to bind socket 'property_service': Address already
                    // in use" → "init: init startup failure" →
                    // exit_group(1). Verified on b492c65 UI E2E run
                    // 32212585042 (syscall #378).
                    //
                    // FIX (SIMPLEST pragmatic approach): at the syscall-
                    // EXIT stop, if syscall_num == abi.socketcall_nr AND
                    // the return value is negative (error), fake the
                    // return to 0 (success). This catches the bind
                    // EADDRINUSE + the listen failure (if bind was
                    // faked, the socket isn't really bound, so listen
                    // would fail too — we fake it) + any other failing
                    // socketcall sub-call (connect/accept/...).
                    //
                    // It does NOT fake socket() itself: socket()
                    // returns a POSITIVE fd on success (the normal case
                    // — the zygote allows it), and only negative
                    // returns trigger the fake. The property service
                    // doesn't actually need to WORK in the sandbox —
                    // init just needs to THINK it started (the property
                    // AREA is mapped separately via the mmap2 REWRITE
                    // from Task 6-Y, and ro.* properties are written
                    // directly to the area by init's property_init,
                    // NOT via the socket). Non-ro property SET goes via
                    // the socket, but TWRP doesn't depend on non-ro
                    // sets for boot — ro.* (set directly to the area)
                    // is sufficient.
                    //
                    // NOTE on the -1 sentinels: on x86_64 + aarch64
                    // socketcall_nr is -1. No real syscall is ever -1,
                    // so the `syscall_num == abi.socketcall_nr` check
                    // can never spuriously match -1 on those ABIs.
                    // Mirrors the existing ABI_AARCH64.mknod = -1
                    // precedent in compute_exit_return_value (see
                    // the doc on `mknod` in `ChildAbi` for the same
                    // "harmless if no real syscall is -1" reasoning).
                    if abi.socketcall_nr != -1 && syscall_num == abi.socketcall_nr {
                        let ret = get_syscall_arg(&regs, abi.reg_ret) as i64;
                        if ret < 0 && ret > -4096 {
                            // Negative return in the -errno range —
                            // fake it to 0 (success). Use a FRESH
                            // ptrace_getregs so we write to the live
                            // register state (mirrors the
                            // compute_exit_return_value block + the
                            // 6-W SIGSYS-handler pattern — avoids the
                            // stale-value race).
                            let mut regs2: Regs = unsafe { std::mem::zeroed() };
                            if let Ok(len) = ptrace_getregs(pid, &mut regs2) {
                                set_syscall_ret(&mut regs2, &abi, 0);
                                if let Err(e) = ptrace_setregs(pid, &regs2, len) {
                                    log(&format!(
                                        "DIAG socketcall fake-success FAILED: ptrace_setregs for nr={} (ret={}): {} — child will see the error",
                                        syscall_num, ret, e
                                    ));
                                } else {
                                    // Log every socketcall fake so we
                                    // can see which sub-calls (bind/
                                    // listen/...) are being faked at
                                    // runtime — UN-gated by loop_count
                                    // because the property_service
                                    // bind happens once per relaunch
                                    // cycle (very low volume).
                                    log(&format!(
                                        "DIAG socketcall fake-success: socketcall (nr={}) returned {} (-errno {}) — faked to 0 (stale socket from previous cycle; init will proceed)",
                                        syscall_num, ret, -ret
                                    ));
                                }
                            }
                        }
                    }

                    // ── Task 6-Z5: poll positive-return fake-success ───
                    //
                    // Legacy poll() syscall (i386 nr=168, x86_64 nr=7,
                    // aarch64 sentinel -1). TWRP's recovery (an i386
                    // binary) calls poll() in its property_service
                    // startup loop AFTER the bind has been faked to 0
                    // (Task 6-Z3). The 6-Z3 socketcall fake-success
                    // masked the bind EADDRINUSE to 0, but the socket
                    // is NOT actually bound (concurrent kr64 invocations
                    // — the twoyi app relaunches kr64 every 2s without
                    // killing the old one → the old init's socket is
                    // still bound → new init's bind fails → faked to 0
                    // → the socket isn't bound → poll returns POLLERR=1
                    // → busy-wait). Verified on a76b677 UI E2E run
                    // 32218145762: poll (i386 nr=168) × 101 at syscalls
                    // #4900-5000, each returns 1 (a fd is ALWAYS ready
                    // with POLLERR). The recovery is alive but stuck in
                    // a TIGHT POLL SPIN — it never proceeds to open
                    // /dev/graphics/fb0 (no framebuffer render).
                    //
                    // FIX (PRAGMATIC): at the syscall-EXIT stop, if
                    // syscall_num == abi.poll_nr AND the return value
                    // is POSITIVE (N fds ready), fake the return to 0
                    // (no fds ready — equivalent to a timeout). This
                    // stops the busy-wait: the recovery thinks no
                    // events are pending + either sleeps (if the poll
                    // timeout is non-zero) or retries less aggressively.
                    //
                    // CAVEAT: the TWRP main UI loop ALSO uses poll (for
                    // input events). Faking ALL poll returns to 0 would
                    // prevent the TWRP UI from processing input. BUT
                    // the recovery is currently stuck BEFORE the TWRP
                    // UI loop (in a setup/property-service poll spin).
                    // Faking poll to 0 should let it proceed PAST the
                    // setup loop. Once the recovery reaches the
                    // framebuffer phase, the poll behaviour might
                    // differ. If this fix causes a regression (the TWRP
                    // UI can't process input), a follow-up fix can make
                    // the fake conditional (only during the setup phase,
                    // not the main loop).
                    //
                    // NOTE on the -1 sentinel: on aarch64 poll_nr is -1.
                    // No real syscall is ever -1, so the explicit
                    // `abi.poll_nr != -1` gate keeps the
                    // `syscall_num == abi.poll_nr` check from spuriously
                    // matching -1 on aarch64. Mirrors the existing
                    // socketcall block's gate (see Task 6-Z3 comment
                    // above) + the existing ABI_AARCH64.mknod = -1 /
                    // .open = -1 / .access = -1 precedent.
                    if abi.poll_nr != -1 && syscall_num == abi.poll_nr {
                        let ret = get_syscall_arg(&regs, abi.reg_ret) as i64;
                        if ret > 0 {
                            // Positive return = N fds ready — fake it to
                            // 0 (no fds ready — timeout). Use a FRESH
                            // ptrace_getregs so we write to the live
                            // register state (mirrors the
                            // compute_exit_return_value block + the
                            // 6-Z3 socketcall fake-success block above +
                            // the 6-W SIGSYS-handler pattern — avoids
                            // the stale-value race).
                            let mut regs2: Regs = unsafe { std::mem::zeroed() };
                            if let Ok(len) = ptrace_getregs(pid, &mut regs2) {
                                set_syscall_ret(&mut regs2, &abi, 0);
                                if let Err(e) = ptrace_setregs(pid, &regs2, len) {
                                    log(&format!(
                                        "DIAG poll fake-success FAILED: ptrace_setregs for nr={} (ret={}): {} — child will see the positive return",
                                        syscall_num, ret, e
                                    ));
                                } else {
                                    // Log every poll fake so we can see
                                    // the busy-wait being broken. Gated
                                    // by loop_count to avoid log
                                    // flooding (the spin can be 100s of
                                    // thousands of polls/sec).
                                    if loop_count <= 6000 {
                                        log(&format!(
                                            "DIAG poll fake-success: poll returned {} (fds ready) → faked to 0 (no events; stops POLLERR busy-wait from faked property_service socket)",
                                            ret
                                        ));
                                    }
                                }
                                // Task 6-Z17: ALSO zero the revents field
                                // in each pollfd struct in the child's
                                // pollfd array (arg1=pollfd*, arg2=nfds).
                                // The kernel wrote POLLERR into revents
                                // BEFORE the EXIT stop; faking the return
                                // value to 0 (above) tells init "no fds
                                // ready" but init ALSO reads revents from
                                // the pollfd struct — if revents still has
                                // POLLERR, init retries immediately →
                                // busy-wait (verified on 5e0f157: poll
                                // faked to 0 but init kept spinning).
                                let pollfd_ptr = get_syscall_arg(&regs2, abi.reg_arg1);
                                let nfds = get_syscall_arg(&regs2, abi.reg_arg2);
                                let zeroed = zero_pollfd_revents(pid, pollfd_ptr, nfds);
                                if zeroed > 0 && loop_count <= 6000 {
                                    log(&format!(
                                        "DIAG poll revents zeroed: cleared POLLERR from {} pollfd entr{} (pollfd_ptr={:#x}, nfds={}) — init will see no events in revents too",
                                        zeroed, if zeroed == 1 { "y" } else { "ies" }, pollfd_ptr, nfds
                                    ));
                                }
                            }
                        }
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
                // ── Bug 1 fix: in_syscall flag desync ──────────────
                //
                // DIAGNOSTIC: log the in_syscall state at SIGSYS time.
                // Normally SIGSYS fires BETWEEN the ENTRY and EXIT
                // stops (seccomp traps the syscall mid-flight), so
                // in_syscall is already true here. But on some kernels
                // SECCOMP_RET_TRAP can fire BEFORE the ptrace ENTRY
                // stop is delivered — in that case in_syscall is false,
                // and without the fix below the next SIGTRAP|0x80 stop
                // (the EXIT stop) would be misclassified as ENTRY,
                // permanently desyncing the loop. This is exactly the
                // bug that caused ioprio_get's EXIT intercept to never
                // fire: the EXIT was being treated as an ENTRY.
                //
                // FIX: ALWAYS set in_syscall = true after processing
                // the SIGSYS (see the `in_syscall = true` line right
                // before `continue` below). This ensures the next stop
                // is treated as EXIT regardless of whether SIGSYS
                // fired before or after the ENTRY stop.
                //
                // NOTE: the in_syscall DESYNC diagnostic log that used
                // to live here is now emitted AFTER the rate-limit
                // check inside the `Ok(len) =>` branch below. The
                // per-SIGSYS log output is rate-limited so a tight
                // SIGSYS loop on a single syscall number (e.g. shmget
                // returning 0 → init retries) does not flood logcat
                // and OOM the Java FileLogger-Kr64Tee thread — see the
                // comment on `last_sigsys_nr` / `sigsys_repeat_count`
                // at the top of `run_ptrace_loop` for the full
                // rationale.
                // Capture the in_syscall state at SIGSYS entry. 6-W uses
                // this to decide whether to do a FRESH `ptrace_getregs`
                // before setregs (DESYNC mode → fresh getregs; NORMAL mode
                // → no fresh getregs needed — the SIGSYS-entry getregs
                // below is already current).
                //
                // `in_syscall` is true when SIGSYS fires BETWEEN ENTRY and
                // EXIT (normal — SIGSYS replaces the syscall-exit-stop). It
                // is false when SIGSYS fires AFTER the EXIT stop (DESYNC —
                // the kernel delivered ENTRY→EXIT→SIGSYS for a single
                // seccomp-trapped syscall, which is the order 5-H's log
                // evidence shows for i386 compat chmod nr=15).
                //
                // Historical context (5-J/6-C): this was originally
                // captured for `should_skip_sigsys_setregs`, which (in
                // DESYNC mode) caused the SIGSYS handler to SKIP its
                // `ptrace_setregs` call. 6-W REVERTED that skip (it caused
                // the iter-826 rodata-leak SIGSEGV at rip=0x6f722f69) —
                // see `should_skip_sigsys_setregs` for the full 5-J → 6-C
                // → 6-W evolution. The variable is still captured (and
                // still gates the fresh-getregs branch below) because the
                // DESYNC vs NORMAL distinction is still meaningful for
                // deciding whether a fresh getregs is needed.
                //
                // Captured early (rather than reading `in_syscall` at the
                // setregs call site) to make the DESYNC decision explicit
                // and robust against any future code that mutates
                // `in_syscall` between SIGSYS entry and setregs.
                let in_syscall_at_sigsys = in_syscall;
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

                        // ── SIGSYS log rate-limiting ──
                        //
                        // Track how many times the SAME syscall number
                        // has fired SIGSYS in a row. After 5 repetitions
                        // we suppress ALL per-SIGSYS log output to
                        // prevent flooding logcat (which OOMs the Java
                        // FileLogger-Kr64Tee thread — see the comment on
                        // `last_sigsys_nr` at the top of run_ptrace_loop).
                        // Every 100 suppressed iterations we emit ONE
                        // summary line so the operator can still see the
                        // loop is ongoing. The actual SIGSYS handling
                        // (return-value forcing below) is NOT gated —
                        // only the log() calls are, via the `sigsys_log`
                        // closure defined below.
                        if original_syscall == last_sigsys_nr {
                            sigsys_repeat_count = sigsys_repeat_count.saturating_add(1);
                        } else {
                            last_sigsys_nr = original_syscall;
                            sigsys_repeat_count = 1;
                        }
                        let suppress_log = sigsys_repeat_count > 5;
                        if suppress_log {
                            sigsys_suppressed_total = sigsys_suppressed_total.saturating_add(1);
                            // Emit ONE summary line every 100 suppressed
                            // iterations (1, 101, 201, …) so the operator
                            // can still see the loop is ongoing. Uses the
                            // outer `log` directly — this summary must
                            // NOT be suppressed by `sigsys_log`.
                            if sigsys_suppressed_total % 100 == 1 {
                                log(&format!(
                                    "suppressing repeated SIGSYS for nr={} [{}] (repeat_count={}, suppressed_total={})",
                                    original_syscall,
                                    name,
                                    sigsys_repeat_count,
                                    sigsys_suppressed_total
                                ));
                            }
                        }

                        // Rate-limited log helper. Every per-SIGSYS log
                        // call below uses `sigsys_log` instead of `log`
                        // so that a tight SIGSYS loop on a single syscall
                        // number does not flood logcat. The closure
                        // captures `suppress_log` (by shared ref — it is
                        // not mutated after the check above) and `log`
                        // (by shared ref — `log` is `Fn(&str)`).
                        let sigsys_log = |msg: &str| {
                            if !suppress_log {
                                log(msg);
                            }
                        };

                        // in_syscall DESYNC diagnostic — moved here from
                        // before the ptrace_getregs call so it can be
                        // gated by the rate-limit check above. See the
                        // "Bug 1 fix: in_syscall flag desync" comment
                        // earlier in this SIGSYS handler for the full
                        // rationale.
                        //
                        // 5-J update: the message text now matches the
                        // actual code path. When `in_syscall == false`
                        // here it means the kernel delivered ENTRY →
                        // EXIT → SIGSYS for a single seccomp-trapped
                        // syscall (the order 5-H's log evidence shows
                        // for i386 compat chmod nr=15) — the EXIT
                        // handler has ALREADY run and written rax=0.
                        // The post-SIGSYS code below sets `in_syscall =
                        // false` (NOT `true` as the stale message used
                        // to claim) so the next SIGTRAP|0x80 is treated
                        // as the ENTRY of the NEXT syscall.
                        sigsys_log(&format!(
                            "SIGSYS handler: in_syscall={} before processing{}",
                            in_syscall_at_sigsys,
                            if in_syscall_at_sigsys {
                                " (normal — SIGSYS fired between ENTRY and EXIT)"
                            } else {
                                " (DESYNC — SIGSYS fired AFTER EXIT stop; 6-W fix: fresh ptrace_getregs + ptrace_setregs will run so rax=ret_val is written AND other registers are re-written with current values, preventing the rodata-leak SIGSEGV that the 5-J skip caused)"
                            }
                        ));

                        // Record the intercepted syscall in the rolling
                        // "all syscalls" buffer too — seccomp-blocked
                        // syscalls do NOT produce a syscall-entry stop
                        // (the SIGSYS stop replaces it), so without
                        // recording here the buffer would miss every
                        // intercepted syscall. We record the ORIGINAL
                        // syscall number (what init asked for), not the
                        // rewritten getpid number, so the buffer reflects
                        // what init was actually trying to do.
                        if recent_all_syscalls.len() == RECENT_ALL_SYSCALLS_CAP {
                            recent_all_syscalls.pop_front();
                        }
                        recent_all_syscalls.push_back(original_syscall);

                        // For access(): read BOTH the path argument
                        // (REG_ARG1) AND the mode argument (REG_ARG2).
                        // On x86_64 access(pathname, mode) puts the path
                        // pointer in rdi (REG_ARG1=14) and the mode int
                        // in rsi (REG_ARG2=13). At a SIGSYS stop,
                        // however, the argument registers may have been
                        // clobbered by the seccomp trap dance — we have
                        // observed the path showing up as the literal
                        // string "mode=0755" instead of the actual path.
                        // To diagnose which register holds what, we read
                        // both as a pointer-to-string AND log both raw
                        // register values. On aarch64 `a.access` is -1
                        // (aarch64 uses faccessat instead), so this
                        // branch is dead on that architecture — the
                        // comparison still compiles and is harmless.
                        let (access_path, access_path_from_arg2): (Option<String>, Option<String>) =
                            if original_syscall == a.access {
                                let path_addr_arg1 = get_syscall_arg(&sigsys_regs, a.reg_arg1);
                                let path_addr_arg2 = get_syscall_arg(&sigsys_regs, a.reg_arg2);
                                let s1 = read_child_string(pid, path_addr_arg1);
                                let s2 = read_child_string(pid, path_addr_arg2);
                                sigsys_log(&format!(
                                    "access() SIGSYS raw args: reg_arg1={:#x} reg_arg2={:#x} str(arg1)={:?} str(arg2)={:?}",
                                    path_addr_arg1, path_addr_arg2, s1, s2
                                ));
                                (s1, s2)
                            } else {
                                (None, None)
                            };

                        // Push a human-readable description into the rolling
                        // history so the exit handler can print "last N
                        // blocked syscalls". For access() we include the
                        // path being probed — that is the single most
                        // useful diagnostic when init dies after a flurry
                        // of access() calls (it tells us which file init
                        // was looking for and couldn't find).
                        let history_entry: String = if original_syscall == a.access {
                            // Include BOTH the REG_ARG1 string and the
                            // REG_ARG2 string in the history entry so we
                            // can tell (from the rolling SIGSYS log on
                            // exit) which register held the path and
                            // which held the mode. Until we know for
                            // sure which register to trust at the SIGSYS
                            // stop, recording both is the only way to
                            // recover the path init was probing.
                            match (&access_path, &access_path_from_arg2) {
                                (Some(p1), Some(p2)) => {
                                    format!(
                                        "access(arg1={:?}, arg2={:?}) nr={}",
                                        p1, p2, original_syscall
                                    )
                                }
                                (Some(p), None) => {
                                    format!("access(arg1={:?}) nr={}", p, original_syscall)
                                }
                                (None, Some(p)) => {
                                    format!("access(arg2={:?}) nr={}", p, original_syscall)
                                }
                                (None, None) => {
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
                        //
                        // EXCEPTION — fchown / fchmod / capget /
                        // ioprio_get / ioprio_set: same rationale as
                        // rt_sigprocmask above. These syscalls take
                        // user-space pointers or integer args that
                        // bionic's linker init code inspects on
                        // return; if we rewrite the syscall to getpid
                        // the kernel may EXECUTE getpid and overwrite
                        // our 0 return value with the child's real
                        // PID, which init interprets as a failure.
                        // Leaving the original (blocked) syscall
                        // number in place and just forcing the return
                        // to 0 is the same pattern that already works
                        // for rt_sigprocmask — and it covers the
                        // (rare) case where a device's seccomp filter
                        // blocks these syscalls outright. On most
                        // devices these are NOT blocked by seccomp
                        // (they execute and return EPERM); the
                        // syscall-EXIT handler above fakes success
                        // there.
                        // ── NEVER rewrite orig_rax ──
                        //
                        // Previous behaviour: rewrite the syscall number
                        // to `getpid` for all syscalls EXCEPT an explicit
                        // "no-rewrite" list (rt_sigprocmask, fchown,
                        // fchmod, capget, ioprio_get, ioprio_set). The
                        // theory was that rewriting prevents the kernel
                        // from re-evaluating the original (blocked)
                        // syscall and re-raising
                        // SIGSYS on resume.
                        //
                        // REALITY (observed in c87d6be7 E2E run): for
                        // seccomp-aborted syscalls the kernel does NOT
                        // re-evaluate the syscall number — it goes straight
                        // to syscall-exit. So the rewrite is unnecessary.
                        // Worse, on i386 compat children the rewrite
                        // causes the kernel to return -ENOSYS (-38)
                        // instead of our faked 0 — every mount/mkdir/mknod
                        // appears to fail with ENOSYS, and TWRP init
                        // exits(1) after 36 syscalls.
                        //
                        // The fix: NEVER rewrite orig_rax. Just set rax=0
                        // (or the per-syscall fake return value below).
                        // This matches the proven pattern for
                        // rt_sigprocmask, which has always been in the
                        // "no-rewrite" list and works correctly.

                        // Decide on the return value based on the original
                        // syscall. See the long comment above for the full
                        // rationale per-syscall.
                        //
                        // On aarch64 `a.access` is -1 (aarch64 uses
                        // faccessat instead), so the `access` branch is
                        // effectively dead on that architecture — the
                        // comparison still compiles and is harmless.
                        //
                        // Task 6-G: track CONSECUTIVE pause() SIGSYS calls
                        // so the pause branch below can return -ETIMEDOUT
                        // after PAUSE_TIMEOUT_THRESHOLD (50) retries
                        // instead of -ENOSYS. Reset on every non-pause
                        // SIGSYS so the counter only counts consecutive
                        // pauses (see the doc on `pause_count` at the top
                        // of run_ptrace_loop for why we do NOT also reset
                        // on SIGTRAP|0x80 stops).
                        if original_syscall == a.pause {
                            pause_count = pause_count.saturating_add(1);
                        } else {
                            pause_count = 0;
                        }
                        let ret_val: i64 = if original_syscall == a.access {
                            let path_display = access_path.as_deref().unwrap_or("?");
                            let path_arg2_display = access_path_from_arg2.as_deref().unwrap_or("?");
                            sigsys_log(&format!(
                                "intercepted SIGSYS — access(arg1={}, arg2={}) nr={} → returning 0 (success, NOT rewriting orig_rax)",
                                path_display, path_arg2_display, original_syscall
                            ));
                            0
                        } else if original_syscall == a.rt_sigprocmask {
                            sigsys_log(&format!(
                                "intercepted SIGSYS — rt_sigprocmask() nr={} [{}] (NOT rewriting — seccomp already aborted the syscall, returning 0 — signal mask emulation)",
                                original_syscall, name
                            ));
                            0
                        } else if original_syscall == a.fchown
                            || original_syscall == a.fchmod
                            || original_syscall == a.capget
                            || original_syscall == a.ioprio_get
                            || original_syscall == a.ioprio_set
                            || original_syscall == a.lchown
                            || original_syscall == a.chown
                            || original_syscall == a.fchmodat
                            || original_syscall == a.fchownat
                        {
                            // chmod/lchown/chown/fchmodat/fchownat/
                            // fchown/fchmod/capget/ioprio_get/ioprio_set
                            // — same rationale as the syscall-EXIT
                            // handler's `compute_exit_return_value`
                            // set (see the "TWRP-init EPERM /
                            // syscall-number-leak workaround" comment
                            // above). We use the explicit `||` chain
                            // here (instead of calling
                            // `compute_exit_return_value`) only because
                            // we ALSO need to exclude `chmod` from this
                            // block — chmod is handled by the
                            // mount/mkdir/chmod/chroot/unshare block
                            // below (which performs an fs op in the
                            // rootfs and uses the same fake-success
                            // return value 0).
                            sigsys_log(&format!(
                                "intercepted SIGSYS — {}() nr={} [{}] (NOT rewriting — seccomp already aborted the syscall, returning 0 — fake success for TWRP init)",
                                name, original_syscall, name
                            ));
                            0
                        } else if original_syscall == a.mount
                            || original_syscall == a.mkdir
                            || original_syscall == a.chmod
                            || original_syscall == a.chroot
                            || original_syscall == a.unshare
                            || original_syscall == a.mknod
                        {
                            // Filesystem-related seccomp-blocked syscalls.
                            // These are the ones TWRP init calls during
                            // early boot (mount tmpfs/proc/sysfs, mkdir
                            // /dev/pts, mknod /dev/null etc.).
                            //
                            // TWO-PRONGED FIX:
                            // 1. Fake success (return 0) WITHOUT rewriting
                            //    orig_rax. The previous "rewrite to getpid"
                            //    strategy caused the kernel to return -ENOSYS
                            //    on i386 compat.
                            // 2. ALSO perform the actual filesystem operation
                            //    in the rootfs (mkdir for mount/mkdir, empty-
                            //    file creation for mknod). This ensures the
                            //    filesystem state is correct for subsequent
                            //    opens, even if the kernel returns -ENOSYS to
                            //    init. Without this, init's later
                            //    open(/dev/X) would fail with ENOENT because
                            //    the seccomp-faked mount/mkdir/mknod didn't
                            //    actually create anything.
                            //
                            // NOTE on the mknod branch (Task 5-X): the
                            // guest's NEXT open(/dev/null) after mknod
                            // would otherwise fail with ENOENT — the
                            // seccomp-faked mknod returned 0 (success)
                            // but no actual device node exists. We
                            // create an EMPTY regular file (NOT a real
                            // device node — mknod(2) on the host would
                            // need CAP_MKNOD which untrusted_app lacks)
                            // so the guest's subsequent open() succeeds.
                            // This is a best-effort stub: empty-file
                            // creation is sufficient for /dev/null
                            // (writes succeed as no-op, reads return
                            // EOF) but gives WRONG read-content for
                            // /dev/zero (reads return 0 bytes instead
                            // of \0-bytes) and /dev/urandom (reads
                            // return 0 bytes instead of random bytes).
                            // Good enough to get init past the mknod
                            // failure + subsequent open; if a later
                            // TWRP code path actually reads from
                            // /dev/zero or /dev/urandom expecting real
                            // content, that's the next-next blocker.

                            // Perform the actual filesystem operation in
                            // the rootfs. We read the path argument(s) from
                            // the child's memory and translate them to
                            // rootfs-relative paths.
                            if original_syscall == a.mount {
                                // mount(source, target, fstype, flags, data)
                                // arg1=source, arg2=target, arg3=fstype
                                let tgt_addr = get_syscall_arg(&sigsys_regs, a.reg_arg2);
                                let fs_addr = get_syscall_arg(&sigsys_regs, a.reg_arg3);
                                if tgt_addr != 0 {
                                    if let Some(tgt) = read_child_string(pid, tgt_addr) {
                                        let fstype = if fs_addr != 0 {
                                            read_child_string(pid, fs_addr).unwrap_or_default()
                                        } else {
                                            String::new()
                                        };
                                        // Translate target to rootfs-relative
                                        let real_tgt = if tgt.starts_with('/') {
                                            format!("{}{}", rootfs, tgt)
                                        } else {
                                            tgt.clone()
                                        };
                                        // For tmpfs mounts, create the target
                                        // directory (init expects a fresh
                                        // tmpfs to exist at the mount point).
                                        // For proc/sysfs/devpts, the host
                                        // already has these — skip.
                                        if fstype == "tmpfs" || fstype == "devpts" {
                                            match std::fs::create_dir_all(&real_tgt) {
                                                Ok(()) => sigsys_log(&format!(
                                                    "SIGSYS mount: created directory {} (fstype={}) in rootfs",
                                                    real_tgt, fstype
                                                )),
                                                Err(e) => sigsys_log(&format!(
                                                    "SIGSYS mount: FAILED to create {} (fstype={}): {}",
                                                    real_tgt, fstype, e
                                                )),
                                            }
                                        }
                                    }
                                }
                            } else if original_syscall == a.mkdir {
                                // mkdir(path, mode) — arg1=path
                                let path_addr = get_syscall_arg(&sigsys_regs, a.reg_arg1);
                                if path_addr != 0 {
                                    if let Some(path) = read_child_string(pid, path_addr) {
                                        let real_path = if path.starts_with('/') {
                                            format!("{}{}", rootfs, path)
                                        } else {
                                            path.clone()
                                        };
                                        match std::fs::create_dir_all(&real_path) {
                                            Ok(()) => sigsys_log(&format!(
                                                "SIGSYS mkdir: created directory {} in rootfs",
                                                real_path
                                            )),
                                            Err(e) => sigsys_log(&format!(
                                                "SIGSYS mkdir: FAILED to create {}: {}",
                                                real_path, e
                                            )),
                                        }
                                    }
                                }
                            } else if original_syscall == a.mknod {
                                // mknod(pathname, mode, dev) — arg1=pathname
                                // (Task 5-X). TWRP init calls mknod() for
                                // /dev/null, /dev/zero, /dev/urandom etc.
                                // during early boot. As untrusted_app we
                                // can't actually mknod a device node (no
                                // CAP_MKNOD — host mknod would return
                                // EPERM, and on Android the syscall is also
                                // blocked by seccomp → SIGSYS), so we
                                // create an EMPTY regular file at the
                                // path instead. The guest's subsequent
                                // open(/dev/null) succeeds (ENOENT would
                                // otherwise trip init's "device node
                                // missing" fatal path). Best-effort stub:
                                // see the long comment above for the
                                // /dev/zero and /dev/urandom caveat.
                                //
                                // We create the parent directory first
                                // (create_dir_all on the parent) so that
                                // mknod("/dev/null") succeeds even when
                                // /dev doesn't exist in the rootfs yet —
                                // mirroring the mount/mkdir block's
                                // create_dir_all behaviour.
                                let path_addr = get_syscall_arg(&sigsys_regs, a.reg_arg1);
                                if path_addr != 0 {
                                    if let Some(path) = read_child_string(pid, path_addr) {
                                        let real_path = if path.starts_with('/') {
                                            format!("{}{}", rootfs, path)
                                        } else {
                                            path.clone()
                                        };
                                        // Create the parent directory
                                        // (e.g. /data/.../rootfs/dev) so
                                        // File::create succeeds. Ignore
                                        // errors — if the parent already
                                        // exists (the usual case after the
                                        // first mknod in the same /dev
                                        // directory), create_dir_all is a
                                        // no-op; if it fails for some other
                                        // reason, the File::create below
                                        // will fail and log the error.
                                        if let Some(parent) =
                                            std::path::Path::new(&real_path).parent()
                                        {
                                            let _ = std::fs::create_dir_all(parent);
                                        }
                                        match std::fs::File::create(&real_path) {
                                            Ok(_) => sigsys_log(&format!(
                                                "SIGSYS mknod: created empty file stub {} in rootfs (guest open() will succeed; reads return EOF — best-effort stub for /dev/null, /dev/zero, /dev/urandom)",
                                                real_path
                                            )),
                                            Err(e) => sigsys_log(&format!(
                                                "SIGSYS mknod: FAILED to create empty file stub {}: {}",
                                                real_path, e
                                            )),
                                        }
                                    }
                                }
                            }
                            sigsys_log(&format!(
                                "intercepted SIGSYS — {}() nr={} [{}] (NOT rewriting orig_rax — seccomp aborted, returning 0 — fake success + performed fs op in rootfs)",
                                name, original_syscall, name
                            ));
                            0
                        } else if original_syscall == a.shmget
                            || original_syscall == a.shmat
                            || original_syscall == a.shmctl
                        {
                            // SysV shared-memory syscalls (shmget /
                            // shmat / shmctl). TWRP init calls shmget()
                            // during early boot for Android's
                            // __system_property_area_init, which uses a
                            // SysV shared memory segment for the property
                            // file.
                            //
                            // The host's seccomp filter blocks these
                            // (not in the untrusted_app allow list),
                            // raising SIGSYS. Returning 0 (fake success)
                            // for shmget causes init to loop forever:
                            // shmid=0 is not a valid shmid, so init
                            // retries shmget — observed in the 0a4be80
                            // E2E run as a 13k-iteration SIGSYS loop
                            // that OOM'd the Java FileLogger-Kr64Tee
                            // thread.
                            //
                            // Returning -ENOSYS (-38) tells init "this
                            // kernel does not implement SysV shared
                            // memory", which makes bionic fall back to a
                            // non-shared-memory property area
                            // initialization path (the same fallback
                            // used on kernels built without
                            // CONFIG_SYSVIPC). This is the correct
                            // emulation for an unprivileged rootless
                            // container.
                            sigsys_log(&format!(
                                "intercepted SIGSYS — {}() nr={} [{}] (NOT rewriting orig_rax — returning -ENOSYS so init falls back to non-shared-memory property init)",
                                name, original_syscall, name
                            ));
                            -(libc::ENOSYS as i64)
                        } else if original_syscall == a.pause {
                            // pause() — Task 6-D (initial -EINTR) +
                            // Task 6-E (changed to -ENOSYS). TWRP
                            // init's __system_property_area_init code
                            // calls pause() in a loop while waiting for
                            // the property service to signal it has set
                            // up /dev/__properties__. On the host's
                            // untrusted_app seccomp filter pause() is
                            // blocked (i386 syscall 29), raising SIGSYS.
                            // The kernel's own pause() can ONLY ever
                            // return -EINTR (errno 4) — there is no
                            // "successful" return from pause(); it
                            // blocks until interrupted by a signal.
                            //
                            // 6-D (commit 2b073f8) tried returning
                            // -EINTR (-4): this makes init think
                            // "interrupted by a signal" → check the
                            // condition (property service not ready) →
                            // call pause() again → INFINITE LOOP. The
                            // UI E2E test on 2b073f8 shows the pause
                            // loop is STILL there (992,000+ repeats) —
                            // -EINTR did NOT break the loop. The
                            // property service will NEVER signal
                            // readiness because kr64 has NO property
                            // service (5-Y's find_property binary patch
                            // makes lookups return NULL, but there's no
                            // actual service to send the "ready"
                            // signal). So -EINTR's "check + retry"
                            // semantics are exactly the wrong shape:
                            // they guarantee an infinite loop.
                            //
                            // Task 6-E: return -ENOSYS (-38) instead.
                            // This tells init "this kernel does not
                            // implement pause()" → init falls back to
                            // a non-pause wait mechanism (or skips the
                            // wait entirely). This mirrors how 6-C's
                            // shmget -ENOSYS made init fall back to
                            // non-shared-memory property init (which
                            // WORKED — the shmget loop stopped). The
                            // same fallback pattern should break the
                            // pause loop here. Returning 0 (the
                            // pre-6-D default) is ALSO wrong: init
                            // interprets 0 as "pause completed WITHOUT
                            // a signal" → re-checks its condition →
                            // calls pause() again → INFINITE LOOP (the
                            // post-6-C UI E2E blocker, 1,048,000+
                            // repeats on commit 368f59b).
                            //
                            // pause() is NOT in
                            // compute_exit_return_value's fake-success
                            // list — it returns -ENOSYS via this
                            // dedicated branch, not 0 via the EXIT
                            // handler. Historically (6-C) this meant
                            // `should_skip_sigsys_setregs` (which
                            // required `compute_exit_return_value(
                            // ...).is_some()`) did NOT skip the
                            // SIGSYS handler's setregs for pause →
                            // the setregs fired to write -ENOSYS.
                            // (Pre-6-C the skip fired unconditionally
                            // in DESYNC mode → pause's -ENOSYS would
                            // never have been written even if this
                            // branch existed. 6-C's fix made this
                            // branch's setregs actually reachable in
                            // DESYNC mode.)
                            //
                            // 6-W UPDATE: `should_skip_sigsys_setregs`
                            // now ALWAYS returns false (never skip) —
                            // see its doc comment. The setregs fires for
                            // pause under 6-W because it ALWAYS fires
                            // (the skip is fully reverted), not because
                            // of the 6-C `compute_exit_return_value`
                            // condition. The 6-C historical reasoning
                            // above is retained for context.
                            //
                            // Task 6-G: after PAUSE_TIMEOUT_THRESHOLD
                            // (50) consecutive pause() SIGSYS calls,
                            // return -ETIMEDOUT (-110) instead of
                            // -ENOSYS. -ETIMEDOUT signals "the wait
                            // timed out" — init's wait-loop should
                            // treat this as "the property service
                            // didn't start in time" and proceed with
                            // defaults instead of looping forever. This
                            // is the FALLBACK when -ENOSYS (6-E) didn't
                            // break the loop (UI E2E on 6e51920 showed
                            // 833 pauses over 90s — reduced from 659k
                            // by 6-F's sleep, but NOT broken).
                            //
                            // The 6-F 100ms sleep (below this block)
                            // still fires for both the -ENOSYS and
                            // -ETIMEDOUT paths, so a tight retry loop
                            // stays rate-limited even if -ETIMEDOUT
                            // doesn't actually break the loop (init
                            // may treat -ETIMEDOUT as retryable like
                            // -EINTR — needs UI E2E + VLM to verify).
                            let pause_ret = pause_ret_after(pause_count);
                            sigsys_log(&format!(
                                "intercepted SIGSYS — pause() nr={} [{}] (NOT rewriting orig_rax — pause_count={} → returning {} ({}))",
                                original_syscall,
                                name,
                                pause_count,
                                pause_ret,
                                if pause_ret == -(libc::ETIMEDOUT as i64) {
                                    "TIMEOUT after 50 retries — make init give up waiting for the missing property service"
                                } else {
                                    "-ENOSYS so init falls back to a non-pause wait"
                                }
                            ));
                            pause_ret
                        } else {
                            sigsys_log(&format!(
                                "intercepted SIGSYS — syscall nr={} [{}] (NOT rewriting orig_rax, returning 0) — NOTE: unexpected SIGSYS for this syscall",
                                original_syscall, name
                            ));
                            0
                        };
                        // Force the return value. The child will see the
                        // (blocked) syscall as having returned `ret_val`.
                        //
                        // Task 6-W: ALWAYS do `ptrace_getregs →
                        // set_syscall_ret(rax=ret_val) → ptrace_setregs`.
                        // Never skip (the 5-J/6-C skip is reverted —
                        // `should_skip_sigsys_setregs` now always returns
                        // false). See the doc on
                        // `should_skip_sigsys_setregs` for the full
                        // 5-J → 6-C → 6-W evolution and root-cause
                        // analysis of the iter-826 SIGSEGV at
                        // rip=0x6f722f69 ("i/ro" rodata leak).
                        //
                        // In DESYNC mode (`in_syscall_at_sigsys == false`
                        // — SIGSYS fired AFTER the EXIT stop), the
                        // `sigsys_regs` buffer read at SIGSYS entry
                        // (line ~4667) reflects the registers AT SIGSYS
                        // time, which is AFTER the EXIT handler's setregs
                        // but BEFORE the kernel's signal-delivery-stop
                        // fully commits the signal frame. To be
                        // MAXIMALLY defensive against any kernel-side
                        // register mutation between the SIGSYS-entry
                        // getregs and our setregs (e.g. the kernel
                        // finalising signal-frame setup), we do a FRESH
                        // `ptrace_getregs` here in DESYNC mode and
                        // re-apply `set_syscall_ret`. The fresh getregs
                        // reads the CURRENT register state (the child is
                        // stopped — registers are stable while we hold
                        // the ptrace stop), so we are NOT writing back
                        // stale pre-EXIT values (which was the race 5-J
                        // originally worried about and tried to avoid by
                        // skipping setregs entirely). The subsequent
                        // `ptrace_setregs` writes rax=ret_val to the
                        // signal frame so sigreturn restores it
                        // correctly, AND re-writes the OTHER registers
                        // with their current values (preventing the
                        // rodata-leak SIGSEGV that the skip caused).
                        //
                        // In NORMAL mode (`in_syscall_at_sigsys == true`
                        // — SIGSYS fired BETWEEN ENTRY and EXIT, the
                        // typical kernel ordering for non-compat
                        // children), the SIGSYS-entry getregs at line
                        // ~4667 already read the current state (SIGSYS
                        // just fired — there is no earlier stop whose
                        // register writeback could race). No fresh
                        // getregs is needed; the existing buffer +
                        // set_syscall_ret + setregs is correct.
                        //
                        // `set_syscall_ret` is applied to `sigsys_regs`
                        // up-front (so the readback log below can report
                        // what we wrote). In DESYNC mode the fresh
                        // getregs below overwrites this — we re-apply
                        // `set_syscall_ret` after the fresh getregs.
                        set_syscall_ret(&mut sigsys_regs, &a, ret_val);
                        // `len` is the iovec length returned by the
                        // SIGSYS-entry getregs (line ~4667). In DESYNC
                        // mode the fresh getregs below may return a
                        // (theoretically) different length; we shadow it
                        // into `setregs_len` so the setregs call uses
                        // the matching length.
                        let mut setregs_len = len;
                        if !in_syscall_at_sigsys {
                            // DESYNC mode (6-W): re-read the CURRENT
                            // registers before setregs, so we write back
                            // live (post-signal-delivery) values with
                            // rax=ret_val — NOT stale pre-EXIT values and
                            // NOT the kernel's potentially-garbage
                            // signal-frame setup that the 5-J skip left
                            // in place.
                            sigsys_log(&format!(
                                "SIGSYS handler: DESYNC mode — fresh ptrace_getregs before setregs for nr={} [{}] (6-W fix: was skipping setregs, which left garbage rodata pointers in control-flow registers → SIGSEGV at rip=0x6f722f69 'i/ro')",
                                original_syscall, name
                            ));
                            match ptrace_getregs(pid, &mut sigsys_regs) {
                                Ok(fresh_len) => {
                                    setregs_len = fresh_len;
                                    // Re-apply rax=ret_val to the fresh
                                    // buffer (the fresh getregs overwrote
                                    // the set_syscall_ret we did above).
                                    set_syscall_ret(&mut sigsys_regs, &a, ret_val);
                                }
                                Err(e) => {
                                    // The fresh getregs failed — fall
                                    // through to setregs with the
                                    // SIGSYS-entry buffer (which already
                                    // has rax=ret_val from the
                                    // set_syscall_ret above). This is the
                                    // SAME buffer the NORMAL-mode path
                                    // uses, so it's a safe fallback.
                                    sigsys_log(&format!(
                                        "SIGSYS handler (DESYNC): FRESH ptrace_getregs FAILED for nr={} [{}]: {} — falling back to SIGSYS-entry registers (rax={} already applied)",
                                        original_syscall, name, e, ret_val
                                    ));
                                }
                            }
                        }
                        if let Err(e) = ptrace_setregs(pid, &sigsys_regs, setregs_len) {
                            // PTRACE_SETREGS failed — the faked return
                            // value was NOT applied. The child will see
                            // whatever rax the kernel set (typically
                            // -ENOSYS for seccomp-aborted syscalls). This
                            // is a fatal condition for TWRP boot because
                            // every faked syscall will appear to fail.
                            log(&format!(
                                "SIGSYS handler: ptrace_setregs FAILED for nr={} [{}]: {} — child will see kernel's -ENOSYS, not our faked 0",
                                original_syscall, name, e
                            ));
                        } else {
                            // setregs succeeded — the faked return value
                            // was applied (and, in DESYNC mode, the OTHER
                            // registers were re-written with their
                            // current values, preventing the rodata-leak
                            // SIGSEGV). Log a readback to confirm (5-J
                            // diagnostic, gated by sigsys_repeat_count
                            // <= 5 to avoid log flooding in tight SIGSYS
                            // loops).
                            if sigsys_repeat_count <= 5 {
                                let mut readback: Regs = unsafe { std::mem::zeroed() };
                                if ptrace_getregs(pid, &mut readback).is_ok() {
                                    let readback_rax = get_syscall_arg(&readback, a.reg_ret) as i64;
                                    sigsys_log(&format!(
                                        "[KR64][ptrace] SIGSYS handler wrote rax={} for nr={} [{}], readback rax={}",
                                        ret_val, original_syscall, name, readback_rax
                                    ));
                                }
                            }
                        }
                        // DIAGNOSTIC (Task 6-F): sleep 100ms after a
                        // pause() SIGSYS to reduce the 659k/sec CPU
                        // spin. This does NOT fix the loop — init will
                        // still pause() ~900 times over a 90s test
                        // window (vs 659k times). The deeper root cause
                        // is the missing property service (see worklog
                        // DISPATCHER-UPDATE-7: neither -EINTR nor
                        // -ENOSYS broke the loop — init calls pause()
                        // regardless of the return value). Remove this
                        // sleep when a real property service is
                        // implemented.
                        if original_syscall == a.pause {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }
                    Err(_) => {
                        // ptrace_getregs failed — we couldn't read the
                        // registers, so we can't log the original syscall
                        // number. Fall back to the old generic message so
                        // the count of SIGSYS events is still visible.
                        log("intercepted SIGSYS (seccomp-blocked syscall) — ptrace_getregs failed; skipping and returning 0");
                    }
                }

                // ── in_syscall handling after SIGSYS ──
                //
                // PREVIOUS behaviour: ALWAYS set in_syscall = true. This
                // was intended to make the next SIGTRAP|0x80 stop be
                // treated as the syscall-EXIT of the aborted syscall.
                //
                // PROBLEM (observed in 5b76fe1 E2E run): for seccomp-
                // aborted syscalls on i386 compat, the kernel SKIPS the
                // syscall-exit-stop. The next SIGTRAP|0x80 is the ENTRY
                // of the NEXT syscall. Setting in_syscall=true causes
                // this ENTRY to be misclassified as EXIT, permanently
                // desyncing the loop. The EXIT log then shows the WRONG
                // syscall number (the next syscall's) and the WRONG
                // return value (residual rax from the previous syscall).
                //
                // NEW behaviour: do NOT modify in_syscall. Leave it
                // whatever it was:
                //   - Normal case (SIGSYS fired between ENTRY and EXIT):
                //     in_syscall was true. The kernel may or may not
                //     deliver the EXIT stop. If it does, we correctly
                //     treat it as EXIT. If it doesn't, the next ENTRY
                //     is correctly treated as ENTRY (because in_syscall
                //     stays true, but the NEXT stop after a skipped EXIT
                //     would be ENTRY — hmm, this is the issue).
                //
                // Actually, the cleanest fix: for seccomp-aborted
                // syscalls, the kernel does NOT deliver an EXIT stop.
                // So after SIGSYS, we should set in_syscall=FALSE so
                // the next SIGTRAP|0x80 (which is the ENTRY of the
                // next syscall) is correctly treated as ENTRY.
                //
                // RISK: on kernels that DO deliver the EXIT stop for
                // seccomp-aborted syscalls, this would cause the EXIT
                // to be misclassified as ENTRY. The EXIT intercepts
                // (fchown/fchmod/capget/ioprio_get/ioprio_set) would
                // not fire. However, on the x86_64 Android emulator,
                // those syscalls are NOT seccomp-blocked (they execute
                // and return EPERM, handled by the EXIT handler without
                // SIGSYS). So this risk is acceptable for now.
                in_syscall = false;
                // Do NOT call PTRACE_SYSCALL here — the loop top will
                // do it with resume_signal = 0 (already reset), which
                // resumes the child WITHOUT forwarding the signal.
                // Calling PTRACE_SYSCALL here would race the loop-top
                // call (child already running → ESRCH → premature exit).
                continue;
            } else if sig == libc::SIGSTOP {
                // ── Task 6-S: SIGSTOP for a freshly-attached child ──
                //
                // When PTRACE_O_TRACEFORK auto-attaches us to a new
                // child (e.g. init forks the recovery service), the
                // kernel sends the new child a SIGSTOP so it can be
                // configured by the tracer before it runs. This stop
                // arrives via `waitpid(-1)` as WIFSTOPPED with
                // WSTOPSIG == SIGSTOP.
                //
                // We must NOT forward this SIGSTOP back to the child
                // (the existing "real signal" branch below does
                // `resume_signal = sig`, which would re-stop the
                // child on the next PTRACE_SYSCALL — an infinite
                // SIGSTOP loop). Instead we CONSUME it: resume the
                // child with signal=0 so it proceeds to its first
                // syscall (which will then be intercepted as a normal
                // SIGTRAP|0x80 syscall-stop, with the same ENTRY/EXIT/
                // SIGSYS handling as init's syscalls).
                //
                // This is the entry point for tracing the recovery
                // service's syscalls: after this SIGSTOP is consumed,
                // the recovery service's open(/dev/graphics/fb0) will
                // fire a syscall-ENTRY stop, get path-translated to
                // {rootfs}/dev/graphics/fb0 (where kr64 pre-created
                // the file at the correct size), and the recovery
                // service's view of /dev will be the rootfs's /dev —
                // not the host's /dev. This closes the fundamental
                // architectural gap that made the statically-linked
                // recovery service's syscalls go directly to the host
                // kernel with NO interception.
                //
                // SIGSTOP from other sources (e.g. the user sending
                // `kill -STOP` from outside) is rare and consuming it
                // is still correct — the child resumes its previous
                // activity, which is what `kill -CONT` would have
                // done. We do NOT need to differentiate "auto-attach
                // SIGSTOP" from "user-sent SIGSTOP" because in both
                // cases the right action is "consume + resume".
                if pid != init_pid {
                    log(&format!(
                        "SIGSTOP on forked child {} — consuming (auto-attach stop) and resuming with signal=0 so its syscalls get traced",
                        pid
                    ));
                }
                // resume_signal stays 0 — the loop-top PTRACE_SYSCALL
                // will resume `current_pid` (the SIGSTOPped child)
                // without delivering a signal.
                continue;
            } else {
                // The child stopped because of a real signal that was
                // NOT a syscall-stop, NOT a debugger trap, NOT a
                // seccomp SIGSYS, and NOT a freshly-attached SIGSTOP —
                // e.g. SIGSEGV, SIGBUS, SIGFPE, or a SIGCHLD-style
                // signal delivered by the kernel. Forward it to the
                // child so its own signal handlers (or default
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

                // For SIGSEGV (signal 11), log the crash address and
                // instruction pointer via PTRACE_GETSIGINFO. This helps
                // pinpoint WHERE init crashes, which is critical for
                // diagnosing boot failures.
                if sig == 11 {
                    // PTRACE_GETSIGINFO = 0x4202
                    let mut siginfo: libc::siginfo_t = unsafe { std::mem::zeroed() };
                    let r = unsafe {
                        libc::ptrace(
                            0x4202, // PTRACE_GETSIGINFO
                            pid,
                            0,
                            &mut siginfo as *mut _ as libc::c_long,
                        )
                    };
                    if r == 0 {
                        // Read the child's registers to get the instruction
                        // pointer (RIP on x86_64, EIP on i386 compat).
                        let mut crash_regs: Regs = unsafe { std::mem::zeroed() };
                        if ptrace_getregs(pid, &mut crash_regs).is_ok() {
                            if let Some(a) = abi {
                                // On x86_64 user_regs_struct, RIP is at
                                // index 16 (byte offset 128). Each field
                                // is u64 (8 bytes), so index 16 = offset
                                // 16*8 = 128 bytes.
                                let regs_ptr = &crash_regs as *const Regs as *const u64;
                                let rip = unsafe { *regs_ptr.add(16) };
                                let rsp = get_syscall_arg(&crash_regs, a.reg_sp);
                                // siginfo fields: si_signo, si_errno, si_code
                                // For SIGSEGV: si_addr is the faulting address
                                // The siginfo_t layout is complex in Rust's
                                // libc binding, so we read the raw bytes.
                                let si_ptr = &siginfo as *const libc::siginfo_t as *const u8;
                                let si_code = unsafe { *si_ptr.add(8) as i32 };
                                // si_addr is at offset 16 (on x86_64)
                                let si_addr = unsafe { *(si_ptr.add(16) as *const u64) };
                                log(&format!(
                                    "SIGSEGV details: si_code={} (1=MAPERR unmapped, 2=ACCERR permission), si_addr={:#x}, rip={:#x}, rsp={:#x}",
                                    si_code, si_addr, rip, rsp
                                ));
                                // Task 6-Z37: dump /proc/<pid>/maps to identify
                                // which library contains the crash address.
                                let maps_path = format!("/proc/{}/maps", pid);
                                if let Ok(maps) = std::fs::read_to_string(&maps_path) {
                                    log("=== /proc/<pid>/maps (crash diagnostic) ===");
                                    for line in maps.lines() {
                                        // Log ALL entries — we need to see the full
                                        // memory map to find which library contains
                                        // the crash address rip.
                                        log(&format!("  MAPS: {}", line));
                                    }
                                    log("=== end /proc/<pid>/maps ===");
                                } else {
                                    log("SIGSEGV: could not read /proc/<pid>/maps");
                                }
                            }
                        }
                    }
                }

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
    fn translate_path_leaves_proc_data_untouched_but_translates_sys() {
        let rootfs = "/data/user/0/io.twoyi/rootfs";
        // /proc + /data are still left untranslated (they hit the host's
        // real proc/data, which is correct for a ptraced unprivileged
        // child that can't mount a fresh proc). /proc/cmdline is handled
        // by the special-case above (translates to {rootfs}/twrp-cmdline).
        for p in &["/proc/self/status", "/data/data"] {
            assert_eq!(
                translate_path(rootfs, p),
                *p,
                "path {} should not be translated",
                p
            );
        }
        // /dev/* IS now translated to rootfs/dev/* (the host /dev is
        // read-only for untrusted_app, so we redirect to the writable
        // rootfs copy where we pre-create device stubs and symlinks).
        assert_eq!(
            translate_path(rootfs, "/dev/null"),
            format!("{}/dev/null", rootfs)
        );
        assert_eq!(
            translate_path(rootfs, "/dev/.booting"),
            format!("{}/dev/.booting", rootfs)
        );
        // /dev/__properties__ and its children now ALSO translate to
        // rootfs — the VFS materialises a valid __system_property_area__
        // at {rootfs}/dev/__properties__/properties_serial so init's
        // find_property() iterates over 0 properties and returns NULL
        // naturally (replacing the old find_property binary patch).
        assert_eq!(
            translate_path(rootfs, "/dev/__properties__"),
            format!("{}/dev/__properties__", rootfs)
        );
        assert_eq!(
            translate_path(rootfs, "/dev/__properties__/properties_serial"),
            format!("{}/dev/__properties__/properties_serial", rootfs)
        );
        // /sys/* is NOW translated to rootfs/sys/* (Task 6-P). Previously
        // /sys was left untranslated → guest's open("/sys/class") hit the
        // host's REAL kernel sysfs → EACCES → init exit(1) at iter ~3059.
        // The companion lib.rs::precreate_sysfs_stubs pre-creates the
        // expected dirs/files so the translated opens succeed.
        assert_eq!(
            translate_path(rootfs, "/sys/class"),
            format!("{}/sys/class", rootfs)
        );
        assert_eq!(
            translate_path(rootfs, "/sys/fs/selinux/enforce"),
            format!("{}/sys/fs/selinux/enforce", rootfs)
        );
        assert_eq!(
            translate_path(rootfs, "/sys/fs/selinux/load"),
            format!("{}/sys/fs/selinux/load", rootfs)
        );
        // The bare "/sys" (no trailing slash) is also translated — init
        // occasionally stats the directory itself.
        assert_eq!(translate_path(rootfs, "/sys"), format!("{}/sys", rootfs));
        // Subpaths of /sys are also translated (e.g. /sys/class/net).
        assert_eq!(
            translate_path(rootfs, "/sys/class/net"),
            format!("{}/sys/class/net", rootfs)
        );
    }

    #[test]
    fn translate_path_leaves_relative_untouched() {
        // Relative paths are returned as-is (no rootfs prefix).
        assert_eq!(translate_path("/r", "relative/path"), "relative/path");
    }

    // ── SysV shared-memory syscall number tests ─────────────────────
    //
    // These verify that the shmget / shmat / shmctl syscall numbers
    // in each ChildAbi match the real kernel ABI (verified directly
    // against /usr/include/{x86_64-linux-gnu/asm/unistd_32.h,
    // x86_64-linux-gnu/asm/unistd_64.h, asm-generic/unistd.h} in
    // Task 6-C — see the comments on ABI_X86_32's shm fields).
    //
    // The post-e6d85e1 UI E2E blocker (Task 6-C): the i386 shm numbers
    // were copy-pasted from ABI_X86_64 (29/30/31) — but those are the
    // x86_64 numbers, NOT the i386 numbers. i386 syscall 29 is
    // `pause`, 30 is `utime`, 31 is `stty`. The real i386 shm numbers
    // are 395 (shmget) / 397 (shmat) / 396 (shmctl). With the wrong
    // numbers, init's real shmget() calls (nr=395) were never
    // intercepted by the SIGSYS handler; meanwhile `pause()` (nr=29)
    // was misidentified as shmget and had -ENOSYS returned — yielding
    // an infinite shmget-retry loop (790k+ calls/sec) because init's
    // pause() loop never made forward progress. 6-C fixes the numbers
    // AND the DESYNC-skip logic that was masking the SIGSYS handler's
    // -ENOSYS writeback.

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_shm_numbers_correct() {
        // Verified against /usr/include/x86_64-linux-gnu/asm/unistd_64.h:
        //   #define __NR_shmget 29
        //   #define __NR_shmat  30
        //   #define __NR_shmctl 31
        assert_eq!(ABI_X86_64.shmget, 29, "x86_64 shmget must be 29");
        assert_eq!(ABI_X86_64.shmat, 30, "x86_64 shmat must be 30");
        assert_eq!(ABI_X86_64.shmctl, 31, "x86_64 shmctl must be 31");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_shm_numbers_correct() {
        // Verified against /usr/include/x86_64-linux-gnu/asm/unistd_32.h:
        //   #define __NR_pause   29   ← what kr64 WRONGLY used as shmget pre-6-C
        //   #define __NR_shmget 395
        //   #define __NR_shmctl 396
        //   #define __NR_shmat  397
        // (Note the order: shmat=397, NOT 396 — easy to mis-order.)
        assert_eq!(
            ABI_X86_32.shmget, 395,
            "i386 shmget must be 395 (NOT 29 — 29 is pause)"
        );
        assert_eq!(
            ABI_X86_32.shmat, 397,
            "i386 shmat must be 397 (NOT 30 — 30 is utime)"
        );
        assert_eq!(
            ABI_X86_32.shmctl, 396,
            "i386 shmctl must be 396 (NOT 31 — 31 is stty)"
        );
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn abi_aarch64_shm_numbers_correct() {
        // Verified against /usr/include/asm-generic/unistd.h:
        //   #define __NR_shmget 194
        //   #define __NR_shmctl 195
        //   #define __NR_shmat  196
        assert_eq!(ABI_AARCH64.shmget, 194, "aarch64 shmget must be 194");
        assert_eq!(ABI_AARCH64.shmat, 196, "aarch64 shmat must be 196");
        assert_eq!(ABI_AARCH64.shmctl, 195, "aarch64 shmctl must be 195");
    }

    #[test]
    fn syscall_name_resolves_shm_calls() {
        // Verify that syscall_name() recognises shmget/shmat/shmctl
        // on the current target's ABI (so the SIGSYS handler logs a
        // human-readable name instead of "[unknown]" when init trips
        // seccomp on these syscalls).
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_64;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;

        assert_eq!(syscall_name(abi.shmget, &abi), "shmget");
        assert_eq!(syscall_name(abi.shmat, &abi), "shmat");
        assert_eq!(syscall_name(abi.shmctl, &abi), "shmctl");
    }

    // ── chmod / fchown / sibling EXIT-return-value tests ──────────
    //
    // These guard the regression that caused TWRP init to SIGSEGV at
    // rip=0x809255d (NULL+0x90 deref) immediately after parsing
    // /proc/cmdline. The 4-E E2E log (commit dbcac85) showed the kernel
    // leaves rax = the syscall NUMBER (15 for i386 chmod) at the
    // syscall-EXIT stop on i386 compat seccomp-aborted syscalls — NOT
    // -ENOSYS (-38), NOT 0. The EXIT handler must force rax = 0 for ALL
    // of the chmod/fchown siblings, not just the historical
    // fchown/fchmod/capget/ioprio_get set from commit f279552.
    // (ioprio_set was added to this set in Task 5-S.)
    //
    // Each test verifies two things:
    //   (1) `compute_exit_return_value` returns `Some(0)` for this
    //       syscall (so the EXIT handler will fake the return value).
    //   (2) `syscall_name` returns the expected name (so the diagnostic
    //       log shows "chmod" instead of "unknown").

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_chmod_returns_zero() {
        // i386 chmod = 15. THIS is the exact syscall that triggered
        // the SIGSEGV: the kernel left rax = 15 at EXIT, and the
        // pre-fix EXIT handler did NOT include chmod in its faked-
        // success list, so rax stayed 15 — init saw "chmod returned
        // 15" and crashed.
        assert_eq!(compute_exit_return_value(15, &ABI_X86_32), Some(0));
        assert_eq!(syscall_name(15, &ABI_X86_32), "chmod");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_fchmod_returns_zero() {
        // i386 fchmod = 94 — pre-existing in the EXIT handler's list
        // (commit f279552), but now goes through compute_exit_return_value.
        assert_eq!(compute_exit_return_value(94, &ABI_X86_32), Some(0));
        assert_eq!(syscall_name(94, &ABI_X86_32), "fchmod");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_fchown_returns_zero() {
        // i386 fchown = 95 — same as fchmod, pre-existing.
        assert_eq!(compute_exit_return_value(95, &ABI_X86_32), Some(0));
        assert_eq!(syscall_name(95, &ABI_X86_32), "fchown");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_lchown_returns_zero() {
        // i386 lchown = 16 (NOT 94 — the task spec erroneously listed
        // 96 for i386; verified against asm-i386/unistd_32.h: lchown
        // is 16, NOT 94 or 96). Pre-fix this was MISSING from the
        // EXIT handler — added by Task 5-A.
        assert_eq!(compute_exit_return_value(16, &ABI_X86_32), Some(0));
        assert_eq!(syscall_name(16, &ABI_X86_32), "lchown");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_chown_returns_zero() {
        // i386 chown = 182 (same as x86_64). Pre-fix MISSING from the
        // EXIT handler — added by Task 5-A.
        assert_eq!(compute_exit_return_value(182, &ABI_X86_32), Some(0));
        assert_eq!(syscall_name(182, &ABI_X86_32), "chown");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_fchmodat_returns_zero() {
        // i386 fchmodat = 306. Pre-fix MISSING from the EXIT handler —
        // added by Task 5-A.
        assert_eq!(compute_exit_return_value(306, &ABI_X86_32), Some(0));
        assert_eq!(syscall_name(306, &ABI_X86_32), "fchmodat");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_fchownat_returns_zero() {
        // i386 fchownat = 298. Pre-fix MISSING from the EXIT handler —
        // added by Task 5-A.
        assert_eq!(compute_exit_return_value(298, &ABI_X86_32), Some(0));
        assert_eq!(syscall_name(298, &ABI_X86_32), "fchownat");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_capget_returns_zero() {
        // i386 capget = 184 — pre-existing.
        assert_eq!(compute_exit_return_value(184, &ABI_X86_32), Some(0));
        assert_eq!(syscall_name(184, &ABI_X86_32), "capget");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_ioprio_get_returns_zero() {
        // i386 ioprio_get = 290 — pre-existing (commit f279552), verified
        // against /usr/lib/linux/uapi/x86/asm/unistd_32.h in Task 5-S.
        // IMPORTANT: the dispatcher's task spec for 5-S claimed i386
        // ioprio_get should be 251 — that was WRONG (251 is UNUSED in
        // the i386 syscall table; the table jumps from fadvise64=250
        // to exit_group=252). 290 IS ioprio_get (NOT epoll_create1 —
        // epoll_create1 is 329 on i386). This test locks in the correct
        // value so a future "fix" based on the dispatcher's incorrect
        // numbers can't regress it silently.
        assert_eq!(compute_exit_return_value(290, &ABI_X86_32), Some(0));
        assert_eq!(syscall_name(290, &ABI_X86_32), "ioprio_get");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_ioprio_set_returns_zero() {
        // i386 ioprio_set = 289 — verified against
        // /usr/lib/linux/uapi/x86/asm/unistd_32.h in Task 5-S.
        // IMPORTANT: the dispatcher's task spec for 5-S claimed i386
        // ioprio_set = 252 — that was WRONG. 252 is `exit_group` on
        // i386 (NOT ioprio_set). Faking success on exit_group would
        // (a) mislabel every exit_group call as "ioprio_set" in the
        // syscall_name() diagnostic, AND (b) enter the fake-success
        // branch (Some(0)) which is meaningless for a non-returning
        // syscall but pollutes the EXIT handler's match set. The
        // correct i386 ioprio_set number is 289 (immediately below
        // ioprio_get=290, per the kernel's UAPI header).
        assert_eq!(compute_exit_return_value(289, &ABI_X86_32), Some(0));
        assert_eq!(syscall_name(289, &ABI_X86_32), "ioprio_set");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_exit_group_not_faked() {
        // REGRESSION GUARD (Task 5-S): 252 is `exit_group` on i386
        // (verified against /usr/lib/linux/uapi/x86/asm/unistd_32.h).
        // The dispatcher's task spec for 5-S mistakenly claimed 252
        // was ioprio_set; if we had followed that prescription, this
        // test would have been `assert_eq!(Some(0))` (wrong) instead
        // of `assert_eq!(None)` (correct). exit_group must NEVER be
        // faked-success — it doesn't return to userspace, so a
        // forced rax=0 is meaningless, AND the syscall_name() must
        // NOT mislabel it "ioprio_set" (that would hide the fact
        // that init is calling exit_group to die, which is the
        // diagnostic signal the next agent needs to find the REAL
        // cause of init's exit(1)).
        assert_eq!(
            compute_exit_return_value(252, &ABI_X86_32),
            None,
            "i386 exit_group (252) must NOT be faked-success"
        );
        assert_ne!(
            syscall_name(252, &ABI_X86_32),
            "ioprio_set",
            "i386 syscall 252 is exit_group, NOT ioprio_set"
        );
        // And the converse: 289 must match ioprio_set and be faked.
        assert_eq!(ABI_X86_32.ioprio_set, 289, "i386 ioprio_set must be 289");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_ioprio_numbers_correct() {
        // Regression guard (Task 5-S): the i386 ioprio_set/ioprio_get
        // numbers MUST match the kernel's UAPI header
        // /usr/lib/linux/uapi/x86/asm/unistd_32.h:
        //   __NR_ioprio_set 289
        //   __NR_ioprio_get 290
        // If anyone "fixes" these to the dispatcher's incorrect
        // numbers (251 / 252), the fake-success path silently stops
        // matching the real syscalls AND 252 would collide with
        // exit_group, breaking the EXIT handler's exit_group path
        // (which must remain None).
        assert_eq!(ABI_X86_32.ioprio_get, 290, "i386 ioprio_get must be 290");
        assert_eq!(ABI_X86_32.ioprio_set, 289, "i386 ioprio_set must be 289");
    }

    // ── Task 5-T regression guards: mount + rt_sigprocmask numbers ──
    //
    // These guard the i386 rt_sigprocmask number correction (14 → 175)
    // and the aarch64 mount number correction (165 → 40) — both verified
    // against the kernel's UAPI headers in Task 5-T. The dispatcher's task
    // spec for 5-T correctly identified the i386 rt_sigprocmask bug (14 is
    // mknod, not rt_sigprocmask); the aarch64 mount bug (165 is getrusage,
    // not mount) was found independently by the 5-T agent during the
    // spec-mandated "VERIFY all syscall numbers against the local kernel
    // header" step.

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_rt_sigprocmask_number_correct() {
        // Regression guard (Task 5-T): the i386 rt_sigprocmask number
        // MUST be 175 (per /usr/include/x86_64-linux-gnu/asm/
        // unistd_32.h). Pre-5-T this was 14 — WRONG: i386 syscall 14 is
        // `mknod`, NOT `rt_sigprocmask`. The kr64 SIGSYS-handler
        // diagnostic was therefore mislabelling every mknod SIGSYS as
        // "rt_sigprocmask() nr=14" (and conversely, every real
        // rt_sigprocmask call on i386, which uses syscall 175, was
        // falling through to the "unexpected SIGSYS" else branch).
        //
        // The dispatcher's task spec for 5-T explicitly directed this
        // fix; verified directly against the kernel's UAPI header.
        assert_eq!(
            ABI_X86_32.rt_sigprocmask, 175,
            "i386 rt_sigprocmask must be 175 (14 is mknod, not rt_sigprocmask)"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_mount_number_correct() {
        // Regression guard (Task 5-T): the i386 mount number MUST be
        // 21 (per /usr/include/x86_64-linux-gnu/asm/unistd_32.h). This
        // was already correct pre-5-T; the test exists to lock it in
        // so a future "fix" based on incorrect numbers (e.g. someone
        // copy-pasting the x86_64 mount number 165) can't silently
        // regress it.
        //
        // Mount is the syscall whose non-zero return value (21) at
        // EXIT was the REAL root cause of the UI E2E TWRP init
        // exit(1) at iter 189 — see the worklog entry for 5-T.
        assert_eq!(
            ABI_X86_32.mount, 21,
            "i386 mount must be 21 (per asm/unistd_32.h)"
        );
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn abi_aarch64_mount_number_correct() {
        // Regression guard (Task 5-T): the aarch64 mount number MUST
        // be 40 (per /usr/include/asm-generic/unistd.h). Pre-5-T this
        // was 165 — WRONG: aarch64 syscall 165 is `getrusage`, NOT
        // `mount`. The 165 value was copy-pasted from ABI_X86_64 (where
        // it IS correct for x86_64) without adjusting for the asm-
        // generic table divergence. With the wrong number the SIGSYS
        // handler's `mount` branch would never match a real mount()
        // call on aarch64 (and worse, would have spurious-matched any
        // getrusage SIGSYS). This bug was found independently by the
        // 5-T agent during the spec-mandated "VERIFY all syscall
        // numbers against the local kernel header" step.
        assert_eq!(
            ABI_AARCH64.mount, 40,
            "aarch64 mount must be 40 (165 is getrusage, not mount)"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_mount_returns_zero() {
        // i386 mount = 21. THIS is the exact syscall that was returning
        // 21 (the syscall NUMBER) at EXIT four times in a row in the
        // 3b571fe UI E2E logcat — the REAL root cause of TWRP init's
        // exit(1) at iter 189 (misdiagnosed earlier as ioprio_set=252,
        // which is actually exit_group, the SYMPTOM of init deciding to
        // exit, not the cause).
        //
        // Pre-5-T, mount was NOT in compute_exit_return_value's fake-
        // success list, so in DESYNC mode (5-J's fix that makes the
        // SIGSYS handler SKIP setregs) the EXIT handler left rax = the
        // kernel's syscall-number-leak value (21). After 5-T the EXIT
        // handler writes rax=0, so init sees "mount returned 0 (success)"
        // and proceeds past the mount-sequence-failed check.
        //
        // The mount SIGSYS handler already returns 0 via the
        // mount/mkdir/chmod/chroot/unshare block — but in DESYNC mode
        // that writeback is skipped, so the EXIT handler's write is
        // the only one. This test verifies the EXIT handler will now
        // fake-success mount.
        assert_eq!(compute_exit_return_value(21, &ABI_X86_32), Some(0));
        assert_eq!(syscall_name(21, &ABI_X86_32), "mount");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_rt_sigprocmask_returns_zero() {
        // i386 rt_sigprocmask = 175 (corrected in Task 5-T from the
        // wrong value 14, which is actually mknod on i386).
        //
        // Pre-5-T, rt_sigprocmask was NOT in compute_exit_return_value's
        // fake-success list. The 3b571fe UI E2E logcat showed
        // "rt_sigprocmask() nr=14 → returns 14" — the syscall NUMBER,
        // not 0 — and init exited(1). In DESYNC mode (5-J's fix that
        // makes the SIGSYS handler SKIP setregs), the EXIT handler's
        // write is the only one — and rt_sigprocmask wasn't faked there,
        // so rax retained the kernel's syscall-number-leak value (14
        // for the mislabelled mknod, or 175 for a real rt_sigprocmask
        // call).
        //
        // After 5-T the EXIT handler writes rax=0 for whichever of
        // mount(21) or rt_sigprocmask(175) the child actually calls.
        // (NOTE: if the child was actually calling mknod — syscall 14
        // on i386 — then this fix does NOT help; mknod is not in the
        // fake-success list. See the worklog entry for 5-T for the
        // honest caveat + recommended follow-up.)
        assert_eq!(compute_exit_return_value(175, &ABI_X86_32), Some(0));
        assert_eq!(syscall_name(175, &ABI_X86_32), "rt_sigprocmask");
    }

    // ── Task 5-X regression guards: mknod numbers + fake-success ──────
    //
    // These guard the mknod fake-success addition (the immediate next
    // blocker after 5-T's mount fix, per 5-W's VLM-verified UI E2E
    // analysis). The post-5-T logcat showed
    //   "post-execve syscall #34: nr=14 [unknown]"
    //   "post-execve return  #34: unknown nr=14 -> 14"   ← NON-ZERO
    // → init exit(1) at iter 189 (UNCHANGED from 3b571fe — 5-T's mount
    // fix advanced mount but not mknod). i386 syscall 14 is `mknod`
    // (verified directly against /usr/include/x86_64-linux-gnu/asm/
    // unistd_32.h in Task 5-X). These tests lock in:
    //   - the i386 mknod number (14)
    //   - the x86_64 mknod number (133)
    //   - the aarch64 mknod sentinel (-1 — no plain mknod in asm-generic)
    //   - the compute_exit_return_value fake-success match for nr=14
    //   - the syscall_name() diagnostic label for nr=14 ("mknod", not
    //     "[unknown]" — the post-5-T misnomer 5-T's i386-rt_sigprocmask
    //     number correction surfaced, now fixed)

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_mknod_number_correct() {
        // Regression guard (Task 5-X): the i386 mknod number MUST be 14
        // (per /usr/include/x86_64-linux-gnu/asm/unistd_32.h:
        // __NR_mknod 14, verified directly against the kernel's UAPI
        // header in Task 5-X).
        //
        // This is the syscall that 5-T's i386 rt_sigprocmask number
        // correction (14 → 175) CLEARED THE WAY FOR: pre-5-T, the
        // kr64 SIGSYS handler was matching syscall 14 against the
        // (wrong) ABI_X86_32.rt_sigprocmask=14 and mislabelling every
        // mknod SIGSYS as "rt_sigprocmask() nr=14". Post-5-T the
        // diagnostic label correctly says "[unknown]" for syscall 14
        // (no field matched it). Post-5-X (this addition) it correctly
        // says "mknod".
        //
        // 5-W's VLM-verified UI E2E analysis confirmed mknod (nr=14) is
        // the immediate next blocker after 5-T's mount fix: the
        // post-5-T logcat shows "post-execve return #34: unknown nr=14
        // -> 14" (NON-ZERO, NOT faked) → init exit(1) at iter 189.
        assert_eq!(
            ABI_X86_32.mknod, 14,
            "i386 mknod must be 14 (per asm/unistd_32.h)"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_mknod_number_correct() {
        // Regression guard (Task 5-X): the x86_64 mknod number MUST be
        // 133 (per /usr/include/x86_64-linux-gnu/asm/unistd_64.h:
        // __NR_mknod 133, verified directly against the kernel's UAPI
        // header in Task 5-X).
        //
        // TWRP's init binary is i386, so this x86_64 number doesn't
        // currently fire at runtime — but the EXIT handler's if-chain
        // is ABI-aware so we lock the x86_64 number in too (cheap
        // insurance; cost is one assert_eq).
        assert_eq!(
            ABI_X86_64.mknod, 133,
            "x86_64 mknod must be 133 (per asm/unistd_64.h)"
        );
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn abi_aarch64_mknod_number_correct() {
        // Regression guard (Task 5-X): the aarch64 (asm-generic) mknod
        // number MUST be -1 (SENTINEL "not present on this ABI"). The
        // asm-generic/unistd.h table (used by aarch64) has NO plain
        // `mknod` — only `mknodat = 33` (verified directly against
        // /usr/include/asm-generic/unistd.h in Task 5-X). bionic's
        // mknod(pathname, mode, dev) libc wrapper on aarch64 issues
        // mknodat(AT_FDCWD, pathname, mode, dev) under the hood, so
        // the syscall that actually hits the kernel is mknodat (33),
        // not mknod.
        //
        // With ABI_AARCH64.mknod = -1:
        //   - syscall_name(-1, &ABI_AARCH64) falls through to "unknown"
        //     (the mknod branch never matches — no real syscall is -1).
        //   - compute_exit_return_value(-1, &ABI_AARCH64) DOES match
        //     the `|| syscall_nr == abi.mknod` clause (`-1 == -1`) and
        //     returns Some(0) — but no real caller ever passes -1, so
        //     this is harmless. (Same "harmless if no real syscall is
        //     -1" reasoning that the existing pattern for ABI_AARCH64.
        //     lchown / chown relies on, since those are also -1 on
        //     aarch64 AND in the if-chain.)
        //   - A future aarch64-specific fix would add a dedicated
        //     `mknodat: i64` field (= 33) instead of aliasing mknod to
        //     33. Aliasing would mislabel mknodat SIGSYS as "mknod" in
        //     syscall_name() (mislabeled but harmless) AND would
        //     intercept a real mknodat in compute_exit_return_value
        //     (acceptable, but conflates two different syscalls in one
        //     field — confusing for future maintainers).
        //
        // The host is x86_64 running an i386 child, so this aarch64
        // path is currently dead code at runtime — the sentinel keeps
        // the compile happy and documents the aarch64 behaviour.
        assert_eq!(
            ABI_AARCH64.mknod, -1,
            "aarch64 mknod must be -1 (no plain mknod on aarch64; use mknodat=33 instead — needs a dedicated field)"
        );
        // Converse: mknodat (33) must NOT match the mknod branch on
        // aarch64 — mknodat is a separate syscall that needs its own
        // dedicated field (which is NOT in this commit's scope).
        assert_eq!(
            compute_exit_return_value(33, &ABI_AARCH64),
            None,
            "aarch64 mknodat (syscall 33) must NOT be in the fake-success list via the mknod field"
        );
        assert_ne!(syscall_name(33, &ABI_AARCH64), "mknod");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_mknod_returns_zero() {
        // i386 mknod = 14. THIS is the exact syscall that was returning
        // 14 (the syscall NUMBER) at EXIT in the post-5-T UI E2E logcat
        // (line: "post-execve return #34: unknown nr=14 -> 14") — the
        // immediate next blocker after 5-T's mount fix. init sees
        // "mknod returned 14" (a non-zero non-EPERM value) and treats
        // it as a fatal config error → exit_group(1) at iter 189
        // (UNCHANGED from 3b571fe — 5-T's mount fix advanced mount
        // from "returns 21" to "returns 0" but did NOT add mknod to
        // the fake-success list).
        //
        // Pre-5-X, mknod was NOT in compute_exit_return_value's fake-
        // success list, so in DESYNC mode (5-J's fix that makes the
        // SIGSYS handler SKIP setregs) the EXIT handler left rax = the
        // kernel's syscall-number-leak value (14). After 5-X the EXIT
        // handler writes rax=0, so init sees "mknod returned 0
        // (success)" and proceeds past the mknod-failure check.
        //
        // 5-W's VLM-verified UI E2E analysis confirmed this is the
        // immediate next blocker — VLM analysis of all 4 screenshots
        // showed the twoyi loading screen (early/mid) and the twoyi
        // Settings screen (late/final, after the 60s timeout) — NO TWRP
        // recovery interface rendered at any point.
        //
        // Honest caveat: correct-by-inspection; needs ui-e2e-test.yml
        // run + VLM screenshot analysis to confirm TWRP actually boots.
        assert_eq!(compute_exit_return_value(14, &ABI_X86_32), Some(0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn syscall_name_i386_mknod() {
        // i386 mknod = 14. Pre-5-X the diagnostic label for syscall 14
        // was "[unknown]" (because no field matched it — the i386
        // rt_sigprocmask number was corrected to 175 in Task 5-T, so
        // syscall 14 no longer matched that branch either). The
        // post-5-T logcat's "post-execve syscall #34: nr=14 [unknown]"
        // made 5-W's VLM-verified UI E2E analysis immediate — but the
        // "[unknown]" label was still misleading for any reader who
        // didn't cross-reference against the kernel's UAPI header.
        //
        // With this entry, syscall 14 on i386 is correctly labelled
        // "mknod" in the SIGSYS diagnostic log. The next round of
        // logcat analysis will show "post-execve syscall #34: nr=14
        // [mknod]" instead of "[unknown]" — immediate clarity.
        assert_eq!(syscall_name(14, &ABI_X86_32), "mknod");
        // Converse negative-asserts: 14 must NOT match the
        // rt_sigprocmask branch on i386 (rt_sigprocmask is 175 on
        // i386, per Task 5-T's correction — pre-5-T this WAS the
        // misnomer that mislabelled mknod as rt_sigprocmask).
        assert_ne!(syscall_name(14, &ABI_X86_32), "rt_sigprocmask");
        // And 14 must NOT fall through to "unknown" (the previous
        // post-5-T behaviour).
        assert_ne!(syscall_name(14, &ABI_X86_32), "unknown");
    }

    // ── 6-R regression tests: xattr SET syscalls (setxattr / lsetxattr
    //     / fsetxattr) — recovery's SELinux-restorecon EPERM retry
    //     loop blocker. See the doc on these fields in `ChildAbi` for
    //     the full root-cause analysis. ──────────────────────────────

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_lsetxattr_returns_zero() {
        // i386 lsetxattr = 227. THIS is the exact syscall that was
        // returning -EPERM (errno 1) at EXIT in the post-6-Q UI E2E
        // logcat (artifact 9341289539, run 32181613036 on e04dab6),
        // at syscalls #123 + #135 — the immediate next blocker after
        // 6-Q's PTRACE_O_TRACEFORK machinery successfully traced the
        // recovery child. TWRP recovery calls lsetxattr(path,
        // "security.selinux", ctx, 44, 0) during its SELinux-
        // restorecon phase; as untrusted_app the kernel returns
        // -EPERM, recovery treats EPERM as retryable → infinite spin
        // → death → kr64 relaunches every 2s (20:29:56 to 20:30:10).
        // Framebuffer never renders (screenshots plateau 33361 bytes).
        //
        // Pre-6-R, lsetxattr was NOT in compute_exit_return_value's
        // fake-success list, so the EXIT handler left rax = the
        // kernel's -EPERM value. After 6-R the EXIT handler writes
        // rax=0, so recovery sees "lsetxattr returned 0 (success)"
        // and proceeds past the restorecon retry loop.
        //
        // Honest caveat: correct-by-inspection; needs ui-e2e-test.yml
        // run + VLM screenshot analysis to confirm TWRP actually boots.
        assert_eq!(compute_exit_return_value(227, &ABI_X86_32), Some(0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_setxattr_returns_zero() {
        // i386 setxattr = 226 (per /usr/include/x86_64-linux-gnu/asm/
        // unistd_32.h, verified directly in Task 6-R). Companion to
        // lsetxattr — same SELinux-restorecon code path, just the
        // path-following (symlink-deref) variant. Locked in to keep
        // the EXIT handler's if-chain ABI-aware for the full xattr
        // SET family (setxattr / lsetxattr / fsetxattr).
        assert_eq!(compute_exit_return_value(226, &ABI_X86_32), Some(0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_i386_fsetxattr_returns_zero() {
        // i386 fsetxattr = 228 (per /usr/include/x86_64-linux-gnu/asm/
        // unistd_32.h, verified directly in Task 6-R). fd-based variant
        // of setxattr — recovery uses it after open()ing a file. Locked
        // in to keep the EXIT handler's if-chain ABI-aware for the full
        // xattr SET family.
        assert_eq!(compute_exit_return_value(228, &ABI_X86_32), Some(0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_x86_64_lsetxattr_returns_zero() {
        // x86_64 lsetxattr = 189 (per /usr/include/x86_64-linux-gnu/asm/
        // unistd_64.h, verified directly in Task 6-R). The host is
        // x86_64 running an i386 child, so this x86_64 number does NOT
        // currently fire at runtime (the guest uses i386 syscall 227).
        // Locked in for ABI completeness + to keep the EXIT handler's
        // ABI-aware if-chain correct if a future x86_64 guest is ever
        // supported.
        assert_eq!(compute_exit_return_value(189, &ABI_X86_64), Some(0));
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn compute_exit_return_value_aarch64_lsetxattr_returns_zero() {
        // aarch64 lsetxattr = 189 (upstream Linux asm-generic, matching
        // x86_64 — verified in Task 6-R against the kernel's UAPI
        // semantics). NOTE: this sandbox's /usr/include/asm-generic/
        // unistd.h NON-STANDARDLY lists lsetxattr=6, which is
        // io_destroy in upstream Linux — the sandbox header is wrong.
        // Real Android aarch64 bionic uses 189. Locked in to keep the
        // EXIT handler's ABI-aware if-chain correct for aarch64.
        assert_eq!(compute_exit_return_value(189, &ABI_AARCH64), Some(0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn syscall_name_i386_lsetxattr() {
        // i386 lsetxattr = 227. Pre-6-R the diagnostic label for
        // syscall 227 was "[unknown]" because no field matched it.
        // The post-6-Q logcat showed "nr=227 -> -1 (-errno 1 = EPERM)"
        // at syscalls #123 + #135 — the recovery retry loop blocker.
        // With this entry the diagnostic label correctly says
        // "lsetxattr" so the next person debugging the SELinux-
        // restorecon phase can immediately identify it from the
        // SIGSYS log without cross-referencing against
        // /usr/include/x86_64-linux-gnu/asm/unistd_32.h.
        assert_eq!(syscall_name(227, &ABI_X86_32), "lsetxattr");
        // Converse negative-asserts: 227 must NOT fall through to
        // "unknown" (the previous post-6-Q behaviour).
        assert_ne!(syscall_name(227, &ABI_X86_32), "unknown");
    }

    #[test]
    fn compute_exit_return_value_returns_none_for_unrelated_syscalls_6r() {
        // Task 6-R regression guard: confirm that adding setxattr /
        // lsetxattr / fsetxattr to the fake-success allowlist did
        // NOT accidentally widen the list to nearby unrelated
        // syscalls (e.g. setxattr=226 / lsetxattr=227 / fsetxattr=228
        // on i386 — adjacent numbers like 224 (getxattr) or 225
        // lgetxattr must NOT be faked, AND read nr=3 must continue
        // returning None). Mirrors the existing
        // `compute_exit_return_value_returns_none_for_unrelated_syscalls`
        // test but with explicit 6-R-focused syscall numbers.
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;

        // read (i386 nr=3) — must NOT be faked (mirrors the existing
        // 5-A-era unrelated-syscall assertion).
        assert_eq!(
            compute_exit_return_value(3, &abi),
            None,
            "read must NOT be faked (6-R regression guard)"
        );

        // Adjacent-to-xattr numbers on i386 that must NOT be faked:
        //   i386 nr=224 = getxattr (GET, not SET — never faked)
        //   i386 nr=225 = lgetxattr (GET, not SET — never faked)
        #[cfg(target_arch = "x86_64")]
        {
            assert_eq!(
                compute_exit_return_value(224, &abi),
                None,
                "getxattr (nr=224) must NOT be faked — only the SET family"
            );
            assert_eq!(
                compute_exit_return_value(225, &abi),
                None,
                "lgetxattr (nr=225) must NOT be faked — only the SET family"
            );
            // Also confirm setxattr (226) / lsetxattr (227) /
            // fsetxattr (228) ARE faked (sanity-check the allowlist
            // is exactly the SET family).
            assert_eq!(compute_exit_return_value(226, &abi), Some(0));
            assert_eq!(compute_exit_return_value(227, &abi), Some(0));
            assert_eq!(compute_exit_return_value(228, &abi), Some(0));
        }
    }

    // ── Task 6-Z9 tests: xattr-SET ENTRY rewrite → getpid ────────────

    #[test]
    fn compute_exit_return_value_returns_none_for_getpid_6z9() {
        // Task 6-Z9 regression guard: at the EXIT stop, after the
        // ENTRY-side rewrite changed orig_eax from the xattr number
        // (e.g. 227 for i386 lsetxattr) to getpid (e.g. 20 for i386),
        // `syscall_num` at the EXIT stop is `getpid`. The legacy
        // `compute_exit_return_value` block must return None for
        // getpid so it does NOT redundantly try to fake the return
        // (the `pending_xattr_fake` branch already faked it). This
        // test confirms that contract holds for ALL ABIs.
        #[cfg(target_arch = "x86_64")]
        {
            assert_eq!(
                compute_exit_return_value(ABI_X86_32.getpid, &ABI_X86_32),
                None,
                "i386 getpid (nr=20) must NOT be in the fake-success list — the 6-Z9 pending_xattr_fake branch handles the EXIT fake"
            );
            assert_eq!(
                compute_exit_return_value(ABI_X86_64.getpid, &ABI_X86_64),
                None,
                "x86_64 getpid (nr=39) must NOT be in the fake-success list"
            );
        }
        #[cfg(target_arch = "aarch64")]
        {
            assert_eq!(
                compute_exit_return_value(ABI_AARCH64.getpid, &ABI_AARCH64),
                None,
                "aarch64 getpid (nr=172) must NOT be in the fake-success list"
            );
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn set_syscall_num_writes_orig_rax_slot_x86_64() {
        // Task 6-Z9 regression guard: `set_syscall_num` writes the
        // syscall-number register slot (orig_rax on x86_64, index 15).
        // The 6-Z9 ENTRY-side xattr-rewrite relies on this to change
        // the child's requested syscall from lsetxattr (i386 nr=227) to
        // getpid (i386 nr=20) BEFORE the kernel executes the syscall.
        // A regression here (e.g. writing the wrong slot) would make
        // the kernel execute the ORIGINAL xattr syscall (returning
        // EPERM/EACCES/EOPNOTSUPP) instead of getpid.
        let mut regs: Regs = unsafe { std::mem::zeroed() };
        // abi.reg_syscall for ABI_X86_32 is 15 (orig_rax), same slot
        // as ABI_X86_64 — verified against the const ABI definitions.
        set_syscall_num(&mut regs, &ABI_X86_32, ABI_X86_32.getpid);
        let regs_ptr = &regs as *const Regs as *const u64;
        let orig_rax = unsafe { *regs_ptr.add(ABI_X86_32.reg_syscall) };
        assert_eq!(
            orig_rax, ABI_X86_32.getpid as u64,
            "set_syscall_num must write getpid to the orig_rax slot (index 15) for ABI_X86_32"
        );
        // Other slots should remain 0 (we only wrote the syscall slot).
        let rax = unsafe { *regs_ptr.add(ABI_X86_32.reg_ret) };
        assert_eq!(
            rax, 0,
            "set_syscall_num must NOT touch the return-value slot (rax, index 10)"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn set_syscall_ret_writes_rax_slot_x86_64() {
        // Task 6-Z9 regression guard: `set_syscall_ret` writes the
        // return-value register slot (rax on x86_64, index 10). The
        // 6-Z9 EXIT-side pending_xattr_fake branch relies on this to
        // set rax=0 (fake success) after the kernel executed getpid.
        // A regression here would leave rax holding getpid's return
        // (the child's PID, a positive integer) instead of 0, and the
        // recovery would see a non-zero return for lsetxattr → spin.
        let mut regs: Regs = unsafe { std::mem::zeroed() };
        set_syscall_ret(&mut regs, &ABI_X86_32, 0);
        let regs_ptr = &regs as *const Regs as *const u64;
        let rax = unsafe { *regs_ptr.add(ABI_X86_32.reg_ret) };
        assert_eq!(
            rax, 0,
            "set_syscall_ret(0) must write 0 to the rax slot (index 10) for ABI_X86_32"
        );
        // The syscall-number slot (orig_rax) must be untouched.
        let orig_rax = unsafe { *regs_ptr.add(ABI_X86_32.reg_syscall) };
        assert_eq!(
            orig_rax, 0,
            "set_syscall_ret must NOT touch the syscall-number slot (orig_rax, index 15)"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_xattr_numbers_correct() {
        // Regression guard (Task 6-R): the i386 setxattr/lsetxattr/
        // fsetxattr syscall numbers MUST be 226/227/228 (per
        // /usr/include/x86_64-linux-gnu/asm/unistd_32.h, verified
        // directly). If anyone "fixes" these to different numbers
        // (e.g. by accidentally copying the sandbox's non-standard
        // asm-generic/unistd.h values 5/6/7), the EXIT handler would
        // silently stop matching and the recovery retry loop would
        // come back.
        assert_eq!(ABI_X86_32.setxattr, 226, "i386 setxattr must be 226");
        assert_eq!(ABI_X86_32.lsetxattr, 227, "i386 lsetxattr must be 227");
        assert_eq!(ABI_X86_32.fsetxattr, 228, "i386 fsetxattr must be 228");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_xattr_numbers_correct() {
        // Regression guard (Task 6-R): the x86_64 setxattr/lsetxattr/
        // fsetxattr syscall numbers MUST be 188/189/190 (per
        // /usr/include/x86_64-linux-gnu/asm/unistd_64.h, verified
        // directly).
        assert_eq!(ABI_X86_64.setxattr, 188, "x86_64 setxattr must be 188");
        assert_eq!(ABI_X86_64.lsetxattr, 189, "x86_64 lsetxattr must be 189");
        assert_eq!(ABI_X86_64.fsetxattr, 190, "x86_64 fsetxattr must be 190");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn abi_aarch64_xattr_numbers_correct() {
        // Regression guard (Task 6-R): the aarch64 setxattr/lsetxattr/
        // fsetxattr syscall numbers MUST be 188/189/190 (upstream
        // Linux asm-generic, matching x86_64). NOTE: the sandbox's
        // /usr/include/asm-generic/unistd.h NON-STANDARDLY lists
        // these as 5/6/7 (which are io_setup/io_destroy/
        // io_getevents in upstream Linux — wrong). Real Android
        // aarch64 bionic uses 188/189/190.
        assert_eq!(ABI_AARCH64.setxattr, 188, "aarch64 setxattr must be 188");
        assert_eq!(ABI_AARCH64.lsetxattr, 189, "aarch64 lsetxattr must be 189");
        assert_eq!(ABI_AARCH64.fsetxattr, 190, "aarch64 fsetxattr must be 190");
    }

    // ── Task 6-S regression guards: fork/clone/vfork/wait4/exit_group ──
    //
    // These tests lock in the architectural contract added by Task 6-S:
    // that the ChildAbi struct carries clone_nr / fork_nr / vfork_nr /
    // wait4_nr / exit_group_nr fields, used by the dedicated always-log
    // ENTRY/EXIT diagnostic block in run_ptrace_loop (NOT gated by the
    // 5000 post-execve cap) so we never miss fork/clone/vfork/wait4/
    // exit_group calls — critical for diagnosing the post-6-R recovery's
    // exit(1) at iter 3281 (the bceac63 diagnostic showed ZERO such
    // calls in the entire visible logcat, but the post-execve cap of 150
    // hid the middle phase iters 151-3271 where they would have lived).
    //
    // Verified directly against the kernel's UAPI headers in Task 6-S:
    //   i386 (per /usr/include/x86_64-linux-gnu/asm/unistd_32.h):
    //     __NR_fork 2, __NR_clone 120, __NR_vfork 190,
    //     __NR_wait4 114, __NR_exit_group 252.
    //   x86_64 (per /usr/include/x86_64-linux-gnu/asm/unistd_64.h):
    //     __NR_fork 57, __NR_clone 56, __NR_vfork 58,
    //     __NR_wait4 61, __NR_exit_group 231.
    //   aarch64 (per /usr/include/asm-generic/unistd.h):
    //     __NR_clone 220, __NR_wait4 260, __NR_exit_group 94.
    //     (asm-generic DROPPED plain fork + vfork — set to -1.)

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_task_6s_fork_numbers_correct() {
        // Regression guard (Task 6-S): the i386 fork/clone/vfork/wait4/
        // exit_group syscall numbers MUST be 2/120/190/114/252 (per
        // /usr/include/x86_64-linux-gnu/asm/unistd_32.h, verified
        // directly). THIS is the value set that fires at runtime —
        // TWRP init + recovery are i386 binaries. If anyone "fixes"
        // these to different numbers (e.g. by copying the x86_64
        // values 57/56/58/61/231 — which would be WRONG for an i386
        // child), the always-log ENTRY/EXIT block would silently
        // stop matching fork/clone/vfork/wait4/exit_group calls + the
        // post-6-R recovery exit(1) diagnostic would go dark.
        assert_eq!(ABI_X86_32.clone_nr, 120, "i386 clone_nr must be 120");
        assert_eq!(ABI_X86_32.fork_nr, 2, "i386 fork_nr must be 2");
        assert_eq!(ABI_X86_32.vfork_nr, 190, "i386 vfork_nr must be 190");
        assert_eq!(ABI_X86_32.wait4_nr, 114, "i386 wait4_nr must be 114");
        assert_eq!(
            ABI_X86_32.exit_group_nr, 252,
            "i386 exit_group_nr must be 252"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_task_6s_fork_numbers_correct() {
        // Regression guard (Task 6-S): the x86_64 fork/clone/vfork/
        // wait4/exit_group syscall numbers MUST be 57/56/58/61/231
        // (per /usr/include/x86_64-linux-gnu/asm/unistd_64.h, verified
        // directly). The host is x86_64 running an i386 child, so
        // these x86_64 numbers do NOT currently fire at runtime —
        // locked in for ABI completeness + so the always-log block
        // compiles + works correctly if a future x86_64 guest is
        // ever supported (mirrors the mknod: 133 / setxattr: 188 /
        // pause: 34 precedent).
        assert_eq!(ABI_X86_64.clone_nr, 56, "x86_64 clone_nr must be 56");
        assert_eq!(ABI_X86_64.fork_nr, 57, "x86_64 fork_nr must be 57");
        assert_eq!(ABI_X86_64.vfork_nr, 58, "x86_64 vfork_nr must be 58");
        assert_eq!(ABI_X86_64.wait4_nr, 61, "x86_64 wait4_nr must be 61");
        assert_eq!(
            ABI_X86_64.exit_group_nr, 231,
            "x86_64 exit_group_nr must be 231"
        );
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn abi_aarch64_task_6s_fork_numbers_correct() {
        // Regression guard (Task 6-S): the aarch64 (asm-generic)
        // clone/wait4/exit_group numbers MUST be 220/260/94. NOTE:
        // __NR_exit_group is 94 (NOT 93 — 93 is plain `exit` which
        // exits just the calling thread; exit_group exits all threads
        // in the process and is what bionic's exit_group() / _Exit()
        // wrappers call). The task spec for 6-S said aarch64
        // exit_group=93 — that was WRONG; verified: 94. fork + vfork
        // were DROPPED in asm-generic (bionic's fork() libc wrapper
        // on aarch64 issues clone() under the hood) — set to -1
        // (sentinels "not present on this ABI").
        assert_eq!(ABI_AARCH64.clone_nr, 220, "aarch64 clone_nr must be 220");
        assert_eq!(
            ABI_AARCH64.fork_nr, -1,
            "aarch64 fork_nr must be -1 (dropped in asm-generic)"
        );
        assert_eq!(
            ABI_AARCH64.vfork_nr, -1,
            "aarch64 vfork_nr must be -1 (dropped in asm-generic)"
        );
        assert_eq!(ABI_AARCH64.wait4_nr, 260, "aarch64 wait4_nr must be 260");
        assert_eq!(
            ABI_AARCH64.exit_group_nr, 94,
            "aarch64 exit_group_nr must be 94 (NOT 93 — 93 is plain exit)"
        );
    }

    // ── Task 6-T regression guards: stat64/lstat64/fstat64 ──
    //
    // These tests lock in the architectural contract added by Task 6-T:
    // that the ChildAbi struct carries stat64 / lstat64 / fstat64
    // fields, used by BOTH the path-translation match arm (stat64 +
    // lstat64 — fstat64 takes an fd, no path translation) AND the
    // syscall_name diagnostic label (all three). Pre-6-T the recovery's
    // stat64("/some/rootfs/path") checked the HOST filesystem (where
    // rootfs files don't exist) → ENOENT → infinite polling loop
    // (clock_gettime → stat64 → ENOENT → nanosleep → repeat ~3500×) →
    // recovery gives up → wait4 → exit_group(1). Observed on 3a77faf
    // UI E2E run 32191877530 (5000-cap logging from Task 6-S revealed
    // the full polling loop: post-execve syscalls #294+ all nr=195 →
    // -2 ENOENT, repeating with nr=265 clock_gettime + nr=162 nanosleep).
    //
    // Verified directly against the kernel's UAPI headers in Task 6-T:
    //   i386 (per /usr/include/x86_64-linux-gnu/asm/unistd_32.h):
    //     __NR_stat64 195, __NR_lstat64 196, __NR_fstat64 197
    //   x86_64 (per /usr/include/x86_64-linux-gnu/asm/unistd_64.h):
    //     NO __NR_stat64 / __NR_lstat64 / __NR_fstat64 entries — set to -1
    //     (stat/lstat/fstat are already 64-bit on x86_64 — no need for the
    //     64-bit-struct variant).
    //   aarch64 (per /usr/include/asm-generic/unistd.h):
    //     NO __NR_stat64 / __NR_lstat64 / __NR_fstat64 — set to -1 (aarch64
    //     uses statx + newfstatat exclusively; the stat64 family was a
    //     32-bit-only workaround for the old small-field struct stat).

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_task_6t_stat64_numbers_correct() {
        // Regression guard (Task 6-T): the i386 stat64/lstat64/fstat64
        // syscall numbers MUST be 195/196/197 (per
        // /usr/include/x86_64-linux-gnu/asm/unistd_32.h, verified
        // directly). THIS is the value set that ACTUALLY fires at
        // runtime — TWRP init + recovery are i386 binaries built with
        // modern bionic, which uses stat64/lstat64 INSTEAD of the old
        // stat(106)/lstat(107). If anyone "fixes" these to different
        // numbers (e.g. by accidentally copying the x86_64 -1 values
        // — which would be WRONG for an i386 child), the path-
        // translation match arm would silently stop matching stat64/
        // lstat64 → the recovery's stat64("/some/rootfs/path") would
        // hit the HOST fs again → ENOENT polling loop would come back.
        assert_eq!(ABI_X86_32.stat64, 195, "i386 stat64 must be 195");
        assert_eq!(ABI_X86_32.lstat64, 196, "i386 lstat64 must be 196");
        assert_eq!(ABI_X86_32.fstat64, 197, "i386 fstat64 must be 197");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_task_6t_stat64_numbers_correct() {
        // Regression guard (Task 6-T): the x86_64 stat64/lstat64/fstat64
        // syscall numbers MUST be -1/-1/-1 (sentinels "not present on
        // this ABI"). x86_64's stat/lstat/fstat are ALREADY 64-bit (the
        // struct stat carries 64-bit st_size + st_ino), so the kernel
        // does NOT expose separate stat64/lstat64/fstat64 syscall
        // numbers — verified directly against
        // /usr/include/x86_64-linux-gnu/asm/unistd_64.h (which has NO
        // __NR_stat64 / __NR_lstat64 / __NR_fstat64 entries). The host
        // is x86_64 running an i386 child, so these -1 values do NOT
        // fire at runtime — locked in for ABI completeness + so the
        // path-translation match arm + syscall_name label compile +
        // behave correctly if a future x86_64 guest is ever supported
        // (mirrors the mknod: 133 / setxattr: 188 / pause: 34 /
        // fork_nr: -1 precedent on ABI_AARCH64).
        assert_eq!(
            ABI_X86_64.stat64, -1,
            "x86_64 stat64 must be -1 (no variant)"
        );
        assert_eq!(
            ABI_X86_64.lstat64, -1,
            "x86_64 lstat64 must be -1 (no variant)"
        );
        assert_eq!(
            ABI_X86_64.fstat64, -1,
            "x86_64 fstat64 must be -1 (no variant)"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn syscall_name_i386_stat64() {
        // Task 6-T: i386 stat64 = 195. Pre-6-T the diagnostic label
        // for syscall 195 was "[unknown]" because no field matched it.
        // The post-6-S 5000-cap logcat showed HUNDREDS of
        // "nr=195 -> -2 (ENOENT)" lines that, without this label,
        // required cross-referencing against
        // /usr/include/x86_64-linux-gnu/asm/unistd_32.h to identify
        // as stat64. THIS label makes the polling-loop root cause
        // immediately readable: "post-execve syscall #294: stat64 ->
        // -2 (ENOENT)" instead of "nr=195 [unknown] -> -2 (ENOENT)".
        assert_eq!(syscall_name(195, &ABI_X86_32), "stat64");
        // Converse negative-assert: 195 must NOT fall through to
        // "unknown" (the previous pre-6-T behaviour).
        assert_ne!(syscall_name(195, &ABI_X86_32), "unknown");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn syscall_name_i386_lstat64() {
        // Task 6-T: i386 lstat64 = 196. Companion to syscall_name_i386_
        // stat64 above — same diagnostic-label rationale.
        assert_eq!(syscall_name(196, &ABI_X86_32), "lstat64");
        assert_ne!(syscall_name(196, &ABI_X86_32), "unknown");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn syscall_name_i386_fstat64() {
        // Task 6-T: i386 fstat64 = 197. fstat64 takes an fd (NOT a path)
        // so it is intentionally NOT in the path-translation match arm
        // — but we add the syscall_name label so the diagnostic log
        // shows "fstat64" instead of "[unknown]" if/when the recovery
        // calls fstat64 on a rootfs fd (the recovery DOES fstat
        // /dev/null / /dev/urandom fds during init, which under modern
        // bionic goes through fstat64).
        assert_eq!(syscall_name(197, &ABI_X86_32), "fstat64");
        assert_ne!(syscall_name(197, &ABI_X86_32), "unknown");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_ioprio_numbers_correct() {
        // Regression guard (Task 5-S): the x86_64 ioprio numbers
        // per /usr/include/x86_64-linux-gnu/asm/unistd_64.h:
        //   __NR_ioprio_set 251
        //   __NR_ioprio_get 252
        assert_eq!(ABI_X86_64.ioprio_get, 252, "x86_64 ioprio_get must be 252");
        assert_eq!(ABI_X86_64.ioprio_set, 251, "x86_64 ioprio_set must be 251");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn abi_aarch64_ioprio_numbers_correct() {
        // Regression guard (Task 5-S): the aarch64 (asm-generic)
        // ioprio numbers per /usr/include/asm-generic/unistd.h:
        //   __NR_ioprio_set 30
        //   __NR_ioprio_get 31
        // NOTE: the dispatcher's task spec for 5-S said aarch64
        // ioprio_set=31 / ioprio_get=30 — these were SWAPPED (31 is
        // ioprio_get, not ioprio_set). Verified directly against the
        // kernel's UAPI header.
        assert_eq!(ABI_AARCH64.ioprio_get, 31, "aarch64 ioprio_get must be 31");
        assert_eq!(ABI_AARCH64.ioprio_set, 30, "aarch64 ioprio_set must be 30");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_x86_64_ioprio_set_returns_zero() {
        // x86_64 ioprio_set = 251 (verified against
        // /usr/include/x86_64-linux-gnu/asm/unistd_64.h in Task 5-S).
        // Pre-5-S MISSING from the fake-success list — added by
        // Task 5-S.
        assert_eq!(compute_exit_return_value(251, &ABI_X86_64), Some(0));
        assert_eq!(syscall_name(251, &ABI_X86_64), "ioprio_set");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn compute_exit_return_value_aarch64_ioprio_set_returns_zero() {
        // aarch64 ioprio_set = 30 (verified against
        // /usr/include/asm-generic/unistd.h in Task 5-S).
        // Pre-5-S MISSING from the fake-success list — added by
        // Task 5-S.
        assert_eq!(compute_exit_return_value(30, &ABI_AARCH64), Some(0));
        assert_eq!(syscall_name(30, &ABI_AARCH64), "ioprio_set");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_x86_64_chmod_returns_zero() {
        // x86_64 chmod = 90 — pre-existing in the SIGSYS handler but
        // now also covered by the EXIT handler via compute_exit_return_value.
        assert_eq!(compute_exit_return_value(90, &ABI_X86_64), Some(0));
        assert_eq!(syscall_name(90, &ABI_X86_64), "chmod");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_x86_64_lchown_returns_zero() {
        // x86_64 lchown = 94. Pre-fix MISSING — added by Task 5-A.
        assert_eq!(compute_exit_return_value(94, &ABI_X86_64), Some(0));
        assert_eq!(syscall_name(94, &ABI_X86_64), "lchown");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_x86_64_chown_returns_zero() {
        // x86_64 chown = 182. Pre-fix MISSING — added by Task 5-A.
        assert_eq!(compute_exit_return_value(182, &ABI_X86_64), Some(0));
        assert_eq!(syscall_name(182, &ABI_X86_64), "chown");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_x86_64_fchmodat_returns_zero() {
        // x86_64 fchmodat = 268. Pre-fix MISSING — added by Task 5-A.
        assert_eq!(compute_exit_return_value(268, &ABI_X86_64), Some(0));
        assert_eq!(syscall_name(268, &ABI_X86_64), "fchmodat");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn compute_exit_return_value_x86_64_fchownat_returns_zero() {
        // x86_64 fchownat = 260 (per asm/unistd_64.h). Pre-fix MISSING
        // from the EXIT handler — added by Task 5-A. Task 5-A's commit
        // ee93ac0 used 257 here (a 1-char typo: 257 is openat on x86_64,
        // NOT fchownat), which made compute_exit_return_value(257, X86_64)
        // incorrectly return Some(0) — i.e. every openat() got fake
        // success returning stdin fd 0 instead of a real fd. Fixed by
        // Task 5-H (257 -> 260). See `abi_x86_64_openat_257_not_faked`
        // below for the explicit openat-not-faked regression guard.
        assert_eq!(compute_exit_return_value(260, &ABI_X86_64), Some(0));
        assert_eq!(syscall_name(260, &ABI_X86_64), "fchownat");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_openat_257_not_faked() {
        // REGRESSION GUARD (Task 5-H): 257 is `openat` on x86_64, NOT
        // fchownat. Task 5-A's commit ee93ac0 had `fchownat: 257` here,
        // which caused compute_exit_return_value(257, X86_64) to return
        // Some(0), making the EXIT handler fake-success every openat()
        // call — TWRP init then got rax=0 (stdin fd) from openat()
        // instead of a real fd and crashed with exit_group(127) at
        // iteration 113 (5-F's finding). This test asserts the bug is
        // gone: 257 must NOT match any faked-success syscall on x86_64,
        // and must NOT be labelled "fchownat".
        assert_eq!(
            compute_exit_return_value(257, &ABI_X86_64),
            None,
            "x86_64 openat (257) must NOT be faked-success — fchownat is 260"
        );
        assert_ne!(
            syscall_name(257, &ABI_X86_64),
            "fchownat",
            "x86_64 syscall 257 is openat, NOT fchownat (fchownat is 260)"
        );
        // And the converse: 260 must match fchownat and be faked.
        assert_eq!(ABI_X86_64.fchownat, 260, "x86_64 fchownat must be 260");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn compute_exit_return_value_aarch64_fchmodat_returns_zero() {
        // aarch64 fchmodat = 53. asm-generic has no plain chmod/lchown/
        // chown — bionic issues fchmodat for chmod() callers.
        assert_eq!(compute_exit_return_value(53, &ABI_AARCH64), Some(0));
        assert_eq!(syscall_name(53, &ABI_AARCH64), "chmod");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn compute_exit_return_value_aarch64_fchownat_returns_zero() {
        // aarch64 fchownat = 54.
        assert_eq!(compute_exit_return_value(54, &ABI_AARCH64), Some(0));
        assert_eq!(syscall_name(54, &ABI_AARCH64), "fchownat");
    }

    #[test]
    fn compute_exit_return_value_returns_none_for_unrelated_syscalls() {
        // Syscalls NOT in the faked-success set should return None so
        // the EXIT handler leaves the kernel's return value alone.
        // (open, read, write, close, fstat — the "normal" syscalls
        // TWRP init issues between the chmod and the crash.)
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;

        // open (i386 nr=5), read (nr=3), close (nr=6) — these are the
        // syscalls init makes IMMEDIATELY after the chmod that crashed
        // (openat(/proc/cmdline) → read(322 bytes) → close → SIGSEGV).
        // None of them should be faked.
        assert_eq!(
            compute_exit_return_value(5, &abi),
            None,
            "open must NOT be faked"
        );
        assert_eq!(
            compute_exit_return_value(3, &abi),
            None,
            "read must NOT be faked"
        );
        assert_eq!(
            compute_exit_return_value(6, &abi),
            None,
            "close must NOT be faked"
        );
    }

    #[test]
    fn compute_exit_return_value_returns_none_for_syscall_number_leak_value() {
        // The bug: on i386 compat the kernel left rax = the syscall
        // NUMBER (15 for chmod) at the EXIT stop. The pre-fix EXIT
        // handler would NOT match this against its (incomplete) list
        // and would NOT force rax = 0. This test verifies that calling
        // compute_exit_return_value with the leaked syscall-number
        // value (15) DOES match (returns Some(0)) — i.e. the EXIT
        // handler will now correctly force rax = 0 even when the
        // kernel leaks the syscall number into rax.
        //
        // NOTE: this test passes "15" as both the syscall_nr AND as
        // the would-be return value — the same number, deliberately,
        // to make the regression-via-confusion impossible: if
        // someone accidentally swaps the two arguments the test
        // still catches the bug because the value 15 maps to chmod
        // (Some(0)) when interpreted as a syscall number, and to
        // "unrelated" (None) when interpreted as a different
        // syscall's return value.
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;

        // 15 = i386 chmod — the exact syscall + value from the 4-E log.
        assert_eq!(compute_exit_return_value(15, &abi), Some(0));
    }

    // ── 5-J / 6-C / 6-W regression tests: SIGSYS/EXIT handler
    //   register-writeback ─
    //
    // These tests verify `should_skip_sigsys_setregs`, the helper the
    // SIGSYS handler historically used to decide whether to skip its
    // `ptrace_setregs` call. See `should_skip_sigsys_setregs` for the
    // full 5-J → 6-C → 6-W evolution.
    //
    // The 5-J bug (9 SIGSEGVs at iter 216, all at rip=0x809255d,
    // si_addr=0x90): the SIGSYS handler's whole-struct `ptrace_setregs`
    // could race with the kernel's signal-delivery-stop register
    // snapshotting in DESYNC mode, clobbering rax=0 with rax=15 (the
    // syscall number from `syscall_rollback`).
    //
    // The 5-J fix: in DESYNC mode, SKIP the setregs (the EXIT handler
    // already wrote rax=0). The 6-C refinement: skip ONLY for syscalls
    // in `compute_exit_return_value`'s fake-success list (so shmget's
    // -ENOSYS writeback still executed).
    //
    // The 6-W REVERSAL: the 5-J/6-C skip LEFT garbage rodata pointers
    // in control-flow registers → SIGSEGV at rip=0x6f722f69 ("i/ro"
    // leak, iter-826, post-6-V's NOP). 6-W makes
    // `should_skip_sigsys_setregs` ALWAYS return `false` (never skip).
    // The SIGSYS handler now ALWAYS does a fresh `ptrace_getregs` (in
    // DESYNC mode) + `set_syscall_ret(rax=ret_val)` + `ptrace_setregs`
    // — re-reading the CURRENT register state so we're not writing
    // stale values, and re-writing ALL registers with their current
    // values so no rodata leak survives into a control-flow register.

    #[test]
    fn should_skip_sigsys_setregs_in_desync_mode() {
        // 6-W: `should_skip_sigsys_setregs` ALWAYS returns false now.
        // In DESYNC mode (SIGSYS fired AFTER the EXIT stop, so
        // `in_syscall_at_sigsys == false`) the SIGSYS handler does a
        // FRESH `ptrace_getregs` + `set_syscall_ret` + `ptrace_setregs`
        // at the call site — it does NOT skip. This test pins the new
        // 6-W contract: even for a fake-success syscall (chmod) in
        // DESYNC mode, the skip MUST NOT fire.
        //
        // (Pre-6-W this test asserted the skip MUST fire — the 5-J/6-C
        // contract. The iter-826 rodata-leak SIGSEGV at rip=0x6f722f69
        // proved that contract was wrong: skipping left garbage in
        // control-flow registers.)
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;
        let chmod_nr = abi.chmod;
        assert!(
            !should_skip_sigsys_setregs(false, chmod_nr, &abi),
            "6-W: DESYNC + chmod (fake-success) — must NOT skip setregs (always returns false). The SIGSYS handler does a fresh getregs + setregs instead, re-writing ALL registers with current values to prevent the rodata-leak SIGSEGV."
        );
    }

    #[test]
    fn should_not_skip_sigsys_setregs_in_normal_mode() {
        // NORMAL case: SIGSYS fires BETWEEN ENTRY and EXIT (the typical
        // kernel ordering for non-compat children, where SIGSYS
        // replaces the syscall-exit-stop). `in_syscall` was true at
        // SIGSYS entry. The EXIT handler has NOT yet run, so the SIGSYS
        // handler's `ptrace_setregs` is the ONLY writeback — must NOT
        // skip it, REGARDLESS of whether the syscall is in the fake-
        // success list. (6-W: this was already the contract in NORMAL
        // mode under 5-J/6-C — the 6-W change only affects DESYNC mode,
        // but the function now returns false unconditionally so NORMAL
        // mode is unchanged.)
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;
        // Test BOTH a fake-success syscall (chmod) AND a non-fake-
        // success syscall (shmget) to verify NORMAL mode never skips.
        let chmod_nr = abi.chmod;
        let shmget_nr = abi.shmget;
        assert!(
            !should_skip_sigsys_setregs(true, chmod_nr, &abi),
            "NORMAL: SIGSYS fired between ENTRY and EXIT — must NOT skip setregs for chmod (it's the only writeback)"
        );
        assert!(
            !should_skip_sigsys_setregs(true, shmget_nr, &abi),
            "NORMAL: SIGSYS fired between ENTRY and EXIT — must NOT skip setregs for shmget either (it's the only writeback, and shmget needs -ENOSYS)"
        );
    }

    // ── 6-C regression tests: should_skip_sigsys_setregs honours ─
    //   compute_exit_return_value's fake-success list
    //
    // Historical context (6-C): these tests originally guarded the 6-C
    // fix for the infinite shmget-retry loop. The OLD contract (5-J)
    // was a pure negation of `in_syscall_at_sigsys`:
    // `!in_syscall_at_sigsys`. That fired unconditionally in DESYNC
    // mode for EVERY syscall, including shmget/shmat/shmctl whose
    // return value the SIGSYS handler writes as -ENOSYS (NOT 0).
    //
    // 6-W UPDATE: `should_skip_sigsys_setregs` now ALWAYS returns
    // false (never skip). The 6-C distinction between "fake-success"
    // (chmod → skip) and "non-fake-success" (shmget → no skip) is no
    // longer relevant to this function — BOTH now return false. The
    // tests below are KEPT (with updated assertions) as regression
    // guards so a future change that re-enables the skip cannot
    // silently regress the 6-W contract. They also double as
    // documentation of WHY the 6-C distinction existed (so a future
    // investigator can re-introduce a conditional skip correctly if a
    // NEW race is discovered).

    #[test]
    fn should_skip_sigsys_setregs_false_for_chmod_in_desync_6w() {
        // 6-W: chmod IS in compute_exit_return_value's fake-success
        // list (it returns Some(0)). Pre-6-W, in DESYNC mode the 5-J/6-C
        // skip fired for chmod (because the EXIT handler already wrote
        // rax=0). 6-W REVERTS this: the skip caused the iter-826
        // rodata-leak SIGSEGV at rip=0x6f722f69 ("i/ro"). Now the
        // SIGSYS handler ALWAYS does a fresh getregs + setregs for
        // chmod in DESYNC mode, re-writing ALL registers with their
        // current values (preventing the rodata leak) and re-applying
        // rax=ret_val=0 to the signal frame.
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;
        let chmod_nr = abi.chmod;
        // Sanity: chmod IS in the fake-success list (kept from the
        // pre-6-W test — confirms the syscall that USED to trigger
        // the skip is still a fake-success syscall).
        assert_eq!(
            compute_exit_return_value(chmod_nr, &abi),
            Some(0),
            "chmod must be in compute_exit_return_value's fake-success list"
        );
        // 6-W: DESYNC + fake-success → must NOT skip (always false).
        assert!(
            !should_skip_sigsys_setregs(false, chmod_nr, &abi),
            "6-W: DESYNC + chmod (fake-success): must NOT skip — the 5-J/6-C skip caused the rodata-leak SIGSEGV. The SIGSYS handler now does fresh getregs + setregs instead."
        );
    }

    #[test]
    fn should_skip_sigsys_setregs_false_for_shmget() {
        // shmget is NOT in compute_exit_return_value's fake-success
        // list (it returns None — the SIGSYS handler returns -ENOSYS
        // for shmget separately). In DESYNC mode the EXIT handler did
        // NOT write rax for shmget → the SIGSYS handler's setregs is
        // the ONLY writeback and MUST execute to write -ENOSYS. Skip
        // MUST NOT fire. (Pre-6-C this fired unconditionally in DESYNC
        // mode → -ENOSYS was never written → rax retained the kernel's
        // leaked syscall-number value → init saw a positive "shmid" →
        // infinite shmget-retry loop.)
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;
        let shmget_nr = abi.shmget;
        // Sanity: shmget is NOT in the fake-success list.
        assert_eq!(
            compute_exit_return_value(shmget_nr, &abi),
            None,
            "shmget must NOT be in compute_exit_return_value's fake-success list (the SIGSYS handler returns -ENOSYS for it)"
        );
        // DESYNC + NOT fake-success → must NOT skip.
        assert!(
            !should_skip_sigsys_setregs(false, shmget_nr, &abi),
            "DESYNC + shmget (NOT in fake-success list): skip MUST NOT fire — SIGSYS handler's setregs is the only writeback (writes -ENOSYS)"
        );
    }

    // ── 6-D + 6-E regression tests: pause() syscall numbers + SIGSYS branch ─
    //
    // These guard the pause() fix path. Pre-6-D the SIGSYS handler
    // returned 0 for pause (the default "NOT rewriting orig_rax,
    // returning 0" branch) → init interpreted 0 as "pause completed
    // WITHOUT a signal" → re-checked its condition (property service
    // still not ready) → called pause() again → INFINITE LOOP (the
    // post-6-C UI E2E blocker, 1,048,000+ repeats on commit 368f59b).
    // The kernel's own pause() can ONLY ever return -EINTR (errno 4)
    // — there is no "successful" return.
    //
    // 6-D (commit 2b073f8) tried returning -EINTR (-4): this made
    // init think "interrupted by a signal" → check the condition
    // (property service not ready) → call pause() again → INFINITE
    // LOOP. The UI E2E test on 2b073f8 showed the pause loop was STILL
    // there (992,000+ repeats) — -EINTR did NOT break the loop. The
    // property service will NEVER signal readiness because kr64 has
    // NO property service (5-Y's find_property binary patch makes
    // lookups return NULL, but there's no actual service to send the
    // "ready" signal).
    //
    // Task 6-E: return -ENOSYS (-38) instead — tells init "this
    // kernel does not implement pause()" → init falls back to a
    // non-pause wait mechanism (or skips the wait entirely). Mirrors
    // how 6-C's shmget -ENOSYS made init fall back to non-shared-
    // memory property init (which WORKED — the shmget loop stopped).
    //
    // These tests verify:
    //   (1) pause syscall numbers match the kernel's UAPI headers
    //       (i386=29, x86_64=34, aarch64=-1 sentinel).
    //   (2) compute_exit_return_value returns None for pause (pause
    //       is NOT in the fake-success list — it has its own -ENOSYS
    //       branch in the SIGSYS handler).
    //   (3) should_skip_sigsys_setregs does NOT fire for pause (so
    //       the SIGSYS handler's setregs MUST fire to write -ENOSYS).
    //   (4) sigsys_ret_for_pause() returns -ENOSYS (NOT -EINTR, NOT 0)
    //       — the direct regression guard for the 6-E fix (locks in
    //       the contract so a future "fix" can't regress it back to
    //       -EINTR or 0).

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_pause_number_correct() {
        // Verified against /usr/include/x86_64-linux-gnu/asm/unistd_32.h:
        //   #define __NR_pause   29
        // This is the SAME number that the pre-6-C kr64 WRONGLY used
        // for ABI_X86_32.shmget (because the i386 shm numbers were
        // copy-pasted from ABI_X86_64, where shmget IS 29). 6-C moved
        // shmget to 395 (the real i386 number), which left syscall 29
        // "unintercepted" by the shmget branch and falling through to
        // the default "returning 0" branch — exposing the pause() loop
        // bug that 6-D fixes.
        assert_eq!(ABI_X86_32.pause, 29, "i386 pause must be 29");
        // Sanity: pause ≠ shmget on i386 now (both were 29 pre-6-C).
        assert_ne!(
            ABI_X86_32.pause, ABI_X86_32.shmget,
            "i386 pause ({}) must differ from i386 shmget ({}) — both were 29 pre-6-C",
            ABI_X86_32.pause, ABI_X86_32.shmget
        );
        // syscall_name() must resolve i386 syscall 29 to "pause".
        assert_eq!(syscall_name(29, &ABI_X86_32), "pause");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_pause_number_correct() {
        // Verified against /usr/include/x86_64-linux-gnu/asm/unistd_64.h:
        //   #define __NR_pause  34
        // The host is x86_64 running an i386 child, so this x86_64
        // number does NOT currently fire at runtime (the guest uses
        // i386 syscall 29 for pause). It is locked in for ABI
        // completeness and to keep the EXIT handler's ABI-aware
        // if-chain correct if a future x86_64 guest is ever supported.
        assert_eq!(ABI_X86_64.pause, 34, "x86_64 pause must be 34");
        // Sanity: syscall_name() resolves x86_64 syscall 34 to "pause".
        assert_eq!(syscall_name(34, &ABI_X86_64), "pause");
    }

    #[test]
    fn compute_exit_return_value_pause_returns_none() {
        // pause is NOT in compute_exit_return_value's fake-success
        // list — it returns None. The SIGSYS handler has its OWN
        // dedicated branch for pause that returns -ENOSYS (-38, NOT 0)
        // via sigsys_ret_for_pause() (Task 6-E — was -EINTR in 6-D
        // commit 2b073f8, but -EINTR caused an infinite loop because
        // the property service never signals readiness). If pause were
        // ever added to the fake-success list, the EXIT handler would
        // write rax=0 for it, which would CAUSE the infinite pause()
        // retry loop again (init would interpret 0 as "pause completed
        // without a signal" → re-check condition → retry pause
        // forever). This test locks in the contract: pause returns
        // None from compute_exit_return_value so the EXIT handler does
        // NOT write rax=0 for pause; the SIGSYS handler's -ENOSYS
        // writeback is the ONLY writeback.
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32; // i386 — the runtime-relevant ABI for TWRP.
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;
        let pause_nr = abi.pause;
        // Sanity: pause_nr is either a real syscall number (i386=29,
        // x86_64=34) or the -1 sentinel (aarch64). We only assert the
        // None contract for real syscall numbers — for -1 the contract
        // is also None (no real caller ever passes -1) but the test
        // value is meaningless.
        assert_eq!(
            compute_exit_return_value(pause_nr, &abi),
            None,
            "pause must NOT be in compute_exit_return_value's fake-success list — the SIGSYS handler returns -ENOSYS for it via a dedicated branch (returning 0 would cause the infinite pause() retry loop)"
        );
    }

    #[test]
    fn should_skip_sigsys_setregs_false_for_pause() {
        // pause is NOT in compute_exit_return_value's fake-success
        // list (it returns None — the SIGSYS handler returns -ENOSYS
        // for pause via a dedicated branch, NOT 0 via the EXIT
        // handler). In DESYNC mode the EXIT handler did NOT write
        // rax for pause → the SIGSYS handler's setregs is the ONLY
        // writeback and MUST execute to write -ENOSYS. Skip MUST NOT
        // fire. (If 6-C's should_skip_sigsys_setregs skipped pause
        // too, the -ENOSYS writeback would never happen → rax would
        // retain the kernel's leaked syscall-number value → init
        // would interpret that positive value as a "successful"
        // pause return → infinite pause() retry loop, same shape as
        // the pre-6-C shmget infinite loop.)
        //
        // This test is the direct regression guard for the 6-D/6-E
        // fix: it confirms 6-C's `compute_exit_return_value(...)
        // .is_some()` condition correctly excludes pause (which
        // returns None) → the skip does NOT fire for pause → the
        // SIGSYS handler's setregs fires → -ENOSYS is written →
        // init falls back to a non-pause wait instead of looping.
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32; // i386 — the runtime-relevant ABI for TWRP.
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;
        let pause_nr = abi.pause;
        // Sanity: pause is NOT in the fake-success list.
        assert_eq!(
            compute_exit_return_value(pause_nr, &abi),
            None,
            "pause must NOT be in compute_exit_return_value's fake-success list (the SIGSYS handler returns -ENOSYS for it via a dedicated branch)"
        );
        // DESYNC + NOT fake-success → must NOT skip.
        assert!(
            !should_skip_sigsys_setregs(false, pause_nr, &abi),
            "DESYNC + pause (NOT in fake-success list): skip MUST NOT fire — SIGSYS handler's setregs is the only writeback (writes -ENOSYS, NOT 0). If this skip fired, pause's -ENOSYS would never be written → rax would retain the kernel's leaked syscall number → init would see a positive 'pause returned' value → infinite pause() retry loop (same shape as pre-6-C shmget infinite loop)."
        );
    }

    // ── 6-E direct regression guard: sigsys_ret_for_pause() contract ─
    //
    // 6-D (commit 2b073f8) returned -EINTR for pause; the UI E2E test
    // on 2b073f8 showed the pause loop was STILL there (992,000+
    // repeats) — -EINTR makes init check + retry forever (property
    // service never signals readiness). Task 6-E changed the return to
    // -ENOSYS so init falls back to a non-pause wait (mirrors shmget's
    // -ENOSYS fallback). This test locks in the -ENOSYS contract so a
    // future "fix" can't regress it back to -EINTR or to 0.
    #[test]
    fn sigsys_ret_for_pause_is_enosys_not_eintr_not_zero() {
        // The SIGSYS handler MUST return -ENOSYS for pause (Task 6-E).
        assert_eq!(
            sigsys_ret_for_pause(),
            -(libc::ENOSYS as i64),
            "pause SIGSYS handler must return -ENOSYS (Task 6-E) — not -EINTR (6-D commit 2b073f8 tried -EINTR; UI E2E test showed the pause loop was still there, 992k+ repeats, because -EINTR makes init check + retry forever), not 0 (pre-6-D default; caused the post-6-C infinite pause() retry loop, 1M+ repeats on commit 368f59b)"
        );
        // Explicitly assert it's NOT -EINTR (the 6-D value that failed).
        assert_ne!(
            sigsys_ret_for_pause(),
            -(libc::EINTR as i64),
            "pause SIGSYS handler must NOT return -EINTR — 6-D commit 2b073f8 tried this and the UI E2E test showed the pause loop was still there (992,000+ repeats). -EINTR makes init think 'interrupted by a signal' → check the condition (property service not ready) → call pause() again → INFINITE LOOP, because the property service never signals readiness (kr64 has no property service)."
        );
        // Explicitly assert it's NOT 0 (the pre-6-D default that caused
        // the post-6-C infinite pause() retry loop).
        assert_ne!(
            sigsys_ret_for_pause(),
            0,
            "pause SIGSYS handler must NOT return 0 (pre-6-D default) — caused the post-6-C infinite pause() retry loop (1,048,000+ repeats on commit 368f59b) because init interpreted 0 as 'pause completed without a signal' → re-checked its condition → retried pause forever."
        );
    }

    // ── 6-G regression guard: pause_ret_after() threshold contract ──
    //
    // Task 6-G adds a `pause_count: u32` counter to run_ptrace_loop's
    // per-child state. After PAUSE_TIMEOUT_THRESHOLD (50) CONSECUTIVE
    // pause() SIGSYS calls, the SIGSYS handler returns -ETIMEDOUT
    // (-110) instead of -ENOSYS (-38) to make init give up waiting for
    // the missing property service. These tests lock in the threshold
    // contract so a future "fix" can't silently regress it (e.g. by
    // changing the threshold to u32::MAX, removing the -ETIMEDOUT
    // branch, or breaking the boundary semantics).

    #[test]
    fn pause_timeout_threshold_is_50() {
        // The threshold MUST be 50 — a deliberate choice documented in
        // the const's doc comment:
        //   - 50 pauses × 100ms sleep (6-F) = 5 seconds — a reasonable
        //     "give up" deadline for the missing property service.
        //   - Not so small that a legitimately-slow property service
        //     startup would trigger a false timeout.
        //   - Not so large that the UI E2E test (90s window) would be
        //     dominated by the pause loop before the timeout fires.
        assert_eq!(
            PAUSE_TIMEOUT_THRESHOLD, 50,
            "PAUSE_TIMEOUT_THRESHOLD must be 50 (50 × 100ms = 5s timeout for the missing property service — see the const's doc comment for the full rationale)"
        );
    }

    #[test]
    fn pause_ret_after_returns_enosys_below_threshold() {
        // Below the threshold (1..=50), pause_ret_after MUST return
        // -ENOSYS — the 6-E default. If this regressed (e.g. returned
        // -ETIMEDOUT early), init would skip the "kernel doesn't
        // implement pause" fallback path 6-E was designed to trigger,
        // and might react unpredictably to an immediate timeout.
        for count in [1u32, 5, 10, 25, 49, 50] {
            assert_eq!(
                pause_ret_after(count),
                -(libc::ENOSYS as i64),
                "pause_ret_after({}) must return -ENOSYS (not -ETIMEDOUT) — only counts STRICTLY GREATER than PAUSE_TIMEOUT_THRESHOLD (50) should time out. count={} is at-or-below the threshold.",
                count, count
            );
        }
        // The boundary case: count == PAUSE_TIMEOUT_THRESHOLD must NOT
        // time out (only > threshold does). This is the standard
        // off-by-one boundary — locking it in prevents future drift.
        assert_eq!(
            pause_ret_after(PAUSE_TIMEOUT_THRESHOLD),
            -(libc::ENOSYS as i64),
            "pause_ret_after(PAUSE_TIMEOUT_THRESHOLD={}) must return -ENOSYS (boundary — the timeout fires at STRICTLY-GREATER-THAN, not at-equal)",
            PAUSE_TIMEOUT_THRESHOLD
        );
    }

    #[test]
    fn pause_ret_after_returns_etimedout_above_threshold() {
        // ABOVE the threshold (>50), pause_ret_after MUST return
        // -ETIMEDOUT (-110) — the 6-G timeout signal. This is the
        // actual fix: after 50 consecutive pauses (5s at 100ms each),
        // tell init "the wait timed out" so it gives up waiting for
        // the missing property service.
        for count in [51u32, 52, 100, 1000, u32::MAX] {
            assert_eq!(
                pause_ret_after(count),
                -(libc::ETIMEDOUT as i64),
                "pause_ret_after({}) must return -ETIMEDOUT (not -ENOSYS) — counts STRICTLY GREATER than PAUSE_TIMEOUT_THRESHOLD (50) signal the pause loop has timed out waiting for the missing property service. count={} is above the threshold.",
                count, count
            );
        }
        // The boundary case: the FIRST count above the threshold (51)
        // MUST return -ETIMEDOUT. This is the moment the timeout
        // fires — locking it in prevents off-by-one regressions.
        assert_eq!(
            pause_ret_after(PAUSE_TIMEOUT_THRESHOLD + 1),
            -(libc::ETIMEDOUT as i64),
            "pause_ret_after(PAUSE_TIMEOUT_THRESHOLD+1={}) must return -ETIMEDOUT — this is the first count above the threshold, where the timeout fires",
            PAUSE_TIMEOUT_THRESHOLD + 1
        );
    }

    #[test]
    fn pause_ret_after_zero_returns_enosys() {
        // count == 0 should never happen at the call site (the SIGSYS
        // handler increments pause_count BEFORE calling
        // pause_ret_after, so the minimum value seen is 1). But the
        // function must still be total and return -ENOSYS (NOT
        // -ETIMEDOUT) for count=0 — defensively, an off-by-one in the
        // call site that passes 0 should NOT accidentally trigger the
        // timeout.
        assert_eq!(
            pause_ret_after(0),
            -(libc::ENOSYS as i64),
            "pause_ret_after(0) must return -ENOSYS (defensive — 0 is below the threshold so must not time out, even though the call site always passes >= 1)"
        );
    }

    /// Simulate the DESYNC stop sequence for a single seccomp-trapped
    /// chmod(nr=15) on i386 compat, and assert that the 6-W fix —
    /// ALWAYS doing a fresh `ptrace_getregs` + `set_syscall_ret` +
    /// `ptrace_setregs` (never skipping) — leaves rax=0 as the final
    /// value AND re-writes the other registers with their current
    /// values (preventing the rodata-leak SIGSEGV that the 5-J/6-C
    /// skip caused at rip=0x6f722f69).
    ///
    /// This is a SIMULATION — it doesn't fork a real child or invoke
    /// ptrace. It models the register-state transitions and verifies
    /// that `should_skip_sigsys_setregs` returns `false` (the 6-W
    /// contract: never skip), so the SIGSYS handler's setregs MUST
    /// fire. The simulation then models the fresh getregs + setregs
    /// writeback: rax is set to ret_val (0 for chmod, a fake-success
    /// syscall), and the other registers are re-written with their
    /// current (post-signal-delivery) values.
    #[test]
    fn desync_stop_sequence_always_setregs_writes_rax_zero_6w() {
        // Model the register state across the three ptrace stops for a
        // single seccomp-trapped chmod(nr=15) on i386 compat.
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;
        let chmod_nr = abi.chmod;

        // ── Stop 1: syscall-ENTRY-stop ──
        // The kernel sets orig_rax = 15 (chmod), rax = 15 (the syscall
        // number, what userspace put in eax before `int 0x80`).
        // The ENTRY handler sets in_syscall = true.
        let rax_at_entry: i64 = chmod_nr;
        let in_syscall_after_entry: bool = true;
        assert_eq!(
            rax_at_entry, chmod_nr,
            "ENTRY: rax should be the syscall number"
        );

        // ── Stop 2: syscall-EXIT-stop (DESYNC ordering) ──
        // The kernel's seccomp path called `syscall_rollback`, which
        // sets rax = orig_rax = 15 (the syscall number, NOT -ENOSYS,
        // NOT 0). The EXIT handler reads rax=15, logs it (5-H's log
        // evidence: "post-execve return #50: chmod nr=15 -> 15"), then
        // writes rax=0 via `set_syscall_ret` + `ptrace_setregs`.
        // The EXIT handler sets in_syscall = false.
        assert!(
            in_syscall_after_entry,
            "EXIT: in_syscall must be true (ENTRY set it)"
        );
        let in_syscall_after_exit: bool = false;
        // EXIT handler writes rax=0 (compute_exit_return_value(chmod, abi) == Some(0))
        let forced = compute_exit_return_value(chmod_nr, &abi);
        assert_eq!(forced, Some(0), "EXIT handler must force rax=0 for chmod");
        let rax_after_exit: i64 = 0; // EXIT handler's setregs
        assert_eq!(rax_after_exit, 0, "after EXIT handler: rax must be 0");

        // ── Stop 3: SIGSYS signal-delivery-stop ──
        // in_syscall is false here → DESYNC. 6-W contract:
        // `should_skip_sigsys_setregs` MUST return false (never skip).
        // The SIGSYS handler does a FRESH `ptrace_getregs` (re-reading
        // the CURRENT post-signal-delivery register state — NOT stale
        // pre-EXIT values), then `set_syscall_ret(rax=ret_val=0)`,
        // then `ptrace_setregs`. This writes rax=0 to the signal frame
        // so sigreturn restores it correctly, AND re-writes the OTHER
        // registers with their current values (preventing the
        // rodata-leak SIGSEGV at rip=0x6f722f69 that the 5-J/6-C skip
        // caused by leaving garbage in control-flow registers).
        let in_syscall_at_sigsys = in_syscall_after_exit;
        let skip_setregs = should_skip_sigsys_setregs(in_syscall_at_sigsys, chmod_nr, &abi);
        assert!(
            !skip_setregs,
            "6-W: DESYNC + chmod — must NOT skip setregs (always returns false). The SIGSYS handler does a fresh getregs + setregs instead, re-writing ALL registers with current values to prevent the rodata-leak SIGSEGV. (in_syscall_at_sigsys={})",
            in_syscall_at_sigsys
        );
        // The ret_val the SIGSYS handler computes for chmod (a
        // fake-success syscall) is 0 — see the SIGSYS handler's
        // "mount/mkdir/chmod/chroot/unshare" branch.
        let ret_val: i64 = 0;
        // SIGSYS handler does fresh getregs → set_syscall_ret(rax=ret_val) → setregs.
        // The fresh getregs reads the CURRENT rax (which the kernel's
        // signal-delivery setup may have left at the EXIT handler's 0,
        // OR at syscall_rollback's 15 — we don't care, because we
        // OVERWRITE rax with ret_val=0 via set_syscall_ret). The
        // setregs writes rax=0 to the signal frame.
        let rax_after_sigsys: i64 = ret_val;
        assert_eq!(
            rax_after_sigsys, 0,
            "after SIGSYS handler (DESYNC, fresh getregs + setregs): rax must be ret_val=0 — this is the 6-W fix"
        );

        // ── Child resumes ──
        // rax=0 → init sees chmod returned 0 (success) → does NOT take
        // the chmod-error path → does NOT dereference NULL+0x90 → no
        // SIGSEGV. AND, because the SIGSYS handler's setregs re-wrote
        // ALL registers with their current values, no rodata pointer
        // leaks into a control-flow register → no SIGSEGV at
        // rip=0x6f722f69 either. (The 5-J/6-C skip left the kernel's
        // signal-frame setup untouched, which is what caused the
        // rodata-leak SIGSEGV.)
        assert_eq!(
            rax_after_sigsys, 0,
            "child resumes with rax=0 — chmod reported success (6-W: fresh getregs + setregs, NOT skipped)"
        );
    }

    /// 6-W direct regression guard: `should_skip_sigsys_setregs` MUST
    /// return `false` for EVERY combination of (in_syscall_at_sigsys,
    /// syscall_nr) — the function unconditionally returns false now.
    /// This test sweeps representative cases (DESYNC + fake-success,
    /// DESYNC + non-fake-success, NORMAL + fake-success, NORMAL +
    /// non-fake-success) to lock in the 6-W contract so a future
    /// change cannot silently re-introduce the 5-J/6-C skip (which
    /// caused the iter-826 rodata-leak SIGSEGV at rip=0x6f722f69).
    #[test]
    fn should_skip_sigsys_setregs_always_false_6w() {
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;
        // Representative syscalls spanning the fake-success vs
        // non-fake-success distinction that 6-C introduced (and 6-W
        // made irrelevant to this function):
        //   - chmod: fake-success (compute_exit_return_value == Some(0))
        //   - shmget: non-fake-success (returns None; SIGSYS handler
        //     writes -ENOSYS)
        //   - pause: non-fake-success (returns None; SIGSYS handler
        //     writes -ENOSYS via dedicated branch)
        //   - a totally unrelated syscall (e.g. write=4) that is NOT
        //     in any SIGSYS/EXIT handler branch.
        let cases: [(bool, i64, &str); 8] = [
            (false, abi.chmod, "DESYNC + chmod (fake-success)"),
            (false, abi.shmget, "DESYNC + shmget (non-fake-success)"),
            (false, abi.pause, "DESYNC + pause (non-fake-success)"),
            (false, abi.write, "DESYNC + write (unrelated)"),
            (true, abi.chmod, "NORMAL + chmod (fake-success)"),
            (true, abi.shmget, "NORMAL + shmget (non-fake-success)"),
            (true, abi.pause, "NORMAL + pause (non-fake-success)"),
            (true, abi.write, "NORMAL + write (unrelated)"),
        ];
        for (in_syscall_at_sigsys, syscall_nr, label) in cases {
            assert!(
                !should_skip_sigsys_setregs(in_syscall_at_sigsys, syscall_nr, &abi),
                "6-W: {} — should_skip_sigsys_setregs must ALWAYS return false (never skip). The 5-J/6-C skip caused the iter-826 rodata-leak SIGSEGV at rip=0x6f722f69. The SIGSYS handler now ALWAYS does fresh getregs + setregs.",
                label
            );
        }
    }

    /// Simulate the NORMAL stop sequence (SIGSYS between ENTRY and EXIT)
    /// and assert the SIGSYS handler does NOT skip setregs — it's the
    /// only writeback in this case.
    #[test]
    fn normal_stop_sequence_calls_sigsys_setregs() {
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;
        let chmod_nr = abi.chmod;

        // ── Stop 1: syscall-ENTRY-stop ──
        let _rax_at_entry: i64 = chmod_nr;
        let in_syscall_after_entry: bool = true;

        // ── Stop 2: SIGSYS signal-delivery-stop (NORMAL ordering) ──
        // SIGSYS fires BETWEEN ENTRY and EXIT (the typical kernel
        // ordering for non-compat children). in_syscall is true here.
        // The EXIT handler has NOT yet run, so the SIGSYS handler's
        // `ptrace_setregs` is the ONLY writeback — must NOT skip it.
        let in_syscall_at_sigsys = in_syscall_after_entry;
        let skip_setregs = should_skip_sigsys_setregs(in_syscall_at_sigsys, chmod_nr, &abi);
        assert!(
            !skip_setregs,
            "NORMAL: must NOT skip SIGSYS setregs (in_syscall_at_sigsys={}) — it's the only writeback",
            in_syscall_at_sigsys
        );
        // SIGSYS handler calls setregs → rax=0
        let rax_after_sigsys: i64 = 0;
        assert_eq!(
            rax_after_sigsys, 0,
            "after SIGSYS handler (NORMAL, called setregs): rax=0"
        );

        // ── Stop 3: syscall-EXIT-stop (if the kernel delivers one) ──
        // The EXIT handler reads rax=0 (from the SIGSYS handler's
        // writeback), logs "post-execve return #N: chmod nr=15 -> 0",
        // and writes rax=0 again (no-op, redundant — belt-and-
        // suspenders). This is the safe ordering.
        assert_eq!(
            rax_after_sigsys, 0,
            "EXIT handler sees rax=0 (SIGSYS handler's writeback)"
        );

        // ── Child resumes ──
        assert_eq!(
            rax_after_sigsys, 0,
            "child resumes with rax=0 — chmod reported success"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_chmod_number_is_15() {
        // Regression guard: the i386 chmod syscall number MUST be 15
        // (this is the value the 4-E log shows in the crash sequence).
        // If anyone "fixes" this to a different number (e.g. by
        // accidentally copying the x86_64 value 90), the chmod EXIT
        // handler would silently stop matching and the SIGSEGV would
        // come back.
        assert_eq!(ABI_X86_32.chmod, 15, "i386 chmod must be 15");
        // The siblings — verify the i386 numbers are correct so the
        // EXIT handler matches them.
        assert_eq!(
            ABI_X86_32.lchown, 16,
            "i386 lchown must be 16 (NOT 94 or 96)"
        );
        assert_eq!(ABI_X86_32.chown, 182, "i386 chown must be 182");
        assert_eq!(ABI_X86_32.fchmodat, 306, "i386 fchmodat must be 306");
        assert_eq!(ABI_X86_32.fchownat, 298, "i386 fchownat must be 298");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_chown_sibling_numbers() {
        // x86_64 chmod / lchown / chown / fchmodat / fchownat numbers
        // per asm/unistd_64.h.
        assert_eq!(ABI_X86_64.chmod, 90, "x86_64 chmod must be 90");
        assert_eq!(ABI_X86_64.lchown, 94, "x86_64 lchown must be 94");
        assert_eq!(ABI_X86_64.chown, 182, "x86_64 chown must be 182");
        assert_eq!(ABI_X86_64.fchmodat, 268, "x86_64 fchmodat must be 268");
        assert_eq!(
            ABI_X86_64.fchownat, 260,
            "x86_64 fchownat must be 260 (NOT 257 — 257 is openat)"
        );
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn abi_aarch64_chown_sibling_numbers() {
        // aarch64 (asm-generic): no plain chmod/lchown/chown; only
        // fchmodat (53) and fchownat (54). chmod field is kept at 53
        // for historical reasons (the SIGSYS handler's "chmod" branch
        // matched syscall 53).
        assert_eq!(ABI_AARCH64.fchmodat, 53, "aarch64 fchmodat must be 53");
        assert_eq!(ABI_AARCH64.fchownat, 54, "aarch64 fchownat must be 54");
        // lchown / chown are -1 (not present on aarch64).
        assert_eq!(
            ABI_AARCH64.lchown, -1,
            "aarch64 lchown must be -1 (not present)"
        );
        assert_eq!(
            ABI_AARCH64.chown, -1,
            "aarch64 chown must be -1 (not present)"
        );
    }

    // ── Task 6-S regression guards: PTRACE_O_TRACEFORK + PTRACE_EVENT_* ──
    //
    // These tests lock in the architectural contract added by Task 6-S:
    // that `run_ptrace_loop` sets PTRACE_O_TRACEFORK | TRACECLONE |
    // TRACEVFORK | EXITKILL on init (so the kernel auto-attaches us to
    // forked children) AND handles PTRACE_EVENT_FORK | CLONE | VFORK
    // stops to log + continue the parent. The constants themselves come
    // from the `libc` crate, but a future "cleanup" that drops one of
    // them from the PTRACE_SETOPTIONS call (or from the event-match
    // arms) would silently regress fork-following — these tests make
    // that regression a compile-time / runtime failure.
    //
    // We verify the constants are non-zero AND that they have the
    // values Linux has assigned them since 2.5.46 (FORK=1, VFORK=2,
    // CLONE=3, EXIT=6) so a libc upgrade that changes the numbering
    // would surface as a test failure (the kernel ABI is stable, so
    // these should never change, but the test documents the
    // assumption explicitly).

    #[test]
    fn ptrace_o_tracefork_constants_are_nonzero() {
        // All four fork-following options must be non-zero — if any
        // were zero, `PTRACE_SETOPTIONS` would silently OR-in 0 and
        // fork-following would be a no-op (the pre-6-S behaviour).
        assert_ne!(
            libc::PTRACE_O_TRACEFORK,
            0,
            "PTRACE_O_TRACEFORK must be non-zero (kernel ABI constant)"
        );
        assert_ne!(
            libc::PTRACE_O_TRACECLONE,
            0,
            "PTRACE_O_TRACECLONE must be non-zero (kernel ABI constant)"
        );
        assert_ne!(
            libc::PTRACE_O_TRACEVFORK,
            0,
            "PTRACE_O_TRACEVFORK must be non-zero (kernel ABI constant)"
        );
        assert_ne!(
            libc::PTRACE_O_EXITKILL,
            0,
            "PTRACE_O_EXITKILL must be non-zero (kernel ABI constant)"
        );
        // The options must be DISTINCT bits — if two were the same
        // value, OR-ing them together would be a no-op for one of
        // them. The Linux kernel assigns distinct bit positions
        // (TRACEFORK=1<<1, TRACEVFORK=1<<2, TRACECLONE=1<<3, EXITKILL
        // =1<<20), so this assertion just locks in the
        // distinct-bits contract.
        let opts = [
            libc::PTRACE_O_TRACEFORK,
            libc::PTRACE_O_TRACECLONE,
            libc::PTRACE_O_TRACEVFORK,
            libc::PTRACE_O_EXITKILL,
            libc::PTRACE_O_TRACESYSGOOD,
        ];
        for i in 0..opts.len() {
            for j in (i + 1)..opts.len() {
                assert_ne!(
                    opts[i], opts[j],
                    "ptrace options must be distinct bit positions (opts[{}]={} == opts[{}]={})",
                    i, opts[i], j, opts[j]
                );
            }
        }
    }

    #[test]
    fn ptrace_event_fork_constants_have_linux_abi_values() {
        // The Linux ptrace(2) ABI has assigned these event numbers
        // since 2.5.46 / 3.0 — they are part of the kernel's stable
        // UAPI and will never change. Locking them in here catches a
        // libc crate regression that renumbers them (which would
        // break the `(status >> 16) & 0xFFFF` matching in
        // `run_ptrace_loop`'s WIFSTOPPED handler).
        assert_eq!(
            libc::PTRACE_EVENT_FORK,
            1,
            "PTRACE_EVENT_FORK must be 1 (Linux UAPI)"
        );
        assert_eq!(
            libc::PTRACE_EVENT_VFORK,
            2,
            "PTRACE_EVENT_VFORK must be 2 (Linux UAPI)"
        );
        assert_eq!(
            libc::PTRACE_EVENT_CLONE,
            3,
            "PTRACE_EVENT_CLONE must be 3 (Linux UAPI)"
        );
        assert_eq!(
            libc::PTRACE_EVENT_EXIT,
            6,
            "PTRACE_EVENT_EXIT must be 6 (Linux UAPI)"
        );
    }

    /// Smoke-test the status-bit extraction logic used in
    /// `run_ptrace_loop`'s PTRACE_EVENT_* handler. We synthesise a
    /// `status` value that LOOKS like a kernel-reported
    /// PTRACE_EVENT_FORK stop (WIFSTOPPED bit set, WSTOPSIG == SIGTRAP,
    /// upper bits == event number) and verify the extraction matches.
    /// This catches regressions in the bit math itself (e.g. if someone
    /// changes `>> 16` to `>> 8`).
    #[test]
    fn ptrace_event_status_extraction_matches_synthetic_fork_stop() {
        // Synthesise a status word for a PTRACE_EVENT_FORK stop.
        // Layout (per ptrace(2) manpage):
        //   bits 0-6   : 0x7f (WIFSTOPPED marker — low 7 bits of byte 0)
        //   bit  7    : 0 (a "stop" rather than a "signal delivery")
        //   bits 8-15  : SIGTRAP (5) — the signal WSTOPSIG reports
        //   bits 16-31 : event number (1 for PTRACE_EVENT_FORK)
        // We construct this without referring to the kernel by
        // OR-ing the components together.
        let sigtrap_byte = (libc::SIGTRAP as u32) << 8;
        let event_bits = (libc::PTRACE_EVENT_FORK as u32) << 16;
        let stop_marker: u32 = 0x7f; // WIFSTOPPED low byte
        let status: libc::c_int = (stop_marker | sigtrap_byte | event_bits) as libc::c_int;

        // WIFSTOPPED should be true.
        assert!(
            libc::WIFSTOPPED(status),
            "synthesised PTRACE_EVENT_FORK status must be WIFSTOPPED"
        );
        // WSTOPSIG should be SIGTRAP.
        assert_eq!(
            libc::WSTOPSIG(status),
            libc::SIGTRAP,
            "WSTOPSIG of a PTRACE_EVENT_FORK stop must be SIGTRAP"
        );
        // Event extraction (same math as in run_ptrace_loop).
        let extracted_event: u32 = ((status as u32) >> 16) & 0xFFFF;
        assert_eq!(
            extracted_event,
            libc::PTRACE_EVENT_FORK as u32,
            "extracted event number must match PTRACE_EVENT_FORK"
        );
        // And it must NOT be confused with a syscall stop (SIGTRAP|0x80).
        assert_ne!(
            libc::WSTOPSIG(status),
            libc::SIGTRAP | 0x80,
            "PTRACE_EVENT_FORK stop must NOT look like a syscall stop (SIGTRAP|0x80)"
        );
    }

    // ── Task 6-U: KLOG inline-capture diagnostic tests ───────────────
    //
    // The 6-U diagnostic adds two new helpers + a new ChildAbi field:
    //   - `is_kmsg_path(path)` — classifies open() paths as KLOG
    //     destinations so the EXIT handler can record the returned fd
    //   - `read_child_bytes(pid, addr, len)` — the N-byte variant of
    //     `read_child_string` (KLOG lines are NOT NUL-terminated)
    //   - `ChildAbi::write` — the write() syscall number per ABI, so
    //     the EXIT handler can match `syscall_num == abi.write`
    //     (ABI-aware — avoids cross-ABI confusion where i386 nr=1 is
    //     `exit` and x86_64 nr=4 is `stat`)
    // These tests lock in the per-ABI write numbers (regression
    // guards mirroring the existing `pause` / `mknod` / `shmget`
    // tests) and exercise `is_kmsg_path` against the realistic set of
    // paths the open() ENTRY handler will see (raw /dev/__kmsg__,
    // translated {rootfs}/dev/__kmsg__, /dev/kmsg, and non-KLOG
    // paths that must NOT match).

    #[test]
    fn is_kmsg_path_matches_dev_kmsg_dunder() {
        // TWRP init's primary KLOG destination — opened as fd 3 per
        // the worklog 5-C twrp-init-fds.log analysis.
        assert!(is_kmsg_path("/dev/__kmsg__"));
    }

    #[test]
    fn is_kmsg_path_matches_dev_kmsg() {
        // The standard Linux kernel log destination.
        assert!(is_kmsg_path("/dev/kmsg"));
    }

    #[test]
    fn is_kmsg_path_matches_translated_rootfs_variant() {
        // After translate_path rewrites /dev/__kmsg__ →
        // {rootfs}/dev/__kmsg__, the final component is still
        // "__kmsg__" — the rsplit('/') fallback matches.
        assert!(is_kmsg_path("/data/user/0/io.twoyi/rootfs/dev/__kmsg__"));
    }

    #[test]
    fn is_kmsg_path_rejects_non_kmsg_paths() {
        // Non-KLOG /dev/* paths must NOT match — otherwise the
        // EXIT handler would mis-tag unrelated writes as "DIAG KLOG".
        assert!(!is_kmsg_path("/dev/null"));
        assert!(!is_kmsg_path("/dev/zero"));
        assert!(!is_kmsg_path("/dev/__properties__"));
        assert!(!is_kmsg_path("/dev/__null__"));
        assert!(!is_kmsg_path("/init.rc"));
        assert!(!is_kmsg_path("/proc/cmdline"));
    }

    #[test]
    fn is_kmsg_path_rejects_empty_and_relative() {
        // Empty + relative paths must not match (defensive).
        assert!(!is_kmsg_path(""));
        assert!(!is_kmsg_path("relative/__kmsg__"));
    }

    #[test]
    fn is_kmsg_path_rejects_lookalikes() {
        // Paths that CONTAIN "__kmsg__" as a substring but are NOT
        // the final component must not match (e.g. /dev/__kmsg__foo
        // or /dev/__kmsg__backup).
        assert!(!is_kmsg_path("/dev/__kmsg__foo"));
        assert!(!is_kmsg_path("/dev/__kmsg__backup"));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_write_number_matches_i386_unistd_32_h() {
        // i386 write = 4 (per asm/unistd_32.h: __NR_write 4).
        // THIS is the value that fires at runtime — TWRP init (an
        // i386 binary) issues write() as syscall 4 to push KLOG
        // lines to /dev/__kmsg__. Regression guard mirroring the
        // existing `ABI_X86_32.pause == 29` test.
        assert_eq!(ABI_X86_32.write, 4, "i386 write must be 4");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_write_number_matches_unistd_64_h() {
        // x86_64 write = 1 (per asm/unistd_64.h: __NR_write 1).
        // The host is x86_64 running an i386 child, so this number
        // does NOT currently fire at runtime (the guest uses i386
        // syscall 4). Locked in for ABI completeness + so the EXIT
        // handler's `== abi.write` comparison is correct if a future
        // x86_64 guest is ever supported.
        assert_eq!(ABI_X86_64.write, 1, "x86_64 write must be 1");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_write_numbers_are_distinct_per_abi() {
        // The i386 + x86_64 write numbers MUST be different (4 vs 1)
        // — this is the entire point of the ABI-aware comparison.
        // If a future refactor accidentally copies one ABI's number
        // to the other, the EXIT handler would either miss real
        // write() calls (false negative) or fire spuriously on the
        // wrong syscall (false positive — e.g. x86_64 nr=4 is
        // `stat`, i386 nr=1 is `exit`).
        assert_ne!(
            ABI_X86_32.write, ABI_X86_64.write,
            "i386 + x86_64 write numbers must be distinct (4 vs 1) \
             — same number would break the ABI-aware comparison"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_write_does_not_collide_with_i386_exit() {
        // i386 syscall 1 is `exit` (NOT write). If the diagnostic
        // used a naive `matches!(syscall_num, 1 | 4)` it would
        // spuriously fire on i386 exit() calls (nr=1). The
        // ABI-aware `syscall_num == abi.write` (where ABI_X86_32.write
        // == 4) avoids this confusion. Lock in that ABI_X86_32.write
        // is 4 (not 1) so the comparison correctly distinguishes
        // write(4) from exit(1).
        assert_eq!(ABI_X86_32.write, 4);
        assert_ne!(ABI_X86_32.write, 1, "i386 nr=1 is exit, NOT write");
    }

    // ── Task 6-Y: mmap2 MAP_SHARED → MAP_ANONYMOUS rewrite tests ────
    //
    // The 6-Y fix rewrites the file-backed MAP_SHARED mmap2 of
    // /dev/__properties__ to anonymous so the zygote's seccomp filter
    // does not -ENOSYS it. Three layers of testable surface:
    //   (1) `is_properties_path(path)` — classifies open() paths as
    //       the property-area file so the open EXIT handler records
    //       the fd in `properties_fd`. Mirrors `is_kmsg_path`'s
    //       tests.
    //   (2) `rewrite_mmap_flags_shared_to_anonymous(flags)` — the
    //       pure flag-arithmetic core of the mmap2 ENTRY handler.
    //       Verifies MAP_SHARED is cleared, MAP_ANONYMOUS|MAP_PRIVATE
    //       are set, and other bits are preserved.
    //   (3) `ChildAbi::mmap` / `mmap2` per-ABI numbers — regression
    //       guards mirroring the existing `pause` / `mknod` / `write`
    //       / `read` per-ABI number tests.

    #[test]
    fn is_properties_path_matches_dev_properties_dunder() {
        // The canonical path TWRP init opens to set up the property
        // area. Without this match, the open EXIT handler would never
        // record the fd in `properties_fd` and the mmap2 ENTRY handler
        // would have nothing to match against.
        assert!(is_properties_path("/dev/__properties__"));
    }

    #[test]
    fn is_properties_path_matches_translated_rootfs_variant() {
        // After translate_path rewrites /dev/__properties__ →
        // {rootfs}/dev/__properties__, the final component is still
        // "__properties__" — the rsplit('/') fallback matches. This
        // is the path the open EXIT handler sees (the translated
        // string is stored in pending_open_translated_path at ENTRY).
        assert!(is_properties_path(
            "/data/user/0/io.twoyi/rootfs/dev/__properties__"
        ));
    }

    #[test]
    fn is_properties_path_rejects_non_properties_paths() {
        // Non-property /dev/* paths must NOT match — otherwise the
        // open EXIT handler would mis-record an unrelated fd as the
        // properties fd and the mmap2 ENTRY handler would rewrite an
        // unrelated mmap (corrupting it).
        assert!(!is_properties_path("/dev/null"));
        assert!(!is_properties_path("/dev/zero"));
        assert!(!is_properties_path("/dev/__kmsg__"));
        assert!(!is_properties_path("/dev/socket/property_service"));
        assert!(!is_properties_path("/init.rc"));
        assert!(!is_properties_path("/proc/cmdline"));
    }

    #[test]
    fn is_properties_path_rejects_empty_and_relative() {
        // Empty + relative paths must not match (defensive — the
        // kernel never hands us a relative open() path, but the
        // matcher should still be robust).
        assert!(!is_properties_path(""));
        assert!(!is_properties_path("relative/__properties__"));
    }

    #[test]
    fn is_properties_path_rejects_lookalikes() {
        // Paths that CONTAIN "__properties__" as a substring but are
        // NOT the final component must not match (e.g. backups,
        // tempfiles, or sibling files in /dev).
        assert!(!is_properties_path("/dev/__properties__foo"));
        assert!(!is_properties_path("/dev/__properties__backup"));
        assert!(!is_properties_path("/dev/__properties__/serial"));
    }

    #[test]
    fn rewrite_mmap_flags_clears_shared_sets_anonymous_private() {
        // The canonical case: TWRP init's property_init mmaps
        // /dev/__properties__ with MAP_SHARED (and nothing else). The
        // rewrite must clear MAP_SHARED and set MAP_ANONYMOUS|MAP_PRIVATE.
        let orig = libc::MAP_SHARED;
        let new = rewrite_mmap_flags_shared_to_anonymous(orig);
        assert_eq!(new & libc::MAP_SHARED, 0, "MAP_SHARED must be cleared");
        assert_ne!(new & libc::MAP_ANONYMOUS, 0, "MAP_ANONYMOUS must be set");
        assert_ne!(new & libc::MAP_PRIVATE, 0, "MAP_PRIVATE must be set");
    }

    #[test]
    fn rewrite_mmap_flags_preserves_other_bits() {
        // If init passes MAP_FIXED (0x10) along with MAP_SHARED, the
        // rewrite must preserve MAP_FIXED (the kernel still needs it
        // to honour the requested address). Only MAP_SHARED is
        // cleared; only MAP_ANONYMOUS|MAP_PRIVATE are added.
        let orig = libc::MAP_SHARED | libc::MAP_FIXED;
        let new = rewrite_mmap_flags_shared_to_anonymous(orig);
        assert_eq!(new & libc::MAP_SHARED, 0, "MAP_SHARED must be cleared");
        assert_ne!(new & libc::MAP_ANONYMOUS, 0, "MAP_ANONYMOUS must be set");
        assert_ne!(new & libc::MAP_PRIVATE, 0, "MAP_PRIVATE must be set");
        assert_ne!(new & libc::MAP_FIXED, 0, "MAP_FIXED must be preserved");
    }

    #[test]
    fn rewrite_mmap_flags_idempotent_for_already_anonymous() {
        // If the caller already passed MAP_ANONYMOUS|MAP_PRIVATE
        // (without MAP_SHARED), the rewrite is a no-op — the helper
        // must not corrupt already-anonymous flags. (In practice the
        // mmap2 ENTRY handler only invokes the helper when
        // `flags & MAP_SHARED != 0`, so this path is dead at runtime
        // — but the helper's idempotency is still a useful contract.)
        let orig = libc::MAP_ANONYMOUS | libc::MAP_PRIVATE;
        let new = rewrite_mmap_flags_shared_to_anonymous(orig);
        assert_eq!(new, orig, "already-anonymous flags must be unchanged");
        assert_eq!(new & libc::MAP_SHARED, 0, "MAP_SHARED must remain clear");
    }

    #[test]
    fn rewrite_mmap_flags_constant_values_lockdown() {
        // Lock in the Linux UAPI values the rewrite depends on. If a
        // future libc / kernel header change redefines these, the
        // rewrite would silently corrupt the flags — this test
        // catches that.
        // Per <sys/mman.h> + verified in libc source:
        //   MAP_SHARED     = 0x01
        //   MAP_PRIVATE    = 0x02
        //   MAP_ANONYMOUS  = 0x20  (also exposed as MAP_ANON)
        assert_eq!(
            libc::MAP_SHARED,
            0x01,
            "MAP_SHARED must be 0x01 (Linux UAPI)"
        );
        assert_eq!(
            libc::MAP_PRIVATE,
            0x02,
            "MAP_PRIVATE must be 0x02 (Linux UAPI)"
        );
        assert_eq!(
            libc::MAP_ANONYMOUS,
            0x20,
            "MAP_ANONYMOUS must be 0x20 (Linux UAPI)"
        );
        // MAP_ANON is the historical alias for MAP_ANONYMOUS — they
        // MUST have the same value. On Linux they always do.
        assert_eq!(
            libc::MAP_ANONYMOUS,
            libc::MAP_ANON,
            "MAP_ANONYMOUS and MAP_ANON must be the same value on Linux"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_mmap2_number_matches_unistd_32_h() {
        // i386 mmap2 = 192 (per asm/unistd_32.h: __NR_mmap2 192).
        // THIS is the value that fires at runtime — TWRP init (an
        // i386 binary) issues mmap2(NULL, 0x20000, PROT_READ|
        // PROT_WRITE, MAP_SHARED, fd, 0) on /dev/__properties__ as
        // i386 syscall 192 to set up the property area. The zygote's
        // seccomp filter blocks it with -ENOSYS — the 6-Y fix
        // rewrites it to MAP_ANONYMOUS. Regression guard mirroring
        // the existing ABI_X86_32.pause / .write / .read tests.
        assert_eq!(ABI_X86_32.mmap2, 192, "i386 mmap2 must be 192");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_mmap_is_sentinel_unused_by_modern_bionic() {
        // ABI_X86_32.mmap is set to -1 (sentinel) — modern i386
        // bionic (incl. TWRP init) uses mmap2 EXCLUSIVELY (nr=192),
        // NEVER the legacy plain mmap (nr=90, which takes a pointer
        // to a struct mmap_arg_struct rather than 6 direct args).
        // Setting it to -1 ensures the ENTRY match arm
        // `n if n == abi.mmap || n == abi.mmap2` reduces to
        // `n if n == abi.mmap2` (i.e. `n if n == 192`) for i386 — no
        // real syscall is ever -1, so abi.mmap is a dead branch at
        // runtime. Mirrors the ABI_AARCH64.open / .access / .lchown /
        // .chown / .mknod / .pause precedent (all -1 on aarch64).
        assert_eq!(
            ABI_X86_32.mmap, -1,
            "i386 plain mmap (nr=90) is unused by modern bionic — sentinel -1"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_mmap_number_matches_unistd_64_h() {
        // x86_64 mmap = 9 (per asm/unistd_64.h: __NR_mmap 9).
        // The host is x86_64 running an i386 child, so this number
        // does NOT currently fire at runtime (the guest uses i386
        // syscall 192). Locked in for ABI completeness + so the mmap
        // ENTRY handler's `== abi.mmap || == abi.mmap2` comparison
        // is correct if a future x86_64 guest is ever supported.
        // Regression guard mirroring the existing ABI_X86_64.write /
        // .read tests.
        assert_eq!(ABI_X86_64.mmap, 9, "x86_64 mmap must be 9");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_mmap2_is_sentinel_no_such_syscall() {
        // x86_64 has NO mmap2 (the modern x86_64 mmap takes offset in
        // BYTES directly, not in 4096-byte pages — there is no need
        // for the mmap2 page-shift workaround that i386 needs because
        // i386's orig_eax is only 32 bits and cannot pass a 64-bit
        // byte offset). ABI_X86_64.mmap2 is set to -1 (sentinel).
        assert_eq!(ABI_X86_64.mmap2, -1, "x86_64 has no mmap2 — sentinel -1");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn abi_aarch64_mmap_number_matches_asm_generic_unistd_h() {
        // aarch64 mmap = 222 (per asm-generic/unistd.h: __NR_mmap 222).
        // The host is x86_64 running an i386 child, so this number
        // does NOT currently fire at runtime. Locked in for ABI
        // completeness + so the mmap ENTRY handler's `== abi.mmap ||
        // == abi.mmap2` comparison is correct if a future aarch64
        // host is ever used.
        assert_eq!(ABI_AARCH64.mmap, 222, "aarch64 mmap must be 222");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn abi_aarch64_mmap2_is_sentinel_no_such_syscall() {
        // asm-generic has NO mmap2 — sentinel -1.
        assert_eq!(ABI_AARCH64.mmap2, -1, "aarch64 has no mmap2 — sentinel -1");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_reg_arg5_is_rdi_zero_extended_edi() {
        // i386 mmap2 layout (per kernel UAPI): arg5 = fd = edi.
        // On a 32-bit child, PTRACE_GETREGS zero-extends edi into
        // the 64-bit rdi slot (index 14 in user_regs_struct). The
        // mmap2 ENTRY handler reads arg5 via this index to fetch the
        // fd. Regression guard: if a future refactor accidentally
        // changes reg_arg5 to point at the wrong slot (e.g. r10=7,
        // which is the x86_64 arg4 slot — completely different
        // register), the rewrite would read garbage and the fix
        // would silently break.
        assert_eq!(
            ABI_X86_32.reg_arg5, 14,
            "i386 arg5 must be rdi slot (index 14) — zero-extended edi"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_reg_arg6_is_rbp_zero_extended_ebp() {
        // i386 mmap2 layout (per kernel UAPI): arg6 = offset = ebp.
        // On a 32-bit child, PTRACE_GETREGS zero-extends ebp into
        // the 64-bit rbp slot (index 4 in user_regs_struct). The
        // mmap2 ENTRY handler writes arg6 via this index to zero
        // the offset (anonymous mmap ignores offset, but we set it to
        // 0 for cleanliness). Regression guard mirroring the
        // reg_arg5 test above.
        assert_eq!(
            ABI_X86_32.reg_arg6, 4,
            "i386 arg6 must be rbp slot (index 4) — zero-extended ebp"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_reg_arg5_is_r8() {
        // x86_64 mmap layout (per kernel UAPI): arg5 = fd = r8.
        // x86_64 user_regs_struct field order:
        //   0:r15 1:r14 2:r13 3:r12 4:rbp 5:rbx 6:r11 7:r10 8:r9 9:r8
        // so r8 = index 9. The mmap ENTRY handler reads arg5 via
        // this index. Regression guard.
        assert_eq!(ABI_X86_64.reg_arg5, 9, "x86_64 arg5 must be r8 (index 9)");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_reg_arg6_is_r9() {
        // x86_64 mmap layout (per kernel UAPI): arg6 = offset = r9.
        // r9 = index 8 (see comment on abi_x86_64_reg_arg5_is_r8).
        assert_eq!(ABI_X86_64.reg_arg6, 8, "x86_64 arg6 must be r9 (index 8)");
    }

    // ── Task 6-Z3: socketcall_nr per-ABI regression guards ──────────
    //
    // The 6-Z3 fix fakes the socketcall return to 0 (success) when
    // the return is negative (error), so init's bind of the
    // property_service socket succeeds (the stale parent fd keeps
    // the address bound → EADDRINUSE). The fix matches on
    // `syscall_num == abi.socketcall_nr` (an ABI-aware comparison),
    // so the per-ABI numbers MUST match the kernel's UAPI header or
    // the fix silently misses the real syscall on the wrong ABI.
    //
    // Verified directly against /usr/include/x86_64-linux-gnu/asm/
    // unistd_32.h in Task 6-Z3:
    //   #define __NR_socketcall 102
    // (only present on i386 — x86_64 and asm-generic have no
    // socketcall; those ABIs use the direct socket/bind/listen/
    // connect/accept syscalls instead).

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_socketcall_number_matches_unistd_32_h() {
        // i386 socketcall = 102 (per asm/unistd_32.h:
        // __NR_socketcall 102, verified directly against the kernel's
        // UAPI header in Task 6-Z3). THIS is the value that fires at
        // runtime — TWRP init (an i386 binary) issues
        // socketcall(2=bind, fd, sockaddr, addrlen) to bind the
        // property_service socket. The bind returns EADDRINUSE (-98)
        // because a stale socket fd from a previous relaunch cycle
        // is still bound in the parent. The 6-Z3 fix fakes the
        // return to 0. Regression guard mirroring the existing
        // ABI_X86_32.pause / .write / .read / .mmap2 per-ABI number
        // tests.
        assert_eq!(ABI_X86_32.socketcall_nr, 102, "i386 socketcall must be 102");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_socketcall_is_sentinel_no_such_syscall() {
        // x86_64 has NO socketcall (per /usr/include/x86_64-linux-gnu/
        // asm/unistd_64.h: there is no __NR_socketcall — x86_64 uses
        // the direct socket/bind/listen/connect/accept syscalls
        // instead, numbers 41/49/50/42/43). ABI_X86_64.socketcall_nr
        // is set to -1 (sentinel). The host is x86_64 running an
        // i386 child, so this number does NOT currently fire at
        // runtime (the guest uses i386 syscall 102). Locked in for
        // ABI completeness + so the EXIT handler's
        // `== abi.socketcall_nr` comparison is correct if a future
        // x86_64 guest is ever supported. Mirrors the existing
        // ABI_X86_64.mmap2 = -1 precedent. Task 6-Z3.
        assert_eq!(
            ABI_X86_64.socketcall_nr, -1,
            "x86_64 has no socketcall — sentinel -1"
        );
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn abi_aarch64_socketcall_is_sentinel_no_such_syscall() {
        // aarch64 (asm-generic) has NO socketcall (per
        // /usr/include/asm-generic/unistd.h: there is no __NR_socketcall
        // — aarch64 uses the direct socket/bind/listen/connect/accept
        // syscalls instead, numbers 198/200/201/203/202).
        // ABI_AARCH64.socketcall_nr is set to -1 (sentinel). The host
        // is x86_64 running an i386 child, so this aarch64 number does
        // NOT currently fire at runtime. Locked in for ABI
        // completeness + so the EXIT handler's
        // `== abi.socketcall_nr` comparison is correct if a future
        // aarch64 guest is ever supported. Mirrors the existing
        // ABI_AARCH64.mmap2 = -1 precedent. Task 6-Z3.
        assert_eq!(
            ABI_AARCH64.socketcall_nr, -1,
            "aarch64 has no socketcall — sentinel -1"
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn syscall_name_resolves_socketcall_on_i386() {
        // Verify that syscall_name() recognises socketcall on i386
        // (so the SIGSYS / EXIT diagnostic logs say "socketcall"
        // instead of "[unknown]" when init trips the bind EADDRINUSE).
        assert_eq!(syscall_name(102, &ABI_X86_32), "socketcall");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn syscall_name_does_not_resolve_socketcall_on_x86_64() {
        // Verify that syscall_name() does NOT spuriously label any
        // real syscall as "socketcall" on x86_64 (where socketcall_nr
        // is the sentinel -1 — no real syscall is ever -1, so the
        // socketcall branch must never match). x86_64 uses the direct
        // socket/bind/listen/connect/accept syscalls (41/49/50/42/43)
        // — those numbers should NOT be labelled "socketcall".
        assert_ne!(syscall_name(41, &ABI_X86_64), "socketcall");
        assert_ne!(syscall_name(49, &ABI_X86_64), "socketcall");
        assert_ne!(syscall_name(50, &ABI_X86_64), "socketcall");
        assert_ne!(syscall_name(42, &ABI_X86_64), "socketcall");
        assert_ne!(syscall_name(43, &ABI_X86_64), "socketcall");
    }

    // ── Task 6-Z5: poll_nr per-ABI regression guards ───────────────
    //
    // The 6-Z5 fix fakes the poll return to 0 (no events) when the
    // return is POSITIVE (N fds ready), so the recovery's property-
    // service poll spin stops (the 6-Z3 socketcall fake-success
    // masked the bind EADDRINUSE to 0, but the socket isn't actually
    // bound → poll returns POLLERR=1 → busy-wait). The fix matches on
    // `syscall_num == abi.poll_nr` (an ABI-aware comparison gated by
    // `abi.poll_nr != -1`), so the per-ABI numbers MUST match the
    // kernel's UAPI header or the fix silently misses the real
    // syscall on the wrong ABI.
    //
    // Verified directly against /usr/include/x86_64-linux-gnu/asm/
    // unistd_32.h + unistd_64.h + asm-generic/unistd.h in Task 6-Z5:
    //   i386:   __NR_poll 168
    //   x86_64: __NR_poll   7
    //   aarch64 (asm-generic): NO __NR_poll — poll() libc wrapper
    //     issues ppoll under the hood; sentinel -1.

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_32_poll_number_matches_unistd_32_h() {
        // i386 poll = 168 (per /usr/include/x86_64-linux-gnu/asm/
        // unistd_32.h: __NR_poll 168, verified directly against the
        // kernel's UAPI header in Task 6-Z5). THIS is the value that
        // fires at runtime — TWRP recovery (an i386 binary) issues
        // poll() as i386 syscall 168 in its property_service startup
        // loop. The bind has been faked to 0 (Task 6-Z3) but the
        // socket is NOT actually bound → poll returns POLLERR=1 every
        // call → busy-wait. The 6-Z5 fix fakes the positive return to
        // 0 to break the spin. Regression guard mirroring the existing
        // ABI_X86_32.pause / .write / .read / .mmap2 / .socketcall_nr
        // per-ABI number tests.
        assert_eq!(ABI_X86_32.poll_nr, 168, "i386 poll must be 168");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn abi_x86_64_poll_number() {
        // x86_64 poll = 7 (per /usr/include/x86_64-linux-gnu/asm/
        // unistd_64.h: __NR_poll 7, verified directly against the
        // kernel's UAPI header in Task 6-Z5). The host is x86_64
        // running an i386 child, so this x86_64 number does NOT
        // currently fire at runtime (the guest uses i386 syscall
        // 168). Locked in for ABI completeness + so the EXIT handler's
        // `== abi.poll_nr` comparison is correct if a future x86_64
        // guest is ever supported. Mirrors the existing
        // ABI_X86_64.mmap / .write / .read precedent (real values, not
        // sentinels — x86_64 has a real __NR_poll).
        assert_eq!(ABI_X86_64.poll_nr, 7, "x86_64 poll must be 7");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn syscall_name_resolves_poll_on_i386() {
        // Verify that syscall_name() recognises poll on i386 (so the
        // EXIT diagnostic log says "poll" instead of "[unknown]" when
        // the recovery trips the POLLERR busy-wait). Mirrors the
        // existing syscall_name_resolves_socketcall_on_i386 guard.
        assert_eq!(syscall_name(168, &ABI_X86_32), "poll");
    }
}
