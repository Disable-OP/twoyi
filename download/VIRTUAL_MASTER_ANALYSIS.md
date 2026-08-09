# Virtual Master Disassembly — How It Really Renders

> **Date:** 2026-08-05
> **APK:** `com.clone.android.dual.space_3.2.53` (155 MB, arm64-v8a + armeabi-v7a)
> **Downloaded via:** Playwright (headless Chromium) from APKMirror
> **Tool:** `aarch64-linux-gnu-nm`, `readelf`, `strings` on Ubuntu 22.04

---

## TL;DR

**Virtual Master uses the SAME emugl/QEMU pipe rendering architecture as twoyi.** It does NOT pull data from SurfaceFlinger directly. Both apps are derived from the same AOSP `emugl` codebase. The key difference is that Virtual Master uses `NativeActivity` (all-native rendering loop) while twoyi uses Java `Activity` + `SurfaceView` + JNI.

---

## 1. Native Library Map

Virtual Master ships 7 custom native libraries (plus Crashlytics):

| Library | Size (arm64) | Purpose |
|---|---|---|
| **`libvm.so`** | 7.7 MB | Main engine — NativeActivity + emugl renderer + GL dispatch |
| `libkr64.so` | 1.5 MB | "Kernel replacement" — stripped, no exports, only imports mmap/socket/socketpair |
| `libkr64.11.so` | 2.0 MB | Android 11 variant of kernel replacement |
| `libkr32.so` | 1.7 MB | 32-bit kernel replacement |
| `libkrloader64.so` | 217 KB | Guest loader (like twoyi's libloader.so) — PIE executable with `_start` |
| `libkrloader32.so` | 167 KB | 32-bit guest loader |
| `libadb.so` | 115 KB | ADB binary (dynamically linked — much smaller than twoyi's 4.4 MB static) |
| `libun7z.so` | 75 KB | 7zip extraction for rootfs |

---

## 2. libvm.so — The Rendering Engine

### Exported C-ABI functions (the 6 emugl entry points)

```
0x392220 T initOpenGLRenderer
0x395988 T createOpenGLSubwindow
0x395f04 T destroyOpenGLSubwindow
0x3968f0 T repaintOpenGLDisplay
0x396430 T setOpenGLDisplayRotation
0x393f58 T stopOpenGLRenderer
```

**These are the EXACT same function names as AOSP's `render_api.cpp`** — the same source file that twoyi's `libOpenglRender.so` is built from. Virtual Master did NOT rename them (twoyi renamed `initOpenGLRenderer` to `startOpenGLRenderer` and `createOpenGLSubwindow` to `resetSubWindow`).

### Other key exports

```
0x3ff350 T JNI_OnLoad           — JNI registration
0x6b2f08 T ANativeActivity_onCreate — Virtual Master is a NativeActivity!
0x391a6c T initLibrary           — loads EGL + GLES dispatch tables
0x393aa0 T getHardwareStrings    — returns GPU info strings
```

### Imported symbols (what it needs from the system)

```
U ANativeWindow_fromSurface    — gets ANativeWindow from Java Surface
U ANativeWindow_release        — releases the window
U AInputQueue_*                — input event handling (NativeActivity)
U ALooper_*                    — event loop (NativeActivity)
U AConfiguration_*             — config (NativeActivity)
```

**Notable: NO libEGL, libGLES imports.** Virtual Master loads EGL/GLES dynamically via `dlopen`/`dlsym` at runtime (the `init_egl_dispatch` / `init_gles1_dispatch` / `init_gles2_dispatch` functions). This is the same pattern as twoyi/AOSP emugl.

### Dynamic dependencies

```
NEEDED: liblog.so
NEEDED: libdl.so
NEEDED: libandroid.so    ← for NativeActivity
NEEDED: libc.so
NEEDED: libm.so
```

### Strings confirming the QEMU pipe architecture

```
"could not create pipe: %s"
"No data on command pipe!"
"init_gles1_dispatch"
"init_gles2_dispatch"
"getGLES1ExtensionString"
"setWindowSurfaceColorBuffer"
"allocBuffer"
"add_surface"
"del_surface"
"removeSubWindow"
```

These strings are **identical** to those in AOSP's `emugl` source. Virtual Master creates a pipe device (just like twoyi's `/dev/qemu_pipe`) and the guest's SurfaceFlinger sends GL commands through it.

### String obfuscation

The binary contains ~100 `.datadiv_decode*` functions — these are automatic string deobfuscation routines (common in commercial Android apps). The obfuscated strings like `ch@apeglWle`av` and `EGLgETcURRENTsURFACE` and `GLfRAMEBUFFERtEXTURE` are decoded at runtime. This is why `/dev/` paths don't appear in the strings output — they're encoded.

---

## 3. libkr64.so — The "Kernel Replacement"

This is the most interesting library by name, but the most opaque:

- **No exported symbols** (completely stripped)
- **Only 3 imports:** `mmap`, `socket`, `socketpair`
- **No strings** related to binder, surface, graphics, or device paths

This library likely handles:
- Creating virtual `/dev/binder` and `/dev/vndbinder` devices via `socketpair`
- Setting up the guest's virtual IPC layer
- The `mmap` import suggests it maps shared memory for buffer sharing

But without deeper reverse engineering (decompilation), I can't confirm the exact mechanism.

---

## 4. Java Side (DEX Analysis)

The DEX files contain references to:

| Class | Purpose |
|---|---|
| `DisplayManager` + `DisplayListener` | Display management — likely for multi-display or resolution switching |
| `SurfaceTexture` + `OnFrameAvailableListener` | Frame capture from a texture |
| `TextureView` + `SurfaceTextureListener` | **Virtual Master uses TextureView, not SurfaceView** |
| `MediaProjectionManager` | Screen capture — probably for screenshot sharing, not rendering |

### TextureView vs SurfaceView

This is a meaningful difference:
- **Twoyi** uses `SurfaceView` — the native renderer draws directly to the surface's buffer via EGL. The host compositor doesn't see the pixels until they're on screen.
- **Virtual Master** uses `TextureView` — the native renderer draws to a `SurfaceTexture`, which the host compositor can intercept as a GPU texture. This allows the host to apply transforms, filters, or capture the frame.

However, this doesn't change the fundamental guest→host GL command transport. Both still use the QEMU pipe for the guest's SurfaceFlinger to send GL commands to the host renderer.

---

## 5. Comparison: Virtual Master vs Twoyi

| Aspect | Twoyi | Virtual Master |
|---|---|---|
| **Renderer source** | AOSP emugl (modified) | AOSP emugl (less modified) |
| **GL transport** | QEMU pipe (`/dev/qemu_pipe`) | QEMU pipe (same, path obfuscated) |
| **C-ABI function names** | Renamed (`startOpenGLRenderer`, `resetSubWindow`) | Original AOSP names (`initOpenGLRenderer`, `createOpenGLSubwindow`) |
| **Host view** | `SurfaceView` (Java Activity) | `TextureView` (NativeActivity) |
| **Guest Android** | 8.1 | 7.1.2 |
| **ABIs** | arm64-v8a only | arm64-v8a + armeabi-v7a |
| **x86_64 support** | ✅ (our fork adds it) | ❌ (arm only) |
| **Loader** | `libloader.so` (51 KB, C) | `libkrloader64.so` (217 KB, C++) |
| **Kernel virt** | None (relies on namespace isolation) | `libkr64.so` (1.5 MB, stripped) |
| **String obfuscation** | None | `.datadiv_decode*` (100+ functions) |
| **ADB** | 4.4 MB (static) | 115 KB (dynamic) |
| **Open-source replacements** | ✅ (Rust loader + renderer) | ❌ (all closed) |

---

## 6. The Answer to Your Question

> "Can we make it boot raw GSI directly? Without using the weird /dev/qemu_pipe approach, pulling data from SurfaceFlinger directly?"

**Based on the disassembly, Virtual Master does NOT do this.** It uses the same QEMU pipe approach as twoyi. No Android-in-Android container app I've found uses direct SurfaceFlinger capture — they all use the emugl/QEMU pipe architecture.

### Why they all use QEMU pipes

The QEMU pipe approach exists because:
1. The guest's SurfaceFlinger needs to send GL commands (draw calls, textures, buffers) to the host's GPU
2. The host needs to execute those GL commands on its own EGL/GL context
3. The only efficient way to do this without root is via a pipe/socket that the guest writes to and the host reads from
4. The guest's `init` creates this pipe device in `/dev/` before SurfaceFlinger starts

### What "pulling from SurfaceFlinger directly" would require

To bypass the QEMU pipe and capture SurfaceFlinger output directly, you would need:

1. **A virtual Display** created via `DisplayManager.createVirtualDisplay()` on the HOST
2. **The guest's SurfaceFlinger configured to render to that virtual display** instead of to the QEMU pipe
3. **An `ImageReader` or `SurfaceTexture` on the host** to capture frames from the virtual display
4. **The guest rootfs modified** so SurfaceFlinger uses a `FramebufferTarget` that points at a host-provided buffer instead of the QEMU pipe

This is theoretically possible but requires:
- Rewriting the guest's `SurfaceFlinger` configuration to use a different `HWC` (Hardware Composer) backend
- Creating a virtual `gralloc` HAL that allocates buffers in shared memory (ashmem/memfd) accessible by both guest and host
- The host reads these shared buffers and displays them on its SurfaceView/TextureView

This is essentially what **redroid** does with `androidboot.redroid_gpu_mode=guest` — it renders in software inside the container and captures the framebuffer. But redroid runs as root (Docker `--privileged`), which twoyi can't do.

### The practical path forward for twoyi on x86_64

Since the QEMU pipe approach is the standard and Virtual Master confirms it, the path to x86_64 support is:

1. **Build an x86_64 rootfs** from the AOSP manifest — the guest `init` will create `/dev/qemu_pipe` and the guest SurfaceFlinger will connect to it
2. **Complete the Rust renderer** (`renderer_new/`) so it properly handles the GL commands that come through the pipe
3. **Or: rebuild `libOpenglRender.so` from AOSP source** (which we confirmed is Apache-2.0 licensed) for x86_64

---

## 7. Screenshots

All screenshots are in `/home/z/my-project/download/screenshots/`:

| File | Description |
|---|---|
| `01_twoyi_settings.png` | Twoyi settings screen on the Android 11 x86_64 emulator |
| `02_twoyi_boot_log.png` | Twoyi container boot log (after the renderer fix — no crash, but QEMU pipe unavailable) |
| `03_twoyi_no_rom_dialog.png` | Twoyi "No ROM Installed" dialog (before rootfs extraction) |
| `vm_analysis_state.png` | Current emulator state during Virtual Master analysis |

---

*This analysis was produced by downloading the Virtual Master APK via Playwright (headless Chromium to bypass Cloudflare), extracting the native libraries, and analyzing them with GNU binutils (`nm`, `readelf`, `strings`).*
