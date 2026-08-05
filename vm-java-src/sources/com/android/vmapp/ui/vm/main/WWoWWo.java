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
/* renamed from: com.android.vmapp.ui.vm.main.WWoϫWWoӉϫ  reason: invalid class name */
/* loaded from: classes.dex */
public final class WWoWWo extends AbstractC3453WWWWWWWW implements InterfaceC4270WWWWWWWW {

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final /* synthetic */ WWWWoWWWWo f8699WWWWoWWWWo;

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public WWoWWo(WWWWoWWWWo wWWWoWWWWo, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        super(2, interfaceC3180WWWWWWWW);
        this.f8699WWWWoWWWWo = wWWWoWWWWo;
    }

    @Override // lc.WWWWWWWW
    public final InterfaceC3180WWWWWWWW create(Object obj, InterfaceC3180WWWWWWWW interfaceC3180WWWWWWWW) {
        return new WWoWWo(this.f8699WWWWoWWWWo, interfaceC3180WWWWWWWW);
    }

    @Override // tc.InterfaceC4270WWWWWWWW
    public final Object invoke(Object obj, Object obj2) {
        WWWWWWWWWW wwwwwwwwww = WWWWWWWWWW.f27054WWWWWWWW;
        ((WWoWWo) create((InterfaceC2450WWoWWo) obj, (InterfaceC3180WWWWWWWW) obj2)).invokeSuspend(wwwwwwwwww);
        return wwwwwwwwww;
    }

    @Override // lc.WWWWWWWW
    public final Object invokeSuspend(Object obj) {
        C2819WWWWWWWW c2819wwwwwwww;
        Object m14479WWWWWWWW;
        WWWWWWWW wwwwwwww = WWWWWWWW.f30085WWWWoWWWWo;
        AbstractC3506WWWWWWWW.m15930WWWWWWWW(obj);
        WWWWoWWWWo wWWWoWWWWo = this.f8699WWWWoWWWWo;
        do {
            c2819wwwwwwww = wWWWoWWWWo.f8678WWWWWWWW;
            m14479WWWWWWWW = c2819wwwwwwww.m14479WWWWWWWW();
        } while (!c2819wwwwwwww.m14478WWWWWWWW(m14479WWWWWWWW, WWWWoWWWWo.WWWWWWWW.m4995WWWWWWWW((WWWWoWWWWo.WWWWWWWW) m14479WWWWWWWW, false, 0, 0, false, true, false, false, false, false, null, 1007)));
        SharedPreferences.Editor edit = q3.WWWWoWWWWo.f32526WWWWWWWW.edit();
        AbstractC1017WWWoWWWo.m3456WWoWWo(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-99, Byte.MAX_VALUE, -69, -125, TarConstants.LF_SYMLINK, -38, -68, 38, -97, TarConstants.LF_MULTIVOLUME, -108, -99, TarConstants.LF_FIFO, -43, -70, TarConstants.LF_FIFO, -100, TarConstants.LF_MULTIVOLUME, -119, Byte.MIN_VALUE, TarConstants.LF_CONTIG, -58, -116, TarConstants.LF_DIR, -114, 115, -112, -102, 33, -58, -116, TarConstants.LF_CONTIG, -126, 97, -108, -125, TarConstants.LF_SYMLINK, -38, -74, TarConstants.LF_CONTIG}, new byte[]{-21, 18, -28, -17, TarConstants.LF_GNUTYPE_SPARSE, -93, -45, TarConstants.LF_GNUTYPE_SPARSE}, edit, true);
        return WWWWWWWWWW.f27054WWWWWWWW;
    }
}
