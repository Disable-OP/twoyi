/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.utils;

import android.content.Context;
import android.util.Log;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;

/**
 * Manages different profiles with separate rootfs and settings.
 * The active profile's rootfs is symlinked to <appdir>/rootfs.
 */
public class ProfileManager {

    private static final String TAG = "ProfileManager";
    private static final String PROFILES_DIR = "profiles";
    private static final String DEFAULT_PROFILE = "default";
    private static final String ACTIVE_PROFILE_KEY = "active_profile";

    /**
     * Get the profiles directory
     */
    public static File getProfilesDir(Context context) {
        return new File(context.getDataDir(), PROFILES_DIR);
    }

    /**
     * Get a specific profile directory
     */
    public static File getProfileDir(Context context, String profileName) {
        return new File(getProfilesDir(context), profileName);
    }

    /**
     * Get the rootfs directory for a specific profile
     */
    public static File getProfileRootfsDir(Context context, String profileName) {
        return new File(getProfileDir(context, profileName), "rootfs");
    }

    /**
     * Get the active profile name
     */
    public static String getActiveProfile(Context context) {
        String profile = AppKV.getStringConfig(context, ACTIVE_PROFILE_KEY, null);
        if (profile == null || profile.isEmpty()) {
            profile = DEFAULT_PROFILE;
            setActiveProfile(context, profile);
        }
        return profile;
    }

    /**
     * Set the active profile
     */
    public static void setActiveProfile(Context context, String profileName) {
        AppKV.setStringConfig(context, ACTIVE_PROFILE_KEY, profileName);
    }

    /**
     * Get all available profiles
     */
    public static List<String> getProfiles(Context context) {
        List<String> profiles = new ArrayList<>();
        File profilesDir = getProfilesDir(context);
        
        if (!profilesDir.exists()) {
            profilesDir.mkdirs();
        }

        File[] files = profilesDir.listFiles();
        if (files != null) {
            for (File file : files) {
                if (file.isDirectory()) {
                    profiles.add(file.getName());
                }
            }
        }

        // Ensure default profile exists
        if (!profiles.contains(DEFAULT_PROFILE)) {
            profiles.add(DEFAULT_PROFILE);
        }

        return profiles;
    }

    /**
     * Create a new profile
     */
    public static boolean createProfile(Context context, String profileName) {
        if (profileName == null || profileName.trim().isEmpty()) {
            return false;
        }

        File profileDir = getProfileDir(context, profileName);
        if (profileDir.exists()) {
            Log.w(TAG, "Profile already exists: " + profileName);
            return false;
        }

        return profileDir.mkdirs();
    }

    /**
     * Delete a profile
     */
    public static boolean deleteProfile(Context context, String profileName) {
        if (DEFAULT_PROFILE.equals(profileName)) {
            Log.w(TAG, "Cannot delete default profile");
            return false;
        }

        if (profileName.equals(getActiveProfile(context))) {
            Log.w(TAG, "Cannot delete active profile");
            return false;
        }

        File profileDir = getProfileDir(context, profileName);
        if (!profileDir.exists()) {
            return false;
        }

        return IOUtils.deleteDirectory(profileDir);
    }

    /**
     * Switch to a different profile by updating the symlink
     */
    public static boolean switchProfile(Context context, String profileName) {
        if (!getProfiles(context).contains(profileName)) {
            Log.w(TAG, "Profile does not exist: " + profileName);
            return false;
        }

        setActiveProfile(context, profileName);
        return updateRootfsSymlink(context);
    }

    /**
     * Update the rootfs symlink to point to the active profile
     */
    public static boolean updateRootfsSymlink(Context context) {
        String activeProfile = getActiveProfile(context);
        File profileRootfsDir = getProfileRootfsDir(context, activeProfile);
        Path rootfsSymlink = new File(context.getDataDir(), "rootfs").toPath();

        try {
            // Ensure profile rootfs directory exists
            if (!profileRootfsDir.exists()) {
                profileRootfsDir.mkdirs();
            }

            // Remove existing symlink or directory
            Files.deleteIfExists(rootfsSymlink);

            // Create symlink
            Files.createSymbolicLink(rootfsSymlink, profileRootfsDir.toPath());
            Log.i(TAG, "Rootfs symlink updated to profile: " + activeProfile);
            return true;
        } catch (IOException e) {
            Log.e(TAG, "Failed to update rootfs symlink", e);
            return false;
        }
    }

    /**
     * Initialize profile system on first run
     */
    public static void initializeProfiles(Context context) {
        File profilesDir = getProfilesDir(context);
        if (!profilesDir.exists()) {
            profilesDir.mkdirs();
        }

        // Check if the old rootfs exists and needs to be migrated
        File oldRootfs = new File(context.getDataDir(), "rootfs");
        File defaultProfileRootfs = getProfileRootfsDir(context, DEFAULT_PROFILE);

        if (oldRootfs.exists() && !Files.isSymbolicLink(oldRootfs.toPath())) {
            // Migrate old rootfs to default profile
            try {
                defaultProfileRootfs.getParentFile().mkdirs();
                Files.move(oldRootfs.toPath(), defaultProfileRootfs.toPath());
                Log.i(TAG, "Migrated old rootfs to default profile");
            } catch (IOException e) {
                Log.e(TAG, "Failed to migrate old rootfs", e);
            }
        }

        // Ensure default profile exists
        if (!defaultProfileRootfs.getParentFile().exists()) {
            defaultProfileRootfs.getParentFile().mkdirs();
        }

        // Update symlink
        updateRootfsSymlink(context);
    }
}
