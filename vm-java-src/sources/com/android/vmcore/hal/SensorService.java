package com.android.vmcore.hal;

import android.content.Context;
import android.hardware.Sensor;
import android.hardware.SensorEvent;
import android.hardware.SensorEventListener;
import android.hardware.SensorManager;
import android.os.Handler;
import android.os.HandlerThread;
import android.util.SparseIntArray;
import androidx.fragment.app.AbstractC1017WWWoWWWo;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import java.util.Arrays;
import org.apache.commons.compress.archivers.tar.TarConstants;
import p001WWWWoWWWWo.RunnableC0056WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class SensorService implements SensorEventListener {

    /* renamed from: WWWWآWWWWȫآ  reason: contains not printable characters */
    public static final String f9097WWWWWWWW;

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public static final SparseIntArray f9098WWWW;

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final HALManager f9099WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final VMInstance f9100WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final Sensor[] f9101WWWWWWWW;

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public final int[] f9102WWWWWWWW;

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public final int[] f9103WWWWWWWW;

    /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
    public boolean f9104WWWWWWWW;

    /* renamed from: WWWW֭WWWWྥ֭  reason: contains not printable characters */
    public Handler f9105WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final SensorManager f9106WWWoWWWo;

    /* renamed from: WWWoԻWWWoͷԻ  reason: contains not printable characters */
    public HandlerThread f9107WWWoWWWo;

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public final int[] f9108WWoWWo;

    static {
        byte[] bArr = {TarConstants.LF_GNUTYPE_LONGNAME, -116, TarConstants.LF_GNUTYPE_SPARSE, -23, 19, TarConstants.LF_DIR, -21, TarConstants.LF_DIR};
        StringFog.f8859WWWWWWWW.getClass();
        f9097WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{31, -23, 61, -102, 124, 71, -72, 80, 62, -6, 58, -118, 118}, bArr);
        SparseIntArray sparseIntArray = new SparseIntArray(12);
        f9098WWWW = sparseIntArray;
        sparseIntArray.put(0, 1);
        sparseIntArray.put(1, 2);
        sparseIntArray.put(2, 3);
        sparseIntArray.put(3, 7);
        sparseIntArray.put(4, 8);
        sparseIntArray.put(5, 5);
        sparseIntArray.put(6, 6);
        sparseIntArray.put(7, 12);
        sparseIntArray.put(8, 9);
        sparseIntArray.put(9, 19);
        sparseIntArray.put(10, 18);
        sparseIntArray.put(11, 4);
    }

    public SensorService(Context context, VMInstance vMInstance, HALManager hALManager) {
        this.f9100WWWWWWWW = vMInstance;
        this.f9099WWWWoWWWWo = hALManager;
        StringFog.f8859WWWWWWWW.getClass();
        this.f9106WWWoWWWo = (SensorManager) context.getSystemService(WWWWWWWW.m17835WWWWWWWW(new byte[]{-14, -121, -26, TarConstants.LF_DIR, 102, 44}, new byte[]{-127, -30, -120, 70, 9, 94, 95, -34}));
        this.f9101WWWWWWWW = new Sensor[12];
        this.f9102WWWWWWWW = new int[12];
        int i10 = 0;
        while (true) {
            SparseIntArray sparseIntArray = f9098WWWW;
            if (i10 < sparseIntArray.size()) {
                int keyAt = sparseIntArray.keyAt(i10);
                this.f9101WWWWWWWW[keyAt] = this.f9106WWWoWWWo.getDefaultSensor(sparseIntArray.valueAt(i10));
                Sensor sensor = this.f9101WWWWWWWW[keyAt];
                if (sensor == null) {
                    this.f9102WWWWWWWW[keyAt] = 0;
                } else {
                    m5186WWWWWWWW(keyAt, this.f9100WWWWWWWW.m5059WWWWWWWW(sensor.getStringType()));
                }
                i10++;
            } else {
                int[] iArr = new int[12];
                this.f9108WWoWWo = iArr;
                Arrays.fill(iArr, 0);
                int[] iArr2 = new int[12];
                this.f9103WWWWWWWW = iArr2;
                Arrays.fill(iArr2, 0);
                return;
            }
        }
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final void m5186WWWWWWWW(int i10, String str) {
        char c10;
        int hashCode = str.hashCode();
        if (hashCode != -1640567718) {
            if (hashCode != -290852002) {
                if (hashCode == 857820771) {
                    if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-114, -116, TarConstants.LF_GNUTYPE_LONGLINK, -126, -85, 3, -9, -23, -113, -111, TarConstants.LF_GNUTYPE_LONGLINK}, new byte[]{-32, -29, 63, -35, -40, 118, -121, -103}, str)) {
                        c10 = 0;
                    }
                }
                c10 = 65535;
            } else {
                if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{73, -36, -98, 97, -16, 80, -16, -54, 64, -30, -97, 96, -23, 65, -13, -44, TarConstants.LF_MULTIVOLUME}, new byte[]{57, -67, -20, 21, -103, TarConstants.LF_LINK, -100, -90}, str)) {
                    c10 = 1;
                }
                c10 = 65535;
            }
        } else {
            if (AbstractC1017WWWoWWWo.m3430WWWWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{10, -98, -14, 34, TarConstants.LF_MULTIVOLUME, -78, -89, -87, 28, -101, -15, 60, 64}, new byte[]{108, -21, -98, 78, TarConstants.LF_BLK, -19, -44, -36}, str)) {
                c10 = 2;
            }
            c10 = 65535;
        }
        int[] iArr = this.f9102WWWWWWWW;
        if (c10 != 0) {
            if (c10 != 1) {
                if (c10 != 2) {
                    return;
                }
                iArr[i10] = 3;
                return;
            }
            iArr[i10] = 1;
            return;
        }
        iArr[i10] = 0;
    }

    @Override // android.hardware.SensorEventListener
    public final void onAccuracyChanged(Sensor sensor, int i10) {
    }

    @Override // android.hardware.SensorEventListener
    public final void onSensorChanged(SensorEvent sensorEvent) {
        Handler handler = this.f9105WWWWWWWW;
        if (handler == null) {
            return;
        }
        handler.post(new RunnableC0056WWWWWWWW(17, this, sensorEvent));
    }
}
