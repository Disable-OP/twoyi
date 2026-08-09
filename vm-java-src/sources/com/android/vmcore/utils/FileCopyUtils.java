package com.android.vmcore.utils;

import android.net.Uri;
import android.text.TextUtils;
import com.android.vmcore.StringFog;
import com.blankj.utilcode.util.WWWW;
import java.io.BufferedOutputStream;
import java.io.FileInputStream;
import java.io.InputStream;
import java.util.zip.ZipFile;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class FileCopyUtils {
    /* JADX WARN: Multi-variable type inference failed */
    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static boolean m5259WWWWoWWWWo(String str, String str2, BufferedOutputStream bufferedOutputStream, String[] strArr) {
        InputStream inputStream;
        InputStream inputStream2 = null;
        try {
            ZipFile zipFile = new ZipFile(Uri.parse(str).getPath());
            try {
                inputStream2 = zipFile.getInputStream(zipFile.getEntry(str2));
                byte[] bArr = new byte[8192];
                while (true) {
                    int read = inputStream2.read(bArr, 0, 8192);
                    if (read != -1) {
                        bufferedOutputStream.write(bArr, 0, read);
                    } else {
                        WWWW.m5322WWWWWWWW(inputStream2, bufferedOutputStream, zipFile);
                        return true;
                    }
                }
            } catch (Throwable th2) {
                th = th2;
                inputStream = inputStream2;
                inputStream2 = zipFile;
                try {
                    th.printStackTrace();
                    if (!TextUtils.isEmpty(strArr[0])) {
                        StringBuilder sb2 = new StringBuilder();
                        sb2.append(strArr[0]);
                        byte[] bArr2 = {-17, TarConstants.LF_GNUTYPE_SPARSE, 106, 31, -106, 100, 62, 73};
                        StringFog.f8859WWWWWWWW.getClass();
                        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-109}, bArr2));
                        sb2.append(th.getClass().getSimpleName());
                        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{119}, new byte[]{TarConstants.LF_MULTIVOLUME, 14, 95, -24, -62, -110, TarConstants.LF_GNUTYPE_LONGNAME, -108}));
                        sb2.append(th.getMessage());
                        strArr[0] = sb2.toString();
                    } else {
                        StringBuilder sb3 = new StringBuilder();
                        sb3.append(th.getClass().getSimpleName());
                        StringFog.f8859WWWWWWWW.getClass();
                        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-49}, new byte[]{-11, -81, 93, -36, -50, -55, 46, 23}));
                        sb3.append(th.getMessage());
                        strArr[0] = sb3.toString();
                    }
                    WWWW.m5322WWWWWWWW(inputStream, bufferedOutputStream, inputStream2);
                    return false;
                } catch (Throwable th3) {
                    WWWW.m5322WWWWWWWW(inputStream, bufferedOutputStream, inputStream2);
                    throw th3;
                }
            }
        } catch (Throwable th4) {
            th = th4;
            inputStream = null;
        }
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static boolean m5260WWWWWWWW(String str, BufferedOutputStream bufferedOutputStream, String[] strArr) {
        FileInputStream fileInputStream;
        FileInputStream fileInputStream2 = null;
        try {
            fileInputStream = new FileInputStream(Uri.parse(str).getPath());
        } catch (Throwable th2) {
            th = th2;
        }
        try {
            byte[] bArr = new byte[8192];
            while (true) {
                int read = fileInputStream.read(bArr, 0, 8192);
                if (read != -1) {
                    bufferedOutputStream.write(bArr, 0, read);
                } else {
                    WWWW.m5322WWWWWWWW(fileInputStream, bufferedOutputStream);
                    return true;
                }
            }
        } catch (Throwable th3) {
            th = th3;
            fileInputStream2 = fileInputStream;
            try {
                th.printStackTrace();
                if (!TextUtils.isEmpty(strArr[0])) {
                    StringBuilder sb2 = new StringBuilder();
                    sb2.append(strArr[0]);
                    byte[] bArr2 = {-31, -20, 47, 10, -6, TarConstants.LF_BLK, 41, -9};
                    StringFog.f8859WWWWWWWW.getClass();
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-99}, bArr2));
                    sb2.append(th.getClass().getSimpleName());
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-94}, new byte[]{-104, 56, 74, -127, -91, -30, 109, -79}));
                    sb2.append(th.getMessage());
                    strArr[0] = sb2.toString();
                } else {
                    StringBuilder sb3 = new StringBuilder();
                    sb3.append(th.getClass().getSimpleName());
                    byte[] bArr3 = {124, 43, Byte.MIN_VALUE, -76, TarConstants.LF_NORMAL, -71, -94, -51};
                    StringFog.f8859WWWWWWWW.getClass();
                    sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{70}, bArr3));
                    sb3.append(th.getMessage());
                    strArr[0] = sb3.toString();
                }
                WWWW.m5322WWWWWWWW(fileInputStream2, bufferedOutputStream);
                return false;
            } catch (Throwable th4) {
                WWWW.m5322WWWWWWWW(fileInputStream2, bufferedOutputStream);
                throw th4;
            }
        }
    }
}
