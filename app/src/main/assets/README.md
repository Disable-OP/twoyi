# Asset Placeholders

These files are minimal placeholders to allow CI builds to complete.

## For Production Use

**No longer bundled in the APK:**
- Users must download and import rootfs.7z manually after installing the app
- Download from: https://github.com/cyanmint/twoyi/releases/download/original/rootfs.7z

## How to Use

1. Install the Twoyi APK
2. Open the app (will show Settings screen)
3. Download rootfs.7z from the release page
4. Go to Settings → Advanced → Import Rootfs
5. Select the downloaded rootfs.7z file
6. Wait for extraction to complete
7. Tap "Launch Container" to start

## CI Builds

The placeholder files allow the build to compile. The resulting APK requires users to manually import a rootfs file.

## ROM Metadata

The rom.ini file should be included in the rootfs.7z archive and contains:
- author
- version
- code (version number)
- desc (description)
- md5 (checksum)

