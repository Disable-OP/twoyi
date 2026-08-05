package com.android.vmcore.hal;

import android.content.Context;
import android.graphics.SurfaceTexture;
import android.hardware.Camera;
import android.text.TextUtils;
import android.util.Log;
import com.android.vmcore.KLog;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.util.ArrayList;
import java.util.List;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import vf.AbstractC4470WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class CameraService implements Camera.PreviewCallback, Camera.PictureCallback, Camera.ErrorCallback {

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public static final String f9028WWWW;

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final VMInstance f9029WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final Context f9030WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public Camera f9031WWWWWWWW;

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public String f9032WWWWWWWW;

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public int f9033WWWWWWWW;

    /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
    public int f9034WWWWWWWW;

    /* renamed from: WWWW֭WWWWྥ֭  reason: contains not printable characters */
    public SurfaceTexture f9035WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final HALManager f9037WWWoWWWo;

    /* renamed from: WWWoԻWWWoͷԻ  reason: contains not printable characters */
    public int f9038WWWoWWWo;

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public CameraState f9039WWoWWo = CameraState.f9041WWWWoWWWWo;

    /* renamed from: WWWWآWWWWȫآ  reason: contains not printable characters */
    public final Object f9036WWWWWWWW = new Object();

    /* JADX WARN: Failed to restore enum class, 'enum' modifier and super class removed */
    /* JADX WARN: Unknown enum class pattern. Please report as an issue! */
    /* loaded from: classes.dex */
    public static final class CameraState {

        /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
        public static final CameraState f9040WWWWWWWWWW;

        /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
        public static final CameraState f9041WWWWoWWWWo;

        /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
        public static final CameraState f9042WWWWWWWW;

        /* renamed from: WWWWཀྵWWWWࠤཀྵ  reason: contains not printable characters */
        public static final /* synthetic */ CameraState[] f9043WWWWWWWW;

        /* renamed from: WWີWWഫີ  reason: contains not printable characters */
        public static final CameraState f9044WWWW;

        /* JADX WARN: Multi-variable type inference failed */
        /* JADX WARN: Type inference failed for: r10v4, types: [com.android.vmcore.hal.CameraService$CameraState, java.lang.Enum] */
        /* JADX WARN: Type inference failed for: r5v0, types: [com.android.vmcore.hal.CameraService$CameraState, java.lang.Enum] */
        /* JADX WARN: Type inference failed for: r6v3, types: [com.android.vmcore.hal.CameraService$CameraState, java.lang.Enum] */
        /* JADX WARN: Type inference failed for: r9v4, types: [com.android.vmcore.hal.CameraService$CameraState, java.lang.Enum] */
        static {
            StringFog.f8859WWWWWWWW.getClass();
            ?? r52 = new Enum(WWWWWWWW.m17835WWWWWWWW(new byte[]{-15, 25, 99, TarConstants.LF_GNUTYPE_SPARSE, 98, 13, -46, -59, -15}, new byte[]{-76, 90, 39, 0, 61, 67, -99, -117}), 0);
            f9041WWWWoWWWWo = r52;
            ?? r62 = new Enum(WWWWWWWW.m17835WWWWWWWW(new byte[]{-117, 90, -113, 98, TarConstants.LF_GNUTYPE_LONGNAME, -54, -123, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, Byte.MIN_VALUE, 92, -120, 101, 86, -51}, new byte[]{-50, 25, -53, TarConstants.LF_LINK, 19, -119, -54, 41}), 1);
            f9040WWWWWWWWWW = r62;
            ?? r92 = new Enum(WWWWWWWW.m17835WWWWWWWW(new byte[]{-106, 117, -10, -72, -46, -74, TarConstants.LF_CONTIG, 67, -127, 98, -9, -81}, new byte[]{-45, TarConstants.LF_FIFO, -78, -21, -115, -27, 99, 2}), 2);
            f9042WWWWWWWW = r92;
            ?? r10 = new Enum(WWWWWWWW.m17835WWWWWWWW(new byte[]{65, 7, -14, 20, 102, 31, -23, 37, 84, 20, -13, 3}, new byte[]{4, 68, -74, 71, 57, TarConstants.LF_GNUTYPE_LONGNAME, -67, 106}), 3);
            f9044WWWW = r10;
            f9043WWWWWWWW = new CameraState[]{r52, r62, r92, r10};
        }

        public static CameraState valueOf(String str) {
            return (CameraState) Enum.valueOf(CameraState.class, str);
        }

        public static CameraState[] values() {
            return (CameraState[]) f9043WWWWWWWW.clone();
        }
    }

    static {
        StringFog.f8859WWWWWWWW.getClass();
        f9028WWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-118, -50, -88, -28, -12, -63, -109, -94, -69, -39, -84, -30, -29}, new byte[]{-55, -81, -59, -127, -122, -96, -64, -57});
    }

    public CameraService(Context context, VMInstance vMInstance, HALManager hALManager) {
        this.f9030WWWWWWWW = context;
        this.f9029WWWWoWWWWo = vMInstance;
        this.f9037WWWoWWWo = hALManager;
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public static String m5121WWWoWWWo(int i10) {
        Camera camera;
        String m17835WWWWWWWW;
        byte b8 = -24;
        StringBuilder sb2 = new StringBuilder();
        try {
            camera = Camera.open(i10);
            if (camera == null) {
                if (camera != null) {
                    camera.release();
                }
                return FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING;
            }
            try {
                sb2.setLength(0);
                Camera.Parameters parameters = camera.getParameters();
                List<Camera.Size> supportedPreviewSizes = parameters.getSupportedPreviewSizes();
                ArrayList arrayList = new ArrayList();
                arrayList.add(new Camera.Size(camera, 1920, 1920));
                supportedPreviewSizes.removeAll(arrayList);
                for (Camera.Size size : supportedPreviewSizes) {
                    if (size.width <= 1920 && size.height <= 1920) {
                        if (sb2.length() != 0) {
                            byte[] bArr = {15, -29, -52, -95, -29, ConstantPoolEntry.CP_InterfaceMethodref, -96, 39};
                            StringFog.f8859WWWWWWWW.getClass();
                            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{35}, bArr));
                        }
                        sb2.append(size.width);
                        byte[] bArr2 = {TarConstants.LF_BLK};
                        byte[] bArr3 = {TarConstants.LF_GNUTYPE_LONGNAME, 43, -115, 122, -4, -88, -41, -119};
                        StringFog.f8859WWWWWWWW.getClass();
                        sb2.append(WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
                        sb2.append(size.height);
                    }
                }
                String sb3 = sb2.toString();
                Camera.CameraInfo cameraInfo = new Camera.CameraInfo();
                Camera.getCameraInfo(i10, cameraInfo);
                if (cameraInfo.facing != 1) {
                    byte[] bArr4 = {63, ConstantPoolEntry.CP_InterfaceMethodref, 99, -82, ConstantPoolEntry.CP_InterfaceMethodref, -100, -83, 87};
                    StringFog.f8859WWWWWWWW.getClass();
                    m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{93, 106, 0, -59}, bArr4);
                } else {
                    StringFog.f8859WWWWWWWW.getClass();
                    m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{-103, -42, -42, -16, -41}, new byte[]{-1, -92, -71, -98, -93, 124, -126, -114});
                }
                int maxNumFocusAreas = parameters.getMaxNumFocusAreas();
                sb2.setLength(0);
                List<Integer> zoomRatios = parameters.getZoomRatios();
                if (zoomRatios != null) {
                    for (Integer num : zoomRatios) {
                        if (sb2.length() != 0) {
                            StringFog.f8859WWWWWWWW.getClass();
                            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{b8}, new byte[]{-60, 59, -14, -63, -42, 66, 96, -89}));
                        }
                        sb2.append(num);
                        b8 = -24;
                    }
                }
                String sb4 = sb2.toString();
                sb2.setLength(0);
                List<String> supportedFocusModes = parameters.getSupportedFocusModes();
                if (supportedFocusModes != null) {
                    for (String str : supportedFocusModes) {
                        if (sb2.length() != 0) {
                            StringFog.f8859WWWWWWWW.getClass();
                            sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-21}, new byte[]{-57, -35, 121, 21, 45, -123, -37, -121}));
                        }
                        sb2.append(str);
                    }
                }
                String sb5 = sb2.toString();
                sb2.setLength(0);
                StringFog.f8859WWWWWWWW.getClass();
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{5, -68, -58, -87, -87, -7, 37, -85, 14, -81, -54, -109}, new byte[]{107, -35, -85, -52, -108, -102, 68, -58}));
                sb2.append(i10);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-45, -90, -106, -59, -46, -81, -114, 14, -98, -77, -39}, new byte[]{-13, -64, -28, -92, -65, -54, -22, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}));
                sb2.append(sb3);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-56, -87, 15, -81, 2}, new byte[]{-24, -51, 102, -35, 63, 123, 61, -7}));
                sb2.append(m17835WWWWWWWW);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-35, -66, -79, -116, -83, -69, -105, -23, -112, -84, -2}, new byte[]{-3, -33, -61, -23, -52, -56, -7, -100}));
                sb2.append(maxNumFocusAreas);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-24, -10, -89, ConstantPoolEntry.CP_NameAndType, 39, 56}, new byte[]{-56, -116, -56, 99, 74, 5, 39, 46}));
                sb2.append(sb4);
                sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, -46, 95, 119, 82, -92, 85}, new byte[]{-66, -76, TarConstants.LF_NORMAL, 20, 39, -41, 104, 42}));
                sb2.append(sb5);
                sb2.append(" \n");
                String sb6 = sb2.toString();
                camera.release();
                return sb6;
            } catch (Throwable th2) {
                th = th2;
                if (camera != null) {
                    camera.release();
                }
                throw th;
            }
        } catch (Throwable th3) {
            th = th3;
            camera = null;
        }
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final void m5122WWWWoWWWWo(String str) {
        StringBuilder sb2 = new StringBuilder();
        String m17683WWWWWWWW = AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{122, -113, -92, -106, -58, -76, -36, -7, 68, -120, -71, -90, -60, -74, -41, -27, 64, -74, -19, -127, -52, -88, -47, -8, 79, -123, -88, -122, -47, -5}, new byte[]{33, -21, -51, -27, -91, -37, -78, -105}, sb2, str);
        String str2 = f9028WWWW;
        KLog.m5043WWWWWWWW(str2, m17683WWWWWWWW);
        Camera camera = this.f9031WWWWWWWW;
        if (camera != null) {
            this.f9031WWWWWWWW = null;
            this.f9032WWWWWWWW = null;
            this.f9039WWoWWo = CameraState.f9041WWWWoWWWWo;
            try {
                camera.setErrorCallback(null);
                camera.setPreviewCallback(null);
                camera.stopPreview();
                camera.release();
            } catch (Throwable th2) {
                byte[] bArr = {79, 81, 125, -39, 72, 60, -127, -15, 113, 86, 96, -23, 74, 62, -118, -19, 117, 104, TarConstants.LF_BLK, -49, TarConstants.LF_GNUTYPE_SPARSE, TarConstants.LF_NORMAL, -118, -17, 96, 92, 123, -60, 17, 115};
                byte[] bArr2 = {20, TarConstants.LF_DIR, 20, -86, 43, TarConstants.LF_GNUTYPE_SPARSE, -17, -97};
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5044WWWoWWWo(str2, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2), th2);
            }
        }
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final void m5123WWWWWWWW(String str) {
        StringBuilder sb2 = new StringBuilder();
        String m17683WWWWWWWW = AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{82, -117, -103, 26, 126, 18, 78, 41, 74, -119, -101, 17, 98, 22, 112, 125, 106, -121, -104, 26, 117, 20, 89, 125}, new byte[]{9, -24, -10, 116, 16, 119, 45, 93}, sb2, str);
        String str2 = f9028WWWW;
        KLog.m5043WWWWWWWW(str2, m17683WWWWWWWW);
        if (this.f9031WWWWWWWW == null) {
            try {
                this.f9031WWWWWWWW = Camera.open(Integer.parseInt(str));
                this.f9032WWWWWWWW = str;
                this.f9039WWoWWo = CameraState.f9040WWWWWWWWWW;
            } catch (Throwable th2) {
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5044WWWoWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-55, 9, 80, -23, -2, 106, -112, -33, -47, ConstantPoolEntry.CP_InterfaceMethodref, 82, -30, -30, 110, -82, -117, -9, 18, 92, -30, -32, 123, -102, -60, -4, 80, 31}, new byte[]{-110, 106, 63, -121, -112, 15, -13, -85}), th2);
            }
        }
    }

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final int m5124WWWWWWWW(int i10, int i11, int i12, String str) {
        String str2 = f9028WWWW;
        KLog.m5043WWWWWWWW(str2, AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{32, 15, 124, 56, 117, -10, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -68, 22, 25, 122, 56, 90, -94, 72, -87, 26, 14, 124, 121}, new byte[]{123, 124, 8, 89, 7, -126, 59, -35}, new StringBuilder(), str));
        Camera camera = this.f9031WWWWWWWW;
        if (camera == null) {
            KLog.m5040WWWWoWWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{5, -97, -26, -8, 59, -127, 86, -6, TarConstants.LF_CHR, -119, -32, -8, 20, -43, 123, -12, 126, -113, -13, -12, 44, -121, 116}, new byte[]{94, -20, -110, -103, 73, -11, 21, -101}));
            return -1;
        }
        try {
            Camera.Parameters parameters = camera.getParameters();
            parameters.setPreviewSize(i10, i11);
            List<String> supportedFocusModes = parameters.getSupportedFocusModes();
            if (supportedFocusModes.contains(WWWWWWWW.m17835WWWWWWWW(new byte[]{84, -124, -69, 115, -33, 93, -82, 58, 66, -104, -8, 113, -33, 87, -66, 58}, new byte[]{TarConstants.LF_CONTIG, -21, -43, 7, -74, TarConstants.LF_CHR, -37, 85}))) {
                parameters.setFocusMode(WWWWWWWW.m17835WWWWWWWW(new byte[]{-33, -122, -56, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 17, -17, -29, TarConstants.LF_CONTIG, -55, -102, -117, 90, 17, -27, -13, TarConstants.LF_CONTIG}, new byte[]{-68, -23, -90, 44, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -127, -106, TarConstants.LF_PAX_EXTENDED_HEADER_UC}));
            } else if (supportedFocusModes.contains(WWWWWWWW.m17835WWWWWWWW(new byte[]{-52, 118, 123, -106, -115, 22, -74, -5, -38, 106, 56, -110, -115, 27, -73, -31, -35, 124}, new byte[]{-81, 25, 21, -30, -28, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -61, -108}))) {
                parameters.setFocusMode(WWWWWWWW.m17835WWWWWWWW(new byte[]{78, -73, -50, 61, -60, -120, 97, 123, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -85, -115, 57, -60, -123, 96, 97, 95, -67}, new byte[]{45, -40, -96, 73, -83, -26, 20, 20}));
            } else {
                KLog.m5040WWWWoWWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{Byte.MAX_VALUE, 37, -96, -85, -87, -81, -119, -12, 73, TarConstants.LF_CHR, -90, -85, -122, -5, -65, -5, 87, 35, -92, -70, -76, -87, -66, -16, 64, 118, -78, -91, -72, -82, -71, -75, 73, 57, -80, -81, -88, -31, -22}, new byte[]{36, 86, -44, -54, -37, -37, -54, -107}) + supportedFocusModes);
            }
            int i13 = 842094169;
            if (i12 != 842093913 && i12 != 842094169) {
                i13 = i12 != 876758866 ? 17 : 42;
            }
            parameters.setPreviewFormat(i13);
            camera.setParameters(parameters);
            if (this.f9035WWWWWWWW == null) {
                this.f9035WWWWWWWW = new SurfaceTexture(911000);
            }
            camera.setPreviewTexture(this.f9035WWWWWWWW);
            camera.setPreviewCallback(this);
            camera.setErrorCallback(this);
            camera.cancelAutoFocus();
            camera.startPreview();
            this.f9039WWoWWo = CameraState.f9042WWWWWWWW;
            this.f9033WWWWWWWW = i10;
            this.f9034WWWWWWWW = i11;
            this.f9038WWWoWWWo = i12;
            synchronized (this.f9036WWWWWWWW) {
                try {
                    this.f9036WWWWWWWW.wait(2000L);
                } catch (InterruptedException unused) {
                }
            }
            return 0;
        } catch (Throwable th2) {
            String str3 = f9028WWWW;
            StringFog.f8859WWWWWWWW.getClass();
            Log.e(str3, WWWWWWWW.m17835WWWWWWWW(new byte[]{124, -114, 90, -53, TarConstants.LF_NORMAL, -43, -107, -67, 74, -104, 92, -53, 31, -127, -77, -92, 68, -104, 94, -34, 43, -50, -72, -26, 7}, new byte[]{39, -3, 46, -86, 66, -95, -42, -36}), th2);
            return -1;
        }
    }

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public final void m5125WWWWWWWW(String str) {
        StringBuilder sb2 = new StringBuilder();
        String m17683WWWWWWWW = AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-1, 112, 28, 33, -58, 98, -80, -7, -63, 113, 9, 19, -106, 82, -91, -5, -44, 35}, new byte[]{-92, 3, 104, 78, -74, 33, -47, -108}, sb2, str);
        String str2 = f9028WWWW;
        KLog.m5043WWWWWWWW(str2, m17683WWWWWWWW);
        this.f9039WWoWWo = CameraState.f9044WWWW;
        Camera camera = this.f9031WWWWWWWW;
        if (camera != null) {
            try {
                camera.setErrorCallback(null);
                camera.setPreviewCallback(null);
                camera.stopPreview();
            } catch (Throwable th2) {
                StringFog.f8859WWWWWWWW.getClass();
                KLog.m5044WWWoWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{-45, 10, 124, -48, -69, -115, -69, ConstantPoolEntry.CP_NameAndType, -19, ConstantPoolEntry.CP_InterfaceMethodref, 105, -30, -21, -85, -94, 2, -19, 9, 124, -42, -92, -96, -32, 65}, new byte[]{-120, 121, 8, -65, -53, -50, -38, 97}), th2);
            }
        }
    }

    @Override // android.hardware.Camera.ErrorCallback
    public final void onError(int i10, Camera camera) {
        StringBuilder sb2 = new StringBuilder();
        StringFog.f8859WWWWWWWW.getClass();
        sb2.append(WWWWWWWW.m17835WWWWWWWW(new byte[]{70, -102, -43, 122, 123, 61, -75, 81, 64, -43, -34, TarConstants.LF_MULTIVOLUME, 123, 32, -88, 3}, new byte[]{29, -11, -69, 63, 9, 79, -38, 35}));
        sb2.append(i10);
        KLog.m5040WWWWoWWWWo(f9028WWWW, sb2.toString());
    }

    @Override // android.hardware.Camera.PictureCallback
    public final void onPictureTaken(byte[] bArr, Camera camera) {
        boolean isEmpty = TextUtils.isEmpty(this.f9032WWWWWWWW);
        String str = f9028WWWW;
        if (isEmpty) {
            byte[] bArr2 = {-17, -79, -13, 20, -104, 82, 59, TarConstants.LF_FIFO, -58, -69, -55, 37, -102, 84, 33, 30, -108, -80, -14, 100, -110, 80, 34, 38, -58, -65, -67, 39, -98, 95, 33, 38, -41, -86, -8, 32};
            byte[] bArr3 = {-76, -34, -99, 68, -15, TarConstants.LF_LINK, 79, 67};
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5040WWWWoWWWWo(str, WWWWWWWW.m17835WWWWWWWW(bArr2, bArr3));
        } else if (this.f9039WWoWWo != CameraState.f9042WWWWWWWW) {
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5040WWWWoWWWWo(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-82, -110, -23, -93, 63, -27, 6, -79, -121, -104, -45, -110, 61, -29, 28, -103, -43, -98, -26, -98, TarConstants.LF_CHR, -12, 19, -28, -101, -110, -13, -45, 37, -14, 19, -74, -127, -104, -29}, new byte[]{-11, -3, -121, -13, 86, -122, 114, -60}));
        } else {
            this.f9037WWWoWWWo.CameraPicture(this.f9032WWWWWWWW, bArr);
        }
    }

    @Override // android.hardware.Camera.PreviewCallback
    public final void onPreviewFrame(byte[] bArr, Camera camera) {
        if (TextUtils.isEmpty(this.f9032WWWWWWWW)) {
            String str = f9028WWWW;
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5040WWWWoWWWWo(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{7, 31, -57, 15, -24, -86, -68, 57, 57, 7, -17, 45, -5, -94, -81, 13, 124, 30, -58, Byte.MAX_VALUE, -7, -82, -89, TarConstants.LF_DIR, 46, 17, -119, 60, -11, -95, -92, TarConstants.LF_DIR, 63, 4, -52, 59}, new byte[]{92, 112, -87, 95, -102, -49, -54, 80}));
        } else if (this.f9039WWoWWo != CameraState.f9042WWWWWWWW) {
            String str2 = f9028WWWW;
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5040WWWWoWWWWo(str2, WWWWWWWW.m17835WWWWWWWW(new byte[]{37, -90, 40, -112, 16, ConstantPoolEntry.CP_NameAndType, -52, -14, 27, -66, 0, -78, 3, 4, -33, -58, 94, -86, 39, -83, 7, 27, -37, -69, 16, -90, TarConstants.LF_SYMLINK, -32, 17, 29, -37, -23, 10, -84, 34}, new byte[]{126, -55, 70, -64, 98, 105, -70, -101}));
        } else {
            synchronized (this.f9036WWWWWWWW) {
                this.f9036WWWWWWWW.notify();
            }
            this.f9037WWWoWWWo.CameraPreview(this.f9032WWWWWWWW, bArr);
        }
    }
}
