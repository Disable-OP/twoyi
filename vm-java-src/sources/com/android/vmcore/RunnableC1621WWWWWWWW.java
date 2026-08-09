package com.android.vmcore;

import com.android.vmcore.bridge.VMEventManager;
import com.android.vmcore.ui.VMSurfaceView;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* renamed from: com.android.vmcore.WWWWͶWWWWᆑͶ  reason: invalid class name and case insensitive filesystem */
/* loaded from: classes.dex */
public final /* synthetic */ class RunnableC1621WWWWWWWW implements Runnable {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final /* synthetic */ int f8962WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final /* synthetic */ int f8963WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final /* synthetic */ int f8964WWWWWWWW;

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public final /* synthetic */ Object f8965WWWW;

    public /* synthetic */ RunnableC1621WWWWWWWW(Object obj, int i10, int i11, int i12) {
        this.f8963WWWWoWWWWo = i12;
        this.f8965WWWW = obj;
        this.f8962WWWWWWWWWW = i10;
        this.f8964WWWWWWWW = i11;
    }

    @Override // java.lang.Runnable
    public final void run() {
        int i10 = this.f8964WWWWWWWW;
        int i11 = this.f8962WWWWWWWWWW;
        Object obj = this.f8965WWWW;
        switch (this.f8963WWWWoWWWWo) {
            case 0:
                String str = VMInstance.f8925WWWoWWWo;
                VMInstance vMInstance = (VMInstance) obj;
                vMInstance.getClass();
                String str2 = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + i11 + " " + i10;
                VMEventManager vMEventManager = vMInstance.f8935WWWWWWWW;
                if (vMEventManager != null) {
                    StringFog.f8859WWWWWWWW.getClass();
                    vMEventManager.m5116WWWoWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{30, TarConstants.LF_NORMAL, 45, -109, -51, -39, 113, -12, 18, TarConstants.LF_FIFO, 36, -109, -38, -38, 118, -23, 15, 58, 110, -36, -49, -61, 124, -23, 19, 113, 19, -8, -30, -13, 74, -51, 56, 6, 31, -8, -6, -14, 91, -46}, new byte[]{125, 95, 64, -67, -84, -73, 21, -122}), str2);
                    return;
                }
                return;
            default:
                ((VMSurfaceView) obj).f9289WWWoWWWo.mo5236WWWWWWWW(i11, i10);
                return;
        }
    }
}
