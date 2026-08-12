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
 *   <li><b>.tar / .tar.gz</b> — standard rootfs tarball (existing behavior).</li>
 *   <li><b>.img</b> — Android boot image (e.g. TWRP recovery.img).</li>
 *   <li><b>.cpio / .cpio.gz</b> — raw ramdisk cpio archive.</li>
 *   <li><b>.zip</b> — ZIP file containing a ramdisk.</li>
 * </ul>
 */
public class RamdiskImporter {
    private static final String TAG = "RamdiskImporter";

    public static boolean importRamdisk(Context context, Uri uri, File targetDir) throws IOException {
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

        Log.i(TAG, "Import file size: " + tempFile.length() + " bytes");

        String format = detectFormat(tempFile);
        Log.i(TAG, "Detected format: " + format);

        boolean result;
        switch (format) {
            case "android_bootimg":
                result = importBootImage(tempFile, targetDir);
                break;
            case "gzip":
                result = importGzipFile(tempFile, targetDir);
                break;
            case "zip":
                result = importZipFile(tempFile, targetDir);
                break;
            case "cpio":
                result = extractCpioStreaming(tempFile, targetDir);
                break;
            case "tar":
                result = importTarFile(tempFile, targetDir);
                break;
            default:
                Log.w(TAG, "Unknown format, trying tar then cpio");
                // Try tar first, then cpio as fallback
                result = importTarFile(tempFile, targetDir);
                if (!result) {
                    Log.w(TAG, "tar failed, trying cpio");
                    result = extractCpioStreaming(tempFile, targetDir);
                }
                break;
        }

        tempFile.delete();
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

    private static boolean importBootImage(File imgFile, File targetDir) throws IOException {
        try (FileInputStream fis = new FileInputStream(imgFile)) {
            byte[] header = new byte[16384];
            int headerSize = fis.read(header);
            if (headerSize < 110) throw new IOException("Boot image too small");

            ByteBuffer bb = ByteBuffer.wrap(header).order(ByteOrder.LITTLE_ENDIAN);
            int kernelSize = bb.getInt(8);
            int ramdiskSize = bb.getInt(16);
            int pageSize = bb.getInt(36);

            if (pageSize == 0 || ramdiskSize == 0) {
                throw new IOException("Invalid boot image: pageSize=" + pageSize + " ramdiskSize=" + ramdiskSize);
            }

            int kernelPages = (kernelSize + pageSize - 1) / pageSize;
            int ramdiskOffset = pageSize + kernelPages * pageSize;

            Log.i(TAG, "Boot image: kernel=" + kernelSize + " ramdisk=" + ramdiskSize +
                  " page=" + pageSize + " ramdiskOffset=" + ramdiskOffset);

            fis.getChannel().position(ramdiskOffset);
            byte[] ramdiskGz = new byte[ramdiskSize];
            int read = 0;
            while (read < ramdiskSize) {
                int n = fis.read(ramdiskGz, read, ramdiskSize - read);
                if (n < 0) break;
                read += n;
            }

            // Decompress gzip -> cpio, then extract streaming
            File cpioTemp = new File(targetDir.getParentFile(), "ramdisk.cpio");
            try (GZIPInputStream gzis = new GZIPInputStream(new java.io.ByteArrayInputStream(ramdiskGz));
                 FileOutputStream fos = new FileOutputStream(cpioTemp)) {
                byte[] buf = new byte[8192];
                int n;
                while ((n = gzis.read(buf)) > 0) fos.write(buf, 0, n);
            }

            boolean result = extractCpioStreaming(cpioTemp, targetDir);
            cpioTemp.delete();
            return result;
        }
    }

    private static boolean importGzipFile(File gzFile, File targetDir) throws IOException {
        // Peek at decompressed content to determine if it's tar or cpio
        byte[] header = new byte[512];
        try (GZIPInputStream gzis = new GZIPInputStream(new FileInputStream(gzFile))) {
            int read = gzis.read(header);
            if (read < 8) throw new IOException("Decompressed file too small");
        }

        // Check if it's a cpio (070701)
        if (header[0] == '0' && header[1] == '7' && header[2] == '0' && header[3] == '7') {
            // It's a cpio.gz — decompress to temp file, then extract
            File cpioTemp = new File(targetDir.getParentFile(), "ramdisk.cpio");
            try (GZIPInputStream gzis = new GZIPInputStream(new FileInputStream(gzFile));
                 FileOutputStream fos = new FileOutputStream(cpioTemp)) {
                byte[] buf = new byte[8192];
                int n;
                while ((n = gzis.read(buf)) > 0) fos.write(buf, 0, n);
            }
            boolean result = extractCpioStreaming(cpioTemp, targetDir);
            cpioTemp.delete();
            return result;
        }

        // Assume tar.gz
        return extractTarGz(gzFile, targetDir);
    }

    private static boolean importZipFile(File zipFile, File targetDir) throws IOException {
        try (ZipInputStream zis = new ZipInputStream(new FileInputStream(zipFile))) {
            ZipEntry entry;
            while ((entry = zis.getNextEntry()) != null) {
                String name = entry.getName().toLowerCase();
                if (name.endsWith(".cpio") || name.endsWith(".cpio.gz") ||
                    name.endsWith(".ramdisk") || name.endsWith(".ramdisk.gz")) {

                    File cpioTemp = new File(targetDir.getParentFile(), "ramdisk.cpio");
                    try (FileOutputStream fos = new FileOutputStream(cpioTemp)) {
                        byte[] buf = new byte[8192];
                        int n;
                        while ((n = zis.read(buf)) > 0) fos.write(buf, 0, n);
                    }

                    // Decompress if gzipped
                    if (name.endsWith(".gz")) {
                        File decompressed = new File(cpioTemp.getParent(), "ramdisk_dec.cpio");
                        try (GZIPInputStream gzis = new GZIPInputStream(new FileInputStream(cpioTemp));
                             FileOutputStream fos = new FileOutputStream(decompressed)) {
                            byte[] buf = new byte[8192];
                            int n;
                            while ((n = gzis.read(buf)) > 0) fos.write(buf, 0, n);
                        }
                        cpioTemp.delete();
                        cpioTemp = decompressed;
                    }

                    boolean result = extractCpioStreaming(cpioTemp, targetDir);
                    cpioTemp.delete();
                    return result;
                }
            }
        }
        throw new IOException("No .cpio or .ramdisk entry found in ZIP");
    }

    private static boolean importTarFile(File tarFile, File targetDir) throws IOException {
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

    private static boolean extractTarGz(File gzFile, File targetDir) throws IOException {
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
     * Extract a SVR4 cpio archive (newc format) using streaming.
     * Reads the file in chunks instead of loading everything into memory.
     */
    private static boolean extractCpioStreaming(File cpioFile, File targetDir) throws IOException {
        try (FileInputStream fis = new FileInputStream(cpioFile)) {
            int fileCount = 0;
            long fileLength = cpioFile.length();
            long pos = 0;

            while (pos + 110 <= fileLength) {
                // Read 110-byte header
                byte[] header = new byte[110];
                int headerRead = readFully(fis, header);
                if (headerRead < 110) break;
                pos += 110;

                // Check magic
                String magic = new String(header, 0, 6);
                if (!magic.equals("070701") && !magic.equals("070702")) {
                    Log.w(TAG, "Invalid cpio magic at pos " + (pos - 110) + ": " + magic);
                    break;
                }

                // Parse header fields
                int mode = parseHex(header, 14, 8);
                long filesize = parseHexLong(header, 54, 8);
                int namesize = parseHex(header, 62, 8);

                if (namesize <= 0 || namesize > 4096) {
                    Log.w(TAG, "Invalid namesize: " + namesize);
                    break;
                }

                // Read filename
                byte[] nameBytes = new byte[namesize];
                int nameRead = readFully(fis, nameBytes);
                if (nameRead < namesize) break;
                pos += namesize;

                String name = new String(nameBytes, 0, namesize - 1); // -1 for null

                // Align to 4 bytes after name
                int namePadding = ((4 - ((110 + namesize) % 4)) % 4);
                if (namePadding > 0) {
                    byte[] pad = new byte[namePadding];
                    readFully(fis, pad);
                    pos += namePadding;
                }

                if (name.equals("TRAILER!!!") || name.isEmpty()) {
                    Log.i(TAG, "Reached cpio trailer");
                    break;
                }

                // Read file data
                long dataPos = pos;
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
                    fileCount++;
                } else if (modeType == 0xA000) {
                    // Symlink — read target, save as .symlink file
                    byte[] target = new byte[(int) filesize];
                    readFully(fis, target);
                    pos += filesize;

                    File linkFile = new File(targetDir, name + ".symlink");
                    linkFile.getParentFile().mkdirs();
                    try (FileOutputStream fos = new FileOutputStream(linkFile)) {
                        fos.write(target);
                    }
                    fileCount++;
                } else {
                    // Skip unknown types (char devices, block devices, fifos)
                    if (filesize > 0) {
                        byte[] skip = new byte[(int) Math.min(filesize, 65536)];
                        long remaining = filesize;
                        while (remaining > 0) {
                            int toRead = (int) Math.min(skip.length, remaining);
                            int n = fis.read(skip, 0, toRead);
                            if (n < 0) break;
                            remaining -= n;
                            pos += n;
                        }
                    }
                }

                // Align to 4 bytes after data
                long dataEnd = dataPos + filesize;
                long dataPadding = ((4 - (dataEnd % 4)) % 4);
                if (dataPadding > 0 && (modeType == 0xA000 || modeType == 0x8000)) {
                    // Already read the data above, just skip padding
                    byte[] pad = new byte[(int) dataPadding];
                    readFully(fis, pad);
                    pos += dataPadding;
                } else if (dataPadding > 0) {
                    // For skipped entries, data was already skipped including padding
                    // This case shouldn't happen since we skip inline above
                }
            }

            Log.i(TAG, "Extracted " + fileCount + " entries from cpio");
            return fileCount > 0;
        }
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
