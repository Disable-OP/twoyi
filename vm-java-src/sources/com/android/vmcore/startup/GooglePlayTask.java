package com.android.vmcore.startup;

import android.net.Uri;
import android.util.Log;
import com.android.vmapp.VMApp;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.NativeHelper;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.VMStatusEvent;
import com.android.vmcore.installer.ImageInstallerV1;
import com.android.vmcore.utils.ClearAppHelper;
import java.io.File;
import java.util.ArrayList;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.archivers.zip.UnixStat;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class GooglePlayTask implements IVMStartupTask {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9269WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static void m5218WWWWWWWW(VMConfig vMConfig) {
        String str = vMConfig.f8868WWWWWWWW;
        StringFog.f8859WWWWWWWW.getClass();
        NativeHelper.chmodRecursively(new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-115, 31, -45, TarConstants.LF_LINK, 107, -90, -88, 97, -46, 30, -61, TarConstants.LF_BLK, TarConstants.LF_SYMLINK, -94, -75, 62, -115, 43, -59, 45, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -81, -96, 29, -57, 30, -36, 43, 124, -90, -74, 8, -48, 13, -57, 39, 104, -84, -73, 37, -115}, new byte[]{-94, 108, -86, 66, 31, -61, -59, 78})).getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        NativeHelper.chmodRecursively(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{32, 125, -40, 86, 71, 58, -30, 9, Byte.MAX_VALUE, 124, -56, TarConstants.LF_GNUTYPE_SPARSE, 30, 62, -1, 86, 32, 94, -55, 74, 93, 58, -4, TarConstants.LF_MULTIVOLUME, 118, 33}, new byte[]{15, 14, -95, 37, TarConstants.LF_CHR, 95, -113, 38})).getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        NativeHelper.chmodRecursively(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-79, 43, 100, 99, 63, TarConstants.LF_DIR, -11, 78, -18, 42, 116, 102, 102, TarConstants.LF_LINK, -24, 17, -79, 8, 111, 117, 41, 37, -15, 13, -22, 31, 112, 99, 8, 63, -22, 4, -79}, new byte[]{-98, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 29, 16, TarConstants.LF_GNUTYPE_LONGLINK, 80, -104, 97})).getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public static boolean m5219WWWWWWWW(VMConfig vMConfig) {
        String str = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {101, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -118, -65, 82, 60, TarConstants.LF_DIR, 37, 58, 89, -102, -70, ConstantPoolEntry.CP_InterfaceMethodref, 56, 40, 122, 101, 108, -100, -93, 65, TarConstants.LF_DIR, 61, 89, 47, 89, -123, -91, 69, 60, 43, TarConstants.LF_GNUTYPE_LONGNAME, 56, 74, -98, -87, 81, TarConstants.LF_FIFO, 42, 97, 101, 108, -100, -93, 65, TarConstants.LF_DIR, 61, 89, 47, 89, -123, -91, 69, 60, 43, TarConstants.LF_GNUTYPE_LONGNAME, 56, 74, -98, -87, 81, TarConstants.LF_FIFO, 42, 97, 100, 74, -125, -89};
        byte[] bArr2 = {74, 43, -13, -52, 38, 89, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 10};
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        File file2 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-36, Byte.MAX_VALUE, -111, 122, -103, -74, 29, -83, -125, 126, -127, Byte.MAX_VALUE, -64, -78, 0, -14, -36, 92, Byte.MIN_VALUE, 102, -125, -74, 3, -23, -118, 35, -72, 97, -126, -67, 21, -15, -104, 117, -58, 104, -99, -72}, new byte[]{-13, ConstantPoolEntry.CP_NameAndType, -24, 9, -19, -45, 112, -126}));
        File file3 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-59, -96, -49, 87, -125, -126, -90, 72, -102, -95, -33, 82, -38, -122, -69, 23, -59, -125, -60, 65, -107, -110, -94, ConstantPoolEntry.CP_InterfaceMethodref, -98, -108, -37, 87, -76, -120, -71, 2, -59, -125, -60, 65, -107, -110, -94, ConstantPoolEntry.CP_InterfaceMethodref, -98, -108, -37, 87, -76, -120, -71, 2, -60, -78, -58, 79}, new byte[]{-22, -45, -74, 36, -9, -25, -53, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}));
        if (file.exists() && file2.exists() && file3.exists()) {
            return true;
        }
        return false;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5037WWWWoWWWWo() {
        return false;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5038WWWWWWWW() {
        return this.f9269WWWWWWWW;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5039WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        boolean z10 = true;
        if (!vMConfig.f8886WWWWWWWW) {
            if (m5219WWWWWWWW(vMConfig)) {
                byte[] bArr = {91, TarConstants.LF_DIR, -39, -42, -48, 90, -34, -52, 84, 63, -102, -103, -39, 81, -61, -60, 81, 62, -102, -97, -60, TarConstants.LF_GNUTYPE_SPARSE};
                byte[] bArr2 = {56, 90, -76, -8, -73, TarConstants.LF_DIR, -79, -85};
                StringFog.f8859WWWWWWWW.getClass();
                vMInstance.m5053WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                vMInstance.m5053WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{46, 1, -126, -10, -79, -86, -127, TarConstants.LF_GNUTYPE_SPARSE, 33, ConstantPoolEntry.CP_InterfaceMethodref, -63, -71, -72, -95, -100, 91, 36, 10, -63, -65, -69, -74}, new byte[]{TarConstants.LF_MULTIVOLUME, 110, -17, -40, -42, -59, -18, TarConstants.LF_BLK}));
                vMInstance.m5053WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{-103, -93, -95, -49, 0, -109, -19, -117, -107, -91, -88, -49, 23, -104, -25, -99, -109, -94, -85}, new byte[]{-6, -52, -52, -31, 97, -3, -119, -7}));
            }
            byte[] bArr3 = {42, -28, -37, -52, TarConstants.LF_BLK, 119, -40, -119, 31, -3, -35, -56, 61, 97, -51, -98, ConstantPoolEntry.CP_NameAndType, -26, -47, -36, TarConstants.LF_CONTIG, 96, -32};
            byte[] bArr4 = {109, -117, -76, -85, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 18, -117, -20};
            StringFog.f8859WWWWWWWW.getClass();
            ClearAppHelper.m5245WWWWWWWW(vMConfig, WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4), WWWWWWWW.m17835WWWWWWWW(new byte[]{87, 111, 119, -72, 109, -2, -123, 93, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 101, TarConstants.LF_BLK, -9, 100, -11, -104, 85, 93, 100, TarConstants.LF_BLK, -15, 121, -9}, new byte[]{TarConstants.LF_BLK, 0, 26, -106, 10, -111, -22, 58}));
            ClearAppHelper.m5245WWWWWWWW(vMConfig, WWWWWWWW.m17835WWWWWWWW(new byte[]{24, -82, -126, 114, -94, -121, -17, -23}, new byte[]{72, -58, -19, 28, -57, -12, -124, -112}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-52, -119, 9, -71, 36, TarConstants.LF_GNUTYPE_LONGNAME, -25, -102, -64, -113, 0, -71, TarConstants.LF_CHR, 71, -19, -116, -58, -120, 3}, new byte[]{-81, -26, 100, -105, 69, 34, -125, -24}));
            ClearAppHelper.m5245WWWWWWWW(vMConfig, WWWWWWWW.m17835WWWWWWWW(new byte[]{-60, -79, -116, -76, TarConstants.LF_GNUTYPE_SPARSE, 116, 101, 70, -45, -82, -102, -107, 73, 111, 108}, new byte[]{-108, -61, -23, -42, 38, 29, 9, TarConstants.LF_SYMLINK}), WWWWWWWW.m17835WWWWWWWW(new byte[]{27, 106, 2, -14, 63, -89, 111, 0, 20, 96, 65, -67, TarConstants.LF_FIFO, -84, 114, 8, 17, 97, 65, -69, TarConstants.LF_DIR, -69}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, 5, 111, -36, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -56, 0, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}));
            return true;
        }
        byte[] bArr5 = {-51, 8, -36, -117, -109, 0, -116, -10, -62, 2, -97, -60, -102, ConstantPoolEntry.CP_InterfaceMethodref, -111, -2, -57, 3, -97, -62, -121, 9};
        byte[] bArr6 = {-82, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -79, -91, -12, 111, -29, -111};
        StringFog.f8859WWWWWWWW.getClass();
        vMInstance.m5063WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(bArr5, bArr6));
        vMInstance.m5063WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_FIFO, -80, TarConstants.LF_CONTIG, -98, -123, -101, TarConstants.LF_SYMLINK, ConstantPoolEntry.CP_InterfaceMethodref, 57, -70, 116, -47, -116, -112, 47, 3, 60, -69, 116, -41, -113, -121}, new byte[]{85, -33, 90, -80, -30, -12, 93, 108}));
        vMInstance.m5063WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{-127, 13, -113, -74, -30, 58, -12, -66, -115, ConstantPoolEntry.CP_InterfaceMethodref, -122, -74, -11, TarConstants.LF_LINK, -2, -88, -117, ConstantPoolEntry.CP_NameAndType, -123}, new byte[]{-30, 98, -30, -104, -125, 84, -112, -52}));
        if (m5219WWWWWWWW(vMConfig) && WWWWWWWW.m17835WWWWWWWW(new byte[]{-10, -52, -119, -102}, new byte[]{-104, -93, -25, -1, 112, -108, -72, 74}).equals(vMConfig.f8923WoWo)) {
            return true;
        }
        vMInstance.f8939WWWoWWWo.m13942WWWWWWWW(new VMStatusEvent(WWWWWWWW.m17835WWWWWWWW(new byte[]{-2, TarConstants.LF_GNUTYPE_LONGNAME, -89, -12, 42, 115, -27, 43, -25, 78, -75, -7}, new byte[]{-105, 34, -44, Byte.MIN_VALUE, TarConstants.LF_GNUTYPE_LONGLINK, 31, -119, 116})));
        String[] strArr = new String[1];
        try {
            ArrayList arrayList = new ArrayList();
            arrayList.add(Uri.parse(vMConfig.f8895WWWoWWWo.f8856WWoWWo));
            String absolutePath = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_SPARSE, -19, 43, -76, 16, -121, -93, -24, ConstantPoolEntry.CP_NameAndType, -20, 59, -79, 73, -125, -66, -73}, new byte[]{124, -98, 82, -57, 100, -30, -50, -57})).getAbsolutePath();
            Uri uri = (Uri) arrayList.get(0);
            new ImageInstallerV1(vMApp).m5205WWWoWWWo(vMConfig, arrayList, absolutePath, null);
            m5218WWWWWWWW(vMConfig);
        } catch (Throwable th2) {
            strArr[0] = Log.getStackTraceString(th2);
            z10 = false;
        }
        if (!z10) {
            vMInstance.m5071WWWWWWWW(false);
            this.f9269WWWWWWWW = strArr[0];
        }
        return z10;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-120, -25, 98, -76, -74, -68, -113, -58, -82, -15, 89, -78, -87, -78}, new byte[]{-49, -120, 13, -45, -38, -39, -33, -86});
    }
}
