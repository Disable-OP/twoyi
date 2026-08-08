// twoyi_glue.cpp — FFI glue between the Rust renderer_bindings.rs and
// the AOSP emugl render_api.cpp.
//
// The Rust side (renderer_bindings.rs) declares these six C-ABI symbols:
//
//   startOpenGLRenderer(win, w, h, xdpi, ydpi, fps) -> int
//   setNativeWindow(window) -> int
//   resetSubWindow(window, x, y, w, h, fbw, fbh, dpr, zRot) -> int
//   removeSubWindow(window) -> int
//   destroyOpenGLSubwindow() -> int
//   repaintOpenGLDisplay()
//
// The AOSP emugl render_api.cpp provides:
//
//   initOpenGLRenderer(w, h, portNum, onPost, ctx) -> int
//   createOpenGLSubwindow(window, x, y, w, h, zRot) -> int
//   destroyOpenGLSubwindow() -> int
//   repaintOpenGLDisplay()
//   setOpenGLDisplayRotation(zRot)
//   stopOpenGLRenderer() -> int
//
// This file provides the six Rust-facing symbols by delegating to the
// AOSP functions. It replaces the old twoyi_api.cpp stub (which had its
// own EGL clear-loop) — now the real AOSP pipeline (RenderServer +
// RenderThread + FrameBuffer + GL decoders) executes guest GL commands.

#include "libOpenglRender/render_api.h"
#include "FrameBuffer.h"

#include <android/log.h>
#include <stdlib.h>  // getenv

#define LOG_TAG "TWOYI_RENDERER"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO,  LOG_TAG, __VA_ARGS__)
#define LOGW(...) __android_log_print(ANDROID_LOG_WARN,  LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

// ---------------------------------------------------------------------------
// startOpenGLRenderer — called from core.rs::init_renderer.
//
// Initializes the AOSP emugl renderer with:
//   - STREAM_MODE_UNIX so RenderServer listens on a Unix socket
//     ($TWOYI_ROOTFS/opengles) instead of a TCP port.
//   - portNum = 0 (Unix socket mode uses port % 3 to pick the socket
//     suffix; 0 → "opengles").
//   - onPost = NULL (no framebuffer callback needed; the subwindow
//     displays directly).
//
// Then creates the subwindow (binds the ANativeWindow to the
// FrameBuffer so EGL renders onto it).
//
// Returns 0 on success, non-zero on failure (matching the old stub's
// contract — the Rust side checks `if result != 0`).
// ---------------------------------------------------------------------------
extern "C" int startOpenGLRenderer(void* win, int width, int height,
                                   int xdpi, int ydpi, int fps) {
    (void)xdpi;  // AOSP initOpenGLRenderer doesn't take DPI
    (void)ydpi;
    (void)fps;   // AOSP renderer swaps on guest demand, not fixed FPS

    LOGI("startOpenGLRenderer: win=%p %dx%d", win, width, height);

    // 1. initLibrary — loads the EGL/GL dispatch tables (dlopens
    //    libEGL.so / libGLESv2.so from /system/lib<abi>/).
    if (!initLibrary()) {
        LOGE("initLibrary failed — cannot load EGL/GL dispatch tables");
        return -1;
    }
    LOGI("initLibrary: OK");

    // 2. setStreamMode — Unix socket (not TCP).
    if (!setStreamMode(STREAM_MODE_UNIX)) {
        LOGE("setStreamMode(UNIX) failed");
        return -1;
    }
    LOGI("setStreamMode(UNIX): OK");

    // 3. initOpenGLRenderer — starts FrameBuffer + RenderServer.
    //    portNum=0 → RenderServer listens on $TWOYI_ROOTFS/opengles.
    //    (UnixStream.cpp uses port % 3 to pick opengles{,2,3}.)
    //    The TWOYI_ROOTFS env var must be set before this call —
    //    UnixStream::listen() calls getenv("TWOYI_ROOTFS") to build
    //    the socket path. The Rust side sets it in core.rs before
    //    calling startOpenGLRenderer.
    const char* rootfs_env = getenv("TWOYI_ROOTFS");
    LOGI("TWOYI_ROOTFS=%s", rootfs_env ? rootfs_env : "(not set)");
    if (!rootfs_env || !rootfs_env[0]) {
        LOGE("TWOYI_ROOTFS not set — UnixStream will use default /data/data/io.twoyi/rootfs");
        // Not fatal — the default path might work if the app data dir
        // matches. But on work profiles it would be wrong.
    }
    if (!initOpenGLRenderer(width, height, /*portNum=*/0,
                            /*onPost=*/NULL, /*onPostContext=*/NULL)) {
        LOGE("initOpenGLRenderer failed — FrameBuffer::initialize or RenderServer::create returned false");
        LOGE("  Possible causes:");
        LOGE("    1. TWOYI_ROOTFS dir doesn't exist or isn't writable");
        LOGE("    2. EGL init failed (no EGL display)");
        LOGE("    3. Unix socket bind failed (path too long or no permission)");
        return -1;
    }
    LOGI("initOpenGLRenderer: OK (listening on $TWOYI_ROOTFS/opengles)");

    // 4. createOpenGLSubwindow — bind the ANativeWindow so EGL can
    //    render onto it. If win is NULL, skip (the subwindow can be
    //    created later via resetSubWindow).
    if (win != NULL) {
        if (!createOpenGLSubwindow((FBNativeWindowType)win,
                                   /*x=*/0, /*y=*/0,
                                   width, height, /*zRot=*/0.0f)) {
            LOGE("createOpenGLSubwindow failed");
            // Non-fatal — the renderer is running, just no display surface.
            // The guest can still boot; resetSubWindow can be called later.
        } else {
            LOGI("createOpenGLSubwindow: OK");
        }
    }

    return 0;
}

// ---------------------------------------------------------------------------
// setNativeWindow — update the subwindow's ANativeWindow.
//
// In the AOSP API, this is done by destroyOpenGLSubwindow + createOpenGLSubwindow.
// We destroy the old subwindow and create a new one with the new window.
// ---------------------------------------------------------------------------
extern "C" int setNativeWindow(void* window) {
    LOGI("setNativeWindow: %p", window);
    if (window == NULL) {
        return 0;
    }
    // Destroy the old subwindow (if any), then create a new one.
    destroyOpenGLSubwindow();
    FrameBuffer* fb = FrameBuffer::getFB();
    int w = fb ? fb->getWidth() : 1080;
    int h = fb ? fb->getHeight() : 1920;
    if (!createOpenGLSubwindow((FBNativeWindowType)window,
                               /*x=*/0, /*y=*/0, w, h, /*zRot=*/0.0f)) {
        LOGE("setNativeWindow: createOpenGLSubwindow failed");
        return -1;
    }
    return 0;
}

// ---------------------------------------------------------------------------
// resetSubWindow — update the subwindow's window + dimensions.
//
// Called from core.rs when the Surface is recreated (e.g. orientation
// change). We destroy + recreate the subwindow with the new params.
// ---------------------------------------------------------------------------
extern "C" int resetSubWindow(void* window, int x, int y,
                              int w, int h,
                              int fbw, int fbh,
                              float dpr, float zRot) {
    (void)fbw; (void)fbh; (void)dpr;  // AOSP subwindow doesn't use these
    LOGI("resetSubWindow: win=%p pos=(%d,%d) size=%dx%d zRot=%.2f",
         window, x, y, w, h, zRot);
    if (window == NULL) {
        return 0;
    }
    destroyOpenGLSubwindow();
    if (!createOpenGLSubwindow((FBNativeWindowType)window, x, y, w, h, zRot)) {
        LOGE("resetSubWindow: createOpenGLSubwindow failed");
        return -1;
    }
    return 0;
}

// ---------------------------------------------------------------------------
// removeSubWindow — destroy the subwindow (keep the renderer running).
// ---------------------------------------------------------------------------
extern "C" int removeSubWindow(void* /*window*/) {
    LOGI("removeSubWindow");
    destroyOpenGLSubwindow();
    return 0;
}

// ---------------------------------------------------------------------------
// destroyOpenGLSubwindow — AOSP provides this directly. Just delegate.
// ---------------------------------------------------------------------------
// (No wrapper needed — the AOSP destroyOpenGLSubwindow has the same
//  signature the Rust side expects. It's already declared in render_api.h
//  and exported from render_api.cpp.)

// ---------------------------------------------------------------------------
// repaintOpenGLDisplay — AOSP provides this directly. Just delegate.
// ---------------------------------------------------------------------------
// (No wrapper needed — same signature.)

// ---------------------------------------------------------------------------
// Android-compatible dynamic library loading helpers.
// The legacy closed-source blob exported these; some Java code may still
// try to dlsym them, so we provide transparent wrappers.
// ---------------------------------------------------------------------------
#include <dlfcn.h>

extern "C" {

void* dlopen_ex(const char* filename, int flag) {
    return dlopen(filename, flag);
}

void* dlsym_ex(void* handle, const char* symbol) {
    return dlsym(handle, symbol);
}

int dlclose_ex(void* handle) {
    return dlclose(handle);
}

const char* dlerror_ex() {
    return dlerror();
}

}  // extern "C"
