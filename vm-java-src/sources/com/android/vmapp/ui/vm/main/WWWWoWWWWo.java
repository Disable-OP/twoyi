package com.android.vmapp.ui.vm.main;

import android.app.Application;
import android.content.SharedPreferences;
import androidx.lifecycle.AbstractC1071WWWWWWWW;
import com.android.vmapp.billing.C1603WWWWWWWW;
import com.android.vmapp.billing.InterfaceC1602WWWWWWWW;
import com.google.android.gms.internal.ads.pr0;
import ed.AbstractC2403WWWWoWWWWo;
import hd.AbstractC2817WWWWWWWW;
import hd.C2819WWWWWWWW;
import hd.WW;
import j3.C3164WWWWWWWW;
import java.util.Collections;
import java.util.List;
import k4.C3243WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import org.apache.commons.compress.archivers.cpio.CpioConstants;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
/* renamed from: com.android.vmapp.ui.vm.main.WWWWo̐WWWWoȄ̐  reason: invalid class name */
/* loaded from: classes.dex */
public final class WWWWoWWWWo extends androidx.lifecycle.WWWWWWWW implements InterfaceC1602WWWWWWWW, SharedPreferences.OnSharedPreferenceChangeListener {

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final C2819WWWWWWWW f8678WWWWWWWW;

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public final WW f8679WWWW;

    /* renamed from: com.android.vmapp.ui.vm.main.WWWWo̐WWWWoȄ̐$WWWW̏WWWWβ̏  reason: invalid class name */
    /* loaded from: classes.dex */
    public static final class WWWWWWWW {

        /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
        public final int f8680WWWWoWWWWo;

        /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
        public final boolean f8681WWWWWWWW;

        /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
        public final boolean f8682WWWWWWWW;

        /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
        public final boolean f8683WWWWWWWW;

        /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
        public final boolean f8684WWWWWWWW;

        /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
        public final boolean f8685WWWWWWWW;

        /* renamed from: WWWW֭WWWWྥ֭  reason: contains not printable characters */
        public final List f8686WWWWWWWW;

        /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
        public final int f8687WWWoWWWo;

        /* renamed from: WWWoԻWWWoͷԻ  reason: contains not printable characters */
        public final boolean f8688WWWoWWWo;

        /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
        public final boolean f8689WWoWWo;

        public WWWWWWWW() {
            this(0);
        }

        /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
        public static WWWWWWWW m4995WWWWWWWW(WWWWWWWW wwwwwwww, boolean z10, int i10, int i11, boolean z11, boolean z12, boolean z13, boolean z14, boolean z15, boolean z16, List list, int i12) {
            int i13;
            boolean z17;
            boolean z18;
            boolean z19;
            boolean z20;
            boolean z21;
            boolean z22;
            List list2;
            if ((i12 & 1) != 0) {
                z10 = wwwwwwww.f8681WWWWWWWW;
            }
            boolean z23 = z10;
            if ((i12 & 2) != 0) {
                i10 = wwwwwwww.f8680WWWWoWWWWo;
            }
            int i14 = i10;
            if ((i12 & 4) != 0) {
                i13 = wwwwwwww.f8687WWWoWWWo;
            } else {
                i13 = i11;
            }
            if ((i12 & 8) != 0) {
                z17 = wwwwwwww.f8682WWWWWWWW;
            } else {
                z17 = z11;
            }
            if ((i12 & 16) != 0) {
                z18 = wwwwwwww.f8683WWWWWWWW;
            } else {
                z18 = z12;
            }
            if ((i12 & 32) != 0) {
                z19 = wwwwwwww.f8689WWoWWo;
            } else {
                z19 = z13;
            }
            if ((i12 & 64) != 0) {
                z20 = wwwwwwww.f8684WWWWWWWW;
            } else {
                z20 = z14;
            }
            if ((i12 & 128) != 0) {
                z21 = wwwwwwww.f8685WWWWWWWW;
            } else {
                z21 = z15;
            }
            if ((i12 & CpioConstants.C_IRUSR) != 0) {
                z22 = wwwwwwww.f8688WWWoWWWo;
            } else {
                z22 = z16;
            }
            if ((i12 & 512) != 0) {
                list2 = wwwwwwww.f8686WWWWWWWW;
            } else {
                list2 = list;
            }
            wwwwwwww.getClass();
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            AbstractC3339WWWWWWWW.m15439WWoWWo(list2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{92, 19, -69, -69, 114, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{42, 126, -9, -46, 1, 63, 66, -119}));
            return new WWWWWWWW(z23, i14, i13, z17, z18, z19, z20, z21, z22, list2);
        }

        public final boolean equals(Object obj) {
            if (this == obj) {
                return true;
            }
            if (obj instanceof WWWWWWWW) {
                WWWWWWWW wwwwwwww = (WWWWWWWW) obj;
                return this.f8681WWWWWWWW == wwwwwwww.f8681WWWWWWWW && this.f8680WWWWoWWWWo == wwwwwwww.f8680WWWWoWWWWo && this.f8687WWWoWWWo == wwwwwwww.f8687WWWoWWWo && this.f8682WWWWWWWW == wwwwwwww.f8682WWWWWWWW && this.f8683WWWWWWWW == wwwwwwww.f8683WWWWWWWW && this.f8689WWoWWo == wwwwwwww.f8689WWoWWo && this.f8684WWWWWWWW == wwwwwwww.f8684WWWWWWWW && this.f8685WWWWWWWW == wwwwwwww.f8685WWWWWWWW && this.f8688WWWoWWWo == wwwwwwww.f8688WWWoWWWo && AbstractC3339WWWWWWWW.m15427WWWWWWWW(this.f8686WWWWWWWW, wwwwwwww.f8686WWWWWWWW);
            }
            return false;
        }

        public final int hashCode() {
            int i10;
            int i11;
            int i12;
            int i13;
            int i14;
            int i15;
            int i16 = 1237;
            if (this.f8681WWWWWWWW) {
                i10 = 1231;
            } else {
                i10 = 1237;
            }
            int i17 = ((((i10 * 31) + this.f8680WWWWoWWWWo) * 31) + this.f8687WWWoWWWo) * 31;
            if (this.f8682WWWWWWWW) {
                i11 = 1231;
            } else {
                i11 = 1237;
            }
            int i18 = (i17 + i11) * 31;
            if (this.f8683WWWWWWWW) {
                i12 = 1231;
            } else {
                i12 = 1237;
            }
            int i19 = (i18 + i12) * 31;
            if (this.f8689WWoWWo) {
                i13 = 1231;
            } else {
                i13 = 1237;
            }
            int i20 = (i19 + i13) * 31;
            if (this.f8684WWWWWWWW) {
                i14 = 1231;
            } else {
                i14 = 1237;
            }
            int i21 = (i20 + i14) * 31;
            if (this.f8685WWWWWWWW) {
                i15 = 1231;
            } else {
                i15 = 1237;
            }
            int i22 = (i21 + i15) * 31;
            if (this.f8688WWWoWWWo) {
                i16 = 1231;
            }
            return this.f8686WWWWWWWW.hashCode() + ((i22 + i16) * 31);
        }

        public final String toString() {
            StringBuilder sb2 = new StringBuilder();
            pr0.m9002WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{42, 79, 63, 21, 42, ConstantPoolEntry.CP_NameAndType, 28, -34, 22, 85, 37, 15, 56, ConstantPoolEntry.CP_NameAndType, 24, -104, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 28, 17, 118}, new byte[]{Byte.MAX_VALUE, 38, 108, 97, TarConstants.LF_GNUTYPE_LONGLINK, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 121, -10}, sb2);
            sb2.append(this.f8681WWWWWWWW);
            sb2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{62, -91, 59, TarConstants.LF_GNUTYPE_SPARSE, -27, Byte.MIN_VALUE, -106, -100, 95, -22, TarConstants.LF_CHR, 87, -43, -127, -121, -115, 106, -72}, new byte[]{18, -123, 87, TarConstants.LF_SYMLINK, -100, -17, -29, -24}));
            sb2.append(this.f8680WWWWoWWWWo);
            sb2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{26, -51, 100, 0, -11, 38, -13, -125, 89, -119, 117, 38, -18, 33, -2, -74, ConstantPoolEntry.CP_InterfaceMethodref}, new byte[]{TarConstants.LF_FIFO, -19, 16, 111, Byte.MIN_VALUE, 69, -101, -50}));
            sb2.append(this.f8687WWWoWWWo);
            sb2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{42, 73, TarConstants.LF_FIFO, -120, -87, -93, 60, -91, 113, 36, 41, -98, -87, -24}, new byte[]{6, 105, 70, -6, -52, -43, 85, -64}));
            sb2.append(this.f8682WWWWWWWW);
            sb2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-97, -77, 1, -72, -33, 81, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_GNUTYPE_SPARSE, -60, -34, 30, -82, -33, 97, 0, 87, -57, -26, 3, -81, -2, 78, 22, 70, -33, -14, 8, -81, -34, 26}, new byte[]{-77, -109, 113, -54, -70, 39, 101, TarConstants.LF_FIFO}));
            sb2.append(this.f8683WWWWWWWW);
            sb2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{21, -95, -15, TarConstants.LF_MULTIVOLUME, -18, -120, -55, 4, 78, -17, -43, 81, -14, -105, -50, 4, 87, -25, -24, 74, -17, -127, -23, 86}, new byte[]{57, -127, -127, 56, -126, -28, -115, 107}));
            sb2.append(this.f8689WWoWWo);
            sb2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{13, -21, -2, 34, 97, 42, -16, -77, 85, -10}, new byte[]{33, -53, -100, 69, TarConstants.LF_SYMLINK, 94, -111, -63}));
            sb2.append(this.f8684WWWWWWWW);
            sb2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, 117, -79, 111, 27, -76, -78, -58, -35, 33, -125, 105, 29, -82, -111, -35, -37, 58, -84, 58}, new byte[]{-78, 85, -62, 7, 116, -61, -16, -87}));
            sb2.append(this.f8685WWWWWWWW);
            sb2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{105, 28, 42, -44, 29, -111, 25, -101}, new byte[]{69, 60, 67, -89, TarConstants.LF_GNUTYPE_LONGLINK, -8, 105, -90}));
            sb2.append(this.f8688WWWoWWWo);
            sb2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, 36, 124, -112, TarConstants.LF_BLK, -56, -3, -60, 38}, new byte[]{27, 4, 10, -3, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -95, -114, -80}));
            sb2.append(this.f8686WWWWWWWW);
            sb2.append(')');
            return sb2.toString();
        }

        public WWWWWWWW(boolean z10, int i10, int i11, boolean z11, boolean z12, boolean z13, boolean z14, boolean z15, boolean z16, List list) {
            byte[] bArr = {90, 66, 22, TarConstants.LF_GNUTYPE_SPARSE, -77, -80, -21, -46};
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{44, 47, 90, 58, -64, -60}, bArr);
            this.f8681WWWWWWWW = z10;
            this.f8680WWWWoWWWWo = i10;
            this.f8687WWWoWWWo = i11;
            this.f8682WWWWWWWW = z11;
            this.f8683WWWWWWWW = z12;
            this.f8689WWoWWo = z13;
            this.f8684WWWWWWWW = z14;
            this.f8685WWWWWWWW = z15;
            this.f8688WWWoWWWo = z16;
            this.f8686WWWWWWWW = list;
        }

        /* JADX WARN: Illegal instructions before constructor call */
        /*
            Code decompiled incorrectly, please refer to instructions dump.
        */
        public WWWWWWWW(int i10) {
            this(false, -1, 0, false, false, false, false, false, false, r10);
            List list = Collections.EMPTY_LIST;
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            AbstractC3339WWWWWWWW.m15429WWWWWWWW(list, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-37, 107, 33, 80, -1, -67, -61, TarConstants.LF_CONTIG, -54, 46, Byte.MAX_VALUE, 10, -88, -40}, new byte[]{-66, 6, 81, 36, -122, -15, -86, 68}));
        }
    }

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public WWWWoWWWWo(Application application) {
        super(application);
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(application, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-77, -43, -7}, new byte[]{-46, -91, -119, 90, -99, -53, -63, -67}));
        C2819WWWWWWWW m14471WWWoWWWo = AbstractC2817WWWWWWWW.m14471WWWoWWWo(new WWWWWWWW(0));
        this.f8678WWWWWWWW = m14471WWWoWWWo;
        this.f8679WWWW = new WW(m14471WWWoWWWo);
        C1603WWWWWWWW.f8434WWWoWWWo.m4910WWoWWo(this);
        q3.WWWWoWWWWo.f32526WWWWWWWW.registerOnSharedPreferenceChangeListener(this);
        AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(AbstractC1071WWWWWWWW.m3515WWWWWWWW(this), null, new C3243WWWWWWWW(this, null), 3);
    }

    @Override // androidx.lifecycle.AbstractC1092WWoWWo
    /* renamed from: WWWWϙWWWWეϙ */
    public final void mo3482WWWWWWWW() {
        C1603WWWWWWWW.f8434WWWoWWWo.m4894WWWWWWWWWW(this);
        q3.WWWWoWWWWo.f32526WWWWWWWW.unregisterOnSharedPreferenceChangeListener(this);
    }

    @Override // com.android.vmapp.billing.InterfaceC1602WWWWWWWW
    /* renamed from: WWWȏWWWoನ̑ */
    public final void mo4887WWWoWWWo(boolean z10) {
        C2819WWWWWWWW c2819wwwwwwww;
        Object m14479WWWWWWWW;
        do {
            c2819wwwwwwww = this.f8678WWWWWWWW;
            m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
        } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, WWWWWWWW.m4995WWWWWWWW((WWWWWWWW) m14479WWWWWWWW, false, 0, 0, false, false, false, false, false, C1603WWWWWWWW.f8434WWWoWWWo.m4908WWoWWo(), null, 767)));
    }

    @Override // com.android.vmapp.billing.InterfaceC1602WWWWWWWW
    /* renamed from: WoڄWoᄴڄ */
    public final /* synthetic */ void mo4888WoWo(int i10) {
    }

    @Override // android.content.SharedPreferences.OnSharedPreferenceChangeListener
    public final void onSharedPreferenceChanged(SharedPreferences sharedPreferences, String str) {
        Object m14479WWWWWWWW;
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(sharedPreferences, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-89, -73, 89, -84, -32, TarConstants.LF_GNUTYPE_LONGLINK, -114, -15, -79, -71, 93, -84, -32, 65, -67, -26, -89}, new byte[]{-44, -33, 56, -34, -123, 47, -34, -125}));
        boolean z10 = q3.WWWWoWWWWo.f32526WWWWWWWW.getBoolean(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{84, 32, -15, 109, 8, 93, -46, 30, 123, TarConstants.LF_NORMAL, -5, 100, 35, 105, -34, 9, 123, TarConstants.LF_NORMAL, -13}, new byte[]{36, 82, -108, ConstantPoolEntry.CP_InterfaceMethodref, 87, TarConstants.LF_FIFO, -73, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}), false);
        boolean z11 = q3.WWWWoWWWWo.f32526WWWWWWWW.getBoolean(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-24, 19, -57, -49, -127, -4, 8, -44, -57, 18, -54, -58, -87, -56, 15, -62, -9, 21, -3, -56, -80, -2, 0, -52, -20, 8, -51, -57}, new byte[]{-104, 97, -94, -87, -34, -105, 109, -83}), false);
        C2819WWWWWWWW c2819wwwwwwww = this.f8678WWWWWWWW;
        if (((WWWWWWWW) c2819wwwwwwww.m14479WWWWWWWW()).f8684WWWWWWWW != z10 || ((WWWWWWWW) c2819wwwwwwww.m14479WWWWWWWW()).f8685WWWWWWWW != z11) {
            do {
                m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
            } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, WWWWWWWW.m4995WWWWWWWW((WWWWWWWW) m14479WWWWWWWW, false, 0, 0, false, false, false, z10, z11, false, null, 831)));
        }
    }
}
