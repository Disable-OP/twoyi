package com.android.vmcore.hal;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.os.BatteryManager;
import android.os.Build;
import android.os.Handler;
import android.os.HandlerThread;
import android.text.TextUtils;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.blankj.utilcode.util.WWWW;
import com.blankj.utilcode.util.WWWWoWWWWo;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.io.File;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.compressors.bzip2.BZip2Constants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p001WWWWoWWWWo.RunnableC0056WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class BatteryService extends BroadcastReceiver {

    /* renamed from: WWWW֭WWWWྥ֭  reason: contains not printable characters */
    public static final String f9018WWWWWWWW;

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final BatteryManager f9019WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final Context f9020WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final File f9021WWWWWWWW;

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public final File f9022WWWWWWWW;

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public boolean f9023WWWWWWWW;

    /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
    public HandlerThread f9024WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final File f9025WWWoWWWo;

    /* renamed from: WWWoԻWWWoͷԻ  reason: contains not printable characters */
    public Handler f9026WWWoWWWo;

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public final File f9027WWoWWo;

    static {
        StringFog.f8859WWWWWWWW.getClass();
        f9018WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, -98, -95, 114, -14, TarConstants.LF_FIFO, -1, 39, -110, -115, -93, 111, -12, 33}, new byte[]{-9, -1, -43, 6, -105, 68, -122, 116});
    }

    public BatteryService(Context context, VMInstance vMInstance) {
        this.f9020WWWWWWWW = context;
        StringFog.f8859WWWWWWWW.getClass();
        this.f9019WWWWoWWWWo = (BatteryManager) context.getSystemService(WWWWWWWW.m17835WWWWWWWW(new byte[]{-16, 19, -24, -116, 121, -122, -85, -49, -13, 28, -3, -97, 121, -122}, new byte[]{-110, 114, -100, -8, 28, -12, -46, -94}));
        VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
        File file = new File(vMConfig.f8867WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{35, -56, 9, 112, Byte.MAX_VALUE, -123, 32, -50, 35, -36, 3, 113, TarConstants.LF_DIR, -97, 30, -47, 121, -36, 28, 106, 41, -62, 35, -61, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -40, 9, 116, 41}, new byte[]{ConstantPoolEntry.CP_NameAndType, -84, 108, 6, 80, -19, 65, -94}));
        this.f9025WWWoWWWo = file;
        WWWWoWWWWo.m5284WWWWWWWW(file);
        File file2 = new File(vMConfig.f8867WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{61, -15, -91, -38, 118, -78, Byte.MIN_VALUE, 93, 61, -27, -81, -37, 60, -88, -66, 66, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -27, -80, -64, 32, -11, -123, 82}, new byte[]{18, -107, -64, -84, 89, -38, -31, TarConstants.LF_LINK}));
        this.f9021WWWWWWWW = file2;
        WWWWoWWWWo.m5284WWWWWWWW(file2);
        File file3 = new File(vMConfig.f8867WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-10, 38, -119, 97, -87, -100, 17, 9, -10, TarConstants.LF_SYMLINK, -125, 96, -29, -122, 47, 22, -84, TarConstants.LF_SYMLINK, -100, 123, -1, -37, 5, 22, -69}, new byte[]{-39, 66, -20, 23, -122, -12, 112, 101}));
        this.f9022WWWWWWWW = file3;
        WWWWoWWWWo.m5284WWWWWWWW(file3);
        File file4 = new File(vMConfig.f8867WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-47, 82, 92, 65, -90, -14, -88, -32, -47, 70, 86, 64, -20, -24, -106, -7, -114, 82, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 67, -20}, new byte[]{-2, TarConstants.LF_FIFO, 57, TarConstants.LF_CONTIG, -119, -102, -55, -116}));
        this.f9027WWoWWo = file4;
        WWWWoWWWWo.m5285WWWWWWWW(file4);
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static String m5118WWWWoWWWWo(int i10) {
        switch (i10) {
            case 2:
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{111, -9, -44, 66}, new byte[]{40, -104, -69, 38, -53, -33, 85, 34});
            case 3:
                byte[] bArr = {47, -75, -24, 71, TarConstants.LF_CHR, 14, 45, -85};
                byte[] bArr2 = {96, -61, -115, TarConstants.LF_DIR, 91, 107, TarConstants.LF_GNUTYPE_LONGNAME, -33};
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
            case 4:
                byte[] bArr3 = {-27, 7, 31, -29, 66, TarConstants.LF_BLK, -1, 15};
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{-95, 98, 126, -121}, bArr3);
            case 5:
                byte[] bArr4 = {-27, TarConstants.LF_GNUTYPE_LONGLINK, -78, -60, 42, 93, 115, -126};
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{-86, 61, -41, -74, 10, 43, 28, -18, -111, 42, -43, -95}, bArr4);
            case 6:
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{-82, 100, -77, -109, -14, 74, 35, -17, -110, 111, -92, -61, -15, 72, 35, -27, -114, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -91}, new byte[]{-5, 10, -64, -29, -105, 41, 74, -119});
            case 7:
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{-109, 24, -41, -95}, new byte[]{-48, 119, -69, -59, -99, -53, 96, 107});
            default:
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{-23, -22, 79, -24, -39, -17, -117}, new byte[]{-68, -124, 36, -122, -74, -104, -27, -115});
        }
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static String m5119WWWoWWWo(int i10) {
        if (i10 != 2) {
            if (i10 != 3) {
                if (i10 != 4) {
                    if (i10 != 5) {
                        StringFog.f8859WWWWWWWW.getClass();
                        return WWWWWWWW.m17835WWWWWWWW(new byte[]{16, -32, TarConstants.LF_FIFO, -106, -109, -122, 74}, new byte[]{69, -114, 93, -8, -4, -15, 36, -1});
                    }
                    StringFog.f8859WWWWWWWW.getClass();
                    return WWWWWWWW.m17835WWWWWWWW(new byte[]{-15, 110, 86, 87}, new byte[]{-73, 27, 58, 59, -101, 114, -39, -67});
                }
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{-74, 115, -107, -120, -57, -47, -46, 35, -97, 117, -113, -49}, new byte[]{-8, 28, -31, -88, -92, -71, -77, 81});
            }
            StringFog.f8859WWWWWWWW.getClass();
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, 96, -13, -47, 80, 7, 96, 71, 26, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -25}, new byte[]{115, 9, Byte.MIN_VALUE, -78, 56, 102, 18, 32});
        }
        StringFog.f8859WWWWWWWW.getClass();
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{-21, 65, TarConstants.LF_BLK, -1, -60, -107, -68, 60}, new byte[]{-88, 41, 85, -115, -93, -4, -46, 91});
    }

    /* JADX WARN: Removed duplicated region for block: B:40:0x03ce A[Catch: all -> 0x0040, TryCatch #0 {all -> 0x0040, blocks: (B:4:0x001b, B:6:0x0034, B:11:0x0043, B:13:0x005a, B:16:0x0063, B:19:0x00a2, B:22:0x00b2, B:24:0x00ed, B:27:0x00f9, B:29:0x011a, B:30:0x012e, B:32:0x014b, B:34:0x0151, B:38:0x015c, B:40:0x03ce, B:42:0x043b, B:44:0x0474, B:46:0x04e1, B:45:0x04ab, B:41:0x0405), top: B:51:0x001b }] */
    /* JADX WARN: Removed duplicated region for block: B:41:0x0405 A[Catch: all -> 0x0040, TryCatch #0 {all -> 0x0040, blocks: (B:4:0x001b, B:6:0x0034, B:11:0x0043, B:13:0x005a, B:16:0x0063, B:19:0x00a2, B:22:0x00b2, B:24:0x00ed, B:27:0x00f9, B:29:0x011a, B:30:0x012e, B:32:0x014b, B:34:0x0151, B:38:0x015c, B:40:0x03ce, B:42:0x043b, B:44:0x0474, B:46:0x04e1, B:45:0x04ab, B:41:0x0405), top: B:51:0x001b }] */
    /* JADX WARN: Removed duplicated region for block: B:44:0x0474 A[Catch: all -> 0x0040, TryCatch #0 {all -> 0x0040, blocks: (B:4:0x001b, B:6:0x0034, B:11:0x0043, B:13:0x005a, B:16:0x0063, B:19:0x00a2, B:22:0x00b2, B:24:0x00ed, B:27:0x00f9, B:29:0x011a, B:30:0x012e, B:32:0x014b, B:34:0x0151, B:38:0x015c, B:40:0x03ce, B:42:0x043b, B:44:0x0474, B:46:0x04e1, B:45:0x04ab, B:41:0x0405), top: B:51:0x001b }] */
    /* JADX WARN: Removed duplicated region for block: B:45:0x04ab A[Catch: all -> 0x0040, TryCatch #0 {all -> 0x0040, blocks: (B:4:0x001b, B:6:0x0034, B:11:0x0043, B:13:0x005a, B:16:0x0063, B:19:0x00a2, B:22:0x00b2, B:24:0x00ed, B:27:0x00f9, B:29:0x011a, B:30:0x012e, B:32:0x014b, B:34:0x0151, B:38:0x015c, B:40:0x03ce, B:42:0x043b, B:44:0x0474, B:46:0x04e1, B:45:0x04ab, B:41:0x0405), top: B:51:0x001b }] */
    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final synchronized void m5120WWWWWWWW(Intent intent) {
        int i10;
        int i11;
        boolean isCharging;
        try {
            WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
            wwwwwwww.getClass();
            int intExtra = intent.getIntExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{64, -39, -119, -75, Byte.MIN_VALUE}, new byte[]{44, -68, -1, -48, -20, -100, 18, 8}), 0);
            if (intExtra == 0 && (intExtra = this.f9019WWWWoWWWWo.getIntProperty(4)) <= 0) {
                intExtra = 100;
            }
            wwwwwwww.getClass();
            int intExtra2 = intent.getIntExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{-104, 34, 67, -111, 101, 42}, new byte[]{-21, 86, 34, -27, 16, 89, -57, 116}), 1);
            if (intExtra2 == 1 && (intExtra2 = this.f9019WWWWoWWWWo.getIntProperty(6)) <= 0) {
                intExtra2 = 5;
            }
            wwwwwwww.getClass();
            int intExtra3 = intent.getIntExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{-1, 109, 2, -33, -113, -109, -113, -106, -2, 122, 10}, new byte[]{-117, 8, 111, -81, -22, -31, -18, -30}), 359);
            wwwwwwww.getClass();
            int intExtra4 = intent.getIntExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{16, 117, 119, -45, 104, 124, -9}, new byte[]{102, 26, 27, -89, 9, 27, -110, -34}), 4000);
            int intProperty = this.f9019WWWWoWWWWo.getIntProperty(2);
            if (intProperty == Integer.MIN_VALUE) {
                intProperty = -100000;
            }
            int i12 = intExtra2;
            int intProperty2 = this.f9019WWWWoWWWWo.getIntProperty(3);
            if (intProperty2 == Integer.MIN_VALUE) {
                intProperty2 = BZip2Constants.BASEBLOCKSIZE;
            }
            byte[] bArr = {TarConstants.LF_PAX_EXTENDED_HEADER_UC, 59, 23, -28, -52, TarConstants.LF_SYMLINK};
            byte[] bArr2 = {TarConstants.LF_NORMAL, 94, 118, -120, -72, 90, 25, -43};
            wwwwwwww.getClass();
            int intExtra5 = intent.getIntExtra(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), 2);
            byte[] bArr3 = {64, 109, -3, -23, 17, TarConstants.LF_CONTIG, 1, 5};
            wwwwwwww.getClass();
            int intExtra6 = intent.getIntExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{35, 5, -100, -101, 118, 82, 94, 102, 47, 24, -109, -99, 116, 69}, bArr3), 0);
            if (intExtra6 == 0 && (intExtra6 = this.f9019WWWWoWWWWo.getIntProperty(1)) <= 0) {
                intExtra6 = 4000000;
            }
            byte[] bArr4 = {-27, -66, -86, -46, -73, -59, -114, TarConstants.LF_CONTIG, -10, -94};
            int i13 = intExtra6;
            byte[] bArr5 = {-111, -37, -55, -70, -39, -86, -30, TarConstants.LF_PAX_EXTENDED_HEADER_UC};
            wwwwwwww.getClass();
            String stringExtra = intent.getStringExtra(WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5));
            if (TextUtils.isEmpty(stringExtra)) {
                byte[] bArr6 = {TarConstants.LF_NORMAL, -47, 107, -45, -81, 123};
                byte[] bArr7 = {124, -72, 70, -70, -64, 21, TarConstants.LF_NORMAL, 84};
                wwwwwwww.getClass();
                stringExtra = WWWWWWWW.m17835WWWWWWWW(bArr6, bArr7);
            }
            String str = stringExtra;
            wwwwwwww.getClass();
            int intExtra7 = intent.getIntExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{-9, -100, -28, TarConstants.LF_GNUTYPE_LONGNAME, -3, TarConstants.LF_MULTIVOLUME, 71}, new byte[]{-121, -16, -111, 43, -102, 40, 35, 62}), 0);
            if (intExtra7 == 0 && Build.VERSION.SDK_INT >= 23) {
                isCharging = this.f9019WWWWoWWWWo.isCharging();
                if (isCharging) {
                    i10 = 1;
                    File file = this.f9025WWWoWWWo;
                    i11 = i10;
                    int i14 = intProperty2;
                    byte[] bArr8 = {79, TarConstants.LF_SYMLINK, 106, 90, TarConstants.LF_GNUTYPE_SPARSE, 74, 38, 28};
                    wwwwwwww.getClass();
                    String absolutePath = new File(file, WWWWWWWW.m17835WWWWWWWW(new byte[]{96, 66, 24, 63, 32, 47, 72, 104}, bArr8)).getAbsolutePath();
                    wwwwwwww.getClass();
                    WWWW.m5334WWWWWWWW(absolutePath, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_BLK, 102}, new byte[]{5, 108, 124, Byte.MIN_VALUE, 6, -85, -93, 99}));
                    File file2 = this.f9025WWWoWWWo;
                    byte[] bArr9 = {36, -58, TarConstants.LF_GNUTYPE_LONGLINK, 96, -103};
                    byte[] bArr10 = {ConstantPoolEntry.CP_InterfaceMethodref, -78, TarConstants.LF_SYMLINK, 16, -4, -7, 0, 97};
                    wwwwwwww.getClass();
                    String absolutePath2 = new File(file2, WWWWWWWW.m17835WWWWWWWW(bArr9, bArr10)).getAbsolutePath();
                    byte[] bArr11 = {-7, -22, -51, 0, TarConstants.LF_GNUTYPE_LONGLINK, 3, TarConstants.LF_MULTIVOLUME, 6};
                    byte[] bArr12 = {-69, -117, -71, 116, 46, 113, TarConstants.LF_BLK, ConstantPoolEntry.CP_NameAndType};
                    wwwwwwww.getClass();
                    WWWW.m5334WWWWWWWW(absolutePath2, WWWWWWWW.m17835WWWWWWWW(bArr11, bArr12));
                    File file3 = this.f9025WWWoWWWo;
                    byte[] bArr13 = {18, -85, -120, TarConstants.LF_GNUTYPE_LONGNAME, -7, 57, -122, 97};
                    wwwwwwww.getClass();
                    String absolutePath3 = new File(file3, WWWWWWWW.m17835WWWWWWWW(new byte[]{61, -56, -23, 60, -104, 90, -17, 21, 107}, bArr13)).getAbsolutePath();
                    WWWW.m5334WWWWWWWW(absolutePath3, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + intExtra + "\n");
                    File file4 = this.f9025WWWoWWWo;
                    wwwwwwww.getClass();
                    WWWW.m5334WWWWWWWW(new File(file4, WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, 23, -53, 96, 38, -14, 38}, new byte[]{-102, 100, -65, 1, 82, -121, 85, -39})).getAbsolutePath(), m5119WWWoWWWo(i12).concat("\n"));
                    File file5 = this.f9025WWWoWWWo;
                    wwwwwwww.getClass();
                    String absolutePath4 = new File(file5, WWWWWWWW.m17835WWWWWWWW(new byte[]{104, TarConstants.LF_NORMAL, 69, 109, -102}, new byte[]{71, 68, 32, 0, -22, 72, 17, 106})).getAbsolutePath();
                    WWWW.m5334WWWWWWWW(absolutePath4, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + intExtra3 + "\n");
                    File file6 = this.f9025WWWoWWWo;
                    wwwwwwww.getClass();
                    String absolutePath5 = new File(file6, WWWWWWWW.m17835WWWWWWWW(new byte[]{111, -18, -112, TarConstants.LF_CHR, 101, -68, -46, -124, 31, -10, -112, 40}, new byte[]{64, -104, -1, 95, 17, -35, -75, -31})).getAbsolutePath();
                    WWWW.m5334WWWWWWWW(absolutePath5, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + intExtra4 + "\n");
                    File file7 = this.f9025WWWoWWWo;
                    wwwwwwww.getClass();
                    String absolutePath6 = new File(file7, WWWWWWWW.m17835WWWWWWWW(new byte[]{-33, -121, -123, 24, -15, -14, -109, -109, -81, -118, -97, 29}, new byte[]{-16, -28, -16, 106, -125, -105, -3, -25})).getAbsolutePath();
                    WWWW.m5334WWWWWWWW(absolutePath6, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + intProperty + "\n");
                    File file8 = this.f9025WWWoWWWo;
                    byte[] bArr14 = {5, 14, 91, 114, -86, -62, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -111};
                    wwwwwwww.getClass();
                    String absolutePath7 = new File(file8, WWWWWWWW.m17835WWWWWWWW(new byte[]{42, 109, 46, 0, -40, -89, 9, -27, 90, 111, 45, 21}, bArr14)).getAbsolutePath();
                    WWWW.m5334WWWWWWWW(absolutePath7, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + i14 + "\n");
                    File file9 = this.f9025WWWoWWWo;
                    wwwwwwww.getClass();
                    WWWW.m5334WWWWWWWW(new File(file9, WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, TarConstants.LF_BLK, 8, -116, -12, 22, 40}, new byte[]{-79, 92, 109, -19, -104, 98, 64, -67})).getAbsolutePath(), m5118WWWWoWWWWo(intExtra5).concat("\n"));
                    File file10 = this.f9025WWWoWWWo;
                    wwwwwwww.getClass();
                    String absolutePath8 = new File(file10, WWWWWWWW.m17835WWWWWWWW(new byte[]{106, -17, 82, 105, -114, 47, -16, 125, 38, -29, 79, 102, -120, 45, -25}, new byte[]{69, -116, 58, 8, -4, 72, -107, 34})).getAbsolutePath();
                    WWWW.m5334WWWWWWWW(absolutePath8, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + i13 + "\n");
                    File file11 = this.f9025WWWoWWWo;
                    byte[] bArr15 = {-19, -30, -66, -47, 38, 39, TarConstants.LF_CHR, 34};
                    wwwwwwww.getClass();
                    String absolutePath9 = new File(file11, WWWWWWWW.m17835WWWWWWWW(new byte[]{-62, -106, -37, -78, 78, 73, 92, 78, -126, -123, -57}, bArr15)).getAbsolutePath();
                    WWWW.m5334WWWWWWWW(absolutePath9, str + "\n");
                    File file12 = this.f9021WWWWWWWW;
                    byte[] bArr16 = {-76, 102, -126, -34, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -43, 87, 108};
                    wwwwwwww.getClass();
                    String absolutePath10 = new File(file12, WWWWWWWW.m17835WWWWWWWW(new byte[]{-101, 18, -5, -82, 29}, bArr16)).getAbsolutePath();
                    wwwwwwww.getClass();
                    WWWW.m5334WWWWWWWW(absolutePath10, WWWWWWWW.m17835WWWWWWWW(new byte[]{-79, 114, -53, -36, -94, -25}, new byte[]{-4, 19, -94, -78, -47, -19, 73, -44}));
                    if (i11 != 1) {
                        File file13 = this.f9021WWWWWWWW;
                        byte[] bArr17 = {91, -40, -94, TarConstants.LF_MULTIVOLUME, 107, 98, 58};
                        byte[] bArr18 = {116, -73, -52, 33, 2, ConstantPoolEntry.CP_NameAndType, 95, 27};
                        wwwwwwww.getClass();
                        String absolutePath11 = new File(file13, WWWWWWWW.m17835WWWWWWWW(bArr17, bArr18)).getAbsolutePath();
                        wwwwwwww.getClass();
                        WWWW.m5334WWWWWWWW(absolutePath11, WWWWWWWW.m17835WWWWWWWW(new byte[]{123, -22}, new byte[]{74, -32, -92, 0, -97, 1, -27, -78}));
                    } else {
                        File file14 = this.f9021WWWWWWWW;
                        wwwwwwww.getClass();
                        String absolutePath12 = new File(file14, WWWWWWWW.m17835WWWWWWWW(new byte[]{79, -74, -33, -17, -4, 111, -52}, new byte[]{96, -39, -79, -125, -107, 1, -87, 47})).getAbsolutePath();
                        byte[] bArr19 = {121, 27, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 115, -5, -55, 57, -106};
                        wwwwwwww.getClass();
                        WWWW.m5334WWWWWWWW(absolutePath12, WWWWWWWW.m17835WWWWWWWW(new byte[]{73, 17}, bArr19));
                    }
                    File file15 = this.f9022WWWWWWWW;
                    byte[] bArr20 = {21, -5, -4, -112, 124, -8, Byte.MAX_VALUE, TarConstants.LF_DIR};
                    wwwwwwww.getClass();
                    String absolutePath13 = new File(file15, WWWWWWWW.m17835WWWWWWWW(new byte[]{58, -113, -123, -32, 25}, bArr20)).getAbsolutePath();
                    wwwwwwww.getClass();
                    WWWW.m5334WWWWWWWW(absolutePath13, WWWWWWWW.m17835WWWWWWWW(new byte[]{-99, -89, -50, -123}, new byte[]{-56, -12, -116, -113, -8, -5, -104, -73}));
                    if (i11 != 2) {
                        File file16 = this.f9022WWWWWWWW;
                        wwwwwwww.getClass();
                        String absolutePath14 = new File(file16, WWWWWWWW.m17835WWWWWWWW(new byte[]{-117, -100, -108, 121, -6, -101, 116}, new byte[]{-92, -13, -6, 21, -109, -11, 17, 40})).getAbsolutePath();
                        byte[] bArr21 = {-96, -33, -51, ConstantPoolEntry.CP_InterfaceMethodref, -45, -38, -116, -25};
                        wwwwwwww.getClass();
                        WWWW.m5334WWWWWWWW(absolutePath14, WWWWWWWW.m17835WWWWWWWW(new byte[]{-111, -43}, bArr21));
                    } else {
                        File file17 = this.f9022WWWWWWWW;
                        byte[] bArr22 = {5, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 66, 63, -45, -66, 46};
                        byte[] bArr23 = {42, 23, 44, TarConstants.LF_GNUTYPE_SPARSE, -70, -48, TarConstants.LF_GNUTYPE_LONGLINK, 93};
                        wwwwwwww.getClass();
                        String absolutePath15 = new File(file17, WWWWWWWW.m17835WWWWWWWW(bArr22, bArr23)).getAbsolutePath();
                        byte[] bArr24 = {110, -114, 72, TarConstants.LF_CHR, -44, 38, -23, 108};
                        wwwwwwww.getClass();
                        WWWW.m5334WWWWWWWW(absolutePath15, WWWWWWWW.m17835WWWWWWWW(new byte[]{94, -124}, bArr24));
                    }
                    byte[] bArr25 = {TarConstants.LF_BLK, TarConstants.LF_CHR, -90, TarConstants.LF_GNUTYPE_LONGLINK, -117, -6, -122, -5};
                    wwwwwwww.getClass();
                    WWWW.m5339WWWoWWWo(this.f9027WWoWWo, WWWWWWWW.m17835WWWWWWWW(new byte[]{5}, bArr25), false);
                }
            }
            i10 = intExtra7;
            File file18 = this.f9025WWWoWWWo;
            i11 = i10;
            int i142 = intProperty2;
            byte[] bArr82 = {79, TarConstants.LF_SYMLINK, 106, 90, TarConstants.LF_GNUTYPE_SPARSE, 74, 38, 28};
            wwwwwwww.getClass();
            String absolutePath16 = new File(file18, WWWWWWWW.m17835WWWWWWWW(new byte[]{96, 66, 24, 63, 32, 47, 72, 104}, bArr82)).getAbsolutePath();
            wwwwwwww.getClass();
            WWWW.m5334WWWWWWWW(absolutePath16, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_BLK, 102}, new byte[]{5, 108, 124, Byte.MIN_VALUE, 6, -85, -93, 99}));
            File file22 = this.f9025WWWoWWWo;
            byte[] bArr92 = {36, -58, TarConstants.LF_GNUTYPE_LONGLINK, 96, -103};
            byte[] bArr102 = {ConstantPoolEntry.CP_InterfaceMethodref, -78, TarConstants.LF_SYMLINK, 16, -4, -7, 0, 97};
            wwwwwwww.getClass();
            String absolutePath22 = new File(file22, WWWWWWWW.m17835WWWWWWWW(bArr92, bArr102)).getAbsolutePath();
            byte[] bArr112 = {-7, -22, -51, 0, TarConstants.LF_GNUTYPE_LONGLINK, 3, TarConstants.LF_MULTIVOLUME, 6};
            byte[] bArr122 = {-69, -117, -71, 116, 46, 113, TarConstants.LF_BLK, ConstantPoolEntry.CP_NameAndType};
            wwwwwwww.getClass();
            WWWW.m5334WWWWWWWW(absolutePath22, WWWWWWWW.m17835WWWWWWWW(bArr112, bArr122));
            File file32 = this.f9025WWWoWWWo;
            byte[] bArr132 = {18, -85, -120, TarConstants.LF_GNUTYPE_LONGNAME, -7, 57, -122, 97};
            wwwwwwww.getClass();
            String absolutePath32 = new File(file32, WWWWWWWW.m17835WWWWWWWW(new byte[]{61, -56, -23, 60, -104, 90, -17, 21, 107}, bArr132)).getAbsolutePath();
            WWWW.m5334WWWWWWWW(absolutePath32, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + intExtra + "\n");
            File file42 = this.f9025WWWoWWWo;
            wwwwwwww.getClass();
            WWWW.m5334WWWWWWWW(new File(file42, WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, 23, -53, 96, 38, -14, 38}, new byte[]{-102, 100, -65, 1, 82, -121, 85, -39})).getAbsolutePath(), m5119WWWoWWWo(i12).concat("\n"));
            File file52 = this.f9025WWWoWWWo;
            wwwwwwww.getClass();
            String absolutePath42 = new File(file52, WWWWWWWW.m17835WWWWWWWW(new byte[]{104, TarConstants.LF_NORMAL, 69, 109, -102}, new byte[]{71, 68, 32, 0, -22, 72, 17, 106})).getAbsolutePath();
            WWWW.m5334WWWWWWWW(absolutePath42, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + intExtra3 + "\n");
            File file62 = this.f9025WWWoWWWo;
            wwwwwwww.getClass();
            String absolutePath52 = new File(file62, WWWWWWWW.m17835WWWWWWWW(new byte[]{111, -18, -112, TarConstants.LF_CHR, 101, -68, -46, -124, 31, -10, -112, 40}, new byte[]{64, -104, -1, 95, 17, -35, -75, -31})).getAbsolutePath();
            WWWW.m5334WWWWWWWW(absolutePath52, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + intExtra4 + "\n");
            File file72 = this.f9025WWWoWWWo;
            wwwwwwww.getClass();
            String absolutePath62 = new File(file72, WWWWWWWW.m17835WWWWWWWW(new byte[]{-33, -121, -123, 24, -15, -14, -109, -109, -81, -118, -97, 29}, new byte[]{-16, -28, -16, 106, -125, -105, -3, -25})).getAbsolutePath();
            WWWW.m5334WWWWWWWW(absolutePath62, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + intProperty + "\n");
            File file82 = this.f9025WWWoWWWo;
            byte[] bArr142 = {5, 14, 91, 114, -86, -62, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -111};
            wwwwwwww.getClass();
            String absolutePath72 = new File(file82, WWWWWWWW.m17835WWWWWWWW(new byte[]{42, 109, 46, 0, -40, -89, 9, -27, 90, 111, 45, 21}, bArr142)).getAbsolutePath();
            WWWW.m5334WWWWWWWW(absolutePath72, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + i142 + "\n");
            File file92 = this.f9025WWWoWWWo;
            wwwwwwww.getClass();
            WWWW.m5334WWWWWWWW(new File(file92, WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, TarConstants.LF_BLK, 8, -116, -12, 22, 40}, new byte[]{-79, 92, 109, -19, -104, 98, 64, -67})).getAbsolutePath(), m5118WWWWoWWWWo(intExtra5).concat("\n"));
            File file102 = this.f9025WWWoWWWo;
            wwwwwwww.getClass();
            String absolutePath82 = new File(file102, WWWWWWWW.m17835WWWWWWWW(new byte[]{106, -17, 82, 105, -114, 47, -16, 125, 38, -29, 79, 102, -120, 45, -25}, new byte[]{69, -116, 58, 8, -4, 72, -107, 34})).getAbsolutePath();
            WWWW.m5334WWWWWWWW(absolutePath82, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + i13 + "\n");
            File file112 = this.f9025WWWoWWWo;
            byte[] bArr152 = {-19, -30, -66, -47, 38, 39, TarConstants.LF_CHR, 34};
            wwwwwwww.getClass();
            String absolutePath92 = new File(file112, WWWWWWWW.m17835WWWWWWWW(new byte[]{-62, -106, -37, -78, 78, 73, 92, 78, -126, -123, -57}, bArr152)).getAbsolutePath();
            WWWW.m5334WWWWWWWW(absolutePath92, str + "\n");
            File file122 = this.f9021WWWWWWWW;
            byte[] bArr162 = {-76, 102, -126, -34, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -43, 87, 108};
            wwwwwwww.getClass();
            String absolutePath102 = new File(file122, WWWWWWWW.m17835WWWWWWWW(new byte[]{-101, 18, -5, -82, 29}, bArr162)).getAbsolutePath();
            wwwwwwww.getClass();
            WWWW.m5334WWWWWWWW(absolutePath102, WWWWWWWW.m17835WWWWWWWW(new byte[]{-79, 114, -53, -36, -94, -25}, new byte[]{-4, 19, -94, -78, -47, -19, 73, -44}));
            if (i11 != 1) {
            }
            File file152 = this.f9022WWWWWWWW;
            byte[] bArr202 = {21, -5, -4, -112, 124, -8, Byte.MAX_VALUE, TarConstants.LF_DIR};
            wwwwwwww.getClass();
            String absolutePath132 = new File(file152, WWWWWWWW.m17835WWWWWWWW(new byte[]{58, -113, -123, -32, 25}, bArr202)).getAbsolutePath();
            wwwwwwww.getClass();
            WWWW.m5334WWWWWWWW(absolutePath132, WWWWWWWW.m17835WWWWWWWW(new byte[]{-99, -89, -50, -123}, new byte[]{-56, -12, -116, -113, -8, -5, -104, -73}));
            if (i11 != 2) {
            }
            byte[] bArr252 = {TarConstants.LF_BLK, TarConstants.LF_CHR, -90, TarConstants.LF_GNUTYPE_LONGLINK, -117, -6, -122, -5};
            wwwwwwww.getClass();
            WWWW.m5339WWWoWWWo(this.f9027WWoWWo, WWWWWWWW.m17835WWWWWWWW(new byte[]{5}, bArr252), false);
        } catch (Throwable th2) {
            throw th2;
        }
    }

    @Override // android.content.BroadcastReceiver
    public final void onReceive(Context context, Intent intent) {
        Handler handler;
        if (intent != null) {
            byte[] bArr = {-43, 78, 42, -95, TarConstants.LF_CONTIG, -70, 7, -95, -35, 78, 58, -74, TarConstants.LF_FIFO, -89, TarConstants.LF_MULTIVOLUME, -18, -41, 84, 39, -68, TarConstants.LF_FIFO, -3, 33, -50, -32, 116, ConstantPoolEntry.CP_InterfaceMethodref, -127, 1, -116, 32, -57, -11, 110, 9, -106, 28};
            byte[] bArr2 = {-76, 32, 78, -45, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -45, 99, -113};
            StringFog.f8859WWWWWWWW.getClass();
            if (!WWWWWWWW.m17835WWWWWWWW(bArr, bArr2).equals(intent.getAction()) || (handler = this.f9026WWWoWWWo) == null) {
                return;
            }
            handler.post(new RunnableC0056WWWWWWWW(16, this, intent));
        }
    }
}
