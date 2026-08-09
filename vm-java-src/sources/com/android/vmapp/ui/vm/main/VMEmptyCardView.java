package com.android.vmapp.ui.vm.main;

import a3.WWWoWWWo;
import android.content.Context;
import android.text.method.ScrollingMovementMethod;
import android.util.AttributeSet;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.TextView;
import com.clone.android.dual.space.R;
import com.google.android.material.button.MaterialButton;
import i0.WWWWWWWW;
import j3.C3164WWWWWWWW;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
/* loaded from: classes.dex */
public final class VMEmptyCardView extends FrameLayout {

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public static final /* synthetic */ int f8653WWWWoWWWWo = 0;

    /* JADX WARN: 'this' call moved to the top of the method (can break code semantics) */
    public VMEmptyCardView(Context context) {
        this(context, null, 6, 0);
        byte[] bArr = {-29, -94, TarConstants.LF_NORMAL, 46, -15, -115, -51, 38};
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{Byte.MIN_VALUE, -51, 94, 90, -108, -11, -71}, bArr, context);
    }

    /* JADX WARN: 'this' call moved to the top of the method (can break code semantics) */
    public VMEmptyCardView(Context context, AttributeSet attributeSet) {
        this(context, attributeSet, 4, 0);
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{-116, -86, 123, -32, TarConstants.LF_MULTIVOLUME, 86, -39}, new byte[]{-17, -59, 21, -108, 40, 46, -83, 60}, context);
    }

    /* JADX WARN: 'super' call moved to the top of the method (can break code semantics) */
    public VMEmptyCardView(Context context, AttributeSet attributeSet, int i10) {
        super(context, attributeSet, i10);
        WWWWWWWW.m14516WWWWWWWW(C3164WWWWWWWW.f28918WWWWWWWW, new byte[]{Byte.MIN_VALUE, 82, -73, -41, 69, -36, 90}, new byte[]{-29, 61, -39, -93, 32, -92, 46, -64}, context);
        LayoutInflater.from(getContext()).inflate(R.layout.view_vm_empty_card, (ViewGroup) this, true);
        View findViewById = findViewById(R.id.slogan);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{125, 73, 3, 19, TarConstants.LF_FIFO, -57, 93, -45, 89, 89, 36, 19, 72, Byte.MIN_VALUE, 22, -118, TarConstants.LF_SYMLINK}, new byte[]{27, 32, 109, 119, 96, -82, 56, -92}));
        ((TextView) findViewById).setMovementMethod(new ScrollingMovementMethod());
        View findViewById2 = findViewById(R.id.start);
        AbstractC3339WWWWWWWW.m15429WWWWWWWW(findViewById2, x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{29, ConstantPoolEntry.CP_InterfaceMethodref, -112, 99, 59, -90, -122, -24, 57, 27, -73, 99, 69, -31, -51, -79, 82}, new byte[]{123, 98, -2, 7, 109, -49, -29, -97}));
        ((MaterialButton) findViewById2).setOnClickListener(new WWWoWWWo(11, this));
    }

    public /* synthetic */ VMEmptyCardView(Context context, AttributeSet attributeSet, int i10, int i11) {
        this(context, (i10 & 2) != 0 ? null : attributeSet, 0);
    }
}
