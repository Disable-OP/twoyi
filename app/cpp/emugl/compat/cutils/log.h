#pragma once
#include <android/log.h>
#define ALOGE(...) __android_log_print(ANDROID_LOG_ERROR,   "emugl", __VA_ARGS__)
#define ALOGW(...) __android_log_print(ANDROID_LOG_WARN,    "emugl", __VA_ARGS__)
#define ALOGI(...) __android_log_print(ANDROID_LOG_INFO,    "emugl", __VA_ARGS__)
#define ALOGD(...) __android_log_print(ANDROID_LOG_DEBUG,   "emugl", __VA_ARGS__)
#define ALOGV(...) __android_log_print(ANDROID_LOG_VERBOSE, "emugl", __VA_ARGS__)
