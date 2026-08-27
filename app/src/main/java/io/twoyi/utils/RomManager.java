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

        // VM-inspired: symlink every host-shipped native lib the guest
        // needs into the guest rootfs so the guest linker can dlopen()
        // them by canonical name. Each call is idempotent and never
        // clobbers a pre-existing real file (see ensureLibSymlink).
        // Failures are non-fatal: a missing symlink means the guest
        // can't load that specific lib, but boot continues.
        //
        // libtwrp_fb_hook.so (Task ID: twrp-fb-hook-symlink): added here
        // so kr64's hook_library_candidates() can find it via the
        // {data_dir}/rootfs/system/lib64/libtwrp_fb_hook.so symlink
        // (candidate #3 in hook_library_candidates). Without this
        // symlink, kr64's only fallback on real devices is the
        // TWOYI_NATIVE_LIB_DIR env var (candidate #0, passed by
        // core.rs) — keeping the symlink as defense-in-depth so the
        // find still succeeds even if the env var is missing or stale
        // (e.g. older APK + new kr64, or vice versa).
        File rootfsLib64 = new File(getRootfsDir(context), "system/lib64");
        for (String lib : new String[]{
                "libkr64.so",
                "libOpenglRender.so",
                "libgetpid_hook.so",
                "libtwrp_fb_hook.so",
        }) {
            try {
                ensureLibSymlink(context, lib, new File(rootfsLib64, lib));
            } catch (IOException e) {
                Log.w(TAG, "ensureLibSymlink failed for " + lib + ": " + e.getMessage());
            }
        }

        // Option D (5-U's recommendation): extract the real libdl.so APK
        // asset to {filesDir}/libdl.so so kr64 can read it via
        // apex_extract::read_libdl_asset() (lib.rs Step 3.7 PRIMARY path).
        // This bypasses the fragile APEX loopback-mount pipeline that hit
        // 4 sequential failure modes in 5-L/5-N/5-O/5-P/5-U (temp-write
        // ENOENT → loop_open ENOENT → mknod+fallback loop_open ENXIO for
        // all N 0..31 → kernel has no registered gendisk). Each fix
        // exposed the next layer; the loopback-mount approach depends on
        // too many kernel/permission prerequisites (CAP_MKNOD +
        // CAP_SYS_ADMIN + kernel loop driver + init.rc mknod + ext4 driver).
        //
        // The asset ships as app/src/main/assets/libdl.so. Until CI/dev
        // runs scripts/extract_libdl_from_apex.sh to drop the REAL
        // libdl.so in, the asset is a 5848-byte PLACEHOLDER (text +
        // NUL padding). The kr64 Rust side's is_real_libdl() validator
        // rejects the placeholder (> 5848 byte-size guard + ELF magic
        // check) and falls through to find_real_libdl_so() (APEX
        // extraction, still broken on the Android emulator per 5-U,
        // but kept as a defensive fallback). So a placeholder asset
        // degrades gracefully to the existing pre-Option-D behaviour.
        //
        // Always overwrite (idempotent + picks up the latest asset
        // whenever the APK is updated). The file is small (~13KB max
        // for the real libdl.so), so the I/O cost on every app start
        // is negligible (~0.1ms).
        extractLibdlAsset(context);

        saveLastKmsg(context);
    }

    /**
     * Extract the {@code libdl.so} APK asset to the app's files dir
     * ({@code /data/data/io.twoyi/files/libdl.so} or the work-profile
     * equivalent). This is the Option D primary path (5-U's recommendation):
     * instead of extracting libdl.so from the APEX ext4 image at runtime
     * (which depends on CAP_MKNOD + CAP_SYS_ADMIN + kernel loop driver +
     * ext4 driver — 4 sequential failure modes documented in 5-L/5-N/5-O/
     * 5-P/5-U), we ship the real libdl.so as an APK asset + extract it on
     * app init.
     *
     * <p>The kr64 daemon reads this file via
     * {@code apex_extract::read_libdl_asset()} (lib.rs Step 3.7 Option D
     * path) BEFORE falling back to {@code find_real_libdl_so()} (the APEX
     * extraction pipeline). The Rust side validates the bytes via
     * {@code is_real_libdl()} (&gt; 5848 bytes + ELF magic) so a
     * placeholder asset is gracefully rejected and falls through to APEX
     * extraction.
     *
     * <p>If the asset is missing from the APK (e.g. CI hasn't yet run
     * {@code scripts/extract_libdl_from_apex.sh} to drop the real libdl.so
     * in), this method logs + skips — kr64 will fall through to APEX
     * extraction (which is broken on the Android emulator per 5-U, but
     * kept as a defensive fallback).
     *
     * <p>This method is idempotent: always overwrites the destination so
     * the latest asset is picked up whenever the APK is updated. The file
     * is small (~13KB max for the real libdl.so), so the I/O cost on every
     * app start is negligible (~0.1ms).
     *
     * @param context the application context (used to locate
     *                {@code getFilesDir()} + {@code getAssets()})
     */
    private static void extractLibdlAsset(Context context) {
        File target = new File(context.getFilesDir(), "libdl.so");
        File parent = target.getParentFile();
        if (parent != null && !parent.exists()) {
            //noinspection ResultOfMethodCallIgnored
            parent.mkdirs();
        }

        InputStream in = null;
        FileOutputStream out = null;
        try {
            in = context.getAssets().open("libdl.so");
            out = new FileOutputStream(target);
            byte[] buf = new byte[8192];
            int n;
            while ((n = in.read(buf)) > 0) {
                out.write(buf, 0, n);
            }
            out.flush();
            Log.i(TAG, "extractLibdlAsset: extracted libdl.so APK asset ("
                    + target.length() + " bytes) to " + target
                    + " — kr64 will use this if it passes is_real_libdl (> 5848 bytes + ELF magic)");
        } catch (IOException e) {
            // Asset missing — kr64 will fall through to APEX extraction.
            // This is the expected state until CI/dev drops the real
            // libdl.so into app/src/main/assets/ (via
            // scripts/extract_libdl_from_apex.sh). The placeholder asset
            // ALSO flows through this code path successfully — it just
            // gets rejected by the Rust-side is_real_libdl() validator,
            // which is exactly the graceful degradation we want.
            if (target.exists()) {
                //noinspection ResultOfMethodCallIgnored
                target.delete();
            }
            Log.w(TAG, "extractLibdlAsset: libdl.so APK asset not readable"
                    + " (expected until CI/dev runs scripts/extract_libdl_from_apex.sh"
                    + " to drop the real one in) — kr64 will fall through to APEX"
                    + " extraction (find_real_libdl_so): " + e.getMessage());
        } finally {
            IOUtils.closeSilently(in);
            IOUtils.closeSilently(out);
        }
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

    /**
     * Ensure a native library from the host app's {@code nativeLibraryDir}
     * is reachable at a given guest-visible path via a symlink.
     *
     * <p>This is the VM-inspired generalisation of {@link #createLoaderSymlink}:
     * VM symlinks every host-shipped native lib the guest needs (loader,
     * kr64, openglrenderer, ...) into the guest rootfs so the guest's
     * linker can {@code dlopen()} them by their canonical names. The
     * loader symlink alone isn't enough once the kr64 daemon and the
     * AOSP emugl renderer are involved — both ship as native libs in
     * the APK and need to be reachable from the guest.
     *
     * <p>This method:
     * <ol>
     *   <li>Resolves {@code libName} against the app's
     *       {@code nativeLibraryDir} (e.g. {@code libloader.so} →
     *       {@code /data/app/.../lib/arm64/libloader.so}).</li>
     *   <li>If the target symlink already exists and points at the
     *       right file, this is a no-op (idempotent — safe to call on
     *       every boot).</li>
     *   <li>If the symlink exists but is stale (points elsewhere, or
     *       the target is gone), it is replaced.</li>
     *   <li>If the path exists as a regular file/dir (not a symlink),
     *       it is <b>left alone</b> — the rootfs may ship a real file
     *       there and we must not clobber it.</li>
     *   <li>Parent directories of the symlink are created on demand.</li>
     * </ol>
     *
     * @param context  the application context (used to locate
     *                 {@code nativeLibraryDir}).
     * @param libName  the library file name as it appears in
     *                 {@code nativeLibraryDir} (e.g. {@code "libloader.so"}).
     * @param linkPath the absolute guest-visible path where the symlink
     *                 should appear (e.g.
     *                 {@code new File(getRootfsDir(context), "system/lib64/libkr64.so")}).
     * @return the canonical path of the resolved host library (the
     *         symlink target), for logging / diagnostics.
     * @throws IOException if the symlink cannot be created or the
     *         source library does not exist.
     */
    private static String ensureLibSymlink(Context context, String libName, File linkPath)
            throws IOException {
        ApplicationInfo appInfo = context.getApplicationInfo();
        File sourceLib = new File(appInfo.nativeLibraryDir, libName);
        String sourcePath = sourceLib.getAbsolutePath();

        if (!sourceLib.exists()) {
            throw new IOException("source library not found: " + sourcePath);
        }

        Path link = linkPath.toPath();

        // Idempotency: if the symlink already points at the right file,
        // there's nothing to do. Files.isSymbolicLink returns false for
        // regular files/dirs, so a pre-existing real file at linkPath
        // is left alone (the rootfs ships its own copy — don't clobber).
        if (Files.isSymbolicLink(link)) {
            Path currentTarget = Files.readSymbolicLink(link);
            if (currentTarget.toString().equals(sourcePath)) {
                // Already correctly linked — nothing to do.
                return sourcePath;
            }
            // Stale symlink (points elsewhere) — replace it.
            Files.deleteIfExists(link);
        } else if (Files.exists(link)) {
            // A real file or directory already lives at linkPath. The
            // rootfs intentionally ships this — leave it alone. This is
            // the "don't clobber a real blob" branch.
            Log.i(TAG, "ensureLibSymlink: keeping existing non-symlink at " + linkPath);
            return sourcePath;
        }

        // Make sure the parent dir exists (e.g. rootfs/system/lib64/).
        File parent = linkPath.getParentFile();
        if (parent != null && !parent.exists()) {
            // noinspection ResultOfMethodCallIgnored — best-effort mkdir
            parent.mkdirs();
        }

        Files.createSymbolicLink(link, Paths.get(sourcePath));
        Log.i(TAG, "ensureLibSymlink: " + libName + " -> " + linkPath
                + " (target " + sourcePath + ")");
        return sourcePath;
    }

    private static void killOrphanProcess() {
        // 6-Z184: the old filter (`$3==1`, i.e. every PPID==1 process
        // system-wide) attempted to kill hundreds of unrelated system
        // daemons on every boot (each EPERM'd today, but would be lethal
        // under an elevated shell). Filter to OUR OWN uid only.
        Shell shell = ShellUtil.newSh();
        // 6-Z184 audit follow-up: ps -ef's $1 is the user NAME (u0_aNN),
        // not the numeric uid — match id -un. Keep the PPID==1 filter.
        shell.newJob().add("ps -ef | awk -v u=$(id -un) '{if($1==u && $3==1) print $2}'"
                + " | xargs -r kill -9").exec();
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
        // Fixed: getLaunchIntentForPackage returns null if the package is
        // disabled, suspended, or absent (e.g. on some OEM ROMs that strip
        // the launcher activity during backup/restore). The previous code
        // unconditionally called startActivity(intent) which would NPE —
        // and since shutdown() follows immediately, the NPE prevented the
        // clean exit/restart path from running, leaving the app stuck.
        // Fall back to System.exit(0) so the system restarts us cleanly
        // via the manifest launch mode, matching shutdown()'s contract.
        if (intent != null) {
            context.getApplicationContext().startActivity(intent);
        } else {
            Log.w(TAG, "reboot: getLaunchIntentForPackage returned null — exiting without restart intent");
        }

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
        // Fixed: the old check (PREVIEW_SDK_INT + SDK_INT == S) was backwards.
        // On Android 12 preview: 1 + 31 = 32 != 31 → false (wrong!)
        // On Android 12 stable: 0 + 31 = 31 == 31 → true (correct)
        // On Android 13+: 0 + 33 = 33 != 31 → false (should be true for 13+)
        // Correct check: SDK_INT >= S (Android 12 or later)
        return Build.VERSION.SDK_INT >= Build.VERSION_CODES.S;
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
     *
     * <p>The implementation now lives in {@link DalvikCacheManager} (ported from
     * cyanmint/Nogitsune's {@code BootHelper.kt::clearDalvikCacheIfNeeded}).
     * This method is kept as a thin delegating wrapper so existing callers
     * (Render2Activity, future external code) don't need to change.
     */
    public static void clearDalvikCacheIfNeeded(Context context) {
        DalvikCacheManager.checkAndInvalidate(context, getRootfsDir(context));
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
            // Use the two-arg getProperty(key, default) so a missing
            // optional field preserves the DEFAULT_INFO initialiser
            // instead of being silently overwritten with null. The
            // previous one-arg getProperty("author") would set info.author
            // to null if the rom.ini lacked the "author" key, defeating
            // the field's DEFAULT_INFO initialiser and causing
            // NPE-risking downstream consumers (e.g. crash reports that
            // toString() the RomInfo, or trackBootFailure which puts
            // info.author into a Map) to see "null" instead of "unknown".
            info.author = prop.getProperty("author", DEFAULT_INFO);
            info.code = Long.parseLong(prop.getProperty("code", "0"));
            info.version = prop.getProperty("version", DEFAULT_INFO);
            info.desc = prop.getProperty("desc", DEFAULT_INFO);
            info.md5 = prop.getProperty("md5", "");
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
                // Fixed: ZipEntry.getSize() may return -1 if the central
                // directory entry lacks a size field (e.g. for streaming-mode
                // ZIPs written by some tools). ByteArrayOutputStream(int)
                // throws IllegalArgumentException on a negative size, which
                // would turn a recoverable "skip the patch" path into an
                // uncaught crash on every boot. Clamp to 0 like
                // replaceClassesDexInJar does (which uses Math.max(getSize,0)).
                int initialSize = (int) Math.max(entry.getSize(), 0);
                ByteArrayOutputStream baos = new ByteArrayOutputStream(initialSize);
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
