package com.android.vmapp.ui.vm.settings;

import android.content.DialogInterface;
import android.os.Bundle;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.C1020WWWoWWWo;
import androidx.fragment.app.WWWWWWWW;
import androidx.preference.o;
import com.android.vmapp.ui.base.preference.BasePreferenceActivity;
import com.android.vmapp.ui.vm.settings.VMHWInfoActivity;
import com.android.vmcore.VMInstance;
import com.clone.android.dual.space.R;
import da.WWWWoWWWWo;
import e4.C2344WWWWWWWW;
import j3.C3164WWWWWWWW;
import o4.C3643WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
/* loaded from: classes.dex */
public class VMHWInfoActivity extends BasePreferenceActivity {

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public static final /* synthetic */ int f8741WWWW = 0;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public C3643WWWWWWWW f8742WWWWWWWW;

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity
    /* renamed from: WWoᆑWWoӁᆑ */
    public final o mo4936WWoWWo() {
        if (this.f8505WWWWWWWW == null) {
            finish();
            return null;
        }
        C3643WWWWWWWW c3643wwwwwwww = new C3643WWWWWWWW();
        C1020WWWoWWWo m3298WWWWWWWW = m3298WWWWWWWW();
        m3298WWWWWWWW.getClass();
        WWWWWWWW wwwwwwww = new WWWWWWWW(m3298WWWWWWWW);
        wwwwwwww.m3423WWoWWo(R.id.settings_container, c3643wwwwwwww, null);
        wwwwwwww.m3390WWWoWWWo();
        this.f8742WWWWWWWW = c3643wwwwwwww;
        return c3643wwwwwwww;
    }

    @Override // androidx.activity.ComponentActivity, android.app.Activity
    public final void onBackPressed() {
        C3643WWWWWWWW c3643wwwwwwww = this.f8742WWWWWWWW;
        if (c3643wwwwwwww != null && c3643wwwwwwww.f38326h && this.f8505WWWWWWWW.f8940WWoWWo >= 4) {
            WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
            wWWWoWWWWo.m13648WoWo(R.string.dialog_title_restart);
            wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_restart);
            wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new DialogInterface.OnClickListener(this) { // from class: o4.WWWW̏WWWWβ̏

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMHWInfoActivity f31627WWWWWWWWWW;

                {
                    this.f31627WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMHWInfoActivity vMHWInfoActivity = this.f31627WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMHWInfoActivity.f8741WWWW;
                            VMInstance vMInstance = vMHWInfoActivity.f8505WWWWWWWW;
                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                            C2344WWWWWWWW.m13706WWWoWWWo(vMHWInfoActivity, vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_MULTIVOLUME, -51, 96, -75, -120, 44, -17}, new byte[]{63, -88, 19, -63, -23, 94, -101, -40}));
                            return;
                        default:
                            int i12 = VMHWInfoActivity.f8741WWWW;
                            vMHWInfoActivity.finish();
                            return;
                    }
                }
            });
            wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, new DialogInterface.OnClickListener(this) { // from class: o4.WWWW̏WWWWβ̏

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMHWInfoActivity f31627WWWWWWWWWW;

                {
                    this.f31627WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMHWInfoActivity vMHWInfoActivity = this.f31627WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMHWInfoActivity.f8741WWWW;
                            VMInstance vMInstance = vMHWInfoActivity.f8505WWWWWWWW;
                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                            C2344WWWWWWWW.m13706WWWoWWWo(vMHWInfoActivity, vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_MULTIVOLUME, -51, 96, -75, -120, 44, -17}, new byte[]{63, -88, 19, -63, -23, 94, -101, -40}));
                            return;
                        default:
                            int i12 = VMHWInfoActivity.f8741WWWW;
                            vMHWInfoActivity.finish();
                            return;
                    }
                }
            });
            wWWWoWWWWo.mo742WWWW().show();
            return;
        }
        super.onBackPressed();
    }

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity, com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_vm_hw_info);
        m2306WWWoWWWo((Toolbar) findViewById(R.id.toolbar));
        m2307WWoWWo().mo2341WoWo(true);
    }
}
