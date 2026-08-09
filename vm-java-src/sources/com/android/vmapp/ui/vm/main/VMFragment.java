package com.android.vmapp.ui.vm.main;

import android.content.Context;
import android.content.Intent;
import android.content.res.Resources;
import android.content.res.TypedArray;
import android.graphics.PorterDuff;
import android.graphics.PorterDuffColorFilter;
import android.graphics.drawable.Drawable;
import android.net.Uri;
import android.os.Bundle;
import android.text.TextUtils;
import android.view.LayoutInflater;
import android.view.Menu;
import android.view.MenuInflater;
import android.view.MenuItem;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.ImageView;
import android.widget.TextView;
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
import b4.WWoWWo;
import c0.C1458WWWW;
import com.android.vmapp.ui.vm.advanced.SettingsActivity;
import com.android.vmapp.ui.vm.backup.VMBackupRestoreActivity;
import com.android.vmapp.ui.vm.create.VMCreateActivity;
import com.android.vmapp.ui.vm.main.WWWWoWWWWo;
import com.android.vmapp.ui.vm.settings.VMSettingsActivity;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.PermissionEvent;
import com.android.vmcore.hal.AudioService;
import com.clone.android.dual.space.R;
import com.google.android.gms.internal.measurement.p1;
import com.google.android.material.floatingactionbutton.ExtendedFloatingActionButton;
import com.google.android.material.snackbar.BaseTransientBottomBar$SnackbarBaseLayout;
import com.google.android.material.snackbar.SnackbarContentLayout;
import com.google.firebase.Firebase;
import com.google.firebase.analytics.AnalyticsKt;
import com.google.firebase.analytics.FirebaseAnalytics;
import com.google.firebase.analytics.ParametersBuilder;
import e0.C2320WWWWWWWW;
import e4.C2344WWWWWWWW;
import ed.AbstractC2403WWWWoWWWWo;
import ed.C2427WWWWWWWW;
import fc.C2520WWWWWWWW;
import fc.EnumC2528WWWoWWWo;
import fc.InterfaceC2519WWWWWWWW;
import i4.WWWWWWWWWW;
import j3.C3164WWWWWWWW;
import java.util.ArrayList;
import java.util.List;
import k4.C3235WWWWWWWW;
import k4.C3245WWWoWWWo;
import k4.C3247WWoWWo;
import k4.C3248WWoWWo;
import k4.InterfaceC3250WWoWWo;
import k4.View$OnClickListenerC3244WWWoWWWo;
import k4.WWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import l3.C3379WWWWoWWWWo;
import l3.C3403WWWWWWWW;
import ld.C3455WWWWWWWW;
import n6.AbstractC3585WWoWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p.AbstractC3783WWWWWWWW;
import p.C3784WWWWWWWW;
import p.InterfaceC3766WWWWoWWWWo;
import p013WWWWWWWW.o;
import p021WWWWWWWW.AbstractC0264WWWWWWWW;
import p029WWWWWWWW.WWWoWWWo;
import ta.C4248WWWoWWWo;
import v3.WWWWWWWW;
/* loaded from: classes.dex */
public final class VMFragment extends WWWWWWWW implements InterfaceC3250WWoWWo, InterfaceC3766WWWWoWWWWo {

    /* renamed from: WWWWỖWWWWࢥỖ  reason: contains not printable characters */
    public RecyclerView f8654WWWWWWWW;

    /* renamed from: WWWWίWWWWРί  reason: contains not printable characters */
    public VMEmptyCardView f8655WWWWWWWW;

    /* renamed from: WWWWℯWWWW၂ℯ  reason: contains not printable characters */
    public View f8656WWWWWWWW;

    /* renamed from: a  reason: collision with root package name */
    public View f36431a;

    /* renamed from: b  reason: collision with root package name */
    public ExtendedFloatingActionButton f36432b;

    /* renamed from: c  reason: collision with root package name */
    public k4.WWWWWWWW f36433c;

    /* renamed from: d  reason: collision with root package name */
    public C3245WWWoWWWo f36434d;

    /* renamed from: e  reason: collision with root package name */
    public C1192WWoWWo f36435e;

    /* renamed from: f  reason: collision with root package name */
    public C1155WWWWWWWW f36436f;

    /* renamed from: g  reason: collision with root package name */
    public Menu f36437g;

    /* renamed from: h  reason: collision with root package name */
    public int f36438h;

    /* renamed from: i  reason: collision with root package name */
    public int f36439i;

    /* renamed from: j  reason: collision with root package name */
    public int f36440j;

    /* renamed from: k  reason: collision with root package name */
    public int f36441k;

    /* renamed from: l  reason: collision with root package name */
    public WWWWoWWWWo.WWWWWWWW f36442l;

    /* renamed from: m  reason: collision with root package name */
    public final o f36443m;

    public VMFragment() {
        C1458WWWW c1458wwww = new C1458WWWW(10);
        InterfaceC2519WWWWWWWW m13999WWWWWWWW = C2520WWWWWWWW.m13999WWWWWWWW(EnumC2528WWWoWWWo.f27073WWWWWWWWWW, new WWWoWWWo(10, new WWWoWWWo(9, this)));
        this.f36443m = new o(C3333WWWWoWWWWo.m15421WWWWWWWW(WWWWoWWWWo.class), new WWWWWWWWWW(m13999WWWWWWWW, 2), c1458wwww, new WWWWWWWWWW(m13999WWWWWWWW, 3));
    }

    /* renamed from: WWWếWWW෨ế  reason: contains not printable characters */
    public static final void m4973WWWWWW(VMFragment vMFragment, boolean z10, int i10) {
        C1192WWoWWo c1192WWoWWo = vMFragment.f36435e;
        if (c1192WWoWWo != null) {
            c1192WWoWWo.m3961WWWWoWWWWo(null);
            C1155WWWWWWWW c1155wwwwwwww = vMFragment.f36436f;
            if (c1155wwwwwwww != null) {
                c1155wwwwwwww.m3919WWWoWWWo(null);
                RecyclerView recyclerView = vMFragment.f8654WWWWWWWW;
                if (recyclerView != null) {
                    ArrayList arrayList = recyclerView.f36400b;
                    if (arrayList != null) {
                        arrayList.clear();
                    }
                    RecyclerView recyclerView2 = vMFragment.f8654WWWWWWWW;
                    if (recyclerView2 != null) {
                        recyclerView2.setOnFlingListener(null);
                        RecyclerView recyclerView3 = vMFragment.f8654WWWWWWWW;
                        if (recyclerView3 != null) {
                            int itemDecorationCount = recyclerView3.getItemDecorationCount();
                            for (int i11 = 0; i11 < itemDecorationCount; i11++) {
                                RecyclerView recyclerView4 = vMFragment.f8654WWWWWWWW;
                                if (recyclerView4 != null) {
                                    recyclerView4.m3751WWoWWo();
                                } else {
                                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-79, -16, -59, -87, -95, -20, 13, -11, -82, -12, -55, -81, -81}, new byte[]{-36, -94, -96, -54, -40, -113, 97, -112});
                                    throw null;
                                }
                            }
                            int i12 = i10 % 3;
                            if (i12 == 1) {
                                if (z10) {
                                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                    AbstractC3585WWoWWo.m16060WWWWoWWWWo(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -20, 112, -38, 37, -85, -56, -62, 102, -27}, new byte[]{20, -127, 17, -74, 73, -12, -85, -93}));
                                }
                                RecyclerView recyclerView5 = vMFragment.f8654WWWWWWWW;
                                if (recyclerView5 != null) {
                                    recyclerView5.setAdapter(new C3235WWWWWWWW(vMFragment, 2));
                                    RecyclerView recyclerView6 = vMFragment.f8654WWWWWWWW;
                                    if (recyclerView6 != null) {
                                        RecyclerView.WWWWWWWW adapter = recyclerView6.getAdapter();
                                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(adapter, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-117, -53, 5, -73, 94, 95, -94, -60, -117, -47, 29, -5, 28, 89, -29, -55, -124, -51, 29, -5, 10, TarConstants.LF_GNUTYPE_SPARSE, -29, -60, -118, -48, 68, -75, ConstantPoolEntry.CP_InterfaceMethodref, 80, -81, -118, -111, -57, 25, -66, 94, 95, -84, -57, -53, -33, 7, -65, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_GNUTYPE_SPARSE, -86, -50, -53, -56, 4, -70, 14, TarConstants.LF_GNUTYPE_LONGNAME, -19, -33, -116, -112, 31, -74, 80, 81, -94, -61, -117, -112, 43, -70, 13, 89, -107, -25, -92, -38, 8, -85, 10, 89, -79, -106, -91, -27, 47, -73, 27, 68, -86, -56, -119, -37, 39, -82, 18, 80, -94, -56, -116, -46, 0, -81, 7, 97, -29, -53, -117, -38, 27, -76, 23, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -69, -124, -105, -37, 10, -94, 29, 80, -90, -40, -109, -41, ConstantPoolEntry.CP_NameAndType, -84, 80, TarConstants.LF_GNUTYPE_LONGLINK, -86, -50, -126, -37, 29, -11, 44, 89, -96, -45, -122, -46, ConstantPoolEntry.CP_NameAndType, -87, 40, 85, -90, -35, -53, -24, 0, -66, 9, 116, -84, -58, -127, -37, 27, -28, 64}, new byte[]{-27, -66, 105, -37, 126, 60, -61, -86}));
                                        vMFragment.f36433c = (k4.WWWWWWWW) adapter;
                                        vMFragment.m3287WWoWWo();
                                        GridLayoutManager gridLayoutManager = new GridLayoutManager();
                                        RecyclerView recyclerView7 = vMFragment.f8654WWWWWWWW;
                                        if (recyclerView7 != null) {
                                            recyclerView7.setLayoutManager(gridLayoutManager);
                                            RecyclerView recyclerView8 = vMFragment.f8654WWWWWWWW;
                                            if (recyclerView8 != null) {
                                                recyclerView8.m3713WWWWWWWW(new p4.WWWWWWWW(1, vMFragment.m3281WWWoWWWo().getDimensionPixelOffset(R.dimen.cat_card_grid_spacing_horizontal), vMFragment.m3281WWWoWWWo().getDimensionPixelOffset(R.dimen.cat_list_horizontal_padding), vMFragment.m3281WWWoWWWo().getDimensionPixelOffset(R.dimen.cat_card_grid_spacing_vertical), vMFragment.m3281WWWoWWWo().getDimensionPixelOffset(R.dimen.cat_list_vertical_padding)));
                                            } else {
                                                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{6, -70, -122, 81, 25, -64, 28, -43, 25, -66, -118, 87, 23}, new byte[]{107, -24, -29, TarConstants.LF_SYMLINK, 96, -93, 112, -80}));
                                                throw null;
                                            }
                                        } else {
                                            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{66, 115, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 125, TarConstants.LF_SYMLINK, ConstantPoolEntry.CP_NameAndType, -74, Byte.MAX_VALUE, 93, 119, 84, 123, 60}, new byte[]{47, 33, 61, 30, TarConstants.LF_GNUTYPE_LONGLINK, 111, -38, 26}));
                                            throw null;
                                        }
                                    } else {
                                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{3, 61, 79, 122, -66, 69, Byte.MAX_VALUE, 64, 28, 57, 67, 124, -80}, new byte[]{110, 111, 42, 25, -57, 38, 19, 37});
                                        throw null;
                                    }
                                } else {
                                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-112, -56, 24, 67, 107, 109, -34, 3, -113, -52, 20, 69, 101}, new byte[]{-3, -102, 125, 32, 18, 14, -78, 102});
                                    throw null;
                                }
                            } else if (i12 == 2) {
                                if (z10) {
                                    byte[] bArr = {-4, -107, TarConstants.LF_CHR, 102, -7, -76, 33, -90};
                                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                    AbstractC3585WWoWWo.m16060WWWWoWWWWo(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -4, 64, 18}, bArr));
                                }
                                RecyclerView recyclerView9 = vMFragment.f8654WWWWWWWW;
                                if (recyclerView9 != null) {
                                    recyclerView9.setAdapter(new C3235WWWWWWWW(vMFragment, 1));
                                    RecyclerView recyclerView10 = vMFragment.f8654WWWWWWWW;
                                    if (recyclerView10 != null) {
                                        RecyclerView.WWWWWWWW adapter2 = recyclerView10.getAdapter();
                                        byte[] bArr2 = {-18, -77, 90, -66, 23, 118, 5, 99, -18, -87, 66, -14, 85, 112, 68, 110, -31, -75, 66, -14, 67, 122, 68, 99, -17, -88, 27, -68, 66, 121, 8, 45, -12, -65, 70, -73, 23, 118, ConstantPoolEntry.CP_InterfaceMethodref, 96, -82, -89, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -74, 69, 122, 13, 105, -82, -80, 91, -77, 71, 101, 74, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -23, -24, 64, -65, 25, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 5, 100, -18, -24, 116, -77, 68, 112, TarConstants.LF_SYMLINK, 64, -63, -94, 87, -94, 67, 112, 22, TarConstants.LF_LINK, -64, -99, 112, -66, 82, 109, 13, 111, -20, -93, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -89, 91, 121, 5, 111, -23, -86, 95, -90, 78, 72, 68, 108, -18, -94, 68, -67, 94, 113, 28, 35, -14, -93, 85, -85, 84, 121, 1, Byte.MAX_VALUE, -10, -81, TarConstants.LF_GNUTYPE_SPARSE, -91, 25, 98, 13, 105, -25, -93, 66, -4, 101, 112, 7, 116, -29, -86, TarConstants.LF_GNUTYPE_SPARSE, -96, 97, 124, 1, 122, -82, -112, 95, -73, 64, 93, ConstantPoolEntry.CP_InterfaceMethodref, 97, -28, -93, 68, -19, 9};
                                        byte[] bArr3 = {Byte.MIN_VALUE, -58, TarConstants.LF_FIFO, -46, TarConstants.LF_CONTIG, 21, 100, 13};
                                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(adapter2, x5.WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                                        vMFragment.f36433c = (k4.WWWWWWWW) adapter2;
                                        vMFragment.m3287WWoWWo();
                                        LinearLayoutManager linearLayoutManager = new LinearLayoutManager(1);
                                        RecyclerView recyclerView11 = vMFragment.f8654WWWWWWWW;
                                        if (recyclerView11 != null) {
                                            recyclerView11.setLayoutManager(linearLayoutManager);
                                            RecyclerView recyclerView12 = vMFragment.f8654WWWWWWWW;
                                            if (recyclerView12 != null) {
                                                recyclerView12.m3713WWWWWWWW(new C1187WWoWWo(vMFragment.m3287WWoWWo()));
                                            } else {
                                                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-119, -66, -32, -62, -51, -55, -28, Byte.MAX_VALUE, -106, -70, -20, -60, -61}, new byte[]{-28, -20, -123, -95, -76, -86, -120, 26}));
                                                throw null;
                                            }
                                        } else {
                                            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-113, 35, 72, -93, 105, 57, 16, -4, -112, 39, 68, -91, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}, new byte[]{-30, 113, 45, -64, 16, 90, 124, -103}));
                                            throw null;
                                        }
                                    } else {
                                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{38, -2, 42, 85, 97, -110, 60, -26, 57, -6, 38, TarConstants.LF_GNUTYPE_SPARSE, 111}, new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -84, 79, TarConstants.LF_FIFO, 24, -15, 80, -125});
                                        throw null;
                                    }
                                } else {
                                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{43, -23, 102, -120, -36, -58, -17, -112, TarConstants.LF_BLK, -19, 106, -114, -46}, new byte[]{70, -69, 3, -21, -91, -91, -125, -11});
                                    throw null;
                                }
                            } else {
                                if (z10) {
                                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                    AbstractC3585WWoWWo.m16060WWWWoWWWWo(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-67, -77, -32, -50, 56, -48, -74, -49}, new byte[]{-33, -38, -121, -111, 91, -79, -60, -85}));
                                }
                                RecyclerView recyclerView13 = vMFragment.f8654WWWWWWWW;
                                if (recyclerView13 != null) {
                                    recyclerView13.setAdapter(new C3235WWWWWWWW(vMFragment, 0));
                                    RecyclerView recyclerView14 = vMFragment.f8654WWWWWWWW;
                                    if (recyclerView14 != null) {
                                        RecyclerView.WWWWWWWW adapter3 = recyclerView14.getAdapter();
                                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                                        AbstractC3339WWWWWWWW.m15428WWWWWWWW(adapter3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, -65, -57, 44, -19, 10, -45, -17, -75, -91, -33, 96, -81, ConstantPoolEntry.CP_NameAndType, -110, -30, -70, -71, -33, 96, -71, 6, -110, -17, -76, -92, -122, 46, -72, 5, -34, -95, -81, -77, -37, 37, -19, 10, -35, -20, -11, -85, -59, 36, -65, 6, -37, -27, -11, -68, -58, 33, -67, 25, -100, -12, -78, -28, -35, 45, -29, 4, -45, -24, -75, -28, -23, 33, -66, ConstantPoolEntry.CP_NameAndType, -28, -52, -102, -82, -54, TarConstants.LF_NORMAL, -71, ConstantPoolEntry.CP_NameAndType, -64, -67, -101, -111, -19, 44, -88, 17, -37, -29, -73, -81, -27, TarConstants.LF_DIR, -95, 5, -45, -29, -78, -90, -62, TarConstants.LF_BLK, -76, TarConstants.LF_BLK, -110, -32, -75, -82, -39, 47, -92, 13, -54, -81, -87, -81, -56, 57, -82, 5, -41, -13, -83, -93, -50, TarConstants.LF_CONTIG, -29, 30, -37, -27, -68, -81, -33, 110, -97, ConstantPoolEntry.CP_NameAndType, -47, -8, -72, -90, -50, TarConstants.LF_SYMLINK, -101, 0, -41, -10, -11, -100, -62, 37, -70, 33, -35, -19, -65, -81, -39, Byte.MAX_VALUE, -13}, new byte[]{-37, -54, -85, 64, -51, 105, -78, -127}));
                                        vMFragment.f36433c = (k4.WWWWWWWW) adapter3;
                                        VMBigCardLayoutManager vMBigCardLayoutManager = new VMBigCardLayoutManager(vMFragment.m3287WWoWWo());
                                        RecyclerView recyclerView15 = vMFragment.f8654WWWWWWWW;
                                        if (recyclerView15 != null) {
                                            recyclerView15.setLayoutManager(vMBigCardLayoutManager);
                                            TypedArray obtainStyledAttributes = vMFragment.m3293WWWW().getTheme().obtainStyledAttributes(new int[]{R.attr.textAppearanceBodySmall});
                                            AbstractC3339WWWWWWWW.m15429WWWWWWWW(obtainStyledAttributes, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{5, TarConstants.LF_CHR, 115, 32, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -125, -114, 3, 19, 61, 98, 37, 79, -103, -87, 5, 3, TarConstants.LF_CHR, 114, TarConstants.LF_DIR, 107, -98, -11, 89, 68, Byte.MAX_VALUE, 46}, new byte[]{106, 81, 7, 65, 14, -19, -35, 119}));
                                            int resourceId = obtainStyledAttributes.getResourceId(0, 0);
                                            obtainStyledAttributes.recycle();
                                            TypedArray obtainStyledAttributes2 = vMFragment.m3293WWWW().getTheme().obtainStyledAttributes(resourceId, new int[]{16842901, 16842904});
                                            AbstractC3339WWWWWWWW.m15429WWWWWWWW(obtainStyledAttributes2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-105, TarConstants.LF_FIFO, 25, -125, -5, -80, -90, -8, -127, 56, 8, -122, -45, -86, -127, -2, -111, TarConstants.LF_FIFO, 24, -106, -9, -83, -35, -94, -42, 122, 68}, new byte[]{-8, 84, 109, -30, -110, -34, -11, -116}));
                                            int color = obtainStyledAttributes2.getColor(1, 0);
                                            int dimensionPixelSize = obtainStyledAttributes2.getDimensionPixelSize(0, 0);
                                            obtainStyledAttributes2.recycle();
                                            RecyclerView recyclerView16 = vMFragment.f8654WWWWWWWW;
                                            if (recyclerView16 != null) {
                                                recyclerView16.m3713WWWWWWWW(new p4.WWWWoWWWWo(dimensionPixelSize, color, true));
                                                C1192WWoWWo c1192WWoWWo2 = vMFragment.f36435e;
                                                if (c1192WWoWWo2 != null) {
                                                    RecyclerView recyclerView17 = vMFragment.f8654WWWWWWWW;
                                                    if (recyclerView17 != null) {
                                                        c1192WWoWWo2.m3961WWWWoWWWWo(recyclerView17);
                                                    } else {
                                                        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-27, 35, 24, 22, 82, -49, 110, -50, -6, 39, 20, 16, 92}, new byte[]{-120, 113, 125, 117, 43, -84, 2, -85}));
                                                        throw null;
                                                    }
                                                } else {
                                                    AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_NORMAL, TarConstants.LF_CONTIG, -15, 17, -109, Byte.MAX_VALUE, 74, -115, 45, 1, -19}, new byte[]{93, 100, -97, 112, -29, TarConstants.LF_CONTIG, 47, -31}));
                                                    throw null;
                                                }
                                            } else {
                                                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, -107, -33, 68, 31, -76, TarConstants.LF_GNUTYPE_SPARSE, -76, -86, -111, -45, 66, 17}, new byte[]{-40, -57, -70, 39, 102, -41, 63, -47}));
                                                throw null;
                                            }
                                        } else {
                                            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{71, -75, -4, 89, -32, -10, TarConstants.LF_GNUTYPE_SPARSE, -94, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -79, -16, 95, -18}, new byte[]{42, -25, -103, 58, -103, -107, 63, -57}));
                                            throw null;
                                        }
                                    } else {
                                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-18, 7, 65, 10, 10, -103, -2, 111, -15, 3, TarConstants.LF_MULTIVOLUME, ConstantPoolEntry.CP_NameAndType, 4}, new byte[]{-125, 85, 36, 105, 115, -6, -110, 10});
                                        throw null;
                                    }
                                } else {
                                    byte[] bArr4 = {7, -124, -25, 100, -17, -126, TarConstants.LF_CHR, -45};
                                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{106, -42, -126, 7, -106, -31, 95, -74, 117, -46, -114, 1, -104}, bArr4);
                                    throw null;
                                }
                            }
                            C1155WWWWWWWW c1155wwwwwwww2 = vMFragment.f36436f;
                            if (c1155wwwwwwww2 != null) {
                                RecyclerView recyclerView18 = vMFragment.f8654WWWWWWWW;
                                if (recyclerView18 != null) {
                                    c1155wwwwwwww2.m3919WWWoWWWo(recyclerView18);
                                    return;
                                }
                                i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-47, 64, 47, 98, -91, -3, -5, 1, -50, 68, 35, 100, -85}, new byte[]{-68, 18, 74, 1, -36, -98, -105, 100});
                                throw null;
                            }
                            i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-117, -81, -56, 56, -71, 118, 56, 84, -123, -114, -12, 56, -72, 82, TarConstants.LF_SYMLINK, TarConstants.LF_GNUTYPE_SPARSE}, new byte[]{-26, -26, -68, 93, -44, 34, 87, 33});
                            throw null;
                        }
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{106, 15, -63, -103, 42, 1, -111, 63, 117, ConstantPoolEntry.CP_InterfaceMethodref, -51, -97, 36}, new byte[]{7, 93, -92, -6, TarConstants.LF_GNUTYPE_SPARSE, 98, -3, 90});
                        throw null;
                    }
                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-97, -72, -74, -16, -1, 59, -77, TarConstants.LF_PAX_EXTENDED_HEADER_UC, Byte.MIN_VALUE, -68, -70, -10, -15}, new byte[]{-14, -22, -45, -109, -122, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -33, 61});
                    throw null;
                }
                byte[] bArr5 = {TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -126, 126, 38, -25, -32, TarConstants.LF_GNUTYPE_LONGLINK, -70};
                i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{10, -48, 27, 69, -98, -125, 39, -33, 21, -44, 23, 67, -112}, bArr5);
                throw null;
            }
            i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-36, -65, -24, -42, TarConstants.LF_GNUTYPE_LONGLINK, 109, 64, 43, -46, -98, -44, -42, 74, 73, 74, 44}, new byte[]{-79, -10, -100, -77, 38, 57, 47, 94});
            throw null;
        }
        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{Byte.MAX_VALUE, -17, -101, 112, -95, -119, -39, 85, 98, -39, -121}, new byte[]{18, -68, -11, 17, -47, -63, -68, 57});
        throw null;
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final void mo4974WWWWoWWWWo(View view, VMInstance vMInstance) {
        boolean z10;
        boolean z11 = true;
        byte[] bArr = {-14, TarConstants.LF_GNUTYPE_SPARSE, 78, 115};
        byte[] bArr2 = {-124, 58, 43, 4, -51, 32, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_NORMAL};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{117, -70}, new byte[]{3, -41, -33, -26, 109, -93, TarConstants.LF_SYMLINK, 15}));
        if (vMInstance.f8940WWoWWo > 0) {
            z10 = true;
        } else {
            z10 = false;
        }
        Firebase firebase = Firebase.INSTANCE;
        FirebaseAnalytics analytics = AnalyticsKt.getAnalytics(firebase);
        String m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-109, 56, -125, Byte.MAX_VALUE, 104, -33, -49, -35, -81, 39, -98, 125, 113, -12, -26, -39, -98, ConstantPoolEntry.CP_InterfaceMethodref, -98, 125, 97}, new byte[]{-16, 84, -22, 28, 3, Byte.MIN_VALUE, -71, -80});
        ParametersBuilder parametersBuilder = new ParametersBuilder();
        parametersBuilder.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{58, 90, 90, 64, -97, 107, -20}, new byte[]{73, 46, 59, TarConstants.LF_SYMLINK, -21, 14, -120, 34}), String.valueOf(z10));
        analytics.logEvent(m17835WWWWWWWW, parametersBuilder.getBundle());
        if (t8.WWWWWWWW.m17201WWWWWWWW(m3287WWoWWo()).m4644WWWWoWWWWo()) {
            AnalyticsKt.getAnalytics(firebase).logEvent(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{64, -69, 105, -110, -110, TarConstants.LF_GNUTYPE_LONGNAME, 38, -18, 89, -89, 117, -117, -125, 84, 21, -43, 69, -78, 69, -107, -101, 79, 61}, new byte[]{41, -43, 26, -26, -13, 32, 74, -79}), new ParametersBuilder().getBundle());
            da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(m3293WWWW());
            View inflate = LayoutInflater.from(m3287WWoWWo()).inflate(R.layout.dialog_install_prompt, (ViewGroup) null);
            wWWWoWWWWo.m13644WWWWWWWW(inflate);
            ((TextView) inflate.findViewById(16908299)).setText(R.string.dialog_msg_install_prompt);
            ((Button) inflate.findViewById(16908313)).setOnClickListener(new View$OnClickListenerC3244WWWoWWWo(this, 1));
            wWWWoWWWWo.m741WWoWWo();
            return;
        }
        WWWWoWWWWo.WWWWWWWW wwwwwwww = this.f36442l;
        z11 = (wwwwwwww == null || !wwwwwwww.f8684WWWWWWWW) ? false : false;
        FragmentActivity m3293WWWW = m3293WWWW();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{31, -23, -123, 87, -45, 57, -15, -6, 14, -8, -99, 84, -45, 63, -19, -109, 67, -94, -38, ConstantPoolEntry.CP_InterfaceMethodref}, new byte[]{109, -116, -12, 34, -70, TarConstants.LF_GNUTYPE_LONGLINK, -108, -69});
        C2344WWWWWWWW.m13703WWWWoWWWWo(m3293WWWW, view, vMInstance, z11);
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWoᕭWWWWoࢨᕭ */
    public final boolean mo3262WWWWoWWWWo(MenuItem menuItem) {
        byte[] bArr = {98, -96, 91, TarConstants.LF_DIR};
        byte[] bArr2 = {ConstantPoolEntry.CP_InterfaceMethodref, -44, 62, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_GNUTYPE_LONGNAME, -125, -47, -57};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(menuItem, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        int itemId = menuItem.getItemId();
        if (itemId == R.id.create_vm) {
            AnalyticsKt.getAnalytics(Firebase.INSTANCE).logEvent(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{91, -119, -55, 44, -38, 104, -24, -20, 93, -124, -44, 42, -18, 65, -26}, new byte[]{56, -27, -96, 79, -79, TarConstants.LF_CONTIG, -117, -98}), new ParametersBuilder().getBundle());
            m3279WWWWWWWW(new Intent(m3287WWoWWo(), VMCreateActivity.class));
            return true;
        } else if (itemId == R.id.backup_restore) {
            AnalyticsKt.getAnalytics(Firebase.INSTANCE).logEvent(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{99, 118, 110, -121, 79, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -5, 30, 99, 113, 114, -108, 123, 85, -4, ConstantPoolEntry.CP_NameAndType, 116, 117, 117, -127}, new byte[]{0, 26, 7, -28, 36, 39, -103, Byte.MAX_VALUE}), new ParametersBuilder().getBundle());
            m3279WWWWWWWW(new Intent(m3287WWoWWo(), VMBackupRestoreActivity.class));
            return true;
        } else if (itemId == R.id.layout_mode) {
            WWWWoWWWWo m4981WWWoWWWo = m4981WWWoWWWo();
            AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(AbstractC1071WWWWWWWW.m3515WWWWWWWW(m4981WWWoWWWo), null, new C1615WWWWWWWW(m4981WWWoWWWo, null), 3);
            return true;
        } else if (itemId == R.id.floating_window) {
            AnalyticsKt.getAnalytics(Firebase.INSTANCE).logEvent(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, -125, -116, -78, 16, -80, 15, 72, 59, -114, -111, -72, 21, -120, TarConstants.LF_FIFO, TarConstants.LF_GNUTYPE_SPARSE, 61, -127, -127, -66, ConstantPoolEntry.CP_NameAndType}, new byte[]{84, -17, -27, -47, 123, -17, 105, 36}), new ParametersBuilder().getBundle());
            m3279WWWWWWWW(new Intent(m3287WWoWWo(), SettingsActivity.class));
            return true;
        } else if (itemId == R.id.touch_mode) {
            WWWWoWWWWo m4981WWWoWWWo2 = m4981WWWoWWWo();
            AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(AbstractC1071WWWWWWWW.m3515WWWWWWWW(m4981WWWoWWWo2), null, new C1616WWWWWWWW(m4981WWWoWWWo2, null), 3);
            return true;
        } else if (itemId == R.id.preview_mode) {
            boolean isChecked = menuItem.isChecked();
            C2320WWWWWWWW c2320wwwwwwww = new C2320WWWWWWWW(!isChecked, this);
            if (isChecked) {
                c2320wwwwwwww.invoke();
                return true;
            }
            WWWWoWWWWo.WWWWWWWW wwwwwwww = this.f36442l;
            if (wwwwwwww != null && wwwwwwww.f8688WWWoWWWo) {
                if (wwwwwwww.f8683WWWWWWWW) {
                    c2320wwwwwwww.invoke();
                    return true;
                }
                WWWWoWWWWo m4981WWWoWWWo3 = m4981WWWoWWWo();
                AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(AbstractC1071WWWWWWWW.m3515WWWWWWWW(m4981WWWoWWWo3), null, new WWoWWo(m4981WWWoWWWo3, null), 3);
                m4985o(c2320wwwwwwww);
                return true;
            }
            m4985o(c2320wwwwwwww);
            return true;
        } else if (itemId != R.id.advanced_options) {
            return false;
        } else {
            m3279WWWWWWWW(new Intent(m3287WWoWWo(), SettingsActivity.class));
            return true;
        }
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public final void mo4975WWWWWWWW(View view, VMInstance vMInstance) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-106, -88, 123, -64}, new byte[]{-32, -63, 30, -73, -96, 61, 59, 40});
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-43, -79}, new byte[]{-93, -36, 22, 27, -40, -97, 45, 14}));
        if (vMInstance.f8940WWoWWo > 0) {
            AbstractC3585WWoWWo.m16061WWWWoWWWWo(true);
            FragmentActivity m3293WWWW = m3293WWWW();
            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{94, -118, 63, -16, -63, -32, -96, 99, 79, -101, 39, -13, -63, -26, -68, 10, 2, -63, 96, -84}, new byte[]{44, -17, 78, -123, -88, -110, -59, 34});
            C2344WWWWWWWW.m13706WWWoWWWo(m3293WWWW, vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-68, 47, TarConstants.LF_FIFO, -6, -15, 26, -6, 98}, new byte[]{-49, 71, 67, -114, -107, 117, -115, ConstantPoolEntry.CP_NameAndType}));
            return;
        }
        AbstractC3585WWoWWo.m16061WWWWoWWWWo(false);
        int[] iArr = C4248WWWoWWWo.f34225WWWWWWWW;
        C4248WWWoWWWo m17236WWWoWWWo = C4248WWWoWWWo.m17236WWWoWWWo(view, view.getResources().getText(R.string.vm_already_stopped_tips), -1);
        m17236WWWoWWWo.m17231WWWWWWWW(view);
        m17236WWWoWWWo.m17237WWWWWWWW();
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WWWWآWWWWȫآ  reason: contains not printable characters */
    public final void mo4976WWWWWWWW(VMInstance vMInstance, m3.WWWoWWWo wWWoWWWo) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-127, -88, 13, 4, -84}, new byte[]{-28, -34, 104, 106, -40, -32, -120, -123});
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_SYMLINK, 17}, new byte[]{68, 124, 6, -40, 17, -82, -70, 118}));
        FragmentActivity m3293WWWW = m3293WWWW();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{98, 114, -108, -91, 15, -79, -98, -100, 115, 99, -116, -90, 15, -73, -126, -11, 62, 57, -53, -7}, new byte[]{16, 23, -27, -48, 102, -61, -5, -35});
        C0962WWWoWWWo.m3113WWWWWWWW(m3293WWWW, vMInstance, wWWoWWWo);
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WWWWެWWWWܕެ  reason: contains not printable characters */
    public final void mo4977WWWWWWWW(View view, VMInstance vMInstance) {
        byte[] bArr = {-47, -34, -48, 4, 85, 79, TarConstants.LF_CHR, -8};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-89, -73, -75, 115}, bArr);
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_SYMLINK, -97}, new byte[]{68, -14, 111, -124, 21, -106, -5, 65}));
        if (vMInstance.f8940WWoWWo >= 7) {
            AbstractC3585WWoWWo.m16110WWoWWo(true, !vMInstance.m5062WWWWWWWW());
            AudioService audioService = vMInstance.f8944WWWW;
            if (audioService != null) {
                audioService.toggleMute();
                return;
            }
            return;
        }
        AbstractC3585WWoWWo.m16110WWoWWo(false, !vMInstance.m5062WWWWWWWW());
        int[] iArr = C4248WWWoWWWo.f34225WWWWWWWW;
        C4248WWWoWWWo m17236WWWoWWWo = C4248WWWoWWWo.m17236WWWoWWWo(view, view.getResources().getText(R.string.vm_not_started_tips), -1);
        m17236WWWoWWWo.m17231WWWWWWWW(view);
        m17236WWWoWWWo.m17237WWWWWWWW();
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWᐡWWWWೱᐡ */
    public final View mo2047WWWWWWWW(LayoutInflater layoutInflater, ViewGroup viewGroup, Bundle bundle) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(layoutInflater, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-92, -27, 123, -126, 29, -124, -35, -20}, new byte[]{-51, -117, 29, -18, 124, -16, -72, -98}));
        View inflate = layoutInflater.inflate(R.layout.fragment_vm, viewGroup, false);
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
        TypedArray obtainStyledAttributes = m3293WWWW().getTheme().obtainStyledAttributes(new int[]{R.attr.colorPrimary, R.attr.colorOnSurfaceVariant, 16842801});
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(obtainStyledAttributes, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-127, -113, 101, -106, 71, -64, -80, -99, -105, -127, 116, -109, 111, -38, -105, -101, -121, -113, 100, -125, TarConstants.LF_GNUTYPE_LONGLINK, -35, -53, -57, -64, -61, 56}, new byte[]{-18, -19, 17, -9, 46, -82, -29, -23}));
        this.f36438h = obtainStyledAttributes.getColor(0, 0);
        int color = obtainStyledAttributes.getColor(1, 0);
        this.f36439i = color;
        this.f36440j = p022WWWWWWWW.WWWWoWWWWo.m1041WWWWWWWW(color, 143);
        this.f36441k = obtainStyledAttributes.getColor(2, 0);
        obtainStyledAttributes.recycle();
        View findViewById = inflate.findViewById(R.id.pullTipsView);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-87, 107, 15, -25, -85, -57, Byte.MAX_VALUE, 44, -115, 123, 40, -25, -43, Byte.MIN_VALUE, TarConstants.LF_BLK, 117, -26}, new byte[]{-49, 2, 97, -125, -3, -82, 26, 91}));
        this.f36431a = findViewById;
        findViewById.findViewById(R.id.pullCloseView).setOnClickListener(new View$OnClickListenerC3244WWWoWWWo(this, 0));
        View findViewById2 = inflate.findViewById(R.id.releaseTipsView);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-109, -103, 68, -7, TarConstants.LF_CHR, 111, 14, -89, -73, -119, 99, -7, TarConstants.LF_MULTIVOLUME, 40, 69, -2, -36}, new byte[]{-11, -16, 42, -99, 101, 6, 107, -48}));
        this.f8656WWWWWWWW = findViewById2;
        View findViewById3 = inflate.findViewById(R.id.emptyView);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-123, 15, 43, 9, 28, -105, -126, -49, -95, 31, ConstantPoolEntry.CP_NameAndType, 9, 98, -48, -55, -106, -54}, new byte[]{-29, 102, 69, 109, 74, -2, -25, -72}));
        this.f8655WWWWWWWW = (VMEmptyCardView) findViewById3;
        View findViewById4 = inflate.findViewById(R.id.recyclerview);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById4, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-115, -81, TarConstants.LF_SYMLINK, 34, -36, 124, -30, 19, -87, -65, 21, 34, -94, 59, -87, 74, -62}, new byte[]{-21, -58, 92, 70, -118, 21, -121, 100}));
        RecyclerView recyclerView = (RecyclerView) findViewById4;
        this.f8654WWWWWWWW = recyclerView;
        recyclerView.setHasFixedSize(true);
        RecyclerView recyclerView2 = this.f8654WWWWWWWW;
        if (recyclerView2 != null) {
            recyclerView2.setBackgroundColor(this.f36441k);
            RecyclerView recyclerView3 = this.f8654WWWWWWWW;
            if (recyclerView3 != null) {
                recyclerView3.setEdgeEffectFactory(new WWWW(this));
                C3245WWWoWWWo c3245WWWoWWWo = new C3245WWWoWWWo();
                this.f36434d = c3245WWWoWWWo;
                RecyclerView recyclerView4 = this.f8654WWWWWWWW;
                if (recyclerView4 != null) {
                    recyclerView4.f5934WWWWWWWW.add(c3245WWWoWWWo);
                    this.f36435e = new C1192WWoWWo();
                    this.f36436f = new C1155WWWWWWWW(new C3248WWoWWo(this));
                    View findViewById5 = inflate.findViewById(R.id.install);
                    AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById5, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-16, -121, -71, 90, -71, 79, 122, -50, -44, -105, -98, 90, -57, 8, TarConstants.LF_LINK, -105, -65}, new byte[]{-106, -18, -41, 62, -17, 38, 31, -71}));
                    this.f36432b = (ExtendedFloatingActionButton) findViewById5;
                    p1.m11605WoWo(this).m16413WWWWWWWW(this);
                    return inflate;
                }
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-122, -113, 115, -25, -108, -97, 90, 126, -103, -117, Byte.MAX_VALUE, -31, -102}, new byte[]{-21, -35, 22, -124, -19, -4, TarConstants.LF_FIFO, 27}));
                throw null;
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-8, -17, 101, -27, -17, -78, 31, -57, -25, -21, 105, -29, -31}, new byte[]{-107, -67, 0, -122, -106, -47, 115, -94}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{3, -109, -126, -33, 41, TarConstants.LF_CONTIG, 69, 69, 28, -105, -114, -39, 39}, new byte[]{110, -63, -25, -68, 80, 84, 41, 32}));
        throw null;
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWᢎWWWWယᢎ */
    public final void mo2048WWWWWWWW(Bundle bundle, View view) {
        i0.WWWWWWWW.m14505WWWWoWWWWo(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{35, -70, 107, -78}, new byte[]{85, -45, 14, -59, -12, 7, -66, 66}, view);
        this.f36442l = null;
        AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(ib.WWWWoWWWWo.m14598WWWWWWWW(this), null, new C3247WWoWWo(this, view, null), 3);
    }

    /* renamed from: WWWWệWWWW֙ệ  reason: contains not printable characters */
    public final void m4978WWWWWWWW() {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        Intent intent = new Intent(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-27, 8, 122, -81, -20, 70, 90, -111, -19, 8, 106, -72, -19, 91, 16, -34, -25, 18, 119, -78, -19, 1, 115, -2, -51, 40}, new byte[]{-124, 102, 30, -35, -125, 47, 62, -65})).addCategory(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_FIFO, -96, 58, -49, 60, -39, -76, 29, 62, -96, 42, -40, 61, -60, -2, 80, TarConstants.LF_FIFO, -70, 59, -38, 60, -62, -87, 29, 19, -117, 24, -4, 6, -4, -124}, new byte[]{87, -50, 94, -67, TarConstants.LF_GNUTYPE_SPARSE, -80, -48, TarConstants.LF_CHR})).setPackage(m3287WWoWWo().getPackageName());
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(intent, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{57, 126, -31, -120, -68, TarConstants.LF_NORMAL, 112, -62, 45, 126, -67, -10, -13, 125, TarConstants.LF_SYMLINK}, new byte[]{74, 27, -107, -40, -35, TarConstants.LF_GNUTYPE_SPARSE, 27, -93}));
        FragmentActivity m3293WWWW = m3293WWWW();
        String m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-52, -15, -73, -7, 33, -45, -10}, new byte[]{-91, -97, -60, -115, 64, -67, -126, -115});
        if (!t8.WWWWWWWW.m17201WWWWWWWW(m3293WWWW).m4644WWWWoWWWWo()) {
            return;
        }
        Uri.Builder appendQueryParameter = new Uri.Builder().scheme("market").authority("details").appendQueryParameter("id", m3293WWWW.getPackageName());
        if (!TextUtils.isEmpty(m17835WWWWWWWW)) {
            appendQueryParameter.appendQueryParameter("referrer", m17835WWWWWWWW);
        }
        Intent intent2 = new Intent("com.google.android.finsky.action.IA_INSTALL").setData(appendQueryParameter.build()).setPackage("com.android.vending");
        intent2.putExtra("postInstallIntent", intent);
        if (m3293WWWW.getPackageManager().resolveActivity(intent2, 0) != null) {
            m3293WWWW.startActivityForResult(intent2, -1);
            return;
        }
        Intent putExtra = new Intent("android.intent.action.VIEW").setPackage("com.android.vending").addCategory("android.intent.category.DEFAULT").putExtra("callerId", m3293WWWW.getPackageName()).putExtra("overlay", true);
        Uri.Builder appendQueryParameter2 = new Uri.Builder().scheme("market").authority("details").appendQueryParameter("id", m3293WWWW.getPackageName());
        if (!TextUtils.isEmpty(m17835WWWWWWWW)) {
            appendQueryParameter2.appendQueryParameter("referrer", m17835WWWWWWWW);
        }
        putExtra.setData(appendQueryParameter2.build());
        m3293WWWW.startActivityForResult(putExtra, -1);
    }

    /* renamed from: WWWWỖWWWWࢥỖ  reason: contains not printable characters */
    public final void m4979WWWWWWWW(Menu menu, int i10, boolean z10, int i11) {
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
                        drawable.setColorFilter(new PorterDuffColorFilter(this.f36440j, PorterDuff.Mode.SRC_ATOP));
                    }
                    findItem2.setIcon(drawable);
                    findItem2.setTitle(m3290WW(R.string.vm_tab_menu_touch));
                } else if (i13 == 1) {
                    Drawable drawable2 = m3287WWoWWo().getDrawable(R.drawable.outline_trackpad_input_24);
                    if (drawable2 != null) {
                        drawable2.setColorFilter(new PorterDuffColorFilter(this.f36439i, PorterDuff.Mode.SRC_ATOP));
                    }
                    findItem2.setIcon(drawable2);
                    findItem2.setTitle(m3290WW(R.string.vm_tab_menu_touch_passthrough));
                } else {
                    Drawable drawable3 = m3287WWoWWo().getDrawable(R.drawable.outline_trackpad_input_stack_24);
                    if (drawable3 != null) {
                        drawable3.setColorFilter(new PorterDuffColorFilter(this.f36438h, PorterDuff.Mode.SRC_ATOP));
                    }
                    findItem2.setIcon(drawable3);
                    findItem2.setTitle(m3290WW(R.string.vm_tab_menu_touch_sync));
                }
            }
            MenuItem findItem3 = menu.findItem(R.id.preview_mode);
            if (findItem3 != null) {
                findItem3.setChecked(z10);
            }
        }
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final void mo4980WWWoWWWo(View view, VMInstance vMInstance) {
        boolean z10 = false;
        byte[] bArr = {118, 121, -106, -106, ConstantPoolEntry.CP_InterfaceMethodref, 2, 80, -4};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{0, 16, -13, -31}, bArr);
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-78, -84}, new byte[]{-60, -63, 67, 110, -118, -118, -7, 2}));
        if (vMInstance.f8940WWoWWo >= 7) {
            z10 = true;
        }
        FirebaseAnalytics analytics = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
        String m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, 74, 110, 99, 35, -50, 13, -49, -84, 85, 98, 116, 60, -8, 21, -59, -84, 79, 105, 95, 60, -16, 25}, new byte[]{-13, 38, 7, 0, 72, -111, 123, -94});
        ParametersBuilder parametersBuilder = new ParametersBuilder();
        parametersBuilder.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, -104, 7, TarConstants.LF_SYMLINK, 97, -127, 26}, new byte[]{-58, -20, 102, 64, 21, -28, 126, -11}), String.valueOf(z10));
        analytics.logEvent(m17835WWWWWWWW, parametersBuilder.getBundle());
        Intent intent = new Intent(m3266WWWWWWWW(), VMSettingsActivity.class);
        intent.putExtra(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, 10, 40, -67, 2}, new byte[]{58, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 119, -44, 102, 5, 16, 125}), vMInstance.f8937WWWoWWWo.f8866WWWWWWWW);
        m3293WWWW().startActivity(intent);
    }

    /* renamed from: WWWoễWWWoಇễ  reason: contains not printable characters */
    public final WWWWoWWWWo m4981WWWoWWWo() {
        return (WWWWoWWWWo) this.f36443m.getValue();
    }

    @Override // p.InterfaceC3766WWWWoWWWWo
    /* renamed from: WWoॹWWoࠔॹ  reason: contains not printable characters */
    public final void mo4982WWoWWo(C3784WWWWWWWW c3784wwwwwwww, AbstractC3783WWWWWWWW abstractC3783WWWWWWWW, Bundle bundle) {
        ViewGroup viewGroup;
        View view;
        int i10;
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(c3784wwwwwwww, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-54, 39, -51, 105, -33, -83, 78, -83, -52, 58}, new byte[]{-87, 72, -93, 29, -83, -62, 34, -63}));
        AbstractC3339WWWWWWWW.m15439WWoWWo(abstractC3783WWWWWWWW, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{70, -81, 91, -8, -115, -123, 117, 87, TarConstants.LF_GNUTYPE_LONGLINK, -91, 70}, new byte[]{34, -54, 40, -116, -28, -21, 20, 35}));
        RecyclerView recyclerView = this.f8654WWWWWWWW;
        if (recyclerView != null) {
            int childCount = recyclerView.getChildCount();
            for (int i11 = 0; i11 < childCount; i11++) {
                View childAt = recyclerView.getChildAt(i11);
                if (childAt instanceof ViewGroup) {
                    viewGroup = (ViewGroup) childAt;
                } else {
                    viewGroup = null;
                }
                if (viewGroup != null) {
                    view = viewGroup.getChildAt(0);
                } else {
                    view = null;
                }
                if ((view instanceof VMBigPreviewCardView) || (view instanceof VMSmallPreviewCardView)) {
                    if (abstractC3783WWWWWWWW.f32045WWWWWWWWWW.f4164WWWWWWWW != this.f5315WW) {
                        i10 = 4;
                    } else {
                        i10 = 0;
                    }
                    view.setVisibility(i10);
                }
            }
            return;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-87, 9, 40, 63, 25, 35, 106, -96, -74, 13, 36, 57, 23}, new byte[]{-60, 91, TarConstants.LF_MULTIVOLUME, 92, 96, 64, 6, -59}));
        throw null;
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WWoহWWoȗহ  reason: contains not printable characters */
    public final void mo4983WWoWWo(VMInstance vMInstance, PermissionEvent permissionEvent) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{69, 23, 32, -55, -15}, new byte[]{32, 97, 69, -89, -123, 107, 115, 124});
        AbstractC3339WWWWWWWW.m15439WWoWWo(vMInstance, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{9, 58}, new byte[]{Byte.MAX_VALUE, 87, 32, 81, -89, 78, 101, -69}));
        FragmentActivity m3293WWWW = m3293WWWW();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-69, -68, -3, TarConstants.LF_GNUTYPE_LONGNAME, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -20, -13, -82, -86, -83, -27, 79, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -22, -17, -57, -25, -9, -94, 16}, new byte[]{-55, -39, -116, 57, 17, -98, -106, -17});
        C2344WWWWWWWW.m13703WWWWoWWWWo(m3293WWWW, this.f5289WWWWWWWW, vMInstance, false);
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWoᐛWWoʄᐛ */
    public final void mo3285WWoWWo(Menu menu, MenuInflater menuInflater) {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(menu, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{46, -22, -39, -19}, new byte[]{67, -113, -73, -104, -12, -44, -96, 44}));
        AbstractC3339WWWWWWWW.m15439WWoWWo(menuInflater, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-84, 34, 46, -119, 7, -58, -44, -49}, new byte[]{-59, TarConstants.LF_GNUTYPE_LONGNAME, 72, -27, 102, -78, -79, -67}));
        menuInflater.inflate(R.menu.vm_tab_menu, menu);
        WWWWoWWWWo.WWWWWWWW wwwwwwww = this.f36442l;
        if (wwwwwwww != null) {
            m4979WWWWWWWW(menu, wwwwwwww.f8680WWWWoWWWWo, wwwwwwww.f8682WWWWWWWW, wwwwwwww.f8687WWWoWWWo);
        }
        this.f36437g = menu;
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWᐤԂᐤ */
    public final void mo3292WW() {
        this.f5273WWWWoWWWWo = true;
        p1.m11605WoWo(this).m16414WWWWWWWW(this);
    }

    @Override // k4.InterfaceC3250WWoWWo
    /* renamed from: WoڄWoᄴڄ  reason: contains not printable characters */
    public final void mo4984WoWo() {
        FragmentActivity m3265WWWWWWWW = m3265WWWWWWWW();
        View view = this.f5289WWWWWWWW;
        WWWWoWWWWo.WWWWWWWW wwwwwwww = this.f36442l;
        if ((wwwwwwww == null || !wwwwwwww.f8688WWWoWWWo) && m3265WWWWWWWW != null && view != null) {
            int[] iArr = C4248WWWoWWWo.f34225WWWWWWWW;
            C4248WWWoWWWo m17236WWWoWWWo = C4248WWWoWWWo.m17236WWWoWWWo(view, view.getResources().getText(R.string.drag_to_reorder_vm_not_unlocked_tips), 0);
            BaseTransientBottomBar$SnackbarBaseLayout baseTransientBottomBar$SnackbarBaseLayout = m17236WWWoWWWo.f34210WWWoWWWo;
            ((SnackbarContentLayout) baseTransientBottomBar$SnackbarBaseLayout.getChildAt(0)).getMessageView().setMaxLines(3);
            WWoWWo wWoWWo = new WWoWWo(8, this, m3265WWWWWWWW);
            CharSequence text = m17236WWWoWWWo.f34203WWWWWWWW.getText(R.string.dialog_button_purchase);
            Button actionView = ((SnackbarContentLayout) baseTransientBottomBar$SnackbarBaseLayout.getChildAt(0)).getActionView();
            if (!TextUtils.isEmpty(text)) {
                m17236WWWoWWWo.f34227WWoWWo = true;
                actionView.setVisibility(0);
                actionView.setText(text);
                actionView.setOnClickListener(new WWoWWo(18, m17236WWWoWWWo, wWoWWo));
            } else {
                actionView.setVisibility(8);
                actionView.setOnClickListener(null);
                m17236WWWoWWWo.f34227WWoWWo = false;
            }
            if (m3281WWWoWWWo().getConfiguration().orientation == 1) {
                m17236WWWoWWWo.m17231WWWWWWWW(m3265WWWWWWWW.findViewById(R.id.nav_bar));
            } else {
                m17236WWWoWWWo.m17231WWWWWWWW(null);
            }
            m17236WWWoWWWo.m17237WWWWWWWW();
        }
        m4981WWWoWWWo();
        k4.WWWWWWWW wwwwwwww2 = this.f36433c;
        if (wwwwwwww2 != null) {
            List m14886WWoWWo = wwwwwwww2.m14886WWoWWo();
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-53, -20, -32, -123}, new byte[]{-89, -123, -109, -15, -101, -83, -68, -25});
            C3455WWWWWWWW c3455wwwwwwww = C3403WWWWWWWW.f30491WWWWWWWW;
            x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{56, 27, -52, 35, 123, 115}, new byte[]{78, 118, Byte.MIN_VALUE, 74, 8, 7, 59, -72});
            AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(C2427WWWWWWWW.f26690WWWWoWWWWo, C3403WWWWWWWW.f30491WWWWWWWW, new C3379WWWWoWWWWo(m14886WWoWWo, null), 2);
            return;
        }
        byte[] bArr = {59, TarConstants.LF_GNUTYPE_LONGNAME, -35, TarConstants.LF_NORMAL, -53, 5, -113, -81};
        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{86, 13, -71, 81, -69, 113, -22, -35}, bArr);
        throw null;
    }

    /* renamed from: oỈɨỈ  reason: contains not printable characters */
    public final void m4985o(C2320WWWWWWWW c2320wwwwwwww) {
        da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(m3293WWWW());
        wWWWoWWWWo.m13648WoWo(R.string.vm_tab_menu_preview_mode);
        View inflate = LayoutInflater.from(m3287WWoWWo()).inflate(R.layout.dialog_vm_preview_mode_feature, (ViewGroup) null);
        Context m3266WWWWWWWW = m3266WWWWWWWW();
        u5.WWoWWo.m17451WWWoWWWo(m3266WWWWWWWW, "You cannot start a load on a not yet attached View or a Fragment where getActivity() returns null (which usually occurs when getActivity() is called before the Fragment is attached or after the Fragment is destroyed).");
        com.bumptech.glide.WWWWoWWWWo.m5381WWWWWWWW(m3266WWWWWWWW).f9428WWWWWWWW.m16044WWWWWWWW(this).mo5468WWoWWo().mo5399WWWW(Integer.valueOf((int) R.raw.vm_layout_preview_mode)).m5391WWWWWWWW((ImageView) inflate.findViewById(R.id.feature_image));
        wWWWoWWWWo.m13644WWWWWWWW(inflate);
        ((Button) inflate.findViewById(16908313)).setOnClickListener(new WWoWWo(9, c2320wwwwwwww, wWWWoWWWWo.m741WWoWWo()));
    }
}
