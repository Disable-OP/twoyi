package com.android.vmapp.ui.vm.main;

import android.content.Context;
import android.util.AttributeSet;
import android.view.View;
import android.widget.RelativeLayout;
import android.widget.TextView;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmapp.ui.vm.main.VMListItemView;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.VMConfigEvent;
import com.android.vmcore.event.VMStatusEvent;
import com.clone.android.dual.space.R;
import com.google.android.material.button.MaterialButton;
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
public final class VMListItemView extends RelativeLayout {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public MaterialButton f8658WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public TextView f8659WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public MaterialButton f8660WWWWWWWW;

    /* renamed from: WWWWཀྵWWWWࠤཀྵ  reason: contains not printable characters */
    public InterfaceC3250WWoWWo f8661WWWWWWWW;

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public VMInstance f8662WWWW;

    /* JADX WARN: 'this' call moved to the top of the method (can break code semantics) */
    public VMListItemView(Context context) {
        this(context, null, 6, 0);
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{21, -51, -64, -20, 98, 5, -37}, new byte[]{118, -94, -82, -104, 7, 125, -81, 95}, context);
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final void m4986WWWWoWWWWo(MaterialButton materialButton) {
        VMInstance vMInstance = this.f8662WWWW;
        if (vMInstance != null) {
            if (vMInstance.m5062WWWWWWWW()) {
                materialButton.setIconResource(R.drawable.outline_volume_off_24);
                return;
            } else {
                materialButton.setIconResource(R.drawable.outline_volume_up_24);
                return;
            }
        }
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-115, -82, -112, -43, 13, 15, 32, -118, -114, -101, -72}, new byte[]{-32, -8, -35, -100, 99, 124, 84, -21});
        throw null;
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final void m4987WWWWWWWW(MaterialButton materialButton) {
        VMInstance vMInstance = this.f8662WWWW;
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
        byte[] bArr = {ConstantPoolEntry.CP_NameAndType, -54, -120, -12, 126, -123, 123, -39};
        WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{97, -100, -59, -67, 16, -10, 15, -72, 98, -87, -19}, bArr);
        throw null;
    }

    @Override // android.view.View
    public final void onFinishInflate() {
        super.onFinishInflate();
        View findViewById = findViewById(R.id.name);
        AbstractC1017WWWoWWWo.m3443WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-40, ConstantPoolEntry.CP_InterfaceMethodref, 72, -27, 57, 87, -45, -91, -4, 27, 111, -27, 71, 16, -104, -4, -105}, new byte[]{-66, 98, 38, -127, 111, 62, -74, -46}, findViewById);
        this.f8659WWWWoWWWWo = (TextView) findViewById;
        View findViewById2 = findViewById(R.id.start);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{25, -12, 113, 17, 58, -104, -34, -20, 61, -28, 86, 17, 68, -33, -107, -75, 86}, new byte[]{Byte.MAX_VALUE, -99, 31, 117, 108, -15, -69, -101}));
        MaterialButton materialButton = (MaterialButton) findViewById2;
        this.f8658WWWWWWWWWW = materialButton;
        materialButton.setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWWWެWWWWܕެ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMListItemView f29277WWWWWWWWWW;

            {
                this.f29277WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMListItemView vMListItemView = this.f29277WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMListItemView.f8662WWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            byte[] bArr = {-91, -105, -92, 113, TarConstants.LF_MULTIVOLUME, -92, -80, -96};
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-56, -63, -23, 56, 35, -41, -60, -63, -53, -12, -63}, bArr);
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMListItemView.f8662WWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-70, 34, -9, -114, -87, -121, 42, TarConstants.LF_GNUTYPE_SPARSE, -71, 23, -33}, new byte[]{-41, 116, -70, -57, -57, -12, 94, TarConstants.LF_SYMLINK});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMListItemView.f8662WWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-40, -25, 96, -11, -109, Byte.MIN_VALUE, -40, 107, -37, -46, 72}, new byte[]{-75, -79, 45, -68, -3, -13, -84, 10});
                                throw null;
                            }
                        }
                        byte[] bArr2 = {36, -57, TarConstants.LF_BLK, -50, -118, -78, 98, 117, 36, -35, 44, -126, -56, -76, 35, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 43, -63, 44, -126, -34, -66, 35, 117, 37, -36, 117, -52, -33, -67, 111, 59, 62, -53, 40, -57, -118, -78, 108, 118, 100, -43, TarConstants.LF_CONTIG, -51, -51, -67, 102, TarConstants.LF_DIR, 43, -36, 60, -48, -59, -72, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_DIR, 39, -45, 44, -57, -40, -72, 98, 119, 100, -48, 45, -42, -34, -66, 109, TarConstants.LF_DIR, 7, -45, 44, -57, -40, -72, 98, 119, 8, -57, 44, -42, -59, -65};
                        byte[] bArr3 = {74, -78, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -94, -86, -47, 3, 27};
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                        vMListItemView.m4986WWWWoWWWWo((MaterialButton) view);
                        return;
                    default:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMListItemView.f8662WWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_MULTIVOLUME, 40, -78, 7, -39, -82, -59, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 78, 29, -102}, new byte[]{32, 126, -1, 78, -73, -35, -79, 57});
                            throw null;
                        }
                        return;
                }
            }
        });
        View findViewById3 = findViewById(R.id.shutdown);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-116, -28, 110, 107, 22, 69, 24, -116, -88, -12, 73, 107, 104, 2, TarConstants.LF_GNUTYPE_SPARSE, -43, -61}, new byte[]{-22, -115, 0, 15, 64, 44, 125, -5}));
        ((MaterialButton) findViewById3).setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWWWެWWWWܕެ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMListItemView f29277WWWWWWWWWW;

            {
                this.f29277WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMListItemView vMListItemView = this.f29277WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMListItemView.f8662WWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            byte[] bArr = {-91, -105, -92, 113, TarConstants.LF_MULTIVOLUME, -92, -80, -96};
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-56, -63, -23, 56, 35, -41, -60, -63, -53, -12, -63}, bArr);
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMListItemView.f8662WWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-70, 34, -9, -114, -87, -121, 42, TarConstants.LF_GNUTYPE_SPARSE, -71, 23, -33}, new byte[]{-41, 116, -70, -57, -57, -12, 94, TarConstants.LF_SYMLINK});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMListItemView.f8662WWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-40, -25, 96, -11, -109, Byte.MIN_VALUE, -40, 107, -37, -46, 72}, new byte[]{-75, -79, 45, -68, -3, -13, -84, 10});
                                throw null;
                            }
                        }
                        byte[] bArr2 = {36, -57, TarConstants.LF_BLK, -50, -118, -78, 98, 117, 36, -35, 44, -126, -56, -76, 35, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 43, -63, 44, -126, -34, -66, 35, 117, 37, -36, 117, -52, -33, -67, 111, 59, 62, -53, 40, -57, -118, -78, 108, 118, 100, -43, TarConstants.LF_CONTIG, -51, -51, -67, 102, TarConstants.LF_DIR, 43, -36, 60, -48, -59, -72, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_DIR, 39, -45, 44, -57, -40, -72, 98, 119, 100, -48, 45, -42, -34, -66, 109, TarConstants.LF_DIR, 7, -45, 44, -57, -40, -72, 98, 119, 8, -57, 44, -42, -59, -65};
                        byte[] bArr3 = {74, -78, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -94, -86, -47, 3, 27};
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                        vMListItemView.m4986WWWWoWWWWo((MaterialButton) view);
                        return;
                    default:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMListItemView.f8662WWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_MULTIVOLUME, 40, -78, 7, -39, -82, -59, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 78, 29, -102}, new byte[]{32, 126, -1, 78, -73, -35, -79, 57});
                            throw null;
                        }
                        return;
                }
            }
        });
        View findViewById4 = findViewById(R.id.volume);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById4, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-58, -14, 14, -4, -1, 79, -77, 0, -30, -30, 41, -4, -127, 8, -8, 89, -119}, new byte[]{-96, -101, 96, -104, -87, 38, -42, 119}));
        MaterialButton materialButton2 = (MaterialButton) findViewById4;
        this.f8660WWWWWWWW = materialButton2;
        materialButton2.setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWWWެWWWWܕެ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMListItemView f29277WWWWWWWWWW;

            {
                this.f29277WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMListItemView vMListItemView = this.f29277WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMListItemView.f8662WWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            byte[] bArr = {-91, -105, -92, 113, TarConstants.LF_MULTIVOLUME, -92, -80, -96};
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-56, -63, -23, 56, 35, -41, -60, -63, -53, -12, -63}, bArr);
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMListItemView.f8662WWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-70, 34, -9, -114, -87, -121, 42, TarConstants.LF_GNUTYPE_SPARSE, -71, 23, -33}, new byte[]{-41, 116, -70, -57, -57, -12, 94, TarConstants.LF_SYMLINK});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMListItemView.f8662WWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-40, -25, 96, -11, -109, Byte.MIN_VALUE, -40, 107, -37, -46, 72}, new byte[]{-75, -79, 45, -68, -3, -13, -84, 10});
                                throw null;
                            }
                        }
                        byte[] bArr2 = {36, -57, TarConstants.LF_BLK, -50, -118, -78, 98, 117, 36, -35, 44, -126, -56, -76, 35, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 43, -63, 44, -126, -34, -66, 35, 117, 37, -36, 117, -52, -33, -67, 111, 59, 62, -53, 40, -57, -118, -78, 108, 118, 100, -43, TarConstants.LF_CONTIG, -51, -51, -67, 102, TarConstants.LF_DIR, 43, -36, 60, -48, -59, -72, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_DIR, 39, -45, 44, -57, -40, -72, 98, 119, 100, -48, 45, -42, -34, -66, 109, TarConstants.LF_DIR, 7, -45, 44, -57, -40, -72, 98, 119, 8, -57, 44, -42, -59, -65};
                        byte[] bArr3 = {74, -78, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -94, -86, -47, 3, 27};
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                        vMListItemView.m4986WWWWoWWWWo((MaterialButton) view);
                        return;
                    default:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMListItemView.f8662WWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_MULTIVOLUME, 40, -78, 7, -39, -82, -59, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 78, 29, -102}, new byte[]{32, 126, -1, 78, -73, -35, -79, 57});
                            throw null;
                        }
                        return;
                }
            }
        });
        View findViewById5 = findViewById(R.id.settings);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById5, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -82, -117, 8, 99, 20, -124, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -76, -66, -84, 8, 29, TarConstants.LF_GNUTYPE_SPARSE, -49, 62, -33}, new byte[]{-10, -57, -27, 108, TarConstants.LF_DIR, 125, -31, 16}));
        ((MaterialButton) findViewById5).setOnClickListener(new View.OnClickListener(this) { // from class: k4.WWWWެWWWWܕެ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMListItemView f29277WWWWWWWWWW;

            {
                this.f29277WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMListItemView vMListItemView = this.f29277WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance = vMListItemView.f8662WWWW;
                            if (vMInstance != null) {
                                interfaceC3250WWoWWo.mo4974WWWWoWWWWo(view, vMInstance);
                                return;
                            }
                            byte[] bArr = {-91, -105, -92, 113, TarConstants.LF_MULTIVOLUME, -92, -80, -96};
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-56, -63, -23, 56, 35, -41, -60, -63, -53, -12, -63}, bArr);
                            throw null;
                        }
                        return;
                    case 1:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo2 = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo2 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance2 = vMListItemView.f8662WWWW;
                            if (vMInstance2 != null) {
                                interfaceC3250WWoWWo2.mo4975WWWWWWWW(view, vMInstance2);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-70, 34, -9, -114, -87, -121, 42, TarConstants.LF_GNUTYPE_SPARSE, -71, 23, -33}, new byte[]{-41, 116, -70, -57, -57, -12, 94, TarConstants.LF_SYMLINK});
                            throw null;
                        }
                        return;
                    case 2:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo3 = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo3 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance3 = vMListItemView.f8662WWWW;
                            if (vMInstance3 != null) {
                                interfaceC3250WWoWWo3.mo4977WWWWWWWW(view, vMInstance3);
                            } else {
                                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-40, -25, 96, -11, -109, Byte.MIN_VALUE, -40, 107, -37, -46, 72}, new byte[]{-75, -79, 45, -68, -3, -13, -84, 10});
                                throw null;
                            }
                        }
                        byte[] bArr2 = {36, -57, TarConstants.LF_BLK, -50, -118, -78, 98, 117, 36, -35, 44, -126, -56, -76, 35, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 43, -63, 44, -126, -34, -66, 35, 117, 37, -36, 117, -52, -33, -67, 111, 59, 62, -53, 40, -57, -118, -78, 108, 118, 100, -43, TarConstants.LF_CONTIG, -51, -51, -67, 102, TarConstants.LF_DIR, 43, -36, 60, -48, -59, -72, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_DIR, 39, -45, 44, -57, -40, -72, 98, 119, 100, -48, 45, -42, -34, -66, 109, TarConstants.LF_DIR, 7, -45, 44, -57, -40, -72, 98, 119, 8, -57, 44, -42, -59, -65};
                        byte[] bArr3 = {74, -78, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -94, -86, -47, 3, 27};
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(view, x5.WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                        vMListItemView.m4986WWWWoWWWWo((MaterialButton) view);
                        return;
                    default:
                        InterfaceC3250WWoWWo interfaceC3250WWoWWo4 = vMListItemView.f8661WWWWWWWW;
                        if (interfaceC3250WWoWWo4 != null) {
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(view);
                            VMInstance vMInstance4 = vMListItemView.f8662WWWW;
                            if (vMInstance4 != null) {
                                interfaceC3250WWoWWo4.mo4980WWWoWWWo(view, vMInstance4);
                                return;
                            }
                            WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_MULTIVOLUME, 40, -78, 7, -39, -82, -59, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 78, 29, -102}, new byte[]{32, 126, -1, 78, -73, -35, -79, 57});
                            throw null;
                        }
                        return;
                }
            }
        });
    }

    @InterfaceC2472WWWWWWWW(sticky = AndUn7z.f28850WWWWWWWW, threadMode = ThreadMode.MAIN)
    public final void onVMConfigEvent(VMConfigEvent vMConfigEvent) {
        byte[] bArr = {-39, 45, 23, TarConstants.LF_FIFO, 73};
        byte[] bArr2 = {-68, 91, 114, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 61, -99, 66, -36};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMConfigEvent, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        TextView textView = this.f8659WWWWoWWWWo;
        if (textView != null) {
            VMInstance vMInstance = this.f8662WWWW;
            if (vMInstance != null) {
                textView.setText(vMInstance.f8937WWWoWWWo.f8861WWWWoWWWWo);
                return;
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-69, 68, -76, -5, -86, 99, -13, -94, -72, 113, -100}, new byte[]{-42, 18, -7, -78, -60, 16, -121, -61}));
                throw null;
            }
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-111, 99, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -27, 74, 125, 15, 36, -117}, new byte[]{-4, 45, 6, -120, 47, 43, 102, 65}));
        throw null;
    }

    @InterfaceC2472WWWWWWWW(sticky = AndUn7z.f28850WWWWWWWW, threadMode = ThreadMode.MAIN)
    public final void onVMStatusEvent(VMStatusEvent vMStatusEvent) {
        byte[] bArr = {TarConstants.LF_GNUTYPE_LONGNAME, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -87, -115, -109, -60, -74, -60};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMStatusEvent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{41, 46, -52, -29, -25}, bArr));
        MaterialButton materialButton = this.f8658WWWWWWWWWW;
        if (materialButton != null) {
            m4987WWWWWWWW(materialButton);
            MaterialButton materialButton2 = this.f8660WWWWWWWW;
            if (materialButton2 != null) {
                m4986WWWWoWWWWo(materialButton2);
                return;
            } else {
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-40, 107, 25, 61, 80, 107, -90, -74, -64, 73, 2, 62, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{-75, 61, 118, 81, 37, 6, -61, -12}));
                throw null;
            }
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-103, -110, -46, 15, 65, 31, 118, 108, Byte.MIN_VALUE, -75, -55, 0}, new byte[]{-12, -63, -90, 110, TarConstants.LF_CHR, 107, TarConstants.LF_BLK, 25}));
        throw null;
    }

    @Override // android.view.View
    public final void onWindowVisibilityChanged(int i10) {
        if (i10 == 0) {
            VMInstance vMInstance = this.f8662WWWW;
            if (vMInstance != null) {
                C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
                if (!c2467wwwwwwww.m13948WWoWWo(this)) {
                    c2467wwwwwwww.m13950WWWW(this);
                }
            } else {
                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-99, -21, -121, -1, 91, 39, 62, TarConstants.LF_GNUTYPE_LONGNAME, -98, -34, -81}, new byte[]{-16, -67, -54, -74, TarConstants.LF_DIR, 84, 74, 45});
                throw null;
            }
        } else {
            VMInstance vMInstance2 = this.f8662WWWW;
            if (vMInstance2 != null) {
                C2467WWWWWWWW c2467wwwwwwww2 = vMInstance2.f8939WWWoWWWo;
                if (c2467wwwwwwww2.m13948WWoWWo(this)) {
                    c2467wwwwwwww2.m13945WWWWWWWW(this);
                }
            } else {
                WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{13, -20, -52, -58, -92, -30, 1, -59, 14, -39, -28}, new byte[]{96, -70, -127, -113, -54, -111, 117, -92});
                throw null;
            }
        }
        super.onWindowVisibilityChanged(i10);
    }

    public final void setVMInstance(VMInstance vMInstance) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{99, 28}, new byte[]{21, 113, 78, 58, -70, 123, 5, 111}));
        this.f8662WWWW = vMInstance;
        C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
        if (!c2467wwwwwwww.m13948WWoWWo(this)) {
            c2467wwwwwwww.m13950WWWW(this);
        }
        MaterialButton materialButton = this.f8658WWWWWWWWWW;
        if (materialButton != null) {
            m4987WWWWWWWW(materialButton);
            MaterialButton materialButton2 = this.f8660WWWWWWWW;
            if (materialButton2 != null) {
                m4986WWWWoWWWWo(materialButton2);
                TextView textView = this.f8659WWWWoWWWWo;
                if (textView != null) {
                    textView.setText(vMInstance.f8937WWWoWWWo.f8861WWWWoWWWWo);
                    return;
                } else {
                    AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-42, -93, 84, TarConstants.LF_NORMAL, -44, TarConstants.LF_DIR, TarConstants.LF_CONTIG, -15, -52}, new byte[]{-69, -19, TarConstants.LF_DIR, 93, -79, 99, 94, -108}));
                    throw null;
                }
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-59, -1, 123, TarConstants.LF_FIFO, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -66, 19, TarConstants.LF_MULTIVOLUME, -35, -35, 96, TarConstants.LF_DIR, 67}, new byte[]{-88, -87, 20, 90, 45, -45, 118, 15}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, -101, -127, -78, -101, TarConstants.LF_BLK, -93, TarConstants.LF_GNUTYPE_LONGLINK, -84, -68, -102, -67}, new byte[]{-40, -56, -11, -45, -23, 64, -31, 62}));
        throw null;
    }

    public final void setVMViewActionCallback(InterfaceC3250WWoWWo interfaceC3250WWoWWo) {
        byte[] bArr = {122, 26, -101, -90, 121, TarConstants.LF_GNUTYPE_LONGNAME, -57, 3};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(interfaceC3250WWoWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{25, 123, -9, -54, 27, 45, -92, 104}, bArr));
        this.f8661WWWWWWWW = interfaceC3250WWoWWo;
    }

    /* JADX WARN: 'this' call moved to the top of the method (can break code semantics) */
    public VMListItemView(Context context, AttributeSet attributeSet) {
        this(context, attributeSet, 4, 0);
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{78, -64, 89, -52, 0, -19, TarConstants.LF_LINK}, new byte[]{45, -81, TarConstants.LF_CONTIG, -72, 101, -107, 69, -6}, context);
    }

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public VMListItemView(Context context, AttributeSet attributeSet, int i10) {
        super(context, attributeSet, i10);
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-79, -57, TarConstants.LF_GNUTYPE_LONGLINK, TarConstants.LF_GNUTYPE_LONGNAME, 68, 65, 31}, new byte[]{-46, -88, 37, 56, 33, 57, 107, 92}, context);
    }

    public /* synthetic */ VMListItemView(Context context, AttributeSet attributeSet, int i10, int i11) {
        this(context, (i10 & 2) != 0 ? null : attributeSet, 0);
    }
}
