package com.android.vmcore.hal;

import android.annotation.SuppressLint;
import android.content.Context;
import android.media.AudioRecord;
import android.media.AudioTrack;
import com.android.vmcore.KLog;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMInstance;
import com.android.vmcore.event.PermissionEvent;
import java.util.ArrayList;
import java.util.List;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import p020WWWWWWWW.AbstractC0211WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class AudioService {
    private static final String TAG;
    private final Context mContext;
    private boolean mMute;
    private long mNativePtr;
    private final VMInstance mVMInstance;
    private List<AudioTrack> mTrackList = new ArrayList();
    private List<AudioRecord> mRecordList = new ArrayList();

    static {
        StringFog.f8859WWWWWWWW.getClass();
        TAG = WWWWWWWW.m17835WWWWWWWW(new byte[]{124, 74, -56, 86, ConstantPoolEntry.CP_InterfaceMethodref, -88, -49, -10, TarConstants.LF_GNUTYPE_LONGLINK, 86, -49, 90}, new byte[]{61, 63, -84, 63, 100, -5, -86, -124});
    }

    public AudioService(Context context, VMInstance vMInstance) {
        this.mContext = context;
        this.mVMInstance = vMInstance;
        this.mNativePtr = nativeSetup(vMInstance.f8937WWWoWWWo.f8866WWWWWWWW);
    }

    @SuppressLint({"MissingPermission"})
    private AudioRecord acquireAudioRecord(int[] iArr) {
        if (!isRecordAudioPermissionGranted()) {
            String str = TAG;
            byte[] bArr = {109, 39, TarConstants.LF_CONTIG, 114, -116, 62, -93, -124, 119, TarConstants.LF_CHR, TarConstants.LF_NORMAL, 106, -106, 5, -76, -126, 89, TarConstants.LF_BLK, TarConstants.LF_NORMAL, 94, -39, 57, -66, -63, 70, 35, 38, 110, -112, 36, -94, -120, 89, 40};
            byte[] bArr2 = {TarConstants.LF_FIFO, 70, 84, 3, -7, 87, -47, -31};
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5041WWWWWWWW(str, WWWWWWWW.m17835WWWWWWWW(bArr, bArr2));
            if (this.mVMInstance.f8939WWWoWWWo.m13946WWWoWWWo(PermissionEvent.class) == null) {
                this.mVMInstance.f8939WWWoWWWo.m13942WWWWWWWW(new PermissionEvent(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_CHR, -37, -124, 104, -58, 65, -77, 114, 34, -48, -110, 119, -64, 91, -92, TarConstants.LF_DIR, 61, -37, -50, 72, -20, 107, -104, 14, 22, -22, -95, 79, -19, 97, -104}, new byte[]{82, -75, -32, 26, -87, 40, -41, 92}), null));
            }
            return null;
        }
        try {
            int minBufferSize = AudioRecord.getMinBufferSize(11025, 2, 2);
            AudioRecord audioRecord = new AudioRecord(1, 11025, 2, 2, minBufferSize);
            audioRecord.startRecording();
            this.mRecordList.add(audioRecord);
            iArr[0] = minBufferSize;
            return audioRecord;
        } catch (Throwable th2) {
            String str2 = TAG;
            byte[] bArr3 = {60, 19, -102, TarConstants.LF_MULTIVOLUME, -16, 13, 112, -5, 38, 7, -99, 85, -22, TarConstants.LF_FIFO, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -3, 8, 0, -99, 97, -91, 1, 122, -3, 2, 2, -115, 85, -22, 10, 56, -66};
            byte[] bArr4 = {TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 114, -7, 60, -123, 100, 2, -98};
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5044WWWoWWWo(str2, WWWWWWWW.m17835WWWWWWWW(bArr3, bArr4), th2);
            return null;
        }
    }

    private AudioTrack acquireAudioTrack(int[] iArr) {
        float maxVolume;
        try {
            int minBufferSize = AudioTrack.getMinBufferSize(44100, 3, 2);
            AudioTrack audioTrack = new AudioTrack(3, 44100, 3, 2, minBufferSize, 1);
            if (this.mMute) {
                maxVolume = 0.0f;
            } else {
                maxVolume = AudioTrack.getMaxVolume();
            }
            audioTrack.setVolume(maxVolume);
            audioTrack.play();
            this.mTrackList.add(audioTrack);
            iArr[0] = minBufferSize;
            return audioTrack;
        } catch (Throwable th2) {
            String str = TAG;
            StringFog.f8859WWWWWWWW.getClass();
            KLog.m5044WWWoWWWo(str, WWWWWWWW.m17835WWWWWWWW(new byte[]{-32, 96, 91, ConstantPoolEntry.CP_InterfaceMethodref, 79, -39, 20, TarConstants.LF_MULTIVOLUME, -6, 116, 92, 19, 85, -28, 20, 73, -40, 106, 101, 90, 95, -56, 5, TarConstants.LF_MULTIVOLUME, -53, 117, 81, 21, 84, -118, 70}, new byte[]{-69, 1, 56, 122, 58, -80, 102, 40}), th2);
            return null;
        }
    }

    private void clearAudioRecord() {
        for (AudioRecord audioRecord : this.mRecordList) {
            try {
                audioRecord.release();
            } catch (Throwable th2) {
                th2.printStackTrace();
            }
        }
        this.mRecordList.clear();
    }

    private void clearAudioTrack() {
        for (AudioTrack audioTrack : this.mTrackList) {
            try {
                audioTrack.release();
            } catch (Throwable th2) {
                th2.printStackTrace();
            }
        }
        this.mTrackList.clear();
    }

    private boolean isRecordAudioPermissionGranted() {
        Context context = this.mContext;
        StringFog.f8859WWWWWWWW.getClass();
        if (AbstractC0211WWWWWWWW.m824WWWWWWWW(context, WWWWWWWW.m17835WWWWWWWW(new byte[]{35, -82, -120, 116, -5, -104, 78, -111, TarConstants.LF_SYMLINK, -91, -98, 107, -3, -126, 89, -42, 45, -82, -62, 84, -47, -78, 101, -19, 6, -97, -83, TarConstants.LF_GNUTYPE_SPARSE, -48, -72, 101}, new byte[]{66, -64, -20, 6, -108, -15, 42, -65})) == 0) {
            return true;
        }
        return false;
    }

    private native void nativeDispose(long j10);

    private native long nativeSetup(int i10);

    private native int nativeStartService(long j10);

    private native int nativeStopService(long j10);

    private int readRecordData(AudioRecord audioRecord, byte[] bArr, int i10, int i11) {
        if (audioRecord != null) {
            try {
                return audioRecord.read(bArr, i10, i11);
            } catch (Throwable th2) {
                th2.printStackTrace();
                return 0;
            }
        }
        return 0;
    }

    private void releaseAudioRecord(AudioRecord audioRecord) {
        if (audioRecord != null) {
            this.mRecordList.remove(audioRecord);
            try {
                audioRecord.release();
            } catch (Throwable th2) {
                th2.printStackTrace();
            }
        }
    }

    private void releaseAudioTrack(AudioTrack audioTrack) {
        if (audioTrack != null) {
            this.mTrackList.remove(audioTrack);
            try {
                audioTrack.release();
            } catch (Throwable th2) {
                th2.printStackTrace();
            }
        }
    }

    private int writeAudioData(AudioTrack audioTrack, byte[] bArr, int i10, int i11) {
        if (audioTrack != null) {
            try {
                return audioTrack.write(bArr, i10, i11);
            } catch (Throwable th2) {
                th2.printStackTrace();
                return 0;
            }
        }
        return 0;
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

    public boolean isMute() {
        return this.mMute;
    }

    public void setMute(boolean z10) {
        float maxVolume;
        this.mMute = z10;
        try {
            for (AudioTrack audioTrack : this.mTrackList) {
                if (this.mMute) {
                    maxVolume = 0.0f;
                } else {
                    maxVolume = AudioTrack.getMaxVolume();
                }
                audioTrack.setVolume(maxVolume);
            }
        } catch (Throwable th2) {
            th2.printStackTrace();
        }
    }

    public int start() {
        return nativeStartService(this.mNativePtr);
    }

    public int stop() {
        clearAudioTrack();
        clearAudioRecord();
        return nativeStopService(this.mNativePtr);
    }

    public void toggleMute() {
        setMute(!this.mMute);
    }
}
