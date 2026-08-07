#!/usr/bin/env python3
"""Patch vendor.img to remove forceencrypt=/dev/block/vdd from fstab.ranchu.

Background
----------
The user's original task was to "patch the ramdisk" to remove fileencryption
from fstab.ranchu.early. That approach does NOT work for this system image
because:

  1. The ramdisk (ramdisk.img) does NOT contain any fstab.* files. The only
     `fileencryption=` / `forceencrypt=` strings in the ramdisk are string
     constants inside the statically-linked /init binary (used by libfs_mgr
     to parse fstab files at runtime).

  2. The actual fstab that mounts /data is `/vendor/etc/fstab.ranchu`
     (NOT `fstab.ranchu.early`). It lives inside vendor.img and is loaded
     by `/vendor/etc/init/hw/init.ranchu.rc` via
         mount_all /vendor/etc/fstab.ranchu

  3. The flag in this fstab is `forceencrypt=/dev/block/vdd`, not
     `fileencryption=software`. Either way, the symptom is identical:
     libfs_mgr sees an encryption flag, fails to set up encryption (no
     keymaster, no real key partition), and silently falls back to
     mounting /data with MS_RDONLY.

The fix
-------
vendor.img is an ext4 image with metadata_csum DISABLED (verified via the
superblock feature_ro_compat = 0x7b, bit 0x400 is clear). That means we
can safely do an in-place binary search-and-replace on the data blocks
without recomputing any checksums.

We find the unique byte sequence `forceencrypt=/dev/block/vdd` (28 bytes)
inside vendor.img and overwrite it with a same-length, parser-safe
placeholder. libfs_mgr's flag parser uses strtok_r on commas and then
strcmps each token against known flag names; an unknown token is logged
but does not cause failure. We replace the 28 bytes with 28 dashes so
the resulting token is clearly a "patched-out" placeholder.
"""
import os
import shutil
import sys

SYSIMG = sys.argv[1] if len(sys.argv) > 1 else \
    "/tmp/my-project/android-sdk/system-images/android-28/default/x86_64"
VENDOR_IN = os.path.join(SYSIMG, "vendor.img")
VENDOR_OUT = sys.argv[2] if len(sys.argv) > 2 else \
    "/tmp/my-project/vendor_patched.img"

# The exact byte sequence we want to neutralise. It appears exactly once
# in vendor.img (inside /etc/fstab.ranchu).
NEEDLE = b"forceencrypt=/dev/block/vdd"   # 27 bytes
assert len(NEEDLE) == 27
# Same-length replacement: 27 dashes. After comma-split, this becomes a
# single token "---------------------------" which matches no known
# fs_mgr flag and is therefore ignored (just logged as "unknown flag").
REPLACEMENT = b"-" * len(NEEDLE)


def main():
    print(f"Reading {VENDOR_IN}...")
    with open(VENDOR_IN, 'rb') as f:
        data = f.read()
    print(f"  {len(data)} bytes")

    count = data.count(NEEDLE)
    print(f"Found {count} occurrence(s) of {NEEDLE!r}")
    if count == 0:
        print("ERROR: needle not found. Cannot patch.")
        return 1
    if count > 1:
        print(f"ERROR: needle appears {count} times; expected exactly 1.")
        return 1

    offset = data.find(NEEDLE)
    print(f"  at offset {offset} (0x{offset:x})")

    # Show context before patching
    ctx_start = max(0, offset - 40)
    ctx_end = min(len(data), offset + len(NEEDLE) + 40)
    print(f"  context before: {data[ctx_start:ctx_end]!r}")

    new_data = data.replace(NEEDLE, REPLACEMENT)
    print(f"  context after : {new_data[ctx_start:ctx_end]!r}")

    print(f"Writing {VENDOR_OUT}...")
    with open(VENDOR_OUT, 'wb') as f:
        f.write(new_data)
    print(f"  {len(new_data)} bytes written")

    # Sanity-check: verify the patched file no longer contains the needle
    with open(VENDOR_OUT, 'rb') as f:
        check = f.read()
    remaining = check.count(NEEDLE)
    print(f"Verification: {remaining} occurrence(s) of needle remain in patched file")
    if remaining != 0:
        print("ERROR: patching failed!")
        return 1

    print("\nDone. Boot the emulator with:")
    print(f"  emulator ... -vendor {VENDOR_OUT}")
    return 0


if __name__ == '__main__':
    sys.exit(main())
