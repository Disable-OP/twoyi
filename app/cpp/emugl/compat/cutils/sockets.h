#pragma once
#include <sys/socket.h>
static inline int socket_local_server(const char* name, int type) {
    (void)name; (void)type; return -1;
}
