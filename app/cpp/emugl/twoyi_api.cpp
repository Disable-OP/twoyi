// twoyi_api.cpp — Real EGL rendering for libOpenglRender.so.
//
// These six C-ABI functions are the only symbols the Rust side
// (renderer_bindings.rs) links against from libOpenglRender.so:
//
//   startOpenGLRenderer, destroyOpenGLSubwindow, repaintOpenGLDisplay,
//   setNativeWindow, resetSubWindow, removeSubWindow
//
// Unlike the previous logging-stub implementation, this version creates a
// real EGL context bound to the ANativeWindow the Java/Rust side hands
// us, and runs a background render thread that periodically clears the
// color buffer and swaps it to the display.  The screen therefore shows
// a black (or any single-color) frame instead of nothing, which is
// enough for the guest's SurfaceFlinger init to proceed — it expects an
// EGL surface to exist and to be swap-able.  Later, the guest's GL
// commands will travel through the pipe and be replayed onto this same
// surface, but for now we just need a live EGL pipeline.
//
// Threading model:
//   * The EGL context is current on the render thread only.
//   * API entry points (called from the UI / renderer-control thread)
//     never touch EGL directly — they set pending-window / pending-remove
//     flags and signal the render thread, which performs the actual
//     surface (re)creation, GL clears and eglSwapBuffers under the
//     global mutex.  This avoids the EGL_BAD_ACCESS that would result
//     from trying to make the context current on a second thread.

#include <EGL/egl.h>
#include <GLES2/gl2.h>
#include <android/native_window.h>
#include <android/log.h>

#include <stdlib.h>
#include <string.h>
#include <pthread.h>
#include <unistd.h>
#include <dlfcn.h>
#include <time.h>
#include <stdbool.h>

#define LOG_TAG "TWOYI_RENDERER"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO,  LOG_TAG, __VA_ARGS__)
#define LOGW(...) __android_log_print(ANDROID_LOG_WARN,  LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

// ---------------------------------------------------------------------------
// Singleton state
// ---------------------------------------------------------------------------
static pthread_mutex_t g_lock  = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t  g_cond  = PTHREAD_COND_INITIALIZER;
static pthread_t       g_thread;
static bool            g_thread_running = false;

// Input flags (set by API thread, consumed by render thread).  All
// accesses must be performed while holding g_lock.
static ANativeWindow* g_pending_window  = NULL;  // switch surface to this window
static bool           g_pending_remove  = false; // destroy the current surface
static bool           g_shutdown         = false; // stop the render thread
static bool           g_repaint_now      = false; // extra swap requested

// EGL state — only ever touched by the render thread (and by
// startOpenGLRenderer before the render thread is spawned).
static EGLDisplay     g_display = EGL_NO_DISPLAY;
static EGLConfig      g_config  = NULL;
static EGLContext     g_context = EGL_NO_CONTEXT;
static EGLSurface     g_surface = EGL_NO_SURFACE;
static ANativeWindow* g_current_window = NULL;

// Renderer parameters (set by API thread, read by render thread — both
// under g_lock).
static int g_width  = 0;
static int g_height = 0;
static int g_fps    = 60;

static void* render_thread_main(void* arg);

// ---------------------------------------------------------------------------
// choose_egl_config — pick an RGBA8888 EGLConfig that supports
// window surfaces and OpenGL ES 2.0.
// ---------------------------------------------------------------------------
static EGLConfig choose_egl_config(EGLDisplay display) {
    const EGLint attribs[] = {
        EGL_SURFACE_TYPE,    EGL_WINDOW_BIT,
        EGL_BLUE_SIZE,       8,
        EGL_GREEN_SIZE,      8,
        EGL_RED_SIZE,        8,
        EGL_ALPHA_SIZE,      8,
        EGL_DEPTH_SIZE,      0,
        EGL_STENCIL_SIZE,    0,
        EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT,
        EGL_NONE
    };
    EGLConfig config = NULL;
    EGLint    num_configs = 0;
    if (!eglChooseConfig(display, attribs, &config, 1, &num_configs) ||
        num_configs < 1) {
        LOGE("eglChooseConfig failed: 0x%x", eglGetError());
        return NULL;
    }
    return config;
}

// ---------------------------------------------------------------------------
// create_window_surface — wraps eglCreateWindowSurface with logging.
// ---------------------------------------------------------------------------
static EGLSurface create_window_surface(EGLDisplay display,
                                        EGLConfig  config,
                                        ANativeWindow* window) {
    if (window == NULL) {
        return EGL_NO_SURFACE;
    }
    EGLSurface surf = eglCreateWindowSurface(display, config,
                                             (EGLNativeWindowType)window, NULL);
    if (surf == EGL_NO_SURFACE) {
        LOGE("eglCreateWindowSurface failed: 0x%x", eglGetError());
    }
    return surf;
}

// ---------------------------------------------------------------------------
// render_thread_main — owns the EGL context for its entire lifetime.
//
// Loop:
//   1. Acquire g_lock.
//   2. If shutdown requested, break out.
//   3. If a surface remove is pending, release context + destroy surface.
//   4. Else if a window switch is pending, release context, destroy old
//      surface, create new surface, make context current.
//   5. If a surface is current, glClear + eglSwapBuffers.
//   6. Wait on g_cond with a timeout of 1000/fps ms (so we render at the
//      requested frame rate even if nobody pings us, but wake immediately
//      when an API call signals the cond).
// ---------------------------------------------------------------------------
static void* render_thread_main(void* /*arg*/) {
    LOGI("render thread started");

    pthread_mutex_lock(&g_lock);

    // Make the context current on the initial surface (if any was
    // created by startOpenGLRenderer).
    if (g_surface != EGL_NO_SURFACE && g_context != EGL_NO_CONTEXT) {
        if (!eglMakeCurrent(g_display, g_surface, g_surface, g_context)) {
            LOGE("initial eglMakeCurrent failed: 0x%x", eglGetError());
        }
    }

    while (!g_shutdown) {
        // -- Handle pending surface removal -------------------------------
        if (g_pending_remove) {
            eglMakeCurrent(g_display, EGL_NO_SURFACE, EGL_NO_SURFACE,
                           EGL_NO_CONTEXT);
            if (g_surface != EGL_NO_SURFACE) {
                eglDestroySurface(g_display, g_surface);
                g_surface = EGL_NO_SURFACE;
            }
            g_current_window = NULL;
            g_pending_remove = false;
            LOGI("render thread: surface removed");
        }

        // -- Handle pending window switch ---------------------------------
        if (g_pending_window != NULL &&
            g_pending_window != g_current_window) {
            eglMakeCurrent(g_display, EGL_NO_SURFACE, EGL_NO_SURFACE,
                           EGL_NO_CONTEXT);
            if (g_surface != EGL_NO_SURFACE) {
                eglDestroySurface(g_display, g_surface);
            }
            g_surface = create_window_surface(g_display, g_config,
                                              g_pending_window);
            g_current_window = g_pending_window;
            g_pending_window = NULL;
            if (g_surface != EGL_NO_SURFACE) {
                if (!eglMakeCurrent(g_display, g_surface, g_surface,
                                    g_context)) {
                    LOGE("eglMakeCurrent after switch failed: 0x%x",
                         eglGetError());
                }
            }
            LOGI("render thread: surface switched to window %p",
                 g_current_window);
        }

        // -- Render one frame --------------------------------------------
        if (g_surface != EGL_NO_SURFACE && g_current_window != NULL) {
            glClearColor(0.0f, 0.0f, 0.0f, 1.0f);
            glClear(GL_COLOR_BUFFER_BIT);
            eglSwapBuffers(g_display, g_surface);
        }
        g_repaint_now = false;

        // -- Wait for next frame or a signal -----------------------------
        int fps = (g_fps > 0) ? g_fps : 60;
        struct timespec ts;
        clock_gettime(CLOCK_REALTIME, &ts);
        long add_ns = 1000000000L / fps;
        ts.tv_sec  += add_ns / 1000000000L;
        ts.tv_nsec += add_ns % 1000000000L;
        if (ts.tv_nsec >= 1000000000L) {
            ts.tv_sec  += 1;
            ts.tv_nsec -= 1000000000L;
        }
        pthread_cond_timedwait(&g_cond, &g_lock, &ts);
    }

    // -- Teardown (still holding the lock) --------------------------------
    eglMakeCurrent(g_display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
    if (g_surface != EGL_NO_SURFACE) {
        eglDestroySurface(g_display, g_surface);
        g_surface = EGL_NO_SURFACE;
    }
    g_current_window = NULL;

    pthread_mutex_unlock(&g_lock);
    LOGI("render thread exiting");
    return NULL;
}

extern "C" {

// ---------------------------------------------------------------------------
// startOpenGLRenderer(win, w, h, xdpi, ydpi, fps)
//
// Creates the EGL display, config, context and (if win != NULL) window
// surface, then spawns the render thread.  Returns 0 on success.
// ---------------------------------------------------------------------------
int startOpenGLRenderer(void* win, int width, int height,
                        int xdpi, int ydpi, int fps) {
    LOGI("startOpenGLRenderer: win=%p %dx%d dpi=%dx%d fps=%d",
         win, width, height, xdpi, ydpi, fps);

    pthread_mutex_lock(&g_lock);

    if (g_thread_running) {
        // Renderer is already running — just switch the window.
        LOGW("startOpenGLRenderer: already running, switching window");
        if (win != NULL) {
            g_pending_window = (ANativeWindow*)win;
            g_pending_remove = false;
        }
        g_width  = width;
        g_height = height;
        if (fps > 0) g_fps = fps;
        pthread_cond_signal(&g_cond);
        pthread_mutex_unlock(&g_lock);
        return 0;
    }

    g_width  = width;
    g_height = height;
    g_fps    = (fps > 0) ? fps : 60;

    // 1. EGL display
    g_display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    if (g_display == EGL_NO_DISPLAY) {
        LOGE("eglGetDisplay failed");
        pthread_mutex_unlock(&g_lock);
        return -1;
    }

    // 2. eglInitialize
    EGLint major = 0, minor = 0;
    if (!eglInitialize(g_display, &major, &minor)) {
        LOGE("eglInitialize failed: 0x%x", eglGetError());
        g_display = EGL_NO_DISPLAY;
        pthread_mutex_unlock(&g_lock);
        return -1;
    }
    LOGI("EGL initialized: version %d.%d", major, minor);

    // 3. eglChooseConfig (RGBA8888 + GLES2)
    g_config = choose_egl_config(g_display);
    if (g_config == NULL) {
        eglTerminate(g_display);
        g_display = EGL_NO_DISPLAY;
        pthread_mutex_unlock(&g_lock);
        return -1;
    }

    // 4. eglCreateContext (GLES 2)
    const EGLint ctx_attribs[] = {
        EGL_CONTEXT_CLIENT_VERSION, 2,
        EGL_NONE
    };
    g_context = eglCreateContext(g_display, g_config, EGL_NO_CONTEXT,
                                 ctx_attribs);
    if (g_context == EGL_NO_CONTEXT) {
        LOGE("eglCreateContext failed: 0x%x", eglGetError());
        eglTerminate(g_display);
        g_display = EGL_NO_DISPLAY;
        pthread_mutex_unlock(&g_lock);
        return -1;
    }

    // 5. eglCreateWindowSurface (if a window was provided)
    if (win != NULL) {
        g_surface = create_window_surface(g_display, g_config,
                                          (ANativeWindow*)win);
        g_current_window = (ANativeWindow*)win;
    } else {
        g_surface = EGL_NO_SURFACE;
        g_current_window = NULL;
    }

    // 6. Spawn the render thread (it calls eglMakeCurrent + loops).
    g_shutdown        = false;
    g_pending_window  = NULL;
    g_pending_remove  = false;
    g_repaint_now     = false;
    g_thread_running  = true;

    if (pthread_create(&g_thread, NULL, render_thread_main, NULL) != 0) {
        LOGE("pthread_create failed");
        if (g_surface != EGL_NO_SURFACE) {
            eglDestroySurface(g_display, g_surface);
            g_surface = EGL_NO_SURFACE;
        }
        eglDestroyContext(g_display, g_context);
        g_context = EGL_NO_CONTEXT;
        eglTerminate(g_display);
        g_display = EGL_NO_DISPLAY;
        g_current_window = NULL;
        g_thread_running = false;
        pthread_mutex_unlock(&g_lock);
        return -1;
    }

    pthread_mutex_unlock(&g_lock);
    LOGI("startOpenGLRenderer: success (window=%p)", win);
    return 0;
}

// ---------------------------------------------------------------------------
// stopOpenGLRenderer — full teardown.  Not in the required-six list but
// used internally by destroyOpenGLSubwindow.
// ---------------------------------------------------------------------------
int stopOpenGLRenderer() {
    LOGI("stopOpenGLRenderer");

    pthread_mutex_lock(&g_lock);
    if (!g_thread_running) {
        pthread_mutex_unlock(&g_lock);
        return 0;
    }
    g_shutdown = true;
    pthread_cond_signal(&g_cond);
    pthread_mutex_unlock(&g_lock);

    pthread_join(g_thread, NULL);

    pthread_mutex_lock(&g_lock);
    // The render thread already destroyed the surface.  Tear down the
    // context + display here.
    if (g_context != EGL_NO_CONTEXT) {
        eglDestroyContext(g_display, g_context);
        g_context = EGL_NO_CONTEXT;
    }
    if (g_display != EGL_NO_DISPLAY) {
        eglTerminate(g_display);
        g_display = EGL_NO_DISPLAY;
    }
    g_current_window = NULL;
    g_thread_running = false;
    g_shutdown       = false;
    pthread_mutex_unlock(&g_lock);
    return 0;
}

// ---------------------------------------------------------------------------
// setNativeWindow(window)
// Store the window pointer.  If the EGL surface exists, mark it for
// recreation by the render thread with the new window.
// ---------------------------------------------------------------------------
int setNativeWindow(void* window) {
    LOGI("setNativeWindow: %p", window);
    pthread_mutex_lock(&g_lock);
    if (g_thread_running) {
        if (window != NULL) {
            g_pending_window = (ANativeWindow*)window;
            g_pending_remove = false;
            pthread_cond_signal(&g_cond);
        }
    } else {
        LOGW("setNativeWindow: renderer not started yet");
    }
    pthread_mutex_unlock(&g_lock);
    return 0;
}

// ---------------------------------------------------------------------------
// resetSubWindow(window, x, y, w, h, fbw, fbh, dpr, zRot)
// Update the stored window and dimensions; recreate the EGL surface if
// the window pointer changed.
// ---------------------------------------------------------------------------
int resetSubWindow(void* window, int x, int y, int w, int h,
                   int fbw, int fbh, float dpr, float zRot) {
    LOGI("resetSubWindow: win=%p pos=(%d,%d) size=%dx%d fb=%dx%d "
         "dpr=%.2f zRot=%.2f",
         window, x, y, w, h, fbw, fbh, dpr, zRot);
    pthread_mutex_lock(&g_lock);
    g_width  = w;
    g_height = h;
    if (g_thread_running && window != NULL) {
        g_pending_window = (ANativeWindow*)window;
        g_pending_remove = false;
        pthread_cond_signal(&g_cond);
    }
    pthread_mutex_unlock(&g_lock);
    return 0;
}

// ---------------------------------------------------------------------------
// removeSubWindow(window)
// Destroy the EGL surface (the render thread does the actual destruction
// on its next iteration).  The EGL context and display are left intact
// so a later resetSubWindow can re-create a surface.
// ---------------------------------------------------------------------------
int removeSubWindow(void* /*window*/) {
    LOGI("removeSubWindow");
    pthread_mutex_lock(&g_lock);
    if (g_thread_running) {
        g_pending_remove = true;
        g_pending_window = NULL;
        pthread_cond_signal(&g_cond);
    }
    pthread_mutex_unlock(&g_lock);
    return 0;
}

// ---------------------------------------------------------------------------
// destroyOpenGLSubwindow()
// Destroy the EGL surface, context and display; terminate EGL.
// ---------------------------------------------------------------------------
int destroyOpenGLSubwindow() {
    LOGI("destroyOpenGLSubwindow");
    stopOpenGLRenderer();
    return 0;
}

// ---------------------------------------------------------------------------
// repaintOpenGLDisplay()
// Request an immediate eglSwapBuffers from the render thread.
// ---------------------------------------------------------------------------
void repaintOpenGLDisplay() {
    pthread_mutex_lock(&g_lock);
    if (g_thread_running) {
        g_repaint_now = true;
        pthread_cond_signal(&g_cond);
    }
    pthread_mutex_unlock(&g_lock);
}

// ---------------------------------------------------------------------------
// setOpenGLDisplayRotation — kept for API completeness; not in the
// required-six list.
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
