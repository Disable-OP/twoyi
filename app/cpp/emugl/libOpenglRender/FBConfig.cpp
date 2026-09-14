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
#include "FBConfig.h"
#include "FrameBuffer.h"
#include "EGLDispatch.h"
#include <stdio.h>

// 6-Z347: logcat-visible diagnostics (same rationale as RenderControl's
// 6-Z339 macro — ErrorLog.h's fprintf leg loses the evidence).
#if defined(__ANDROID__)
#include <android/log.h>
#define RLOG_6Z339(...) __android_log_print(ANDROID_LOG_ERROR, "TWOYI_RENDERER", __VA_ARGS__)
#else
#define RLOG_6Z339(...) fprintf(stderr, __VA_ARGS__)
#endif

FBConfig **FBConfig::s_fbConfigs = NULL;
int FBConfig::s_numConfigs = 0;

const GLuint FBConfig::s_configAttribs[] = {
    EGL_DEPTH_SIZE,     // must be first - see getDepthSize()
    EGL_STENCIL_SIZE,   // must be second - see getStencilSize()
    EGL_RENDERABLE_TYPE,// must be third - see getRenderableType()
    EGL_SURFACE_TYPE,   // must be fourth - see getSurfaceType()
    EGL_CONFIG_ID,      // must be fifth  - see chooseConfig()
    EGL_BUFFER_SIZE,
    EGL_ALPHA_SIZE,
    EGL_BLUE_SIZE,
    EGL_GREEN_SIZE,
    EGL_RED_SIZE,
    EGL_CONFIG_CAVEAT,
    EGL_LEVEL,
    EGL_MAX_PBUFFER_HEIGHT,
    EGL_MAX_PBUFFER_PIXELS,
    EGL_MAX_PBUFFER_WIDTH,
    EGL_NATIVE_RENDERABLE,
    EGL_NATIVE_VISUAL_ID,
    EGL_NATIVE_VISUAL_TYPE,
    EGL_SAMPLES,
    EGL_SAMPLE_BUFFERS,
    EGL_TRANSPARENT_TYPE,
    EGL_TRANSPARENT_BLUE_VALUE,
    EGL_TRANSPARENT_GREEN_VALUE,
    EGL_TRANSPARENT_RED_VALUE,
    EGL_BIND_TO_TEXTURE_RGB,
    EGL_BIND_TO_TEXTURE_RGBA,
    EGL_MIN_SWAP_INTERVAL,
    EGL_MAX_SWAP_INTERVAL,
    EGL_LUMINANCE_SIZE,
    EGL_ALPHA_MASK_SIZE,
    EGL_COLOR_BUFFER_TYPE,
    //EGL_MATCH_NATIVE_PIXMAP,
    EGL_CONFORMANT
};

const int FBConfig::s_numConfigAttribs = sizeof(FBConfig::s_configAttribs) / sizeof(GLuint);

InitConfigStatus FBConfig::initConfigList(FrameBuffer *fb)
{
    InitConfigStatus ret = INIT_CONFIG_FAILED;

    if (!fb) {
        return ret;
    }

    const FrameBufferCaps &caps = fb->getCaps();
    EGLDisplay dpy = fb->getDisplay();

    if (dpy == EGL_NO_DISPLAY) {
        fprintf(stderr,"Could not get EGL Display\n");
        return ret;
    }

    //
    // Query the set of configs in the EGL backend
    //
    EGLint nConfigs;
    if (!s_egl.eglGetConfigs(dpy, NULL, 0, &nConfigs)) {
        fprintf(stderr, "Could not get number of available configs\n");
        return ret;
    }
    EGLConfig *configs = new EGLConfig[nConfigs];
    s_egl.eglGetConfigs(dpy, configs, nConfigs, &nConfigs);

    //
    // copy the config attributes, filter out
    // configs we do not want to support.
    //
    int j = 0;
    s_fbConfigs = new FBConfig*[nConfigs];
    for (int i=0; i<nConfigs; i++) {

        //
        // filter out configs which does not support pbuffers.
        // we only support pbuffer configs since we use a pbuffer
        // handle to bind a guest created window object.
        //
        EGLint surfaceType;
        s_egl.eglGetConfigAttrib(dpy, configs[i],
                                 EGL_SURFACE_TYPE, &surfaceType);
        if (!(surfaceType & EGL_PBUFFER_BIT)) continue;

        //
        // Filter out not RGB configs
        //
        EGLint redSize, greenSize, blueSize;
        s_egl.eglGetConfigAttrib(dpy, configs[i], EGL_RED_SIZE, &redSize);
        s_egl.eglGetConfigAttrib(dpy, configs[i], EGL_BLUE_SIZE, &blueSize);
        s_egl.eglGetConfigAttrib(dpy, configs[i], EGL_GREEN_SIZE, &greenSize);
        if (redSize==0 || greenSize==0 || blueSize==0) continue;

        s_fbConfigs[j++] = new FBConfig(dpy, configs[i]);
    }
    s_numConfigs = j;

    delete[] configs;

    return s_numConfigs > 0 ? INIT_CONFIG_PASSED : INIT_CONFIG_FAILED;
}

const FBConfig *FBConfig::get(int p_config)
{
    if (p_config >= 0 && p_config < s_numConfigs) {
        return s_fbConfigs[p_config];
    }
    return NULL;
}

int FBConfig::getNumConfigs()
{
    return s_numConfigs;
}

void FBConfig::packConfigsInfo(GLuint *buffer)
{
    memcpy(buffer, s_configAttribs, s_numConfigAttribs * sizeof(GLuint));
    for (int i=0; i<s_numConfigs; i++) {
        memcpy(buffer+(i+1)*s_numConfigAttribs,
               s_fbConfigs[i]->m_attribValues,
               s_numConfigAttribs * sizeof(GLuint));
    }
}

int FBConfig::chooseConfig(FrameBuffer *fb, EGLint * attribs, uint32_t * configs, uint32_t configs_size)
{
    // 6-Z348 (rn298 decode): the previous implementation delegated the
    // match to the HOST's eglChooseConfig (with a forced EGL_SURFACE_TYPE
    // = EGL_PBUFFER_BIT) and intersected by CONFIG_ID. On the redroid
    // host EGL the inner chooser returns ok=1 with ZERO matches for every
    // request — including the no-attrib one — while the SAME display
    // enumerates 40 configs; the host chooser is simply not trustworthy
    // here. We OWN the full config table (40 entries × 32 attribs, the
    // same data rcGetConfigs serves to the client), so match the client's
    // attribs against it directly with the EGL selection semantics:
    //   mask attribs (RENDERABLE_TYPE / SURFACE_TYPE / CONFORMANT):
    //       (cfg & req) == req;
    //   size attribs (RED/GREEN/BLUE/ALPHA/DEPTH/STENCIL/BUFFER_SIZE/
    //                 SAMPLES/SAMPLE_BUFFERS/LUMINANCE/ALPHA_MASK):
    //       cfg >= req;
    //   everything else: exact match. An absent attrib imposes no
    //   constraint (the pragmatic subset the A11 guests actually send).
    // The 6-Z347 inner diagnostic stays for one more run as evidence.
    (void)fb;

    static unsigned s_6z348_tbl = 0;
    if (s_6z348_tbl < 12) {
        s_6z348_tbl++;
        int req_renderable = -1;
        if (attribs) {
            for (EGLint *ap = attribs; ap[0] != EGL_NONE; ap += 2) {
                if (ap[0] == EGL_RENDERABLE_TYPE) { req_renderable = ap[1]; break; }
            }
        }
        RLOG_6Z339("6-Z348 table-based chooseConfig: %d entries, request RENDERABLE_TYPE=0x%x",
                   s_numConfigs, (unsigned)req_renderable);
    }

    uint32_t nVerifiedCfgs = 0;
    if (!attribs || s_numConfigs <= 0) {
        // No constraints: the whole table in order (config 0 first).
        for (int fbIdx = 0; fbIdx < s_numConfigs; fbIdx++) {
            if (configs && nVerifiedCfgs < configs_size) {
                configs[nVerifiedCfgs] = (uint32_t)fbIdx;
            }
            nVerifiedCfgs++;
        }
        return (int)nVerifiedCfgs;
    }

    for (int fbIdx = 0; fbIdx < s_numConfigs; fbIdx++) {
        GLint *cfg = s_fbConfigs[fbIdx]->m_attribValues;
        bool ok = true;
        for (EGLint *ap = attribs; ok && ap[0] != EGL_NONE; ap += 2) {
            const EGLint req = ap[1];
            switch (ap[0]) {
            case EGL_RENDERABLE_TYPE:
            case EGL_SURFACE_TYPE:
            case EGL_CONFORMANT:
                // mask semantics, resolved explicitly per attrib:
                if (ap[0] == EGL_RENDERABLE_TYPE)      ok = ((cfg[2] & req) == req);
                else if (ap[0] == EGL_SURFACE_TYPE)    ok = ((cfg[3] & req) == req);
                else                                   ok = ((cfg[31] & req) == req);
                break;
            case EGL_DEPTH_SIZE:      ok = (cfg[0]  >= req); break;
            case EGL_STENCIL_SIZE:    ok = (cfg[1]  >= req); break;
            case EGL_BUFFER_SIZE:     ok = (cfg[5]  >= req); break;
            case EGL_ALPHA_SIZE:      ok = (cfg[6]  >= req); break;
            case EGL_BLUE_SIZE:       ok = (cfg[7]  >= req); break;
            case EGL_GREEN_SIZE:      ok = (cfg[8]  >= req); break;
            case EGL_RED_SIZE:        ok = (cfg[9]  >= req); break;
            case EGL_CONFIG_CAVEAT:         ok = (cfg[10] == req); break;
            case EGL_LEVEL:                 ok = (cfg[11] == req); break;
            case EGL_MAX_PBUFFER_HEIGHT:    ok = (cfg[12] >= req); break;
            case EGL_MAX_PBUFFER_PIXELS:    ok = (cfg[13] >= req); break;
            case EGL_MAX_PBUFFER_WIDTH:     ok = (cfg[14] >= req); break;
            case EGL_NATIVE_RENDERABLE:     ok = (cfg[15] == req); break;
            case EGL_NATIVE_VISUAL_ID:      ok = (cfg[16] == req); break;
            case EGL_NATIVE_VISUAL_TYPE:    ok = (cfg[17] == req); break;
            case EGL_SAMPLES:               ok = (cfg[18] >= req); break;
            case EGL_SAMPLE_BUFFERS:        ok = (cfg[19] >= req); break;
            case EGL_TRANSPARENT_TYPE:      ok = (cfg[20] == req); break;
            case EGL_TRANSPARENT_BLUE_VALUE:  ok = (cfg[21] == req); break;
            case EGL_TRANSPARENT_GREEN_VALUE: ok = (cfg[22] == req); break;
            case EGL_TRANSPARENT_RED_VALUE:   ok = (cfg[23] == req); break;
            case EGL_BIND_TO_TEXTURE_RGB:   ok = (cfg[24] == req); break;
            case EGL_BIND_TO_TEXTURE_RGBA:  ok = (cfg[25] == req); break;
            case EGL_MIN_SWAP_INTERVAL:     ok = (cfg[26] <= req); break;
            case EGL_MAX_SWAP_INTERVAL:     ok = (cfg[27] >= req); break;
            case EGL_LUMINANCE_SIZE:        ok = (cfg[28] >= req); break;
            case EGL_ALPHA_MASK_SIZE:       ok = (cfg[29] >= req); break;
            case EGL_COLOR_BUFFER_TYPE:     ok = (cfg[30] == req); break;
            case EGL_CONFIG_ID:             ok = (cfg[4]  == req); break;
            case EGL_MATCH_NATIVE_PIXMAP:   ok = true; break; // not supported
            default: ok = true; break; // unknown attribs don't filter
            }
        }
        if (ok) {
            if (configs && nVerifiedCfgs < configs_size) {
                configs[nVerifiedCfgs] = (uint32_t)fbIdx;
            }
            nVerifiedCfgs++;
        }
    }

    return (int)nVerifiedCfgs;
}

FBConfig::FBConfig(EGLDisplay p_eglDpy, EGLConfig p_eglCfg)
{
    m_eglConfig = p_eglCfg;
    m_attribValues = new GLint[s_numConfigAttribs];
    for (int i=0; i<s_numConfigAttribs; i++) {
        m_attribValues[i] = 0;
        s_egl.eglGetConfigAttrib(p_eglDpy, p_eglCfg, s_configAttribs[i], &m_attribValues[i]);

        //
        // All exported configs supports android native window rendering
        //
        if (s_configAttribs[i] == EGL_SURFACE_TYPE) {
            m_attribValues[i] |= EGL_WINDOW_BIT;
        }

        // 6-Z339 (rn288 decode): this emugl port is an ES2-class host
        // (GL2 decoder + ES2 GLDispatch). The host EGL's configs (redroid)
        // advertise EGL_OPENGL_ES3_BIT (0x40), which made A11 surfaceflinger
        // pick contextClientVersion=3 (renderableType & ES3_BIT in
        // GLESRenderEngine::createEglContext) — a request the client's own
        // max-version gate rejects with EGL_BAD_CONFIG (no advertised
        // ANDROID_EMU_gles_max_version token) → SF SIGABRT crash-loop.
        // The guest must see exactly the GPU the host can serve: mask the
        // ES3 bit out of EGL_RENDERABLE_TYPE so SF requests an ES2 context
        // (rcCreateContext glVersion=2 → the ES2 path this renderer was
        // built and validated for). EGL_OPENGL_ES2_BIT stays advertised.
        if (s_configAttribs[i] == EGL_RENDERABLE_TYPE) {
            m_attribValues[i] &= ~((GLint)0x40 /* EGL_OPENGL_ES3_BIT_KHR */);
        }
    }
}

FBConfig::~FBConfig()
{
    if (m_attribValues) {
        delete[] m_attribValues;
    }
}
