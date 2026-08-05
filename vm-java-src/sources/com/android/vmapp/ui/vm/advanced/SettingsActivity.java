package com.android.vmapp.ui.vm.advanced;

import android.os.Bundle;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.C1020WWWoWWWo;
import androidx.preference.o;
import com.android.vmapp.ui.base.preference.BasePreferenceActivity;
import com.clone.android.dual.space.R;
import g4.WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
/* loaded from: classes.dex */
public final class SettingsActivity extends BasePreferenceActivity {
    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity
    /* renamed from: WWoᆑWWoӁᆑ */
    public final o mo4936WWoWWo() {
        WWWWWWWW wwwwwwww = new WWWWWWWW();
        C1020WWWoWWWo m3298WWWWWWWW = m3298WWWWWWWW();
        m3298WWWWWWWW.getClass();
        androidx.fragment.app.WWWWWWWW wwwwwwww2 = new androidx.fragment.app.WWWWWWWW(m3298WWWWWWWW);
        wwwwwwww2.m3423WWoWWo(R.id.settings_container, wwwwwwww, null);
        wwwwwwww2.m3390WWWoWWWo();
        return wwwwwwww;
    }

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity, com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_advanced_options);
        m2306WWWoWWWo((Toolbar) findViewById(R.id.toolbar));
        androidx.appcompat.app.WWWWWWWW m2307WWoWWo = m2307WWoWWo();
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(m2307WWoWWo);
        m2307WWoWWo.mo2341WoWo(true);
    }
}
