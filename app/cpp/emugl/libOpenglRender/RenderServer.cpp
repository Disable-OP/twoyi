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
#include "RenderServer.h"
#include "TcpStream.h"
#ifdef _WIN32
#include "Win32PipeStream.h"
#else
#include "UnixStream.h"
#endif
#include "RenderThread.h"
#include "FrameBuffer.h"
#include <set>
#include <stdio.h>

// 6-Z336 wire observer — logcat-visible regardless of which ERR variant
// the include chain resolves to (ErrorLog.h's fprintf leg loses the
// evidence; the artifacts only capture logcat + the app's own streams).
#if defined(__ANDROID__)
#include <android/log.h>
#define RLOG_6Z336(...) __android_log_print(ANDROID_LOG_ERROR, "TWOYI_RENDERER", __VA_ARGS__)
#else
#define RLOG_6Z336(...) fprintf(stderr, __VA_ARGS__)
#endif

typedef std::set<RenderThread *> RenderThreadsSet;

RenderServer::RenderServer() :
    m_listenSock(NULL),
    m_exiting(false)
{
}

extern "C" int gRendererStreamMode;

RenderServer *RenderServer::create(int port)
{
    RenderServer *server = new RenderServer();
    if (!server) {
        return NULL;
    }

    if (gRendererStreamMode == STREAM_MODE_TCP) {
        server->m_listenSock = new TcpStream();
    } else {
#ifdef _WIN32
        server->m_listenSock = new Win32PipeStream();
#else
        server->m_listenSock = new UnixStream();
#endif
    }

    if (server->m_listenSock->listen(port) < 0) {
        ERR("RenderServer::create failed to listen on port %d\n", port);
        delete server;
        return NULL;
    }

    return server;
}

int RenderServer::Main()
{
    RenderThreadsSet threads;
    // 6-Z336 (rn285 decode): the composer deadlocked at its FIRST
    // renderControl read while this loop sat silent — nothing in the
    // artifacts could distinguish "renderer never accepted" from
    // "accepted but desynced". Log the accept path (bounded: one line
    // per accept + one per failed flags read; session counts are small).
    static int s_6z336_accepts = 0;

    while(1) {
        SocketStream *stream = m_listenSock->accept();
        if (!stream) {
            ERR("Error accepting connection, aborting\n");
            break;
        }
        ++s_6z336_accepts;
        RLOG_6Z336("6-Z336 RenderServer: accept #%d ok", s_6z336_accepts);

        unsigned int clientFlags;
        if (!stream->readFully(&clientFlags, sizeof(unsigned int))) {
            RLOG_6Z336("6-Z336 RenderServer: accept #%d clientFlags read FAILED"
                       " (client hung up before sending 4-byte flags)",
                       s_6z336_accepts);
            fprintf(stderr,"Error reading clientFlags\n");
            delete stream;
            continue;
        }
        RLOG_6Z336("6-Z336 RenderServer: accept #%d clientFlags=0x%08x",
                   s_6z336_accepts, clientFlags);

        DBG("\n\n\n\n Got new stream!!!! \n\n\n\n\n");
        // check if we have been requested to exit while waiting on accept
        if ((clientFlags & IOSTREAM_CLIENT_EXIT_SERVER) != 0) {
            m_exiting = true;
            break;
        }

        RenderThread *rt = RenderThread::create(stream);
        if (!rt) {
            // RenderThread::create only fails if `new RenderThread()`
            // returns NULL — in which case nobody took ownership of
            // `stream`, so we delete it here. We must also `continue`,
            // otherwise the next `if (!rt->start())` would dereference
            // the NULL pointer (the original bug).
            fprintf(stderr,"Failed to create RenderThread\n");
            delete stream;
            continue;
        }

        if (!rt->start()) {
            // RenderThread::create succeeded, so `rt` now holds `stream`
            // in its m_stream field. The RenderThread class does NOT
            // delete m_stream in its destructor, so we still own it and
            // must delete it ourselves before deleting rt.
            fprintf(stderr,"Failed to start RenderThread\n");
            delete stream;
            delete rt;
            continue;
        }

        //
        // remove from the threads list threads which are
        // no longer running
        //
        for (RenderThreadsSet::iterator n,t = threads.begin();
             t != threads.end();
             t = n) {
            // first find next iterator
            n = t;
            n++;

            // delete and erase the current iterator
            // if thread is no longer running
            if ((*t)->isFinished()) {
                // 6-Z389 (rn348 decode): isFinished() (m_finished) is set
                // inside RenderThread::Main() BEFORE osUtils::Thread's exit
                // handshake (thread_main's pthread_mutex_lock(&m_lock) to
                // publish m_isRunning=false) runs. Deleting the object in
                // that window destroys m_lock while thread_main is about to
                // lock it -> bionic FORTIFY "pthread_mutex_lock called on a
                // destroyed mutex" -> SIGABRT kills the whole VMM process
                // (and the guest with it). rn348 run-ender: tid 3100,
                // 12:21:17.987, abort pc in osUtils::Thread::thread_main,
                // 93s after the app's stderr went quiet, one render-stream
                // churn cycle after a clean rcCreateContext/rcMakeCurrent
                // sequence. Main() having returned means the thread is
                // microseconds from exit - join it first (bounded, no hang
                // risk) so the delete can never race the exit handshake.
                int exitStatus;
                (*t)->wait(&exitStatus);
                delete (*t);
                threads.erase(t);
            }
        }

        // insert the added thread to the list
        threads.insert(rt);

        DBG("Started new RenderThread\n");
    }

    //
    // Wait for all threads to finish
    //
    for (RenderThreadsSet::iterator t = threads.begin();
         t != threads.end();
         t++) {
        int exitStatus;
        (*t)->wait(&exitStatus);
        delete (*t);
    }
    threads.clear();

    //
    // de-initialize the FrameBuffer object
    //
    FrameBuffer::finalize();
    return 0;
}
