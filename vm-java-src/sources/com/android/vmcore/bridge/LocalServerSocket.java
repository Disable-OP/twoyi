package com.android.vmcore.bridge;

import android.net.LocalSocket;
import android.net.LocalSocketAddress;
import com.android.vmcore.StringFog;
import com.blankj.utilcode.util.WoWo;
import java.io.Closeable;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class LocalServerSocket implements Closeable, AutoCloseable {

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final Object f8984WWWWoWWWWo;

    public LocalServerSocket(String str) {
        LocalSocketAddress.Namespace namespace = LocalSocketAddress.Namespace.FILESYSTEM;
        StringFog.f8859WWWWWWWW.getClass();
        Object obj = WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{33, 46, 105, -67, 125, 84, -47, -80, 46, 37, 121, -31, 94, 82, -42, -1, 44, 19, 98, -84, 121, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -63, -41, 45, TarConstants.LF_NORMAL, 97}, new byte[]{64, 64, 13, -49, 18, 61, -75, -98})).m5362WWWWWWWW(new Object[0]).f9408WWWWoWWWWo;
        this.f8984WWWWoWWWWo = obj;
        WoWo.m5356WWWW(obj).m5361WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{-71, -118, -90, 22, -82, 60}, new byte[]{-38, -8, -61, 119, -38, 89, 69, 38}), 2);
        WoWo.m5356WWWW(obj).m5361WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{109, -115, 121, 31}, new byte[]{15, -28, 23, 123, -111, 56, -45, 86}), new LocalSocketAddress(str, namespace));
        WoWo.m5356WWWW(obj).m5361WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{-21, -29, -79, -92, 0, 80}, new byte[]{-121, -118, -62, -48, 101, 62, 18, 41}), 50);
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final LocalSocket m5112WWWWoWWWWo() {
        StringFog.f8859WWWWWWWW.getClass();
        WoWo m5362WWWWWWWW = WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{-35, 97, -70, -46, -114, -79, -112, -75, -46, 106, -86, -114, -83, -73, -105, -6, -48, 92, -79, -61, -118, -67, Byte.MIN_VALUE, -46, -47, Byte.MAX_VALUE, -78}, new byte[]{-68, 15, -34, -96, -31, -40, -12, -101})).m5362WWWWWWWW(new Object[0]);
        WoWo m5356WWWW = WoWo.m5356WWWW(this.f8984WWWWoWWWWo);
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{16, -43, -37, 116, -43, -102}, new byte[]{113, -74, -72, 17, -91, -18, 79, -86});
        Object obj = m5362WWWWWWWW.f9408WWWWoWWWWo;
        m5356WWWW.m5361WWWWWWWW(m17835WWWWWWWW, obj);
        try {
            return (LocalSocket) WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{21, TarConstants.LF_GNUTYPE_LONGNAME, -91, -11, -46, 67, -44, -113, 26, 71, -75, -87, -15, 69, -45, -64, 24, 113, -82, -28, -42, 79, -60}, new byte[]{116, 34, -63, -121, -67, 42, -80, -95})).m5361WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(new byte[]{69, 63, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -76, -86, -122, 79, -37, 69, 44, 110, -122, -79, Byte.MIN_VALUE, 104, -47, 82, ConstantPoolEntry.CP_InterfaceMethodref, 109, -89, -97, Byte.MIN_VALUE, 96, -47, 86, 57}, new byte[]{38, TarConstants.LF_MULTIVOLUME, 2, -43, -34, -29, 3, -76}), obj).f9408WWWWoWWWWo;
        } catch (Throwable unused) {
            StringFog.f8859WWWWWWWW.getClass();
            return (LocalSocket) WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{-54, -6, -113, 72, 62, 80, 99, -104, -59, -15, -97, 20, 29, 86, 100, -41, -57, -57, -124, 89, 58, 92, 115}, new byte[]{-85, -108, -21, 58, 81, 57, 7, -74})).m5362WWWWWWWW(obj, 0).f9408WWWWoWWWWo;
        }
    }

    @Override // java.io.Closeable, java.lang.AutoCloseable
    public final void close() {
        WoWo m5356WWWW = WoWo.m5356WWWW(this.f8984WWWWoWWWWo);
        byte[] bArr = {97, -22, -63, 58, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER};
        byte[] bArr2 = {2, -122, -82, 73, 2, TarConstants.LF_GNUTYPE_SPARSE, 86, 15};
        StringFog.f8859WWWWWWWW.getClass();
        m5356WWWW.m5360WWWWWWWW(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
    }
}
