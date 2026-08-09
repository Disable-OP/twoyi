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
import android.graphics.RectF;
import android.os.SystemClock;
import android.util.AttributeSet;
import android.view.TextureView;
import android.view.View;

import androidx.annotation.NonNull;
import androidx.annotation.Nullable;

/**
 * A host-side boot animation view that mimics the classic Android boot
 * animation (spinning ring with pulsing dots).
 *
 * <p>This is a FAKE boot animation — it does NOT play the guest's
 * bootanimation.zip. It provides visual feedback to the user while the
 * guest Android system boots underneath.
 *
 * <p>The real guest bootanimation requires the full SurfaceFlinger + HWC +
 * gralloc + emugl pipeline to be running, which is a heavy stack. This
 * host-side animation is a pragmatic alternative that shows immediately
 * when the container starts.
 *
 * <p>The animation consists of:
 * <ul>
 *   <li>A spinning ring of dots (like the Android boot spinner)</li>
 *   <li>A pulsing center logo</li>
 *   <li>A progress text below</li>
 * </ul>
 *
 * <p>Architecture:
 * <ul>
 *   <li>Uses TextureView for hardware-accelerated rendering</li>
 *   <li>Render loop runs at 60fps on a background thread</li>
 *   <li>Stops automatically when detached or visibility changes</li>
 *   <li>Thread-safe — no locks needed (volatile flags only)</li>
 * </ul>
 *
 * @author Disable-OP
 * @date 2026/08/09.
 */
public class BootAnimationView extends TextureView implements TextureView.SurfaceTextureListener {

    private volatile boolean mRendering = false;
    private volatile boolean mBooted = false;

    // Animation state
    private float mRotation = 0f;
    private float mPulse = 0f;
    private long mStartTime = 0;

    // Paints (pre-allocated, reused every frame)
    private final Paint mDotPaint = new Paint();
    private final Paint mRingPaint = new Paint();
    private final Paint mCenterPaint = new Paint();
    private final Paint mTextPaint = new Paint();
    private final RectF mRingRect = new RectF();

    // Colors (Android-style)
    private static final int COLOR_BG = Color.BLACK;
    private static final int COLOR_DOT = Color.WHITE;
    private static final int COLOR_RING = 0xFFFFFFFF;
    private static final int COLOR_CENTER = 0xFF4CAF50; // Android green
    private static final int COLOR_TEXT = 0xCCFFFFFF;

    // Animation parameters
    private static final float ROTATION_SPEED = 180f; // degrees per second
    private static final float PULSE_SPEED = 2.5f; // cycles per second
    private static final int DOT_COUNT = 8;
    private static final float RING_RADIUS_RATIO = 0.35f; // fraction of min(width,height)
    private static final float DOT_RADIUS_RATIO = 0.04f;

    public BootAnimationView(@NonNull Context context) {
        this(context, null);
    }

    public BootAnimationView(@NonNull Context context, @Nullable AttributeSet attrs) {
        this(context, attrs, 0);
    }

    public BootAnimationView(@NonNull Context context, @Nullable AttributeSet attrs, int defStyleAttr) {
        super(context, attrs, defStyleAttr);
        init();
    }

    private void init() {
        setSurfaceTextureListener(this);

        mDotPaint.setAntiAlias(true);
        mDotPaint.setColor(COLOR_DOT);

        mRingPaint.setAntiAlias(true);
        mRingPaint.setColor(COLOR_RING);
        mRingPaint.setStyle(Paint.Style.STROKE);
        mRingPaint.setStrokeWidth(3f);

        mCenterPaint.setAntiAlias(true);
        mCenterPaint.setColor(COLOR_CENTER);

        mTextPaint.setAntiAlias(true);
        mTextPaint.setColor(COLOR_TEXT);
        mTextPaint.setTextSize(28f);
        mTextPaint.setTextAlign(Paint.Align.CENTER);
    }

    @Override
    protected void onAttachedToWindow() {
        super.onAttachedToWindow();
        mRendering = true;
    }

    @Override
    protected void onDetachedFromWindow() {
        super.onDetachedFromWindow();
        mRendering = false;
    }

    @Override
    protected void onVisibilityChanged(@NonNull View changedView, int visibility) {
        super.onVisibilityChanged(changedView, visibility);
        mRendering = (visibility == VISIBLE);
    }

    @Override
    public void onSurfaceTextureAvailable(@NonNull android.graphics.SurfaceTexture surface, int width, int height) {
        mStartTime = SystemClock.elapsedRealtime();
        new Thread(() -> {
            while (mRendering) {
                renderFrame();
                SystemClock.sleep(16); // ~60fps
            }
        }, "BootAnimation-Render").start();
    }

    @Override
    public void onSurfaceTextureSizeChanged(@NonNull android.graphics.SurfaceTexture surface, int width, int height) {
    }

    @Override
    public boolean onSurfaceTextureDestroyed(@NonNull android.graphics.SurfaceTexture surface) {
        mRendering = false;
        return true;
    }

    @Override
    public void onSurfaceTextureUpdated(@NonNull android.graphics.SurfaceTexture surface) {
    }

    private void renderFrame() {
        Canvas canvas = null;
        try {
            canvas = lockCanvas();
            if (canvas == null) return;

            int width = getWidth();
            int height = getHeight();
            if (width <= 0 || height <= 0) return;

            // Clear background
            canvas.drawColor(COLOR_BG, PorterDuff.Mode.CLEAR);
            canvas.drawColor(COLOR_BG);

            // Update animation state
            long now = SystemClock.elapsedRealtime();
            float elapsed = (now - mStartTime) / 1000f;
            mRotation = (elapsed * ROTATION_SPEED) % 360f;
            mPulse = 0.5f + 0.5f * (float) Math.sin(elapsed * PULSE_SPEED * 2 * Math.PI);

            float cx = width / 2f;
            float cy = height / 2f;
            float minDim = Math.min(width, height);
            float ringRadius = minDim * RING_RADIUS_RATIO;
            float dotRadius = minDim * DOT_RADIUS_RATIO;

            // Draw spinning dots ring
            for (int i = 0; i < DOT_COUNT; i++) {
                float angle = mRotation + (360f / DOT_COUNT) * i;
                float rad = (float) Math.toRadians(angle - 90); // -90 so 0° is top
                float x = cx + ringRadius * (float) Math.cos(rad);
                float y = cy + ringRadius * (float) Math.sin(rad);

                // Fade dots based on position in the ring (trailing fade)
                float dotAlpha = 1f - (float) i / DOT_COUNT;
                int alpha = (int) (255 * dotAlpha * (0.3f + 0.7f * mPulse));
                mDotPaint.setAlpha(alpha);
                canvas.drawCircle(x, y, dotRadius, mDotPaint);
            }

            // Draw outer ring (subtle)
            mRingRect.set(cx - ringRadius - dotRadius, cy - ringRadius - dotRadius,
                          cx + ringRadius + dotRadius, cy + ringRadius + dotRadius);
            mRingPaint.setAlpha(40);
            canvas.drawArc(mRingRect, 0, 360, false, mRingPaint);

            // Draw pulsing center logo (Android robot head silhouette)
            float centerRadius = dotRadius * 2.5f * (0.8f + 0.2f * mPulse);
            mCenterPaint.setAlpha(200);
            // Draw two "antennas" and a semi-circle head (simplified Android logo)
            canvas.drawCircle(cx, cy - centerRadius * 0.3f, centerRadius, mCenterPaint);
            // Antennas
            float antOffset = centerRadius * 0.4f;
            float antLen = centerRadius * 0.5f;
            mCenterPaint.setStrokeWidth(centerRadius * 0.15f);
            mCenterPaint.setStyle(Paint.Style.STROKE);
            canvas.drawLine(cx - antOffset, cy - centerRadius * 0.8f,
                           cx - antOffset * 1.2f, cy - centerRadius * 0.8f - antLen, mCenterPaint);
            canvas.drawLine(cx + antOffset, cy - centerRadius * 0.8f,
                           cx + antOffset * 1.2f, cy - centerRadius * 0.8f - antLen, mCenterPaint);
            mCenterPaint.setStyle(Paint.Style.FILL);

            // Draw progress text
            String progressText;
            if (mBooted) {
                progressText = "Booted!";
            } else if (elapsed < 5) {
                progressText = "Starting...";
            } else if (elapsed < 15) {
                progressText = "Loading system...";
            } else if (elapsed < 30) {
                progressText = "Almost there...";
            } else {
                progressText = "Booting (" + (int) elapsed + "s)";
            }
            canvas.drawText(progressText, cx, cy + ringRadius + dotRadius * 4, mTextPaint);

        } finally {
            if (canvas != null) {
                unlockCanvasAndPost(canvas);
            }
        }
    }

    /**
     * Notifies the animation that boot has completed.
     * The animation will show "Booted!" briefly before stopping.
     */
    public void setBooted(boolean booted) {
        mBooted = booted;
    }
}
