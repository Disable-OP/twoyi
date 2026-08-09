package com.android.vmcore.startup;

import java.io.File;
import java.io.FileFilter;
import java.util.HashSet;
/* renamed from: com.android.vmcore.startup.WWWWo̐WWWWoȄ̐  reason: invalid class name */
/* loaded from: classes.dex */
public final /* synthetic */ class WWWWoWWWWo implements FileFilter {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final /* synthetic */ HashSet f9277WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final /* synthetic */ int f9278WWWWWWWW;

    public /* synthetic */ WWWWoWWWWo(HashSet hashSet, int i10) {
        this.f9278WWWWWWWW = i10;
        this.f9277WWWWoWWWWo = hashSet;
    }

    @Override // java.io.FileFilter
    public final boolean accept(File file) {
        HashSet hashSet = this.f9277WWWWoWWWWo;
        switch (this.f9278WWWWWWWW) {
            case 0:
                String str = MagiskTask.f9271WWWoWWWo;
                return !hashSet.contains(file.getName());
            default:
                return hashSet.contains(file.getPath());
        }
    }
}
