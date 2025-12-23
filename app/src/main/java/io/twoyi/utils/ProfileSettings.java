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
}
