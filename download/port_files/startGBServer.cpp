/*
 * startGBServer.cpp — Twoyi Graphics Buffer server entry point.
 *
 * startGBServer() is the twoyi-specific API exported by libOpenglRender.so
 * that boots the GraphicBuffer /dev/gb-equivalent proxy server.
 *
 * Disassembly of the legacy blob (libOpenglRender.so @ 0x057ad4, 220 B,
 * symbol _Z13startGBServerv) shows the following sequence:
 *
 *   1. x19 = GraphicBuffer::create()         // allocate + open opengles3
 *   2. x21 = dlopen_ex("libandroid.so", 0)
 *   3. logger(x21, "libandroid.so handle: %p")
 *   4. x20 = dlsym_ex(x21, "AHardwareBuffer_recvHandleFromUnixSocket")
 *   5. x21 = dlsym_ex(x21,
 *         "_ZN7android38AHardwareBuffer_to_ANativeWindowBufferEP15AHardwareBuffer")
 *   6. logger(x21, "sym1: %p")               // (legacy logs sym2 here)
 *   7. if (x20 == NULL) {
 *        logger(0, "Can not found symbol!");
 *        return 0;
 *      }
 *   8. g_recvHandle          = x20;           // .bss @ 0x10bcc0
 *      g_toNativeWindowBuffer = x21;          // .bss @ 0x10bcc8
 *   9. logger(x19, "GraphicBuffer_unflatten: %p, GraphicBuffer_create: %p")
 *  10. GraphicBuffer->thread_start()          // emugl::Thread::start
 *  11. return 1
 *
 * The two cached function-pointer globals (g_recvHandle /
 * g_toNativeWindowBuffer) are also referenced from the GraphicBuffer
 * thread's Main() loop in the legacy; we mirror that here.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include "GraphicBuffer.h"

#include <cutils/log.h>
#include <dlfcn.h>
#include <stdlib.h>

// Defined in dl_ex.cpp — these are the twoyi-specific wrappers that
// know how to find symbols in libandroid.so on Android 7+.
extern "C" {
void* dlopen_ex(const char* filename, int flag);
void* dlsym_ex(void* handle, const char* symbol);
}

// ---------------------------------------------------------------------------
// Module-level cache of the looked-up libandroid.so symbols.  In the
// legacy blob these live at .bss 0x10bcc0 / 0x10bcc8; here we keep them
// as plain file-statics.  The GraphicBuffer thread reads them via the
// GraphicBuffer::setRecvHandle / setToNativeWindowBuffer setters.
// ---------------------------------------------------------------------------
static AHardwareBuffer_recvHandleFromUnixSocketFn g_recvHandle = NULL;
static AHardwareBuffer_to_ANativeWindowBufferFn   g_toNativeWindowBuffer = NULL;

// Mangled C++ name for android::AHardwareBuffer_to_ANativeWindowBuffer
// — exact same string the legacy dlsym_ex call passes (see .rodata @
// 0xdd382).
static const char kSymToANativeWindowBuffer[] =
    "_ZN7android38AHardwareBuffer_to_ANativeWindowBufferEP15AHardwareBuffer";

// Singleton — startGBServer may be called more than once by mistake; we
// only spin up one GraphicBuffer server per process.
static GraphicBuffer* g_gbServer = NULL;

extern "C" {

// startGBServer — boot the GraphicBuffer proxy.  Returns 1 on success,
// 0 on failure (matching the legacy's return convention).
int startGBServer()
{
    if (g_gbServer) {
        // Already started.
        return 1;
    }

    // 1. Allocate the GraphicBuffer server and open the opengles3 socket.
    GraphicBuffer* gb = GraphicBuffer::create();
    if (!gb) {
        ALOGE("startGBServer: GraphicBuffer::create failed");
        return 0;
    }

    // 2. dlopen libandroid.so (RTLD_LAZY=0 — matches the legacy's
    //    `mov w1, wzr` before the dlopen_ex call).
    void* handle = dlopen_ex("libandroid.so", 0);
    ALOGI("startGBServer: libandroid.so handle: %p", handle);
    if (!handle) {
        ALOGE("startGBServer: dlopen_ex(libandroid.so) failed");
        delete gb;
        return 0;
    }

    // 4. Look up AHardwareBuffer_recvHandleFromUnixSocket (public NDK API).
    void* sym1 = dlsym_ex(handle, "AHardwareBuffer_recvHandleFromUnixSocket");

    // 5. Look up android::AHardwareBuffer_to_ANativeWindowBuffer (internal
    //    libandroid.so symbol; only resolvable via the custom ELF .dynsym
    //    walker in dlsym_ex on Android 7+).
    void* sym2 = dlsym_ex(handle, kSymToANativeWindowBuffer);

    ALOGI("startGBServer: sym1: %p", sym1);

    // 7. If recvHandleFromUnixSocket is missing, give up (legacy logs
    //    "Can not found symbol!" here).
    if (!sym1) {
        ALOGE("startGBServer: Can not found symbol!");
        delete gb;
        return 0;
    }

    // 8. Cache the pointers and inject them into the GraphicBuffer.
    g_recvHandle          = (AHardwareBuffer_recvHandleFromUnixSocketFn)sym1;
    g_toNativeWindowBuffer =
        (AHardwareBuffer_to_ANativeWindowBufferFn)sym2;
    gb->setRecvHandle(g_recvHandle);
    gb->setToNativeWindowBuffer(g_toNativeWindowBuffer);

    ALOGI("startGBServer: GraphicBuffer_unflatten: %p, GraphicBuffer_create: %p",
          (void*)g_toNativeWindowBuffer, (void*)gb);

    // 10. Start the accept loop on a background thread.
    if (!gb->start()) {
        ALOGE("startGBServer: GraphicBuffer thread failed to start");
        delete gb;
        return 0;
    }

    g_gbServer = gb;
    return 1;
}

} // extern "C"
