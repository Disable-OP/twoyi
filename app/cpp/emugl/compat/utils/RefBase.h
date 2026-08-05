#pragma once
#include <atomic>
namespace android {
class RefBase {
public:
    RefBase() : mRefCnt(0) {}
    virtual ~RefBase() {}
    void incStrong(const void* id) const { mRefCnt.fetch_add(1, std::memory_order_relaxed); }
    void decStrong(const void* id) const { if (mRefCnt.fetch_sub(1, std::memory_order_acq_rel) == 1) delete this; }
private:
    mutable std::atomic<int> mRefCnt;
};
template <typename T>
class sp {
public:
    sp() : mPtr(nullptr) {}
    sp(T* p) : mPtr(p) { if (p) p->incStrong(nullptr); }
    sp(const sp& other) : mPtr(other.mPtr) { if (mPtr) mPtr->incStrong(nullptr); }
    ~sp() { if (mPtr) mPtr->decStrong(nullptr); }
    sp& operator=(T* p) { if (p) p->incStrong(nullptr); if (mPtr) mPtr->decStrong(nullptr); mPtr = p; return *this; }
    sp& operator=(const sp& other) { if (other.mPtr) other.mPtr->incStrong(nullptr); if (mPtr) mPtr->decStrong(nullptr); mPtr = other.mPtr; return *this; }
    T& operator*() const { return *mPtr; }
    T* operator->() const { return mPtr; }
    T* get() const { return mPtr; }
    operator bool() const { return mPtr != nullptr; }
private:
    T* mPtr;
};
}
