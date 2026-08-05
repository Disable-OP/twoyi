#ifndef _TWOYI_UTILS_REF_BASE_H
#define _TWOYI_UTILS_REF_BASE_H
/*
 * Compatibility shim for <utils/RefBase.h>. The emugl sources use
 * RefBase's incStrong / decStrong methods, but they don't actually
 * rely on the weak-reference machinery. We provide the minimum API
 * surface: a RefBase class with incStrong()/decStrong() that drive an
 * atomic refcount, and a virtual destructor that frees the object
 * when the count drops to zero.
 */
#include <atomic>
#include <cstddef>

namespace android {

class RefBase {
public:
    RefBase() : mRefCount(0) {}
    virtual ~RefBase() {}

    void incStrong(const void * /*id*/) const {
        mRefCount.fetch_add(1, std::memory_order_relaxed);
    }
    void decStrong(const void * /*id*/) const {
        if (mRefCount.fetch_sub(1, std::memory_order_acq_rel) == 1) {
            delete this;
        }
    }

    int32_t getStrongCount() const {
        return mRefCount.load(std::memory_order_relaxed);
    }

private:
    mutable std::atomic<int32_t> mRefCount;
};

}  // namespace android

#endif /* _TWOYI_UTILS_REF_BASE_H */
