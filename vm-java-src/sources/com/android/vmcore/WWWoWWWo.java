package com.android.vmcore;

import com.android.vmcore.bridge.VMEventManager;
import com.android.vmcore.hal.PhoneService;
import com.android.vmcore.hal.phone.CallPdu;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.util.ArrayList;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* renamed from: com.android.vmcore.WWWȏWWWoನ̑  reason: invalid class name */
/* loaded from: classes.dex */
public final /* synthetic */ class WWWoWWWo implements Runnable {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final /* synthetic */ VMInstance f8973WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final /* synthetic */ int f8974WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final /* synthetic */ String f8975WWWWWWWW;

    public /* synthetic */ WWWoWWWo(VMInstance vMInstance, String str, int i10) {
        this.f8974WWWWoWWWWo = i10;
        this.f8973WWWWWWWWWW = vMInstance;
        this.f8975WWWWWWWW = str;
    }

    @Override // java.lang.Runnable
    public final void run() {
        String str = this.f8975WWWWWWWW;
        VMInstance vMInstance = this.f8973WWWWWWWWWW;
        switch (this.f8974WWWWoWWWWo) {
            case 0:
                PhoneService phoneService = vMInstance.f8933WWWWWWWW.getPhoneService();
                phoneService.getClass();
                CallPdu callPdu = new CallPdu();
                ArrayList arrayList = phoneService.f9076WWWWWWWW;
                callPdu.f9127WWWWWWWW = arrayList.size() + 1;
                callPdu.f9126WWWWoWWWWo = 1;
                callPdu.f9130WWWoWWWo = 4;
                callPdu.f9128WWWWWWWW = str;
                callPdu.f9129WWWWWWWW = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
                callPdu.f9131WWoWWo = 4;
                arrayList.add(callPdu);
                StringFog.f8859WWWWWWWW.getClass();
                phoneService.f9070WWWWWWWW.PhoneUnsolicited(WWWWWWWW.m17835WWWWWWWW(new byte[]{-77, -73, -42, -35}, new byte[]{-31, -2, -104, -102, 3, -3, -124, -91}));
                return;
            default:
                VMEventManager vMEventManager = vMInstance.f8935WWWWWWWW;
                if (vMEventManager != null) {
                    byte[] bArr = {-46, 34, 30, -10, -20, -35, 73, TarConstants.LF_BLK, -34, 36, 23, -10, -5, -34, 78, 41, -61, 40, 93, -71, -18, -57, 68, 41, -33, 99, 56, -111, -63, -1, 114, 7, -31, 29};
                    byte[] bArr2 = {-79, TarConstants.LF_MULTIVOLUME, 115, -40, -115, -77, 45, 70};
                    StringFog.f8859WWWWWWWW.getClass();
                    vMEventManager.m5116WWWoWWWo(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), str);
                    return;
                }
                return;
        }
    }
}
