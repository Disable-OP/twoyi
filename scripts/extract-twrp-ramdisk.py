#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
"""
Extract the TWRP recovery ramdisk from an Android boot image.

TWRP ships as a single Android bootimg (kernel + ramdisk). The kernel
is the host kernel's responsibility (we never boot it under twoyi —
twoyi's kr64 IS the kernel-replacement). We only need the RAMDISK,
which contains:

  - /init                — statically linked i386 init binary
  - /sbin/recovery       — dynamically linked i386 recovery UI
  - /sbin/linker         — 32-bit Android dynamic linker
  - /system, /vendor     — minimal recovery rootfs
  - init.rc              — recovery boot script (very simple)
  - ueventd.rc, fstab.*  — recovery mount tables

Usage:
    scripts/extract-twrp-ramdisk.py \\
        --boot-img assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img \\
        --output-dir /tmp/twrp-rootfs

The output directory will contain the ramdisk contents (3107 files for
TWRP 3.7.0_9-0 for byt_t_crv2). This can be tar'd and pushed into
/data/data/io.twoyi/profiles/default/rootfs for kr64 to pivot_root into.

The script ALSO supports writing a .tar file (the format twoyi's
ProfileManager expects) via --output-tar. Use either --output-dir OR
--output-tar (not both).

Boot image format (Android bootimg v0):
  - 2048-byte header (magic "ANDROID!", 9 uint32 fields, cmdline, ...)
  - kernel (page-aligned, padded to page_size)
  - ramdisk (page-aligned, gzip-compressed cpio)
  - second stage (usually 0 bytes for TWRP)

The ramdisk cpio is SVR4 format ("newc", magic 070701):
  - 6 ASCII magic bytes
  - 13 fields of 8 ASCII hex chars each (ino, mode, uid, gid, nlink,
    mtime, filesize, devmajor, devminor, rdevmajor, rdevminor,
    namesize, check)
  - Total header: 110 bytes
  - Then name (namesize bytes, NUL-padded to 4-byte boundary)
  - Then file data (filesize bytes, NUL-padded to 4-byte boundary)
  - "TRAILER!!!" name marks end of archive

Symlink handling: this script tries to create real symlinks (which is
what kr64's pivot_root expects). On filesystems that block symlink
creation (some sandboxes, some Android data partitions before app
startup), it falls back to writing a `.lnk` file containing the link
target as ASCII text. The KVM e2e test runs `adb push` to a real ext4
partition where symlinks work, so the fallback is rarely needed.
"""
from __future__ import annotations

import argparse
import gzip
import math
import os
import struct
import sys
import tarfile
import io
from typing import Optional


# ---------------------------------------------------------------------------
# Boot image header parsing (Android bootimg v0)
# ---------------------------------------------------------------------------

BOOT_MAGIC = b"ANDROID!"
BOOT_MAGIC_SIZE = 8


def parse_boot_header(header: bytes) -> dict:
    """Parse the 2048-byte Android boot image header.

    Returns a dict with the fields we need:
        kernel_size, kernel_addr, ramdisk_size, ramdisk_addr,
        second_size, second_addr, tags_addr, page_size, header_version
    """
    if header[:BOOT_MAGIC_SIZE] != BOOT_MAGIC:
        raise ValueError(
            f"not an Android boot image: bad magic {header[:BOOT_MAGIC_SIZE]!r} "
            f"(expected {BOOT_MAGIC!r})"
        )
    # 9 uint32 little-endian fields after the 8-byte magic.
    fields = struct.unpack("<9I", header[BOOT_MAGIC_SIZE:BOOT_MAGIC_SIZE + 36])
    keys = (
        "kernel_size",
        "kernel_addr",
        "ramdisk_size",
        "ramdisk_addr",
        "second_size",
        "second_addr",
        "tags_addr",
        "page_size",
        "header_version",
    )
    return dict(zip(keys, fields))


def read_ramdisk(boot_img_path: str) -> bytes:
    """Read the gzip-compressed ramdisk from an Android boot image.

    Returns the DECOMPRESSED cpio archive bytes.
    """
    with open(boot_img_path, "rb") as f:
        header = f.read(2048)
        if len(header) < 2048:
            raise ValueError(
                f"boot image too small: {len(header)} bytes (need >= 2048 for header)"
            )
        meta = parse_boot_header(header)
        page_size = meta["page_size"]
        if page_size not in (2048, 4096):
            # The format technically allows any power of two >= 2048,
            # but every shipping Android image uses one of these two.
            print(
                f"warning: unusual page_size {page_size} (expected 2048 or 4096)",
                file=sys.stderr,
            )

        kernel_pages = math.ceil(meta["kernel_size"] / page_size) if meta["kernel_size"] else 0
        second_pages = math.ceil(meta["second_size"] / page_size) if meta["second_size"] else 0
        # Layout: 1 page header | kernel_pages | ramdisk_pages | second_pages
        ramdisk_offset = (1 + kernel_pages) * page_size
        # Sanity: if second stage is present, it comes after ramdisk
        second_offset = (1 + kernel_pages + math.ceil(meta["ramdisk_size"] / page_size)) * page_size
        ramdisk_size = meta["ramdisk_size"]

        if ramdisk_size == 0:
            raise ValueError("boot image has no ramdisk (ramdisk_size=0)")

        f.seek(ramdisk_offset)
        rd = f.read(ramdisk_size)
        if len(rd) != ramdisk_size:
            raise ValueError(
                f"short read: wanted {ramdisk_size} ramdisk bytes at offset "
                f"{ramdisk_offset}, got {len(rd)}"
            )

    if rd[:4] != b"\x1f\x8b\x08\x00":
        raise ValueError(
            f"ramdisk is not gzip-compressed (first 4 bytes: {rd[:4].hex()}); "
            f"this extractor only handles gzip ramdisks"
        )

    cpio = gzip.decompress(rd)
    if cpio[:6] not in (b"070701", b"070702"):
        raise ValueError(
            f"decompressed ramdisk is not a SVR4 cpio archive "
            f"(first 6 bytes: {cpio[:6]!r})"
        )

    print(
        f"boot image: kernel_size={meta['kernel_size']} ramdisk_size={ramdisk_size} "
        f"(gzip) -> cpio_size={len(cpio)} page_size={page_size}",
        file=sys.stderr,
    )
    return cpio


# ---------------------------------------------------------------------------
# SVR4 cpio extraction (magic 070701 / 070702)
# ---------------------------------------------------------------------------

CPIO_MAGIC_NEWC = b"070701"
CPIO_MAGIC_CRC = b"070702"


def iter_cpio_entries(data: bytes):
    """Yield (name, mode, filesize, file_data) for each entry in a SVR4 cpio.

    Stops at the TRAILER!!! entry. Each yielded tuple has:
        name      : str  (entry name, e.g. "sbin/recovery")
        mode      : int  (raw st_mode, including type bits)
        filesize  : int
        file_data : bytes (the file's contents; for symlinks, the target path)
    """
    pos = 0
    n = len(data)
    while pos + 110 <= n:
        magic = data[pos:pos + 6]
        if magic not in (CPIO_MAGIC_NEWC, CPIO_MAGIC_CRC):
            raise ValueError(
                f"bad cpio magic at offset {pos}: {magic!r} "
                f"(expected {CPIO_MAGIC_NEWC!r} or {CPIO_MAGIC_CRC!r})"
            )
        # 13 fields of 8 ASCII hex chars each.
        header_str = data[pos:pos + 110].decode("ascii")
        fields = []
        for i in range(13):
            start = 6 + i * 8
            fields.append(int(header_str[start:start + 8], 16))
        (
            _ino,
            mode,
            _uid,
            _gid,
            _nlink,
            _mtime,
            filesize,
            _devmajor,
            _devminor,
            _rdevmajor,
            _rdevminor,
            namesize,
            _check,
        ) = fields

        name_start = pos + 110
        name_end = name_start + namesize
        # Name is NUL-padded to a 4-byte boundary AFTER the 110-byte header.
        name = data[name_start:name_end].rstrip(b"\x00").decode("ascii", errors="replace")

        # Data starts at the next 4-byte boundary after the name.
        data_start = (name_end + 3) & ~3
        data_end = data_start + filesize
        file_data = data[data_start:data_end]

        # Next entry starts at the next 4-byte boundary after the data.
        next_pos = (data_end + 3) & ~3

        if name == "TRAILER!!!" or not name:
            return  # end of archive

        yield (name, mode, filesize, file_data)
        pos = next_pos


def mode_type_str(mode: int) -> str:
    """Return a short string describing the cpio entry's type."""
    t = mode & 0o170000
    return {
        0o040000: "dir",
        0o100000: "file",
        0o120000: "symlink",
        0o020000: "chardev",
        0o060000: "blockdev",
        0o010000: "fifo",
        0o140000: "socket",
    }.get(t, f"unknown(0o{t:o})")


def safe_join(base: str, name: str) -> str:
    """Join base + name, refusing to escape base via .. or absolute paths.

    TWRP's cpio archive contains entries like "sbin/recovery", "init",
    "system/lib/foo.so" — all relative. But defensive validation
    prevents path traversal if a malicious archive contains "../.."
    entries.
    """
    if name.startswith("/"):
        name = name.lstrip("/")
    parts = []
    for part in name.split("/"):
        if part in ("", ".", ".."):
            continue
        parts.append(part)
    return os.path.join(base, *parts)


def extract_cpio_to_dir(data: bytes, outdir: str) -> dict:
    """Extract a SVR4 cpio archive to a directory.

    Returns a stats dict with counts: {dirs, files, symlinks, devs, other,
    symlink_fallbacks}.
    """
    os.makedirs(outdir, exist_ok=True)
    stats = {"dirs": 0, "files": 0, "symlinks": 0, "devs": 0, "other": 0, "symlink_fallbacks": 0}

    for name, mode, _filesize, file_data in iter_cpio_entries(data):
        dest = safe_join(outdir, name)
        mode_type = mode & 0o170000

        if mode_type == 0o040000:  # directory
            os.makedirs(dest, exist_ok=True)
            try:
                os.chmod(dest, mode & 0o7777)
            except OSError:
                pass
            stats["dirs"] += 1

        elif mode_type == 0o100000:  # regular file
            os.makedirs(os.path.dirname(dest) or ".", exist_ok=True)
            with open(dest, "wb") as out:
                out.write(file_data)
            try:
                os.chmod(dest, mode & 0o7777)
            except OSError:
                pass
            stats["files"] += 1

        elif mode_type == 0o120000:  # symlink
            target = file_data.decode("utf-8", errors="replace")
            os.makedirs(os.path.dirname(dest) or ".", exist_ok=True)
            if os.path.lexists(dest):
                try:
                    os.remove(dest)
                except OSError:
                    pass
            try:
                os.symlink(target, dest)
                stats["symlinks"] += 1
            except OSError as e:
                # Some sandboxes (and some Android data partitions before
                # app startup) block symlink creation. Fall back to writing
                # a .lnk file containing the target as ASCII. The KVM e2e
                # test runs on a real ext4 partition where symlinks work.
                lnk_path = dest + ".lnk"
                with open(lnk_path, "w", encoding="ascii") as out:
                    out.write(target)
                stats["symlink_fallbacks"] += 1
                # Don't print a warning for every symlink — too noisy.
                # The summary at the end reports the fallback count.

        elif mode_type in (0o020000, 0o060000):  # char/block device
            # Can't mknod without root; skip but count.
            stats["devs"] += 1

        else:  # fifo, socket, etc.
            stats["other"] += 1

    return stats


# ---------------------------------------------------------------------------
# Tar output (for pushing to the device via adb push)
# ---------------------------------------------------------------------------


def extract_cpio_to_tar(data: bytes, out_tar: str) -> dict:
    """Extract a SVR4 cpio archive to a tar file.

    The tar preserves directory/file/symlink types and modes. Device
    nodes (char/block) are skipped because tarfile.addfile() with
    character/block device types is finicky and kr64 creates its own
    /dev/ anyway.

    Returns a stats dict (same shape as extract_cpio_to_dir).
    """
    stats = {"dirs": 0, "files": 0, "symlinks": 0, "devs": 0, "other": 0, "symlink_fallbacks": 0}
    # Use a streaming tar with format GNU to handle long names + symlinks.
    with tarfile.open(out_tar, "w", format=tarfile.GNU_FORMAT) as tf:
        for name, mode, _filesize, file_data in iter_cpio_entries(data):
            mode_type = mode & 0o170000
            # tarfile wants a TarInfo with name + size.
            ti = tarfile.TarInfo(name)
            ti.mode = mode & 0o7777
            ti.mtime = 0  # deterministic; we don't have per-entry mtime parsed

            if mode_type == 0o040000:
                ti.type = tarfile.DIRTYPE
                ti.size = 0
                tf.addfile(ti)
                stats["dirs"] += 1
            elif mode_type == 0o100000:
                ti.type = tarfile.REGTYPE
                ti.size = len(file_data)
                tf.addfile(ti, io.BytesIO(file_data))
                stats["files"] += 1
            elif mode_type == 0o120000:
                target = file_data.decode("utf-8", errors="replace")
                ti.type = tarfile.SYMTYPE
                ti.linkname = target
                ti.size = 0
                tf.addfile(ti)
                stats["symlinks"] += 1
            elif mode_type in (0o020000, 0o060000):
                # Skip device nodes — kr64 creates /dev/ at runtime.
                stats["devs"] += 1
            else:
                stats["other"] += 1
    return stats


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def main(argv: Optional[list] = None) -> int:
    p = argparse.ArgumentParser(
        description="Extract the TWRP recovery ramdisk from an Android boot image.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    p.add_argument(
        "--boot-img",
        required=True,
        help="Path to the Android boot image (e.g. assets/twrp/twrp-3.7.0_9-0-byt_t_crv2.img)",
    )
    out = p.add_mutually_exclusive_group(required=True)
    out.add_argument(
        "--output-dir",
        help="Directory to extract the ramdisk contents into (created if missing)",
    )
    out.add_argument(
        "--output-tar",
        help="Write the ramdisk contents to a tar file at this path (for adb push)",
    )
    args = p.parse_args(argv)

    if not os.path.isfile(args.boot_img):
        print(f"error: boot image not found: {args.boot_img}", file=sys.stderr)
        return 2

    cpio = read_ramdisk(args.boot_img)

    if args.output_dir:
        print(f"→ extracting ramdisk to directory: {args.output_dir}", file=sys.stderr)
        stats = extract_cpio_to_dir(cpio, args.output_dir)
    else:
        print(f"→ writing ramdisk to tar: {args.output_tar}", file=sys.stderr)
        # tarfile.open creates the file; ensure parent dir exists.
        parent = os.path.dirname(os.path.abspath(args.output_tar))
        os.makedirs(parent, exist_ok=True)
        stats = extract_cpio_to_tar(cpio, args.output_tar)

    total = sum(stats.values())
    print(
        f"✓ extracted {total} entries: "
        f"{stats['dirs']} dirs, {stats['files']} files, {stats['symlinks']} symlinks, "
        f"{stats['devs']} device nodes skipped, {stats['other']} other skipped",
        file=sys.stderr,
    )
    if stats["symlink_fallbacks"] > 0:
        print(
            f"  ⚠ {stats['symlink_fallbacks']} symlinks were written as .lnk files "
            f"(filesystem blocks symlink creation). This is fine for inspection but "
            f"will break kr64's pivot_root — re-extract on a filesystem that supports "
            f"symlinks (e.g. /tmp on Linux, or push the .tar to the device and extract "
            f"via `tar xf` on ext4).",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
