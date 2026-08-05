cmake_minimum_required(VERSION 3.22.1)
project(libOpenglRender)

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# ---------------------------------------------------------------------------
# Source files.
#
# The full AOSP emugl renderer (FrameBuffer / RenderServer / GL decoders)
# depends on emugen-generated sources, the desktop-GL translator libraries
# and platform-private headers (cutils/*, utils/*) that are not shipped in
# the NDK.  Those sources are kept in the tree for reference but are not
# compiled — they do not build as-is (single-quoted includes, commented-out
# function signatures, undefined macros, missing emugen output).
#
# libOpenglRender.so therefore exports only the six C-ABI entry points that
# the Rust side (renderer_bindings.rs) links against.  Each entry point is
# implemented in twoyi_api.cpp as a logging stub that maintains window /
# surface state so a future iteration can wire them into a real EGL loop
# without changing the ABI.
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

# Link against system libraries.  `log` is needed for __android_log_print;
# `dl` is needed for the dlopen_ex / dlsym_ex wrappers; `android` is needed
# for the ANativeWindow types referenced indirectly.
target_link_libraries(OpenglRender
    log
    dl
)

# Set output name to libOpenglRender.so
set_target_properties(OpenglRender PROPERTIES OUTPUT_NAME "OpenglRender")

# Make all symbols visible so the Rust FFI can dlsym them.
set_target_properties(OpenglRender PROPERTIES CXX_VISIBILITY_PRESET default)
