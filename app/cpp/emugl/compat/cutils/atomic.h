#ifndef _TWOYI_CUTILS_ATOMIC_H
#define _TWOYI_CUTILS_ATOMIC_H
/*
 * Compatibility shim for <cutils/atomic.h>.
 * Implemented with GCC/Clang __atomic_* builtins.
 */
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

static __inline__ int32_t android_atomic_inc(volatile int32_t *addr) {
    return __atomic_fetch_add(addr, 1, __ATOMIC_SEQ_CST);
}
static __inline__ int32_t android_atomic_dec(volatile int32_t *addr) {
    return __atomic_fetch_sub(addr, 1, __ATOMIC_SEQ_CST);
}
static __inline__ int32_t android_atomic_acquire_load(volatile const int32_t *addr) {
    int32_t v;
    __atomic_load(addr, &v, __ATOMIC_ACQUIRE);
    return v;
}
static __inline__ int32_t android_atomic_release_store(volatile int32_t *addr, int32_t value) {
    __atomic_store(addr, &value, __ATOMIC_RELEASE);
    return value;
}
static __inline__ int android_atomic_acquire_cas(int32_t oldval, int32_t newval,
                                                  volatile int32_t *addr) {
    int32_t expected = oldval;
    return __atomic_compare_exchange_n(addr, &expected, newval,
                                       0 /* weak */,
                                       __ATOMIC_ACQUIRE,
                                       __ATOMIC_RELAXED) ? 0 : 1;
}

#ifdef __cplusplus
}
#endif

#endif /* _TWOYI_CUTILS_ATOMIC_H */
