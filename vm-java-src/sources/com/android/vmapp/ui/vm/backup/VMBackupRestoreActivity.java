package com.android.vmapp.ui.vm.backup;

import a3.WWWoWWWo;
import android.os.Bundle;
import android.view.View;
import androidx.appcompat.app.WWWWWWWW;
import androidx.appcompat.widget.Toolbar;
import androidx.core.widget.NestedScrollView;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import c0.C1458WWWW;
import com.android.vmapp.ui.base.BaseActivity;
import com.android.vmapp.ui.widget.CommonEmptyView;
import com.clone.android.dual.space.R;
import com.google.android.material.card.MaterialCardView;
import com.google.android.material.floatingactionbutton.FloatingActionButton;
import ed.AbstractC2403WWWWoWWWWo;
import h4.C2702WWWWoWWWWo;
import h4.C2735WWWoWWWo;
import h4.WW;
import h4.WWWWWWWWWW;
import j3.C3164WWWWWWWW;
import java.util.WeakHashMap;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import p004WWWWoWWWWo.WWWWoWWWWo;
import p008WWWWWWWW.C0098WWWWWWWW;
import p013WWWWWWWW.o;
import p024WWWWWWWW.WWoWWo;
import p069WoWo.AbstractC0550WWWWWWWW;
import p069WoWo.AbstractC0593WoWo;
/* loaded from: classes.dex */
public final class VMBackupRestoreActivity extends BaseActivity {

    /* renamed from: WWWWᮭWWWWᆏᮭ  reason: contains not printable characters */
    public static final /* synthetic */ int f8615WWWWWWWW = 0;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public NestedScrollView f8616WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public RecyclerView f8617WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public WWWWWWWWWW f8618WWWWWWWW;

    /* renamed from: WWWWᬭWWWWɿᬭ  reason: contains not printable characters */
    public final o f8619WWWWWWWW = new o(C3333WWWWoWWWWo.m15421WWWWWWWW(C2702WWWWoWWWWo.class), new WW(this, 0), new C1458WWWW(7), new WW(this, 1));

    /* renamed from: WWo᪙WWoг᪙  reason: contains not printable characters */
    public C0098WWWWWWWW f8620WWoWWo;

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public CommonEmptyView f8621WWWW;

    /* renamed from: Wo᪅Woै᪅  reason: contains not printable characters */
    public FloatingActionButton f8622WoWo;

    /* renamed from: WWoᆑWWoӁᆑ  reason: contains not printable characters */
    public final C2702WWWWoWWWWo m4963WWoWWo() {
        return (C2702WWWWoWWWWo) this.f8619WWWWWWWW.getValue();
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_vm_backup_restore);
        View findViewById = findViewById(R.id.toolbar);
        AbstractC1017WWWoWWWo.m3443WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{46, 72, -127, 34, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 15, -7, 80, 10, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -90, 34, 6, 72, -78, 9, 97}, new byte[]{72, 33, -17, 70, 46, 102, -100, 39}, findViewById);
        m2306WWWoWWWo((Toolbar) findViewById);
        WWWWWWWW m2307WWoWWo = m2307WWoWWo();
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(m2307WWoWWo);
        m2307WWoWWo.mo2341WoWo(true);
        View findViewById2 = findViewById(R.id.contentView);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{64, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 70, -23, -11, -42, -35, -18, 100, 104, 97, -23, -117, -111, -106, -73, 15}, new byte[]{38, 17, 40, -115, -93, -65, -72, -103}));
        this.f8616WWWWWWWW = (NestedScrollView) findViewById2;
        View findViewById3 = findViewById(R.id.backup_keep_card);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-25, -74, 80, -8, 62, -47, 18, TarConstants.LF_LINK, -61, -90, 119, -8, 64, -106, 89, 104, -88}, new byte[]{-127, -33, 62, -100, 104, -72, 119, 70}));
        MaterialCardView materialCardView = (MaterialCardView) findViewById3;
        View findViewById4 = findViewById(R.id.listview);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById4, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_NORMAL, 81, -66, -15, -106, 95, -30, -89, 20, 65, -103, -15, -24, 24, -87, -2, Byte.MAX_VALUE}, new byte[]{86, 56, -48, -107, -64, TarConstants.LF_FIFO, -121, -48}));
        this.f8617WWWWWWWW = (RecyclerView) findViewById4;
        LinearLayoutManager linearLayoutManager = new LinearLayoutManager(1);
        RecyclerView recyclerView = this.f8617WWWWWWWW;
        if (recyclerView != null) {
            recyclerView.setLayoutManager(linearLayoutManager);
            WWWWWWWWWW wwwwwwwwww = new WWWWWWWWWW(new WWoWWo(this));
            this.f8618WWWWWWWW = wwwwwwwwww;
            RecyclerView recyclerView2 = this.f8617WWWWWWWW;
            if (recyclerView2 != null) {
                recyclerView2.setAdapter(wwwwwwwwww);
                View findViewById5 = findViewById(R.id.emptyView);
                AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById5, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-123, -24, 109, 74, 27, 82, -98, 19, -95, -8, 74, 74, 101, 21, -43, 74, -54}, new byte[]{-29, -127, 3, 46, TarConstants.LF_MULTIVOLUME, 59, -5, 100}));
                this.f8621WWWW = (CommonEmptyView) findViewById5;
                View findViewById6 = findViewById(R.id.add_imports);
                AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById6, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-117, -35, -114, 108, -57, -100, 23, 4, -81, -51, -87, 108, -71, -37, 92, 93, -60}, new byte[]{-19, -76, -32, 8, -111, -11, 114, 115}));
                FloatingActionButton floatingActionButton = (FloatingActionButton) findViewById6;
                this.f8622WoWo = floatingActionButton;
                floatingActionButton.setOnClickListener(new WWWoWWWo(10, this));
                this.f8620WWoWWo = (C0098WWWWWWWW) m2296WWWWWWWWWW(new WWWWoWWWWo(25, this), new p009WWWWWWWW.WWWWoWWWWo());
                RecyclerView recyclerView3 = this.f8617WWWWWWWW;
                if (recyclerView3 != null) {
                    p024WWWWWWWW.WWWWWWWW wwwwwwww = new p024WWWWWWWW.WWWWWWWW(25);
                    WeakHashMap weakHashMap = AbstractC0550WWWWWWWW.f2596WWWWWWWW;
                    AbstractC0593WoWo.m1914WoWo(recyclerView3, wwwwwwww);
                    AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(ib.WWWWoWWWWo.m14598WWWWWWWW(this), null, new C2735WWWoWWWo(this, null), 3);
                    return;
                }
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-31, 45, -29, 98, 70, -15, 124, -45, -5}, new byte[]{-116, 97, -118, 17, TarConstants.LF_SYMLINK, -89, 21, -74}));
                throw null;
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{112, -43, -2, 22, -102, 23, -100, -83, 106}, new byte[]{29, -103, -105, 101, -18, 65, -11, -56}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{118, -25, 117, -90, 9, 79, TarConstants.LF_CONTIG, 119, 108}, new byte[]{27, -85, 28, -43, 125, 25, 94, 18}));
        throw null;
    }
}
