# Copyright Disclaimer: AI-Generated Content
# This file was created by GitHub Copilot, an AI coding assistant.
# AI-generated content is not subject to copyright protection and is provided
# without any warranty, express or implied, including warranties of merchantability,
# fitness for a particular purpose, or non-infringement.
# Use at your own risk.

# Summary of Changes

This document summarizes all the changes made to implement the requested features.

## 1. GitHub Actions Workflow

**File:** `.github/workflows/build.yml`

Created a CI/CD workflow that:
- Builds the APK on push to main/develop branches
- Builds on pull requests
- Supports manual triggering via workflow_dispatch
- Caches cargo-xdk to speed up builds
- Uses Android NDK r22b as required
- Uploads built APKs as artifacts (30-day retention)

**Documentation:** `.github/workflows/README.md`

## 2. Settings Activity as Default Launcher

**Changed:** `app/src/main/AndroidManifest.xml`

- Made `SettingsActivity` the LAUNCHER activity
- Removed LAUNCHER intent filter from `Render2Activity`
- Added `launchMode="singleTask"` to `Render2Activity` to keep it running

**Benefits:**
- Users land on settings first
- Can configure before launching container
- Better user experience for managing profiles and settings

## 3. Launch Container Option

**Changed:** 
- `app/src/main/res/xml/pref_settings.xml`
- `app/src/main/res/values/strings.xml`
- `app/src/main/java/io/twoyi/ui/SettingsActivity.java`

Added a new "Launch Container" preference that:
- Starts the Render2Activity (container) while keeping settings open
- Uses `singleTask` launch mode to maintain container state
- Allows users to easily return to settings

## 4. Profile Manager

**New Files:**
- `app/src/main/java/io/twoyi/utils/ProfileManager.java`
- `PROFILE_MANAGER.md` (documentation)

**Changed:**
- `app/src/main/java/io/twoyi/TwoyiApplication.java`
- `app/src/main/java/io/twoyi/utils/AppKV.java`
- `app/src/main/java/io/twoyi/ui/SettingsActivity.java`
- `app/src/main/res/values/strings.xml`
- `app/src/main/res/xml/pref_settings.xml`

**Features:**
- Create, switch, and delete profiles
- Each profile has its own rootfs directory
- Active profile's rootfs is symlinked to `<appdir>/rootfs`
- Automatic migration of existing rootfs to "default" profile
- Profile management UI in settings

**Directory Structure:**
```
/data/data/io.twoyi/
├── profiles/
│   ├── default/rootfs/
│   ├── testing/rootfs/
│   └── custom/rootfs/
└── rootfs -> profiles/<active>/rootfs (symlink)
```

## 5. Rootfs Import/Export

**Changed:**
- `app/src/main/java/io/twoyi/ui/SettingsActivity.java`
- `app/src/main/res/values/strings.xml`
- `app/src/main/res/xml/pref_settings.xml`

**Import Features:**
- Import from .tar files (created by export)
- Import from .7z files (official ROM format)
- Automatic format detection based on file extension
- Progress dialog during import
- Replaces current profile's rootfs

**Export Features:**
- Export current rootfs to .tar file
- Uses system tar command
- Creates shareable archive
- Can be imported later or on another device

## 6. Lifted ROM Name Checking

**Changed:** `app/src/main/java/io/twoyi/ui/SettingsActivity.java`

**What Changed:**
- Removed validation that rejected ROMs with "weishu" or "twoyi" as author
- Now allows importing official initial rootfs if not bundled in APK
- Users can import any valid rootfs regardless of author name

**Original Code (removed):**
```java
if ("weishu".equalsIgnoreCase(author) || "twoyi".equalsIgnoreCase(author)) {
    Toast.makeText(activity, R.string.replace_rom_unofficial_tips, Toast.LENGTH_SHORT).show();
    rootfs3rd.delete();
    return;
}
```

## 7. Build Configuration Updates

**Changed:** `app/build.gradle`, `build.gradle`

- Updated dependency resolution to use mavenCentral (JCenter deprecated)
- Replaced `com.hzy:libp7zip:1.7.0` with `com.github.hzy3774:AndroidP7zip:v1.7.1` (jitpack)
- Reverted `compileSdkVersion` to 31 (from 35) for compatibility with build tools 30.0.3
- Maintains `targetSdkVersion` at 27 as required

**Why:** JCenter repository was shut down in 2022, making the original libp7zip dependency unavailable. The jitpack alternative provides the same functionality.

## 8. Other Changes

**Changed:** `.gitignore`

Added exclusions for:
- `app/src/main/assets/rootfs.7z` (placeholder)
- `app/src/main/assets/rom.ini` (placeholder)

These are placeholders for development. Actual rootfs must be provided by users.

## Code Organization

### New Classes
1. **ProfileManager** - Manages multiple rootfs profiles
   - Profile creation, deletion, switching
   - Symlink management
   - Migration from old format

### Modified Classes
1. **TwoyiApplication** - Added profile initialization
2. **SettingsActivity** - Added new preferences and handlers
3. **AppKV** - Added string storage methods

### New UI Elements
- Profile Manager preference
- Launch Container preference
- Import Rootfs preference
- Export Rootfs preference
- Profile management dialog
- Create/Switch/Delete profile dialogs

## Testing Considerations

### Local Testing
Due to network issues in the sandbox, full Gradle build couldn't be tested locally. However:
- YAML workflow syntax validated with yamllint ✓
- Java code structure verified ✓
- Resource files checked ✓
- Manifest validated ✓

### CI Testing
The workflow will be tested when:
1. Rootfs.7z is added to assets
2. Code is pushed to main/develop branch
3. Or manually triggered via workflow_dispatch

### Manual Testing Checklist
- [ ] Settings opens as launcher
- [ ] Launch Container button works
- [ ] Profile creation works
- [ ] Profile switching works
- [ ] Profile deletion works
- [ ] Import .tar rootfs works
- [ ] Import .7z rootfs works
- [ ] Export rootfs works
- [ ] Container stays running when opening settings
- [ ] Symlink properly points to active profile

## Breaking Changes

### Migration Required
Existing users will experience a one-time migration:
- Old rootfs moved to `profiles/default/rootfs/`
- Symlink created at old rootfs location
- Data and settings preserved

### New Default Behavior
- App opens to Settings instead of Container
- User must click "Launch Container" to start

## Documentation

Created comprehensive documentation:
- `.github/workflows/README.md` - Workflow usage
- `PROFILE_MANAGER.md` - Profile manager guide
- This file - Complete change summary

## Security Considerations

- Profile switching requires reboot (prevents state corruption)
- Cannot delete active or default profile (prevents data loss)
- Import validates file format before extraction
- Export uses standard tar format (widely compatible)

## Future Enhancements (Not Implemented)

Potential improvements for future work:
- Profile export/import (entire profile with settings)
- Profile renaming
- Profile cloning
- Custom profile icons/colors
- Profile-specific app installations
- Backup/restore functionality
- Cloud sync for profiles
