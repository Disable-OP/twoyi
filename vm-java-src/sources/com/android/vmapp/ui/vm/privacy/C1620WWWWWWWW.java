package com.android.vmapp.ui.vm.privacy;

import com.android.vmapp.ui.vm.privacy.WWWWoWWWWo;
import ed.AbstractC2403WWWWoWWWWo;
import ed.InterfaceC2450WWoWWo;
import fc.WWWWWWWWWW;
import hd.C2819WWWWWWWW;
import j3.C3164WWWWWWWW;
import j3.SharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW;
import jc.InterfaceC3180WWWWWWWW;
import kc.WWWWWWWW;
import lc.AbstractC3453WWWWWWWW;
import me.AbstractC3506WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import tc.InterfaceC4270WWWWWWWW;
/* renamed from: com.android.vmapp.ui.vm.privacy.WWWWϙWWWWეϙ  reason: invalid class name and case insensitive filesystem */
/* loaded from: classes.dex */
public final class C1620WWWWWWWW extends AbstractC3453WWWWWWWW implements InterfaceC4270WWWWWWWW {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final /* synthetic */ WWWWoWWWWo f8724WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public int f8725WWWWoWWWWo;

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public C1620WWWWWWWW(WWWWoWWWWo wWWWoWWWWo, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        super(2, interfaceC3180WWWWWWWW);
        this.f8724WWWWWWWWWW = wWWWoWWWWo;
    }

    @Override // lc.WWWWWWWW
    public final InterfaceC3180WWWWWWWW create(Object obj, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        return new C1620WWWWWWWW(this.f8724WWWWWWWWWW, interfaceC3180WWWWWWWW);
    }

    @Override // tc.InterfaceC4270WWWWWWWW
    public final Object invoke(Object obj, Object obj2) {
        return ((C1620WWWWWWWW) create((InterfaceC2450WWoWWo) obj, (InterfaceC3180WWWWWWWW) obj2)).invokeSuspend(WWWWWWWWWW.f27054WWWWWWWW);
    }

    @Override // lc.WWWWWWWW
    public final Object invokeSuspend(Object obj) {
        Object m14479WWWWWWWW;
        WWWWWWWW wwwwwwww = WWWWWWWW.f30085WWWWoWWWWo;
        int i10 = this.f8725WWWWoWWWWo;
        if (i10 != 0) {
            if (i10 == 1) {
                AbstractC3506WWWWWWWW.m15930WWWWWWWW(obj);
            } else {
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                throw new IllegalStateException(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{15, 37, -19, 87, -101, TarConstants.LF_GNUTYPE_LONGNAME, -77, -67, TarConstants.LF_GNUTYPE_LONGLINK, TarConstants.LF_FIFO, -28, 72, -50, 85, -71, -70, TarConstants.LF_GNUTYPE_LONGNAME, 38, -28, 93, -44, 74, -71, -67, TarConstants.LF_GNUTYPE_LONGLINK, 45, -17, TarConstants.LF_MULTIVOLUME, -44, TarConstants.LF_GNUTYPE_SPARSE, -71, -70, TarConstants.LF_GNUTYPE_LONGNAME, TarConstants.LF_CHR, -24, 79, -45, 24, -65, -14, 30, 43, -12, 79, -46, 86, -71}, new byte[]{108, 68, -127, 59, -69, 56, -36, -99}));
            }
        } else {
            AbstractC3506WWWWWWWW.m15930WWWWWWWW(obj);
            this.f8725WWWWoWWWWo = 1;
            if (AbstractC2403WWWWoWWWWo.m13808WWWoWWWo(100L, this) == wwwwwwww) {
                return wwwwwwww;
            }
        }
        C2819WWWWWWWW c2819wwwwwwww = this.f8724WWWWWWWWWW.f8710WWWW;
        boolean z10 = ((WWWWoWWWWo.WWWWWWWW) c2819wwwwwwww.m14479WWWWWWWW()).f8716WWWWWWWW;
        SharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW.f28903WWWWoWWWWo.getClass();
        if (z10 != SharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW.m14754WWWoWWWo()) {
            do {
                m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
                SharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW.f28903WWWWoWWWWo.getClass();
            } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, WWWWoWWWWo.WWWWWWWW.m4998WWWWWWWW((WWWWoWWWWo.WWWWWWWW) m14479WWWWWWWW, false, 0, 0, false, false, false, false, SharedPreferences$OnSharedPreferenceChangeListenerC3163WWWWWWWW.m14754WWWoWWWo(), null, 383)));
            return WWWWWWWWWW.f27054WWWWWWWW;
        }
        return WWWWWWWWWW.f27054WWWWWWWW;
    }
}
