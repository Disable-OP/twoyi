package com.android.vmcore.ui;

import android.content.Context;
import android.graphics.Outline;
import android.graphics.PointF;
import android.graphics.Rect;
import android.os.Build;
import android.util.AttributeSet;
import android.view.MotionEvent;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.View;
import android.view.ViewOutlineProvider;
import android.widget.FrameLayout;
import com.android.vmcore.RunnableC1621WWWWWWWW;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import com.android.vmcore.VMResConfig;
import com.android.vmcore.hal.DisplayService;
import com.android.vmcore.hal.InputService;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.pack200.PackingOptions;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class VMSurfaceView extends FrameLayout implements SurfaceHolder.Callback, View.OnTouchListener {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public boolean f9284WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public SurfaceView f9285WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public boolean f9286WWWWWWWW;

    /* renamed from: WWWWཀྵWWWWࠤཀྵ  reason: contains not printable characters */
    public float f9287WWWWWWWW;

    /* renamed from: WWWWၗWWWW३ၗ  reason: contains not printable characters */
    public final PointF f9288WWWWWWWW;

    /* renamed from: WWWoၙWWWo੮ၙ  reason: contains not printable characters */
    public OnVMSurfaceSizeListener f9289WWWoWWWo;

    /* renamed from: WWoၑWWoړၑ  reason: contains not printable characters */
    public boolean f9290WWoWWo;

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public int f9291WWWW;

    /* renamed from: WWၚஊၚ  reason: contains not printable characters */
    public TouchEventHandler f9292WW;

    /* renamed from: WoოWo੍ო  reason: contains not printable characters */
    public VMInstance f9293WoWo;

    /* loaded from: classes.dex */
    public interface OnVMSurfaceSizeListener {
        /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
        void mo5236WWWWWWWW(int i10, int i11);
    }

    /* loaded from: classes.dex */
    public interface TouchEventHandler {
        /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
        void mo5237WWWWWWWW(VMInstance vMInstance, int i10, int i11, long j10, float f10, float f11);
    }

    static {
        StringFog.f8859WWWWWWWW.getClass();
        WWWWWWWW.m17835WWWWWWWW(new byte[]{-24, -39, 32, 80, 72, -120, -75, 18, -37, -62, 26, 64, TarConstants.LF_MULTIVOLUME}, new byte[]{-66, -108, 115, 37, 58, -18, -44, 113});
    }

    public VMSurfaceView(Context context) {
        super(context);
        this.f9284WWWWWWWWWW = true;
        this.f9286WWWWWWWW = false;
        this.f9291WWWW = 0;
        this.f9287WWWWWWWW = 1.0f;
        this.f9290WWoWWo = false;
        this.f9288WWWWWWWW = new PointF();
        m5233WWWWoWWWWo(context);
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final void m5233WWWWoWWWWo(Context context) {
        SurfaceView surfaceView = new SurfaceView(context);
        this.f9285WWWWoWWWWo = surfaceView;
        addView(surfaceView, new FrameLayout.LayoutParams(-1, -1));
        this.f9285WWWWoWWWWo.getHolder().addCallback(this);
        this.f9285WWWWoWWWWo.setOnTouchListener(this);
        setKeepScreenOn(true);
        setBackgroundColor(-16777216);
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final void m5234WWWWWWWW(int i10, int i11, MotionEvent motionEvent) {
        long eventTime;
        int width = this.f9285WWWWoWWWWo.getWidth();
        int height = this.f9285WWWWoWWWWo.getHeight();
        float f10 = this.f9287WWWWWWWW;
        int i12 = this.f9291WWWW;
        int pointerId = motionEvent.getPointerId(i11);
        float x2 = motionEvent.getX(i11);
        float y10 = motionEvent.getY(i11);
        if (Build.VERSION.SDK_INT >= 34) {
            eventTime = motionEvent.getEventTimeNanos();
        } else {
            eventTime = motionEvent.getEventTime() * PackingOptions.SEGMENT_LIMIT;
        }
        PointF pointF = this.f9288WWWWWWWW;
        pointF.set(x2, y10);
        float f11 = pointF.x;
        float f12 = pointF.y;
        if (i12 != 90) {
            if (i12 != 180) {
                if (i12 == 270) {
                    pointF.x = f12;
                    pointF.y = width - f11;
                }
            } else {
                pointF.x = width - f11;
                pointF.y = height - f12;
            }
        } else {
            pointF.x = height - f12;
            pointF.y = f11;
        }
        float f13 = pointF.x / f10;
        pointF.x = f13;
        float f14 = pointF.y / f10;
        pointF.y = f14;
        TouchEventHandler touchEventHandler = this.f9292WW;
        if (touchEventHandler != null) {
            touchEventHandler.mo5237WWWWWWWW(this.f9293WoWo, i10, pointerId, eventTime, f13, f14);
            return;
        }
        VMInstance vMInstance = this.f9293WoWo;
        if (vMInstance.f8941WWoWWo == null) {
            vMInstance.f8941WWoWWo = new InputService(vMInstance);
        }
        vMInstance.f8941WWoWWo.m5131WWWWWWWW(i10, pointerId, eventTime, pointF.x, pointF.y);
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final void m5235WWWoWWWo(SurfaceHolder surfaceHolder) {
        Surface surface;
        if (!this.f9290WWoWWo && surfaceHolder != null && (surface = surfaceHolder.getSurface()) != null && surface.isValid()) {
            int width = this.f9285WWWWoWWWWo.getWidth();
            int height = this.f9285WWWWoWWWWo.getHeight();
            int i10 = this.f9291WWWW;
            VMInstance vMInstance = this.f9293WoWo;
            if (vMInstance.f8945WoWo == null) {
                vMInstance.f8945WoWo = new DisplayService(vMInstance);
            }
            vMInstance.f8945WoWo.m5126WWWWoWWWWo(hashCode());
            VMInstance vMInstance2 = this.f9293WoWo;
            if (vMInstance2.f8945WoWo == null) {
                vMInstance2.f8945WoWo = new DisplayService(vMInstance2);
            }
            vMInstance2.f8945WoWo.m5127WWWWWWWW(hashCode(), surface, width, height, i10);
        }
    }

    public SurfaceView getSurfaceView() {
        return this.f9285WWWWoWWWWo;
    }

    public VMInstance getVM() {
        return this.f9293WoWo;
    }

    /* JADX WARN: Removed duplicated region for block: B:19:0x00b3  */
    /* JADX WARN: Removed duplicated region for block: B:23:? A[RETURN, SYNTHETIC] */
    @Override // android.widget.FrameLayout, android.view.View
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final void onMeasure(int i10, int i11) {
        float min;
        int i12;
        int i13;
        int i14;
        int size = View.MeasureSpec.getSize(i10);
        int size2 = View.MeasureSpec.getSize(i11);
        if (size != 0 && size2 != 0) {
            VMResConfig m5061WWWWWWWW = this.f9293WoWo.m5061WWWWWWWW();
            int i15 = 0;
            if (this.f9286WWWWWWWW) {
                int i16 = m5061WWWWWWWW.f8955WWWoWWWo;
                int i17 = m5061WWWWWWWW.f8952WWWWoWWWWo;
                if (i16 <= i17) {
                    min = Math.min((size * 1.0f) / i17, (size2 * 1.0f) / i16);
                    i12 = (int) (m5061WWWWWWWW.f8952WWWWoWWWWo * min);
                    i14 = m5061WWWWWWWW.f8955WWWoWWWo;
                    i13 = (int) (i14 * min);
                    FrameLayout.LayoutParams layoutParams = (FrameLayout.LayoutParams) this.f9285WWWWoWWWWo.getLayoutParams();
                    int i18 = (size - i12) / 2;
                    layoutParams.leftMargin = i18;
                    layoutParams.rightMargin = i18;
                    int i19 = (size2 - i13) / 2;
                    layoutParams.topMargin = i19;
                    layoutParams.bottomMargin = i19;
                    this.f9287WWWWWWWW = min;
                    this.f9291WWWW = i15;
                    super.onMeasure(i10, i11);
                    if (this.f9289WWWoWWWo == null) {
                        post(new RunnableC1621WWWWWWWW(this, i12, i13, 1));
                        return;
                    }
                    return;
                }
                min = Math.min((size * 1.0f) / i16, (size2 * 1.0f) / i17);
                i12 = (int) (m5061WWWWWWWW.f8955WWWoWWWo * min);
                i13 = (int) (m5061WWWWWWWW.f8952WWWWoWWWWo * min);
                i15 = 90;
                FrameLayout.LayoutParams layoutParams2 = (FrameLayout.LayoutParams) this.f9285WWWWoWWWWo.getLayoutParams();
                int i182 = (size - i12) / 2;
                layoutParams2.leftMargin = i182;
                layoutParams2.rightMargin = i182;
                int i192 = (size2 - i13) / 2;
                layoutParams2.topMargin = i192;
                layoutParams2.bottomMargin = i192;
                this.f9287WWWWWWWW = min;
                this.f9291WWWW = i15;
                super.onMeasure(i10, i11);
                if (this.f9289WWWoWWWo == null) {
                }
            } else {
                int i20 = m5061WWWWWWWW.f8952WWWWoWWWWo;
                int i21 = m5061WWWWWWWW.f8955WWWoWWWo;
                if (i20 <= i21) {
                    min = Math.min((size * 1.0f) / i20, (size2 * 1.0f) / i21);
                    i12 = (int) (m5061WWWWWWWW.f8952WWWWoWWWWo * min);
                    i14 = m5061WWWWWWWW.f8955WWWoWWWo;
                    i13 = (int) (i14 * min);
                    FrameLayout.LayoutParams layoutParams22 = (FrameLayout.LayoutParams) this.f9285WWWWoWWWWo.getLayoutParams();
                    int i1822 = (size - i12) / 2;
                    layoutParams22.leftMargin = i1822;
                    layoutParams22.rightMargin = i1822;
                    int i1922 = (size2 - i13) / 2;
                    layoutParams22.topMargin = i1922;
                    layoutParams22.bottomMargin = i1922;
                    this.f9287WWWWWWWW = min;
                    this.f9291WWWW = i15;
                    super.onMeasure(i10, i11);
                    if (this.f9289WWWoWWWo == null) {
                    }
                } else {
                    min = Math.min((size * 1.0f) / i21, (size2 * 1.0f) / i20);
                    i12 = (int) (m5061WWWWWWWW.f8955WWWoWWWo * min);
                    i13 = (int) (m5061WWWWWWWW.f8952WWWWoWWWWo * min);
                    i15 = 270;
                    FrameLayout.LayoutParams layoutParams222 = (FrameLayout.LayoutParams) this.f9285WWWWoWWWWo.getLayoutParams();
                    int i18222 = (size - i12) / 2;
                    layoutParams222.leftMargin = i18222;
                    layoutParams222.rightMargin = i18222;
                    int i19222 = (size2 - i13) / 2;
                    layoutParams222.topMargin = i19222;
                    layoutParams222.bottomMargin = i19222;
                    this.f9287WWWWWWWW = min;
                    this.f9291WWWW = i15;
                    super.onMeasure(i10, i11);
                    if (this.f9289WWWoWWWo == null) {
                    }
                }
            }
        } else {
            super.onMeasure(i10, i11);
        }
    }

    @Override // android.view.View.OnTouchListener
    public final boolean onTouch(View view, MotionEvent motionEvent) {
        if (!this.f9284WWWWWWWWWW) {
            return super.onTouchEvent(motionEvent);
        }
        int actionMasked = motionEvent.getActionMasked();
        if (actionMasked == 2) {
            for (int i10 = 0; i10 < motionEvent.getPointerCount(); i10++) {
                m5234WWWWWWWW(actionMasked, i10, motionEvent);
            }
            return true;
        }
        m5234WWWWWWWW(actionMasked, motionEvent.getActionIndex(), motionEvent);
        return true;
    }

    @Override // android.view.View
    public final void onWindowFocusChanged(boolean z10) {
        super.onWindowFocusChanged(z10);
        synchronized (VMSurfaceView.class) {
            if (z10) {
                try {
                    SurfaceHolder holder = this.f9285WWWWoWWWWo.getHolder();
                    StringFog.f8859WWWWWWWW.getClass();
                    WWWWWWWW.m17835WWWWWWWW(new byte[]{71, Byte.MIN_VALUE, 21, -15, -28, -126, 68, -94, 110, -127, 33, -19, -7, -91, 67, -76, 70, -119, 39, -4}, new byte[]{40, -18, 66, -104, -118, -26, 43, -43});
                    m5235WWWoWWWo(holder);
                } catch (Throwable th2) {
                    throw th2;
                }
            }
        }
    }

    public void setCornerRadius(final float f10) {
        setOutlineProvider(new ViewOutlineProvider() { // from class: com.android.vmcore.ui.VMSurfaceView.1
            @Override // android.view.ViewOutlineProvider
            public final void getOutline(View view, Outline outline) {
                VMSurfaceView vMSurfaceView = VMSurfaceView.this;
                outline.setRoundRect(new Rect(0, 0, vMSurfaceView.getWidth(), vMSurfaceView.getHeight()), f10);
            }
        });
        setClipToOutline(true);
    }

    public void setLandscape(boolean z10) {
        this.f9286WWWWWWWW = z10;
        this.f9285WWWWoWWWWo.getHolder().setSizeFromLayout();
        requestLayout();
    }

    public void setOnVMSurfaceSizeListener(OnVMSurfaceSizeListener onVMSurfaceSizeListener) {
        this.f9289WWWoWWWo = onVMSurfaceSizeListener;
    }

    public void setScaling(boolean z10) {
        if (this.f9290WWoWWo && !z10) {
            this.f9290WWoWWo = false;
            this.f9285WWWWoWWWWo.getHolder().setFixedSize(getMeasuredWidth(), getMeasuredHeight());
            synchronized (VMSurfaceView.class) {
                StringFog.f8859WWWWWWWW.getClass();
                WWWWWWWW.m17835WWWWWWWW(new byte[]{-11, 124, 117, -93, 117, -39, -102, 65, -24, 126}, new byte[]{-122, 25, 1, -16, 22, -72, -10, 40});
                m5235WWWoWWWo(this.f9285WWWWoWWWWo.getHolder());
            }
            requestLayout();
            return;
        }
        this.f9290WWoWWo = z10;
        this.f9285WWWWoWWWWo.getHolder().setSizeFromLayout();
    }

    public void setTouchEnabled(boolean z10) {
        this.f9284WWWWWWWWWW = z10;
    }

    public void setTouchEventHandler(TouchEventHandler touchEventHandler) {
        this.f9292WW = touchEventHandler;
    }

    public void setVM(VMInstance vMInstance) {
        this.f9293WoWo = vMInstance;
        requestLayout();
    }

    @Override // android.view.View
    public void setVisibility(int i10) {
        super.setVisibility(i10);
        this.f9285WWWWoWWWWo.setVisibility(i10);
    }

    @Override // android.view.SurfaceHolder.Callback
    public final void surfaceChanged(SurfaceHolder surfaceHolder, int i10, int i11, int i12) {
        synchronized (VMSurfaceView.class) {
            StringFog.f8859WWWWWWWW.getClass();
            WWWWWWWW.m17835WWWWWWWW(new byte[]{-18, 73, -115, 92, 19, -107, -72, -6, -11, 93, -111, 93, 23, -110}, new byte[]{-99, 60, -1, 58, 114, -10, -35, -71});
            m5235WWWoWWWo(surfaceHolder);
        }
    }

    @Override // android.view.SurfaceHolder.Callback
    public final void surfaceCreated(SurfaceHolder surfaceHolder) {
        synchronized (VMSurfaceView.class) {
            StringFog.f8859WWWWWWWW.getClass();
            WWWWWWWW.m17835WWWWWWWW(new byte[]{-116, 14, -46, -38, 64, -94, 42, TarConstants.LF_GNUTYPE_LONGNAME, -115, 30, -63, -56, 68, -91}, new byte[]{-1, 123, -96, -68, 33, -63, 79, 15});
            m5235WWWoWWWo(surfaceHolder);
        }
    }

    @Override // android.view.SurfaceHolder.Callback
    public final void surfaceDestroyed(SurfaceHolder surfaceHolder) {
        synchronized (VMSurfaceView.class) {
            VMInstance vMInstance = this.f9293WoWo;
            if (vMInstance.f8945WoWo == null) {
                vMInstance.f8945WoWo = new DisplayService(vMInstance);
            }
            vMInstance.f8945WoWo.m5126WWWWoWWWWo(hashCode());
        }
    }

    public VMSurfaceView(Context context, AttributeSet attributeSet) {
        super(context, attributeSet);
        this.f9284WWWWWWWWWW = true;
        this.f9286WWWWWWWW = false;
        this.f9291WWWW = 0;
        this.f9287WWWWWWWW = 1.0f;
        this.f9290WWoWWo = false;
        this.f9288WWWWWWWW = new PointF();
        m5233WWWWoWWWWo(context);
    }

    public VMSurfaceView(Context context, AttributeSet attributeSet, int i10) {
        super(context, attributeSet, i10);
        this.f9284WWWWWWWWWW = true;
        this.f9286WWWWWWWW = false;
        this.f9291WWWW = 0;
        this.f9287WWWWWWWW = 1.0f;
        this.f9290WWoWWo = false;
        this.f9288WWWWWWWW = new PointF();
        m5233WWWWoWWWWo(context);
    }
}
