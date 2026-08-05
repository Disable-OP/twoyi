package com.android.vmcore.hal.phone;

import android.content.res.Resources;
import android.util.Log;
import com.android.vmcore.StringFog;
import com.android.vmcore.hal.phone.GsmAlphabet;
import com.google.android.gms.internal.ads.pr0;
import java.util.ArrayList;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class IccUtils {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static final char[] f9156WWWWWWWW;

    static {
        byte[] bArr = {109, -48, 24, -91, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 68, -23, 111};
        StringFog.f8859WWWWWWWW.getClass();
        WWWWWWWW.m17835WWWWWWWW(new byte[]{36, -77, 123, -16, 19, 45, -123, 28}, bArr);
        f9156WWWWWWWW = new char[]{'0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F'};
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static byte[] m5197WWWWoWWWWo(String str) {
        int[] iArr;
        byte[] bArr;
        GsmAlphabet.TextEncodingDetails textEncodingDetails;
        char c10;
        int i10;
        int i11;
        int i12;
        char c11 = 27;
        int i13 = 160;
        int i14 = 61;
        String str2 = GsmAlphabet.f9137WWWWWWWW;
        Resources.getSystem();
        GsmAlphabet.f9138WWWWWWWW = new int[0];
        GsmAlphabet.f9139WWWWWWWW = new int[0];
        GsmAlphabet.f9143WWoWWo = 0;
        int i15 = -1;
        if (GsmAlphabet.f9138WWWWWWWW.length + GsmAlphabet.f9139WWWWWWWW.length == 0) {
            textEncodingDetails = new GsmAlphabet.TextEncodingDetails();
            int m5195WWWWWWWW = GsmAlphabet.m5195WWWWWWWW(str, false);
            if (m5195WWWWWWWW == -1) {
                textEncodingDetails = null;
            } else {
                textEncodingDetails.f9149WWWWWWWW = 1;
                textEncodingDetails.f9147WWWWoWWWWo = m5195WWWWWWWW;
                if (m5195WWWWWWWW > 160) {
                    int i16 = (m5195WWWWWWWW + 152) / 153;
                    textEncodingDetails.f9148WWWWWWWW = i16;
                    textEncodingDetails.f9151WWWoWWWo = (i16 * 153) - m5195WWWWWWWW;
                } else {
                    textEncodingDetails.f9148WWWWWWWW = 1;
                    textEncodingDetails.f9151WWWoWWWo = 160 - m5195WWWWWWWW;
                }
            }
            c10 = 0;
            bArr = null;
        } else {
            int i17 = GsmAlphabet.f9143WWoWWo;
            ArrayList arrayList = new ArrayList(GsmAlphabet.f9139WWWWWWWW.length + 1);
            arrayList.add(new GsmAlphabet.LanguagePairCount(0));
            for (int i18 : GsmAlphabet.f9139WWWWWWWW) {
                if (i18 != 0 && !GsmAlphabet.f9140WWWWWWWW[i18].isEmpty()) {
                    arrayList.add(new GsmAlphabet.LanguagePairCount(i18));
                }
            }
            int length = str.length();
            int i19 = 0;
            while (i19 < length && !arrayList.isEmpty()) {
                char charAt = str.charAt(i19);
                if (charAt == c11) {
                    byte[] bArr2 = new byte[i14];
                    // fill-array-data instruction
                    bArr2[0] = 29;
                    bArr2[1] = -62;
                    bArr2[2] = 39;
                    bArr2[3] = -24;
                    bArr2[4] = -51;
                    bArr2[5] = 29;
                    bArr2[6] = 67;
                    bArr2[7] = -87;
                    bArr2[8] = 45;
                    bArr2[9] = -56;
                    bArr2[10] = 34;
                    bArr2[11] = -14;
                    bArr2[12] = -36;
                    bArr2[13] = 46;
                    bArr2[14] = 67;
                    bArr2[15] = -20;
                    bArr2[16] = 87;
                    bArr2[17] = -115;
                    bArr2[18] = 33;
                    bArr2[19] = -14;
                    bArr2[20] = -53;
                    bArr2[21] = 51;
                    bArr2[22] = 94;
                    bArr2[23] = -93;
                    bArr2[24] = 94;
                    bArr2[25] = -50;
                    bArr2[26] = 61;
                    bArr2[27] = -24;
                    bArr2[28] = -51;
                    bArr2[29] = 59;
                    bArr2[30] = 89;
                    bArr2[31] = -86;
                    bArr2[32] = 13;
                    bArr2[33] = -115;
                    bArr2[34] = 23;
                    bArr2[35] = -11;
                    bArr2[36] = -38;
                    bArr2[37] = 59;
                    bArr2[38] = 64;
                    bArr2[39] = -95;
                    bArr2[40] = 94;
                    bArr2[41] = -50;
                    bArr2[42] = 58;
                    bArr2[43] = -25;
                    bArr2[44] = -53;
                    bArr2[45] = 59;
                    bArr2[46] = 83;
                    bArr2[47] = -80;
                    bArr2[48] = 27;
                    bArr2[49] = -33;
                    bArr2[50] = 126;
                    bArr2[51] = -90;
                    bArr2[52] = -48;
                    bArr2[53] = 61;
                    bArr2[54] = 94;
                    bArr2[55] = -85;
                    bArr2[56] = 12;
                    bArr2[57] = -60;
                    bArr2[58] = 60;
                    bArr2[59] = -31;
                    bArr2[60] = -104;
                    byte[] bArr3 = {126, -83, 82, -122, -71, 90, TarConstants.LF_NORMAL, -60};
                    StringFog.f8859WWWWWWWW.getClass();
                    Log.w(GsmAlphabet.f9137WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                } else {
                    int size = arrayList.size();
                    int i20 = 0;
                    while (i20 < size) {
                        Object obj = arrayList.get(i20);
                        i20++;
                        GsmAlphabet.LanguagePairCount languagePairCount = (GsmAlphabet.LanguagePairCount) obj;
                        int i21 = GsmAlphabet.f9136WWWWoWWWWo[languagePairCount.f9145WWWWWWWW].get(charAt, -1);
                        int[] iArr2 = languagePairCount.f9144WWWWoWWWWo;
                        if (i21 == -1) {
                            for (int i22 = 0; i22 <= i17; i22++) {
                                if (iArr2[i22] != -1) {
                                    if (GsmAlphabet.f9142WWWoWWWo[i22].get(charAt, -1) == -1) {
                                        iArr2[i22] = -1;
                                    } else {
                                        iArr2[i22] = iArr2[i22] + 2;
                                    }
                                }
                            }
                        } else {
                            for (int i23 = 0; i23 <= i17; i23++) {
                                int i24 = iArr2[i23];
                                if (i24 != -1) {
                                    iArr2[i23] = i24 + 1;
                                }
                            }
                        }
                    }
                }
                i19++;
                c11 = 27;
                i14 = 61;
            }
            bArr = null;
            textEncodingDetails = new GsmAlphabet.TextEncodingDetails();
            textEncodingDetails.f9148WWWWWWWW = Integer.MAX_VALUE;
            textEncodingDetails.f9149WWWWWWWW = 1;
            int size2 = arrayList.size();
            int i25 = 0;
            while (i25 < size2) {
                Object obj2 = arrayList.get(i25);
                i25++;
                GsmAlphabet.LanguagePairCount languagePairCount2 = (GsmAlphabet.LanguagePairCount) obj2;
                int i26 = 0;
                while (i26 <= i17) {
                    int i27 = languagePairCount2.f9144WWWWoWWWWo[i26];
                    if (i27 != i15) {
                        int i28 = languagePairCount2.f9145WWWWWWWW;
                        if (i28 != 0 && i26 != 0) {
                            i10 = 8;
                        } else if (i28 == 0 && i26 == 0) {
                            i10 = 0;
                        } else {
                            i10 = 5;
                        }
                        if (i27 + i10 > i13) {
                            if (i10 == 0) {
                                i10 = 1;
                            }
                            int i29 = 160 - (i10 + 6);
                            i12 = ((i27 + i29) - 1) / i29;
                            i11 = (i29 * i12) - i27;
                        } else {
                            i11 = (160 - i10) - i27;
                            i12 = 1;
                        }
                        int i30 = languagePairCount2.f9146WWWoWWWo[i26];
                        int i31 = textEncodingDetails.f9148WWWWWWWW;
                        if (i12 < i31 || (i12 == i31 && i11 > textEncodingDetails.f9151WWWoWWWo)) {
                            textEncodingDetails.f9148WWWWWWWW = i12;
                            textEncodingDetails.f9147WWWWoWWWWo = i27;
                            textEncodingDetails.f9151WWWoWWWo = i11;
                            textEncodingDetails.f9150WWWWWWWW = i28;
                            textEncodingDetails.f9152WWoWWo = i26;
                        }
                    }
                    i26++;
                    i13 = 160;
                    i15 = -1;
                }
            }
            c10 = 0;
            if (textEncodingDetails.f9148WWWWWWWW == Integer.MAX_VALUE) {
                textEncodingDetails = null;
            }
        }
        if (textEncodingDetails != null && textEncodingDetails.f9149WWWWWWWW == 1) {
            return GsmAlphabet.m5194WWWWoWWWWo(str);
        }
        try {
            byte[] bArr4 = {TarConstants.LF_FIFO, -93, -98, -81, -80, 1, 87, 37};
            byte[] bArr5 = {67, -41, -8, -126, -127, TarConstants.LF_CONTIG, TarConstants.LF_DIR, 64};
            StringFog.f8859WWWWWWWW.getClass();
            byte[] bytes = str.getBytes(WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5));
            byte[] bArr6 = new byte[bytes.length + 1];
            bArr6[c10] = Byte.MIN_VALUE;
            System.arraycopy(bytes, 0, bArr6, 1, bytes.length);
            return bArr6;
        } catch (Exception unused) {
            return bArr;
        }
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static byte m5198WWWWWWWW(char c10) {
        int i10;
        if (c10 >= '0' && c10 <= '9') {
            i10 = c10 - '0';
        } else if (c10 >= 'A' && c10 <= 'F') {
            i10 = c10 - '7';
        } else if (c10 < 'a' || c10 > 'f') {
            return (byte) 0;
        } else {
            i10 = c10 - 'W';
        }
        return (byte) i10;
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static int m5199WWWoWWWo(char c10) {
        if (c10 >= '0' && c10 <= '9') {
            return c10 - '0';
        }
        if (c10 >= 'A' && c10 <= 'F') {
            return c10 - '7';
        }
        if (c10 >= 'a' && c10 <= 'f') {
            return c10 - 'W';
        }
        StringBuilder sb2 = new StringBuilder();
        byte[] bArr = {40, -29, -8, -73, -82, 6, TarConstants.LF_NORMAL, TarConstants.LF_DIR};
        StringFog.f8859WWWWWWWW.getClass();
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{65, -115, -114, -42, -62, 111, 84, 21, 64, -122, Byte.MIN_VALUE, -105, -51, 110, 81, 71, 8, -60}, bArr));
        sb2.append(c10);
        throw new RuntimeException(pr0.m9000WWWWWWWW(new byte[]{-127}, new byte[]{-90, 97, 87, -4, 69, 104, 27, -77}, sb2));
    }
}
