package com.android.vmcore;

import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import org.json.JSONArray;
import org.json.JSONObject;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class RomConfig {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public String f8845WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public String f8846WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public int f8847WWWWWWWW;

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public boolean f8848WWWWWWWW;

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public int f8849WWWWWWWW;

    /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
    public String f8850WWWWWWWW;

    /* renamed from: WWWW֭WWWWྥ֭  reason: contains not printable characters */
    public String[] f8851WWWWWWWW;

    /* renamed from: WWWWآWWWWȫآ  reason: contains not printable characters */
    public String f8852WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public int f8853WWWoWWWo;

    /* renamed from: WWWoԻWWWoͷԻ  reason: contains not printable characters */
    public String[] f8854WWWoWWWo;

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public boolean f8855WWoWWo;

    /* renamed from: WWoڢWWo࢞ڢ  reason: not valid java name and contains not printable characters */
    public String f8856WWoWWo;

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public String f8857WWWW;

    /* renamed from: WoڄWoᄴڄ  reason: contains not printable characters */
    public String f8858WoWo;

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static String[] m5046WWWWoWWWWo(JSONObject jSONObject, String str) {
        Object opt = jSONObject.opt(str);
        if (opt instanceof JSONArray) {
            JSONArray jSONArray = (JSONArray) opt;
            int length = jSONArray.length();
            String[] strArr = new String[length];
            for (int i10 = 0; i10 < length; i10++) {
                strArr[i10] = jSONArray.getString(i10);
            }
            return strArr;
        } else if (opt != null) {
            return new String[]{opt.toString()};
        } else {
            return new String[0];
        }
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static RomConfig m5047WWWWWWWW(String str) {
        RomConfig romConfig = new RomConfig();
        JSONObject jSONObject = new JSONObject(str);
        byte[] bArr = {126, -86, -93, -3, -67, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -126, 126};
        StringFog.f8859WWWWWWWW.getClass();
        romConfig.f8846WWWWWWWW = jSONObject.getString(WWWWWWWW.m17835WWWWWWWW(new byte[]{23, -50}, bArr));
        romConfig.f8845WWWWoWWWWo = jSONObject.getString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-105, 91, -70, -30, -55, 9, 90, -123, -99, TarConstants.LF_GNUTYPE_SPARSE, -92, -9}, new byte[]{-13, TarConstants.LF_SYMLINK, -55, -110, -91, 104, 35, -38}));
        romConfig.f8853WWWoWWWo = jSONObject.getInt(WWWWWWWW.m17835WWWWWWWW(new byte[]{-61, -80, 102, 119, -29, 16, 87, -85, -40, -80, 101}, new byte[]{-79, -33, ConstantPoolEntry.CP_InterfaceMethodref, 40, -107, 117, 37, -40}));
        romConfig.f8847WWWWWWWW = jSONObject.getInt(WWWWWWWW.m17835WWWWWWWW(new byte[]{-41, 125, -27, 25, -68, 73, 47, 111, -41, 96}, new byte[]{-72, 14, -70, 111, -39, 59, 92, 6}));
        romConfig.f8848WWWWWWWW = jSONObject.getBoolean(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_SYMLINK, -82, 1, TarConstants.LF_MULTIVOLUME, -30, -9, -90, 123, 32, -24, 67}, new byte[]{65, -37, 113, 61, -115, -123, -46, 36}));
        romConfig.f8855WWoWWo = jSONObject.getBoolean(WWWWWWWW.m17835WWWWWWWW(new byte[]{-58, 87, -32, 47, -60, -32, -54, 45, -44, 20, -92}, new byte[]{-75, 34, -112, 95, -85, -110, -66, 114}));
        romConfig.f8849WWWWWWWW = jSONObject.optInt(WWWWWWWW.m17835WWWWWWWW(new byte[]{-32, -18, -54, 96, -76, -75, -120, 115, -2, -29, -49, 86, -80, -82, -111}, new byte[]{-115, -121, -92, 9, -39, -64, -27, 44}), 0);
        romConfig.f8850WWWWWWWW = jSONObject.optString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-93, 42, 2, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 35, -33, 61, -34, -81, TarConstants.LF_CHR, 28, 78, 56, -49, 34}, new byte[]{-50, 67, 108, 17, 78, -86, 80, -127}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        romConfig.f8854WWWoWWWo = m5046WWWWoWWWWo(jSONObject, WWWWWWWW.m17835WWWWWWWW(new byte[]{16, -60, Byte.MIN_VALUE, -67, -105, 39, 87}, new byte[]{98, -85, -19, -30, -30, 85, 62, 24}));
        romConfig.f8851WWWWWWWW = m5046WWWWoWWWWo(jSONObject, WWWWWWWW.m17835WWWWWWWW(new byte[]{-9, -100, 10, -14, 92, -31, -104, 3, -19, -104, 6}, new byte[]{-104, -22, 111, Byte.MIN_VALUE, TarConstants.LF_NORMAL, Byte.MIN_VALUE, -31, 92}));
        romConfig.f8852WWWWWWWW = jSONObject.optString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-68, TarConstants.LF_SYMLINK, -15, 82, 81, 89, TarConstants.LF_MULTIVOLUME, 90, -93, 58}, new byte[]{-47, TarConstants.LF_GNUTYPE_SPARSE, -106, 59, 34, TarConstants.LF_SYMLINK, 18, 47}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        romConfig.f8857WWWW = jSONObject.optString(WWWWWWWW.m17835WWWWWWWW(new byte[]{Byte.MIN_VALUE, 110, Byte.MIN_VALUE, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -103, 118}, new byte[]{-13, 27, -33, 13, -21, 31, 125, -75}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        romConfig.f8858WoWo = jSONObject.optString(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, 14, 18, 126, -56, 69, 8, -84, 70, 23}, new byte[]{TarConstants.LF_BLK, 126, 125, 13, -83, 33, 87, -39}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        romConfig.f8856WWoWWo = jSONObject.optString(WWWWWWWW.m17835WWWWWWWW(new byte[]{-89, TarConstants.LF_GNUTYPE_SPARSE, -55, 100, 94, -93, -119, 35}, new byte[]{-41, 63, -88, 29, 1, -42, -5, 74}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        return romConfig;
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final String m5048WWWoWWWo() {
        JSONObject jSONObject = new JSONObject();
        StringFog.f8859WWWWWWWW.getClass();
        jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{-98, 29}, new byte[]{-9, 121, -73, -120, 38, -45, -96, 44}), this.f8846WWWWWWWW);
        jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, 19, -38, 38, TarConstants.LF_FIFO, 19, -70, -95, -65, 27, -60, TarConstants.LF_CHR}, new byte[]{-47, 122, -87, 86, 90, 114, -61, -2}), this.f8845WWWWoWWWWo);
        jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{38, -61, -35, -124, 102, 17, -101, 38, 61, -61, -34}, new byte[]{84, -84, -80, -37, 16, 116, -23, 85}), this.f8853WWWoWWWo);
        jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{TarConstants.LF_BLK, 62, -7, 73, -43, -4, -83, -66, TarConstants.LF_BLK, 35}, new byte[]{91, TarConstants.LF_MULTIVOLUME, -90, 63, -80, -114, -34, -41}), this.f8847WWWWWWWW);
        jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{-116, -12, TarConstants.LF_SYMLINK, 6, -18, 26, -13, 113, -98, -78, 112}, new byte[]{-1, -127, 66, 118, -127, 104, -121, 46}), this.f8848WWWWWWWW);
        jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{111, 19, -96, -113, 0, -93, -97, 105, 125, 80, -28}, new byte[]{28, 102, -48, -1, 111, -47, -21, TarConstants.LF_FIFO}), this.f8855WWoWWo);
        jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{-53, 123, -47, -81, 27, -4, 29, -34, -43, 118, -44, -103, 31, -25, 4}, new byte[]{-90, 18, -65, -58, 118, -119, 112, -127}), this.f8849WWWWWWWW);
        jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{-66, 91, 40, 14, 87, -45, -92, -98, -78, 66, TarConstants.LF_FIFO, 56, TarConstants.LF_GNUTYPE_LONGNAME, -61, -69}, new byte[]{-45, TarConstants.LF_SYMLINK, 70, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 58, -90, -55, -63}), this.f8850WWWWWWWW);
        String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(new byte[]{6, 84, 82, 93, -79, -103, -49}, new byte[]{116, 59, 63, 2, -60, -21, -90, 99});
        String[] strArr = this.f8854WWWoWWWo;
        JSONArray jSONArray = new JSONArray();
        for (int i10 = 0; i10 < strArr.length; i10++) {
            jSONArray.put(i10, strArr[i10]);
        }
        jSONObject.put(m17835WWWWWWWW, jSONArray);
        String m17835WWWWWWWW2 = WWWWWWWW.m17835WWWWWWWW(new byte[]{-91, 116, 2, -83, -29, 119, -107, 18, -65, 112, 14}, new byte[]{-54, 2, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -33, -113, 22, -20, TarConstants.LF_MULTIVOLUME});
        String[] strArr2 = this.f8851WWWWWWWW;
        JSONArray jSONArray2 = new JSONArray();
        for (int i11 = 0; i11 < strArr2.length; i11++) {
            jSONArray2.put(i11, strArr2[i11]);
        }
        jSONObject.put(m17835WWWWWWWW2, jSONArray2);
        jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{14, 20, TarConstants.LF_LINK, -71, -111, 126, -60, 93, 17, 28}, new byte[]{99, 117, 86, -48, -30, 21, -101, 40}), this.f8852WWWWWWWW);
        jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{46, -35, 64, 70, -2, 101}, new byte[]{93, -88, 31, TarConstants.LF_CHR, -116, ConstantPoolEntry.CP_NameAndType, -87, -85}), this.f8857WWWW);
        jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{-116, -24, -40, -5, 99, 64, -22, -85, -122, -15}, new byte[]{-12, -104, -73, -120, 6, 36, -75, -34}), this.f8858WoWo);
        jSONObject.put(WWWWWWWW.m17835WWWWWWWW(new byte[]{-86, -49, 82, -10, -99, -104, -89, -68}, new byte[]{-38, -93, TarConstants.LF_CHR, -113, -62, -19, -43, -43}), this.f8856WWoWWo);
        return jSONObject.toString();
    }
}
