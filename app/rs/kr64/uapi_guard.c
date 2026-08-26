// Build-time UAPI guard for the ptrace constants used by
// src/ptrace_emu.rs.
//
// The Rust code takes its ptrace request numbers from the `libc` crate
// (libc::PTRACE_GETREGSET / PTRACE_SETREGSET / PTRACE_GETREGS /
// PTRACE_SETREGS) and defines NT_PRSTATUS locally (the libc crate has
// no binding for it on the android/gnu targets this crate is built
// for). This translation unit is compiled by build.rs for EVERY target
// triple (host gnu x86_64, aarch64-linux-android, x86_64-linux-android)
// and _Static_asserts that those values agree with the target
// toolchain's OWN headers (<sys/ptrace.h>, <elf.h> — glibc on the
// host, bionic from the NDK for Android targets).
//
// The literals below are assertion ORACLES — the expected UAPI values,
// taken from include/uapi/linux/ptrace.h and include/uapi/linux/elf.h
// — not runtime constants. If any toolchain ever disagreed with them,
// the BUILD fails here with a clear message instead of the tracer
// silently corrupting (or failing to read) guest registers at runtime.
//
// Note: PTRACE_GETREGS/SETREGS are asserted only on x86, where the
// x86_64 register path uses them. bionic's sys/ptrace.h also defines
// them on arm64, but the arm64 KERNEL rejects those requests — which
// is exactly why ptrace_emu.rs uses the regset interface there.

#include <elf.h>
#include <sys/ptrace.h>

_Static_assert(NT_PRSTATUS == 1,
    "kr64: NT_PRSTATUS (ptrace_emu.rs) != 1 — out of sync with target <elf.h>");

_Static_assert(PTRACE_GETREGSET == 0x4204,
    "kr64: libc PTRACE_GETREGSET != 0x4204 — out of sync with target <sys/ptrace.h>");
_Static_assert(PTRACE_SETREGSET == 0x4205,
    "kr64: libc PTRACE_SETREGSET != 0x4205 — out of sync with target <sys/ptrace.h>");

#if defined(__x86_64__) || defined(__i386__)
_Static_assert(PTRACE_GETREGS == 12,
    "kr64: libc PTRACE_GETREGS != 12 — out of sync with target <sys/ptrace.h>");
_Static_assert(PTRACE_SETREGS == 13,
    "kr64: libc PTRACE_SETREGS != 13 — out of sync with target <sys/ptrace.h>");
#endif

// Give the archive a symbol so the linker keeps it; the guard itself
// is purely compile-time.
int twoyi_uapi_guard = 1;
