package com.android.libadb.ui;

import a3.WoWo;
import android.annotation.TargetApi;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.RemoteInput;
import android.app.Service;
import android.content.Intent;
import android.os.Build;
import android.os.Bundle;
import android.os.IBinder;
import android.util.Log;
import com.android.libadb.ui.AdbActivationTutorialActivity;
import com.android.libadb.ui.AdbPairingService;
import com.clone.android.dual.space.R;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import d2.AbstractC2285WWWWWWWW;
import ed.AbstractC2403WWWWoWWWWo;
import ed.C2427WWWWWWWW;
import ed.WW;
import java.util.ArrayList;
import java.util.HashSet;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import l2.C3365WWWWWWWW;
import ld.C3455WWWWWWWW;
import ld.ExecutorC3454WWWWWWWW;
import me.AbstractC3506WWWWWWWW;
import n2.C3534WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p020WWWWWWWW.AbstractC0241WWoWWo;
import p020WWWWWWWW.C0204WWWWoWWWWo;
import p020WWWWWWWW.C0242WWoWWo;
import p020WWWWWWWW.C0243WWoWWo;
import p020WWWWWWWW.C0257WoWo;
import p2.C3817WWWWWWWW;
import tc.WWWWWWWW;
@TargetApi(30)
/* loaded from: classes.dex */
public final class AdbPairingService extends Service {

    /* renamed from: WWWWoᆣWWWWoϿᆣ  reason: contains not printable characters */
    public static final String f8283WWWWoWWWWo;

    /* renamed from: WWWWၗWWWW३ၗ  reason: contains not printable characters */
    public static final C3365WWWWWWWW f8284WWWWWWWW;

    /* renamed from: WWWWᄳWWWW़ᄳ  reason: contains not printable characters */
    public static final String f8285WWWWWWWW;

    /* renamed from: WWWoၙWWWo੮ၙ  reason: contains not printable characters */
    public static final String f8286WWWoWWWo;

    /* renamed from: WWoᆑWWoӁᆑ  reason: contains not printable characters */
    public static final String f8287WWoWWo;

    /* renamed from: WWၚஊၚ  reason: contains not printable characters */
    public static final String f8288WW;

    /* renamed from: WoოWo੍ო  reason: contains not printable characters */
    public static final String f8289WoWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final Object f8292WWWWWWWW;

    /* renamed from: WWWWཀྵWWWWࠤཀྵ  reason: contains not printable characters */
    public final Object f8293WWWWWWWW;

    /* renamed from: WWoၑWWoړၑ  reason: contains not printable characters */
    public final Object f8294WWoWWo;

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public final Object f8295WWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final WoWo f8291WWWWoWWWWo = new WoWo(2, this);

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final Object f8290WWWWWWWWWW = AbstractC3506WWWWWWWW.m15931WWWWWWWW(new WWWWWWWW(this) { // from class: p2.WWoϫWWoӉϫ

        /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
        public final /* synthetic */ AdbPairingService f32192WWWWWWWWWW;

        {
            this.f32192WWWWWWWWWW = this;
        }

        /* JADX WARN: Type inference failed for: r1v27, types: [java.lang.Object, fc.WWWWҍWWWWּҍ] */
        @Override // tc.WWWWWWWW
        public final Object invoke() {
            int i10;
            PendingIntent foregroundService;
            int color;
            int color2;
            int i11 = 0;
            AdbPairingService adbPairingService = this.f32192WWWWWWWWWW;
            switch (r2) {
                case 0:
                    AdbPairingService.f8284WWWWWWWW.getClass();
                    Intent intent = new Intent(adbPairingService, AdbPairingService.class);
                    C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                    Intent action = intent.setAction(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{112, -68, 5, 45}, new byte[]{3, -56, 106, 93, 43, Byte.MIN_VALUE, -58, -55}));
                    AbstractC3339WWWWWWWW.m15429WWWWWWWW(action, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{113, 33, -45, 45, -104, -81, 64, 44, 108, 108, -119, 66, -43, -14}, new byte[]{2, 68, -89, 108, -5, -37, 41, 67}));
                    if (Build.VERSION.SDK_INT >= 31) {
                        i11 = 67108864;
                    }
                    return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_stop_searching), PendingIntent.getService(adbPairingService, 2, action, i11)).m961WWWWWWWW();
                case 1:
                    AdbPairingService.f8284WWWWWWWW.getClass();
                    Intent m15676WWWW = C3365WWWWWWWW.m15676WWWW(adbPairingService);
                    if (Build.VERSION.SDK_INT >= 31) {
                        i11 = 67108864;
                    }
                    return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_retry), PendingIntent.getService(adbPairingService, 3, m15676WWWW, i11)).m961WWWWWWWW();
                case 2:
                    C3365WWWWWWWW c3365wwwwwwww = AdbPairingService.f8284WWWWWWWW;
                    HashSet hashSet = new HashSet();
                    Bundle bundle = new Bundle();
                    String str = AdbPairingService.f8283WWWWoWWWWo;
                    if (str != null) {
                        C0257WoWo c0257WoWo = new C0257WoWo(str, adbPairingService.getString(R.string.dialog_adb_pairing_paring_code), null, true, 0, bundle, hashSet);
                        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, -98, 2, 90, -76, -92, 87, -83}, new byte[]{-70, -21, 108, 114, -102, -118, 121, -124});
                        Intent m15670WWWWWWWW = C3365WWWWWWWW.m15670WWWWWWWW(AdbPairingService.f8284WWWWWWWW, adbPairingService, -1);
                        if (Build.VERSION.SDK_INT >= 31) {
                            i10 = 167772160;
                        } else {
                            i10 = 134217728;
                        }
                        foregroundService = PendingIntent.getForegroundService(adbPairingService, 1, m15670WWWWWWWW, i10);
                        C0242WWoWWo c0242WWoWWo = new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_input_paring_code), foregroundService);
                        c0242WWoWWo.f1253WWoWWo = new ArrayList();
                        c0242WWoWWo.f1253WWoWWo.add(c0257WoWo);
                        return c0242WWoWWo.m961WWWWWWWW();
                    }
                    throw new IllegalArgumentException("Result key can't be null");
                case 3:
                    C3365WWWWWWWW c3365wwwwwwww2 = AdbPairingService.f8284WWWWWWWW;
                    Intent intent2 = new Intent(adbPairingService, AdbActivationTutorialActivity.class);
                    intent2.setFlags(131072);
                    if (Build.VERSION.SDK_INT >= 31) {
                        i11 = 67108864;
                    }
                    return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_activate_now), PendingIntent.getActivity(adbPairingService, 3, intent2, i11)).m961WWWWWWWW();
                case 4:
                    C0204WWWWoWWWWo c0204WWWWoWWWWo = new C0204WWWWoWWWWo(adbPairingService, AdbPairingService.f8286WWWoWWWo);
                    color = adbPairingService.getColor(R.color.notification);
                    c0204WWWWoWWWWo.f1145WWWWWWWW = color;
                    c0204WWWWoWWWWo.f1132WWWWoWWWWo.icon = R.drawable.ic_wadb;
                    c0204WWWWoWWWWo.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(adbPairingService.getString(R.string.notification_adb_pairing_searching_for_service_title));
                    C0243WWoWWo c0243WWoWWo = (C0243WWoWWo) adbPairingService.f8290WWWWWWWWWW.getValue();
                    if (c0243WWoWWo != null) {
                        c0204WWWWoWWWWo.f1130WWWWoWWWWo.add(c0243WWoWWo);
                    }
                    return c0204WWWWoWWWWo.m784WWWWWWWW();
                default:
                    C0204WWWWoWWWWo c0204WWWWoWWWWo2 = new C0204WWWWoWWWWo(adbPairingService, AdbPairingService.f8286WWWoWWWo);
                    color2 = adbPairingService.getColor(R.color.notification);
                    c0204WWWWoWWWWo2.f1145WWWWWWWW = color2;
                    c0204WWWWoWWWWo2.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(adbPairingService.getString(R.string.notification_adb_pairing_working_title));
                    c0204WWWWoWWWWo2.f1132WWWWoWWWWo.icon = R.drawable.ic_wadb;
                    return c0204WWWWoWWWWo2.m784WWWWWWWW();
            }
        }
    });

    static {
        byte[] bArr = {-89, 117, 67, 73, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 40, 64, 40};
        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
        f8286WWWoWWWo = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-58, 17, 33, 22, 23, 73, 41, 90, -50, 27, 36}, bArr);
        f8288WW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{73, 45, 44, -75, TarConstants.LF_LINK, -78, 119, 82, 102, 46, 29, Byte.MIN_VALUE, 34, -83, 108, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 109}, new byte[]{8, 73, 78, -27, 80, -37, 5, 59});
        f8289WoWo = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{117, 2, -25, -95, -63}, new byte[]{6, 118, -122, -45, -75, 44, 31, 86});
        f8285WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{42, 101, -71, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{89, 17, -42, 59, -125, TarConstants.LF_CONTIG, -18, 78});
        f8287WWoWWo = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-81, -32, -23, 86, 84}, new byte[]{-35, -123, -103, 58, 45, 29, 63, 86});
        f8283WWWWoWWWWo = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{126, -28, 109, Byte.MIN_VALUE, 40, -20, 79, 43, 97, -31, 122}, new byte[]{14, -123, 31, -23, 70, -117, 16, 72});
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-63, 92, 117, -84, -105, 89, 109, -3, -34, 89, 98}, new byte[]{-79, 61, 7, -59, -7, 62, TarConstants.LF_SYMLINK, -98});
        f8284WWWWWWWW = new C3365WWWWWWWW(20);
    }

    public AdbPairingService() {
        AbstractC3506WWWWWWWW.m15931WWWWWWWW(new WWWWWWWW(this) { // from class: p2.WWoϫWWoӉϫ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbPairingService f32192WWWWWWWWWW;

            {
                this.f32192WWWWWWWWWW = this;
            }

            /* JADX WARN: Type inference failed for: r1v27, types: [java.lang.Object, fc.WWWWҍWWWWּҍ] */
            @Override // tc.WWWWWWWW
            public final Object invoke() {
                int i10;
                PendingIntent foregroundService;
                int color;
                int color2;
                int i11 = 0;
                AdbPairingService adbPairingService = this.f32192WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        AdbPairingService.f8284WWWWWWWW.getClass();
                        Intent intent = new Intent(adbPairingService, AdbPairingService.class);
                        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                        Intent action = intent.setAction(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{112, -68, 5, 45}, new byte[]{3, -56, 106, 93, 43, Byte.MIN_VALUE, -58, -55}));
                        AbstractC3339WWWWWWWW.m15429WWWWWWWW(action, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{113, 33, -45, 45, -104, -81, 64, 44, 108, 108, -119, 66, -43, -14}, new byte[]{2, 68, -89, 108, -5, -37, 41, 67}));
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_stop_searching), PendingIntent.getService(adbPairingService, 2, action, i11)).m961WWWWWWWW();
                    case 1:
                        AdbPairingService.f8284WWWWWWWW.getClass();
                        Intent m15676WWWW = C3365WWWWWWWW.m15676WWWW(adbPairingService);
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_retry), PendingIntent.getService(adbPairingService, 3, m15676WWWW, i11)).m961WWWWWWWW();
                    case 2:
                        C3365WWWWWWWW c3365wwwwwwww = AdbPairingService.f8284WWWWWWWW;
                        HashSet hashSet = new HashSet();
                        Bundle bundle = new Bundle();
                        String str = AdbPairingService.f8283WWWWoWWWWo;
                        if (str != null) {
                            C0257WoWo c0257WoWo = new C0257WoWo(str, adbPairingService.getString(R.string.dialog_adb_pairing_paring_code), null, true, 0, bundle, hashSet);
                            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, -98, 2, 90, -76, -92, 87, -83}, new byte[]{-70, -21, 108, 114, -102, -118, 121, -124});
                            Intent m15670WWWWWWWW = C3365WWWWWWWW.m15670WWWWWWWW(AdbPairingService.f8284WWWWWWWW, adbPairingService, -1);
                            if (Build.VERSION.SDK_INT >= 31) {
                                i10 = 167772160;
                            } else {
                                i10 = 134217728;
                            }
                            foregroundService = PendingIntent.getForegroundService(adbPairingService, 1, m15670WWWWWWWW, i10);
                            C0242WWoWWo c0242WWoWWo = new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_input_paring_code), foregroundService);
                            c0242WWoWWo.f1253WWoWWo = new ArrayList();
                            c0242WWoWWo.f1253WWoWWo.add(c0257WoWo);
                            return c0242WWoWWo.m961WWWWWWWW();
                        }
                        throw new IllegalArgumentException("Result key can't be null");
                    case 3:
                        C3365WWWWWWWW c3365wwwwwwww2 = AdbPairingService.f8284WWWWWWWW;
                        Intent intent2 = new Intent(adbPairingService, AdbActivationTutorialActivity.class);
                        intent2.setFlags(131072);
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_activate_now), PendingIntent.getActivity(adbPairingService, 3, intent2, i11)).m961WWWWWWWW();
                    case 4:
                        C0204WWWWoWWWWo c0204WWWWoWWWWo = new C0204WWWWoWWWWo(adbPairingService, AdbPairingService.f8286WWWoWWWo);
                        color = adbPairingService.getColor(R.color.notification);
                        c0204WWWWoWWWWo.f1145WWWWWWWW = color;
                        c0204WWWWoWWWWo.f1132WWWWoWWWWo.icon = R.drawable.ic_wadb;
                        c0204WWWWoWWWWo.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(adbPairingService.getString(R.string.notification_adb_pairing_searching_for_service_title));
                        C0243WWoWWo c0243WWoWWo = (C0243WWoWWo) adbPairingService.f8290WWWWWWWWWW.getValue();
                        if (c0243WWoWWo != null) {
                            c0204WWWWoWWWWo.f1130WWWWoWWWWo.add(c0243WWoWWo);
                        }
                        return c0204WWWWoWWWWo.m784WWWWWWWW();
                    default:
                        C0204WWWWoWWWWo c0204WWWWoWWWWo2 = new C0204WWWWoWWWWo(adbPairingService, AdbPairingService.f8286WWWoWWWo);
                        color2 = adbPairingService.getColor(R.color.notification);
                        c0204WWWWoWWWWo2.f1145WWWWWWWW = color2;
                        c0204WWWWoWWWWo2.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(adbPairingService.getString(R.string.notification_adb_pairing_working_title));
                        c0204WWWWoWWWWo2.f1132WWWWoWWWWo.icon = R.drawable.ic_wadb;
                        return c0204WWWWoWWWWo2.m784WWWWWWWW();
                }
            }
        });
        this.f8292WWWWWWWW = AbstractC3506WWWWWWWW.m15931WWWWWWWW(new WWWWWWWW(this) { // from class: p2.WWoϫWWoӉϫ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbPairingService f32192WWWWWWWWWW;

            {
                this.f32192WWWWWWWWWW = this;
            }

            /* JADX WARN: Type inference failed for: r1v27, types: [java.lang.Object, fc.WWWWҍWWWWּҍ] */
            @Override // tc.WWWWWWWW
            public final Object invoke() {
                int i10;
                PendingIntent foregroundService;
                int color;
                int color2;
                int i11 = 0;
                AdbPairingService adbPairingService = this.f32192WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        AdbPairingService.f8284WWWWWWWW.getClass();
                        Intent intent = new Intent(adbPairingService, AdbPairingService.class);
                        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                        Intent action = intent.setAction(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{112, -68, 5, 45}, new byte[]{3, -56, 106, 93, 43, Byte.MIN_VALUE, -58, -55}));
                        AbstractC3339WWWWWWWW.m15429WWWWWWWW(action, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{113, 33, -45, 45, -104, -81, 64, 44, 108, 108, -119, 66, -43, -14}, new byte[]{2, 68, -89, 108, -5, -37, 41, 67}));
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_stop_searching), PendingIntent.getService(adbPairingService, 2, action, i11)).m961WWWWWWWW();
                    case 1:
                        AdbPairingService.f8284WWWWWWWW.getClass();
                        Intent m15676WWWW = C3365WWWWWWWW.m15676WWWW(adbPairingService);
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_retry), PendingIntent.getService(adbPairingService, 3, m15676WWWW, i11)).m961WWWWWWWW();
                    case 2:
                        C3365WWWWWWWW c3365wwwwwwww = AdbPairingService.f8284WWWWWWWW;
                        HashSet hashSet = new HashSet();
                        Bundle bundle = new Bundle();
                        String str = AdbPairingService.f8283WWWWoWWWWo;
                        if (str != null) {
                            C0257WoWo c0257WoWo = new C0257WoWo(str, adbPairingService.getString(R.string.dialog_adb_pairing_paring_code), null, true, 0, bundle, hashSet);
                            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, -98, 2, 90, -76, -92, 87, -83}, new byte[]{-70, -21, 108, 114, -102, -118, 121, -124});
                            Intent m15670WWWWWWWW = C3365WWWWWWWW.m15670WWWWWWWW(AdbPairingService.f8284WWWWWWWW, adbPairingService, -1);
                            if (Build.VERSION.SDK_INT >= 31) {
                                i10 = 167772160;
                            } else {
                                i10 = 134217728;
                            }
                            foregroundService = PendingIntent.getForegroundService(adbPairingService, 1, m15670WWWWWWWW, i10);
                            C0242WWoWWo c0242WWoWWo = new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_input_paring_code), foregroundService);
                            c0242WWoWWo.f1253WWoWWo = new ArrayList();
                            c0242WWoWWo.f1253WWoWWo.add(c0257WoWo);
                            return c0242WWoWWo.m961WWWWWWWW();
                        }
                        throw new IllegalArgumentException("Result key can't be null");
                    case 3:
                        C3365WWWWWWWW c3365wwwwwwww2 = AdbPairingService.f8284WWWWWWWW;
                        Intent intent2 = new Intent(adbPairingService, AdbActivationTutorialActivity.class);
                        intent2.setFlags(131072);
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_activate_now), PendingIntent.getActivity(adbPairingService, 3, intent2, i11)).m961WWWWWWWW();
                    case 4:
                        C0204WWWWoWWWWo c0204WWWWoWWWWo = new C0204WWWWoWWWWo(adbPairingService, AdbPairingService.f8286WWWoWWWo);
                        color = adbPairingService.getColor(R.color.notification);
                        c0204WWWWoWWWWo.f1145WWWWWWWW = color;
                        c0204WWWWoWWWWo.f1132WWWWoWWWWo.icon = R.drawable.ic_wadb;
                        c0204WWWWoWWWWo.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(adbPairingService.getString(R.string.notification_adb_pairing_searching_for_service_title));
                        C0243WWoWWo c0243WWoWWo = (C0243WWoWWo) adbPairingService.f8290WWWWWWWWWW.getValue();
                        if (c0243WWoWWo != null) {
                            c0204WWWWoWWWWo.f1130WWWWoWWWWo.add(c0243WWoWWo);
                        }
                        return c0204WWWWoWWWWo.m784WWWWWWWW();
                    default:
                        C0204WWWWoWWWWo c0204WWWWoWWWWo2 = new C0204WWWWoWWWWo(adbPairingService, AdbPairingService.f8286WWWoWWWo);
                        color2 = adbPairingService.getColor(R.color.notification);
                        c0204WWWWoWWWWo2.f1145WWWWWWWW = color2;
                        c0204WWWWoWWWWo2.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(adbPairingService.getString(R.string.notification_adb_pairing_working_title));
                        c0204WWWWoWWWWo2.f1132WWWWoWWWWo.icon = R.drawable.ic_wadb;
                        return c0204WWWWoWWWWo2.m784WWWWWWWW();
                }
            }
        });
        this.f8295WWWW = AbstractC3506WWWWWWWW.m15931WWWWWWWW(new WWWWWWWW(this) { // from class: p2.WWoϫWWoӉϫ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbPairingService f32192WWWWWWWWWW;

            {
                this.f32192WWWWWWWWWW = this;
            }

            /* JADX WARN: Type inference failed for: r1v27, types: [java.lang.Object, fc.WWWWҍWWWWּҍ] */
            @Override // tc.WWWWWWWW
            public final Object invoke() {
                int i10;
                PendingIntent foregroundService;
                int color;
                int color2;
                int i11 = 0;
                AdbPairingService adbPairingService = this.f32192WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        AdbPairingService.f8284WWWWWWWW.getClass();
                        Intent intent = new Intent(adbPairingService, AdbPairingService.class);
                        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                        Intent action = intent.setAction(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{112, -68, 5, 45}, new byte[]{3, -56, 106, 93, 43, Byte.MIN_VALUE, -58, -55}));
                        AbstractC3339WWWWWWWW.m15429WWWWWWWW(action, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{113, 33, -45, 45, -104, -81, 64, 44, 108, 108, -119, 66, -43, -14}, new byte[]{2, 68, -89, 108, -5, -37, 41, 67}));
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_stop_searching), PendingIntent.getService(adbPairingService, 2, action, i11)).m961WWWWWWWW();
                    case 1:
                        AdbPairingService.f8284WWWWWWWW.getClass();
                        Intent m15676WWWW = C3365WWWWWWWW.m15676WWWW(adbPairingService);
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_retry), PendingIntent.getService(adbPairingService, 3, m15676WWWW, i11)).m961WWWWWWWW();
                    case 2:
                        C3365WWWWWWWW c3365wwwwwwww = AdbPairingService.f8284WWWWWWWW;
                        HashSet hashSet = new HashSet();
                        Bundle bundle = new Bundle();
                        String str = AdbPairingService.f8283WWWWoWWWWo;
                        if (str != null) {
                            C0257WoWo c0257WoWo = new C0257WoWo(str, adbPairingService.getString(R.string.dialog_adb_pairing_paring_code), null, true, 0, bundle, hashSet);
                            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, -98, 2, 90, -76, -92, 87, -83}, new byte[]{-70, -21, 108, 114, -102, -118, 121, -124});
                            Intent m15670WWWWWWWW = C3365WWWWWWWW.m15670WWWWWWWW(AdbPairingService.f8284WWWWWWWW, adbPairingService, -1);
                            if (Build.VERSION.SDK_INT >= 31) {
                                i10 = 167772160;
                            } else {
                                i10 = 134217728;
                            }
                            foregroundService = PendingIntent.getForegroundService(adbPairingService, 1, m15670WWWWWWWW, i10);
                            C0242WWoWWo c0242WWoWWo = new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_input_paring_code), foregroundService);
                            c0242WWoWWo.f1253WWoWWo = new ArrayList();
                            c0242WWoWWo.f1253WWoWWo.add(c0257WoWo);
                            return c0242WWoWWo.m961WWWWWWWW();
                        }
                        throw new IllegalArgumentException("Result key can't be null");
                    case 3:
                        C3365WWWWWWWW c3365wwwwwwww2 = AdbPairingService.f8284WWWWWWWW;
                        Intent intent2 = new Intent(adbPairingService, AdbActivationTutorialActivity.class);
                        intent2.setFlags(131072);
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_activate_now), PendingIntent.getActivity(adbPairingService, 3, intent2, i11)).m961WWWWWWWW();
                    case 4:
                        C0204WWWWoWWWWo c0204WWWWoWWWWo = new C0204WWWWoWWWWo(adbPairingService, AdbPairingService.f8286WWWoWWWo);
                        color = adbPairingService.getColor(R.color.notification);
                        c0204WWWWoWWWWo.f1145WWWWWWWW = color;
                        c0204WWWWoWWWWo.f1132WWWWoWWWWo.icon = R.drawable.ic_wadb;
                        c0204WWWWoWWWWo.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(adbPairingService.getString(R.string.notification_adb_pairing_searching_for_service_title));
                        C0243WWoWWo c0243WWoWWo = (C0243WWoWWo) adbPairingService.f8290WWWWWWWWWW.getValue();
                        if (c0243WWoWWo != null) {
                            c0204WWWWoWWWWo.f1130WWWWoWWWWo.add(c0243WWoWWo);
                        }
                        return c0204WWWWoWWWWo.m784WWWWWWWW();
                    default:
                        C0204WWWWoWWWWo c0204WWWWoWWWWo2 = new C0204WWWWoWWWWo(adbPairingService, AdbPairingService.f8286WWWoWWWo);
                        color2 = adbPairingService.getColor(R.color.notification);
                        c0204WWWWoWWWWo2.f1145WWWWWWWW = color2;
                        c0204WWWWoWWWWo2.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(adbPairingService.getString(R.string.notification_adb_pairing_working_title));
                        c0204WWWWoWWWWo2.f1132WWWWoWWWWo.icon = R.drawable.ic_wadb;
                        return c0204WWWWoWWWWo2.m784WWWWWWWW();
                }
            }
        });
        this.f8293WWWWWWWW = AbstractC3506WWWWWWWW.m15931WWWWWWWW(new WWWWWWWW(this) { // from class: p2.WWoϫWWoӉϫ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbPairingService f32192WWWWWWWWWW;

            {
                this.f32192WWWWWWWWWW = this;
            }

            /* JADX WARN: Type inference failed for: r1v27, types: [java.lang.Object, fc.WWWWҍWWWWּҍ] */
            @Override // tc.WWWWWWWW
            public final Object invoke() {
                int i10;
                PendingIntent foregroundService;
                int color;
                int color2;
                int i11 = 0;
                AdbPairingService adbPairingService = this.f32192WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        AdbPairingService.f8284WWWWWWWW.getClass();
                        Intent intent = new Intent(adbPairingService, AdbPairingService.class);
                        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                        Intent action = intent.setAction(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{112, -68, 5, 45}, new byte[]{3, -56, 106, 93, 43, Byte.MIN_VALUE, -58, -55}));
                        AbstractC3339WWWWWWWW.m15429WWWWWWWW(action, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{113, 33, -45, 45, -104, -81, 64, 44, 108, 108, -119, 66, -43, -14}, new byte[]{2, 68, -89, 108, -5, -37, 41, 67}));
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_stop_searching), PendingIntent.getService(adbPairingService, 2, action, i11)).m961WWWWWWWW();
                    case 1:
                        AdbPairingService.f8284WWWWWWWW.getClass();
                        Intent m15676WWWW = C3365WWWWWWWW.m15676WWWW(adbPairingService);
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_retry), PendingIntent.getService(adbPairingService, 3, m15676WWWW, i11)).m961WWWWWWWW();
                    case 2:
                        C3365WWWWWWWW c3365wwwwwwww = AdbPairingService.f8284WWWWWWWW;
                        HashSet hashSet = new HashSet();
                        Bundle bundle = new Bundle();
                        String str = AdbPairingService.f8283WWWWoWWWWo;
                        if (str != null) {
                            C0257WoWo c0257WoWo = new C0257WoWo(str, adbPairingService.getString(R.string.dialog_adb_pairing_paring_code), null, true, 0, bundle, hashSet);
                            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, -98, 2, 90, -76, -92, 87, -83}, new byte[]{-70, -21, 108, 114, -102, -118, 121, -124});
                            Intent m15670WWWWWWWW = C3365WWWWWWWW.m15670WWWWWWWW(AdbPairingService.f8284WWWWWWWW, adbPairingService, -1);
                            if (Build.VERSION.SDK_INT >= 31) {
                                i10 = 167772160;
                            } else {
                                i10 = 134217728;
                            }
                            foregroundService = PendingIntent.getForegroundService(adbPairingService, 1, m15670WWWWWWWW, i10);
                            C0242WWoWWo c0242WWoWWo = new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_input_paring_code), foregroundService);
                            c0242WWoWWo.f1253WWoWWo = new ArrayList();
                            c0242WWoWWo.f1253WWoWWo.add(c0257WoWo);
                            return c0242WWoWWo.m961WWWWWWWW();
                        }
                        throw new IllegalArgumentException("Result key can't be null");
                    case 3:
                        C3365WWWWWWWW c3365wwwwwwww2 = AdbPairingService.f8284WWWWWWWW;
                        Intent intent2 = new Intent(adbPairingService, AdbActivationTutorialActivity.class);
                        intent2.setFlags(131072);
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_activate_now), PendingIntent.getActivity(adbPairingService, 3, intent2, i11)).m961WWWWWWWW();
                    case 4:
                        C0204WWWWoWWWWo c0204WWWWoWWWWo = new C0204WWWWoWWWWo(adbPairingService, AdbPairingService.f8286WWWoWWWo);
                        color = adbPairingService.getColor(R.color.notification);
                        c0204WWWWoWWWWo.f1145WWWWWWWW = color;
                        c0204WWWWoWWWWo.f1132WWWWoWWWWo.icon = R.drawable.ic_wadb;
                        c0204WWWWoWWWWo.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(adbPairingService.getString(R.string.notification_adb_pairing_searching_for_service_title));
                        C0243WWoWWo c0243WWoWWo = (C0243WWoWWo) adbPairingService.f8290WWWWWWWWWW.getValue();
                        if (c0243WWoWWo != null) {
                            c0204WWWWoWWWWo.f1130WWWWoWWWWo.add(c0243WWoWWo);
                        }
                        return c0204WWWWoWWWWo.m784WWWWWWWW();
                    default:
                        C0204WWWWoWWWWo c0204WWWWoWWWWo2 = new C0204WWWWoWWWWo(adbPairingService, AdbPairingService.f8286WWWoWWWo);
                        color2 = adbPairingService.getColor(R.color.notification);
                        c0204WWWWoWWWWo2.f1145WWWWWWWW = color2;
                        c0204WWWWoWWWWo2.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(adbPairingService.getString(R.string.notification_adb_pairing_working_title));
                        c0204WWWWoWWWWo2.f1132WWWWoWWWWo.icon = R.drawable.ic_wadb;
                        return c0204WWWWoWWWWo2.m784WWWWWWWW();
                }
            }
        });
        this.f8294WWoWWo = AbstractC3506WWWWWWWW.m15931WWWWWWWW(new WWWWWWWW(this) { // from class: p2.WWoϫWWoӉϫ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbPairingService f32192WWWWWWWWWW;

            {
                this.f32192WWWWWWWWWW = this;
            }

            /* JADX WARN: Type inference failed for: r1v27, types: [java.lang.Object, fc.WWWWҍWWWWּҍ] */
            @Override // tc.WWWWWWWW
            public final Object invoke() {
                int i10;
                PendingIntent foregroundService;
                int color;
                int color2;
                int i11 = 0;
                AdbPairingService adbPairingService = this.f32192WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        AdbPairingService.f8284WWWWWWWW.getClass();
                        Intent intent = new Intent(adbPairingService, AdbPairingService.class);
                        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                        Intent action = intent.setAction(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{112, -68, 5, 45}, new byte[]{3, -56, 106, 93, 43, Byte.MIN_VALUE, -58, -55}));
                        AbstractC3339WWWWWWWW.m15429WWWWWWWW(action, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{113, 33, -45, 45, -104, -81, 64, 44, 108, 108, -119, 66, -43, -14}, new byte[]{2, 68, -89, 108, -5, -37, 41, 67}));
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_stop_searching), PendingIntent.getService(adbPairingService, 2, action, i11)).m961WWWWWWWW();
                    case 1:
                        AdbPairingService.f8284WWWWWWWW.getClass();
                        Intent m15676WWWW = C3365WWWWWWWW.m15676WWWW(adbPairingService);
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_retry), PendingIntent.getService(adbPairingService, 3, m15676WWWW, i11)).m961WWWWWWWW();
                    case 2:
                        C3365WWWWWWWW c3365wwwwwwww = AdbPairingService.f8284WWWWWWWW;
                        HashSet hashSet = new HashSet();
                        Bundle bundle = new Bundle();
                        String str = AdbPairingService.f8283WWWWoWWWWo;
                        if (str != null) {
                            C0257WoWo c0257WoWo = new C0257WoWo(str, adbPairingService.getString(R.string.dialog_adb_pairing_paring_code), null, true, 0, bundle, hashSet);
                            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, -98, 2, 90, -76, -92, 87, -83}, new byte[]{-70, -21, 108, 114, -102, -118, 121, -124});
                            Intent m15670WWWWWWWW = C3365WWWWWWWW.m15670WWWWWWWW(AdbPairingService.f8284WWWWWWWW, adbPairingService, -1);
                            if (Build.VERSION.SDK_INT >= 31) {
                                i10 = 167772160;
                            } else {
                                i10 = 134217728;
                            }
                            foregroundService = PendingIntent.getForegroundService(adbPairingService, 1, m15670WWWWWWWW, i10);
                            C0242WWoWWo c0242WWoWWo = new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_pairing_input_paring_code), foregroundService);
                            c0242WWoWWo.f1253WWoWWo = new ArrayList();
                            c0242WWoWWo.f1253WWoWWo.add(c0257WoWo);
                            return c0242WWoWWo.m961WWWWWWWW();
                        }
                        throw new IllegalArgumentException("Result key can't be null");
                    case 3:
                        C3365WWWWWWWW c3365wwwwwwww2 = AdbPairingService.f8284WWWWWWWW;
                        Intent intent2 = new Intent(adbPairingService, AdbActivationTutorialActivity.class);
                        intent2.setFlags(131072);
                        if (Build.VERSION.SDK_INT >= 31) {
                            i11 = 67108864;
                        }
                        return new C0242WWoWWo(null, adbPairingService.getString(R.string.notification_adb_activate_now), PendingIntent.getActivity(adbPairingService, 3, intent2, i11)).m961WWWWWWWW();
                    case 4:
                        C0204WWWWoWWWWo c0204WWWWoWWWWo = new C0204WWWWoWWWWo(adbPairingService, AdbPairingService.f8286WWWoWWWo);
                        color = adbPairingService.getColor(R.color.notification);
                        c0204WWWWoWWWWo.f1145WWWWWWWW = color;
                        c0204WWWWoWWWWo.f1132WWWWoWWWWo.icon = R.drawable.ic_wadb;
                        c0204WWWWoWWWWo.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(adbPairingService.getString(R.string.notification_adb_pairing_searching_for_service_title));
                        C0243WWoWWo c0243WWoWWo = (C0243WWoWWo) adbPairingService.f8290WWWWWWWWWW.getValue();
                        if (c0243WWoWWo != null) {
                            c0204WWWWoWWWWo.f1130WWWWoWWWWo.add(c0243WWoWWo);
                        }
                        return c0204WWWWoWWWWo.m784WWWWWWWW();
                    default:
                        C0204WWWWoWWWWo c0204WWWWoWWWWo2 = new C0204WWWWoWWWWo(adbPairingService, AdbPairingService.f8286WWWoWWWo);
                        color2 = adbPairingService.getColor(R.color.notification);
                        c0204WWWWoWWWWo2.f1145WWWWWWWW = color2;
                        c0204WWWWoWWWWo2.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(adbPairingService.getString(R.string.notification_adb_pairing_working_title));
                        c0204WWWWoWWWWo2.f1132WWWWoWWWWo.icon = R.drawable.ic_wadb;
                        return c0204WWWWoWWWWo2.m784WWWWWWWW();
                }
            }
        });
    }

    /* JADX WARN: Type inference failed for: r0v1, types: [java.lang.Object, fc.WWWWҍWWWWּҍ] */
    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final Notification m4835WWWWWWWW() {
        try {
            o2.WoWo m16212WWWoWWWo = o2.WoWo.m16212WWWoWWWo();
            WoWo woWo = this.f8291WWWWoWWWWo;
            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
            m16212WWWoWWWo.m16214WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{125, 5, -15, 70, -110, -86, TarConstants.LF_GNUTYPE_SPARSE, -62, 15, 20, -12, TarConstants.LF_MULTIVOLUME, -51, -73, 81, -42, ConstantPoolEntry.CP_NameAndType, 59, -31, 71, -49}, new byte[]{34, 100, -107, 36, -65, -34, 63, -79}), woWo);
        } catch (Throwable th2) {
            th2.printStackTrace();
        }
        return (Notification) this.f8293WWWWWWWW.getValue();
    }

    @Override // android.app.Service
    public final IBinder onBind(Intent intent) {
        return null;
    }

    @Override // android.app.Service
    public final void onCreate() {
        Object systemService;
        super.onCreate();
        systemService = getSystemService(NotificationManager.class);
        l8.WWWWWWWW.m15773WWWWWWWW();
        NotificationChannel m936WWWWWWWW = AbstractC0241WWoWWo.m936WWWWWWWW(f8286WWWoWWWo, getString(R.string.notification_channel_adb_pairing));
        m936WWWWWWWW.setSound(null, null);
        m936WWWWWWWW.setShowBadge(false);
        m936WWWWWWWW.setAllowBubbles(false);
        ((NotificationManager) systemService).createNotificationChannel(m936WWWWWWWW);
    }

    @Override // android.app.Service
    public final void onDestroy() {
        super.onDestroy();
        try {
            o2.WoWo.m16212WWWoWWWo().m16215WWWWWWWW(this.f8291WWWWoWWWWo);
        } catch (Throwable th2) {
            th2.printStackTrace();
        }
    }

    /* JADX WARN: Type inference failed for: r5v6, types: [java.lang.Object, fc.WWWWҍWWWWּҍ] */
    @Override // android.app.Service
    public final int onStartCommand(Intent intent, int i10, int i11) {
        String str;
        Object systemService;
        CharSequence charSequence;
        Notification notification = null;
        if (intent != null) {
            str = intent.getAction();
        } else {
            str = null;
        }
        if (str != null) {
            int hashCode = str.hashCode();
            if (hashCode != 3540994) {
                if (hashCode != 108401386) {
                    if (hashCode == 109757538 && str.equals(f8289WoWo)) {
                        notification = m4835WWWWWWWW();
                    }
                } else if (str.equals(f8287WWoWWo)) {
                    Bundle resultsFromIntent = RemoteInput.getResultsFromIntent(intent);
                    String str2 = f8283WWWWoWWWWo;
                    if (resultsFromIntent == null || (charSequence = resultsFromIntent.getCharSequence(str2)) == null) {
                        charSequence = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
                    }
                    int intExtra = intent.getIntExtra(str2, -1);
                    if (intExtra != -1) {
                        String obj = charSequence.toString();
                        C2427WWWWWWWW c2427wwwwwwww = C2427WWWWWWWW.f26690WWWWoWWWWo;
                        C3455WWWWWWWW c3455wwwwwwww = WW.f26728WWWWWWWW;
                        AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(c2427wwwwwwww, ExecutorC3454WWWWWWWW.f30758WWWWWWWW, new C3817WWWWWWWW(intExtra, obj, this, null), 2);
                        notification = (Notification) this.f8294WWoWWo.getValue();
                    } else {
                        notification = m4835WWWWWWWW();
                    }
                }
            } else if (str.equals(f8285WWWWWWWW)) {
                stopForeground(1);
            }
            if (notification != null) {
                try {
                    startForeground(1, notification);
                    return 3;
                } catch (Throwable th2) {
                    if (Build.VERSION.SDK_INT >= 31 && AbstractC2285WWWWWWWW.m13602WWWW(th2)) {
                        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                        Log.e(f8288WW, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-76, -66, 63, 106, 108, 20, -93, -45, -94, -83, 44, 119, 109, 60, -88, -127, -95, -85, TarConstants.LF_CONTIG, 116, 125, TarConstants.LF_FIFO}, new byte[]{-57, -54, 94, 24, 24, 82, -52, -95}), th2);
                        systemService = getSystemService(NotificationManager.class);
                        ((NotificationManager) systemService).notify(1, notification);
                    }
                }
            }
        }
        return 3;
    }
}
