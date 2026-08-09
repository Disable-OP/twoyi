package com.android.vmcore.setup;

import com.android.vmapp.VMApp;
import com.android.vmcore.IVMSetupTask;
import com.android.vmcore.NativeHelper;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.blankj.utilcode.util.WWWWoWWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.archivers.zip.UnixStat;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class PrepareFsTask implements IVMSetupTask {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public int f9257WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9258WWWWWWWW;

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5034WWWWoWWWWo() {
        return true;
    }

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5035WWWWWWWW() {
        return this.f9258WWWWWWWW;
    }

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5036WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        int chmodRecursively;
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        if (WWWWoWWWWo.m5295WoWo(vMConfig.f8868WWWWWWWW) && (chmodRecursively = NativeHelper.chmodRecursively(vMConfig.f8868WWWWWWWW, UnixStat.DEFAULT_LINK_PERM)) != 0) {
            StringBuilder sb2 = new StringBuilder();
            byte[] bArr = {-121, -57, 95, -91, -31, -87, 37, -77, -60, -55, 65, -22, -14, -31, TarConstants.LF_FIFO, -80, -60, -33, 64, -81, -11, -24, 33, -69, -60, -39, 95, -22, -29, -6, 115, -72, -123, -58, 94, -81, -31, -87};
            byte[] bArr2 = {-28, -81, TarConstants.LF_SYMLINK, -54, -123, -119, TarConstants.LF_GNUTYPE_SPARSE, -34};
            StringFog.f8859WWWWWWWW.getClass();
            sb2.append(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
            sb2.append(chmodRecursively);
            this.f9258WWWWWWWW = sb2.toString();
            this.f9257WWWWoWWWWo = 104000;
            return false;
        }
        StringBuilder sb3 = new StringBuilder();
        sb3.append(vMConfig.f8867WWWWWWWW);
        StringFog.f8859WWWWWWWW.getClass();
        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, ConstantPoolEntry.CP_InterfaceMethodref, 97, 20}, new byte[]{87, 111, 4, 98, 4, 61, -105, 6}));
        boolean m5284WWWWWWWW = WWWWoWWWWo.m5284WWWWWWWW(WWWWoWWWWo.m5289WWWWWWWW(sb3.toString()));
        if (!m5284WWWWWWWW) {
            this.f9258WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{119, 6, 38, -70, -10, -42, 93, -53, 113, 2, 99, -65, -21, -63, 93, -55, 117, 29, 47, -66, -26}, new byte[]{20, 116, 67, -37, -126, -77, 125, -81});
            this.f9257WWWWoWWWWo = 104000;
        }
        return m5284WWWWWWWW;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final int getErrorCode() {
        return this.f9257WWWWoWWWWo;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final String getName() {
        byte[] bArr = {34, 6, 13, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 84, 63, -40, 99, 1, 32, 9, 91, 94};
        byte[] bArr2 = {114, 116, 104, 40, TarConstants.LF_DIR, TarConstants.LF_MULTIVOLUME, -67, 37};
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
    }
}
