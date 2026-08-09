#!/bin/bash
# Sets up compat shim headers + patched source files for libOpenglRender build.
set -e

BUILD=/tmp/build_opengl
AOSP=/tmp/aosp-sdk/emulator/opengl
COMPAT=$BUILD/compat
SRC=$BUILD/src
mkdir -p $COMPAT/cutils $COMPAT/utils $SRC

# ----------------------------------------------------------------------------
# compat/cutils/threads.h
# ----------------------------------------------------------------------------
cat > $COMPAT/cutils/threads.h <<'EOF'
#ifndef _CUTILS_THREADS_H
#define _CUTILS_THREADS_H
#include <pthread.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    pthread_key_t key;
    int inited;
} thread_store_t;

#define THREAD_STORE_INITIALIZER { 0, 0 }

void* thread_store_get(thread_store_t* store);
void  thread_store_set(thread_store_t* store, void* value, void (*destruct)(void*));

typedef pthread_mutex_t mutex_t;

static inline void mutex_init(mutex_t* m) { pthread_mutex_init(m, NULL); }
static inline void mutex_destroy(mutex_t* m) { pthread_mutex_destroy(m); }
static inline void mutex_lock(mutex_t* m) { pthread_mutex_lock(m); }
static inline void mutex_unlock(mutex_t* m) { pthread_mutex_unlock(m); }

#ifdef __cplusplus
}
#endif

#endif
EOF

# ----------------------------------------------------------------------------
# compat/cutils/atomic.h
# ----------------------------------------------------------------------------
cat > $COMPAT/cutils/atomic.h <<'EOF'
#ifndef _CUTILS_ATOMIC_H
#define _CUTILS_ATOMIC_H
#include <stdint.h>
#include <stdatomic.h>

#ifdef __cplusplus
extern "C" {
#endif

static inline int32_t android_atomic_inc(volatile int32_t* addr) {
    return atomic_fetch_add((_Atomic int32_t*)addr, 1);
}
static inline int32_t android_atomic_dec(volatile int32_t* addr) {
    return atomic_fetch_sub((_Atomic int32_t*)addr, 1);
}
static inline int32_t android_atomic_acquire_load(volatile const int32_t* addr) {
    return atomic_load_explicit((_Atomic const int32_t*)addr, memory_order_acquire);
}
static inline void android_atomic_release_store(int32_t val, volatile int32_t* addr) {
    atomic_store_explicit((_Atomic int32_t*)addr, val, memory_order_release);
}
static inline int android_atomic_acquire_cas(int32_t oldval, int32_t newval, volatile int32_t* addr) {
    int32_t expected = oldval;
    return !atomic_compare_exchange_strong_explicit((_Atomic int32_t*)addr, &expected, newval,
                                                    memory_order_acquire, memory_order_acquire);
}

#ifdef __cplusplus
}
#endif

#endif
EOF

# ----------------------------------------------------------------------------
# compat/cutils/log.h
# ----------------------------------------------------------------------------
cat > $COMPAT/cutils/log.h <<'EOF'
#ifndef _CUTILS_LOG_H
#define _CUTILS_LOG_H
#include <android/log.h>

#ifdef __cplusplus
extern "C" {
#endif

#define ALOGE(...) __android_log_print(ANDROID_LOG_ERROR, "emugl", __VA_ARGS__)
#define ALOGW(...) __android_log_print(ANDROID_LOG_WARN,  "emugl", __VA_ARGS__)
#define ALOGI(...) __android_log_print(ANDROID_LOG_INFO,  "emugl", __VA_ARGS__)
#define ALOGD(...) __android_log_print(ANDROID_LOG_DEBUG, "emugl", __VA_ARGS__)
#define ALOGV(...) __android_log_print(ANDROID_LOG_VERBOSE,"emugl", __VA_ARGS__)
#define ALOG_ASSERT(cond, ...) do { if (!(cond)) { __android_log_assert(#cond, "emugl", __VA_ARGS__); } } while (0)

#define LOG_PRI(prio, tag, ...) __android_log_print(prio, tag, __VA_ARGS__)
#define LOG_TAG "emugl"

#ifdef __cplusplus
}
#endif

#endif
EOF

# ----------------------------------------------------------------------------
# compat/cutils/sockets.h
# ----------------------------------------------------------------------------
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

#ifdef __cplusplus
}
#endif
#endif
EOF

# ----------------------------------------------------------------------------
# compat/utils/threads.h
# ----------------------------------------------------------------------------
cat > $COMPAT/utils/threads.h <<'EOF'
#ifndef _UTILS_THREADS_H
#define _UTILS_THREADS_H
#include <pthread.h>
#include <stdint.h>
#include <stdlib.h>
#include <utils/Errors.h>

namespace android {

class Mutex {
public:
    Mutex() { pthread_mutex_init(&mMutex, NULL); }
    ~Mutex() { pthread_mutex_destroy(&mMutex); }
    int lock() { return pthread_mutex_lock(&mMutex); }
    int unlock() { return pthread_mutex_unlock(&mMutex); }
    int tryLock() { return pthread_mutex_trylock(&mMutex); }

    class Autolock {
    public:
        explicit Autolock(Mutex& m) : mLock(m) { mLock.lock(); }
        ~Autolock() { mLock.unlock(); }
    private:
        Mutex& mLock;
    };

private:
    pthread_mutex_t mMutex;
};

typedef Mutex::Autolock AutoMutex;

} // namespace android
#endif
EOF

# ----------------------------------------------------------------------------
# compat/utils/Errors.h
# ----------------------------------------------------------------------------
cat > $COMPAT/utils/Errors.h <<'EOF'
#ifndef _UTILS_ERRORS_H
#define _UTILS_ERRORS_H
#include <sys/types.h>
namespace android {
typedef int32_t status_t;
}
#ifndef NO_ERROR
#define NO_ERROR 0
#endif
#define UNKNOWN_ERROR (-1)
#define BAD_VALUE (-2)
#define BAD_INDEX (-3)
#define ALREADY_EXISTS (-4)
#define NO_INIT (-5)
#define NO_MEMORY (-6)
#define INVALID_OPERATION (-7)
#endif
EOF

# ----------------------------------------------------------------------------
# compat/utils/Vector.h
# ----------------------------------------------------------------------------
cat > $COMPAT/utils/Vector.h <<'EOF'
#ifndef _UTILS_VECTOR_H
#define _UTILS_VECTOR_H
#include <vector>
#include <cstddef>
#include <utils/Errors.h>
namespace android {

template <class T>
class Vector {
public:
    Vector() {}
    inline size_t size() const { return mVec.size(); }
    inline bool isEmpty() const { return mVec.empty(); }
    inline T& operator[](size_t ix) { return mVec[ix]; }
    inline const T& operator[](size_t ix) const { return mVec[ix]; }
    inline ssize_t add(const T& val) { mVec.push_back(val); return (ssize_t)mVec.size() - 1; }
    inline ssize_t insertAt(const T& val, size_t index, size_t count = 1) {
        if (index > mVec.size()) return BAD_VALUE;
        mVec.insert(mVec.begin() + index, count, val);
        return (ssize_t)index;
    }
    inline void removeAt(size_t index) { if (index < mVec.size()) mVec.erase(mVec.begin() + index); }
    inline void clear() { mVec.clear(); }
    inline T* editArray() { return mVec.data(); }
    inline const T* array() const { return mVec.data(); }

private:
    std::vector<T> mVec;
};

} // namespace android
#endif
EOF

# ----------------------------------------------------------------------------
# compat/utils/List.h
# ----------------------------------------------------------------------------
cat > $COMPAT/utils/List.h <<'EOF'
#ifndef _UTILS_LIST_H
#define _UTILS_LIST_H
#include <list>
namespace android {
template <class T>
class List : public std::list<T> {
public:
    typedef typename std::list<T>::iterator iterator;
    typedef typename std::list<T>::const_iterator const_iterator;
};
} // namespace android
#endif
EOF

# ----------------------------------------------------------------------------
# compat/utils/String8.h
# ----------------------------------------------------------------------------
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
    size_t size() const { return mStr.size(); }
    size_t length() const { return mStr.size(); }
    bool isEmpty() const { return mStr.empty(); }
    String8& append(const char* s) { mStr.append(s); return *this; }
    String8& append(const String8& s) { mStr.append(s.mStr); return *this; }
    bool operator==(const String8& rhs) const { return mStr == rhs.mStr; }
    bool operator<(const String8& rhs) const { return mStr < rhs.mStr; }
    operator const char*() const { return mStr.c_str(); }
private:
    std::string mStr;
};

} // namespace android
#endif
EOF

# ----------------------------------------------------------------------------
# compat/utils/KeyedVector.h
# ----------------------------------------------------------------------------
cat > $COMPAT/utils/KeyedVector.h <<'EOF'
#ifndef _UTILS_KEYED_VECTOR_H
#define _UTILS_KEYED_VECTOR_H
#include <map>
#include <vector>
#include <cstddef>
#include <utils/Errors.h>
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
    inline const VALUE& valueAt(size_t idx) const { return mVec[idx].second; }
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
    inline ssize_t removeItem(const KEY& key) {
        typename std::map<KEY,size_t>::iterator it = mIndex.find(key);
        if (it == mIndex.end()) return (ssize_t)-1;
        size_t idx = it->second;
        mVec.erase(mVec.begin() + idx);
        mIndex.erase(it);
        // rebuild index
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

# ----------------------------------------------------------------------------
# compat/utils/RefBase.h
# ----------------------------------------------------------------------------
cat > $COMPAT/utils/RefBase.h <<'EOF'
#ifndef _UTILS_REF_BASE_H
#define _UTILS_REF_BASE_H
#include <utils/Errors.h>
namespace android {
class RefBase {
public:
    RefBase() : mRefCnt(0) {}
    virtual ~RefBase() {}
    void incStrong(const void* id) const { mRefCnt++; }
    void decStrong(const void* id) const { if (--mRefCnt == 0) delete this; }
private:
    mutable int mRefCnt;
};
} // namespace android
#endif
EOF

# ----------------------------------------------------------------------------
# compat.cpp - implementations of cutils functions
# ----------------------------------------------------------------------------
cat > $SRC/compat.cpp <<'EOF'
// Implementation of compat shims (cutils/sockets, cutils/threads)
#include <cutils/threads.h>
#include <cutils/sockets.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/stat.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include <errno.h>
#include <stdio.h>

// ----- thread_store -----
extern "C" void* thread_store_get(thread_store_t* store) {
    if (!store->inited) return NULL;
    return pthread_getspecific(store->key);
}

extern "C" void thread_store_set(thread_store_t* store, void* value, void (*destruct)(void*)) {
    if (!store->inited) {
        if (pthread_key_create(&store->key, destruct) != 0) return;
        store->inited = 1;
    }
    pthread_setspecific(store->key, value);
}

// ----- socket_local_server / socket_local_client -----
extern "C" int socket_local_server(const char* name, int namespaceId, int type) {
    int sock = socket(AF_UNIX, type, 0);
    if (sock < 0) return -1;

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;

    if (namespaceId == ANDROID_SOCKET_NAMESPACE_ABSTRACT) {
        const size_t maxlen = sizeof(addr.sun_path) - 1;
        size_t namelen = strlen(name);
        if (namelen > maxlen) namelen = maxlen;
        addr.sun_path[0] = '\0';
        memcpy(addr.sun_path + 1, name, namelen);
        socklen_t alen = offsetof(struct sockaddr_un, sun_path) + 1 + namelen;
        if (bind(sock, (struct sockaddr*)&addr, alen) < 0) { close(sock); return -1; }
    } else {
        size_t maxlen = sizeof(addr.sun_path) - 1;
        if (strlen(name) >= maxlen) { close(sock); errno = ENAMETOOLONG; return -1; }
        strncpy(addr.sun_path, name, maxlen);
        addr.sun_path[maxlen] = '\0';
        unlink(name);
        char tmp[256];
        strncpy(tmp, name, sizeof(tmp)-1);
        tmp[sizeof(tmp)-1] = '\0';
        char* slash = strrchr(tmp, '/');
        if (slash) {
            *slash = '\0';
            if (tmp[0]) mkdir(tmp, 0777);
        }
        if (bind(sock, (struct sockaddr*)&addr, sizeof(addr)) < 0) { close(sock); return -1; }
        chmod(name, 0777);
    }

    if (type == SOCK_STREAM) {
        if (listen(sock, 5) < 0) { close(sock); return -1; }
    }
    return sock;
}

extern "C" int socket_local_client(const char* name, int namespaceId, int type) {
    int sock = socket(AF_UNIX, type, 0);
    if (sock < 0) return -1;

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;

    if (namespaceId == ANDROID_SOCKET_NAMESPACE_ABSTRACT) {
        const size_t maxlen = sizeof(addr.sun_path) - 1;
        size_t namelen = strlen(name);
        if (namelen > maxlen) namelen = maxlen;
        addr.sun_path[0] = '\0';
        memcpy(addr.sun_path + 1, name, namelen);
        socklen_t alen = offsetof(struct sockaddr_un, sun_path) + 1 + namelen;
        if (connect(sock, (struct sockaddr*)&addr, alen) < 0) { close(sock); return -1; }
    } else {
        size_t maxlen = sizeof(addr.sun_path) - 1;
        if (strlen(name) >= maxlen) { close(sock); errno = ENAMETOOLONG; return -1; }
        strncpy(addr.sun_path, name, maxlen);
        addr.sun_path[maxlen] = '\0';
        if (connect(sock, (struct sockaddr*)&addr, sizeof(addr)) < 0) { close(sock); return -1; }
    }
    return sock;
}
EOF

echo "=== compat layer created ==="
ls $COMPAT/cutils/ $COMPAT/utils/ $SRC/compat.cpp
