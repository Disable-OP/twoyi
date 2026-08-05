package com.android.vmcore.startup;

import com.android.vmapp.VMApp;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.utils.FileDeleteUtils;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.io.File;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class Bug8FixTask implements IVMStartupTask {
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
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        if (vMConfig.f8895WWWoWWWo.f8847WWWWWWWW == 11) {
            StringFog.f8859WWWWWWWW.getClass();
            if (!WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, -37, 28, 109}, new byte[]{-37, -76, 114, 8, -118, 33, -88, -92}).equals(vMConfig.f8923WoWo)) {
                FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-96, -14, -124, 3, 71, 89, -78, -123, -19, -19, -113, 66, TarConstants.LF_GNUTYPE_LONGLINK, 65, -17, -53, -31, -32, -109, 2, 74, 82, -18, -62, -18, -10, -123, 26, 66, 68, -91, -124, -7, -19, -125, 31, 66, 66, -81, -40, -94, -9, -124, 31, 85, 95, -93, -49, -95, -31, -103, ConstantPoolEntry.CP_NameAndType, 78, 70, -84, -49}, new byte[]{-113, -124, -31, 109, 35, TarConstants.LF_FIFO, -64, -86})));
                FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-44, -31, 62, 108, -44, 100, 41, TarConstants.LF_DIR, -98, -29, 56, 45, -39, 101, TarConstants.LF_SYMLINK, 110, -44, -31, TarConstants.LF_SYMLINK, 96, -62, 106, 47, 117, -119, -70, 63, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -42, 106, 46, 118, -113, -71, 41, 97}, new byte[]{-5, -105, 91, 2, -80, ConstantPoolEntry.CP_InterfaceMethodref, 91, 26})));
                FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-60, -71, 71, -71, 90, 117, 19, -4, -114, -69, 65, -8, 72, 115, 15, -89, -115, -32, 79, -74, 80, 115, 7, -74, -104, -69, 13, -95, 87, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 19, -78, -97, -96, 80, -6, 90, Byte.MAX_VALUE, 7, -78, -98, -93, 86, -7, 70, 119, 13}, new byte[]{-21, -49, 34, -41, 62, 26, 97, -45})));
                return true;
            }
        }
        return true;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{98, -96, 109, -63, -72, -121, -42, 57, 65, -90, 97}, new byte[]{32, -43, 10, -7, -2, -18, -82, 109});
    }
}
