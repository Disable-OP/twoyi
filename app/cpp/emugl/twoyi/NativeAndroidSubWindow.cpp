/*
 * NativeAndroidSubWindow.cpp — twoyi-specific replacement for the
 * AOSP NativeLinuxSubWindow.cpp. On Android there is no X11; the
 * ANativeWindow passed in from Java IS the EGLNativeWindow, so we
 * just hand it straight back to the EGL driver.
 *
 * Original AOSP file: emulator/opengl/host/libs/libOpenglRender/NativeLinuxSubWindow.cpp
 * (kept out of the build; this file replaces it)
 */
#include "NativeSubWindow.h"
#include <EGL/egl.h>

EGLNativeWindowType createSubWindow(FBNativeWindowType p_window,
                                    EGLNativeDisplayType *display_out,
                                    int /*x*/, int /*y*/,
                                    int /*width*/, int /*height*/)
{
    if (display_out) {
        *display_out = EGL_DEFAULT_DISPLAY;
    }
    /* On Android the FBNativeWindowType IS the ANativeWindow*, which
     * is also the EGLNativeWindowType. No child window needs to be
     * created. */
    return (EGLNativeWindowType)p_window;
}

void destroySubWindow(EGLNativeDisplayType /*dis*/, EGLNativeWindowType /*win*/)
{
    /* Nothing to do — the ANativeWindow is owned by the caller
     * (the twoyi Java code / Surface). */
}
