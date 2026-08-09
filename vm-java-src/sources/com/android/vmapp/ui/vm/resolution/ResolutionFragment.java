package com.android.vmapp.ui.vm.resolution;

import a3.WWWoWWWo;
import android.app.Application;
import android.os.Bundle;
import android.text.TextUtils;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import com.android.vmcore.VMResConfig;
import com.blankj.utilcode.util.WWWW;
import com.clone.android.dual.space.R;
import com.google.android.gms.internal.measurement.p1;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import i6.C2899WWWWWWWW;
import j3.C3164WWWWWWWW;
import java.io.File;
import java.util.ArrayList;
import java.util.regex.Pattern;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;
/* loaded from: classes.dex */
public class ResolutionFragment extends v3.WWWWWWWW {

    /* renamed from: WWWWỖWWWWࢥỖ  reason: contains not printable characters */
    public RecyclerView f8728WWWWWWWW;

    /* renamed from: WWWWίWWWWРί  reason: contains not printable characters */
    public View f8729WWWWWWWW;

    /* renamed from: WWWWℯWWWW၂ℯ  reason: contains not printable characters */
    public ArrayList f8730WWWWWWWW;

    /* renamed from: a  reason: collision with root package name */
    public WWWWoWWWWo f36453a;

    /* renamed from: b  reason: collision with root package name */
    public com.android.vmapp.ui.vm.resolution.WWWWWWWW f36454b;

    /* renamed from: com.android.vmapp.ui.vm.resolution.ResolutionFragment$WWWW̏WWWWβ̏  reason: invalid class name */
    /* loaded from: classes.dex */
    public static class WWWWWWWW {

        /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
        public String f8731WWWWoWWWWo;

        /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
        public VMResConfig f8732WWWWWWWW;

        /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
        public boolean f8733WWWWWWWW;

        /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
        public boolean f8734WWWWWWWW;

        /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
        public String f8735WWWoWWWo;
    }

    /* renamed from: WWWếWWW෨ế  reason: contains not printable characters */
    public static boolean m4999WWWWWW(String str) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        return !Pattern.compile(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-42, -122, -75, -84, 3, 20}, new byte[]{-115, -74, -104, -107, 94, 62, 66, -28})).matcher(str).matches();
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWᐡWWWWೱᐡ */
    public final View mo2047WWWWWWWW(LayoutInflater layoutInflater, ViewGroup viewGroup, Bundle bundle) {
        View inflate = layoutInflater.inflate(R.layout.fragment_resolution, viewGroup, false);
        RecyclerView recyclerView = (RecyclerView) inflate.findViewById(R.id.list);
        this.f8728WWWWWWWW = recyclerView;
        m3287WWoWWo();
        recyclerView.setLayoutManager(new LinearLayoutManager(1));
        View findViewById = inflate.findViewById(R.id.add);
        this.f8729WWWWWWWW = findViewById;
        findViewById.setOnClickListener(new WWWoWWWo(13, this));
        return inflate;
    }

    /* JADX WARN: Removed duplicated region for block: B:28:0x019d  */
    /* JADX WARN: Removed duplicated region for block: B:51:0x03c8  */
    /* JADX WARN: Removed duplicated region for block: B:67:? A[RETURN, SYNTHETIC] */
    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWᢎWWWWယᢎ */
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final void mo2048WWWWWWWW(Bundle bundle, View view) {
        String str;
        int size;
        int i10;
        JSONArray jSONArray;
        Bundle bundle2 = this.f5304WWoWWo;
        if (bundle2 != null) {
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            str = bundle2.getString(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-3, -88, -10, 101, -26, -45, TarConstants.LF_NORMAL, -18, -20, -72, -9, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -17, -56, TarConstants.LF_NORMAL, -40, -15, -77}, new byte[]{-98, -35, -124, 23, -125, -67, 68, -79}));
        } else {
            str = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        byte[] bArr = {109, -113, 41, -50, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 123, 38, -76, 118, -121, 62, -56};
        byte[] bArr2 = {0, -18, 93, -83, TarConstants.LF_NORMAL, 36, 66, -47};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        if (x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2).equals(str)) {
            str = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-48, 126, -40, -51, 71, 97, -101, -119, -37, 105, -38, -42, 69, 109, -80}, new byte[]{-76, 27, -82, -92, 36, 4, -60, -7});
        }
        ArrayList arrayList = new ArrayList();
        Application m5336WWWoWWWo = WWWW.m5336WWWoWWWo();
        ArrayList arrayList2 = new ArrayList();
        arrayList2.add(p1.m11594WWWoWWWo(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-126, 90, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -116, 21}, new byte[]{-74, 98, 104, -4, 39, 31, 17, -76})));
        arrayList2.add(p1.m11594WWWoWWWo(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, -32, TarConstants.LF_MULTIVOLUME, 79}, new byte[]{TarConstants.LF_MULTIVOLUME, -44, 125, 63, 92, -88, -114, -35})));
        arrayList2.add(p1.m11594WWWoWWWo(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-93, 26, 66, -122}, new byte[]{-108, 40, 114, -10, -110, -120, 8, TarConstants.LF_PAX_EXTENDED_HEADER_LC})));
        arrayList2.add(p1.m11594WWWoWWWo(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{9, TarConstants.LF_GNUTYPE_SPARSE, -15, -105, 98}, new byte[]{56, 99, -55, -89, 18, -69, 41, 92})));
        arrayList2.add(p1.m11594WWWoWWWo(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-24, 121, 89, 41, 61, -50, -94, TarConstants.LF_BLK, -29, 110, 91, TarConstants.LF_SYMLINK, 63, -62, -119}, new byte[]{-116, 28, 47, 64, 94, -85, -3, 68})));
        arrayList2.add(p1.m11594WWWoWWWo(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{15, -48, -60, -69, 57, 125, -100, 42, 10, -37, -42, -95, 57, 121, -77, 35}, new byte[]{107, -75, -78, -46, 90, 24, -61, 70})));
        File fileStreamPath = m5336WWWoWWWo.getFileStreamPath(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-31, 81, -24, 30, TarConstants.LF_CHR, -30, -66, 6, -30, 72, -34, 3, 56, -50, -67, 3, -28, 72}, new byte[]{-105, 60, -73, 108, 86, -111, -47, 106}));
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(fileStreamPath, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-59, -111, 3, Byte.MAX_VALUE, 31, -18, -109, -111, -42, -122, 18, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 27, -46, -105, -74, -54, -36, 89, 23, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -85}, new byte[]{-94, -12, 119, 57, 118, -126, -10, -62}));
        try {
            String m5318WWWWWWWWWW = WWWW.m5318WWWWWWWWWW(fileStreamPath);
            if (TextUtils.isEmpty(m5318WWWWWWWWWW)) {
                jSONArray = new JSONArray();
            } else {
                jSONArray = new JSONArray(m5318WWWWWWWWWW);
            }
            int length = jSONArray.length();
            for (int i11 = 0; i11 < length; i11++) {
                JSONObject jSONObject = jSONArray.getJSONObject(i11);
                VMResConfig vMResConfig = new VMResConfig();
                try {
                    byte[] bArr3 = {TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -45, -107, -126, 94, -122, -77, 80};
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    vMResConfig.f8953WWWWWWWW = jSONObject.getString(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{9, -78, -8, -25}, bArr3));
                    vMResConfig.f8952WWWWoWWWWo = jSONObject.getInt(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-6, -55, -109, -54, -24}, new byte[]{-115, -96, -9, -66, Byte.MIN_VALUE, -1, -85, 23}));
                    vMResConfig.f8955WWWoWWWo = jSONObject.getInt(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-86, -4, -58, -127, 105, -81}, new byte[]{-62, -103, -81, -26, 1, -37, 70, 106}));
                    vMResConfig.f8954WWWWWWWW = jSONObject.getInt(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{123, 124, -27}, new byte[]{31, ConstantPoolEntry.CP_NameAndType, -116, 82, -37, 0, -33, 119}));
                    arrayList2.add(vMResConfig);
                } catch (JSONException e10) {
                    e = e10;
                    e.printStackTrace();
                    size = arrayList2.size();
                    i10 = 0;
                    while (i10 < size) {
                    }
                    this.f8730WWWWWWWW = arrayList;
                    WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(arrayList);
                    this.f36453a = wWWWoWWWWo;
                    this.f8728WWWWWWWW.setAdapter(wWWWoWWWWo);
                    this.f36453a.f35026WWWWWWWW = new C2899WWWWWWWW(6, this);
                    if (this.f36454b == null) {
                    }
                }
            }
        } catch (JSONException e11) {
            e = e11;
        }
        size = arrayList2.size();
        i10 = 0;
        while (i10 < size) {
            Object obj = arrayList2.get(i10);
            i10++;
            VMResConfig vMResConfig2 = (VMResConfig) obj;
            WWWWWWWW wwwwwwww = new WWWWWWWW();
            String str2 = vMResConfig2.f8953WWWWWWWW;
            if (!AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, 87, -23, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, 111, -39, 60, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 37, -106, 98}, str2) && !vMResConfig2.f8953WWWWWWWW.equals(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-70, 102, -92, -123, 0}, new byte[]{-114, 94, -108, -11, TarConstants.LF_SYMLINK, 40, 81, 65})) && !vMResConfig2.f8953WWWWWWWW.equals(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-100, -45, 47, 59}, new byte[]{-87, -25, 31, TarConstants.LF_GNUTYPE_LONGLINK, 33, -105, 117, -122})) && !vMResConfig2.f8953WWWWWWWW.equals(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{86, -54, -31, 105}, new byte[]{97, -8, -47, 25, 87, 108, 114, -115})) && !vMResConfig2.f8953WWWWWWWW.equals(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-124, 96, -102, 101, 29}, new byte[]{-75, 80, -94, 85, 109, 107, 124, -104}))) {
                if (vMResConfig2.f8953WWWWWWWW.equals(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_BLK, 26, TarConstants.LF_CONTIG, 108, -20, 31, 114, 60, 63, 13, TarConstants.LF_DIR, 119, -18, 19, 89}, new byte[]{80, Byte.MAX_VALUE, 65, 5, -113, 122, 45, TarConstants.LF_GNUTYPE_LONGNAME}))) {
                    wwwwwwww.f8732WWWWWWWW = vMResConfig2;
                    wwwwwwww.f8731WWWWoWWWWo = m3281WWWoWWWo().getString(R.string.vm_settings_resolution_device_portrait);
                    wwwwwwww.f8735WWWoWWWo = vMResConfig2.f8952WWWWoWWWWo + x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-105, -30, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{-73, -70, 108, -106, -54, 36, -54, -107}) + vMResConfig2.f8955WWWoWWWo + x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-101, -63, 123, -33, -119}, new byte[]{-69, -123, 43, -106, -87, 111, -37, 70}) + vMResConfig2.f8954WWWWWWWW;
                    wwwwwwww.f8733WWWWWWWW = false;
                    wwwwwwww.f8734WWWWWWWW = vMResConfig2.f8953WWWWWWWW.equals(str);
                } else if (vMResConfig2.f8953WWWWWWWW.equals(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-116, -107, -4, -69, -122, -9, 21, 113, -119, -98, -18, -95, -122, -13, 58, TarConstants.LF_PAX_EXTENDED_HEADER_LC}, new byte[]{-24, -16, -118, -46, -27, -110, 74, 29}))) {
                    wwwwwwww.f8732WWWWWWWW = vMResConfig2;
                    wwwwwwww.f8731WWWWoWWWWo = m3281WWWoWWWo().getString(R.string.vm_settings_resolution_device_landscape);
                    wwwwwwww.f8735WWWoWWWo = vMResConfig2.f8952WWWWoWWWWo + x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{13, -124, -59}, new byte[]{45, -36, -27, -61, -34, -4, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 107}) + vMResConfig2.f8955WWWoWWWo + x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-59, -88, 71, 22, TarConstants.LF_PAX_EXTENDED_HEADER_LC}, new byte[]{-27, -20, 23, 95, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -43, -81, -47}) + vMResConfig2.f8954WWWWWWWW;
                    wwwwwwww.f8733WWWWWWWW = false;
                    wwwwwwww.f8734WWWWWWWW = vMResConfig2.f8953WWWWWWWW.equals(str);
                } else {
                    wwwwwwww.f8732WWWWWWWW = vMResConfig2;
                    wwwwwwww.f8731WWWWoWWWWo = vMResConfig2.f8952WWWWoWWWWo + x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, -79, -97}, new byte[]{43, -23, -65, -59, -59, 24, -24, TarConstants.LF_NORMAL}) + vMResConfig2.f8955WWWoWWWo;
                    StringBuilder sb2 = new StringBuilder();
                    sb2.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-81, 25, -123, -49}, new byte[]{-21, 73, -52, -17, Byte.MIN_VALUE, 7, -32, 71}));
                    sb2.append(vMResConfig2.f8954WWWWWWWW);
                    wwwwwwww.f8735WWWoWWWo = sb2.toString();
                    wwwwwwww.f8733WWWWWWWW = true;
                    wwwwwwww.f8734WWWWWWWW = vMResConfig2.f8953WWWWWWWW.equals(str);
                }
            } else {
                wwwwwwww.f8732WWWWWWWW = vMResConfig2;
                wwwwwwww.f8731WWWWoWWWWo = vMResConfig2.f8952WWWWoWWWWo + x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-65, -59, TarConstants.LF_MULTIVOLUME}, new byte[]{-97, -99, 109, -120, -105, 97, 19, -93}) + vMResConfig2.f8955WWWoWWWo;
                StringBuilder sb3 = new StringBuilder();
                sb3.append(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_BLK, -122, -36, -15}, new byte[]{112, -42, -107, -47, -96, 64, 35, -114}));
                sb3.append(vMResConfig2.f8954WWWWWWWW);
                wwwwwwww.f8735WWWoWWWo = sb3.toString();
                wwwwwwww.f8733WWWWWWWW = false;
                wwwwwwww.f8734WWWWWWWW = vMResConfig2.f8953WWWWWWWW.equals(str);
            }
            arrayList.add(wwwwwwww);
        }
        this.f8730WWWWWWWW = arrayList;
        WWWWoWWWWo wWWWoWWWWo2 = new WWWWoWWWWo(arrayList);
        this.f36453a = wWWWoWWWWo2;
        this.f8728WWWWWWWW.setAdapter(wWWWoWWWWo2);
        this.f36453a.f35026WWWWWWWW = new C2899WWWWWWWW(6, this);
        if (this.f36454b == null) {
            ArrayList arrayList3 = this.f8730WWWWWWWW;
            int size2 = arrayList3.size();
            int i12 = 0;
            while (i12 < size2) {
                Object obj2 = arrayList3.get(i12);
                i12++;
                WWWWWWWW wwwwwwww2 = (WWWWWWWW) obj2;
                if (wwwwwwww2.f8734WWWWWWWW) {
                    this.f36454b.mo4964WWWWWWWW(wwwwwwww2, false);
                    return;
                }
            }
        }
    }
}
