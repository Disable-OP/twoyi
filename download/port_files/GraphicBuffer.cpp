/*
 * GraphicBuffer.cpp — Twoyi graphics-buffer proxy server implementation.
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#include "GraphicBuffer.h"

#include <cutils/sockets.h>
#include <cutils/log.h>

#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <stdio.h>
#include <sys/stat.h>
#include <sys/socket.h>
#include <sys/un.h>

#ifndef PATH_MAX
#define PATH_MAX 256
#endif

// ---------------------------------------------------------------------------
// Build the Unix socket path.  Same convention as UnixStream.cpp:
//   $TWOYI_ROOTFS/opengles3   (default /data/data/io.twoyi/rootfs/opengles3)
// The legacy blob hardcodes "/data/data/io.twoyi/rootfs/opengles3" at
// .rodata vaddr 0xdd2d8 (referenced by GraphicBuffer::create).
// ---------------------------------------------------------------------------
static int make_gb_path(char* path, size_t pathlen)
{
    const char* rootfs = getenv("TWOYI_ROOTFS");
    if (rootfs == NULL || rootfs[0] == 0) {
        rootfs = "/data/data/io.twoyi/rootfs";
    }
    snprintf(path, pathlen, "%s/opengles3", rootfs);
    return 0;
}

GraphicBuffer::GraphicBuffer()
    : m_listenSock(-1),
      m_recvHandle(NULL),
      m_toNativeWindowBuffer(NULL)
{
}

GraphicBuffer::~GraphicBuffer()
{
    if (m_listenSock >= 0) {
        close(m_listenSock);
        m_listenSock = -1;
    }
}

// ---------------------------------------------------------------------------
// create — open the listening socket.  Mirrors the legacy's use of
// emugl::socketLocalServer() (the legacy blob calls
//   emugl::socketLocalServer("/data/data/io.twoyi/rootfs/opengles3", 1)
// — the second arg, 1, is ANDROID_SOCKET_NAMESPACE_FILESYSTEM).
// We first unlink any stale socket file (the legacy does access()+remove()).
// ---------------------------------------------------------------------------
GraphicBuffer* GraphicBuffer::create()
{
    GraphicBuffer* gb = new GraphicBuffer();
    if (!gb) return NULL;

    char path[PATH_MAX];
    if (make_gb_path(path, sizeof(path)) < 0) {
        delete gb;
        return NULL;
    }

    // Remove a stale socket file if present (legacy: access()+remove()).
    if (access(path, F_OK) == 0) {
        unlink(path);
    }

    int fd = socket_local_server(path, ANDROID_SOCKET_NAMESPACE_FILESYSTEM,
                                 SOCK_STREAM);
    if (fd < 0) {
        ALOGE("GraphicBuffer::create: socket_local_server(%s) failed: %s",
              path, strerror(errno));
        delete gb;
        return NULL;
    }
    // Match the legacy permission (chmod 0777) so the guest can connect.
    chmod(path, 0777);

    gb->m_listenSock = fd;
    return gb;
}

// ---------------------------------------------------------------------------
// Main — accept loop.  For each client connection, call
// AHardwareBuffer_recvHandleFromUnixSocket() to receive an AHardwareBuffer
// file descriptor from the guest, then convert it to an ANativeWindowBuffer
// via AHardwareBuffer_to_ANativeWindowBuffer().
//
// The received buffer is held by libandroid internally; we don't free it.
// (In a full SurfaceFlinger-compositing implementation, the buffer would
// be handed to FrameBuffer for compositing; that's deferred to a future
// task — see PORT_RESULTS.md.)
// ---------------------------------------------------------------------------
int GraphicBuffer::Main()
{
    if (m_listenSock < 0) {
        ALOGE("GraphicBuffer::Main: no listening socket");
        return -1;
    }
    if (!m_recvHandle) {
        ALOGE("GraphicBuffer::Main: AHardwareBuffer_recvHandleFromUnixSocket "
              "not set");
        return -1;
    }

    while (true) {
        struct sockaddr_un addr;
        socklen_t len = sizeof(addr);
        int client = ::accept(m_listenSock, (sockaddr*)&addr, &len);
        if (client < 0) {
            if (errno == EINTR) continue;
            ALOGE("GraphicBuffer::Main: accept failed: %s", strerror(errno));
            break;
        }

        // Receive one AHardwareBuffer from the guest over this socket.
        AHardwareBuffer* buf = NULL;
        int rc = m_recvHandle(client, &buf);
        if (rc != 0 || !buf) {
            ALOGW("GraphicBuffer::Main: recvHandleFromUnixSocket rc=%d", rc);
            close(client);
            continue;
        }

        // Convert to ANativeWindowBuffer (the legacy buffer type used by
        // the rest of the renderer).  We don't currently do anything with
        // the converted buffer; in the legacy blob it's stored in a
        // global lookup table keyed by the buffer id (GraphicBuffer has
        // 15 methods — most are register/unregister/lookup helpers that
        // we deliberately omit here).
        if (m_toNativeWindowBuffer) {
            ANativeWindowBuffer* anwb = m_toNativeWindowBuffer(buf);
            (void)anwb;  // reserved for future FrameBuffer integration
        }

        close(client);
    }

    return 0;
}
