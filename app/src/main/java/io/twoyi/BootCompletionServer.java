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
import java.util.concurrent.BrokenBarrierException;
import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
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
 *   <li>The boot-completion latch ({@link CyclicBarrier} with 2 parties)
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
     * Two-party barrier: one party is the UI thread in
     * {@link Render2Activity#showBootingProcedure()} (waiting via
     * {@link #waitBoot(long, TimeUnit)}), the other is the socket-worker
     * thread that called {@link #markCompleted()}.
     *
     * <p>Using a {@link CyclicBarrier} (rather than a
     * {@link java.util.concurrent.CountDownLatch}) means both sides
     * rendezvous — the worker doesn't release the UI and race ahead to
     * do post-boot work before the UI has actually woken up and hidden
     * the loading screen. This matches Nogitsune's BootStatus behaviour.
     */
    private final CyclicBarrier mBootLatch = new CyclicBarrier(2);

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
     * <p>The 60 s {@link #mBootLatch} await is bounded so that a worker
     * thread isn't pinned forever if the UI never shows up to rendezvous
     * (e.g. the activity was destroyed before reaching
     * {@link Render2Activity#showBootingProcedure()}).
     */
    public void markCompleted() {
        if (!mCompleted.compareAndSet(false, true)) {
            // Already marked — duplicate BOOT_COMPLETED (e.g. guest sent
            // to both sockets, or sent twice). Harmless.
            FileLogger.boot("boot_completed_duplicate", "ignored (already marked)");
            return;
        }
        FileLogger.boot("boot_completed_marked", "rendezvous with UI (60s)");

        // Notify TwoyiStatusManager so its switchOs() / isStarted() keep
        // working. markStarted() is now a plain setter (the boot latch
        // lives here), so this can't block.
        TwoyiStatusManager.getInstance().markStarted();

        try {
            // Rendezvous with the UI thread waiting in waitBoot().
            // 60 s mirrors the UI's wait window — if the UI never shows
            // up (activity destroyed before showBootingProcedure), we
            // don't pin this worker thread forever.
            mBootLatch.await(BOOT_TIMEOUT_SECONDS, TimeUnit.SECONDS);
        } catch (BrokenBarrierException e) {
            // Barrier was reset() (e.g. UI re-launched) — expected during
            // a re-boot, not worth tracking as an error.
            LogEvents.trackError(e);
        } catch (InterruptedException e) {
            // Restore the interrupt flag so the executor worker shuts
            // down cleanly if the JVM is winding down.
            Thread.currentThread().interrupt();
            LogEvents.trackError(e);
        } catch (TimeoutException e) {
            // UI never showed up to rendezvous. The guest has still
            // booted, so we just track and move on rather than blocking.
            LogEvents.trackError(e);
        }
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
            mBootLatch.await(timeout, unit);
            FileLogger.boot("wait_boot_success", null);
            return true;
        } catch (TimeoutException e) {
            FileLogger.boot("wait_boot_timeout", null);
            return false;
        } catch (BrokenBarrierException e) {
            // Barrier was reset() out from under us — treat as "not
            // booted yet" so the caller falls through to the boot-failure
            // path (which is the safer default for a re-launch race).
            FileLogger.boot("wait_boot_broken_barrier", null);
            LogEvents.trackError(e);
            return false;
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
        mCompleted.set(false);
        mBootLatch.reset();
    }
}
