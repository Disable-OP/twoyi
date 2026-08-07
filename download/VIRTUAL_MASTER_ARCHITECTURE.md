# Virtual Master Architecture — Complete Reverse Engineering Report

> **Date:** 2026-08-06
> **Source:** Virtual Master 3.2.66 APK (com.clone.android.dual.space)
> **Libraries analyzed:** libvm.so, libkr64.so, libkr64.11.so, libkr64.12.so, libkrloader64.so
> **Tools:** readelf, nm, strings, objdump (aarch64-linux-gnu), XOR brute force scripts

---

## 1. Executive Summary

Virtual Master is **not** a simple app-cloning framework. It is a **full Android emulator engine** repackaged as an in-app virtualization solution. The core library (`libvm.so`, 9.7MB) contains a near-complete copy of the AOSP Android Emulator's rendering pipeline (emugl), networking stack (libslirp), and NativeActivity glue. Guest Android instances are launched as child processes via `vfork+execve`, with the linker hooked by ByteDance's shadowhook v1.0.8 to redirect library loading and create a per-VM virtualized environment.

---

## 2. Library Inventory

| Library | Size | Architecture | Purpose |
|---------|------|-------------|---------|
| **libvm.so** | 9.7 MB | AArch64 | Core virtualization engine (JNI, rendering, networking, process spawn) |
| **libkr64.so** | 1.7 MB | AArch64 | Kernel replacement daemon (Android 7.x) |
| **libkr64.11.so** | 2.3 MB | AArch64 | Kernel replacement daemon (Android 11) |
| **libkr64.12.so** | 2.5 MB | AArch64 | Kernel replacement daemon (Android 12) |
| **libkrloader64.so** | 234 KB | AArch64 | ELF loader (PIE executable disguised as .so) |
| libkr32.so | 1.5 MB | ARM32 | 32-bit kernel replacement (Android 7.x) |
| libkr32.11.so | 2.4 MB | ARM32 | 32-bit kernel replacement (Android 11) |
| libkr32.12.so | 2.4 MB | ARM32 | 32-bit kernel replacement (Android 12) |
| libkrloader32.so | 151 KB | ARM32 | 32-bit ELF loader |
| libadb.so | 116 KB | AArch64 | ADB client |
| libun7z.so | 76 KB | AArch64 | 7-Zip extraction |

---

## 3. Complete Boot Flow

```
┌─── Java App (com.clone.android.dual.space) ───┐
│                                                │
│  VMManager.java                                │
│    └── System.loadLibrary("vm")                │
│         └── JNI_OnLoad() in libvm.so           │
│              └── RegisterNatives() (encrypted)  │
│                                                │
│  Java calls native VM create method            │
│    └── vfork() + execve("libkrloader64.so")    │
│         └── 7 arguments: vmid, data_dir,        │
│             rom_dir, kernel_path, config_path,  │
│             log_level, socket_fd                │
│                                                │
└────────────────────────────────────────────────┘
          │
          ▼ execve
┌─── libkrloader64.so (PIE executable) ──────────┐
│                                                 │
│  .interp = /system/bin/linker64                 │
│  (loaded by system linker normally)             │
│                                                 │
│  _start @ 0x2cd0:                              │
│    1. Embedded static bionic libc init          │
│    2. Read libkr64.so PT_LOAD segments          │
│    3. Apply RELA relocations                    │
│    4. Run .init_array (24 ctors)                │
│    5. Jump to main() @ 0x7244                   │
│                                                 │
└─────────────────────────────────────────────────┘
          │
          ▼
┌─── libkr64.so (.init_array — 24 constructors) ─┐
│                                                 │
│  [0] Decode XOR strings, open /dev/__kmsg__     │
│  [1] memset .bss buffers                        │
│  [2] Install seccomp BPF filter                 │
│      prctl(PR_SET_NO_NEW_PRIVS, 1)              │
│      prctl(PR_SET_SECCOMP, FILTER, &fprog)      │
│  [3] Create virtual devices (mknodat tree)      │
│  [4-5] Resolve own path via /proc/self/exe      │
│  [6] Load configuration                         │
│  [7-8] Setup ashmem emulator                    │
│        dlopen("libcutils.so")                   │
│        dlsym("ashmem_create_region")            │
│  [9] Install shadowhook on linker's do_dlopen   │
│  [10-22] Netlink servers, /proc emulation,      │
│          properties, etc.                       │
│  [24] prctl(PR_SET_PDEATHSIG, SIGKILL)          │
│                                                 │
└─────────────────────────────────────────────────┘
          │
          ▼
┌─── main() @ 0x7244 ────────────────────────────┐
│                                                 │
│  1. Parse argv (expects argc == 7)              │
│  2. Initialize 80-byte config struct            │
│  3. Setup VM handle                             │
│  4. Get VM_MOUNT_NS from environment            │
│  5. Scrub env vars: strip "VM_" prefix          │
│     VM_LD_PRELOAD → LD_PRELOAD                  │
│     VM_MOUNT_NS → (used internally)             │
│  6. fork() + socketpair() for parent-child IPC  │
│                                                 │
│  Child path:                                    │
│    a. prctl(PR_SET_PDEATHSIG, SIGKILL)          │
│    b. Read guest binary path from /proc/exe     │
│    c. Format exec path (app_process64)           │
│    d. execve(path, argv, envp)                   │
│                                                 │
│  Parent path:                                   │
│    a. sem_wait() for child ready signal          │
│    b. Return child PID                           │
│                                                 │
└─────────────────────────────────────────────────┘
          │
          ▼
┌─── Guest: app_process64 (Android runtime) ─────┐
│                                                 │
│  Runs as normal Android process with:            │
│  • Virtualized /dev (binder, qemu_pipe, touch)  │
│  • Virtualized /proc (maps, status, mounts)     │
│  • Per-VM mount namespace                       │
│  • shadowhook intercepting all dlopen calls     │
│  • Seccomp BPF filtering syscalls               │
│  • LD_PRELOAD hook library loaded               │
│                                                 │
│  → Zygote → SystemServer → SurfaceFlinger       │
│  → ActivityManager → PackageManager → Apps      │
│                                                 │
└─────────────────────────────────────────────────┘
```

---

## 4. Virtual Device Creation

All devices created under: `/data/data/com.clone.android.dual.space/vm/vm%d/dev/`

| Device | XOR Key | Purpose |
|--------|---------|---------|
| `/dev/vmproc`, `/dev/vmproc/%d` | 0x47, 0x64, 0x6c | Process emulation interface |
| `/dev/__kmsg__` | 0x03 | Kernel log |
| `/dev/__kmsg2__` | 0x63 | Secondary kernel log |
| `/dev/__krlog__` | 0xa6, 0xc7 | VM-specific log |
| `/dev/__properties__` | 0x27 | Android property area |
| `/dev/socket/process_pid` | 0x55 | Per-process PID socket |
| `/dev/socket/logdw`, `/dev/socket/logdr` | 0xda, 0xdb | Android log sockets |
| `/dev/ashmem` | 0x0a | Shared memory (backed by real ashmem_create_region) |
| `/dev/ashmemsim` | 0x15 | Simulated ashmem |
| `/dev/tmpfs`, `/dev/tmpfs/ns` | 0x33, 0x76, 0xc8 | Tmpfs mount points |
| `/dev/.busybox` | 0xdc | BusyBox marker |
| `/dev/.magisk` | 0x88 | Magisk emulation marker |
| `/dev/.coldboot_done` | 0xc7 | Init coldboot completion |
| `/dev/input`, `/dev/input/touch` | 0x6d, 0x95, 0x9a | Input devices |
| `/dev/qemu_pipe` | 0x7a, 0xba | GL transport (QEMU pipe protocol) |
| `/dev/gb`, `/dev/gb2` | (A11+ only) | Graphics buffer char devices |
| `/dev/goldfish_pipe` | (A11+ only, 0xd5) | Newer GL transport |
| `/dev/block/vdc` | (A11+ only) | Block device |
| `/dev/fuse` | (A11+ only) | FUSE filesystem |
| `/dev/hal/power_supply%s` | (A11+ only) | Power supply HAL |
| `/dev/tun.ctrl_client/tun_%d_%d` | (libvm.so) | VPN tunnel control |
| `/dev/netlink_server` | 0x2c | Main netlink server |
| `/dev/netlink_client/nl_dhcp_%d_%d` | 0x37 | DHCP netlink |
| `/dev/netlink_client/netdevice_%d_%d` | 0x1e | Netdevice events |

**Block device handling:** `mknodat` wrapper at 0x11d680 creates regular files with 8-byte markers for block devices (instead of real device nodes, which would require root).

**Binder:** Created by `libvm.so` (JNI), not by `libkr64.so`. Uses raw `ioctl()` bypassing libbinder C++ wrappers.

---

## 5. Shadowhook v1.0.8 — Linker Hooking

VM uses ByteDance's shadowhook v1.0.8 to inline-hook the Android dynamic linker's `do_dlopen` function. This intercepts every `dlopen()` call made by the guest.

### Hooked linker symbols:
- `__dl__Z9do_dlopenPKciPK17android_dlextinfoPKv` (Android 5-9)
- `__dl__Z9do_dlopenPKciPK17android_dlextinfoPv` (Android 10+)
- `__dl__Z9do_dlopenPKciPK17android_dlextinfo` (older)
- `__dl__ZL10dlopen_extPKciPK17android_dlextinfoPv`
- `__dl__Z8__dlopenPKciPKv`
- `__loader_dlopen`

By hooking all three `do_dlopen` signatures, a single binary supports Android 5 through 13.

### Hook modes:
- **UNIQUE**: Single hook per target (replaces function)
- **SHARED**: Multiple chained hooks (cooperative interception)

### What the hook does:
When the guest calls `dlopen("libfoo.so")`, the hook redirects the load to a VM-controlled library in the VM's data directory. This is how VM injects:
- Per-VM HAL shims
- Property hooks
- Binder proxies
- Service manager substitutes

---

## 6. /proc Emulation

The guest sees a fully virtualized `/proc`:

| Real path | VM-internal template | XOR Key |
|-----------|---------------------|---------|
| `/proc/self/exe` | `%s/proc/exe_%d` | 0x90 |
| `/proc/self/maps` | `%s/proc/maps_%d_%d` | 0xa6 |
| `/proc/self/status` | `%s/proc/status_%d_%d` | 0xbc |
| `/proc/self/mounts` | `%s/proc/mounts_%d_%d` | 0x77 |
| `/proc/self/fd/%d` | passthrough | 0xad |
| `/proc/%d/cmdline` | — | 0x67 |
| `/proc/%d/status` | — | 0x8c |
| `/proc/%d/maps` | — | 0x83 |
| `/proc/%d/mounts` | — | 0x8b |
| `/proc/%d/exe` | — | 0xfe |
| `/proc/%d/fd/%d` | — | 0x82 |
| `/proc/cmdline` | — | 0x21 |
| `/proc/version` | — | 0xa1 |
| `/proc/mounts` | — | 0xca |
| `/proc/net/if_inet6/` | — | 0x25, 0x4b |
| `/sys/class/net` | — | 0x5d, 0xa6 |

---

## 7. Mount Namespace

- Created via `unshare(CLONE_NEWNS)` (syscall 0x61)
- Bind-mounts guest's `/system`, `/vendor`, `/data` from `/fs/system`, `/fs/vendor`, `/fs/data`
- Skips `/dev`, `/mnt`, `/storage` as "special" (VM-specific virtualized versions)
- Bind-loop detection prevents infinite recursion
- Mount flag strings: `ms.bind`, `ms.unbindable`, `ms.remount`, `tmpfs`, `ramfs`

### Mount manager log strings (XOR-decoded):
- `mount_mgr: %s -> %s -> %s` (key 0x1a)
- `mount_mgr: bind loop detected %s` (key 0x55)
- `mount_mgr: propagation %s not supported` (key 0x1d)
- `mount_mgr: /dev is special, skip` (key 0x0c)
- `mount_mgr: /mnt is special, skip` (key 0x21)
- `mount_mgr: /storage is special, skip` (key 0xdb)

---

## 8. Seccomp BPF Filter

Installed in `.init_array[2]` @ 0x111cd8:

1. `prctl(PR_SET_NO_NEW_PRIVS=38, 1, 0, 0, 0)`
2. Build BPF program on stack
3. `prctl(PR_SET_SECCOMP=22, SECCOMP_MODE_FILTER, &fprog)`

BPF opcodes observed:
- `0x20` → BPF_LD|BPF_W|BPF_ABS (load syscall nr)
- `0x15` → BPF_JMP|BPF_JEQ|BPF_K (jump-if-equal)
- `0x35` → BPF_JMP|BPF_JGE|BPF_K (jump-if-greater-equal)
- `0x25` → BPF_JMP|BPF_JGT|BPF_K (jump-if-greater)
- `0x06 0x00 0xff 0x7f` → BPF_RET SECCOMP_RET_ALLOW

SIGSYS handler catches forbidden syscalls and either emulates them (redirects paths) or kills the guest.

---

## 9. Environment Variables

| Variable | XOR Key | Purpose |
|----------|---------|---------|
| `VM_LD_PRELOAD` | 0x2b | Hook library path (stripped to `LD_PRELOAD` before exec) |
| `VM_MOUNT_NS=%d` | 0x4d | Mount namespace ID |
| `VM_%s` | 0xb3 | Generic VM env var prefix |
| `VMINIT_PID=%d` | 0x3e | Init's PID |

The `VM_` prefix stripping at `main()` @ 0x70b0 is a clever trick:
1. Pass `VM_LD_PRELOAD=/path/to/hook.so` to the daemon
2. Daemon ignores it (not a standard `LD_PRELOAD`)
3. Before `execve()`, strip `VM_` prefix → `LD_PRELOAD=/path/to/hook.so`
4. Guest's bionic linker sees standard `LD_PRELOAD` and loads the hook

---

## 10. Rendering Pipeline (libvm.so)

libvm.so contains a near-complete copy of AOSP's emugl rendering pipeline:

### Components:
- `initOpenGLRenderer` / `stopOpenGLRenderer` — renderer lifecycle
- `createOpenGLSubwindow` / `destroyOpenGLSubwindow` — window management
- `setupSubWindow` / `removeSubWindow` — subwindow configuration
- `setWindowSurfaceColorBuffer` — ColorBuffer ↔ ANativeWindow binding
- `setPostCallback` — frame completion callback
- `TextureDraw` — rotation/batching helper
- `init_gles1_dispatch` / `init_gles2_dispatch` — GL dispatch tables
- `GEGLA` — emugl address_space device magic

### Pipeline:
1. Java hands a `Surface` to native code
2. `ANativeWindow_fromSurface()` gets the host window
3. `createOpenGLSubwindow()` binds a guest ColorBuffer to it
4. Guest GPU commands render into a ColorBuffer
5. `setWindowSurfaceColorBuffer()` + `setPostCallback()` blit ColorBuffer → host window
6. GLES1 + GLES2 supported (no Vulkan)

### Embedded GLSL shaders:
- Vertex stage: rotation matrix
- Fragment stage: `texture2D` sample

---

## 11. Android Version Support

### Version detection:
- `ro.build.version.sdk` (property)
- `/system/build.prop` (file)
- Java app picks which `libkr64.NN.so` variant to exec

### Android 7 (libkr64.so, 1.7MB):
- Base feature set
- 165 PLT imports
- No APEX/VNDK-SP support

### Android 11 (libkr64.11.so, 2.3MB):
- Adds: `/dev/gb`, `/dev/gb2`, `/dev/goldfish_pipe`
- Adds: APEX paths (`/apex/com.android.vndk.v*`)
- Adds: SOCKS5 proxy with IPv4/IPv6
- Adds: Samsung GameSDK integration
- Adds: Magisk emulation (`/sbin/.magisk/su_request_`)
- Adds: `libbinder.so` + `libutils.so` as DT_NEEDED
- Adds: `opendir`/`readdir`/`wait4`/`timerfd_create`

### Android 12 (libkr64.12.so, 2.5MB):
- Adds: ODM path coverage (`/odm/lib64/`, `/odm/lib64/vndk-sp/`)
- Adds: `EXECVE` hook mode (intercepts execve of app_process64)
- Adds: Samsung GLESv2 driver workaround
- Adds: `vminit` bootstrap binary
- Removes: `/dev/socket/logdr` (restricted on Android 12)
- +230KB of additional shadowhook trampoline capacity

---

## 12. Multi-VM Support

All paths are templated with `%d` (vmid):
```
/data/data/com.clone.android.dual.space/vm/vm%d/dev/touch
/data/data/com.clone.android.dual.space/vm/vm%d/dev/qemu_pipe
/data/data/com.clone.android.dual.space/vm/vm%d/dev/netlink_server
/data/data/com.clone.android.dual.space/vm/vm%d/dev/netlink_client/nl_dhcp_%d_%d
/data/data/com.clone.android.dual.space/vm/vm%d/dev/netlink_client/netdevice_%d_%d
/data/data/com.clone.android.dual.space/vm/vm%d/dev/input/touch
/data/data/com.clone.android.dual.space/vm/vm%d/dev/gb
/data/data/com.clone.android.dual.space/vm/vm%d/dev/gb2
/data/data/com.clone.android.dual.space/vm/vm%d/dev/tun.ctrl_client/tun_%d_%d
/data/data/com.clone.android.dual.space/vm/vm%d/fs
```

Each VM gets:
- Own device tree under `vm/vmN/dev/`
- Own rootfs under `vm/vmN/fs/`
- Own mount namespace
- Own netlink socket set (3 unix sockets)
- Own /proc virtualization tree

---

## 13. Networking (libvm.so)

libvm.so contains:
- **libslirp** TCP/IP stack (tcp/udp/icmp/ip input/output)
- **GPS mock** (`qon_gps_nmea_changed`)
- **VPN service integration**
- **TUN device** support (`/dev/tun.ctrl_client/tun_%d_%d`)
- **SOCKS5 proxy** (Android 11+, IPv4/IPv6 dual-stack)

---

## 14. Key Insights for Twoyi

Based on this analysis, twoyi can improve by:

1. **Adopt shadowhook-style linker hooking** instead of (or in addition to) the current rootfs linker approach. This would allow intercepting dlopen calls to redirect library loading.

2. **Implement the VM_LD_PRELOAD trick** — pass hook library paths with a VM_ prefix, strip before exec. This avoids the daemon loading the hook itself.

3. **Add Magisk emulation** — create `/dev/.magisk` and `/dev/.busybox` markers so root-aware apps work.

4. **Add SOCKS5 proxy support** for network virtualization.

5. **Support ODM paths** for Android 12 compatibility.

6. **Use the emugl ColorBuffer model** for rendering (libvm.so's approach is more complete than twoyi's current stub).

7. **Implement EXECVE hook mode** to intercept guest process spawning.

8. **Add Samsung GameSDK compatibility** for Samsung devices.

---

## 15. XOR String Decoding

VM uses per-string XOR keys (80+ distinct keys found). Each string in `.data` is XOR'd with a unique byte. The brute-force script `xor_scan_text.py` tries all 256 keys and reports matches against known patterns.

### Key examples:
- Key 0x03: `/data/data/com.clone.android.dual.space/vm/vm%d/dev/touch`
- Key 0x15: `/data/data/com.clone.android.dual.space`
- Key 0x1e: `/data/data/.../vm/vm%d/dev/netlink_client/netdevice_%d_%d`
- Key 0x2b: `VM_LD_PRELOAD`
- Key 0x4d: `VM_MOUNT_NS=%d`
- Key 0x53: `libcutils.so`
- Key 0xb0: `ashmem_create_region`

---

*This document is the result of static analysis only. No dynamic instrumentation was performed. All findings are derived from readelf, nm, strings, and XOR brute-force output on the supplied .so files.*
