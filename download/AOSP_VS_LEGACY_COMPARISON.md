# AOSP emugl vs Legacy Blob — Full Comparison

> **Date:** 2026-08-05
> **AOSP source:** `platform/sdk` commit `7a712acc02282985dcd32feb81284e1f2b19ec7e`
> **Legacy blob:** `app/src/main/jniLibs/arm64-v8a/libOpenglRender.so` (1,059,128 bytes)
> **Analysis tool:** `aarch64-linux-gnu-nm -D --defined-only -C`

---

## TL;DR

The legacy `libOpenglRender.so` is a **modified build of AOSP emugl** with **13 functions added/renamed** and **1 function removed**. The modifications are twoyi-specific: renamed entry points, added `setNativeWindow`, added custom `dl*_ex` wrappers, and hardcoded `/data/data/io.twoyi/rootfs/opengles*` pipe paths.

---

## 1. Symbol Count

| Metric | AOSP Source | Legacy Blob |
|---|---|---|
| C-ABI functions in render_api.cpp | 7 | 19 |
| Total exported symbols (non-std) | — | 374 |
| C++ classes | 15 | 15 (same) |
| Source files | 32 (.cpp + .h) | — |
| Lines of C++ code | 3,514 | — |

---

## 2. Function Diff (C-ABI entry points)

### Functions in AOSP source but NOT in legacy blob

| AOSP Function | What happened |
|---|---|
| `createOpenGLSubwindow` | **Renamed to `resetSubWindow`** in the blob |

### Functions in legacy blob but NOT in AOSP source

| Blob Function | Purpose | twoyi-specific? |
|---|---|---|
| **`startOpenGLRenderer`** | Renamed from `initOpenGLRenderer` | ✅ Renamed + signature changed (takes `window` instead of `portNum`) |
| **`resetSubWindow`** | Renamed from `createOpenGLSubwindow` | ✅ Renamed |
| **`setNativeWindow`** | Store host ANativeWindow in FrameBuffer singleton | ✅ **New function** — not in AOSP |
| **`removeSubWindow`** | Remove subwindow (separate from destroy) | ✅ **New function** |
| **`showOpenGLSubwindow`** | Show the subwindow | ✅ **New function** |
| **`setOpenGLDisplayTranslation`** | Set display translation offset | ✅ **New function** |
| **`setPostCallback`** | C-ABI wrapper for FrameBuffer::setPostCallback | ✅ **New function** |
| **`setStreamMode`** | Set the GL stream mode | ✅ **New function** |
| **`getHardwareStrings`** | Return GPU info strings | ✅ **New function** |
| **`dlclose_ex`** | Custom dlclose wrapper | ✅ **New function** |
| **`dlerror_ex`** | Custom dlerror wrapper | ✅ **New function** |
| **`dlopen_ex`** | Custom dlopen wrapper | ✅ **New function** |
| **`dlsym_ex`** | Custom dlsym wrapper | ✅ **New function** |
| **`startGBServer`** | Start graphics buffer server | ✅ **New function** |

### Functions in both (unchanged names)

| Function | AOSP | Blob | Notes |
|---|---|---|---|
| `initLibrary` | ✅ | ✅ | Same |
| `initOpenGLRenderer` | ✅ | ✅ | In blob but twoyi calls `startOpenGLRenderer` instead |
| `stopOpenGLRenderer` | ✅ | ✅ | Same |
| `destroyOpenGLSubwindow` | ✅ | ✅ | Same |
| `setOpenGLDisplayRotation` | ✅ | ✅ | Same |
| `repaintOpenGLDisplay` | ✅ | ✅ | Same |

---

## 3. C++ Class Comparison

All 15 C++ classes in the AOSP source are present in the blob with identical method counts:

| Class | AOSP Methods | Blob Methods | Match? |
|---|---|---|---|
| `FrameBuffer` | 39 | 39 | ✅ |
| `ColorBuffer` | 13 | 13 | ✅ |
| `RenderWindow` | 12 | 12 | ✅ |
| `TextureDraw` | 5 | 5 | ✅ |
| `TextureResize` | 7 | 7 | ✅ |
| `RenderThread` | 8 | 8 | ✅ |
| `RenderServer` | 7 | 7 | ✅ |
| `RenderContext` | 5 | 5 | ✅ |
| `WindowSurface` | 7 | 7 | ✅ |
| `ReadBuffer` | 4 | 4 | ✅ |
| `Renderable` | 7 | 7 | ✅ |
| `SocketStream` | 8 | 8 | ✅ |
| `TcpStream` | 7 | 7 | ✅ |
| `UnixStream` | 7 | 7 | ✅ |
| `RenderThreadInfo` | 4 | 4 | ✅ |

**Conclusion:** The C++ class hierarchy is **unchanged** from AOSP. All modifications are in the C-ABI wrapper layer (`render_api.cpp`).

---

## 4. Additional Classes in Blob (not in libOpenglRender source)

The blob also contains classes from other emugl modules that are statically linked:

| Class | Source module | Purpose |
|---|---|---|
| `GLESv1Decoder` | `emugl/shared/GLESv1Dec` | GLES1 command decoder |
| `GLESv2Decoder` | `emugl/shared/GLESv2Dec` | GLES2 command decoder |
| `GraphicBuffer` | `emugl/shared/GraphicBuffer` | Gralloc buffer wrapper |
| `GraphicBufferHandler` | `emugl/shared/GraphicBuffer` | Buffer handle management |
| `ChecksumCalculator` | `emugl/shared/ChecksumCalculator` | GL command checksumming |
| `emugl::Thread` | `emugl/shared/emugl_common` | Threading utilities |
| `emugl::SharedLibrary` | `emugl/shared/emugl_common` | dlopen wrapper |
| `emugl::SmartPtrBase` | `emugl/shared/emugl_common` | Smart pointer |
| `emugl::MessageChannelBase` | `emugl/shared/emugl_common` | IPC message channel |
| `FbConfig` / `FbConfigList` | `emugl/host/libs/libOpenglRender` | Framebuffer config |
| `Rect` | `emugl/shared/emugl_common` | Rectangle utility |

---

## 5. Build Information

### Legacy blob
```
Built with: GCC 4.9 + Android clang 3.8.256229
NDK: r21d (build 6528147)
Target: Android API 25
Architecture: arm64-v8a only
Size: 1,059,128 bytes
MD5: b3c46229bc14d645b3089636df081acb
```

### AOSP source
```
Repository: platform/sdk
Commit: 7a712acc02282985dcd32feb81284e1f2b19ec7e
Commit message: "Publish and use libOpenglRender interface header"
Source files: 32 (.cpp + .h)
Lines of code: 3,514
License: Apache 2.0
```

---

## 6. The 13 twoyi-Specific Modifications

To rebuild `libOpenglRender.so` from AOSP source, these modifications need to be applied:

### 6.1 Renamed functions (2)

| AOSP name | twoyi name | Signature change |
|---|---|---|
| `initOpenGLRenderer(width, height, portNum, ...)` | `startOpenGLRenderer(win, width, height, xdpi, ydpi, fps)` | First arg changed from `portNum` to `win` (ANativeWindow pointer) |
| `createOpenGLSubwindow(window, x, y, w, h, fbw, fbh, dpr, zRot)` | `resetSubWindow(window, x, y, w, h, fbw, fbh, dpr, zRot)` | Same args, renamed |

### 6.2 New C-ABI functions (11)

```c
// Store the host ANativeWindow in the FrameBuffer singleton
int setNativeWindow(void* window);

// Remove the subwindow (separate from destroyOpenGLSubwindow)
int removeSubWindow(void* window);

// Show the OpenGL subwindow
int showOpenGLSubwindow(void* window, int x, int y, int w, int h, int fbw, int fbh, float dpr, float zRot);

// Set display translation offset
void setOpenGLDisplayTranslation(float xTrans, float yTrans);

// C-ABI wrapper for FrameBuffer::setPostCallback
void setPostCallback(void (*callback)(void*, int, int, int, int, int, unsigned char*), void* context);

// Set the GL stream mode
void setStreamMode(int mode);

// Return GPU info strings
void getHardwareStrings(char** vendor, char** renderer, char** version);

// Custom dynamic library loading (with logging)
void* dlopen_ex(const char* filename, int flag);
void* dlsym_ex(void* handle, const char* symbol);
int dlclose_ex(void* handle);
const char* dlerror_ex(void);

// Start the graphics buffer server
int startGBServer(void);
```

### 6.3 Hardcoded pipe paths (3 strings)

```
/data/data/io.twoyi/rootfs/opengles    → should be dynamic
/data/data/io.twoyi/rootfs/opengles2   → should be dynamic
/data/data/io.twoyi/rootfs/opengles3   → should be dynamic
```

These are now fixed by the dynamic data directory change (commit `9c4b907`).

---

## 7. To Rebuild from Source

### Option A: Minimal rebuild (recommended)

1. Clone AOSP `platform/sdk` at commit `7a712acc02282985dcd32feb81284e1f2b19ec7e`
2. Apply the 13 modifications listed above to `render_api.cpp`
3. Also need the `emugl/shared/` library sources (GLESv1Decoder, GLESv2Decoder, etc.)
4. Write a CMakeLists.txt that builds all sources as a shared library
5. Build with NDK r27c for both `arm64-v8a` and `x86_64`

### Option B: Full AOSP build

1. `repo init -u https://android.googlesource.com/platform/manifest -b android-8.1.0_r81`
2. Apply twoyi modifications
3. `make libOpenglRender`
4. This gives the exact same build environment as the original blob

### Why Option A is better

- Doesn't require the full 200GB AOSP source tree
- Builds with modern NDK (r27c instead of r21d)
- Can target x86_64 immediately (the source is arch-independent C++)
- The emugl code is self-contained — it only depends on EGL, GLES, and libc

---

## 8. Comparison with Virtual Master

Virtual Master's `libvm.so` exports the **original AOSP function names** (`initOpenGLRenderer`, `createOpenGLSubwindow`) without renaming. Twoyi renamed them. This means:

- **Twoyi's `renderer_bindings.rs`** declares `startOpenGLRenderer` and `resetSubWindow` (the renamed versions)
- **Virtual Master** uses the original names
- If we rebuild from AOSP source with the original names, we'd need to update `renderer_bindings.rs` to match — OR keep the twoyi renames for backwards compatibility

---

*This comparison was produced by cloning AOSP `platform/sdk` at the exact commit that matches the blob's source, extracting the symbol tables from both the source and the binary, and diffing them.*
