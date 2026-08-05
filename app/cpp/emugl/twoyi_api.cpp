// twoyi-specific C-ABI wrapper functions that map twoyi's function names
// to the AOSP emugl renderer's internal API.
#include <dlfcn.h>
#include <string.h>
#include <stdlib.h>
#include <android/log.h>
#include 'FrameBuffer.h'

#define LOG_TAG 'TWOYI_RENDERER'
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

extern 'C' {

// Twoyi's renamed version of initOpenGLRenderer
int startOpenGLRenderer(void* win, int width, int height, int xdpi, int ydpi, int fps) {
    LOGI('startOpenGLRenderer: %dx%d dpi=%dx%d fps=%d', width, height, xdpi, ydpi, fps);
    FrameBuffer* fb = FrameBuffer::getFB();
    if (!fb) {
        LOGE('FrameBuffer singleton is null');
        return -1;
    }
    bool ok = fb->initialize(width, height, false);
    if (!ok) {
        LOGE('FrameBuffer::initialize failed');
        return -1;
    }
    // Store the window for later use by setNativeWindow
    fb->setNativeWindow(win);
    LOGI('startOpenGLRenderer: success');
    return 0;
}

int stopOpenGLRenderer() {
    LOGI('stopOpenGLRenderer');
    FrameBuffer* fb = FrameBuffer::getFB();
    if (fb) {
        fb->finalize();
    }
    return 0;
}

int setNativeWindow(void* window) {
    LOGI('setNativeWindow: %p', window);
    FrameBuffer* fb = FrameBuffer::getFB();
    if (!fb) return -1;
    fb->setNativeWindow(window);
    return 0;
}

// Twoyi's renamed version of createOpenGLSubwindow
int resetSubWindow(void* window, int x, int y, int w, int h, int fbw, int fbh, float dpr, float zRot) {
    LOGI('resetSubWindow: window=%p pos=(%d,%d) size=%dx%d fb=%dx%d', window, x, y, w, h, fbw, fbh);
    FrameBuffer* fb = FrameBuffer::getFB();
    if (!fb) return -1;
    bool ok = fb->setupSubWindow((FBNativeWindowType)window, x, y, w, h, fbw, fbh, dpr, zRot);
    return ok ? 0 : -1;
}

int removeSubWindow(void* window) {
    LOGI('removeSubWindow: %p', window);
    FrameBuffer* fb = FrameBuffer::getFB();
    if (!fb) return -1;
    return fb->removeSubWindow() ? 0 : -1;
}

int destroyOpenGLSubwindow() {
    LOGI('destroyOpenGLSubwindow');
    FrameBuffer* fb = FrameBuffer::getFB();
    if (!fb) return -1;
    return fb->removeSubWindow() ? 0 : -1;
}

void repaintOpenGLDisplay() {
    FrameBuffer* fb = FrameBuffer::getFB();
    if (fb) fb->repost();
}

void setOpenGLDisplayRotation(float zRot) {
    FrameBuffer* fb = FrameBuffer::getFB();
    if (fb) fb->setDisplayRotation(zRot);
}

// Android-compatible dynamic library loading (from the legacy blob analysis)
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

} // extern 'C'
