package com.android.vmapp.ui.vm.backup;

import a3.WWWoWWWo;
import ad.WWWW;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;
import androidx.fragment.app.C1013WWWWWWWW;
import com.clone.android.dual.space.R;
import com.google.android.material.progressindicator.LinearProgressIndicator;
import ed.AbstractC2403WWWWoWWWWo;
import h4.C2733WWWoWWWo;
import h4.C2739WWoWWo;
import h4.C2750WWWW;
import ib.WWWWoWWWWo;
import j3.C3164WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p013WWWWWWWW.o;
import v3.WWWWWWWW;
/* loaded from: classes.dex */
public final class VMBackupFragment2 extends WWWWWWWW {

    /* renamed from: WWWWỖWWWWࢥỖ  reason: contains not printable characters */
    public View f8609WWWWWWWW;

    /* renamed from: WWWWίWWWWРί  reason: contains not printable characters */
    public LinearProgressIndicator f8610WWWWWWWW;

    /* renamed from: WWWWℯWWWW၂ℯ  reason: contains not printable characters */
    public TextView f8611WWWWWWWW;

    /* renamed from: a  reason: collision with root package name */
    public final o f36428a = new o(C3333WWWWoWWWWo.m15421WWWWWWWW(C2750WWWW.class), new C2739WWoWWo(this, 0), new WWWW(13, this), new C2739WWoWWo(this, 1));

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWᐡWWWWೱᐡ */
    public final View mo2047WWWWWWWW(LayoutInflater layoutInflater, ViewGroup viewGroup, Bundle bundle) {
        byte[] bArr = {TarConstants.LF_GNUTYPE_LONGLINK, -69, -80, -60, 8, 121, 102, -31};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(layoutInflater, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{34, -43, -42, -88, 105, 13, 3, -109}, bArr));
        View inflate = layoutInflater.inflate(R.layout.fragment_vm_backup2, viewGroup, false);
        View findViewById = inflate.findViewById(R.id.progress_logo);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{119, -63, 80, 6, -84, 80, -20, 102, TarConstants.LF_GNUTYPE_SPARSE, -47, 119, 6, -46, 23, -89, 63, 56}, new byte[]{17, -88, 62, 98, -6, 57, -119, 17}));
        this.f8609WWWWWWWW = findViewById;
        View findViewById2 = inflate.findViewById(R.id.progress_indicator);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-35, -98, 104, 66, -82, -44, -108, 85, -7, -114, 79, 66, -48, -109, -33, ConstantPoolEntry.CP_NameAndType, -110}, new byte[]{-69, -9, 6, 38, -8, -67, -15, 34}));
        this.f8610WWWWWWWW = (LinearProgressIndicator) findViewById2;
        View findViewById3 = inflate.findViewById(R.id.progress_text);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-20, -123, 86, 110, -15, 98, 94, 3, -56, -107, 113, 110, -113, 37, 21, 90, -93}, new byte[]{-118, -20, 56, 10, -89, ConstantPoolEntry.CP_InterfaceMethodref, 59, 116}));
        this.f8611WWWWWWWW = (TextView) findViewById3;
        inflate.findViewById(R.id.backup_cancel).setOnClickListener(new WWWoWWWo(8, this));
        C1013WWWWWWWW m3284WWoWWo = m3284WWoWWo();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{109, -91, 3, 119, -111, 59, 41, -60, 99, -90, 18, 66, -127, 61, TarConstants.LF_SYMLINK, -19, 69, -73, 25, 68, -118, 118, 112, -90, 36, -23}, new byte[]{10, -64, 119, 33, -8, 94, 94, -120});
        AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(WWWWoWWWWo.m14598WWWWWWWW(m3284WWoWWo), null, new C2733WWWoWWWo(this, null), 3);
        return inflate;
    }
}
