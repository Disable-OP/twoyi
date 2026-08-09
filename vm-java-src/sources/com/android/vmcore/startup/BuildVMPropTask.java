package com.android.vmcore.startup;

import android.os.Build;
import android.system.ErrnoException;
import android.system.Os;
import android.text.TextUtils;
import android.util.AtomicFile;
import android.util.Log;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmapp.VMApp;
import com.android.vmcore.IVMStartupTask;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.VMResConfig;
import com.blankj.utilcode.util.WWWW;
import com.blankj.utilcode.util.WWWWoWWWWo;
import com.google.android.gms.internal.ads.pr0;
import java.io.Closeable;
import java.io.File;
import java.io.FileOutputStream;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Iterator;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.archivers.zip.UnixStat;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p053WWoWWo.AbstractC0470WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class BuildVMPropTask implements IVMStartupTask {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public int f9267WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f9268WWWWWWWW;

    /* loaded from: classes.dex */
    public interface PropLineCallback {
        /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
        String mo5217WWWWWWWW(String str);
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static String m5208WWWWWWWW(VMInstance vMInstance, String str) {
        if (!vMInstance.f8937WWWoWWWo.f8886WWWWWWWW) {
            byte[] bArr = {4, TarConstants.LF_CHR, -77, -113, 10, -66, -75, 102};
            StringFog.f8859WWWWWWWW.getClass();
            if (str.endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{62, 84, -61}, bArr))) {
                return str.substring(0, str.length() - 3);
            }
            try {
                String[] split = str.split(WWWWWWWW.m17835WWWWWWWW(new byte[]{-100}, new byte[]{-90, -46, 87, -23, 84, -23, 109, -88}));
                if (split[1].endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-69, -16, -57}, new byte[]{-108, -105, -73, 118, -127, -57, 38, 63}))) {
                    return split[0] + WWWWWWWW.m17835WWWWWWWW(new byte[]{3}, new byte[]{57, -59, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -97, 123, 45, 92, 114}) + split[1].substring(0, split[1].length() - 3) + WWWWWWWW.m17835WWWWWWWW(new byte[]{74}, new byte[]{112, 3, 36, 21, 42, TarConstants.LF_GNUTYPE_LONGLINK, 0, 126}) + split[2];
                }
            } catch (Throwable unused) {
            }
            return str;
        }
        StringFog.f8859WWWWWWWW.getClass();
        if (!str.endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{32, TarConstants.LF_DIR, -71}, new byte[]{26, 82, -55, -3, 110, 106, -118, -45}))) {
            try {
                String[] split2 = str.split(WWWWWWWW.m17835WWWWWWWW(new byte[]{-90}, new byte[]{-100, 10, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -29, -122, -70, -29, 118}));
                if (!split2[1].endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, -34, 24}, new byte[]{99, -71, 104, -50, -118, -96, TarConstants.LF_NORMAL, 30}))) {
                    return split2[0] + WWWWWWWW.m17835WWWWWWWW(new byte[]{36}, new byte[]{30, -64, 30, -17, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -62, 89, -118}) + split2[1] + WWWWWWWW.m17835WWWWWWWW(new byte[]{45, -14, -3, -42}, new byte[]{2, -107, -115, -20, -10, -15, TarConstants.LF_SYMLINK, -16}) + split2[2];
                }
            } catch (Throwable unused2) {
                byte[] bArr2 = {-31, -42, -97, -3, -1, 107, -79, TarConstants.LF_DIR};
                StringFog.f8859WWWWWWWW.getClass();
                return str.concat(WWWWWWWW.m17835WWWWWWWW(new byte[]{-37, -79, -17}, bArr2));
            }
        }
        return str;
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public static void m5209WWWWWWWW(VMInstance vMInstance) {
        File file;
        int indexOf;
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        if (vMConfig.f8895WWWoWWWo.f8847WWWWWWWW == 11) {
            String str = vMConfig.f8868WWWWWWWW;
            StringFog.f8859WWWWWWWW.getClass();
            file = new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_SYMLINK, -91, -88, -77, -102, -107, 114, 19, 109, -92, -66, -92, -101, -109, 107, 19, Byte.MAX_VALUE, -93, -72, -84, -118, -34, 111, 78, 114, -90}, new byte[]{29, -42, -47, -64, -18, -16, 31, 60}));
        } else {
            String str2 = vMConfig.f8868WWWWWWWW;
            StringFog.f8859WWWWWWWW.getClass();
            file = new File(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-9, TarConstants.LF_DIR, -27, -84, 47, -113, -52, -106, -70, TarConstants.LF_CHR, -11, -77, 63, -60, -47, -53, -73, TarConstants.LF_FIFO}, new byte[]{-40, 70, -100, -33, 91, -22, -95, -71}));
        }
        ArrayList m5320WWWWoWWWWo = WWWW.m5320WWWWoWWWWo(file);
        if (m5320WWWWoWWWWo != null) {
            HashSet hashSet = new HashSet();
            byte[] bArr = {-72, -90, 89, 2, -62, -94, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 4};
            StringFog.f8859WWWWWWWW.getClass();
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{-54, -55, 119, 114, -80, -51, 60, 113, -37, -46, 119, 111, -83, -58, 61, 104}, bArr));
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{-17, -86, -20, 115, TarConstants.LF_GNUTYPE_SPARSE, -105, 104, -66, -2, -79, -20, 97, TarConstants.LF_GNUTYPE_SPARSE, -103, 98, -81}, new byte[]{-99, -59, -62, 3, 33, -8, ConstantPoolEntry.CP_NameAndType, -53}));
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{-115, 2, -94, -1, 108, -122, -36, -65, -100, 25, -94, -31, Byte.MAX_VALUE, -124, -35}, new byte[]{-1, 109, -116, -113, 30, -23, -72, -54}));
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_FIFO, -66, 16, -23, -93, -95, -65, -38, 39, -91, 16, -3, -76, -72, -78, -52, 33}, new byte[]{68, -47, 62, -103, -47, -50, -37, -81}));
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{29, -66, -4, -119, 59, 61, 33, 36, ConstantPoolEntry.CP_NameAndType, -91, -4, -108, 40, 60, TarConstants.LF_NORMAL, TarConstants.LF_CONTIG, 14, -78, -90, -116, 59, TarConstants.LF_CONTIG, TarConstants.LF_CONTIG}, new byte[]{111, -47, -46, -7, 73, 82, 69, 81}));
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, -17, -39, -57, 10, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -17, 87, -92, -12, -39, -57, 10, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -17, 87, -92, -12, -39, -38, 23, 115, -18, 78}, new byte[]{-57, Byte.MIN_VALUE, -9, -73, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 23, -117, 34}));
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{81, -49, 86, -55, -102, -22, -15, -48, 64, -44, 86, -55, -102, -22, -15, -48, 64, -44, 86, -37, -102, -28, -5, -63}, new byte[]{35, -96, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -71, -24, -123, -107, -91}));
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{-14, -120, 9, -84, 124, 121, -51, 111, -29, -109, 9, -84, 124, 121, -51, 111, -29, -109, 9, -78, 111, 123, -52}, new byte[]{Byte.MIN_VALUE, -25, 39, -36, 14, 22, -87, 26}));
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{47, -113, 68, -118, 63, -28, -65, -16, 62, -108, 68, -118, 63, -28, -65, -16, 62, -108, 68, -98, 40, -3, -78, -26, 56}, new byte[]{93, -32, 106, -6, TarConstants.LF_MULTIVOLUME, -117, -37, -123}));
            hashSet.add(WWWWWWWW.m17835WWWWWWWW(new byte[]{117, -117, -89, 112, ConstantPoolEntry.CP_InterfaceMethodref, -15, 6, 14, 100, -112, -89, 112, ConstantPoolEntry.CP_InterfaceMethodref, -15, 6, 14, 100, -112, -89, 109, 24, -16, 23, 29, 102, -121, -3, 117, ConstantPoolEntry.CP_InterfaceMethodref, -5, 16}, new byte[]{7, -28, -119, 0, 121, -98, 98, 123}));
            int size = m5320WWWWoWWWWo.size();
            int i10 = 0;
            while (i10 < size) {
                Object obj = m5320WWWWoWWWWo.get(i10);
                i10++;
                String str3 = (String) obj;
                if (!TextUtils.isEmpty(str3) && !AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{116}, new byte[]{87, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 13, 21, -34, 84, TarConstants.LF_GNUTYPE_LONGLINK, TarConstants.LF_GNUTYPE_LONGNAME}, str3) && (indexOf = str3.indexOf(WWWWWWWW.m17835WWWWWWWW(new byte[]{-72}, new byte[]{-123, -117, 94, Byte.MIN_VALUE, -72, 119, -21, -67}))) > 0) {
                    String substring = str3.substring(0, indexOf);
                    String substring2 = str3.substring(indexOf + 1);
                    if (hashSet.contains(substring)) {
                        if (substring.contains(WWWWWWWW.m17835WWWWWWWW(new byte[]{60, -62, 24, 92, 99, 43, 15, 37, 60}, new byte[]{18, -78, 106, TarConstants.LF_CHR, 7, 94, 108, 81}))) {
                            vMInstance.m5083WWWWWW(substring.replace(WWWWWWWW.m17835WWWWWWWW(new byte[]{56, -88, 37, 67, 3, -53, 119, -112, 56}, new byte[]{22, -40, 87, 44, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -66, 20, -28}), WWWWWWWW.m17835WWWWWWWW(new byte[]{60}, new byte[]{18, 47, TarConstants.LF_GNUTYPE_LONGNAME, 20, 16, -7, 33, 17})), substring2, true);
                        } else {
                            vMInstance.m5083WWWWWW(substring, substring2, true);
                        }
                    }
                }
            }
        }
    }

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public static void m5210WWWWWWWW(VMInstance vMInstance) {
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        VMResConfig m5061WWWWWWWW = vMInstance.m5061WWWWWWWW();
        String str = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {99, 30, 21, 101, ConstantPoolEntry.CP_NameAndType, 82, 41, 7, 46, 29, 25, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, ConstantPoolEntry.CP_NameAndType, 19, 43, 90, 35, 24};
        byte[] bArr2 = {TarConstants.LF_GNUTYPE_LONGNAME, 104, 112, ConstantPoolEntry.CP_InterfaceMethodref, 104, 61, 91, 40};
        WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
        wwwwwwww.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        if (!file.exists()) {
            WWWWoWWWWo.m5285WWWWWWWW(file);
        }
        HashMap hashMap = new HashMap();
        byte[] bArr3 = {-15, -84, -26, 92, TarConstants.LF_GNUTYPE_LONGNAME, Byte.MIN_VALUE, -116, 117};
        wwwwwwww.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-125, -61, -56, 47, 42, -82, -32, 22, -107, -13, -126, 57, 34, -13, -27, 1, -120}, bArr3);
        StringBuilder sb2 = new StringBuilder();
        pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{78, TarConstants.LF_GNUTYPE_LONGLINK, 74, -33, 44, -21, 26, 105, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 123, 0, -55, 36, -74, 31, 126, 69, 25}, new byte[]{60, 36, 100, -84, 74, -59, 118, 10}, sb2);
        sb2.append(m5061WWWWWWWW.f8954WWWWWWWW);
        hashMap.put(m17835WWWWWWWW, sb2.toString());
        byte[] bArr4 = {6, 92, -45, -83, 24, -19, -121, TarConstants.LF_CONTIG, 16, 108, -118, -73, 26, -73, -125};
        byte[] bArr5 = {116, TarConstants.LF_CHR, -3, -34, 126, -61, -21, 84};
        wwwwwwww.getClass();
        String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5);
        StringBuilder sb3 = new StringBuilder();
        byte[] bArr6 = {TarConstants.LF_FIFO, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -71, -61, -121, -78, -103, -111};
        wwwwwwww.getClass();
        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{68, 23, -105, -80, -31, -100, -11, -14, 82, 39, -50, -86, -29, -58, -15, -84}, bArr6));
        sb3.append(m5061WWWWWWWW.f8952WWWWoWWWWo);
        hashMap.put(m17835WWWWWWWW2, sb3.toString());
        byte[] bArr7 = {94, -42, -1, -47, TarConstants.LF_FIFO, 110, -8, -60};
        wwwwwwww.getClass();
        String m17835WWWWWWWW3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{44, -71, -47, -94, 80, 64, -108, -89, 58, -119, -105, -76, 95, 9, -112, -80}, bArr7);
        StringBuilder sb4 = new StringBuilder();
        pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{TarConstants.LF_CHR, TarConstants.LF_BLK, 20, -39, -86, -35, -3, -63, 37, 4, 82, -49, -91, -108, -7, -42, 124}, new byte[]{65, 91, 58, -86, -52, -13, -111, -94}, sb4);
        sb4.append(m5061WWWWWWWW.f8955WWWoWWWo);
        hashMap.put(m17835WWWWWWWW3, sb4.toString());
        wwwwwwww.getClass();
        String m17835WWWWWWWW4 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -107, -115, -125, 15, -105, 121, 123, -122, -91, -59, Byte.MIN_VALUE, 26}, new byte[]{-30, -6, -93, -16, 105, -71, 21, 24});
        StringBuilder sb5 = new StringBuilder();
        byte[] bArr8 = {21, -57, -48, -125, 9, TarConstants.LF_DIR, 84, TarConstants.LF_CHR, 3, -9, -104, Byte.MIN_VALUE, 28, 38};
        byte[] bArr9 = {TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -88, -2, -16, 111, 27, 56, 80};
        wwwwwwww.getClass();
        sb5.append(WWWWWWWW.m17835WWWWWWWW(bArr8, bArr9));
        sb5.append(vMConfig.f8918WW);
        hashMap.put(m17835WWWWWWWW4, sb5.toString());
        byte[] bArr10 = {-35, -91, -28, 14, -4, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -111, 89};
        wwwwwwww.getClass();
        String m17835WWWWWWWW5 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-81, -54, -54, 105, -116, 45, -65, 47, -72, -53, Byte.MIN_VALUE, 97, -114}, bArr10);
        StringBuilder sb6 = new StringBuilder();
        pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{ConstantPoolEntry.CP_NameAndType, 101, -23, 9, -47, -38, TarConstants.LF_DIR, -15, 27, 100, -93, 1, -45, -110}, new byte[]{126, 10, -57, 110, -95, -81, 27, -121}, sb6);
        sb6.append(vMConfig.f8892WWWWWWWW);
        hashMap.put(m17835WWWWWWWW5, sb6.toString());
        byte[] bArr11 = {-36, -53, 42, -77, -93, ConstantPoolEntry.CP_NameAndType, -56, -82};
        wwwwwwww.getClass();
        String m17835WWWWWWWW6 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-82, -92, 4, -44, -45, 121, -26, -36, -71, -91, 78, -42, -47, 105, -70}, bArr11);
        StringBuilder sb7 = new StringBuilder();
        wwwwwwww.getClass();
        sb7.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType, -110, -87, 125, TarConstants.LF_BLK, 102, -81, 78, 27, -109, -29, Byte.MAX_VALUE, TarConstants.LF_FIFO, 118, -13, 1}, new byte[]{126, -3, -121, 26, 68, 19, -127, 60}));
        sb7.append(vMConfig.f8893WWWWWWWW);
        hashMap.put(m17835WWWWWWWW6, sb7.toString());
        if (vMConfig.f8887WWWWWWWW) {
            byte[] bArr12 = {-23, -54, 3, TarConstants.LF_CONTIG, -28, 69, 24, 8};
            wwwwwwww.getClass();
            String m17835WWWWWWWW7 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-104, -81, 110, 66, -54, 45, 111, 38, -124, -85, 106, 89, -113, 32, 97, 123}, bArr12);
            wwwwwwww.getClass();
            hashMap.put(m17835WWWWWWWW7, WWWWWWWW.m17835WWWWWWWW(new byte[]{-42, -69, -104, 85, 62, -90, -95, -101, -54, -65, -100, 78, 123, -85, -81, -58, -102, -18}, new byte[]{-89, -34, -11, 32, 16, -50, -42, -75}));
        } else {
            wwwwwwww.getClass();
            String m17835WWWWWWWW8 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-63, 9, -104, 6, -76, -32, 117, 100, -35, 13, -100, 29, -15, -19, 123, 57}, new byte[]{-80, 108, -11, 115, -102, -120, 2, 74});
            wwwwwwww.getClass();
            hashMap.put(m17835WWWWWWWW8, WWWWWWWW.m17835WWWWWWWW(new byte[]{111, 20, 24, -8, 66, 78, 84, -49, 115, 16, 28, -29, 7, 67, 90, -110, 35, 64}, new byte[]{30, 113, 117, -115, 108, 38, 35, -31}));
        }
        if (vMConfig.f8912WWoWWo) {
            wwwwwwww.getClass();
            String m17835WWWWWWWW9 = WWWWWWWW.m17835WWWWWWWW(new byte[]{94, -45, TarConstants.LF_GNUTYPE_LONGLINK, -67, 4, 105, -10, -69, 90, -29, 7, -87, 31, 105, -22, -82, 64}, new byte[]{44, -68, 101, -56, 109, 71, -104, -38});
            byte[] bArr13 = {-47, -87, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_GNUTYPE_LONGNAME, -17, 36, 95, -79, -43, -103, 64, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -12, 36, 67, -92, -49, -5, 19};
            byte[] bArr14 = {-93, -58, 34, 57, -122, 10, TarConstants.LF_LINK, -48};
            wwwwwwww.getClass();
            hashMap.put(m17835WWWWWWWW9, WWWWWWWW.m17835WWWWWWWW(bArr13, bArr14));
        } else {
            wwwwwwww.getClass();
            String m17835WWWWWWWW10 = WWWWWWWW.m17835WWWWWWWW(new byte[]{44, -59, -10, TarConstants.LF_GNUTYPE_LONGNAME, 73, -13, 44, Byte.MAX_VALUE, 40, -11, -70, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 82, -13, TarConstants.LF_NORMAL, 106, TarConstants.LF_SYMLINK}, new byte[]{94, -86, -40, 57, 32, -35, 66, 30});
            wwwwwwww.getClass();
            hashMap.put(m17835WWWWWWWW10, WWWWWWWW.m17835WWWWWWWW(new byte[]{107, -36, -72, 10, ConstantPoolEntry.CP_InterfaceMethodref, -94, 81, 121, 111, -20, -12, 30, 16, -94, TarConstants.LF_MULTIVOLUME, 108, 117, -114, -90}, new byte[]{25, -77, -106, Byte.MAX_VALUE, 98, -116, 63, 24}));
        }
        byte[] bArr15 = {87, 91, -25, 106, 102, -103, 6, -53, 74, 87, -94, 119, TarConstants.LF_FIFO, -78, TarConstants.LF_MULTIVOLUME, -42, 68, 86, -91, 97, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER};
        byte[] bArr16 = {37, TarConstants.LF_BLK, -55, 4, 3, -19, 40, -72};
        wwwwwwww.getClass();
        String m17835WWWWWWWW11 = WWWWWWWW.m17835WWWWWWWW(bArr15, bArr16);
        StringBuilder sb8 = new StringBuilder();
        pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{21, 91, -35, -2, 79, -83, 40, -100, 8, 87, -104, -29, 31, -122, 99, -127, 6, 86, -97, -11, 78, -28}, new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_BLK, -13, -112, 42, -39, 6, -17}, sb8);
        sb8.append(vMConfig.f8863WWWWoWWWWo ? 1 : 0);
        hashMap.put(m17835WWWWWWWW11, sb8.toString());
        wwwwwwww.getClass();
        String m17835WWWWWWWW12 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-51, -109, -107, 30, 71, -54, -12, 14, -48, -97, -48, 3, 23, -31, -87, 24, -51, -118, -34, 2}, new byte[]{-65, -4, -69, 112, 34, -66, -38, 125});
        StringBuilder sb9 = new StringBuilder();
        pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{42, TarConstants.LF_LINK, -105, TarConstants.LF_BLK, -92, -71, -29, TarConstants.LF_DIR, TarConstants.LF_CONTIG, 61, -46, 41, -12, -110, -66, 35, 42, 40, -36, 40, -4}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_UC, 94, -71, 90, -63, -51, -51, 70}, sb9);
        sb9.append(vMConfig.f8881WWWWWWWW);
        hashMap.put(m17835WWWWWWWW12, sb9.toString());
        wwwwwwww.getClass();
        String m17835WWWWWWWW13 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-51, -34, -69, -123, -61, -84, TarConstants.LF_CHR, -127, -48, -46, -2, -104, -109, -121, 109, -99, -51, -59}, new byte[]{-65, -79, -107, -21, -90, -40, 29, -14});
        StringBuilder sb10 = new StringBuilder();
        wwwwwwww.getClass();
        sb10.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{61, -80, -39, -117, 102, -112, 72, 4, 32, -68, -100, -106, TarConstants.LF_FIFO, -69, 22, 24, 61, -85, -54}, new byte[]{79, -33, -9, -27, 3, -28, 102, 119}));
        sb10.append(vMConfig.f8882WWWWWWWW);
        hashMap.put(m17835WWWWWWWW13, sb10.toString());
        byte[] bArr17 = {-58, 31, -65, 7, 45, Byte.MAX_VALUE, -33, TarConstants.LF_GNUTYPE_SPARSE, -37, 19, -6, 26, 125, 84, -124, TarConstants.LF_GNUTYPE_SPARSE, -47, 2, -1, 8, 37, 110};
        byte[] bArr18 = {-76, 112, -111, 105, 72, ConstantPoolEntry.CP_InterfaceMethodref, -15, 32};
        wwwwwwww.getClass();
        String m17835WWWWWWWW14 = WWWWWWWW.m17835WWWWWWWW(bArr17, bArr18);
        StringBuilder sb11 = new StringBuilder();
        pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{-50, 91, -46, 34, -61, -62, ConstantPoolEntry.CP_NameAndType, -91, -45, 87, -105, 63, -109, -23, 87, -91, -39, 70, -110, 45, -53, -45, 31}, new byte[]{-68, TarConstants.LF_BLK, -4, TarConstants.LF_GNUTYPE_LONGNAME, -90, -74, 34, -42}, sb11);
        sb11.append(vMConfig.f8883WWWWWWWW);
        hashMap.put(m17835WWWWWWWW14, sb11.toString());
        byte[] bArr19 = {TarConstants.LF_GNUTYPE_SPARSE, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -80, -3, 33, TarConstants.LF_CHR, -40, 17, 78, 84, -11, -32, 113, 24, -122, 3, 82, 68, -23, -4, TarConstants.LF_FIFO, 35};
        byte[] bArr20 = {33, TarConstants.LF_CONTIG, -98, -109, 68, 71, -10, 98};
        wwwwwwww.getClass();
        String m17835WWWWWWWW15 = WWWWWWWW.m17835WWWWWWWW(bArr19, bArr20);
        StringBuilder sb12 = new StringBuilder();
        pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{93, 108, -17, -41, 119, 24, -40, -11, 64, 96, -86, -54, 39, TarConstants.LF_CHR, -122, -25, 92, 112, -74, -42, 96, 8, -53}, new byte[]{47, 3, -63, -71, 18, 108, -10, -122}, sb12);
        sb12.append(vMConfig.f8917WWWW);
        hashMap.put(m17835WWWWWWWW15, sb12.toString());
        byte[] bArr21 = {67, 42, 95, -111, 112, -21, TarConstants.LF_SYMLINK, -44, 82, 43, 79, -52, 109, -5, TarConstants.LF_FIFO, -44, 67, 32, 95, -106};
        byte[] bArr22 = {TarConstants.LF_CHR, 79, 45, -30, 25, -104, 70, -6};
        wwwwwwww.getClass();
        String m17835WWWWWWWW16 = WWWWWWWW.m17835WWWWWWWW(bArr21, bArr22);
        StringBuilder sb13 = new StringBuilder();
        wwwwwwww.getClass();
        sb13.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-73, -111, -44, -45, 115, 106, -29, -39, -90, -112, -60, -114, 110, 122, -25, -39, -73, -101, -44, -44, 39}, new byte[]{-57, -12, -90, -96, 26, 25, -105, -9}));
        sb13.append(vMConfig.f8884WWWWWWWW);
        hashMap.put(m17835WWWWWWWW16, sb13.toString());
        if (vMConfig.f8910WWoWWo) {
            wwwwwwww.getClass();
            String m17835WWWWWWWW17 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-124, -48, 56, -112, 18, TarConstants.LF_GNUTYPE_LONGNAME, -41, -24, -121, -52, 57, -51, 14, TarConstants.LF_GNUTYPE_LONGNAME, -63, -24, -105, -38, 36, -123, 18, TarConstants.LF_PAX_EXTENDED_HEADER_UC}, new byte[]{-12, -75, 74, -29, 123, 63, -93, -58});
            byte[] bArr23 = {34, 84, -45, 93, -117, 21, -11, -52, 33, 72, -46, 0, -105, 21, -29, -52, TarConstants.LF_LINK, 94, -49, 72, -117, 1, -68, -125, TarConstants.LF_FIFO, TarConstants.LF_GNUTYPE_SPARSE};
            byte[] bArr24 = {82, TarConstants.LF_LINK, -95, 46, -30, 102, -127, -30};
            wwwwwwww.getClass();
            hashMap.put(m17835WWWWWWWW17, WWWWWWWW.m17835WWWWWWWW(bArr23, bArr24));
        } else {
            byte[] bArr25 = {126, -74, -8, 94, -5, 125, 64, -61, 125, -86, -7, 3, -25, 125, 86, -61, 109, -68, -28, TarConstants.LF_GNUTYPE_LONGLINK, -5, 105};
            byte[] bArr26 = {14, -45, -118, 45, -110, 14, TarConstants.LF_BLK, -19};
            wwwwwwww.getClass();
            String m17835WWWWWWWW18 = WWWWWWWW.m17835WWWWWWWW(bArr25, bArr26);
            wwwwwwww.getClass();
            hashMap.put(m17835WWWWWWWW18, WWWWWWWW.m17835WWWWWWWW(new byte[]{-76, 64, -127, 19, -40, -71, 125, 114, -73, 92, Byte.MIN_VALUE, 78, -60, -71, 107, 114, -89, 74, -99, 6, -40, -83, TarConstants.LF_BLK, TarConstants.LF_SYMLINK, -85, TarConstants.LF_GNUTYPE_LONGLINK, -106}, new byte[]{-60, 37, -13, 96, -79, -54, 9, 92}));
        }
        m5212WWoWWo(file, hashMap, null);
    }

    /* renamed from: WWWoԻWWWoͷԻ  reason: contains not printable characters */
    public static void m5211WWWoWWWo(VMInstance vMInstance) {
        String[] split;
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        String str = vMConfig.f8868WWWWWWWW;
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-107, -121, -108, -26, -63, -5, -86, -113, -34, -101, -113, -25, -57}, new byte[]{-70, -9, -26, -119, -94, -44, -55, -30}));
        String m5318WWWWWWWWWW = WWWW.m5318WWWWWWWWWW(file);
        if (!TextUtils.isEmpty(m5318WWWWWWWWWW)) {
            StringBuilder sb2 = new StringBuilder();
            boolean z10 = false;
            for (String str2 : m5318WWWWWWWWWW.split(" ")) {
                if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-114, 4, -94, 16, -45, 118, -1, 44, Byte.MIN_VALUE, 5, -78, TarConstants.LF_GNUTYPE_LONGNAME, -49, 122, -23, 39, -114, 6, -88, 13, -127}, new byte[]{-17, 106, -58, 98, -68, 31, -101, 78}, str2)) {
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-93, 1, 70, 95, 62, -47, -58, 27, -83, 0, 86, 3, 34, -35, -48, 16, -93, 3, TarConstants.LF_GNUTYPE_LONGNAME, 66, 108}, new byte[]{-62, 111, 34, 45, 81, -72, -94, 121}));
                    sb2.append(vMConfig.f8869WWWWWWWW);
                    sb2.append(" ");
                    z10 = true;
                } else {
                    sb2.append(str2);
                    sb2.append(" ");
                }
            }
            if (!z10) {
                pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{TarConstants.LF_CHR, 28, -23, -99, ConstantPoolEntry.CP_NameAndType, 111, 91, 66, 61, 29, -7, -63, 16, 99, TarConstants.LF_MULTIVOLUME, 73, TarConstants.LF_CHR, 30, -29, Byte.MIN_VALUE, 94}, new byte[]{82, 114, -115, -17, 99, 6, 63, 32}, sb2);
                sb2.append(vMConfig.f8869WWWWWWWW);
            }
            try {
                Os.chmod(file.getAbsolutePath(), UnixStat.DEFAULT_LINK_PERM);
            } catch (ErrnoException e10) {
                e10.printStackTrace();
            }
            WWWW.m5339WWWoWWWo(file, sb2.toString(), false);
        }
    }

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public static void m5212WWoWWo(File file, HashMap hashMap, PropLineCallback propLineCallback) {
        boolean z10;
        FileOutputStream fileOutputStream;
        ArrayList m5320WWWWoWWWWo = WWWW.m5320WWWWoWWWWo(file);
        if (m5320WWWWoWWWWo == null) {
            m5320WWWWoWWWWo = new ArrayList();
            z10 = false;
        } else {
            z10 = false;
            for (int i10 = 0; i10 < m5320WWWWoWWWWo.size(); i10++) {
                String str = (String) m5320WWWWoWWWWo.get(i10);
                if (!TextUtils.isEmpty(str)) {
                    if (!AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{100}, new byte[]{71, 121, 33, -29, 105, -38, -99, 109}, str)) {
                        Iterator it = hashMap.keySet().iterator();
                        while (true) {
                            if (!it.hasNext()) {
                                break;
                            }
                            String str2 = (String) it.next();
                            StringBuilder m1577WWWWoWWWWo = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(str2);
                            byte[] bArr = {-33, -85, -88, 68, ConstantPoolEntry.CP_InterfaceMethodref, -74, 7, 1};
                            StringFog.f8859WWWWWWWW.getClass();
                            m1577WWWWoWWWWo.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-30}, bArr));
                            if (str.startsWith(m1577WWWWoWWWWo.toString())) {
                                String str3 = (String) hashMap.remove(str2);
                                if (!str.equals(str3)) {
                                    m5320WWWWoWWWWo.set(i10, str3);
                                    z10 = true;
                                }
                            }
                        }
                        if (propLineCallback != null) {
                            String mo5217WWWWWWWW = propLineCallback.mo5217WWWWWWWW(str);
                            if (!str.equals(mo5217WWWWWWWW)) {
                                m5320WWWWoWWWWo.set(i10, mo5217WWWWWWWW);
                                z10 = true;
                            }
                        }
                    }
                }
            }
        }
        for (Object obj : hashMap.keySet()) {
            m5320WWWWoWWWWo.add((String) hashMap.get(obj));
            z10 = true;
        }
        if (z10) {
            AtomicFile atomicFile = new AtomicFile(file);
            try {
                fileOutputStream = atomicFile.startWrite();
            } catch (Throwable unused) {
                fileOutputStream = null;
            }
            try {
                PrintWriter printWriter = new PrintWriter(fileOutputStream);
                int size = m5320WWWWoWWWWo.size();
                int i11 = 0;
                while (i11 < size) {
                    Object obj2 = m5320WWWWoWWWWo.get(i11);
                    i11++;
                    printWriter.println((String) obj2);
                }
                printWriter.flush();
                printWriter.close();
                atomicFile.finishWrite(fileOutputStream);
                WWWW.m5322WWWWWWWW(fileOutputStream);
            } catch (Throwable unused2) {
                try {
                    atomicFile.failWrite(fileOutputStream);
                    WWWW.m5322WWWWWWWW(fileOutputStream);
                } catch (Throwable th2) {
                    WWWW.m5322WWWWWWWW(fileOutputStream);
                    throw th2;
                }
            }
        }
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final boolean mo5037WWWWoWWWWo() {
        return true;
    }

    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5038WWWWWWWW() {
        return this.f9268WWWWWWWW;
    }

    /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
    public final void m5213WWWWWWWW(VMInstance vMInstance) {
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        String str = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {-38, -9, 90, -36, -52, 5, TarConstants.LF_GNUTYPE_SPARSE, 80};
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-11, -109, 63, -70, -83, 112, 63, 36, -12, -121, 40, -77, -68}, bArr));
        if (!file.exists()) {
            file = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-104, -51, 114, -116, 14, -110, -92, 25, -46, -54, 104, -48, 10, -123, -90, 70, -103, -38, 110, -103, 27, -126, -91, 66}, new byte[]{-73, -66, ConstantPoolEntry.CP_InterfaceMethodref, -1, 122, -9, -55, TarConstants.LF_FIFO}));
        }
        if (!file.exists()) {
            return;
        }
        m5212WWoWWo(file, new HashMap(), new WWWWWWWW(this, vMConfig, vMInstance, 4));
    }

    /* renamed from: WWWW֭WWWWྥ֭  reason: contains not printable characters */
    public final void m5214WWWWWWWW(VMInstance vMInstance) {
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        String str = vMConfig.f8868WWWWWWWW;
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -118, -108, 22, 124, 116, 108, -102, 6, -116, -124, 9, 108, 63, 113, -57, ConstantPoolEntry.CP_InterfaceMethodref, -119}, new byte[]{100, -7, -19, 101, 8, 17, 1, -75}));
        if (!file.exists()) {
            return;
        }
        HashMap hashMap = new HashMap();
        if (vMConfig.f8895WWWoWWWo.f8847WWWWWWWW != 11) {
            for (String str2 : vMConfig.f8870WWWWWWWW.keySet()) {
                if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{28, 15, 104, 80, 84}, new byte[]{108, 125, 7, 32, 122, 91, 8, -43}, str2) && !str2.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-41, 112, -46, -7, -63, -8, 41, 90, -58, 119, -47, -3, -63}, new byte[]{-89, 2, -67, -119, -17, -100, TarConstants.LF_GNUTYPE_LONGNAME, 60}))) {
                    String substring = str2.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{-107, 7, 116, 122, -34}, new byte[]{-27, 117, 27, 10, -16, 20, -86, 115}).length());
                    StringBuilder m1577WWWWoWWWWo = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(substring);
                    m1577WWWWoWWWWo.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{36}, new byte[]{25, 84, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 66, 74, TarConstants.LF_GNUTYPE_LONGNAME, 85, 78}));
                    m1577WWWWoWWWWo.append((String) vMConfig.f8870WWWWWWWW.get(str2));
                    hashMap.put(substring, m1577WWWWoWWWWo.toString());
                }
            }
        }
        m5212WWoWWo(file, hashMap, new WWWWWWWW(this, vMConfig, vMInstance, 2));
    }

    /* renamed from: WWWWآWWWWȫآ  reason: contains not printable characters */
    public final void m5215WWWWWWWW(VMInstance vMInstance) {
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        String str = vMConfig.f8868WWWWWWWW;
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{2, -27, -11, 4, 33, -43, -88, -50, 79, -26, -7, 6, 33, -108, -86, -109, 66, -29}, new byte[]{45, -109, -112, 106, 69, -70, -38, -31}));
        if (!file.exists()) {
            return;
        }
        m5212WWoWWo(file, new HashMap(), new WWWWWWWW(this, vMConfig, vMInstance, 1));
    }

    /* JADX WARN: Removed duplicated region for block: B:29:0x03eb  */
    /* JADX WARN: Removed duplicated region for block: B:33:0x0414  */
    /* JADX WARN: Removed duplicated region for block: B:37:0x0440  */
    @Override // com.android.vmcore.IVMStartupTask
    /* renamed from: WWWȏWWWoನ̑ */
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final boolean mo5039WWWoWWWo(VMApp vMApp, VMInstance vMInstance) {
        char c10;
        PrintWriter printWriter;
        boolean z10;
        VMConfig vMConfig;
        File file;
        File file2;
        boolean z11 = true;
        VMConfig vMConfig2 = vMInstance.f8937WWWoWWWo;
        if (vMConfig2.f8895WWWoWWWo.f8847WWWWWWWW == 11) {
            String str = vMConfig2.f8868WWWWWWWW;
            byte[] bArr = {84, 102, -95, 95, 71, -27, 79, -59, 20, 116, -87, 30, 70, -2, 94, -59, 25, 101, -83, 93, 71, -92, TarConstants.LF_MULTIVOLUME, -104, 20, 96};
            byte[] bArr2 = {123, 16, -60, TarConstants.LF_LINK, 35, -118, 61, -22};
            StringFog.f8859WWWWWWWW.getClass();
            File file3 = new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
            if (file3.exists()) {
                HashMap hashMap = new HashMap();
                String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{68, 24, -88, -47, TarConstants.LF_LINK, -8, 91, 116, 69, 3, -88, -50, 44}, new byte[]{TarConstants.LF_FIFO, 119, -122, -89, 92, -42, TarConstants.LF_CHR, 27});
                hashMap.put(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-67, -71, 35, -70, 111, TarConstants.LF_NORMAL, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -124, -68, -94, 35, -91, 114, 35}, new byte[]{-49, -42, 13, -52, 2, 30, TarConstants.LF_NORMAL, -21}) + vMConfig2.f8909WWoWWo);
                String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-25, 113, TarConstants.LF_MULTIVOLUME, -122, 58, 45, -18, -65, -26, 106, TarConstants.LF_MULTIVOLUME, -99, TarConstants.LF_FIFO, 96}, new byte[]{-107, 30, 99, -16, 87, 3, -122, -48});
                hashMap.put(m17835WWWWWWWW2, WWWWWWWW.m17835WWWWWWWW(new byte[]{42, -15, -40, -110, 44, 42, 30, 2, 43, -22, -40, -119, 32, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_UC, -98, -10, -28, 65, 4, 118, 109}) + vMConfig2.f8880WWWWWWWW);
                String m17835WWWWWWWW3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CHR, -101, -74, 0, 93, 10, 74, -40, TarConstants.LF_SYMLINK, Byte.MIN_VALUE, -74, 1, 92, 69, TarConstants.LF_GNUTYPE_LONGNAME, -24, 40, -124}, new byte[]{65, -12, -104, 118, TarConstants.LF_NORMAL, 36, 34, -73});
                hashMap.put(m17835WWWWWWWW3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-24, 7, -77, ConstantPoolEntry.CP_InterfaceMethodref, 70, 31, -83, 13, -23, 28, -77, 10, 71, 80, -85, 61, -13, 24, -96}, new byte[]{-102, 104, -99, 125, 43, TarConstants.LF_LINK, -59, 98}) + vMConfig2.f8916WW);
                String m17835WWWWWWWW4 = WWWWWWWW.m17835WWWWWWWW(new byte[]{126, 26, -46, -88, 4, -69, 40, 17, Byte.MAX_VALUE, 1, -46, -87, 5, -12, 46, 33, 97, 20, -97}, new byte[]{ConstantPoolEntry.CP_NameAndType, 117, -4, -34, 105, -107, 64, 126});
                hashMap.put(m17835WWWWWWWW4, WWWWWWWW.m17835WWWWWWWW(new byte[]{-4, 94, -83, 15, -59, -121, 8, 109, -3, 69, -83, 14, -60, -56, 14, 93, -29, 80, -32, 68}, new byte[]{-114, TarConstants.LF_LINK, -125, 121, -88, -87, 96, 2}) + vMConfig2.f8899WWWoWWWo);
                String m17835WWWWWWWW5 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-113, -67, 91, -56, -14, -111, -101, 41, -114, -90, 91, -55, -10, -39, -102, 25, -97, -95, 6, -41, -5}, new byte[]{-3, -46, 117, -66, -97, -65, -13, 70});
                hashMap.put(m17835WWWWWWWW5, WWWWWWWW.m17835WWWWWWWW(new byte[]{-15, 117, 108, -20, TarConstants.LF_MULTIVOLUME, -97, -63, -11, -16, 110, 108, -19, 73, -41, -64, -59, -31, 105, TarConstants.LF_LINK, -13, 68, -116}, new byte[]{-125, 26, 66, -102, 32, -79, -87, -102}) + vMConfig2.f8879WWWWWWWW);
                String m17835WWWWWWWW6 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-38, 108, 70, 67, -108, -108, 59, TarConstants.LF_CHR, -37, 119, 70, 66, -112, -36, 58, 3, -58, 98, 5, 80}, new byte[]{-88, 3, 104, TarConstants.LF_DIR, -7, -70, TarConstants.LF_GNUTYPE_SPARSE, 92});
                hashMap.put(m17835WWWWWWWW6, WWWWWWWW.m17835WWWWWWWW(new byte[]{-67, -77, 58, 116, -108, 62, 117, 113, -68, -88, 58, 117, -112, 118, 116, 65, -95, -67, 121, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -60}, new byte[]{-49, -36, 20, 2, -7, 16, 29, 30}) + vMConfig2.f8908WWoWWo);
                String m17835WWWWWWWW7 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-52, 47, -84, 59, 96, 2, -99, 58, -51, TarConstants.LF_BLK, -84, 47, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 69, -103, TarConstants.LF_LINK, -112, TarConstants.LF_FIFO, -25, 63, 126, 69, -102, 59, -112, TarConstants.LF_CHR, -26, 38}, new byte[]{-66, 64, -126, TarConstants.LF_MULTIVOLUME, 13, 44, -11, 85});
                hashMap.put(m17835WWWWWWWW7, WWWWWWWW.m17835WWWWWWWW(new byte[]{-22, -113, -84, 114, 68, 114, -82, -2, -21, -108, -84, 102, 92, TarConstants.LF_DIR, -86, -11, -74, -106, -25, 118, 90, TarConstants.LF_DIR, -87, -1, -74, -109, -26, 111, 20}, new byte[]{-104, -32, -126, 4, 41, 92, -58, -111}) + Build.VERSION.SDK_INT);
                m5212WWoWWo(file3, hashMap, null);
            }
        } else {
            VMResConfig m5061WWWWWWWW = vMInstance.m5061WWWWWWWW();
            try {
                String str2 = vMConfig2.f8868WWWWWWWW;
                c10 = 0;
                try {
                    WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
                    wwwwwwww.getClass();
                    PrintWriter printWriter2 = new PrintWriter(new FileOutputStream(new File(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{80, -11, -84, -85, 42, 61, 94, 98, 81, -13, -77, -101, TarConstants.LF_SYMLINK}, new byte[]{Byte.MAX_VALUE, -125, -63, -12, 66, 82, 45, 22}))));
                    try {
                        if (vMInstance.m5085WWoWWo()) {
                            wwwwwwww.getClass();
                            printWriter2.println(WWWWWWWW.m17835WWWWWWWW(new byte[]{-28, 2, -76, 3, TarConstants.LF_LINK, -13, 93, 108, -1, 10, -24, 10, 123, -20}, new byte[]{-110, 111, -102, 107, 70, -35, 62, 13}));
                        } else {
                            wwwwwwww.getClass();
                            printWriter2.println(WWWWWWWW.m17835WWWWWWWW(new byte[]{-105, -24, -38, -126, -91, -46, -73, -95, -116, -32, -122, -117, -17, -52}, new byte[]{-31, -123, -12, -22, -46, -4, -44, -64}));
                        }
                        StringBuilder sb2 = new StringBuilder();
                        wwwwwwww.getClass();
                        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{0, -33, -77, -44, -1, -80, -30, -115, 22, -60, -11, -97}, new byte[]{114, -80, -99, -94, -110, -98, -107, -28}));
                        sb2.append(m5061WWWWWWWW.f8952WWWWoWWWWo);
                        printWriter2.println(sb2.toString());
                        StringBuilder sb3 = new StringBuilder();
                        wwwwwwww.getClass();
                        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{4, -122, -43, -77, -31, -53, -69, 69, 31, -114, -109, -79, -79}, new byte[]{118, -23, -5, -59, -116, -27, -45, 32}));
                        sb3.append(m5061WWWWWWWW.f8955WWWoWWWo);
                        printWriter2.println(sb3.toString());
                        printWriter2.println(StringFog.m5049WWWWWWWW(new byte[]{42, -114, 56, -84, -99, -41, 1, -56, 43, -36}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_UC, -31, 22, -38, -16, -7, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -72}) + vMConfig2.f8918WW);
                        printWriter2.println(StringFog.m5049WWWWWWWW(new byte[]{80, 84, -84, 118, 97, 114, -120, -32, 87, 21, -12, 101, 98, 56, Byte.MIN_VALUE, -30, 31}, new byte[]{34, 59, -126, 0, ConstantPoolEntry.CP_NameAndType, 92, -17, -112}) + vMConfig2.f8892WWWWWWWW);
                        printWriter2.println(StringFog.m5049WWWWWWWW(new byte[]{125, -87, 18, 19, 8, 4, -103, 33, 122, -24, 78, 0, ConstantPoolEntry.CP_InterfaceMethodref, 78, -101, 35, 106, -76, 1}, new byte[]{15, -58, 60, 101, 101, 42, -2, 81}) + vMConfig2.f8893WWWWWWWW);
                        printWriter2.println(StringFog.m5049WWWWWWWW(new byte[]{-64, 70, 97, -59, 121, -26, 40, 96, -63, 93, 97, -38, 100, -11}, new byte[]{-78, 41, 79, -77, 20, -56, 64, 15}) + vMConfig2.f8909WWoWWo);
                        printWriter2.println(StringFog.m5049WWWWWWWW(new byte[]{-54, 107, -124, -36, 7, -101, 106, 22, -53, 112, -124, -57, ConstantPoolEntry.CP_InterfaceMethodref, -42, 63}, new byte[]{-72, 4, -86, -86, 106, -75, 2, 121}) + vMConfig2.f8880WWWWWWWW);
                        printWriter2.println(StringFog.m5049WWWWWWWW(new byte[]{-87, -98, -100, 14, 19, 64, -75, TarConstants.LF_GNUTYPE_LONGNAME, -88, -123, -100, 15, 18, 15, -77, 124, -78, -127, -113}, new byte[]{-37, -15, -78, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 126, 110, -35, 35}) + vMConfig2.f8916WW);
                        printWriter2.println(StringFog.m5049WWWWWWWW(new byte[]{122, 107, Byte.MAX_VALUE, -100, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 60, 25, -11, 123, 112, Byte.MAX_VALUE, -99, 121, 115, 31, -59, 101, 101, TarConstants.LF_SYMLINK, -41}, new byte[]{8, 4, 81, -22, 21, 18, 113, -102}) + vMConfig2.f8899WWWoWWWo);
                        printWriter2.println(StringFog.m5049WWWWWWWW(new byte[]{5, 17, -70, TarConstants.LF_CHR, TarConstants.LF_GNUTYPE_SPARSE, -40, -31, -113, 4, 10, -70, TarConstants.LF_SYMLINK, 87, -112, -32, -65, 21, 13, -25, 44, 90, -53}, new byte[]{119, 126, -108, 69, 62, -10, -119, -32}) + vMConfig2.f8879WWWWWWWW);
                        printWriter2.println(StringFog.m5049WWWWWWWW(new byte[]{41, 43, -123, 10, -45, 69, 41, -98, 40, TarConstants.LF_NORMAL, -123, ConstantPoolEntry.CP_InterfaceMethodref, -41, 13, 40, -82, TarConstants.LF_DIR, 37, -58, 25, -125}, new byte[]{91, 68, -85, 124, -66, 107, 65, -15}) + vMConfig2.f8908WWoWWo);
                        printWriter2.println(StringFog.m5049WWWWWWWW(new byte[]{-93, 108, -99, TarConstants.LF_BLK, 114, 125, 64, 61, -94, 119, -99, 32, 106, 58, 68, TarConstants.LF_FIFO, -1, 117, -42, TarConstants.LF_NORMAL, 108, 58, 71, 60, -1, 112, -41, 41, 34}, new byte[]{-47, 3, -77, 66, 31, TarConstants.LF_GNUTYPE_SPARSE, 40, 82}) + Build.VERSION.SDK_INT);
                        printWriter2.flush();
                        WWWW.m5335WWWoWWWo(printWriter2);
                        z10 = true;
                    } catch (Throwable th2) {
                        th = th2;
                        printWriter = printWriter2;
                        try {
                            this.f9268WWWWWWWW = Log.getStackTraceString(th);
                            this.f9267WWWWoWWWWo = 116000;
                            Closeable[] closeableArr = new Closeable[1];
                            closeableArr[c10] = printWriter;
                            WWWW.m5335WWWoWWWo(closeableArr);
                            z10 = false;
                            z11 = z10;
                            m5210WWWWWWWW(vMInstance);
                            m5211WWWoWWWo(vMInstance);
                            vMConfig = vMInstance.f8937WWWoWWWo;
                            if (vMConfig.f8870WWWWWWWW.isEmpty()) {
                            }
                            m5214WWWWWWWW(vMInstance);
                            String str3 = vMConfig.f8868WWWWWWWW;
                            byte[] bArr3 = {-111, -20, 15, -52, -115, TarConstants.LF_GNUTYPE_LONGNAME, -99, Byte.MIN_VALUE};
                            StringFog.f8859WWWWWWWW.getClass();
                            file = new File(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-66, -97, 118, -65, -7, 41, -16, -81, -31, -98, 96, -88, -8, 47, -23, -81, -13, -103, 102, -96, -23, 98, -19, -14, -2, -100}, bArr3));
                            if (file.exists()) {
                            }
                            file2 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-3, 6, 1, -76, -24, -72, 118, -27, -95, ConstantPoolEntry.CP_NameAndType, ConstantPoolEntry.CP_InterfaceMethodref, -77, -7, -80, 68, -81, -86, 1, 87, -91, -23, -76, 119, -82, -4, 5, 10, -88, -20}, new byte[]{-46, 117, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -57, -100, -35, 27, -54}));
                            if (file2.exists()) {
                            }
                            m5213WWWWWWWW(vMInstance);
                            m5215WWWWWWWW(vMInstance);
                            m5216WWWW(vMInstance);
                            return z11;
                        } catch (Throwable th3) {
                            Closeable[] closeableArr2 = new Closeable[1];
                            closeableArr2[c10] = printWriter;
                            WWWW.m5335WWWoWWWo(closeableArr2);
                            throw th3;
                        }
                    }
                } catch (Throwable th4) {
                    th = th4;
                    printWriter = null;
                    this.f9268WWWWWWWW = Log.getStackTraceString(th);
                    this.f9267WWWWoWWWWo = 116000;
                    Closeable[] closeableArr3 = new Closeable[1];
                    closeableArr3[c10] = printWriter;
                    WWWW.m5335WWWoWWWo(closeableArr3);
                    z10 = false;
                    z11 = z10;
                    m5210WWWWWWWW(vMInstance);
                    m5211WWWoWWWo(vMInstance);
                    vMConfig = vMInstance.f8937WWWoWWWo;
                    if (vMConfig.f8870WWWWWWWW.isEmpty()) {
                    }
                    m5214WWWWWWWW(vMInstance);
                    String str32 = vMConfig.f8868WWWWWWWW;
                    byte[] bArr32 = {-111, -20, 15, -52, -115, TarConstants.LF_GNUTYPE_LONGNAME, -99, Byte.MIN_VALUE};
                    StringFog.f8859WWWWWWWW.getClass();
                    file = new File(str32, WWWWWWWW.m17835WWWWWWWW(new byte[]{-66, -97, 118, -65, -7, 41, -16, -81, -31, -98, 96, -88, -8, 47, -23, -81, -13, -103, 102, -96, -23, 98, -19, -14, -2, -100}, bArr32));
                    if (file.exists()) {
                    }
                    file2 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-3, 6, 1, -76, -24, -72, 118, -27, -95, ConstantPoolEntry.CP_NameAndType, ConstantPoolEntry.CP_InterfaceMethodref, -77, -7, -80, 68, -81, -86, 1, 87, -91, -23, -76, 119, -82, -4, 5, 10, -88, -20}, new byte[]{-46, 117, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -57, -100, -35, 27, -54}));
                    if (file2.exists()) {
                    }
                    m5213WWWWWWWW(vMInstance);
                    m5215WWWWWWWW(vMInstance);
                    m5216WWWW(vMInstance);
                    return z11;
                }
            } catch (Throwable th5) {
                th = th5;
                c10 = 0;
            }
            z11 = z10;
        }
        m5210WWWWWWWW(vMInstance);
        m5211WWWoWWWo(vMInstance);
        vMConfig = vMInstance.f8937WWWoWWWo;
        if (vMConfig.f8870WWWWWWWW.isEmpty()) {
            m5209WWWWWWWW(vMInstance);
        }
        m5214WWWWWWWW(vMInstance);
        String str322 = vMConfig.f8868WWWWWWWW;
        byte[] bArr322 = {-111, -20, 15, -52, -115, TarConstants.LF_GNUTYPE_LONGNAME, -99, Byte.MIN_VALUE};
        StringFog.f8859WWWWWWWW.getClass();
        file = new File(str322, WWWWWWWW.m17835WWWWWWWW(new byte[]{-66, -97, 118, -65, -7, 41, -16, -81, -31, -98, 96, -88, -8, 47, -23, -81, -13, -103, 102, -96, -23, 98, -19, -14, -2, -100}, bArr322));
        if (file.exists()) {
            m5212WWoWWo(file, new HashMap(), new WWWWWWWW(this, vMConfig, vMInstance, 5));
        }
        file2 = new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-3, 6, 1, -76, -24, -72, 118, -27, -95, ConstantPoolEntry.CP_NameAndType, ConstantPoolEntry.CP_InterfaceMethodref, -77, -7, -80, 68, -81, -86, 1, 87, -91, -23, -76, 119, -82, -4, 5, 10, -88, -20}, new byte[]{-46, 117, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -57, -100, -35, 27, -54}));
        if (file2.exists()) {
            m5212WWoWWo(file2, new HashMap(), new WWWWWWWW(this, vMConfig, vMInstance, 3));
        }
        m5213WWWWWWWW(vMInstance);
        m5215WWWWWWWW(vMInstance);
        m5216WWWW(vMInstance);
        return z11;
    }

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public final void m5216WWWW(VMInstance vMInstance) {
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        String str = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {-36, -120, 98, 26, 74, 70, 126, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -100, -102, 106, 91, TarConstants.LF_GNUTYPE_LONGLINK, 93, 111, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -111, -117, 110, 24, 74, 7, 124, 37, -100, -114};
        byte[] bArr2 = {-13, -2, 7, 116, 46, 41, ConstantPoolEntry.CP_NameAndType, 87};
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        if (!file.exists()) {
            return;
        }
        m5212WWoWWo(file, new HashMap(), new WWWWWWWW(this, vMConfig, vMInstance, 0));
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final int getErrorCode() {
        return this.f9267WWWWoWWWWo;
    }

    @Override // com.android.vmcore.IVMStartupTask
    public final String getName() {
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-17, -101, -30, -107, -92, -63, -39, 31, -33, -127, -5, -83, -95, -28, -1}, new byte[]{-83, -18, -117, -7, -64, -105, -108, 79});
    }
}
