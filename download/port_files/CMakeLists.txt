cmake_minimum_required(VERSION 3.10)
project(libOpenglRender CXX)

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# Source files (relative to /tmp/build_opengl/src)
set(EMUGL_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src)

set(LIB_SRC
    ${EMUGL_SRC_DIR}/render_api.cpp
    ${EMUGL_SRC_DIR}/ColorBuffer.cpp
    ${EMUGL_SRC_DIR}/EGLDispatch.cpp
    ${EMUGL_SRC_DIR}/FBConfig.cpp
    ${EMUGL_SRC_DIR}/FrameBuffer.cpp
    ${EMUGL_SRC_DIR}/GLDispatch.cpp
    ${EMUGL_SRC_DIR}/GL2Dispatch.cpp
    ${EMUGL_SRC_DIR}/RenderContext.cpp
    ${EMUGL_SRC_DIR}/WindowSurface.cpp
    ${EMUGL_SRC_DIR}/RenderControl.cpp
    ${EMUGL_SRC_DIR}/ThreadInfo.cpp
    ${EMUGL_SRC_DIR}/RenderThread.cpp
    ${EMUGL_SRC_DIR}/ReadBuffer.cpp
    ${EMUGL_SRC_DIR}/RenderServer.cpp
    ${EMUGL_SRC_DIR}/NativeAndroidSubWindow.cpp
    ${EMUGL_SRC_DIR}/twoyi_api.cpp
    # Twoyi port (PORT-1): Graphics-Buffer server + Android-7+ dlopen shim.
    ${EMUGL_SRC_DIR}/GraphicBuffer.cpp
    ${EMUGL_SRC_DIR}/startGBServer.cpp
    ${EMUGL_SRC_DIR}/dl_ex.cpp
    # OpenglCodecCommon
    ${EMUGL_SRC_DIR}/GLClientState.cpp
    ${EMUGL_SRC_DIR}/GLSharedGroup.cpp
    ${EMUGL_SRC_DIR}/glUtils.cpp
    ${EMUGL_SRC_DIR}/SocketStream.cpp
    ${EMUGL_SRC_DIR}/TcpStream.cpp
    ${EMUGL_SRC_DIR}/TimeUtils.cpp
    ${EMUGL_SRC_DIR}/UnixStream.cpp
    # OpenglOsUtils
    ${EMUGL_SRC_DIR}/osDynLibrary.cpp
    ${EMUGL_SRC_DIR}/osProcessUnix.cpp
    ${EMUGL_SRC_DIR}/osThreadUnix.cpp
    # GLESv1_dec / GLESv2_dec
    ${EMUGL_SRC_DIR}/GLDecoder.cpp
    ${EMUGL_SRC_DIR}/GL2Decoder.cpp
    # generated
    ${EMUGL_SRC_DIR}/renderControl_dec.cpp
    ${EMUGL_SRC_DIR}/renderControl_server_context.cpp
    ${EMUGL_SRC_DIR}/gl_dec.cpp
    ${EMUGL_SRC_DIR}/gl_server_context.cpp
    ${EMUGL_SRC_DIR}/gl2_dec.cpp
    ${EMUGL_SRC_DIR}/gl2_server_context.cpp
    # compat shims
    ${EMUGL_SRC_DIR}/compat.cpp
)

set(LIB_INCLUDE_DIRS
    ${EMUGL_SRC_DIR}
    ${EMUGL_SRC_DIR}/libOpenglRender
    ${CMAKE_CURRENT_SOURCE_DIR}/compat
)

add_library(OpenglRender SHARED ${LIB_SRC})

target_include_directories(OpenglRender PRIVATE ${LIB_INCLUDE_DIRS})

target_compile_definitions(OpenglRender PRIVATE
    ANDROID
    HAVE_ANDROID_OS=1
    WITH_GLES2
    LOG_TAG=\"emugl\"
)

target_compile_options(OpenglRender PRIVATE
    -include assert.h
    -O2
    -Wno-unused-parameter
    -Wno-deprecated-declarations
    -Wno-multichar
    -Wno-format
    -fno-rtti
)

# Find the system EGL/GLES libraries
find_library(EGL_LIB EGL REQUIRED)
find_library(GLESv1_LIB GLESv1_CM REQUIRED)
find_library(GLESv2_LIB GLESv2 REQUIRED)
find_library(LOG_LIB log REQUIRED)

target_link_libraries(OpenglRender
    ${EGL_LIB}
    ${GLESv1_LIB}
    ${GLESv2_LIB}
    ${LOG_LIB}
    dl
    m
)

# Strip the final .so to reduce size (matching the legacy blob)
if(NOT CMAKE_BUILD_TYPE STREQUAL "Debug")
    add_custom_command(TARGET OpenglRender POST_BUILD
        COMMAND ${CMAKE_STRIP} -x $<TARGET_FILE:OpenglRender>
        COMMENT "Stripping libOpenglRender.so")
endif()
