package com.android.vmcore.hal.phone;
/* loaded from: classes.dex */
public class IccIoResult {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final int f9153WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final int f9154WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final byte[] f9155WWWoWWWo;

    public IccIoResult(int i10, int i11, byte[] bArr) {
        this.f9154WWWWWWWW = i10;
        this.f9153WWWWoWWWWo = i11;
        this.f9155WWWoWWWo = bArr;
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static byte[] m5196WWWWWWWW(int i10, int i11, int i12, int i13) {
        byte[] bArr = new byte[15];
        bArr[0] = 0;
        bArr[1] = 0;
        bArr[2] = (byte) 0;
        bArr[3] = (byte) (i12 & 255);
        bArr[4] = (byte) ((i10 >> 8) & 255);
        bArr[5] = (byte) (i10 & 255);
        bArr[6] = 4;
        bArr[7] = 0;
        bArr[8] = 0;
        bArr[9] = 0;
        bArr[10] = 0;
        bArr[11] = 1;
        bArr[12] = 2;
        bArr[13] = (byte) i11;
        if (i11 == 0) {
            bArr[14] = 0;
            return bArr;
        }
        bArr[14] = (byte) i13;
        return bArr;
    }
}
