package com.android.vmapp.ui.vm.settings;

import android.content.DialogInterface;
import android.os.Bundle;
import androidx.appcompat.app.WWWW;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.C1020WWWoWWWo;
import androidx.fragment.app.WWWWWWWW;
import androidx.preference.o;
import com.android.vmapp.ui.base.preference.BasePreferenceActivity;
import com.android.vmapp.ui.vm.settings.VMPhoneActivity;
import com.android.vmcore.VMInstance;
import com.clone.android.dual.space.R;
import da.WWWWoWWWWo;
import e4.C2344WWWWWWWW;
import j3.C3164WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import o4.C3674WWoWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
/* loaded from: classes.dex */
public final class VMPhoneActivity extends BasePreferenceActivity {

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public static final /* synthetic */ int f8746WWWW = 0;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public C3674WWoWWo f8747WWWWWWWW;

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity
    /* renamed from: WWoᆑWWoӁᆑ */
    public final o mo4936WWoWWo() {
        if (this.f8505WWWWWWWW == null) {
            finish();
            return null;
        }
        C3674WWoWWo c3674WWoWWo = new C3674WWoWWo();
        C1020WWWoWWWo m3298WWWWWWWW = m3298WWWWWWWW();
        m3298WWWWWWWW.getClass();
        WWWWWWWW wwwwwwww = new WWWWWWWW(m3298WWWWWWWW);
        wwwwwwww.m3423WWoWWo(R.id.settings_container, c3674WWoWWo, null);
        wwwwwwww.m3390WWWoWWWo();
        this.f8747WWWWWWWW = c3674WWoWWo;
        return c3674WWoWWo;
    }

    @Override // androidx.activity.ComponentActivity, android.app.Activity
    public final void onBackPressed() {
        C3674WWoWWo c3674WWoWWo = this.f8747WWWWWWWW;
        if (c3674WWoWWo != null && c3674WWoWWo.f38341i && this.f8505WWWWWWWW.f8940WWoWWo >= 4) {
            WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
            wWWWoWWWWo.m13648WoWo(R.string.dialog_title_restart);
            wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_restart);
            wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new DialogInterface.OnClickListener(this) { // from class: o4.WWWWoඤWWWWoెඤ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMPhoneActivity f31618WWWWWWWWWW;

                {
                    this.f31618WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMPhoneActivity vMPhoneActivity = this.f31618WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMPhoneActivity.f8746WWWW;
                            VMInstance vMInstance = vMPhoneActivity.f8505WWWWWWWW;
                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                            AbstractC3339WWWWWWWW.m15429WWWWWWWW(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-32, 74, 25, 65, -48, -119, -89, Byte.MIN_VALUE, -87, 6}, new byte[]{-121, 47, 109, 23, -99, -95, -119, -82}));
                            C2344WWWWWWWW.m13706WWWoWWWo(vMPhoneActivity, vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{10, 35, -109, 126, 17, -76, 35}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, 70, -32, 10, 112, -58, 87, 20}));
                            return;
                        default:
                            int i12 = VMPhoneActivity.f8746WWWW;
                            vMPhoneActivity.finish();
                            return;
                    }
                }
            });
            wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, new DialogInterface.OnClickListener(this) { // from class: o4.WWWWoඤWWWWoెඤ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMPhoneActivity f31618WWWWWWWWWW;

                {
                    this.f31618WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMPhoneActivity vMPhoneActivity = this.f31618WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMPhoneActivity.f8746WWWW;
                            VMInstance vMInstance = vMPhoneActivity.f8505WWWWWWWW;
                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                            AbstractC3339WWWWWWWW.m15429WWWWWWWW(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-32, 74, 25, 65, -48, -119, -89, Byte.MIN_VALUE, -87, 6}, new byte[]{-121, 47, 109, 23, -99, -95, -119, -82}));
                            C2344WWWWWWWW.m13706WWWoWWWo(vMPhoneActivity, vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{10, 35, -109, 126, 17, -76, 35}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, 70, -32, 10, 112, -58, 87, 20}));
                            return;
                        default:
                            int i12 = VMPhoneActivity.f8746WWWW;
                            vMPhoneActivity.finish();
                            return;
                    }
                }
            });
            WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-67, 32, -74, 15, -115, 63, -66, -102, -16, 124, -6}, new byte[]{-34, 82, -45, 110, -7, 90, -106, -76});
            mo742WWWW.show();
            return;
        }
        super.onBackPressed();
    }

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity, com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_vm_phone);
        m2306WWWoWWWo((Toolbar) findViewById(R.id.toolbar));
        androidx.appcompat.app.WWWWWWWW m2307WWoWWo = m2307WWoWWo();
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(m2307WWoWWo);
        m2307WWoWWo.mo2341WoWo(true);
    }
}
