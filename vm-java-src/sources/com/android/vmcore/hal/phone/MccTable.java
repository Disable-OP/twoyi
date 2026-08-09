package com.android.vmcore.hal.phone;

import com.android.vmcore.StringFog;
import com.google.android.gms.internal.ads.pr0;
import j$.util.Objects;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.Locale;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.archivers.zip.UnixStat;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public final class MccTable {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static final ArrayList f9157WWWWWWWW;

    /* loaded from: classes.dex */
    public static class MccEntry implements Comparable<MccEntry> {

        /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
        public final String f9158WWWWWWWWWW;

        /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
        public final int f9159WWWWoWWWWo;

        public MccEntry(int i10, String str) {
            this.f9159WWWWoWWWWo = i10;
            this.f9158WWWWWWWWWW = str;
        }

        @Override // java.lang.Comparable
        public final int compareTo(MccEntry mccEntry) {
            return this.f9159WWWWoWWWWo - mccEntry.f9159WWWWoWWWWo;
        }
    }

    /* loaded from: classes.dex */
    public static class MccMnc {
        public final boolean equals(Object obj) {
            if (this == obj) {
                return true;
            }
            if (obj != null && getClass() == obj.getClass()) {
                MccMnc mccMnc = (MccMnc) obj;
                throw null;
            }
            return false;
        }

        public final int hashCode() {
            return Objects.hash(null, null);
        }

        public final String toString() {
            StringBuilder sb2 = new StringBuilder();
            pr0.m9003WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{85, -26, 37, 26, 70, -91, -102, 113, 123, -26, 123, 112}, new byte[]{24, -123, 70, 87, 40, -58, -31, 28}, sb2, "null'");
            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-51, 99, -91, 92, -110, 14, 87}, new byte[]{-31, 67, -56, TarConstants.LF_SYMLINK, -15, TarConstants.LF_CHR, 112, -61}));
            sb2.append("null'}");
            return sb2.toString();
        }
    }

    static {
        StringFog.m5049WWWWWWWW(new byte[]{26, -108, -105, 111, 107, TarConstants.LF_DIR, Byte.MIN_VALUE, -109}, new byte[]{87, -9, -12, 59, 10, 87, -20, -10});
        new HashMap().put(Locale.ENGLISH, Locale.US);
        ArrayList arrayList = new ArrayList(240);
        f9157WWWWWWWW = arrayList;
        arrayList.add(new MccEntry(202, StringFog.m5049WWWWWWWW(new byte[]{13, -93}, new byte[]{106, -47, TarConstants.LF_MULTIVOLUME, 40, -89, 7, 109, 114})));
        arrayList.add(new MccEntry(204, StringFog.m5049WWWWWWWW(new byte[]{-123, 61}, new byte[]{-21, 81, 25, -50, -84, 124, 35, 3})));
        arrayList.add(new MccEntry(206, StringFog.m5049WWWWWWWW(new byte[]{-87, ConstantPoolEntry.CP_NameAndType}, new byte[]{-53, 105, TarConstants.LF_SYMLINK, -14, 94, 110, -61, -97})));
        arrayList.add(new MccEntry(208, StringFog.m5049WWWWWWWW(new byte[]{29, -118}, new byte[]{123, -8, 13, Byte.MIN_VALUE, -19, -13, -10, -82})));
        arrayList.add(new MccEntry(212, StringFog.m5049WWWWWWWW(new byte[]{86, -125}, new byte[]{59, -32, -15, 78, -126, -100, 23, 121})));
        arrayList.add(new MccEntry(213, StringFog.m5049WWWWWWWW(new byte[]{-50, 22}, new byte[]{-81, 114, 32, TarConstants.LF_BLK, -36, 61, TarConstants.LF_GNUTYPE_SPARSE, -91})));
        arrayList.add(new MccEntry(214, StringFog.m5049WWWWWWWW(new byte[]{66, 94}, new byte[]{39, 45, 7, 109, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -84, -75, -48})));
        arrayList.add(new MccEntry(216, StringFog.m5049WWWWWWWW(new byte[]{28, -32}, new byte[]{116, -107, -104, 74, -81, 26, 32, -108})));
        arrayList.add(new MccEntry(218, StringFog.m5049WWWWWWWW(new byte[]{-9, 40}, new byte[]{-107, 73, -82, -38, 106, -40, 93, -107})));
        arrayList.add(new MccEntry(219, StringFog.m5049WWWWWWWW(new byte[]{-27, -63}, new byte[]{-115, -77, -33, -52, -90, 125, 118, 113})));
        arrayList.add(new MccEntry(220, StringFog.m5049WWWWWWWW(new byte[]{-42, -12}, new byte[]{-92, -121, 95, -111, 104, -54, -90, -60})));
        arrayList.add(new MccEntry(221, StringFog.m5049WWWWWWWW(new byte[]{34, -105}, new byte[]{90, -4, -36, -98, 99, 84, 102, 32})));
        arrayList.add(new MccEntry(222, StringFog.m5049WWWWWWWW(new byte[]{125, -25}, new byte[]{20, -109, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -72, 40, -23, -125, 38})));
        arrayList.add(new MccEntry(225, StringFog.m5049WWWWWWWW(new byte[]{107, 97}, new byte[]{29, 0, 66, 31, -38, 72, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 115})));
        arrayList.add(new MccEntry(226, StringFog.m5049WWWWWWWW(new byte[]{9, 111}, new byte[]{123, 0, 121, 90, -115, 17, 27, -10})));
        arrayList.add(new MccEntry(228, StringFog.m5049WWWWWWWW(new byte[]{81, -90}, new byte[]{TarConstants.LF_SYMLINK, -50, TarConstants.LF_NORMAL, -109, -61, 68, 19, -63})));
        arrayList.add(new MccEntry(230, StringFog.m5049WWWWWWWW(new byte[]{23, -78}, new byte[]{116, -56, -93, 99, -107, -106, -59, -26})));
        arrayList.add(new MccEntry(231, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_SYMLINK, -104}, new byte[]{65, -13, -113, 0, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -19, -111, -50})));
        arrayList.add(new MccEntry(232, StringFog.m5049WWWWWWWW(new byte[]{-52, -122}, new byte[]{-83, -14, 116, 91, 41, -38, -80, 10})));
        arrayList.add(new MccEntry(234, StringFog.m5049WWWWWWWW(new byte[]{-26, ConstantPoolEntry.CP_InterfaceMethodref}, new byte[]{-127, 105, ConstantPoolEntry.CP_NameAndType, -93, 80, 58, 71, 102})));
        arrayList.add(new MccEntry(235, StringFog.m5049WWWWWWWW(new byte[]{92, -98}, new byte[]{59, -4, 101, -17, 73, 90, -70, -56})));
        arrayList.add(new MccEntry(238, StringFog.m5049WWWWWWWW(new byte[]{-42, -72}, new byte[]{-78, -45, -61, -77, 32, -99, 45, -94})));
        arrayList.add(new MccEntry(240, StringFog.m5049WWWWWWWW(new byte[]{33, -54}, new byte[]{82, -81, -79, -76, -5, -120, 114, 113})));
        arrayList.add(new MccEntry(242, StringFog.m5049WWWWWWWW(new byte[]{-88, 18}, new byte[]{-58, 125, -120, -79, -81, 122, -50, -49})));
        arrayList.add(new MccEntry(244, StringFog.m5049WWWWWWWW(new byte[]{-109, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{-11, 37, -98, 109, 90, Byte.MAX_VALUE, -116, -21})));
        arrayList.add(new MccEntry(246, StringFog.m5049WWWWWWWW(new byte[]{-56, 46}, new byte[]{-92, 90, -14, -101, 67, 41, 41, 104})));
        arrayList.add(new MccEntry(247, StringFog.m5049WWWWWWWW(new byte[]{-104, -121}, new byte[]{-12, -15, -30, -41, 105, -83, -25, 122})));
        arrayList.add(new MccEntry(248, StringFog.m5049WWWWWWWW(new byte[]{40, -66}, new byte[]{TarConstants.LF_MULTIVOLUME, -37, 39, -62, -26, -76, -79, 115})));
        arrayList.add(new MccEntry(250, StringFog.m5049WWWWWWWW(new byte[]{117, 8}, new byte[]{7, 125, 90, -103, 121, -6, TarConstants.LF_GNUTYPE_LONGNAME, -79})));
        arrayList.add(new MccEntry(255, StringFog.m5049WWWWWWWW(new byte[]{62, -3}, new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -100, 89, 125, 90, -124, -106, 1})));
        arrayList.add(new MccEntry(TarConstants.MAGIC_OFFSET, StringFog.m5049WWWWWWWW(new byte[]{-70, 65}, new byte[]{-40, 56, TarConstants.LF_GNUTYPE_SPARSE, 118, 63, 106, TarConstants.LF_CONTIG, 118})));
        arrayList.add(new MccEntry(259, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, -90}, new byte[]{33, -62, -52, 43, 39, 84, ConstantPoolEntry.CP_NameAndType, 84})));
        arrayList.add(new MccEntry(260, StringFog.m5049WWWWWWWW(new byte[]{60, 79}, new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, 35, 118, 22, 122, -2, TarConstants.LF_GNUTYPE_LONGNAME, -71})));
        arrayList.add(new MccEntry(262, StringFog.m5049WWWWWWWW(new byte[]{29, -22}, new byte[]{121, -113, 60, -3, -97, -122, 15, 40})));
        arrayList.add(new MccEntry(266, StringFog.m5049WWWWWWWW(new byte[]{84, -53}, new byte[]{TarConstants.LF_CHR, -94, -123, 68, -84, -52, -1, TarConstants.LF_NORMAL})));
        arrayList.add(new MccEntry(268, StringFog.m5049WWWWWWWW(new byte[]{-43, 57}, new byte[]{-91, TarConstants.LF_MULTIVOLUME, TarConstants.LF_GNUTYPE_SPARSE, 95, 122, -21, -56, 60})));
        arrayList.add(new MccEntry(270, StringFog.m5049WWWWWWWW(new byte[]{-65, 121}, new byte[]{-45, ConstantPoolEntry.CP_NameAndType, -120, 45, -3, 123, 117, 107})));
        arrayList.add(new MccEntry(272, StringFog.m5049WWWWWWWW(new byte[]{113, 47}, new byte[]{24, 74, -16, -92, 91, 78, 16, 67})));
        arrayList.add(new MccEntry(274, StringFog.m5049WWWWWWWW(new byte[]{-21, -24}, new byte[]{-126, -101, -9, 20, 25, 100, -75, 82})));
        arrayList.add(new MccEntry(276, StringFog.m5049WWWWWWWW(new byte[]{-83, 35}, new byte[]{-52, 79, 17, ConstantPoolEntry.CP_InterfaceMethodref, 125, 73, 113, 56})));
        arrayList.add(new MccEntry(278, StringFog.m5049WWWWWWWW(new byte[]{99, -114}, new byte[]{14, -6, -115, -99, 95, -15, TarConstants.LF_GNUTYPE_LONGLINK, -89})));
        arrayList.add(new MccEntry(280, StringFog.m5049WWWWWWWW(new byte[]{-105, -87}, new byte[]{-12, -48, -126, -50, 1, -38, 41, 1})));
        arrayList.add(new MccEntry(282, StringFog.m5049WWWWWWWW(new byte[]{-45, 34}, new byte[]{-76, 71, 39, TarConstants.LF_CONTIG, 57, 115, -6, 107})));
        arrayList.add(new MccEntry(283, StringFog.m5049WWWWWWWW(new byte[]{89, -121}, new byte[]{56, -22, 33, -52, 40, 78, 78, -27})));
        arrayList.add(new MccEntry(284, StringFog.m5049WWWWWWWW(new byte[]{79, 117}, new byte[]{45, 18, ConstantPoolEntry.CP_NameAndType, 27, -57, -100, -52, 5})));
        arrayList.add(new MccEntry(286, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, -31}, new byte[]{67, -109, 93, -4, -17, TarConstants.LF_FIFO, -69, -26})));
        arrayList.add(new MccEntry(288, StringFog.m5049WWWWWWWW(new byte[]{-64, -81}, new byte[]{-90, -64, ConstantPoolEntry.CP_NameAndType, -26, 113, -50, 115, -115})));
        arrayList.add(new MccEntry(289, StringFog.m5049WWWWWWWW(new byte[]{-7, Byte.MIN_VALUE}, new byte[]{-98, -27, 100, -15, 65, -48, 97, TarConstants.LF_PAX_EXTENDED_HEADER_LC})));
        arrayList.add(new MccEntry(290, StringFog.m5049WWWWWWWW(new byte[]{-109, 108}, new byte[]{-12, 0, -15, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -14, -2, 3, -12})));
        arrayList.add(new MccEntry(292, StringFog.m5049WWWWWWWW(new byte[]{67, -98}, new byte[]{TarConstants.LF_NORMAL, -13, -96, 106, -100, TarConstants.LF_LINK, 84, 107})));
        arrayList.add(new MccEntry(293, StringFog.m5049WWWWWWWW(new byte[]{-27, 17}, new byte[]{-106, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 24, 56, 20, 28, 86, 63})));
        arrayList.add(new MccEntry(294, StringFog.m5049WWWWWWWW(new byte[]{-121, -36}, new byte[]{-22, -73, -32, -78, 26, 23, -101, 123})));
        arrayList.add(new MccEntry(295, StringFog.m5049WWWWWWWW(new byte[]{-9, -110}, new byte[]{-101, -5, -100, 118, -83, 43, -73, 43})));
        arrayList.add(new MccEntry(297, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -46}, new byte[]{10, -73, 98, 64, 35, -112, -102, TarConstants.LF_CHR})));
        arrayList.add(new MccEntry(302, StringFog.m5049WWWWWWWW(new byte[]{18, -19}, new byte[]{113, -116, -2, -99, -25, -33, 122, 92})));
        arrayList.add(new MccEntry(308, StringFog.m5049WWWWWWWW(new byte[]{68, 80}, new byte[]{TarConstants.LF_BLK, 61, 40, -115, 24, 46, 39, -8})));
        arrayList.add(new MccEntry(310, StringFog.m5049WWWWWWWW(new byte[]{-94, -111}, new byte[]{-41, -30, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 122, 84, -53, -116, -20})));
        arrayList.add(new MccEntry(311, StringFog.m5049WWWWWWWW(new byte[]{47, 41}, new byte[]{90, 90, 15, 35, 60, -122, -65, 89})));
        arrayList.add(new MccEntry(312, StringFog.m5049WWWWWWWW(new byte[]{24, 63}, new byte[]{109, TarConstants.LF_GNUTYPE_LONGNAME, 33, -11, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 60, 31, 70})));
        arrayList.add(new MccEntry(313, StringFog.m5049WWWWWWWW(new byte[]{-6, 3}, new byte[]{-113, 112, -66, ConstantPoolEntry.CP_NameAndType, 27, -6, -117, -32})));
        arrayList.add(new MccEntry(314, StringFog.m5049WWWWWWWW(new byte[]{66, 71}, new byte[]{TarConstants.LF_CONTIG, TarConstants.LF_BLK, 26, -108, 111, -16, 42, 105})));
        arrayList.add(new MccEntry(315, StringFog.m5049WWWWWWWW(new byte[]{-7, -117}, new byte[]{-116, -8, 46, -15, 84, 100, -101, -49})));
        arrayList.add(new MccEntry(316, StringFog.m5049WWWWWWWW(new byte[]{-127, 13}, new byte[]{-12, 126, 84, -52, -56, -11, -77, -60})));
        arrayList.add(new MccEntry(330, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_NORMAL, -59}, new byte[]{64, -73, -119, 84, -106, 37, -74, 90})));
        arrayList.add(new MccEntry(332, StringFog.m5049WWWWWWWW(new byte[]{87, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{33, 34, 6, 5, -28, 89, TarConstants.LF_BLK, -109})));
        arrayList.add(new MccEntry(334, StringFog.m5049WWWWWWWW(new byte[]{-56, 3}, new byte[]{-91, 123, 64, -82, -94, 38, TarConstants.LF_NORMAL, -27})));
        arrayList.add(new MccEntry(338, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_SYMLINK, -92}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_UC, -55, 47, 112, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 60, 89, -35})));
        arrayList.add(new MccEntry(340, StringFog.m5049WWWWWWWW(new byte[]{82, -119}, new byte[]{TarConstants.LF_DIR, -7, -115, -100, 78, -87, 30, 28})));
        arrayList.add(new MccEntry(342, StringFog.m5049WWWWWWWW(new byte[]{94, -48}, new byte[]{60, -78, -44, -28, -116, 20, -6, -13})));
        arrayList.add(new MccEntry(344, StringFog.m5049WWWWWWWW(new byte[]{115, -62}, new byte[]{18, -91, -11, -41, -38, -7, 34, -6})));
        arrayList.add(new MccEntry(346, StringFog.m5049WWWWWWWW(new byte[]{-3, -51}, new byte[]{-106, -76, -75, -29, -3, -32, 32, 106})));
        arrayList.add(new MccEntry(348, StringFog.m5049WWWWWWWW(new byte[]{61, -113}, new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -24, -116, 60, TarConstants.LF_CONTIG, Byte.MAX_VALUE, -67, -19})));
        arrayList.add(new MccEntry(350, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_SPARSE, -6}, new byte[]{TarConstants.LF_LINK, -105, -38, 119, 70, 118, 117, 59})));
        arrayList.add(new MccEntry(352, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_BLK, -90}, new byte[]{TarConstants.LF_GNUTYPE_SPARSE, -62, TarConstants.LF_LINK, 100, -123, 109, -11, -114})));
        arrayList.add(new MccEntry(354, StringFog.m5049WWWWWWWW(new byte[]{-48, 86}, new byte[]{-67, 37, -47, 39, 124, 124, -26, -126})));
        arrayList.add(new MccEntry(356, StringFog.m5049WWWWWWWW(new byte[]{-85, 1}, new byte[]{-64, 111, 121, 58, 73, -73, 98, 42})));
        arrayList.add(new MccEntry(358, StringFog.m5049WWWWWWWW(new byte[]{-14, TarConstants.LF_NORMAL}, new byte[]{-98, TarConstants.LF_GNUTYPE_SPARSE, -36, TarConstants.LF_NORMAL, -34, -11, 74, 66})));
        arrayList.add(new MccEntry(360, StringFog.m5049WWWWWWWW(new byte[]{86, 112}, new byte[]{32, 19, TarConstants.LF_CONTIG, -13, -4, -40, 45, -27})));
        arrayList.add(new MccEntry(362, StringFog.m5049WWWWWWWW(new byte[]{108, 99}, new byte[]{15, 20, 41, 6, -21, -93, -58, -53})));
        arrayList.add(new MccEntry(363, StringFog.m5049WWWWWWWW(new byte[]{-71, TarConstants.LF_CONTIG}, new byte[]{-40, 64, TarConstants.LF_GNUTYPE_LONGNAME, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 123, 79, -36, -113})));
        arrayList.add(new MccEntry(364, StringFog.m5049WWWWWWWW(new byte[]{43, Byte.MAX_VALUE}, new byte[]{73, ConstantPoolEntry.CP_NameAndType, 38, -80, -123, TarConstants.LF_MULTIVOLUME, 108, -5})));
        arrayList.add(new MccEntry(365, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_BLK, -101}, new byte[]{85, -14, -74, -75, -73, 60, 4, -50})));
        arrayList.add(new MccEntry(366, StringFog.m5049WWWWWWWW(new byte[]{-47, -62}, new byte[]{-75, -81, -120, -64, 38, -87, 7, -47})));
        arrayList.add(new MccEntry(368, StringFog.m5049WWWWWWWW(new byte[]{7, 60}, new byte[]{100, 73, 106, 68, 93, -58, -125, 117})));
        arrayList.add(new MccEntry(370, StringFog.m5049WWWWWWWW(new byte[]{122, 94}, new byte[]{30, TarConstants.LF_LINK, -79, 30, -58, 46, -125, -119})));
        arrayList.add(new MccEntry(372, StringFog.m5049WWWWWWWW(new byte[]{26, -22}, new byte[]{114, -98, -100, -1, 110, -94, -89, 116})));
        arrayList.add(new MccEntry(374, StringFog.m5049WWWWWWWW(new byte[]{87, -53}, new byte[]{35, -65, -36, -108, -54, -77, -15, 95})));
        arrayList.add(new MccEntry(376, StringFog.m5049WWWWWWWW(new byte[]{108, -100}, new byte[]{24, -1, 5, -10, -107, 73, -88, -21})));
        arrayList.add(new MccEntry(400, StringFog.m5049WWWWWWWW(new byte[]{3, -77}, new byte[]{98, -55, 64, -37, 125, 28, 123, 2})));
        arrayList.add(new MccEntry(401, StringFog.m5049WWWWWWWW(new byte[]{0, 70}, new byte[]{107, 60, -64, -36, 119, -125, -46, 39})));
        arrayList.add(new MccEntry(402, StringFog.m5049WWWWWWWW(new byte[]{-86, TarConstants.LF_LINK}, new byte[]{-56, 69, 67, -58, 27, 100, TarConstants.LF_CHR, -48})));
        arrayList.add(new MccEntry(404, StringFog.m5049WWWWWWWW(new byte[]{108, 45}, new byte[]{5, 67, -19, -53, 60, 25, -45, 23})));
        arrayList.add(new MccEntry(405, StringFog.m5049WWWWWWWW(new byte[]{82, 80}, new byte[]{59, 62, -127, -72, -121, TarConstants.LF_LINK, 124, 123})));
        arrayList.add(new MccEntry(406, StringFog.m5049WWWWWWWW(new byte[]{46, 126}, new byte[]{71, 16, -119, -70, 1, -11, -38, 118})));
        arrayList.add(new MccEntry(410, StringFog.m5049WWWWWWWW(new byte[]{122, 21}, new byte[]{10, 126, -115, Byte.MIN_VALUE, 27, 59, 93, -11})));
        arrayList.add(new MccEntry(412, StringFog.m5049WWWWWWWW(new byte[]{20, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{117, 45, 27, -106, -21, -13, 29, -34})));
        arrayList.add(new MccEntry(413, StringFog.m5049WWWWWWWW(new byte[]{-109, -62}, new byte[]{-1, -87, TarConstants.LF_DIR, -101, -9, -29, -125, Byte.MAX_VALUE})));
        arrayList.add(new MccEntry(414, StringFog.m5049WWWWWWWW(new byte[]{-87, -65}, new byte[]{-60, -46, -12, 67, 31, -56, Byte.MIN_VALUE, -95})));
        arrayList.add(new MccEntry(415, StringFog.m5049WWWWWWWW(new byte[]{93, 72}, new byte[]{TarConstants.LF_LINK, 42, -55, 122, 47, 65, -96, -106})));
        arrayList.add(new MccEntry(416, StringFog.m5049WWWWWWWW(new byte[]{-21, 100}, new byte[]{-127, ConstantPoolEntry.CP_InterfaceMethodref, -17, -102, 85, TarConstants.LF_GNUTYPE_LONGNAME, 38, 38})));
        arrayList.add(new MccEntry(417, StringFog.m5049WWWWWWWW(new byte[]{98, -73}, new byte[]{17, -50, -110, 105, -19, -40, 90, TarConstants.LF_BLK})));
        arrayList.add(new MccEntry(418, StringFog.m5049WWWWWWWW(new byte[]{-27, -31}, new byte[]{-116, -112, -4, -67, 60, -85, -57, 85})));
        arrayList.add(new MccEntry(419, StringFog.m5049WWWWWWWW(new byte[]{-46, -51}, new byte[]{-71, -70, 117, -4, Byte.MAX_VALUE, -44, -57, 60})));
        arrayList.add(new MccEntry(UnixStat.DEFAULT_FILE_PERM, StringFog.m5049WWWWWWWW(new byte[]{-65, -89}, new byte[]{-52, -58, 40, -11, 0, -73, 17, -64})));
        arrayList.add(new MccEntry(421, StringFog.m5049WWWWWWWW(new byte[]{14, 4}, new byte[]{119, 97, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 16, 2, 25, -25, 16})));
        arrayList.add(new MccEntry(422, StringFog.m5049WWWWWWWW(new byte[]{-18, -31}, new byte[]{-127, -116, 23, -123, 79, 74, -49, -105})));
        arrayList.add(new MccEntry(423, StringFog.m5049WWWWWWWW(new byte[]{17, -97}, new byte[]{97, -20, 86, 70, 21, -110, -55, TarConstants.LF_GNUTYPE_SPARSE})));
        arrayList.add(new MccEntry(424, StringFog.m5049WWWWWWWW(new byte[]{Byte.MIN_VALUE, -81}, new byte[]{-31, -54, 114, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 58, TarConstants.LF_CHR, 45, -11})));
        arrayList.add(new MccEntry(425, StringFog.m5049WWWWWWWW(new byte[]{72, -8}, new byte[]{33, -108, 57, -15, 89, -68, -106, -39})));
        arrayList.add(new MccEntry(426, StringFog.m5049WWWWWWWW(new byte[]{45, -46}, new byte[]{79, -70, TarConstants.LF_GNUTYPE_LONGNAME, 28, 61, 117, -49, 104})));
        arrayList.add(new MccEntry(427, StringFog.m5049WWWWWWWW(new byte[]{23, -5}, new byte[]{102, -102, -46, 40, -85, -90, -16, -98})));
        arrayList.add(new MccEntry(428, StringFog.m5049WWWWWWWW(new byte[]{-125, TarConstants.LF_DIR}, new byte[]{-18, 91, -52, -57, -120, -43, -82, 71})));
        arrayList.add(new MccEntry(429, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_NORMAL, -81}, new byte[]{94, -33, -29, -120, TarConstants.LF_MULTIVOLUME, -55, -81, -52})));
        arrayList.add(new MccEntry(430, StringFog.m5049WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType, -9}, new byte[]{109, -110, -106, -48, -54, 9, 100, -120})));
        arrayList.add(new MccEntry(431, StringFog.m5049WWWWWWWW(new byte[]{-88, -21}, new byte[]{-55, -114, 82, -49, 73, -127, TarConstants.LF_GNUTYPE_LONGLINK, 17})));
        arrayList.add(new MccEntry(432, StringFog.m5049WWWWWWWW(new byte[]{119, 81}, new byte[]{30, 35, -83, 17, 33, 35, 26, -116})));
        arrayList.add(new MccEntry(434, StringFog.m5049WWWWWWWW(new byte[]{-45, -88}, new byte[]{-90, -46, -30, -44, 116, -75, -90, -11})));
        arrayList.add(new MccEntry(436, StringFog.m5049WWWWWWWW(new byte[]{72, -108}, new byte[]{60, -2, -1, 57, 85, 45, 119, ConstantPoolEntry.CP_NameAndType})));
        arrayList.add(new MccEntry(437, StringFog.m5049WWWWWWWW(new byte[]{67, 102}, new byte[]{40, 1, 7, -1, -41, 72, -113, -114})));
        arrayList.add(new MccEntry(438, StringFog.m5049WWWWWWWW(new byte[]{60, 108}, new byte[]{72, 1, -24, TarConstants.LF_BLK, -65, -114, -27, -6})));
        arrayList.add(new MccEntry(440, StringFog.m5049WWWWWWWW(new byte[]{44, 84}, new byte[]{70, 36, 116, TarConstants.LF_MULTIVOLUME, -46, 17, 2, -47})));
        arrayList.add(new MccEntry(441, StringFog.m5049WWWWWWWW(new byte[]{24, 66}, new byte[]{114, TarConstants.LF_SYMLINK, -40, 69, 22, 105, -121, 45})));
        arrayList.add(new MccEntry(450, StringFog.m5049WWWWWWWW(new byte[]{37, 91}, new byte[]{78, 41, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -47, 94, -74, -33, 89})));
        arrayList.add(new MccEntry(452, StringFog.m5049WWWWWWWW(new byte[]{-59, -126}, new byte[]{-77, -20, 6, -85, 91, -98, -93, -41})));
        arrayList.add(new MccEntry(454, StringFog.m5049WWWWWWWW(new byte[]{-53, 47}, new byte[]{-93, 68, 124, 79, 105, 3, -107, -19})));
        arrayList.add(new MccEntry(455, StringFog.m5049WWWWWWWW(new byte[]{-13, TarConstants.LF_NORMAL}, new byte[]{-98, 95, TarConstants.LF_GNUTYPE_SPARSE, -60, ConstantPoolEntry.CP_NameAndType, -115, 13, -106})));
        arrayList.add(new MccEntry(456, StringFog.m5049WWWWWWWW(new byte[]{81, 14}, new byte[]{58, 102, 29, 42, -78, 100, 7, -86})));
        arrayList.add(new MccEntry(457, StringFog.m5049WWWWWWWW(new byte[]{-89, 13}, new byte[]{-53, 108, 109, 96, -3, -46, -94, TarConstants.LF_CHR})));
        arrayList.add(new MccEntry(460, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_BLK, 113}, new byte[]{87, 31, 43, -113, TarConstants.LF_NORMAL, -47, -106, 117})));
        arrayList.add(new MccEntry(461, StringFog.m5049WWWWWWWW(new byte[]{-93, -30}, new byte[]{-64, -116, -60, -99, -23, -10, 20, -44})));
        arrayList.add(new MccEntry(466, StringFog.m5049WWWWWWWW(new byte[]{111, -19}, new byte[]{27, -102, 79, -87, 85, 126, -89, -7})));
        arrayList.add(new MccEntry(467, StringFog.m5049WWWWWWWW(new byte[]{-59, -33}, new byte[]{-82, -81, 110, 32, 85, -73, -60, 109})));
        arrayList.add(new MccEntry(470, StringFog.m5049WWWWWWWW(new byte[]{86, -29}, new byte[]{TarConstants.LF_BLK, -121, -16, -56, -55, TarConstants.LF_SYMLINK, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 113})));
        arrayList.add(new MccEntry(472, StringFog.m5049WWWWWWWW(new byte[]{-111, -89}, new byte[]{-4, -47, -75, -50, 23, -33, -32, TarConstants.LF_FIFO})));
        arrayList.add(new MccEntry(502, StringFog.m5049WWWWWWWW(new byte[]{-7, 123}, new byte[]{-108, 2, 41, -15, ConstantPoolEntry.CP_NameAndType, -53, -26, -53})));
        arrayList.add(new MccEntry(505, StringFog.m5049WWWWWWWW(new byte[]{-109, -75}, new byte[]{-14, -64, -121, 113, -111, 56, -116, -91})));
        arrayList.add(new MccEntry(510, StringFog.m5049WWWWWWWW(new byte[]{68, 1}, new byte[]{45, 101, -20, -87, -119, -13, 94, 17})));
        arrayList.add(new MccEntry(514, StringFog.m5049WWWWWWWW(new byte[]{-23, -26}, new byte[]{-99, -118, -4, -19, -5, -31, 97, -4})));
        arrayList.add(new MccEntry(515, StringFog.m5049WWWWWWWW(new byte[]{-91, -24}, new byte[]{-43, Byte.MIN_VALUE, TarConstants.LF_LINK, -73, -39, 60, -16, TarConstants.LF_GNUTYPE_SPARSE})));
        arrayList.add(new MccEntry(520, StringFog.m5049WWWWWWWW(new byte[]{106, -118}, new byte[]{30, -30, -104, -68, -52, 86, -31, 20})));
        arrayList.add(new MccEntry(525, StringFog.m5049WWWWWWWW(new byte[]{97, -60}, new byte[]{18, -93, 98, -46, -9, 112, -20, 94})));
        arrayList.add(new MccEntry(528, StringFog.m5049WWWWWWWW(new byte[]{101, -30}, new byte[]{7, -116, 34, -8, -101, -51, 34, TarConstants.LF_MULTIVOLUME})));
        arrayList.add(new MccEntry(530, StringFog.m5049WWWWWWWW(new byte[]{60, -107}, new byte[]{82, -17, -62, -39, -59, -12, -95, -50})));
        arrayList.add(new MccEntry(534, StringFog.m5049WWWWWWWW(new byte[]{-121, 109}, new byte[]{-22, 29, 36, TarConstants.LF_NORMAL, 15, 101, -18, -86})));
        arrayList.add(new MccEntry(535, StringFog.m5049WWWWWWWW(new byte[]{-8, -45}, new byte[]{-97, -90, -125, -54, 27, -122, 23, -83})));
        arrayList.add(new MccEntry(536, StringFog.m5049WWWWWWWW(new byte[]{-99, 5}, new byte[]{-13, 119, 86, 98, -91, 38, -14, TarConstants.LF_CHR})));
        arrayList.add(new MccEntry(537, StringFog.m5049WWWWWWWW(new byte[]{71, -123}, new byte[]{TarConstants.LF_CONTIG, -30, -4, -13, -65, -28, TarConstants.LF_FIFO, -91})));
        arrayList.add(new MccEntry(539, StringFog.m5049WWWWWWWW(new byte[]{62, 100}, new byte[]{74, ConstantPoolEntry.CP_InterfaceMethodref, -52, 29, TarConstants.LF_GNUTYPE_LONGLINK, -47, 42, 15})));
        arrayList.add(new MccEntry(540, StringFog.m5049WWWWWWWW(new byte[]{-116, 7}, new byte[]{-1, 101, TarConstants.LF_BLK, -26, 44, -15, 5, -17})));
        arrayList.add(new MccEntry(541, StringFog.m5049WWWWWWWW(new byte[]{81, TarConstants.LF_PAX_EXTENDED_HEADER_LC}, new byte[]{39, 13, 10, -71, 36, -22, 61, 38})));
        arrayList.add(new MccEntry(542, StringFog.m5049WWWWWWWW(new byte[]{-50, 98}, new byte[]{-88, 8, -82, -91, TarConstants.LF_NORMAL, -38, -14, 57})));
        arrayList.add(new MccEntry(543, StringFog.m5049WWWWWWWW(new byte[]{82, 41}, new byte[]{37, 79, 13, 104, -26, -117, 56, -51})));
        arrayList.add(new MccEntry(544, StringFog.m5049WWWWWWWW(new byte[]{10, -126}, new byte[]{107, -15, -87, -32, -63, -121, 37, -76})));
        arrayList.add(new MccEntry(545, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, -71}, new byte[]{92, -48, 104, 67, -65, -58, -66, -10})));
        arrayList.add(new MccEntry(546, StringFog.m5049WWWWWWWW(new byte[]{-29, 84}, new byte[]{-115, TarConstants.LF_CONTIG, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 24, -28, -60, -118, TarConstants.LF_GNUTYPE_LONGNAME})));
        arrayList.add(new MccEntry(547, StringFog.m5049WWWWWWWW(new byte[]{-37, 38}, new byte[]{-85, 64, 37, -1, -18, -68, 42, 112})));
        arrayList.add(new MccEntry(548, StringFog.m5049WWWWWWWW(new byte[]{4, -54}, new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -95, -22, -97, 72, -29, -35, -127})));
        arrayList.add(new MccEntry(549, StringFog.m5049WWWWWWWW(new byte[]{108, 80}, new byte[]{27, 35, -113, 109, -123, 79, 82, 26})));
        arrayList.add(new MccEntry(550, StringFog.m5049WWWWWWWW(new byte[]{-19, 36}, new byte[]{-117, 73, 119, -7, 2, -120, 68, -25})));
        arrayList.add(new MccEntry(551, StringFog.m5049WWWWWWWW(new byte[]{-103, -89}, new byte[]{-12, -49, -42, 101, -67, 82, -11, -22})));
        arrayList.add(new MccEntry(552, StringFog.m5049WWWWWWWW(new byte[]{-75, -122}, new byte[]{-59, -15, -80, 99, -82, 61, TarConstants.LF_DIR, 70})));
        arrayList.add(new MccEntry(553, StringFog.m5049WWWWWWWW(new byte[]{-84, 15}, new byte[]{-40, 121, 41, -89, -12, 126, 45, 111})));
        arrayList.add(new MccEntry(554, StringFog.m5049WWWWWWWW(new byte[]{-85, -11}, new byte[]{-33, -98, 5, -91, 17, 57, 17, -79})));
        arrayList.add(new MccEntry(555, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, 62}, new byte[]{37, TarConstants.LF_GNUTYPE_LONGLINK, 85, -83, 122, 65, 95, -101})));
        arrayList.add(new MccEntry(602, StringFog.m5049WWWWWWWW(new byte[]{30, 108}, new byte[]{123, ConstantPoolEntry.CP_InterfaceMethodref, -26, ConstantPoolEntry.CP_NameAndType, 114, -61, -120, 109})));
        arrayList.add(new MccEntry(603, StringFog.m5049WWWWWWWW(new byte[]{-102, 26}, new byte[]{-2, 96, 0, -52, -90, 65, -124, -104})));
        arrayList.add(new MccEntry(604, StringFog.m5049WWWWWWWW(new byte[]{56, 112}, new byte[]{85, 17, 108, -59, -124, 14, -65, -97})));
        arrayList.add(new MccEntry(605, StringFog.m5049WWWWWWWW(new byte[]{-39, 65}, new byte[]{-83, 47, -43, -65, 46, -47, 86, -99})));
        arrayList.add(new MccEntry(606, StringFog.m5049WWWWWWWW(new byte[]{94, 89}, new byte[]{TarConstants.LF_SYMLINK, 32, -8, Byte.MAX_VALUE, -24, 9, -101, -5})));
        arrayList.add(new MccEntry(607, StringFog.m5049WWWWWWWW(new byte[]{16, -65}, new byte[]{119, -46, 13, 8, 7, -12, -46, 106})));
        arrayList.add(new MccEntry(608, StringFog.m5049WWWWWWWW(new byte[]{6, -17}, new byte[]{117, -127, -41, -74, 21, -120, -70, 58})));
        arrayList.add(new MccEntry(609, StringFog.m5049WWWWWWWW(new byte[]{-44, -116}, new byte[]{-71, -2, 40, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 7, 112, -55, -108})));
        arrayList.add(new MccEntry(610, StringFog.m5049WWWWWWWW(new byte[]{9, 73}, new byte[]{100, 37, -41, -60, -73, -44, 38, -121})));
        arrayList.add(new MccEntry(611, StringFog.m5049WWWWWWWW(new byte[]{125, -100}, new byte[]{26, -14, -44, 44, -107, -111, -87, 79})));
        arrayList.add(new MccEntry(612, StringFog.m5049WWWWWWWW(new byte[]{-9, -110}, new byte[]{-108, -5, TarConstants.LF_SYMLINK, -62, 92, 101, 69, 71})));
        arrayList.add(new MccEntry(613, StringFog.m5049WWWWWWWW(new byte[]{62, -79}, new byte[]{92, -41, TarConstants.LF_FIFO, -126, 71, 85, -88, 90})));
        arrayList.add(new MccEntry(614, StringFog.m5049WWWWWWWW(new byte[]{41, TarConstants.LF_MULTIVOLUME}, new byte[]{71, 40, -94, 24, 125, 116, 115, 58})));
        arrayList.add(new MccEntry(615, StringFog.m5049WWWWWWWW(new byte[]{81, -94}, new byte[]{37, -59, 28, -73, -89, TarConstants.LF_FIFO, -11, -102})));
        arrayList.add(new MccEntry(616, StringFog.m5049WWWWWWWW(new byte[]{-110, 34}, new byte[]{-16, 72, -87, -116, -57, ConstantPoolEntry.CP_InterfaceMethodref, -77, 71})));
        arrayList.add(new MccEntry(617, StringFog.m5049WWWWWWWW(new byte[]{124, -67}, new byte[]{17, -56, 29, TarConstants.LF_GNUTYPE_SPARSE, TarConstants.LF_SYMLINK, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, ConstantPoolEntry.CP_NameAndType})));
        arrayList.add(new MccEntry(618, StringFog.m5049WWWWWWWW(new byte[]{-53, 92}, new byte[]{-89, 46, 4, 78, -21, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_GNUTYPE_SPARSE, 28})));
        arrayList.add(new MccEntry(619, StringFog.m5049WWWWWWWW(new byte[]{-37, -6}, new byte[]{-88, -106, -102, 38, -45, -50, 96, 34})));
        arrayList.add(new MccEntry(620, StringFog.m5049WWWWWWWW(new byte[]{-94, -112}, new byte[]{-59, -8, 44, 85, -5, -111, -114, 109})));
        arrayList.add(new MccEntry(621, StringFog.m5049WWWWWWWW(new byte[]{-36, -118}, new byte[]{-78, -19, -93, -71, 108, 3, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 86})));
        arrayList.add(new MccEntry(622, StringFog.m5049WWWWWWWW(new byte[]{111, -78}, new byte[]{27, -42, 96, -26, 95, -33, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 40})));
        arrayList.add(new MccEntry(623, StringFog.m5049WWWWWWWW(new byte[]{-58, -31}, new byte[]{-91, -121, 90, -119, 7, -1, 71, TarConstants.LF_NORMAL})));
        arrayList.add(new MccEntry(624, StringFog.m5049WWWWWWWW(new byte[]{94, -57}, new byte[]{61, -86, -10, -34, -37, -61, -21, 28})));
        arrayList.add(new MccEntry(625, StringFog.m5049WWWWWWWW(new byte[]{-23, -45}, new byte[]{-118, -91, 71, -114, -97, 68, 99, 23})));
        arrayList.add(new MccEntry(626, StringFog.m5049WWWWWWWW(new byte[]{-123, 65}, new byte[]{-10, TarConstants.LF_DIR, -25, 110, 22, -13, -2, -86})));
        arrayList.add(new MccEntry(627, StringFog.m5049WWWWWWWW(new byte[]{125, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}, new byte[]{26, 22, -109, -11, -15, -69, 92, 113})));
        arrayList.add(new MccEntry(628, StringFog.m5049WWWWWWWW(new byte[]{-6, 47}, new byte[]{-99, 78, 105, -125, 27, -112, -28, 92})));
        arrayList.add(new MccEntry(629, StringFog.m5049WWWWWWWW(new byte[]{-65, -98}, new byte[]{-36, -7, -68, -32, -124, 6, 56, -83})));
        arrayList.add(new MccEntry(630, StringFog.m5049WWWWWWWW(new byte[]{-76, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}, new byte[]{-41, 3, -108, -7, TarConstants.LF_LINK, 57, 27, -82})));
        arrayList.add(new MccEntry(631, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_LINK, 7}, new byte[]{80, 104, -16, -12, -86, -70, 81, -67})));
        arrayList.add(new MccEntry(632, StringFog.m5049WWWWWWWW(new byte[]{92, 113}, new byte[]{59, 6, -103, -118, -126, 65, 26, -1})));
        arrayList.add(new MccEntry(633, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_DIR, 73}, new byte[]{70, 42, 45, 64, 24, 105, 112, TarConstants.LF_CHR})));
        arrayList.add(new MccEntry(634, StringFog.m5049WWWWWWWW(new byte[]{22, -9}, new byte[]{101, -109, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 92, 118, 119, 59, 72})));
        arrayList.add(new MccEntry(635, StringFog.m5049WWWWWWWW(new byte[]{74, -1}, new byte[]{56, -120, -68, 29, -9, 111, TarConstants.LF_CHR, -71})));
        arrayList.add(new MccEntry(636, StringFog.m5049WWWWWWWW(new byte[]{-92, -92}, new byte[]{-63, -48, -107, 81, -105, -89, 38, -100})));
        arrayList.add(new MccEntry(637, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_CHR, TarConstants.LF_PAX_EXTENDED_HEADER_LC}, new byte[]{64, 23, -81, -9, 64, -102, 19, 14})));
        arrayList.add(new MccEntry(638, StringFog.m5049WWWWWWWW(new byte[]{10, 13}, new byte[]{110, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -122, -103, -116, -74, 9, -61})));
        arrayList.add(new MccEntry(639, StringFog.m5049WWWWWWWW(new byte[]{-95, -42}, new byte[]{-54, -77, -28, 107, TarConstants.LF_BLK, -93, -84, -113})));
        arrayList.add(new MccEntry(640, StringFog.m5049WWWWWWWW(new byte[]{-72, Byte.MAX_VALUE}, new byte[]{-52, 5, 26, 23, -83, 111, -76, -119})));
        arrayList.add(new MccEntry(641, StringFog.m5049WWWWWWWW(new byte[]{78, 102}, new byte[]{59, 1, 65, -77, -84, -83, 41, 28})));
        arrayList.add(new MccEntry(642, StringFog.m5049WWWWWWWW(new byte[]{-101, -7}, new byte[]{-7, -112, ConstantPoolEntry.CP_NameAndType, 95, -80, -121, 58, -8})));
        arrayList.add(new MccEntry(643, StringFog.m5049WWWWWWWW(new byte[]{92, -55}, new byte[]{TarConstants.LF_LINK, -77, 100, 89, -42, -82, -54, 111})));
        arrayList.add(new MccEntry(645, StringFog.m5049WWWWWWWW(new byte[]{-8, -2}, new byte[]{-126, -109, 112, 85, 97, -120, -81, 107})));
        arrayList.add(new MccEntry(646, StringFog.m5049WWWWWWWW(new byte[]{-3, 60}, new byte[]{-112, 91, -67, -55, 18, -45, -45, 87})));
        arrayList.add(new MccEntry(647, StringFog.m5049WWWWWWWW(new byte[]{31, -16}, new byte[]{109, -107, 74, -17, -97, -82, -88, 67})));
        arrayList.add(new MccEntry(648, StringFog.m5049WWWWWWWW(new byte[]{-6, -94}, new byte[]{Byte.MIN_VALUE, -43, -73, -47, 68, 107, 42, TarConstants.LF_PAX_EXTENDED_HEADER_LC})));
        arrayList.add(new MccEntry(649, StringFog.m5049WWWWWWWW(new byte[]{-77, -110}, new byte[]{-35, -13, 106, -28, 126, 65, -32, -10})));
        arrayList.add(new MccEntry(650, StringFog.m5049WWWWWWWW(new byte[]{-12, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{-103, 59, 81, 114, -14, 123, -102, -47})));
        arrayList.add(new MccEntry(651, StringFog.m5049WWWWWWWW(new byte[]{-96, -92}, new byte[]{-52, -41, -71, -76, 41, 80, 105, 67})));
        arrayList.add(new MccEntry(652, StringFog.m5049WWWWWWWW(new byte[]{-39, -48}, new byte[]{-69, -89, TarConstants.LF_CONTIG, 114, 65, -97, 18, -102})));
        arrayList.add(new MccEntry(653, StringFog.m5049WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType, -8}, new byte[]{Byte.MAX_VALUE, -126, 29, 93, -36, 27, -96, -18})));
        arrayList.add(new MccEntry(654, StringFog.m5049WWWWWWWW(new byte[]{-27, 23}, new byte[]{-114, 122, -107, TarConstants.LF_SYMLINK, TarConstants.LF_DIR, -89, -98, -58})));
        arrayList.add(new MccEntry(655, StringFog.m5049WWWWWWWW(new byte[]{-61, 99}, new byte[]{-71, 2, TarConstants.LF_FIFO, -72, -67, 119, -87, -102})));
        arrayList.add(new MccEntry(657, StringFog.m5049WWWWWWWW(new byte[]{-29, 44}, new byte[]{-122, 94, -109, 35, -81, -22, -68, 25})));
        arrayList.add(new MccEntry(658, StringFog.m5049WWWWWWWW(new byte[]{67, -14}, new byte[]{TarConstants.LF_NORMAL, -102, 6, 33, -5, 2, 126, 62})));
        arrayList.add(new MccEntry(659, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, 102}, new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, 21, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 23, -106, 86, -36, 2})));
        arrayList.add(new MccEntry(702, StringFog.m5049WWWWWWWW(new byte[]{2, 19}, new byte[]{96, 105, 42, -112, 30, -26, -22, 31})));
        arrayList.add(new MccEntry(704, StringFog.m5049WWWWWWWW(new byte[]{36, -23}, new byte[]{67, -99, 19, -124, 106, 42, -70, -79})));
        arrayList.add(new MccEntry(706, StringFog.m5049WWWWWWWW(new byte[]{Byte.MIN_VALUE, 72}, new byte[]{-13, 62, -5, 27, -117, 42, -35, -27})));
        arrayList.add(new MccEntry(708, StringFog.m5049WWWWWWWW(new byte[]{-66, -108}, new byte[]{-42, -6, -4, 28, -73, TarConstants.LF_FIFO, 66, ConstantPoolEntry.CP_NameAndType})));
        arrayList.add(new MccEntry(710, StringFog.m5049WWWWWWWW(new byte[]{-49, -65}, new byte[]{-95, -42, 6, -98, -114, -4, -117, -30})));
        arrayList.add(new MccEntry(712, StringFog.m5049WWWWWWWW(new byte[]{69, -117}, new byte[]{38, -7, 7, 42, -40, TarConstants.LF_NORMAL, 74, -48})));
        arrayList.add(new MccEntry(714, StringFog.m5049WWWWWWWW(new byte[]{-64, 18}, new byte[]{-80, 115, 35, 13, -56, -50, -108, -25})));
        arrayList.add(new MccEntry(716, StringFog.m5049WWWWWWWW(new byte[]{41, -55}, new byte[]{89, -84, -55, 5, -90, 63, -76, 125})));
        arrayList.add(new MccEntry(722, StringFog.m5049WWWWWWWW(new byte[]{29, -3}, new byte[]{124, -113, 90, 36, -45, 33, -55, 43})));
        arrayList.add(new MccEntry(724, StringFog.m5049WWWWWWWW(new byte[]{-66, -92}, new byte[]{-36, -42, -54, 9, -65, 94, 107, 8})));
        arrayList.add(new MccEntry(730, StringFog.m5049WWWWWWWW(new byte[]{40, -94}, new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -50, 46, 1, -38, 118, 28, 4})));
        arrayList.add(new MccEntry(732, StringFog.m5049WWWWWWWW(new byte[]{8, -13}, new byte[]{107, -100, 17, 82, -28, 61, -83, -22})));
        arrayList.add(new MccEntry(734, StringFog.m5049WWWWWWWW(new byte[]{-91, -5}, new byte[]{-45, -98, TarConstants.LF_FIFO, -118, -89, -122, -29, 89})));
        arrayList.add(new MccEntry(736, StringFog.m5049WWWWWWWW(new byte[]{-116, 8}, new byte[]{-18, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 78, -59, -91, -2, -109, 87})));
        arrayList.add(new MccEntry(738, StringFog.m5049WWWWWWWW(new byte[]{15, 82}, new byte[]{104, 43, -124, -28, 59, 58, -74, -34})));
        arrayList.add(new MccEntry(740, StringFog.m5049WWWWWWWW(new byte[]{104, 45}, new byte[]{13, 78, -27, -41, -124, 66, 5, -71})));
        arrayList.add(new MccEntry(742, StringFog.m5049WWWWWWWW(new byte[]{61, 40}, new byte[]{90, 78, 64, 102, TarConstants.LF_NORMAL, -30, -21, 13})));
        arrayList.add(new MccEntry(744, StringFog.m5049WWWWWWWW(new byte[]{35, 27}, new byte[]{TarConstants.LF_GNUTYPE_SPARSE, 98, -19, 105, 122, Byte.MIN_VALUE, 71, -25})));
        arrayList.add(new MccEntry(746, StringFog.m5049WWWWWWWW(new byte[]{-83, -33}, new byte[]{-34, -83, -12, 58, -5, 111, 43, -97})));
        arrayList.add(new MccEntry(748, StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, -87}, new byte[]{13, -48, -117, TarConstants.LF_FIFO, -109, -97, 4, 2})));
        arrayList.add(new MccEntry(750, StringFog.m5049WWWWWWWW(new byte[]{-116, -97}, new byte[]{-22, -12, -96, 22, 13, TarConstants.LF_GNUTYPE_LONGLINK, 7, 84})));
        Collections.sort(arrayList);
    }
}
