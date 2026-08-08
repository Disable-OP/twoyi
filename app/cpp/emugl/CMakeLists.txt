cmake_minimum_required(VERSION 3.22.1)
project(libOpenglRender)

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# ---------------------------------------------------------------------------
# Phase 0: Full AOSP emugl pipeline.
#
# Previously only twoyi_api.cpp (a stub EGL clear-loop) was compiled.
# Phase 0 expands the build to the full AOSP emugl renderer:
#   - RenderServer: accepts guest connections on the Unix socket
#   - RenderThread: per-connection GL command decoder
#   - FrameBuffer: manages color buffers, EGL contexts, the subwindow
#   - GLESv1/v2/renderControl decoders: emugen-generated command parsers
#   - EGLDispatch/GLDispatch/GL2Dispatch: dlopen the system EGL/GL libs
#
# The entry points (initOpenGLRenderer, createOpenGLSubwindow, etc.)
# come from libOpenglRender/render_api.cpp (the pristine AOSP version
# with RENDER_API_USE_THREAD defined for in-process operation).
#
# The old twoyi_api.cpp stub is renamed to twoyi_api.cpp.disabled and
# is no longer compiled. Its EGL clear-loop is replaced by the real
# AOSP pipeline that actually executes guest GL commands.
# ---------------------------------------------------------------------------

# Build with RENDER_API_USE_THREAD so initOpenGLRenderer runs the
# RenderServer in-process (no separate emulator_renderer process).
# Build with WITH_GLES2 so the GLESv2 dispatch tables (init_gl2_dispatch,
# gl2_dispatch_get_proc_func, s_gl2) are visible — they're guarded by
# #ifdef WITH_GLES2 in GL2Dispatch.h and GL2Dispatch.cpp.
add_compile_definitions(RENDER_API_USE_THREAD WITH_GLES2)

# Source files — use the libOpenglRender/ subdir (which has bug fixes
# over the top-level copies: RenderServer NULL-deref fix, ColorBuffer
# memory leak fix, EGLDispatch Android-specific dlopen path).
set(EMUGL_SOURCES
    # twoyi FFI glue — adapts the six Rust-facing symbols
    # (startOpenGLRenderer, setNativeWindow, resetSubWindow, etc.)
    # to the AOSP emugl API (initOpenGLRenderer, createOpenGLSubwindow, etc.)
    twoyi_glue.cpp
    # AOSP entry points (initOpenGLRenderer, createOpenGLSubwindow, ...)
    libOpenglRender/render_api.cpp
    # renderer core
    libOpenglRender/RenderServer.cpp
    libOpenglRender/RenderThread.cpp
    libOpenglRender/FrameBuffer.cpp
    libOpenglRender/RenderControl.cpp
    libOpenglRender/ColorBuffer.cpp
    libOpenglRender/WindowSurface.cpp
    libOpenglRender/RenderContext.cpp
    libOpenglRender/FBConfig.cpp
    libOpenglRender/ReadBuffer.cpp
    libOpenglRender/ThreadInfo.cpp
    libOpenglRender/EGLDispatch.cpp
    libOpenglRender/GLDispatch.cpp
    libOpenglRender/GL2Dispatch.cpp
    # native subwindow (Android — ANativeWindow passthrough)
    NativeLinuxSubWindow.cpp
    # codec common (socket streams, GL state, shared groups)
    shared/OpenglCodecCommon/SocketStream.cpp
    shared/OpenglCodecCommon/UnixStream.cpp
    shared/OpenglCodecCommon/TcpStream.cpp
    shared/OpenglCodecCommon/glUtils.cpp
    shared/OpenglCodecCommon/GLClientState.cpp
    shared/OpenglCodecCommon/GLSharedGroup.cpp
    shared/OpenglCodecCommon/TimeUtils.cpp
    # OS utils (threads, dynamic libraries, process handling)
    shared/OpenglOsUtils/osThreadUnix.cpp
    shared/OpenglOsUtils/osProcessUnix.cpp
    shared/OpenglOsUtils/osDynLibrary.cpp
    # emugen-generated decoders (GLESv1, GLESv2, renderControl)
    generated/gl_dec/gl_dec.cpp
    generated/gl_dec/gl_server_context.cpp
    generated/gl2_dec/gl2_dec.cpp
    generated/gl2_dec/gl2_server_context.cpp
    generated/renderControl_dec/renderControl_dec.cpp
    generated/renderControl_dec/renderControl_server_context.cpp
    # compat shims (cutils/log.h, utils/threads.h, etc.)
    compat/compat.cpp
)

# Include directories:
# - compat/        : cutils/* and utils/* shims for AOSP-private headers
# - . (emugl root) : NativeSubWindow.h, NativeLinuxSubWindow.cpp
# - include/       : libOpenglRender/render_api.h, IOStream.h
# - libOpenglRender/ : the renderer core headers (FrameBuffer.h, etc.)
# - shared/OpenglOsUtils : osThread.h, osDynLibrary.h, etc.
# - shared/OpenglCodecCommon : SocketStream.h, UnixStream.h, etc.
# - generated/gl_dec, gl2_dec, renderControl_dec : decoder headers
include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/compat
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/include/libOpenglRender
    ${CMAKE_CURRENT_SOURCE_DIR}/libOpenglRender
    ${CMAKE_CURRENT_SOURCE_DIR}/shared/OpenglOsUtils
    ${CMAKE_CURRENT_SOURCE_DIR}/shared/OpenglCodecCommon
    ${CMAKE_CURRENT_SOURCE_DIR}/GLESv1_dec
    ${CMAKE_CURRENT_SOURCE_DIR}/GLESv2_dec
    ${CMAKE_CURRENT_SOURCE_DIR}/generated/gl_dec
    ${CMAKE_CURRENT_SOURCE_DIR}/generated/gl2_dec
    ${CMAKE_CURRENT_SOURCE_DIR}/generated/renderControl_dec
)

# Build the shared library
add_library(OpenglRender SHARED ${EMUGL_SOURCES})

# Link against system libraries:
#   EGL    — eglGetDisplay, eglInitialize, eglCreateContext, ...
#   GLESv2 — glClearColor, glClear, glTexImage2D, ...
#   android — ANativeWindow type (referenced by EGL's eglplatform.h)
#   log    — __android_log_print
#   dl     — dlopen / dlsym (EGLDispatch/GLDispatch dlopen libEGL.so,
#            libGLESv2.so, libGLESv1_CM.so at runtime — no link-time
#            dependency on GLESv1 because the NDK doesn't ship a
#            separate libGLESv1.so; the GLESv1 dispatch table is
#            populated by dlopening libGLESv1_CM.so.)
target_link_libraries(OpenglRender
    EGL
    GLESv2
    android
    log
    dl
)

# Set output name to libOpenglRender.so
set_target_properties(OpenglRender PROPERTIES OUTPUT_NAME "OpenglRender")

# Hide all symbols by default — only the required FFI entry points
# (marked with TWOYI_EXPORT or extern "C" __attribute__((visibility("default"))))
# will be exported. This reduces the ABI surface and enables better
# dead-code elimination via --gc-sections.
set_target_properties(OpenglRender PROPERTIES
    CXX_VISIBILITY_PRESET hidden
    VISIBILITY_INLINES_HIDDEN ON
)
