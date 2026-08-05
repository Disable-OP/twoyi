package com.android.vmcore.hal;

import android.content.Context;
import android.net.wifi.ScanResult;
import android.net.wifi.WifiManager;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.SystemClock;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.google.android.gms.internal.ads.pr0;
import java.util.Locale;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class WiFiService {

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public static final String f9114WWoWWo;

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final HALManager f9115WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final VMInstance f9116WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public HandlerThread f9117WWWWWWWW;

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public Handler f9118WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final WifiManager f9119WWWoWWWo;

    static {
        StringFog.f8859WWWWWWWW.getClass();
        f9114WWoWWo = WWWWWWWW.m17835WWWWWWWW(new byte[]{-33, 108, -100, -16, -101, -6, -122, TarConstants.LF_GNUTYPE_LONGNAME, -31, 102, -65}, new byte[]{-120, 5, -38, -103, -56, -97, -12, 58});
    }

    public WiFiService(Context context, VMInstance vMInstance, HALManager hALManager) {
        this.f9116WWWWWWWW = vMInstance;
        this.f9115WWWWoWWWWo = hALManager;
        Context applicationContext = context.getApplicationContext();
        byte[] bArr = {-42, -11, 44, -19, 125, TarConstants.LF_SYMLINK, -29, 92};
        StringFog.f8859WWWWWWWW.getClass();
        this.f9119WWWoWWWo = (WifiManager) applicationContext.getSystemService(WWWWWWWW.m17835WWWWWWWW(new byte[]{-95, -100, 74, -124}, bArr));
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static String m5187WWWoWWWo(String str) {
        StringBuilder sb2 = new StringBuilder();
        Locale locale = Locale.US;
        byte[] bArr = {-18, 80, -14, TarConstants.LF_DIR};
        byte[] bArr2 = {-53, 96, -64, TarConstants.LF_MULTIVOLUME, TarConstants.LF_CONTIG, 80, -7, -4};
        StringFog.f8859WWWWWWWW.getClass();
        sb2.append(String.format(locale, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), Integer.valueOf(str.length())));
        for (int i10 = 0; i10 < str.length(); i10++) {
            char charAt = str.charAt(i10);
            Locale locale2 = Locale.US;
            StringFog.f8859WWWWWWWW.getClass();
            sb2.append(String.format(locale2, WWWWWWWW.m17835WWWWWWWW(new byte[]{78, 62, 26, 108}, new byte[]{107, 14, 40, 20, 18, 62, 82, -125}), Integer.valueOf(charAt)));
        }
        return sb2.toString();
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final String m5188WWWWoWWWWo(ScanResult scanResult, int i10) {
        String str;
        int i11;
        int i12;
        long uptimeMillis;
        String str2;
        String str3;
        StringBuilder sb2 = new StringBuilder();
        Locale locale = Locale.US;
        WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
        wwwwwwww.getClass();
        sb2.append(String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{-12, 115, 82, -68, 20, 97}, new byte[]{-99, 23, 111, -103, 112, 107, -118, 15}), Integer.valueOf(i10)));
        wwwwwwww.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-126, -110, -69, -6, -16, -101, -79, ConstantPoolEntry.CP_NameAndType, -22}, new byte[]{-32, -31, -56, -109, -108, -90, -108, Byte.MAX_VALUE});
        VMConfig vMConfig = this.f9116WWWWWWWW.f8937WWWoWWWo;
        if (scanResult != null) {
            str = scanResult.BSSID;
        } else {
            str = vMConfig.f8879WWWWWWWW;
        }
        sb2.append(String.format(locale, m17835WWWWWWWW, str));
        byte[] bArr = {110, -36, -43, -88, 67, 8, -10, TarConstants.LF_GNUTYPE_LONGNAME};
        wwwwwwww.getClass();
        String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{8, -82, -80, -39, 126, 45, -123, 70}, bArr);
        if (scanResult != null) {
            i11 = scanResult.frequency;
        } else {
            i11 = 2412;
        }
        sb2.append(String.format(locale, m17835WWWWWWWW2, Integer.valueOf(i11)));
        wwwwwwww.getClass();
        String m17835WWWWWWWW3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{107, -82, -38, 70, 6, -15, -107, -18, 13}, new byte[]{7, -53, -84, 35, 106, -52, -80, -99});
        if (scanResult != null) {
            i12 = scanResult.level;
        } else {
            i12 = -55;
        }
        sb2.append(String.format(locale, m17835WWWWWWWW3, Integer.valueOf(i12)));
        byte[] bArr2 = {-54, -10, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -75, 31, 100, 84, -2};
        wwwwwwww.getClass();
        String m17835WWWWWWWW4 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-66, -123, 30, -120, 58, 23, 94}, bArr2);
        if (scanResult != null) {
            uptimeMillis = scanResult.timestamp;
        } else {
            uptimeMillis = SystemClock.uptimeMillis();
        }
        sb2.append(String.format(locale, m17835WWWWWWWW4, Long.valueOf(uptimeMillis)));
        wwwwwwww.getClass();
        String m17835WWWWWWWW5 = WWWWWWWW.m17835WWWWWWWW(new byte[]{64, -15, -22, -6, -52, 3, -8, -4, 24, -92, -17, -14, -50, 30, -65, -12, TarConstants.LF_GNUTYPE_LONGLINK, -83, -31, -8, -56, 21, -69, -8, 17, -94, -76, -6, -49, 22, -70, -4, 31, -92, -32, -6, -54, 18, -72, -8, TarConstants.LF_GNUTYPE_LONGNAME, -90, -25, -6, -51, 22, -17, -3, 29, -90, -25, -6, -51, 22, -69, -2, 26, -92, -27, -5, -51, 22, -69, -2, 72, -92, -26, -6, -52, 20, -19, -4, 24, -92, -25, -7, -50, 22, -65, -4, 74, -91, -27, -5, -60, 16, -69, -2, TarConstants.LF_MULTIVOLUME, -91, -74, -8, -104, 23, -70, -3, 30, -14, -79, -6, -52, 22, -69, -4, 25, -92, -25, -6, -52, 22, -69, -4, 25, -92, -25, -6, -52, 22, -69, -4, 25, -92, -25, -6, -52, 22, -69, -4, 25, -92, -25, -6, -52, 22, -69, -4, 25, -92, -25, -6, -52, 22, -69, -1, TarConstants.LF_MULTIVOLUME, -91, -31, -6, -54, 22, -77, -4, 25, -92, -25, -6, -52, 22, -69, -4, 25, -92, -25, -6, -52, 22, -69, -4, 25, -92, -25, -6, -52, 22, -69, -4, 25, -92, -25, -6, -52, 22, -69, -4, 25, -92, -25, -6, -52, 22, -69, -5, 79, -92, -17, -6, -56, 22, -69, -4, 25, -92, -25, -6, -52, 22, -69, -4, 25, -96, -25, -82, -104, 22, -78, -4, 25, -91, -25, -5, -60, 22, -71, -4, 25, -92, -25, -5, -97, 22, -69, -4, 25, -16, -77, -5, -60, 22, -69, -7, 25, -14, -27, -6, -50, 22, -70, -4, 24, -84, -25, -6, -52, 22, -72, -83, 29, -92, -25, -6, -52, 20, -68, -83, 29, -92, -25, -6, -52, 18, -71, -8, 26, -95, -78, -6, -52, 16, -71, -1, 27, -90, -79, -6, -52, 66, -17, -4, 79, -92, -25, -85, -52, 69, -67, -4, 25, -92, -26, -6, -52, 22, -69, -88, 28, -92, -25, -6, -52, 22, -69, -4, 26, -11, -27, -82, -53, 66, -24, -58}, new byte[]{41, -108, -41, -54, -4, 38, -117, -52});
        if (scanResult != null) {
            str2 = scanResult.SSID;
        } else {
            str2 = vMConfig.f8908WWoWWo;
        }
        sb2.append(String.format(locale, m17835WWWWWWWW5, m5187WWWoWWWo(str2)));
        pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{-110, -51, 38, -86, TarConstants.LF_MULTIVOLUME, -41, -63, 100, -89, -14, 26, -57}, new byte[]{-12, -95, 71, -51, 62, -22, -102, 33}, sb2);
        wwwwwwww.getClass();
        String m17835WWWWWWWW6 = WWWWWWWW.m17835WWWWWWWW(new byte[]{9, 31, -58, -58, -66, -125, -106, 98}, new byte[]{122, 108, -81, -94, -125, -90, -27, 104});
        if (scanResult != null) {
            str3 = scanResult.SSID;
        } else {
            str3 = vMConfig.f8908WWoWWo;
        }
        sb2.append(String.format(locale, m17835WWWWWWWW6, str3));
        wwwwwwww.getClass();
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-121, -65, TarConstants.LF_CHR, -71, -77}, new byte[]{-70, -126, 14, -124, -71, 122, -106, -28}));
        return sb2.toString();
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final String m5189WWWWWWWW(ScanResult scanResult) {
        String str;
        int i10;
        long uptimeMillis;
        String str2;
        StringBuilder sb2 = new StringBuilder();
        pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-35, 4, TarConstants.LF_MULTIVOLUME, -80}, new byte[]{-76, 97, 112, -70, -4, -22, TarConstants.LF_GNUTYPE_LONGLINK, -71}, sb2);
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{80, 94, 30, -88, -121}, new byte[]{57, 58, 35, -104, -115, 73, 2, -101}));
        Locale locale = Locale.US;
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{35, 33, -35, -52, -87, 117, -10, -16, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{65, 82, -82, -91, -51, 72, -45, -125});
        VMConfig vMConfig = this.f9116WWWWWWWW.f8937WWWoWWWo;
        if (scanResult != null) {
            str = scanResult.BSSID;
        } else {
            str = vMConfig.f8879WWWWWWWW;
        }
        sb2.append(String.format(locale, m17835WWWWWWWW, str));
        String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-102, ConstantPoolEntry.CP_NameAndType, -72, 117, -39, -106, -64, 43}, new byte[]{-4, 126, -35, 4, -28, -77, -77, 33});
        if (scanResult != null) {
            i10 = scanResult.frequency;
        } else {
            i10 = 2412;
        }
        sb2.append(String.format(locale, m17835WWWWWWWW2, Integer.valueOf(i10)));
        String m17835WWWWWWWW3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-115, 98, 80, -21, 20, -90, 106, 23, -21}, new byte[]{-31, 7, 38, -114, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -101, 79, 100});
        int i11 = -55;
        if (scanResult != null) {
            i11 = scanResult.level;
        }
        sb2.append(String.format(locale, m17835WWWWWWWW3, Integer.valueOf(i11)));
        String m17835WWWWWWWW4 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-42, 105, TarConstants.LF_MULTIVOLUME, -95, 6, 21, 68}, new byte[]{-94, 26, 43, -100, 35, 102, 78, 107});
        if (scanResult != null) {
            uptimeMillis = scanResult.timestamp;
        } else {
            uptimeMillis = SystemClock.uptimeMillis();
        }
        sb2.append(String.format(locale, m17835WWWWWWWW4, Long.valueOf(uptimeMillis)));
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{24, -126, 36, 30, -13, -74, 24, -81, 45, -67, 24, 115}, new byte[]{126, -18, 69, 121, Byte.MIN_VALUE, -117, 67, -22}));
        String m17835WWWWWWWW5 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-122, 22, TarConstants.LF_GNUTYPE_SPARSE, 99, -40, 6, 113, ConstantPoolEntry.CP_NameAndType}, new byte[]{-11, 101, 58, 7, -27, 35, 2, 6});
        if (scanResult != null) {
            str2 = scanResult.SSID;
        } else {
            str2 = vMConfig.f8908WWoWWo;
        }
        sb2.append(String.format(locale, m17835WWWWWWWW5, str2));
        return pr0.m9000WWWWWWWW(new byte[]{-74, -110, -55, Byte.MIN_VALUE}, new byte[]{-107, -79, -22, -93, 14, -44, 89, 37}, sb2);
    }
}
