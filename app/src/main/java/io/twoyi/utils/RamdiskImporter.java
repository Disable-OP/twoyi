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
        FileLogger.i(TAG, "Import file size: " + tempFile.length() + " bytes");

        String format = detectFormat(tempFile);
        Log.i(TAG, "Detected format: " + format);
        FileLogger.i(TAG, "Detected format: " + format);
        FileLogger.boot("ramdisk_import_format_detected", "format=" + format + " size=" + tempFile.length());

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

        tempFile.delete();
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
                if (entry.isDirectory()) {
                    new File(targetDir, entry.getName()).mkdirs();
                    count++;
                } else {
                    File f = new File(targetDir, entry.getName());
                    f.getParentFile().mkdirs();
                    try (FileOutputStream fos = new FileOutputStream(f)) {
                        byte[] buf = new byte[8192];
                        int n;
                        while ((n = tais.read(buf)) > 0) fos.write(buf, 0, n);
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
                    break;
                }

                long dataPos = pos;
                int modeType = mode & 0xF000;

                if (modeType == 0x4000) {
                    // Directory
                    new File(targetDir, name).mkdirs();
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
                    // Symlink — save target as .symlink file
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

            Log.i(TAG, "Extracted " + fileCount + " entries from cpio");
            if (fileCount == 0) {
                throw new IOException("CPIO archive contained 0 entries (parsed magic="
                    + (fileLength > 0 ? "valid" : "empty")
                    + ", fileLength=" + fileLength + ")");
            }
            return true;
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
