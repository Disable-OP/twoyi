package com.android.vmapp.ui.vm.main;

import android.content.Context;
import android.graphics.Canvas;
import android.util.AttributeSet;
import android.view.MotionEvent;
import android.view.View;
import android.widget.TextView;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmapp.ui.vm.main.VMSmallPreviewCardView;
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
import k4.RunnableC3246WWWoWWWo;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import m3.WWWoWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.greenrobot.eventbus.ThreadMode;
import r4.C3970WWWoWWWo;
/* loaded from: classes.dex */
public final class VMSmallPreviewCardView extends MaterialCardView {

    /* renamed from: WWWWሗWWWWшሗ  reason: contains not printable characters */
    public VMSurfaceView f8670WWWWWWWW;

    /* renamed from: WWWWኈWWWWˡኈ  reason: contains not printable characters */
    public View f8671WWWWWWWW;

    /* renamed from: WWWWዪWWWWͯዪ  reason: contains not printable characters */
    public MaterialButton f8672WWWWWWWW;

    /* renamed from: WWWWᏊWWWWటᏊ  reason: contains not printable characters */
    public VMInstance f8673WWWWWWWW;

    /* renamed from: WWWᏛWWW෮Ꮫ  reason: contains not printable characters */
    public InterfaceC3250WWoWWo f8674WWWWWW;

    /* renamed from: WWoዕWWoూዕ  reason: contains not printable characters */
    public MaterialButton f8675WWoWWo;

    /* renamed from: WWoᐛWWoʄᐛ  reason: contains not printable characters */
    public RunnableC3246WWWoWWWo f8676WWoWWo;

    /* renamed from: WWኮWWႉኮ  reason: contains not printable characters */
    public TextView f8677WWWW;

    /* JADX WARN: 'this' call moved to the top of the method (can break code semantics) */
    public VMSmallPreviewCardView(Context context) {
        this(context, null, 6, 0);
        byte[] bArr = {TarConstants.LF_MULTIVOLUME, -48, 68, 97, 123, 37, -61, -87};
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{46, -65, 42, 21, 30, 93, -73}, bArr, context);
    }

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public static void m4990WWWW(VMSmallPreviewCardView vMSmallPreviewCardView, int i10) {
        super.setVisibility(i10);
    }

    /* renamed from: WWWWܬWWWWೖܬ  reason: contains not printable characters */
    public final void m4991WWWWWWWW() {
        Runnable runnable = this.f8676WWoWWo;
        if (runnable != null) {
            removeCallbacks(runnable);
        }
        RunnableC3246WWWoWWWo runnableC3246WWWoWWWo = new RunnableC3246WWWoWWWo(this, 1);
        this.f8676WWoWWo = runnableC3246WWWoWWWo;
        postDelayed(runnableC3246WWWoWWWo, 2000L);
        View view = this.f8671WWWWWWWW;
        if (view != null) {
            view.setVisibility(0);
            return;
        }
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-37, -40, 41, 39, 126, 24, -126, -46, -59, -63, TarConstants.LF_FIFO, 39, 123}, new byte[]{-74, -105, 95, 66, ConstantPoolEntry.CP_NameAndType, 116, -29, -85});
        throw null;
    }

    /* renamed from: WWWWެWWWWܕެ  reason: contains not printable characters */
    public final void m4992WWWWWWWW(MaterialButton materialButton) {
        VMInstance vMInstance = this.f8673WWWWWWWW;
        if (vMInstance != null) {
            if (vMInstance.f8940WWoWWo > 0) {
                materialButton.setIconResource(R.drawable.outline_play_circle_outline_24);
                return;
            } else {
                materialButton.setIconResource(R.drawable.outline_play_circle_outline_24);
                return;
            }
        }
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{7, -80, 61, -42, 109, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -32, -88, 4, -123, 21}, new byte[]{106, -26, 112, -97, 3, ConstantPoolEntry.CP_InterfaceMethodref, -108, -55});
        throw null;
    }

    /* renamed from: WWWoࠟWWWoॣࠟ  reason: contains not printable characters */
    public final void m4993WWWoWWWo(MaterialButton materialButton) {
        VMInstance vMInstance = this.f8673WWWWWWWW;
        if (vMInstance != null) {
            if (vMInstance.m5062WWWWWWWW()) {
                materialButton.setIconResource(R.drawable.outline_volume_off_24);
                return;
            } else {
                materialButton.setIconResource(R.drawable.outline_volume_up_24);
                return;
            }
        }
        byte[] bArr = {60, TarConstants.LF_GNUTYPE_LONGNAME, 116, -27, -35, TarConstants.LF_GNUTYPE_LONGLINK, TarConstants.LF_SYMLINK, -68};
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{81, 26, 57, -84, -77, 56, 70, -35, 82, 47, 17}, bArr);
        throw null;
    }

    /* renamed from: WWoڢWWo࢞ڢ  reason: not valid java name and contains not printable characters */
    public final void m4994WWoWWo() {
        View view = this.f8671WWWWWWWW;
        if (view != null) {
            view.setVisibility(4);
            RunnableC3246WWWoWWWo runnableC3246WWWoWWWo = this.f8676WWoWWo;
            if (runnableC3246WWWoWWWo != null) {
                removeCallbacks(runnableC3246WWWoWWWo);
                return;
            }
            return;
        }
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -8, 2, -24, -50, 21, -48, -66, 85, -31, 29, -24, -53}, new byte[]{38, -73, 116, -115, -68, 121, -79, -57});
        throw null;
    }

    @Override // android.view.ViewGroup, android.view.View
    public final void dispatchDraw(Canvas canvas) {
        byte[] bArr = {90, 15, TarConstants.LF_FIFO, -93, -120, -69};
        byte[] bArr2 = {57, 110, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -43, -23, -56, -14, -36};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(canvas, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        super.dispatchDraw(canvas);
        post(new RunnableC3246WWWoWWWo(this, 0));
    }

    @Override // android.view.ViewGroup, android.view.View
    public final boolean dispatchTouchEvent(MotionEvent motionEvent) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(motionEvent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-40, -116}, new byte[]{-67, -6, -82, Byte.MIN_VALUE, -68, -56, 13, -42}));
        View view = this.f8671WWWWWWWW;
        if (view != null) {
            if (view.getVisibility() == 0) {
                m4991WWWWWWWW();
            }
            VMSurfaceView vMSurfaceView = this.f8670WWWWWWWW;
            if (vMSurfaceView != null) {
                requestDisallowInterceptTouchEvent(vMSurfaceView.f9284WWWWWWWWWW);
                return super.dispatchTouchEvent(motionEvent);
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{86, 38, -83, Byte.MIN_VALUE, TarConstants.LF_SYMLINK, 37, -21, 23, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 21, -74, -70, 34, 32}, new byte[]{59, 112, -32, -45, 71, 87, -115, 118}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-126, 85, -94, -29, -6, -54, 91, -42, -100, TarConstants.LF_GNUTYPE_LONGNAME, -67, -29, -1}, new byte[]{-17, 26, -44, -122, -120, -90, 58, -81}));
        throw null;
    }

    @Override // android.view.View
    public final void onFinishInflate() {
        super.onFinishInflate();
        View findViewById = findViewById(R.id.vmsurfaceview);
        AbstractC1017WWWoWWWo.m3443WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-59, -37, 9, 110, -124, 97, -10, TarConstants.LF_BLK, -31, -53, 46, 110, -6, 38, -67, 109, -118}, new byte[]{-93, -78, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 10, -46, 8, -109, 67}, findViewById);
        this.f8670WWWWWWWW = (VMSurfaceView) findViewById;
        View findViewById2 = findViewById(R.id.overlays);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{106, -44, 72, 61, 30, 113, 102, 5, 78, -60, 111, 61, 96, TarConstants.LF_FIFO, 45, 92, 37}, new byte[]{ConstantPoolEntry.CP_NameAndType, -67, 38, 89, 72, 24, 3, 114}));
        this.f8671WWWWWWWW = findViewById2;
        View findViewById3 = findViewById(R.id.name);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{40, 78, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -33, 73, 43, 21, 79, ConstantPoolEntry.CP_NameAndType, 94, 95, -33, TarConstants.LF_CONTIG, 108, 94, 22, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}, new byte[]{78, 39, 22, -69, 31, 66, 112, 56}));
        this.f8677WWWW = (TextView) findViewById3;
        View findViewById4 = findViewById(R.id.start);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById4, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{47, -113, 86, -47, 84, 117, -62, -44, ConstantPoolEntry.CP_InterfaceMethodref, -97, 113, -47, 42, TarConstants.LF_SYMLINK, -119, -115, 96}, new byte[]{73, -26, 56, -75, 2, 28, -89, -93}));
        MaterialButton materialButton = (MaterialButton) findViewById4;
        this.f8675WWoWWo = materialButton;
        materialButton.setOnClickListener(new View.OnClickListener(this) { // from class: k4.oેᄈે

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMSmallPreviewCardView f29308WWWWWWWWWW;

            {
                this.f29308WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMSmallPreviewCardView vMSmallPreviewCardView = this.f29308WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{19, 72, -87, -1, -115, -52, -29, 37, 16, 125, -127}, new byte[]{126, 30, -28, -74, -29, -65, -105, 68});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-13, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_CHR, 91, -63, -34, -16, 109, 121}, new byte[]{-98, 14, 28, 69, 93, 40, -75, -65});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{31, -80, 63, 21, -12, -91, -123, -3, 28, -123, 23}, new byte[]{114, -26, 114, 92, -102, -42, -15, -100});
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, -67, 31, 47, 13, 13, 71, -109, -98, -89, 7, 99, 79, ConstantPoolEntry.CP_InterfaceMethodref, 6, -98, -111, -69, 7, 99, 89, 1, 6, -109, -97, -90, 94, 45, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 2, 74, -35, -124, -79, 3, 38, 13, 13, 73, -112, -34, -81, 28, 44, 74, 2, 67, -45, -111, -90, 23, TarConstants.LF_LINK, 66, 7, 66, -45, -99, -87, 7, 38, 95, 7, 71, -111, -34, -86, 6, TarConstants.LF_CONTIG, 89, 1, 72, -45, -67, -87, 7, 38, 95, 7, 71, -111, -78, -67, 7, TarConstants.LF_CONTIG, 66, 0}, new byte[]{-16, -56, 115, 67, 45, 110, 38, -3}));
                        vMSmallPreviewCardView.m4993WWWoWWWo((MaterialButton) view);
                        return;
                    case 3:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-44, -14, 87, -15, -123, 101, -80, -113, -41, -57, Byte.MAX_VALUE}, new byte[]{-71, -92, 26, -72, -21, 22, -60, -18});
                            throw null;
                        }
                        return;
                    default:
                        VMSurfaceView vMSurfaceView = vMSmallPreviewCardView.f8670WWWWWWWW;
                        if (vMSurfaceView != null) {
                            if (!vMSurfaceView.f9284WWWWWWWWWW) {
                                vMSmallPreviewCardView.m4991WWWWWWWW();
                                return;
                            }
                            return;
                        }
                        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{20, 125, -42, -85, 36, 81, 71, 109, 26, 78, -51, -111, TarConstants.LF_BLK, 84}, new byte[]{121, 43, -101, -8, 81, 35, 33, ConstantPoolEntry.CP_NameAndType});
                        throw null;
                }
            }
        });
        View findViewById5 = findViewById(R.id.shutdown);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById5, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-114, -121, -110, -85, 112, 87, -7, -126, -86, -105, -75, -85, 14, 16, -78, -37, -63}, new byte[]{-24, -18, -4, -49, 38, 62, -100, -11}));
        ((MaterialButton) findViewById5).setOnClickListener(new View.OnClickListener(this) { // from class: k4.oેᄈે

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMSmallPreviewCardView f29308WWWWWWWWWW;

            {
                this.f29308WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMSmallPreviewCardView vMSmallPreviewCardView = this.f29308WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{19, 72, -87, -1, -115, -52, -29, 37, 16, 125, -127}, new byte[]{126, 30, -28, -74, -29, -65, -105, 68});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-13, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_CHR, 91, -63, -34, -16, 109, 121}, new byte[]{-98, 14, 28, 69, 93, 40, -75, -65});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{31, -80, 63, 21, -12, -91, -123, -3, 28, -123, 23}, new byte[]{114, -26, 114, 92, -102, -42, -15, -100});
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, -67, 31, 47, 13, 13, 71, -109, -98, -89, 7, 99, 79, ConstantPoolEntry.CP_InterfaceMethodref, 6, -98, -111, -69, 7, 99, 89, 1, 6, -109, -97, -90, 94, 45, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 2, 74, -35, -124, -79, 3, 38, 13, 13, 73, -112, -34, -81, 28, 44, 74, 2, 67, -45, -111, -90, 23, TarConstants.LF_LINK, 66, 7, 66, -45, -99, -87, 7, 38, 95, 7, 71, -111, -34, -86, 6, TarConstants.LF_CONTIG, 89, 1, 72, -45, -67, -87, 7, 38, 95, 7, 71, -111, -78, -67, 7, TarConstants.LF_CONTIG, 66, 0}, new byte[]{-16, -56, 115, 67, 45, 110, 38, -3}));
                        vMSmallPreviewCardView.m4993WWWoWWWo((MaterialButton) view);
                        return;
                    case 3:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-44, -14, 87, -15, -123, 101, -80, -113, -41, -57, Byte.MAX_VALUE}, new byte[]{-71, -92, 26, -72, -21, 22, -60, -18});
                            throw null;
                        }
                        return;
                    default:
                        VMSurfaceView vMSurfaceView = vMSmallPreviewCardView.f8670WWWWWWWW;
                        if (vMSurfaceView != null) {
                            if (!vMSurfaceView.f9284WWWWWWWWWW) {
                                vMSmallPreviewCardView.m4991WWWWWWWW();
                                return;
                            }
                            return;
                        }
                        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{20, 125, -42, -85, 36, 81, 71, 109, 26, 78, -51, -111, TarConstants.LF_BLK, 84}, new byte[]{121, 43, -101, -8, 81, 35, 33, ConstantPoolEntry.CP_NameAndType});
                        throw null;
                }
            }
        });
        View findViewById6 = findViewById(R.id.volume);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById6, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{10, 123, -116, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 79, -4, 1, 44, 46, 107, -85, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_LINK, -69, 74, 117, 69}, new byte[]{108, 18, -30, 3, 25, -107, 100, 91}));
        MaterialButton materialButton2 = (MaterialButton) findViewById6;
        this.f8672WWWWWWWW = materialButton2;
        materialButton2.setOnClickListener(new View.OnClickListener(this) { // from class: k4.oેᄈે

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMSmallPreviewCardView f29308WWWWWWWWWW;

            {
                this.f29308WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMSmallPreviewCardView vMSmallPreviewCardView = this.f29308WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{19, 72, -87, -1, -115, -52, -29, 37, 16, 125, -127}, new byte[]{126, 30, -28, -74, -29, -65, -105, 68});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-13, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_CHR, 91, -63, -34, -16, 109, 121}, new byte[]{-98, 14, 28, 69, 93, 40, -75, -65});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{31, -80, 63, 21, -12, -91, -123, -3, 28, -123, 23}, new byte[]{114, -26, 114, 92, -102, -42, -15, -100});
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, -67, 31, 47, 13, 13, 71, -109, -98, -89, 7, 99, 79, ConstantPoolEntry.CP_InterfaceMethodref, 6, -98, -111, -69, 7, 99, 89, 1, 6, -109, -97, -90, 94, 45, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 2, 74, -35, -124, -79, 3, 38, 13, 13, 73, -112, -34, -81, 28, 44, 74, 2, 67, -45, -111, -90, 23, TarConstants.LF_LINK, 66, 7, 66, -45, -99, -87, 7, 38, 95, 7, 71, -111, -34, -86, 6, TarConstants.LF_CONTIG, 89, 1, 72, -45, -67, -87, 7, 38, 95, 7, 71, -111, -78, -67, 7, TarConstants.LF_CONTIG, 66, 0}, new byte[]{-16, -56, 115, 67, 45, 110, 38, -3}));
                        vMSmallPreviewCardView.m4993WWWoWWWo((MaterialButton) view);
                        return;
                    case 3:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-44, -14, 87, -15, -123, 101, -80, -113, -41, -57, Byte.MAX_VALUE}, new byte[]{-71, -92, 26, -72, -21, 22, -60, -18});
                            throw null;
                        }
                        return;
                    default:
                        VMSurfaceView vMSurfaceView = vMSmallPreviewCardView.f8670WWWWWWWW;
                        if (vMSurfaceView != null) {
                            if (!vMSurfaceView.f9284WWWWWWWWWW) {
                                vMSmallPreviewCardView.m4991WWWWWWWW();
                                return;
                            }
                            return;
                        }
                        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{20, 125, -42, -85, 36, 81, 71, 109, 26, 78, -51, -111, TarConstants.LF_BLK, 84}, new byte[]{121, 43, -101, -8, 81, 35, 33, ConstantPoolEntry.CP_NameAndType});
                        throw null;
                }
            }
        });
        View findViewById7 = findViewById(R.id.settings);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById7, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{93, -68, 92, -57, -89, -48, -71, 109, 121, -84, 123, -57, -39, -105, -14, TarConstants.LF_BLK, 18}, new byte[]{59, -43, TarConstants.LF_SYMLINK, -93, -15, -71, -36, 26}));
        ((MaterialButton) findViewById7).setOnClickListener(new View.OnClickListener(this) { // from class: k4.oેᄈે

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMSmallPreviewCardView f29308WWWWWWWWWW;

            {
                this.f29308WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMSmallPreviewCardView vMSmallPreviewCardView = this.f29308WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{19, 72, -87, -1, -115, -52, -29, 37, 16, 125, -127}, new byte[]{126, 30, -28, -74, -29, -65, -105, 68});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-13, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_CHR, 91, -63, -34, -16, 109, 121}, new byte[]{-98, 14, 28, 69, 93, 40, -75, -65});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{31, -80, 63, 21, -12, -91, -123, -3, 28, -123, 23}, new byte[]{114, -26, 114, 92, -102, -42, -15, -100});
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, -67, 31, 47, 13, 13, 71, -109, -98, -89, 7, 99, 79, ConstantPoolEntry.CP_InterfaceMethodref, 6, -98, -111, -69, 7, 99, 89, 1, 6, -109, -97, -90, 94, 45, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 2, 74, -35, -124, -79, 3, 38, 13, 13, 73, -112, -34, -81, 28, 44, 74, 2, 67, -45, -111, -90, 23, TarConstants.LF_LINK, 66, 7, 66, -45, -99, -87, 7, 38, 95, 7, 71, -111, -34, -86, 6, TarConstants.LF_CONTIG, 89, 1, 72, -45, -67, -87, 7, 38, 95, 7, 71, -111, -78, -67, 7, TarConstants.LF_CONTIG, 66, 0}, new byte[]{-16, -56, 115, 67, 45, 110, 38, -3}));
                        vMSmallPreviewCardView.m4993WWWoWWWo((MaterialButton) view);
                        return;
                    case 3:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-44, -14, 87, -15, -123, 101, -80, -113, -41, -57, Byte.MAX_VALUE}, new byte[]{-71, -92, 26, -72, -21, 22, -60, -18});
                            throw null;
                        }
                        return;
                    default:
                        VMSurfaceView vMSurfaceView = vMSmallPreviewCardView.f8670WWWWWWWW;
                        if (vMSurfaceView != null) {
                            if (!vMSurfaceView.f9284WWWWWWWWWW) {
                                vMSmallPreviewCardView.m4991WWWWWWWW();
                                return;
                            }
                            return;
                        }
                        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{20, 125, -42, -85, 36, 81, 71, 109, 26, 78, -51, -111, TarConstants.LF_BLK, 84}, new byte[]{121, 43, -101, -8, 81, 35, 33, ConstantPoolEntry.CP_NameAndType});
                        throw null;
                }
            }
        });
        m4994WWoWWo();
        setOnClickListener(new View.OnClickListener(this) { // from class: k4.oેᄈે

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMSmallPreviewCardView f29308WWWWWWWWWW;

            {
                this.f29308WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMSmallPreviewCardView vMSmallPreviewCardView = this.f29308WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{19, 72, -87, -1, -115, -52, -29, 37, 16, 125, -127}, new byte[]{126, 30, -28, -74, -29, -65, -105, 68});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-13, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 81, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_CHR, 91, -63, -34, -16, 109, 121}, new byte[]{-98, 14, 28, 69, 93, 40, -75, -65});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{31, -80, 63, 21, -12, -91, -123, -3, 28, -123, 23}, new byte[]{114, -26, 114, 92, -102, -42, -15, -100});
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, -67, 31, 47, 13, 13, 71, -109, -98, -89, 7, 99, 79, ConstantPoolEntry.CP_InterfaceMethodref, 6, -98, -111, -69, 7, 99, 89, 1, 6, -109, -97, -90, 94, 45, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 2, 74, -35, -124, -79, 3, 38, 13, 13, 73, -112, -34, -81, 28, 44, 74, 2, 67, -45, -111, -90, 23, TarConstants.LF_LINK, 66, 7, 66, -45, -99, -87, 7, 38, 95, 7, 71, -111, -34, -86, 6, TarConstants.LF_CONTIG, 89, 1, 72, -45, -67, -87, 7, 38, 95, 7, 71, -111, -78, -67, 7, TarConstants.LF_CONTIG, 66, 0}, new byte[]{-16, -56, 115, 67, 45, 110, 38, -3}));
                        vMSmallPreviewCardView.m4993WWWoWWWo((MaterialButton) view);
                        return;
                    case 3:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMSmallPreviewCardView.f8674WWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMSmallPreviewCardView.f8673WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-44, -14, 87, -15, -123, 101, -80, -113, -41, -57, Byte.MAX_VALUE}, new byte[]{-71, -92, 26, -72, -21, 22, -60, -18});
                            throw null;
                        }
                        return;
                    default:
                        VMSurfaceView vMSurfaceView = vMSmallPreviewCardView.f8670WWWWWWWW;
                        if (vMSurfaceView != null) {
                            if (!vMSurfaceView.f9284WWWWWWWWWW) {
                                vMSmallPreviewCardView.m4991WWWWWWWW();
                                return;
                            }
                            return;
                        }
                        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{20, 125, -42, -85, 36, 81, 71, 109, 26, 78, -51, -111, TarConstants.LF_BLK, 84}, new byte[]{121, 43, -101, -8, 81, 35, 33, ConstantPoolEntry.CP_NameAndType});
                        throw null;
                }
            }
        });
    }

    @Override // com.google.android.material.card.MaterialCardView, androidx.cardview.widget.CardView, android.widget.FrameLayout, android.view.View
    public final void onMeasure(int i10, int i11) {
        super.onMeasure(i10, View.MeasureSpec.makeMeasureSpec((int) (View.MeasureSpec.getSize(i10) / ((WWWW.m5329WWWWWWWW() * 1.0f) / WWWW.m5328WWWWWWWW())), 1073741824));
    }

    @InterfaceC2472WWWWWWWW(sticky = AndUn7z.f28850WWWWWWWW, threadMode = ThreadMode.MAIN)
    public final void onVMConfigEvent(VMConfigEvent vMConfigEvent) {
        byte[] bArr = {82, 116, TarConstants.LF_GNUTYPE_LONGLINK, 46, 38};
        byte[] bArr2 = {TarConstants.LF_CONTIG, 2, 46, 64, 82, 87, 106, 96};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMConfigEvent, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        TextView textView = this.f8677WWWW;
        if (textView != null) {
            VMInstance vMInstance = this.f8673WWWWWWWW;
            if (vMInstance != null) {
                textView.setText(vMInstance.f8937WWWoWWWo.f8861WWWWoWWWWo);
                return;
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-79, Byte.MIN_VALUE, -37, -122, 67, TarConstants.LF_SYMLINK, -73, 118, -78, -75, -13}, new byte[]{-36, -42, -106, -49, 45, 65, -61, 23}));
                throw null;
            }
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-9, -117, 46, 7, -70, 118, -115, -26, -19}, new byte[]{-102, -59, 79, 106, -33, 32, -28, -125}));
        throw null;
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.MAIN)
    public final void onVMOutsidePageRequestEvent(WWWoWWWo wWWoWWWo) {
        byte[] bArr = {-119, TarConstants.LF_NORMAL, TarConstants.LF_LINK, 30, 60, 93, 69, 17};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(wWWoWWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-20, 70, 84, 112, 72}, bArr));
        InterfaceC3250WWoWWo interfaceC3250WWoWWo = this.f8674WWWWWW;
        if (interfaceC3250WWoWWo != null) {
            VMInstance vMInstance = this.f8673WWWWWWWW;
            if (vMInstance != null) {
                interfaceC3250WWoWWo.mo4976WWWWWWWW(vMInstance, wWWoWWWo);
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-49, -89, -126, -100, 79, -61, 7, -13, -52, -110, -86}, new byte[]{-94, -15, -49, -43, 33, -80, 115, -110}));
                throw null;
            }
        }
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.MAIN)
    public final void onVMPermissionEvent(PermissionEvent permissionEvent) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(permissionEvent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-109, -114, 74, 66, -108}, new byte[]{-10, -8, 47, 44, -32, -87, 105, -35}));
        InterfaceC3250WWoWWo interfaceC3250WWoWWo = this.f8674WWWWWW;
        if (interfaceC3250WWoWWo != null) {
            VMInstance vMInstance = this.f8673WWWWWWWW;
            if (vMInstance != null) {
                interfaceC3250WWoWWo.mo4983WWoWWo(vMInstance, permissionEvent);
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, 43, TarConstants.LF_BLK, 40, 8, 73, 96, 96, 72, 30, 28}, new byte[]{38, 125, 121, 97, 102, 58, 20, 1}));
                throw null;
            }
        }
    }

    @InterfaceC2472WWWWWWWW(sticky = AndUn7z.f28850WWWWWWWW, threadMode = ThreadMode.MAIN)
    public final void onVMStatusEvent(VMStatusEvent vMStatusEvent) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMStatusEvent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-44, -31, -8, -7, 87}, new byte[]{-79, -105, -99, -105, 35, -30, 10, -56}));
        MaterialButton materialButton = this.f8675WWoWWo;
        if (materialButton != null) {
            m4992WWWWWWWW(materialButton);
            MaterialButton materialButton2 = this.f8672WWWWWWWW;
            if (materialButton2 != null) {
                m4993WWWoWWWo(materialButton2);
                return;
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{71, -6, 126, -6, -41, 74, 97, -70, 95, -40, 101, -7, -52}, new byte[]{42, -84, 17, -106, -94, 39, 4, -8}));
                throw null;
            }
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-72, 28, 16, 60, 106, -48, TarConstants.LF_GNUTYPE_LONGNAME, 70, -95, 59, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_CHR}, new byte[]{-43, 79, 100, 93, 24, -92, 14, TarConstants.LF_CHR}));
        throw null;
    }

    @Override // android.view.View
    public final void onWindowVisibilityChanged(int i10) {
        if (i10 == 0) {
            VMInstance vMInstance = this.f8673WWWWWWWW;
            if (vMInstance != null) {
                C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
                if (!c2467wwwwwwww.m13948WWoWWo(this)) {
                    c2467wwwwwwww.m13950WWWW(this);
                }
            } else {
                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{57, -87, -8, -37, -89, 16, -69, TarConstants.LF_MULTIVOLUME, 58, -100, -48}, new byte[]{84, -1, -75, -110, -55, 99, -49, 44});
                throw null;
            }
        } else {
            VMInstance vMInstance2 = this.f8673WWWWWWWW;
            if (vMInstance2 != null) {
                C2467WWWWWWWW c2467wwwwwwww2 = vMInstance2.f8939WWWoWWWo;
                if (c2467wwwwwwww2.m13948WWoWWo(this)) {
                    c2467wwwwwwww2.m13945WWWWWWWW(this);
                }
            } else {
                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, 72, -104, 6, 23, -7, -85, -32, 8, 125, -80}, new byte[]{102, 30, -43, 79, 121, -118, -33, -127});
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
        VMSurfaceView vMSurfaceView = this.f8670WWWWWWWW;
        if (vMSurfaceView != null) {
            vMSurfaceView.setTouchEnabled(z10);
            if (i11 == 2) {
                VMSurfaceView vMSurfaceView2 = this.f8670WWWWWWWW;
                if (vMSurfaceView2 != null) {
                    vMSurfaceView2.setTouchEventHandler(new C3970WWWoWWWo());
                } else {
                    WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-42, -82, 7, TarConstants.LF_MULTIVOLUME, 22, -72, -114, 116, -40, -99, 28, 119, 6, -67}, new byte[]{-69, -8, 74, 30, 99, -54, -24, 21});
                    throw null;
                }
            } else {
                VMSurfaceView vMSurfaceView3 = this.f8670WWWWWWWW;
                if (vMSurfaceView3 != null) {
                    vMSurfaceView3.setTouchEventHandler(null);
                } else {
                    WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, 107, -41, 95, 98, -17, 59, -120, 5, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -52, 101, 114, -22}, new byte[]{102, 61, -102, ConstantPoolEntry.CP_NameAndType, 23, -99, 93, -23});
                    throw null;
                }
            }
            if (z10) {
                View view = this.f8671WWWWWWWW;
                if (view != null) {
                    if (view.getVisibility() == 0) {
                        m4994WWoWWo();
                        return;
                    }
                    return;
                }
                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{45, -30, 31, -61, 62, TarConstants.LF_CHR, 9, 122, TarConstants.LF_CHR, -5, 0, -61, 59}, new byte[]{64, -83, 105, -90, TarConstants.LF_GNUTYPE_LONGNAME, 95, 104, 3});
                throw null;
            }
            return;
        }
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-23, 68, -127, -33, -30, 6, -41, 14, -25, 119, -102, -27, -14, 3}, new byte[]{-124, 18, -52, -116, -105, 116, -79, 111});
        throw null;
    }

    public final void setVMInstance(VMInstance vMInstance) {
        boolean z10 = true;
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-34, 1}, new byte[]{-88, 108, -117, 91, 40, 25, 20, 2}));
        this.f8673WWWWWWWW = vMInstance;
        C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
        if (!c2467wwwwwwww.m13948WWoWWo(this)) {
            c2467wwwwwwww.m13950WWWW(this);
        }
        MaterialButton materialButton = this.f8675WWoWWo;
        if (materialButton != null) {
            m4992WWWWWWWW(materialButton);
            MaterialButton materialButton2 = this.f8672WWWWWWWW;
            if (materialButton2 != null) {
                m4993WWWoWWWo(materialButton2);
                TextView textView = this.f8677WWWW;
                if (textView != null) {
                    textView.setText(vMInstance.f8937WWWoWWWo.f8861WWWWoWWWWo);
                    View findViewById = findViewById(R.id.vmsurfaceview);
                    AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_SPARSE, -44, -28, 115, 60, -91, 10, 19, 119, -60, -61, 115, 66, -30, 65, 74, 28}, new byte[]{TarConstants.LF_DIR, -67, -118, 23, 106, -52, 111, 100}));
                    VMSurfaceView vMSurfaceView = (VMSurfaceView) findViewById;
                    this.f8670WWWWWWWW = vMSurfaceView;
                    vMSurfaceView.setVM(vMInstance);
                    VMSurfaceView vMSurfaceView2 = this.f8670WWWWWWWW;
                    if (vMSurfaceView2 != null) {
                        vMSurfaceView2.setCornerRadius(WWWW.m5340WWoWWo(8.0f));
                        VMSurfaceView vMSurfaceView3 = this.f8670WWWWWWWW;
                        if (vMSurfaceView3 != null) {
                            vMSurfaceView3.setTouchEnabled(false);
                            VMSurfaceView vMSurfaceView4 = this.f8670WWWWWWWW;
                            if (vMSurfaceView4 != null) {
                                if (WWWW.m5336WWWoWWWo().getResources().getConfiguration().orientation != 2) {
                                    z10 = false;
                                }
                                vMSurfaceView4.setLandscape(z10);
                                return;
                            }
                            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-126, 39, -114, -15, 15, 74, TarConstants.LF_CHR, -119, -116, 20, -107, -53, 31, 79}, new byte[]{-17, 113, -61, -94, 122, 56, 85, -24}));
                            throw null;
                        }
                        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{111, -111, 93, -88, 66, -30, -119, -116, 97, -94, 70, -110, 82, -25}, new byte[]{2, -57, 16, -5, TarConstants.LF_CONTIG, -112, -17, -19}));
                        throw null;
                    }
                    AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-94, -68, -3, 6, -83, -41, 27, 98, -84, -113, -26, 60, -67, -46}, new byte[]{-49, -22, -80, 85, -40, -91, 125, 3}));
                    throw null;
                }
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{3, -124, -118, -117, -124, -109, -39, 66, 25}, new byte[]{110, -54, -21, -26, -31, -59, -80, 39}));
                throw null;
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-113, 10, TarConstants.LF_DIR, 116, -57, 104, -56, 97, -105, 40, 46, 119, -36}, new byte[]{-30, 92, 90, 24, -78, 5, -83, 35}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{94, -59, -92, 85, 46, 90, -119, 106, 71, -30, -65, 90}, new byte[]{TarConstants.LF_CHR, -106, -48, TarConstants.LF_BLK, 92, 46, -53, 31}));
        throw null;
    }

    public final void setVMViewActionCallback(InterfaceC3250WWoWWo interfaceC3250WWoWWo) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(interfaceC3250WWoWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{98, -71, -5, -52, ConstantPoolEntry.CP_InterfaceMethodref, 9, 62, 70}, new byte[]{1, -40, -105, -96, 105, 104, 93, 45}));
        this.f8674WWWWWW = interfaceC3250WWoWWo;
    }

    @Override // android.view.View
    public void setVisibility(int i10) {
        if (i10 != 0) {
            super.setVisibility(i10);
            VMSurfaceView vMSurfaceView = this.f8670WWWWWWWW;
            if (vMSurfaceView != null) {
                vMSurfaceView.setVisibility(i10);
                return;
            }
            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-91, -95, -45, -19, -83, -86, 35, -54, -85, -110, -56, -41, -67, -81}, new byte[]{-56, -9, -98, -66, -40, -40, 69, -85});
            throw null;
        }
        post(new p021WWWWWWWW.WWWW(i10, 2, this));
    }

    /* JADX WARN: 'this' call moved to the top of the method (can break code semantics) */
    public VMSmallPreviewCardView(Context context, AttributeSet attributeSet) {
        this(context, attributeSet, 4, 0);
        byte[] bArr = {125, 112, TarConstants.LF_CHR, -62, 27, 94, -51, -39};
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{30, 31, 93, -74, 126, 38, -71}, bArr, context);
    }

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public VMSmallPreviewCardView(Context context, AttributeSet attributeSet, int i10) {
        super(context, attributeSet, i10);
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{3, -77, 118, 59, -27, -110, 36}, new byte[]{96, -36, 24, 79, Byte.MIN_VALUE, -22, 80, -121}, context);
    }

    public /* synthetic */ VMSmallPreviewCardView(Context context, AttributeSet attributeSet, int i10, int i11) {
        this(context, (i10 & 2) != 0 ? null : attributeSet, 0);
    }
}
