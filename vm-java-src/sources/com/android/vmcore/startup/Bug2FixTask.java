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
public class Bug2FixTask implements IVMStartupTask {
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
        byte[] bArr = {-27, 30, 110, -112, 38, ConstantPoolEntry.CP_NameAndType, 93, 26};
        StringFog.f8859WWWWWWWW.getClass();
        if (!WWWWWWWW.m17835WWWWWWWW(new byte[]{-117, 113, 0, -11}, bArr).equals(vMConfig.f8923WoWo)) {
            return FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, 21, -55, 57, 8, -93, 112, TarConstants.LF_CHR, -53, 16, -121, 46, 6, -31, 58, 58, -54, 16, -33, 40, 0, -94, 124, 37, -42, 21, -121, 43, 0, -32, 113, 33, -112, 26, -63, 57, 26, -93}, new byte[]{-65, 113, -88, TarConstants.LF_MULTIVOLUME, 105, -116, 20, 82})));
        }
        return true;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        byte[] bArr = {-116, -80, -77, TarConstants.LF_SYMLINK, -71, -40, 94, TarConstants.LF_MULTIVOLUME};
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-50, -59, -44, 0, -1, -79, 38, 25, -19, -61, -40}, bArr);
    }
}
