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
# NOTE: WITH_GLES2 is intentionally NOT defined. When enabled, it
# causes FrameBuffer::initialize to call getGLES2ExtensionString()
# which dlopens libGLESv2.so and calls s_gl2.glGetString — but on the
# Android emulator, the dlopened libGLESv2.so doesn't match the
# emulator's EGL implementation, causing crashes or bad function
# pointers. The renderer works fine with GLESv1 only (the guest's
# GLESv2 commands are still decoded by gl2_dec.cpp — the WITH_GLES2
# flag only controls the host-side GLESv2 dispatch table for the
# FrameBuffer's own EGL context, not the guest command decoding).
add_compile_definitions(RENDER_API_USE_THREAD)

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
    libOpenglRender/GL2DispatchStub.cpp
    # native subwindow (Android — ANativeWindow passthrough)
    NativeLinuxSubWindow.cpp
    # codec common (socket streams, GL state, shared groups)
    # NOTE: UnixStream.cpp is the PATCHED version from OpenglCodecCommon/
    # (not shared/OpenglCodecCommon/) — it builds the pipe path as
    # $TWOYI_ROOTFS/opengles{,2,3} instead of /tmp/android-$USER/qemu-gles-$port.
    # The shared/ version is the original AOSP code kept for reference.
    shared/OpenglCodecCommon/SocketStream.cpp
    OpenglCodecCommon/UnixStream.cpp
    shared/OpenglCodecCommon/TcpStream.cpp
    shared/OpenglCodecCommon/glUtils.cpp
    shared/OpenglCodecCommon/GLClientState.cpp
    shared/OpenglCodecCommon/GLSharedGroup.cpp
    shared/OpenglCodecCommon/TimeUtils.cpp
    # OS utils (threads, dynamic libraries, process handling)
    shared/OpenglOsUtils/osThreadUnix.cpp
    shared/OpenglOsUtils/osProcessUnix.cpp
    shared/OpenglOsUtils/osDynLibrary.cpp
    # GLDecoder / GL2Decoder — the AOSP decoder wrapper classes that
    # inherit from gl_decoder_context_t / gl2_decoder_context_t and
    # provide initGL(), s_getProc(), and the static GL callback stubs.
    # Without these, the linker fails with "undefined symbol:
    # GLDecoder::GLDecoder()" etc.
    GLESv1_dec/GLDecoder.cpp
    GLESv2_dec/GL2Decoder.cpp
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

# Export ALL symbols by default. The Rust crate libtwoyi.so links against
# libOpenglRender.so via #[link(name = "OpenglRender")] and dlopen-resolves
# the FFI entry points (startOpenGLRenderer, setNativeWindow, resetSubWindow,
# removeSubWindow, destroyOpenGLSubwindow, repaintOpenGLDisplay) at load
# time. If these symbols are hidden, System.loadLibrary("twoyi") fails with
# "dlopen failed: cannot locate symbol removeSubWindow".
#
# The old CXX_VISIBILITY_PRESET=hidden setting was intended to reduce the
# ABI surface, but it broke the Rust→C++ FFI linkage. Reverting to default
# visibility (all extern "C" symbols exported) fixes the link and costs
# only ~20 KB of extra symbol table.
set_target_properties(OpenglRender PROPERTIES
    CXX_VISIBILITY_PRESET default
    VISIBILITY_INLINES_HIDDEN OFF
)
