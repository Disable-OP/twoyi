package com.android.vmcore.utils;

import android.system.ErrnoException;
import android.system.Os;
import com.android.vmcore.StringFog;
import java.io.File;
import java.io.FileFilter;
import java.io.IOException;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public final class FileDeleteUtils {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static final /* synthetic */ int f9297WWWWWWWW = 0;

    /* renamed from: com.android.vmcore.utils.FileDeleteUtils$1  reason: invalid class name */
    /* loaded from: classes.dex */
    class AnonymousClass1 implements FileFilter {
        @Override // java.io.FileFilter
        public final boolean accept(File file) {
            return true;
        }
    }

    /* renamed from: com.android.vmcore.utils.FileDeleteUtils$2  reason: invalid class name */
    /* loaded from: classes.dex */
    class AnonymousClass2 implements FileFilter {
        @Override // java.io.FileFilter
        public final boolean accept(File file) {
            return file.isFile();
        }
    }

    static {
        StringFog.f8859WWWWWWWW.getClass();
        System.getProperty(WWWWWWWW.m17835WWWWWWWW(new byte[]{2, -50, 14, -105, 114, 112, -79, -101, 15, -43, 1, -122, TarConstants.LF_CHR, 113}, new byte[]{110, -89, 96, -14, 92, 3, -44, -21}));
    }

    /* JADX WARN: Type inference failed for: r0v0, types: [java.io.FileFilter, java.lang.Object] */
    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static boolean m5261WWWWoWWWWo(File file) {
        return m5263WWWWWWWW(file, new Object(), false);
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static boolean m5262WWWWWWWW(File file) {
        if (file.isDirectory()) {
            return m5264WWWoWWWo(file);
        }
        if (!file.isFile() && !file.isDirectory()) {
            try {
                Os.remove(file.getAbsolutePath());
            } catch (ErrnoException unused) {
            }
        }
        if (file.exists() && (!file.isFile() || !file.delete())) {
            return false;
        }
        return true;
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static boolean m5263WWWWWWWW(File file, FileFilter fileFilter, boolean z10) {
        String absolutePath;
        if (file.exists()) {
            if (file.isDirectory()) {
                File[] listFiles = file.listFiles();
                if (listFiles != null && listFiles.length != 0) {
                    for (File file2 : listFiles) {
                        if (fileFilter.accept(file2)) {
                            String absolutePath2 = file2.getAbsolutePath();
                            try {
                                absolutePath = file2.getCanonicalPath();
                            } catch (IOException unused) {
                                absolutePath = file2.getAbsolutePath();
                            }
                            if (!absolutePath2.equals(absolutePath)) {
                                try {
                                    Os.remove(file2.getAbsolutePath());
                                } catch (ErrnoException unused2) {
                                }
                            } else if (file2.isFile()) {
                                if (!file2.delete()) {
                                }
                            } else if (file2.isDirectory()) {
                                if (!m5264WWWoWWWo(file2)) {
                                }
                            } else {
                                Os.remove(file2.getAbsolutePath());
                            }
                        } else if (z10 && file2.isDirectory() && !m5263WWWWWWWW(file2, fileFilter, true)) {
                        }
                    }
                }
            }
            return false;
        }
        return true;
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static boolean m5264WWWoWWWo(File file) {
        String absolutePath;
        if (!file.exists()) {
            return true;
        }
        if (file.isDirectory()) {
            File[] listFiles = file.listFiles();
            if (listFiles != null && listFiles.length > 0) {
                for (File file2 : listFiles) {
                    String absolutePath2 = file2.getAbsolutePath();
                    try {
                        absolutePath = file2.getCanonicalPath();
                    } catch (IOException unused) {
                        absolutePath = file2.getAbsolutePath();
                    }
                    if (!absolutePath2.equals(absolutePath)) {
                        try {
                            Os.remove(file2.getAbsolutePath());
                        } catch (ErrnoException unused2) {
                        }
                    } else if (file2.isFile()) {
                        if (!file2.delete()) {
                        }
                    } else if (file2.isDirectory()) {
                        if (!m5264WWWoWWWo(file2)) {
                        }
                    } else {
                        Os.remove(file2.getAbsolutePath());
                    }
                }
            }
            return file.delete();
        }
        return false;
    }
}
