package com.android.vmapp.ui.vm.main;

import android.content.Context;
import android.graphics.Canvas;
import android.util.AttributeSet;
import android.view.MotionEvent;
import android.view.View;
import android.widget.TextView;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmapp.ui.vm.main.VMBigPreviewCardView;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.PermissionEvent;
import com.android.vmcore.event.VMConfigEvent;
import com.android.vmcore.event.VMStatusEvent;
import com.android.vmcore.ui.VMSurfaceView;
import com.blankj.utilcode.util.WWWW;
import com.clone.android.dual.space.R;
import com.google.android.material.button.MaterialButton;
import com.google.android.material.card.MaterialCardView;
import eh.C2467WWWWWWWW;
import eh.InterfaceC2472WWWWWWWW;
import i0.WWWWWWWW;
import im.amomo.andun7z.AndUn7z;
import j3.C3164WWWWWWWW;
import k4.InterfaceC3250WWoWWo;
import k4.RunnableC3236WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import m3.WWWoWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.greenrobot.eventbus.ThreadMode;
import r4.C3970WWWoWWWo;
/* loaded from: classes.dex */
public final class VMBigPreviewCardView extends MaterialCardView {

    /* renamed from: WWWWሗWWWWшሗ  reason: contains not printable characters */
    public VMSurfaceView f8645WWWWWWWW;

    /* renamed from: WWWWኈWWWWˡኈ  reason: contains not printable characters */
    public View f8646WWWWWWWW;

    /* renamed from: WWWWዪWWWWͯዪ  reason: contains not printable characters */
    public MaterialButton f8647WWWWWWWW;

    /* renamed from: WWWWᏊWWWWటᏊ  reason: contains not printable characters */
    public VMInstance f8648WWWWWWWW;

    /* renamed from: WWWᏛWWW෮Ꮫ  reason: contains not printable characters */
    public InterfaceC3250WWoWWo f8649WWWWWW;

    /* renamed from: WWoዕWWoూዕ  reason: contains not printable characters */
    public MaterialButton f8650WWoWWo;

    /* renamed from: WWoᐛWWoʄᐛ  reason: contains not printable characters */
    public RunnableC3236WWWWWWWW f8651WWoWWo;

    /* renamed from: WWኮWWႉኮ  reason: contains not printable characters */
    public TextView f8652WWWW;

    /* JADX WARN: 'this' call moved to the top of the method (can break code semantics) */
    public VMBigPreviewCardView(Context context) {
        this(context, null, 6, 0);
        byte[] bArr = {91, -98, -31, -71, -40, -99, 15, TarConstants.LF_DIR};
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{56, -15, -113, -51, -67, -27, 123}, bArr, context);
    }

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public static void m4968WWWW(VMBigPreviewCardView vMBigPreviewCardView, int i10) {
        super.setVisibility(i10);
    }

    /* renamed from: WWWWܬWWWWೖܬ  reason: contains not printable characters */
    public final void m4969WWWWWWWW() {
        Runnable runnable = this.f8651WWoWWo;
        if (runnable != null) {
            removeCallbacks(runnable);
        }
        RunnableC3236WWWWWWWW runnableC3236WWWWWWWW = new RunnableC3236WWWWWWWW(this, 1);
        this.f8651WWoWWo = runnableC3236WWWWWWWW;
        postDelayed(runnableC3236WWWWWWWW, 2000L);
        View view = this.f8646WWWWWWWW;
        if (view != null) {
            view.setVisibility(0);
            return;
        }
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{ConstantPoolEntry.CP_NameAndType, -35, 5, -78, -121, 15, TarConstants.LF_SYMLINK, 101, 18, -60, 26, -78, -126}, new byte[]{97, -110, 115, -41, -11, 99, TarConstants.LF_GNUTYPE_SPARSE, 28});
        throw null;
    }

    /* renamed from: WWWWެWWWWܕެ  reason: contains not printable characters */
    public final void m4970WWWWWWWW(MaterialButton materialButton) {
        VMInstance vMInstance = this.f8648WWWWWWWW;
        if (vMInstance != null) {
            if (vMInstance.f8940WWoWWo > 0) {
                materialButton.setIconResource(R.drawable.outline_play_circle_outline_24);
                return;
            } else {
                materialButton.setIconResource(R.drawable.outline_play_circle_outline_24);
                return;
            }
        }
        byte[] bArr = {-37, -71, -63, TarConstants.LF_MULTIVOLUME, -106, 13, -91, 26};
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-74, -17, -116, 4, -8, 126, -47, 123, -75, -38, -92}, bArr);
        throw null;
    }

    /* renamed from: WWWoࠟWWWoॣࠟ  reason: contains not printable characters */
    public final void m4971WWWoWWWo(MaterialButton materialButton) {
        VMInstance vMInstance = this.f8648WWWWWWWW;
        if (vMInstance != null) {
            if (vMInstance.m5062WWWWWWWW()) {
                materialButton.setIconResource(R.drawable.outline_volume_off_24);
                return;
            } else {
                materialButton.setIconResource(R.drawable.outline_volume_up_24);
                return;
            }
        }
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-62, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_GNUTYPE_LONGLINK, 72, 87, TarConstants.LF_NORMAL, -120, 116, -63, 109, 99}, new byte[]{-81, 14, 6, 1, 57, 67, -4, 21});
        throw null;
    }

    /* renamed from: WWoڢWWo࢞ڢ  reason: not valid java name and contains not printable characters */
    public final void m4972WWoWWo() {
        View view = this.f8646WWWWWWWW;
        if (view != null) {
            view.setVisibility(4);
            RunnableC3236WWWWWWWW runnableC3236WWWWWWWW = this.f8651WWoWWo;
            if (runnableC3236WWWWWWWW != null) {
                removeCallbacks(runnableC3236WWWWWWWW);
                return;
            }
            return;
        }
        byte[] bArr = {-65, 38, TarConstants.LF_CONTIG, -34, -94, 79, -16, 25};
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-46, 105, 65, -69, -48, 35, -111, 96, -52, 112, 94, -69, -43}, bArr);
        throw null;
    }

    @Override // android.view.ViewGroup, android.view.View
    public final void dispatchDraw(Canvas canvas) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(canvas, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, 28, 86, 0, -55, -62}, new byte[]{27, 125, 56, 118, -88, -79, -115, -124}));
        super.dispatchDraw(canvas);
        post(new RunnableC3236WWWWWWWW(this, 0));
    }

    @Override // android.view.ViewGroup, android.view.View
    public final boolean dispatchTouchEvent(MotionEvent motionEvent) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(motionEvent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, 33}, new byte[]{29, 87, -119, 105, -66, -24, -22, -59}));
        View view = this.f8646WWWWWWWW;
        if (view != null) {
            if (view.getVisibility() == 0) {
                m4969WWWWWWWW();
            }
            VMSurfaceView vMSurfaceView = this.f8645WWWWWWWW;
            if (vMSurfaceView != null) {
                requestDisallowInterceptTouchEvent(vMSurfaceView.f9284WWWWWWWWWW);
                return super.dispatchTouchEvent(motionEvent);
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-96, 72, 115, -75, 36, -49, -125, Byte.MIN_VALUE, -82, 123, 104, -113, TarConstants.LF_BLK, -54}, new byte[]{-51, 30, 62, -26, 81, -67, -27, -31}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{87, 27, -83, -38, TarConstants.LF_GNUTYPE_LONGLINK, 111, 31, -53, 73, 2, -78, -38, 78}, new byte[]{58, 84, -37, -65, 57, 3, 126, -78}));
        throw null;
    }

    @Override // android.view.View
    public final void onFinishInflate() {
        super.onFinishInflate();
        View findViewById = findViewById(R.id.vmsurfaceview);
        AbstractC1017WWWoWWWo.m3443WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-27, -124, -22, 65, -30, 112, -55, -66, -63, -108, -51, 65, -100, TarConstants.LF_CONTIG, -126, -25, -86}, new byte[]{-125, -19, -124, 37, -76, 25, -84, -55}, findViewById);
        this.f8645WWWWWWWW = (VMSurfaceView) findViewById;
        View findViewById2 = findViewById(R.id.overlays);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_SYMLINK, 41, 30, 71, 7, 68, 16, TarConstants.LF_MULTIVOLUME, 22, 57, 57, 71, 121, 3, 91, 20, 125}, new byte[]{84, 64, 112, 35, 81, 45, 117, 58}));
        this.f8646WWWWWWWW = findViewById2;
        View findViewById3 = findViewById(R.id.name);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-51, 70, -29, TarConstants.LF_MULTIVOLUME, -78, -84, 116, 60, -23, 86, -60, TarConstants.LF_MULTIVOLUME, -52, -21, 63, 101, -126}, new byte[]{-85, 47, -115, 41, -28, -59, 17, TarConstants.LF_GNUTYPE_LONGLINK}));
        this.f8652WWWW = (TextView) findViewById3;
        View findViewById4 = findViewById(R.id.start);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById4, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{116, -45, 105, 113, -75, -86, -64, -74, 80, -61, 78, 113, -53, -19, -117, -17, 59}, new byte[]{18, -70, 7, 21, -29, -61, -91, -63}));
        MaterialButton materialButton = (MaterialButton) findViewById4;
        this.f8650WWoWWo = materialButton;
        materialButton.setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWWWӈWWWWीӈ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMBigPreviewCardView f29271WWWWWWWWWW;

            {
                this.f29271WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMBigPreviewCardView vMBigPreviewCardView = this.f29271WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{24, 44, -63, -75, 104, -92, 27, -81, 27, 25, -23}, new byte[]{117, 122, -116, -4, 6, -41, 111, -50});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{68, -92, -125, 32, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 2, 62, -89, 71, -111, -85}, new byte[]{41, -14, -50, 105, 22, 113, 74, -58});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-100, 109, 121, -105, 84, 86, 90, 27, -97, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81}, new byte[]{-15, 59, TarConstants.LF_BLK, -34, 58, 37, 46, 122});
                                throw null;
                            }
                        }
                        byte[] bArr = {TarConstants.LF_PAX_EXTENDED_HEADER_UC, -80, -110, TarConstants.LF_GNUTYPE_LONGLINK, 42, 124, 67, -92, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -86, -118, 7, 104, 122, 2, -87, 87, -74, -118, 7, 126, 112, 2, -92, 89, -85, -45, 73, Byte.MAX_VALUE, 115, 78, -22, 66, -68, -114, 66, 42, 124, TarConstants.LF_MULTIVOLUME, -89, 24, -94, -111, 72, 109, 115, 71, -28, 87, -85, -102, 85, 101, 118, 70, -28, 91, -92, -118, 66, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 118, 67, -90, 24, -89, -117, TarConstants.LF_GNUTYPE_SPARSE, 126, 112, TarConstants.LF_GNUTYPE_LONGNAME, -28, 123, -92, -118, 66, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 118, 67, -90, 116, -80, -118, TarConstants.LF_GNUTYPE_SPARSE, 101, 113};
                        byte[] bArr2 = {TarConstants.LF_FIFO, -59, -2, 39, 10, 31, 34, -54};
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                        vMBigPreviewCardView.m4971WWWoWWWo((MaterialButton) view);
                        return;
                    case 3:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{82, -35, -15, -1, 109, -69, 116, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81, -24, -39}, new byte[]{63, -117, -68, -74, 3, -56, 0, 57});
                            throw null;
                        }
                        return;
                    default:
                        VMSurfaceView vMSurfaceView = vMBigPreviewCardView.f8645WWWWWWWW;
                        if (vMSurfaceView != null) {
                            if (!vMSurfaceView.f9284WWWWWWWWWW) {
                                vMBigPreviewCardView.m4969WWWWWWWW();
                                return;
                            }
                            return;
                        }
                        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-57, -22, -65, 62, -4, 9, -50, -11, -55, -39, -92, 4, -20, ConstantPoolEntry.CP_NameAndType}, new byte[]{-86, -68, -14, 109, -119, 123, -88, -108});
                        throw null;
                }
            }
        });
        View findViewById5 = findViewById(R.id.shutdown);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById5, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-119, 102, -95, -1, -79, 16, 96, 107, -83, 118, -122, -1, -49, 87, 43, TarConstants.LF_SYMLINK, -58}, new byte[]{-17, 15, -49, -101, -25, 121, 5, 28}));
        ((MaterialButton) findViewById5).setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWWWӈWWWWीӈ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMBigPreviewCardView f29271WWWWWWWWWW;

            {
                this.f29271WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMBigPreviewCardView vMBigPreviewCardView = this.f29271WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{24, 44, -63, -75, 104, -92, 27, -81, 27, 25, -23}, new byte[]{117, 122, -116, -4, 6, -41, 111, -50});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{68, -92, -125, 32, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 2, 62, -89, 71, -111, -85}, new byte[]{41, -14, -50, 105, 22, 113, 74, -58});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-100, 109, 121, -105, 84, 86, 90, 27, -97, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81}, new byte[]{-15, 59, TarConstants.LF_BLK, -34, 58, 37, 46, 122});
                                throw null;
                            }
                        }
                        byte[] bArr = {TarConstants.LF_PAX_EXTENDED_HEADER_UC, -80, -110, TarConstants.LF_GNUTYPE_LONGLINK, 42, 124, 67, -92, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -86, -118, 7, 104, 122, 2, -87, 87, -74, -118, 7, 126, 112, 2, -92, 89, -85, -45, 73, Byte.MAX_VALUE, 115, 78, -22, 66, -68, -114, 66, 42, 124, TarConstants.LF_MULTIVOLUME, -89, 24, -94, -111, 72, 109, 115, 71, -28, 87, -85, -102, 85, 101, 118, 70, -28, 91, -92, -118, 66, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 118, 67, -90, 24, -89, -117, TarConstants.LF_GNUTYPE_SPARSE, 126, 112, TarConstants.LF_GNUTYPE_LONGNAME, -28, 123, -92, -118, 66, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 118, 67, -90, 116, -80, -118, TarConstants.LF_GNUTYPE_SPARSE, 101, 113};
                        byte[] bArr2 = {TarConstants.LF_FIFO, -59, -2, 39, 10, 31, 34, -54};
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                        vMBigPreviewCardView.m4971WWWoWWWo((MaterialButton) view);
                        return;
                    case 3:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{82, -35, -15, -1, 109, -69, 116, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81, -24, -39}, new byte[]{63, -117, -68, -74, 3, -56, 0, 57});
                            throw null;
                        }
                        return;
                    default:
                        VMSurfaceView vMSurfaceView = vMBigPreviewCardView.f8645WWWWWWWW;
                        if (vMSurfaceView != null) {
                            if (!vMSurfaceView.f9284WWWWWWWWWW) {
                                vMBigPreviewCardView.m4969WWWWWWWW();
                                return;
                            }
                            return;
                        }
                        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-57, -22, -65, 62, -4, 9, -50, -11, -55, -39, -92, 4, -20, ConstantPoolEntry.CP_NameAndType}, new byte[]{-86, -68, -14, 109, -119, 123, -88, -108});
                        throw null;
                }
            }
        });
        View findViewById6 = findViewById(R.id.volume);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById6, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-17, -49, -61, 110, 28, -21, -23, 19, -53, -33, -28, 110, 98, -84, -94, 74, -96}, new byte[]{-119, -90, -83, 10, 74, -126, -116, 100}));
        MaterialButton materialButton2 = (MaterialButton) findViewById6;
        this.f8647WWWWWWWW = materialButton2;
        materialButton2.setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWWWӈWWWWीӈ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMBigPreviewCardView f29271WWWWWWWWWW;

            {
                this.f29271WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMBigPreviewCardView vMBigPreviewCardView = this.f29271WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{24, 44, -63, -75, 104, -92, 27, -81, 27, 25, -23}, new byte[]{117, 122, -116, -4, 6, -41, 111, -50});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{68, -92, -125, 32, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 2, 62, -89, 71, -111, -85}, new byte[]{41, -14, -50, 105, 22, 113, 74, -58});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-100, 109, 121, -105, 84, 86, 90, 27, -97, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81}, new byte[]{-15, 59, TarConstants.LF_BLK, -34, 58, 37, 46, 122});
                                throw null;
                            }
                        }
                        byte[] bArr = {TarConstants.LF_PAX_EXTENDED_HEADER_UC, -80, -110, TarConstants.LF_GNUTYPE_LONGLINK, 42, 124, 67, -92, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -86, -118, 7, 104, 122, 2, -87, 87, -74, -118, 7, 126, 112, 2, -92, 89, -85, -45, 73, Byte.MAX_VALUE, 115, 78, -22, 66, -68, -114, 66, 42, 124, TarConstants.LF_MULTIVOLUME, -89, 24, -94, -111, 72, 109, 115, 71, -28, 87, -85, -102, 85, 101, 118, 70, -28, 91, -92, -118, 66, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 118, 67, -90, 24, -89, -117, TarConstants.LF_GNUTYPE_SPARSE, 126, 112, TarConstants.LF_GNUTYPE_LONGNAME, -28, 123, -92, -118, 66, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 118, 67, -90, 116, -80, -118, TarConstants.LF_GNUTYPE_SPARSE, 101, 113};
                        byte[] bArr2 = {TarConstants.LF_FIFO, -59, -2, 39, 10, 31, 34, -54};
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                        vMBigPreviewCardView.m4971WWWoWWWo((MaterialButton) view);
                        return;
                    case 3:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{82, -35, -15, -1, 109, -69, 116, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81, -24, -39}, new byte[]{63, -117, -68, -74, 3, -56, 0, 57});
                            throw null;
                        }
                        return;
                    default:
                        VMSurfaceView vMSurfaceView = vMBigPreviewCardView.f8645WWWWWWWW;
                        if (vMSurfaceView != null) {
                            if (!vMSurfaceView.f9284WWWWWWWWWW) {
                                vMBigPreviewCardView.m4969WWWWWWWW();
                                return;
                            }
                            return;
                        }
                        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-57, -22, -65, 62, -4, 9, -50, -11, -55, -39, -92, 4, -20, ConstantPoolEntry.CP_NameAndType}, new byte[]{-86, -68, -14, 109, -119, 123, -88, -108});
                        throw null;
                }
            }
        });
        View findViewById7 = findViewById(R.id.settings);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById7, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{63, -121, 113, -74, -43, -96, 124, -39, 27, -105, 86, -74, -85, -25, TarConstants.LF_CONTIG, Byte.MIN_VALUE, 112}, new byte[]{89, -18, 31, -46, -125, -55, 25, -82}));
        ((MaterialButton) findViewById7).setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWWWӈWWWWीӈ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMBigPreviewCardView f29271WWWWWWWWWW;

            {
                this.f29271WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMBigPreviewCardView vMBigPreviewCardView = this.f29271WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{24, 44, -63, -75, 104, -92, 27, -81, 27, 25, -23}, new byte[]{117, 122, -116, -4, 6, -41, 111, -50});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{68, -92, -125, 32, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 2, 62, -89, 71, -111, -85}, new byte[]{41, -14, -50, 105, 22, 113, 74, -58});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-100, 109, 121, -105, 84, 86, 90, 27, -97, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81}, new byte[]{-15, 59, TarConstants.LF_BLK, -34, 58, 37, 46, 122});
                                throw null;
                            }
                        }
                        byte[] bArr = {TarConstants.LF_PAX_EXTENDED_HEADER_UC, -80, -110, TarConstants.LF_GNUTYPE_LONGLINK, 42, 124, 67, -92, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -86, -118, 7, 104, 122, 2, -87, 87, -74, -118, 7, 126, 112, 2, -92, 89, -85, -45, 73, Byte.MAX_VALUE, 115, 78, -22, 66, -68, -114, 66, 42, 124, TarConstants.LF_MULTIVOLUME, -89, 24, -94, -111, 72, 109, 115, 71, -28, 87, -85, -102, 85, 101, 118, 70, -28, 91, -92, -118, 66, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 118, 67, -90, 24, -89, -117, TarConstants.LF_GNUTYPE_SPARSE, 126, 112, TarConstants.LF_GNUTYPE_LONGNAME, -28, 123, -92, -118, 66, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 118, 67, -90, 116, -80, -118, TarConstants.LF_GNUTYPE_SPARSE, 101, 113};
                        byte[] bArr2 = {TarConstants.LF_FIFO, -59, -2, 39, 10, 31, 34, -54};
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                        vMBigPreviewCardView.m4971WWWoWWWo((MaterialButton) view);
                        return;
                    case 3:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{82, -35, -15, -1, 109, -69, 116, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81, -24, -39}, new byte[]{63, -117, -68, -74, 3, -56, 0, 57});
                            throw null;
                        }
                        return;
                    default:
                        VMSurfaceView vMSurfaceView = vMBigPreviewCardView.f8645WWWWWWWW;
                        if (vMSurfaceView != null) {
                            if (!vMSurfaceView.f9284WWWWWWWWWW) {
                                vMBigPreviewCardView.m4969WWWWWWWW();
                                return;
                            }
                            return;
                        }
                        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-57, -22, -65, 62, -4, 9, -50, -11, -55, -39, -92, 4, -20, ConstantPoolEntry.CP_NameAndType}, new byte[]{-86, -68, -14, 109, -119, 123, -88, -108});
                        throw null;
                }
            }
        });
        m4972WWoWWo();
        setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWWWӈWWWWीӈ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMBigPreviewCardView f29271WWWWWWWWWW;

            {
                this.f29271WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMBigPreviewCardView vMBigPreviewCardView = this.f29271WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{24, 44, -63, -75, 104, -92, 27, -81, 27, 25, -23}, new byte[]{117, 122, -116, -4, 6, -41, 111, -50});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{68, -92, -125, 32, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 2, 62, -89, 71, -111, -85}, new byte[]{41, -14, -50, 105, 22, 113, 74, -58});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-100, 109, 121, -105, 84, 86, 90, 27, -97, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81}, new byte[]{-15, 59, TarConstants.LF_BLK, -34, 58, 37, 46, 122});
                                throw null;
                            }
                        }
                        byte[] bArr = {TarConstants.LF_PAX_EXTENDED_HEADER_UC, -80, -110, TarConstants.LF_GNUTYPE_LONGLINK, 42, 124, 67, -92, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -86, -118, 7, 104, 122, 2, -87, 87, -74, -118, 7, 126, 112, 2, -92, 89, -85, -45, 73, Byte.MAX_VALUE, 115, 78, -22, 66, -68, -114, 66, 42, 124, TarConstants.LF_MULTIVOLUME, -89, 24, -94, -111, 72, 109, 115, 71, -28, 87, -85, -102, 85, 101, 118, 70, -28, 91, -92, -118, 66, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 118, 67, -90, 24, -89, -117, TarConstants.LF_GNUTYPE_SPARSE, 126, 112, TarConstants.LF_GNUTYPE_LONGNAME, -28, 123, -92, -118, 66, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 118, 67, -90, 116, -80, -118, TarConstants.LF_GNUTYPE_SPARSE, 101, 113};
                        byte[] bArr2 = {TarConstants.LF_FIFO, -59, -2, 39, 10, 31, 34, -54};
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                        vMBigPreviewCardView.m4971WWWoWWWo((MaterialButton) view);
                        return;
                    case 3:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMBigPreviewCardView.f8649WWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMBigPreviewCardView.f8648WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{82, -35, -15, -1, 109, -69, 116, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81, -24, -39}, new byte[]{63, -117, -68, -74, 3, -56, 0, 57});
                            throw null;
                        }
                        return;
                    default:
                        VMSurfaceView vMSurfaceView = vMBigPreviewCardView.f8645WWWWWWWW;
                        if (vMSurfaceView != null) {
                            if (!vMSurfaceView.f9284WWWWWWWWWW) {
                                vMBigPreviewCardView.m4969WWWWWWWW();
                                return;
                            }
                            return;
                        }
                        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-57, -22, -65, 62, -4, 9, -50, -11, -55, -39, -92, 4, -20, ConstantPoolEntry.CP_NameAndType}, new byte[]{-86, -68, -14, 109, -119, 123, -88, -108});
                        throw null;
                }
            }
        });
    }

    @Override // com.google.android.material.card.MaterialCardView, androidx.cardview.widget.CardView, android.widget.FrameLayout, android.view.View
    public final void onMeasure(int i10, int i11) {
        int size = View.MeasureSpec.getSize(i10);
        int size2 = View.MeasureSpec.getSize(i11);
        float m5329WWWWWWWW = (WWWW.m5329WWWWWWWW() * 1.0f) / WWWW.m5328WWWWWWWW();
        float f10 = size / m5329WWWWWWWW;
        float f11 = size2;
        if (f10 > f11) {
            size = (int) (f11 * m5329WWWWWWWW);
        } else {
            size2 = (int) f10;
        }
        super.onMeasure(View.MeasureSpec.makeMeasureSpec(size, 1073741824), View.MeasureSpec.makeMeasureSpec(size2, 1073741824));
    }

    @InterfaceC2472WWWWWWWW(sticky = AndUn7z.f28850WWWWWWWW, threadMode = ThreadMode.MAIN)
    public final void onVMConfigEvent(VMConfigEvent vMConfigEvent) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMConfigEvent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-125, -102, Byte.MIN_VALUE, -109, 119}, new byte[]{-26, -20, -27, -3, 3, -92, 78, 61}));
        TextView textView = this.f8652WWWW;
        if (textView != null) {
            VMInstance vMInstance = this.f8648WWWWWWWW;
            if (vMInstance != null) {
                textView.setText(vMInstance.f8937WWWoWWWo.f8861WWWWoWWWWo);
                return;
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType, 99, 7, -27, 93, 6, 2, 94, 15, 86, 47}, new byte[]{97, TarConstants.LF_DIR, 74, -84, TarConstants.LF_CHR, 117, 118, 63}));
                throw null;
            }
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-115, 57, 60, -45, TarConstants.LF_NORMAL, 124, 1, 14, -105}, new byte[]{-32, 119, 93, -66, 85, 42, 104, 107}));
        throw null;
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.MAIN)
    public final void onVMOutsidePageRequestEvent(WWWoWWWo wWWoWWWo) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(wWWoWWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, -77, -12, -100, 112}, new byte[]{-83, -59, -111, -14, 4, -10, 81, 113}));
        InterfaceC3250WWoWWo interfaceC3250WWoWWo = this.f8649WWWWWW;
        if (interfaceC3250WWoWWo != null) {
            VMInstance vMInstance = this.f8648WWWWWWWW;
            if (vMInstance != null) {
                interfaceC3250WWoWWo.mo4976WWWWWWWW(vMInstance, wWWoWWWo);
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_BLK, -65, -89, 40, -70, 35, -120, TarConstants.LF_FIFO, TarConstants.LF_CONTIG, -118, -113}, new byte[]{89, -23, -22, 97, -44, 80, -4, 87}));
                throw null;
            }
        }
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.MAIN)
    public final void onVMPermissionEvent(PermissionEvent permissionEvent) {
        byte[] bArr = {TarConstants.LF_CHR, 15, -74, -5, -86, -62, -94, 107};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(permissionEvent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{86, 121, -45, -107, -34}, bArr));
        InterfaceC3250WWoWWo interfaceC3250WWoWWo = this.f8649WWWWWW;
        if (interfaceC3250WWoWWo != null) {
            VMInstance vMInstance = this.f8648WWWWWWWW;
            if (vMInstance != null) {
                interfaceC3250WWoWWo.mo4983WWoWWo(vMInstance, permissionEvent);
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-109, 18, 91, 69, -106, -25, -87, 74, -112, 39, 115}, new byte[]{-2, 68, 22, ConstantPoolEntry.CP_NameAndType, -8, -108, -35, 43}));
                throw null;
            }
        }
    }

    @InterfaceC2472WWWWWWWW(sticky = AndUn7z.f28850WWWWWWWW, threadMode = ThreadMode.MAIN)
    public final void onVMStatusEvent(VMStatusEvent vMStatusEvent) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMStatusEvent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-21, -5, -113, 114, -115}, new byte[]{-114, -115, -22, 28, -7, -6, 17, 65}));
        MaterialButton materialButton = this.f8650WWoWWo;
        if (materialButton != null) {
            m4970WWWWWWWW(materialButton);
            MaterialButton materialButton2 = this.f8647WWWWWWWW;
            if (materialButton2 != null) {
                m4971WWWoWWWo(materialButton2);
                return;
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{64, -52, -74, -90, 109, -96, 7, 98, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -18, -83, -91, 118}, new byte[]{45, -102, -39, -54, 24, -51, 98, 32}));
                throw null;
            }
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-91, -16, -59, -66, 109, 69, -73, 13, -68, -41, -34, -79}, new byte[]{-56, -93, -79, -33, 31, TarConstants.LF_LINK, -11, TarConstants.LF_PAX_EXTENDED_HEADER_LC}));
        throw null;
    }

    @Override // android.view.View
    public final void onWindowVisibilityChanged(int i10) {
        if (i10 == 0) {
            VMInstance vMInstance = this.f8648WWWWWWWW;
            if (vMInstance != null) {
                C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
                if (!c2467wwwwwwww.m13948WWoWWo(this)) {
                    c2467wwwwwwww.m13950WWWW(this);
                }
            } else {
                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-1, -52, -10, -93, 113, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -98, -99, -4, -7, -34}, new byte[]{-110, -102, -69, -22, 31, ConstantPoolEntry.CP_InterfaceMethodref, -22, -4});
                throw null;
            }
        } else {
            VMInstance vMInstance2 = this.f8648WWWWWWWW;
            if (vMInstance2 != null) {
                C2467WWWWWWWW c2467wwwwwwww2 = vMInstance2.f8939WWWoWWWo;
                if (c2467wwwwwwww2.m13948WWoWWo(this)) {
                    c2467wwwwwwww2.m13945WWWWWWWW(this);
                }
            } else {
                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-21, -31, 3, TarConstants.LF_MULTIVOLUME, 82, -68, -51, -48, -24, -44, 43}, new byte[]{-122, -73, 78, 4, 60, -49, -71, -79});
                throw null;
            }
        }
        super.onWindowVisibilityChanged(i10);
    }

    public final void setTouchMode(int i10) {
        boolean z10 = false;
        int i11 = i10 % 3;
        if (i11 != 0) {
            z10 = true;
        }
        VMSurfaceView vMSurfaceView = this.f8645WWWWWWWW;
        if (vMSurfaceView != null) {
            vMSurfaceView.setTouchEnabled(z10);
            if (i11 == 2) {
                VMSurfaceView vMSurfaceView2 = this.f8645WWWWWWWW;
                if (vMSurfaceView2 != null) {
                    vMSurfaceView2.setTouchEventHandler(new C3970WWWoWWWo());
                } else {
                    WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{32, TarConstants.LF_CHR, -41, -26, -85, -38, 26, -53, 46, 0, -52, -36, -69, -33}, new byte[]{TarConstants.LF_MULTIVOLUME, 101, -102, -75, -34, -88, 124, -86});
                    throw null;
                }
            } else {
                VMSurfaceView vMSurfaceView3 = this.f8645WWWWWWWW;
                if (vMSurfaceView3 != null) {
                    vMSurfaceView3.setTouchEventHandler(null);
                } else {
                    WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_GNUTYPE_SPARSE, 32, 3, 18, 56, -21, 66, -114, 93, 19, 24, 40, 40, -18}, new byte[]{62, 118, 78, 65, TarConstants.LF_MULTIVOLUME, -103, 36, -17});
                    throw null;
                }
            }
            if (z10) {
                View view = this.f8646WWWWWWWW;
                if (view != null) {
                    if (view.getVisibility() == 0) {
                        m4972WWoWWo();
                        return;
                    }
                    return;
                }
                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{97, 24, 15, -44, -6, 21, -14, TarConstants.LF_CONTIG, Byte.MAX_VALUE, 1, 16, -44, -1}, new byte[]{ConstantPoolEntry.CP_NameAndType, 87, 121, -79, -120, 121, -109, 78});
                throw null;
            }
            return;
        }
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-40, TarConstants.LF_GNUTYPE_LONGLINK, -78, 114, 112, -13, -77, -45, -42, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -87, 72, 96, -10}, new byte[]{-75, 29, -1, 33, 5, -127, -43, -78});
        throw null;
    }

    public final void setVMInstance(VMInstance vMInstance) {
        boolean z10 = true;
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{24, -37}, new byte[]{110, -74, 124, 65, 73, 97, 16, -17}));
        this.f8648WWWWWWWW = vMInstance;
        C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
        if (!c2467wwwwwwww.m13948WWoWWo(this)) {
            c2467wwwwwwww.m13950WWWW(this);
        }
        MaterialButton materialButton = this.f8650WWoWWo;
        if (materialButton != null) {
            m4970WWWWWWWW(materialButton);
            MaterialButton materialButton2 = this.f8647WWWWWWWW;
            if (materialButton2 != null) {
                m4971WWWoWWWo(materialButton2);
                TextView textView = this.f8652WWWW;
                if (textView != null) {
                    textView.setText(vMInstance.f8937WWWoWWWo.f8861WWWWoWWWWo);
                    View findViewById = findViewById(R.id.vmsurfaceview);
                    AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{116, -53, 8, -68, -124, -99, 107, -99, 80, -37, 47, -68, -6, -38, 32, -60, 59}, new byte[]{18, -94, 102, -40, -46, -12, 14, -22}));
                    VMSurfaceView vMSurfaceView = (VMSurfaceView) findViewById;
                    this.f8645WWWWWWWW = vMSurfaceView;
                    vMSurfaceView.setVM(vMInstance);
                    VMSurfaceView vMSurfaceView2 = this.f8645WWWWWWWW;
                    if (vMSurfaceView2 != null) {
                        vMSurfaceView2.setCornerRadius(WWWW.m5340WWoWWo(8.0f));
                        VMSurfaceView vMSurfaceView3 = this.f8645WWWWWWWW;
                        if (vMSurfaceView3 != null) {
                            vMSurfaceView3.setTouchEnabled(false);
                            VMSurfaceView vMSurfaceView4 = this.f8645WWWWWWWW;
                            if (vMSurfaceView4 != null) {
                                if (WWWW.m5336WWWoWWWo().getResources().getConfiguration().orientation != 2) {
                                    z10 = false;
                                }
                                vMSurfaceView4.setLandscape(z10);
                                return;
                            }
                            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{19, ConstantPoolEntry.CP_InterfaceMethodref, 69, -86, 16, ConstantPoolEntry.CP_InterfaceMethodref, 0, 24, 29, 56, 94, -112, 0, 14}, new byte[]{126, 93, 8, -7, 101, 121, 102, 121}));
                            throw null;
                        }
                        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{90, -9, 92, -13, -44, -84, -1, 91, 84, -60, 71, -55, -60, -87}, new byte[]{TarConstants.LF_CONTIG, -95, 17, -96, -95, -34, -103, 58}));
                        throw null;
                    }
                    AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-89, -33, -26, 67, 17, -5, 89, -42, -87, -20, -3, 121, 1, -2}, new byte[]{-54, -119, -85, 16, 100, -119, 63, -73}));
                    throw null;
                }
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{37, 3, -30, -9, Byte.MAX_VALUE, -118, 78, Byte.MAX_VALUE, 63}, new byte[]{72, TarConstants.LF_MULTIVOLUME, -125, -102, 26, -36, 39, 26}));
                throw null;
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, 41, -76, -50, 71, 121, TarConstants.LF_GNUTYPE_LONGLINK, 23, -48, ConstantPoolEntry.CP_InterfaceMethodref, -81, -51, 92}, new byte[]{-91, Byte.MAX_VALUE, -37, -94, TarConstants.LF_SYMLINK, 20, 46, 85}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-26, -59, TarConstants.LF_LINK, 74, TarConstants.LF_FIFO, 114, 38, -48, -1, -30, 42, 69}, new byte[]{-117, -106, 69, 43, 68, 6, 100, -91}));
        throw null;
    }

    public final void setVMViewActionCallback(InterfaceC3250WWoWWo interfaceC3250WWoWWo) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(interfaceC3250WWoWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-54, 92, -66, -30, 95, 117, TarConstants.LF_MULTIVOLUME, -33}, new byte[]{-87, 61, -46, -114, 61, 20, 46, -76}));
        this.f8649WWWWWW = interfaceC3250WWoWWo;
    }

    @Override // android.view.View
    public void setVisibility(int i10) {
        if (i10 != 0) {
            super.setVisibility(i10);
            VMSurfaceView vMSurfaceView = this.f8645WWWWWWWW;
            if (vMSurfaceView != null) {
                vMSurfaceView.setVisibility(i10);
                return;
            }
            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{10, -75, -40, 93, 4, -37, -95, 15, 4, -122, -61, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 20, -34}, new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -29, -107, 14, 113, -87, -57, 110});
            throw null;
        }
        post(new p021WWWWWWWW.WWWW(i10, 1, this));
    }

    /* JADX WARN: 'this' call moved to the top of the method (can break code semantics) */
    public VMBigPreviewCardView(Context context, AttributeSet attributeSet) {
        this(context, attributeSet, 4, 0);
        byte[] bArr = {TarConstants.LF_FIFO, 13, 82, 117, -43, 89, -86, -52};
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{85, 98, 60, 1, -80, 33, -34}, bArr, context);
    }

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public VMBigPreviewCardView(Context context, AttributeSet attributeSet, int i10) {
        super(context, attributeSet, i10);
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-69, -117, TarConstants.LF_MULTIVOLUME, 0, 62, 90, -64}, new byte[]{-40, -28, 35, 116, 91, 34, -76, -73}, context);
    }

    public /* synthetic */ VMBigPreviewCardView(Context context, AttributeSet attributeSet, int i10, int i11) {
        this(context, (i10 & 2) != 0 ? null : attributeSet, 0);
    }
}
