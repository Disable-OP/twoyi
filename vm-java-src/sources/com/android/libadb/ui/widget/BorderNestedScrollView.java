package com.android.libadb.ui.widget;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.drawable.Drawable;
import android.util.AttributeSet;
import android.view.View;
import android.widget.FrameLayout;
import androidx.core.widget.NestedScrollView;
import com.clone.android.dual.space.R;
import r2.C3942WWWWWWWW;
import r2.InterfaceC3941WWWWWWWW;
import r2.WWWWWWWW;
import r2.WWWWoWWWWo;
import r2.WWWoWWWo;
/* loaded from: classes.dex */
public class BorderNestedScrollView extends NestedScrollView implements InterfaceC3941WWWWWWWW {

    /* renamed from: WWWWoᜑWWWWoเᜑ  reason: contains not printable characters */
    public final C3942WWWWWWWW f8300WWWWoWWWWo;

    public BorderNestedScrollView(Context context) {
        this(context, null);
    }

    private int getScrollRange() {
        if (getChildCount() <= 0) {
            return 0;
        }
        View childAt = getChildAt(0);
        FrameLayout.LayoutParams layoutParams = (FrameLayout.LayoutParams) childAt.getLayoutParams();
        return Math.max(0, ((childAt.getHeight() + layoutParams.topMargin) + layoutParams.bottomMargin) - ((getHeight() - getPaddingTop()) - getPaddingBottom()));
    }

    /* renamed from: WWWWമWWWWုമ  reason: contains not printable characters */
    public final void m4837WWWWWWWW() {
        boolean z10;
        boolean z11;
        boolean z12;
        int scrollY = getScrollY();
        int scrollRange = getScrollRange();
        if (scrollRange != 0) {
            boolean z13 = false;
            if (scrollY == 0) {
                z10 = true;
            } else {
                z10 = false;
            }
            if (scrollY == scrollRange) {
                z11 = true;
            } else {
                z11 = false;
            }
            WWWWoWWWWo borderTopVisibility = getBorderTopVisibility();
            WWWWoWWWWo wWWWoWWWWo = WWWWoWWWWo.f32834WWWW;
            if (borderTopVisibility != wWWWoWWWWo && ((getBorderTopVisibility() != WWWWoWWWWo.f32830WWWWWWWWWW || !z10) && (getBorderTopVisibility() != WWWWoWWWWo.f32832WWWWWWWW || z10))) {
                z12 = false;
            } else {
                z12 = true;
            }
            if (getBorderBottomVisibility() == wWWWoWWWWo || ((getBorderBottomVisibility() == WWWWoWWWWo.f32830WWWWWWWWWW && z11) || (getBorderBottomVisibility() == WWWWoWWWWo.f32832WWWWWWWW && !z11))) {
                z13 = true;
            }
            if (!Boolean.valueOf(getBorderViewDelegate().m16622WWWWoWWWWo()).equals(Boolean.valueOf(z12)) || !Boolean.valueOf(getBorderViewDelegate().m16623WWWWWWWW()).equals(Boolean.valueOf(z13))) {
                getBorderViewDelegate().m16622WWWWoWWWWo();
                getBorderViewDelegate().m16623WWWWWWWW();
                C3942WWWWWWWW borderViewDelegate = getBorderViewDelegate();
                borderViewDelegate.f32845WWWoWWWo = Boolean.valueOf(z12);
                borderViewDelegate.f32840WWWWWWWW = Boolean.valueOf(z13);
                borderViewDelegate.f32839WWWWWWWW.postInvalidate();
            }
        }
    }

    public Drawable getBorderBottomDrawable() {
        return getBorderViewDelegate().f32844WWWWWWWW;
    }

    public WWWWWWWW getBorderBottomStyle() {
        return getBorderViewDelegate().f32843WWWWWWWW;
    }

    public WWWWoWWWWo getBorderBottomVisibility() {
        return getBorderViewDelegate().f32847WWoWWo;
    }

    public Drawable getBorderTopDrawable() {
        return getBorderViewDelegate().f32846WWWoWWWo;
    }

    public WWWWWWWW getBorderTopStyle() {
        return getBorderViewDelegate().f32842WWWWWWWW;
    }

    public WWWWoWWWWo getBorderTopVisibility() {
        return getBorderViewDelegate().f32841WWWWWWWW;
    }

    @Override // r2.InterfaceC3941WWWWWWWW
    public C3942WWWWWWWW getBorderViewDelegate() {
        return this.f8300WWWWoWWWWo;
    }

    public WWWoWWWo getBorderVisibilityChangedListener() {
        getBorderViewDelegate().getClass();
        return null;
    }

    @Override // android.view.View
    public final void onDrawForeground(Canvas canvas) {
        super.onDrawForeground(canvas);
        C3942WWWWWWWW borderViewDelegate = getBorderViewDelegate();
        if (borderViewDelegate.f32846WWWoWWWo == null && borderViewDelegate.f32844WWWWWWWW == null) {
            return;
        }
        int save = canvas.save();
        Drawable drawable = borderViewDelegate.f32846WWWoWWWo;
        BorderNestedScrollView borderNestedScrollView = borderViewDelegate.f32839WWWWWWWW;
        if (drawable != null) {
            int scrollY = borderNestedScrollView.getScrollY();
            if (borderViewDelegate.f32842WWWWWWWW == WWWWWWWW.f32836WWWWoWWWWo) {
                scrollY += borderNestedScrollView.getPaddingTop();
            }
            canvas.translate(0.0f, scrollY);
            if (borderViewDelegate.m16622WWWWoWWWWo()) {
                borderViewDelegate.f32846WWWoWWWo.setBounds(0, 0, canvas.getWidth(), borderViewDelegate.f32846WWWoWWWo.getIntrinsicHeight());
                borderViewDelegate.f32846WWWoWWWo.draw(canvas);
            }
            canvas.translate(0.0f, -scrollY);
        }
        if (borderViewDelegate.f32844WWWWWWWW != null) {
            int height = (canvas.getHeight() + borderNestedScrollView.getScrollY()) - borderViewDelegate.f32844WWWWWWWW.getIntrinsicHeight();
            if (borderViewDelegate.f32842WWWWWWWW == WWWWWWWW.f32836WWWWoWWWWo) {
                height -= borderNestedScrollView.getPaddingBottom();
            }
            canvas.translate(0.0f, height);
            if (borderViewDelegate.m16623WWWWWWWW()) {
                borderViewDelegate.f32844WWWWWWWW.setBounds(0, 0, canvas.getWidth(), borderViewDelegate.f32844WWWWWWWW.getIntrinsicHeight());
                borderViewDelegate.f32844WWWWWWWW.draw(canvas);
            }
        }
        canvas.restoreToCount(save);
    }

    @Override // androidx.core.widget.NestedScrollView, android.widget.FrameLayout, android.view.ViewGroup, android.view.View
    public final void onLayout(boolean z10, int i10, int i11, int i12, int i13) {
        super.onLayout(z10, i10, i11, i12, i13);
        m4837WWWWWWWW();
    }

    @Override // androidx.core.widget.NestedScrollView, android.view.View
    public final void onScrollChanged(int i10, int i11, int i12, int i13) {
        m4837WWWWWWWW();
        super.onScrollChanged(i10, i11, i12, i13);
    }

    public void setBorderBottomDrawable(Drawable drawable) {
        C3942WWWWWWWW borderViewDelegate = getBorderViewDelegate();
        if (drawable != borderViewDelegate.f32844WWWWWWWW) {
            borderViewDelegate.f32844WWWWWWWW = drawable;
            borderViewDelegate.f32839WWWWWWWW.postInvalidate();
        }
    }

    public void setBorderBottomStyle(WWWWWWWW wwwwwwww) {
        C3942WWWWWWWW borderViewDelegate = getBorderViewDelegate();
        if (borderViewDelegate.f32843WWWWWWWW != wwwwwwww) {
            borderViewDelegate.f32843WWWWWWWW = wwwwwwww;
            borderViewDelegate.f32839WWWWWWWW.postInvalidate();
        }
    }

    public void setBorderBottomVisibility(WWWWoWWWWo wWWWoWWWWo) {
        C3942WWWWWWWW borderViewDelegate = getBorderViewDelegate();
        if (wWWWoWWWWo != borderViewDelegate.f32847WWoWWo) {
            borderViewDelegate.f32847WWoWWo = wWWWoWWWWo;
            ((BorderNestedScrollView) borderViewDelegate.f32838WWWWoWWWWo).m4837WWWWWWWW();
        }
    }

    public void setBorderTopDrawable(Drawable drawable) {
        C3942WWWWWWWW borderViewDelegate = getBorderViewDelegate();
        if (drawable != borderViewDelegate.f32846WWWoWWWo) {
            borderViewDelegate.f32846WWWoWWWo = drawable;
            borderViewDelegate.f32839WWWWWWWW.postInvalidate();
        }
    }

    public void setBorderTopStyle(WWWWWWWW wwwwwwww) {
        C3942WWWWWWWW borderViewDelegate = getBorderViewDelegate();
        if (borderViewDelegate.f32842WWWWWWWW != wwwwwwww) {
            borderViewDelegate.f32842WWWWWWWW = wwwwwwww;
            borderViewDelegate.f32839WWWWWWWW.postInvalidate();
        }
    }

    public void setBorderTopVisibility(WWWWoWWWWo wWWWoWWWWo) {
        C3942WWWWWWWW borderViewDelegate = getBorderViewDelegate();
        if (wWWWoWWWWo != borderViewDelegate.f32841WWWWWWWW) {
            borderViewDelegate.f32841WWWWWWWW = wWWWoWWWWo;
            ((BorderNestedScrollView) borderViewDelegate.f32838WWWWoWWWWo).m4837WWWWWWWW();
        }
    }

    public void setBorderVisibilityChangedListener(WWWoWWWo wWWoWWWo) {
        getBorderViewDelegate().getClass();
    }

    public BorderNestedScrollView(Context context, AttributeSet attributeSet) {
        this(context, attributeSet, R.attr.borderViewStyle);
    }

    public BorderNestedScrollView(Context context, AttributeSet attributeSet, int i10) {
        super(context, attributeSet, i10);
        this.f8300WWWWoWWWWo = new C3942WWWWWWWW(this, context, attributeSet, i10);
    }
}
