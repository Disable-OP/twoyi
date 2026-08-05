package com.android.vmapp.ui.vm.backup;

import a3.WWWoWWWo;
import ad.WWWW;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.ImageView;
import android.widget.TextView;
import androidx.fragment.app.C1013WWWWWWWW;
import com.clone.android.dual.space.R;
import com.google.android.material.button.MaterialButton;
import ed.AbstractC2403WWWWoWWWWo;
import h4.C2734WWWoWWWo;
import h4.C2750WWWW;
import ib.WWWWoWWWWo;
import j3.C3164WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import p013WWWWWWWW.o;
import v3.WWWWWWWW;
/* loaded from: classes.dex */
public final class VMBackupFragment3 extends WWWWWWWW {

    /* renamed from: WWWWỖWWWWࢥỖ  reason: contains not printable characters */
    public ImageView f8612WWWWWWWW;

    /* renamed from: WWWWίWWWWРί  reason: contains not printable characters */
    public TextView f8613WWWWWWWW;

    /* renamed from: WWWWℯWWWW၂ℯ  reason: contains not printable characters */
    public TextView f8614WWWWWWWW;

    /* renamed from: a  reason: collision with root package name */
    public MaterialButton f36429a;

    /* renamed from: b  reason: collision with root package name */
    public final o f36430b = new o(C3333WWWWoWWWWo.m15421WWWWWWWW(C2750WWWW.class), new h4.o(this, 0), new WWWW(14, this), new h4.o(this, 1));

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWᐡWWWWೱᐡ */
    public final View mo2047WWWWWWWW(LayoutInflater layoutInflater, ViewGroup viewGroup, Bundle bundle) {
        byte[] bArr = {99, -9, -55, TarConstants.LF_GNUTYPE_SPARSE, 93, -111, -71, -27};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(layoutInflater, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{10, -103, -81, 63, 60, -27, -36, -105}, bArr));
        View inflate = layoutInflater.inflate(R.layout.fragment_vm_backup3, viewGroup, false);
        View findViewById = inflate.findViewById(R.id.backup_status_logo);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{33, TarConstants.LF_LINK, -55, -67, 24, -8, -7, -113, 5, 33, -18, -67, 102, -65, -78, -42, 110}, new byte[]{71, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -89, -39, 78, -111, -100, -8}));
        this.f8612WWWWWWWW = (ImageView) findViewById;
        View findViewById2 = inflate.findViewById(R.id.backup_name);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-8, -33, Byte.MIN_VALUE, 112, -53, 3, -101, 34, -36, -49, -89, 112, -75, 68, -48, 123, -73}, new byte[]{-98, -74, -18, 20, -99, 106, -2, 85}));
        this.f8613WWWWWWWW = (TextView) findViewById2;
        View findViewById3 = inflate.findViewById(R.id.backup_status_text);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-91, -72, -64, 29, TarConstants.LF_GNUTYPE_LONGNAME, 95, 30, 5, -127, -88, -25, 29, TarConstants.LF_SYMLINK, 24, 85, 92, -22}, new byte[]{-61, -47, -82, 121, 26, TarConstants.LF_FIFO, 123, 114}));
        this.f8614WWWWWWWW = (TextView) findViewById3;
        View findViewById4 = inflate.findViewById(R.id.backup_error_link);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById4, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{24, 24, -10, Byte.MIN_VALUE, 108, -87, -101, -5, 60, 8, -47, Byte.MIN_VALUE, 18, -18, -48, -94, 87}, new byte[]{126, 113, -104, -28, 58, -64, -2, -116}));
        MaterialButton materialButton = (MaterialButton) findViewById4;
        this.f36429a = materialButton;
        materialButton.setOnClickListener(new WWWoWWWo(9, this));
        C1013WWWWWWWW m3284WWoWWo = m3284WWoWWo();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-54, -124, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -96, 99, -47, -76, 43, -60, -121, 73, -107, 115, -41, -81, 2, -30, -106, 66, -109, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -100, -19, 73, -125, -56}, new byte[]{-83, -31, 44, -10, 10, -76, -61, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER});
        AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(WWWWoWWWWo.m14598WWWWWWWW(m3284WWoWWo), null, new C2734WWWoWWWo(this, null), 3);
        return inflate;
    }
}
