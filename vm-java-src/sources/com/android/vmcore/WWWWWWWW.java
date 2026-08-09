package com.android.vmcore;

import android.telephony.SmsMessage;
import com.android.vmcore.bridge.VMEventManager;
import com.android.vmcore.hal.PhoneService;
import com.android.vmcore.hal.phone.IccUtils;
import com.blankj.utilcode.util.WoWo;
import java.util.Date;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.json.JSONObject;
/* renamed from: com.android.vmcore.WWWW̏WWWWβ̏  reason: invalid class name */
/* loaded from: classes.dex */
public final /* synthetic */ class WWWWWWWW implements Runnable {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final /* synthetic */ VMInstance f8958WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final /* synthetic */ int f8959WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final /* synthetic */ String f8960WWWWWWWW;

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public final /* synthetic */ String f8961WWWW;

    public /* synthetic */ WWWWWWWW(VMInstance vMInstance, String str, String str2, int i10) {
        this.f8959WWWWoWWWWo = i10;
        this.f8958WWWWWWWWWW = vMInstance;
        this.f8960WWWWWWWW = str;
        this.f8961WWWW = str2;
    }

    @Override // java.lang.Runnable
    public final void run() {
        String str = null;
        VMInstance vMInstance = this.f8958WWWWWWWWWW;
        String str2 = this.f8961WWWW;
        String str3 = this.f8960WWWWWWWW;
        switch (this.f8959WWWWoWWWWo) {
            case 0:
                String str4 = VMInstance.f8925WWWoWWWo;
                vMInstance.getClass();
                try {
                    JSONObject jSONObject = new JSONObject();
                    StringFog.f8859WWWWWWWW.getClass();
                    jSONObject.put(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-91, -103, -115}, new byte[]{-50, -4, -12, -79, 74, -116, -8, -15}), str3);
                    jSONObject.put(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{35, -83, -66, TarConstants.LF_SYMLINK, 16}, new byte[]{85, -52, -46, 71, 117, -11, -65, 95}), str2);
                    VMEventManager vMEventManager = vMInstance.f8935WWWWWWWW;
                    if (vMEventManager != null) {
                        vMEventManager.m5116WWWoWWWo(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-4, -126, 15, -74, -7, -10, -52, -120, -16, -124, 6, -74, -18, -11, -53, -107, -19, -120, TarConstants.LF_GNUTYPE_LONGNAME, -7, -5, -20, -63, -107, -15, -61, TarConstants.LF_LINK, -63, -42, -37, -9, -86, -51, -94, TarConstants.LF_SYMLINK, -57, -35, -50, -19, -76, -53}, new byte[]{-97, -19, 98, -104, -104, -104, -88, -6}), jSONObject.toString());
                        return;
                    }
                    return;
                } catch (Throwable unused) {
                    return;
                }
            default:
                PhoneService phoneService = vMInstance.f8933WWWWWWWW.getPhoneService();
                phoneService.getClass();
                try {
                    WoWo woWo = new WoWo(SmsMessage.class, SmsMessage.class);
                    StringFog.f8859WWWWWWWW.getClass();
                    String m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-45, 113}, new byte[]{-29, 65, 114, 79, 81, -13, TarConstants.LF_LINK, -119});
                    byte[] bArr = ((SmsMessage.SubmitPdu) woWo.m5361WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-38, 64, -70, -48, -116, 35, -2, -105, -56}, new byte[]{-67, 37, -50, -125, -31, 80, -82, -13}), 0, 1, null, str3, str2, Long.valueOf(new Date().getTime())).f9408WWWWoWWWWo).encodedMessage;
                    char[] cArr = IccUtils.f9156WWWWWWWW;
                    if (bArr != null) {
                        StringBuilder sb2 = new StringBuilder(bArr.length * 2);
                        for (int i10 = 0; i10 < bArr.length; i10++) {
                            char[] cArr2 = IccUtils.f9156WWWWWWWW;
                            sb2.append(cArr2[(bArr[i10] >> 4) & 15]);
                            sb2.append(cArr2[bArr[i10] & 15]);
                        }
                        str = sb2.toString();
                    }
                    StringBuilder sb3 = new StringBuilder();
                    StringFog.f8859WWWWWWWW.getClass();
                    sb3.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{21, 102, 72, -23, -42, -74, ConstantPoolEntry.CP_NameAndType, -100, 102, 40, 15}, new byte[]{62, 37, 5, -67, -20, -106, 84, -60}));
                    sb3.append(m17835WWWWWWWW);
                    sb3.append(str);
                    phoneService.f9070WWWWWWWW.PhoneUnsolicited(sb3.toString());
                    return;
                } catch (Throwable th2) {
                    th2.printStackTrace();
                    return;
                }
        }
    }
}
