package com.android.vmcore.setup;

import android.text.TextUtils;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmapp.VMApp;
import com.android.vmcore.IVMSetupTask;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.blankj.utilcode.util.WWWW;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.io.File;
import java.util.ArrayList;
import java.util.HashSet;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class LoadVMPropTask implements IVMSetupTask {
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
        int indexOf;
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        if (vMConfig.f8870WWWWWWWW.isEmpty()) {
            String str = vMConfig.f8868WWWWWWWW;
            byte[] bArr = {-54, -50, TarConstants.LF_DIR, -110, ConstantPoolEntry.CP_InterfaceMethodref, -5, -36, -15, -121, -56, 37, -115, 27, -80, -63, -84, -118, -51};
            byte[] bArr2 = {-27, -67, TarConstants.LF_GNUTYPE_LONGNAME, -31, Byte.MAX_VALUE, -98, -79, -34};
            StringFog.f8859WWWWWWWW.getClass();
            ArrayList m5320WWWWoWWWWo = WWWW.m5320WWWWoWWWWo(new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)));
            if (m5320WWWWoWWWWo != null) {
                HashSet hashSet = new HashSet();
                hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{-78, 79, TarConstants.LF_FIFO, -34, 9, 37, 35, -67, -93, 84, TarConstants.LF_FIFO, -61, 20, 46, 34, -92}, new byte[]{-64, 32, 24, -82, 123, 74, 71, -56}));
                hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{-3, 101, -106, -66, 7, 1, 79, -84, -20, 126, -106, -84, 7, 15, 69, -67}, new byte[]{-113, 10, -72, -50, 117, 110, 43, -39}));
                hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{9, TarConstants.LF_BLK, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 118, 25, 65, -44, -73, 24, 47, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 104, 10, 67, -43}, new byte[]{123, 91, 86, 6, 107, 46, -80, -62}));
                hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_DIR, -96, -30, TarConstants.LF_LINK, -91, 2, -57, 110, 36, -69, -30, 37, -78, 27, -54, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 34}, new byte[]{71, -49, -52, 65, -41, 109, -93, 27}));
                hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{-115, -55, 3, 78, -91, -33, TarConstants.LF_GNUTYPE_LONGLINK, 104, -100, -46, 3, TarConstants.LF_GNUTYPE_SPARSE, -74, -34, 90, 123, -98, -59, 89, TarConstants.LF_GNUTYPE_LONGLINK, -91, -43, 93}, new byte[]{-1, -90, 45, 62, -41, -80, 47, 29}));
                int size = m5320WWWWoWWWWo.size();
                int i10 = 0;
                while (i10 < size) {
                    Object obj = m5320WWWWoWWWWo.get(i10);
                    i10++;
                    String str2 = (String) obj;
                    if (!TextUtils.isEmpty(str2)) {
                        if (!AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-80}, new byte[]{-109, 14, 10, -6, -42, 92, 105, -119}, str2) && (indexOf = str2.indexOf(WWWWWWWW.m17835WWWWWWWW(new byte[]{22}, new byte[]{43, 38, 57, -72, -41, 0, TarConstants.LF_MULTIVOLUME, -33}))) > 0) {
                            String substring = str2.substring(0, indexOf);
                            String substring2 = str2.substring(indexOf + 1);
                            if (hashSet.contains(substring)) {
                                vMInstance.m5083WWWWWW(substring, substring2, true);
                            }
                        }
                    }
                }
            }
        }
        return true;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final String getName() {
        byte[] bArr = {-59, -15, -62, 99, -31, -38, TarConstants.LF_SYMLINK, -21};
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-119, -98, -93, 7, -73, -105, 98, -103, -86, -127, -106, 2, -110, -79}, bArr);
    }
}
