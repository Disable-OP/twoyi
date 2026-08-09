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
#ifndef _GLES2_DISPATCH_H
#define _GLES2_DISPATCH_H

// twoyi patch: always include gl2_dec.h and declare the dispatch
// functions, even when WITH_GLES2 is not defined. The original AOSP
// code guards these behind #ifdef WITH_GLES2, but render_api.cpp and
// RenderThread.cpp call init_gl2_dispatch() and
// gl2_dispatch_get_proc_func() unconditionally. When WITH_GLES2 is
// off, GL2DispatchStub.cpp provides no-op implementations.
#include "gl2_dec.h"

bool init_gl2_dispatch();
void *gl2_dispatch_get_proc_func(const char *name, void *userData);

extern gl2_decoder_context_t s_gl2;
extern int                   s_gl2_enabled;

#ifdef WITH_GLES2
// The real init_gl2_dispatch implementation (in GL2Dispatch.cpp) is
// only compiled when WITH_GLES2 is defined. Otherwise
// GL2DispatchStub.cpp provides the no-op stub.
#endif

#endif
