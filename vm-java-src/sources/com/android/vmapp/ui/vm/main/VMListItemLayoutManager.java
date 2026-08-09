package com.android.vmapp.ui.vm.main;

import android.content.Context;
import android.util.AttributeSet;
import androidx.recyclerview.widget.C1167WWWWWWWW;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
/* loaded from: classes.dex */
public class VMListItemLayoutManager extends LinearLayoutManager {

    /* renamed from: WWWWᗡWWWWنᗡ  reason: contains not printable characters */
    public RecyclerView f8657WWWWWWWW;

    public VMListItemLayoutManager(Context context, AttributeSet attributeSet, int i10, int i11) {
        super(context, attributeSet, i10, i11);
    }

    @Override // androidx.recyclerview.widget.LinearLayoutManager, androidx.recyclerview.widget.RecyclerView.WWoWWo
    /* renamed from: WWWWoᕭWWWWoࢨᕭ */
    public final void mo3686WWWWoWWWWo(RecyclerView recyclerView) {
        this.f8657WWWWWWWW = null;
    }

    @Override // androidx.recyclerview.widget.LinearLayoutManager, androidx.recyclerview.widget.RecyclerView.WWoWWo
    /* renamed from: WWWWίWWWWРί */
    public final int mo3672WWWWWWWW(int i10, C1167WWWWWWWW c1167wwwwwwww, RecyclerView.WWWW wwww) {
        RecyclerView recyclerView = this.f8657WWWWWWWW;
        if (recyclerView != null && recyclerView.getScrollState() == 1 && recyclerView.getTranslationY() != 0.0f) {
            return 0;
        }
        return super.mo3672WWWWWWWW(i10, c1167wwwwwwww, wwww);
    }

    @Override // androidx.recyclerview.widget.RecyclerView.WWoWWo
    /* renamed from: WWoᕛWWoउᕛ */
    public final void mo3836WWoWWo(RecyclerView recyclerView) {
        this.f8657WWWWWWWW = recyclerView;
    }
}
