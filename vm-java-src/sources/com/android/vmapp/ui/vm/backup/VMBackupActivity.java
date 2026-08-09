package com.android.vmapp.ui.vm.backup;

import android.content.Intent;
import android.os.Bundle;
import android.view.MenuItem;
import android.view.View;
import android.widget.Button;
import androidx.appcompat.app.WWWW;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import androidx.fragment.app.Fragment;
import androidx.navigation.fragment.NavHostFragment;
import com.android.vmapp.ui.base.BaseActivity;
import com.android.vmapp.ui.vm.backup.VMBackupActivity;
import com.android.vmapp.ui.vm.backup.VMBackupRestoreActivity;
import com.clone.android.dual.space.R;
import com.google.android.gms.internal.consent_sdk.AbstractC1812WWWW;
import da.WWWWoWWWWo;
import ed.AbstractC2403WWWWoWWWWo;
import f4.DialogInterface$OnDismissListenerC2500WWWWWWWW;
import gc.C2601WWWWWWWW;
import h4.AbstractC2705WWWWWWWW;
import h4.C2706WWWWWWWW;
import h4.C2708WWWWWWWW;
import h4.C2732WWWoWWWo;
import h4.C2750WWWW;
import h4.C2752WoWo;
import h4.WWWWWWWW;
import h4.WWWoWWWo;
import hd.C2819WWWWWWWW;
import j3.C3164WWWWWWWW;
import java.util.HashSet;
import kotlin.NoWhenBranchMatchedException;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import l3.C3404WWWoWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p.C3784WWWWWWWW;
import p001WWWWoWWWWo.C0066WWWWWWWW;
import p013WWWWWWWW.o;
/* loaded from: classes.dex */
public final class VMBackupActivity extends BaseActivity {

    /* renamed from: WWo᪙WWoг᪙  reason: contains not printable characters */
    public static final /* synthetic */ int f8600WWoWWo = 0;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public C3784WWWWWWWW f8601WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public Toolbar f8602WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public Button f8603WWWWWWWW;

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public WWWW f8604WWWW;

    /* renamed from: Wo᪅Woै᪅  reason: contains not printable characters */
    public final o f8605WoWo = new o(C3333WWWWoWWWWo.m15421WWWWWWWW(C2750WWWW.class), new C2732WWWoWWWo(this, 0), new ad.WWWW(11, this), new C2732WWWoWWWo(this, 1));

    /* renamed from: WWoᆑWWoӁᆑ  reason: contains not printable characters */
    public static final void m4956WWoWWo(VMBackupActivity vMBackupActivity) {
        try {
            WWWW wwww = vMBackupActivity.f8604WWWW;
            if (wwww != null) {
                wwww.dismiss();
            }
        } catch (Throwable th2) {
            th2.printStackTrace();
        }
    }

    /* renamed from: WWWWoᆣWWWWoϿᆣ  reason: contains not printable characters */
    public final C2750WWWW m4957WWWWoWWWWo() {
        return (C2750WWWW) this.f8605WoWo.getValue();
    }

    /* renamed from: WWWWሎWWWWܣሎ  reason: contains not printable characters */
    public final void m4958WWWWWWWW() {
        if (AbstractC2705WWWWWWWW.f27731WWWWWWWW[((C2752WoWo) ((C2819WWWWWWWW) m4957WWWWoWWWWo().f27868WWWWWWWW.f28194WWWWoWWWWo).m14479WWWWWWWW()).f27886WWWWWWWW.ordinal()] == 2) {
            m4960WWWWWWWW();
        } else {
            finish();
        }
    }

    /* renamed from: WWWWሗWWWWшሗ  reason: contains not printable characters */
    public final void m4959WWWWWWWW(WWWW wwww) {
        try {
            WWWW wwww2 = this.f8604WWWW;
            if (wwww2 != null) {
                wwww2.dismiss();
            }
            this.f8604WWWW = wwww;
            wwww.setOnDismissListener(new DialogInterface$OnDismissListenerC2500WWWWWWWW(this, 1));
            wwww.show();
        } catch (Throwable th2) {
            th2.printStackTrace();
        }
    }

    /* renamed from: WWWWኈWWWWˡኈ  reason: contains not printable characters */
    public final void m4960WWWWWWWW() {
        WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
        wWWWoWWWWo.m13648WoWo(R.string.dialog_title_stop_backup);
        wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_stop_backup);
        wWWWoWWWWo.m13645WWWoWWWo(R.string.backup_bg, new WWWWWWWW(this, 0));
        wWWWoWWWWo.m13643WWWWWWWW(R.string.backup_cancel, new WWWWWWWW(this, 1));
        wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, null);
        WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-61, 92, -36, -4, -48, 70, -90, 61, -114, 0, -112}, new byte[]{-96, 46, -71, -99, -92, 35, -114, 19});
        m4959WWWWWWWW(mo742WWWW);
    }

    @Override // androidx.activity.ComponentActivity, android.app.Activity
    public final void onBackPressed() {
        m4958WWWWWWWW();
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_vm_backup);
        View findViewById = findViewById(R.id.toolbar);
        AbstractC1017WWWoWWWo.m3443WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{18, -31, TarConstants.LF_GNUTYPE_LONGNAME, -57, -103, 42, 122, -112, TarConstants.LF_FIFO, -15, 107, -57, -25, 109, TarConstants.LF_LINK, -55, 93}, new byte[]{116, -120, 34, -93, -49, 67, 31, -25}, findViewById);
        Toolbar toolbar = (Toolbar) findViewById;
        this.f8602WWWWWWWW = toolbar;
        m2306WWWoWWWo(toolbar);
        androidx.appcompat.app.WWWWWWWW m2307WWoWWo = m2307WWoWWo();
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(m2307WWoWWo);
        m2307WWoWWo.mo2341WoWo(true);
        View findViewById2 = findViewById(R.id.button);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-26, 9, TarConstants.LF_FIFO, 72, -65, 46, 105, 112, -62, 25, 17, 72, -63, 105, 34, 41, -87}, new byte[]{Byte.MIN_VALUE, 96, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 44, -23, 71, ConstantPoolEntry.CP_NameAndType, 7}));
        this.f8603WWWWWWWW = (Button) findViewById2;
        Fragment m3350WWoWWo = m3298WWWWWWWW().m3350WWoWWo(R.id.nav_host_container);
        AbstractC3339WWWWWWWW.m15428WWWWWWWW(m3350WWoWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{16, TarConstants.LF_CHR, 43, 32, -2, -61, -93, -99, 16, 41, TarConstants.LF_CHR, 108, -68, -59, -30, -112, 31, TarConstants.LF_DIR, TarConstants.LF_CHR, 108, -86, -49, -30, -99, 17, 40, 106, 34, -85, -52, -82, -45, 10, 63, TarConstants.LF_CONTIG, 41, -2, -63, -84, -105, ConstantPoolEntry.CP_NameAndType, 41, 46, 40, -90, -114, -84, -110, 8, 47, 32, 45, -86, -55, -83, -99, 80, 32, TarConstants.LF_DIR, 45, -71, -51, -89, -99, 10, 104, 9, 45, -88, -24, -83, Byte.MIN_VALUE, 10, 0, TarConstants.LF_DIR, 45, -71, -51, -89, -99, 10}, new byte[]{126, 70, 71, TarConstants.LF_GNUTYPE_LONGNAME, -34, -96, -62, -13}));
        this.f8601WWWWWWWW = ((NavHostFragment) m3350WWoWWo).m3584WWoWWo();
        C2601WWWWWWWW c2601wwwwwwww = C2601WWWWWWWW.f27306WWWWoWWWWo;
        C0066WWWWWWWW c0066wwwwwwww = new C0066WWWWWWWW(0, this, VMBackupActivity.class, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-102, 90, -36, 13, 44, 125, -26, 111, -111, 80, -13, 10, TarConstants.LF_BLK, 113, -53, 96}, new byte[]{-14, 59, -78, 105, 64, 24, -92, 14}), x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{102, 70, 97, 5, -47, -117, 126, 113, 109, TarConstants.LF_GNUTYPE_LONGNAME, 78, 2, -55, -121, TarConstants.LF_GNUTYPE_SPARSE, 126, 38, 14, 85}, new byte[]{14, 39, 15, 97, -67, -18, 60, 16}), 0, 3);
        HashSet hashSet = new HashSet();
        hashSet.addAll(c2601wwwwwwww);
        l1.WWWW wwww = new l1.WWWW(29, hashSet, new C2706WWWWWWWW(c0066wwwwwwww));
        Toolbar toolbar2 = this.f8602WWWWWWWW;
        if (toolbar2 != null) {
            C3784WWWWWWWW c3784wwwwwwww = this.f8601WWWWWWWW;
            if (c3784wwwwwwww != null) {
                AbstractC1812WWWW.m10938WWWWWWWW(toolbar2, c3784wwwwwwww, wwww);
                Toolbar toolbar3 = this.f8602WWWWWWWW;
                if (toolbar3 != null) {
                    toolbar3.setNavigationOnClickListener(new View.OnClickListener(this) { // from class: h4.WWWWo̐WWWWoȄ̐

                        /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                        public final /* synthetic */ VMBackupActivity f27713WWWWWWWWWW;

                        {
                            this.f27713WWWWWWWWWW = this;
                        }

                        @Override // android.view.View.OnClickListener
                        public final void onClick(View view) {
                            Boolean bool;
                            VMBackupActivity vMBackupActivity = this.f27713WWWWWWWWWW;
                            switch (r2) {
                                case 0:
                                    int i10 = VMBackupActivity.f8600WWoWWo;
                                    vMBackupActivity.m4958WWWWWWWW();
                                    return;
                                case 1:
                                    int i11 = VMBackupActivity.f8600WWoWWo;
                                    int ordinal = ((C2752WoWo) ((C2819WWWWWWWW) vMBackupActivity.m4957WWWWoWWWWo().f27868WWWWWWWW.f28194WWWWoWWWWo).m14479WWWWWWWW()).f27886WWWWWWWW.ordinal();
                                    if (ordinal != 0) {
                                        if (ordinal != 1) {
                                            if (ordinal == 2) {
                                                vMBackupActivity.finish();
                                                return;
                                            }
                                            throw new NoWhenBranchMatchedException();
                                        }
                                        C2750WWWW m4957WWWWoWWWWo = vMBackupActivity.m4957WWWWoWWWWo();
                                        C3404WWWoWWWo c3404WWWoWWWo = m4957WWWWoWWWWo.f27867WWWWWWWW;
                                        if (c3404WWWoWWWo != null && (bool = ((C2752WoWo) m4957WWWWoWWWWo.f27869WWoWWo.m14479WWWWWWWW()).f27889WWWWWWWW) != null) {
                                            m4957WWWWoWWWWo.m14349WWWWWWWW(new C2746WWoWWo(bool.booleanValue(), c3404WWWoWWWo, null));
                                            return;
                                        }
                                        return;
                                    }
                                    da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(vMBackupActivity);
                                    wWWWoWWWWo.m13648WoWo(R.string.dialog_title_start_backup);
                                    wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_start_backup);
                                    wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new WWWWWWWW(vMBackupActivity, 2));
                                    wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, null);
                                    WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
                                    byte[] bArr = {-59, -99, 39, ConstantPoolEntry.CP_NameAndType, -66, -14, 113, -95};
                                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                    x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-90, -17, 66, 109, -54, -105, 89, -113, -21, -77, 14}, bArr);
                                    vMBackupActivity.m4959WWWWWWWW(mo742WWWW);
                                    return;
                                default:
                                    int i12 = VMBackupActivity.f8600WWoWWo;
                                    Intent intent = new Intent(vMBackupActivity, VMBackupRestoreActivity.class);
                                    intent.addFlags(67108864);
                                    vMBackupActivity.startActivity(intent);
                                    return;
                            }
                        }
                    });
                    C3784WWWWWWWW c3784wwwwwwww2 = this.f8601WWWWWWWW;
                    if (c3784wwwwwwww2 != null) {
                        c3784wwwwwwww2.m16413WWWWWWWW(new WWWoWWWo(this, 0));
                        Button button = this.f8603WWWWWWWW;
                        if (button != null) {
                            button.setOnClickListener(new View.OnClickListener(this) { // from class: h4.WWWWo̐WWWWoȄ̐

                                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                                public final /* synthetic */ VMBackupActivity f27713WWWWWWWWWW;

                                {
                                    this.f27713WWWWWWWWWW = this;
                                }

                                @Override // android.view.View.OnClickListener
                                public final void onClick(View view) {
                                    Boolean bool;
                                    VMBackupActivity vMBackupActivity = this.f27713WWWWWWWWWW;
                                    switch (r2) {
                                        case 0:
                                            int i10 = VMBackupActivity.f8600WWoWWo;
                                            vMBackupActivity.m4958WWWWWWWW();
                                            return;
                                        case 1:
                                            int i11 = VMBackupActivity.f8600WWoWWo;
                                            int ordinal = ((C2752WoWo) ((C2819WWWWWWWW) vMBackupActivity.m4957WWWWoWWWWo().f27868WWWWWWWW.f28194WWWWoWWWWo).m14479WWWWWWWW()).f27886WWWWWWWW.ordinal();
                                            if (ordinal != 0) {
                                                if (ordinal != 1) {
                                                    if (ordinal == 2) {
                                                        vMBackupActivity.finish();
                                                        return;
                                                    }
                                                    throw new NoWhenBranchMatchedException();
                                                }
                                                C2750WWWW m4957WWWWoWWWWo = vMBackupActivity.m4957WWWWoWWWWo();
                                                C3404WWWoWWWo c3404WWWoWWWo = m4957WWWWoWWWWo.f27867WWWWWWWW;
                                                if (c3404WWWoWWWo != null && (bool = ((C2752WoWo) m4957WWWWoWWWWo.f27869WWoWWo.m14479WWWWWWWW()).f27889WWWWWWWW) != null) {
                                                    m4957WWWWoWWWWo.m14349WWWWWWWW(new C2746WWoWWo(bool.booleanValue(), c3404WWWoWWWo, null));
                                                    return;
                                                }
                                                return;
                                            }
                                            da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(vMBackupActivity);
                                            wWWWoWWWWo.m13648WoWo(R.string.dialog_title_start_backup);
                                            wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_start_backup);
                                            wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new WWWWWWWW(vMBackupActivity, 2));
                                            wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, null);
                                            WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
                                            byte[] bArr = {-59, -99, 39, ConstantPoolEntry.CP_NameAndType, -66, -14, 113, -95};
                                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-90, -17, 66, 109, -54, -105, 89, -113, -21, -77, 14}, bArr);
                                            vMBackupActivity.m4959WWWWWWWW(mo742WWWW);
                                            return;
                                        default:
                                            int i12 = VMBackupActivity.f8600WWoWWo;
                                            Intent intent = new Intent(vMBackupActivity, VMBackupRestoreActivity.class);
                                            intent.addFlags(67108864);
                                            vMBackupActivity.startActivity(intent);
                                            return;
                                    }
                                }
                            });
                            findViewById(R.id.backup_list_link).setOnClickListener(new View.OnClickListener(this) { // from class: h4.WWWWo̐WWWWoȄ̐

                                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                                public final /* synthetic */ VMBackupActivity f27713WWWWWWWWWW;

                                {
                                    this.f27713WWWWWWWWWW = this;
                                }

                                @Override // android.view.View.OnClickListener
                                public final void onClick(View view) {
                                    Boolean bool;
                                    VMBackupActivity vMBackupActivity = this.f27713WWWWWWWWWW;
                                    switch (r2) {
                                        case 0:
                                            int i10 = VMBackupActivity.f8600WWoWWo;
                                            vMBackupActivity.m4958WWWWWWWW();
                                            return;
                                        case 1:
                                            int i11 = VMBackupActivity.f8600WWoWWo;
                                            int ordinal = ((C2752WoWo) ((C2819WWWWWWWW) vMBackupActivity.m4957WWWWoWWWWo().f27868WWWWWWWW.f28194WWWWoWWWWo).m14479WWWWWWWW()).f27886WWWWWWWW.ordinal();
                                            if (ordinal != 0) {
                                                if (ordinal != 1) {
                                                    if (ordinal == 2) {
                                                        vMBackupActivity.finish();
                                                        return;
                                                    }
                                                    throw new NoWhenBranchMatchedException();
                                                }
                                                C2750WWWW m4957WWWWoWWWWo = vMBackupActivity.m4957WWWWoWWWWo();
                                                C3404WWWoWWWo c3404WWWoWWWo = m4957WWWWoWWWWo.f27867WWWWWWWW;
                                                if (c3404WWWoWWWo != null && (bool = ((C2752WoWo) m4957WWWWoWWWWo.f27869WWoWWo.m14479WWWWWWWW()).f27889WWWWWWWW) != null) {
                                                    m4957WWWWoWWWWo.m14349WWWWWWWW(new C2746WWoWWo(bool.booleanValue(), c3404WWWoWWWo, null));
                                                    return;
                                                }
                                                return;
                                            }
                                            da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(vMBackupActivity);
                                            wWWWoWWWWo.m13648WoWo(R.string.dialog_title_start_backup);
                                            wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_start_backup);
                                            wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new WWWWWWWW(vMBackupActivity, 2));
                                            wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, null);
                                            WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
                                            byte[] bArr = {-59, -99, 39, ConstantPoolEntry.CP_NameAndType, -66, -14, 113, -95};
                                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-90, -17, 66, 109, -54, -105, 89, -113, -21, -77, 14}, bArr);
                                            vMBackupActivity.m4959WWWWWWWW(mo742WWWW);
                                            return;
                                        default:
                                            int i12 = VMBackupActivity.f8600WWoWWo;
                                            Intent intent = new Intent(vMBackupActivity, VMBackupRestoreActivity.class);
                                            intent.addFlags(67108864);
                                            vMBackupActivity.startActivity(intent);
                                            return;
                                    }
                                }
                            });
                            AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(ib.WWWWoWWWWo.m14598WWWWWWWW(this), null, new C2708WWWWWWWW(this, null), 3);
                            return;
                        }
                        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{97, -87, TarConstants.LF_LINK, -80, -2, 104, -79}, new byte[]{ConstantPoolEntry.CP_NameAndType, -21, 68, -60, -118, 7, -33, -64}));
                        throw null;
                    }
                    AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{104, 108, -19, -118, 4, 7, -67, ConstantPoolEntry.CP_NameAndType, 105, 97, -9, -84, 25}, new byte[]{6, 13, -101, -55, 107, 105, -55, 126}));
                    throw null;
                }
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-11, -8, -16, -106, -43, -80, -50, 94}, new byte[]{-104, -84, -97, -7, -71, -46, -81, 44}));
                throw null;
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-59, 74, 38, -17, 116, -123, 97, -36, -60, 71, 60, -55, 105}, new byte[]{-85, 43, 80, -84, 27, -21, 21, -82}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{21, -67, -53, -40, -73, 69, -56, 27}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, -23, -92, -73, -37, 39, -87, 105}));
        throw null;
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, android.app.Activity
    public final boolean onOptionsItemSelected(MenuItem menuItem) {
        byte[] bArr = {TarConstants.LF_MULTIVOLUME, 101, -73, -58, 74, 25, -21, -98};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(menuItem, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{36, 17, -46, -85}, bArr));
        if (menuItem.getItemId() == 16908332) {
            m4958WWWWWWWW();
        }
        return super.onOptionsItemSelected(menuItem);
    }
}
