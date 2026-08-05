package com.android.vmcore.startup;

import android.text.TextUtils;
import android.util.Log;
import com.android.vmapp.VMApp;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.utils.FileDeleteUtils;
import com.blankj.utilcode.util.WWWW;
import java.io.File;
import java.net.Inet6Address;
import java.net.InterfaceAddress;
import java.net.NetworkInterface;
import java.util.ArrayList;
import java.util.Enumeration;
import java.util.Locale;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class Bug5FixTask implements IVMStartupTask {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9260WWWWWWWW;

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5037WWWWoWWWWo() {
        return false;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5038WWWWWWWW() {
        return this.f9260WWWWWWWW;
    }

    /* JADX WARN: Removed duplicated region for block: B:41:0x0142  */
    /* JADX WARN: Removed duplicated region for block: B:46:0x014e  */
    /* JADX WARN: Removed duplicated region for block: B:49:0x0214 A[Catch: all -> 0x00da, TryCatch #0 {all -> 0x00da, blocks: (B:4:0x0012, B:5:0x003a, B:8:0x0042, B:10:0x0065, B:12:0x007c, B:14:0x0093, B:16:0x00aa, B:18:0x00c1, B:23:0x00dd, B:47:0x0150, B:49:0x0214, B:52:0x0219, B:53:0x021f, B:55:0x0222, B:57:0x0254, B:60:0x0274, B:59:0x0270, B:61:0x0277, B:63:0x02cb, B:64:0x02d3, B:66:0x02d9, B:68:0x02e7, B:70:0x02f8, B:73:0x02fe, B:75:0x0302, B:79:0x034d, B:83:0x036d, B:78:0x0334, B:62:0x029a, B:30:0x00f4, B:33:0x010d, B:36:0x0126, B:86:0x03d7), top: B:93:0x0012 }] */
    /* JADX WARN: Removed duplicated region for block: B:55:0x0222 A[Catch: all -> 0x00da, TryCatch #0 {all -> 0x00da, blocks: (B:4:0x0012, B:5:0x003a, B:8:0x0042, B:10:0x0065, B:12:0x007c, B:14:0x0093, B:16:0x00aa, B:18:0x00c1, B:23:0x00dd, B:47:0x0150, B:49:0x0214, B:52:0x0219, B:53:0x021f, B:55:0x0222, B:57:0x0254, B:60:0x0274, B:59:0x0270, B:61:0x0277, B:63:0x02cb, B:64:0x02d3, B:66:0x02d9, B:68:0x02e7, B:70:0x02f8, B:73:0x02fe, B:75:0x0302, B:79:0x034d, B:83:0x036d, B:78:0x0334, B:62:0x029a, B:30:0x00f4, B:33:0x010d, B:36:0x0126, B:86:0x03d7), top: B:93:0x0012 }] */
    /* JADX WARN: Removed duplicated region for block: B:66:0x02d9 A[Catch: all -> 0x00da, TryCatch #0 {all -> 0x00da, blocks: (B:4:0x0012, B:5:0x003a, B:8:0x0042, B:10:0x0065, B:12:0x007c, B:14:0x0093, B:16:0x00aa, B:18:0x00c1, B:23:0x00dd, B:47:0x0150, B:49:0x0214, B:52:0x0219, B:53:0x021f, B:55:0x0222, B:57:0x0254, B:60:0x0274, B:59:0x0270, B:61:0x0277, B:63:0x02cb, B:64:0x02d3, B:66:0x02d9, B:68:0x02e7, B:70:0x02f8, B:73:0x02fe, B:75:0x0302, B:79:0x034d, B:83:0x036d, B:78:0x0334, B:62:0x029a, B:30:0x00f4, B:33:0x010d, B:36:0x0126, B:86:0x03d7), top: B:93:0x0012 }] */
    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWȏWWWoನ̑ */
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final boolean mo5039WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        int i10;
        int i11;
        byte[] hardwareAddress;
        File file;
        int i12;
        int i13;
        int i14 = 11;
        int i15 = 5;
        int i16 = 6;
        int i17 = 2;
        int i18 = 1;
        int i19 = 8;
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        if (vMConfig.f8895WWWoWWWo.f8847WWWWWWWW == 5) {
            try {
                String str = vMConfig.f8868WWWWWWWW;
                byte[] bArr = {2, -93, TarConstants.LF_SYMLINK, 94, 96, 45, -106, -118, 94, -93, 100, 67, 42, 58, -43};
                byte[] bArr2 = {45, -48, TarConstants.LF_GNUTYPE_LONGLINK, 45, 79, 78, -6, -21};
                StringFog.f8859WWWWWWWW.getClass();
                FileDeleteUtils.m5261WWWWoWWWWo(new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)));
                ArrayList arrayList = new ArrayList();
                Enumeration<NetworkInterface> networkInterfaces = NetworkInterface.getNetworkInterfaces();
                while (networkInterfaces.hasMoreElements()) {
                    NetworkInterface nextElement = networkInterfaces.nextElement();
                    String name = nextElement.getName();
                    byte[] bArr3 = new byte[i17];
                    // fill-array-data instruction
                    bArr3[0] = -74;
                    bArr3[1] = 108;
                    byte[] bArr4 = new byte[i19];
                    // fill-array-data instruction
                    bArr4[0] = -38;
                    bArr4[1] = 3;
                    bArr4[2] = 89;
                    bArr4[3] = 20;
                    bArr4[4] = -25;
                    bArr4[5] = 86;
                    bArr4[6] = 10;
                    bArr4[7] = -65;
                    WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
                    wwwwwwww.getClass();
                    if (!name.equals(WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4))) {
                        byte[] bArr5 = new byte[i15];
                        // fill-array-data instruction
                        bArr5[0] = 40;
                        bArr5[1] = 125;
                        bArr5[2] = -79;
                        bArr5[3] = 114;
                        bArr5[4] = -33;
                        byte[] bArr6 = new byte[i19];
                        // fill-array-data instruction
                        bArr6[0] = 95;
                        bArr6[1] = 17;
                        bArr6[2] = -48;
                        bArr6[3] = 28;
                        bArr6[4] = -17;
                        bArr6[5] = -110;
                        bArr6[6] = 59;
                        bArr6[7] = -96;
                        wwwwwwww.getClass();
                        if (!name.equals(WWWWWWWW.m17835WWWWWWWW(bArr5, bArr6))) {
                            byte[] bArr7 = new byte[i16];
                            // fill-array-data instruction
                            bArr7[0] = 86;
                            bArr7[1] = -8;
                            bArr7[2] = 51;
                            bArr7[3] = 29;
                            bArr7[4] = -61;
                            bArr7[5] = -59;
                            byte[] bArr8 = new byte[i19];
                            // fill-array-data instruction
                            bArr8[0] = 50;
                            bArr8[1] = -115;
                            bArr8[2] = 94;
                            bArr8[3] = 112;
                            bArr8[4] = -70;
                            bArr8[5] = -11;
                            bArr8[6] = -41;
                            bArr8[7] = 86;
                            wwwwwwww.getClass();
                            if (!name.equals(WWWWWWWW.m17835WWWWWWWW(bArr7, bArr8))) {
                                byte[] bArr9 = new byte[i14];
                                // fill-array-data instruction
                                bArr9[0] = 124;
                                bArr9[1] = -15;
                                bArr9[2] = 79;
                                bArr9[3] = 61;
                                bArr9[4] = 108;
                                bArr9[5] = 53;
                                bArr9[6] = -105;
                                bArr9[7] = -43;
                                bArr9[8] = 122;
                                bArr9[9] = -3;
                                bArr9[10] = 17;
                                byte[] bArr10 = new byte[i19];
                                // fill-array-data instruction
                                bArr10[0] = 14;
                                bArr10[1] = -100;
                                bArr10[2] = 33;
                                bArr10[3] = 88;
                                bArr10[4] = 24;
                                bArr10[5] = 106;
                                bArr10[6] = -13;
                                bArr10[7] = -76;
                                wwwwwwww.getClass();
                                if (!name.equals(WWWWWWWW.m17835WWWWWWWW(bArr9, bArr10))) {
                                    byte[] bArr11 = new byte[i14];
                                    // fill-array-data instruction
                                    bArr11[0] = -9;
                                    bArr11[1] = -33;
                                    bArr11[2] = 111;
                                    bArr11[3] = 18;
                                    bArr11[4] = -16;
                                    bArr11[5] = -42;
                                    bArr11[6] = -91;
                                    bArr11[7] = 123;
                                    bArr11[8] = -15;
                                    bArr11[9] = -45;
                                    bArr11[10] = 48;
                                    byte[] bArr12 = new byte[i19];
                                    // fill-array-data instruction
                                    bArr12[0] = -123;
                                    bArr12[1] = -78;
                                    bArr12[2] = 1;
                                    bArr12[3] = 119;
                                    bArr12[4] = -124;
                                    bArr12[5] = -119;
                                    bArr12[6] = -63;
                                    bArr12[7] = 26;
                                    wwwwwwww.getClass();
                                    if (!name.equals(WWWWWWWW.m17835WWWWWWWW(bArr11, bArr12))) {
                                        byte[] bArr13 = new byte[i14];
                                        // fill-array-data instruction
                                        bArr13[0] = 91;
                                        bArr13[1] = 46;
                                        bArr13[2] = -4;
                                        bArr13[3] = 68;
                                        bArr13[4] = 32;
                                        bArr13[5] = -23;
                                        bArr13[6] = 70;
                                        bArr13[7] = -9;
                                        bArr13[8] = 93;
                                        bArr13[9] = 34;
                                        bArr13[10] = -96;
                                        byte[] bArr14 = new byte[i19];
                                        // fill-array-data instruction
                                        bArr14[0] = 41;
                                        bArr14[1] = 67;
                                        bArr14[2] = -110;
                                        bArr14[3] = 33;
                                        bArr14[4] = 84;
                                        bArr14[5] = -74;
                                        bArr14[6] = 34;
                                        bArr14[7] = -106;
                                        wwwwwwww.getClass();
                                        if (!name.equals(WWWWWWWW.m17835WWWWWWWW(bArr13, bArr14))) {
                                        }
                                    }
                                }
                            }
                        }
                    }
                    int index = nextElement.getIndex();
                    int hashCode = name.hashCode();
                    if (hashCode != -1320644472) {
                        if (hashCode != 3459) {
                            if (hashCode == 113213102) {
                                byte[] bArr15 = new byte[i15];
                                // fill-array-data instruction
                                bArr15[0] = 100;
                                bArr15[1] = -3;
                                bArr15[2] = 37;
                                bArr15[3] = -4;
                                bArr15[4] = -123;
                                byte[] bArr16 = new byte[i19];
                                // fill-array-data instruction
                                bArr16[0] = 19;
                                bArr16[1] = -111;
                                bArr16[2] = 68;
                                bArr16[3] = -110;
                                bArr16[4] = -75;
                                bArr16[5] = 72;
                                bArr16[6] = 51;
                                bArr16[7] = -16;
                                wwwwwwww.getClass();
                                if (name.equals(WWWWWWWW.m17835WWWWWWWW(bArr15, bArr16))) {
                                    i10 = 1;
                                    if (i10 == 0) {
                                        if (i10 != i18) {
                                            if (i10 != i17) {
                                                i11 = 0;
                                            } else {
                                                i11 = TarConstants.PREFIXLEN_XSTAR;
                                            }
                                        } else {
                                            i11 = 4099;
                                        }
                                    } else {
                                        i11 = 9;
                                    }
                                    int mtu = nextElement.getMTU();
                                    hardwareAddress = nextElement.getHardwareAddress();
                                    String str2 = vMConfig.f8868WWWWWWWW;
                                    StringBuilder sb2 = new StringBuilder();
                                    byte[] bArr17 = new byte[i19];
                                    // fill-array-data instruction
                                    bArr17[0] = 41;
                                    bArr17[1] = 24;
                                    bArr17[2] = 65;
                                    bArr17[3] = -42;
                                    bArr17[4] = -41;
                                    bArr17[5] = -28;
                                    bArr17[6] = -114;
                                    bArr17[7] = -35;
                                    wwwwwwww.getClass();
                                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{6, 107, 56, -91, -8, -121, -30, -68, 90, 107, 110, -72, -78, -112, -95}, bArr17));
                                    sb2.append(name);
                                    file = new File(str2, sb2.toString());
                                    byte[] bArr18 = new byte[i19];
                                    // fill-array-data instruction
                                    bArr18[0] = 52;
                                    bArr18[1] = 59;
                                    bArr18[2] = -113;
                                    bArr18[3] = Byte.MAX_VALUE;
                                    bArr18[4] = -30;
                                    bArr18[5] = 30;
                                    bArr18[6] = 53;
                                    bArr18[7] = 21;
                                    byte[] bArr19 = new byte[i19];
                                    // fill-array-data instruction
                                    bArr19[0] = 27;
                                    bArr19[1] = 82;
                                    bArr19[2] = -23;
                                    bArr19[3] = 22;
                                    bArr19[4] = -116;
                                    bArr19[5] = 122;
                                    bArr19[6] = 80;
                                    bArr19[7] = 109;
                                    wwwwwwww.getClass();
                                    WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(bArr18, bArr19)), index + "\n", false);
                                    byte[] bArr20 = {-16, 15, -37, 23, TarConstants.LF_GNUTYPE_LONGNAME, 122};
                                    byte[] bArr21 = new byte[i19];
                                    // fill-array-data instruction
                                    bArr21[0] = -33;
                                    bArr21[1] = 105;
                                    bArr21[2] = -73;
                                    bArr21[3] = 118;
                                    bArr21[4] = 43;
                                    bArr21[5] = 9;
                                    bArr21[6] = -116;
                                    bArr21[7] = 122;
                                    wwwwwwww.getClass();
                                    File file2 = new File(file, WWWWWWWW.m17835WWWWWWWW(bArr20, bArr21));
                                    Locale locale = Locale.US;
                                    byte[] bArr22 = new byte[i19];
                                    // fill-array-data instruction
                                    bArr22[0] = 48;
                                    bArr22[1] = 126;
                                    bArr22[2] = -24;
                                    bArr22[3] = -60;
                                    bArr22[4] = 54;
                                    bArr22[5] = -10;
                                    bArr22[6] = 84;
                                    bArr22[7] = 43;
                                    wwwwwwww.getClass();
                                    WWWW.m5339WWWoWWWo(file2, String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{0, 6, -51, -68, 60}, bArr22), Integer.valueOf(i11)), false);
                                    byte[] bArr23 = {57, -120, 100, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -42, 71, -68, 69};
                                    wwwwwwww.getClass();
                                    WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(new byte[]{22, -27, 16, 18}, bArr23)), mtu + "\n", false);
                                    if (hardwareAddress != null && hardwareAddress.length != 0) {
                                        StringBuilder sb3 = new StringBuilder();
                                        for (i13 = 0; i13 < hardwareAddress.length; i13++) {
                                            Locale locale2 = Locale.US;
                                            WWWWWWWW wwwwwwww2 = StringFog.f8859WWWWWWWW;
                                            wwwwwwww2.getClass();
                                            sb3.append(String.format(locale2, WWWWWWWW.m17835WWWWWWWW(new byte[]{68, -20, 27, -82}, new byte[]{97, -36, 41, -42, Byte.MIN_VALUE, 98, -12, -79}), Byte.valueOf(hardwareAddress[i13])));
                                            if (i13 != hardwareAddress.length - 1) {
                                                byte[] bArr24 = {-16, -64, TarConstants.LF_MULTIVOLUME, 92, TarConstants.LF_CHR, -86, -62, -74};
                                                wwwwwwww2.getClass();
                                                sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-54}, bArr24));
                                            } else {
                                                sb3.append("\n");
                                            }
                                        }
                                        StringFog.f8859WWWWWWWW.getClass();
                                        WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(new byte[]{58, 20, -74, TarConstants.LF_NORMAL, -123, -43, -54, -20}, new byte[]{21, 117, -46, 84, -9, -80, -71, -97})), sb3.toString(), false);
                                        for (InterfaceAddress interfaceAddress : nextElement.getInterfaceAddresses()) {
                                            if (interfaceAddress.getAddress() instanceof Inet6Address) {
                                                byte[] address = ((Inet6Address) interfaceAddress.getAddress()).getAddress();
                                                StringBuilder sb4 = new StringBuilder();
                                                if (address != null && address.length != 0) {
                                                    for (byte b8 : address) {
                                                        Locale locale3 = Locale.US;
                                                        byte[] bArr25 = {-18, -35, -123, TarConstants.LF_SYMLINK, 62, -36, -67, 78};
                                                        StringFog.f8859WWWWWWWW.getClass();
                                                        sb4.append(String.format(locale3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-53, -19, -73, 74}, bArr25), Byte.valueOf(b8)));
                                                    }
                                                } else {
                                                    byte[] bArr26 = {37, -53, -82, 43, 17, TarConstants.LF_GNUTYPE_LONGLINK, -111, -95};
                                                    StringFog.f8859WWWWWWWW.getClass();
                                                    sb4.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{21, -5, -98, 27, 33, 123, -95, -111, 21, -5, -98, 27, 33, 123, -95, -111, 21, -5, -98, 27, 33, 123, -95, -111, 21, -5, -98, 27, 33, 123, -95, -111}, bArr26));
                                                }
                                                WWWWWWWW wwwwwwww3 = StringFog.f8859WWWWWWWW;
                                                wwwwwwww3.getClass();
                                                if (name.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{-101, -97}, new byte[]{-9, -16, -106, 95, -22, -18, 108, -30}))) {
                                                    i12 = 16;
                                                } else {
                                                    i12 = 32;
                                                }
                                                Locale locale4 = Locale.US;
                                                wwwwwwww3.getClass();
                                                arrayList.add(String.format(locale4, WWWWWWWW.m17835WWWWWWWW(new byte[]{-111, -38, 46, -101, 100, -57, -42, -85, -111, -103, 60, -58, 116, -48, -98, -71, -52, -119, 43, -114, 102, -115, -114, -82, -116, -38}, new byte[]{-76, -87, 14, -66, 84, -11, -82, -117}), sb4.toString(), Integer.valueOf(index), Short.valueOf(interfaceAddress.getNetworkPrefixLength()), Integer.valueOf(i12), 128, name));
                                            }
                                        }
                                        i14 = 11;
                                        i15 = 5;
                                        i16 = 6;
                                        i17 = 2;
                                        i18 = 1;
                                        i19 = 8;
                                    }
                                    byte[] bArr27 = {-81, 106, 35, ConstantPoolEntry.CP_InterfaceMethodref, -124, -58, 21, 45};
                                    byte[] bArr28 = {Byte.MIN_VALUE, ConstantPoolEntry.CP_InterfaceMethodref, 71, 111, -10, -93, 102, 94};
                                    wwwwwwww.getClass();
                                    File file3 = new File(file, WWWWWWWW.m17835WWWWWWWW(bArr27, bArr28));
                                    wwwwwwww.getClass();
                                    WWWW.m5339WWWoWWWo(file3, WWWWWWWW.m17835WWWWWWWW(new byte[]{68, 30, -97, 8, -97, 41, 116, Byte.MAX_VALUE, 78, 30, -107, 2, -97, 35, 126, Byte.MAX_VALUE, 68, 36}, new byte[]{116, 46, -91, 56, -81, 19, 68, 79}), false);
                                    while (r0.hasNext()) {
                                    }
                                    i14 = 11;
                                    i15 = 5;
                                    i16 = 6;
                                    i17 = 2;
                                    i18 = 1;
                                    i19 = 8;
                                }
                            }
                            i10 = -1;
                            if (i10 == 0) {
                            }
                            int mtu2 = nextElement.getMTU();
                            hardwareAddress = nextElement.getHardwareAddress();
                            String str22 = vMConfig.f8868WWWWWWWW;
                            StringBuilder sb22 = new StringBuilder();
                            byte[] bArr172 = new byte[i19];
                            // fill-array-data instruction
                            bArr172[0] = 41;
                            bArr172[1] = 24;
                            bArr172[2] = 65;
                            bArr172[3] = -42;
                            bArr172[4] = -41;
                            bArr172[5] = -28;
                            bArr172[6] = -114;
                            bArr172[7] = -35;
                            wwwwwwww.getClass();
                            sb22.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{6, 107, 56, -91, -8, -121, -30, -68, 90, 107, 110, -72, -78, -112, -95}, bArr172));
                            sb22.append(name);
                            file = new File(str22, sb22.toString());
                            byte[] bArr182 = new byte[i19];
                            // fill-array-data instruction
                            bArr182[0] = 52;
                            bArr182[1] = 59;
                            bArr182[2] = -113;
                            bArr182[3] = Byte.MAX_VALUE;
                            bArr182[4] = -30;
                            bArr182[5] = 30;
                            bArr182[6] = 53;
                            bArr182[7] = 21;
                            byte[] bArr192 = new byte[i19];
                            // fill-array-data instruction
                            bArr192[0] = 27;
                            bArr192[1] = 82;
                            bArr192[2] = -23;
                            bArr192[3] = 22;
                            bArr192[4] = -116;
                            bArr192[5] = 122;
                            bArr192[6] = 80;
                            bArr192[7] = 109;
                            wwwwwwww.getClass();
                            WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(bArr182, bArr192)), index + "\n", false);
                            byte[] bArr202 = {-16, 15, -37, 23, TarConstants.LF_GNUTYPE_LONGNAME, 122};
                            byte[] bArr212 = new byte[i19];
                            // fill-array-data instruction
                            bArr212[0] = -33;
                            bArr212[1] = 105;
                            bArr212[2] = -73;
                            bArr212[3] = 118;
                            bArr212[4] = 43;
                            bArr212[5] = 9;
                            bArr212[6] = -116;
                            bArr212[7] = 122;
                            wwwwwwww.getClass();
                            File file22 = new File(file, WWWWWWWW.m17835WWWWWWWW(bArr202, bArr212));
                            Locale locale5 = Locale.US;
                            byte[] bArr222 = new byte[i19];
                            // fill-array-data instruction
                            bArr222[0] = 48;
                            bArr222[1] = 126;
                            bArr222[2] = -24;
                            bArr222[3] = -60;
                            bArr222[4] = 54;
                            bArr222[5] = -10;
                            bArr222[6] = 84;
                            bArr222[7] = 43;
                            wwwwwwww.getClass();
                            WWWW.m5339WWWoWWWo(file22, String.format(locale5, WWWWWWWW.m17835WWWWWWWW(new byte[]{0, 6, -51, -68, 60}, bArr222), Integer.valueOf(i11)), false);
                            byte[] bArr232 = {57, -120, 100, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -42, 71, -68, 69};
                            wwwwwwww.getClass();
                            WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(new byte[]{22, -27, 16, 18}, bArr232)), mtu2 + "\n", false);
                            if (hardwareAddress != null) {
                                StringBuilder sb32 = new StringBuilder();
                                while (i13 < hardwareAddress.length) {
                                }
                                StringFog.f8859WWWWWWWW.getClass();
                                WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(new byte[]{58, 20, -74, TarConstants.LF_NORMAL, -123, -43, -54, -20}, new byte[]{21, 117, -46, 84, -9, -80, -71, -97})), sb32.toString(), false);
                                while (r0.hasNext()) {
                                }
                                i14 = 11;
                                i15 = 5;
                                i16 = 6;
                                i17 = 2;
                                i18 = 1;
                                i19 = 8;
                            }
                            byte[] bArr272 = {-81, 106, 35, ConstantPoolEntry.CP_InterfaceMethodref, -124, -58, 21, 45};
                            byte[] bArr282 = {Byte.MIN_VALUE, ConstantPoolEntry.CP_InterfaceMethodref, 71, 111, -10, -93, 102, 94};
                            wwwwwwww.getClass();
                            File file32 = new File(file, WWWWWWWW.m17835WWWWWWWW(bArr272, bArr282));
                            wwwwwwww.getClass();
                            WWWW.m5339WWWoWWWo(file32, WWWWWWWW.m17835WWWWWWWW(new byte[]{68, 30, -97, 8, -97, 41, 116, Byte.MAX_VALUE, 78, 30, -107, 2, -97, 35, 126, Byte.MAX_VALUE, 68, 36}, new byte[]{116, 46, -91, 56, -81, 19, 68, 79}), false);
                            while (r0.hasNext()) {
                            }
                            i14 = 11;
                            i15 = 5;
                            i16 = 6;
                            i17 = 2;
                            i18 = 1;
                            i19 = 8;
                        } else {
                            byte[] bArr29 = new byte[i17];
                            // fill-array-data instruction
                            bArr29[0] = 49;
                            bArr29[1] = -81;
                            byte[] bArr30 = new byte[i19];
                            // fill-array-data instruction
                            bArr30[0] = 93;
                            bArr30[1] = -64;
                            bArr30[2] = -28;
                            bArr30[3] = -108;
                            bArr30[4] = 20;
                            bArr30[5] = 102;
                            bArr30[6] = 49;
                            bArr30[7] = 120;
                            wwwwwwww.getClass();
                            if (name.equals(WWWWWWWW.m17835WWWWWWWW(bArr29, bArr30))) {
                                i10 = 0;
                                if (i10 == 0) {
                                }
                                int mtu22 = nextElement.getMTU();
                                hardwareAddress = nextElement.getHardwareAddress();
                                String str222 = vMConfig.f8868WWWWWWWW;
                                StringBuilder sb222 = new StringBuilder();
                                byte[] bArr1722 = new byte[i19];
                                // fill-array-data instruction
                                bArr1722[0] = 41;
                                bArr1722[1] = 24;
                                bArr1722[2] = 65;
                                bArr1722[3] = -42;
                                bArr1722[4] = -41;
                                bArr1722[5] = -28;
                                bArr1722[6] = -114;
                                bArr1722[7] = -35;
                                wwwwwwww.getClass();
                                sb222.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{6, 107, 56, -91, -8, -121, -30, -68, 90, 107, 110, -72, -78, -112, -95}, bArr1722));
                                sb222.append(name);
                                file = new File(str222, sb222.toString());
                                byte[] bArr1822 = new byte[i19];
                                // fill-array-data instruction
                                bArr1822[0] = 52;
                                bArr1822[1] = 59;
                                bArr1822[2] = -113;
                                bArr1822[3] = Byte.MAX_VALUE;
                                bArr1822[4] = -30;
                                bArr1822[5] = 30;
                                bArr1822[6] = 53;
                                bArr1822[7] = 21;
                                byte[] bArr1922 = new byte[i19];
                                // fill-array-data instruction
                                bArr1922[0] = 27;
                                bArr1922[1] = 82;
                                bArr1922[2] = -23;
                                bArr1922[3] = 22;
                                bArr1922[4] = -116;
                                bArr1922[5] = 122;
                                bArr1922[6] = 80;
                                bArr1922[7] = 109;
                                wwwwwwww.getClass();
                                WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(bArr1822, bArr1922)), index + "\n", false);
                                byte[] bArr2022 = {-16, 15, -37, 23, TarConstants.LF_GNUTYPE_LONGNAME, 122};
                                byte[] bArr2122 = new byte[i19];
                                // fill-array-data instruction
                                bArr2122[0] = -33;
                                bArr2122[1] = 105;
                                bArr2122[2] = -73;
                                bArr2122[3] = 118;
                                bArr2122[4] = 43;
                                bArr2122[5] = 9;
                                bArr2122[6] = -116;
                                bArr2122[7] = 122;
                                wwwwwwww.getClass();
                                File file222 = new File(file, WWWWWWWW.m17835WWWWWWWW(bArr2022, bArr2122));
                                Locale locale52 = Locale.US;
                                byte[] bArr2222 = new byte[i19];
                                // fill-array-data instruction
                                bArr2222[0] = 48;
                                bArr2222[1] = 126;
                                bArr2222[2] = -24;
                                bArr2222[3] = -60;
                                bArr2222[4] = 54;
                                bArr2222[5] = -10;
                                bArr2222[6] = 84;
                                bArr2222[7] = 43;
                                wwwwwwww.getClass();
                                WWWW.m5339WWWoWWWo(file222, String.format(locale52, WWWWWWWW.m17835WWWWWWWW(new byte[]{0, 6, -51, -68, 60}, bArr2222), Integer.valueOf(i11)), false);
                                byte[] bArr2322 = {57, -120, 100, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -42, 71, -68, 69};
                                wwwwwwww.getClass();
                                WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(new byte[]{22, -27, 16, 18}, bArr2322)), mtu22 + "\n", false);
                                if (hardwareAddress != null) {
                                }
                                byte[] bArr2722 = {-81, 106, 35, ConstantPoolEntry.CP_InterfaceMethodref, -124, -58, 21, 45};
                                byte[] bArr2822 = {Byte.MIN_VALUE, ConstantPoolEntry.CP_InterfaceMethodref, 71, 111, -10, -93, 102, 94};
                                wwwwwwww.getClass();
                                File file322 = new File(file, WWWWWWWW.m17835WWWWWWWW(bArr2722, bArr2822));
                                wwwwwwww.getClass();
                                WWWW.m5339WWWoWWWo(file322, WWWWWWWW.m17835WWWWWWWW(new byte[]{68, 30, -97, 8, -97, 41, 116, Byte.MAX_VALUE, 78, 30, -107, 2, -97, 35, 126, Byte.MAX_VALUE, 68, 36}, new byte[]{116, 46, -91, 56, -81, 19, 68, 79}), false);
                                while (r0.hasNext()) {
                                }
                                i14 = 11;
                                i15 = 5;
                                i16 = 6;
                                i17 = 2;
                                i18 = 1;
                                i19 = 8;
                            }
                            i10 = -1;
                            if (i10 == 0) {
                            }
                            int mtu222 = nextElement.getMTU();
                            hardwareAddress = nextElement.getHardwareAddress();
                            String str2222 = vMConfig.f8868WWWWWWWW;
                            StringBuilder sb2222 = new StringBuilder();
                            byte[] bArr17222 = new byte[i19];
                            // fill-array-data instruction
                            bArr17222[0] = 41;
                            bArr17222[1] = 24;
                            bArr17222[2] = 65;
                            bArr17222[3] = -42;
                            bArr17222[4] = -41;
                            bArr17222[5] = -28;
                            bArr17222[6] = -114;
                            bArr17222[7] = -35;
                            wwwwwwww.getClass();
                            sb2222.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{6, 107, 56, -91, -8, -121, -30, -68, 90, 107, 110, -72, -78, -112, -95}, bArr17222));
                            sb2222.append(name);
                            file = new File(str2222, sb2222.toString());
                            byte[] bArr18222 = new byte[i19];
                            // fill-array-data instruction
                            bArr18222[0] = 52;
                            bArr18222[1] = 59;
                            bArr18222[2] = -113;
                            bArr18222[3] = Byte.MAX_VALUE;
                            bArr18222[4] = -30;
                            bArr18222[5] = 30;
                            bArr18222[6] = 53;
                            bArr18222[7] = 21;
                            byte[] bArr19222 = new byte[i19];
                            // fill-array-data instruction
                            bArr19222[0] = 27;
                            bArr19222[1] = 82;
                            bArr19222[2] = -23;
                            bArr19222[3] = 22;
                            bArr19222[4] = -116;
                            bArr19222[5] = 122;
                            bArr19222[6] = 80;
                            bArr19222[7] = 109;
                            wwwwwwww.getClass();
                            WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(bArr18222, bArr19222)), index + "\n", false);
                            byte[] bArr20222 = {-16, 15, -37, 23, TarConstants.LF_GNUTYPE_LONGNAME, 122};
                            byte[] bArr21222 = new byte[i19];
                            // fill-array-data instruction
                            bArr21222[0] = -33;
                            bArr21222[1] = 105;
                            bArr21222[2] = -73;
                            bArr21222[3] = 118;
                            bArr21222[4] = 43;
                            bArr21222[5] = 9;
                            bArr21222[6] = -116;
                            bArr21222[7] = 122;
                            wwwwwwww.getClass();
                            File file2222 = new File(file, WWWWWWWW.m17835WWWWWWWW(bArr20222, bArr21222));
                            Locale locale522 = Locale.US;
                            byte[] bArr22222 = new byte[i19];
                            // fill-array-data instruction
                            bArr22222[0] = 48;
                            bArr22222[1] = 126;
                            bArr22222[2] = -24;
                            bArr22222[3] = -60;
                            bArr22222[4] = 54;
                            bArr22222[5] = -10;
                            bArr22222[6] = 84;
                            bArr22222[7] = 43;
                            wwwwwwww.getClass();
                            WWWW.m5339WWWoWWWo(file2222, String.format(locale522, WWWWWWWW.m17835WWWWWWWW(new byte[]{0, 6, -51, -68, 60}, bArr22222), Integer.valueOf(i11)), false);
                            byte[] bArr23222 = {57, -120, 100, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -42, 71, -68, 69};
                            wwwwwwww.getClass();
                            WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(new byte[]{22, -27, 16, 18}, bArr23222)), mtu222 + "\n", false);
                            if (hardwareAddress != null) {
                            }
                            byte[] bArr27222 = {-81, 106, 35, ConstantPoolEntry.CP_InterfaceMethodref, -124, -58, 21, 45};
                            byte[] bArr28222 = {Byte.MIN_VALUE, ConstantPoolEntry.CP_InterfaceMethodref, 71, 111, -10, -93, 102, 94};
                            wwwwwwww.getClass();
                            File file3222 = new File(file, WWWWWWWW.m17835WWWWWWWW(bArr27222, bArr28222));
                            wwwwwwww.getClass();
                            WWWW.m5339WWWoWWWo(file3222, WWWWWWWW.m17835WWWWWWWW(new byte[]{68, 30, -97, 8, -97, 41, 116, Byte.MAX_VALUE, 78, 30, -107, 2, -97, 35, 126, Byte.MAX_VALUE, 68, 36}, new byte[]{116, 46, -91, 56, -81, 19, 68, 79}), false);
                            while (r0.hasNext()) {
                            }
                            i14 = 11;
                            i15 = 5;
                            i16 = 6;
                            i17 = 2;
                            i18 = 1;
                            i19 = 8;
                        }
                    } else {
                        byte[] bArr31 = new byte[i16];
                        // fill-array-data instruction
                        bArr31[0] = -109;
                        bArr31[1] = -99;
                        bArr31[2] = -116;
                        bArr31[3] = 122;
                        bArr31[4] = -56;
                        bArr31[5] = -127;
                        byte[] bArr32 = new byte[i19];
                        // fill-array-data instruction
                        bArr32[0] = -9;
                        bArr32[1] = -24;
                        bArr32[2] = -31;
                        bArr32[3] = 23;
                        bArr32[4] = -79;
                        bArr32[5] = -79;
                        bArr32[6] = 60;
                        bArr32[7] = -114;
                        wwwwwwww.getClass();
                        if (name.equals(WWWWWWWW.m17835WWWWWWWW(bArr31, bArr32))) {
                            i10 = 2;
                            if (i10 == 0) {
                            }
                            int mtu2222 = nextElement.getMTU();
                            hardwareAddress = nextElement.getHardwareAddress();
                            String str22222 = vMConfig.f8868WWWWWWWW;
                            StringBuilder sb22222 = new StringBuilder();
                            byte[] bArr172222 = new byte[i19];
                            // fill-array-data instruction
                            bArr172222[0] = 41;
                            bArr172222[1] = 24;
                            bArr172222[2] = 65;
                            bArr172222[3] = -42;
                            bArr172222[4] = -41;
                            bArr172222[5] = -28;
                            bArr172222[6] = -114;
                            bArr172222[7] = -35;
                            wwwwwwww.getClass();
                            sb22222.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{6, 107, 56, -91, -8, -121, -30, -68, 90, 107, 110, -72, -78, -112, -95}, bArr172222));
                            sb22222.append(name);
                            file = new File(str22222, sb22222.toString());
                            byte[] bArr182222 = new byte[i19];
                            // fill-array-data instruction
                            bArr182222[0] = 52;
                            bArr182222[1] = 59;
                            bArr182222[2] = -113;
                            bArr182222[3] = Byte.MAX_VALUE;
                            bArr182222[4] = -30;
                            bArr182222[5] = 30;
                            bArr182222[6] = 53;
                            bArr182222[7] = 21;
                            byte[] bArr192222 = new byte[i19];
                            // fill-array-data instruction
                            bArr192222[0] = 27;
                            bArr192222[1] = 82;
                            bArr192222[2] = -23;
                            bArr192222[3] = 22;
                            bArr192222[4] = -116;
                            bArr192222[5] = 122;
                            bArr192222[6] = 80;
                            bArr192222[7] = 109;
                            wwwwwwww.getClass();
                            WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(bArr182222, bArr192222)), index + "\n", false);
                            byte[] bArr202222 = {-16, 15, -37, 23, TarConstants.LF_GNUTYPE_LONGNAME, 122};
                            byte[] bArr212222 = new byte[i19];
                            // fill-array-data instruction
                            bArr212222[0] = -33;
                            bArr212222[1] = 105;
                            bArr212222[2] = -73;
                            bArr212222[3] = 118;
                            bArr212222[4] = 43;
                            bArr212222[5] = 9;
                            bArr212222[6] = -116;
                            bArr212222[7] = 122;
                            wwwwwwww.getClass();
                            File file22222 = new File(file, WWWWWWWW.m17835WWWWWWWW(bArr202222, bArr212222));
                            Locale locale5222 = Locale.US;
                            byte[] bArr222222 = new byte[i19];
                            // fill-array-data instruction
                            bArr222222[0] = 48;
                            bArr222222[1] = 126;
                            bArr222222[2] = -24;
                            bArr222222[3] = -60;
                            bArr222222[4] = 54;
                            bArr222222[5] = -10;
                            bArr222222[6] = 84;
                            bArr222222[7] = 43;
                            wwwwwwww.getClass();
                            WWWW.m5339WWWoWWWo(file22222, String.format(locale5222, WWWWWWWW.m17835WWWWWWWW(new byte[]{0, 6, -51, -68, 60}, bArr222222), Integer.valueOf(i11)), false);
                            byte[] bArr232222 = {57, -120, 100, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -42, 71, -68, 69};
                            wwwwwwww.getClass();
                            WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(new byte[]{22, -27, 16, 18}, bArr232222)), mtu2222 + "\n", false);
                            if (hardwareAddress != null) {
                            }
                            byte[] bArr272222 = {-81, 106, 35, ConstantPoolEntry.CP_InterfaceMethodref, -124, -58, 21, 45};
                            byte[] bArr282222 = {Byte.MIN_VALUE, ConstantPoolEntry.CP_InterfaceMethodref, 71, 111, -10, -93, 102, 94};
                            wwwwwwww.getClass();
                            File file32222 = new File(file, WWWWWWWW.m17835WWWWWWWW(bArr272222, bArr282222));
                            wwwwwwww.getClass();
                            WWWW.m5339WWWoWWWo(file32222, WWWWWWWW.m17835WWWWWWWW(new byte[]{68, 30, -97, 8, -97, 41, 116, Byte.MAX_VALUE, 78, 30, -107, 2, -97, 35, 126, Byte.MAX_VALUE, 68, 36}, new byte[]{116, 46, -91, 56, -81, 19, 68, 79}), false);
                            while (r0.hasNext()) {
                            }
                            i14 = 11;
                            i15 = 5;
                            i16 = 6;
                            i17 = 2;
                            i18 = 1;
                            i19 = 8;
                        }
                        i10 = -1;
                        if (i10 == 0) {
                        }
                        int mtu22222 = nextElement.getMTU();
                        hardwareAddress = nextElement.getHardwareAddress();
                        String str222222 = vMConfig.f8868WWWWWWWW;
                        StringBuilder sb222222 = new StringBuilder();
                        byte[] bArr1722222 = new byte[i19];
                        // fill-array-data instruction
                        bArr1722222[0] = 41;
                        bArr1722222[1] = 24;
                        bArr1722222[2] = 65;
                        bArr1722222[3] = -42;
                        bArr1722222[4] = -41;
                        bArr1722222[5] = -28;
                        bArr1722222[6] = -114;
                        bArr1722222[7] = -35;
                        wwwwwwww.getClass();
                        sb222222.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{6, 107, 56, -91, -8, -121, -30, -68, 90, 107, 110, -72, -78, -112, -95}, bArr1722222));
                        sb222222.append(name);
                        file = new File(str222222, sb222222.toString());
                        byte[] bArr1822222 = new byte[i19];
                        // fill-array-data instruction
                        bArr1822222[0] = 52;
                        bArr1822222[1] = 59;
                        bArr1822222[2] = -113;
                        bArr1822222[3] = Byte.MAX_VALUE;
                        bArr1822222[4] = -30;
                        bArr1822222[5] = 30;
                        bArr1822222[6] = 53;
                        bArr1822222[7] = 21;
                        byte[] bArr1922222 = new byte[i19];
                        // fill-array-data instruction
                        bArr1922222[0] = 27;
                        bArr1922222[1] = 82;
                        bArr1922222[2] = -23;
                        bArr1922222[3] = 22;
                        bArr1922222[4] = -116;
                        bArr1922222[5] = 122;
                        bArr1922222[6] = 80;
                        bArr1922222[7] = 109;
                        wwwwwwww.getClass();
                        WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(bArr1822222, bArr1922222)), index + "\n", false);
                        byte[] bArr2022222 = {-16, 15, -37, 23, TarConstants.LF_GNUTYPE_LONGNAME, 122};
                        byte[] bArr2122222 = new byte[i19];
                        // fill-array-data instruction
                        bArr2122222[0] = -33;
                        bArr2122222[1] = 105;
                        bArr2122222[2] = -73;
                        bArr2122222[3] = 118;
                        bArr2122222[4] = 43;
                        bArr2122222[5] = 9;
                        bArr2122222[6] = -116;
                        bArr2122222[7] = 122;
                        wwwwwwww.getClass();
                        File file222222 = new File(file, WWWWWWWW.m17835WWWWWWWW(bArr2022222, bArr2122222));
                        Locale locale52222 = Locale.US;
                        byte[] bArr2222222 = new byte[i19];
                        // fill-array-data instruction
                        bArr2222222[0] = 48;
                        bArr2222222[1] = 126;
                        bArr2222222[2] = -24;
                        bArr2222222[3] = -60;
                        bArr2222222[4] = 54;
                        bArr2222222[5] = -10;
                        bArr2222222[6] = 84;
                        bArr2222222[7] = 43;
                        wwwwwwww.getClass();
                        WWWW.m5339WWWoWWWo(file222222, String.format(locale52222, WWWWWWWW.m17835WWWWWWWW(new byte[]{0, 6, -51, -68, 60}, bArr2222222), Integer.valueOf(i11)), false);
                        byte[] bArr2322222 = {57, -120, 100, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -42, 71, -68, 69};
                        wwwwwwww.getClass();
                        WWWW.m5339WWWoWWWo(new File(file, WWWWWWWW.m17835WWWWWWWW(new byte[]{22, -27, 16, 18}, bArr2322222)), mtu22222 + "\n", false);
                        if (hardwareAddress != null) {
                        }
                        byte[] bArr2722222 = {-81, 106, 35, ConstantPoolEntry.CP_InterfaceMethodref, -124, -58, 21, 45};
                        byte[] bArr2822222 = {Byte.MIN_VALUE, ConstantPoolEntry.CP_InterfaceMethodref, 71, 111, -10, -93, 102, 94};
                        wwwwwwww.getClass();
                        File file322222 = new File(file, WWWWWWWW.m17835WWWWWWWW(bArr2722222, bArr2822222));
                        wwwwwwww.getClass();
                        WWWW.m5339WWWoWWWo(file322222, WWWWWWWW.m17835WWWWWWWW(new byte[]{68, 30, -97, 8, -97, 41, 116, Byte.MAX_VALUE, 78, 30, -107, 2, -97, 35, 126, Byte.MAX_VALUE, 68, 36}, new byte[]{116, 46, -91, 56, -81, 19, 68, 79}), false);
                        while (r0.hasNext()) {
                        }
                        i14 = 11;
                        i15 = 5;
                        i16 = 6;
                        i17 = 2;
                        i18 = 1;
                        i19 = 8;
                    }
                }
                String str3 = vMConfig.f8868WWWWWWWW;
                byte[] bArr33 = {84, -90, -109, TarConstants.LF_FIFO, 9, -42, -102, -116};
                StringFog.f8859WWWWWWWW.getClass();
                WWWW.m5339WWWoWWWo(new File(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{123, -42, -31, 89, 106, -7, -12, -23, 32, -119, -6, 80, 86, -65, -12, -23, 32, -112}, bArr33)), TextUtils.join("\n", arrayList) + "\n", false);
                return true;
            } catch (Throwable th2) {
                this.f9260WWWWWWWW = Log.getStackTraceString(th2);
                return false;
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
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{105, 32, -24, -68, 56, TarConstants.LF_CHR, 68, 92, 74, 38, -28}, new byte[]{43, 85, -113, -119, 126, 90, 60, 8});
    }
}
