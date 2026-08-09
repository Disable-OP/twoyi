/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi;

import java.util.concurrent.ConcurrentLinkedQueue;

/**
 * Thread-safe bounded FIFO queue backed by {@link ConcurrentLinkedQueue}.
 *
 * <p>Unlike a synchronized {@link java.util.LinkedList}, this implementation
 * never blocks: writes use lock-free CAS primitives, and iterators returned by
 * this class are <em>weakly consistent</em> &mdash; they reflect the queue's
 * state at some point at or since the iterator was created and never throw
 * {@link java.util.ConcurrentModificationException}.
 *
 * <p>This lets a reader thread iterate the queue (e.g. via
 * {@code Collection#addAll(LimitedQueue)}) without taking a lock held by the
 * writer, which is exactly what the boot-log renderer in {@code BootLogTexture}
 * needs: the logcat pump thread keeps appending lines while the render loop
 * snapshots the queue every frame, all without either thread blocking the
 * other.
 *
 * <p>Size tracking is best-effort: under concurrent additions the queue may
 * briefly exceed the configured limit by a small number of entries before each
 * {@link #offer(Object)} call trims it back. The bound is therefore a soft
 * ceiling, not a hard one &mdash; acceptable for a rolling log buffer where a
 * few extra entries are harmless.
 */
public class LimitedQueue<E> extends ConcurrentLinkedQueue<E> {
    private static final long serialVersionUID = 1L;

    private final int mLimit;

    public LimitedQueue(int limit) {
        this.mLimit = limit;
    }

    @Override
    public boolean offer(E e) {
        if (!super.offer(e)) {
            return false;
        }
        // Trim oldest entries when over the limit. Lock-free; another thread
        // racing with us on poll() may cause us to observe size() <= limit
        // and exit early, which is fine — the bound is soft.
        while (super.size() > mLimit) {
            if (super.poll() == null) {
                break;
            }
        }
        return true;
    }
}
