// GLES2 dispatch stub — used when WITH_GLES2 is NOT defined.
//
// When WITH_GLES2 is undefined, GL2Dispatch.cpp compiles to an empty
// translation unit (its entire body is wrapped in #ifdef WITH_GLES2).
// But render_api.cpp still calls init_gl2_dispatch() and
// gl2_dispatch_get_proc_func(). This stub provides no-op definitions
// so the link succeeds.
//
// The renderer will operate in GLESv1-only mode. Guest GLESv2 commands
// are still decoded by gl2_dec.cpp (which is always compiled) — this
// only affects the host-side GLESv2 dispatch table used by the
// FrameBuffer's own EGL context for post rendering.

#include "GL2Dispatch.h"

#ifndef WITH_GLES2

// Provide empty definitions when WITH_GLES2 is not defined.
// These match the declarations in GL2Dispatch.h (which are also
// guarded by #ifdef WITH_GLES2, so we need to declare them here too).

// Declare the types so this compiles without WITH_GLES2.
#include "gl2_dec.h"

gl2_decoder_context_t s_gl2;
int s_gl2_enabled = 0;

bool init_gl2_dispatch() {
    // No-op — GLES2 dispatch not available.
    return false;
}

void *gl2_dispatch_get_proc_func(const char *name, void *userData) {
    (void)name;
    (void)userData;
    return NULL;
}

#endif  // !WITH_GLES2
