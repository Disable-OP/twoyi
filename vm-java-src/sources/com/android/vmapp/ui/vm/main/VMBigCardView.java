package com.android.vmapp.ui.vm.main;

import android.content.Context;
import android.text.method.ScrollingMovementMethod;
import android.util.AttributeSet;
import android.view.View;
import android.widget.TextView;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmapp.ui.vm.main.VMBigCardView;
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
public final class VMBigCardView extends MaterialCardView {

    /* renamed from: WWWWሗWWWWшሗ  reason: contains not printable characters */
    public TextView f8639WWWWWWWW;

    /* renamed from: WWWWኈWWWWˡኈ  reason: contains not printable characters */
    public TextView f8640WWWWWWWW;

    /* renamed from: WWWWዪWWWWͯዪ  reason: contains not printable characters */
    public VMInstance f8641WWWWWWWW;

    /* renamed from: WWWWᏊWWWWటᏊ  reason: contains not printable characters */
    public InterfaceC3250WWoWWo f8642WWWWWWWW;

    /* renamed from: WWoዕWWoూዕ  reason: contains not printable characters */
    public MaterialButton f8643WWoWWo;

    /* renamed from: WWኮWWႉኮ  reason: contains not printable characters */
    public MaterialButton f8644WWWW;

    /* JADX WARN: 'this' call moved to the top of the method (can break code semantics) */
    public VMBigCardView(Context context) {
        this(context, null, 6, 0);
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_CHR, 23, -114, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_GNUTYPE_SPARSE, 43, -100}, new byte[]{80, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -32, TarConstants.LF_PAX_EXTENDED_HEADER_LC, TarConstants.LF_FIFO, TarConstants.LF_GNUTYPE_SPARSE, -24, Byte.MAX_VALUE}, context);
    }

    /* renamed from: WWoڢWWo࢞ڢ  reason: not valid java name and contains not printable characters */
    public final void m4966WWoWWo(MaterialButton materialButton) {
        VMInstance vMInstance = this.f8641WWWWWWWW;
        if (vMInstance != null) {
            if (vMInstance.m5062WWWWWWWW()) {
                materialButton.setIconResource(R.drawable.outline_volume_off_24);
                return;
            } else {
                materialButton.setIconResource(R.drawable.outline_volume_up_24);
                return;
            }
        }
        byte[] bArr = {TarConstants.LF_CONTIG, -123, 92, -103, -29, 8, -99, TarConstants.LF_NORMAL};
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{90, -45, 17, -48, -115, 123, -23, 81, 89, -26, 57}, bArr);
        throw null;
    }

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public final void m4967WWWW(MaterialButton materialButton) {
        VMInstance vMInstance = this.f8641WWWWWWWW;
        if (vMInstance != null) {
            int i10 = vMInstance.f8940WWoWWo;
            if (i10 == -5) {
                materialButton.setText(R.string.vm_stopping);
                return;
            } else if (i10 < 0) {
                materialButton.setText(R.string.vm_error);
                return;
            } else if (i10 == 0) {
                materialButton.setText(R.string.vm_start);
                return;
            } else if (i10 < 6) {
                materialButton.setText(R.string.vm_starting);
                return;
            } else {
                materialButton.setText(R.string.vm_enter);
                return;
            }
        }
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, -2, -21, -103, -7, -19, 40, TarConstants.LF_DIR, 123, -53, -61}, new byte[]{21, -88, -90, -48, -105, -98, 92, 84});
        throw null;
    }

    @Override // android.view.View
    public final void onFinishInflate() {
        super.onFinishInflate();
        View findViewById = findViewById(R.id.name);
        AbstractC1017WWWoWWWo.m3443WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{74, 100, 16, -3, 84, TarConstants.LF_GNUTYPE_LONGNAME, 13, 23, 110, 116, TarConstants.LF_CONTIG, -3, 42, ConstantPoolEntry.CP_InterfaceMethodref, 70, 78, 5}, new byte[]{44, 13, 126, -103, 2, 37, 104, 96}, findViewById);
        this.f8639WWWWWWWW = (TextView) findViewById;
        View findViewById2 = findViewById(R.id.slogan);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_DIR, 114, -9, -101, -112, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -98, TarConstants.LF_LINK, 17, 98, -48, -101, -18, 32, -43, 104, 122}, new byte[]{TarConstants.LF_GNUTYPE_SPARSE, 27, -103, -1, -58, 14, -5, 70}));
        TextView textView = (TextView) findViewById2;
        this.f8640WWWWWWWW = textView;
        textView.setMovementMethod(new ScrollingMovementMethod());
        if (!WWWW.m5349o()) {
            TextView textView2 = this.f8640WWWWWWWW;
            if (textView2 != null) {
                textView2.setVisibility(8);
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-13, 22, -90, 32, 40, -123, -61, 31, -9, 32, -67}, new byte[]{-98, 69, -54, 79, 79, -28, -83, 73}));
                throw null;
            }
        }
        View findViewById3 = findViewById(R.id.start);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-123, 32, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 31, -89, -3, -43, -7, -95, TarConstants.LF_NORMAL, Byte.MAX_VALUE, 31, -39, -70, -98, -96, -54}, new byte[]{-29, 73, TarConstants.LF_FIFO, 123, -15, -108, -80, -114}));
        MaterialButton materialButton = (MaterialButton) findViewById3;
        this.f8644WWWW = materialButton;
        materialButton.setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWoϫWWoӉϫ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMBigCardView f29295WWWWWWWWWW;

            {
                this.f29295WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMBigCardView vMBigCardView = this.f29295WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            byte[] bArr = {108, -49, 10, 35, 81, TarConstants.LF_CONTIG, -127, 72};
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{1, -103, 71, 106, 63, 68, -11, 41, 2, -84, 111}, bArr);
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-7, -113, -4, 110, -4, 0, 47, 15, -6, -70, -44}, new byte[]{-108, -39, -79, 39, -110, 115, 91, 110});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                byte[] bArr2 = {116, 10, ConstantPoolEntry.CP_NameAndType, -69, -4, 17, 41, 62};
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{25, 92, 65, -14, -110, 98, 93, 95, 26, 105, 105}, bArr2);
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{67, -41, 0, -100, 86, -120, 93, 126, 67, -51, 24, -48, 20, -114, 28, 115, TarConstants.LF_GNUTYPE_LONGNAME, -47, 24, -48, 2, -124, 28, 126, 66, -52, 65, -98, 3, -121, 80, TarConstants.LF_NORMAL, 89, -37, 28, -107, 86, -120, TarConstants.LF_GNUTYPE_SPARSE, 125, 3, -59, 3, -97, 17, -121, 89, 62, TarConstants.LF_GNUTYPE_LONGNAME, -52, 8, -126, 25, -126, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 62, 64, -61, 24, -107, 4, -126, 93, 124, 3, -64, 25, -124, 2, -124, 82, 62, 96, -61, 24, -107, 4, -126, 93, 124, 111, -41, 24, -124, 25, -123}, new byte[]{45, -94, 108, -16, 118, -21, 60, 16}));
                        vMBigCardView.m4966WWoWWo((MaterialButton) view);
                        return;
                    default:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_GNUTYPE_SPARSE, 124, 65, 114, 96, -124, 100, 79, 80, 73, 105}, new byte[]{62, 42, ConstantPoolEntry.CP_NameAndType, 59, 14, -9, 16, 46});
                            throw null;
                        }
                        return;
                }
            }
        });
        View findViewById4 = findViewById(R.id.shutdown);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById4, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{86, -25, 121, -72, -32, -26, TarConstants.LF_LINK, 71, 114, -9, 94, -72, -98, -95, 122, 30, 25}, new byte[]{TarConstants.LF_NORMAL, -114, 23, -36, -74, -113, 84, TarConstants.LF_NORMAL}));
        ((MaterialButton) findViewById4).setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWoϫWWoӉϫ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMBigCardView f29295WWWWWWWWWW;

            {
                this.f29295WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMBigCardView vMBigCardView = this.f29295WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            byte[] bArr = {108, -49, 10, 35, 81, TarConstants.LF_CONTIG, -127, 72};
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{1, -103, 71, 106, 63, 68, -11, 41, 2, -84, 111}, bArr);
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-7, -113, -4, 110, -4, 0, 47, 15, -6, -70, -44}, new byte[]{-108, -39, -79, 39, -110, 115, 91, 110});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                byte[] bArr2 = {116, 10, ConstantPoolEntry.CP_NameAndType, -69, -4, 17, 41, 62};
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{25, 92, 65, -14, -110, 98, 93, 95, 26, 105, 105}, bArr2);
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{67, -41, 0, -100, 86, -120, 93, 126, 67, -51, 24, -48, 20, -114, 28, 115, TarConstants.LF_GNUTYPE_LONGNAME, -47, 24, -48, 2, -124, 28, 126, 66, -52, 65, -98, 3, -121, 80, TarConstants.LF_NORMAL, 89, -37, 28, -107, 86, -120, TarConstants.LF_GNUTYPE_SPARSE, 125, 3, -59, 3, -97, 17, -121, 89, 62, TarConstants.LF_GNUTYPE_LONGNAME, -52, 8, -126, 25, -126, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 62, 64, -61, 24, -107, 4, -126, 93, 124, 3, -64, 25, -124, 2, -124, 82, 62, 96, -61, 24, -107, 4, -126, 93, 124, 111, -41, 24, -124, 25, -123}, new byte[]{45, -94, 108, -16, 118, -21, 60, 16}));
                        vMBigCardView.m4966WWoWWo((MaterialButton) view);
                        return;
                    default:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_GNUTYPE_SPARSE, 124, 65, 114, 96, -124, 100, 79, 80, 73, 105}, new byte[]{62, 42, ConstantPoolEntry.CP_NameAndType, 59, 14, -9, 16, 46});
                            throw null;
                        }
                        return;
                }
            }
        });
        View findViewById5 = findViewById(R.id.volume);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById5, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-20, 18, -86, -107, -17, 86, 126, -90, -56, 2, -115, -107, -111, 17, TarConstants.LF_DIR, -1, -93}, new byte[]{-118, 123, -60, -15, -71, 63, 27, -47}));
        MaterialButton materialButton2 = (MaterialButton) findViewById5;
        this.f8643WWoWWo = materialButton2;
        materialButton2.setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWoϫWWoӉϫ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMBigCardView f29295WWWWWWWWWW;

            {
                this.f29295WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMBigCardView vMBigCardView = this.f29295WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            byte[] bArr = {108, -49, 10, 35, 81, TarConstants.LF_CONTIG, -127, 72};
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{1, -103, 71, 106, 63, 68, -11, 41, 2, -84, 111}, bArr);
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-7, -113, -4, 110, -4, 0, 47, 15, -6, -70, -44}, new byte[]{-108, -39, -79, 39, -110, 115, 91, 110});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                byte[] bArr2 = {116, 10, ConstantPoolEntry.CP_NameAndType, -69, -4, 17, 41, 62};
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{25, 92, 65, -14, -110, 98, 93, 95, 26, 105, 105}, bArr2);
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{67, -41, 0, -100, 86, -120, 93, 126, 67, -51, 24, -48, 20, -114, 28, 115, TarConstants.LF_GNUTYPE_LONGNAME, -47, 24, -48, 2, -124, 28, 126, 66, -52, 65, -98, 3, -121, 80, TarConstants.LF_NORMAL, 89, -37, 28, -107, 86, -120, TarConstants.LF_GNUTYPE_SPARSE, 125, 3, -59, 3, -97, 17, -121, 89, 62, TarConstants.LF_GNUTYPE_LONGNAME, -52, 8, -126, 25, -126, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 62, 64, -61, 24, -107, 4, -126, 93, 124, 3, -64, 25, -124, 2, -124, 82, 62, 96, -61, 24, -107, 4, -126, 93, 124, 111, -41, 24, -124, 25, -123}, new byte[]{45, -94, 108, -16, 118, -21, 60, 16}));
                        vMBigCardView.m4966WWoWWo((MaterialButton) view);
                        return;
                    default:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_GNUTYPE_SPARSE, 124, 65, 114, 96, -124, 100, 79, 80, 73, 105}, new byte[]{62, 42, ConstantPoolEntry.CP_NameAndType, 59, 14, -9, 16, 46});
                            throw null;
                        }
                        return;
                }
            }
        });
        View findViewById6 = findViewById(R.id.settings);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById6, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, TarConstants.LF_BLK, -83, 39, ConstantPoolEntry.CP_NameAndType, 26, -26, -125, -76, 36, -118, 39, 114, 93, -83, -38, -33}, new byte[]{-10, 93, -61, 67, 90, 115, -125, -12}));
        ((MaterialButton) findViewById6).setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWoϫWWoӉϫ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMBigCardView f29295WWWWWWWWWW;

            {
                this.f29295WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMBigCardView vMBigCardView = this.f29295WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            byte[] bArr = {108, -49, 10, 35, 81, TarConstants.LF_CONTIG, -127, 72};
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{1, -103, 71, 106, 63, 68, -11, 41, 2, -84, 111}, bArr);
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-7, -113, -4, 110, -4, 0, 47, 15, -6, -70, -44}, new byte[]{-108, -39, -79, 39, -110, 115, 91, 110});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                byte[] bArr2 = {116, 10, ConstantPoolEntry.CP_NameAndType, -69, -4, 17, 41, 62};
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{25, 92, 65, -14, -110, 98, 93, 95, 26, 105, 105}, bArr2);
                                throw null;
                            }
                        }
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{67, -41, 0, -100, 86, -120, 93, 126, 67, -51, 24, -48, 20, -114, 28, 115, TarConstants.LF_GNUTYPE_LONGNAME, -47, 24, -48, 2, -124, 28, 126, 66, -52, 65, -98, 3, -121, 80, TarConstants.LF_NORMAL, 89, -37, 28, -107, 86, -120, TarConstants.LF_GNUTYPE_SPARSE, 125, 3, -59, 3, -97, 17, -121, 89, 62, TarConstants.LF_GNUTYPE_LONGNAME, -52, 8, -126, 25, -126, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 62, 64, -61, 24, -107, 4, -126, 93, 124, 3, -64, 25, -124, 2, -124, 82, 62, 96, -61, 24, -107, 4, -126, 93, 124, 111, -41, 24, -124, 25, -123}, new byte[]{45, -94, 108, -16, 118, -21, 60, 16}));
                        vMBigCardView.m4966WWoWWo((MaterialButton) view);
                        return;
                    default:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMBigCardView.f8642WWWWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMBigCardView.f8641WWWWWWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_GNUTYPE_SPARSE, 124, 65, 114, 96, -124, 100, 79, 80, 73, 105}, new byte[]{62, 42, ConstantPoolEntry.CP_NameAndType, 59, 14, -9, 16, 46});
                            throw null;
                        }
                        return;
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
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMConfigEvent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{14, -62, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -88, 35}, new byte[]{107, -76, 61, -58, 87, -59, 29, 112}));
        TextView textView = this.f8639WWWWWWWW;
        if (textView != null) {
            VMInstance vMInstance = this.f8641WWWWWWWW;
            if (vMInstance != null) {
                textView.setText(vMInstance.f8937WWWoWWWo.f8861WWWWoWWWWo);
                return;
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-110, -27, -34, 118, -11, Byte.MAX_VALUE, TarConstants.LF_NORMAL, 96, -111, -48, -10}, new byte[]{-1, -77, -109, 63, -101, ConstantPoolEntry.CP_NameAndType, 68, 1}));
                throw null;
            }
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-127, -117, 20, -25, -75, -114, -92, 16, -101}, new byte[]{-20, -59, 117, -118, -48, -40, -51, 117}));
        throw null;
    }

    @InterfaceC2472WWWWWWWW(sticky = AndUn7z.f28850WWWWWWWW, threadMode = ThreadMode.MAIN)
    public final void onVMStatusEvent(VMStatusEvent vMStatusEvent) {
        byte[] bArr = {-104, 97, 104, 18, 122, -46, TarConstants.LF_GNUTYPE_SPARSE, -48};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMStatusEvent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-3, 23, 13, 124, 14}, bArr));
        MaterialButton materialButton = this.f8644WWWW;
        if (materialButton != null) {
            m4967WWWW(materialButton);
            MaterialButton materialButton2 = this.f8643WWoWWo;
            if (materialButton2 != null) {
                m4966WWoWWo(materialButton2);
                return;
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{28, 21, -70, 118, -82, -41, -15, 46, 4, TarConstants.LF_CONTIG, -95, 117, -75}, new byte[]{113, 67, -43, 26, -37, -70, -108, 108}));
                throw null;
            }
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-31, 124, -117, 106, TarConstants.LF_CHR, 19, -40, 21, -8, 91, -112, 101}, new byte[]{-116, 47, -1, ConstantPoolEntry.CP_InterfaceMethodref, 65, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -102, 96}));
        throw null;
    }

    @Override // android.view.View
    public final void onWindowVisibilityChanged(int i10) {
        if (i10 == 0) {
            VMInstance vMInstance = this.f8641WWWWWWWW;
            if (vMInstance != null) {
                C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
                if (!c2467wwwwwwww.m13948WWoWWo(this)) {
                    c2467wwwwwwww.m13950WWWW(this);
                }
            } else {
                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-103, 13, TarConstants.LF_NORMAL, -13, -101, 105, ConstantPoolEntry.CP_InterfaceMethodref, -10, -102, 56, 24}, new byte[]{-12, 91, 125, -70, -11, 26, Byte.MAX_VALUE, -105});
                throw null;
            }
        } else {
            VMInstance vMInstance2 = this.f8641WWWWWWWW;
            if (vMInstance2 != null) {
                C2467WWWWWWWW c2467wwwwwwww2 = vMInstance2.f8939WWWoWWWo;
                if (c2467wwwwwwww2.m13948WWoWWo(this)) {
                    c2467wwwwwwww2.m13945WWWWWWWW(this);
                }
            } else {
                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-83, 121, 34, 116, -83, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_BLK, 56, -82, TarConstants.LF_GNUTYPE_LONGNAME, 10}, new byte[]{-64, 47, 111, 61, -61, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 64, 89});
                throw null;
            }
        }
        super.onWindowVisibilityChanged(i10);
    }

    public final void setVMInstance(VMInstance vMInstance) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-117, -52}, new byte[]{-3, -95, -39, -126, -96, -112, -54, -118}));
        this.f8641WWWWWWWW = vMInstance;
        C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
        if (!c2467wwwwwwww.m13948WWoWWo(this)) {
            c2467wwwwwwww.m13950WWWW(this);
        }
        MaterialButton materialButton = this.f8644WWWW;
        if (materialButton != null) {
            m4967WWWW(materialButton);
            MaterialButton materialButton2 = this.f8643WWoWWo;
            if (materialButton2 != null) {
                m4966WWoWWo(materialButton2);
                TextView textView = this.f8639WWWWWWWW;
                if (textView != null) {
                    textView.setText(vMInstance.f8937WWWoWWWo.f8861WWWWoWWWWo);
                    return;
                } else {
                    AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-51, -101, -106, -64, 95, 70, -7, 115, -41}, new byte[]{-96, -43, -9, -83, 58, 16, -112, 22}));
                    throw null;
                }
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{68, -39, -96, -75, TarConstants.LF_LINK, -16, 71, 58, 92, -5, -69, -74, 42}, new byte[]{41, -113, -49, -39, 68, -99, 34, TarConstants.LF_PAX_EXTENDED_HEADER_LC}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_FIFO, -88, 73, -125, 92, -58, -44, 109, 47, -113, 82, -116}, new byte[]{91, -5, 61, -30, 46, -78, -106, 24}));
        throw null;
    }

    public final void setVMViewActionCallback(InterfaceC3250WWoWWo interfaceC3250WWoWWo) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(interfaceC3250WWoWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{107, 31, -84, -27, 96, -63, 19, -53}, new byte[]{8, 126, -64, -119, 2, -96, 112, -96}));
        this.f8642WWWWWWWW = interfaceC3250WWoWWo;
    }

    /* JADX WARN: 'this' call moved to the top of the method (can break code semantics) */
    public VMBigCardView(Context context, AttributeSet attributeSet) {
        this(context, attributeSet, 4, 0);
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-39, -38, -15, -99, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 95, TarConstants.LF_PAX_EXTENDED_HEADER_UC}, new byte[]{-70, -75, -97, -23, 61, 39, 44, -60}, context);
    }

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public VMBigCardView(Context context, AttributeSet attributeSet, int i10) {
        super(context, attributeSet, i10);
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{124, TarConstants.LF_BLK, -123, -72, -42, -23, 93}, new byte[]{31, 91, -21, -52, -77, -111, 41, -14}, context);
    }

    public /* synthetic */ VMBigCardView(Context context, AttributeSet attributeSet, int i10, int i11) {
        this(context, (i10 & 2) != 0 ? null : attributeSet, 0);
    }
}
