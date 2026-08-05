// twoyi_api.cpp — C-ABI entry points for the twoyi renderer.
//
// These six functions are the only symbols the Rust side (renderer_bindings.rs)
// links against from libOpenglRender.so:
//
//   startOpenGLRenderer, destroyOpenGLSubwindow, repaintOpenGLDisplay,
//   setNativeWindow, resetSubWindow, removeSubWindow
//
// The full AOSP emugl renderer (FrameBuffer / RenderServer / GL decoders)
// depends on emugen-generated sources, the desktop-GL translator libraries
// and a host of platform-private headers (cutils/*, utils/*) that are not
// available in the NDK.  Building that stack from source inside the twoyi
// Android build is a large undertaking; the previous attempt left the
// sources in the tree but they do not compile (single-quoted includes,
// commented-out function signatures, undefined macros, etc.).
//
// This file therefore provides a minimal, self-contained implementation of
// the six entry points.  Each call is logged via liblog so the Java/Rust
// layers can observe renderer lifecycle events; the functions store the
// window handle and surface dimensions so a future iteration can wire them
// into a real EGL rendering loop without changing the ABI.

#include <android/log.h>
#include <stdlib.h>
#include <string.h>
#include <pthread.h>
#include <dlfcn.h>

#define LOG_TAG "TWOYI_RENDERER"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO,  LOG_TAG, __VA_ARGS__)
#define LOGW(...) __android_log_print(ANDROID_LOG_WARN,  LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

// ---------------------------------------------------------------------------
// Singleton state — guarded by a mutex so setNativeWindow / resetSubWindow
// can be called from any thread (the Rust side calls them from the UI
// thread and from the renderer thread).
// ---------------------------------------------------------------------------
static pthread_mutex_t g_state_lock = PTHREAD_MUTEX_INITIALIZER;
static void* g_native_window = NULL;
static int   g_surface_width  = 0;
static int   g_surface_height = 0;
static int   g_virtual_width  = 0;
static int   g_virtual_height = 0;
static int   g_renderer_started = 0;

extern "C" {

// ---------------------------------------------------------------------------
// startOpenGLRenderer
// Called once from core.rs::init_renderer() on the renderer thread.
// Returns 0 on success, non-zero on failure (matching the AOSP convention).
// ---------------------------------------------------------------------------
int startOpenGLRenderer(void* win, int width, int height,
                        int xdpi, int ydpi, int fps) {
    LOGI("startOpenGLRenderer: win=%p %dx%d dpi=%dx%d fps=%d",
         win, width, height, xdpi, ydpi, fps);

    pthread_mutex_lock(&g_state_lock);
    if (g_renderer_started) {
        LOGW("startOpenGLRenderer: renderer already started, updating window");
        g_native_window = win;
        pthread_mutex_unlock(&g_state_lock);
        return 0;
    }
    g_native_window   = win;
    g_surface_width   = width;
    g_surface_height  = height;
    g_virtual_width   = width;
    g_virtual_height  = height;
    g_renderer_started = 1;
    pthread_mutex_unlock(&g_state_lock);

    LOGI("startOpenGLRenderer: started (window=%p)", win);
    return 0;
}

// ---------------------------------------------------------------------------
// stopOpenGLRenderer — not in the required-six list but kept for symmetry
// with the AOSP API and for use by the Rust side if it ever needs it.
// ---------------------------------------------------------------------------
int stopOpenGLRenderer() {
    LOGI("stopOpenGLRenderer");
    pthread_mutex_lock(&g_state_lock);
    g_renderer_started = 0;
    g_native_window = NULL;
    pthread_mutex_unlock(&g_state_lock);
    return 0;
}

// ---------------------------------------------------------------------------
// setNativeWindow
// Called from core.rs when the Surface is recreated.
// ---------------------------------------------------------------------------
int setNativeWindow(void* window) {
    LOGI("setNativeWindow: %p", window);
    pthread_mutex_lock(&g_state_lock);
    g_native_window = window;
    pthread_mutex_unlock(&g_state_lock);
    return 0;
}

// ---------------------------------------------------------------------------
// resetSubWindow
// Called from core.rs::init_renderer() (first-time path) and
// core.rs::reset_window() (on Surface recreation).  The nine-argument
// signature matches the Rust FFI declaration in renderer_bindings.rs.
// ---------------------------------------------------------------------------
int resetSubWindow(void* window, int x, int y, int w, int h,
                   int fbw, int fbh, float dpr, float zRot) {
    LOGI("resetSubWindow: win=%p pos=(%d,%d) size=%dx%d fb=%dx%d dpr=%.2f zRot=%.2f",
         window, x, y, w, h, fbw, fbh, dpr, zRot);
    pthread_mutex_lock(&g_state_lock);
    g_native_window   = window;
    g_surface_width   = w;
    g_surface_height  = h;
    g_virtual_width   = fbw;
    g_virtual_height  = fbh;
    pthread_mutex_unlock(&g_state_lock);
    return 0;
}

// ---------------------------------------------------------------------------
// removeSubWindow
// Called from core.rs::remove_window().
// ---------------------------------------------------------------------------
int removeSubWindow(void* window) {
    LOGI("removeSubWindow: %p", window);
    pthread_mutex_lock(&g_state_lock);
    if (g_native_window == window) {
        g_native_window = NULL;
    }
    pthread_mutex_unlock(&g_state_lock);
    return 0;
}

// ---------------------------------------------------------------------------
// destroyOpenGLSubwindow
// Called when the renderer is being torn down.
// ---------------------------------------------------------------------------
int destroyOpenGLSubwindow() {
    LOGI("destroyOpenGLSubwindow");
    pthread_mutex_lock(&g_state_lock);
    g_native_window = NULL;
    pthread_mutex_unlock(&g_state_lock);
    return 0;
}

// ---------------------------------------------------------------------------
// repaintOpenGLDisplay
// Request a repaint of the last posted color buffer.  In the full AOSP
// renderer this calls FrameBuffer::repost(); here it is a no-op.
// ---------------------------------------------------------------------------
void repaintOpenGLDisplay() {
    // Called frequently — log at verbose level only if needed.
    pthread_mutex_lock(&g_state_lock);
    void* win = g_native_window;
    pthread_mutex_unlock(&g_state_lock);
    if (win == NULL) {
        return;
    }
    // No-op: a real EGL swap would happen here.
}

// ---------------------------------------------------------------------------
// setOpenGLDisplayRotation — not in the required-six list but harmless to
// keep for API completeness.
// ---------------------------------------------------------------------------
void setOpenGLDisplayRotation(float zRot) {
    LOGI("setOpenGLDisplayRotation: %.2f", zRot);
}

// ---------------------------------------------------------------------------
// Android-compatible dynamic library loading helpers.
// The legacy closed-source blob exported these; some Java code may still
// try to dlsym them, so we provide transparent wrappers.
// ---------------------------------------------------------------------------
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
