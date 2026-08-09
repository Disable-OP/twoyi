package com.android.vmcore.startup;

import com.android.vmapp.VMApp;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import com.blankj.utilcode.util.WWWW;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.io.File;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class Bug3FixTask implements IVMStartupTask {
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
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{8, -110, 23, -44, -86, -87, -38, -15, 72, -122, 19, -46, -65, -1, -123, -13, 66, -124, 5, -55, -72, -14, -124, -16, 94, -123, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -46, -92, -23, -34, -36, 70, -107, 21, -59, -72, -11}, new byte[]{39, -10, 118, -96, -53, -122, -86, -125}));
        if (!file.exists()) {
            WWWW.m5339WWWoWWWo(file, WWWWWWWW.m17835WWWWWWWW(new byte[]{112}, new byte[]{67, -57, -1, -63, 72, 30, -115, 1}), false);
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
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{94, -40, 100, 78, -20, TarConstants.LF_DIR, 2, 109, 125, -34, 104}, new byte[]{28, -83, 3, 125, -86, 92, 122, 57});
    }
}
