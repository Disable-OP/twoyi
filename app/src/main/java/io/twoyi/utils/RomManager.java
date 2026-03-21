/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.utils;

import android.content.Context;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.os.Build;
import android.os.Process;
import android.util.DisplayMetrics;
import android.util.Log;

import com.topjohnwu.superuser.Shell;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.FileWriter;
import java.io.IOException;
import java.io.InputStream;
import java.io.Writer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.Enumeration;
import java.util.Locale;
import java.util.Properties;
import java.util.TimeZone;
import java.util.zip.Adler32;
import java.util.zip.CRC32;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;
import java.util.zip.ZipOutputStream;

/**
 * @author weishu
 * @date 2021/10/22.
 */

public final class RomManager {

    private static final String TAG = "RomManager";

    private static final String ROM_INFO_FILE = "rom.ini";

    private static final String DEFAULT_INFO = "unknown";

    private static final String LOADER_FILE = "libloader.so";

    private RomManager() {
    }

    public static void initRootfs(Context context) {
        File propFile = getVendorPropFile(context);
        String language = Locale.getDefault().getLanguage();
        String country = Locale.getDefault().getCountry();

        Properties properties = new Properties();

        properties.setProperty("persist.sys.language", language);
        properties.setProperty("persist.sys.country", country);

        TimeZone timeZone = TimeZone.getDefault();
        String timeZoneID = timeZone.getID();
        Log.i(TAG, "timezone: " + timeZoneID);
        properties.setProperty("persist.sys.timezone", timeZoneID);

        properties.setProperty("ro.sf.lcd_density", String.valueOf(DisplayMetrics.DENSITY_DEVICE_STABLE));

        try (Writer writer = new FileWriter(propFile)) {
            properties.store(writer, null);
        } catch (IOException ignored) {
        }
    }

    public static void ensureBootFiles(Context context) {

        // Kill orphan container processes FIRST so they don't interfere with
        // directory setup or hold on to stale dalvik-cache entries.
        killOrphanProcess();

        // Ensure /data/local/tmp exists with world-writable permissions.
        // twoyi's init.rc omits the mkdir for /data/local/tmp that AOSP includes,
        // so adbd never has a place to push APKs during `adb install`.
        // This is done here (not via `adb shell mkdir` in the Installer) to avoid
        // a synchronous adb call that hangs when adbd is unresponsive.
        // chmod 777 ensures adbd can write regardless of which UID it runs as.
        ensureDataLocalTmp(context);

        // Patch services.jar to fix the in-container APK install failure.
        // Android 8.0 (SDK 26) PackageInstallerSession.openWriteInternal() calls
        // target.delete() then Os.stat(target) when offsetBytes==0.  Because the
        // file never existed, Os.stat() throws ENOENT.  The SDK 26 code path does
        // not handle ENOENT, causing "stat failed: ENOENT" on every in-container
        // install attempt.  The fix replaces the throw with "stat = null", matching
        // the SDK 27 behaviour that was added in AOSP to handle this exact case.
        patchServicesJarForPackageInstaller(context);

        // <rootdir>/dev/
        File devDir = new File(getRootfsDir(context), "dev");
        ensureDir(new File(devDir, "input"));
        ensureDir(new File(devDir, "socket"));
        ensureDir(new File(devDir, "maps"));

        ensureDir(new File(context.getDataDir(), "socket"));

        createLoaderSymlink(context);

        saveLastKmsg(context);
    }

    private static void createLoaderSymlink(Context context) {
        Path loaderSymlink = new File(context.getDataDir(), "loader64").toPath();
        String loaderPath = getLoaderPath(context);
        try {
            Files.deleteIfExists(loaderSymlink);
            Files.createSymbolicLink(loaderSymlink, Paths.get(loaderPath));
        } catch (IOException e) {
            throw new RuntimeException("symlink loader failed.", e);
        }
    }

    private static void killOrphanProcess() {
        Shell shell = ShellUtil.newSh();
        shell.newJob().add("ps -ef | awk '{if($3==1) print $2}' | xargs kill -9").exec();
    }

    private static void saveLastKmsg(Context context) {
        // Save global last kmsg
        File lastKmsgFile = LogEvents.getLastKmsgFile(context);
        File kmsgFile = LogEvents.getKmsgFile(context);
        try {
            Files.move(kmsgFile.toPath(), lastKmsgFile.toPath(), StandardCopyOption.REPLACE_EXISTING);
        } catch (IOException ignored) {
        }
        
        // Save profile-specific last kmsg
        File profileLastKmsgFile = LogEvents.getProfileLastKmsgFile(context);
        File profileKmsgFile = LogEvents.getProfileKmsgFile(context);
        try {
            if (profileKmsgFile.exists()) {
                Files.move(profileKmsgFile.toPath(), profileLastKmsgFile.toPath(), StandardCopyOption.REPLACE_EXISTING);
            }
        } catch (IOException ignored) {
        }
    }

    public static class RomInfo {
        public String author = DEFAULT_INFO;
        public String version = DEFAULT_INFO;
        public String desc = DEFAULT_INFO;
        public String md5 = "";
        public long code = 0;

        @Override
        public String toString() {
            return "RomInfo{" +
                    "author='" + author + '\'' +
                    ", version='" + version + '\'' +
                    ", md5='" + md5 + '\'' +
                    ", code=" + code +
                    '}';
        }

        public boolean isValid() {
            return this != DEFAULT_ROM_INFO;
        }
    }

    public static final RomInfo DEFAULT_ROM_INFO = new RomInfo();

    public static boolean romExist(Context context) {
        File initFile = new File(getRootfsDir(context), "init");
        return initFile.exists();
    }

    public static boolean needsUpgrade(Context context) {
        // No longer supporting automatic upgrades from assets
        return false;
    }

    public static RomInfo getCurrentRomInfo(Context context) {
        File infoFile = new File(getRootfsDir(context), ROM_INFO_FILE);
        try (FileInputStream inputStream = new FileInputStream(infoFile)) {
            return getRomInfo(inputStream);
        } catch (Throwable e) {
            return DEFAULT_ROM_INFO;
        }
    }

    public static String getLoaderPath(Context context) {
        ApplicationInfo applicationInfo = context.getApplicationInfo();
        return new File(applicationInfo.nativeLibraryDir, LOADER_FILE).getAbsolutePath();
    }



    public static void extractRootfs(Context context, boolean romExist, boolean needsUpgrade, boolean forceInstall, boolean use3rdRom) {
        // This method is now deprecated - ROM extraction is handled through Import Rootfs UI
        // Just ensure system/vendor partitions are cleaned up
        removeSystemPartition(context);
        removeVendorPartition(context);
    }



    public static void reboot(Context context) {
        Intent intent = context.getPackageManager().getLaunchIntentForPackage(context.getPackageName());
        context.getApplicationContext().startActivity(intent);

        shutdown(context);
    }

    public static void shutdown(Context context) {
        System.exit(0);
        Process.killProcess(Process.myPid());
    }

    public static File getRootfsDir(Context context) {
        return new File(context.getDataDir(), "rootfs");
    }

    public static File getRomSdcardDir(Context context) {
        return new File(getRootfsDir(context), "sdcard");
    }

    public static File getVendorDir(Context context) {
        return new File(getRootfsDir(context), "vendor");
    }

    public static File getVendorPropFile(Context context) {
        return new File(getVendorDir(context), "default.prop");
    }



    public static boolean isAndroid12() {
        return Build.VERSION.PREVIEW_SDK_INT + Build.VERSION.SDK_INT == Build.VERSION_CODES.S;
    }

    private static void removePartition(Context context, String partition) {
        File rootfsDir = getRootfsDir(context);
        File systemDir = new File(rootfsDir, partition);

        IOUtils.deleteDirectory(systemDir);
    }

    private static void removeSystemPartition(Context context) {
        removePartition(context, "system");
    }

    private static void removeVendorPartition(Context context) {
        removePartition(context, "vendor");
    }

    private static final String PREF_HOST_FINGERPRINT = "host_build_fingerprint";

    /**
     * Clears the guest Android's dalvik-cache only when the host build fingerprint
     * has changed since the last successful clear (i.e. after a host OTA update).
     *
     * <p>Background: the container's ART OAT/VDEX files in dalvik-cache are compiled
     * against the host ART version.  When the host receives an OTA update the ART
     * version changes, making all existing OAT entries stale.  If the twoyi process
     * has been alive across the OTA (e.g. Xiaomi "frozen" app resurrection), calling
     * clearDalvikCache only in attachBaseContext is not enough because that runs just
     * once per process lifetime.  This method is called synchronously on the UI thread
     * in bootSystem() before addView(mSurfaceView), so the cache is guaranteed to be
     * fully cleared before Renderer.init() starts the container.
     */
    public static void clearDalvikCacheIfNeeded(Context context) {
        String currentFingerprint = Build.FINGERPRINT;
        String lastFingerprint = AppKV.getStringConfig(context, PREF_HOST_FINGERPRINT, "");
        if (!currentFingerprint.equals(lastFingerprint)) {
            Log.i(TAG, "Host fingerprint changed (" + lastFingerprint + " → " + currentFingerprint + "), clearing dalvik-cache");
            clearDalvikCache(context);
            AppKV.setStringConfig(context, PREF_HOST_FINGERPRINT, currentFingerprint);
        } else {
            Log.i(TAG, "Host fingerprint unchanged, skipping dalvik-cache clear");
        }
    }

    private static void clearDalvikCache(Context context) {
        String path = new File(getRootfsDir(context), "data/dalvik-cache").getAbsolutePath();
        Shell.Result result = ShellUtil.newSh().newJob().add("rm -rf '" + path + "'").exec();
        if (!result.isSuccess()) {
            Log.w(TAG, "rm -rf dalvik-cache failed: " + Arrays.toString(result.getErr().toArray(new String[0])));
        }
        Log.i(TAG, "dalvik-cache cleared: " + path);
    }

    private static void ensureDataLocalTmp(Context context) {
        String path = new File(getRootfsDir(context), "data/local/tmp").getAbsolutePath();
        Shell.Result result = ShellUtil.newSh().newJob()
                .add("mkdir -p '" + path + "'")
                .add("chmod 777 '" + path + "'")
                .exec();
        if (!result.isSuccess()) {
            Log.w(TAG, "ensureDataLocalTmp failed: " + Arrays.toString(result.getErr().toArray(new String[0])));
        }
    }

    private static RomInfo getRomInfo(InputStream in) {
        Properties prop = new Properties();
        try {
            prop.load(in);

            RomInfo info = new RomInfo();
            info.author = prop.getProperty("author");
            info.code = Long.parseLong(prop.getProperty("code"));
            info.version = prop.getProperty("version");
            info.desc = prop.getProperty("desc", DEFAULT_INFO);
            info.md5 = prop.getProperty("md5");
            return info;
        } catch (Throwable e) {
            Log.e(TAG, "read rom info err", e);
            return DEFAULT_ROM_INFO;
        }
    }

    private static void ensureDir(File file) {
        if (file.exists()) {
            return;
        }
        //noinspection ResultOfMethodCallIgnored
        file.mkdirs();
    }

    // =========================================================================
    // services.jar DEX patch – fix PackageInstallerSession.openWriteInternal()
    // =========================================================================
    //
    // Root cause (Android 8.0 / SDK 26):
    //   openWriteInternal() calls target.delete() then Os.stat(target).
    //   When offsetBytes == 0 (a fresh install) the file was just deleted (or
    //   never existed), so Os.stat() throws ErrnoException(ENOENT).  SDK 26
    //   code does NOT handle ENOENT and re-throws as
    //     IOException("stat failed: ENOENT (No such file or directory)")
    //   making every GUI-triggered in-container APK install fail immediately.
    //
    // Fix (mirrors the SDK 27 change in AOSP):
    //   Replace the two-byte "throw v_ioe" instruction inside the ErrnoException
    //   catch block with "const/4 v_stat, 0" (sets the stat register to null).
    //   The code that follows already handles stat == null by opening the file
    //   with O_CREAT | O_TRUNC, so the install proceeds normally.
    //
    // Encoding equivalence (both instructions are exactly 2 bytes in DEX):
    //   throw  vN  =  0x27  0xNN
    //   const/4 vM, 0  =  0x12  0x0M   (value 0 in the high nibble)
    // =========================================================================

    /**
     * Patches {@code system/framework/services.jar} inside the container rootfs
     * to fix the ENOENT bug in {@code PackageInstallerSession.openWriteInternal}.
     * The patch is idempotent: a flag file records the last-patched mod-time of
     * {@code services.jar}, so we only re-patch when the JAR has been replaced.
     */
    private static void patchServicesJarForPackageInstaller(Context context) {
        File rootfsDir   = getRootfsDir(context);
        File servicesJar = new File(rootfsDir, "system/framework/services.jar");
        if (!servicesJar.exists()) {
            Log.w(TAG, "patchServicesJar: services.jar not found, skipping");
            return;
        }

        // Re-use the flag file inside the rootfs data directory (writable by us).
        File patchFlag = new File(rootfsDir, "data/.twoyi_pi_patched");
        if (patchFlag.exists() && patchFlag.lastModified() >= servicesJar.lastModified()) {
            Log.d(TAG, "patchServicesJar: already up-to-date, skipping");
            return;
        }

        Log.i(TAG, "patchServicesJar: patching " + servicesJar);
        try {
            byte[] dex = extractClassesDexFromJar(servicesJar);
            if (dex == null) {
                Log.e(TAG, "patchServicesJar: failed to extract classes.dex");
                return;
            }

            int patchedAt = applyOpenWriteInternalEnoentPatch(dex);
            if (patchedAt < 0) {
                // Pattern not found – ROM may already be patched or have a
                // different code layout.  Mark the flag so we don't retry.
                Log.w(TAG, "patchServicesJar: instruction pattern not found – skipping");
                touchFile(patchFlag);
                return;
            }
            Log.i(TAG, "patchServicesJar: patched throw→const/4 at DEX offset 0x"
                    + Integer.toHexString(patchedAt));

            recomputeDexChecksums(dex);

            if (!replaceClassesDexInJar(servicesJar, dex)) {
                Log.e(TAG, "patchServicesJar: failed to write back services.jar");
                return;
            }

            deleteServicesJarOatFiles(rootfsDir);
            touchFile(patchFlag);
            Log.i(TAG, "patchServicesJar: patch applied successfully");

        } catch (Exception e) {
            Log.e(TAG, "patchServicesJar: unexpected error", e);
        }
    }

    /**
     * Returns the raw bytes of {@code classes.dex} extracted from a JAR/ZIP,
     * or {@code null} on failure.
     */
    private static byte[] extractClassesDexFromJar(File jar) throws IOException {
        try (ZipFile zf = new ZipFile(jar)) {
            ZipEntry entry = zf.getEntry("classes.dex");
            if (entry == null) {
                Log.w(TAG, "extractClassesDex: no classes.dex in " + jar);
                return null;
            }
            try (InputStream in = zf.getInputStream(entry)) {
                ByteArrayOutputStream baos =
                        new ByteArrayOutputStream((int) entry.getSize());
                byte[] buf = new byte[8192];
                int n;
                while ((n = in.read(buf)) >= 0) baos.write(buf, 0, n);
                return baos.toByteArray();
            }
        }
    }

    /**
     * Binary-patches {@code dex} in-place.
     *
     * <p>Algorithm:
     * <ol>
     *   <li>Find string-ID for {@code "stat failed: "} in the DEX string pool.</li>
     *   <li>Find the {@code const-string vR, "stat failed: "} instruction
     *       (opcode 0x1a) in the code section.</li>
     *   <li>Scan backwards to find {@code move-exception} (0x0d) – the entry
     *       point of the {@code ErrnoException} catch handler.</li>
     *   <li>Scan further backwards to find {@code move-result-object v_stat}
     *       (0x0c) – the instruction that captures {@code Os.stat()}'s return
     *       value and whose register we must set to {@code null}.</li>
     *   <li>Scan forwards from the {@code const-string} to find the
     *       {@code throw v_ioe} instruction (0x27).</li>
     *   <li>Replace {@code 0x27 vN} with {@code 0x12 0x0M} where M is the
     *       v_stat register index (must be 0–15 for {@code const/4}).</li>
     * </ol>
     *
     * @return the byte offset of the patched instruction, or -1 if the pattern
     *         was not found or cannot be safely patched.
     */
    private static int applyOpenWriteInternalEnoentPatch(byte[] dex) {
        // ── Step 1: find the string ID for "stat failed: " ────────────────────
        final byte[] TARGET = "stat failed: ".getBytes(StandardCharsets.UTF_8);
        int strIdsSize = readInt32LE(dex, 56);
        int strIdsOff  = readInt32LE(dex, 60);

        int statFailedId = -1;
        for (int i = 0; i < strIdsSize; i++) {
            int dataOff = readInt32LE(dex, strIdsOff + i * 4);
            if (dataOff < 0 || dataOff + 1 + TARGET.length >= dex.length) continue;
            int charCount = readUleb128(dex, dataOff);
            if (charCount != TARGET.length) continue;
            int headerLen = uleb128Size(charCount);
            boolean match = true;
            for (int j = 0; j < TARGET.length; j++) {
                if (dex[dataOff + headerLen + j] != TARGET[j]) { match = false; break; }
            }
            if (match) { statFailedId = i; break; }
        }
        if (statFailedId < 0) {
            Log.w(TAG, "applyPatch: string 'stat failed: ' not found in DEX");
            return -1;
        }

        // ── Step 2: find const-string vR, statFailedId  (opcode 0x1a) ─────────
        byte idLo = (byte) (statFailedId & 0xff);
        byte idHi = (byte) ((statFailedId >> 8) & 0xff);
        int csOff = -1;
        // Start searching after the string-IDs table (string data can't hold code).
        int codeSearchStart = strIdsOff + strIdsSize * 4;
        for (int i = codeSearchStart; i < dex.length - 4; i++) {
            if ((dex[i] & 0xff) == 0x1a && dex[i + 2] == idLo && dex[i + 3] == idHi) {
                csOff = i;
                break;
            }
        }
        if (csOff < 0) {
            Log.w(TAG, "applyPatch: const-string for 'stat failed: ' not found");
            return -1;
        }

        // ── Step 3: find move-exception (0x0d) backwards from csOff ──────────
        //   The ErrnoException catch handler starts with move-exception.
        int meOff = -1;
        // Scan in 2-byte steps (Dalvik code units are 2-byte aligned).
        for (int i = csOff - 2; i >= Math.max(0, csOff - 128); i -= 2) {
            if ((dex[i] & 0xff) == 0x0d) { meOff = i; break; }
        }
        if (meOff < 0) { // fallback: byte-by-byte
            for (int i = csOff - 1; i >= Math.max(0, csOff - 128); i--) {
                if ((dex[i] & 0xff) == 0x0d) { meOff = i; break; }
            }
        }
        if (meOff < 0) {
            Log.w(TAG, "applyPatch: move-exception not found before 'stat failed: '");
            return -1;
        }

        // ── Step 4: find move-result-object (0x0c) backwards from move-exception
        //   This is the instruction that stores Os.stat()'s return value into
        //   v_stat.  It is the last move-result-object before the catch handler.
        int statReg = -1;
        for (int i = meOff - 2; i >= Math.max(0, meOff - 512); i -= 2) {
            if ((dex[i] & 0xff) == 0x0c) { statReg = dex[i + 1] & 0xff; break; }
        }
        if (statReg < 0) { // fallback
            for (int i = meOff - 1; i >= Math.max(0, meOff - 512); i--) {
                if ((dex[i] & 0xff) == 0x0c) { statReg = dex[i + 1] & 0xff; break; }
            }
        }
        if (statReg < 0) {
            Log.w(TAG, "applyPatch: move-result-object for v_stat not found");
            return -1;
        }
        if (statReg > 15) {
            // const/4 can only address registers v0–v15.
            Log.w(TAG, "applyPatch: v_stat = v" + statReg + " is out of range for const/4");
            return -1;
        }

        // ── Step 5: find throw (0x27) forwards from csOff ────────────────────
        //   After const-string comes string-building code (new-instance,
        //   invoke-virtual, move-result-object …) then invoke-direct for
        //   IOException.<init> and finally throw v_ioe.
        int throwOff = -1;
        int limit = Math.min(csOff + 256, dex.length - 2);
        for (int i = csOff + 4; i < limit; i++) {
            if ((dex[i] & 0xff) != 0x27) continue;
            // Extra confidence check: the 6 bytes before should be the
            // invoke-direct for IOException.<init>(String,Throwable) which
            // has 3 arguments, encoded as opcode=0x70, count|0=0x30.
            boolean precededByInvokeDirect =
                    i >= 6
                    && (dex[i - 6] & 0xff) == 0x70   // invoke-direct opcode
                    && (dex[i - 5] & 0xff) == 0x30;  // arg-count nibble = 3
            if (precededByInvokeDirect) { throwOff = i; break; }
        }
        if (throwOff < 0) {
            // Retry without the invoke-direct check (some compiler variants).
            for (int i = csOff + 4; i < limit; i++) {
                if ((dex[i] & 0xff) == 0x27) { throwOff = i; break; }
            }
        }
        if (throwOff < 0) {
            Log.w(TAG, "applyPatch: throw instruction not found after 'stat failed: '");
            return -1;
        }

        // ── Step 6: apply the patch ───────────────────────────────────────────
        // Replace  throw v_ioe      [0x27 0xNN]
        // with     const/4 v_stat, 0 [0x12 0x0M]  (M = stat register, value = 0)
        dex[throwOff]     = 0x12;
        dex[throwOff + 1] = (byte) (statReg & 0x0f); // value=0 in high nibble, reg in low nibble

        Log.d(TAG, "applyPatch: throw→const/4 v" + statReg
                + " at 0x" + Integer.toHexString(throwOff)
                + " (move-exception@0x" + Integer.toHexString(meOff)
                + ", v_stat=v" + statReg + ")");
        return throwOff;
    }

    /**
     * Re-computes the Adler-32 checksum (bytes 8–11) and SHA-1 signature
     * (bytes 12–31) of a DEX file in-place after patching.
     *
     * <p>DEX layout:
     * <ul>
     *   <li>bytes  0– 7  magic</li>
     *   <li>bytes  8–11  Adler-32 of bytes 12..end</li>
     *   <li>bytes 12–31  SHA-1   of bytes 32..end</li>
     *   <li>bytes 32–35  file_size …</li>
     * </ul>
     */
    private static void recomputeDexChecksums(byte[] dex)
            throws java.security.NoSuchAlgorithmException {
        // SHA-1 of bytes[32 .. end]
        MessageDigest sha1 = MessageDigest.getInstance("SHA-1");
        sha1.update(dex, 32, dex.length - 32);
        byte[] digest = sha1.digest();
        System.arraycopy(digest, 0, dex, 12, 20);

        // Adler-32 of bytes[12 .. end]  (which now includes the updated SHA-1)
        Adler32 adler = new Adler32();
        adler.update(dex, 12, dex.length - 12);
        int checksum = (int) adler.getValue();
        dex[8]  = (byte)  (checksum         & 0xff);
        dex[9]  = (byte) ((checksum >>  8)  & 0xff);
        dex[10] = (byte) ((checksum >> 16)  & 0xff);
        dex[11] = (byte) ((checksum >> 24)  & 0xff);
    }

    /**
     * Replaces {@code classes.dex} inside {@code jar} with {@code patchedDex},
     * preserving all other ZIP entries and their compression settings.
     *
     * @return {@code true} on success.
     */
    private static boolean replaceClassesDexInJar(File jar, byte[] patchedDex) {
        File tmp = new File(jar.getParent(), jar.getName() + ".twoyi_tmp");
        try (ZipFile  zin  = new ZipFile(jar);
             ZipOutputStream zout = new ZipOutputStream(new FileOutputStream(tmp))) {

            Enumeration<? extends ZipEntry> entries = zin.entries();
            while (entries.hasMoreElements()) {
                ZipEntry src = entries.nextElement();
                byte[] data;
                if ("classes.dex".equals(src.getName())) {
                    data = patchedDex;
                } else {
                    try (InputStream in = zin.getInputStream(src)) {
                        ByteArrayOutputStream baos =
                                new ByteArrayOutputStream((int) Math.max(src.getSize(), 0));
                        byte[] buf = new byte[8192]; int n;
                        while ((n = in.read(buf)) >= 0) baos.write(buf, 0, n);
                        data = baos.toByteArray();
                    }
                }

                ZipEntry dst = new ZipEntry(src.getName());
                if (src.getMethod() == ZipEntry.STORED) {
                    // Must supply size/CRC for STORED entries.
                    dst.setMethod(ZipEntry.STORED);
                    dst.setSize(data.length);
                    dst.setCompressedSize(data.length);
                    CRC32 crc = new CRC32();
                    crc.update(data);
                    dst.setCrc(crc.getValue());
                } else {
                    dst.setMethod(ZipEntry.DEFLATED);
                }
                zout.putNextEntry(dst);
                zout.write(data);
                zout.closeEntry();
            }
            zout.finish();

        } catch (IOException e) {
            Log.e(TAG, "replaceClassesDexInJar: IOException", e);
            tmp.delete();
            return false;
        }

        try {
            Files.move(tmp.toPath(), jar.toPath(), StandardCopyOption.REPLACE_EXISTING);
            return true;
        } catch (IOException e) {
            Log.e(TAG, "replaceClassesDexInJar: move failed", e);
            tmp.delete();
            return false;
        }
    }

    /**
     * Deletes OAT / VDEX files under {@code dalvik-cache} that correspond to
     * {@code services.jar} so that ART recompiles from the patched DEX on the
     * next container boot.
     */
    private static void deleteServicesJarOatFiles(File rootfsDir) {
        File dalvikCache = new File(rootfsDir, "data/dalvik-cache");
        deleteMatchingFiles(dalvikCache, "services");
    }

    /** Recursively deletes files whose names contain {@code pattern}. */
    private static void deleteMatchingFiles(File dir, String pattern) {
        if (dir == null || !dir.isDirectory()) return;
        File[] children = dir.listFiles();
        if (children == null) return;
        for (File child : children) {
            if (child.isDirectory()) {
                deleteMatchingFiles(child, pattern);
            } else if (child.getName().contains(pattern)) {
                if (!child.delete()) {
                    Log.w(TAG, "deleteMatchingFiles: could not delete " + child);
                }
            }
        }
    }

    /** Creates or updates the modification timestamp of a file. */
    private static void touchFile(File file) {
        try {
            file.getParentFile().mkdirs();
            if (!file.exists()) file.createNewFile();
            //noinspection ResultOfMethodCallIgnored
            file.setLastModified(System.currentTimeMillis());
        } catch (IOException e) {
            Log.w(TAG, "touchFile: " + file + ": " + e.getMessage());
        }
    }

    // ── DEX parsing helpers ──────────────────────────────────────────────────

    /** Reads a 4-byte little-endian signed integer from {@code buf[off..off+3]}. */
    private static int readInt32LE(byte[] buf, int off) {
        return  (buf[off]     & 0xff)
             | ((buf[off + 1] & 0xff) <<  8)
             | ((buf[off + 2] & 0xff) << 16)
             | ((buf[off + 3] & 0xff) << 24);
    }

    /**
     * Reads an unsigned LEB128 (ULEB128) value from {@code buf} starting at
     * {@code off} and returns the decoded integer.
     */
    private static int readUleb128(byte[] buf, int off) {
        int result = 0, shift = 0;
        while (off < buf.length) {
            int b = buf[off++] & 0xff;
            result |= (b & 0x7f) << shift;
            if ((b & 0x80) == 0) break;
            shift += 7;
        }
        return result;
    }

    /**
     * Returns the number of bytes required to encode {@code value} as a
     * ULEB128 integer.
     */
    private static int uleb128Size(int value) {
        int size = 1;
        while ((value & ~0x7f) != 0) { value >>>= 7; size++; }
        return size;
    }
}
