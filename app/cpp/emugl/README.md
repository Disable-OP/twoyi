# libOpenglRender.so — vendored AOSP emugl source (100% open source)

This directory contains the AOSP emugl renderer source, vendored into
the twoyi repo so that `libOpenglRender.so` can be built entirely from
open source code (task **AOSP-VENDOR-1**). No closed-source blobs are
shipped.

## Source

Fetched from `https://android.googlesource.com/platform/sdk` at commit
`7a712acc02282985dcd32feb81284e1f2b19ec7e` ("Publish and use
libOpenglRender interface header"). The following subtrees were pulled
in via `git sparse-checkout`:

| Path in AOSP | Vendored to | Notes |
|---|---|---|
| `emulator/opengl/host/libs/libOpenglRender/` | `libOpenglRender/` | main renderer (lightly patched — see below) |
| `emulator/opengl/shared/OpenglCodecCommon/` | `OpenglCodecCommon/` | codec / stream layer (UnixStream.cpp patched) |
| `emulator/opengl/shared/OpenglOsUtils/` | `OpenglOsUtils/` | OS utils (Unix-only; Windows variants excluded) |
| `emulator/opengl/host/libs/GLESv1_dec/` | `GLESv1_dec/` | GLESv1 decoder |
| `emulator/opengl/host/libs/GLESv2_dec/` | `GLESv2_dec/` | GLESv2 decoder |
| `emulator/opengl/host/include/libOpenglRender/` | `include/libOpenglRender/` | public headers (render_api_platform_types.h patched) |
| `emulator/opengl/system/{renderControl_enc,GLESv1_enc,GLESv2_enc}/` | `generated/` | emugen input specs → generated decoder sources |

The `NativeLinuxSubWindow.cpp`, `NativeMacSubWindow.m`,
`NativeWindowsSubWindow.cpp` and `Win32PipeStream.cpp` files from the
AOSP tree are deliberately **not** part of the active build (they're
platform-specific X11 / Win32 / Carbon code that doesn't apply to
Android). They are still present in the tree as part of the reference
AOSP source; they are simply not listed in `CMakeLists.txt`'s
`EMUGL_SOURCES`.

## Generated decoder sources

`generated/{renderControl_dec,gl_dec,gl2_dec}/` contains the output of
the AOSP `emugen` wire-protocol code generator, run once at vendor
time. The generated `.cpp` / `.h` files are committed so that
**`build.sh` does not require `emugen` on the host** — the build is
fully self-contained.

To regenerate (only needed if the `.in` / `.attrib` / `.types` specs in
`emulator/opengl/system/*_enc/` change upstream):

```bash
# build emugen from emulator/opengl/host/tools/emugen/
g++ -std=c++11 -O2 -D_GNU_SOURCE -include unistd.h \
    -I$AOSP/emulator/opengl/host/tools/emugen \
    -o emugen \
    $AOSP/emulator/opengl/host/tools/emugen/{ApiGen,EntryPoint,main,strUtils,TypeFactory}.cpp

./emugen -i $AOSP/emulator/opengl/system/renderControl_enc -D generated/renderControl_dec renderControl
./emugen -i $AOSP/emulator/opengl/system/GLESv1_enc       -D generated/gl_dec            gl
./emugen -i $AOSP/emulator/opengl/system/GLESv2_enc       -D generated/gl2_dec           gl2
```

Also copy the type headers next to the generated files:
```bash
cp $AOSP/emulator/opengl/system/renderControl_enc/renderControl_types.h generated/renderControl_dec/
cp $AOSP/emulator/opengl/system/GLESv1_enc/gl_types.h                   generated/gl_dec/
cp $AOSP/emulator/opengl/system/GLESv2_enc/gl2_types.h                  generated/gl2_dec/
```

## Patches applied to AOSP source

> **Note:** the patched AOSP source files under `libOpenglRender/`,
> `OpenglCodecCommon/`, `OpenglOsUtils/`, `GLESv1_dec/`, `GLESv2_dec/` and
> `generated/` are kept in the tree as a reference for the original
> "compose the AOSP FrameBuffer / RenderServer API" design. They are **not**
> part of the active build — `CMakeLists.txt` compiles only `twoyi_api.cpp`
> (see "twoyi-specific additions" below), which talks to EGL / GLESv2
> directly. The patches are documented here for historical / future-reference
> purposes.

| File | Patch | Why |
|---|---|---|
| `include/libOpenglRender/render_api_platform_types.h` | add `__ANDROID__` branch using `void*` | Android has no X11; ANativeWindow* is an opaque pointer |
| `libOpenglRender/EGLDispatch.cpp` | `EMUGL_LIBNAME("EGL_translator")` → `"libEGL.so"` | use the system EGL, not the desktop-GL translator |
| `libOpenglRender/GLDispatch.cpp` | `EMUGL_LIBNAME("GLES_CM_translator")` → `"libGLESv1_CM.so"` | use the system GLESv1 |
| `libOpenglRender/GL2Dispatch.cpp` | `EMUGL_LIBNAME("GLES_V2_translator")` → `"libGLESv2.so"` | use the system GLESv2 |
| `libOpenglRender/render_api.cpp` | `static RenderServer *s_renderThread` → `RenderServer *s_renderThread` | was needed by the now-deleted reference `twoyi/twoyi_api.cpp` so it could `extern` the global; the patch is preserved on the reference source for historical consistency |
| `OpenglCodecCommon/UnixStream.cpp` | `make_unix_path()` rewritten | pipe path = `$TWOYI_ROOTFS/opengles{,2,3}` (default `/data/data/io.twoyi/rootfs`) instead of `/tmp/android-$USER/qemu-gles-$port` |

## twoyi-specific additions

| File | Purpose |
|---|---|
| `twoyi_api.cpp` | **The only file actually compiled into `libOpenglRender.so`.** Implements the 6 twoyi-flavored C-ABI entry points (`startOpenGLRenderer`, `setNativeWindow`, `resetSubWindow`, `removeSubWindow`, `destroyOpenGLSubwindow`, `repaintOpenGLDisplay`) plus the 4 `dl*_ex` legacy wrappers, by talking to the system EGL / GLESv2 directly (`eglGetDisplay` / `eglInitialize` / `eglChooseConfig` / `eglCreateContext` / `eglCreateWindowSurface` / `eglSwapBuffers`). A background render thread owns the EGL context for its entire lifetime and performs surface (re)creation, `glClear` and `eglSwapBuffers` under a mutex, woken by a condition variable — this avoids the `EGL_BAD_ACCESS` that would result from making the context current on a second thread. |
| `compat/` | shim layer for the Android platform-private headers that the reference AOSP emugl source uses (`cutils/{threads,atomic,log,sockets}.h`, `utils/{threads,Errors,Vector,List,String8,KeyedVector,RefBase}.h`) — implemented on top of pthreads / std::vector / std::map / liblog. Not compiled into the shipping `.so`; kept so the reference AOSP source can be built if a future task re-enables the full FrameBuffer / RenderServer pipeline. |

## Build

```bash
# from app/cpp/
./build.sh all            # builds both arm64-v8a and x86_64
./build.sh arm64-v8a      # one ABI only
```

Output is written to `../../src/main/jniLibs/<abi>/libOpenglRender.so`.

The build is driven by the Gradle `cmakeBuild` task (see
`app/build.gradle`), which runs before `cargoBuild` (the Rust
`libtwoyi.so` build) because `app/rs/build.rs` links against
`libOpenglRender.so`.

## Required C-ABI symbols

The Rust FFI declarations in `app/rs/src/renderer_bindings.rs` require
these 6 symbols — all present in both ABIs:

```
startOpenGLRenderer
destroyOpenGLSubwindow
repaintOpenGLDisplay
setNativeWindow
resetSubWindow
removeSubWindow
```

Verify with:
```bash
nm -D --defined-only app/src/main/jniLibs/<abi>/libOpenglRender.so \
    | grep -E 'startOpenGLRenderer|destroyOpenGLSubwindow|repaintOpenGLDisplay|setNativeWindow|resetSubWindow|removeSubWindow'
```

## License

The AOSP emugl source is Apache 2.0 licensed (see the original
`Copyright (C) 2011 The Android Open Source Project` headers in each
file). The twoyi-specific additions (`twoyi_api.cpp`, `compat/`) are
Mozilla Public License 2.0.
