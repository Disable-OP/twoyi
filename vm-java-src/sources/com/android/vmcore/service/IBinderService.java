package com.android.vmcore.service;

import android.os.Binder;
import android.os.IBinder;
import android.os.IInterface;
import android.os.Parcel;
import com.android.vmcore.StringFog;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public interface IBinderService extends IInterface {

    /* loaded from: classes.dex */
    public static class Default implements IBinderService {
        @Override // android.os.IInterface
        public final IBinder asBinder() {
            return null;
        }
    }

    /* loaded from: classes.dex */
    public static abstract class Stub extends Binder implements IBinderService {

        /* loaded from: classes.dex */
        public static class Proxy implements IBinderService {
            @Override // android.os.IInterface
            public final IBinder asBinder() {
                return null;
            }
        }

        public Stub() {
            byte[] bArr = {69, 20, 21, -81, -44, -126, TarConstants.LF_SYMLINK, -48, 73, 18, 28, -81, -61, -127, TarConstants.LF_DIR, -51, 84, 30, 86, -14, -48, -98, 32, -53, 69, 30, 86, -56, -9, -123, 56, -58, 67, 9, 43, -28, -57, -102, 63, -63, 67};
            byte[] bArr2 = {38, 123, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -127, -75, -20, 86, -94};
            StringFog.f8859WWWWWWWW.getClass();
            attachInterface(this, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        }

        @Override // android.os.IInterface
        public final IBinder asBinder() {
            return this;
        }

        @Override // android.os.Binder
        public final boolean onTransact(int i10, Parcel parcel, Parcel parcel2, int i11) {
            IBinder iBinder;
            byte[] bArr = {-31, 62, 126, -88, 82, -106, -23, 121, -19, 56, 119, -88, 69, -107, -18, 100, -16, TarConstants.LF_BLK, 61, -11, 86, -118, -5, 98, -31, TarConstants.LF_BLK, 61, -49, 113, -111, -29, 111, -25, 35, 64, -29, 65, -114, -28, 104, -25};
            byte[] bArr2 = {-126, 81, 19, -122, TarConstants.LF_CHR, -8, -115, ConstantPoolEntry.CP_InterfaceMethodref};
            StringFog.f8859WWWWWWWW.getClass();
            String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
            if (i10 >= 1 && i10 <= 16777215) {
                parcel.enforceInterface(m17835WWWWWWWW);
            }
            if (i10 == 1598968902) {
                parcel2.writeString(m17835WWWWWWWW);
                return true;
            } else if (i10 != 1) {
                if (i10 != 2) {
                    return super.onTransact(i10, parcel, parcel2, i11);
                }
                int readInt = parcel.readInt();
                int readInt2 = parcel.readInt();
                if (readInt2 == 0) {
                    iBinder = (IBinder) BinderService.f9240WWWWWWWW.get(readInt);
                } else if (readInt2 == 1) {
                    iBinder = (IBinder) BinderService.f9243WWWW.get(readInt);
                } else if (readInt2 == 2) {
                    iBinder = (IBinder) BinderService.f9241WWWWWWWW.get(readInt);
                } else {
                    iBinder = null;
                }
                parcel2.writeNoException();
                parcel2.writeStrongBinder(iBinder);
                return true;
            } else {
                int readInt3 = parcel.readInt();
                int readInt4 = parcel.readInt();
                IBinder readStrongBinder = parcel.readStrongBinder();
                if (readInt4 == 0) {
                    BinderService.f9240WWWWWWWW.put(readInt3, readStrongBinder);
                } else if (readInt4 == 1) {
                    BinderService.f9243WWWW.put(readInt3, readStrongBinder);
                } else if (readInt4 == 2) {
                    BinderService.f9241WWWWWWWW.put(readInt3, readStrongBinder);
                }
                parcel2.writeNoException();
                return true;
            }
        }
    }
}
