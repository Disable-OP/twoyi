package com.android.vmcore.startup;

import android.net.Uri;
import android.system.ErrnoException;
import android.system.Os;
import android.util.Log;
import com.android.vmapp.VMApp;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.NativeHelper;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.VMStatusEvent;
import com.android.vmcore.installer.ImageInstallerV1;
import com.android.vmcore.utils.ClearAppHelper;
import com.android.vmcore.utils.FileDeleteUtils;
import com.blankj.utilcode.util.WWWWoWWWWo;
import java.io.File;
import java.util.ArrayList;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.archivers.zip.UnixStat;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class XposedTask implements IVMStartupTask {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9283WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static void m5231WWWWWWWW(VMConfig vMConfig) {
        String str = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {-96, -39, 71, -10, -113, TarConstants.LF_GNUTYPE_SPARSE, 0, -24, -19, -61, 80, -86, -102, 70, 29, -104, -1, -40, 81, -26, -98, 69, 30, -12, -67, -11, 70, -11, -108, 69, 8, -93};
        byte[] bArr2 = {-113, -86, 62, -123, -5, TarConstants.LF_FIFO, 109, -57};
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        if (file.exists()) {
            Os.chmod(file.getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        }
        File file2 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-92, -61, 8, TarConstants.LF_FIFO, -105, 78, -127, -78, -23, -39, 31, 106, -126, 91, -100, -62, -5, -62, 30, 38, -122, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -97, -85, -65, -17, 9, TarConstants.LF_DIR, -116, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -119, -7}, new byte[]{-117, -80, 113, 69, -29, 43, -20, -99}));
        if (file2.exists()) {
            Os.chmod(file2.getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        }
        File file3 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-125, 15, -75, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_DIR, 37, -98, -77, -64, 21, -82, 4, 45, 41, -111, -28, -36, 19, -65, 78, 37, 31, -110, -18, -40, 82, -65, 68}, new byte[]{-84, 124, -52, 43, 65, 64, -13, -100}));
        if (file3.exists()) {
            Os.chmod(file3.getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        }
        File file4 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-114, 5, 101, -48, 57, -39, 8, 1, -51, 31, 126, -107, 121, -109, 9, 71, -61, 14, 108, -52, 62, -39, 1, 113, -64, 4, 104, -115, 62, -45}, new byte[]{-95, 118, 28, -93, TarConstants.LF_MULTIVOLUME, -68, 101, 46}));
        if (file4.exists()) {
            Os.chmod(file4.getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        }
        Os.chmod(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, 119, 8, TarConstants.LF_CONTIG, TarConstants.LF_SYMLINK, -71, 125, -110, 126, 118, 16, 41, 35, -85, Byte.MAX_VALUE, -49, 115, 43, 41, TarConstants.LF_BLK, 41, -81, 117, -39, 90, 118, 24, 32, 33, -71, 62, -41, 121, 118}, new byte[]{24, 4, 113, 68, 70, -36, 16, -67})).getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        Os.chmod(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{4, -27, -45, -110, -114, -38, 33, 18, TarConstants.LF_GNUTYPE_SPARSE, -26, -59, -110, -97, -37, 98, TarConstants.LF_MULTIVOLUME, 89, -7, -38}, new byte[]{43, -106, -86, -31, -6, -65, TarConstants.LF_GNUTYPE_LONGNAME, 61})).getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        NativeHelper.chmodRecursively(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{68, TarConstants.LF_LINK, -42, 65, -114, 29, -29, -22, 10, TarConstants.LF_SYMLINK, -33, 29, -94, 8, -31, -74, 14, 38, -26, 92, -119, ConstantPoolEntry.CP_NameAndType, -17, -87, 7, 39, -35, 29}, new byte[]{107, 66, -81, TarConstants.LF_SYMLINK, -6, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -114, -59})).getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        File[] fileArr = {new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{33, -118, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -108, -39, -7, -66, 112, 108, -112, 79, -56, -52, -20, -93, 0, 126, -117, 78, -124, -56, -17, -96, 105, 58}, new byte[]{14, -7, 33, -25, -83, -100, -45, 95})), new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{Byte.MAX_VALUE, 15, -31, 72, -15, 63, 81, -35, TarConstants.LF_SYMLINK, 21, -10, 20, -28, 42, TarConstants.LF_GNUTYPE_LONGNAME, -83, 32, 14, -9, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -32, 41, 79, -63, 98}, new byte[]{80, 124, -104, 59, -123, 90, 60, -14}))};
        File[] fileArr2 = {new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{6, -53, 68, 72, 58, TarConstants.LF_MULTIVOLUME, -82, -46, TarConstants.LF_GNUTYPE_LONGLINK, -47, TarConstants.LF_GNUTYPE_SPARSE, 20, 47, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -77, -94, 89, -54, 82, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 43, 91, -80, -53, 29, -25, 69, TarConstants.LF_GNUTYPE_LONGLINK, 33, 91, -90, -103}, new byte[]{41, -72, 61, 59, 78, 40, -61, -3})), new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-95, 115, -39, -120, 38, -5, -50, -98, -20, 105, -50, -44, TarConstants.LF_CHR, -18, -45, -18, -2, 114, -49, -104, TarConstants.LF_CONTIG, -19, -48, -126, -68, 95, -40, -117, 61, -19, -58, -43}, new byte[]{-114, 0, -96, -5, 82, -98, -93, -79}))};
        for (int i10 = 0; i10 < 2; i10++) {
            if (fileArr2[i10].exists()) {
                WWWWoWWWWo.m5283WWWWWWWW(fileArr2[i10], fileArr[i10]);
                Os.chmod(fileArr[i10].getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
            }
        }
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public static boolean m5232WWWWWWWW(VMConfig vMConfig) {
        String str = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {-89, 71, -104, 115, 69, -30, -115, -40, -23, 68, -111, 47, 105, -9, -113, -124, -19, 80, -88, 110, 66, -13, -127, -101, -28, 81, -109, 47, 94, -26, -108, -40, -23, 70, -116, TarConstants.LF_FIFO, 5};
        byte[] bArr2 = {-120, TarConstants.LF_BLK, -31, 0, TarConstants.LF_LINK, -121, -32, -9};
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        if (!file.exists() || new File(file, WWWWWWWW.m17835WWWWWWWW(new byte[]{8, -20, 13, 39, -93, 29, 4, 69, 35, -24, 3, 56, -86, 28, 63, 5, 63, -8, 7, 44}, new byte[]{80, -100, 98, 84, -58, 121, TarConstants.LF_MULTIVOLUME, 43})).exists()) {
            File file2 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{16, 6, -74, 5, -7, 25, -52, 87, 94, 5, -65, 89, -43, ConstantPoolEntry.CP_NameAndType, -50, ConstantPoolEntry.CP_InterfaceMethodref, 90, 17, -122, 24, -2, 8, -64, 20, TarConstants.LF_GNUTYPE_SPARSE, 16, -67, 89, -20, 14, -52}, new byte[]{63, 117, -49, 118, -115, 124, -95, TarConstants.LF_PAX_EXTENDED_HEADER_LC}));
            if (!file2.exists() || new File(file2, WWWWWWWW.m17835WWWWWWWW(new byte[]{44, -44, 119, -8, -47, 115, -121, 107, 7, -48, 121, -25, -40, 114, -68, 43, 27, -64, 125, -13}, new byte[]{116, -92, 24, -117, -76, 23, -50, 5})).exists()) {
                File file3 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{9, 38, -43, 18, -92, -33, -68, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 71, 37, -36, 78, -120, -54, -66, 59, 67, TarConstants.LF_LINK, -27, 15, -93, -50, -80, 36, 74, TarConstants.LF_NORMAL, -34, 78, -120, -54, -66, 59, 67, TarConstants.LF_LINK, -27, 15, -93, -50, -80, 36, 74, TarConstants.LF_NORMAL, -34, 79, -79, -54, -70}, new byte[]{38, 85, -84, 97, -48, -70, -47, 72}));
                File file4 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{117, -35, 72, 85, -106, 46, 111, 6, 60, -36, 80, TarConstants.LF_GNUTYPE_LONGLINK, -121, 60, 109, 91, TarConstants.LF_LINK, -127, 105, 86, -115, 56, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_MULTIVOLUME, 24, -36, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 66, -123, 46, 44, 67, 59, -36}, new byte[]{90, -82, TarConstants.LF_LINK, 38, -30, TarConstants.LF_GNUTYPE_LONGLINK, 2, 41}));
                File file5 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-8, TarConstants.LF_BLK, 27, 41, -10, TarConstants.LF_CHR, 79, -16, -81, TarConstants.LF_CONTIG, 13, 41, -25, TarConstants.LF_SYMLINK, ConstantPoolEntry.CP_NameAndType, -81, -91, 40, 18}, new byte[]{-41, 71, 98, 90, -126, 86, 34, -33}));
                if (file3.exists() && file4.exists() && file5.exists()) {
                    return true;
                }
                return false;
            }
            return false;
        }
        return false;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5037WWWWoWWWWo() {
        return false;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5038WWWWWWWW() {
        return this.f9283WWWWWWWW;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5039WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        boolean z10 = true;
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        if (!vMConfig.f8911WWoWWo) {
            if (m5232WWWWWWWW(vMConfig)) {
                StringFog.f8859WWWWWWWW.getClass();
                vMInstance.m5053WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{-15, -97, -22, 17, TarConstants.LF_FIFO, -102, 115, -79, -12, -108, -96, 17, TarConstants.LF_FIFO, -111, 97, -79, -19, -118, -85, 16, 60, -100, 43, -10, -5, -119, -80, 2, TarConstants.LF_DIR, -108, 96, -19}, new byte[]{-107, -6, -60, 99, 89, -8, 5, -97}));
            }
            String str = vMConfig.f8868WWWWWWWW;
            WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
            wwwwwwww.getClass();
            FileDeleteUtils.m5262WWWWWWWW(new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-120, 108, 3, 35, 99, -43, 96, 13, -59, 118, 20, Byte.MAX_VALUE, 118, -64, 125, 125, -41, 109, 21, TarConstants.LF_CHR, 114, -61, 126, 17, -107, 64, 2, 32, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -61, 104, 70}, new byte[]{-89, 31, 122, 80, 23, -80, 13, 34})));
            String str2 = vMConfig.f8868WWWWWWWW;
            byte[] bArr = {-90, 66, 72, -14, -38, -15, 126, 25, -21, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 95, -82, -49, -28, 99, 105, -7, 67, 94, -30, -53, -25, 96, 0, -67, 110, 73, -15, -63, -25, 118, 82};
            byte[] bArr2 = {-119, TarConstants.LF_LINK, TarConstants.LF_LINK, -127, -82, -108, 19, TarConstants.LF_FIFO};
            wwwwwwww.getClass();
            FileDeleteUtils.m5262WWWWWWWW(new File(str2, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)));
            String str3 = vMConfig.f8868WWWWWWWW;
            byte[] bArr3 = {TarConstants.LF_NORMAL, -11, 39, -51, -1, -10, -7, 89};
            wwwwwwww.getClass();
            FileDeleteUtils.m5262WWWWWWWW(new File(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{31, -122, 94, -66, -117, -109, -108, 118, 92, -100, 69, -30, -109, -97, -101, 33, 64, -102, 84, -88, -101, -87, -104, 43, 68, -37, 84, -94}, bArr3)));
            String str4 = vMConfig.f8868WWWWWWWW;
            wwwwwwww.getClass();
            FileDeleteUtils.m5262WWWWWWWW(new File(str4, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -29, -76, -54, -85, TarConstants.LF_BLK, -112, -104, 36, -7, -81, -113, -21, 126, -111, -34, 42, -24, -67, -42, -84, TarConstants.LF_BLK, -103, -24, 41, -30, -71, -105, -84, 62}, new byte[]{72, -112, -51, -71, -33, 81, -3, -73})));
            String str5 = vMConfig.f8868WWWWWWWW;
            wwwwwwww.getClass();
            FileDeleteUtils.m5262WWWWWWWW(new File(str5, WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, -117, 44, -16, 119, 14, Byte.MIN_VALUE, -87, -4, -118, TarConstants.LF_BLK, -18, 102, 28, -126, -12, -15, -41, 13, -13, 108, 24, -120, -30, -40, -118, 60, -25, 100, 14, -61, -20, -5, -118}, new byte[]{-102, -8, 85, -125, 3, 107, -19, -122})));
            String str6 = vMConfig.f8868WWWWWWWW;
            wwwwwwww.getClass();
            FileDeleteUtils.m5262WWWWWWWW(new File(str6, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 6, -92, -74, 79, -57, -90, 40, TarConstants.LF_NORMAL, 5, -78, -74, 94, -58, -27, 119, 58, 26, -83}, new byte[]{72, 117, -35, -59, 59, -94, -53, 7})));
            wwwwwwww.getClass();
            String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-59, 42, 113, -62, 36, -26, -113, 23, -18, 46, Byte.MAX_VALUE, -35, 45, -25, -76}, new byte[]{-99, 90, 30, -79, 65, -126, -58, 121});
            byte[] bArr4 = {86, -27, 46, -115, 10, -89, 58, -114, TarConstants.LF_GNUTYPE_SPARSE, -18, 100, -115, 10, -84, 40, -114, 74, -16, 111, -116, 0, -95, 98, -55, 92, -13, 116, -98, 9, -87, 41, -46};
            byte[] bArr5 = {TarConstants.LF_SYMLINK, Byte.MIN_VALUE, 0, -1, 101, -59, TarConstants.LF_GNUTYPE_LONGNAME, -96};
            wwwwwwww.getClass();
            ClearAppHelper.m5246WWWoWWWo(vMConfig, m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5));
            wwwwwwww.getClass();
            String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-6, -75, -38, 105, -76, 7}, new byte[]{-126, -59, -75, 26, -47, 99, -82, -48});
            byte[] bArr6 = {20, 108, -33, 21, -87, 97, 1, -32, 17, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -107, 21, -87, 106, 19, -32, 8, 121, -98, 20, -93, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 89, -89, 30, 122, -123, 6, -86, 111, 18, -68};
            byte[] bArr7 = {112, 9, -15, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -58, 3, 119, -50};
            wwwwwwww.getClass();
            ClearAppHelper.m5246WWWoWWWo(vMConfig, m17835WWWWWWWW2, WWWWWWWW.m17835WWWWWWWW(bArr6, bArr7));
            byte[] bArr8 = {TarConstants.LF_GNUTYPE_LONGLINK, 22, 82, -14, 125, -20, 61, -22};
            wwwwwwww.getClass();
            String m17835WWWWWWWW3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{19, 102, 61, -127, 24, -120}, bArr8);
            byte[] bArr9 = {-38, 57, TarConstants.LF_GNUTYPE_LONGNAME, 43, 94, 28, -77, 24, -33, TarConstants.LF_SYMLINK, 6, 43, 94, 23, -95, 24, -58, 44, 13, 42, 84, 26, -21, 95, -48, 47, 22, 56, 93, 18, -96, 68};
            byte[] bArr10 = {-66, 92, 98, 89, TarConstants.LF_LINK, 126, -59, TarConstants.LF_FIFO};
            wwwwwwww.getClass();
            ClearAppHelper.m5246WWWoWWWo(vMConfig, m17835WWWWWWWW3, WWWWWWWW.m17835WWWWWWWW(bArr9, bArr10));
            String str7 = vMConfig.f8868WWWWWWWW;
            wwwwwwww.getClass();
            File file = new File(str7, WWWWWWWW.m17835WWWWWWWW(new byte[]{73, 97, -114, -96, 112, 32, TarConstants.LF_GNUTYPE_LONGLINK, -24, 4, 123, -103, -4, 101, TarConstants.LF_DIR, 86, -104, 22, 96, -104, -80, 97, TarConstants.LF_FIFO, 85, -15, 82}, new byte[]{102, 18, -9, -45, 4, 69, 38, -57}));
            String str8 = vMConfig.f8868WWWWWWWW;
            wwwwwwww.getClass();
            File[] fileArr = {file, new File(str8, WWWWWWWW.m17835WWWWWWWW(new byte[]{74, TarConstants.LF_NORMAL, -111, -80, 95, 123, 110, -99, 7, 42, -122, -20, 74, 110, 115, -19, 21, TarConstants.LF_LINK, -121, -96, 78, 109, 112, -127, 87}, new byte[]{101, 67, -24, -61, 43, 30, 3, -78}))};
            String str9 = vMConfig.f8868WWWWWWWW;
            byte[] bArr11 = {-37, 67, -90, TarConstants.LF_CHR, -75, -127, -34, 73, -106, 89, -79, 111, -96, -108, -61, 57, -124, 66, -80, 35, -92, -105, -64, 80, -64, 111, -80, TarConstants.LF_SYMLINK, -88, -125};
            byte[] bArr12 = {-12, TarConstants.LF_NORMAL, -33, 64, -63, -28, -77, 102};
            wwwwwwww.getClass();
            File file2 = new File(str9, WWWWWWWW.m17835WWWWWWWW(bArr11, bArr12));
            String str10 = vMConfig.f8868WWWWWWWW;
            byte[] bArr13 = {6, -74, -70, -1, 64, -117, -123, 80, TarConstants.LF_GNUTYPE_LONGLINK, -84, -83, -93, 85, -98, -104, 32, 89, -73, -84, -17, 81, -99, -101, TarConstants.LF_GNUTYPE_LONGNAME, 27, -102, -84, -2, 93, -119};
            byte[] bArr14 = {41, -59, -61, -116, TarConstants.LF_BLK, -18, -24, Byte.MAX_VALUE};
            wwwwwwww.getClass();
            File[] fileArr2 = {file2, new File(str10, WWWWWWWW.m17835WWWWWWWW(bArr13, bArr14))};
            for (int i10 = 0; i10 < 2; i10++) {
                if (fileArr2[i10].exists()) {
                    WWWWoWWWWo.m5283WWWWWWWW(fileArr2[i10], fileArr[i10]);
                    try {
                        Os.chmod(fileArr[i10].getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
                    } catch (ErrnoException unused) {
                    }
                }
            }
        } else {
            StringFog.f8859WWWWWWWW.getClass();
            vMInstance.m5063WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{-102, 23, -70, -70, -33, 94, -2, -104, -97, 28, -16, -70, -33, 85, -20, -104, -122, 2, -5, -69, -43, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -90, -33, -112, 1, -32, -87, -36, 80, -19, -60}, new byte[]{-2, 114, -108, -56, -80, 60, -120, -74}));
            if (!m5232WWWWWWWW(vMConfig) || !WWWWWWWW.m17835WWWWWWWW(new byte[]{-108, -67, 43, 123}, new byte[]{-6, -46, 69, 30, -6, ConstantPoolEntry.CP_InterfaceMethodref, 18, 22}).equals(vMConfig.f8923WoWo)) {
                vMInstance.f8939WWWoWWWo.m13942WWWWWWWW(new VMStatusEvent(WWWWWWWW.m17835WWWWWWWW(new byte[]{-32, -96, -17, 81, 5, -24, -114, -127, -15, -66, -13, 86, 1, -32}, new byte[]{-119, -50, -100, 37, 100, -124, -30, -34})));
                String[] strArr = new String[1];
                try {
                    ArrayList arrayList = new ArrayList();
                    arrayList.add(Uri.parse(vMConfig.f8895WWWoWWWo.f8858WoWo));
                    String str11 = vMConfig.f8868WWWWWWWW;
                    Uri uri = (Uri) arrayList.get(0);
                    new ImageInstallerV1(vMApp).m5205WWWoWWWo(vMConfig, arrayList, str11, null);
                    m5231WWWWWWWW(vMConfig);
                } catch (Throwable th2) {
                    strArr[0] = Log.getStackTraceString(th2);
                    z10 = false;
                }
                if (!z10) {
                    vMInstance.m5082WWWoWWWo(false);
                    this.f9283WWWWWWWW = strArr[0];
                }
                return z10;
            }
        }
        return true;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-118, -53, 121, 109, 107, -94, -42, 4, -95, -48}, new byte[]{-46, -69, 22, 30, 14, -58, -126, 101});
    }
}
