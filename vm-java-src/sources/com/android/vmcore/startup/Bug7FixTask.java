package com.android.vmcore.startup;

import android.os.Build;
import android.text.TextUtils;
import android.util.Log;
import com.android.vmapp.VMApp;
import com.android.vmcore.C1623WWWWWWWW;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.KLog;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.utils.FileDeleteUtils;
import com.blankj.utilcode.util.WWWWoWWWWo;
import java.io.File;
import java.util.ArrayList;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class Bug7FixTask implements IVMStartupTask {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9264WWWWWWWW;

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5037WWWWoWWWWo() {
        return false;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5038WWWWWWWW() {
        return this.f9264WWWWWWWW;
    }

    /* JADX WARN: Code restructure failed: missing block: B:18:0x0053, code lost:
        if (x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-16, -94, 118, -123, 22, -56}, new byte[]{-126, -57, 6, -28, Byte.MAX_VALUE, -70, -116, -76}).equals(r7.f8923WoWo) == false) goto L31;
     */
    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWȏWWWoನ̑ */
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final boolean mo5039WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        boolean z10 = true;
        String[] strArr = new String[1];
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        if (Build.VERSION.SDK_INT >= 35 && vMConfig.f8895WWWoWWWo.f8847WWWWWWWW == 9) {
            String str = Build.FINGERPRINT;
            if (!TextUtils.isEmpty(str)) {
                if (TextUtils.isEmpty(vMConfig.f8894WWWWWWWW)) {
                    vMInstance.m5076WWWWWWWW();
                    return true;
                }
                if (str.equals(vMConfig.f8894WWWWWWWW)) {
                    StringFog.f8859WWWWWWWW.getClass();
                }
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5043WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_DIR, TarConstants.LF_MULTIVOLUME, TarConstants.LF_CONTIG, -55, -41, 110, -4, 116, 22, TarConstants.LF_GNUTYPE_LONGLINK, 59}, new byte[]{119, 56, 80, -2, -111, 7, -124, 32}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-77, -13, 47, -49, -32, 85, -108, -16, -91, -26, 32, -99, -9, 26, -109, -7, -32, -28, 47, -34, -4, 16}, new byte[]{-64, -121, 78, -67, -108, 117, -9, -100}));
                try {
                    ArrayList m5293WWoWWo = WWWWoWWWWo.m5293WWoWWo(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{69, -34, -9, 59, 86, 37, 26, -56, 15, -56, -55, 43, 82}, new byte[]{106, -70, -106, 79, TarConstants.LF_CONTIG, 10, 111, -69})), new C1623WWWWWWWW(3));
                    int size = m5293WWoWWo.size();
                    int i10 = 0;
                    while (i10 < size) {
                        Object obj = m5293WWoWWo.get(i10);
                        i10++;
                        File file = (File) obj;
                        StringFog.f8859WWWWWWWW.getClass();
                        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-62, 1, -68, 47, 9, 61, 90, -6, -31, 7, -80}, new byte[]{Byte.MIN_VALUE, 116, -37, 24, 79, 84, 34, -82});
                        KLog.m5043WWWWWWWW(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-13, -125, -33, 107, -3, -9, 85}, new byte[]{-105, -26, -77, 14, -119, -110, 117, ConstantPoolEntry.CP_InterfaceMethodref}) + file.getPath());
                        FileDeleteUtils.m5262WWWWWWWW(file);
                    }
                    vMInstance.m5076WWWWWWWW();
                } catch (Throwable th2) {
                    strArr[0] = Log.getStackTraceString(th2);
                    z10 = false;
                }
                byte[] bArr = {-20, 28, -111, 121, 7, ConstantPoolEntry.CP_NameAndType, -120, -32};
                StringFog.f8859WWWWWWWW.getClass();
                String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-82, 105, -10, 78, 65, 101, -16, -76, -115, 111, -6}, bArr);
                KLog.m5043WWWWWWWW(m17835WWWWWWWW2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-37, -99, -39, 7, 28, 30, -125, -47, -48, -45, -34, 72, 27, 23, -58, -45, -33, -112, -43, 66, 95}, new byte[]{-66, -13, -67, 39, Byte.MAX_VALUE, 114, -26, -80}) + z10);
                if (!z10) {
                    this.f9264WWWWWWWW = strArr[0];
                }
                return z10;
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
        byte[] bArr = {104, -117, 116, TarConstants.LF_NORMAL, TarConstants.LF_SYMLINK, -15, TarConstants.LF_MULTIVOLUME, -37, TarConstants.LF_GNUTYPE_LONGLINK, -115, TarConstants.LF_PAX_EXTENDED_HEADER_LC};
        byte[] bArr2 = {42, -2, 19, 7, 116, -104, TarConstants.LF_DIR, -113};
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
    }
}
