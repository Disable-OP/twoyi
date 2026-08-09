package com.android.vmapp.vm;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Intent;
import android.content.res.Configuration;
import android.os.Build;
import android.os.IBinder;
import android.util.Log;
import com.android.vmapp.ui.MainActivity;
import com.clone.android.dual.space.R;
import j3.C3164WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p020WWWWWWWW.AbstractC0235WWWoWWWo;
import p020WWWWWWWW.C0204WWWWoWWWWo;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public final class VMCoreService extends Service {

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public static final /* synthetic */ int f8760WWWWoWWWWo = 0;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final Notification m5010WWWWWWWW() {
        if (Build.VERSION.SDK_INT >= 26) {
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            Object systemService = getSystemService(WWWWWWWW.m17835WWWWWWWW(new byte[]{45, TarConstants.LF_LINK, 60, 6, -99, 79, -54, -116, TarConstants.LF_CONTIG, TarConstants.LF_CONTIG, 39, 1}, new byte[]{67, 94, 72, 111, -5, 38, -87, -19}));
            AbstractC3339WWWWWWWW.m15428WWWWWWWW(systemService, WWWWWWWW.m17835WWWWWWWW(new byte[]{45, 99, -43, Byte.MAX_VALUE, -89, -97, -108, 8, 45, 121, -51, TarConstants.LF_CHR, -27, -103, -43, 5, 34, 101, -51, TarConstants.LF_CHR, -13, -109, -43, 8, 44, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -108, 125, -14, -112, -103, 70, TarConstants.LF_CONTIG, 111, -55, 118, -89, -99, -101, 2, TarConstants.LF_LINK, 121, -48, 119, -87, -99, -123, 22, 109, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -42, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -18, -102, -100, 5, 34, 98, -48, 124, -23, -79, -108, 8, 34, 113, -36, 97}, new byte[]{67, 22, -71, 19, -121, -4, -11, 102}));
            l8.WWWWWWWW.m15773WWWWWWWW();
            NotificationChannel m15768WWWWWWWW = l8.WWWWWWWW.m15768WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{-49, -109, -70, -56, 106, -65, 64, -21, -54, -101, -105, -35, 108, -82, 64}, new byte[]{-71, -2, -27, -85, 5, -51, 37, -76}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-77, -82, 56, -77, 87, -54, -98, -127, -92, -87, 46, -75, TarConstants.LF_MULTIVOLUME, -62, -106, -127, -74, -94, 56, -79, TarConstants.LF_GNUTYPE_LONGLINK, -56, -105}, new byte[]{-27, -57, 74, -57, 34, -85, -14, -95}));
            l8.WWWWWWWW.m15794o(m15768WWWWWWWW);
            l8.WWWWWWWW.m15767WWWWoWWWWo(m15768WWWWWWWW);
            ((NotificationManager) systemService).createNotificationChannel(m15768WWWWWWWW);
        }
        Intent intent = new Intent(this, MainActivity.class);
        intent.addFlags(268435456);
        intent.addFlags(32768);
        PendingIntent activity = PendingIntent.getActivity(this, 0, intent, 201326592);
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        C0204WWWWoWWWWo c0204WWWWoWWWWo = new C0204WWWWoWWWWo(this, WWWWWWWW.m17835WWWWWWWW(new byte[]{-123, -41, -55, -69, -42, -46, 45, -58, Byte.MIN_VALUE, -33, -28, -82, -48, -61, 45}, new byte[]{-13, -70, -106, -40, -71, -96, 72, -103}));
        c0204WWWWoWWWWo.m791WWWWWWWW(2, true);
        c0204WWWWoWWWWo.f1165WoWo = true;
        c0204WWWWoWWWWo.f1147WWWWWWWW = 1;
        c0204WWWWoWWWWo.f1139WWWWWWWW = -1;
        c0204WWWWoWWWWo.f1132WWWWoWWWWo.icon = R.mipmap.ic_stat_vm_core;
        c0204WWWWoWWWWo.f1135WWWWWWWW = C0204WWWWoWWWWo.m779WWWoWWWo(getString(R.string.vm_notification_title));
        c0204WWWWoWWWWo.f1155WWoWWo = C0204WWWWoWWWWo.m779WWWoWWWo(getString(R.string.vm_notification_content));
        c0204WWWWoWWWWo.f1136WWWWWWWW = activity;
        c0204WWWWoWWWWo.f1144WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-8, 13, -101, 61, 1, -30, -64}, new byte[]{-117, 104, -23, TarConstants.LF_GNUTYPE_LONGLINK, 104, -127, -91, 33});
        c0204WWWWoWWWWo.f1161WWoWWo = false;
        Notification m784WWWWWWWW = c0204WWWWoWWWWo.m784WWWWWWWW();
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(m784WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-24, -34, 40, -95, 16, -71, -25, -101, -92, -126}, new byte[]{-118, -85, 65, -51, 116, -111, -55, -75}));
        return m784WWWWWWWW;
    }

    @Override // android.app.Service
    public final IBinder onBind(Intent intent) {
        byte[] bArr = {-92, 61, -99, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 71, 96, TarConstants.LF_MULTIVOLUME, -55, Byte.MIN_VALUE, 6, -73, 84, 80};
        byte[] bArr2 = {-14, 112, -34, TarConstants.LF_CONTIG, TarConstants.LF_DIR, 5, 30, -84};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{104, -84, -68, 74, 85, -25, TarConstants.LF_MULTIVOLUME, Byte.MIN_VALUE}, new byte[]{7, -62, -2, 35, 59, -125, 119, -96}) + this);
        return null;
    }

    @Override // android.app.Service, android.content.ComponentCallbacks
    public final void onConfigurationChanged(Configuration configuration) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(configuration, WWWWWWWW.m17835WWWWWWWW(new byte[]{-82, 5, -54, 80, -37, Byte.MAX_VALUE, -51, -71, -89}, new byte[]{-64, 96, -67, 19, -76, 17, -85, -48}));
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{10, -70, 69, -75, 93, 56, -120, 9, 46, -127, 111, -71, 74}, new byte[]{92, -9, 6, -38, 47, 93, -37, 108});
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-61, -29, 10, -12, 68, TarConstants.LF_LINK, 102, 98, -39, -1, 40, -17, 67, 56, 97, 70, -60, -20, 39, -4, 79, TarConstants.LF_CHR, TarConstants.LF_DIR, 37}, new byte[]{-84, -115, 73, -101, 42, 87, 15, 5}) + this);
        super.onConfigurationChanged(configuration);
    }

    @Override // android.app.Service
    public final void onCreate() {
        super.onCreate();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-88, 61, -79, 73, 90, -3, Byte.MAX_VALUE, -102, -116, 6, -101, 69, TarConstants.LF_MULTIVOLUME}, new byte[]{-2, 112, -14, 38, 40, -104, 44, -1});
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-53, -20, 111, -84, 106, -17, 119, -39, -98, -94}, new byte[]{-92, -126, 44, -34, 15, -114, 3, -68}) + this);
    }

    @Override // android.app.Service
    public final void onDestroy() {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{35, -59, TarConstants.LF_MULTIVOLUME, 46, -51, TarConstants.LF_LINK, -40, 30, 7, -2, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 34, -38}, new byte[]{117, -120, 14, 65, -65, 84, -117, 123});
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-60, -58, -73, 81, -49, -88, TarConstants.LF_MULTIVOLUME, 67, -46, -110, -45}, new byte[]{-85, -88, -13, TarConstants.LF_BLK, -68, -36, 63, 44}) + this);
        super.onDestroy();
    }

    @Override // android.app.Service, android.content.ComponentCallbacks
    public final void onLowMemory() {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{32, 33, -104, 65, 91, 44, 125, -17, 4, 26, -78, TarConstants.LF_MULTIVOLUME, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{118, 108, -37, 46, 41, 73, 46, -118});
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, 101, 42, -117, TarConstants.LF_SYMLINK, 126, 9, 65, TarConstants.LF_CONTIG, 121, 31, -34, 101}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_UC, ConstantPoolEntry.CP_InterfaceMethodref, 102, -28, 69, TarConstants.LF_CHR, 108, 44}) + this);
        super.onLowMemory();
    }

    @Override // android.app.Service
    public final void onRebind(Intent intent) {
        byte[] bArr = {-108, TarConstants.LF_FIFO, Byte.MAX_VALUE, 26, -44, -79, 81, 93};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-62, 123, 60, 117, -90, -44, 2, 56, -26, 64, 22, 121, -79}, bArr);
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{85, -73, -97, -68, -23, -107, -58, -70, 0, -7}, new byte[]{58, -39, -51, -39, -117, -4, -88, -34}) + this);
        super.onRebind(intent);
    }

    @Override // android.app.Service
    public final int onStartCommand(Intent intent, int i10, int i11) {
        int i12;
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{122, 9, 2, 98, -47, -53, -114, 3, 94, TarConstants.LF_SYMLINK, 40, 110, -58}, new byte[]{44, 68, 65, 13, -93, -82, -35, 102});
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{7, -5, 32, -24, 122, 24, 46, 74, 7, -8, 30, -3, 117, 14, 96, 41}, new byte[]{104, -107, 115, -100, 27, 106, 90, 9}) + this);
        try {
            Notification m5010WWWWWWWW = m5010WWWWWWWW();
            int i13 = Build.VERSION.SDK_INT;
            if (i13 >= 34) {
                i12 = 1073741824;
            } else {
                i12 = 0;
            }
            if (i13 >= 34) {
                AbstractC0235WWWoWWWo.m914WWWWWWWW(this, m5010WWWWWWWW, i12);
            } else if (i13 >= 29) {
                AbstractC0235WWWoWWWo.m913WWWWWWWW(this, m5010WWWWWWWW, i12);
            } else {
                startForeground(R.string.vm_notification_title, m5010WWWWWWWW);
            }
        } catch (Exception unused) {
            stopSelf();
        }
        return 2;
    }

    @Override // android.app.Service
    public final void onTaskRemoved(Intent intent) {
        byte[] bArr = {36, 16, -126, 7, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 3, -117, -14};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{114, 93, -63, 104, 42, 102, -40, -105, 86, 102, -21, 100, 61}, bArr);
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, 36, TarConstants.LF_GNUTYPE_LONGLINK, TarConstants.LF_GNUTYPE_SPARSE, -125, -50, -58, 118, -100, 37, 105, 87, -108, -97, -76}, new byte[]{-15, 74, 31, TarConstants.LF_SYMLINK, -16, -91, -108, 19}) + this);
        super.onTaskRemoved(intent);
    }

    public final void onTimeout(int i10) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-24, -123, 93, TarConstants.LF_MULTIVOLUME, -34, TarConstants.LF_GNUTYPE_LONGLINK, -76, 3, -52, -66, 119, 65, -55}, new byte[]{-66, -56, 30, 34, -84, 46, -25, 102});
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-78, ConstantPoolEntry.CP_NameAndType, 117, -41, 40, 58, -50, 109, -87, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 1}, new byte[]{-35, 98, 33, -66, 69, 95, -95, 24}) + this);
        super.onTimeout(i10);
    }

    @Override // android.app.Service, android.content.ComponentCallbacks2
    public final void onTrimMemory(int i10) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-121, 116, -80, -36, -45, -72, 27, 19, -93, 79, -102, -48, -60}, new byte[]{-47, 57, -13, -77, -95, -35, 72, 118});
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-19, TarConstants.LF_DIR, 68, 20, -61, -11, -25, 73, -17, TarConstants.LF_BLK, 98, 31, -112, -72}, new byte[]{-126, 91, 16, 102, -86, -104, -86, 44}) + this);
        super.onTrimMemory(i10);
    }

    @Override // android.app.Service
    public final boolean onUnbind(Intent intent) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{69, 41, 104, 9, -107, 104, 105, 98, 97, 18, 66, 5, -126}, new byte[]{19, 100, 43, 102, -25, 13, 58, 7});
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{5, TarConstants.LF_CONTIG, -64, 14, -69, TarConstants.LF_GNUTYPE_SPARSE, 63, -82, 80, 121}, new byte[]{106, 89, -107, 96, -39, 58, 81, -54}) + this);
        return super.onUnbind(intent);
    }
}
