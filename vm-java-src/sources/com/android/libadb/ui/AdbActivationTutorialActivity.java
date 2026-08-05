package com.android.libadb.ui;

import android.app.AppOpsManager;
import android.content.Intent;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Process;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;
import androidx.appcompat.app.C0791WWWWWWWW;
import androidx.appcompat.app.WWWW;
import com.android.libadb.ui.AdbActivationTutorialActivity;
import com.android.libadb.ui.AdbPairingService;
import com.android.libadb.ui.base.AppBarActivity;
import com.blankj.utilcode.util.AbstractC1631WWWWWWWW;
import com.clone.android.dual.space.R;
import com.google.android.material.button.MaterialButton;
import d2.AbstractC2285WWWWWWWW;
import i6.C2899WWWWWWWW;
import l2.C3365WWWWWWWW;
import n2.C3534WWWWWWWW;
import o2.WoWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p020WWWWWWWW.AbstractC0211WWWWWWWW;
import p2.WWWWoWWWWo;
import ta.C4248WWWoWWWo;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class AdbActivationTutorialActivity extends AppBarActivity {

    /* renamed from: Wo᪅Woै᪅  reason: contains not printable characters */
    public static final /* synthetic */ int f8269WoWo = 0;

    /* renamed from: WWWWoᜑWWWWoเᜑ  reason: contains not printable characters */
    public WWWWoWWWWo f8270WWWWoWWWWo;

    /* renamed from: WWWWᜐWWWWଙᜐ  reason: contains not printable characters */
    public Handler f8271WWWWWWWW;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public MaterialButton f8272WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public MaterialButton f8273WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public View f8274WWWWWWWW;

    /* renamed from: WWWoᜒWWWo೧ᜒ  reason: contains not printable characters */
    public MaterialButton f8275WWWoWWWo;

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public WWWW f8276WWWW;

    @Override // com.android.libadb.ui.base.AppBarActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_adb_activation_tutorial);
        m2307WWoWWo().mo2341WoWo(true);
        MaterialButton materialButton = (MaterialButton) findViewById(R.id.wifi_options_btn);
        this.f8275WWWoWWWo = materialButton;
        materialButton.setOnClickListener(new View.OnClickListener(this) { // from class: p2.WWWȏWWWoನ̑

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbActivationTutorialActivity f32190WWWWWWWWWW;

            {
                this.f32190WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                int noteOpNoThrow;
                AdbActivationTutorialActivity adbActivationTutorialActivity = this.f32190WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        int i10 = AdbActivationTutorialActivity.f8269WoWo;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            try {
                                C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                adbActivationTutorialActivity.startActivity(new Intent(WWWWWWWW.m17835WWWWWWWW(new byte[]{-111, -5, -115, 90, 56, 47, -53, -70, -125, -16, -99, 92, 62, 40, -56, -25, -34, -62, -96, 110, 30, 25, -4, -47, -92, -63, -96, 102, 16, 21}, new byte[]{-16, -107, -23, 40, 87, 70, -81, -108})));
                                return;
                            } catch (Exception unused) {
                                return;
                            }
                        }
                        C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_connected, 0).m17237WWWWWWWW();
                        return;
                    case 1:
                        int i11 = AdbActivationTutorialActivity.f8269WoWo;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_disconnected, 0).m17237WWWWWWWW();
                            return;
                        } else if (!WoWo.m16212WWWoWWWo().f31586WWWWWWWWWW && s2.WWWWWWWW.m16902WWWWWWWW() == -1) {
                            if (Build.VERSION.SDK_INT >= 33) {
                                C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                if (AbstractC0211WWWWWWWW.m824WWWWWWWW(adbActivationTutorialActivity, WWWWWWWW.m17835WWWWWWWW(new byte[]{-120, -76, 101, -16, -57, 29, 123, 107, -103, -65, 115, -17, -63, 7, 108, 44, -122, -76, 47, -46, -25, 39, TarConstants.LF_GNUTYPE_LONGLINK, 26, -89, -107, 85, -53, -18, 61, 92, 4, -67, -109, 78, -52, -5}, new byte[]{-23, -38, 1, -126, -88, 116, 31, 69})) != 0) {
                                    da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(adbActivationTutorialActivity);
                                    wWWWoWWWWo.m13648WoWo(R.string.dialog_msg_activation_title);
                                    wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_allow_notification_tips);
                                    wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_msg_allow_notification_now, new e4.WWWWoWWWWo(5, adbActivationTutorialActivity));
                                    ((C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW).f3567WoWo = false;
                                    wWWWoWWWWo.m741WWoWWo();
                                    return;
                                }
                            }
                            byte[] bArr = {TarConstants.LF_LINK, -127, 27, -45, 90, 65, 118, -65, 33, -116, 35, -61, 95, 86, TarConstants.LF_GNUTYPE_LONGNAME, -78, 32, -99, 15};
                            byte[] bArr2 = {69, -18, 124, -76, TarConstants.LF_FIFO, 36, 41, -34};
                            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                            s2.WWWWoWWWWo.m16899WWWWoWWWWo(adbActivationTutorialActivity, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                            AdbPairingService.f8284WWWWWWWW.getClass();
                            Intent m15676WWWW = C3365WWWWWWWW.m15676WWWW(adbActivationTutorialActivity);
                            try {
                                adbActivationTutorialActivity.startForegroundService(m15676WWWW);
                                return;
                            } catch (Throwable th2) {
                                if (Build.VERSION.SDK_INT >= 31 && AbstractC2285WWWWWWWW.m13602WWWW(th2)) {
                                    byte[] bArr3 = {104, 18, -3, 68, -1, TarConstants.LF_GNUTYPE_LONGLINK, 44, 59};
                                    C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                    noteOpNoThrow = ((AppOpsManager) adbActivationTutorialActivity.getSystemService(WWWWWWWW.m17835WWWWWWWW(new byte[]{9, 98, -115, 43, -113, 56}, bArr3))).noteOpNoThrow(WWWWWWWW.m17835WWWWWWWW(new byte[]{40, 58, -114, -104, -67, 84, 78, 80, 58, 32, -117, -104, -90, 98, TarConstants.LF_GNUTYPE_LONGNAME, 5, 59, TarConstants.LF_LINK, -115, -104, -67, 72, 68, 14}, new byte[]{73, 84, -22, -22, -46, 61, 42, 106}), Process.myUid(), adbActivationTutorialActivity.getPackageName(), null, null);
                                    if (noteOpNoThrow == 2) {
                                        C4248WWWoWWWo.m17236WWWoWWWo(adbActivationTutorialActivity.f8275WWWoWWWo, WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -86, 101, -122, 5, 14, -62, 60, Byte.MIN_VALUE, -68, 117, -121, 20, 8, -62, 39, -118, -76, 126, -11, 56, 60, -80, ConstantPoolEntry.CP_NameAndType, -70, -108, TarConstants.LF_GNUTYPE_SPARSE, -80, TarConstants.LF_DIR, 97, -80, 63, -73, -101, 78, -11, TarConstants.LF_NORMAL, 61, -11, 72, -90, -107, 79, -11, TarConstants.LF_DIR, 32, -7, 6, -72, -59}, new byte[]{-33, -6, 58, -43, 81, 79, -112, 104}), 0).m17237WWWWWWWW();
                                    }
                                    adbActivationTutorialActivity.startService(m15676WWWW);
                                    return;
                                }
                                return;
                            }
                        } else {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_paired, 0).m17237WWWWWWWW();
                            return;
                        }
                    case 2:
                        int i12 = AdbActivationTutorialActivity.f8269WoWo;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_disconnected, 0).m17237WWWWWWWW();
                            return;
                        } else if (!s2.WWWWoWWWWo.m16900WWWWWWWW(adbActivationTutorialActivity)) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_developer_closed, 0).m17237WWWWWWWW();
                            return;
                        } else if (!WoWo.m16212WWWoWWWo().f31586WWWWWWWWWW && s2.WWWWWWWW.m16902WWWWWWWW() == -1) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_not_paired, 0).m17237WWWWWWWW();
                            return;
                        } else if (s2.WWWWWWWW.m16901WWWWoWWWWo()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_activated, 0).m17237WWWWWWWW();
                            return;
                        } else {
                            WoWo m16212WWWoWWWo = WoWo.m16212WWWoWWWo();
                            C2899WWWWWWWW c2899wwwwwwww = new C2899WWWWWWWW(16, adbActivationTutorialActivity);
                            if (!m16212WWWoWWWo.f31587WWWWoWWWWo) {
                                m16212WWWoWWWo.f31592WWoWWo = c2899wwwwwwww;
                                int m16902WWWWWWWW = s2.WWWWWWWW.m16902WWWWWWWW();
                                if (m16902WWWWWWWW != -1) {
                                    new Thread(new p021WWWWWWWW.WWWW(m16902WWWWWWWW, 3, m16212WWWoWWWo)).start();
                                } else {
                                    o2.WWWW wwww = new o2.WWWW(m16212WWWoWWWo);
                                    m16212WWWoWWWo.f31590WWWWWWWW = wwww;
                                    wwww.start();
                                    byte[] bArr4 = {-67, 85, -39, 28, -31, 9, 17, TarConstants.LF_DIR, -49, 87, -46, 16, -94, 24, 30, TarConstants.LF_SYMLINK, -52, 107, -55, 29, -68};
                                    byte[] bArr5 = {-30, TarConstants.LF_BLK, -67, 126, -52, 125, 125, 70};
                                    C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                    m16212WWWoWWWo.m16214WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5), m16212WWWoWWWo);
                                }
                            }
                            da.WWWWoWWWWo wWWWoWWWWo2 = new da.WWWWoWWWWo(adbActivationTutorialActivity);
                            View inflate = LayoutInflater.from(adbActivationTutorialActivity).inflate(R.layout.dialog_loading, (ViewGroup) null, false);
                            ((TextView) inflate.findViewById(R.id.message)).setText(R.string.dialog_msg_activation_in_progress);
                            wWWWoWWWWo2.m13644WWWWWWWW(inflate);
                            ((C0791WWWWWWWW) wWWWoWWWWo2.f1045WWWWWWWW).f3567WoWo = false;
                            WWWW mo742WWWW = wWWWoWWWWo2.mo742WWWW();
                            adbActivationTutorialActivity.f8276WWWW = mo742WWWW;
                            mo742WWWW.show();
                            return;
                        }
                    default:
                        int i13 = AdbActivationTutorialActivity.f8269WoWo;
                        da.WWWWoWWWWo wWWWoWWWWo3 = new da.WWWWoWWWWo(adbActivationTutorialActivity);
                        wWWWoWWWWo3.m13648WoWo(R.string.adb_activation_tutorial_developer_dlg_title);
                        wWWWoWWWWo3.m13642WWWWWWWW(R.string.adb_activation_tutorial_developer_dlg_msg);
                        wWWWoWWWWo3.m13645WWWoWWWo(R.string.adb_activation_tutorial_dlg_i_known, null);
                        wWWWoWWWWo3.m741WWoWWo();
                        return;
                }
            }
        });
        ((MaterialButton) findViewById(R.id.dev_options_btn)).setIcon(getDrawable(R.drawable.ic_adb_okay));
        findViewById(R.id.dev_options_help_btn).setOnClickListener(new View.OnClickListener(this) { // from class: p2.WWWȏWWWoನ̑

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbActivationTutorialActivity f32190WWWWWWWWWW;

            {
                this.f32190WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                int noteOpNoThrow;
                AdbActivationTutorialActivity adbActivationTutorialActivity = this.f32190WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        int i10 = AdbActivationTutorialActivity.f8269WoWo;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            try {
                                C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                adbActivationTutorialActivity.startActivity(new Intent(WWWWWWWW.m17835WWWWWWWW(new byte[]{-111, -5, -115, 90, 56, 47, -53, -70, -125, -16, -99, 92, 62, 40, -56, -25, -34, -62, -96, 110, 30, 25, -4, -47, -92, -63, -96, 102, 16, 21}, new byte[]{-16, -107, -23, 40, 87, 70, -81, -108})));
                                return;
                            } catch (Exception unused) {
                                return;
                            }
                        }
                        C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_connected, 0).m17237WWWWWWWW();
                        return;
                    case 1:
                        int i11 = AdbActivationTutorialActivity.f8269WoWo;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_disconnected, 0).m17237WWWWWWWW();
                            return;
                        } else if (!WoWo.m16212WWWoWWWo().f31586WWWWWWWWWW && s2.WWWWWWWW.m16902WWWWWWWW() == -1) {
                            if (Build.VERSION.SDK_INT >= 33) {
                                C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                if (AbstractC0211WWWWWWWW.m824WWWWWWWW(adbActivationTutorialActivity, WWWWWWWW.m17835WWWWWWWW(new byte[]{-120, -76, 101, -16, -57, 29, 123, 107, -103, -65, 115, -17, -63, 7, 108, 44, -122, -76, 47, -46, -25, 39, TarConstants.LF_GNUTYPE_LONGLINK, 26, -89, -107, 85, -53, -18, 61, 92, 4, -67, -109, 78, -52, -5}, new byte[]{-23, -38, 1, -126, -88, 116, 31, 69})) != 0) {
                                    da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(adbActivationTutorialActivity);
                                    wWWWoWWWWo.m13648WoWo(R.string.dialog_msg_activation_title);
                                    wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_allow_notification_tips);
                                    wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_msg_allow_notification_now, new e4.WWWWoWWWWo(5, adbActivationTutorialActivity));
                                    ((C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW).f3567WoWo = false;
                                    wWWWoWWWWo.m741WWoWWo();
                                    return;
                                }
                            }
                            byte[] bArr = {TarConstants.LF_LINK, -127, 27, -45, 90, 65, 118, -65, 33, -116, 35, -61, 95, 86, TarConstants.LF_GNUTYPE_LONGNAME, -78, 32, -99, 15};
                            byte[] bArr2 = {69, -18, 124, -76, TarConstants.LF_FIFO, 36, 41, -34};
                            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                            s2.WWWWoWWWWo.m16899WWWWoWWWWo(adbActivationTutorialActivity, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                            AdbPairingService.f8284WWWWWWWW.getClass();
                            Intent m15676WWWW = C3365WWWWWWWW.m15676WWWW(adbActivationTutorialActivity);
                            try {
                                adbActivationTutorialActivity.startForegroundService(m15676WWWW);
                                return;
                            } catch (Throwable th2) {
                                if (Build.VERSION.SDK_INT >= 31 && AbstractC2285WWWWWWWW.m13602WWWW(th2)) {
                                    byte[] bArr3 = {104, 18, -3, 68, -1, TarConstants.LF_GNUTYPE_LONGLINK, 44, 59};
                                    C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                    noteOpNoThrow = ((AppOpsManager) adbActivationTutorialActivity.getSystemService(WWWWWWWW.m17835WWWWWWWW(new byte[]{9, 98, -115, 43, -113, 56}, bArr3))).noteOpNoThrow(WWWWWWWW.m17835WWWWWWWW(new byte[]{40, 58, -114, -104, -67, 84, 78, 80, 58, 32, -117, -104, -90, 98, TarConstants.LF_GNUTYPE_LONGNAME, 5, 59, TarConstants.LF_LINK, -115, -104, -67, 72, 68, 14}, new byte[]{73, 84, -22, -22, -46, 61, 42, 106}), Process.myUid(), adbActivationTutorialActivity.getPackageName(), null, null);
                                    if (noteOpNoThrow == 2) {
                                        C4248WWWoWWWo.m17236WWWoWWWo(adbActivationTutorialActivity.f8275WWWoWWWo, WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -86, 101, -122, 5, 14, -62, 60, Byte.MIN_VALUE, -68, 117, -121, 20, 8, -62, 39, -118, -76, 126, -11, 56, 60, -80, ConstantPoolEntry.CP_NameAndType, -70, -108, TarConstants.LF_GNUTYPE_SPARSE, -80, TarConstants.LF_DIR, 97, -80, 63, -73, -101, 78, -11, TarConstants.LF_NORMAL, 61, -11, 72, -90, -107, 79, -11, TarConstants.LF_DIR, 32, -7, 6, -72, -59}, new byte[]{-33, -6, 58, -43, 81, 79, -112, 104}), 0).m17237WWWWWWWW();
                                    }
                                    adbActivationTutorialActivity.startService(m15676WWWW);
                                    return;
                                }
                                return;
                            }
                        } else {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_paired, 0).m17237WWWWWWWW();
                            return;
                        }
                    case 2:
                        int i12 = AdbActivationTutorialActivity.f8269WoWo;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_disconnected, 0).m17237WWWWWWWW();
                            return;
                        } else if (!s2.WWWWoWWWWo.m16900WWWWWWWW(adbActivationTutorialActivity)) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_developer_closed, 0).m17237WWWWWWWW();
                            return;
                        } else if (!WoWo.m16212WWWoWWWo().f31586WWWWWWWWWW && s2.WWWWWWWW.m16902WWWWWWWW() == -1) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_not_paired, 0).m17237WWWWWWWW();
                            return;
                        } else if (s2.WWWWWWWW.m16901WWWWoWWWWo()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_activated, 0).m17237WWWWWWWW();
                            return;
                        } else {
                            WoWo m16212WWWoWWWo = WoWo.m16212WWWoWWWo();
                            C2899WWWWWWWW c2899wwwwwwww = new C2899WWWWWWWW(16, adbActivationTutorialActivity);
                            if (!m16212WWWoWWWo.f31587WWWWoWWWWo) {
                                m16212WWWoWWWo.f31592WWoWWo = c2899wwwwwwww;
                                int m16902WWWWWWWW = s2.WWWWWWWW.m16902WWWWWWWW();
                                if (m16902WWWWWWWW != -1) {
                                    new Thread(new p021WWWWWWWW.WWWW(m16902WWWWWWWW, 3, m16212WWWoWWWo)).start();
                                } else {
                                    o2.WWWW wwww = new o2.WWWW(m16212WWWoWWWo);
                                    m16212WWWoWWWo.f31590WWWWWWWW = wwww;
                                    wwww.start();
                                    byte[] bArr4 = {-67, 85, -39, 28, -31, 9, 17, TarConstants.LF_DIR, -49, 87, -46, 16, -94, 24, 30, TarConstants.LF_SYMLINK, -52, 107, -55, 29, -68};
                                    byte[] bArr5 = {-30, TarConstants.LF_BLK, -67, 126, -52, 125, 125, 70};
                                    C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                    m16212WWWoWWWo.m16214WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5), m16212WWWoWWWo);
                                }
                            }
                            da.WWWWoWWWWo wWWWoWWWWo2 = new da.WWWWoWWWWo(adbActivationTutorialActivity);
                            View inflate = LayoutInflater.from(adbActivationTutorialActivity).inflate(R.layout.dialog_loading, (ViewGroup) null, false);
                            ((TextView) inflate.findViewById(R.id.message)).setText(R.string.dialog_msg_activation_in_progress);
                            wWWWoWWWWo2.m13644WWWWWWWW(inflate);
                            ((C0791WWWWWWWW) wWWWoWWWWo2.f1045WWWWWWWW).f3567WoWo = false;
                            WWWW mo742WWWW = wWWWoWWWWo2.mo742WWWW();
                            adbActivationTutorialActivity.f8276WWWW = mo742WWWW;
                            mo742WWWW.show();
                            return;
                        }
                    default:
                        int i13 = AdbActivationTutorialActivity.f8269WoWo;
                        da.WWWWoWWWWo wWWWoWWWWo3 = new da.WWWWoWWWWo(adbActivationTutorialActivity);
                        wWWWoWWWWo3.m13648WoWo(R.string.adb_activation_tutorial_developer_dlg_title);
                        wWWWoWWWWo3.m13642WWWWWWWW(R.string.adb_activation_tutorial_developer_dlg_msg);
                        wWWWoWWWWo3.m13645WWWoWWWo(R.string.adb_activation_tutorial_dlg_i_known, null);
                        wWWWoWWWWo3.m741WWoWWo();
                        return;
                }
            }
        });
        MaterialButton materialButton2 = (MaterialButton) findViewById(R.id.wifi_pair_options_btn);
        this.f8272WWWWWWWW = materialButton2;
        materialButton2.setOnClickListener(new View.OnClickListener(this) { // from class: p2.WWWȏWWWoನ̑

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbActivationTutorialActivity f32190WWWWWWWWWW;

            {
                this.f32190WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                int noteOpNoThrow;
                AdbActivationTutorialActivity adbActivationTutorialActivity = this.f32190WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        int i10 = AdbActivationTutorialActivity.f8269WoWo;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            try {
                                C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                adbActivationTutorialActivity.startActivity(new Intent(WWWWWWWW.m17835WWWWWWWW(new byte[]{-111, -5, -115, 90, 56, 47, -53, -70, -125, -16, -99, 92, 62, 40, -56, -25, -34, -62, -96, 110, 30, 25, -4, -47, -92, -63, -96, 102, 16, 21}, new byte[]{-16, -107, -23, 40, 87, 70, -81, -108})));
                                return;
                            } catch (Exception unused) {
                                return;
                            }
                        }
                        C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_connected, 0).m17237WWWWWWWW();
                        return;
                    case 1:
                        int i11 = AdbActivationTutorialActivity.f8269WoWo;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_disconnected, 0).m17237WWWWWWWW();
                            return;
                        } else if (!WoWo.m16212WWWoWWWo().f31586WWWWWWWWWW && s2.WWWWWWWW.m16902WWWWWWWW() == -1) {
                            if (Build.VERSION.SDK_INT >= 33) {
                                C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                if (AbstractC0211WWWWWWWW.m824WWWWWWWW(adbActivationTutorialActivity, WWWWWWWW.m17835WWWWWWWW(new byte[]{-120, -76, 101, -16, -57, 29, 123, 107, -103, -65, 115, -17, -63, 7, 108, 44, -122, -76, 47, -46, -25, 39, TarConstants.LF_GNUTYPE_LONGLINK, 26, -89, -107, 85, -53, -18, 61, 92, 4, -67, -109, 78, -52, -5}, new byte[]{-23, -38, 1, -126, -88, 116, 31, 69})) != 0) {
                                    da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(adbActivationTutorialActivity);
                                    wWWWoWWWWo.m13648WoWo(R.string.dialog_msg_activation_title);
                                    wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_allow_notification_tips);
                                    wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_msg_allow_notification_now, new e4.WWWWoWWWWo(5, adbActivationTutorialActivity));
                                    ((C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW).f3567WoWo = false;
                                    wWWWoWWWWo.m741WWoWWo();
                                    return;
                                }
                            }
                            byte[] bArr = {TarConstants.LF_LINK, -127, 27, -45, 90, 65, 118, -65, 33, -116, 35, -61, 95, 86, TarConstants.LF_GNUTYPE_LONGNAME, -78, 32, -99, 15};
                            byte[] bArr2 = {69, -18, 124, -76, TarConstants.LF_FIFO, 36, 41, -34};
                            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                            s2.WWWWoWWWWo.m16899WWWWoWWWWo(adbActivationTutorialActivity, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                            AdbPairingService.f8284WWWWWWWW.getClass();
                            Intent m15676WWWW = C3365WWWWWWWW.m15676WWWW(adbActivationTutorialActivity);
                            try {
                                adbActivationTutorialActivity.startForegroundService(m15676WWWW);
                                return;
                            } catch (Throwable th2) {
                                if (Build.VERSION.SDK_INT >= 31 && AbstractC2285WWWWWWWW.m13602WWWW(th2)) {
                                    byte[] bArr3 = {104, 18, -3, 68, -1, TarConstants.LF_GNUTYPE_LONGLINK, 44, 59};
                                    C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                    noteOpNoThrow = ((AppOpsManager) adbActivationTutorialActivity.getSystemService(WWWWWWWW.m17835WWWWWWWW(new byte[]{9, 98, -115, 43, -113, 56}, bArr3))).noteOpNoThrow(WWWWWWWW.m17835WWWWWWWW(new byte[]{40, 58, -114, -104, -67, 84, 78, 80, 58, 32, -117, -104, -90, 98, TarConstants.LF_GNUTYPE_LONGNAME, 5, 59, TarConstants.LF_LINK, -115, -104, -67, 72, 68, 14}, new byte[]{73, 84, -22, -22, -46, 61, 42, 106}), Process.myUid(), adbActivationTutorialActivity.getPackageName(), null, null);
                                    if (noteOpNoThrow == 2) {
                                        C4248WWWoWWWo.m17236WWWoWWWo(adbActivationTutorialActivity.f8275WWWoWWWo, WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -86, 101, -122, 5, 14, -62, 60, Byte.MIN_VALUE, -68, 117, -121, 20, 8, -62, 39, -118, -76, 126, -11, 56, 60, -80, ConstantPoolEntry.CP_NameAndType, -70, -108, TarConstants.LF_GNUTYPE_SPARSE, -80, TarConstants.LF_DIR, 97, -80, 63, -73, -101, 78, -11, TarConstants.LF_NORMAL, 61, -11, 72, -90, -107, 79, -11, TarConstants.LF_DIR, 32, -7, 6, -72, -59}, new byte[]{-33, -6, 58, -43, 81, 79, -112, 104}), 0).m17237WWWWWWWW();
                                    }
                                    adbActivationTutorialActivity.startService(m15676WWWW);
                                    return;
                                }
                                return;
                            }
                        } else {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_paired, 0).m17237WWWWWWWW();
                            return;
                        }
                    case 2:
                        int i12 = AdbActivationTutorialActivity.f8269WoWo;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_disconnected, 0).m17237WWWWWWWW();
                            return;
                        } else if (!s2.WWWWoWWWWo.m16900WWWWWWWW(adbActivationTutorialActivity)) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_developer_closed, 0).m17237WWWWWWWW();
                            return;
                        } else if (!WoWo.m16212WWWoWWWo().f31586WWWWWWWWWW && s2.WWWWWWWW.m16902WWWWWWWW() == -1) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_not_paired, 0).m17237WWWWWWWW();
                            return;
                        } else if (s2.WWWWWWWW.m16901WWWWoWWWWo()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_activated, 0).m17237WWWWWWWW();
                            return;
                        } else {
                            WoWo m16212WWWoWWWo = WoWo.m16212WWWoWWWo();
                            C2899WWWWWWWW c2899wwwwwwww = new C2899WWWWWWWW(16, adbActivationTutorialActivity);
                            if (!m16212WWWoWWWo.f31587WWWWoWWWWo) {
                                m16212WWWoWWWo.f31592WWoWWo = c2899wwwwwwww;
                                int m16902WWWWWWWW = s2.WWWWWWWW.m16902WWWWWWWW();
                                if (m16902WWWWWWWW != -1) {
                                    new Thread(new p021WWWWWWWW.WWWW(m16902WWWWWWWW, 3, m16212WWWoWWWo)).start();
                                } else {
                                    o2.WWWW wwww = new o2.WWWW(m16212WWWoWWWo);
                                    m16212WWWoWWWo.f31590WWWWWWWW = wwww;
                                    wwww.start();
                                    byte[] bArr4 = {-67, 85, -39, 28, -31, 9, 17, TarConstants.LF_DIR, -49, 87, -46, 16, -94, 24, 30, TarConstants.LF_SYMLINK, -52, 107, -55, 29, -68};
                                    byte[] bArr5 = {-30, TarConstants.LF_BLK, -67, 126, -52, 125, 125, 70};
                                    C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                    m16212WWWoWWWo.m16214WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5), m16212WWWoWWWo);
                                }
                            }
                            da.WWWWoWWWWo wWWWoWWWWo2 = new da.WWWWoWWWWo(adbActivationTutorialActivity);
                            View inflate = LayoutInflater.from(adbActivationTutorialActivity).inflate(R.layout.dialog_loading, (ViewGroup) null, false);
                            ((TextView) inflate.findViewById(R.id.message)).setText(R.string.dialog_msg_activation_in_progress);
                            wWWWoWWWWo2.m13644WWWWWWWW(inflate);
                            ((C0791WWWWWWWW) wWWWoWWWWo2.f1045WWWWWWWW).f3567WoWo = false;
                            WWWW mo742WWWW = wWWWoWWWWo2.mo742WWWW();
                            adbActivationTutorialActivity.f8276WWWW = mo742WWWW;
                            mo742WWWW.show();
                            return;
                        }
                    default:
                        int i13 = AdbActivationTutorialActivity.f8269WoWo;
                        da.WWWWoWWWWo wWWWoWWWWo3 = new da.WWWWoWWWWo(adbActivationTutorialActivity);
                        wWWWoWWWWo3.m13648WoWo(R.string.adb_activation_tutorial_developer_dlg_title);
                        wWWWoWWWWo3.m13642WWWWWWWW(R.string.adb_activation_tutorial_developer_dlg_msg);
                        wWWWoWWWWo3.m13645WWWoWWWo(R.string.adb_activation_tutorial_dlg_i_known, null);
                        wWWWoWWWWo3.m741WWoWWo();
                        return;
                }
            }
        });
        MaterialButton materialButton3 = (MaterialButton) findViewById(R.id.activation_btn);
        this.f8273WWWWWWWW = materialButton3;
        materialButton3.setOnClickListener(new View.OnClickListener(this) { // from class: p2.WWWȏWWWoನ̑

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbActivationTutorialActivity f32190WWWWWWWWWW;

            {
                this.f32190WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                int noteOpNoThrow;
                AdbActivationTutorialActivity adbActivationTutorialActivity = this.f32190WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        int i10 = AdbActivationTutorialActivity.f8269WoWo;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            try {
                                C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                adbActivationTutorialActivity.startActivity(new Intent(WWWWWWWW.m17835WWWWWWWW(new byte[]{-111, -5, -115, 90, 56, 47, -53, -70, -125, -16, -99, 92, 62, 40, -56, -25, -34, -62, -96, 110, 30, 25, -4, -47, -92, -63, -96, 102, 16, 21}, new byte[]{-16, -107, -23, 40, 87, 70, -81, -108})));
                                return;
                            } catch (Exception unused) {
                                return;
                            }
                        }
                        C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_connected, 0).m17237WWWWWWWW();
                        return;
                    case 1:
                        int i11 = AdbActivationTutorialActivity.f8269WoWo;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_disconnected, 0).m17237WWWWWWWW();
                            return;
                        } else if (!WoWo.m16212WWWoWWWo().f31586WWWWWWWWWW && s2.WWWWWWWW.m16902WWWWWWWW() == -1) {
                            if (Build.VERSION.SDK_INT >= 33) {
                                C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                if (AbstractC0211WWWWWWWW.m824WWWWWWWW(adbActivationTutorialActivity, WWWWWWWW.m17835WWWWWWWW(new byte[]{-120, -76, 101, -16, -57, 29, 123, 107, -103, -65, 115, -17, -63, 7, 108, 44, -122, -76, 47, -46, -25, 39, TarConstants.LF_GNUTYPE_LONGLINK, 26, -89, -107, 85, -53, -18, 61, 92, 4, -67, -109, 78, -52, -5}, new byte[]{-23, -38, 1, -126, -88, 116, 31, 69})) != 0) {
                                    da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(adbActivationTutorialActivity);
                                    wWWWoWWWWo.m13648WoWo(R.string.dialog_msg_activation_title);
                                    wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_allow_notification_tips);
                                    wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_msg_allow_notification_now, new e4.WWWWoWWWWo(5, adbActivationTutorialActivity));
                                    ((C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW).f3567WoWo = false;
                                    wWWWoWWWWo.m741WWoWWo();
                                    return;
                                }
                            }
                            byte[] bArr = {TarConstants.LF_LINK, -127, 27, -45, 90, 65, 118, -65, 33, -116, 35, -61, 95, 86, TarConstants.LF_GNUTYPE_LONGNAME, -78, 32, -99, 15};
                            byte[] bArr2 = {69, -18, 124, -76, TarConstants.LF_FIFO, 36, 41, -34};
                            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                            s2.WWWWoWWWWo.m16899WWWWoWWWWo(adbActivationTutorialActivity, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                            AdbPairingService.f8284WWWWWWWW.getClass();
                            Intent m15676WWWW = C3365WWWWWWWW.m15676WWWW(adbActivationTutorialActivity);
                            try {
                                adbActivationTutorialActivity.startForegroundService(m15676WWWW);
                                return;
                            } catch (Throwable th2) {
                                if (Build.VERSION.SDK_INT >= 31 && AbstractC2285WWWWWWWW.m13602WWWW(th2)) {
                                    byte[] bArr3 = {104, 18, -3, 68, -1, TarConstants.LF_GNUTYPE_LONGLINK, 44, 59};
                                    C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                    noteOpNoThrow = ((AppOpsManager) adbActivationTutorialActivity.getSystemService(WWWWWWWW.m17835WWWWWWWW(new byte[]{9, 98, -115, 43, -113, 56}, bArr3))).noteOpNoThrow(WWWWWWWW.m17835WWWWWWWW(new byte[]{40, 58, -114, -104, -67, 84, 78, 80, 58, 32, -117, -104, -90, 98, TarConstants.LF_GNUTYPE_LONGNAME, 5, 59, TarConstants.LF_LINK, -115, -104, -67, 72, 68, 14}, new byte[]{73, 84, -22, -22, -46, 61, 42, 106}), Process.myUid(), adbActivationTutorialActivity.getPackageName(), null, null);
                                    if (noteOpNoThrow == 2) {
                                        C4248WWWoWWWo.m17236WWWoWWWo(adbActivationTutorialActivity.f8275WWWoWWWo, WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -86, 101, -122, 5, 14, -62, 60, Byte.MIN_VALUE, -68, 117, -121, 20, 8, -62, 39, -118, -76, 126, -11, 56, 60, -80, ConstantPoolEntry.CP_NameAndType, -70, -108, TarConstants.LF_GNUTYPE_SPARSE, -80, TarConstants.LF_DIR, 97, -80, 63, -73, -101, 78, -11, TarConstants.LF_NORMAL, 61, -11, 72, -90, -107, 79, -11, TarConstants.LF_DIR, 32, -7, 6, -72, -59}, new byte[]{-33, -6, 58, -43, 81, 79, -112, 104}), 0).m17237WWWWWWWW();
                                    }
                                    adbActivationTutorialActivity.startService(m15676WWWW);
                                    return;
                                }
                                return;
                            }
                        } else {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_paired, 0).m17237WWWWWWWW();
                            return;
                        }
                    case 2:
                        int i12 = AdbActivationTutorialActivity.f8269WoWo;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_disconnected, 0).m17237WWWWWWWW();
                            return;
                        } else if (!s2.WWWWoWWWWo.m16900WWWWWWWW(adbActivationTutorialActivity)) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_developer_closed, 0).m17237WWWWWWWW();
                            return;
                        } else if (!WoWo.m16212WWWoWWWo().f31586WWWWWWWWWW && s2.WWWWWWWW.m16902WWWWWWWW() == -1) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_not_paired, 0).m17237WWWWWWWW();
                            return;
                        } else if (s2.WWWWWWWW.m16901WWWWoWWWWo()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_activated, 0).m17237WWWWWWWW();
                            return;
                        } else {
                            WoWo m16212WWWoWWWo = WoWo.m16212WWWoWWWo();
                            C2899WWWWWWWW c2899wwwwwwww = new C2899WWWWWWWW(16, adbActivationTutorialActivity);
                            if (!m16212WWWoWWWo.f31587WWWWoWWWWo) {
                                m16212WWWoWWWo.f31592WWoWWo = c2899wwwwwwww;
                                int m16902WWWWWWWW = s2.WWWWWWWW.m16902WWWWWWWW();
                                if (m16902WWWWWWWW != -1) {
                                    new Thread(new p021WWWWWWWW.WWWW(m16902WWWWWWWW, 3, m16212WWWoWWWo)).start();
                                } else {
                                    o2.WWWW wwww = new o2.WWWW(m16212WWWoWWWo);
                                    m16212WWWoWWWo.f31590WWWWWWWW = wwww;
                                    wwww.start();
                                    byte[] bArr4 = {-67, 85, -39, 28, -31, 9, 17, TarConstants.LF_DIR, -49, 87, -46, 16, -94, 24, 30, TarConstants.LF_SYMLINK, -52, 107, -55, 29, -68};
                                    byte[] bArr5 = {-30, TarConstants.LF_BLK, -67, 126, -52, 125, 125, 70};
                                    C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                    m16212WWWoWWWo.m16214WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5), m16212WWWoWWWo);
                                }
                            }
                            da.WWWWoWWWWo wWWWoWWWWo2 = new da.WWWWoWWWWo(adbActivationTutorialActivity);
                            View inflate = LayoutInflater.from(adbActivationTutorialActivity).inflate(R.layout.dialog_loading, (ViewGroup) null, false);
                            ((TextView) inflate.findViewById(R.id.message)).setText(R.string.dialog_msg_activation_in_progress);
                            wWWWoWWWWo2.m13644WWWWWWWW(inflate);
                            ((C0791WWWWWWWW) wWWWoWWWWo2.f1045WWWWWWWW).f3567WoWo = false;
                            WWWW mo742WWWW = wWWWoWWWWo2.mo742WWWW();
                            adbActivationTutorialActivity.f8276WWWW = mo742WWWW;
                            mo742WWWW.show();
                            return;
                        }
                    default:
                        int i13 = AdbActivationTutorialActivity.f8269WoWo;
                        da.WWWWoWWWWo wWWWoWWWWo3 = new da.WWWWoWWWWo(adbActivationTutorialActivity);
                        wWWWoWWWWo3.m13648WoWo(R.string.adb_activation_tutorial_developer_dlg_title);
                        wWWWoWWWWo3.m13642WWWWWWWW(R.string.adb_activation_tutorial_developer_dlg_msg);
                        wWWWoWWWWo3.m13645WWWoWWWo(R.string.adb_activation_tutorial_dlg_i_known, null);
                        wWWWoWWWWo3.m741WWoWWo();
                        return;
                }
            }
        });
        if (com.blankj.utilcode.util.WWWW.f9384WWWWWWWW[0].equals(com.blankj.utilcode.util.WWWW.m5348WoWo().f9367WWWWWWWW)) {
            View findViewById = findViewById(R.id.miui);
            this.f8274WWWWWWWW = findViewById;
            findViewById.setVisibility(0);
        }
        Handler handler = new Handler();
        this.f8271WWWWWWWW = handler;
        WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this, 1);
        this.f8270WWWWoWWWWo = wWWWoWWWWo;
        handler.postDelayed(wWWWoWWWWo, 500L);
    }

    @Override // androidx.appcompat.app.AppCompatActivity, androidx.fragment.app.FragmentActivity, android.app.Activity
    public final void onDestroy() {
        super.onDestroy();
        this.f8271WWWWWWWW.removeCallbacks(this.f8270WWWWoWWWWo);
        WoWo m16212WWWoWWWo = WoWo.m16212WWWoWWWo();
        o2.WWWWWWWW wwwwwwww = m16212WWWoWWWo.f31589WWWWWWWW;
        if (wwwwwwww != null) {
            try {
                wwwwwwww.close();
            } catch (Throwable unused) {
            }
            m16212WWWoWWWo.f31589WWWWWWWW = null;
        }
    }
}
