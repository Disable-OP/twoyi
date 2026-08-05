// render_api_platform_types.h — Android version (no X11)
// On Android, FBNativeWindowType is just void* (ANativeWindow)
#pragma once
#include <stdint.h>
typedef void* FBNativeWindowType;

// Logging macros
#include <android/log.h>
#define ERR(...) __android_log_print(ANDROID_LOG_ERROR, 'emugl', __VA_ARGS__)
#define INFO(...) __android_log_print(ANDROID_LOG_INFO, 'emugl', __VA_ARGS__)
#define ERRLOG(...) ERR(__VA_ARGS__)
