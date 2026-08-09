package com.android.vmapp.ui.vm.settings;

import android.content.DialogInterface;
import android.os.Bundle;
import androidx.appcompat.app.WWWW;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.C1020WWWoWWWo;
import androidx.fragment.app.WWWWWWWW;
import androidx.preference.o;
import com.android.vmapp.ui.base.preference.BasePreferenceActivity;
import com.android.vmapp.ui.vm.settings.VMNetworkActivity;
import com.android.vmcore.VMInstance;
import com.clone.android.dual.space.R;
import da.WWWWoWWWWo;
import e4.C2344WWWWWWWW;
import j3.C3164WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import o4.C3671WWoWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
/* loaded from: classes.dex */
public final class VMNetworkActivity extends BasePreferenceActivity {

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public static final /* synthetic */ int f8743WWWW = 0;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public C3671WWoWWo f8744WWWWWWWW;

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity
    /* renamed from: WWoᆑWWoӁᆑ */
    public final o mo4936WWoWWo() {
        if (this.f8505WWWWWWWW == null) {
            finish();
            return null;
        }
        C3671WWoWWo c3671WWoWWo = new C3671WWoWWo();
        C1020WWWoWWWo m3298WWWWWWWW = m3298WWWWWWWW();
        m3298WWWWWWWW.getClass();
        WWWWWWWW wwwwwwww = new WWWWWWWW(m3298WWWWWWWW);
        wwwwwwww.m3423WWoWWo(R.id.settings_container, c3671WWoWWo, null);
        wwwwwwww.m3390WWWoWWWo();
        this.f8744WWWWWWWW = c3671WWoWWo;
        return c3671WWoWWo;
    }

    @Override // androidx.activity.ComponentActivity, android.app.Activity
    public final void onBackPressed() {
        C3671WWoWWo c3671WWoWWo = this.f8744WWWWWWWW;
        if (c3671WWoWWo != null && c3671WWoWWo.f38337i && this.f8505WWWWWWWW.f8940WWoWWo >= 4) {
            WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
            wWWWoWWWWo.m13648WoWo(R.string.dialog_title_restart);
            wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_restart);
            wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new DialogInterface.OnClickListener(this) { // from class: o4.WWWW֭WWWWྥ֭

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMNetworkActivity f31634WWWWWWWWWW;

                {
                    this.f31634WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMNetworkActivity vMNetworkActivity = this.f31634WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMNetworkActivity.f8743WWWW;
                            VMInstance vMInstance = vMNetworkActivity.f8505WWWWWWWW;
                            byte[] bArr = {105, 110, 20, 63, TarConstants.LF_GNUTYPE_SPARSE, -72, 70, -11, 32, 34};
                            byte[] bArr2 = {14, ConstantPoolEntry.CP_InterfaceMethodref, 96, 105, 30, -112, 104, -37};
                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                            AbstractC3339WWWWWWWW.m15429WWWWWWWW(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                            C2344WWWWWWWW.m13706WWWoWWWo(vMNetworkActivity, vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-86, -123, -122, 9, 80, 123, 109}, new byte[]{-40, -32, -11, 125, TarConstants.LF_LINK, 9, 25, 56}));
                            return;
                        default:
                            int i12 = VMNetworkActivity.f8743WWWW;
                            vMNetworkActivity.finish();
                            return;
                    }
                }
            });
            wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, new DialogInterface.OnClickListener(this) { // from class: o4.WWWW֭WWWWྥ֭

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMNetworkActivity f31634WWWWWWWWWW;

                {
                    this.f31634WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMNetworkActivity vMNetworkActivity = this.f31634WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMNetworkActivity.f8743WWWW;
                            VMInstance vMInstance = vMNetworkActivity.f8505WWWWWWWW;
                            byte[] bArr = {105, 110, 20, 63, TarConstants.LF_GNUTYPE_SPARSE, -72, 70, -11, 32, 34};
                            byte[] bArr2 = {14, ConstantPoolEntry.CP_InterfaceMethodref, 96, 105, 30, -112, 104, -37};
                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                            AbstractC3339WWWWWWWW.m15429WWWWWWWW(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                            C2344WWWWWWWW.m13706WWWoWWWo(vMNetworkActivity, vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-86, -123, -122, 9, 80, 123, 109}, new byte[]{-40, -32, -11, 125, TarConstants.LF_LINK, 9, 25, 56}));
                            return;
                        default:
                            int i12 = VMNetworkActivity.f8743WWWW;
                            vMNetworkActivity.finish();
                            return;
                    }
                }
            });
            WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, -14, TarConstants.LF_CONTIG, 91, -11, 57, 1, 68, 122, -82, 123}, new byte[]{84, Byte.MIN_VALUE, 82, 58, -127, 92, 41, 106});
            mo742WWWW.show();
            return;
        }
        super.onBackPressed();
    }

    @Override // com.android.vmapp.ui.base.preference.BasePreferenceActivity, com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_vm_network);
        m2306WWWoWWWo((Toolbar) findViewById(R.id.toolbar));
        androidx.appcompat.app.WWWWWWWW m2307WWoWWo = m2307WWoWWo();
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(m2307WWoWWo);
        m2307WWoWWo.mo2341WoWo(true);
    }
}
