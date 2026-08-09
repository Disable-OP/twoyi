package com.android.vmcore.startup;

import android.content.SharedPreferences;
import android.net.Uri;
import android.text.TextUtils;
import android.util.Log;
import com.android.vmapp.VMApp;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.ImageInstaller;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.VMStatusEvent;
import com.android.vmcore.installer.ImageInstallerV1;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.Set;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p000WWWWWWWWWW.WWoWWo;
import p041WWWoWWWo.C0434WWWWWWWW;
import vf.AbstractC4470WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class ApplyOverlaysTask implements IVMStartupTask {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9259WWWWWWWW;

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5037WWWWoWWWWo() {
        return false;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5038WWWWWWWW() {
        return this.f9259WWWWWWWW;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5039WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        int i10 = 8;
        vMInstance.getClass();
        StringFog.f8859WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-111, -20, -56, 119, -125, TarConstants.LF_NORMAL, -62, -108, -108, -16, -55, 111, -121, 36, -18}, new byte[]{-30, -107, -69, 3, -26, 93, -99, -5});
        C0434WWWWWWWW c0434wwwwwwww = new C0434WWWWWWWW();
        SharedPreferences sharedPreferences = vMInstance.f8926WWWWoWWWWo;
        Set<String> stringSet = sharedPreferences.getStringSet(m17835WWWWWWWW, c0434wwwwwwww);
        boolean z10 = true;
        if (stringSet.isEmpty()) {
            return true;
        }
        String[] strArr = new String[1];
        HashSet hashSet = new HashSet();
        for (String str : stringSet) {
            if (!TextUtils.isEmpty(strArr[0])) {
                strArr[0] = WWoWWo.m57WWoWWo(new StringBuilder(), strArr[0], "\n");
            }
            Uri parse = Uri.parse(str);
            String lastPathSegment = parse.getLastPathSegment();
            StringBuilder sb2 = new StringBuilder();
            byte[] bArr = new byte[i10];
            // fill-array-data instruction
            bArr[0] = 23;
            bArr[1] = -44;
            bArr[2] = 93;
            bArr[3] = 54;
            bArr[4] = -26;
            bArr[5] = -54;
            bArr[6] = 29;
            bArr[7] = -19;
            vMInstance.f8939WWWoWWWo.m13942WWWWWWWW(new VMStatusEvent(AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{118, -92, 45, 90, -97, -107, 114, -101, 114, -90, TarConstants.LF_LINK, 87, -97, -107}, bArr, sb2, lastPathSegment)));
            VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
            try {
                ArrayList arrayList = new ArrayList();
                arrayList.add(parse);
                String str2 = vMConfig.f8868WWWWWWWW;
                ImageInstaller.InstallOptions installOptions = new ImageInstaller.InstallOptions();
                installOptions.f8842WWWWoWWWWo = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
                installOptions.f8844WWWoWWWo = false;
                Uri uri = (Uri) arrayList.get(0);
                try {
                    new ImageInstallerV1(vMApp).m5205WWWoWWWo(vMConfig, arrayList, str2, installOptions);
                } catch (Throwable th2) {
                    th = th2;
                    strArr[0] = Log.getStackTraceString(th);
                    hashSet.add(str);
                    z10 = false;
                    i10 = 8;
                }
            } catch (Throwable th3) {
                th = th3;
            }
            i10 = 8;
        }
        SharedPreferences.Editor edit = sharedPreferences.edit();
        byte[] bArr2 = {-111, -89, -101, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -18, 92, 93, -20, -108, -69, -102, 64, -22, 72, 113};
        byte[] bArr3 = {-30, -34, -24, 44, -117, TarConstants.LF_LINK, 2, -125};
        StringFog.f8859WWWWWWWW.getClass();
        edit.putStringSet(WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3), hashSet).apply();
        if (!z10) {
            this.f9259WWWWWWWW = strArr[0];
        }
        return z10;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        byte[] bArr = {-92, Byte.MAX_VALUE, -24, -106, -55, -112, 34, 110, -105, 99, -7, -125, -61, -117, TarConstants.LF_DIR, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -114};
        byte[] bArr2 = {-27, 15, -104, -6, -80, -33, 84, ConstantPoolEntry.CP_InterfaceMethodref};
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
    }
}
