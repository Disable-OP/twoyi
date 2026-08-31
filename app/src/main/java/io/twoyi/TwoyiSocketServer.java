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
import io.twoyi.utils.FileLogger;
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

    // 6-Z271: host-backed HAL bridge requests from the guest's virtual
    // binder services / sysfs (kr64 hostbridge.rs).
    private static final String TWOYI_VIBRATE = "TWOYI_VIBRATE";
    private static final String TWOYI_VIBRATE_OFF = "TWOYI_VIBRATE_OFF";
    private static final String TWOYI_TORCH = "TWOYI_TORCH";

    // Fixed: use fixed thread pool with daemon threads to prevent
    // unbounded thread growth and JVM shutdown issues
    private static ExecutorService EXECUTOR = Executors.newFixedThreadPool(4, r -> {
        Thread t = new Thread(r, "TwoyiSocket-Worker");
        t.setDaemon(true);
        return t;
    });

    private final AtomicBoolean mStarted = new AtomicBoolean(false);
    private final Context mContext;

    private TwoyiSocketServer(Context context) {
        // Fixed: use ApplicationContext to prevent Activity memory leak
        mContext = context.getApplicationContext();
    }

    // Fixed: synchronize getInstance to prevent race condition
    public static synchronized TwoyiSocketServer getInstance(Context context) {
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

            // Fixed: mark the server as started BEFORE entering the accept
            // loop. The catch block below sets mStarted=false on bind
            // failure so that start() can be re-invoked; without re-setting
            // it to true here, a successful retry (start0(attempt+1)) would
            // leave mStarted=false even though the server is now listening.
            // That in turn would let a subsequent start() call spawn ANOTHER
            // start0(0), which would fail to bind (EADDRINUSE) and trigger
            // yet another retry — a slow thundering-herd leak.
            mStarted.set(true);

            Thread currentThread = Thread.currentThread();
            while (!currentThread.isInterrupted()) {
                LocalSocket localSocket = localServerSocket.accept();
                handleSocket(localSocket);
            }
        } catch (IOException e) {
            Log.e(TAG, "start socket failed (attempt " + (attempt + 1) + "/" + MAX_START_RETRIES + ")", e);

            IOUtils.closeSilently(socket);
            // Only clear mStarted if we're giving up — otherwise the
            // in-flight retry (scheduled below) owns the lifecycle and
            // will re-set mStarted=true on its own success.
            if (attempt + 1 >= MAX_START_RETRIES) {
                Log.e(TAG, "giving up after " + MAX_START_RETRIES + " failed bind attempts; "
                        + "the guest will not be able to send control messages to the host.");
                mStarted.set(false);
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
                // Fixed: check for EOF (read returns -1) to prevent
                // StringIndexOutOfBoundsException from new String(data, 0, -1, ...)
                if (read <= 0) break;
                handleData(new String(data, 0, read, StandardCharsets.US_ASCII));
            }

        } catch (IOException ignored) {
        } finally {
            // Close the accepted socket to prevent FD leak. Uses
            // IOUtils.closeSilently for consistency with start0()'s
            // finally block (single idiom across the file).
            IOUtils.closeSilently(socket);
        }
    }

    private void handleData(String msg) {
        if (msg.startsWith(SWITCH_HOST)) {
            // switch host system
            TwoyiStatusManager.getInstance().switchOs(mContext);
        } else if (msg.startsWith(BOOT_COMPLETED)) {
            // machine started — delegate to BootCompletionServer, which
            // owns the boot latch (ported from cyanmint/Nogitsune's
            // BootStatus.kt + NogitsuneSocketServer.kt pattern).
            // BootCompletionServer.markCompleted() will in turn call
            // TwoyiStatusManager.markStarted() so switchOs() still works.
            // Task 6-Z62: kr64's SEQPACKET client hits @TWOYI_SOCK first
            // (bound in attachBaseContext) — log the receipt so E2E
            // logcat shows the delivery on the legacy path too.
            FileLogger.boot("boot_completed_received_via_twoyi_sock", null);
            BootCompletionServer.getInstance().markCompleted();
        } else if (msg.startsWith(JUMP_HOST_SETTINGS)) {
            // UIHelper.startActivity(mContext, AboutActivity.class);
            UIHelper.startActivity(mContext, SettingsActivity.class);
        } else if (msg.startsWith(TWOYI_VIBRATE_OFF)) {
            // 6-Z271: guest IVibrator.off() / sysfs one-shot reset —
            // cancel the host phone's real vibration.
            HostHalBridge.cancelVibrate(mContext);
        } else if (msg.startsWith(TWOYI_VIBRATE)) {
            // 6-Z271: guest IVibrator.on(ms) — REAL host vibration.
            // Format: "TWOYI_VIBRATE:<ms>".
            try {
                long ms = Long.parseLong(msg.substring(TWOYI_VIBRATE.length() + 1).trim());
                HostHalBridge.vibrate(mContext, ms);
            } catch (RuntimeException e) {
                Log.w(TAG, "malformed vibrate request: " + msg);
            }
        } else if (msg.startsWith(TWOYI_TORCH)) {
            // 6-Z271: guest torch LED write — real camera flash.
            // Format: "TWOYI_TORCH:1" / "TWOYI_TORCH:0".
            String arg = msg.substring(TWOYI_TORCH.length() + 1).trim();
            HostHalBridge.setTorch(mContext, arg.equals("1"));
        }
    }
}
