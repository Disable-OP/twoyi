package com.android.vmapp.ui.vm.privacy;

import android.content.Intent;
import android.content.res.Resources;
import android.content.res.TypedArray;
import android.graphics.PorterDuff;
import android.graphics.PorterDuffColorFilter;
import android.graphics.drawable.Drawable;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.Menu;
import android.view.MenuInflater;
import android.view.MenuItem;
import android.view.View;
import android.view.ViewGroup;
import androidx.appcompat.app.AppCompatActivity;
import androidx.appcompat.widget.Toolbar;
import androidx.datastore.preferences.protobuf.C0962WWWoWWWo;
import androidx.fragment.app.FragmentActivity;
import androidx.lifecycle.AbstractC1071WWWWWWWW;
import androidx.recyclerview.widget.C1155WWWWWWWW;
import androidx.recyclerview.widget.C1187WWoWWo;
import androidx.recyclerview.widget.C1192WWoWWo;
import androidx.recyclerview.widget.GridLayoutManager;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import c0.C1458WWWW;
import com.android.vmapp.ui.vm.privacy.WWWWoWWWWo;
import com.android.vmapp.ui.vm.settings.VMSettingsActivity;
import com.android.vmapp.ui.widget.CommonEmptyView;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.PermissionEvent;
import com.android.vmcore.hal.AudioService;
import com.clone.android.dual.space.R;
import e4.C2344WWWWWWWW;
import ed.AbstractC2403WWWWoWWWWo;
import ed.C2427WWWWWWWW;
import fc.C2520WWWWWWWW;
import fc.EnumC2528WWWoWWWo;
import fc.InterfaceC2519WWWWWWWW;
import hd.C2819WWWWWWWW;
import i4.WWWWWWWWWW;
import j3.C3164WWWWWWWW;
import java.util.ArrayList;
import java.util.List;
import k4.C3235WWWWWWWW;
import k4.C3245WWWoWWWo;
import k4.C3248WWoWWo;
import k4.InterfaceC3250WWoWWo;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import l3.C3401WWWWWWWW;
import l3.C3403WWWWWWWW;
import l4.WWWW;
import ld.C3455WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p013WWWWWWWW.o;
import p021WWWWWWWW.AbstractC0264WWWWWWWW;
import p029WWWWWWWW.WWWoWWWo;
import ta.C4248WWWoWWWo;
import v3.WWWWWWWW;
/* loaded from: classes.dex */
public final class PrivacyMainFragment extends WWWWWWWW implements InterfaceC3250WWoWWo {

    /* renamed from: WWWWỖWWWWࢥỖ  reason: contains not printable characters */
    public RecyclerView f8705WWWWWWWW;

    /* renamed from: WWWWίWWWWРί  reason: contains not printable characters */
    public CommonEmptyView f8706WWWWWWWW;

    /* renamed from: WWWWℯWWWW၂ℯ  reason: contains not printable characters */
    public k4.WWWWWWWW f8707WWWWWWWW;

    /* renamed from: a  reason: collision with root package name */
    public C3245WWWoWWWo f36444a;

    /* renamed from: b  reason: collision with root package name */
    public C1192WWoWWo f36445b;

    /* renamed from: c  reason: collision with root package name */
    public C1155WWWWWWWW f36446c;

    /* renamed from: d  reason: collision with root package name */
    public Menu f36447d;

    /* renamed from: e  reason: collision with root package name */
    public int f36448e;

    /* renamed from: f  reason: collision with root package name */
    public int f36449f;

    /* renamed from: g  reason: collision with root package name */
    public int f36450g;

    /* renamed from: h  reason: collision with root package name */
    public WWWWoWWWWo.WWWWWWWW f36451h;

    /* renamed from: i  reason: collision with root package name */
    public final o f36452i;

    public PrivacyMainFragment() {
        C1458WWWW c1458wwww = new C1458WWWW(12);
        InterfaceC2519WWWWWWWW m13999WWWWWWWW = C2520WWWWWWWW.m13999WWWWWWWW(EnumC2528WWWoWWWo.f27073WWWWWWWWWW, new WWWoWWWo(12, new WWWoWWWo(11, this)));
        this.f36452i = new o(C3333WWWWoWWWWo.m15421WWWWWWWW(WWWWoWWWWo.class), new WWWWWWWWWW(m13999WWWWWWWW, 4), c1458wwww, new WWWWWWWWWW(m13999WWWWWWWW, 5));
    }

    /* renamed from: WWWếWWW෨ế  reason: contains not printable characters */
    public static final void m4996WWWWWW(PrivacyMainFragment privacyMainFragment, int i10) {
        C1192WWoWWo c1192WWoWWo = privacyMainFragment.f36445b;
        if (c1192WWoWWo != null) {
            c1192WWoWWo.m3961WWWWoWWWWo(null);
            C1155WWWWWWWW c1155wwwwwwww = privacyMainFragment.f36446c;
            if (c1155wwwwwwww != null) {
                c1155wwwwwwww.m3919WWWoWWWo(null);
                RecyclerView recyclerView = privacyMainFragment.f8705WWWWWWWW;
                if (recyclerView != null) {
                    ArrayList arrayList = recyclerView.f36400b;
                    if (arrayList != null) {
                        arrayList.clear();
                    }
                    RecyclerView recyclerView2 = privacyMainFragment.f8705WWWWWWWW;
                    if (recyclerView2 != null) {
                        recyclerView2.setOnFlingListener(null);
                        RecyclerView recyclerView3 = privacyMainFragment.f8705WWWWWWWW;
                        if (recyclerView3 != null) {
                            int itemDecorationCount = recyclerView3.getItemDecorationCount();
                            for (int i11 = 0; i11 < itemDecorationCount; i11++) {
                                RecyclerView recyclerView4 = privacyMainFragment.f8705WWWWWWWW;
                                if (recyclerView4 != null) {
                                    recyclerView4.m3751WWoWWo();
                                } else {
                                    byte[] bArr = {-105, ConstantPoolEntry.CP_NameAndType, 121, -71, -55, -83, -55, 79};
                                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-6, 94, 28, -38, -80, -50, -91, 42, -27, 90, 16, -36, -66}, bArr);
                                    throw null;
                                }
                            }
                            int i12 = i10 % 3;
                            if (i12 == 1) {
                                RecyclerView recyclerView5 = privacyMainFragment.f8705WWWWWWWW;
                                if (recyclerView5 != null) {
                                    recyclerView5.setAdapter(new C3235WWWWWWWW(privacyMainFragment, 2));
                                    RecyclerView recyclerView6 = privacyMainFragment.f8705WWWWWWWW;
                                    if (recyclerView6 != null) {
                                        RecyclerView.WWWWWWWW adapter = recyclerView6.getAdapter();
                                        byte[] bArr2 = {111, 62, 119, 118, TarConstants.LF_CONTIG, -117, -33, 68, 111, 36, 111, 58, 117, -115, -98, 73, 96, 56, 111, 58, 99, -121, -98, 68, 110, 37, TarConstants.LF_FIFO, 116, 98, -124, -46, 10, 117, TarConstants.LF_SYMLINK, 107, Byte.MAX_VALUE, TarConstants.LF_CONTIG, -117, -47, 71, 47, 42, 117, 126, 101, -121, -41, 78, 47, 61, 118, 123, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -104, -112, 95, 104, 101, 109, 119, 57, -123, -33, 67, 111, 101, 89, 123, 100, -115, -24, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 64, 47, 122, 106, 99, -115, -52, 22, 65, 16, 93, 118, 114, -112, -41, 72, 109, 46, 85, 111, 123, -124, -33, 72, 104, 39, 114, 110, 110, -75, -98, TarConstants.LF_GNUTYPE_LONGLINK, 111, 47, 105, 117, 126, -116, -58, 4, 115, 46, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 99, 116, -124, -37, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 119, 34, 126, 109, 57, -97, -41, 78, 102, 46, 111, TarConstants.LF_BLK, 69, -115, -35, TarConstants.LF_GNUTYPE_SPARSE, 98, 39, 126, 104, 65, -127, -37, 93, 47, 29, 114, Byte.MAX_VALUE, 96, -96, -47, 70, 101, 46, 105, 37, 41};
                                        byte[] bArr3 = {1, TarConstants.LF_GNUTYPE_LONGLINK, 27, 26, 23, -24, -66, 42};
                                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(adapter, x5.WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                                        privacyMainFragment.f8707WWWWWWWW = (k4.WWWWWWWW) adapter;
                                        privacyMainFragment.m3287WWoWWo();
                                        GridLayoutManager gridLayoutManager = new GridLayoutManager();
                                        RecyclerView recyclerView7 = privacyMainFragment.f8705WWWWWWWW;
                                        if (recyclerView7 != null) {
                                            recyclerView7.setLayoutManager(gridLayoutManager);
                                            RecyclerView recyclerView8 = privacyMainFragment.f8705WWWWWWWW;
                                            if (recyclerView8 != null) {
                                                recyclerView8.m3713WWWWWWWW(new p4.WWWWWWWW(1, privacyMainFragment.m3281WWWoWWWo().getDimensionPixelOffset(R.dimen.cat_card_grid_spacing_horizontal), privacyMainFragment.m3281WWWoWWWo().getDimensionPixelOffset(R.dimen.cat_list_horizontal_padding), privacyMainFragment.m3281WWWoWWWo().getDimensionPixelOffset(R.dimen.cat_card_grid_spacing_vertical), privacyMainFragment.m3281WWWoWWWo().getDimensionPixelOffset(R.dimen.cat_list_vertical_padding)));
                                            } else {
                                                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{28, -37, -53, 98, -49, -81, -113, 115, 3, -33, -57, 100, -63}, new byte[]{113, -119, -82, 1, -74, -52, -29, 22}));
                                                throw null;
                                            }
                                        } else {
                                            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-59, 32, -118, 67, 99, 57, 84, -47, -38, 36, -122, 69, 109}, new byte[]{-88, 114, -17, 32, 26, 90, 56, -76}));
                                            throw null;
                                        }
                                    } else {
                                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-17, 0, 47, -95, 116, -121, -76, 1, -16, 4, 35, -89, 122}, new byte[]{-126, 82, 74, -62, 13, -28, -40, 100});
                                        throw null;
                                    }
                                } else {
                                    byte[] bArr4 = {-103, -96, -72, -30, -26, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -86, 92};
                                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-12, -14, -35, -127, -97, 27, -58, 57, -21, -10, -47, -121, -111}, bArr4);
                                    throw null;
                                }
                            } else if (i12 == 2) {
                                RecyclerView recyclerView9 = privacyMainFragment.f8705WWWWWWWW;
                                if (recyclerView9 != null) {
                                    recyclerView9.setAdapter(new C3235WWWWWWWW(privacyMainFragment, 1));
                                    RecyclerView recyclerView10 = privacyMainFragment.f8705WWWWWWWW;
                                    if (recyclerView10 != null) {
                                        RecyclerView.WWWWWWWW adapter2 = recyclerView10.getAdapter();
                                        byte[] bArr5 = {-65, 44, -76, 64, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -54, 44, -52, -65, TarConstants.LF_FIFO, -84, ConstantPoolEntry.CP_NameAndType, 58, -52, 109, -63, -80, 42, -84, ConstantPoolEntry.CP_NameAndType, 44, -58, 109, -52, -66, TarConstants.LF_CONTIG, -11, 66, 45, -59, 33, -126, -91, 32, -88, 73, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -54, 34, -49, -1, 56, -74, 72, 42, -58, 36, -58, -1, 47, -75, TarConstants.LF_MULTIVOLUME, 40, -39, 99, -41, -72, 119, -82, 65, 118, -60, 44, -53, -65, 119, -102, TarConstants.LF_MULTIVOLUME, 43, -52, 27, -17, -112, 61, -71, 92, 44, -52, 63, -98, -111, 2, -98, 64, 61, -47, 36, -64, -67, 60, -106, 89, TarConstants.LF_BLK, -59, 44, -64, -72, TarConstants.LF_DIR, -79, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 33, -12, 109, -61, -65, 61, -86, 67, TarConstants.LF_LINK, -51, TarConstants.LF_DIR, -116, -93, 60, -69, 85, 59, -59, 40, -48, -89, TarConstants.LF_NORMAL, -67, 91, 118, -34, 36, -58, -74, 60, -84, 2, 10, -52, 46, -37, -78, TarConstants.LF_DIR, -67, 94, 14, -64, 40, -43, -1, 15, -79, 73, 47, -31, 34, -50, -75, 60, -86, 19, 102};
                                        byte[] bArr6 = {-47, 89, -40, 44, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -87, TarConstants.LF_MULTIVOLUME, -94};
                                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(adapter2, x5.WWWWWWWW.m17835WWWWWWWW(bArr5, bArr6));
                                        privacyMainFragment.f8707WWWWWWWW = (k4.WWWWWWWW) adapter2;
                                        privacyMainFragment.m3287WWoWWo();
                                        LinearLayoutManager linearLayoutManager = new LinearLayoutManager(1);
                                        RecyclerView recyclerView11 = privacyMainFragment.f8705WWWWWWWW;
                                        if (recyclerView11 != null) {
                                            recyclerView11.setLayoutManager(linearLayoutManager);
                                            RecyclerView recyclerView12 = privacyMainFragment.f8705WWWWWWWW;
                                            if (recyclerView12 != null) {
                                                recyclerView12.m3713WWWWWWWW(new C1187WWoWWo(privacyMainFragment.m3287WWoWWo()));
                                            } else {
                                                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-66, -63, 8, -6, -13, -1, -40, 32, -95, -59, 4, -4, -3}, new byte[]{-45, -109, 109, -103, -118, -100, -76, 69}));
                                                throw null;
                                            }
                                        } else {
                                            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{80, 25, 91, 22, -50, -57, -7, 33, 79, 29, 87, 16, -64}, new byte[]{61, TarConstants.LF_GNUTYPE_LONGLINK, 62, 117, -73, -92, -107, 68}));
                                            throw null;
                                        }
                                    } else {
                                        byte[] bArr7 = {-91, 84, ConstantPoolEntry.CP_NameAndType, 38, -88, TarConstants.LF_DIR, 84, -25};
                                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-56, 6, 105, 69, -47, 86, 56, -126, -41, 2, 101, 67, -33}, bArr7);
                                        throw null;
                                    }
                                } else {
                                    byte[] bArr8 = {-65, TarConstants.LF_GNUTYPE_LONGNAME, TarConstants.LF_DIR, -5, -93, TarConstants.LF_SYMLINK, 46, -14};
                                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-46, 30, 80, -104, -38, 81, 66, -105, -51, 26, 92, -98, -44}, bArr8);
                                    throw null;
                                }
                            } else {
                                RecyclerView recyclerView13 = privacyMainFragment.f8705WWWWWWWW;
                                if (recyclerView13 != null) {
                                    recyclerView13.setAdapter(new C3235WWWWWWWW(privacyMainFragment, 0));
                                    RecyclerView recyclerView14 = privacyMainFragment.f8705WWWWWWWW;
                                    if (recyclerView14 != null) {
                                        RecyclerView.WWWWWWWW adapter3 = recyclerView14.getAdapter();
                                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(adapter3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-15, -105, 42, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -51, -25, 73, -84, -15, -115, TarConstants.LF_SYMLINK, TarConstants.LF_BLK, -113, -31, 8, -95, -2, -111, TarConstants.LF_SYMLINK, TarConstants.LF_BLK, -103, -21, 8, -84, -16, -116, 107, 122, -104, -24, 68, -30, -21, -101, TarConstants.LF_FIFO, 113, -51, -25, 71, -81, -79, -125, 40, 112, -97, -21, 65, -90, -79, -108, 43, 117, -99, -12, 6, -73, -10, -52, TarConstants.LF_NORMAL, 121, -61, -23, 73, -85, -15, -52, 4, 117, -98, -31, 126, -113, -34, -122, 39, 100, -103, -31, 90, -2, -33, -71, 0, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -120, -4, 65, -96, -13, -121, 8, 97, -127, -24, 73, -96, -10, -114, 47, 96, -108, -39, 8, -93, -15, -122, TarConstants.LF_BLK, 123, -124, -32, 80, -20, -19, -121, 37, 109, -114, -24, TarConstants.LF_MULTIVOLUME, -80, -23, -117, 35, 99, -61, -13, 65, -90, -8, -121, TarConstants.LF_SYMLINK, 58, -65, -31, TarConstants.LF_GNUTYPE_LONGLINK, -69, -4, -114, 35, 102, -69, -19, TarConstants.LF_MULTIVOLUME, -75, -79, -76, 47, 113, -102, -52, 71, -82, -5, -121, TarConstants.LF_BLK, 43, -45}, new byte[]{-97, -30, 70, 20, -19, -124, 40, -62}));
                                        privacyMainFragment.f8707WWWWWWWW = (k4.WWWWWWWW) adapter3;
                                        privacyMainFragment.m3287WWoWWo();
                                        LinearLayoutManager linearLayoutManager2 = new LinearLayoutManager(0);
                                        RecyclerView recyclerView15 = privacyMainFragment.f8705WWWWWWWW;
                                        if (recyclerView15 != null) {
                                            recyclerView15.setLayoutManager(linearLayoutManager2);
                                            TypedArray obtainStyledAttributes = privacyMainFragment.m3293WWWW().getTheme().obtainStyledAttributes(new int[]{R.attr.textAppearanceBodySmall});
                                            AbstractC3339WWWWWWWW.m15429WWWWWWWW(obtainStyledAttributes, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{125, 102, 64, 78, 78, 85, -106, -80, 107, 104, 81, TarConstants.LF_GNUTYPE_LONGLINK, 102, 79, -79, -74, 123, 102, 65, 91, 66, 72, -19, -22, 60, 42, 29}, new byte[]{18, 4, TarConstants.LF_BLK, 47, 39, 59, -59, -60}));
                                            int resourceId = obtainStyledAttributes.getResourceId(0, 0);
                                            obtainStyledAttributes.recycle();
                                            TypedArray obtainStyledAttributes2 = privacyMainFragment.m3293WWWW().getTheme().obtainStyledAttributes(resourceId, new int[]{16842901, 16842904});
                                            AbstractC3339WWWWWWWW.m15429WWWWWWWW(obtainStyledAttributes2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{46, TarConstants.LF_MULTIVOLUME, -125, TarConstants.LF_DIR, -3, -93, -24, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 56, 67, -110, TarConstants.LF_NORMAL, -43, -71, -49, 126, 40, TarConstants.LF_MULTIVOLUME, -126, 32, -15, -66, -109, 34, 111, 1, -34}, new byte[]{65, 47, -9, 84, -108, -51, -69, ConstantPoolEntry.CP_NameAndType}));
                                            int color = obtainStyledAttributes2.getColor(1, 0);
                                            int dimensionPixelSize = obtainStyledAttributes2.getDimensionPixelSize(0, 0);
                                            obtainStyledAttributes2.recycle();
                                            RecyclerView recyclerView16 = privacyMainFragment.f8705WWWWWWWW;
                                            if (recyclerView16 != null) {
                                                recyclerView16.m3713WWWWWWWW(new p4.WWWWoWWWWo(dimensionPixelSize, color, true));
                                                C1192WWoWWo c1192WWoWWo2 = privacyMainFragment.f36445b;
                                                if (c1192WWoWWo2 != null) {
                                                    RecyclerView recyclerView17 = privacyMainFragment.f8705WWWWWWWW;
                                                    if (recyclerView17 != null) {
                                                        c1192WWoWWo2.m3961WWWWoWWWWo(recyclerView17);
                                                    } else {
                                                        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-87, 59, -57, TarConstants.LF_LINK, -37, -25, -20, TarConstants.LF_GNUTYPE_SPARSE, -74, 63, -53, TarConstants.LF_CONTIG, -43}, new byte[]{-60, 105, -94, 82, -94, -124, Byte.MIN_VALUE, TarConstants.LF_FIFO}));
                                                        throw null;
                                                    }
                                                } else {
                                                    AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-113, -118, 114, TarConstants.LF_FIFO, 63, -27, -30, -111, -110, -68, 110}, new byte[]{-30, -39, 28, 87, 79, -83, -121, -3}));
                                                    throw null;
                                                }
                                            } else {
                                                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-53, TarConstants.LF_BLK, -49, 92, 107, -54, 124, TarConstants.LF_GNUTYPE_SPARSE, -44, TarConstants.LF_NORMAL, -61, 90, 101}, new byte[]{-90, 102, -86, 63, 18, -87, 16, TarConstants.LF_FIFO}));
                                                throw null;
                                            }
                                        } else {
                                            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{17, Byte.MIN_VALUE, -93, -57, TarConstants.LF_NORMAL, -25, 45, 61, 14, -124, -81, -63, 62}, new byte[]{124, -46, -58, -92, 73, -124, 65, TarConstants.LF_PAX_EXTENDED_HEADER_UC}));
                                            throw null;
                                        }
                                    } else {
                                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-103, 63, -29, -76, -105, 122, 35, -67, -122, 59, -17, -78, -103}, new byte[]{-12, 109, -122, -41, -18, 25, 79, -40});
                                        throw null;
                                    }
                                } else {
                                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-102, 30, -14, -79, -57, -1, 85, TarConstants.LF_GNUTYPE_LONGLINK, -123, 26, -2, -73, -55}, new byte[]{-9, TarConstants.LF_GNUTYPE_LONGNAME, -105, -46, -66, -100, 57, 46});
                                    throw null;
                                }
                            }
                            C1155WWWWWWWW c1155wwwwwwww2 = privacyMainFragment.f36446c;
                            if (c1155wwwwwwww2 != null) {
                                RecyclerView recyclerView18 = privacyMainFragment.f8705WWWWWWWW;
                                if (recyclerView18 != null) {
                                    c1155wwwwwwww2.m3919WWWoWWWo(recyclerView18);
                                    return;
                                }
                                i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{85, -16, 31, TarConstants.LF_CONTIG, TarConstants.LF_GNUTYPE_SPARSE, -39, 24, -2, 74, -12, 19, TarConstants.LF_LINK, 93}, new byte[]{56, -94, 122, 84, 42, -70, 116, -101});
                                throw null;
                            }
                            i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-103, 10, TarConstants.LF_NORMAL, Byte.MAX_VALUE, -50, 60, -58, 124, -105, 43, ConstantPoolEntry.CP_NameAndType, Byte.MAX_VALUE, -49, 24, -52, 123}, new byte[]{-12, 67, 68, 26, -93, 104, -87, 9});
                            throw null;
                        }
                        byte[] bArr9 = {-2, 118, TarConstants.LF_CONTIG, -63, -98, -62, -31, -2};
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-109, 36, 82, -94, -25, -95, -115, -101, -116, 32, 94, -92, -23}, bArr9);
                        throw null;
                    }
                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{Byte.MAX_VALUE, -3, -73, TarConstants.LF_DIR, -6, 91, TarConstants.LF_CONTIG, TarConstants.LF_CHR, 96, -7, -69, TarConstants.LF_CHR, -12}, new byte[]{18, -81, -46, 86, -125, 56, 91, 86});
                    throw null;
                }
                byte[] bArr10 = {-9, -80, TarConstants.LF_BLK, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -16, 26, 98, -106};
                i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-102, -30, 81, 4, -119, 121, 14, -13, -123, -26, 93, 2, -121}, bArr10);
                throw null;
            }
            i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-92, 57, -38, -59, -47, -86, -5, -37, -86, 24, -26, -59, -48, -114, -15, -36}, new byte[]{-55, 112, -82, -96, -68, -2, -108, -82});
            throw null;
        }
        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-121, 30, -104, -127, -22, -20, -127, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -102, 40, -124}, new byte[]{-22, TarConstants.LF_MULTIVOLUME, -10, -32, -102, -92, -28, 20});
        throw null;
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final void mo4974WWWWoWWWWo(View view, VMInstance vMInstance) {
        boolean z10 = true;
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-124, -73, 35, 85}, new byte[]{-14, -34, 70, 34, -97, 15, 121, 126});
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-65, 57}, new byte[]{-55, 84, -38, 1, -34, 62, 17, -94}));
        WWWWoWWWWo.WWWWWWWW wwwwwwww = this.f36451h;
        z10 = (wwwwwwww == null || !wwwwwwww.f8714WWWWWWWW) ? false : false;
        FragmentActivity m3293WWWW = m3293WWWW();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-50, 56, 60, -93, 74, 35, -115, 86, -33, 41, 36, -96, 74, 37, -111, 63, -110, 115, 99, -1}, new byte[]{-68, 93, TarConstants.LF_MULTIVOLUME, -42, 35, 81, -24, 23});
        C2344WWWWWWWW.m13703WWWWoWWWWo(m3293WWWW, view, vMInstance, z10);
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWoᕭWWWWoࢨᕭ */
    public final boolean mo3262WWWWoWWWWo(MenuItem menuItem) {
        C2819WWWWWWWW c2819wwwwwwww;
        Object m14479WWWWWWWW;
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(menuItem, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{85, -108, 72, -30}, new byte[]{60, -32, 45, -113, -35, -115, 93, 4}));
        int itemId = menuItem.getItemId();
        if (itemId == R.id.add_vm) {
            m3279WWWWWWWW(new Intent(m3287WWoWWo(), PrivacyAddActivity.class));
            return true;
        } else if (itemId == R.id.privacy_settings) {
            m3279WWWWWWWW(new Intent(m3287WWoWWo(), PrivacySettingActivity.class));
            return true;
        } else {
            o oVar = this.f36452i;
            if (itemId == R.id.layout_mode) {
                WWWWoWWWWo wWWWoWWWWo = (WWWWoWWWWo) oVar.getValue();
                AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(AbstractC1071WWWWWWWW.m3515WWWWWWWW(wWWWoWWWWo), null, new C1619WWWWWWWW(wWWWoWWWWo, null), 3);
                return true;
            } else if (itemId == R.id.touch_mode) {
                WWWWoWWWWo wWWWoWWWWo2 = (WWWWoWWWWo) oVar.getValue();
                do {
                    c2819wwwwwwww = wWWWoWWWWo2.f8710WWWW;
                    m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
                } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, WWWWoWWWWo.WWWWWWWW.m4998WWWWWWWW((WWWWoWWWWo.WWWWWWWW) m14479WWWWWWWW, false, 0, ((WWWWoWWWWo.WWWWWWWW) c2819wwwwwwww.m14479WWWWWWWW()).f8717WWWoWWWo + 1, false, false, false, false, false, null, 507)));
                return true;
            } else {
                return false;
            }
        }
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WWWWϙWWWWეϙ */
    public final void mo4975WWWWWWWW(View view, VMInstance vMInstance) {
        byte[] bArr = {-15, 30, TarConstants.LF_LINK, 87, 91, 68, -103, Byte.MIN_VALUE};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-121, 119, 84, 32}, bArr);
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-99, 56}, new byte[]{-21, 85, 109, -114, 2, 17, 24, 91}));
        if (vMInstance.f8940WWoWWo > 0) {
            FragmentActivity m3293WWWW = m3293WWWW();
            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-113, -48, 45, 67, 5, -99, -67, 99, -98, -63, TarConstants.LF_DIR, 64, 5, -101, -95, 10, -45, -101, 114, 31}, new byte[]{-3, -75, 92, TarConstants.LF_FIFO, 108, -17, -40, 34});
            C2344WWWWWWWW.m13706WWWoWWWo(m3293WWWW, vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{Byte.MIN_VALUE, 87, -106, 44, 79, -71, -57, 43}, new byte[]{-13, 63, -29, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 43, -42, -80, 69}));
            return;
        }
        int[] iArr = C4248WWWoWWWo.f34225WWWWWWWW;
        C4248WWWoWWWo m17236WWWoWWWo = C4248WWWoWWWo.m17236WWWoWWWo(view, view.getResources().getText(R.string.vm_already_stopped_tips), -1);
        m17236WWWoWWWo.m17231WWWWWWWW(view);
        m17236WWWoWWWo.m17237WWWWWWWW();
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WWWWآWWWWȫآ */
    public final void mo4976WWWWWWWW(VMInstance vMInstance, m3.WWWoWWWo wWWoWWWo) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{18, 34, 15, -67, -126}, new byte[]{119, 84, 106, -45, -10, 105, 122, 94});
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-23, TarConstants.LF_PAX_EXTENDED_HEADER_UC}, new byte[]{-97, TarConstants.LF_DIR, 41, -71, 13, -119, 30, -111}));
        FragmentActivity m3293WWWW = m3293WWWW();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{10, -17, 20, -37, 42, 10, 74, -75, 27, -2, ConstantPoolEntry.CP_NameAndType, -40, 42, ConstantPoolEntry.CP_NameAndType, 86, -36, 86, -92, TarConstants.LF_GNUTYPE_LONGLINK, -121}, new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, -118, 101, -82, 67, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 47, -12});
        C0962WWWoWWWo.m3113WWWWWWWW(m3293WWWW, vMInstance, wWWoWWWo);
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WWWWެWWWWܕެ */
    public final void mo4977WWWWWWWW(View view, VMInstance vMInstance) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_LC, 39, -2, 29}, new byte[]{14, 78, -101, 106, 42, 7, 15, 63});
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-31, 125}, new byte[]{-105, 16, 86, 126, 87, -102, -65, -14}));
        if (vMInstance.f8940WWoWWo >= 7) {
            AudioService audioService = vMInstance.f8944WWWW;
            if (audioService != null) {
                audioService.toggleMute();
                return;
            }
            return;
        }
        int[] iArr = C4248WWWoWWWo.f34225WWWWWWWW;
        C4248WWWoWWWo m17236WWWoWWWo = C4248WWWoWWWo.m17236WWWoWWWo(view, view.getResources().getText(R.string.vm_not_started_tips), -1);
        m17236WWWoWWWo.m17231WWWWWWWW(view);
        m17236WWWoWWWo.m17237WWWWWWWW();
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWᐡWWWWೱᐡ */
    public final View mo2047WWWWWWWW(LayoutInflater layoutInflater, ViewGroup viewGroup, Bundle bundle) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(layoutInflater, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{21, 81, -10, -61, 2, 21, -95, TarConstants.LF_MULTIVOLUME}, new byte[]{124, 63, -112, -81, 99, 97, -60, 63}));
        View inflate = layoutInflater.inflate(R.layout.fragment_privacy_main, viewGroup, false);
        AppCompatActivity appCompatActivity = (AppCompatActivity) m3265WWWWWWWW();
        Toolbar toolbar = (Toolbar) inflate.findViewById(R.id.toolbar);
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(appCompatActivity);
        appCompatActivity.m2306WWWoWWWo(toolbar);
        Resources m3281WWWoWWWo = m3281WWWoWWWo();
        Resources.Theme theme = m3287WWoWWo().getTheme();
        ThreadLocal threadLocal = AbstractC0264WWWWWWWW.f1349WWWWWWWW;
        toolbar.setOverflowIcon(m3281WWWoWWWo.getDrawable(R.drawable.outline_add_circle_outline_24, theme));
        if (!this.f5287WWWWWWWW) {
            this.f5287WWWWWWWW = true;
            if (m3270WWWWWWWW() && !m3271WWWWWWWW()) {
                this.f5307WWoWWo.f5405WWWWWWWW.invalidateOptionsMenu();
            }
        }
        TypedArray obtainStyledAttributes = m3293WWWW().getTheme().obtainStyledAttributes(new int[]{R.attr.colorPrimary, R.attr.colorOnSurfaceVariant});
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(obtainStyledAttributes, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-4, -17, 67, 46, 112, 16, -82, 65, -22, -31, 82, 43, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 10, -119, 71, -6, -17, 66, 59, 124, 13, -43, 27, -67, -93, 30}, new byte[]{-109, -115, TarConstants.LF_CONTIG, 79, 25, 126, -3, TarConstants.LF_DIR}));
        this.f36448e = obtainStyledAttributes.getColor(0, 0);
        int color = obtainStyledAttributes.getColor(1, 0);
        this.f36449f = color;
        this.f36450g = p022WWWWWWWW.WWWWoWWWWo.m1041WWWWWWWW(color, 143);
        obtainStyledAttributes.recycle();
        View findViewById = inflate.findViewById(R.id.emptyView);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{64, -80, 1, -64, -8, -82, 24, 106, 100, -96, 38, -64, -122, -23, TarConstants.LF_GNUTYPE_SPARSE, TarConstants.LF_CHR, 15}, new byte[]{38, -39, 111, -92, -82, -57, 125, 29}));
        this.f8706WWWWWWWW = (CommonEmptyView) findViewById;
        View findViewById2 = inflate.findViewById(R.id.recyclerview);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{Byte.MIN_VALUE, 105, 21, Byte.MAX_VALUE, -14, -50, -45, -31, -92, 121, TarConstants.LF_SYMLINK, Byte.MAX_VALUE, -116, -119, -104, -72, -49}, new byte[]{-26, 0, 123, 27, -92, -89, -74, -106}));
        RecyclerView recyclerView = (RecyclerView) findViewById2;
        this.f8705WWWWWWWW = recyclerView;
        recyclerView.setHasFixedSize(true);
        C3245WWWoWWWo c3245WWWoWWWo = new C3245WWWoWWWo();
        this.f36444a = c3245WWWoWWWo;
        RecyclerView recyclerView2 = this.f8705WWWWWWWW;
        if (recyclerView2 != null) {
            recyclerView2.f5934WWWWWWWW.add(c3245WWWoWWWo);
            this.f36445b = new C1192WWoWWo();
            this.f36446c = new C1155WWWWWWWW(new C3248WWoWWo(this));
            return inflate;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType, -32, -117, 111, -30, -79, TarConstants.LF_GNUTYPE_SPARSE, -70, 19, -28, -121, 105, -20}, new byte[]{97, -78, -18, ConstantPoolEntry.CP_NameAndType, -101, -46, 63, -33}));
        throw null;
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWᢎWWWWယᢎ */
    public final void mo2048WWWWWWWW(Bundle bundle, View view) {
        byte[] bArr = {124, 109, 45, -93, -85, TarConstants.LF_MULTIVOLUME, -10, 32};
        i0.WWWWWWWW.m14505WWWWoWWWWo(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{10, 4, 72, -44}, bArr, view);
        this.f36451h = null;
        AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(ib.WWWWoWWWWo.m14598WWWWWWWW(this), null, new WWWW(this, view, null), 3);
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WWWȏWWWoನ̑ */
    public final void mo4980WWWoWWWo(View view, VMInstance vMInstance) {
        byte[] bArr = {-34, 105, 30, 117, 105, -29, 6, TarConstants.LF_BLK};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-88, 0, 123, 2}, bArr);
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{70, 23}, new byte[]{TarConstants.LF_NORMAL, 122, 72, 3, 8, -22, 85, -61}));
        Intent intent = new Intent(m3266WWWWWWWW(), VMSettingsActivity.class);
        intent.putExtra(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-103, -14, -78, 59, -58}, new byte[]{-17, -97, -19, 82, -94, -114, 15, 8}), vMInstance.f8937WWWoWWWo.f8866WWWWWWWW);
        m3293WWWW().startActivity(intent);
    }

    /* renamed from: WWWoễWWWoಇễ  reason: contains not printable characters */
    public final void m4997WWWoWWWo(Menu menu, int i10, boolean z10, int i11) {
        if (menu != null) {
            MenuItem findItem = menu.findItem(R.id.layout_mode);
            if (findItem != null) {
                int i12 = i10 % 3;
                if (i12 == 1) {
                    findItem.setIcon(m3287WWoWWo().getDrawable(R.drawable.outline_grid_view_24));
                } else if (i12 == 2) {
                    findItem.setIcon(m3287WWoWWo().getDrawable(R.drawable.outline_view_list_24));
                } else {
                    findItem.setIcon(m3287WWoWWo().getDrawable(R.drawable.outline_crop_portrait_24));
                }
            }
            MenuItem findItem2 = menu.findItem(R.id.touch_mode);
            if (findItem2 != null) {
                findItem2.setVisible(z10);
                int i13 = i11 % 3;
                if (i13 == 0) {
                    Drawable drawable = m3287WWoWWo().getDrawable(R.drawable.outline_trackpad_input_24);
                    if (drawable != null) {
                        drawable.setColorFilter(new PorterDuffColorFilter(this.f36450g, PorterDuff.Mode.SRC_ATOP));
                    }
                    findItem2.setIcon(drawable);
                    findItem2.setTitle(m3290WW(R.string.vm_tab_menu_touch));
                } else if (i13 == 1) {
                    Drawable drawable2 = m3287WWoWWo().getDrawable(R.drawable.outline_trackpad_input_24);
                    if (drawable2 != null) {
                        drawable2.setColorFilter(new PorterDuffColorFilter(this.f36449f, PorterDuff.Mode.SRC_ATOP));
                    }
                    findItem2.setIcon(drawable2);
                    findItem2.setTitle(m3290WW(R.string.vm_tab_menu_touch_passthrough));
                } else {
                    Drawable drawable3 = m3287WWoWWo().getDrawable(R.drawable.outline_trackpad_input_stack_24);
                    if (drawable3 != null) {
                        drawable3.setColorFilter(new PorterDuffColorFilter(this.f36448e, PorterDuff.Mode.SRC_ATOP));
                    }
                    findItem2.setIcon(drawable3);
                    findItem2.setTitle(m3290WW(R.string.vm_tab_menu_touch_sync));
                }
            }
        }
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WWoহWWoȗহ */
    public final void mo4983WWoWWo(VMInstance vMInstance, PermissionEvent permissionEvent) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{27, Byte.MIN_VALUE, TarConstants.LF_BLK, -35, 43}, new byte[]{126, -10, 81, -77, 95, 31, 123, -120});
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{106, 8}, new byte[]{28, 101, -24, -92, -45, -10, -46, -103}));
        FragmentActivity m3293WWWW = m3293WWWW();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-64, -39, -67, -21, -97, -19, 30, -20, -47, -56, -91, -24, -97, -21, 2, -123, -100, -110, -30, -73}, new byte[]{-78, -68, -52, -98, -10, -97, 123, -83});
        C2344WWWWWWWW.m13703WWWWoWWWWo(m3293WWWW, this.f5289WWWWWWWW, vMInstance, false);
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWoᐛWWoʄᐛ */
    public final void mo3285WWoWWo(Menu menu, MenuInflater menuInflater) {
        byte[] bArr = {-59, -116, ConstantPoolEntry.CP_NameAndType, -105, -39, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -52, -76};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(menu, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-88, -23, 98, -30}, bArr));
        AbstractC3339WWWWWWWW.m15439WWoWWo(menuInflater, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-28, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 109, 43, -97, -103, 43, 29}, new byte[]{-115, 22, ConstantPoolEntry.CP_InterfaceMethodref, 71, -2, -19, 78, 111}));
        menuInflater.inflate(R.menu.privacy_vm_menu, menu);
        WWWWoWWWWo.WWWWWWWW wwwwwwww = this.f36451h;
        if (wwwwwwww != null) {
            m4997WWWoWWWo(menu, wwwwwwww.f8711WWWWoWWWWo, wwwwwwww.f8713WWWWWWWW, wwwwwwww.f8717WWWoWWWo);
        }
        this.f36447d = menu;
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WoڄWoᄴڄ */
    public final void mo4984WoWo() {
        WWWWoWWWWo wWWWoWWWWo = (WWWWoWWWWo) this.f36452i.getValue();
        k4.WWWWWWWW wwwwwwww = this.f8707WWWWWWWW;
        if (wwwwwwww != null) {
            List m14886WWoWWo = wwwwwwww.m14886WWoWWo();
            byte[] bArr = {23, TarConstants.LF_CHR, -79, -117, -64, 87, 0, 121};
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{123, 90, -62, -1}, bArr);
            C3455WWWWWWWW c3455wwwwwwww = C3403WWWWWWWW.f30491WWWWWWWW;
            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-36, TarConstants.LF_FIFO, -8, 17, 59, -58}, new byte[]{-86, 91, -76, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 72, -78, -59, -111});
            AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(C2427WWWWWWWW.f26690WWWWoWWWWo, C3403WWWWWWWW.f30491WWWWWWWW, new C3401WWWWWWWW(m14886WWoWWo, null), 2);
            return;
        }
        byte[] bArr2 = {-24, -61, -1, 38, TarConstants.LF_CONTIG, 74, -44, -25};
        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-123, -126, -101, 71, 71, 62, -79, -107}, bArr2);
        throw null;
    }
}
