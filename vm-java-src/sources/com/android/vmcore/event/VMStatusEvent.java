package com.android.vmcore.event;
/* loaded from: classes.dex */
public class VMStatusEvent {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final int f9015WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final int f9016WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final String f9017WWWoWWWo;

    public VMStatusEvent(int i10, int i11) {
        this.f9016WWWWWWWW = i10;
        this.f9015WWWWoWWWWo = i11;
    }

    public VMStatusEvent(String str) {
        this.f9016WWWWWWWW = 4;
        this.f9015WWWWoWWWWo = 0;
        this.f9017WWWoWWWo = str;
    }
}
