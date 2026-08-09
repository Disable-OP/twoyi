package com.android.vmcore.hal;

import android.annotation.SuppressLint;
import android.content.Context;
import android.location.Criteria;
import android.location.Location;
import android.location.LocationListener;
import android.location.LocationManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.text.TextUtils;
import com.android.vmcore.KLog;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.PermissionEvent;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p020WWWWWWWW.AbstractC0211WWWWWWWW;
import x5.WWWWWWWW;
@SuppressLint({"MissingPermission"})
/* loaded from: classes.dex */
public class LocationService implements LocationListener, PermissionEvent.IPermissionResultCallback {

    /* renamed from: WWኮWWႉኮ  reason: contains not printable characters */
    public static final String f9050WWWW;

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public final VMInstance f9051WWWWWWWWWW;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public final Context f9052WWWWoWWWWo;

    /* renamed from: WWWWoᆣWWWWoϿᆣ  reason: contains not printable characters */
    public final SimpleDateFormat f9053WWWWoWWWWo;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public final HALManager f9054WWWWWWWW;

    /* renamed from: WWWWཀྵWWWWࠤཀྵ  reason: contains not printable characters */
    public boolean f9055WWWWWWWW;

    /* renamed from: WWWWၗWWWW३ၗ  reason: contains not printable characters */
    public boolean f9056WWWWWWWW;

    /* renamed from: WWWWᄳWWWW़ᄳ  reason: contains not printable characters */
    public Handler f9057WWWWWWWW;

    /* renamed from: WWWWሎWWWWܣሎ  reason: contains not printable characters */
    public final SimpleDateFormat f9058WWWWWWWW;

    /* renamed from: WWWWሗWWWWшሗ  reason: contains not printable characters */
    public final Date f9059WWWWWWWW;

    /* renamed from: WWWWኈWWWWˡኈ  reason: contains not printable characters */
    public Location f9060WWWWWWWW;

    /* renamed from: WWWoၙWWWo੮ၙ  reason: contains not printable characters */
    public boolean f9061WWWoWWWo;

    /* renamed from: WWoၑWWoړၑ  reason: contains not printable characters */
    public boolean f9062WWoWWo;

    /* renamed from: WWoᆑWWoӁᆑ  reason: contains not printable characters */
    public final StringBuilder f9063WWoWWo = new StringBuilder();

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public final LocationManager f9064WWWW;

    /* renamed from: WWၚஊၚ  reason: contains not printable characters */
    public String f9065WW;

    /* renamed from: WoოWo੍ო  reason: contains not printable characters */
    public HandlerThread f9066WoWo;

    static {
        StringFog.f8859WWWWWWWW.getClass();
        f9050WWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{37, -80, 65, -60, -60, 7, 112, -100, 58, -70, 80, -45, -39, 13, 122}, new byte[]{105, -33, 34, -91, -80, 110, 31, -14});
    }

    public LocationService(Context context, VMInstance vMInstance, HALManager hALManager) {
        StringFog.f8859WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_LINK, 29, -19, 19, 46, -85}, new byte[]{121, 85, Byte.MIN_VALUE, 126, 93, -40, 126, -100});
        Locale locale = Locale.US;
        this.f9053WWWWoWWWWo = new SimpleDateFormat(m17835WWWWWWWW, locale);
        this.f9058WWWWWWWW = new SimpleDateFormat(WWWWWWWW.m17835WWWWWWWW(new byte[]{-93, ConstantPoolEntry.CP_NameAndType, 87, 16, -110, -46}, new byte[]{-57, 104, 26, 93, -21, -85, -115, -61}), locale);
        this.f9059WWWWWWWW = new Date();
        this.f9052WWWWoWWWWo = context;
        this.f9051WWWWWWWWWW = vMInstance;
        this.f9054WWWWWWWW = hALManager;
        this.f9064WWWW = (LocationManager) context.getSystemService(WWWWWWWW.m17835WWWWWWWW(new byte[]{-21, -60, -116, 126, -85, 34, -64, 43}, new byte[]{-121, -85, -17, 31, -33, TarConstants.LF_GNUTYPE_LONGLINK, -81, 69}));
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public static double[] m5133WWWWWWWW(String str) {
        try {
            if (TextUtils.isEmpty(str)) {
                return null;
            }
            StringFog.f8859WWWWWWWW.getClass();
            return new double[]{Double.parseDouble(str.split(WWWWWWWW.m17835WWWWWWWW(new byte[]{-27}, new byte[]{-55, -65, 35, -110, -105, 114, 15, 71}))[0]), Double.parseDouble(str.split(WWWWWWWW.m17835WWWWWWWW(new byte[]{0}, new byte[]{44, -101, 2, -109, 92, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 17, 63}))[1])};
        } catch (Throwable th2) {
            th2.printStackTrace();
            return null;
        }
    }

    @Override // com.android.vmcore.event.PermissionEvent.IPermissionResultCallback
    /* renamed from: WWWWo̐WWWWoȄ̐ */
    public final void mo5117WWWWoWWWWo(int[] iArr) {
        this.f9061WWWoWWWo = false;
        boolean z10 = this.f9062WWoWWo;
        String str = f9050WWWW;
        if (z10 && !this.f9056WWWWWWWW) {
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5043WWWWWWWW(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{15, -70, -40, 6, -108, 5, 14, -122, 61, -70, -40, 26, -98, 20, 2, -101, 39, -90, -33, 37, -107, 37, 7, -109, 58, -78, -45, 46, -90, 70, 29, -105, TarConstants.LF_SYMLINK, -89, -45, 57, -109, 70, 3, -99, TarConstants.LF_CONTIG, -76, -62, 35, -108, 8}, new byte[]{84, -43, -74, 74, -5, 102, 111, -14}));
            m5134WWWWWWWW(false);
            return;
        }
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{45, 21, -33, 78, 37, -4, TarConstants.LF_NORMAL, -80, 31, 21, -33, 82, 47, -19, 60, -83, 5, 9, -40, 109, 36, -36, 57, -91, 24, 29, -44, 102, 23, -65, 56, -93, 24, 21, -61, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}, new byte[]{118, 122, -79, 2, 74, -97, 81, -60}));
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Type inference failed for: r1v25, types: [java.lang.Object, com.android.vmcore.event.PermissionEvent] */
    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final void m5134WWWWWWWW(boolean z10) {
        VMInstance vMInstance = this.f9051WWWWWWWWWW;
        String str = vMInstance.f8937WWWoWWWo.f8901WWWoWWWo;
        this.f9065WW = str;
        byte[] bArr = {59, 32, 35, TarConstants.LF_SYMLINK, -127, -55, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 3};
        StringFog.f8859WWWWWWWW.getClass();
        boolean z11 = true;
        if (WWWWWWWW.m17835WWWWWWWW(new byte[]{95, 69, 85, 91, -30, -84, 56, 113, 94, 65, 79, 70, -24, -92, 2}, bArr).equals(str)) {
            String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{71, -49, -35, -117, 104, TarConstants.LF_CHR, 5, -40, 86, -60, -53, -108, 110, 41, 18, -97, 73, -49, -105, -72, 68, 25, 36, -91, 117, -2, -1, -80, 73, 31, 62, -70, 105, -30, -8, -83, 78, 21, 47}, new byte[]{38, -95, -71, -7, 7, 90, 97, -10});
            Context context = this.f9052WWWWoWWWWo;
            int m824WWWWWWWW = AbstractC0211WWWWWWWW.m824WWWWWWWW(context, m17835WWWWWWWW);
            String str2 = f9050WWWW;
            if (m824WWWWWWWW == 0 || AbstractC0211WWWWWWWW.m824WWWWWWWW(context, WWWWWWWW.m17835WWWWWWWW(new byte[]{-17, -111, -9, -45, -37, TarConstants.LF_SYMLINK, 38, -91, -2, -102, -31, -52, -35, 40, TarConstants.LF_LINK, -30, -31, -111, -67, -32, -9, 24, 7, -40, -35, -96, -48, -18, -11, 9, 17, -50, -47, -77, -36, -30, -11, 15, ConstantPoolEntry.CP_InterfaceMethodref, -60, -64}, new byte[]{-114, -1, -109, -95, -76, 91, 66, -117})) == 0) {
                try {
                    this.f9060WWWWWWWW = null;
                    this.f9064WWWW.requestLocationUpdates(10L, 0.0f, new Criteria(), this, this.f9057WWWWWWWW.getLooper());
                    this.f9057WWWWWWWW.post(new Runnable() { // from class: com.android.vmcore.hal.LocationService.1
                        @Override // java.lang.Runnable
                        public final void run() {
                            Location location;
                            Location location2;
                            LocationService locationService = LocationService.this;
                            if (locationService.f9060WWWWWWWW == null) {
                                LocationManager locationManager = locationService.f9064WWWW;
                                try {
                                    StringFog.f8859WWWWWWWW.getClass();
                                    location = locationManager.getLastKnownLocation(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, -61, 93, 32, -23, -62, 37}, new byte[]{34, -90, 41, 87, -122, -80, 78, -102}));
                                    Location lastKnownLocation = locationManager.getLastKnownLocation(WWWWWWWW.m17835WWWWWWWW(new byte[]{102, 14, 74}, new byte[]{1, 126, 57, -86, -20, -2, 8, -124}));
                                    String bestProvider = locationManager.getBestProvider(new Criteria(), true);
                                    if (!TextUtils.isEmpty(bestProvider)) {
                                        location2 = locationManager.getLastKnownLocation(bestProvider);
                                    } else {
                                        location2 = null;
                                    }
                                    if (location2 != null) {
                                        location = location2;
                                    } else if (lastKnownLocation != null) {
                                        location = lastKnownLocation;
                                    }
                                } catch (Throwable unused) {
                                    location = null;
                                }
                                if (location != null) {
                                    locationService.onLocationChanged(location);
                                }
                            }
                            locationService.f9060WWWWWWWW = null;
                            locationService.f9057WWWWWWWW.postDelayed(this, 10000L);
                        }
                    });
                } catch (Throwable th2) {
                    byte[] bArr2 = {94, -13, -112, -110, -123, -17, ConstantPoolEntry.CP_NameAndType, 25};
                    StringFog.f8859WWWWWWWW.getClass();
                    KLog.m5044WWWoWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{5, -105, -1, -63, -15, -114, 126, 109, 25, -93, -61, -49, -91, -118, 116, 122, 59, -125, -28, -5, -22, -127, 44}, bArr2), th2);
                }
                this.f9056WWWWWWWW = z11;
            }
            KLog.m5041WWWWWWWW(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{23, -42, -79, 3, 15, -95, TarConstants.LF_SYMLINK, 35, ConstantPoolEntry.CP_InterfaceMethodref, -30, -115, 13, 65, -32, 46, 56, 108, -62, -69, 34, 22, -87, TarConstants.LF_CHR, 36, 37, -35, -80}, new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, -78, -34, 80, 123, -64, 64, 87}));
            if (vMInstance.f8939WWWoWWWo.m13946WWWoWWWo(PermissionEvent.class) == null && this.f9055WWWWWWWW && z10) {
                ?? obj = new Object();
                obj.f9007WWWWWWWW = new String[]{WWWWWWWW.m17835WWWWWWWW(new byte[]{124, 21, 7, -121, 107, 1, -104, -66, 109, 30, 17, -104, 109, 27, -113, -7, 114, 21, TarConstants.LF_MULTIVOLUME, -76, 71, 43, -71, -61, 78, 36, 32, -70, 69, 58, -81, -43, 66, TarConstants.LF_CONTIG, 44, -74, 69, 60, -75, -33, TarConstants.LF_GNUTYPE_SPARSE}, new byte[]{29, 123, 99, -11, 4, 104, -4, -112}), WWWWWWWW.m17835WWWWWWWW(new byte[]{-57, -23, -74, 74, 116, -62, -4, -62, -42, -30, -96, 85, 114, -40, -21, -123, -55, -23, -4, 121, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -24, -35, -65, -11, -40, -108, 113, 85, -18, -57, -96, -23, -60, -109, 108, 82, -28, -42}, new byte[]{-90, -121, -46, 56, 27, -85, -104, -20})};
                obj.f9006WWWWoWWWWo = this;
                vMInstance.f8939WWWoWWWo.m13942WWWWWWWW(obj);
            }
            z11 = false;
            this.f9056WWWWWWWW = z11;
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{72, -114, -13, -31, TarConstants.LF_GNUTYPE_SPARSE, -16, -41, -100, 94, -108, -16, -6, 105, -25}, new byte[]{61, -3, -106, -109, ConstantPoolEntry.CP_NameAndType, -125, -89, -7}).equals(str)) {
            if (m5133WWWWWWWW(vMInstance.f8937WWWoWWWo.f8891WWWWWWWW) == null) {
                z11 = false;
            } else {
                this.f9057WWWWWWWW.post(new Runnable() { // from class: com.android.vmcore.hal.LocationService.2
                    @Override // java.lang.Runnable
                    public final void run() {
                        LocationService locationService = LocationService.this;
                        double[] m5133WWWWWWWW = LocationService.m5133WWWWWWWW(locationService.f9051WWWWWWWWWW.f8937WWWoWWWo.f8891WWWWWWWW);
                        if (m5133WWWWWWWW != null) {
                            byte[] bArr3 = {TarConstants.LF_FIFO, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_SYMLINK, 9, TarConstants.LF_BLK, 104, 80, -48, 10, TarConstants.LF_MULTIVOLUME, 62, 30, 3, 84, 90, -48, 2, 95, 62, 20, 9};
                            byte[] bArr4 = {99, 43, 87, 123, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 24, TarConstants.LF_DIR, -77};
                            StringFog.f8859WWWWWWWW.getClass();
                            Location location = new Location(WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4));
                            location.setTime(System.currentTimeMillis());
                            location.setLatitude(m5133WWWWWWWW[0]);
                            location.setLongitude(m5133WWWWWWWW[1]);
                            locationService.onLocationChanged(location);
                        }
                        locationService.f9057WWWWWWWW.postDelayed(this, 1000L);
                    }
                });
            }
            this.f9056WWWWWWWW = z11;
        }
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public final void m5135WWWWWWWW() {
        byte[] bArr = {-114, TarConstants.LF_GNUTYPE_LONGNAME, -41, 60, ConstantPoolEntry.CP_InterfaceMethodref, -43, 47, 5, -76, TarConstants.LF_GNUTYPE_LONGLINK, -54, 60, 21, -60};
        byte[] bArr2 = {-43, 63, -93, TarConstants.LF_GNUTYPE_SPARSE, 123, -103, 64, 102};
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(f9050WWWW, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        this.f9062WWoWWo = false;
        if (this.f9056WWWWWWWW) {
            m5136WWWoWWWo();
        }
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final void m5136WWWoWWWo() {
        boolean z10 = false;
        StringFog.f8859WWWWWWWW.getClass();
        if (WWWWWWWW.m17835WWWWWWWW(new byte[]{96, 57, 57, -21, 70, -43, 123, -72, 97, 61, 35, -10, TarConstants.LF_GNUTYPE_LONGNAME, -35, 65}, new byte[]{4, 92, 79, -126, 37, -80, 36, -54}).equals(this.f9065WW)) {
            try {
                this.f9057WWWWWWWW.removeCallbacksAndMessages(null);
                this.f9064WWWW.removeUpdates(this);
                z10 = true;
            } catch (Throwable th2) {
                byte[] bArr = {26, 5, 110, 3, 39, -110, 93, 42, 17, TarConstants.LF_SYMLINK, 92, 112, TarConstants.LF_FIFO, -123, 78, 8, TarConstants.LF_LINK, 21, 104, 63, 61, -35};
                byte[] bArr2 = {65, 97, 1, 80, TarConstants.LF_GNUTYPE_SPARSE, -3, 45, 109};
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5044WWWoWWWo(f9050WWWW, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), th2);
            }
            this.f9056WWWWWWWW = true ^ z10;
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-84, -38, 15, -56, 0, 64, 81, -6, -70, -64, ConstantPoolEntry.CP_NameAndType, -45, 58, 87}, new byte[]{-39, -87, 106, -70, 95, TarConstants.LF_CHR, 33, -97}).equals(this.f9065WW)) {
            this.f9057WWWWWWWW.removeCallbacksAndMessages(null);
            this.f9056WWWWWWWW = false;
        }
    }

    @Override // android.location.LocationListener
    public final void onLocationChanged(Location location) {
        String m5049WWWWWWWW;
        String m5049WWWWWWWW2;
        String m5049WWWWWWWW3;
        String m5049WWWWWWWW4;
        StringBuilder sb2 = this.f9063WWoWWo;
        try {
            this.f9060WWWWWWWW = location;
            sb2.setLength(0);
            Date date = this.f9059WWWWWWWW;
            date.setTime(location.getTime());
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-57, 74, 3, 42, -84, -109}, new byte[]{-29, 13, TarConstants.LF_GNUTYPE_SPARSE, 109, -21, -46, 46, -123}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-32}, new byte[]{-52, -50, ConstantPoolEntry.CP_NameAndType, -91, -87, -87, 94, 38}));
            SimpleDateFormat simpleDateFormat = this.f9053WWWWoWWWWo;
            sb2.append(simpleDateFormat.format(date));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{35}, new byte[]{15, -19, 44, -94, 60, 107, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -125}));
            double abs = Math.abs(location.getLatitude());
            int i10 = (int) abs;
            int i11 = i10 * 100;
            Locale locale = Locale.US;
            sb2.append(String.format(locale, StringFog.m5049WWWWWWWW(new byte[]{-111, 79, -75, 99}, new byte[]{-76, 97, -127, 5, -87, 25, 20, -32}), Double.valueOf(i11 + ((abs - i10) * 60.0d))));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{21}, new byte[]{57, 90, -24, 10, 89, TarConstants.LF_BLK, -24, 65}));
            if (location.getLatitude() >= FirebaseRemoteConfig.DEFAULT_VALUE_FOR_DOUBLE) {
                m5049WWWWWWWW = StringFog.m5049WWWWWWWW(new byte[]{-127}, new byte[]{-49, -7, -10, 7, 125, 5, 92, -13});
            } else {
                m5049WWWWWWWW = StringFog.m5049WWWWWWWW(new byte[]{122}, new byte[]{41, Byte.MAX_VALUE, -86, 86, -30, -4, -64, -85});
            }
            sb2.append(m5049WWWWWWWW);
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{102}, new byte[]{74, 91, 94, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 22, 106, 3, 56}));
            double abs2 = Math.abs(location.getLongitude());
            int i12 = (int) abs2;
            sb2.append(String.format(locale, StringFog.m5049WWWWWWWW(new byte[]{-21, 45, 67, 126}, new byte[]{-50, 3, 119, 24, 13, 64, 62, 111}), Double.valueOf((i12 * 100) + ((abs2 - i12) * 60.0d))));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-31}, new byte[]{-51, 106, TarConstants.LF_SYMLINK, -31, -93, 113, -9, -107}));
            int i13 = 1;
            if (location.getLongitude() >= FirebaseRemoteConfig.DEFAULT_VALUE_FOR_DOUBLE) {
                m5049WWWWWWWW2 = StringFog.m5049WWWWWWWW(new byte[]{6}, new byte[]{67, -112, ConstantPoolEntry.CP_NameAndType, 116, -95, 10, -91, 22});
                i13 = 1;
            } else {
                m5049WWWWWWWW2 = StringFog.m5049WWWWWWWW(new byte[]{1}, new byte[]{86, -109, 95, 91, -79, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -42, 21});
            }
            sb2.append(m5049WWWWWWWW2);
            byte[] bArr = new byte[i13];
            bArr[0] = 57;
            sb2.append(StringFog.m5049WWWWWWWW(bArr, new byte[]{21, 106, TarConstants.LF_CONTIG, -90, -84, -85, -90, Byte.MIN_VALUE}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-86, 62}, new byte[]{-101, 18, 80, -122, -114, TarConstants.LF_DIR, -54, -125}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{92, 97}, new byte[]{106, TarConstants.LF_MULTIVOLUME, -89, -13, 95, 119, -74, 107}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{Byte.MAX_VALUE, 43, -47, -2}, new byte[]{78, 5, -28, -46, 94, -14, -104, -123}));
            sb2.append(String.format(locale, StringFog.m5049WWWWWWWW(new byte[]{-51, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 7, -36}, new byte[]{-24, 73, TarConstants.LF_FIFO, -70, -62, -91, 56, 33}), Double.valueOf(location.getAltitude())));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{20}, new byte[]{56, -21, -81, -87, -95, ConstantPoolEntry.CP_InterfaceMethodref, TarConstants.LF_GNUTYPE_LONGLINK, 38}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{115}, new byte[]{62, -10, 8, -56, 22, -4, 94, -1}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-77}, new byte[]{-97, 38, -26, TarConstants.LF_NORMAL, Byte.MIN_VALUE, -123, 28, 63}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{90, 65, -64}, new byte[]{106, 111, -20, 6, -76, -8, 79, 124}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-4, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}, new byte[]{-79, TarConstants.LF_GNUTYPE_LONGLINK, -87, -78, 125, 106, -81, -79}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_MULTIVOLUME}, new byte[]{97, 115, 72, -125, -48, -72, -17, 14}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-7}, new byte[]{-43, 97, 81, 64, -46, -45, -60, -82}));
            sb2.append("\n");
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-75, 113, 109, 37, 122, 0}, new byte[]{-111, TarConstants.LF_FIFO, 61, 119, TarConstants.LF_CONTIG, 67, 69, -68}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{71}, new byte[]{107, -127, 97, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -17, -46, -122, 95}));
            sb2.append(simpleDateFormat.format(date));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-17}, new byte[]{-61, -113, -40, 78, -17, 8, 82, -38}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_LINK}, new byte[]{112, -50, 115, -119, -23, 91, -120, 41}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-113}, new byte[]{-93, -84, -68, -47, 7, -1, -14, -68}));
            double abs3 = Math.abs(location.getLatitude());
            int i14 = (int) abs3;
            sb2.append(String.format(locale, StringFog.m5049WWWWWWWW(new byte[]{-15, 114, 42, 42}, new byte[]{-44, 92, 30, TarConstants.LF_GNUTYPE_LONGNAME, 68, -118, 56, 41}), Double.valueOf((i14 * 100) + ((abs3 - i14) * 60.0d))));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{94}, new byte[]{114, -89, 39, -120, 66, 34, 87, 82}));
            if (location.getLatitude() >= FirebaseRemoteConfig.DEFAULT_VALUE_FOR_DOUBLE) {
                m5049WWWWWWWW3 = StringFog.m5049WWWWWWWW(new byte[]{-101}, new byte[]{-43, 26, TarConstants.LF_CONTIG, -63, -10, 98, -120, -4});
            } else {
                m5049WWWWWWWW3 = StringFog.m5049WWWWWWWW(new byte[]{-101}, new byte[]{-56, -97, TarConstants.LF_CONTIG, TarConstants.LF_BLK, 78, 112, 80, 47});
            }
            sb2.append(m5049WWWWWWWW3);
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-23}, new byte[]{-59, 59, -112, 84, 46, 70, 37, 96}));
            double abs4 = Math.abs(location.getLongitude());
            int i15 = (int) abs4;
            sb2.append(String.format(locale, StringFog.m5049WWWWWWWW(new byte[]{-121, -57, -46, -6}, new byte[]{-94, -23, -26, -100, 126, 116, 32, -86}), Double.valueOf((i15 * 100) + ((abs4 - i15) * 60.0d))));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{25}, new byte[]{TarConstants.LF_DIR, 18, -77, 111, 32, 72, -71, -76}));
            if (location.getLongitude() >= FirebaseRemoteConfig.DEFAULT_VALUE_FOR_DOUBLE) {
                m5049WWWWWWWW4 = StringFog.m5049WWWWWWWW(new byte[]{74}, new byte[]{15, -81, 43, -106, -84, 63, 0, 101});
            } else {
                m5049WWWWWWWW4 = StringFog.m5049WWWWWWWW(new byte[]{119}, new byte[]{32, -55, 72, -62, -33, -117, -114, 100});
            }
            sb2.append(m5049WWWWWWWW4);
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-58}, new byte[]{-22, 92, -102, -48, -125, -29, 60, 64}));
            sb2.append(String.format(locale, StringFog.m5049WWWWWWWW(new byte[]{-29, -82, 70, 72}, new byte[]{-58, Byte.MIN_VALUE, 119, 46, 92, -104, -63, 16}), Double.valueOf(location.getSpeed() * 0.514d)));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-66}, new byte[]{-110, 106, 95, -8, -59, -118, 78, TarConstants.LF_FIFO}));
            sb2.append(String.format(locale, StringFog.m5049WWWWWWWW(new byte[]{45, -98, -77, 19}, new byte[]{8, -80, -126, 117, 40, -38, -9, 41}), Float.valueOf(location.getBearing())));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{82}, new byte[]{126, 21, 81, -54, -54, 72, -79, -77}));
            sb2.append(this.f9058WWWWWWWW.format(date));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-91}, new byte[]{-119, -93, 35, -52, 125, -75, TarConstants.LF_BLK, 31}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{0, 114, -77}, new byte[]{TarConstants.LF_NORMAL, 92, -97, -11, 9, 95, -100, 106}));
            sb2.append(StringFog.m5049WWWWWWWW(new byte[]{-90, -23}, new byte[]{-15, -59, -66, 26, -65, 19, -73, -29}));
            sb2.append("\n");
            this.f9054WWWWWWWW.GPSNmeaChanged(sb2.toString());
        } catch (Throwable th2) {
            KLog.m5044WWWoWWWo(f9050WWWW, StringFog.m5049WWWWWWWW(new byte[]{73, 28, -81, -30, 38, -117, -20, 0, 123, 28, -81, -19, 33, -119, -29, 19, 119, 23, -100, -114, 44, -112, -18, 17, 98, 7, -88, -63, 39, -56}, new byte[]{18, 115, -63, -82, 73, -24, -115, 116}), th2);
        }
    }

    @Override // android.location.LocationListener
    public final void onProviderDisabled(String str) {
    }

    @Override // android.location.LocationListener
    public final void onProviderEnabled(String str) {
    }

    @Override // android.location.LocationListener
    public final void onStatusChanged(String str, int i10, Bundle bundle) {
    }
}
