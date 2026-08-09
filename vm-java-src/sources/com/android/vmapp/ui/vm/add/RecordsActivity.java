package com.android.vmapp.ui.vm.add;

import android.os.Bundle;
import androidx.appcompat.widget.Toolbar;
import androidx.lifecycle.C1043WWWWoWWWWo;
import androidx.lifecycle.InterfaceC1040WWWWoWWWWo;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import com.android.vmapp.ui.base.BaseActivity;
import com.android.vmapp.ui.vm.add.RecordsActivity;
import com.android.vmapp.ui.widget.CommonEmptyView;
import com.clone.android.dual.space.R;
import f4.C2506WWWWWWWW;
import f4.C2508WWoWWo;
import f4.WWoWWo;
import j3.C3164WWWWWWWW;
import java.util.List;
import java.util.WeakHashMap;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import p069WoWo.AbstractC0550WWWWWWWW;
import p069WoWo.AbstractC0593WoWo;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class RecordsActivity extends BaseActivity {

    /* renamed from: WWo᪙WWoг᪙  reason: contains not printable characters */
    public static final /* synthetic */ int f8594WWoWWo = 0;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public Toolbar f8595WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public RecyclerView f8596WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public C2508WWoWWo f8597WWWWWWWW;

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public CommonEmptyView f8598WWWW;

    /* renamed from: Wo᪅Woै᪅  reason: contains not printable characters */
    public C2506WWWWWWWW f8599WoWo;

    static {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        WWWWWWWW.m17835WWWWWWWW(new byte[]{-97, 18, -24, 10, 79, 115, -69, -66, -82, 3, -30, 19, 84, 99, -79}, new byte[]{-51, 119, -117, 101, 61, 23, -56, -1});
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_records);
        if (this.f8505WWWWWWWW == null) {
            finish();
            return;
        }
        Toolbar toolbar = (Toolbar) findViewById(R.id.toolbar);
        this.f8595WWWWWWWW = toolbar;
        m2306WWWoWWWo(toolbar);
        m2307WWoWWo().mo2341WoWo(true);
        this.f8595WWWWWWWW.setOverflowIcon(getDrawable(R.drawable.outline_filter_list_24));
        this.f8596WWWWWWWW = (RecyclerView) findViewById(R.id.listview);
        this.f8596WWWWWWWW.setLayoutManager(new LinearLayoutManager(1));
        C2508WWoWWo c2508WWoWWo = new C2508WWoWWo();
        this.f8597WWWWWWWW = c2508WWoWWo;
        this.f8596WWWWWWWW.setAdapter(c2508WWoWWo);
        this.f8598WWWW = (CommonEmptyView) findViewById(R.id.emptyView);
        RecyclerView recyclerView = this.f8596WWWWWWWW;
        p024WWWWWWWW.WWWWWWWW wwwwwwww = new p024WWWWWWWW.WWWWWWWW(21);
        WeakHashMap weakHashMap = AbstractC0550WWWWWWWW.f2596WWWWWWWW;
        AbstractC0593WoWo.m1914WoWo(recyclerView, wwwwwwww);
        C2506WWWWWWWW c2506wwwwwwww = (C2506WWWWWWWW) new C1043WWWWoWWWWo(this, new WWoWWo(this, 1)).m3493WWWWWWWW(C3333WWWWoWWWWo.m15421WWWWWWWW(C2506WWWWWWWW.class));
        this.f8599WoWo = c2506wwwwwwww;
        c2506wwwwwwww.f26993WoWo.m3570WWWWWWWW(this, new InterfaceC1040WWWWoWWWWo(this) { // from class: f4.WWَWWَཻ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ RecordsActivity f27002WWWWWWWWWW;

            {
                this.f27002WWWWWWWWWW = this;
            }

            @Override // androidx.lifecycle.InterfaceC1040WWWWoWWWWo
            /* renamed from: WWWW̏WWWWβ̏ */
            public final void mo497WWWWWWWW(Object obj) {
                RecordsActivity recordsActivity = this.f27002WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        int i10 = RecordsActivity.f8594WWoWWo;
                        recordsActivity.getClass();
                        if (((Boolean) obj).booleanValue()) {
                            recordsActivity.f8598WWWW.setVisibility(0);
                            recordsActivity.f8598WWWW.m5009WWWWWWWW(recordsActivity.getString(R.string.records_loading_tips));
                            recordsActivity.f8596WWWWWWWW.setVisibility(4);
                            return;
                        }
                        return;
                    default:
                        List list = (List) obj;
                        if (list != null) {
                            int i11 = RecordsActivity.f8594WWoWWo;
                            if (!list.isEmpty()) {
                                recordsActivity.f8598WWWW.setVisibility(4);
                                recordsActivity.f8596WWWWWWWW.setVisibility(0);
                                C2508WWoWWo c2508WWoWWo2 = recordsActivity.f8597WWWWWWWW;
                                c2508WWoWWo2.f27001WWWoWWWo = list;
                                c2508WWoWWo2.m3768WWWWWWWW();
                                return;
                            }
                        }
                        recordsActivity.f8598WWWW.setVisibility(0);
                        recordsActivity.f8598WWWW.m5008WWWWoWWWWo(recordsActivity.getString(R.string.records_empty_tips), null, null);
                        recordsActivity.f8596WWWWWWWW.setVisibility(4);
                        return;
                }
            }
        });
        this.f8599WoWo.f26992WW.m3570WWWWWWWW(this, new InterfaceC1040WWWWoWWWWo(this) { // from class: f4.WWَWWَཻ

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ RecordsActivity f27002WWWWWWWWWW;

            {
                this.f27002WWWWWWWWWW = this;
            }

            @Override // androidx.lifecycle.InterfaceC1040WWWWoWWWWo
            /* renamed from: WWWW̏WWWWβ̏ */
            public final void mo497WWWWWWWW(Object obj) {
                RecordsActivity recordsActivity = this.f27002WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        int i10 = RecordsActivity.f8594WWoWWo;
                        recordsActivity.getClass();
                        if (((Boolean) obj).booleanValue()) {
                            recordsActivity.f8598WWWW.setVisibility(0);
                            recordsActivity.f8598WWWW.m5009WWWWWWWW(recordsActivity.getString(R.string.records_loading_tips));
                            recordsActivity.f8596WWWWWWWW.setVisibility(4);
                            return;
                        }
                        return;
                    default:
                        List list = (List) obj;
                        if (list != null) {
                            int i11 = RecordsActivity.f8594WWoWWo;
                            if (!list.isEmpty()) {
                                recordsActivity.f8598WWWW.setVisibility(4);
                                recordsActivity.f8596WWWWWWWW.setVisibility(0);
                                C2508WWoWWo c2508WWoWWo2 = recordsActivity.f8597WWWWWWWW;
                                c2508WWoWWo2.f27001WWWoWWWo = list;
                                c2508WWoWWo2.m3768WWWWWWWW();
                                return;
                            }
                        }
                        recordsActivity.f8598WWWW.setVisibility(0);
                        recordsActivity.f8598WWWW.m5008WWWWoWWWWo(recordsActivity.getString(R.string.records_empty_tips), null, null);
                        recordsActivity.f8596WWWWWWWW.setVisibility(4);
                        return;
                }
            }
        });
        C2506WWWWWWWW c2506wwwwwwww2 = this.f8599WoWo;
        synchronized (c2506wwwwwwww2.f26989WWWoWWWo) {
            try {
                if (c2506wwwwwwww2.f26990WWoWWo) {
                    return;
                }
                if (c2506wwwwwwww2.f26987WWWWWWWW) {
                    return;
                }
                c2506wwwwwwww2.f26990WWoWWo = true;
                c2506wwwwwwww2.f26993WoWo.m3527WWWWWWWW(Boolean.TRUE);
                c2506wwwwwwww2.f26985WWWWWWWW.f30478WWWWWWWW.m3576WWoWWo(c2506wwwwwwww2.f26988WWWWWWWW);
            } catch (Throwable th2) {
                throw th2;
            }
        }
    }
}
