#pragma once
// compat/cutils/sockets.h — compatibility shim for the Android platform-private
// <cutils/sockets.h>. The AOSP emugl sources (UnixStream.cpp, TcpStream.cpp)
// call socket_local_server / socket_local_client / socket_loopback_server /
// socket_network_client. On Android these are in libcutils.so, but we can't
// link against the platform-private cutils. This shim reimplements the
// Unix-socket variants on top of plain POSIX sockets, and stubs out the
// TCP variants (twoyi uses STREAM_MODE_UNIX, so the TCP paths are never
// taken at runtime).

#include <sys/socket.h>
#include <sys/un.h>
#include <netinet/in.h>
#include <string.h>
#include <unistd.h>

// Android uses these namespace constants. Twoyi always uses FILESYSTEM
// (the path is a real filesystem path like /data/.../opengles).
#define ANDROID_SOCKET_NAMESPACE_FILESYSTEM  0
#define ANDROID_SOCKET_NAMESPACE_ABSTRACT   1

// socket_local_server — create a Unix-domain listening socket at `name`.
// `namespaceType` is ignored (we always treat `name` as a filesystem path).
// Returns the listening fd, or -1 on error.
static inline int socket_local_server(const char* name, int namespaceType, int type) {
    (void)namespaceType;
    int fd = socket(AF_UNIX, type, 0);
    if (fd < 0) return -1;

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    // sockaddr_un.sun_path is 108 bytes max (UNIX_PATH_MAX).
    strncpy(addr.sun_path, name, sizeof(addr.sun_path) - 1);

    // Unlink any stale socket file at this path.
    unlink(name);

    if (bind(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(fd);
        return -1;
    }
    if (listen(fd, 5) < 0) {
        close(fd);
        return -1;
    }
    return fd;
}

// socket_local_client — connect to a Unix-domain socket at `name`.
// Returns the connected fd, or -1 on error.
static inline int socket_local_client(const char* name, int namespaceType, int type) {
    (void)namespaceType;
    int fd = socket(AF_UNIX, type, 0);
    if (fd < 0) return -1;

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, name, sizeof(addr.sun_path) - 1);

    if (connect(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(fd);
        return -1;
    }
    return fd;
}

// socket_loopback_server — create a TCP listening socket on 127.0.0.1.
// Stub: twoyi uses Unix sockets, so this should never be called.
// Returns -1 to indicate "not supported".
static inline int socket_loopback_server(int port, int type) {
    (void)port; (void)type;
    return -1;
}

// socket_network_client — connect to a TCP host:port.
// Stub: twoyi uses Unix sockets, so this should never be called.
static inline int socket_network_client(const char* host, int port, int type) {
    (void)host; (void)port; (void)type;
    return -1;
}
