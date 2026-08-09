/*
 * UnixStream.cpp — patched for twoyi.
 *
 * Original AOSP file: emulator/opengl/shared/OpenglCodecCommon/UnixStream.cpp
 * Patch: make_unix_path() now builds the pipe path as
 *        $TWOYI_ROOTFS/opengles{,2,3} (default /data/data/io.twoyi/rootfs)
 *        instead of /tmp/android-$USER/qemu-gles-$port. This matches
 *        the path the twoyi rootfs expects the renderer to listen on.
 *
 * The rest of the file is byte-for-byte identical to the AOSP original.
 */
#include "UnixStream.h"
#include "cutils/sockets.h"
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

#include <netinet/in.h>
#include <netinet/tcp.h>
#include <sys/un.h>
#include <sys/stat.h>

#ifndef PATH_MAX
#define PATH_MAX   128
#endif

UnixStream::UnixStream(size_t bufSize) :
    SocketStream(bufSize)
{
}

UnixStream::UnixStream(int sock, size_t bufSize) :
    SocketStream(sock, bufSize)
{
}

/* Initialize a sockaddr_un path for a given 'virtual port'.
 *
 * twoyi patch: the path is $TWOYI_ROOTFS/opengles{,2,3} where the
 * suffix is selected by (port % 3). The default rootfs is
 * /data/data/io.twoyi/rootfs (matching the legacy closed-source blob's
 * hardcoded path). TWOYI_ROOTFS can override it at runtime.
 */
static int
make_unix_path(char *path, size_t pathlen, int port_number)
{
    const char *rootfs = getenv("TWOYI_ROOTFS");
    if (rootfs == NULL || rootfs[0] == 0) {
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


int UnixStream::listen(unsigned short port)
{
    char  path[PATH_MAX];

    if (make_unix_path(path, sizeof(path), port) < 0) {
        return -1;
    }

    m_sock = socket_local_server(path, ANDROID_SOCKET_NAMESPACE_FILESYSTEM, SOCK_STREAM);
    if (!valid()) return int(ERR_INVALID_SOCKET);

    return 0;
}

SocketStream * UnixStream::accept()
{
    int clientSock = -1;

    while (true) {
        struct sockaddr_un addr;
        socklen_t len = sizeof(addr);
        clientSock = ::accept(m_sock, (sockaddr *)&addr, &len);

        if (clientSock < 0 && errno == EINTR) {
            continue;
        }
        break;
    }

    UnixStream *clientStream = NULL;

    if (clientSock >= 0) {
        clientStream =  new UnixStream(clientSock, m_bufsize);
    }
    return clientStream;
}

int UnixStream::connect(unsigned short port)
{
    char  path[PATH_MAX];

    if (make_unix_path(path, sizeof(path), port) < 0)
        return -1;

    m_sock = socket_local_client(path, ANDROID_SOCKET_NAMESPACE_FILESYSTEM, SOCK_STREAM);
    if (!valid()) return -1;

    return 0;
}
