package com.android.vmapp.ui.vm.privacy;

import android.os.Bundle;
import android.view.View;
import android.view.ViewGroup;
import com.android.vmapp.ui.base.BaseActivity;
import com.blankj.utilcode.util.WWWW;
import com.clone.android.dual.space.R;
/* loaded from: classes.dex */
public final class PrivacyMainActivity extends BaseActivity {
    @Override // com.android.vmapp.ui.base.BaseActivity, androidx.fragment.app.FragmentActivity, androidx.activity.ComponentActivity, androidx.core.app.ComponentActivity, android.app.Activity
    public final void onCreate(Bundle bundle) {
        ViewGroup.LayoutParams layoutParams;
        super.onCreate(bundle);
        setContentView(R.layout.activity_privacy_main);
        if (getResources().getConfiguration().orientation == 2) {
            View findViewById = findViewById(R.id.fragment_container_view);
            ViewGroup.MarginLayoutParams marginLayoutParams = null;
            if (findViewById != null) {
                layoutParams = findViewById.getLayoutParams();
            } else {
                layoutParams = null;
            }
            if (layoutParams instanceof ViewGroup.MarginLayoutParams) {
                marginLayoutParams = (ViewGroup.MarginLayoutParams) layoutParams;
            }
            if (marginLayoutParams != null) {
                marginLayoutParams.setMarginStart((WWWW.m5340WWoWWo(80.0f) + BaseActivity.f8501WWWoWWWo) / 2);
                marginLayoutParams.setMarginEnd((WWWW.m5340WWoWWo(80.0f) + BaseActivity.f8501WWWoWWWo) / 2);
            }
        }
    }
}
