package com.android.libadb.core;

import n2.C3534WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
final class PairingContext {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final byte[] f8264WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final long f8265WWWWWWWW;

    public PairingContext(long j10) {
        this.f8265WWWWWWWW = j10;
        this.f8264WWWWoWWWWo = nativeMsg(j10);
    }

    /* JADX INFO: Access modifiers changed from: private */
    public static final native long nativeConstructor(boolean z10, byte[] bArr);

    private final native byte[] nativeDecrypt(long j10, byte[] bArr);

    private final native void nativeDestroy(long j10);

    private final native byte[] nativeEncrypt(long j10, byte[] bArr);

    private final native boolean nativeInitCipher(long j10, byte[] bArr);

    private final native byte[] nativeMsg(long j10);

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final byte[] m4831WWWWoWWWWo(byte[] bArr) {
        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
        WWWWWWWW.m17835WWWWWWWW(new byte[]{-33, 106}, new byte[]{-74, 4, 111, -115, 79, -22, 106, 10});
        return nativeDecrypt(this.f8265WWWWWWWW, bArr);
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final byte[] m4832WWWWWWWW(byte[] bArr) {
        byte[] bArr2 = {-61, -11, 105, 93, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 40, -88, 125};
        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
        WWWWWWWW.m17835WWWWWWWW(new byte[]{-86, -101}, bArr2);
        return nativeEncrypt(this.f8265WWWWWWWW, bArr);
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public final boolean m4833WWWWWWWW(byte[] bArr) {
        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
        WWWWWWWW.m17835WWWWWWWW(new byte[]{102, -88, -4, -28, 44, -107, 9, -39}, new byte[]{18, -64, -103, -115, 94, -40, 122, -66});
        return nativeInitCipher(this.f8265WWWWWWWW, bArr);
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final void m4834WWWoWWWo() {
        nativeDestroy(this.f8265WWWWWWWW);
    }
}
