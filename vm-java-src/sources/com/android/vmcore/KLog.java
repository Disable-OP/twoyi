package com.android.vmcore;

import android.util.Log;
import org.apache.commons.compress.archivers.tar.TarConstants;
import vf.AbstractC4470WWWWWWWW;
/* loaded from: classes.dex */
public final class KLog {
    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static void m5040WWWWoWWWWo(String str, String str2) {
        StringBuilder sb2 = new StringBuilder();
        m5045WWoWWo(6, AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{46, -75, -2, -101, 116}, new byte[]{69, -39, -111, -4, 43, -95, -11, -79}, sb2, str), str2);
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static void m5041WWWWWWWW(String str, String str2) {
        StringBuilder sb2 = new StringBuilder();
        m5045WWoWWo(3, AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{TarConstants.LF_BLK, 13, 118, -125, TarConstants.LF_NORMAL}, new byte[]{95, 97, 25, -28, 111, -106, 59, 21}, sb2, str), str2);
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static void m5042WWWWWWWW() {
        nativeFlushKLog();
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public static void m5043WWWWWWWW(String str, String str2) {
        StringBuilder sb2 = new StringBuilder();
        m5045WWoWWo(4, AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-119, 100, -83, -13, 6}, new byte[]{-30, 8, -62, -108, 89, 21, -107, 66}, sb2, str), str2);
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static void m5044WWWoWWWo(String str, String str2, Throwable th2) {
        StringBuilder sb2 = new StringBuilder();
        String m17683WWWWWWWW = AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{99, -123, -60, -108, TarConstants.LF_CHR}, new byte[]{8, -23, -85, -13, 108, -117, TarConstants.LF_CONTIG, -81}, sb2, str);
        m5045WWoWWo(6, m17683WWWWWWWW, str2 + '\n' + Log.getStackTraceString(th2));
    }

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public static void m5045WWoWWo(int i10, String str, String str2) {
        StringBuilder sb2 = new StringBuilder();
        for (int i11 = 0; i11 < str2.length(); i11++) {
            char charAt = str2.charAt(i11);
            if (charAt <= 127 && charAt != '%') {
                sb2.append(charAt);
            } else {
                sb2.append('*');
            }
        }
        nativeKLog(i10, str, sb2.toString());
    }

    private static native void nativeFlushKLog();

    private static native void nativeKLog(int i10, String str, String str2);
}
