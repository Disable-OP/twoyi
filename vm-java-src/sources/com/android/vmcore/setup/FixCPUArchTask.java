package com.android.vmcore.setup;

import android.util.Log;
import com.android.vmapp.VMApp;
import com.android.vmcore.IVMSetupTask;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import com.android.vmcore.utils.CPUUtils;
import com.blankj.utilcode.util.WWWW;
import java.io.Closeable;
import java.io.File;
import java.io.FileOutputStream;
import java.io.PrintWriter;
import java.util.ArrayList;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class FixCPUArchTask implements IVMSetupTask {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public int f9251WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9252WWWWWWWW;

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5034WWWWoWWWWo() {
        return true;
    }

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5035WWWWWWWW() {
        return this.f9252WWWWWWWW;
    }

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5036WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        boolean z10;
        PrintWriter printWriter;
        PrintWriter printWriter2;
        int i10 = 21;
        if (CPUUtils.m5240WWWWoWWWWo()) {
            return true;
        }
        try {
            String str = vMInstance.f8937WWWoWWWo.f8868WWWWWWWW;
            StringFog.f8859WWWWWWWW.getClass();
            File file = new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{46, -108, -37, 104, 100, -51, 13, -10, 47, Byte.MIN_VALUE, -52, 97, 117}, new byte[]{1, -16, -66, 14, 5, -72, 97, -126}));
            ArrayList m5320WWWWoWWWWo = WWWW.m5320WWWWoWWWWo(file);
            PrintWriter printWriter3 = new PrintWriter(new FileOutputStream(file));
            if (m5320WWWWoWWWWo != null) {
                try {
                    int size = m5320WWWWoWWWWo.size();
                    int i11 = 0;
                    while (i11 < size) {
                        Object obj = m5320WWWWoWWWWo.get(i11);
                        i11++;
                        z10 = false;
                        try {
                            String str2 = (String) obj;
                            byte[] bArr = new byte[i10];
                            // fill-array-data instruction
                            bArr[0] = -2;
                            bArr[1] = -82;
                            bArr[2] = 116;
                            bArr[3] = -125;
                            bArr[4] = 71;
                            bArr[5] = -21;
                            bArr[6] = -107;
                            bArr[7] = -26;
                            bArr[8] = -23;
                            bArr[9] = -4;
                            bArr[10] = 32;
                            bArr[11] = Byte.MIN_VALUE;
                            bArr[12] = 89;
                            bArr[13] = -29;
                            bArr[14] = -114;
                            bArr[15] = -9;
                            bArr[16] = -70;
                            bArr[17] = -11;
                            bArr[18] = 5;
                            bArr[19] = -54;
                            bArr[20] = 12;
                            StringFog.f8859WWWWWWWW.getClass();
                            if (str2.equals(WWWWWWWW.m17835WWWWWWWW(bArr, new byte[]{-116, -63, 90, -7, 62, -116, -6, -110}))) {
                                printWriter3.println(WWWWWWWW.m17835WWWWWWWW(new byte[]{-90, -33, -111, -127, 14, 122, 24, -127, -79, -115, -59, -126, 16, 114, 3, -112, -30, -124}, new byte[]{-44, -80, -65, -5, 119, 29, 119, -11}));
                            } else {
                                printWriter3.println(str2);
                            }
                            i10 = 21;
                        } catch (Throwable th2) {
                            th = th2;
                            printWriter = printWriter3;
                            try {
                                this.f9252WWWWWWWW = Log.getStackTraceString(th);
                                this.f9251WWWWoWWWWo = 108000;
                                Closeable[] closeableArr = new Closeable[1];
                                closeableArr[z10 ? 1 : 0] = printWriter;
                                WWWW.m5335WWWoWWWo(closeableArr);
                                return z10;
                            } catch (Throwable th3) {
                                Closeable[] closeableArr2 = new Closeable[1];
                                closeableArr2[z10 ? 1 : 0] = printWriter;
                                WWWW.m5335WWWoWWWo(closeableArr2);
                                throw th3;
                            }
                        }
                    }
                } catch (Throwable th4) {
                    th = th4;
                    z10 = false;
                    printWriter = printWriter3;
                    this.f9252WWWWWWWW = Log.getStackTraceString(th);
                    this.f9251WWWWoWWWWo = 108000;
                    Closeable[] closeableArr3 = new Closeable[1];
                    closeableArr3[z10 ? 1 : 0] = printWriter;
                    WWWW.m5335WWWoWWWo(closeableArr3);
                    return z10;
                }
            }
            z10 = false;
            z10 = false;
            printWriter3.flush();
            printWriter3.close();
            WWWW.m5335WWWoWWWo(printWriter3);
            try {
                String str3 = vMInstance.f8937WWWoWWWo.f8868WWWWWWWW;
                StringFog.f8859WWWWWWWW.getClass();
                File file2 = new File(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{91, 35, -125, 21, -52, TarConstants.LF_BLK, -34, -125, 22, 37, -109, 10, -36, Byte.MAX_VALUE, -61, -34, 27, 32}, new byte[]{116, 80, -6, 102, -72, 81, -77, -84}));
                ArrayList m5320WWWWoWWWWo2 = WWWW.m5320WWWWoWWWWo(file2);
                PrintWriter printWriter4 = new PrintWriter(new FileOutputStream(file2));
                if (m5320WWWWoWWWWo2 != null) {
                    try {
                        int size2 = m5320WWWWoWWWWo2.size();
                        int i12 = 0;
                        while (i12 < size2) {
                            Object obj2 = m5320WWWWoWWWWo2.get(i12);
                            i12++;
                            String str4 = (String) obj2;
                            WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
                            wwwwwwww.getClass();
                            if (str4.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-95, -82, -108, 16, -26, -36, -62, TarConstants.LF_GNUTYPE_LONGLINK, -80, -75, -108, 3, -28, -58, -120, 95, -79, -88, -121}, new byte[]{-45, -63, -70, 96, -108, -77, -90, 62}))) {
                                byte[] bArr2 = {-74, 95, -105, 10, -22, TarConstants.LF_LINK, 126, -88, -89, 68, -105, 25, -24, 43, TarConstants.LF_BLK, -68, -90, 89, -124, 27, -22, TarConstants.LF_CHR, 44, -23, -23, 70, -127, 27};
                                byte[] bArr3 = {-60, TarConstants.LF_NORMAL, -71, 122, -104, 94, 26, -35};
                                wwwwwwww.getClass();
                                printWriter4.println(WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                            } else {
                                byte[] bArr4 = {119, -46, TarConstants.LF_CHR, TarConstants.LF_FIFO, 22, 96, 16, 74};
                                wwwwwwww.getClass();
                                if (str4.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{5, -67, 29, 70, 100, 15, 116, 63, 20, -90, 29, 85, 102, 21, 62, 43, 21, -69, 95, 95, 101, 20, 45}, bArr4))) {
                                    byte[] bArr5 = {-55, 107, TarConstants.LF_NORMAL, 106, -98, -17, 39, 114};
                                    wwwwwwww.getClass();
                                    printWriter4.println(WWWWWWWW.m17835WWWWWWWW(new byte[]{-69, 4, 30, 26, -20, Byte.MIN_VALUE, 67, 7, -86, 31, 30, 9, -18, -102, 9, 19, -85, 2, 92, 3, -19, -101, 26, 19, -69, 6, 6, 94, -77, -103, 31, 19}, bArr5));
                                } else {
                                    wwwwwwww.getClass();
                                    if (str4.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-18, -42, TarConstants.LF_CHR, 0, -59, 72, 8, 85, -1, -51, TarConstants.LF_CHR, 19, -57, 82, 66, 65, -2, -48, 113, 25, -60, TarConstants.LF_GNUTYPE_SPARSE, 95, 18, -95}, new byte[]{-100, -71, 29, 112, -73, 39, 108, 32}))) {
                                        wwwwwwww.getClass();
                                        printWriter4.println(WWWWWWWW.m17835WWWWWWWW(new byte[]{-22, 71, -22, -45, 114, TarConstants.LF_SYMLINK, -93, 122, -5, 92, -22, -64, 112, 40, -23, 110, -6, 65, -88, -54, 115, 41, -12, 61, -91}, new byte[]{-104, 40, -60, -93, 0, 93, -57, 15}));
                                    } else {
                                        wwwwwwww.getClass();
                                        if (str4.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-17, -120, -83, -97, 17, 96, 10, -60, -2, -109, -83, -116, 19, 122, 64, -48, -1, -114, -17, -122, 16, 123, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -123, -96}, new byte[]{-99, -25, -125, -17, 99, 15, 110, -79}))) {
                                            byte[] bArr6 = {TarConstants.LF_CONTIG, -26, 60, 63, -85, -33, 3, -8, 38, -3, 60, 44, -87, -59, 73, -20, 39, -32, 126, 38, -86, -60, 81, -71, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -24, 96, 34, -17, -124, 74, -5, 125, -24};
                                            byte[] bArr7 = {69, -119, 18, 79, -39, -80, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -115};
                                            wwwwwwww.getClass();
                                            printWriter4.println(WWWWWWWW.m17835WWWWWWWW(bArr6, bArr7));
                                        } else {
                                            printWriter4.println(str4);
                                        }
                                    }
                                }
                            }
                        }
                    } catch (Throwable th5) {
                        th = th5;
                        printWriter2 = printWriter4;
                        try {
                            this.f9252WWWWWWWW = Log.getStackTraceString(th);
                            this.f9251WWWWoWWWWo = 108000;
                            WWWW.m5335WWWoWWWo(printWriter2);
                            return z10;
                        } catch (Throwable th6) {
                            WWWW.m5335WWWoWWWo(printWriter2);
                            throw th6;
                        }
                    }
                }
                printWriter4.flush();
                printWriter4.close();
                WWWW.m5335WWWoWWWo(printWriter4);
                return true;
            } catch (Throwable th7) {
                th = th7;
                printWriter2 = null;
            }
        } catch (Throwable th8) {
            th = th8;
            z10 = false;
            printWriter = null;
        }
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final int getErrorCode() {
        return this.f9251WWWWoWWWWo;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final String getName() {
        byte[] bArr = {-31, -113, -22, -6, -106, 16, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 19};
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-89, -26, -110, -71, -58, 69, 25, 97, -126, -25, -66, -101, -27, 123}, bArr);
    }
}
