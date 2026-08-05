package com.android.vmapp.ui.vm.settings;

import android.os.Bundle;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.C1020WWWoWWWo;
import androidx.fragment.app.WWWWWWWW;
import androidx.preference.o;
import com.android.vmapp.ui.base.preference.BasePreferenceActivity;
import com.clone.android.dual.space.R;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import o4.C3649WWWWWWWW;
/* loaded from: classes.dex */
public final class VMNotificationActivity extends BasePreferenceActivity {

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public C3649WWWWWWWW f8745WWWWWWWW;

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity
    /* renamed from: WWoᆑWWoӁᆑ */
    public final o mo4936WWoWWo() {
        if (this.f8505WWWWWWWW == null) {
            finish();
            return null;
        }
        C3649WWWWWWWW c3649wwwwwwww = new C3649WWWWWWWW();
        C1020WWWoWWWo m3298WWWWWWWW = m3298WWWWWWWW();
        m3298WWWWWWWW.getClass();
        WWWWWWWW wwwwwwww = new WWWWWWWW(m3298WWWWWWWW);
        wwwwwwww.m3423WWoWWo(R.id.settings_container, c3649wwwwwwww, null);
        wwwwwwww.m3390WWWoWWWo();
        this.f8745WWWWWWWW = c3649wwwwwwww;
        return c3649wwwwwwww;
    }

    @Override // androidx.activity.ComponentActivity, android.app.Activity
    public final void onBackPressed() {
        C3649WWWWWWWW c3649wwwwwwww = this.f8745WWWWWWWW;
        if (c3649wwwwwwww != null) {
            AbstractC3339WWWWWWWW.m15436WWWoWWWo(c3649wwwwwwww);
        }
        super.onBackPressed();
    }

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity, com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_vm_notification);
        m2306WWWoWWWo((Toolbar) findViewById(R.id.toolbar));
        androidx.appcompat.app.WWWWWWWW m2307WWoWWo = m2307WWoWWo();
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(m2307WWoWWo);
        m2307WWoWWo.mo2341WoWo(true);
    }
}
