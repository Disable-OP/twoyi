package com.android.vmapp.ui.vm.backup;

import ad.WWWW;
import android.content.res.TypedArray;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.RadioButton;
import android.widget.TextView;
import androidx.fragment.app.C1013WWWWWWWW;
import com.android.vmapp.ui.vm.backup.VMBackupFragment1;
import com.clone.android.dual.space.R;
import com.google.android.material.card.MaterialCardView;
import ed.AbstractC2403WWWWoWWWWo;
import h4.C2738WWoWWo;
import h4.C2750WWWW;
import h4.WoWo;
import hd.C2819WWWWWWWW;
import ib.WWWWoWWWWo;
import j3.C3164WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p013WWWWWWWW.o;
import v3.WWWWWWWW;
/* loaded from: classes.dex */
public final class VMBackupFragment1 extends WWWWWWWW {

    /* renamed from: WWWWỖWWWWࢥỖ  reason: contains not printable characters */
    public View f8606WWWWWWWW;

    /* renamed from: WWWWίWWWWРί  reason: contains not printable characters */
    public View f8607WWWWWWWW;

    /* renamed from: WWWWℯWWWW၂ℯ  reason: contains not printable characters */
    public int f8608WWWWWWWW;

    /* renamed from: a  reason: collision with root package name */
    public int f36426a;

    /* renamed from: b  reason: collision with root package name */
    public final o f36427b = new o(C3333WWWWoWWWWo.m15421WWWWWWWW(C2750WWWW.class), new C2738WWoWWo(this, 0), new WWWW(12, this), new C2738WWoWWo(this, 1));

    /* renamed from: WWWoễWWWoಇễ  reason: contains not printable characters */
    public static final void m4961WWWoWWWo(VMBackupFragment1 vMBackupFragment1, View view) {
        vMBackupFragment1.getClass();
        ((RadioButton) view.findViewById(R.id.enabled_selected)).setChecked(false);
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{85, -43, -49, 22, -64, 121, -83, -95, 85, -49, -41, 90, -126, Byte.MAX_VALUE, -20, -84, 90, -45, -41, 90, -108, 117, -20, -95, 84, -50, -114, 20, -107, 118, -96, -17, 79, -39, -45, 31, -64, 121, -93, -94, 21, -57, -52, 21, -121, 118, -87, -31, 90, -50, -57, 8, -113, 115, -88, -31, 86, -63, -41, 31, -110, 115, -83, -93, 21, -61, -62, 8, -124, TarConstants.LF_BLK, -127, -82, 79, -59, -47, 19, -127, 118, -113, -82, 73, -60, -11, 19, -123, 109}, new byte[]{59, -96, -93, 122, -32, 26, -52, -49});
        MaterialCardView materialCardView = (MaterialCardView) view;
        materialCardView.setChecked(false);
        materialCardView.setStrokeColor(vMBackupFragment1.f36426a);
    }

    /* renamed from: WWWếWWW෨ế  reason: contains not printable characters */
    public static final void m4962WWWWWW(VMBackupFragment1 vMBackupFragment1, View view) {
        vMBackupFragment1.getClass();
        ((RadioButton) view.findViewById(R.id.enabled_selected)).setChecked(true);
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-104, 95, -26, 117, 34, -90, -2, TarConstants.LF_GNUTYPE_LONGNAME, -104, 69, -2, 57, 96, -96, -65, 65, -105, 89, -2, 57, 118, -86, -65, TarConstants.LF_GNUTYPE_LONGNAME, -103, 68, -89, 119, 119, -87, -13, 2, -126, TarConstants.LF_GNUTYPE_SPARSE, -6, 124, 34, -90, -16, 79, -40, TarConstants.LF_MULTIVOLUME, -27, 118, 101, -87, -6, ConstantPoolEntry.CP_NameAndType, -105, 68, -18, 107, 109, -84, -5, ConstantPoolEntry.CP_NameAndType, -101, TarConstants.LF_GNUTYPE_LONGLINK, -2, 124, 112, -84, -2, 78, -40, 73, -21, 107, 102, -21, -46, 67, -126, 79, -8, 112, 99, -87, -36, 67, -124, 78, -36, 112, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -78}, new byte[]{-10, 42, -118, 25, 2, -59, -97, 34});
        MaterialCardView materialCardView = (MaterialCardView) view;
        materialCardView.setChecked(true);
        materialCardView.setStrokeColor(vMBackupFragment1.f8608WWWWWWWW);
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWᐡWWWWೱᐡ */
    public final View mo2047WWWWWWWW(LayoutInflater layoutInflater, ViewGroup viewGroup, Bundle bundle) {
        byte[] bArr = {81, 27, -21, TarConstants.LF_SYMLINK, -100, 8, 107, 23};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(layoutInflater, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{56, 117, -115, 94, -3, 124, 14, 101}, bArr));
        TypedArray obtainStyledAttributes = m3293WWWW().getTheme().obtainStyledAttributes(new int[]{R.attr.colorPrimary, R.attr.colorOnSurfaceVariant});
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(obtainStyledAttributes, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{8, 109, -20, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -28, 13, 15, 27, 30, 99, -3, 93, -52, 23, 40, 29, 14, 109, -19, TarConstants.LF_MULTIVOLUME, -24, 16, 116, 65, 73, 33, -79}, new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 15, -104, 57, -115, 99, 92, 111}));
        this.f8608WWWWWWWW = obtainStyledAttributes.getColor(0, 0);
        this.f36426a = obtainStyledAttributes.getColor(1, 0);
        obtainStyledAttributes.recycle();
        View inflate = layoutInflater.inflate(R.layout.fragment_vm_backup1, viewGroup, false);
        View findViewById = inflate.findViewById(R.id.full_backup);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{46, 80, -73, 94, -97, -94, 38, 40, 10, 64, -112, 94, -31, -27, 109, 113, 97}, new byte[]{72, 57, -39, 58, -55, -53, 67, 95}));
        this.f8606WWWWWWWW = findViewById;
        ((TextView) findViewById.findViewById(R.id.title)).setText(R.string.backup_type_full);
        ((TextView) findViewById.findViewById(R.id.desc)).setText(R.string.backup_type_full_desc);
        View view = this.f8606WWWWWWWW;
        if (view != null) {
            view.setOnClickListener(new View.OnClickListener(this) { // from class: h4.WWWW֭WWWWྥ֭

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMBackupFragment1 f27738WWWWWWWWWW;

                {
                    this.f27738WWWWWWWWWW = this;
                }

                @Override // android.view.View.OnClickListener
                public final void onClick(View view2) {
                    C2819WWWWWWWW c2819wwwwwwww;
                    Object m14479WWWWWWWW;
                    C2819WWWWWWWW c2819wwwwwwww2;
                    Object m14479WWWWWWWW2;
                    switch (r2) {
                        case 0:
                            C2750WWWW c2750wwww = (C2750WWWW) this.f27738WWWWWWWWWW.f36427b.getValue();
                            do {
                                c2819wwwwwwww = c2750wwww.f27869WWoWWo;
                                m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
                            } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, C2752WoWo.m14350WWWWWWWW((C2752WoWo) m14479WWWWWWWW, null, true, null, false, null, null, null, null, null, null, 1021)));
                            return;
                        default:
                            C2750WWWW c2750wwww2 = (C2750WWWW) this.f27738WWWWWWWWWW.f36427b.getValue();
                            do {
                                c2819wwwwwwww2 = c2750wwww2.f27869WWoWWo;
                                m14479WWWWWWWW2 = c2819wwwwwwww2.m14479WWWWWWWW();
                            } while (!c2819wwwwwwww2.m14478WWWWWWWW(m14479WWWWWWWW2, C2752WoWo.m14350WWWWWWWW((C2752WoWo) m14479WWWWWWWW2, null, false, null, false, null, null, null, null, null, null, 1021)));
                            return;
                    }
                }
            });
            View findViewById2 = inflate.findViewById(R.id.data_backup);
            AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{94, -73, 63, -39, -36, -12, 82, -127, 122, -89, 24, -39, -94, -77, 25, -40, 17}, new byte[]{56, -34, 81, -67, -118, -99, TarConstants.LF_CONTIG, -10}));
            this.f8607WWWWWWWW = findViewById2;
            ((TextView) findViewById2.findViewById(R.id.title)).setText(R.string.backup_type_data);
            ((TextView) findViewById2.findViewById(R.id.desc)).setText(R.string.backup_type_data_desc);
            View view2 = this.f8607WWWWWWWW;
            if (view2 != null) {
                view2.setOnClickListener(new View.OnClickListener(this) { // from class: h4.WWWW֭WWWWྥ֭

                    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                    public final /* synthetic */ VMBackupFragment1 f27738WWWWWWWWWW;

                    {
                        this.f27738WWWWWWWWWW = this;
                    }

                    @Override // android.view.View.OnClickListener
                    public final void onClick(View view22) {
                        C2819WWWWWWWW c2819wwwwwwww;
                        Object m14479WWWWWWWW;
                        C2819WWWWWWWW c2819wwwwwwww2;
                        Object m14479WWWWWWWW2;
                        switch (r2) {
                            case 0:
                                C2750WWWW c2750wwww = (C2750WWWW) this.f27738WWWWWWWWWW.f36427b.getValue();
                                do {
                                    c2819wwwwwwww = c2750wwww.f27869WWoWWo;
                                    m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
                                } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, C2752WoWo.m14350WWWWWWWW((C2752WoWo) m14479WWWWWWWW, null, true, null, false, null, null, null, null, null, null, 1021)));
                                return;
                            default:
                                C2750WWWW c2750wwww2 = (C2750WWWW) this.f27738WWWWWWWWWW.f36427b.getValue();
                                do {
                                    c2819wwwwwwww2 = c2750wwww2.f27869WWoWWo;
                                    m14479WWWWWWWW2 = c2819wwwwwwww2.m14479WWWWWWWW();
                                } while (!c2819wwwwwwww2.m14478WWWWWWWW(m14479WWWWWWWW2, C2752WoWo.m14350WWWWWWWW((C2752WoWo) m14479WWWWWWWW2, null, false, null, false, null, null, null, null, null, null, 1021)));
                                return;
                        }
                    }
                });
                C1013WWWWWWWW m3284WWoWWo = m3284WWoWWo();
                x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-117, 124, 22, 3, -2, -11, 44, 13, -123, Byte.MAX_VALUE, 7, TarConstants.LF_FIFO, -18, -13, TarConstants.LF_CONTIG, 36, -93, 110, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_NORMAL, -27, -72, 117, 111, -62, TarConstants.LF_NORMAL}, new byte[]{-20, 25, 98, 85, -105, -112, 91, 65});
                AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(WWWWoWWWWo.m14598WWWWWWWW(m3284WWoWWo), null, new WoWo(this, null), 3);
                return inflate;
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{81, 20, -96, 5, -43, -75, -110, TarConstants.LF_GNUTYPE_LONGLINK, 87, 37, -79, 39, -35, -110, -124}, new byte[]{60, 80, -63, 113, -76, -9, -13, 40}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-90, -31, -77, 4, 106, -37, 40, -2, -96, -46, -74, 62, 111, -4, 62}, new byte[]{-53, -89, -58, 104, 6, -103, 73, -99}));
        throw null;
    }
}
