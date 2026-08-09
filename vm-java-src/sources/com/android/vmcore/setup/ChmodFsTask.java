package com.android.vmcore.setup;

import com.android.vmapp.VMApp;
import com.android.vmcore.IVMSetupTask;
import com.android.vmcore.NativeHelper;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.archivers.zip.UnixStat;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class ChmodFsTask implements IVMSetupTask {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public int f9249WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9250WWWWWWWW;

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5034WWWWoWWWWo() {
        return true;
    }

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5035WWWWWWWW() {
        return this.f9250WWWWWWWW;
    }

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5036WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        int chmodRecursively = NativeHelper.chmodRecursively(vMInstance.f8937WWWoWWWo.f8868WWWWWWWW, UnixStat.DEFAULT_LINK_PERM);
        if (chmodRecursively != 0) {
            StringBuilder sb2 = new StringBuilder();
            byte[] bArr = {-59, -3, 16, 18, -109, -86, -84, 89, -122, -13, 14, 93, -106, -20, -82, 81, -44, -75, 20, 19, -124, -2, -69, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -54, -75, ConstantPoolEntry.CP_InterfaceMethodref, 16, -41, -20, -87, 20, -64, -12, 20, 17, -110, -18, -6};
            byte[] bArr2 = {-90, -107, 125, 125, -9, -118, -38, TarConstants.LF_BLK};
            StringFog.f8859WWWWWWWW.getClass();
            sb2.append(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
            sb2.append(chmodRecursively);
            this.f9250WWWWWWWW = sb2.toString();
            this.f9249WWWWoWWWWo = 107000;
        }
        if (chmodRecursively == 0) {
            return true;
        }
        return false;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final int getErrorCode() {
        return this.f9249WWWWoWWWWo;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final String getName() {
        byte[] bArr = {TarConstants.LF_LINK, -62, -63, -13, -49, 97, 92, 23};
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{114, -86, -84, -100, -85, 39, 47, 67, 80, -79, -86}, bArr);
    }
}
