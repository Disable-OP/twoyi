package com.android.vmcore.utils;

import android.os.Build;
import android.text.TextUtils;
import com.android.vmcore.StringFog;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class CPUUtils {
    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static boolean m5240WWWWoWWWWo() {
        String[] strArr = Build.SUPPORTED_ABIS;
        int length = strArr.length;
        boolean z10 = false;
        int i10 = 0;
        while (true) {
            if (i10 < length) {
                String str = strArr[i10];
                StringFog.f8859WWWWWWWW.getClass();
                if (!str.contains(WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -93}, new byte[]{-90, -105, 87, -43, 65, -109, 5, -51}))) {
                    break;
                }
                i10++;
            } else {
                z10 = true;
                break;
            }
        }
        return true ^ z10;
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static String m5241WWWWWWWW() {
        String str = Build.SUPPORTED_ABIS[0];
        if (TextUtils.isEmpty(str)) {
            StringFog.f8859WWWWWWWW.getClass();
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{60, 3, 107}, new byte[]{93, 113, 6, -79, -104, 115, -121, -14});
        }
        return str;
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static boolean m5242WWWoWWWo() {
        String[] strArr;
        for (String str : Build.SUPPORTED_ABIS) {
            byte[] bArr = {-9, 89, -26, 110, -113, TarConstants.LF_CONTIG, -58, -104};
            StringFog.f8859WWWWWWWW.getClass();
            if (str.contains(WWWWWWWW.m17835WWWWWWWW(new byte[]{-63, 109}, bArr))) {
                return true;
            }
        }
        return false;
    }
}
