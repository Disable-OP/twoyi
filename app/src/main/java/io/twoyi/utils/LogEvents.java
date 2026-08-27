/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.utils;

import android.content.Context;
import android.content.pm.PackageInfo;
import android.os.Build;

import com.microsoft.appcenter.crashes.Crashes;
import com.microsoft.appcenter.crashes.ingestion.models.ErrorAttachmentLog;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.io.PrintWriter;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;
import java.util.concurrent.TimeUnit;

/**
 * @author weishu
 * @date 2022/2/16.
 */

public class LogEvents {

    private static final RuntimeException BOOT_FAILURE = new RuntimeException("BootFailureException");

    public static void trackError(Throwable e) {
        Crashes.trackError(e);
    }
    public static void trackError(Throwable e, Map<String, String> properties, Iterable<ErrorAttachmentLog> attachments) {
        Crashes.trackError(e, properties, attachments);
    }

    public static void trackBootFailure(Context context) {

        Map<String, String> properties = new HashMap<>();
        RomManager.RomInfo info = RomManager.getCurrentRomInfo(context);

        properties.put("rom_ver", String.valueOf(info.code));
        properties.put("rom_author", info.author);
        properties.put("rom_md5", info.md5);

        List<ErrorAttachmentLog> errors = new ArrayList<>();

        errors.add(ErrorAttachmentLog.attachmentWithBinary(getBugreport(context), "bugreport.zip", "application/zip"));

        trackError(BOOT_FAILURE, properties, errors);
    }

    public static File getLogcatFile(Context context) {
        return new File(context.getCacheDir(), "logcat.txt");
    }

    public static File getKmsgFile(Context context) {
        return new File(context.getDataDir(), "log.txt");
    }

    public static File getLastKmsgFile(Context context) {
        return new File(context.getDataDir(), "last_kmsg.txt");
    }

    public static File getProfileKmsgFile(Context context) {
        String activeProfile = ProfileManager.getActiveProfile(context);
        File profileDir = ProfileManager.getProfileDir(context, activeProfile);
        return new File(profileDir, "log.txt");
    }

    public static File getProfileLastKmsgFile(Context context) {
        String activeProfile = ProfileManager.getActiveProfile(context);
        File profileDir = ProfileManager.getProfileDir(context, activeProfile);
        return new File(profileDir, "last_kmsg.txt");
    }

    private static class ReportItem {
        File file;
        String entry;

        public static ReportItem create(File file, String entry) {
            ReportItem item = new ReportItem();
            item.file = file;
            item.entry = entry;
            return item;
        }

        public static ReportItem create(File file) {
            return create(file, file.getName());
        }
    }

    /** Hard cap on how long we wait for an external command (logcat -d / ps -ef)
     *  to finish while building a bug report. Without this cap, a hung child
     *  process would block {@link #getBugreport} indefinitely — and since
     *  getBugreport is called from {@link io.twoyi.Render2Activity#showBootingProcedure}
     *  on a background thread, that thread (named "waiting-boot") would be
     *  stuck forever, never finishing the boot-failure handling that
     *  follows the trackBootFailure call. 5 s is generous: cold logcat -d
     *  on a busy device returns in <500 ms; ps -ef in <100 ms.
     */
    private static final long BUGREPORT_CMD_TIMEOUT_SECONDS = 5;

    public static byte[] getBugreport(Context context) {

        ByteArrayOutputStream baos = new ByteArrayOutputStream();
        ZipOutputStream zout = new ZipOutputStream(baos);

        // file, entry
        List<ReportItem> reportItems = new ArrayList<>();

        // Global kmsg log
        File initLogFile = getKmsgFile(context);
        reportItems.add(ReportItem.create(initLogFile, "global_kmsg.txt"));

        // Global last kmsg log
        File lastKmsgFile = getLastKmsgFile(context);
        reportItems.add(ReportItem.create(lastKmsgFile, "global_last_kmsg.txt"));

        // Profile-specific kmsg log
        File profileKmsgFile = getProfileKmsgFile(context);
        if (profileKmsgFile.exists()) {
            String activeProfile = ProfileManager.getActiveProfile(context);
            reportItems.add(ReportItem.create(profileKmsgFile, "profile_" + activeProfile + "_kmsg.txt"));
        }

        // Profile-specific last kmsg log
        File profileLastKmsgFile = getProfileLastKmsgFile(context);
        if (profileLastKmsgFile.exists()) {
            String activeProfile = ProfileManager.getActiveProfile(context);
            reportItems.add(ReportItem.create(profileLastKmsgFile, "profile_" + activeProfile + "_last_kmsg.txt"));
        }

        // logcat
        File logcatFile = getLogcatFile(context);
        ProcessBuilder logcat = new ProcessBuilder("logcat", "-d");
        logcat.redirectOutput(logcatFile);
        try {
            Process process = logcat.start();
            // Fixed: waitFor() without a timeout can hang forever if logcat
            // blocks (e.g. circular log buffer corruption, SELinux denial
            // storm). Since this runs on the boot-failure reporting path,
            // a hang here would block the boot-failure handling and
            // leave the app stuck on the boot screen.
            if (!process.waitFor(BUGREPORT_CMD_TIMEOUT_SECONDS, TimeUnit.SECONDS)) {
                process.destroyForcibly();
            }
        } catch (Throwable ignored) {}

        reportItems.add(ReportItem.create(logcatFile));

        // tombstones
        File rootfsDir = RomManager.getRootfsDir(context);
        File romDataDir = new File(rootfsDir, "data");
        File tombstoneDir = new File(romDataDir, "tombstones");
        File[] tombstones = tombstoneDir.listFiles();
        if (tombstones != null) {
            for (File tombstone : tombstones) {
                reportItems.add(ReportItem.create(tombstone, "tombstones/" + tombstone.getName()));
            }
        }

        // dropboxs
        File dataSystemDir = new File(romDataDir, "system");
        File dropboxDir = new File(dataSystemDir, "dropbox");
        File[] dropboxs = dropboxDir.listFiles();
        if (dropboxs != null) {
            for (File dropbox : dropboxs) {
                reportItems.add(ReportItem.create(dropbox, "dropbox/" + dropbox.getName()));
            }
        }

        // proc info
        File procInfo = new File(context.getCacheDir(), "proc.txt");
        ProcessBuilder pb = new ProcessBuilder("ps", "-ef");
        pb.redirectOutput(procInfo);
        try {
            Process process = pb.start();
            // Fixed: same waitFor(timeout) pattern as logcat above.
            if (process.waitFor(BUGREPORT_CMD_TIMEOUT_SECONDS, TimeUnit.SECONDS)) {
                reportItems.add(ReportItem.create(procInfo));
            } else {
                process.destroyForcibly();
            }
        } catch (Throwable ignored) {
        }

        // build.prop
        File buildInfo = new File(context.getCacheDir(), "basic.txt");
        try (PrintWriter pw = new PrintWriter(new FileWriter(buildInfo))) {
            pw.println("BRAND: " + Build.BRAND);
            pw.println("MODEL: " + Build.MODEL);
            pw.println("PRODUCT: " + Build.PRODUCT);
            pw.println("MANUFACTURER: " + Build.MANUFACTURER);
            pw.println("SDK: " + Build.VERSION.SDK_INT);
            pw.println("PREVIEW_SDK: " + Build.VERSION.PREVIEW_SDK_INT);
            pw.println("FINGERPRINT: " + Build.FINGERPRINT);
            pw.println("DEVICE: " + Build.DEVICE);

            RomManager.RomInfo romInfo = RomManager.getCurrentRomInfo(context);
            pw.println("ROM: " + romInfo.version);
            PackageInfo packageInfo = context.getPackageManager().getPackageInfo(context.getPackageName(), 0);
            pw.println("PACKAGE: " + packageInfo.packageName);
            pw.println("VERSION: " + packageInfo.versionName);
            
            // Profile information
            String activeProfile = ProfileManager.getActiveProfile(context);
            pw.println("ACTIVE_PROFILE: " + activeProfile);
            pw.println("VERBOSE_LOGGING: " + ProfileSettings.isVerboseLoggingEnabled(context));
            pw.println("DEBUG_RENDERER: " + ProfileSettings.isDebugRendererEnabled(context));
        } catch (Throwable ignored) {}

        reportItems.add(ReportItem.create(buildInfo));

        // Debug renderer logs (if enabled)
        if (ProfileSettings.isDebugRendererEnabled(context)) {
            File debugLogsDir = new File(context.getFilesDir(), "twoyi_renderer_debug");
            if (debugLogsDir.exists() && debugLogsDir.isDirectory()) {
                File[] debugLogs = debugLogsDir.listFiles();
                if (debugLogs != null) {
                    for (File debugLog : debugLogs) {
                        if (debugLog.isFile()) {
                            reportItems.add(ReportItem.create(debugLog, "renderer_debug/" + debugLog.getName()));
                        }
                    }
                }
            }
        }

        for (ReportItem item : reportItems) {
            try {
                // Read the file BEFORE adding the ZIP entry. The previous
                // ordering (putNextEntry → readAllBytes → closeEntry) left
                // an empty entry in the archive whenever readAllBytes
                // threw (e.g. file missing, permission denied, I/O error):
                // putNextEntry had already opened the entry, the catch
                // skipped closeEntry, and the next iteration's putNextEntry
                // would auto-close the orphaned entry with zero bytes.
                // Some ZIP consumers reject archives with empty entries,
                // which would defeat the whole bug-report purpose.
                byte[] bytes = Files.readAllBytes(item.file.toPath());
                ZipEntry ze = new ZipEntry(item.entry);
                zout.putNextEntry(ze);
                zout.write(bytes, 0, bytes.length);
                zout.closeEntry();
            } catch (IOException ignored) {
            }
        }
        IOUtils.closeSilently(zout);

        return baos.toByteArray();
    }
}
