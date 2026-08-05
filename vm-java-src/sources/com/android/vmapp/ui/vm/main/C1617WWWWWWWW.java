package com.android.vmapp.ui.vm.main;

import android.content.SharedPreferences;
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
/* renamed from: com.android.vmapp.ui.vm.main.WWWWҍWWWWּҍ  reason: invalid class name and case insensitive filesystem */
/* loaded from: classes.dex */
public final class C1617WWWWWWWW extends AbstractC3453WWWWWWWW implements InterfaceC4270WWWWWWWW {

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final /* synthetic */ WWWWoWWWWo f8695WWWWoWWWWo;

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public C1617WWWWWWWW(WWWWoWWWWo wWWWoWWWWo, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        super(2, interfaceC3180WWWWWWWW);
        this.f8695WWWWoWWWWo = wWWWoWWWWo;
    }

    @Override // lc.WWWWWWWW
    public final InterfaceC3180WWWWWWWW create(Object obj, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        return new C1617WWWWWWWW(this.f8695WWWWoWWWWo, interfaceC3180WWWWWWWW);
    }

    @Override // tc.InterfaceC4270WWWWWWWW
    public final Object invoke(Object obj, Object obj2) {
        WWWWWWWWWW wwwwwwwwww = WWWWWWWWWW.f27054WWWWWWWW;
        ((C1617WWWWWWWW) create((InterfaceC2450WWoWWo) obj, (InterfaceC3180WWWWWWWW) obj2)).invokeSuspend(wwwwwwwwww);
        return wwwwwwwwww;
    }

    @Override // lc.WWWWWWWW
    public final Object invokeSuspend(Object obj) {
        Object m14479WWWWWWWW;
        WWWWWWWW wwwwwwww = WWWWWWWW.f30085WWWWoWWWWo;
        AbstractC3506WWWWWWWW.m15930WWWWWWWW(obj);
        C2819WWWWWWWW c2819wwwwwwww = this.f8695WWWWoWWWWo.f8678WWWWWWWW;
        do {
            m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
        } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, WWWWoWWWWo.WWWWWWWW.m4995WWWWWWWW((WWWWoWWWWo.WWWWWWWW) m14479WWWWWWWW, false, 0, 0, false, false, true, false, false, false, null, 991)));
        SharedPreferences.Editor edit = q3.WWWWoWWWWo.f32526WWWWWWWW.edit();
        AbstractC1017WWWoWWWo.m3456WWoWWo(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-8, 24, 108, -49, -21, -19, 31, 60, -2, 7, 90, -55, -1, -30, 10, 60, -20, 5, 114, -41, -43, -6, 15, 19, -5, TarConstants.LF_DIR, 102, -42, -28, -24, 15, 17, -27, 15, 97}, new byte[]{-120, 106, 5, -71, -118, -114, 102, 99}, edit, true);
        return WWWWWWWWWW.f27054WWWWWWWW;
    }
}
