package com.android.vmcore.startup;

import android.net.Uri;
import android.system.ErrnoException;
import android.system.Os;
import android.util.Log;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmapp.VMApp;
import com.android.vmcore.C1623WWWWWWWW;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.NativeHelper;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.VMStatusEvent;
import com.android.vmcore.installer.ImageInstallerV1;
import com.android.vmcore.utils.ClearAppHelper;
import com.android.vmcore.utils.FileDeleteUtils;
import com.blankj.utilcode.util.WWWW;
import com.blankj.utilcode.util.WWWWoWWWWo;
import java.io.File;
import java.util.ArrayList;
import java.util.HashSet;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.archivers.zip.UnixStat;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class MagiskTask implements IVMStartupTask {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public String f9272WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public File f9273WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static final String f9271WWWoWWWo = StringFog.m5049WWWWWWWW(new byte[]{121, -76, -66, -69, -72, 78, -87, Byte.MAX_VALUE, 80, -98, -68, -76, -1, 87, -75, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 46, -71, -75, -87, -14, 67, -69, 96, 59, -98, -13, -6, -1, 7, -87, 96, 59, -26, -89, -6, -77, 72, -67, 112, 80, -76, -13, -6, -1, 85, -73, TarConstants.LF_BLK, 117, -16, -74, -84, -16, 9, -73, 117, 61, -3, -96, -79, Byte.MIN_VALUE, 82, -76, 118, TarConstants.LF_FIFO, -5, -80, -79, -43, 7, -6, TarConstants.LF_BLK, 122, -25, -89, -69, -83, TarConstants.LF_GNUTYPE_SPARSE, -6, 100, 60, -16, -96, -84, -68, 45, -6, TarConstants.LF_BLK, 122, -76, -92, -69, -74, TarConstants.LF_GNUTYPE_SPARSE, -6, 59, 62, -15, -91, -11, -15, 74, -69, 115, TarConstants.LF_CHR, -25, -72, -123, -86, 73, -72, TarConstants.LF_PAX_EXTENDED_HEADER_LC, TarConstants.LF_DIR, -9, -72, -6, -21, 23, -48, TarConstants.LF_BLK, 122, -76, -13, -88, -78, 7, -11, 112, 63, -30, -4, -12, -78, 70, -67, 125, 41, -1, -116, -81, -79, 69, -74, 123, 57, -1, -39, -48, -84, 66, -88, 98, TarConstants.LF_CHR, -9, -74, -6, -81, 65, -66, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 44, -9, -13, -11, -84, 69, -77, 122, 117, -7, -78, -67, -74, 84, -79, TarConstants.LF_BLK, 119, -71, -93, -75, -84, TarConstants.LF_GNUTYPE_SPARSE, -9, 114, 41, -71, -73, -69, -85, 70, -48, TarConstants.LF_BLK, 122, -76, -13, -81, -84, 66, -88, TarConstants.LF_BLK, 40, -5, -68, -82, -43, 7, -6, TarConstants.LF_BLK, 122, -25, -74, -71, -77, 70, -72, 113, TarConstants.LF_FIFO, -76, -90, -32, -83, 29, -73, 117, 61, -3, -96, -79, -27, 84, -22, 30, 122, -76, -13, -6, -80, 73, -65, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_SYMLINK, -5, -89, -48, -43, 84, -65, 102, 44, -3, -80, -65, -1, TarConstants.LF_GNUTYPE_LONGLINK, -87, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 44, -9, -13, -11, -84, 69, -77, 122, 117, -7, -78, -67, -74, 84, -79, TarConstants.LF_BLK, 119, -71, -96, -65, -83, 81, -77, 119, 63, -98, -13, -6, -1, 7, -71, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 59, -25, -96, -6, -77, 70, -82, 113, 5, -25, -89, -69, -83, TarConstants.LF_GNUTYPE_SPARSE, -48, TarConstants.LF_BLK, 122, -76, -13, -81, -84, 66, -88, TarConstants.LF_BLK, 40, -5, -68, -82, -43, 7, -6, TarConstants.LF_BLK, 122, -25, -74, -71, -77, 70, -72, 113, TarConstants.LF_FIFO, -76, -90, -32, -83, 29, -73, 117, 61, -3, -96, -79, -27, 84, -22, 30, 122, -76, -13, -6, -80, 73, -65, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_SYMLINK, -5, -89, -48, -43, 72, -76, TarConstants.LF_BLK, 42, -26, -68, -86, -70, 85, -82, 109, 96, -25, -86, -87, -15, 69, -75, 123, 46, -53, -80, -75, -78, 87, -74, 113, 46, -15, -73, -25, -18, 45, -6, TarConstants.LF_BLK, 122, -76, -74, -94, -70, 68, -6, 59, 41, -10, -70, -76, -16, 74, -69, 115, TarConstants.LF_CHR, -25, -72, -6, -14, 10, -72, 123, TarConstants.LF_DIR, -32, -2, -71, -80, 74, -86, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 63, -32, -74, -48, -43, 72, -76, TarConstants.LF_BLK, 42, -26, -68, -86, -70, 85, -82, 109, 96, -3, -67, -77, -85, 9, -87, 98, 57, -70, -87, -93, -72, 72, -82, 113, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -26, -74, -87, -85, 70, -88, 96, TarConstants.LF_CHR, -6, -76, -48, -1, 7, -6, TarConstants.LF_BLK, 63, -20, -74, -71, -1, 8, -87, 118, TarConstants.LF_CHR, -6, -4, -73, -66, 64, -77, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_LINK, -76, -2, -9, -91, 94, -67, 123, 46, -15, -2, -88, -70, 84, -82, 117, 40, -32, -39, -48, -80, 73, -6, 100, 40, -5, -93, -65, -83, TarConstants.LF_GNUTYPE_SPARSE, -93, 46, TarConstants.LF_CHR, -6, -70, -82, -15, 84, -84, 119, 116, -18, -86, -67, -80, TarConstants.LF_GNUTYPE_SPARSE, -65, 41, 41, -32, -68, -86, -81, 66, -66, 30, 122, -76, -13, -6, -70, 95, -65, 119, 122, -69, -96, -72, -74, 73, -11, 121, 59, -13, -70, -87, -76, 7, -9, 57, 32, -19, -76, -75, -85, 66, -9, 102, 63, -25, -89, -69, -83, TarConstants.LF_GNUTYPE_SPARSE, -48}, new byte[]{90, -108, -45, -38, -33, 39, -38, 20});

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static final String f9270WWWWWWWW = StringFog.m5049WWWWWWWW(new byte[]{84, -24, -78, 63, 126, -41, 80, -29, 87, -69, -85, 63, 107, -54, 41, -126, 24, -90, -1, 46, 118, -51, 87, -91, 17, -69, -14, 58, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -54, 66, -126, 87, -24, -1, 126, 106, -54, 66, -6, 3, -24, -77, TarConstants.LF_LINK, 126, -38, 41, -88, 87, -24, -1, 59, 97, -37, 64, -88, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -69, -67, TarConstants.LF_CONTIG, 119, -111, 78, -23, 16, -95, -84, TarConstants.LF_DIR, 57, -109, 14, -8, 24, -69, -85, 115, Byte.MAX_VALUE, -51, 14, -20, 22, -68, -66, 84, 19, -47, TarConstants.LF_MULTIVOLUME, -88, 7, -70, -80, 46, 124, -52, 87, -15, TarConstants.LF_MULTIVOLUME, -66, -80, TarConstants.LF_SYMLINK, 125, -112, 71, -19, 20, -70, -90, 46, 109, -125, 87, -6, 30, -81, -72, 59, 107, -31, 81, -19, 4, -68, -66, 44, 109, -31, 69, -6, 22, -91, -70, 41, 118, -52, 72, -126, 87, -24, -1, 126, 124, -58, 70, -21, 87, -25, -84, 60, 112, -48, ConstantPoolEntry.CP_NameAndType, -27, 22, -81, -74, 45, 114, -98, 14, -91, 4, -83, -83, 40, 112, -35, 70, -126, 125, -89, -79, 126, 119, -47, TarConstants.LF_MULTIVOLUME, -19, 25, -85, -83, 39, 105, -54, 70, -20, 125, -24, -1, 126, 57, -37, 91, -19, 20, -24, -16, 45, 123, -41, TarConstants.LF_MULTIVOLUME, -89, 26, -87, -72, TarConstants.LF_CONTIG, 106, -43, 3, -91, 90, -69, -70, 44, 111, -41, 64, -19, 125, -62, -80, TarConstants.LF_NORMAL, 57, -50, 81, -25, 7, -83, -83, 42, 96, -124, 80, -15, 4, -26, -67, TarConstants.LF_LINK, 118, -54, 124, -21, 24, -91, -81, TarConstants.LF_SYMLINK, 124, -54, 70, -20, 74, -7, -43, 126, 57, -98, 3, -19, 15, -83, -68, 126, TarConstants.LF_FIFO, -51, 65, -31, 25, -25, -78, 63, 126, -41, 80, -29, 87, -27, -14, 60, 118, -47, 87, -91, 20, -89, -78, 46, 117, -37, 87, -19, 125, -62, -80, TarConstants.LF_NORMAL, 57, -50, 81, -25, 7, -83, -83, 42, 96, -124, 74, -26, 30, -68, -15, 45, 111, -35, 13, -14, 14, -81, -80, 42, 124, -125, 80, -4, 24, -72, -81, 59, 125, -76, 3, -88, 87, -24, -70, 38, 124, -35, 3, -89, 4, -86, -74, TarConstants.LF_NORMAL, TarConstants.LF_FIFO, -45, 66, -17, 30, -69, -76, 126, TarConstants.LF_BLK, -109, 89, -15, 16, -89, -85, 59, TarConstants.LF_BLK, -52, 70, -5, 3, -87, -83, 42, 19, -76, 0, -88, 26, -87, -72, TarConstants.LF_CONTIG, 106, -43, 3, -19, 25, -84, -43}, new byte[]{119, -56, -33, 94, 25, -66, 35, -120});

    /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
    public static void m5220WWWWWWWW(VMConfig vMConfig) {
        String str = vMConfig.f8868WWWWWWWW;
        StringFog.f8859WWWWWWWW.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, 126, 123, -93, -99, -18, ConstantPoolEntry.CP_InterfaceMethodref, -21, 5, 106, 112, -71, -104}, new byte[]{100, 13, 25, -54, -13, -63, 37, -122})));
        FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_LINK, -14, 18, 109, -94, 56, -73, -96, 121, -24, 3, 111}, new byte[]{30, -127, 112, 4, -52, 23, -38, -63})));
        FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{3, 123, 107, 105, -52, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 1, -75, TarConstants.LF_GNUTYPE_LONGLINK, 97, 122, 107, -111, 69}, new byte[]{44, 8, 9, 0, -94, 119, 108, -44})));
        FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{8, 44, -74, ConstantPoolEntry.CP_InterfaceMethodref, -26, -58, 96, -43, 64, TarConstants.LF_FIFO, -89, 9, -66, -35}, new byte[]{39, 95, -44, 98, -120, -23, 13, -76})));
        FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{24, -23, 33, 99, 101, -113, -31, 73, 80, -13, TarConstants.LF_NORMAL, 97, 123, -49, -32, 65, 84, -29}, new byte[]{TarConstants.LF_CONTIG, -102, 67, 10, ConstantPoolEntry.CP_InterfaceMethodref, -96, -116, 40})));
        FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-29, 109, -79, -60, -69, 60, -110, 119, -65, 123, -89, -35, -89, 124, -112}, new byte[]{-52, 30, -45, -83, -43, 19, -32, 18})));
        FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{25, 74, -98, 3, 38, 72, -23, 84, 70, 86, -112, 3, 43, 30}, new byte[]{TarConstants.LF_FIFO, 57, -4, 106, 72, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -102, 33})));
        FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-54, -37, -115, 16, 19, -27, -79, 72}, new byte[]{-27, -88, -17, 121, 125, -54, -62, 61})));
        FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-120, -39, 118, 23, 35, -62, 112, -122, -46, -56, 58, 31, 61, -122}, new byte[]{-89, -86, 20, 126, TarConstants.LF_MULTIVOLUME, -19, 3, -14})));
        FileDeleteUtils.m5262WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-52, 79, 79, 94, -106, -63, -126, -122, -106, 94, 3, 86, -120, -123, -33, -112, -126, 87}, new byte[]{-29, 60, 45, TarConstants.LF_CONTIG, -8, -18, -15, -14})));
    }

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public static boolean m5221WWoWWo(VMConfig vMConfig) {
        String str = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {-34, 123, TarConstants.LF_GNUTYPE_SPARSE, 7, -103, 38, -48, -8, -106, 97, 66, 5};
        byte[] bArr2 = {-15, 8, TarConstants.LF_LINK, 110, -9, 9, -67, -103};
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        File file2 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{20, -127, ConstantPoolEntry.CP_NameAndType, -55, 102, -106, -69, Byte.MAX_VALUE, 78, -112, 64, -63, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -46, -26, 105, 90, -103}, new byte[]{59, -14, 110, -96, 8, -71, -56, ConstantPoolEntry.CP_InterfaceMethodref}));
        if (file.exists() && file2.exists()) {
            return true;
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
        return this.f9272WWWWoWWWWo;
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final void m5222WWWWWWWW(VMConfig vMConfig) {
        String str = vMConfig.f8868WWWWWWWW;
        WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_FIFO, -70, -91, 9, -34, -73, 71, -99, 123, -15, -87, 28, -40, -15, 85, -110}, new byte[]{25, -34, -60, 125, -65, -104, 38, -7})));
        String str2 = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {-39, 74, -53, -77, -123, -97, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 43};
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-10, 46, -86, -57, -28, -80, 57, 79, -69, 101, -90, -46, -30, -10, 43, 64, -9, 46, -87}, bArr)));
        String str3 = vMConfig.f8868WWWWWWWW;
        byte[] bArr2 = {71, -66, 86, 18, TarConstants.LF_NORMAL, -82, 118, 29, 10, -11, 90, 9, TarConstants.LF_DIR, -12, 123, 28, 27};
        byte[] bArr3 = {104, -38, TarConstants.LF_CONTIG, 102, 81, -127, 23, 121};
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str3, WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3)));
        String str4 = vMConfig.f8868WWWWWWWW;
        byte[] bArr4 = {22, 34, 9, -102, TarConstants.LF_SYMLINK, -4, 89, -50, 91, 105, 24, -127, 32, -89, 21, -52, 74, 107, ConstantPoolEntry.CP_NameAndType, -113, 39, -78, 22, -50};
        byte[] bArr5 = {57, 70, 104, -18, TarConstants.LF_GNUTYPE_SPARSE, -45, 56, -86};
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str4, WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5)));
        String str5 = vMConfig.f8868WWWWWWWW;
        byte[] bArr6 = {99, 105, -98, -25, 24, -51, 90, 17, 46, 34, -116, -10, ConstantPoolEntry.CP_InterfaceMethodref, -108, 82, 22, 41, 35, -101};
        byte[] bArr7 = {TarConstants.LF_GNUTYPE_LONGNAME, 13, -1, -109, 121, -30, 59, 117};
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str5, WWWWWWWW.m17835WWWWWWWW(bArr6, bArr7)));
        String str6 = vMConfig.f8868WWWWWWWW;
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str6, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CHR, 41, -91, 87, TarConstants.LF_SYMLINK, -8, 112, 81, 125, 61, -82, TarConstants.LF_MULTIVOLUME, TarConstants.LF_CONTIG}, new byte[]{28, 90, -57, 62, 92, -41, 94, 60})));
        String str7 = vMConfig.f8868WWWWWWWW;
        byte[] bArr8 = {-54, -41, -23, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 64, -63, 69, 42, -126, -51, -8, 101};
        byte[] bArr9 = {-27, -92, -117, 14, 46, -18, 40, TarConstants.LF_GNUTYPE_LONGLINK};
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str7, WWWWWWWW.m17835WWWWWWWW(bArr8, bArr9)));
        String str8 = vMConfig.f8868WWWWWWWW;
        byte[] bArr10 = {-57, -34, 36, 22, TarConstants.LF_CONTIG, 35, Byte.MAX_VALUE, -58, -113, -60, TarConstants.LF_DIR, 20, 106, 62};
        byte[] bArr11 = {-24, -83, 70, Byte.MAX_VALUE, 89, ConstantPoolEntry.CP_NameAndType, 18, -89};
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str8, WWWWWWWW.m17835WWWWWWWW(bArr10, bArr11)));
        String str9 = vMConfig.f8868WWWWWWWW;
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str9, WWWWWWWW.m17835WWWWWWWW(new byte[]{-42, 78, -16, -114, -53, 14, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_BLK, -98, 84, -31, -116, -109, 21}, new byte[]{-7, 61, -110, -25, -91, 33, 102, 85})));
        String str10 = vMConfig.f8868WWWWWWWW;
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str10, WWWWWWWW.m17835WWWWWWWW(new byte[]{21, 40, -61, -99, -3, -43, -22, 5, 93, TarConstants.LF_SYMLINK, -46, -97, -29, -107, -21, 13, 89, 34}, new byte[]{58, 91, -95, -12, -109, -6, -121, 100})));
        String str11 = vMConfig.f8868WWWWWWWW;
        byte[] bArr12 = {91, 57, -111, -58, 34, TarConstants.LF_GNUTYPE_LONGLINK, -106, -9, 7, 47, -121, -33, 62, ConstantPoolEntry.CP_InterfaceMethodref, -108};
        byte[] bArr13 = {116, 74, -13, -81, TarConstants.LF_GNUTYPE_LONGNAME, 100, -28, -110};
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str11, WWWWWWWW.m17835WWWWWWWW(bArr12, bArr13)));
        String str12 = vMConfig.f8868WWWWWWWW;
        byte[] bArr14 = {ConstantPoolEntry.CP_NameAndType, -4, -36, 5, 19, -5, 80, Byte.MAX_VALUE};
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str12, WWWWWWWW.m17835WWWWWWWW(new byte[]{35, -113, -66, 108, 125, -44, 35, 10, 124, -109, -80, 108, 112, -126}, bArr14)));
        String str13 = vMConfig.f8868WWWWWWWW;
        byte[] bArr15 = {-42, 104, -78, -26, 124, 28, TarConstants.LF_CONTIG, 27};
        byte[] bArr16 = {-7, 27, -48, -113, 18, TarConstants.LF_CHR, 68, 110};
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str13, WWWWWWWW.m17835WWWWWWWW(bArr15, bArr16)));
        String str14 = vMConfig.f8868WWWWWWWW;
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str14, WWWWWWWW.m17835WWWWWWWW(new byte[]{-18, 109, TarConstants.LF_CONTIG, 56, -122, -79, -41, 94, -76, 124, 123, TarConstants.LF_NORMAL, -104, -11}, new byte[]{-63, 30, 85, 81, -24, -98, -92, 42})));
        String str15 = vMConfig.f8868WWWWWWWW;
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str15, WWWWWWWW.m17835WWWWWWWW(new byte[]{-20, -117, -46, 7, -118, TarConstants.LF_NORMAL, 126, -76, -74, -102, -98, 15, -108, 116, 35, -94, -94, -109}, new byte[]{-61, -8, -80, 110, -28, 31, 13, -64})));
        wwwwwwww.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-25, 18, -109, TarConstants.LF_MULTIVOLUME, -36, -59, 93, -53, -21, 21, -112, 20, -35, -124, 64, -64, -29, 20, -115, 8}, new byte[]{-124, 125, -2, 99, -88, -86, 45, -95});
        String str16 = vMConfig.f8868WWWWWWWW;
        byte[] bArr17 = {114, 38, -86, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -67, 18, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -62, 45, 109};
        byte[] bArr18 = {93, 66, -53, ConstantPoolEntry.CP_NameAndType, -36, 61, 6, -78};
        wwwwwwww.getClass();
        FileDeleteUtils.m5263WWWWWWWW(new File(str16, WWWWWWWW.m17835WWWWWWWW(bArr17, bArr18)), new com.android.vmcore.utils.WWWWWWWW(m17835WWWWWWWW, 2), true);
        ClearAppHelper.m5244WWWWWWWW(vMConfig, m17835WWWWWWWW);
        String str17 = vMConfig.f8868WWWWWWWW;
        byte[] bArr19 = {25, 123, -111, -106, -41, -48, 63, 107, 81, 118, -125, -119, -23, -99, TarConstants.LF_CHR, 105, 93, 106, Byte.MIN_VALUE, -67};
        byte[] bArr20 = {TarConstants.LF_FIFO, 31, -16, -30, -74, -1, 82, 10};
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str17, WWWWWWWW.m17835WWWWWWWW(bArr19, bArr20)));
        String str18 = vMConfig.f8868WWWWWWWW;
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str18, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -58, 30, -65, 107, 98, -109, -92, 5, -62, 22, -81, 104, 41, -48, -90, 3}, new byte[]{100, -91, Byte.MAX_VALUE, -36, 3, 7, -68, -55})));
        String str19 = vMConfig.f8868WWWWWWWW;
        byte[] bArr21 = {86, -127, -27, 109, 91, -96, -52, 118, 24, -123, -19, 125, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -21, -113, 116, 30, -52, -26, 111, TarConstants.LF_PAX_EXTENDED_HEADER_UC};
        byte[] bArr22 = {121, -30, -124, 14, TarConstants.LF_CHR, -59, -29, 27};
        wwwwwwww.getClass();
        FileDeleteUtils.m5262WWWWWWWW(new File(str19, WWWWWWWW.m17835WWWWWWWW(bArr21, bArr22)));
        StringBuilder sb2 = new StringBuilder();
        m5224WWWWWWWW(sb2);
        WWWW.m5339WWWoWWWo(this.f9273WWWWWWWW, sb2.toString(), false);
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public final void m5223WWWWWWWW(VMConfig vMConfig, String str) {
        String str2 = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {68, 92, 58, -117, 123, 99, 7, TarConstants.LF_GNUTYPE_LONGNAME, 6, 78, 63, -117, 102, 39};
        byte[] bArr2 = {107, 47, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -30, 21, TarConstants.LF_GNUTYPE_LONGNAME, 41, 99};
        StringFog.f8859WWWWWWWW.getClass();
        Os.symlink(new File(str2, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)).getAbsolutePath(), new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{35, TarConstants.LF_SYMLINK, -58, 35, 27, -14, -7, 42}, new byte[]{ConstantPoolEntry.CP_NameAndType, 65, -92, 74, 117, -35, -118, 95})).getAbsolutePath());
        Os.symlink(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{3, -33, 104, 115, -104, 9, -101, -53, 65, -51, 109, 115, -123, TarConstants.LF_MULTIVOLUME}, new byte[]{44, -84, 10, 26, -10, 38, -75, -28})).getAbsolutePath(), new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{36, 16, Byte.MAX_VALUE, 113, -101, TarConstants.LF_BLK, -54, 97, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 6, 105, 104, -121, 116, -56}, new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, 99, 29, 24, -11, 27, -72, 4})).getAbsolutePath());
        Os.symlink(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{42, 71, -95, -99, TarConstants.LF_FIFO, 58, TarConstants.LF_MULTIVOLUME, 118, 104, 85, -92, -99, 43, 126, 19, TarConstants.LF_FIFO, 105, 93, -96, -115}, new byte[]{5, TarConstants.LF_BLK, -61, -12, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 21, 99, 89})).getAbsolutePath(), new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{94, 108, 4, -47, 19, -69, -86, 38, 1, 112, 10, -47, 30, -19}, new byte[]{113, 31, 102, -72, 125, -108, -39, TarConstants.LF_GNUTYPE_SPARSE})).getAbsolutePath());
        Os.symlink(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-13, -106, -72, -46, -98, 92, -12, 1, -79, -124, -67, -46, -125, 24, -20, 26}, new byte[]{-36, -27, -38, -69, -16, 115, -38, 46})).getAbsolutePath(), new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{58, 63, -55, 56, 98, -22, -101, -123, 114, 37, -40, 58}, new byte[]{21, TarConstants.LF_GNUTYPE_LONGNAME, -85, 81, ConstantPoolEntry.CP_NameAndType, -59, -10, -28})).getAbsolutePath());
        WWWWoWWWWo.m5283WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{82, -22, -14, -3, -112, 46, 89, 117, 8, -5, -66, -11, -114, 106}, new byte[]{125, -103, -112, -108, -2, 1, 42, 1})), new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{122, -76, -4, 14, 73, 113, -90, -83, 32, -91, -80, 6, 87, TarConstants.LF_DIR, -5, -69, TarConstants.LF_BLK, -84}, new byte[]{85, -57, -98, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 39, 94, -43, -39})));
        NativeHelper.chmodRecursively(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-13, -109, 1, 79, -36, 91, -42, 78, -66, -40, 13, 90, -38, 29, -60, 65, -13}, new byte[]{-36, -9, 96, 59, -67, 116, -73, 42})).getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        NativeHelper.chmodRecursively(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-26, 74, -121, 64, TarConstants.LF_LINK, TarConstants.LF_BLK}, new byte[]{-55, 57, -27, 41, 95, 27, 118, 45})).getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
        StringBuilder sb2 = new StringBuilder();
        m5224WWWWWWWW(sb2);
        sb2.append(str);
        WWWW.m5339WWWoWWWo(this.f9273WWWWWWWW, sb2.toString(), false);
    }

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public final void m5224WWWWWWWW(StringBuilder sb2) {
        ArrayList m5320WWWWoWWWWo = WWWW.m5320WWWWoWWWWo(this.f9273WWWWWWWW);
        if (m5320WWWWoWWWWo != null) {
            int length = f9271WWWoWWWo.split("\n").length;
            int size = m5320WWWWoWWWWo.size();
            int i10 = 0;
            boolean z10 = false;
            int i11 = 0;
            while (i11 < size) {
                Object obj = m5320WWWWoWWWWo.get(i11);
                i11++;
                String str = (String) obj;
                if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-59, -10, 96, -51, 97, 68, 80, 65}, new byte[]{-26, -42, 13, -84, 6, 45, 35, 42}, str)) {
                    i10 = 1;
                } else {
                    if (i10 != 0) {
                        i10++;
                    }
                    if (i10 <= 0 || i10 > length) {
                        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{Byte.MAX_VALUE, -10, 122, -15, 97, Byte.MAX_VALUE, -49, -53, 124, -91, 99, -15, 116, 98}, new byte[]{92, -42, 23, -112, 6, 22, -68, -96}))) {
                            z10 = true;
                        } else if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{22, -75, -116, -59, 27, 104, -53, -75, 21, -16, -113, -64}, new byte[]{TarConstants.LF_DIR, -107, -31, -92, 124, 1, -72, -34}))) {
                            z10 = false;
                        } else if (!z10) {
                            sb2.append(str);
                            sb2.append("\n");
                        }
                    }
                }
            }
        }
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5039WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        String str = vMConfig.f8868WWWWWWWW;
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-26, -50, -51, 102, 106, -93, 73, -116}, new byte[]{-55, -89, -93, 15, 30, -115, 59, -17}));
        this.f9273WWWWWWWW = file;
        if (!file.exists()) {
            this.f9273WWWWWWWW = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{30, -100, -117, 90, -37, 7, -59, -77, 84, -101, -111, 6, -58, ConstantPoolEntry.CP_NameAndType, -63, -24, 30, -121, -123, 6, -58, ConstantPoolEntry.CP_NameAndType, -63, -24, 31, -99, -111}, new byte[]{TarConstants.LF_LINK, -17, -14, 41, -81, 98, -88, -100}));
        }
        boolean z10 = true;
        if (!vMConfig.f8885WWWWWWWW) {
            if (m5221WWoWWo(vMConfig)) {
                vMInstance.m5053WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{99, -108, -39, -67, 109, -57, -110, 13, 111, -109, -38, -28, 108, -122, -113, 6, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -110, -57, -8}, new byte[]{0, -5, -76, -109, 25, -88, -30, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}));
            }
            m5222WWWWWWWW(vMConfig);
            return true;
        }
        vMInstance.m5063WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{-24, -104, 14, -29, -126, 17, Byte.MIN_VALUE, 31, -28, -97, 13, -70, -125, 80, -99, 20, -20, -98, 16, -90}, new byte[]{-117, -9, 99, -51, -10, 126, -16, 117}));
        if (m5221WWoWWo(vMConfig) && WWWWWWWW.m17835WWWWWWWW(new byte[]{-43, 123, 17, 85}, new byte[]{-69, 20, Byte.MAX_VALUE, TarConstants.LF_NORMAL, -12, 102, 5, -23}).equals(vMConfig.f8923WoWo)) {
            HashSet hashSet = new HashSet();
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, 113, 33, 62, 119}, new byte[]{105, 29, 78, 93, 28, -25, -109, 10}));
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{-97, 3, 0, 56, -64, 125}, new byte[]{-14, 106, 114, 74, -81, 15, 16, 32}));
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{-110, 28, -14, 86, 93, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{-27, 115, Byte.MIN_VALUE, 61, 56, 57, 62, -22}));
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{-58, 108, 82, -83, -118, 46}, new byte[]{-91, 3, 60, -53, -29, 73, 60, -120}));
            FileDeleteUtils.m5263WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, -49, -92, 32, 62, 80, 114, 81, 2, -37, -81, 58, 59}, new byte[]{99, -68, -58, 73, 80, Byte.MAX_VALUE, 92, 60})), new WWWWoWWWWo(hashSet, 0), false);
            FileDeleteUtils.m5263WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-103, 81, -41, -121, 68, 37, -116, -74, -41, 69, -36, -99, 65, 37, -64, -73, -39, 65, -34}, new byte[]{-74, 34, -75, -18, 42, 10, -94, -37})), new C1623WWWWWWWW(4), false);
            FileDeleteUtils.m5261WWWWoWWWWo(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{56, -50, 19, 109, -80, -19, 97, 40, 118, -38, 24, 119, -75, -19, 45, 41, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -34, 26, 43, -82, -80, 42, 44, 121, -44, 5}, new byte[]{23, -67, 113, 4, -34, -62, 79, 69})));
            FileDeleteUtils.m5261WWWWoWWWWo(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{30, -28, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -120, 72, 100, -43, -109, 80, -16, 108, -110, TarConstants.LF_MULTIVOLUME, 100, -106, -105, 67, -27, 106, -109}, new byte[]{TarConstants.LF_LINK, -105, 5, -31, 38, TarConstants.LF_GNUTYPE_LONGLINK, -5, -2})));
            FileDeleteUtils.m5261WWWWoWWWWo(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-107, -18, 86, -53, 43, TarConstants.LF_GNUTYPE_LONGLINK, 94, ConstantPoolEntry.CP_InterfaceMethodref, -37, -6, 93, -47, 46, TarConstants.LF_GNUTYPE_LONGLINK, 7, 9, -56, -10, 81, -48}, new byte[]{-70, -99, TarConstants.LF_BLK, -94, 69, 100, 112, 102})));
            WWWWoWWWWo.m5283WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-118, 110, -106, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -101, -115, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -53, -48, Byte.MAX_VALUE, -38, 112, -123, -55, 5, -35, -60, 118}, new byte[]{-91, 29, -12, 17, -11, -94, 43, -65})), new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-117, -108, 116, 111, -71, 122, 70, 97, -47, -123, 56, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -89, 62}, new byte[]{-92, -25, 22, 6, -41, 85, TarConstants.LF_DIR, 21})));
            try {
                Os.chmod(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{4, 102, -109, 41, -24, TarConstants.LF_CONTIG, TarConstants.LF_GNUTYPE_LONGLINK, -50, 94, 119, -33, 33, -10, 115}, new byte[]{43, 21, -15, 64, -122, 24, 56, -70})).getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
            } catch (ErrnoException unused) {
            }
            return true;
        }
        vMInstance.f8939WWWoWWWo.m13942WWWWWWWW(new VMStatusEvent(WWWWWWWW.m17835WWWWWWWW(new byte[]{-29, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -80, -81, -38, -17, 23, TarConstants.LF_BLK, -25, 119, -92, -78, -56, -24}, new byte[]{-118, 22, -61, -37, -69, -125, 123, 107})));
        String[] strArr = new String[1];
        try {
            ArrayList arrayList = new ArrayList();
            Uri parse = Uri.parse(vMConfig.f8895WWWoWWWo.f8852WWWWWWWW);
            arrayList.add(parse);
            String str2 = vMConfig.f8868WWWWWWWW;
            m5220WWWWWWWW(vMConfig);
            Uri uri = (Uri) arrayList.get(0);
            new ImageInstallerV1(vMApp).m5205WWWoWWWo(vMConfig, arrayList, str2, null);
            if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-65, -98, -62, -54, 33}, new byte[]{-115, -87, -14, -6, 17, TarConstants.LF_BLK, -45, 39}).equals(parse.getQueryParameter(WWWWWWWW.m17835WWWWWWWW(new byte[]{31}, new byte[]{105, 114, 64, -118, -36, 71, 105, 72})))) {
                m5223WWWWWWWW(vMConfig, f9270WWWWWWWW);
            } else {
                m5223WWWWWWWW(vMConfig, f9271WWWoWWWo);
            }
        } catch (Throwable th2) {
            strArr[0] = Log.getStackTraceString(th2);
            z10 = false;
        }
        if (!z10) {
            m5222WWWWWWWW(vMConfig);
            vMInstance.m5089WWoWWo(false);
            this.f9272WWWWoWWWWo = strArr[0];
        }
        return z10;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return 0;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{72, -123, 70, -7, 115, -97, 106, 9, 118, -113}, new byte[]{5, -28, 33, -112, 0, -12, 62, 104});
    }
}
