/*
 * render_api_platform_types.h — patched for twoyi / Android.
 *
 * Original AOSP file: emulator/opengl/host/include/libOpenglRender/render_api_platform_types.h
 * Patch: add an __ANDROID__ branch using void* (matching the __APPLE__
 * branch). On Android the EGLNativeWindowType is ANativeWindow* and
 * FBNativeWindowType carries it as an opaque pointer — no X11.
 */
#ifndef _RENDER_API_PLATFORM_TYPES_H
#define _RENDER_API_PLATFORM_TYPES_H

#if defined(_WIN32) || defined(__VC32__) && !defined(__CYGWIN__) && !defined(__SCITECH_SNAP__)
#include <windows.h>

typedef HDC     FBNativeDisplayType;
typedef HWND    FBNativeWindowType;

#elif defined(__ANDROID__) || defined(__APPLE__)

/* Android and Apple both carry the native window as an opaque
 * pointer. On Android this is ANativeWindow*. */
typedef void*   FBNativeDisplayType;
typedef void*   FBNativeWindowType;

#elif defined(__linux__)

#include <X11/Xlib.h>
#include <X11/Xutil.h>

typedef Window   FBNativeWindowType;

#else
#warning "Unsupported platform"
#endif

#endif /* of _RENDER_API_PLATFORM_TYPES_H */
