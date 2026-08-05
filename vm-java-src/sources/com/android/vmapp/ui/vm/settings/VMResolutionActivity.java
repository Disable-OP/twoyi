package com.android.vmapp.ui.vm.settings;

import android.os.Bundle;
import androidx.appcompat.widget.Toolbar;
import com.android.vmapp.ui.base.BaseActivity;
import com.android.vmapp.ui.vm.resolution.ResolutionFragment;
import com.clone.android.dual.space.R;
import i6.C2899WWWWWWWW;
import j3.C3164WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class VMResolutionActivity extends BaseActivity {

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public static final /* synthetic */ int f8748WWWWWWWW = 0;

    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_vm_resolution);
        if (this.f8505WWWWWWWW == null) {
            finish();
            return;
        }
        m2306WWWoWWWo((Toolbar) findViewById(R.id.toolbar));
        m2307WWoWWo().mo2341WoWo(true);
        ResolutionFragment resolutionFragment = (ResolutionFragment) m3298WWWWWWWW().m3350WWoWWo(R.id.fragment_container_view);
        Bundle bundle2 = new Bundle();
        byte[] bArr = {31, 125, -39, -70, 34, -110, -35, TarConstants.LF_GNUTYPE_LONGLINK};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        bundle2.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{124, 8, -85, -56, 71, -4, -87, 20, 109, 24, -86, -43, 78, -25, -87, 34, 112, 19}, bArr), this.f8505WWWWWWWW.f8937WWWoWWWo.f8900WWWoWWWo.f8953WWWWWWWW);
        resolutionFragment.m3278WWWWWWWW(bundle2);
        resolutionFragment.f36454b = new C2899WWWWWWWW(10, this);
    }
}
