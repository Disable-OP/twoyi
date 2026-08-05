package com.android.vmcore.startup;

import android.net.Uri;
import android.os.Build;
import android.util.Log;
import com.android.vmapp.VMApp;
import com.android.vmcore.C1623WWWWWWWW;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.ImageInstaller;
import com.android.vmcore.KLog;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.VMStatusEvent;
import com.android.vmcore.installer.ImageInstallerV1;
import java.io.File;
import java.io.FileFilter;
import java.util.ArrayList;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class Bug6FixTask implements IVMStartupTask {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public int f9261WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9262WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public FileFilter f9263WWWoWWWo = new C1623WWWWWWWW(1);

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5037WWWWoWWWWo() {
        return true;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5038WWWWWWWW() {
        return this.f9262WWWWWWWW;
    }

    /* JADX WARN: Removed duplicated region for block: B:27:0x0126 A[RETURN] */
    /* JADX WARN: Removed duplicated region for block: B:28:0x0127  */
    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWȏWWWoನ̑ */
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final boolean mo5039WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        File file;
        boolean exists;
        boolean z10;
        boolean z11 = true;
        String str = vMInstance.f8937WWWoWWWo.f8868WWWWWWWW;
        StringFog.f8859WWWWWWWW.getClass();
        boolean exists2 = new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{114, TarConstants.LF_GNUTYPE_LONGLINK, 63, 81}, new byte[]{27, 37, 86, 37, -38, 56, 59, 125})).exists();
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        if (!exists2) {
            this.f9263WWWoWWWo = null;
        } else {
            if (vMConfig.f8895WWWoWWWo.f8847WWWWWWWW == 11) {
                file = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{9, -15, 20, -105, -15, -54, -75, 119, 19, -26, 72, -106, -15, -47, -1, 123, 14, -20}, new byte[]{122, -120, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -29, -108, -89, -102, 21}));
            } else {
                file = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{90, -13, -122, 98, 79, -67, 47, TarConstants.LF_CONTIG, TarConstants.LF_GNUTYPE_LONGNAME, -1, -101, 104}, new byte[]{41, -111, -17, ConstantPoolEntry.CP_NameAndType, 96, -56, 74, 65}));
            }
            if (!file.exists()) {
                this.f9263WWWoWWWo = new C1623WWWWWWWW(2);
            } else {
                if (Build.VERSION.SDK_INT >= 34) {
                    File file2 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, 22, 65, -75, 6, 92, -94, -20, TarConstants.LF_MULTIVOLUME, 14, 95, -92, 20, 94, -1, -31, 16, 0, TarConstants.LF_GNUTYPE_SPARSE, -75, TarConstants.LF_GNUTYPE_LONGNAME, 80, -1, -25, 9, 91, 29}, new byte[]{63, 111, TarConstants.LF_SYMLINK, -63, 99, TarConstants.LF_LINK, -115, -118}));
                    File file3 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{106, 117, 41, -21, -75, 70, 82, -18, 107, 109, TarConstants.LF_CONTIG, -6, -89, 68, 15, -29, TarConstants.LF_FIFO, 109, 40, -14, -26, 31, 82}, new byte[]{25, ConstantPoolEntry.CP_NameAndType, 90, -97, -48, 43, 125, -120}));
                    File file4 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-61, 92, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 93, 118, -28, -32, -62, -62, 68, 102, TarConstants.LF_GNUTYPE_LONGNAME, 100, -26, -67, -49, -97, 74, 106, 93, 60, -24, -67, -55, -97}, new byte[]{-80, 37, ConstantPoolEntry.CP_InterfaceMethodref, 41, 19, -119, -49, -92}));
                    if (file2.exists()) {
                        exists = new File(file2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-22, 39, TarConstants.LF_NORMAL, 118, -120, -44, 89, -5, -73, 45, 38, 101, -103}, new byte[]{-103, 66, 66, 0, -31, -73, 60, -120})).exists();
                    } else if (file3.exists()) {
                        exists = new File(file3, WWWWWWWW.m17835WWWWWWWW(new byte[]{65, -87, 68, -110, TarConstants.LF_NORMAL, -85, 0, Byte.MAX_VALUE, 28, -93, 82, -127, 33}, new byte[]{TarConstants.LF_SYMLINK, -52, TarConstants.LF_FIFO, -28, 89, -56, 101, ConstantPoolEntry.CP_NameAndType})).exists();
                    } else if (file4.exists()) {
                        exists = new File(file4, WWWWWWWW.m17835WWWWWWWW(new byte[]{-69, ConstantPoolEntry.CP_InterfaceMethodref, -32, -65, -127, -46, 20, TarConstants.LF_SYMLINK, -26, 1, -10, -84, -112}, new byte[]{-56, 110, -110, -55, -24, -79, 113, 65})).exists();
                    }
                    z10 = !exists;
                    if (!z10) {
                        return true;
                    }
                    KLog.m5043WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{-19, -111, -93, Byte.MAX_VALUE, TarConstants.LF_SYMLINK, 69, 31, 14, -50, -105, -81}, new byte[]{-81, -28, -60, 73, 116, 44, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 90}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-91, -35, -49, TarConstants.LF_LINK, 71, -78, 80, 121, -10, -49, -57, 59}, new byte[]{-42, -87, -82, 67, TarConstants.LF_CHR, -110, TarConstants.LF_FIFO, 10}));
                    vMInstance.f8939WWWoWWWo.m13942WWWWWWWW(new VMStatusEvent(WWWWWWWW.m17835WWWWWWWW(new byte[]{-95, -5, -84, TarConstants.LF_LINK, 6, -85}, new byte[]{-57, -110, -44, 110, 96, -40, -40, -11})));
                    String[] strArr = new String[1];
                    try {
                        ArrayList arrayList = new ArrayList();
                        for (String str2 : vMConfig.f8895WWWoWWWo.f8854WWWoWWWo) {
                            arrayList.add(Uri.parse(str2));
                        }
                        String str3 = vMConfig.f8867WWWWWWWW;
                        ImageInstaller.InstallOptions installOptions = new ImageInstaller.InstallOptions();
                        installOptions.f8843WWWWWWWW = this.f9263WWWoWWWo;
                        byte[] bArr = {-44, 122, -124, 71, -102, TarConstants.LF_GNUTYPE_SPARSE, -105, -103};
                        StringFog.f8859WWWWWWWW.getClass();
                        installOptions.f8842WWWWoWWWWo = WWWWWWWW.m17835WWWWWWWW(new byte[]{-78, 9}, bArr);
                        Uri uri = (Uri) arrayList.get(0);
                        new ImageInstallerV1(vMApp).m5205WWWoWWWo(vMConfig, arrayList, str3, installOptions);
                    } catch (Throwable th2) {
                        strArr[0] = Log.getStackTraceString(th2);
                        z11 = false;
                    }
                    if (!z11) {
                        this.f9262WWWWWWWW = strArr[0];
                        this.f9261WWWWoWWWWo = 114500;
                    }
                    StringFog.f8859WWWWWWWW.getClass();
                    KLog.m5043WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{-60, 58, 121, -40, 43, -97, -77, -38, -25, 60, 117}, new byte[]{-122, 79, 30, -18, 109, -10, -53, -114}), WWWWWWWW.m17835WWWWWWWW(new byte[]{93, 41, -68, 74, -37, -110, 72, -45, 81, 63, -8}, new byte[]{56, 71, -40, 106, -67, -31, 104, -75}) + z11);
                    return z11;
                }
                z10 = false;
                if (!z10) {
                }
            }
        }
        z10 = true;
        if (!z10) {
        }
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return this.f9261WWWWoWWWWo;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-127, -13, -77, 47, -38, 46, -26, TarConstants.LF_LINK, -94, -11, -65}, new byte[]{-61, -122, -44, 25, -100, 71, -98, 101});
    }
}
