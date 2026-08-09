package com.android.vmapp.ui.vm.settings;

import android.os.Bundle;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.C1020WWWoWWWo;
import androidx.fragment.app.WWWWWWWW;
import androidx.preference.o;
import com.android.vmapp.ui.base.preference.BasePreferenceActivity;
import com.clone.android.dual.space.R;
import o4.C3662WWWWWWWW;
/* loaded from: classes.dex */
public class VMSettingsActivity extends BasePreferenceActivity {
    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity
    /* renamed from: WWoᆑWWoӁᆑ */
    public final o mo4936WWoWWo() {
        if (this.f8505WWWWWWWW == null) {
            finish();
            return null;
        }
        C3662WWWWWWWW c3662wwwwwwww = new C3662WWWWWWWW();
        C1020WWWoWWWo m3298WWWWWWWW = m3298WWWWWWWW();
        m3298WWWWWWWW.getClass();
        WWWWWWWW wwwwwwww = new WWWWWWWW(m3298WWWWWWWW);
        wwwwwwww.m3423WWoWWo(R.id.settings_container, c3662wwwwwwww, null);
        wwwwwwww.m3390WWWoWWWo();
        return c3662wwwwwwww;
    }

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity, com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_vm_settings);
        m2306WWWoWWWo((Toolbar) findViewById(R.id.toolbar));
        m2307WWoWWo().mo2341WoWo(true);
    }
}
