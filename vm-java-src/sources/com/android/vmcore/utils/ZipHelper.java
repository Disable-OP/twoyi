package com.android.vmcore.utils;

import android.system.ErrnoException;
import android.system.Os;
import android.util.Log;
import android.util.Pair;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmcore.StringFog;
import com.google.android.gms.internal.ads.pr0;
import j$.io.FileRetargetClass;
import j$.nio.file.Files;
import j$.nio.file.StandardOpenOption;
import j$.nio.file.attribute.FileAttribute;
import java.io.BufferedInputStream;
import java.io.BufferedOutputStream;
import java.io.Closeable;
import java.io.File;
import java.io.FileFilter;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.channels.SeekableByteChannel;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.Enumeration;
import org.apache.commons.compress.archivers.sevenz.SevenZArchiveEntry;
import org.apache.commons.compress.archivers.sevenz.SevenZFile;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.archivers.zip.UnixStat;
import org.apache.commons.compress.archivers.zip.X7875_NewUnix;
import org.apache.commons.compress.archivers.zip.ZipArchiveEntry;
import org.apache.commons.compress.archivers.zip.ZipExtraField;
import org.apache.commons.compress.archivers.zip.ZipFile;
import org.apache.commons.compress.archivers.zip.ZipShort;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.apache.commons.compress.utils.IOUtils;
import p000WWWWWWWWWW.WWoWWo;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class ZipHelper {
    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static File m5269WWWWoWWWWo(String str) {
        if (str != null) {
            int length = str.length();
            for (int i10 = 0; i10 < length; i10++) {
                if (!Character.isWhitespace(str.charAt(i10))) {
                    return new File(str);
                }
            }
            return null;
        }
        return null;
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static boolean m5270WWWWWWWW(File file) {
        if (file != null) {
            if (file.exists()) {
                if (file.isDirectory()) {
                    return true;
                }
                return false;
            } else if (file.mkdirs()) {
                return true;
            } else {
                return false;
            }
        }
        return false;
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static void m5271WWWWWWWW(File file, ArrayList arrayList, Closeable closeable, Object obj, String str, String str2) {
        boolean isDirectory;
        boolean createNewFile;
        InputStream inputStream;
        String unixSymlink;
        File file2;
        ZipExtraField extraField;
        ZipExtraField extraField2;
        long j10;
        long unixMode;
        int length;
        ZipExtraField extraField3;
        File file3 = new File(file, str);
        boolean z10 = obj instanceof SevenZArchiveEntry;
        if (z10) {
            isDirectory = ((SevenZArchiveEntry) obj).isDirectory();
        } else {
            isDirectory = ((ZipArchiveEntry) obj).isDirectory();
        }
        byte[] bArr = null;
        if (isDirectory) {
            if (!m5270WWWWWWWW(file3)) {
                StringBuilder sb2 = new StringBuilder();
                StringFog.f8859WWWWWWWW.getClass();
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{61, 122, 63, -91, 124, -35, -10, 23, TarConstants.LF_CONTIG, 122, 122}, new byte[]{94, 8, 90, -60, 8, -72, -42, 115}));
                sb2.append(file3.getAbsolutePath());
                String m9000WWWWWWWW = pr0.m9000WWWWWWWW(new byte[]{68, -91, -88, -101, -113, 24, 78}, new byte[]{100, -61, -55, -14, -29, 125, 42, -45}, sb2);
                arrayList.add(new Pair(str, m9000WWWWWWWW));
                throw new IOException(m9000WWWWWWWW);
            }
        } else if (m5273WWWoWWWo(obj)) {
            if (m5270WWWWWWWW(file3.getParentFile())) {
                if (closeable instanceof SevenZFile) {
                    SevenZFile sevenZFile = (SevenZFile) closeable;
                    if (m5273WWWoWWWo(obj)) {
                        InputStream inputStream2 = sevenZFile.getInputStream((SevenZArchiveEntry) obj);
                        try {
                            unixSymlink = new String(IOUtils.toByteArray(inputStream2));
                            if (inputStream2 != null) {
                                inputStream2.close();
                            }
                        } catch (Throwable th2) {
                            if (inputStream2 != null) {
                                try {
                                    inputStream2.close();
                                } catch (Throwable th3) {
                                    th2.addSuppressed(th3);
                                }
                            }
                            throw th2;
                        }
                    } else {
                        unixSymlink = null;
                    }
                } else {
                    unixSymlink = ((ZipFile) closeable).getUnixSymlink((ZipArchiveEntry) obj);
                }
                if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-84}, new byte[]{-125, 16, -20, -29, TarConstants.LF_CHR, TarConstants.LF_GNUTYPE_LONGLINK, TarConstants.LF_GNUTYPE_LONGLINK, -91}, unixSymlink)) {
                    file2 = new File(file, WWoWWo.m60WWoWWo(str2, unixSymlink));
                } else {
                    file2 = new File(file3.getParentFile(), unixSymlink);
                }
                try {
                    Os.remove(file3.getAbsolutePath());
                } catch (Throwable unused) {
                }
                try {
                    Os.symlink(file2.getAbsolutePath(), file3.getAbsolutePath());
                } catch (ErrnoException e10) {
                    StringBuilder sb3 = new StringBuilder();
                    byte[] bArr2 = {112, 65, 109, 117, 66, TarConstants.LF_GNUTYPE_LONGLINK, -42, 15, 106, 94, 100, 125, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 69, -42};
                    byte[] bArr3 = {19, TarConstants.LF_CHR, 8, 20, TarConstants.LF_FIFO, 46, -10, 124};
                    StringFog.f8859WWWWWWWW.getClass();
                    sb3.append(WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                    sb3.append(file3.getAbsolutePath());
                    sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-107, -42, -3, TarConstants.LF_GNUTYPE_SPARSE, -84, TarConstants.LF_CHR, 89, -40, -107}, new byte[]{-75, -80, -100, 58, -64, 86, 61, -30}));
                    sb3.append(e10.errno);
                    arrayList.add(new Pair(str, sb3.toString()));
                }
            } else {
                StringBuilder sb4 = new StringBuilder();
                byte[] bArr4 = {80, 43, -8, -83, -88, 60, TarConstants.LF_NORMAL, -4, 82, 43, -8, -94, -88, 121};
                byte[] bArr5 = {TarConstants.LF_CHR, 89, -99, -52, -36, 89, 16, -116};
                StringFog.f8859WWWWWWWW.getClass();
                sb4.append(WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5));
                sb4.append(file3.getAbsolutePath());
                String m9000WWWWWWWW2 = pr0.m9000WWWWWWWW(new byte[]{119, -70, 64, -50, 9, -58, -80}, new byte[]{87, -36, 33, -89, 101, -93, -44, 68}, sb4);
                arrayList.add(new Pair(str, m9000WWWWWWWW2));
                throw new IOException(m9000WWWWWWWW2);
            }
        } else {
            if (file3.exists()) {
                createNewFile = file3.isFile();
            } else {
                if (m5270WWWWWWWW(file3.getParentFile())) {
                    try {
                        createNewFile = file3.createNewFile();
                    } catch (Throwable unused2) {
                    }
                }
                createNewFile = false;
            }
            if (createNewFile) {
                if (closeable instanceof SevenZFile) {
                    inputStream = ((SevenZFile) closeable).getInputStream((SevenZArchiveEntry) obj);
                } else {
                    inputStream = ((ZipFile) closeable).getInputStream((ZipArchiveEntry) obj);
                }
                BufferedInputStream bufferedInputStream = new BufferedInputStream(inputStream);
                try {
                    BufferedOutputStream bufferedOutputStream = new BufferedOutputStream(new FileOutputStream(file3));
                    IOUtils.copy(bufferedInputStream, bufferedOutputStream, 8192);
                    bufferedOutputStream.close();
                    bufferedInputStream.close();
                } catch (Throwable th4) {
                    try {
                        bufferedInputStream.close();
                    } catch (Throwable th5) {
                        th4.addSuppressed(th5);
                    }
                    throw th4;
                }
            } else {
                StringBuilder sb5 = new StringBuilder();
                StringFog.f8859WWWWWWWW.getClass();
                sb5.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-51, -32, 73, -29, -38, 101, 63, -32, -57, -2, 73, -94}, new byte[]{-82, -110, 44, -126, -82, 0, 31, -122}));
                sb5.append(file3.getAbsolutePath());
                String m9000WWWWWWWW3 = pr0.m9000WWWWWWWW(new byte[]{-36, -22, 87, -36, 20, 68, -118}, new byte[]{-4, -116, TarConstants.LF_FIFO, -75, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 33, -18, -122}, sb5);
                arrayList.add(new Pair(str, m9000WWWWWWWW3));
                throw new IOException(m9000WWWWWWWW3);
            }
        }
        long uid = (z10 || (extraField = ((ZipArchiveEntry) obj).getExtraField(new ZipShort(30837))) == null) ? -1L : ((X7875_NewUnix) extraField).getUID();
        long gid = (z10 || (extraField2 = ((ZipArchiveEntry) obj).getExtraField(new ZipShort(30837))) == null) ? -1L : ((X7875_NewUnix) extraField2).getGID();
        if (z10) {
            unixMode = 0;
            j10 = -1;
        } else {
            j10 = -1;
            unixMode = ((ZipArchiveEntry) obj).getUnixMode();
        }
        if (!z10 && (extraField3 = ((ZipArchiveEntry) obj).getExtraField(new ZipShort(24949))) != null) {
            bArr = extraField3.getLocalFileDataData();
        }
        String absolutePath = file3.getAbsolutePath();
        if (uid != j10) {
            int i10 = (int) uid;
            ByteBuffer allocate = ByteBuffer.allocate(4);
            allocate.order(ByteOrder.LITTLE_ENDIAN);
            allocate.putInt(i10);
            byte[] array = allocate.array();
            byte[] bArr6 = {-36, TarConstants.LF_CHR, -100, 56, 29, 4, 47, 63};
            byte[] bArr7 = {-87, 64, -7, 74, TarConstants.LF_CHR, 113, 70, 91};
            StringFog.f8859WWWWWWWW.getClass();
            OsExt.m5267WWWWWWWW(absolutePath, WWWWWWWW.m17835WWWWWWWW(bArr6, bArr7), array);
        }
        if (gid != j10) {
            ByteBuffer allocate2 = ByteBuffer.allocate(4);
            allocate2.order(ByteOrder.LITTLE_ENDIAN);
            allocate2.putInt((int) gid);
            byte[] array2 = allocate2.array();
            StringFog.f8859WWWWWWWW.getClass();
            OsExt.m5267WWWWWWWW(absolutePath, WWWWWWWW.m17835WWWWWWWW(new byte[]{112, 93, -51, -108, -69, 56, 123, 123}, new byte[]{5, 46, -88, -26, -107, 95, 18, 31}), array2);
        }
        if (unixMode != 0) {
            int i11 = (int) unixMode;
            ByteBuffer allocate3 = ByteBuffer.allocate(4);
            allocate3.order(ByteOrder.LITTLE_ENDIAN);
            allocate3.putInt(i11);
            byte[] array3 = allocate3.array();
            StringFog.f8859WWWWWWWW.getClass();
            OsExt.m5267WWWWWWWW(absolutePath, WWWWWWWW.m17835WWWWWWWW(new byte[]{-41, -11, 8, 72, -72, -35, 123, -118, -57}, new byte[]{-94, -122, 109, 58, -106, -80, 20, -18}), array3);
            OsExt.m5266WWWWWWWW(i11, absolutePath);
        } else {
            OsExt.m5266WWWWWWWW(UnixStat.DEFAULT_LINK_PERM, absolutePath);
        }
        int i12 = 0;
        while (true) {
            if (bArr != null) {
                try {
                    length = bArr.length;
                } catch (Exception e11) {
                    byte[] bArr8 = {-120, 30, 64, Byte.MIN_VALUE, TarConstants.LF_NORMAL, 17, -79, -57};
                    StringFog.f8859WWWWWWWW.getClass();
                    String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-50, 119, 44, -27, 113, 101, -59, -75, -64, 123, 44, -16, 85, 99}, bArr8);
                    StringBuilder sb6 = new StringBuilder();
                    pr0.m9009WWWoWWWo(new byte[]{-127, 81, -45, 125, 91, 23, 0, -119, -110, 1, -44, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 86, 62, 84, -104, -104, 85, -47, 112, 2, 34, 27, -35}, new byte[]{-32, 33, -93, 17, 34, 86, 116, -3}, sb6, absolutePath);
                    sb6.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-48, -78, -77, TarConstants.LF_GNUTYPE_LONGLINK, -64, 95, 102, TarConstants.LF_CONTIG}, new byte[]{-16, -44, -46, 34, -84, 58, 2, 25}));
                    Log.e(m17835WWWWWWWW, sb6.toString(), e11);
                    return;
                }
            } else {
                length = 0;
            }
            if (i12 >= length) {
                return;
            }
            String str3 = new String(bArr, i12 + 1, bArr[i12]);
            int i13 = bArr[i12] + 1 + i12;
            int i14 = bArr[i13];
            byte[] bArr9 = new byte[i14];
            System.arraycopy(bArr, i13 + 1, bArr9, 0, i14);
            OsExt.m5267WWWWWWWW(absolutePath, str3, bArr9);
            i12 = bArr[i13] + 1 + i13;
        }
    }

    /* JADX WARN: Removed duplicated region for block: B:27:0x006c A[Catch: all -> 0x00fc, TryCatch #0 {all -> 0x00fc, blocks: (B:13:0x003d, B:17:0x004a, B:19:0x004e, B:27:0x006c, B:29:0x0070, B:31:0x007f, B:33:0x00b7, B:37:0x0101, B:39:0x010c, B:30:0x0078, B:21:0x0057, B:23:0x005c, B:25:0x0065, B:16:0x0043), top: B:49:0x003d }] */
    /* JADX WARN: Removed duplicated region for block: B:53:0x0115 A[SYNTHETIC] */
    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public static void m5272WWWWWWWW(String str, String str2, String str3, FileFilter fileFilter, boolean z10, ArrayList arrayList) {
        SeekableByteChannel newByteChannel;
        Closeable zipFile;
        Enumeration<ZipArchiveEntry> enumeration;
        Object obj;
        Object nextElement;
        String name;
        File m5269WWWWoWWWWo = m5269WWWWoWWWWo(str);
        File m5269WWWWoWWWWo2 = m5269WWWWoWWWWo(str2);
        if (m5269WWWWoWWWWo != null && m5269WWWWoWWWWo2 != null && (newByteChannel = Files.newByteChannel(FileRetargetClass.toPath(m5269WWWWoWWWWo), EnumSet.of(StandardOpenOption.READ), new FileAttribute[0])) != null) {
            if (z10) {
                zipFile = new SevenZFile(newByteChannel);
            } else {
                zipFile = new ZipFile(newByteChannel);
            }
            Closeable closeable = zipFile;
            try {
                if (closeable instanceof SevenZFile) {
                    enumeration = closeable;
                } else {
                    enumeration = ((ZipFile) closeable).getEntries();
                }
                while (true) {
                    if (enumeration instanceof SevenZFile) {
                        nextElement = ((SevenZFile) enumeration).getNextEntry();
                    } else {
                        obj = null;
                        if (enumeration instanceof Enumeration) {
                            Enumeration enumeration2 = (Enumeration) enumeration;
                            if (enumeration2.hasMoreElements()) {
                                nextElement = enumeration2.nextElement();
                            }
                        }
                        if (obj == null) {
                            if (obj instanceof SevenZArchiveEntry) {
                                name = ((SevenZArchiveEntry) obj).getName();
                            } else {
                                name = ((ZipArchiveEntry) obj).getName();
                            }
                            byte[] bArr = {TarConstants.LF_DIR};
                            byte[] bArr2 = {105, -19, 41, 57, ConstantPoolEntry.CP_InterfaceMethodref, -98, -42, -50};
                            StringFog.f8859WWWWWWWW.getClass();
                            String replace = name.replace(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), WWWWWWWW.m17835WWWWWWWW(new byte[]{-53}, new byte[]{-28, TarConstants.LF_BLK, 100, -37, -18, -40, -46, 108}));
                            if (replace.contains(WWWWWWWW.m17835WWWWWWWW(new byte[]{66, 70, -33}, new byte[]{108, 104, -16, -120, -119, -54, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 100}))) {
                                String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{72, 104, 109, -56, 35, 94, -74, -40, 96}, new byte[]{18, 1, 29, Byte.MIN_VALUE, 70, TarConstants.LF_SYMLINK, -58, -67});
                                Log.e(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_LINK, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 65, 47, -63, 81, 85, -114, TarConstants.LF_LINK, ConstantPoolEntry.CP_NameAndType, 21}, new byte[]{84, TarConstants.LF_FIFO, TarConstants.LF_DIR, 93, -72, 31, TarConstants.LF_BLK, -29}) + replace + WWWWWWWW.m17835WWWWWWWW(new byte[]{-57, -72, 46, 21, 69, 72, -91, -41, -126, -93, TarConstants.LF_SYMLINK, 64, 82, 8}, new byte[]{-25, -47, 93, TarConstants.LF_DIR, 33, 41, -53, -80}));
                            } else if (fileFilter == null || fileFilter.accept(new File(replace))) {
                                m5271WWWWWWWW(m5269WWWWoWWWWo2, arrayList, closeable, obj, replace, str3);
                            }
                        } else {
                            closeable.close();
                            return;
                        }
                    }
                    obj = nextElement;
                    if (obj == null) {
                    }
                }
            } catch (Throwable th2) {
                try {
                    closeable.close();
                } catch (Throwable th3) {
                    th2.addSuppressed(th3);
                }
                throw th2;
            }
        }
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static boolean m5273WWWoWWWo(Object obj) {
        if (obj instanceof SevenZArchiveEntry) {
            int windowsAttributes = ((SevenZArchiveEntry) obj).getWindowsAttributes();
            if ((32768 & windowsAttributes) != 0 && ((windowsAttributes >> 16) & 61440) == 40960) {
                return true;
            }
            return false;
        }
        return ((ZipArchiveEntry) obj).isUnixSymlink();
    }
}
