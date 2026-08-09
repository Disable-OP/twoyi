package com.android.vmcore.startup;

import android.content.pm.ApplicationInfo;
import android.system.Os;
import android.util.Log;
import com.android.vmapp.VMApp;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import com.android.vmcore.utils.FileDeleteUtils;
import java.io.File;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class BuildExecPathTask implements IVMStartupTask {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public int f9265WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9266WWWWWWWW;

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5037WWWWoWWWWo() {
        return true;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5038WWWWWWWW() {
        return this.f9266WWWWWWWW;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5039WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        try {
            ApplicationInfo applicationInfo = vMApp.getApplicationInfo();
            String str = applicationInfo.dataDir;
            StringFog.f8859WWWWWWWW.getClass();
            File file = new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{8, 102, -105, 25, 71}, new byte[]{100, 15, -11, 47, 115, 39, -66, 124}));
            if (!file.exists()) {
                file.delete();
                Os.symlink(applicationInfo.nativeLibraryDir, file.getAbsolutePath());
            } else if (!Os.readlink(file.getAbsolutePath()).equals(applicationInfo.nativeLibraryDir)) {
                file.delete();
                Os.symlink(applicationInfo.nativeLibraryDir, file.getAbsolutePath());
            }
            FileDeleteUtils.m5262WWWWWWWW(new File(applicationInfo.dataDir, WWWWWWWW.m17835WWWWWWWW(new byte[]{-70, -64, -3, 16}, new byte[]{-42, -87, -97, 34, -101, -16, 38, 74})));
            return true;
        } catch (Throwable th2) {
            this.f9266WWWWWWWW = Log.getStackTraceString(th2);
            this.f9265WWWWoWWWWo = 115000;
            return false;
        }
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return this.f9265WWWWoWWWWo;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -55, -109, -7, -24, -25, 126, 78, 106, -20, -101, -31, -28, -10, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 98}, new byte[]{9, -68, -6, -107, -116, -94, 6, 43});
    }
}
