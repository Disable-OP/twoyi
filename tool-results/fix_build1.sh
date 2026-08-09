#!/bin/bash
# Fix multiple build issues:
# 1. Rewrite UnixStream.cpp cleanly (avoid Python patch issues)
# 2. Add Vector.h include + replaceValueFor to compat KeyedVector
# 3. Add operator==(const char*) to compat String8
# 4. Add -include assert.h to CMakeLists.txt
set -e
BUILD=/tmp/build_opengl
SRC=$BUILD/src
COMPAT=$BUILD/compat

# ---- 1. Rewrite UnixStream.cpp cleanly ----
cat > $SRC/UnixStream.cpp <<'EOF'
/*
 * Patched for twoyi: Unix socket paths under the container rootfs.
 * Original Copyright (C) 2011 The Android Open Source Project, Apache 2.0.
 */
#include "UnixStream.h"
#include <cutils/sockets.h>
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
#define PATH_MAX   256
#endif

UnixStream::UnixStream(size_t bufSize) :
    SocketStream(bufSize)
{
}

UnixStream::UnixStream(int sock, size_t bufSize) :
    SocketStream(sock, bufSize)
{
}

/*
 * twoyi: build the opengles pipe path. The "port" selects which pipe:
 *   port % 3 == 0 -> $TWOYI_ROOTFS/opengles   (default /data/data/io.twoyi/rootfs/opengles)
 *   port % 3 == 1 -> $TWOYI_ROOTFS/opengles2
 *   port % 3 == 2 -> $TWOYI_ROOTFS/opengles3
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
EOF

# ---- 2. Update compat/utils/KeyedVector.h: include Vector.h, add replaceValueFor ----
cat > $COMPAT/utils/KeyedVector.h <<'EOF'
#ifndef _UTILS_KEYED_VECTOR_H
#define _UTILS_KEYED_VECTOR_H
#include <map>
#include <vector>
#include <cstddef>
#include <utils/Errors.h>
#include <utils/Vector.h>
namespace android {

template <typename KEY, typename VALUE>
class KeyedVector {
public:
    KeyedVector() {}
    inline size_t size() const { return mVec.size(); }
    inline bool isEmpty() const { return mVec.empty(); }
    inline const VALUE& valueFor(const KEY& key) const {
        typename std::map<KEY,size_t>::const_iterator it = mIndex.find(key);
        return (it == mIndex.end()) ? mDefault : mVec[it->second].second;
    }
    inline VALUE& editValueFor(const KEY& key) {
        typename std::map<KEY,size_t>::iterator it = mIndex.find(key);
        if (it != mIndex.end()) return mVec[it->second].second;
        // not present: add default-constructed value
        mIndex[key] = mVec.size();
        mVec.push_back(std::make_pair(key, VALUE()));
        return mVec.back().second;
    }
    inline const VALUE& valueAt(size_t idx) const { return mVec[idx].second; }
    inline VALUE& editValueAt(size_t idx) { return mVec[idx].second; }
    inline const KEY& keyAt(size_t idx) const { return mVec[idx].first; }
    inline ssize_t indexOfKey(const KEY& key) const {
        typename std::map<KEY,size_t>::const_iterator it = mIndex.find(key);
        return (it == mIndex.end()) ? (ssize_t)-1 : (ssize_t)it->second;
    }
    inline ssize_t add(const KEY& key, const VALUE& val) {
        typename std::map<KEY,size_t>::iterator it = mIndex.find(key);
        if (it != mIndex.end()) { mVec[it->second].second = val; return (ssize_t)it->second; }
        mIndex[key] = mVec.size();
        mVec.push_back(std::make_pair(key, val));
        return (ssize_t)mVec.size() - 1;
    }
    inline ssize_t replaceValueFor(const KEY& key, const VALUE& val) {
        typename std::map<KEY,size_t>::iterator it = mIndex.find(key);
        if (it != mIndex.end()) { mVec[it->second].second = val; return (ssize_t)it->second; }
        mIndex[key] = mVec.size();
        mVec.push_back(std::make_pair(key, val));
        return (ssize_t)mVec.size() - 1;
    }
    inline ssize_t removeItem(const KEY& key) {
        typename std::map<KEY,size_t>::iterator it = mIndex.find(key);
        if (it == mIndex.end()) return (ssize_t)-1;
        size_t idx = it->second;
        mVec.erase(mVec.begin() + idx);
        mIndex.erase(it);
        mIndex.clear();
        for (size_t i = 0; i < mVec.size(); i++) mIndex[mVec[i].first] = i;
        return (ssize_t)idx;
    }
    inline void removeItemsAt(size_t idx, size_t count = 1) {
        if (idx >= mVec.size()) return;
        size_t end = idx + count; if (end > mVec.size()) end = mVec.size();
        mVec.erase(mVec.begin() + idx, mVec.begin() + end);
        mIndex.clear();
        for (size_t i = 0; i < mVec.size(); i++) mIndex[mVec[i].first] = i;
    }
    inline void clear() { mVec.clear(); mIndex.clear(); }
protected:
    VALUE mDefault;
    std::vector<std::pair<KEY,VALUE> > mVec;
    std::map<KEY,size_t> mIndex;
};

template <typename KEY, typename VALUE>
class DefaultKeyedVector : public KeyedVector<KEY,VALUE> {
public:
    explicit DefaultKeyedVector(const VALUE& def) { this->mDefault = def; }
};

} // namespace android
#endif
EOF

# ---- 3. Update compat/utils/String8.h: add operator==(const char*) and operator!= ----
cat > $COMPAT/utils/String8.h <<'EOF'
#ifndef _UTILS_STRING8_H
#define _UTILS_STRING8_H
#include <string.h>
#include <string>
namespace android {

class String8 {
public:
    String8() {}
    String8(const char* s) : mStr(s ? s : "") {}
    String8(const std::string& s) : mStr(s) {}
    const char* string() const { return mStr.c_str(); }
    const char* c_str() const { return mStr.c_str(); }
    size_t size() const { return mStr.size(); }
    size_t length() const { return mStr.size(); }
    bool isEmpty() const { return mStr.empty(); }
    String8& append(const char* s) { mStr.append(s); return *this; }
    String8& append(const String8& s) { mStr.append(s.mStr); return *this; }
    bool operator==(const String8& rhs) const { return mStr == rhs.mStr; }
    bool operator==(const char* rhs) const { return rhs ? mStr == rhs : false; }
    bool operator!=(const String8& rhs) const { return mStr != rhs.mStr; }
    bool operator!=(const char* rhs) const { return rhs ? mStr != rhs : true; }
    bool operator<(const String8& rhs) const { return mStr < rhs.mStr; }
    operator const char*() const { return mStr.c_str(); }
private:
    std::string mStr;
};

} // namespace android
#endif
EOF

# ---- 4. Update CMakeLists.txt: add -include assert.h and string.h ----
sed -i 's|target_compile_options(OpenglRender PRIVATE|target_compile_options(OpenglRender PRIVATE\n    -include assert.h\n    -include string.h|' $BUILD/CMakeLists.txt

echo "=== fixes applied ==="
grep -n "include assert.h" $BUILD/CMakeLists.txt
