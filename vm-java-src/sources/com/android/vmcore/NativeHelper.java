package com.android.vmcore;

import java.util.ArrayList;
/* loaded from: classes.dex */
public class NativeHelper {
    public static native void buildVMProcessReport(int i10, String str);

    public static native int chmodRecursively(String str, int i10);

    public static native int clearZombieProcess(int i10);

    public static native int deleteRecursively(String str);

    public static native ArrayList<VMProcessInfo> getProcessList(int i10);

    public static native void printXAttr(String str);
}
