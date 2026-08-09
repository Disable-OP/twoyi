// SmartPtr.h — fixed version for Android NDK without platform-private headers
#pragma once
#include <pthread.h>
#include <stdlib.h>

typedef pthread_mutex_t mutex_t;
static inline int mutex_init(mutex_t* m) { return pthread_mutex_init(m, NULL); }
static inline int mutex_destroy(mutex_t* m) { return pthread_mutex_destroy(m); }
static inline int mutex_lock(mutex_t* m) { return pthread_mutex_lock(m); }
static inline int mutex_unlock(mutex_t* m) { return pthread_mutex_unlock(m); }

static inline int android_atomic_inc(volatile int* addr) {
    return __sync_fetch_and_add(addr, 1);
}
static inline int android_atomic_dec(volatile int* addr) {
    return __sync_fetch_and_sub(addr, 1);
}

template <class T>
class SmartPtr {
public:
    SmartPtr() : m_ptr(NULL), m_pRefCount(NULL), m_lock(NULL) {
        m_pRefCount = new int;
        *m_pRefCount = 0;
        m_lock = new mutex_t;
        mutex_init(m_lock);
    }
    SmartPtr(T* ptr) : m_ptr(ptr), m_pRefCount(NULL), m_lock(NULL) {
        m_pRefCount = new int;
        *m_pRefCount = 1;
        m_lock = new mutex_t;
        mutex_init(m_lock);
    }
    SmartPtr(const SmartPtr<T>& other) : m_ptr(NULL), m_pRefCount(NULL), m_lock(NULL) {
        m_lock = new mutex_t;
        mutex_init(m_lock);
        *this = other;
    }
    virtual ~SmartPtr() {
        if (m_lock) {
            mutex_lock(m_lock);
            if (m_pRefCount && --(*m_pRefCount) == 0) {
                if (m_ptr) delete m_ptr;
                delete m_pRefCount;
                m_pRefCount = NULL;
            }
            mutex_unlock(m_lock);
            mutex_destroy(m_lock);
            delete m_lock;
        }
    }
    SmartPtr<T>& operator=(const SmartPtr<T>& other) {
        if (this != &other) {
            if (other.m_lock) mutex_lock(other.m_lock);
            m_ptr = other.m_ptr;
            m_pRefCount = other.m_pRefCount;
            m_lock = other.m_lock;
            if (m_pRefCount) (*m_pRefCount)++;
            if (other.m_lock) mutex_unlock(other.m_lock);
        }
        return *this;
    }
    T* operator->() const { return m_ptr; }
    T& operator*() const { return *m_ptr; }
    operator T*() const { return m_ptr; }
    bool operator!() const { return m_ptr == NULL; }
    T* get() const { return m_ptr; }
    T* Ptr() const { return m_ptr; }
    T* release() {
        T* tmp = m_ptr;
        m_ptr = NULL;
        return tmp;
    }
    void attach(T* ptr) {
        if (m_ptr && --(*m_pRefCount) == 0) {
            delete m_ptr;
            delete m_pRefCount;
        }
        m_ptr = ptr;
        m_pRefCount = new int;
        *m_pRefCount = 1;
    }
private:
    T* m_ptr;
    int* m_pRefCount;
    mutex_t* m_lock;
};
