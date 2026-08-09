package com.android.vmcore.hal;

import android.view.Surface;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
/* loaded from: classes.dex */
public class DisplayService {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public long f9045WWWWWWWW;

    public DisplayService(VMInstance vMInstance) {
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        this.f9045WWWWWWWW = nativeSetup(vMConfig.f8866WWWWWWWW, vMConfig.f8895WWWoWWWo.f8847WWWWWWWW);
    }

    private native boolean nativeAddSurface(long j10, int i10, Surface surface, int i11, int i12, float f10);

    private native void nativeDispose(long j10);

    private native float nativeGetFPS(long j10);

    private native boolean nativeRemoveSurface(long j10, int i10);

    private native long nativeSetup(int i10, int i11);

    private native int nativeStartService(long j10, int i10, int i11);

    private native int nativeStopService(long j10);

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final void m5126WWWWoWWWWo(int i10) {
        nativeRemoveSurface(this.f9045WWWWWWWW, i10);
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final void m5127WWWWWWWW(int i10, Surface surface, int i11, int i12, float f10) {
        nativeAddSurface(this.f9045WWWWWWWW, i10, surface, i11, i12, f10);
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final void m5128WWWWWWWW() {
        nativeStopService(this.f9045WWWWWWWW);
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final int m5129WWWoWWWo(int i10, int i11) {
        return nativeStartService(this.f9045WWWWWWWW, i10, i11);
    }

    public final void finalize() {
        try {
            long j10 = this.f9045WWWWWWWW;
            if (j10 != 0) {
                nativeDispose(j10);
                this.f9045WWWWWWWW = 0L;
            }
        } finally {
            super.finalize();
        }
    }
}
