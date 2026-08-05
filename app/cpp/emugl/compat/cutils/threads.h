#ifndef _TWOYI_CUTILS_THREADS_H
#define _TWOYI_CUTILS_THREADS_H
/*
 * Compatibility shim for the Android platform-private <cutils/threads.h>.
 *
 * The AOSP emugl sources (ThreadInfo.cpp) use the thread_store_t API:
 *
 *     thread_store_t s_tls = THREAD_STORE_INITIALIZER;
 *     thread_store_get(&s_tls);
 *     thread_store_set(&s_tls, value, destructor);
 *
 * This header provides that API on top of pthread keys.  The
 * implementation lives in compat/compat.cpp because pthread_key_create
 * can only run once per key.
 */
#include <pthread.h>

typedef pthread_t thread_id_t;

static inline thread_id_t getThreadId() { return pthread_self(); }

/* thread_store_t — a pthread-key wrapper that can live in static storage. */
typedef void (*thread_store_destruct_t)(void *);

typedef struct {
    int                inited;   /* 0 until pthread_key_create succeeds */
    pthread_key_t      key;
} thread_store_t;

#define THREAD_STORE_INITIALIZER { 0, 0 }

#ifdef __cplusplus
extern "C" {
#endif

void *thread_store_get(thread_store_t *store);
void  thread_store_set(thread_store_t *store, void *value,
                       thread_store_destruct_t destruct);

#ifdef __cplusplus
}
#endif

#endif /* _TWOYI_CUTILS_THREADS_H */
