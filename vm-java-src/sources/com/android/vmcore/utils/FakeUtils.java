package com.android.vmcore.utils;

import com.android.vmcore.StringFog;
import com.google.android.gms.internal.ads.pr0;
import java.util.Locale;
import java.util.Random;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p053WWoWWo.AbstractC0470WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class FakeUtils {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static final String f9296WWWWWWWW;

    static {
        StringFog.f8859WWWWWWWW.getClass();
        f9296WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-24, -99, -75, -49, -36, -31, -22, 97, -32, -107, -67, -57, -44, -23, -30, 121, -8, -115, -91, -33, -52, -15, -6, 113, -16, -123}, new byte[]{-87, -33, -10, -117, -103, -89, -83, 41});
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static String m5247WWWWoWWWWo() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-34, 22, -98, -22, -2, -6, 46, TarConstants.LF_GNUTYPE_SPARSE, -119, 29, -109, -2, -12, -26, 46, 86, -119, 25, -107, -30, -11, -31}, new byte[]{-71, 46, -89, -45, -58, -41, 30, 99});
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static int m5248WWWWWWWW(String str) {
        int length = str.length();
        int[] iArr = new int[length];
        int i10 = 0;
        while (i10 < str.length()) {
            int i11 = i10 + 1;
            iArr[i10] = Integer.parseInt(str.substring(i10, i11));
            i10 = i11;
        }
        int i12 = 0;
        for (int i13 = 0; i13 < length - 1; i13++) {
            int i14 = iArr[i13];
            if (i13 % 2 == 0 && (i14 = i14 * 2) > 9) {
                i14 = (i14 / 10) + (i14 % 10);
            }
            i12 += i14;
        }
        return (10 - (i12 % 10)) % 10;
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static String m5249WWWWWWWW() {
        Random random = new Random();
        StringBuilder sb2 = new StringBuilder();
        pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-75, -105}, new byte[]{-115, -82, 14, 1, 94, -56, 61, -60}, sb2);
        Locale locale = Locale.US;
        sb2.append(String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{-119, 8, TarConstants.LF_DIR, -94}, new byte[]{-84, 56, 7, -58, 19, 63, -58, -6}), Integer.valueOf(random.nextInt(100))));
        sb2.append(String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{-16, 80, -93, -101}, new byte[]{-43, 96, -111, -1, 73, ConstantPoolEntry.CP_NameAndType, 87, -94}), Integer.valueOf(random.nextInt(100))));
        sb2.append(String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{101, 70, 100, -114, 9}, new byte[]{64, 118, 85, -68, 109, 15, -48, -102}), Integer.valueOf(random.nextInt(1000000000))));
        sb2.append(m5248WWWWWWWW(sb2.toString()));
        return sb2.toString();
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public static String m5250WWWWWWWW() {
        Random random = new Random();
        StringBuilder sb2 = new StringBuilder();
        Locale locale = Locale.US;
        StringFog.f8859WWWWWWWW.getClass();
        sb2.append(String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{-48, -82, 34, 58}, new byte[]{-11, -98, 20, 94, -35, 57, 13, -69}), 352931));
        sb2.append(String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{115, -28, Byte.MIN_VALUE, -62}, new byte[]{86, -44, -78, -90, 46, 124, -97, 86}), Integer.valueOf(random.nextInt(100))));
        sb2.append(String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{-65, 80, 115, TarConstants.LF_FIFO}, new byte[]{-102, 96, 69, 82, -119, 47, -22, 23}), Integer.valueOf(random.nextInt(1000000))));
        sb2.append(m5248WWWWWWWW(sb2.toString()));
        return sb2.toString();
    }

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public static String m5251WWWWWWWW(String str) {
        Random random = new Random();
        StringBuilder m1577WWWWoWWWWo = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(str);
        if (str.length() == 5) {
            Locale locale = Locale.US;
            StringFog.f8859WWWWWWWW.getClass();
            m1577WWWWoWWWWo.append(String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{114, 71, ConstantPoolEntry.CP_NameAndType, -53, 79}, new byte[]{87, 119, 61, -5, 43, 65, -94, -103}), Integer.valueOf(random.nextInt(1000000000))));
        } else {
            Locale locale2 = Locale.US;
            StringFog.f8859WWWWWWWW.getClass();
            m1577WWWWoWWWWo.append(String.format(locale2, WWWWWWWW.m17835WWWWWWWW(new byte[]{81, -47, -33, -47}, new byte[]{116, -31, -26, -75, -101, 122, 0, -44}), Integer.valueOf(random.nextInt(100000000))));
        }
        return m1577WWWWoWWWWo.toString();
    }

    /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
    public static String m5252WWWWWWWW() {
        Random random = new Random();
        int[] iArr = new int[6];
        for (int i10 = 0; i10 < 6; i10++) {
            iArr[i10] = random.nextInt(255);
        }
        Locale locale = Locale.US;
        StringFog.f8859WWWWWWWW.getClass();
        return String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{-84, 36, -75, 110, 60, 39, 39, -117, -15, 46, -94, 38, TarConstants.LF_BLK, 122, 45, -100, -71, 38, -1, 44, 35, TarConstants.LF_SYMLINK, 37, -63, -77, TarConstants.LF_LINK, -73, 36, 126}, new byte[]{-119, 20, -121, 22, 6, 2, 23, -71}), Integer.valueOf(iArr[0]), Integer.valueOf(iArr[1]), Integer.valueOf(iArr[2]), Integer.valueOf(iArr[3]), Integer.valueOf(iArr[4]), Integer.valueOf(iArr[5]));
    }

    /* renamed from: WWWW֭WWWWྥ֭  reason: contains not printable characters */
    public static String m5253WWWWWWWW() {
        StringBuilder sb2 = new StringBuilder();
        for (int i10 = 0; i10 < 4; i10++) {
            Random random = new Random();
            String str = f9296WWWWWWWW;
            int nextInt = random.nextInt(str.length());
            int i11 = nextInt - 1;
            if (i11 != -1) {
                nextInt = i11;
            }
            sb2.append(str.charAt(nextInt));
        }
        StringBuilder sb3 = new StringBuilder();
        sb3.append((Object) sb2);
        StringFog.f8859WWWWWWWW.getClass();
        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{79, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -114, 27, -19}, new byte[]{16, 47, -57, 93, -92, 100, 6, -65}));
        return sb3.toString();
    }

    /* renamed from: WWWWآWWWWȫآ  reason: contains not printable characters */
    public static String m5254WWWWWWWW() {
        byte[] bArr = {63, 125, 101, 90, TarConstants.LF_CHR, -113, -33, -71, 79};
        byte[] bArr2 = {TarConstants.LF_PAX_EXTENDED_HEADER_LC, TarConstants.LF_MULTIVOLUME, 87, 106, 122, -70, -17, -127};
        StringFog.f8859WWWWWWWW.getClass();
        StringBuilder sb2 = new StringBuilder(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        for (int i10 = 0; i10 < 7; i10++) {
            Random random = new Random();
            String str = f9296WWWWWWWW;
            int nextInt = random.nextInt(str.length());
            int i11 = nextInt - 1;
            if (i11 != -1) {
                nextInt = i11;
            }
            sb2.append(str.charAt(nextInt));
        }
        return sb2.toString();
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static String m5255WWWoWWWo(String str) {
        String upperCase = str.substring(0, 8).toUpperCase(Locale.US);
        StringBuilder sb2 = new StringBuilder();
        pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{91, -127}, new byte[]{99, -79, -38, 17, 19, -27, -41, 98}, sb2);
        sb2.append(upperCase.substring(0, 6));
        return sb2.toString();
    }

    /* renamed from: WWWoԻWWWoͷԻ  reason: contains not printable characters */
    public static String m5256WWWoWWWo() {
        Random random = new Random();
        StringBuilder sb2 = new StringBuilder();
        Locale locale = Locale.US;
        byte[] bArr = {84, -11, 114, TarConstants.LF_LINK, -25, 105, 44, 58};
        StringFog.f8859WWWWWWWW.getClass();
        sb2.append(String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{113, -59, 64, 105}, bArr), Integer.valueOf(random.nextInt(96) + 160)));
        for (int i10 = 0; i10 < 6; i10++) {
            Locale locale2 = Locale.US;
            StringFog.f8859WWWWWWWW.getClass();
            sb2.append(String.format(locale2, WWWWWWWW.m17835WWWWWWWW(new byte[]{122, -109, 95, -34}, new byte[]{95, -93, 110, -122, 16, 118, Byte.MIN_VALUE, -37}), Integer.valueOf(random.nextInt(16))));
        }
        for (int i11 = 0; i11 < 6; i11++) {
            Locale locale3 = Locale.US;
            StringFog.f8859WWWWWWWW.getClass();
            sb2.append(String.format(locale3, WWWWWWWW.m17835WWWWWWWW(new byte[]{126, 67, -83, -78}, new byte[]{91, 115, -100, -22, 32, 62, 112, -31}), Integer.valueOf(random.nextInt(16))));
        }
        return sb2.toString();
    }

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public static String m5257WWoWWo() {
        Random random = new Random();
        Locale locale = Locale.US;
        StringFog.f8859WWWWWWWW.getClass();
        return String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{-95, -88, -74, 37}, new byte[]{-124, -104, -124, 65, 45, -107, 15, -70}), Integer.valueOf(random.nextInt(100)));
    }

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public static boolean m5258WWWW(String str) {
        StringFog.f8859WWWWWWWW.getClass();
        return str.matches(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_SPARSE, -7, -44, Byte.MIN_VALUE, 7, -4, 106, -19, TarConstants.LF_GNUTYPE_LONGLINK, -80, -94, -42, 119, -66, 25, -67, 86, -21, -94, -19, 3, -66, 30, -67, 37, -118, -65, -99, 19, -124, 6, -122, 108, -4, -23, -19, 81, -9, 86, -23, 41}, new byte[]{13, -47, -113, -80, 42, -59, 43, -64}));
    }
}
