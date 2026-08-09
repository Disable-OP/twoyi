/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi;

import android.content.Context;
import android.content.Intent;

import java.util.concurrent.atomic.AtomicBoolean;

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

    /**
     * Returns {@code true} if the guest has signalled
     * {@code BOOT_COMPLETED} for the current boot cycle.
     *
     * <p>Set by {@link #markStarted()}, which is invoked by
     * {@link BootCompletionServer#markCompleted()} — i.e. the boot latch
     * itself now lives in {@link BootCompletionServer} (ported from
     * cyanmint/Nogitsune's {@code BootStatus.kt}). This class retains
     * only the {@code mStarted}/{@code mShown} flags that
     * {@link #switchOs(Context)} needs to decide whether to bring the
     * guest UI forward.
     */
    public static TwoyiStatusManager getInstance() {
        return INSTANCE;
    }

    public void updateVisibility(boolean visible) {
        mShown.set(visible);
    }

    /**
     * Marks the guest as started. Called by
     * {@link BootCompletionServer#markCompleted()} after the guest sends
     * {@code BOOT_COMPLETED}.
     *
     * <p>This method used to also await a {@link java.util.concurrent.CyclicBarrier}
     * to rendezvous with the UI thread waiting in {@code waitBoot()}.
     * That barrier has been moved to {@link BootCompletionServer} so the
     * boot-completion state and the boot latch live together in one
     * focused class (mirroring Nogitsune's {@code BootStatus.kt}). This
     * method is now a plain setter — the rendezvous happens in
     * {@link BootCompletionServer#markCompleted()}.
     *
     * <p>Idempotent: setting {@code true} on an already-{@code true}
     * flag is a no-op.
     */
    public void markStarted() {
        mStarted.set(true);
    }

    public boolean isStarted() {
        return mStarted.get();
    }

    /**
     * Resets the {@code mStarted} flag for a new boot cycle.
     *
     * <p>Note: the boot latch (formerly here) now lives in
     * {@link BootCompletionServer#reset()} — callers that want a full
     * reset (e.g. {@link Render2Activity#onCreate(android.os.Bundle)})
     * must call both {@link BootCompletionServer#reset()} and this
     * method.
     */
    public void reset() {
        mStarted.set(false);
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
