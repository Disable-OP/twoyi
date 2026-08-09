package com.android.vmcore.installer;

import j$.io.DesugarInputStream;
import j$.io.InputStreamRetargetInterface;
import java.io.FilterInputStream;
import java.io.OutputStream;
/* loaded from: classes.dex */
public class XORInputStream extends FilterInputStream implements InputStreamRetargetInterface {
    @Override // java.io.FilterInputStream, java.io.InputStream
    public final boolean markSupported() {
        return false;
    }

    @Override // java.io.FilterInputStream, java.io.InputStream
    public final int read() {
        if (((FilterInputStream) this).in.read() == -1) {
            return -1;
        }
        throw null;
    }

    @Override // j$.io.InputStreamRetargetInterface
    public final /* synthetic */ long transferTo(OutputStream outputStream) {
        return DesugarInputStream.transferTo(this, outputStream);
    }

    @Override // java.io.FilterInputStream, java.io.InputStream
    public final int read(byte[] bArr) {
        int read = ((FilterInputStream) this).in.read(bArr);
        if (read <= 0) {
            return read;
        }
        byte b8 = bArr[0];
        throw null;
    }

    @Override // java.io.FilterInputStream, java.io.InputStream
    public final int read(byte[] bArr, int i10, int i11) {
        int read = ((FilterInputStream) this).in.read(bArr, i10, i11);
        if (read <= 0) {
            return read;
        }
        byte b8 = bArr[i10];
        throw null;
    }
}
