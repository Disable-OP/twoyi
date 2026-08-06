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
import android.content.Intent;

import java.util.concurrent.BrokenBarrierException;
import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicBoolean;

import io.twoyi.utils.LogEvents;

/**
 * @author weishu
 * @date 2021/10/27.
 */

public class TwoyiStatusManager {

    private static final TwoyiStatusManager INSTANCE = new TwoyiStatusManager();
    private TwoyiStatusManager() {
    }

    private final AtomicBoolean mStarted = new AtomicBoolean(false);
    private final AtomicBoolean mShown = new AtomicBoolean(false);

    private final CyclicBarrier mBootLatch = new CyclicBarrier(2);

    public static TwoyiStatusManager getInstance() {
        return INSTANCE;
    }

    public void updateVisibility(boolean visible) {
        mShown.set(visible);
    }

    public void markStarted() {
        if (mStarted.compareAndSet(false, true)) {
            try {
                // Fixed: previously this called await() with no timeout. The UI
                // side (Render2Activity.showBootingProcedure) calls waitBoot()
                // with a 60 s timeout, so if it times out and reset() is later
                // invoked (e.g. the activity is re-launched), a late
                // BOOT_COMPLETED message would block the socket-server worker
                // thread forever waiting for a UI party that may never arrive.
                // Bounding the wait keeps the backend responsive on slow/failed
                // boots. 60 s mirrors the UI's wait window.
                mBootLatch.await(60, TimeUnit.SECONDS);
            } catch (BrokenBarrierException e) {
                // Barrier was reset() (e.g. UI re-launched) — expected during
                // a re-boot, not an error worth tracking.
                LogEvents.trackError(e);
            } catch (InterruptedException e) {
                // Fixed: catching InterruptedException clears the thread's
                // interrupt flag. If we don't restore it, the executor worker
                // that called us keeps running as if nothing happened, which
                // can suppress shutdown hooks and other interrupt-driven
                // cancellation in callers up the stack.
                Thread.currentThread().interrupt();
                LogEvents.trackError(e);
            } catch (TimeoutException e) {
                // UI never showed up to rendezvous (activity destroyed before
                // reaching waitBoot, or a reset() race). The guest has still
                // booted, so we just track and move on rather than blocking.
                LogEvents.trackError(e);
            }
        }
    }

    public boolean isStarted() {
        return mStarted.get();
    }

    public void reset() {
        mStarted.set(false);
        mBootLatch.reset();
    }

    public boolean waitBoot(long timeout, TimeUnit unit) throws InterruptedException, BrokenBarrierException {
        try {
            mBootLatch.await(timeout, unit);
            return true;
        } catch (TimeoutException e) {
            return false;
        }
    }

    public synchronized void switchOs(Context context) {
        if (!mStarted.get()) {
            return;
        }

        // Fixed: use synchronized to make the check-then-act atomic.
        // Previously, two concurrent calls could both observe the same
        // mShown value and both launch the same activity, then both
        // flip the state, leaving it inverted from reality.
        //
        // Also fixed: capture mShown into a local once and flip it via
        // compareAndSet on the SAME value, rather than re-reading
        // mShown.get() at the end. The original two-step read
        // (if (mShown.get()) ... mShown.set(!mShown.get())) is a TOCTOU
        // race: another thread calling updateVisibility() between the
        // two reads could change mShown, and we'd then write back the
        // INVERTED value of what we just observed, losing the visibility
        // update from updateVisibility().
        boolean currentlyShown = mShown.get();
        Intent intent;
        if (currentlyShown) {
            intent = new Intent(Intent.ACTION_MAIN);
            intent.addCategory(Intent.CATEGORY_HOME);
        } else {
            intent = new Intent(context, Render2Activity.class);
        }
        intent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP);

        context.startActivity(intent);
        // Atomically flip the bit IF no other thread changed it under us.
        // If updateVisibility() raced in between, the CAS fails harmlessly
        // and the new visibility state wins through.
        mShown.compareAndSet(currentlyShown, !currentlyShown);
    }
}
