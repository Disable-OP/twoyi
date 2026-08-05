package com.android.vmapp.ui.vm.privacy;

import a2.C0639WWWWWWWW;
import android.os.Bundle;
import android.view.View;
import androidx.appcompat.app.WWWWWWWW;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import c0.C1458WWWW;
import com.android.vmapp.ui.base.BaseActivity;
import com.android.vmapp.ui.widget.CommonEmptyView;
import com.clone.android.dual.space.R;
import ed.AbstractC2403WWWWoWWWWo;
import i4.WoWo;
import ib.WWWWoWWWWo;
import j3.C3164WWWWWWWW;
import java.util.WeakHashMap;
import k3.C3232WWoWWo;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import l4.C3423WWWWWWWW;
import l4.C3426WWWWWWWW;
import l4.WWWoWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p013WWWWWWWW.o;
import p069WoWo.AbstractC0550WWWWWWWW;
import p069WoWo.AbstractC0593WoWo;
/* loaded from: classes.dex */
public final class PrivacyAddActivity extends BaseActivity {

    /* renamed from: Wo᪅Woै᪅  reason: contains not printable characters */
    public static final /* synthetic */ int f8700WoWo = 0;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public RecyclerView f8701WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public WoWo f8702WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public CommonEmptyView f8703WWWWWWWW;

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public final o f8704WWWW = new o(C3333WWWWoWWWWo.m15421WWWWWWWW(C3426WWWWWWWW.class), new C3423WWWWWWWW(this, 0), new C1458WWWW(11), new C3423WWWWWWWW(this, 1));

    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_privacy_add);
        View findViewById = findViewById(R.id.toolbar);
        AbstractC1017WWWoWWWo.m3443WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{86, -121, TarConstants.LF_LINK, 69, -64, -13, 32, -24, 114, -105, 22, 69, -66, -76, 107, -79, 25}, new byte[]{TarConstants.LF_NORMAL, -18, 95, 33, -106, -102, 69, -97}, findViewById);
        m2306WWWoWWWo((Toolbar) findViewById);
        WWWWWWWW m2307WWoWWo = m2307WWoWWo();
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(m2307WWoWWo);
        m2307WWoWWo.mo2341WoWo(true);
        View findViewById2 = findViewById(R.id.emptyView);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-73, 37, -12, 79, 89, -88, 93, -62, -109, TarConstants.LF_DIR, -45, 79, 39, -17, 22, -101, -8}, new byte[]{-47, TarConstants.LF_GNUTYPE_LONGNAME, -102, 43, 15, -63, 56, -75}));
        this.f8703WWWWWWWW = (CommonEmptyView) findViewById2;
        View findViewById3 = findViewById(R.id.listview);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-14, 56, -27, -124, 101, -17, -60, 6, -42, 40, -62, -124, 27, -88, -113, 95, -67}, new byte[]{-108, 81, -117, -32, TarConstants.LF_CHR, -122, -95, 113}));
        RecyclerView recyclerView = (RecyclerView) findViewById3;
        this.f8701WWWWWWWW = recyclerView;
        recyclerView.setLayoutManager(new LinearLayoutManager(1));
        WoWo woWo = new WoWo(new C0639WWWWWWWW(24, this));
        this.f8702WWWWWWWW = woWo;
        RecyclerView recyclerView2 = this.f8701WWWWWWWW;
        if (recyclerView2 != null) {
            recyclerView2.setAdapter(woWo);
            AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(WWWWoWWWWo.m14598WWWWWWWW(this), null, new WWWoWWWo(this, null), 3);
            RecyclerView recyclerView3 = this.f8701WWWWWWWW;
            if (recyclerView3 != null) {
                C3232WWoWWo c3232WWoWWo = new C3232WWoWWo(1);
                WeakHashMap weakHashMap = AbstractC0550WWWWWWWW.f2596WWWWWWWW;
                AbstractC0593WoWo.m1914WoWo(recyclerView3, c3232WWoWWo);
                return;
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-80, 34, TarConstants.LF_NORMAL, -92, -75, -48, 126, 112, -86}, new byte[]{-35, 110, 89, -41, -63, -122, 23, 21}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-17, -43, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, ConstantPoolEntry.CP_NameAndType, 57, -47, ConstantPoolEntry.CP_NameAndType, 5, -11}, new byte[]{-126, -103, 14, Byte.MAX_VALUE, TarConstants.LF_MULTIVOLUME, -121, 101, 96}));
        throw null;
    }
}
