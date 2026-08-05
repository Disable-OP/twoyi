package com.android.vmcore;

import android.content.SharedPreferences;
import com.blankj.utilcode.util.WoWo;
import com.google.firebase.remoteconfig.FirebaseRemoteConfig;
import java.io.File;
import java.util.HashMap;
import java.util.Map;
import java.util.Set;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class VMConfig {

    /* renamed from: WWWWWກWWWWWȝກ  reason: contains not printable characters */
    public String f8860WWWWWWWWWW;

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public String f8861WWWWoWWWWo;

    /* renamed from: WWWWoඤWWWWoెඤ  reason: contains not printable characters */
    public String f8862WWWWoWWWWo;

    /* renamed from: WWWWoᆣWWWWoϿᆣ  reason: contains not printable characters */
    public boolean f8863WWWWoWWWWo;

    /* renamed from: WWWWoᕭWWWWoࢨᕭ  reason: contains not printable characters */
    public HashMap f8864WWWWoWWWWo;

    /* renamed from: WWWWoᜑWWWWoเᜑ  reason: contains not printable characters */
    public boolean f8865WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public int f8866WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public String f8867WWWWWWWW;

    /* renamed from: WWWWϙWWWWეϙ  reason: contains not printable characters */
    public String f8868WWWWWWWW;

    /* renamed from: WWWWҍWWWWּҍ  reason: contains not printable characters */
    public String f8869WWWWWWWW;

    /* renamed from: WWWWӈWWWWीӈ  reason: contains not printable characters */
    public HashMap f8870WWWWWWWW;

    /* renamed from: WWWW֭WWWWྥ֭  reason: contains not printable characters */
    public String f8871WWWWWWWW;

    /* renamed from: WWWWآWWWWȫآ  reason: contains not printable characters */
    public String f8872WWWWWWWW;

    /* renamed from: WWWWܬWWWWೖܬ  reason: contains not printable characters */
    public boolean f8873WWWWWWWW;

    /* renamed from: WWWWެWWWWܕެ  reason: contains not printable characters */
    public String f8874WWWWWWWW;

    /* renamed from: WWWWॾWWWWȏॾ  reason: contains not printable characters */
    public String f8875WWWWWWWW;

    /* renamed from: WWWWമWWWWုമ  reason: contains not printable characters */
    public String f8876WWWWWWWW;

    /* renamed from: WWWWຍWWWWોຍ  reason: contains not printable characters */
    public String f8877WWWWWWWW;

    /* renamed from: WWWWཀྵWWWWࠤཀྵ  reason: contains not printable characters */
    public String f8878WWWWWWWW;

    /* renamed from: WWWWၗWWWW३ၗ  reason: contains not printable characters */
    public String f8879WWWWWWWW;

    /* renamed from: WWWWᄳWWWW़ᄳ  reason: contains not printable characters */
    public String f8880WWWWWWWW;

    /* renamed from: WWWWሎWWWWܣሎ  reason: contains not printable characters */
    public String f8881WWWWWWWW;

    /* renamed from: WWWWሗWWWWшሗ  reason: contains not printable characters */
    public int f8882WWWWWWWW;

    /* renamed from: WWWWኈWWWWˡኈ  reason: contains not printable characters */
    public String f8883WWWWWWWW;

    /* renamed from: WWWWዪWWWWͯዪ  reason: contains not printable characters */
    public int f8884WWWWWWWW;

    /* renamed from: WWWWᏊWWWWటᏊ  reason: contains not printable characters */
    public boolean f8885WWWWWWWW;

    /* renamed from: WWWWᐡWWWWೱᐡ  reason: contains not printable characters */
    public boolean f8886WWWWWWWW;

    /* renamed from: WWWWᓽWWWWϼᓽ  reason: contains not printable characters */
    public boolean f8887WWWWWWWW;

    /* renamed from: WWWWᗘWWWWఛᗘ  reason: contains not printable characters */
    public boolean f8888WWWWWWWW;

    /* renamed from: WWWWᗡWWWWنᗡ  reason: contains not printable characters */
    public boolean f8889WWWWWWWW;

    /* renamed from: WWWWᜐWWWWଙᜐ  reason: contains not printable characters */
    public boolean f8890WWWWWWWW;

    /* renamed from: WWWWᢎWWWWယᢎ  reason: contains not printable characters */
    public String f8891WWWWWWWW;

    /* renamed from: WWWWᣉWWWWঘᣉ  reason: contains not printable characters */
    public String f8892WWWWWWWW;

    /* renamed from: WWWWᨣWWWWۑᨣ  reason: contains not printable characters */
    public String f8893WWWWWWWW;

    /* renamed from: WWWWᬭWWWWɿᬭ  reason: contains not printable characters */
    public String f8894WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public RomConfig f8895WWWoWWWo;

    /* renamed from: WWWoԻWWWoͷԻ  reason: contains not printable characters */
    public boolean f8896WWWoWWWo;

    /* renamed from: WWWoࠟWWWoॣࠟ  reason: contains not printable characters */
    public String f8897WWWoWWWo;

    /* renamed from: WWWoૄWWWoѽૄ  reason: contains not printable characters */
    public String f8898WWWoWWWo;

    /* renamed from: WWWoၙWWWo੮ၙ  reason: contains not printable characters */
    public String f8899WWWoWWWo;

    /* renamed from: WWWoᐣWWWoҁᐣ  reason: contains not printable characters */
    public VMResConfig f8900WWWoWWWo;

    /* renamed from: WWWoᜒWWWo೧ᜒ  reason: contains not printable characters */
    public String f8901WWWoWWWo;

    /* renamed from: WWWᏛWWW෮Ꮫ  reason: contains not printable characters */
    public boolean f8902WWWWWW;

    /* renamed from: WWoϫWWoӉϫ  reason: contains not printable characters */
    public String f8903WWoWWo;

    /* renamed from: WWoڢWWo࢞ڢ  reason: not valid java name and contains not printable characters */
    public String f8904WWoWWo;

    /* renamed from: WWoॹWWoࠔॹ  reason: contains not printable characters */
    public String f8905WWoWWo;

    /* renamed from: WWoহWWoȗহ  reason: contains not printable characters */
    public String f8906WWoWWo;

    /* renamed from: WWo௹WWoਠ௹  reason: contains not printable characters */
    public String f8907WWoWWo;

    /* renamed from: WWoၑWWoړၑ  reason: contains not printable characters */
    public String f8908WWoWWo;

    /* renamed from: WWoᆑWWoӁᆑ  reason: contains not printable characters */
    public String f8909WWoWWo;

    /* renamed from: WWoዕWWoూዕ  reason: contains not printable characters */
    public boolean f8910WWoWWo;

    /* renamed from: WWoᐛWWoʄᐛ  reason: contains not printable characters */
    public boolean f8911WWoWWo;

    /* renamed from: WWoᕛWWoउᕛ  reason: contains not printable characters */
    public boolean f8912WWoWWo;

    /* renamed from: WWo᪙WWoг᪙  reason: contains not printable characters */
    public boolean f8913WWoWWo;

    /* renamed from: WWَWWَཻ  reason: contains not printable characters */
    public String f8914WWWW;

    /* renamed from: WWີWWഫີ  reason: contains not printable characters */
    public String f8915WWWW;

    /* renamed from: WWၚஊၚ  reason: contains not printable characters */
    public String f8916WW;

    /* renamed from: WWኮWWႉኮ  reason: contains not printable characters */
    public String f8917WWWW;

    /* renamed from: WWᐤԂᐤ  reason: contains not printable characters */
    public int f8918WW;

    /* renamed from: WWᩏWWɻᩏ  reason: contains not printable characters */
    public boolean f8919WWWW;

    /* renamed from: WoڄWoᄴڄ  reason: contains not printable characters */
    public String f8920WoWo;

    /* renamed from: WoოWo੍ო  reason: contains not printable characters */
    public boolean f8921WoWo;

    /* renamed from: WoᒧWoᄜᒧ  reason: contains not printable characters */
    public boolean f8922WoWo;

    /* renamed from: Wo᪅Woै᪅  reason: contains not printable characters */
    public String f8923WoWo;

    /* renamed from: oેᄈે  reason: contains not printable characters */
    public String f8924o;

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public static VMConfig m5050WWWWoWWWWo(SharedPreferences sharedPreferences) {
        Object obj;
        VMConfig vMConfig = new VMConfig();
        vMConfig.f8861WWWWoWWWWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-46, 18, -87, -85}, new byte[]{-68, 115, -60, -50, -6, -80, -51, 38}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8895WWWoWWWo = RomConfig.m5047WWWWWWWW(sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-45, 98, -51, -124, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -38, 111, 2, -56, 106}, new byte[]{-95, 13, -96, -37, 4, -75, 1, 100}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING));
        if (StringFog.m5049WWWWWWWW(new byte[]{-65, 45, 101, Byte.MAX_VALUE, 3, 65, -127, 14, -15, 44, 121, 119, TarConstants.LF_PAX_EXTENDED_HEADER_UC, 9, -63, TarConstants.LF_GNUTYPE_LONGNAME, -127, 105, 73, 43, 40, 73, -127, TarConstants.LF_GNUTYPE_LONGNAME, -65, 57, Byte.MAX_VALUE, 105, 28, 85, -44, 72, -82}, new byte[]{-34, 94, 22, 26, 119, 123, -82, 33}).equals(vMConfig.f8895WWWoWWWo.f8852WWWWWWWW)) {
            vMConfig.f8895WWWoWWWo.f8852WWWWWWWW = StringFog.m5049WWWWWWWW(new byte[]{94, 118, -28, 122, -15, 23, Byte.MAX_VALUE, -125, 16, 117, -5, 106, -30, 68, 62, -33, 16, 104, -10, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -20, 94, 59, -126, 69, 108, -25}, new byte[]{63, 5, -105, 31, -123, 45, 80, -84});
        }
        if (StringFog.m5049WWWWWWWW(new byte[]{-48, 38, -101, -101, 97, 41, -81, 63, -98, 39, -121, -109, 58, 97, -17, 125, -18, 98, -73, -49, 74, 33, -81, 99, -60, 37, -115, -116, 96, 96, -27, 98, -97, 47, -127, -114}, new byte[]{-79, 85, -24, -2, 21, 19, Byte.MIN_VALUE, 16}).equals(vMConfig.f8895WWWoWWWo.f8857WWWW)) {
            vMConfig.f8895WWWoWWWo.f8857WWWW = StringFog.m5049WWWWWWWW(new byte[]{-82, -96, -108, -63, -49, 111, -107, 38, -32, -93, -117, -47, -36, 60, -44, 122, -32, -96, -110, -44, -34, 39, -49, 122, -86, -95, -55, -34, -46, 37}, new byte[]{-49, -45, -25, -92, -69, 85, -70, 9});
        }
        if (StringFog.m5049WWWWWWWW(new byte[]{-36, -120, -119, 86, -71, 80, -59, 113, -110, -119, -107, 94, -30, 24, -123, TarConstants.LF_CHR, -30, -52, -91, 2, -110, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -59, 38, -51, -108, -119, 86, -87, 68, -112, TarConstants.LF_CONTIG, -51}, new byte[]{-67, -5, -6, TarConstants.LF_CHR, -51, 106, -22, 94}).equals(vMConfig.f8895WWWoWWWo.f8858WoWo)) {
            vMConfig.f8895WWWoWWWo.f8858WoWo = StringFog.m5049WWWWWWWW(new byte[]{-19, -110, 110, 74, -93, 13, -10, -83, -93, -111, 113, 90, -80, 94, -73, -15, -93, -103, 109, 64, -92, 82, -67, -84, -10, -120, 109}, new byte[]{-116, -31, 29, 47, -41, TarConstants.LF_CONTIG, -39, -126});
        }
        if (StringFog.m5049WWWWWWWW(new byte[]{78, 27, -124, -109, -35, 73, -95, 13, 0, 26, -104, -101, -122, 1, -31, 79, 112, 95, -88, -57, -10, 65, -95, 82, 67, 9, -114, -40, -45, 26, -2}, new byte[]{47, 104, -9, -10, -87, 115, -114, 34}).equals(vMConfig.f8895WWWoWWWo.f8856WWoWWo)) {
            vMConfig.f8895WWWoWWWo.f8856WWoWWo = StringFog.m5049WWWWWWWW(new byte[]{86, 113, 93, -58, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 71, -38, -44, 24, 114, 66, -42, 116, 20, -101, -120, 24, 114, 66, -62, 106, TarConstants.LF_GNUTYPE_SPARSE, -113, -110, 71}, new byte[]{TarConstants.LF_CONTIG, 2, 46, -93, 19, 125, -11, -5});
        }
        vMConfig.f8869WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-57, -71, -73, 115, 104, -28}, new byte[]{-76, -36, -59, 26, 9, -120, -3, -101}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8870WWWWWWWW = new HashMap();
        Map<String, ?> all = sharedPreferences.getAll();
        if (all != null) {
            for (String str : all.keySet()) {
                if (str.startsWith(StringFog.m5049WWWWWWWW(new byte[]{-54, -50, -105, 116, 8}, new byte[]{-70, -68, -8, 4, 38, -51, -58, -84})) && (obj = all.get(str)) != null) {
                    vMConfig.f8870WWWWWWWW.put(str, obj.toString());
                }
            }
        }
        vMConfig.f8896WWWoWWWo = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{101, -9, -99, 123, -57, -27, -69, 35, 100, -21, -84, 121}, new byte[]{ConstantPoolEntry.CP_NameAndType, -124, -62, 28, -76, -120, -28, TarConstants.LF_GNUTYPE_SPARSE}), true);
        vMConfig.f8871WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-54, -64, 17, -95}, new byte[]{-93, -83, 116, -56, -111, -60, -41, -83}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8872WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{81, -74, TarConstants.LF_FIFO, 119, 31, 19, -122}, new byte[]{56, -37, TarConstants.LF_GNUTYPE_SPARSE, 30, 64, 96, -16, -108}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8914WWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-93, 30, 116, -48}, new byte[]{-50, 123, 29, -76, -117, -112, 18, 25}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8920WoWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-121, 106, 94}, new byte[]{-30, 25, TarConstants.LF_NORMAL, -50, -76, TarConstants.LF_BLK, -125, 47}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8904WWoWWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{97, -44, -50, -95, -103, 80, -111, 126, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER}, new byte[]{3, -75, -67, -60, -58, TarConstants.LF_SYMLINK, -16, 16}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8873WWWWWWWW = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{-55, 62, 15, -47, -119, -15, -68, -88, -42, TarConstants.LF_SYMLINK, 6}, new byte[]{-70, 87, 98, -114, -20, -97, -35, -54}), false);
        vMConfig.f8874WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-125, 82, 99, -108, -86, -112, -77, -119, -103, 94, 124}, new byte[]{-16, 59, 14, -53, -55, -15, -63, -5}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8897WWWoWWWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-94, -34, -103, 23, 106, 86, 108}, new byte[]{-47, -73, -12, 72, 25, 38, 2, 61}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8905WWoWWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{63, -72, -5, -26, 113, -121, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 89, 34, -78}, new byte[]{TarConstants.LF_GNUTYPE_LONGNAME, -47, -106, -71, 28, -28, 4, TarConstants.LF_BLK}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8875WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{68, 81, -113, -90, -83, 35, 92, -16, TarConstants.LF_GNUTYPE_SPARSE}, new byte[]{TarConstants.LF_CONTIG, 56, -30, -7, -60, 64, 63, -103}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8906WWoWWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-87, 40, -108, -102, -40, -72, -59, 15}, new byte[]{-38, 65, -7, -59, -79, -43, -74, 102}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8898WWWoWWWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-13, -56, 2, 62, 63, 40, -49, -78, -27, -2, 1, 20, 34, 34, -59, -82}, new byte[]{Byte.MIN_VALUE, -95, 111, 97, 79, 64, -96, -36}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8924o = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-3, 104, 36, 122, -109, 38, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -37, -7, 119, 36, 102, -99, 38, 85, -33, -1, 114, 34, 113, -124}, new byte[]{-115, 0, TarConstants.LF_GNUTYPE_LONGLINK, 20, -10, 121, TarConstants.LF_FIFO, -66}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8907WWoWWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{31, -26, 40, TarConstants.LF_MULTIVOLUME, 119, -86, -64, 81, 27, -7, 40, 81, 121, -86, -35, 68, 1}, new byte[]{111, -114, 71, 35, 18, -11, -82, TarConstants.LF_BLK}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8876WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{111, 43, -21, 46, -45, -17, -110, TarConstants.LF_LINK, 107, TarConstants.LF_BLK, -21, TarConstants.LF_SYMLINK, -35, -17, -111, TarConstants.LF_CONTIG, 124, 46, -22, 35}, new byte[]{31, 67, -124, 64, -74, -80, -4, 84}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8862WWWWoWWWWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{2, -3, -6, -3, 37, 44, -75, 124, 6, -30, -6, -31, 43, 44, -81, 96, 2, -16}, new byte[]{114, -107, -107, -109, 64, 115, -37, 25}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8860WWWWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-75, -56, 81, 22, 40, 109, -71, -119, -94, -50, 95, 20, 18, 65, -66, -110, -96, -50, 89, ConstantPoolEntry.CP_NameAndType, 37}, new byte[]{-59, -96, 62, TarConstants.LF_PAX_EXTENDED_HEADER_LC, TarConstants.LF_MULTIVOLUME, TarConstants.LF_SYMLINK, -54, -32}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8877WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{24, -51, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -13, 118, 67, -116, -81, 28, -46, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -17, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 67, -111, -66, 9, -47, 125, -18}, new byte[]{104, -91, 8, -99, 19, 28, -30, -54}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8915WWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_GNUTYPE_LONGLINK, 2, -64, -84, 116, -119, TarConstants.LF_SYMLINK, 79, 72, TarConstants.LF_DIR, -64, -78, 101, -65, 46, TarConstants.LF_GNUTYPE_LONGNAME}, new byte[]{59, 106, -81, -62, 17, -42, 65, 34}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8878WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-126, -65, 97, 80, 60, -108, -18, 63, -109, -69, 81, 81, 41, -65, -29, 57, -100}, new byte[]{-14, -41, 14, 62, 89, -53, -118, 86}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8908WWoWWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{70, 29, -36, 28, -27, 41, -12, -97, 85}, new byte[]{TarConstants.LF_LINK, 116, -70, 117, -70, 90, -121, -10}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8879WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{40, 82, -22, 85, -102, 44, 9, 62, TarConstants.LF_FIFO, 95}, new byte[]{95, 59, -116, 60, -59, 78, 122, TarConstants.LF_MULTIVOLUME}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8916WW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-54, -90, -120, -30, -21, -82, -57}, new byte[]{-67, -49, -18, -117, -76, -57, -73, -42}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8899WWWoWWWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{18, 2, 38, ConstantPoolEntry.CP_NameAndType, 25, -28, -42, 65}, new byte[]{101, 107, 64, 101, 70, -119, -73, 34}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8921WoWo = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{122, -33, 69, -58, 21, -107, 64, -113, 111, -34, 123, -40, 25, -71, 67}, new byte[]{9, -73, 36, -76, 112, -54, TarConstants.LF_CONTIG, -26}), false);
        vMConfig.f8909WWoWWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{84, -2, 112, -17, -53, 124, -29, 6}, new byte[]{56, -111, 19, -114, -89, 35, -118, 118}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8880WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-18, -78, 123, 105, 70, 68, 84, 80, -31}, new byte[]{-126, -35, 24, 8, 42, 27, 57, TarConstants.LF_LINK}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8863WWWWoWWWWo = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{-1, 98, TarConstants.LF_GNUTYPE_LONGLINK, 106, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 37, 70, -113, -30, 108, 74, 109, 110, 116}, new byte[]{-116, 13, 40, 1, ConstantPoolEntry.CP_InterfaceMethodref, 16, 25, -22}), false);
        vMConfig.f8881WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{57, -21, -30, 26, 10, -100, 16, 74, 47, -10, -9, 20, ConstantPoolEntry.CP_InterfaceMethodref}, new byte[]{74, -124, -127, 113, 121, -87, 79, 57}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8882WWWWWWWW = sharedPreferences.getInt(StringFog.m5049WWWWWWWW(new byte[]{-40, 87, -30, -5, -21, -14, -45, -85, -60, 74, -11}, new byte[]{-85, 56, -127, -112, -104, -57, -116, -37}), -1);
        vMConfig.f8883WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{84, -44, 28, -55, 47, -80, -110, 46, 84, -34, 13, -52, 61, -24, -88}, new byte[]{39, -69, Byte.MAX_VALUE, -94, 92, -123, -51, 91}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8917WWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-104, -111, -52, 5, -83, -84, 91, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -118, -115, -36, 25, -79, -21, 96}, new byte[]{-21, -2, -81, 110, -34, -103, 4, 8}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8910WWoWWo = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{-32, 57, -118, -40, -59, -55, -94, -95, -19, 56, -116}, new byte[]{-127, 93, -24, -121, -96, -89, -61, -61}), false);
        vMConfig.f8885WWWWWWWW = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{-118, -6, -96, -119, -102, 31, 80, 28, -119, -6, -91, -116, -116, 16}, new byte[]{-25, -101, -57, -32, -23, 116, 15, 121}), false);
        vMConfig.f8902WWWWWW = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{-12, 64, 1, TarConstants.LF_SYMLINK, -67, 106, -41, 41, -28, 67, ConstantPoolEntry.CP_InterfaceMethodref, 34}, new byte[]{-122, 47, 110, 70, -30, 15, -71, 72}), false);
        vMConfig.f8911WWoWWo = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 91, -114, -89, 121, -75, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -24, 113, 74, -125, -72, 121, -75}, new byte[]{31, 43, -31, -44, 28, -47, 7, -115}), false);
        vMConfig.f8886WWWWWWWW = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{-60, 109, 114, 119, -19, 60, -15, 31, -42, 109, 118, 106}, new byte[]{-76, 1, 19, 14, -78, 89, -97, 126}), false);
        VMResConfig vMResConfig = new VMResConfig();
        vMResConfig.f8953WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-13, -82, 111, 99, -84, 58, -71, 13, -12, -88, 114, 117, -87, 60, -97, 60, -10, -86, 121}, new byte[]{-105, -57, 28, 19, -64, 91, -64, 82}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMResConfig.f8952WWWWoWWWWo = sharedPreferences.getInt(StringFog.m5049WWWWWWWW(new byte[]{95, -124, 74, -87, -102, -73, -41, 82, TarConstants.LF_GNUTYPE_LONGNAME, -124, 93, -83, -98}, new byte[]{59, -19, 57, -39, -10, -42, -82, 13}), 0);
        vMResConfig.f8955WWWoWWWo = sharedPreferences.getInt(StringFog.m5049WWWWWWWW(new byte[]{-60, -14, -69, -41, 42, -80, -66, 37, -56, -2, -95, -64, 46, -91}, new byte[]{-96, -101, -56, -89, 70, -47, -57, 122}), 0);
        vMResConfig.f8954WWWWWWWW = sharedPreferences.getInt(StringFog.m5049WWWWWWWW(new byte[]{-82, -111, -111, -84, -95, 85, -105, 59, -82, -120, -117}, new byte[]{-54, -8, -30, -36, -51, TarConstants.LF_BLK, -18, 100}), 0);
        vMConfig.f8900WWWoWWWo = vMResConfig;
        vMConfig.f8918WW = sharedPreferences.getInt(StringFog.m5049WWWWWWWW(new byte[]{19, -78, 43, -96, -22, -1, -19, TarConstants.LF_PAX_EXTENDED_HEADER_LC, 19, -74, 57, -73}, new byte[]{97, -41, TarConstants.LF_MULTIVOLUME, -46, -113, -116, -123, 39}), 60);
        vMConfig.f8922WoWo = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{-52, -5, 110, 58, -17, -67, -107, 121, -48, -15, Byte.MAX_VALUE, TarConstants.LF_CONTIG, -40, -121, -120, 123, -64, -8, Byte.MAX_VALUE}, new byte[]{-94, -108, 26, 89, -121, -30, -26, 26}), true);
        vMConfig.f8887WWWWWWWW = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{0, 80, -126, 84, 97, 57, 90, -24, ConstantPoolEntry.CP_NameAndType, 68, -99, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 119, TarConstants.LF_LINK, 70}, new byte[]{98, 37, -21, 56, 21, 80, TarConstants.LF_BLK, -73}), true);
        vMConfig.f8912WWoWWo = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{63, 3, -56, 99, 22, -30, -39, 41, TarConstants.LF_CHR, 23, -41, 80, 0, -22, -59, 41, 47, 2, -51}, new byte[]{93, 118, -95, 15, 98, -117, -73, 118}), false);
        vMConfig.f8864WWWWoWWWWo = new HashMap();
        Set<String> stringSet = sharedPreferences.getStringSet(StringFog.m5049WWWWWWWW(new byte[]{-18, -112, 110, 73, -99, -16, 87, -46, -4, -103, 117, 95, -127}, new byte[]{-99, -11, 0, 58, -14, -126, 8, -92}), null);
        if (stringSet != null) {
            for (String str2 : stringSet) {
                vMConfig.f8864WWWWoWWWWo.put(str2.split(StringFog.m5049WWWWWWWW(new byte[]{-41}, new byte[]{-22, -32, TarConstants.LF_NORMAL, 59, -34, 27, -110, TarConstants.LF_PAX_EXTENDED_HEADER_LC}))[0], str2.split(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_PAX_EXTENDED_HEADER_UC}, new byte[]{101, 126, -51, 10, -51, -103, 86, 109}))[1]);
            }
        } else {
            vMConfig.f8864WWWWoWWWWo.put(StringFog.m5049WWWWWWWW(new byte[]{-23, 66, -47, 20, TarConstants.LF_MULTIVOLUME, 114, 87, -39, -5, 73, -37, 21, TarConstants.LF_MULTIVOLUME, 105, 29, -112, -15, 94, -38, 21, 65, 116, 67, -110}, new byte[]{-120, 44, -75, 102, 34, 27, TarConstants.LF_CHR, -9}), sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{2, Byte.MAX_VALUE, 107, TarConstants.LF_SYMLINK, -65, -38, -74, 124, 7, 106, 124, 57}, new byte[]{101, 6, 25, 93, -32, -65, -40, 29}), false) ? StringFog.m5049WWWWWWWW(new byte[]{94, -39, -79, -112, 40, -32, 82, -5, 72, -61, -81, -120}, new byte[]{56, -84, -35, -4, 119, -109, 39, -117}) : StringFog.m5049WWWWWWWW(new byte[]{-2, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -2, -21, 62, 71, 113, -13, -1, 101, -2}, new byte[]{-112, 23, -118, -76, TarConstants.LF_MULTIVOLUME, TarConstants.LF_SYMLINK, 1, -125}));
            vMConfig.f8864WWWWoWWWWo.put(StringFog.m5049WWWWWWWW(new byte[]{64, -29, 98, -47, 114, -111, 91, -10, 82, -24, 104, -48, 114, -118, 17, -71, 66, -18, 99, -49, TarConstants.LF_PAX_EXTENDED_HEADER_LC, -118, 80, -75, 68, -7, 99, -47}, new byte[]{33, -115, 6, -93, 29, -8, 63, -40}), sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{99, 68, 97, 30, -43, -104, -47, -86, 111, 66, 118, 30, -53, -94, -58, -85, 99, 69, 110, 30, -35}, new byte[]{2, 39, 2, 123, -71, -3, -93, -59}), false) ? StringFog.m5049WWWWWWWW(new byte[]{110, 101, -12, 69, 33, -21, -22, 7, TarConstants.LF_PAX_EXTENDED_HEADER_LC, Byte.MAX_VALUE, -22, 93}, new byte[]{8, 16, -104, 41, 126, -104, -97, 119}) : StringFog.m5049WWWWWWWW(new byte[]{102, 117, TarConstants.LF_CHR, 87, -12, -114, TarConstants.LF_DIR, -93, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, 104, TarConstants.LF_CHR}, new byte[]{8, 26, 71, 8, -121, -5, 69, -45}));
        }
        vMConfig.f8888WWWWWWWW = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{-46, -88, 4, -80, 13, 91, -88, -94, -33, -88, ConstantPoolEntry.CP_InterfaceMethodref, -71, 26, 94}, new byte[]{-79, -55, 105, -43, Byte.MAX_VALUE, 58, -9, -57}), false);
        vMConfig.f8889WWWWWWWW = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{-59, -98, -17, 34, 40, -65, 57, 16, -33, -122, -20, 63, 44, -110, 62}, new byte[]{-74, -10, -114, 80, TarConstants.LF_MULTIVOLUME, -32, 90, 124}), true);
        vMConfig.f8890WWWWWWWW = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{-60, -124, -42, TarConstants.LF_PAX_EXTENDED_HEADER_UC, TarConstants.LF_DIR, TarConstants.LF_SYMLINK, -63, 46, -37, -120, -46, TarConstants.LF_PAX_EXTENDED_HEADER_UC}, new byte[]{-73, -20, -73, 42, 80, 109, -89, 65}), false);
        vMConfig.f8865WWWWoWWWWo = sharedPreferences.getBoolean(StringFog.m5049WWWWWWWW(new byte[]{-92, 14, -53, 79, -48, 71, 70, -54, -93, 15, -52, 84, -42, 121, 92, -52, -72, 8}, new byte[]{-41, 102, -86, 61, -75, 24, 40, -91}), false);
        vMConfig.f8901WWWoWWWo = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-90, 89, -14, -77, 21, 43, -35, 113, -107, 66, -24, -94, 4}, new byte[]{-54, TarConstants.LF_FIFO, -111, -46, 97, 66, -78, 31}), StringFog.m5049WWWWWWWW(new byte[]{-89, -6, 37, -16, -111, 47, 3, -61, -90, -2, 63, -19, -101, 39, 57}, new byte[]{-61, -97, TarConstants.LF_GNUTYPE_SPARSE, -103, -14, 74, 92, -79}));
        vMConfig.f8891WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-50, 10, -121, 87, 98, 60, 116, 32, -38, 13, -117, 74, TarConstants.LF_GNUTYPE_SPARSE}, new byte[]{-69, 121, -30, 37, 61, 80, 27, 67}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8892WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{TarConstants.LF_DIR, 4, 7, 40, -102, -84, 93, -4, 61, 6}, new byte[]{82, 116, 114, 119, -20, -55, TarConstants.LF_CHR, -104}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        vMConfig.f8893WWWWWWWW = sharedPreferences.getString(StringFog.m5049WWWWWWWW(new byte[]{-46, -70, 2, -73, -20, 116, 40, 19, -48, -72, 18, -102}, new byte[]{-75, -54, 119, -24, -98, 17, 70, 119}), FirebaseRemoteConfig.DEFAULT_VALUE_FOR_STRING);
        return vMConfig;
    }

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public static VMConfig m5051WWWWWWWW(File file) {
        byte[] bArr = {71, 94, -121, 19, -124, 72, 5, 82, 71, 64, -109, 79, -72, 73, 0, 14, 67, 84, -77, 19, -114, 71, 4, 14, 67, 94, Byte.MIN_VALUE, 4, -104, 104, ConstantPoolEntry.CP_NameAndType, ConstantPoolEntry.CP_NameAndType, 74};
        byte[] bArr2 = {38, TarConstants.LF_NORMAL, -29, 97, -21, 33, 97, 124};
        StringFog.f8859WWWWWWWW.getClass();
        return m5050WWWWoWWWWo((SharedPreferences) WoWo.m5357WoWo(WWWWWWWW.m17835WWWWWWWW(bArr, bArr2)).m5362WWWWWWWW(file, 0).f9408WWWWoWWWWo);
    }
}
