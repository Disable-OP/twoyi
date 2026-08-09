package com.android.vmcore.installer;

import android.content.Context;
import android.net.Uri;
import android.os.Build;
import android.os.Process;
import android.os.SystemClock;
import android.text.TextUtils;
import android.util.Log;
import android.util.Pair;
import com.android.vmcore.ImageInstaller;
import com.android.vmcore.KLog;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.installer.ImageInstallerV1;
import com.android.vmcore.startup.WWWWoWWWWo;
import com.android.vmcore.utils.AssetsUtils;
import com.android.vmcore.utils.FileCopyUtils;
import com.android.vmcore.utils.FileDeleteUtils;
import com.android.vmcore.utils.ZipHelper;
import com.blankj.utilcode.util.C1644WWWoWWWo;
import com.blankj.utilcode.util.WoWo;
import com.google.android.gms.internal.ads.pr0;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import im.amomo.andun7z.AndUn7z;
import java.io.BufferedOutputStream;
import java.io.File;
import java.io.FileFilter;
import java.io.FileOutputStream;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.concurrent.Future;
import javax.crypto.Cipher;
import javax.crypto.CipherOutputStream;
import javax.crypto.spec.SecretKeySpec;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class ImageInstallerV1 implements ImageInstaller {

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final Context f9235WWWWWWWW;

    public ImageInstallerV1(Context context) {
        this.f9235WWWWWWWW = context;
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static void m5201WWWWoWWWWo(Uri uri, String str, String str2, String str3, FileFilter fileFilter, ArrayList arrayList) {
        boolean z10 = false;
        StringFog.f8859WWWWWWWW.getClass();
        if (WWWWWWWW.m17835WWWWWWWW(new byte[]{104, -47}, new byte[]{30, -29, Byte.MAX_VALUE, 40, 124, -53, 21, -55}).equals(uri.getQueryParameter(WWWWWWWW.m17835WWWWWWWW(new byte[]{-121}, new byte[]{-28, 18, -91, -104, -123, -30, TarConstants.LF_PAX_EXTENDED_HEADER_LC, TarConstants.LF_CHR})))) {
            if (Build.VERSION.SDK_INT >= 23) {
                z10 = Process.is64Bit();
            } else {
                try {
                    z10 = ((Boolean) WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_BLK, -56, 24, 9, -13, -19, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -123, 41, -38, 0, 26, -9, -88, 31, -69, 2, -36, 26, ConstantPoolEntry.CP_InterfaceMethodref, -13, -21, 44}, new byte[]{80, -87, 116, Byte.MAX_VALUE, -102, -122, 73, -10})).m5360WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{81, 99, TarConstants.LF_NORMAL, 93, 0, 6, -12, 100, 91, 99}, new byte[]{TarConstants.LF_FIFO, 6, 68, 15, 117, 104, Byte.MIN_VALUE, 13})).m5360WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -17, 122, 26, -44, 110, -49}, new byte[]{14, -100, TarConstants.LF_GNUTYPE_LONGNAME, 46, -106, 7, -69, 39})).f9408WWWWoWWWWo).booleanValue();
                } catch (Throwable unused) {
                }
            }
            if (z10) {
                AndUn7z.m14728WWWWWWWW(str, str2, str3, fileFilter, arrayList);
                return;
            } else {
                ZipHelper.m5272WWWWWWWW(str, str2, str3, fileFilter, true, arrayList);
                return;
            }
        }
        ZipHelper.m5272WWWWWWWW(str, str2, str3, fileFilter, false, arrayList);
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static BufferedOutputStream m5202WWWWWWWW(Uri uri, String str, ImageInstaller.InstallOptions installOptions) {
        byte[] bArr = {-105, 109, -31, -31, -124, ConstantPoolEntry.CP_InterfaceMethodref, 107, 5};
        StringFog.f8859WWWWWWWW.getClass();
        if (!WWWWWWWW.m17835WWWWWWWW(new byte[]{-7}, bArr).equals(uri.getQueryParameter(WWWWWWWW.m17835WWWWWWWW(new byte[]{-114}, new byte[]{-21, -99, -79, -51, -30, 57, -87, -79}))) && (installOptions == null || installOptions.f8844WWWoWWWo)) {
            if (WWWWWWWW.m17835WWWWWWWW(new byte[]{38}, new byte[]{94, -73, 114, -21, -105, -37, 93, 46}).equals(uri.getQueryParameter(WWWWWWWW.m17835WWWWWWWW(new byte[]{25}, new byte[]{124, 13, -38, 18, -14, -120, -63, -36})))) {
                return new BufferedOutputStream(new XOROutputStream(new FileOutputStream(str), WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, -13, 104, 69, 42, -114, -20, 23, 95, -71, 27, 23, 41, -67, -10, 109}, new byte[]{18, -119, 80, 124, TarConstants.LF_GNUTYPE_LONGLINK, -8, -123, 84})));
            }
            Cipher cipher = Cipher.getInstance(WWWWWWWW.m17835WWWWWWWW(new byte[]{-18, -119, -85}, new byte[]{-81, -52, -8, 123, -87, 45, 108, 31}));
            cipher.init(2, new SecretKeySpec(WWWWWWWW.m17835WWWWWWWW(new byte[]{-19, ConstantPoolEntry.CP_NameAndType, -90, -78, -66, TarConstants.LF_NORMAL, -104, 93, -123, 70, -43, -32, -67, 3, -126, 39}, new byte[]{-56, 118, -98, -117, -33, 70, -15, 30}).getBytes(), WWWWWWWW.m17835WWWWWWWW(new byte[]{104, TarConstants.LF_MULTIVOLUME, 29}, new byte[]{41, 8, 78, 94, -98, 116, -109, TarConstants.LF_SYMLINK})));
            return new BufferedOutputStream(new CipherOutputStream(new FileOutputStream(str), cipher));
        }
        return new BufferedOutputStream(new FileOutputStream(str));
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public static void m5203WWWWWWWW(String str, ArrayList arrayList) {
        StringBuilder sb2 = new StringBuilder(str);
        int size = arrayList.size();
        if (size > 5) {
            size = 5;
        }
        for (int i10 = 0; i10 < size; i10++) {
            sb2.append("\n\t");
            sb2.append((String) ((Pair) arrayList.get(i10)).second);
        }
        if (size < arrayList.size()) {
            pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, -13, TarConstants.LF_MULTIVOLUME, 72, 7, 23, -118, 59, 47}, new byte[]{1, -6, 0, 39, 117, 114, -92, 21}, sb2);
        }
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5041WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{-125, -111, ConstantPoolEntry.CP_NameAndType, -91, 62, -53, 28, -77, -72}, new byte[]{-54, -1, Byte.MAX_VALUE, -47, 95, -89, 112, -42}), sb2.toString());
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final void m5204WWWWWWWW(VMConfig vMConfig, Uri uri, String str, ImageInstaller.InstallOptions installOptions) {
        boolean m5260WWWWWWWW;
        String str2;
        FileFilter fileFilter;
        String[] strArr = new String[1];
        StringBuilder sb2 = new StringBuilder();
        sb2.append(uri.getLastPathSegment());
        int i10 = 0;
        pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{10}, new byte[]{85, -103, -26, -66, -15, -124, 107, -81}, sb2);
        sb2.append(vMConfig.f8866WWWWWWWW);
        String sb3 = sb2.toString();
        Context context = this.f9235WWWWWWWW;
        String absolutePath = new File(context.getCacheDir(), sb3).getAbsolutePath();
        int i11 = FileDeleteUtils.f9297WWWWWWWW;
        FileDeleteUtils.m5262WWWWWWWW(new File(absolutePath));
        try {
            long uptimeMillis = SystemClock.uptimeMillis();
            BufferedOutputStream m5202WWWWWWWW = m5202WWWWWWWW(uri, absolutePath, installOptions);
            if (uri.toString().startsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{108, 32, 2, 114, 21, 68, -38, 24}, new byte[]{13, TarConstants.LF_GNUTYPE_SPARSE, 113, 23, 97, 126, -11, TarConstants.LF_CONTIG}))) {
                m5260WWWWWWWW = AssetsUtils.m5239WWWWWWWW(context, uri.toString(), m5202WWWWWWWW, strArr);
            } else if (uri.toString().contains(WWWWWWWW.m17835WWWWWWWW(new byte[]{-119, 30}, new byte[]{-88, TarConstants.LF_LINK, 112, 106, 56, 85, -89, TarConstants.LF_CHR}))) {
                int indexOf = uri.getPath().indexOf(WWWWWWWW.m17835WWWWWWWW(new byte[]{-89, 41}, new byte[]{-122, 6, -111, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 71, -53, 74, -115}));
                m5260WWWWWWWW = FileCopyUtils.m5259WWWWoWWWWo(Uri.fromFile(new File(uri.getPath().substring(0, indexOf))).toString(), uri.getPath().substring(indexOf + 2), m5202WWWWWWWW, strArr);
            } else {
                m5260WWWWWWWW = FileCopyUtils.m5260WWWWWWWW(uri.toString(), m5202WWWWWWWW, strArr);
            }
            String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-6, -7, -123, -19, -75, -118, -61, -7, -63}, new byte[]{-77, -105, -10, -103, -44, -26, -81, -100});
            Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-83, -93, -120, -115, 112, -95, 64, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -85, -20}, new byte[]{-50, -52, -8, -12, 80, -43, 41, 21}) + (SystemClock.uptimeMillis() - uptimeMillis));
            if (m5260WWWWWWWW) {
                long uptimeMillis2 = SystemClock.uptimeMillis();
                if (installOptions == null || (str2 = installOptions.f8842WWWWoWWWWo) == null) {
                    str2 = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
                }
                String str3 = str2;
                if (installOptions != null) {
                    fileFilter = installOptions.f8843WWWWWWWW;
                } else {
                    fileFilter = null;
                }
                FileFilter fileFilter2 = fileFilter;
                ArrayList arrayList = new ArrayList();
                m5201WWWWoWWWWo(uri, absolutePath, str, str3, fileFilter2, arrayList);
                ArrayList arrayList2 = arrayList;
                if (!arrayList2.isEmpty()) {
                    m5203WWWWWWWW(arrayList2.size() + WWWWWWWW.m17835WWWWWWWW(new byte[]{74, 94, -15, 78, 105, -123, 60, 85, ConstantPoolEntry.CP_InterfaceMethodref, 81, -12, 71, 104, -42, 117, 93, 74, TarConstants.LF_GNUTYPE_LONGNAME, -16, 71, 44, -112, 117, 65, 25, TarConstants.LF_GNUTYPE_LONGNAME, -72, 82, 109, -123, 111, 9, 74}, new byte[]{106, 56, -104, 34, ConstantPoolEntry.CP_NameAndType, -10, 28, TarConstants.LF_CHR}), arrayList2);
                    HashSet hashSet = new HashSet();
                    int size = arrayList2.size();
                    while (i10 < size) {
                        Object obj = arrayList2.get(i10);
                        i10++;
                        Pair pair = (Pair) obj;
                        if (!TextUtils.isEmpty((CharSequence) pair.first)) {
                            hashSet.add((String) pair.first);
                        }
                    }
                    WWWWoWWWWo wWWWoWWWWo = new WWWWoWWWWo(hashSet, 1);
                    arrayList2.clear();
                    m5201WWWWoWWWWo(uri, absolutePath, str, str3, wWWWoWWWWo, arrayList2);
                    arrayList2 = arrayList2;
                }
                if (!arrayList2.isEmpty()) {
                    StringBuilder sb4 = new StringBuilder();
                    sb4.append(arrayList2.size());
                    StringFog.f8859WWWWWWWW.getClass();
                    sb4.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_LINK, -88, -49, 24, TarConstants.LF_GNUTYPE_LONGNAME, -66, TarConstants.LF_GNUTYPE_LONGNAME, 119, 112, -89, -54, 17, TarConstants.LF_MULTIVOLUME, -19, 5, Byte.MAX_VALUE, TarConstants.LF_LINK, -70, -50, 17, 9, -66, 9, 114, 126, -96, -62, 84, 89, -84, 31, 98, 43, -18}, new byte[]{17, -50, -90, 116, 41, -51, 108, 17}));
                    m5203WWWWWWWW(sb4.toString(), arrayList2);
                }
                StringFog.f8859WWWWWWWW.getClass();
                String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{14, -38, -17, 3, 69, -84, -26, -96, TarConstants.LF_DIR}, new byte[]{71, -76, -100, 119, 36, -64, -118, -59});
                Log.d(m17835WWWWWWWW2, WWWWWWWW.m17835WWWWWWWW(new byte[]{123, -116, TarConstants.LF_GNUTYPE_SPARSE, -21, 39, 80, 79, 33, 102, -117, TarConstants.LF_MULTIVOLUME, -6, 102}, new byte[]{18, -30, 32, -97, 70, 60, 35, 1}) + (SystemClock.uptimeMillis() - uptimeMillis2));
                FileDeleteUtils.m5262WWWWWWWW(new File(absolutePath));
                return;
            }
            throw new Exception(strArr[0]);
        } catch (Throwable th2) {
            FileDeleteUtils.m5262WWWWWWWW(new File(absolutePath));
            throw th2;
        }
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final void m5205WWWoWWWo(final VMConfig vMConfig, ArrayList arrayList, final String str, final ImageInstaller.InstallOptions installOptions) {
        long uptimeMillis = SystemClock.uptimeMillis();
        ArrayList arrayList2 = new ArrayList();
        int size = arrayList.size();
        int i10 = 0;
        int i11 = 0;
        while (i11 < size) {
            Object obj = arrayList.get(i11);
            i11++;
            final Uri uri = (Uri) obj;
            arrayList2.add(C1644WWWoWWWo.m5312WWWWoWWWWo(-4).submit(new Runnable() { // from class: u4.WWWW̏WWWWβ̏
                @Override // java.lang.Runnable
                public final void run() {
                    VMConfig vMConfig2 = vMConfig;
                    Uri uri2 = uri;
                    String str2 = str;
                    ImageInstaller.InstallOptions installOptions2 = installOptions;
                    ImageInstallerV1 imageInstallerV1 = ImageInstallerV1.this;
                    imageInstallerV1.getClass();
                    try {
                        imageInstallerV1.m5204WWWWWWWW(vMConfig2, uri2, str2, installOptions2);
                    } catch (Exception e10) {
                        throw new RuntimeException(e10);
                    }
                }
            }));
        }
        for (int i12 = 0; i12 < arrayList2.size(); i12++) {
            try {
                try {
                    ((Future) arrayList2.get(i12)).get();
                } catch (Exception e10) {
                    if (e10.getCause() != null && e10.getCause().getCause() != null) {
                        throw ((Exception) e10.getCause().getCause());
                    }
                    throw e10;
                }
            } finally {
                while (i10 < arrayList2.size()) {
                    Future future = (Future) arrayList2.get(i10);
                    if (!future.isDone()) {
                        try {
                            future.cancel(true);
                        } catch (Throwable unused) {
                        }
                    }
                    i10++;
                }
            }
        }
        long uptimeMillis2 = SystemClock.uptimeMillis();
        byte[] bArr = {72, 86, -56, 87, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -78, 35, 112};
        StringFog.f8859WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{1, 56, -69, 35, 25, -34, 79, 21, 58}, bArr);
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-120, 101, -89, -86, 111, 113, -119, -31, -113, 126, -78, -89, 111, 113, -108, -26, -111, 111, -13}, new byte[]{-4, 10, -45, -53, 3, 81, -32, -113}) + (uptimeMillis2 - uptimeMillis));
    }
}
