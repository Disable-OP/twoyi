package com.android.vmcore;

import android.database.sqlite.SQLiteDatabase;
import android.util.AtomicFile;
import com.android.vmcore.bridge.VMEventManager;
import com.android.vmcore.event.ResetEvent;
import com.android.vmcore.utils.FileDeleteUtils;
import com.blankj.utilcode.util.WWWW;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import eh.C2467WWWWWWWW;
import java.io.File;
import java.io.FileOutputStream;
import java.io.PrintWriter;
import java.util.ArrayList;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.archivers.zip.UnixStat;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import t4.ComponentCallbacksC4221WWWWWWWW;
import x5.WWWWWWWW;
/* renamed from: com.android.vmcore.WWWWo̐WWWWoȄ̐  reason: invalid class name */
/* loaded from: classes.dex */
public final /* synthetic */ class WWWWoWWWWo implements Runnable {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final /* synthetic */ VMInstance f8956WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final /* synthetic */ int f8957WWWWoWWWWo;

    public /* synthetic */ WWWWoWWWWo(VMInstance vMInstance, int i10) {
        this.f8957WWWWoWWWWo = i10;
        this.f8956WWWWWWWWWW = vMInstance;
    }

    /* JADX WARN: Code restructure failed: missing block: B:103:?, code lost:
        return;
     */
    /* JADX WARN: Code restructure failed: missing block: B:32:0x00f0, code lost:
        com.android.vmcore.StringFog.f8859WWWWWWWW.getClass();
        com.android.vmcore.KLog.m5041WWWWWWWW(r5, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-31, -72, org.apache.commons.compress.archivers.tar.TarConstants.LF_CHR, -75, -19, org.apache.commons.compress.archivers.tar.TarConstants.LF_GNUTYPE_LONGLINK, 100, -46, -20, -111, 1, -57, -6, 93, 114, -61, -50, -4, 42, -118, -88, org.apache.commons.compress.archivers.tar.TarConstants.LF_GNUTYPE_LONGLINK, 116, -59, -39, -71, 57, -125}, new byte[]{-70, -36, 92, -25, -120, 56, 1, -90}));
        r6.m13940WWWWWWWW(new com.android.vmcore.event.ResetEvent(true));
     */
    /* JADX WARN: Removed duplicated region for block: B:107:? A[RETURN, SYNTHETIC] */
    /* JADX WARN: Removed duplicated region for block: B:84:0x0240 A[EXC_TOP_SPLITTER, SYNTHETIC] */
    @Override // java.lang.Runnable
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final void run() {
        File file;
        ArrayList m5320WWWWoWWWWo;
        FileOutputStream fileOutputStream;
        VMInstance vMInstance = this.f8956WWWWWWWWWW;
        switch (this.f8957WWWWoWWWWo) {
            case 0:
                String str = VMInstance.f8925WWWoWWWo;
                VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
                String str2 = vMConfig.f8868WWWWWWWW;
                StringFog.f8859WWWWWWWW.getClass();
                FileDeleteUtils.m5262WWWWWWWW(new File(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{73, -7, 105, -33, -55, 115, -31, -9, 21, -23, 109, -58, -121, 44, -13, -3, 21, -22, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -39, -52, 114, -7, -21, 31}, new byte[]{102, -99, 8, -85, -88, 92, -110, -114})));
                FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-68, 34, 37, 8, 106, 38, 41, -51, -32, TarConstants.LF_SYMLINK, 33, 17, 36, 110, 63, -57, -25, TarConstants.LF_CHR, TarConstants.LF_FIFO, 25, 37, 98, 63, -51}, new byte[]{-109, 70, 68, 124, ConstantPoolEntry.CP_InterfaceMethodref, 9, 90, -76})));
                FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-119, -71, 109, -52, 6, -46, 60, -66, -43, -87, 105, -43, 72, -102, 46, -77, -61, -74, 105, -35, 23, -104, 61, -23, -42, -68, Byte.MAX_VALUE, -53, 16, -110, 61, -93, -120, -74, 105, -63}, new byte[]{-90, -35, ConstantPoolEntry.CP_NameAndType, -72, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -3, 79, -57})));
                FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_FIFO, -1, -85, -95, -91, -111, -100, 5, 106, -17, -81, -72, -21, -39, -114, 8, 124, -16, -81, -80, -76, -37, -99, 82, 105, -6, -66, -95, -95, -52, -127, 82, 114, -2, -77}, new byte[]{25, -101, -54, -43, -60, -66, -17, 124})));
                FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-19, -113, -102, -78, -78, -98, 67, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -79, -97, -98, -85, -4, -35, 95, 66, -87, -104, -98, -78, -89, -40, 94, 70, -79, -59, -97, -92}, new byte[]{-62, -21, -5, -58, -45, -79, TarConstants.LF_NORMAL, 33})));
                FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{33, TarConstants.LF_PAX_EXTENDED_HEADER_LC, TarConstants.LF_GNUTYPE_LONGLINK, -91, -81, -52, -59, 44, 125, 104, 79, -68, -31, -113, -39, TarConstants.LF_FIFO, 101, 111, 79, -91, -70, -118, -40, TarConstants.LF_SYMLINK, 125, TarConstants.LF_SYMLINK, 78, -77, -29, -112, -34, 56}, new byte[]{14, 28, 42, -47, -50, -29, -74, 85})));
                FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{80, -46, 45, 122, -97, 114, -114, -61, ConstantPoolEntry.CP_NameAndType, -62, 41, 99, -47, TarConstants.LF_LINK, -110, -39, 20, -59, 41, 122, -118, TarConstants.LF_BLK, -109, -35, ConstantPoolEntry.CP_NameAndType, -104, 40, 108, -45, 42, -100, -42}, new byte[]{Byte.MAX_VALUE, -74, TarConstants.LF_GNUTYPE_LONGNAME, 14, -2, 93, -3, -70})));
                return;
            case 1:
                String str3 = VMInstance.f8925WWWoWWWo;
                VMConfig vMConfig2 = vMInstance.f8937WWWoWWWo;
                String str4 = vMConfig2.f8868WWWWWWWW;
                byte[] bArr = {Byte.MAX_VALUE, -103, -124, 70, -85, 21, -85, -77, 35, -119, Byte.MIN_VALUE, 95, -27, 79, -85, -81, 34, -114, -54, 2, -27, 73, -67, -66, 36, -108, -117, 85, -71, 101, -85, -71, TarConstants.LF_LINK, -108, -127, 28, -78, 87, -76};
                byte[] bArr2 = {80, -3, -27, TarConstants.LF_SYMLINK, -54, 58, -40, -54};
                StringFog.f8859WWWWWWWW.getClass();
                com.blankj.utilcode.util.WWWWoWWWWo.m5286WWWWWWWW(new File(str4, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)));
                File file2 = new File(vMConfig2.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-107, 115, 87, -27, -59, -100, -19, 18, -55, 99, TarConstants.LF_GNUTYPE_SPARSE, -4, -117, -58, -19, 14, -56, 100, 25, -95, -117, -64, -5, 31, -50, 126, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -10, -41, -20, -19, 14, -39, 98, 68, -12, -118, -53, -13, 7}, new byte[]{-70, 23, TarConstants.LF_FIFO, -111, -92, -77, -98, 107}));
                SQLiteDatabase sQLiteDatabase = null;
                if (file2.exists() && (m5320WWWWoWWWWo = WWWW.m5320WWWWoWWWWo(file2)) != null) {
                    int size = m5320WWWWoWWWWo.size();
                    int i10 = 0;
                    while (true) {
                        if (i10 < size) {
                            Object obj = m5320WWWWoWWWWo.get(i10);
                            i10++;
                            String str5 = (String) obj;
                            StringFog.f8859WWWWWWWW.getClass();
                            if (str5.contains(WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType, -108, -33, TarConstants.LF_DIR, -29, -90, -124, -66, 6, -121, -35, 57, -70, -37, -116, -76, 64}, new byte[]{98, -11, -78, 80, -34, -124, -27, -48}))) {
                                m5320WWWWoWWWWo.remove(str5);
                            }
                        }
                    }
                    AtomicFile atomicFile = new AtomicFile(file2);
                    try {
                        fileOutputStream = atomicFile.startWrite();
                    } catch (Throwable unused) {
                        fileOutputStream = null;
                    }
                    try {
                        PrintWriter printWriter = new PrintWriter(fileOutputStream);
                        int size2 = m5320WWWWoWWWWo.size();
                        int i11 = 0;
                        while (i11 < size2) {
                            Object obj2 = m5320WWWWoWWWWo.get(i11);
                            i11++;
                            printWriter.println((String) obj2);
                        }
                        printWriter.flush();
                        printWriter.close();
                        atomicFile.finishWrite(fileOutputStream);
                        WWWW.m5322WWWWWWWW(fileOutputStream);
                    } catch (Throwable unused2) {
                        try {
                            atomicFile.failWrite(fileOutputStream);
                            WWWW.m5322WWWWWWWW(fileOutputStream);
                            String str6 = vMConfig2.f8868WWWWWWWW;
                            byte[] bArr3 = {-22, 7, -88, TarConstants.LF_GNUTYPE_SPARSE, 58, -65, 111, 41, -79, 2, -26, 68, TarConstants.LF_BLK, -3, 37, 41, -85, 7, -69, 72, TarConstants.LF_SYMLINK, -12, 37, 56, -73, ConstantPoolEntry.CP_NameAndType, -65, 78, 63, -11, 121, 59, -21, 16, -84, TarConstants.LF_GNUTYPE_SPARSE, 47, -7, 101, 47, -74, TarConstants.LF_GNUTYPE_LONGNAME, -83, 70, 47, -15, 105, 41, -74, 6, -70, 8, 40, -11, Byte.MAX_VALUE, 60, -84, 13, -82, 84, 117, -12, 105};
                            byte[] bArr4 = {-59, 99, -55, 39, 91, -112, ConstantPoolEntry.CP_InterfaceMethodref, 72};
                            StringFog.f8859WWWWWWWW.getClass();
                            file = new File(str6, WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4));
                            if (!file.exists()) {
                            }
                        } catch (Throwable th2) {
                            WWWW.m5322WWWWWWWW(fileOutputStream);
                            throw th2;
                        }
                    }
                }
                String str62 = vMConfig2.f8868WWWWWWWW;
                byte[] bArr32 = {-22, 7, -88, TarConstants.LF_GNUTYPE_SPARSE, 58, -65, 111, 41, -79, 2, -26, 68, TarConstants.LF_BLK, -3, 37, 41, -85, 7, -69, 72, TarConstants.LF_SYMLINK, -12, 37, 56, -73, ConstantPoolEntry.CP_NameAndType, -65, 78, 63, -11, 121, 59, -21, 16, -84, TarConstants.LF_GNUTYPE_SPARSE, 47, -7, 101, 47, -74, TarConstants.LF_GNUTYPE_LONGNAME, -83, 70, 47, -15, 105, 41, -74, 6, -70, 8, 40, -11, Byte.MAX_VALUE, 60, -84, 13, -82, 84, 117, -12, 105};
                byte[] bArr42 = {-59, 99, -55, 39, 91, -112, ConstantPoolEntry.CP_InterfaceMethodref, 72};
                StringFog.f8859WWWWWWWW.getClass();
                file = new File(str62, WWWWWWWW.m17835WWWWWWWW(bArr32, bArr42));
                if (!file.exists()) {
                    try {
                        sQLiteDatabase = SQLiteDatabase.openDatabase(file.getAbsolutePath(), null, 0);
                        sQLiteDatabase.execSQL(WWWWWWWW.m17835WWWWWWWW(new byte[]{-53, 23, -61, -23, 15, TarConstants.LF_GNUTYPE_LONGLINK, TarConstants.LF_DIR, -98, -35, 29, -62, -116, 40, 107, 118, -83, -3, TarConstants.LF_CONTIG, -81, -5, 19, TarConstants.LF_GNUTYPE_LONGLINK, 71, -99, -81, 60, -18, -63, 62, TarConstants.LF_CHR, TarConstants.LF_SYMLINK, -71, -31, TarConstants.LF_FIFO, -3, -61, TarConstants.LF_SYMLINK, 106, 74, -79, -21, 117}, new byte[]{-113, 82, -113, -84, 91, 14, 21, -40}));
                        WWWW.m5322WWWWWWWW(sQLiteDatabase);
                        return;
                    } catch (Throwable th3) {
                        try {
                            th3.printStackTrace();
                            WWWW.m5322WWWWWWWW(sQLiteDatabase);
                            return;
                        } catch (Throwable th4) {
                            WWWW.m5322WWWWWWWW(sQLiteDatabase);
                            throw th4;
                        }
                    }
                }
                return;
            case 2:
                VMEventManager vMEventManager = vMInstance.f8935WWWWWWWW;
                if (vMEventManager != null) {
                    StringFog.f8859WWWWWWWW.getClass();
                    vMEventManager.m5116WWWoWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{-48, -117, -122, -117, -4, -13, -76, 3, -36, -115, -113, -117, -21, -16, -77, 30, -63, -127, -59, -60, -2, -23, -71, 30, -35, -54, -82, -3, -51, -36, -98, TarConstants.LF_DIR, -20, -86, -92, -15, -44, -37, -103, TarConstants.LF_SYMLINK, -14, -80, -94, -22, -45, -50, -113, 33, -14, -86, -82, -23}, new byte[]{-77, -28, -21, -91, -99, -99, -48, 113}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
                    return;
                }
                return;
            case 3:
                VMEventManager vMEventManager2 = vMInstance.f8935WWWWWWWW;
                if (vMEventManager2 != null) {
                    byte[] bArr5 = {TarConstants.LF_LINK, -7, -123, 23, 71, 43, 119, 65, 61, -1, -116, 23, 80, 40, 112, 92, 32, -13, -58, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 69, TarConstants.LF_LINK, 122, 92, 60, -72, -68, 118, 97, 2, 95, 118, 13, -60, -83, 122, 99, ConstantPoolEntry.CP_InterfaceMethodref, 71, 96};
                    byte[] bArr6 = {82, -106, -24, 57, 38, 69, 19, TarConstants.LF_CHR};
                    StringFog.f8859WWWWWWWW.getClass();
                    vMEventManager2.m5116WWWoWWWo(WWWWWWWW.m17835WWWWWWWW(bArr5, bArr6), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
                    return;
                }
                return;
            case 4:
                String str7 = VMInstance.f8925WWWoWWWo;
                vMInstance.getClass();
                byte[] bArr7 = {-37, 31, 2, -14, 105, 3, -95, 40, -42, TarConstants.LF_FIFO, TarConstants.LF_NORMAL, Byte.MIN_VALUE, Byte.MAX_VALUE, 24, -79, 40, -28, 20, 26, -50, 44, 6, -87};
                byte[] bArr8 = {Byte.MIN_VALUE, 123, 109, -96, ConstantPoolEntry.CP_NameAndType, 112, -60, 92};
                StringFog.f8859WWWWWWWW.getClass();
                String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(bArr7, bArr8);
                String str8 = VMInstance.f8925WWWoWWWo;
                KLog.m5041WWWWWWWW(str8, m17835WWWWWWWW);
                boolean m5057WWWWWWWW = vMInstance.m5057WWWWWWWW();
                C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
                if (!m5057WWWWWWWW) {
                    KLog.m5040WWWWoWWWWo(str8, WWWWWWWW.m17835WWWWWWWW(new byte[]{27, Byte.MIN_VALUE, -43, 72, 101, -29, 109, -7, 22, -87, -25, 58, 115, -8, 125, -7, 36, -117, -51, 116, 32, -26, 101, -83, 38, -123, -45, 118, 101, -12}, new byte[]{64, -28, -70, 26, 0, -112, 8, -115}));
                    c2467wwwwwwww.m13940WWWWWWWW(new ResetEvent(false));
                    return;
                }
                KLog.m5041WWWWWWWW(str8, WWWWWWWW.m17835WWWWWWWW(new byte[]{36, -49, 118, TarConstants.LF_CHR, -68, 56, 95, -18, 41, -26, 68, 65, -70, 39, 95, -5, 13, -117, 111, ConstantPoolEntry.CP_NameAndType, -7, 45, TarConstants.LF_GNUTYPE_SPARSE, -10, 26, -40}, new byte[]{Byte.MAX_VALUE, -85, 25, 97, -39, TarConstants.LF_GNUTYPE_LONGLINK, 58, -102}));
                VMConfig vMConfig3 = vMInstance.f8937WWWoWWWo;
                vMConfig3.f8919WWWW = false;
                vMInstance.f8926WWWWoWWWWo.edit().remove(WWWWWWWW.m17835WWWWWWWW(new byte[]{-114, 5, -12, 95, 42, 122, -101, -66}, new byte[]{-26, 100, -121, 0, 67, 20, -14, -54})).apply();
                File file3 = new File(vMConfig3.f8868WWWWWWWW);
                int i12 = 0;
                while (true) {
                    int i13 = i12 + 1;
                    if (i12 < 10) {
                        NativeHelper.chmodRecursively(file3.getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
                        NativeHelper.deleteRecursively(file3.getAbsolutePath());
                        File[] listFiles = file3.listFiles();
                        if (listFiles != null && listFiles.length != 0) {
                            try {
                                Thread.sleep(1000L);
                            } catch (Throwable unused3) {
                            }
                            i12 = i13;
                        }
                    } else {
                        byte[] bArr9 = {3, -72, TarConstants.LF_FIFO, 107, -119, 96, -48, 68, 14, -111, 4, 25, -113, Byte.MAX_VALUE, -48, 81, 42, -4, 47, 84, -52, 117, -36, 92, 61, -81, 121, 95, -115, 122, -39, 85, 60};
                        byte[] bArr10 = {TarConstants.LF_PAX_EXTENDED_HEADER_UC, -36, 89, 57, -20, 19, -75, TarConstants.LF_NORMAL};
                        StringFog.f8859WWWWWWWW.getClass();
                        KLog.m5040WWWWoWWWWo(str8, WWWWWWWW.m17835WWWWWWWW(bArr9, bArr10));
                        c2467wwwwwwww.m13940WWWWWWWW(new ResetEvent(false));
                        return;
                    }
                }
                break;
            case 5:
                ComponentCallbacksC4221WWWWWWWW.f34042WWWWoWWWWo.getClass();
                ComponentCallbacksC4221WWWWWWWW.m17176WWWoWWWo(ComponentCallbacksC4221WWWWWWWW.m17173WWWoWWWo(), ComponentCallbacksC4221WWWWWWWW.m17179WWoWWo(), vMInstance);
                return;
            case 6:
                ComponentCallbacksC4221WWWWWWWW.f34042WWWWoWWWWo.m17188WW(vMInstance);
                ComponentCallbacksC4221WWWWWWWW.m17158WWWWWWWWWW(true);
                return;
            case 7:
                ComponentCallbacksC4221WWWWWWWW.f34042WWWWoWWWWo.m17187WWWWWWWW(vMInstance, 0);
                ComponentCallbacksC4221WWWWWWWW.m17165WWWWWWWW();
                ComponentCallbacksC4221WWWWWWWW.m17158WWWWWWWWWW(true);
                ComponentCallbacksC4221WWWWWWWW.m17180WWoWWo();
                return;
            case 8:
                vMInstance.m5086WWoWWo(3, 1);
                return;
            case 9:
                vMInstance.m5086WWoWWo(4, 1);
                return;
            default:
                ComponentCallbacksC4221WWWWWWWW.f34042WWWWoWWWWo.m17188WW(vMInstance);
                ComponentCallbacksC4221WWWWWWWW.m17158WWWWWWWWWW(true);
                return;
        }
    }
}
