// NativeLinuxSubWindow.cpp — Android version
//
// On Android, the native window is an ANativeWindow (passed in from the
// Java Surface via JNI). There is no X11 window to create — we just
// return the same window pointer and stub out the destroy/repaint
// functions (the FrameBuffer's EGL surface handles display).
//
// This file implements the declarations in NativeSubWindow.h.
#include "NativeSubWindow.h"

#include <android/log.h>

#define LOG_TAG "NativeSubWindow"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGW(...) __android_log_print(ANDROID_LOG_WARN, LOG_TAG, __VA_ARGS__)

// createSubWindow — on Android, the ANativeWindow is already created
// by the Java side (SurfaceView / Surface). We just pass it through.
// The display_out parameter is set to EGL_DEFAULT_DISPLAY since Android
// doesn't have a per-window display handle.
extern "C" EGLNativeWindowType createSubWindow(FBNativeWindowType p_window,
                                                EGLNativeDisplayType* display_out,
                                                int x, int y,
                                                int width, int height) {
    LOGI("createSubWindow: window=%p %dx%d", p_window, width, height);
    if (display_out) {
        *display_out = EGL_DEFAULT_DISPLAY;
    }
    return (EGLNativeWindowType)p_window;
}

// destroySubWindow — no-op on Android. The ANativeWindow is owned by
// the Java side and will be released when the Surface is destroyed.
extern "C" void destroySubWindow(EGLNativeDisplayType dis,
                                  EGLNativeWindowType win) {
    LOGI("destroySubWindow: display=%p window=%p", dis, win);
    // No-op — the ANativeWindow is managed by Java.
}
