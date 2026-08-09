package com.android.vmapp.vm;

import android.app.ActivityManager;
import android.content.Context;
import android.content.DialogInterface;
import android.content.Intent;
import android.content.res.Configuration;
import android.os.Build;
import android.os.Bundle;
import android.text.TextUtils;
import android.util.Log;
import android.view.KeyEvent;
import android.view.ViewGroup;
import android.view.Window;
import android.widget.FrameLayout;
import androidx.appcompat.app.C0791WWWWWWWW;
import androidx.datastore.preferences.protobuf.C0962WWWoWWWo;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.libadb.ui.AdbActivationMethodActivity;
import com.android.vmapp.ui.base.BaseActivity;
import com.android.vmapp.vm.VMDisplayActivity;
import com.android.vmcore.VMInstance;
import com.android.vmcore.VMResConfig;
import com.android.vmcore.event.DialNumberEvent;
import com.android.vmcore.event.PermissionEvent;
import com.android.vmcore.event.SendSmsEvent;
import com.android.vmcore.hal.AudioService;
import com.android.vmcore.hal.HALManager;
import com.android.vmcore.ui.VMSurfaceView;
import com.blankj.utilcode.util.WWWW;
import com.clone.android.dual.space.R;
import com.google.android.gms.internal.ads.pr0;
import com.google.android.gms.internal.consent_sdk.AbstractC1812WWWW;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import da.WWWWoWWWWo;
import dc.WWWoWWWo;
import eh.InterfaceC2472WWWWWWWW;
import im.amomo.andun7z.AndUn7z;
import j3.C3164WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.greenrobot.eventbus.ThreadMode;
import p000WWWWWWWWWW.WWoWWo;
import p001WWWWoWWWWo.RunnableC0054WWWWWWWW;
import p020WWWWWWWW.AbstractC0211WWWWWWWW;
import p053WWoWWo.AbstractC0470WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class VMDisplayActivity extends BaseActivity {

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public static final String f8761WWWW;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public FrameLayout f8762WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public VMSurfaceView f8763WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public boolean f8764WWWWWWWW;

    static {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        f8761WWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{107, 44, -47, -113, 101, TarConstants.LF_GNUTYPE_SPARSE, 8, -118, 68, 32, -10, -110, Byte.MAX_VALUE, 85, 13, -97, 68}, new byte[]{61, 97, -107, -26, 22, 35, 100, -21});
    }

    /* renamed from: WWoᆑWWoӁᆑ  reason: contains not printable characters */
    public final void m5011WWoWWo(final boolean z10) {
        final FrameLayout.LayoutParams layoutParams = (FrameLayout.LayoutParams) this.f8763WWWWWWWW.getLayoutParams();
        layoutParams.topMargin = 0;
        layoutParams.leftMargin = 0;
        this.f8763WWWWWWWW.setLayoutParams(layoutParams);
        this.f8763WWWWWWWW.setLandscape(!z10);
        final boolean z11 = this.f8505WWWWWWWW.f8937WWWoWWWo.f8922WoWo;
        final int i10 = BaseActivity.f8501WWWoWWWo;
        this.f8763WWWWWWWW.setOnVMSurfaceSizeListener(new VMSurfaceView.OnVMSurfaceSizeListener() { // from class: r4.WWWWo̐WWWWoȄ̐
            @Override // com.android.vmcore.ui.VMSurfaceView.OnVMSurfaceSizeListener
            /* renamed from: WWWW̏WWWWβ̏ */
            public final void mo5236WWWWWWWW(int i11, int i12) {
                String str = VMDisplayActivity.f8761WWWW;
                VMDisplayActivity vMDisplayActivity = VMDisplayActivity.this;
                vMDisplayActivity.getClass();
                if (!z11 && i11 > 0 && i12 > 0 && vMDisplayActivity.f8762WWWWWWWW.getWidth() > 0 && vMDisplayActivity.f8762WWWWWWWW.getHeight() > 0) {
                    FrameLayout.LayoutParams layoutParams2 = layoutParams;
                    boolean z12 = z10;
                    int i13 = i10;
                    if (z12) {
                        int height = (vMDisplayActivity.f8762WWWWWWWW.getHeight() - i12) / 2;
                        if (height < i13 && layoutParams2.topMargin != i13 - height) {
                            layoutParams2.topMargin = i13;
                            vMDisplayActivity.f8763WWWWWWWW.setLayoutParams(layoutParams2);
                            return;
                        }
                        return;
                    }
                    int width = (vMDisplayActivity.f8762WWWWWWWW.getWidth() - i11) / 2;
                    if (width < i13 && layoutParams2.leftMargin != i13 - width) {
                        layoutParams2.leftMargin = i13;
                        vMDisplayActivity.f8763WWWWWWWW.setLayoutParams(layoutParams2);
                    }
                }
            }
        });
        this.f8762WWWWWWWW.post(new RunnableC0054WWWWWWWW(26, this));
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, android.app.Activity, android.view.Window.Callback
    public final void onAttachedToWindow() {
        super.onAttachedToWindow();
        Window window = getWindow();
        AbstractC3339WWWWWWWW.m15439WWoWWo(window, "window");
        AbstractC1812WWWW.m10922WWWWWWWW();
        WWWoWWWo wWWoWWWo = AbstractC1812WWWW.f22354WWWoWWWo;
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(wWWoWWWo);
        wWWoWWWo.mo13655WWWWWWWW(window);
    }

    @Override // androidx.appcompat.app.AppCompatActivity, androidx.activity.ComponentActivity, android.app.Activity, android.content.ComponentCallbacks
    public final void onConfigurationChanged(Configuration configuration) {
        super.onConfigurationChanged(configuration);
        if (this.f8762WWWWWWWW != null) {
            if (configuration.orientation == 1) {
                m5011WWoWWo(true);
            } else {
                m5011WWoWWo(false);
            }
        }
    }

    /* JADX WARN: Code restructure failed: missing block: B:15:0x00a3, code lost:
        if (r9.orientation == 1) goto L16;
     */
    /* JADX WARN: Code restructure failed: missing block: B:16:0x00a5, code lost:
        r9 = true;
     */
    /* JADX WARN: Code restructure failed: missing block: B:17:0x00a7, code lost:
        r9 = false;
     */
    /* JADX WARN: Code restructure failed: missing block: B:19:0x00b3, code lost:
        if (r9.f8952WWWWoWWWWo <= r9.f8955WWWoWWWo) goto L16;
     */
    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final void onCreate(Bundle bundle) {
        boolean z10;
        StringBuilder sb2 = new StringBuilder();
        byte[] bArr = {ConstantPoolEntry.CP_InterfaceMethodref, -85, -2, -16, TarConstants.LF_GNUTYPE_SPARSE, TarConstants.LF_GNUTYPE_SPARSE, 86, -71, 68, -74, -55, -29, 68, 70, 2};
        byte[] bArr2 = {100, -59, -67, -126, TarConstants.LF_FIFO, TarConstants.LF_SYMLINK, 34, -36};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        sb2.append(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        sb2.append(this);
        Log.d(f8761WWWW, sb2.toString());
        super.onCreate(bundle);
        VMInstance vMInstance = this.f8505WWWWWWWW;
        if (vMInstance == null) {
            finish();
        } else if (vMInstance.f8940WWoWWo < 5) {
            finish();
        } else {
            getWindow().getDecorView().setSystemUiVisibility(5894);
            setTaskDescription(new ActivityManager.TaskDescription(this.f8505WWWWWWWW.f8937WWWoWWWo.f8861WWWWoWWWWo));
            FrameLayout frameLayout = new FrameLayout(this);
            this.f8762WWWWWWWW = frameLayout;
            frameLayout.setBackgroundColor(-16777216);
            VMSurfaceView vMSurfaceView = new VMSurfaceView(this);
            this.f8763WWWWWWWW = vMSurfaceView;
            vMSurfaceView.setVM(this.f8505WWWWWWWW);
            this.f8763WWWWWWWW.setId(100);
            ViewGroup.LayoutParams layoutParams = new ViewGroup.LayoutParams(-1, -1);
            this.f8762WWWWWWWW.addView(this.f8763WWWWWWWW, layoutParams);
            setContentView(this.f8762WWWWWWWW, layoutParams);
            m3.WWWWWWWW wwwwwwww = (m3.WWWWWWWW) this.f8505WWWWWWWW.f8939WWWoWWWo.m13946WWWoWWWo(m3.WWWWWWWW.class);
            if (wwwwwwww == null || (r9 = wwwwwwww.f30921WWWWWWWW) == null) {
                VMResConfig m5061WWWWWWWW = this.f8505WWWWWWWW.m5061WWWWWWWW();
            }
            if (z10) {
                setRequestedOrientation(1);
            } else {
                setRequestedOrientation(0);
            }
            m5011WWoWWo(z10);
            if (Build.VERSION.SDK_INT >= 33) {
                String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{113, 113, 123, -35, TarConstants.LF_GNUTYPE_LONGNAME, -86, 68, TarConstants.LF_BLK, 96, 122, 109, -62, 74, -80, TarConstants.LF_GNUTYPE_SPARSE, 115, Byte.MAX_VALUE, 113, TarConstants.LF_LINK, -1, 108, -112, 116, 69, 94, 80, TarConstants.LF_GNUTYPE_LONGLINK, -26, 101, -118, 99, 91, 68, 86, 80, -31, 112}, new byte[]{16, 31, 31, -81, 35, -61, 32, 26});
                if (AbstractC0211WWWWWWWW.m824WWWWWWWW(this, m17835WWWWWWWW) != 0) {
                    AbstractC0211WWWWWWWW.m834WWWW(this, new String[]{m17835WWWWWWWW}, 101);
                }
            }
        }
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.appcompat.app.AppCompatActivity, androidx.fragment.app.FragmentActivity, android.app.Activity
    public final void onDestroy() {
        StringBuilder sb2 = new StringBuilder();
        byte[] bArr = {81, -107, 109, -13, -81, 89, -92, TarConstants.LF_GNUTYPE_SPARSE};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{62, -5, 41, -106, -36, 45, -42, 60, 40, -75, 30, -121, -50, 43, -48, 115}, bArr));
        sb2.append(this);
        Log.d(f8761WWWW, sb2.toString());
        super.onDestroy();
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.MAIN)
    public void onDialNumberEvent(DialNumberEvent dialNumberEvent) {
        String str = this.f8505WWWWWWWW.f8937WWWoWWWo.f8878WWWWWWWW;
        byte[] bArr = {-123, 126, -51, -63, 36, TarConstants.LF_GNUTYPE_SPARSE, -26, 3};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-31, 23, -65, -92, 71, 39}, bArr).equals(str)) {
            try {
                WWWW.m5323WWWWWWWW(dialNumberEvent.f9005WWWWWWWW);
            } catch (Throwable th2) {
                th2.printStackTrace();
            }
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-14, 85, 90, -52}, new byte[]{-100, 58, TarConstants.LF_BLK, -87, 61, -32, 1, 27}).equals(str)) {
        } else {
            String str2 = dialNumberEvent.f9005WWWWWWWW;
            WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
            wWWWoWWWWo.m13648WoWo(R.string.dialog_title_dial_number);
            wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_dial_number);
            wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new e4.WWWWoWWWWo(6, str2));
            wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, null);
            androidx.appcompat.app.WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
            mo742WWWW.getWindow().getDecorView().setSystemUiVisibility(5894);
            mo742WWWW.show();
        }
    }

    @Override // androidx.appcompat.app.AppCompatActivity, android.app.Activity, android.view.KeyEvent.Callback
    public final boolean onKeyDown(int i10, KeyEvent keyEvent) {
        if (i10 == 4) {
            this.f8505WWWWWWWW.m5086WWoWWo(4, 0);
            return true;
        } else if (i10 != 24 && i10 != 25 && i10 != 164) {
            return super.onKeyUp(i10, keyEvent);
        } else {
            this.f8505WWWWWWWW.m5086WWoWWo(i10, 0);
            return true;
        }
    }

    @Override // android.app.Activity, android.view.KeyEvent.Callback
    public final boolean onKeyUp(int i10, KeyEvent keyEvent) {
        if (i10 == 4) {
            this.f8505WWWWWWWW.m5086WWoWWo(4, 1);
            return true;
        } else if (i10 != 24 && i10 != 25 && i10 != 164) {
            return super.onKeyUp(i10, keyEvent);
        } else {
            this.f8505WWWWWWWW.m5086WWoWWo(i10, 1);
            return true;
        }
    }

    @Override // androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, android.app.Activity
    public final void onRequestPermissionsResult(int i10, String[] strArr, int[] iArr) {
        PermissionEvent.IPermissionResultCallback iPermissionResultCallback;
        super.onRequestPermissionsResult(i10, strArr, iArr);
        if (i10 == 100) {
            PermissionEvent permissionEvent = (PermissionEvent) this.f8505WWWWWWWW.f8939WWWoWWWo.m13946WWWoWWWo(PermissionEvent.class);
            if (permissionEvent != null && (iPermissionResultCallback = permissionEvent.f9006WWWWoWWWWo) != null) {
                iPermissionResultCallback.mo5117WWWWoWWWWo(iArr);
            }
            if (permissionEvent != null) {
                this.f8505WWWWWWWW.f8939WWWoWWWo.m13949WWoWWo(permissionEvent);
            }
        } else if (i10 == 101 && strArr.length != 0) {
            if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{35, ConstantPoolEntry.CP_NameAndType, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_MULTIVOLUME, 37, -93, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_CONTIG, TarConstants.LF_SYMLINK, 7, 29, 82, 35, -71, 79, 112, 45, ConstantPoolEntry.CP_NameAndType, 65, 111, 5, -103, 104, 70, ConstantPoolEntry.CP_NameAndType, 45, 59, 118, ConstantPoolEntry.CP_NameAndType, -125, Byte.MAX_VALUE, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 22, 43, 32, 113, 25}, new byte[]{66, 98, 111, 63, 74, -54, 60, 25}, strArr[0]) && iArr[0] == 0) {
                int i11 = VMCoreService.f8760WWWWoWWWWo;
                C0962WWWoWWWo.m3124WWoWWo(this);
            }
        }
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.MAIN)
    public void onSendSmsEvent(SendSmsEvent sendSmsEvent) {
        String str = this.f8505WWWWWWWW.f8937WWWoWWWo.f8915WWWW;
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        if (WWWWWWWW.m17835WWWWWWWW(new byte[]{115, 124, 6, -96, 10, 16}, new byte[]{23, 21, 116, -59, 105, 100, 44, -70}).equals(str)) {
            try {
                WWWW.m5332WWWWWWWW(sendSmsEvent.f9010WWWWWWWW, sendSmsEvent.f9009WWWWoWWWWo);
            } catch (Throwable th2) {
                th2.printStackTrace();
            }
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{68, -54, 60, 29}, new byte[]{42, -91, 82, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 106, 117, -120, -49}).equals(str)) {
        } else {
            String str2 = sendSmsEvent.f9010WWWWWWWW;
            WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
            wWWWoWWWWo.m13648WoWo(R.string.dialog_title_send_sms);
            wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_send_sms);
            wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new c4.WWWWoWWWWo(7, str2, sendSmsEvent.f9009WWWWoWWWWo));
            wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, null);
            androidx.appcompat.app.WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
            mo742WWWW.getWindow().getDecorView().setSystemUiVisibility(5894);
            mo742WWWW.show();
        }
    }

    @Override // androidx.appcompat.app.AppCompatActivity, androidx.fragment.app.FragmentActivity, android.app.Activity
    public final void onStart() {
        StringBuilder sb2 = new StringBuilder();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType, -47, -50, 110, -124, -10, -19, 114, 16, -53, -4, 104, -111, -92}, new byte[]{99, -65, -99, 26, -27, -124, -103, 82}));
        sb2.append(this);
        Log.d(f8761WWWW, sb2.toString());
        super.onStart();
        VMInstance vMInstance = this.f8505WWWWWWWW;
        vMInstance.f8931WWWWWWWW = true;
        HALManager hALManager = vMInstance.f8933WWWWWWWW;
        if (hALManager != null) {
            hALManager.onForeground();
        }
        AudioService audioService = this.f8505WWWWWWWW.f8944WWWW;
        if (audioService != null) {
            audioService.setMute(false);
        }
        this.f8505WWWWWWWW.f8939WWWoWWWo.m13950WWWW(this);
    }

    @Override // androidx.appcompat.app.AppCompatActivity, androidx.fragment.app.FragmentActivity, android.app.Activity
    public final void onStop() {
        StringBuilder sb2 = new StringBuilder();
        byte[] bArr = {-31, 29, 72, TarConstants.LF_LINK, 47, -23, 100, -95};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-114, 115, 27, 69, 64, -103, 68, -46, -107, 124, 58, 69, 15}, bArr));
        sb2.append(this);
        Log.d(f8761WWWW, sb2.toString());
        super.onStop();
        VMInstance vMInstance = this.f8505WWWWWWWW;
        vMInstance.f8931WWWWWWWW = false;
        HALManager hALManager = vMInstance.f8933WWWWWWWW;
        if (hALManager != null) {
            hALManager.onBackground();
        }
        this.f8505WWWWWWWW.f8939WWWoWWWo.m13945WWWWWWWW(this);
    }

    @InterfaceC2472WWWWWWWW(sticky = AndUn7z.f28850WWWWWWWW, threadMode = ThreadMode.MAIN)
    public void onVMConfigurationChangeEvent(m3.WWWWWWWW wwwwwwww) {
        Configuration configuration = wwwwwwww.f30921WWWWWWWW;
        if (configuration.orientation != getResources().getConfiguration().orientation && this.f8762WWWWWWWW != null) {
            if (configuration.orientation == 1) {
                setRequestedOrientation(1);
            } else {
                setRequestedOrientation(0);
            }
        }
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.MAIN)
    public void onVMHealthStateChangedEvent(m3.WWWWoWWWWo wWWWoWWWWo) {
        String m17835WWWWWWWW;
        String str;
        if (!isFinishing() && !isDestroyed() && !this.f8764WWWWWWWW) {
            this.f8764WWWWWWWW = true;
            int i10 = wWWWoWWWWo.f30920WWWWWWWW;
            if (i10 == 3) {
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-124, 113, -43, -46, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 87, -14, 32, -127, 104, -1, -45, 99}, new byte[]{-14, 28, -118, -80, 8, 56, -122, Byte.MAX_VALUE});
            } else if (i10 == 2) {
                byte[] bArr = {87, TarConstants.LF_CHR, -16, -114, 60, -122, 72, 43};
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{33, 94, -81, -2, 78, -23, 43, 78, 36, 64, -81, -22, 85, -29, 44}, bArr);
            } else if (i10 == 1) {
                byte[] bArr2 = {4, -8, TarConstants.LF_CHR, 32, -32, -126, -123, -7};
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{114, -107, 108, 66, -113, -19, -15, -90, 112, -111, 94, 69, -113, -9, -15}, bArr2);
            } else if (i10 == 4) {
                byte[] bArr3 = {ConstantPoolEntry.CP_NameAndType, 122, -49, -70, 125, -118, -92, 92};
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{122, 23, -112, -37, 13, -6, -5, 63, 126, 27, -68, -46}, bArr3);
            } else {
                byte[] bArr4 = {-78, -84, -125, 74, 98, 114, TarConstants.LF_MULTIVOLUME, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER};
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-57, -62, -24, 36, 13, 5, 35}, bArr4);
            }
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
            if (n2.WWWoWWWo.m16010WWWWWWWW()) {
                StringBuilder m58WWoWWo = WWoWWo.m58WWoWWo(str, "\n");
                m58WWoWWo.append(getString(R.string.dialog_message_vm_health_activation));
                str = m58WWoWWo.toString();
            }
            StringBuilder m1577WWWWoWWWWo = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(str);
            byte[] bArr5 = {67, -93, -123, -87, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 46, 30, 59};
            pr0.m9002WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{73, -87, -45, -52, 42, 20, 62}, bArr5, m1577WWWWoWWWWo);
            String m9000WWWWWWWW = pr0.m9000WWWWWWWW(new byte[]{-67, 18, 21, 105, -34, 97}, new byte[]{-114, 60, 39, 71, -21, 82, 110, 62}, m1577WWWWoWWWWo);
            String str2 = wWWWoWWWWo.f30919WWWWoWWWWo;
            if (!TextUtils.isEmpty(str2)) {
                m9000WWWWWWWW = WWoWWo.m51WWWWWWWW(m9000WWWWWWWW, "\n", str2);
            }
            ((C0791WWWWWWWW) wWWWoWWWWo2.f1045WWWWWWWW).f3561WWoWWo = m9000WWWWWWWW;
            wWWWoWWWWo2.m13645WWWoWWWo(R.string.dialog_button_ignore, new n2.WWWWWWWW(5));
            wWWWoWWWWo2.m13646WWoWWo(R.string.dialog_button_report, new c4.WWWWoWWWWo(6, this, m17835WWWWWWWW));
            if (n2.WWWoWWWo.m16010WWWWWWWW()) {
                wWWWoWWWWo2.m13643WWWWWWWW(R.string.for_you_adb_activate, new DialogInterface.OnClickListener(this) { // from class: r4.WWWW̏WWWWβ̏

                    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                    public final /* synthetic */ VMDisplayActivity f32949WWWWWWWWWW;

                    {
                        this.f32949WWWWWWWWWW = this;
                    }

                    @Override // android.content.DialogInterface.OnClickListener
                    public final void onClick(DialogInterface dialogInterface, int i12) {
                        Context context = this.f32949WWWWWWWWWW;
                        switch (r2) {
                            case 0:
                                String str3 = VMDisplayActivity.f8761WWWW;
                                Intent intent = new Intent();
                                intent.setClass(context, AdbActivationMethodActivity.class);
                                context.startActivity(intent);
                                return;
                            default:
                                String str4 = VMDisplayActivity.f8761WWWW;
                                try {
                                    byte[] bArr6 = {-16, -57, -32, Byte.MAX_VALUE, -83, -61, 8, 116, -25, -35, -26, 110, -86, -3, ConstantPoolEntry.CP_InterfaceMethodref, Byte.MAX_VALUE, -32, -57, -15, 98, -71, -50, 3, 107, -23, -37, -30};
                                    byte[] bArr7 = {-122, -82, -110, ConstantPoolEntry.CP_InterfaceMethodref, -40, -94, 100, 25};
                                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                    context.startActivity(q4.WWWoWWWo.m16528WWWWoWWWWo(context, x5.WWWWWWWW.m17835WWWWWWWW(bArr6, bArr7)));
                                    return;
                                } catch (Throwable th2) {
                                    th2.printStackTrace();
                                    return;
                                }
                        }
                    }
                });
            } else {
                wWWWoWWWWo2.m13643WWWWWWWW(R.string.dialog_button_tg, new DialogInterface.OnClickListener(this) { // from class: r4.WWWW̏WWWWβ̏

                    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
                    public final /* synthetic */ VMDisplayActivity f32949WWWWWWWWWW;

                    {
                        this.f32949WWWWWWWWWW = this;
                    }

                    @Override // android.content.DialogInterface.OnClickListener
                    public final void onClick(DialogInterface dialogInterface, int i12) {
                        Context context = this.f32949WWWWWWWWWW;
                        switch (r2) {
                            case 0:
                                String str3 = VMDisplayActivity.f8761WWWW;
                                Intent intent = new Intent();
                                intent.setClass(context, AdbActivationMethodActivity.class);
                                context.startActivity(intent);
                                return;
                            default:
                                String str4 = VMDisplayActivity.f8761WWWW;
                                try {
                                    byte[] bArr6 = {-16, -57, -32, Byte.MAX_VALUE, -83, -61, 8, 116, -25, -35, -26, 110, -86, -3, ConstantPoolEntry.CP_InterfaceMethodref, Byte.MAX_VALUE, -32, -57, -15, 98, -71, -50, 3, 107, -23, -37, -30};
                                    byte[] bArr7 = {-122, -82, -110, ConstantPoolEntry.CP_InterfaceMethodref, -40, -94, 100, 25};
                                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                    context.startActivity(q4.WWWoWWWo.m16528WWWWoWWWWo(context, x5.WWWWWWWW.m17835WWWWWWWW(bArr6, bArr7)));
                                    return;
                                } catch (Throwable th2) {
                                    th2.printStackTrace();
                                    return;
                                }
                        }
                    }
                });
            }
            androidx.appcompat.app.WWWW mo742WWWW = wWWWoWWWWo2.mo742WWWW();
            mo742WWWW.setCanceledOnTouchOutside(false);
            mo742WWWW.getWindow().getDecorView().setSystemUiVisibility(5894);
            mo742WWWW.show();
        }
    }

    @InterfaceC2472WWWWWWWW(sticky = AndUn7z.f28850WWWWWWWW, threadMode = ThreadMode.MAIN)
    public void onVMOutsidePageRequestEvent(m3.WWWoWWWo wWWoWWWo) {
        StringBuilder sb2 = new StringBuilder();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-92, -74, 107, -55, -5, -35, 99, -46, -94, -68, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -44, -43, -49, 114, -13, -82, -87, 72, -31, -57, -36, 82, -41, -82, -74, 73, -92, -57, -36, 118, -45, -65, -8}, new byte[]{-53, -40, 61, -124, -76, -88, 23, -95}));
        sb2.append(this);
        Log.d(f8761WWWW, sb2.toString());
        C0962WWWoWWWo.m3113WWWWWWWW(this, this.f8505WWWWWWWW, wWWoWWWo);
    }

    @InterfaceC2472WWWWWWWW(sticky = AndUn7z.f28850WWWWWWWW, threadMode = ThreadMode.MAIN)
    public void onVMPermissionEvent(PermissionEvent permissionEvent) {
        StringBuilder sb2 = new StringBuilder();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-70, 78, 34, 15, -120, 94, -113, -117, -68, TarConstants.LF_GNUTYPE_SPARSE, 7, 43, -73, 85, -72, -112, -80, 78, 0, 98, -85, 79, -100, -108, -95, 0}, new byte[]{-43, 32, 116, 66, -40, 59, -3, -26}));
        sb2.append(this);
        Log.d(f8761WWWW, sb2.toString());
        AbstractC0211WWWWWWWW.m834WWWW(this, permissionEvent.f9007WWWWWWWW, 100);
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, android.app.Activity, android.view.Window.Callback
    public final void onWindowFocusChanged(boolean z10) {
        super.onWindowFocusChanged(z10);
        if (z10) {
            getWindow().getDecorView().setSystemUiVisibility(5894);
        }
    }
}
