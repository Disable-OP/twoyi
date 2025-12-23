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
     * Get display width for active profile (default: 1080)
     */
    public static int getDisplayWidth(Context context) {
        return getInt(context, DISPLAY_WIDTH, 1080);
    }

    /**
     * Set display width for active profile
     */
    public static void setDisplayWidth(Context context, int width) {
        setInt(context, DISPLAY_WIDTH, width);
    }

    /**
     * Get display height for active profile (default: 1920)
     */
    public static int getDisplayHeight(Context context) {
        return getInt(context, DISPLAY_HEIGHT, 1920);
    }

    /**
     * Set display height for active profile
     */
    public static void setDisplayHeight(Context context, int height) {
        setInt(context, DISPLAY_HEIGHT, height);
    }

    /**
     * Get display DPI for active profile (default: 160)
     */
    public static int getDisplayDpi(Context context) {
        return getInt(context, DISPLAY_DPI, 160);
    }

    /**
     * Set display DPI for active profile
     */
    public static void setDisplayDpi(Context context, int dpi) {
        setInt(context, DISPLAY_DPI, dpi);
    }
}
