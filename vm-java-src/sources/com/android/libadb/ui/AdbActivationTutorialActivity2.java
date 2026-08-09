package com.android.libadb.ui;

import android.content.Intent;
import android.os.Bundle;
import android.os.Handler;
import android.view.View;
import com.android.libadb.ui.AdbActivationTutorialActivity2;
import com.android.libadb.ui.base.AppBarActivity;
import com.blankj.utilcode.util.AbstractC1631WWWWWWWW;
import com.clone.android.dual.space.R;
import com.google.android.material.button.MaterialButton;
import n2.C3534WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p2.RunnableC3815WWWWWWWW;
import s2.C4086WWWWWWWW;
import s2.WWWWoWWWWo;
import ta.C4248WWWoWWWo;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class AdbActivationTutorialActivity2 extends AppBarActivity {

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public static final /* synthetic */ int f8277WWWWWWWW = 0;

    /* renamed from: WWWWoᜑWWWWoเᜑ  reason: contains not printable characters */
    public RunnableC3815WWWWWWWW f8278WWWWoWWWWo;

    /* renamed from: WWWWᜐWWWWଙᜐ  reason: contains not printable characters */
    public Handler f8279WWWWWWWW;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public MaterialButton f8280WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public View f8281WWWWWWWW;

    /* renamed from: WWWoᜒWWWo೧ᜒ  reason: contains not printable characters */
    public MaterialButton f8282WWWoWWWo;

    @Override // com.android.libadb.ui.base.AppBarActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_adb_activation_tutorial2);
        m2307WWoWWo().mo2341WoWo(true);
        View findViewById = findViewById(R.id.keep_dev_options);
        this.f8281WWWWWWWW = findViewById;
        findViewById.setVisibility(0);
        MaterialButton materialButton = (MaterialButton) findViewById(R.id.dev_options_btn);
        this.f8282WWWoWWWo = materialButton;
        materialButton.setOnClickListener(new View.OnClickListener(this) { // from class: p2.WWWWϙWWWWეϙ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbActivationTutorialActivity2 f32185WWWWWWWWWW;

            {
                this.f32185WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                AdbActivationTutorialActivity2 adbActivationTutorialActivity2 = this.f32185WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        int i10 = AdbActivationTutorialActivity2.f8277WWWWWWWW;
                        if (!WWWWoWWWWo.m16900WWWWWWWW(adbActivationTutorialActivity2)) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_developer_closed, 0).m17237WWWWWWWW();
                            return;
                        }
                        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                        if (!C4086WWWWWWWW.m16904WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{21, 60, 61, -81, 116, -1, -122, TarConstants.LF_LINK, 57, 60, 39, -70, Byte.MAX_VALUE, -3, -124, 29, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_FIFO, 39, -78, 105, -2, -109, 29, 22, TarConstants.LF_LINK, 40, -75, 105, -2, -116, 29, 22, 43, 38, -72, 110}, new byte[]{102, 89, 73, -37, 29, -111, -31, 66})).booleanValue()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_activated, 0).m17237WWWWWWWW();
                            return;
                        } else {
                            WWWWoWWWWo.m16899WWWWoWWWWo(adbActivationTutorialActivity2, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CHR, -36, -69, -53, -39, -2, 61, 78, 39, -35, -87, -60, -49, -3, TarConstants.LF_DIR, 78, 39, -57, -89, -55, -34, -31, 43, 78, 58, -38, -90, -61, -49, -3, 42}, new byte[]{87, -75, -56, -86, -69, -110, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 17}));
                            return;
                        }
                    case 1:
                        int i11 = AdbActivationTutorialActivity2.f8277WWWWWWWW;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_disconnected, 0).m17237WWWWWWWW();
                            return;
                        } else if (WWWWoWWWWo.m16900WWWWWWWW(adbActivationTutorialActivity2)) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_developer_opened, 0).m17237WWWWWWWW();
                            return;
                        } else {
                            try {
                                byte[] bArr = {36, 61, -40, 107, -80, 100, 68, -104, TarConstants.LF_FIFO, TarConstants.LF_FIFO, -56, 109, -74, 99, 71, -59, 107, 0, -7, TarConstants.LF_MULTIVOLUME, -117, 68, 110, -15, 22};
                                byte[] bArr2 = {69, TarConstants.LF_GNUTYPE_SPARSE, -68, 25, -33, 13, 32, -74};
                                C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                adbActivationTutorialActivity2.startActivity(new Intent(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)));
                                return;
                            } catch (Exception unused) {
                                return;
                            }
                        }
                    default:
                        int i12 = AdbActivationTutorialActivity2.f8277WWWWWWWW;
                        da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(adbActivationTutorialActivity2);
                        wWWWoWWWWo.m13648WoWo(R.string.adb_activation_tutorial_developer_dlg_title);
                        wWWWoWWWWo.m13642WWWWWWWW(R.string.adb_activation_tutorial_developer_dlg_msg);
                        wWWWoWWWWo.m13645WWWoWWWo(R.string.adb_activation_tutorial_dlg_i_known, null);
                        wWWWoWWWWo.m741WWoWWo();
                        return;
                }
            }
        });
        findViewById(R.id.dev_options_help_btn).setOnClickListener(new View.OnClickListener(this) { // from class: p2.WWWWϙWWWWეϙ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbActivationTutorialActivity2 f32185WWWWWWWWWW;

            {
                this.f32185WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                AdbActivationTutorialActivity2 adbActivationTutorialActivity2 = this.f32185WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        int i10 = AdbActivationTutorialActivity2.f8277WWWWWWWW;
                        if (!WWWWoWWWWo.m16900WWWWWWWW(adbActivationTutorialActivity2)) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_developer_closed, 0).m17237WWWWWWWW();
                            return;
                        }
                        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                        if (!C4086WWWWWWWW.m16904WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{21, 60, 61, -81, 116, -1, -122, TarConstants.LF_LINK, 57, 60, 39, -70, Byte.MAX_VALUE, -3, -124, 29, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_FIFO, 39, -78, 105, -2, -109, 29, 22, TarConstants.LF_LINK, 40, -75, 105, -2, -116, 29, 22, 43, 38, -72, 110}, new byte[]{102, 89, 73, -37, 29, -111, -31, 66})).booleanValue()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_activated, 0).m17237WWWWWWWW();
                            return;
                        } else {
                            WWWWoWWWWo.m16899WWWWoWWWWo(adbActivationTutorialActivity2, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CHR, -36, -69, -53, -39, -2, 61, 78, 39, -35, -87, -60, -49, -3, TarConstants.LF_DIR, 78, 39, -57, -89, -55, -34, -31, 43, 78, 58, -38, -90, -61, -49, -3, 42}, new byte[]{87, -75, -56, -86, -69, -110, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 17}));
                            return;
                        }
                    case 1:
                        int i11 = AdbActivationTutorialActivity2.f8277WWWWWWWW;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_disconnected, 0).m17237WWWWWWWW();
                            return;
                        } else if (WWWWoWWWWo.m16900WWWWWWWW(adbActivationTutorialActivity2)) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_developer_opened, 0).m17237WWWWWWWW();
                            return;
                        } else {
                            try {
                                byte[] bArr = {36, 61, -40, 107, -80, 100, 68, -104, TarConstants.LF_FIFO, TarConstants.LF_FIFO, -56, 109, -74, 99, 71, -59, 107, 0, -7, TarConstants.LF_MULTIVOLUME, -117, 68, 110, -15, 22};
                                byte[] bArr2 = {69, TarConstants.LF_GNUTYPE_SPARSE, -68, 25, -33, 13, 32, -74};
                                C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                adbActivationTutorialActivity2.startActivity(new Intent(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)));
                                return;
                            } catch (Exception unused) {
                                return;
                            }
                        }
                    default:
                        int i12 = AdbActivationTutorialActivity2.f8277WWWWWWWW;
                        da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(adbActivationTutorialActivity2);
                        wWWWoWWWWo.m13648WoWo(R.string.adb_activation_tutorial_developer_dlg_title);
                        wWWWoWWWWo.m13642WWWWWWWW(R.string.adb_activation_tutorial_developer_dlg_msg);
                        wWWWoWWWWo.m13645WWWoWWWo(R.string.adb_activation_tutorial_dlg_i_known, null);
                        wWWWoWWWWo.m741WWoWWo();
                        return;
                }
            }
        });
        MaterialButton materialButton2 = (MaterialButton) findViewById(R.id.activation_btn);
        this.f8280WWWWWWWW = materialButton2;
        materialButton2.setOnClickListener(new View.OnClickListener(this) { // from class: p2.WWWWϙWWWWეϙ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbActivationTutorialActivity2 f32185WWWWWWWWWW;

            {
                this.f32185WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                AdbActivationTutorialActivity2 adbActivationTutorialActivity2 = this.f32185WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        int i10 = AdbActivationTutorialActivity2.f8277WWWWWWWW;
                        if (!WWWWoWWWWo.m16900WWWWWWWW(adbActivationTutorialActivity2)) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_developer_closed, 0).m17237WWWWWWWW();
                            return;
                        }
                        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                        if (!C4086WWWWWWWW.m16904WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{21, 60, 61, -81, 116, -1, -122, TarConstants.LF_LINK, 57, 60, 39, -70, Byte.MAX_VALUE, -3, -124, 29, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_FIFO, 39, -78, 105, -2, -109, 29, 22, TarConstants.LF_LINK, 40, -75, 105, -2, -116, 29, 22, 43, 38, -72, 110}, new byte[]{102, 89, 73, -37, 29, -111, -31, 66})).booleanValue()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_activated, 0).m17237WWWWWWWW();
                            return;
                        } else {
                            WWWWoWWWWo.m16899WWWWoWWWWo(adbActivationTutorialActivity2, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CHR, -36, -69, -53, -39, -2, 61, 78, 39, -35, -87, -60, -49, -3, TarConstants.LF_DIR, 78, 39, -57, -89, -55, -34, -31, 43, 78, 58, -38, -90, -61, -49, -3, 42}, new byte[]{87, -75, -56, -86, -69, -110, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 17}));
                            return;
                        }
                    case 1:
                        int i11 = AdbActivationTutorialActivity2.f8277WWWWWWWW;
                        if (!AbstractC1631WWWWWWWW.m5303WWWWWWWW()) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_wifi_disconnected, 0).m17237WWWWWWWW();
                            return;
                        } else if (WWWWoWWWWo.m16900WWWWWWWW(adbActivationTutorialActivity2)) {
                            C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.adb_activation_tutorial_developer_opened, 0).m17237WWWWWWWW();
                            return;
                        } else {
                            try {
                                byte[] bArr = {36, 61, -40, 107, -80, 100, 68, -104, TarConstants.LF_FIFO, TarConstants.LF_FIFO, -56, 109, -74, 99, 71, -59, 107, 0, -7, TarConstants.LF_MULTIVOLUME, -117, 68, 110, -15, 22};
                                byte[] bArr2 = {69, TarConstants.LF_GNUTYPE_SPARSE, -68, 25, -33, 13, 32, -74};
                                C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                                adbActivationTutorialActivity2.startActivity(new Intent(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)));
                                return;
                            } catch (Exception unused) {
                                return;
                            }
                        }
                    default:
                        int i12 = AdbActivationTutorialActivity2.f8277WWWWWWWW;
                        da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(adbActivationTutorialActivity2);
                        wWWWoWWWWo.m13648WoWo(R.string.adb_activation_tutorial_developer_dlg_title);
                        wWWWoWWWWo.m13642WWWWWWWW(R.string.adb_activation_tutorial_developer_dlg_msg);
                        wWWWoWWWWo.m13645WWWoWWWo(R.string.adb_activation_tutorial_dlg_i_known, null);
                        wWWWoWWWWo.m741WWoWWo();
                        return;
                }
            }
        });
        Handler handler = new Handler();
        this.f8279WWWWWWWW = handler;
        RunnableC3815WWWWWWWW runnableC3815WWWWWWWW = new RunnableC3815WWWWWWWW(this, 0);
        this.f8278WWWWoWWWWo = runnableC3815WWWWWWWW;
        handler.postDelayed(runnableC3815WWWWWWWW, 500L);
    }

    @Override // androidx.appcompat.app.AppCompatActivity, androidx.fragment.app.FragmentActivity, android.app.Activity
    public final void onDestroy() {
        super.onDestroy();
        this.f8279WWWWWWWW.removeCallbacks(this.f8278WWWWoWWWWo);
    }
}
