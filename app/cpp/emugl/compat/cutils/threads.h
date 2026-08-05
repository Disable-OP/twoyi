#ifndef _TWOYI_CUTILS_THREADS_H
#define _TWOYI_CUTILS_THREADS_H
/*
 * Compatibility shim for the Android platform-private <cutils/threads.h>
 * used by the AOSP emugl sources. Implemented on top of pthreads.
 *
 * The real header (from system/core/include/cutils/threads.h) exposes a
 * thread-local-storage abstraction (thread_store_t) and a thin mutex
 * abstraction (mutex_t). The emugl code only uses:
 *   - thread_store_t / THREAD_STORE_INITIALIZER / thread_store_get / thread_store_set
 *   - mutex_t / mutex_init / mutex_lock / mutex_unlock / mutex_destroy
 */
#include <pthread.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- thread_store_t (TLS) ---- */
typedef void (*thread_store_destruct_t)(void *value);

typedef struct {
    pthread_key_t key;
    int inited;
} thread_store_t;

#define THREAD_STORE_INITIALIZER ((thread_store_t){0, 0})

void *thread_store_get(thread_store_t *store);
void  thread_store_set(thread_store_t *store, void *value,
                       thread_store_destruct_t destruct);

/* ---- mutex_t ---- */
typedef pthread_mutex_t mutex_t;

static __inline__ int mutex_init(mutex_t *m) {
    return pthread_mutex_init(m, NULL);
}
static __inline__ int mutex_lock(mutex_t *m) {
    return pthread_mutex_lock(m);
}
static __inline__ int mutex_unlock(mutex_t *m) {
    return pthread_mutex_unlock(m);
}
static __inline__ int mutex_destroy(mutex_t *m) {
    return pthread_mutex_destroy(m);
}

#ifdef __cplusplus
}
#endif

#endif /* _TWOYI_CUTILS_THREADS_H */
