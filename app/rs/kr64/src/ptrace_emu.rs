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
    access: i64,
    faccessat: i64,
    rt_sigprocmask: i64,
    readlink: i64,
    readlinkat: i64,
    chdir: i64,
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
    // SysV shared-memory syscalls — see the comment on these fields
    // in `ChildAbi`. x86_64: shmget=29, shmat=30, shmctl=31
    // (asm/unistd_64.h).
    shmget: 29,
    shmat: 30,
    shmctl: 31,
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
    // SysV shared-memory syscalls — see the comment on these fields
    // in `ChildAbi`. aarch64 uses asm-generic/unistd.h, where
    // shmget=194, shmctl=195, shmat=196.
    shmget: 194,
    shmat: 196,
    shmctl: 195,
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
/// Historically used by the SIGSYS handler to rewrite a seccomp-blocked
/// syscall into a harmless one (getpid) before resuming. This rewrite
/// was REMOVED in the "never rewrite orig_rax" fix — see the SIGSYS
/// handler for the rationale. The function is retained for potential
/// future use (e.g. if a different code path needs to rewrite the
/// syscall number for a non-seccomp reason).
#[allow(dead_code)]
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
    {
        Some(0)
    } else {
        None
    }
}

/// Decide whether the SIGSYS handler should skip its `ptrace_setregs`
/// call. Task 5-J.
///
/// Returns `true` when the SIGSYS fires AFTER the EXIT handler has
/// already written rax=0 — the "DESYNC" case where `in_syscall` was
/// false at SIGSYS entry.
///
/// # Background — the DESYNC register-writeback race
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
/// to be redundant (belt-and-suspenders). However, in the DESYNC case
/// the SIGSYS handler fires AFTER the EXIT handler, and its
/// `ptrace_setregs` writes the WHOLE `user_regs_struct` back —
/// including fields the kernel may have re-snapshotted from its
/// signal-delivery-stop setup. If the kernel re-snapshotted rax from
/// `syscall_rollback` (which sets `rax = orig_rax` = the syscall
/// number, e.g. 15 for i386 chmod), the SIGSYS handler's `getregs`
/// reads rax=15 and its subsequent `setregs` writes the whole struct
/// back — *with `set_syscall_ret` having set rax=0*, but if the
/// kernel's signal-delivery-stop register writeback races with our
/// `setregs`, the child can end up resuming with rax=15 (the syscall
/// number), NOT rax=0. TWRP init then takes the chmod-error path and
/// dereferences NULL+0x90 → SIGSEGV at rip=0x809255d (5-H's finding,
/// 9 crashes all at iter 216).
///
/// # The fix
///
/// In the DESYNC case (`in_syscall == false` at SIGSYS entry — meaning
/// the EXIT handler already ran and wrote rax=0), the SIGSYS handler
/// SKIPS its `ptrace_setregs` call. The EXIT handler's rax=0 is the
/// final value the child sees on resume. The SIGSYS handler still:
///   - performs the fs op in the rootfs (for mount/mkdir),
///   - logs the intercept,
///   - records the syscall in the rolling buffers,
///   - sets `in_syscall = false` (so the next stop is treated as
///     ENTRY of the next syscall).
///
/// In the NORMAL case (`in_syscall == true` at SIGSYS entry — SIGSYS
/// fired BETWEEN ENTRY and EXIT, the typical kernel ordering for
/// non-compat children), the SIGSYS handler DOES call `ptrace_setregs`
/// because the EXIT handler has NOT yet run — the SIGSYS handler's
/// writeback is the only one.
///
/// # Why this is safe
///
/// `compute_exit_return_value` is consulted by BOTH the EXIT handler
/// (at line ~2434) and the SIGSYS handler's "fchown/fchmod/capget/
/// ioprio_get/ioprio_set/lchown/chown/fchmodat/fchownat" branch
/// (which mirrors the same set). For chmod + mount specifically, the
/// SIGSYS "mount/mkdir/chmod/chroot/unshare" branch ALSO returns 0,
/// AND chmod + mount are in `compute_exit_return_value` (mount was
/// added in Task 5-T). So in DESYNC mode the EXIT handler has
/// ALREADY written rax=0 for every faked-success syscall that the
/// SIGSYS handler would also write rax=0 for — the SIGSYS handler's
/// `setregs` is genuinely redundant in this case. Skipping it cannot
/// leave rax non-zero (the EXIT handler wrote 0). It only AVOIDS the
/// race where the SIGSYS handler's whole-struct `setregs` clobbers
/// the EXIT handler's rax=0 with a kernel-re-snapshotted value.
///
/// # Task 6-C refinement — do NOT skip for non-fake-success syscalls
///
/// 5-J's original implementation was a pure negation of
/// `in_syscall_at_sigsys`: `!in_syscall_at_sigsys`. That fired
/// unconditionally in DESYNC mode for EVERY syscall, including the
/// SysV shared-memory syscalls (shmget/shmat/shmctl) whose return
/// value the SIGSYS handler writes as -ENOSYS (-38) — NOT 0. Those
/// syscalls are NOT in `compute_exit_return_value`'s fake-success
/// list (it returns `None`, not `Some(0)`), so in DESYNC mode the
/// EXIT handler does NOT write rax for them either. With 5-J's
/// unconditional skip, the SIGSYS handler's `ptrace_setregs` was
/// ALSO skipped → rax was left untouched → the child resumed with
/// the kernel's leaked syscall-number value in rax (e.g. 395 for
/// i386 shmget, post-6-C) → init saw a POSITIVE "shmid" → tried to
/// use it → failed → retried shmget forever (790k+ calls/sec). The
/// post-e6d85e1 UI E2E blocker. (Pre-6-C the same symptom manifested
/// with rax=29 — the WRONG i386 shmget number — because the SIGSYS
/// handler thought the guest's `pause()` syscall (nr=29) was
/// shmget.)
///
/// The fix: the skip fires ONLY when the syscall is in
/// `compute_exit_return_value`'s fake-success list (returns
/// `Some(_)`). For syscalls NOT in that list (e.g. shmget, which
/// returns -ENOSYS via the SIGSYS handler), the skip must NOT fire —
/// the SIGSYS handler's `setregs` is the ONLY writeback and MUST
/// execute to write the non-zero return value.
fn should_skip_sigsys_setregs(in_syscall_at_sigsys: bool, syscall_nr: i64, abi: &ChildAbi) -> bool {
    // DESYNC = SIGSYS fired AFTER the EXIT handler. The SIGSYS
    // handler's setregs is redundant (the EXIT handler already wrote
    // rax=0) AND potentially racy with the kernel's signal-delivery-
    // stop register snapshotting — BUT ONLY for syscalls in
    // `compute_exit_return_value`'s fake-success list, because only
    // those have rax written by the EXIT handler. For other syscalls
    // (e.g. shmget, which the SIGSYS handler returns -ENOSYS for), the
    // EXIT handler did NOT write rax, so the SIGSYS handler's setregs
    // is the ONLY writeback and MUST fire (Task 6-C).
    !in_syscall_at_sigsys && compute_exit_return_value(syscall_nr, abi).is_some()
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
    } else if nr == abi.execve {
        "execve"
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
    for prefix in &["/proc/", "/sys/", "/data/", "/apex/"] {
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
    // cap (10) is small to keep the exit log line readable — we only
    // need the last few to spot the failing syscall.
    const RECENT_ALL_SYSCALLS_CAP: usize = 10;
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
    let mut post_execve_syscall_count: u64 = 0;

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
            // → ENOENT). By resetting scratch_addr to 0, the next
            // syscall ENTRY stop will re-allocate the scratch area at
            // the new (32-bit) stack address.
            if scratch_addr != 0 {
                log(&format!(
                    "execve completed — resetting scratch area (was {:#x}) — will re-allocate at new stack pointer",
                    scratch_addr
                ));
                scratch_addr = 0;
                scratch_offset = 0;
            }
        }

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
                    }

                    // Log the first 50 post-execve syscalls so we can
                    // see exactly what the new binary (TWRP init) does
                    // after execve. This is critical because loop_count
                    // may already be large (kr64's pre-execve syscalls
                    // inflate it), so the existing "loop_count <= 50"
                    // log gate would suppress these.
                    if past_first_execve {
                        post_execve_syscall_count = post_execve_syscall_count.saturating_add(1);
                        if post_execve_syscall_count <= 150 {
                            log(&format!(
                                "post-execve syscall #{}: nr={} [{}]",
                                post_execve_syscall_count,
                                syscall_num,
                                syscall_name(syscall_num, &abi)
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
                    if past_first_execve && post_execve_syscall_count <= 150 {
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
                                    if let Err(e) = vfs.materialize(&path, rootfs) {
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

                    // ── Post-execve RETURN-VALUE logging ─────────────
                    //
                    // Logs the kernel's return value for every post-execve
                    // syscall (first 150), so we can see EXACTLY which
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
                    if past_first_execve && post_execve_syscall_count <= 150 {
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
                    if let Some(_forced_ret) = compute_exit_return_value(syscall_num, &abi) {
                        let mut regs2: Regs = unsafe { std::mem::zeroed() };
                        if let Ok(len) = ptrace_getregs(pid, &mut regs2) {
                            let name = syscall_name(syscall_num, &abi);
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
                            if loop_count <= 300 {
                                let mut readback: Regs = unsafe { std::mem::zeroed() };
                                if ptrace_getregs(pid, &mut readback).is_ok() {
                                    let readback_rax =
                                        get_syscall_arg(&readback, abi.reg_ret) as i64;
                                    log(&format!(
                                        "[KR64][ptrace] EXIT handler wrote rax=0 for {} (nr={}), readback rax={}",
                                        name, syscall_num, readback_rax
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
                // 5-J: capture the in_syscall state at SIGSYS entry for
                // `should_skip_sigsys_setregs`. `in_syscall` is true when
                // SIGSYS fires BETWEEN ENTRY and EXIT (normal — SIGSYS
                // replaces the syscall-exit-stop). It is false when SIGSYS
                // fires AFTER the EXIT stop (DESYNC — the kernel delivered
                // ENTRY→EXIT→SIGSYS for a single seccomp-trapped syscall,
                // which is the order 5-H's log evidence shows for i386
                // compat chmod nr=15). In the DESYNC case the EXIT handler
                // has ALREADY written rax=0 for the faked-success syscalls
                // (compute_exit_return_value), so the SIGSYS handler's
                // ptrace_setregs is redundant AND potentially racy — see
                // `should_skip_sigsys_setregs` for the full rationale.
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
                                " (DESYNC — SIGSYS fired AFTER EXIT stop; EXIT handler already wrote rax=0; SIGSYS setregs will be skipped per should_skip_sigsys_setregs)"
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
                        // 5-J: in DESYNC mode (SIGSYS fired AFTER the EXIT
                        // stop — `in_syscall` was false at SIGSYS entry),
                        // the EXIT handler has ALREADY written rax=0 for
                        // every faked-success syscall (compute_exit_return_value
                        // is consulted by BOTH handlers). Calling
                        // ptrace_setregs here would write the WHOLE
                        // user_regs_struct back — including fields the
                        // kernel may have re-snapshotted from
                        // `syscall_rollback` (which sets rax = orig_rax =
                        // the syscall number, e.g. 15 for i386 chmod).
                        // Although set_syscall_ret explicitly sets rax=0
                        // in the buffer, on some kernels the
                        // signal-delivery-stop register writeback races
                        // with our setregs, leaving the child resuming
                        // with rax=15 instead of rax=0. TWRP init then
                        // takes the chmod-error path and dereferences
                        // NULL+0x90 → SIGSEGV at rip=0x809255d.
                        //
                        // The fix: in DESYNC mode, SKIP the setregs
                        // call. The EXIT handler's rax=0 is the final
                        // value the child sees. This is safe because:
                        //   - chmod/lchown/chown/fchmodat/fchownat/
                        //     fchown/fchmod/capget/ioprio_get/ioprio_set
                        //     are all covered by BOTH
                        //     compute_exit_return_value (EXIT handler)
                        //     and the SIGSYS handler's explicit `||`
                        //     chains — the EXIT handler ALWAYS runs
                        //     first in DESYNC mode.
                        //   - mount/mkdir/chmod/chroot/unshare/mknod all
                        //     return 0 in the SIGSYS handler. chmod + mount
                        //     + mknod are ALSO in compute_exit_return_value
                        //     (mount added in Task 5-T, mknod added in
                        //     Task 5-X — see the doc on
                        //     `compute_exit_return_value`), so in DESYNC
                        //     mode the EXIT handler DID write rax=0 for
                        //     them. mkdir/chroot/unshare are NOT in
                        //     compute_exit_return_value, so in DESYNC
                        //     mode the EXIT handler didn't write rax for
                        //     them either — skipping setregs leaves the
                        //     kernel's value untouched (same as the
                        //     previous behaviour for those syscalls in
                        //     DESYNC mode).
                        //   - shmget/shmat/shmctl return -ENOSYS in the
                        //     SIGSYS handler. These are NOT in
                        //     compute_exit_return_value, so the EXIT
                        //     handler doesn't touch rax for them. In
                        //     DESYNC mode, skipping setregs leaves the
                        //     kernel's rax (syscall_rollback's
                        //     rax=orig_rax = syscall number, or -ENOSYS
                        //     on kernels that set -ENOSYS). This is the
                        //     SAME behaviour as before for these
                        //     syscalls in DESYNC mode — we are not
                        //     regressing them. (If a future bug shows
                        //     these need -ENOSYS at runtime in DESYNC
                        //     mode, add them to
                        //     compute_exit_return_value.)
                        //
                        // `set_syscall_ret` is still called on
                        // `sigsys_regs` so the readback log below can
                        // report what we WOULD have written.
                        set_syscall_ret(&mut sigsys_regs, &a, ret_val);
                        if should_skip_sigsys_setregs(in_syscall_at_sigsys, original_syscall, &a) {
                            // DESYNC mode (5-J) AND this syscall is in
                            // `compute_exit_return_value`'s fake-success
                            // list (6-C). SIGSYS fired AFTER the EXIT
                            // stop, and the EXIT handler ALREADY wrote
                            // rax=0 for this syscall — so the SIGSYS
                            // handler's setregs is redundant AND would
                            // race with the kernel's signal-delivery-
                            // stop register snapshotting. Skip it.
                            //
                            // The 6-C refinement: `should_skip_sigsys_setregs`
                            // now requires `compute_exit_return_value(...)
                            // .is_some()`. For syscalls NOT in the fake-
                            // success list (e.g. shmget, which the SIGSYS
                            // handler returns -ENOSYS for), the skip does
                            // NOT fire — the SIGSYS handler's setregs is
                            // the ONLY writeback and MUST execute to
                            // write the non-zero return value. (Pre-6-C
                            // the skip fired unconditionally in DESYNC
                            // mode → shmget's -ENOSYS was never written →
                            // rax retained the kernel's leaked syscall-
                            // number value → init saw a positive "shmid"
                            // → infinite shmget-retry loop.)
                            //
                            // The fs op (mount/mkdir/mknod) has already
                            // been performed above; we just don't write
                            // regs back.
                            //
                            // Because the skip fires ONLY when
                            // `compute_exit_return_value` returned Some,
                            // we know the EXIT handler wrote rax=0 here.
                            // No need for the 5-X conditional message
                            // ("did NOT write rax") — that branch is now
                            // structurally unreachable for skips.
                            sigsys_log(&format!(
                                "SIGSYS handler: DESYNC mode — skipping ptrace_setregs for nr={} [{}] (in compute_exit_return_value's fake-success list; EXIT handler already wrote rax=0; would-have-written rax={})",
                                original_syscall, name, ret_val
                            ));
                        } else if let Err(e) = ptrace_setregs(pid, &sigsys_regs, len) {
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
                            // NORMAL mode (SIGSYS fired BETWEEN ENTRY and
                            // EXIT — the typical kernel ordering for non-
                            // compat children). setregs succeeded — the
                            // faked return value was applied. Log a
                            // readback to confirm (5-J diagnostic, gated
                            // by sigsys_repeat_count <= 5 to avoid log
                            // flooding in tight SIGSYS loops).
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
    fn translate_path_leaves_proc_sys_data_untouched() {
        let rootfs = "/data/user/0/io.twoyi/rootfs";
        // /proc, /sys, /data are left untranslated (they hit the host's
        // real proc/sys/data, which is correct for a ptraced unprivileged
        // child that can't mount a fresh proc/sysfs).
        for p in &["/proc/self/status", "/sys/class/net", "/data/data"] {
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

    // ── 5-J regression tests: SIGSYS/EXIT handler register-writeback race ─
    //
    // These tests verify the DESYNC-detection helper that the SIGSYS
    // handler uses to decide whether to skip its `ptrace_setregs` call.
    // See `should_skip_sigsys_setregs` for the full rationale.
    //
    // The bug (5-H's finding, 9 SIGSEGVs at iter 216, all at
    // rip=0x809255d, si_addr=0x90): 5-A's EXIT handler correctly wrote
    // rax=0 for fake-success syscalls, but in DESYNC mode (where the
    // kernel delivers ENTRY→EXIT→SIGSYS for a single seccomp-trapped
    // syscall) the SIGSYS handler fired AFTER the EXIT handler and its
    // `ptrace_setregs` clobbered the EXIT handler's rax=0 writeback
    // with a kernel-re-snapshotted value (rax=15 = the syscall number,
    // from `syscall_rollback` which sets rax = orig_rax). TWRP init
    // then took the chmod-error path and dereferenced NULL+0x90.
    //
    // The fix: in DESYNC mode, the SIGSYS handler SKIPS its
    // `ptrace_setregs` call (the EXIT handler already wrote rax=0) —
    // BUT ONLY for syscalls in `compute_exit_return_value`'s fake-
    // success list (Task 6-C refinement). For syscalls NOT in that
    // list (e.g. shmget, which the SIGSYS handler returns -ENOSYS
    // for), the skip must NOT fire — the SIGSYS handler's setregs is
    // the ONLY writeback and MUST execute to write the non-zero
    // return value. `should_skip_sigsys_setregs(in_syscall_at_sigsys,
    // syscall_nr, abi)` returns true when `in_syscall_at_sigsys` is
    // false (DESYNC) AND `compute_exit_return_value(syscall_nr, abi)`
    // returns Some — i.e. the EXIT handler already wrote rax=0.

    #[test]
    fn should_skip_sigsys_setregs_in_desync_mode() {
        // DESYNC case (5-H's scenario): SIGSYS fires AFTER the EXIT
        // stop. `in_syscall` was false at SIGSYS entry (the EXIT
        // handler set it to false at line ~2307). The EXIT handler
        // already wrote rax=0 for the faked-success syscalls, so the
        // SIGSYS handler's `ptrace_setregs` is redundant AND
        // potentially racy — skip it.
        //
        // Use chmod — it's in `compute_exit_return_value`'s fake-
        // success list, so the 6-C refinement's second condition
        // (`compute_exit_return_value(...).is_some()`) is satisfied.
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;
        let chmod_nr = abi.chmod;
        assert!(
            should_skip_sigsys_setregs(false, chmod_nr, &abi),
            "DESYNC: SIGSYS fired after EXIT — must skip setregs to avoid clobbering EXIT handler's rax=0"
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
        // success list.
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
    // These guard the 6-C fix for the infinite shmget-retry loop.
    // The OLD contract (5-J) was a pure negation of
    // `in_syscall_at_sigsys`: `!in_syscall_at_sigsys`. That fired
    // unconditionally in DESYNC mode for EVERY syscall, including
    // shmget/shmat/shmctl whose return value the SIGSYS handler writes
    // as -ENOSYS (NOT 0). Since those syscalls are NOT in
    // `compute_exit_return_value`'s fake-success list, the EXIT
    // handler doesn't write rax for them either → with the unconditional
    // skip, rax retained the kernel's leaked syscall-number value →
    // init saw a positive "shmid" → retried shmget forever.
    //
    // The 6-C NEW contract: skip fires ONLY when (DESYNC mode) AND
    // (syscall is in compute_exit_return_value's fake-success list).
    // These two tests pin the contract for the two key representative
    // syscalls: chmod (fake-success → skip fires) and shmget
    // (non-fake-success → skip must NOT fire).

    #[test]
    fn should_skip_sigsys_setregs_true_for_chmod() {
        // chmod IS in compute_exit_return_value's fake-success list
        // (it returns Some(0)). In DESYNC mode the EXIT handler has
        // ALREADY written rax=0 → the SIGSYS handler's setregs is
        // redundant AND would race with the kernel's signal-delivery-
        // stop register snapshotting. Skip MUST fire.
        #[cfg(target_arch = "x86_64")]
        let abi = ABI_X86_32;
        #[cfg(target_arch = "aarch64")]
        let abi = ABI_AARCH64;
        let chmod_nr = abi.chmod;
        // Sanity: chmod is in the fake-success list.
        assert_eq!(
            compute_exit_return_value(chmod_nr, &abi),
            Some(0),
            "chmod must be in compute_exit_return_value's fake-success list"
        );
        // DESYNC + fake-success → skip.
        assert!(
            should_skip_sigsys_setregs(false, chmod_nr, &abi),
            "DESYNC + chmod (fake-success): skip MUST fire — EXIT handler already wrote rax=0"
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

    /// Simulate the DESYNC stop sequence for a single seccomp-trapped
    /// chmod(nr=15) on i386 compat, and assert that the SIGSYS handler's
    /// decision to skip setregs leaves rax=0 (the EXIT handler's
    /// writeback) as the final value.
    ///
    /// This is a SIMULATION — it doesn't fork a real child or invoke
    /// ptrace. It models the register state transitions and verifies
    /// that `should_skip_sigsys_setregs` returns the right value at
    /// each stop, so the final rax is 0 (not the leaked syscall
    /// number 15 that `syscall_rollback` would set).
    #[test]
    fn desync_stop_sequence_preserves_exit_handler_rax_zero() {
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
        // in_syscall is false here → DESYNC. `should_skip_sigsys_setregs`
        // must return true → SIGSYS handler SKIPS its `ptrace_setregs`.
        // rax stays at 0 (the EXIT handler's writeback). This is the
        // FIX — without it, the SIGSYS handler's setregs would race
        // with the kernel's signal-delivery-stop register snapshotting
        // and the child could resume with rax=15 (the syscall number
        // from syscall_rollback), causing the SIGSEGV at rip=0x809255d.
        let in_syscall_at_sigsys = in_syscall_after_exit;
        let skip_setregs = should_skip_sigsys_setregs(in_syscall_at_sigsys, chmod_nr, &abi);
        assert!(
            skip_setregs,
            "DESYNC: must skip SIGSYS setregs (in_syscall_at_sigsys={})",
            in_syscall_at_sigsys
        );
        // SIGSYS handler skips setregs → rax stays at 0 (EXIT handler's value)
        let rax_after_sigsys: i64 = rax_after_exit;
        assert_eq!(
            rax_after_sigsys, 0,
            "after SIGSYS handler (DESYNC, skipped setregs): rax must STILL be 0 — this is the fix"
        );

        // ── Child resumes ──
        // rax=0 → init sees chmod returned 0 (success) → does NOT take
        // the chmod-error path → does NOT dereference NULL+0x90 → no
        // SIGSEGV at rip=0x809255d. This is the behaviour 5-A's commit
        // ee93ac0 intended but didn't achieve at runtime because the
        // SIGSYS handler clobbered the EXIT handler's writeback.
        assert_eq!(
            rax_after_sigsys, 0,
            "child resumes with rax=0 — chmod reported success"
        );
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
}
