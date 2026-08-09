package com.android.vmapp.ui.vm.rom;

import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import androidx.appcompat.app.C0791WWWWWWWW;
import androidx.appcompat.app.WWWW;
import androidx.fragment.app.C1023WWoWWo;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import bd.C1406WWoWWo;
import com.android.vmapp.rom.Asset;
import com.android.vmapp.rom.RomModel;
import com.android.vmapp.ui.widget.CommonEmptyView;
import com.clone.android.dual.space.R;
import e4.DialogInterface$OnClickListenerC2340WWWWWWWW;
import ed.AbstractC2403WWWWoWWWWo;
import gc.C2597WWWWWWWW;
import gc.C2609WWoWWo;
import gc.C2612WWoWWo;
import j3.C3164WWWWWWWW;
import java.util.ArrayList;
import java.util.List;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import n4.C3540WWWWWWWW;
import n4.C3543WWWWWWWW;
import n4.C3549WWoWWo;
import n4.WWWoWWWo;
import n4.WWoWWo;
import n4.WoWo;
import o3.WWWWoWWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p013WWWWWWWW.o;
import ta.C4248WWWoWWWo;
import v3.WWWWWWWW;
/* loaded from: classes.dex */
public final class RomFragment extends WWWWWWWW {

    /* renamed from: WWWWỖWWWWࢥỖ  reason: contains not printable characters */
    public RecyclerView f8737WWWWWWWW;

    /* renamed from: WWWWίWWWWРί  reason: contains not printable characters */
    public WoWo f8738WWWWWWWW;

    /* renamed from: WWWWℯWWWW၂ℯ  reason: contains not printable characters */
    public CommonEmptyView f8739WWWWWWWW;

    /* renamed from: a  reason: collision with root package name */
    public final o f36455a = new o(C3333WWWWoWWWWo.m15421WWWWWWWW(C3543WWWWWWWW.class), new C3540WWWWWWWW(this, 0), new C3540WWWWWWWW(this, 2), new C3540WWWWWWWW(this, 1));

    /* renamed from: b  reason: collision with root package name */
    public final C1023WWoWWo f36456b = WWWWoWWWWo.m16216WWWWoWWWWo(this, new p009WWWWWWWW.WWWWoWWWWo());

    /* renamed from: c  reason: collision with root package name */
    public WWWoWWWo f36457c;

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWᐡWWWWೱᐡ */
    public final View mo2047WWWWWWWW(LayoutInflater layoutInflater, ViewGroup viewGroup, Bundle bundle) {
        byte[] bArr = {-76, 89, TarConstants.LF_LINK, -126, 61, -75, 100, -44};
        byte[] bArr2 = {-35, TarConstants.LF_CONTIG, 87, -18, 92, -63, 1, -90};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(layoutInflater, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        View inflate = layoutInflater.inflate(R.layout.fragment_rom, viewGroup, false);
        View findViewById = inflate.findViewById(R.id.listview);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{114, -101, 15, -55, 66, -20, 121, 64, 86, -117, 40, -55, 60, -85, TarConstants.LF_SYMLINK, 25, 61}, new byte[]{20, -14, 97, -83, 20, -123, 28, TarConstants.LF_CONTIG}));
        this.f8737WWWWWWWW = (RecyclerView) findViewById;
        m3287WWoWWo();
        LinearLayoutManager linearLayoutManager = new LinearLayoutManager(1);
        RecyclerView recyclerView = this.f8737WWWWWWWW;
        if (recyclerView != null) {
            recyclerView.setLayoutManager(linearLayoutManager);
            WoWo woWo = new WoWo(this);
            this.f8738WWWWWWWW = woWo;
            RecyclerView recyclerView2 = this.f8737WWWWWWWW;
            if (recyclerView2 != null) {
                recyclerView2.setAdapter(woWo);
                View findViewById2 = inflate.findViewById(R.id.emptyView);
                AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-109, -77, 24, -6, 109, -72, -83, -90, -73, -93, 63, -6, 19, -1, -26, -1, -36}, new byte[]{-11, -38, 118, -98, 59, -47, -56, -47}));
                this.f8739WWWWWWWW = (CommonEmptyView) findViewById2;
                return inflate;
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{41, -126, 24, -111, -17, 79, 26, 23, TarConstants.LF_FIFO, -122, 20, -105, -31}, new byte[]{68, -48, 125, -14, -106, 44, 118, 114}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-115, -60, 58, -74, 44, -77, -118, 40, -110, -64, TarConstants.LF_FIFO, -80, 34}, new byte[]{-32, -106, 95, -43, 85, -48, -26, TarConstants.LF_MULTIVOLUME}));
        throw null;
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWᢎWWWWယᢎ */
    public final void mo2048WWWWWWWW(Bundle bundle, View view) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(view, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-69, 111, 9, -7}, new byte[]{-51, 6, 108, -114, 122, -106, 8, 78}));
        AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(ib.WWWWoWWWWo.m14598WWWWWWWW(this), null, new WWoWWo(this, null), 3);
    }

    /* renamed from: WWWWệWWWW֙ệ  reason: contains not printable characters */
    public final void m5000WWWWWWWW(View view, C3549WWoWWo c3549WWoWWo) {
        try {
            C1023WWoWWo c1023WWoWWo = this.f36456b;
            byte[] bArr = {16, -82, -35, -73, -102, -56, TarConstants.LF_SYMLINK, -81};
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            WWWWoWWWWo.m16217WWWWWWWW(c1023WWoWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{58, -127, -9}, bArr), new fd.WWWoWWWo(2, this, c3549WWoWWo));
        } catch (Exception unused) {
            C4248WWWoWWWo m17235WWWWWWWW = C4248WWWoWWWo.m17235WWWWWWWW(view, R.string.open_file_browser_failed_tips, -1);
            m17235WWWWWWWW.m17231WWWWWWWW(view);
            m17235WWWWWWWW.m17237WWWWWWWW();
        }
    }

    /* renamed from: WWWWỖWWWWࢥỖ  reason: contains not printable characters */
    public final void m5001WWWWWWWW(C3549WWoWWo c3549WWoWWo) {
        da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(m3287WWoWWo());
        wWWWoWWWWo.m13648WoWo(R.string.download_dialog_title_cancel);
        wWWWoWWWWo.m13642WWWWWWWW(R.string.download_dialog_msg_cancel);
        wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_dismiss, null);
        wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_cancel, new n4.WWWWoWWWWo(this, c3549WWoWWo, 0));
        WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-50, 43, -37, -55, 80, 60, -84, -14, -125, 119, -105}, new byte[]{-83, 89, -66, -88, 36, 89, -124, -36});
        mo742WWWW.show();
    }

    /* renamed from: WWWWίWWWWРί  reason: contains not printable characters */
    public final void m5002WWWWWWWW(View view, C3549WWoWWo c3549WWoWWo, Asset asset) {
        da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(m3287WWoWWo());
        wWWWoWWWWo.m13648WoWo(R.string.download_dialog_title_download);
        wWWWoWWWWo.m13642WWWWWWWW(R.string.download_dialog_msg_download_or_import);
        wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_download, new DialogInterface$OnClickListenerC2340WWWWWWWW(this, asset, view, 2));
        wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_import, new n4.WWWWWWWW(this, c3549WWoWWo, view));
        WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
        byte[] bArr = {123, -36, 107, 112, -35, -127, 112, -81, TarConstants.LF_FIFO, Byte.MIN_VALUE, 39};
        byte[] bArr2 = {24, -82, 14, 17, -87, -28, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -127};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
        mo742WWWW.show();
    }

    /* renamed from: WWWWℯWWWW၂ℯ  reason: contains not printable characters */
    public final void m5003WWWWWWWW(C3549WWoWWo c3549WWoWWo, Asset asset) {
        da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(m3287WWoWWo());
        wWWWoWWWWo.m13648WoWo(R.string.download_dialog_title_download);
        ((C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW).f3561WWoWWo = m3294WoWo(new Object[]{com.blankj.utilcode.util.WWWW.m5321WWWWWWWW(0, asset.getSize()).toString()}, R.string.download_dialog_msg_download);
        wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_dismiss, null);
        wWWWoWWWWo.m13645WWWoWWWo(R.string.download_dialog_title_download, new DialogInterface$OnClickListenerC2340WWWWWWWW(this, c3549WWoWWo, asset, 3));
        WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-63, 34, -74, 115, 26, 108, -47, 36, -116, 126, -6}, new byte[]{-94, 80, -45, 18, 110, 9, -7, 10});
        mo742WWWW.show();
    }

    /* renamed from: WWWoễWWWoಇễ  reason: contains not printable characters */
    public final C3543WWWWWWWW m5004WWWoWWWo() {
        return (C3543WWWWWWWW) this.f36455a.getValue();
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWᏛWWW෮Ꮫ */
    public final void mo471WWWWWW(Bundle bundle) {
        super.mo471WWWWWW(bundle);
        Bundle bundle2 = this.f5304WWoWWo;
        if (bundle2 != null) {
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            String string = bundle2.getString(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{56, -30, 15, -101, -70, 125}, new byte[]{74, -115, 98, -60, -45, 25, 10, -11}));
            if (string != null) {
                m5004WWWoWWWo().f31160WWWWWWWW = new C1406WWoWWo(string, 1);
            }
        }
    }

    /* renamed from: WWWếWWW෨ế  reason: contains not printable characters */
    public final void m5005WWWWWW(View view, C3549WWoWWo c3549WWoWWo) {
        String m3294WoWo;
        int size = c3549WWoWWo.f31192WWWWWWWW.getRom_asset().size();
        RomModel romModel = c3549WWoWWo.f31192WWWWWWWW;
        if (size > 1) {
            da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(m3287WWoWWo());
            List<Asset> rom_asset = romModel.getRom_asset();
            ArrayList arrayList = new ArrayList(C2597WWWWWWWW.m14141WWWoWWWo(rom_asset, 10));
            int i10 = 0;
            for (Object obj : rom_asset) {
                int i11 = i10 + 1;
                if (i10 >= 0) {
                    m5004WWWoWWWo();
                    if (C3543WWWWWWWW.m16034WWWWWWWW((Asset) obj)) {
                        m3294WoWo = m3290WW(R.string.rom_download_internal);
                    } else {
                        m3294WoWo = m3294WoWo(new Object[]{Integer.valueOf(i10)}, R.string.rom_download_external);
                    }
                    arrayList.add(m3294WoWo);
                    i10 = i11;
                } else {
                    C2609WWoWWo.m14151WWWWWWWW();
                    throw null;
                }
            }
            n4.WWWWWWWW wwwwwwww = new n4.WWWWWWWW(c3549WWoWWo, this, view);
            C0791WWWWWWWW c0791wwwwwwww = (C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW;
            c0791wwwwwwww.f3559WWWoWWWo = (String[]) arrayList.toArray(new String[0]);
            c0791wwwwwwww.f3555WWWWWWWW = wwwwwwww;
            WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
            byte[] bArr = {22, -47, 121, 30, 2, 39, 36, -58, 91, -115, TarConstants.LF_DIR};
            byte[] bArr2 = {117, -93, 28, Byte.MAX_VALUE, 118, 66, ConstantPoolEntry.CP_NameAndType, -24};
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
            mo742WWWW.show();
            return;
        }
        Asset asset = (Asset) C2612WWoWWo.m14162WWWWWWWW(0, romModel.getRom_asset());
        if (asset != null) {
            m5004WWWoWWWo();
            if (C3543WWWWWWWW.m16034WWWWWWWW(asset)) {
                m5003WWWWWWWW(c3549WWoWWo, asset);
            } else {
                m5002WWWWWWWW(view, c3549WWoWWo, asset);
            }
        }
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWᐤԂᐤ */
    public final void mo3292WW() {
        this.f5273WWWWoWWWWo = true;
    }

    public final void a(String str) {
        da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(m3287WWoWWo());
        ((C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW).f3561WWoWWo = str;
        wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_got_it, null);
        WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{2, 92, 116, -26, 119, -44, -119, 62, 79, 0, 56}, new byte[]{97, 46, 17, -121, 3, -79, -95, 16});
        mo742WWWW.show();
    }

    /* renamed from: oỈɨỈ  reason: contains not printable characters */
    public final void m5006o(WWWoWWWo wWWoWWWo) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{18, -23, 126, -104, 90, 73, -71, 13}, new byte[]{126, Byte.MIN_VALUE, 13, -20, 63, 39, -36, Byte.MAX_VALUE});
        this.f36457c = wWWoWWWo;
    }
}
