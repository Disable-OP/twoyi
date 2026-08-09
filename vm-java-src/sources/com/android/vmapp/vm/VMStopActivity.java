package com.android.vmapp.vm;

import android.app.ActivityManager;
import android.content.DialogInterface;
import android.content.Intent;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;
import androidx.appcompat.app.C0791WWWWWWWW;
import androidx.appcompat.app.WWWW;
import com.android.vmapp.ui.base.BaseActivity;
import com.android.vmapp.vm.VMStopActivity;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.ResetEvent;
import com.android.vmcore.event.ShutdownEvent;
import com.blankj.utilcode.util.C1628WWWWWWWW;
import com.blankj.utilcode.util.C1644WWWoWWWo;
import com.clone.android.dual.space.R;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import da.WWWWoWWWWo;
import eh.InterfaceC2472WWWWWWWW;
import j3.C3164WWWWWWWW;
import java.util.Iterator;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.greenrobot.eventbus.ThreadMode;
import r4.DialogInterface$OnDismissListenerC3968WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class VMStopActivity extends BaseActivity {

    /* renamed from: Wo᪅Woै᪅  reason: contains not printable characters */
    public static final /* synthetic */ int f8787WoWo = 0;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public WWWW f8788WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public String f8789WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public boolean f8790WWWWWWWW;

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public boolean f8791WWWW;

    /* renamed from: WWoᆑWWoӁᆑ  reason: contains not printable characters */
    public static void m5022WWoWWo(View view) {
        view.setSystemUiVisibility(5894);
    }

    /* renamed from: WWWWoᆣWWWWoϿᆣ  reason: contains not printable characters */
    public final ActivityManager.AppTask m5023WWWWoWWWWo() {
        Intent intent;
        int i10 = this.f8504WWWWWWWW;
        byte[] bArr = {-105, -18, -11, -92, -1, -90, -7, TarConstants.LF_LINK};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        for (ActivityManager.AppTask appTask : ((ActivityManager) getSystemService(WWWWWWWW.m17835WWWWWWWW(new byte[]{-10, -115, -127, -51, -119, -49, -115, 72}, bArr))).getAppTasks()) {
            intent = appTask.getTaskInfo().baseIntent;
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            if (i10 == intent.getIntExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_SYMLINK, 78, 4, 99, -114}, new byte[]{68, 35, 91, 10, -22, -102, -27, 123}), -1)) {
                return appTask;
            }
        }
        return null;
    }

    /* renamed from: WWWWሎWWWWܣሎ  reason: contains not printable characters */
    public final void m5024WWWWWWWW() {
        ActivityManager.AppTask appTask;
        Intent launchIntentForPackage;
        Intent intent;
        Intent intent2;
        Intent launchIntentForPackage2 = getPackageManager().getLaunchIntentForPackage(getPackageName());
        byte[] bArr = {TarConstants.LF_LINK, Byte.MAX_VALUE, -121, -105, -100, 10, 107, 73};
        byte[] bArr2 = {80, 28, -13, -2, -22, 99, 31, TarConstants.LF_NORMAL};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        Iterator<ActivityManager.AppTask> it = ((ActivityManager) getSystemService(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2))).getAppTasks().iterator();
        while (true) {
            if (it.hasNext()) {
                appTask = it.next();
                intent2 = appTask.getTaskInfo().baseIntent;
                if (intent2.getComponent().equals(launchIntentForPackage2.getComponent())) {
                    break;
                }
            } else {
                appTask = null;
                break;
            }
        }
        if (appTask != null) {
            intent = appTask.getTaskInfo().baseIntent;
            launchIntentForPackage = new Intent(intent);
        } else {
            launchIntentForPackage = getPackageManager().getLaunchIntentForPackage(getPackageName());
            launchIntentForPackage.addFlags(268435456);
        }
        byte[] bArr3 = {110, 4, -3, TarConstants.LF_DIR, -68, 60, -81, 85};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        launchIntentForPackage.putExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{24, 105, -94, 92, -40}, bArr3), -1);
        launchIntentForPackage.addFlags(67108864);
        launchIntentForPackage.addFlags(536870912);
        startActivity(launchIntentForPackage);
    }

    /* renamed from: WWWWሗWWWWшሗ  reason: contains not printable characters */
    public final void m5025WWWWWWWW() {
        String str;
        WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-67, -58, TarConstants.LF_GNUTYPE_LONGLINK, TarConstants.LF_MULTIVOLUME, -98, -72, -35, 13}, new byte[]{-50, -82, 62, 57, -6, -41, -86, 99}).equals(this.f8789WWWWWWWW)) {
            str = getString(R.string.failed_with_arg, getString(R.string.dialog_title_shutdown));
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{109, -98, 68, 121, -9, 26, 16}, new byte[]{31, -5, TarConstants.LF_CONTIG, 13, -106, 104, 100, 93}).equals(this.f8789WWWWWWWW)) {
            str = getString(R.string.failed_with_arg, getString(R.string.dialog_title_restart));
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-22, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -24, -103, 41}, new byte[]{-104, 2, -101, -4, 93, 110, -13, 67}).equals(this.f8789WWWWWWWW)) {
            str = getString(R.string.failed_with_arg, getString(R.string.dialog_title_reset));
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{7, 38, -4, 79, 44, -4}, new byte[]{117, 67, -116, 46, 69, -114, 25, 57}).equals(this.f8789WWWWWWWW)) {
            str = getString(R.string.failed_with_arg, getString(R.string.dialog_title_repair));
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{119, -127, 42, TarConstants.LF_MULTIVOLUME, -50, -39}, new byte[]{19, -28, 70, 40, -70, -68, 99, -119}).equals(this.f8789WWWWWWWW)) {
            str = getString(R.string.failed_with_arg, getString(R.string.dialog_title_delete));
        } else {
            str = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        C0791WWWWWWWW c0791wwwwwwww = (C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW;
        c0791wwwwwwww.f3561WWoWWo = str;
        wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_got_it, null);
        c0791wwwwwwww.f3553WWWWWWWW = new DialogInterface$OnDismissListenerC3968WWWWWWWW(this, 1);
        WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
        mo742WWWW.show();
        if (this.f8790WWWWWWWW) {
            m5022WWoWWo(mo742WWWW.getWindow().getDecorView());
        }
    }

    /* renamed from: WWWWኈWWWWˡኈ  reason: contains not printable characters */
    public final void m5026WWWWWWWW() {
        WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
        View inflate = LayoutInflater.from(this).inflate(R.layout.dialog_loading, (ViewGroup) null, false);
        ((TextView) inflate.findViewById(R.id.message)).setText(R.string.dialog_msg_in_progress);
        wWWWoWWWWo.m13644WWWWWWWW(inflate);
        ((C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW).f3567WoWo = false;
        WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
        this.f8788WWWWWWWW = mo742WWWW;
        mo742WWWW.show();
        if (this.f8790WWWWWWWW) {
            m5022WWoWWo(this.f8788WWWWWWWW.getWindow().getDecorView());
        }
    }

    /* renamed from: WWኮWWႉኮ  reason: contains not printable characters */
    public final void m5027WWWW() {
        String str;
        WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-126, 17, -122, -81, TarConstants.LF_SYMLINK, 71, -123, 82}, new byte[]{-15, 121, -13, -37, 86, 40, -14, 60}).equals(this.f8789WWWWWWWW)) {
            str = getString(R.string.succeed_with_arg, getString(R.string.dialog_title_shutdown));
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{8, -15, -107, TarConstants.LF_MULTIVOLUME, Byte.MAX_VALUE, -90, -119}, new byte[]{122, -108, -26, 57, 30, -44, -3, 66}).equals(this.f8789WWWWWWWW)) {
            str = getString(R.string.succeed_with_arg, getString(R.string.dialog_title_restart));
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, 29, 1, -105, -82}, new byte[]{-20, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 114, -14, -38, 106, 124, 33}).equals(this.f8789WWWWWWWW)) {
            str = getString(R.string.succeed_with_arg, getString(R.string.dialog_title_reset));
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-48, 45, TarConstants.LF_NORMAL, -124, -50, -98}, new byte[]{-94, 72, 64, -27, -89, -20, 117, -54}).equals(this.f8789WWWWWWWW)) {
            str = getString(R.string.dialog_msg_repair_succeed);
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-58, -65, 14, 72, 85, -107}, new byte[]{-94, -38, 98, 45, 33, -16, 66, -44}).equals(this.f8789WWWWWWWW)) {
            str = getString(R.string.succeed_with_arg, getString(R.string.dialog_title_delete));
        } else {
            str = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        C0791WWWWWWWW c0791wwwwwwww = (C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW;
        c0791wwwwwwww.f3561WWoWWo = str;
        wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_got_it, null);
        c0791wwwwwwww.f3553WWWWWWWW = new DialogInterface$OnDismissListenerC3968WWWWWWWW(this, 0);
        WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
        mo742WWWW.show();
        if (this.f8790WWWWWWWW) {
            m5022WWoWWo(mo742WWWW.getWindow().getDecorView());
        }
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        getWindow().setBackgroundDrawableResource(17170445);
        super.onCreate(bundle);
        overridePendingTransition(0, 0);
        VMInstance vMInstance = this.f8505WWWWWWWW;
        if (vMInstance == null) {
            finish();
            return;
        }
        vMInstance.f8939WWWoWWWo.m13950WWWW(this);
        Intent intent = getIntent();
        byte[] bArr = {8, -44, -84, 43, -59, 47, -16, -109, 15, -29, -94, TarConstants.LF_SYMLINK, -13, 32, -2, -120, 18, -45, -83};
        byte[] bArr2 = {123, -68, -61, 92, -102, TarConstants.LF_MULTIVOLUME, -97, -4};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        this.f8791WWWW = intent.getBooleanExtra(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), false);
        this.f8790WWWWWWWW = getIntent().getBooleanExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{122, 86, -115, TarConstants.LF_LINK, 14, -55, -93, -74, 121, 70, -113}, new byte[]{28, 35, -31, 93, 81, -70, -64, -60}), false);
        this.f8789WWWWWWWW = getIntent().getStringExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{-43, 86, 96}, new byte[]{-74, 59, 4, 26, -45, TarConstants.LF_GNUTYPE_LONGNAME, 37, 40}));
        if (WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, -30, 82, -50, TarConstants.LF_MULTIVOLUME, -85, 25, 78}, new byte[]{68, -118, 39, -70, 41, -60, 110, 32}).equals(this.f8789WWWWWWWW)) {
            WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
            wWWWoWWWWo.m13648WoWo(R.string.dialog_title_shutdown);
            wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_shutdown);
            wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new DialogInterface.OnClickListener(this) { // from class: r4.WWoڢWWo࢞ڢ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMStopActivity f32989WWWWWWWWWW;

                {
                    this.f32989WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMStopActivity vMStopActivity = this.f32989WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            vMStopActivity.f8505WWWWWWWW.m5097WW(false, true);
                            return;
                        case 1:
                            int i12 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            vMStopActivity.f8505WWWWWWWW.m5097WW(false, false);
                            return;
                        case 2:
                            int i13 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            VMInstance vMInstance2 = vMStopActivity.f8505WWWWWWWW;
                            vMInstance2.m5098WoWo().post(new com.android.vmcore.WWWWoWWWWo(vMInstance2, 4));
                            return;
                        case 3:
                            int i14 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            VMInstance vMInstance3 = vMStopActivity.f8505WWWWWWWW;
                            vMInstance3.m5098WoWo().post(new com.android.vmcore.WWWWoWWWWo(vMInstance3, 4));
                            return;
                        default:
                            int i15 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            vMStopActivity.f8505WWWWWWWW.m5097WW(true, false);
                            return;
                    }
                }
            });
            wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, new n2.WWWWWWWW(8));
            ((C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW).f3562WWoWWo = new DialogInterface.OnCancelListener(this) { // from class: r4.WWWWܬWWWWೖܬ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMStopActivity f32977WWWWWWWWWW;

                {
                    this.f32977WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnCancelListener
                public final void onCancel(DialogInterface dialogInterface) {
                    VMStopActivity vMStopActivity = this.f32977WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i10 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        case 1:
                            int i11 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        case 2:
                            int i12 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        case 3:
                            int i13 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        default:
                            int i14 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                    }
                }
            };
            WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
            mo742WWWW.show();
            if (this.f8790WWWWWWWW) {
                m5022WWoWWo(mo742WWWW.getWindow().getDecorView());
            }
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-19, -53, -101, 93, 43, -114, 28}, new byte[]{-97, -82, -24, 41, 74, -4, 104, -75}).equals(this.f8789WWWWWWWW)) {
            if (getIntent().getBooleanExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{-104, -42, 30, 45, -68, 14, -21}, new byte[]{-5, -71, 112, TarConstants.LF_GNUTYPE_LONGLINK, -43, 124, -122, -84}), false)) {
                WWWWoWWWWo wWWWoWWWWo2 = new WWWWoWWWWo(this);
                wWWWoWWWWo2.m13648WoWo(R.string.dialog_title_restart);
                wWWWoWWWWo2.m13642WWWWWWWW(R.string.dialog_msg_restart2);
                wWWWoWWWWo2.m13645WWWoWWWo(R.string.dialog_button_confirm, new DialogInterface.OnClickListener(this) { // from class: r4.WWoڢWWo࢞ڢ

                    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                    public final /* synthetic */ VMStopActivity f32989WWWWWWWWWW;

                    {
                        this.f32989WWWWWWWWWW = this;
                    }

                    @Override // android.content.DialogInterface.OnClickListener
                    public final void onClick(DialogInterface dialogInterface, int i10) {
                        VMStopActivity vMStopActivity = this.f32989WWWWWWWWWW;
                        switch (r2) {
                            case 0:
                                int i11 = VMStopActivity.f8787WoWo;
                                vMStopActivity.m5026WWWWWWWW();
                                vMStopActivity.f8505WWWWWWWW.m5097WW(false, true);
                                return;
                            case 1:
                                int i12 = VMStopActivity.f8787WoWo;
                                vMStopActivity.m5026WWWWWWWW();
                                vMStopActivity.f8505WWWWWWWW.m5097WW(false, false);
                                return;
                            case 2:
                                int i13 = VMStopActivity.f8787WoWo;
                                vMStopActivity.m5026WWWWWWWW();
                                VMInstance vMInstance2 = vMStopActivity.f8505WWWWWWWW;
                                vMInstance2.m5098WoWo().post(new com.android.vmcore.WWWWoWWWWo(vMInstance2, 4));
                                return;
                            case 3:
                                int i14 = VMStopActivity.f8787WoWo;
                                vMStopActivity.m5026WWWWWWWW();
                                VMInstance vMInstance3 = vMStopActivity.f8505WWWWWWWW;
                                vMInstance3.m5098WoWo().post(new com.android.vmcore.WWWWoWWWWo(vMInstance3, 4));
                                return;
                            default:
                                int i15 = VMStopActivity.f8787WoWo;
                                vMStopActivity.m5026WWWWWWWW();
                                vMStopActivity.f8505WWWWWWWW.m5097WW(true, false);
                                return;
                        }
                    }
                });
                wWWWoWWWWo2.m13646WWoWWo(R.string.dialog_button_cancel, new n2.WWWWWWWW(12));
                ((C0791WWWWWWWW) wWWWoWWWWo2.f1045WWWWWWWW).f3562WWoWWo = new DialogInterface.OnCancelListener(this) { // from class: r4.WWWWܬWWWWೖܬ

                    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                    public final /* synthetic */ VMStopActivity f32977WWWWWWWWWW;

                    {
                        this.f32977WWWWWWWWWW = this;
                    }

                    @Override // android.content.DialogInterface.OnCancelListener
                    public final void onCancel(DialogInterface dialogInterface) {
                        VMStopActivity vMStopActivity = this.f32977WWWWWWWWWW;
                        switch (r2) {
                            case 0:
                                int i10 = VMStopActivity.f8787WoWo;
                                vMStopActivity.finish();
                                return;
                            case 1:
                                int i11 = VMStopActivity.f8787WoWo;
                                vMStopActivity.finish();
                                return;
                            case 2:
                                int i12 = VMStopActivity.f8787WoWo;
                                vMStopActivity.finish();
                                return;
                            case 3:
                                int i13 = VMStopActivity.f8787WoWo;
                                vMStopActivity.finish();
                                return;
                            default:
                                int i14 = VMStopActivity.f8787WoWo;
                                vMStopActivity.finish();
                                return;
                        }
                    }
                };
                WWWW mo742WWWW2 = wWWWoWWWWo2.mo742WWWW();
                mo742WWWW2.show();
                if (this.f8790WWWWWWWW) {
                    m5022WWoWWo(mo742WWWW2.getWindow().getDecorView());
                    return;
                }
                return;
            }
            m5026WWWWWWWW();
            this.f8505WWWWWWWW.m5097WW(true, false);
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{10, -60, 23, 118, -2}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, -95, 100, 19, -118, -120, 21, -16}).equals(this.f8789WWWWWWWW)) {
            WWWWoWWWWo wWWWoWWWWo3 = new WWWWoWWWWo(this);
            wWWWoWWWWo3.m13648WoWo(R.string.dialog_title_reset);
            wWWWoWWWWo3.m13642WWWWWWWW(R.string.dialog_msg_reset);
            wWWWoWWWWo3.m13645WWWoWWWo(R.string.dialog_button_confirm, new DialogInterface.OnClickListener(this) { // from class: r4.WWoڢWWo࢞ڢ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMStopActivity f32989WWWWWWWWWW;

                {
                    this.f32989WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMStopActivity vMStopActivity = this.f32989WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            vMStopActivity.f8505WWWWWWWW.m5097WW(false, true);
                            return;
                        case 1:
                            int i12 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            vMStopActivity.f8505WWWWWWWW.m5097WW(false, false);
                            return;
                        case 2:
                            int i13 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            VMInstance vMInstance2 = vMStopActivity.f8505WWWWWWWW;
                            vMInstance2.m5098WoWo().post(new com.android.vmcore.WWWWoWWWWo(vMInstance2, 4));
                            return;
                        case 3:
                            int i14 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            VMInstance vMInstance3 = vMStopActivity.f8505WWWWWWWW;
                            vMInstance3.m5098WoWo().post(new com.android.vmcore.WWWWoWWWWo(vMInstance3, 4));
                            return;
                        default:
                            int i15 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            vMStopActivity.f8505WWWWWWWW.m5097WW(true, false);
                            return;
                    }
                }
            });
            wWWWoWWWWo3.m13646WWoWWo(R.string.dialog_button_cancel, new n2.WWWWWWWW(9));
            ((C0791WWWWWWWW) wWWWoWWWWo3.f1045WWWWWWWW).f3562WWoWWo = new DialogInterface.OnCancelListener(this) { // from class: r4.WWWWܬWWWWೖܬ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMStopActivity f32977WWWWWWWWWW;

                {
                    this.f32977WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnCancelListener
                public final void onCancel(DialogInterface dialogInterface) {
                    VMStopActivity vMStopActivity = this.f32977WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i10 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        case 1:
                            int i11 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        case 2:
                            int i12 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        case 3:
                            int i13 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        default:
                            int i14 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                    }
                }
            };
            WWWW mo742WWWW3 = wWWWoWWWWo3.mo742WWWW();
            mo742WWWW3.show();
            if (this.f8790WWWWWWWW) {
                m5022WWoWWo(mo742WWWW3.getWindow().getDecorView());
            }
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-14, -89, -9, -82, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -72}, new byte[]{Byte.MIN_VALUE, -62, -121, -49, TarConstants.LF_LINK, -54, -71, -43}).equals(this.f8789WWWWWWWW)) {
            WWWWoWWWWo wWWWoWWWWo4 = new WWWWoWWWWo(this);
            wWWWoWWWWo4.m13648WoWo(R.string.dialog_title_repair);
            wWWWoWWWWo4.m13642WWWWWWWW(R.string.dialog_msg_repair);
            wWWWoWWWWo4.m13645WWWoWWWo(R.string.dialog_button_confirm, new DialogInterface.OnClickListener(this) { // from class: r4.WWoڢWWo࢞ڢ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMStopActivity f32989WWWWWWWWWW;

                {
                    this.f32989WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMStopActivity vMStopActivity = this.f32989WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            vMStopActivity.f8505WWWWWWWW.m5097WW(false, true);
                            return;
                        case 1:
                            int i12 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            vMStopActivity.f8505WWWWWWWW.m5097WW(false, false);
                            return;
                        case 2:
                            int i13 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            VMInstance vMInstance2 = vMStopActivity.f8505WWWWWWWW;
                            vMInstance2.m5098WoWo().post(new com.android.vmcore.WWWWoWWWWo(vMInstance2, 4));
                            return;
                        case 3:
                            int i14 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            VMInstance vMInstance3 = vMStopActivity.f8505WWWWWWWW;
                            vMInstance3.m5098WoWo().post(new com.android.vmcore.WWWWoWWWWo(vMInstance3, 4));
                            return;
                        default:
                            int i15 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            vMStopActivity.f8505WWWWWWWW.m5097WW(true, false);
                            return;
                    }
                }
            });
            wWWWoWWWWo4.m13646WWoWWo(R.string.dialog_button_cancel, new n2.WWWWWWWW(10));
            ((C0791WWWWWWWW) wWWWoWWWWo4.f1045WWWWWWWW).f3562WWoWWo = new DialogInterface.OnCancelListener(this) { // from class: r4.WWWWܬWWWWೖܬ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMStopActivity f32977WWWWWWWWWW;

                {
                    this.f32977WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnCancelListener
                public final void onCancel(DialogInterface dialogInterface) {
                    VMStopActivity vMStopActivity = this.f32977WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i10 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        case 1:
                            int i11 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        case 2:
                            int i12 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        case 3:
                            int i13 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        default:
                            int i14 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                    }
                }
            };
            WWWW mo742WWWW4 = wWWWoWWWWo4.mo742WWWW();
            mo742WWWW4.show();
            if (this.f8790WWWWWWWW) {
                m5022WWoWWo(mo742WWWW4.getWindow().getDecorView());
            }
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{104, 10, 37, -78, 71, -72}, new byte[]{ConstantPoolEntry.CP_NameAndType, 111, 73, -41, TarConstants.LF_CHR, -35, 34, -71}).equals(this.f8789WWWWWWWW)) {
            WWWWoWWWWo wWWWoWWWWo5 = new WWWWoWWWWo(this);
            wWWWoWWWWo5.m13648WoWo(R.string.dialog_title_delete);
            wWWWoWWWWo5.m13642WWWWWWWW(R.string.dialog_msg_delete);
            wWWWoWWWWo5.m13645WWWoWWWo(R.string.dialog_button_confirm, new DialogInterface.OnClickListener(this) { // from class: r4.WWoڢWWo࢞ڢ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMStopActivity f32989WWWWWWWWWW;

                {
                    this.f32989WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnClickListener
                public final void onClick(DialogInterface dialogInterface, int i10) {
                    VMStopActivity vMStopActivity = this.f32989WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i11 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            vMStopActivity.f8505WWWWWWWW.m5097WW(false, true);
                            return;
                        case 1:
                            int i12 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            vMStopActivity.f8505WWWWWWWW.m5097WW(false, false);
                            return;
                        case 2:
                            int i13 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            VMInstance vMInstance2 = vMStopActivity.f8505WWWWWWWW;
                            vMInstance2.m5098WoWo().post(new com.android.vmcore.WWWWoWWWWo(vMInstance2, 4));
                            return;
                        case 3:
                            int i14 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            VMInstance vMInstance3 = vMStopActivity.f8505WWWWWWWW;
                            vMInstance3.m5098WoWo().post(new com.android.vmcore.WWWWoWWWWo(vMInstance3, 4));
                            return;
                        default:
                            int i15 = VMStopActivity.f8787WoWo;
                            vMStopActivity.m5026WWWWWWWW();
                            vMStopActivity.f8505WWWWWWWW.m5097WW(true, false);
                            return;
                    }
                }
            });
            wWWWoWWWWo5.m13646WWoWWo(R.string.dialog_button_cancel, new n2.WWWWWWWW(11));
            ((C0791WWWWWWWW) wWWWoWWWWo5.f1045WWWWWWWW).f3562WWoWWo = new DialogInterface.OnCancelListener(this) { // from class: r4.WWWWܬWWWWೖܬ

                /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                public final /* synthetic */ VMStopActivity f32977WWWWWWWWWW;

                {
                    this.f32977WWWWWWWWWW = this;
                }

                @Override // android.content.DialogInterface.OnCancelListener
                public final void onCancel(DialogInterface dialogInterface) {
                    VMStopActivity vMStopActivity = this.f32977WWWWWWWWWW;
                    switch (r2) {
                        case 0:
                            int i10 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        case 1:
                            int i11 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        case 2:
                            int i12 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        case 3:
                            int i13 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                        default:
                            int i14 = VMStopActivity.f8787WoWo;
                            vMStopActivity.finish();
                            return;
                    }
                }
            };
            WWWW mo742WWWW5 = wWWWoWWWWo5.mo742WWWW();
            mo742WWWW5.show();
            if (this.f8790WWWWWWWW) {
                m5022WWoWWo(mo742WWWW5.getWindow().getDecorView());
            }
        } else {
            finish();
        }
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.appcompat.app.AppCompatActivity, androidx.fragment.app.FragmentActivity, android.app.Activity
    public final void onDestroy() {
        super.onDestroy();
        VMInstance vMInstance = this.f8505WWWWWWWW;
        if (vMInstance != null) {
            vMInstance.f8939WWWoWWWo.m13945WWWWWWWW(this);
        }
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.MAIN)
    public void onResetEvent(ResetEvent resetEvent) {
        try {
            WWWW wwww = this.f8788WWWWWWWW;
            if (wwww != null) {
                wwww.dismiss();
            }
        } catch (Throwable th2) {
            th2.printStackTrace();
        }
        if (resetEvent.f9008WWWWWWWW) {
            byte[] bArr = {-76, -98, -117, -47, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -71, 70, -123};
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-48, -5, -25, -76, 19, -36}, bArr).equals(this.f8789WWWWWWWW)) {
                C1644WWWoWWWo.m5313WWWWWWWW(C1644WWWoWWWo.m5312WWWWoWWWWo(-2), new C1628WWWWWWWW(2, this), 0L, null);
            }
            ActivityManager.AppTask m5023WWWWoWWWWo = m5023WWWWoWWWWo();
            if (m5023WWWWoWWWWo != null) {
                m5023WWWWoWWWWo.finishAndRemoveTask();
                m5024WWWWWWWW();
                finish();
                return;
            } else if (!WWWWWWWW.m17835WWWWWWWW(new byte[]{108, 104, -27, 42, 125, -113}, new byte[]{8, 13, -119, 79, 9, -22, -119, -62}).equals(this.f8789WWWWWWWW)) {
                m5027WWWW();
                return;
            } else {
                return;
            }
        }
        m5025WWWWWWWW();
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.MAIN)
    public void onShutdownEvent(ShutdownEvent shutdownEvent) {
        Intent intent;
        try {
            WWWW wwww = this.f8788WWWWWWWW;
            if (wwww != null) {
                wwww.dismiss();
            }
        } catch (Throwable th2) {
            th2.printStackTrace();
        }
        if (shutdownEvent.f9012WWWWWWWW) {
            ActivityManager.AppTask m5023WWWWoWWWWo = m5023WWWWoWWWWo();
            if (shutdownEvent.f9011WWWWoWWWWo) {
                m5024WWWWWWWW();
                if (m5023WWWWoWWWWo != null) {
                    intent = m5023WWWWoWWWWo.getTaskInfo().baseIntent;
                    Intent intent2 = new Intent(intent);
                    intent2.addFlags(32768);
                    startActivity(intent2);
                } else {
                    VMStartActivity0.m5016WWWW(this, this.f8504WWWWWWWW, this.f8791WWWW, false);
                }
                finish();
                return;
            }
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-42, -11, -1, 59, 79, -23}, new byte[]{-92, -112, -113, 90, 38, -101, 67, 104}).equals(this.f8789WWWWWWWW)) {
                m5027WWWW();
                return;
            } else if (m5023WWWWoWWWWo != null) {
                m5023WWWWoWWWWo.finishAndRemoveTask();
                m5024WWWWWWWW();
                finish();
                return;
            } else {
                m5027WWWW();
                return;
            }
        }
        m5025WWWWWWWW();
    }
}
