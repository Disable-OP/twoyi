package com.android.vmapp.vm;

import android.app.ActivityManager;
import android.content.BroadcastReceiver;
import android.content.ClipData;
import android.content.ClipDescription;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.res.Configuration;
import android.os.Build;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Looper;
import android.os.SystemClock;
import android.text.format.DateFormat;
import android.util.Log;
import androidx.appcompat.widget.r0;
import androidx.datastore.preferences.protobuf.C0962WWWoWWWo;
import com.android.vmapp.VMApp;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.VMManager;
import com.android.vmcore.bridge.IVMEventCallback;
import com.android.vmcore.bridge.VMEventManager;
import com.android.vmcore.bridge.VMEvents;
import com.android.vmcore.event.PermissionEvent;
import com.android.vmcore.event.VMStatusEvent;
import com.blankj.utilcode.util.C1628WWWWWWWW;
import com.blankj.utilcode.util.C1644WWWoWWWo;
import com.google.firebase.Firebase;
import com.google.firebase.analytics.AnalyticsKt;
import com.google.firebase.analytics.FirebaseAnalytics;
import com.google.firebase.analytics.ParametersBuilder;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import eh.C2467WWWWWWWW;
import eh.InterfaceC2472WWWWWWWW;
import fc.WoWo;
import i6.C2899WWWWWWWW;
import j3.C3164WWWWWWWW;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.Locale;
import java.util.TimeZone;
import java.util.concurrent.TimeUnit;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import l2.C3365WWWWWWWW;
import me.AbstractC3506WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.greenrobot.eventbus.ThreadMode;
import org.json.JSONObject;
import p000WWWWWWWWWW.WWoWWo;
import p001WWWWoWWWWo.RunnableC0054WWWWWWWW;
import p020WWWWWWWW.AbstractC0211WWWWWWWW;
import q3.WWWWoWWWWo;
import r4.C3962WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class VMExtension extends BroadcastReceiver implements IVMEventCallback, ClipboardManager.OnPrimaryClipChangedListener {

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public static final /* synthetic */ int f8765WWoWWo = 0;

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final VMInstance f8766WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final VMApp f8767WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final ClipboardManager f8768WWWWWWWW;

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public long f8769WWWWWWWW = 0;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final C3962WWWWWWWW f8770WWWoWWWo;

    public VMExtension(VMApp vMApp, VMInstance vMInstance) {
        this.f8767WWWWWWWW = vMApp;
        this.f8766WWWWoWWWWo = vMInstance;
        this.f8770WWWoWWWo = new C3962WWWWWWWW(vMInstance);
        vMInstance.f8939WWWoWWWo.m13950WWWW(this);
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        ClipboardManager clipboardManager = (ClipboardManager) vMApp.getSystemService(WWWWWWWW.m17835WWWWWWWW(new byte[]{-53, -93, TarConstants.LF_SYMLINK, -80, -113, -53, 97, -25, -52}, new byte[]{-88, -49, 91, -64, -19, -92, 0, -107}));
        this.f8768WWWWWWWW = clipboardManager;
        if (clipboardManager != null) {
            clipboardManager.addPrimaryClipChangedListener(this);
        }
        try {
            int i10 = vMInstance.f8937WWWoWWWo.f8866WWWWWWWW;
            AbstractC0211WWWWWWWW.m830WWWWWWWW(vMApp, this, new IntentFilter(WWWWWWWW.m17835WWWWWWWW(new byte[]{-107, 125, -60, TarConstants.LF_MULTIVOLUME, -116, 27, -60, -118, -103, 123, -51, TarConstants.LF_MULTIVOLUME, -101, 24, -59, Byte.MIN_VALUE, -126, 60, -56, 0, -103, 28, -49, -106, -40, 92, -26, TarConstants.LF_CONTIG, -92, TarConstants.LF_CHR, -23, -69, -73, 70, -32, 44, -93, 42, -31, -69, -94, 91, -26, 45, -78}, new byte[]{-10, 18, -87, 99, -19, 117, -96, -8}) + i10), 4);
        } catch (Throwable th2) {
            th2.printStackTrace();
        }
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final void m5012WWWWoWWWWo() {
        VMInstance vMInstance = this.f8766WWWWoWWWWo;
        VMEventManager vMEventManager = vMInstance.f8935WWWWWWWW;
        if (vMEventManager != null) {
            if (vMInstance.f8937WWWoWWWo.f8895WWWoWWWo.f8847WWWWWWWW == 11) {
                String str = VMEvents.f8996WWWWWWWW;
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                vMEventManager.m5116WWWoWWWo(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{67, 115, -35, -41, -94, 44, -68, -30, 80, 125, -33, -41, -86, TarConstants.LF_FIFO, -122, -48, 79, 118, -56, -125, -5}, new byte[]{32, 18, -83, -93, -53, 90, -39, -67}));
                return;
            }
            String str2 = VMEvents.f8996WWWWWWWW;
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            vMEventManager.m5116WWWoWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-36, -63, 45, 4, 32, -104, -32, -1, -49, -49, 47, 4, 40, -126, -38, -60, -38, -44, 56, 19, 61, -121, -22, -50, -32, -59, TarConstants.LF_CHR, 17, 43, -126, -32, -60, -97, -112}, new byte[]{-65, -96, 93, 112, 73, -18, -123, -96}));
        }
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Type inference failed for: r3v28, types: [java.lang.Object, m3.WWWȏWWWoನ̑] */
    /* JADX WARN: Type inference failed for: r3v8, types: [m3.WWWW̏WWWWβ̏, java.lang.Object] */
    @Override // com.android.vmcore.bridge.IVMEventCallback
    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final void mo5013WWWWWWWW(String str, String str2) {
        ClipboardManager clipboardManager;
        String m17835WWWWWWWW;
        boolean equals = VMEvents.f8992WWWWoWWWWo.equals(str);
        VMApp vMApp = this.f8767WWWWWWWW;
        VMInstance vMInstance = this.f8766WWWWoWWWWo;
        if (equals) {
            Locale m16514WWWWoWWWWo = WWWWoWWWWo.m16514WWWWoWWWWo();
            boolean is24HourFormat = DateFormat.is24HourFormat(vMApp);
            String id2 = TimeZone.getDefault().getID();
            StringBuilder sb2 = new StringBuilder();
            String language = m16514WWWWoWWWWo.getLanguage();
            String country = m16514WWWWoWWWWo.getCountry();
            String variant = m16514WWWWoWWWWo.getVariant();
            sb2.append(language);
            sb2.append(" ");
            sb2.append(country);
            WWoWWo.m59WWoWWo(sb2, " ", variant, " ");
            byte[] bArr = {-71, 33};
            if (is24HourFormat) {
                // fill-array-data instruction
                bArr[0] = 107;
                bArr[1] = 18;
                byte[] bArr2 = {89, 38, -30, TarConstants.LF_DIR, 96, -112, -20, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER};
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
            } else {
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(bArr, new byte[]{-120, 19, -67, -86, -9, -106, 73, -40});
            }
            WWoWWo.m59WWoWWo(sb2, m17835WWWWWWWW, " ", id2);
            VMEventManager vMEventManager = vMInstance.f8935WWWWWWWW;
            if (vMEventManager != null) {
                vMEventManager.m5116WWWoWWWo(VMEvents.f8999WWoWWo, sb2.toString());
            }
            VMEventManager vMEventManager2 = vMInstance.f8935WWWWWWWW;
            if (vMEventManager2 != null) {
                String str3 = VMEvents.f8996WWWWWWWW;
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                vMEventManager2.m5116WWWoWWWo(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{26, 64, 91, 28, -28, TarConstants.LF_SYMLINK, 59, TarConstants.LF_BLK, 29, 65, 70, TarConstants.LF_CONTIG, -24, 63, 37, 0, 22, 90, 119, 9, -11, 46, 36, TarConstants.LF_GNUTYPE_LONGLINK, 66}, new byte[]{115, 46, 40, 104, -123, 94, 87, 107}));
            }
            m5012WWWWoWWWWo();
            C1644WWWoWWWo.m5313WWWWWWWW(C1644WWWoWWWo.m5312WWWWoWWWWo(-8), new C1628WWWWWWWW(1, this), 1L, TimeUnit.SECONDS);
            if (vMInstance.f8937WWWoWWWo.f8865WWWWoWWWWo) {
                byte[] bArr3 = {56, -122, -10, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 61, 37, 18};
                byte[] bArr4 = {89, -24, -110, 10, 82, TarConstants.LF_GNUTYPE_LONGNAME, 118, 29};
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                vMInstance.m5079WWWoWWWo(WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4), WWWWWWWW.m17835WWWWWWWW(new byte[]{93, -2, TarConstants.LF_CHR, 19, 4, -93, 67, TarConstants.LF_CONTIG, 71, -8, 40, 20, TarConstants.LF_GNUTYPE_LONGNAME, -81, 78, TarConstants.LF_CONTIG, 81, -3, 34}, new byte[]{TarConstants.LF_CHR, -111, 71, 122, 98, -54, 32, 86}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
            } else {
                byte[] bArr5 = {TarConstants.LF_SYMLINK, 66, 38, -90, -59, 107, -68};
                byte[] bArr6 = {TarConstants.LF_GNUTYPE_SPARSE, 44, 66, -44, -86, 2, -40, 104};
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                vMInstance.m5079WWWoWWWo(WWWWWWWW.m17835WWWWWWWW(bArr5, bArr6), WWWWWWWW.m17835WWWWWWWW(new byte[]{84, 40, -1, 24, 101, TarConstants.LF_NORMAL, TarConstants.LF_FIFO, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 78, 46, -28, 31, 45, 61, 60, 106, 91, 37, -25, 20}, new byte[]{58, 71, -117, 113, 3, 89, 85, 25}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
            }
            m5014WWWWWWWW();
        } else if (VMEvents.f8998WWWoWWWo.equals(str)) {
            String str4 = str2.split(" ")[0];
            vMInstance.f8939WWWoWWWo.m13942WWWWWWWW(new C3365WWWWWWWW(4));
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-81, 58, -28, 41, -61, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 35, -62, -93, 60, -19, 41, -63, 104, 42, -43, -66, TarConstants.LF_BLK, -69}, new byte[]{-52, 85, -119, 7, -94, 9, 71, -80}).equals(str4) && !vMInstance.m5085WWoWWo()) {
                if (r0.m2673WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{33, 63, 95, 98, -19, -11, 65, 63, TarConstants.LF_NORMAL, TarConstants.LF_BLK, 73, 125, -21, -17, 86, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 47, 63, 21, TarConstants.LF_GNUTYPE_SPARSE, -61, -47, 96, 67, 1}, new byte[]{64, 81, 59, 16, -126, -100, 37, 17}))) {
                    vMInstance.m5101o(true);
                } else {
                    vMInstance.f8939WWWoWWWo.m13942WWWWWWWW(new PermissionEvent(WWWWWWWW.m17835WWWWWWWW(new byte[]{116, -1, 61, -33, -39, -33, -63, 106, 101, -12, 43, -64, -33, -59, -42, 45, 122, -1, 119, -18, -9, -5, -32, 22, 84}, new byte[]{21, -111, 89, -83, -74, -74, -91, 68}), new C2899WWWWWWWW(19, this)));
                }
            }
        } else {
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-81, -52, 18, -64, -68, 112, 10, -75, -89, -52, 2, -41, -67, 109, 64, -6, -83, -42, 31, -35, -67, TarConstants.LF_CONTIG, 45, -44, Byte.MIN_VALUE, -28, 63, -11, -122, TarConstants.LF_GNUTYPE_LONGLINK, 47, -49, -121, -19, 56, -19, -112, 81, 47, -43, -119, -25, TarConstants.LF_SYMLINK}, new byte[]{-50, -94, 118, -78, -45, 25, 110, -101}).equals(str)) {
                try {
                    JSONObject jSONObject = new JSONObject(str2);
                    Configuration configuration = new Configuration();
                    configuration.orientation = jSONObject.getInt(WWWWWWWW.m17835WWWWWWWW(new byte[]{107, -117, 100, -111, -49, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -126, 20, 109, -106, 99}, new byte[]{4, -7, 13, -12, -95, 19, -29, 96}));
                    C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
                    ?? obj = new Object();
                    obj.f30921WWWWWWWW = configuration;
                    c2467wwwwwwww.m13942WWWWWWWW(obj);
                } catch (Throwable th2) {
                    th2.printStackTrace();
                }
            } else if (VMEvents.f8994WWWWWWWW.equals(str)) {
                VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
                if (vMConfig != null && vMConfig.f8889WWWWWWWW && (clipboardManager = this.f8768WWWWWWWW) != null) {
                    clipboardManager.setPrimaryClip(ClipData.newPlainText(WWWWWWWW.m17835WWWWWWWW(new byte[]{35, -86, 24, -10, 93, -34, 104}, new byte[]{69, -40, 119, -101, 2, -88, 5, 1}), str2));
                }
            } else if (VMEvents.f8995WWWWWWWW.equals(str)) {
                C2467WWWWWWWW c2467wwwwwwww2 = vMInstance.f8939WWWoWWWo;
                ?? obj2 = new Object();
                obj2.f30922WWWWWWWW = str2;
                c2467wwwwwwww2.m13942WWWWWWWW(obj2);
            } else if (VMEvents.f8993WWWWWWWW.equals(str)) {
                m5012WWWWoWWWWo();
            } else if (VMEvents.f8997WWWWWWWW.equals(str)) {
                try {
                    JSONObject jSONObject2 = new JSONObject(str2);
                    jSONObject2.getString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, 63, -45}, new byte[]{-32, 84, -76, 14, -120, -118, 37, -9}));
                    String string = jSONObject2.getString(WWWWWWWW.m17835WWWWWWWW(new byte[]{34, -95, 29}, new byte[]{65, -52, 121, TarConstants.LF_GNUTYPE_SPARSE, -9, -60, -107, -39}));
                    String string2 = jSONObject2.getString(WWWWWWWW.m17835WWWWWWWW(new byte[]{104, 78, -83}, new byte[]{9, 60, -54, -45, 22, 30, -27, TarConstants.LF_GNUTYPE_LONGLINK}));
                    if (string.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{-33, -118, 17, 4, -18, -5, -35, 26, -59, -116, 10, 3, -90, -30, -47, 8, -59}, new byte[]{-79, -27, 101, 109, -120, -110, -66, 123}))) {
                        s4.WWoWWo.m16928WWWWWWWW(vMApp, vMInstance, string2);
                    } else if (string.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{116, 3, 43, 57, -47, -39, 66, -8, 110, 5, TarConstants.LF_NORMAL, 62, -103, -62, 68, -12, 117, 26, 58}, new byte[]{26, 108, 95, 80, -73, -80, 33, -103}))) {
                        s4.WWoWWo.m16926WWWWWWWW(vMApp, vMInstance, string2);
                    } else if (string.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{42, -92, 116, 1, -81, -98, -8, TarConstants.LF_PAX_EXTENDED_HEADER_LC, TarConstants.LF_NORMAL, -94, 111, 6, -25, -123, -2, 116, 43, -67, 101, 41, -91, -101}, new byte[]{68, -53, 0, 104, -55, -9, -101, 25}))) {
                        s4.WWoWWo.m16929WWWoWWWo(vMApp, vMInstance);
                    }
                } catch (Throwable th3) {
                    th3.printStackTrace();
                }
            }
        }
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final void m5014WWWWWWWW() {
        VMConfig vMConfig;
        ClipboardManager clipboardManager;
        ClipData primaryClip;
        ClipData.Item itemAt;
        CharSequence text;
        VMEventManager vMEventManager;
        VMInstance vMInstance = this.f8766WWWWoWWWWo;
        try {
            if (vMInstance.f8940WWoWWo >= 7 && (vMConfig = vMInstance.f8937WWWoWWWo) != null && vMConfig.f8889WWWWWWWW && (clipboardManager = this.f8768WWWWWWWW) != null && (primaryClip = clipboardManager.getPrimaryClip()) != null) {
                ClipDescription description = primaryClip.getDescription();
                if (description != null && description.getLabel() != null) {
                    String charSequence = description.getLabel().toString();
                    byte[] bArr = {82, 85, -96, 89, -20, TarConstants.LF_FIFO, 89};
                    byte[] bArr2 = {TarConstants.LF_BLK, 39, -49, TarConstants.LF_BLK, -77, 64, TarConstants.LF_BLK, -4};
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    if (charSequence.equals(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2))) {
                        return;
                    }
                }
                if (primaryClip.getItemCount() > 0 && (itemAt = primaryClip.getItemAt(0)) != null && (text = itemAt.getText()) != null && (vMEventManager = vMInstance.f8935WWWWWWWW) != null) {
                    vMEventManager.m5116WWWoWWWo(VMEvents.f8994WWWWWWWW, text.toString());
                }
            }
        } catch (Throwable th2) {
            th2.printStackTrace();
        }
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final void m5015WWWoWWWo() {
        byte[] bArr = {-54, -52, -61, -112, -67, TarConstants.LF_BLK, 46, 101};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-85, -81, -73, -7, -53, 93, 90, 28}, bArr);
        VMApp vMApp = this.f8767WWWWWWWW;
        for (ActivityManager.AppTask appTask : ((ActivityManager) vMApp.getSystemService(m17835WWWWWWWW)).getAppTasks()) {
            try {
                appTask.moveToFront();
                break;
            } catch (Throwable th2) {
                th2.printStackTrace();
            }
        }
        VMStartActivity0.m5016WWWW(vMApp, this.f8766WWWWoWWWWo.f8937WWWoWWWo.f8866WWWWWWWW, true, false);
    }

    @Override // android.content.ClipboardManager.OnPrimaryClipChangedListener
    public final void onPrimaryClipChanged() {
        long[] jArr = new long[1];
        long uptimeMillis = SystemClock.uptimeMillis();
        if (uptimeMillis - jArr[0] < 500) {
            return;
        }
        jArr[0] = uptimeMillis;
        m5014WWWWWWWW();
    }

    /* JADX WARN: Code restructure failed: missing block: B:17:0x0119, code lost:
        m5015WWWoWWWo();
     */
    /* JADX WARN: Code restructure failed: missing block: B:18:0x011c, code lost:
        return;
     */
    @Override // android.content.BroadcastReceiver
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final void onReceive(Context context, Intent intent) {
        boolean z10;
        VMInstance vMInstance = this.f8766WWWWoWWWWo;
        try {
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            String stringExtra = intent.getStringExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{115, 70, -103, 58}, new byte[]{29, 45, -4, 67, 89, -12, -73, -56}));
            String stringExtra2 = intent.getStringExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{-51, 117, -80, 95, 42, -45, -63, 124, -35}, new byte[]{-92, 27, -60, 58, 68, -89, -118, 25}));
            String stringExtra3 = intent.getStringExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{66, -109, -81, -64, 73, 25, -83, -50, 91, -104}, new byte[]{43, -3, -37, -91, 39, 109, -7, -73}));
            String stringExtra4 = intent.getStringExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{113, -55, -104, -19, -4, 57, -4, -24, 96, -49}, new byte[]{16, -86, -20, -124, -109, 87, -88, -111}));
            JSONObject m16925WWWWWWWW = s4.WWoWWo.m16925WWWWWWWW(intent);
            JSONObject jSONObject = new JSONObject();
            jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{-4, -125, TarConstants.LF_NORMAL, -82}, new byte[]{-110, -24, 85, -41, -86, 107, -127, 116}), stringExtra);
            jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{-41, -120, -82, 73, -45, 124, -99, -118, -57}, new byte[]{-66, -26, -38, 44, -67, 8, -42, -17}), stringExtra2);
            jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{-4, 124, 111, 100, -120, 60, -17, -82, -19, 122}, new byte[]{-99, 31, 27, 13, -25, 82, -69, -41}), stringExtra4);
            jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{39, -123, -107, -95, -125, -90, -16, -14, 37, -107, -116}, new byte[]{85, -32, -8, -50, -9, -61, -71, -100}), m16925WWWWWWWW);
            vMInstance.m5079WWWoWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{44, -55, -103, 34, 8, -90, -33, -42, 32, -49, -112, 34, 31, -91, -56, -63, 61, -48, -99, 111, ConstantPoolEntry.CP_NameAndType}, new byte[]{79, -90, -12, ConstantPoolEntry.CP_NameAndType, 105, -56, -69, -92}), WWWWWWWW.m17835WWWWWWWW(new byte[]{4, 101, -115, 118, 3, -59, -119, 87, 30, 99, -106, 113, TarConstants.LF_GNUTYPE_LONGLINK, -51, -119, 66, 3, 101, -105}, new byte[]{106, 10, -7, 31, 101, -84, -22, TarConstants.LF_FIFO}), jSONObject.toString());
            if (vMInstance.f8940WWoWWo >= 7) {
                z10 = true;
            } else {
                z10 = false;
            }
            if (!WWWWWWWW.m17835WWWWWWWW(new byte[]{78, -90, 16, 79, -52, 37, 105, 79}, new byte[]{47, -59, 100, 38, -70, TarConstants.LF_GNUTYPE_LONGNAME, 29, TarConstants.LF_FIFO}).equals(stringExtra3) && !WWWWWWWW.m17835WWWWWWWW(new byte[]{-123, 73, -66, 47, 72, -6, 21, 64, -120, 82, -75, TarConstants.LF_DIR, 89}, new byte[]{-26, 38, -48, 91, 45, -108, 97, 9}).equals(stringExtra4) && !WWWWWWWW.m17835WWWWWWWW(new byte[]{-70, -112, 112, 44, 73, 22, -31, -25, -80, -111, 106, 61, 66, ConstantPoolEntry.CP_NameAndType}, new byte[]{-39, -1, 30, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 44, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -107, -72}).equals(stringExtra4)) {
            }
        } catch (Throwable th2) {
            th2.printStackTrace();
        }
    }

    @InterfaceC2472WWWWWWWW(threadMode = ThreadMode.POSTING)
    public void onVMStatusEvent(VMStatusEvent vMStatusEvent) {
        VMEventManager vMEventManager;
        Object m15937WWoWWo;
        int i10 = vMStatusEvent.f9016WWWWWWWW;
        VMApp vMApp = this.f8767WWWWWWWW;
        if (i10 == 1) {
            int i11 = VMCoreService.f8760WWWWoWWWWo;
            C0962WWWoWWWo.m3124WWoWWo(vMApp);
        }
        int i12 = vMStatusEvent.f9016WWWWWWWW;
        if (i12 <= 0 && i12 != -5) {
            Iterator it = VMManager.m5102WWWWWWWW().m5108WWoWWo().iterator();
            while (true) {
                if (it.hasNext()) {
                    if (((VMInstance) it.next()).f8940WWoWWo > 0) {
                        break;
                    }
                } else {
                    int i13 = VMCoreService.f8760WWWWoWWWWo;
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    AbstractC3339WWWWWWWW.m15439WWoWWo(vMApp, WWWWWWWW.m17835WWWWWWWW(new byte[]{41, 56, 87, -94, 89, 117, 119}, new byte[]{74, 87, 57, -42, 60, 13, 3, -47}));
                    try {
                        Log.d(WWWWWWWW.m17835WWWWWWWW(new byte[]{25, -55, -63, -73, 39, -3, -14, -34, 61, -14, -21, -69, TarConstants.LF_NORMAL}, new byte[]{79, -124, -126, -40, 85, -104, -95, -69}), WWWWWWWW.m17835WWWWWWWW(new byte[]{90, -85, 92, 93}, new byte[]{41, -33, TarConstants.LF_CHR, 45, 63, 79, 104, -41}));
                        m15937WWoWWo = Boolean.valueOf(vMApp.stopService(new Intent(vMApp, VMCoreService.class)));
                    } catch (Throwable th2) {
                        m15937WWoWWo = AbstractC3506WWWWWWWW.m15937WWoWWo(th2);
                    }
                    Throwable m14005WWWWWWWW = WoWo.m14005WWWWWWWW(m15937WWoWWo);
                    if (m14005WWWWWWWW != null) {
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        Log.d(WWWWWWWW.m17835WWWWWWWW(new byte[]{-15, -25, Byte.MIN_VALUE, 42, 31, -36, -44, -98, -43, -36, -86, 38, 8}, new byte[]{-89, -86, -61, 69, 109, -71, -121, -5}), WWWWWWWW.m17835WWWWWWWW(new byte[]{36, -35, -69, -62, 7, 80, -52, -40, 59, -52, -80}, new byte[]{87, -87, -44, -78, 39, TarConstants.LF_FIFO, -83, -79}), m14005WWWWWWWW);
                    }
                }
            }
        }
        VMInstance vMInstance = this.f8766WWWWoWWWWo;
        if (i12 == 4 && (vMEventManager = vMInstance.f8935WWWWWWWW) != null) {
            ArrayList arrayList = vMEventManager.f8989WWWWWWWW;
            if (!arrayList.contains(this)) {
                arrayList.add(this);
            }
        }
        final C3962WWWWWWWW c3962wwwwwwww = this.f8770WWWoWWWo;
        if (i12 == 5) {
            c3962wwwwwwww.getClass();
            HandlerThread handlerThread = new HandlerThread(C3962WWWWWWWW.f32952WWWWWWWW);
            c3962wwwwwwww.f32958WWWWWWWW = handlerThread;
            handlerThread.start();
            Handler handler = new Handler(c3962wwwwwwww.f32958WWWWWWWW.getLooper());
            c3962wwwwwwww.f32961WWWoWWWo = handler;
            handler.post(new Runnable() { // from class: r4.WWWȏWWWoನ̑
                @Override // java.lang.Runnable
                public final void run() {
                    switch (r2) {
                        case 0:
                            C3962WWWWWWWW c3962wwwwwwww2 = c3962wwwwwwww;
                            c3962wwwwwwww2.f32960WWWoWWWo.clear();
                            c3962wwwwwwww2.f32955WWWWWWWW.clear();
                            return;
                        default:
                            C3962WWWWWWWW c3962wwwwwwww3 = c3962wwwwwwww;
                            c3962wwwwwwww3.f32960WWWoWWWo.clear();
                            c3962wwwwwwww3.f32955WWWWWWWW.clear();
                            return;
                    }
                }
            });
            c3962wwwwwwww.f32961WWWoWWWo.post(c3962wwwwwwww.f32959WWWWWWWW);
            if (c3962wwwwwwww.f32962WWoWWo.exists()) {
                c3962wwwwwwww.f32957WWWWWWWW.startWatching();
            }
        }
        if (i12 == -5) {
            VMEventManager vMEventManager2 = vMInstance.f8935WWWWWWWW;
            if (vMEventManager2 != null) {
                vMEventManager2.f8989WWWWWWWW.remove(this);
            }
            Handler handler2 = c3962wwwwwwww.f32961WWWoWWWo;
            if (handler2 != null) {
                handler2.post(new Runnable() { // from class: r4.WWWȏWWWoನ̑
                    @Override // java.lang.Runnable
                    public final void run() {
                        switch (r2) {
                            case 0:
                                C3962WWWWWWWW c3962wwwwwwww2 = c3962wwwwwwww;
                                c3962wwwwwwww2.f32960WWWoWWWo.clear();
                                c3962wwwwwwww2.f32955WWWWWWWW.clear();
                                return;
                            default:
                                C3962WWWWWWWW c3962wwwwwwww3 = c3962wwwwwwww;
                                c3962wwwwwwww3.f32960WWWoWWWo.clear();
                                c3962wwwwwwww3.f32955WWWWWWWW.clear();
                                return;
                        }
                    }
                });
            }
            HandlerThread handlerThread2 = c3962wwwwwwww.f32958WWWWWWWW;
            if (handlerThread2 != null) {
                handlerThread2.quit();
            }
            c3962wwwwwwww.f32956WWWWWWWW = 0;
            c3962wwwwwwww.f32957WWWWWWWW.stopWatching();
            s4.WWoWWo.m16929WWWoWWWo(vMApp, vMInstance);
        }
        int i14 = vMStatusEvent.f9015WWWWoWWWWo;
        if (i14 != 0) {
            VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
            int i15 = vMConfig.f8866WWWWWWWW;
            String str = vMConfig.f8923WoWo;
            i0.WWWWWWWW.m14530WWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{7, 46, 95, -17}, new byte[]{115, 87, 47, -118, -84, 101, 35, 124}, str);
            FirebaseAnalytics analytics = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
            String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{65, -71, -55, 74, -35, -94, 82, -100, 81, -75, -1, 68, -41, -87}, new byte[]{TarConstants.LF_CONTIG, -44, -106, 40, -78, -51, 38, -61});
            ParametersBuilder parametersBuilder = new ParametersBuilder();
            parametersBuilder.param(WWWWWWWW.m17835WWWWWWWW(new byte[]{126, 102, 114, -88, -51}, new byte[]{8, ConstantPoolEntry.CP_InterfaceMethodref, 45, -63, -87, -12, 36, 104}), String.valueOf(i15));
            parametersBuilder.param(WWWWWWWW.m17835WWWWWWWW(new byte[]{40, 27, -32, -26, -82, 123, 90, TarConstants.LF_FIFO, 47}, new byte[]{74, 116, -113, -110, -15, 15, 35, 70}), str);
            parametersBuilder.param(WWWWWWWW.m17835WWWWWWWW(new byte[]{-70, 112, -44, -80, -69}, new byte[]{-33, 2, -90, -33, -55, Byte.MAX_VALUE, -16, 71}), String.valueOf(i14));
            parametersBuilder.param(WWWWWWWW.m17835WWWWWWWW(new byte[]{-52, -86, 71, -13, -104, 108, 28}, new byte[]{-65, -50, 44, -84, -15, 2, 104, -72}), String.valueOf(Build.VERSION.SDK_INT));
            analytics.logEvent(m17835WWWWWWWW, parametersBuilder.getBundle());
        }
        if (i12 == 0) {
            this.f8769WWWWWWWW = SystemClock.uptimeMillis();
        }
        if (i12 == 7 && this.f8769WWWWWWWW != 0) {
            int i16 = vMInstance.f8937WWWoWWWo.f8866WWWWWWWW;
            int uptimeMillis = (int) ((SystemClock.uptimeMillis() - this.f8769WWWWWWWW) / 1000);
            String str2 = vMInstance.f8937WWWoWWWo.f8923WoWo;
            i0.WWWWWWWW.m14530WWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{Byte.MAX_VALUE, -110, 108, 40}, new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, -21, 28, TarConstants.LF_MULTIVOLUME, -106, TarConstants.LF_DIR, 44, 85}, str2);
            FirebaseAnalytics analytics2 = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
            String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{117, 1, -5, -21, 6, 110, -126, 87, 119, 5, -55, -20}, new byte[]{3, 108, -92, -119, 105, 1, -10, 8});
            ParametersBuilder parametersBuilder2 = new ParametersBuilder();
            parametersBuilder2.param(WWWWWWWW.m17835WWWWWWWW(new byte[]{15, -2, 2, 117, 107}, new byte[]{121, -109, 93, 28, 15, -50, -53, -63}), String.valueOf(i16));
            parametersBuilder2.param(WWWWWWWW.m17835WWWWWWWW(new byte[]{84, 111, 5, -26, 66, -4, -29, 96, TarConstants.LF_GNUTYPE_SPARSE}, new byte[]{TarConstants.LF_FIFO, 0, 106, -110, 29, -120, -102, 16}), str2);
            parametersBuilder2.param(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 32, -5, 78, -2, -65, 30}, new byte[]{20, 69, -104, 33, -112, -37, 109, 121}), String.valueOf(uptimeMillis));
            parametersBuilder2.param(WWWWWWWW.m17835WWWWWWWW(new byte[]{106, 118, Byte.MIN_VALUE, 20, 38, 122, -125}, new byte[]{25, 18, -21, TarConstants.LF_GNUTYPE_LONGLINK, 79, 20, -9, -76}), String.valueOf(Build.VERSION.SDK_INT));
            analytics2.logEvent(m17835WWWWWWWW2, parametersBuilder2.getBundle());
        }
        if (i12 == 5) {
            new Handler(Looper.getMainLooper()).postDelayed(new RunnableC0054WWWWWWWW(27, this), 60000L);
        }
    }
}
