/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.utils;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertTrue;

import org.junit.Rule;
import org.junit.Test;
import org.junit.rules.TemporaryFolder;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.zip.GZIPOutputStream;

/**
 * 6-Z272i — host-side regression tests for the boot-image importer.
 *
 * Background: the 2026-09-03 corpus sweep showed an entire failure class
 * where large boot images (LineageOS 22.2 sailfish, 192 MiB; plus a
 * 59 MiB OrangeFox-family trio) silently stalled the import at
 * ramdisk_import_format_detected. Root causes: (1) v3/v4 ramdisk_size
 * was read from offset 16 (os_version territory) instead of 12, decoding
 * a bogus 512 MiB size; (2) the full `new byte[ramdiskSize]` read OOMed
 * the app heap and the Error vanished into the executor.
 *
 * These tests build synthetic boot images with the exact header layouts
 * and assert the importer (a) decodes v4 ramdisk_size@12, (b) streams
 * the ramdisk region correctly (offset math), (c) lands the cpio payload
 * in the staging dir.
 */
public class RamdiskImporterTest {

    @Rule
    public TemporaryFolder tmp = new TemporaryFolder();

    private static final int PAGE = 4096;

    /** Build a cpio (newc) archive containing one regular file. */
    private static byte[] buildCpio(String path, byte[] content) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        try {
            // newc entry: magic(6) + 13 × 8-hex fields + name + data
            StringBuilder sb = new StringBuilder();
            sb.append("070701");                              // magic
            sb.append(String.format("%08X", 1));              // ino
            sb.append(String.format("%08X", 0x81A4));         // mode: 0100644
            sb.append(String.format("%08X", 0));              // uid
            sb.append(String.format("%08X", 0));              // gid
            sb.append(String.format("%08X", 1));              // nlink
            sb.append(String.format("%08X", 0));              // mtime
            sb.append(String.format("%08X", content.length)); // filesize
            sb.append(String.format("%08X", 0));              // devmajor
            sb.append(String.format("%08X", 0));              // devminor
            sb.append(String.format("%08X", 0));              // rdevmajor
            sb.append(String.format("%08X", 0));              // rdevminor
            sb.append(String.format("%08X", path.length() + 1)); // namesize
            sb.append(String.format("%08X", 0));              // check
            byte[] nameBytes = path.getBytes(StandardCharsets.UTF_8);
            byte[] name = new byte[nameBytes.length + 1];
            System.arraycopy(nameBytes, 0, name, 0, nameBytes.length); // NUL-terminated
            out.write(sb.toString().getBytes(StandardCharsets.US_ASCII));
            out.write(name);
            pad4(out, out.size());
            out.write(content);
            pad4(out, out.size());
            // Trailer entry ("TRAILER!!!")
            StringBuilder tb = new StringBuilder();
            tb.append("070701");
            for (int i = 0; i < 11; i++) tb.append(String.format("%08X", 0));
            tb.append(String.format("%08X", 11));             // namesize (NUL incl.)
            tb.append(String.format("%08X", 0));              // check
            byte[] trailerName = new byte[11];
            byte[] tn = "TRAILER!!!".getBytes(StandardCharsets.US_ASCII);
            System.arraycopy(tn, 0, trailerName, 0, tn.length);
            out.write(tb.toString().getBytes(StandardCharsets.US_ASCII));
            out.write(trailerName);
            pad4(out, out.size());
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
        return out.toByteArray();
    }

    private static void pad4(ByteArrayOutputStream out, int size) {
        while (size % 4 != 0) {
            out.write(0);
            size++;
        }
    }

    /**
     * Assemble a boot image: 16 KiB header page + kernel page(s) +
     * ramdisk page(s). `v4` decides where ramdisk_size lives and what
     * page_size@36 decodes to.
     */
    private static byte[] buildBootImage(boolean v4, int kernelSize, byte[] ramdisk) throws Exception {
        int kernelPages = (kernelSize + PAGE - 1) / PAGE;
        int ramdiskOffset = PAGE + kernelPages * PAGE;
        int total = ramdiskOffset + ((ramdisk.length + PAGE - 1) / PAGE) * PAGE;

        ByteBuffer bb = ByteBuffer.allocate(16384).order(ByteOrder.LITTLE_ENDIAN);
        bb.put(0, "ANDROID!".getBytes(StandardCharsets.US_ASCII));
        bb.putInt(8, kernelSize);
        if (v4) {
            // v3/v4: ramdisk_size@12, os_version@16, header_size@20,
            // reserved@24..40, header_version@40, cmdline@44.
            // page_size@36 reads garbage (os_version/reserved territory)
            // — exactly what forces the v3/v4 branch in the importer.
            bb.putInt(12, ramdisk.length);
            bb.putInt(16, 0x20000018);              // os_version-shaped garbage
            bb.putInt(20, 1584);                    // header_size (v4)
            bb.putInt(40, 4);                       // header_version = 4
        } else {
            // v0/v1/v2: kernel_addr@12, ramdisk_size@16, ramdisk_addr@20,
            // page_size@36.
            bb.putInt(12, 0x10008000);              // kernel_addr
            bb.putInt(16, ramdisk.length);
            bb.putInt(20, 0x11000000);              // ramdisk_addr
            bb.putInt(36, PAGE);                    // page_size = 4096 (valid → v0/v1/v2 branch)
        }

        byte[] img = new byte[total];
        System.arraycopy(bb.array(), 0, img, 0, 16384);
        // Kernel bytes spread over its pages (content irrelevant).
        for (int i = 0; i < kernelSize; i++) img[PAGE + i] = (byte) i;
        System.arraycopy(ramdisk, 0, img, ramdiskOffset, ramdisk.length);
        return img;
    }

    private static byte[] gzip(byte[] data) throws Exception {
        ByteArrayOutputStream bos = new ByteArrayOutputStream();
        try (GZIPOutputStream gz = new GZIPOutputStream(bos)) {
            gz.write(data);
        }
        return bos.toByteArray();
    }

    @Test
    public void bootImageV4_readsRamdiskSizeAtOffset12_andExtracts() throws Exception {
        byte[] cpio = buildCpio("hello.txt", "v4-import-works".getBytes(StandardCharsets.UTF_8));
        byte[] ramdisk = gzip(cpio);
        byte[] img = buildBootImage(true, 8192, ramdisk);

        File image = tmp.newFile("sailfish.img");
        try (FileOutputStream fos = new FileOutputStream(image)) {
            fos.write(img);
        }
        File staging = tmp.newFolder("staging");

        boolean ok = RamdiskImporter.importBootImage(image, staging);
        assertTrue("importBootImage must succeed on a v4 image", ok);

        File extracted = new File(staging, "hello.txt");
        assertTrue("extracted file must exist", extracted.isFile());
        assertEquals("v4-import-works", new String(Files.readAllBytes(extracted.toPath()),
                StandardCharsets.UTF_8));
    }

    @Test
    public void bootImageV0_readsRamdiskSizeAtOffset16_andExtracts() throws Exception {
        byte[] cpio = buildCpio("v0file", "zero-config".getBytes(StandardCharsets.UTF_8));
        byte[] ramdisk = gzip(cpio);
        byte[] img = buildBootImage(false, 8192, ramdisk);

        File image = tmp.newFile("twrp.img");
        try (FileOutputStream fos = new FileOutputStream(image)) {
            fos.write(img);
        }
        File staging = tmp.newFolder("staging0");

        boolean ok = RamdiskImporter.importBootImage(image, staging);
        assertTrue("importBootImage must succeed on a v0 image", ok);

        File extracted = new File(staging, "v0file");
        assertTrue("extracted file must exist", extracted.isFile());
        assertEquals("zero-config", new String(Files.readAllBytes(extracted.toPath()),
                StandardCharsets.UTF_8));
    }

    @Test
    public void bootImageV4_rejectsRamdiskBeyondImageBounds() throws Exception {
        // Craft a v4 header claiming a ramdisk LARGER than the file — the
        // bounds check must fail loudly instead of short-reading silently.
        byte[] img = buildBootImage(true, 8192, gzip(buildCpio("x", new byte[8])));
        byte[] bogus = new byte[img.length + 4096];
        System.arraycopy(img, 0, bogus, 0, img.length);
        ByteBuffer.wrap(bogus).order(ByteOrder.LITTLE_ENDIAN).putInt(12, 0x7FFFFFF0);

        File image = tmp.newFile("bad.img");
        try (FileOutputStream fos = new FileOutputStream(image)) {
            fos.write(bogus);
        }
        File staging = tmp.newFolder("stagingbad");

        try {
            RamdiskImporter.importBootImage(image, staging);
            org.junit.Assert.fail("out-of-bounds ramdisk must throw");
        } catch (java.io.IOException expected) {
            assertNotNull(expected.getMessage());
            assertTrue(expected.getMessage().contains("out of bounds"));
        }
    }
}
