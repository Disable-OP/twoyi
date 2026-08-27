// SmartPtr.h — intrusive-style refcounted smart pointer for the emugl
// renderer (Android NDK build, no platform-private headers).
//
// 6-Z184 AUDIT FIX (agents 34/50): the previous "fixed for NDK" rewrite
// had two memory bugs:
//   1. destructor unconditionally ran mutex_destroy(m_lock); delete
//      m_lock; even when the shared refcount was still > 0 — destroying
//      ANY non-last copy freed the mutex surviving copies still point
//      at → use-after-free (SmartPtr copies happen on every by-value
//      pass / std::map insert of ColorBufferPtr / RenderContextPtr /
//      WindowSurfacePtr).
//   2. copy-constructor allocated a fresh mutex and then let
//      operator= overwrite m_lock with the source's lock → leaked the
//      fresh mutex on every copy.
//
// The rewrite below makes ALL copies share ONE refcount + ONE mutex,
// and only the LAST owner frees them. This mirrors the original AOSP
// <cutils> SmartPtr ownership model with plain pthreads.
#pragma once
#include <pthread.h>
#include <stdlib.h>

typedef pthread_mutex_t mutex_t;
static inline int mutex_init(mutex_t* m) { return pthread_mutex_init(m, NULL); }
static inline int mutex_destroy(mutex_t* m) { return pthread_mutex_destroy(m); }
static inline int mutex_lock(mutex_t* m) { return pthread_mutex_lock(m); }
static inline int mutex_unlock(mutex_t* m) { return pthread_mutex_unlock(m); }

template <class T>
class SmartPtr {
public:
    SmartPtr() : m_ptr(NULL), m_pRefCount(new int), m_lock(new mutex_t) {
        // Invariant: *m_pRefCount == the number of SmartPtr objects
        // sharing this control block (always >= 1; the block is freed
        // when it drops to 0). A default-constructed instance IS an
        // owner of its own block.
        *m_pRefCount = 1;
        mutex_init(m_lock);
    }
    explicit SmartPtr(T* ptr) : m_ptr(ptr), m_pRefCount(new int), m_lock(new mutex_t) {
        *m_pRefCount = 1;
        mutex_init(m_lock);
    }
    // Copy: share the source's pointer, refcount AND lock. No fresh
    // allocations, nothing to leak.
    SmartPtr(const SmartPtr<T>& other)
        : m_ptr(other.m_ptr), m_pRefCount(other.m_pRefCount), m_lock(other.m_lock) {
        mutex_lock(m_lock);
        ++(*m_pRefCount);
        mutex_unlock(m_lock);
    }
    ~SmartPtr() {
        release();
    }

    SmartPtr<T>& operator=(const SmartPtr<T>& other) {
        if (this == &other) return *this;
        // Self-defense: assigning from a copy that shares our control
        // block would destroy it under us if we released first.
        mutex_lock(other.m_lock);
        if (m_pRefCount == other.m_pRefCount) {
            // Same control block — nothing to do (ptr identical).
            mutex_unlock(other.m_lock);
            return *this;
        }
        ++(*other.m_pRefCount);
        mutex_unlock(other.m_lock);
        // Now drop our old reference (cannot touch other's block).
        release();
        m_ptr = other.m_ptr;
        m_pRefCount = other.m_pRefCount;
        m_lock = other.m_lock;
        return *this;
    }

    T* operator->() const { return m_ptr; }
    T& operator*() const { return *m_ptr; }
    operator T*() const { return m_ptr; }
    bool operator!() const { return m_ptr == NULL; }
    T* get() const { return m_ptr; }
    T* Ptr() const { return m_ptr; }

    // Detach the raw pointer without decrementing (transfers ownership
    // out of the smart pointer).
    T* detach() {
        T* tmp = m_ptr;
        m_ptr = NULL;
        return tmp;
    }

private:
    // Drop one reference; the LAST owner frees ptr + control block.
    // Correct even for a default-constructed (ptr==NULL, count==0)
    // instance: --0 == -1 != 0, so the control block survives until the
    // real owners are done — the block is freed when count drops to 0
    // from 1.
    void release() {
        if (!m_pRefCount) return; // moved-from state
        mutex_lock(m_lock);
        const bool last = (--(*m_pRefCount) == 0);
        mutex_unlock(m_lock);
        if (last) {
            if (m_ptr) delete m_ptr;
            delete m_pRefCount;
            mutex_destroy(m_lock);
            delete m_lock;
        }
        m_ptr = NULL;
        m_pRefCount = NULL;
        m_lock = NULL;
    }

    T* m_ptr;
    int* m_pRefCount;
    mutex_t* m_lock;
};
