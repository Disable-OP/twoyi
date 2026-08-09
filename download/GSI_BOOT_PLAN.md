# Twoyi — GSI Boot Plan (Task GSI-BOOT-1)

> **Author:** general-purpose sub-agent
> **Date:** 2026-08-05
> **Codespace:** `twoyi-dev-3-jr47xg6xvx7ghq6p`
> **Inputs:** `VM_JAVA_ANALYSIS.md`, `VM_KR64_ANALYSIS.md`, `VM_ROM_ANALYSIS.md`, `ARCHITECTURE.md`
> **Goal:** Define a concrete, file-and-function-level implementation plan for booting an Android Treble GSI inside twoyi on x86_64 (with KVM available in the codespace and the AOSP-built `libOpenglRender_aosp_x86_64.so` in hand).

---

## 0. Executive summary (read this first)

Virtual Master proves that **a Treble GSI can be booted in an unprivileged Android app process**, without KVM and without root, by replacing the kernel's role with a userspace daemon (`libkr64.so`) plus an in-process binder proxy (`libvm.so` + `BinderService.java`). Twoyi already does about 30 % of this work — it has the in-process loader, the open-source `libOpenglRender.so`, an input subsystem, and a socket IPC. What it is missing is:

1. A **kernel-replacement daemon** that materialises a virtual `/dev` tree (binder, qemu_pipe, gb, gb2, vmproc, ashmem, __properties__, etc.).
2. **Binder virtualisation** — a per-VM `/vm%d/dev/binder` plus a Java-side `IActivityManager` proxy that re-routes the guest's `servicemanager` lookups back into the host app.
3. A **seccomp filter with a SIGSYS emulation handler** (this is what makes "blocked" syscalls become no-ops or fake-success instead of killing the guest).
4. A **`/proc` emulator** that synthesises `/proc/self/maps`, `/proc/self/status`, `/proc/cmdline`, `/proc/version`, `/proc/mounts`, … per VM.
5. A **GSI ROM extractor** that knows how to unpack `system.img`, `vendor.img`, `product.img`, `system_ext.img`, `boot.img`-derived ramdisk into the `<vmDataDir>/fs/` tree (not just unzip a flat folder like today).
6. **Init configuration** — patch the guest's `init.rc` / `init.{vendor,product}.rc` so it talks to the virtual devices instead of real hardware HALs.

For **x86_64**, all of the above still applies, but additionally:
- The guest GSI must be an **x86_64** GSI (`system-x86_64.img`, `product-x86_64.img`, …) — an ARM GSI will not run natively on x86_64 (and twoyi's container model is *not* an emulator — it shares the host kernel).
- `libOpenglRender_aosp_x86_64.so` is already built and present at `/home/z/my-project/app/src/main/jniLibs/x86_64/libOpenglRender_aosp.so` (597 KB, all 6 twoyi ABI symbols exported, see `AOSP_BUILD_RESULTS.md`).
- If KVM is available (codespace `twoyi-dev-3-jr47xg6xvx7ghq6p` has working KVM per `TWOYI_HONEST_STATUS.md`), we *could* alternatively boot the GSI in a true VM via crosvm/QEMU. That is **out of scope** for the container path; it is listed in §6 as a future option.

The plan below targets the **container path** (no KVM, shares the host kernel) because that is the architectural direction twoyi is already on and is what `libkr64.so` does. The KVM path is a separate project.

---

## 1. What is a GSI?

### 1.1 Definition

A **GSI (Generic System Image)** is an Android `system.img` that conforms to the Treble HAL interface contract. It was introduced in Android 8.0 (Project Treble) so that the **system** partition can be updated independently of the **vendor** partition. The same GSI boots on any device whose `vendor` partition implements the matching VINTF manifest.

* Source: https://source.android.com/docs/core/architecture/halse
* Source: https://source.android.com/docs/core/ota/gsi

### 1.2 Differences from a pre-Treble `system.img`

| Aspect | Pre-Treble (≤ Android 7.1) | Treble GSI (≥ Android 8.0) |
|---|---|---|
| `system.img` contents | All framework + most HALs jammed together | Framework + generic HALs only; vendor-specific HALs live in `vendor.img` |
| `vendor.img` | Optional, often empty | Required, contains all SoC-specific HALs |
| HAL interface discovery | Hard-coded class lookups | **VINTF manifest** at `/vendor/etc/vintf/manifest/*.xml` (XML manifest of HALs and versions) |
| `product.img` | Did not exist | Optional (Treble) / required (Android 10+) — contains product-specific apps/overlays |
| `system_ext.img` | Did not exist | Required (Android 10+) — system extensions that are product-agnostic but vendor-specific |
| `odm.img` | Did not exist | Optional — original-design-manufacturer partition |
| Boot flow | `boot` → mount `system` → `init` reads `/system/**/init.rc` | `boot` → mount `system` + `vendor` + `product` + `system_ext` → `init` reads `/system/etc/init/hw/init.rc` AND `/vendor/etc/init/*.rc` |
| Binder namespace | Single binder driver `/dev/binder` | Three binder contexts: `/dev/binder` (framework), `/dev/hwbinder` (hwbinder HALs), `/dev/vndbinder` (vendor binder) |
| APEX | Did not exist | Required (Android 10+) — `/system/apex/com.android.*` are mountable mini-images |
| Properties | `/system/build.prop` + `/vendor/build.prop` | Adds `/system/product/build.prop`, `/system/system_ext/build.prop`, `/vendor/etc/prop/default.prop` |

### 1.3 Minimum requirements to boot one

To boot a Treble GSI (in any environment — bare metal, VM, or container) you must provide:

1. **Kernel** — must match the GSI's kernel ABI. For Android 11 GSIs that is typically Linux 4.14+ with selinux, binder, ashmem, and the Android-specific ioctl extensions enabled. In a container, you reuse the host kernel.
2. **`/dev/binder`** — the binder driver. Treble additionally requires `/dev/hwbinder` and `/dev/vndbinder`. In a container you must either proxy these (VM's approach: per-VM `/vm%d/dev/binder`) or share the host's.
3. **`/dev/ashmem`** (Android ≤ 10) or **`/dev/dm-user`** + memfd (Android 11+) — shared memory for SurfaceFlinger and binder transactions.
4. **`/dev/__properties__`** — the property area file (mmap'd read-only by every process). The `init` process writes here; everyone else reads.
5. **`init`** binary at `/system/bin/init` — parses `/system/etc/init/hw/init.rc` and starts `servicemanager`, `surfaceflinger`, `zygote`, etc.
6. **`servicemanager`** at `/system/bin/servicemanager` — registers framework services in the binder namespace.
7. **`surfaceflinger`** + a gralloc HAL — without a gralloc that produces buffers SurfaceFlinger can publish, you get no display.
8. **HALs declared in `/vendor/etc/vintf/manifest/*.xml`** — every HAL listed must be either implemented or stubbed. A missing HAL causes `init` to fail to start its dependent services.
9. **Mount points** — `/system`, `/vendor`, `/product`, `/system_ext`, `/apex/*`, `/data`, `/cache`, `/dev`, `/proc`, `/sys`.
10. **`/proc` and `/sys`** — Android expects these to look like real Linux procfs/sysfs (e.g. `/proc/cmdline`, `/proc/mounts`, `/proc/self/maps`, `/sys/class/net/*`).

### 1.4 What the GSI doesn't include (and you must supply)

A GSI ships **only** `system.img`, `product.img`, `system_ext.img` (and sometimes a `boot.img` containing the kernel + ramdisk). It does **not** ship:

- `vendor.img` — must come from the device (or in our case, be a synthetic stub)
- `boot.img` kernel — must be the host kernel
- `/data` partition — must be created empty by the boot environment
- `/cache` — same

For twoyi's container path, we will:
- Use the host kernel.
- Use a synthetic `vendor.img` with stub HALs (a pre-built one is fine — see §5.7).
- Use the host's `/dev/binder` for *some* services and per-VM proxy binder for *others* (see §3.2 for the split).

---

## 2. How Virtual Master boots GSIs (from our analysis)

This section summarises what the three prior analysis reports (`VM_JAVA_ANALYSIS.md`, `VM_KR64_ANALYSIS.md`, `VM_ROM_ANALYSIS.md`) established about VM's GSI boot. Every fact below is traceable to one of those reports.

### 2.1 The startup pipeline (Java side)

From `VM_JAVA_ANALYSIS.md` §2.3 and §6, the boot sequence is driven by an explicit state machine in `com.android.vmcore.VMInstance.f8940WWoWWo` with 11 states (`-5..7`):

```
state 0 (STOPPED) → 1 (CHECKING_ENV)
  ├─ CPU/SDK/data-dir checks
  └─ clearZombieProcess loop
state 1 → 2 (INSTALLING)  [only if ROM changed or first boot]
  Setup pipeline (sequential, each task returns bool):
    PrepareFsTask  → chmod the fs dir
    InstallFsTask  → download + decrypt + extract rom.zip (see §2.2)
    FixFsTask      → fix fs paths/symlinks (ROM-version specific)
    CleanFsTask    → remove stale cache
    ChmodFsTask    → NativeHelper.chmodRecursively(fsDir, 0xA1FF)
    CleanCacheTask → clear caches
    FixCPUArchTask → rewrite /system/bin/app_process{32,64}_xposed shims
    LoadVMPropTask → parse /system/build.prop into VMConfig.f8870 HashMap
state 2 → 3 (STARTING_SVC)
  Start HAL services:
    BinderService  → reflect IActivityManager, install Java Proxy, call
                     native setupBinder(vmId, binderVer, 1, 2,
                       "com.android.vmcore.service.IBinderService",
                       parcelledIntent)
                     → creates /vm%d/dev/binder in libvm.so
                     → bindService(BinderService.class) — wait up to 5 s
    InputService   → nativeStartService   (creates /dev/input/touch)
    AudioService   → start                 (creates /dev/audio)
    HALManager     → nativeStartHALMgr     (creates camera/sensor/etc. dev nodes)
    DisplayService → nativeStartService    (opens /dev/qemu_pipe)
    NetlinkManager → start                 (creates /dev/netlink_client/*)
    VMEventManager → new Thread() { LocalServerSocket("<vmDataDir>/dev/event"); accept loop }
state 3 → 4 (BOOTING)
  Startup pipeline (sequential):
    ApplyOverlaysTask  → copy /system/product overlay files
    Bug1..Bug8FixTask  → ROM-version-specific patches
    CleanLogTask       → clear logs
    SuperuserTask      → extract superuser.zip
    XposedTask         → extract xposed.zip
    GooglePlayTask     → extract play.zip (GApps)
    MagiskTask         → extract magisk.zip, patch init.rc
    BuildTmpfsTask     → mount tmpfs on /tmp, /dev, etc. (via native)
    BuildVMPropTask    → write /system/build.prop (PIE/build fingerprint/Build.ID)
    BuildExecPathTask  → set PATH and exec dirs
state 4 → 5 (OS_BOOTING)
  int pid = startOS(vmId, dpi, kernelPath);   // JNI → libvm.so
    kernelPath = dataDir + "/lib64"
    libvm.so forks a child process, chroots into <vmDataDir>/fs,
    LD_PRELOADs libkr64.so as a "kernel replacement", and exec's
    /system/bin/init from the guest ROM.
state 5 → 6 (OS_READY_1)
  Guest eventually calls back through the /dev/event socket with:
    "com.android.vmcore.action.BOOT_COMPLETED`<payload>"
  → VMInstance.mo5013WWWWWWWW(...) sets state = 6.
state 6 → 7 (OS_READY_2)
  Guest signals SHUTDOWN on /dev/event → state = 7.
```

### 2.2 ROM extraction

From `VM_ROM_ANALYSIS.md` §4 and `VM_JAVA_ANALYSIS.md` §4.3, the ROM is downloaded as `rom.zip` (one file per `rom_uri` mirror, parallel download, on-the-fly AES-128-ECB decryption with key `%z89aviCM0KkbEs9`, then extracted via ZIP or 7-Zip into `<vmDataDir>/fs/`). After extraction, the dex code references these specific Treble paths that we now know exist inside `rom.zip`:

```
/system/build.prop
/system/etc/prop.default
/system/etc/init/hw/init.rc
/system/product/build.prop           ← Treble product partition
/system/system_ext/build.prop        ← Android 10+ system_ext partition
/vendor/build.prop
/vendor/etc/vintf/manifest/vibrator-default.xml      ← VINTF HAL manifest
/vendor/etc/init/vibrator-default.rc                  ← vendor init .rc
/vendor/bin/hw/android.hardware.vibrator-service.example   ← vendor HAL binary
```

The presence of these paths in the *runtime* dex confirms the newer ROMs (9.0, 11.0) are real Treble GSIs (with the partitions flattened into the extracted directory tree — i.e. `/system/product/...` rather than a separate `product.img` mount).

### 2.3 How `libkr64.11.so` sets up the environment

From `VM_KR64_ANALYSIS.md`:

- `libkr64.11.so` is **not a JNI library**. It is a standalone ELF executable disguised as a `.so`, with `.interp = libkrloader64.so` (a custom dynamic linker built from AOSP source for product `marlin`/Pixel XL, embedding a static bionic libc).
- Launched by the kernel via `fork`+`exec`. Entry point `0x4e04` (A 7) / `0x5594` (A 11) → `_start` → `__libc_init` → 24 `.init_array` constructors → `main()` at `0x7244` (A 7).
- `main()` parses `argv` (expects 7 args: `vmid`, `data_dir`, `rom_dir`, `kernel_path`, `config_path`, `log_level`, `socket_fd` — best inference from `VM_KR64_ANALYSIS.md` §2 and Action 2).
- Builds a config struct (80 bytes), calls `0x115a90()` to get a handle, then dispatches.
- `libkr64.11.so` is 2.03 MB (35 % larger than A 7's 1.50 MB) and links against **`libbinder.so` + `libutils.so`** (but doesn't actually use any binder symbols — the deps are loaded only so libkr64.11.so can hook or dlsym them).
- Has 213 undefined symbols (vs. 187 in A 7) and uses 100 direct `syscall()` calls to bypass its own shadowhook hooks.

### 2.4 Virtual devices that libkr64.11.so creates

The complete device inventory (decoded from XOR-obfuscated strings in `.data`, see `VM_KR64_ANALYSIS.md` §4.2 and §6 — the single `mknodat()` call site is at `0x11d770`):

| Path | Type | Created by | Purpose |
|---|---|---|---|
| `/dev/binder` | socket | `libvm.so` (NOT libkr64) | Per-VM virtual binder (proxy to host servicemanager) |
| `/dev/event` | socket | `libvm.so` | Event IPC: `LocalServerSocket` in `VMEventManager.java` |
| `…/vm/vm%d/dev/qemu_pipe` | socket (mknodat S_IFSOCK) | libkr64 | GL command transport — SurfaceFlinger writes GL here |
| `/dev/goldfish_pipe` (A 11) | socket | libkr64 | Same as above, alternate name |
| `…/vm/vm%d/dev/gb` | char dev (mknodat S_IFCHR) | libkr64 (A 11 only) | Graphics buffer device — `gralloc`-like ioctl interface |
| `…/vm/vm%d/dev/gb2` | char dev | libkr64 (A 11 only) | Second graphics buffer device (probably for hwbinder gralloc) |
| `…/vm/vm%d/dev/touch` | char dev | libkr64 | Touch input — guest's `EventHub` reads `EV_ABS` events |
| `/dev/input` + `/dev/input/touch` | dir + char dev | libkr64 (A 11) | Alternative input path for A 11 init.rc |
| `/dev/vmproc` | socket | libkr64 | `/proc` emulator entry point — `open("/proc/…")` redirected here |
| `/dev/__kmsg__` | socket | libkr64 | Kernel log buffer (init writes boot messages here) |
| `/dev/__kmsg2__` | socket | libkr64 | Newer kmsg variant |
| `/dev/__krlog__` | socket | libkr64 | libkr64's own log |
| `/dev/__properties__` | socket | libkr64 | Property area file (init writes, everyone reads) |
| `/dev/ashmem` | socket | libkr64 | Shared memory region for SurfaceFlinger buffers |
| `/dev/ashmemsim` | socket | libkr64 | Simulated ashmem (fallback path) |
| `/dev/tmpfs` + `/dev/tmpfs/ns` | dir | libkr64 | Mount-namespace tmpfs |
| `/dev/socket/process_pid` | socket | libkr64 | PID socket for process tracking |
| `/dev/socket/logdw` | socket | libkr64 | logd write socket |
| `/dev/socket/logdr` | socket | libkr64 | logd read socket |
| `/dev/block/vdc` (A 11) | socket | libkr64 | Virtual block device controller |
| `/dev/fuse` (A 11) | socket | libkr64 | FUSE filesystem for /storage emulation |
| `/dev/hal/power_supply%s` (A 11) | socket | libkr64 | HAL proxy for battery info |
| `/dev/.busybox` | file | libkr64 (openat) | Marker that busybox was installed |
| `/dev/.coldboot_done` | file | libkr64 (openat) | Marker that coldboot is complete (init waits on this) |
| `…/vm/vm%d/dev/netlink_server` | socket (bind) | libkr64 | Netlink emulation — guest's RTNETLINK goes here |
| `…/vm/vm%d/dev/netlink_client/nl_dhcp_%d_%d` | socket (bind) | libkr64 | Per-thread DHCP client socket |
| `…/vm/vm%d/dev/netlink_client/netdevice_%d_%d` | socket (bind) | libkr64 | Per-thread netdevice event socket |

The `bind()` clusters are at `0x134328`, `0x1381d0`, `0x1387f8` (each appears twice due to OLLVM control-flow flattening — see `VM_KR64_ANALYSIS.md` §5).

**Binder virtualisation is NOT in libkr64.so** — it's in `libvm.so`. The three `bind()` clusters in libkr64.so are all for netlink emulation. `libvm.so` creates `/vm%d/dev/binder` via the `setupBinder()` JNI called from `BinderService.m5206WWWWoWWWWo(vMApp, vmId)` (see `VM_JAVA_ANALYSIS.md` §5.2).

### 2.5 Binder virtualisation

From `VM_JAVA_ANALYSIS.md` §5.2 and `VM_KR64_ANALYSIS.md` §16:

The split is:
- **`libvm.so`** (loaded into VM Java app process): binder virtualisation via a Java `Proxy` of `android.app.IActivityManager`, OpenGL render server, JNI bindings.
- **`libkr64.so`** (separate daemon process via `libkrloader64.so`): everything else (proc, dev, mounts, netlink, seccomp, shadowhook).

The Java-side setup (`com.android.vmcore.service.BinderService.m5206WWWWoWWWWo`) does:

1. **Reflect into `ActivityManager`** to obtain the system `IBinder` for `android.app.IActivityManager` (hidden-API access enabled by FreeReflection's `exemptAll()` in `VMApp.attachBaseContext`).
2. **Wrap that IBinder with a `java.lang.reflect.Proxy`** whose `transact()` invocation captures the calling thread ID and the return code into `iArr[0]`.
3. **Reinstall the proxy** via reflection on `ActivityManager.IActivityTaskManager` (replacing the system's `IActivityTaskManager` field with the proxied one).
4. **Trigger the proxy** by calling `peekService(vMApp, new Intent())`. The captured integer is the system's binder version.
5. **Call native `setupBinder(vmId, binderVersion, 1, 2, "com.android.vmcore.service.IBinderService", parcelledIntent)`**. This is the JNI into `libvm.so` that:
   - Creates the per-VM `/vm%d/dev/binder` device.
   - Sets up a binder-redirect mapping so the guest's `servicemanager` calls for `activity`/`package`/`window`/etc. are proxied back to the host's `BinderService.f9244WWWWoWWWWo` (the `IBinderService.Stub` instance).
   - The host then fulfills those calls (or passes them through to the real system service with the host's identity).
6. **`bindService(Intent(BinderService.class), …, BIND_AUTO_CREATE)`** — wait up to 5 s for `onServiceConnected`.

So the guest's `Context.startActivity()` ends up calling host `VMDisplayActivity.onDialNumberEvent()`, etc. The guest's `PackageManager` queries are routed to a per-VM package list maintained by the host.

### 2.6 APEX support

From `VM_KR64_ANALYSIS.md` §14:

`libkr64.11.so` references these APEX paths (decoded from XOR-obfuscated `.data`):
- `/system/apex/com.android.vndk.current` — VNDK APEX (Android 10+)
- `/system/apex/com.android.art.release` — ART runtime APEX

An APEX file is a ZIP with an `apex_payload.img` (ext4 or sparse-ext4) inside. The guest's `init` mounts each APEX via the `apexd` daemon, which loops-mounts `apex_payload.img` onto `/apex/com.android.<name>/`. The libkr64 daemon supports this because:
1. It runs as a separate process with `loop` mount permissions (or it emulates loop mounts via bind mounts).
2. It can hook `mount()` (via shadowhook) so when `apexd` calls `mount("/dev/block/loopN", "/apex/…", "ext4", …)`, libkr64 redirects it to a bind mount of the pre-extracted APEX directory.

For a minimal viable GSI boot we can **skip APEX** by pre-extracting all APEXes into the `fs/system/apex/<name>/` directory and patching `apexd`'s init.rc to be a no-op. This is what `Bug8FixTask` (in `com.android.vmcore.startup.Bug8FixTask`) likely does — see §2.7 below.

### 2.7 Seccomp filter

From `VM_KR64_ANALYSIS.md` §12 and §11:

`libkr64.so` installs a **seccomp filter** on the guest process. Strings decoded from `.data`:
- `INIT.SECCOMP` (key 0xb4)
- `init_seccomp` (key 0x67, A 11 only)
- `__NR_rt_sigaction SIGSYS` (key 0x1a, A 11 only)
- `BLOCKED.SYSCALL.FAILED` (key 0xc9)
- `blocked syscall failed %d` (key 0xc2 / 0xe2)

Mechanism:
1. `libkr64.so` calls `prctl(PR_SET_NO_NEW_PRIVS, 1)` (4 paired `prctl` calls at `0x111e28`/`0x112694`/`0x11270c`/`0x112f78`).
2. `libkr64.so` installs a SIGSYS handler via `sigaction(SIGSYS, …)` (the `__NR_rt_sigaction SIGSYS` string is the giveaway).
3. `libkr64.so` installs a `SECCOMP_SET_MODE_FILTER` (syscall 277 on aarch64) BPF program via the `syscall()` wrapper.
4. The BPF program traps forbidden syscalls (the ones that would let the guest escape its sandbox — e.g. `mount` without our hook, `chroot`, `pivot_root`, `reboot`, raw `swapon`/`swapoff`, `acct`, etc.).
5. When a forbidden syscall is hit, the kernel delivers SIGSYS to the guest.
6. The SIGSYS handler logs `BLOCKED.SYSCALL.FAILED: <syscall_nr>` and either:
   - **Emulates** the syscall (returns 0 or a fake-success value), OR
   - **Kills** the guest (via `_exit`).

This is the core of VM's "kernel replacement" pattern: the seccomp filter turns syscalls into traps that the userspace daemon can intercept and synthesise, mimicking the kernel's behaviour for a controlled subset of syscalls.

### 2.8 `/proc` emulation

From `VM_KR64_ANALYSIS.md` §4.2 and §16:

`libkr64.so` emulates `/proc` by intercepting `open("/proc/…")` calls (via shadowhook on `open`/`openat`). When the path matches one of these patterns, libkr64 synthesises the content:

| Decoded path | Synthesised content |
|---|---|
| `/proc/self/exe` | Resolves to the guest's `/system/bin/init` (not the host's `app_process64`) |
| `/proc/self/maps` | Per-VM map (filtered to show only guest mappings, not host's) — uses `/proc/maps_%d_%d` template (vmid, pid) |
| `/proc/self/status` | Per-VM status — template `/proc/status_%d_%d` |
| `/proc/self/mounts` | Per-VM mounts (only guest mounts, not host's) — template `/proc/mounts_%d_%d` |
| `/proc/self/fd/%d` | Resolves to the per-VM view of FDs |
| `/proc/cmdline` | Synthesised as `androidboot.hardware=… androidboot.bootdevice=…` (matches the GSI's expected cmdline) |
| `/proc/version` | Synthesised as `Linux version 4.14.x …` (matches the GSI's expected kernel version) |
| `/proc/mounts` | Alias for `/proc/self/mounts` |
| `/proc/net/if_inet6/` | Synthesised based on `NetlinkManager`'s virtual interfaces |
| `/proc/sys/kernel/kptr_restrict` | Returns `1` (A 11 hardening) |
| `/proc/sys/vm/mmap_rnd_bits` | Returns `16` (A 11 hardening) |
| `/proc/%d/%s` | Per-VM view of any process's `/proc/<pid>/<file>` |
| `/proc/exe_%d` | Per-VM `exe` symlink for PID `<d>` |
| `/proc/mnt_points` | Per-VM mount point list |

The `/dev/vmproc` device (created via `mknodat` at `0x11d770`, decoded with keys 0x47/0x64/0x07) is the entry point — `open("/proc/…")` is redirected to `open("/dev/vmproc")` + an ioctl that selects which virtual file to read.

### 2.9 Init configuration patches

From `VM_JAVA_ANALYSIS.md` §2.3 (startup pipeline) and `VM_ROM_ANALYSIS.md` §4.3:

Eight `BugNFixTask` classes (`com.android.vmcore.startup.Bug1FixTask` … `Bug8FixTask`) apply ROM-version-specific patches to the extracted filesystem before boot. The pipeline (state 4) is:

```
ApplyOverlaysTask → Bug1FixTask → … → Bug8FixTask → CleanLogTask →
SuperuserTask → XposedTask → GooglePlayTask → MagiskTask →
BuildTmpfsTask → BuildVMPropTask → BuildExecPathTask
```

Notable patches inferred from path constants decoded in `VM_ROM_ANALYSIS.md` §4:
- **`app_process32`/`app_process64` shims** — rewrite the symlinks at `/system/bin/app_process32`, `/system/bin/app_process32_xposed`, `/system/bin/app_process64`, `/system/bin/app_process64_xposed` so they point at the host's `app_process` binary (avoiding the need to ship a full ART runtime).
- **`libui.so` ABI selection** — replace `/system/lib/libui.so` with `libui10.so` (Android 10 variant), `libui51.so` (Android 5.1), or `libhostlibui.so`/`libhostlibui_10.so` (the host-aware shim) depending on ROM version.
- **`build.prop` patching** — `BuildVMPropTask` writes `/system/build.prop` with the host's actual build fingerprint, `Build.ID`, and a synthetic kernel version. This makes `Build.FINGERPRINT` inside the guest match the host (required by SafetyNet / Play Integrity basic attestation).
- **`init.rc` patching** — `MagiskTask` injects magisk service entries into `/system/etc/init/hw/init.rc`. The same mechanism is presumably used to inject the `libkr64.so` LD_PRELOAD.
- **`PATH` and exec dirs** — `BuildExecPathTask` sets the guest's `PATH` to include `/system/bin:/system/xbin:/system/sbin:/vendor/bin:/system/apex/com.android.runtime/bin`.

---

## 3. What twoyi needs to boot GSIs (concrete implementation plan)

This is the actionable section. Each subsection has: (a) what VM does, (b) what twoyi currently does, (c) what to build, (d) which files to create or modify, (e) acceptance criteria.

### 3.1 Kernel replacement daemon (the big one)

**What VM does.** `libkr64.11.so` (2.0 MB) runs as a separate process via `libkrloader64.so` and creates 20+ virtual devices via `mknodat` + `bind` (see §2.4).

**What twoyi currently does.** No kernel replacement. Touch input is `app/rs/src/input.rs` — creates `/dev/input/touch` and `/dev/input/key0` as plain AF_UNIX sockets in the rootfs. The OpenGL transport is in `app/rs/openglrenderer/src/pipe.rs` — creates `/dev/qemu_pipe` but only as a server socket. There is no `mknodat` of char/block devices.

**What to build.** A new Rust crate `app/rs/kr64/` that produces a PIE executable `libkr64.so` (mirroring VM's name). It will:
1. Be launched as a separate process by `core.rs` (currently `core.rs` does `Command::new("./init").spawn()` — change to `Command::new("./libkr64.so").arg(vmId).arg(dataDir).arg(romDir).arg(kernelPath).arg(configPath).arg(logLevel).arg(socketFd).spawn()` first, then have `libkr64.so` itself fork+exec `./init` with `LD_PRELOAD=libkr64.so`).
2. Use the existing PIE pattern from `app/rs/src/interp.c` and `app/rs/.cargo/config.toml` — copy them into `app/rs/kr64/`.
3. Create all 20+ devices listed in §2.4 table.
4. Install the seccomp filter (§3.4).
5. Install the `/proc` emulator (§3.5).
6. Install the shadowhook-equivalent (§3.6) — at minimum hook `open`/`openat` and `mount`.

**Files to create.**
- `app/rs/kr64/Cargo.toml` — crate manifest, output type `cdylib`, name `kr64`.
- `app/rs/kr64/build.rs` — compile `interp.c` (copy from `app/rs/src/`).
- `app/rs/kr64/interp.c` — copy from `app/rs/src/interp.c`, change `.interp` to `./libloader.so` (we already have an open-source loader).
- `app/rs/kr64/.cargo/config.toml` — PIE + dynamic linker flags, copy from `app/rs/.cargo/config.toml`.
- `app/rs/kr64/src/main.rs` — entry point, argv parsing (7 args), config struct.
- `app/rs/kr64/src/devices.rs` — `mknodat` wrapper + the device-creation table (each device = a struct with path template, mode, type, handler function pointer).
- `app/rs/kr64/src/seccomp.rs` — BPF program + SIGSYS handler (see §3.4).
- `app/rs/kr64/src/proc_emu.rs` — `/proc` synthesiser (see §3.5).
- `app/rs/kr64/src/mount_mgr.rs` — bind-mount + tmpfs orchestration (skip /dev, /mnt, /storage as "special" per VM's `mount_mgr` strings).
- `app/rs/kr64/src/shadowhook.rs` — or use the actual `shadowhook` crate (https://github.com/bytedance/android-inline-hook has a Rust binding via `shadowhook-sys`).

**Files to modify.**
- `app/rs/src/core.rs` — change `Command::new("./init").spawn()` to spawn `libkr64.so` first, then have `libkr64.so` spawn `init`.
- `app/build.gradle` — add `kr64` to the list of Rust crates built by `cargo-xdk`.
- `app/rs/build_rs.sh` — add `kr64` to the build loop.

**Acceptance criteria.**
- `./libkr64.so --help` prints usage.
- After exec'ing `libkr64.so 0 /data/data/io.twoyi/rootfs /data/data/io.twoyi/rootfs/fs /data/data/io.twoyi/lib64 /data/data/io.twoyi/config.json 3 0`, the directory `/data/data/io.twoyi/vm/vm0/dev/` contains all 20+ device files (verified via `ls`).
- The guest's `init` can `open("/dev/__properties__")` without ENOENT.
- The guest's `init` can `open("/dev/qemu_pipe")` and write the GL handshake bytes.

### 3.2 Binder virtualisation (the hard one)

**What VM does.** `libvm.so` (in the Java app process) creates `/vm%d/dev/binder` via the `setupBinder()` JNI, and the Java `BinderService` wraps the host's `IActivityManager` with a `java.lang.reflect.Proxy` so the guest's `servicemanager` lookups are proxied back into the host app (see §2.5).

**What twoyi currently does.** Uses the host's `/dev/binder` directly. The guest's `servicemanager` registers with the host's `servicemanager`. This means the guest's `getSystemService(ACTIVITY_SERVICE)` returns the *host's* ActivityManager — the guest can launch host apps but not its own. This is why twoyi can't have its own package manager.

**What to build.** Two pieces:

**(a) Native binder proxy in Rust** — `app/rs/kr64/src/binder_proxy.rs`:
- Open `/dev/binder` on the host side.
- Create `/vm%d/dev/binder` as a char device via `mknodat(S_IFCHR, …)`.
- Accept `BINDER_WRITE_READ` / `BINDER_VERSION` / `BINDER_SET_MAX_THREADS` ioctls from the guest.
- Parse the binder transaction header.
- For transactions targeting `android.app.IActivityManager` / `android.app.IActivityTaskManager` / `android.content.pm.IPackageManager` / `android.view.IWindowManager` — route them to the Java side via the event socket.
- For transactions targeting `android.os.IServiceManager` — synthesise responses (return a fake service handle for each well-known service name).
- For all other transactions — proxy directly to the host binder.

**(b) Java-side binder service** — `app/src/main/java/io/twoyi/service/BinderService.java`:
- Copy VM's `BinderService.m5206WWWWoWWWWo` structure (see §2.5).
- Reflect into `ActivityManager` to get the system `IActivityManager` IBinder.
- Wrap with a `Proxy`.
- Add a `setupBinder()` JNI that calls into `libkr64.so`.
- Implement `IBinderService.Stub` with the AIDL interface `io.twoyi.service.IBinderService`.

**Files to create.**
- `app/src/main/java/io/twoyi/service/BinderService.java`
- `app/src/main/java/io/twoyi/service/IBinderService.aidl` — AIDL stub (interface descriptor `"io.twoyi.service.IBinderService"`).
- `app/src/main/java/io/twoyi/FreeReflection.java` — copy the FreeReflection trick (or use ` Reflection_exemptAll` from `me.weishu.reflection.BootstrapClass` — bundle the dex).
- `app/rs/kr64/src/binder_proxy.rs` (above).

**Files to modify.**
- `app/src/main/java/io/twoyi/TwoyiApplication.java` — add `FreeReflection.exemptAll()` in `attachBaseContext` (currently twoyi doesn't do this — VM does).
- `app/src/main/java/io/twoyi/utils/RomManager.java` — add a `setupBinder()` call in `ensureBootFiles()` (currently only kills orphans, etc.).

**Acceptance criteria.**
- After boot, `adb shell service list` inside the guest shows `android.app.IActivityManager: […]` resolving to the twoyi proxy, not the host's.
- `adb shell am start -n com.android.settings/.Settings` inside the guest starts the guest's Settings, not the host's.
- `adb shell pm list packages` inside the guest lists only guest-installed packages, not the host's.

**MVP skip.** If implementing full binder virtualisation is too much, an initial GSI boot can run with the host's binder shared (as today). The guest will boot but every `getSystemService()` call returns host services — so the guest's `system_server` won't be able to register itself. This will likely crash early. Recommend implementing at least the `IActivityManager` and `IPackageManager` proxies as the minimum.

### 3.3 Graphics buffer management (`/dev/gb` and `/dev/gb2`)

**What VM does.** `libkr64.11.so` (A 11 only) creates `/dev/gb` and `/dev/gb2` as char devices (via `mknodat S_IFCHR` at `0x11d770`, decoded with keys 0xe0 and 0x0c). These expose a `gralloc`-like ioctl interface for allocating graphics buffers. The guest's `surfaceflinger` calls these ioctls to allocate buffers for compositing; the host returns a buffer handle that maps to either an ashmem region or a host `AHardwareBuffer`.

The presence of two devices (`gb` + `gb2`) suggests:
- `/dev/gb` is the **framework gralloc** (used by surfaceflinger, accessed via `/dev/binder`).
- `/dev/gb2` is the **vendor gralloc** (used by HALs, accessed via `/dev/vndbinder`).

This split matches Android 11's gralloc HAL layout where `android.hardware.graphics.allocator@4.0` (framework) and `android.hardware.graphics.allocator-V1-ndk` (vendor) are separate services.

**What twoyi currently does.** No gralloc emulation. The `libOpenglRender_aosp.so` (the rebuilt AOSP emugl renderer at `/home/z/my-project/app/src/main/jniLibs/x86_64/libOpenglRender_aosp.so`) implements its own buffer management via the `ColorBuffer` class in `app/rs/openglrenderer/src/gralloc.rs`. The guest's SurfaceFlinger has no gralloc HAL, so it crashes on the first composite.

**What to build.** `app/rs/kr64/src/gb.rs`:
- Create `/dev/gb` and `/dev/gb2` as char devices.
- Implement these ioctls (the exact numbers can be inferred from `android.hardware.graphics.allocator@4.0` HAL header in AOSP `hardware/interfaces/graphics/allocator/4.0/IAllocator.hal`):
  - `ALLOCATE` — allocate a buffer of given width/height/format/usage. Return a buffer handle that's a host `AHardwareBuffer` (we already have these via `libEGL.so`).
  - `DUMP_DEBUG_INFO` — return a fake debug string.
  - `GET_ALL_ALLOCATOR_FUNCTIONS` — return function pointers for the gralloc module.

For the MVP, we can route gralloc allocation through the existing `libOpenglRender_aosp.so` ColorBuffer infrastructure — each guest gralloc allocation creates a host-side `ColorBuffer` and returns its ID as the buffer handle.

**Files to create.**
- `app/rs/kr64/src/gb.rs` — char device + ioctls.
- `app/rs/openglrenderer/src/gralloc.rs` — already has `ColorBuffer`; add a `from_guest_handle(u64) -> ColorBuffer` lookup.

**Files to modify.**
- `app/rs/kr64/src/devices.rs` — register `/dev/gb` and `/dev/gb2`.

**Acceptance criteria.**
- After boot, `adb shell dumpsys SurfaceFlinger` inside the guest shows a non-zero buffer count.
- The guest's launcher renders its first frame (visible via the Surface registered through `nativeAddSurface`).

### 3.4 Seccomp filter

**What VM does.** Installs a `SECCOMP_SET_MODE_FILTER` BPF program with a SIGSYS handler that emulates blocked syscalls (see §2.7).

**What twoyi currently does.** No seccomp. The guest can call any syscall the host kernel allows. This is unsafe and also means the guest sees the host's `/proc`, `/sys`, etc. (no isolation).

**What to build.** `app/rs/kr64/src/seccomp.rs`:

1. **Allowlist of syscalls** — start from Android's own seccomp policy (in AOSP at `system/seccomp/policy.txt`). For twoyi we need to also allow the syscalls that `libkr64.so` intercepts (so they trap to our SIGSYS handler).

2. **Blocklist (trap-to-SIGSYS)** — these syscalls are blocked and emulated:
   - `open`/`openat` for `/proc/…`, `/sys/…`, `/dev/…` paths (intercepted by the proc emulator)
   - `mount`/`umount`/`umount2` (intercepted by the mount_mgr)
   - `pivot_root`/`chroot` (intercepted to redirect to the per-VM rootfs)
   - `reboot` (intercepted to send a shutdown event)
   - `swapon`/`swapoff` (intercepted to be no-ops)
   - `acct` (intercepted to be a no-op)
   - `sethostname`/`setsid` (intercepted for the per-VM hostname)

3. **SIGSYS handler** — `sigaction(SIGSYS, …)`:
   - Read the syscall number from `ucontext->uc_mcontext`.
   - Log `BLOCKED.SYSCALL.FAILED: <nr>`.
   - Dispatch to the appropriate emulator function.
   - Set the return value in `ucontext->uc_mcontext.regs[0]`.
   - Increment the instruction pointer past the syscall instruction.
   - Return.

**Files to create.**
- `app/rs/kr64/src/seccomp.rs`
- `app/rs/kr64/src/bpf_filter.rs` — the compiled BPF program (generate with `libseccomp` or hand-write).

**Files to modify.**
- `app/rs/kr64/src/main.rs` — call `seccomp::init()` after all devices are created and before `fork`+`exec` of the guest `init`.

**Acceptance criteria.**
- After boot, `adb shell cat /proc/self/status` inside the guest returns a synthesised status (not the host's).
- After boot, `adb shell mount` inside the guest shows only guest mounts.
- After boot, `adb shell reboot` inside the guest sends a shutdown event (doesn't actually reboot the host).

### 3.5 `/proc` emulator

**What VM does.** Intercepts `open("/proc/…")` and synthesises content (see §2.8).

**What twoyi currently does.** Uses the host's `/proc`. The guest sees the host's process list, mounts, cmdline, version — which breaks the guest's `init` (it expects `androidboot.hardware=…` on the cmdline) and leaks host info.

**What to build.** `app/rs/kr64/src/proc_emu.rs`:

- Create `/dev/vmproc` as a char device (per VM's design).
- Hook `open`/`openat` via shadowhook (or via the seccomp SIGSYS trap — see §3.4).
- When the path matches `/proc/…`, redirect to `/dev/vmproc` + an ioctl that selects the virtual file.
- Implement these synthesised files (per §2.8 table):
  - `/proc/cmdline` — `androidboot.hardware=twoyi androidboot.bootdevice=/dev/block/vdc androidboot.serialno=…`
  - `/proc/version` — `Linux version 4.14.x-perf-g… (build@host) …` (match what the GSI expects)
  - `/proc/self/maps` — filtered host maps (only show guest-mapped regions)
  - `/proc/self/status` — synthesised with the guest's PID/UID/GID
  - `/proc/self/mounts` — only guest mounts
  - `/proc/self/exe` — symlink to `/system/bin/init` (the guest's init, not the host's)
  - `/proc/net/if_inet6/` — based on `NetlinkManager`'s virtual interfaces
  - `/proc/sys/kernel/kptr_restrict` — `1`
  - `/proc/sys/vm/mmap_rnd_bits` — `16`

**Files to create.**
- `app/rs/kr64/src/proc_emu.rs`

**Files to modify.**
- `app/rs/kr64/src/devices.rs` — register `/dev/vmproc`.
- `app/rs/kr64/src/seccomp.rs` — add `open`/`openat` to the trap list (or use shadowhook instead).

**Acceptance criteria.**
- After boot, `adb shell cat /proc/cmdline` inside the guest returns the synthesised cmdline.
- After boot, `adb shell cat /proc/version` inside the guest returns a Linux version string matching the GSI's expected kernel.
- The guest's `init` doesn't crash on `open("/proc/cmdline")`.

### 3.6 Inline hooking (shadowhook equivalent)

**What VM does.** Embeds `shadowhook v1.0.8` (ByteDance's inline hook library) — hooks the dynamic linker's `do_dlopen` so guest `dlopen` calls can be redirected (see `VM_KR64_ANALYSIS.md` §4.1).

**What twoyi currently does.** No hooking. The guest's `dlopen` calls go to the system linker, which loads from `/system/lib64/` (the host's system).

**What to build.** Two options:

**(a) Use shadowhook directly** — there's a Rust binding `shadowhook-sys` (https://crates.io/crates/shadowhook-sys). Add it to `app/rs/kr64/Cargo.toml`. Hook:
- `do_dlopen` in the linker (so guest `dlopen` loads from the per-VM rootfs).
- `open`/`openat` in libc (for `/proc` emulation).
- `mount`/`umount` (for the mount_mgr).
- `__system_property_get` (for property emulation).

**(b) Use LD_PRELOAD** — simpler, less invasive. Set `LD_PRELOAD=libkr64.so` when exec'ing the guest. Override the libc functions in `libkr64.so` itself. This is what VM probably does for some hooks (shadowhook is for hooks they can't override via LD_PRELOAD because the function is already loaded).

Recommend (b) for the MVP — it's simpler and works for all the hooks we need.

**Files to create.**
- `app/rs/kr64/src/hooks.rs` — LD_PRELOAD overrides for `open`, `openat`, `mount`, `umount`, `__system_property_get`, `dlopen`.

**Files to modify.**
- `app/rs/kr64/src/main.rs` — set `LD_PRELOAD=libkr64.so` in the env before `execve("./init")`.

**Acceptance criteria.**
- After boot, `adb shell ls /system/lib64/` inside the guest shows the guest's libraries (not the host's).
- After boot, `adb shell getprop ro.build.fingerprint` returns the guest's fingerprint.

### 3.7 ROM extraction (GSI-aware)

**What VM does.** Downloads `rom.zip` (a ZIP), decrypts with AES-128-ECB, extracts via ZIP or 7-Zip into `<vmDataDir>/fs/` (see §2.2). The extracted tree is already flattened (e.g. `/system/product/...` is a directory inside `/system/`, not a separate mount).

**What twoyi currently does.** `RomManager.java` extracts a pre-built `rootfs.7z` (Android 8.1 only) into `/data/data/io.twoyi/rootfs/`. The format is a flat directory tree, not a multi-partition GSI.

**What to build.** `app/src/main/java/io/twoyi/utils/GsiExtractor.java`:

1. **Input**: a GSI `system.img` (ext4 or sparse-ext4) + optionally `product.img`, `system_ext.img`, `boot.img` (for the ramdisk).
2. **Extract `system.img`**:
   - If sparse: `simg2img system.img system.raw.img` (use the `libsparse` Rust crate or shell out to `simg2img`).
   - If ext4: mount via `fuse2fs` (no root needed) or extract via `rust-ext4` crate (https://crates.io/crates/ext4).
   - Walk the filesystem and copy files into `<vmDataDir>/fs/system/`.
3. **Extract `product.img`** (if present) → `<vmDataDir>/fs/system/product/` (note: Treble flattens product under system on the device).
4. **Extract `system_ext.img`** (if present) → `<vmDataDir>/fs/system/system_ext/`.
5. **Extract `boot.img`** ramdisk (if present):
   - `boot.img` = header + kernel + ramdisk (gzip/lz4) + second stage.
   - Use the `bootimage` Rust crate to parse.
   - Extract the ramdisk and unpack with `cpio -idmv` (shell out).
   - Place into `<vmDataDir>/fs/ramdisk/`.
6. **Synthesise a minimal `vendor.img`** if the user didn't supply one:
   - Pre-built at `app/src/main/assets/vendor.img` — contains stub HALs for `vibrator`, `graphics.allocator`, `graphics.mapper`, `graphics.composer`, `audio`, `camera`, `sensors`, `gatekeeper`, `keymaster`, `health`, `power`.
   - Each HAL is a tiny binary that returns success for all calls.
   - Extract into `<vmDataDir>/fs/vendor/`.
7. **Apply init patches**:
   - Patch `/system/etc/init/hw/init.rc` to remove `mount` directives that would fail (e.g. `mount tmpfs tmpfs /tmp` is fine, `mount ext4 /dev/block/by-name/system /system` is not — remove these).
   - Add `LD_PRELOAD=libkr64.so` to the zygote service.
   - Patch `/vendor/etc/init/*.rc` similarly.

**Files to create.**
- `app/src/main/java/io/twoyi/utils/GsiExtractor.java`
- `app/rs/gsi_extractor/Cargo.toml` — Rust crate for sparse/ext4/cpio parsing (call from Java via JNI).
- `app/rs/gsi_extractor/src/lib.rs` — JNI entry points: `nativeExtractSystemImg(path, destDir)`, `nativeExtractProductImg`, `nativeExtractBootImg`.
- `app/src/main/assets/vendor.img` — pre-built minimal vendor image (or a script to generate one from AOSP — see §5.7).

**Files to modify.**
- `app/src/main/java/io/twoyi/utils/RomManager.java` — replace `initRootfs()` with a call to `GsiExtractor.extract(context, gsiFile)`.

**Acceptance criteria.**
- Given an Android 11 x86_64 GSI `system.img` (from https://ci.android.com/builds/branches/aosp-master/grid), `GsiExtractor.extract()` produces a directory tree at `<vmDataDir>/fs/` containing `/system/bin/init`, `/system/etc/init/hw/init.rc`, `/system/product/`, `/system/system_ext/`, `/vendor/etc/vintf/manifest/`, etc.
- `file <vmDataDir>/fs/system/bin/init` reports `ELF 64-bit LSB shared object, x86-64`.
- The total size of the extracted tree is within 2x of the input `system.img` size (some files compress better as ext4 than as plain files).

### 3.8 Init configuration

**What VM does.** Eight `BugNFixTask` classes patch the extracted filesystem (see §2.9). Notably:
- `BuildVMPropTask` rewrites `/system/build.prop` with the host's build fingerprint.
- `MagiskTask` patches `/system/etc/init/hw/init.rc` to inject magisk service entries.
- `FixCPUArchTask` rewrites `app_process32`/`app_process64` symlinks.
- `BuildTmpfsTask` mounts tmpfs on `/tmp`, `/dev`, etc.

**What twoyi currently does.** Patches `services.jar` for the `PackageInstallerSession` bug (`patchServicesJarForPackageInstaller`). No other init patches.

**What to build.** A `GsiInitPatcher` Java class that applies these patches to the extracted GSI:

1. **`/system/build.prop`** — overwrite these properties:
   - `ro.build.fingerprint` — set to the host's `Build.FINGERPRINT` (so Play Integrity basic attestation passes).
   - `ro.build.id` — set to the host's `Build.ID`.
   - `ro.build.version.incremental` — set to the host's `Build.VERSION.INCREMENTAL`.
   - `ro.product.cpu.abi` — set to the host's primary ABI (`x86_64` or `arm64-v8a`).
   - `ro.hardware` — set to `twoyi` (matches what `/proc/cmdline` returns).
2. **`/system/etc/init/hw/init.rc`** — apply these patches:
   - Remove all `mount ext4 …` and `mount f2fs …` lines (the filesystems are already mounted by libkr64).
   - Remove the `service flash_recovery` block (no recovery partition).
   - Add `setenv LD_PRELOAD /system/lib64/libkr64.so` to the `service zygote` block.
   - Add a new `service twoyi_event /system/bin/twoyi_event` that connects to the host's event socket and signals `BOOT_COMPLETED`.
3. **`/vendor/etc/init/*.rc`** — remove all `service` blocks that start HALs we don't implement (camera HAL, sensors HAL if not proxying). Replace with stub services that exit 0 immediately.
4. **`/system/bin/app_process64`** — replace with a wrapper script that execs the host's `app_process64` with `LD_PRELOAD` set:
   ```sh
   #!/system/bin/sh
   exec /system/bin/linker64 /system/lib64/libkr64.so --preload /system/bin/app_process64_real "$@"
   ```
   (Or better — keep the original `app_process64` binary from the GSI, but preload `libkr64.so` via `LD_PRELOAD` in the zygote service in init.rc.)
5. **`/system/etc/prop.default`** — apply the same patches as `/system/build.prop`.
6. **`/vendor/build.prop`** — set `ro.vendor.build.fingerprint` to match.

**Files to create.**
- `app/src/main/java/io/twoyi/utils/GsiInitPatcher.java` — implements the above patches.
- `app/src/main/java/io/twoyi/utils/setup/` — port VM's `Bug1FixTask` … `Bug8FixTask` classes from `/home/z/my-project/vm-java-src/sources/com/android/vmcore/startup/`.

**Files to modify.**
- `app/src/main/java/io/twoyi/utils/RomManager.java` — call `GsiInitPatcher.patch()` after `GsiExtractor.extract()`.

**Acceptance criteria.**
- After patching, `grep "mount ext4" <vmDataDir>/fs/system/etc/init/hw/init.rc` returns no matches.
- After patching, `cat <vmDataDir>/fs/system/build.prop | grep ro.build.fingerprint` returns the host's fingerprint.
- After patching, `cat <vmDataDir>/fs/system/etc/init/hw/init.rc | grep LD_PRELOAD` shows the libkr64 preload.

### 3.9 HAL virtualisation

**What VM does.** Full HAL proxy via `HALManager` (display, input, audio, camera, sensor, location, wifi, phone, battery, network) — see `VM_JAVA_ANALYSIS.md` §5.4 table.

**What twoyi currently does.** Display (via `libOpenglRender`), input (touch + key), and that's it. No audio, camera, sensor, location, wifi, phone, battery, or network HALs.

**What to build.** The minimum viable HAL set for a GSI to boot:

| HAL | Priority | Implementation |
|---|---|---|
| `graphics.allocator` / `graphics.mapper` / `graphics.composer` | **Critical** | Implement via `/dev/gb` + `/dev/gb2` (see §3.3) |
| `audio` (`audio.primary`, `audio.a2dp`, `audio.usb`) | **High** | Stub: return success for all `init`/`standby` calls; route `createTrack`/`createRecordBuffer` to host `AudioTrack`/`AudioRecord` via the event socket |
| `keymaster` / `gatekeeper` | **High** | Stub: return a fixed key for all operations (will fail SafetyNet but allow boot) |
| `health` | Medium | Stub: return 100 % charged |
| `power` | Medium | Stub: return success for all calls |
| `vibrator` | Medium | Stub: no-op |
| `sensors` (12 types) | Low (for boot) | Stub: return empty sensor list; the GSI will boot without sensors, just no auto-rotate |
| `camera` | Low | Stub: return "no cameras available" |
| `gps` / `location` | Low | Stub: return last-known location from host |
| `wifi` | Low | Stub: return disconnected |
| `telephony` | Low | Stub: return no SIM |
| `bluetooth` | Low | Stub: return disabled |

**For each stub HAL**, the implementation is:
1. A small Rust binary in `app/rs/hals/<name>/` that implements the HIDL/AIDL interface and returns stubs.
2. Place at `<vmDataDir>/fs/vendor/bin/hw/android.hardware.<name>-service.twoyi`.
3. Add a VINTF manifest entry at `<vmDataDir>/fs/vendor/etc/vintf/manifest/twoyi-<name>.xml`:
   ```xml
   <manifest version="1.0" type="hal">
       <name>android.hardware.<name></name>
       <version>4.0</version>
       <interface>
           <name>I<Name></name>
           <instance>default</instance>
       </interface>
   </manifest>
   ```
4. Add an init.rc entry at `<vmDataDir>/fs/vendor/etc/init/twoyi-<name>.rc`:
   ```
   service twoyi-<name> /vendor/bin/hw/android.hardware.<name>-service.twoyi
       class hal
       user system
       group system
   ```

**Files to create.**
- `app/rs/hals/graphics/` — critical (uses `/dev/gb`).
- `app/rs/hals/audio/` — high.
- `app/rs/hals/keymaster/` — high.
- `app/rs/hals/health/` — medium.
- `app/rs/hals/power/` — medium.
- `app/rs/hals/vibrator/` — medium.
- `app/rs/hals/sensors/` — low (stub).
- `app/rs/hals/camera/` — low (stub).
- `app/rs/hals/gps/` — low (stub).
- `app/rs/hals/wifi/` — low (stub).
- `app/rs/hals/telephony/` — low (stub).
- `app/rs/hals/bluetooth/` — low (stub).

**For HALs we want to actually proxy to the host** (audio, camera, sensors, gps), the implementation is:
1. Stub the HIDL interface on the guest side.
2. When the guest calls a HAL method, send an event over the event socket to the host.
3. Host fulfills the call (e.g. `AudioTrack.write(...)`) and sends the result back.

**Files to modify.**
- `app/rs/kr64/src/main.rs` — register the HAL binaries in the device tree.
- `app/src/main/java/io/twoyi/utils/GsiInitPatcher.java` — register the VINTF manifest entries.

**Acceptance criteria.**
- After boot, `adb shell lshal` inside the guest lists all HALs as `OK` (not `CRASHED`).
- After boot, `adb shell dumpsys audio` returns without crashing.
- After boot, the guest can play an audio file (audible on the host speaker).

---

## 4. Implementation priority (MVP → full)

### 4.1 Minimum Viable Boot (MVP) — get a GSI to show its first frame

To boot an Android 11 x86_64 GSI to the launcher (no apps installed, no audio, no camera), the minimum set is:

1. **`app/rs/kr64/`** with:
   - Device tree creation (all 20+ devices from §2.4 table) — `devices.rs`.
   - `/dev/qemu_pipe` server (port from `app/rs/openglrenderer/src/pipe.rs`).
   - `/dev/__properties__` writer (init writes here, everyone reads).
   - `/dev/ashmem` proxy (use host's ashmem — Android 11 still supports it via the host kernel if the host is Android 10 or earlier; for Android 11+ hosts, use memfd).
   - `/dev/input/touch` (port from `app/rs/src/input.rs`).
   - `/dev/event` socket server (port from `app/src/main/java/io/twoyi/TwoyiSocketServer.java` — but make it a unix socket at `<vmDataDir>/dev/event`, not an abstract socket).
2. **`app/rs/kr64/src/proc_emu.rs`** — `/proc/cmdline` and `/proc/version` only (the rest can wait).
3. **`app/rs/kr64/src/gb.rs`** — `/dev/gb` with `ALLOCATE` ioctl routing to `libOpenglRender_aosp.so` ColorBuffer.
4. **`app/src/main/java/io/twoyi/utils/GsiExtractor.java`** — extract `system.img` + `product.img` + `system_ext.img` from a GSI.
5. **`app/src/main/java/io/twoyi/utils/GsiInitPatcher.java`** — apply the init.rc + build.prop patches.
6. **`app/rs/hals/graphics/`** — graphics allocator/mapper/composer HALs (critical).
7. **`app/rs/hals/keymaster/`** + **`app/rs/hals/gatekeeper/`** — stubs (return fixed keys).
8. **`app/rs/hals/health/`** + **`app/rs/hals/power/`** + **`app/rs/hals/vibrator/`** — stubs.
9. **`app/src/main/assets/vendor.img`** — pre-built minimal vendor image with the stub HALs.

**Estimated effort:** 4–6 weeks for one engineer.

### 4.2 What can be skipped initially

- **Binder virtualisation (§3.2)** — skip for MVP. Use host's binder. The guest will boot but every `getSystemService()` returns host services. The guest's `system_server` will fail to register itself — boot will hang at the Android boot animation. To work around this for the MVP, patch `system_server` to not register (remove the `publishService` calls). This is hacky but lets you verify the rest of the stack works.
- **Seccomp filter (§3.4)** — skip for MVP. Just don't install it. The guest will see the host's `/proc` etc., which is wrong but won't crash immediately.
- **Full `/proc` emulator (§3.5)** — implement only `/proc/cmdline` and `/proc/version`. The rest can be the host's (until we hit a boot failure caused by it).
- **Inline hooking (§3.6)** — skip for MVP. The guest's `dlopen` will load from the host's `/system/lib64/`. This may cause ABI mismatches (host is Android 11, guest GSI is Android 11 — should be OK if versions match).
- **Audio/camera/sensors/gps/wifi/telephony/bluetooth HALs (§3.9)** — all stubs for MVP.
- **APEX support (§2.6)** — pre-extract all APEXes into `fs/system/apex/<name>/` and patch `apexd` to be a no-op.

### 4.3 What's the hardest part?

**Binder virtualisation (§3.2)** is by far the hardest. Reasons:
1. The binder protocol is complex (transactions, async/sync, death notifications, file descriptor passing).
2. The guest's `servicemanager` must register with our virtual binder, but the host's `servicemanager` is still running on the host's `/dev/binder`. We need to either:
   - Run a second `servicemanager` instance on our virtual binder (this is what VM does), or
   - Proxy all binder transactions to the host's `servicemanager` and translate the responses.
3. The Java-side `Proxy` of `IActivityManager` requires hidden-API access (FreeReflection) and careful handling of `transact()` calls.
4. Each Android version changes the binder transaction codes — we need to support A 9, A 11, A 13+ if we want multi-version support.

**Graphics buffer management (§3.3)** is the second hardest — getting gralloc right so that SurfaceFlinger can composite is fiddly.

**Seccomp + SIGSYS handler (§3.4)** is the third hardest — getting the BPF filter right so we trap exactly the right syscalls without breaking the guest requires careful testing.

### 4.4 Suggested milestone order

1. **Week 1–2:** `app/rs/kr64/` skeleton with device tree creation + `/dev/qemu_pipe` + `/dev/input/touch` + `/dev/event`. Test: can spawn `libkr64.so` and see the device files appear.
2. **Week 2–3:** `GsiExtractor.java` + `GsiInitPatcher.java`. Test: can extract an Android 11 x86_64 GSI into `<vmDataDir>/fs/` and patch init.rc.
3. **Week 3–4:** `app/rs/hals/graphics/` (graphics allocator/mapper/composer). Test: can allocate a buffer via the HAL.
4. **Week 4–5:** `/dev/gb` + integration with `libOpenglRender_aosp.so` ColorBuffer. Test: SurfaceFlinger can composite a frame.
5. **Week 5–6:** `app/rs/hals/keymaster/` + `gatekeeper/` + `health/` + `power/` + `vibrator/` stubs. Test: GSI boots to launcher.
6. **Week 6–8:** `/proc` emulator (full) + seccomp filter. Test: `adb shell cat /proc/cmdline` returns synthesised cmdline.
7. **Week 8–12:** Binder virtualisation (§3.2). Test: `adb shell am start` inside the guest starts the guest's activity, not the host's.
8. **Week 12+:** Audio, camera, sensors, gps, wifi, telephony, bluetooth HAL proxies.

---

## 5. Architecture for x86_64

### 5.1 Why x86_64 is different

Twoyi was originally arm64-only. The codespace `twoyi-dev-3-jr47xg6xvx7ghq6p` is x86_64 with KVM. Per `TWOYI_HONEST_STATUS.md`, the codespace has working KVM (AMD EPYC 7763, EastUs region). The Android emulator (AVD) on the codespace runs an x86_64 Android 11 image.

For the container path (no KVM), we need:
1. An **x86_64 GSI** — `system-x86_64.img`, `product-x86_64.img`, etc. (from https://ci.android.com/builds/branches/aosp-master/grid).
2. **x86_64 native libraries** — `libkr64.so` must be built for x86_64. The existing PIE pattern in `app/rs/src/interp.c` already supports x86_64 (per `PIE_IMPLEMENTATION.md` and `app/rs/.cargo/config.toml`).
3. **x86_64 `libOpenglRender.so`** — already built and present at `/home/z/my-project/app/src/main/jniLibs/x86_64/libOpenglRender_aosp.so` (597 KB, all 6 twoyi ABI symbols exported — see `AOSP_BUILD_RESULTS.md`).
4. **x86_64 host** — the codespace runs Android Studio's emulator (or redroid container — see `REDROID_TESTING.md`). The host kernel must be x86_64 Linux (it is).

### 5.2 x86_64-specific concerns

1. **CPU architecture detection** — `libkr64.so` must detect the host ABI at runtime (via `__system_property_get("ro.product.cpu.abi")` or `getauxval(AT_BASE_PLATFORM)`) and load the correct guest libraries.
2. **Host vs guest ABI mismatch** — if the host is x86_64 and the guest GSI is arm64, the guest binaries won't run (no binary translation in the container path). **Always use a GSI matching the host ABI.**
3. **Kernel modules** — x86_64 kernels may not have `binder` or `ashmem` modules loaded by default. The codespace has them (per `TWOYI_HONEST_STATUS.md` — KVM is working). For non-codespace x86_64 hosts (e.g. a Linux desktop), the user may need to `modprobe binder_linux ashmem_linux`.
4. **SELinux** — x86_64 Android hosts (emulator, redroid) typically run with SELinux permissive. This is good for debugging but means we don't need to worry about SELinux denials for the MVP.
5. **GL driver** — `libOpenglRender_aosp.so` loads `libEGL.so` / `libGLESv1_CM.so` / `libGLESv2.so` from the system. On the x86_64 emulator these are SwiftShader (software GL). On a real x86_64 device they'd be the vendor's GL driver. Both work for our purposes.

### 5.3 The x86_64 build matrix

For the container path on x86_64, the build matrix is:

| Component | x86_64 build status | Notes |
|---|---|---|
| `libtwoyi.so` (Rust, JNI) | ✅ Already builds for x86_64 per `ARCHITECTURE.md` §7.1 — `abiFilters` includes `x86_64` |
| `libOpenglRender_aosp.so` | ✅ Built — `/home/z/my-project/app/src/main/jniLibs/x86_64/libOpenglRender_aosp.so` (597 KB) |
| `libloader.so` | ✅ Builds for x86_64 per `app/rs/loader/build.sh` (accepts ABI list) |
| `libadb.so` | ❌ Closed-source, arm64-only. Need to either build from AOSP or use an open-source Java ADB client (`adblib`) |
| `libkr64.so` (new) | 🔨 To be built — `app/rs/kr64/` (this plan) |
| `libkrloader64.so` (new) | 🔨 To be built — or reuse `libloader.so` (which already does PIE) |
| HAL binaries (new) | 🔨 To be built — `app/rs/hals/*/` (this plan) |
| Guest GSI | 🔨 To be sourced — download an x86_64 GSI from `ci.android.com` |

### 5.4 The x86_64 boot flow (target)

```
TwoyiApplication.onCreate (Java, x86_64)
  ├─ FreeReflection.exemptAll()
  ├─ System.loadLibrary("twoyi")  → libtwoyi.so (x86_64)
  └─ RomManager.ensureBootFiles()
      ├─ GsiExtractor.extract(context, systemImg)  → <vmDataDir>/fs/
      ├─ GsiInitPatcher.patch(<vmDataDir>/fs/)
      └─ BinderService.setupBinder(vmId, …)  → creates /vm0/dev/binder

Render2Activity.onCreate (Java)
  └─ surfaceCreated → Renderer.init(surface, loaderPath, w, h, dpi)
      → libtwoyi.so::core::init_renderer
         ├─ input::start_input_system  (creates /dev/input/touch, /dev/input/key0)
         ├─ thread::spawn (renderer thread — opens /dev/qemu_pipe server)
         └─ Command::new("./libkr64.so")
              .arg("0").arg(vmDataDir).arg(fsDir).arg(libDir).arg(configPath).arg("3").arg("0")
              .env("LD_PRELOAD", "")  ← not for libkr64 itself
              .spawn()
              │
              └─ libkr64.so (x86_64 process):
                  ├─ parse argv (7 args)
                  ├─ create /dev/qemu_pipe, /dev/gb, /dev/gb2, /dev/input/touch,
                  │   /dev/event, /dev/__properties__, /dev/ashmem, /dev/vmproc, …
                  ├─ install seccomp filter
                  ├─ install SIGSYS handler
                  ├─ fork() → child:
                  │   ├─ chroot(<vmDataDir>/fs)
                  │   ├─ setenv("LD_PRELOAD", "/system/lib64/libkr64.so")
                  │   ├─ setenv("TWOYI_VM_ID", "0")
                  │   └─ execve("/system/bin/init", argv, envp)
                  └─ parent: accept loop on /dev/event socket

Guest init (x86_64, in chroot):
  ├─ reads /system/etc/init/hw/init.rc (patched)
  ├─ mounts tmpfs on /tmp, /dev (already done by libkr64)
  ├─ starts servicemanager → registers with /vm0/dev/binder
  ├─ starts surfaceflinger → opens /dev/qemu_pipe (GL), /dev/gb (gralloc)
  ├─ starts hwservicemanager → registers with /dev/hwbinder (proxy to /vm0/dev/binder)
  ├─ starts audioserver, cameraserver → stub HALs
  ├─ starts zygote (with LD_PRELOAD=libkr64.so) → fork system_server
  ├─ system_server boots PackageManagerService, ActivityManagerService, etc.
  │   (these register with /vm0/dev/binder and proxy to host BinderService)
  └─ sys.boot_completed=1 → guest sends event via /dev/event socket:
       "BOOT_COMPLETED`"
       → TwoyiStatusManager.markStarted() → boot latch released → first frame
```

### 5.5 KVM alternative (out of scope for this plan)

If the container path proves too hard, the alternative is to use KVM to boot the GSI in a real VM. The codespace has KVM. The flow would be:

1. Use `crosvm` (Rust, https://crosvm.dev/) or QEMU to boot a minimal Linux kernel + the GSI's ramdisk.
2. The GSI boots in the VM as if on real hardware.
3. Display: `crosvm`'s GPU passthrough or `virglrenderer` — render to a host surface.
4. Input: `crosvm`'s virtual input devices.
5. Network: `crosvm`'s virtual network.
6. Binder: native (the guest has its own kernel binder).

This is much simpler conceptually (no binder virtualisation, no `/proc` emulator, no seccomp — the guest kernel does everything) but requires:
- A Linux kernel configured for Android (binder, ashmem, etc.) — `android-common kernel`.
- A `boot.img` with that kernel + the GSI's ramdisk.
- `crosvm` or QEMU built for x86_64 Android (or run on the codespace's Linux host).

**This is a separate project.** It's mentioned here for completeness. The container path is the architectural direction twoyi is already on, and is what `libkr64.so` does.

### 5.6 Testing on the codespace

The codespace has:
- Working KVM (`/dev/kvm` accessible, AMD EPYC 7763).
- An Android emulator (AVD) running x86_64 Android 11.
- `redroid/redroid:13.0.0` Docker image (x86_64).

For testing the GSI boot:
1. **Build the APK:** `./gradlew assembleRelease -Pabis=x86_64` (or `all`).
2. **Install on the emulator:** `adb install -r app/build/outputs/apk/release/app-release.apk`.
3. **Download an Android 11 x86_64 GSI:** from https://ci.android.com/builds/branches/aosp-master/grid (look for `aosp_x86_64-userdebug`).
4. **Extract `system.img`, `product.img`, `system_ext.img`** from the GSI zip.
5. **Place them in the twoyi data dir:** `/data/data/io.twoyi/gsi/`.
6. **Launch twoyi** — it should auto-detect the GSI and boot.

For automated testing, the codespace has `.devcontainer/scripts/test-twoyi.sh` and `.devcontainer/scripts/analyze-screenshots.sh` (per `ARCHITECTURE.md` §7.4) — these can be extended to verify the GSI boot.

### 5.7 Building the minimal vendor.img

The pre-built `vendor.img` for the MVP can be built from AOSP. The manifest is at `/home/z/my-project/default.xml` (pinned at `android-8.1.0_r81` — needs updating to `android-11.0.0_r48` or similar for a Treble GSI).

Steps:
1. Update `default.xml` to pin `android-11.0.0_r48` (or the matching GSI build).
2. Add the twoyi-specific stub HAL projects under `twoyi/hardware/interfaces/`.
3. Build with `make vendorimage -j8`.
4. The output is `out/target/product/<device>/vendor.img`.

For the MVP, an alternative is to manually create the vendor tree:
1. `mkdir -p <vmDataDir>/fs/vendor/{bin/hw,etc/vintf/manifest,etc/init,lib64/hw}`
2. Write stub HAL binaries (small Rust programs that exit 0).
3. Write VINTF manifest XMLs.
4. Write init.rc files.

This avoids the AOSP build entirely.

---

## 6. Future work (out of scope for this plan)

- **Multi-VM support** — VM supports up to 4 concurrent VMs. Twoyi currently supports 1. To add multi-VM, we need: per-VM data dirs (`vm/vmN/fs`), per-VM SharedPreferences, per-VM renderer pointer (already discussed in `VM_JAVA_ANALYSIS.md` Action 1).
- **KVM path** — see §5.5.
- **Server-side ROM distribution** — VM's AES-128-ECB download pipeline (key `%z89aviCM0KkbEs9`). See `VM_ROM_ANALYSIS.md` §2.
- **Samsung GameSDK hooks** — VM hooks `libGamesAware.so`, `libVSR.so`, `libGLESv2_samsung.so` for game performance. See `VM_KR64_ANALYSIS.md` §14. Not relevant for twoyi (we're not Samsung).
- **APEX support** — pre-extract for MVP (see §2.6). Full APEX support requires `apexd` to work, which requires loop mounts (needs root or FUSE).
- **Magisk / Xposed / GApps plugins** — VM ships 4 AES-encrypted plugin ZIPs (play.zip, magisk.zip, xposed.zip, superuser.zip). See `VM_ROM_ANALYSIS.md` §1. Not needed for GSI boot, but useful for compatibility.

---

## 7. References

### 7.1 Twoyi project files

- `/home/z/my-project/ARCHITECTURE.md` — twoyi architecture (3-layer: app, native, guest).
- `/home/z/my-project/PIE_IMPLEMENTATION.md` — how `libtwoyi.so` was made into a PIE executable (the same pattern applies to `libkr64.so`).
- `/home/z/my-project/app/rs/src/core.rs` — current guest spawn (`Command::new("./init").spawn()`).
- `/home/z/my-project/app/rs/src/input.rs` — current input system (touch + key sockets).
- `/home/z/my-project/app/rs/openglrenderer/src/pipe.rs` — current `/dev/qemu_pipe` server.
- `/home/z/my-project/app/rs/openglrenderer/src/gralloc.rs` — current `ColorBuffer` (extend for `/dev/gb`).
- `/home/z/my-project/app/src/main/java/io/twoyi/utils/RomManager.java` — current ROM extraction.
- `/home/z/my-project/app/src/main/jniLibs/x86_64/libOpenglRender_aosp.so` — AOSP-built x86_64 renderer (already done).
- `/home/z/my-project/download/AOSP_BUILD_RESULTS.md` — how the AOSP renderer was built.
- `/home/z/my-project/default.xml` — AOSP manifest (currently pinned at `android-8.1.0_r81`, needs bumping to A 11 for GSI).

### 7.2 Virtual Master analysis files

- `/home/z/my-project/download/VM_JAVA_ANALYSIS.md` — Java-side analysis (state machine, BinderService, HALManager, startup pipeline).
- `/home/z/my-project/download/VM_KR64_ANALYSIS.md` — Native-side analysis (libkr64.so structure, 187 imported symbols, 24 init_array ctors, decoded strings, 3 bind clusters, mknodat at 0x11d770, fork at 0x13dd14, seccomp filter, /proc emulator, libkrloader64.so, A 11 vs A 7 comparison).
- `/home/z/my-project/download/VM_ROM_ANALYSIS.md` — ROM analysis (rom.zip format, Treble paths, plugin ZIPs, AES key).
- `/home/z/my-project/download/VM_DEEP_DISASSEMBLY.md` — libvm.so disassembly (nativeAddSurface, startGBServer-equivalent).
- `/home/z/my-project/vm-java-src/sources/com/android/vmcore/` — decompiled Java sources (for porting BinderService, HALManager, startup tasks).
- `/home/z/my-project/kr64-analysis/DECODED_STRINGS.md` — full decoded string catalog from libkr64.so.

### 7.3 AOSP / external references

- Treble architecture: https://source.android.com/docs/core/architecture/halse
- GSI: https://source.android.com/docs/core/ota/gsi
- GSI download: https://ci.android.com/builds/branches/aosp-master/grid (look for `aosp_x86_64-userdebug`)
- Binder: https://source.android.com/docs/core/architecture/hidl/binder-ipc
- VINTF: https://source.android.com/docs/core/architecture/hals/vintf-manifest
- APEX: https://source.android.com/docs/core/ota/apex
- Seccomp: https://source.android.com/docs/core/permissions/seccomp
- crosvm (KVM alternative): https://crosvm.dev/
- shadowhook: https://github.com/bytedance/android-inline-hook
- FreeReflection: https://github.com/tiann/FreeReflection
- ext4 Rust crate: https://crates.io/crates/ext4
- sparse image Rust crate: https://crates.io/crates/libsparse

### 7.4 Key decoded strings (from VM analysis, for cross-reference)

| Path / string | Source | Used for |
|---|---|---|
| `/dev/event` | `VM_JAVA_ANALYSIS.md` §5.1 | Event IPC socket (Java `LocalServerSocket`) |
| `/vm%d/dev/binder` | `VM_JAVA_ANALYSIS.md` §5.2 | Per-VM virtual binder (created by `setupBinder()` JNI in libvm.so) |
| `/vm%d/dev/qemu_pipe` | `VM_KR64_ANALYSIS.md` §4.2 (key 0xba) | GL transport |
| `/vm%d/dev/gb` | `VM_KR64_ANALYSIS.md` §4.3 (key 0xe0) | Graphics buffer (framework) |
| `/vm%d/dev/gb2` | `VM_KR64_ANALYSIS.md` §4.3 (key 0x0c) | Graphics buffer (vendor) |
| `/vm%d/dev/touch` | `VM_KR64_ANALYSIS.md` §4.2 (key 0x03) | Touch input |
| `/dev/vmproc` | `VM_KR64_ANALYSIS.md` §4.2 (keys 0x47, 0x64, 0x07) | `/proc` emulator entry |
| `/dev/__properties__` | `VM_KR64_ANALYSIS.md` §4.2 (key 0x27) | Property area |
| `/dev/ashmem` | `VM_KR64_ANALYSIS.md` §4.2 (key 0x0a) | Shared memory |
| `/vm%d/dev/netlink_server` | `VM_KR64_ANALYSIS.md` §5.1 (key 0x2c) | Netlink emulation |
| `/vm%d/dev/netlink_client/nl_dhcp_%d_%d` | `VM_KR64_ANALYSIS.md` §5.2 (key 0x37) | DHCP client socket |
| `/vm%d/dev/netlink_client/netdevice_%d_%d` | `VM_KR64_ANALYSIS.md` §5.3 (key 0x1e) | Netdevice event socket |
| `INIT.SECCOMP` | `VM_KR64_ANALYSIS.md` §12 (key 0xb4) | Seccomp init log |
| `init_seccomp` | `VM_KR64_ANALYSIS.md` §12 (key 0x67, A 11 only) | Seccomp init function name |
| `__NR_rt_sigaction SIGSYS` | `VM_KR64_ANALYSIS.md` §12 (key 0x1a, A 11 only) | SIGSYS handler installation |
| `BLOCKED.SYSCALL.FAILED` | `VM_KR64_ANALYSIS.md` §12 (key 0xc9) | SIGSYS handler log |
| `android.app.IActivityManager` | `VM_JAVA_ANALYSIS.md` §5.2 | Binder interface token for the proxy |
| `com.android.vmcore.service.IBinderService` | `VM_JAVA_ANALYSIS.md` §5.2 | Binder service descriptor |
| `/system/apex/com.android.vndk.current` | `VM_KR64_ANALYSIS.md` §14 (key 0x30) | VNDK APEX (A 10+) |
| `/system/apex/com.android.art.release` | `VM_KR64_ANALYSIS.md` §14 (key 0xb9) | ART runtime APEX |
| `/vendor/etc/vintf/manifest/vibrator-default.xml` | `VM_ROM_ANALYSIS.md` §4.2 | VINTF HAL manifest example |
| `/system/etc/init/hw/init.rc` | `VM_ROM_ANALYSIS.md` §4.1 | Init script |
| `/system/build.prop` | `VM_ROM_ANALYSIS.md` §4.1 | Build properties |
| `/system/product/build.prop` | `VM_ROM_ANALYSIS.md` §4.2 | Treble product partition |
| `/system/system_ext/build.prop` | `VM_ROM_ANALYSIS.md` §4.2 | Android 10+ system_ext partition |
| `/vendor/build.prop` | `VM_ROM_ANALYSIS.md` §4.2 | Treble vendor partition |
| AES key `%z89aviCM0KkbEs9` (hex `257a3839617669434d304b6b62457339`) | `VM_ROM_ANALYSIS.md` §2 | Plugin ZIP decryption |

---

## 8. Conclusion

Booting a GSI inside twoyi is a substantial but achievable project. The minimum viable boot (GSI to launcher, no audio/camera/etc.) requires:

1. A new `app/rs/kr64/` Rust crate that mirrors VM's `libkr64.so` — creates the virtual `/dev` tree, installs seccomp, emulates `/proc`.
2. A new `app/src/main/java/io/twoyi/utils/GsiExtractor.java` that can extract an Android 11 GSI's `system.img`/`product.img`/`system_ext.img` into the per-VM `fs/` directory.
3. A new `app/src/main/java/io/twoyi/utils/GsiInitPatcher.java` that patches the extracted init.rc/build.prop for the container environment.
4. Stub HALs for graphics (critical), keymaster, health, power, vibrator.
5. A pre-built minimal `vendor.img` with the stub HALs.

The hardest piece — binder virtualisation (§3.2) — can be skipped for the MVP by patching `system_server` to not register its services. The guest will boot to the launcher but won't be able to start its own apps. Once the rest of the stack works, implementing binder virtualisation is the next priority.

For x86_64, all the necessary infrastructure is already in place: the codespace has KVM (for the alternative KVM path), the AOSP-built `libOpenglRender_aosp_x86_64.so` is ready, the Rust crates already build for x86_64, and x86_64 GSIs are downloadable from `ci.android.com`.

**Estimated total effort:** 8–12 weeks for one engineer to reach a MVP GSI boot on x86_64, 16–24 weeks for full feature parity with Virtual Master.

— End of plan —
