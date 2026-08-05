package com.android.vmcore;

import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmcore.startup.MagiskTask;
import j3.C3164WWWWWWWW;
import java.io.File;
import java.io.FileFilter;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* renamed from: com.android.vmcore.WWWWҍWWWWּҍ  reason: invalid class name and case insensitive filesystem */
/* loaded from: classes.dex */
public final /* synthetic */ class C1623WWWWWWWW implements FileFilter {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final /* synthetic */ int f8969WWWWWWWW;

    public /* synthetic */ C1623WWWWWWWW(int i10) {
        this.f8969WWWWWWWW = i10;
    }

    @Override // java.io.FileFilter
    public final boolean accept(File file) {
        switch (this.f8969WWWWWWWW) {
            case 0:
                String str = VMInstance.f8925WWWoWWWo;
                String name = file.getName();
                byte[] bArr = {-72, 87, -80, -55, -107, TarConstants.LF_GNUTYPE_SPARSE, 126, -118};
                if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-53, 35, -47, -67, -32, 32, 33}, bArr, name) || file.getName().startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{8, 15, -17, 71, -102}, new byte[]{101, 110, -97, TarConstants.LF_BLK, -59, 122, 74, TarConstants.LF_DIR})) || file.getName().startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-102, 23, -102, -114}, new byte[]{-1, 111, -1, -47, -118, 3, -122, 43})) || file.getName().startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{Byte.MAX_VALUE, -17, -119, -47, TarConstants.LF_BLK, -109, -114}, new byte[]{18, Byte.MIN_VALUE, -4, -65, 64, -32, -47, 15})) || file.getName().startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, Byte.MIN_VALUE, -62, -60, 114, -18, -34, 35, 82, -99}, new byte[]{38, -18, -74, -101, 2, -127, -73, TarConstants.LF_MULTIVOLUME}))) {
                    return true;
                }
                return false;
            case 1:
                String absolutePath = file.getAbsolutePath();
                if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-96, 38, 23, -105, 7, 84, -16, -52, -22, 45, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{-113, 64, 100, -72, 116, 45, -125, -72}, absolutePath) && (file.getName().endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{94, 86, -6, -23, 96}, new byte[]{112, 57, -98, -116, 24, 106, TarConstants.LF_GNUTYPE_LONGLINK, 67})) || file.getName().endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-31, -98, -120, 118, -100}, new byte[]{-49, -24, -20, 19, -28, 25, 28, 23})) || file.getName().endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{2, -71, -38, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{44, -53, -65, 32, 122, -23, -96, -69})) || file.getName().endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{37, 117, -3, TarConstants.LF_SYMLINK}, new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, 26, -100, 70, -47, -73, TarConstants.LF_BLK, -121})) || file.getName().endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-34, 40, -125, 28}, new byte[]{-16, 73, -15, 104, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 64, 112, 23})))) {
                    return true;
                }
                return false;
            case 2:
                String name2 = file.getName();
                return AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{10, -59, 10, TarConstants.LF_MULTIVOLUME, 78, -118, -65}, new byte[]{Byte.MAX_VALUE, -96, 124, 40, 32, -2, -37, 73}, name2);
            case 3:
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{-84, -120, 73, ConstantPoolEntry.CP_InterfaceMethodref, 113, 105, -39, 30, -89, -126}, new byte[]{-49, -25, 45, 110, 46, 10, -72, 125}).equals(file.getParentFile().getName());
            case 4:
                String str2 = MagiskTask.f9271WWWoWWWo;
                if (!file.isDirectory()) {
                    return true;
                }
                String name3 = file.getName();
                if (!AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-44, -33, 8, 60, -7, -12, -68}, new byte[]{-92, -83, 109, 85, -105, -99, -56, -117}, name3)) {
                    return true;
                }
                return false;
            default:
                String name4 = file.getName();
                byte[] bArr2 = {71, TarConstants.LF_BLK, -9, -87, 92, -47, -25, 19};
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                return name4.endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{105, 91, -107, -53}, bArr2));
        }
    }
}
