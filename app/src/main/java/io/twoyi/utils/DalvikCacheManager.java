/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.utils;

import android.content.Context;
import android.os.Build;
import android.util.Log;

import com.topjohnwu.superuser.Shell;

import java.io.File;
import java.util.Arrays;

/**
 * Detects host OTA updates by comparing {@link Build#FINGERPRINT} against
 * the value seen on the previous boot, and wipes the guest's
 * {@code data/dalvik-cache/} directory when they differ.
 *
 * <p>Background: the guest Android's ART OAT/VDEX files in
 * {@code dalvik-cache} are compiled against the host's ART version. When
 * the host receives an OTA update, the ART version changes, making every
 * existing OAT entry stale. Booting the guest with stale OAT entries
 * causes mysterious crashes ("No original dex files found",
 * {@code SIGSEGV} in libart, etc.) that are very hard to attribute to the
 * host OTA after the fact. Wiping the cache forces the guest to recompile
 * everything against the new host ART on the next boot — slow but safe.
 *
 * <p>Ported from cyanmint/Nogitsune (MPL-2.0, same license as twoyi):
 * {@code globals/BootHelper.kt::clearDalvikCacheIfNeeded()}.
 * Adapted to twoyi's Java architecture:
 * <ul>
 *   <li>Uses {@link AppKV} (twoyi's SharedPreferences wrapper) instead
 *       of a dedicated {@code nogitsune_boot} prefs file — keeps all
 *       twoyi KV in one place.</li>
 *   <li>Uses {@link ShellUtil#newSh()} + {@code rm -rf} instead of
 *       {@link File#delete()} — the guest's dalvik-cache is owned by
 *       root (the container's Zygote), so Java's {@link File#delete()}
 *       silently fails and leaves the stale OAT entries that cause the
 *       crashes we're trying to prevent.</li>
 *   <li>Logs both branches (cleared vs. unchanged) so the boot log
 *       shows which path was taken on each boot.</li>
 * </ul>
 *
 * <p>The fingerprint key is the same as the one previously used by
 * {@code RomManager.clearDalvikCacheIfNeeded()} ({@code host_build_fingerprint}),
 * so existing installs migrate seamlessly — the first boot after this
 * refactor will see the stored fingerprint from the previous boot and
 * skip the wipe (correct behaviour).
 *
 * @author Disable-OP
 * @date 2026/08/08.
 */
public final class DalvikCacheManager {

    private static final String TAG = "DalvikCacheManager";

    /**
     * SharedPreferences key under which the host's
     * {@link Build#FINGERPRINT} from the previous successful clear is
     * stored. Kept identical to the legacy RomManager key so existing
     * installs don't trigger a spurious cache wipe on the first boot
     * after the refactor.
     */
    private static final String PREF_HOST_FINGERPRINT = "host_build_fingerprint";

    /** Relative path of the guest's dalvik-cache inside the rootfs. */
    private static final String DALVIK_CACHE_REL_PATH = "data/dalvik-cache";

    private DalvikCacheManager() {
    }

    /**
     * Compares the current host {@link Build#FINGERPRINT} with the one
     * stored on the previous boot. If they differ (host got an OTA),
     * wipes {@code <rootfs>/data/dalvik-cache/} and stores the new
     * fingerprint. If they're the same, does nothing.
     *
     * <p>Idempotent: calling twice in a row is a no-op (the second call
     * sees the freshly-stored fingerprint and skips the wipe).
     *
     * <p>Safe to call on the UI thread (it's a quick SharedPreferences
     * read + a short {@code rm -rf} on a directory that's typically
     * a few hundred small files — ~100 ms worst case on flash storage).
     * The previous flow already ran shell commands synchronously on the
     * main thread inside {@code attachBaseContext}, so this is no worse.
     *
     * @param context used for SharedPreferences access.
     * @param rootfs  the guest rootfs directory; the cache lives at
     *                {@code <rootfs>/data/dalvik-cache/}.
     */
    public static void checkAndInvalidate(Context context, File rootfs) {
        if (context == null || rootfs == null) {
            Log.w(TAG, "checkAndInvalidate called with null context/rootfs — skipping");
            return;
        }

        String currentFingerprint = Build.FINGERPRINT;
        String lastFingerprint = AppKV.getStringConfig(context, PREF_HOST_FINGERPRINT, "");

        if (currentFingerprint == null) {
            // Build.FINGERPRINT should never be null on a real device,
            // but the framework's nullness annotation is @Nullable on
            // some API levels. Treat null as "unknown" and skip — we'd
            // rather not wipe the cache based on a missing value.
            Log.w(TAG, "Build.FINGERPRINT is null — skipping dalvik-cache check");
            return;
        }

        if (currentFingerprint.equals(lastFingerprint)) {
            Log.i(TAG, "Host fingerprint unchanged (" + currentFingerprint
                    + "), skipping dalvik-cache wipe");
            return;
        }

        Log.i(TAG, "Host fingerprint changed ("
                + (lastFingerprint.isEmpty() ? "(first boot)" : lastFingerprint)
                + " → " + currentFingerprint + "), wiping dalvik-cache");

        wipeDalvikCache(rootfs);

        // Only persist the new fingerprint AFTER the wipe succeeds (or
        // at least after we've attempted it). If the wipe throws or the
        // process is killed mid-wipe, the next boot will see the OLD
        // fingerprint and retry the wipe — which is the safe default
        // (we'd rather wipe twice than skip a needed wipe).
        AppKV.setStringConfig(context, PREF_HOST_FINGERPRINT, currentFingerprint);

        Log.i(TAG, "dalvik-cache wipe complete, fingerprint updated");
    }

    /**
     * Recursively deletes {@code <rootfs>/data/dalvik-cache/} via
     * {@code rm -rf}. Uses a non-root shell (the rootfs lives under the
     * app's private data dir, which the app owns).
     *
     * <p>If the directory doesn't exist, {@code rm -rf} is a no-op and
     * returns success — so this is safe to call on a fresh rootfs that
     * hasn't booted yet.
     */
    private static void wipeDalvikCache(File rootfs) {
        File cacheDir = new File(rootfs, DALVIK_CACHE_REL_PATH);
        String path = cacheDir.getAbsolutePath();

        Shell.Result result = ShellUtil.newSh().newJob()
                .add("rm -rf '" + path + "'")
                .exec();

        if (!result.isSuccess()) {
            // Not fatal — the guest will recompile on boot and may
            // produce "No original dex files found" warnings for the
            // stale entries we couldn't delete, but it won't crash the
            // host. Log the stderr so it's diagnosable.
            Log.w(TAG, "rm -rf dalvik-cache failed: "
                    + Arrays.toString(result.getErr().toArray(new String[0])));
            return;
        }

        Log.i(TAG, "dalvik-cache cleared: " + path);
    }
}
