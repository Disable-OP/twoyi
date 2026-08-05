// NativeLinuxSubWindow.cpp — Android version
// On Android, the native window is an ANativeWindow, which is already
// provided by the Java Surface. No X11 window creation needed.
#include 'NativeSubWindow.h'
#include <android/log.h>

#define LOG_TAG 'NativeSubWindow'
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)

FBNativeWindowType createSubWindow(FBNativeWindowType p_window, int x, int y, int width, int height, float dpr, float zRot) {
    LOGI('createSubWindow: %p %dx%d', p_window, width, height);
    return p_window;
}

void destroySubWindow(FBNativeWindowType p_window) {
    LOGI('destroySubWindow: %p', p_window);
}

int repaintSubWindow(FBNativeWindowType p_window) {
    return 0;
}

int setSubWindowRotation(FBNativeWindowType p_window, float zRot) {
    return 0;
}

int setSubWindowTranslation(FBNativeWindowType p_window, float dx, float dy) {
    return 0;
}
