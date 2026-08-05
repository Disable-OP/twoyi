package com.android.vmapp.ui.vm.carrier;

import android.os.Bundle;
import android.view.Menu;
import android.view.MenuItem;
import android.view.View;
import android.widget.ImageView;
import androidx.appcompat.app.WWWWWWWW;
import androidx.appcompat.widget.SearchView;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import c0.C1458WWWW;
import com.android.vmapp.ui.base.BaseActivity;
import com.android.vmapp.ui.widget.CommonEmptyView;
import com.clone.android.dual.space.R;
import ed.AbstractC2403WWWWoWWWWo;
import i4.C2873WWWWWWWW;
import i4.C2874WWWWWWWW;
import i4.C2876WWWWWWWW;
import i4.C2879WWWWWWWW;
import i4.WoWo;
import ib.WWWWoWWWWo;
import j3.C3164WWWWWWWW;
import java.util.WeakHashMap;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p013WWWWWWWW.o;
import p024WWWWWWWW.WWoWWo;
import p025WWWWWWWW.C0279WWWWWWWW;
import p069WoWo.AbstractC0550WWWWWWWW;
import p069WoWo.AbstractC0593WoWo;
/* loaded from: classes.dex */
public final class CarrierListActivity extends BaseActivity {

    /* renamed from: WWWWᬭWWWWɿᬭ  reason: contains not printable characters */
    public static final /* synthetic */ int f8623WWWWWWWW = 0;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public Menu f8624WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public RecyclerView f8625WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public WoWo f8626WWWWWWWW;

    /* renamed from: WWo᪙WWoг᪙  reason: contains not printable characters */
    public final o f8627WWoWWo = new o(C3333WWWWoWWWWo.m15421WWWWWWWW(C2876WWWWWWWW.class), new C2874WWWWWWWW(this, 0), new C1458WWWW(8), new C2874WWWWWWWW(this, 1));

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public CommonEmptyView f8628WWWW;

    /* renamed from: Wo᪅Woै᪅  reason: contains not printable characters */
    public int f8629WoWo;

    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_carrier_list);
        View findViewById = findViewById(R.id.toolbar);
        AbstractC1017WWWoWWWo.m3443WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-100, -76, TarConstants.LF_CONTIG, -36, 23, -10, 32, 73, -72, -92, 16, -36, 105, -79, 107, 16, -45}, new byte[]{-6, -35, 89, -72, 65, -97, 69, 62}, findViewById);
        m2306WWWoWWWo((Toolbar) findViewById);
        WWWWWWWW m2307WWoWWo = m2307WWoWWo();
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(m2307WWoWWo);
        m2307WWoWWo.mo2341WoWo(true);
        View findViewById2 = findViewById(R.id.emptyView);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{25, 121, -81, -119, 22, -51, 86, ConstantPoolEntry.CP_NameAndType, 61, 105, -120, -119, 104, -118, 29, 85, 86}, new byte[]{Byte.MAX_VALUE, 16, -63, -19, 64, -92, TarConstants.LF_CHR, 123}));
        this.f8628WWWW = (CommonEmptyView) findViewById2;
        View findViewById3 = findViewById(R.id.listview);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById3, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{36, 68, -96, -20, -72, 45, -1, 9, 0, 84, -121, -20, -58, 106, -76, 80, 107}, new byte[]{66, 45, -50, -120, -18, 68, -102, 126}));
        this.f8625WWWWWWWW = (RecyclerView) findViewById3;
        LinearLayoutManager linearLayoutManager = new LinearLayoutManager(1);
        RecyclerView recyclerView = this.f8625WWWWWWWW;
        if (recyclerView != null) {
            recyclerView.setLayoutManager(linearLayoutManager);
            WoWo woWo = new WoWo(new WWoWWo(this));
            this.f8626WWWWWWWW = woWo;
            RecyclerView recyclerView2 = this.f8625WWWWWWWW;
            if (recyclerView2 != null) {
                recyclerView2.setAdapter(woWo);
                AbstractC2403WWWWoWWWWo.m13803WWWWWWWW(WWWWoWWWWo.m14598WWWWWWWW(this), null, new C2873WWWWWWWW(this, null), 3);
                RecyclerView recyclerView3 = this.f8625WWWWWWWW;
                if (recyclerView3 != null) {
                    p024WWWWWWWW.WWWWWWWW wwwwwwww = new p024WWWWWWWW.WWWWWWWW(26);
                    WeakHashMap weakHashMap = AbstractC0550WWWWWWWW.f2596WWWWWWWW;
                    AbstractC0593WoWo.m1914WoWo(recyclerView3, wwwwwwww);
                    return;
                }
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{93, -47, 65, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 87, -67, 74, 4, 71}, new byte[]{TarConstants.LF_NORMAL, -99, 40, 43, 35, -21, 35, 97}));
                throw null;
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-84, TarConstants.LF_GNUTYPE_LONGNAME, -54, TarConstants.LF_FIFO, -13, -5, -3, -59, -74}, new byte[]{-63, 0, -93, 69, -121, -83, -108, -96}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-1, -82, -116, 45, -24, 116, -41, -24, -27}, new byte[]{-110, -30, -27, 94, -100, 34, -66, -115}));
        throw null;
    }

    @Override // android.app.Activity
    public final boolean onCreateOptionsMenu(Menu menu) {
        byte[] bArr = {19, -93, -127, 107, -121, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 74, 124};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(menu, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{126, -58, -17, 30}, bArr));
        getMenuInflater().inflate(R.menu.carrier_list_menu, menu);
        SearchView searchView = (SearchView) menu.findItem(R.id.menu_search).getActionView();
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(searchView);
        searchView.setIconifiedByDefault(false);
        ImageView imageView = (ImageView) searchView.findViewById(R.id.search_mag_icon);
        if (imageView != null) {
            imageView.setImageDrawable(null);
        }
        searchView.setInputType(1);
        searchView.setImeOptions(3);
        searchView.setOnQueryTextListener(new C0279WWWWWWWW(20, this));
        int i10 = this.f8629WoWo;
        MenuItem findItem = menu.findItem(R.id.menu_country);
        if (findItem != null) {
            if (i10 != 0) {
                findItem.setIcon(i10);
            } else {
                findItem.setIcon(R.drawable.outline_filter_list_24);
            }
        }
        this.f8624WWWWWWWW = menu;
        return super.onCreateOptionsMenu(menu);
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, android.app.Activity
    public final boolean onOptionsItemSelected(MenuItem menuItem) {
        byte[] bArr = {-29, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -24, -74};
        byte[] bArr2 = {-118, 44, -115, -37, 84, 92, ConstantPoolEntry.CP_NameAndType, -69};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(menuItem, x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        if (menuItem.getItemId() == R.id.menu_country) {
            C2879WWWWWWWW c2879wwwwwwww = new C2879WWWWWWWW();
            c2879wwwwwwww.f37263r = new qh.WWWWWWWW(22, this);
            c2879wwwwwwww.m3400WWWWWWWW(m3298WWWWWWWW(), x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-79, -91, -61, 6, -5, 43, 99, 15, -101, -71, -62, 44, -26, 56, 118, 44, -107}, new byte[]{-14, -54, -74, 104, -113, 89, 26, 67}));
            return true;
        }
        return super.onOptionsItemSelected(menuItem);
    }
}
