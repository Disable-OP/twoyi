package com.android.vmcore.hal;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.wifi.ScanResult;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.VMNetworkConfig;
import com.blankj.utilcode.util.AbstractC1631WWWWWWWW;
import com.blankj.utilcode.util.C1630WWWWWWWW;
import com.blankj.utilcode.util.C1644WWWoWWWo;
import com.blankj.utilcode.util.InterfaceC1653WoWo;
import com.blankj.utilcode.util.RunnableC1629WWWWWWWW;
import com.blankj.utilcode.util.WWWWoWWWWo;
import java.io.File;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CopyOnWriteArraySet;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p069WoWo.AbstractC0576WWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class NetlinkManager implements InterfaceC1653WoWo {
    private final Context mContext;
    private long mNativePtr;
    private final VMInstance mVM;

    public NetlinkManager(Context context, VMInstance vMInstance) {
        this.mContext = context;
        this.mVM = vMInstance;
        this.mNativePtr = nativeSetup(vMInstance.f8937WWWoWWWo.f8866WWWWWWWW);
    }

    @SuppressLint({"NewApi"})
    private List<ScanResult> buildDefaultBSS() {
        ArrayList arrayList = new ArrayList();
        try {
            ScanResult m1865WWWoWWWo = AbstractC0576WWWWWW.m1865WWWoWWWo();
            VMConfig vMConfig = this.mVM.f8937WWWoWWWo;
            m1865WWWoWWWo.BSSID = vMConfig.f8879WWWWWWWW;
            m1865WWWoWWWo.SSID = vMConfig.f8908WWoWWo;
            m1865WWWoWWWo.frequency = 0;
            m1865WWWoWWWo.level = 0;
            arrayList.add(m1865WWWoWWWo);
            ScanResult m1865WWWoWWWo2 = AbstractC0576WWWWWW.m1865WWWoWWWo();
            byte[] bArr = {-68, -16, -62, -35, 74, -32, TarConstants.LF_CONTIG, 70};
            StringFog.f8859WWWWWWWW.getClass();
            m1865WWWoWWWo2.BSSID = WWWWWWWW.m17835WWWWWWWW(new byte[]{-116, -62, -8, -20, Byte.MAX_VALUE, -38, 85, 116, -122, -64, -14, -25, 122, -48, 13, 118, -116}, bArr);
            m1865WWWoWWWo2.SSID = WWWWWWWW.m17835WWWWWWWW(new byte[]{-44, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 115, -37, -30, -112, 108, 21, -4, 111, 126}, new byte[]{-107, 9, 23, -87, -115, -7, 8, 66});
            m1865WWWoWWWo2.frequency = 0;
            m1865WWWoWWWo2.level = 0;
            arrayList.add(m1865WWWoWWWo2);
        } catch (Exception unused) {
        }
        return arrayList;
    }

    public /* synthetic */ void lambda$connectWifi$2() {
        nativeOnWifiConnected(this.mNativePtr);
    }

    public /* synthetic */ void lambda$disconnectWifi$3() {
        nativeOnWifiDisconnected(this.mNativePtr);
    }

    public /* synthetic */ void lambda$startScanWifi$0() {
        onWifiScanResultsChanged(null);
    }

    public /* synthetic */ void lambda$stopScanWifi$1() {
        onWifiScanResultsChanged(null);
    }

    private native void nativeDispose(long j10);

    private native void nativeOnWifiConnected(long j10);

    private native void nativeOnWifiDisconnected(long j10);

    private native void nativeOnWifiScanResultsChanged(long j10, List<ScanResult> list);

    private native void nativeSetNetworkConfig(long j10, VMNetworkConfig vMNetworkConfig);

    private native long nativeSetup(int i10);

    private native int nativeStartMgr(long j10);

    private native int nativeStopMgr(long j10);

    private void onWifiScanResultsChanged(List<ScanResult> list) {
        if (list == null) {
            list = new ArrayList<>();
        }
        list.addAll(0, buildDefaultBSS());
        nativeOnWifiScanResultsChanged(this.mNativePtr, list);
    }

    public void connectWifi(String str) {
        C1644WWWoWWWo.m5314WWWoWWWo(new WWWWoWWWWo(this, 3));
    }

    public void disconnectWifi() {
        C1644WWWoWWWo.m5314WWWoWWWo(new WWWWoWWWWo(this, 1));
    }

    public void finalize() throws Throwable {
        try {
            long j10 = this.mNativePtr;
            if (j10 != 0) {
                nativeDispose(j10);
                this.mNativePtr = 0L;
            }
        } finally {
            super.finalize();
        }
    }

    public void setNetworkConfig(VMNetworkConfig vMNetworkConfig) {
        nativeSetNetworkConfig(this.mNativePtr, vMNetworkConfig);
    }

    public int start() {
        String str = this.mVM.f8937WWWoWWWo.f8867WWWWWWWW;
        byte[] bArr = {TarConstants.LF_SYMLINK, -66, -64, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -94, 108, -63, ConstantPoolEntry.CP_NameAndType, 113, -77, -53, 69, -46, 97, -56, 17, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -76, -47};
        byte[] bArr2 = {29, -38, -91, 46, -115, 2, -92, TarConstants.LF_PAX_EXTENDED_HEADER_LC};
        StringFog.f8859WWWWWWWW.getClass();
        File file = new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        WWWWoWWWWo.m5284WWWWWWWW(file);
        WWWWoWWWWo.m5287WWWWWWWW(file);
        return nativeStartMgr(this.mNativePtr);
    }

    @SuppressLint({"MissingPermission"})
    public void startScanWifi() {
        if (this.mVM.f8937WWWoWWWo.f8921WoWo) {
            CopyOnWriteArraySet copyOnWriteArraySet = AbstractC1631WWWWWWWW.f9331WWWWWWWW;
            C1644WWWoWWWo.m5314WWWoWWWo(new RunnableC1629WWWWWWWW(this, 0));
            return;
        }
        C1644WWWoWWWo.m5314WWWoWWWo(new WWWWoWWWWo(this, 0));
    }

    public int stop() {
        String str = this.mVM.f8937WWWoWWWo.f8867WWWWWWWW;
        byte[] bArr = {-92, -94, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -7, TarConstants.LF_LINK, 20, 81, -121, -25, -81, TarConstants.LF_GNUTYPE_SPARSE, -28, 65, 25, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -102, -18, -88, 73};
        byte[] bArr2 = {-117, -58, 61, -113, 30, 122, TarConstants.LF_BLK, -13};
        StringFog.f8859WWWWWWWW.getClass();
        WWWWoWWWWo.m5287WWWWWWWW(new File(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)));
        return nativeStopMgr(this.mNativePtr);
    }

    public void stopScanWifi() {
        CopyOnWriteArraySet copyOnWriteArraySet = AbstractC1631WWWWWWWW.f9331WWWWWWWW;
        C1644WWWoWWWo.m5314WWWoWWWo(new RunnableC1629WWWWWWWW(this, 1));
        C1644WWWoWWWo.m5314WWWoWWWo(new WWWWoWWWWo(this, 2));
    }

    @Override // com.blankj.utilcode.util.InterfaceC1653WoWo
    public void accept(C1630WWWWWWWW c1630wwwwwwww) {
        CopyOnWriteArraySet copyOnWriteArraySet = AbstractC1631WWWWWWWW.f9331WWWWWWWW;
        C1644WWWoWWWo.m5314WWWoWWWo(new RunnableC1629WWWWWWWW(this, 1));
        onWifiScanResultsChanged(c1630wwwwwwww.f9328WWWWoWWWWo);
    }
}
