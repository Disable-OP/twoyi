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

import android.annotation.SuppressLint;
import android.content.Context;
import android.content.SharedPreferences;
import android.os.Build;
import android.util.DisplayMetrics;
import android.view.WindowManager;

/**
 * Profile-specific settings storage.
 * Each profile has its own settings file.
 */
public class ProfileSettings {

    private static final String PREF_PREFIX = "profile_settings_";
    
    // Setting keys
    public static final String VERBOSE_LOGGING = "verbose_logging";
    public static final String DISPLAY_WIDTH = "display_width";
    public static final String DISPLAY_HEIGHT = "display_height";
    public static final String DISPLAY_DPI = "display_dpi";
    public static final String DISPLAY_COLOR_DEPTH = "display_color_depth";
    public static final String USE_NEW_RENDERER = "use_new_renderer";
    public static final String DEBUG_RENDERER = "debug_renderer";
    /**
     * Boot to TWRP recovery instead of full Android. When true, the
     * next container launch will pass --boot-recovery to kr64, which
     * skips LD_PRELOAD (TWRP init is statically linked), the /apex
     * bind mount, the binderfs mount, and the SELinux permissive
     * watchdog — the TWRP init.rc handles all of those itself.
     *
     * Stored per-profile so the user can have one profile with a
     * full-Android rootfs and another with a TWRP rootfs, switching
     * between them via the Profile Manager.
     */
    public static final String BOOT_RECOVERY = "boot_recovery";

    /**
     * Get SharedPreferences for the active profile
     */
    private static SharedPreferences getProfilePrefs(Context context) {
        String activeProfile = ProfileManager.getActiveProfile(context);
        String prefName = PREF_PREFIX + activeProfile;
        return context.getSharedPreferences(prefName, Context.MODE_PRIVATE);
    }

    /**
     * Get SharedPreferences for a specific profile
     */
    private static SharedPreferences getProfilePrefs(Context context, String profileName) {
        String prefName = PREF_PREFIX + profileName;
        return context.getSharedPreferences(prefName, Context.MODE_PRIVATE);
    }

    /**
     * Get boolean setting for active profile
     */
    public static boolean getBoolean(Context context, String key, boolean defaultValue) {
        return getProfilePrefs(context).getBoolean(key, defaultValue);
    }

    /**
     * Set boolean setting for active profile
     */
    @SuppressLint("ApplySharedPref")
    public static void setBoolean(Context context, String key, boolean value) {
        getProfilePrefs(context).edit().putBoolean(key, value).commit();
    }

    /**
     * Get string setting for active profile
     */
    public static String getString(Context context, String key, String defaultValue) {
        return getProfilePrefs(context).getString(key, defaultValue);
    }

    /**
     * Set string setting for active profile
     */
    @SuppressLint("ApplySharedPref")
    public static void setString(Context context, String key, String value) {
        getProfilePrefs(context).edit().putString(key, value).commit();
    }

    /**
     * Get int setting for active profile
     */
    public static int getInt(Context context, String key, int defaultValue) {
        return getProfilePrefs(context).getInt(key, defaultValue);
    }

    /**
     * Set int setting for active profile
     */
    @SuppressLint("ApplySharedPref")
    public static void setInt(Context context, String key, int value) {
        getProfilePrefs(context).edit().putInt(key, value).commit();
    }

    /**
     * Delete all settings for a specific profile
     */
    @SuppressLint("ApplySharedPref")
    public static void deleteProfileSettings(Context context, String profileName) {
        SharedPreferences prefs = getProfilePrefs(context, profileName);
        prefs.edit().clear().commit();
    }

    /**
     * Check if verbose logging is enabled for active profile (default: true)
     */
    public static boolean isVerboseLoggingEnabled(Context context) {
        return getBoolean(context, VERBOSE_LOGGING, true);
    }

    /**
     * Set verbose logging for active profile
     */
    public static void setVerboseLogging(Context context, boolean enabled) {
        setBoolean(context, VERBOSE_LOGGING, enabled);
    }

    /**
     * Get the physical screen width in pixels. Uses DisplayMetrics.
     */
    private static int getScreenWidth(Context context) {
        // 6-Z174: on API 30+ use getMaximumWindowMetrics — getRealMetrics
        // reports the app's COMPAT-SCALED window for legacy targetSdk (28)
        // apps, which on the arm64 redroid E2E gave 320x640 while the
        // real panel was 720x1600 (run 33016909850: uiautomator bounds
        // 320x640 vs wm size 720x1600; fb0 created at 819200 instead of
        // 4608000). The TWRP container must run at the REAL native
        // resolution — MaximumWindowMetrics reports it regardless of the
        // app's compat window.
        try {
            WindowManager wm = (WindowManager) context.getSystemService(Context.WINDOW_SERVICE);
            if (wm != null) {
                if (android.os.Build.VERSION.SDK_INT >= 30) {
                    android.graphics.Rect b = wm.getMaximumWindowMetrics().getBounds();
                    if (b.width() > 0) return b.width();
                }
                DisplayMetrics dm = new DisplayMetrics();
                wm.getDefaultDisplay().getRealMetrics(dm);
                if (dm.widthPixels > 0) return dm.widthPixels;
            }
        } catch (Throwable ignored) {}
        return 1080; // fallback
    }

    /**
     * Get the physical screen height in pixels. Uses DisplayMetrics.
     */
    private static int getScreenHeight(Context context) {
        // 6-Z174: same real-display source as getScreenWidth above.
        try {
            WindowManager wm = (WindowManager) context.getSystemService(Context.WINDOW_SERVICE);
            if (wm != null) {
                if (android.os.Build.VERSION.SDK_INT >= 30) {
                    android.graphics.Rect b = wm.getMaximumWindowMetrics().getBounds();
                    if (b.height() > 0) return b.height();
                }
                DisplayMetrics dm = new DisplayMetrics();
                wm.getDefaultDisplay().getRealMetrics(dm);
                if (dm.heightPixels > 0) return dm.heightPixels;
            }
        } catch (Throwable ignored) {}
        return 1920; // fallback
    }

    /**
     * Get the physical screen DPI. Uses DisplayMetrics.
     */
    private static int getScreenDpi(Context context) {
        try {
            DisplayMetrics dm = context.getResources().getDisplayMetrics();
            if (dm.densityDpi > 0) return dm.densityDpi;
        } catch (Throwable ignored) {}
        return 160; // fallback
    }

    /**
     * Get the screen's color depth in bits per pixel.
     * Android doesn't expose a direct API for this, but we can infer it
     * from the display's pixel format. Most modern Android devices use
     * 32-bit RGBA8888. Some older or budget devices use 24-bit RGB888
     * or 16-bit RGB565.
     *
     * We default to 32 (RGBA8888) which matches the vast majority of
     * modern Android devices.
     */
    private static int getScreenColorDepth(Context context) {
        // Android's PixelFormat doesn't expose a direct BPP query, but
        // WINDOW_FORMAT_RGBA_8888 (5) is the standard for modern devices.
        // Default to 32 bpp (RGBA8888).
        return 32;
    }

    /**
     * Get display width for active profile.
     * Default: the physical screen width (auto-detected).
     * A stored value of 0 means "auto-detect" and returns the screen width.
     */
    public static int getDisplayWidth(Context context) {
        int val = getInt(context, DISPLAY_WIDTH, 0);
        return val > 0 ? val : getScreenWidth(context);
    }

    /**
     * Set display width for active profile
     */
    public static void setDisplayWidth(Context context, int width) {
        setInt(context, DISPLAY_WIDTH, width);
    }

    /**
     * Get display height for active profile.
     * Default: the physical screen height (auto-detected).
     * A stored value of 0 means "auto-detect" and returns the screen height.
     */
    public static int getDisplayHeight(Context context) {
        int val = getInt(context, DISPLAY_HEIGHT, 0);
        return val > 0 ? val : getScreenHeight(context);
    }

    /**
     * Set display height for active profile
     */
    public static void setDisplayHeight(Context context, int height) {
        setInt(context, DISPLAY_HEIGHT, height);
    }

    /**
     * Get display DPI for active profile.
     * Default: the physical screen DPI (auto-detected).
     * A stored value of 0 means "auto-detect" and returns the screen DPI.
     */
    public static int getDisplayDpi(Context context) {
        int val = getInt(context, DISPLAY_DPI, 0);
        return val > 0 ? val : getScreenDpi(context);
    }

    /**
     * Set display DPI for active profile
     */
    public static void setDisplayDpi(Context context, int dpi) {
        setInt(context, DISPLAY_DPI, dpi);
    }

    /**
     * Get display color depth in bits per pixel for active profile.
     * Default: the physical screen's color depth (auto-detected, usually 32).
     * Supported values: 32 (RGBA8888), 24 (RGB888), 16 (RGB565).
     * Used by the TWRP framebuffer renderer to determine the pixel format.
     */
    public static int getDisplayColorDepth(Context context) {
        return getInt(context, DISPLAY_COLOR_DEPTH, getScreenColorDepth(context));
    }

    /**
     * Set display color depth for active profile.
     * @param depth 32 (RGBA8888), 24 (RGB888), or 16 (RGB565)
     */
    public static void setDisplayColorDepth(Context context, int depth) {
        setInt(context, DISPLAY_COLOR_DEPTH, depth);
    }

    /**
     * Check if new renderer should be used for active profile.
     *
     * Default is false on arm64-v8a (where the legacy closed-source
     * libOpenglRender.so blob is available and is the more complete
     * implementation), but true on x86_64 (where the legacy blob is
     * not shipped and the renderer_bindings stubs would panic).
     *
     * The user can still override this via the settings UI.
     */
    public static boolean useNewRenderer(Context context) {
        boolean defaultVal = false;
        return getBoolean(context, USE_NEW_RENDERER, defaultVal);
    }

    /**
     * Set renderer type for active profile
     */
    public static void setUseNewRenderer(Context context, boolean useNew) {
        setBoolean(context, USE_NEW_RENDERER, useNew);
    }

    /**
     * Check if debug renderer mode should be enabled for active profile (default: false)
     */
    public static boolean isDebugRendererEnabled(Context context) {
        return getBoolean(context, DEBUG_RENDERER, false);
    }

    /**
     * Set debug renderer mode for active profile
     */
    public static void setDebugRenderer(Context context, boolean enabled) {
        setBoolean(context, DEBUG_RENDERER, enabled);
    }

    /**
     * Check if Boot Recovery (TWRP) mode should be enabled for the
     * active profile (default: false).
     *
     * When true, the next container launch passes --boot-recovery to
     * kr64, which boots a TWRP recovery image instead of full Android.
     * The user is responsible for installing a TWRP rootfs (e.g.
     * extracted from assets/twrp/twrp-*.img via scripts/extract-twrp-ramdisk.py)
     * into the profile's rootfs directory before enabling this.
     */
    public static boolean isBootRecoveryEnabled(Context context) {
        return getBoolean(context, BOOT_RECOVERY, false);
    }

    /**
     * Set Boot Recovery (TWRP) mode for the active profile.
     */
    public static void setBootRecovery(Context context, boolean enabled) {
        setBoolean(context, BOOT_RECOVERY, enabled);
    }
}
