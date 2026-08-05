#ifndef _TWOYI_CUTILS_SOCKETS_H
#define _TWOYI_CUTILS_SOCKETS_H
/*
 * Compatibility shim for <cutils/sockets.h>.
 *
 * The AOSP emugl UnixStream.cpp uses socket_local_server() /
 * socket_local_client() to create/listen/connect on a named Unix
 * domain socket. The real implementation lives in libcutils, which is
 * not shipped with the NDK, so we re-implement the two functions we
 * need on top of plain POSIX AF_UNIX sockets.
 */
#include <sys/socket.h>
#include <sys/un.h>
#include <stddef.h>
#include <string.h>
#include <unistd.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
    ANDROID_SOCKET_NAMESPACE_NONE = 0,
    ANDROID_SOCKET_NAMESPACE_ABSTRACT,
    ANDROID_SOCKET_NAMESPACE_RESERVED,
    ANDROID_SOCKET_NAMESPACE_FILESYSTEM,
};

static __inline__ int socket_local_server(const char *name,
                                          int namespaceId, int type) {
    (void)namespaceId;  // we always use the filesystem namespace
    int fd = socket(AF_UNIX, type, 0);
    if (fd < 0) return -1;

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, name, sizeof(addr.sun_path) - 1);

    /* unlink any stale socket file so bind() succeeds */
    unlink(name);

    if (bind(fd, (struct sockaddr *)&addr,
             offsetof(struct sockaddr_un, sun_path) + strlen(name) + 1) < 0) {
        close(fd);
        return -1;
    }
    if (type == SOCK_STREAM) {
        if (listen(fd, 5) < 0) {
            close(fd);
            return -1;
        }
    }
    return fd;
}

static __inline__ int socket_local_client(const char *name,
                                          int namespaceId, int type) {
    (void)namespaceId;
    int fd = socket(AF_UNIX, type, 0);
    if (fd < 0) return -1;

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, name, sizeof(addr.sun_path) - 1);

    if (connect(fd, (struct sockaddr *)&addr,
                offsetof(struct sockaddr_un, sun_path) + strlen(name) + 1) < 0) {
        close(fd);
        return -1;
    }
    return fd;
}

/* These two are referenced by some emugl sources but never actually
 * called at runtime in twoyi. Provide stubs that fail gracefully. */
static __inline__ int socket_loopback_server(int port, int type) {
    (void)port; (void)type;
    return -1;
}
static __inline__ int socket_inaddr_any_server(int port, int type) {
    (void)port; (void)type;
    return -1;
}
static __inline__ int socket_network_client(const char *host, int port, int type) {
    (void)host; (void)port; (void)type;
    return -1;
}

#ifdef __cplusplus
}
#endif

#endif /* _TWOYI_CUTILS_SOCKETS_H */
