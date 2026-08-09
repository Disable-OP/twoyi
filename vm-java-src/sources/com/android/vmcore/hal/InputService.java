package com.android.vmcore.hal;

import com.android.vmcore.VMInstance;
/* loaded from: classes.dex */
public class InputService {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public long f9049WWWWWWWW;

    public InputService(VMInstance vMInstance) {
        this.f9049WWWWWWWW = nativeSetup(vMInstance.f8937WWWoWWWo.f8866WWWWWWWW);
    }

    private native void nativeDispose(long j10);

    private native boolean nativeOnTouchEvent(long j10, int i10, int i11, long j11, float f10, float f11);

    private native long nativeSetup(int i10);

    private native int nativeStartService(long j10);

    private native int nativeStopService(long j10);

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final int m5130WWWWoWWWWo() {
        return nativeStartService(this.f9049WWWWWWWW);
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final void m5131WWWWWWWW(int i10, int i11, long j10, float f10, float f11) {
        nativeOnTouchEvent(this.f9049WWWWWWWW, i10, i11, j10, f10, f11);
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final void m5132WWWoWWWo() {
        nativeStopService(this.f9049WWWWWWWW);
    }

    public final void finalize() {
        try {
            long j10 = this.f9049WWWWWWWW;
            if (j10 != 0) {
                nativeDispose(j10);
                this.f9049WWWWWWWW = 0L;
            }
        } finally {
            super.finalize();
        }
    }
}
