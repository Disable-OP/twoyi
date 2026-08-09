#!/usr/bin/env python3
"""Patch the ramdisk to remove vendor partition from fstab.
This allows the AOSP 8.1 default system image to boot without a vendor partition.
"""
import gzip
import io
import os
import struct
import sys

RAMDISK_IN = sys.argv[1] if len(sys.argv) > 1 else "/home/z/my-project/.android-sdk/system-images/android-27/default/x86_64/ramdisk.img"
RAMDISK_OUT = sys.argv[2] if len(sys.argv) > 2 else "/home/z/my-project/.avd-ramdisk.img"

def parse_cpio(data):
    """Parse a cpio archive (newc format) and return list of (name, data, mode) tuples."""
    entries = []
    pos = 0
    while pos < len(data):
        # Check for the trailer
        if data[pos:pos+6] == b'070701':
            # Newc format header
            header = data[pos:pos+110]
            if len(header) < 110:
                break

            # Parse header fields (hex strings)
            namesize = int(header[94:102], 16)
            filesize = int(header[54:62], 16)
            mode = int(header[14:22], 16)

            # Name starts at offset 110, padded to 4-byte boundary
            name_start = pos + 110
            name = data[name_start:name_start+namesize-1].decode('ascii', errors='replace')

            # Data starts after name, padded to 4-byte boundary
            data_start = name_start + namesize
            if data_start % 4 != 0:
                data_start += 4 - (data_start % 4)

            file_data = data[data_start:data_start+filesize]

            # Next entry after data, padded to 4-byte boundary
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
    """Build a cpio archive (newc format) from entries."""
    out = io.BytesIO()
    for name, data, mode in entries:
        name_bytes = name.encode('ascii') + b'\0'
        namesize = len(name_bytes)
        filesize = len(data)

        # Build header
        header = b'070701'  # magic
        header += b'%08x' % 0  # ino
        header += b'%08x' % mode  # mode
        header += b'%08x' % 0  # uid
        header += b'%08x' % 0  # gid
        header += b'%08x' % 1  # nlink
        header += b'%08x' % 0  # mtime
        header += b'%08x' % filesize  # filesize
        header += b'%08x' % 0  # devmajor
        header += b'%08x' % 0  # devminor
        header += b'%08x' % 0  # rdevmajor
        header += b'%08x' % 0  # rdevminor
        header += b'%08x' % namesize  # namesize
        header += b'%08x' % 0  # check

        out.write(header)

        # Write name (padded to 4 bytes)
        out.write(name_bytes)
        padding = (4 - ((110 + namesize) % 4)) % 4
        out.write(b'\0' * padding)

        # Write data (padded to 4 bytes)
        out.write(data)
        padding = (4 - (filesize % 4)) % 4
        out.write(b'\0' * padding)

    # Write trailer
    trailer_name = b'TRAILER!!!\0'
    trailer_header = b'070701'
    trailer_header += b'%08x' % 0  # ino
    trailer_header += b'%08x' % 0  # mode
    trailer_header += b'%08x' % 0  # uid
    trailer_header += b'%08x' % 0  # gid
    trailer_header += b'%08x' % 1  # nlink
    trailer_header += b'%08x' % 0  # mtime
    trailer_header += b'%08x' % 0  # filesize
    trailer_header += b'%08x' % 0  # devmajor
    trailer_header += b'%08x' % 0  # devminor
    trailer_header += b'%08x' % 0  # rdevmajor
    trailer_header += b'%08x' % 0  # rdevminor
    trailer_header += b'%08x' % len(trailer_name)  # namesize
    trailer_header += b'%08x' % 0  # check
    out.write(trailer_header)
    out.write(trailer_name)
    padding = (4 - ((110 + len(trailer_name)) % 4)) % 4
    out.write(b'\0' * padding)

    # Pad to 512-byte boundary
    pos = out.tell()
    if pos % 512 != 0:
        out.write(b'\0' * (512 - pos % 512))

    return out.getvalue()

# Read and decompress the ramdisk
print(f"Reading {RAMDISK_IN}...")
with open(RAMDISK_IN, 'rb') as f:
    compressed = f.read()

print(f"Decompressing ({len(compressed)} bytes compressed)...")
decompressed = gzip.decompress(compressed)
print(f"Decompressed: {len(decompressed)} bytes")

# Parse the cpio archive
entries = parse_cpio(decompressed)
print(f"Found {len(entries)} entries in ramdisk")

# Find and modify fstab files
modified = False
for i, (name, data, mode) in enumerate(entries):
    if name.startswith('fstab.') or name.startswith('/fstab.'):
        print(f"\nFound fstab: {name}")
        content = data.decode('ascii', errors='replace')
        print(f"Original content:")
        print(content)

        # Remove or comment out vendor lines
        lines = content.split('\n')
        new_lines = []
        for line in lines:
            if '/vendor' in line and not line.strip().startswith('#'):
                print(f"  COMMENTING OUT: {line.strip()}")
                new_lines.append('# ' + line)
                modified = True
            else:
                new_lines.append(line)

        new_content = '\n'.join(new_lines)
        entries[i] = (name, new_content.encode('ascii'), mode)

if not modified:
    print("\nNo vendor entries found in fstab files.")
    # Check all entries for vendor references
    for name, data, mode in entries:
        content = data.decode('ascii', errors='replace')
        if 'vendor' in content.lower():
            print(f"\n{name} contains 'vendor':")
            for line in content.split('\n'):
                if 'vendor' in line.lower():
                    print(f"  {line.strip()}")
else:
    # Rebuild the cpio archive
    print("\nRebuilding cpio archive...")
    new_cpio = build_cpio(entries)
    print(f"New cpio size: {len(new_cpio)} bytes")

    # Compress with gzip
    print("Compressing with gzip...")
    compressed_out = gzip.compress(new_cpio)
    print(f"Compressed size: {len(compressed_out)} bytes")

    # Write the new ramdisk
    print(f"Writing to {RAMDISK_OUT}...")
    with open(RAMDISK_OUT, 'wb') as f:
        f.write(compressed_out)

    print(f"\nDone! Modified ramdisk written to {RAMDISK_OUT}")
    print(f"Original size: {len(compressed)} bytes")
    print(f"New size: {len(compressed_out)} bytes")
