package com.android.vmcore.utils;

import android.content.Context;
import android.net.Uri;
import android.text.TextUtils;
import com.android.vmcore.StringFog;
import com.blankj.utilcode.util.WWWW;
import java.io.BufferedOutputStream;
import java.io.IOException;
import java.io.InputStream;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class AssetsUtils {
    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static boolean m5238WWWWoWWWWo(BufferedOutputStream bufferedOutputStream, InputStream inputStream, String[] strArr) {
        try {
            try {
                byte[] bArr = new byte[8192];
                while (true) {
                    int read = inputStream.read(bArr, 0, 8192);
                    if (read != -1) {
                        bufferedOutputStream.write(bArr, 0, read);
                    } else {
                        WWWW.m5322WWWWWWWW(inputStream, bufferedOutputStream);
                        return true;
                    }
                }
            } catch (IOException e10) {
                e10.printStackTrace();
                if (!TextUtils.isEmpty(strArr[0])) {
                    StringBuilder sb2 = new StringBuilder();
                    sb2.append(strArr[0]);
                    StringFog.f8859WWWWWWWW.getClass();
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-2}, new byte[]{-126, 21, 66, -115, -78, 82, 3, 90}));
                    sb2.append(e10.getClass().getSimpleName());
                    sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-108}, new byte[]{-82, 8, -22, 107, 68, TarConstants.LF_GNUTYPE_LONGNAME, 87, 27}));
                    sb2.append(e10.getMessage());
                    strArr[0] = sb2.toString();
                } else {
                    StringBuilder sb3 = new StringBuilder();
                    sb3.append(e10.getClass().getSimpleName());
                    byte[] bArr2 = {TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER};
                    byte[] bArr3 = {93, -101, -118, 101, 58, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_NORMAL, -101};
                    StringFog.f8859WWWWWWWW.getClass();
                    sb3.append(WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                    sb3.append(e10.getMessage());
                    strArr[0] = sb3.toString();
                }
                WWWW.m5322WWWWWWWW(inputStream, bufferedOutputStream);
                return false;
            }
        } catch (Throwable th2) {
            WWWW.m5322WWWWWWWW(inputStream, bufferedOutputStream);
            throw th2;
        }
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static boolean m5239WWWWWWWW(Context context, String str, BufferedOutputStream bufferedOutputStream, String[] strArr) {
        try {
            Uri parse = Uri.parse(str);
            String authority = parse.getAuthority();
            if (!TextUtils.isEmpty(authority) && !context.getPackageName().equals(authority)) {
                context = context.createPackageContext(authority, 0);
            }
            return m5238WWWWoWWWWo(bufferedOutputStream, context.getAssets().open(parse.getPath().substring(1)), strArr);
        } catch (Throwable th2) {
            th2.printStackTrace();
            if (!TextUtils.isEmpty(strArr[0])) {
                StringBuilder sb2 = new StringBuilder();
                sb2.append(strArr[0]);
                byte[] bArr = {-105, -5, -33, -63, -74, 9, -34, TarConstants.LF_LINK};
                StringFog.f8859WWWWWWWW.getClass();
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-21}, bArr));
                sb2.append(th2.getClass().getSimpleName());
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{59}, new byte[]{1, 20, -80, -92, -61, 59, -44, 105}));
                sb2.append(th2.getMessage());
                strArr[0] = sb2.toString();
            } else {
                StringBuilder sb3 = new StringBuilder();
                sb3.append(th2.getClass().getSimpleName());
                StringFog.f8859WWWWWWWW.getClass();
                sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-99}, new byte[]{-89, 23, 78, -100, 0, 118, -26, 4}));
                sb3.append(th2.getMessage());
                strArr[0] = sb3.toString();
            }
            return false;
        }
    }
}
