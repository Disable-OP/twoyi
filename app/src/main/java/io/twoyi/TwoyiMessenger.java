/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi;

import android.net.LocalSocket;
import android.net.LocalSocketAddress;
import android.os.Looper;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.OutputStreamWriter;
import java.util.concurrent.Executor;
import java.util.concurrent.Executors;
import java.util.concurrent.locks.ReentrantReadWriteLock;

public class TwoyiMessenger {
    public static final String TAG = "TwoyiMessenger";

    public static final String SWITCH_HOST = "SWITCH_HOST";

    public static final String PING = "PING";

    private static final String SOCK_NAME = "TWOYI_SOCK";

    private volatile OutputStreamWriter mWriter;
    private final ReentrantReadWriteLock mLock;
    private static TwoyiMessenger INSTANCE;
    private LocalSocket socket;

    private static final Executor EXECUTOR = Executors.newCachedThreadPool();

    private TwoyiMessenger() {
        this.socket = null;
        this.mLock = new ReentrantReadWriteLock();
    }

    public synchronized TwoyiMessenger connect() {
        if (this.socket != null) {
            return this;
        }

        LocalSocket socket = null;
        try {
            socket = new LocalSocket(LocalSocket.SOCKET_SEQPACKET);
            socket.connect(new LocalSocketAddress(SOCK_NAME));
            // Only assign to this.socket AFTER successful connect
            this.socket = socket;
            socket = null; // ownership transferred
            InputStream is = this.socket.getInputStream();
            OutputStream os = this.socket.getOutputStream();
            this.mWriter = new OutputStreamWriter(os);
        } catch (IOException e) {
            e.printStackTrace();
            // Fixed: clean up the socket on failure so connect() can be retried.
            // Previously, this.socket was assigned before connect(), so a
            // failed connect left a non-null but unusable socket, and the
            // guard at the top prevented all future reconnection attempts.
            //
            // Also: the original code only closed this.socket, but if connect()
            // threw before the assignment, the local `socket` variable still
            // held an open FD that was leaked. Close the local copy too.
            if (socket != null) {
                try { socket.close(); } catch (IOException ignored) {}
            }
            if (this.socket != null) {
                try { this.socket.close(); } catch (IOException ignored) {}
                this.socket = null;
            }
            this.mWriter = null;
        }
        return this;
    }

    public static synchronized TwoyiMessenger getInstance() {
        if (TwoyiMessenger.INSTANCE == null) {
            TwoyiMessenger.INSTANCE = new TwoyiMessenger();
        }

        return TwoyiMessenger.INSTANCE;
    }

    public void send(String msg) {

        if (Looper.getMainLooper() == Looper.myLooper()) {
            EXECUTOR.execute(new Runnable() {
                @Override
                public void run() {
                    write(msg);
                }
            });
        } else {
            this.write(msg);
        }
    }

    private void write(String msg) {
        synchronized (this) {
            if (mWriter == null) {
                connect();
            }
        }

        // Fixed: connect() may have failed (mWriter still null). The original
        // code unconditionally called mWriter.write(msg) here, which NPE'd.
        // The NPE was swallowed by `catch (Throwable ignored)` so messages
        // were silently dropped without any log. Bail out explicitly instead.
        OutputStreamWriter writer = this.mWriter;
        if (writer == null) {
            return;
        }

        try {
            this.mLock.writeLock().lock();
            writer.write(msg);
            writer.flush();
        } catch (Throwable ignored) {
            // Fixed: a broken pipe (e.g. the guest crashed or rebooted)
            // leaves mWriter pointing at a dead socket. Without resetting
            // the connection state here, every subsequent send() would
            // skip connect() (because mWriter != null) and silently drop
            // the message via the same catch. This made host<->guest
            // control messages (BOOT_COMPLETED, SWITCH_HOST, SETTINGS)
            // disappear forever after the first post-disconnect send,
            // leaving the host UI stuck on "booting..." even after the
            // guest had fully booted. Close + null out the socket so the
            // next send() retries connect().
            synchronized (this) {
                if (this.socket != null) {
                    try { this.socket.close(); } catch (IOException ignored2) {}
                    this.socket = null;
                }
                this.mWriter = null;
            }
        } finally {
            this.mLock.writeLock().unlock();
        }
    }
}

