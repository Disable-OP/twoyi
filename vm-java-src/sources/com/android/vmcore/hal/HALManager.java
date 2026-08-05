package com.android.vmcore.hal;

import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.SharedPreferences;
import android.graphics.Rect;
import android.hardware.Camera;
import android.hardware.Sensor;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.SystemClock;
import android.text.TextUtils;
import android.util.SparseIntArray;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmcore.KLog;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import com.android.vmcore.hal.CameraService;
import com.android.vmcore.hal.phone.CallPdu;
import com.android.vmcore.hal.phone.Types;
import com.google.android.gms.internal.ads.pr0;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.util.ArrayList;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p001WWWWoWWWWo.RunnableC0054WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class HALManager {
    private final BatteryService mBatteryService;
    private final CameraService mCameraService;
    private final HWControlService mHWControlService;
    private final LocationService mLocationService;
    private long mNativePtr;
    private final PhoneService mPhoneService;
    private final SensorService mSensorService;
    private final WiFiService mWiFiService;

    public HALManager(Context context, VMInstance vMInstance) {
        this.mNativePtr = nativeSetup(vMInstance.f8937WWWoWWWo.f8866WWWWWWWW);
        this.mCameraService = new CameraService(context, vMInstance, this);
        this.mLocationService = new LocationService(context, vMInstance, this);
        this.mHWControlService = new HWControlService(context);
        this.mWiFiService = new WiFiService(context, vMInstance, this);
        this.mSensorService = new SensorService(context, vMInstance, this);
        this.mPhoneService = new PhoneService(vMInstance, this);
        this.mBatteryService = new BatteryService(context, vMInstance);
    }

    private int CameraConnect(String str) {
        this.mCameraService.m5123WWWWWWWW(str);
        return 0;
    }

    private int CameraDisconnect(String str) {
        this.mCameraService.m5122WWWWoWWWWo(str);
        return 0;
    }

    private int CameraFlash(String str, String str2) {
        this.mCameraService.getClass();
        return 0;
    }

    private int CameraFocus(String str, String str2, int i10, int i11, int i12, int i13, int i14) {
        Camera camera = this.mCameraService.f9031WWWWWWWW;
        String str3 = CameraService.f9028WWWW;
        if (camera == null) {
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5040WWWWoWWWWo(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-111, 124, -42, -58, -108, -63, 3, TarConstants.LF_SYMLINK, -91, 124, -62, -40, -84, -109, ConstantPoolEntry.CP_NameAndType, 27, -22, 124, -42, -58, -108, -63, 3}, new byte[]{-54, 31, -73, -85, -15, -77, 98, 116}));
            return 0;
        }
        try {
            camera.cancelAutoFocus();
            Camera.Parameters parameters = camera.getParameters();
            if (parameters.getMaxNumFocusAreas() > 0) {
                ArrayList arrayList = new ArrayList();
                arrayList.add(new Camera.Area(new Rect(i10, i11, i12, i13), i14));
                parameters.setFocusAreas(arrayList);
            }
            if (!TextUtils.isEmpty(str2)) {
                parameters.setFocusMode(str2);
            }
            byte[] bArr = {-98, -17, 57, TarConstants.LF_CHR, 106, 82, -101, -30};
            StringFog.f8859WWWWWWWW.getClass();
            if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-8, -122, 65, 86, 14}, bArr).equals(parameters.getFocusMode())) {
                parameters.setFocusAreas(null);
            }
            camera.setParameters(parameters);
            camera.autoFocus(null);
            return 0;
        } catch (Throwable th2) {
            byte[] bArr2 = {81, 113, -92, 21, 125, -127, 45, 15, 101, 113, -80, ConstantPoolEntry.CP_InterfaceMethodref, 69, -45, 41, TarConstants.LF_LINK, 105, 119, -75, ConstantPoolEntry.CP_NameAndType, 113, -100, 34, 115, 42};
            byte[] bArr3 = {10, 18, -59, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 24, -13, TarConstants.LF_GNUTYPE_LONGNAME, 73};
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5044WWWoWWWo(str3, WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3), th2);
            return 0;
        }
    }

    private int CameraFrame(String str, float f10, float f11, float f12, float f13, int i10, String str2) {
        Camera camera = this.mCameraService.f9031WWWWWWWW;
        String str3 = CameraService.f9028WWWW;
        if (camera == null) {
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5040WWWWoWWWWo(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_LINK, 16, -116, 2, 63, 100, -35, -99, 24, 18, Byte.MIN_VALUE, 10, 7, TarConstants.LF_FIFO, -46, -76, 74, 16, -116, 2, 63, 100, -35}, new byte[]{106, 115, -19, 111, 90, 22, -68, -37}));
            return 0;
        }
        try {
            Camera.Parameters parameters = camera.getParameters();
            if (!parameters.isZoomSupported() && !parameters.isSmoothZoomSupported()) {
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5040WWWWoWWWWo(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-14, 119, 39, 42, 59, 3, 38, 93, -37, 117, 43, 34, 3, 81, 61, 116, -58, 121, 102, 41, TarConstants.LF_LINK, 5, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 104, -36, 100, TarConstants.LF_FIFO, 40, 44, 5, 34, Byte.MAX_VALUE}, new byte[]{-87, 20, 70, 71, 94, 113, 71, 27}));
                return 0;
            }
            parameters.setZoom(i10);
            camera.setParameters(parameters);
            return 0;
        } catch (Throwable th2) {
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5044WWWoWWWo(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{-51, -46, -53, 119, -117, -39, -83, -74, -28, -48, -57, Byte.MAX_VALUE, -77, -117, -87, -120, -11, -44, -38, 110, -121, -60, -94, -54, -74}, new byte[]{-106, -79, -86, 26, -18, -85, -52, -16}), th2);
            return 0;
        }
    }

    private String CameraList() {
        CameraService cameraService = this.mCameraService;
        boolean m5085WWoWWo = cameraService.f9029WWWWoWWWWo.m5085WWoWWo();
        String str = CameraService.f9028WWWW;
        if (!m5085WWoWWo) {
            byte[] bArr = {40, -77, 91, 107, 19, 111, 62, -16, 22, -83, TarConstants.LF_GNUTYPE_SPARSE, 69, 71, 79, 62, -16, 22, -83, TarConstants.LF_GNUTYPE_SPARSE, 56, 9, 67, 43, -67, 22, -79, TarConstants.LF_GNUTYPE_SPARSE, 122, ConstantPoolEntry.CP_InterfaceMethodref, 73, 59};
            byte[] bArr2 = {115, -33, TarConstants.LF_SYMLINK, 24, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 44, 95, -99};
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5043WWWWWWWW(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
            return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
        }
        byte[] bArr3 = {-91, 71, -26, TarConstants.LF_LINK, TarConstants.LF_DIR, -73, -25, 91};
        StringFog.f8859WWWWWWWW.getClass();
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-58, 38, -117, 84, 71, -42}, bArr3);
        SharedPreferences sharedPreferences = cameraService.f9030WWWWWWWW.getSharedPreferences(m17835WWWWWWWW, 0);
        String string = sharedPreferences.getString(WWWWWWWW.m17835WWWWWWWW(new byte[]{18, -14, -77, TarConstants.LF_GNUTYPE_SPARSE, -122, -120, -69, TarConstants.LF_FIFO, 24, -32, -86}, new byte[]{113, -109, -34, TarConstants.LF_FIFO, -12, -23, -28, 90}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        int i10 = sharedPreferences.getInt(WWWWWWWW.m17835WWWWWWWW(new byte[]{97, -10, -29, -89, -102, 34, 13}, new byte[]{23, -109, -111, -44, -13, TarConstants.LF_MULTIVOLUME, 99, -92}), 0);
        if (TextUtils.isEmpty(string) || i10 != 1) {
            StringBuilder sb2 = new StringBuilder();
            int numberOfCameras = Camera.getNumberOfCameras();
            for (int i11 = 0; i11 < numberOfCameras; i11++) {
                try {
                    sb2.append(CameraService.m5121WWWoWWWo(i11));
                } catch (Throwable th2) {
                    byte[] bArr4 = {14, -47, 13, -85, -114, 118, -76, -94, TarConstants.LF_NORMAL, -49, 5, -123, -38, 80, -83, -84, TarConstants.LF_NORMAL, -51, 16, -79, -107, 91, -11};
                    byte[] bArr5 = {85, -67, 100, -40, -6, TarConstants.LF_DIR, -43, -49};
                    StringFog.f8859WWWWWWWW.getClass();
                    KLog.m5044WWWoWWWo(str, WWWWWWWW.m17835WWWWWWWW(bArr4, bArr5), th2);
                }
            }
            string = sb2.toString();
            SharedPreferences.Editor edit = sharedPreferences.edit();
            StringFog.f8859WWWWWWWW.getClass();
            edit.putString(WWWWWWWW.m17835WWWWWWWW(new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, 3, -38, -65, -63, -103, TarConstants.LF_BLK, -39, 1, 17, -61}, new byte[]{104, 98, -73, -38, -77, -8, 107, -75}), string).putInt(WWWWWWWW.m17835WWWWWWWW(new byte[]{-108, 104, -45, 105, -113, TarConstants.LF_FIFO, -120}, new byte[]{-30, 13, -95, 26, -26, 89, -26, -123}), 1).apply();
        }
        StringBuilder sb3 = new StringBuilder();
        pr0.m9002WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-123, -51, -96, -71, 7, -12, -74, -64, -69, -45, -88, -105, TarConstants.LF_GNUTYPE_SPARSE, -44, -74, -64, -69, -45, -88, -22, 16, -40, -94, -61, -86, -127}, new byte[]{-34, -95, -55, -54, 115, -73, -41, -83}, sb3);
        sb3.append(string.split("\n").length - 1);
        KLog.m5043WWWWWWWW(str, sb3.toString());
        return string;
    }

    private int CameraStart(String str, int i10, int i11, int i12) {
        return this.mCameraService.m5124WWWWWWWW(i10, i11, i12, str);
    }

    private int CameraStop(String str) {
        this.mCameraService.m5125WWWWWWWW(str);
        return 0;
    }

    private boolean CheckSensorsSupport(int i10) {
        SensorService sensorService = this.mSensorService;
        sensorService.getClass();
        if (i10 >= 0 && i10 < 12 && (sensorService.f9102WWWWWWWW[i10] & 1) == 1) {
            return true;
        }
        return false;
    }

    private void DisableSensors(int i10) {
        SensorService sensorService = this.mSensorService;
        sensorService.getClass();
        if (i10 >= 0 && i10 < 12) {
            int[] iArr = sensorService.f9102WWWWWWWW;
            iArr[i10] = iArr[i10] & (-5);
            Sensor sensor = sensorService.f9101WWWWWWWW[i10];
            if (sensor != null) {
                sensorService.f9106WWWoWWWo.unregisterListener(sensorService, sensor);
            }
        }
    }

    private void EnableSensors(int i10) {
        Sensor sensor;
        SensorService sensorService = this.mSensorService;
        sensorService.getClass();
        if (i10 >= 0 && i10 < 12) {
            int[] iArr = sensorService.f9102WWWWWWWW;
            int i11 = iArr[i10];
            if ((i11 & 2) == 2) {
                iArr[i10] = i11 | 4;
                if (sensorService.f9104WWWWWWWW && (sensor = sensorService.f9101WWWWWWWW[i10]) != null) {
                    int i12 = sensorService.f9108WWoWWo[i10];
                    if (i12 == 0) {
                        i12 = 1;
                    }
                    sensorService.f9106WWWoWWWo.registerListener(sensorService, sensor, i12, sensorService.f9103WWWWWWWW[i10]);
                }
            }
        }
    }

    /* JADX WARN: Removed duplicated region for block: B:188:0x0767  */
    /* JADX WARN: Removed duplicated region for block: B:199:0x076f A[SYNTHETIC] */
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    private String ExecPhoneCommand(int i10, String str) {
        String m17835WWWWWWWW;
        int i11;
        int i12;
        Types.Cdma2000RegistrationInfo cdma2000RegistrationInfo;
        boolean z10;
        int i13 = 3;
        int i14 = 6;
        int i15 = 7;
        int i16 = 5;
        PhoneService phoneService = this.mPhoneService;
        phoneService.getClass();
        StringBuilder sb2 = new StringBuilder();
        byte[] bArr = {ConstantPoolEntry.CP_NameAndType};
        byte[] bArr2 = {TarConstants.LF_CONTIG, -86, 87, -59, 81, -88, -63, -120};
        StringFog.f8859WWWWWWWW.getClass();
        String[] split = str.split(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
        int length = split.length;
        int i17 = 0;
        while (i17 < length) {
            String str2 = split[i17];
            byte[] bArr3 = new byte[i16];
            // fill-array-data instruction
            bArr3[0] = -87;
            bArr3[1] = -117;
            bArr3[2] = -19;
            bArr3[3] = -30;
            bArr3[4] = 85;
            WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
            boolean m3444WWWWWWWW = AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, bArr3, new byte[]{-126, -56, -65, -89, 18, 43, 19, Byte.MIN_VALUE}, str2);
            String str3 = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
            if (m3444WWWWWWWW) {
                byte[] bArr4 = new byte[i15];
                // fill-array-data instruction
                bArr4[0] = -88;
                bArr4[1] = 16;
                bArr4[2] = 17;
                bArr4[3] = -22;
                bArr4[4] = -43;
                bArr4[5] = -3;
                bArr4[6] = 50;
                if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(wwwwwwww, bArr4, new byte[]{-125, TarConstants.LF_GNUTYPE_SPARSE, 67, -81, -110, -64, 13, -119}, str2)) {
                    str3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-82, -29, 57, -127, -51, 98, 94, -80, -75, -115, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -24, -86, 105, TarConstants.LF_GNUTYPE_LONGNAME, -96, -84}, new byte[]{-123, -96, 107, -60, -118, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 126, -104});
                } else {
                    byte[] bArr5 = new byte[i14];
                    // fill-array-data instruction
                    bArr5[0] = 29;
                    bArr5[1] = 57;
                    bArr5[2] = 61;
                    bArr5[3] = -95;
                    bArr5[4] = 18;
                    bArr5[5] = -1;
                    if (str2.startsWith(WWWWWWWW.m17835WWWWWWWW(bArr5, new byte[]{TarConstants.LF_FIFO, 122, 111, -28, 85, -62, -54, 1}))) {
                        try {
                            byte[] bArr6 = new byte[i14];
                            // fill-array-data instruction
                            bArr6[0] = -95;
                            bArr6[1] = -98;
                            bArr6[2] = 103;
                            bArr6[3] = 2;
                            bArr6[4] = -5;
                            bArr6[5] = 34;
                            i11 = Integer.parseInt(str2.substring(WWWWWWWW.m17835WWWWWWWW(bArr6, new byte[]{-118, -35, TarConstants.LF_DIR, 71, -68, 31, 25, -69}).length()));
                        } catch (Exception unused) {
                            i11 = -1;
                        }
                        if ((i11 >= 0 && i11 <= i13) || i11 == 128) {
                            phoneService.f9071WWWWWWWW = i11;
                        } else {
                            StringFog.f8859WWWWWWWW.getClass();
                            str3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-23, TarConstants.LF_GNUTYPE_LONGLINK, 109, 92, -106, TarConstants.LF_GNUTYPE_LONGLINK, TarConstants.LF_LINK, TarConstants.LF_GNUTYPE_SPARSE, -115, 90, 26, 57, -125, 62}, new byte[]{-62, 8, 32, 25, -74, 14, 99, 1});
                        }
                    } else {
                        byte[] bArr7 = new byte[i14];
                        // fill-array-data instruction
                        bArr7[0] = -67;
                        bArr7[1] = -39;
                        bArr7[2] = 32;
                        bArr7[3] = -124;
                        bArr7[4] = 65;
                        bArr7[5] = 20;
                        if (str2.equals(WWWWWWWW.m17835WWWWWWWW(bArr7, new byte[]{-106, -102, 114, -63, 6, 43, -60, -111}))) {
                            int m5164WWWWoWWWWo = phoneService.m5164WWWWoWWWWo(true);
                            int m5174WWWWWWWW = phoneService.m5174WWWWWWWW(m5164WWWWoWWWWo, true);
                            if (m5174WWWWWWWW == 0) {
                                m5164WWWWoWWWWo = 0;
                            }
                            Object m5173WWWWWWWW = phoneService.m5173WWWWWWWW(m5164WWWWoWWWWo);
                            Types.RegStateResult regStateResult = new Types.RegStateResult();
                            regStateResult.f9224WWWWWWWW = m5174WWWWWWWW;
                            regStateResult.f9223WWWWoWWWWo = m5164WWWWoWWWWo;
                            regStateResult.f9227WWWoWWWo = m5173WWWWWWWW;
                            regStateResult.f9225WWWWWWWW = FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
                            if (PhoneService.m5161WWWW(m5164WWWWoWWWWo) == 1) {
                                Types.Cdma2000RegistrationInfo cdma2000RegistrationInfo2 = new Types.Cdma2000RegistrationInfo();
                                if (m5164WWWWoWWWWo != 4 && m5164WWWWoWWWWo != 5 && m5164WWWWoWWWWo != i14) {
                                    z10 = true;
                                } else {
                                    z10 = false;
                                }
                                cdma2000RegistrationInfo2.f9162WWWWWWWW = z10;
                                if (m5174WWWWWWWW == 1) {
                                    cdma2000RegistrationInfo2.f9161WWWWoWWWWo = 1;
                                    cdma2000RegistrationInfo2.f9164WWWoWWWo = 1;
                                    cdma2000RegistrationInfo2.f9163WWWWWWWW = 1;
                                } else {
                                    cdma2000RegistrationInfo2.f9161WWWWoWWWWo = 0;
                                    cdma2000RegistrationInfo2.f9164WWWoWWWo = 0;
                                    cdma2000RegistrationInfo2.f9163WWWWWWWW = 0;
                                }
                                regStateResult.f9226WWWWWWWW = cdma2000RegistrationInfo2;
                            }
                            int i18 = phoneService.f9071WWWWWWWW;
                            if (i18 != 0 && i18 != 1) {
                                if (i18 != 2 && i18 != i13) {
                                    StringBuilder sb3 = new StringBuilder();
                                    sb3.append(regStateResult.f9224WWWWWWWW);
                                    sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-46}, new byte[]{-2, 24, 107, -75, 27, -57, 18, -84}));
                                    sb3.append(regStateResult.f9223WWWWoWWWWo);
                                    sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-48}, new byte[]{-4, -1, -8, -56, 115, -44, 29, -53}));
                                    if (PhoneService.m5161WWWW(regStateResult.f9223WWWWoWWWWo) == 1 && (cdma2000RegistrationInfo = regStateResult.f9226WWWWWWWW) != null) {
                                        sb3.append(cdma2000RegistrationInfo.f9162WWWWWWWW ? 1 : 0);
                                        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-63}, new byte[]{-19, 108, -67, 85, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 44, 96, -23}));
                                        sb3.append(regStateResult.f9226WWWWWWWW.f9161WWWWoWWWWo);
                                        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{42}, new byte[]{6, TarConstants.LF_PAX_EXTENDED_HEADER_UC, Byte.MIN_VALUE, -55, 35, 34, -13, TarConstants.LF_DIR}));
                                        sb3.append(regStateResult.f9226WWWWWWWW.f9164WWWoWWWo);
                                        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-23}, new byte[]{-59, 59, -56, 27, -40, -68, 18, -127}));
                                        sb3.append(regStateResult.f9226WWWWWWWW.f9163WWWWWWWW);
                                        i12 = 0;
                                        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{45}, new byte[]{1, 35, -47, -91, -61, ConstantPoolEntry.CP_NameAndType, TarConstants.LF_FIFO, 37}));
                                    } else {
                                        i12 = 0;
                                        sb3.append(0);
                                        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-44}, new byte[]{-8, 119, -115, -113, -16, -67, TarConstants.LF_NORMAL, -71}));
                                        sb3.append(-1);
                                        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{91}, new byte[]{119, 87, 16, 5, 14, 81, -68, -121}));
                                        sb3.append(-1);
                                        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{80}, new byte[]{124, 84, -50, 33, -75, 107, 123, -92}));
                                        sb3.append(-1);
                                        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{126}, new byte[]{82, -105, 94, -93, -29, TarConstants.LF_MULTIVOLUME, -25, TarConstants.LF_MULTIVOLUME}));
                                    }
                                    sb3.append(i12);
                                    byte[] bArr8 = new byte[1];
                                    bArr8[i12] = -96;
                                    sb3.append(WWWWWWWW.m17835WWWWWWWW(bArr8, new byte[]{-116, -113, TarConstants.LF_SYMLINK, 118, ConstantPoolEntry.CP_InterfaceMethodref, 80, TarConstants.LF_DIR, 21}));
                                    sb3.append(PhoneService.m5143WWWWWWWW(regStateResult.f9227WWWoWWWo));
                                    if (!TextUtils.isEmpty(regStateResult.f9225WWWWWWWW)) {
                                        sb3.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-39}, new byte[]{-11, 1, 57, -6, 105, 61, 31, 82}));
                                        sb3.append(regStateResult.f9225WWWWWWWW);
                                    } else {
                                        pr0.m9009WWWoWWWo(new byte[]{-92}, new byte[]{-120, -15, 6, 40, 26, 26, -11, -19}, sb3, FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
                                    }
                                    str3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{95, -106, 114, -40, 68, 73, -75}, new byte[]{116, -43, 32, -99, 3, 115, -107, 95}) + phoneService.f9071WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{34}, new byte[]{14, -123, 60, -42, -113, 60, -79, -69}) + sb3.toString();
                                } else {
                                    str3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{94, 59, 105, -38, 68, -106, -71}, new byte[]{117, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 59, -97, 3, -84, -103, -13}) + phoneService.f9071WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{-90}, new byte[]{-118, 117, 1, 68, 106, -17, -80, -90}) + regStateResult.f9224WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{95, 123, -14, 108, -87, 92, -123, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 21, 96, -93, 108, -87, 92, -123, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 21, 42, -72}, new byte[]{115, TarConstants.LF_GNUTYPE_LONGNAME, -108, 10, -49, 58, -29, 62}) + regStateResult.f9223WWWWoWWWWo;
                                }
                            } else {
                                str3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{3, 6, 3, -110, -97, TarConstants.LF_BLK, -76}, new byte[]{40, 69, 81, -41, -40, 14, -108, 31}) + phoneService.f9071WWWWWWWW + WWWWWWWW.m17835WWWWWWWW(new byte[]{58}, new byte[]{22, -39, -106, -69, -29, 22, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -122}) + regStateResult.f9224WWWWWWWW;
                            }
                        } else {
                            str3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{4, 37, 114, 105, -118, -3, 104, 63, 96, TarConstants.LF_BLK, 5, ConstantPoolEntry.CP_NameAndType, -98}, new byte[]{47, 102, 63, 44, -86, -72, 58, 109});
                        }
                    }
                }
            } else {
                byte[] bArr9 = new byte[i14];
                // fill-array-data instruction
                bArr9[0] = -105;
                bArr9[1] = -24;
                bArr9[2] = -42;
                bArr9[3] = -64;
                bArr9[4] = -89;
                bArr9[5] = 17;
                if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, bArr9, new byte[]{-68, -85, -111, -110, -30, 86, 64, 25}, str2)) {
                    str3 = phoneService.m5182WWWW(str2);
                } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-25, -71, -113, 118}, new byte[]{-52, -6, -36, 39, -125, -127, 7, 45}, str2)) {
                    str3 = phoneService.m5170WWWWWWWW(str2);
                } else {
                    if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{89, -83, -94, -114, 57}, new byte[]{114, -18, -10, -53, 122, Byte.MIN_VALUE, -7, 91}, str2)) {
                        str3 = phoneService.m5171WWWWWWWW(str2);
                    } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{99, 100, -70, 102, -63}, new byte[]{72, 39, -7, TarConstants.LF_DIR, -110, -78, 17, -77}, str2)) {
                        byte[] bArr10 = new byte[i14];
                        // fill-array-data instruction
                        bArr10[0] = 114;
                        bArr10[1] = 113;
                        bArr10[2] = 75;
                        bArr10[3] = -79;
                        bArr10[4] = -92;
                        bArr10[5] = 108;
                        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(wwwwwwww, bArr10, new byte[]{89, TarConstants.LF_SYMLINK, 8, -30, -9, TarConstants.LF_GNUTYPE_SPARSE, -5, -75}, str2)) {
                            str3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{85, 100, 92, -81, -50, Byte.MIN_VALUE, 84, -92}, new byte[]{126, 39, 31, -4, -99, -70, 116, -108});
                        } else {
                            byte[] bArr11 = new byte[i14];
                            // fill-array-data instruction
                            bArr11[0] = -34;
                            bArr11[1] = 38;
                            bArr11[2] = 37;
                            bArr11[3] = -3;
                            bArr11[4] = -23;
                            bArr11[5] = -127;
                            if (!str2.startsWith(WWWWWWWW.m17835WWWWWWWW(bArr11, new byte[]{-11, 101, 102, -82, -70, -68, -57, 47}))) {
                                str3 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-110, 69, -56, TarConstants.LF_GNUTYPE_SPARSE, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -101, 74, TarConstants.LF_CHR, -10, 84, -65, TarConstants.LF_FIFO, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{-71, 6, -123, 22, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -34, 24, 97});
                            }
                        }
                    } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-116, 23, -122, 104, 74}, new byte[]{-89, 84, -55, 56, 25, 25, -50, 100}, str2)) {
                        str3 = phoneService.m5180WWoWWo(str2);
                    } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{TarConstants.LF_CONTIG, 96, TarConstants.LF_CONTIG, 38, -49}, new byte[]{28, 35, 112, 107, -99, -102, TarConstants.LF_CHR, TarConstants.LF_NORMAL}, str2)) {
                        str3 = phoneService.m5177WWWoWWWo(str2);
                    } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-11, 37, -61, -60, -95}, new byte[]{-34, 102, -124, -105, -17, -107, 47, 97}, str2)) {
                        str3 = phoneService.m5184WoWo(str2);
                    } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-36, 80, 99, -126, -72}, new byte[]{-9, 7, TarConstants.LF_CHR, -48, -12, 18, 22, 65}, str2)) {
                        byte[] bArr12 = new byte[i14];
                        // fill-array-data instruction
                        bArr12[0] = 42;
                        bArr12[1] = 40;
                        bArr12[2] = 50;
                        bArr12[3] = -107;
                        bArr12[4] = 106;
                        bArr12[5] = 15;
                        if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, bArr12, new byte[]{1, Byte.MAX_VALUE, 98, -57, 38, TarConstants.LF_NORMAL, 84, 93}, str2)) {
                            m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_NORMAL, -31, -4, -56, -91, 61, 40, -17}, new byte[]{27, -74, -84, -102, -23, 7, 8, -34});
                        } else {
                            m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-64, -88, -34, -108, 79, -79, 80, -55, -92, -71, -87, -15, 91}, new byte[]{-21, -21, -109, -47, 111, -12, 2, -101});
                        }
                        str3 = m17835WWWWWWWW;
                    } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{126, TarConstants.LF_CONTIG, TarConstants.LF_DIR, 102, -88}, new byte[]{85, 96, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 43, -8, 79, 13, -14}, str2)) {
                        str3 = PhoneService.m5153WWWoWWWo(str2);
                    } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{TarConstants.LF_CHR, -100, 114, TarConstants.LF_NORMAL, -41}, new byte[]{24, -53, 33, 101, -107, 20, 111, 20}, str2)) {
                        str3 = phoneService.m5183WW(str2);
                    } else if (AbstractC1017WWWoWWWo.m3444WWWWWWWW(wwwwwwww, new byte[]{-60, 59, TarConstants.LF_LINK, 44, TarConstants.LF_LINK}, new byte[]{-17, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 99, Byte.MAX_VALUE, 124, 31, -124, -59}, str2)) {
                        str3 = phoneService.m5163WWWWoWWWWo(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{-119, -84, TarConstants.LF_GNUTYPE_LONGNAME, -73, -22}, new byte[]{-94, -17, 5, -6, -93, -74, -19, -8}))) {
                        str3 = phoneService.m5166WWWWWWWW(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{21, -17, -114, -64, -70}, new byte[]{62, -84, -56, -107, -12, -59, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 123}))) {
                        str3 = phoneService.m5165WWWWWWWW(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{96, 116, 112, 10, -96}, new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, TarConstants.LF_CONTIG, 32, 67, -18, 69, 72, 4}))) {
                        str3 = phoneService.m5169WWWWWWWW(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{-122}, new byte[]{-62, 15, -54, -88, 44, TarConstants.LF_NORMAL, 37, ConstantPoolEntry.CP_NameAndType}))) {
                        phoneService.m5172WWWWWWWW(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{60, -97, -78, 56, -8}, new byte[]{23, -36, -2, 123, -69, -10, 31, -73}))) {
                        str3 = phoneService.m5167WWWWWWWW(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{-70, -127, 106, TarConstants.LF_GNUTYPE_LONGLINK, 84}, new byte[]{-111, -62, 34, 7, 16, -98, -126, 102}))) {
                        str3 = phoneService.m5179WWoWWo(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{ConstantPoolEntry.CP_NameAndType, TarConstants.LF_GNUTYPE_SPARSE, -111, 26, -58}, new byte[]{39, 16, -62, 89, -107, 33, -65, 100}))) {
                        str3 = PhoneService.m5141WWWWWWWWWW(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{95, 89, 78, 5, 111}, new byte[]{116, 26, 3, 66, 41, 74, 92, 7}))) {
                        str3 = PhoneService.m5156WWoWWo(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{-34, 25, 21, 28, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -94, 84}, new byte[]{-11, 90, 82, TarConstants.LF_MULTIVOLUME, TarConstants.LF_DIR, -25, 5, -54}))) {
                        str3 = PhoneService.m5148WWWWWWWW(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 44, 59, -23, -19, -105, -84}, new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, 111, 124, -72, -96, -34, -30, 104}))) {
                        str3 = PhoneService.m5147WWWWWWWW(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{7, 97, -116, -63, -21}, new byte[]{44, 34, -63, -122, -72, -127, -59, 69}))) {
                        str3 = phoneService.m5168WWWWWWWW(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{-8, Byte.MAX_VALUE, 90, -5, 115}, new byte[]{-45, 60, 23, -66, TarConstants.LF_FIFO, 38, -80, 105}))) {
                        str3 = PhoneService.m5152WWWoWWWo(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{-32, 124, -43, -35, -52}, new byte[]{-53, 63, -104, -120, -104, 19, 102, -1}))) {
                        str3 = phoneService.m5178WWWoWWWo(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{-12, -67, -103, 69, -127}, new byte[]{-33, -2, -38, 18, -64, -45, -82, TarConstants.LF_PAX_EXTENDED_HEADER_UC}))) {
                        str3 = PhoneService.m5144WWWWWWWW(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{-65, 72, -77, 89, -101}, new byte[]{-108, ConstantPoolEntry.CP_InterfaceMethodref, -2, 22, -33, -4, 86, -9}))) {
                        str3 = PhoneService.m5157WWoWWo(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_CONTIG, 0, -92, TarConstants.LF_NORMAL, -34}, new byte[]{28, 67, -9, 99, -112, Byte.MAX_VALUE, 17, -37}))) {
                        str3 = PhoneService.m5160WWWW(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{99, 78, 86, -24, TarConstants.LF_MULTIVOLUME}, new byte[]{72, 13, 25, -92, 29, -3, -56, -71}))) {
                        str3 = PhoneService.m5162o(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{-72, -113, 5, -52, -31}, new byte[]{-109, -52, 80, -97, -91, 108, 44, -13}))) {
                        str3 = PhoneService.m5158WWoWWo(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{113, 85, -17, 4, -68}, new byte[]{90, 22, -84, TarConstants.LF_GNUTYPE_LONGNAME, -13, 15, -104, 39}))) {
                        str3 = PhoneService.m5151WWWoWWWo(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{109, 64, -50, 94, -85}, new byte[]{70, 3, -115, 22, -24, -55, -94, 21}))) {
                        str3 = PhoneService.m5142WWWWoWWWWo(str2);
                    } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{-36, -55, -125, -37, -115, TarConstants.LF_GNUTYPE_SPARSE, -59}, new byte[]{-9, -118, -60, -98, -33, 22, -107, 3}))) {
                        str3 = PhoneService.m5146WWWWWWWW(str2);
                    } else {
                        byte[] bArr13 = new byte[i14];
                        // fill-array-data instruction
                        bArr13[0] = 46;
                        bArr13[1] = -7;
                        bArr13[2] = 51;
                        bArr13[3] = -98;
                        bArr13[4] = -88;
                        bArr13[5] = -22;
                        if (str2.startsWith(StringFog.m5049WWWWWWWW(bArr13, new byte[]{5, -70, 116, -33, -21, -66, 14, 31}))) {
                            str3 = PhoneService.m5155WWoWWo(str2);
                        } else if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{126, -54, -41, 37, 25, -4, -46, -88}, new byte[]{85, -119, -112, 97, 90, -77, -100, -4}))) {
                            str3 = PhoneService.m5145WWWWWWWW(str2);
                        } else {
                            if (str2.startsWith(StringFog.m5049WWWWWWWW(new byte[]{-10}, new byte[]{-73, -114, -124, -108, 92, 62, 71, 7}))) {
                                ArrayList arrayList = phoneService.f9076WWWWWWWW;
                                int size = arrayList.size();
                                int i19 = 0;
                                while (i19 < size) {
                                    Object obj = arrayList.get(i19);
                                    i19++;
                                    CallPdu callPdu = (CallPdu) obj;
                                    if (callPdu.f9130WWWoWWWo == 4) {
                                        callPdu.f9130WWWoWWWo = 0;
                                    }
                                }
                            } else {
                                str3 = StringFog.m5049WWWWWWWW(new byte[]{96, 98, 37, -115, 92, 99, -47, -20, 4, 115, 82, -24, 72}, new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, 33, 104, -56, 124, 38, -125, -66});
                            }
                            if (!TextUtils.isEmpty(str3)) {
                                sb2.append(str3);
                                sb2.append("\r");
                            }
                            i17++;
                            i13 = 3;
                            i14 = 6;
                            i15 = 7;
                            i16 = 5;
                        }
                    }
                    if (!TextUtils.isEmpty(str3)) {
                    }
                    i17++;
                    i13 = 3;
                    i14 = 6;
                    i15 = 7;
                    i16 = 5;
                }
            }
            if (!TextUtils.isEmpty(str3)) {
            }
            i17++;
            i13 = 3;
            i14 = 6;
            i15 = 7;
            i16 = 5;
        }
        if (sb2.length() != 0) {
            sb2.deleteCharAt(sb2.length() - 1);
        }
        return sb2.toString();
    }

    private void SetDelay(int i10, int i11) {
        SensorService sensorService = this.mSensorService;
        sensorService.getClass();
        if (i10 >= 0 && i10 < 12) {
            sensorService.f9108WWoWWo[i10] = 0;
            sensorService.f9103WWWWWWWW[i10] = 0;
        }
    }

    private void SetGPSStart() {
        LocationService locationService = this.mLocationService;
        locationService.getClass();
        byte[] bArr = {99, 37, -91, 68, TarConstants.LF_GNUTYPE_LONGNAME, 24, -121, TarConstants.LF_PAX_EXTENDED_HEADER_LC};
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(LocationService.f9050WWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{56, 86, -47, 37, 62, 108, -53, 23, 0, 68, -47, 45, 35, 118, -38}, bArr));
        locationService.f9062WWoWWo = true;
        locationService.f9061WWWoWWWo = true;
        if (!locationService.f9056WWWWWWWW) {
            locationService.m5134WWWWWWWW(true);
        }
    }

    private void SetGPSStop() {
        this.mLocationService.m5135WWWWWWWW();
    }

    private void SetWiFiStart() {
        WiFiService wiFiService = this.mWiFiService;
        wiFiService.f9118WWWWWWWW.post(new RunnableC0054WWWWWWWW(14, wiFiService));
    }

    private String Vibrate(String str) {
        String str2;
        HWControlService hWControlService = this.mHWControlService;
        hWControlService.getClass();
        if (str == null) {
            byte[] bArr = {13, TarConstants.LF_GNUTYPE_LONGNAME, -10, 116};
            byte[] bArr2 = {99, 57, -102, 24, 23, 2, TarConstants.LF_SYMLINK, -51};
            StringFog.f8859WWWWWWWW.getClass();
            return WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
        }
        StringFog.f8859WWWWWWWW.getClass();
        String[] split = str.split(WWWWWWWW.m17835WWWWWWWW(new byte[]{-39}, new byte[]{-29, 100, -28, -123, -99, -84, -6, -100}));
        if (split.length >= 2 && (str2 = split[1]) != null) {
            try {
                long parseLong = Long.parseLong(str2);
                if (parseLong <= 0) {
                    return WWWWWWWW.m17835WWWWWWWW(new byte[]{-44, -11, 94, 66}, new byte[]{-70, Byte.MIN_VALUE, TarConstants.LF_SYMLINK, 46, -50, -29, 98, 81});
                }
                if (SystemClock.uptimeMillis() - hWControlService.f9048WWWoWWWo < 100) {
                    return WWWWWWWW.m17835WWWWWWWW(new byte[]{-78, -106, 124, -9}, new byte[]{-36, -29, 16, -101, TarConstants.LF_SYMLINK, 62, 104, -1});
                }
                hWControlService.f9048WWWoWWWo = SystemClock.uptimeMillis();
                hWControlService.f9047WWWWWWWW.post(new WWWWWWWW(hWControlService, parseLong, 0));
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{-86, -79, -12, 78}, new byte[]{-60, -60, -104, 34, 40, -57, -112, -85});
            } catch (Throwable unused) {
                byte[] bArr3 = {TarConstants.LF_SYMLINK, 17, -119, -50, 110, 110, -4, -56};
                StringFog.f8859WWWWWWWW.getClass();
                return WWWWWWWW.m17835WWWWWWWW(new byte[]{92, 100, -27, -94}, bArr3);
            }
        }
        return WWWWWWWW.m17835WWWWWWWW(new byte[]{7, -12, 13, 113}, new byte[]{105, -127, 97, 29, 87, -97, -51, -43});
    }

    private int getPicture(String str, int i10, int i11, int i12) {
        CameraService cameraService = this.mCameraService;
        Camera camera = cameraService.f9031WWWWWWWW;
        String str2 = CameraService.f9028WWWW;
        if (camera == null) {
            byte[] bArr = {32, -74, -92, TarConstants.LF_LINK, 81, 29, -112, -8};
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5040WWWWoWWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{123, -47, -63, 69, 1, 116, -13, -116, 85, -60, -63, 108, 113, 115, -1, -40, 67, -41, -55, 84, 35, 124}, bArr));
        } else {
            try {
                Camera.Parameters parameters = camera.getParameters();
                parameters.setPictureSize(i10, i11);
                parameters.setJpegQuality(100);
                parameters.setPictureFormat(i12);
                camera.setParameters(parameters);
                camera.takePicture(null, null, cameraService);
            } catch (Throwable th2) {
                byte[] bArr2 = {62, 82, -32, 6, -99, -98, -44, TarConstants.LF_GNUTYPE_LONGLINK, 16, 71, -32, 47, -19, -110, -49, 92, 0, 69, -15, 27, -94, -103, -115, 31};
                byte[] bArr3 = {101, TarConstants.LF_DIR, -123, 114, -51, -9, -73, 63};
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5044WWWoWWWo(str2, WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3), th2);
            }
        }
        return 0;
    }

    private native void nativeCameraPicture(long j10, String str, byte[] bArr);

    private native void nativeCameraPreview(long j10, String str, byte[] bArr);

    private native void nativeDispose(long j10);

    private native void nativeGPSNmeaChanged(long j10, String str);

    private native void nativePhoneUnsolicited(long j10, String str);

    private native void nativeSensorChanged(long j10, int i10, long j11, float f10, float f11, float f12);

    private native long nativeSetup(int i10);

    private native int nativeStartHALMgr(long j10);

    private native int nativeStopHALMgr(long j10);

    private native void nativeWIFIChanged(long j10, String str);

    public void CameraPicture(String str, byte[] bArr) {
        nativeCameraPicture(this.mNativePtr, str, bArr);
    }

    public void CameraPreview(String str, byte[] bArr) {
        nativeCameraPreview(this.mNativePtr, str, bArr);
    }

    public void GPSNmeaChanged(String str) {
        nativeGPSNmeaChanged(this.mNativePtr, str);
    }

    public void PhoneUnsolicited(String str) {
        nativePhoneUnsolicited(this.mNativePtr, str);
    }

    public void SensorChanged(int i10, long j10, float f10, float f11, float f12) {
        nativeSensorChanged(this.mNativePtr, i10, j10, f10, f11, f12);
    }

    public void WIFIChanged(String str) {
        nativeWIFIChanged(this.mNativePtr, str);
    }

    public void finalize() throws Throwable {
        try {
            long j10 = this.mNativePtr;
            if (j10 != 0) {
                nativeDispose(j10);
                this.mNativePtr = 0L;
            }
        } finally {
            super.finalize();
        }
    }

    public LocationService getLocationService() {
        return this.mLocationService;
    }

    public PhoneService getPhoneService() {
        return this.mPhoneService;
    }

    public SensorService getSensorService() {
        return this.mSensorService;
    }

    public void onBackground() {
        Sensor sensor;
        SensorService sensorService = this.mSensorService;
        sensorService.getClass();
        byte[] bArr = {47, 81, 20, -114, -61, 25, -9, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER};
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(SensorService.f9097WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{116, 62, 122, -52, -94, 122, -100, 0, 93, 62, 97, -32, -89, 68}, bArr));
        sensorService.f9104WWWWWWWW = false;
        int i10 = 0;
        while (true) {
            SparseIntArray sparseIntArray = SensorService.f9098WWWW;
            if (i10 >= sparseIntArray.size()) {
                break;
            }
            int keyAt = sparseIntArray.keyAt(i10);
            if ((sensorService.f9102WWWWWWWW[keyAt] & 4) == 4 && (sensor = sensorService.f9101WWWWWWWW[keyAt]) != null) {
                sensorService.f9106WWWoWWWo.unregisterListener(sensorService, sensor);
            }
            i10++;
        }
        this.mLocationService.f9055WWWWWWWW = false;
        byte[] bArr2 = {17, -12, -103, -120, -56, -27, -98, TarConstants.LF_BLK, 56, -12, -126, -92, -51, -37, -43, 58, 45, -11, -104, -72, -52};
        byte[] bArr3 = {74, -101, -9, -54, -87, -122, -11, TarConstants.LF_GNUTYPE_SPARSE};
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(LocationService.f9050WWWW, WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
        CameraService cameraService = this.mCameraService;
        cameraService.getClass();
        KLog.m5043WWWWWWWW(CameraService.f9028WWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{60, -69, -11, 115, 101, 4, -23, -108, 21, -69, -18, 95, 96, 58}, new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -44, -101, TarConstants.LF_LINK, 4, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -126, -13}));
        String str = cameraService.f9032WWWWWWWW;
        CameraService.CameraState cameraState = cameraService.f9039WWoWWo;
        if (cameraState == CameraService.CameraState.f9042WWWWWWWW) {
            cameraService.m5125WWWWWWWW(str);
        }
        if (cameraState != CameraService.CameraState.f9041WWWWoWWWWo) {
            cameraService.m5122WWWWoWWWWo(str);
        }
        cameraService.f9032WWWWWWWW = str;
        cameraService.f9039WWoWWo = cameraState;
    }

    public void onForeground() {
        Sensor sensor;
        SensorService sensorService = this.mSensorService;
        sensorService.getClass();
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(SensorService.f9097WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{45, 5, -117, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -127, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -76, TarConstants.LF_GNUTYPE_LONGLINK, 4, 5, -112, 80, -118, 87}, new byte[]{118, 106, -27, 62, -18, 10, -47, 44}));
        sensorService.f9104WWWWWWWW = true;
        int i10 = 0;
        while (true) {
            SparseIntArray sparseIntArray = SensorService.f9098WWWW;
            if (i10 >= sparseIntArray.size()) {
                break;
            }
            int keyAt = sparseIntArray.keyAt(i10);
            if ((sensorService.f9102WWWWWWWW[keyAt] & 4) == 4 && (sensor = sensorService.f9101WWWWWWWW[keyAt]) != null) {
                int i11 = sensorService.f9108WWoWWo[keyAt];
                if (i11 == 0) {
                    i11 = 1;
                }
                sensorService.f9106WWWoWWWo.registerListener(sensorService, sensor, i11, sensorService.f9103WWWWWWWW[keyAt]);
            }
            i10++;
        }
        LocationService locationService = this.mLocationService;
        locationService.f9055WWWWWWWW = true;
        boolean z10 = locationService.f9062WWoWWo;
        String str = LocationService.f9050WWWW;
        if (z10 && !locationService.f9056WWWWWWWW) {
            byte[] bArr = {23, 125, 118, TarConstants.LF_FIFO, 60, -42, -71, TarConstants.LF_BLK, 62, 125, 109, 30, TarConstants.LF_CONTIG, -7, -4, 33, 41, 97, 109, 29, TarConstants.LF_FIFO, -124, -80, 60, 47, 115, 108, 25, 60, -54};
            byte[] bArr2 = {TarConstants.LF_GNUTYPE_LONGNAME, 18, 24, 112, TarConstants.LF_GNUTYPE_SPARSE, -92, -36, TarConstants.LF_GNUTYPE_SPARSE};
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5043WWWWWWWW(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
            locationService.m5134WWWWWWWW(locationService.f9061WWWoWWWo);
        } else {
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5043WWWWWWWW(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-4, 124, -24, -120, -48, 41, -98, -70, -43, 124, -13, -96, -37, 6, -37, -76, -64, 125, -23, -68, -38}, new byte[]{-89, 19, -122, -50, -65, 91, -5, -35}));
        }
        CameraService cameraService = this.mCameraService;
        cameraService.getClass();
        StringFog.f8859WWWWWWWW.getClass();
        KLog.m5043WWWWWWWW(CameraService.f9028WWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-47, -106, -120, -120, 110, -125, 24, -112, -8, -106, -109, -96, 101, -84}, new byte[]{-118, -7, -26, -50, 1, -15, 125, -9}));
        CameraService.CameraState cameraState = cameraService.f9039WWoWWo;
        if (cameraState != CameraService.CameraState.f9041WWWWoWWWWo) {
            cameraService.m5123WWWWWWWW(cameraService.f9032WWWWWWWW);
        }
        if (cameraState == CameraService.CameraState.f9042WWWWWWWW) {
            cameraService.m5124WWWWWWWW(cameraService.f9033WWWWWWWW, cameraService.f9034WWWWWWWW, cameraService.f9038WWWoWWWo, cameraService.f9032WWWWWWWW);
        }
    }

    public int startHALMgr() {
        this.mCameraService.getClass();
        LocationService locationService = this.mLocationService;
        locationService.getClass();
        HandlerThread handlerThread = new HandlerThread(LocationService.f9050WWWW);
        locationService.f9066WoWo = handlerThread;
        handlerThread.start();
        locationService.f9057WWWWWWWW = new Handler(locationService.f9066WoWo.getLooper());
        this.mHWControlService.getClass();
        WiFiService wiFiService = this.mWiFiService;
        wiFiService.getClass();
        HandlerThread handlerThread2 = new HandlerThread(WiFiService.f9114WWoWWo);
        wiFiService.f9117WWWWWWWW = handlerThread2;
        handlerThread2.start();
        wiFiService.f9118WWWWWWWW = new Handler(wiFiService.f9117WWWWWWWW.getLooper());
        SensorService sensorService = this.mSensorService;
        sensorService.getClass();
        HandlerThread handlerThread3 = new HandlerThread(SensorService.f9097WWWWWWWW);
        sensorService.f9107WWWoWWWo = handlerThread3;
        handlerThread3.start();
        sensorService.f9105WWWWWWWW = new Handler(sensorService.f9107WWWoWWWo.getLooper());
        this.mPhoneService.f9076WWWWWWWW.clear();
        BatteryService batteryService = this.mBatteryService;
        batteryService.getClass();
        String str = BatteryService.f9018WWWWWWWW;
        HandlerThread handlerThread4 = new HandlerThread(str);
        batteryService.f9024WWWWWWWW = handlerThread4;
        handlerThread4.start();
        batteryService.f9026WWWoWWWo = new Handler(batteryService.f9024WWWWWWWW.getLooper());
        try {
            IntentFilter intentFilter = new IntentFilter();
            byte[] bArr = {-124, 90, -94, 98, 19, -70, ConstantPoolEntry.CP_NameAndType, -126, -116, 90, -78, 117, 18, -89, 70, -51, -122, 64, -81, Byte.MAX_VALUE, 18, -3, 42, -19, -79, 96, -125, 66, 37, -116, 43, -28, -92, 122, -127, 85, 56};
            byte[] bArr2 = {-27, TarConstants.LF_BLK, -58, 16, 124, -45, 104, -84};
            StringFog.f8859WWWWWWWW.getClass();
            intentFilter.addAction(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
            batteryService.f9020WWWWWWWW.registerReceiver(batteryService, intentFilter);
            batteryService.f9023WWWWWWWW = true;
            batteryService.m5120WWWWWWWW(new Intent());
        } catch (Throwable th2) {
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5044WWWoWWWo(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-86, -121, -47, -18, TarConstants.LF_SYMLINK, -104, 73, -98, -108, -116, -58, -22, TarConstants.LF_NORMAL, -104, 125, -47, -97, -50, -123}, new byte[]{-15, -12, -91, -113, 64, -20, 20, -66}), th2);
        }
        return nativeStartHALMgr(this.mNativePtr);
    }

    public int stopHALMgr() {
        this.mCameraService.getClass();
        LocationService locationService = this.mLocationService;
        HandlerThread handlerThread = locationService.f9066WoWo;
        if (handlerThread != null) {
            handlerThread.quit();
        }
        locationService.m5135WWWWWWWW();
        this.mHWControlService.getClass();
        HandlerThread handlerThread2 = this.mWiFiService.f9117WWWWWWWW;
        if (handlerThread2 != null) {
            handlerThread2.quit();
        }
        SensorService sensorService = this.mSensorService;
        HandlerThread handlerThread3 = sensorService.f9107WWWoWWWo;
        if (handlerThread3 != null) {
            handlerThread3.quit();
        }
        int i10 = 0;
        while (true) {
            SparseIntArray sparseIntArray = SensorService.f9098WWWW;
            if (i10 >= sparseIntArray.size()) {
                break;
            }
            int keyAt = sparseIntArray.keyAt(i10);
            int[] iArr = sensorService.f9102WWWWWWWW;
            int i11 = iArr[keyAt];
            if ((i11 & 4) == 4 && keyAt >= 0 && keyAt < 12) {
                iArr[keyAt] = i11 & (-5);
                Sensor sensor = sensorService.f9101WWWWWWWW[keyAt];
                if (sensor != null) {
                    sensorService.f9106WWWoWWWo.unregisterListener(sensorService, sensor);
                }
            }
            i10++;
        }
        this.mPhoneService.f9076WWWWWWWW.clear();
        BatteryService batteryService = this.mBatteryService;
        HandlerThread handlerThread4 = batteryService.f9024WWWWWWWW;
        if (handlerThread4 != null) {
            handlerThread4.quit();
        }
        if (batteryService.f9023WWWWWWWW) {
            try {
                batteryService.f9020WWWWWWWW.unregisterReceiver(batteryService);
                batteryService.f9023WWWWWWWW = false;
            } catch (Throwable th2) {
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5044WWWoWWWo(BatteryService.f9018WWWWWWWW, WWWWWWWW.m17835WWWWWWWW(new byte[]{-42, 124, 85, -61, -79, 122, -112, -124, -11, 108, 68, -36, -75, 78, -33, -113, -73, 47}, new byte[]{-115, 15, 33, -84, -63, 39, -80, -31}), th2);
            }
        }
        return nativeStopHALMgr(this.mNativePtr);
    }
}
