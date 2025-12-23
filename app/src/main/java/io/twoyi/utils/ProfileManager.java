/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.utils;

import android.content.Context;
import android.content.SharedPreferences;
import android.util.Log;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
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
     * Rename a profile
     */
    public static boolean renameProfile(Context context, String oldName, String newName) {
        if (DEFAULT_PROFILE.equals(oldName)) {
            Log.w(TAG, "Cannot rename default profile");
            return false;
        }

        if (newName == null || newName.trim().isEmpty()) {
            Log.w(TAG, "New profile name is empty");
            return false;
        }

        File oldProfileDir = getProfileDir(context, oldName);
        if (!oldProfileDir.exists()) {
            Log.w(TAG, "Old profile does not exist: " + oldName);
            return false;
        }

        File newProfileDir = getProfileDir(context, newName);
        if (newProfileDir.exists()) {
            Log.w(TAG, "Profile already exists: " + newName);
            return false;
        }

        // Rename the directory
        if (!oldProfileDir.renameTo(newProfileDir)) {
            Log.e(TAG, "Failed to rename profile directory");
            return false;
        }

        // Copy settings to new profile name and delete old
        SharedPreferences oldPrefs = context.getSharedPreferences(
                "profile_settings_" + oldName, Context.MODE_PRIVATE);
        SharedPreferences newPrefs = context.getSharedPreferences(
                "profile_settings_" + newName, Context.MODE_PRIVATE);
        
        newPrefs.edit().clear().commit();
        for (String key : oldPrefs.getAll().keySet()) {
            Object value = oldPrefs.getAll().get(key);
            if (value instanceof Boolean) {
                newPrefs.edit().putBoolean(key, (Boolean) value).commit();
            } else if (value instanceof String) {
                newPrefs.edit().putString(key, (String) value).commit();
            } else if (value instanceof Integer) {
                newPrefs.edit().putInt(key, (Integer) value).commit();
            }
        }
        oldPrefs.edit().clear().commit();

        // Update active profile if it was the renamed one
        if (oldName.equals(getActiveProfile(context))) {
            setActiveProfile(context, newName);
            updateRootfsSymlink(context);
        }

        return true;
    }

    /**
     * Copy a profile using tar for reliable symlink preservation
     */
    public static boolean copyProfile(Context context, String sourceName, String targetName) {
        if (targetName == null || targetName.trim().isEmpty()) {
            Log.w(TAG, "Target profile name is empty");
            return false;
        }

        File sourceDir = getProfileDir(context, sourceName);
        if (!sourceDir.exists()) {
            Log.w(TAG, "Source profile does not exist: " + sourceName);
            return false;
        }

        File targetDir = getProfileDir(context, targetName);
        if (targetDir.exists()) {
            Log.w(TAG, "Target profile already exists: " + targetName);
            return false;
        }

        File tempTarFile = new File(context.getCacheDir(), "profile_copy_temp.tar");
        
        try {
            // Create tar from source
            String sourceDirPath = sourceDir.getAbsolutePath();
            String tempTarPath = tempTarFile.getAbsolutePath();
            String sourceParentPath = sourceDir.getParent();
            
            if (sourceDirPath.contains(";") || sourceDirPath.contains("&") ||
                tempTarPath.contains(";") || tempTarPath.contains("&")) {
                throw new SecurityException("Invalid path detected");
            }
            
            // Pack the source profile into tar
            ProcessBuilder pb1 = new ProcessBuilder(
                "tar", "-cf", tempTarPath,
                "-C", sourceParentPath, sourceDir.getName()
            );
            Process process1 = pb1.start();
            int exitCode1 = process1.waitFor();
            
            if (exitCode1 != 0) {
                throw new IOException("tar create failed with exit code: " + exitCode1);
            }
            
            // Create target directory
            File profilesDir = getProfilesDir(context);
            String profilesDirPath = profilesDir.getAbsolutePath();
            
            if (profilesDirPath.contains(";") || profilesDirPath.contains("&")) {
                throw new SecurityException("Invalid path detected");
            }
            
            // Extract tar to target
            ProcessBuilder pb2 = new ProcessBuilder(
                "tar", "-xf", tempTarPath,
                "-C", profilesDirPath
            );
            Process process2 = pb2.start();
            int exitCode2 = process2.waitFor();
            
            if (exitCode2 != 0) {
                throw new IOException("tar extract failed with exit code: " + exitCode2);
            }
            
            // Rename extracted directory to target name if needed
            File extractedDir = new File(profilesDir, sourceDir.getName());
            if (!extractedDir.equals(targetDir)) {
                if (!extractedDir.renameTo(targetDir)) {
                    throw new IOException("Failed to rename extracted profile");
                }
            }

            // Copy settings
            SharedPreferences sourcePrefs = context.getSharedPreferences(
                    "profile_settings_" + sourceName, Context.MODE_PRIVATE);
            SharedPreferences targetPrefs = context.getSharedPreferences(
                    "profile_settings_" + targetName, Context.MODE_PRIVATE);
            
            targetPrefs.edit().clear().commit();
            for (String key : sourcePrefs.getAll().keySet()) {
                Object value = sourcePrefs.getAll().get(key);
                if (value instanceof Boolean) {
                    targetPrefs.edit().putBoolean(key, (Boolean) value).commit();
                } else if (value instanceof String) {
                    targetPrefs.edit().putString(key, (String) value).commit();
                } else if (value instanceof Integer) {
                    targetPrefs.edit().putInt(key, (Integer) value).commit();
                }
            }

            return true;
        } catch (Exception e) {
            Log.e(TAG, "Failed to copy profile", e);
            // Clean up partial copy
            IOUtils.deleteDirectory(targetDir);
            return false;
        } finally {
            // Clean up temp tar file
            tempTarFile.delete();
        }
    }

    /**
     * Helper to copy a directory recursively, preserving symlinks
     */
    private static void copyDirectory(File source, File target) throws IOException {
        if (!target.exists()) {
            target.mkdirs();
        }

        File[] files = source.listFiles();
        if (files != null) {
            for (File file : files) {
                File targetFile = new File(target, file.getName());
                Path sourcePath = file.toPath();
                Path targetPath = targetFile.toPath();
                
                // Handle symlinks specially
                if (Files.isSymbolicLink(sourcePath)) {
                    // Read the symlink target and create a new symlink
                    Path linkTarget = Files.readSymbolicLink(sourcePath);
                    Files.createSymbolicLink(targetPath, linkTarget);
                } else if (file.isDirectory()) {
                    copyDirectory(file, targetFile);
                } else {
                    Files.copy(sourcePath, targetPath, StandardCopyOption.REPLACE_EXISTING);
                }
            }
        }
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

        // Delete profile settings
        ProfileSettings.deleteProfileSettings(context, profileName);

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

        try {
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
        } catch (Exception e) {
            Log.e(TAG, "Error checking rootfs symlink status", e);
        }

        // Ensure default profile exists
        if (!defaultProfileRootfs.getParentFile().exists()) {
            defaultProfileRootfs.getParentFile().mkdirs();
        }

        // Update symlink
        updateRootfsSymlink(context);
    }
}
