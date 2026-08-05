package com.android.vmcore.startup;

import com.android.vmapp.VMApp;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.utils.FileDeleteUtils;
import com.blankj.utilcode.util.WWWWoWWWWo;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.io.File;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class BuildTmpfsTask implements IVMStartupTask {
    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5037WWWWoWWWWo() {
        return false;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5038WWWWWWWW() {
        return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5039WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        String str = vMInstance.f8937WWWoWWWo.f8868WWWWWWWW;
        StringFog.f8859WWWWWWWW.getClass();
        WWWWoWWWWo.m5284WWWWWWWW(new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{56, -102, 44, -75, TarConstants.LF_NORMAL, -76, 46, 45, 124, -101, 61}, new byte[]{23, -2, 73, -61, 31, -57, 65, 78})));
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        FileDeleteUtils.m5261WWWWoWWWWo(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-93, 95, 84, -59, 25, 18, -101, -42, -1}, new byte[]{-116, 47, 38, -86, 122, 61, -24, -81})));
        WWWWoWWWWo.m5285WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, 20, -2, 41, -82, -28, -91, 35, -52, TarConstants.LF_GNUTYPE_LONGLINK, -25, 35, -65, -91, -77, TarConstants.LF_FIFO, -112, 15, -4, TarConstants.LF_SYMLINK, -65, -108, -92, 63, -52, 16, -2, 47, -82, -65}, new byte[]{-65, 100, -116, 70, -51, -53, -42, 90})));
        WWWWoWWWWo.m5285WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{41, -4, 78, -65, 5, 13, 114, -56, 117, -93, 74, -67, 73, 79, 108, -48, 118, -45, 78, -66, 2, 125, 99, -40, 114, -1}, new byte[]{6, -116, 60, -48, 102, 34, 1, -79})));
        WWWWoWWWWo.m5285WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_UC, -23, TarConstants.LF_NORMAL, 126, 124, -38, -24, -72, 4, -74, TarConstants.LF_BLK, 124, TarConstants.LF_NORMAL, -104, -10, -96, 7, -58, TarConstants.LF_NORMAL, Byte.MAX_VALUE, 123, -86, -8, -82, 26, -23, 35, 101, 64, -105, -14, -75, 4}, new byte[]{119, -103, 66, 17, 31, -11, -101, -63})));
        FileDeleteUtils.m5261WWWWoWWWWo(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{73, 95, -51, -26, 37, 26, -32, TarConstants.LF_DIR, 13, 94, -36}, new byte[]{102, 59, -88, -112, 10, 105, -113, 86})));
        WWWWoWWWWo.m5284WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{126, -78, 34, 2, -49, 101, 97, 96, 35, -71, 36}, new byte[]{81, -42, 71, 116, -32, 19, ConstantPoolEntry.CP_NameAndType, 16})));
        FileDeleteUtils.m5261WWWWoWWWWo(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-100, -23, -23, 112, -4, TarConstants.LF_GNUTYPE_LONGNAME, 36, 115, -63, -30, -17}, new byte[]{-77, -115, -116, 6, -45, 58, 73, 3})));
        FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{16, 29, -48, -58, 32, 8, -40, 124, TarConstants.LF_MULTIVOLUME, 22, -59, -43, 125, 35, -18, 105, TarConstants.LF_GNUTYPE_LONGNAME, 38, -22}, new byte[]{63, 121, -75, -80, 15, 87, -121, ConstantPoolEntry.CP_NameAndType})));
        FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-49, 38, -36, -12, 0, -20, -25, 125, -115, TarConstants.LF_LINK, -34, -35, 112}, new byte[]{-32, 66, -71, -126, 47, -77, -72, 22})));
        return true;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{38, -93, Byte.MAX_VALUE, TarConstants.LF_SYMLINK, -82, TarConstants.LF_GNUTYPE_LONGLINK, 56, 121, 2, -91, 66, 63, -71, 116}, new byte[]{100, -42, 22, 94, -54, 31, 85, 9});
    }
}
