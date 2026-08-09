package com.android.vmcore.hal;

import android.content.Context;
import android.os.Handler;
import android.os.Looper;
import android.os.Vibrator;
import com.android.vmcore.StringFog;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class HWControlService {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final Vibrator f9046WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final Handler f9047WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public long f9048WWWoWWWo;

    static {
        byte[] bArr = {28, 70, -40, 122, -75, TarConstants.LF_BLK, -35, -66};
        StringFog.f8859WWWWWWWW.getClass();
        WWWWWWWW.m17835WWWWWWWW(new byte[]{84, 17, -101, 21, -37, 64, -81, -47, 112, 21, -67, 8, -61, 93, -66, -37}, bArr);
    }

    public HWControlService(Context context) {
        byte[] bArr = {TarConstants.LF_FIFO, -79, 86, 31, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 117, -85, -24};
        byte[] bArr2 = {64, -40, TarConstants.LF_BLK, 109, 57, 1, -60, -102};
        StringFog.f8859WWWWWWWW.getClass();
        this.f9046WWWWoWWWWo = (Vibrator) context.getSystemService(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        this.f9047WWWWWWWW = new Handler(Looper.getMainLooper());
    }
}
