/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.PorterDuff;
import android.graphics.SurfaceTexture;
import android.os.SystemClock;
import android.text.TextUtils;
import android.util.AttributeSet;
import android.util.SparseArray;
import android.util.SparseIntArray;
import android.view.TextureView;
import android.view.View;

import androidx.annotation.NonNull;
import androidx.annotation.Nullable;

import com.topjohnwu.superuser.CallbackList;
import com.topjohnwu.superuser.Shell;

import java.util.LinkedList;
import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

import io.twoyi.utils.ShellUtil;

/**
 * @author weishu
 * @date 2022/1/1.
 */

public class BootLogTexture extends TextureView implements TextureView.SurfaceTextureListener {

    private final AtomicBoolean mRendering = new AtomicBoolean(false);

    /** Bumped on every onSurfaceTextureAvailable — see the loop-guard note there. */
    private final java.util.concurrent.atomic.AtomicInteger mSurfaceGeneration =
            new java.util.concurrent.atomic.AtomicInteger(0);

    // LimitedQueue is backed by java.util.concurrent.ConcurrentLinkedQueue, so
    // mutations and iterations are lock-free and thread-safe. The render loop
    // reads from this queue without holding any monitor — the previous
    // "synchronized (mLogMessages)" block on the read path is gone, which
    // removes contention between the logcat pump thread (writer) and the
    // render loop (reader) at 60 fps.
    private final LimitedQueue<String> mLogMessages = new LimitedQueue<>(160);
    private final LinkedList<String> mSnapShot = new LinkedList<>();

    /**
     * Set by the logcat callback when a new line arrives, cleared by the
     * render loop after the snapshot is refreshed. Avoids copying the full
     * 160-message buffer every frame (60 fps × 160 strings = ~10k copies/sec)
     * when nothing has changed.
     */
    private volatile boolean mSnapshotDirty = false;

    private final SparseArray<Paint> mPaints = new SparseArray<>();
    private final Paint mDefaultPaint = new Paint();

    private static final SparseIntArray COLOR_MAP = new SparseIntArray();

    static {
        COLOR_MAP.put('V', 0xBBBBBB);
        COLOR_MAP.put('D', 0x5EBB1E);
        COLOR_MAP.put('I', 0x4CBBA2);
        COLOR_MAP.put('W', 0xFFD21C);
        COLOR_MAP.put('E', 0xFF6B68);
        COLOR_MAP.put('F', Color.RED);
        COLOR_MAP.put('S', Color.WHITE);

//        COLOR_MAP.put('V', 0xFFFFFF);
//        COLOR_MAP.put('D', 0x5FAFFE);
//        COLOR_MAP.put('I', 0x02D701);
//        COLOR_MAP.put('W', 0xD75E02);
//        COLOR_MAP.put('E', 0xFF2600);
//        COLOR_MAP.put('F', 0xFF2600);
//        COLOR_MAP.put('S', Color.WHITE);
    }

    public BootLogTexture(@NonNull Context context) {
        this(context, null);
    }

    public BootLogTexture(@NonNull Context context, @Nullable AttributeSet attrs) {
        this(context, attrs, 0);
    }

    public BootLogTexture(@NonNull Context context, @Nullable AttributeSet attrs, int defStyleAttr) {
        this(context, attrs, defStyleAttr, 0);
    }

    public BootLogTexture(@NonNull Context context, @Nullable AttributeSet attrs, int defStyleAttr, int defStyleRes) {
        super(context, attrs, defStyleAttr, defStyleRes);
        init(context);
    }

    private void init(Context context) {
        setSurfaceTextureListener(this);

        for (int i = 0; i < COLOR_MAP.size(); i++) {
            int key = COLOR_MAP.keyAt(i);
            int value = COLOR_MAP.valueAt(i);

            Paint paint = new Paint();
            setPaint(paint, value);

            mPaints.put(key, paint);
        }

        setPaint(mDefaultPaint, Color.WHITE);
    }

    private void setPaint(Paint paint, int color) {
        paint.setColor(color);
        paint.setAntiAlias(true);
        paint.setTextSize(16);
        paint.setAlpha(128);
    }

    @Override
    protected void onAttachedToWindow() {
        super.onAttachedToWindow();
        mRendering.set(true);
    }

    @Override
    protected void onDetachedFromWindow() {
        super.onDetachedFromWindow();
        mRendering.set(false);
    }

    @Override
    protected void onVisibilityChanged(View changedView, int visibility) {
        super.onVisibilityChanged(changedView, visibility);

        mRendering.set(visibility == VISIBLE);
    }

    @Override
    public void onSurfaceTextureAvailable(@NonNull SurfaceTexture surface, int width, int height) {

        // 6-Z184: per-surface generation. A fast destroy/recreate cycle
        // used to leave the OLD render loop running: the shared mRendering
        // flag was set back to true by the new surface before the old
        // loop checked it, so two (or more) 60 fps loops + logcat shells
        // ran in parallel. Each loop now dies unless ITS generation is
        // still the current one.
        final int myGeneration = mSurfaceGeneration.incrementAndGet();

        Shell.EXECUTOR.execute(() -> {
            List<String> callbackList = new CallbackList<String>() {
                @Override
                public void onAddElement(String s) {
                    if (TextUtils.isEmpty(s)) {
                        return;
                    }
                    // LimitedQueue is backed by ConcurrentLinkedQueue, so the
                    // bounded add is already lock-free and thread-safe — no
                    // monitor needed on the writer side either.
                    mLogMessages.add(s);
                    // Mark the snapshot as dirty so the render loop knows it
                    // needs to rebuild mSnapShot from mLogMessages on the next
                    // frame. Without this flag, render() would copy up to 160
                    // strings into mSnapShot every 16 ms even when no new
                    // logcat output had arrived.
                    mSnapshotDirty = true;
                }
            };

            // 6-Z184: quote the filter spec and use the canonical
            // '*:I' form — the unquoted '*I' is glob-prone and parsed as
            // a TAG literally named '*I' by strict logcat parsers (no
            // output at all).
            Shell shell = ShellUtil.newSh();
            shell.newJob().add("timeout -s 9 30 logcat -v brief '*:I'").to(callbackList).submit();

            while (mRendering.get() && mSurfaceGeneration.get() == myGeneration) {
                render();
                SystemClock.sleep(16);
            }

            try {
                shell.waitAndClose(1, TimeUnit.SECONDS);
            } catch (Throwable ignored) {
            }

        });
    }

    @Override
    public void onSurfaceTextureSizeChanged(@NonNull SurfaceTexture surface, int width, int height) {

    }

    @Override
    public boolean onSurfaceTextureDestroyed(@NonNull SurfaceTexture surface) {
        mRendering.set(false);
        // Fixed: was returning false, which tells the framework NOT to release
        // the SurfaceTexture. Since we never manually release it, the native
        // texture and its buffers leak on every surface destroy (rotation,
        // backgrounding, etc.). Returning true lets the framework release it.
        return true;
    }

    @Override
    public void onSurfaceTextureUpdated(@NonNull SurfaceTexture surface) {

    }

    private void render() {
        Canvas canvas = null;
        try {
            canvas = lockCanvas();
            // Fixed: lockCanvas() returns null when the SurfaceTexture is not
            // yet available or has been destroyed (e.g. during a configuration
            // change or backgrounding). Dereferencing the canvas below would
            // NPE and crash the boot-log render loop.
            if (canvas == null) {
                return;
            }

            // clear canvas
            canvas.drawColor(Color.TRANSPARENT, PorterDuff.Mode.CLEAR);

            // Only rebuild the snapshot when new logcat lines have arrived
            // since the last frame. The previous implementation did
            // mSnapShot.clear() + addAll(mLogMessages) every frame (60 fps),
            // which copied up to 160 String references 60 times per second
            // even when the logcat stream was idle.
            if (mSnapshotDirty) {
                mSnapShot.clear();
                // Lock-free snapshot: LimitedQueue is a ConcurrentLinkedQueue
                // whose iterator is weakly consistent and never throws
                // ConcurrentModificationException. addAll() walks that
                // iterator without holding any monitor, so the logcat writer
                // thread is never blocked by the render loop.
                mSnapShot.addAll(mLogMessages);
                mSnapshotDirty = false;
            }

            int count = 0;
            for (String log : mSnapShot) {

                char chr = log.charAt(0);

                Paint paint = mPaints.get(chr);
                if (paint == null) {
                    paint = mDefaultPaint;
                }

                canvas.drawText(log, 0, count++ * 20, paint);
            }

        } finally {
            if (canvas != null) {
                unlockCanvasAndPost(canvas);
            }
        }

    }
}
