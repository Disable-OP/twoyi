/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.utils;

import android.annotation.SuppressLint;
import android.content.Context;
import android.content.SharedPreferences;
import android.util.Log;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

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
     * Validate a profile name to prevent path traversal.
     *
     * <p>Profile names are used directly as path segments under
     * {@code <dataDir>/profiles/<name>}, so a name containing "/" or ".."
     * would let a caller escape the profiles directory (e.g. "../../etc"
     * resolves to {@code <dataDir>/etc}). Reject any name that:
     * <ul>
     *   <li>is null, empty, or all-whitespace</li>
     *   <li>contains a path separator ('/' or File.separatorChar)</li>
     *   <li>equals "." or ".."</li>
     *   <li>contains a NUL byte (defence against C-string truncation in
     *       any downstream native code that receives the name)</li>
     * </ul>
     * The default profile name is always considered valid.
     */
    private static boolean isValidProfileName(String name) {
        if (name == null || name.trim().isEmpty()) {
            return false;
        }
        if (name.indexOf('/') >= 0 || name.indexOf(File.separatorChar) >= 0) {
            return false;
        }
        if (".".equals(name) || "..".equals(name)) {
            return false;
        }
        if (name.indexOf('\0') >= 0) {
            return false;
        }
        return true;
    }

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
        // Fixed: was only checking trim().isEmpty(), which let names like
        // "../../etc" or "a/b" escape the profiles directory via path
        // traversal. Now uses the shared validator.
        if (!isValidProfileName(profileName)) {
            Log.w(TAG, "Invalid profile name: " + profileName);
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
    @SuppressLint("ApplySharedPref")
    public static boolean renameProfile(Context context, String oldName, String newName) {
        if (DEFAULT_PROFILE.equals(oldName)) {
            Log.w(TAG, "Cannot rename default profile");
            return false;
        }

        // Fixed: validate newName against path traversal (same as createProfile).
        // oldName is also validated defensively, in case the caller tampered
        // with SharedPreferences.
        if (!isValidProfileName(newName) || !isValidProfileName(oldName)) {
            Log.w(TAG, "Invalid profile name: old=" + oldName + " new=" + newName);
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

        // Copy settings to new profile name and delete old.
        // Fixed: original code called .commit() once per key, which is a
        // synchronous disk write per SharedPreferences entry. Batch them
        // into a single edit() so we only fsync once.
        SharedPreferences oldPrefs = context.getSharedPreferences(
                "profile_settings_" + oldName, Context.MODE_PRIVATE);
        SharedPreferences newPrefs = context.getSharedPreferences(
                "profile_settings_" + newName, Context.MODE_PRIVATE);

        SharedPreferences.Editor newEditor = newPrefs.edit().clear();
        // Note: getAll() returns a snapshot; safe to iterate after we've
        // started the new editor.
        for (Map.Entry<String, ?> entry : oldPrefs.getAll().entrySet()) {
            String key = entry.getKey();
            Object value = entry.getValue();
            if (value instanceof Boolean) {
                newEditor.putBoolean(key, (Boolean) value);
            } else if (value instanceof String) {
                newEditor.putString(key, (String) value);
            } else if (value instanceof Integer) {
                newEditor.putInt(key, (Integer) value);
            }
        }
        newEditor.commit();
        oldPrefs.edit().clear().commit();

        // Update active profile if it was the renamed one
        if (oldName.equals(getActiveProfile(context))) {
            setActiveProfile(context, newName);
            updateRootfsSymlink(context);
        }

        return true;
    }

    /**
     * Copy a profile - copies files manually then creates tar
     */
    @SuppressLint("ApplySharedPref")
    public static boolean copyProfile(Context context, String sourceName, String targetName) {
        // Fixed: validate both names against path traversal (same as createProfile).
        if (!isValidProfileName(sourceName) || !isValidProfileName(targetName)) {
            Log.w(TAG, "Invalid profile name: source=" + sourceName + " target=" + targetName);
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

        try {
            // Create target directory
            targetDir.mkdirs();

            // Copy directory contents manually (preserving symlinks, skipping sockets)
            Log.d(TAG, "Copying profile from " + sourceDir + " to " + targetDir);
            copyDirectoryPreservingSymlinks(sourceDir, targetDir);

            // Copy settings
            SharedPreferences sourcePrefs = context.getSharedPreferences(
                    "profile_settings_" + sourceName, Context.MODE_PRIVATE);
            SharedPreferences targetPrefs = context.getSharedPreferences(
                    "profile_settings_" + targetName, Context.MODE_PRIVATE);

            // Fixed: batch into one edit() to avoid one fsync per key.
            SharedPreferences.Editor targetEditor = targetPrefs.edit().clear();
            for (Map.Entry<String, ?> entry : sourcePrefs.getAll().entrySet()) {
                String key = entry.getKey();
                Object value = entry.getValue();
                if (value instanceof Boolean) {
                    targetEditor.putBoolean(key, (Boolean) value);
                } else if (value instanceof String) {
                    targetEditor.putString(key, (String) value);
                } else if (value instanceof Integer) {
                    targetEditor.putInt(key, (Integer) value);
                }
            }
            targetEditor.commit();

            Log.d(TAG, "Profile copied successfully");
            return true;
        } catch (Exception e) {
            Log.e(TAG, "Failed to copy profile", e);
            // Clean up partial copy
            IOUtils.deleteDirectory(targetDir);
            return false;
        }
    }
    
    /**
     * Copy directory recursively while preserving symlinks and skipping sockets - soft fail on errors
     */
    private static void copyDirectoryPreservingSymlinks(File source, File target) throws IOException {
        File[] files = source.listFiles();
        if (files == null) {
            return;
        }
        
        for (File file : files) {
            File destFile = new File(target, file.getName());
            Path sourcePath = file.toPath();
            Path targetPath = destFile.toPath();
            
            try {
                // Skip socket files (file type 140000) - tar cannot archive them
                try {
                    java.nio.file.attribute.BasicFileAttributes attrs = 
                        Files.readAttributes(sourcePath, java.nio.file.attribute.BasicFileAttributes.class, 
                                           java.nio.file.LinkOption.NOFOLLOW_LINKS);
                    if (!attrs.isRegularFile() && !attrs.isDirectory() && !attrs.isSymbolicLink()) {
                        // Skip special files like sockets, named pipes, etc.
                        Log.d(TAG, "Skipping special file: " + file.getName());
                        continue;
                    }
                } catch (Exception e) {
                    Log.w(TAG, "Could not check file type for: " + file.getName() + ", skipping");
                    continue;
                }
                
                if (Files.isSymbolicLink(sourcePath)) {
                    // Preserve symlink
                    Path linkTarget = Files.readSymbolicLink(sourcePath);
                    Files.createSymbolicLink(targetPath, linkTarget);
                    Log.d(TAG, "Created symlink: " + destFile.getName() + " -> " + linkTarget);
                } else if (file.isDirectory()) {
                    // Recursively copy directory
                    destFile.mkdirs();
                    copyDirectoryPreservingSymlinks(file, destFile);
                } else {
                    // Copy regular file
                    Files.copy(sourcePath, targetPath, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.COPY_ATTRIBUTES);
                }
            } catch (java.nio.file.AccessDeniedException e) {
                // Soft fail on permission errors - log and continue
                Log.w(TAG, "Permission denied copying: " + file.getAbsolutePath() + ", skipping");
            } catch (Exception e) {
                // Soft fail on other errors - log and continue
                Log.w(TAG, "Error copying: " + file.getAbsolutePath() + " - " + e.getMessage() + ", skipping");
            }
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

        // Fixed: validate to prevent deleting arbitrary directories via path
        // traversal in the profile name.
        if (!isValidProfileName(profileName)) {
            Log.w(TAG, "Invalid profile name: " + profileName);
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
        // Fixed: validate to prevent path traversal via a tampered profile name.
        if (!isValidProfileName(profileName)) {
            Log.w(TAG, "Invalid profile name: " + profileName);
            return false;
        }
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

            // 6-Z268: skip-if-unchanged — the old delete-then-recreate ran
            // on EVERY process start (attachBaseContext), including a crash
            // window between the two calls that presented as
            // "No ROM Installed" (romExist() stats <dataDir>/rootfs/init)
            // and blocked boot on the user. When the symlink already
            // points at the active profile there is nothing to do.
            if (Files.isSymbolicLink(rootfsSymlink)
                    && profileRootfsDir.toPath().equals(Files.readSymbolicLink(rootfsSymlink))) {
                return true;
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
