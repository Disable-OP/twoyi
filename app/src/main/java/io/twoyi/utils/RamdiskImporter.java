/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.utils;

import android.content.Context;
import android.net.Uri;
import android.util.Log;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.util.Arrays;
import java.util.HashMap;
import java.util.Map;
import java.util.zip.GZIPInputStream;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;

import org.apache.commons.compress.compressors.lzma.LZMACompressorInputStream;
import org.apache.commons.compress.compressors.xz.XZCompressorInputStream;

/**
 * Import ramdisk/rootfs from various formats — ALL PURE JAVA, no external commands.
 *
 * Supported formats (auto-detected by magic bytes):
 *   - .img — Android boot image (extracts ramdisk, decompresses gzip/LZMA/uncompressed)
 *   - .cpio — Raw SVR4 cpio archive (newc format)
 *   - .cpio.gz — Gzipped cpio
 *   - .cpio.lzma / .cpio.xz — LZMA/XZ compressed cpio
 *   - .zip — ZIP containing a ramdisk entry
 *   - .tar — TAR archive (extracted via Java TarInputStream)
 *   - .tar.gz — Gzipped TAR
 */
public class RamdiskImporter {
    private static final String TAG = "RamdiskImporter";

    /**
     * Resolve an archive entry name against the extraction root, rejecting
     * path-traversal attacks (Zip-Slip / Tar-Slip).
     *
     * A malicious .tar/.cpio/.zip (imported via the file picker or
     * ACTION_VIEW) can carry entries like "../../files/bin/su" that would
     * otherwise write OUTSIDE targetDir — into app-writable storage,
     * including the container rootfs binaries that later get executed.
     * Every extraction site must route through this helper.
     *
     * @return the resolved File, guaranteed (canonical-path) to be inside
     *         {@code targetDir}; or {@code null} if the entry tries to
     *         escape (caller skips + logs).
     */
    private static File safeTargetFile(File targetDir, String entryName) {
        if (entryName == null) return null;
        // Reject obviously-absolute names early ("/sbin/x" → strip-lead or skip).
        String name = entryName.startsWith("/") ? entryName.substring(1) : entryName;
        if (name.isEmpty()) return targetDir;
        File root;
        File dest;
        try {
            root = targetDir.getCanonicalFile();
            dest = new File(root, name).getCanonicalFile();
        } catch (IOException e) {
            Log.w(TAG, "rejecting entry with unresolvable path: " + entryName);
            return null;
        }
        if (!dest.getPath().startsWith(root.getPath() + File.separator)
                && !dest.equals(root)) {
            Log.w(TAG, "rejecting path-traversal entry: '" + entryName + "' resolves to "
                    + dest.getPath() + " (outside " + root.getPath() + ")");
            return null;
        }
        return dest;
    }

    public static boolean importRamdisk(Context context, Uri uri, File targetDir) throws IOException {
        File tempFile = new File(context.getCacheDir(), "ramdisk_import");
        boolean result;
        try {
            try (InputStream is = context.getContentResolver().openInputStream(uri);
                 OutputStream os = new FileOutputStream(tempFile)) {
                if (is == null) throw new IOException("Cannot open: " + uri);
                byte[] buffer = new byte[8192];
                int count;
                while ((count = is.read(buffer)) > 0) {
                    os.write(buffer, 0, count);
                }
            }

            Log.i(TAG, "Import file size: " + tempFile.length() + " bytes");
            FileLogger.i(TAG, "Import file size: " + tempFile.length() + " bytes");

            String format = detectFormat(tempFile);
            Log.i(TAG, "Detected format: " + format);
            FileLogger.i(TAG, "Detected format: " + format);
            FileLogger.boot("ramdisk_import_format_detected", "format=" + format + " size=" + tempFile.length());
            result = importDetected(tempFile, targetDir, format);
        } finally {
            // Delete the staged copy on EVERY exit path — an exception in
            // detectFormat()/import*() used to leave the full-size temp
            // copy (100 MB+) rotting in cacheDir.
            tempFile.delete();
        }
        return result;
    }

    private static boolean importDetected(File tempFile, File targetDir, String format) throws IOException {
        boolean result;
        switch (format) {
            case "android_bootimg":
                result = importBootImage(tempFile, targetDir);
                break;
            case "gzip":
                result = importGzipFile(tempFile, targetDir);
                break;
            case "lzma":
                result = importLzmaFile(tempFile, targetDir);
                break;
            case "xz":
                result = importXzFile(tempFile, targetDir);
                break;
            case "zip":
                result = importZipFile(tempFile, targetDir);
                break;
            case "cpio":
                result = extractCpioStreaming(tempFile, targetDir);
                break;
            case "tar":
                result = extractTarJava(tempFile, targetDir);
                break;
            default:
                Log.w(TAG, "Unknown format, trying cpio then tar");
                try {
                    result = extractCpioStreaming(tempFile, targetDir);
                } catch (Exception e) {
                    Log.w(TAG, "cpio failed: " + e.getMessage());
                    result = false;
                }
                if (!result) {
                    Log.w(TAG, "cpio failed, trying tar");
                    try {
                        result = extractTarJava(tempFile, targetDir);
                    } catch (Exception e) {
                        Log.w(TAG, "tar failed: " + e.getMessage());
                        result = false;
                    }
                }
                break;
        }

        FileLogger.boot("ramdisk_import_complete", "result=" + result);
        return result;
    }

    private static String detectFormat(File file) throws IOException {
        byte[] header = new byte[512];
        try (FileInputStream fis = new FileInputStream(file)) {
            int read = fis.read(header);
            if (read < 8) return "unknown";
        }

        // Android boot image: "ANDROID!"
        if (header[0] == 'A' && header[1] == 'N' && header[2] == 'D' &&
            header[3] == 'R' && header[4] == 'O' && header[5] == 'I' &&
            header[6] == 'D' && header[7] == '!') {
            return "android_bootimg";
        }

        // GZIP: 0x1f 0x8b
        if ((header[0] & 0xFF) == 0x1f && (header[1] & 0xFF) == 0x8b) {
            return "gzip";
        }

        // XZ: 0xfd 0x37 0x7a 0x58 0x5a 0x00
        if ((header[0] & 0xFF) == 0xfd && header[1] == '7' && header[2] == 'z' &&
            header[3] == 'X' && header[4] == 'Z' && header[5] == 0) {
            return "xz";
        }

        // LZMA: 0x5d 0x00 (raw LZMA stream — properties byte + dict size)
        if ((header[0] & 0xFF) == 0x5d) {
            return "lzma";
        }

        // ZIP: "PK"
        if (header[0] == 'P' && header[1] == 'K') {
            return "zip";
        }

        // CPIO newc: "070701" or "070702"
        if (header[0] == '0' && header[1] == '7' && header[2] == '0' &&
            header[3] == '7' && header[4] == '0' && (header[5] == '1' || header[5] == '2')) {
            return "cpio";
        }

        // TAR: "ustar" at offset 257
        if (header.length > 262 && header[257] == 'u' && header[258] == 's' &&
            header[259] == 't' && header[260] == 'a' && header[261] == 'r') {
            return "tar";
        }

        return "unknown";
    }

    /**
     * Import an Android boot image (.img).
     * Pure Java: parses header, reads ramdisk, decompresses (gzip/LZMA/uncompressed),
     * extracts cpio to targetDir.
     */
    private static boolean importBootImage(File imgFile, File targetDir) throws IOException {
        try (FileInputStream fis = new FileInputStream(imgFile)) {
            byte[] header = new byte[16384];
            int headerSize = fis.read(header);
            if (headerSize < 110) throw new IOException("Boot image too small");

            ByteBuffer bb = ByteBuffer.wrap(header).order(ByteOrder.LITTLE_ENDIAN);
            int kernelSize = bb.getInt(8);
            int ramdiskSize = bb.getInt(16);
            int pageSize = bb.getInt(36);

            // 6-Z208: boot-image v3/v4 detection.
            //
            // Android 10+ A/B devices ship recovery-in-boot images in
            // the v3/v4 boot-image format (boot_signature + 4KiB page
            // + no dt_size field + different header layout). The v0/v1/
            // v2 layout — which this importer was written against —
            // has page_size at offset 36; v3/v4's offset 36 holds
            // part of os_version/header_size, which decodes to a non-
            // power-of-2 page_size and triggers the "Invalid boot
            // image" IOException at line 230 — the LineageOS-22.2
            // failure mode (run 33202601022 lineage-r0s: pageSize=0
            // ramdiskSize=536871336 — the importer read header_size +
            // reserved[0] in place of pageSize + dt_size).
            //
            // Detection (mirrors scripts/recovery-corpus/
            // inspect_image.py): if the page_size at offset 36 isn't
            // in {512, 1024, 2048, 4096}, read the header_version at
            // offset 40; if it's 3 or 4, switch to v3/v4 layout:
            //   page_size = 4096 (hardcoded by spec)
            //   ramdisk_offset = page_size + ceil(kernel_size / page_size) * page_size
            // (no dt_size + no second_size field; the kernel_size and
            // ramdisk_size fields at offsets 8/16 are COMMON across
            // all versions).
            if (pageSize != 512 && pageSize != 1024
                    && pageSize != 2048 && pageSize != 4096) {
                // Candidate v3/v4 — verify header_version.
                int headerVersion = bb.getInt(40);
                if (headerVersion == 3 || headerVersion == 4) {
                    pageSize = 4096;
                    Log.i(TAG, "Boot image: v" + headerVersion
                        + " layout detected (page_size=4096 hardcoded)");
                } else {
                    throw new IOException("Invalid boot image: pageSize="
                        + pageSize + " ramdiskSize=" + ramdiskSize
                        + " header_version=" + headerVersion
                        + " (not v0/v1/v2/v3/v4 — unrecognized format)");
                }
            }

            if (pageSize == 0 || ramdiskSize == 0) {
                throw new IOException("Invalid boot image: pageSize=" + pageSize + " ramdiskSize=" + ramdiskSize);
            }

            int kernelPages = (kernelSize + pageSize - 1) / pageSize;
            int ramdiskOffset = pageSize + kernelPages * pageSize;

            Log.i(TAG, "Boot image: kernel=" + kernelSize + " ramdisk=" + ramdiskSize +
                  " page=" + pageSize + " ramdiskOffset=" + ramdiskOffset);

            fis.getChannel().position(ramdiskOffset);
            byte[] ramdiskCompressed = new byte[ramdiskSize];
            int read = 0;
            while (read < ramdiskSize) {
                int n = fis.read(ramdiskCompressed, read, ramdiskSize - read);
                if (n < 0) break;
                read += n;
            }
            if (read != ramdiskSize) {
                // A short read here used to feed a truncated ramdisk to the
                // decompressor, which may not always fail loudly. Fail here.
                throw new IOException("Boot image ramdisk short read: got " + read
                    + " of " + ramdiskSize + " bytes (corrupt source or I/O failure)");
            }

            // Decompress ramdisk -> cpio temp file
            File cpioTemp = new File(targetDir.getParentFile(), "ramdisk.cpio");
            if ((ramdiskCompressed[0] & 0xFF) == 0x1f && (ramdiskCompressed[1] & 0xFF) == 0x8b) {
                // GZIP
                Log.i(TAG, "Ramdisk is gzip-compressed");
                try (GZIPInputStream gzis = new GZIPInputStream(new java.io.ByteArrayInputStream(ramdiskCompressed));
                     FileOutputStream fos = new FileOutputStream(cpioTemp)) {
                    byte[] buf = new byte[8192];
                    int n;
                    while ((n = gzis.read(buf)) > 0) fos.write(buf, 0, n);
                }
            } else if ((ramdiskCompressed[0] & 0xFF) == 0x5d) {
                // LZMA — use Apache Commons Compress (pure Java)
                Log.i(TAG, "Ramdisk is LZMA-compressed");
                try (LZMACompressorInputStream lzis = new LZMACompressorInputStream(
                         new java.io.ByteArrayInputStream(ramdiskCompressed));
                     FileOutputStream fos = new FileOutputStream(cpioTemp)) {
                    byte[] buf = new byte[8192];
                    int n;
                    while ((n = lzis.read(buf)) > 0) fos.write(buf, 0, n);
                }
            } else if (ramdiskCompressed.length >= 6
                    && (ramdiskCompressed[0] & 0xFF) == 0xfd
                    && (ramdiskCompressed[1] & 0xFF) == 0x37
                    && (ramdiskCompressed[2] & 0xFF) == 0x7a
                    && (ramdiskCompressed[3] & 0xFF) == 0x58
                    && (ramdiskCompressed[4] & 0xFF) == 0x5a
                    && (ramdiskCompressed[5] & 0xFF) == 0x00) {
                // 6-Z208: XZ container (magic FD 37 7A 58 5A 00) — the
                // standard format for Android 11+ boot-image ramdisks
                // (LineageOS 22.2 nightly + AOSP mainline both default
                // to it). Apache Commons Compress's XZCompressorInputStream
                // handles the container → raw LZMA2 stream decode.
                Log.i(TAG, "Ramdisk is XZ-compressed");
                try (XZCompressorInputStream xzis = new XZCompressorInputStream(
                         new java.io.ByteArrayInputStream(ramdiskCompressed));
                     FileOutputStream fos = new FileOutputStream(cpioTemp)) {
                    byte[] buf = new byte[8192];
                    int n;
                    while ((n = xzis.read(buf)) > 0) fos.write(buf, 0, n);
                }
            } else if (ramdiskCompressed[0] == '0' && ramdiskCompressed[1] == '7') {
                // Uncompressed cpio
                Log.i(TAG, "Ramdisk is uncompressed cpio");
                try (FileOutputStream fos = new FileOutputStream(cpioTemp)) {
                    fos.write(ramdiskCompressed);
                }
            } else {
                throw new IOException("Unknown ramdisk compression: 0x" +
                    Integer.toHexString(ramdiskCompressed[0] & 0xFF) +
                    Integer.toHexString(ramdiskCompressed[1] & 0xFF));
            }

            boolean result = extractCpioStreaming(cpioTemp, targetDir);
            cpioTemp.delete();
            return result;
        }
    }

    /**
     * Import a gzip file. Could be .tar.gz (rootfs) or .cpio.gz (ramdisk).
     * Pure Java: uses GZIPInputStream.
     *
     * Fully decompresses the gzip stream to a temp file, then dispatches to the
     * correct inner handler (CPIO newc / TAR) based on the decompressed magic.
     */
    private static boolean importGzipFile(File gzFile, File targetDir) throws IOException {
        Log.i(TAG, "GZIP-compressed file detected");

        // Fully decompress the gzip stream to a temp file ONCE.
        // We don't peek-and-restart because GZIPInputStream doesn't support
        // mark/reset reliably across implementations.
        File innerTemp = new File(targetDir.getParentFile(), "ramdisk.inner");
        long decompressedBytes = 0;
        try (GZIPInputStream gzis = new GZIPInputStream(new FileInputStream(gzFile));
             FileOutputStream fos = new FileOutputStream(innerTemp)) {
            byte[] buf = new byte[8192];
            int n;
            while ((n = gzis.read(buf)) > 0) {
                fos.write(buf, 0, n);
                decompressedBytes += n;
            }
        } catch (java.util.zip.ZipException ze) {
            innerTemp.delete();
            throw new IOException("GZIP decompress failed: " + ze.getMessage(), ze);
        }
        Log.i(TAG, "GZIP decompressed " + decompressedBytes + " bytes -> " + innerTemp.getName());

        if (decompressedBytes < 8) {
            innerTemp.delete();
            throw new IOException("Decompressed file too small (" + decompressedBytes + " bytes)");
        }

        // Read the inner magic to dispatch
        byte[] header = new byte[Math.min(512, (int) Math.min(innerTemp.length(), 512))];
        try (FileInputStream fis = new FileInputStream(innerTemp)) {
            int read = readFully(fis, header);
            if (read < 8) {
                innerTemp.delete();
                throw new IOException("Could not read inner header");
            }
        }

        boolean isCpio  = header[0] == '0' && header[1] == '7' && header[2] == '0' && header[3] == '7'
                       && (header[4] == '0' && (header[5] == '1' || header[5] == '2'));
        boolean isTar   = header.length > 262 && header[257] == 'u' && header[258] == 's'
                       && header[259] == 't' && header[260] == 'a' && header[261] == 'r';

        boolean result;
        if (isCpio) {
            Log.i(TAG, "CPIO inside detected format GZIP-compressed file");
            result = extractCpioStreaming(innerTemp, targetDir);
        } else if (isTar) {
            Log.i(TAG, "TAR inside detected format GZIP-compressed file");
            result = extractTarJava(innerTemp, targetDir);
        } else {
            // Unknown inner format — try cpio first, then tar, as a last resort.
            Log.w(TAG, "Unknown inner format inside gzip (magic=0x"
                    + Integer.toHexString(header[0] & 0xFF)
                    + Integer.toHexString(header[1] & 0xFF)
                    + "), trying cpio then tar");
            try {
                result = extractCpioStreaming(innerTemp, targetDir);
            } catch (Exception e) {
                Log.w(TAG, "cpio fallback failed: " + e.getMessage());
                result = false;
            }
            if (!result) {
                try {
                    result = extractTarJava(innerTemp, targetDir);
                } catch (Exception e) {
                    Log.w(TAG, "tar fallback failed: " + e.getMessage());
                    result = false;
                }
            }
        }

        innerTemp.delete();
        return result;
    }

    /**
     * Import an LZMA-compressed file. Pure Java: uses Apache Commons Compress.
     */
    private static boolean importLzmaFile(File lzmaFile, File targetDir) throws IOException {
        Log.i(TAG, "LZMA-compressed file detected");
        File cpioTemp = new File(targetDir.getParentFile(), "ramdisk.cpio");
        try (LZMACompressorInputStream lzis = new LZMACompressorInputStream(new FileInputStream(lzmaFile));
             FileOutputStream fos = new FileOutputStream(cpioTemp)) {
            byte[] buf = new byte[8192];
            int n;
            while ((n = lzis.read(buf)) > 0) fos.write(buf, 0, n);
        }
        Log.i(TAG, "LZMA decompressed -> " + cpioTemp.length() + " bytes");

        // Check if decompressed content is cpio
        byte[] header = new byte[6];
        try (FileInputStream fis = new FileInputStream(cpioTemp)) {
            readFully(fis, header);
        }
        if (new String(header).equals("070701") || new String(header).equals("070702")) {
            Log.i(TAG, "CPIO inside detected format LZMA-compressed file");
            boolean result = extractCpioStreaming(cpioTemp, targetDir);
            cpioTemp.delete();
            return result;
        }

        Log.i(TAG, "TAR inside detected format LZMA-compressed file");
        boolean result = extractTarJava(cpioTemp, targetDir);
        cpioTemp.delete();
        return result;
    }

    /**
     * Import an XZ-compressed file. Pure Java: uses Apache Commons Compress.
     */
    private static boolean importXzFile(File xzFile, File targetDir) throws IOException {
        Log.i(TAG, "XZ-compressed file detected");
        File cpioTemp = new File(targetDir.getParentFile(), "ramdisk.cpio");
        try (XZCompressorInputStream xzis = new XZCompressorInputStream(new FileInputStream(xzFile));
             FileOutputStream fos = new FileOutputStream(cpioTemp)) {
            byte[] buf = new byte[8192];
            int n;
            while ((n = xzis.read(buf)) > 0) fos.write(buf, 0, n);
        }
        Log.i(TAG, "XZ decompressed -> " + cpioTemp.length() + " bytes");

        byte[] header = new byte[6];
        try (FileInputStream fis = new FileInputStream(cpioTemp)) {
            readFully(fis, header);
        }
        if (new String(header).equals("070701") || new String(header).equals("070702")) {
            Log.i(TAG, "CPIO inside detected format XZ-compressed file");
            boolean result = extractCpioStreaming(cpioTemp, targetDir);
            cpioTemp.delete();
            return result;
        }

        Log.i(TAG, "TAR inside detected format XZ-compressed file");
        boolean result = extractTarJava(cpioTemp, targetDir);
        cpioTemp.delete();
        return result;
    }

    /**
     * Import a ZIP file containing a ramdisk. Pure Java: uses java.util.zip.ZipInputStream.
     */
    private static boolean importZipFile(File zipFile, File targetDir) throws IOException {
        try (ZipInputStream zis = new ZipInputStream(new FileInputStream(zipFile))) {
            ZipEntry entry;
            while ((entry = zis.getNextEntry()) != null) {
                String name = entry.getName().toLowerCase();
                if (name.endsWith(".cpio") || name.endsWith(".cpio.gz") ||
                    name.endsWith(".ramdisk") || name.endsWith(".ramdisk.gz") ||
                    name.endsWith(".cpio.lzma") || name.endsWith(".cpio.xz")) {

                    File cpioTemp = new File(targetDir.getParentFile(), "ramdisk.cpio");
                    try (FileOutputStream fos = new FileOutputStream(cpioTemp)) {
                        byte[] buf = new byte[8192];
                        int n;
                        while ((n = zis.read(buf)) > 0) fos.write(buf, 0, n);
                    }

                    // Decompress if needed
                    if (name.endsWith(".gz")) {
                        File dec = new File(cpioTemp.getParent(), "ramdisk_dec.cpio");
                        try (GZIPInputStream gzis = new GZIPInputStream(new FileInputStream(cpioTemp));
                             FileOutputStream fos = new FileOutputStream(dec)) {
                            byte[] buf = new byte[8192];
                            int n;
                            while ((n = gzis.read(buf)) > 0) fos.write(buf, 0, n);
                        }
                        cpioTemp.delete();
                        cpioTemp = dec;
                    } else if (name.endsWith(".lzma")) {
                        File dec = new File(cpioTemp.getParent(), "ramdisk_dec.cpio");
                        try (LZMACompressorInputStream lzis = new LZMACompressorInputStream(new FileInputStream(cpioTemp));
                             FileOutputStream fos = new FileOutputStream(dec)) {
                            byte[] buf = new byte[8192];
                            int n;
                            while ((n = lzis.read(buf)) > 0) fos.write(buf, 0, n);
                        }
                        cpioTemp.delete();
                        cpioTemp = dec;
                    } else if (name.endsWith(".xz")) {
                        File dec = new File(cpioTemp.getParent(), "ramdisk_dec.cpio");
                        try (XZCompressorInputStream xzis = new XZCompressorInputStream(new FileInputStream(cpioTemp));
                             FileOutputStream fos = new FileOutputStream(dec)) {
                            byte[] buf = new byte[8192];
                            int n;
                            while ((n = xzis.read(buf)) > 0) fos.write(buf, 0, n);
                        }
                        cpioTemp.delete();
                        cpioTemp = dec;
                    }

                    boolean result = extractCpioStreaming(cpioTemp, targetDir);
                    cpioTemp.delete();
                    return result;
                }
            }
        }
        throw new IOException("No .cpio or .ramdisk entry found in ZIP");
    }

    /**
     * Extract a TAR archive using pure Java (Apache Commons Compress TarArchiveInputStream).
     */
    private static boolean extractTarJava(File tarFile, File targetDir) throws IOException {
        try (org.apache.commons.compress.archivers.tar.TarArchiveInputStream tais =
                 new org.apache.commons.compress.archivers.tar.TarArchiveInputStream(
                     new FileInputStream(tarFile))) {
            org.apache.commons.compress.archivers.tar.TarArchiveEntry entry;
            int count = 0;
            while ((entry = tais.getNextTarEntry()) != null) {
                File dest = safeTargetFile(targetDir, entry.getName());
                if (dest == null) {
                    continue; // path-traversal entry — skipped
                }
                if (entry.isDirectory()) {
                    dest.mkdirs();
                    count++;
                } else {
                    File f = dest;
                    f.getParentFile().mkdirs();
                    try (FileOutputStream fos = new FileOutputStream(f)) {
                        byte[] buf = new byte[8192];
                        int n;
                        while ((n = tais.read(buf)) > 0) fos.write(buf, 0, n);
                    }
                    // Task 6-Z31: set executable permission from the tar mode.
                    // Java's FileOutputStream creates files with mode 0644 (NOT
                    // executable). Without this, /sbin/recovery + /sbin/linker
                    // are created without the execute bit → execve fails with
                    // EACCES → exit 127 → TWRP recovery service never starts.
                    int mode = entry.getMode();
                    if ((mode & 0100) != 0) {
                        f.setExecutable(true, false); // owner + group + other exec
                    }
                    if ((mode & 0400) != 0) {
                        f.setReadable(true, false);
                    }
                    if ((mode & 0200) != 0) {
                        f.setWritable(true, true);
                    }
                    count++;
                }
            }
            Log.i(TAG, "Extracted " + count + " entries from tar");
            return count > 0;
        }
    }

    /**
     * Extract a SVR4 cpio archive (newc format) using pure Java streaming.
     * Reads the file in chunks — does NOT load everything into memory.
     */
    private static boolean extractCpioStreaming(File cpioFile, File targetDir) throws IOException {
        try (FileInputStream fis = new FileInputStream(cpioFile)) {
            int fileCount = 0;
            long fileLength = cpioFile.length();
            long pos = 0;
            boolean sawTrailer = false;
            // Task 6-Z82: hardlink bookkeeping — see the regular-file arm below.
            // Key "ino:devmajor:devminor" -> absolute path of the first
            // data-carrying extraction of that (c_ino, c_dev) identity.
            Map<String, String> hardlinkOwners = new HashMap<>();

            while (pos + 110 <= fileLength) {
                byte[] header = new byte[110];
                int headerRead = readFully(fis, header);
                if (headerRead < 110) break;
                pos += 110;

                String magic = new String(header, 0, 6);
                if (!magic.equals("070701") && !magic.equals("070702")) {
                    Log.w(TAG, "Invalid cpio magic at pos " + (pos - 110) + ": " + magic);
                    break;
                }

                // cpio newc header field offsets (each field is 8 hex chars,
                // magic is 6 bytes, total header = 110 bytes):
                //   0   magic ("070701")
                //   6   ino
                //   14  mode
                //   22  uid / 30 gid / 38 nlink / 46 mtime
                //   54  filesize
                //   62  devmajor / 70 devminor / 78 rdevmajor / 86 rdevminor
                //   94  namesize  <-- was incorrectly 62 before, reading devmajor
                //   102 check
                int mode = parseHex(header, 14, 8);
                long filesize = parseHexLong(header, 54, 8);
                int namesize = parseHex(header, 94, 8);
                // Task 6-Z82: hardlink identity fields (newc, 8 hex chars
                // each): c_ino @6, c_nlink @38, c_devmajor @62, c_devminor @70.
                int ino = parseHex(header, 6, 8);
                int nlink = parseHex(header, 38, 8);
                int devmajor = parseHex(header, 62, 8);
                int devminor = parseHex(header, 70, 8);

                if (namesize <= 0 || namesize > 4096) {
                    Log.w(TAG, "Invalid namesize: " + namesize);
                    break;
                }

                byte[] nameBytes = new byte[namesize];
                int nameRead = readFully(fis, nameBytes);
                if (nameRead < namesize) break;
                pos += namesize;

                String name = new String(nameBytes, 0, namesize - 1);

                // Align to 4 bytes after name
                int namePadding = ((4 - ((110 + namesize) % 4)) % 4);
                if (namePadding > 0) {
                    byte[] pad = new byte[namePadding];
                    readFully(fis, pad);
                    pos += namePadding;
                }

                if (name.equals("TRAILER!!!") || name.isEmpty()) {
                    Log.i(TAG, "Reached cpio trailer");
                    sawTrailer = true;
                    break;
                }

                long dataPos = pos;
                int modeType = mode & 0xF000;

                if (modeType == 0x4000) {
                    // Directory
                    File dirDest = safeTargetFile(targetDir, name);
                    if (dirDest == null) {
                        // Path traversal — skip the entry AND its data so the
                        // stream stays in sync.
                        long rem = filesize;
                        while (rem > 0) {
                            long skipped = fis.skip(rem);
                            if (skipped <= 0) {
                                byte[] skip = new byte[(int) Math.min(65536, rem)];
                                int n = fis.read(skip);
                                if (n < 0) break;
                                rem -= n;
                                pos += n;
                            } else {
                                rem -= skipped;
                                pos += skipped;
                            }
                        }
                        continue;
                    }
                    dirDest.mkdirs();
                    fileCount++;
                } else if (modeType == 0x8000) {
                    // Regular file
                    File file = safeTargetFile(targetDir, name);
                    if (file == null) {
                        // Path traversal — skip the entry's DATA bytes too so
                        // the cpio stream stays in sync.
                        long rem2 = filesize;
                        while (rem2 > 0) {
                            long skipped = fis.skip(rem2);
                            if (skipped <= 0) {
                                byte[] skip = new byte[(int) Math.min(65536, rem2)];
                                int n = fis.read(skip);
                                if (n < 0) break;
                                rem2 -= n;
                                pos += n;
                            } else {
                                rem2 -= skipped;
                                pos += skipped;
                            }
                        }
                        continue;
                    }
                    file.getParentFile().mkdirs();

                    // Task 6-Z82: cpio newc hardlink handling. Hardlinked
                    // files appear as multiple entries sharing (c_ino, c_dev);
                    // only the FIRST carries data, repeats have filesize=0
                    // (nlink>1 corroborates). The kernel's initramfs extractor
                    // materializes repeats via link(); this Java extractor used
                    // to write them as EMPTY regular files — leaving e.g.
                    // sbin/libcrecovery.so at 0 bytes -> guest linker
                    // "libcrecovery.so is too small to be an ELF" -> CANNOT
                    // LINK -> exit(1) x6 (E2E run 32619835085). Materialize
                    // repeats as real byte copies of the first data-carrying
                    // entry (Files.copy): identical content, safe on every
                    // filesystem, no link() permission requirements.
                    String linkKey = ino + ":" + devmajor + ":" + devminor;
                    boolean materializedHardlink = false;
                    if (filesize == 0 && nlink > 1) {
                        String firstPath = hardlinkOwners.get(linkKey);
                        File firstFile = firstPath != null ? new File(firstPath) : null;
                        if (firstFile != null && firstFile.isFile() && firstFile.length() > 0) {
                            Files.copy(firstFile.toPath(), file.toPath(),
                                StandardCopyOption.REPLACE_EXISTING);
                            materializedHardlink = true;
                            String msg = "hardlink materialized: " + name + " -> " + firstPath
                                + " (" + file.length() + " bytes)";
                            Log.i(TAG, msg);
                            FileLogger.i(TAG, msg);
                        } else {
                            // cpio convention is data-first (GNU cpio /
                            // gen_init_cpio never emit data-last); a repeat
                            // with no seen owner means a malformed archive.
                            // Keep the legacy empty-file behavior but say so —
                            // the post-import sbin/lib*.so scan in
                            // verifyCriticalPayload will fail loudly if this
                            // file matters.
                            Log.w(TAG, "hardlink entry '" + name + "' (nlink=" + nlink
                                + ", ino=" + ino + ") has no data-carrying first entry;"
                                + " writing empty file");
                        }
                    }

                    if (!materializedHardlink) {
                        try (FileOutputStream fos = new FileOutputStream(file)) {
                            byte[] buf = new byte[8192];
                            long remaining = filesize;
                            while (remaining > 0) {
                                int toRead = (int) Math.min(buf.length, remaining);
                                int n = fis.read(buf, 0, toRead);
                                if (n < 0) break;
                                fos.write(buf, 0, n);
                                remaining -= n;
                                pos += n;
                            }
                        }
                        // Per-entry size verification: a short/failed stream read used
                        // to silently `break` out of the loop above, leaving this file
                        // at whatever had been written so far (e.g. 0 bytes) while the
                        // import still reported SUCCESS (E2E run 32616016488:
                        // sbin/libminuitwrp.so = 0 bytes -> guest linker died "too
                        // small to be an ELF"). Fail loudly instead and do not leave
                        // the partially-written file behind.
                        // NOTE (6-Z82): this assertion is skipped for a
                        // materialized hardlink on purpose — the copy has the
                        // SOURCE entry's size, not this entry's declared
                        // filesize (0); Files.copy's success is its own
                        // verification (plus the post-import sbin/lib*.so ELF
                        // scan in verifyCriticalPayload).
                        if (file.length() != filesize) {
                            String msg = "CORRUPT cpio: entry '" + name + "' short write: want="
                                + filesize + " got=" + file.length() + " (stopped at " + pos
                                + "/" + fileLength + ")";
                            Log.e(TAG, msg);
                            FileLogger.e(TAG, msg);
                            if (!file.delete()) {
                                Log.e(TAG, "Failed to delete partially-written " + file);
                            }
                            throw new IOException(msg);
                        }
                    }
                    // Task 6-Z31: set executable permission from the cpio mode.
                    // Java's FileOutputStream creates files with mode 0644 (NOT
                    // executable). Without this, /sbin/recovery + /sbin/linker
                    // are created without the execute bit → execve fails with
                    // EACCES → exit 127 → TWRP recovery service never starts.
                    // The cpio mode's 0o100 bit (S_IXUSR) is the owner-execute
                    // bit. setExecutable(true) sets it (and group/other exec
                    // via setExecutable(true, false) if we want world-exec, but
                    // owner-exec is sufficient for execve).
                    boolean isExec = (mode & 0100) != 0;
                    if (isExec) {
                        file.setExecutable(true, false); // owner + group + other exec
                    }
                    // Also set readable (cpio mode 0o400 = S_IRUSR). Java's
                    // FileOutputStream creates files readable by owner by
                    // default, but be explicit to match the cpio mode.
                    if ((mode & 0400) != 0) {
                        file.setReadable(true, false);
                    }
                    // Also set writable for owner (cpio mode 0o200 = S_IWUSR).
                    if ((mode & 0200) != 0) {
                        file.setWritable(true, true); // owner-write only
                    }
                    // 6-Z82: remember this data-carrying entry as the copy
                    // source for any later hardlink repeats sharing
                    // (c_ino, c_dev).
                    if (filesize > 0) {
                        hardlinkOwners.put(linkKey, file.getAbsolutePath());
                    }
                    fileCount++;
                } else if (modeType == 0xA000) {
                    // Symlink — save target as .symlink file
                    byte[] target = new byte[(int) filesize];
                    readFully(fis, target);
                    pos += filesize;
                    File linkBase = safeTargetFile(targetDir, name);
                    if (linkBase == null) {
                        continue; // path-traversal entry — data already consumed
                    }
                    File linkFile = new File(linkBase.getPath() + ".symlink");
                    linkFile.getParentFile().mkdirs();
                    try (FileOutputStream fos = new FileOutputStream(linkFile)) {
                        fos.write(target);
                    }
                    fileCount++;
                } else {
                    // Skip device nodes, fifos, etc.
                    long remaining = filesize;
                    while (remaining > 0) {
                        long skipped = fis.skip(remaining);
                        if (skipped <= 0) {
                            byte[] skip = new byte[(int) Math.min(65536, remaining)];
                            int n = fis.read(skip);
                            if (n < 0) break;
                            remaining -= n;
                            pos += n;
                        } else {
                            remaining -= skipped;
                            pos += skipped;
                        }
                    }
                }

                // Align to 4 bytes after data
                long dataEnd = dataPos + filesize;
                long dataPadding = ((4 - (dataEnd % 4)) % 4);
                if (dataPadding > 0) {
                    byte[] pad = new byte[(int) dataPadding];
                    readFully(fis, pad);
                    pos += dataPadding;
                }
            }

            Log.i(TAG, "Extracted " + fileCount + " entries from cpio (sawTrailer=" + sawTrailer + ")");
            if (fileCount == 0) {
                throw new IOException("CPIO archive contained 0 entries (parsed magic="
                    + (fileLength > 0 ? "valid" : "empty")
                    + ", fileLength=" + fileLength + ")");
            }
            if (!sawTrailer) {
                // The stream ended (or desynced) before the cpio TRAILER!!!
                // marker: every silent `break` above (EOF mid-header,
                // mid-name or mid-data) lands here. A truncated import must
                // never be reported as SUCCESS.
                String msg = "CORRUPT cpio: no TRAILER!!! entry — archive truncated (extracted "
                    + fileCount + " entries, stopped at " + pos + "/" + fileLength + ")";
                Log.e(TAG, msg);
                FileLogger.e(TAG, msg);
                throw new IOException(msg);
            }
            // Second-layer defense: verify the critical payload actually landed
            // on disk with the expected sizes before reporting success.
            verifyCriticalPayload(targetDir);
            return true;
        }
    }

    /**
     * Post-import verification pass (flake E2E run 32616016488): a
     * TWRP-style payload must contain the critical sbin/ files at their
     * exact expected sizes (from the cpio headers of the bundled
     * twrp-3.7.0_9-0-byt_t_crv2.img). Extraction-loop accounting can miss
     * on-disk state (e.g. silent write loss), so re-check the real files.
     *
     * Exact sizes are only enforced when this is the known bundled payload
     * (detected via sbin/recovery == 1271264); any other recovery image
     * merely gets a non-zero sanity check so unrelated imports cannot
     * false-fail on build-specific size differences.
     */
    private static void verifyCriticalPayload(File targetDir) throws IOException {
        File recovery = new File(targetDir, "sbin/recovery");
        if (!recovery.exists()) {
            return; // not a recovery-style payload (e.g. plain rootfs) — nothing to verify
        }

        StringBuilder problems = new StringBuilder();
        long recoveryLen = recovery.length();

        if (recoveryLen == 1271264L) {
            // Known bundled payload — enforce the exact manifest.
            // sbin/libtwrp_fb_hook.so is deliberately NOT in this manifest:
            // it is staged into the rootfs by the app AFTER import (from the
            // APK native lib dir) — not part of the stock ramdisk; run
            // 32619179713 aborted because this was required at import time.
            String[][] exact = {
                {"sbin/recovery", "1271264"},
                {"sbin/linker", "148291"},
                {"sbin/libminuitwrp.so", "129364"},
            };
            for (String[] spec : exact) {
                File f = new File(targetDir, spec[0]);
                long want = Long.parseLong(spec[1]);
                if (!f.exists()) {
                    problems.append(spec[0]).append(" MISSING; ");
                } else if (f.length() != want) {
                    problems.append(spec[0]).append(" size=").append(f.length())
                        .append(" want=").append(want).append("; ");
                }
            }
        }

        // For any recovery payload: critical files must at least be non-zero
        // (libtwrp.so / libguitwrp.so are build-specific — only when present).
        // libtwrp_fb_hook.so is OPTIONAL here too: staged into the rootfs by
        // the app AFTER import (from the APK native lib dir) — not part of
        // the stock ramdisk; run 32619179713 aborted because this was
        // required at import time. If present, require >0; if absent, skip.
        String[] nonZero = {"sbin/recovery", "sbin/linker", "sbin/libminuitwrp.so",
            "sbin/libcrecovery.so",
            "sbin/libtwrp_fb_hook.so", "sbin/libtwrp.so", "sbin/libguitwrp.so"};
        for (String n : nonZero) {
            File f = new File(targetDir, n);
            if (f.exists() && f.length() <= 0) {
                problems.append(n).append(" size=0; ");
            }
        }

        // Task 6-Z82: scan EVERY sbin/lib*.so — the guest linker DT_NEEDED-
        // loads these; any 0-byte or non-ELF lib aborts linking before main()
        // ("too small to be an ELF", E2E run 32619835085: sbin/libcrecovery.so
        // = 0 bytes slipped past the fixed manifest above). Require length>0
        // AND first 4 bytes == ELF magic (0x7F 'E' 'L' 'F'). Fail loudly
        // listing EVERY bad file (sorted for deterministic messages).
        File sbinDir = new File(targetDir, "sbin");
        File[] sbinFiles = sbinDir.listFiles();
        if (sbinFiles != null) {
            Arrays.sort(sbinFiles);
            for (File f : sbinFiles) {
                String fname = f.getName();
                if (!f.isFile() || !fname.startsWith("lib") || !fname.endsWith(".so")) {
                    continue;
                }
                long len = f.length();
                if (len <= 0) {
                    problems.append("sbin/").append(fname).append(" size=0; ");
                    continue;
                }
                byte[] magic = new byte[4];
                int magicRead;
                try (FileInputStream efis = new FileInputStream(f)) {
                    magicRead = readFully(efis, magic);
                }
                if (magicRead < 4 || (magic[0] & 0xFF) != 0x7F || magic[1] != 'E'
                    || magic[2] != 'L' || magic[3] != 'F') {
                    problems.append("sbin/").append(fname).append(" not-ELF (")
                        .append(len).append(" bytes); ");
                }
            }
        }

        if (problems.length() > 0) {
            String msg = "CORRUPT import — payload verification failed: " + problems;
            Log.e(TAG, msg);
            FileLogger.e(TAG, msg);
            throw new IOException(msg);
        }
        Log.i(TAG, "Payload verification OK (recovery=" + recoveryLen + " bytes)");
    }

    private static int readFully(InputStream is, byte[] buf) throws IOException {
        int total = 0;
        while (total < buf.length) {
            int n = is.read(buf, total, buf.length - total);
            if (n < 0) return total;
            total += n;
        }
        return total;
    }

    private static int parseHex(byte[] data, int offset, int len) {
        return (int) parseHexLong(data, offset, len);
    }

    private static long parseHexLong(byte[] data, int offset, int len) {
        long result = 0;
        for (int i = 0; i < len; i++) {
            byte c = data[offset + i];
            int val;
            if (c >= '0' && c <= '9') val = c - '0';
            else if (c >= 'a' && c <= 'f') val = c - 'a' + 10;
            else if (c >= 'A' && c <= 'F') val = c - 'A' + 10;
            else return result;
            result = (result << 4) | val;
        }
        return result;
    }
}
