package com.android.vmcore.utils;

import j3.C3164WWWWWWWW;
import java.io.File;
import java.io.FileFilter;
/* renamed from: com.android.vmcore.utils.WWWW̏WWWWβ̏  reason: invalid class name */
/* loaded from: classes.dex */
public final /* synthetic */ class WWWWWWWW implements FileFilter {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final /* synthetic */ String f9298WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final /* synthetic */ int f9299WWWWWWWW;

    public /* synthetic */ WWWWWWWW(String str, int i10) {
        this.f9299WWWWWWWW = i10;
        this.f9298WWWWoWWWWo = str;
    }

    @Override // java.io.FileFilter
    public final boolean accept(File file) {
        String str = this.f9298WWWWoWWWWo;
        switch (this.f9299WWWWWWWW) {
            case 0:
                return file.getName().contains(str);
            case 1:
                return file.getName().contains(str);
            case 2:
                return file.getName().contains(str);
            case 3:
                return file.getName().contains(str);
            case 4:
                return file.getName().contains(str);
            case 5:
                return file.getName().contains(str);
            case 6:
                return file.getName().contains(str);
            case 7:
                return file.getName().contains(str);
            default:
                String name = file.getName();
                C3164WWWWWWWW.f28918WWWWWWWW.getClass();
                if (name.endsWith(x5.WWWWWWWW.m17835WWWWWWWW(new byte[]{-25, -48, 14, -26}, new byte[]{-55, -79, 126, -115, 99, 124, 81, -47})) && !file.getAbsolutePath().startsWith(str)) {
                    return true;
                }
                return false;
        }
    }
}
