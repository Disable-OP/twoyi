# Copyright Disclaimer: AI-Generated Content
# This file was created by GitHub Copilot, an AI coding assistant.
# AI-generated content is not subject to copyright protection and is provided
# without any warranty, express or implied, including warranties of merchantability,
# fitness for a particular purpose, or non-infringement.
# Use at your own risk.

# Profile Manager Feature

## Overview

The Profile Manager allows you to maintain multiple isolated Android environments (profiles) within Twoyi. Each profile has its own rootfs and settings, enabling you to:

- Test different ROM configurations
- Keep separate environments for different purposes
- Switch between profiles easily

## How It Works

### Directory Structure

Profiles are stored in `<app_data_dir>/profiles/`:
```
/data/data/io.twoyi/
├── profiles/
│   ├── default/
│   │   └── rootfs/
│   ├── testing/
│   │   └── rootfs/
│   └── custom/
│       └── rootfs/
└── rootfs -> profiles/<active_profile>/rootfs (symlink)
```

### Active Profile

- The active profile's rootfs is symlinked to `<app_data_dir>/rootfs`
- When you switch profiles, the symlink is updated to point to the new profile
- The container uses the symlinked rootfs directory

## Using Profile Manager

### Accessing Profile Manager

1. Open Twoyi (now opens to Settings by default)
2. Tap "Profile Manager" in the Advanced section

### Creating a Profile

1. Open Profile Manager
2. Select "Create Profile"
3. Enter a profile name
4. The new profile will be created with an empty rootfs

### Switching Profiles

1. Open Profile Manager
2. Select the profile you want to switch to
3. Choose "Switch Profile"
4. Confirm the switch (app will reboot)

### Deleting a Profile

1. Open Profile Manager
2. Select the profile you want to delete
3. Choose "Delete Profile"
4. Confirm deletion

**Note:** You cannot delete the default profile or the currently active profile.

## Importing and Exporting Rootfs

### Import Rootfs

You can import a rootfs from a .tar or .7z file:

1. Go to Settings > Advanced > Import Rootfs
2. Select a .tar or .7z file containing the rootfs
3. The rootfs will be extracted to the active profile
4. Reboot to use the new rootfs

**Supported formats:**
- `.tar` files (created by Export Rootfs)
- `.7z` files (official ROM format)

### Export Rootfs

To backup or share your current rootfs:

1. Go to Settings > Advanced > Export Rootfs
2. The current rootfs will be packaged as a .tar file
3. You can share or save the file for later use

## Migration from Old Versions

When you first run the updated app:

1. Your existing rootfs will be automatically migrated to the "default" profile
2. The old rootfs directory will be replaced with a symlink
3. Your data and settings are preserved

## Launch Container

The "Launch Container" option in Settings allows you to start the Android container while keeping the Settings activity open. This is useful for:

- Quickly returning to Settings
- Managing the container without restarting the app
- Accessing both Settings and Container simultaneously

## Technical Details

### ProfileManager API

```java
// Get all profiles
List<String> profiles = ProfileManager.getProfiles(context);

// Create a new profile
ProfileManager.createProfile(context, "myprofile");

// Switch to a profile
ProfileManager.switchProfile(context, "myprofile");

// Delete a profile
ProfileManager.deleteProfile(context, "myprofile");

// Get active profile
String active = ProfileManager.getActiveProfile(context);
```

### Storage Location

- Profiles directory: `<app_data_dir>/profiles/`
- Active profile setting: SharedPreferences key "active_profile"
- Rootfs symlink: `<app_data_dir>/rootfs`

## Troubleshooting

**Profile won't switch:**
- Ensure the profile exists
- Check that you're not trying to switch to the current profile
- Verify app has permission to create symlinks

**Import failed:**
- Verify the file is a valid .tar or .7z archive
- Check available storage space
- Ensure the archive contains a valid rootfs structure

**Export failed:**
- Check available storage space in cache directory
- Verify the rootfs exists and is accessible
- Ensure tar command is available on the system
