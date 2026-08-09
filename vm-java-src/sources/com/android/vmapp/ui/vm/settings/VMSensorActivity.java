package com.android.vmapp.ui.vm.settings;

import android.content.DialogInterface;
import android.os.Bundle;
import androidx.appcompat.app.WWWW;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.C1020WWWoWWWo;
import androidx.fragment.app.WWWWWWWW;
import androidx.preference.o;
import com.android.vmapp.ui.base.preference.BasePreferenceActivity;
import com.android.vmapp.ui.vm.settings.VMSensorActivity;
import com.android.vmcore.VMInstance;
import com.clone.android.dual.space.R;
import da.WWWWoWWWWo;
import e4.C2344WWWWWWWW;
import j3.C3164WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import o4.C3667WWWoWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
/* loaded from: classes.dex */
public final class VMSensorActivity extends BasePreferenceActivity {

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public static final /* synthetic */ int f8751WWWW = 0;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public C3667WWWoWWWo f8752WWWWWWWW;

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity
    /* renamed from: WWoᆑWWoӁᆑ */
    public final o mo4936WWoWWo() {
        if (this.f8505WWWWWWWW == null) {
            finish();
            return null;
        }
        C3667WWWoWWWo c3667WWWoWWWo = new C3667WWWoWWWo();
        C1020WWWoWWWo m3298WWWWWWWW = m3298WWWWWWWW();
        m3298WWWWWWWW.getClass();
        WWWWWWWW wwwwwwww = new WWWWWWWW(m3298WWWWWWWW);
        wwwwwwww.m3423WWoWWo(R.id.settings_container, c3667WWWoWWWo, null);
        wwwwwwww.m3390WWWoWWWo();
        this.f8752WWWWWWWW = c3667WWWoWWWo;
        return c3667WWWoWWWo;
    }

    @Override // androidx.activity.ComponentActivity, android.app.Activity
    public final void onBackPressed() {
        C3667WWWoWWWo c3667WWWoWWWo = this.f8752WWWWWWWW;
        if (c3667WWWoWWWo != null && c3667WWWoWWWo.f38333g && this.f8505WWWWWWWW.f8940WWoWWo >= 4) {
            WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
            wWWWoWWWWo.m13648WoWo(R.string.dialog_title_restart);
            wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_restart);
            wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new DialogInterface.OnClickListener(this) { // from class: o4.WWWWᐡWWWWೱᐡ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMSensorActivity f31691WWWWWWWWWW;

                {
                    this.f31691WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMSensorActivity vMSensorActivity = this.f31691WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMSensorActivity.f8751WWWW;
                            VMInstance vMInstance = vMSensorActivity.f8505WWWWWWWW;
                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                            AbstractC3339WWWWWWWW.m15429WWWWWWWW(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{95, -67, 15, 86, -94, 4, -81, 23, 22, -15}, new byte[]{56, -40, 123, 0, -17, 44, -127, 57}));
                            C2344WWWWWWWW.m13706WWWoWWWo(vMSensorActivity, vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{36, 59, -36, -58, 29, 117, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{86, 94, -81, -78, 124, 7, 56, -64}));
                            return;
                        default:
                            int i12 = VMSensorActivity.f8751WWWW;
                            vMSensorActivity.finish();
                            return;
                    }
                }
            });
            wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, new DialogInterface.OnClickListener(this) { // from class: o4.WWWWᐡWWWWೱᐡ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMSensorActivity f31691WWWWWWWWWW;

                {
                    this.f31691WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMSensorActivity vMSensorActivity = this.f31691WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMSensorActivity.f8751WWWW;
                            VMInstance vMInstance = vMSensorActivity.f8505WWWWWWWW;
                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                            AbstractC3339WWWWWWWW.m15429WWWWWWWW(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{95, -67, 15, 86, -94, 4, -81, 23, 22, -15}, new byte[]{56, -40, 123, 0, -17, 44, -127, 57}));
                            C2344WWWWWWWW.m13706WWWoWWWo(vMSensorActivity, vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{36, 59, -36, -58, 29, 117, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{86, 94, -81, -78, 124, 7, 56, -64}));
                            return;
                        default:
                            int i12 = VMSensorActivity.f8751WWWW;
                            vMSensorActivity.finish();
                            return;
                    }
                }
            });
            WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{122, 96, -40, -58, 29, TarConstants.LF_LINK, -32, 122, TarConstants.LF_CONTIG, 60, -108}, new byte[]{25, 18, -67, -89, 105, 84, -56, 84});
            mo742WWWW.show();
            return;
        }
        super.onBackPressed();
    }

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity, com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_vm_sensor);
        m2306WWWoWWWo((Toolbar) findViewById(R.id.toolbar));
        androidx.appcompat.app.WWWWWWWW m2307WWoWWo = m2307WWoWWo();
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(m2307WWoWWo);
        m2307WWoWWo.mo2341WoWo(true);
    }
}
