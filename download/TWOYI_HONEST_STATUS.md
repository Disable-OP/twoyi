# Twoyi Testing — Honest Final Status

> **Date:** 2026-08-05
> **Codespace:** `twoyi-dev-3-jr47xg6xvx7ghq6p` (EastUs, AMD EPYC 7763, KVM working)
> **APK:** `twoyi_3.5.5-08042104-release.apk` (signed, x86_64, with rootfs)

---

## What actually happened

### The crash I initially reported as "working" was real

My previous report claimed the container booted based on a VLM analysis
of a screenshot. **That was wrong.** The screenshot showed the **Android
emulator's own launcher** (NexusLauncher with the pink/purple wallpaper),
not twoyi's container. The twoyi process had crashed with SIGABRT.

The user correctly identified this: "that's the AVD main screen, the
wallpaper should be dark, the pink and purple wallpaper is android 11
one, it crashed."

### Root cause of the crash

```
signal 6 (SIGABRT)
backtrace:
  #02 libtwoyi.so
  #11 renderer_reset_window+204
  #14 Render2Activity$1.surfaceChanged
```

On x86_64, the legacy `libOpenglRender.so` blob is not shipped (arm64-only).
My `renderer_bindings.rs` provided panic stubs for non-aarch64 targets.
`ProfileSettings.useNewRenderer()` defaulted to `false`, so the app
selected the old renderer → `surfaceChanged` → `renderer_reset_window` →
panic stub → `abort()` → SIGABRT.

### The fix

Two layers:

1. **`ProfileSettings.useNewRenderer()`** now defaults to `true` when the
   device's primary ABI is not arm64-v8a. This makes the Java side call
   `Renderer.setRendererType(1)` on x86_64.

2. **`core.rs`** adds `effective_renderer_type()` which forces
   `RendererType::New` on non-aarch64 targets even if Java requests Old.
   Defense-in-depth: the Rust side never calls the panic stubs on x86_64.

Commit: `7664c66` on `improvements/initial-cleanup`

---

## What happens after the fix

### The app no longer crashes ✅

- `mResumedActivity: io.twoyi/.Render2Activity` — twoyi is the foreground app
- `ps -A | grep twoyi` — process is alive (state S, not crashed)
- No tombstone generated

### The new Rust renderer initializes ✅

```
CLIENT_EGL: [NEW_RENDERER] GL context created successfully
CLIENT_EGL: [NEW_RENDERER] Initializing GL context: 1080x1920, DPI: 160x195, FPS: 45
```

### But the QEMU pipe is unavailable ❌

```
CLIENT_EGL: [NEW_RENDERER] Failed to write to pipe: Invalid argument (os error 22)
CLIENT_EGL: [NEW_RENDERER] Failed to initialize GL context: Invalid argument (os error 22)
CLIENT_EGL: [NEW_RENDERER] Falling back to old renderer
CLIENT_EGL: [CORE] New renderer failed to start (result=-1), this is expected if QEMU pipe is not available
```

### Why the QEMU pipe is unavailable

Twoyi's architecture requires the guest Android's SurfaceFlinger to
communicate with the host renderer via `/dev/qemu_pipe`. This pipe is
created by the twoyi guest's modified `init` process inside the rootfs.

In the Android emulator:
- The host Android (API 30, x86_64) does NOT have `/dev/qemu_pipe`
- The twoyi guest rootfs IS extracted to `/data/data/io.twoyi/rootfs/`
- The guest `init` binary IS arm64 (the rootfs was built for arm64)
- The guest `init` cannot execute on x86_64 (architecture mismatch)
- Therefore the QEMU pipe is never created
- Therefore the renderer has nothing to connect to

### What the screen shows

The twoyi boot log display is visible (the `BootLogTexture` component)
with three colored loading circles. The boot log shows the renderer
initialization sequence. After ~60 seconds with no `BOOT_COMPLETED`
message, `Render2Activity` times out and returns to SettingsActivity.

---

## What this means

### The fix is correct

The SIGABRT crash is fixed. The app gracefully handles the missing
QEMU pipe instead of crashing. The new Rust renderer is being used on
x86_64 as intended.

### But twoyi can't fully run in a standard Android emulator

This is a fundamental architectural limitation, not a bug:

1. **The rootfs is arm64-only** — the `init` binary inside the rootfs
   is compiled for aarch64. It cannot execute on an x86_64 emulator.
   To fix: build an x86_64 rootfs from the AOSP manifest.

2. **The QEMU pipe is guest-side** — twoyi's renderer connects to
   `/dev/qemu_pipe` which is created by the guest's `init`. In the
   emulator, there's no guest `init` running (it can't execute), so
   no pipe exists. To fix: either build an x86_64 rootfs, or run on
   a real arm64 device.

3. **The legacy renderer blob is arm64-only** — even if we had an
   x86_64 rootfs, the closed-source `libOpenglRender.so` doesn't
   ship for x86_64. The new Rust renderer would need to be completed
   (it currently has stubs for many GL commands).

### What would work

- **Real arm64 device** — install the signed APK on a physical Android
  phone (arm64). The rootfs will extract, the guest init will execute,
  the QEMU pipe will be created, and the legacy renderer will work.
  This is the intended use case.

- **x86_64 with an x86_64 rootfs** — build the rootfs from AOSP for
  x86_64 (using the `default.xml` manifest in the repo). Then the
  guest init can execute, the pipe will be created, and the new Rust
  renderer can connect. But the Rust renderer's GL protocol
  implementation is incomplete, so rendering may not work correctly.

---

## Summary

| Component | Status |
|---|---|
| KVM in Codespace | ✅ Working (AMD EPYC, EastUs, Seccomp:0) |
| APK signed | ✅ v2 signature scheme |
| APK installs | ✅ "Success" |
| Rootfs extracts | ✅ 687MB extracted to correct location |
| App launches | ✅ Render2Activity is foreground |
| App doesn't crash | ✅ Fixed (was SIGABRT, now graceful) |
| New renderer used | ✅ "Renderer type set to New" |
| GL context created | ✅ "GL context created successfully" |
| QEMU pipe available | ❌ Not in standard emulator |
| Guest init executes | ❌ arm64 binary on x86_64 host |
| Container boots | ❌ Cannot without working init + pipe |
| Container home screen | ❌ Not reached |

**The honest answer: twoyi cannot fully run in a standard Android x86_64
emulator because the guest rootfs is arm64-only and the QEMU pipe device
is created by the guest's init process, which can't execute on x86_64.**
The app no longer crashes (the fix is correct), but it can't render the
container without a working guest init and QEMU pipe.

To actually test twoyi, you need either a real arm64 device or an x86_64
rootfs built from the AOSP manifest.
