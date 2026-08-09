package com.android.vmcore;

import android.content.SharedPreferences;
import android.os.Build;
import android.telephony.TelephonyManager;
import android.text.TextUtils;
import com.android.vmapp.VMApp;
import com.android.vmcore.event.VMCreationEvent;
import com.android.vmcore.utils.FakeUtils;
import com.android.vmcore.utils.FileDeleteUtils;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import eh.C2467WWWWWWWW;
import j$.util.DesugarCollections;
import java.io.File;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p053WWoWWo.AbstractC0470WWWWWWWW;
import p057WWoWWo.WWWWoWWWWo;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class VMManager {

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static int f8947WWWWWWWW;

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public static VMManager f8948WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static final String f8949WWWoWWWo;

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final ArrayList f8950WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final VMApp f8951WWWWWWWW;

    static {
        byte[] bArr = {TarConstants.LF_GNUTYPE_SPARSE, 78, 125, 5, 23, 26, 29, 74, 119};
        byte[] bArr2 = {5, 3, TarConstants.LF_NORMAL, 100, 121, 123, 122, 47};
        StringFog.f8859WWWWWWWW.getClass();
        f8949WWWoWWWo = WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
        f8947WWWWWWWW = -1;
    }

    public VMManager(VMApp vMApp) {
        this.f8951WWWWWWWW = vMApp;
        ArrayList arrayList = new ArrayList();
        String str = vMApp.getApplicationInfo().dataDir;
        StringFog.f8859WWWWWWWW.getClass();
        File[] listFiles = new File(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-89, -113, 27, -116, -117, 71, -4, -59, -90, -126, 28, -115}, new byte[]{-44, -25, 122, -2, -18, 35, -93, -75})).listFiles(new C1625WWWoWWWo(1));
        if (listFiles != null) {
            for (File file : listFiles) {
                String name = file.getName();
                byte[] bArr = {-20, -107, 125, 91, TarConstants.LF_FIFO, 102, -67, ConstantPoolEntry.CP_NameAndType};
                StringFog.f8859WWWWWWWW.getClass();
                try {
                    int parseInt = Integer.parseInt(name.replace(WWWWWWWW.m17835WWWWWWWW(new byte[]{-102, -8, 34, 56, 89, 8, -37, 101, -117, -54}, bArr), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING).replace(WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, -14, 33, -66}, new byte[]{-101, -118, TarConstants.LF_GNUTYPE_LONGNAME, -46, -15, TarConstants.LF_GNUTYPE_SPARSE, -62, 73}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING));
                    m5106WWWWWWWW(vMApp.getSharedPreferences(WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, -119, -64, -48, 98, -70, -39, 62, -39, -69}, new byte[]{-66, -28, -97, -77, 13, -44, -65, 87}) + parseInt, 0));
                    arrayList.add(new VMInstance(vMApp, parseInt));
                    if (parseInt > f8947WWWWWWWW) {
                        f8947WWWWWWWW = parseInt;
                    }
                } catch (Throwable unused) {
                    StringBuilder sb2 = new StringBuilder();
                    StringFog.f8859WWWWWWWW.getClass();
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-42, -60, 98, 79, -77, -2, 64, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -28, -37, 121, 115, -9, -54, 108, 112, -83, -34, 96, 14, -79, -63, 97, 113, -83}, new byte[]{-115, -88, 13, 46, -41, -88, 13, 20}));
                    sb2.append(file.getName());
                    KLog.m5040WWWWoWWWWo(f8949WWWoWWWo, sb2.toString());
                }
            }
            Collections.sort(arrayList, new WWWWoWWWWo(2));
            f8947WWWWWWWW++;
        }
        this.f8950WWWWoWWWWo = arrayList;
        String str2 = this.f8951WWWWWWWW.getApplicationInfo().dataDir;
        StringFog.f8859WWWWWWWW.getClass();
        File[] listFiles2 = new File(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-44, 72}, new byte[]{-94, 37, 32, 13, -118, 29, -34, 20})).listFiles(new C1625WWWoWWWo(0));
        if (listFiles2 != null) {
            for (File file2 : listFiles2) {
                try {
                    if (m5105WWWWWWWW(Integer.parseInt(file2.getName().substring(2))) == null) {
                        FileDeleteUtils.m5262WWWWWWWW(file2);
                    }
                } catch (Throwable unused2) {
                }
            }
        }
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static synchronized VMManager m5102WWWWWWWW() {
        VMManager vMManager;
        synchronized (VMManager.class) {
            vMManager = f8948WWWWWWWW;
            if (vMManager == null) {
                byte[] bArr = {-55, -23, 109, TarConstants.LF_SYMLINK, 26, 29, 117, -55, -19, -124, 78, 60, 0, 92, 123, -62, -10, -48, 73, TarConstants.LF_SYMLINK, 24, 21, 104, -55, -5};
                byte[] bArr2 = {-97, -92, 32, TarConstants.LF_GNUTYPE_SPARSE, 116, 124, 18, -84};
                StringFog.f8859WWWWWWWW.getClass();
                throw new RuntimeException(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
            }
        }
        return vMManager;
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final void m5103WWWWoWWWWo(RomConfig romConfig, VMResConfig vMResConfig, String str) {
        char c10;
        String str2;
        int i10 = f8947WWWWWWWW;
        int i11 = i10 + 1;
        f8947WWWWWWWW = i11;
        if (TextUtils.isEmpty(str)) {
            StringBuilder sb2 = new StringBuilder();
            c10 = 2;
            sb2.append(romConfig.f8845WWWWoWWWWo);
            byte[] bArr = {-84, -10, TarConstants.LF_GNUTYPE_SPARSE, 93, 34, 82, -44, TarConstants.LF_FIFO};
            StringFog.f8859WWWWWWWW.getClass();
            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-13}, bArr));
            sb2.append(i11);
            str2 = sb2.toString();
        } else {
            c10 = 2;
            str2 = str;
        }
        String m5254WWWWWWWW = FakeUtils.m5254WWWWWWWW();
        String m5250WWWWWWWW = FakeUtils.m5250WWWWWWWW();
        String m5257WWoWWo = FakeUtils.m5257WWoWWo();
        String m5247WWWWoWWWWo = FakeUtils.m5247WWWWoWWWWo();
        String[] m5107WWWoWWWo = m5107WWWoWWWo();
        String m5249WWWWWWWW = FakeUtils.m5249WWWWWWWW();
        String m5251WWWWWWWW = FakeUtils.m5251WWWWWWWW(m5107WWWoWWWo[c10]);
        WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
        wwwwwwww.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{73, TarConstants.LF_BLK, -57}, new byte[]{5, 96, -126, -123, -70, 107, -64, -45});
        byte[] bArr2 = {-111, -38, -21, -49, TarConstants.LF_SYMLINK, -10, -104, -35};
        wwwwwwww.getClass();
        String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-10, -75, -124, -85}, bArr2);
        byte[] bArr3 = {-84, 112, -101, TarConstants.LF_PAX_EXTENDED_HEADER_LC};
        byte[] bArr4 = {-60, 31, -10, 29, 96, 34, 58, TarConstants.LF_FIFO};
        wwwwwwww.getClass();
        String m17835WWWWWWWW3 = WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4);
        byte[] bArr5 = {124, 102, -7, -39, -57, -30, -124, TarConstants.LF_GNUTYPE_LONGNAME};
        wwwwwwww.getClass();
        String m17835WWWWWWWW4 = WWWWWWWW.m17835WWWWWWWW(new byte[]{18, 9, -105, -68}, bArr5);
        wwwwwwww.getClass();
        String m17835WWWWWWWW5 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-90, 67, -74, 0}, new byte[]{-56, 44, -40, 101, -81, -16, -87, -60});
        String m5253WWWWWWWW = FakeUtils.m5253WWWWWWWW();
        String m5252WWWWWWWW = FakeUtils.m5252WWWWWWWW();
        byte[] bArr6 = {95, ConstantPoolEntry.CP_InterfaceMethodref, 100, 123, 26, -116, 17, 94, 95, 28, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER};
        byte[] bArr7 = {110, TarConstants.LF_SYMLINK, 86, 85, 43, -70, 41, 112};
        wwwwwwww.getClass();
        String m17835WWWWWWWW6 = WWWWWWWW.m17835WWWWWWWW(bArr6, bArr7);
        StringBuilder sb3 = new StringBuilder();
        byte[] bArr8 = {-106, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 1, -76, -14, -20, -45, TarConstants.LF_GNUTYPE_SPARSE, -106, 79};
        byte[] bArr9 = {-89, 97, TarConstants.LF_CHR, -102, -61, -38, -21, 125};
        wwwwwwww.getClass();
        sb3.append(WWWWWWWW.m17835WWWWWWWW(bArr8, bArr9));
        sb3.append(i10 + 2);
        String sb4 = sb3.toString();
        String m5252WWWWWWWW2 = FakeUtils.m5252WWWWWWWW();
        VMApp vMApp = this.f8951WWWWWWWW;
        StringBuilder sb5 = new StringBuilder();
        wwwwwwww.getClass();
        sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{57, Byte.MAX_VALUE, 17, -82, 125, -115, 5, 96, 40, TarConstants.LF_MULTIVOLUME}, new byte[]{79, 18, 78, -51, 18, -29, 99, 9}));
        sb5.append(i10);
        SharedPreferences.Editor edit = vMApp.getSharedPreferences(sb5.toString(), 0).edit();
        wwwwwwww.getClass();
        SharedPreferences.Editor putString = edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -28, 108, -47}, new byte[]{37, -123, 1, -76, 109, -64, 109, -15}), str2);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString2 = putString.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{80, -34, 107, -117, 73, -65}, new byte[]{35, -69, 25, -30, 40, -45, -60, -16}), m5254WWWWWWWW);
        byte[] bArr10 = {122, -50, 2, 4, 7, -28, 22, TarConstants.LF_CHR};
        wwwwwwww.getClass();
        SharedPreferences.Editor putBoolean = putString2.putBoolean(WWWWWWWW.m17835WWWWWWWW(new byte[]{19, -67, 93, 99, 116, -119, 73, 67, 18, -95, 108, 97}, bArr10), true);
        byte[] bArr11 = {TarConstants.LF_PAX_EXTENDED_HEADER_LC, 35, 17, 110, 2, 18, 27, -49};
        wwwwwwww.getClass();
        SharedPreferences.Editor putString3 = putBoolean.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{17, 78, 116, 7}, bArr11), m5250WWWWWWWW);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString4 = putString3.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-122, 108, 27, 39, 2, -62, 102}, new byte[]{-17, 1, 126, 78, 93, -79, 16, -40}), m5257WWoWWo);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString5 = putString4.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-120, 100, -34, 17, 122, 107, 65, 2, -114}, new byte[]{-22, 5, -83, 116, 37, 9, 32, 108}), m5247WWWWoWWWWo);
        byte[] bArr12 = {ConstantPoolEntry.CP_NameAndType, 59, -64, -75, TarConstants.LF_CONTIG, 44, 93, -126};
        wwwwwwww.getClass();
        SharedPreferences.Editor putBoolean2 = putString5.putBoolean(WWWWWWWW.m17835WWWWWWWW(new byte[]{Byte.MAX_VALUE, 82, -83, -22, 82, 66, 60, -32, 96, 94, -92}, bArr12), true);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString6 = putBoolean2.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-25, -51, -37, -6, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -72, -67, -52, -3, -63, -60}, new byte[]{-108, -92, -74, -91, 59, -39, -49, -66}), m5107WWWoWWWo[0]);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString7 = putString6.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-97, -5, 7, -58, -5, -69, -11}, new byte[]{-20, -110, 106, -103, -120, -53, -101, -27}), m5107WWWoWWWo[1]);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString8 = putString7.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-9, -33, -40, -56, 47, -82, -31, -118, -22, -43}, new byte[]{-124, -74, -75, -105, 66, -51, -126, -25}), m5107WWWoWWWo[c10]);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString9 = putString8.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{2, -55, 122, 81, 78, 109, TarConstants.LF_CONTIG, 102, 21}, new byte[]{113, -96, 23, 14, 39, 14, 84, 15}), m5249WWWWWWWW);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString10 = putString9.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, 96, 65, 36, -28, -7, -59, -90}, new byte[]{68, 9, 44, 123, -115, -108, -74, -49}), m5251WWWWWWWW);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString11 = putString10.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-114, 70, 117, -120, -45, TarConstants.LF_BLK, -45, -14, -118, 89, 117, -108, -35, TarConstants.LF_BLK, -34, -10, -116, 92, 115, -125, -60}, new byte[]{-2, 46, 26, -26, -74, 107, -67, -105}), m5107WWWoWWWo[0]);
        byte[] bArr13 = {122, -37, 101, 84, -13, 7, TarConstants.LF_NORMAL, -30, 126, -60, 101, 72, -3, 7, 45, -9, 100};
        byte[] bArr14 = {10, -77, 10, 58, -106, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 94, -121};
        wwwwwwww.getClass();
        SharedPreferences.Editor putString12 = putString11.putString(WWWWWWWW.m17835WWWWWWWW(bArr13, bArr14), m5107WWWoWWWo[1]);
        byte[] bArr15 = {37, 73, -99, -62, -40, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 73, 31};
        wwwwwwww.getClass();
        SharedPreferences.Editor putString13 = putString12.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{85, 33, -14, -84, -67, 56, 39, 122, 81, 62, -14, -80, -77, 56, 36, 124, 70, 36, -13, -95}, bArr15), m5107WWWoWWWo[c10]);
        byte[] bArr16 = {66, -24, 69, -95, 66, -110, -1, 36, 70, -9, 69, -67, TarConstants.LF_GNUTYPE_LONGNAME, -110, -27, 56, 66, -27};
        byte[] bArr17 = {TarConstants.LF_SYMLINK, Byte.MIN_VALUE, 42, -49, 39, -51, -111, 65};
        wwwwwwww.getClass();
        SharedPreferences.Editor putString14 = putString13.putString(WWWWWWWW.m17835WWWWWWWW(bArr16, bArr17), m17835WWWWWWWW);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString15 = putString14.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-115, 42, -3, 2, 102, -27, -18, 22, -102, 44, -13, 0, 92, -55, -23, 13, -104, 44, -11, 24, 107}, new byte[]{-3, 66, -110, 108, 3, -70, -99, Byte.MAX_VALUE}), m17835WWWWWWWW2);
        byte[] bArr18 = {67, 28, -59, -67, 26, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, Byte.MIN_VALUE, -125, 71, 3, -59, -95, 20, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -99, -110, 82, 0, -33, -96};
        byte[] bArr19 = {TarConstants.LF_CHR, 116, -86, -45, Byte.MAX_VALUE, 56, -18, -26};
        wwwwwwww.getClass();
        SharedPreferences.Editor putString16 = putString15.putString(WWWWWWWW.m17835WWWWWWWW(bArr18, bArr19), m17835WWWWWWWW3);
        byte[] bArr20 = {-81, -10, 32, 61, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -14, 73, -116, -84, -63, 32, 35, 105, -60, 85, -113};
        byte[] bArr21 = {-33, -98, 79, TarConstants.LF_GNUTYPE_SPARSE, 29, -83, 58, -31};
        wwwwwwww.getClass();
        SharedPreferences.Editor putString17 = putString16.putString(WWWWWWWW.m17835WWWWWWWW(bArr20, bArr21), m17835WWWWWWWW4);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString18 = putString17.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{113, -45, 104, TarConstants.LF_BLK, -45, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 24, 68, 96, -41, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_DIR, -58, 115, 21, 66, 111}, new byte[]{1, -69, 7, 90, -74, 7, 124, 45}), m17835WWWWWWWW5);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString19 = putString18.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, 86, -32, 110, -94, -55, -100, 35, 95}, new byte[]{59, 63, -122, 7, -3, -70, -17, 74}), m5253WWWWWWWW);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString20 = putString19.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-8, Byte.MIN_VALUE, 19, -119, -94, 112, -81, 61, -26, -115}, new byte[]{-113, -23, 117, -32, -3, 18, -36, 78}), m5252WWWWWWWW);
        byte[] bArr22 = {98, 109, -50, 24, 59, -98, 31, TarConstants.LF_DIR};
        wwwwwwww.getClass();
        SharedPreferences.Editor putString21 = putString20.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{21, 4, -88, 113, 100, -9, 111}, bArr22), m17835WWWWWWWW6);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString22 = putString21.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{126, 41, -71, 122, -90, -122, TarConstants.LF_LINK, 60}, new byte[]{9, 64, -33, 19, -7, -21, 80, 95}), m5252WWWWWWWW);
        wwwwwwww.getClass();
        SharedPreferences.Editor putString23 = putString22.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-36, 46, -11, -7, TarConstants.LF_LINK, 21, 25, 82}, new byte[]{-80, 65, -106, -104, 93, 74, 112, 34}), sb4);
        byte[] bArr23 = {ConstantPoolEntry.CP_NameAndType, 29, 124, -9, 111, -59, -73, -103};
        wwwwwwww.getClass();
        SharedPreferences.Editor putString24 = putString23.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{96, 114, 31, -106, 3, -102, -38, -8, 111}, bArr23), m5252WWWWWWWW2);
        byte[] bArr24 = {-11, -60, -68, 99, 63, 45, 91, ConstantPoolEntry.CP_InterfaceMethodref, -18, -52};
        byte[] bArr25 = {-121, -85, -47, 60, 92, 66, TarConstants.LF_DIR, 109};
        wwwwwwww.getClass();
        SharedPreferences.Editor putString25 = putString24.putString(WWWWWWWW.m17835WWWWWWWW(bArr24, bArr25), romConfig.m5048WWWoWWWo());
        wwwwwwww.getClass();
        SharedPreferences.Editor putString26 = putString25.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-119, -124, -96, -60, -111, -86, 6, -109, -114, -126, -67, -46, -108, -84, 32, -94, -116, Byte.MIN_VALUE, -74}, new byte[]{-19, -19, -45, -76, -3, -53, Byte.MAX_VALUE, -52}), vMResConfig.f8953WWWWWWWW);
        wwwwwwww.getClass();
        SharedPreferences.Editor putInt = putString26.putInt(WWWWWWWW.m17835WWWWWWWW(new byte[]{4, -54, 73, 107, -67, -121, 45, -34, 23, -54, 94, 111, -71}, new byte[]{96, -93, 58, 27, -47, -26, 84, -127}), vMResConfig.f8952WWWWoWWWWo);
        wwwwwwww.getClass();
        SharedPreferences.Editor putInt2 = putInt.putInt(WWWWWWWW.m17835WWWWWWWW(new byte[]{-63, -93, -24, -24, -24, -24, 105, -79, -51, -81, -14, -1, -20, -3}, new byte[]{-91, -54, -101, -104, -124, -119, 16, -18}), vMResConfig.f8955WWWoWWWo);
        byte[] bArr26 = {TarConstants.LF_GNUTYPE_SPARSE, 60, 78, 24, -54, 124, 72, 123, TarConstants.LF_GNUTYPE_SPARSE, 37, 84};
        byte[] bArr27 = {TarConstants.LF_CONTIG, 85, 61, 104, -90, 29, TarConstants.LF_LINK, 36};
        wwwwwwww.getClass();
        SharedPreferences.Editor putInt3 = putInt2.putInt(WWWWWWWW.m17835WWWWWWWW(bArr26, bArr27), vMResConfig.f8954WWWWWWWW);
        wwwwwwww.getClass();
        SharedPreferences.Editor putLong = putInt3.putLong(WWWWWWWW.m17835WWWWWWWW(new byte[]{35, -40, TarConstants.LF_LINK, -90, TarConstants.LF_NORMAL, 56, -33, -35, 41, -57, TarConstants.LF_LINK}, new byte[]{64, -86, 84, -57, 68, 93, Byte.MIN_VALUE, -87}), System.currentTimeMillis());
        wwwwwwww.getClass();
        putLong.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{113, 60, -56, 29, 90, -22, 101, -12, 125, 35}, new byte[]{18, 78, -83, 124, 46, -113, 58, -122}), romConfig.m5048WWWoWWWo()).apply();
        VMInstance vMInstance = new VMInstance(this.f8951WWWWWWWW, i10);
        synchronized (this.f8950WWWWoWWWWo) {
            this.f8950WWWWoWWWWo.add(vMInstance);
        }
        C2467WWWWWWWW.m13936WWWWoWWWWo().m13940WWWWWWWW(new VMCreationEvent(vMInstance));
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final VMInstance m5104WWWWWWWW(VMConfig vMConfig) {
        int i10 = f8947WWWWWWWW;
        f8947WWWWWWWW = i10 + 1;
        VMApp vMApp = this.f8951WWWWWWWW;
        SharedPreferences.Editor putString = vMApp.getSharedPreferences(StringFog.m5049WWWWWWWW(new byte[]{44, 33, 29, 126, -116, -7, -42, -1, 61, 19}, new byte[]{90, TarConstants.LF_GNUTYPE_LONGNAME, 66, 29, -29, -105, -80, -106}) + i10, 0).edit().putString(StringFog.m5049WWWWWWWW(new byte[]{112, 42, -19, -37}, new byte[]{30, TarConstants.LF_GNUTYPE_LONGLINK, Byte.MIN_VALUE, -66, -7, 17, 24, 124}), vMConfig.f8861WWWWoWWWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{89, 33, -29, -58, 114, 16}, new byte[]{42, 68, -111, -81, 19, 124, 118, -5}), vMConfig.f8869WWWWWWWW).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{-78, 69, 82, -8, 6, 65, 64, 17, -77, 89, 99, -6}, new byte[]{-37, TarConstants.LF_FIFO, 13, -97, 117, 44, 31, 97}), vMConfig.f8896WWWoWWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{-20, 126, 117, -57}, new byte[]{-123, 19, 16, -82, -102, TarConstants.LF_GNUTYPE_LONGLINK, 78, 122}), vMConfig.f8871WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{-74, -106, -73, 102, TarConstants.LF_BLK, 23, 78}, new byte[]{-33, -5, -46, 15, 107, 100, 56, -69}), vMConfig.f8872WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{5, -17, -19, 116}, new byte[]{104, -118, -124, 16, -105, 30, 65, -56}), vMConfig.f8914WWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{68, -125, 84}, new byte[]{33, -16, 58, 8, -3, -29, TarConstants.LF_BLK, 28}), vMConfig.f8920WoWo).putString(StringFog.m5049WWWWWWWW(new byte[]{24, 9, -106, 125, 116, 111, -61, -93, 30}, new byte[]{122, 104, -27, 24, 43, 13, -94, -51}), vMConfig.f8904WWoWWo).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{29, 62, -109, -58, 57, 35, -29, -71, 2, TarConstants.LF_SYMLINK, -102}, new byte[]{110, 87, -2, -103, 92, TarConstants.LF_MULTIVOLUME, -126, -37}), vMConfig.f8873WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{-31, 113, 33, 46, TarConstants.LF_MULTIVOLUME, 91, 3, 85, -5, 125, 62}, new byte[]{-110, 24, TarConstants.LF_GNUTYPE_LONGNAME, 113, 46, 58, 113, 39}), vMConfig.f8874WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{-41, -52, 20, -13, -81, -71, -3}, new byte[]{-92, -91, 121, -84, -36, -55, -109, -76}), vMConfig.f8897WWWoWWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{-39, -69, 23, -87, 74, -123, -38, 125, -60, -79}, new byte[]{-86, -46, 122, -10, 39, -26, -71, 16}), vMConfig.f8905WWoWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{21, 15, 124, -20, -83, -113, -56, -105, 2}, new byte[]{102, 102, 17, -77, -60, -20, -85, -2}), vMConfig.f8875WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_SPARSE, -124, 26, -111, -72, -6, 109, 57}, new byte[]{32, -19, 119, -50, -47, -105, 30, 80}), vMConfig.f8906WWoWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{-127, -112, 65, -92, -54, -70, -53, 18, -105, -90, 66, -114, -41, -80, -63, 14}, new byte[]{-14, -7, 44, -5, -70, -46, -92, 124}), vMConfig.f8898WWWoWWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{-16, 109, -11, 118, TarConstants.LF_FIFO, -2, 74, 106, -12, 114, -11, 106, 56, -2, 71, 110, -14, 119, -13, 125, 33}, new byte[]{Byte.MIN_VALUE, 5, -102, 24, TarConstants.LF_GNUTYPE_SPARSE, -95, 36, 15}), vMConfig.f8924o).putString(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_SYMLINK, -14, -10, 59, 96, -44, Byte.MIN_VALUE, -16, TarConstants.LF_FIFO, -19, -10, 39, 110, -44, -99, -27, 44}, new byte[]{66, -102, -103, 85, 5, -117, -18, -107}), vMConfig.f8907WWoWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{116, 32, TarConstants.LF_CONTIG, -76, 9, -54, -79, TarConstants.LF_GNUTYPE_SPARSE, 112, 63, TarConstants.LF_CONTIG, -88, 7, -54, -78, 85, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 37, TarConstants.LF_FIFO, -71}, new byte[]{4, 72, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -38, 108, -107, -33, TarConstants.LF_FIFO}), vMConfig.f8876WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{-99, -29, 122, TarConstants.LF_BLK, 106, 70, -24, -64, -103, -4, 122, 40, 100, 70, -14, -36, -99, -18}, new byte[]{-19, -117, 21, 90, 15, 25, -122, -91}), vMConfig.f8862WWWWoWWWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{-81, TarConstants.LF_FIFO, 93, 106, 19, -25, -119, -92, -72, TarConstants.LF_NORMAL, TarConstants.LF_GNUTYPE_SPARSE, 104, 41, -53, -114, -65, -70, TarConstants.LF_NORMAL, 85, 112, 30}, new byte[]{-33, 94, TarConstants.LF_SYMLINK, 4, 118, -72, -6, -51}), vMConfig.f8860WWWWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{124, 115, -70, TarConstants.LF_CHR, -35, 19, -13, -16, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 108, -70, 47, -45, 19, -18, -31, 109, 111, -96, 46}, new byte[]{ConstantPoolEntry.CP_NameAndType, 27, -43, 93, -72, TarConstants.LF_GNUTYPE_LONGNAME, -99, -107}), vMConfig.f8877WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{-67, 32, 47, -75, -113, 98, -119, -104, -66, 23, 47, -85, -98, 84, -107, -101}, new byte[]{-51, 72, 64, -37, -22, 61, -6, -11}), vMConfig.f8915WWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{-42, 8, -86, -126, 116, 86, TarConstants.LF_GNUTYPE_LONGLINK, -103, -57, ConstantPoolEntry.CP_NameAndType, -102, -125, 97, 125, 70, -97, -56}, new byte[]{-90, 96, -59, -20, 17, 9, 47, -16}), vMConfig.f8878WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{64, 65, 35, -29, 85, -82, 81, -53, TarConstants.LF_GNUTYPE_SPARSE}, new byte[]{TarConstants.LF_CONTIG, 40, 69, -118, 10, -35, 34, -94}), vMConfig.f8908WWoWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{-70, -123, -26, 106, -94, -37, 1, -113, -92, -120}, new byte[]{-51, -20, Byte.MIN_VALUE, 3, -3, -71, 114, -4}), vMConfig.f8879WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{79, -87, -27, 8, -81, -67, 98}, new byte[]{56, -64, -125, 97, -16, -44, 18, 114}), vMConfig.f8916WW).putString(StringFog.m5049WWWWWWWW(new byte[]{112, -64, 114, 26, -23, -49, -100, 1}, new byte[]{7, -87, 20, 115, -74, -94, -3, 98}), vMConfig.f8899WWWoWWWo).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{118, -19, 13, 35, 41, 79, -28, -26, 99, -20, TarConstants.LF_CHR, 61, 37, 99, -25}, new byte[]{5, -123, 108, 81, TarConstants.LF_GNUTYPE_LONGNAME, 16, -109, -113}), vMConfig.f8921WoWo).putString(StringFog.m5049WWWWWWWW(new byte[]{-71, 115, -14, -76, -43, 32, -115, Byte.MAX_VALUE}, new byte[]{-43, 28, -111, -43, -71, Byte.MAX_VALUE, -28, 15}), vMConfig.f8909WWoWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{121, 2, 84, 4, 101, 96, 113, 89, 118}, new byte[]{21, 109, TarConstants.LF_CONTIG, 101, 9, 63, 28, 56}), vMConfig.f8880WWWWWWWW).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{82, 95, 29, 92, -26, -54, -5, 15, 79, 81, 28, 91, -16, -101}, new byte[]{33, TarConstants.LF_NORMAL, 126, TarConstants.LF_CONTIG, -107, -1, -92, 106}), vMConfig.f8863WWWWoWWWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{121, -53, -6, 20, 119, -4, -14, 64, 111, -42, -17, 26, 118}, new byte[]{10, -92, -103, Byte.MAX_VALUE, 4, -55, -83, TarConstants.LF_CHR}), vMConfig.f8881WWWWWWWW).putInt(StringFog.m5049WWWWWWWW(new byte[]{56, 25, -38, -67, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -44, 41, 118, 36, 4, -51}, new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, 118, -71, -42, 43, -31, 118, 6}), vMConfig.f8882WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{-7, TarConstants.LF_SYMLINK, -53, -18, TarConstants.LF_CHR, -57, 15, -9, -7, 56, -38, -21, 33, -97, TarConstants.LF_DIR}, new byte[]{-118, 93, -88, -123, 64, -14, 80, -126}), vMConfig.f8883WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{-23, -109, 72, 40, 13, -32, -48, 108, -5, -113, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_BLK, 17, -89, -21}, new byte[]{-102, -4, 43, 67, 126, -43, -113, 28}), vMConfig.f8917WWWW).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{96, 122, -14, -22, -57, 2, -8, 27, 109, 123, -12}, new byte[]{1, 30, -112, -75, -94, 108, -103, 121}), vMConfig.f8910WWoWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{-108, 74, -105, -26, -27, 105, -31, -35, -109, TarConstants.LF_GNUTYPE_LONGNAME, -118, -16, -32, 111, -57, -20, -111, 78, -127}, new byte[]{-16, 35, -28, -106, -119, 8, -104, -126}), vMConfig.f8900WWWoWWWo.f8953WWWWWWWW).putInt(StringFog.m5049WWWWWWWW(new byte[]{-87, -30, 97, -73, TarConstants.LF_CHR, -88, TarConstants.LF_BLK, 92, -70, -30, 118, -77, TarConstants.LF_CONTIG}, new byte[]{-51, -117, 18, -57, 95, -55, TarConstants.LF_MULTIVOLUME, 3}), vMConfig.f8900WWWoWWWo.f8952WWWWoWWWWo).putInt(StringFog.m5049WWWWWWWW(new byte[]{-92, -66, 63, 25, 68, -61, -5, -63, -88, -78, 37, 14, 64, -42}, new byte[]{-64, -41, TarConstants.LF_GNUTYPE_LONGNAME, 105, 40, -94, -126, -98}), vMConfig.f8900WWWoWWWo.f8955WWWoWWWo).putInt(StringFog.m5049WWWWWWWW(new byte[]{-10, -85, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -120, -71, 125, -93, 16, -10, -78, 125}, new byte[]{-110, -62, 20, -8, -43, 28, -38, 79}), vMConfig.f8900WWWoWWWo.f8954WWWWWWWW).putInt(StringFog.m5049WWWWWWWW(new byte[]{17, 8, 117, -31, -105, -119, -4, -16, 17, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -10}, new byte[]{99, 109, 19, -109, -14, -6, -108, -81}), vMConfig.f8918WW).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{117, 98, -63, 16, 79, TarConstants.LF_CHR, -97, 67, 105, 104, -48, 29, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 9, -126, 65, 121, 97, -48}, new byte[]{27, 13, -75, 115, 39, 108, -20, 32}), vMConfig.f8922WoWo).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{80, -31, 25, -116, -43, 115, 82, 72, 92, -11, 6, -65, -61, 123, 78}, new byte[]{TarConstants.LF_SYMLINK, -108, 112, -32, -95, 26, 60, 23}), vMConfig.f8887WWWWWWWW).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{97, ConstantPoolEntry.CP_NameAndType, -87, -102, -21, 56, -102, -107, 109, 24, -74, -87, -3, TarConstants.LF_NORMAL, -122, -107, 113, 13, -84}, new byte[]{3, 121, -64, -10, -97, 81, -12, -54}), vMConfig.f8912WWoWWo).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{-125, -37, -105, 27, -116, 4, -60, -118, -114, -37, -104, 18, -101, 1}, new byte[]{-32, -70, -6, 126, -2, 101, -101, -17}), vMConfig.f8888WWWWWWWW).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{121, -20, 13, -8, 31, -67, 38, -80, 99, -12, 14, -27, 27, -112, 33}, new byte[]{10, -124, 108, -118, 122, -30, 69, -36}), vMConfig.f8889WWWWWWWW).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{-41, 24, 13, 22, 67, 10, TarConstants.LF_CONTIG, -48, -56, 20, 9, 22}, new byte[]{-92, 112, 108, 100, 38, 85, 81, -65}), vMConfig.f8890WWWWWWWW).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_CHR, 23, -42, TarConstants.LF_LINK, -79, Byte.MIN_VALUE, 66, 1, TarConstants.LF_BLK, 22, -47, 42, -73, -66, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 7, 47, 17}, new byte[]{64, Byte.MAX_VALUE, -73, 67, -44, -33, 44, 110}), vMConfig.f8865WWWWoWWWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{86, 18, -29, 32, TarConstants.LF_CHR, 68, -11, -117, 101, 9, -7, TarConstants.LF_LINK, 34}, new byte[]{58, 125, Byte.MIN_VALUE, 65, 71, 45, -102, -27}), vMConfig.f8901WWWoWWWo).putString(StringFog.m5049WWWWWWWW(new byte[]{113, 34, -123, -72, 122, -107, Byte.MAX_VALUE, TarConstants.LF_MULTIVOLUME, 101, 37, -119, -91, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{4, 81, -32, -54, 37, -7, 16, 46}), vMConfig.f8891WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{-1, 85, -97, 121, 6, -46, 1, -60, -9, 87}, new byte[]{-104, 37, -22, 38, 112, -73, 111, -96}), vMConfig.f8892WWWWWWWW).putString(StringFog.m5049WWWWWWWW(new byte[]{-26, 20, -30, -48, -73, 14, 3, TarConstants.LF_BLK, -28, 22, -14, -3}, new byte[]{-127, 100, -105, -113, -59, 107, 109, 80}), vMConfig.f8893WWWWWWWW).putLong(StringFog.m5049WWWWWWWW(new byte[]{3, 73, -82, -65, 107, -10, -31, 42, 9, 86, -82}, new byte[]{96, 59, -53, -34, 31, -109, -66, 94}), System.currentTimeMillis()).putString(StringFog.m5049WWWWWWWW(new byte[]{104, TarConstants.LF_FIFO, 109, 3, 44, 80, -95, 117, 115, 62}, new byte[]{26, 89, 0, 92, 79, 63, -49, 19}), vMConfig.f8895WWWoWWWo.m5048WWWoWWWo()).putString(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_MULTIVOLUME, -32, TarConstants.LF_FIFO, -36, -113, 111, 86, -127, 65, -1}, new byte[]{46, -110, TarConstants.LF_GNUTYPE_SPARSE, -67, -5, 10, 9, -13}), vMConfig.f8895WWWoWWWo.m5048WWWoWWWo());
        if (vMConfig.f8885WWWWWWWW) {
            putString.putBoolean(StringFog.m5049WWWWWWWW(new byte[]{-77, -10, -125, 31, TarConstants.LF_FIFO, 18, 15, -111, -80, -10, -122, 26, 32, 29}, new byte[]{-34, -105, -28, 118, 69, 121, 80, -12}), vMConfig.f8885WWWWWWWW);
        }
        if (vMConfig.f8902WWWWWW) {
            putString.putBoolean(StringFog.m5049WWWWWWWW(new byte[]{-66, 78, 23, -63, 60, -8, -61, -41, -82, TarConstants.LF_MULTIVOLUME, 29, -47}, new byte[]{-52, 33, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -75, 99, -99, -83, -74}), vMConfig.f8902WWWWWW);
        }
        if (vMConfig.f8911WWoWWo) {
            putString.putBoolean(StringFog.m5049WWWWWWWW(new byte[]{-39, -29, -71, -73, 15, -2, -52, 31, -49, -14, -76, -88, 15, -2}, new byte[]{-95, -109, -42, -60, 106, -102, -109, 122}), vMConfig.f8911WWoWWo);
        }
        if (vMConfig.f8886WWWWWWWW) {
            putString.putBoolean(StringFog.m5049WWWWWWWW(new byte[]{86, -91, -83, -31, 24, 98, 18, 84, 68, -91, -87, -4}, new byte[]{38, -55, -52, -104, 71, 7, 124, TarConstants.LF_DIR}), vMConfig.f8886WWWWWWWW);
        }
        HashMap hashMap = vMConfig.f8870WWWWWWWW;
        if (hashMap != null) {
            for (String str : hashMap.keySet()) {
                putString.putString(str, (String) vMConfig.f8870WWWWWWWW.get(str));
            }
        }
        if (vMConfig.f8864WWWWoWWWWo != null) {
            HashSet hashSet = new HashSet();
            for (String str2 : vMConfig.f8864WWWWoWWWWo.keySet()) {
                StringBuilder m1577WWWWoWWWWo = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(str2);
                m1577WWWWoWWWWo.append(StringFog.m5049WWWWWWWW(new byte[]{3}, new byte[]{62, 25, -94, 16, TarConstants.LF_FIFO, -56, 41, -12}));
                m1577WWWWoWWWWo.append((String) vMConfig.f8864WWWWoWWWWo.get(str2));
                hashSet.add(m1577WWWWoWWWWo.toString());
            }
            putString.putStringSet(StringFog.m5049WWWWWWWW(new byte[]{-122, -17, 85, -44, -26, -107, 116, -109, -108, -26, 78, -62, -6}, new byte[]{-11, -118, 59, -89, -119, -25, 43, -27}), hashSet);
        }
        putString.apply();
        VMInstance vMInstance = new VMInstance(this.f8951WWWWWWWW, i10);
        synchronized (this.f8950WWWWoWWWWo) {
            this.f8950WWWWoWWWWo.add(vMInstance);
        }
        C2467WWWWWWWW.m13936WWWWoWWWWo().m13940WWWWWWWW(new VMCreationEvent(vMInstance));
        return vMInstance;
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public final VMInstance m5105WWWWWWWW(int i10) {
        synchronized (this.f8950WWWWoWWWWo) {
            try {
                ArrayList arrayList = this.f8950WWWWoWWWWo;
                int size = arrayList.size();
                int i11 = 0;
                while (i11 < size) {
                    Object obj = arrayList.get(i11);
                    i11++;
                    VMInstance vMInstance = (VMInstance) obj;
                    if (vMInstance.f8937WWWoWWWo.f8866WWWWWWWW == i10) {
                        return vMInstance;
                    }
                }
                return null;
            } catch (Throwable th2) {
                throw th2;
            }
        }
    }

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public final void m5106WWWWWWWW(SharedPreferences sharedPreferences) {
        if (sharedPreferences.contains(StringFog.m5049WWWWWWWW(new byte[]{-6, 66, 108, -46, 40, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -69, -17, -24}, new byte[]{-115, 43, 10, -69, 119, 9, -38, -126}))) {
            sharedPreferences.edit().putString(StringFog.m5049WWWWWWWW(new byte[]{121, -114, -96, -116, TarConstants.LF_CHR, 66, 93, -39, 106}, new byte[]{14, -25, -58, -27, 108, TarConstants.LF_LINK, 46, -80}), sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{33, 80, 118, TarConstants.LF_BLK, 64, -19, -90, -88, TarConstants.LF_CHR}, new byte[]{86, 57, 16, 93, 31, -125, -57, -59}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING)).remove(StringFog.m5049WWWWWWWW(new byte[]{-72, 108, 24, 4, -16, 8, 86, 36, -86}, new byte[]{-49, 5, 126, 109, -81, 102, TarConstants.LF_CONTIG, 73})).apply();
        }
        if (sharedPreferences.contains(StringFog.m5049WWWWWWWW(new byte[]{21, -46, -127, 19, 26, -61, -49, TarConstants.LF_SYMLINK, ConstantPoolEntry.CP_InterfaceMethodref, -38}, new byte[]{98, -66, -32, 125, 69, -95, -68, 65}))) {
            sharedPreferences.edit().putString(StringFog.m5049WWWWWWWW(new byte[]{-65, -110, TarConstants.LF_BLK, -33, -67, -86, TarConstants.LF_CHR, -28, -95, -97}, new byte[]{-56, -5, 82, -74, -30, -56, 64, -105}), sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{90, -84, -4, -110, -64, -18, 36, 24, 68, -92}, new byte[]{45, -64, -99, -4, -97, -116, 87, 107}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING)).remove(StringFog.m5049WWWWWWWW(new byte[]{-120, -119, -64, 60, 4, 110, TarConstants.LF_LINK, -37, -106, -127}, new byte[]{-1, -27, -95, 82, 91, ConstantPoolEntry.CP_NameAndType, 66, -88})).apply();
        }
        if (sharedPreferences.contains(StringFog.m5049WWWWWWWW(new byte[]{-53, 20, -4, -10, -39, 82, 58, 79}, new byte[]{-68, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -99, -104, -122, 63, 91, 44}))) {
            sharedPreferences.edit().putString(StringFog.m5049WWWWWWWW(new byte[]{123, ConstantPoolEntry.CP_InterfaceMethodref, -72, 94, -73, 72, 124, -62}, new byte[]{ConstantPoolEntry.CP_NameAndType, 98, -34, TarConstants.LF_CONTIG, -24, 37, 29, -95}), sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{15, -127, -58, -4, -54, 96, 91, 29}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, -19, -89, -110, -107, 13, 58, 126}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING)).remove(StringFog.m5049WWWWWWWW(new byte[]{-1, -101, -116, -126, ConstantPoolEntry.CP_NameAndType, 97, -78, 97}, new byte[]{-120, -9, -19, -20, TarConstants.LF_GNUTYPE_SPARSE, ConstantPoolEntry.CP_NameAndType, -45, 2})).apply();
        }
        if (sharedPreferences.contains(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_BLK, -61, 13}, new byte[]{89, -94, 110, -119, -20, -91, -80, -37}))) {
            sharedPreferences.edit().putString(StringFog.m5049WWWWWWWW(new byte[]{86, 43, -86, 2, 16, -82, 123, -81, 89}, new byte[]{58, 68, -55, 99, 124, -15, 22, -50}), sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{45, 74, 78}, new byte[]{64, 43, 45, 47, -10, 4, 9, -34}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING)).remove(StringFog.m5049WWWWWWWW(new byte[]{24, TarConstants.LF_BLK, -52}, new byte[]{117, 85, -81, -32, 25, 99, -43, -72})).apply();
        }
        if (!sharedPreferences.contains(StringFog.m5049WWWWWWWW(new byte[]{56, TarConstants.LF_NORMAL, -121, -82, 0, -64, -21}, new byte[]{81, 93, -30, -57, 95, -77, -99, -91}))) {
            sharedPreferences.edit().putString(StringFog.m5049WWWWWWWW(new byte[]{-58, -43, -35, 124, -53, TarConstants.LF_CONTIG, 45}, new byte[]{-81, -72, -72, 21, -108, 68, 91, -19}), FakeUtils.m5257WWoWWo()).apply();
        }
        if (!sharedPreferences.contains(StringFog.m5049WWWWWWWW(new byte[]{99, 93, 32, -46, -36, 85, -16, 24, 101}, new byte[]{1, 60, TarConstants.LF_GNUTYPE_SPARSE, -73, -125, TarConstants.LF_CONTIG, -111, 118}))) {
            sharedPreferences.edit().putString(StringFog.m5049WWWWWWWW(new byte[]{111, -12, -44, -34, -19, -80, TarConstants.LF_LINK, TarConstants.LF_BLK, 105}, new byte[]{13, -107, -89, -69, -78, -46, 80, 90}), FakeUtils.m5247WWWWoWWWWo()).apply();
        }
        if (sharedPreferences.contains(StringFog.m5049WWWWWWWW(new byte[]{42, 116, 60, 107, -105, 61, -18, -44, 56, 113}, new byte[]{89, 29, 81, TarConstants.LF_BLK, -28, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -100, -67}))) {
            sharedPreferences.edit().putString(StringFog.m5049WWWWWWWW(new byte[]{-17, TarConstants.LF_CHR, 32, -68, -53, -5, 37, 78, -8}, new byte[]{-100, 90, TarConstants.LF_MULTIVOLUME, -29, -94, -104, 70, 39}), sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-93, 3, TarConstants.LF_SYMLINK, 35, 13, 73, -110, 19, -79, 6}, new byte[]{-48, 106, 95, 124, 126, 44, -32, 122}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING)).remove(StringFog.m5049WWWWWWWW(new byte[]{14, 100, -69, 21, 13, -42, 13, -125, 28, 97}, new byte[]{125, 13, -42, 74, 126, -77, Byte.MAX_VALUE, -22})).apply();
        }
        if (sharedPreferences.contains(StringFog.m5049WWWWWWWW(new byte[]{-113, -113, TarConstants.LF_FIFO, -43}, new byte[]{-26, -30, 69, -68, 108, -56, -9, -75}))) {
            sharedPreferences.edit().putString(StringFog.m5049WWWWWWWW(new byte[]{-27, 67, -45, 84, ConstantPoolEntry.CP_InterfaceMethodref, 40, -35, -42}, new byte[]{-106, 42, -66, ConstantPoolEntry.CP_InterfaceMethodref, 98, 69, -82, -65}), sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-8, -102, -80, -27}, new byte[]{-111, -9, -61, -116, -63, 87, 13, 23}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING)).remove(StringFog.m5049WWWWWWWW(new byte[]{25, 0, TarConstants.LF_CONTIG, -120}, new byte[]{112, 109, 68, -31, -77, 93, 69, -125})).apply();
        }
        if (sharedPreferences.contains(StringFog.m5049WWWWWWWW(new byte[]{86, -94, -41, -118, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 65, -16, -82, TarConstants.LF_GNUTYPE_LONGLINK, -88, -35, -106}, new byte[]{38, -54, -72, -28, 29, 30, -98, -37}))) {
            sharedPreferences.edit().putString(StringFog.m5049WWWWWWWW(new byte[]{126, 57, -84, -101, 116, -71, -49, 7, 104, 15, -81, -79, 105, -77, -59, 27}, new byte[]{13, 80, -63, -60, 4, -47, -96, 105}), sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{21, 47, 45, TarConstants.LF_GNUTYPE_SPARSE, ConstantPoolEntry.CP_InterfaceMethodref, 41, -17, 30, 8, 37, 39, 79}, new byte[]{101, 71, 66, 61, 110, 118, -127, 107}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING)).remove(StringFog.m5049WWWWWWWW(new byte[]{-85, -127, 126, TarConstants.LF_LINK, TarConstants.LF_DIR, 126, 93, 39, -74, -117, 116, 45}, new byte[]{-37, -23, 17, 95, 80, 33, TarConstants.LF_CHR, 82})).apply();
        }
        if (!sharedPreferences.contains(StringFog.m5049WWWWWWWW(new byte[]{-117, -22, 61, -7, -96, 21, -38, Byte.MIN_VALUE, -118, -10, ConstantPoolEntry.CP_NameAndType, -5}, new byte[]{-30, -103, 98, -98, -45, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -123, -16}))) {
            String[] m5107WWWoWWWo = m5107WWWoWWWo();
            sharedPreferences.edit().putBoolean(StringFog.m5049WWWWWWWW(new byte[]{-124, -96, 5, -66, -81, -86, -92, TarConstants.LF_FIFO, -123, -68, TarConstants.LF_BLK, -68}, new byte[]{-19, -45, 90, -39, -36, -57, -5, 70}), true).putBoolean(StringFog.m5049WWWWWWWW(new byte[]{-24, 99, 66, TarConstants.LF_BLK, -101, -29, -102, TarConstants.LF_NORMAL, -9, 111, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{-101, 10, 47, 107, -2, -115, -5, 82}), true).putString(StringFog.m5049WWWWWWWW(new byte[]{-106, -119, 93, 86, -77, -69, -64, -103, -116, -123, 66}, new byte[]{-27, -32, TarConstants.LF_NORMAL, 9, -48, -38, -78, -21}), m5107WWWoWWWo[0]).putString(StringFog.m5049WWWWWWWW(new byte[]{-13, -84, 63, -10, -64, -124, -22}, new byte[]{Byte.MIN_VALUE, -59, 82, -87, -77, -12, -124, 100}), m5107WWWoWWWo[1]).putString(StringFog.m5049WWWWWWWW(new byte[]{121, 37, -15, -44, -94, -73, 24, 26, 100, 47}, new byte[]{10, TarConstants.LF_GNUTYPE_LONGNAME, -100, -117, -49, -44, 123, 119}), m5107WWWoWWWo[2]).putString(StringFog.m5049WWWWWWWW(new byte[]{57, -95, 60, -102, TarConstants.LF_FIFO, -89, -54, -69, 61, -66, 60, -122, 56, -89, -57, -65, 59, -69, 58, -111, 33}, new byte[]{73, -55, TarConstants.LF_GNUTYPE_SPARSE, -12, TarConstants.LF_GNUTYPE_SPARSE, -8, -92, -34}), m5107WWWoWWWo[0]).putString(StringFog.m5049WWWWWWWW(new byte[]{2, -116, 66, 89, -63, -87, -91, 117, 6, -109, 66, 69, -49, -87, -72, 96, 28}, new byte[]{114, -28, 45, TarConstants.LF_CONTIG, -92, -10, -53, 16}), m5107WWWoWWWo[1]).putString(StringFog.m5049WWWWWWWW(new byte[]{-4, -72, 10, 10, 92, -30, -9, -69, -8, -89, 10, 22, 82, -30, -12, -67, -17, -67, ConstantPoolEntry.CP_InterfaceMethodref, 7}, new byte[]{-116, -48, 101, 100, 57, -67, -103, -34}), m5107WWWoWWWo[2]).putString(StringFog.m5049WWWWWWWW(new byte[]{-115, -57, -21, 28, -126, 82, 97, -104, -119, -40, -21, 0, -116, 82, 123, -124, -115, -54}, new byte[]{-3, -81, -124, 114, -25, 13, 15, -3}), StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_NORMAL, 99, ConstantPoolEntry.CP_InterfaceMethodref}, new byte[]{124, TarConstants.LF_CONTIG, 78, -103, -75, -78, -97, -124})).putString(StringFog.m5049WWWWWWWW(new byte[]{-40, TarConstants.LF_CONTIG, -33, -49, -2, 116, -19, 121, -49, TarConstants.LF_LINK, -47, -51, -60, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -22, 98, -51, TarConstants.LF_LINK, -41, -43, -13}, new byte[]{-88, 95, -80, -95, -101, 43, -98, 16}), StringFog.m5049WWWWWWWW(new byte[]{-4, 47, 20, -101}, new byte[]{-101, 64, 123, -1, -97, -100, -63, 25})).putString(StringFog.m5049WWWWWWWW(new byte[]{-108, -32, 66, -95, 29, -3, -67, 33, -112, -1, 66, -67, 19, -3, -96, TarConstants.LF_NORMAL, -123, -4, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -68}, new byte[]{-28, -120, 45, -49, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -94, -45, 68}), StringFog.m5049WWWWWWWW(new byte[]{-125, 91, -5, -34}, new byte[]{-21, TarConstants.LF_BLK, -106, -69, 85, 22, 59, 98})).apply();
        }
        if (!sharedPreferences.contains(StringFog.m5049WWWWWWWW(new byte[]{90, 66, -125, -22, 62, -99, -82, 15, 89, 117, -125, -12, 47, -85, -78, ConstantPoolEntry.CP_NameAndType}, new byte[]{42, 42, -20, -124, 91, -62, -35, 98}))) {
            sharedPreferences.edit().putString(StringFog.m5049WWWWWWWW(new byte[]{40, -3, -63, -23, 91, 95, 126, -8, 43, -54, -63, -9, 74, 105, 98, -5}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_UC, -107, -82, -121, 62, 0, 13, -107}), StringFog.m5049WWWWWWWW(new byte[]{-21, 87, -75, 78}, new byte[]{-123, 56, -37, 43, 104, -71, -64, 90})).putString(StringFog.m5049WWWWWWWW(new byte[]{8, 14, -115, TarConstants.LF_MULTIVOLUME, 74, 84, -93, 114, 25, 10, -67, TarConstants.LF_GNUTYPE_LONGNAME, 95, Byte.MAX_VALUE, -82, 116, 22}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, 102, -30, 35, 47, ConstantPoolEntry.CP_InterfaceMethodref, -57, 27}), StringFog.m5049WWWWWWWW(new byte[]{-21, 9, -32, 117}, new byte[]{-123, 102, -114, 16, -56, 89, -31, -47})).apply();
        }
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final String[] m5107WWWoWWWo() {
        String str;
        String str2;
        int i10;
        boolean z10;
        String m17835WWWWWWWW;
        StringFog.f8859WWWWWWWW.getClass();
        TelephonyManager telephonyManager = (TelephonyManager) this.f8951WWWWWWWW.getSystemService(WWWWWWWW.m17835WWWWWWWW(new byte[]{33, -64, 85, 80, -77}, new byte[]{81, -88, 58, 62, -42, -73, -36, -104}));
        if (telephonyManager != null && Build.VERSION.SDK_INT >= 28) {
            i10 = telephonyManager.getSimCarrierId();
            str = telephonyManager.getSimOperatorName();
            str2 = telephonyManager.getSimOperator();
        } else {
            str = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
            str2 = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
            i10 = 0;
        }
        if (i10 != 0 && !TextUtils.isEmpty(str) && !TextUtils.isEmpty(str2)) {
            z10 = true;
        } else {
            z10 = false;
        }
        if (z10) {
            m17835WWWWWWWW = Integer.toString(i10);
        } else {
            m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-6}, new byte[]{-53, 68, -93, -113, ConstantPoolEntry.CP_NameAndType, 22, -45, -39});
        }
        if (!z10) {
            str = WWWWWWWW.m17835WWWWWWWW(new byte[]{28, -108, -27, 98, -54, -7, 73, 42}, new byte[]{72, -71, -88, 13, -88, -112, 37, 79});
        }
        if (!z10) {
            str2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{13, -35, -114, -95, -56, -127}, new byte[]{62, -20, -66, -109, -2, -79, 23, -120});
        }
        return new String[]{m17835WWWWWWWW, str, str2};
    }

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public final List m5108WWoWWo() {
        List unmodifiableList;
        synchronized (this.f8950WWWWoWWWWo) {
            unmodifiableList = DesugarCollections.unmodifiableList(this.f8950WWWWoWWWWo);
        }
        return unmodifiableList;
    }
}
