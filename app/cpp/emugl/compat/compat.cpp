/*
 * compat.cpp — non-inline implementations for the twoyi emugl compat
 * shim layer. Only the thread_store_t helpers need a real translation
 * unit (pthread_key_create can only be called once per key, so it
 * must be lazy-initialized inside a .cpp, not a header).
 */
#include "cutils/threads.h"
#include <pthread.h>

extern "C" {

void *thread_store_get(thread_store_t *store)
{
    if (!store->inited) {
        return NULL;
    }
    return pthread_getspecific(store->key);
}

void thread_store_set(thread_store_t *store, void *value,
                      thread_store_destruct_t destruct)
{
    if (!store->inited) {
        /* pthread_key_create only succeeds once; subsequent calls are
         * a no-op because we guard with the inited flag. */
        if (pthread_key_create(&store->key, destruct) != 0) {
            return;
        }
        store->inited = 1;
    }
    pthread_setspecific(store->key, value);
}

}  /* extern "C" */
