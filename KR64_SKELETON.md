# Twoyi — `kr64` Skeleton Implementation (Task KR64-IMPL-1)

> **Author:** general-purpose sub-agent
> **Date:** 2026-08-05
> **Task ID:** KR64-IMPL-1
> **Codespace:** `twoyi-dev-3-jr47xg6xvx7ghq6p`
> **Crate path:** `app/rs/kr64/`
> **Inputs:** `VM_KR64_ANALYSIS.md`, `GSI_BOOT_PLAN.md`, `worklog.md`

---

## 0. TL;DR

A compiling, tested **skeleton** of the twoyi kernel-replacement daemon
(the Rust port of Virtual Master's `libkr64.so`) has been created at
`app/rs/kr64/`. It depends on **only** `libc` (no `log`, no `once_cell`,
no `nix` — per the task spec). All 26 unit tests pass on the Linux host.
Zero compiler warnings.

```
kr64 v0.1.0 (app/rs/kr64)
└── libc v0.2.189
[build-dependencies]
└── cc v1.4.0          (for compiling interp.c)
```

---

## 1. Files created / modified

| File | Lines | Purpose |
|------|-------|---------|
| `app/rs/kr64/Cargo.toml` | 39 | Crate manifest. `crate-type = ["cdylib", "rlib"]` + `[[bin]]`. Deps: `libc` only. |
| `app/rs/kr64/build.rs` | 88 | Compiles `interp.c`; emits PIE linker flags (`-Wl,-e,kr64_main`, `-Wl,--undefined=interp`). Android-only: `--dynamic-linker=/system/bin/linker64`. |
| `app/rs/kr64/interp.c` | 40 | Forces a `.interp` section (PT_INTERP) so `libkr64.so` is directly executable via the PIE-as-cdylib trick. On Android: `/system/bin/linker64`. On Linux host: no override (so `cargo test` runs). |
| `app/rs/kr64/src/main.rs` | 38 | Binary entry point. Thin wrapper around `kr64::run(args)`. |
| `app/rs/kr64/src/lib.rs` | 652 | Crate root. `Config` struct, `parse_args()`, `run()` daemon entry point, `kr64_main` cdylib entry, `info!`/`warning!`/`error!` macros (eprintln-based, no `log` crate). |
| `app/rs/kr64/src/devices.rs` | 405 | Virtual `/dev` tree: `qemu_pipe`, `touch`, `key0`, `event`, `gb`, `gb2` via `UnixListener::bind`. |
| `app/rs/kr64/src/seccomp.rs` | 831 | BPF seccomp filter + SIGSYS handler. Allow ~80 syscalls, trap `mount`/`umount2`/`swapon`/`reboot`/etc., kill on `ptrace`/`kexec_load`/`init_module`/`pivot_root`. |
| `app/rs/kr64/src/proc_emu.rs` | 534 | Synthesises `/proc/version`, `/proc/cpuinfo`, `/proc/meminfo`, `/proc/cmdline`, `/proc/mounts`, `/proc/self/`, `/proc/sys/kernel/*`, `/proc/sys/vm/*`. |
| `app/rs/kr64/src/mount_mgr.rs` | 457 | `unshare(CLONE_NEWNS)` → bind-mount ROM partitions → tmpfs on `/dev`/`/proc`/`/sys`/`/tmp`/`/apex`/`/mnt` → `pivot_root` → `umount2(old_root)`. Falls back to `chroot` on EPERM. |
| **Total** | **3,084** | |

---

## 2. How it maps to Virtual Master's `libkr64.so`

| VM's `libkr64.so` behaviour (from `VM_KR64_ANALYSIS.md`) | Twoyi `kr64` skeleton |
|---|---|
| Standalone PIE executable disguised as `.so`, launched via `fork`+`exec` | `crate-type = ["cdylib", "rlib"]` + `interp.c` + `build.rs` PIE flags. Entry point `kr64_main` (cdylib) / `main` (bin). |
| 7-arg invocation: `vmid, data_dir, rom_dir, kernel_path, config_path, log_level, socket_fd` | `parse_args()` with named flags: `--rootfs`, `--data-dir`, `--rom-dir`, `--init`, `--vmid`, `--width`, `--height`, `--dpi`, `--log-level`, `--no-namespaces`, `--rw-rom`, `--no-seccomp`. |
| Creates 20+ virtual devices via `mknodat(S_IFSOCK)` + `bind()` (§6, call site `0x11d770`) | `devices::create_all_devices()` — creates 6 MVP devices via `UnixListener::bind` (qemu_pipe, touch, key0, event, gb, gb2). Full 20+ inventory is a follow-up task. |
| `seccomp(SECCOMP_SET_MODE_FILTER, …)` + `sigaction(SIGSYS, …)` (§12) | `seccomp::install()` — `PR_SET_NO_NEW_PRIVS` → `sigaction(SIGSYS, sigsys_handler)` → `seccomp(SET_MODE_FILTER, TSYNC, &bpf_prog)`. |
| SIGSYS handler logs `BLOCKED.SYSCALL.FAILED: <nr>`, emulates or kills (§11, key 0xc9/0xc2) | `seccomp::sigsys_handler()` — reads `si_syscall` via a `SigsysSiginfo` reinterpret-cast, classifies via `classify()`, sets return value + advances PC (arch-specific for aarch64/x86_64). |
| `/dev/vmproc` emulates `/proc` (§2.8, key 0x47/0x64/0x07) | `proc_emu::populate_proc()` — writes synthesised `/proc/{version,cpuinfo,meminfo,cmdline,mounts,self/,sys/}` as static files (MVP — shadowhook interception is a follow-up). |
| `mount_mgr: %s -> %s -> %s` (§4.2, key 0x1a) — `unshare`+`pivot_root` | `mount_mgr::setup_mounts()` — `unshare(CLONE_NEWNS)` → `MS_REC|MS_PRIVATE` → bind-mount `/system`,`/vendor`,`/product`,`/system_ext` → tmpfs on 6 paths → `pivot_root` → `umount2(MNT_DETACH)`. |
| `execve(/system/bin/init)` as the guest's PID 1 | `run()` forks; child calls `mount_mgr::setup_mounts()` → `seccomp::install()` → `execve(init_path)`. Parent runs device-accept threads + `waitpid`. |

---

## 3. Design decisions

### 3.1 std + libc only (no external crates)

The task spec said "Use only std + libc (no external crates for now)."
The skeleton respects this strictly:

- **Logging:** Replaced the `log` crate with crate-local `info!` /
  `warning!` / `error!` macros that expand to `eprintln!("[KR64 <LEVEL>] …")`.
  (Named `warning!` not `warn!` to avoid a conflict with Rust's built-in
  `#[warn(...)]` lint attribute.)
- **Lazy statics:** Replaced `once_cell::sync::Lazy` with
  `std::sync::OnceLock` (stabilised in Rust 1.70).
- **Syscall wrappers:** Used `libc::*` directly instead of `nix`.

### 3.2 PIE-as-cdylib (directly-executable `.so`)

Mirrors VM's `libkr64.so` (which is a standalone ELF executable
disguised as a `.so`, with `.interp` pointing at `libkrloader64.so`).

Twoyi's version:
- `interp.c` puts `/system/bin/linker64` in the `.interp` section (on
  Android) so the kernel exec's `linker64`, which loads `libkr64.so`
  and jumps to `kr64_main` (set via `-Wl,-e,kr64_main`).
- On Linux host builds, `interp.c` emits a plain `.rodata` symbol (no
  `.interp` override) so `cargo test` runs with the default
  `/lib64/ld-linux-x86-64.so.2` interpreter.
- `build.rs` emits the PIE flags only for the cdylib target, and only
  emits `--dynamic-linker=/system/bin/linker64` when
  `CARGO_CFG_TARGET_OS == "android"`.

### 3.3 SIGSYS handler — `si_syscall` access

The `libc` crate's `siginfo_t` doesn't expose `si_syscall` as a method.
The handler reinterprets the `*mut siginfo_t` pointer through a
`#[repr(C)] struct SigsysSiginfo` that mirrors the kernel's
`__sifields.__sigsys` layout (signo, errno, code, _pad, call_addr,
syscall, arch). This is safe because the kernel-userland `siginfo` ABI
is fixed and architecture-independent.

### 3.4 Seccomp BPF program structure

```
  1. ld arch                                   // load audit arch
  2. jeq EXPECTED_ARCH, jt=0, jf=N             // wrong arch → kill
  3. ld nr                                     // load syscall number
  4. for each allowed syscall: jeq + ret ALLOW
  5. for each trapped syscall: jeq + ret TRAP
  6. for each killed  syscall: jeq + ret KILL_PROCESS
  7. ret ALLOW                                 // default: allow
  8. ret KILL_PROCESS                          // wrong-arch target
```

Uses `jt=0, jf=1` (fall-through on match, skip 1 on miss) so the 8-bit
jt/jf offsets never overflow regardless of set size.

### 3.5 Mount namespace fallback

`unshare(CLONE_NEWNS)` requires `CAP_SYS_ADMIN`, which the twoyi app
process won't have. The skeleton detects the EPERM and falls back to
`chroot` (weaker isolation, but lets the rest of the boot proceed for
testing). VM works around this via `libkrloader64.so` (a custom ELF
interpreter with elevated privileges) — twoyi will need a similar
approach in a follow-up task.

---

## 4. Build & test results

```
$ cd app/rs/kr64
$ cargo build          # 0 warnings, 0 errors
$ cargo test           # 26 passed, 0 failed
$ cargo build --bins   # kr64 binary
$ ./target/debug/kr64 --help
twoyi kr64 — kernel-replacement daemon
...
```

### Test inventory (26 tests)

| Module | Tests | What they verify |
|--------|-------|------------------|
| `lib.rs` (parse_args) | 7 | minimal/full args, missing required args, unknown args, `--help`, default init path |
| `devices.rs` | 3 | `create_qemu_pipe` creates socket file, `create_all_devices` succeeds, marker files created |
| `seccomp.rs` | 6 | BPF filter builds, allowed set has read/write/openat, trapped set has mount/umount2, killed set has ptrace/kexec, `classify()` returns correct `Action` |
| `proc_emu.rs` | 5 | `populate_proc` creates all files, `/proc/version` has `Linux version 4.14.`, `/proc/cmdline` has `androidboot.hardware=twoyi`, `/proc/meminfo` has `MemTotal:`, `/proc/cpuinfo` has N processors |
| `mount_mgr.rs` | 4 | `MountSpec` defaults, `unshare(CLONE_NEWUSER)` works, `list_mounts()` returns non-empty, `pivot_root` wrapper exists |

### End-to-end smoke test (Linux host)

```
$ ./target/debug/kr64 --rootfs /tmp/rfs --data-dir /tmp/data --no-seccomp --no-namespaces
[KR64 INFO] [KR64] starting daemon with config: Config { vmid: 0, ... }
[KR64 INFO] [KR64][devices] bound unix socket: /tmp/rfs/dev/qemu_pipe (fd=3)
[KR64 INFO] [KR64][devices] bound unix socket: /tmp/rfs/dev/input/touch (fd=4)
[KR64 INFO] [KR64][devices] bound unix socket: /tmp/rfs/dev/input/key0 (fd=5)
[KR64 INFO] [KR64][devices] bound unix socket: /tmp/data/dev/event (fd=6)
[KR64 INFO] [KR64][devices] bound unix socket: /tmp/rfs/dev/gb (fd=7)
[KR64 INFO] [KR64][devices] bound unix socket: /tmp/rfs/dev/gb2 (fd=8)
[KR64 INFO] [KR64][proc_emu] populated /tmp/rfs/proc/proc with synthesised files (cpu_count=8, mem_mb=4096)
[KR64 INFO] [KR64] forking guest process
[KR64 INFO] [KR64][parent] guest pid = 31511
[KR64 INFO] [KR64][parent] guest exited with status 1
```

The guest exits with status 1 because (a) `mount_mgr::setup_mounts()`
fails with EPERM (no `CAP_SYS_ADMIN` on the host) and (b) there's no
real `/system/bin/init` to exec. Both are expected on a Linux host
without an Android rootfs.

---

## 5. What's NOT here yet (follow-up tasks)

These items are documented in each module's `// What's NOT here yet`
section and in `GSI_BOOT_PLAN.md`:

1. **Full device inventory** — VM creates 20+ devices (`/dev/vmproc`,
   `/dev/__kmsg__`, `/dev/__properties__`, `/dev/ashmem`, `/dev/socket/*`,
   `/dev/block/vdc`, `/dev/fuse`, netlink sockets, …). Skeleton has 6.
2. **Binder virtualisation** — per-VM `/dev/binder` + Java-side
   `IActivityManager` proxy. Not started.
3. **`/proc` dynamic files** — `/proc/self/maps`, `/proc/self/status`,
   `/proc/<pid>/…`, `/proc/self/exe`, `/proc/self/fd/%d` require
   shadowhook interception of `open`/`openat`. Skeleton uses static
   files only.
4. **Per-syscall emulation** — `seccomp::emulate_syscall()` currently
   returns 0 for all trapped syscalls. Production version needs to
   dispatch `mount`→`mount_mgr::bind_mount()`, `umount2`→unbind, etc.
5. **mknodat-based socket creation** — skeleton uses
   `UnixListener::bind()` (creates the socket file as a side effect).
   VM uses `mknodat(S_IFSOCK)` + `bind()` (requires `CAP_MKNOD`).
   The production version should gate on a capability check.
6. **GSI ROM extractor** — the daemon expects the rootfs to already
   contain `/system`, `/vendor`, etc. The extraction logic (unpacking
   `system.img`, `vendor.img`, … into `<vmDataDir>/fs/`) is a separate
   task.
7. **Workspace integration** — the `kr64` crate is currently
   standalone (not a member of the parent `twoyi` workspace). It
   should be added to a `[workspace]` section in `app/rs/Cargo.toml`
   and built by `build_rs.sh` alongside `libtwoyi.so`.

---

## 6. How to build on Android

```sh
# Add the Android targets if not already installed:
rustup target add aarch64-linux-android x86_64-linux-android

# Build the cdylib (libkr64.so) for arm64:
cargo build --target aarch64-linux-android --release
# → target/aarch64-linux-android/release/libkr64.so

# Verify it's a directly-executable PIE:
file target/aarch64-linux-android/release/libkr64.so
# ELF 64-bit LSB pie executable, ARM aarch64, ...
# interpreter /system/bin/linker64

# Copy to the app's jniLibs:
cp target/aarch64-linux-android/release/libkr64.so \
   ../src/main/jniLibs/arm64-v8a/

# Run on-device:
adb shell
$ /data/data/io.twoyi/.../libkr64.so --rootfs /data/.../fs --data-dir /data/...
```
