package com.android.vmcore.bridge;

import android.net.LocalSocket;
import android.net.LocalSocketAddress;
import android.text.TextUtils;
import com.android.vmcore.KLog;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import java.io.Closeable;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.EOFException;
import java.io.File;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class VMEventManager {

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public static final String f8985WWoWWo;

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public DataOutputStream f8986WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public DataInputStream f8987WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final VMInstance f8988WWWWWWWW;

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public final ArrayList f8989WWWWWWWW = new ArrayList();

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public volatile boolean f8990WWWoWWWo;

    static {
        byte[] bArr = {-72, 122, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 15, -102, -110, ConstantPoolEntry.CP_InterfaceMethodref, 87, -113, 89, 67, 30, -102, -114};
        byte[] bArr2 = {-18, TarConstants.LF_CONTIG, 34, 121, -1, -4, Byte.MAX_VALUE, 26};
        StringFog.f8859WWWWWWWW.getClass();
        f8985WWoWWo = WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
    }

    public VMEventManager(VMInstance vMInstance) {
        this.f8988WWWWWWWW = vMInstance;
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static void m5113WWWWWWWW(Object obj) {
        try {
            if (obj instanceof LocalSocket) {
                ((LocalSocket) obj).close();
            } else if (obj instanceof Closeable) {
                ((Closeable) obj).close();
            }
        } catch (Throwable unused) {
        }
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final void m5114WWWWoWWWWo() {
        String str;
        int i10 = 0;
        DataInputStream dataInputStream = this.f8987WWWWWWWW;
        String str2 = f8985WWoWWo;
        if (dataInputStream == null) {
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5040WWWWoWWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-1, -54, -76, TarConstants.LF_NORMAL, -108, -117, -96, -100, -54, -52, -116, 113, -98, -95, -10, -102, -53, -42, -65, TarConstants.LF_BLK, -109, -70, -65, -106, -54}, new byte[]{-92, -72, -47, 81, -16, -50, -42, -7}));
            return;
        }
        byte[] bArr = {22, TarConstants.LF_CHR, -44, -25, 16, 108, 101, 45, 35, TarConstants.LF_DIR, -20, -90, 7, 93, 114, 58, 57, 97, -61, -29, 21, TarConstants.LF_MULTIVOLUME};
        byte[] bArr2 = {TarConstants.LF_MULTIVOLUME, 65, -79, -122, 116, 41, 19, 72};
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(str2, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        String readUTF = this.f8987WWWWWWWW.readUTF();
        KLog.m5043WWWWWWWW(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{85, -4, 67, 126, 60, 121, -17, -32, 96, -6, 123, 63, 42, 89, -8, -31, 46}, new byte[]{14, -114, 38, 31, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 60, -103, -123}) + readUTF);
        if (!TextUtils.isEmpty(readUTF)) {
            String[] split = readUTF.split(WWWWWWWW.m17835WWWWWWWW(new byte[]{-34}, new byte[]{-66, 21, 16, -14, 46, TarConstants.LF_GNUTYPE_SPARSE, -17, -59}));
            String str3 = split[0];
            if (split.length > 1) {
                str = split[1];
            } else {
                str = null;
            }
            try {
                ArrayList arrayList = this.f8989WWWWWWWW;
                int size = arrayList.size();
                while (i10 < size) {
                    Object obj = arrayList.get(i10);
                    i10++;
                    IVMEventCallback iVMEventCallback = (IVMEventCallback) obj;
                    if (iVMEventCallback != null) {
                        iVMEventCallback.mo5013WWWWWWWW(str3, str);
                    }
                }
            } catch (Throwable th2) {
                byte[] bArr3 = {-9, 74, -70, -64, TarConstants.LF_MULTIVOLUME, -89, 104, 64};
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5044WWWoWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-84, 46, -43, -125, 44, -53, 4, 34, -106, 41, -47, -99, 109, -62, 16, 122, -41}, bArr3), th2);
            }
        }
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final void m5115WWWWWWWW() {
        this.f8990WWWoWWWo = false;
        new Thread() { // from class: com.android.vmcore.bridge.VMEventManager.1
            @Override // java.lang.Thread, java.lang.Runnable
            public final void run() {
                LocalSocket localSocket;
                LocalServerSocket localServerSocket;
                String str;
                String m17835WWWWWWWW;
                String str2;
                WWWWWWWW wwwwwwww;
                InputStream inputStream;
                OutputStream outputStream;
                while (!VMEventManager.this.f8990WWWoWWWo) {
                    VMEventManager vMEventManager = VMEventManager.this;
                    if (vMEventManager.f8988WWWWWWWW.f8940WWoWWo != -5) {
                        try {
                            str2 = VMEventManager.f8985WWoWWo;
                            byte[] bArr = {56, -4, 35, 91, 97, 64, -52, -50, 13, -31, 35, TarConstants.LF_GNUTYPE_LONGNAME, 96, 89, -32, -49, 62, -81, TarConstants.LF_DIR, 91, 117, 66, -5};
                            byte[] bArr2 = {99, -113, 70, 47, 20, TarConstants.LF_NORMAL, -113, -95};
                            wwwwwwww = StringFog.f8859WWWWWWWW;
                            wwwwwwww.getClass();
                            KLog.m5043WWWWWWWW(str2, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                            String str3 = vMEventManager.f8988WWWWWWWW.f8937WWWoWWWo.f8867WWWWWWWW;
                            wwwwwwww.getClass();
                            String absolutePath = new File(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-59, 26, -109, 89, -67, ConstantPoolEntry.CP_InterfaceMethodref, 5, -116, -124, 10}, new byte[]{-22, 126, -10, 47, -110, 110, 115, -23})).getAbsolutePath();
                            LocalSocketAddress.Namespace namespace = LocalSocketAddress.Namespace.FILESYSTEM;
                            localServerSocket = new LocalServerSocket(absolutePath);
                            try {
                                localSocket = localServerSocket.m5112WWWWoWWWWo();
                                try {
                                    try {
                                        inputStream = localSocket.getInputStream();
                                        outputStream = localSocket.getOutputStream();
                                    } catch (Exception e10) {
                                        e = e10;
                                        if (!(e instanceof EOFException)) {
                                            String str4 = VMEventManager.f8985WWoWWo;
                                            StringFog.f8859WWWWWWWW.getClass();
                                            KLog.m5044WWWoWWWo(str4, WWWWWWWW.m17835WWWWWWWW(new byte[]{-76, 121, 68, -88, ConstantPoolEntry.CP_NameAndType, 86, -114, 117, -127, 100, 68, -65, 13, 79, -94, 116, -78, 42, 68, -92, 67, 6}, new byte[]{-17, 10, 33, -36, 121, 38, -51, 26}), e);
                                        }
                                        str = VMEventManager.f8985WWoWWo;
                                        byte[] bArr3 = {27, 84, 8, 115, 33, 69, 24, 113, 46, 73, 8, 100, 32, 92, TarConstants.LF_BLK, 112, 29, 7, 8, 105, TarConstants.LF_NORMAL};
                                        byte[] bArr4 = {64, 39, 109, 7, 84, TarConstants.LF_DIR, 91, 30};
                                        StringFog.f8859WWWWWWWW.getClass();
                                        m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4);
                                        KLog.m5043WWWWWWWW(str, m17835WWWWWWWW);
                                        VMEventManager.m5113WWWWWWWW(localServerSocket);
                                        VMEventManager.m5113WWWWWWWW(localSocket);
                                        VMEventManager.m5113WWWWWWWW(vMEventManager.f8987WWWWWWWW);
                                        vMEventManager.f8987WWWWWWWW = null;
                                        VMEventManager.m5113WWWWWWWW(vMEventManager.f8986WWWWoWWWWo);
                                        vMEventManager.f8986WWWWoWWWWo = null;
                                    }
                                } catch (Throwable th2) {
                                    th = th2;
                                    String str5 = VMEventManager.f8985WWoWWo;
                                    StringFog.f8859WWWWWWWW.getClass();
                                    KLog.m5043WWWWWWWW(str5, WWWWWWWW.m17835WWWWWWWW(new byte[]{115, -28, -101, 47, -112, -61, 40, -116, 70, -7, -101, 56, -111, -38, 4, -115, 117, -73, -101, TarConstants.LF_DIR, -127}, new byte[]{40, -105, -2, 91, -27, -77, 107, -29}));
                                    VMEventManager.m5113WWWWWWWW(localServerSocket);
                                    VMEventManager.m5113WWWWWWWW(localSocket);
                                    VMEventManager.m5113WWWWWWWW(vMEventManager.f8987WWWWWWWW);
                                    vMEventManager.f8987WWWWWWWW = null;
                                    VMEventManager.m5113WWWWWWWW(vMEventManager.f8986WWWWoWWWWo);
                                    vMEventManager.f8986WWWWoWWWWo = null;
                                    throw th;
                                }
                            } catch (Exception e11) {
                                e = e11;
                                localSocket = null;
                            } catch (Throwable th3) {
                                th = th3;
                                localSocket = null;
                            }
                        } catch (Exception e12) {
                            e = e12;
                            localSocket = null;
                            localServerSocket = null;
                        } catch (Throwable th4) {
                            th = th4;
                            localSocket = null;
                            localServerSocket = null;
                        }
                        if (inputStream != null && outputStream != null) {
                            vMEventManager.f8987WWWWWWWW = new DataInputStream(inputStream);
                            vMEventManager.f8986WWWWoWWWWo = new DataOutputStream(outputStream);
                            wwwwwwww.getClass();
                            KLog.m5043WWWWWWWW(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-14, -98, 57, TarConstants.LF_NORMAL, 16, -35, -78, -2, -57, -125, 57, 39, 17, -60, -98, -1, -12, -51, TarConstants.LF_SYMLINK, 33, 18, -115, -110, -2, -57, -125, 57, 39, 17, -60, -98, -1}, new byte[]{-87, -19, 92, 68, 101, -83, -15, -111}));
                            while (!vMEventManager.f8990WWWoWWWo && vMEventManager.f8988WWWWWWWW.f8940WWoWWo != -5) {
                                vMEventManager.m5114WWWWoWWWWo();
                            }
                            str = VMEventManager.f8985WWoWWo;
                            StringFog.f8859WWWWWWWW.getClass();
                            m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{85, -31, -122, -84, 26, -53, 34, -79, 96, -4, -122, -69, 27, -46, 14, -80, TarConstants.LF_GNUTYPE_SPARSE, -78, -122, -74, ConstantPoolEntry.CP_InterfaceMethodref}, new byte[]{14, -110, -29, -40, 111, -69, 97, -34});
                            KLog.m5043WWWWWWWW(str, m17835WWWWWWWW);
                            VMEventManager.m5113WWWWWWWW(localServerSocket);
                            VMEventManager.m5113WWWWWWWW(localSocket);
                            VMEventManager.m5113WWWWWWWW(vMEventManager.f8987WWWWWWWW);
                            vMEventManager.f8987WWWWWWWW = null;
                            VMEventManager.m5113WWWWWWWW(vMEventManager.f8986WWWWoWWWWo);
                            vMEventManager.f8986WWWWoWWWWo = null;
                        }
                        wwwwwwww.getClass();
                        KLog.m5040WWWWoWWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-43, -99, 59, 73, -60, -78, -18, -17, -32, Byte.MIN_VALUE, 59, 94, -59, -85, -62, -18, -45, -50, TarConstants.LF_NORMAL, 82, -111, -85, -62}, new byte[]{-114, -18, 94, 61, -79, -62, -83, Byte.MIN_VALUE}));
                        byte[] bArr5 = {32, TarConstants.LF_FIFO, ConstantPoolEntry.CP_NameAndType, 41, -122, Byte.MIN_VALUE, 36, -13, 21, 43, ConstantPoolEntry.CP_NameAndType, 62, -121, -103, 8, -14, 38, 101, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_CHR, -105};
                        byte[] bArr6 = {123, 69, 105, 93, -13, -16, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -100};
                        wwwwwwww.getClass();
                        KLog.m5043WWWWWWWW(str2, WWWWWWWW.m17835WWWWWWWW(bArr5, bArr6));
                        VMEventManager.m5113WWWWWWWW(localServerSocket);
                        VMEventManager.m5113WWWWWWWW(localSocket);
                        VMEventManager.m5113WWWWWWWW(vMEventManager.f8987WWWWWWWW);
                        vMEventManager.f8987WWWWWWWW = null;
                        VMEventManager.m5113WWWWWWWW(vMEventManager.f8986WWWWoWWWWo);
                        vMEventManager.f8986WWWWoWWWWo = null;
                    } else {
                        return;
                    }
                }
            }
        }.start();
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final void m5116WWWoWWWo(String str, String str2) {
        try {
            if (this.f8986WWWWoWWWWo == null) {
                String str3 = f8985WWoWWo;
                byte[] bArr = {-47, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -126, -38, -87, -15, 99, 71, -28, Byte.MAX_VALUE, -70, -108, -93, -37, TarConstants.LF_DIR, 65, -27, 101, -119, -47, -82, -64, 124, TarConstants.LF_MULTIVOLUME, -28};
                byte[] bArr2 = {-118, ConstantPoolEntry.CP_InterfaceMethodref, -25, -76, -51, -76, 21, 34};
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5040WWWWoWWWWo(str3, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
                return;
            }
            String str4 = f8985WWoWWo;
            StringBuilder sb2 = new StringBuilder();
            byte[] bArr3 = {TarConstants.LF_NORMAL, -12, 89, -83, -66, 33, -35, Byte.MIN_VALUE};
            StringFog.f8859WWWWWWWW.getClass();
            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{107, -121, 60, -61, -38, 100, -85, -27, 94, Byte.MIN_VALUE, 4, -115, -51, 85, -68, -14, 68, -44, 42, -56, -48, 69, -3}, bArr3));
            sb2.append(str);
            sb2.append(" ");
            sb2.append(str2);
            KLog.m5043WWWWWWWW(str4, sb2.toString());
            synchronized (this) {
                this.f8986WWWWoWWWWo.writeUTF(str + WWWWWWWW.m17835WWWWWWWW(new byte[]{42}, new byte[]{74, 9, 117, 85, -42, 39, -103, 58}) + str2);
                this.f8986WWWWoWWWWo.flush();
            }
            KLog.m5043WWWWWWWW(str4, WWWWWWWW.m17835WWWWWWWW(new byte[]{-65, -98, 117, 126, -57, -57, 30, 2, -118, -103, TarConstants.LF_MULTIVOLUME, TarConstants.LF_NORMAL, -48, -25, 6, 19}, new byte[]{-28, -19, 16, 16, -93, -126, 104, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}));
        } catch (Throwable th2) {
            String str5 = f8985WWoWWo;
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5044WWWoWWWo(str5, WWWWWWWW.m17835WWWWWWWW(new byte[]{82, TarConstants.LF_FIFO, 9, 125, -20, 2, -68, 59, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, TarConstants.LF_LINK, TarConstants.LF_LINK, TarConstants.LF_CHR, -19, 63, -16, 126}, new byte[]{9, 69, 108, 19, -120, 71, -54, 94}), th2);
        }
    }
}
