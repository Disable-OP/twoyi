package com.android.libadb.core;

import android.util.Log;
import bd.WWWoWWWo;
import com.android.org.conscrypt.Conscrypt;
import gc.C2596WWWWWWWW;
import java.io.Closeable;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.net.Socket;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.Charset;
import javax.net.ssl.SSLSocket;
import javax.net.ssl.SSLSocketFactory;
import kotlin.jvm.internal.AbstractC3339WWWWWWWW;
import n2.C3534WWWWWWWW;
import o2.C3625WWWWWWWW;
import o2.C3631WWWWWWWW;
import o2.C3634WWWoWWWo;
import o2.EnumC3629WWWWWWWW;
import o2.EnumC3632WWWWWWWW;
import o2.EnumC3635WWoWWo;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public final class AdbPairingClient implements Closeable, AutoCloseable {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final int f8254WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final String f8255WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final String f8256WWWWWWWW;

    /* renamed from: WWWWཀྵWWWWࠤཀྵ  reason: contains not printable characters */
    public Socket f8257WWWWWWWW;

    /* renamed from: WWWWၗWWWW३ၗ  reason: contains not printable characters */
    public DataOutputStream f8258WWWWWWWW;

    /* renamed from: WWWoၙWWWo੮ၙ  reason: contains not printable characters */
    public final C3634WWWoWWWo f8259WWWoWWWo;

    /* renamed from: WWoၑWWoړၑ  reason: contains not printable characters */
    public DataInputStream f8260WWoWWo;

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public final C3625WWWWWWWW f8261WWWW;

    /* renamed from: WWၚஊၚ  reason: contains not printable characters */
    public PairingContext f8262WW;

    /* renamed from: WoოWo੍ო  reason: contains not printable characters */
    public EnumC3629WWWWWWWW f8263WoWo;

    static {
        byte[] bArr = {105, TarConstants.LF_GNUTYPE_LONGLINK, -122};
        byte[] bArr2 = {8, 47, -28, 91, 106, TarConstants.LF_CONTIG, 106, 3};
        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
        System.loadLibrary(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
    }

    /* JADX WARN: Type inference failed for: r6v2, types: [java.lang.Object, fc.WWWWҍWWWWּҍ] */
    public AdbPairingClient(String str, int i10, String str2, C3625WWWWWWWW c3625wwwwwwww) {
        byte[] bArr = {-41, TarConstants.LF_PAX_EXTENDED_HEADER_LC, TarConstants.LF_GNUTYPE_SPARSE, -119, 107, 23, -13, -45};
        i0.WWWWWWWW.m14530WWWW(C3534WWWWWWWW.f31122WWWWWWWW, new byte[]{-65, 23, 32, -3}, bArr, str);
        AbstractC3339WWWWWWWW.m15439WWoWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-106, -113, TarConstants.LF_NORMAL, -125, 85, 92, -76, 44}, new byte[]{-26, -18, 89, -15, 22, TarConstants.LF_CHR, -48, 73}));
        AbstractC3339WWWWWWWW.m15439WWoWWo(c3625wwwwwwww, WWWWWWWW.m17835WWWWWWWW(new byte[]{-93, -51, -88}, new byte[]{-56, -88, -47, -28, -23, 65, 62, -53}));
        this.f8255WWWWoWWWWo = str;
        this.f8254WWWWWWWWWW = i10;
        this.f8256WWWWWWWW = str2;
        this.f8261WWWW = c3625wwwwwwww;
        EnumC3632WWWWWWWW.f31568WWWWoWWWWo.getClass();
        this.f8259WWWoWWWo = new C3634WWWoWWWo((byte) 0, (byte[]) c3625wwwwwwww.f31544WWWWWWWW.getValue());
        this.f8263WoWo = EnumC3629WWWWWWWW.f31557WWWWoWWWWo;
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final C3631WWWWWWWW m4826WWWWoWWWWo() {
        byte[] bArr = new byte[6];
        DataInputStream dataInputStream = this.f8260WWoWWo;
        if (dataInputStream != null) {
            dataInputStream.readFully(bArr);
            ByteBuffer order = ByteBuffer.wrap(bArr).order(ByteOrder.BIG_ENDIAN);
            AbstractC3339WWWWWWWW.m15436WWWoWWWo(order);
            byte[] bArr2 = {-43, TarConstants.LF_LINK, -88, -16, -52, -35, -30, 125};
            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
            WWWWWWWW.m17835WWWWWWWW(new byte[]{-73, 68, -50, -106, -87, -81}, bArr2);
            byte b8 = order.get();
            byte b10 = order.get();
            int i10 = order.getInt();
            if (b8 >= 1 && b8 <= 1) {
                if (b10 != EnumC3635WWoWWo.f31579WWWWWWWWWW.f31582WWWWoWWWWo && b10 != EnumC3635WWoWWo.f31580WWWWWWWW.f31582WWWWoWWWWo) {
                    String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{9, 93, -40, -26, 110, TarConstants.LF_CHR, 5, -122, 36, 80, -33, -40, 123}, new byte[]{72, 57, -70, -74, 15, 90, 119, -59});
                    Log.e(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{110, -11, TarConstants.LF_DIR, 95, 59, 35, -98, -60, 107, -6, TarConstants.LF_CONTIG, 67, 61, 58, -105, -76, 90, -8, TarConstants.LF_DIR, 84, 32, 116, -124, -99, TarConstants.LF_GNUTYPE_LONGLINK, -2, 99}, new byte[]{59, -101, 94, TarConstants.LF_LINK, 84, 84, -16, -28}) + ((int) b10));
                    return null;
                } else if (i10 > 0 && i10 <= 16384) {
                    C3631WWWWWWWW c3631wwwwwwww = new C3631WWWWWWWW(b8, b10, i10);
                    String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{93, 61, -85, -89, -12, -121, -15, 64, 112, TarConstants.LF_NORMAL, -84, -103, -31}, new byte[]{28, 89, -55, -9, -107, -18, -125, 3});
                    Log.d(m17835WWWWWWWW2, WWWWWWWW.m17835WWWWWWWW(new byte[]{9, 71, -52, 29, -66, -73, 101, -13, 9, TarConstants.LF_GNUTYPE_LONGLINK, -61, 30, -50, -122, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -15, 30, 86, -27, 28, -1, -125, 97, -24, 91}, new byte[]{123, 34, -83, 121, -98, -25, 4, -102}) + c3631wwwwwwww.m16209WWWWWWWW());
                    return c3631wwwwwwww;
                } else {
                    String m17835WWWWWWWW3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-59, 80, -42, 5, 92, -114, TarConstants.LF_GNUTYPE_LONGNAME, -37, -24, 93, -47, 59, 73}, new byte[]{-124, TarConstants.LF_BLK, -76, 85, 61, -25, 62, -104});
                    Log.e(m17835WWWWWWWW3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-22, -92, -7, 112, -53, -89, -103, 111, -29, -72, -12, 123, -49, -79, -103, 113, -19, -75, -72, 99, -57, -95, -47, 118, -20, -31, -7, TarConstants.LF_BLK, -35, -76, -33, 122, -94, -79, -7, 109, -62, -70, -40, 123, -94, -78, -15, 110, -53, -11, -111, 108, -21, -69, -3, 41}, new byte[]{-126, -63, -104, 20, -82, -43, -71, 31}) + i10 + ')');
                    return null;
                }
            }
            String m17835WWWWWWWW4 = WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 0, 16, 93, 35, 21, -56, 74, 74, 13, 23, 99, TarConstants.LF_FIFO}, new byte[]{38, 100, 114, 13, 66, 124, -70, 9});
            Log.e(m17835WWWWWWWW4, WWWWWWWW.m17835WWWWWWWW(new byte[]{-102, 57, 68, 47, -11, TarConstants.LF_GNUTYPE_LONGNAME, -24, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -85, 59, 70, 56, -24, 106, -22, 73, -82, 61, 95, 125, -22, 71, -3, 91, -93, TarConstants.LF_CONTIG, 67, 125, -15, TarConstants.LF_GNUTYPE_LONGLINK, -4, 69, -85, 44, 78, TarConstants.LF_DIR, -68, 10, -6, 91, -9, 105, 13, 41, -12, 71, -30, 21}, new byte[]{-54, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 45, 93, -100, 34, -113, 40}) + ((int) b8) + ')');
            return null;
        }
        i0.WWWWWWWW.m14532o(C3534WWWWWWWW.f31122WWWWWWWW, new byte[]{105, -90, -7, 99, 34, -37, -8, 122, 101, -87, -28}, new byte[]{0, -56, -119, 22, 86, -120, -116, 8});
        throw null;
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final void m4827WWWWWWWW() {
        long nativeConstructor;
        String str = this.f8255WWWWoWWWWo;
        int i10 = this.f8254WWWWWWWWWW;
        Socket socket = new Socket(str, i10);
        this.f8257WWWWWWWW = socket;
        socket.setTcpNoDelay(true);
        SSLSocketFactory socketFactory = this.f8261WWWW.m16207WWWWWWWW().getSocketFactory();
        Socket socket2 = this.f8257WWWWWWWW;
        PairingContext pairingContext = null;
        if (socket2 != null) {
            Socket createSocket = socketFactory.createSocket(socket2, str, i10, true);
            byte[] bArr = {61, -117, 107, 41, 90, 8, -12, -29, 61, -111, 115, 101, 24, 14, -75, -18, TarConstants.LF_SYMLINK, -115, 115, 101, 14, 4, -75, -29, 60, -112, 42, 43, 15, 7, -7, -83, 39, -121, 119, 32, 90, 1, -12, -5, TarConstants.LF_SYMLINK, -122, 41, 43, 31, 31, -69, -2, 32, -110, 41, 22, 41, 39, -58, -30, TarConstants.LF_NORMAL, -107, 98, TarConstants.LF_LINK};
            byte[] bArr2 = {TarConstants.LF_GNUTYPE_SPARSE, -2, 7, 69, 122, 107, -107, -115};
            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
            AbstractC3339WWWWWWWW.m15428WWWWWWWW(createSocket, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
            SSLSocket sSLSocket = (SSLSocket) createSocket;
            sSLSocket.startHandshake();
            Log.d(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, -66, -25, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 123, 13, -15, 61, 102, -77, -32, 70, 110}, new byte[]{10, -38, -123, 40, 26, 100, -125, 126}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-57, -3, TarConstants.LF_SYMLINK, -14, 3, 108, -85, -80, -22, -68, 47, -29, 19, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -81, -66, -21, -7, 56, -72}, new byte[]{-113, -100, 92, -106, 112, 4, -54, -37}));
            this.f8260WWoWWo = new DataInputStream(sSLSocket.getInputStream());
            this.f8258WWWWWWWW = new DataOutputStream(sSLSocket.getOutputStream());
            Charset charset = WWWoWWWo.f7191WWWWWWWW;
            String str2 = this.f8256WWWWWWWW;
            byte[] bytes = str2.getBytes(charset);
            AbstractC3339WWWWWWWW.m15429WWWWWWWW(bytes, WWWWWWWW.m17835WWWWWWWW(new byte[]{-127, 61, TarConstants.LF_CHR, 37, -84, -54, 91, -109, -50, 118, 105, 73, -4}, new byte[]{-26, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 71, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -43, -66, 62, -32}));
            byte[] exportKeyingMaterial = Conscrypt.exportKeyingMaterial(sSLSocket, WWWWWWWW.m17835WWWWWWWW(new byte[]{-81, -44, -112, 93, -16, -36, 4, 65, -94, -80}, new byte[]{-50, -80, -14, 112, -100, -67, 102, 36}), (byte[]) null, 64);
            byte[] bArr3 = new byte[str2.length() + exportKeyingMaterial.length];
            C2596WWWWWWWW.m14133WWWWWWWW(bytes, 0, 0, bArr3, 0, 14);
            C2596WWWWWWWW.m14133WWWWWWWW(exportKeyingMaterial, bytes.length, 0, bArr3, 0, 12);
            WWWWWWWW.m17835WWWWWWWW(new byte[]{-127, -21, -14, 25, -110, 6, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -99}, new byte[]{-15, -118, -127, 106, -27, 105, 10, -7});
            nativeConstructor = PairingContext.nativeConstructor(true, bArr3);
            if (nativeConstructor != 0) {
                pairingContext = new PairingContext(nativeConstructor);
            }
            if (pairingContext != null) {
                this.f8262WW = pairingContext;
                return;
            }
            throw new IllegalStateException(WWWWWWWW.m17835WWWWWWWW(new byte[]{-85, 4, -28, -2, -127, -2, 113, -82, -111, 74, -26, -18, -120, -6, 37, -65, -34, 58, -28, -11, -97, -14, 63, -67, -67, 5, -21, -24, -120, -29, 37, -12}, new byte[]{-2, 106, -123, -100, -19, -101, 81, -38}).toString());
        }
        byte[] bArr4 = {TarConstants.LF_SYMLINK, -19, 71, 40, 92, -35, -73, -35};
        i0.WWWWWWWW.m14532o(C3534WWWWWWWW.f31122WWWWWWWW, new byte[]{65, -126, 36, 67, 57, -87}, bArr4);
        throw null;
    }

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public final void m4828WWWWWWWW(C3631WWWWWWWW c3631wwwwwwww, byte[] bArr) {
        ByteBuffer order = ByteBuffer.allocate(6).order(ByteOrder.BIG_ENDIAN);
        AbstractC3339WWWWWWWW.m15436WWWoWWWo(order);
        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
        WWWWWWWW.m17835WWWWWWWW(new byte[]{-96, TarConstants.LF_FIFO, -118, -124, 37, 47}, new byte[]{-62, 67, -20, -30, 64, 93, 34, -32});
        order.put(c3631wwwwwwww.f31565WWWWWWWW);
        order.put(c3631wwwwwwww.f31564WWWWoWWWWo);
        order.putInt(c3631wwwwwwww.f31566WWWoWWWo);
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-114, 70, -32, -123, 4, -36, 64, 82, -93, TarConstants.LF_GNUTYPE_LONGLINK, -25, -69, 17}, new byte[]{-49, 34, -126, -43, 101, -75, TarConstants.LF_SYMLINK, 17});
        Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{Byte.MAX_VALUE, 116, -103, 124, -14, TarConstants.LF_MULTIVOLUME, 121, 41, 97, 116, -103, 102, -16, 61, 72, 43, 99, 99, -124, 64, -14, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_MULTIVOLUME, 45, 122, 38}, new byte[]{8, 6, -16, 8, -105, 109, 41, 72}) + c3631wwwwwwww.m16209WWWWWWWW());
        DataOutputStream dataOutputStream = this.f8258WWWWWWWW;
        if (dataOutputStream != null) {
            dataOutputStream.write(order.array());
            DataOutputStream dataOutputStream2 = this.f8258WWWWWWWW;
            if (dataOutputStream2 != null) {
                dataOutputStream2.write(bArr);
                String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-121, -84, -16, -6, TarConstants.LF_CONTIG, -99, -89, 98, -86, -95, -9, -60, 34}, new byte[]{-58, -56, -110, -86, 86, -12, -43, 33});
                Log.d(m17835WWWWWWWW2, WWWWWWWW.m17835WWWWWWWW(new byte[]{65, 64, TarConstants.LF_DIR, 69, -65, 92, TarConstants.LF_DIR, 35, 79, 94, TarConstants.LF_CHR, 80, -66, 80, 101, TarConstants.LF_LINK, 95, 72, 57, ConstantPoolEntry.CP_NameAndType}, new byte[]{TarConstants.LF_FIFO, TarConstants.LF_SYMLINK, 92, TarConstants.LF_LINK, -38, 124, 69, 66}) + bArr.length);
                return;
            }
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{81, -10, 13, -102, 3, 2, -34, -71, TarConstants.LF_GNUTYPE_LONGNAME, -26, 24, -121}, new byte[]{62, -125, 121, -22, 118, 118, -115, -51}));
            throw null;
        }
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{36, TarConstants.LF_GNUTYPE_SPARSE, -98, 32, -112, -119, -122, 69, 57, 67, -117, 61}, new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, 38, -22, 80, -27, -3, -43, TarConstants.LF_LINK}));
        throw null;
    }

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public final boolean m4829WWoWWo() {
        m4827WWWWWWWW();
        this.f8263WoWo = EnumC3629WWWWWWWW.f31556WWWWWWWWWW;
        PairingContext pairingContext = this.f8262WW;
        if (pairingContext != null) {
            byte[] bArr = pairingContext.f8264WWWWoWWWWo;
            int length = bArr.length;
            EnumC3635WWoWWo enumC3635WWoWWo = EnumC3635WWoWWo.f31579WWWWWWWWWW;
            m4828WWWWWWWW(new C3631WWWWWWWW((byte) 1, enumC3635WWoWWo.f31582WWWWoWWWWo, length), bArr);
            C3631WWWWWWWW m4826WWWWoWWWWo = m4826WWWWoWWWWo();
            if (m4826WWWWoWWWWo != null && m4826WWWWoWWWWo.f31564WWWWoWWWWo == enumC3635WWoWWo.f31582WWWWoWWWWo) {
                byte[] bArr2 = new byte[m4826WWWWoWWWWo.f31566WWWoWWWo];
                DataInputStream dataInputStream = this.f8260WWoWWo;
                if (dataInputStream != null) {
                    dataInputStream.readFully(bArr2);
                    PairingContext pairingContext2 = this.f8262WW;
                    if (pairingContext2 != null) {
                        if (pairingContext2.m4833WWWWWWWW(bArr2)) {
                            this.f8263WoWo = EnumC3629WWWWWWWW.f31558WWWWWWWW;
                            ByteBuffer order = ByteBuffer.allocate(8192).order(ByteOrder.BIG_ENDIAN);
                            AbstractC3339WWWWWWWW.m15436WWWoWWWo(order);
                            C3634WWWoWWWo c3634WWWoWWWo = this.f8259WWWoWWWo;
                            c3634WWWoWWWo.getClass();
                            byte[] bArr3 = {9, -126, -98, -95, 114, TarConstants.LF_MULTIVOLUME};
                            byte[] bArr4 = {107, -9, -8, -57, 23, 63, TarConstants.LF_GNUTYPE_SPARSE, 46};
                            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                            WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4);
                            order.put(c3634WWWoWWWo.f31577WWWWWWWW);
                            order.put(c3634WWWoWWWo.f31576WWWWoWWWWo);
                            String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-23, -5, -83, -43, -86, -121, -47, 59, -60, -10, -86, -21, -65}, new byte[]{-88, -97, -49, -123, -53, -18, -93, TarConstants.LF_PAX_EXTENDED_HEADER_LC});
                            Log.d(m17835WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-49, 6, -42, -13, 36, -51, 61, -28, -35, 6, -10, -23, 39, -126, TarConstants.LF_MULTIVOLUME}, new byte[]{-72, 116, -65, -121, 65, -19, 109, -127}) + c3634WWWoWWWo.m16211WWWWWWWW());
                            PairingContext pairingContext3 = this.f8262WW;
                            if (pairingContext3 != null) {
                                byte[] array = order.array();
                                AbstractC3339WWWWWWWW.m15429WWWWWWWW(array, WWWWWWWW.m17835WWWWWWWW(new byte[]{-36, 81, 98, 62, -23, 27, TarConstants.LF_NORMAL, -41, -109, 10}, new byte[]{-67, 35, 16, 95, -112, TarConstants.LF_CHR, 30, -7}));
                                byte[] m4832WWWWWWWW = pairingContext3.m4832WWWWWWWW(array);
                                if (m4832WWWWWWWW != null) {
                                    EnumC3635WWoWWo enumC3635WWoWWo2 = EnumC3635WWoWWo.f31580WWWWWWWW;
                                    m4828WWWWWWWW(new C3631WWWWWWWW((byte) 1, enumC3635WWoWWo2.f31582WWWWoWWWWo, m4832WWWWWWWW.length), m4832WWWWWWWW);
                                    C3631WWWWWWWW m4826WWWWoWWWWo2 = m4826WWWWoWWWWo();
                                    if (m4826WWWWoWWWWo2 != null && m4826WWWWoWWWWo2.f31564WWWWoWWWWo == enumC3635WWoWWo2.f31582WWWWoWWWWo) {
                                        byte[] bArr5 = new byte[m4826WWWWoWWWWo2.f31566WWWoWWWo];
                                        DataInputStream dataInputStream2 = this.f8260WWoWWo;
                                        if (dataInputStream2 != null) {
                                            dataInputStream2.readFully(bArr5);
                                            PairingContext pairingContext4 = this.f8262WW;
                                            if (pairingContext4 != null) {
                                                byte[] m4831WWWWoWWWWo = pairingContext4.m4831WWWWoWWWWo(bArr5);
                                                if (m4831WWWWoWWWWo != null) {
                                                    if (m4831WWWWoWWWWo.length != 8192) {
                                                        String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-10, 117, -41, -93, -90, -84, -21, -88, -37, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -48, -99, -77}, new byte[]{-73, 17, -75, -13, -57, -59, -103, -21});
                                                        Log.e(m17835WWWWWWWW2, WWWWWWWW.m17835WWWWWWWW(new byte[]{20, 20, -3, -25, 107, 4, -109, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 110}, new byte[]{TarConstants.LF_GNUTYPE_SPARSE, 123, -119, -57, 24, 109, -23, 61}) + m4831WWWWoWWWWo.length + WWWWWWWW.m17835WWWWWWWW(new byte[]{90, 62, 87, -90, 112, -81, 28, 111, 21, 64, 65, -86, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -125, 79, TarConstants.LF_LINK, TarConstants.LF_GNUTYPE_LONGLINK, 87, 0}, new byte[]{122, 110, TarConstants.LF_SYMLINK, -61, 2, -26, 114, 9}));
                                                    } else {
                                                        ByteBuffer wrap = ByteBuffer.wrap(m4831WWWWoWWWWo);
                                                        AbstractC3339WWWWWWWW.m15429WWWWWWWW(wrap, WWWWWWWW.m17835WWWWWWWW(new byte[]{42, -11, 22, 39, 60, 87, 80, 89, 116}, new byte[]{93, -121, 119, 87, 20, 121, 126, 119}));
                                                        WWWWWWWW.m17835WWWWWWWW(new byte[]{46, 68, 108, 64, -108, -101}, new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, TarConstants.LF_LINK, 10, 38, -15, -23, -119, 95});
                                                        byte b8 = wrap.get();
                                                        byte[] bArr6 = new byte[8191];
                                                        wrap.get(bArr6);
                                                        Log.d(WWWWWWWW.m17835WWWWWWWW(new byte[]{-35, 125, -42, -90, -22, -94, 101, 24, -16, 112, -47, -104, -1}, new byte[]{-100, 25, -76, -10, -117, -53, 23, 91}), new C3634WWWoWWWo(b8, bArr6).toString());
                                                        this.f8263WoWo = EnumC3629WWWWWWWW.f31560WWWW;
                                                        return true;
                                                    }
                                                } else {
                                                    throw new AdbInvalidPairingCodeException();
                                                }
                                            } else {
                                                AbstractC3339WWWWWWWW.m15434WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{44, -74, -48, 121, TarConstants.LF_DIR, 3, -68, TarConstants.LF_CHR, TarConstants.LF_CHR, -71, -51, 110, 36, 25}, new byte[]{92, -41, -71, ConstantPoolEntry.CP_InterfaceMethodref, 92, 109, -37, 112}));
                                                throw null;
                                            }
                                        } else {
                                            AbstractC3339WWWWWWWW.m15434WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{0, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 36, 116, -51, -18, -44, -109, ConstantPoolEntry.CP_NameAndType, 119, 57}, new byte[]{105, 22, 84, 1, -71, -67, -96, -31}));
                                            throw null;
                                        }
                                    }
                                }
                                this.f8263WoWo = EnumC3629WWWWWWWW.f31560WWWW;
                                return false;
                            }
                            AbstractC3339WWWWWWWW.m15434WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{110, -69, Byte.MAX_VALUE, 2, 23, -5, -54, -2, 113, -76, 98, 21, 6, -31}, new byte[]{30, -38, 22, 112, 126, -107, -83, -67}));
                            throw null;
                        }
                    } else {
                        i0.WWWWWWWW.m14532o(C3534WWWWWWWW.f31122WWWWWWWW, new byte[]{20, -89, 38, 82, 101, 5, -118, 67, ConstantPoolEntry.CP_InterfaceMethodref, -88, 59, 69, 116, 31}, new byte[]{100, -58, 79, 32, ConstantPoolEntry.CP_NameAndType, 107, -19, 0});
                        throw null;
                    }
                } else {
                    byte[] bArr7 = {-74, 79, -56, TarConstants.LF_GNUTYPE_SPARSE, -82, 69, 122, -8};
                    i0.WWWWWWWW.m14532o(C3534WWWWWWWW.f31122WWWWWWWW, new byte[]{-33, 33, -72, 38, -38, 22, 14, -118, -45, 46, -91}, bArr7);
                    throw null;
                }
            }
            this.f8263WoWo = EnumC3629WWWWWWWW.f31560WWWW;
            return false;
        }
        i0.WWWWWWWW.m14532o(C3534WWWWWWWW.f31122WWWWWWWW, new byte[]{Byte.MIN_VALUE, -96, 82, -64, 34, -48, -95, 65, -97, -81, 79, -41, TarConstants.LF_CHR, -54}, new byte[]{-16, -63, 59, -78, TarConstants.LF_GNUTYPE_LONGLINK, -66, -58, 2});
        throw null;
    }

    @Override // java.io.Closeable, java.lang.AutoCloseable
    public final void close() {
        Socket socket;
        DataOutputStream dataOutputStream;
        DataInputStream dataInputStream;
        try {
            dataInputStream = this.f8260WWoWWo;
        } catch (Throwable unused) {
        }
        if (dataInputStream != null) {
            dataInputStream.close();
            try {
                dataOutputStream = this.f8258WWWWWWWW;
            } catch (Throwable unused2) {
            }
            if (dataOutputStream != null) {
                dataOutputStream.close();
                try {
                    socket = this.f8257WWWWWWWW;
                } catch (Exception unused3) {
                }
                if (socket != null) {
                    socket.close();
                    if (this.f8263WoWo != EnumC3629WWWWWWWW.f31557WWWWoWWWWo) {
                        PairingContext pairingContext = this.f8262WW;
                        if (pairingContext != null) {
                            pairingContext.m4834WWWoWWWo();
                            return;
                        }
                        i0.WWWWWWWW.m14532o(C3534WWWWWWWW.f31122WWWWWWWW, new byte[]{-79, 107, -100, 91, 94, -56, 89, 71, -82, 100, -127, TarConstants.LF_GNUTYPE_LONGNAME, 79, -46}, new byte[]{-63, 10, -11, 41, TarConstants.LF_CONTIG, -90, 62, 4});
                        throw null;
                    }
                    return;
                }
                C3534WWWWWWWW.f31122WWWWWWWW.getClass();
                AbstractC3339WWWWWWWW.m15434WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{101, -94, -78, 35, 104, -69}, new byte[]{22, -51, -47, 72, 13, -49, 57, 2}));
                throw null;
            }
            C3534WWWWWWWW.f31122WWWWWWWW.getClass();
            AbstractC3339WWWWWWWW.m15434WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{-38, -99, 105, -108, -109, -1, -90, -34, -57, -115, 124, -119}, new byte[]{-75, -24, 29, -28, -26, -117, -11, -86}));
            throw null;
        }
        byte[] bArr = {43, -88, 66, 79, -72, -108, TarConstants.LF_BLK, -1, 39, -89, 95};
        byte[] bArr2 = {66, -58, TarConstants.LF_SYMLINK, 58, -52, -57, 64, -115};
        C3534WWWWWWWW.f31122WWWWWWWW.getClass();
        AbstractC3339WWWWWWWW.m15434WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        throw null;
    }
}
