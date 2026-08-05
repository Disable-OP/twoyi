package com.android.vmcore;

import android.content.ServiceConnection;
import android.content.SharedPreferences;
import android.hardware.Sensor;
import android.os.Build;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Process;
import android.text.TextUtils;
import android.util.SparseArray;
import androidx.appcompat.widget.r0;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmapp.VMApp;
import com.android.vmcore.app.VMAppManager;
import com.android.vmcore.bridge.IVMEventCallback;
import com.android.vmcore.bridge.VMEventManager;
import com.android.vmcore.event.VMConfigEvent;
import com.android.vmcore.event.VMStatusEvent;
import com.android.vmcore.hal.AudioService;
import com.android.vmcore.hal.DisplayService;
import com.android.vmcore.hal.HALManager;
import com.android.vmcore.hal.InputService;
import com.android.vmcore.hal.LocationService;
import com.android.vmcore.hal.NetlinkManager;
import com.android.vmcore.hal.SensorService;
import com.android.vmcore.service.BinderService;
import com.android.vmcore.setup.ChmodFsTask;
import com.android.vmcore.setup.CleanCacheTask;
import com.android.vmcore.setup.CleanFsTask;
import com.android.vmcore.setup.FixCPUArchTask;
import com.android.vmcore.setup.FixFsTask;
import com.android.vmcore.setup.InstallFsTask;
import com.android.vmcore.setup.LoadVMPropTask;
import com.android.vmcore.setup.PrepareFsTask;
import com.android.vmcore.startup.ApplyOverlaysTask;
import com.android.vmcore.startup.Bug1FixTask;
import com.android.vmcore.startup.Bug2FixTask;
import com.android.vmcore.startup.Bug3FixTask;
import com.android.vmcore.startup.Bug4FixTask;
import com.android.vmcore.startup.Bug5FixTask;
import com.android.vmcore.startup.Bug6FixTask;
import com.android.vmcore.startup.Bug7FixTask;
import com.android.vmcore.startup.Bug8FixTask;
import com.android.vmcore.startup.BuildExecPathTask;
import com.android.vmcore.startup.BuildTmpfsTask;
import com.android.vmcore.startup.BuildVMPropTask;
import com.android.vmcore.startup.CleanLogTask;
import com.android.vmcore.startup.GooglePlayTask;
import com.android.vmcore.startup.MagiskTask;
import com.android.vmcore.startup.SuperuserTask;
import com.android.vmcore.startup.XposedTask;
import com.android.vmcore.utils.CPUUtils;
import com.android.vmcore.utils.FileDeleteUtils;
import com.blankj.utilcode.util.WWWW;
import com.google.android.gms.internal.ads.pr0;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import eh.C2467WWWWWWWW;
import eh.C2468WWWWWWWW;
import java.io.File;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Set;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.json.JSONObject;
import p041WWWoWWWo.C0434WWWWWWWW;
import p053WWoWWo.AbstractC0470WWWWWWWW;
import p057WWoWWo.WWWWoWWWWo;
import vf.AbstractC4470WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class VMInstance implements IVMEventCallback {

    /* renamed from: WWWoૄWWWoѽૄ  reason: contains not printable characters */
    public static final String f8925WWWoWWWo;

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final SharedPreferences f8926WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final VMApp f8927WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public RomConfig f8928WWWWWWWW;

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public VMResConfig f8929WWWWWWWW;

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public int f8930WWWWWWWW;

    /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
    public boolean f8931WWWWWWWW;

    /* renamed from: WWWW֭WWWWྥ֭  reason: contains not printable characters */
    public final ArrayList f8932WWWWWWWW = new ArrayList();

    /* renamed from: WWWWآWWWWȫآ  reason: contains not printable characters */
    public HALManager f8933WWWWWWWW;

    /* renamed from: WWWWܬWWWWೖܬ  reason: contains not printable characters */
    public NetlinkManager f8934WWWWWWWW;

    /* renamed from: WWWWެWWWWܕެ  reason: contains not printable characters */
    public final VMEventManager f8935WWWWWWWW;

    /* renamed from: WWWWॾWWWWȏॾ  reason: contains not printable characters */
    public HandlerThread f8936WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final VMConfig f8937WWWoWWWo;

    /* renamed from: WWWoԻWWWoͷԻ  reason: contains not printable characters */
    public int f8938WWWoWWWo;

    /* renamed from: WWWoࠟWWWoॣࠟ  reason: contains not printable characters */
    public final C2467WWWWWWWW f8939WWWoWWWo;

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public int f8940WWoWWo;

    /* renamed from: WWoڢWWo࢞ڢ  reason: not valid java name and contains not printable characters */
    public InputService f8941WWoWWo;

    /* renamed from: WWoॹWWoࠔॹ  reason: contains not printable characters */
    public VMAppManager f8942WWoWWo;

    /* renamed from: WWoহWWoȗহ  reason: contains not printable characters */
    public Handler f8943WWoWWo;

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public AudioService f8944WWWW;

    /* renamed from: WoڄWoᄴڄ  reason: contains not printable characters */
    public DisplayService f8945WoWo;

    /* loaded from: classes.dex */
    public static class DeferredVMApp {

        /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
        public String f8946WWWWWWWW;
    }

    static {
        StringFog.f8859WWWWWWWW.getClass();
        f8925WWWoWWWo = WWWWWWWW.m17835WWWWWWWW(new byte[]{25, -18, -6, -73, 108, 74, -127, 117, 44, -58}, new byte[]{79, -93, -77, -39, 31, 62, -32, 27});
    }

    public VMInstance(VMApp vMApp, int i10) {
        this.f8927WWWWWWWW = vMApp;
        StringBuilder sb2 = new StringBuilder();
        StringFog.f8859WWWWWWWW.getClass();
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-45, -25, 17, -6, 108, -77, -86, 113, -62, -43}, new byte[]{-91, -118, 78, -103, 3, -35, -52, 24}));
        sb2.append(i10);
        SharedPreferences sharedPreferences = vMApp.getSharedPreferences(sb2.toString(), 0);
        this.f8926WWWWoWWWWo = sharedPreferences;
        VMConfig m5050WWWWoWWWWo = VMConfig.m5050WWWWoWWWWo(sharedPreferences);
        m5050WWWWoWWWWo.f8866WWWWWWWW = i10;
        m5050WWWWoWWWWo.f8884WWWWWWWW = i10 + 10000;
        if (TextUtils.isEmpty(m5050WWWWoWWWWo.f8861WWWWoWWWWo)) {
            m5050WWWWoWWWWo.f8861WWWWoWWWWo = m5050WWWWoWWWWo.f8895WWWoWWWo.f8845WWWWoWWWWo + WWWWWWWW.m17835WWWWWWWW(new byte[]{-126}, new byte[]{-35, -83, -3, TarConstants.LF_GNUTYPE_LONGNAME, 24, -22, -32, 20}) + (1 + i10);
        }
        String replace = vMApp.getApplicationInfo().dataDir.replace(WWWWWWWW.m17835WWWWWWWW(new byte[]{126, TarConstants.LF_CONTIG, -72, -80, 36, -5, 16, 108, TarConstants.LF_BLK, 33, -10, -12, 106}, new byte[]{81, TarConstants.LF_GNUTYPE_SPARSE, -39, -60, 69, -44, 101, 31}), WWWWWWWW.m17835WWWWWWWW(new byte[]{66, -98, TarConstants.LF_GNUTYPE_LONGNAME, -2, TarConstants.LF_GNUTYPE_SPARSE, -104, -86, 24, 25, -101, 2}, new byte[]{109, -6, 45, -118, TarConstants.LF_SYMLINK, -73, -50, 121}));
        m5050WWWWoWWWWo.f8867WWWWWWWW = new File(replace, WWWWWWWW.m17835WWWWWWWW(new byte[]{2, -88, 60, -42, -63}, new byte[]{116, -59, 19, -96, -84, -26, 5, -122}) + i10).getAbsolutePath();
        m5050WWWWoWWWWo.f8868WWWWWWWW = new File(m5050WWWWoWWWWo.f8867WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-32, -43, -118}, new byte[]{-49, -77, -7, 10, -102, 67, 32, -7})).getAbsolutePath();
        m5050WWWWoWWWWo.f8903WWoWWo = vMApp.getApplicationInfo().dataDir + WWWWWWWW.m17835WWWWWWWW(new byte[]{25, -55, 67, -84, -113, -127}, new byte[]{TarConstants.LF_FIFO, -91, 42, -50, -71, -75, 82, -78});
        m5050WWWWoWWWWo.f8919WWWW = sharedPreferences.getBoolean(WWWWWWWW.m17835WWWWWWWW(new byte[]{-91, -123, TarConstants.LF_CONTIG, 0, 7, 85, -81, 62}, new byte[]{-51, -28, 68, 95, 110, 59, -58, 74}), false);
        m5050WWWWoWWWWo.f8923WoWo = WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, -102, Byte.MIN_VALUE, -87}, new byte[]{-90, -11, -18, -52, TarConstants.LF_BLK, TarConstants.LF_NORMAL, 112, -56});
        m5050WWWWoWWWWo.f8894WWWWWWWW = sharedPreferences.getString(WWWWWWWW.m17835WWWWWWWW(new byte[]{115, 44, 29, -90, -89, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -65, 101, 126, 39, ConstantPoolEntry.CP_NameAndType, -86, -74, 114, -110, 106, 121, 61}, new byte[]{23, 73, 107, -49, -60, 2, -32, 3}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        this.f8937WWWoWWWo = m5050WWWWoWWWWo;
        C2467WWWWWWWW c2467wwwwwwww = C2467WWWWWWWW.f26772WWWoWWWo;
        this.f8939WWWoWWWo = new C2467WWWWWWWW(new C2468WWWWWWWW());
        this.f8935WWWWWWWW = new VMEventManager(this);
    }

    private static native int startOS(int i10, int i11, String str);

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final void m5052WWWWWWWWWW(String str, String str2) {
        VMConfig vMConfig = this.f8937WWWoWWWo;
        if (str != null) {
            vMConfig.f8871WWWWWWWW = str;
        }
        vMConfig.f8872WWWWWWWW = str2;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-38, 107, 115, -80}, new byte[]{-77, 6, 22, -39, 14, 8, -85, -99}), vMConfig.f8871WWWWWWWW).putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, -110, -29, -15, -19, -63, -114}, new byte[]{-36, -1, -122, -104, -78, -78, -8, -29}), vMConfig.f8872WWWWWWWW).apply();
    }

    /* JADX WARN: Type inference failed for: r2v1, types: [java.lang.Object, com.android.vmcore.VMInstance$DeferredVMApp] */
    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final void m5053WWWWoWWWWo(String str) {
        synchronized (this.f8932WWWWWWWW) {
            try {
                ArrayList arrayList = this.f8932WWWWWWWW;
                int size = arrayList.size();
                int i10 = 0;
                while (i10 < size) {
                    Object obj = arrayList.get(i10);
                    i10++;
                    if (((DeferredVMApp) obj).f8946WWWWWWWW.equals(str)) {
                        return;
                    }
                }
                ArrayList arrayList2 = this.f8932WWWWWWWW;
                ?? obj2 = new Object();
                obj2.f8946WWWWWWWW = str;
                arrayList2.add(obj2);
            } catch (Throwable th2) {
                throw th2;
            }
        }
    }

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final void m5054WWWWoWWWWo(boolean z10) {
        this.f8937WWWoWWWo.f8896WWWoWWWo = z10;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        byte[] bArr = {-89, 84, -70, -96, -118, -83, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -87};
        AbstractC1017WWWoWWWo.m3456WWoWWo(StringFog.f8859WWWWWWWW, new byte[]{-50, 39, -27, -57, -7, -64, 7, -39, -49, 59, -44, -59}, bArr, edit, z10);
    }

    /* renamed from: WWWWoᆣWWWWoϿᆣ  reason: contains not printable characters */
    public final void m5055WWWWoWWWWo(String str) {
        this.f8937WWWoWWWo.f8860WWWWWWWWWW = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-60, -117, 41, 25, -32, -108, 85, -124, -45, -115, 39, 27, -38, -72, 82, -97, -47, -115, 33, 3, -19}, new byte[]{-76, -29, 70, 119, -123, -53, 38, -19}), str).apply();
    }

    @Override // com.android.vmcore.bridge.IVMEventCallback
    /* renamed from: WWWW̏WWWWβ̏ */
    public final void mo5013WWWWWWWW(String str, String str2) {
        StringFog.f8859WWWWWWWW.getClass();
        int i10 = 0;
        if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-26, 29, 20, 39, -37, -89, TarConstants.LF_SYMLINK, 117, -22, 27, 29, 39, -52, -92, TarConstants.LF_DIR, 104, -9, 23, 87, 104, -39, -67, 63, 104, -21, 92, 56, 71, -2, -101, 25, 78, -63, 45, TarConstants.LF_FIFO, 90, -27, -101, 19, 70, -63, 43}, new byte[]{-123, 114, 121, 9, -70, -55, 86, 7}).equals(str)) {
            if (this.f8940WWoWWo >= 6) {
                this.f8940WWoWWo = 6;
                KLog.m5040WWWWoWWWWo(f8925WWWoWWWo, WWWWWWWW.m17835WWWWWWWW(new byte[]{-1, 43, -76, -44, 74, 123, 109, 16, -27, 40, -93, -63, 90, 125, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 16, -21, TarConstants.LF_DIR, -72, -63, 80, 117, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 22, -7, 70, -107, -10, 119, 91, TarConstants.LF_GNUTYPE_LONGNAME, 59, -124, 2, -126, -25, 112, 87, 87, 42, -64}, new byte[]{-92, 102, -25, -109, 21, TarConstants.LF_BLK, 35, 79}));
            } else {
                this.f8940WWoWWo = 6;
                this.f8939WWWoWWWo.m13942WWWWWWWW(new VMStatusEvent(6, 0));
            }
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{86, -76, 97, 108, -56, -9, -78, 2, 90, -78, 104, 108, -33, -12, -75, 31, 71, -66, 34, 35, -54, -19, -65, 31, 91, -11, 78, 13, -26, -51, -119, TarConstants.LF_CHR, 122, -106, 92, 14, -20, -51, -109, TarConstants.LF_BLK}, new byte[]{TarConstants.LF_DIR, -37, ConstantPoolEntry.CP_NameAndType, 66, -87, -103, -42, 112}).equals(str)) {
            if (this.f8940WWoWWo >= 7) {
                this.f8940WWoWWo = 7;
                KLog.m5040WWWWoWWWWo(f8925WWWoWWWo, WWWWWWWW.m17835WWWWWWWW(new byte[]{-68, 126, -50, -1, -7, -121, 82, -68, -91, 124, -46, -20, -7, -117, TarConstants.LF_GNUTYPE_SPARSE, -82, -73, Byte.MAX_VALUE, -40, -20, -29, -116, 65, -61, -107, 86, -1, -41, -55, -68, 60, -121, -126, 71, -8, -37, -46, -83, TarConstants.LF_PAX_EXTENDED_HEADER_LC}, new byte[]{-25, TarConstants.LF_CHR, -99, -72, -90, -56, 28, -29}));
            } else {
                this.f8940WWoWWo = 7;
                this.f8939WWWoWWWo.m13942WWWWWWWW(new VMStatusEvent(7, 0));
            }
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-78, 13, 45, 1, 47, -81, -7, -97, -66, ConstantPoolEntry.CP_InterfaceMethodref, 36, 1, 56, -84, -2, -126, -93, 7, 110, 78, 45, -75, -12, -126, -65, TarConstants.LF_GNUTYPE_LONGNAME, 22, 98, 17, -110, -40, -65, -121, 39, 18, 112, 28, -124, -36, -87, -120}, new byte[]{-47, 98, 64, 47, 78, -63, -99, -19}).equals(str)) {
            m5064WWWWWWWW(this.f8937WWWoWWWo.f8910WWoWWo);
        }
        if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-83, -87, 45, -58, 73, 32, 94, TarConstants.LF_MULTIVOLUME, -95, -81, 36, -58, 94, 35, 89, 80, -68, -93, 110, -119, TarConstants.LF_GNUTYPE_LONGLINK, 58, TarConstants.LF_GNUTYPE_SPARSE, 80, -96, -24, 2, -89, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 26, 101, 124, -127, -117, 16, -92, 109, 26, Byte.MAX_VALUE, 123}, new byte[]{-50, -58, 64, -24, 40, 78, 58, 63}).equals(str)) {
            synchronized (this.f8932WWWWWWWW) {
                try {
                    ArrayList arrayList = this.f8932WWWWWWWW;
                    int size = arrayList.size();
                    while (i10 < size) {
                        Object obj = arrayList.get(i10);
                        i10++;
                        DeferredVMApp deferredVMApp = (DeferredVMApp) obj;
                        deferredVMApp.getClass();
                        ((VMAppManager) m5058WWWWWWWW()).m5111WWWoWWWo(deferredVMApp.f8946WWWWWWWW);
                    }
                    this.f8932WWWWWWWW.clear();
                } catch (Throwable th2) {
                    throw th2;
                }
            }
        }
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final boolean m5056WWWWWWWW() {
        VMConfig vMConfig;
        boolean z10 = true;
        int i10 = 0;
        while (true) {
            int i11 = i10 + 1;
            vMConfig = this.f8937WWWoWWWo;
            if (i10 < 10) {
                NativeHelper.clearZombieProcess(vMConfig.f8866WWWWWWWW);
                ArrayList<VMProcessInfo> processList = NativeHelper.getProcessList(vMConfig.f8866WWWWWWWW);
                if (processList.isEmpty()) {
                    this.f8938WWWoWWWo = 0;
                    break;
                }
                int i12 = this.f8938WWWoWWWo;
                if (i12 > 0) {
                    Process.killProcess(i12);
                    this.f8938WWWoWWWo = 0;
                }
                Collections.sort(processList, new WWWWoWWWWo(1));
                int size = processList.size();
                int i13 = 0;
                while (i13 < size) {
                    VMProcessInfo vMProcessInfo = processList.get(i13);
                    i13++;
                    Process.killProcess(vMProcessInfo.pid);
                }
                try {
                    Thread.sleep(1000L);
                } catch (Throwable unused) {
                }
                i10 = i11;
            } else {
                z10 = false;
                break;
            }
        }
        String str = vMConfig.f8868WWWWWWWW;
        byte[] bArr = {-115, -16, TarConstants.LF_GNUTYPE_SPARSE, -47, -43, 59, 2, 23, -48, -5, 85};
        byte[] bArr2 = {-94, -108, TarConstants.LF_FIFO, -89, -6, TarConstants.LF_MULTIVOLUME, 111, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER};
        StringFog.f8859WWWWWWWW.getClass();
        FileDeleteUtils.m5261WWWWoWWWWo(new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)));
        FileDeleteUtils.m5261WWWWoWWWWo(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-61, -80, -11, 73, -88, -124, 73, 126, -118, -89}, new byte[]{-20, -44, -112, 63, -121, -16, 36, 14})));
        FileDeleteUtils.m5261WWWWoWWWWo(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{67, -35, 109, -57, 29}, new byte[]{108, -68, 14, -92, 105, 40, -19, 117})));
        FileDeleteUtils.m5263WWWWWWWW(new File(vMConfig.f8868WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-32, -6, 90, -106, 20, 10}, new byte[]{-49, -118, 40, -7, 119, 37, -72, -82})), new C1623WWWWWWWW(0), false);
        return z10;
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public final boolean m5057WWWWWWWW() {
        this.f8940WWoWWo = -5;
        this.f8939WWWoWWWo.m13942WWWWWWWW(new VMStatusEvent(-5, this.f8930WWWWWWWW));
        String str = f8925WWWoWWWo;
        byte[] bArr = {78, TarConstants.LF_GNUTYPE_SPARSE, -29, 87, 2, 42, -124, -26, 122, 64, -30, 82, 39, 2, -48, -23, 124, 91, -32, 36, 28, TarConstants.LF_SYMLINK, -48, -14, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -17, 97, 25, 44, -107, -15};
        byte[] bArr2 = {21, TarConstants.LF_CONTIG, -116, 4, 106, 95, -16, -126};
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5041WWWWWWWW(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        if (!m5056WWWWWWWW()) {
            KLog.m5040WWWWoWWWWo(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{69, -113, -119, 9, -117, 40, 108, TarConstants.LF_CONTIG, 113, -100, -120, ConstantPoolEntry.CP_NameAndType, -82, 0, 56, 56, 119, -121, -118, 122, -107, TarConstants.LF_NORMAL, 56, 35, 108, -124, -123, 63, -112, 46, 125, 32, 62, -115, -121, TarConstants.LF_CHR, -113, 56, 124}, new byte[]{30, -21, -26, 90, -29, 93, 24, TarConstants.LF_GNUTYPE_SPARSE}));
            this.f8940WWoWWo = -999;
            this.f8930WWWWWWWW = 103000;
            this.f8939WWWoWWWo.m13942WWWWWWWW(new VMStatusEvent(-999, 103000));
            return false;
        }
        KLog.m5041WWWWWWWW(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{111, 64, -79, -58, -95, 84, 124, -125, 91, TarConstants.LF_GNUTYPE_SPARSE, -80, -61, -124, 124, 40, -108, 64, TarConstants.LF_GNUTYPE_LONGLINK, -82, -75, -65, TarConstants.LF_GNUTYPE_LONGNAME, 40, -108, 66, 71}, new byte[]{TarConstants.LF_BLK, 36, -34, -107, -55, 33, 8, -25}));
        AudioService audioService = this.f8944WWWW;
        if (audioService != null) {
            audioService.stop();
        }
        InputService inputService = this.f8941WWoWWo;
        if (inputService != null) {
            inputService.m5132WWWoWWWo();
        }
        HALManager hALManager = this.f8933WWWWWWWW;
        if (hALManager != null) {
            hALManager.stopHALMgr();
        }
        DisplayService displayService = this.f8945WoWo;
        if (displayService != null) {
            displayService.m5128WWWWWWWW();
        }
        NetlinkManager netlinkManager = this.f8934WWWWWWWW;
        if (netlinkManager != null) {
            netlinkManager.stop();
        }
        VMEventManager vMEventManager = this.f8935WWWWWWWW;
        if (vMEventManager != null) {
            vMEventManager.f8990WWWoWWWo = true;
        }
        VMApp vMApp = this.f8927WWWWWWWW;
        int i10 = this.f8937WWWoWWWo.f8866WWWWWWWW;
        BinderService.f9240WWWWWWWW.delete(i10);
        BinderService.f9243WWWW.delete(i10);
        BinderService.f9241WWWWWWWW.delete(i10);
        try {
            SparseArray sparseArray = BinderService.f9242WWoWWo;
            ServiceConnection serviceConnection = (ServiceConnection) sparseArray.get(i10);
            if (serviceConnection != null) {
                sparseArray.delete(i10);
                vMApp.unbindService(serviceConnection);
            }
        } catch (Throwable unused) {
        }
        String str2 = f8925WWWoWWWo;
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5041WWWWWWWW(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{41, -83, -4, 113, -37, 61, 35, 31, 29, -66, -3, 116, -2, 21, 119, 8, 26, -68, -25, 70, -36, 63, 57, 91, 4, -92, -77, 81, -58, 43, TarConstants.LF_BLK, 30, 23, -83}, new byte[]{114, -55, -109, 34, -77, 72, 87, 123}));
        this.f8940WWoWWo = 0;
        this.f8939WWWoWWWo.m13942WWWWWWWW(new VMStatusEvent(0, 0));
        HandlerThread handlerThread = this.f8936WWWWWWWW;
        if (handlerThread != null) {
            handlerThread.quit();
        }
        this.f8943WWoWWo = null;
        return true;
    }

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public final IAppManager m5058WWWWWWWW() {
        if (this.f8942WWoWWo == null) {
            this.f8942WWoWWo = new VMAppManager(this.f8927WWWWWWWW, this);
        }
        return this.f8942WWoWWo;
    }

    /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
    public final String m5059WWWWWWWW(String str) {
        String str2 = (String) this.f8937WWWoWWWo.f8864WWWWoWWWWo.get(str);
        if (str2 == null) {
            byte[] bArr = {TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -11, 7, 42, -15, 72, 105, TarConstants.LF_GNUTYPE_SPARSE};
            StringFog.f8859WWWWWWWW.getClass();
            return WWWWWWWW.m17835WWWWWWWW(new byte[]{23, -108, 117, 94, -104, 41, 5, 63, 30, -86, 116, 95, -127, 56, 6, 33, 19}, bArr);
        }
        return str2;
    }

    /* renamed from: WWWW֭WWWWྥ֭  reason: contains not printable characters */
    public final VMConfig m5060WWWWWWWW() {
        return this.f8937WWWoWWWo;
    }

    /* renamed from: WWWWآWWWWȫآ  reason: contains not printable characters */
    public final VMResConfig m5061WWWWWWWW() {
        VMResConfig vMResConfig = new VMResConfig();
        m5093WWWW(vMResConfig);
        return vMResConfig;
    }

    /* renamed from: WWWWܬWWWWೖܬ  reason: contains not printable characters */
    public final boolean m5062WWWWWWWW() {
        AudioService audioService = this.f8944WWWW;
        if (audioService != null && audioService.isMute()) {
            return true;
        }
        return false;
    }

    /* renamed from: WWWWެWWWWܕެ  reason: contains not printable characters */
    public final void m5063WWWWWWWW(String str) {
        DeferredVMApp deferredVMApp;
        synchronized (this.f8932WWWWWWWW) {
            try {
                ArrayList arrayList = this.f8932WWWWWWWW;
                int size = arrayList.size();
                int i10 = 0;
                while (true) {
                    if (i10 < size) {
                        Object obj = arrayList.get(i10);
                        i10++;
                        deferredVMApp = (DeferredVMApp) obj;
                        if (deferredVMApp.f8946WWWWWWWW.equals(str)) {
                        }
                    } else {
                        deferredVMApp = null;
                        break;
                    }
                }
                if (deferredVMApp != null) {
                    this.f8932WWWWWWWW.remove(deferredVMApp);
                }
            } finally {
            }
        }
    }

    /* renamed from: WWWWॾWWWWȏॾ  reason: contains not printable characters */
    public final void m5064WWWWWWWW(boolean z10) {
        VMConfig vMConfig = this.f8937WWWoWWWo;
        vMConfig.f8910WWoWWo = z10;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        byte[] bArr = {-99, -83, -121, -27, TarConstants.LF_LINK, -111, 74, 14};
        AbstractC1017WWWoWWWo.m3456WWoWWo(StringFog.f8859WWWWWWWW, new byte[]{-4, -55, -27, -70, 84, -1, 43, 108, -15, -56, -29}, bArr, edit, z10);
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{29, 114, -111, -69, 46, -1, 105, TarConstants.LF_NORMAL, 15, 115, -127, -29, TarConstants.LF_CHR, -1, 124, TarConstants.LF_NORMAL, 30, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -111, -71}, new byte[]{110, 23, -29, -51, 71, -100, ConstantPoolEntry.CP_NameAndType, 30});
        m5098WoWo().post(new WWWWWWWW(this, m17835WWWWWWWW, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING + vMConfig.f8884WWWWWWWW, 0));
        if (z10) {
            m5098WoWo().post(new WWWWWWWW(this, WWWWWWWW.m17835WWWWWWWW(new byte[]{-51, 82, -77, 15, -43, -102, 74, -68, -35, 68, -82, 71, -55, -114}, new byte[]{-66, 43, -64, 33, -96, -23, 40, -110}), WWWWWWWW.m17835WWWWWWWW(new byte[]{59, -108, -26}, new byte[]{90, -16, -124, 21, -93, -67, -16, TarConstants.LF_SYMLINK}), 0));
            return;
        }
        m5098WoWo().post(new WWWWWWWW(this, WWWWWWWW.m17835WWWWWWWW(new byte[]{57, 17, 46, ConstantPoolEntry.CP_NameAndType, -92, -22, -27, 40, 41, 7, TarConstants.LF_CHR, 68, -72, -2}, new byte[]{74, 104, 93, 34, -47, -103, -121, 6}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-66, -116, -13, -101}, new byte[]{-48, -29, -99, -2, 80, -28, -108, -54}), 0));
    }

    /* renamed from: WWWWമWWWWုമ  reason: contains not printable characters */
    public final void m5065WWWWWWWW(String str, String str2) {
        VMConfig vMConfig = this.f8937WWWoWWWo;
        vMConfig.f8892WWWWWWWW = str;
        vMConfig.f8893WWWWWWWW = str2;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        byte[] bArr = {81, ConstantPoolEntry.CP_InterfaceMethodref, 98, 46, 63, -89, -71, 22, 89, 9};
        byte[] bArr2 = {TarConstants.LF_FIFO, 123, 23, 113, 73, -62, -41, 114};
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), str).putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{126, 37, -76, 96, 17, 25, 47, -44, 124, 39, -92, TarConstants.LF_MULTIVOLUME}, new byte[]{25, 85, -63, 63, 99, 124, 65, -80}), str2).apply();
    }

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final void m5066WWWWWWWW(String str) {
        this.f8937WWWoWWWo.f8909WWoWWo = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        byte[] bArr = {-4, 36, 34, -19, -50, TarConstants.LF_NORMAL, 84, -73};
        byte[] bArr2 = {-112, TarConstants.LF_GNUTYPE_LONGLINK, 65, -116, -94, 111, 61, -57};
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), str).apply();
    }

    /* renamed from: WWWWཀྵWWWWࠤཀྵ  reason: contains not printable characters */
    public final void m5067WWWWWWWW(String str) {
        this.f8937WWWoWWWo.f8901WWWoWWWo = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-111, -51, -70, -94, 114, 117, 66, -37, -94, -42, -96, -77, 99}, new byte[]{-3, -94, -39, -61, 6, 28, 45, -75}), str).apply();
        HALManager hALManager = this.f8933WWWWWWWW;
        if (hALManager != null) {
            LocationService locationService = hALManager.getLocationService();
            if (locationService.f9056WWWWWWWW) {
                locationService.m5136WWWoWWWo();
            }
            if (locationService.f9062WWoWWo && !locationService.f9056WWWWWWWW) {
                locationService.m5134WWWWWWWW(locationService.f9061WWWoWWWo);
            }
        }
    }

    /* renamed from: WWWWၗWWWW३ၗ  reason: contains not printable characters */
    public final void m5068WWWWWWWW(String str, String str2) {
        VMConfig vMConfig = this.f8937WWWoWWWo;
        vMConfig.f8914WWWW = str;
        if (str2 != null) {
            vMConfig.f8920WoWo = str2;
        }
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{122, 72, 56, 58}, new byte[]{23, 45, 81, 94, -112, -106, -60, -74}), vMConfig.f8914WWWW).putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{66, 17, -122}, new byte[]{39, 98, -24, -14, 15, 107, -70, 95}), vMConfig.f8920WoWo).apply();
    }

    /* renamed from: WWWWᄳWWWW़ᄳ  reason: contains not printable characters */
    public final void m5069WWWWWWWW(String str) {
        this.f8937WWWoWWWo.f8877WWWWWWWW = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, 40, -12, 59, -100, 60, -1, -117, -79, TarConstants.LF_CONTIG, -12, 39, -110, 60, -30, -102, -92, TarConstants.LF_BLK, -18, 38}, new byte[]{-59, 64, -101, 85, -7, 99, -111, -18}), str).apply();
    }

    /* renamed from: WWWWሎWWWWܣሎ  reason: contains not printable characters */
    public final void m5070WWWWWWWW(String str) {
        this.f8937WWWoWWWo.f8915WWWW = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        byte[] bArr = {0, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -12, -86, 16, 101, 40, -76, 3, 111, -12, -76, 1, TarConstants.LF_GNUTYPE_SPARSE, TarConstants.LF_BLK, -73};
        byte[] bArr2 = {112, TarConstants.LF_NORMAL, -101, -60, 117, 58, 91, -39};
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), str).apply();
    }

    /* renamed from: WWWWሗWWWWшሗ  reason: contains not printable characters */
    public final void m5071WWWWWWWW(boolean z10) {
        this.f8937WWWoWWWo.f8886WWWWWWWW = z10;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        AbstractC1017WWWoWWWo.m3456WWoWWo(StringFog.f8859WWWWWWWW, new byte[]{-52, -69, TarConstants.LF_DIR, -45, 91, -41, 97, Byte.MIN_VALUE, -34, -69, TarConstants.LF_LINK, -50}, new byte[]{-68, -41, 84, -86, 4, -78, 15, -31}, edit, z10);
    }

    /* renamed from: WWWWኈWWWWˡኈ  reason: contains not printable characters */
    public final void m5072WWWWWWWW(int i10) {
        this.f8937WWWoWWWo.f8918WW = i10;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        byte[] bArr = {-83, 24, -36, -60, -65, TarConstants.LF_BLK, 105, -112};
        StringFog.f8859WWWWWWWW.getClass();
        edit.putInt(WWWWWWWW.m17835WWWWWWWW(new byte[]{-33, 125, -70, -74, -38, 71, 1, -49, -33, 121, -88, -95}, bArr), i10).apply();
    }

    /* renamed from: WWWWዪWWWWͯዪ  reason: contains not printable characters */
    public final void m5073WWWWWWWW(String str) {
        this.f8937WWWoWWWo.f8869WWWWWWWW = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-39, -104, -78, -91, 65, 115}, new byte[]{-86, -3, -64, -52, 32, 31, -37, 47}), str).apply();
    }

    /* renamed from: WWWWᏊWWWWటᏊ  reason: contains not printable characters */
    public final void m5074WWWWWWWW(int i10, String str, String str2, String str3) {
        VMConfig vMConfig = this.f8937WWWoWWWo;
        vMConfig.f8881WWWWWWWW = str;
        vMConfig.f8882WWWWWWWW = i10;
        vMConfig.f8883WWWWWWWW = str2;
        vMConfig.f8917WWWW = str3;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{29, Byte.MIN_VALUE, -9, -90, 24, 84, -35, 121, ConstantPoolEntry.CP_InterfaceMethodref, -99, -30, -88, 25}, new byte[]{110, -17, -108, -51, 107, 97, -126, 10}), str).putInt(WWWWWWWW.m17835WWWWWWWW(new byte[]{-50, -17, 90, 35, 17, 89, 46, 124, -46, -14, TarConstants.LF_MULTIVOLUME}, new byte[]{-67, Byte.MIN_VALUE, 57, 72, 98, 108, 113, ConstantPoolEntry.CP_NameAndType}), i10).putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-81, 6, 21, -68, 107, 84, 4, -51, -81, ConstantPoolEntry.CP_NameAndType, 4, -71, 121, ConstantPoolEntry.CP_NameAndType, 62}, new byte[]{-36, 105, 118, -41, 24, 97, 91, -72}), str2).putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{22, 15, -87, 111, 28, -25, -87, 17, 4, 19, -71, 115, 0, -96, -110}, new byte[]{101, 96, -54, 4, 111, -46, -10, 97}), str3).apply();
    }

    /* renamed from: WWWWᐡWWWWೱᐡ  reason: contains not printable characters */
    public final void m5075WWWWWWWW(String str) {
        this.f8937WWWoWWWo.f8908WWoWWo = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{89, 115, -103, 3, -92, -40, -57, 72, 74}, new byte[]{46, 26, -1, 106, -5, -85, -76, 33}), str).apply();
    }

    /* renamed from: WWWWᓽWWWWϼᓽ  reason: contains not printable characters */
    public final void m5076WWWWWWWW() {
        String str = Build.FINGERPRINT;
        this.f8937WWWoWWWo.f8894WWWWWWWW = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-42, -116, -120, 42, -113, 115, 79, 112, -37, -121, -103, 38, -98, 102, 98, Byte.MAX_VALUE, -36, -99}, new byte[]{-78, -23, -2, 67, -20, 22, 16, 22}), str).apply();
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final void m5077WWWoWWWo(Set set) {
        byte[] bArr = {-9, ConstantPoolEntry.CP_NameAndType, 35, TarConstants.LF_FIFO, 61, 45, 74, 78, -14, 16, 34, 46, 57, 57, 102};
        byte[] bArr2 = {-124, 117, 80, 66, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 64, 21, 33};
        StringFog.f8859WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
        C0434WWWWWWWW c0434wwwwwwww = new C0434WWWWWWWW();
        SharedPreferences sharedPreferences = this.f8926WWWWoWWWWo;
        Set<String> stringSet = sharedPreferences.getStringSet(m17835WWWWWWWW, c0434wwwwwwww);
        C0434WWWWWWWW c0434wwwwwwww2 = new C0434WWWWWWWW();
        c0434wwwwwwww2.addAll(stringSet);
        c0434wwwwwwww2.addAll(set);
        sharedPreferences.edit().putStringSet(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -80, 35, -85, 6, -27, -63, -41, 78, -84, 34, -77, 2, -15, -19}, new byte[]{56, -55, 80, -33, 99, -120, -98, -72}), c0434wwwwwwww2).apply();
    }

    /* renamed from: WWWoԻWWWoͷԻ  reason: contains not printable characters */
    public final String m5078WWWoWWWo(String str) {
        HashMap hashMap = this.f8937WWWoWWWo.f8870WWWWWWWW;
        byte[] bArr = {-15, 113, -15, -118, 115, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 106, -22};
        StringFog.f8859WWWWWWWW.getClass();
        String str2 = (String) hashMap.get(WWWWWWWW.m17835WWWWWWWW(new byte[]{-127, 3, -98, -6, 93}, bArr).concat(str));
        if (TextUtils.isEmpty(str2)) {
            return null;
        }
        return str2;
    }

    /* renamed from: WWWoࠟWWWoॣࠟ  reason: contains not printable characters */
    public final void m5079WWWoWWWo(String str, String str2, String str3) {
        try {
            JSONObject jSONObject = new JSONObject();
            byte[] bArr = {TarConstants.LF_LINK, 35, 122};
            byte[] bArr2 = {65, 72, 29, 124, TarConstants.LF_NORMAL, -104, -26, -112};
            StringFog.f8859WWWWWWWW.getClass();
            jSONObject.put(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), str);
            jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{-13, 86, -21}, new byte[]{-112, 59, -113, TarConstants.LF_CHR, -120, 114, 13, 41}), str2);
            jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{96, 1, -97}, new byte[]{1, 115, -8, -85, 117, 99, 33, -7}), str3);
            VMEventManager vMEventManager = this.f8935WWWWWWWW;
            if (vMEventManager != null) {
                vMEventManager.m5116WWWoWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{Byte.MIN_VALUE, 62, Byte.MAX_VALUE, 80, -25, TarConstants.LF_DIR, -111, 42, -116, 56, 118, 80, -16, TarConstants.LF_FIFO, -106, TarConstants.LF_CONTIG, -111, TarConstants.LF_BLK, 60, 31, -27, 47, -100, TarConstants.LF_CONTIG, -115, Byte.MAX_VALUE, 87, 38, -61, 24, -96, ConstantPoolEntry.CP_NameAndType, -90, 14, 81, TarConstants.LF_LINK, -53, 22, -76, 22, -89}, new byte[]{-29, 81, 18, 126, -122, 91, -11, TarConstants.LF_PAX_EXTENDED_HEADER_UC}), jSONObject.toString());
            }
        } catch (Throwable unused) {
        }
    }

    /* renamed from: WWWoૄWWWoѽૄ  reason: contains not printable characters */
    public final void m5080WWWoWWWo(boolean z10) {
        m5098WoWo().post(new RunnableC1624WWWWWWWW(this, z10, 1));
        this.f8937WWWoWWWo.f8887WWWWWWWW = z10;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        AbstractC1017WWWoWWWo.m3456WWoWWo(StringFog.f8859WWWWWWWW, new byte[]{-90, -10, Byte.MAX_VALUE, 22, 3, 126, -102, 10, -86, -30, 96, 37, 21, 118, -122}, new byte[]{-60, -125, 22, 122, 119, 23, -12, 85}, edit, z10);
    }

    /* renamed from: WWWoၙWWWo੮ၙ  reason: contains not printable characters */
    public final void m5081WWWoWWWo(boolean z10) {
        this.f8937WWWoWWWo.f8922WoWo = z10;
        this.f8929WWWWWWWW = null;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        AbstractC1017WWWoWWWo.m3456WWoWWo(StringFog.f8859WWWWWWWW, new byte[]{-37, -76, -36, -86, -51, -112, 38, -55, -57, -66, -51, -89, -6, -86, 59, -53, -41, -73, -51}, new byte[]{-75, -37, -88, -55, -91, -49, 85, -86}, edit, z10);
    }

    /* renamed from: WWWoᐣWWWoҁᐣ  reason: contains not printable characters */
    public final void m5082WWWoWWWo(boolean z10) {
        this.f8937WWWoWWWo.f8911WWoWWo = z10;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        AbstractC1017WWWoWWWo.m3456WWoWWo(StringFog.f8859WWWWWWWW, new byte[]{90, 117, -110, 80, -62, 67, 25, 15, TarConstants.LF_GNUTYPE_LONGNAME, 100, -97, 79, -62, 67}, new byte[]{34, 5, -3, 35, -89, 39, 70, 106}, edit, z10);
    }

    /* renamed from: WWWᏛWWW෮Ꮫ  reason: contains not printable characters */
    public final void m5083WWWWWW(String str, String str2, boolean z10) {
        SharedPreferences sharedPreferences = this.f8926WWWWoWWWWo;
        VMConfig vMConfig = this.f8937WWWoWWWo;
        if (z10) {
            HashMap hashMap = vMConfig.f8870WWWWWWWW;
            StringBuilder sb2 = new StringBuilder();
            if (!hashMap.containsKey(AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-79, 72, -79, 97, 8}, new byte[]{-63, 58, -34, 17, 38, -41, -119, -69}, sb2, str)) && !TextUtils.isEmpty(str2)) {
                HashMap hashMap2 = vMConfig.f8870WWWWWWWW;
                hashMap2.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{-36, -125, 99, 6, -67, 47, 68, TarConstants.LF_GNUTYPE_LONGNAME, -51, -124, 96, 2, -67}, new byte[]{-84, -15, ConstantPoolEntry.CP_NameAndType, 118, -109, TarConstants.LF_GNUTYPE_LONGLINK, 33, 42}) + str, str2);
                SharedPreferences.Editor edit = sharedPreferences.edit();
                edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-46, 125, -122, ConstantPoolEntry.CP_InterfaceMethodref, -9, 34, -52, -99, -61, 122, -123, 15, -9}, new byte[]{-94, 15, -23, 123, -39, 70, -87, -5}) + str, str2).apply();
            }
        }
        if (TextUtils.isEmpty(str2)) {
            HashMap hashMap3 = vMConfig.f8870WWWWWWWW;
            StringBuilder sb3 = new StringBuilder();
            str2 = (String) hashMap3.get(AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{57, -125, -46, -92, 74, 66, -39, 26, 40, -124, -47, -96, 74}, new byte[]{73, -15, -67, -44, 100, 38, -68, 124}, sb3, str));
        }
        if (TextUtils.isEmpty(str2)) {
            return;
        }
        HashMap hashMap4 = vMConfig.f8870WWWWWWWW;
        StringBuilder sb4 = new StringBuilder();
        hashMap4.put(AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-31, TarConstants.LF_NORMAL, 43, 115, 8}, new byte[]{-111, 66, 68, 3, 38, -9, -21, 64}, sb4, str), str2);
        SharedPreferences.Editor edit2 = sharedPreferences.edit();
        edit2.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{1, TarConstants.LF_BLK, -38, -63, -108}, new byte[]{113, 70, -75, -79, -70, -101, 67, 18}) + str, str2).apply();
    }

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public final void m5084WWoWWo() {
        this.f8940WWoWWo = 4;
        VMStatusEvent vMStatusEvent = new VMStatusEvent(4, 0);
        C2467WWWWWWWW c2467wwwwwwww = this.f8939WWWoWWWo;
        c2467wwwwwwww.m13942WWWWWWWW(vMStatusEvent);
        ArrayList arrayList = new ArrayList();
        arrayList.add(new ApplyOverlaysTask());
        arrayList.add(new Bug1FixTask());
        arrayList.add(new Bug2FixTask());
        arrayList.add(new Bug3FixTask());
        arrayList.add(new Bug4FixTask());
        arrayList.add(new Bug5FixTask());
        arrayList.add(new Bug6FixTask());
        arrayList.add(new Bug7FixTask());
        arrayList.add(new Bug8FixTask());
        arrayList.add(new CleanLogTask());
        arrayList.add(new SuperuserTask());
        arrayList.add(new XposedTask());
        arrayList.add(new GooglePlayTask());
        arrayList.add(new MagiskTask());
        arrayList.add(new BuildTmpfsTask());
        arrayList.add(new BuildVMPropTask());
        arrayList.add(new BuildExecPathTask());
        int size = arrayList.size();
        int i10 = 0;
        while (true) {
            String str = f8925WWWoWWWo;
            if (i10 < size) {
                Object obj = arrayList.get(i10);
                i10++;
                IVMStartupTask iVMStartupTask = (IVMStartupTask) obj;
                byte[] bArr = {112, -73, -42, -104, 21, TarConstants.LF_GNUTYPE_LONGNAME, 95, -7};
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5041WWWWWWWW(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{43, -45, -71, -53, 97, 45, 45, -115, 63, -28, -117, -72, 102, 56, 62, -117, 4, -105, -94, -7, 102, 39, Byte.MAX_VALUE}, bArr).concat(iVMStartupTask.getName()));
                if (!iVMStartupTask.mo5039WWWoWWWo(this.f8927WWWWWWWW, this)) {
                    KLog.m5040WWWWoWWWWo(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-29, 78, 99, TarConstants.LF_NORMAL, -66, 2, -83, 90, -9, 121, 81, 67, -66, 2, -84, 69, -104}, new byte[]{-72, 42, ConstantPoolEntry.CP_NameAndType, 99, -54, 99, -33, 46}) + iVMStartupTask.getName() + WWWWWWWW.m17835WWWWWWWW(new byte[]{97, 70, -65, -96, 104, -80, -113, -44, 97, 42}, new byte[]{65, 32, -34, -55, 4, -43, -21, -18}) + iVMStartupTask.mo5038WWWWWWWW());
                    if (iVMStartupTask.mo5037WWWWoWWWWo()) {
                        this.f8940WWoWWo = -4;
                        int errorCode = iVMStartupTask.getErrorCode();
                        this.f8930WWWWWWWW = errorCode;
                        c2467wwwwwwww.m13942WWWWWWWW(new VMStatusEvent(this.f8940WWoWWo, errorCode));
                        return;
                    }
                }
            } else {
                byte[] bArr2 = {TarConstants.LF_PAX_EXTENDED_HEADER_UC, -59, -2, 14, -38, 85, -66, 32, TarConstants.LF_GNUTYPE_LONGNAME, -14, -52, 125, -35, 64, -83, 38, 119, -127, -2, 46};
                byte[] bArr3 = {3, -95, -111, 93, -82, TarConstants.LF_BLK, -52, 84};
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5041WWWWWWWW(str, WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                VMConfig vMConfig = this.f8937WWWoWWWo;
                int startOS = startOS(vMConfig.f8866WWWWWWWW, vMConfig.f8895WWWoWWWo.f8847WWWWWWWW, vMConfig.f8903WWoWWo);
                this.f8938WWWoWWWo = startOS;
                if (startOS < 0) {
                    KLog.m5040WWWWoWWWWo(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, 2, 80, -61, -43, 68, -102, 23, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_DIR, 98, -80, -46, 81, -119, 17, 99, 70, 80, -29, -127, 67, -119, 10, 123, 3, 91, -86, -127}, new byte[]{23, 102, 63, -112, -95, 37, -24, 99}) + this.f8938WWWoWWWo);
                    this.f8940WWoWWo = -4;
                    int i11 = (-this.f8938WWWoWWWo) + 117000;
                    this.f8930WWWWWWWW = i11;
                    c2467wwwwwwww.m13942WWWWWWWW(new VMStatusEvent(-4, i11));
                    return;
                }
                KLog.m5041WWWWWWWW(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{117, -66, -107, -68, 109, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -11, 67, 97, -119, -89, -49, 118, 74, -89, 68, 90, -69, -120, -101, 112, 87, -32}, new byte[]{46, -38, -6, -17, 25, 57, -121, TarConstants.LF_CONTIG}));
                this.f8940WWoWWo = 5;
                c2467wwwwwwww.m13942WWWWWWWW(new VMStatusEvent(5, 0));
                return;
            }
        }
    }

    /* renamed from: WWoڢWWo࢞ڢ  reason: not valid java name and contains not printable characters */
    public final boolean m5085WWoWWo() {
        if (this.f8937WWWoWWWo.f8888WWWWWWWW) {
            byte[] bArr = {-125, -14, -117, 46, 89, -38, 13, -119, -110, -7, -99, TarConstants.LF_LINK, 95, -64, 26, -50, -115, -14, -63, 31, 119, -2, 44, -11, -93};
            byte[] bArr2 = {-30, -100, -17, 92, TarConstants.LF_FIFO, -77, 105, -89};
            StringFog.f8859WWWWWWWW.getClass();
            if (r0.m2673WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2))) {
                return true;
            }
            return false;
        }
        return false;
    }

    /* renamed from: WWoॹWWoࠔॹ  reason: contains not printable characters */
    public final void m5086WWoWWo(int i10, int i11) {
        m5098WoWo().post(new RunnableC1621WWWWWWWW(this, i10, i11, 0));
    }

    /* renamed from: WWoহWWoȗহ  reason: contains not printable characters */
    public final void m5087WWoWWo(String str) {
        this.f8937WWWoWWWo.f8904WWoWWo = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-69, 29, -40, -83, 28, 119, -38, 18, -67}, new byte[]{-39, 124, -85, -56, 67, 21, -69, 124}), str).apply();
    }

    /* renamed from: WWo௹WWoਠ௹  reason: contains not printable characters */
    public final void m5088WWoWWo(String str) {
        this.f8937WWWoWWWo.f8861WWWWoWWWWo = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        byte[] bArr = {TarConstants.LF_BLK, 59, -16, 56, -87, -126, -66, 112};
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{90, 90, -99, 93}, bArr), str).apply();
        this.f8939WWWoWWWo.m13942WWWWWWWW(new VMConfigEvent());
    }

    /* renamed from: WWoၑWWoړၑ  reason: contains not printable characters */
    public final void m5089WWoWWo(boolean z10) {
        this.f8937WWWoWWWo.f8885WWWWWWWW = z10;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        AbstractC1017WWWoWWWo.m3456WWoWWo(StringFog.f8859WWWWWWWW, new byte[]{-59, -6, 92, TarConstants.LF_CHR, 1, TarConstants.LF_CONTIG, -104, 59, -58, -6, 89, TarConstants.LF_FIFO, 23, 56}, new byte[]{-88, -101, 59, 90, 114, 92, -57, 94}, edit, z10);
    }

    /* renamed from: WWoᆑWWoӁᆑ  reason: contains not printable characters */
    public final void m5090WWoWWo(String str) {
        this.f8937WWWoWWWo.f8862WWWWoWWWWo = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-67, -113, 28, -91, -101, -53, 126, -85, -71, -112, 28, -71, -107, -53, 100, -73, -67, -126}, new byte[]{-51, -25, 115, -53, -2, -108, 16, -50}), str).apply();
    }

    /* renamed from: WWoዕWWoూዕ  reason: contains not printable characters */
    public final void m5091WWoWWo(String str, String str2) {
        HALManager hALManager = this.f8933WWWWWWWW;
        if (hALManager != null) {
            SensorService sensorService = hALManager.getSensorService();
            int i10 = 0;
            while (true) {
                if (i10 < 12) {
                    Sensor sensor = sensorService.f9101WWWWWWWW[i10];
                    if (sensor != null && sensor.getStringType().equals(str)) {
                        break;
                    }
                    i10++;
                } else {
                    sensorService.getClass();
                    i10 = -1;
                    break;
                }
            }
            if (i10 >= 0 && i10 < 12) {
                sensorService.m5186WWWWWWWW(i10, str2);
            }
        }
        VMConfig vMConfig = this.f8937WWWoWWWo;
        vMConfig.f8864WWWWoWWWWo.put(str, str2);
        HashSet hashSet = new HashSet();
        for (String str3 : vMConfig.f8864WWWWoWWWWo.keySet()) {
            StringBuilder m1577WWWWoWWWWo = AbstractC0470WWWWWWWW.m1577WWWWoWWWWo(str3);
            pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-51}, new byte[]{-16, 40, 18, -34, 85, 112, TarConstants.LF_GNUTYPE_SPARSE, 6}, m1577WWWWoWWWWo);
            m1577WWWWoWWWWo.append((String) vMConfig.f8864WWWWoWWWWo.get(str3));
            hashSet.add(m1577WWWWoWWWWo.toString());
        }
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        byte[] bArr = {90, -57, 39, 64, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -44, -45, 36, 72, -50, 60, 86, 68};
        byte[] bArr2 = {41, -94, 73, TarConstants.LF_CHR, TarConstants.LF_CONTIG, -90, -116, 82};
        StringFog.f8859WWWWWWWW.getClass();
        edit.putStringSet(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), hashSet).apply();
    }

    /* renamed from: WWoᐛWWoʄᐛ  reason: contains not printable characters */
    public final void m5092WWoWWo(String str) {
        this.f8937WWWoWWWo.f8879WWWWWWWW = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-52, -109, 123, -8, -39, -19, 1, 30, -46, -98}, new byte[]{-69, -6, 29, -111, -122, -113, 114, 109}), str).apply();
    }

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public final void m5093WWWW(VMResConfig vMResConfig) {
        if (this.f8929WWWWWWWW == null) {
            VMResConfig vMResConfig2 = new VMResConfig();
            this.f8929WWWWWWWW = vMResConfig2;
            VMConfig vMConfig = this.f8937WWWoWWWo;
            VMResConfig vMResConfig3 = vMConfig.f8900WWWoWWWo;
            vMResConfig2.f8953WWWWWWWW = vMResConfig3.f8953WWWWWWWW;
            vMResConfig2.f8952WWWWoWWWWo = vMResConfig3.f8952WWWWoWWWWo;
            vMResConfig2.f8955WWWoWWWo = vMResConfig3.f8955WWWoWWWo;
            vMResConfig2.f8954WWWWWWWW = vMResConfig3.f8954WWWWWWWW;
            if (!vMConfig.f8922WoWo) {
                StringFog.f8859WWWWWWWW.getClass();
                if (WWWWWWWW.m17835WWWWWWWW(new byte[]{95, 97, -9, -84, -67, -1, 67, -94, 84, 118, -11, -73, -65, -13, 104}, new byte[]{59, 4, -127, -59, -34, -102, 28, -46}).equals(this.f8929WWWWWWWW.f8953WWWWWWWW)) {
                    this.f8929WWWWWWWW.f8955WWWoWWWo -= WWWW.m5342WWoWWo();
                } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-32, -64, 81, -24, -60, -103, -87, -43, -27, -53, 67, -14, -60, -99, -122, -36}, new byte[]{-124, -91, 39, -127, -89, -4, -10, -71}).equals(this.f8929WWWWWWWW.f8953WWWWWWWW)) {
                    this.f8929WWWWWWWW.f8952WWWWoWWWWo -= WWWW.m5342WWoWWo();
                }
            }
        }
        VMResConfig vMResConfig4 = this.f8929WWWWWWWW;
        vMResConfig.f8953WWWWWWWW = vMResConfig4.f8953WWWWWWWW;
        vMResConfig.f8952WWWWoWWWWo = vMResConfig4.f8952WWWWoWWWWo;
        vMResConfig.f8955WWWoWWWo = vMResConfig4.f8955WWWoWWWo;
        vMResConfig.f8954WWWWWWWW = vMResConfig4.f8954WWWWWWWW;
    }

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public final void m5094WWWW(String str) {
        this.f8937WWWoWWWo.f8880WWWWWWWW = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{125, 33, -4, -115, -75, TarConstants.LF_LINK, -125, -102, 114}, new byte[]{17, 78, -97, -20, -39, 110, -18, -5}), str).apply();
    }

    /* renamed from: WWၚஊၚ  reason: contains not printable characters */
    public final void m5095WW(String str) {
        this.f8937WWWoWWWo.f8878WWWWWWWW = str;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{Byte.MIN_VALUE, Byte.MIN_VALUE, 124, -67, -89, -74, -108, -27, -111, -124, TarConstants.LF_GNUTYPE_LONGNAME, -68, -78, -99, -103, -29, -98}, new byte[]{-16, -24, 19, -45, -62, -23, -16, -116}), str).apply();
    }

    /* renamed from: WWኮWWႉኮ  reason: contains not printable characters */
    public final void m5096WWWW(boolean z10) {
        this.f8937WWWoWWWo.f8902WWWWWW = z10;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        AbstractC1017WWWoWWWo.m3456WWoWWo(StringFog.f8859WWWWWWWW, new byte[]{23, Byte.MAX_VALUE, -100, -112, -17, -107, -55, -94, 7, 124, -106, Byte.MIN_VALUE}, new byte[]{101, 16, -13, -28, -80, -16, -89, -61}, edit, z10);
    }

    /* renamed from: WWᐤԂᐤ  reason: contains not printable characters */
    public final void m5097WW(final boolean z10, final boolean z11) {
        m5098WoWo().post(new Runnable() { // from class: com.android.vmcore.WWoϫWWoӉϫ
            /* JADX WARN: Multi-variable type inference failed */
            /* JADX WARN: Type inference failed for: r2v0, types: [java.lang.Object, com.android.vmcore.event.ShutdownEvent] */
            @Override // java.lang.Runnable
            public final void run() {
                String str = VMInstance.f8925WWWoWWWo;
                VMInstance vMInstance = VMInstance.this;
                boolean m5057WWWWWWWW = vMInstance.m5057WWWWWWWW();
                ?? obj = new Object();
                obj.f9012WWWWWWWW = m5057WWWWWWWW;
                boolean z12 = z10;
                obj.f9011WWWWoWWWWo = z12;
                vMInstance.f8939WWWoWWWo.m13940WWWWWWWW(obj);
                if (z11 && m5057WWWWWWWW) {
                    vMInstance.f8937WWWoWWWo.f8913WWoWWo = true;
                }
                if (z12 && m5057WWWWWWWW) {
                    vMInstance.m5100WoWo(false);
                }
            }
        });
    }

    /* renamed from: WoڄWoᄴڄ  reason: contains not printable characters */
    public final synchronized Handler m5098WoWo() {
        Handler handler = this.f8943WWoWWo;
        if (handler != null) {
            return handler;
        }
        HandlerThread handlerThread = new HandlerThread(f8925WWWoWWWo);
        this.f8936WWWWWWWW = handlerThread;
        handlerThread.start();
        Handler handler2 = new Handler(this.f8936WWWWWWWW.getLooper());
        this.f8943WWoWWo = handler2;
        return handler2;
    }

    /* renamed from: WoოWo੍ო  reason: contains not printable characters */
    public final void m5099WoWo(String str, String str2, String str3) {
        VMConfig vMConfig = this.f8937WWWoWWWo;
        vMConfig.f8924o = str;
        vMConfig.f8907WWoWWo = str2;
        vMConfig.f8876WWWWWWWW = str3;
        SharedPreferences.Editor edit = this.f8926WWWWoWWWWo.edit();
        StringFog.f8859WWWWWWWW.getClass();
        edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-1, 27, 78, TarConstants.LF_BLK, 42, 47, -76, 90, -5, 4, 78, 40, 36, 47, -71, 94, -3, 1, 72, 63, 61}, new byte[]{-113, 115, 33, 90, 79, 112, -38, 63}), str).putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{119, 25, -49, 7, 119, -76, 116, 116, 115, 6, -49, 27, 121, -76, 105, 97, 105}, new byte[]{7, 113, -96, 105, 18, -21, 26, 17}), str2).putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{38, -38, 20, TarConstants.LF_GNUTYPE_LONGNAME, TarConstants.LF_SYMLINK, -127, 21, -19, 34, -59, 20, 80, 60, -127, 22, -21, TarConstants.LF_DIR, -33, 21, 65}, new byte[]{86, -78, 123, 34, 87, -34, 123, -120}), str3).apply();
    }

    /* renamed from: WoᒧWoᄜᒧ  reason: contains not printable characters */
    public final void m5100WoWo(final boolean z10) {
        if (this.f8940WWoWWo < 0) {
            C2467WWWWWWWW c2467wwwwwwww = this.f8939WWWoWWWo;
            synchronized (c2467wwwwwwww.f26784WWWoWWWo) {
                VMStatusEvent.class.cast(c2467wwwwwwww.f26784WWWoWWWo.remove(VMStatusEvent.class));
            }
        }
        final RomConfig romConfig = this.f8928WWWWWWWW;
        if (romConfig == null) {
            romConfig = this.f8937WWWoWWWo.f8895WWWoWWWo;
        }
        m5098WoWo().post(new Runnable() { // from class: com.android.vmcore.WWWWϙWWWWეϙ
            @Override // java.lang.Runnable
            public final void run() {
                boolean m5240WWWWoWWWWo;
                boolean z11;
                int i10 = 1;
                VMInstance vMInstance = VMInstance.this;
                if (vMInstance.f8940WWoWWo <= 0) {
                    vMInstance.f8940WWoWWo = 0;
                    StringFog.f8859WWWWWWWW.getClass();
                    String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-108, -44, -84, -52}, new byte[]{-6, -69, -62, -87, -114, -46, 118, -66});
                    VMConfig vMConfig = vMInstance.f8937WWWoWWWo;
                    vMConfig.f8923WoWo = m17835WWWWWWWW;
                    VMStatusEvent vMStatusEvent = new VMStatusEvent(vMInstance.f8940WWoWWo, 0);
                    C2467WWWWWWWW c2467wwwwwwww2 = vMInstance.f8939WWWoWWWo;
                    c2467wwwwwwww2.m13942WWWWWWWW(vMStatusEvent);
                    vMInstance.f8940WWoWWo = 1;
                    c2467wwwwwwww2.m13942WWWWWWWW(new VMStatusEvent(1, 0));
                    if (vMConfig.f8895WWWoWWWo.f8855WWoWWo) {
                        m5240WWWWoWWWWo = CPUUtils.m5242WWWoWWWo();
                    } else {
                        m5240WWWWoWWWWo = CPUUtils.m5240WWWWoWWWWo();
                    }
                    if (!m5240WWWWoWWWWo) {
                        vMInstance.f8940WWoWWo = -1;
                        vMInstance.f8930WWWWWWWW = 101000;
                        c2467wwwwwwww2.m13942WWWWWWWW(new VMStatusEvent(-1, 101000));
                        return;
                    }
                    VMApp vMApp = vMInstance.f8927WWWWWWWW;
                    String str = vMApp.getApplicationInfo().dataDir;
                    if (!str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{66, 44, TarConstants.LF_GNUTYPE_LONGNAME, 25, 59, -57, -53, 92, 25, 41}, new byte[]{109, 72, 45, 109, 90, -24, -81, 61})) && !str.startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -89, -111, -52, 6, 119, 35, -62, 1, -79, -33, -120, 72}, new byte[]{100, -61, -16, -72, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 86, -79}))) {
                        vMInstance.f8940WWoWWo = -1;
                        vMInstance.f8930WWWWWWWW = 102000;
                        c2467wwwwwwww2.m13942WWWWWWWW(new VMStatusEvent(-1, 102000));
                        return;
                    }
                    int i11 = Build.VERSION.SDK_INT;
                    RomConfig romConfig2 = vMConfig.f8895WWWoWWWo;
                    if (i11 >= romConfig2.f8849WWWWWWWW) {
                        if (!TextUtils.isEmpty(romConfig2.f8850WWWWWWWW)) {
                            if (WWWW.m5327WWWWWWWW().compareTo(vMConfig.f8895WWWoWWWo.f8850WWWWWWWW) < 0) {
                                vMInstance.f8940WWoWWo = -1;
                                vMInstance.f8930WWWWWWWW = 101500;
                                c2467wwwwwwww2.m13942WWWWWWWW(new VMStatusEvent(-1, 101500));
                                return;
                            }
                        }
                        if (!vMInstance.m5056WWWWWWWW()) {
                            vMInstance.f8940WWoWWo = -1;
                            vMInstance.f8930WWWWWWWW = 103000;
                            c2467wwwwwwww2.m13942WWWWWWWW(new VMStatusEvent(-1, 103000));
                            return;
                        }
                        if (z10) {
                            vMConfig.f8913WWoWWo = false;
                        }
                        RomConfig romConfig3 = romConfig;
                        if (romConfig3.f8853WWWoWWWo != vMConfig.f8895WWWoWWWo.f8853WWWoWWWo) {
                            z11 = true;
                        } else {
                            z11 = false;
                        }
                        boolean z12 = vMConfig.f8919WWWW;
                        String str2 = VMInstance.f8925WWWoWWWo;
                        SharedPreferences sharedPreferences = vMInstance.f8926WWWWoWWWWo;
                        if (z12 && !vMConfig.f8913WWoWWo && !z11) {
                            vMConfig.f8895WWWoWWWo = romConfig3;
                            try {
                                sharedPreferences.edit().putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{126, 121, 41, -40, 121, 34, 91, -19, 101, 113}, new byte[]{ConstantPoolEntry.CP_NameAndType, 22, 68, -121, 26, TarConstants.LF_MULTIVOLUME, TarConstants.LF_DIR, -117}), romConfig3.m5048WWWoWWWo()).apply();
                            } catch (Exception e10) {
                                e10.printStackTrace();
                                StringFog.f8859WWWWWWWW.getClass();
                                KLog.m5044WWWoWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-54, 108, -50, -39, -96, -13, -71, -98, -48, 113, -11, -37, -69, -8, -1, -123, -40, 60, -52, -39, -67, -6, -4, -120}, new byte[]{-65, 28, -86, -72, -44, -106, -103, -20}), e10);
                            }
                        } else {
                            if (!z12) {
                                vMConfig.f8923WoWo = WWWWWWWW.m17835WWWWWWWW(new byte[]{10, -111, -101, -88}, new byte[]{99, -1, -14, -36, TarConstants.LF_FIFO, -4, -96, 59});
                            } else if (vMConfig.f8913WWoWWo) {
                                vMConfig.f8923WoWo = WWWWWWWW.m17835WWWWWWWW(new byte[]{8, -36, 34, -121, -97, -101}, new byte[]{122, -71, 82, -26, -10, -23, -116, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER});
                            } else {
                                vMConfig.f8923WoWo = WWWWWWWW.m17835WWWWWWWW(new byte[]{34, -98, -121, -123, Byte.MIN_VALUE, 113}, new byte[]{87, -18, -29, -28, -12, 20, -88, 64});
                            }
                            RomConfig romConfig4 = vMConfig.f8895WWWoWWWo;
                            vMConfig.f8895WWWoWWWo = romConfig3;
                            vMInstance.f8940WWoWWo = 2;
                            c2467wwwwwwww2.m13942WWWWWWWW(new VMStatusEvent(2, 0));
                            ArrayList arrayList = new ArrayList();
                            arrayList.add(new PrepareFsTask());
                            arrayList.add(new InstallFsTask());
                            arrayList.add(new FixFsTask());
                            arrayList.add(new CleanFsTask());
                            arrayList.add(new ChmodFsTask());
                            arrayList.add(new CleanCacheTask());
                            arrayList.add(new FixCPUArchTask());
                            arrayList.add(new LoadVMPropTask());
                            int size = arrayList.size();
                            int i12 = 0;
                            while (i12 < size) {
                                Object obj = arrayList.get(i12);
                                i12 += i10;
                                IVMSetupTask iVMSetupTask = (IVMSetupTask) obj;
                                StringFog.f8859WWWWWWWW.getClass();
                                KLog.m5041WWWWWWWW(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{45, 13, -47, -44, -84, 93, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 71, 26, 5, -24, -48, -97, 14, 95, 82, 23, 27, -54, -67, -74, 79, 95, TarConstants.LF_MULTIVOLUME, 86}, new byte[]{118, 105, -66, -99, -62, 46, 44, 38}).concat(iVMSetupTask.getName()));
                                if (!iVMSetupTask.mo5036WWWoWWWo(vMApp, vMInstance)) {
                                    KLog.m5040WWWWoWWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_UC, -17, -12, -112, 1, -19, -50, -89, 111, -25, -51, -108, TarConstants.LF_SYMLINK, -66, -50, -89, 112, -32, -69}, new byte[]{3, -117, -101, -39, 111, -98, -70, -58}) + iVMSetupTask.getName() + WWWWWWWW.m17835WWWWWWWW(new byte[]{-14, 110, -80, -89, 57, -40, 1, 34, -14, 2}, new byte[]{-46, 8, -47, -50, 85, -67, 101, 24}) + iVMSetupTask.mo5035WWWWWWWW());
                                    if (iVMSetupTask.mo5034WWWWoWWWWo()) {
                                        vMInstance.f8940WWoWWo = -2;
                                        int errorCode = iVMSetupTask.getErrorCode();
                                        vMInstance.f8930WWWWWWWW = errorCode;
                                        c2467wwwwwwww2.m13942WWWWWWWW(new VMStatusEvent(vMInstance.f8940WWoWWo, errorCode));
                                        vMConfig.f8895WWWoWWWo = romConfig4;
                                        return;
                                    }
                                }
                                i10 = 1;
                            }
                            vMConfig.f8913WWoWWo = false;
                            vMConfig.f8919WWWW = true;
                            SharedPreferences.Editor edit = sharedPreferences.edit();
                            AbstractC1017WWWoWWWo.m3456WWoWWo(StringFog.f8859WWWWWWWW, new byte[]{TarConstants.LF_GNUTYPE_SPARSE, 100, 2, 17, -55, 17, -12, -29}, new byte[]{59, 5, 113, 78, -96, Byte.MAX_VALUE, -99, -105}, edit, true);
                            try {
                                sharedPreferences.edit().putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{91, -46, 111, -127, 92, -89, TarConstants.LF_LINK, 68, 64, -38}, new byte[]{41, -67, 2, -34, 63, -56, 95, 34}), romConfig3.m5048WWWoWWWo()).putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-88, 116, -39, -104, 111, 6, -118, 4, -101, 118, -59, -126, 86, 29, -126}, new byte[]{-60, 21, -86, -20, TarConstants.LF_NORMAL, 116, -27, 105}), romConfig4.m5048WWWoWWWo()).apply();
                            } catch (Exception e11) {
                                e11.printStackTrace();
                                byte[] bArr = {42, 115, -66, -64, -80, -41, 71, -10, TarConstants.LF_NORMAL, 110, -123, -62, -85, -36, 1, -19, 56, 35, -68, -64, -83, -34, 2, -32};
                                byte[] bArr2 = {95, 3, -38, -95, -60, -78, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -124};
                                StringFog.f8859WWWWWWWW.getClass();
                                KLog.m5044WWWoWWWo(str2, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), e11);
                            }
                        }
                        byte[] bArr3 = {56, -45, -29, 41, 72, 106, 122, -70, TarConstants.LF_DIR, -6, -47, 90, 79, Byte.MAX_VALUE, 105, -68, 23, -105, -6, 23, 28, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 126, -83};
                        byte[] bArr4 = {99, -73, -116, 122, 60, ConstantPoolEntry.CP_InterfaceMethodref, 8, -50};
                        WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
                        wwwwwwww.getClass();
                        KLog.m5041WWWWWWWW(str2, WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4));
                        byte[] bArr5 = {68, -106, 71, -115, -62, 86, -23, 36, TarConstants.LF_GNUTYPE_LONGNAME, -124, TarConstants.LF_GNUTYPE_LONGLINK, -125, -106, 68, -17, TarConstants.LF_LINK, 109, -122, 65, -80, -47, 23, -24, TarConstants.LF_DIR, 109, -124, 65, -67, -45, 68};
                        byte[] bArr6 = {31, -14, 40, -34, -74, TarConstants.LF_CONTIG, -101, 80};
                        wwwwwwww.getClass();
                        String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(bArr5, bArr6);
                        String str3 = VMInstance.f8925WWWoWWWo;
                        KLog.m5041WWWWWWWW(str3, m17835WWWWWWWW2);
                        KLog.m5041WWWWWWWW(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-7, 95, 112, -111, Byte.MIN_VALUE, -67, 44, 104, -52, 72, 112, -105, -88, -19, 29, 117, -61, 94, 97, -59, -105, -92, 0, 101, -57, 94}, new byte[]{-94, 44, 21, -27, -11, -51, 110, 1}));
                        int m5206WWWWoWWWWo = BinderService.m5206WWWWoWWWWo(vMApp, vMConfig.f8866WWWWWWWW);
                        C2467WWWWWWWW c2467wwwwwwww3 = vMInstance.f8939WWWoWWWo;
                        if (m5206WWWWoWWWWo == 0) {
                            KLog.m5041WWWWWWWW(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{124, -41, -91, -33, 17, 122, -125, -2, 87, -47, -76, -10, 68, 121, -66, -15, 85, -48, -32, -62, 10, 122, -65, -28}, new byte[]{39, -92, -64, -85, 100, 10, -54, -112}));
                            if (vMInstance.f8941WWoWWo == null) {
                                vMInstance.f8941WWoWWo = new InputService(vMInstance);
                            }
                            int m5130WWWWoWWWWo = vMInstance.f8941WWoWWo.m5130WWWWoWWWWo();
                            if (m5130WWWWoWWWWo == 0) {
                                KLog.m5041WWWWWWWW(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-59, 0, 70, -120, -79, 13, 71, 105, -6, 26, TarConstants.LF_GNUTYPE_LONGNAME, -95, -28, 14, 114, 125, -20, 7, 3, -99, -79, 25, 111, 115}, new byte[]{-98, 115, 35, -4, -60, 125, 6, 28}));
                                if (vMInstance.f8944WWWW == null) {
                                    vMInstance.f8944WWWW = new AudioService(vMApp, vMInstance);
                                }
                                int start = vMInstance.f8944WWWW.start();
                                if (start == 0) {
                                    KLog.m5041WWWWWWWW(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-90, 119, -76, -119, -93, -101, 29, -1, -79, 89, -15, -114, -94, -118, 39, -54, -35, 108, -80, -111}, new byte[]{-3, 4, -47, -3, -42, -21, 85, -66}));
                                    if (vMInstance.f8933WWWWWWWW == null) {
                                        vMInstance.f8933WWWWWWWW = new HALManager(vMApp, vMInstance);
                                    }
                                    int startHALMgr = vMInstance.f8933WWWWWWWW.startHALMgr();
                                    if (startHALMgr == 0) {
                                        KLog.m5041WWWWWWWW(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-59, TarConstants.LF_FIFO, TarConstants.LF_GNUTYPE_SPARSE, -43, 123, -85, 100, -90, -16, 33, TarConstants.LF_GNUTYPE_SPARSE, -45, TarConstants.LF_GNUTYPE_SPARSE, -5, 69, -73, -1, TarConstants.LF_CONTIG, 66, -127, 124, -66, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -89, -5, TarConstants.LF_CONTIG}, new byte[]{-98, 69, TarConstants.LF_FIFO, -95, 14, -37, TarConstants.LF_FIFO, -61}));
                                        if (vMInstance.f8945WoWo == null) {
                                            vMInstance.f8945WoWo = new DisplayService(vMInstance);
                                        }
                                        VMResConfig m5061WWWWWWWW = vMInstance.m5061WWWWWWWW();
                                        int m5129WWWoWWWo = vMInstance.f8945WoWo.m5129WWWoWWWo(m5061WWWWWWWW.f8952WWWWoWWWWo, m5061WWWWWWWW.f8955WWWoWWWo);
                                        if (m5129WWWoWWWo == 0) {
                                            KLog.m5041WWWWWWWW(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-29, -43, -69, -122, 34, -67, -51, -75, -52, -54, -73, -100, 60, -112, -93, -93, -52, -57, -84, -122, 119, -93, -26, -92, -44, -49, -80, -103}, new byte[]{-72, -90, -34, -14, 87, -51, -125, -48}));
                                            if (vMInstance.f8934WWWWWWWW == null) {
                                                vMInstance.f8934WWWWWWWW = new NetlinkManager(vMApp, vMInstance);
                                            }
                                            int start2 = vMInstance.f8934WWWWWWWW.start();
                                            if (start2 == 0) {
                                                VMNetworkConfig vMNetworkConfig = new VMNetworkConfig();
                                                vMNetworkConfig.ifname = WWWWWWWW.m17835WWWWWWWW(new byte[]{-15, -81, -35, -74, 62}, new byte[]{-122, -61, -68, -40, 14, -120, -31, -37});
                                                vMNetworkConfig.mac = vMConfig.f8880WWWWWWWW;
                                                vMNetworkConfig.ip = vMConfig.f8909WWoWWo;
                                                vMNetworkConfig.gateway_ip = vMConfig.f8916WW;
                                                vMNetworkConfig.dns_ip = WWWWWWWW.m17835WWWWWWWW(new byte[]{40, -127, -125, -60, 3, -36, -3}, new byte[]{16, -81, -69, -22, 59, -14, -59, TarConstants.LF_BLK});
                                                vMInstance.f8934WWWWWWWW.setNetworkConfig(vMNetworkConfig);
                                                KLog.m5041WWWWWWWW(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{38, 43, -21, 15, 19, 101, 0, -22, 20, 60, -23, 30, 59, TarConstants.LF_DIR, TarConstants.LF_LINK, -20, 28, 42, -6, 91, 4, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 43, -4, 26, 61, -82, 22, 7, 123, 35, -1, 24, 42}, new byte[]{125, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -114, 123, 102, 21, 66, -104}));
                                                VMEventManager vMEventManager = vMInstance.f8935WWWWWWWW;
                                                ArrayList arrayList2 = vMEventManager.f8989WWWWWWWW;
                                                if (!arrayList2.contains(vMInstance)) {
                                                    arrayList2.add(vMInstance);
                                                }
                                                vMEventManager.m5115WWWWWWWW();
                                                KLog.m5041WWWWWWWW(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{26, 116, 30, -58, 107, -62, -91, 82, 18, 102, 18, -56, 63, -62, -69, 74, 97, 99, 20, -25, 105, -54, -76, 67, TarConstants.LF_SYMLINK, TarConstants.LF_NORMAL, 2, -31, 126, -47, -93, 67, 37}, new byte[]{65, 16, 113, -107, 31, -93, -41, 38}));
                                                vMInstance.m5084WWoWWo();
                                                return;
                                            }
                                            KLog.m5040WWWWoWWWWo(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -61, -97, TarConstants.LF_NORMAL, -93, -38, 31, -34, -65, -36, -109, 42, -67, -9, 113, -56, -65, -47, -120, TarConstants.LF_NORMAL, -10, -60, TarConstants.LF_BLK, -49, -89, -39, -108, 47, -10, -52, TarConstants.LF_NORMAL, -46, -89, -43, -98, 126, -10}, new byte[]{-53, -80, -6, 68, -42, -86, 81, -69}) + start2);
                                            vMInstance.f8940WWoWWo = -3;
                                            int i13 = (-start2) + 112500;
                                            vMInstance.f8930WWWWWWWW = i13;
                                            c2467wwwwwwww3.m13942WWWWWWWW(new VMStatusEvent(-3, i13));
                                            return;
                                        }
                                        KLog.m5040WWWWoWWWWo(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-108, -90, -67, -8, -9, -91, 13, -23, -95, -79, -67, -2, -33, -11, 44, -8, -82, -89, -84, -84, -16, -80, TarConstants.LF_LINK, -24, -86, -89, -8, -22, -29, -68, TarConstants.LF_CHR, -23, -85, -17, -8}, new byte[]{-49, -43, -40, -116, -126, -43, 95, -116}) + m5129WWWoWWWo);
                                        vMInstance.f8940WWoWWo = -3;
                                        int i14 = (-m5129WWWoWWWo) + 109000;
                                        vMInstance.f8930WWWWWWWW = i14;
                                        c2467wwwwwwww3.m13942WWWWWWWW(new VMStatusEvent(-3, i14));
                                        return;
                                    }
                                    KLog.m5040WWWWoWWWWo(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{56, -6, -30, -61, -53, -121, -27, 79, 47, -44, -89, -60, -54, -106, -33, 122, 67, -31, -26, -37, -98, -111, -52, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 15, -20, -29, -115, -98}, new byte[]{99, -119, -121, -73, -66, -9, -83, 14}) + startHALMgr);
                                    vMInstance.f8940WWoWWo = -3;
                                    int i15 = (-startHALMgr) + 112000;
                                    vMInstance.f8930WWWWWWWW = i15;
                                    c2467wwwwwwww3.m13942WWWWWWWW(new VMStatusEvent(-3, i15));
                                    return;
                                }
                                KLog.m5040WWWWoWWWWo(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{16, -127, 66, -102, 25, -90, -111, 124, 47, -101, 72, -77, TarConstants.LF_GNUTYPE_LONGNAME, -91, -92, 104, 57, -122, 7, -113, 25, -78, -71, 102, 107, -108, 70, -121, 0, -77, -76, TarConstants.LF_CHR, 107}, new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -14, 39, -18, 108, -42, -48, 9}) + start);
                                vMInstance.f8940WWoWWo = -3;
                                int i16 = (-start) + 111000;
                                vMInstance.f8930WWWWWWWW = i16;
                                c2467wwwwwwww3.m13942WWWWWWWW(new VMStatusEvent(-3, i16));
                                return;
                            }
                            KLog.m5040WWWWoWWWWo(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, -40, -85, 111, -36, -106, 111, -47, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -34, -70, 70, -119, -107, 82, -34, 101, -33, -18, 114, -57, -106, TarConstants.LF_GNUTYPE_SPARSE, -53, TarConstants.LF_CONTIG, -51, -81, 114, -59, -125, 66, -123, TarConstants.LF_CONTIG}, new byte[]{23, -85, -50, 27, -87, -26, 38, -65}) + m5130WWWWoWWWWo);
                            vMInstance.f8940WWoWWo = -3;
                            int i17 = (-m5130WWWWoWWWWo) + 110000;
                            vMInstance.f8930WWWWWWWW = i17;
                            c2467wwwwwwww3.m13942WWWWWWWW(new VMStatusEvent(-3, i17));
                            return;
                        }
                        KLog.m5040WWWWoWWWWo(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{67, -37, 84, -49, -96, 109, TarConstants.LF_FIFO, 94, 118, -52, 84, -55, -120, 61, 7, 67, 121, -38, 69, -101, -73, 116, 26, TarConstants.LF_GNUTYPE_SPARSE, 125, -38, 17, -35, -76, 116, 24, 82, 124, -110, 17}, new byte[]{24, -88, TarConstants.LF_LINK, -69, -43, 29, 116, TarConstants.LF_CONTIG}) + m5206WWWWoWWWWo);
                        vMInstance.f8940WWoWWo = -3;
                        int i18 = (-m5206WWWWoWWWWo) + 114000;
                        vMInstance.f8930WWWWWWWW = i18;
                        c2467wwwwwwww3.m13942WWWWWWWW(new VMStatusEvent(-3, i18));
                        return;
                    }
                    vMInstance.f8940WWoWWo = -1;
                    vMInstance.f8930WWWWWWWW = 100500;
                    c2467wwwwwwww2.m13942WWWWWWWW(new VMStatusEvent(-1, 100500));
                }
            }
        });
    }

    /* renamed from: oેᄈે  reason: contains not printable characters */
    public final void m5101o(boolean z10) {
        String m17835WWWWWWWW;
        this.f8937WWWoWWWo.f8888WWWWWWWW = z10;
        AbstractC1017WWWoWWWo.m3456WWoWWo(StringFog.f8859WWWWWWWW, new byte[]{-125, -41, 47, -37, 20, -102, TarConstants.LF_GNUTYPE_LONGNAME, 96, -114, -41, 32, -46, 3, -97}, new byte[]{-32, -74, 66, -66, 102, -5, 19, 5}, this.f8926WWWWoWWWWo.edit(), z10);
        String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-57, -27, -33, Byte.MAX_VALUE, 33, -3, -69, TarConstants.LF_GNUTYPE_LONGLINK, -36, -19, -125, 118}, new byte[]{-79, -120, -15, 23, 86, -45, -40, 42});
        if (z10) {
            m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{25}, new byte[]{40, 17, 87, -48, 80, -112, -96, 22});
        } else {
            m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-36}, new byte[]{-20, 7, TarConstants.LF_LINK, -17, 34, -41, TarConstants.LF_FIFO, 63});
        }
        m5098WoWo().post(new WWWWWWWW(this, m17835WWWWWWWW2, m17835WWWWWWWW, 0));
        m5098WoWo().post(new WWWoWWWo(this, WWWWWWWW.m17835WWWWWWWW(new byte[]{-71, -75, 63, 37, 30, 62, -35, 97, -75, -77, TarConstants.LF_FIFO, 37, 28, TarConstants.LF_LINK, -44, 118, -88, -69, 96}, new byte[]{-38, -38, 82, ConstantPoolEntry.CP_InterfaceMethodref, Byte.MAX_VALUE, 80, -71, 19}), 1));
    }
}
