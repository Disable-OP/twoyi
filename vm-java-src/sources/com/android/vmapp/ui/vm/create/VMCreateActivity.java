package com.android.vmapp.ui.vm.create;

import android.os.Bundle;
import android.view.View;
import android.widget.Button;
import androidx.appcompat.widget.Toolbar;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import androidx.fragment.app.Fragment;
import androidx.fragment.app.FragmentManager;
import androidx.navigation.fragment.NavHostFragment;
import b4.WWoWWo;
import com.android.vmapp.ui.base.BaseActivity;
import com.android.vmapp.ui.vm.resolution.ResolutionFragment;
import com.android.vmapp.ui.vm.resolution.WWWWWWWW;
import com.android.vmapp.ui.vm.rom.RomFragment;
import com.android.vmcore.VMResConfig;
import com.clone.android.dual.space.R;
import com.google.android.gms.internal.consent_sdk.AbstractC1812WWWW;
import gc.C2601WWWWWWWW;
import h4.C2706WWWWWWWW;
import j3.C3164WWWWWWWW;
import j4.WWWWoWWWWo;
import java.util.HashSet;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import kotlin.jvm.internal.C3333WWWWoWWWWo;
import l1.WWWW;
import n4.WWWoWWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import p.C3784WWWWWWWW;
import p.C3797WWWoWWWo;
import p001WWWWoWWWWo.C0066WWWWWWWW;
import p013WWWWWWWW.o;
/* loaded from: classes.dex */
public final class VMCreateActivity extends BaseActivity implements WWWoWWWo, WWWWWWWW {

    /* renamed from: Wo᪅Woै᪅  reason: contains not printable characters */
    public static final /* synthetic */ int f8630WoWo = 0;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public C3784WWWWWWWW f8631WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public Toolbar f8632WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public Button f8633WWWWWWWW;

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public final o f8634WWWW = new o(C3333WWWWoWWWWo.m15421WWWWWWWW(j4.WWWoWWWo.class), new WWWWoWWWWo(this, 1), new WWWWoWWWWo(this, 0), new WWWWoWWWWo(this, 2));

    @Override // com.android.vmapp.ui.vm.resolution.WWWWWWWW
    /* renamed from: WWWWآWWWWȫآ  reason: contains not printable characters */
    public final void mo4964WWWWWWWW(ResolutionFragment.WWWWWWWW wwwwwwww, boolean z10) {
        Bundle m6952WWWWWWWW;
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(wwwwwwww, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, -34, 15, -77, 57}, new byte[]{-13, -79, 107, -42, 85, 124, 39, 119}));
        m4965WWoWWo().f28932WWWWWWWW = wwwwwwww.f8732WWWWWWWW;
        C3784WWWWWWWW c3784wwwwwwww = this.f8631WWWWWWWW;
        if (c3784wwwwwwww != null) {
            C3797WWWoWWWo c3797WWWoWWWo = (C3797WWWoWWWo) c3784wwwwwwww.f32050WWWWoWWWWo.f33400WWoWWo.m14123WWWWWWWW();
            if (c3797WWWoWWWo != null && (m6952WWWWWWWW = c3797WWWoWWWo.f32086WWWoWWWo.m6952WWWWWWWW()) != null) {
                String m17835WWWWWWWW = x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-42, -125, -9, -64, -39, TarConstants.LF_GNUTYPE_LONGNAME, 31, TarConstants.LF_CONTIG, -57, -109, -10, -35, -48, 87, 31, 1, -38, -104}, new byte[]{-75, -10, -123, -78, -68, 34, 107, 104});
                VMResConfig vMResConfig = m4965WWoWWo().f28932WWWWWWWW;
                AbstractC3339WWWWWWWW.m15436WWWoWWWo(vMResConfig);
                m6952WWWWWWWW.putString(m17835WWWWWWWW, vMResConfig.f8953WWWWWWWW);
                return;
            }
            return;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{99, -112, -103, -87, -124, -108, -2, -83, 98, -99, -125, -113, -103}, new byte[]{13, -15, -17, -22, -21, -6, -118, -33}));
        throw null;
    }

    @Override // androidx.appcompat.app.AppCompatActivity
    /* renamed from: WWWWၗWWWW३ၗ */
    public final boolean mo2305WWWWWWWW() {
        finish();
        return true;
    }

    /* renamed from: WWoᆑWWoӁᆑ  reason: contains not printable characters */
    public final j4.WWWoWWWo m4965WWoWWo() {
        return (j4.WWWoWWWo) this.f8634WWWW.getValue();
    }

    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_vm_create);
        View findViewById = findViewById(R.id.toolbar);
        byte[] bArr = {-30, -74, -57, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -94, -55, 22, 17};
        AbstractC1017WWWoWWWo.m3443WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-124, -33, -87, 28, -12, -96, 115, 102, -96, -49, -114, 28, -118, -25, 56, 63, -53}, bArr, findViewById);
        Toolbar toolbar = (Toolbar) findViewById;
        this.f8632WWWWWWWW = toolbar;
        m2306WWWoWWWo(toolbar);
        androidx.appcompat.app.WWWWWWWW m2307WWoWWo = m2307WWoWWo();
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(m2307WWoWWo);
        m2307WWoWWo.mo2341WoWo(true);
        Fragment m3350WWoWWo = m3298WWWWWWWW().m3350WWoWWo(R.id.nav_host_container);
        AbstractC3339WWWWWWWW.m15428WWWWWWWW(m3350WWoWWo, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{97, -80, 108, -17, -43, -116, -117, 124, 97, -86, 116, -93, -105, -118, -54, 113, 110, -74, 116, -93, -127, Byte.MIN_VALUE, -54, 124, 96, -85, 45, -19, Byte.MIN_VALUE, -125, -122, TarConstants.LF_SYMLINK, 123, -68, 112, -26, -43, -114, -124, 118, 125, -86, 105, -25, -115, -63, -124, 115, 121, -84, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -30, -127, -122, -123, 124, 33, -93, 114, -30, -110, -126, -113, 124, 123, -21, 78, -30, -125, -89, -123, 97, 123, -125, 114, -30, -110, -126, -113, 124, 123}, new byte[]{15, -59, 0, -125, -11, -17, -22, 18}));
        NavHostFragment navHostFragment = (NavHostFragment) m3350WWoWWo;
        this.f8631WWWWWWWW = navHostFragment.m3584WWoWWo();
        C2601WWWWWWWW c2601wwwwwwww = C2601WWWWWWWW.f27306WWWWoWWWWo;
        C0066WWWWWWWW c0066wwwwwwww = new C0066WWWWWWWW(0, this, VMCreateActivity.class, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{123, Byte.MAX_VALUE, -1, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 22, 6, 14, 111, 96, 95, -51, 91, 15, 17, 0, 105, 113, 68, -36}, new byte[]{20, 17, -84, 45, 102, 118, 97, 29}), x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-29, -62, -75, 13, -11, 13, -105, 108, -8, -30, -121, 14, -20, 26, -103, 106, -23, -7, -106, 80, -84, 39}, new byte[]{-116, -84, -26, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -123, 125, -8, 30}), 0, 4);
        HashSet hashSet = new HashSet();
        hashSet.addAll(c2601wwwwwwww);
        WWWW wwww = new WWWW(29, hashSet, new C2706WWWWWWWW(c0066wwwwwwww, (byte) 0));
        Toolbar toolbar2 = this.f8632WWWWWWWW;
        if (toolbar2 != null) {
            C3784WWWWWWWW c3784wwwwwwww = this.f8631WWWWWWWW;
            if (c3784wwwwwwww != null) {
                AbstractC1812WWWW.m10938WWWWWWWW(toolbar2, c3784wwwwwwww, wwww);
                View findViewById2 = findViewById(R.id.button);
                AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{97, -31, -106, -68, 72, -16, -105, -2, 69, -15, -79, -68, TarConstants.LF_FIFO, -73, -36, -89, 46}, new byte[]{7, -120, -8, -40, 30, -103, -14, -119}));
                Button button = (Button) findViewById2;
                this.f8633WWWWWWWW = button;
                button.setOnClickListener(new WWoWWo(6, this, navHostFragment));
                C3784WWWWWWWW c3784wwwwwwww2 = this.f8631WWWWWWWW;
                if (c3784wwwwwwww2 != null) {
                    c3784wwwwwwww2.m16413WWWWWWWW(new h4.WWWoWWWo(this, 1));
                    Fragment fragment = navHostFragment.m3289WWWW().f5331WWWWWWWWWW;
                    if (fragment instanceof RomFragment) {
                        ((RomFragment) fragment).m5006o(this);
                    } else if (fragment instanceof ResolutionFragment) {
                        ((ResolutionFragment) fragment).f36454b = this;
                    }
                    FragmentManager m3289WWWW = navHostFragment.m3289WWWW();
                    m3289WWWW.f5343WWWWWWWW.add(new j4.WWWWWWWW(0, this));
                    return;
                }
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{0, -86, -75, 60, -90, 80, -93, -58, 1, -89, -81, 26, -69}, new byte[]{110, -53, -61, Byte.MAX_VALUE, -55, 62, -41, -76}));
                throw null;
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{84, -63, -66, -93, 42, 24, 2, TarConstants.LF_DIR, 85, -52, -92, -123, TarConstants.LF_CONTIG}, new byte[]{58, -96, -56, -32, 69, 118, 118, 71}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{123, 79, -98, 34, 72, -72, 57}, new byte[]{15, 32, -15, 78, 42, -39, TarConstants.LF_GNUTYPE_LONGLINK, -58}));
        throw null;
    }
}
