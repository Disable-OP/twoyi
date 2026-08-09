package com.android.vmapp.ui.vm.main;

import android.content.Context;
import android.text.method.ScrollingMovementMethod;
import android.util.AttributeSet;
import android.view.View;
import android.widget.TextView;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmapp.ui.vm.main.VMSmallCardView;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.VMConfigEvent;
import com.android.vmcore.event.VMStatusEvent;
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
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.greenrobot.eventbus.ThreadMode;
/* loaded from: classes.dex */
public final class VMSmallCardView extends MaterialCardView {

    /* renamed from: WWWWሗWWWWшሗ  reason: contains not printable characters */
    public TextView f8664WWWWWWWW;

    /* renamed from: WWWWኈWWWWˡኈ  reason: contains not printable characters */
    public TextView f8665WWWWWWWW;

    /* renamed from: WWWWዪWWWWͯዪ  reason: contains not printable characters */
    public VMInstance f8666WWWWWWWW;

    /* renamed from: WWWWᏊWWWWటᏊ  reason: contains not printable characters */
    public InterfaceC3250WWoWWo f8667WWWWWWWW;

    /* renamed from: WWoዕWWoూዕ  reason: contains not printable characters */
    public MaterialButton f8668WWoWWo;

    /* renamed from: WWኮWWႉኮ  reason: contains not printable characters */
    public MaterialButton f8669WWWW;

    /* JADX WARN: 'this' call moved to the top of the method (can break code semantics) */
    public VMSmallCardView(Context context) {
        this(context, null, 6, 0);
        byte[] bArr = {-68, TarConstants.LF_CHR, 64, 74, 91, 22, 37, 90};
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-33, 92, 46, 62, 62, 110, 81}, bArr, context);
    }

    /* renamed from: WWoڢWWo࢞ڢ  reason: not valid java name and contains not printable characters */
    public final void m4988WWoWWo(MaterialButton materialButton) {
        VMInstance vMInstance = this.f8666WWWWWWWW;
        if (vMInstance != null) {
            if (vMInstance.m5062WWWWWWWW()) {
                materialButton.setIconResource(R.drawable.outline_volume_off_24);
                return;
            } else {
                materialButton.setIconResource(R.drawable.outline_volume_up_24);
                return;
            }
        }
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{73, -117, -92, 95, -1, -74, -54, -24, 74, -66, -116}, new byte[]{36, -35, -23, 22, -111, -59, -66, -119});
        throw null;
    }

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public final void m4989WWWW(MaterialButton materialButton) {
        VMInstance vMInstance = this.f8666WWWWWWWW;
        if (vMInstance != null) {
            int i10 = vMInstance.f8940WWoWWo;
            if (i10 == -5) {
                materialButton.setText(R.string.vm_stopping);
                return;
            } else if (i10 < 0) {
                materialButton.setText(R.string.vm_error);
                return;
            } else if (i10 == 0) {
                materialButton.setText(R.string.vm_start_short);
                return;
            } else if (i10 < 6) {
                materialButton.setText(R.string.vm_starting);
                return;
            } else {
                materialButton.setText(R.string.vm_enter_short);
                return;
            }
        }
        byte[] bArr = {-35, -81, -25, -4, 20, TarConstants.LF_LINK, 82, -50};
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-80, -7, -86, -75, 122, 66, 38, -81, -77, -52, -126}, bArr);
        throw null;
    }

    @Override // android.view.View
    public final void onFinishInflate() {
        super.onFinishInflate();
        View findViewById = findViewById(R.id.name);
        AbstractC1017WWWoWWWo.m3443WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-119, TarConstants.LF_GNUTYPE_SPARSE, 25, 82, -8, -120, -120, 56, -83, 67, 62, 82, -122, -49, -61, 97, -58}, new byte[]{-17, 58, 119, TarConstants.LF_FIFO, -82, -31, -19, 79}, findViewById);
        this.f8664WWWWWWWW = (TextView) findViewById;
        View findViewById2 = findViewById(R.id.slogan);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{79, -86, -74, -71, -18, -73, 67, -10, 107, -70, -111, -71, -112, -16, 8, -81, 0}, new byte[]{41, -61, -40, -35, -72, -34, 38, -127}));
        TextView textView = (TextView) findViewById2;
        this.f8665WWWWWWWW = textView;
        textView.setMovementMethod(new ScrollingMovementMethod());
        if (!WWWW.m5349o()) {
            TextView textView2 = this.f8665WWWWWWWW;
            if (textView2 != null) {
                textView2.setVisibility(8);
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-83, 29, 33, -106, 46, 59, 107, 22, -87, 43, 58}, new byte[]{-64, 78, TarConstants.LF_MULTIVOLUME, -7, 73, 90, 5, 64}));
                throw null;
            }
        }
        View findViewById3 = findViewById(R.id.start);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-101, 20, -93, 40, -92, 27, -117, -57, -65, 4, -124, 40, -38, 92, -64, -98, -44}, new byte[]{-3, 125, -51, TarConstants.LF_GNUTYPE_LONGNAME, -14, 114, -18, -80}));
        MaterialButton materialButton = (MaterialButton) findViewById3;
        this.f8669WWWW = materialButton;
        materialButton.setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWoহWWoȗহ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMSmallCardView f29302WWWWWWWWWW;

            {
                this.f29302WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMSmallCardView vMSmallCardView = this.f29302WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-61, 24, TarConstants.LF_GNUTYPE_LONGNAME, 57, 47, 97, 107, -109, -64, 45, 100}, new byte[]{-82, 78, 1, 112, 65, 18, 31, -14});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{110, -79, -115, -66, 110, 1, 111, -13, 109, -124, -91}, new byte[]{3, -25, -64, -9, 0, 114, 27, -110});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                byte[] bArr = {107, -20, -37, -69, 59, -27, 1, ConstantPoolEntry.CP_InterfaceMethodref};
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{6, -70, -106, -14, 85, -106, 117, 106, 5, -113, -66}, bArr);
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{32, -32, 0, -69, -4, -2, -118, 17, 32, -6, 24, -9, -66, -8, -53, 28, 47, -26, 24, -9, -88, -14, -53, 17, 33, -5, 65, -71, -87, -15, -121, 95, 58, -20, 28, -78, -4, -2, -124, 18, 96, -14, 3, -72, -69, -15, -114, 81, 47, -5, 8, -91, -77, -12, -113, 81, 35, -12, 24, -78, -82, -12, -118, 19, 96, -9, 25, -93, -88, -14, -123, 81, 3, -12, 24, -78, -82, -12, -118, 19, ConstantPoolEntry.CP_NameAndType, -32, 24, -93, -77, -13}, new byte[]{78, -107, 108, -41, -36, -99, -21, Byte.MAX_VALUE}));
                        vMSmallCardView.m4988WWoWWo((MaterialButton) view);
                        return;
                    default:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{47, -101, TarConstants.LF_DIR, TarConstants.LF_MULTIVOLUME, 105, -83, TarConstants.LF_SYMLINK, -125, 44, -82, 29}, new byte[]{66, -51, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 4, 7, -34, 70, -30});
                            throw null;
                        }
                        return;
                }
            }
        });
        View findViewById4 = findViewById(R.id.shutdown);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById4, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-1, -51, -8, 98, 30, 59, 38, -73, -37, -35, -33, 98, 96, 124, 109, -18, -80}, new byte[]{-103, -92, -106, 6, 72, 82, 67, -64}));
        ((MaterialButton) findViewById4).setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWoহWWoȗহ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMSmallCardView f29302WWWWWWWWWW;

            {
                this.f29302WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMSmallCardView vMSmallCardView = this.f29302WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-61, 24, TarConstants.LF_GNUTYPE_LONGNAME, 57, 47, 97, 107, -109, -64, 45, 100}, new byte[]{-82, 78, 1, 112, 65, 18, 31, -14});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{110, -79, -115, -66, 110, 1, 111, -13, 109, -124, -91}, new byte[]{3, -25, -64, -9, 0, 114, 27, -110});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                byte[] bArr = {107, -20, -37, -69, 59, -27, 1, ConstantPoolEntry.CP_InterfaceMethodref};
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{6, -70, -106, -14, 85, -106, 117, 106, 5, -113, -66}, bArr);
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{32, -32, 0, -69, -4, -2, -118, 17, 32, -6, 24, -9, -66, -8, -53, 28, 47, -26, 24, -9, -88, -14, -53, 17, 33, -5, 65, -71, -87, -15, -121, 95, 58, -20, 28, -78, -4, -2, -124, 18, 96, -14, 3, -72, -69, -15, -114, 81, 47, -5, 8, -91, -77, -12, -113, 81, 35, -12, 24, -78, -82, -12, -118, 19, 96, -9, 25, -93, -88, -14, -123, 81, 3, -12, 24, -78, -82, -12, -118, 19, ConstantPoolEntry.CP_NameAndType, -32, 24, -93, -77, -13}, new byte[]{78, -107, 108, -41, -36, -99, -21, Byte.MAX_VALUE}));
                        vMSmallCardView.m4988WWoWWo((MaterialButton) view);
                        return;
                    default:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{47, -101, TarConstants.LF_DIR, TarConstants.LF_MULTIVOLUME, 105, -83, TarConstants.LF_SYMLINK, -125, 44, -82, 29}, new byte[]{66, -51, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 4, 7, -34, 70, -30});
                            throw null;
                        }
                        return;
                }
            }
        });
        View findViewById5 = findViewById(R.id.volume);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById5, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{105, -36, -10, 26, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -101, 90, 116, TarConstants.LF_MULTIVOLUME, -52, -47, 26, 6, -36, 17, 45, 38}, new byte[]{15, -75, -104, 126, 46, -14, 63, 3}));
        MaterialButton materialButton2 = (MaterialButton) findViewById5;
        this.f8668WWoWWo = materialButton2;
        materialButton2.setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWoহWWoȗহ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMSmallCardView f29302WWWWWWWWWW;

            {
                this.f29302WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMSmallCardView vMSmallCardView = this.f29302WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-61, 24, TarConstants.LF_GNUTYPE_LONGNAME, 57, 47, 97, 107, -109, -64, 45, 100}, new byte[]{-82, 78, 1, 112, 65, 18, 31, -14});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{110, -79, -115, -66, 110, 1, 111, -13, 109, -124, -91}, new byte[]{3, -25, -64, -9, 0, 114, 27, -110});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                byte[] bArr = {107, -20, -37, -69, 59, -27, 1, ConstantPoolEntry.CP_InterfaceMethodref};
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{6, -70, -106, -14, 85, -106, 117, 106, 5, -113, -66}, bArr);
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{32, -32, 0, -69, -4, -2, -118, 17, 32, -6, 24, -9, -66, -8, -53, 28, 47, -26, 24, -9, -88, -14, -53, 17, 33, -5, 65, -71, -87, -15, -121, 95, 58, -20, 28, -78, -4, -2, -124, 18, 96, -14, 3, -72, -69, -15, -114, 81, 47, -5, 8, -91, -77, -12, -113, 81, 35, -12, 24, -78, -82, -12, -118, 19, 96, -9, 25, -93, -88, -14, -123, 81, 3, -12, 24, -78, -82, -12, -118, 19, ConstantPoolEntry.CP_NameAndType, -32, 24, -93, -77, -13}, new byte[]{78, -107, 108, -41, -36, -99, -21, Byte.MAX_VALUE}));
                        vMSmallCardView.m4988WWoWWo((MaterialButton) view);
                        return;
                    default:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{47, -101, TarConstants.LF_DIR, TarConstants.LF_MULTIVOLUME, 105, -83, TarConstants.LF_SYMLINK, -125, 44, -82, 29}, new byte[]{66, -51, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 4, 7, -34, 70, -30});
                            throw null;
                        }
                        return;
                }
            }
        });
        View findViewById6 = findViewById(R.id.settings);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById6, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{80, -83, 56, 109, -89, -65, -43, -10, 116, -67, 31, 109, -39, -8, -98, -81, 31}, new byte[]{TarConstants.LF_FIFO, -60, 86, 9, -15, -42, -80, -127}));
        ((MaterialButton) findViewById6).setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWoহWWoȗহ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMSmallCardView f29302WWWWWWWWWW;

            {
                this.f29302WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMSmallCardView vMSmallCardView = this.f29302WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-61, 24, TarConstants.LF_GNUTYPE_LONGNAME, 57, 47, 97, 107, -109, -64, 45, 100}, new byte[]{-82, 78, 1, 112, 65, 18, 31, -14});
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{110, -79, -115, -66, 110, 1, 111, -13, 109, -124, -91}, new byte[]{3, -25, -64, -9, 0, 114, 27, -110});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                byte[] bArr = {107, -20, -37, -69, 59, -27, 1, ConstantPoolEntry.CP_InterfaceMethodref};
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{6, -70, -106, -14, 85, -106, 117, 106, 5, -113, -66}, bArr);
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{32, -32, 0, -69, -4, -2, -118, 17, 32, -6, 24, -9, -66, -8, -53, 28, 47, -26, 24, -9, -88, -14, -53, 17, 33, -5, 65, -71, -87, -15, -121, 95, 58, -20, 28, -78, -4, -2, -124, 18, 96, -14, 3, -72, -69, -15, -114, 81, 47, -5, 8, -91, -77, -12, -113, 81, 35, -12, 24, -78, -82, -12, -118, 19, 96, -9, 25, -93, -88, -14, -123, 81, 3, -12, 24, -78, -82, -12, -118, 19, ConstantPoolEntry.CP_NameAndType, -32, 24, -93, -77, -13}, new byte[]{78, -107, 108, -41, -36, -99, -21, Byte.MAX_VALUE}));
                        vMSmallCardView.m4988WWoWWo((MaterialButton) view);
                        return;
                    default:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMSmallCardView.f8667WWWWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMSmallCardView.f8666WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{47, -101, TarConstants.LF_DIR, TarConstants.LF_MULTIVOLUME, 105, -83, TarConstants.LF_SYMLINK, -125, 44, -82, 29}, new byte[]{66, -51, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 4, 7, -34, 70, -30});
                            throw null;
                        }
                        return;
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
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMConfigEvent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{93, -24, -11, -69, -81}, new byte[]{56, -98, -112, -43, -37, 4, -11, 87}));
        TextView textView = this.f8664WWWWWWWW;
        if (textView != null) {
            VMInstance vMInstance = this.f8666WWWWWWWW;
            if (vMInstance != null) {
                textView.setText(vMInstance.f8937WWWoWWWo.f8861WWWWoWWWWo);
                return;
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{60, TarConstants.LF_FIFO, 74, 126, -96, -56, -38, 78, 63, 3, 98}, new byte[]{81, 96, 7, TarConstants.LF_CONTIG, -50, -69, -82, 47}));
                throw null;
            }
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-120, 124, -111, -127, -106, 87, -92, 16, -110}, new byte[]{-27, TarConstants.LF_SYMLINK, -16, -20, -13, 1, -51, 117}));
        throw null;
    }

    @InterfaceC2472WWWWWWWW(sticky = AndUn7z.f28850WWWWWWWW, threadMode = ThreadMode.MAIN)
    public final void onVMStatusEvent(VMStatusEvent vMStatusEvent) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMStatusEvent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-95, 102, -65, 97, TarConstants.LF_NORMAL}, new byte[]{-60, 16, -38, 15, 68, 90, 126, -43}));
        MaterialButton materialButton = this.f8669WWWW;
        if (materialButton != null) {
            m4989WWWW(materialButton);
            MaterialButton materialButton2 = this.f8668WWoWWo;
            if (materialButton2 != null) {
                m4988WWoWWo(materialButton2);
                return;
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{8, -77, 32, 59, -65, 74, 117, 24, 16, -111, 59, 56, -92}, new byte[]{101, -27, 79, 87, -54, 39, 16, 90}));
                throw null;
            }
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-80, -52, 3, 102, 56, -37, -62, TarConstants.LF_LINK, -87, -21, 24, 105}, new byte[]{-35, -97, 119, 7, 74, -81, Byte.MIN_VALUE, 68}));
        throw null;
    }

    @Override // android.view.View
    public final void onWindowVisibilityChanged(int i10) {
        if (i10 == 0) {
            VMInstance vMInstance = this.f8666WWWWWWWW;
            if (vMInstance != null) {
                C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
                if (!c2467wwwwwwww.m13948WWoWWo(this)) {
                    c2467wwwwwwww.m13950WWWW(this);
                }
            } else {
                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-101, -15, -56, 95, 111, -55, -64, -65, -104, -60, -32}, new byte[]{-10, -89, -123, 22, 1, -70, -76, -34});
                throw null;
            }
        } else {
            VMInstance vMInstance2 = this.f8666WWWWWWWW;
            if (vMInstance2 != null) {
                C2467WWWWWWWW c2467wwwwwwww2 = vMInstance2.f8939WWWoWWWo;
                if (c2467wwwwwwww2.m13948WWoWWo(this)) {
                    c2467wwwwwwww2.m13945WWWWWWWW(this);
                }
            } else {
                byte[] bArr = {-39, 118, TarConstants.LF_SYMLINK, -114, 65, -73, -74, -119};
                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-76, 32, Byte.MAX_VALUE, -57, 47, -60, -62, -24, -73, 21, 87}, bArr);
                throw null;
            }
        }
        super.onWindowVisibilityChanged(i10);
    }

    public final void setVMInstance(VMInstance vMInstance) {
        byte[] bArr = {TarConstants.LF_CHR, 13};
        byte[] bArr2 = {69, 96, 28, TarConstants.LF_DIR, 6, -40, 29, -84};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        this.f8666WWWWWWWW = vMInstance;
        C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
        if (!c2467wwwwwwww.m13948WWoWWo(this)) {
            c2467wwwwwwww.m13950WWWW(this);
        }
        MaterialButton materialButton = this.f8669WWWW;
        if (materialButton != null) {
            m4989WWWW(materialButton);
            MaterialButton materialButton2 = this.f8668WWoWWo;
            if (materialButton2 != null) {
                m4988WWoWWo(materialButton2);
                TextView textView = this.f8664WWWWWWWW;
                if (textView != null) {
                    textView.setText(vMInstance.f8937WWWoWWWo.f8861WWWWoWWWWo);
                    return;
                } else {
                    AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-10, -49, -63, 122, 38, -64, -54, 25, -20}, new byte[]{-101, -127, -96, 23, 67, -106, -93, 124}));
                    throw null;
                }
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-44, 3, 101, 63, -93, -111, 106, -2, -52, 33, 126, 60, -72}, new byte[]{-71, 85, 10, TarConstants.LF_GNUTYPE_SPARSE, -42, -4, 15, -68}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{74, 122, 29, 30, -28, -77, -46, -60, TarConstants.LF_GNUTYPE_SPARSE, 93, 6, 17}, new byte[]{39, 41, 105, Byte.MAX_VALUE, -106, -57, -112, -79}));
        throw null;
    }

    public final void setVMViewActionCallback(InterfaceC3250WWoWWo interfaceC3250WWoWWo) {
        byte[] bArr = {TarConstants.LF_GNUTYPE_LONGNAME, -46, -11, -46, -80, -14, 67, -17};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(interfaceC3250WWoWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{47, -77, -103, -66, -46, -109, 32, -124}, bArr));
        this.f8667WWWWWWWW = interfaceC3250WWoWWo;
    }

    /* JADX WARN: 'this' call moved to the top of the method (can break code semantics) */
    public VMSmallCardView(Context context, AttributeSet attributeSet) {
        this(context, attributeSet, 4, 0);
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{105, -11, TarConstants.LF_MULTIVOLUME, -77, -10, 101, 80}, new byte[]{10, -102, 35, -57, -109, 29, 36, -85}, context);
    }

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public VMSmallCardView(Context context, AttributeSet attributeSet, int i10) {
        super(context, attributeSet, i10);
        byte[] bArr = {TarConstants.LF_CONTIG, 21, 58, Byte.MIN_VALUE, 72, -6, 72, -43};
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{84, 122, 84, -12, 45, -126, 60}, bArr, context);
    }

    public /* synthetic */ VMSmallCardView(Context context, AttributeSet attributeSet, int i10, int i11) {
        this(context, (i10 & 2) != 0 ? null : attributeSet, 0);
    }
}
