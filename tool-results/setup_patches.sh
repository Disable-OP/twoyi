#!/bin/bash
# Copy sources, apply twoyi patches, write CMakeLists.txt
set -e

BUILD=/tmp/build_opengl
AOSP=/tmp/aosp-sdk/emulator/opengl
SRC=$BUILD/src
COMPAT=$BUILD/compat
GEN=$BUILD/generated

mkdir -p $SRC

# ---- 1. Copy original source files (so we don't modify the git checkout) ----
# libOpenglRender sources
cp $AOSP/host/libs/libOpenglRender/{render_api,ColorBuffer,EGLDispatch,FBConfig,FrameBuffer,GLDispatch,GL2Dispatch,RenderContext,WindowSurface,RenderControl,ThreadInfo,RenderThread,ReadBuffer,RenderServer}.cpp $SRC/
cp $AOSP/host/libs/libOpenglRender/{ColorBuffer,EGLDispatch,FBConfig,FrameBuffer,GLDispatch,GL2Dispatch,RenderContext,WindowSurface,RenderControl,ThreadInfo,RenderThread,ReadBuffer,RenderServer,NativeSubWindow,egl_proc,gl_proc}.h $SRC/

# OpenglCodecCommon sources
cp $AOSP/shared/OpenglCodecCommon/{GLClientState,GLSharedGroup,glUtils,SocketStream,TcpStream,TimeUtils,UnixStream}.cpp $SRC/
cp $AOSP/shared/OpenglCodecCommon/{ErrorLog,FixedBuffer,GLClientState,GLDecoderContextData,GLErrorLog,GLSharedGroup,SmartPtr,SocketStream,TcpStream,TimeUtils,UnixStream,codec_defs,glUtils,gl_base_types}.h $SRC/

# OpenglOsUtils sources
cp $AOSP/shared/OpenglOsUtils/{osDynLibrary,osProcessUnix,osThreadUnix}.cpp $SRC/
cp $AOSP/shared/OpenglOsUtils/{osDynLibrary,osProcess,osThread}.h $SRC/

# GLESv1_dec / GLESv2_dec
cp $AOSP/host/libs/GLESv1_dec/{GLDecoder.cpp,GLDecoder.h} $SRC/
cp $AOSP/host/libs/GLESv2_dec/{GL2Decoder.cpp,GL2Decoder.h} $SRC/

# Generated renderControl_dec / gl_dec / gl2_dec
cp $GEN/renderControl_dec/{renderControl_dec.cpp,renderControl_dec.h,renderControl_opcodes.h,renderControl_server_context.cpp,renderControl_server_context.h,renderControl_server_proc.h} $SRC/
cp $GEN/gl_dec/{gl_dec.cpp,gl_dec.h,gl_opcodes.h,gl_server_context.cpp,gl_server_context.h,gl_server_proc.h} $SRC/
cp $GEN/gl2_dec/{gl2_dec.cpp,gl2_dec.h,gl2_opcodes.h,gl2_server_context.cpp,gl2_server_context.h,gl2_server_proc.h} $SRC/

# renderControl_types.h / gl_types.h / gl2_types.h (from system/renderControl_enc, system/GLESv1_enc, system/GLESv2_enc)
cp $AOSP/system/renderControl_enc/renderControl_types.h $SRC/
cp $AOSP/system/GLESv1_enc/gl_types.h $SRC/
cp $AOSP/system/GLESv2_enc/gl2_types.h $SRC/

# Public API header (libOpenglRender/render_api.h, IOStream.h, render_api_platform_types.h)
mkdir -p $SRC/libOpenglRender
cp $AOSP/host/include/libOpenglRender/{render_api.h,render_api_platform_types.h,IOStream.h} $SRC/libOpenglRender/

echo "=== Copied $(ls $SRC/*.cpp | wc -l) cpp files and $(ls $SRC/*.h $SRC/libOpenglRender/*.h | wc -l) header files ==="

# ---- 2. Patch render_api_platform_types.h: add __ANDROID__ branch ----
cat > $SRC/libOpenglRender/render_api_platform_types.h <<'EOF'
/* Patched for twoyi: __ANDROID__ uses void* for FBNativeWindowType */
#ifndef _RENDER_API_PLATFORM_TYPES_H
#define _RENDER_API_PLATFORM_TYPES_H

#if defined(_WIN32)
#include <windows.h>
typedef HDC     FBNativeDisplayType;
typedef HWND    FBNativeWindowType;
#elif defined(__ANDROID__) || defined(__APPLE__)
/* On Android the ANativeWindow is passed in as an opaque void*.
 * On Apple platforms we also use void*. */
typedef void*   FBNativeDisplayType;
typedef void*   FBNativeWindowType;
#elif defined(__linux__)
#include <X11/Xlib.h>
#include <X11/Xutil.h>
typedef Window  FBNativeWindowType;
#else
#warning "Unsupported platform"
typedef void*   FBNativeWindowType;
#endif

#endif
EOF

# ---- 3. Patch EGLDispatch.cpp / GLDispatch.cpp / GL2Dispatch.cpp: use Android system libs ----
sed -i 's|#define DEFAULT_EGL_LIB EMUGL_LIBNAME("EGL_translator")|#define DEFAULT_EGL_LIB "libEGL.so"|' $SRC/EGLDispatch.cpp
sed -i 's|#define DEFAULT_GLES_CM_LIB EMUGL_LIBNAME("GLES_CM_translator")|#define DEFAULT_GLES_CM_LIB "libGLESv1_CM.so"|' $SRC/GLDispatch.cpp
sed -i 's|#define DEFAULT_GLES_V2_LIB EMUGL_LIBNAME("GLES_V2_translator")|#define DEFAULT_GLES_V2_LIB "libGLESv2.so"|' $SRC/GL2Dispatch.cpp

# ---- 4. Patch UnixStream.cpp: use twoyi pipe path ----
# Replace the make_unix_path function with a twoyi-specific one.
python3 - <<'PYEOF'
import re
src = open('/tmp/build_opengl/src/UnixStream.cpp').read()

# Replace the make_unix_path function entirely
new_make = '''/* Patched for twoyi: use the twoyi opengles pipe path under the
 * container rootfs. The path is chosen by the "port" number:
 *   port % 3 == 0 -> /data/data/io.twoyi/rootfs/opengles
 *   port % 3 == 1 -> /data/data/io.twoyi/rootfs/opengles2
 *   port % 3 == 2 -> /data/data/io.twoyi/rootfs/opengles3
 * The base path can be overridden at runtime via the TWOYI_ROOTFS env var.
 */
static int
make_unix_path(char *path, size_t pathlen, int port_number)
{
    const char *rootfs = getenv("TWOYI_ROOTFS");
    if (rootfs == NULL || rootfs[0] == '\\0') {
        rootfs = "/data/data/io.twoyi/rootfs";
    }
    const char *suffix;
    int idx = port_number % 3;
    if (idx == 0)      suffix = "opengles";
    else if (idx == 1) suffix = "opengles2";
    else               suffix = "opengles3";
    snprintf(path, pathlen, "%s/%s", rootfs, suffix);
    return 0;
}
'''

# Replace from "static int\nmake_unix_path" up to the closing "return 0;\n}"
pat = re.compile(r'/\* Not all systems define PATH_MAX.*?\n\}\n', re.DOTALL)
src = pat.sub(new_make, src, count=1)
open('/tmp/build_opengl/src/UnixStream.cpp','w').write(src)
print("UnixStream.cpp patched")
PYEOF

# Also remove the cutils/sockets.h include since we use raw socket APIs in compat
# Actually keep it - our compat/cutils/sockets.h provides socket_local_server/client.
# But remove the unused #include <cutils/sockets.h> conflict... no, keep it.

# ---- 5. Write NativeAndroidSubWindow.cpp (replacing NativeLinuxSubWindow.cpp) ----
cat > $SRC/NativeAndroidSubWindow.cpp <<'EOF'
/* Patched for twoyi: Android native subwindow handling.
 * On Android there is no X11; the ANativeWindow passed in IS the EGLNativeWindow.
 * createSubWindow() just returns the same window back. */
#include "NativeSubWindow.h"
#include <EGL/egl.h>

EGLNativeWindowType createSubWindow(FBNativeWindowType p_window,
                                    EGLNativeDisplayType* display_out,
                                    int x, int y, int width, int height)
{
    // On Android, the host ANativeWindow is used directly as the EGL window.
    // The display is the default EGLDisplay.
    if (display_out) *display_out = EGL_DEFAULT_DISPLAY;
    return (EGLNativeWindowType)p_window;
}

void destroySubWindow(EGLNativeDisplayType dis, EGLNativeWindowType win)
{
    // Nothing to do - the ANativeWindow is owned by the caller (twoyi).
    (void)dis;
    (void)win;
}
EOF
# Remove the X11-dependent NativeLinuxSubWindow.cpp from the build
rm -f $SRC/NativeLinuxSubWindow.cpp

# ---- 6. Write twoyi_api.cpp: the twoyi-specific C-ABI functions ----
cat > $SRC/twoyi_api.cpp <<'EOF'
/* twoyi-specific C-ABI functions added to libOpenglRender.
 * These wrap the existing AOSP FrameBuffer / RenderServer API. */
#include "libOpenglRender/render_api.h"
#include "IOStream.h"
#include "FrameBuffer.h"
#include "RenderServer.h"
#include "EGLDispatch.h"
#include "GLDispatch.h"
#include "GL2Dispatch.h"
#include "TimeUtils.h"
#include "TcpStream.h"
#include "UnixStream.h"

#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

// Forward declarations from render_api.cpp
extern RenderServer *s_renderThread;
extern int gRendererStreamMode;
extern "C" int setStreamMode(int mode);

// Twoyi pipe port (used by UnixStream::listen to build the path).
// 0 -> opengles, 1 -> opengles2, 2 -> opengles3
static int s_twoyi_pipe_port = 0;

// Stored host ANativeWindow (set via setNativeWindow).
static FBNativeWindowType s_nativeWindow = NULL;

extern "C" {

// Store the host ANativeWindow for later use by resetSubWindow.
int setNativeWindow(void* window)
{
    s_nativeWindow = (FBNativeWindowType)window;
    return 1;
}

// startOpenGLRenderer: initialize the renderer and start listening on the
// twoyi Unix pipe path. Signature matches twoyi's renderer_bindings.rs.
int startOpenGLRenderer(void* win, int width, int height,
                        int xdpi, int ydpi, int fps)
{
    if (s_renderThread != NULL) return 0;

    // Store the native window
    s_nativeWindow = (FBNativeWindowType)win;

    // Initialize EGL/GLES dispatch tables (dlopens libEGL.so, libGLESv1_CM.so, libGLESv2.so)
    if (!init_egl_dispatch()) {
        fprintf(stderr, "startOpenGLRenderer: init_egl_dispatch failed\n");
        return 0;
    }
    if (!init_gl_dispatch()) {
        fprintf(stderr, "startOpenGLRenderer: init_gl_dispatch failed\n");
        return 0;
    }
    init_gl2_dispatch();  // failure is non-fatal

    // Initialize the FrameBuffer singleton.
    bool inited = FrameBuffer::initialize(width, height, NULL, NULL);
    if (!inited) {
        fprintf(stderr, "startOpenGLRenderer: FrameBuffer::initialize failed\n");
        return 0;
    }

    // Use Unix socket mode so RenderServer listens on the twoyi pipe path.
    gRendererStreamMode = STREAM_MODE_UNIX;
    s_twoyi_pipe_port = 0;

    // Start the RenderServer thread (listens on /data/data/io.twoyi/rootfs/opengles).
    s_renderThread = RenderServer::create(s_twoyi_pipe_port);
    if (!s_renderThread) {
        fprintf(stderr, "startOpenGLRenderer: RenderServer::create failed\n");
        return 0;
    }
    s_renderThread->start();

    return 1;
}

// resetSubWindow: create/recreate the EGLSurface bound to the given ANativeWindow.
int resetSubWindow(void* p_window, int wx, int wy, int ww, int wh,
                   int fbw, int fbh, float dpr, float zRot)
{
    (void)dpr; (void)fbw; (void)fbh;  // FB dimensions already set at startOpenGLRenderer
    FBNativeWindowType win = (FBNativeWindowType)p_window;
    if (win == NULL) win = s_nativeWindow;
    if (win == NULL) return 0;
    return FrameBuffer::setupSubWindow(win, wx, wy, ww, wh, zRot) ? 1 : 0;
}

// removeSubWindow: destroy the current EGLSurface (separate from destroyOpenGLSubwindow).
int removeSubWindow(void* window)
{
    (void)window;
    return FrameBuffer::removeSubWindow() ? 1 : 0;
}

// dlopen_ex / dlsym_ex / dlclose_ex / dlerror_ex: thin wrappers around libdl
// (matches the legacy blob's exported symbols).
void* dlopen_ex(const char* filename, int flag)
{
    return dlopen(filename, flag);
}
void* dlsym_ex(void* handle, const char* symbol)
{
    return dlsym(handle, symbol);
}
int dlclose_ex(void* handle)
{
    return dlclose(handle);
}
const char* dlerror_ex(void)
{
    return dlerror();
}

} // extern "C"
EOF

# ---- 7. Update compat/cutils/sockets.h with socket_loopback_server, socket_network_client ----
cat > $COMPAT/cutils/sockets.h <<'EOF'
#ifndef _CUTILS_SOCKETS_H
#define _CUTILS_SOCKETS_H
#ifdef __cplusplus
extern "C" {
#endif

#define ANDROID_SOCKET_NAMESPACE_FILESYSTEM 0
#define ANDROID_SOCKET_NAMESPACE_ABSTRACT   1

int socket_local_server(const char* name, int namespaceId, int type);
int socket_local_client(const char* name, int namespaceId, int type);
int socket_loopback_server(int port, int type);
int socket_inaddr_any_server(int port, int type);
int socket_network_client(const char* host, int port, int type);

#ifdef __cplusplus
}
#endif
#endif
EOF

# Add implementations of socket_loopback_server, socket_inaddr_any_server, socket_network_client to compat.cpp
cat >> $SRC/compat.cpp <<'EOF'

// ----- socket_loopback_server / socket_inaddr_any_server / socket_network_client -----
#include <netinet/in.h>
#include <netdb.h>

extern "C" int socket_loopback_server(int port, int type) {
    int sock = socket(AF_INET, type, 0);
    if (sock < 0) return -1;
    int opt = 1;
    setsockopt(sock, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    if (bind(sock, (struct sockaddr*)&addr, sizeof(addr)) < 0) { close(sock); return -1; }
    if (type == SOCK_STREAM) { if (listen(sock, 5) < 0) { close(sock); return -1; } }
    return sock;
}

extern "C" int socket_inaddr_any_server(int port, int type) {
    int sock = socket(AF_INET, type, 0);
    if (sock < 0) return -1;
    int opt = 1;
    setsockopt(sock, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    addr.sin_addr.s_addr = htonl(INADDR_ANY);
    if (bind(sock, (struct sockaddr*)&addr, sizeof(addr)) < 0) { close(sock); return -1; }
    if (type == SOCK_STREAM) { if (listen(sock, 5) < 0) { close(sock); return -1; } }
    return sock;
}

extern "C" int socket_network_client(const char* host, int port, int type) {
    struct addrinfo hints, *res, *rp;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = type;
    char portstr[16];
    snprintf(portstr, sizeof(portstr), "%d", port);
    if (getaddrinfo(host, portstr, &hints, &res) != 0) return -1;
    int sock = -1;
    for (rp = res; rp != NULL; rp = rp->ai_next) {
        sock = socket(rp->ai_family, rp->ai_socktype, rp->ai_protocol);
        if (sock < 0) continue;
        if (connect(sock, rp->ai_addr, rp->ai_addrlen) == 0) break;
        close(sock);
        sock = -1;
    }
    freeaddrinfo(res);
    return sock;
}
EOF

# ---- 8. Write CMakeLists.txt ----
cat > $BUILD/CMakeLists.txt <<'EOF'
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
    -O2
    -fvisibility=hidden
    -fvisibility-inlines-hidden
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
EOF

echo "=== Patches applied and CMakeLists.txt written ==="
ls -la $SRC/twoyi_api.cpp $SRC/NativeAndroidSubWindow.cpp $BUILD/CMakeLists.txt
echo "=== Done ==="
