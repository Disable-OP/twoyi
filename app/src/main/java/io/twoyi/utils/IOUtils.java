/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.utils;

import android.text.TextUtils;
import android.util.Log;

import androidx.annotation.Keep;

import java.io.BufferedReader;
import java.io.Closeable;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.FileReader;
import java.io.FileWriter;
import java.io.IOException;
import java.lang.reflect.Method;
import java.nio.channels.FileChannel;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Comparator;
import java.util.List;

/**
 * @author weishu
 * @date 2018/8/28.
 */
@Keep
public class IOUtils {
    private static final String TAG = "IOUtils";

    public static void ensureCreated(File file) {
        if (!file.exists()) {
            boolean ret = file.mkdirs();
            if (!ret) {
                throw new RuntimeException("create dir: " + file + " failed");
            }
        }
    }

    public static boolean deleteDir(File dir) {
        if (dir == null) {
            return false;
        }
        boolean success = true;
        if (dir.isDirectory()) {
            String[] children = dir.list();
            // Fixed: dir.list() can return null (TOCTOU or I/O error)
            if (children == null) {
                return dir.delete();
            }
            for (String file : children) {
                boolean ret = deleteDir(new File(dir, file));
                if (!ret) {
                    success = false;
                }
            }
            if (success) {
                // if all subdirectory are deleted, delete the dir itself.
                return dir.delete();
            }
        }
        return dir.delete();
    }

    public static void deleteAll(List<File> files) {
        if (files.isEmpty()) {
            return;
        }

        for (File file : files) {
            //noinspection ResultOfMethodCallIgnored
            file.delete();
        }
    }

    public static void copyFile(File source, File target) throws IOException {
        FileInputStream inputStream = null;
        FileOutputStream outputStream = null;
        try {
            inputStream = new FileInputStream(source);
            outputStream = new FileOutputStream(target);
            FileChannel iChannel = inputStream.getChannel();
            FileChannel oChannel = outputStream.getChannel();

            // Performance: use FileChannel.transferTo(), which on Linux maps
            // to the sendfile(2) syscall and copies data entirely inside the
            // kernel (zero-copy) instead of bouncing every chunk through a
            // user-space ByteBuffer. This is dramatically faster for the
            // large rootfs / system image files twoyi ships (>100 MB) and
            // also sidesteps the FileChannel.write(ByteBuffer) partial-write
            // hazard that the previous manual loop had to work around.
            //
            // transferTo() is permitted to return fewer bytes than requested
            // (its contract mirrors sendfile(2), which may return 0 on some
            // kernels for very large counts). Loop until the source is
            // exhausted; bail if no forward progress is possible to avoid
            // an infinite spin.
            long size = iChannel.size();
            long transferred = 0;
            while (transferred < size) {
                long n = iChannel.transferTo(transferred, size - transferred, oChannel);
                if (n <= 0) {
                    break;
                }
                transferred += n;
            }
            // 6-Z184 AUDIT FIX (agent 80): a short transferTo loop (n<=0
            // break) used to return "success" with a TRUNCATED target —
            // profile copies then booted from half-written files. Fail
            // loudly instead.
            if (transferred < size) {
                throw new IOException("copyFile: short copy " + transferred + "/" + size
                        + " bytes: " + source + " -> " + target);
            }
        } finally {
            closeSilently(inputStream);
            closeSilently(outputStream);
        }
    }

    public static void closeSilently(Closeable closeable) {
        if (closeable == null) {
            return;
        }
        try {
            closeable.close();
        } catch (IOException ignored) {
            // Intentionally silent: this is a best-effort close during
            // cleanup; callers don't want to see close-time I/O errors
            // masking the primary exception (if any).
        }
    }

    public static void setPermissions(String path, int mode, int uid, int gid) {
        try {
            Class<?> fileUtilsClass = Class.forName("android.os.FileUtils");
            Method setPermissions = fileUtilsClass.getDeclaredMethod("setPermissions", String.class, int.class, int.class, int.class);
            setPermissions.setAccessible(true);
            setPermissions.invoke(null, path, mode, uid, gid);
        } catch (Throwable e) {
            Log.e(TAG, "IOUtils failure", e);
        }
    }

    public static void writeContent(File file, String content) {
        if (file == null || TextUtils.isEmpty(content)) {
            return;
        }
        FileWriter fileWriter = null;
        try {
            fileWriter = new FileWriter(file);
            fileWriter.write(content);
            fileWriter.flush();
        } catch (Throwable ignored) {
        } finally {
            IOUtils.closeSilently(fileWriter);
        }
    }

    public static String readContent(File file) {
        if (file == null) {
            return null;
        }
        BufferedReader fileReader = null;
        try {
            fileReader = new BufferedReader(new FileReader(file));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = fileReader.readLine()) != null) {
                sb.append(line);
                sb.append('\n');
            }
            return sb.toString().trim();
        } catch (Throwable ignored) {
            return null;
        } finally {
            IOUtils.closeSilently(fileReader);
        }
    }

    public static boolean deleteDirectory(File directory) {
        // Fixed: callers (e.g. RomManager.removePartition via IOUtils.deleteDirectory,
        // and Render2Activity.importRomAndStart) can pass null when a profile
        // rootfs dir doesn't exist yet. The original code NPE'd on
        // `directory.toPath()`.
        if (directory == null) {
            return false;
        }
        // 6-Z186: tar-extracted TWRP ramdisks ship mode-0555 directories
        // (read+execute for all, NO write). Deleting a child entry needs
        // WRITE on the PARENT directory, so a plain delete walk left every
        // file inside those dirs behind — the caller then hit
        // renameTo() == ENOTEMPTY ("staging rename failed"), with the old
        // rootfs already gutted (its `init` deleted) leaving the app in
        // "No ROM Installed" limbo where EVERY later import also failed.
        // Pass 1 grants owner-write on every directory first (we own all
        // of them — the app's own uid extracted the tree), pass 2 deletes.
        try (java.util.stream.Stream<Path> prep = Files.walk(directory.toPath())) {
            prep.filter(Files::isDirectory)
                    .forEach(p -> {
                        File d = p.toFile();
                        if (!d.canWrite()) {
                            d.setWritable(true, true);
                        }
                    });
        } catch (IOException ignored) {
            // Pass-2 will surface a real failure; an unreadable tree here
            // just means the delete below will fail and report false.
        }
        // Fixed: Files.walk() returns a Stream that holds open directory
        // descriptors. Must use try-with-resources to close them, otherwise
        // FDs leak and can exhaust the per-process FD limit.
        try (java.util.stream.Stream<Path> walk = Files.walk(directory.toPath())) {
            // 6-Z184: track per-file failures — the old version returned
            // true whenever the WALK succeeded, so profile deletion /
            // factory-reset reported success with files left behind
            // (ghost profiles).
            java.util.List<File> failures = new java.util.ArrayList<>();
            walk.sorted(Comparator.reverseOrder())
                    .map(Path::toFile)
                    .forEach(f -> { if (!f.delete() && f.exists()) failures.add(f); });
            if (!failures.isEmpty()) {
                return false;
            }
            return !directory.exists();
        } catch (IOException e) {
            return false;
        }
    }
}
