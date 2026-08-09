package com.android.vmcore.service;

import android.app.Service;
import android.content.BroadcastReceiver;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.ServiceConnection;
import android.os.Build;
import android.os.IBinder;
import android.os.Parcel;
import android.util.SparseArray;
import com.android.vmapp.VMApp;
import com.android.vmcore.KLog;
import com.android.vmcore.StringFog;
import com.android.vmcore.service.IBinderService;
import com.blankj.utilcode.util.WoWo;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class BinderService extends Service {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public static final String f9239WWWWWWWWWW;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public static final SparseArray f9240WWWWWWWW;

    /* renamed from: WWWWཀྵWWWWࠤཀྵ  reason: contains not printable characters */
    public static final SparseArray f9241WWWWWWWW;

    /* renamed from: WWoၑWWoړၑ  reason: contains not printable characters */
    public static final SparseArray f9242WWoWWo;

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public static final SparseArray f9243WWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final IBinderService.Stub f9244WWWWoWWWWo = new IBinderService.Stub();

    /* renamed from: com.android.vmcore.service.BinderService$1  reason: invalid class name */
    /* loaded from: classes.dex */
    public class AnonymousClass1 extends IBinderService.Stub {
    }

    /* JADX INFO: Access modifiers changed from: package-private */
    /* renamed from: com.android.vmcore.service.BinderService$3  reason: invalid class name */
    /* loaded from: classes.dex */
    public class AnonymousClass3 extends BroadcastReceiver {
        @Override // android.content.BroadcastReceiver
        public final void onReceive(Context context, Intent intent) {
        }
    }

    static {
        StringFog.f8859WWWWWWWW.getClass();
        f9239WWWWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{84, -101, 44, -111, -82, -28, -50, -36, 100, -124, 43, -106, -82}, new byte[]{22, -14, 66, -11, -53, -106, -99, -71});
        f9240WWWWWWWW = new SparseArray();
        f9243WWWW = new SparseArray();
        f9241WWWWWWWW = new SparseArray();
        f9242WWoWWo = new SparseArray();
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static int m5206WWWWoWWWWo(VMApp vMApp, int i10) {
        Intent intent = new Intent();
        String packageName = vMApp.getPackageName();
        byte[] bArr = {57, -61, 9, -70, 109, 69, -29, -33, TarConstants.LF_DIR, -59, 0, -70, 122, 70, -28, -62, 40, -55, 74, -25, 105, 89, -15, -60, 57, -55, 74, -42, 101, 69, -29, -56, 40, -1, 1, -26, 122, 66, -28, -56};
        byte[] bArr2 = {90, -84, 100, -108, ConstantPoolEntry.CP_NameAndType, 43, -121, -83};
        StringFog.f8859WWWWWWWW.getClass();
        intent.setComponent(new ComponentName(packageName, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)));
        int m5207WWWWWWWW = m5207WWWWWWWW(vMApp);
        String str = f9239WWWWWWWWWW;
        if (m5207WWWWWWWW == 0) {
            KLog.m5040WWWWoWWWWo(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{6, 34, 102, -24, -25, 67, 92, 90, 0, 114, 112, -21, -27, 10, 65, 93, 41, 114, 116, -17, -1, 10, 95, 87, 56, 57, TarConstants.LF_CHR, -23, -28, 78, 74}, new byte[]{93, 82, 19, -118, -117, 42, 47, TarConstants.LF_SYMLINK}));
            return -501;
        }
        Parcel obtain = Parcel.obtain();
        try {
            obtain.writeInterfaceToken(WWWWWWWW.m17835WWWWWWWW(new byte[]{104, 17, -37, -63, -69, -10, -35, 72, 104, 15, -49, -99, -99, -34, -38, 18, 96, 9, -42, -57, -83, -46, -40, 8, 104, 24, -38, -63}, new byte[]{9, Byte.MAX_VALUE, -65, -77, -44, -97, -71, 102}));
            obtain.writeInt(1);
            intent.writeToParcel(obtain, 0);
            obtain.writeString(null);
            obtain.writeString(vMApp.getPackageName());
            int i11 = setupBinder(i10, m5207WWWWWWWW, 1, 2, WWWWWWWW.m17835WWWWWWWW(new byte[]{86, 123, 101, -68, 121, Byte.MAX_VALUE, 112, -65, 90, 125, 108, -68, 110, 124, 119, -94, 71, 113, 38, -31, 125, 99, 98, -92, 86, 113, 38, -37, 90, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 122, -87, 80, 102, 91, -9, 106, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 125, -82, 80}, new byte[]{TarConstants.LF_DIR, 20, 8, -110, 24, 17, 20, -51}), obtain.marshall());
            if (i11 != 0) {
                KLog.m5040WWWWoWWWWo(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{57, -101, -82, -54, 116, 125, 98, -6, 63, -53, -88, -51, 108, 97, 97, -78, 0, -126, -75, -52, 125, 102, TarConstants.LF_LINK, -12, 3, -126, -73, -51, 124}, new byte[]{98, -21, -37, -88, 24, 20, 17, -110}));
                obtain.recycle();
                return i11;
            }
            obtain.recycle();
            final CountDownLatch countDownLatch = new CountDownLatch(1);
            ServiceConnection serviceConnection = new ServiceConnection() { // from class: com.android.vmcore.service.BinderService.2
                @Override // android.content.ServiceConnection
                public final void onServiceConnected(ComponentName componentName, IBinder iBinder) {
                    StringFog.f8859WWWWWWWW.getClass();
                    KLog.m5043WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, -108, -90, -94, -80, -81, -1, -40, -8, -117, -95, -91, -80}, new byte[]{-118, -3, -56, -58, -43, -35, -84, -67}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-127, 68, 7, -76, -20, -33, 36, -19, -71, 78, 42, -120, -25, -61, TarConstants.LF_CONTIG, -25, -82, 78, 13, -70}, new byte[]{-38, 43, 105, -25, -119, -83, 82, -124}));
                    countDownLatch.countDown();
                }

                @Override // android.content.ServiceConnection
                public final void onServiceDisconnected(ComponentName componentName) {
                    byte[] bArr3 = {-103, -73, -118, 10, -95, -26, -114, TarConstants.LF_FIFO, -87, -88, -115, 13, -95};
                    byte[] bArr4 = {-37, -34, -28, 110, -60, -108, -35, TarConstants.LF_GNUTYPE_SPARSE};
                    StringFog.f8859WWWWWWWW.getClass();
                    KLog.m5043WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4), WWWWWWWW.m17835WWWWWWWW(new byte[]{-81, -38, 59, -6, -6, -79, 94, 121, -105, -48, 17, -64, -20, -96, 71, 126, -102, -48, TarConstants.LF_FIFO, -35, -6, -89, 117}, new byte[]{-12, -75, 85, -87, -97, -61, 40, 16}));
                    countDownLatch.countDown();
                }
            };
            vMApp.bindService(intent, serviceConnection, 1);
            f9242WWoWWo.put(i10, serviceConnection);
            while (true) {
                try {
                    break;
                } catch (InterruptedException e10) {
                    e10.printStackTrace();
                }
            }
            if (countDownLatch.await(5L, TimeUnit.SECONDS)) {
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5043WWWWWWWW(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-102, -74, 47, -123, -51, -84, -30, 86, -100, -26, 41, -126, -45, -77, -8, 93, -92, -26, 56, -114, -49, -95, -79, 81, -86}, new byte[]{-63, -58, 90, -25, -95, -59, -111, 62}));
                return 0;
            }
            byte[] bArr3 = {115, 30, 60, 57, -30, 59, 111, 100, 117, 78, 58, 62, -4, 36, 117, 111, TarConstants.LF_MULTIVOLUME, 78, 43, TarConstants.LF_SYMLINK, -32, TarConstants.LF_FIFO, 60, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 65, 3, 44, TarConstants.LF_BLK, -5, 38};
            byte[] bArr4 = {40, 110, 73, 91, -114, 82, 28, ConstantPoolEntry.CP_NameAndType};
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5040WWWWoWWWWo(str, WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4));
            return -502;
        } catch (Throwable th2) {
            obtain.recycle();
            throw th2;
        }
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static int m5207WWWWWWWW(VMApp vMApp) {
        char c10;
        int i10;
        Object obj;
        String str = f9239WWWWWWWWWW;
        final int[] iArr = {0};
        final long[] jArr = new long[1];
        try {
            byte[] bArr = {74, -53, TarConstants.LF_GNUTYPE_LONGLINK, 62, -34, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -55, -3, 68, -42, 1, 31, -44, 99, -37, -70, 72, -64, 98, 45, -33, 112, -54, -74, 89};
            byte[] bArr2 = {43, -91, 47, TarConstants.LF_GNUTYPE_LONGNAME, -79, 17, -83, -45};
            WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
            wwwwwwww.getClass();
            WoWo m5357WoWo = WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
            byte[] bArr3 = {TarConstants.LF_BLK, 99, -79, -89, -79, 59, 58, -94, TarConstants.LF_NORMAL, 99};
            byte[] bArr4 = {TarConstants.LF_GNUTYPE_SPARSE, 6, -59, -12, -44, 73, TarConstants.LF_GNUTYPE_LONGNAME, -53};
            wwwwwwww.getClass();
            String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4);
            byte[] bArr5 = {TarConstants.LF_GNUTYPE_SPARSE, -25, 24, -53, 80, -41, TarConstants.LF_DIR, -3};
            byte[] bArr6 = {TarConstants.LF_SYMLINK, -124, 108, -94, 38, -66, 65, -124};
            wwwwwwww.getClass();
            final IBinder iBinder = (IBinder) m5357WoWo.m5361WWWWWWWW(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(bArr5, bArr6)).f9408WWWWoWWWWo;
            if (iBinder == null) {
                try {
                    wwwwwwww.getClass();
                    KLog.m5040WWWWoWWWWo(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{117, -77, -25, -103, -44, -71, -2, -24, 125, -79, -16, -101, -19, -65, -2, -41, 92, -75, -20, -98, -27, -65, -17, -22, 65, -70, -63, -126, -32, -71, -58, -93, 73, -79, -10, -51, -27, -65, -17, -22, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -67, -10, -108, -92, -81, -2, -15, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -67, -31, -120, -92, -70, -6, -22, 66, -79, -26}, new byte[]{46, -44, -126, -19, -124, -36, -101, -125}));
                    return iArr[0];
                } catch (Throwable th2) {
                    th = th2;
                    i10 = 43;
                    c10 = 0;
                    byte[] bArr7 = new byte[i10];
                    // fill-array-data instruction
                    bArr7[0] = 74;
                    bArr7[1] = -71;
                    bArr7[2] = -71;
                    bArr7[3] = -65;
                    bArr7[4] = -6;
                    bArr7[5] = -92;
                    bArr7[6] = -50;
                    bArr7[7] = -66;
                    bArr7[8] = 66;
                    bArr7[9] = -69;
                    bArr7[10] = -82;
                    bArr7[11] = -67;
                    bArr7[12] = -61;
                    bArr7[13] = -94;
                    bArr7[14] = -50;
                    bArr7[15] = -127;
                    bArr7[16] = 99;
                    bArr7[17] = -65;
                    bArr7[18] = -78;
                    bArr7[19] = -72;
                    bArr7[20] = -53;
                    bArr7[21] = -94;
                    bArr7[22] = -33;
                    bArr7[23] = -68;
                    bArr7[24] = 126;
                    bArr7[25] = -80;
                    bArr7[26] = -97;
                    bArr7[27] = -92;
                    bArr7[28] = -50;
                    bArr7[29] = -92;
                    bArr7[30] = -10;
                    bArr7[31] = -11;
                    bArr7[32] = 116;
                    bArr7[33] = -90;
                    bArr7[34] = -65;
                    bArr7[35] = -82;
                    bArr7[36] = -38;
                    bArr7[37] = -75;
                    bArr7[38] = -62;
                    bArr7[39] = -70;
                    bArr7[40] = Byte.MAX_VALUE;
                    bArr7[41] = -28;
                    bArr7[42] = -4;
                    StringFog.f8859WWWWWWWW.getClass();
                    KLog.m5044WWWoWWWo(str, WWWWWWWW.m17835WWWWWWWW(bArr7, new byte[]{17, -34, -36, -53, -86, -63, -85, -43}), th);
                    return iArr[c10];
                }
            }
            IBinder iBinder2 = (IBinder) Proxy.newProxyInstance(vMApp.getClassLoader(), new Class[]{IBinder.class}, new InvocationHandler() { // from class: com.android.vmcore.service.WWWW̏WWWWβ̏
                @Override // java.lang.reflect.InvocationHandler
                public final Object invoke(Object obj2, Method method, Object[] objArr) {
                    String str2 = BinderService.f9239WWWWWWWWWW;
                    long id2 = Thread.currentThread().getId();
                    long[] jArr2 = jArr;
                    if (jArr2[0] == id2) {
                        jArr2[0] = 0;
                        iArr[0] = ((Integer) objArr[0]).intValue();
                    }
                    return method.invoke(iBinder, objArr);
                }
            });
            int i11 = Build.VERSION.SDK_INT;
            try {
                if (i11 >= 26) {
                    byte[] bArr8 = {68, 32, -72, -97, -37, 43, 24, 95, 68, 62, -84, -61, -11, 33, 8, 24, TarConstants.LF_GNUTYPE_SPARSE, 39, -88, -108, -7, 35, 18, 16, 66, 43, -82};
                    c10 = 0;
                    wwwwwwww.getClass();
                    WoWo m5357WoWo2 = WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(bArr8, new byte[]{37, 78, -36, -19, -76, 66, 124, 113}));
                    byte[] bArr9 = {-122, 96, ConstantPoolEntry.CP_NameAndType, 68, 59, 89, 86, 112, -74, 108, 14, 94, TarConstants.LF_CHR, 72, 90, 118, -100, 72, 1, 87, 62, 74, TarConstants.LF_GNUTYPE_LONGLINK, 107, -95};
                    byte[] bArr10 = {-49, 33, 111, TarConstants.LF_NORMAL, 82, 47, 63, 4};
                    wwwwwwww.getClass();
                    WoWo m5358WWWWoWWWWo = m5357WoWo2.m5358WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(bArr9, bArr10));
                    wwwwwwww.getClass();
                    obj = m5358WWWWoWWWWo.m5358WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{74, 59, -66, 22, -54, -84, -9, 74, 66}, new byte[]{39, 114, -48, 101, -66, -51, -103, 41})).f9408WWWWoWWWWo;
                    wwwwwwww.getClass();
                    WoWo m5357WoWo3 = WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, -66, -76, 84, -109, 8, -28, 107, -98, -96, -96, 8, -75, 32, -29, TarConstants.LF_LINK, -106, -90, -71, 82, -123, 44, -31, 43, -98, -73, -75, 84, -40, TarConstants.LF_SYMLINK, -12, TarConstants.LF_NORMAL, -99}, new byte[]{-1, -48, -48, 38, -4, 97, Byte.MIN_VALUE, 69}));
                    wwwwwwww.getClass();
                    WoWo m5361WWWWWWWW = m5357WoWo3.m5361WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{43, 3, -26, -117, -42, -114, -73, 56, 43, 19, -54}, new byte[]{74, 112, -81, -27, -94, -21, -59, 94}), iBinder2);
                    byte[] bArr11 = {-67, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 20, 27, -41, -90, 114, -8, -67, 70, 0, 71, -7, -84, 98, -65, -86, 95, 4, 16, -11, -82, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -73, -69, TarConstants.LF_GNUTYPE_SPARSE, 2};
                    byte[] bArr12 = {-36, TarConstants.LF_FIFO, 112, 105, -72, -49, 22, -42};
                    wwwwwwww.getClass();
                    WoWo m5357WoWo4 = WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(bArr11, bArr12));
                    wwwwwwww.getClass();
                    WoWo m5358WWWWoWWWWo2 = m5357WoWo4.m5358WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{118, 8, -94, -32, 16, -108, -91, 59, 70, 4, -96, -6, 24, -123, -87, 61, 108, 32, -81, -13, 21, -121, -72, 32, 81}, new byte[]{63, 73, -63, -108, 121, -30, -52, 79}));
                    wwwwwwww.getClass();
                    m5358WWWWoWWWWo2.m5363WWWoWWWo(m5361WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{105, -49, -19, -126, -115, -120, 47, -58, 97}, new byte[]{4, -122, -125, -15, -7, -23, 65, -91}));
                } else {
                    c10 = 0;
                    wwwwwwww.getClass();
                    WoWo m5357WoWo5 = WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{10, 40, -111, TarConstants.LF_GNUTYPE_LONGNAME, -90, -51, 67, -110, 10, TarConstants.LF_FIFO, -123, 16, -120, -57, TarConstants.LF_GNUTYPE_SPARSE, -43, 29, 47, -127, 71, -124, -59, 73, -35, ConstantPoolEntry.CP_NameAndType, 35, -121, 112, -88, -48, 78, -54, 14}, new byte[]{107, 70, -11, 62, -55, -92, 39, -68}));
                    wwwwwwww.getClass();
                    WoWo m5358WWWWoWWWWo3 = m5357WoWo5.m5358WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 57, 36, 73, 47, 72, -69, -67}, new byte[]{0, 125, 65, 47, 78, 61, -41, -55}));
                    byte[] bArr13 = {113, -43, -87, ConstantPoolEntry.CP_InterfaceMethodref, -95, -64, 111, 67, 121};
                    byte[] bArr14 = {28, -100, -57, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -43, -95, 1, 32};
                    wwwwwwww.getClass();
                    obj = m5358WWWWoWWWWo3.m5358WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(bArr13, bArr14)).f9408WWWWoWWWWo;
                    wwwwwwww.getClass();
                    WoWo m5357WoWo6 = WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{116, 19, 34, -65, -14, 39, -116, 5, 116, 13, TarConstants.LF_FIFO, -29, -36, 45, -100, 66, 99, 20, TarConstants.LF_SYMLINK, -76, -48, 47, -122, 74, 114, 24, TarConstants.LF_BLK, -125, -4, 58, -127, 93, 112}, new byte[]{21, 125, 70, -51, -99, 78, -24, 43}));
                    wwwwwwww.getClass();
                    WoWo m5361WWWWWWWW2 = m5357WoWo6.m5361WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{26, 102, -92, 58, 82, 102, 40, -116, 26, 118, -120}, new byte[]{123, 21, -19, 84, 38, 3, 90, -22}), iBinder2);
                    wwwwwwww.getClass();
                    WoWo m5357WoWo7 = WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{122, 27, 23, TarConstants.LF_MULTIVOLUME, 40, 74, 25, 70, 122, 5, 3, 17, 6, 64, 9, 1, 109, 28, 7, 70, 10, 66, 19, 9, 124, 16, 1, 113, 38, 87, 20, 30, 126}, new byte[]{27, 117, 115, 63, 71, 35, 125, 104}));
                    wwwwwwww.getClass();
                    WoWo m5358WWWWoWWWWo4 = m5357WoWo7.m5358WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{-46, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_CONTIG, Byte.MAX_VALUE, -8, TarConstants.LF_CHR, -22, -20}, new byte[]{-75, 35, 82, 25, -103, 70, -122, -104}));
                    wwwwwwww.getClass();
                    m5358WWWWoWWWWo4.m5363WWWoWWWo(m5361WWWWWWWW2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-11, 106, -39, 28, 102, 10, 8, -90, -3}, new byte[]{-104, 35, -73, 111, 18, 107, 102, -59}));
                }
                BroadcastReceiver broadcastReceiver = new BroadcastReceiver();
                jArr[c10] = Thread.currentThread().getId();
                broadcastReceiver.peekService(vMApp, new Intent());
                if (i11 >= 26) {
                    wwwwwwww.getClass();
                    WoWo m5357WoWo8 = WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{0, -41, -13, 94, 108, 104, -86, 114, 0, -55, -25, 2, 66, 98, -70, TarConstants.LF_DIR, 23, -48, -29, 85, 78, 96, -96, 61, 6, -36, -27}, new byte[]{97, -71, -105, 44, 3, 1, -50, 92}));
                    wwwwwwww.getClass();
                    WoWo m5358WWWWoWWWWo5 = m5357WoWo8.m5358WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{117, 16, -12, -85, 117, 10, 30, 87, 69, 28, -10, -79, 125, 27, 18, 81, 111, 56, -7, -72, 112, 25, 3, TarConstants.LF_GNUTYPE_LONGNAME, 82}, new byte[]{60, 81, -105, -33, 28, 124, 119, 35}));
                    byte[] bArr15 = {-95, -19, -30, 16, 101, TarConstants.LF_FIFO, -10, -52};
                    wwwwwwww.getClass();
                    m5358WWWWoWWWWo5.m5363WWWoWWWo(obj, WWWWWWWW.m17835WWWWWWWW(new byte[]{-52, -92, -116, 99, 17, 87, -104, -81, -60}, bArr15));
                } else {
                    wwwwwwww.getClass();
                    WoWo m5357WoWo9 = WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{15, -33, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -24, 126, 32, 16, 92, 15, -63, TarConstants.LF_GNUTYPE_LONGNAME, -76, 80, 42, 0, 27, 24, -40, 72, -29, 92, 40, 26, 19, 9, -44, 78, -44, 112, 61, 29, 4, ConstantPoolEntry.CP_InterfaceMethodref}, new byte[]{110, -79, 60, -102, 17, 73, 116, 114}));
                    wwwwwwww.getClass();
                    WoWo m5358WWWWoWWWWo6 = m5357WoWo9.m5358WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{109, -97, -111, -109, 40, -33, -116, -125}, new byte[]{10, -37, -12, -11, 73, -86, -32, -9}));
                    wwwwwwww.getClass();
                    m5358WWWWoWWWWo6.m5363WWWoWWWo(obj, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CHR, 27, 38, -28, -65, -68, 58, -109, 59}, new byte[]{94, 82, 72, -105, -53, -35, 84, -16}));
                }
                return iArr[c10];
            } catch (Throwable th3) {
                th = th3;
                i10 = 43;
                byte[] bArr72 = new byte[i10];
                // fill-array-data instruction
                bArr72[0] = 74;
                bArr72[1] = -71;
                bArr72[2] = -71;
                bArr72[3] = -65;
                bArr72[4] = -6;
                bArr72[5] = -92;
                bArr72[6] = -50;
                bArr72[7] = -66;
                bArr72[8] = 66;
                bArr72[9] = -69;
                bArr72[10] = -82;
                bArr72[11] = -67;
                bArr72[12] = -61;
                bArr72[13] = -94;
                bArr72[14] = -50;
                bArr72[15] = -127;
                bArr72[16] = 99;
                bArr72[17] = -65;
                bArr72[18] = -78;
                bArr72[19] = -72;
                bArr72[20] = -53;
                bArr72[21] = -94;
                bArr72[22] = -33;
                bArr72[23] = -68;
                bArr72[24] = 126;
                bArr72[25] = -80;
                bArr72[26] = -97;
                bArr72[27] = -92;
                bArr72[28] = -50;
                bArr72[29] = -92;
                bArr72[30] = -10;
                bArr72[31] = -11;
                bArr72[32] = 116;
                bArr72[33] = -90;
                bArr72[34] = -65;
                bArr72[35] = -82;
                bArr72[36] = -38;
                bArr72[37] = -75;
                bArr72[38] = -62;
                bArr72[39] = -70;
                bArr72[40] = Byte.MAX_VALUE;
                bArr72[41] = -28;
                bArr72[42] = -4;
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5044WWWoWWWo(str, WWWWWWWW.m17835WWWWWWWW(bArr72, new byte[]{17, -34, -36, -53, -86, -63, -85, -43}), th);
                return iArr[c10];
            }
        } catch (Throwable th4) {
            th = th4;
            c10 = 0;
        }
    }

    public static native int getBinderVersion();

    private static native int setupBinder(int i10, int i11, int i12, int i13, String str, byte[] bArr);

    @Override // android.app.Service
    public final IBinder onBind(Intent intent) {
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(f9239WWWWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{46, 121, -112, Byte.MAX_VALUE, -76, -7, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_MULTIVOLUME}, new byte[]{117, 22, -2, 61, -35, -105, 60, 16}));
        return this.f9244WWWWoWWWWo;
    }

    @Override // android.app.Service
    public final void onCreate() {
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(f9239WWWWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{1, -84, -71, -16, 17, 31, -99, 27, 63, -98}, new byte[]{90, -61, -41, -77, 99, 122, -4, 111}));
        super.onCreate();
    }

    @Override // android.app.Service
    public final void onDestroy() {
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(f9239WWWWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-39, 72, -126, -85, 95, 28, -88, -102, -19, 94, -79}, new byte[]{-126, 39, -20, -17, 58, 111, -36, -24}));
        super.onDestroy();
    }

    @Override // android.app.Service
    public final int onStartCommand(Intent intent, int i10, int i11) {
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(f9239WWWWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-9, 124, -98, TarConstants.LF_DIR, 24, -81, 30, -87, -17, 124, -99, ConstantPoolEntry.CP_InterfaceMethodref, 13, -96, 8, Byte.MIN_VALUE}, new byte[]{-84, 19, -16, 102, 108, -50, 108, -35}));
        return super.onStartCommand(intent, i10, i11);
    }

    @Override // android.app.Service
    public final boolean onUnbind(Intent intent) {
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(f9239WWWWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-37, 114, -62, 27, -97, 112, -47, 97, -28, 64}, new byte[]{Byte.MIN_VALUE, 29, -84, 78, -15, 18, -72, 15}));
        return super.onUnbind(intent);
    }
}
