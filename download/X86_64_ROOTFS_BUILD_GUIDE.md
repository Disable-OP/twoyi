# Twoyi — x86_64 Rootfs Build Guide

> **Task ID:** ROOTFS-GUIDE-1
> **Author:** general-purpose sub-agent
> **Date:** 2026-08-05
> **Codespace:** `twoyi-dev-3-jr47xg6xvx7ghq6p`
> **Branch:** `improvements/initial-cleanup`
> **Inputs:** `default.xml` (AOSP manifest, pins `android-8.1.0_r81`), `TWOYI_HONEST_STATUS.md`, `DEVELOPMENT_ROADMAP.md` §3.2, `AOSP_BUILD_RESULTS.md` (pipe-path patch precedent), `RomManager.java` (rootfs on-disk layout).
> **Goal:** Make it possible to produce a bootable **x86_64** `rootfs.tar.gz` for twoyi, so end-to-end testing in the Android emulator / redroid x86_64 / codespace is unblocked. This is the single biggest blocker for x86_64 testing today (Roadmap item #2).

---

## 0. TL;DR — pick one of three paths

| Path | What it gives you | First-build wall-clock | Fidelity | Recommended for |
|---|---|---|---|---|
| **A. Build from AOSP source** (this guide's main path) | A rootfs whose every binary matches the twoyi architecture exactly (init, SurfaceFlinger, libc, libui, … all x86_64, all from `android-8.1.0_r81`). | 2–6 hours (one-time); ~30 min incremental | Highest | Anyone doing serious twoyi development. **Required** if you will later patch `init`, `surfaceflinger`, or `libui` for twoyi. |
| **B. Download a pre-built x86_64 GSI** | A Treble `system.img` you convert to twoyi's flat `rootfs/` layout. | 20–60 min (download + convert) | Medium — the GSI is `android-11+`, not 8.1, and ships Treble HALs twoyi's kernel-replacement daemon doesn't yet virtualise. | Quick smoke test of the boot flow; not for production. |
| **C. Cross-translate the arm64 rootfs** | Use `qemu-user-static` to run the existing arm64 `init` under binary translation. | 5 min to set up | Lowest — the renderer is x86_64 but the guest binaries are arm64. Defeats the point of going x86_64. | Only as a debugging crutch. Not covered here. |

This guide covers **Path A** in detail (§3) and **Path B** as an alternative (§4).

---

## 1. Why we need an x86_64 rootfs

This is the **single biggest blocker for x86_64 testing** of twoyi. The proof is in
[`download/TWOYI_HONEST_STATUS.md`](TWOYI_HONEST_STATUS.md) §"Why the QEMU pipe is unavailable":

> Twoyi's architecture requires the guest Android's SurfaceFlinger to communicate with
> the host renderer via `/dev/qemu_pipe`. This pipe is created by the twoyi guest's
> modified `init` process inside the rootfs.
>
> In the Android emulator:
> - The host Android (API 30, x86_64) does NOT have `/dev/qemu_pipe`
> - The twoyi guest rootfs IS extracted to `/data/data/io.twoyi/rootfs/`
> - The guest `init` binary IS arm64 (the rootfs was built for arm64)
> - The guest `init` cannot execute on x86_64 (architecture mismatch)
> - Therefore the QEMU pipe is never created
> - Therefore the renderer has nothing to connect to

Concretely, the failure sequence on x86_64 today is:

```
1. Host: twoyi APK launches, extracts rootfs.tar.gz → /data/data/io.twoyi/rootfs/
2. Host: libtwoyi.so calls Command::new("./init").spawn()   ← ./init is arm64 ELF
3. Kernel: execve("./init") returns ENOEXEC on x86_64 host   ← architecture mismatch
4. Host: renderer connects to /dev/qemu_pipe                 ← doesn't exist
5. Host: CLIENT_EGL: [NEW_RENDERER] Failed to write to pipe: Invalid argument (os error 22)
6. Host: BootLogTexture spins forever; after 60s, Render2Activity times out
```

A successful boot requires every guest binary that the host kernel will `execve` to be
a native x86_64 ELF. The minimum set is:

- `init` — the twoyi custom init at the **rootfs root** (not `/system/bin/init`)
- `/system/bin/init` — the AOSP init that the custom init exec's into
- `/system/bin/app_process64` — the zygote / ART runtime
- `/system/bin/servicemanager`, `/system/bin/surfaceflinger`, `/system/bin/logd`, …
- `/system/lib64/*.so` — bionic libc, libart, libui, libgui, libbinder, …

All of these come "for free" if you build AOSP for `x86_64` instead of `arm64`.

---

## 2. Prerequisites

### 2.1 Hardware

| Resource | Minimum | Recommended | Notes |
|---|---|---|---|
| **CPU** | 4 cores, x86_64, VT-x/AMD-V | 8+ cores | AOSP `make -jN` scales nearly linearly to ~16 cores. |
| **RAM** | 16 GB | 32 GB | Below 16 GB, the link step for `libart.so` will OOM. Add swap if you must. |
| **Disk** | 250 GB free | 500 GB SSD | A full `android-8.1.0_r81` sync is ~30 GB; a full build adds ~150 GB; `ccache` adds ~30 GB. |
| **Network** | 50 Mbit/s downlink | Gigabit | First `repo sync` downloads ~30 GB. |

### 2.2 Operating system

This guide assumes **Ubuntu 22.04 LTS** (the same OS the twoyi codespace devcontainer
uses — see `.devcontainer/Dockerfile`). Ubuntu 20.04 also works; 24.04 needs the
OpenJDK 8 backport (see §2.3).

```bash
lsb_release -a
# Distributor ID: Ubuntu
# Description:    Ubuntu 22.04.x LTS
```

### 2.3 AOSP build tools

AOSP `android-8.1.0_r81` (the tag pinned in `/home/z/my-project/default.xml` line 7)
was released in 2018 and expects an older toolchain than Ubuntu 22.04 ships by default.
Install the AOSP-required packages exactly as the upstream
[Establishing a Build Environment](https://source.android.com/setup/develop/requirements)
page lists for Ubuntu 22.04:

```bash
sudo apt-get update
sudo apt-get install -y \
    git-core gnupg flex bison gperf build-essential zip curl zlib1g-dev \
    gcc-multilib g++-multilib libc6-dev-i386 lib32ncurses-dev libtinfo5 \
    libncurses5 libx11-dev libreadline-dev libgl1-mesa-dev g++-multilib \
    mesa-common-dev tofrodos python3-markdown libxml2-utils xsltproc \
    schedtool ccache libssl-dev mtools openssh-server \
    repo simg2img img2simg ext2simg mkbootimg \
    openjdk-8-jdk
```

Notes on specific packages:

- **`openjdk-8-jdk`** — AOSP 8.1 still requires JDK 8 *for the host-side build tools*
  (`signapk`, `apicheck`, …). The output target's Java is unrelated. On Ubuntu 22.04
  this is in the `universe` repo; on 24.04 you must install it from a PPA
  (e.g. `ppa:openjdk-r/ppa`).
- **`repo`** — Google's repo manifest tool. If your distro's `repo` is too old, install
  the latest from <https://gerrit.googlesource.com/git-repo>.
- **`simg2img`** — converts Android sparse ext4 images (`system.img`) to raw ext4.
  Critical for §3.6 below. Comes from the `android-tools-fsutils` package on Ubuntu.
- **`python3-markdown`** — AOSP 8.1's build still has a few `python2` scripts. If you
  hit a `python` not found error, symlink: `sudo ln -sf /usr/bin/python3 /usr/bin/python`.

Configure `ccache`:

```bash
export USE_CCACHE=1
export CCACHE_DIR=/path/to/ccache   # put this on fast SSD, not HDD
ccache -M 50G                       # 50 GB cache; bump to 100 GB if you have space
```

Configure `git` (repo requires this):

```bash
git config --global user.name "Your Name"
git config --global user.email "you@example.com"
git config --global color.ui auto
```

### 2.4 Twoyi source tree (for the patches)

You need the twoyi repo on the build host so you can copy the patch files into the
AOSP tree. Either clone it:

```bash
git clone https://github.com/Disable-OP/twoyi.git /path/to/twoyi
cd /path/to/twoyi
git checkout improvements/initial-cleanup
```

Or just keep using the local checkout at `/home/z/my-project/`.

The repo provides:

- `default.xml` — the **exact** AOSP manifest to use (pins `android-8.1.0_r81`).
  This is the manifest you should `repo init -m` against, *not* the upstream default
  `default.xml` from googlesource.
- `app/src/main/jniLibs/x86_64/libOpenglRender_aosp.so` — the AOSP-built
  host-side GL renderer (597 KB, see `AOSP_BUILD_RESULTS.md`). This is the *host-side*
  renderer; the *guest-side* `libOpenglRender.so` shim inside the rootfs is built by
  your AOSP build below.
- `app/rs/src/renderer_new/pipe.rs` — the Rust code that opens `/dev/qemu_pipe` and
  speaks the emugl wire protocol. Useful to read so you understand what the guest's
  `SurfaceFlinger` must do on the other side.

---

## 3. Step-by-step build process (Path A)

This section builds a complete x86_64 userland from `android-8.1.0_r81` and packages
it as a `rootfs.tar.gz` that twoyi's `RomManager` will accept.

### 3.1 Lay out the workspace

```bash
mkdir -p ~/aosp && cd ~/aosp

# Twoyi patches live here (we'll create them in §3.4):
mkdir -p patches/{system,core,frameworks}
```

### 3.2 `repo init` — use the twoyi manifest

You have two equivalent options. **Option 1 is preferred** because it pins the exact
commit tree that twoyi was developed against.

**Option 1 — use the manifest from the twoyi repo (recommended):**

```bash
cd ~/aosp
# Copy the recovered manifest from the twoyi repo:
cp /home/z/my-project/default.xml ./.local-manifest.xml
# Trick repo into using it: `repo init -m` reads the named manifest from .repo/manifests/
mkdir -p .repo/manifests
cp /home/z/my-project/default.xml .repo/manifests/twoyi.xml
repo init -u https://android.googlesource.com/platform/manifest -m twoyi.xml
```

The twoyi manifest's `<default>` element (line 7–9 of `default.xml`) is:

```xml
<default revision="refs/tags/android-8.1.0_r81"
         remote="aosp"
         sync-j="4" />
```

This pins every project to the `android-8.1.0_r81` tag — the build label Google
shipped as the official Android 8.1.0 release for Pixel 2 / Pixel 2 XL.

**Option 2 — use the upstream manifest at the same tag:**

```bash
cd ~/aosp
repo init -u https://android.googlesource.com/platform/manifest -b android-8.1.0_r81
```

This is functionally identical because the twoyi `default.xml` was generated from
this exact tag — but Option 1 lets you diff against future manifest changes.

### 3.3 `repo sync -c -j8`

```bash
repo sync -c -j8
```

Flags:

- `-c` — checkout the *current* branch (faster; skips fetching other heads).
- `-j8` — 8 parallel fetches. Bump to `-j16` on a Gigabit link; drop to `-j4` on slow
  networks. Google rate-limits per-IP, so very high `-j` often backfires.

Expected output: ~30 GB downloaded, ~1,100 git repositories checked out. Wall-clock:
30 min on Gigabit, 2–4 hours on a 50 Mbit/s link.

If a project fails to sync (network blip), just re-run `repo sync -c -j8` — `repo`
will resume where it left off.

After sync, sanity-check the tree:

```bash
# Confirm the build tag is what we expect:
cd build/make && git describe --tags && cd -
# Expect: android-8.1.0_r81

# Confirm the x86_64 device target exists:
ls device/generic/x86_64
# Expect: AndroidBoard.mk  AndroidProducts.mk  Box.mk  ...  vendor/
```

### 3.4 Apply twoyi-specific patches

These are the changes that make the AOSP userland talk to twoyi's host renderer
instead of to a real kernel/emulator. There are four patches; each is small.

#### 3.4.1 Patch 1 — `init` creates `/dev/qemu_pipe` early in boot

Twoyi's host renderer (`libOpenglRender_aosp.so`) listens on a Unix domain socket at
`/dev/qemu_pipe` inside the guest rootfs. The guest's `SurfaceFlinger` connects to
this socket and writes emugl wire-protocol GLES commands.

The guest `init` must `mknod`/`bind-mount` this socket *before* `surfaceflinger`
starts. Add a new early-init action to `system/core/init/init.rc`:

```bash
cd ~/aosp
cat > patches/system/init_qemu_pipe.rc <<'EOF'
# Twoyi: create /dev/qemu_pipe as a Unix domain socket for the host renderer.
# The socket is abstract; the host (libOpenglRender_aosp.so) bind-listens here.
on early-init
    mkdir /dev/socket 0777 root root
    start twoyi_qemu_pipe_setup

service twoyi_qemu_pipe_setup /system/bin/twoyi_pipe_setup
    class core
    user root
    group root
    oneshot
    disabled
EOF

cat > patches/system/twoyi_pipe_setup.c <<'EOF'
// Tiny helper that creates the /dev/qemu_pipe Unix socket symlink
// pointing at the abstract-namespace socket the host renderer binds.
// (Equivalent to what real QEMU does on the kernel side, but in
// userspace because twoyi has no kernel of its own.)
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
#include <stdio.h>
#include <string.h>
#include <errno.h>
#include <fcntl.h>

int main(void) {
    // Create a filesystem-backed Unix socket the host will bind to.
    // The host (libOpenglRender_aosp.so) does bind() first; we just
    // pre-create the path so SurfaceFlinger's connect() succeeds.
    int fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (fd < 0) { perror("socket"); return 1; }

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, "/dev/qemu_pipe", sizeof(addr.sun_path) - 1);

    // If the socket already exists, unlink it (leftover from previous boot).
    unlink("/dev/qemu_pipe");

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("bind /dev/qemu_pipe");
        // Fall through — the host may have already bound it.
    }
    listen(fd, 4);
    close(fd);

    // Mark the pipe as "QEMU pipe" magic — guest code checks this signature
    // via the "qemu_pipe" device path and refuses to connect if absent.
    int magic = open("/dev/qemu_pipe", O_WRONLY | O_CREAT, 0666);
    if (magic >= 0) {
        write(magic, "QEMU", 4);
        close(magic);
    }
    return 0;
}
EOF

# Drop the .rc file into the device init dir that gets installed:
cp patches/system/init_qemu_pipe.rc system/core/rootdir/init.rc
cp patches/system/twoyi_pipe_setup.c system/core/init/twoyi_pipe_setup.cpp
```

You'll also need to add a one-line `Android.bp` entry to compile
`twoyi_pipe_setup.cpp` (look at how `/system/bin/init` itself is declared in
`system/core/init/Android.bp` for a template).

> **Alternative:** skip the C helper and use the `init.rc` `mkdir` + `chmod`
> directives only — the host renderer can create the socket itself with a small
> change to `libOpenglRender_aosp.so` (see `AOSP_BUILD_RESULTS.md` §5.3 for the
> `UnixStream.cpp` patch that controls the path). The guest then just needs
> `/dev/qemu_pipe` to exist as an empty file. This is what the cyanmint arm64
> rootfs does.

#### 3.4.2 Patch 2 — SurfaceFlinger uses the qemu_pipe renderer

SurfaceFlinger picks its rendering backend via a build property. In AOSP 8.1 the
default is `ro.surface_flinger.use_gl=0` (HWC path), which needs a real hardware
composer HAL. Twoyi has no HWC, so we force the GL/emugl path.

Edit `frameworks/native/services/surfaceflinger/SurfaceFlinger.cpp` and add at the
top of `SurfaceFlinger::init()`:

```cpp
// Twoyi: force the GL ES renderer over the QEMU pipe. There is no HWC.
char val[PROPERTY_VALUE_MAX];
property_set("ro.surface_flinger.use_gl", "1");
property_set("ro.surface_flinger.use_hwc", "0");
property_set("debug.egl.hwcomposer", "0");
property_set("ro.hardware.egl", "emugl");
```

(The cyanmint arm64 rootfs does the equivalent by editing `system/build.prop`
post-extract — see `BuildVMPropTask.java` in VM's decompiled sources — but doing
it at AOSP build time is cleaner.)

#### 3.4.3 Patch 3 — disable SELinux enforcing

Twoyi runs as an unprivileged app; it cannot set SELinux contexts on the guest
filesystem. The guest `init` would fail to relabel `/dev`, `/system`, etc. Force
permissive mode by editing `system/core/init/init.cpp`:

```cpp
// Twoyi: we run inside an unprivileged host app — no SELinux.
// Skip selinux_init() and force permissive.
selinux_setenforce(0);
// Comment out:  selinux_init_selinux_handle();
//               selinux_set_policyload();
```

(Equivalent to the kernel cmdline `androidboot.selinux=permissive`, but twoyi has
no kernel cmdline.)

#### 3.4.4 Patch 4 — `ro.build.fingerprint` is consistent

Twoyi's host-side `BuildVMPropTask.java` (decompiled from Virtual Master) rewrites
`/system/build.prop` post-extract to make the fingerprint match the host's. To
avoid that step entirely, set the fingerprint at build time:

```bash
# In ~/aosp/build/make/tools/buildinfo.sh, prepend:
cat >> build/make/tools/buildinfo.sh.twoyi_override <<'EOF'
ro.build.fingerprint=twoyi/twoyi/twoyi:8.1.0/$(shell date +%Y%m%d)/1
ro.bootimage.build.fingerprint=twoyi/twoyi/twoyi:8.1.0/$(shell date +%Y%m%d)/1
ro.product.cpu.abi=x86_64
ro.product.cpu.abilist=x86_64,x86
EOF
```

### 3.5 `lunch sdk_gphone_x86_64-userdebug`

```bash
. build/envsetup.sh

# List x86_64 lunch targets:
lunch | grep x86_64

# Pick the SDK phone x86_64 target — this is the closest to twoyi's
# "Android 8.1 userland running on a virtual device" use case:
lunch sdk_gphone_x86_64-userdebug
```

> **Why `sdk_gphone_x86_64` and not `aosp_x86_64`?**
>
> The `sdk_gphone_*` targets include the goldfish HALs (`device/generic/goldfish-opengl`),
> which speak the QEMU pipe protocol that twoyi's renderer also speaks. The bare
> `aosp_x86_64` target omits them and you'd have to add them back as a separate
> `device/generic/goldfish-opengl` build.
>
> Other valid choices:
> - `aosp_x86_64-userdebug` — minimal AOSP; you'll need to add `goldfish-opengl` yourself.
> - `sdk_phone_x86_64-userdebug` — same as `sdk_gphone_x86_64` but for the non-goldfish
>   "phone" form factor.

Sanity-check the chosen target:

```bash
printconfig
# Expect:
#   TARGET_PRODUCT=sdk_gphone_x86_64
#   TARGET_BUILD_VARIANT=userdebug
#   TARGET_BUILD_TYPE=release
#   TARGET_ARCH=x86_64
#   TARGET_ARCH_VARIANT=x86_64
#   TARGET_CPU_ABI=x86_64
#   TARGET_CPU_ABI_LIST_64_BIT=x86_64
```

If `TARGET_ARCH` is not `x86_64`, your `lunch` was wrong — re-run it.

### 3.6 `make -j8`

```bash
make -j8
```

Build targets worth knowing:

- `make -j8` (no arg) — full build. Produces `out/target/product/sdk_gphone_x86_64/system.img`,
  `userdata.img`, `ramdisk.img`, and `boot.img` (the latter two we don't need for twoyi).
- `make snod -j8` — "system image no dependency": rebuilds `system.img` from the
  current `out/.../system/` tree without re-running the C++/Java compilation. Fast
  iteration when you've only changed `init.rc` or `.prop` files.
- `make init -j8` — builds just `/system/bin/init`. Useful when iterating on Patch 1
  above.

Wall-clock expectations:

| Hardware | First full `make -j8` | Incremental `make snod` |
|---|---|---|
| 4 cores, 16 GB, HDD | 6–8 hours | 2–3 min |
| 8 cores, 32 GB, SSD | 2–4 hours | 1–2 min |
| 16 cores, 64 GB, NVMe | 1.5–2.5 hours | <1 min |

The link step for `libart.so` is the heaviest single task — it can use 12+ GB RAM
briefly. If `make` dies with `collect2: ld terminated with signal 9 (SIGKILL)`,
add swap:

```bash
sudo fallocate -l 16G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### 3.7 Extract the system image

After a successful `make`, the system image is at:

```bash
ls -lh out/target/product/sdk_gphone_x86_64/system.img
# -rw-rw-r-- 1 user user 1.1G ... system.img
file out/target/product/sdk_gphone_x86_64/system.img
# out/.../system.img: Android sparse image, ...
```

It's an Android sparse image — convert it to raw ext4:

```bash
simg2img out/target/product/sdk_gphone_x86_64/system.img system.raw.img
file system.raw.img
# system.raw.img: Linux rev 1.0 ext4 filesystem data, ...
```

Mount and copy the contents out:

```bash
mkdir -p /mnt/system
sudo mount -o loop,ro system.raw.img /mnt/system
ls /mnt/system
# Expect: bin/  build.prop  etc/  framework/  lib64/  priv-app/  ...

# Copy the system partition into the rootfs skeleton:
mkdir -p ~/twoyi-rootfs
sudo cp -a /mnt/system/. ~/twoyi-rootfs/system/
sudo umount /mnt/system
```

### 3.8 Package as `rootfs.tar.gz` matching twoyi's layout

Twoyi's `RomManager.romExist()` checks for `init` at the **rootfs root** (not
`/system/bin/init`) — see `app/src/main/java/io/twoyi/utils/RomManager.java:182`:

```java
public static boolean romExist(Context context) {
    File initFile = new File(getRootfsDir(context), "init");
    return initFile.exists();
}
```

So the rootfs layout twoyi expects is:

```
rootfs/                 ← what gets extracted to /data/data/io.twoyi/rootfs/
├── init                ← custom twoyi init (exec's /system/bin/init after setup)
├── rom.ini             ← ROM metadata (see §3.8.3)
├── system/             ← the AOSP system partition you just extracted
│   ├── bin/
│   │   └── init        ← the AOSP init that init (above) exec's into
│   ├── build.prop
│   ├── etc/
│   ├── framework/
│   ├── lib64/          ← x86_64 .so files
│   ├── priv-app/
│   └── ...
├── vendor/             ← empty or stub (twoyi doesn't use a vendor partition yet)
├── data/               ← writable; created empty
│   └── local/tmp/      ← created by RomManager.ensureDataLocalTmp()
├── dev/
│   ├── input/          ← created by RomManager.ensureBootFiles()
│   ├── socket/         ← created by RomManager.ensureBootFiles()
│   └── maps/           ← created by RomManager.ensureBootFiles()
└── sdcard/             ← created empty
```

#### 3.8.1 Build the custom twoyi `init` binary

The twoyi custom `init` at the rootfs root is a tiny C program that:

1. `mknod`s `/dev/null`, `/dev/zero`, `/dev/random`, `/dev/urandom`, `/dev/ptmx`,
   `/dev/tty`, `/dev/qemu_pipe`, `/dev/binder`, `/dev/ashmem`, `/dev/__properties__`.
2. Sets up `/proc` and `/sys` (mounts or symlinks to host's, depending on whether
   the `kr64` daemon is in use).
3. Sets a few environment variables (`ANDROID_ROOT=/system`, `ANDROID_DATA=/data`,
   `BOOTCLASSPATH=...`).
4. Exec's `/system/bin/init` with the standard Android init args.

The legacy cyanmint arm64 rootfs ships this as a 64-bit PIE binary. For x86_64 you
build it the same way, just with a different target. A minimal version:

```c
// twoyi_init.c — compile with:
//   gcc -static -o twoyi_init twoyi_init.c
// then rename to `init` and place at the rootfs root.
#define _GNU_SOURCE
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

static void mknod_safe(const char *path, mode_t mode, dev_t dev) {
    if (mknod(path, mode | 0666, dev) < 0 && errno != EEXIST) {
        fprintf(stderr, "mknod %s: %s\n", path, strerror(errno));
    }
}

int main(int argc, char **argv) {
    setenv("ANDROID_ROOT",  "/system", 1);
    setenv("ANDROID_DATA",  "/data",   1);
    setenv("PATH", "/system/bin:/system/xbin:/vendor/bin", 1);

    // Create essential device nodes (or rely on kr64 daemon to create them).
    mkdir("/dev",    0755);
    mkdir("/dev/socket", 0755);
    mkdir("/dev/input",  0755);
    mkdir("/proc",   0555);
    mkdir("/sys",    0555);

    mknod_safe("/dev/null",     S_IFCHR, makedev(1, 3));
    mknod_safe("/dev/zero",     S_IFCHR, makedev(1, 5));
    mknod_safe("/dev/random",   S_IFCHR, makedev(1, 8));
    mknod_safe("/dev/urandom",  S_IFCHR, makedev(1, 9));
    mknod_safe("/dev/ptmx",     S_IFCHR, makedev(5, 2));
    mknod_safe("/dev/tty",      S_IFCHR, makedev(5, 0));

    // qemu_pipe: created by libOpenglRender_aosp.so (host side); we just
    // make sure the /dev directory exists. See Patch 1 in §3.4.1 if you
    // prefer to pre-create the socket path here.

    // Mount /proc and /sys (or, if running under the kr64 daemon, those
    // mounts are emulated).
    mount("proc",     "/proc", "proc",     0, NULL);
    mount("sysfs",    "/sys",  "sysfs",    0, NULL);
    mount("devpts",   "/dev/pts", "devpts", 0, NULL);
    mount("tmpfs",    "/dev/socket", "tmpfs", 0, "mode=0777");

    // Hand off to the real AOSP init:
    char *init_args[] = { "/system/bin/init", NULL };
    execv(init_args[0], init_args);
    perror("exec /system/bin/init");
    return 1;
}
```

Build it:

```bash
gcc -static -O2 -o ~/twoyi-rootfs/init twoyi_init.c
file ~/twoyi-rootfs/init
# Expect: ... ELF 64-bit LSB executable, x86-64, version 1 (SYSV), statically linked, ...
```

#### 3.8.2 Create `rom.ini`

```bash
cat > ~/twoyi-rootfs/rom.ini <<'EOF'
author=twoyi-x86_64-build
version=android-8.1.0_r81
code=81000081
md5=00000000000000000000000000000000
desc=x86_64 AOSP 8.1.0_r81 built for twoyi (Rootfs Build Guide)
EOF
```

(`code` is an integer version code — pick a sensible scheme; `md5` can be left as
zeros — `RomManager` uses it only to detect changes.)

#### 3.8.3 Create the empty writable directories

```bash
mkdir -p ~/twoyi-rootfs/{data/local/tmp,dev/{input,socket,maps},sdcard,vendor}
chmod 0777 ~/twoyi-rootfs/data/local/tmp
```

#### 3.8.4 Tar it up

```bash
cd ~/twoyi-rootfs
sudo tar --owner=0 --group=0 --numeric-owner -czf ~/rootfs.tar.gz .
ls -lh ~/rootfs.tar.gz
# Expect: ~400-800 MB depending on what got built.
```

Verify the contents:

```bash
tar tzf ~/rootfs.tar.gz | head -20
# Expect:
# ./
# ./init
# ./rom.ini
# ./system/
# ./system/bin/
# ./system/bin/init
# ./system/bin/linker64           <-- the x86_64 dynamic linker
# ./system/bin/servicemanager
# ./system/bin/surfaceflinger
# ./system/bin/app_process64
# ./system/lib64/
# ./system/lib64/libc.so
# ./system/lib64/libart.so
# ./system/lib64/libui.so
# ./system/lib64/libgui.so
# ./system/lib64/libbinder.so
# ...

# Confirm the init binary is x86_64:
mkdir /tmp/verify-init && tar xzf ~/rootfs.tar.gz -C /tmp/verify-init ./init
file /tmp/verify-init/init
# Expect: ELF 64-bit LSB executable, x86-64, ...
```

You now have `~/rootfs.tar.gz` — drop it into the twoyi APK build as
`app/src/main/assets/rootfs.tar.gz` (or push it to a running device and use the
"Import ROM" flow).

---

## 4. Alternative: use a pre-built x86_64 GSI (Path B)

If you don't want to do a full AOSP build, you can download a pre-built Treble GSI
and convert it. This is faster but lower fidelity: GSIs are built for newer Android
versions (10+) and assume Treble HALs that twoyi's kernel-replacement daemon
doesn't yet virtualise.

### 4.1 Where to get an x86_64 GSI

| Source | URL | Notes |
|---|---|---|
| **Google GSI downloads** (official) | <https://developers.google.com/android/images> | Look under "Treble GSI images" for the latest `system-*.img`. These are signed by Google. |
| **AOSP CI** | <https://ci.android.com/builds/branches/aosp-master/grid> | Search for `aosp_x86_64-userdebug`. Bleeding-edge, may be unstable. |
| **Android GSI documentation** | <https://source.android.com/docs/core/ota/gsi> | Background reading; explains what a GSI is and which one to pick for your device. |

For twoyi, pick:

- **Architecture**: `x86_64`
- **Variant**: `userdebug` (so you have `adb root`)
- **Android version**: ideally **8.1** to match `default.xml`. Google does not ship
  8.1 GSIs anymore — the closest signed GSI is `android-9.0_r46` (`system-x86_64.img`).
  If you can tolerate a version mismatch, an Android 11 GSI (`RQ3A.211001.001`) works
  too but expect more HALs to fail.

Example download:

```bash
# Android 11 GSI, x86_64, userdebug:
curl -L -o gsi.zip \
  https://dl.google.com/developers/android/r11/images/gsi/aosp_x86_64-img-211001001.zip
unzip gsi.zip
ls
# Expect: system.img   (sparse ext4, ~1.5 GB)
```

### 4.2 Convert the GSI to a twoyi rootfs

The GSI ships only `system.img` — no `vendor.img`, no `boot.img`, no `init` at the
root. To turn it into a twoyi rootfs:

```bash
mkdir -p ~/twoyi-gsi-rootfs
cd ~/twoyi-gsi-rootfs

# 1. Convert sparse → raw:
simg2img ../system.img system.raw.img

# 2. Mount and copy out the system partition:
mkdir -p /mnt/system
sudo mount -o loop,ro system.raw.img /mnt/system
sudo cp -a /mnt/system/. system/
sudo umount /mnt/system

# 3. Apply the same patches as §3.4 (init.rc, build.prop, SurfaceFlinger).
#    For a GSI these must be applied post-extract because you didn't build it:
sudo sed -i 's/ro.surface_flinger.use_gl=0/ro.surface_flinger.use_gl=1/' \
    system/build.prop
echo 'ro.surface_flinger.use_hwc=0' | sudo tee -a system/build.prop
echo 'debug.egl.hwcomposer=0'       | sudo tee -a system/build.prop
echo 'ro.hardware.egl=emugl'        | sudo tee -a system/build.prop
sudo setenforce 0 2>/dev/null || true  # no-op on host; guest init does it

# 4. Build the custom twoyi init binary (same as §3.8.1):
gcc -static -O2 -o init twoyi_init.c   # same source as §3.8.1

# 5. Create empty writable dirs + rom.ini (same as §3.8.2 and §3.8.3):
mkdir -p {data/local/tmp,dev/{input,socket,maps},sdcard,vendor}
echo -e "author=twoyi-gsi-x86_64\nversion=android-11-gsi\ncode=11000000\nmd5=0\ndesc=x86_64 GSI converted for twoyi" > rom.ini

# 6. Tar it:
sudo tar --owner=0 --group=0 --numeric-owner -czf ~/rootfs.tar.gz .

# 7. Sanity-check:
file init    # must be x86-64
ls system/bin/init system/bin/app_process64 system/lib64/libc.so
```

### 4.3 Caveats with the GSI path

- **No matching `vendor.img`** — the GSI expects Treble HALs that twoyi doesn't yet
  provide. `init` will log many "service ... not found" errors. Some are fatal
  (e.g. `gatekeeperd` missing → boot stalls at "Decrypting /data").
- **Android 10+ requires APEX** — `system/apex/com.android.*` are mini-images that
  `apexd` must mount. Either pre-extract them (set `ro.apex.updatable=false` and
  bind-mount each APEX's `apex_payload.img`) or skip by patching `init.rc`.
- **Different bionic ABI** — the GSI's `libc.so` was built for Android 11; twoyi's
  host kernel is whatever the emulator provides. Usually fine, but watch for
  `getrandom()` / `memfd_create()` syscall-number mismatches.

For the **first end-to-end smoke test** of twoyi on x86_64, the GSI path is fine.
For ongoing development, build from AOSP source (§3) so you have full control.

---

## 5. How to test

Once you have `rootfs.tar.gz`, verify it end-to-end.

### 5.1 Verify the init binary is x86_64 (5 seconds)

```bash
mkdir /tmp/rootfs-check && tar xzf ~/rootfs.tar.gz -C /tmp/rootfs-check ./init
file /tmp/rootfs-check/init
# Expect: ELF 64-bit LSB executable, x86-64, version 1 (SYSV), statically linked, ...

# Also check the AOSP init and a representative .so:
tar xzf ~/rootfs.tar.gz -C /tmp/rootfs-check \
    ./system/bin/init ./system/lib64/libc.so
file /tmp/rootfs-check/system/bin/init
# Expect: ELF 64-bit LSB shared object, x86-64, ...
file /tmp/rootfs-check/system/lib64/libc.so
# Expect: ELF 64-bit LSB shared object, x86-64, ...
```

If any of these say `aarch64` or `ARM`, your lunch target was wrong — go back to §3.5.

### 5.2 Push the rootfs into the twoyi APK build

Two options:

**Option A — bundle into the APK at build time:**

```bash
cp ~/rootfs.tar.gz /home/z/my-project/app/src/main/assets/rootfs.tar.gz
cd /home/z/my-project
./gradlew assembleRelease -Pabis=x86_64
ls -lh app/build/outputs/apk/release/*.apk
```

**Option B — push to a running device and use the "Import ROM" flow:**

```bash
adb push ~/rootfs.tar.gz /sdcard/Download/rootfs.tar.gz
# In the twoyi app: Settings → "Import ROM" → pick /sdcard/Download/rootfs.tar.gz
```

### 5.3 Boot the emulator and watch the boot log

```bash
# Start an x86_64 emulator (if not already running):
sdkmanager "system-images;android-30;google_apis;x86_64"
avdmanager create avd -n twoyi_x86_64 -k "system-images;android-30;google_apis;x86_64" -d pixel_5
emulator -avd twoyi_x86_64 -no-window -no-audio &

adb wait-for-device
adb install -r -t /home/z/my-project/app/build/outputs/apk/release/*.apk
adb shell am start -n io.twoyi/.ui.SettingsActivity

# Watch for the boot progress:
adb logcat -s CLIENT_EGL:V RomManager:V TwoyiApplication:V
```

Success criteria (in order of how far the boot gets):

1. `RomManager: romExist=true` — twoyi found the new `init`.
2. `CLIENT_EGL: [NEW_RENDERER] GL context created successfully` — host renderer up.
3. `CLIENT_EGL: [NEW_RENDERER] Connected to /dev/qemu_pipe` — **the big one**. This
   means your guest `init` ran and created the pipe. If you see this, you've solved
   the blocker.
4. `init: starting service 'surfaceflinger'...` — the guest's init is making progress.
5. `TwoyiMessenger: BOOT_COMPLETED` — the guest's `system_server` is up.

If you stop at step 2 with `Failed to write to pipe: Invalid argument`, the guest
`init` either didn't run (re-check §3.8.1 / `file init`) or didn't create the pipe
(re-check §3.4.1).

### 5.4 `adb shell` into the guest (advanced)

Twoyi runs `adbd` inside the guest on TCP 22122. The host app's `libadb.so` connects
to it. From your host:

```bash
adb forward tcp:22122 tcp:22122
adb -s localhost:22122 shell
# You're now inside the guest. Sanity-check:
getprop ro.product.cpu.abi      # should be x86_64
uname -a                         # should show the host kernel
ls /dev/qemu_pipe                # should exist
```

---

## 6. What modifications are needed — summary

This is the consolidated list of every change you must make beyond a stock AOSP
build. Each is referenced from the section where it's described in detail.

| # | What | Where | Section |
|---|---|---|---|
| 1 | `init` creates `/dev/qemu_pipe` early in boot | `system/core/init/init.rc` + `twoyi_pipe_setup.cpp` | §3.4.1 |
| 2 | SurfaceFlinger forces GL/emugl renderer, no HWC | `frameworks/native/services/surfaceflinger/SurfaceFlinger.cpp` or `system/build.prop` | §3.4.2 |
| 3 | SELinux set to permissive (no kernel to enforce it) | `system/core/init/init.cpp` | §3.4.3 |
| 4 | `ro.build.fingerprint` set at build time | `build/make/tools/buildinfo.sh` | §3.4.4 |
| 5 | Custom twoyi `init` binary at rootfs root | new file `init` (compiled from `twoyi_init.c`) | §3.8.1 |
| 6 | `rom.ini` metadata file | new file `rom.ini` | §3.8.2 |
| 7 | Empty writable dirs: `data/`, `dev/`, `sdcard/`, `vendor/` | new directories | §3.8.3 |

Additionally, the host-side `libOpenglRender_aosp.so` was already patched (in
`AOSP_BUILD_RESULTS.md` §5.3) to bind the qemu_pipe socket at
`$TWOYI_ROOTFS/opengles` — that's the *host*-side counterpart of modification #1.
The two sides meet at the `/dev/qemu_pipe` socket path.

For reference, Virtual Master's `libkr64.so` does all of the above (and much more)
at runtime — see `download/VM_KR64_ANALYSIS.md` §4.2 for the full device inventory
that `libkr64` materialises (binder, qemu_pipe, gb, gb2, touch, vmproc, ashmem,
__properties__, netlink, kmsg, etc.). Twoyi's `kr64` Rust skeleton
(`app/rs/kr64/`) covers the first 6 of those today; the rest are Roadmap items.

---

## 7. Estimated time

| Step | Wall-clock (8-core / 32 GB / SSD) | Wall-clock (4-core / 16 GB / HDD) |
|---|---|---|
| §2.2 Install AOSP build tools | 10 min | 10 min |
| §3.2 `repo init` | 1 min | 1 min |
| §3.3 `repo sync -c -j8` | 30 min | 2–4 hours |
| §3.4 Apply twoyi patches | 30 min | 30 min |
| §3.5 `lunch sdk_gphone_x86_64-userdebug` | 1 min | 1 min |
| §3.6 `make -j8` (first build) | **2–4 hours** | **6–8 hours** |
| §3.7 Extract system image | 5 min | 5 min |
| §3.8 Package as rootfs.tar.gz | 15 min | 15 min |
| §5 Test on emulator | 10 min | 10 min |
| **Total** | **3.5–5.5 hours** | **9–13 hours** |

After the first build, incremental rebuilds (`make snod -j8`) take 1–2 minutes,
so iterating on the init patches is fast.

The GSI path (§4) skips the build entirely:

| Step | Wall-clock |
|---|---|
| §4.1 Download GSI | 10–30 min |
| §4.2 Convert to twoyi rootfs | 15 min |
| §5 Test on emulator | 10 min |
| **Total** | **35–55 min** |

---

## 8. Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `make` fails with `unsupported reloc 256 against _GLOBAL_OFFSET_TABLE_` | Your host `gcc` is too new (Ubuntu 24.04 ships gcc-13; AOSP 8.1 expects gcc-9 or earlier). | Install an older GCC via `apt install gcc-9 g++-9` and `export HOST_CC=gcc-9 HOST_CXX=g++-9`. |
| `make` fails with `Java version 17 found, expected 8` | OpenJDK 8 not on PATH. | `sudo apt install openjdk-8-jdk` then `export JAVA_HOME=/usr/lib/jvm/java-8-openjdk-amd64`. |
| `make` dies with `ld: fatal error: out of memory` | The `libart.so` link step needs >12 GB RAM. | Add 16 GB swap (§3.6), or `export ART_BUILD_HOST_DEBUG=false` to skip debug symbols. |
| `simg2img` not found | The `android-tools-fsutils` package isn't installed. | `sudo apt install android-tools-fsutils`. |
| `repo init` fails with `fatal: manifest missing` | You didn't copy `default.xml` into `.repo/manifests/` (Option 1 in §3.2). | Follow Option 1's `mkdir -p .repo/manifests && cp ... .repo/manifests/twoyi.xml` step. |
| Guest `init` runs but `/dev/qemu_pipe` never appears | The twoyi custom `init` (§3.8.1) didn't create it, and Patch 1 (§3.4.1) wasn't applied. | Apply Patch 1, OR have the host `libOpenglRender_aosp.so` create the socket itself (see note in §3.4.1). |
| Guest boots to "Decrypting /data" and stalls | The GSI's `gatekeeperd` expects a real gatekeeper HAL. | Build from AOSP source (§3) instead of using a GSI; OR set `ro.crypto.state=unsupported` in `system/build.prop`. |
| `adb -s localhost:22122` times out | The guest `adbd` didn't start, OR the host's `libadb.so` (closed-source) failed to connect. | Check `adb logcat \| grep -i adbd`; the closed-source `libadb.so` is Roadmap item #10 to replace. |

---

## 9. References

- `/home/z/my-project/default.xml` — the AOSP manifest pinning `android-8.1.0_r81`.
- `/home/z/my-project/download/TWOYI_HONEST_STATUS.md` — the root cause analysis of the
  x86_64 boot failure (the "why" of this guide).
- `/home/z/my-project/download/AOSP_BUILD_RESULTS.md` §5.3 — the precedent for the
  pipe-path patch (the host-side `UnixStream.cpp` patch that controls where
  `libOpenglRender_aosp.so` listens for the qemu_pipe).
- `/home/z/my-project/download/DEVELOPMENT_ROADMAP.md` §3.2 — the two-path plan
  (build from source vs. use GSI) this guide implements.
- `/home/z/my-project/download/GSI_BOOT_PLAN.md` — the deeper architectural plan for
  booting a real Treble GSI inside twoyi (kernel-replacement daemon, binder
  virtualisation, etc.). Read this once you've got a basic x86_64 rootfs booting and
  want to push further.
- `/home/z/my-project/app/src/main/java/io/twoyi/utils/RomManager.java` — defines the
  rootfs on-disk layout twoyi expects (the `init` at rootfs root, the `system/`,
  `vendor/`, `data/`, `dev/`, `sdcard/` subdirs, the `rom.ini` format).
- `/home/z/my-project/app/src/main/jniLibs/x86_64/libOpenglRender_aosp.so` — the
  pre-built host-side renderer (597 KB). Drop into the APK as-is; no rebuild needed.
- AOSP upstream docs:
  - [Establishing a Build Environment](https://source.android.com/setup/develop/requirements)
  - [Downloading the Source](https://source.android.com/setup/build/downloading)
  - [Preparing to Build](https://source.android.com/setup/build/building)
  - [GSI images](https://source.android.com/docs/core/ota/gsi)

---

## 10. Checklist (print this and tick off as you go)

- [ ] Hardware: ≥4 cores, ≥16 GB RAM, ≥250 GB disk.
- [ ] Ubuntu 22.04 + AOSP build packages installed (§2.3).
- [ ] `ccache -M 50G` and `git config --global user.email` done.
- [ ] `repo init -m twoyi.xml` (using `default.xml` from the twoyi repo).
- [ ] `repo sync -c -j8` completed (~30 GB).
- [ ] `build/make && git describe --tags` reports `android-8.1.0_r81`.
- [ ] Patch 1 applied — `init` creates `/dev/qemu_pipe` (§3.4.1).
- [ ] Patch 2 applied — SurfaceFlinger forces GL/emugl (§3.4.2).
- [ ] Patch 3 applied — SELinux permissive (§3.4.3).
- [ ] Patch 4 applied — `ro.build.fingerprint` set (§3.4.4).
- [ ] `. build/envsetup.sh && lunch sdk_gphone_x86_64-userdebug` succeeds.
- [ ] `printconfig` shows `TARGET_ARCH=x86_64`.
- [ ] `make -j8` completed; `out/.../system.img` exists.
- [ ] `simg2img` converted to raw ext4 and mounted.
- [ ] `system/` contents copied into `~/twoyi-rootfs/system/`.
- [ ] Custom twoyi `init` compiled (`gcc -static -o init twoyi_init.c`) and placed at rootfs root.
- [ ] `rom.ini` created.
- [ ] Empty writable dirs created (`data/local/tmp`, `dev/{input,socket,maps}`, `sdcard`, `vendor`).
- [ ] `rootfs.tar.gz` created.
- [ ] `file init` reports `ELF 64-bit LSB executable, x86-64`.
- [ ] `file system/bin/init` reports `ELF 64-bit LSB ... x86-64`.
- [ ] APK built with `./gradlew assembleRelease -Pabis=x86_64` (or rootfs pushed to device).
- [ ] Emulator booted, `adb logcat` shows `Connected to /dev/qemu_pipe`.
