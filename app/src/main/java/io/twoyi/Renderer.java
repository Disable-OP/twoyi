/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi;

import android.view.MotionEvent;
import android.view.Surface;

/**
 * @author weishu
 * @date 2021/10/20.
 */
public class Renderer {

    static {
        System.loadLibrary("twoyi");
    }

    public static native void init(Surface surface, String loader, int width, int height, float xdpi, float ydpi, int fps);

    public static native void resetWindow(Surface surface, int top, int left, int width, int height, int fbWidth, int fbHeight);

    public static native void removeWindow(Surface surface);

    public static native void handleTouch(MotionEvent event);

    public static native void sendKeycode(int keycode);

    /**
     * Set the app's data directory. Must be called before init() so that
     * all internal paths (rootfs, log, input sockets, opengles pipes)
     * resolve correctly. In a work profile the data dir is different
     * from the default /data/data/io.twoyi.
     * @param dataDir absolute path to the app's data directory
     */
    public static native void setDataDir(String dataDir);

    /**
     * Set whether the next container launch boots an AOSP-STYLE recovery
     * (OrangeFox R12, Lineage recovery-in-boot, modern AOSP recovery, …)
     * through the NORMAL loader path. When {@code true}:
     * <ul>
     *   <li>{@code --boot-recovery} is <b>NOT</b> passed to kr64 (the
     *       guest init is dynamic — {@code /init} → {@code /system/bin/init}
     *       — and the recovery binary lives at {@code /system/bin/recovery},
     *       exec'd with the LD_PRELOAD hook chain);</li>
     *   <li>the native core starts the fb0 → SurfaceView blit loop
     *       (exactly like TWRP mode) instead of the AOSP emugl GL
     *       renderer — the recovery's minui draws into the
     *       LD_PRELOAD-hooked {@code /dev/graphics/fb0} regular file and
     *       never speaks GL.</li>
     * </ul>
     * Without this flag the loader-path recovery rendered into fb0 with
     * nobody reading it: the app showed the black loading screen forever
     * while the guest UI was alive internally (OrangeFox-lavender run
     * 33317227548).
     *
     * Detected by {@link io.twoyi.utils.RomManager#isAospRecoveryLayout}.
     * Must be called before {@code init()}. The value is reset to
     * {@code false} on process restart.
     *
     * @param enabled {@code true} to present an AOSP-style loader-path
     *                recovery through the fb0 blit loop
     */
    public static native void setRecoveryLoader(boolean enabled);

    /**
     * Set whether the next container launch should boot a TWRP recovery
     * image instead of full Android. When {@code true}, the native
     * renderer core passes {@code --boot-recovery} to kr64, which:
     * <ul>
     *   <li>skips LD_PRELOAD (TWRP init is statically linked)</li>
     *   <li>skips the /apex bind mount (TWRP doesn't use APEX packages)</li>
     *   <li>skips the binderfs mount (TWRP doesn't use binder)</li>
     *   <li>skips the SELinux permissive watchdog (TWRP handles SELinux in init.rc)</li>
     *   <li>auto-sets init_path=/init (TWRP's init is at the root of the ramdisk)</li>
     * </ul>
     * Must be called before init(). The value is reset to {@code false}
     * on process restart.
     *
     * @param enabled {@code true} to boot TWRP, {@code false} for full Android
     */
    public static native void setBootRecovery(boolean enabled);
}
