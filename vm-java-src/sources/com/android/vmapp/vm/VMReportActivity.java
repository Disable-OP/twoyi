package com.android.vmapp.vm;

import a2.C0639WWWWWWWW;
import android.content.Intent;
import android.opengl.GLSurfaceView;
import android.os.Bundle;
import android.view.ViewGroup;
import androidx.appcompat.app.C0791WWWWWWWW;
import androidx.appcompat.app.WWWW;
import com.android.vmapp.ui.base.BaseActivity;
import com.clone.android.dual.space.R;
import da.WWWWoWWWWo;
import j3.C3164WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import r4.C3966WWWWWWWW;
import r4.DialogInterface$OnCancelListenerC3963WWWWWWWW;
import r4.WWoWWo;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class VMReportActivity extends BaseActivity {

    /* renamed from: Wo᪅Woै᪅  reason: contains not printable characters */
    public static final /* synthetic */ int f8771WoWo = 0;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public WWWW f8772WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public boolean f8773WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public String f8774WWWWWWWW;

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public C3966WWWWWWWW f8775WWWW;

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Type inference failed for: r2v6, types: [java.lang.Object, android.opengl.GLSurfaceView$Renderer] */
    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        getWindow().setBackgroundDrawableResource(17170445);
        super.onCreate(bundle);
        overridePendingTransition(0, 0);
        if (this.f8505WWWWWWWW == null) {
            finish();
            return;
        }
        Intent intent = getIntent();
        byte[] bArr = {89, 38, -13, 73, 6, 41, TarConstants.LF_SYMLINK, 81, 90, TarConstants.LF_FIFO, -15};
        byte[] bArr2 = {63, TarConstants.LF_GNUTYPE_SPARSE, -97, 37, 89, 90, 81, 35};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        this.f8773WWWWWWWW = intent.getBooleanExtra(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), false);
        this.f8774WWWWWWWW = getIntent().getStringExtra(WWWWWWWW.m17835WWWWWWWW(new byte[]{87, 68, -7, 1}, new byte[]{TarConstants.LF_LINK, TarConstants.LF_FIFO, -106, 108, -54, -53, -72, -85}));
        GLSurfaceView gLSurfaceView = new GLSurfaceView(this);
        setContentView(gLSurfaceView, new ViewGroup.LayoutParams(1, 1));
        gLSurfaceView.setRenderer(new Object());
        WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(this);
        wWWWoWWWWo.m13648WoWo(R.string.dialog_title_vm_report);
        wWWWoWWWWo.m13642WWWWWWWW(R.string.dialog_msg_vm_report);
        wWWWoWWWWo.m13645WWWoWWWo(R.string.dialog_button_confirm, new WWoWWo(this, 0));
        wWWWoWWWWo.m13646WWoWWo(R.string.dialog_button_cancel, new n2.WWWWWWWW(6));
        ((C0791WWWWWWWW) wWWWoWWWWo.f1045WWWWWWWW).f3562WWoWWo = new DialogInterface$OnCancelListenerC3963WWWWWWWW(this, 0);
        WWWW mo742WWWW = wWWWoWWWWo.mo742WWWW();
        mo742WWWW.show();
        if (this.f8773WWWWWWWW) {
            mo742WWWW.getWindow().getDecorView().setSystemUiVisibility(5894);
        }
        this.f8775WWWW = new C3966WWWWWWWW(getApplication(), this.f8505WWWWWWWW, new C0639WWWWWWWW(26, this));
    }
}
