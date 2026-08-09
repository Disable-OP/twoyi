package com.android.libadb.ui.base;

import android.os.Bundle;
import android.view.View;
import android.view.ViewGroup;
import android.view.Window;
import android.widget.FrameLayout;
import androidx.appcompat.app.AppCompatActivity;
import androidx.appcompat.widget.Toolbar;
import com.android.libadb.ui.base.AppBarActivity;
import com.clone.android.dual.space.R;
import com.google.android.material.appbar.AppBarLayout;
import com.google.android.material.internal.AbstractC2025WoWo;
import java.util.WeakHashMap;
import k3.C3232WWoWWo;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import me.AbstractC3506WWWWWWWW;
import n2.C3534WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import p069WoWo.AbstractC0550WWWWWWWW;
import p069WoWo.AbstractC0593WoWo;
import tc.WWWWWWWW;
/* loaded from: classes.dex */
public abstract class AppBarActivity extends AppCompatActivity {

    /* renamed from: WWWWᗡWWWWنᗡ  reason: contains not printable characters */
    public static final /* synthetic */ int f8296WWWWWWWW = 0;

    /* renamed from: WWoᕛWWoउᕛ  reason: contains not printable characters */
    public final Object f8299WWoWWo = AbstractC3506WWWWWWWW.m15931WWWWWWWW(new WWWWWWWW(this) { // from class: q2.WWWW̏WWWWβ̏

        /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
        public final /* synthetic */ AppBarActivity f32519WWWWWWWWWW;

        {
            this.f32519WWWWWWWWWW = this;
        }

        @Override // tc.WWWWWWWW
        public final Object invoke() {
            AppBarActivity appBarActivity = this.f32519WWWWWWWWWW;
            switch (r2) {
                case 0:
                    int i10 = AppBarActivity.f8296WWWWWWWW;
                    return (ViewGroup) appBarActivity.findViewById(R.id.root);
                case 1:
                    int i11 = AppBarActivity.f8296WWWWWWWW;
                    return (AppBarLayout) appBarActivity.findViewById(R.id.toolbar_container);
                default:
                    int i12 = AppBarActivity.f8296WWWWWWWW;
                    return (Toolbar) appBarActivity.findViewById(R.id.toolbar);
            }
        }
    });

    /* renamed from: WWWWoᕭWWWWoࢨᕭ  reason: contains not printable characters */
    public final Object f8297WWWWoWWWWo = AbstractC3506WWWWWWWW.m15931WWWWWWWW(new WWWWWWWW(this) { // from class: q2.WWWW̏WWWWβ̏

        /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
        public final /* synthetic */ AppBarActivity f32519WWWWWWWWWW;

        {
            this.f32519WWWWWWWWWW = this;
        }

        @Override // tc.WWWWWWWW
        public final Object invoke() {
            AppBarActivity appBarActivity = this.f32519WWWWWWWWWW;
            switch (r2) {
                case 0:
                    int i10 = AppBarActivity.f8296WWWWWWWW;
                    return (ViewGroup) appBarActivity.findViewById(R.id.root);
                case 1:
                    int i11 = AppBarActivity.f8296WWWWWWWW;
                    return (AppBarLayout) appBarActivity.findViewById(R.id.toolbar_container);
                default:
                    int i12 = AppBarActivity.f8296WWWWWWWW;
                    return (Toolbar) appBarActivity.findViewById(R.id.toolbar);
            }
        }
    });

    /* renamed from: WWWWᗘWWWWఛᗘ  reason: contains not printable characters */
    public final Object f8298WWWWWWWW = AbstractC3506WWWWWWWW.m15931WWWWWWWW(new WWWWWWWW(this) { // from class: q2.WWWW̏WWWWβ̏

        /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
        public final /* synthetic */ AppBarActivity f32519WWWWWWWWWW;

        {
            this.f32519WWWWWWWWWW = this;
        }

        @Override // tc.WWWWWWWW
        public final Object invoke() {
            AppBarActivity appBarActivity = this.f32519WWWWWWWWWW;
            switch (r2) {
                case 0:
                    int i10 = AppBarActivity.f8296WWWWWWWW;
                    return (ViewGroup) appBarActivity.findViewById(R.id.root);
                case 1:
                    int i11 = AppBarActivity.f8296WWWWWWWW;
                    return (AppBarLayout) appBarActivity.findViewById(R.id.toolbar_container);
                default:
                    int i12 = AppBarActivity.f8296WWWWWWWW;
                    return (Toolbar) appBarActivity.findViewById(R.id.toolbar);
            }
        }
    });

    @Override // androidx.appcompat.app.AppCompatActivity
    /* renamed from: WWWWၗWWWW३ၗ */
    public final boolean mo2305WWWWWWWW() {
        if (!super.mo2305WWWWWWWW()) {
            finish();
            return true;
        }
        return true;
    }

    /* JADX WARN: Type inference failed for: r0v0, types: [java.lang.Object, fc.WWWWҍWWWWּҍ] */
    /* renamed from: WWၚஊၚ  reason: contains not printable characters */
    public final ViewGroup m4836WW() {
        Object value = this.f8299WWoWWo.getValue();
        byte[] bArr = {TarConstants.LF_CONTIG, 57, -101, -55, -126, 98, 123, 107};
        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(value, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{80, 92, -17, -97, -29, 14, 14, 14, 31, 23, -75, -25, -85}, bArr));
        return (ViewGroup) value;
    }

    /* JADX WARN: Type inference failed for: r4v3, types: [java.lang.Object, fc.WWWWҍWWWWּҍ] */
    @Override // androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        C3232WWoWWo c3232WWoWWo = new C3232WWoWWo(16);
        Window window = getWindow();
        AbstractC2025WoWo.m12497WWWWWWWW(window, null);
        View decorView = window.getDecorView();
        WeakHashMap weakHashMap = AbstractC0550WWWWWWWW.f2596WWWWWWWW;
        AbstractC0593WoWo.m1914WoWo(decorView, c3232WWoWWo);
        super.setContentView(R.layout.activity_appbar);
        Object value = this.f8298WWWWWWWW.getValue();
        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(value, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{37, -108, 82, 60, -104, 111, 81, -8, 106, -33, 8, 68, -48}, new byte[]{66, -15, 38, 106, -7, 3, 36, -99}));
        m2306WWWoWWWo((Toolbar) value);
    }

    /* JADX WARN: Type inference failed for: r0v1, types: [java.lang.Object, fc.WWWWҍWWWWּҍ] */
    @Override // androidx.appcompat.app.AppCompatActivity, androidx.activity.ComponentActivity, android.app.Activity
    public final void setContentView(int i10) {
        getLayoutInflater().inflate(i10, m4836WW(), true);
        ViewGroup m4836WW = m4836WW();
        Object value = this.f8297WWWWoWWWWo.getValue();
        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(value, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-72, -58, -37, -20, -51, 70, -32, -92, -9, -115, -127, -108, -123}, new byte[]{-33, -93, -81, -70, -84, 42, -107, -63}));
        m4836WW.bringChildToFront((AppBarLayout) value);
    }

    @Override // androidx.appcompat.app.AppCompatActivity, androidx.activity.ComponentActivity, android.app.Activity
    public void setContentView(View view) {
        setContentView(view, new FrameLayout.LayoutParams(-1, -1));
    }

    @Override // androidx.appcompat.app.AppCompatActivity, androidx.activity.ComponentActivity, android.app.Activity
    public final void setContentView(View view, ViewGroup.LayoutParams layoutParams) {
        m4836WW().addView(view, 0, layoutParams);
    }
}
