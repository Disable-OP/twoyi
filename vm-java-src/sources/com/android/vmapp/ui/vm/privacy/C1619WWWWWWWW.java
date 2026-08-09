package com.android.vmapp.ui.vm.privacy;

import android.content.SharedPreferences;
import com.android.vmapp.ui.vm.privacy.WWWWoWWWWo;
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
/* renamed from: com.android.vmapp.ui.vm.privacy.WWWWͶWWWWᆑͶ  reason: invalid class name and case insensitive filesystem */
/* loaded from: classes.dex */
public final class C1619WWWWWWWW extends AbstractC3453WWWWWWWW implements InterfaceC4270WWWWWWWW {

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final /* synthetic */ WWWWoWWWWo f8723WWWWoWWWWo;

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public C1619WWWWWWWW(WWWWoWWWWo wWWWoWWWWo, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        super(2, interfaceC3180WWWWWWWW);
        this.f8723WWWWoWWWWo = wWWWoWWWWo;
    }

    @Override // lc.WWWWWWWW
    public final InterfaceC3180WWWWWWWW create(Object obj, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        return new C1619WWWWWWWW(this.f8723WWWWoWWWWo, interfaceC3180WWWWWWWW);
    }

    @Override // tc.InterfaceC4270WWWWWWWW
    public final Object invoke(Object obj, Object obj2) {
        WWWWWWWWWW wwwwwwwwww = WWWWWWWWWW.f27054WWWWWWWW;
        ((C1619WWWWWWWW) create((InterfaceC2450WWoWWo) obj, (InterfaceC3180WWWWWWWW) obj2)).invokeSuspend(wwwwwwwwww);
        return wwwwwwwwww;
    }

    @Override // lc.WWWWWWWW
    public final Object invokeSuspend(Object obj) {
        Object m14479WWWWWWWW;
        WWWWWWWW wwwwwwww = WWWWWWWW.f30085WWWWoWWWWo;
        AbstractC3506WWWWWWWW.m15930WWWWWWWW(obj);
        C2819WWWWWWWW c2819wwwwwwww = this.f8723WWWWoWWWWo.f8710WWWW;
        int i10 = ((WWWWoWWWWo.WWWWWWWW) c2819wwwwwwww.m14479WWWWWWWW()).f8711WWWWoWWWWo + 1;
        do {
            m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
        } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, WWWWoWWWWo.WWWWWWWW.m4998WWWWWWWW((WWWWoWWWWo.WWWWWWWW) m14479WWWWWWWW, false, i10, 0, false, false, false, false, false, null, 509)));
        SharedPreferences.Editor edit = q3.WWWWoWWWWo.f32526WWWWWWWW.edit();
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        edit.putInt(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-94, 118, 124, TarConstants.LF_CONTIG, -25, -84, 68, -16, -92, 105, 74, 45, -25, -74, 82, -38, -90, 91, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 46, -30, -86, 98, -58, -68, 96, 112, 57}, new byte[]{-46, 4, 21, 65, -122, -49, 61, -81}), i10).apply();
        return WWWWWWWWWW.f27054WWWWWWWW;
    }
}
