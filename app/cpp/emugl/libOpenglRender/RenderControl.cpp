/*
* Copyright (C) 2011 The Android Open Source Project
*
* Licensed under the Apache License, Version 2.0 (the "License");
* you may not use this file except in compliance with the License.
* You may obtain a copy of the License at
*
* http://www.apache.org/licenses/LICENSE-2.0
*
* Unless required by applicable law or agreed to in writing, software
* distributed under the License is distributed on an "AS IS" BASIS,
* WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
* See the License for the specific language governing permissions and
* limitations under the License.
*/
#include "renderControl_dec.h"
#include "FrameBuffer.h"
#include "FBConfig.h"
#include "EGLDispatch.h"
#include "GLDispatch.h"
#include "GL2Dispatch.h"
#include "ThreadInfo.h"

#include <string>

// 6-Z339 observer — logcat-visible regardless of which ERR variant the
// include chain resolves to (same rationale as RenderServer's 6-Z336 macro).
#if defined(__ANDROID__)
#include <android/log.h>
#define RLOG_6Z339(...) __android_log_print(ANDROID_LOG_ERROR, "TWOYI_RENDERER", __VA_ARGS__)
#else
#define RLOG_6Z339(...) fprintf(stderr, __VA_ARGS__)
#endif

// 6-Z339: backing store for the rcGetGLString reply when the GLES
// max-version token is appended (GL_EXTENSIONS only). Thread-safety: the
// renderControl stream is served per RenderThread and the string is
// consumed before the next call on the same thread — but distinct
// threads share this static, so a per-call copy into the caller's buffer
// happens before any other thread mutates it (strcpy below runs while we
// hold no lock; the mutation window is assign+append immediately before
// the strcpy — acceptable for the single-threaded per-stream protocol).
static std::string s_6z339_glStringBuf;
static std::string s_6z340_hostExt;

static const GLint rendererVersion = 1;

static GLint rcGetRendererVersion()
{
    return rendererVersion;
}

static EGLint rcGetEGLVersion(EGLint* major, EGLint* minor)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return EGL_FALSE;
    }
    *major = (EGLint)fb->getCaps().eglMajor;
    *minor = (EGLint)fb->getCaps().eglMinor;

    return EGL_TRUE;
}

static EGLint rcQueryEGLString(EGLenum name, void* buffer, EGLint bufferSize)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return 0;
    }

    const char *str = s_egl.eglQueryString(fb->getDisplay(), name);
    if (!str) {
        return 0;
    }

    int len = strlen(str) + 1;
    if (!buffer || len > bufferSize) {
        return -len;
    }

    strcpy((char *)buffer, str);
    return len;
}

static EGLint rcGetGLString(EGLenum name, void* buffer, EGLint bufferSize)
{
    RenderThreadInfo *tInfo = getRenderThreadInfo();
    if (!tInfo || !tInfo->currContext.Ptr()) {
        return 0;
    }

    const char *str = NULL;
#ifdef WITH_GLES2
    if (tInfo->currContext->isGL2()) {
        str = (const char *)s_gl2.glGetString(name);
    }
    else {
#endif
        str = (const char *)s_gl.glGetString(name);
#ifdef WITH_GLES2
    }
#endif

    if (!str) {
        return 0;
    }

    // 6-Z339 (rn288 decode): the A11 goldfish client negotiates the max
    // GLES version by looking for the ANDROID_EMU_gles_max_version_*
    // tokens in GL_EXTENSIONS (HostConnection::queryAndSetGLESMaxVersion).
    // With NO token it warns ("Unrecognized GLES max version string in
    // extensions: " — the rn288 kmsg) and falls back to GLES_MAX_VERSION_2,
    // and then every explicit ES3 request (A11 SF: renderableType &
    // EGL_OPENGL_ES3_BIT → contextClientVersion=3) dies at the client's
    // own ES3 gate with EGL_BAD_CONFIG BEFORE the host is ever asked →
    // surfaceflinger SIGABRT crash-loop (265 deaths, rung 7).
    // The device the guest sees must be exactly the GPU the host can
    // serve: this emugl port is ES2-class (GL2 decoder + ES2 dispatch),
    // so advertise the ES2 token explicitly (the FBConfig RENDERABLE_TYPE
    // mask makes the config table agree). The client then keeps SF on an
    // ES2 context — the path this renderer was built and validated for.
    // 6-Z340 (rn289 decode): the client's HostConnection::init queries
    // GL_EXTENSIONS BEFORE any context is current — the proxy path below
    // early-returns 0 there (rn289: the 6-Z339 token never reached the
    // client; the reply stayed empty). Serve the token standalone for
    // pre-context queries (it is the ONLY thing
    // queryAndSetGLESMaxVersion parses at connection init); once any
    // context is current the full host string is cached below and served
    // with the token appended.
    if (name == GL_EXTENSIONS) {
        static const char kGlesMaxVersionToken[] = "ANDROID_EMU_gles_max_version_2";
        if (str) {
            s_6z340_hostExt.assign(str);
        }
        if (s_6z340_hostExt.empty()) {
            s_6z339_glStringBuf.assign(kGlesMaxVersionToken);
        } else {
            s_6z339_glStringBuf.assign(s_6z340_hostExt);
            s_6z339_glStringBuf.append(" ");
            s_6z339_glStringBuf.append(kGlesMaxVersionToken);
        }
        str = s_6z339_glStringBuf.c_str();
    }

    int len = strlen(str) + 1;
    if (!buffer || len > bufferSize) {
        return -len;
    }

    strcpy((char *)buffer, str);
    return len;
}

static EGLint rcGetNumConfigs(uint32_t* numAttribs)
{
    if (numAttribs) {
        *numAttribs = FBConfig::getNumAttribs();
    }
    return FBConfig::getNumConfigs();
}

static EGLint rcGetConfigs(uint32_t bufSize, GLuint* buffer)
{
    int configSize = FBConfig::getNumAttribs();
    int nConfigs = FBConfig::getNumConfigs();
    uint32_t neededSize = (nConfigs + 1) * configSize * sizeof(GLuint);
    if (!buffer || bufSize < neededSize) {
        return -neededSize;
    }
    FBConfig::packConfigsInfo(buffer);
    return nConfigs;
}

static EGLint rcChooseConfig(EGLint *attribs, uint32_t attribs_size, uint32_t *configs, uint32_t configs_size)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return 0;
    }

    return FBConfig::chooseConfig(fb, attribs, configs, configs_size);
}

static EGLint rcGetFBParam(EGLint param)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return 0;
    }

    EGLint ret = 0;

    switch(param) {
        case FB_WIDTH:
            ret = fb->getWidth();
            break;
        case FB_HEIGHT:
            ret = fb->getHeight();
            break;
        case FB_XDPI:
            ret = 72; // XXX: should be implemented
            break;
        case FB_YDPI:
            ret = 72; // XXX: should be implemented
            break;
        case FB_FPS:
            ret = 60;
            break;
        case FB_MIN_SWAP_INTERVAL:
            ret = 1; // XXX: should be implemented
            break;
        case FB_MAX_SWAP_INTERVAL:
            ret = 1; // XXX: should be implemented
            break;
        default:
            break;
    }

    return ret;
}

static uint32_t rcCreateContext(uint32_t config,
                                uint32_t share, uint32_t glVersion)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return 0;
    }

    // 6-Z339 instrument: the rn289 decode reads the guest's actual
    // context-version request + the host outcome from these lines.
    static unsigned s_6z339_ctx = 0;
    if (s_6z339_ctx < 60) {
        s_6z339_ctx++;
        RLOG_6Z339("6-Z339 rcCreateContext config=%u share=%u glVersion=%u", config, share, glVersion);
    }

    // 6-Z340: the A11 goldfish client IGNORES the app's EGL_CONTEXT_CLIENT_
    // VERSION attrib (its eglCreateContext switch only knows the KHR names,
    // the default case ALOGVs and falls through — GLESRenderEngine sends
    // CLIENT_VERSION, so wantedMajorVersion stays false) and always sends
    // rcMajorVersion=1 for that attrib style. The old "glVersion == 2" host
    // mapping therefore handed EVERY A11 client an ES1 context — SF then
    // read GL_VERSION 1.x and LOG_ALWAYS_FATAL("SurfaceFlinger requires
    // OpenGL ES 2.0 minimum to run."). The client cannot be trusted as a
    // version signal: this renderer IS an ES2-class host (GL2 decoder +
    // ES2 dispatch, the masked config table advertises exactly this), so
    // serve ES2 unconditionally. (Legacy A8-era ES1 clients sent the same
    // "1" — their fixed-function path is not part of the A11 mission; the
    // legacy distinction is recorded here honestly: glVersion is now
    // informational only.)
    HandleType ret = fb->createRenderContext(config, share, true);
    if (!ret && s_6z339_ctx < 60) {
        RLOG_6Z339("6-Z339 rcCreateContext FAILED (config=%u glVersion=%u) — FBConfig::get(%u)=%s or host eglCreateContext rejected",
                   config, glVersion, config, (int)config < FBConfig::getNumConfigs() ? "ok" : "OUT-OF-RANGE");
    }
    return ret;
}

static void rcDestroyContext(uint32_t context)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return;
    }

    fb->DestroyRenderContext(context);
}

static uint32_t rcCreateWindowSurface(uint32_t config,
                                      uint32_t width, uint32_t height)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return 0;
    }

    return fb->createWindowSurface(config, width, height);
}

static void rcDestroyWindowSurface(uint32_t windowSurface)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return;
    }

    fb->DestroyWindowSurface( windowSurface );
}

static uint32_t rcCreateColorBuffer(uint32_t width,
                                    uint32_t height, GLenum internalFormat)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return 0;
    }

    return fb->createColorBuffer(width, height, internalFormat);
}

static void rcOpenColorBuffer(uint32_t colorbuffer)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return;
    }
    fb->openColorBuffer( colorbuffer );
}

static void rcCloseColorBuffer(uint32_t colorbuffer)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return;
    }
    fb->closeColorBuffer( colorbuffer );
}

static int rcFlushWindowColorBuffer(uint32_t windowSurface)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return -1;
    }
    fb->flushWindowSurfaceColorBuffer(windowSurface);
    return 0;
}

static void rcSetWindowColorBuffer(uint32_t windowSurface,
                                   uint32_t colorBuffer)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return;
    }
    fb->setWindowSurfaceColorBuffer(windowSurface, colorBuffer);
}

static EGLint rcMakeCurrent(uint32_t context,
                            uint32_t drawSurf, uint32_t readSurf)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return EGL_FALSE;
    }

    bool ret = fb->bindContext(context, drawSurf, readSurf);

    return (ret ? EGL_TRUE : EGL_FALSE);
}

static void rcFBPost(uint32_t colorBuffer)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return;
    }

    fb->post(colorBuffer);
}

static void rcFBSetSwapInterval(EGLint interval)
{
   // XXX: TBD - should be implemented
}

static void rcBindTexture(uint32_t colorBuffer)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return;
    }

    fb->bindColorBufferToTexture(colorBuffer);
}

static void rcBindRenderbuffer(uint32_t colorBuffer)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return;
    }

    fb->bindColorBufferToRenderbuffer(colorBuffer);
}

static EGLint rcColorBufferCacheFlush(uint32_t colorBuffer,
                                      EGLint postCount, int forRead)
{
   // XXX: TBD - should be implemented
   return 0;
}

static void rcReadColorBuffer(uint32_t colorBuffer,
                              GLint x, GLint y,
                              GLint width, GLint height,
                              GLenum format, GLenum type, void* pixels)
{
   // XXX: TBD - should be implemented
}

static int rcUpdateColorBuffer(uint32_t colorBuffer,
                                GLint x, GLint y,
                                GLint width, GLint height,
                                GLenum format, GLenum type, void* pixels)
{
    FrameBuffer *fb = FrameBuffer::getFB();
    if (!fb) {
        return -1;
    }

    fb->updateColorBuffer(colorBuffer, x, y, width, height, format, type, pixels);
    return 0;
}

void initRenderControlContext(renderControl_decoder_context_t *dec)
{
    dec->set_rcGetRendererVersion(rcGetRendererVersion);
    dec->set_rcGetEGLVersion(rcGetEGLVersion);
    dec->set_rcQueryEGLString(rcQueryEGLString);
    dec->set_rcGetGLString(rcGetGLString);
    dec->set_rcGetNumConfigs(rcGetNumConfigs);
    dec->set_rcGetConfigs(rcGetConfigs);
    dec->set_rcChooseConfig(rcChooseConfig);
    dec->set_rcGetFBParam(rcGetFBParam);
    dec->set_rcCreateContext(rcCreateContext);
    dec->set_rcDestroyContext(rcDestroyContext);
    dec->set_rcCreateWindowSurface(rcCreateWindowSurface);
    dec->set_rcDestroyWindowSurface(rcDestroyWindowSurface);
    dec->set_rcCreateColorBuffer(rcCreateColorBuffer);
    dec->set_rcOpenColorBuffer(rcOpenColorBuffer);
    dec->set_rcCloseColorBuffer(rcCloseColorBuffer);
    dec->set_rcSetWindowColorBuffer(rcSetWindowColorBuffer);
    dec->set_rcFlushWindowColorBuffer(rcFlushWindowColorBuffer);
    dec->set_rcMakeCurrent(rcMakeCurrent);
    dec->set_rcFBPost(rcFBPost);
    dec->set_rcFBSetSwapInterval(rcFBSetSwapInterval);
    dec->set_rcBindTexture(rcBindTexture);
    dec->set_rcBindRenderbuffer(rcBindRenderbuffer);
    dec->set_rcColorBufferCacheFlush(rcColorBufferCacheFlush);
    dec->set_rcReadColorBuffer(rcReadColorBuffer);
    dec->set_rcUpdateColorBuffer(rcUpdateColorBuffer);
}
