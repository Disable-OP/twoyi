package com.android.vmcore.event;

import i6.C2899WWWWWWWW;
/* loaded from: classes.dex */
public class PermissionEvent {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public IPermissionResultCallback f9006WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String[] f9007WWWWWWWW;

    /* loaded from: classes.dex */
    public interface IPermissionResultCallback {
        /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
        void mo5117WWWWoWWWWo(int[] iArr);
    }

    public PermissionEvent(String str, C2899WWWWWWWW c2899wwwwwwww) {
        this.f9007WWWWWWWW = r0;
        String[] strArr = {str};
        this.f9006WWWWoWWWWo = c2899wwwwwwww;
    }
}
