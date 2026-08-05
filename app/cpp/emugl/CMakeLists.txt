cmake_minimum_required(VERSION 3.22.1)
project(libOpenglRender)

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# Source files from AOSP emugl
set(EMUGL_SOURCES
    ColorBuffer.cpp
    EGLDispatch.cpp
    FBConfig.cpp
    FrameBuffer.cpp
    GL2Dispatch.cpp
    GLDispatch.cpp
    ReadBuffer.cpp
    RenderContext.cpp
    RenderControl.cpp
    RenderServer.cpp
    RenderThread.cpp
    ThreadInfo.cpp
    WindowSurface.cpp
    render_api.cpp
    twoyi_api.cpp
)

# Use NativeLinuxSubWindow for Android (no X11)
if(ANDROID)
    list(APPEND EMUGL_SOURCES NativeLinuxSubWindow.cpp)
else()
    list(APPEND EMUGL_SOURCES NativeWindowsSubWindow.cpp)
endif()

# Include directories
include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/compat
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/common
    ${CMAKE_CURRENT_SOURCE_DIR}/shared/OpenglOsUtils
    ${CMAKE_CURRENT_SOURCE_DIR}/shared/OpenglCodecCommon
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Build the shared library
add_library(OpenglRender SHARED ${EMUGL_SOURCES})

# Link against system libraries
target_link_libraries(OpenglRender
    EGL
    GLESv1_CM
    GLESv2
    log
    dl
    android
)

# Set output name to libOpenglRender.so
set_target_properties(OpenglRender PROPERTIES OUTPUT_NAME 'OpenglRender')
