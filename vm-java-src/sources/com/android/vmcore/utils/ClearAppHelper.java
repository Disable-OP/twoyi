package com.android.vmcore.utils;

import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import java.io.File;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class ClearAppHelper {
    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static void m5243WWWWoWWWWo(VMConfig vMConfig, String str, String str2) {
        String str3 = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {-60, -88, TarConstants.LF_GNUTYPE_LONGLINK, -55, -118, -79, 64, -38};
        StringFog.f8859WWWWWWWW.getClass();
        FileDeleteUtils.m5263WWWWWWWW(new File(str3, str.concat(WWWWWWWW.m17835WWWWWWWW(new byte[]{-21, -23, 37, -83, -8, -34, 41, -66, -21, -52, 42, -67, -21, -98}, bArr))), new WWWWWWWW(str2, 5), false);
        FileDeleteUtils.m5263WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, str.concat(WWWWWWWW.m17835WWWWWWWW(new byte[]{-123, -121, 123, -64, 61, 105, -89, -35, -123, -87, 119, -58, 96}, new byte[]{-86, -58, 21, -92, 79, 6, -50, -71}))), new WWWWWWWW(str2, 6), false);
        FileDeleteUtils.m5263WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, str.concat(WWWWWWWW.m17835WWWWWWWW(new byte[]{-47, 81, 14, 66, -11, -87, -6, 43, -47, 125, 5, 66, -18, -89, -68}, new byte[]{-2, 16, 96, 38, -121, -58, -109, 79}))), new WWWWWWWW(str2, 7), false);
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static void m5244WWWWWWWW(VMConfig vMConfig, String str) {
        StringFog.f8859WWWWWWWW.getClass();
        m5243WWWWoWWWWo(vMConfig, WWWWWWWW.m17835WWWWWWWW(new byte[]{63, 35, -37, -114, -121, 111, Byte.MIN_VALUE, -71, 63, 35, -53, -126, -108, 124, -125}, new byte[]{16, 80, -81, -31, -11, 14, -25, -36}), str);
        m5243WWWWoWWWWo(vMConfig, WWWWWWWW.m17835WWWWWWWW(new byte[]{-61, -107, 16, -28, -127, 110, 71, 94, -61, -125, 9, -2, -97, 110, 84, 94, -120, -55, 84}, new byte[]{-20, -26, 100, -117, -13, 15, 32, 59}), str);
        m5243WWWWoWWWWo(vMConfig, WWWWWWWW.m17835WWWWWWWW(new byte[]{-123, 78, 125, -26, TarConstants.LF_BLK, 89, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -30, -50, 67, 125, -67, 101}, new byte[]{-86, 42, 28, -110, 85, 118, 21, -121}), str);
        File[] listFiles = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-12, 57, 91, -106, -15, 8}, new byte[]{-37, 93, 58, -30, -112, 39, 72, 106})).listFiles();
        if (listFiles != null) {
            File file = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, TarConstants.LF_CONTIG, -23, -7, -15, -16, 110, -109, -59, TarConstants.LF_SYMLINK, -89}, new byte[]{-79, TarConstants.LF_GNUTYPE_SPARSE, -120, -115, -112, -33, 10, -14}));
            File file2 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{14, -113, -21, 31, -53, 29, -73, 111, 68, -103, -43, 15, -49, 29, -14, TarConstants.LF_CHR}, new byte[]{33, -21, -118, 107, -86, TarConstants.LF_SYMLINK, -62, 28}));
            File file3 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_BLK, 105, 56, 93, 101, -33, -127, 94, 107, 34}, new byte[]{27, 13, 89, 41, 4, -16, -32, 46}));
            File file4 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-23, -76, 34, -30, TarConstants.LF_GNUTYPE_LONGLINK, -48, -10, -7, -74, -3, 47, -1, 72, -48}, new byte[]{-58, -48, 67, -106, 42, -1, -105, -119}));
            File file5 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 29, TarConstants.LF_CONTIG, -74, 66, 69, -55, Byte.MIN_VALUE, 44, 16, TarConstants.LF_CONTIG, -19}, new byte[]{72, 121, 86, -62, 35, 106, -92, -27}));
            for (File file6 : listFiles) {
                if (!file6.equals(file3) && !file6.equals(file5)) {
                    if (!file6.equals(file) && !file6.equals(file2) && !file6.equals(file4)) {
                        FileDeleteUtils.m5263WWWWWWWW(file6, new WWWWWWWW(str, 4), true);
                    } else {
                        FileDeleteUtils.m5263WWWWWWWW(file6, new WWWWWWWW(str, 3), false);
                    }
                }
            }
        }
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static void m5245WWWWWWWW(VMConfig vMConfig, String str, String str2) {
        String str3 = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {-80, -77, -114, -45, 19, TarConstants.LF_NORMAL, 44, -55, -17, -78, -98, -42, 74, TarConstants.LF_BLK, TarConstants.LF_LINK, -106, -80};
        byte[] bArr2 = {-97, -64, -9, -96, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 85, 65, -26};
        StringFog.f8859WWWWWWWW.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str3, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2).concat(str)));
        FileDeleteUtils.m5263WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-48, -24, -10, TarConstants.LF_DIR, -119, -79, -89, -30, -113, -93}, new byte[]{-1, -116, -105, 65, -24, -98, -58, -110})), new WWWWWWWW(str2, 0), true);
        m5244WWWWWWWW(vMConfig, str2);
        m5244WWWWWWWW(vMConfig, str);
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static void m5246WWWoWWWo(VMConfig vMConfig, String str, String str2) {
        String str3 = vMConfig.f8868WWWWWWWW;
        StringFog.f8859WWWWWWWW.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-95, 95, -98, TarConstants.LF_CONTIG, 116, 42, 81, 112, -17, 92, -105, 107}, new byte[]{-114, 44, -25, 68, 0, 79, 60, 95}).concat(str)));
        FileDeleteUtils.m5263WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{40, 118, -70, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 68, 102, 112, -104, 119, 61}, new byte[]{7, 18, -37, 44, 37, 73, 17, -24})), new WWWWWWWW(str2, 1), true);
        m5244WWWWWWWW(vMConfig, str2);
        m5244WWWWWWWW(vMConfig, str);
    }
}
