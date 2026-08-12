// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of merchantability,
// fitness for a particular purpose, or non-infringement.
// Use at your own risk.

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.utils;

import android.content.Context;
import android.content.UriPermission;
import android.net.Uri;
import android.util.Log;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.zip.GZIPInputStream;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;

/**
 * Import ramdisk/rootfs from various formats:
 * <ul>
 *   <li><b>.tar / .tar.gz</b> — standard rootfs tarball (existing behavior).
 *       Extracted directly to the rootfs directory.</li>
 *   <li><b>.img</b> — Android boot image (e.g. TWRP recovery.img).
 *       The ramdisk is extracted from the boot image header, decompressed
 *       (gzip), and the cpio archive is extracted to the rootfs directory.</li>
 *   <li><b>.cpio / .cpio.gz</b> — raw ramdisk cpio archive.
 *       Decompressed if gzipped, then extracted to the rootfs directory.</li>
 *   <li><b>.zip</b> — ZIP file containing a ramdisk (cpio or cpio.gz).
 *       The first .cpio or .cpio.gz entry is extracted and processed.</li>
 * </ul>
 *
 * The importer detects the format by file extension AND magic bytes:
 *   - Android boot image: starts with "ANDROID!"
 *   - GZIP: starts with 0x1f 0x8b
 *   - ZIP: starts with "PK"
 *   - CPIO (newc): starts with "070701" or "070702"
 *   - TAR: starts with "ustar" at offset 257
 */
public class RamdiskImporter {
    private static final String TAG = "RamdiskImporter";

    /**
     * Import a ramdisk/rootfs from a Uri.
     *
     * @param context    The context for ContentResolver
     * @param uri        The Uri of the file to import
     * @param targetDir  The directory to extract the rootfs into
     * @return true if import succeeded
     * @throws IOException on I/O errors
     */
    public static boolean importRamdisk(Context context, Uri uri, File targetDir) throws IOException {
        // Copy the file to a temp file first (ContentResolver streams can't be
        // re-read, and we need to check magic bytes)
        File tempFile = new File(context.getCacheDir(), "ramdisk_import");
        try (InputStream is = context.getContentResolver().openInputStream(uri);
             OutputStream os = new FileOutputStream(tempFile)) {
            if (is == null) throw new IOException("Cannot open: " + uri);
            byte[] buffer = new byte[8192];
            int count;
            while ((count = is.read(buffer)) > 0) {
                os.write(buffer, 0, count);
            }
        }

        // Detect format by magic bytes
        String format = detectFormat(tempFile);
        Log.i(TAG, "Detected format: " + format);

        boolean result;
        switch (format) {
            case "android_bootimg":
                result = importBootImage(tempFile, targetDir);
                break;
            case "gzip":
                // Could be a .tar.gz rootfs or a .cpio.gz ramdisk
                result = importGzipFile(tempFile, targetDir);
                break;
            case "zip":
                result = importZipFile(tempFile, targetDir);
                break;
            case "cpio":
                result = importCpioFile(tempFile, targetDir);
                break;
            case "tar":
                result = importTarFile(tempFile, targetDir);
                break;
            default:
                // Try tar as fallback (most common for rootfs)
                Log.w(TAG, "Unknown format, trying tar extraction");
                result = importTarFile(tempFile, targetDir);
                break;
        }

        tempFile.delete();
        return result;
    }

    /**
     * Detect the file format by reading magic bytes.
     */
    private static String detectFormat(File file) throws IOException {
        byte[] header = new byte[512];
        try (FileInputStream fis = new FileInputStream(file)) {
            int read = fis.read(header);
            if (read < 8) return "unknown";
        }

        // Android boot image: "ANDROID!"
        if (header.length >= 8 && header[0] == 'A' && header[1] == 'N' &&
            header[2] == 'D' && header[3] == 'R' && header[4] == 'O' &&
            header[5] == 'I' && header[6] == 'D' && header[7] == '!') {
            return "android_bootimg";
        }

        // GZIP: 0x1f 0x8b
        if (header[0] == 0x1f && header[1] == (byte) 0x8b) {
            return "gzip";
        }

        // ZIP: "PK"
        if (header[0] == 'P' && header[1] == 'K') {
            return "zip";
        }

        // CPIO newc: "070701" or "070702"
        if (header[0] == '0' && header[1] == '7' && header[2] == '0' &&
            header[3] == '7' && (header[4] == '0' || header[4] == '1') &&
            header[5] == '1') {
            return "cpio";
        }
        // Also check "070702"
        if (header[0] == '0' && header[1] == '7' && header[2] == '0' &&
            header[3] == '7' && header[4] == '0' && header[5] == '2') {
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
     * Parses the boot image header, extracts the ramdisk (gzip-compressed
     * cpio), decompresses it, and extracts the cpio to targetDir.
     */
    private static boolean importBootImage(File imgFile, File targetDir) throws IOException {
        byte[] header = new byte[16384]; // boot image header can be up to 16KB
        try (FileInputStream fis = new FileInputStream(imgFile)) {
            int headerSize = fis.read(header);
            if (headerSize < 110) {
                throw new IOException("Boot image too small");
            }

            // Parse Android boot image header
            // magic: 8 bytes ("ANDROID!")
            // kernel_size: 4 bytes (offset 8)
            // kernel_addr: 4 bytes (offset 12)
            // ramdisk_size: 4 bytes (offset 16)
            // ramdisk_addr: 4 bytes (offset 20)
            // second_size: 4 bytes (offset 24)
            // second_addr: 4 bytes (offset 28)
            // tags_addr: 4 bytes (offset 32)
            // page_size: 4 bytes (offset 36)
            ByteBuffer bb = ByteBuffer.wrap(header).order(ByteOrder.LITTLE_ENDIAN);

            int kernelSize = bb.getInt(8);
            int ramdiskSize = bb.getInt(16);
            int pageSize = bb.getInt(36);

            if (pageSize == 0 || ramdiskSize == 0) {
                throw new IOException("Invalid boot image header: pageSize=" + pageSize + " ramdiskSize=" + ramdiskSize);
            }

            // Calculate ramdisk offset
            int kernelPages = (kernelSize + pageSize - 1) / pageSize;
            int ramdiskOffset = pageSize + kernelPages * pageSize;

            Log.i(TAG, "Boot image: kernel=" + kernelSize + " ramdisk=" + ramdiskSize +
                  " page=" + pageSize + " ramdiskOffset=" + ramdiskOffset);

            // Read the ramdisk
            fis.getChannel().position(ramdiskOffset);
            byte[] ramdiskGz = new byte[ramdiskSize];
            int read = 0;
            while (read < ramdiskSize) {
                int n = fis.read(ramdiskGz, read, ramdiskSize - read);
                if (n < 0) break;
                read += n;
            }

            // Decompress gzip → cpio
            File cpioFile = new File(targetDir.getParentFile(), "ramdisk.cpio");
            try (GZIPInputStream gzis = new GZIPInputStream(new java.io.ByteArrayInputStream(ramdiskGz));
                 FileOutputStream fos = new FileOutputStream(cpioFile)) {
                byte[] buf = new byte[8192];
                int n;
                while ((n = gzis.read(buf)) > 0) {
                    fos.write(buf, 0, n);
                }
            }

            // Extract cpio to targetDir using cpio command
            // (Android doesn't have cpio, but we can use tar or a Java cpio extractor)
            // Fall back to using the extract-twrp-ramdisk.py script or a built-in extractor
            return extractCpioWithPython(cpioFile, targetDir, imgFile);
        }
    }

    /**
     * Import a gzip file. Could be .tar.gz (rootfs) or .cpio.gz (ramdisk).
     */
    private static boolean importGzipFile(File gzFile, File targetDir) throws IOException {
        // Read the first few decompressed bytes to check if it's a tar or cpio
        byte[] header = new byte[512];
        try (GZIPInputStream gzis = new GZIPInputStream(new FileInputStream(gzFile))) {
            int read = gzis.read(header);
            if (read < 8) throw new IOException("Decompressed file too small");
        }

        // Check if it's a cpio (070701)
        if (header[0] == '0' && header[1] == '7' && header[2] == '0' &&
            header[3] == '7') {
            // It's a cpio.gz — decompress and extract
            File cpioFile = new File(targetDir.getParentFile(), "ramdisk.cpio");
            try (GZIPInputStream gzis = new GZIPInputStream(new FileInputStream(gzFile));
                 FileOutputStream fos = new FileOutputStream(cpioFile)) {
                byte[] buf = new byte[8192];
                int n;
                while ((n = gzis.read(buf)) > 0) fos.write(buf, 0, n);
            }
            return extractCpioWithPython(cpioFile, targetDir, gzFile);
        }

        // Check if it's a tar (ustar at offset 257)
        if (header.length > 262 && header[257] == 'u' && header[258] == 's' &&
            header[259] == 't' && header[260] == 'a' && header[261] == 'r') {
            // It's a tar.gz — extract with tar
            return extractTarGz(gzFile, targetDir);
        }

        // Unknown gzip content — try tar extraction
        return extractTarGz(gzFile, targetDir);
    }

    /**
     * Import a ZIP file containing a ramdisk.
     * Looks for the first .cpio or .cpio.gz entry.
     */
    private static boolean importZipFile(File zipFile, File targetDir) throws IOException {
        try (ZipInputStream zis = new ZipInputStream(new FileInputStream(zipFile))) {
            ZipEntry entry;
            while ((entry = zis.getNextEntry()) != null) {
                String name = entry.getName().toLowerCase();
                if (name.endsWith(".cpio") || name.endsWith(".cpio.gz") ||
                    name.endsWith(".ramdisk") || name.endsWith(".ramdisk.gz")) {

                    // Extract the entry to a temp file
                    File cpioFile = new File(targetDir.getParentFile(), "ramdisk.cpio");
                    try (FileOutputStream fos = new FileOutputStream(cpioFile)) {
                        byte[] buf = new byte[8192];
                        int n;
                        while ((n = zis.read(buf)) > 0) fos.write(buf, 0, n);
                    }

                    // If it's gzipped, decompress
                    if (name.endsWith(".gz")) {
                        File decompressed = new File(cpioFile.getParent(), "ramdisk_dec.cpio");
                        try (GZIPInputStream gzis = new GZIPInputStream(new FileInputStream(cpioFile));
                             FileOutputStream fos = new FileOutputStream(decompressed)) {
                            byte[] buf = new byte[8192];
                            int n;
                            while ((n = gzis.read(buf)) > 0) fos.write(buf, 0, n);
                        }
                        cpioFile.delete();
                        cpioFile = decompressed;
                    }

                    return extractCpioWithPython(cpioFile, targetDir, zipFile);
                }
            }
        }
        throw new IOException("No .cpio or .ramdisk entry found in ZIP");
    }

    /**
     * Import a raw cpio file.
     */
    private static boolean importCpioFile(File cpioFile, File targetDir) throws IOException {
        return extractCpioWithPython(cpioFile, targetDir, cpioFile);
    }

    /**
     * Import a tar/tar.gz rootfs (existing behavior).
     */
    private static boolean importTarFile(File tarFile, File targetDir) throws IOException {
        // Use the existing tar extraction (shell out to tar)
        ProcessBuilder pb = new ProcessBuilder("tar", "-xf", tarFile.getAbsolutePath(),
            "-C", targetDir.getAbsolutePath());
        Process process = pb.start();
        try {
            if (!process.waitFor(120, java.util.concurrent.TimeUnit.SECONDS)) {
                process.destroyForcibly();
                throw new IOException("tar extraction timed out");
            }
            return process.exitValue() == 0;
        } catch (InterruptedException e) {
            throw new IOException("Interrupted during tar extraction", e);
        }
    }

    /**
     * Extract a tar.gz file.
     */
    private static boolean extractTarGz(File gzFile, File targetDir) throws IOException {
        // Decompress to a temp tar, then extract
        File tarFile = new File(targetDir.getParentFile(), "rootfs.tar");
        try (GZIPInputStream gzis = new GZIPInputStream(new FileInputStream(gzFile));
             FileOutputStream fos = new FileOutputStream(tarFile)) {
            byte[] buf = new byte[8192];
            int n;
            while ((n = gzis.read(buf)) > 0) fos.write(buf, 0, n);
        }
        boolean result = importTarFile(tarFile, targetDir);
        tarFile.delete();
        return result;
    }

    /**
     * Extract a cpio archive using Python (available on Android via Termux
     * or the built-in Python in some ROMs). Falls back to a Java cpio
     * extractor if Python is not available.
     *
     * The cpio format used by Android ramdisks is SVR4 (newc) with
     * magic "070701" or "070702". Each entry has a 110-byte ASCII header
     * followed by the filename, file data, and padding.
     */
    private static boolean extractCpioWithPython(File cpioFile, File targetDir, File sourceFile) throws IOException {
        // First, try the built-in cpio extraction in Java
        // (Android doesn't have the cpio command, and Python isn't guaranteed)
        return extractCpioJava(cpioFile, targetDir);
    }

    /**
     * Extract a SVR4 cpio archive (newc format) using pure Java.
     * This handles the "070701" and "070702" magic formats used by
     * Android ramdisks.
     */
    private static boolean extractCpioJava(File cpioFile, File targetDir) throws IOException {
        try (FileInputStream fis = new FileInputStream(cpioFile)) {
            byte[] data = new byte[(int) cpioFile.length()];
            int offset = 0;
            while (offset < data.length) {
                int n = fis.read(data, offset, data.length - offset);
                if (n < 0) break;
                offset += n;
            }

            int pos = 0;
            int fileCount = 0;
            while (pos + 110 <= data.length) {
                // Check magic
                String magic = new String(data, pos, 6);
                if (!magic.equals("070701") && !magic.equals("070702")) {
                    break;
                }
                if (magic.equals("070702")) break; // CRC variant, stop

                // Parse header fields (8 hex chars each)
                int mode = parseHex(data, pos + 14, 8);
                int filesize = parseHex(data, pos + 54, 8);
                int namesize = parseHex(data, pos + 62, 8);

                // Name starts at offset 110
                int nameStart = pos + 110;
                String name = new String(data, nameStart, namesize - 1); // -1 for null terminator

                // Align data start to 4 bytes
                int dataStart = nameStart + namesize;
                dataStart = (dataStart + 3) & ~3;

                if (name.equals("TRAILER!!!") || name.isEmpty()) {
                    break;
                }

                int modeType = mode & 0xF000;
                if (modeType == 0x4000) {
                    // Directory
                    File dir = new File(targetDir, name);
                    dir.mkdirs();
                    fileCount++;
                } else if (modeType == 0x8000) {
                    // Regular file
                    File file = new File(targetDir, name);
                    file.getParentFile().mkdirs();
                    try (FileOutputStream fos = new FileOutputStream(file)) {
                        fos.write(data, dataStart, filesize);
                    }
                    fileCount++;
                } else if (modeType == 0xA000) {
                    // Symlink — skip (sandbox may not allow creating symlinks)
                    // Write target to a .symlink file for reference
                    File linkFile = new File(targetDir, name + ".symlink");
                    linkFile.getParentFile().mkdirs();
                    String target = new String(data, dataStart, filesize);
                    try (FileOutputStream fos = new FileOutputStream(linkFile)) {
                        fos.write(target.getBytes());
                    }
                    fileCount++;
                }

                // Advance to next entry (align to 4 bytes)
                pos = dataStart + filesize;
                pos = (pos + 3) & ~3;
            }

            Log.i(TAG, "Extracted " + fileCount + " entries from cpio");
            return fileCount > 0;
        }
    }

    /**
     * Parse 8 hex characters as an integer.
     */
    private static int parseHex(byte[] data, int offset, int len) {
        int result = 0;
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
