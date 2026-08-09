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
public class Bug1FixTask implements IVMStartupTask {
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
        byte[] bArr = {-118, -32, 101, 63, TarConstants.LF_CHR, 91, 125, -16, -42, -25, 43, 56, 58, 21, 98, -4, -63, -37, 118, 46, 62, 6, Byte.MAX_VALUE, -74};
        byte[] bArr2 = {-91, -124, 4, TarConstants.LF_GNUTYPE_LONGLINK, 82, 116, 16, -103};
        StringFog.f8859WWWWWWWW.getClass();
        return FileDeleteUtils.m5261WWWWoWWWWo(new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)));
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        byte[] bArr = {-48, 4, TarConstants.LF_GNUTYPE_SPARSE, 72, -12, -120, 23, 60, -13, 2, 95};
        byte[] bArr2 = {-110, 113, TarConstants.LF_BLK, 121, -78, -31, 111, 104};
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
    }
}
