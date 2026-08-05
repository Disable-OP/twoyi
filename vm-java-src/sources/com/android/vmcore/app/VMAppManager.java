package com.android.vmcore.app;

import android.text.TextUtils;
import com.android.vmapp.VMApp;
import com.android.vmcore.IAppManager;
import com.android.vmcore.StringFog;
import com.android.vmcore.VMConfig;
import com.android.vmcore.VMInstance;
import com.android.vmcore.bridge.IVMEventCallback;
import com.android.vmcore.bridge.VMEventManager;
import com.android.vmcore.event.AppAddEvent;
import com.android.vmcore.event.AppDelEvent;
import com.android.vmcore.utils.FileDeleteUtils;
import com.clone.android.dual.space.R;
import eh.C2467WWWWWWWW;
import java.io.File;
import java.util.ArrayList;
import org.apache.commons.compress.archivers.tar.TarConstants;
import org.apache.commons.compress.harmony.unpack200.bytecode.ConstantPoolEntry;
import vf.AbstractC4470WWWWWWWW;
import x5.WWWWWWWW;
/* loaded from: classes.dex */
public class VMAppManager implements IAppManager, IVMEventCallback {

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final VMConfig f8980WWWWoWWWWo;

    /* renamed from: WWWW̏WWWWβ̏  reason: contains not printable characters */
    public final VMApp f8981WWWWWWWW;

    /* renamed from: WWWWͶWWWWᆑͶ  reason: contains not printable characters */
    public final VMEventManager f8982WWWWWWWW;

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final VMInstance f8983WWWoWWWo;

    public VMAppManager(VMApp vMApp, VMInstance vMInstance) {
        this.f8981WWWWWWWW = vMApp;
        this.f8983WWWoWWWo = vMInstance;
        this.f8980WWWWoWWWWo = vMInstance.f8937WWWoWWWo;
        VMEventManager vMEventManager = vMInstance.f8935WWWWWWWW;
        this.f8982WWWWWWWW = vMEventManager;
        ArrayList arrayList = vMEventManager.f8989WWWWWWWW;
        if (!arrayList.contains(this)) {
            arrayList.add(this);
        }
    }

    /* renamed from: WWWWo̐WWWWoȄ̐  reason: contains not printable characters */
    public final void m5110WWWWoWWWWo(String str, ArrayList arrayList) {
        int i10 = 0;
        this.f8983WWWoWWWo.f8939WWWoWWWo.m13940WWWWWWWW(new AppAddEvent(1, str));
        ArrayList arrayList2 = new ArrayList();
        int size = arrayList.size();
        while (i10 < size) {
            Object obj = arrayList.get(i10);
            i10++;
            StringBuilder sb2 = new StringBuilder();
            WWWWWWWW wwwwwwww = StringFog.f8859WWWWWWWW;
            arrayList2.add(AbstractC4470WWWWWWWW.m17683WWWWWWWW(wwwwwwww, new byte[]{-70, -86, -8, 89, -110, 111, 93}, new byte[]{-107, -36, -107, 41, -13, 28, 46, -65}, sb2, (String) obj));
        }
        VMEventManager vMEventManager = this.f8982WWWWWWWW;
        if (vMEventManager != null) {
            String join = TextUtils.join(" ", arrayList2);
            byte[] bArr = {-1, 125, 63, -64, -33, -9, 40, -71, -13, 123, TarConstants.LF_FIFO, -64, -56, -12, 47, -92, -18, 119, 124, -113, -35, -19, 37, -92, -14, 60, 1, -70, -1, -53, 24, -108, -43, 92, 1, -70, -1, -43, 0, -108, -35, 66, 2};
            byte[] bArr2 = {-100, 18, 82, -18, -66, -103, TarConstants.LF_GNUTYPE_LONGNAME, -53};
            StringFog.f8859WWWWWWWW.getClass();
            String m17835WWWWWWWW = WWWWWWWW.m17835WWWWWWWW(bArr, bArr2);
            vMEventManager.m5116WWWoWWWo(m17835WWWWWWWW, str + " " + join);
        }
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Removed duplicated region for block: B:24:0x0075  */
    /* JADX WARN: Removed duplicated region for block: B:68:0x01e2  */
    /* JADX WARN: Removed duplicated region for block: B:70:0x01eb  */
    /* JADX WARN: Removed duplicated region for block: B:98:? A[RETURN, SYNTHETIC] */
    /* JADX WARN: Type inference failed for: r1v3, types: [com.android.vmcore.event.AppDelEvent, java.lang.Object] */
    /* JADX WARN: Type inference failed for: r1v4, types: [com.android.vmcore.event.AppAddEvent, java.lang.Object] */
    @Override // com.android.vmcore.bridge.IVMEventCallback
    /* renamed from: WWWW̏WWWWβ̏ */
    /*
        Code decompiled incorrectly, please refer to instructions dump.
    */
    public final void mo5013WWWWWWWW(String str, String str2) {
        int parseInt;
        String string;
        byte[] bArr = {107, -107, 121, -93, 109, -122, Byte.MIN_VALUE, -82, TarConstants.LF_PAX_GLOBAL_EXTENDED_HEADER, -109, 112, -93, 122, -123, -121, -77, 122, -97, 58, -20, 111, -100, -115, -77, 102, -44, 93, -61, 95, -68, -91, -112, 68, -91, 85, -35, 92, -73, -74, -103, 91, -81, TarConstants.LF_PAX_EXTENDED_HEADER_UC, -39};
        byte[] bArr2 = {8, -6, 20, -115, ConstantPoolEntry.CP_NameAndType, -24, -28, -36};
        StringFog.f8859WWWWWWWW.getClass();
        boolean equals = WWWWWWWW.m17835WWWWWWWW(bArr, bArr2).equals(str);
        VMInstance vMInstance = this.f8983WWWoWWWo;
        if (equals) {
            if (!TextUtils.isEmpty(str2)) {
                String[] split = str2.split(" ");
                String str3 = split[0];
                if (split.length == 2) {
                    try {
                        parseInt = Integer.parseInt(split[1]);
                    } catch (Throwable th2) {
                        th2.printStackTrace();
                        parseInt = -500;
                        VMApp vMApp = this.f8981WWWWWWWW;
                        if (parseInt != -115) {
                        }
                        if (parseInt != 1) {
                        }
                    }
                } else {
                    try {
                        String str4 = split[2];
                        if (str4.endsWith(WWWWWWWW.m17835WWWWWWWW(new byte[]{92}, new byte[]{102, -10, 106, -79, 44, -4, -91, -27}))) {
                            str4 = str4.substring(0, str4.length() - 1);
                        }
                        parseInt = InstallAppResult.m5109WWWWWWWW(str4);
                    } catch (Throwable th3) {
                        th3.printStackTrace();
                        parseInt = -500;
                        VMApp vMApp2 = this.f8981WWWWWWWW;
                        if (parseInt != -115) {
                        }
                        if (parseInt != 1) {
                        }
                    }
                }
                VMApp vMApp22 = this.f8981WWWWWWWW;
                if (parseInt != -115) {
                    if (parseInt != 1) {
                        switch (parseInt) {
                            case -113:
                                string = vMApp22.getString(R.string.install_failed_no_matching_abis_msg);
                                break;
                            case -112:
                                string = vMApp22.getString(R.string.install_failed_duplicate_permission_msg);
                                break;
                            case -111:
                                string = vMApp22.getString(R.string.install_failed_user_restricted_msg);
                                break;
                            case -110:
                                string = vMApp22.getString(R.string.install_failed_internal_error_msg);
                                break;
                            case -109:
                                string = vMApp22.getString(R.string.install_parse_failed_manifest_empty_msg);
                                break;
                            case -108:
                                string = vMApp22.getString(R.string.install_parse_failed_manifest_malformed_msg);
                                break;
                            case -107:
                                string = vMApp22.getString(R.string.install_parse_failed_bad_shared_user_id_msg);
                                break;
                            case -106:
                                string = vMApp22.getString(R.string.install_parse_failed_bad_package_name_msg);
                                break;
                            case -105:
                                string = vMApp22.getString(R.string.install_parse_failed_certificate_encoding_msg);
                                break;
                            case -104:
                                string = vMApp22.getString(R.string.install_parse_failed_inconsistent_certificates_msg);
                                break;
                            case -103:
                                string = vMApp22.getString(R.string.install_parse_failed_no_certificates_msg);
                                break;
                            case -102:
                                string = vMApp22.getString(R.string.install_parse_failed_unexpected_exception_msg);
                                break;
                            case -101:
                                string = vMApp22.getString(R.string.install_parse_failed_bad_manifest_msg);
                                break;
                            case -100:
                                string = vMApp22.getString(R.string.install_parse_failed_not_apk_msg);
                                break;
                            default:
                                switch (parseInt) {
                                    case -25:
                                        string = vMApp22.getString(R.string.install_failed_version_downgrade_msg);
                                        break;
                                    case -24:
                                        string = vMApp22.getString(R.string.install_failed_uid_changed_msg);
                                        break;
                                    case -23:
                                        string = vMApp22.getString(R.string.install_failed_package_changed_msg);
                                        break;
                                    case -22:
                                        string = vMApp22.getString(R.string.install_failed_verification_failure_msg);
                                        break;
                                    case -21:
                                        string = vMApp22.getString(R.string.install_failed_verification_timeout_msg);
                                        break;
                                    case -20:
                                        string = vMApp22.getString(R.string.install_failed_media_unavailable_msg);
                                        break;
                                    case -19:
                                        string = vMApp22.getString(R.string.install_failed_invalid_install_location_msg);
                                        break;
                                    case -18:
                                        string = vMApp22.getString(R.string.install_failed_container_error_msg);
                                        break;
                                    case -17:
                                        string = vMApp22.getString(R.string.install_failed_missing_feature_msg);
                                        break;
                                    case -16:
                                        string = vMApp22.getString(R.string.install_failed_cpu_abi_incompatible_msg);
                                        break;
                                    case -15:
                                        string = vMApp22.getString(R.string.install_failed_test_only_msg);
                                        break;
                                    case -14:
                                        string = vMApp22.getString(R.string.install_failed_newer_sdk_msg);
                                        break;
                                    case -13:
                                        string = vMApp22.getString(R.string.install_failed_conflicting_provider_msg);
                                        break;
                                    case -12:
                                        string = vMApp22.getString(R.string.install_failed_older_sdk_msg);
                                        break;
                                    case -11:
                                        string = vMApp22.getString(R.string.install_failed_dexopt_msg);
                                        break;
                                    case -10:
                                        string = vMApp22.getString(R.string.install_failed_replace_couldnt_delete_msg);
                                        break;
                                    case -9:
                                        string = vMApp22.getString(R.string.install_failed_missing_shared_library_msg);
                                        break;
                                    case -8:
                                        string = vMApp22.getString(R.string.install_failed_shared_user_incompatible_msg);
                                        break;
                                    case -7:
                                        string = vMApp22.getString(R.string.install_failed_update_incompatible_msg);
                                        break;
                                    case -6:
                                        string = vMApp22.getString(R.string.install_failed_no_shared_user_msg);
                                        break;
                                    case -5:
                                        string = vMApp22.getString(R.string.install_failed_duplicate_package_msg);
                                        break;
                                    case -4:
                                        string = vMApp22.getString(R.string.install_failed_insufficient_storage_msg);
                                        break;
                                    case -3:
                                        string = vMApp22.getString(R.string.install_failed_invalid_uri_msg);
                                        break;
                                    case -2:
                                        string = vMApp22.getString(R.string.install_failed_invalid_apk_msg);
                                        break;
                                    case -1:
                                        string = vMApp22.getString(R.string.install_failed_already_exists_msg);
                                        break;
                                    default:
                                        string = vMApp22.getString(R.string.install_failed_unknown_msg);
                                        break;
                                }
                        }
                    } else {
                        string = vMApp22.getString(R.string.install_succeeded_msg);
                    }
                } else {
                    string = vMApp22.getString(R.string.install_failed_aborted_msg);
                }
                if (parseInt != 1) {
                    C2467WWWWWWWW c2467wwwwwwww = vMInstance.f8939WWWoWWWo;
                    ?? obj = new Object();
                    obj.f9001WWWWWWWW = 2;
                    obj.f9002WWWoWWWo = str3;
                    obj.f9000WWWWoWWWWo = string;
                    c2467wwwwwwww.m13940WWWWWWWW(obj);
                }
            }
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{14, -68, 65, -64, 109, -39, -40, 125, 2, -70, 72, -64, 122, -38, -33, 96, 31, -74, 2, -113, 111, -61, -43, 96, 3, -3, 121, -96, 69, -7, -17, 91, 44, -97, 96, -79, TarConstants.LF_MULTIVOLUME, -25, -20, 80, 63, -106, Byte.MAX_VALUE, -69, 64, -29}, new byte[]{109, -45, 44, -18, ConstantPoolEntry.CP_NameAndType, -73, -68, 15}).equals(str)) {
            if (!TextUtils.isEmpty(str2)) {
                String[] split2 = str2.split(" ");
                String str5 = split2[0];
                if (Integer.parseInt(split2[1]) != 1) {
                    C2467WWWWWWWW c2467wwwwwwww2 = vMInstance.f8939WWWoWWWo;
                    ?? obj2 = new Object();
                    obj2.f9004WWWWWWWW = 2;
                    obj2.f9003WWWWoWWWWo = str5;
                    c2467wwwwwwww2.m13940WWWWWWWW(obj2);
                }
            }
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{106, 6, -33, -44, -118, -118, 73, -106, 98, 6, -49, -61, -117, -105, 3, -39, 104, 28, -46, -55, -117, -51, 125, -7, 72, 35, -6, -31, -96, -68, 108, -4, 79, 45, -1}, new byte[]{ConstantPoolEntry.CP_InterfaceMethodref, 104, -69, -90, -27, -29, 45, -72}).equals(str)) {
            if (!TextUtils.isEmpty(str2)) {
                vMInstance.f8939WWWoWWWo.m13940WWWWWWWW(new AppAddEvent(0, str2.split(" ")[0]));
            }
        } else if (WWWWWWWW.m17835WWWWWWWW(new byte[]{-75, -120, -111, -9, -53, -97, TarConstants.LF_GNUTYPE_LONGLINK, -8, -67, -120, -127, -32, -54, -126, 1, -73, -73, -110, -100, -22, -54, -40, Byte.MAX_VALUE, -105, -105, -83, -76, -62, -31, -87, 125, -109, -103, -87, -93, -64, -32}, new byte[]{-44, -26, -11, -123, -92, -10, 47, -42}).equals(str) && !TextUtils.isEmpty(str2)) {
            vMInstance.f8939WWWoWWWo.m13940WWWWWWWW(new AppDelEvent(0, str2.split(" ")[0]));
        }
    }

    /* renamed from: WWWȏWWWoನ̑  reason: contains not printable characters */
    public final void m5111WWWoWWWo(String str) {
        this.f8983WWWoWWWo.f8939WWWoWWWo.m13940WWWWWWWW(new AppDelEvent(1, str));
        VMEventManager vMEventManager = this.f8982WWWWWWWW;
        if (vMEventManager != null) {
            StringFog.f8859WWWWWWWW.getClass();
            vMEventManager.m5116WWWoWWWo(WWWWWWWW.m17835WWWWWWWW(new byte[]{-101, -101, -77, 69, -74, TarConstants.LF_LINK, 33, 28, -105, -99, -70, 69, -95, TarConstants.LF_SYMLINK, 38, 1, -118, -111, -16, 10, -76, 43, 44, 1, -106, -38, -115, 63, -106, 13, 17, TarConstants.LF_LINK, -83, -70, -105, 37, -124, ConstantPoolEntry.CP_InterfaceMethodref, 4, 34, -76, -85, -97, 59, -121}, new byte[]{-8, -12, -34, 107, -41, 95, 69, 110}), str);
        }
        String str2 = this.f8980WWWWoWWWWo.f8868WWWWWWWW;
        StringBuilder sb2 = new StringBuilder();
        FileDeleteUtils.m5262WWWWWWWW(new File(str2, AbstractC4470WWWWWWWW.m17683WWWWWWWW(StringFog.f8859WWWWWWWW, new byte[]{-110, -16, -93, -92, 68, -78, -17, -56, -4, -19, -93, -75, 74, -87, -17, -56, -46, -31, -91, -24}, new byte[]{-67, -125, -57, -57, 37, -64, -117, -25}, sb2, str)));
    }
}
