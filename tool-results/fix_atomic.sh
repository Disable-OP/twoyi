#!/bin/bash
# Fix the compat/cutils/atomic.h to use __atomic_* builtins (works in C and C++)
set -e
COMPAT=/tmp/build_opengl/compat

cat > $COMPAT/cutils/atomic.h <<'EOF'
#ifndef _CUTILS_ATOMIC_H
#define _CUTILS_ATOMIC_H
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Use the GCC/Clang __atomic_* builtins, which work on plain volatile
 * integral types in both C and C++ (unlike the C++11 std::atomic templates). */
static inline int32_t android_atomic_inc(volatile int32_t* addr) {
    return __atomic_fetch_add(addr, 1, __ATOMIC_SEQ_CST);
}
static inline int32_t android_atomic_dec(volatile int32_t* addr) {
    return __atomic_fetch_sub(addr, 1, __ATOMIC_SEQ_CST);
}
static inline int32_t android_atomic_acquire_load(volatile const int32_t* addr) {
    int32_t v;
    __atomic_load(addr, &v, __ATOMIC_ACQUIRE);
    return v;
}
static inline void android_atomic_release_store(int32_t val, volatile int32_t* addr) {
    __atomic_store(addr, &val, __ATOMIC_RELEASE);
}
static inline int android_atomic_acquire_cas(int32_t oldval, int32_t newval, volatile int32_t* addr) {
    int32_t expected = oldval;
    return !__atomic_compare_exchange_n(addr, &expected, newval, 1,
                                        __ATOMIC_ACQUIRE, __ATOMIC_ACQUIRE);
}
static inline int android_atomic_release_cas(int32_t oldval, int32_t newval, volatile int32_t* addr) {
    int32_t expected = oldval;
    return !__atomic_compare_exchange_n(addr, &expected, newval, 1,
                                        __ATOMIC_RELEASE, __ATOMIC_RELAXED);
}

#ifdef __cplusplus
}
#endif

#endif
EOF
echo "=== atomic.h fixed ==="
