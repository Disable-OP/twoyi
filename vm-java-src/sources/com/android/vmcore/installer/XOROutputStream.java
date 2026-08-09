package com.android.vmcore.installer;

import java.io.FileOutputStream;
import java.io.FilterOutputStream;
import java.io.OutputStream;
/* loaded from: classes.dex */
public class XOROutputStream extends FilterOutputStream {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public int f9236WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final byte[] f9237WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public byte[] f9238WWWWWWWW;

    public XOROutputStream(FileOutputStream fileOutputStream, String str) {
        super(fileOutputStream);
        this.f9237WWWWoWWWWo = str.getBytes();
        this.f9236WWWWWWWWWW = 0;
    }

    @Override // java.io.FilterOutputStream, java.io.OutputStream
    public final void write(int i10) {
        OutputStream outputStream = ((FilterOutputStream) this).out;
        int i11 = this.f9236WWWWWWWWWW;
        byte[] bArr = this.f9237WWWWoWWWWo;
        outputStream.write(i10 ^ bArr[i11 % bArr.length]);
        this.f9236WWWWWWWWWW++;
    }

    @Override // java.io.FilterOutputStream, java.io.OutputStream
    public final void write(byte[] bArr) {
        byte[] bArr2 = this.f9238WWWWWWWW;
        if (bArr2 == null || bArr2.length != bArr.length) {
            this.f9238WWWWWWWW = new byte[bArr.length];
        }
        for (int i10 = 0; i10 < bArr.length; i10++) {
            byte[] bArr3 = this.f9238WWWWWWWW;
            byte b8 = bArr[i10];
            byte[] bArr4 = this.f9237WWWWoWWWWo;
            bArr3[i10] = (byte) (b8 ^ bArr4[(this.f9236WWWWWWWWWW + i10) % bArr4.length]);
        }
        ((FilterOutputStream) this).out.write(this.f9238WWWWWWWW);
        this.f9236WWWWWWWWWW += bArr.length;
    }

    @Override // java.io.FilterOutputStream, java.io.OutputStream
    public final void write(byte[] bArr, int i10, int i11) {
        byte[] bArr2 = this.f9238WWWWWWWW;
        if (bArr2 == null || bArr2.length != i11) {
            this.f9238WWWWWWWW = new byte[i11];
        }
        for (int i12 = 0; i12 < i11; i12++) {
            byte[] bArr3 = this.f9238WWWWWWWW;
            byte b8 = bArr[i12 + i10];
            byte[] bArr4 = this.f9237WWWWoWWWWo;
            bArr3[i12] = (byte) (b8 ^ bArr4[(this.f9236WWWWWWWWWW + i12) % bArr4.length]);
        }
        ((FilterOutputStream) this).out.write(this.f9238WWWWWWWW);
        this.f9236WWWWWWWWWW += i11;
    }
}
