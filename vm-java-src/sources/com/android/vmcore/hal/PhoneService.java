package com.android.vmcore.hal;

import android.annotation.SuppressLint;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.telephony.PhoneNumberUtils;
import android.telephony.SmsMessage;
import android.text.TextUtils;
import android.util.Log;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.hal.phone.AdnRecord;
import com.android.vmcore.hal.phone.CallPdu;
import com.android.vmcore.hal.phone.ConvertHelper;
import com.android.vmcore.hal.phone.GsmAlphabet;
import com.android.vmcore.hal.phone.IccIoResult;
import com.android.vmcore.hal.phone.IccUtils;
import com.android.vmcore.hal.phone.SignalStrengthUtils;
import com.android.vmcore.hal.phone.Types;
import com.blankj.utilcode.util.WWWW;
import com.blankj.utilcode.util.WoWo;
import com.google.android.gms.internal.ads.pr0;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import eh.C2467WWWWWWWW;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.LinkedHashSet;
import java.util.Locale;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p000WWWWWWWWWW.WWoWWo;
import vf.AbstractC4470WWWWWWWW;
import x5.WWWWWWWW;
@SuppressLint({"MissingPermission"})
/* loaded from: classes.dex */
public class PhoneService {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final Handler f9069WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final HALManager f9070WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public int f9071WWWWWWWW;

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public int f9072WWWWWWWW;

    /* renamed from: WWWW֭WWWWྥ֭  reason: contains not printable characters */
    public String f9075WWWWWWWW;

    /* renamed from: WWWWآWWWWȫآ  reason: contains not printable characters */
    public final ArrayList f9076WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final VMInstance f9077WWWoWWWo;

    /* renamed from: WWWoԻWWWoͷԻ  reason: contains not printable characters */
    public int f9078WWWoWWWo;

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public int f9080WWWW;

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public boolean f9079WWoWWo = true;

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public int f9073WWWWWWWW = 0;

    /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
    public int f9074WWWWWWWW = 2;

    /* loaded from: classes.dex */
    public static class CellConfig {

        /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
        public String f9083WWWWoWWWWo;

        /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
        public String f9084WWWWWWWW;

        /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
        public int f9085WWWWWWWW;

        /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
        public String f9086WWWWWWWW;

        /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
        public int f9087WWWWWWWW;

        /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
        public int f9088WWWWWWWW;

        /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
        public int f9089WWWoWWWo;

        /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
        public String f9090WWoWWo;
    }

    /* loaded from: classes.dex */
    public static class IccConfig {

        /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
        public String f9091WWWWoWWWWo;

        /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
        public String f9092WWWWWWWW;

        /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
        public String f9093WWWWWWWW;

        /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
        public String f9094WWWWWWWW;

        /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
        public String f9095WWWoWWWo;

        /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
        public String f9096WWoWWo;
    }

    static {
        StringFog.f8859WWWWWWWW.getClass();
        WWWWWWWW.m17835WWWWWWWW(new byte[]{-73, -22, -60, -12, 95, -124, -30, -26, -111, -21, -56, -1}, new byte[]{-25, -126, -85, -102, 58, -41, -121, -108});
    }

    public PhoneService(VMInstance vMInstance, HALManager hALManager) {
        StringFog.f8859WWWWWWWW.getClass();
        this.f9075WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-86, -24, -37, -116, -83}, new byte[]{-8, -83, -102, -56, -12, -95, 24, 6});
        this.f9076WWWWWWWW = new ArrayList();
        this.f9077WWWoWWWo = vMInstance;
        this.f9070WWWWWWWW = hALManager;
        this.f9069WWWWoWWWWo = new Handler(Looper.getMainLooper());
    }

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public static String m5141WWWWWWWWWW(String str) {
        byte[] bArr = {-104, -3, 110, TarConstants.LF_DIR, -105, -119, -107, -99};
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-77, -66, 61, 118, -60, -76, -86}, bArr, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{23, 125, 91, -104, -15, 84, -96, -117, 116, 123, 80, -14}, new byte[]{60, 62, 8, -37, -94, 110, Byte.MIN_VALUE, -93});
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{-104, 113, -13, -4, -22, -95}, new byte[]{-77, TarConstants.LF_SYMLINK, -96, -65, -71, -98, 115, 82}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-32, 39, 8, 46, 58, 69, 62, Byte.MIN_VALUE, -114, 60}, new byte[]{-53, 100, 91, 109, 105, Byte.MAX_VALUE, 30, -56});
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-54, 81, -104, -79, -15, -113}, new byte[]{-31, 18, -53, -14, -94, -78, 15, -42}))) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{74, 102, 0, -19, -114, 58, -120, -66, 46, 119, 119, -120, -102}, new byte[]{97, 37, TarConstants.LF_MULTIVOLUME, -88, -82, Byte.MAX_VALUE, -38, -20});
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static String m5142WWWWoWWWWo(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{95, -65, -6, -46, -84, 37, -65}, new byte[]{116, -4, -71, -102, -17, 24, Byte.MIN_VALUE, -77}, str)) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-22, 87, -38, -80, 0, TarConstants.LF_FIFO}, new byte[]{-63, 20, -103, -8, 67, ConstantPoolEntry.CP_InterfaceMethodref, -10, 6}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_MULTIVOLUME, TarConstants.LF_GNUTYPE_SPARSE, -55, -109, -125, 7, -90, 70, 41, 66, -66, -10, -105}, new byte[]{102, 16, -124, -42, -93, 66, -12, 20});
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-108, TarConstants.LF_SYMLINK, 73, TarConstants.LF_BLK, 33, -79, 98, -77, -16, 35, 62, 81, TarConstants.LF_DIR}, new byte[]{-65, 113, 4, 113, 1, -12, TarConstants.LF_NORMAL, -31});
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static String m5143WWWWWWWW(Object obj) {
        if (obj instanceof Types.CellIdentityGsm) {
            Types.CellIdentityGsm cellIdentityGsm = (Types.CellIdentityGsm) obj;
            StringBuilder sb2 = new StringBuilder();
            sb2.append(cellIdentityGsm.f9174WWWWWWWW);
            pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-22}, new byte[]{-58, -56, -116, 28, 70, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 95, 38}, sb2);
            sb2.append(cellIdentityGsm.f9173WWWWoWWWWo);
            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-61}, new byte[]{-17, -111, 67, 116, -1, -86, -107, -7}));
            Locale locale = Locale.US;
            sb2.append(String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{-64, -83, -116, -38}, new byte[]{-27, -99, -72, -94, 106, 45, TarConstants.LF_SYMLINK, 34}), Integer.valueOf(cellIdentityGsm.f9177WWWoWWWo)));
            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-106}, new byte[]{-70, -105, 65, TarConstants.LF_SYMLINK, TarConstants.LF_GNUTYPE_LONGNAME, -73, -17, -108}));
            sb2.append(String.format(locale, WWWWWWWW.m17835WWWWWWWW(new byte[]{118, 114, -114, 124}, new byte[]{TarConstants.LF_GNUTYPE_SPARSE, 66, -70, 4, -29, -127, 118, -56}), Integer.valueOf(cellIdentityGsm.f9175WWWWWWWW)));
            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-119}, new byte[]{-91, TarConstants.LF_GNUTYPE_LONGNAME, -87, 109, -95, -79, 35, -76}));
            sb2.append(cellIdentityGsm.f9176WWWWWWWW);
            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{14}, new byte[]{34, -4, 74, 62, -80, -23, 78, -50}));
            sb2.append(0);
            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-35}, new byte[]{-15, -62, -44, -60, 112, 63, -71, 97}));
            Types.CellIdentityOperatorNames cellIdentityOperatorNames = cellIdentityGsm.f9178WWoWWo;
            if (cellIdentityOperatorNames != null) {
                sb2.append(cellIdentityOperatorNames.f9195WWWWWWWW);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{74}, new byte[]{102, -125, 32, 29, 93, -59, 126, 58}));
                sb2.append(cellIdentityGsm.f9178WWoWWo.f9194WWWWoWWWWo);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_SYMLINK}, new byte[]{30, -73, 64, -15, 46, -77, -9, TarConstants.LF_CHR}));
            } else {
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{26}, new byte[]{TarConstants.LF_FIFO, 101, ConstantPoolEntry.CP_NameAndType, 122, 82, -77, 15, 126}));
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{32}, new byte[]{ConstantPoolEntry.CP_NameAndType, -95, -41, 87, 119, -61, TarConstants.LF_BLK, 7}));
            }
            sb2.append(FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
            return sb2.toString();
        } else if (obj instanceof Types.CellIdentityWcdma) {
            Types.CellIdentityWcdma cellIdentityWcdma = (Types.CellIdentityWcdma) obj;
            StringBuilder sb3 = new StringBuilder();
            sb3.append(cellIdentityWcdma.f9203WWWWWWWW);
            pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-102}, new byte[]{-74, -95, 85, 1, TarConstants.LF_FIFO, -90, -8, 47}, sb3);
            sb3.append(cellIdentityWcdma.f9202WWWWoWWWWo);
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-24}, new byte[]{-60, 108, -88, -114, -35, -107, 42, 65}));
            Locale locale2 = Locale.US;
            sb3.append(String.format(locale2, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_NORMAL, 28, -100, -30}, new byte[]{21, 44, -88, -102, 57, 15, 72, -34}), Integer.valueOf(cellIdentityWcdma.f9206WWWoWWWo)));
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{68}, new byte[]{104, -1, -117, -53, 80, -5, -34, -40}));
            sb3.append(String.format(locale2, WWWWWWWW.m17835WWWWWWWW(new byte[]{57, -38, -58, -29}, new byte[]{28, -22, -14, -101, -122, -38, 38, -102}), Integer.valueOf(cellIdentityWcdma.f9204WWWWWWWW)));
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-19}, new byte[]{-63, 126, 37, TarConstants.LF_BLK, -10, -54, -115, 113}));
            sb3.append(0);
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-19}, new byte[]{-63, 15, 106, -18, 111, TarConstants.LF_BLK, -81, 64}));
            sb3.append(cellIdentityWcdma.f9205WWWWWWWW);
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{96}, new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, Byte.MIN_VALUE, -49, 121, 2, 30, 31, 0}));
            Types.CellIdentityOperatorNames cellIdentityOperatorNames2 = cellIdentityWcdma.f9207WWoWWo;
            if (cellIdentityOperatorNames2 != null) {
                sb3.append(cellIdentityOperatorNames2.f9195WWWWWWWW);
                sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-54}, new byte[]{-26, 36, -84, 104, -66, TarConstants.LF_MULTIVOLUME, -55, -89}));
                sb3.append(cellIdentityWcdma.f9207WWoWWo.f9194WWWWoWWWWo);
                sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-62}, new byte[]{-18, -66, -25, 81, 68, -124, -35, 80}));
            } else {
                sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{30}, new byte[]{TarConstants.LF_SYMLINK, 59, 106, -64, 4, 125, -67, -52}));
                sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-111}, new byte[]{-67, -21, 125, 116, 30, 24, 61, -44}));
            }
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{90}, new byte[]{118, -104, -54, -110, -125, -105, TarConstants.LF_LINK, 60}));
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-87}, new byte[]{-123, -36, 34, -20, -14, 17, -61, 115}));
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{22}, new byte[]{58, -121, -78, 71, -96, -57, -69, 101}));
            sb3.append(FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
            return sb3.toString();
        } else if (obj instanceof Types.CellIdentityCdma) {
            Types.CellIdentityCdma cellIdentityCdma = (Types.CellIdentityCdma) obj;
            StringBuilder sb4 = new StringBuilder();
            sb4.append(cellIdentityCdma.f9168WWWWWWWW);
            pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{96}, new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, -109, -43, -24, 60, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -10, -12}, sb4);
            sb4.append(cellIdentityCdma.f9167WWWWoWWWWo);
            sb4.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{40}, new byte[]{4, -96, -36, 81, 16, -77, 105, -91}));
            sb4.append(cellIdentityCdma.f9171WWWoWWWo);
            sb4.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-59}, new byte[]{-23, 47, 32, 114, 32, 119, 105, 68}));
            sb4.append(cellIdentityCdma.f9169WWWWWWWW);
            sb4.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{62}, new byte[]{18, 111, 70, -84, -85, 71, TarConstants.LF_CONTIG, -91}));
            sb4.append(cellIdentityCdma.f9170WWWWWWWW);
            sb4.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-55}, new byte[]{-27, TarConstants.LF_GNUTYPE_LONGNAME, -21, 113, -34, -56, ConstantPoolEntry.CP_InterfaceMethodref, 26}));
            Types.CellIdentityOperatorNames cellIdentityOperatorNames3 = cellIdentityCdma.f9172WWoWWo;
            if (cellIdentityOperatorNames3 != null) {
                sb4.append(cellIdentityOperatorNames3.f9195WWWWWWWW);
                sb4.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{123}, new byte[]{87, -102, 39, 19, -108, 115, -23, 85}));
                sb4.append(cellIdentityCdma.f9172WWoWWo.f9194WWWWoWWWWo);
            } else {
                pr0.m9009WWWoWWWo(new byte[]{1}, new byte[]{45, -76, -122, 25, -29, 97, 117, -96}, sb4, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
            }
            return sb4.toString();
        } else if (obj instanceof Types.CellIdentityTdscdma) {
            Types.CellIdentityTdscdma cellIdentityTdscdma = (Types.CellIdentityTdscdma) obj;
            StringBuilder sb5 = new StringBuilder();
            sb5.append(cellIdentityTdscdma.f9197WWWWWWWW);
            pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-50}, new byte[]{-30, -55, -15, -118, -46, -92, 78, TarConstants.LF_CONTIG}, sb5);
            sb5.append(cellIdentityTdscdma.f9196WWWWoWWWWo);
            sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{15}, new byte[]{35, -51, TarConstants.LF_MULTIVOLUME, 46, -18, 27, 67, -55}));
            Locale locale3 = Locale.US;
            sb5.append(String.format(locale3, WWWWWWWW.m17835WWWWWWWW(new byte[]{59, Byte.MIN_VALUE, TarConstants.LF_SYMLINK, 114}, new byte[]{30, -80, 6, 10, 62, 9, -104, -49}), Integer.valueOf(cellIdentityTdscdma.f9200WWWoWWWo)));
            sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{24}, new byte[]{TarConstants.LF_BLK, 34, 96, 36, -53, -77, 57, Byte.MAX_VALUE}));
            sb5.append(String.format(locale3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-25, -59, -116, -102}, new byte[]{-62, -11, -72, -30, 14, 31, 43, -82}), Integer.valueOf(cellIdentityTdscdma.f9198WWWWWWWW)));
            sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-121}, new byte[]{-85, 59, 30, -73, TarConstants.LF_GNUTYPE_LONGNAME, 104, 42, 93}));
            sb5.append(0);
            sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-127}, new byte[]{-83, TarConstants.LF_SYMLINK, 40, TarConstants.LF_NORMAL, 30, TarConstants.LF_LINK, ConstantPoolEntry.CP_NameAndType, -123}));
            sb5.append(cellIdentityTdscdma.f9199WWWWWWWW);
            sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-19}, new byte[]{-63, 89, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -54, 56, 30, 57, TarConstants.LF_NORMAL}));
            Types.CellIdentityOperatorNames cellIdentityOperatorNames4 = cellIdentityTdscdma.f9201WWoWWo;
            if (cellIdentityOperatorNames4 != null) {
                sb5.append(cellIdentityOperatorNames4.f9195WWWWWWWW);
                sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-17}, new byte[]{-61, -93, 2, -64, 112, 124, 95, 107}));
                sb5.append(cellIdentityTdscdma.f9201WWoWWo.f9194WWWWoWWWWo);
                sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{7}, new byte[]{43, -56, 79, -66, 30, -7, TarConstants.LF_CHR, 9}));
            } else {
                sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-44}, new byte[]{-8, -11, 96, ConstantPoolEntry.CP_InterfaceMethodref, 4, -102, -83, -44}));
                sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{13}, new byte[]{33, -9, 121, 7, 16, 80, -105, 94}));
            }
            sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-24}, new byte[]{-60, 86, 80, 119, -4, -121, -84, 20}));
            sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{108}, new byte[]{64, 59, 91, 71, 3, -22, -110, -42}));
            sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{95}, new byte[]{115, -111, -106, 86, 112, 8, -45, -76}));
            sb5.append(FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
            return sb5.toString();
        } else if (obj instanceof Types.CellIdentityLte) {
            Types.CellIdentityLte cellIdentityLte = (Types.CellIdentityLte) obj;
            StringBuilder sb6 = new StringBuilder();
            sb6.append(cellIdentityLte.f9180WWWWWWWW);
            pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-48}, new byte[]{-4, 111, TarConstants.LF_FIFO, -38, -84, ConstantPoolEntry.CP_InterfaceMethodref, 16, -43}, sb6);
            sb6.append(cellIdentityLte.f9179WWWWoWWWWo);
            sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{65}, new byte[]{109, 99, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -107, 39, 60, 46, 13}));
            sb6.append(cellIdentityLte.f9185WWWoWWWo);
            sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-36}, new byte[]{-16, 122, 126, TarConstants.LF_BLK, 0, 43, -13, 91}));
            sb6.append(0);
            sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{81}, new byte[]{125, -9, -67, 65, 62, -124, 89, -98}));
            sb6.append(cellIdentityLte.f9181WWWWWWWW);
            sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{33}, new byte[]{13, 65, 99, TarConstants.LF_DIR, -9, -109, -72, -30}));
            sb6.append(cellIdentityLte.f9182WWWWWWWW);
            sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{95}, new byte[]{115, 28, -124, -113, -24, 90, -118, -64}));
            Types.CellIdentityOperatorNames cellIdentityOperatorNames5 = cellIdentityLte.f9186WWoWWo;
            if (cellIdentityOperatorNames5 != null) {
                sb6.append(cellIdentityOperatorNames5.f9195WWWWWWWW);
                sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_MULTIVOLUME}, new byte[]{97, -27, -2, 99, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 64, 87, -95}));
                sb6.append(cellIdentityLte.f9186WWoWWo.f9194WWWWoWWWWo);
                sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-105}, new byte[]{-69, 69, 28, 34, -7, -97, 8, 7}));
            } else {
                sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CHR}, new byte[]{31, 19, -80, -119, 66, -106, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 59}));
                sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_NORMAL}, new byte[]{28, 73, -89, 87, 58, 96, -51, 87}));
            }
            sb6.append(cellIdentityLte.f9183WWWWWWWW);
            sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{109}, new byte[]{65, 30, 63, 15, -52, 98, 119, -77}));
            sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{108}, new byte[]{64, 121, -30, 93, 3, 59, -87, -25}));
            sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{89}, new byte[]{117, 82, 105, -100, -40, -25, -19, -79}));
            sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CHR}, new byte[]{31, -50, 0, -105, -83, TarConstants.LF_CONTIG, -99, 32}));
            sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{19}, new byte[]{63, 111, -16, -28, -10, -56, 69, 118}));
            LinkedHashSet linkedHashSet = cellIdentityLte.f9184WWWWWWWW;
            if (linkedHashSet != null && !linkedHashSet.isEmpty()) {
                sb6.append(TextUtils.join(WWWWWWWW.m17835WWWWWWWW(new byte[]{19}, new byte[]{111, -1, 20, 110, 122, 8, -62, -108}), cellIdentityLte.f9184WWWWWWWW));
            } else {
                sb6.append(FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
            }
            return sb6.toString();
        } else if (!(obj instanceof Types.CellIdentityNr)) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        } else {
            Types.CellIdentityNr cellIdentityNr = (Types.CellIdentityNr) obj;
            StringBuilder sb7 = new StringBuilder();
            sb7.append(cellIdentityNr.f9188WWWWWWWW);
            pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{112}, new byte[]{92, -50, -90, 67, -106, -11, -11, -92}, sb7);
            sb7.append(cellIdentityNr.f9187WWWWoWWWWo);
            sb7.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{115}, new byte[]{95, -49, 93, 16, 73, -45, 102, 39}));
            sb7.append(cellIdentityNr.f9192WWWoWWWo);
            sb7.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{84}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, -7, -37, -24, -75, -59, 123, 60}));
            sb7.append(0);
            sb7.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{44}, new byte[]{0, -63, -80, -120, 63, -24, -113, 78}));
            sb7.append(cellIdentityNr.f9189WWWWWWWW);
            sb7.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{20}, new byte[]{56, 16, 78, 61, -117, 26, 111, 93}));
            sb7.append(cellIdentityNr.f9190WWWWWWWW);
            sb7.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{26}, new byte[]{TarConstants.LF_FIFO, -31, -73, -52, -72, -6, Byte.MIN_VALUE, TarConstants.LF_LINK}));
            Types.CellIdentityOperatorNames cellIdentityOperatorNames6 = cellIdentityNr.f9193WWoWWo;
            if (cellIdentityOperatorNames6 != null) {
                sb7.append(cellIdentityOperatorNames6.f9195WWWWWWWW);
                sb7.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_DIR}, new byte[]{25, -58, -9, 108, -45, 5, -81, 90}));
                sb7.append(cellIdentityNr.f9193WWoWWo.f9194WWWWoWWWWo);
                sb7.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-25}, new byte[]{-53, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 43, -42, 115, -112, -49, 121}));
            } else {
                sb7.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{28}, new byte[]{TarConstants.LF_NORMAL, 124, -100, -40, 100, -98, -91, -118}));
                sb7.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{106}, new byte[]{70, -56, 61, -99, 66, -94, -45, -61}));
            }
            sb7.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-9}, new byte[]{-37, -66, -56, -54, -125, -101, 57, 34}));
            LinkedHashSet linkedHashSet2 = cellIdentityNr.f9191WWWWWWWW;
            if (linkedHashSet2 != null && !linkedHashSet2.isEmpty()) {
                sb7.append(TextUtils.join(WWWWWWWW.m17835WWWWWWWW(new byte[]{79}, new byte[]{TarConstants.LF_CHR, -125, 80, 91, 96, 96, -94, -106}), cellIdentityNr.f9191WWWWWWWW));
            } else {
                sb7.append(FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
            }
            return sb7.toString();
        }
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static String m5144WWWWWWWW(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{16, -34, TarConstants.LF_GNUTYPE_LONGNAME, 70, -12, -59, -41}, new byte[]{59, -99, 15, 17, -75, -8, -24, -100}, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{95, Byte.MAX_VALUE, 28, -91, TarConstants.LF_BLK, 29, 18, 1, 68, 17, 110, -37}, new byte[]{116, 60, 95, -14, 117, 39, TarConstants.LF_SYMLINK, 41});
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{110, -91, -83, -33, -59, 111}, new byte[]{69, -26, -18, -120, -124, 80, 118, 84}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{63, 71, Byte.MAX_VALUE, 6, 99, TarConstants.LF_NORMAL, 95, 62}, new byte[]{20, 4, 60, 81, 34, 10, Byte.MAX_VALUE, 14});
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, -30, 38, -53, TarConstants.LF_FIFO, 113}, new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -95, 101, -100, 119, TarConstants.LF_GNUTYPE_LONGNAME, -40, -41}))) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_LINK, -87, -54, 72, 118, -12, -97, -48, 85, -72, -67, 45, 98}, new byte[]{26, -22, -121, 13, 86, -79, -51, -126});
    }

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public static String m5145WWWWWWWW(String str) {
        if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-19, -66, 123, -81, -30, 23, 62, -55, -5, -52}, new byte[]{-58, -3, 60, -21, -95, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 112, -99}, str)) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_DIR, -14, -84, -28, 109, -101, -11, -117, 33}, new byte[]{30, -79, -21, -96, 46, -44, -69, -33}))) {
            return String.format(Locale.US, WWWWWWWW.m17835WWWWWWWW(new byte[]{-64, 112, -71, 13, -4, -89, 30, 79, -47, 19, -37, 45, -109, -54, 117, 104, -55, 31, -36, 108, -52, -54, 124, 57, -50, 64, -36, 101, -113, -60, 96}, new byte[]{-21, TarConstants.LF_CHR, -2, 73, -65, -24, 80, 27}), 1, WWWWWWWW.m17835WWWWWWWW(new byte[]{101, 91}, new byte[]{44, ConstantPoolEntry.CP_InterfaceMethodref, -69, 59, -124, 117, -75, -60}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, -18, -48, 57, -109, -116, -70, -104, -110, -14, -42, 57, -124, -114, -72}, new byte[]{-5, -98, -77, 23, -25, -31, -43, -6}), WWWWWWWW.m17835WWWWWWWW(new byte[]{74, 19, -110, -45, TarConstants.LF_CONTIG, -117, 47, -41, 78}, new byte[]{123, 35, -68, -29, 25, -71, 1, -26}));
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-70, -20, -62, 111, 69, 34, -61, -41, -34, -3, -75, 10, 81}, new byte[]{-111, -81, -113, 42, 101, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -111, -123});
    }

    /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
    public static String m5146WWWWWWWW(String str) {
        byte[] bArr = {-52, 9, 107, TarConstants.LF_CONTIG, 123, -109, 82, -90};
        if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-25, 74, 44, 114, 41, -42, 2, -101}, bArr, str)) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{112, 115, -39, 89, 92, -54, -85, TarConstants.LF_GNUTYPE_SPARSE, 20, 98, -82, 60, 72}, new byte[]{91, TarConstants.LF_NORMAL, -108, 28, 124, -113, -7, 1});
    }

    /* renamed from: WWWW֭WWWWྥ֭  reason: contains not printable characters */
    public static String m5147WWWWWWWW(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-74, -86, 71, -100, 95, 38, 107, -87, -84}, new byte[]{-99, -23, 0, -51, 18, 111, 37, -108}, str)) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-46, -112, -78, 56, -127, -80, -32, 28, -74, -127, -59, 93, -107}, new byte[]{-7, -45, -1, 125, -95, -11, -78, 78});
    }

    /* renamed from: WWWWآWWWWȫآ  reason: contains not printable characters */
    public static String m5148WWWWWWWW(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-57, -96, 1, -4, 33, 108, -119, -65, -35}, new byte[]{-20, -29, 70, -83, 115, 41, -40, -126}, str)) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-67, -29, -25, -79, 2, -62, 2, 46, -39, -14, -112, -44, 22}, new byte[]{-106, -96, -86, -12, 34, -121, 80, 124});
    }

    /* renamed from: WWWWዪWWWWͯዪ  reason: contains not printable characters */
    public static int m5149WWWWWWWW(int i10) {
        return (i10 == 14 || i10 == 19) ? 1 : 0;
    }

    /* renamed from: WWWWᏊWWWWటᏊ  reason: contains not printable characters */
    public static boolean m5150WWWWWWWW(String str, SmsMessage smsMessage) {
        byte[] userData;
        byte[] bArr = {36, -122, 117, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 100, 74, -91, 80};
        byte[] bArr2 = {101, -27, 1, TarConstants.LF_LINK, 18, 43, -47, TarConstants.LF_DIR};
        StringFog.f8859WWWWWWWW.getClass();
        WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
        WWWWWWWW.m17835WWWWWWWW(new byte[]{38, 45, 15, -67, -117, 10, -72, 21, 22, 45}, new byte[]{98, 72, 110, -34, -1, 99, -50, 116});
        WWWWWWWW.m17835WWWWWWWW(new byte[]{-87, -65, 86, 116, -91, 112}, new byte[]{-6, -53, TarConstants.LF_CONTIG, 0, -48, 3, -74, 124});
        if (WoWo.m5356WWWW(smsMessage).m5358WWWWoWWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{-108, 86, 41, 70, -11, -43, 36, 65, -86, 108, 40, 106, -32, -42, TarConstants.LF_SYMLINK, 68, -98, 100}, new byte[]{-7, 1, 91, 39, -123, -91, 65, 37})).m5360WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{60, 121, 80, -64, -43, -118, 42, -62, 58, 104, 69, -35, -61, -114, 60, -29, 41}, new byte[]{91, 28, 36, -107, -90, -17, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -122})).f9408WWWWoWWWWo != null && (userData = smsMessage.getUserData()) != null) {
            String str2 = new String(userData, StandardCharsets.UTF_8);
            if (str2.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{92, 32, TarConstants.LF_GNUTYPE_LONGNAME, 113, -30, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 94, 19}, new byte[]{29, 67, 56, 24, -108, 25, 42, 118})) || str2.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, -15, 102, -70, -121, 122, -36, -21, -123, -15}, new byte[]{-15, -108, 7, -39, -13, 19, -86, -118})) || str2.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{-36, -11, -8, 17, 113, -14}, new byte[]{-113, -127, -103, 101, 4, -127, 110, TarConstants.LF_PAX_EXTENDED_HEADER_LC})) || str2.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-49, 31, 86, -23, -69, 16, 21, 10, -76}, new byte[]{-114, 124, 34, Byte.MIN_VALUE, -51, 113, 97, 111})) || str2.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-88, -81, -73, -59, -35, 109, -45, -21, -104, -81, -20}, new byte[]{-20, -54, -42, -90, -87, 4, -91, -118})) || str2.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{69, -98, -44, -106, 34, -118, -118}, new byte[]{22, -22, -75, -30, 87, -7, -80, 125})) || WWWWWWWW.m17835WWWWWWWW(new byte[]{32, -45, -76}, new byte[]{17, -31, -122, 46, 122, -84, -52, -106}).equals(str) || WWWWWWWW.m17835WWWWWWWW(new byte[]{-11, -99, -121}, new byte[]{-57, -81, -76, 116, -117, -77, -39, -126}).equals(str) || WWWWWWWW.m17835WWWWWWWW(new byte[]{-52, -35, -112}, new byte[]{-12, -27, -89, -67, 5, 16, -122, -70}).equals(str)) {
                return true;
            }
            return false;
        }
        return false;
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static String m5151WWWoWWWo(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-62, -84, -61, TarConstants.LF_LINK, -77, 80, 44}, new byte[]{-23, -17, Byte.MIN_VALUE, 121, -4, 109, 19, 0}, str)) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-55, -116, -51, 27, -80, -81}, new byte[]{-30, -49, -114, TarConstants.LF_GNUTYPE_SPARSE, -1, -110, 119, -94}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{72, TarConstants.LF_FIFO, -42, -110, -78, 117, 21, -115, 44, 39, -95, -9, -90}, new byte[]{99, 117, -101, -41, -110, TarConstants.LF_NORMAL, 71, -33});
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-111, -54, -75, 79, -65, -28, -78, 122, -11, -37, -62, 42, -85}, new byte[]{-70, -119, -8, 10, -97, -95, -32, 40});
    }

    /* renamed from: WWWoࠟWWWoॣࠟ  reason: contains not printable characters */
    public static String m5152WWWoWWWo(String str) {
        byte[] bArr = {7, 56, -22, -27, -15, -76, TarConstants.LF_GNUTYPE_SPARSE, -39};
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{44, 123, -89, -96, -76, -119, 108}, bArr, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{0, 105, 13, 46, -26, -59, 74, -81, 27, 7, 113, 66}, new byte[]{43, 42, 64, 107, -93, -1, 106, -121});
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{14, -14, -100, -13, TarConstants.LF_CONTIG, -24}, new byte[]{37, -79, -47, -74, 114, -41, 104, 30}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{23, 65, -86, 93, TarConstants.LF_SYMLINK, 18, -105, -24}, new byte[]{60, 2, -25, 24, 119, 40, -73, -39});
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-4, -35, -125, -91, TarConstants.LF_GNUTYPE_SPARSE, -45}, new byte[]{-41, -98, -50, -32, 22, -18, -54, 113}))) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_BLK, -17, TarConstants.LF_GNUTYPE_LONGLINK, 66, -79, 31, -86, -11, 80, -2, 60, 39, -91}, new byte[]{31, -84, 6, 7, -111, 90, -8, -89});
    }

    /* renamed from: WWWoၙWWWo੮ၙ  reason: contains not printable characters */
    public static String m5153WWWoWWWo(String str) {
        int i10;
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{85, 57, 24, -88, -31, -95}, new byte[]{126, 110, 74, -27, -79, -98, -5, -86}, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{42, -86, -69, 45, -17, ConstantPoolEntry.CP_NameAndType, 47, -18}, new byte[]{1, -3, -23, 96, -65, TarConstants.LF_FIFO, 15, -36});
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{125, 68, 40, 115, 101, -82}, new byte[]{86, 19, 122, 62, TarConstants.LF_DIR, -109, 5, -98}))) {
            try {
                i10 = Integer.parseInt(str.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{62, 110, -34, TarConstants.LF_GNUTYPE_SPARSE, -73, -28}, new byte[]{21, 57, -116, 30, -25, -39, -115, -33}).length()));
            } catch (Exception unused) {
                i10 = -1;
            }
            if (i10 != 0 && i10 != 1 && i10 != 2) {
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{35, -17, 37, 32, -40, -63, 29, 45, 71, -2, 82, 69, -51, -76}, new byte[]{8, -84, 104, 101, -8, -124, 79, Byte.MAX_VALUE});
            }
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-14, 125, 16, -28, 24, -107, -43, 40, -106, 108, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -127, ConstantPoolEntry.CP_NameAndType}, new byte[]{-39, 62, 93, -95, 56, -48, -121, 122});
    }

    /* renamed from: WWWᏛWWW෮Ꮫ  reason: contains not printable characters */
    public static int m5154WWWWWW(int i10) {
        if (i10 == 3 || i10 == 15) {
            return 1;
        }
        switch (i10) {
            case 9:
            case 10:
            case 11:
                return 1;
            default:
                return 0;
        }
    }

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public static String m5155WWoWWo(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{7, -73, -43, 0, 89, -111, -112, 68, 0, -60}, new byte[]{44, -12, -110, 65, 26, -59, -83, 117}, str)) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{-3, -121, -108, -123, 97, TarConstants.LF_NORMAL, 42}, new byte[]{-42, -60, -45, -60, 34, 100, 21, TarConstants.LF_GNUTYPE_SPARSE}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{73, -92, -87, -35, -60, 39, -79, -127, TarConstants.LF_GNUTYPE_SPARSE, -53, -33}, new byte[]{98, -25, -18, -100, -121, 115, -117, -95});
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-92, TarConstants.LF_MULTIVOLUME, -86, -95, -107, -46, -3, 26, -64, 92, -35, -60, -127}, new byte[]{-113, 14, -25, -28, -75, -105, -81, 72});
    }

    /* renamed from: WWoॹWWoࠔॹ  reason: contains not printable characters */
    public static String m5156WWoWWo(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-40, -116, 106, 40, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 29, 35}, new byte[]{-13, -49, 39, 111, 62, 32, 28, 38}, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-60, 23, -22, -59, 34, 105, 93, -73, -33, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -106, -85}, new byte[]{-17, 84, -89, -126, 100, TarConstants.LF_GNUTYPE_SPARSE, 125, -97});
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, -2, 58, -114, -66, -2}, new byte[]{-29, -67, 119, -55, -8, -63, 92, 19}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-32, 42, -86, -34, 15, 19, 41, 123}, new byte[]{-53, 105, -25, -103, 73, 41, 9, TarConstants.LF_GNUTYPE_LONGLINK});
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{111, TarConstants.LF_NORMAL, -39, -114, TarConstants.LF_GNUTYPE_LONGLINK, -90}, new byte[]{68, 115, -108, -55, 13, -101, 84, 114}))) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{109, -1, -114, -115, -113, 31, -69, 33, 9, -18, -7, -24, -101}, new byte[]{70, -68, -61, -56, -81, 90, -23, 115});
    }

    /* renamed from: WWoহWWoȗহ  reason: contains not printable characters */
    public static String m5157WWoWWo(String str) {
        byte[] bArr = {-70, 70, -54, -59, 125, -8, 87, ConstantPoolEntry.CP_NameAndType};
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-111, 5, -121, -118, 57, -59, 104}, bArr, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{101, 33, -4, 6, 65, Byte.MAX_VALUE, TarConstants.LF_GNUTYPE_LONGNAME, -34, 126, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{78, 98, -79, 73, 5, 69, 108, -10});
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{15, -70, 63, 126, -15, 91}, new byte[]{36, -7, 114, TarConstants.LF_LINK, -75, 100, 25, -62}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{8, 58, 8, -55, 30, -45, -35, 112}, new byte[]{35, 121, 69, -122, 90, -23, -3, 64});
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-66, -112, -118, TarConstants.LF_DIR, 35, -105}, new byte[]{-107, -45, -57, 122, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -86, 44, 18}))) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-60, 66, 109, -95, 90, TarConstants.LF_LINK, 118, 45, -96, TarConstants.LF_GNUTYPE_SPARSE, 26, -60, 78}, new byte[]{-17, 1, 32, -28, 122, 116, 36, Byte.MAX_VALUE});
    }

    /* renamed from: WWoၑWWoړၑ  reason: contains not printable characters */
    public static String m5158WWoWWo(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-94, 101, 38, -69, -74, -57, 91}, new byte[]{-119, 38, 115, -24, -14, -6, 100, 2}, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -59, -61, -100, TarConstants.LF_MULTIVOLUME, TarConstants.LF_FIFO, 81, 7, 124, -85, -89, -26}, new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, -122, -106, -49, 9, ConstantPoolEntry.CP_NameAndType, 113, 47});
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{66, 63, 44, 96, -124, -34}, new byte[]{105, 124, 121, TarConstants.LF_CHR, -64, -31, -105, 18}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{95, 33, -3, 100, TarConstants.LF_DIR, -97, 29, 57}, new byte[]{116, 98, -88, TarConstants.LF_CONTIG, 113, -91, 61, 9});
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{117, -72, -72, 87, 100, -64}, new byte[]{94, -5, -19, 4, 32, -3, -7, -99}))) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-1, -107, 118, 116, -96, -48, TarConstants.LF_LINK, 62, -101, -124, 1, 17, -76}, new byte[]{-44, -42, 59, TarConstants.LF_LINK, Byte.MIN_VALUE, -107, 99, 108});
    }

    /* renamed from: WWoዕWWoూዕ  reason: contains not printable characters */
    public static int m5159WWoWWo(int i10) {
        return (i10 == 1 || i10 == 2 || i10 == 16) ? 1 : 0;
    }

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public static String m5160WWWW(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{41, -40, -90, 42, 86, -114, 63}, new byte[]{2, -101, -11, 121, 24, -77, 0, -115}, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{65, 124, 113, -80, 94, -11, 121, -88, 90, 18, 19, -54, 60, -17, 113, -80, 71, 14, ConstantPoolEntry.CP_InterfaceMethodref}, new byte[]{106, 63, 34, -29, 16, -49, 89, Byte.MIN_VALUE});
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{-88, -89, -21, -109, -59, 106}, new byte[]{-125, -28, -72, -64, -117, 85, 0, 34}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-105, 31, -45, -73, -33, -127, -92, 69, -112, 109}, new byte[]{-68, 92, Byte.MIN_VALUE, -28, -111, -69, -124, 117});
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{13, -94, -35, 15, -102, -26}, new byte[]{38, -31, -114, 92, -44, -37, 42, 41}))) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-45, 28, 124, -121, 92, 14, 0, 69, -73, 13, ConstantPoolEntry.CP_InterfaceMethodref, -30, 72}, new byte[]{-8, 95, TarConstants.LF_LINK, -62, 124, TarConstants.LF_GNUTYPE_LONGLINK, 82, 23});
    }

    /* renamed from: WWኮWWႉኮ  reason: contains not printable characters */
    public static int m5161WWWW(int i10) {
        if (i10 == 12 || i10 == 13) {
            return 1;
        }
        switch (i10) {
            case 4:
            case 5:
            case 6:
            case 7:
            case 8:
                return 1;
            default:
                return 0;
        }
    }

    /* renamed from: oેᄈે  reason: contains not printable characters */
    public static String m5162o(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-17, 99, -105, TarConstants.LF_GNUTYPE_SPARSE, -19, 105, 97}, new byte[]{-60, 32, -40, 31, -67, 84, 94, -104}, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-87, 78, 36, -94, -21, -71, 1, 81, -78, 32, 90, -57}, new byte[]{-126, 13, 107, -18, -69, -125, 33, 121});
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{-39, -25, 110, -46, -5, -67}, new byte[]{-14, -92, 33, -98, -85, -126, 122, -114}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-126, 67, -70, 119, -106, -76, 92, 9, -123, TarConstants.LF_LINK}, new byte[]{-87, 0, -11, 59, -58, -114, 124, 57});
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-97, 24, TarConstants.LF_GNUTYPE_LONGNAME, -84, 33, -77}, new byte[]{-76, 91, 3, -32, 113, -114, -98, 67}))) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, 35, -93, -39, -92, -59, -126, 42, -6, TarConstants.LF_SYMLINK, -44, -68, -80}, new byte[]{-75, 96, -18, -100, -124, Byte.MIN_VALUE, -48, TarConstants.LF_PAX_EXTENDED_HEADER_LC});
    }

    /* JADX WARN: Type inference failed for: r1v11, types: [com.android.vmcore.hal.phone.AdnRecord, java.lang.Object] */
    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final String m5163WWWWoWWWWo(String str) {
        IccIoResult iccIoResult;
        byte[] bArr;
        byte b8;
        byte b10;
        boolean m3430WWWWWWWWWW = AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{108, -99, -126, 15, 35, 102, -57}, new byte[]{71, -34, -48, 92, 110, 91, -8, ConstantPoolEntry.CP_NameAndType}, str);
        String str2 = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        if (m3430WWWWWWWWWW) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{35, 41, 69, 109, TarConstants.LF_BLK, -16}, new byte[]{8, 106, 23, 62, 121, -51, 69, 91}))) {
            try {
                String[] split = str.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{15, TarConstants.LF_FIFO, -55, -2, 36, 104}, new byte[]{36, 117, -101, -83, 105, 85, 24, -103}).length()).split(WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType}, new byte[]{32, 106, 63, -61, -17, 59, -126, 98}));
                int parseInt = Integer.parseInt(split[0]);
                int parseInt2 = Integer.parseInt(split[1]);
                int parseInt3 = Integer.parseInt(split[2]);
                int parseInt4 = Integer.parseInt(split[3]);
                int parseInt5 = Integer.parseInt(split[4]);
                if (split.length >= 6) {
                    String str3 = split[5];
                }
                if (split.length >= 7) {
                    String str4 = split[6];
                }
                String str5 = null;
                IccIoResult iccIoResult2 = new IccIoResult(106, 129, null);
                if (parseInt == 192) {
                    if (parseInt2 == 28589) {
                        if (parseInt3 == 0 && parseInt4 == 0) {
                            if (parseInt5 != 15) {
                                iccIoResult2 = new IccIoResult(103, 0, null);
                            } else {
                                iccIoResult = new IccIoResult(144, 0, IccIoResult.m5196WWWWWWWW(28589, 0, 4, 0));
                                iccIoResult2 = iccIoResult;
                            }
                        } else {
                            iccIoResult2 = new IccIoResult(106, 134, null);
                        }
                    } else if (parseInt2 == 12258) {
                        if (parseInt3 == 0 && parseInt4 == 0) {
                            if (parseInt5 != 15) {
                                iccIoResult2 = new IccIoResult(103, 0, null);
                            } else {
                                iccIoResult = new IccIoResult(144, 0, IccIoResult.m5196WWWWWWWW(12258, 0, 10, 0));
                                iccIoResult2 = iccIoResult;
                            }
                        } else {
                            iccIoResult2 = new IccIoResult(106, 134, null);
                        }
                    } else if (parseInt2 == 28480) {
                        if (parseInt3 == 0 && parseInt4 == 0) {
                            if (parseInt5 != 15) {
                                iccIoResult2 = new IccIoResult(103, 0, null);
                            } else {
                                iccIoResult = new IccIoResult(144, 0, IccIoResult.m5196WWWWWWWW(28480, 1, 14, 14));
                                iccIoResult2 = iccIoResult;
                            }
                        } else {
                            iccIoResult2 = new IccIoResult(106, 134, null);
                        }
                    } else if (parseInt2 == 28617) {
                        iccIoResult2 = (parseInt3 == 0 && parseInt4 == 0) ? parseInt5 != 15 ? new IccIoResult(103, 0, null) : new IccIoResult(144, 0, IccIoResult.m5196WWWWWWWW(28617, 1, 4, 4)) : new IccIoResult(106, 134, null);
                    } else if (parseInt2 == 28618) {
                        if (parseInt3 == 0 && parseInt4 == 0) {
                            if (parseInt5 != 15) {
                                iccIoResult2 = new IccIoResult(103, 0, null);
                            } else {
                                iccIoResult = new IccIoResult(144, 0, IccIoResult.m5196WWWWWWWW(28618, 1, 5, 5));
                                iccIoResult2 = iccIoResult;
                            }
                        } else {
                            iccIoResult2 = new IccIoResult(106, 134, null);
                        }
                    } else if (parseInt2 == 28486) {
                        if (parseInt3 == 0 && parseInt4 == 0) {
                            if (parseInt5 != 15) {
                                iccIoResult2 = new IccIoResult(103, 0, null);
                            } else {
                                iccIoResult = new IccIoResult(144, 0, IccIoResult.m5196WWWWWWWW(28486, 0, 17, 0));
                                iccIoResult2 = iccIoResult;
                            }
                        } else {
                            iccIoResult2 = new IccIoResult(106, 134, null);
                        }
                    }
                } else if (parseInt == 176) {
                    if (parseInt2 == 28589) {
                        iccIoResult2 = new IccIoResult(144, 0, new byte[]{0, 0, 0, (byte) m5181WWoWWo().f9092WWWWWWWW.length()});
                    } else if (parseInt2 == 12258) {
                        byte[] bArr2 = new byte[10];
                        String str6 = m5181WWoWWo().f9091WWWWoWWWWo;
                        char[] cArr = IccUtils.f9156WWWWWWWW;
                        for (int i10 = 0; i10 < 10; i10++) {
                            int i11 = i10 * 2;
                            if (i11 < str6.length()) {
                                b8 = IccUtils.m5198WWWWWWWW(str6.charAt(i11));
                            } else {
                                b8 = 15;
                            }
                            int i12 = i11 + 1;
                            if (i12 < str6.length()) {
                                b10 = IccUtils.m5198WWWWWWWW(str6.charAt(i12));
                            } else {
                                b10 = 15;
                            }
                            bArr2[i10] = (byte) (((b10 & 15) << 4) | (b8 & 15));
                        }
                        iccIoResult2 = new IccIoResult(144, 0, bArr2);
                    } else if (parseInt2 == 28486) {
                        IccConfig m5181WWoWWo = m5181WWoWWo();
                        byte[] bArr3 = new byte[17];
                        Arrays.fill(bArr3, (byte) -1);
                        bArr3[0] = 0;
                        byte[] m5197WWWWoWWWWo = IccUtils.m5197WWWWoWWWWo(m5181WWoWWo.f9096WWoWWo);
                        if (m5197WWWWoWWWWo != null) {
                            System.arraycopy(m5197WWWWoWWWWo, 0, bArr3, 1, Math.min(16, m5197WWWWoWWWWo.length));
                        }
                        iccIoResult2 = new IccIoResult(144, 0, bArr3);
                    }
                } else if (parseInt == 178) {
                    if (parseInt2 == 28480) {
                        String str7 = m5181WWoWWo().f9094WWWWWWWW;
                        ?? obj = new Object();
                        obj.f9125WWWW = 0;
                        obj.f9124WWWWWWWW = 0;
                        obj.f9122WWWWoWWWWo = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
                        obj.f9121WWWWWWWWWW = str7;
                        obj.f9123WWWWWWWW = null;
                        byte[] bArr4 = new byte[14];
                        for (int i13 = 0; i13 < 14; i13++) {
                            bArr4[i13] = -1;
                        }
                        String str8 = obj.f9121WWWWWWWWWW;
                        boolean isEmpty = TextUtils.isEmpty(str8);
                        String str9 = AdnRecord.f9120WWoWWo;
                        if (isEmpty) {
                            StringFog.f8859WWWWWWWW.getClass();
                            Log.w(str9, WWWWWWWW.m17835WWWWWWWW(new byte[]{20, 68, TarConstants.LF_FIFO, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -83, -90, -2, -6, 33, 117, TarConstants.LF_CONTIG, 99, -88, -84, -40, -61, 111, 99, 46, 97, -75, -69, -97, -6, 38, 71, 47, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -81, -91, -97, -16, 58, TarConstants.LF_GNUTYPE_LONGLINK, 33, 116, -77}, new byte[]{79, 38, 67, 17, -63, -62, -65, -98}));
                        } else {
                            if (str8.length() > 20) {
                                byte[] bArr5 = {-79, 37, 121, -113, -81, 121, 21, -73, -124, 20, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -108, -86, 115, TarConstants.LF_CHR, -114, -54, 10, 109, -98, -29, 113, TarConstants.LF_LINK, -67, -115, TarConstants.LF_CHR, 100, -58, -84, 123, 116, -73, -125, 38, 96, -113, -83, 122, 116, -67, -97, 42, 110, -125, -79, 61, 61, -96, -54, 117, 60};
                                byte[] bArr6 = {-22, 71, ConstantPoolEntry.CP_NameAndType, -26, -61, 29, 84, -45};
                                StringFog.f8859WWWWWWWW.getClass();
                                Log.w(str9, WWWWWWWW.m17835WWWWWWWW(bArr5, bArr6));
                            } else {
                                String str10 = obj.f9122WWWWoWWWWo;
                                if (!TextUtils.isEmpty(str10)) {
                                    bArr = GsmAlphabet.m5194WWWWoWWWWo(str10);
                                } else {
                                    bArr = new byte[0];
                                }
                                if (bArr.length > 0) {
                                    byte[] bArr7 = {87, -37, 92, -113, -48, -13, 42, 109, 98, -22, 93, -108, -43, -7, ConstantPoolEntry.CP_NameAndType, 84, 44, -12, 72, -98, -100, -5, 14, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 107, -51, 65, -58, -45, -15, TarConstants.LF_GNUTYPE_LONGLINK, 125, 109, -34, 9, -113, -49, -73};
                                    byte[] bArr8 = {ConstantPoolEntry.CP_NameAndType, -71, 41, -26, -68, -105, 107, 9};
                                    StringFog.f8859WWWWWWWW.getClass();
                                    Log.w(str9, WWWWWWWW.m17835WWWWWWWW(bArr7, bArr8).concat("0"));
                                } else {
                                    byte[] numberToCalledPartyBCD = PhoneNumberUtils.numberToCalledPartyBCD(str8);
                                    if (numberToCalledPartyBCD != null) {
                                        System.arraycopy(numberToCalledPartyBCD, 0, bArr4, 1, numberToCalledPartyBCD.length);
                                        bArr4[0] = (byte) numberToCalledPartyBCD.length;
                                        bArr4[12] = -1;
                                        bArr4[13] = -1;
                                        if (bArr.length > 0) {
                                            System.arraycopy(bArr, 0, bArr4, 0, bArr.length);
                                        }
                                    }
                                }
                            }
                            bArr4 = null;
                        }
                        iccIoResult2 = new IccIoResult(144, 0, bArr4);
                    } else {
                        if (parseInt2 == 28617) {
                            iccIoResult = new IccIoResult(144, 0, new byte[]{0, 0, 0, 0});
                        } else if (parseInt2 == 28618) {
                            iccIoResult = new IccIoResult(144, 0, new byte[]{0, 0, 0, 0, 0});
                        }
                        iccIoResult2 = iccIoResult;
                    }
                }
                byte[] bArr9 = iccIoResult2.f9155WWWoWWWo;
                if (bArr9 != null) {
                    str5 = WWWW.m5319WWWWoWWWWo(bArr9, false);
                }
                StringBuilder sb2 = new StringBuilder();
                pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{86, -101, -65, -67, 110, 1, -10}, new byte[]{125, -40, -19, -18, 35, 59, -42, 106}, sb2);
                sb2.append(iccIoResult2.f9154WWWWWWWW);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-105}, new byte[]{-69, 45, 69, 67, -93, 61, -10, TarConstants.LF_SYMLINK}));
                sb2.append(iccIoResult2.f9153WWWWoWWWWo);
                if (str5 != null) {
                    str2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-97}, new byte[]{-77, 114, -102, Byte.MAX_VALUE, -70, -9, -96, 125}).concat(str5);
                }
                sb2.append(str2);
                return sb2.toString();
            } catch (Exception unused) {
                byte[] bArr10 = {-107, 111, 69, -21, -79, -59, -77, 101, -15, 126, TarConstants.LF_SYMLINK, -114, -92, -80};
                byte[] bArr11 = {-66, 44, 8, -82, -111, Byte.MIN_VALUE, -31, TarConstants.LF_CONTIG};
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(bArr10, bArr11);
            }
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, -34, 70, 45, -69, 80, -100, 70, TarConstants.LF_GNUTYPE_SPARSE, -49, TarConstants.LF_LINK, 72, -81}, new byte[]{28, -99, ConstantPoolEntry.CP_InterfaceMethodref, 104, -101, 21, -50, 20});
    }

    /* renamed from: WWWWoᆣWWWWoϿᆣ  reason: contains not printable characters */
    public final int m5164WWWWoWWWWo(boolean z10) {
        int m5193WWWWWWWW = ConvertHelper.m5193WWWWWWWW(this.f9077WWWoWWWo.f8937WWWoWWWo.f8862WWWWoWWWWo);
        if (!z10) {
            if (m5193WWWWWWWW == 16 || m5193WWWWWWWW == 4 || m5193WWWWWWWW == 5) {
                return 0;
            }
        } else if (m5159WWoWWo(m5193WWWWWWWW) == 1) {
            return 16;
        } else {
            if (m5154WWWWWW(m5193WWWWWWWW) == 1) {
                return 3;
            }
            if (m5193WWWWWWWW == 7 || m5193WWWWWWWW == 8 || m5193WWWWWWWW == 12) {
                return 6;
            }
        }
        return m5193WWWWWWWW;
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public final String m5165WWWWWWWW(String str) {
        int i10;
        int i11;
        int i12 = -1;
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{105, 81, -24, -57, -100, 38, -37}, new byte[]{66, 18, -82, -110, -46, 27, -28, -121}, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-45, -23, 68, -64, -77, 41, 65, -124, -56, -121, TarConstants.LF_FIFO, -68, -47, TarConstants.LF_CHR, 73, -100, -43, -101, 43}, new byte[]{-8, -86, 2, -107, -3, 19, 97, -84});
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{66, -88, -1, -46, 113, -4}, new byte[]{105, -21, -71, -121, 63, -61, 57, 102}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{42, 39, 99, -31, 29, 68, 47}, new byte[]{1, 100, 37, -76, TarConstants.LF_GNUTYPE_SPARSE, 126, 15, 105}) + this.f9078WWWoWWWo;
        } else if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-109, -9, 73, 87, -23, -109}, new byte[]{-72, -76, 15, 2, -89, -82, -108, 14}))) {
            try {
                String[] split = str.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{25, 100, 29, 74, -70, ConstantPoolEntry.CP_InterfaceMethodref}, new byte[]{TarConstants.LF_SYMLINK, 39, 91, 31, -12, TarConstants.LF_FIFO, -114, -48}).length()).split(WWWWWWWW.m17835WWWWWWWW(new byte[]{19}, new byte[]{63, 18, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 8, 110, -19, 123, 102}));
                int parseInt = Integer.parseInt(split[0]);
                if (split.length > 1) {
                    i11 = Integer.parseInt(split[1]);
                } else {
                    i11 = 0;
                }
                i10 = i11;
                i12 = parseInt;
            } catch (Exception unused) {
                i10 = -1;
            }
            if (i12 >= 0 && i12 <= 4 && i10 >= 0 && i10 <= 1) {
                this.f9078WWWoWWWo = i12;
                return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
            }
            StringFog.f8859WWWWWWWW.getClass();
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{82, -21, -71, 42, -23, -9, -29, -30, TarConstants.LF_FIFO, -6, -50, 79, -4, -126}, new byte[]{121, -88, -12, 111, -55, -78, -79, -80});
        } else {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{35, Byte.MIN_VALUE, -112, 25, -124, -106, -112, 10, 71, -111, -25, 124, -112}, new byte[]{8, -61, -35, 92, -92, -45, -62, TarConstants.LF_PAX_EXTENDED_HEADER_UC});
        }
    }

    /* renamed from: WWWWܬWWWWೖܬ  reason: contains not printable characters */
    public final String m5166WWWWWWWW(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-27, 59, -32, -105, -42, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -66}, new byte[]{-50, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -87, -38, -97, 90, -127, -56}, str)) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{35, -44, -44, 105, -38}, new byte[]{8, -105, -99, 36, -109, -2, 96, -103}))) {
            return this.f9077WWWoWWWo.f8937WWWoWWWo.f8906WWoWWo;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-1, 63, -59, 33, -27, 9, -89, 96, -101, 46, -78, 68, -15}, new byte[]{-44, 124, -120, 100, -59, TarConstants.LF_GNUTYPE_LONGNAME, -11, TarConstants.LF_SYMLINK});
    }

    /* renamed from: WWWWެWWWWܕެ  reason: contains not printable characters */
    public final String m5167WWWWWWWW(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{37, 98, -98, -120, -72, -114, TarConstants.LF_BLK}, new byte[]{14, 33, -46, -53, -5, -77, ConstantPoolEntry.CP_InterfaceMethodref, -119}, str)) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{94, -19, 1, -110, 100}, new byte[]{117, -82, TarConstants.LF_MULTIVOLUME, -47, 39, -33, -49, -42}))) {
            ArrayList arrayList = new ArrayList(this.f9076WWWWWWWW);
            StringBuilder sb2 = new StringBuilder();
            int size = arrayList.size();
            int i10 = 0;
            while (i10 < size) {
                Object obj = arrayList.get(i10);
                i10++;
                CallPdu callPdu = (CallPdu) obj;
                pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{TarConstants.LF_DIR, 59, -63, -80, 45, -12, -96}, new byte[]{30, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -115, -13, 110, -50, Byte.MIN_VALUE, -106}, sb2);
                sb2.append(callPdu.f9127WWWWWWWW);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-100}, new byte[]{-80, 42, 87, 62, ConstantPoolEntry.CP_NameAndType, 61, 63, 21}));
                sb2.append(callPdu.f9126WWWWoWWWWo);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{97}, new byte[]{TarConstants.LF_MULTIVOLUME, -43, 30, TarConstants.LF_GNUTYPE_LONGLINK, -10, 73, 82, 85}));
                sb2.append(callPdu.f9130WWWoWWWo);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{56}, new byte[]{20, 45, 104, -29, 105, -84, -109, 116}));
                sb2.append(0);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-27}, new byte[]{-55, -44, 31, -47, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -14, -7, -52}));
                sb2.append(0);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{125}, new byte[]{81, -31, -115, 101, -9, TarConstants.LF_SYMLINK, 60, 96}));
                sb2.append(callPdu.f9128WWWWWWWW);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{10}, new byte[]{38, -88, -80, 16, ConstantPoolEntry.CP_NameAndType, 94, 122, 20}));
                if (callPdu.f9128WWWWWWWW.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-3}, new byte[]{-42, -126, -95, TarConstants.LF_GNUTYPE_LONGLINK, 43, -120, -45, -89}))) {
                    sb2.append(145);
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-109}, new byte[]{-65, -30, 8, 124, -118, -56, 45, 126}));
                } else {
                    sb2.append(129);
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CONTIG}, new byte[]{27, TarConstants.LF_CONTIG, 9, 37, -49, -58, 21, 42}));
                }
                sb2.append(callPdu.f9129WWWWWWWW);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{30}, new byte[]{TarConstants.LF_SYMLINK, 20, TarConstants.LF_LINK, 65, -95, 110, -65, -24}));
                sb2.append(callPdu.f9131WWoWWo);
                pr0.m9009WWWoWWWo(new byte[]{-125}, new byte[]{-81, 121, 21, TarConstants.LF_GNUTYPE_LONGNAME, 13, 25, 87, -110}, sb2, "0\r");
            }
            if (sb2.length() != 0) {
                sb2.deleteCharAt(sb2.length() - 1);
            }
            return sb2.toString();
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{68, -125, -53, -112, 31, -27, -81, -125, 32, -110, -68, -11, ConstantPoolEntry.CP_InterfaceMethodref}, new byte[]{111, -64, -122, -43, 63, -96, -3, -47});
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Type inference failed for: r1v6, types: [com.android.vmcore.event.SendSmsEvent, java.lang.Object] */
    /* renamed from: WWWWॾWWWWȏॾ  reason: contains not printable characters */
    public final String m5168WWWWWWWW(String str) {
        byte[] bArr;
        SmsMessage createFromPdu;
        boolean m3430WWWWWWWWWW = AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-82, -95, 118, -106, -22, -20, -64}, new byte[]{-123, -30, 59, -47, -71, -47, -1, -104}, str);
        String str2 = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        if (m3430WWWWWWWWWW) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-1, -37, 113, 13, 46, -9}, new byte[]{-44, -104, 60, 74, 125, -54, -75, -24}))) {
            try {
                String[] split = str.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{73, -94, 34, 115, 111, 16}, new byte[]{98, -31, 111, TarConstants.LF_BLK, 60, 45, -120, -12}).length()).split(WWWWWWWW.m17835WWWWWWWW(new byte[]{1}, new byte[]{45, 43, -117, 112, -39, 59, -94, -89}));
                Integer.parseInt(split[0]);
                String str3 = split[1];
                char[] cArr = IccUtils.f9156WWWWWWWW;
                if (str3 == null) {
                    bArr = null;
                } else {
                    int length = str3.length();
                    byte[] bArr2 = new byte[length / 2];
                    for (int i10 = 0; i10 < length; i10 += 2) {
                        bArr2[i10 / 2] = (byte) ((IccUtils.m5199WWWoWWWo(str3.charAt(i10)) << 4) | IccUtils.m5199WWWoWWWo(str3.charAt(i10 + 1)));
                    }
                    bArr = bArr2;
                }
                if (Build.VERSION.SDK_INT >= 23) {
                    byte[] bArr3 = {-71, -124, -87, -109, 26, -112, -57, ConstantPoolEntry.CP_InterfaceMethodref};
                    StringFog.f8859WWWWWWWW.getClass();
                    createFromPdu = SmsMessage.createFromPdu(bArr, WWWWWWWW.m17835WWWWWWWW(new byte[]{-118, -29, -39, -29}, bArr3));
                } else {
                    createFromPdu = SmsMessage.createFromPdu(bArr);
                }
                WoWo m5356WWWW = WoWo.m5356WWWW(createFromPdu);
                StringFog.f8859WWWWWWWW.getClass();
                String str4 = (String) m5356WWWW.m5360WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{-114, 35, 16, 90, 57, -57, -42, 63, Byte.MIN_VALUE, 35, 10, 124, 29, -64, -37, 61, -116, TarConstants.LF_DIR, 23}, new byte[]{-23, 70, 100, 8, 92, -92, -65, 79})).f9408WWWWoWWWWo;
                if (str4 == null) {
                    str4 = createFromPdu.getOriginatingAddress();
                }
                if (str4 != null) {
                    str2 = str4;
                }
                if (!TextUtils.isEmpty(str2)) {
                    String formatNumberToE164 = PhoneNumberUtils.formatNumberToE164(str2, Locale.getDefault().getCountry());
                    if (!TextUtils.isEmpty(formatNumberToE164)) {
                        str2 = formatNumberToE164;
                    }
                }
                String messageBody = createFromPdu.getMessageBody();
                if (!m5150WWWWWWWW(str2, createFromPdu)) {
                    C2467WWWWWWWW c2467wwwwwwww = this.f9077WWWoWWWo.f8939WWWoWWWo;
                    ?? obj = new Object();
                    obj.f9010WWWWWWWW = str2;
                    obj.f9009WWWWoWWWWo = messageBody;
                    c2467wwwwwwww.m13940WWWWWWWW(obj);
                }
            } catch (Exception e10) {
                e10.printStackTrace();
            }
            byte[] bArr4 = {-57, 26, 69, TarConstants.LF_SYMLINK, 85, -76, -95, -51};
            StringFog.f8859WWWWWWWW.getClass();
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-20, 89, 8, 117, 6, -114, -127, -3}, bArr4);
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-5, -83, -11, -12, -123, -99, -113, 58, -97, -68, -126, -111, -111}, new byte[]{-48, -18, -72, -79, -91, -40, -35, 104});
    }

    /* renamed from: WWWWമWWWWုമ  reason: contains not printable characters */
    public final String m5169WWWWWWWW(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{95, Byte.MAX_VALUE, -98, 89, -42, -29, 73}, new byte[]{116, 60, -50, 16, -104, -34, 118, 69}, str)) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_MULTIVOLUME, -88, -30, -53, TarConstants.LF_BLK, 27}, new byte[]{102, -21, -78, -126, 122, 36, 74, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}))) {
            if (this.f9077WWWoWWWo.f8937WWWoWWWo.f8873WWWWWWWW) {
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{67, 8, 24, -78, -71, 74, 116}, new byte[]{104, TarConstants.LF_GNUTYPE_LONGLINK, 72, -5, -9, 112, 84, 100}) + this.f9075WWWWWWWW;
            }
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{27, 71, 125, 37, -77, 15, -14, -65, Byte.MAX_VALUE, 86, 10, 64, -94, 122}, new byte[]{TarConstants.LF_NORMAL, 4, TarConstants.LF_NORMAL, 96, -109, 74, -96, -19});
        } else if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_DIR, Byte.MAX_VALUE, -126, -5, 9, 74}, new byte[]{30, 60, -46, -78, 71, 119, 93, -79}))) {
            try {
                String[] split = str.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType, 118, 123, -40, TarConstants.LF_CHR, -72}, new byte[]{39, TarConstants.LF_DIR, 43, -111, 125, -123, -18, 74}).length()).split(WWWWWWWW.m17835WWWWWWWW(new byte[]{74}, new byte[]{102, -8, -70, -100, -54, -120, -5, TarConstants.LF_LINK}));
                String str2 = split[0];
                if (split.length > 1) {
                    String str3 = split[1];
                }
            } catch (Exception unused) {
            }
            if (!AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{41, Byte.MIN_VALUE, TarConstants.LF_GNUTYPE_LONGLINK, 113, -6}, new byte[]{123, -59, 10, TarConstants.LF_DIR, -93, -82, 42, TarConstants.LF_BLK}, this.f9075WWWWWWWW)) {
                this.f9075WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{106, -74, -110, -17, -11}, new byte[]{56, -13, -45, -85, -84, -110, -72, -61});
                return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
            }
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-53, TarConstants.LF_NORMAL, -105, -30, -93, 95, 107, 40, -81, 33, -32, -121, -80}, new byte[]{-32, 115, -38, -89, -125, 26, 57, 122});
        } else {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{95, -48, 74, 92, -26, 0, -12, 92, 59, -63, 61, 57, -14}, new byte[]{116, -109, 7, 25, -58, 69, -90, 14});
        }
    }

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final String m5170WWWWWWWW(String str) {
        boolean z10;
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{108, 0, 24, -94}, new byte[]{71, 67, TarConstants.LF_GNUTYPE_LONGLINK, -13, 23, 70, -17, TarConstants.LF_MULTIVOLUME}, str)) {
            VMConfig vMConfig = this.f9077WWWoWWWo.f8937WWWoWWWo;
            String str2 = vMConfig.f8860WWWWWWWWWW;
            int m5193WWWWWWWW = ConvertHelper.m5193WWWWWWWW(vMConfig.f8862WWWWoWWWWo);
            Types.SignalStrength signalStrength = new Types.SignalStrength();
            if (m5159WWoWWo(m5193WWWWWWWW) != 1 && m5154WWWWWW(m5193WWWWWWWW) != 1 && m5193WWWWWWWW != 20) {
                Types.GsmSignalStrength gsmSignalStrength = new Types.GsmSignalStrength();
                gsmSignalStrength.f9214WWWWWWWW = 99;
                gsmSignalStrength.f9213WWWWoWWWWo = 99;
                signalStrength.f9230WWWWWWWW = gsmSignalStrength;
            } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{59, 5, TarConstants.LF_LINK, TarConstants.LF_LINK, 104}, new byte[]{92, 119, 84, 80, 28, -123, 23, 44}).equals(str2)) {
                Types.GsmSignalStrength gsmSignalStrength2 = new Types.GsmSignalStrength();
                gsmSignalStrength2.f9214WWWWWWWW = SignalStrengthUtils.m5200WWWWWWWW(-89) + 1;
                gsmSignalStrength2.f9213WWWWoWWWWo = 0;
                signalStrength.f9230WWWWWWWW = gsmSignalStrength2;
            } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, -51, -61, 18, 7, -17, -17, -100}, new byte[]{102, -94, -89, 119, 117, -114, -101, -7}).equals(str2)) {
                Types.GsmSignalStrength gsmSignalStrength3 = new Types.GsmSignalStrength();
                gsmSignalStrength3.f9214WWWWWWWW = SignalStrengthUtils.m5200WWWWWWWW(-103) + 1;
                gsmSignalStrength3.f9213WWWWoWWWWo = 3;
                signalStrength.f9230WWWWWWWW = gsmSignalStrength3;
            } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{100, 5, 41, -114}, new byte[]{20, 106, 70, -4, -38, TarConstants.LF_GNUTYPE_SPARSE, -89, 57}).equals(str2)) {
                Types.GsmSignalStrength gsmSignalStrength4 = new Types.GsmSignalStrength();
                gsmSignalStrength4.f9214WWWWWWWW = SignalStrengthUtils.m5200WWWWWWWW(-107) + 1;
                gsmSignalStrength4.f9213WWWWoWWWWo = 4;
                signalStrength.f9230WWWWWWWW = gsmSignalStrength4;
            } else {
                Types.GsmSignalStrength gsmSignalStrength5 = new Types.GsmSignalStrength();
                gsmSignalStrength5.f9214WWWWWWWW = SignalStrengthUtils.m5200WWWWWWWW(-97) + 1;
                gsmSignalStrength5.f9213WWWWoWWWWo = 2;
                signalStrength.f9230WWWWWWWW = gsmSignalStrength5;
            }
            if (m5161WWWW(m5193WWWWWWWW) == 1) {
                if (WWWWWWWW.m17835WWWWWWWW(new byte[]{66, 8, 7, TarConstants.LF_GNUTYPE_SPARSE, TarConstants.LF_CONTIG}, new byte[]{37, 122, 98, TarConstants.LF_SYMLINK, 67, 60, -57, -7}).equals(str2)) {
                    Types.CdmaSignalStrength cdmaSignalStrength = new Types.CdmaSignalStrength();
                    cdmaSignalStrength.f9166WWWWWWWW = 74;
                    cdmaSignalStrength.f9165WWWWoWWWWo = 89;
                    signalStrength.f9229WWWWoWWWWo = cdmaSignalStrength;
                } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{2, 87, -50, 0, -55, -64, 116, -40}, new byte[]{111, 56, -86, 101, -69, -95, 0, -67}).equals(str2)) {
                    Types.CdmaSignalStrength cdmaSignalStrength2 = new Types.CdmaSignalStrength();
                    cdmaSignalStrength2.f9166WWWWWWWW = 94;
                    cdmaSignalStrength2.f9165WWWWoWWWWo = 129;
                    signalStrength.f9229WWWWoWWWWo = cdmaSignalStrength2;
                } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{68, -109, -19, 118}, new byte[]{TarConstants.LF_BLK, -4, -126, 4, TarConstants.LF_CONTIG, 24, -95, 5}).equals(str2)) {
                    Types.CdmaSignalStrength cdmaSignalStrength3 = new Types.CdmaSignalStrength();
                    cdmaSignalStrength3.f9166WWWWWWWW = 98;
                    cdmaSignalStrength3.f9165WWWWoWWWWo = 149;
                    signalStrength.f9229WWWWoWWWWo = cdmaSignalStrength3;
                } else {
                    Types.CdmaSignalStrength cdmaSignalStrength4 = new Types.CdmaSignalStrength();
                    cdmaSignalStrength4.f9166WWWWWWWW = 84;
                    cdmaSignalStrength4.f9165WWWWoWWWWo = 109;
                    signalStrength.f9229WWWWoWWWWo = cdmaSignalStrength4;
                }
            } else {
                Types.CdmaSignalStrength cdmaSignalStrength5 = new Types.CdmaSignalStrength();
                cdmaSignalStrength5.f9166WWWWWWWW = Integer.MAX_VALUE;
                cdmaSignalStrength5.f9165WWWWoWWWWo = Integer.MAX_VALUE;
                signalStrength.f9229WWWWoWWWWo = cdmaSignalStrength5;
            }
            if (m5193WWWWWWWW != 7 && m5193WWWWWWWW != 8 && m5193WWWWWWWW != 12 && m5193WWWWWWWW != 13) {
                Types.EvdoSignalStrength evdoSignalStrength = new Types.EvdoSignalStrength();
                evdoSignalStrength.f9211WWWWWWWW = Integer.MAX_VALUE;
                evdoSignalStrength.f9210WWWWoWWWWo = Integer.MAX_VALUE;
                evdoSignalStrength.f9212WWWoWWWo = Integer.MAX_VALUE;
                signalStrength.f9233WWWoWWWo = evdoSignalStrength;
            } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-9, -5, -101, -80, Byte.MIN_VALUE}, new byte[]{-112, -119, -2, -47, -12, -34, 17, -70}).equals(str2)) {
                Types.EvdoSignalStrength evdoSignalStrength2 = new Types.EvdoSignalStrength();
                evdoSignalStrength2.f9211WWWWWWWW = 64;
                evdoSignalStrength2.f9210WWWWoWWWWo = 79;
                evdoSignalStrength2.f9212WWWoWWWo = 7;
                signalStrength.f9233WWWoWWWo = evdoSignalStrength2;
            } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{90, 21, -59, -99, -73, -110, -33, -104}, new byte[]{TarConstants.LF_CONTIG, 122, -95, -8, -59, -13, -85, -3}).equals(str2)) {
                Types.EvdoSignalStrength evdoSignalStrength3 = new Types.EvdoSignalStrength();
                evdoSignalStrength3.f9211WWWWWWWW = 89;
                evdoSignalStrength3.f9210WWWWoWWWWo = 124;
                evdoSignalStrength3.f9212WWWoWWWo = 3;
                signalStrength.f9233WWWoWWWo = evdoSignalStrength3;
            } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{18, 111, -127, -55}, new byte[]{98, 0, -18, -69, TarConstants.LF_GNUTYPE_SPARSE, -20, 67, 40}).equals(str2)) {
                Types.EvdoSignalStrength evdoSignalStrength4 = new Types.EvdoSignalStrength();
                evdoSignalStrength4.f9211WWWWWWWW = 104;
                evdoSignalStrength4.f9210WWWWoWWWWo = 154;
                evdoSignalStrength4.f9212WWWoWWWo = 1;
                signalStrength.f9233WWWoWWWo = evdoSignalStrength4;
            } else {
                Types.EvdoSignalStrength evdoSignalStrength5 = new Types.EvdoSignalStrength();
                evdoSignalStrength5.f9211WWWWWWWW = 74;
                evdoSignalStrength5.f9210WWWWoWWWWo = 98;
                evdoSignalStrength5.f9212WWWoWWWo = 5;
                signalStrength.f9233WWWoWWWo = evdoSignalStrength5;
            }
            if (m5149WWWWWWWW(m5193WWWWWWWW) == 1) {
                if (WWWWWWWW.m17835WWWWWWWW(new byte[]{116, 89, -79, 91, -94}, new byte[]{19, 43, -44, 58, -42, 39, -103, -69}).equals(str2)) {
                    Types.LteSignalStrength lteSignalStrength = new Types.LteSignalStrength();
                    lteSignalStrength.f9216WWWWWWWW = 13;
                    lteSignalStrength.f9215WWWWoWWWWo = 80;
                    lteSignalStrength.f9219WWWoWWWo = 11;
                    lteSignalStrength.f9217WWWWWWWW = 150;
                    lteSignalStrength.f9218WWWWWWWW = 14;
                    lteSignalStrength.f9220WWoWWo = Integer.MAX_VALUE;
                    signalStrength.f9231WWWWWWWW = lteSignalStrength;
                } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-85, -85, -50, -125, -33, 18, -125, 0}, new byte[]{-58, -60, -86, -26, -83, 115, -9, 101}).equals(str2)) {
                    Types.LteSignalStrength lteSignalStrength2 = new Types.LteSignalStrength();
                    lteSignalStrength2.f9216WWWWWWWW = 6;
                    lteSignalStrength2.f9215WWWWoWWWWo = 110;
                    lteSignalStrength2.f9219WWWoWWWo = 16;
                    lteSignalStrength2.f9217WWWWWWWW = 30;
                    lteSignalStrength2.f9218WWWWWWWW = 5;
                    lteSignalStrength2.f9220WWoWWo = Integer.MAX_VALUE;
                    signalStrength.f9231WWWWWWWW = lteSignalStrength2;
                } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{4, TarConstants.LF_MULTIVOLUME, -51, 115}, new byte[]{116, 34, -94, 1, -6, 17, -111, 5}).equals(str2)) {
                    Types.LteSignalStrength lteSignalStrength3 = new Types.LteSignalStrength();
                    lteSignalStrength3.f9216WWWWWWWW = 4;
                    lteSignalStrength3.f9215WWWWoWWWWo = 120;
                    lteSignalStrength3.f9219WWWoWWWo = 18;
                    lteSignalStrength3.f9217WWWWWWWW = -10;
                    lteSignalStrength3.f9218WWWWWWWW = 2;
                    lteSignalStrength3.f9220WWoWWo = Integer.MAX_VALUE;
                    signalStrength.f9231WWWWWWWW = lteSignalStrength3;
                } else {
                    Types.LteSignalStrength lteSignalStrength4 = new Types.LteSignalStrength();
                    lteSignalStrength4.f9216WWWWWWWW = 9;
                    lteSignalStrength4.f9215WWWWoWWWWo = 100;
                    lteSignalStrength4.f9219WWWoWWWo = 13;
                    lteSignalStrength4.f9217WWWWWWWW = 100;
                    lteSignalStrength4.f9218WWWWWWWW = 8;
                    lteSignalStrength4.f9220WWoWWo = Integer.MAX_VALUE;
                    signalStrength.f9231WWWWWWWW = lteSignalStrength4;
                }
            } else {
                Types.LteSignalStrength lteSignalStrength5 = new Types.LteSignalStrength();
                lteSignalStrength5.f9216WWWWWWWW = 99;
                lteSignalStrength5.f9215WWWWoWWWWo = Integer.MAX_VALUE;
                lteSignalStrength5.f9219WWWoWWWo = Integer.MAX_VALUE;
                lteSignalStrength5.f9217WWWWWWWW = Integer.MAX_VALUE;
                lteSignalStrength5.f9218WWWWWWWW = Integer.MAX_VALUE;
                lteSignalStrength5.f9220WWoWWo = Integer.MAX_VALUE;
                signalStrength.f9231WWWWWWWW = lteSignalStrength5;
            }
            if (m5193WWWWWWWW != 17) {
                z10 = false;
            } else {
                z10 = true;
            }
            if (z10) {
                if (WWWWWWWW.m17835WWWWWWWW(new byte[]{92, -124, -112, 95, -17}, new byte[]{59, -10, -11, 62, -101, 110, 60, -86}).equals(str2)) {
                    Types.TdscdmaSignalStrength tdscdmaSignalStrength = new Types.TdscdmaSignalStrength();
                    tdscdmaSignalStrength.f9234WWWWWWWW = 40;
                    signalStrength.f9232WWWWWWWW = tdscdmaSignalStrength;
                } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{17, 117, -51, -66, -119, 14, TarConstants.LF_NORMAL, -10}, new byte[]{124, 26, -87, -37, -5, 111, 68, -109}).equals(str2)) {
                    Types.TdscdmaSignalStrength tdscdmaSignalStrength2 = new Types.TdscdmaSignalStrength();
                    tdscdmaSignalStrength2.f9234WWWWWWWW = 80;
                    signalStrength.f9232WWWWWWWW = tdscdmaSignalStrength2;
                } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{116, 87, 3, -117}, new byte[]{4, 56, 108, -7, -104, TarConstants.LF_BLK, 3, 68}).equals(str2)) {
                    Types.TdscdmaSignalStrength tdscdmaSignalStrength3 = new Types.TdscdmaSignalStrength();
                    tdscdmaSignalStrength3.f9234WWWWWWWW = 100;
                    signalStrength.f9232WWWWWWWW = tdscdmaSignalStrength3;
                } else {
                    Types.TdscdmaSignalStrength tdscdmaSignalStrength4 = new Types.TdscdmaSignalStrength();
                    tdscdmaSignalStrength4.f9234WWWWWWWW = 60;
                    signalStrength.f9232WWWWWWWW = tdscdmaSignalStrength4;
                }
            } else {
                Types.TdscdmaSignalStrength tdscdmaSignalStrength5 = new Types.TdscdmaSignalStrength();
                tdscdmaSignalStrength5.f9234WWWWWWWW = Integer.MAX_VALUE;
                signalStrength.f9232WWWWWWWW = tdscdmaSignalStrength5;
            }
            if (m5154WWWWWW(m5193WWWWWWWW) == 1) {
                if (WWWWWWWW.m17835WWWWWWWW(new byte[]{84, -93, -109, -81, -17}, new byte[]{TarConstants.LF_CHR, -47, -10, -50, -101, -87, -28, -88}).equals(str2)) {
                    new Types.WcdmaSignalStrength();
                } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, 66, -100, -25, 16, -18, -22, -98}, new byte[]{-40, 45, -8, -126, 98, -113, -98, -5}).equals(str2)) {
                    new Types.WcdmaSignalStrength();
                } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-59, 23, 21, -116}, new byte[]{-75, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 122, -2, -29, 65, -117, 100}).equals(str2)) {
                    new Types.WcdmaSignalStrength();
                } else {
                    new Types.WcdmaSignalStrength();
                }
            } else {
                new Types.WcdmaSignalStrength();
            }
            if (m5193WWWWWWWW != 20) {
                new Types.NrSignalStrength();
            } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-70, -31, 39, 119, -3}, new byte[]{-35, -109, 66, 22, -119, -14, -9, -90}).equals(str2)) {
                new Types.NrSignalStrength();
            } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-63, 104, 60, 104, 122, -73, -111, TarConstants.LF_PAX_EXTENDED_HEADER_UC}, new byte[]{-84, 7, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 13, 8, -42, -27, 61}).equals(str2)) {
                new Types.NrSignalStrength();
            } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-72, -50, -84, 14}, new byte[]{-56, -95, -61, 124, 64, 82, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -98}).equals(str2)) {
                new Types.NrSignalStrength();
            } else {
                new Types.NrSignalStrength();
            }
            StringBuilder sb2 = new StringBuilder();
            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-10, -118, 33, -40, 23, 74}, new byte[]{-35, -55, 114, -119, 45, 106, -15, -16}));
            sb2.append(signalStrength.f9230WWWWWWWW.f9214WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{67}, new byte[]{111, 3, 93, -34, -53, -61, 8, TarConstants.LF_BLK}) + signalStrength.f9230WWWWWWWW.f9213WWWWoWWWWo + WWWWWWWW.m17835WWWWWWWW(new byte[]{78}, new byte[]{98, 14, 7, -38, 115, -77, 56, Byte.MAX_VALUE}) + signalStrength.f9229WWWWoWWWWo.f9166WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{31}, new byte[]{TarConstants.LF_CHR, -49, 87, -107, -23, 116, -8, -94}) + signalStrength.f9229WWWWoWWWWo.f9165WWWWoWWWWo + WWWWWWWW.m17835WWWWWWWW(new byte[]{-14}, new byte[]{-34, -45, -51, 125, 63, -122, 70, 0}) + signalStrength.f9233WWWoWWWo.f9211WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{-21}, new byte[]{-57, -80, 9, -69, -50, -13, 31, -106}) + signalStrength.f9233WWWoWWWo.f9210WWWWoWWWWo + WWWWWWWW.m17835WWWWWWWW(new byte[]{-120}, new byte[]{-92, -8, 104, 29, 44, 116, TarConstants.LF_MULTIVOLUME, -16}) + signalStrength.f9233WWWoWWWo.f9212WWWoWWWo + WWWWWWWW.m17835WWWWWWWW(new byte[]{-36}, new byte[]{-16, 44, 86, -115, -31, Byte.MIN_VALUE, 42, 7}) + signalStrength.f9231WWWWWWWW.f9216WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{98}, new byte[]{78, -114, -18, -104, -32, -64, 111, 38}) + signalStrength.f9231WWWWWWWW.f9215WWWWoWWWWo + WWWWWWWW.m17835WWWWWWWW(new byte[]{65}, new byte[]{109, -83, 81, -58, -112, -72, 59, -66}) + signalStrength.f9231WWWWWWWW.f9219WWWoWWWo + WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_SYMLINK}, new byte[]{30, -23, -119, 66, -18, -5, -85, -63}) + signalStrength.f9231WWWWWWWW.f9217WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_LINK}, new byte[]{29, -4, -114, -56, -35, -54, 66, 99}) + signalStrength.f9231WWWWWWWW.f9218WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{-27}, new byte[]{-55, -8, 69, -20, -106, -104, 117, TarConstants.LF_GNUTYPE_SPARSE}) + signalStrength.f9231WWWWWWWW.f9220WWoWWo + WWWWWWWW.m17835WWWWWWWW(new byte[]{-29}, new byte[]{-49, 118, -19, 94, 101, -15, 20, 90}) + signalStrength.f9232WWWWWWWW.f9234WWWWWWWW);
            return sb2.toString();
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-53, TarConstants.LF_SYMLINK, 106, -63, 17, -31, -78, 30, -81, 35, 29, -92, 5}, new byte[]{-32, 113, 39, -124, TarConstants.LF_LINK, -92, -32, TarConstants.LF_GNUTYPE_LONGNAME});
    }

    /* renamed from: WWWWཀྵWWWWࠤཀྵ  reason: contains not printable characters */
    public final String m5171WWWWWWWW(String str) {
        int numberOfTrailingZeros;
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{89, -86, 82, 80, TarConstants.LF_NORMAL, -20, -99}, new byte[]{114, -23, 6, 21, 115, -47, -94, -89}, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-49, 95, -106, 41, -39, 111, -113, -40, -56, 45, -18, 94, -74, 102, -125, -36, -56, 41, -18, 90}, new byte[]{-28, 28, -62, 108, -102, 85, -81, -24});
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{70, 7, -101, -43, -39, 92}, new byte[]{109, 68, -49, -112, -102, 99, -120, 57}))) {
            int m5164WWWWoWWWWo = m5164WWWWoWWWWo(true);
            if (m5159WWoWWo(m5164WWWWoWWWWo) == 1) {
                numberOfTrailingZeros = Integer.numberOfTrailingZeros(1);
            } else if (m5154WWWWWW(m5164WWWWoWWWWo) == 1) {
                numberOfTrailingZeros = Integer.numberOfTrailingZeros(2);
            } else if (m5164WWWWoWWWWo != 17) {
                if (m5161WWWW(m5164WWWWoWWWWo) == 1) {
                    numberOfTrailingZeros = Integer.numberOfTrailingZeros(4);
                } else if (m5149WWWWWWWW(m5164WWWWoWWWWo) == 1) {
                    numberOfTrailingZeros = Integer.numberOfTrailingZeros(16);
                } else if (m5164WWWWoWWWWo != 20) {
                    numberOfTrailingZeros = 0;
                } else {
                    numberOfTrailingZeros = Integer.numberOfTrailingZeros(64);
                }
            } else {
                numberOfTrailingZeros = Integer.numberOfTrailingZeros(32);
            }
            StringBuilder sb2 = new StringBuilder();
            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-46, -110, -118, -16, 110, -119, -81}, new byte[]{-7, -47, -34, -75, 45, -77, -113, 1}));
            sb2.append(numberOfTrailingZeros);
            return pr0.m9000WWWWWWWW(new byte[]{109, 47, 94}, new byte[]{65, 24, 56, 110, 71, -62, -30, 32}, sb2);
        } else if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{104, 105, -46, 84, -107, -85}, new byte[]{67, 42, -122, 17, -42, -106, -68, -5}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-34, 46, 102, -109, 63, -114, -89, 117, -70, 63, 17, -10, 43}, new byte[]{-11, 109, 43, -42, 31, -53, -11, 39});
        } else {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, 1, -21, 78, -72, -84, -63, 93, -12, 16, -100, 43, -84}, new byte[]{-69, 66, -90, ConstantPoolEntry.CP_InterfaceMethodref, -104, -23, -109, 15});
        }
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Type inference failed for: r1v4, types: [com.android.vmcore.event.DialNumberEvent, java.lang.Object] */
    /* renamed from: WWWWၗWWWW३ၗ  reason: contains not printable characters */
    public final void m5172WWWWWWWW(String str) {
        String substring;
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-54, -59, -59, -2, 28, -75, -96, 6, -83}, new byte[]{-114, -17, -4, -57, TarConstants.LF_FIFO, -97, -118, TarConstants.LF_CONTIG}, str)) {
            return;
        }
        if (str.endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-104}, new byte[]{-15, -102, -31, -23, -9, -34, -117, 111}))) {
            substring = str.substring(1, str.length() - 1);
        } else if (str.endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{82}, new byte[]{27, TarConstants.LF_GNUTYPE_LONGNAME, -34, 74, -6, 4, 18, -25}))) {
            substring = str.substring(1, str.length() - 1);
        } else {
            substring = str.substring(1);
        }
        final CallPdu callPdu = new CallPdu();
        ArrayList arrayList = this.f9076WWWWWWWW;
        callPdu.f9127WWWWWWWW = arrayList.size() + 1;
        callPdu.f9126WWWWoWWWWo = 0;
        callPdu.f9130WWWoWWWo = 2;
        callPdu.f9128WWWWWWWW = substring;
        callPdu.f9129WWWWWWWW = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        callPdu.f9131WWoWWo = 4;
        arrayList.add(callPdu);
        this.f9069WWWWoWWWWo.postDelayed(new Runnable() { // from class: com.android.vmcore.hal.PhoneService.1
            @Override // java.lang.Runnable
            public final void run() {
                CallPdu callPdu2 = callPdu;
                int i10 = callPdu2.f9130WWWoWWWo;
                if (i10 == 2) {
                    callPdu2.f9130WWWoWWWo = 3;
                    PhoneService.this.f9069WWWWoWWWWo.postDelayed(this, 1000L);
                } else if (i10 == 3) {
                    callPdu2.f9130WWWoWWWo = 0;
                }
            }
        }, 1500L);
        C2467WWWWWWWW c2467wwwwwwww = this.f9077WWWoWWWo.f8939WWWoWWWo;
        ?? obj = new Object();
        obj.f9005WWWWWWWW = substring;
        c2467wwwwwwww.m13940WWWWWWWW(obj);
    }

    /* renamed from: WWWWᄳWWWW़ᄳ  reason: contains not printable characters */
    public final Object m5173WWWWWWWW(int i10) {
        CellConfig m5185WoWo = m5185WoWo(i10);
        String str = m5185WoWo.f9084WWWWWWWW;
        String str2 = m5185WoWo.f9083WWWWoWWWWo;
        int i11 = m5185WoWo.f9089WWWoWWWo;
        int i12 = m5185WoWo.f9085WWWWWWWW;
        Types.CellIdentityOperatorNames cellIdentityOperatorNames = new Types.CellIdentityOperatorNames();
        cellIdentityOperatorNames.f9195WWWWWWWW = m5185WoWo.f9086WWWWWWWW;
        cellIdentityOperatorNames.f9194WWWWoWWWWo = m5185WoWo.f9090WWoWWo;
        int i13 = m5185WoWo.f9087WWWWWWWW;
        int i14 = m5185WoWo.f9088WWWWWWWW;
        if (m5159WWoWWo(i10) == 1) {
            Types.CellIdentityGsm cellIdentityGsm = new Types.CellIdentityGsm();
            cellIdentityGsm.f9174WWWWWWWW = str;
            cellIdentityGsm.f9173WWWWoWWWWo = str2;
            cellIdentityGsm.f9177WWWoWWWo = Integer.MAX_VALUE;
            cellIdentityGsm.f9175WWWWWWWW = Integer.MAX_VALUE;
            cellIdentityGsm.f9176WWWWWWWW = i14;
            cellIdentityGsm.f9178WWoWWo = cellIdentityOperatorNames;
            return cellIdentityGsm;
        } else if (m5154WWWWWW(i10) == 1) {
            Types.CellIdentityWcdma cellIdentityWcdma = new Types.CellIdentityWcdma();
            cellIdentityWcdma.f9203WWWWWWWW = str;
            cellIdentityWcdma.f9202WWWWoWWWWo = str2;
            cellIdentityWcdma.f9206WWWoWWWo = Integer.MAX_VALUE;
            cellIdentityWcdma.f9204WWWWWWWW = Integer.MAX_VALUE;
            cellIdentityWcdma.f9205WWWWWWWW = i14;
            cellIdentityWcdma.f9207WWoWWo = cellIdentityOperatorNames;
            return cellIdentityWcdma;
        } else if (i10 != 17) {
            if (m5161WWWW(i10) == 1) {
                Types.CellIdentityCdma cellIdentityCdma = new Types.CellIdentityCdma();
                cellIdentityCdma.f9168WWWWWWWW = i12;
                cellIdentityCdma.f9167WWWWoWWWWo = i11;
                cellIdentityCdma.f9171WWWoWWWo = Integer.MAX_VALUE;
                cellIdentityCdma.f9169WWWWWWWW = Integer.MAX_VALUE;
                cellIdentityCdma.f9170WWWWWWWW = Integer.MAX_VALUE;
                cellIdentityCdma.f9172WWoWWo = cellIdentityOperatorNames;
                return cellIdentityCdma;
            } else if (m5149WWWWWWWW(i10) == 1) {
                Types.CellIdentityLte cellIdentityLte = new Types.CellIdentityLte();
                cellIdentityLte.f9180WWWWWWWW = str;
                cellIdentityLte.f9179WWWWoWWWWo = str2;
                cellIdentityLte.f9185WWWoWWWo = Integer.MAX_VALUE;
                cellIdentityLte.f9181WWWWWWWW = Integer.MAX_VALUE;
                cellIdentityLte.f9182WWWWWWWW = i14;
                cellIdentityLte.f9186WWoWWo = cellIdentityOperatorNames;
                cellIdentityLte.f9183WWWWWWWW = 10000;
                LinkedHashSet linkedHashSet = new LinkedHashSet();
                cellIdentityLte.f9184WWWWWWWW = linkedHashSet;
                linkedHashSet.add(Integer.valueOf(i13));
                return cellIdentityLte;
            } else if (i10 != 20) {
                return null;
            } else {
                Types.CellIdentityNr cellIdentityNr = new Types.CellIdentityNr();
                cellIdentityNr.f9188WWWWWWWW = str;
                cellIdentityNr.f9187WWWWoWWWWo = str2;
                cellIdentityNr.f9192WWWoWWWo = Long.MAX_VALUE;
                cellIdentityNr.f9189WWWWWWWW = Integer.MAX_VALUE;
                cellIdentityNr.f9190WWWWWWWW = i14;
                cellIdentityNr.f9193WWoWWo = cellIdentityOperatorNames;
                LinkedHashSet linkedHashSet2 = new LinkedHashSet();
                cellIdentityNr.f9191WWWWWWWW = linkedHashSet2;
                linkedHashSet2.add(Integer.valueOf(i13));
                return cellIdentityNr;
            }
        } else {
            Types.CellIdentityTdscdma cellIdentityTdscdma = new Types.CellIdentityTdscdma();
            cellIdentityTdscdma.f9197WWWWWWWW = str;
            cellIdentityTdscdma.f9196WWWWoWWWWo = str2;
            cellIdentityTdscdma.f9200WWWoWWWo = Integer.MAX_VALUE;
            cellIdentityTdscdma.f9198WWWWWWWW = Integer.MAX_VALUE;
            cellIdentityTdscdma.f9199WWWWWWWW = i14;
            cellIdentityTdscdma.f9201WWoWWo = cellIdentityOperatorNames;
            return cellIdentityTdscdma;
        }
    }

    /* renamed from: WWWWሎWWWWܣሎ  reason: contains not printable characters */
    public final int m5174WWWWWWWW(int i10, boolean z10) {
        int i11 = 5;
        if (z10 || (i10 != 16 && i10 != 4 && i10 != 5)) {
            String str = this.f9077WWWoWWWo.f8937WWWoWWWo.f8877WWWWWWWW;
            byte[] bArr = {-45, -126, -87, -99, -76, -35, -112, TarConstants.LF_GNUTYPE_SPARSE};
            StringFog.f8859WWWWWWWW.getClass();
            if (str.equalsIgnoreCase(WWWWWWWW.m17835WWWWWWWW(new byte[]{-69, -19, -60, -8}, bArr))) {
                i11 = 1;
            } else if (!str.equalsIgnoreCase(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -70, -55, -102, 96, 10, -44}, new byte[]{21, -43, -88, -9, 9, 100, -77, -21}))) {
                if (str.equalsIgnoreCase(WWWWWWWW.m17835WWWWWWWW(new byte[]{30, 60, -73, -5, 20, 45, 3, 89, 2}, new byte[]{123, 81, -46, -119, 115, 72, 109, 58}))) {
                    i11 = 10;
                } else {
                    i11 = 0;
                }
            }
            if (i11 != 10 || z10) {
                return i11;
            }
        }
        return 0;
    }

    /* JADX WARN: Removed duplicated region for block: B:17:0x0038  */
    /* renamed from: WWWWሗWWWWшሗ  reason: contains not printable characters */
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final void m5175WWWWWWWW(int i10, boolean z10) {
        ArrayList arrayList = new ArrayList();
        ArrayList arrayList2 = this.f9076WWWWWWWW;
        int size = arrayList2.size();
        int i11 = 0;
        while (i11 < size) {
            Object obj = arrayList2.get(i11);
            i11++;
            CallPdu callPdu = (CallPdu) obj;
            if (i10 == -1 && callPdu.f9130WWWoWWWo == 0) {
                arrayList.add(callPdu);
            } else if (i10 != -1 && callPdu.f9127WWWWWWWW == i10) {
                arrayList.add(callPdu);
            }
        }
        arrayList2.removeAll(arrayList);
        if (z10) {
            int size2 = arrayList.size();
            int i12 = 0;
            while (i12 < size2) {
                Object obj2 = arrayList.get(i12);
                i12++;
                CallPdu callPdu2 = (CallPdu) obj2;
                int i13 = callPdu2.f9130WWWoWWWo;
                if (i13 == 1 || i13 == 5) {
                    callPdu2.f9130WWWoWWWo = 0;
                    return;
                }
                while (i12 < size2) {
                }
            }
        }
    }

    /* renamed from: WWWWኈWWWWˡኈ  reason: contains not printable characters */
    public final void m5176WWWWWWWW(int i10) {
        ArrayList arrayList = this.f9076WWWWWWWW;
        int size = arrayList.size();
        int i11 = -1;
        int i12 = 0;
        while (i12 < size) {
            Object obj = arrayList.get(i12);
            i12++;
            CallPdu callPdu = (CallPdu) obj;
            int i13 = callPdu.f9130WWWoWWWo;
            if (i13 == 1 || i13 == 5) {
                i11 = callPdu.f9127WWWWWWWW;
            }
            if (callPdu.f9127WWWWWWWW != i10) {
                if (i13 == 0) {
                    callPdu.f9130WWWoWWWo = 1;
                }
            } else {
                callPdu.f9130WWWoWWWo = 0;
            }
        }
        if (i11 != -1 && i10 == -1) {
            int size2 = arrayList.size();
            int i14 = 0;
            while (i14 < size2) {
                Object obj2 = arrayList.get(i14);
                i14++;
                CallPdu callPdu2 = (CallPdu) obj2;
                if (callPdu2.f9127WWWWWWWW == i11) {
                    callPdu2.f9130WWWoWWWo = 0;
                }
            }
        }
    }

    /* renamed from: WWWoԻWWWoͷԻ  reason: contains not printable characters */
    public final String m5177WWWoWWWo(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{100, 67, 18, 24, Byte.MIN_VALUE, -127, 90}, new byte[]{79, 0, 85, 85, -46, -68, 101, -111}, str)) {
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{112, -57, TarConstants.LF_LINK, 43, -25}, new byte[]{91, -124, 118, 102, -75, 124, TarConstants.LF_LINK, -62}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-106, 118, 115, 35, -6, 61, -74}, new byte[]{-67, TarConstants.LF_DIR, TarConstants.LF_BLK, 110, -88, 7, -106, -123}) + this.f9077WWWoWWWo.f8937WWWoWWWo.f8904WWoWWo;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{82, 9, -26, -35, 35, -68, 30, 23, TarConstants.LF_FIFO, 24, -111, -72, TarConstants.LF_CONTIG}, new byte[]{121, 74, -85, -104, 3, -7, TarConstants.LF_GNUTYPE_LONGNAME, 69});
    }

    /* renamed from: WWWoૄWWWoѽૄ  reason: contains not printable characters */
    public final String m5178WWWoWWWo(String str) {
        int i10;
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-99, -33, -28, 17, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 47, -118}, new byte[]{-74, -100, -87, 68, 44, 18, -75, 105}, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, 79, 18, -60, -81, TarConstants.LF_NORMAL, -114, 100, -82, 33, 110, -72}, new byte[]{-98, ConstantPoolEntry.CP_NameAndType, 95, -111, -5, 10, -82, TarConstants.LF_GNUTYPE_LONGNAME});
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{-120, TarConstants.LF_DIR, -8, 6, 47, 79}, new byte[]{-93, 118, -75, TarConstants.LF_GNUTYPE_SPARSE, 123, 112, -95, 6}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-6, 92, 8, 118, -72, 95, -29}, new byte[]{-47, 31, 69, 35, -20, 101, -61, 10}) + this.f9080WWWW;
        } else if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{82, -69, -44, 66, 7, -56}, new byte[]{121, -8, -103, 23, TarConstants.LF_GNUTYPE_SPARSE, -11, -107, -13}))) {
            try {
                i10 = Integer.parseInt(str.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{-10, -101, 41, 89, -82, 2}, new byte[]{-35, -40, 100, ConstantPoolEntry.CP_NameAndType, -6, 63, -40, 105}).length()));
            } catch (Exception unused) {
                i10 = -1;
            }
            if (i10 >= 0 && i10 <= 1) {
                this.f9080WWWW = i10;
                return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
            }
            StringFog.f8859WWWWWWWW.getClass();
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-42, -4, -60, 67, 95, -94, -118, 59, -78, -19, -77, 38, 74, -41}, new byte[]{-3, -65, -119, 6, Byte.MAX_VALUE, -25, -40, 105});
        } else {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-35, -75, 23, 32, -124, 113, -112, -15, -71, -92, 96, 69, -112}, new byte[]{-10, -10, 90, 101, -92, TarConstants.LF_BLK, -62, -93});
        }
    }

    /* renamed from: WWoڢWWo࢞ڢ  reason: not valid java name and contains not printable characters */
    public final String m5179WWoWWo(String str) {
        int i10;
        boolean z10;
        boolean z11;
        byte[] bArr = {-47, 23, 63, -114, -103, 3, TarConstants.LF_LINK, TarConstants.LF_PAX_EXTENDED_HEADER_UC};
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-6, 84, 119, -62, -35, 62, 14}, bArr, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-62, TarConstants.LF_CHR, -44, -97, TarConstants.LF_CHR, -53, 79, -25, -39, 92, -83, -1, 70, -119, 67, -3, -59, 66, -28, -1, 68, -40}, new byte[]{-23, 112, -100, -45, 119, -15, 111, -49});
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-17, 43, -85, -95, -101, 72}, new byte[]{-60, 104, -29, -19, -33, 117, -70, 23}))) {
            try {
                i10 = Integer.parseInt(str.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{-120, -3, 29, 20, 94, -29}, new byte[]{-93, -66, 85, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 26, -34, -88, -19}).length()));
            } catch (Exception unused) {
                i10 = -1;
            }
            ArrayList arrayList = this.f9076WWWWWWWW;
            int i11 = 0;
            if (i10 == 0) {
                ArrayList arrayList2 = new ArrayList();
                int size = arrayList.size();
                while (i11 < size) {
                    Object obj = arrayList.get(i11);
                    i11++;
                    CallPdu callPdu = (CallPdu) obj;
                    int i12 = callPdu.f9130WWWoWWWo;
                    if (i12 == 5) {
                        arrayList2.add(callPdu);
                    } else if (i12 == 1) {
                        arrayList2.add(callPdu);
                    } else if (i12 == 4) {
                        arrayList2.add(callPdu);
                    }
                }
                arrayList.removeAll(arrayList2);
            } else if (i10 == 1) {
                m5175WWWWWWWW(-1, true);
                return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
            } else if (i10 >= 10 && i10 < 20) {
                m5175WWWWWWWW(i10 - 10, false);
                return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
            } else if (i10 == 2) {
                m5176WWWWWWWW(-1);
                return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
            } else {
                if (i10 >= 20) {
                    z10 = true;
                } else {
                    z10 = false;
                }
                if (i10 < 30) {
                    z11 = true;
                } else {
                    z11 = false;
                }
                if (z10 & z11) {
                    m5176WWWWWWWW(i10 - 20);
                    return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
                } else if (i10 == 3) {
                    int size2 = arrayList.size();
                    int i13 = 0;
                    while (i13 < size2) {
                        Object obj2 = arrayList.get(i13);
                        i13++;
                        CallPdu callPdu2 = (CallPdu) obj2;
                        if (callPdu2.f9130WWWoWWWo == 1) {
                            callPdu2.f9130WWWoWWWo = 0;
                        }
                    }
                } else {
                    StringFog.f8859WWWWWWWW.getClass();
                    return WWWWWWWW.m17835WWWWWWWW(new byte[]{-30, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, ConstantPoolEntry.CP_NameAndType, 89, -110, TarConstants.LF_MULTIVOLUME, -2, 62, -122, 118, 123, 60, -122}, new byte[]{-55, 36, 65, 28, -78, 8, -84, 108});
                }
            }
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{44, 62, 126, -68, 96, -2, 81, -43, 72, 47, 9, -39, 116}, new byte[]{7, 125, TarConstants.LF_CHR, -7, 64, -69, 3, -121});
    }

    /* renamed from: WWo௹WWoਠ௹  reason: contains not printable characters */
    public final String m5180WWoWWo(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{108, -20, 91, TarConstants.LF_CONTIG, -14, TarConstants.LF_FIFO}, new byte[]{71, -81, 20, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -95, 9, 23, -93}, str)) {
            StringBuilder sb2 = new StringBuilder(WWWWWWWW.m17835WWWWWWWW(new byte[]{-105, -42, 69, 34, 121, -112, -89}, new byte[]{-68, -107, 10, 114, 42, -86, -121, -97}));
            sb2.append(this.f9073WWWWWWWW);
            if (!this.f9079WWoWWo) {
                return sb2.toString();
            }
            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{114}, new byte[]{94, 35, 5, 59, 42, 35, 32, -93}));
            sb2.append(this.f9074WWWWWWWW);
            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-64}, new byte[]{-20, -40, 39, -47, -46, -52, -34, 58}));
            CellConfig m5185WoWo = m5185WoWo(m5164WWWWoWWWWo(true));
            int i10 = this.f9074WWWWWWWW;
            if (i10 != 0) {
                if (i10 != 1) {
                    if (i10 != 2) {
                        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{19, 91, 110, -79, 106, 96}, new byte[]{35, 107, 94, -127, 90, 80, -54, -106}));
                    } else {
                        sb2.append(m5185WoWo.f9084WWWWWWWW);
                        sb2.append(m5185WoWo.f9083WWWWoWWWWo);
                    }
                } else {
                    sb2.append(m5185WoWo.f9090WWoWWo);
                }
            } else {
                sb2.append(m5185WoWo.f9086WWWWWWWW);
            }
            return sb2.toString();
        } else if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{-65, 80, 69, -89, -97, -22, 113}, new byte[]{-108, 19, 10, -9, -52, -41, 78, TarConstants.LF_BLK}))) {
            StringBuilder sb3 = new StringBuilder(WWWWWWWW.m17835WWWWWWWW(new byte[]{74, TarConstants.LF_NORMAL, -46, -77, -27, -58, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{97, 115, -99, -29, -74, -4, 108, -36}));
            if (this.f9079WWoWWo) {
                sb3.append(2);
                sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-86}, new byte[]{-122, 37, 34, 25, -81, 86, -48, -122}));
            } else {
                sb3.append(1);
                sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-60}, new byte[]{-24, 94, 110, -43, 43, -27, 108, -44}));
            }
            int m5164WWWWoWWWWo = m5164WWWWoWWWWo(true);
            CellConfig m5185WoWo2 = m5185WoWo(m5164WWWWoWWWWo);
            sb3.append(m5185WoWo2.f9086WWWWWWWW);
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{85}, new byte[]{121, 9, 34, 90, -123, 32, -77, -29}));
            sb3.append(m5185WoWo2.f9090WWoWWo);
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{99}, new byte[]{79, -116, 116, 58, -103, Byte.MAX_VALUE, 23, 36}));
            sb3.append(m5185WoWo2.f9084WWWWWWWW);
            sb3.append(m5185WoWo2.f9083WWWWoWWWWo);
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-39}, new byte[]{-11, 85, TarConstants.LF_LINK, -6, -50, TarConstants.LF_GNUTYPE_LONGNAME, 106, TarConstants.LF_GNUTYPE_LONGNAME}));
            sb3.append(m5164WWWWoWWWWo);
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType, -108}, new byte[]{32, -72, 78, -25, 24, -8, 42, -122}));
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, 100, -115, -36, ConstantPoolEntry.CP_InterfaceMethodref}, new byte[]{-32, 84, -96, -24, 34, 109, 74, 18}));
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-87}, new byte[]{-123, -63, -7, -74, -90, -81, 8, 14}));
            sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{19, -114, -94, -68, 85}, new byte[]{59, -66, -113, -114, 124, 3, -67, -60}));
            return sb3.toString();
        } else if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{-88, -79, 107, -58, -50, 47}, new byte[]{-125, -14, 36, -106, -99, 18, 33, 14}))) {
            String[] split = str.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{-25, -94, -16, 115, -89, 5}, new byte[]{-52, -31, -65, 35, -12, 56, -61, -74}).length()).split(WWWWWWWW.m17835WWWWWWWW(new byte[]{97}, new byte[]{TarConstants.LF_MULTIVOLUME, -11, 15, 85, -10, -95, -105, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}));
            try {
                int parseInt = Integer.parseInt(split[0]);
                if (parseInt == 3) {
                    this.f9074WWWWWWWW = Integer.parseInt(split[1]);
                } else {
                    this.f9073WWWWWWWW = parseInt;
                }
                if (parseInt == 2) {
                    this.f9079WWoWWo = false;
                    return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
                } else if (parseInt != 3) {
                    this.f9079WWoWWo = true;
                    return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
                } else {
                    return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
                }
            } catch (Exception unused) {
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{15, -115, 45, 60, -96, -109, -62, -105, 107, -100, 90, 89, -75, -26}, new byte[]{36, -50, 96, 121, Byte.MIN_VALUE, -42, -112, -59});
            }
        } else {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{117, TarConstants.LF_GNUTYPE_LONGLINK, -65, -47, -77, TarConstants.LF_GNUTYPE_SPARSE, 105, -76, 17, 90, -56, -76, -89}, new byte[]{94, 8, -14, -108, -109, 22, 59, -26});
        }
    }

    /* JADX WARN: Type inference failed for: r3v1, types: [java.lang.Object, com.android.vmcore.hal.PhoneService$IccConfig] */
    /* renamed from: WWoᆑWWoӁᆑ  reason: contains not printable characters */
    public final IccConfig m5181WWoWWo() {
        String str;
        VMInstance vMInstance = this.f9077WWWoWWWo;
        String str2 = vMInstance.f8937WWWoWWWo.f8905WWoWWo;
        if (str2.length() > 3) {
            str2.substring(0, 3);
        }
        if (str2.length() > 3) {
            str = str2.substring(3);
        } else {
            str = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        ?? obj = new Object();
        obj.f9091WWWWoWWWWo = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        obj.f9095WWWoWWWo = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        obj.f9093WWWWWWWW = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        obj.f9094WWWWWWWW = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        obj.f9096WWoWWo = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        obj.f9092WWWWWWWW = str;
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        obj.f9091WWWWoWWWWo = vMConfig.f8875WWWWWWWW;
        String str3 = vMConfig.f8906WWoWWo;
        obj.f9095WWWoWWWo = str3;
        if (str3.length() > str2.length()) {
            obj.f9093WWWWWWWW = obj.f9095WWWoWWWo.substring(str2.length());
        } else {
            obj.f9093WWWWWWWW = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        obj.f9094WWWWWWWW = PhoneNumberUtils.normalizeNumber(vMConfig.f8898WWWoWWWo);
        obj.f9096WWoWWo = vMConfig.f8897WWWoWWWo;
        return obj;
    }

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public final String m5182WWWW(String str) {
        int i10;
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{73, -127, -69, -64, 33, -14, 46, 28}, new byte[]{98, -62, -4, -110, 100, -75, 19, 35}, str)) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{113, -1, -124, -20, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 118, -95, -80, 114, -116, -18, -115, 17, 17, -86, -94, 98, -107}, new byte[]{90, -68, -61, -66, 61, TarConstants.LF_LINK, -101, -112});
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{25, 39, 69, -43, 38, -2, -9}, new byte[]{TarConstants.LF_SYMLINK, 100, 2, -121, 99, -71, -54, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}))) {
            try {
                i10 = Integer.parseInt(str.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{-67, -76, -102, ConstantPoolEntry.CP_NameAndType, -99, -76, 74}, new byte[]{-106, -9, -35, 94, -40, -13, 119, -108}).length()));
            } catch (Exception unused) {
                i10 = -1;
            }
            if ((i10 >= 0 && i10 <= 3) || i10 == 128) {
                this.f9072WWWWWWWW = i10;
                return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
            }
            byte[] bArr = {TarConstants.LF_MULTIVOLUME, -117, 20, TarConstants.LF_BLK, 58, -78, -26, 124};
            StringFog.f8859WWWWWWWW.getClass();
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{102, -56, 89, 113, 26, -9, -76, 46, 2, -39, 46, 20, 15, -126}, bArr);
        } else if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{-14, 106, -105, -127, -65, 63, TarConstants.LF_DIR}, new byte[]{-39, 41, -48, -45, -6, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 10, -83}))) {
            int m5164WWWWoWWWWo = m5164WWWWoWWWWo(false);
            int m5174WWWWWWWW = m5174WWWWWWWW(m5164WWWWoWWWWo, false);
            if (m5174WWWWWWWW == 0) {
                m5164WWWWoWWWWo = 0;
            }
            Object m5173WWWWWWWW = m5173WWWWWWWW(m5164WWWWoWWWWo);
            Types.RegStateResult regStateResult = new Types.RegStateResult();
            regStateResult.f9224WWWWWWWW = m5174WWWWWWWW;
            regStateResult.f9223WWWWoWWWWo = m5164WWWWoWWWWo;
            regStateResult.f9227WWWoWWWo = m5173WWWWWWWW;
            regStateResult.f9225WWWWWWWW = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
            if (m5149WWWWWWWW(m5164WWWWoWWWWo) == 1) {
                Types.LteVopsInfo lteVopsInfo = new Types.LteVopsInfo();
                lteVopsInfo.f9222WWWWWWWW = true;
                lteVopsInfo.f9221WWWWoWWWWo = true;
                Types.NrIndicators nrIndicators = new Types.NrIndicators();
                Types.EutranRegistrationInfo eutranRegistrationInfo = new Types.EutranRegistrationInfo();
                regStateResult.f9228WWoWWo = eutranRegistrationInfo;
                eutranRegistrationInfo.f9209WWWWWWWW = lteVopsInfo;
                eutranRegistrationInfo.f9208WWWWoWWWWo = nrIndicators;
            }
            int i11 = this.f9072WWWWWWWW;
            if (i11 != 0 && i11 != 1) {
                if (i11 != 2 && i11 != 3) {
                    StringBuilder sb2 = new StringBuilder();
                    sb2.append(regStateResult.f9224WWWWWWWW);
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType}, new byte[]{32, -9, 28, -96, 95, 72, -57, 72}));
                    sb2.append(regStateResult.f9223WWWWoWWWWo);
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-53}, new byte[]{-25, 7, 113, -87, -116, 72, 32, 30}));
                    sb2.append(0);
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-36}, new byte[]{-16, -42, 102, 64, 91, -77, 90, 33}));
                    sb2.append(16);
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-32}, new byte[]{-52, -91, -67, -72, -96, 113, 42, 85}));
                    sb2.append(m5143WWWWWWWW(regStateResult.f9227WWWoWWWo));
                    if (!TextUtils.isEmpty(regStateResult.f9225WWWWWWWW)) {
                        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-106}, new byte[]{-70, -61, 38, -110, 73, 0, 93, -93}));
                        sb2.append(regStateResult.f9225WWWWWWWW);
                    } else {
                        pr0.m9009WWWoWWWo(new byte[]{5}, new byte[]{41, -121, 58, -70, -24, -98, -117, 98}, sb2, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
                    }
                    Types.EutranRegistrationInfo eutranRegistrationInfo2 = regStateResult.f9228WWoWWo;
                    if (eutranRegistrationInfo2 != null) {
                        if (eutranRegistrationInfo2.f9209WWWWWWWW != null) {
                            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-48}, new byte[]{-4, 31, 56, 96, -79, 4, -26, -43}));
                            sb2.append(regStateResult.f9228WWoWWo.f9209WWWWWWWW.f9222WWWWWWWW);
                            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-100}, new byte[]{-80, 34, 18, 27, -108, 95, 19, -77}));
                            sb2.append(regStateResult.f9228WWoWWo.f9209WWWWWWWW.f9221WWWWoWWWWo);
                        }
                        if (regStateResult.f9228WWoWWo.f9208WWWWoWWWWo != null) {
                            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CHR}, new byte[]{31, -33, -14, 13, 59, 68, -31, -68}));
                            regStateResult.f9228WWoWWo.f9208WWWWoWWWWo.getClass();
                            sb2.append(false);
                            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-26}, new byte[]{-54, 99, -76, 93, -26, -105, -103, -30}));
                            regStateResult.f9228WWoWWo.f9208WWWWoWWWWo.getClass();
                            sb2.append(false);
                            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-18}, new byte[]{-62, -87, -27, 23, 15, -36, -100, -6}));
                            regStateResult.f9228WWoWWo.f9208WWWWoWWWWo.getClass();
                            sb2.append(false);
                        }
                    }
                    String sb3 = sb2.toString();
                    return WWWWWWWW.m17835WWWWWWWW(new byte[]{-125, 3, -4, -43, -66, 113, -27, -47}, new byte[]{-88, 64, -69, -121, -5, TarConstants.LF_FIFO, -33, -15}) + this.f9072WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{-126}, new byte[]{-82, -33, -7, -106, 26, TarConstants.LF_GNUTYPE_SPARSE, -44, -81}) + sb3;
                }
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{-108, -67, 92, 104, 118, -56, -17, 2}, new byte[]{-65, -2, 27, 58, TarConstants.LF_CHR, -113, -43, 34}) + this.f9072WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{-106}, new byte[]{-70, -51, -15, 123, -31, -5, -91, -96}) + regStateResult.f9224WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{-16, -100, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 115, -72, -75, 115, 116, -70, -121, 41, 115, -72, -75, 115, 116, -70, -51, TarConstants.LF_SYMLINK}, new byte[]{-36, -85, 30, 21, -34, -45, 21, 18}) + regStateResult.f9223WWWWoWWWWo;
            }
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{8, 27, TarConstants.LF_GNUTYPE_SPARSE, -40, Byte.MIN_VALUE, -116, 65, 98}, new byte[]{35, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 20, -118, -59, -53, 123, 66}) + this.f9072WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}, new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, 112, -14, 112, -76, -50, TarConstants.LF_NORMAL, -22}) + regStateResult.f9224WWWWWWWW;
        } else {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-8, -26, -106, -22, 81, -78, 38, -65, -100, -9, -31, -113, 69}, new byte[]{-45, -91, -37, -81, 113, -9, 116, -19});
        }
    }

    /* renamed from: WWၚஊၚ  reason: contains not printable characters */
    public final String m5183WW(String str) {
        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-71, -20, 125, 58, -61, 107}, new byte[]{-110, -69, 46, 111, -127, 84, 95, 3}, str)) {
            IccConfig m5181WWoWWo = m5181WWoWWo();
            return String.format(Locale.US, WWWWWWWW.m17835WWWWWWWW(new byte[]{-126, 7, TarConstants.LF_CONTIG, 71, 72, 109, -56, 14, -38, 124, 85, 62, 59, 123, -51, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -123, 97}, new byte[]{-87, 80, 100, 18, 10, 87, -24, 43}), m5181WWoWWo.f9094WWWWWWWW, m5181WWoWWo.f9093WWWWWWWW);
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{57, -60, -55, 29, -82, -13, 123, -49, 93, -43, -66, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -70}, new byte[]{18, -121, -124, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -114, -74, 41, -99});
    }

    /* renamed from: WoڄWoᄴڄ  reason: contains not printable characters */
    public final String m5184WoWo(String str) {
        int i10;
        boolean m3430WWWWWWWWWW = AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-90, 39, TarConstants.LF_SYMLINK, 100, -103}, new byte[]{-115, 100, 117, TarConstants.LF_CONTIG, -41, -19, -84, -19}, str);
        VMInstance vMInstance = this.f9077WWWoWWWo;
        if (m3430WWWWWWWWWW) {
            return vMInstance.f8937WWWoWWWo.f8871WWWWWWWW;
        }
        if (str.equals(WWWWWWWW.m17835WWWWWWWW(new byte[]{41, 89, TarConstants.LF_GNUTYPE_LONGLINK, -112, -84, -30, -45}, new byte[]{2, 26, ConstantPoolEntry.CP_NameAndType, -61, -30, -33, -20, -120}))) {
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{-105, 84, -111, 63, 84, 78, 104, 107, -116, 58, -27, 69}, new byte[]{-68, 23, -42, 108, 26, 116, 72, 67});
        }
        if (str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{29, 59, TarConstants.LF_GNUTYPE_LONGLINK, -107, 21, 17}, new byte[]{TarConstants.LF_FIFO, TarConstants.LF_PAX_EXTENDED_HEADER_LC, ConstantPoolEntry.CP_NameAndType, -58, 91, 44, -93, 90}))) {
            try {
                i10 = Integer.parseInt(str.substring(WWWWWWWW.m17835WWWWWWWW(new byte[]{-15, -126, 63, 116, 81, -82}, new byte[]{-38, -63, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 39, 31, -109, 5, 97}).length()));
            } catch (Exception unused) {
                i10 = -1;
            }
            if (i10 == 0) {
                StringBuilder sb2 = new StringBuilder();
                if (m5161WWWW(m5164WWWWoWWWWo(true)) == 1) {
                    VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
                    WWoWWo.m59WWoWWo(sb2, vMConfig.f8920WoWo, "\r", vMConfig.f8914WWWW);
                } else {
                    VMConfig vMConfig2 = vMInstance.f8937WWWoWWWo;
                    WWoWWo.m59WWoWWo(sb2, vMConfig2.f8871WWWWWWWW, "\r", vMConfig2.f8872WWWWWWWW);
                }
                return sb2.toString();
            } else if (i10 == 1) {
                StringBuilder sb3 = new StringBuilder();
                pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-59, 65, -4, 18, 96, 115, -102}, new byte[]{-18, 2, -69, 65, 46, 73, -70, -14}, sb3);
                sb3.append(vMInstance.f8937WWWoWWWo.f8871WWWWWWWW);
                return sb3.toString();
            } else if (i10 == 2) {
                VMConfig vMConfig3 = vMInstance.f8937WWWoWWWo;
                String str2 = vMConfig3.f8871WWWWWWWW;
                String str3 = str2.substring(0, str2.length() - 1) + vMConfig3.f8872WWWWWWWW;
                return AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{110, 87, 29, -69, 65, 24, -16}, new byte[]{69, 20, 90, -24, 15, 34, -48, 112}, new StringBuilder(), str3);
            } else if (i10 == 3) {
                String str4 = vMInstance.f8937WWWoWWWo.f8872WWWWWWWW;
                return AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-69, -96, -109, 6, -53, 123, -28}, new byte[]{-112, -29, -44, 85, -123, 65, -60, -10}, new StringBuilder(), str4);
            } else {
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{22, 125, 91, 66, -98, 44, TarConstants.LF_GNUTYPE_SPARSE, 93, 114, 108, 44, 39, -117, 89}, new byte[]{61, 62, 22, 7, -66, 105, 1, 15});
            }
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, -72, 96, -53, -9, 123, 84, -90, 111, -87, 23, -82, -29}, new byte[]{32, -5, 45, -114, -41, 62, 6, -12});
    }

    /* JADX WARN: Type inference failed for: r6v1, types: [com.android.vmcore.hal.PhoneService$CellConfig, java.lang.Object] */
    /* renamed from: WoოWo੍ო  reason: contains not printable characters */
    public final CellConfig m5185WoWo(int i10) {
        String str;
        String str2;
        VMInstance vMInstance = this.f9077WWWoWWWo;
        String str3 = vMInstance.f8937WWWoWWWo.f8876WWWWWWWW;
        if (str3.length() > 3) {
            str = str3.substring(0, 3);
        } else {
            str = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        if (str3.length() > 3) {
            str2 = str3.substring(3);
        } else {
            str2 = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        ?? obj = new Object();
        obj.f9086WWWWWWWW = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        obj.f9090WWoWWo = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        obj.f9084WWWWWWWW = str;
        obj.f9083WWWWoWWWWo = str2;
        obj.f9089WWWoWWWo = 1;
        obj.f9085WWWWWWWW = 1;
        String str4 = vMInstance.f8937WWWoWWWo.f8907WWoWWo;
        obj.f9086WWWWWWWW = str4;
        obj.f9090WWoWWo = str4;
        if (m5159WWoWWo(i10) == 1) {
            obj.f9087WWWWWWWW = 12;
            obj.f9088WWWWWWWW = 533;
            return obj;
        } else if (m5154WWWWWW(i10) == 1) {
            obj.f9087WWWWWWWW = 1;
            obj.f9088WWWWWWWW = 10562;
            return obj;
        } else if (i10 != 17) {
            if (m5149WWWWWWWW(i10) == 1) {
                obj.f9087WWWWWWWW = 3;
                obj.f9088WWWWWWWW = 1300;
                return obj;
            } else if (i10 != 20) {
                obj.f9087WWWWWWWW = 0;
                obj.f9088WWWWWWWW = 0;
                return obj;
            } else {
                obj.f9087WWWWWWWW = 41;
                obj.f9088WWWWWWWW = 504990;
                return obj;
            }
        } else {
            obj.f9087WWWWWWWW = 0;
            obj.f9088WWWWWWWW = 10054;
            return obj;
        }
    }
}
