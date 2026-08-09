package com.android.vmcore.setup;

import com.android.vmapp.VMApp;
import com.android.vmcore.IVMSetupTask;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import com.android.vmcore.utils.FileDeleteUtils;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.io.File;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class CleanFsTask implements IVMSetupTask {
    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5034WWWWoWWWWo() {
        return false;
    }

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5035WWWWWWWW() {
        return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
    }

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5036WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        StringFog.f8859WWWWWWWW.getClass();
        String[] strArr = {WWWWWWWW.m17835WWWWWWWW(new byte[]{-97, -46, 0, 17, -7, 27, 71}, new byte[]{-10, -68, 105, 101, -90, 42, 119, 110}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-55, 4, 81, -18, 101, 34, -22, -45, -34, 57, 9, -80}, new byte[]{-70, 102, 56, Byte.MIN_VALUE, 74, 67, -114, -79}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-88, 65, 69, -43, -126, -18, -31, -37, -78, 86, 25, -46, -113, -36, -1, -119}, new byte[]{-37, 56, TarConstants.LF_FIFO, -95, -25, -125, -50, -71}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-115, -21, 42, 110, 111, 115, 94, 26, -105, -4, 118, 113, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 40, 69, 39, -49, -94}, new byte[]{-2, -110, 89, 26, 10, 30, 113, TarConstants.LF_PAX_EXTENDED_HEADER_LC}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-9, -14, -82, 38, 79, 71, -22, -85, -19, -27, -14, 57, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 28, -15, -106, -68, -70}, new byte[]{-124, -117, -35, 82, 42, 42, -59, -55}), WWWWWWWW.m17835WWWWWWWW(new byte[]{0, 85, TarConstants.LF_DIR, 39, 23, -77, 61, -14, 26, 66, 105, 56, 0, -19, 32, -49, 66, 28}, new byte[]{115, 44, 70, TarConstants.LF_GNUTYPE_SPARSE, 114, -34, 18, -112}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-4, 45, 126, Byte.MIN_VALUE, -28, -41, -26, 35, -26, 58, 34, -97, -13, -119, -5, 30, -73, 101}, new byte[]{-113, 84, 13, -12, -127, -70, -55, 65}), WWWWWWWW.m17835WWWWWWWW(new byte[]{27, -67, -110, -74, -9, 121, -67, -8, 1, -90, -50, -82, -5, 118, -25, -3, 89, -12, -49, -79, -3}, new byte[]{104, -60, -31, -62, -110, 20, -110, -108}), WWWWWWWW.m17835WWWWWWWW(new byte[]{30, -30, -119, 84, -78, 10, 33, -100, 4, -7, -52, 20, -8, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -110, 24, -14, -53, 16, -7, 20, 97}, new byte[]{109, -101, -6, 32, -41, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 14, -16}), WWWWWWWW.m17835WWWWWWWW(new byte[]{14, -82, 111, -112, 102, 86, 0, -102, 20, -75, TarConstants.LF_CHR, -120, 106, 89, 90, -97, 72, -26, TarConstants.LF_SYMLINK, -105, 108}, new byte[]{125, -41, 28, -28, 3, 59, 47, -10}), WWWWWWWW.m17835WWWWWWWW(new byte[]{8, 121, -17, -13, -24, 87, 8, -36, 18, 98, -86, -77, -94, 86, 78, -46, 14, 105, -87, -74, -93, 73, 72}, new byte[]{123, 0, -100, -121, -115, 58, 39, -80}), WWWWWWWW.m17835WWWWWWWW(new byte[]{32, -41, -101, 43, -69, -115, -12, -95, 58, -52, -57, TarConstants.LF_CHR, -73, -126, -77, -94, 32, -38, -124, TarConstants.LF_FIFO, -68, -107, -78, -110, 98, -98, -58, 44, -79}, new byte[]{TarConstants.LF_GNUTYPE_SPARSE, -82, -24, 95, -34, -32, -37, -51}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-7, -80, 124, -9, 122, -23, 8, 65, -29, -85, 57, -73, TarConstants.LF_NORMAL, -24, 78, 79, -30, -90, 124, -9, 115, -19, 69, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -29, -106, 62, -77, TarConstants.LF_LINK, -9, 72}, new byte[]{-118, -55, 15, -125, 31, -124, 39, 45})};
        for (int i10 = 0; i10 < 13; i10++) {
            FileDeleteUtils.m5262WWWWWWWW(new File(vMInstance.f8937WWWoWWWo.f8868WWWWWWWW, strArr[i10]));
        }
        return true;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-104, -84, -36, 16, -29, 78, 6, -124, -70, -77, -46}, new byte[]{-37, -64, -71, 113, -115, 8, 117, -48});
    }
}
