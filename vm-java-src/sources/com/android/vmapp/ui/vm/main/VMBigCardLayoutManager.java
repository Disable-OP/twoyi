package com.android.vmapp.ui.vm.main;

import android.content.Context;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewConfiguration;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
/* loaded from: classes.dex */
public class VMBigCardLayoutManager extends LinearLayoutManager implements View.OnTouchListener {

    /* renamed from: WWWWoᜑWWWWoเᜑ  reason: contains not printable characters */
    public float f8635WWWWoWWWWo;

    /* renamed from: WWWWᗡWWWWنᗡ  reason: contains not printable characters */
    public final float f8636WWWWWWWW;

    /* renamed from: WWWWᜐWWWWଙᜐ  reason: contains not printable characters */
    public float f8637WWWWWWWW;

    /* renamed from: WWWoᜒWWWo೧ᜒ  reason: contains not printable characters */
    public boolean f8638WWWoWWWo;

    public VMBigCardLayoutManager(Context context) {
        super(0);
        this.f8637WWWWWWWW = 0.0f;
        this.f8635WWWWoWWWWo = 0.0f;
        this.f8638WWWoWWWo = false;
        this.f8636WWWWWWWW = ViewConfiguration.get(context).getScaledTouchSlop();
    }

    @Override // androidx.recyclerview.widget.LinearLayoutManager, androidx.recyclerview.widget.RecyclerView.WWoWWo
    /* renamed from: WWWWoᕭWWWWoࢨᕭ */
    public final void mo3686WWWWoWWWWo(RecyclerView recyclerView) {
        recyclerView.setOnTouchListener(null);
        this.f8638WWWoWWWo = false;
        this.f8637WWWWWWWW = 0.0f;
        this.f8635WWWWoWWWWo = 0.0f;
    }

    @Override // androidx.recyclerview.widget.LinearLayoutManager, androidx.recyclerview.widget.RecyclerView.WWoWWo
    /* renamed from: WWWWϙWWWWეϙ */
    public final boolean mo3688WWWWWWWW() {
        return !this.f8638WWWoWWWo;
    }

    @Override // androidx.recyclerview.widget.LinearLayoutManager, androidx.recyclerview.widget.RecyclerView.WWoWWo
    /* renamed from: WWWWҍWWWWּҍ */
    public final boolean mo3689WWWWWWWW() {
        return this.f8638WWWoWWWo;
    }

    @Override // androidx.recyclerview.widget.RecyclerView.WWoWWo
    /* renamed from: WWoᕛWWoउᕛ */
    public final void mo3836WWoWWo(RecyclerView recyclerView) {
        recyclerView.setOnTouchListener(this);
        this.f8638WWWoWWWo = false;
        this.f8637WWWWWWWW = 0.0f;
        this.f8635WWWWoWWWWo = 0.0f;
    }

    @Override // android.view.View.OnTouchListener
    public final boolean onTouch(View view, MotionEvent motionEvent) {
        if (motionEvent.getAction() == 0) {
            this.f8637WWWWWWWW = motionEvent.getX();
            this.f8635WWWWoWWWWo = motionEvent.getY();
        } else if (motionEvent.getAction() == 2) {
            if (this.f8637WWWWWWWW == 0.0f || this.f8635WWWWoWWWWo == 0.0f) {
                this.f8637WWWWWWWW = motionEvent.getX();
                this.f8635WWWWoWWWWo = motionEvent.getY();
            }
            float abs = Math.abs(motionEvent.getX() - this.f8637WWWWWWWW);
            float abs2 = Math.abs(motionEvent.getY() - this.f8635WWWWoWWWWo);
            if (abs2 > this.f8636WWWWWWWW && abs2 > abs) {
                this.f8638WWWoWWWo = true;
            }
        } else if (motionEvent.getAction() == 1 || motionEvent.getAction() == 3) {
            this.f8638WWWoWWWo = false;
            this.f8637WWWWWWWW = 0.0f;
            this.f8635WWWWoWWWWo = 0.0f;
        }
        return false;
    }
}
