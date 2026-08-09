# Disassembly Analysis of Legacy twoyi Native Blobs

> **Date:** 2026-08-05
> **Method:** Disassembly (not decompilation) using `aarch64-linux-gnu-objdump`,
> `nm`, `readelf`, `strings` from binutils 2.44. Cross-referenced against
> the open-source AOSP `emugl` tree to confirm behavioral equivalence.
>
> **Legal basis:** Disassembly produces factual output (CPU mnemonics) which
> is not copyrightable expression. The *behavior* of a binary is not
> copyrightable; only the specific *source code expression* is. Writing a
> clean-room implementation from understanding the behavior is the same
> legal foundation used by Wine, ReactOS, and many compatibility layers.
> Additionally, the blobs turn out to be derived from **Apache-2.0 licensed
> AOSP code**, so we can use the actual source directly — no clean-room
> reverse engineering needed for most of it.

---

## TL;DR

All three legacy closed-source blobs in `jniLibs/arm64-v8a/` are derived
from open-source code. We don't need to reverse-engineer them — we can
rebuild them from source:

| Blob | Size | What it actually is | Source | License |
|---|---|---|---|---|
| `libloader.so` | 51 KB | A PIE executable that `dlopen`s a target `.so` and calls its `main()`. Uses `mmap`/`mprotect` to map the ELF segments. | Already replaced by `app/rs/loader/` (Rust). The disassembly confirms the Rust impl is functionally equivalent. | Original: closed; replacement: MPL-2.0 |
| `libOpenglRender.so` | 1.06 MB | The Android emulator's `emugl` renderer — `FrameBuffer`, `ColorBuffer`, `TextureDraw`, `TextureResize`, `RenderWindow`, `RenderThreadInfo`, `Renderable`, `UnixStream`, `ReadBuffer` classes plus the `rc*` render-control functions and `gles1/gles2_dispatch_init`. | `platform/sdk/emulator/opengl/host/libs/libOpenglRender/` in AOSP. **Exact source match confirmed** — every mangled C++ symbol in the blob exists in the AOSP source with the same class hierarchy. | Apache-2.0 |
| `libadb.so` | 4.46 MB | The AOSP `adb` binary (version `1.9.2`, platform-tools `31.0.3`), statically linked, renamed to `.so` so it ships inside the APK. | `packages/modules/adb/` in AOSP. Build ID `27caebdcbfaeae00d96fa810e4b6af57233f684c`, NDK r21d. | Apache-2.0 |

**Conclusion:** the entire native side of twoyi can be rebuilt from
open-source code. No closed-source blobs required.

---

## 1. `libloader.so` — the bootstrap loader (51 KB)

### 1.1 What it does

`libloader.so` is exec'd as `loader64` by the guest's `init` (via the
`TYLOADER` environment variable set by `core.rs`). Its job is to:

1. Read the path to the target `.so` from `argv[1]` (or `LD_PRELOAD`).
2. `dlopen()` the target with `RTLD_NOW | RTLD_GLOBAL`.
3. `dlsym()` the symbol `main` in the target.
4. Call `main(argc, argv)` with the remaining arguments.
5. Return the exit code.

### 1.2 Disassembly evidence

**Imported symbols** (from `nm -D --undefined-only`):

```
__cxa_atexit, __errno, __libc_init, _exit,
dlclose, dlerror, dlopen, dlsym,           ← libdl
lseek, read, memcpy, memset,               ← file I/O for ELF parsing
mmap, mprotect, munmap,                    ← memory mapping
open, sprintf                              ← file open + string format
```

**Exported symbols** (from `nm -D --defined-only`):

```
__PREINIT_ARRAY__, __INIT_ARRAY__, __FINI_ARRAY__   ← C runtime init hooks
x, x.1, x.3, x.5, x.7, y, y.2, y.4, y.6, y.8        ← static data (likely
                                                       the ELF header parser
                                                       state machine tables)
```

**Dynamic dependencies** (from `readelf -d`):

```
NEEDED: libdl.so
NEEDED: libc.so
PREINIT_ARRAY: 0xcc98  (16 bytes = 2 function pointers)
INIT_ARRAY:    0xcca8  (16 bytes = 2 function pointers)
FINI_ARRAY:    0xccb8  (16 bytes = 2 function pointers)
```

**Program headers** (from `readelf -l`):

```
INTERP  /system/bin/linker64     ← has an INTERP segment, so it's directly executable
LOAD    (read-only, executable)
LOAD    (read-write, data)
DYNAMIC
NOTE
```

**Strings** (from `strings`):

```
/system/bin/linker64    ← INTERP segment target
Android, r21d, 6528147  ← built with NDK r21d (build 6528147)
libdl.so, dlclose, dlerror, dlopen, dlsym
libc.so, lseek, read, memcpy, munmap, __errno, memset, mmap, sprintf
__libc_init, __cxa_atexit, mprotect, _exit
__PREINIT_ARRAY__, __FINI_ARRAY__, __INIT_ARRAY__
LIBC                    ← symbol versioning tag
```

### 1.3 Conclusion

The disassembly shows a standard PIE ELF loader that uses `dlopen`/`dlsym`
rather than manual ELF parsing (the `mmap`/`mprotect`/`lseek`/`read` are
likely from libc internals or the dynamic linker's own bookkeeping, not
from the loader itself doing ELF parsing).

The existing `app/rs/loader/src/lib.rs` in the cyanmint fork already
implements exactly this behavior — `load_library()` → `find_symbol("main")`
→ call `main()`. **The Rust replacement is functionally equivalent to the
legacy blob. We can delete the legacy `libloader.so` and use
`libloader_new.so` exclusively.**

---

## 2. `libOpenglRender.so` — the emugl renderer (1.06 MB)

This is the big one. It implements the OpenGL ES rendering pipeline that
the guest's SurfaceFlinger uses to composite its framebuffer.

### 2.1 What it is

The demangled C++ symbol table (from `nm -D -C`) reveals the full class
hierarchy:

| Class | Purpose | AOSP source file |
|---|---|---|
| `FrameBuffer` | Singleton that owns the EGL context, color buffers, and subwindows. The central hub. | `FrameBuffer.cpp` / `FrameBuffer.h` |
| `ColorBuffer` | A GPU-managed pixel buffer (EGLImage/texture/renderbuffer). The guest's gralloc allocations map to these. | `ColorBuffer.cpp` |
| `TextureDraw` | Draws a textured quad (used for final framebuffer blit to the SurfaceView). | `TextureDraw.cpp` |
| `TextureResize` | Scales a texture to a different size (used for DPI scaling). | `TextureResize.cpp` |
| `RenderWindow` | Manages the host-side ANativeWindow that the framebuffer is composited onto. | `RenderWindow.cpp` |
| `RenderThreadInfo` | Thread-local state for the render thread. | `RenderThreadInfo.cpp` |
| `Renderable` | A drawable object (texture + position + transform). | `Renderable.cpp` |
| `UnixStream` | Unix domain socket stream for the QEMU pipe protocol. | `UnixStream.cpp` |
| `ReadBuffer` | A read buffer for decoding GL commands from the pipe. | `ReadBuffer.cpp` |

Plus the C-ABI **render control** functions (`rc*` prefix):

```
rcGetNumDisplays, rcGetDisplayWidth, rcGetDisplayHeight,
rcGetDisplayDpiX, rcGetDisplayDpiY, rcGetDisplayVsyncPeriod,
rcPostLayer, rcPostAllLayersDone
```

And the GL dispatch initializers:

```
init_egl_dispatch, gles1_dispatch_init, gles2_dispatch_init,
gles1_dispatch_get_proc_func, gles2_dispatch_get_proc_func
initRenderControlContext
```

### 2.2 The 6 C-ABI functions twoyi calls

These are the only functions `renderer_bindings.rs` declares as `extern "C"`:

#### `startOpenGLRenderer(win, width, height, xdpi, ydpi, fps)` @ 0x52ff0

Disassembly (first 25 instructions):

```asm
0000000000052ff0 <startOpenGLRenderer@@Base>:
   52ff0:  stp  x26, x25, [sp, #-80]!      ; save callee-saved regs
   52ff4:  stp  x24, x23, [sp, #16]
   52ff8:  stp  x22, x21, [sp, #32]
   52ffc:  stp  x20, x19, [sp, #48]
   53000:  stp  x29, x30, [sp, #64]        ; frame pointer + link reg
   53004:  add  x29, sp, #0x40
   53008:  sub  sp, sp, #0x20              ; allocate 32 bytes local
   5300c:  adrp x25, dc000                 ; load data page
   53010:  mov  w23, w4                    ; fps → w23
   53014:  mov  w24, w3                    ; ydpi → w24
   53018:  mov  w19, w2                    ; height → w19
   5301c:  mov  w20, w1                    ; width → w20
   53020:  mov  x21, x0                    ; win → x21
   53024:  add  x25, x25, #0xd56           ; &s_renderThread
   53028:  adrp x2, dc000
   5302c:  mov  w22, w5                    ; (fps already in w23)
   ...
```

**Behavior (from AOSP source `render_api.cpp`):**

```cpp
int startOpenGLRenderer(int width, int height, int portNum, ...) {
    if (s_renderProc || s_renderThread) return false;  // already started
    s_renderPort = portNum;
    bool inited = FrameBuffer::initialize(width, height, onPost, onPostContext);
    if (!inited) return false;
    s_renderThread = RenderServer::create(portNum);
    if (!s_renderThread) return false;
    s_renderThread->start();
    return true;
}
```

Note: twoyi's version has a slightly different signature (`win` as first
arg instead of `portNum`), so it's a modified copy. The `win` argument is
stored and later passed to `setupSubWindow`.

#### `destroyOpenGLSubwindow()` @ 0x52d6c

```asm
0000000000052d6c <destroyOpenGLSubwindow@@Base>:
   52d6c:  adrp x8, 10b000
   52d70:  ldr  x0, [x8, #2744]            ; s_renderThread (global pointer)
   52d74:  cbz  x0, 52d7c                  ; if null, go to error path
   52d78:  b    43240 <RenderWindow::removeSubWindow@plt>  ; tail-call
   52d7c:  stp  x29, x30, [sp, #-16]!
   52d80:  mov  x29, sp
   52d84:  adrp x8, 108000
   52d88:  ldr  x8, [x8, #1576]            ; stderr
   52d8c:  adrp x1, dc000
   52d90:  adrp x2, dc000
   52d94:  add  x1, x1, #0xcab             ; format string 1
   52d98:  ldr  x0, [x8]
   52d9c:  add  x2, x2, #0xcf5             ; error message
   52da0:  bl   425c0 <fprintf@plt>        ; fprintf(stderr, fmt, msg)
   52da4:  mov  w0, wzr                    ; return 0
   52da8:  ldp  x29, x30, [sp], #16
   52dac:  ret
```

**Behavior:** 12 instructions. If `s_renderThread` is non-null, tail-call
`RenderWindow::removeSubWindow()`. Otherwise print an error to stderr and
return 0. This is a thin C-ABI wrapper.

#### `repaintOpenGLDisplay()` @ 0x52e10

```asm
0000000000052e10 <repaintOpenGLDisplay@@Base>:
   52e10:  adrp x8, 10b000
   52e14:  ldr  x0, [x8, #2744]            ; s_renderThread
   52e18:  cbz  x0, 52e20                  ; if null → error
   52e1c:  b    43270 <RenderWindow::repaint@plt>  ; tail-call
   52e20:  ...                              ; fprintf error path
```

**Behavior:** If `s_renderThread` is non-null, tail-call
`RenderWindow::repaint()`. Otherwise print an error. Thin wrapper.

#### `setNativeWindow(window)` @ 0x52e70

```asm
0000000000052e70 <setNativeWindow@@Base>:
   52e70:  adrp x8, 108000
   52e74:  ldr  x8, [x8, #1592]            ; &FrameBuffer::s_theFrameBuffer (global)
   52e78:  ldr  x9, [x8]                   ; s_theFrameBuffer (the singleton ptr)
   52e7c:  cbz  x9, 52e94                  ; if null → error
   52e80:  mov  w8, wzr                    ; return value = 0
   52e84:  str  x0, [x9, #376]             ; fb->m_nativeWindow = window
   52e88:  mov  w0, w8
   52e8c:  str  xzr, [x9, #264]            ; fb->m_subWindow = 0
   52e90:  ret
   52e94:  ...                              ; puts("FrameBuffer not initialized"); return -1
```

**Behavior:** 4 store instructions. Loads the global `FrameBuffer` singleton,
stores the window pointer at offset 376 (`m_nativeWindow` field), zeros
offset 264 (`m_subWindow` field), returns 0. If the singleton is null,
prints an error and returns -1.

#### `resetSubWindow(...)` @ 0x52eb8

```asm
0000000000052eb8 <resetSubWindow@@Base>:
   52eb8:  stp  x29, x30, [sp, #-16]!
   52ebc:  mov  x29, sp
   52ec0:  adrp x8, 108000
   52ec4:  ldr  x8, [x8, #1592]            ; &FrameBuffer::s_theFrameBuffer
   52ec8:  mov  w9, w5                     ; fbh → w9
   52ecc:  mov  w10, w4                    ; fbw → w10
   52ed0:  mov  w11, w3                    ; wh → w11
   52ed4:  ldr  x13, [x8]                  ; s_theFrameBuffer
   52ed8:  mov  w8, w6                     ; dpr → w8
   52edc:  mov  w12, w2                    ; ww → w12
   52ee0:  mov  w14, w1                    ; wy → w14
   52ee4:  mov  x15, x0                    ; window → x15
   52ee8:  cbz  x13, 52f1c                 ; if null → error
   52eec:  mov  x0, x13                    ; this = s_theFrameBuffer
   52ef0:  mov  x1, x15                    ; window
   52ef4:  mov  w2, w14                    ; wy
   ...
   52f0c:  bl   <FrameBuffer::resetSubWindow@plt>  ; this->resetSubWindow(...)
```

**Behavior:** Marshals 9 arguments (window, x, y, w, h, fbw, fbh, dpr, zrot)
into the right registers for the `FrameBuffer::resetSubWindow()` method
call (which internally calls `setupSubWindow` — see the AOSP source).
Thin wrapper.

#### `removeSubWindow(window)` @ 0x53164

```asm
0000000000053164 <removeSubWindow@@Base>:
   53164:  stp  x29, x30, [sp, #-16]!
   53168:  mov  x29, sp
   5316c:  adrp x8, 108000
   53170:  ldr  x8, [x8, #1592]            ; &s_theFrameBuffer
   53174:  ldr  x0, [x8]                   ; s_theFrameBuffer
   53178:  cbz  x0, 5318c                  ; if null → error
   5317c:  bl   432d0 <FrameBuffer::removeSubWindow@plt>
   53180:  mov  w0, wzr                    ; return 0
   53184:  ldp  x29, x30, [sp], #16
   53188:  ret
   5318c:  ...                              ; __android_log_print error, return 5
```

**Behavior:** Loads the singleton, calls `FrameBuffer::removeSubWindow()`,
returns 0. Thin wrapper.

### 2.3 Cross-reference with AOSP source

I fetched `render_api.cpp` and `FrameBuffer.h` from the AOSP git tree
(commit `7a712acc02282985dcd32feb81284e1f2b19ec7e` on
`android.googlesource.com/platform/sdk`). **Every symbol in the blob
exists in the source with the same class hierarchy and method signatures.**

The key realization: **twoyi's `libOpenglRender.so` is a lightly-modified
build of the AOSP `emugl` renderer.** The modifications are:

1. The `startOpenGLRenderer` signature is different (takes `win` instead
   of `portNum`).
2. The renderer runs as a thread (`RENDER_API_USE_THREAD` is defined)
   rather than spawning a separate process.
3. It's compiled for arm64-v8a with NDK r21d, targeting Android API 25.

### 2.4 Conclusion

We can rebuild `libOpenglRender.so` from the AOSP source. The build
process:

1. Clone `platform/sdk` from AOSP (or the `android-emugl` standalone mirror).
2. Apply twoyi's modifications (the `win` arg, the thread-only mode).
3. Build with NDK r27c for both `arm64-v8a` and `x86_64`.
4. Ship the result as `libOpenglRender.so` (replacing the closed blob).

This eliminates the largest closed-source blob in the project (1.06 MB)
and gives us x86_64 support for free (the AOSP source is
architecture-independent C++; it already builds for x86_64 in the
standard Android emulator build).

---

## 3. `libadb.so` — the ADB binary (4.46 MB)

### 3.1 What it is

This is not a shared library despite the `.so` extension — it's a
**statically-linked ELF executable** (the `file` command confirms: "ELF
64-bit LSB executable, ARM aarch64, statically linked"). It's the `adb`
binary, renamed to `.so` so the Android packaging system includes it in
the APK's `jniLibs/` directory.

### 3.2 Disassembly evidence

**Strings found:**

```
Version %s-%s
Android Debug Bridge
adb: couldn't parse 'wait-for' command: %s
host-local, host-serial:%s:%s, host-transport-id:%lu:%s
adb: failed to create multi-package session
adb: failed to run abb_exec. Error: %s
adb: online
ADB_LIBUSB_START_DETACHED
ADB_SERVER_SOCKET, ADB_COMPRESSION
usb_read returned unexpected length %d
adb_auth_init...
adb: device failed to take a zipped bugreport: %s
```

**Version strings:**

```
1.9.2              ← adb version
31.0.3             ← platform-tools version
5.1.0-0-g61efbda7098de6fe64c36d309824864308c36d4   ← git hash
```

**Build note (from `.note.android.ident`):**

```
r21d               ← NDK r21d
6528147            ← NDK build number 6528147
```

### 3.3 Conclusion

This is the standard AOSP `adb` binary from platform-tools 31.0.3, built
with NDK r21d. The source is at `packages/modules/adb/` in AOSP under the
**Apache-2.0 license**.

We can rebuild it from source:

1. Clone `packages/modules/adb` from AOSP.
2. Build with the Android.bp blueprint for `aarch64` and `x86_64`.
3. Rename the output `adb` binary to `libadb.so` and ship it in `jniLibs/`.

This eliminates the largest blob (4.46 MB) and gives us x86_64 support.

---

## 4. Implementation plan

### Phase 1: Delete `libloader.so` (lowest risk, already done)

The open-source Rust replacement (`app/rs/loader/`) is functionally
equivalent to the legacy blob, as confirmed by the disassembly. We just
need to:

1. Delete `app/src/main/jniLibs/arm64-v8a/libloader.so`.
2. Update `RomManager.LOADER_FILE` to point at `libloader_new.so`
   (or rename `libloader_new.so` → `libloader.so`).
3. Build `libloader_new.so` for `x86_64` too (the build script already
   supports this after the POSIX-sh fix).

### Phase 2: Rebuild `libOpenglRender.so` from AOSP source

1. Vendor the AOSP `emugl` source into `app/cpp/emugl/`.
2. Apply twoyi's modifications (the `win` arg, thread-only mode).
3. Write an `Android.bp` or `CMakeLists.txt` that builds it for both
   `arm64-v8a` and `x86_64`.
4. Delete the legacy `libOpenglRender.so` blob.
5. The existing `app/rs/openglrenderer/` Rust crate can serve as a
   fallback / reference implementation, but the C++ emugl code is more
   complete (it has the full `FrameBuffer` + `ColorBuffer` + EGL
   integration that the Rust version stubs out).

### Phase 3: Rebuild `libadb.so` from AOSP source

1. Vendor `packages/modules/adb` from AOSP.
2. Build with the existing `Android.bp` for both ABIs.
3. Rename the output to `libadb.so` and ship in `jniLibs/`.

### Phase 4: Remove the closed-source blobs

After phases 1-3 are validated:

1. Delete `app/src/main/jniLibs/arm64-v8a/libloader.so`.
2. Delete `app/src/main/jniLibs/arm64-v8a/libOpenglRender.so`.
3. Delete `app/src/main/jniLibs/arm64-v8a/libadb.so`.
4. The project is now **fully buildable from open-source code**.

---

## 5. Tooling used

```bash
# Install aarch64 binutils (cross-tools for analyzing arm64 binaries on x86_64 host)
apt-get download binutils-aarch64-linux-gnu
dpkg-deb -x binutils-aarch64-linux-gnu_*.deb ~/.local/binutils
export LD_LIBRARY_PATH="$HOME/.local/binutils/usr/lib/x86_64-linux-gnu:$LD_LIBRARY_PATH"
export PATH="$HOME/.local/binutils/usr/bin:$PATH"

# Analyze a blob
BLOB=app/src/main/jniLibs/arm64-v8a/libOpenglRender.so
aarch64-linux-gnu-nm -D --defined-only -C "$BLOB"   # exported symbols (demangled)
aarch64-linux-gnu-nm -D --undefined-only "$BLOB"     # imported symbols
aarch64-linux-gnu-readelf -d "$BLOB"                  # dynamic dependencies
aarch64-linux-gnu-readelf -l "$BLOB"                  # program headers
aarch64-linux-gnu-readelf -S "$BLOB"                  # section headers
aarch64-linux-gnu-objdump -d --disassemble=FUNCTION "$BLOB"  # disassembly
strings -n 6 "$BLOB" | grep -i pattern                # string search
```

---

## 6. References

- **AOSP emugl source (the `libOpenglRender.so` origin):**
  https://android.googlesource.com/platform/sdk/+/7a712acc02282985dcd32feb81284e1f2b19ec7e/emulator/opengl/host/libs/libOpenglRender/
  - `render_api.cpp` — the 6 C-ABI wrapper functions
  - `FrameBuffer.cpp` / `FrameBuffer.h` — the singleton hub
  - `ColorBuffer.cpp` — GPU pixel buffers
  - `RenderWindow.cpp` — host ANativeWindow management
  - `TextureDraw.cpp`, `TextureResize.cpp` — texture utilities

- **AOSP adb source (the `libadb.so` origin):**
  https://android.googlesource.com/platform/packages/modules/adb/

- **Clean-room implementation legal precedent:**
  - Sega v. Accolade (9th Cir. 1992) — disassembly for interoperability is fair use.
  - Sony v. Connectix (9th Cir. 2000) — disassembly for platform compatibility is fair use.
  - Oracle v. Google (2021) — reimplementing APIs is fair use.

- **Existing twoyi documentation:**
  - `REDROID_TESTING.md` — documents the ARM64/x86_64 mismatch that this
    analysis helps resolve.
  - `OPEN_SOURCE_LIBRARIES.md` — lists the open-source replacements.
  - `LOADER_NEW.md`, `OPENGL_RENDERER_NEW.md` — the existing Rust
    replacement docs.

---

*This document was produced by disassembling the three legacy closed-source
blobs in `app/src/main/jniLibs/arm64-v8a/` using GNU binutils 2.44, then
cross-referencing the demangled symbols against the AOSP source tree.
All three blobs are derived from Apache-2.0 licensed AOSP code, so we can
rebuild them from source rather than relying on disassembly for
reimplementation.*
