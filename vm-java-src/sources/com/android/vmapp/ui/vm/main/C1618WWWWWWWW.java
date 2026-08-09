package com.android.vmapp.ui.vm.main;

import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmapp.ui.vm.main.WWWWoWWWWo;
import ed.InterfaceC2450WWoWWo;
import fc.WWWWWWWWWW;
import hd.C2819WWWWWWWW;
import j3.C3164WWWWWWWW;
import jc.InterfaceC3180WWWWWWWW;
import kc.WWWWWWWW;
import lc.AbstractC3453WWWWWWWW;
import me.AbstractC3506WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import tc.InterfaceC4270WWWWWWWW;
/* renamed from: com.android.vmapp.ui.vm.main.WWWWӈWWWWीӈ  reason: invalid class name and case insensitive filesystem */
/* loaded from: classes.dex */
public final class C1618WWWWWWWW extends AbstractC3453WWWWWWWW implements InterfaceC4270WWWWWWWW {

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final /* synthetic */ WWWWoWWWWo f8696WWWWoWWWWo;

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public C1618WWWWWWWW(WWWWoWWWWo wWWWoWWWWo, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        super(2, interfaceC3180WWWWWWWW);
        this.f8696WWWWoWWWWo = wWWWoWWWWo;
    }

    @Override // lc.WWWWWWWW
    public final InterfaceC3180WWWWWWWW create(Object obj, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        return new C1618WWWWWWWW(this.f8696WWWWoWWWWo, interfaceC3180WWWWWWWW);
    }

    @Override // tc.InterfaceC4270WWWWWWWW
    public final Object invoke(Object obj, Object obj2) {
        WWWWWWWWWW wwwwwwwwww = WWWWWWWWWW.f27054WWWWWWWW;
        ((C1618WWWWWWWW) create((InterfaceC2450WWoWWo) obj, (InterfaceC3180WWWWWWWW) obj2)).invokeSuspend(wwwwwwwwww);
        return wwwwwwwwww;
    }

    @Override // lc.WWWWWWWW
    public final Object invokeSuspend(Object obj) {
        Object m14479WWWWWWWW;
        WWWWWWWW wwwwwwww = WWWWWWWW.f30085WWWWoWWWWo;
        AbstractC3506WWWWWWWW.m15930WWWWWWWW(obj);
        C2819WWWWWWWW c2819wwwwwwww = this.f8696WWWWoWWWWo.f8678WWWWWWWW;
        boolean z10 = !((WWWWoWWWWo.WWWWWWWW) c2819wwwwwwww.m14479WWWWWWWW()).f8682WWWWWWWW;
        do {
            m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
        } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, WWWWoWWWWo.WWWWWWWW.m4995WWWWWWWW((WWWWoWWWWo.WWWWWWWW) m14479WWWWWWWW, false, 0, 0, z10, false, false, false, false, false, null, 1011)));
        AbstractC1017WWWoWWWo.m3456WWoWWo(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-70, 5, 14, 59, -121, TarConstants.LF_GNUTYPE_SPARSE, 21, -84, -72, TarConstants.LF_CONTIG, 33, 37, -125, 92, 19, -68, -69, TarConstants.LF_CONTIG, 60, 56, -126, 79}, new byte[]{-52, 104, 81, 87, -26, 42, 122, -39}, q3.WWWWoWWWWo.f32526WWWWWWWW.edit(), z10);
        return WWWWWWWWWW.f27054WWWWWWWW;
    }
}
