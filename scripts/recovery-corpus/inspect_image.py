#!/usr/bin/env python3
"""Universal Android recovery image inspector for the Twoyi corpus.

Given a recovery image (.img Android boot image, or a path we can extract
one from), produce machine-readable metadata.json:

  - sha256/md5, size
  - boot-image format (header version, kernel/ramdisk sizes, page size)
  - ramdisk compression format
  - guest rootfs listing (top-level)
  - init binary: ELF class/machine/static-or-dynamic/PT_INTERP
  - key binaries under /sbin, /system/bin (static vs dynamic, interpreters)
  - recovery family + version detection (TWRP/OrangeFox/SHRP/Lineage/AOSP)
  - kernel cmdline

Design goal (master prompt §25): every image is identified BEFORE boot;
identical artifacts dedupe by SHA256.

Usage: inspect_image.py <image.img> [--out metadata.json] [--root-dir DIR]
  --root-dir: extract the ramdisk tree to DIR (optional)
"""
import argparse
import gzip
import hashlib
import json
import lzma
import os
import struct
import sys
import zlib

try:
    import lz4.frame as lz4frame  # optional
except ImportError:
    lz4frame = None


# ────────────────────────────── boot image header ──────────────────────────

BOOT_MAGIC = b"ANDROID!"


def parse_boot_header(data: bytes):
    if len(data) < 8 or data[:8] != BOOT_MAGIC:
        return None
    h = {"magic": "ANDROID!"}
    # ── try v0/v1/v2 layout first ──────────────────────────────
    # magic(8) kernel_size kernel_addr ramdisk_size ramdisk_addr
    # second_size second_addr tags_addr page_size dt_size unused
    (k_size, k_addr, r_size, r_addr, s_size, s_addr,
     tags_addr, page_size, dt_size, _unused) = struct.unpack(
        "<10I", data[8:8 + 40])
    if page_size not in (512, 1024, 2048, 4096):
        # ── v3/v4 layout: magic(8) kernel_size ramdisk_size
        #    os_version header_size reserved[4] header_version cmdline…
        if len(data) >= 44:
            (k_size2, r_size2, os_ver, hdr_size) = struct.unpack_from(
                "<4I", data, 8)
            hver, = struct.unpack_from("<I", data, 40)
            if hver in (3, 4):
                page_size = 4096
                h.update({
                    "kernel_size": k_size2, "ramdisk_size": r_size2,
                    "page_size": page_size, "dt_size": 0,
                    "header_version": hver, "os_version_raw": os_ver,
                    "cmdline": data[44:44 + 1560].rstrip(b"\x00").decode(
                        "ascii", "replace"),
                    "extra_cmdline": "",
                    "product_name": "",
                })
                h["_offsets"] = {"kernel": page_size,
                                 "ramdisk": page_size}
                h["kernel_bytes"] = data[page_size:page_size + k_size2]
                # v3: ramdisk directly after kernel pages
                kp = page_size + ((k_size2 + page_size - 1)
                                  // page_size) * page_size
                h["_offsets"]["ramdisk"] = kp
                h["ramdisk_bytes"] = data[kp:kp + r_size2]
                return h
        return None
    h.update({
        "kernel_size": k_size, "kernel_addr": k_addr,
        "ramdisk_size": r_size, "ramdisk_addr": r_addr,
        "second_size": s_size, "page_size": page_size,
        "dt_size": dt_size, "tags_addr": tags_addr,
        "header_version": (2 if _unused == 2 else
                            (1 if dt_size else 0)),
    })
    off = 8 + 40
    h["product_name"] = data[off:off + 16].rstrip(b"\x00").decode(
        "ascii", "replace")
    off += 16
    h["cmdline"] = data[off:off + 512].rstrip(b"\x00").decode(
        "ascii", "replace")
    off += 512 + 32  # skip id
    h["extra_cmdline"] = data[off:off + 1024].rstrip(b"\x00").decode(
        "ascii", "replace")
    pages = lambda n: (n + page_size - 1) // page_size
    kernel_off = page_size
    ramdisk_off = kernel_off + pages(k_size) * page_size
    second_off = ramdisk_off + pages(r_size) * page_size
    h["_offsets"] = {"kernel": kernel_off, "ramdisk": ramdisk_off,
                     "second": second_off}
    h["kernel_bytes"] = data[kernel_off:kernel_off + k_size]
    h["ramdisk_bytes"] = data[ramdisk_off:ramdisk_off + r_size]
    return h


# ────────────────────────────── ramdisk decompress ─────────────────────────

def sniff_and_decompress(blob: bytes):
    """Return (format_name, cpio_bytes)."""
    if blob[:2] == b"\x1f\x8b":
        return "gzip", gzip.decompress(blob)
    if blob[:4] == b"\x04\x22\x4d\x18":
        if lz4frame is None:
            return "lz4", None
        return "lz4", lz4frame.decompress(blob)
    if blob[:6] == b"\xfd7zXZ\x00":
        return "xz", lzma.decompress(blob)
    if blob[:2] == b"\x5d\x00":  # lzma alone
        try:
            return "lzma", lzma.decompress(blob)
        except lzma.LZMAError:
            pass
    if blob[:6] in (b"070701", b"070702"):
        return "none(cpio)", blob
    try:
        return "zlib", zlib.decompress(blob)
    except zlib.error:
        pass
    if b"070701" in blob[:4096]:
        return "unknown(cpio)", blob
    return "unknown", None


# ────────────────────────────── cpio (newc) parser ─────────────────────────

CPIO_MAGIC = b"070701"


def parse_cpio(data: bytes):
    """Yield (name, mode, filesize, filebytes). Supports newc."""
    entries = []
    off = 0
    n = len(data)
    while off + 110 <= n:
        if data[off:off + 6] != CPIO_MAGIC:
            idx = data.find(CPIO_MAGIC, off)
            if idx < 0:
                break
            off = idx
        try:
            f = [int(data[off + 6 + 8 * i: off + 6 + 8 * (i + 1)], 16)
                 for i in range(13)]
        except ValueError:
            break
        (ino, mode, uid, gid, nlink, mtime, fsize, dmaj, dmin,
         rmaj, rmin, namesize, chk) = f
        name_off = off + 110
        name = data[name_off:name_off + namesize - 1].decode(
            "utf-8", "replace")
        file_off = (name_off + namesize + 3) & ~3
        filebytes = data[file_off:file_off + fsize]
        entries.append((name, mode, fsize, filebytes))
        off = (file_off + fsize + 3) & ~3
        if name == "TRAILER!!!":
            break
    return entries


# ────────────────────────────── ELF analysis ───────────────────────────────

EM_NAMES = {3: "x86", 40: "arm", 62: "x86_64", 183: "aarch64",
            8: "mips", 243: "riscv64"}


def analyze_elf(blob: bytes):
    if len(blob) < 64 or blob[:4] != b"\x7fELF":
        return None
    ei_class = blob[4]
    ei_data = blob[5]
    is64 = ei_class == 2
    out = {"class": "ELF64" if is64 else "ELF32",
           "endian": "LE" if ei_data == 1 else "BE"}
    if is64:
        e_type, e_machine = struct.unpack_from("<HH", blob, 16)
        e_phoff, = struct.unpack_from("<Q", blob, 32)
        e_phentsize, e_phnum = struct.unpack_from("<HH", blob, 54)
    else:
        e_type, e_machine = struct.unpack_from("<HH", blob, 16)
        e_phoff, = struct.unpack_from("<I", blob, 28)
        e_phentsize, e_phnum = struct.unpack_from("<HH", blob, 42)
    out["type"] = {2: "EXEC", 3: "DYN"}.get(e_type, str(e_type))
    out["machine"] = EM_NAMES.get(e_machine, f"em_{e_machine}")
    interp = None
    dynamic = False
    for i in range(e_phnum):
        ph = blob[e_phoff + i * e_phentsize:
                  e_phoff + (i + 1) * e_phentsize]
        if len(ph) < 8:
            break
        p_type, = struct.unpack_from("<I", ph, 0)
        if p_type == 3:  # PT_INTERP
            if is64:
                p_offset, = struct.unpack_from("<Q", ph, 8)
            else:
                p_offset, = struct.unpack_from("<I", ph, 4)
            end = blob.find(b"\x00", p_offset)
            interp = blob[p_offset:end].decode("utf-8", "replace")
        elif p_type == 2:  # PT_DYNAMIC
            dynamic = True
    out["dynamic"] = dynamic
    out["pt_interp"] = interp
    out["static"] = not dynamic
    return out


# ────────────────────────────── family detection ───────────────────────────

def detect_family(files: dict, cmdline: str):
    fam = {"family": "unknown", "version": None, "extra": {}}
    names = set(files.keys())
    text = b""
    for cand in ("sbin/twrp", "init.rc", "default.prop", "twrp.rc"):
        if cand in files and files[cand][2]:
            text += files[cand][2][:65536]
    if any(n.startswith("twres/") for n in names) or "sbin/twrp" in names:
        fam["family"] = "TWRP"
        for cand in ("default.prop", "prop.default", "twrp.rc"):
            if cand in files and files[cand][2]:
                t = files[cand][2].decode("utf-8", "replace")
                for line in t.splitlines():
                    if "twrp.version" in line and "=" in line:
                        fam["version"] = line.split("=", 1)[1].strip()
    # OrangeFox: specific markers only (the terminfo 'fox' entry in
    # sbin/etc/terminfo/f/fox is a terminal database, NOT OrangeFox).
    ofx_names = ("sbin/fox", "fox.rc", "init.fox.rc", "foxfmttools",
                 "sbin/foxlib", "fox.prop")
    if any(n in names for n in ofx_names) or \
            b"OrangeFox" in text or b"orangefox." in text:
        fam["family"] = "OrangeFox"
    if any("shrp" in n.lower() for n in names) or b"SkyHawk" in text or \
            b"SHRP" in text:
        fam["family"] = "SHRP"
    if fam["family"] in ("unknown",) and "init.rc" in names:
        rc = files["init.rc"][2]
        if rc:
            t = rc.decode("utf-8", "replace")
            if "LineageOS" in t or "lineage" in t:
                fam["family"] = "Lineage"
            else:
                fam["family"] = "AOSP-like"
    return fam


# ────────────────────────────── main ───────────────────────────────────────

def inspect(path: str, extract_dir=None):
    with open(path, "rb") as f:
        data = f.read()
    meta = {
        "image_file": os.path.basename(path),
        "size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
        "md5": hashlib.md5(data).hexdigest(),
    }
    hdr = parse_boot_header(data)
    if hdr is None:
        meta["format"] = "not-an-android-boot-image"
        if data[:4] == b"\x7fELF":
            meta["format"] = "raw-elf"
        elif data[:2] == b"PK":
            meta["format"] = "zip"
        return meta
    meta["format"] = "android-boot"
    meta["boot"] = {
        k: hdr[k] for k in ("kernel_size", "ramdisk_size", "page_size",
                            "dt_size", "product_name", "cmdline",
                            "extra_cmdline", "header_version", "tags_addr")
        if k in hdr}
    fmt, cpio = sniff_and_decompress(hdr["ramdisk_bytes"])
    meta["ramdisk_format"] = fmt
    if cpio is None:
        meta["error"] = f"cannot decompress ramdisk ({fmt})"
        return meta
    entries = parse_cpio(cpio)
    files = {}
    for name, mode, fsize, fbytes in entries:
        files[name] = (mode, fsize, fbytes if fsize <= 8 * 1024 * 1024
                       else None)
    top = sorted({n.split("/", 1)[0] for n in files})
    meta["rootfs_top"] = top
    bins = {}
    key = ["init", "sbin/twrp", "sbin/sh", "sbin/busybox", "sbin/ueventd",
           "sbin/recovery", "system/bin/sh", "system/bin/linker64",
           "system/bin/linker"]
    for k in key:
        if k in files:
            mode, fsize, fbytes = files[k]
            if fbytes and len(fbytes) >= 64:
                elf = analyze_elf(fbytes)
                if elf:
                    bins[k] = {"size": fsize, **elf}
            else:
                bins[k] = {"size": fsize}
    links = {}
    for name, (mode, fsize, fbytes) in files.items():
        if name.startswith("sbin/") and (mode & 0o170000) == 0o120000 \
                and fbytes:
            links[name] = fbytes.decode("utf-8", "replace")
    meta["sbin_symlinks"] = links
    interps = set()
    scanned = 0
    for name, (mode, fsize, fbytes) in sorted(files.items()):
        if fbytes and fsize >= 64 and fbytes[:4] == b"\x7fELF" \
                and scanned < 400:
            elf = analyze_elf(fbytes)
            if elf and elf.get("pt_interp"):
                interps.add(elf["pt_interp"])
            scanned += 1
    meta["pt_interps_present"] = sorted(interps)
    meta["key_binaries"] = bins
    fam = detect_family(files, hdr.get("cmdline", ""))
    meta["recovery"] = fam
    meta["ramdisk_file_count"] = len(files)
    if extract_dir:
        os.makedirs(extract_dir, exist_ok=True)
        for name, (mode, fsize, fbytes) in files.items():
            if not name or name == "TRAILER!!!":
                continue
            dest = os.path.join(extract_dir, name)
            if (mode & 0o170000) == 0o040000:
                os.makedirs(dest, exist_ok=True)
            elif (mode & 0o170000) == 0o120000 and fbytes:
                try:
                    os.symlink(fbytes.decode("utf-8", "replace"), dest)
                except OSError:
                    pass
            elif fbytes is not None:
                os.makedirs(os.path.dirname(dest) or ".", exist_ok=True)
                with open(dest, "wb") as g:
                    g.write(fbytes)
        for name in files:
            d = os.path.dirname(name)
            while d:
                os.makedirs(os.path.join(extract_dir, d), exist_ok=True)
                d = os.path.dirname(d)
    return meta


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("image")
    ap.add_argument("--out", default=None)
    ap.add_argument("--root-dir", default=None)
    args = ap.parse_args()
    meta = inspect(args.image, args.root_dir)
    out = args.out or (os.path.splitext(args.image)[0] + ".metadata.json")
    with open(out, "w") as f:
        json.dump(meta, f, indent=2, sort_keys=True)
    print(f"file:      {meta['image_file']}")
    print(f"sha256:    {meta['sha256'][:32]}…")
    print(f"format:    {meta.get('format')} / ramdisk {meta.get('ramdisk_format')}")
    if "recovery" in meta:
        print(f"family:    {meta['recovery']['family']} "
              f"{meta['recovery'].get('version') or ''}")
    if "rootfs_top" in meta:
        print(f"root top:  {' '.join(meta['rootfs_top'][:24])}")
    if "key_binaries" in meta:
        for k, v in meta["key_binaries"].items():
            if "class" in v:
                print(f"  {k:22} {v['class']:6} {v['machine']:8} "
                      f"{'static' if v.get('static') else 'dynamic'}"
                      f"{' interp=' + v['pt_interp'] if v.get('pt_interp') else ''}")
    if meta.get("pt_interps_present"):
        print(f"interps:   {meta['pt_interps_present']}")
    print(f"metadata:  {out}")


if __name__ == "__main__":
    main()
