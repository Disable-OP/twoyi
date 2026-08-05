package com.android.vmcore.setup;

import com.android.vmapp.VMApp;
import com.android.vmcore.IVMSetupTask;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import com.android.vmcore.utils.FileDeleteUtils;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.io.File;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class CleanCacheTask implements IVMSetupTask {
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
        String str = vMInstance.f8937WWWoWWWo.f8868WWWWWWWW;
        StringFog.f8859WWWWWWWW.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-41, 73, 39, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 22, -61, 4, -43, -108, 91, 47, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 90, -113, 1, -41, -112, 72}, new byte[]{-8, 45, 70, 19, 119, -20, 96, -76})));
        return true;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-93, -121, -92, -104, 23, 107, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -65, -120, -114, -107, -104, 10, 67}, new byte[]{-32, -21, -63, -7, 121, 40, 57, -36});
    }
}
