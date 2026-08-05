#pragma once
#include <pthread.h>
typedef pthread_t thread_id_t;
static inline thread_id_t getThreadId() { return pthread_self(); }
