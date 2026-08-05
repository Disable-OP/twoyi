package com.android.vmcore.startup;

import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.startup.BuildVMPropTask;
import com.google.android.gms.internal.ads.pr0;
import java.util.HashMap;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p053WWoWWo.AbstractC0470WWWWWWWW;
import vf.AbstractC4470WWWWWWWW;
/* renamed from: com.android.vmcore.startup.WWWW̏WWWWβ̏  reason: invalid class name */
/* loaded from: classes.dex */
public final /* synthetic */ class WWWWWWWW implements BuildVMPropTask.PropLineCallback {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final /* synthetic */ BuildVMPropTask f9279WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final /* synthetic */ int f9280WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final /* synthetic */ VMInstance f9281WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final /* synthetic */ VMConfig f9282WWWoWWWo;

    public /* synthetic */ WWWWWWWW(BuildVMPropTask buildVMPropTask, VMConfig vMConfig, VMInstance vMInstance, int i10) {
        this.f9280WWWWWWWW = i10;
        this.f9279WWWWoWWWWo = buildVMPropTask;
        this.f9282WWWoWWWo = vMConfig;
        this.f9281WWWWWWWW = vMInstance;
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    private final String m5227WWWWoWWWWo(String str) {
        this.f9279WWWWoWWWWo.getClass();
        byte[] bArr = {-98, -52, -101, TarConstants.LF_BLK, 90, -71, -118, TarConstants.LF_MULTIVOLUME};
        x5.WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
        boolean m3444WWWWWWWW = AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-20, -93, -75, 91, 62, -44, -92, 47, -21, -91, -9, 80, 116, -33, -29, 35, -7, -87, -23, 68, 40, -48, -28, 57, -93}, bArr, str);
        VMConfig vMConfig = this.f9282WWWoWWWo;
        if (m3444WWWWWWWW) {
            byte[] bArr2 = {37, 111, -67, 34, 62, TarConstants.LF_CHR, -25, 116, 34, 105, -1, 41, 116, 56, -96, TarConstants.LF_PAX_EXTENDED_HEADER_LC, TarConstants.LF_NORMAL, 101, -31, 61, 40, TarConstants.LF_CONTIG, -89, 98, 106};
            byte[] bArr3 = {87, 0, -109, TarConstants.LF_MULTIVOLUME, 90, 94, -55, 22};
            wwwwwwww.getClass();
            String substring = str.substring(x5.WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3).length());
            HashMap hashMap = vMConfig.f8870WWWWWWWW;
            HashMap hashMap2 = vMConfig.f8870WWWWWWWW;
            wwwwwwww.getClass();
            HashMap hashMap3 = vMConfig.f8870WWWWWWWW;
            StringBuilder m1577WWWWoWWWWo = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo((String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-5, -9, -111, -114, TarConstants.LF_FIFO, -82, -93, 21, -5, -9, -111, -102, 109, -65, -72, 21, -23, -9, -97, -112, 124}, new byte[]{-117, -123, -2, -2, 24, -36, -52, 59}, hashMap));
            pr0.m9003WWWWWWWW(wwwwwwww, new byte[]{8}, new byte[]{39, 57, -103, 84, 117, -16, -111, -16}, m1577WWWWoWWWWo, (String) hashMap2.get(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{37, -36, 44, 101, 92, 40, -65, 72, 37, -36, 44, 113, 7, 57, -92, 72, 59, -49, 46, 112}, new byte[]{85, -82, 67, 21, 114, 90, -48, 102})));
            String m17683WWWWWWWW = AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{125}, new byte[]{82, -1, -51, 38, 117, 107, -23, 68}, m1577WWWWoWWWWo, (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{56, 87, TarConstants.LF_GNUTYPE_LONGLINK, 7, 114, 10, -59, 87, 56, 87, TarConstants.LF_GNUTYPE_LONGLINK, 19, 41, 27, -34, 87, 44, 64, 82, 30, 63, 29}, new byte[]{72, 37, 36, 119, 92, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -86, 121}, hashMap3));
            wwwwwwww.getClass();
            int indexOf = substring.indexOf(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-81}, new byte[]{-107, 80, -15, 3, 5, -19, -33, -98}));
            if (indexOf >= 0) {
                StringBuilder m1577WWWWoWWWWo2 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(m17683WWWWWWWW);
                byte[] bArr4 = {-85, 126, -117, ConstantPoolEntry.CP_NameAndType, 37, 1, -92, -84};
                wwwwwwww.getClass();
                m1577WWWWoWWWWo2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-111}, bArr4));
                m1577WWWWoWWWWo2.append(substring.substring(indexOf + 1));
                substring = m1577WWWWoWWWWo2.toString();
            }
            StringBuilder sb2 = new StringBuilder();
            pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{-50, 72, 4, 27, TarConstants.LF_FIFO, 42, -111, -87, -55, 78, 70, 16, 124, 33, -42, -91, -37, 66, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 4, 32, 46, -47, -65, -127}, new byte[]{-68, 39, 42, 116, 82, 71, -65, -53}, sb2);
            sb2.append(BuildVMPropTask.m5208WWWWWWWW(this.f9281WWWWWWWW, substring));
            return sb2.toString();
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-94, 85, -30, 106, 30, -9, TarConstants.LF_MULTIVOLUME, -99, -77, 78, -30, 117, 8, -11, 7, -123, -79, 84, -71, 124, 13, -5, 93, -99, -94, 95, -66, 39}, new byte[]{-48, 58, -52, 26, 108, -104, 41, -24}, str)) {
            HashMap hashMap4 = vMConfig.f8870WWWWWWWW;
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-102, 27, -47, -90, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 25, 106, 111, -117, 0, -47, -71, 78, 27, 32, 119, -119, 26, -118, -80, TarConstants.LF_GNUTYPE_LONGLINK, 21, 122, 111, -102, 17, -115, -21}, new byte[]{-24, 116, -1, -42, 42, 118, 14, 26}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{5, 38, 33, -76, -51, -95, 58, 13, 5, 38, 33, -96, -106, -80, 33, 13, 24, TarConstants.LF_DIR, 32, -79, -123, -78, TarConstants.LF_FIFO, 87, 0, 38, 43, -74}, new byte[]{117, 84, 78, -60, -29, -45, 85, 35}, hashMap4));
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-11, -67, -17, -68, -76, 18, -63, -119, -28, -90, -17, -93, -94, 16, -117, -111, -24, -74, -92, -96, -5}, new byte[]{-121, -46, -63, -52, -58, 125, -91, -4}, str)) {
            HashMap hashMap5 = vMConfig.f8870WWWWWWWW;
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-51, 108, TarConstants.LF_BLK, -77, -108, -120, 34, -24, -36, 119, TarConstants.LF_BLK, -84, -126, -118, 104, -16, -48, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, Byte.MAX_VALUE, -81, -37}, new byte[]{-65, 3, 26, -61, -26, -25, 70, -99}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{TarConstants.LF_LINK, 27, -3, 116, -3, 29, 43, 13, TarConstants.LF_LINK, 27, -3, 96, -90, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_NORMAL, 13, 44, 6, -10, 97, -65}, new byte[]{65, 105, -110, 4, -45, 111, 68, 35}, hashMap5));
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-81, -115, 2, -103, -54, -100, 99, -20, -66, -106, 2, -122, -36, -98, 41, -5, -81, -125, 66, -115}, new byte[]{-35, -30, 44, -23, -72, -13, 7, -103}, str)) {
            HashMap hashMap6 = vMConfig.f8870WWWWWWWW;
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{110, 108, -19, -57, 121, 109, -102, 26, Byte.MAX_VALUE, 119, -19, -40, 111, 111, -48, 13, 110, 98, -83, -45, TarConstants.LF_FIFO}, new byte[]{28, 3, -61, -73, ConstantPoolEntry.CP_InterfaceMethodref, 2, -2, 111}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{4, 2, 60, 60, Byte.MIN_VALUE, 17, -37, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 4, 2, 60, 40, -37, 0, -64, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 22, 2, TarConstants.LF_SYMLINK, 34, -54}, new byte[]{116, 112, TarConstants.LF_GNUTYPE_SPARSE, TarConstants.LF_GNUTYPE_LONGNAME, -82, 99, -76, 118}, hashMap6));
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-8, -78, -8, -38, 64, 2, -118, 24, -23, -87, -8, -59, 86, 0, -64, 3, -21, -80, -77}, new byte[]{-118, -35, -42, -86, TarConstants.LF_SYMLINK, 109, -18, 109}, str)) {
            HashMap hashMap7 = vMConfig.f8870WWWWWWWW;
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{19, 123, -14, 17, ConstantPoolEntry.CP_InterfaceMethodref, 95, 13, -80, 2, 96, -14, 14, 29, 93, 71, -85, 0, 121, -71, 92}, new byte[]{97, 20, -36, 97, 121, TarConstants.LF_NORMAL, 105, -59}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{65, -57, 19, 91, -49, 28, -43, 113, 65, -57, 19, 79, -108, 13, -50, 113, 95, -44, 17, 78}, new byte[]{TarConstants.LF_LINK, -75, 124, 43, -31, 110, -70, 95}, hashMap7));
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{Byte.MAX_VALUE, 86, -35, -62, -62, 85, 68, -86, 110, TarConstants.LF_MULTIVOLUME, -35, -35, -44, 87, 14, -69, 104, 79, -102, -47, -43}, new byte[]{13, 57, -13, -78, -80, 58, 32, -33}, str)) {
            HashMap hashMap8 = vMConfig.f8870WWWWWWWW;
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{87, -108, -40, -106, -127, -65, 102, TarConstants.LF_NORMAL, 70, -113, -40, -119, -105, -67, 44, 33, 64, -115, -97, -123, -106, -19}, new byte[]{37, -5, -10, -26, -13, -48, 2, 69}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{58, 57, 46, 69, 15, 56, -70, 47, 58, 57, 46, 81, 84, 41, -95, 47, 46, 46, TarConstants.LF_CONTIG, 92, 66, 47}, new byte[]{74, TarConstants.LF_GNUTYPE_LONGLINK, 65, TarConstants.LF_DIR, 33, 74, -43, 1}, hashMap8));
        } else {
            return str;
        }
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    private final String m5228WWWWWWWW(String str) {
        this.f9279WWWWoWWWWo.getClass();
        byte[] bArr = {-52, 111, -111, -66, -84, -98, 8, TarConstants.LF_GNUTYPE_SPARSE, -112, 102, -45, -67, -81, -104, 22, 10};
        byte[] bArr2 = {-66, 0, -65, -36, -39, -9, 100, TarConstants.LF_CONTIG};
        x5.WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
        boolean m3444WWWWWWWW = AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, bArr, bArr2, str);
        VMConfig vMConfig = this.f9282WWWoWWWo;
        if (m3444WWWWWWWW) {
            byte[] bArr3 = {57, 106, 4, 91, 26, -103, -89, 96, 101, 99, 70, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 25, -97, -71, 57};
            byte[] bArr4 = {TarConstants.LF_GNUTYPE_LONGLINK, 5, 42, 57, 111, -16, -53, 4};
            wwwwwwww.getClass();
            String substring = str.substring(x5.WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4).length());
            String str2 = (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-18, 121, 42, -112, -110, -100, -116, -33, -18, 121, 42, -124, -55, -115, -105, -33, -16, 106, 40, -123}, new byte[]{-98, ConstantPoolEntry.CP_InterfaceMethodref, 69, -32, -68, -18, -29, -15}, vMConfig.f8870WWWWWWWW);
            byte[] bArr5 = {-39, -11, -3, -7, 117, 31, -58, TarConstants.LF_SYMLINK};
            wwwwwwww.getClass();
            int indexOf = substring.indexOf(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-12}, bArr5));
            if (indexOf >= 0) {
                StringBuilder m1577WWWWoWWWWo = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(str2);
                byte[] bArr6 = {-71, -9, TarConstants.LF_CHR, -105, -63, 38, -102, 59};
                wwwwwwww.getClass();
                m1577WWWWoWWWWo.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-108}, bArr6));
                m1577WWWWoWWWWo.append(substring.substring(indexOf + 1));
                substring = m1577WWWWoWWWWo.toString();
            }
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-66, -82, -31, 113, -122, 24, 61, 124, -30, -89, -93, 114, -123, 30, 35, 37}, new byte[]{-52, -63, -49, 19, -13, 113, 81, 24}, new StringBuilder(), substring);
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-91, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -124, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 24, 2, -114, -49, -7, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -40, 85, 9, 30, -127, -33, -22}, new byte[]{-41, 8, -86, 58, 109, 107, -30, -85}, str)) {
            HashMap hashMap = vMConfig.f8870WWWWWWWW;
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{63, -122, 29, 93, -105, -38, 4, 17, 99, -103, 65, 80, -122, -58, ConstantPoolEntry.CP_InterfaceMethodref, 1, 112}, new byte[]{TarConstants.LF_MULTIVOLUME, -23, TarConstants.LF_CHR, 63, -30, -77, 104, 117}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-61, 0, -40, -58, 92, 121, 72, 93, -61, 0, -40, -46, 7, 104, TarConstants.LF_GNUTYPE_SPARSE, 93, -41, 23, -63, -33, 17, 110}, new byte[]{-77, 114, -73, -74, 114, ConstantPoolEntry.CP_InterfaceMethodref, 39, 115}, hashMap));
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{46, -119, -24, 78, 2, 72, 116, -65, 114, -126, -93, 95, 20, TarConstants.LF_GNUTYPE_SPARSE, 113, -85, 40, -113, -87, 66, 74}, new byte[]{92, -26, -58, 44, 119, 33, 24, -37}, str)) {
            wwwwwwww.getClass();
            String substring2 = str.substring(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{98, -117, -38, -91, -88, -106, 65, -14, 62, Byte.MIN_VALUE, -111, -76, -66, -115, 68, -26, 100, -115, -101, -87, -32}, new byte[]{16, -28, -12, -57, -35, -1, 45, -106}).length());
            String str3 = (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-96, -17, TarConstants.LF_SYMLINK, -58, -54, 86, TarConstants.LF_GNUTYPE_LONGNAME, 94, -96, -17, TarConstants.LF_SYMLINK, -46, -111, 71, 87, 94, -66, -4, TarConstants.LF_NORMAL, -45}, new byte[]{-48, -99, 93, -74, -28, 36, 35, 112}, vMConfig.f8870WWWWWWWW);
            byte[] bArr7 = {TarConstants.LF_GNUTYPE_LONGLINK};
            wwwwwwww.getClass();
            int indexOf2 = substring2.indexOf(x5.WWWWWWWW.m17835WWWWWWWW(bArr7, new byte[]{102, 98, -65, 69, -13, 27, -59, -24}));
            if (indexOf2 >= 0) {
                StringBuilder m1577WWWWoWWWWo2 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(str3);
                byte[] bArr8 = {-106, -88, 108, -122, 62, TarConstants.LF_DIR, 36, 82};
                wwwwwwww.getClass();
                m1577WWWWoWWWWo2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-69}, bArr8));
                m1577WWWWoWWWWo2.append(substring2.substring(indexOf2 + 1));
                substring2 = m1577WWWWoWWWWo2.toString();
            }
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{40, -7, -2, -33, Byte.MAX_VALUE, 66, -101, -49, 116, -14, -75, -50, 105, 89, -98, -37, 46, -1, -65, -45, TarConstants.LF_CONTIG}, new byte[]{90, -106, -48, -67, 10, 43, -9, -85}, new StringBuilder(), substring2);
        } else {
            boolean m3444WWWWWWWW2 = AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-17, -124, -96, TarConstants.LF_CHR, -16, 85, -18, -78, -77, -115, -25, 63, -30, 89, -16, -90, -17, -126, -32, 37, -72}, new byte[]{-99, -21, -114, 81, -123, 60, -126, -42}, str);
            VMInstance vMInstance = this.f9281WWWWWWWW;
            if (m3444WWWWWWWW2) {
                byte[] bArr9 = {106, 23, 90, 72, -2, 69, 102, -29, TarConstants.LF_FIFO, 30, 29, 68, -20, 73, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -9, 106, 17, 26, 94, -74};
                byte[] bArr10 = {24, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 116, 42, -117, 44, 10, -121};
                wwwwwwww.getClass();
                String substring3 = str.substring(x5.WWWWWWWW.m17835WWWWWWWW(bArr9, bArr10).length());
                HashMap hashMap2 = vMConfig.f8870WWWWWWWW;
                HashMap hashMap3 = vMConfig.f8870WWWWWWWW;
                wwwwwwww.getClass();
                HashMap hashMap4 = vMConfig.f8870WWWWWWWW;
                StringBuilder m1577WWWWoWWWWo3 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo((String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-56, -2, -37, 31, 7, 113, -95, 93, -56, -2, -37, ConstantPoolEntry.CP_InterfaceMethodref, 92, 96, -70, 93, -38, -2, -43, 1, TarConstants.LF_MULTIVOLUME}, new byte[]{-72, -116, -76, 111, 41, 3, -50, 115}, hashMap2));
                pr0.m9003WWWWWWWW(wwwwwwww, new byte[]{-9}, new byte[]{-40, -8, -82, 33, -74, -124, -101, Byte.MAX_VALUE}, m1577WWWWoWWWWo3, (String) hashMap3.get(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-76, -19, 30, -78, -45, -106, -35, 67, -76, -19, 30, -90, -120, -121, -58, 67, -86, -2, 28, -89}, new byte[]{-60, -97, 113, -62, -3, -28, -78, 109})));
                String m17683WWWWWWWW = AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-51}, new byte[]{-30, -50, 42, -67, 86, 92, 61, -116}, m1577WWWWoWWWWo3, (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-100, 68, 41, 6, 99, 100, -92, -10, -100, 68, 41, 18, 56, 117, -65, -10, -120, TarConstants.LF_GNUTYPE_SPARSE, TarConstants.LF_NORMAL, 31, 46, 115}, new byte[]{-20, TarConstants.LF_FIFO, 70, 118, TarConstants.LF_MULTIVOLUME, 22, -53, -40}, hashMap4));
                wwwwwwww.getClass();
                int indexOf3 = substring3.indexOf(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{58}, new byte[]{0, -41, 84, 116, 80, 74, 108, 74}));
                if (indexOf3 >= 0) {
                    StringBuilder m1577WWWWoWWWWo4 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(m17683WWWWWWWW);
                    wwwwwwww.getClass();
                    m1577WWWWoWWWWo4.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-117}, new byte[]{-79, -15, -43, -11, -31, 106, 108, 125}));
                    m1577WWWWoWWWWo4.append(substring3.substring(indexOf3 + 1));
                    substring3 = m1577WWWWoWWWWo4.toString();
                }
                StringBuilder sb2 = new StringBuilder();
                pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{-119, 62, 94, 41, 116, -127, 38, 30, -43, TarConstants.LF_CONTIG, 25, 37, 102, -115, 56, 10, -119, 56, 30, 63, 60}, new byte[]{-5, 81, 112, TarConstants.LF_GNUTYPE_LONGLINK, 1, -24, 74, 122}, sb2);
                sb2.append(BuildVMPropTask.m5208WWWWWWWW(vMInstance, substring3));
                return sb2.toString();
            } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-60, 73, 90, -87, -3, -84, -101, -16, -37, 8, 22, -81, -19, -77, -117, -69, -48, 79, 26, -67, -31, -83, -97, -25, -33, 72, 0, -25}, new byte[]{-74, 38, 116, -38, -124, -33, -17, -107}, str)) {
                wwwwwwww.getClass();
                String substring4 = str.substring(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-81, -55, 71, TarConstants.LF_GNUTYPE_LONGLINK, -66, 111, 99, -51, -80, -120, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_MULTIVOLUME, -82, 112, 115, -122, -69, -49, 7, 95, -94, 110, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -38, -76, -56, 84}, new byte[]{-35, -90, 105, 56, -57, 28, 23, -88}).length());
                HashMap hashMap5 = vMConfig.f8870WWWWWWWW;
                HashMap hashMap6 = vMConfig.f8870WWWWWWWW;
                byte[] bArr11 = {1, 122, 8, 108, -47, -123, -48, -36, 1, 122, 8, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -118, -108, -53, -36, 31, 105, 10, 121};
                byte[] bArr12 = {113, 8, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 28, -1, -9, -65, -14};
                wwwwwwww.getClass();
                HashMap hashMap7 = vMConfig.f8870WWWWWWWW;
                StringBuilder m1577WWWWoWWWWo5 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo((String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-10, -7, -41, -58, -54, 96, TarConstants.LF_MULTIVOLUME, 4, -10, -7, -41, -46, -111, 113, 86, 4, -28, -7, -39, -40, Byte.MIN_VALUE}, new byte[]{-122, -117, -72, -74, -28, 18, 34, 42}, hashMap5));
                pr0.m9003WWWWWWWW(wwwwwwww, new byte[]{56}, new byte[]{23, -44, -51, -60, -59, 35, -12, -89}, m1577WWWWoWWWWo5, (String) hashMap6.get(x5.WWWWWWWW.m17835WWWWWWWW(bArr11, bArr12)));
                String m17683WWWWWWWW2 = AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-101}, new byte[]{-76, -8, -34, -74, TarConstants.LF_CONTIG, 118, 30, 8}, m1577WWWWoWWWWo5, (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-20, 28, -44, -15, TarConstants.LF_GNUTYPE_SPARSE, -22, -69, -31, -20, 28, -44, -27, 8, -5, -96, -31, -8, ConstantPoolEntry.CP_InterfaceMethodref, -51, -24, 30, -3}, new byte[]{-100, 110, -69, -127, 125, -104, -44, -49}, hashMap7));
                wwwwwwww.getClass();
                int indexOf4 = substring4.indexOf(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{123}, new byte[]{65, -99, 100, 33, 46, 29, -15, 96}));
                if (indexOf4 >= 0) {
                    StringBuilder m1577WWWWoWWWWo6 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(m17683WWWWWWWW2);
                    wwwwwwww.getClass();
                    m1577WWWWoWWWWo6.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-85}, new byte[]{-111, 16, -106, 66, 123, -10, -55, -48}));
                    m1577WWWWoWWWWo6.append(substring4.substring(indexOf4 + 1));
                    substring4 = m1577WWWWoWWWWo6.toString();
                }
                StringBuilder sb3 = new StringBuilder();
                pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{34, -109, TarConstants.LF_SYMLINK, 101, -45, 105, -122, -21, 61, -46, 126, 99, -61, 118, -106, -96, TarConstants.LF_FIFO, -107, 114, 113, -49, 104, -126, -4, 57, -110, 104, 43}, new byte[]{80, -4, 28, 22, -86, 26, -14, -114}, sb3);
                sb3.append(BuildVMPropTask.m5208WWWWWWWW(vMInstance, substring4));
                return sb3.toString();
            } else {
                return str;
            }
        }
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    private final String m5229WWWWWWWW(String str) {
        this.f9279WWWWoWWWWo.getClass();
        x5.WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
        boolean m3444WWWWWWWW = AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-64, -81, -30, 39, -21, 40, TarConstants.LF_GNUTYPE_LONGLINK, 22, -33, -97, -87, 44, -26, 117, 93, 6, -37, -84, -88, 122, -12, TarConstants.LF_SYMLINK, 81, 20, -41, -78, -68, 38, -5, TarConstants.LF_DIR, TarConstants.LF_GNUTYPE_LONGLINK, 78}, new byte[]{-78, -64, -52, 84, -110, 91, 63, 115}, str);
        VMConfig vMConfig = this.f9282WWWoWWWo;
        if (m3444WWWWWWWW) {
            wwwwwwww.getClass();
            String substring = str.substring(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{46, 82, -25, ConstantPoolEntry.CP_NameAndType, 100, -67, 1, 30, TarConstants.LF_LINK, 98, -84, 7, 105, -32, 23, 14, TarConstants.LF_DIR, 81, -83, 81, 123, -89, 27, 28, 57, 79, -71, 13, 116, -96, 1, 70}, new byte[]{92, 61, -55, Byte.MAX_VALUE, 29, -50, 117, 123}).length());
            HashMap hashMap = vMConfig.f8870WWWWWWWW;
            HashMap hashMap2 = vMConfig.f8870WWWWWWWW;
            wwwwwwww.getClass();
            HashMap hashMap3 = vMConfig.f8870WWWWWWWW;
            StringBuilder m1577WWWWoWWWWo = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo((String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{TarConstants.LF_SYMLINK, -98, 115, 97, -98, -63, 60, -30, TarConstants.LF_SYMLINK, -98, 115, 117, -59, -48, 39, -30, 32, -98, 125, Byte.MAX_VALUE, -44}, new byte[]{66, -20, 28, 17, -80, -77, TarConstants.LF_GNUTYPE_SPARSE, -52}, hashMap));
            pr0.m9003WWWWWWWW(wwwwwwww, new byte[]{92}, new byte[]{115, 125, -65, 67, -84, 87, TarConstants.LF_FIFO, TarConstants.LF_CHR}, m1577WWWWoWWWWo, (String) hashMap2.get(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, -113, -70, 31, -35, 99, 121, -108, ConstantPoolEntry.CP_InterfaceMethodref, -113, -70, ConstantPoolEntry.CP_InterfaceMethodref, -122, 114, 98, -108, 21, -100, -72, 10}, new byte[]{123, -3, -43, 111, -13, 17, 22, -70})));
            String m17683WWWWWWWW = AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{110}, new byte[]{65, -13, -62, -43, 69, 71, -44, -57}, m1577WWWWoWWWWo, (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{32, 104, -63, -48, TarConstants.LF_GNUTYPE_LONGNAME, -121, 30, -92, 32, 104, -63, -60, 23, -106, 5, -92, TarConstants.LF_BLK, Byte.MAX_VALUE, -40, -55, 1, -112}, new byte[]{80, 26, -82, -96, 98, -11, 113, -118}, hashMap3));
            byte[] bArr = {31, -68, 68, 98, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_GNUTYPE_LONGLINK, -103, 85};
            wwwwwwww.getClass();
            int indexOf = substring.indexOf(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{37}, bArr));
            if (indexOf >= 0) {
                StringBuilder m1577WWWWoWWWWo2 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(m17683WWWWWWWW);
                wwwwwwww.getClass();
                m1577WWWWoWWWWo2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{107}, new byte[]{81, -48, 37, -101, -22, 123, -84, -84}));
                m1577WWWWoWWWWo2.append(substring.substring(indexOf + 1));
                substring = m1577WWWWoWWWWo2.toString();
            }
            StringBuilder sb2 = new StringBuilder();
            pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{20, TarConstants.LF_SYMLINK, -125, TarConstants.LF_GNUTYPE_LONGNAME, 28, -23, 4, -11, ConstantPoolEntry.CP_InterfaceMethodref, 2, -56, 71, 17, -76, 18, -27, 15, TarConstants.LF_LINK, -55, 17, 3, -13, 30, -9, 3, 47, -35, TarConstants.LF_MULTIVOLUME, ConstantPoolEntry.CP_NameAndType, -12, 4, -83}, new byte[]{102, 93, -83, 63, 101, -102, 112, -112}, sb2);
            sb2.append(BuildVMPropTask.m5208WWWWWWWW(this.f9281WWWWWWWW, substring));
            return sb2.toString();
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{115, -18, -114, -45, -46, 66, 70, 122, 98, -11, -114, -48, -39, 94, 86, 106, 108, -34, -59, -37, -44, 3, 79, 110, 111, -12, -58, -62, -61, 89, 87, 125, 100, -13, -99}, new byte[]{1, -127, -96, -93, -96, 45, 34, 15}, str)) {
            HashMap hashMap4 = vMConfig.f8870WWWWWWWW;
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, 71, -91, -99, -76, 67, -112, -70, 105, 92, -91, -98, -65, 95, Byte.MIN_VALUE, -86, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 119, -18, -107, -78, 2, -103, -82, 100, 93, -19, -116, -91, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -127, -67, 111, 90, -74}, new byte[]{10, 40, -117, -19, -58, 44, -12, -49}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{24, 125, -78, -80, -68, -20, 71, -4, 24, 125, -78, -92, -25, -3, 92, -4, 5, 110, -77, -75, -12, -1, TarConstants.LF_GNUTYPE_LONGLINK, -90, 29, 125, -72, -78}, new byte[]{104, 15, -35, -64, -110, -98, 40, -46}, hashMap4));
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-28, 63, 109, -37, 57, -19, -37, -46, -11, 36, 109, -40, TarConstants.LF_SYMLINK, -15, -53, -62, -5, 15, 38, -45, 63, -84, -46, -56, -14, TarConstants.LF_DIR, 47, -106}, new byte[]{-106, 80, 67, -85, TarConstants.LF_GNUTYPE_LONGLINK, -126, -65, -89}, str)) {
            byte[] bArr2 = {-22, 119, -116, -104, -116, -127, -22, 85, -22, 119, -116, -116, -41, -112, -15, 85, -9, 106, -121, -115, -50};
            byte[] bArr3 = {-102, 5, -29, -24, -94, -13, -123, 123};
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-81, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 40, -93, -32, 43, -1, 121, -66, 124, 40, -96, -21, TarConstants.LF_CONTIG, -17, 105, -80, 87, 99, -85, -26, 106, -10, 99, -71, 109, 106, -18}, new byte[]{-35, 8, 6, -45, -110, 68, -101, ConstantPoolEntry.CP_NameAndType}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, bArr2, bArr3, vMConfig.f8870WWWWWWWW));
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{90, -82, 62, 97, 5, 27, ConstantPoolEntry.CP_NameAndType, -116, TarConstants.LF_GNUTYPE_LONGLINK, -75, 62, 98, 14, 7, 28, -100, 69, -98, 117, 105, 3, 90, 10, -117, 73, -81, 116}, new byte[]{40, -63, 16, 17, 119, 116, 104, -7}, str)) {
            byte[] bArr4 = {-10, 3, -82, -86, -15, -82, -58, -75, -10, 3, -82, -66, -86, -65, -35, -75, -28, 3, -96, -76, -69};
            byte[] bArr5 = {-122, 113, -63, -38, -33, -36, -87, -101};
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-46, 90, 122, 93, -76, -31, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -33, -61, 65, 122, 94, -65, -3, 104, -49, -51, 106, TarConstants.LF_LINK, 85, -78, -96, 126, -40, -63, 91, TarConstants.LF_NORMAL, 16}, new byte[]{-96, TarConstants.LF_DIR, 84, 45, -58, -114, 28, -86}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, bArr4, bArr5, vMConfig.f8870WWWWWWWW));
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-42, TarConstants.LF_CHR, 47, 111, 28, -115, -11, -31, -57, 40, 47, 108, 23, -111, -27, -15, -55, 3, 100, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 26, -52, -1, -11, -55, 57}, new byte[]{-92, 92, 1, 31, 110, -30, -111, -108}, str)) {
            HashMap hashMap5 = vMConfig.f8870WWWWWWWW;
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{7, 82, TarConstants.LF_SYMLINK, 92, 70, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 79, -119, 22, 73, TarConstants.LF_SYMLINK, 95, TarConstants.LF_MULTIVOLUME, 100, 95, -103, 24, 98, 121, 84, 64, 57, 69, -99, 24, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 33}, new byte[]{117, 61, 28, 44, TarConstants.LF_BLK, 23, 43, -4}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{81, TarConstants.LF_GNUTYPE_SPARSE, 92, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -42, -58, -43, -101, 81, TarConstants.LF_GNUTYPE_SPARSE, 92, 108, -115, -41, -50, -101, 79, 64, 94, 109}, new byte[]{33, 33, TarConstants.LF_CHR, 8, -8, -76, -70, -75}, hashMap5));
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-108, 118, -68, 118, 86, -113, -31, -26, -123, 109, -68, 117, 93, -109, -15, -10, -117, 70, -9, 126, 80, -50, -31, -10, -112, 112, -15, 99}, new byte[]{-26, 25, -110, 6, 36, -32, -123, -109}, str)) {
            HashMap hashMap6 = vMConfig.f8870WWWWWWWW;
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{62, 66, 93, -119, 0, -13, -107, 19, 47, 89, 93, -118, ConstantPoolEntry.CP_InterfaceMethodref, -17, -123, 3, 33, 114, 22, -127, 6, -78, -107, 3, 58, 68, 16, -100, 79}, new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, 45, 115, -7, 114, -100, -15, 102}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{115, 70, -107, -106, 101, 122, -51, -121, 115, 70, -107, -126, 62, 107, -42, -121, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 81, -116, -113, 40, 109}, new byte[]{3, TarConstants.LF_BLK, -6, -26, TarConstants.LF_GNUTYPE_LONGLINK, 8, -94, -87}, hashMap6));
        } else {
            return str;
        }
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    private final String m5230WWWoWWWo(String str) {
        this.f9279WWWWoWWWWo.getClass();
        byte[] bArr = {98, -24, -15, 31, -13, 64, TarConstants.LF_CHR, 8, 98, -87, -67, 28, -1, 66, TarConstants.LF_CHR, 73, 118, -18, -79, 14, -13, 92, 39, 21, 121, -23, -85, 84};
        byte[] bArr2 = {16, -121, -33, 105, -106, 46, 87, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER};
        x5.WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
        boolean m3444WWWWWWWW = AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, bArr, bArr2, str);
        VMConfig vMConfig = this.f9282WWWoWWWo;
        VMInstance vMInstance = this.f9281WWWWWWWW;
        if (m3444WWWWWWWW) {
            byte[] bArr3 = {8, -30, 59, -113, -10, 17, TarConstants.LF_SYMLINK, -71};
            wwwwwwww.getClass();
            String substring = str.substring(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{122, -115, 21, -7, -109, Byte.MAX_VALUE, 86, -42, 122, -52, 89, -6, -97, 125, 86, -105, 110, -117, 85, -24, -109, 99, 66, -53, 97, -116, 79, -78}, bArr3).length());
            HashMap hashMap = vMConfig.f8870WWWWWWWW;
            HashMap hashMap2 = vMConfig.f8870WWWWWWWW;
            wwwwwwww.getClass();
            HashMap hashMap3 = vMConfig.f8870WWWWWWWW;
            StringBuilder m1577WWWWoWWWWo = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo((String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, -54, 112, 101, -112, -77, -118, 21, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -54, 112, 113, -53, -94, -111, 21, 106, -54, 126, 123, -38}, new byte[]{8, -72, 31, 21, -66, -63, -27, 59}, hashMap));
            pr0.m9003WWWWWWWW(wwwwwwww, new byte[]{-117}, new byte[]{-92, 13, 17, 3, 81, 67, 0, -71}, m1577WWWWoWWWWo, (String) hashMap2.get(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{94, -118, -61, 126, -106, -12, -20, TarConstants.LF_MULTIVOLUME, 94, -118, -61, 106, -51, -27, -9, TarConstants.LF_MULTIVOLUME, 64, -103, -63, 107}, new byte[]{46, -8, -84, 14, -72, -122, -125, 99})));
            String m17683WWWWWWWW = AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{ConstantPoolEntry.CP_NameAndType}, new byte[]{35, -84, -48, 96, -82, -95, -118, 108}, m1577WWWWoWWWWo, (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{123, 35, 59, -53, TarConstants.LF_GNUTYPE_LONGNAME, -122, 60, 6, 123, 35, 59, -33, 23, -105, 39, 6, 111, TarConstants.LF_BLK, 34, -46, 1, -111}, new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, 81, 84, -69, 98, -12, TarConstants.LF_GNUTYPE_SPARSE, 40}, hashMap3));
            byte[] bArr4 = {106, -93, 10, 81, 110, 124, TarConstants.LF_CONTIG, -4};
            wwwwwwww.getClass();
            int indexOf = substring.indexOf(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{80}, bArr4));
            if (indexOf >= 0) {
                StringBuilder m1577WWWWoWWWWo2 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(m17683WWWWWWWW);
                wwwwwwww.getClass();
                m1577WWWWoWWWWo2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-43}, new byte[]{-17, -29, -101, -71, -45, 19, -75, -12}));
                m1577WWWWoWWWWo2.append(substring.substring(indexOf + 1));
                substring = m1577WWWWoWWWWo2.toString();
            }
            StringBuilder sb2 = new StringBuilder();
            pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{93, -32, -27, -108, 96, 30, -101, -110, 93, -95, -87, -105, 108, 28, -101, -45, 73, -26, -91, -123, 96, 2, -113, -113, 70, -31, -65, -33}, new byte[]{47, -113, -53, -30, 5, 112, -1, -3}, sb2);
            sb2.append(BuildVMPropTask.m5208WWWWWWWW(vMInstance, substring));
            return sb2.toString();
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-58, 28, -114, 0, 102, 85, -42, 90, -39, 18, -57, 7, 39, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -41, 90, -40, 23, -114, 4, 96, 84, -59, 86, -58, 3, -46, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 78, -97}, new byte[]{-76, 115, -96, 98, 9, 58, -94, TarConstants.LF_CHR}, str)) {
            wwwwwwww.getClass();
            String substring2 = str.substring(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-60, 70, -7, 125, -3, -84, 14, 85, -37, 72, -80, 122, -68, -95, 15, 85, -38, TarConstants.LF_MULTIVOLUME, -7, 121, -5, -83, 29, 89, -60, 89, -91, 118, -4, -73, 71}, new byte[]{-74, 41, -41, 31, -110, -61, 122, 60}).length());
            HashMap hashMap4 = vMConfig.f8870WWWWWWWW;
            HashMap hashMap5 = vMConfig.f8870WWWWWWWW;
            wwwwwwww.getClass();
            HashMap hashMap6 = vMConfig.f8870WWWWWWWW;
            StringBuilder m1577WWWWoWWWWo3 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo((String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{89, 115, -78, -126, -53, 14, 87, -88, 89, 115, -78, -106, -112, 31, TarConstants.LF_GNUTYPE_LONGNAME, -88, TarConstants.LF_GNUTYPE_LONGLINK, 115, -68, -100, -127}, new byte[]{41, 1, -35, -14, -27, 124, 56, -122}, hashMap4));
            pr0.m9003WWWWWWWW(wwwwwwww, new byte[]{24}, new byte[]{TarConstants.LF_CONTIG, 81, -49, -66, 79, -5, -120, -122}, m1577WWWWoWWWWo3, (String) hashMap5.get(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-1, -104, -41, 106, -102, 38, -27, -31, -1, -104, -41, 126, -63, TarConstants.LF_CONTIG, -2, -31, -31, -117, -43, Byte.MAX_VALUE}, new byte[]{-113, -22, -72, 26, -76, 84, -118, -49})));
            String m17683WWWWWWWW2 = AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-40}, new byte[]{-9, TarConstants.LF_CONTIG, -83, -64, -58, 108, -9, -74}, m1577WWWWoWWWWo3, (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{111, 34, -37, -97, 20, -42, 5, -72, 111, 34, -37, -117, 79, -57, 30, -72, 123, TarConstants.LF_DIR, -62, -122, 89, -63}, new byte[]{31, 80, -76, -17, 58, -92, 106, -106}, hashMap6));
            wwwwwwww.getClass();
            int indexOf2 = substring2.indexOf(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-118}, new byte[]{-80, -110, -60, -11, 38, 100, 78, 111}));
            if (indexOf2 >= 0) {
                StringBuilder m1577WWWWoWWWWo4 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(m17683WWWWWWWW2);
                byte[] bArr5 = {TarConstants.LF_GNUTYPE_LONGLINK};
                wwwwwwww.getClass();
                m1577WWWWoWWWWo4.append(x5.WWWWWWWW.m17835WWWWWWWW(bArr5, new byte[]{113, -121, -21, 99, 123, 56, -78, -37}));
                m1577WWWWoWWWWo4.append(substring2.substring(indexOf2 + 1));
                substring2 = m1577WWWWoWWWWo4.toString();
            }
            StringBuilder sb3 = new StringBuilder();
            pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{-50, -4, 125, -1, 114, -103, -119, 8, -47, -14, TarConstants.LF_BLK, -8, TarConstants.LF_CHR, -108, -120, 8, -48, -9, 125, -5, 116, -104, -102, 4, -50, -29, 33, -12, 115, -126, -64}, new byte[]{-68, -109, TarConstants.LF_GNUTYPE_SPARSE, -99, 29, -10, -3, 97}, sb3);
            sb3.append(BuildVMPropTask.m5208WWWWWWWW(vMInstance, substring2));
            return sb3.toString();
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{29, 79, 87, -17, 37, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 35, -48, ConstantPoolEntry.CP_NameAndType, 84, 87, -23, TarConstants.LF_SYMLINK, 121, 35, -54, 29, 14, 20, -2, 57, 98, 33, -60, ConstantPoolEntry.CP_NameAndType, 84, ConstantPoolEntry.CP_NameAndType, -19, TarConstants.LF_SYMLINK, 101, 122}, new byte[]{111, 32, 121, -97, 87, 23, 71, -91}, str)) {
            HashMap hashMap7 = vMConfig.f8870WWWWWWWW;
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{7, -68, 63, -18, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -57, -13, -16, 22, -89, 63, -24, 79, -58, -13, -22, 7, -3, 124, -1, 68, -35, -15, -28, 22, -89, 100, -20, 79, -38, -86}, new byte[]{117, -45, 17, -98, 42, -88, -105, -123}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{58, 23, 118, TarConstants.LF_GNUTYPE_LONGLINK, -77, -79, -35, -125, 58, 23, 118, 95, -24, -96, -58, -125, 39, 4, 119, 78, -5, -94, -47, -39, 63, 23, 124, 73}, new byte[]{74, 101, 25, 59, -99, -61, -78, -83}, hashMap7));
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{32, -109, -32, -71, 5, -48, 34, -125, TarConstants.LF_LINK, -120, -32, -65, 18, -47, 34, -103, 32, -46, -93, -90, 19, -38, 42, -53}, new byte[]{82, -4, -50, -55, 119, -65, 70, -10}, str)) {
            HashMap hashMap8 = vMConfig.f8870WWWWWWWW;
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{15, -102, 115, -94, -23, -79, 16, -15, 30, -127, 115, -92, -2, -80, 16, -21, 15, -37, TarConstants.LF_NORMAL, -67, -1, -69, 24, -71}, new byte[]{125, -11, 93, -46, -101, -34, 116, -124}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -8, -1, Byte.MAX_VALUE, Byte.MIN_VALUE, 80, TarConstants.LF_CHR, 31, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -8, -1, 107, -37, 65, 40, 31, 122, -27, -12, 106, -62}, new byte[]{23, -118, -112, 15, -82, 34, 92, TarConstants.LF_LINK}, hashMap8));
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-55, -46, -102, 104, 47, ConstantPoolEntry.CP_NameAndType, 89, 101, -40, -55, -102, 110, 56, 13, 89, Byte.MAX_VALUE, -55, -109, -42, 106, 60, 13, 89}, new byte[]{-69, -67, -76, 24, 93, 99, 61, 16}, str)) {
            byte[] bArr6 = {-108, 37, 124, 5, -107, 97, -3, -124, -108, 37, 124, 17, -50, 112, -26, -124, -122, 37, 114, 27, -33};
            byte[] bArr7 = {-28, 87, 19, 117, -69, 19, -110, -86};
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{20, -59, -72, 99, ConstantPoolEntry.CP_InterfaceMethodref, 108, 62, 31, 5, -34, -72, 101, 28, 109, 62, 5, 20, -124, -12, 97, 24, 109, 62, 87}, new byte[]{102, -86, -106, 19, 121, 3, 90, 106}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, bArr6, bArr7, vMConfig.f8870WWWWWWWW));
        } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-118, 115, -120, 42, -78, 119, -76, 125, -101, 104, -120, 44, -91, 118, -76, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -118, TarConstants.LF_SYMLINK, -56, 59, -83, 125}, new byte[]{-8, 28, -90, 90, -64, 24, -48, 8}, str)) {
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-2, 15, -9, 121, 26, -11, 64, 107, -17, 20, -9, Byte.MAX_VALUE, 13, -12, 64, 113, -2, 78, -73, 104, 5, -1, 25}, new byte[]{-116, 96, -39, 9, 104, -102, 36, 30}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{123, -123, -17, 73, 13, TarConstants.LF_DIR, -112, -76, 123, -123, -17, 93, 86, 36, -117, -76, 101, -106, -19, 92}, new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, -9, Byte.MIN_VALUE, 57, 35, 71, -1, -102}, vMConfig.f8870WWWWWWWW));
        } else if (!AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{81, -13, -61, 8, 85, 21, -37, TarConstants.LF_GNUTYPE_SPARSE, 64, -24, -61, 14, 66, 20, -37, 73, 81, -78, -119, 29, 81, 19, -36, 67}, new byte[]{35, -100, -19, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 39, 122, -65, 38}, str)) {
            return str;
        } else {
            HashMap hashMap9 = vMConfig.f8870WWWWWWWW;
            return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{26, 29, 100, TarConstants.LF_SYMLINK, 105, -33, 5, -47, ConstantPoolEntry.CP_InterfaceMethodref, 6, 100, TarConstants.LF_BLK, 126, -34, 5, -53, 26, 92, 46, 39, 109, -39, 2, -63, 85}, new byte[]{104, 114, 74, 66, 27, -80, 97, -92}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-60, 99, -55, -68, 63, 62, 41, 29, -60, 99, -55, -88, 100, 47, TarConstants.LF_SYMLINK, 29, -48, 116, -48, -91, 114, 41}, new byte[]{-76, 17, -90, -52, 17, TarConstants.LF_GNUTYPE_LONGNAME, 70, TarConstants.LF_CHR}, hashMap9));
        }
    }

    @Override // com.android.vmcore.startup.BuildVMPropTask.PropLineCallback
    /* renamed from: WWWW̏WWWWβ̏ */
    public final String mo5217WWWWWWWW(String str) {
        switch (this.f9280WWWWWWWW) {
            case 0:
                return m5227WWWWoWWWWo(str);
            case 1:
                return m5230WWWoWWWo(str);
            case 2:
                return m5228WWWWWWWW(str);
            case 3:
                return m5229WWWWWWWW(str);
            case 4:
                this.f9279WWWWoWWWWo.getClass();
                if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{26, -87, TarConstants.LF_NORMAL, -77, 37, -33, Byte.MAX_VALUE, 0, 5, -89, 121, -76, 100, -46, 126, 0, 4, -94, TarConstants.LF_NORMAL, -73, 35, -34, 108, ConstantPoolEntry.CP_NameAndType, 26, -74, 108, -72, 36, -60, TarConstants.LF_FIFO}, new byte[]{104, -58, 30, -47, 74, -80, ConstantPoolEntry.CP_InterfaceMethodref, 105}, str)) {
                    String substring = str.substring(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{21, 37, 64, -64, 125, 40, 117, 84, 10, 43, 9, -57, 60, 37, 116, 84, ConstantPoolEntry.CP_InterfaceMethodref, 46, 64, -60, 123, 41, 102, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 21, 58, 28, -53, 124, TarConstants.LF_CHR, 60}, new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 74, 110, -94, 18, 71, 1, 61}).length());
                    VMConfig vMConfig = this.f9282WWWoWWWo;
                    StringBuilder m1577WWWWoWWWWo = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo((String) vMConfig.f8870WWWWWWWW.get(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-46, 93, 9, 0, -102, -68, 115, -96, -46, 93, 9, 20, -63, -83, 104, -96, -64, 93, 7, 30, -48}, new byte[]{-94, 47, 102, 112, -76, -50, 28, -114})));
                    pr0.m9009WWWoWWWo(new byte[]{14}, new byte[]{33, -68, 25, -29, -9, 110, 110, -98}, m1577WWWWoWWWWo, (String) vMConfig.f8870WWWWWWWW.get(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{110, -108, -124, 63, -88, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -89, 46, 110, -108, -124, 43, -13, 118, -68, 46, 112, -121, -122, 42}, new byte[]{30, -26, -21, 79, -122, 21, -56, 0})));
                    m1577WWWWoWWWWo.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{123}, new byte[]{84, -30, 98, TarConstants.LF_SYMLINK, 72, -38, 66, 102}));
                    m1577WWWWoWWWWo.append((String) vMConfig.f8870WWWWWWWW.get(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{65, TarConstants.LF_BLK, -99, 13, -21, -119, 8, 117, 65, TarConstants.LF_BLK, -99, 25, -80, -104, 19, 117, 85, 35, -124, 20, -90, -98}, new byte[]{TarConstants.LF_LINK, 70, -14, 125, -59, -5, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 91})));
                    String sb2 = m1577WWWWoWWWWo.toString();
                    int indexOf = substring.indexOf(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-126}, new byte[]{-72, -45, 96, 72, 116, 43, 58, -72}));
                    if (indexOf >= 0) {
                        StringBuilder m1577WWWWoWWWWo2 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(sb2);
                        m1577WWWWoWWWWo2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-94}, new byte[]{-104, 24, -19, 96, -107, -45, -86, 116}));
                        m1577WWWWoWWWWo2.append(substring.substring(indexOf + 1));
                        substring = m1577WWWWoWWWWo2.toString();
                    }
                    return x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{74, 100, -103, TarConstants.LF_GNUTYPE_SPARSE, -37, -40, -121, -63, 85, 106, -48, 84, -102, -43, -122, -63, 84, 111, -103, 87, -35, -39, -108, -51, 74, 123, -59, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -38, -61, -50}, new byte[]{56, ConstantPoolEntry.CP_InterfaceMethodref, -73, TarConstants.LF_LINK, -76, -73, -13, -88}) + BuildVMPropTask.m5208WWWWWWWW(this.f9281WWWWWWWW, substring);
                }
                return str;
            default:
                this.f9279WWWWoWWWWo.getClass();
                x5.WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
                boolean m3444WWWWWWWW = AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-90, 37, 35, -113, 40, TarConstants.LF_BLK, -109, 93, -73, 62, 35, -99, 47, TarConstants.LF_SYMLINK, -101, TarConstants.LF_GNUTYPE_LONGNAME, -6, 44, 100, -111, 61, 62, -123, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -90, 35, 99, -117, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}, new byte[]{-44, 74, 13, -1, 90, 91, -9, 40}, str);
                VMConfig vMConfig2 = this.f9282WWWoWWWo;
                if (m3444WWWWWWWW) {
                    wwwwwwww.getClass();
                    String substring2 = str.substring(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{97, -75, Byte.MIN_VALUE, 92, -112, 125, 114, -92, 112, -82, Byte.MIN_VALUE, 78, -105, 123, 122, -75, 61, -68, -57, 66, -123, 119, 100, -95, 97, -77, -64, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -33}, new byte[]{19, -38, -82, 44, -30, 18, 22, -47}).length());
                    HashMap hashMap = vMConfig2.f8870WWWWWWWW;
                    HashMap hashMap2 = vMConfig2.f8870WWWWWWWW;
                    byte[] bArr = {92, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -6, 89, 29, 3, 70, 114, 92, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -6, TarConstants.LF_MULTIVOLUME, 70, 18, 93, 114, 66, 107, -8, TarConstants.LF_GNUTYPE_LONGNAME};
                    byte[] bArr2 = {44, 10, -107, 41, TarConstants.LF_CHR, 113, 41, 92};
                    wwwwwwww.getClass();
                    HashMap hashMap3 = vMConfig2.f8870WWWWWWWW;
                    StringBuilder m1577WWWWoWWWWo3 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo((String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{19, 37, -124, 82, 99, 26, 94, -109, 19, 37, -124, 70, 56, ConstantPoolEntry.CP_InterfaceMethodref, 69, -109, 1, 37, -118, TarConstants.LF_GNUTYPE_LONGNAME, 41}, new byte[]{99, 87, -21, 34, TarConstants.LF_MULTIVOLUME, 104, TarConstants.LF_LINK, -67}, hashMap));
                    pr0.m9003WWWWWWWW(wwwwwwww, new byte[]{82}, new byte[]{125, -109, 14, ConstantPoolEntry.CP_InterfaceMethodref, 42, -101, -127, -97}, m1577WWWWoWWWWo3, (String) hashMap2.get(x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)));
                    String m17683WWWWWWWW = AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{TarConstants.LF_FIFO}, new byte[]{25, 36, 86, 28, 6, 10, -107, -18}, m1577WWWWoWWWWo3, (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-91, -40, -58, -29, -81, 30, 56, -112, -91, -40, -58, -9, -12, 15, 35, -112, -79, -49, -33, -6, -30, 9}, new byte[]{-43, -86, -87, -109, -127, 108, 87, -66}, hashMap3));
                    wwwwwwww.getClass();
                    int indexOf2 = substring2.indexOf(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-83}, new byte[]{-105, -116, 73, 7, 102, 25, -63, -106}));
                    if (indexOf2 >= 0) {
                        StringBuilder m1577WWWWoWWWWo4 = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(m17683WWWWWWWW);
                        byte[] bArr3 = {TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER};
                        byte[] bArr4 = {93, -28, -114, -15, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 94, 96, -124};
                        wwwwwwww.getClass();
                        m1577WWWWoWWWWo4.append(x5.WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4));
                        m1577WWWWoWWWWo4.append(substring2.substring(indexOf2 + 1));
                        substring2 = m1577WWWWoWWWWo4.toString();
                    }
                    StringBuilder sb3 = new StringBuilder();
                    pr0.m9002WWWWWWWW(wwwwwwww, new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, 104, -123, -116, -67, -85, -66, 116, 26, 115, -123, -98, -70, -83, -74, 101, 87, 97, -62, -110, -88, -95, -88, 113, ConstantPoolEntry.CP_InterfaceMethodref, 110, -59, -120, -14}, new byte[]{121, 7, -85, -4, -49, -60, -38, 1}, sb3);
                    sb3.append(BuildVMPropTask.m5208WWWWWWWW(this.f9281WWWWWWWW, substring2));
                    return sb3.toString();
                } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{18, -96, 68, 16, 81, 66, -27, -48, 3, -69, 68, 16, 81, 66, -27, -48, 3, -69, 68, 13, 66, 67, -12, -61, 1, -84, 30, 21, 81, 72, -13, -104}, new byte[]{96, -49, 106, 96, 35, 45, -127, -91}, str)) {
                    HashMap hashMap4 = vMConfig2.f8870WWWWWWWW;
                    return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-95, TarConstants.LF_NORMAL, 22, -12, -60, 7, -86, 79, -80, 43, 22, -12, -60, 7, -86, 79, -80, 43, 22, -23, -41, 6, -69, 92, -78, 60, TarConstants.LF_GNUTYPE_LONGNAME, -15, -60, 13, -68, 7}, new byte[]{-45, 95, 56, -124, -74, 104, -50, 58}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-54, 33, -50, 108, 118, 62, -31, -110, -54, 33, -50, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 45, 47, -6, -110, -41, TarConstants.LF_SYMLINK, -49, 105, 62, 45, -19, -56, -49, 33, -60, 110}, new byte[]{-70, TarConstants.LF_GNUTYPE_SPARSE, -95, 28, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_GNUTYPE_LONGNAME, -114, -68}, hashMap4));
                } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{82, -112, 5, -105, 124, -72, 36, 71, 67, -117, 5, -105, 124, -72, 36, 71, 67, -117, 5, -118, 97, -77, 37, 94, 29}, new byte[]{32, -1, 43, -25, 14, -41, 64, TarConstants.LF_SYMLINK}, str)) {
                    HashMap hashMap5 = vMConfig2.f8870WWWWWWWW;
                    return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-18, -25, -11, TarConstants.LF_FIFO, -87, -20, -97, 124, -1, -4, -11, TarConstants.LF_FIFO, -87, -20, -97, 124, -1, -4, -11, 43, -76, -25, -98, 101, -95}, new byte[]{-100, -120, -37, 70, -37, -125, -5, 9}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-60, -45, 18, 123, 74, 92, -48, -11, -60, -45, 18, 111, 17, TarConstants.LF_MULTIVOLUME, -53, -11, -39, -50, 25, 110, 8}, new byte[]{-76, -95, 125, ConstantPoolEntry.CP_InterfaceMethodref, 100, 46, -65, -37}, hashMap5));
                } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-87, 46, TarConstants.LF_FIFO, 74, -79, 40, 16, 124, -72, TarConstants.LF_DIR, TarConstants.LF_FIFO, 74, -79, 40, 16, 124, -72, TarConstants.LF_DIR, TarConstants.LF_FIFO, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -79, 38, 26, 109}, new byte[]{-37, 65, 24, 58, -61, 71, 116, 9}, str)) {
                    HashMap hashMap6 = vMConfig2.f8870WWWWWWWW;
                    return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-26, 58, -114, 33, -23, TarConstants.LF_GNUTYPE_SPARSE, 102, 125, -9, 33, -114, 33, -23, TarConstants.LF_GNUTYPE_SPARSE, 102, 125, -9, 33, -114, TarConstants.LF_CHR, -23, 93, 108, 108, -87}, new byte[]{-108, 85, -96, 81, -101, 60, 2, 8}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{100, TarConstants.LF_NORMAL, -30, -64, -35, -109, 38, -84, 100, TarConstants.LF_NORMAL, -30, -44, -122, -126, 61, -84, 118, TarConstants.LF_NORMAL, -20, -34, -105}, new byte[]{20, 66, -115, -80, -13, -31, 73, -126}, hashMap6));
                } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-105, 99, 22, -107, -48, 81, 102, -127, -122, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 22, -107, -48, 81, 102, -127, -122, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 22, -117, -61, TarConstants.LF_GNUTYPE_SPARSE, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}, new byte[]{-27, ConstantPoolEntry.CP_NameAndType, 56, -27, -94, 62, 2, -12}, str)) {
                    HashMap hashMap7 = vMConfig2.f8870WWWWWWWW;
                    return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-52, -77, -111, -102, 106, -62, 74, 44, -35, -88, -111, -102, 106, -62, 74, 44, -35, -88, -111, -124, 121, -64, TarConstants.LF_GNUTYPE_LONGLINK, 100}, new byte[]{-66, -36, -65, -22, 24, -83, 46, 89}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{3, 72, -27, TarConstants.LF_DIR, -37, -117, -22, 35, 3, 72, -27, 33, Byte.MIN_VALUE, -102, -15, 35, 29, 91, -25, 32}, new byte[]{115, 58, -118, 69, -11, -7, -123, 13}, hashMap7));
                } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{41, -117, 1, -56, 97, -115, -18, 0, 56, -112, 1, -56, 97, -115, -18, 0, 56, -112, 1, -36, 118, -108, -29, 22, 62}, new byte[]{91, -28, 47, -72, 19, -30, -118, 117}, str)) {
                    HashMap hashMap8 = vMConfig2.f8870WWWWWWWW;
                    return AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-89, -66, 24, 7, 34, -12, -120, -80, -74, -91, 24, 7, 34, -12, -120, -80, -74, -91, 24, 19, TarConstants.LF_DIR, -19, -123, -90, -80, -20}, new byte[]{-43, -47, TarConstants.LF_FIFO, 119, 80, -101, -20, -59}, new StringBuilder(), (String) AbstractC4470WWWWWWWW.m17687WWWoWWWo(wwwwwwww, new byte[]{-32, -50, -26, 125, -87, 36, -47, ConstantPoolEntry.CP_NameAndType, -32, -50, -26, 105, -14, TarConstants.LF_DIR, -54, ConstantPoolEntry.CP_NameAndType, -12, -39, -1, 100, -28, TarConstants.LF_CHR}, new byte[]{-112, -68, -119, 13, -121, 86, -66, 34}, hashMap8));
                } else {
                    return str;
                }
        }
    }
}
