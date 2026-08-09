package com.android.vmcore;

import com.android.vmcore.bridge.VMEventManager;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* renamed from: com.android.vmcore.WWWWӈWWWWीӈ  reason: invalid class name and case insensitive filesystem */
/* loaded from: classes.dex */
public final /* synthetic */ class RunnableC1624WWWWWWWW implements Runnable {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final /* synthetic */ VMInstance f8970WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final /* synthetic */ int f8971WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final /* synthetic */ boolean f8972WWWWWWWW;

    public /* synthetic */ RunnableC1624WWWWWWWW(VMInstance vMInstance, boolean z10, int i10) {
        this.f8971WWWWoWWWWo = i10;
        this.f8970WWWWWWWWWW = vMInstance;
        this.f8972WWWWWWWW = z10;
    }

    @Override // java.lang.Runnable
    public final void run() {
        boolean z10 = this.f8972WWWWWWWW;
        VMInstance vMInstance = this.f8970WWWWWWWWWW;
        switch (this.f8971WWWWoWWWWo) {
            case 0:
                VMEventManager vMEventManager = vMInstance.f8935WWWWWWWW;
                if (vMEventManager != null) {
                    StringFog.f8859WWWWWWWW.getClass();
                    vMEventManager.m5116WWWoWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{-60, -94, -121, TarConstants.LF_NORMAL, -57, -22, -116, TarConstants.LF_GNUTYPE_LONGNAME, -56, -92, -114, TarConstants.LF_NORMAL, -48, -23, -117, 81, -43, -88, -60, Byte.MAX_VALUE, -59, -16, -127, 81, -55, -29, -71, 91, -14, -37, -90, Byte.MAX_VALUE, -15, -124, -83, 95, -14, -51, -89, 112, -8, -113, -85, TarConstants.LF_GNUTYPE_LONGNAME, -7, -42, -68, 114}, new byte[]{-89, -51, -22, 30, -90, -124, -24, 62}), Boolean.toString(z10));
                    return;
                }
                return;
            default:
                VMEventManager vMEventManager2 = vMInstance.f8935WWWWWWWW;
                if (vMEventManager2 != null) {
                    if (z10) {
                        StringFog.f8859WWWWWWWW.getClass();
                        vMEventManager2.m5116WWWoWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{70, -62, -121, -7, 69, 118, -73, -24, 74, -60, -114, -7, 82, 117, -80, -11, 87, -56, -60, -74, 71, 108, -70, -11, TarConstants.LF_GNUTYPE_LONGLINK, -125, -71, -97, 107, 79, -116, -44, 100, -5, -93, -112, 101, TarConstants.LF_GNUTYPE_LONGNAME, -102, -43, 107, -14, -81, -127, 97, 86, -121}, new byte[]{37, -83, -22, -41, 36, 24, -45, -102}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
                        return;
                    }
                    StringFog.f8859WWWWWWWW.getClass();
                    vMEventManager2.m5116WWWoWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{14, -86, 107, -20, -57, 72, 108, Byte.MIN_VALUE, 2, -84, 98, -20, -48, TarConstants.LF_GNUTYPE_LONGLINK, 107, -99, 31, -96, 40, -93, -59, 82, 97, -99, 3, -21, 78, -117, -30, 99, 87, -68, 44, -109, 79, -123, -25, 114, 65, -67, 35, -102, 67, -108, -29, 104, 92}, new byte[]{109, -59, 6, -62, -90, 38, 8, -14}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
                    return;
                }
                return;
        }
    }
}
