package com.android.vmcore.startup;

import android.net.Uri;
import android.system.Os;
import android.util.Log;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmapp.VMApp;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.NativeHelper;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.VMStatusEvent;
import com.android.vmcore.installer.ImageInstallerV1;
import com.android.vmcore.utils.ClearAppHelper;
import com.android.vmcore.utils.FileDeleteUtils;
import com.blankj.utilcode.util.WWWW;
import java.io.File;
import java.util.ArrayList;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.archivers.zip.UnixStat;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class SuperuserTask implements IVMStartupTask {

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static final String f9274WWWoWWWo;

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public String f9275WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public File f9276WWWWWWWW;

    static {
        byte[] bArr = {-117, 43, 70, -13, 33, 70, 78, -6, -59, 100, 91, -116, 114, 71, 89, -19, -63, 104, 80, -90, 101, 67, 78, -10, -57, 101, 70, -13, 33, 13, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -30, -37, Byte.MAX_VALUE, 80, -21, 46, 90, 73, -14, -58, 36, 81, -25, 100, 79, 68, -11, -37, 126, 21, -85, 44, 70, 74, -2, -59, 100, 91, -116, 33, 2, ConstantPoolEntry.CP_InterfaceMethodref, -8, -60, 106, 70, -11, 33, 65, 68, -23, -51, 1, 21, -90, 33, 69, 89, -12, -35, 123, 21, -12, 110, TarConstants.LF_MULTIVOLUME, 95, -111, -120, 43, 21, -13, 114, 71, 89, -69, -120, 121, 90, -23, 117, 40};
        byte[] bArr2 = {-88, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_DIR, -122, 1, 34, 43, -101};
        StringFog.f8859WWWWWWWW.getClass();
        f9274WWWoWWWo = WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public static boolean m5225WWWWWWWW(VMConfig vMConfig) {
        String str = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {TarConstants.LF_PAX_EXTENDED_HEADER_UC, 72, TarConstants.LF_SYMLINK, 4, -1, -114, -27, 6, 91, 65, 110, 35, -17, -109, -81, 21, 94, 66, 36, 2, -75, -116, -85, 19, 4, 80, TarConstants.LF_CHR, 29, -84, -41};
        byte[] bArr2 = {43, TarConstants.LF_LINK, 65, 112, -102, -29, -54, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER};
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        if (!file.exists() || new File(file, WWWWWWWW.m17835WWWWWWWW(new byte[]{-42, 29, 81, -49, 35, -112, 92, 65, -9, 70, 78, -50, TarConstants.LF_BLK, -99}, new byte[]{-123, 104, 33, -86, 81, -27, 47, 36})).exists()) {
            File file2 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-99, 41, -6, 125, 20, 65, -37, -27, -98, 32, -90, 90, 4, 92, -111, -10, -101, 35, -20, 123, 94, TarConstants.LF_MULTIVOLUME, -122, -23}, new byte[]{-18, 80, -119, 9, 113, 44, -12, -124}));
            if (!file2.exists() || new File(file2, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, -84, -40, -74, 101, -124, -46, 41, 89, -9, -57, -73, 114, -119}, new byte[]{43, -39, -88, -45, 23, -15, -95, TarConstants.LF_GNUTYPE_LONGNAME})).exists()) {
                File file3 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-43, -105, 116, -108, 27, 84, -21, -123, -101, -108, 125, -56, 60, 68, -10, -49, -120, -111, 126, -126, 29, 30, -43, -33, -118, -127, Byte.MAX_VALUE, -110, 28, 84, -12, -124, -101, -108, 102}, new byte[]{-6, -28, 13, -25, 111, TarConstants.LF_LINK, -122, -86}));
                File file4 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-91, 99, 85, 115, 104, -90, -6, 44, -14, 114, 69, 110, TarConstants.LF_CHR, -89, -10, 102, -25, Byte.MAX_VALUE, 66, 115, 105}, new byte[]{-118, 16, 44, 0, 28, -61, -105, 3}));
                File file5 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, 100, -125, -15, -85, -61, 112, -39, -55, 117, -109, -20, -16, -43, 104}, new byte[]{-79, 23, -6, -126, -33, -90, 29, -10}));
                File file6 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{90, ConstantPoolEntry.CP_InterfaceMethodref, -117, 101, -12, 5, -102, 47, 23, 17, -100, 57, -13, 21}, new byte[]{117, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -14, 22, Byte.MIN_VALUE, 96, -9, 0}));
                if (file3.exists() && file4.exists() && file5.exists() && file6.exists()) {
                    return true;
                }
            }
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
        return this.f9275WWWWoWWWWo;
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final void m5226WWWWWWWW(VMConfig vMConfig) {
        String str = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {30, 79, -28, 40, -47, -72, 45, -117, 73, 94, -12, TarConstants.LF_DIR, -118, -71, 33, -63, 92, TarConstants.LF_GNUTYPE_SPARSE, -13, 40, -48};
        byte[] bArr2 = {TarConstants.LF_LINK, 60, -99, 91, -91, -35, 64, -92};
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        File file2 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, 42, -62, 39, -16, -51, -46, 61, -45, TarConstants.LF_NORMAL, -43, 123, -9, -35}, new byte[]{-79, 89, -69, 84, -124, -88, -65, 18}));
        File file3 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{20, 121, -125, 31, 16, 66, -119, 119, 67, 104, -109, 2, TarConstants.LF_GNUTYPE_LONGLINK, 84, -111}, new byte[]{59, 10, -6, 108, 100, 39, -28, TarConstants.LF_PAX_EXTENDED_HEADER_UC}));
        FileDeleteUtils.m5262WWWWWWWW(file2);
        FileDeleteUtils.m5262WWWWWWWW(file3);
        Os.symlink(file.getAbsolutePath(), file2.getAbsolutePath());
        Os.symlink(file.getAbsolutePath(), file3.getAbsolutePath());
        Os.chmod(file.getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        Os.chmod(file2.getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        Os.chmod(file3.getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        NativeHelper.chmodRecursively(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{80, ConstantPoolEntry.CP_NameAndType, -55, -60, 98, -91, 87, TarConstants.LF_CHR, 30, 15, -64, -104, 69, -75, 74, 121, 13, 10, -61, -46, 100, -17}, new byte[]{Byte.MAX_VALUE, Byte.MAX_VALUE, -80, -73, 22, -64, 58, 28})).getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        StringBuilder sb2 = new StringBuilder();
        ArrayList m5320WWWWoWWWWo = WWWW.m5320WWWWoWWWWo(this.f9276WWWWWWWW);
        String str2 = f9274WWWoWWWo;
        if (m5320WWWWoWWWWo != null) {
            int length = str2.split("\n").length;
            int size = m5320WWWWoWWWWo.size();
            int i10 = 0;
            int i11 = 0;
            while (i11 < size) {
                Object obj = m5320WWWWoWWWWo.get(i11);
                i11++;
                String str3 = (String) obj;
                if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-12, -92, -26, 94, ConstantPoolEntry.CP_InterfaceMethodref, 119, -70, -113, -70, -21, -5}, new byte[]{-41, -124, -107, 43, 43, 19, -33, -18}, str3)) {
                    i10 = 1;
                } else {
                    if (i10 != 0) {
                        i10++;
                    }
                    if (i10 <= 0 || i10 > length) {
                        sb2.append(str3);
                        sb2.append("\n");
                    }
                }
            }
        }
        sb2.append(str2);
        WWWW.m5339WWWoWWWo(this.f9276WWWWWWWW, sb2.toString(), false);
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5039WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        boolean z10 = true;
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        String str = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {119, -88, 36, -84, TarConstants.LF_MULTIVOLUME, 57, -122, -118};
        byte[] bArr2 = {TarConstants.LF_PAX_EXTENDED_HEADER_UC, -63, 74, -59, 57, 23, -12, -23};
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        this.f9276WWWWWWWW = file;
        if (!file.exists()) {
            this.f9276WWWWWWWW = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{40, 13, TarConstants.LF_MULTIVOLUME, 111, 26, -73, -17, 1, 98, 10, 87, TarConstants.LF_CHR, 7, -68, -21, 90, 40, 22, 67, TarConstants.LF_CHR, 7, -68, -21, 90, 41, ConstantPoolEntry.CP_NameAndType, 87}, new byte[]{7, 126, TarConstants.LF_BLK, 28, 110, -46, -126, 46}));
        }
        if (!vMConfig.f8902WWWWWW) {
            if (m5225WWWWWWWW(vMConfig)) {
                vMInstance.m5053WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CHR, -99, 35, -46, -29, 107, -103, 14, 56, -101, 37, -104, -3, 112, -104, 28, 126, -127, 59, -116, -19, 118, -103, 14, TarConstants.LF_DIR, Byte.MIN_VALUE}, new byte[]{80, -14, 78, -4, -120, 4, -20, 125}));
            }
            FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{100, 59, -35, 99, -26, TarConstants.LF_SYMLINK, -111, -27, TarConstants.LF_CHR, 42, -51, 126, -67, TarConstants.LF_CHR, -99, -81, 38, 39, -54, 99, -25}, new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, 72, -92, 16, -110, 87, -4, -54})));
            FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{23, -52, 23, -74, -53, 112, -13, 43, 90, -42, 0, -22, -52, 96}, new byte[]{56, -65, 110, -59, -65, 21, -98, 4})));
            FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -30, -95, -98, 85, -31, 117, -20, -57, -13, -79, -125, 14, -9, 109}, new byte[]{-65, -111, -40, -19, 33, -124, 24, -61})));
            ClearAppHelper.m5246WWWoWWWo(vMConfig, WWWWWWWW.m17835WWWWWWWW(new byte[]{-124, 32, -125, 24, 125, 31, 119, 8, -91}, new byte[]{-41, 85, -13, 125, 15, 106, 4, 109}), WWWWWWWW.m17835WWWWWWWW(new byte[]{105, 9, 110, -6, 90, -12, -67, -71, 98, 15, 104, -80, 68, -17, -68, -85, 36, 21, 118, -92, 84, -23, -67, -71, 111, 20}, new byte[]{10, 102, 3, -44, TarConstants.LF_LINK, -101, -56, -54}));
            FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{3, -67, 73, -69, 90, 26, 22, -32, 71, -68, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -30, 22, 6, 20, -83, 71, -74, 89, -66, 29, 0, 18, -25, 89, -83, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -84, 91, 26, ConstantPoolEntry.CP_NameAndType, -13, 73, -85, 89, -66, 16, 27, 87, -25, TarConstants.LF_MULTIVOLUME, -68, 65, -94, 27}, new byte[]{44, -39, 44, -51, 117, 105, 121, -125})));
            StringBuilder sb2 = new StringBuilder();
            ArrayList m5320WWWWoWWWWo = WWWW.m5320WWWWoWWWWo(this.f9276WWWWWWWW);
            if (m5320WWWWoWWWWo != null) {
                int length = f9274WWWoWWWo.split("\n").length;
                int size = m5320WWWWoWWWWo.size();
                int i10 = 0;
                int i11 = 0;
                while (i11 < size) {
                    Object obj = m5320WWWWoWWWWo.get(i11);
                    i11++;
                    String str2 = (String) obj;
                    byte[] bArr3 = {ConstantPoolEntry.CP_InterfaceMethodref, -124, -42, -45, -22, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 123, 58};
                    if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{40, -92, -91, -90, -54, 3, 30, 91, 102, -21, -72}, bArr3, str2)) {
                        i10 = 1;
                    } else {
                        if (i10 != 0) {
                            i10++;
                        }
                        if (i10 <= 0 || i10 > length) {
                            sb2.append(str2);
                            sb2.append("\n");
                        }
                    }
                }
            }
            WWWW.m5339WWWoWWWo(this.f9276WWWWWWWW, sb2.toString(), false);
            return true;
        }
        vMInstance.m5063WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{108, -70, 72, -8, -86, -125, 109, -6, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -68, 78, -78, -76, -104, 108, -24, 33, -90, 80, -90, -92, -98, 109, -6, 106, -89}, new byte[]{15, -43, 37, -42, -63, -20, 24, -119}));
        if (m5225WWWWWWWW(vMConfig) && WWWWWWWW.m17835WWWWWWWW(new byte[]{-28, -42, 90, 78}, new byte[]{-118, -71, TarConstants.LF_BLK, 43, -46, 70, -121, 17}).equals(vMConfig.f8923WoWo)) {
            return true;
        }
        vMInstance.f8939WWWoWWWo.m13942WWWWWWWW(new VMStatusEvent(WWWWWWWW.m17835WWWWWWWW(new byte[]{-40, 22, -69, -86, 110, TarConstants.LF_NORMAL, 92, -91, -62, 13, -72, -69, 125, 41, 67, -97, -61}, new byte[]{-79, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -56, -34, 15, 92, TarConstants.LF_NORMAL, -6})));
        String[] strArr = new String[1];
        try {
            ArrayList arrayList = new ArrayList();
            arrayList.add(Uri.parse(vMConfig.f8895WWWoWWWo.f8857WWWW));
            String str3 = vMConfig.f8868WWWWWWWW;
            Uri uri = (Uri) arrayList.get(0);
            new ImageInstallerV1(vMApp).m5205WWWoWWWo(vMConfig, arrayList, str3, null);
            m5226WWWWWWWW(vMConfig);
        } catch (Throwable th2) {
            strArr[0] = Log.getStackTraceString(th2);
            z10 = false;
        }
        if (!z10) {
            vMInstance.m5096WWWW(false);
            this.f9275WWWWoWWWWo = strArr[0];
        }
        return z10;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        byte[] bArr = {27, 17, 58, TarConstants.LF_FIFO, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 38, 23, -42, 58, TarConstants.LF_NORMAL, 43, 32, 97};
        byte[] bArr2 = {72, 100, 74, TarConstants.LF_GNUTYPE_SPARSE, 10, TarConstants.LF_GNUTYPE_SPARSE, 100, -77};
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
    }
}
