package com.android.libadb.ui;

import android.content.Context;
import android.content.Intent;
import android.os.Bundle;
import android.view.View;
import com.android.libadb.ui.AdbActivationMethodActivity;
import com.android.libadb.ui.AdbActivationTutorialActivity;
import com.android.libadb.ui.AdbActivationTutorialActivity2;
import com.android.libadb.ui.base.AppBarActivity;
import com.clone.android.dual.space.R;
/* loaded from: classes.dex */
public class AdbActivationMethodActivity extends AppBarActivity {

    /* renamed from: WWWoᜒWWWo೧ᜒ  reason: contains not printable characters */
    public static final /* synthetic */ int f8266WWWoWWWo = 0;

    /* renamed from: WWWWoᜑWWWWoเᜑ  reason: contains not printable characters */
    public View f8267WWWWoWWWWo;

    /* renamed from: WWWWᜐWWWWଙᜐ  reason: contains not printable characters */
    public View f8268WWWWWWWW;

    @Override // com.android.libadb.ui.base.AppBarActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        setContentView(R.layout.activity_activation_method);
        m2307WWoWWo().mo2341WoWo(true);
        View findViewById = findViewById(R.id.method1);
        this.f8268WWWWWWWW = findViewById;
        findViewById.setOnClickListener(new View.OnClickListener(this) { // from class: p2.WWWW̏WWWWβ̏

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbActivationMethodActivity f32181WWWWWWWWWW;

            {
                this.f32181WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                Context context = this.f32181WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        int i10 = AdbActivationMethodActivity.f8266WWWoWWWo;
                        context.getClass();
                        Intent intent = new Intent();
                        intent.setClass(context, AdbActivationTutorialActivity.class);
                        context.startActivity(intent);
                        return;
                    default:
                        int i11 = AdbActivationMethodActivity.f8266WWWoWWWo;
                        context.getClass();
                        Intent intent2 = new Intent();
                        intent2.setClass(context, AdbActivationTutorialActivity2.class);
                        context.startActivity(intent2);
                        return;
                }
            }
        });
        View findViewById2 = findViewById(R.id.method2);
        this.f8267WWWWoWWWWo = findViewById2;
        findViewById2.setOnClickListener(new View.OnClickListener(this) { // from class: p2.WWWW̏WWWWβ̏

            /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
            public final /* synthetic */ AdbActivationMethodActivity f32181WWWWWWWWWW;

            {
                this.f32181WWWWWWWWWW = this;
            }

            @Override // android.view.View.OnClickListener
            public final void onClick(View view) {
                Context context = this.f32181WWWWWWWWWW;
                switch (r2) {
                    case 0:
                        int i10 = AdbActivationMethodActivity.f8266WWWoWWWo;
                        context.getClass();
                        Intent intent = new Intent();
                        intent.setClass(context, AdbActivationTutorialActivity.class);
                        context.startActivity(intent);
                        return;
                    default:
                        int i11 = AdbActivationMethodActivity.f8266WWWoWWWo;
                        context.getClass();
                        Intent intent2 = new Intent();
                        intent2.setClass(context, AdbActivationTutorialActivity2.class);
                        context.startActivity(intent2);
                        return;
                }
            }
        });
    }
}
