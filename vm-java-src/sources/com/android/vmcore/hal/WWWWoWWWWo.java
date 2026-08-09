package com.android.vmcore.hal;
/* renamed from: com.android.vmcore.hal.WWWWo̐WWWWoȄ̐  reason: invalid class name */
/* loaded from: classes.dex */
public final /* synthetic */ class WWWWoWWWWo implements Runnable {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final /* synthetic */ NetlinkManager f9109WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final /* synthetic */ int f9110WWWWoWWWWo;

    public /* synthetic */ WWWWoWWWWo(NetlinkManager netlinkManager, int i10) {
        this.f9110WWWWoWWWWo = i10;
        this.f9109WWWWWWWWWW = netlinkManager;
    }

    @Override // java.lang.Runnable
    public final void run() {
        switch (this.f9110WWWWoWWWWo) {
            case 0:
                NetlinkManager.m5140WWWoWWWo(this.f9109WWWWWWWWWW);
                return;
            case 1:
                NetlinkManager.m5138WWWWWWWW(this.f9109WWWWWWWWWW);
                return;
            case 2:
                NetlinkManager.m5137WWWWoWWWWo(this.f9109WWWWWWWWWW);
                return;
            default:
                NetlinkManager.m5139WWWWWWWW(this.f9109WWWWWWWWWW);
                return;
        }
    }
}
