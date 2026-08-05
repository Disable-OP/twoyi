cmake_minimum_required(VERSION 3.10)
project(libOpenglRender CXX)

# =============================================================================
# libOpenglRender.so — 100% open-source build from the vendored AOSP emugl
# source (task AOSP-VENDOR-1).
#
# This CMakeLists.txt builds the twoyi-flavored libOpenglRender.so for one
# Android ABI at a time. The build is driven by ../build.sh, which invokes
# cmake once per ABI (arm64-v8a / x86_64) with the NDK toolchain file.
#
# Source layout (all relative to this file):
#   libOpenglRender/    — main renderer .cpp/.h (AOSP, lightly patched)
#   OpenglCodecCommon/  — shared codec / stream .cpp/.h (AOSP)
#   OpenglOsUtils/      — shared OS utils .cpp/.h (AOSP)
#   GLESv1_dec/         — GLDecoder.{cpp,h} (AOSP)
#   GLESv2_dec/         — GL2Decoder.{cpp,h} (AOSP)
#   include/            — public libOpenglRender headers (patched)
#   generated/          — emugen output (committed; no host tool needed)
#   compat/             — cutils/* + utils/* compat shims
#   twoyi/              — twoyi_api.cpp + NativeAndroidSubWindow.cpp
#
# Patches applied to AOSP source (vs upstream commit 7a712ac):
#   - render_api_platform_types.h : __ANDROID__ branch (void*)
#   - EGLDispatch.cpp             : dlopen libEGL.so (system)
#   - GLDispatch.cpp              : dlopen libGLESv1_CM.so (system)
#   - GL2Dispatch.cpp             : dlopen libGLESv2.so (system)
#   - UnixStream.cpp              : pipe path = $TWOYI_ROOTFS/opengles{,2,3}
#   - render_api.cpp              : s_renderThread made non-static
#   - NativeLinuxSubWindow.cpp    : replaced by twoyi/NativeAndroidSubWindow.cpp
# =============================================================================

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

set(EMUGL ${CMAKE_CURRENT_SOURCE_DIR})

set(LIB_SRC
    # libOpenglRender (main renderer)
    ${EMUGL}/libOpenglRender/render_api.cpp
    ${EMUGL}/libOpenglRender/ColorBuffer.cpp
    ${EMUGL}/libOpenglRender/EGLDispatch.cpp
    ${EMUGL}/libOpenglRender/FBConfig.cpp
    ${EMUGL}/libOpenglRender/FrameBuffer.cpp
    ${EMUGL}/libOpenglRender/GLDispatch.cpp
    ${EMUGL}/libOpenglRender/GL2Dispatch.cpp
    ${EMUGL}/libOpenglRender/RenderContext.cpp
    ${EMUGL}/libOpenglRender/WindowSurface.cpp
    ${EMUGL}/libOpenglRender/RenderControl.cpp
    ${EMUGL}/libOpenglRender/ThreadInfo.cpp
    ${EMUGL}/libOpenglRender/RenderThread.cpp
    ${EMUGL}/libOpenglRender/ReadBuffer.cpp
    ${EMUGL}/libOpenglRender/RenderServer.cpp
    # twoyi replacement for NativeLinuxSubWindow.cpp
    ${EMUGL}/twoyi/NativeAndroidSubWindow.cpp
    ${EMUGL}/twoyi/twoyi_api.cpp

    # OpenglCodecCommon
    ${EMUGL}/OpenglCodecCommon/GLClientState.cpp
    ${EMUGL}/OpenglCodecCommon/GLSharedGroup.cpp
    ${EMUGL}/OpenglCodecCommon/glUtils.cpp
    ${EMUGL}/OpenglCodecCommon/SocketStream.cpp
    ${EMUGL}/OpenglCodecCommon/TcpStream.cpp
    ${EMUGL}/OpenglCodecCommon/TimeUtils.cpp
    ${EMUGL}/OpenglCodecCommon/UnixStream.cpp

    # OpenglOsUtils
    ${EMUGL}/OpenglOsUtils/osDynLibrary.cpp
    ${EMUGL}/OpenglOsUtils/osProcessUnix.cpp
    ${EMUGL}/OpenglOsUtils/osThreadUnix.cpp

    # GLESv1_dec / GLESv2_dec
    ${EMUGL}/GLESv1_dec/GLDecoder.cpp
    ${EMUGL}/GLESv2_dec/GL2Decoder.cpp

    # emugen-generated decoder sources (committed)
    ${EMUGL}/generated/renderControl_dec/renderControl_dec.cpp
    ${EMUGL}/generated/renderControl_dec/renderControl_server_context.cpp
    ${EMUGL}/generated/gl_dec/gl_dec.cpp
    ${EMUGL}/generated/gl_dec/gl_server_context.cpp
    ${EMUGL}/generated/gl2_dec/gl2_dec.cpp
    ${EMUGL}/generated/gl2_dec/gl2_server_context.cpp

    # compat shim layer
    ${EMUGL}/compat/compat.cpp
)

set(LIB_INCLUDE_DIRS
    ${EMUGL}/libOpenglRender
    ${EMUGL}/OpenglCodecCommon
    ${EMUGL}/OpenglOsUtils
    ${EMUGL}/GLESv1_dec
    ${EMUGL}/GLESv2_dec
    ${EMUGL}/include
    ${EMUGL}/include/libOpenglRender
    ${EMUGL}/generated/renderControl_dec
    ${EMUGL}/generated/gl_dec
    ${EMUGL}/generated/gl2_dec
    ${EMUGL}/compat
)

add_library(OpenglRender SHARED ${LIB_SRC})
target_include_directories(OpenglRender PRIVATE ${LIB_INCLUDE_DIRS})

target_compile_definitions(OpenglRender PRIVATE
    ANDROID HAVE_ANDROID_OS=1 WITH_GLES2 LOG_TAG="emugl")

target_compile_options(OpenglRender PRIVATE
    -include assert.h -O2
    -Wno-unused-parameter -Wno-deprecated-declarations
    -Wno-multichar -Wno-format -fno-rtti)

find_library(EGL_LIB    EGL       REQUIRED)
find_library(GLESv1_LIB GLESv1_CM REQUIRED)
find_library(GLESv2_LIB GLESv2    REQUIRED)
find_library(LOG_LIB    log       REQUIRED)

target_link_libraries(OpenglRender
    ${EGL_LIB} ${GLESv1_LIB} ${GLESv2_LIB} ${LOG_LIB} dl m)

# Strip the .so in Release builds (matches what NDK does for AS projects).
if(NOT CMAKE_BUILD_TYPE STREQUAL "Debug")
    add_custom_command(TARGET OpenglRender POST_BUILD
        COMMAND ${CMAKE_STRIP} -x $<TARGET_FILE:OpenglRender>
        COMMENT "Stripping libOpenglRender.so")
endif()
