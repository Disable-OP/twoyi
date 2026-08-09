package com.android.vmcore.setup;

import android.os.Build;
import com.android.vmapp.VMApp;
import com.android.vmcore.IVMSetupTask;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.blankj.utilcode.util.WWWW;
import com.blankj.utilcode.util.WWWWoWWWWo;
import java.io.File;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class FixFsTask implements IVMSetupTask {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public int f9253WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9254WWWWWWWW;

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5034WWWWoWWWWo() {
        return true;
    }

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5035WWWWWWWW() {
        return this.f9254WWWWWWWW;
    }

    @Override // com.android.vmcore.IVMSetupTask
    /* renamed from: WWWȏWWWoನ̑ */
    public final boolean mo5036WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        boolean z10;
        boolean z11;
        boolean z12;
        boolean z13 = false;
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        int i10 = Build.VERSION.SDK_INT;
        if (i10 >= 29 && WWWW.f9393WWWoWWWo[0].equals(WWWW.m5348WoWo().f9367WWWWWWWW)) {
            VMConfig vMConfig2 = vMInstance.f8937WWWoWWWo;
            String str = vMConfig2.f8868WWWWWWWW;
            byte[] bArr = {-62, TarConstants.LF_GNUTYPE_LONGLINK, 24, 87, 126, -59, TarConstants.LF_NORMAL, -127};
            StringFog.f8859WWWWWWWW.getClass();
            File file = new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-19, 56, 122, 62, 16, -22, 81, -27, -96, 47, 71, 102, 78}, bArr));
            File file2 = new File(vMConfig2.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{13, -55, 56, -76, -96, -11, TarConstants.LF_SYMLINK, 94, 64, -34}, new byte[]{34, -70, 90, -35, -50, -38, TarConstants.LF_GNUTYPE_SPARSE, 58}));
            if (file.exists()) {
                WWWWoWWWWo.m5283WWWWWWWW(file, file2);
            }
            File file3 = new File(vMConfig2.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{123, -54, 108, 6, -67, 30, 61, 69, TarConstants.LF_FIFO, -48, 123, 90, -70, 19, 15, 91, 100}, new byte[]{84, -71, 21, 117, -55, 123, 80, 106}));
            File file4 = new File(vMConfig2.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-52, 8, 106, 92, -83, -101, -69, 67, -127, 18, 125, 0, -86, -106}, new byte[]{-29, 123, 19, 47, -39, -2, -42, 108}));
            if (file3.exists()) {
                WWWWoWWWWo.m5283WWWWWWWW(file3, file4);
            }
            File file5 = new File(vMConfig2.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-93, 16, 65, -96, -21, 30, -68, 90}, new byte[]{-116, 121, 47, -55, -97, 65, -115, 106}));
            File file6 = new File(vMConfig2.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{16, -74, 42, -49, 44}, new byte[]{63, -33, 68, -90, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -46, 19, 65}));
            if (file5.exists() && !WWWWoWWWWo.m5283WWWWWWWW(file5, file6)) {
                z12 = false;
            } else {
                z12 = true;
            }
            if (!z12) {
                this.f9254WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, 68, 25, 10, -38, 5, -121, -127, TarConstants.LF_MULTIVOLUME, TarConstants.LF_GNUTYPE_LONGLINK, 0, 67, -33, 14, -118}, new byte[]{109, 45, 97, 42, -77, 107, -18, -11});
                this.f9253WWWWoWWWWo = 106000;
            }
            if (!z12) {
                return false;
            }
        }
        if (i10 >= 29) {
            if (vMConfig.f8895WWWoWWWo.f8855WWoWWo) {
                String str2 = vMConfig.f8868WWWWWWWW;
                StringFog.f8859WWWWWWWW.getClass();
                File file7 = new File(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-51, -29, -81, TarConstants.LF_GNUTYPE_SPARSE, 115, -64, -76, Byte.MIN_VALUE, -114, -7, -76, 22, TarConstants.LF_CHR, -118, -75, -58, Byte.MIN_VALUE, -27, -65, 17, TarConstants.LF_CONTIG, -117, -86, -64}, new byte[]{-30, -112, -42, 32, 7, -91, -39, -81}));
                File file8 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-97, -14, -67, -47, -89, 32, 7, 63, -36, -24, -90, -108, -25, 106, 6, 121, -46, -12, -83, -116, -96, 42}, new byte[]{-80, -127, -60, -94, -45, 69, 106, 16}));
                if (file7.exists() && !WWWWoWWWWo.m5283WWWWWWWW(file7, file8)) {
                    z11 = false;
                } else {
                    z11 = true;
                }
                if (!z11) {
                    this.f9254WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-22, -95, -60, -101, -89, 74, -82, 27, -72, -25, -48, -46, -87, 86, -91, 28, -68, -26, -49, -44, -21, 69, -83, 68, -32, -83, -40}, new byte[]{-116, -56, -68, -69, -53, 35, -52, 45});
                    this.f9253WWWWoWWWWo = 106000;
                }
            } else {
                z11 = true;
            }
            if (z11 && vMConfig.f8895WWWoWWWo.f8848WWWWWWWW) {
                String str3 = vMConfig.f8868WWWWWWWW;
                StringFog.f8859WWWWWWWW.getClass();
                File file9 = new File(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-94, -45, -32, 8, 36, 114, 124, 78, -31, -55, -5, 84, 60, 126, 115, 20, -28, -111, -87, 85, 35, TarConstants.LF_PAX_EXTENDED_HEADER_LC}, new byte[]{-115, -96, -103, 123, 80, 23, 17, 97}));
                File file10 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{20, 104, 125, 47, -87, 72, 119, 104, 87, 114, 102, 115, -79, 68, TarConstants.LF_PAX_EXTENDED_HEADER_LC, TarConstants.LF_SYMLINK, 82, TarConstants.LF_DIR, 119, TarConstants.LF_CHR}, new byte[]{59, 27, 4, 92, -35, 45, 26, 71}));
                if (file9.exists() && !WWWWoWWWWo.m5283WWWWWWWW(file9, file10)) {
                    z11 = false;
                } else {
                    z11 = true;
                }
                if (!z11) {
                    this.f9254WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-72, -91, -116, 123, 35, 65, 1, -58, -78, -91, -106, 46, 38, 25, TarConstants.LF_GNUTYPE_SPARSE, -57, -83, -93, -44, 61, 46, 65, 15, -116, -70}, new byte[]{-34, -52, -12, 91, 79, 40, 99, -23});
                    this.f9253WWWWoWWWWo = 106000;
                }
            }
            if (z11 && vMConfig.f8895WWWoWWWo.f8855WWoWWo) {
                String str4 = vMConfig.f8868WWWWWWWW;
                byte[] bArr2 = {97, -29, -87, 65, 89, -96, TarConstants.LF_CONTIG, 17, 34, -7, -78, 4, 25, -22, TarConstants.LF_FIFO, 87, 44, -8, -65, 65, 89, -87, TarConstants.LF_CHR, 92, 59, -7, -113, 3, 29, -21, 41, 81};
                byte[] bArr3 = {78, -112, -48, TarConstants.LF_SYMLINK, 45, -59, 90, 62};
                StringFog.f8859WWWWWWWW.getClass();
                File file11 = new File(str4, WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                File file12 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-70, 33, 34, 115, -40, 56, -19, -15, -7, 59, 57, TarConstants.LF_FIFO, -104, 114, -20, -73, -9, 58, TarConstants.LF_BLK, 115, -40, TarConstants.LF_LINK, -23, -68, -32, 59, 117, 115, -61}, new byte[]{-107, 82, 91, 0, -84, 93, Byte.MIN_VALUE, -34}));
                if (file11.exists() && !WWWWoWWWWo.m5283WWWWWWWW(file11, file12)) {
                    z11 = false;
                } else {
                    z11 = true;
                }
                if (!z11) {
                    this.f9254WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-62, -4, -37, 122, 61, -109, -14, -35, -112, -70, -49, TarConstants.LF_CHR, TarConstants.LF_CHR, -110, -1, -104, -48, -7, -54, 56, 36, -109, -49, -38, -108, -69, -48, TarConstants.LF_DIR, 113, -100, -15, -126, -56, -16, -57}, new byte[]{-92, -107, -93, 90, 81, -6, -112, -21});
                    this.f9253WWWWoWWWWo = 106000;
                }
            }
            if (z11 && vMConfig.f8895WWWoWWWo.f8848WWWWWWWW) {
                String str5 = vMConfig.f8868WWWWWWWW;
                StringFog.f8859WWWWWWWW.getClass();
                File file13 = new File(str5, WWWWWWWW.m17835WWWWWWWW(new byte[]{-49, -38, -82, -84, -58, 43, -73, -18, -116, -64, -75, -16, -34, 39, -72, -87, -113, -38, -93, -77, -37, 44, -81, -88, -65, -104, -25, -15, -63, 33}, new byte[]{-32, -87, -41, -33, -78, 78, -38, -63}));
                z13 = (!file13.exists() || WWWWoWWWWo.m5283WWWWWWWW(file13, new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{47, -62, 21, 72, -92, 78, -7, 58, 108, -40, 14, 20, -68, 66, -10, 125, 111, -62, 24, 87, -71, 73, -31, 124, 46, -62, 3}, new byte[]{0, -79, 108, 59, -48, 43, -108, 21})))) ? true : true;
                if (!z13) {
                    this.f9254WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{95, ConstantPoolEntry.CP_InterfaceMethodref, 61, 0, -118, 125, -18, 86, 85, ConstantPoolEntry.CP_InterfaceMethodref, 39, 72, -119, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -8, 21, 80, 0, TarConstants.LF_NORMAL, 73, -71, 37, -68, 87, 74, 13, 101, 70, -121, 125, -32, 28, 93}, new byte[]{57, 98, 69, 32, -26, 20, -116, 121});
                    this.f9253WWWWoWWWWo = 106000;
                }
                return z13;
            }
            return z11;
        } else if (i10 != 22) {
            return true;
        } else {
            if (vMConfig.f8895WWWoWWWo.f8855WWoWWo) {
                String str6 = vMConfig.f8868WWWWWWWW;
                StringFog.f8859WWWWWWWW.getClass();
                File file14 = new File(str6, WWWWWWWW.m17835WWWWWWWW(new byte[]{-40, -46, 67, 98, 97, -104, -1, 71, -101, -56, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 39, 33, -46, -2, 1, -107, -44, TarConstants.LF_GNUTYPE_SPARSE, 36, 36, -45, -31, 7}, new byte[]{-9, -95, 58, 17, 21, -3, -110, 104}));
                File file15 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{8, -123, 39, 114, -16, 73, -73, TarConstants.LF_GNUTYPE_LONGLINK, TarConstants.LF_GNUTYPE_LONGLINK, -97, 60, TarConstants.LF_CONTIG, -80, 3, -74, 13, 69, -125, TarConstants.LF_CONTIG, 47, -9, 67}, new byte[]{39, -10, 94, 1, -124, 44, -38, 100}));
                if (file14.exists() && !WWWWoWWWWo.m5283WWWWWWWW(file14, file15)) {
                    z10 = false;
                } else {
                    z10 = true;
                }
                if (!z10) {
                    this.f9254WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-25, 105, 20, TarConstants.LF_GNUTYPE_SPARSE, 18, TarConstants.LF_CONTIG, 80, 19, -75, 47, 0, 26, 28, 43, 91, 16, -80, 46, 31, 28, 94, 56, TarConstants.LF_GNUTYPE_SPARSE, TarConstants.LF_GNUTYPE_LONGNAME, -19, 101, 8}, new byte[]{-127, 0, 108, 115, 126, 94, TarConstants.LF_SYMLINK, 37});
                    this.f9253WWWWoWWWWo = 106000;
                }
            } else {
                z10 = true;
            }
            if (z10 && vMConfig.f8895WWWoWWWo.f8848WWWWWWWW) {
                String str7 = vMConfig.f8868WWWWWWWW;
                byte[] bArr4 = {-84, 66, -39, 25, -37, 104, 40, -33, -17, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -62, 69, -61, 100, 39, -123, -22, 4, -111, 68, -36, 98};
                byte[] bArr5 = {-125, TarConstants.LF_LINK, -96, 106, -81, 13, 69, -16};
                StringFog.f8859WWWWWWWW.getClass();
                File file16 = new File(str7, WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5));
                z13 = (!file16.exists() || WWWWoWWWWo.m5283WWWWWWWW(file16, new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{6, 112, 29, -60, -47, 65, -108, -11, 69, 106, 6, -104, -55, TarConstants.LF_MULTIVOLUME, -101, -81, 64, 45, 23, -40}, new byte[]{41, 3, 100, -73, -91, 36, -7, -38})))) ? true : true;
                if (!z13) {
                    this.f9254WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{33, -37, 110, -75, 87, -41, 106, -45, 43, -37, 116, -32, 82, -117, 57, -46, TarConstants.LF_BLK, -35, TarConstants.LF_FIFO, -13, 90, -41, 100, -103, 35}, new byte[]{71, -78, 22, -107, 59, -66, 8, -4});
                    this.f9253WWWWoWWWWo = 106000;
                }
                return z13;
            }
            return z10;
        }
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final int getErrorCode() {
        return this.f9253WWWWoWWWWo;
    }

    @Override // com.android.vmcore.IVMSetupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-55, 86, 117, 84, 31, Byte.MIN_VALUE, 100, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -28}, new byte[]{-113, 63, 13, 18, 108, -44, 5, 43});
    }
}
