# Copyright Disclaimer: AI-Generated Content
# This file was created by GitHub Copilot, an AI coding assistant.
# AI-generated content is not subject to copyright protection and is provided
# without any warranty, express or implied, including warranties of merchantability,
# fitness for a particular purpose, or non-infringement.
# Use at your own risk.

# Getting Started with New Features

This guide helps you get started with the new features added to Twoyi.

## What's New

1. **Settings as Default Screen** - App now opens to Settings first
2. **Profile Manager** - Manage multiple isolated Android environments
3. **Rootfs Import/Export** - Backup and restore your system
4. **Launch Container** - Start the Android container from Settings
5. **Automated Builds** - GitHub Actions workflow for CI/CD

## First Time Setup

### 1. Launch the App

When you first open the updated Twoyi:

1. The app opens to the Settings screen (new behavior)
2. Your existing rootfs is automatically migrated to the "default" profile
3. A symlink is created to maintain compatibility

### 2. Start the Container

To launch the Android container:

1. Open Settings (default screen)
2. Tap "Launch Container" at the top
3. The container boots as before
4. You can return to Settings by using the Recent Apps button

## Using Profile Manager

### Creating a New Profile

Perfect for testing different ROM configurations:

1. Settings → Advanced → Profile Manager
2. Scroll to bottom and tap "Create Profile"
3. Enter a name (e.g., "testing", "custom", "work")
4. Tap OK

The new profile starts empty. You'll need to import a rootfs.

### Switching Profiles

1. Settings → Advanced → Profile Manager
2. Tap the profile you want to switch to
3. Select "Switch Profile"
4. Confirm (app will reboot)

### Deleting a Profile

1. Settings → Advanced → Profile Manager
2. Tap the profile you want to delete
3. Select "Delete Profile"
4. Confirm

**Note:** Cannot delete the active profile or default profile.

## Importing Rootfs

You can import a rootfs from:
- Official .7z ROM files
- .tar backup files (created by Export)
- Third-party ROM files

### Steps:

1. Settings → Advanced → Import Rootfs
2. Select your .tar or .7z file
3. Wait for extraction (may take a few minutes)
4. Reboot when prompted

**Supported Sources:**
- Official ROM: https://github.com/cyanmint/twoyi/releases/download/original/rootfs.7z
- Your own exports
- Third-party ROMs (author name checking removed)

## Exporting Rootfs

Create a backup of your current system:

1. Settings → Advanced → Export Rootfs
2. Wait for compression
3. Share or save the .tar file
4. Import it later or on another device

**Use Cases:**
- Backup before experimenting
- Share your configuration
- Migrate to a new device

## Workflow Usage

### For Users

Pre-built APKs are available as workflow artifacts:

1. Go to repository → Actions tab
2. Find a successful workflow run
3. Download the "twoyi-apk" artifact
4. Extract and install the APK

### For Developers

To enable automated builds:

1. Add rootfs.7z to `app/src/main/assets/`
2. Push to main or develop branch
3. Workflow runs automatically
4. APK available in Artifacts section

See `.github/workflows/README.md` for details.

## Tips and Tricks

### Organized Profiles

Create profiles for different purposes:
- **default** - Your daily driver
- **testing** - For trying new ROMs
- **clean** - Fresh install for debugging
- **custom** - Your customized setup

### Safe Experimentation

Before trying something risky:
1. Export your current rootfs
2. Create a new profile
3. Import the rootfs into the new profile
4. Switch to the new profile
5. Experiment freely

If something breaks, just switch back!

### Quick Profile Switch

Profile switching requires a reboot, so:
- Plan your profile switches
- Save your work before switching
- Consider what each profile is for

### Container Management

The container process runs independently:
- Settings → Launch Container starts it
- It keeps running even when you're in Settings
- Use Recent Apps to switch between Settings and Container
- Shutdown/Reboot from Settings to stop it

## Troubleshooting

### Settings is blank after update
- The app likely failed to migrate
- Uninstall and reinstall (data loss)
- Or use adb to check logs

### Profile won't switch
- Ensure you selected a different profile (not the active one)
- Check that the profile exists
- Try rebooting manually

### Import failed
- Verify the file is a valid .tar or .7z
- Check available storage space
- Try a different file

### Export failed
- Check available storage in cache
- Ensure rootfs directory exists
- Verify tar command works on your device

### Container won't start after profile switch
- The new profile might be empty
- Import a rootfs into the new profile
- Or switch back to a working profile

## Advanced Usage

### Manual Profile Management

Profiles are stored at:
```
/data/data/io.twoyi/profiles/<profile_name>/rootfs/
```

You can manually manage them with root access:
```bash
# List profiles
ls /data/data/io.twoyi/profiles/

# Check active profile symlink
ls -la /data/data/io.twoyi/rootfs

# Manual profile creation (advanced)
mkdir -p /data/data/io.twoyi/profiles/myprofile/rootfs
```

### Build from Source

See main README.md and `.github/workflows/README.md` for build instructions.

## Getting Help

If you encounter issues:

1. Check the logs: Settings → Send Log
2. Read PROFILE_MANAGER.md for detailed documentation
3. See CHANGES.md for what changed
4. Report issues on GitHub
5. Join the Telegram group

## Migration Notes

### From Old Twoyi

Your existing installation will:
- Automatically migrate to the new profile system
- Keep all your data and settings
- Work exactly as before (just opens to Settings first)

### First Boot After Update

1. App opens to Settings (new!)
2. Migration happens in background
3. Tap "Launch Container" to start as usual
4. Everything else works the same

## What's Next

Future improvements could include:
- Profile export/import (with settings)
- Profile cloning
- Cloud backup
- Per-profile app installations
- Custom profile themes

Enjoy the new features!
