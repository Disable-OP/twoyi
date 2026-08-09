package com.android.vmapp.ui.vm.privacy;

import a3.WWWoWWWo;
import android.view.View;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.FragmentActivity;
import com.android.vmapp.ui.vm.privacy.WWWWoWWWWo;
import com.android.vmapp.ui.widget.CommonEmptyView;
import com.clone.android.dual.space.R;
import com.google.android.play.core.assetpacks.AbstractC2131WW;
import fc.WWWWWWWWWW;
import hd.C2819WWWWWWWW;
import j3.C3164WWWWWWWW;
import java.util.List;
import jc.InterfaceC3180WWWWWWWW;
import k4.C3245WWWoWWWo;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import lc.AbstractC3453WWWWWWWW;
import me.AbstractC3506WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import tc.InterfaceC4270WWWWWWWW;
/* renamed from: com.android.vmapp.ui.vm.privacy.WWWW̏WWWWβ̏  reason: invalid class name */
/* loaded from: classes.dex */
public final class WWWWWWWW extends AbstractC3453WWWWWWWW implements InterfaceC4270WWWWWWWW {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final /* synthetic */ PrivacyMainFragment f8720WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public /* synthetic */ Object f8721WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final /* synthetic */ View f8722WWWWWWWW;

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public WWWWWWWW(PrivacyMainFragment privacyMainFragment, View view, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        super(2, interfaceC3180WWWWWWWW);
        this.f8720WWWWWWWWWW = privacyMainFragment;
        this.f8722WWWWWWWW = view;
    }

    @Override // lc.WWWWWWWW
    public final InterfaceC3180WWWWWWWW create(Object obj, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        WWWWWWWW wwwwwwww = new WWWWWWWW(this.f8720WWWWWWWWWW, this.f8722WWWWWWWW, interfaceC3180WWWWWWWW);
        wwwwwwww.f8721WWWWoWWWWo = obj;
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
        C2819WWWWWWWW c2819wwwwwwww;
        Object m14479WWWWWWWW;
        String m17835WWWWWWWW;
        int i10;
        byte[] bArr;
        Toolbar toolbar;
        View findViewById;
        List list;
        WWWWoWWWWo.WWWWWWWW wwwwwwww = (WWWWoWWWWo.WWWWWWWW) this.f8721WWWWoWWWWo;
        kc.WWWWWWWW wwwwwwww2 = kc.WWWWWWWW.f30085WWWWoWWWWo;
        AbstractC3506WWWWWWWW.m15930WWWWWWWW(obj);
        PrivacyMainFragment privacyMainFragment = this.f8720WWWWWWWWWW;
        C3245WWWoWWWo c3245WWWoWWWo = privacyMainFragment.f36444a;
        if (c3245WWWoWWWo != null) {
            int i11 = wwwwwwww.f8711WWWWoWWWWo;
            int i12 = i11 % 3;
            int i13 = wwwwwwww.f8717WWWoWWWo;
            if (i12 != 2 && i13 % 3 != 0) {
                z10 = true;
            } else {
                z10 = false;
            }
            c3245WWWoWWWo.f29289WWWWWWWW = z10;
            WWWWoWWWWo.WWWWWWWW wwwwwwww3 = privacyMainFragment.f36451h;
            List list2 = wwwwwwww.f8718WWWoWWWo;
            boolean z11 = wwwwwwww.f8719WWoWWo;
            boolean z12 = wwwwwwww.f8713WWWWWWWW;
            if (wwwwwwww3 != null && i11 == wwwwwwww3.f8711WWWWoWWWWo) {
                if (wwwwwwww3 == null || z12 != wwwwwwww3.f8713WWWWWWWW) {
                    k4.WWWWWWWW wwwwwwww4 = privacyMainFragment.f8707WWWWWWWW;
                    if (wwwwwwww4 != null) {
                        wwwwwwww4.f29266WWoWWo = z12;
                        wwwwwwww4.f5980WWWWWWWW.m3938WWWWWWWW(0, wwwwwwww4.f29265WWWoWWWo.size(), null);
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-86, -35, 86, 23, -47, -30, 42, -53}, new byte[]{-57, -100, TarConstants.LF_SYMLINK, 118, -95, -106, 79, -71});
                        throw null;
                    }
                }
                WWWWoWWWWo.WWWWWWWW wwwwwwww5 = privacyMainFragment.f36451h;
                if (wwwwwwww5 == null || i13 != wwwwwwww5.f8717WWWoWWWo) {
                    k4.WWWWWWWW wwwwwwww6 = privacyMainFragment.f8707WWWWWWWW;
                    if (wwwwwwww6 != null) {
                        wwwwwwww6.f29263WWWWWWWW = i13;
                        wwwwwwww6.f5980WWWWWWWW.m3938WWWWWWWW(0, wwwwwwww6.f29265WWWoWWWo.size(), null);
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{102, 78, -19, 125, 114, 119, -71, -61}, new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, 15, -119, 28, 2, 3, -36, -79});
                        throw null;
                    }
                }
                WWWWoWWWWo.WWWWWWWW wwwwwwww7 = privacyMainFragment.f36451h;
                if (wwwwwwww7 == null || z11 != wwwwwwww7.f8719WWoWWo) {
                    k4.WWWWWWWW wwwwwwww8 = privacyMainFragment.f8707WWWWWWWW;
                    if (wwwwwwww8 != null) {
                        wwwwwwww8.f29264WWWWWWWW = z11;
                        wwwwwwww8.f5980WWWWWWWW.m3938WWWWWWWW(0, wwwwwwww8.f29265WWWoWWWo.size(), null);
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_FIFO, TarConstants.LF_CONTIG, -2, -37, 24, -105, 58, -4}, new byte[]{91, 118, -102, -70, 104, -29, 95, -114});
                        throw null;
                    }
                }
                WWWWoWWWWo.WWWWWWWW wwwwwwww9 = privacyMainFragment.f36451h;
                if (wwwwwwww9 != null) {
                    list = wwwwwwww9.f8718WWWoWWWo;
                } else {
                    list = null;
                }
                if (!AbstractC3339WWWWWWWW.m15427WWWWWWWW(list2, list)) {
                    k4.WWWWWWWW wwwwwwww10 = privacyMainFragment.f8707WWWWWWWW;
                    if (wwwwwwww10 != null) {
                        wwwwwwww10.m14885WWWWWWWW(list2);
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-49, 17, 117, TarConstants.LF_CONTIG, -114, -47, 9, -9}, new byte[]{-94, 80, 17, 86, -2, -91, 108, -123});
                        throw null;
                    }
                }
            } else {
                PrivacyMainFragment.m4996WWWWWW(privacyMainFragment, i11);
                k4.WWWWWWWW wwwwwwww11 = privacyMainFragment.f8707WWWWWWWW;
                if (wwwwwwww11 != null) {
                    wwwwwwww11.f29266WWoWWo = z12;
                    wwwwwwww11.f5980WWWWWWWW.m3938WWWWWWWW(0, wwwwwwww11.f29265WWWoWWWo.size(), null);
                    k4.WWWWWWWW wwwwwwww12 = privacyMainFragment.f8707WWWWWWWW;
                    if (wwwwwwww12 != null) {
                        wwwwwwww12.f29263WWWWWWWW = i13;
                        wwwwwwww12.f5980WWWWWWWW.m3938WWWWWWWW(0, wwwwwwww12.f29265WWWoWWWo.size(), null);
                        k4.WWWWWWWW wwwwwwww13 = privacyMainFragment.f8707WWWWWWWW;
                        if (wwwwwwww13 != null) {
                            wwwwwwww13.f29264WWWWWWWW = z11;
                            wwwwwwww13.f5980WWWWWWWW.m3938WWWWWWWW(0, wwwwwwww13.f29265WWWoWWWo.size(), null);
                            k4.WWWWWWWW wwwwwwww14 = privacyMainFragment.f8707WWWWWWWW;
                            if (wwwwwwww14 != null) {
                                wwwwwwww14.m14885WWWWWWWW(list2);
                            } else {
                                i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-62, -54, -89, -24, 16, -27, 117, -83}, new byte[]{-81, -117, -61, -119, 96, -111, 16, -33});
                                throw null;
                            }
                        } else {
                            i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-2, -3, 78, -41, 109, -1, -72, -97}, new byte[]{-109, -68, 42, -74, 29, -117, -35, -19});
                            throw null;
                        }
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-30, -52, -117, 16, 116, -101, -62, 102}, new byte[]{-113, -115, -17, 113, 4, -17, -89, 20});
                        throw null;
                    }
                } else {
                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{118, 8, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -95, -125, 101, -88, 35}, new byte[]{27, 73, 3, -64, -13, 17, -51, 81});
                    throw null;
                }
            }
            if (wwwwwwww.f8712WWWWWWWW) {
                CommonEmptyView commonEmptyView = privacyMainFragment.f8706WWWWWWWW;
                if (commonEmptyView != null) {
                    commonEmptyView.setVisibility(0);
                    CommonEmptyView commonEmptyView2 = privacyMainFragment.f8706WWWWWWWW;
                    if (commonEmptyView2 != null) {
                        commonEmptyView2.m5009WWWWWWWW(privacyMainFragment.m3290WW(R.string.privacy_loading_vm_tips));
                    } else {
                        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{2, -40, -15, -72, 70, -74, 116, 45, 10, -22}, new byte[]{111, -99, -100, -56, TarConstants.LF_SYMLINK, -49, 34, 68});
                        throw null;
                    }
                } else {
                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-23, 126, -35, -15, 124, -69, -8, -121, -31, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{-124, 59, -80, -127, 8, -62, -82, -18});
                    throw null;
                }
            } else {
                k4.WWWWWWWW wwwwwwww15 = privacyMainFragment.f8707WWWWWWWW;
                if (wwwwwwww15 != null) {
                    if (wwwwwwww15.f29265WWWoWWWo.size() == 0) {
                        CommonEmptyView commonEmptyView3 = privacyMainFragment.f8706WWWWWWWW;
                        if (commonEmptyView3 != null) {
                            commonEmptyView3.setVisibility(0);
                            CommonEmptyView commonEmptyView4 = privacyMainFragment.f8706WWWWWWWW;
                            if (commonEmptyView4 != null) {
                                String m3290WW = privacyMainFragment.m3290WW(R.string.privacy_vm_add_tips);
                                i0.WWWWWWWW.m14518WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-1, -63, -53, 90, 108, 126, -112, 70, -1, -116, -111, 39, TarConstants.LF_FIFO, 37}, new byte[]{-104, -92, -65, 9, 24, ConstantPoolEntry.CP_NameAndType, -7, 40}, m3290WW);
                                commonEmptyView4.m5008WWWWoWWWWo(m3290WW, privacyMainFragment.m3290WW(R.string.privacy_vm_menu_add), new WWWoWWWo(12, privacyMainFragment));
                            } else {
                                i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-105, TarConstants.LF_GNUTYPE_LONGNAME, ConstantPoolEntry.CP_InterfaceMethodref, 63, 8, -61, 100, -110, -97, 126}, new byte[]{-6, 9, 102, 79, 124, -70, TarConstants.LF_SYMLINK, -5});
                                throw null;
                            }
                        } else {
                            i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{TarConstants.LF_BLK, 108, 26, -91, -9, 92, -90, 109, 60, 94}, new byte[]{89, 41, 119, -43, -125, 37, -16, 4});
                            throw null;
                        }
                    } else {
                        CommonEmptyView commonEmptyView5 = privacyMainFragment.f8706WWWWWWWW;
                        if (commonEmptyView5 != null) {
                            commonEmptyView5.setVisibility(8);
                        } else {
                            i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{ConstantPoolEntry.CP_NameAndType, -85, 17, -123, -16, -12, 112, -76, 4, -103}, new byte[]{97, -18, 124, -11, -124, -115, 38, -35});
                            throw null;
                        }
                    }
                } else {
                    i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-77, 126, 56, ConstantPoolEntry.CP_NameAndType, 79, 57, 65, -8}, new byte[]{-34, 63, 92, 109, 63, TarConstants.LF_MULTIVOLUME, 36, -118});
                    throw null;
                }
            }
            WWWWoWWWWo.WWWWWWWW wwwwwwww16 = privacyMainFragment.f36451h;
            int i14 = wwwwwwww.f8711WWWWoWWWWo;
            if (wwwwwwww16 == null || i14 != wwwwwwww16.f8711WWWWoWWWWo || wwwwwwww16 == null || z12 != wwwwwwww16.f8713WWWWWWWW || wwwwwwww16 == null || i13 != wwwwwwww16.f8717WWWoWWWo) {
                privacyMainFragment.m4997WWWoWWWo(privacyMainFragment.f36447d, i14, z12, i13);
            }
            WWWWoWWWWo.WWWWWWWW wwwwwwww17 = privacyMainFragment.f36451h;
            if (wwwwwwww17 != null && (wwwwwwww17 == null || i13 != wwwwwwww17.f8717WWWoWWWo)) {
                int i15 = i13 % 3;
                if (i15 != 1) {
                    if (i15 != 2) {
                        i10 = 4;
                        byte[] bArr2 = {59, 3, -116, -93, ConstantPoolEntry.CP_NameAndType, -56, ConstantPoolEntry.CP_NameAndType, -33};
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{85, 108, -30, -58}, bArr2);
                        bArr = new byte[i10];
                        // fill-array-data instruction
                        bArr[0] = 41;
                        bArr[1] = 103;
                        bArr[2] = 112;
                        bArr[3] = 20;
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        if (!m17835WWWWWWWW.equals(x5.WWWWWWWW.m17835WWWWWWWW(bArr, new byte[]{71, 8, 30, 113, 71, -56, -54, -110})) && (toolbar = (Toolbar) this.f8722WWWWWWWW.findViewById(R.id.toolbar)) != null && (findViewById = toolbar.findViewById(R.id.touch_mode)) != null) {
                            findViewById.performLongClick();
                        }
                    } else {
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_FIFO, 71, 125, TarConstants.LF_DIR}, new byte[]{69, 62, 19, 86, -86, -105, -111, 20});
                    }
                } else {
                    byte[] bArr3 = {-41, 125, 71, -67, TarConstants.LF_GNUTYPE_SPARSE, 47, -51, 64, -46, 123, 92};
                    byte[] bArr4 = {-89, 28, TarConstants.LF_BLK, -50, 39, 71, -65, 47};
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4);
                }
                i10 = 4;
                bArr = new byte[i10];
                // fill-array-data instruction
                bArr[0] = 41;
                bArr[1] = 103;
                bArr[2] = 112;
                bArr[3] = 20;
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                if (!m17835WWWWWWWW.equals(x5.WWWWWWWW.m17835WWWWWWWW(bArr, new byte[]{71, 8, 30, 113, 71, -56, -54, -110}))) {
                    findViewById.performLongClick();
                }
            }
            if (!wwwwwwww.f8715WWWWWWWW) {
                FragmentActivity m3293WWWW = privacyMainFragment.m3293WWWW();
                byte[] bArr5 = {1, 64, 94, -61, TarConstants.LF_LINK, 65, -28, -124, 16, 81, 70, -64, TarConstants.LF_LINK, 71, -8, -19, 93, ConstantPoolEntry.CP_InterfaceMethodref, 1, -97};
                byte[] bArr6 = {115, 37, 47, -74, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_CHR, -127, -59};
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                x5.WWWWWWWW.m17835WWWWWWWW(bArr5, bArr6);
                AbstractC2131WW.m12843WWoWWo(m3293WWWW, true);
            } else if (wwwwwwww.f8716WWWWWWWW) {
                da.WWWWoWWWWo wWWWoWWWWo = new da.WWWWoWWWWo(privacyMainFragment.m3293WWWW());
                wWWWoWWWWo.m13648WoWo(R.string.dialog_title_alert_setup_password);
                wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_alert_setup_password);
                wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_got_it, null);
                wWWWoWWWWo.m741WWoWWo();
                WWWWoWWWWo wWWWoWWWWo2 = (WWWWoWWWWo) privacyMainFragment.f36452i.getValue();
                wWWWoWWWWo2.f8708WWWWWWWW = true;
                do {
                    c2819wwwwwwww = wWWWoWWWWo2.f8710WWWW;
                    m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
                } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, WWWWoWWWWo.WWWWWWWW.m4998WWWWWWWW((WWWWoWWWWo.WWWWWWWW) m14479WWWWWWWW, false, 0, 0, false, false, false, false, false, null, 383)));
            }
            privacyMainFragment.f36451h = wwwwwwww;
            return WWWWWWWWWW.f27054WWWWWWWW;
        }
        i0.WWWWWWWW.m14532o(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{19, 16, -10, 58, -87, 82, 71, 71, TarConstants.LF_LINK, 46, -51, 43, -70, 86, 118, 95, ConstantPoolEntry.CP_InterfaceMethodref, 35, -20, 19, -74, 72, 86, 85, 16, 37, -10}, new byte[]{126, 64, -124, 95, -33, 59, 34, TarConstants.LF_NORMAL});
        throw null;
    }
}
