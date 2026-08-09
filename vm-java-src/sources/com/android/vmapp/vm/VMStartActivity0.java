package com.android.vmapp.vm;

import android.app.ActivityManager;
import android.content.Context;
import android.content.Intent;
import android.content.res.TypedArray;
import android.os.Bundle;
import android.text.TextUtils;
import android.util.Log;
import android.view.View;
import android.widget.RelativeLayout;
import android.widget.TextView;
import androidx.appcompat.app.C0791WWWWWWWW;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import c4.DialogInterface$OnCancelListenerC1508WWoWWo;
import com.airbnb.lottie.EnumC1552WWWWWWWW;
import com.airbnb.lottie.LottieAnimationView;
import com.android.vmapp.ui.base.BaseActivity;
import com.android.vmapp.vm.VMReportActivity;
import com.android.vmapp.vm.VMStartActivity0;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.VMStatusEvent;
import com.android.vmcore.utils.CPUUtils;
import com.blankj.utilcode.util.WWWW;
import com.clone.android.dual.space.R;
import com.google.android.gms.internal.ads.pr0;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import da.WWWWoWWWWo;
import eh.InterfaceC2472WWWWWWWW;
import h4.DialogInterface$OnClickListenerC2716WWWWWWWW;
import im.amomo.andun7z.AndUn7z;
import j3.C3164WWWWWWWW;
import n2.WWWoWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.greenrobot.eventbus.ThreadMode;
import p000WWWWWWWWWW.WWoWWo;
import p053WWoWWo.AbstractC0470WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class VMStartActivity0 extends BaseActivity {

    /* renamed from: WWoᵺWWoၐᵺ  reason: contains not printable characters */
    public static final String f8776WWoWWo;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public TextView f8777WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public TextView f8778WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public LottieAnimationView f8779WWWWWWWW;

    /* renamed from: WWWWᬭWWWWɿᬭ  reason: contains not printable characters */
    public boolean f8780WWWWWWWW;

    /* renamed from: WWWWᮭWWWWᆏᮭ  reason: contains not printable characters */
    public boolean f8781WWWWWWWW;

    /* renamed from: WWWWᲕWWWWȷᲕ  reason: contains not printable characters */
    public boolean f8782WWWWWWWW;

    /* renamed from: WWWoᰠWWWoઠᰠ  reason: contains not printable characters */
    public boolean f8783WWWoWWWo;

    /* renamed from: WWo᪙WWoг᪙  reason: contains not printable characters */
    public int f8784WWoWWo;

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public TextView f8785WWWW;

    /* renamed from: Wo᪅Woै᪅  reason: contains not printable characters */
    public int f8786WoWo;

    static {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        f8776WWoWWo = WWWWWWWW.m17835WWWWWWWW(new byte[]{-89, 73, -7, 37, 8, 72, -13, 43, -110, 112, -61, 39, 0, 78, -2}, new byte[]{-15, 4, -86, 81, 105, 58, -121, 106});
    }

    /* renamed from: WWኮWWႉኮ  reason: contains not printable characters */
    public static void m5016WWWW(Context context, int i10, boolean z10, boolean z11) {
        Intent intent;
        int i11 = i10 % 4;
        if (i11 != 0) {
            if (i11 != 1) {
                if (i11 != 2) {
                    intent = new Intent(context, VMStartActivity3.class);
                } else {
                    intent = new Intent(context, VMStartActivity2.class);
                }
            } else {
                intent = new Intent(context, VMStartActivity1.class);
            }
        } else {
            intent = new Intent(context, VMStartActivity0.class);
        }
        intent.addFlags(268435456);
        byte[] bArr = {23, 4, 126, -13, 107, 119, 97, TarConstants.LF_GNUTYPE_LONGNAME};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        intent.putExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{97, 105, 33, -102, 15}, bArr), i10);
        intent.putExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, -120, -17, 59, -100, 45, 114, 124, TarConstants.LF_GNUTYPE_LONGLINK, -65, -31, 34, -86, 34, 124, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 86, -113, -18}, new byte[]{63, -32, Byte.MIN_VALUE, TarConstants.LF_GNUTYPE_LONGNAME, -61, 79, 29, 19}), z10);
        intent.putExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{47, TarConstants.LF_SYMLINK, 25, 25, TarConstants.LF_GNUTYPE_SPARSE, 58, 108, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_CHR}, new byte[]{65, 93, 70, 107, TarConstants.LF_FIFO, 74, 13, 14}), z11);
        context.startActivity(intent);
    }

    /* renamed from: WWWWoᆣWWWWoϿᆣ  reason: contains not printable characters */
    public final String m5017WWWWoWWWWo(int i10) {
        switch (i10) {
            case 100500:
                return getString(R.string.vm_error_unsupported_sdk, Integer.valueOf(this.f8505WWWWWWWW.f8937WWWoWWWo.f8895WWWoWWWo.f8847WWWWWWWW));
            case 101000:
                return getString(R.string.vm_error_unsupported_device, CPUUtils.m5241WWWWWWWW());
            case 101500:
                return getString(R.string.vm_error_unsupported_app, this.f8505WWWWWWWW.f8937WWWoWWWo.f8895WWWoWWWo.f8850WWWWWWWW);
            case 102000:
                return getString(R.string.vm_error_unsupported_user);
            case 105000:
                return getString(R.string.vm_error_install_fs_failed);
            case 114500:
                return getString(R.string.vm_error_check_fs_failed);
            default:
                return getString(R.string.vm_error_common_msg);
        }
    }

    /* renamed from: WWWWሎWWWWܣሎ  reason: contains not printable characters */
    public final void m5018WWWWWWWW(String str, boolean z10) {
        TextView textView = this.f8777WWWWWWWW;
        if (textView == null) {
            return;
        }
        if (z10) {
            textView.setTextColor(this.f8784WWoWWo);
            LottieAnimationView lottieAnimationView = this.f8779WWWWWWWW;
            if (lottieAnimationView != null && lottieAnimationView.getVisibility() != 4) {
                this.f8779WWWWWWWW.setVisibility(4);
                LottieAnimationView lottieAnimationView2 = this.f8779WWWWWWWW;
                lottieAnimationView2.f7792WWoWWo = false;
                lottieAnimationView2.f7790WWWoWWWo.m4677WWWWWWWW();
            }
            this.f8778WWWWWWWW.setVisibility(0);
        } else {
            textView.setTextColor(this.f8786WoWo);
            LottieAnimationView lottieAnimationView3 = this.f8779WWWWWWWW;
            if (lottieAnimationView3 != null && lottieAnimationView3.getVisibility() != 0) {
                this.f8779WWWWWWWW.setVisibility(0);
                LottieAnimationView lottieAnimationView4 = this.f8779WWWWWWWW;
                lottieAnimationView4.f7787WWWWWWWW.add(EnumC1552WWWWWWWW.f7866WWoWWo);
                lottieAnimationView4.f7790WWWoWWWo.m4678WWWWWWWW();
            }
            this.f8778WWWWWWWW.setVisibility(4);
        }
        this.f8777WWWWWWWW.setText(str);
    }

    /* renamed from: WWWWሗWWWWшሗ  reason: contains not printable characters */
    public final void m5019WWWWWWWW(int i10) {
        WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
        String string = getString(R.string.dialog_title_vm_error_detail, Integer.valueOf(i10));
        C0791WWWWWWWW c0791wwwwwwww = (C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW;
        c0791wwwwwwww.f3547WWWWWWWW = string;
        c0791wwwwwwww.f3561WWoWWo = m5017WWWWoWWWWo((i10 / 100) * 100);
        wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, null);
        wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_report, new DialogInterface$OnClickListenerC2716WWWWWWWW(this, i10, 1));
        wWWWoWWWWo.mo742WWWW().show();
    }

    /* renamed from: WWWWኈWWWWˡኈ  reason: contains not printable characters */
    public final void m5020WWWWWWWW(boolean z10) {
        VMInstance vMInstance = this.f8505WWWWWWWW;
        int i10 = vMInstance.f8940WWoWWo;
        if ((this.f8782WWWWWWWW && i10 >= 5) || i10 >= 7) {
            this.f8780WWWWWWWW = true;
            if (!this.f8781WWWWWWWW) {
                m5021WWoWWo();
                return;
            }
            return;
        }
        vMInstance.m5100WoWo(z10);
    }

    /* renamed from: WWoᆑWWoӁᆑ  reason: contains not printable characters */
    public final void m5021WWoWWo() {
        if (!isFinishing() && !isDestroyed()) {
            Intent intent = new Intent(this, VMDisplayActivity.class);
            intent.addFlags(32768);
            intent.addFlags(67108864);
            startActivity(intent);
            finish();
        }
    }

    @Override // androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, android.app.Activity
    public final void onActivityResult(int i10, int i11, Intent intent) {
        super.onActivityResult(i10, i11, intent);
        if (i10 == 100) {
            this.f8781WWWWWWWW = false;
            if (this.f8780WWWWWWWW) {
                m5021WWoWWo();
            }
        } else if (i10 == 101) {
            this.f8781WWWWWWWW = false;
            if (this.f8780WWWWWWWW) {
                m5021WWoWWo();
            }
        }
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        boolean z10 = false;
        StringBuilder sb2 = new StringBuilder();
        byte[] bArr = {15, 36, -42, 69, -108, TarConstants.LF_GNUTYPE_SPARSE, 2, 6, 64, 57, -31, 86, -125, 70, 86};
        byte[] bArr2 = {96, 74, -107, TarConstants.LF_CONTIG, -15, TarConstants.LF_SYMLINK, 118, 99};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        sb2.append(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        sb2.append(this);
        String sb3 = sb2.toString();
        String str = f8776WWoWWo;
        Log.d(str, sb3);
        overridePendingTransition(0, 0);
        super.onCreate(bundle);
        if (this.f8505WWWWWWWW == null) {
            finish();
            return;
        }
        setTaskDescription(new ActivityManager.TaskDescription(this.f8505WWWWWWWW.f8937WWWoWWWo.f8861WWWWoWWWWo));
        setContentView(R.layout.activity_vm_start);
        this.f8777WWWWWWWW = (TextView) findViewById(R.id.status);
        this.f8778WWWWWWWW = (TextView) findViewById(R.id.detail);
        this.f8779WWWWWWWW = (LottieAnimationView) findViewById(R.id.loading_animation);
        this.f8785WWWW = (TextView) findViewById(R.id.report);
        try {
            TypedArray obtainStyledAttributes = obtainStyledAttributes(new int[]{16842806, 16844099});
            this.f8786WoWo = obtainStyledAttributes.getColor(0, this.f8777WWWWWWWW.getCurrentTextColor());
            this.f8784WWoWWo = obtainStyledAttributes.getColor(1, -65536);
            obtainStyledAttributes.recycle();
        } catch (Throwable unused) {
            this.f8786WoWo = this.f8777WWWWWWWW.getCurrentTextColor();
            this.f8784WWoWWo = -65536;
        }
        TypedArray obtainStyledAttributes2 = getTheme().obtainStyledAttributes(new int[]{R.attr.colorPrimary});
        int color = obtainStyledAttributes2.getColor(0, 0);
        obtainStyledAttributes2.recycle();
        this.f8778WWWWWWWW.setTextColor(color);
        this.f8778WWWWWWWW.setOnClickListener(new View.OnClickListener(this) { // from class: r4.WoڄWoᄴڄ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMStartActivity0 f32993WWWWWWWWWW;

            {
                this.f32993WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMStartActivity0 vMStartActivity0 = this.f32993WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        String str2 = VMStartActivity0.f8776WWoWWo;
                        try {
                            vMStartActivity0.m5019WWWWWWWW(((Integer) vMStartActivity0.f8778WWWWWWWW.getTag()).intValue());
                            return;
                        } catch (Throwable unused2) {
                            return;
                        }
                    default:
                        vMStartActivity0.f8781WWWWWWWW = true;
                        Intent intent = new Intent(vMStartActivity0, VMReportActivity.class);
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        intent.putExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{-69, -122, 111, 104, -36, 79, 79, 59, -72, -106, 109}, new byte[]{-35, -13, 3, 4, -125, 60, 44, 73}), false);
                        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-47, 18, 25, -21}, new byte[]{-73, 96, 118, -122, 117, -62, 110, -26});
                        intent.putExtra(m17835WWWWWWWW, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + vMStartActivity0.f8785WWWW.getTag());
                        vMStartActivity0.startActivityForResult(intent, 100);
                        return;
                }
            }
        });
        this.f8785WWWW.setTextColor(color);
        RelativeLayout.LayoutParams layoutParams = (RelativeLayout.LayoutParams) this.f8785WWWW.getLayoutParams();
        layoutParams.bottomMargin = WWWW.m5346WWWW();
        this.f8785WWWW.setLayoutParams(layoutParams);
        this.f8785WWWW.setOnClickListener(new View.OnClickListener(this) { // from class: r4.WoڄWoᄴڄ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ VMStartActivity0 f32993WWWWWWWWWW;

            {
                this.f32993WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                VMStartActivity0 vMStartActivity0 = this.f32993WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        String str2 = VMStartActivity0.f8776WWoWWo;
                        try {
                            vMStartActivity0.m5019WWWWWWWW(((Integer) vMStartActivity0.f8778WWWWWWWW.getTag()).intValue());
                            return;
                        } catch (Throwable unused2) {
                            return;
                        }
                    default:
                        vMStartActivity0.f8781WWWWWWWW = true;
                        Intent intent = new Intent(vMStartActivity0, VMReportActivity.class);
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        intent.putExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{-69, -122, 111, 104, -36, 79, 79, 59, -72, -106, 109}, new byte[]{-35, -13, 3, 4, -125, 60, 44, 73}), false);
                        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-47, 18, 25, -21}, new byte[]{-73, 96, 118, -122, 117, -62, 110, -26});
                        intent.putExtra(m17835WWWWWWWW, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + vMStartActivity0.f8785WWWW.getTag());
                        vMStartActivity0.startActivityForResult(intent, 100);
                        return;
                }
            }
        });
        Intent intent = getIntent();
        if (intent != null) {
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            this.f8782WWWWWWWW = intent.getBooleanExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{-113, -51, 114, TarConstants.LF_GNUTYPE_LONGLINK, -55, -97, 87, 18, -120, -6, 124, 82, -1, -112, 89, 9, -107, -54, 115}, new byte[]{-4, -91, 29, 60, -106, -3, 56, 125}), false);
            z10 = intent.getBooleanExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{36, 94, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -81, -82, -18, 25, TarConstants.LF_CONTIG, 56}, new byte[]{74, TarConstants.LF_LINK, 56, -35, -53, -98, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 94}), false);
        }
        m5020WWWWWWWW(z10);
        this.f8505WWWWWWWW.f8939WWWoWWWo.m13950WWWW(this);
        StringBuilder sb4 = new StringBuilder();
        byte[] bArr3 = {-80, -107, TarConstants.LF_GNUTYPE_SPARSE, -41, 78, 4, 8, 57};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        sb4.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-33, -5, 16, -91, 43, 101, 124, 92, -112, -16, 61, -77, 110}, bArr3));
        sb4.append(this);
        Log.d(str, sb4.toString());
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.appcompat.app.AppCompatActivity, androidx.fragment.app.FragmentActivity, android.app.Activity
    public final void onDestroy() {
        StringBuilder sb2 = new StringBuilder();
        byte[] bArr = {-9, ConstantPoolEntry.CP_InterfaceMethodref, -110, 37, -96, -21, 17, 121};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-104, 101, -42, 64, -45, -97, 99, 22, -114, 43, -31, 81, -63, -103, 101, 89}, bArr));
        sb2.append(this);
        String sb3 = sb2.toString();
        String str = f8776WWoWWo;
        Log.d(str, sb3);
        super.onDestroy();
        VMInstance vMInstance = this.f8505WWWWWWWW;
        if (vMInstance != null) {
            vMInstance.f8939WWWoWWWo.m13945WWWWWWWW(this);
        }
        Log.d(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{6, TarConstants.LF_FIFO, -7, 31, -105, -121, -122, -35, 16, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -40, 20, Byte.MIN_VALUE, -45}, new byte[]{105, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -67, 122, -28, -13, -12, -78}) + this);
    }

    @Override // androidx.appcompat.app.AppCompatActivity, androidx.fragment.app.FragmentActivity, android.app.Activity
    public final void onStart() {
        super.onStart();
        getWindow().addFlags(128);
    }

    @Override // androidx.appcompat.app.AppCompatActivity, androidx.fragment.app.FragmentActivity, android.app.Activity
    public final void onStop() {
        super.onStop();
        getWindow().clearFlags(128);
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.MAIN)
    public void onVMHealthStateChangedEvent(m3.WWWWoWWWWo wWWWoWWWWo) {
        String m17835WWWWWWWW;
        String str;
        if (!isFinishing() && !isDestroyed()) {
            int i10 = wWWWoWWWWo.f30920WWWWWWWW;
            if (i10 == 3) {
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{85, -94, -121, -126, 109, -100, 47, -81, 80, -69, -83, -125, 105}, new byte[]{35, -49, -40, -32, 2, -13, 91, -16});
            } else if (i10 == 2) {
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-13, -96, 0, 118, -106, 25, -116, 102, -10, -66, 0, 98, -115, 19, -117}, new byte[]{-123, -51, 95, 6, -28, 118, -17, 3});
            } else if (i10 == 1) {
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{108, -96, 38, 116, -35, -30, -51, -23, 110, -92, 20, 115, -35, -8, -51}, new byte[]{26, -51, 121, 22, -78, -115, -71, -74});
            } else if (i10 == 4) {
                byte[] bArr = {119, 82, -127, -40, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -59, 20, -37, 115, 94, -83, -47};
                byte[] bArr2 = {1, 63, -34, -71, 8, -75, TarConstants.LF_GNUTYPE_LONGLINK, -72};
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
            } else {
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-58, Byte.MIN_VALUE, -57, -8, -73, 24, -32}, new byte[]{-77, -18, -84, -106, -40, 111, -114, 122});
            }
            this.f8785WWWW.setTag(m17835WWWWWWWW);
            this.f8785WWWW.setVisibility(0);
            if (!this.f8783WWWoWWWo) {
                this.f8783WWWoWWWo = true;
                this.f8781WWWWWWWW = true;
                WWWWoWWWWo wWWWoWWWWo2 = new WWWWoWWWWo(this);
                wWWWoWWWWo2.m13648WoWo(R.string.dialog_title_vm_health);
                int i11 = wWWWoWWWWo.f30920WWWWWWWW;
                if (i11 == 3) {
                    str = getString(R.string.dialog_message_vm_health_has_stuck);
                } else if (i11 == 2) {
                    if (this.f8505WWWWWWWW.f8940WWoWWo >= 7) {
                        str = getString(R.string.dialog_message_vm_health_has_died_2);
                    } else {
                        str = getString(R.string.dialog_message_vm_health_has_died_1);
                    }
                } else if (i11 == 1) {
                    str = getString(R.string.dialog_message_vm_health_timeout);
                } else if (i11 == 4) {
                    str = getString(R.string.dialog_message_vm_health_app_crash);
                } else {
                    str = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
                }
                if (WWWoWWWo.m16010WWWWWWWW()) {
                    StringBuilder m58WWoWWo = WWoWWo.m58WWoWWo(str, "\n");
                    m58WWoWWo.append(getString(R.string.dialog_message_vm_health_activation));
                    str = m58WWoWWo.toString();
                }
                StringBuilder m1577WWWWoWWWWo = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(str);
                pr0.m9002WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{26, 124, 1, TarConstants.LF_GNUTYPE_LONGLINK, 13, -102, 90}, new byte[]{16, 118, 87, 46, Byte.MAX_VALUE, -96, 122, 57}, m1577WWWWoWWWWo);
                String m9000WWWWWWWW = pr0.m9000WWWWWWWW(new byte[]{85, 14, 90, 33, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 28}, new byte[]{102, 32, 104, 15, 82, 47, 61, -25}, m1577WWWWoWWWWo);
                String str2 = wWWWoWWWWo.f30919WWWWoWWWWo;
                if (!TextUtils.isEmpty(str2)) {
                    m9000WWWWWWWW = WWoWWo.m51WWWWWWWW(m9000WWWWWWWW, "\n", str2);
                }
                C0791WWWWWWWW c0791wwwwwwww = (C0791WWWWWWWW) wWWWoWWWWo2.f1045WWWWWWWW;
                c0791wwwwwwww.f3561WWoWWo = m9000WWWWWWWW;
                wWWWoWWWWo2.m13645WWWoWWWo(R.string.dialog_button_ignore, new n2.WWWWWWWW(7));
                wWWWoWWWWo2.m13646WWoWWo(R.string.dialog_button_report, new c4.WWWWoWWWWo(8, this, m17835WWWWWWWW));
                if (WWWoWWWo.m16010WWWWWWWW()) {
                    wWWWoWWWWo2.m13643WWWWWWWW(R.string.for_you_adb_activate, new r4.WWWW(this, 1));
                } else {
                    wWWWoWWWWo2.m13643WWWWWWWW(R.string.dialog_button_tg, new r4.WWWW(this, 2));
                }
                c0791wwwwwwww.f3562WWoWWo = new DialogInterface$OnCancelListenerC1508WWoWWo(this, 2);
                androidx.appcompat.app.WWWW mo742WWWW = wWWWoWWWWo2.mo742WWWW();
                mo742WWWW.setCanceledOnTouchOutside(false);
                mo742WWWW.show();
            }
        }
    }

    @InterfaceC2472WWWWWWWW(sticky = AndUn7z.f28850WWWWWWWW, threadMode = ThreadMode.MAIN)
    public void onVMStatusEvent(VMStatusEvent vMStatusEvent) {
        String string;
        if (!isFinishing() && !isDestroyed()) {
            int i10 = vMStatusEvent.f9016WWWWWWWW;
            int i11 = vMStatusEvent.f9015WWWWoWWWWo;
            switch (i10) {
                case -4:
                    this.f8778WWWWWWWW.setTag(Integer.valueOf(i11));
                    m5018WWWWWWWW(getString(R.string.vm_status_start_failed, Integer.valueOf(i11)), true);
                    break;
                case -3:
                    this.f8778WWWWWWWW.setTag(Integer.valueOf(i11));
                    m5018WWWWWWWW(getString(R.string.vm_status_start_svc_failed, Integer.valueOf(i11)), true);
                    break;
                case -2:
                    this.f8778WWWWWWWW.setTag(Integer.valueOf(i11));
                    m5018WWWWWWWW(getString(R.string.vm_status_install_failed, Integer.valueOf(i11)), true);
                    break;
                case -1:
                    this.f8778WWWWWWWW.setTag(Integer.valueOf(i11));
                    m5018WWWWWWWW(getString(R.string.vm_status_env_failed, Integer.valueOf(i11)), true);
                    break;
                case 1:
                    m5018WWWWWWWW(getString(R.string.vm_status_checking_env), false);
                    break;
                case 2:
                    byte[] bArr = {10, -99, -124, 117, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -39};
                    byte[] bArr2 = {TarConstants.LF_PAX_EXTENDED_HEADER_LC, -8, -12, 20, TarConstants.LF_LINK, -85, -55, -79};
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    if (WWWWWWWW.m17835WWWWWWWW(bArr, bArr2).equals(this.f8505WWWWWWWW.f8937WWWoWWWo.f8923WoWo)) {
                        m5018WWWWWWWW(getString(R.string.vm_status_installing_3), false);
                        break;
                    } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType, -94, TarConstants.LF_GNUTYPE_SPARSE, -14, 19, 81}, new byte[]{121, -46, TarConstants.LF_CONTIG, -109, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_BLK, -63, -83}).equals(this.f8505WWWWWWWW.f8937WWWoWWWo.f8923WoWo)) {
                        m5018WWWWWWWW(getString(R.string.vm_status_installing_2), false);
                        break;
                    } else {
                        m5018WWWWWWWW(getString(R.string.vm_status_installing_1), false);
                        break;
                    }
                case 3:
                    m5018WWWWWWWW(getString(R.string.vm_status_starting_svc), false);
                    break;
                case 4:
                    String str = vMStatusEvent.f9017WWWoWWWo;
                    if (TextUtils.isEmpty(str)) {
                        string = getString(R.string.vm_status_starting);
                    } else if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{116, -57, 41, -117, -37, 86}, new byte[]{18, -82, 81, -44, -67, 37, 100, 116}, str)) {
                        string = getString(R.string.vm_status_fixing_fs);
                    } else if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{35, 111, -119, -60, -61, 63, 35, 23}, new byte[]{74, 1, -6, -80, -94, TarConstants.LF_GNUTYPE_SPARSE, 79, 72}))) {
                        string = getString(R.string.vm_status_installing_plugin, str.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{-14, TarConstants.LF_GNUTYPE_LONGLINK, -61, 57, -24, -115, TarConstants.LF_PAX_EXTENDED_HEADER_UC, Byte.MAX_VALUE}, new byte[]{-101, 37, -80, TarConstants.LF_MULTIVOLUME, -119, -31, TarConstants.LF_BLK, 32}).length()));
                    } else if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{37, TarConstants.LF_DIR, -98, 14, 111, -15, 82, -71, 33, TarConstants.LF_CONTIG, -126, 3, 111, -15}, new byte[]{68, 69, -18, 98, 22, -82, 61, -49}))) {
                        string = getString(R.string.vm_status_applying_overlay, str.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{113, 68, -115, 65, -66, 89, -113, ConstantPoolEntry.CP_NameAndType, 117, 70, -111, TarConstants.LF_GNUTYPE_LONGNAME, -66, 89}, new byte[]{16, TarConstants.LF_BLK, -3, 45, -57, 6, -32, 122}).length()));
                    } else {
                        string = getString(R.string.vm_status_starting);
                    }
                    m5018WWWWWWWW(string, false);
                    break;
                case 5:
                    m5018WWWWWWWW(getString(R.string.vm_status_os_booting), false);
                    break;
                case 6:
                    m5018WWWWWWWW(getString(R.string.vm_status_os_ready1), false);
                    break;
                case 7:
                    m5018WWWWWWWW(getString(R.string.vm_status_os_ready2), false);
                    break;
            }
            boolean z10 = this.f8505WWWWWWWW.f8937WWWoWWWo.f8913WWoWWo;
            int i12 = vMStatusEvent.f9016WWWWWWWW;
            if (z10 && i12 == -2) {
                WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
                wWWWoWWWWo.m13648WoWo(R.string.dialog_title_repair_failed);
                wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_repair_failed);
                wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_continue, new r4.WWWW(this, 0));
                wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, null);
                wWWWoWWWWo.mo742WWWW().show();
            } else if ((this.f8782WWWWWWWW && i12 >= 5) || i12 >= 7) {
                this.f8780WWWWWWWW = true;
                if (!this.f8781WWWWWWWW) {
                    m5021WWoWWo();
                }
            } else if (i11 == 114500) {
                WWWWoWWWWo wWWWoWWWWo2 = new WWWWoWWWWo(this);
                wWWWoWWWWo2.m13648WoWo(R.string.dialog_title_check_fs_failed);
                ((C0791WWWWWWWW) wWWWoWWWWo2.f1045WWWWWWWW).f3561WWoWWo = m5017WWWWoWWWWo(114500);
                wWWWoWWWWo2.m13645WWWoWWWo(R.string.dialog_button_exit, new r4.WWWW(this, 3));
                wWWWoWWWWo2.m13646WWoWWo(R.string.dialog_button_cancel, null);
                wWWWoWWWWo2.mo742WWWW().show();
            }
        }
    }
}
