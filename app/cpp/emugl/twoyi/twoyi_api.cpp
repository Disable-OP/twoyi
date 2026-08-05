/*
 * twoyi_api.cpp — the twoyi-flavored C-ABI entry points that
 * app/rs/src/renderer_bindings.rs declares.
 *
 * The upstream AOSP emugl render_api.cpp exposes a *different* API
 * (initOpenGLRenderer / createOpenGLSubwindow / destroyOpenGLSubwindow
 * / repaintOpenGLDisplay / stopOpenGLRenderer / setStreamMode). The
 * legacy twoyi libOpenglRender.so blob renamed/reshaped those into:
 *
 *   startOpenGLRenderer(win, w, h, xdpi, ydpi, fps)
 *   setNativeWindow(win)
 *   resetSubWindow(win, x, y, w, h, fbw, fbh, dpr, zRot)
 *   removeSubWindow(win)
 *   destroyOpenGLSubwindow()
 *   repaintOpenGLDisplay()
 *
 * This file provides the twoyi names by composing the existing AOSP
 * FrameBuffer / RenderServer / EGLDispatch / GLDispatch / GL2Dispatch
 * entry points. It also exports the dl*_ex wrappers that the legacy
 * blob shipped (some twoyi rootfs .so files still dlsym them).
 */
#include "libOpenglRender/render_api.h"
#include "FrameBuffer.h"
#include "RenderServer.h"
#include "EGLDispatch.h"
#include "GLDispatch.h"
#include "GL2Dispatch.h"

#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* render_api.cpp originally declares `static RenderServer *s_renderThread`.
 * We patch it to remove the `static` so this file can reference the same
 * global to check whether the renderer has already been started and to
 * install the RenderServer we create. */
extern RenderServer *s_renderThread;

/* Stash the most recent native window so resetSubWindow can fall back
 * to it if the caller passes NULL. */
static FBNativeWindowType g_nativeWindow = NULL;

extern "C" {

/* ------------------------------------------------------------------ */
/* twoyi-required entry points                                         */
/* ------------------------------------------------------------------ */

/* setNativeWindow — store the ANativeWindow for later use. The AOSP
 * renderer doesn't actually need this (the window is passed to
 * FrameBuffer::setupSubWindow via resetSubWindow), but the twoyi Rust
 * code calls it before resetSubWindow, so we keep a stash. */
int setNativeWindow(void *window)
{
    g_nativeWindow = (FBNativeWindowType)window;
    return 1;
}

/* startOpenGLRenderer — twoyi signature (win, w, h, xdpi, ydpi, fps).
 * Internally: init EGL/GL/GLES2 dispatch tables, initialize the
 * FrameBuffer, switch to Unix-socket stream mode, and start the
 * RenderServer thread listening on $TWOYI_ROOTFS/opengles.
 *
 * The `win`, `xdpi`, `ydpi`, `fps` parameters are accepted for ABI
 * compatibility with the legacy blob but are not used here — the AOSP
 * renderer gets the window from resetSubWindow() and doesn't honor
 * DPI/FPS. */
int startOpenGLRenderer(void *win, int width, int height,
                        int /*xdpi*/, int /*ydpi*/, int /*fps*/)
{
    if (s_renderThread != NULL) {
        /* already started */
        return 1;
    }

    if (win) {
        g_nativeWindow = (FBNativeWindowType)win;
    }

    /* Load system EGL / GLESv1 / GLESv2 dispatch tables. */
    if (!init_egl_dispatch()) {
        fprintf(stderr, "twoyi_api: init_egl_dispatch failed\n");
        return 0;
    }
    if (!init_gl_dispatch()) {
        fprintf(stderr, "twoyi_api: init_gl_dispatch failed\n");
        return 0;
    }
    /* GLES2 dispatch init failure is non-fatal — matches AOSP. */
    init_gl2_dispatch();

    /* Initialize the framebuffer at the requested dimensions. */
    if (!FrameBuffer::initialize(width, height, NULL, NULL)) {
        fprintf(stderr, "twoyi_api: FrameBuffer::initialize failed\n");
        return 0;
    }

    /* Use Unix domain sockets (path-based) so the renderer listens on
     * $TWOYI_ROOTFS/opengles. The port number is interpreted by
     * UnixStream as a path-suffix index (0 -> "opengles"). */
    setStreamMode(STREAM_MODE_UNIX);

    s_renderThread = RenderServer::create(0);
    if (!s_renderThread) {
        fprintf(stderr, "twoyi_api: RenderServer::create failed\n");
        return 0;
    }
    s_renderThread->start();

    return 1;
}

/* resetSubWindow — twoyi signature has 9 args (win, x, y, w, h, fbw,
 * fbh, dpr, zRot). The AOSP FrameBuffer::setupSubWindow only takes
 * (win, x, y, w, h, zRot) — fbw/fbh were already set at
 * startOpenGLRenderer time, and dpr is unused. We just forward. */
int resetSubWindow(void *p_window,
                   int wx, int wy, int ww, int wh,
                   int /*fbw*/, int /*fbh*/,
                   float /*dpr*/, float zRot)
{
    FBNativeWindowType win = (FBNativeWindowType)p_window;
    if (!win) win = g_nativeWindow;
    if (!win) return 0;
    return FrameBuffer::setupSubWindow(win, wx, wy, ww, wh, zRot) ? 1 : 0;
}

/* removeSubWindow — twoyi passes a window pointer (ignored — there is
 * only one subwindow), AOSP's FrameBuffer::removeSubWindow takes no
 * args. */
int removeSubWindow(void * /*window*/)
{
    return FrameBuffer::removeSubWindow() ? 1 : 0;
}

/* destroyOpenGLSubwindow / repaintOpenGLDisplay are already provided
 * by the AOSP render_api.cpp — no wrappers needed. */

/* ------------------------------------------------------------------ */
/* dl*_ex — legacy twoyi ABI for Android-7+ namespace-aware dlopen.   */
/* On modern Android dlopen() already handles the namespace fallback, */
/* so we just forward to the real libdl functions.                    */
/* ------------------------------------------------------------------ */
void *dlopen_ex(const char *filename, int flag) {
    return dlopen(filename, flag);
}
void *dlsym_ex(void *handle, const char *symbol) {
    return dlsym(handle, symbol);
}
int dlclose_ex(void *handle) {
    return dlclose(handle);
}
char *dlerror_ex(void) {
    return dlerror();
}

}  /* extern "C" */
