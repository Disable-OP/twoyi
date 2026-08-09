package com.android.vmapp.ui.vm.main;

import android.content.SharedPreferences;
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
/* renamed from: com.android.vmapp.ui.vm.main.WWWWͶWWWWᆑͶ  reason: invalid class name and case insensitive filesystem */
/* loaded from: classes.dex */
public final class C1615WWWWWWWW extends AbstractC3453WWWWWWWW implements InterfaceC4270WWWWWWWW {

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final /* synthetic */ WWWWoWWWWo f8693WWWWoWWWWo;

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public C1615WWWWWWWW(WWWWoWWWWo wWWWoWWWWo, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        super(2, interfaceC3180WWWWWWWW);
        this.f8693WWWWoWWWWo = wWWWoWWWWo;
    }

    @Override // lc.WWWWWWWW
    public final InterfaceC3180WWWWWWWW create(Object obj, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        return new C1615WWWWWWWW(this.f8693WWWWoWWWWo, interfaceC3180WWWWWWWW);
    }

    @Override // tc.InterfaceC4270WWWWWWWW
    public final Object invoke(Object obj, Object obj2) {
        WWWWWWWWWW wwwwwwwwww = WWWWWWWWWW.f27054WWWWWWWW;
        ((C1615WWWWWWWW) create((InterfaceC2450WWoWWo) obj, (InterfaceC3180WWWWWWWW) obj2)).invokeSuspend(wwwwwwwwww);
        return wwwwwwwwww;
    }

    @Override // lc.WWWWWWWW
    public final Object invokeSuspend(Object obj) {
        Object m14479WWWWWWWW;
        WWWWWWWW wwwwwwww = WWWWWWWW.f30085WWWWoWWWWo;
        AbstractC3506WWWWWWWW.m15930WWWWWWWW(obj);
        C2819WWWWWWWW c2819wwwwwwww = this.f8693WWWWoWWWWo.f8678WWWWWWWW;
        int i10 = ((WWWWoWWWWo.WWWWWWWW) c2819wwwwwwww.m14479WWWWWWWW()).f8680WWWWoWWWWo + 1;
        do {
            m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
        } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, WWWWoWWWWo.WWWWWWWW.m4995WWWWWWWW((WWWWoWWWWo.WWWWWWWW) m14479WWWWWWWW, false, i10, 0, false, false, false, false, false, false, null, 1021)));
        SharedPreferences.Editor edit = q3.WWWWoWWWWo.f32526WWWWWWWW.edit();
        byte[] bArr = {-122, -19, -15, TarConstants.LF_SYMLINK, 89, 56, -64, 38, -124, -33, -61, TarConstants.LF_LINK, 92, 36, -16, 58, -98, -28, -53, 38};
        byte[] bArr2 = {-16, Byte.MIN_VALUE, -82, 94, 56, 65, -81, TarConstants.LF_GNUTYPE_SPARSE};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        edit.putInt(x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), i10).apply();
        return WWWWWWWWWW.f27054WWWWWWWW;
    }
}
