/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi;

import android.net.LocalServerSocket;
import android.net.LocalSocket;
import android.net.LocalSocketAddress;
import android.os.SystemClock;
import android.util.Log;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

import io.twoyi.utils.FileLogger;
import io.twoyi.utils.IOUtils;
import io.twoyi.utils.LogEvents;

/**
 * Dedicated server for the guest's {@code BOOT_COMPLETED} signal, plus the
 * host-side latch that wakes the UI when boot finishes.
 *
 * <p>This is a focused, self-contained class that was previously scattered
 * across {@link TwoyiSocketServer} (which still owns the legacy multiplexed
 * {@code TWOYI_SOCK} socket for {@code SWITCH_HOST}/{@code SETTINGS} etc.)
 * and {@link TwoyiStatusManager} (which still owns the {@code mStarted} flag
 * consumed by {@link TwoyiStatusManager#switchOs(Context)}).
 *
 * <p>Architecture:
 * <ul>
 *   <li>The boot-completion latch (a one-shot {@link java.util.concurrent.CountDownLatch})
 *       lives here, not in {@link TwoyiStatusManager}.</li>
 *   <li>{@link TwoyiSocketServer#handleData(String)} delegates
 *       {@code BOOT_COMPLETED} to {@link #markCompleted()} so the existing
 *       guest binary (which sends to {@code TWOYI_SOCK}) keeps working.</li>
 *   <li>This class additionally binds a <em>second</em> abstract socket,
 *       {@code TWOYI_BOOT_SOCK}, dedicated to boot-completion only. Future
 *       guests can send {@code BOOT_COMPLETED} there directly without
 *       multiplexing with other control messages. The current guest does
 *       not use it, so the listener is effectively a no-op until a guest
 *       opts in — but it is wired up and ready.</li>
 *   <li>{@link #markCompleted()} is idempotent ({@link AtomicBoolean}
 *       compare-and-set), so receiving {@code BOOT_COMPLETED} on both
 *       sockets (or twice on either) is harmless.</li>
 * </ul>
 *
 * <p>Ported from cyanmint/Nogitsune (MPL-2.0, same license as twoyi):
 * <ul>
 *   <li>{@code globals/BootStatus.kt} — the {@link CyclicBarrier} +
 *       {@link AtomicBoolean} state pattern.</li>
 *   <li>{@code globals/NogitsuneSocketServer.kt} — the abstract
 *       {@code SOCKET_SEQPACKET} listener pattern.</li>
 * </ul>
 * Adapted to twoyi's Java architecture: twoyi already had a multiplexed
 * {@link TwoyiSocketServer} on {@code TWOYI_SOCK}, so this class is
 * additive (a dedicated boot socket + the relocated latch) rather than a
 * wholesale replacement of the existing IPC.
 *
 * @author Disable-OP
 * @date 2026/08/08.
 */
public class BootCompletionServer {

    private static final String TAG = "BootCompletionServer";

    /**
     * Dedicated abstract socket name for the boot-completion signal.
     *
     * <p>The legacy multiplexed socket {@code TWOYI_SOCK} (owned by
     * {@link TwoyiSocketServer}) is kept untouched for backwards
     * compatibility with the existing guest binary. This new name is for
     * future guests that want to send {@code BOOT_COMPLETED} on a
     * dedicated channel without multiplexing with {@code SWITCH_HOST} /
     * {@code SETTINGS}.
     */
    private static final String SOCK_NAME = "TWOYI_BOOT_SOCK";

    private static final String BOOT_COMPLETED = "BOOT_COMPLETED";

    /**
     * How long {@link #markCompleted()} waits for the UI to rendezvous on
     * the latch, and the default window {@link Render2Activity} uses when
     * waiting for boot. Mirrors Nogitsune's 60 s timeout.
     */
    public static final long BOOT_TIMEOUT_SECONDS = 60L;

    private static BootCompletionServer INSTANCE;

    /**
     * Fixed-size daemon thread pool. TwoyiSocketServer uses the same
     * pattern; we replicate it here so a runaway bind-retry loop on the
     * dedicated boot socket can't grow an unbounded cached pool.
     */
    private static final ExecutorService EXECUTOR = Executors.newFixedThreadPool(4, r -> {
        Thread t = new Thread(r, "BootCompletion-Worker");
        t.setDaemon(true);
        return t;
    });

    private final AtomicBoolean mCompleted = new AtomicBoolean(false);
    private final AtomicBoolean mListening = new AtomicBoolean(false);

    /**
     * Boot-completion latch.
     *
     * <p>6-Z268: this was a two-party {@link CyclicBarrier} — the worker
     * calling {@link #markCompleted()} BLOCKED in {@code await()} until the
     * UI happened to enter the next {@link #waitBoot(long, TimeUnit)}
     * slice. Render2Activity polls waitBoot in 5 s slices, so a
     * BOOT_COMPLETED landing just after a slice timeout sat unserved for
     * the remainder of the slice: an average +2.5 s, worst-case +5 s of
     * PURE DEAD TIME added to boot-to-UI after the guest had already
     * finished (and the worker thread was pinned the whole time).
     *
     * <p>A one-shot {@link CountDownLatch} inverts the rendezvous:
     * {@link #markCompleted()} counts down and returns immediately (no
     * worker pinned, no slice-miss latency — the UI's in-flight
     * {@code await} wakes the instant the count reaches zero), and
     * {@link #waitBoot(long, TimeUnit)} returns {@code true} immediately
     * whenever completion has already happened. {@link #reset()} swaps in
     * a fresh latch (guarded by {@code synchronized} so a concurrent
     * count-down can never be lost by the swap).
     */
    private volatile CountDownLatch mBootLatch = new CountDownLatch(1);

    private BootCompletionServer() {
    }

    public static synchronized BootCompletionServer getInstance() {
        if (INSTANCE == null) {
            INSTANCE = new BootCompletionServer();
        }
        return INSTANCE;
    }

    /**
     * Starts a fresh boot cycle: resets the latch and the completed flag,
     * then spawns (idempotently) a background listener on
     * {@link #SOCK_NAME} so future guests can signal
     * {@code BOOT_COMPLETED} on a dedicated socket.
     *
     * <p>Safe to call on every {@link Render2Activity#onCreate(Bundle)};
     * the listener bind is guarded by {@link #mListening} so we never
     * leak a second accept thread on a re-launch.
     *
     * <p>Bind failures are retried with exponential backoff (mirroring
     * {@link TwoyiSocketServer}'s retry strategy). If the bind never
     * succeeds, the legacy {@link TwoyiSocketServer} path on
     * {@code TWOYI_SOCK} still delivers {@code BOOT_COMPLETED} to
     * {@link #markCompleted()} — so the boot flow is unaffected.
     */
    public void start() {
        // Reset state for a fresh boot attempt. Both the UI side
        // (waitBoot) and the worker side (markCompleted) rendezvous on
        // the same barrier, so a stale barrier from a previous boot
        // attempt would either hang the UI (if the worker had already
        // passed through) or hang the worker (if the UI had already
        // passed through). reset() re-arms both sides cleanly.
        reset();

        if (mListening.compareAndSet(false, true)) {
            // Explicit Runnable cast to disambiguate — submit(Runnable)
            // and submit(Callable) both match a no-arg method reference,
            // causing a "reference to submit is ambiguous" compile error
            // under JDK 17. Same workaround TwoyiSocketServer uses.
            EXECUTOR.submit((Runnable) this::start0);
        }
    }

    /**
     * Maximum number of times {@link #start0()} will retry binding the
     * abstract SEQPACKET socket before giving up. Mirrors
     * {@link TwoyiSocketServer}'s cap; the legacy socket path is the
     * fallback if this one never binds.
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

            // Re-mark as listening BEFORE entering the accept loop, so a
            // successful retry doesn't leave mListening=false (which would
            // let a subsequent start() spawn a second accept thread).
            mListening.set(true);

            Thread currentThread = Thread.currentThread();
            while (!currentThread.isInterrupted()) {
                LocalSocket client = localServerSocket.accept();
                handleClient(client);
            }
        } catch (IOException e) {
            Log.e(TAG, "bind " + SOCK_NAME + " failed (attempt "
                    + (attempt + 1) + "/" + MAX_START_RETRIES + ")", e);

            IOUtils.closeSilently(socket);

            if (attempt + 1 >= MAX_START_RETRIES) {
                // Not fatal — the legacy TWOYI_SOCK path still delivers
                // BOOT_COMPLETED via TwoyiSocketServer.handleData().
                Log.w(TAG, "giving up on " + SOCK_NAME + " after "
                        + MAX_START_RETRIES + " attempts; "
                        + "falling back to the legacy TWOYI_SOCK path.");
                mListening.set(false);
                return;
            }

            long backoff = Math.min(INITIAL_BACKOFF_MS << attempt, MAX_BACKOFF_MS);
            long jitter  = (long) (Math.random() * (backoff / 2));
            SystemClock.sleep(backoff + jitter);

            // Re-submit instead of recursing, to avoid stack growth and
            // to avoid pinning a thread on a runaway failure.
            EXECUTOR.execute(() -> start0(attempt + 1));
        } finally {
            IOUtils.closeSilently(socket);
        }
    }

    private void handleClient(LocalSocket client) {
        EXECUTOR.execute(() -> {
            try {
                InputStream in = client.getInputStream();
                Thread currentThread = Thread.currentThread();
                while (!currentThread.isInterrupted()) {
                    byte[] data = new byte[1024];
                    int read = in.read(data);
                    // EOF (read == -1) or empty read (read == 0) → close.
                    if (read <= 0) break;
                    String msg = new String(data, 0, read, StandardCharsets.US_ASCII);
                    if (msg.startsWith(BOOT_COMPLETED)) {
                        Log.i(TAG, "BOOT_COMPLETED received on " + SOCK_NAME);
                        FileLogger.boot("boot_completed_received", "sock=" + SOCK_NAME);
                        markCompleted();
                        // 6-Z184: a client that sends BOOT_COMPLETED and then
                        // stays connected blocked this worker in read()
                        // FOREVER — four such clients starved the fixed
                        // 4-thread pool (and the legacy-socket path). The
                        // contract is one notification per connection; break
                        // and let the finally-close clean up.
                        break;
                    }
                }
            } catch (IOException ignored) {
                // Client disconnected — not interesting.
            } finally {
                IOUtils.closeSilently(client);
            }
        });
    }

    /**
     * Signals that the guest has finished booting. Idempotent: a second
     * call (e.g. from both the legacy {@code TWOYI_SOCK} path and the
     * dedicated {@code TWOYI_BOOT_SOCK} path) is a no-op.
     *
     * <p>This method also notifies {@link TwoyiStatusManager} so that
     * {@link TwoyiStatusManager#switchOs(Context)} (which depends on
     * {@code mStarted}) keeps working unchanged.
     *
     * <p>6-Z268: this method NO LONGER BLOCKS. The old 60 s barrier
     * rendezvous pinned a pool worker until the UI's next waitBoot slice
     * and delayed the overlay dismissal by the slice remainder; the
     * latch now releases every waiter immediately.
     */
    public void markCompleted() {
        if (!mCompleted.compareAndSet(false, true)) {
            // Already marked — duplicate BOOT_COMPLETED (e.g. guest sent
            // to both sockets, or sent twice). Harmless.
            FileLogger.boot("boot_completed_duplicate", "ignored (already marked)");
            return;
        }
        FileLogger.boot("boot_completed_marked", "latch released (6-Z268: non-blocking)");

        // Notify TwoyiStatusManager so its switchOs() / isStarted() keep
        // working. markStarted() is now a plain setter (the boot latch
        // lives here), so this can't block.
        TwoyiStatusManager.getInstance().markStarted();

        // Wake every UI-side waiter IMMEDIATELY. No rendezvous, no slice
        // miss, no worker pinned.
        mBootLatch.countDown();
    }

    /**
     * Waits for the guest to finish booting, up to {@code timeout}.
     *
     * <p>Called from {@link Render2Activity#showBootingProcedure()} on a
     * background thread. Returns {@code true} if the guest booted within
     * the timeout, {@code false} otherwise (boot failure path).
     *
     * <p>Unlike the old {@link TwoyiStatusManager#waitBoot(long, TimeUnit)},
     * this method swallows {@link InterruptedException} and
     * {@link BrokenBarrierException} and returns {@code false} — matching
     * Nogitsune's {@code BootStatus.waitBoot()} API. Callers that need to
     * distinguish "timeout" from "interrupted" can inspect
     * {@link #isCompleted()} on return.
     */
    public boolean waitBoot(long timeout, TimeUnit unit) {
        FileLogger.boot("wait_boot_start", "timeout=" + timeout + " unit=" + unit);
        try {
            // 6-Z268: plain latch await — returns true instantly when the
            // count already hit zero, false on timeout. No barrier state
            // to re-arm between polling slices.
            boolean ok = mBootLatch.await(timeout, unit);
            if (ok) {
                FileLogger.boot("wait_boot_success", null);
            } else {
                FileLogger.boot("wait_boot_timeout", null);
            }
            return ok;
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            FileLogger.boot("wait_boot_interrupted", null);
            LogEvents.trackError(e);
            return false;
        }
    }

    /**
     * Returns {@code true} if {@link #markCompleted()} has been called
     * for the current boot cycle.
     */
    public boolean isCompleted() {
        return mCompleted.get();
    }

    /**
     * Re-arms the boot latch and clears the completed flag. Called by
     * {@link Render2Activity#onCreate(Bundle)} before starting a new
     * boot attempt.
     */
    public void reset() {
        synchronized (this) {
            mCompleted.set(false);
            // Fresh one-shot latch for the new boot cycle. The swap is
            // serialized with itself; a markCompleted() racing the swap
            // either counts down the OLD latch (waiters from the old
            // cycle — correct) or CAS's mCompleted after it was cleared
            // and counts down the NEW latch — both orderings are safe.
            mBootLatch = new CountDownLatch(1);
        }
    }
}
