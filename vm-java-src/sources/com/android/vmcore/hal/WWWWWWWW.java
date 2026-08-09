package com.android.vmcore.hal;

import com.bumptech.glide.WWWoWWWo;
import com.google.android.material.datepicker.AbstractC1974WWWWWWWW;
/* renamed from: com.android.vmcore.hal.WWWW̏WWWWβ̏  reason: invalid class name */
/* loaded from: classes.dex */
public final /* synthetic */ class WWWWWWWW implements Runnable {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final /* synthetic */ long f9111WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final /* synthetic */ int f9112WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final /* synthetic */ Object f9113WWWWWWWW;

    public /* synthetic */ WWWWWWWW(Object obj, long j10, int i10) {
        this.f9112WWWWoWWWWo = i10;
        this.f9113WWWWWWWW = obj;
        this.f9111WWWWWWWWWW = j10;
    }

    @Override // java.lang.Runnable
    public final void run() {
        long j10 = this.f9111WWWWWWWWWW;
        Object obj = this.f9113WWWWWWWW;
        switch (this.f9112WWWWoWWWWo) {
            case 0:
                ((HWControlService) obj).f9046WWWWoWWWWo.vibrate(new long[]{0, 0, 0, j10}, -1);
                return;
            default:
                AbstractC1974WWWWWWWW abstractC1974WWWWWWWW = (AbstractC1974WWWWWWWW) obj;
                abstractC1974WWWWWWWW.f24131WWWWoWWWWo.setError(String.format(abstractC1974WWWWWWWW.f24133WWWWWWWW, WWWoWWWo.m5459o(j10).replace(' ', (char) 160)));
                abstractC1974WWWWWWWW.mo12380WWWWWWWW();
                return;
        }
    }
}
