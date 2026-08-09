package com.android.vmapp.ui.vm.add;

import a1.C0633WWWWWWWW;
import a3.WWWoWWWo;
import android.content.ClipData;
import android.content.ContentResolver;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.provider.DocumentsContract;
import android.util.Log;
import android.view.Menu;
import android.view.MenuItem;
import android.widget.ImageView;
import androidx.appcompat.app.C0791WWWWWWWW;
import androidx.appcompat.app.WWWW;
import androidx.appcompat.widget.SearchView;
import androidx.appcompat.widget.Toolbar;
import androidx.lifecycle.C1043WWWWoWWWWo;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import com.android.vmapp.ui.base.BaseActivity;
import com.android.vmapp.ui.vm.add.ImportsActivity;
import com.android.vmapp.ui.widget.CommonEmptyView;
import com.clone.android.dual.space.R;
import com.google.android.material.floatingactionbutton.FloatingActionButton;
import com.google.firebase.Firebase;
import com.google.firebase.analytics.AnalyticsKt;
import com.google.firebase.analytics.FirebaseAnalytics;
import com.google.firebase.analytics.ParametersBuilder;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import da.WWWWoWWWWo;
import f4.C2499WWWWWWWW;
import f4.C2502WWWWWWWW;
import f4.C2504WWWWWWWW;
import f4.DialogInterface$OnDismissListenerC2500WWWWWWWW;
import f4.WWoWWo;
import j3.C3164WWWWWWWW;
import java.util.ArrayList;
import java.util.List;
import java.util.WeakHashMap;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import l3.C3380WWWWWWWW;
import l3.C3398WWWWWWWW;
import l3.RunnableC3419WWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p025WWWWWWWW.C0279WWWWWWWW;
import p069WoWo.AbstractC0550WWWWWWWW;
import p069WoWo.AbstractC0593WoWo;
import t9.WWWWWWWW;
/* loaded from: classes.dex */
public class ImportsActivity extends BaseActivity {

    /* renamed from: WWWoᰠWWWoઠᰠ  reason: contains not printable characters */
    public static final /* synthetic */ int f8585WWWoWWWo = 0;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public Toolbar f8586WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public RecyclerView f8587WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public C2502WWWWWWWW f8588WWWWWWWW;

    /* renamed from: WWWWᬭWWWWɿᬭ  reason: contains not printable characters */
    public C2504WWWWWWWW f8589WWWWWWWW;

    /* renamed from: WWWWᮭWWWWᆏᮭ  reason: contains not printable characters */
    public WWWW f8590WWWWWWWW;

    /* renamed from: WWo᪙WWoг᪙  reason: contains not printable characters */
    public WWWWWWWW f8591WWoWWo;

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public CommonEmptyView f8592WWWW;

    /* renamed from: Wo᪅Woै᪅  reason: contains not printable characters */
    public FloatingActionButton f8593WoWo;

    static {
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{64, -100, -80, -111, 109, -40, 78, -109, 125, -104, -74, -105, 107, -43}, new byte[]{9, -15, -64, -2, 31, -84, 15, -16});
    }

    /* renamed from: WWWWoᆣWWWWoϿᆣ  reason: contains not printable characters */
    public final void m4954WWWWoWWWWo(Runnable runnable, String str) {
        WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
        wWWWoWWWWo.m13648WoWo(R.string.dialog_title_import_restart);
        ((C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW).f3561WWoWWo = getString(R.string.dialog_msg_import_restart, str);
        wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new c4.WWWWoWWWWo(1, this, runnable));
        wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, null);
        m4955WWoWWo(wWWWoWWWWo.mo742WWWW());
    }

    /* renamed from: WWoᆑWWoӁᆑ  reason: contains not printable characters */
    public final void m4955WWoWWo(WWWW wwww) {
        try {
            WWWW wwww2 = this.f8590WWWWWWWW;
            if (wwww2 != null) {
                wwww2.dismiss();
            }
            this.f8590WWWWWWWW = wwww;
            wwww.setOnDismissListener(new DialogInterface$OnDismissListenerC2500WWWWWWWW(this, 0));
            wwww.show();
        } catch (Throwable th2) {
            th2.printStackTrace();
        }
    }

    /* JADX WARN: Removed duplicated region for block: B:45:0x00ce A[LOOP:1: B:43:0x00cb->B:45:0x00ce, LOOP_END] */
    /* JADX WARN: Removed duplicated region for block: B:48:0x00dd A[LOOP:2: B:47:0x00db->B:48:0x00dd, LOOP_END] */
    /* JADX WARN: Removed duplicated region for block: B:63:0x0114  */
    /* JADX WARN: Removed duplicated region for block: B:78:0x00eb A[EXC_TOP_SPLITTER, SYNTHETIC] */
    /* JADX WARN: Removed duplicated region for block: B:85:? A[RETURN, SYNTHETIC] */
    @Override // androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, android.app.Activity
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final void onActivityResult(int i10, int i11, Intent intent) {
        Throwable th2;
        Exception exc;
        Cursor cursor;
        Uri[] uriArr;
        int length;
        int i12;
        int i13;
        super.onActivityResult(i10, i11, intent);
        if (i10 == 100 && i11 == -1 && intent != null) {
            ArrayList arrayList = new ArrayList();
            ClipData clipData = intent.getClipData();
            Cursor cursor2 = null;
            if (clipData == null) {
                Uri data = intent.getData();
                if (data != null) {
                    List<String> pathSegments = data.getPathSegments();
                    if (pathSegments.size() >= 2) {
                        byte[] bArr = {-27, 71, -22, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER};
                        byte[] bArr2 = {-111, TarConstants.LF_DIR, -113, 2, -99, 94, -103, -17};
                        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                        if (x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2).equals(pathSegments.get(0))) {
                            Uri data2 = intent.getData();
                            Uri buildDocumentUriUsingTree = DocumentsContract.buildDocumentUriUsingTree(data2, DocumentsContract.getTreeDocumentId(data2));
                            ContentResolver contentResolver = getContentResolver();
                            Uri buildChildDocumentsUriUsingTree = DocumentsContract.buildChildDocumentsUriUsingTree(buildDocumentUriUsingTree, DocumentsContract.getDocumentId(buildDocumentUriUsingTree));
                            ArrayList arrayList2 = new ArrayList();
                            try {
                                try {
                                    cursor = contentResolver.query(buildChildDocumentsUriUsingTree, new String[]{"document_id"}, null, null, null);
                                    while (cursor.moveToNext()) {
                                        try {
                                            try {
                                                arrayList2.add(DocumentsContract.buildDocumentUriUsingTree(buildDocumentUriUsingTree, cursor.getString(0)));
                                            } catch (Exception e10) {
                                                exc = e10;
                                                Log.w("DocumentFile", "Failed query: " + exc);
                                                if (cursor != null) {
                                                    try {
                                                        i0.WWWWWWWW.m14526WWoWWo(cursor);
                                                    } catch (RuntimeException e11) {
                                                        throw e11;
                                                    }
                                                }
                                                uriArr = (Uri[]) arrayList2.toArray(new Uri[arrayList2.size()]);
                                                length = uriArr.length;
                                                C0279WWWWWWWW[] c0279wwwwwwwwArr = new C0279WWWWWWWW[length];
                                                while (i12 < uriArr.length) {
                                                }
                                                while (i13 < length) {
                                                }
                                                if (arrayList.isEmpty()) {
                                                }
                                            }
                                        } catch (Throwable th3) {
                                            th2 = th3;
                                            cursor2 = cursor;
                                            if (cursor2 != null) {
                                                try {
                                                    i0.WWWWWWWW.m14526WWoWWo(cursor2);
                                                } catch (RuntimeException e12) {
                                                    throw e12;
                                                } catch (Exception unused) {
                                                }
                                            }
                                            throw th2;
                                        }
                                    }
                                    try {
                                        i0.WWWWWWWW.m14526WWoWWo(cursor);
                                    } catch (RuntimeException e13) {
                                        throw e13;
                                    }
                                } catch (Exception unused2) {
                                }
                            } catch (Exception e14) {
                                exc = e14;
                                cursor = null;
                            } catch (Throwable th4) {
                                th2 = th4;
                                if (cursor2 != null) {
                                }
                                throw th2;
                            }
                            uriArr = (Uri[]) arrayList2.toArray(new Uri[arrayList2.size()]);
                            length = uriArr.length;
                            C0279WWWWWWWW[] c0279wwwwwwwwArr2 = new C0279WWWWWWWW[length];
                            for (i12 = 0; i12 < uriArr.length; i12++) {
                                c0279wwwwwwwwArr2[i12] = new C0279WWWWWWWW(this, uriArr[i12]);
                            }
                            for (i13 = 0; i13 < length; i13++) {
                                arrayList.add((Uri) c0279wwwwwwwwArr2[i13].f1424WWWWWWWWWW);
                            }
                        }
                    }
                }
                if (data != null) {
                    arrayList.add(data);
                }
            } else {
                int itemCount = clipData.getItemCount();
                for (int i14 = 0; i14 < itemCount; i14++) {
                    arrayList.add(clipData.getItemAt(i14).getUri());
                }
            }
            if (arrayList.isEmpty()) {
                C3398WWWWWWWW c3398wwwwwwww = this.f8589WWWWWWWW.f26959WWWWWWWW;
                c3398wwwwwwww.m15743WWWWoWWWWo().post(new RunnableC3419WWWW(c3398wwwwwwww, arrayList, 0));
                WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
                wWWWoWWWWo.m13642WWWWWWWW(R.string.imports_dialog_msg_post_import_files);
                wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_got_it, null);
                m4955WWoWWo(wWWWoWWWWo.mo742WWWW());
            }
        }
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Type inference failed for: r8v12, types: [f4.WWWWӈWWWWीӈ, androidx.recyclerview.widget.RecyclerView$WWWW̏WWWWβ̏] */
    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_imports);
        if (this.f8505WWWWWWWW == null) {
            finish();
            return;
        }
        Toolbar toolbar = (Toolbar) findViewById(R.id.toolbar);
        this.f8586WWWWWWWW = toolbar;
        m2306WWWoWWWo(toolbar);
        m2307WWoWWo().mo2341WoWo(true);
        this.f8586WWWWWWWW.setOverflowIcon(getDrawable(R.drawable.outline_filter_list_24));
        this.f8587WWWWWWWW = (RecyclerView) findViewById(R.id.listview);
        this.f8587WWWWWWWW.setLayoutManager(new LinearLayoutManager(1));
        ?? wwwwwwww = new RecyclerView.WWWWWWWW();
        wwwwwwww.f26946WWWWWWWW = this;
        this.f8588WWWWWWWW = wwwwwwww;
        this.f8587WWWWWWWW.setAdapter(wwwwwwww);
        this.f8592WWWW = (CommonEmptyView) findViewById(R.id.emptyView);
        FloatingActionButton floatingActionButton = (FloatingActionButton) findViewById(R.id.add_imports);
        this.f8593WoWo = floatingActionButton;
        floatingActionButton.setOnClickListener(new WWWoWWWo(7, this));
        String str = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        if (getIntent() != null) {
            Intent intent = getIntent();
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            str = intent.getStringExtra(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{14, -2, 107, 89}, new byte[]{122, -121, 27, 60, -1, 64, -2, -2}));
        }
        byte[] bArr = {13, 47, 102, TarConstants.LF_BLK, 118, -117, 119, -54};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        if (x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{107, 70, 10, 81}, bArr).equals(str)) {
            this.f8593WoWo.performClick();
        }
        this.f8591WWoWWo = new WWWWWWWW(this, null);
        RecyclerView recyclerView = this.f8587WWWWWWWW;
        p024WWWWWWWW.WWWWWWWW wwwwwwww2 = new p024WWWWWWWW.WWWWWWWW(20);
        WeakHashMap weakHashMap = AbstractC0550WWWWWWWW.f2596WWWWWWWW;
        AbstractC0593WoWo.m1914WoWo(recyclerView, wwwwwwww2);
        C2504WWWWWWWW c2504wwwwwwww = (C2504WWWWWWWW) new C1043WWWWoWWWWo(this, new WWoWWo(this, 0)).m3493WWWWWWWW(C3333WWWWoWWWWo.m15421WWWWWWWW(C2504WWWWWWWW.class));
        this.f8589WWWWWWWW = c2504wwwwwwww;
        c2504wwwwwwww.f26962WWWWWWWW.m3570WWWWWWWW(this, new C2499WWWWWWWW(this, 0));
        this.f8589WWWWWWWW.f26974WW.m3570WWWWWWWW(this, new C2499WWWWWWWW(this, 1));
        this.f8589WWWWWWWW.f26958WWWWoWWWWo.m3570WWWWWWWW(this, new C2499WWWWWWWW(this, 2));
        this.f8589WWWWWWWW.f26963WWWWWWWW.m3570WWWWWWWW(this, new C2499WWWWWWWW(this, 3));
        this.f8589WWWWWWWW.f26964WWWWWWWW.m3570WWWWWWWW(this, new C2499WWWWWWWW(this, 4));
        C2504WWWWWWWW c2504wwwwwwww2 = this.f8589WWWWWWWW;
        synchronized (c2504wwwwwwww2.f26961WWWWWWWW) {
            try {
                if (c2504wwwwwwww2.f26970WWoWWo) {
                    return;
                }
                c2504wwwwwwww2.f26970WWoWWo = true;
                c2504wwwwwwww2.f26962WWWWWWWW.m3527WWWWWWWW(Boolean.TRUE);
                C3380WWWWWWWW.f30346WWWWWWWW.m15707WWWWWWWW();
                C3380WWWWWWWW.f30346WWWWWWWW.f30351WWWWWWWW.m3576WWoWWo(c2504wwwwwwww2.f26967WWWWWWWW);
            } catch (Throwable th2) {
                throw th2;
            }
        }
    }

    @Override // android.app.Activity
    public final boolean onCreateOptionsMenu(Menu menu) {
        getMenuInflater().inflate(R.menu.imports_menu, menu);
        SearchView searchView = (SearchView) menu.findItem(R.id.menu_search).getActionView();
        searchView.setIconifiedByDefault(false);
        ImageView imageView = (ImageView) searchView.findViewById(R.id.search_mag_icon);
        if (imageView != null) {
            imageView.setImageDrawable(null);
        }
        searchView.setInputType(1);
        searchView.setImeOptions(3);
        searchView.setOnQueryTextListener(new C0633WWWWWWWW(21, this));
        MenuItem findItem = menu.findItem(R.id.menu_not_show_system);
        findItem.setOnMenuItemClickListener(new MenuItem.OnMenuItemClickListener() { // from class: f4.WWWWo̐WWWWoȄ̐
            @Override // android.view.MenuItem.OnMenuItemClickListener
            public final boolean onMenuItemClick(MenuItem menuItem) {
                int i10 = 25;
                int i11 = ImportsActivity.f8585WWWoWWWo;
                ImportsActivity importsActivity = ImportsActivity.this;
                importsActivity.getClass();
                boolean isChecked = menuItem.isChecked();
                boolean z10 = !isChecked;
                menuItem.setChecked(z10);
                if (menuItem.getItemId() == R.id.menu_not_show_system) {
                    FirebaseAnalytics analytics = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
                    byte[] bArr = {112, 35, 27, TarConstants.LF_MULTIVOLUME, -99, -104, 102, 107};
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    String m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{19, 79, 114, 46, -10, -57, 14, 2, 20, 70, 68, 62, -28, -21, 18, 14, 29, 124, 122, 61, -19}, bArr);
                    ParametersBuilder parametersBuilder = new ParametersBuilder();
                    parametersBuilder.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-89, 57, -123, Byte.MIN_VALUE, -62}, new byte[]{-47, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -23, -11, -89, 66, 121, 35}), String.valueOf(z10));
                    analytics.logEvent(m17835WWWWWWWW, parametersBuilder.getBundle());
                    C2504WWWWWWWW c2504wwwwwwww = importsActivity.f8589WWWWWWWW;
                    c2504wwwwwwww.f26975WWWW = isChecked;
                    c2504wwwwwwww.m13993WWWoWWWo();
                } else if (menuItem.getItemId() == R.id.menu_not_show_32bit) {
                    FirebaseAnalytics analytics2 = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    String m17835WWWWWWWW2 = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-35, 86, 81, -66, 41, 25, 96, -99, -38, 95, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -18, 112, 36, 97, Byte.MIN_VALUE, -31, 91, 72, -83}, new byte[]{-66, 58, 56, -35, 66, 70, 8, -12});
                    ParametersBuilder parametersBuilder2 = new ParametersBuilder();
                    parametersBuilder2.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{45, -49, -57, 89, -8}, new byte[]{91, -82, -85, 44, -99, 8, 87, -99}), String.valueOf(z10));
                    analytics2.logEvent(m17835WWWWWWWW2, parametersBuilder2.getBundle());
                    C2504WWWWWWWW c2504wwwwwwww2 = importsActivity.f8589WWWWWWWW;
                    c2504wwwwwwww2.f26972WWoWWo = isChecked;
                    c2504wwwwwwww2.m13993WWWoWWWo();
                } else if (menuItem.getItemId() == R.id.menu_not_show_incompatible) {
                    FirebaseAnalytics analytics3 = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    String m17835WWWWWWWW3 = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -39, 104, 71, -95, -118, 28, TarConstants.LF_NORMAL, -105, -48, 94, TarConstants.LF_MULTIVOLUME, -92, -74, 27, TarConstants.LF_BLK, -125, -22, 96, 84, -70}, new byte[]{-13, -75, 1, 36, -54, -43, 116, 89});
                    ParametersBuilder parametersBuilder3 = new ParametersBuilder();
                    parametersBuilder3.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{21, 92, -41, -105, 102}, new byte[]{99, 61, -69, -30, 3, 21, -39, -13}), String.valueOf(z10));
                    analytics3.logEvent(m17835WWWWWWWW3, parametersBuilder3.getBundle());
                    C2504WWWWWWWW c2504wwwwwwww3 = importsActivity.f8589WWWWWWWW;
                    if (isChecked) {
                        i10 = 0;
                    }
                    c2504wwwwwwww3.f26966WWWWWWWW = i10;
                    c2504wwwwwwww3.m13993WWWoWWWo();
                }
                return true;
            }
        });
        findItem.setChecked(true);
        MenuItem findItem2 = menu.findItem(R.id.menu_not_show_32bit);
        findItem2.setOnMenuItemClickListener(new MenuItem.OnMenuItemClickListener() { // from class: f4.WWWWo̐WWWWoȄ̐
            @Override // android.view.MenuItem.OnMenuItemClickListener
            public final boolean onMenuItemClick(MenuItem menuItem) {
                int i10 = 25;
                int i11 = ImportsActivity.f8585WWWoWWWo;
                ImportsActivity importsActivity = ImportsActivity.this;
                importsActivity.getClass();
                boolean isChecked = menuItem.isChecked();
                boolean z10 = !isChecked;
                menuItem.setChecked(z10);
                if (menuItem.getItemId() == R.id.menu_not_show_system) {
                    FirebaseAnalytics analytics = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
                    byte[] bArr = {112, 35, 27, TarConstants.LF_MULTIVOLUME, -99, -104, 102, 107};
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    String m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{19, 79, 114, 46, -10, -57, 14, 2, 20, 70, 68, 62, -28, -21, 18, 14, 29, 124, 122, 61, -19}, bArr);
                    ParametersBuilder parametersBuilder = new ParametersBuilder();
                    parametersBuilder.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-89, 57, -123, Byte.MIN_VALUE, -62}, new byte[]{-47, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -23, -11, -89, 66, 121, 35}), String.valueOf(z10));
                    analytics.logEvent(m17835WWWWWWWW, parametersBuilder.getBundle());
                    C2504WWWWWWWW c2504wwwwwwww = importsActivity.f8589WWWWWWWW;
                    c2504wwwwwwww.f26975WWWW = isChecked;
                    c2504wwwwwwww.m13993WWWoWWWo();
                } else if (menuItem.getItemId() == R.id.menu_not_show_32bit) {
                    FirebaseAnalytics analytics2 = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    String m17835WWWWWWWW2 = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-35, 86, 81, -66, 41, 25, 96, -99, -38, 95, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -18, 112, 36, 97, Byte.MIN_VALUE, -31, 91, 72, -83}, new byte[]{-66, 58, 56, -35, 66, 70, 8, -12});
                    ParametersBuilder parametersBuilder2 = new ParametersBuilder();
                    parametersBuilder2.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{45, -49, -57, 89, -8}, new byte[]{91, -82, -85, 44, -99, 8, 87, -99}), String.valueOf(z10));
                    analytics2.logEvent(m17835WWWWWWWW2, parametersBuilder2.getBundle());
                    C2504WWWWWWWW c2504wwwwwwww2 = importsActivity.f8589WWWWWWWW;
                    c2504wwwwwwww2.f26972WWoWWo = isChecked;
                    c2504wwwwwwww2.m13993WWWoWWWo();
                } else if (menuItem.getItemId() == R.id.menu_not_show_incompatible) {
                    FirebaseAnalytics analytics3 = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    String m17835WWWWWWWW3 = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -39, 104, 71, -95, -118, 28, TarConstants.LF_NORMAL, -105, -48, 94, TarConstants.LF_MULTIVOLUME, -92, -74, 27, TarConstants.LF_BLK, -125, -22, 96, 84, -70}, new byte[]{-13, -75, 1, 36, -54, -43, 116, 89});
                    ParametersBuilder parametersBuilder3 = new ParametersBuilder();
                    parametersBuilder3.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{21, 92, -41, -105, 102}, new byte[]{99, 61, -69, -30, 3, 21, -39, -13}), String.valueOf(z10));
                    analytics3.logEvent(m17835WWWWWWWW3, parametersBuilder3.getBundle());
                    C2504WWWWWWWW c2504wwwwwwww3 = importsActivity.f8589WWWWWWWW;
                    if (isChecked) {
                        i10 = 0;
                    }
                    c2504wwwwwwww3.f26966WWWWWWWW = i10;
                    c2504wwwwwwww3.m13993WWWoWWWo();
                }
                return true;
            }
        });
        findItem2.setChecked(true);
        menu.findItem(R.id.menu_not_show_incompatible).setOnMenuItemClickListener(new MenuItem.OnMenuItemClickListener() { // from class: f4.WWWWo̐WWWWoȄ̐
            @Override // android.view.MenuItem.OnMenuItemClickListener
            public final boolean onMenuItemClick(MenuItem menuItem) {
                int i10 = 25;
                int i11 = ImportsActivity.f8585WWWoWWWo;
                ImportsActivity importsActivity = ImportsActivity.this;
                importsActivity.getClass();
                boolean isChecked = menuItem.isChecked();
                boolean z10 = !isChecked;
                menuItem.setChecked(z10);
                if (menuItem.getItemId() == R.id.menu_not_show_system) {
                    FirebaseAnalytics analytics = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
                    byte[] bArr = {112, 35, 27, TarConstants.LF_MULTIVOLUME, -99, -104, 102, 107};
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    String m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{19, 79, 114, 46, -10, -57, 14, 2, 20, 70, 68, 62, -28, -21, 18, 14, 29, 124, 122, 61, -19}, bArr);
                    ParametersBuilder parametersBuilder = new ParametersBuilder();
                    parametersBuilder.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-89, 57, -123, Byte.MIN_VALUE, -62}, new byte[]{-47, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -23, -11, -89, 66, 121, 35}), String.valueOf(z10));
                    analytics.logEvent(m17835WWWWWWWW, parametersBuilder.getBundle());
                    C2504WWWWWWWW c2504wwwwwwww = importsActivity.f8589WWWWWWWW;
                    c2504wwwwwwww.f26975WWWW = isChecked;
                    c2504wwwwwwww.m13993WWWoWWWo();
                } else if (menuItem.getItemId() == R.id.menu_not_show_32bit) {
                    FirebaseAnalytics analytics2 = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    String m17835WWWWWWWW2 = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-35, 86, 81, -66, 41, 25, 96, -99, -38, 95, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -18, 112, 36, 97, Byte.MIN_VALUE, -31, 91, 72, -83}, new byte[]{-66, 58, 56, -35, 66, 70, 8, -12});
                    ParametersBuilder parametersBuilder2 = new ParametersBuilder();
                    parametersBuilder2.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{45, -49, -57, 89, -8}, new byte[]{91, -82, -85, 44, -99, 8, 87, -99}), String.valueOf(z10));
                    analytics2.logEvent(m17835WWWWWWWW2, parametersBuilder2.getBundle());
                    C2504WWWWWWWW c2504wwwwwwww2 = importsActivity.f8589WWWWWWWW;
                    c2504wwwwwwww2.f26972WWoWWo = isChecked;
                    c2504wwwwwwww2.m13993WWWoWWWo();
                } else if (menuItem.getItemId() == R.id.menu_not_show_incompatible) {
                    FirebaseAnalytics analytics3 = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
                    C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                    String m17835WWWWWWWW3 = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-112, -39, 104, 71, -95, -118, 28, TarConstants.LF_NORMAL, -105, -48, 94, TarConstants.LF_MULTIVOLUME, -92, -74, 27, TarConstants.LF_BLK, -125, -22, 96, 84, -70}, new byte[]{-13, -75, 1, 36, -54, -43, 116, 89});
                    ParametersBuilder parametersBuilder3 = new ParametersBuilder();
                    parametersBuilder3.param(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{21, 92, -41, -105, 102}, new byte[]{99, 61, -69, -30, 3, 21, -39, -13}), String.valueOf(z10));
                    analytics3.logEvent(m17835WWWWWWWW3, parametersBuilder3.getBundle());
                    C2504WWWWWWWW c2504wwwwwwww3 = importsActivity.f8589WWWWWWWW;
                    if (isChecked) {
                        i10 = 0;
                    }
                    c2504wwwwwwww3.f26966WWWWWWWW = i10;
                    c2504wwwwwwww3.m13993WWWoWWWo();
                }
                return true;
            }
        });
        this.f8589WWWWWWWW.f26971WWoWWo.m3570WWWWWWWW(this, new C2499WWWWWWWW(this, 5));
        return super.onCreateOptionsMenu(menu);
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, android.app.Activity
    public final boolean onOptionsItemSelected(MenuItem menuItem) {
        if (menuItem.getItemId() == R.id.menu_import_records) {
            FirebaseAnalytics analytics = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
            byte[] bArr = {-72, ConstantPoolEntry.CP_InterfaceMethodref, -71, -81, -105, -88, -11, 44, -85, 8, -94, -72, -93, -123, -7, 34, -76, 21, -76};
            byte[] bArr2 = {-37, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -48, -52, -4, -9, -100, 65};
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            analytics.logEvent(x5.WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), new ParametersBuilder().getBundle());
            startActivity(new Intent(this, RecordsActivity.class));
            return true;
        }
        if (menuItem.getItemId() == R.id.menu_search) {
            FirebaseAnalytics analytics2 = AnalyticsKt.getAnalytics(Firebase.INSTANCE);
            byte[] bArr3 = {92, -12, -91, 105, -90, -26, -12, TarConstants.LF_BLK};
            C3164WWWWWWWW.f28918WWWWWWWW.getClass();
            i0.WWWWWWWW.m14515WWWWWWWW(analytics2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{63, -104, -52, 10, -51, -71, -121, 81, 61, -122, -58, 1, -7, -121, -124, 68}, bArr3));
        }
        return super.onOptionsItemSelected(menuItem);
    }
}
