#ifndef _TWOYI_UTILS_THREADS_H
#define _TWOYI_UTILS_THREADS_H
/*
 * Compatibility shim for the Android platform-private <utils/threads.h>.
 * The emugl sources use android::Mutex, android::Mutex::Autolock and
 * android::AutoMutex. This header provides all three on top of
 * pthread_mutex_t.
 */
#include <pthread.h>

namespace android {

class Mutex {
public:
    enum { PRIVATE = 0, SHARED = 1 };

    Mutex()  { pthread_mutex_init(&mMutex, NULL); }
    Mutex(const char *) { pthread_mutex_init(&mMutex, NULL); }
    Mutex(int type, const char * = NULL) {
        if (type == SHARED) {
            pthread_mutexattr_t attr;
            pthread_mutexattr_init(&attr);
            pthread_mutexattr_setpshared(&attr, PTHREAD_PROCESS_SHARED);
            pthread_mutex_init(&mMutex, &attr);
            pthread_mutexattr_destroy(&attr);
        } else {
            pthread_mutex_init(&mMutex, NULL);
        }
    }
    ~Mutex() { pthread_mutex_destroy(&mMutex); }

    int lock()   { return pthread_mutex_lock(&mMutex); }
    int unlock() { return pthread_mutex_unlock(&mMutex); }
    int tryLock() { return pthread_mutex_trylock(&mMutex); }

    class Autolock {
    public:
        inline explicit Autolock(Mutex &m) : mLock(m) { mLock.lock(); }
        inline explicit Autolock(Mutex *m) : mLock(*m) { mLock.lock(); }
        inline ~Autolock() { mLock.unlock(); }
    private:
        Mutex &mLock;
    };

private:
    friend class Condition;
    pthread_mutex_t mMutex;
    Mutex(const Mutex &);
    Mutex &operator=(const Mutex &);
};

typedef Mutex::Autolock AutoMutex;

}  // namespace android

#endif /* _TWOYI_UTILS_THREADS_H */
