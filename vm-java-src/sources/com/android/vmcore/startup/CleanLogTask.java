package com.android.vmcore.startup;

import com.android.vmapp.VMApp;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import com.android.vmcore.utils.FileDeleteUtils;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.io.File;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class CleanLogTask implements IVMStartupTask {
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
        return FileDeleteUtils.m5262WWWWWWWW(new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{67, 94, 57, -119, -108, TarConstants.LF_BLK, -46, 20, 31, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 115, -122, -102, 33}, new byte[]{108, 45, 93, -22, -11, 70, -74, 59})));
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-79, -61, -30, -102, -91, -11, 64, 7, -90, -50, -12, -112}, new byte[]{-14, -81, -121, -5, -53, -71, 47, 96});
    }
}
