#!/usr/bin/env python3
"""scripts/assemble-android11-rootfs.py — Pure stock Android 11 rootfs builder.

Mission 6-Z305 (user directive): the twoyi Android 8.1 rootfs is a heavily
modified from-source build (fingerprint Android/twoyi_arm64/twoyi_arm64:8.1.0,
static init, custom services) that "bypasses everything to just boot". The
mission now requires PURE UNMODIFIED ROMs — the rootfs itself must be stock.
This tool assembles a container rootfs from an OFFICIAL Google-built AOSP
Android 11 (API 30, arm64-v8a) SDK system image and NOTHING else, with a
zero-modification guarantee over ROM content:

  * every byte of every file is copied verbatim from the official images;
  * every symlink target is preserved BYTE-EXACT as stored in the images;
  * every mode and directory structure is preserved;
  * the ONLY assembly decision is the mount topology: ramdisk overlay at /,
    the super's dynamic partitions at their runtime mount points
    (/system, /system_ext, /product, /vendor) — exactly how init mounts
    them on real hardware. The system image root IS the runtime /
    (system-as-root): its payload lives under system/, and its root
    compatibility links (bin -> /system/bin, etc -> /system/etc) are kept
    exactly as stored.

Provenance is written to ROOTFS_MANIFEST.json (fingerprint, sources,
sha256 of every input) so CI and humans can verify the ROM is stock.

Why a custom ext4/cpio reader:
  * GNU cpio is not available in the build environment and the cpio
    members' symlink targets must be byte-exact;
  * 7-Zip (>= 24.08) is used ONLY for GPT(super) -> dynamic partition
    extraction (it parses Android LP super natively), but its ext4 tree
    extraction REWRITES absolute symlink targets to the extraction
    destination (bin -> /system/bin became
    /home/.../rootfs/system/system/bin during testing), which corrupts
    the ROM. This reader preserves the stored targets exactly.

Usage:
  python3 scripts/assemble-android11-rootfs.py \
      --sdk-zip   /path/to/arm64-v8a-30_rXX.zip \
      --parts-dir /path/to/dir/with/{system,system_ext,product,vendor}.img \
      --out       twoyi-android11-aosp-arm64.tar.gz

  (--parts-dir skips the 7zz super step; without it the tool needs the
   7-Zip binary via --7zz, default "7zz".)
"""
import argparse
import gzip
import hashlib
import json
import os
import shutil
import struct
import subprocess
import sys
import tarfile
import tempfile
import zipfile

SECTOR = 512
EXT2_SUPER_MAGIC = 0xEF53
ROOT_INO = 2

# ---------------------------------------------------------------------------
# cpio newc (SVR4 "-H newc") reader — the emulator ramdisk.img format
# ---------------------------------------------------------------------------

CPIO_NEWC_MAGIC = b"070701"
CPIO_TRAILER = b"TRAILER!!!"


def _pad4(n):
    return (4 - (n % 4)) % 4


def cpio_newc_extract(stream, dest):
    """Extract a newc cpio archive preserving symlink targets byte-exact.

    Returns dict with entry/symlink/hardlink counts."""
    stats = dict(entries=0, symlinks=0, hardlinks=0)
    ino_map = {}  # cpio ino -> first extracted path (hardlink data share)
    while True:
        hdr = stream.read(110)
        if not hdr:
            break
        if hdr[:6] != CPIO_NEWC_MAGIC:
            if hdr.strip(b"\0") == b"":
                continue
            raise SystemExit(f"cpio: bad magic {hdr[:6]!r}")
        f = lambda a, b: int(hdr[a:b], 16)  # noqa: E731
        ino, mode = f(6, 14), f(14, 22)
        nlink, fsize = f(38, 46), f(54, 62)
        namesize = f(94, 102)
        name = stream.read(namesize - 1).decode("utf-8", "replace")
        stream.read(1)  # NUL
        stream.read(_pad4(110 + namesize))
        if name == CPIO_TRAILER.decode():
            break
        data = stream.read(fsize) if fsize else b""
        stream.read(_pad4(fsize))
        if name.startswith("./"):
            name = name[2:]
        stats["entries"] += 1
        out = os.path.join(dest, name)
        ftype = mode & 0o170000
        if ftype == 0o040000:  # dir
            os.makedirs(out, exist_ok=True)
            os.chmod(out, mode & 0o7777)
        elif ftype == 0o120000:  # symlink: data IS the target, byte-exact
            os.makedirs(os.path.dirname(out), exist_ok=True)
            target = data.decode("utf-8", "replace")
            if os.path.lexists(out):
                os.remove(out)
            os.symlink(target, out)
            stats["symlinks"] += 1
        elif ftype == 0o100000:  # regular
            os.makedirs(os.path.dirname(out), exist_ok=True)
            if fsize == 0 and ino in ino_map and nlink > 1:
                src = ino_map[ino]
                if src and os.path.isfile(src):
                    shutil.copy2(src, out)
                    stats["hardlinks"] += 1
                    continue
            with open(out, "wb") as fh:
                fh.write(data)
            os.chmod(out, mode & 0o7777)
            ino_map.setdefault(ino, out)
        else:
            print(f"  cpio: skipping special file {name} (mode {oct(mode)})")
    return stats


# ---------------------------------------------------------------------------
# Minimal read-only ext4 reader — enough for official AOSP images:
# extent trees, 64bit feature, linear + htree directories, fast symlinks.
# ---------------------------------------------------------------------------


class Ext4Error(Exception):
    pass


class Ext4:
    def __init__(self, stream):
        self.f = stream
        self.f.seek(1024)
        sb = self.f.read(1024)
        if struct.unpack_from("<H", sb, 56)[0] != EXT2_SUPER_MAGIC:
            raise Ext4Error("not an ext filesystem")
        self.inodes_count = struct.unpack_from("<I", sb, 0)[0]
        self.first_data_block = struct.unpack_from("<I", sb, 20)[0]
        self.log_block = struct.unpack_from("<I", sb, 24)[0]
        self.block_size = 1024 << self.log_block
        self.inodes_per_group = struct.unpack_from("<I", sb, 40)[0]
        self.inode_size = struct.unpack_from("<H", sb, 88)[0]
        feat_incompat = struct.unpack_from("<I", sb, 96)[0]
        self.incompat = feat_incompat
        self.desc_size = struct.unpack_from("<H", sb, 254)[0] or 32
        self.has_64bit = bool(feat_incompat & 0x80)  # INCOMPAT_64BIT
        self.gd_offset = (self.first_data_block + 1) * self.block_size
        self._inode_cache = {}

    def _group_desc(self, group):
        off = self.gd_offset + group * self.desc_size
        self.f.seek(off)
        d = self.f.read(self.desc_size)
        lo = struct.unpack_from("<I", d, 8)[0]  # bg_inode_table_lo
        hi = 0
        if self.has_64bit and self.desc_size >= 64:
            hi = struct.unpack_from("<I", d, 40)[0]  # bg_inode_table_hi
        return (hi << 32) | lo

    def inode(self, ino):
        if ino in self._inode_cache:
            return self._inode_cache[ino]
        group = (ino - 1) // self.inodes_per_group
        index = (ino - 1) % self.inodes_per_group
        table_block = self._group_desc(group)
        off = table_block * self.block_size + index * self.inode_size
        self.f.seek(off)
        raw = self.f.read(self.inode_size)
        mode = struct.unpack_from("<H", raw, 0)[0]
        size_lo = struct.unpack_from("<I", raw, 4)[0]
        size_hi = struct.unpack_from("<I", raw, 108)[0] if self.inode_size >= 128 else 0
        size = (size_hi << 32) | size_lo
        i_block = raw[40:100]
        flags = struct.unpack_from("<I", raw, 32)[0]
        node = dict(mode=mode, size=size, i_block=i_block, flags=flags)
        self._inode_cache[ino] = node
        return node

    def _extent_blocks(self, inode):
        """Yield (file_block_offset, fs_block_nr, n_blocks)."""
        hdr = inode["i_block"]
        n_entries = struct.unpack_from("<H", hdr, 2)[0]
        depth = struct.unpack_from("<H", hdr, 6)[0]
        stack = [(0, hdr, n_entries, depth)]
        while stack:
            base, node, entries, d = stack.pop()
            for i in range(entries):
                off = 12 + i * 12
                if d == 0:
                    # struct ext4_extent: ee_block(4) ee_len(2) start_hi(2) start_lo(4)
                    ee_block = struct.unpack_from("<I", node, off)[0]
                    ee_len = struct.unpack_from("<H", node, off + 4)[0]
                    start_hi = struct.unpack_from("<H", node, off + 6)[0]
                    start_lo = struct.unpack_from("<I", node, off + 8)[0]
                    start = (start_hi << 32) | start_lo
                    if start:
                        yield (base + ee_block, start, ee_len)
                else:
                    # struct ext4_extent_idx: ei_block(4) leaf_lo(4) leaf_hi(2)
                    ei_block = struct.unpack_from("<I", node, off)[0]
                    leaf_lo = struct.unpack_from("<I", node, off + 4)[0]
                    leaf_hi = struct.unpack_from("<H", node, off + 8)[0]
                    leaf = (leaf_hi << 32) | leaf_lo
                    if not leaf:
                        continue
                    raw = self._read_fs_block(leaf)
                    n = struct.unpack_from("<H", raw, 2)[0]
                    d2 = struct.unpack_from("<H", raw, 6)[0]
                    stack.append((base + ei_block, raw, n, d2))

    def _read_fs_block(self, nr):
        self.f.seek(nr * self.block_size)
        return self.f.read(self.block_size)

    def read_file(self, inode):
        size = inode["size"]
        if inode["mode"] & 0xF000 == 0xA000 and size < 60:
            # fast symlink: target bytes live inside i_block
            return inode["i_block"][:size]
        out = bytearray(size)
        for f_off, blk, nblk in self._extent_blocks(inode):
            for k in range(nblk):
                dst_off = (f_off + k) * self.block_size
                if dst_off >= size:
                    break
                data = self._read_fs_block(blk + k)
                take = min(self.block_size, size - dst_off)
                out[dst_off:dst_off + take] = data[:take]
        return bytes(out[:size])

    def _dirents_in_block(self, raw):
        off = 0
        while off + 8 <= len(raw):
            ino = struct.unpack_from("<I", raw, off)[0]
            rec_len = struct.unpack_from("<H", raw, off + 4)[0]
            if rec_len < 8 or off + rec_len > len(raw):
                break
            name_len = raw[off + 6]
            ftype = raw[off + 7]
            if ino and name_len:
                name = raw[off + 8:off + 8 + name_len]
                yield ino, name.decode("utf-8", "replace"), ftype
            off += rec_len

    def _dx_entries(self, raw, entries_off):
        if entries_off + 4 > len(raw):
            return []
        count = struct.unpack_from("<H", raw, entries_off + 2)[0]
        out = []
        for i in range(count):
            o = entries_off + 4 + i * 8
            if o + 8 > len(raw):
                break
            block = struct.unpack_from("<I", raw, o + 4)[0] & 0x00FFFFFF
            out.append(block)
        return out

    def iter_dir(self, dir_ino):
        """Yield (ino, name, ftype) — htree-safe (dx_root / dx_node walk)."""
        inode = self.inode(dir_ino)
        if not inode["flags"] & 0x1000:  # EXT4_INDEX_FL
            for f_off, blk, nblk in self._extent_blocks(inode):
                for k in range(nblk):
                    yield from self._dirents_in_block(self._read_fs_block(blk + k))
            return
        extent_map = [(f_off + k, blk + k)
                      for f_off, blk, nblk in self._extent_blocks(inode)
                      for k in range(nblk)]
        block_of = dict(extent_map)
        raw0 = self._read_fs_block(block_of[0])
        indirect_levels = raw0[30]  # dx_root_info.indirect_levels (@24+6)
        level0 = self._dx_entries(raw0, 32)
        frontier = [(0, b) for b in level0]
        seen = set()
        while frontier:
            level, nr = frontier.pop()
            if nr in seen:
                continue
            seen.add(nr)
            raw = self._read_fs_block(nr)
            if level < indirect_levels:
                for child in self._dx_entries(raw, 8):
                    frontier.append((level + 1, child))
            else:
                yield from self._dirents_in_block(raw)


def extract_ext4_tree(img_path, dest):
    """Extract an ext4 image into dest, symlink targets byte-exact."""
    fs = Ext4(open(img_path, "rb"))
    stats = dict(files=0, dirs=0, symlinks=0, skipped=0)

    def walk(dir_ino, out_dir):
        os.makedirs(out_dir, exist_ok=True)
        seen = set()
        for ino, name, _ftype in fs.iter_dir(dir_ino):
            if name in (".", "..") or ino == 0 or name in seen:
                continue
            seen.add(name)
            node = fs.inode(ino)
            mode = node["mode"]
            kind = mode & 0o170000
            out = os.path.join(out_dir, name)
            if kind == 0o040000:
                stats["dirs"] += 1
                walk(ino, out)
            elif kind == 0o100000:
                stats["files"] += 1
                os.makedirs(os.path.dirname(out), exist_ok=True)
                with open(out, "wb") as fh:
                    fh.write(fs.read_file(node))
                os.chmod(out, mode & 0o7777)
            elif kind == 0o120000:
                stats["symlinks"] += 1
                target = fs.read_file(node).decode("utf-8", "replace")
                os.makedirs(os.path.dirname(out), exist_ok=True)
                if os.path.lexists(out):
                    os.remove(out)
                os.symlink(target, out)
            else:
                stats["skipped"] += 1
                print(f"  ext4: skip special {out} mode={oct(mode)}")

    walk(ROOT_INO, dest)
    return stats


# ---------------------------------------------------------------------------
# super (LP) -> dynamic partitions, via 7zz >= 24.08 (native LP support)
# ---------------------------------------------------------------------------


def super_to_partitions(super_img, out_dir, seven_zip):
    os.makedirs(out_dir, exist_ok=True)
    r = subprocess.run([seven_zip, "x", "-y", f"-o{out_dir}", super_img],
                       capture_output=True, text=True)
    if r.returncode != 0:
        raise SystemExit(f"7zz failed on super: {r.stderr[-500:]}")
    parts = {}
    for fn in sorted(os.listdir(out_dir)):
        if fn.endswith(".img"):
            parts[os.path.splitext(fn)[0]] = os.path.join(out_dir, fn)
    return parts


# ---------------------------------------------------------------------------
# assembly
# ---------------------------------------------------------------------------

# Runtime mount topology. system-as-root: the system image root mounts at /
# and its payload lives at system/. The other partitions mount directly.
PARTITIONS = ["system", "system_ext", "product", "vendor", "odm"]


def merge_image_root_into_root(tmp_dir, rootfs):
    """system-as-root: merge the system image root INTO the rootfs root.

    Runtime semantics on real hardware: the system image root is the
    runtime /, and the boot ramdisk overlays on top of it — ramdisk
    entries win where both exist, directories merge, the image root
    fills in everything else (its system/ payload, apex/, and the root
    compatibility links bin -> /system/bin etc.)."""
    for child in os.listdir(tmp_dir):
        csrc = os.path.join(tmp_dir, child)
        cdst = os.path.join(rootfs, child)
        if not os.path.lexists(cdst):
            shutil.move(csrc, cdst)
        elif os.path.isdir(csrc) and not os.path.islink(csrc) \
                and os.path.isdir(cdst) and not os.path.islink(cdst):
            merge_tree(csrc, cdst)
        else:
            # ramdisk overlay wins: drop the image-root copy
            if os.path.isdir(csrc) and not os.path.islink(csrc):
                shutil.rmtree(csrc)
            else:
                os.remove(csrc)
    os.rmdir(tmp_dir)


def merge_payload(tmp_dir, rootfs, name):
    """Move the extracted payload of a partition to its mountpoint.

    The system image root pre-creates mountpoint placeholders (dirs or
    compatibility symlinks like vendor -> /vendor). At runtime the real
    mount shadows the placeholder — mirror that: the real payload wins;
    if a real dir already exists, merge children (mount semantics)."""
    dst = os.path.join(rootfs, name)
    if os.path.islink(dst) or (os.path.exists(dst) and not os.path.isdir(dst)):
        os.remove(dst)
    if os.path.isdir(dst):
        for child in os.listdir(tmp_dir):
            csrc = os.path.join(tmp_dir, child)
            cdst = os.path.join(dst, child)
            if os.path.lexists(cdst):
                if os.path.isdir(cdst) and not os.path.islink(cdst) \
                        and os.path.isdir(csrc) and not os.path.islink(csrc):
                    merge_tree(csrc, cdst)
                    continue
                os.remove(cdst)
            shutil.move(csrc, cdst)
        os.rmdir(tmp_dir)
    else:
        shutil.move(tmp_dir, dst)


def merge_tree(src, dst):
    for child in os.listdir(src):
        csrc = os.path.join(src, child)
        cdst = os.path.join(dst, child)
        if os.path.isdir(csrc) and not os.path.islink(csrc):
            if os.path.isdir(cdst) and not os.path.islink(cdst):
                merge_tree(csrc, cdst)
                continue
            shutil.move(csrc, cdst)
        else:
            if os.path.lexists(cdst):
                os.remove(cdst)
            shutil.move(csrc, cdst)
    os.rmdir(src)


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def count_symlinks(root):
    n = 0
    for base, dirs, files in os.walk(root):
        for f in dirs + files:
            if os.path.islink(os.path.join(base, f)):
                n += 1
    return n


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sdk-zip", required=True, help="official SDK sys-img zip")
    ap.add_argument("--parts-dir", help="pre-extracted dynamic partition images")
    ap.add_argument("--7zz", dest="sevenzz", default="7zz")
    ap.add_argument("--out", required=True, help="output tar.gz path")
    ap.add_argument("--work", default=None, help="work dir (default: temp)")
    args = ap.parse_args()

    work = args.work or tempfile.mkdtemp(prefix="a11rootfs.")
    os.makedirs(work, exist_ok=True)
    print(f"work dir: {work}")

    # 1) provenance + ramdisk from the official zip
    zsha = sha256(args.sdk_zip)
    zf = zipfile.ZipFile(args.sdk_zip)
    names = zf.namelist()
    sys_member = next(n for n in names if n.endswith("arm64-v8a/system.img"))
    payload_dir = sys_member.rsplit("/", 1)[0]  # .../arm64-v8a
    print(f"zip sha256: {zsha}")

    ramdisk_path = os.path.join(work, "ramdisk.img")
    with zf.open(f"{payload_dir}/ramdisk.img") as src, open(ramdisk_path, "wb") as dst:
        shutil.copyfileobj(src, dst)

    # 2) dynamic partitions
    if args.parts_dir:
        parts = {os.path.splitext(f)[0]: os.path.join(args.parts_dir, f)
                 for f in os.listdir(args.parts_dir) if f.endswith(".img")}
    else:
        system_path = os.path.join(work, "system.img")
        with zf.open(f"{payload_dir}/system.img") as src, open(system_path, "wb") as dst:
            shutil.copyfileobj(src, dst)
        r = subprocess.run([args.sevenzz, "x", "-y", f"-o{work}", system_path],
                           capture_output=True, text=True)
        if r.returncode != 0:
            raise SystemExit("7zz failed to unpack system.img (GPT): " + r.stderr[-400:])
        cand = [os.path.join(work, f) for f in os.listdir(work)
                if f.endswith(".img") and "super" in f]
        if not cand:
            raise SystemExit("no super.img inside system.img — non-dynamic image?")
        parts = super_to_partitions(cand[0], os.path.join(work, "parts"), args.sevenzz)
    print("partitions:", {k: f"{os.path.getsize(v)/1e6:.0f}MB" for k, v in parts.items()})

    # 3) rootfs assembly — ramdisk overlay at /, partitions at mountpoints
    rootfs = os.path.join(work, "rootfs")
    os.makedirs(rootfs, exist_ok=True)

    print("extracting ramdisk (cpio newc)...")
    with gzip.open(ramdisk_path, "rb") as g:
        stats = cpio_newc_extract(g, rootfs)
    print(f"  ramdisk: {stats}")

    for pname in PARTITIONS:
        key = next((k for k in parts if k == pname), None)
        if key is None:
            print(f"  (no {pname} partition — skipped)")
            continue
        tmp = os.path.join(work, f"x_{pname}")
        print(f"extracting {key} -> {'/' if pname == 'system' else '/' + pname} ...")
        st = extract_ext4_tree(parts[key], tmp)
        print(f"  {key}: {st}")
        if pname == "system":
            # system-as-root: the image root IS the runtime /
            merge_image_root_into_root(tmp, rootfs)
        else:
            merge_payload(tmp, rootfs, pname)

    # 4) structural sanity gate — the full-Android layout the container
    #    expects (mirrors RomManager.isFullAndroidLayout expectations)
    checks = {
        "init": "init",
        "framework.jar": "system/framework/framework.jar",
        "app_process64": "system/bin/app_process64",
        "second-stage init": "system/bin/init",
        "system build.prop": "system/build.prop",
    }
    missing = [n for n, rel in checks.items()
               if not os.path.exists(os.path.join(rootfs, rel))]
    if missing:
        raise SystemExit(f"sanity gate FAILED — missing: {missing}")
    nlinks = count_symlinks(rootfs)
    print(f"sanity gate OK — symlinks: {nlinks}")

    fp = ""
    for cand in ("system/system/build.prop", "system/build.prop"):
        bp = os.path.join(rootfs, cand)
        if os.path.exists(bp):
            for line in open(bp, encoding="utf-8", errors="replace"):
                if line.startswith("ro.system.build.fingerprint="):
                    fp = line.strip().split("=", 1)[1]
                    break
            if fp:
                break

    manifest = {
        "role": "pure-stock-android11-rootfs (zero ROM modifications)",
        "sdk_zip": os.path.basename(args.sdk_zip),
        "sdk_zip_sha256": zsha,
        "partitions": {k: sha256(v) for k, v in parts.items()},
        "fingerprint": fp,
        "symlinks": nlinks,
    }
    mpath = os.path.join(os.path.dirname(os.path.abspath(args.out)),
                         "ROOTFS_MANIFEST.json")
    with open(mpath, "w") as fh:
        json.dump(manifest, fh, indent=2)
    print(f"manifest: {mpath}")
    print(f"fingerprint: {fp}")

    # 5) package — GNU-format tar with REAL symlinks (the app's
    #    RamdiskImporter converts them to .symlink sidecars at import;
    #    kr64's symlinks.rs materializes them at boot)
    print("packing tar.gz (this may take a few minutes)...")
    with tarfile.open(args.out, "w:gz", format=tarfile.GNU_FORMAT) as tf:
        tf.add(rootfs, arcname=".", recursive=True)
    print(f"OUT: {args.out} ({os.path.getsize(args.out)/1e6:.0f} MB)")


if __name__ == "__main__":
    main()
