/*
 * GraphicBuffer.h — Twoyi graphics-buffer proxy server.
 *
 * The legacy libOpenglRender.so exposes a `GraphicBuffer` class that
 * listens on $TWOYI_ROOTFS/opengles3 (default
 * /data/data/io.twoyi/rootfs/opengles3) and accepts AHardwareBuffer
 * file descriptors from the guest.  Each received AHardwareBuffer is
 * converted to an ANativeWindowBuffer via
 * android::AHardwareBuffer_to_ANativeWindowBuffer and stored for
 * SurfaceFlinger compositing.
 *
 * This class is spawned by startGBServer() (see startGBServer.cpp).
 *
 * Legacy: GraphicBuffer has 15 methods totalling 1,152 bytes; this open
 * source re-implementation keeps the public create()/dtor/Main API
 * compatible but trims unused internals.
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#ifndef _TWOYI_GRAPHIC_BUFFER_H
#define _TWOYI_GRAPHIC_BUFFER_H

#include "osThread.h"

// Forward declarations — AHardwareBuffer is an opaque type defined in
// <android/hardware_buffer.h>; ANativeWindowBuffer in <system/window.h>.
// We deliberately avoid pulling those headers in here so the file builds
// on the NDK r25 toolchain used by /tmp/build_opengl without extra
// include paths.
struct AHardwareBuffer;
struct ANativeWindowBuffer;

// Function-pointer typedefs for the libandroid.so symbols looked up at
// runtime by startGBServer().  Stored in GraphicBuffer::m_recvHandle and
// GraphicBuffer::m_toNativeWindowBuffer.
typedef int  (*AHardwareBuffer_recvHandleFromUnixSocketFn)(
                 int socket_fd, AHardwareBuffer** out_buffer);
typedef ANativeWindowBuffer* (*AHardwareBuffer_to_ANativeWindowBufferFn)(
                 AHardwareBuffer* buffer);

class GraphicBuffer : public osUtils::Thread
{
public:
    // Factory: allocates a GraphicBuffer, opens the listening socket at
    // $TWOYI_ROOTFS/opengles3, and returns the instance.  Returns NULL
    // on failure (socket could not be created).
    static GraphicBuffer* create();

    virtual ~GraphicBuffer();

    // osUtils::Thread override — runs the accept loop.
    virtual int Main();

    // Set the libandroid.so function pointers.  Called by startGBServer
    // before Thread::start().
    void setRecvHandle(AHardwareBuffer_recvHandleFromUnixSocketFn p) {
        m_recvHandle = p;
    }
    void setToNativeWindowBuffer(AHardwareBuffer_to_ANativeWindowBufferFn p) {
        m_toNativeWindowBuffer = p;
    }

private:
    GraphicBuffer();

    int m_listenSock;   // fd returned by socket_local_server()

    // Function pointers looked up by startGBServer from libandroid.so.
    // NULL until setRecvHandle()/setToNativeWindowBuffer() are called.
    AHardwareBuffer_recvHandleFromUnixSocketFn   m_recvHandle;
    AHardwareBuffer_to_ANativeWindowBufferFn     m_toNativeWindowBuffer;
};

#endif // _TWOYI_GRAPHIC_BUFFER_H
