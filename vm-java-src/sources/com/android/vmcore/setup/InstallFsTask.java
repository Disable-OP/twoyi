package com.android.vmcore.setup;

import android.net.Uri;
import android.util.Log;
import com.android.vmapp.VMApp;
import com.android.vmcore.IVMSetupTask;
import com.android.vmcore.ImageInstaller;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.installer.ImageInstallerV1;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class InstallFsTask implements IVMSetupTask {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public int f9255WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9256WWWWWWWW;

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5034WWWWoWWWWo() {
        return true;
    }

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5035WWWWWWWW() {
        return this.f9256WWWWWWWW;
    }

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5036WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        boolean z10 = true;
        String[] strArr = new String[1];
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        try {
            ArrayList arrayList = new ArrayList();
            for (String str : vMConfig.f8895WWWoWWWo.f8854WWWoWWWo) {
                arrayList.add(Uri.parse(str));
            }
            String str2 = vMConfig.f8867WWWWWWWW;
            ImageInstaller.InstallOptions installOptions = new ImageInstaller.InstallOptions();
            StringFog.f8859WWWWWWWW.getClass();
            installOptions.f8842WWWWoWWWWo = WWWWWWWW.m17835WWWWWWWW(new byte[]{6, TarConstants.LF_DIR}, new byte[]{96, 70, -123, 23, 40, 87, 62, -125});
            Uri uri = (Uri) arrayList.get(0);
            new ImageInstallerV1(vMApp).m5205WWWoWWWo(vMConfig, arrayList, str2, installOptions);
        } catch (Throwable th2) {
            strArr[0] = Log.getStackTraceString(th2);
            z10 = false;
        }
        vMInstance.m5077WWWoWWWo(new HashSet(Arrays.asList(vMConfig.f8895WWWoWWWo.f8851WWWWWWWW)));
        if (!z10) {
            this.f9256WWWWWWWW = strArr[0];
            this.f9255WWWWoWWWWo = 105000;
        }
        return z10;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final int getErrorCode() {
        return this.f9255WWWWoWWWWo;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_MULTIVOLUME, 5, -41, -72, -46, -5, -81, 121, 119, 63, -59, -65, -40}, new byte[]{4, 107, -92, -52, -77, -105, -61, 63});
    }
}
