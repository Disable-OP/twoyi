package com.android.vmapp.ui.vm.create;

import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.view.Window;
import android.view.inputmethod.InputMethodManager;
import android.widget.EditText;
import androidx.appcompat.widget.AppCompatEditText;
import com.blankj.utilcode.util.WWWW;
import com.clone.android.dual.space.R;
import j3.C3164WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import v3.WWWWWWWW;
/* loaded from: classes.dex */
public final class VMNamingFragment extends WWWWWWWW {
    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWWWᐡWWWWೱᐡ */
    public final View mo2047WWWWWWWW(LayoutInflater layoutInflater, ViewGroup viewGroup, Bundle bundle) {
        byte[] bArr = {-3, 122, 95, 82, TarConstants.LF_CONTIG, -65, 21, -3};
        C3164WWWWWWWW.f28918WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15439WWoWWo(layoutInflater, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-108, 20, 57, 62, 86, -53, 112, -113}, bArr));
        View inflate = layoutInflater.inflate(R.layout.fragment_vm_naming, viewGroup, false);
        AppCompatEditText appCompatEditText = (AppCompatEditText) inflate.findViewById(R.id.input_box);
        if (this.f5304WWoWWo != null) {
            appCompatEditText.setText(m3296WoWo().getString(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{93, -99, 59, 85, -66, TarConstants.LF_GNUTYPE_LONGNAME, -23, -46, 87, -103, TarConstants.LF_NORMAL, 81}, new byte[]{57, -8, 93, TarConstants.LF_BLK, -53, 32, -99, -115})));
        }
        appCompatEditText.requestFocus();
        return inflate;
    }

    @Override // androidx.fragment.app.Fragment
    /* renamed from: WWᐤԂᐤ */
    public final void mo3292WW() {
        this.f5273WWWWoWWWWo = true;
        Window window = m3293WWWW().getWindow();
        if (window != null) {
            View currentFocus = window.getCurrentFocus();
            if (currentFocus == null) {
                View decorView = window.getDecorView();
                View findViewWithTag = decorView.findViewWithTag("keyboardTagView");
                if (findViewWithTag == null) {
                    findViewWithTag = new EditText(window.getContext());
                    findViewWithTag.setTag("keyboardTagView");
                    ((ViewGroup) decorView).addView(findViewWithTag, 0, 0);
                }
                currentFocus = findViewWithTag;
                currentFocus.requestFocus();
            }
            InputMethodManager inputMethodManager = (InputMethodManager) WWWW.m5336WWWoWWWo().getSystemService("input_method");
            if (inputMethodManager == null) {
                return;
            }
            inputMethodManager.hideSoftInputFromWindow(currentFocus.getWindowToken(), 0);
        }
    }
}
