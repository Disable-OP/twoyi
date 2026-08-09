package com.android.vmcore.hal.phone;

import android.content.Context;
import android.util.Log;
import com.android.providers.telephony.CarrierIdProto;
import com.android.vmcore.StringFog;
import com.blankj.utilcode.util.WWWW;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class CarrierManager {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static final String f9132WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static final String f9133WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static CarrierIdProto.CarrierList f9134WWWoWWWo;

    static {
        StringFog.f8859WWWWWWWW.getClass();
        f9133WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{108, 72, 73, -45, -118, -102, -3, -95, 78, 71, 90, -58, -122, -115}, new byte[]{47, 41, 59, -95, -29, -1, -113, -20});
        f9132WWWWoWWWWo = WWWWWWWW.m17835WWWWWWWW(new byte[]{-14, -91, 111, -93, -33, 40, 0, -84, -3, -83, 110, -91, -104, 61, 16}, new byte[]{-111, -60, 29, -47, -74, TarConstants.LF_MULTIVOLUME, 114, -13});
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static synchronized CarrierIdProto.CarrierList m5190WWWWoWWWWo(Context context) {
        CarrierIdProto.CarrierList carrierList;
        synchronized (CarrierManager.class) {
            try {
                if (f9134WWWoWWWo == null) {
                    f9134WWWoWWWo = m5192WWWoWWWo(context);
                }
                carrierList = f9134WWWoWWWo;
            } catch (Throwable th2) {
                throw th2;
            }
        }
        return carrierList;
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static synchronized CarrierIdProto.CarrierId m5191WWWWWWWW(Context context, String str) {
        synchronized (CarrierManager.class) {
            CarrierIdProto.CarrierList m5190WWWWoWWWWo = m5190WWWWoWWWWo(context);
            if (m5190WWWWoWWWWo != null && m5190WWWWoWWWWo.getCarrierIdCount() != 0) {
                for (int i10 = 0; i10 < m5190WWWWoWWWWo.getCarrierIdCount(); i10++) {
                    CarrierIdProto.CarrierId carrierId = m5190WWWWoWWWWo.getCarrierId(i10);
                    if (str.equals(FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + carrierId.getCanonicalId())) {
                        return carrierId;
                    }
                }
                return null;
            }
            return null;
        }
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static CarrierIdProto.CarrierList m5192WWWoWWWo(Context context) {
        InputStream inputStream;
        InputStream inputStream2 = null;
        try {
            inputStream = context.getAssets().open(f9132WWWWoWWWWo);
            try {
                try {
                    ByteArrayOutputStream byteArrayOutputStream = new ByteArrayOutputStream();
                    byte[] bArr = new byte[16384];
                    while (true) {
                        int read = inputStream.read(bArr, 0, 16384);
                        if (read != -1) {
                            byteArrayOutputStream.write(bArr, 0, read);
                        } else {
                            byteArrayOutputStream.flush();
                            CarrierIdProto.CarrierList parseFrom = CarrierIdProto.CarrierList.parseFrom(byteArrayOutputStream.toByteArray());
                            WWWW.m5322WWWWWWWW(inputStream);
                            return parseFrom;
                        }
                    }
                } catch (IOException e10) {
                    e = e10;
                    String str = f9133WWWWWWWW;
                    StringBuilder sb2 = new StringBuilder();
                    byte[] bArr2 = {-115, -64, 82, -68, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -71, -50, 37, -115, -52, 86, -86, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -74, -58, 36, -117, -123, 85, -86, TarConstants.LF_CONTIG, -73, -113, TarConstants.LF_FIFO, -116, -42, 86, -84, 43, -6, -33, TarConstants.LF_DIR, -33, -61, 82, -79, TarConstants.LF_BLK, -81, -35, TarConstants.LF_SYMLINK, -59, -123};
                    byte[] bArr3 = {-1, -91, TarConstants.LF_CHR, -40, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -38, -81, 87};
                    StringFog.f8859WWWWWWWW.getClass();
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                    sb2.append(e);
                    Log.e(str, sb2.toString());
                    WWWW.m5322WWWWWWWW(inputStream);
                    return null;
                }
            } catch (Throwable th2) {
                th = th2;
                inputStream2 = inputStream;
                WWWW.m5322WWWWWWWW(inputStream2);
                throw th;
            }
        } catch (IOException e11) {
            e = e11;
            inputStream = null;
        } catch (Throwable th3) {
            th = th3;
            WWWW.m5322WWWWWWWW(inputStream2);
            throw th;
        }
    }
}
