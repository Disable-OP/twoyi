package com.android.vmapp.ui.vm.settings;

import android.content.DialogInterface;
import android.os.Bundle;
import androidx.appcompat.app.WWWW;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.C1020WWWoWWWo;
import androidx.fragment.app.WWWWWWWW;
import androidx.preference.o;
import com.android.vmapp.ui.base.preference.BasePreferenceActivity;
import com.android.vmapp.ui.vm.settings.VMSecurityActivity;
import com.android.vmcore.VMInstance;
import com.clone.android.dual.space.R;
import da.WWWWoWWWWo;
import e4.C2344WWWWWWWW;
import j3.C3164WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import o4.C3675WWoWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
/* loaded from: classes.dex */
public final class VMSecurityActivity extends BasePreferenceActivity {

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public static final /* synthetic */ int f8749WWWW = 0;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public C3675WWoWWo f8750WWWWWWWW;

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity
    /* renamed from: WWoᆑWWoӁᆑ */
    public final o mo4936WWoWWo() {
        if (this.f8505WWWWWWWW == null) {
            finish();
            return null;
        }
        C3675WWoWWo c3675WWoWWo = new C3675WWoWWo();
        C1020WWWoWWWo m3298WWWWWWWW = m3298WWWWWWWW();
        m3298WWWWWWWW.getClass();
        WWWWWWWW wwwwwwww = new WWWWWWWW(m3298WWWWWWWW);
        wwwwwwww.m3423WWoWWo(R.id.settings_container, c3675WWoWWo, null);
        wwwwwwww.m3390WWWoWWWo();
        this.f8750WWWWWWWW = c3675WWoWWo;
        return c3675WWoWWo;
    }

    @Override // androidx.activity.ComponentActivity, android.app.Activity
    public final void onBackPressed() {
        C3675WWoWWo c3675WWoWWo = this.f8750WWWWWWWW;
        if (c3675WWoWWo != null && c3675WWoWWo.f38343f && this.f8505WWWWWWWW.f8940WWoWWo >= 4) {
            WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
            wWWWoWWWWo.m13648WoWo(R.string.dialog_title_restart);
            wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_restart);
            wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new DialogInterface.OnClickListener(this) { // from class: o4.WWWWᏊWWWWటᏊ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMSecurityActivity f31689WWWWWWWWWW;

                {
                    this.f31689WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMSecurityActivity vMSecurityActivity = this.f31689WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMSecurityActivity.f8749WWWW;
                            VMInstance vMInstance = vMSecurityActivity.f8505WWWWWWWW;
                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                            AbstractC3339WWWWWWWW.m15429WWWWWWWW(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-123, -43, 107, -119, -120, -107, 93, -38, -52, -103}, new byte[]{-30, -80, 31, -33, -59, -67, 115, -12}));
                            C2344WWWWWWWW.m13706WWWoWWWo(vMSecurityActivity, vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_FIFO, 126, 92, -7, 60, 30, 43}, new byte[]{68, 27, 47, -115, 93, 108, 95, 111}));
                            return;
                        default:
                            int i12 = VMSecurityActivity.f8749WWWW;
                            vMSecurityActivity.finish();
                            return;
                    }
                }
            });
            wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, new DialogInterface.OnClickListener(this) { // from class: o4.WWWWᏊWWWWటᏊ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMSecurityActivity f31689WWWWWWWWWW;

                {
                    this.f31689WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMSecurityActivity vMSecurityActivity = this.f31689WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMSecurityActivity.f8749WWWW;
                            VMInstance vMInstance = vMSecurityActivity.f8505WWWWWWWW;
                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                            AbstractC3339WWWWWWWW.m15429WWWWWWWW(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-123, -43, 107, -119, -120, -107, 93, -38, -52, -103}, new byte[]{-30, -80, 31, -33, -59, -67, 115, -12}));
                            C2344WWWWWWWW.m13706WWWoWWWo(vMSecurityActivity, vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_FIFO, 126, 92, -7, 60, 30, 43}, new byte[]{68, 27, 47, -115, 93, 108, 95, 111}));
                            return;
                        default:
                            int i12 = VMSecurityActivity.f8749WWWW;
                            vMSecurityActivity.finish();
                            return;
                    }
                }
            });
            WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-80, -94, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -50, 13, -3, -2, -80, -3, -2, 20}, new byte[]{-45, -48, 61, -81, 121, -104, -42, -98});
            mo742WWWW.show();
            return;
        }
        super.onBackPressed();
    }

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity, com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_vm_security);
        m2306WWWoWWWo((Toolbar) findViewById(R.id.toolbar));
        androidx.appcompat.app.WWWWWWWW m2307WWoWWo = m2307WWoWWo();
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(m2307WWoWWo);
        m2307WWoWWo.mo2341WoWo(true);
    }
}
