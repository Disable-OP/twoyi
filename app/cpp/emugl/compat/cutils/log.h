#ifndef _TWOYI_CUTILS_LOG_H
#define _TWOYI_CUTILS_LOG_H
/*
 * Compatibility shim for <cutils/log.h>. Routes ALOG* to Android log
 * (liblog / __android_log_print) when built for Android, otherwise to
 * printf.
 */
#include <android/log.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifndef LOG_TAG
#define LOG_TAG "emugl"
#endif

#define ALOGE(...) __android_log_print(ANDROID_LOG_ERROR,   LOG_TAG, __VA_ARGS__)
#define ALOGW(...) __android_log_print(ANDROID_LOG_WARN,    LOG_TAG, __VA_ARGS__)
#define ALOGI(...) __android_log_print(ANDROID_LOG_INFO,    LOG_TAG, __VA_ARGS__)
#define ALOGD(...) __android_log_print(ANDROID_LOG_DEBUG,   LOG_TAG, __VA_ARGS__)
#define ALOGV(...) __android_log_print(ANDROID_LOG_VERBOSE, LOG_TAG, __VA_ARGS__)

#define ALOG_ASSERT(cond, ...) do { \
    if (!(cond)) __android_log_assert(#cond, LOG_TAG, __VA_ARGS__); \
} while (0)

#ifdef __cplusplus
}
#endif

#endif /* _TWOYI_CUTILS_LOG_H */
