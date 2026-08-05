package com.android.vmcore.utils;

import org.apache.commons.compress.archivers.cpio.CpioConstants;
/* loaded from: classes.dex */
public class OsExt {
    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static byte[] m5265WWWWoWWWWo(String str, String str2) {
        return nativeLgetxattr(str, str2);
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static void m5266WWWWWWWW(int i10, String str) {
        nativeFchmodat(-100, str, i10, CpioConstants.C_IRUSR);
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static void m5267WWWWWWWW(String str, String str2, byte[] bArr) {
        nativeLsetxattr(str, str2, bArr, 0);
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static String[] m5268WWWoWWWo(String str) {
        return nativeLlistxattr(str);
    }

    private static native int nativeErrno();

    private static native int nativeFchmodat(int i10, String str, int i11, int i12);

    private static native byte[] nativeGetxattr(String str, String str2);

    private static native byte[] nativeLgetxattr(String str, String str2);

    private static native String[] nativeListxattr(String str);

    private static native String[] nativeLlistxattr(String str);

    private static native int nativeLsetxattr(String str, String str2, byte[] bArr, int i10);

    private static native int nativeSetxattr(String str, String str2, byte[] bArr, int i10);
}
