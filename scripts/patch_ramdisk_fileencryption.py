#!/usr/bin/env python3
"""Patch ramdisk to remove fileencryption from fstab.

libfs_mgr in Android 9 (API 28) sets MS_RDONLY on /data when it encounters
`fileencryption=software` in the fstab entry but cannot actually enable
file-based encryption (no real userdata key, no keymaster, etc.). The
result is that /data mounts read-only and the boot loops or hangs.

The fix: strip `fileencryption=...` (and the older `forceencrypt=...`)
tokens out of the fstab files embedded in the ramdisk so libfs_mgr no
longer forces MS_RDONLY.

Note: the SDK lives at /tmp/my-project/android-sdk, not /home/z/my-project/.android-sdk
(the task description's path is wrong). Pass any SYSIMG / OUT as args.
"""
import gzip
import io
import os
import sys

SYSIMG = sys.argv[1] if len(sys.argv) > 1 else \
    "/tmp/my-project/android-sdk/system-images/android-28/default/x86_64"
RAMDISK_IN = os.path.join(SYSIMG, "ramdisk.img")
RAMDISK_OUT = sys.argv[2] if len(sys.argv) > 2 else \
    "/tmp/my-project/ramdisk_patched.img"


def parse_cpio(data):
    """Parse a newc-format cpio archive -> [(name, data, mode), ...]."""
    entries = []
    pos = 0
    while pos < len(data):
        if data[pos:pos + 6] == b'070701':
            header = data[pos:pos + 110]
            if len(header) < 110:
                break
            namesize = int(header[94:102], 16)
            filesize = int(header[54:62], 16)
            mode = int(header[14:22], 16)

            name_start = pos + 110
            name = data[name_start:name_start + namesize - 1].decode('ascii', errors='replace')

            data_start = name_start + namesize
            if data_start % 4 != 0:
                data_start += 4 - (data_start % 4)

            file_data = data[data_start:data_start + filesize]

            next_pos = data_start + filesize
            if next_pos % 4 != 0:
                next_pos += 4 - (next_pos % 4)

            if name == 'TRAILER!!!':
                break

            entries.append((name, file_data, mode))
            pos = next_pos
        else:
            pos += 1
    return entries


def build_cpio(entries):
    """Build a newc-format cpio archive from [(name, data, mode), ...]."""
    out = io.BytesIO()
    for name, data, mode in entries:
        name_bytes = name.encode('ascii') + b'\0'
        namesize = len(name_bytes)
        filesize = len(data)

        header = b'070701'
        header += b'%08x' % 0          # ino
        header += b'%08x' % mode       # mode
        header += b'%08x' % 0          # uid
        header += b'%08x' % 0          # gid
        header += b'%08x' % 1          # nlink
        header += b'%08x' % 0          # mtime
        header += b'%08x' % filesize   # filesize
        header += b'%08x' % 0          # devmajor
        header += b'%08x' % 0          # devminor
        header += b'%08x' % 0          # rdevmajor
        header += b'%08x' % 0          # rdevminor
        header += b'%08x' % namesize   # namesize
        header += b'%08x' % 0          # check

        out.write(header)
        out.write(name_bytes)
        out.write(b'\0' * ((4 - ((110 + namesize) % 4)) % 4))
        out.write(data)
        out.write(b'\0' * ((4 - (filesize % 4)) % 4))

    trailer_name = b'TRAILER!!!\0'
    trailer_header = b'070701'
    for _ in range(11):
        trailer_header += b'%08x' % 0
    trailer_header += b'%08x' % len(trailer_name)
    trailer_header += b'%08x' % 0
    out.write(trailer_header)
    out.write(trailer_name)
    out.write(b'\0' * ((4 - ((110 + len(trailer_name)) % 4)) % 4))

    pos = out.tell()
    if pos % 512 != 0:
        out.write(b'\0' * (512 - pos % 512))
    return out.getvalue()


def strip_token(comma_list, token_prefix):
    """Remove a `key=value` token (matched by prefix) from a comma-separated string."""
    parts = comma_list.split(',')
    new_parts = [p for p in parts if not p.startswith(token_prefix)]
    return ','.join(new_parts)


def patch_fstab(content):
    """Remove fileencryption= and forceencrypt= tokens from every fstab line.

    Tokens live in the 4th (mnt_flags) and 5th (fs_mgr_flags) whitespace-
    separated fields. We strip the comma-separated token in place so we
    don't change the field count.
    """
    new_lines = []
    changed = False
    for line in content.split('\n'):
        stripped = line.strip()
        if not stripped or stripped.startswith('#'):
            new_lines.append(line)
            continue
        fields = line.split()
        new_fields = []
        for idx, fld in enumerate(fields):
            if ',' in fld and idx >= 3:
                original = fld
                fld = strip_token(fld, 'fileencryption')
                fld = strip_token(fld, 'forceencrypt')
                if fld != original:
                    changed = True
                fld = fld.strip(',')
                if not fld:
                    fld = 'defaults' if idx == 3 else 'wait'
            new_fields.append(fld)
        new_lines.append(' '.join(new_fields))
    return '\n'.join(new_lines), changed


def main():
    print(f"Reading {RAMDISK_IN}...")
    with open(RAMDISK_IN, 'rb') as f:
        compressed = f.read()
    print(f"  {len(compressed)} bytes compressed")

    decompressed = gzip.decompress(compressed)
    print(f"  {len(decompressed)} bytes decompressed")

    entries = parse_cpio(decompressed)
    print(f"  {len(entries)} cpio entries")

    any_changed = False
    for i, (name, data, mode) in enumerate(entries):
        if name.startswith('fstab.') or name.startswith('/fstab.'):
            content = data.decode('ascii', errors='replace')
            print(f"\n=== {name} (before) ===")
            print(content)
            new_content, changed = patch_fstab(content)
            if changed:
                print(f"=== {name} (after) ===")
                print(new_content)
                entries[i] = (name, new_content.encode('ascii'), mode)
                any_changed = True

    if not any_changed:
        print("\nNo fileencryption/forceencrypt tokens found in any fstab.")
        return 1

    print("\nRebuilding cpio...")
    new_cpio = build_cpio(entries)
    print(f"  new cpio: {len(new_cpio)} bytes")

    compressed_out = gzip.compress(new_cpio)
    print(f"  compressed: {len(compressed_out)} bytes")

    with open(RAMDISK_OUT, 'wb') as f:
        f.write(compressed_out)
    print(f"\nWrote {RAMDISK_OUT}")
    return 0


if __name__ == '__main__':
    sys.exit(main())
