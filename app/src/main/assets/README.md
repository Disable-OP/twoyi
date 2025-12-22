# Asset Placeholders

These files are minimal placeholders to allow CI builds to complete.

## For Production Use

Replace these files with actual ROM files:

1. **rootfs.7z** - Download from: https://github.com/cyanmint/twoyi/releases/download/original/rootfs.7z
   - Or extract from an official release APK
   - This should be a valid 7z archive containing the Android rootfs

2. **rom.ini** - Should be included in the rootfs.7z archive
   - Contains ROM metadata (author, version, code, desc, md5)

## CI Builds

The placeholder files allow the build to compile but the resulting APK will not be functional without a real rootfs.

## Local Development

For local development and testing:
1. Download the real rootfs.7z
2. Replace the placeholder files
3. Build the APK
