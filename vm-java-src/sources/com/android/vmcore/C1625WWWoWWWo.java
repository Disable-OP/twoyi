package com.android.vmcore;

import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.google.firebase.crashlytics.internal.persistence.CrashlyticsReportPersistence;
import j3.C3164WWWWWWWW;
import java.io.File;
import java.io.FilenameFilter;
import org.apache.commons.compress.archivers.tar.TarConstants;
/* renamed from: com.android.vmcore.WWWoԻWWWoͷԻ  reason: invalid class name and case insensitive filesystem */
/* loaded from: classes.dex */
public final /* synthetic */ class C1625WWWoWWWo implements FilenameFilter {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final /* synthetic */ int f8976WWWWWWWW;

    public /* synthetic */ C1625WWWoWWWo(int i10) {
        this.f8976WWWWWWWW = i10;
    }

    @Override // java.io.FilenameFilter
    public final boolean accept(File file, String str) {
        boolean lambda$static$1;
        boolean isNormalPriorityEventFile;
        switch (this.f8976WWWWWWWW) {
            case 0:
                String str2 = VMManager.f8949WWWoWWWo;
                byte[] bArr = {84, -82, -115, 115, 117, -70, TarConstants.LF_NORMAL, 66};
                return AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{34, -61}, bArr, str);
            case 1:
                String str3 = VMManager.f8949WWWoWWWo;
                return AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-111, 74, -67, 10, -52, -96, 34, 65, Byte.MIN_VALUE, TarConstants.LF_PAX_EXTENDED_HEADER_LC}, new byte[]{-25, 39, -30, 105, -93, -50, 68, 40}, str);
            case 2:
                lambda$static$1 = CrashlyticsReportPersistence.lambda$static$1(file, str);
                return lambda$static$1;
            case 3:
                isNormalPriorityEventFile = CrashlyticsReportPersistence.isNormalPriorityEventFile(file, str);
                return isNormalPriorityEventFile;
            default:
                return AbstractC1017WWWoWWWo.m3444WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{107, 46, 35, -87, -53, -38, 2, -54, 122, 28}, new byte[]{29, 67, 124, -54, -92, -76, 100, -93}, str);
        }
    }
}
