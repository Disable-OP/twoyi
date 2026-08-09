package com.android.vmapp.ui.vm.main;

import com.android.vmapp.ui.vm.main.WWWWoWWWWo;
import fc.WWWWWWWWWW;
import hd.C2819WWWWWWWW;
import java.util.List;
import jc.InterfaceC3180WWWWWWWW;
import kc.WWWWWWWW;
import lc.AbstractC3453WWWWWWWW;
import me.AbstractC3506WWWWWWWW;
import org.apache.commons.compress.archivers.zip.UnixStat;
import tc.InterfaceC4270WWWWWWWW;
/* renamed from: com.android.vmapp.ui.vm.main.WWWȏWWWoನ̑  reason: invalid class name */
/* loaded from: classes.dex */
public final class WWWoWWWo extends AbstractC3453WWWWWWWW implements InterfaceC4270WWWWWWWW {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final /* synthetic */ WWWWoWWWWo f8697WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public /* synthetic */ Object f8698WWWWoWWWWo;

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public WWWoWWWo(WWWWoWWWWo wWWWoWWWWo, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        super(2, interfaceC3180WWWWWWWW);
        this.f8697WWWWWWWWWW = wWWWoWWWWo;
    }

    @Override // lc.WWWWWWWW
    public final InterfaceC3180WWWWWWWW create(Object obj, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        WWWoWWWo wWWoWWWo = new WWWoWWWo(this.f8697WWWWWWWWWW, interfaceC3180WWWWWWWW);
        wWWoWWWo.f8698WWWWoWWWWo = obj;
        return wWWoWWWo;
    }

    @Override // tc.InterfaceC4270WWWWWWWW
    public final Object invoke(Object obj, Object obj2) {
        WWWWWWWWWW wwwwwwwwww = WWWWWWWWWW.f27054WWWWWWWW;
        ((WWWoWWWo) create((List) obj, (InterfaceC3180WWWWWWWW) obj2)).invokeSuspend(wwwwwwwwww);
        return wwwwwwwwww;
    }

    @Override // lc.WWWWWWWW
    public final Object invokeSuspend(Object obj) {
        C2819WWWWWWWW c2819wwwwwwww;
        Object m14479WWWWWWWW;
        List list = (List) this.f8698WWWWoWWWWo;
        WWWWWWWW wwwwwwww = WWWWWWWW.f30085WWWWoWWWWo;
        AbstractC3506WWWWWWWW.m15930WWWWWWWW(obj);
        WWWWoWWWWo wWWWoWWWWo = this.f8697WWWWWWWWWW;
        do {
            c2819wwwwwwww = wWWWoWWWWo.f8678WWWWWWWW;
            m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
        } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, WWWWoWWWWo.WWWWWWWW.m4995WWWWWWWW((WWWWoWWWWo.WWWWWWWW) m14479WWWWWWWW, false, 0, 0, false, false, false, false, false, false, list, UnixStat.DEFAULT_LINK_PERM)));
        return WWWWWWWWWW.f27054WWWWWWWW;
    }
}
