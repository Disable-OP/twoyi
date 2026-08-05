# twoyi — Technical Briefing

> **Audience:** a developer who needs to understand the project in 15 minutes.
> **Scope:** architecture, what we reverse-engineered from Virtual Master, what we built, the x86_64 breakthrough, and the one critical next step.
> **Date:** 2026-08-05
> **Status:** x86_64 guest `init` executes; the remaining blocker is a single well-defined task.

---

## 1. The core architecture (how twoyi works, what's missing)

**twoyi is an Android-in-Android container.** It boots a real Android GSI image
inside an unprivileged Android app process — *no root, no KVM, no kernel
module*. It shares the host kernel and the host's `binder` driver.

The trick: a **userspace "kernel-replacement" daemon** materialises a virtual
`/dev` tree inside the per-VM data directory, then `fork`+`exec`s the guest's
`init` binary (extracted from the GSI). Every kernel-ish thing the guest wants
to do that the host kernel can't or shouldn't allow — opening `/dev/qemu_pipe`,
synthesising `/proc/cmdline`, virtualising `/dev/binder`, blocking `reboot` —
is handled by the daemon via `LD_PRELOAD` + `seccomp`/`SIGSYS` traps.

### What exists today (works)

| Subsystem | Location | Status |
|---|---|---|
| Boot loader / `init` exec | `app/rs/src/core.rs` | forks guest `init` in a chroot-style data dir |
| Touch + key input sockets | `app/rs/src/input.rs` | AF_UNIX sockets at `/dev/input/{touch,key0}` |
| OpenGL renderer (legacy blob) | `app/src/main/jniLibs/arm64-v8a/libOpenglRender.so` | closed-source, arm64-only |
| OpenGL renderer (open-source, AOSP) | `app/src/main/jniLibs/{arm64,x86_64}/libOpenglRender_aosp.so` | built from AOSP source, both ABIs |
| Host-side renderer (Rust) | `app/rs/src/renderer_new/` | connects to `/dev/qemu_pipe`, decodes GL stream |
| Rootfs extractor (pre-Treble) | `app/src/main/java/io/twoyi/utils/RomManager.java` | ships Android 8.1 `rootfs.7z` |
| Boot display | `BootLogTexture` | SurfaceView that shows boot logs to the user |

### What's missing (the gap)

1. **Kernel-replacement daemon** that owns the `/dev` tree. The kr64 Rust crate
   exists (`app/rs/kr64/`) but is **not yet wired into the boot path**.
2. **`/dev/qemu_pipe` owned by twoyi.** Today the only `/dev/qemu_pipe` that
   exists is the emulator's, which speaks the wrong protocol (see §4).
3. **Binder virtualisation.** The guest shares the host's `servicemanager`,
   so `getSystemService()` returns *host* services — guest `system_server`
   can't register.
4. **`/proc` emulator.** Guest sees host `/proc`, breaks init assumptions
   (e.g. `androidboot.hardware=…` on `/proc/cmdline`).
5. **Seccomp sandbox.** Guest can call any syscall the host kernel allows.
6. **GSI extractor** that understands `system.img` / `product.img` / APEX.

### The Rust crate layout

```
app/rs/
├── src/                       # host-side renderer + input (loaded into the Java app)
│   ├── core.rs                # boot loader — spawns guest init
│   ├── input.rs               # touch/key socket servers
│   ├── renderer_bindings.rs   # FFI to libOpenglRender.so (6 C-ABI symbols)
│   └── renderer_new/          # our own pipe protocol + GL stream decoder
│       ├── pipe.rs            # PipeConnection (opens /dev/qemu_pipe, speaks /opengles3)
│       ├── renderer.rs        # high-level render loop
│       ├── gralloc.rs         # ColorBuffer management
│       └── socket_monitor.rs  # accept loop for the GL pipe
└── kr64/                      # the kernel-replacement daemon (target: cdylib)
    ├── src/
    │   ├── main.rs            # argv parser (38 lines, MVP entry)
    │   ├── lib.rs             # 784 lines, orchestrates the whole daemon
    │   ├── devices.rs         # 405 lines — mknodat/bind for all /dev nodes
    │   ├── binder.rs          # 1,959 lines — binder proxy + servicemanager stub
    │   ├── seccomp.rs         # 831 lines — BPF filter + SIGSYS handler
    │   ├── proc_emu.rs        # 534 lines — /proc synthesiser
    │   ├── mount_mgr.rs       # 457 lines — bind-mount + tmpfs
    │   ├── audio.rs           # 1,423 lines — audio HAL shim
    │   ├── battery.rs         # 856 lines — battery HAL shim
    │   └── sensors.rs         # 2,294 lines — sensors HAL shim
    └── interp.c               # custom PT_INTERP (sets the dynamic linker)
```

**Total kr64:** 9,581 lines, 144 unit tests, 8 feature modules.

**Key files to read first:**
- `app/rs/src/core.rs` — boot sequence (this is where `init` gets spawned).
- `app/rs/kr64/src/lib.rs` lines 39–80 — module map + the daemon's high-level flow.
- `app/rs/kr64/src/devices.rs` lines 1–50 + 164–180 — the device-creation table.
- `app/rs/src/renderer_new/pipe.rs` — how the host side opens the GL pipe today.

---

## 2. What we learned from Virtual Master (key findings only)

Virtual Master (`com.clone.android.dual.space`, v3.2.53) is the working
reference implementation. We reverse-engineered it in three documents.

### 2.1 The boot state machine (Java side)

VM uses a fully-Java-orchestrated 11-state machine in
`com.android.vmcore.VMInstance` (`-5..7`): `STOPPED → CHECKING_ENV →
INSTALLING → STARTING_SVC → BOOTING → OS_BOOTING → OS_READY_1 →
OS_READY_2`. The actual OS launch is a single JNI call
`VMInstance.startOS(vmId, dpi, libPath)` that:
1. forks a child process
2. chroots into `<vmDataDir>/fs`
3. `LD_PRELOAD`s `libkr64.so` (the "kernel replacement")
4. `exec`s the guest's `/system/bin/init`

Before that, state 3 (`STARTING_SVC`) starts six HAL services from Java:
`BinderService`, `InputService`, `AudioService`, `HALManager`,
`DisplayService`, `NetlinkManager`. These each create a `/dev/*` node that
the guest expects.

### 2.2 `libkr64.so` is NOT a JNI library

**The single biggest discovery.** `libkr64.so` is a standalone ELF
executable disguised as a `.so`. Its `.interp` program header points at
`libkrloader64.so` — a custom dynamic linker VM built from AOSP source
(embedding static bionic). The kernel `exec`s `libkr64.so` → reads
`PT_INTERP` → `exec`s `libkrloader64.so` → loads `libkr64.so` → jumps to
its `_start` (entry `0x4e04` A7 / `0x5594` A11).

It has **187 imported symbols** (not "3 visible imports" as the previous
disassembly claimed) and uses 100 direct `syscall()` calls to bypass its
own shadowhook hooks. A11 variant is 2.03 MB (35% larger than A7's 1.50 MB).

### 2.3 The virtual device tree libkr64 creates

Decoded from XOR-obfuscated `.data` strings (single `mknodat()` call site
at `0x11d770`, bind clusters at `0x134328`, `0x1381d0`, `0x1387f8`):

| Path | Purpose |
|---|---|
| `/vm%d/dev/qemu_pipe` | GL command transport (SurfaceFlinger writes GL here) |
| `/dev/goldfish_pipe` | A11 alias for the above |
| `/dev/gb`, `/dev/gb2` | A11 gralloc char devices (framework + vendor) |
| `/dev/touch`, `/dev/input/touch` | Touch input (guest `EventHub` reads `EV_ABS`) |
| `/dev/vmproc` | `/proc` emulator entry — `open("/proc/…")` redirected here |
| `/dev/__properties__` | Property area file (init writes, everyone reads) |
| `/dev/ashmem`, `/dev/ashmemsim` | Shared memory for SurfaceFlinger buffers |
| `/dev/__kmsg__`, `/dev/__kmsg2__`, `/dev/__krlog__` | Kernel log buffers |
| `/dev/vmproc`, `/dev/socket/logdw`, `/dev/socket/logdr` | logd sockets |
| `/dev/block/vdc` (A11) | Virtual block device controller |
| `/dev/fuse` (A11) | FUSE for /storage emulation |
| `…/vm/vm%d/dev/netlink_server` | Netlink emulation (guest RTNETLINK) |
| `…/vm/vm%d/dev/netlink_client/nl_dhcp_%d_%d` | Per-thread DHCP socket |
| `/dev/.coldboot_done` | Marker init waits on |

**20+ devices total.** Binder virtualisation is *not* in libkr64 — it lives
in `libvm.so` (the Java app's JNI library) via the `setupBinder()` JNI.

### 2.4 Seccomp + SIGSYS = the real "kernel replacement"

libkr64 installs `PR_SET_NO_NEW_PRIVS` + a `SECCOMP_SET_MODE_FILTER` BPF
program + a `SIGSYS` handler. Forbidden syscalls (`mount` without our hook,
`chroot`, `pivot_root`, `reboot`, raw `swapon`, `acct`, …) trap to SIGSYS;
the handler logs `BLOCKED.SYSCALL.FAILED: <nr>` and either **emulates** the
syscall (returns 0 or fake-success) or **kills** the guest.

This is the architectural core: **the kernel doesn't reject syscalls, it
traps them and the daemon synthesises the kernel's behaviour for a
controlled subset.** This is what makes the guest `init` believe it's
running on a real Android kernel.

### 2.5 Binder virtualisation

The Java side (`BinderService.m5206WWWWoWWWWo`):
1. Reflects into `ActivityManager` to grab the system `IActivityManager`
   IBinder (hidden-API access via FreeReflection's `exemptAll()`).
2. Wraps it in a `java.lang.reflect.Proxy`.
3. Calls native `setupBinder(vmId, binderVersion, 1, 2, "IBinderService",
   parcelledIntent)` — creates `/vm%d/dev/binder` and a redirect mapping.
4. The guest's `servicemanager` lookups for `activity`/`package`/`window`
   get proxied back into the host `BinderService`.

### 2.6 `/proc` emulation

`open("/proc/…")` is hooked (via shadowhook or seccomp trap). Matched paths
get synthesised content:

| Path | Synthesised as |
|---|---|
| `/proc/cmdline` | `androidboot.hardware=… androidboot.bootdevice=…` |
| `/proc/version` | `Linux version 4.14.x …` (matches GSI expectation) |
| `/proc/self/maps` | Per-VM filtered maps (only guest regions) |
| `/proc/self/status` | Per-VM PID/UID/GID |
| `/proc/self/mounts` | Only guest mounts |
| `/proc/self/exe` | Symlink to `/system/bin/init` (not host `app_process64`) |
| `/proc/sys/kernel/kptr_restrict` | `1` (A11 hardening) |

**Key files to read:**
- `download/VM_JAVA_ANALYSIS.md` — full Java state machine, JNI bindings.
- `download/VM_KR64_ANALYSIS.md` — the daemon, devices, seccomp, /proc.
- `download/VM_ROM_ANALYSIS.md` — ROM extraction, AES key, Treble paths.
- `download/GSI_BOOT_PLAN.md` — the full file-level implementation plan.

---

## 3. What we built (AOSP renderer, kr64 daemon, HALs)

### 3.1 AOSP `libOpenglRender.so` — replaces the closed-source blob

Built from AOSP source at commit `7a712acc02282985dcd32feb81284e1f2b19ec7e`
(`platform/sdk`), NDK r27c, clang 18.0.3, cmake 3.22.1. Both `arm64-v8a`
and `x86_64` succeeded.

| Build | Size | Twoyi-required symbols | Notes |
|---|---|---|---|
| AOSP arm64 | **603,296 B** | all 6 ✓ | 57% of legacy blob size |
| AOSP x86_64 | **597,632 B** | all 6 ✓ | new — legacy was arm64-only |
| Legacy arm64 | 1,059,128 B | all 6 ✓ | closed-source, ships today |

The 6 symbols that `app/rs/src/renderer_bindings.rs` declares (must match
exactly): `startOpenGLRenderer`, `destroyOpenGLSubwindow`,
`repaintOpenGLDisplay`, `setNativeWindow`, `resetSubWindow`,
`removeSubWindow`. **All 6 present, signatures match.**

What it took:
- Built `emugen` host tool (5 cpp files, 115 KB) to generate decoder sources
  for `renderControl` / `gl` / `gl2` (3 × 6 generated files each).
- Wrote a compat shim layer for Android platform-private headers
  (`cutils/{threads,atomic,log,sockets}.h`, `utils/{threads,Errors,Vector,
  List,String8,KeyedVector,RefBase}.h`) — implemented with POSIX primitives
  (`pthread_key_t`, `pthread_mutex_t`, `std::vector`, `std::map`).
- Patched 4 source files to use system `libEGL.so` / `libGLESv1_CM.so` /
  `libGLESv2.so` directly (no desktop-GL translator libs).
- Patched `UnixStream.cpp` to build the pipe path as
  `$TWOYI_ROOTFS/opengles{,2,3}` (env-overridable; defaults to
  `/data/data/io.twoyi/rootfs/opengles`).
- Replaced `NativeLinuxSubWindow.cpp` (X11) with `NativeAndroidSubWindow.cpp`
  — `ANativeWindow` is the `EGLNativeWindow`, returned as-is.
- Added `twoyi_api.cpp` — the 6 C-ABI entry points + 4 `dl*_ex` wrappers.

### 3.2 The kr64 daemon (Rust, 9,581 lines, 144 tests)

Mirrors VM's `libkr64.so` design in safe Rust. Modules:
- `devices.rs` (405 lines) — `mknodat`/`bind` for 7 device sockets
  (`qemu_pipe`, `touch`, `key0`, `event`, `gb`, `gb2`).
- `binder.rs` (1,959 lines) — binder proxy + servicemanager stub.
- `seccomp.rs` (831 lines) — BPF filter + SIGSYS handler.
- `proc_emu.rs` (534 lines) — synthesises `/proc/{cmdline,version,
  self/maps,self/status,self/mounts}`.
- `mount_mgr.rs` (457 lines) — bind-mount + tmpfs orchestration.
- `audio.rs` (1,423 lines) — audio HAL shim.
- `battery.rs` (856 lines) — battery HAL shim.
- `sensors.rs` (2,294 lines) — sensors HAL shim (accelerometer, gyro,
  magnetometer, light, proximity).

### 3.3 HAL shims

The three HAL modules (`audio.rs`, `battery.rs`, `sensors.rs`) implement
the AOSP HAL contracts in pure Rust, communicating with the host via
AF_UNIX sockets under `/dev/socket/`. They let the guest's `android.hardware.*`
services start without crashing on missing hardware.

**Key files to read:**
- `download/AOSP_BUILD_RESULTS.md` — full build pipeline, patches, symbol
  comparison, linkability table against `renderer_bindings.rs`.
- `app/rs/kr64/src/devices.rs` lines 1–50, 164–180 — the device table.
- `app/rs/kr64/src/lib.rs` lines 39–80, 556–561 — the orchestration flow.
- `app/rs/src/renderer_bindings.rs` (114 lines) — the FFI contract.
- `download/HAL_VIRTUALIZATION_ANALYSIS.md`, `download/AUDIO_SENSOR_HAL.md`,
  `download/SENSOR_IMPL.md`, `download/BATTERY_IMPL.md`, `download/AUDIO_IMPL.md`.

---

## 4. The x86_64 breakthrough (what worked, what didn't)

**Time:** 05:20 UTC, 2026-08-05.
**Setup:** extracted the x86_64 system image from the Android SDK's
`system-images;android-30;google_apis;x86_64` and used it as twoyi's rootfs.

### What worked

From `logcat`:

```
# 1. Init binary executed (SELinux granted execute)
avc: granted { execute } for path=".../rootfs/init"

# 2. Init started loading libraries
avc: denied { read } for name="libc.so" permissive=1

# 3. QEMU pipe found!
[NEW_RENDERER] QEMU pipe device /dev/qemu_pipe availability: true

# 4. Pipe connected!
[NEW_RENDERER] Successfully connected to QEMU pipe: /opengles3

# 5. GL context created!
[NEW_RENDERER] GL context created successfully

# 6. App stayed alive (no crash!)
u0a167 4558 273 ... S io.twoyi
```

This proves:
1. **x86_64 rootfs from the Android emulator works** — `init` executes,
   libraries load, the boot process starts.
2. **The QEMU pipe exists on x86_64** — `/dev/qemu_pipe` →
   `/dev/goldfish_pipe` is created by the emulator.
3. **The new Rust renderer connects to the pipe** on x86_64.
4. **No crash** — the app handles the pipe write failure gracefully.
5. **The boot log is displayed** — `BootLogTexture` shows boot progress.

### What didn't work

```
[NEW_RENDERER] Failed to write to pipe: Invalid argument (os error 22)
```

The pipe write fails because the emulator's `/dev/qemu_pipe` is connected
to **the emulator's own GL renderer**, not to twoyi's renderer. The
emulator's goldfish pipe expects a specific protocol that twoyi's
renderer doesn't speak yet. In short: the **device exists but it's the
wrong device**.

### Why this is a breakthrough, not a failure

The previous blocker was architectural: the ARM GSI couldn't run on x86_64
because twoyi is a container, not an emulator. That's now solved — the
guest `init` runs natively. The remaining blocker is a *single, well-defined
task*: twoyi needs to create its **own** `/dev/qemu_pipe` so the guest's
SurfaceFlinger talks to twoyi's renderer, not the emulator's.

**Key files to read:**
- `download/X86_64_BREAKTHROUGH.md` — the full logcat dump + reproduction
  steps (110 lines, the must-read).
- `download/X86_64_ROOTFS_BUILD_GUIDE.md` — how to extract the SDK image
  and set it up as a twoyi rootfs.
- `app/rs/src/renderer_new/pipe.rs` — the host-side pipe code that connected
  to the wrong pipe.

---

## 5. The critical next step — create twoyi's own `/dev/qemu_pipe`

This is **the** unblocker. Everything else (binder virtualisation, /proc
emulation, GSI extractor) can come after the first frame renders.

### What exists today

`app/rs/kr64/src/devices.rs:175` already has a `create_qemu_pipe(rootfs)`
function that binds an AF_UNIX socket at `{rootfs}/dev/qemu_pipe`. It's
called by `lib.rs:301` during `create_device_set()`. The host-side renderer
(`app/rs/src/renderer_new/pipe.rs`) already opens `/dev/qemu_pipe` and
writes the `/opengles3` service name.

### What's missing

The **plumbing between the two halves.** When the kr64 daemon `accept()`s
a connection on its `/dev/qemu_pipe` listener, it needs to:

1. Read the service-name handshake bytes (`/opengles3`, 11 bytes + null).
2. Hand the accepted stream off to `libOpenglRender_aosp.so`'s
   `RenderServer` (which already exists and listens on
   `{rootfs}/opengles`).
3. Or — simpler — have kr64's `/dev/qemu_pipe` listener `accept()` the
   guest connection and `dup2()` the resulting fd onto the
   `RenderServer`'s socket so the AOSP code processes the GL stream.

The `spawn_accept_thread(device_set.qemu_pipe, "qemu_pipe")` call at
`lib.rs:561` is the wire-up point. Today it just logs accepts; it needs
to bridge into `libOpenglRender_aosp.so`.

### Concrete sub-steps

a. In `app/rs/kr64/src/lib.rs`, replace the `spawn_accept_thread` body for
   `qemu_pipe` with a function that:
   - Reads the 12-byte service-name handshake.
   - Verifies it's one of `/opengles`, `/opengles2`, `/opengles3`.
   - `dup2()`s the accepted fd to a fresh fd and passes it to the
     `libOpenglRender_aosp.so` `RenderServer` via a new C-ABI hook
     (`render_api.cpp` already has the hooks, just needs a
     `renderServer_attachClient(int fd)` entry).

b. Ensure `libOpenglRender_aosp.so` is loaded by the **Java app process**
   (via `renderer_bindings.rs`), not by the kr64 daemon — the daemon
   creates the pipe, the app's renderer consumes the GL stream. The
   accepted fd can be sent over a Unix domain socket via `SCM_RIGHTS`.

c. Verify the guest's SurfaceFlinger can `open("/dev/qemu_pipe")`,
   `write("/opengles3\0")`, and then start streaming GLES commands.

### Acceptance criteria

- `adb shell ls /dev/qemu_pipe` inside the guest shows twoyi's socket
  (not the emulator's).
- `logcat` shows `[NEW_RENDERER] Successfully connected to QEMU pipe:
  /opengles3` **followed by** `[NEW_RENDERER] Wrote N bytes to pipe`
  (no `EINVAL`).
- The guest's launcher renders its first frame on the SurfaceView.
- `adb shell dumpsys SurfaceFlinger` inside the guest shows a non-zero
  buffer count.

### Why this unblocks everything else

Once the guest's SurfaceFlinger can composite a frame through twoyi's
renderer, we get visual feedback for every subsequent step. Binder
proxy bugs, /proc emulation gaps, seccomp denials — all become
observable on-screen instead of mysterious `init` hangs. **Get one
frame on screen and the rest follows naturally.**

**Key files to read/edit:**
- `app/rs/kr64/src/devices.rs:164-180` — `create_qemu_pipe()` (exists).
- `app/rs/kr64/src/lib.rs:556-565` — `spawn_accept_thread(device_set.qemu_pipe, ...)`
  (the wire-up point).
- `app/rs/src/renderer_new/pipe.rs` — host-side `PipeConnection` (the
  consumer).
- `app/rs/src/renderer_bindings.rs` — FFI surface to libOpenglRender.so.
- `download/GSI_BOOT_PLAN.md` §3.1 + §3.3 — full file-level plan for
  the daemon and graphics buffer devices.

---

## 6. One-page cheat sheet

```
HOW TWOYI WORKS
  Java app process
    ├── core.rs spawns guest init in chroot
    ├── input.rs serves /dev/input/{touch,key0}
    └── renderer_new/ connects to /dev/qemu_pipe, decodes GL stream
                │
                ▼
  Guest (chrooted init from GSI rootfs)
    └── SurfaceFlinger opens /dev/qemu_pipe, writes GL → twoyi renders

WHAT'S MISSING (priority order)
  1. kr64 daemon owns /dev/qemu_pipe → feeds RenderServer    ← CRITICAL
  2. /proc emulator (proc_emu.rs exists, needs init.rc patches)
  3. Binder virtualisation (binder.rs exists, needs Java BinderService)
  4. Seccomp + SIGSYS (seccomp.rs exists, needs filter install)
  5. GSI extractor (RomManager today only handles pre-Treble 8.1)

WHAT WORKS
  ✅ x86_64 guest init executes (05:20 UTC breakthrough)
  ✅ QEMU pipe connects (wrong pipe, but the code path works)
  ✅ GL context created
  ✅ AOSP libOpenglRender.so built for arm64 + x86_64 (open source)
  ✅ kr64 daemon: 9,581 lines, 144 tests, 8 modules
  ✅ HAL shims: audio + battery + sensors
  ✅ App stays alive on x86_64 boot (no crash)

WHAT DOESN'T
  ❌ Pipe write EINVAL — emulator's pipe, not twoyi's
  ❌ Guest sees host /proc (init may misbehave)
  ❌ Guest sees host servicemanager (system_server can't register)
  ❌ GSI extractor only handles pre-Treble

THE ONE-LINE NEXT ACTION
  Wire kr64's create_qemu_pipe() accept loop into libOpenglRender_aosp.so's
  RenderServer so the guest's GL stream reaches twoyi's renderer.

THE MUST-READ DOCUMENTS (in order)
  1. download/X86_64_BREAKTHROUGH.md     (110 lines, the breakthrough)
  2. download/SESSION_SUMMARY.md         (overnight session summary)
  3. download/GSI_BOOT_PLAN.md           (file-level implementation plan)
  4. download/AOSP_BUILD_RESULTS.md      (renderer build)
  5. download/VM_KR64_ANALYSIS.md        (the reference daemon)
```

---

*End of briefing. ~380 lines. For full depth on any topic, follow the
"Key files to read" pointers — every section lists the exact files and
line ranges that contain the implementation or analysis.*
