package com.android.vmapp;

import ad.WoWo;
import android.app.ActivityManager;
import android.app.Application;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.SharedPreferences;
import android.net.ConnectivityManager;
import android.net.NetworkRequest;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.text.TextUtils;
import android.util.Base64;
import android.util.Log;
import android.util.SparseArray;
import androidx.appcompat.app.AbstractC0813WWoWWo;
import androidx.lifecycle.AbstractC1099WoWo;
import androidx.lifecycle.C1089WWoWWo;
import c0.C1458WWWW;
import com.android.vmapp.billing.C1603WWWWWWWW;
import com.android.vmapp.billing.C1612WWoWWo;
import com.android.vmapp.vm.VMExtension;
import com.android.vmcore.KLog;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.VMManager;
import com.android.vmcore.event.VMCreationEvent;
import com.android.vmcore.event.VMDeletionEvent;
import com.google.android.gms.internal.ads.m7;
import com.google.android.gms.internal.ads.x21;
import com.google.android.gms.internal.consent_sdk.AbstractC1812WWWW;
import com.google.android.gms.internal.consent_sdk.c0;
import com.google.android.gms.internal.measurement.p1;
import com.google.firebase.Firebase;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import com.google.firebase.remoteconfig.RemoteConfigKt;
import dalvik.system.DexFile;
import ed.AbstractC2403WWWWoWWWWo;
import ed.C2427WWWWWWWW;
import ed.WW;
import eh.C2467WWWWWWWW;
import eh.InterfaceC2472WWWWWWWW;
import f2.C2489WWWWWWWW;
import f2.C2494WWWoWWWo;
import i6.C2899WWWWWWWW;
import j3.C3160WWWWWWWW;
import j3.C3164WWWWWWWW;
import j3.SharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW;
import j3.WWWW;
import j3.WWWoWWWo;
import j3.WWoWWo;
import java.io.File;
import java.io.FileOutputStream;
import java.lang.reflect.Method;
import java.util.Locale;
import java.util.Map;
import k3.C3219WWWWWWWW;
import k3.C3222WWWWWWWW;
import k3.C3226WWWWWWWW;
import k3.C3231WWoWWo;
import k3.C3233WWoWWo;
import k3.o;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import l3.C3380WWWWWWWW;
import l3.C3396WWWWWWWW;
import l3.C3398WWWWWWWW;
import lc.AbstractC3453WWWWWWWW;
import ld.C3455WWWWWWWW;
import ld.ExecutorC3454WWWWWWWW;
import n2.C3534WWWWWWWW;
import n6.C3577WWWWWWWW;
import o2.C3625WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.greenrobot.eventbus.ThreadMode;
import p020WWWWWWWW.AbstractC0211WWWWWWWW;
import q3.WWWWoWWWWo;
import r3.C3960WWoWWo;
import s2.C4086WWWWWWWW;
import t4.ComponentCallbacksC4221WWWWWWWW;
import u6.r;
import wd.WWWWWWWW;
/* loaded from: classes.dex */
public class VMApp extends Application {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public static VMApp f8424WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final SparseArray f8425WWWWoWWWWo = new SparseArray();

    /* JADX WARN: Code restructure failed: missing block: B:44:0x00b1, code lost:
        if (r2.exists() == false) goto L48;
     */
    @Override // android.content.ContextWrapper
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final void attachBaseContext(Context context) {
        File file;
        Method method;
        super.attachBaseContext(context);
        if (Build.VERSION.SDK_INT >= 28) {
            String[] strArr = {"L"};
            Object obj = WWWWWWWW.f35180WWWWWWWW;
            if (obj != null && (method = WWWWWWWW.f35179WWWWoWWWWo) != null) {
                try {
                    method.invoke(obj, strArr);
                } catch (Throwable unused) {
                }
            }
            byte[] decode = Base64.decode("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TGFuZHJvaWQvdXRpbC9Mb2c7ABFMamF2YS9sYW5nL0NsYXNzOwAUTGphdmEvbGFuZy9DbGFzczwqPjsAEkxqYXZhL2xhbmcvT2JqZWN0OwASTGphdmEvbGFuZy9TdHJpbmc7ABJMamF2YS9sYW5nL1N5c3RlbTsAFUxqYXZhL2xhbmcvVGhyb3dhYmxlOwAaTGphdmEvbGFuZy9yZWZsZWN0L01ldGhvZDsAJkxtZS93ZWlzaHUvZnJlZXJlZmxlY3Rpb24vQnVpbGRDb25maWc7ACVMbWUvd2Vpc2h1L3JlZmxlY3Rpb24vQm9vdHN0cmFwQ2xhc3M7ACFMbWUvd2Vpc2h1L3JlZmxlY3Rpb24vUmVmbGVjdGlvbjsAClJlZmxlY3Rpb24AD1JlZmxlY3Rpb24uamF2YQAHU0RLX0lOVAADVEFHAAFWAAxWRVJTSU9OX0NPREUADFZFUlNJT05fTkFNRQACVkwAAVoAAlpMABJbTGphdmEvbGFuZy9DbGFzczsAE1tMamF2YS9sYW5nL09iamVjdDsAE1tMamF2YS9sYW5nL1N0cmluZzsAB2NvbnRleHQAF2RhbHZpay5zeXN0ZW0uVk1SdW50aW1lAAFlAAZleGVtcHQACWV4ZW1wdEFsbAAHZm9yTmFtZQAPZnJlZS1yZWZsZWN0aW9uABJnZXRBcHBsaWNhdGlvbkluZm8AEWdldERlY2xhcmVkTWV0aG9kAApnZXRSdW50aW1lAAZpbnZva2UAC2xvYWRMaWJyYXJ5ABhtZS53ZWlzaHUuZnJlZXJlZmxlY3Rpb24ABm1ldGhvZAAHbWV0aG9kcwAZcmVmbGVjdCBib290c3RyYXAgZmFpbGVkOgAHcmVsZWFzZQAKc1ZtUnVudGltZQAWc2V0SGlkZGVuQXBpRXhlbXB0aW9ucwAQdGFyZ2V0U2RrVmVyc2lvbgAEdGhpcwAGdW5zZWFsAAx1bnNlYWxOYXRpdmUADnZtUnVudGltZUNsYXNzAAYABw4AFgAHDmr/AwEyCwEVEAMCNQvwBAREBhcBEg8DAzYLARsPqQUCBQMFBBkeAwAvCgAOAAcOACwBOgcOADYBOwcsnRriAQEDAC8KHgBIAAcOAA0ABw4AEwEtBx1yGWtaAAYXOBc8HxcABAEXAQEXBgEXHwYAAQACGQEZARkBGQEZARkGgYAEiAcDAAUACBoBCgEKB4iABKAHAYGABLQJAQnMCQGJAfQJAQnMCgEAAwALGgyBgAT4CgEJkAsBigIAAAAADgAAAAAAAAABAAAAAAAAAAEAAABEAAAAcAAAAAIAAAATAAAAgAEAAAMAAAALAAAAzAEAAAQAAAAMAAAAUAIAAAUAAAAPAAAAsAIAAAYAAAADAAAAKAMAAAEgAAAIAAAAiAMAAAEQAAAHAAAA3AUAAAIgAABEAAAAFgYAAAMgAAAIAAAAAwoAAAUgAAADAAAAZgoAAAAgAAADAAAAeAoAAAAQAAABAAAAyAoAAA==", 2);
            if (context != null) {
                file = context.getCodeCacheDir();
            } else {
                String property = System.getProperty("java.io.tmpdir");
                if (!TextUtils.isEmpty(property)) {
                    File file2 = new File(property);
                    if (file2.exists()) {
                        file = file2;
                    }
                }
                file = null;
            }
            if (file != null) {
                File file3 = new File(file, System.currentTimeMillis() + ".dex");
                try {
                    FileOutputStream fileOutputStream = new FileOutputStream(file3);
                    fileOutputStream.write(decode);
                    fileOutputStream.close();
                    try {
                        file3.setReadOnly();
                    } catch (Throwable unused2) {
                    }
                    ((Boolean) new DexFile(file3).loadClass("me.weishu.reflection.BootstrapClass", null).getDeclaredMethod("exemptAll", null).invoke(null, null)).booleanValue();
                } catch (Throwable th2) {
                    try {
                        th2.printStackTrace();
                    } finally {
                        if (file3.exists()) {
                            file3.delete();
                        }
                    }
                }
            }
        }
        f8424WWWWWWWWWW = this;
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Type inference failed for: r1v16, types: [androidx.lifecycle.WWoᆑWWoӁᆑ, androidx.lifecycle.WoოWo੍ო] */
    /* JADX WARN: Type inference failed for: r1v33, types: [o2.WoڄWoᄴڄ, java.lang.Object] */
    /* JADX WARN: Type inference failed for: r7v14, types: [tc.WWWWެWWWWܕެ, lc.WWWWӈWWWWीӈ] */
    /* JADX WARN: Type inference failed for: r9v2, types: [java.util.Map, java.lang.Object] */
    /* JADX WARN: Type inference failed for: r9v4, types: [java.lang.Object, com.google.firebase.remoteconfig.ConfigUpdateListener] */
    @Override // android.app.Application
    public final void onCreate() {
        C3577WWWWWWWW c3577wwwwwwww;
        Intent intent;
        int i10;
        Context context;
        Context context2;
        super.onCreate();
        if (WWWWoWWWWo.f32526WWWWWWWW == null) {
            if (Build.VERSION.SDK_INT >= 24) {
                context2 = createDeviceProtectedStorageContext();
            } else {
                context2 = this;
            }
            WWWWoWWWWo.f32526WWWWWWWW = new q3.WWWWWWWW(context2, 0).getSharedPreferences(WWWWoWWWWo.f32522WWWWWWWW, 0);
        }
        Locale m16514WWWWoWWWWo = WWWWoWWWWo.m16514WWWWoWWWWo();
        Locale locale = WWWoWWWo.f28919WWWWoWWWWo;
        AbstractC1812WWWW.m10962WoWo(m16514WWWWoWWWWo);
        AbstractC0813WWoWWo.m2406WWWWWWWW(WWWWoWWWWo.m16516WWWWWWWW());
        String m16522WWoWWo = WWWWoWWWWo.m16522WWoWWo();
        String str = WWWW.f28925WWWWoWWWWo;
        p1.m11587WWWWWWWW(m16522WWoWWo);
        C3396WWWWWWWW.f30437WWWWWWWW.getClass();
        FirebaseRemoteConfig remoteConfig = RemoteConfigKt.getRemoteConfig(Firebase.INSTANCE);
        remoteConfig.setDefaultsAsync((Map<String, Object>) C3396WWWWWWWW.f30434WWWWWWWWWW);
        remoteConfig.fetchAndActivate().mo16241WWWWWWWW(new C2899WWWWWWWW(5, new WoWo(12)));
        remoteConfig.addOnConfigUpdateListener(new Object());
        C2494WWWoWWWo c2494WWWoWWWo = new C2494WWWoWWWo();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        c2494WWWoWWWo.f26918WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{92, -120, -67, -106, -65, -58, Byte.MIN_VALUE, 61, TarConstants.LF_GNUTYPE_LONGLINK, -110, -69, -121, -72}, new byte[]{42, -31, -49, -30, -54, -89, -20, 80});
        c2494WWWoWWWo.f26917WWWWoWWWWo = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{115, 63, -117, 27, -112, 32, 82, TarConstants.LF_BLK, 121, 40, -96, 95, -98, TarConstants.LF_CONTIG, 67, TarConstants.LF_CHR, 85, 37, -22, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -66, 44, 95, 25, 118, 2, -93, 86, -54, 61, 67, 102}, new byte[]{26, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -45, 111, -6, 100, 19, 82});
        m7.f15885WWWW = new m7(this, new C2489WWWWWWWW(c2494WWWoWWWo.f26918WWWWWWWW, c2494WWWoWWWo.f26917WWWWoWWWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-54, 35, 63, -43, TarConstants.LF_FIFO, -26, -2, 109, -35, 57, 57, -60, TarConstants.LF_LINK}, new byte[]{-68, 74, TarConstants.LF_MULTIVOLUME, -95, 67, -121, -110, 0})));
        if (C1603WWWWWWWW.f8434WWWoWWWo == null) {
            C1603WWWWWWWW c1603wwwwwwww = new C1603WWWWWWWW(this);
            C1603WWWWWWWW.f8434WWWoWWWo = c1603wwwwwwww;
            c1603wwwwwwww.m4896WWWWoWWWWo();
            C1603WWWWWWWW c1603wwwwwwww2 = C1603WWWWWWWW.f8434WWWoWWWo;
            c1603wwwwwwww2.getClass();
            try {
                ((ConnectivityManager) c1603wwwwwwww2.f8438WWWWoWWWWo.getSystemService(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-2, -46, 1, -122, TarConstants.LF_FIFO, -81, -126, 111, -21, -44, 27, -111}, new byte[]{-99, -67, 111, -24, TarConstants.LF_GNUTYPE_SPARSE, -52, -10, 6}))).registerNetworkCallback(new NetworkRequest.Builder().build(), new com.android.vmapp.billing.WWWoWWWo(0, c1603wwwwwwww2));
            } catch (Exception e10) {
                e10.printStackTrace();
            }
        }
        C1612WWoWWo.f8473WWWWoWWWWo.getClass();
        byte[] bArr = {73, 4, -86, -20, -31, ConstantPoolEntry.CP_InterfaceMethodref, -111, -119};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{40, 116, -38}, bArr);
        C1612WWoWWo.f8482WWWW = this;
        C3960WWoWWo.f32923WWWWWWWW.getClass();
        AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(C3960WWoWWo.f32924WWWWWWWW, null, new AbstractC3453WWWWWWWW(2, null), 3);
        C3396WWWWWWWW c3396wwwwwwww = C3396WWWWWWWW.f30437WWWWWWWW;
        C1458WWWW c1458wwww = new C1458WWWW(24);
        c3396wwwwwwww.getClass();
        C3396WWWWWWWW.m15737WWWWWWWW(c1458wwww);
        if (C3380WWWWWWWW.f30346WWWWWWWW == null) {
            C3380WWWWWWWW c3380wwwwwwww = new C3380WWWWWWWW(this);
            C3380WWWWWWWW.f30346WWWWWWWW = c3380wwwwwwww;
            try {
                IntentFilter intentFilter = new IntentFilter(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-16, -33, 17, 28, 81, TarConstants.LF_CHR, -43, 116, -8, -33, 1, ConstantPoolEntry.CP_InterfaceMethodref, 80, 46, -97, 59, -14, -59, 28, 1, 80, 116, -31, 27, -46, -6, TarConstants.LF_BLK, 41, 123, 5, -29, 31, -36, -2, 35, 43, 122}, new byte[]{-111, -79, 117, 110, 62, 90, -79, 90}));
                intentFilter.addAction(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType, -21, 109, 33, 41, TarConstants.LF_DIR, -60, 30, 4, -21, 125, TarConstants.LF_FIFO, 40, 40, -114, 81, 14, -15, 96, 60, 40, 114, -16, 113, 46, -50, 72, 20, 3, 3, -31, 116, 41, -64, TarConstants.LF_MULTIVOLUME}, new byte[]{109, -123, 9, TarConstants.LF_GNUTYPE_SPARSE, 70, 92, -96, TarConstants.LF_NORMAL}));
                intentFilter.addAction(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{62, -115, -42, 23, -17, -53, 60, 97, TarConstants.LF_FIFO, -115, -58, 0, -18, -42, 118, 46, 60, -105, -37, 10, -18, -116, 8, 14, 28, -88, -13, 34, -59, -3, 10, 10, 15, -81, -13, 38, -59, -26}, new byte[]{95, -29, -78, 101, Byte.MIN_VALUE, -94, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 79}));
                intentFilter.addDataScheme(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-22, -49, -106, -45, 89, 99, 40}, new byte[]{-102, -82, -11, -72, 56, 4, TarConstants.LF_MULTIVOLUME, 24}));
                c3380wwwwwwww.f30349WWWWWWWW.registerReceiver(c3380wwwwwwww.f30353WWWWWWWW, intentFilter);
            } catch (Throwable th2) {
                th2.printStackTrace();
            }
        }
        C3219WWWWWWWW.f29158WWWWWWWW.getClass();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{123, -87, -62}, new byte[]{26, -39, -78, 112, 106, 41, 86, -36});
        StringBuilder sb2 = new StringBuilder();
        sb2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{35, 97, 126, TarConstants.LF_DIR, 80, -17, -109, 26, ConstantPoolEntry.CP_InterfaceMethodref, 108, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 62, 89, -86, -14, TarConstants.LF_CHR, 23, 46, 66, 22, 119, -86, -27, TarConstants.LF_SYMLINK, 22, 125, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 61, 82, -80, -109}, new byte[]{100, 14, 17, 82, 60, -118, -77, 87}));
        r.m17495WWWWWWWW();
        String[] split = TextUtils.split("24.8.0", "\\.");
        if (split.length != 3) {
            c3577wwwwwwww = new C3577WWWWWWWW(0, 0, 0);
        } else {
            try {
                c3577wwwwwwww = new C3577WWWWWWWW(Integer.parseInt(split[0]), Integer.parseInt(split[1]), Integer.parseInt(split[2]));
            } catch (NumberFormatException unused) {
                c3577wwwwwwww = new C3577WWWWWWWW(0, 0, 0);
            }
        }
        sb2.append(c3577wwwwwwww);
        Log.d(C3219WWWWWWWW.f29157WWWWoWWWWo, sb2.toString());
        C3219WWWWWWWW.f29161WWWoWWWo = this;
        c0 mo10850WWWWoWWWWo = com.google.android.gms.internal.consent_sdk.WWWWWWWW.m10849WWWWWWWW(this).mo10850WWWWoWWWWo();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(mo10850WWWWoWWWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{57, -55, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 121, -99, -39, 106, -51, TarConstants.LF_NORMAL, -40, 101, 84, -108, -40, 107, -59, 63, -40, 69, 85, -100, -97, TarConstants.LF_CONTIG, -122, 112, -123}, new byte[]{94, -84, 44, 58, -14, -73, 25, -88}));
        C3219WWWWWWWW.f29159WWWWWWWW = mo10850WWWWoWWWWo;
        C3222WWWWWWWW c3222wwwwwwww = C3222WWWWWWWW.f29195WWWWoWWWWo;
        c3222wwwwwwww.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-53, 96, -41}, new byte[]{-86, 16, -89, -99, -89, -83, 17, -78});
        Log.d(C3222WWWWWWWW.f29194WWWWWWWWWW, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-18, -71, 47, -7, 124}, new byte[]{-121, -41, 70, -115, 82, 7, 26, 99}));
        registerActivityLifecycleCallbacks(c3222wwwwwwww);
        C1089WWoWWo.f5668WW.f5675WWoWWo.mo3500WWWWWWWW(c3222wwwwwwww);
        C3233WWoWWo.f29240WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{125, 21, 38}, new byte[]{28, 101, 86, 116, -35, 109, 30, 112});
        Log.d(C3233WWoWWo.f29239WWWWoWWWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{6, TarConstants.LF_CONTIG, -22, -14, 81}, new byte[]{111, 89, -125, -122, Byte.MAX_VALUE, ConstantPoolEntry.CP_InterfaceMethodref, -86, -8}));
        Application.ActivityLifecycleCallbacks activityLifecycleCallbacks = o.f29253WWWWoWWWWo;
        activityLifecycleCallbacks.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-8, 37, -47}, new byte[]{-103, 85, -95, -13, 31, 18, -99, -27});
        Log.d(o.f29252WWWWWWWWWW, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-53, -54, -86, 125, 33}, new byte[]{-94, -92, -61, 9, 15, -73, 9, -102}));
        registerActivityLifecycleCallbacks(activityLifecycleCallbacks);
        C3231WWoWWo.f29228WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{121, -75, -58}, new byte[]{24, -59, -74, -55, 111, -71, -27, 18});
        Log.d(C3231WWoWWo.f29227WWWWoWWWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-113, -93, 20, -112, 89}, new byte[]{-26, -51, 125, -28, 119, 94, TarConstants.LF_BLK, -54}));
        Application.ActivityLifecycleCallbacks activityLifecycleCallbacks2 = C3226WWWWWWWW.f29210WWWWoWWWWo;
        activityLifecycleCallbacks2.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{113, -39, -36}, new byte[]{16, -87, -84, 112, -6, -36, -52, -46});
        Log.d(C3226WWWWWWWW.f29209WWWWWWWWWW, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{80, -120, -67, -63, TarConstants.LF_CONTIG}, new byte[]{57, -26, -44, -75, 25, 14, -59, 61}));
        registerActivityLifecycleCallbacks(activityLifecycleCallbacks2);
        String str2 = VMManager.f8949WWWoWWWo;
        synchronized (VMManager.class) {
            if (VMManager.f8948WWWWWWWW == null) {
                String str3 = VMManager.f8949WWWoWWWo;
                byte[] bArr2 = {-97, TarConstants.LF_CONTIG, TarConstants.LF_MULTIVOLUME, -80, -120, -36, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 106, -85, 63, 71, -80, -110, -26, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 112, -87, 126, 79, -80, -98, -13, 25, 116, -67};
                byte[] bArr3 = {-60, 94, 35, -39, -4, -127, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 6};
                StringFog.f8859WWWWWWWW.getClass();
                Log.i(str3, x5.WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                System.loadLibrary(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-26, -127}, new byte[]{-112, -20, -8, 31, -8, ConstantPoolEntry.CP_NameAndType, 66, -22}));
                VMManager.f8948WWWWWWWW = new VMManager(this);
            } else {
                byte[] bArr4 = {-106, 89, -122, 82, 30, 118, 47, -100, -78, TarConstants.LF_BLK, -86, 95, 2, 114, 41, -99, -71, TarConstants.LF_BLK, -94, 93, 25, 99, 33, -104, -84, 125, -79, 86, 20};
                byte[] bArr5 = {-64, 20, -53, TarConstants.LF_CHR, 112, 23, 72, -7};
                StringFog.f8859WWWWWWWW.getClass();
                throw new RuntimeException(x5.WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5));
            }
        }
        for (VMInstance vMInstance : VMManager.m5102WWWWWWWW().m5108WWoWWo()) {
            this.f8425WWWWoWWWWo.put(vMInstance.f8937WWWoWWWo.f8866WWWWWWWW, new VMExtension(this, vMInstance));
        }
        if (!t8.WWWWWWWW.m17201WWWWWWWW(this).m4644WWWWoWWWWo() && (i10 = Build.VERSION.SDK_INT) >= 31) {
            C4086WWWWWWWW.f33575WWWWWWWW = this;
            if (s2.WWWWWWWW.f33572WWWWoWWWWo == null) {
                if (i10 >= 24) {
                    context = createDeviceProtectedStorageContext();
                } else {
                    context = this;
                }
                s2.WWWWWWWW.f33572WWWWoWWWWo = new q3.WWWWWWWW(context, 1).getSharedPreferences(s2.WWWWWWWW.f33573WWWWWWWW, 0);
            }
            if (o2.WoWo.f31584WWWWWWWW == null) {
                ?? obj = new Object();
                try {
                    obj.f31591WWWoWWWo = this;
                    obj.f31594WW = new Handler(Looper.getMainLooper());
                    obj.f31586WWWWWWWWWW = false;
                    obj.f31587WWWWoWWWWo = false;
                    x21 x21Var = new x21(s2.WWWWWWWW.f33572WWWWoWWWWo);
                    byte[] bArr6 = {-14, 73, 39, -40, -8, 89, 73, 95, -27, TarConstants.LF_GNUTYPE_SPARSE, 33, -55, -1};
                    byte[] bArr7 = {-124, 32, 85, -84, -115, 56, 37, TarConstants.LF_SYMLINK};
                    C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                    obj.f31595WoWo = new C3625WWWWWWWW(x21Var, x5.WWWWWWWW.m17835WWWWWWWW(bArr6, bArr7));
                } catch (Throwable th3) {
                    th3.printStackTrace();
                }
                o2.WoWo.f31584WWWWWWWW = obj;
            }
        }
        ComponentCallbacksC4221WWWWWWWW componentCallbacksC4221WWWWWWWW = ComponentCallbacksC4221WWWWWWWW.f34042WWWWoWWWWo;
        componentCallbacksC4221WWWWWWWW.getClass();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{91, 2, 4}, new byte[]{58, 114, 116, 28, 102, -53, -84, -32});
        C1089WWoWWo.f5668WW.f5675WWoWWo.mo3500WWWWWWWW(componentCallbacksC4221WWWWWWWW);
        ComponentCallbacksC4221WWWWWWWW.f34043WWWWWWWW = this;
        if (!ComponentCallbacksC4221WWWWWWWW.f34046WWoWWo) {
            try {
                AbstractC0211WWWWWWWW.m830WWWWWWWW(this, ComponentCallbacksC4221WWWWWWWW.f34045WWWWWWWW, new IntentFilter(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-38, 56, 110, -39, -17, 109, -50, -125, -46, 56, 126, -50, -18, 112, -124, -52, -40, 34, 99, -60, -18, 42, -23, -31, -12, 5, 79, -12, -45, 93, -7, -7, -2, 27, 85, -17, -55, 69, -26, -30, -4, 5}, new byte[]{-69, 86, 10, -85, Byte.MIN_VALUE, 4, -86, -83})), 4);
                ComponentCallbacksC4221WWWWWWWW.f34046WWoWWo = true;
            } catch (Throwable th4) {
                th4.printStackTrace();
            }
        }
        registerComponentCallbacks(componentCallbacksC4221WWWWWWWW);
        ComponentCallbacksC4221WWWWWWWW.f34047WWWW = new AbstractC1099WoWo(ComponentCallbacksC4221WWWWWWWW.m17174WWWoWWWo());
        registerActivityLifecycleCallbacks(new C3160WWWWWWWW(1));
        SharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW sharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW = SharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW.f28903WWWWoWWWWo;
        sharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW.getClass();
        byte[] bArr8 = {-10, -35, -35, TarConstants.LF_LINK, -39, -88, -108, 38};
        x5.WWWWWWWW wwwwwwww = C3164WWWWWWWW.f28918WWWWWWWW;
        wwwwwwww.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-105, -83, -83}, bArr8);
        SharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW.f28905WWWWWWWW = this;
        C1089WWoWWo.f5668WW.f5675WWoWWo.mo3500WWWWWWWW(sharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW);
        VMApp vMApp = SharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW.f28905WWWWWWWW;
        if (vMApp != null) {
            vMApp.registerActivityLifecycleCallbacks(new C3160WWWWWWWW(0));
            SharedPreferences sharedPreferences = WWWWoWWWWo.f32526WWWWWWWW;
            C2427WWWWWWWW c2427wwwwwwww = C2427WWWWWWWW.f26690WWWWoWWWWo;
            C3455WWWWWWWW c3455wwwwwwww = WW.f26728WWWWWWWW;
            AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(c2427wwwwwwww, ExecutorC3454WWWWWWWW.f30758WWWWWWWW, new WWoWWo(sharedPreferences, null), 2);
            sharedPreferences.registerOnSharedPreferenceChangeListener(sharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW);
            try {
                Intent launchIntentForPackage = getPackageManager().getLaunchIntentForPackage(getPackageName());
                byte[] bArr9 = {-70, 45, TarConstants.LF_DIR, 4, 74, 19, TarConstants.LF_SYMLINK, 62};
                wwwwwwww.getClass();
                for (ActivityManager.AppTask appTask : ((ActivityManager) getSystemService(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-37, 78, 65, 109, 60, 122, 70, 71}, bArr9))).getAppTasks()) {
                    intent = appTask.getTaskInfo().baseIntent;
                    if (!intent.getComponent().equals(launchIntentForPackage.getComponent())) {
                        appTask.finishAndRemoveTask();
                    } else {
                        appTask.moveToFront();
                    }
                }
            } catch (Throwable th5) {
                th5.printStackTrace();
            }
            C2467WWWWWWWW.m13936WWWWoWWWWo().m13950WWWW(this);
            return;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{2, 7, -7}, new byte[]{99, 119, -119, -17, -111, 20, ConstantPoolEntry.CP_InterfaceMethodref, -112}));
        throw null;
    }

    @Override // android.app.Application
    public final void onTerminate() {
        C2467WWWWWWWW.m13936WWWWoWWWWo().m13945WWWWWWWW(this);
        super.onTerminate();
    }

    @Override // android.app.Application, android.content.ComponentCallbacks2
    public final void onTrimMemory(int i10) {
        super.onTrimMemory(i10);
        byte[] bArr = {-52, 39, -91, 108, -51, -52, TarConstants.LF_BLK, -117};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        String m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-102, 106, -28, 28, -67}, bArr);
        KLog.m5043WWWWWWWW(m17835WWWWWWWW, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-53, Byte.MIN_VALUE, 86, 114, 89, Byte.MAX_VALUE, -36, 9, -55, -127, 112, 121, 16}, new byte[]{-92, -18, 2, 0, TarConstants.LF_NORMAL, 18, -111, 108}) + i10);
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.MAIN)
    public void onVMCreationEvent(VMCreationEvent vMCreationEvent) {
        this.f8425WWWWoWWWWo.put(vMCreationEvent.f9013WWWWWWWW.f8937WWWoWWWo.f8866WWWWWWWW, new VMExtension(this, vMCreationEvent.f9013WWWWWWWW));
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.MAIN)
    public void onVMDeletionEvent(VMDeletionEvent vMDeletionEvent) {
        SparseArray sparseArray = this.f8425WWWWoWWWWo;
        VMExtension vMExtension = (VMExtension) sparseArray.get(vMDeletionEvent.f9014WWWWWWWW.f8937WWWoWWWo.f8866WWWWWWWW);
        if (vMExtension != null) {
            vMExtension.f8766WWWWoWWWWo.f8939WWWoWWWo.m13945WWWWWWWW(vMExtension);
            ClipboardManager clipboardManager = vMExtension.f8768WWWWWWWW;
            if (clipboardManager != null) {
                clipboardManager.removePrimaryClipChangedListener(vMExtension);
            }
            try {
                vMExtension.f8767WWWWWWWW.unregisterReceiver(vMExtension);
            } catch (Throwable th2) {
                th2.printStackTrace();
            }
        }
        VMInstance vMInstance = vMDeletionEvent.f9014WWWWWWWW;
        sparseArray.remove(vMInstance.f8937WWWoWWWo.f8866WWWWWWWW);
        SparseArray sparseArray2 = C3398WWWWWWWW.f30474WWWWWWWW;
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        C3398WWWWWWWW c3398wwwwwwww = (C3398WWWWWWWW) sparseArray2.get(vMConfig.f8866WWWWWWWW);
        if (c3398wwwwwwww != null) {
            c3398wwwwwwww.f30479WWWoWWWo.getLooper().quit();
        }
        sparseArray2.remove(vMConfig.f8866WWWWWWWW);
    }
}
