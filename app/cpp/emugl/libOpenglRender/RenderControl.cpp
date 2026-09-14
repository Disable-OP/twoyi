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
static std::string s_6z346_extStr;

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

    // 6-Z346 (rn295 decode — the SF wall finally named): the redroid host
    // EGL advertises EGL_KHR_no_config_context in EGL_EXTENSIONS, and the
    // A11 client PASSES IT THROUGH via rcQueryEGLString(0x3055). A11
    // GLESRenderEngine::create then takes the no-config fast path:
    //   EGLConfig config = EGL_NO_CONFIG;   // chooseEglConfig SKIPPED
    //   ... createEglContext(display, config=EGL_NO_CONFIG, ...)
    // and the goldfish client's eglCreateContext rejects EGL_NO_CONFIG in
    // VALIDATE_CONFIG (getIndexOfConfig(0) = 0xFFFFFFFF > m_numConfigs →
    // EGL_BAD_CONFIG) BEFORE any host round-trip — "RenderEngine
    // EGLContext creation failed" ×156, ZERO rcChooseConfig/context ops in
    // the whole 6-Z344 trace (the 6-Z339 mask and the 3_0 token were both
    // correct but unreachable). The REAL emulator's host EGL never
    // advertises that token, so the fast path never fires there. The
    // device the guest sees must be exactly what the goldfish protocol
    // layer can serve: strip EGL_KHR_no_config_context from the
    // EGL_EXTENSIONS reply so SF picks a real config via rcChooseConfig —
    // the path this renderer was built for.
    if (name == EGL_EXTENSIONS) {
        // 6-Z346: the A11 client's GLExtensions::init takes the no-config
        // fast path when EITHER token is present (GLExtensions.cpp:
        // hasExtension("EGL_ANDROIDX_no_config_context") ||
        // hasExtension("EGL_KHR_no_config_context")) — strip BOTH.
        static const char *kNoConfigTokens[] = {
            "EGL_KHR_no_config_context",
            "EGL_ANDROIDX_no_config_context",
            // 6-Z349 (rn299 decode): with surfaceless advertised, A11 SF
            // SKIPS the dummy pbuffer and calls eglMakeCurrent(NO_SURFACE,
            // NO_SURFACE, ctx) — the goldfish client rejects that binding
            // (EGL_BAD_MATCH) before any host round-trip ("can't make
            // dummy pbuffer current" ×76) — the same
            // the-host-leaks-a-capability-the-protocol-layer-cannot-serve
            // class as the no-config tokens.
            "EGL_KHR_surfaceless_context",
        };
        s_6z346_extStr.assign(str);
        for (const char *tok : kNoConfigTokens) {
            for (size_t pos = s_6z346_extStr.find(tok);
                 pos != std::string::npos;
                 pos = s_6z346_extStr.find(tok)) {
                size_t end = pos + strlen(tok);
                // eat one adjacent separator so the space-separated list
                // stays well-formed for both the head and tail positions.
                if (end < s_6z346_extStr.size() && s_6z346_extStr[end] == ' ') {
                    end++;
                } else if (pos > 0 && s_6z346_extStr[pos - 1] == ' ') {
                    pos--;
                }
                s_6z346_extStr.erase(pos, end - pos);
                static unsigned s_6z346_strips = 0;
                if (s_6z346_strips < 4) {
                    s_6z346_strips++;
                    RLOG_6Z339("6-Z346 stripped a no-config-context token from the EGL_EXTENSIONS reply (strip #%u)", s_6z346_strips);
                }
            }
        }
        str = s_6z346_extStr.c_str();
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
    // 6-Z343 (rn292 decode): the client parsed the token (the "Unrecognized
    // GLES max version" warning is GONE, reply_len=30) — but A11 SF still
    // died at "RenderEngine EGLContext creation failed" ×35 with ZERO
    // rcCreateContext calls: SF's ES3 attempt (a real config → the ES2
    // fallback retry is skipped by design) dies at the client's own
    // max-version gate whichever variant their build takes. The client's
    // ES3 request is now ACCEPTED (the 3_0 token) and the host's 6-Z340
    // rcCreateContext serves an ES2 context regardless of the requested
    // version — the context's GL_VERSION reports 2.0, SF's
    // parseGlesVersion picks the ES2 engine path, the masked config table
    // and the decoder agree. The whole ES-version class dies here.
    if (name == GL_EXTENSIONS) {
        RenderThreadInfo *tInfo = getRenderThreadInfo();
        const bool haveCtx = tInfo && tInfo->currContext.Ptr();
        if (haveCtx) {
            const char *live = NULL;
#ifdef WITH_GLES2
            if (tInfo->currContext->isGL2()) {
                live = (const char *)s_gl2.glGetString(name);
            }
            else {
#endif
                live = (const char *)s_gl.glGetString(name);
#ifdef WITH_GLES2
            }
#endif
            if (live) {
                s_6z340_hostExt.assign(live);
            }
        }
        static const char kGlesMaxVersionToken[] = "ANDROID_EMU_gles_max_version_3_0";
        if (s_6z340_hostExt.empty()) {
            s_6z339_glStringBuf.assign(kGlesMaxVersionToken);
        } else {
            s_6z339_glStringBuf.assign(s_6z340_hostExt);
            s_6z339_glStringBuf.append(" ");
            s_6z339_glStringBuf.append(kGlesMaxVersionToken);
        }
        static unsigned s_6z341_ext_serves = 0;
        if (s_6z341_ext_serves < 12) {
            s_6z341_ext_serves++;
            RLOG_6Z339("6-Z341 rcGetGLString(GL_EXTENSIONS) serve #%u ctx=%s host_ext=%s reply_len=%zu",
                       s_6z341_ext_serves, haveCtx ? "current" : "PRE-CONTEXT",
                       s_6z340_hostExt.empty() ? "<none-yet>" : "cached",
                       s_6z339_glStringBuf.size());
        }
        const char *str = s_6z339_glStringBuf.c_str();
        const int len = (int)strlen(str) + 1;
        if (!buffer || len > bufferSize) {
            return -len;
        }
        strcpy((char *)buffer, str);
        return len;
    }

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

    // 6-Z339/6-Z340 history: the GLES max-version negotiation originally
    // rode this proxy path; since 6-Z341 the GL_EXTENSIONS negotiation is
    // served by the dedicated pre-context-aware branch above (the client's
    // connection-init query has NO current context — the proxy path below
    // can never see it). All other GL strings keep the proxy semantics.

    int len = strlen(str) + 1;
    if (!buffer || len > bufferSize) {
        return -len;
    }

    strcpy((char *)buffer, str);
    return len;
}

static EGLint rcGetNumConfigs(uint32_t* numAttribs)
{
    static unsigned s_6z343_numcfg = 0;
    if (s_6z343_numcfg < 8) {
        s_6z343_numcfg++;
        RLOG_6Z339("6-Z343 rcGetNumConfigs -> %d configs / %d attribs",
                   FBConfig::getNumConfigs(), FBConfig::getNumAttribs());
    }
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

static FrameBuffer *fb6z343();

static EGLint rcChooseConfig(EGLint *attribs, uint32_t attribs_size, uint32_t *configs, uint32_t configs_size)
{
    static unsigned s_6z343_choose = 0;
    if (s_6z343_choose < 12) {
        s_6z343_choose++;
        // surface the RENDERABLE_TYPE request if the client sent one — the
        // rn292 decode hinges on whether the guest still asks for ES3.
        EGLint renderable = -1;
        if (attribs) {
            for (EGLint *ap = attribs; ap[0] != EGL_NONE; ap += 2) {
                if (ap[0] == EGL_RENDERABLE_TYPE) { renderable = ap[1]; break; }
            }
        }
        RLOG_6Z339("6-Z343 rcChooseConfig attribs_size=%u renderable=0x%x configs_size=%u",
                   attribs_size, (unsigned)renderable, configs_size);
    }
    EGLint rc6z343 = FBConfig::chooseConfig(fb6z343(), attribs, configs, configs_size);
    if (s_6z343_choose <= 12) {
        RLOG_6Z339("6-Z343 rcChooseConfig -> %d matches (first=%u)",
                   rc6z343, rc6z343 > 0 ? configs[0] : 0);
    }
    return rc6z343;
}

static FrameBuffer *fb6z343()
{
    return FrameBuffer::getFB();
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

// ── 6-Z344: renderControl op trampolines ────────────────────────────
// rn293's decode stalled on a contradiction (SF dies between "GLES
// Backend" and the context while the config endpoints never log a call)
// that cannot be resolved from the guest side: the vendored A11 client's
// exact gate is invisible. These log-then-call wrappers cover the
// negotiation + surface-lifecycle ops that had no instrument yet
// (bounded globally: the first 160 op invocations of the boot name the
// whole negotiation + the first SF eras), so the next decode reads the
// client's ACTUAL op sequence instead of inferring it.
static unsigned s_6z344_rc_ops = 0;
static unsigned s_6z344_rc_ops_logged = 0;
// 6-Z345 (rn294 decode): the 160-op budget was consumed by ~27 gralloc-class
// client inits (6 ops each) BEFORE surfaceflinger's first era — SF's ops
// never fit in the trace. 4000 ops + tail sampling (every 25th after the
// first 200) covers the whole boot within a bounded count.
inline void rc_op_trace_6z344(const char* name) {
    s_6z344_rc_ops++;
    if (s_6z344_rc_ops <= 200 || (s_6z344_rc_ops % 25) == 0) {
        if (s_6z344_rc_ops_logged < 4000) {
            s_6z344_rc_ops_logged++;
            RLOG_6Z339("6-Z344 rc-op #%u %s", s_6z344_rc_ops, name);
        }
    }
}

static EGLint rcQueryEGLString_6z344(EGLenum name, void* buffer, EGLint bufferSize)
{
    rc_op_trace_6z344("rcQueryEGLString");
    return rcQueryEGLString(name, buffer, bufferSize);
}

static EGLint rcGetConfigs_6z344(uint32_t bufSize, GLuint* buffer)
{
    rc_op_trace_6z344("rcGetConfigs");
    return rcGetConfigs(bufSize, buffer);
}

static uint32_t rcCreateWindowSurface_6z344(uint32_t config, uint32_t width, uint32_t height)
{
    rc_op_trace_6z344("rcCreateWindowSurface");
    return rcCreateWindowSurface(config, width, height);
}

static int rcMakeCurrent_6z344(uint32_t context, uint32_t drawSurf, uint32_t readSurf)
{
    rc_op_trace_6z344("rcMakeCurrent");
    return rcMakeCurrent(context, drawSurf, readSurf);
}

static int rcGetEGLVersion_6z344(EGLint* major, EGLint* minor)
{
    rc_op_trace_6z344("rcGetEGLVersion");
    return rcGetEGLVersion(major, minor);
}

void initRenderControlContext(renderControl_decoder_context_t *dec)
{
    dec->set_rcGetRendererVersion(rcGetRendererVersion);
    dec->set_rcGetEGLVersion(rcGetEGLVersion_6z344);
    dec->set_rcQueryEGLString(rcQueryEGLString_6z344);
    dec->set_rcGetGLString(rcGetGLString);
    dec->set_rcGetNumConfigs(rcGetNumConfigs);
    dec->set_rcGetConfigs(rcGetConfigs_6z344);
    dec->set_rcChooseConfig(rcChooseConfig);
    dec->set_rcGetFBParam(rcGetFBParam);
    dec->set_rcCreateContext(rcCreateContext);
    dec->set_rcDestroyContext(rcDestroyContext);
    dec->set_rcCreateWindowSurface(rcCreateWindowSurface_6z344);
    dec->set_rcDestroyWindowSurface(rcDestroyWindowSurface);
    dec->set_rcCreateColorBuffer(rcCreateColorBuffer);
    dec->set_rcOpenColorBuffer(rcOpenColorBuffer);
    dec->set_rcCloseColorBuffer(rcCloseColorBuffer);
    dec->set_rcSetWindowColorBuffer(rcSetWindowColorBuffer);
    dec->set_rcFlushWindowColorBuffer(rcFlushWindowColorBuffer);
    dec->set_rcMakeCurrent(rcMakeCurrent_6z344);
    dec->set_rcFBPost(rcFBPost);
    dec->set_rcFBSetSwapInterval(rcFBSetSwapInterval);
    dec->set_rcBindTexture(rcBindTexture);
    dec->set_rcBindRenderbuffer(rcBindRenderbuffer);
    dec->set_rcColorBufferCacheFlush(rcColorBufferCacheFlush);
    dec->set_rcReadColorBuffer(rcReadColorBuffer);
    dec->set_rcUpdateColorBuffer(rcUpdateColorBuffer);
}
