package com.android.vmapp.ui.vm.main;

import android.view.View;
import androidx.appcompat.widget.Toolbar;
import androidx.recyclerview.widget.RecyclerView;
import com.android.vmapp.ui.vm.main.WWWWoWWWWo;
import com.clone.android.dual.space.R;
import com.google.android.material.floatingactionbutton.ExtendedFloatingActionButton;
import com.google.firebase.Firebase;
import com.google.firebase.analytics.AnalyticsKt;
import com.google.firebase.analytics.FirebaseAnalytics;
import com.google.firebase.analytics.ParametersBuilder;
import fc.WWWWWWWWWW;
import j3.C3164WWWWWWWW;
import java.util.List;
import jc.InterfaceC3180WWWWWWWW;
import k4.C3245WWWoWWWo;
import k4.RunnableC3238WWWWWWWW;
import k4.View$OnClickListenerC3244WWWoWWWo;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import lc.AbstractC3453WWWWWWWW;
import me.AbstractC3506WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import tc.InterfaceC4270WWWWWWWW;
/* renamed from: com.android.vmapp.ui.vm.main.WWWW̏WWWWβ̏  reason: invalid class name */
/* loaded from: classes.dex */
public final class WWWWWWWW extends AbstractC3453WWWWWWWW implements InterfaceC4270WWWWWWWW {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final /* synthetic */ VMFragment f8690WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public /* synthetic */ Object f8691WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final /* synthetic */ View f8692WWWWWWWW;

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public WWWWWWWW(VMFragment vMFragment, View view, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        super(2, interfaceC3180WWWWWWWW);
        this.f8690WWWWWWWWWW = vMFragment;
        this.f8692WWWWWWWW = view;
    }

    @Override // lc.WWWWWWWW
    public final InterfaceC3180WWWWWWWW create(Object obj, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        WWWWWWWW wwwwwwww = new WWWWWWWW(this.f8690WWWWWWWWWW, this.f8692WWWWWWWW, interfaceC3180WWWWWWWW);
        wwwwwwww.f8691WWWWoWWWWo = obj;
        return wwwwwwww;
    }

    @Override // tc.InterfaceC4270WWWWWWWW
    public final Object invoke(Object obj, Object obj2) {
        WWWWWWWWWW wwwwwwwwww = WWWWWWWWWW.f27054WWWWWWWW;
        ((WWWWWWWW) create((WWWWoWWWWo.WWWWWWWW) obj, (InterfaceC3180WWWWWWWW) obj2)).invokeSuspend(wwwwwwwwww);
        return wwwwwwwwww;
    }

    @Override // lc.WWWWWWWW
    public final Object invokeSuspend(Object obj) {
        boolean z10;
        boolean z11;
        Throwable th2;
        String m17835WWWWWWWW;
        int i10;
        Toolbar toolbar;
        View findViewById;
        List list;
        WWWWoWWWWo.WWWWWWWW wwwwwwww = (WWWWoWWWWo.WWWWWWWW) this.f8691WWWWoWWWWo;
        kc.WWWWWWWW wwwwwwww2 = kc.WWWWWWWW.f30085WWWWoWWWWo;
        AbstractC3506WWWWWWWW.m15930WWWWWWWW(obj);
        int i11 = wwwwwwww.f8680WWWWoWWWWo;
        WWWWWWWWWW wwwwwwwwww = WWWWWWWWWW.f27054WWWWWWWW;
        if (i11 == -1) {
            return wwwwwwwwww;
        }
        VMFragment vMFragment = this.f8690WWWWWWWWWW;
        C3245WWWoWWWo c3245WWWoWWWo = vMFragment.f36434d;
        if (c3245WWWoWWWo != null) {
            int i12 = i11 % 3;
            int i13 = wwwwwwww.f8687WWWoWWWo;
            if (i12 != 2 && i13 % 3 != 0) {
                z10 = true;
            } else {
                z10 = false;
            }
            c3245WWWoWWWo.f29289WWWWWWWW = z10;
            WWWWoWWWWo.WWWWWWWW wwwwwwww3 = vMFragment.f36442l;
            boolean z12 = wwwwwwww.f8685WWWWWWWW;
            List list2 = wwwwwwww.f8686WWWWWWWW;
            boolean z13 = wwwwwwww.f8682WWWWWWWW;
            if (wwwwwwww3 != null && i11 == wwwwwwww3.f8680WWWWoWWWWo) {
                if (wwwwwwww3 == null || z13 != wwwwwwww3.f8682WWWWWWWW) {
                    k4.WWWWWWWW wwwwwwww4 = vMFragment.f36433c;
                    if (wwwwwwww4 != null) {
                        wwwwwwww4.f29266WWoWWo = z13;
                        wwwwwwww4.f5980WWWWWWWW.m3938WWWWWWWW(0, wwwwwwww4.f29265WWWoWWWo.size(), null);
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-106, 2, -39, TarConstants.LF_BLK, 41, 14, 34, -73}, new byte[]{-5, 67, -67, 85, 89, 122, 71, -59});
                        throw null;
                    }
                }
                WWWWoWWWWo.WWWWWWWW wwwwwwww5 = vMFragment.f36442l;
                if (wwwwwwww5 == null || i13 != wwwwwwww5.f8687WWWoWWWo) {
                    k4.WWWWWWWW wwwwwwww6 = vMFragment.f36433c;
                    if (wwwwwwww6 != null) {
                        wwwwwwww6.f29263WWWWWWWW = i13;
                        wwwwwwww6.f5980WWWWWWWW.m3938WWWWWWWW(0, wwwwwwww6.f29265WWWoWWWo.size(), null);
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-71, 40, -122, -2, -3, -111, -39, 119}, new byte[]{-44, 105, -30, -97, -115, -27, -68, 5});
                        throw null;
                    }
                }
                WWWWoWWWWo.WWWWWWWW wwwwwwww7 = vMFragment.f36442l;
                if (wwwwwwww7 == null || z12 != wwwwwwww7.f8685WWWWWWWW) {
                    k4.WWWWWWWW wwwwwwww8 = vMFragment.f36433c;
                    if (wwwwwwww8 != null) {
                        wwwwwwww8.f29264WWWWWWWW = z12;
                        wwwwwwww8.f5980WWWWWWWW.m3938WWWWWWWW(0, wwwwwwww8.f29265WWWoWWWo.size(), null);
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-52, -124, -120, -96, 111, -87, -69, -40}, new byte[]{-95, -59, -20, -63, 31, -35, -34, -86});
                        throw null;
                    }
                }
                WWWWoWWWWo.WWWWWWWW wwwwwwww9 = vMFragment.f36442l;
                if (wwwwwwww9 != null) {
                    list = wwwwwwww9.f8686WWWWWWWW;
                } else {
                    list = null;
                }
                if (!AbstractC3339WWWWWWWW.m15427WWWWWWWW(list2, list)) {
                    k4.WWWWWWWW wwwwwwww10 = vMFragment.f36433c;
                    if (wwwwwwww10 != null) {
                        wwwwwwww10.m14885WWWWWWWW(list2);
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-47, 70, 30, 93, 70, 20, -89, TarConstants.LF_BLK}, new byte[]{-68, 7, 122, 60, TarConstants.LF_FIFO, 96, -62, 70});
                        throw null;
                    }
                }
            } else {
                if (wwwwwwww3 != null) {
                    z11 = true;
                } else {
                    z11 = false;
                }
                VMFragment.m4973WWWWWW(vMFragment, z11, i11);
                k4.WWWWWWWW wwwwwwww11 = vMFragment.f36433c;
                if (wwwwwwww11 != null) {
                    wwwwwwww11.f29266WWoWWo = z13;
                    wwwwwwww11.f5980WWWWWWWW.m3938WWWWWWWW(0, wwwwwwww11.f29265WWWoWWWo.size(), null);
                    k4.WWWWWWWW wwwwwwww12 = vMFragment.f36433c;
                    if (wwwwwwww12 != null) {
                        wwwwwwww12.f29263WWWWWWWW = i13;
                        wwwwwwww12.f5980WWWWWWWW.m3938WWWWWWWW(0, wwwwwwww12.f29265WWWoWWWo.size(), null);
                        k4.WWWWWWWW wwwwwwww13 = vMFragment.f36433c;
                        if (wwwwwwww13 != null) {
                            wwwwwwww13.f29264WWWWWWWW = z12;
                            wwwwwwww13.f5980WWWWWWWW.m3938WWWWWWWW(0, wwwwwwww13.f29265WWWoWWWo.size(), null);
                            k4.WWWWWWWW wwwwwwww14 = vMFragment.f36433c;
                            if (wwwwwwww14 != null) {
                                wwwwwwww14.m14885WWWWWWWW(list2);
                            } else {
                                i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{25, 2, TarConstants.LF_GNUTYPE_LONGLINK, -82, 126, 99, -69, -21}, new byte[]{116, 67, 47, -49, 14, 23, -34, -103});
                                throw null;
                            }
                        } else {
                            i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{72, -92, 32, 27, -4, -16, -14, -116}, new byte[]{37, -27, 68, 122, -116, -124, -105, -2});
                            throw null;
                        }
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-99, ConstantPoolEntry.CP_NameAndType, 116, TarConstants.LF_NORMAL, -121, -59, -95, -20}, new byte[]{-16, TarConstants.LF_MULTIVOLUME, 16, 81, -9, -79, -60, -98});
                        throw null;
                    }
                } else {
                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{101, -87, -2, -49, -40, -110, 100, -127}, new byte[]{8, -24, -102, -82, -88, -26, 1, -13});
                    throw null;
                }
            }
            WWWWoWWWWo.WWWWWWWW wwwwwwww15 = vMFragment.f36442l;
            if (wwwwwwww15 != null) {
                int size = list2.size();
                List list3 = wwwwwwww15.f8686WWWWWWWW;
                if (size > list3.size() && !list3.isEmpty()) {
                    RecyclerView recyclerView = vMFragment.f8654WWWWWWWW;
                    if (recyclerView != null) {
                        recyclerView.post(new RunnableC3238WWWWWWWW(vMFragment, 2));
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-119, 37, -47, 99, -25, -28, -16, -64, -106, 33, -35, 101, -23}, new byte[]{-28, 119, -76, 0, -98, -121, -100, -91});
                        throw null;
                    }
                }
            }
            k4.WWWWWWWW wwwwwwww16 = vMFragment.f36433c;
            if (wwwwwwww16 != null) {
                if (wwwwwwww16.f29265WWWoWWWo.size() == 0) {
                    VMEmptyCardView vMEmptyCardView = vMFragment.f8655WWWWWWWW;
                    if (vMEmptyCardView != null) {
                        vMEmptyCardView.setVisibility(0);
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{113, -77, TarConstants.LF_MULTIVOLUME, TarConstants.LF_FIFO, -96, -126, TarConstants.LF_DIR, -102, 121, -127}, new byte[]{28, -10, 32, 70, -44, -5, 99, -13});
                        throw null;
                    }
                } else {
                    VMEmptyCardView vMEmptyCardView2 = vMFragment.f8655WWWWWWWW;
                    if (vMEmptyCardView2 != null) {
                        vMEmptyCardView2.setVisibility(8);
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-36, -89, -74, 56, 23, 114, 20, -1, -44, -107}, new byte[]{-79, -30, -37, 72, 99, ConstantPoolEntry.CP_InterfaceMethodref, 66, -106});
                        throw null;
                    }
                }
                WWWWoWWWWo.WWWWWWWW wwwwwwww17 = vMFragment.f36442l;
                int i14 = wwwwwwww.f8680WWWWoWWWWo;
                if (wwwwwwww17 == null || i14 != wwwwwwww17.f8680WWWWoWWWWo || wwwwwwww17 == null || z13 != wwwwwwww17.f8682WWWWWWWW || wwwwwwww17 == null || i13 != wwwwwwww17.f8687WWWoWWWo) {
                    vMFragment.m4979WWWWWWWW(vMFragment.f36437g, i14, z13, i13);
                }
                WWWWoWWWWo.WWWWWWWW wwwwwwww18 = vMFragment.f36442l;
                if (wwwwwwww18 == null || (wwwwwwww18 != null && i13 == wwwwwwww18.f8687WWWoWWWo)) {
                    th2 = null;
                } else {
                    int i15 = i13 % 3;
                    if (i15 != 1) {
                        if (i15 != 2) {
                            i10 = 4;
                            byte[] bArr = {-73, 74, 86, -25, -72, ConstantPoolEntry.CP_NameAndType, -90, -113};
                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                            m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-39, 37, 56, -126}, bArr);
                            byte[] bArr2 = new byte[i10];
                            // fill-array-data instruction
                            bArr2[0] = -13;
                            bArr2[1] = -70;
                            bArr2[2] = 57;
                            bArr2[3] = 84;
                            byte[] bArr3 = {-98, -43, 93, TarConstants.LF_LINK, -100, 42, -17, -104};
                            x5.WWWWWWWW wwwwwwww19 = C3164WWWWWWWW.f28918WWWWWWWW;
                            wwwwwwww19.getClass();
                            x5.WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3);
                            FirebaseAnalytics analytics = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
                            String m17835WWWWWWWW2 = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-30, 87, -92, -119, -28, -86, TarConstants.LF_BLK, -15, -12, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -91, -75, -30, -102, 36, -5}, new byte[]{-127, 59, -51, -22, -113, -11, 64, -98});
                            ParametersBuilder parametersBuilder = new ParametersBuilder();
                            th2 = null;
                            parametersBuilder.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-36, 10, -111, 101}, new byte[]{-79, 101, -11, 0, 95, TarConstants.LF_DIR, -4, -38}), m17835WWWWWWWW);
                            analytics.logEvent(m17835WWWWWWWW2, parametersBuilder.getBundle());
                            wwwwwwww19.getClass();
                            if (!m17835WWWWWWWW.equals(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{82, -6, -92, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{60, -107, -54, 41, -31, 68, -124, 13})) && (toolbar = (Toolbar) this.f8692WWWWWWWW.findViewById(R.id.toolbar)) != null && (findViewById = toolbar.findViewById(R.id.touch_mode)) != null) {
                                findViewById.performLongClick();
                            }
                        } else {
                            byte[] bArr4 = {TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -44, -51, 31, 26, -54, -49};
                            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                            m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{43, 1, -70, -82}, bArr4);
                        }
                    } else {
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-44, -2, 59, -103, -120, 61, 3, -83, -47, -8, 32}, new byte[]{-92, -97, 72, -22, -4, 85, 113, -62});
                    }
                    i10 = 4;
                    byte[] bArr22 = new byte[i10];
                    // fill-array-data instruction
                    bArr22[0] = -13;
                    bArr22[1] = -70;
                    bArr22[2] = 57;
                    bArr22[3] = 84;
                    byte[] bArr32 = {-98, -43, 93, TarConstants.LF_LINK, -100, 42, -17, -104};
                    x5.WWWWWWWW wwwwwwww192 = C3164WWWWWWWW.f28918WWWWWWWW;
                    wwwwwwww192.getClass();
                    x5.WWWWWWWW.m17835WWWWWWWW(bArr22, bArr32);
                    FirebaseAnalytics analytics2 = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
                    String m17835WWWWWWWW22 = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-30, 87, -92, -119, -28, -86, TarConstants.LF_BLK, -15, -12, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -91, -75, -30, -102, 36, -5}, new byte[]{-127, 59, -51, -22, -113, -11, 64, -98});
                    ParametersBuilder parametersBuilder2 = new ParametersBuilder();
                    th2 = null;
                    parametersBuilder2.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-36, 10, -111, 101}, new byte[]{-79, 101, -11, 0, 95, TarConstants.LF_DIR, -4, -38}), m17835WWWWWWWW);
                    analytics2.logEvent(m17835WWWWWWWW22, parametersBuilder2.getBundle());
                    wwwwwwww192.getClass();
                    if (!m17835WWWWWWWW.equals(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{82, -6, -92, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{60, -107, -54, 41, -31, 68, -124, 13}))) {
                        findViewById.performLongClick();
                    }
                }
                if (wwwwwwww.f8681WWWWWWWW) {
                    ExtendedFloatingActionButton extendedFloatingActionButton = vMFragment.f36432b;
                    if (extendedFloatingActionButton != null) {
                        extendedFloatingActionButton.setVisibility(0);
                        ExtendedFloatingActionButton extendedFloatingActionButton2 = vMFragment.f36432b;
                        if (extendedFloatingActionButton2 != null) {
                            extendedFloatingActionButton2.setOnClickListener(new View$OnClickListenerC3244WWWoWWWo(vMFragment, 2));
                        } else {
                            i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{89, 40, -53, 80, -16, -24, 74, TarConstants.LF_CHR, 118, 20, -47, 87, -21, -25}, new byte[]{TarConstants.LF_BLK, 97, -91, 35, -124, -119, 38, 95});
                            throw th2;
                        }
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-50, 104, 43, 97, TarConstants.LF_MULTIVOLUME, 85, 4, TarConstants.LF_GNUTYPE_SPARSE, -31, 84, TarConstants.LF_LINK, 102, 86, 90}, new byte[]{-93, 33, 69, 18, 57, TarConstants.LF_BLK, 104, 63});
                        throw th2;
                    }
                } else {
                    ExtendedFloatingActionButton extendedFloatingActionButton3 = vMFragment.f36432b;
                    if (extendedFloatingActionButton3 != null) {
                        extendedFloatingActionButton3.setVisibility(8);
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-64, -114, 100, -54, 89, 47, 47, -97, -17, -78, 126, -51, 66, 32}, new byte[]{-83, -57, 10, -71, 45, 78, 67, -13});
                        throw th2;
                    }
                }
                k4.WWWWWWWW wwwwwwww20 = vMFragment.f36433c;
                if (wwwwwwww20 != null) {
                    if (wwwwwwww20.f29265WWWoWWWo.size() > 0 && !wwwwwwww.f8689WWoWWo) {
                        View view = vMFragment.f36431a;
                        if (view != null) {
                            view.setVisibility(0);
                        } else {
                            i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-40, 14, 25, 38, -54, -126, 124, -104, -58, 8, 5, 47, -47}, new byte[]{-75, 94, 108, 74, -90, -42, 21, -24});
                            throw th2;
                        }
                    } else {
                        View view2 = vMFragment.f36431a;
                        if (view2 != null) {
                            view2.setVisibility(8);
                        } else {
                            i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_SYMLINK, -101, 64, 66, -61, 66, 33, -34, 44, -99, 92, TarConstants.LF_GNUTYPE_LONGLINK, -40}, new byte[]{95, -53, TarConstants.LF_DIR, 46, -81, 22, 72, -82});
                            throw th2;
                        }
                    }
                    vMFragment.f36442l = wwwwwwww;
                    return wwwwwwwwww;
                }
                i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-12, -40, TarConstants.LF_GNUTYPE_SPARSE, 13, -53, 112, -53, -83}, new byte[]{-103, -103, TarConstants.LF_CONTIG, 108, -69, 4, -82, -33});
                throw th2;
            }
            i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{57, 25, -79, -89, -8, -38, -86, -6}, new byte[]{84, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -43, -58, -120, -82, -49, -120});
            throw null;
        }
        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-75, 32, 73, -26, -19, -18, -82, 105, -105, 30, 114, -9, -2, -22, -97, 113, -83, 19, TarConstants.LF_GNUTYPE_SPARSE, -49, -14, -12, -65, 123, -74, 21, 73}, new byte[]{-40, 112, 59, -125, -101, -121, -53, 30});
        throw null;
    }
}
