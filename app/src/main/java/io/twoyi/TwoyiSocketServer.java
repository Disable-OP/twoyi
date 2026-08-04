/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi;

import android.content.Context;
import android.net.LocalServerSocket;
import android.net.LocalSocket;
import android.net.LocalSocketAddress;
import android.os.SystemClock;
import android.util.Log;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;

import io.twoyi.ui.SettingsActivity;
import io.twoyi.utils.IOUtils;
import io.twoyi.utils.UIHelper;

/**
 * @author weishu
 * @date 2021/10/27.
 */

public class TwoyiSocketServer {

    private static final String TAG = "TwoyiSocketServer";

    private static TwoyiSocketServer INSTANCE;

    private static final String SOCK_NAME = "TWOYI_SOCK";

    private static final String SWITCH_HOST = "SWITCH_HOST";
    private static final String BOOT_COMPLETED = "BOOT_COMPLETED";

    private static final String JUMP_HOST_SETTINGS= "SETTINGS";

    private static ExecutorService EXECUTOR = Executors.newCachedThreadPool();

    private final AtomicBoolean mStarted = new AtomicBoolean(false);
    private final Context mContext;

    private TwoyiSocketServer(Context context) {
        mContext = context;
    }

    public static TwoyiSocketServer getInstance(Context context) {
        if (INSTANCE == null) {
            INSTANCE = new TwoyiSocketServer(context);
        }

        return INSTANCE;
    }

    public void start() {
        if (mStarted.compareAndSet(false, true)) {
            // Explicit Runnable cast to disambiguate — start0() and start0(int)
            // both match EXECUTOR.submit(this::start0), causing a "reference to
            // submit is ambiguous" compile error under JDK 17.
            EXECUTOR.submit((Runnable) this::start0);

            EXECUTOR.submit(()-> {

                // some device restrict local socket, just connect it to prompt the permission dialog.
                SystemClock.sleep(3000);

                // SEND PING
                TwoyiMessenger.getInstance().send(TwoyiMessenger.PING);
            });
        }
    }

    /**
     * Maximum number of times {@link #start0()} will retry binding the
     * abstract SEQPACKET socket before giving up. Each retry is separated
     * by an exponential backoff (capped at {@link #MAX_BACKOFF_MS}).
     *
     * <p>The previous implementation called {@link #start()} recursively
     * on every IOException with a fixed 1-second sleep. If the bind kept
     * failing (e.g. because another twoyi instance held the name, or
     * because SELinux denied the operation) the executor thread pool
     * would accumulate one blocked thread per retry, eventually exhausting
     * {@link #EXECUTOR}'s cached pool and starving the rest of the app.
     */
    private static final int MAX_START_RETRIES = 5;
    private static final long INITIAL_BACKOFF_MS = 1_000L;
    private static final long MAX_BACKOFF_MS    = 30_000L;

    private void start0() {
        start0(0);
    }

    private void start0(int attempt) {
        LocalSocket socket = null;
        try {
            socket = new LocalSocket(LocalSocket.SOCKET_SEQPACKET);
            socket.bind(new LocalSocketAddress(SOCK_NAME, LocalSocketAddress.Namespace.ABSTRACT));
            LocalServerSocket localServerSocket = new LocalServerSocket(socket.getFileDescriptor());

            Thread currentThread = Thread.currentThread();
            while (!currentThread.isInterrupted()) {
                LocalSocket localSocket = localServerSocket.accept();
                handleSocket(localSocket);
            }
        } catch (IOException e) {
            Log.e(TAG, "start socket failed (attempt " + (attempt + 1) + "/" + MAX_START_RETRIES + ")", e);

            IOUtils.closeSilently(socket);
            mStarted.set(false);

            if (attempt + 1 >= MAX_START_RETRIES) {
                Log.e(TAG, "giving up after " + MAX_START_RETRIES + " failed bind attempts; "
                        + "the guest will not be able to send control messages to the host.");
                return;
            }

            // Exponential backoff with jitter, capped at MAX_BACKOFF_MS.
            long backoff = Math.min(INITIAL_BACKOFF_MS << attempt, MAX_BACKOFF_MS);
            long jitter  = (long) (Math.random() * (backoff / 2));
            SystemClock.sleep(backoff + jitter);

            // Re-submit to the executor instead of recursing on the same
            // thread, so we don't grow the stack and so a runaway failure
            // doesn't pin a thread forever.
            EXECUTOR.submit(() -> start0(attempt + 1));
        } finally {
            IOUtils.closeSilently(socket);
        }
    }

    private void handleSocket(LocalSocket socket) {
        EXECUTOR.submit(() -> handleSocket0(socket));
    }

    private void handleSocket0(LocalSocket socket) {
        try {
            InputStream inputStream = socket.getInputStream();
            Thread currentThread = Thread.currentThread();

            while (!currentThread.isInterrupted()) {
                byte[] data = new byte[1024];
                int read = inputStream.read(data);
                handleData(new String(data, 0, read, StandardCharsets.US_ASCII));
            }

        } catch (IOException ignored) {
        }
    }

    private void handleData(String msg) {
        if (msg.startsWith(SWITCH_HOST)) {
            // switch host system
            TwoyiStatusManager.getInstance().switchOs(mContext);
        } else if (msg.startsWith(BOOT_COMPLETED)) {
            // machine started
            TwoyiStatusManager.getInstance().markStarted();
        } else if (msg.startsWith(JUMP_HOST_SETTINGS)) {
            // UIHelper.startActivity(mContext, AboutActivity.class);
            UIHelper.startActivity(mContext, SettingsActivity.class);
        }
    }
}
