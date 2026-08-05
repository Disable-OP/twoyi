package com.android.vmcore.hal.phone;

import android.os.Parcel;
import android.os.Parcelable;
import com.android.vmcore.StringFog;
import com.google.android.gms.internal.ads.pr0;
import java.util.Arrays;
import org.apache.commons.compress.archivers.tar.TarConstants;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class AdnRecord implements Parcelable {
    public static final Parcelable.Creator<AdnRecord> CREATOR;

    /* renamed from: WWoၑWWoړၑ  reason: contains not printable characters */
    public static final String f9120WWoWWo;

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public String f9121WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public String f9122WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public String[] f9123WWWWWWWW;

    /* renamed from: WWWWཀྵWWWWࠤཀྵ  reason: contains not printable characters */
    public int f9124WWWWWWWW;

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public int f9125WWWW;

    /* renamed from: com.android.vmcore.hal.phone.AdnRecord$1  reason: invalid class name */
    /* loaded from: classes.dex */
    public class AnonymousClass1 implements Parcelable.Creator<AdnRecord> {
        /* JADX WARN: Type inference failed for: r4v0, types: [com.android.vmcore.hal.phone.AdnRecord, java.lang.Object] */
        @Override // android.os.Parcelable.Creator
        public final AdnRecord createFromParcel(Parcel parcel) {
            int readInt = parcel.readInt();
            int readInt2 = parcel.readInt();
            String readString = parcel.readString();
            String readString2 = parcel.readString();
            String[] createStringArray = parcel.createStringArray();
            ?? obj = new Object();
            obj.f9125WWWW = readInt;
            obj.f9124WWWWWWWW = readInt2;
            obj.f9122WWWWoWWWWo = readString;
            obj.f9121WWWWWWWWWW = readString2;
            obj.f9123WWWWWWWW = createStringArray;
            return obj;
        }

        @Override // android.os.Parcelable.Creator
        public final AdnRecord[] newArray(int i10) {
            return new AdnRecord[i10];
        }
    }

    /* JADX WARN: Type inference failed for: r0v3, types: [java.lang.Object, android.os.Parcelable$Creator<com.android.vmcore.hal.phone.AdnRecord>] */
    static {
        StringFog.f8859WWWWWWWW.getClass();
        f9120WWoWWo = WWWWWWWW.m17835WWWWWWWW(new byte[]{9, -103, -7, -34, TarConstants.LF_BLK, 78, -37, -20, 44}, new byte[]{72, -3, -105, -116, 81, 45, -76, -98});
        CREATOR = new Object();
    }

    @Override // android.os.Parcelable
    public final int describeContents() {
        return 0;
    }

    public final String toString() {
        StringBuilder sb2 = new StringBuilder();
        pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{126, 30, -22, 93, 82, 96, 58, -34, TarConstants.LF_MULTIVOLUME, 62, -124, 90}, new byte[]{63, 90, -92, 125, 0, 5, 89, -79}, sb2);
        sb2.append(this.f9122WWWWoWWWWo);
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{79, 80, 7}, new byte[]{104, 112, 32, -39, -113, 86, -74, TarConstants.LF_BLK}));
        sb2.append(this.f9121WWWWWWWWWW);
        sb2.append(" ");
        sb2.append(Arrays.toString(this.f9123WWWWWWWW));
        return pr0.m9000WWWWWWWW(new byte[]{70}, new byte[]{97, -82, -28, -54, TarConstants.LF_FIFO, -121, 126, -7}, sb2);
    }

    @Override // android.os.Parcelable
    public final void writeToParcel(Parcel parcel, int i10) {
        parcel.writeInt(this.f9125WWWW);
        parcel.writeInt(this.f9124WWWWWWWW);
        parcel.writeString(this.f9122WWWWoWWWWo);
        parcel.writeString(this.f9121WWWWWWWWWW);
        parcel.writeStringArray(this.f9123WWWWWWWW);
    }
}
