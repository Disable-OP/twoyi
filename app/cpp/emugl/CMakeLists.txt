cmake_minimum_required(VERSION 3.22.1)
project(libOpenglRender)

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# ---------------------------------------------------------------------------
# Source files.
#
# twoyi_api.cpp implements the six C-ABI entry points the Rust side
# (renderer_bindings.rs) links against, with a REAL EGL pipeline:
#   - eglGetDisplay / eglInitialize / eglChooseConfig / eglCreateContext
#   - eglCreateWindowSurface with the ANativeWindow passed in
#   - A render thread that does glClearColor / glClear / eglSwapBuffers
#     at the requested frame rate.
# The full AOSP emugl renderer (FrameBuffer / RenderServer / GL decoders)
# is kept in the tree for reference but is not compiled — it depends on
# emugen-generated sources, the desktop-GL translator libraries and
# platform-private headers (cutils/*, utils/*) that are not shipped in
# the NDK.
# ---------------------------------------------------------------------------
set(EMUGL_SOURCES
    twoyi_api.cpp
)

# Include directories — kept for future use when the full AOSP stack is
# brought online.
include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/compat
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/shared/OpenglOsUtils
    ${CMAKE_CURRENT_SOURCE_DIR}/shared/OpenglCodecCommon
)

# Build the shared library
add_library(OpenglRender SHARED ${EMUGL_SOURCES})

# Link against system libraries:
#   EGL    — eglGetDisplay, eglInitialize, eglCreateContext, ...
#   GLESv2 — glClearColor, glClear (GLES2 rendering context)
#   android — ANativeWindow type (referenced by EGL's eglplatform.h)
#   log    — __android_log_print
#   dl     — dlopen_ex / dlsym_ex / dlclose_ex / dlerror_ex wrappers
target_link_libraries(OpenglRender
    EGL
    GLESv2
    android
    log
    dl
)

# Set output name to libOpenglRender.so
set_target_properties(OpenglRender PROPERTIES OUTPUT_NAME "OpenglRender")

# Make all symbols visible so the Rust FFI can dlsym them.
set_target_properties(OpenglRender PROPERTIES CXX_VISIBILITY_PRESET default)
