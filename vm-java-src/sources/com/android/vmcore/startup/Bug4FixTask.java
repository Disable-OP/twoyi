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
public class Bug4FixTask implements IVMStartupTask {
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
        byte[] bArr = {-65, 63, -115, TarConstants.LF_SYMLINK, -15, 70, -67, -68, -9, 99, -108, 21, -31};
        byte[] bArr2 = {-112, TarConstants.LF_MULTIVOLUME, -32, 109, -107, 35, -33, -55};
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        if (file.exists()) {
            FileDeleteUtils.m5262WWWWWWWW(file);
            return true;
        }
        return true;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        byte[] bArr = {-117, 70, 84, -121, -44, -98, 32, -25, -88, 64, TarConstants.LF_PAX_EXTENDED_HEADER_UC};
        byte[] bArr2 = {-55, TarConstants.LF_CHR, TarConstants.LF_CHR, -77, -110, -9, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -77};
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
    }
}
