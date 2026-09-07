// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 6-Z305t: MINIMAL READ-ONLY ext4 reader for APEX payload flattening.
//!
//! # Why this exists (the 6-Z305s decode chain)
//!
//! The stock Android 11 rootfs ships its APEX runtime libraries ONLY as
//! `/system/apex/<name>.apex` ZIP payloads whose `apex_payload.img` is an
//! ext4 filesystem (22 payloads: com.android.art.debug, com.android.i18n,
//! com.android.os.statsd, …). On a REAL device apexd loop-mounts those
//! ext4 images into `/apex/<name>`; rootless twoyi CANNOT mount anything
//! (`open /dev/loop-control: Permission denied` — proven in-run), so the
//! guest's `/apex` tree stays incomplete and every APEX-library consumer
//! (zygote: libnativeloader.so, lmkd: libstatssocket.so, mediaserver/
//! audioserver: libandroidicu.so) dies CANNOT LINK EXECUTABLE at spawn
//! (run 34058989419).
//!
//! THE FIX: read the ext4 payload in pure Rust (std + libc only — crate
//! policy; no loop device, no mount, no FUSE) and materialize the file
//! tree as REAL host directories under `{rootfs}/apex/<name>/` BEFORE the
//! guest boots. Same discipline as the 6-Z218b bootstrap-bionic staging
//! and the 6-Z305r sysctl defaults: the virtual kernel provides the
//! device model (here: a flattened-APEX device) rather than faking
//! per-ROM behavior.
//!
//! # Scope (verified against the real payload images)
//!
//! The AOSP apexer builds `apex_payload.img` with mke2fs defaults for a
//! small read-only image. The statsd payload superblock (decoded from the
//! vendored android11-aosp-arm64-rsr1 corpus): 4096-byte blocks,
//! 256-byte inodes, first_data_block=0, feature_incompat =
//! FILETYPE(0x2) | EXTENTS(0x40) | FLEX_BG(0x200), NO 64BIT, NO META_BG,
//! NO INLINE_DATA. This reader:
//!   * REQUIRES: extents (the only block-mapping implemented),
//!   * ACCEPTS: filetype, flex_bg, sparse_super, large_file, dir_nlink,
//!     extra_isize, large dir variants,
//!   * REJECTS (hard error, caller skips the apex): 64BIT group
//!     descriptors are HANDLED via s_desc_size, but meta_bg, inline_data,
//!     journal-dev, compression and encryption abort the parse — honest
//!     failure, never a partial tree.
//! Journal replay is NOT implemented (payload images are cleanly built
//! and unmounted; s_state != clean is still read-only-parsed — the file
//! DATA is intact for a cleanly-made image).
//!
//! All reads go through [`std::fs::File`] on the payload image (the
//! caller extracts `apex_payload.img` from the .apex ZIP first — see
//! [`crate::apex_extract`] for the ZIP side, which already handles the
//! STORED-entry extraction and the temp-file plumbing).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

// ── ext4 on-disk constants (only what this reader needs) ────────────

const EXT2_SUPER_MAGIC: u16 = 0xEF53;
const EXT4_EXTENT_HEADER_MAGIC: u16 = 0xF30A;
const ROOT_INO: u32 = 2;

// s_feature_incompat flags
const INCOMPAT_FILETYPE: u32 = 0x002;
const INCOMPAT_EXTENTS: u32 = 0x040;
const INCOMPAT_64BIT: u32 = 0x080;
const INCOMPAT_META_BG: u32 = 0x010;
const INCOMPAT_INLINE_DATA: u32 = 0x8000;
const INCOMPAT_JOURNAL_DEV: u32 = 0x008;
const INCOMPAT_COMPRESSION: u32 = 0x001;
const INCOMPAT_ENCRYPT: u32 = 0x1_0000;
// accepted-but-ignored incompat bits (dirdata/csum_seed/largesx … are
// metadata-layout tweaks that don't change the read path for extents)
const INCOMPAT_IGNORED: u32 =
    0x400 | 0x1000 | 0x2000 | 0x4000 | 0x2_0000 | 0x4_0000 | 0x8_0000 | 0x10_0000 | 0x20_0000;

// inode i_flags
const EXT4_EXTENTS_FL: u32 = 0x8_0000;
// inode i_mode file-type bits (also used for dirent file_type mapping)
const S_IFMT: u16 = 0xF000;
const S_IFDIR: u16 = 0x4000;
const S_IFLNK: u16 = 0xA000;

/// Read errors. `Display`-able for the honest-failure log lines.
#[derive(Debug)]
pub enum Ext4Error {
    Io(std::io::Error),
    /// (what, detail) — every variant is a HARD skip for the caller.
    Bad(&'static str, String),
}

impl std::fmt::Display for Ext4Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ext4Error::Io(e) => write!(f, "io error: {}", e),
            Ext4Error::Bad(what, d) => write!(f, "bad ext4 image ({}): {}", what, d),
        }
    }
}

impl From<std::io::Error> for Ext4Error {
    fn from(e: std::io::Error) -> Self {
        Ext4Error::Io(e)
    }
}

/// Parsed superblock (the fields this reader acts on).
struct Superblock {
    block_size: u32,
    blocks_count: u64,
    first_data_block: u32,
    inodes_per_group: u32,
    inode_size: u16,
    desc_size: u16,
    groups: u32,
}

impl Superblock {
    fn parse(img: &mut File) -> Result<Self, Ext4Error> {
        let mut sb = [0u8; 1024];
        img.seek(SeekFrom::Start(1024))?;
        img.read_exact(&mut sb)?;
        let rd16 = |off: usize| u16::from_le_bytes([sb[off], sb[off + 1]]);
        let rd32 =
            |off: usize| u32::from_le_bytes([sb[off], sb[off + 1], sb[off + 2], sb[off + 3]]);
        if rd16(0x38) != EXT2_SUPER_MAGIC {
            return Err(Ext4Error::Bad(
                "magic",
                format!(
                    "{:#06x} != {:#06x} at offset 1024+56",
                    rd16(0x38),
                    EXT2_SUPER_MAGIC
                ),
            ));
        }
        let log_block_size = rd32(0x18);
        if log_block_size > 6 {
            return Err(Ext4Error::Bad(
                "log_block_size",
                format!("{}", log_block_size),
            ));
        }
        let block_size = 1024u32 << log_block_size;
        let incompat = rd32(0x60);
        let rejected = incompat
            & (INCOMPAT_META_BG
                | INCOMPAT_INLINE_DATA
                | INCOMPAT_JOURNAL_DEV
                | INCOMPAT_COMPRESSION
                | INCOMPAT_ENCRYPT);
        if rejected != 0 {
            return Err(Ext4Error::Bad(
                "feature_incompat",
                format!("{:#x} contains unsupported bits {:#x}", incompat, rejected),
            ));
        }
        if incompat & INCOMPAT_EXTENTS == 0 {
            // Some payloads could theoretically be built without extents
            // (tr indirect blocks) — the apexer never does; refuse
            // instead of implementing a second mapper blind.
            return Err(Ext4Error::Bad(
                "feature_incompat",
                format!("{:#x} lacks EXTENTS", incompat),
            ));
        }
        let _ = INCOMPAT_FILETYPE;
        let _ = INCOMPAT_64BIT;
        let _ = INCOMPAT_IGNORED;
        let first_data_block = rd32(0x14);
        let blocks_per_group = rd32(0x20);
        let inodes_per_group = rd32(0x28);
        if blocks_per_group == 0 || inodes_per_group == 0 {
            return Err(Ext4Error::Bad(
                "group geometry",
                "zero blocks/inodes per group".into(),
            ));
        }
        if inodes_per_group > blocks_per_group * (block_size / 128) {
            return Err(Ext4Error::Bad(
                "group geometry",
                format!(
                    "inodes_per_group {} exceeds the inode-table capacity of blocks_per_group {}",
                    inodes_per_group, blocks_per_group
                ),
            ));
        }
        let blocks_count = rd32(0x4) as u64; // 64BIT images >4G blocks: reject via hi check below
        if incompat & INCOMPAT_64BIT != 0 {
            // blocks_count_hi @0x64 — handled so the GDT sizing is right,
            // but payloads this big are out of scope for an apex flatten.
            let hi = rd32(0x64) as u64;
            if hi != 0 {
                return Err(Ext4Error::Bad("blocks_count_hi", format!("{}", hi)));
            }
        }
        let inode_size = if rd32(0x4C) /* s_rev_level */ >= 1 {
            rd16(0x58)
        } else {
            128
        };
        if inode_size < 128 {
            return Err(Ext4Error::Bad("inode_size", format!("{}", inode_size)));
        }
        // s_desc_size @0xFE (only meaningful with 64BIT); default 32.
        let desc_size = if incompat & INCOMPAT_64BIT != 0 {
            let d = rd16(0xFE);
            if d < 32 {
                return Err(Ext4Error::Bad("desc_size", format!("{}", d)));
            }
            d
        } else {
            32
        };
        let groups = ((blocks_count - first_data_block as u64) + blocks_per_group as u64 - 1)
            / blocks_per_group as u64;
        Ok(Superblock {
            block_size,
            blocks_count,
            first_data_block,
            inodes_per_group,
            inode_size,
            desc_size,
            groups: groups.max(1) as u32,
        })
    }
}

/// An opened, read-only ext4 image.
pub struct Ext4Image {
    file: File,
    sb: Superblock,
    /// Group descriptor table bytes (groups × desc_size).
    gdt: Vec<u8>,
}

impl std::fmt::Debug for Ext4Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ext4Image")
            .field("block_size", &self.sb.block_size)
            .field("blocks_count", &self.sb.blocks_count)
            .field("groups", &self.sb.groups)
            .finish()
    }
}

/// One directory entry (name + target inode + file-type nibble).
pub struct DirEntry {
    pub ino: u32,
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
}

/// Parsed inode (the fields this reader acts on).
struct Inode {
    mode: u16,
    size: u64,
    flags: u32,
    /// Raw i_block[15] area (60 bytes): extent header or fast-symlink target.
    i_block: [u8; 60],
}

impl Ext4Image {
    /// Open + validate the image (superblock + GDT read).
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Ext4Error> {
        let mut file = File::open(path)?;
        let sb = Superblock::parse(&mut file)?;
        // Group descriptor table: starts at block first_data_block+1
        // (block 0 IS the superblock for 1KB images; for ≥2KB images
        // first_data_block is 0 and block 1 holds the GDT — both covered
        // by the same formula).
        let gdt_block = sb.first_data_block + 1;
        let gdt_len = sb.groups as usize * sb.desc_size as usize;
        let mut gdt = vec![0u8; gdt_len];
        file.seek(SeekFrom::Start(gdt_block as u64 * sb.block_size as u64))?;
        file.read_exact(&mut gdt)?;
        Ok(Ext4Image { file, sb, gdt })
    }

    fn rd16(b: &[u8], off: usize) -> u16 {
        u16::from_le_bytes([b[off], b[off + 1]])
    }
    fn rd32(b: &[u8], off: usize) -> u32 {
        u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
    }

    /// Locate inode `ino` (1-based) and read its fixed-size fields.
    fn read_inode(&mut self, ino: u32) -> Result<Inode, Ext4Error> {
        if ino == 0 || ino > self.sb.inodes_per_group * self.sb.groups {
            return Err(Ext4Error::Bad("inode", format!("{} out of range", ino)));
        }
        let group = (ino - 1) / self.sb.inodes_per_group;
        let index = (ino - 1) % self.sb.inodes_per_group;
        let d_off = group as usize * self.sb.desc_size as usize;
        let table_block = Self::rd32(&self.gdt, d_off + 8) as u64; // bg_inode_table_lo
        let mut raw = vec![0u8; self.sb.inode_size as usize];
        self.file.seek(SeekFrom::Start(
            table_block * self.sb.block_size as u64 + index as u64 * self.sb.inode_size as u64,
        ))?;
        self.file.read_exact(&mut raw)?;
        Ok(Inode {
            mode: Self::rd16(&raw, 0),
            size: Self::rd32(&raw, 4) as u64 | ((Self::rd32(&raw, 108) as u64) << 32),
            flags: Self::rd32(&raw, 32),
            i_block: {
                let mut b = [0u8; 60];
                b.copy_from_slice(&raw[40..100]);
                b
            },
        })
    }

    /// Map ONE logical block through the extent tree rooted at `node`.
    /// `depth` recursion is bounded by the on-disk tree depth (≤5).
    fn extent_lookup(
        &mut self,
        node: &[u8],
        depth_max: u32,
        logical: u32,
    ) -> Result<Option<u64>, Ext4Error> {
        if Self::rd16(node, 0) != EXT4_EXTENT_HEADER_MAGIC {
            return Err(Ext4Error::Bad(
                "extent header",
                format!("magic {:#06x}", Self::rd16(node, 0)),
            ));
        }
        let entries = Self::rd16(node, 2) as usize;
        let depth = Self::rd16(node, 6) as u32;
        if depth > depth_max {
            return Err(Ext4Error::Bad("extent depth", format!("{}", depth)));
        }
        for i in 0..entries {
            let e = 12 + i * 12;
            if depth == 0 {
                // leaf: ee_block u32, ee_len u16, ee_start_hi u16, ee_start_lo u32
                let blk = Self::rd32(node, e);
                let len = Self::rd16(node, e + 4) as u32;
                let start_hi = Self::rd16(node, e + 6) as u64;
                let start_lo = Self::rd32(node, e + 8) as u64;
                let len = if len > 32768 { len - 32768 } else { len }; // unwritten bit
                if logical >= blk && logical < blk + len {
                    let phys = (start_hi << 32) | (start_lo + (logical - blk) as u64);
                    return Ok(Some(phys));
                }
            } else {
                // idx: ee_block u32, leaf_lo u32, leaf_hi u16
                let blk = Self::rd32(node, e);
                if logical >= blk {
                    let leaf_hi = Self::rd16(node, e + 6) as u64;
                    let leaf_lo = Self::rd32(node, e + 4) as u64;
                    let leaf = leaf_hi << 32 | leaf_lo;
                    let mut child = [0u8; 4096];
                    if self.sb.block_size as usize > child.len() {
                        return Err(Ext4Error::Bad(
                            "block_size",
                            "too large for extent node".into(),
                        ));
                    }
                    self.file
                        .seek(SeekFrom::Start(leaf * self.sb.block_size as u64))?;
                    self.file
                        .read_exact(&mut child[..self.sb.block_size as usize])?;
                    if let Some(hit) = self.extent_lookup(
                        &child[..self.sb.block_size as usize],
                        depth - 1,
                        logical,
                    )? {
                        return Ok(Some(hit));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Read the full contents of a file inode.
    fn read_file_data(&mut self, ino: &Inode) -> Result<Vec<u8>, Ext4Error> {
        if ino.flags & EXT4_EXTENTS_FL == 0 {
            return Err(Ext4Error::Bad(
                "file",
                "no EXTENTS_FL (unsupported layout)".into(),
            ));
        }
        let bs = self.sb.block_size as usize;
        let n_blocks = (ino.size as usize).div_ceil(bs);
        let mut out = Vec::with_capacity(ino.size as usize);
        for logical in 0..n_blocks as u32 {
            let phys = self.extent_lookup(&ino.i_block, 5, logical)?;
            let mut block = vec![0u8; bs];
            if let Some(p) = phys {
                self.file
                    .seek(SeekFrom::Start(p * self.sb.block_size as u64))?;
                self.file.read_exact(&mut block)?;
            }
            let take = ((ino.size as usize) - out.len()).min(bs);
            out.extend_from_slice(&block[..take]);
        }
        Ok(out)
    }

    /// Read a directory inode's entries (`ext4_dir_entry_2` stream).
    fn read_dir_entries(&mut self, ino: &Inode) -> Result<Vec<(u32, String, u8)>, Ext4Error> {
        let data = self.read_file_data(ino)?;
        let mut out = Vec::new();
        let mut off = 0usize;
        while off + 8 <= data.len() {
            let ino = Self::rd32(&data, off);
            let rec_len = Self::rd16(&data, off + 4) as usize;
            // ext4_dir_entry_2: inode@0, rec_len@4, name_len@6, file_type@7
            let name_len = data[off + 6] as usize;
            let ftype = data[off + 7];
            if rec_len < 8 || off + rec_len > data.len() {
                break; // corrupt tail — stop cleanly
            }
            if ino != 0 && name_len > 0 && off + 8 + name_len <= off + rec_len {
                let name = String::from_utf8_lossy(&data[off + 8..off + 8 + name_len]).into_owned();
                out.push((ino, name, ftype));
            }
            off += rec_len;
        }
        Ok(out)
    }

    /// List a directory (`/`-rooted path inside the image).
    pub fn list_dir(&mut self, path: &str) -> Result<Vec<DirEntry>, Ext4Error> {
        let (ino, _leftover) = self.walk(path)?;
        let entries = self.read_dir_entries(&ino)?;
        Ok(entries
            .into_iter()
            .map(|(ino, name, ftype)| DirEntry {
                ino,
                name,
                is_dir: ftype == 2,
                is_symlink: ftype == 7,
            })
            .collect())
    }

    /// Read a file (`/`-rooted path) fully. Symlinks are rejected (the
    /// flattening layer materializes them without reading a target file).
    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>, Ext4Error> {
        let (ino, leftover) = self.walk(path)?;
        if !leftover.is_empty() {
            return Err(Ext4Error::Bad("path", format!("{}: not found", path)));
        }
        if ino.mode & S_IFMT == S_IFLNK {
            return Err(Ext4Error::Bad("path", format!("{}: is a symlink", path)));
        }
        self.read_file_data(&ino)
    }

    /// Walk `/`-rooted `path`; returns the inode of the deepest resolved
    /// component plus the unresolved remainder (empty = full match).
    /// Symlink components are NOT followed (honest error) — the
    /// flattening layer reads symlink inodes directly and materializes
    /// them as guest symlinks (`extract_tree`), which is the only
    /// consumer of this module.
    fn walk(&mut self, path: &str) -> Result<(Inode, String), Ext4Error> {
        let mut cur = self.read_inode(ROOT_INO)?;
        let mut rest = path.trim_start_matches('/').to_string();
        while !rest.is_empty() {
            if cur.mode & S_IFMT != S_IFDIR {
                return Err(Ext4Error::Bad("path", format!("{}: not a directory", path)));
            }
            let (head, tail) = match rest.find('/') {
                Some(i) => (&rest[..i], rest[i + 1..].to_string()),
                None => (rest.as_str(), String::new()),
            };
            if head.is_empty() {
                rest = tail;
                continue;
            }
            if head == "." {
                rest = tail;
                continue;
            }
            if head == ".." {
                return Err(Ext4Error::Bad("path", ".. not supported".into()));
            }
            let entries = self.read_dir_entries(&cur)?;
            match entries.iter().find(|(_, n, _)| n == head) {
                None => return Ok((cur, rest)), // unresolved remainder
                Some(&(ino, _, _)) => {
                    cur = self.read_inode(ino)?;
                    rest = tail;
                }
            }
        }
        Ok((cur, rest))
    }

    /// Extract the whole tree under `src` (e.g. "/") into `dst` on the
    /// host, recreating dirs/files and fast symlinks. Returns the count
    /// of materialized entries. Skips nothing silently: hard failures
    /// propagate (the caller logs + skips the whole apex).
    pub fn extract_tree(&mut self, src: &str, dst: &str) -> Result<usize, Ext4Error> {
        let mut n = 0usize;
        let entries = self.list_dir(src)?;
        for e in entries {
            if e.name == "." || e.name == ".." {
                continue;
            }
            let src_path = format!("{}/{}", src.trim_end_matches('/'), e.name);
            let dst_path = format!("{}/{}", dst.trim_end_matches('/'), e.name);
            let src_ino = self.walk(&src_path)?.0;
            if src_ino.mode & S_IFMT == S_IFLNK {
                let target_len = (src_ino.size as usize).min(59);
                let target = String::from_utf8_lossy(&src_ino.i_block[..target_len]).into_owned();
                let _ = std::fs::remove_file(&dst_path);
                #[cfg(unix)]
                std::os::unix::fs::symlink(&target, &dst_path).map_err(Ext4Error::Io)?;
                n += 1;
            } else if src_ino.mode & S_IFMT == S_IFDIR {
                std::fs::create_dir_all(&dst_path).map_err(Ext4Error::Io)?;
                n += 1 + self.extract_tree(&src_path, &dst_path)?;
            } else {
                let data = self.read_file_data(&src_ino)?;
                if let Some(parent) = Path::new(&dst_path).parent() {
                    std::fs::create_dir_all(parent).map_err(Ext4Error::Io)?;
                }
                std::fs::write(&dst_path, &data).map_err(Ext4Error::Io)?;
                n += 1;
            }
        }
        Ok(n)
    }
}

// ── tests ───────────────────────────────────────────────────────────
#[cfg(test)]
#[allow(clippy::identity_op)]
mod tests {
    use super::*;

    /// A hand-built minimal ext4 image exercising the exact feature set
    /// the apexer produces: 4096-byte blocks, 256-byte inodes, extents,
    /// filetype, flex_bg, no 64BIT. Layout:
    ///   block 0  (4K)  : zeros + superblock at offset 1024
    ///   block 1  (4K)  : GDT (1 group, 32-byte desc)
    ///   block 2  (4K)  : inode table (2 inodes used: root=2, file=11)
    ///   block 3  (4K)  : root dir data (".", "..", "hello")
    ///   block 4  (4K)  : "hello" file data ("hi apex\n")
    ///   block 5  (4K)  : symlink inode target test data (dir "d")
    fn block_at(n: usize) -> usize {
        n * 4096
    }

    fn extent_leaf(block_logical: u32, block_phys: u32) -> [u8; 12] {
        let mut e = [0u8; 12];
        e[0..4].copy_from_slice(&block_logical.to_le_bytes());
        e[4..6].copy_from_slice(&1u16.to_le_bytes()); // len 1
        e[6..8].copy_from_slice(&0u16.to_le_bytes()); // start hi
        e[8..12].copy_from_slice(&block_phys.to_le_bytes());
        e
    }

    fn extent_header(entries: u16, depth: u16) -> [u8; 12] {
        let mut h = [0u8; 12];
        h[0..2].copy_from_slice(&0xF30Au16.to_le_bytes());
        h[2..4].copy_from_slice(&entries.to_le_bytes());
        h[4..6].copy_from_slice(&4u16.to_le_bytes()); // max
        h[6..8].copy_from_slice(&depth.to_le_bytes());
        h
    }

    fn dirent(ino: u32, name: &str, ftype: u8) -> Vec<u8> {
        // rec_len: 8-aligned covering name; last entry gets the block rest.
        let name_b = name.as_bytes();
        let mut rl = (8 + name_b.len() + 3) & !3;
        if rl < 8 {
            rl = 8;
        }
        let mut d = vec![0u8; rl];
        d[0..4].copy_from_slice(&ino.to_le_bytes());
        d[6] = name_b.len() as u8;
        d[7] = ftype;
        d[8..8 + name_b.len()].copy_from_slice(name_b);
        d[4..6].copy_from_slice(&(rl as u16).to_le_bytes());
        d
    }

    fn build_test_image() -> Vec<u8> {
        let mut img = vec![0u8; 6 * 4096];
        let bs = 4096usize;
        // ── superblock @1024 ──
        let sb = 1024usize;
        img[sb + 0x00..sb + 0x04].copy_from_slice(&32u32.to_le_bytes()); // inodes_count
        img[sb + 0x04..sb + 0x08].copy_from_slice(&6u32.to_le_bytes()); // blocks_count
        img[sb + 0x14..sb + 0x18].copy_from_slice(&0u32.to_le_bytes()); // first_data_block
        img[sb + 0x18..sb + 0x1C].copy_from_slice(&2u32.to_le_bytes()); // log_block_size (4096)
        img[sb + 0x20..sb + 0x24].copy_from_slice(&32768u32.to_le_bytes()); // blocks_per_group
        img[sb + 0x28..sb + 0x2C].copy_from_slice(&32u32.to_le_bytes()); // inodes_per_group
        img[sb + 0x38..sb + 0x3A].copy_from_slice(&0xEF53u16.to_le_bytes()); // magic
        img[sb + 0x4C..sb + 0x50].copy_from_slice(&1u32.to_le_bytes()); // rev dynamic
        img[sb + 0x54..sb + 0x58].copy_from_slice(&11u32.to_le_bytes()); // first_ino
        img[sb + 0x58..sb + 0x5A].copy_from_slice(&256u16.to_le_bytes()); // inode_size
        img[sb + 0x60..sb + 0x64].copy_from_slice(&0x242u32.to_le_bytes()); // incompat: FILETYPE|EXTENTS|FLEX_BG
                                                                            // ── GDT @block 1: one 32-byte desc; inode table at block 2 ──
        let gdt = block_at(1);
        img[gdt + 8..gdt + 12].copy_from_slice(&2u32.to_le_bytes()); // bg_inode_table_lo
                                                                     // ── inode table @block 2: inode N at (N-1)*256 ──
        let it = block_at(2);
        // root inode (ino 2, index 1): dir mode, size 4096, extents
        let root = it + 256;
        img[root..root + 2].copy_from_slice(&0x41EDu16.to_le_bytes()); // S_IFDIR|0755
        img[root + 4..root + 8].copy_from_slice(&4096u32.to_le_bytes()); // size
        img[root + 32..root + 36].copy_from_slice(&0x80000u32.to_le_bytes()); // EXTENTS_FL
                                                                              // extent leaf: 1 entry → block 3
        let eh = extent_header(1, 0);
        img[root + 40..root + 52].copy_from_slice(&eh);
        let el = extent_leaf(0, 3);
        img[root + 52..root + 64].copy_from_slice(&el);
        // file inode (ino 11 = first_ino, index 10): regular, size 9
        let f = it + 10 * 256;
        img[f..f + 2].copy_from_slice(&0x81A4u16.to_le_bytes()); // S_IFREG|0644
        img[f + 4..f + 8].copy_from_slice(&8u32.to_le_bytes());
        img[f + 32..f + 36].copy_from_slice(&0x80000u32.to_le_bytes());
        let eh2 = extent_header(1, 0);
        img[f + 40..f + 52].copy_from_slice(&eh2);
        let el2 = extent_leaf(0, 4);
        img[f + 52..f + 64].copy_from_slice(&el2);
        // symlink inode (ino 12, index 11): fast symlink "bionic"
        let sl = it + 11 * 256;
        img[sl..sl + 2].copy_from_slice(&0xA1FFu16.to_le_bytes()); // S_IFLNK|0777
        img[sl + 4..sl + 8].copy_from_slice(&6u32.to_le_bytes()); // size 6
        img[sl + 40..sl + 46].copy_from_slice(b"bionic");
        // dir inode (ino 13, index 12): empty-ish dir → block 5
        let dd = it + 12 * 256;
        img[dd..dd + 2].copy_from_slice(&0x41EDu16.to_le_bytes());
        img[dd + 4..dd + 8].copy_from_slice(&4096u32.to_le_bytes());
        img[dd + 32..dd + 36].copy_from_slice(&0x80000u32.to_le_bytes());
        let eh3 = extent_header(1, 0);
        img[dd + 40..dd + 52].copy_from_slice(&eh3);
        let el3 = extent_leaf(0, 5);
        img[dd + 52..dd + 64].copy_from_slice(&el3);
        // ── root dir data @block 3: ".", "..", "hello", "d", "link" ──
        let mut dirdata = Vec::new();
        dirdata.extend_from_slice(&dirent(2, ".", 2));
        dirdata.extend_from_slice(&dirent(2, "..", 2));
        dirdata.extend_from_slice(&dirent(11, "hello", 1));
        dirdata.extend_from_slice(&dirent(13, "d", 2));
        dirdata.extend_from_slice(&dirent(12, "link", 7));
        dirdata.resize(bs, 0); // pad (last entry rec_len need not cover)
        let d3 = block_at(3);
        img[d3..d3 + bs].copy_from_slice(&dirdata);
        // ── file data @block 4 ──
        let f4 = block_at(4);
        img[f4..f4 + 8].copy_from_slice(b"hi apex\n");
        // ── dir "d" data @block 5: "." ".." ──
        let mut d2 = Vec::new();
        d2.extend_from_slice(&dirent(13, ".", 2));
        d2.extend_from_slice(&dirent(2, "..", 2));
        d2.resize(bs, 0);
        let d5 = block_at(5);
        img[d5..d5 + bs].copy_from_slice(&d2);
        img
    }

    fn open_img_named(name: &str, bytes: Vec<u8>) -> Ext4Image {
        let dir =
            std::env::temp_dir().join(format!("twoyi-apexfs-{}-{}", std::process::id(), name));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("test.img");
        std::fs::write(&p, &bytes).unwrap();
        let r = Ext4Image::open(&p).unwrap();
        // The open fd survives unlink on Unix — reads continue from the
        // already-open file description. Keep the file around anyway so
        // parallel tests can never race on a recreated path.
        r
    }

    fn open_img(bytes: Vec<u8>) -> Ext4Image {
        // Unique per caller: tests run in parallel within one process.
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        open_img_named(&format!("s{}", n), bytes)
    }

    #[test]
    fn z305t_list_and_read_file() {
        let mut fs = open_img(build_test_image());
        let entries = fs.list_dir("/").unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        eprintln!("z305t DEBUG names={:?} count={}", names, entries.len());
        let ib = build_test_image();
        let it = 2 * 4096usize;
        let root = it + 256;
        eprintln!(
            "z305t DEBUG root mode={:04x} size={} flags={:08x} eh={:02x?} leaf={:02x?}",
            u16::from_le_bytes([ib[root], ib[root + 1]]),
            u32::from_le_bytes([ib[root + 4], ib[root + 5], ib[root + 6], ib[root + 7]]),
            u32::from_le_bytes([ib[root + 32], ib[root + 33], ib[root + 34], ib[root + 35]]),
            &ib[root + 40..root + 52],
            &ib[root + 52..root + 64]
        );
        let d3 = 3 * 4096;
        eprintln!("z305t DEBUG dirdata[0..24]={:02x?}", &ib[d3..d3 + 24]);
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"d"));
        assert!(names.contains(&"link"));
        let data = fs.read_file("/hello").unwrap();
        assert_eq!(data, b"hi apex\n");
    }

    #[test]
    fn z305t_missing_path_reports_remainder() {
        let mut fs = open_img(build_test_image());
        let e = fs.read_file("/nope").unwrap_err();
        assert!(matches!(e, Ext4Error::Bad("path", _)), "got {:?}", e);
        // nested miss: /d exists but /d/nope does not
        let e = fs.read_file("/d/nope").unwrap_err();
        assert!(matches!(e, Ext4Error::Bad("path", _)), "got {:?}", e);
    }

    #[test]
    fn z305t_rejects_non_extents_images() {
        let mut img = build_test_image();
        let sb = 1024;
        let cur = u32::from_le_bytes([
            img[sb + 0x60],
            img[sb + 0x61],
            img[sb + 0x62],
            img[sb + 0x63],
        ]);
        let stripped = cur & !INCOMPAT_EXTENTS;
        img[sb + 0x60..sb + 0x64].copy_from_slice(&stripped.to_le_bytes());
        let dir = std::env::temp_dir().join(format!("twoyi-apexfs-neg{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("noext.img");
        std::fs::write(&p, &img).unwrap();
        let err = Ext4Image::open(&p).unwrap_err();
        assert!(
            matches!(err, Ext4Error::Bad("feature_incompat", _)),
            "got {:?}",
            err
        );
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn z305t_rejects_garbage_magic() {
        let mut img = build_test_image();
        img[1024 + 0x38] = 0;
        img[1024 + 0x39] = 0;
        let dir = std::env::temp_dir().join(format!("twoyi-apexfs-magic{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("magic.img");
        std::fs::write(&p, &img).unwrap();
        let err = Ext4Image::open(&p).unwrap_err();
        assert!(matches!(err, Ext4Error::Bad("magic", _)), "got {:?}", err);
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn z305t_extract_tree_materializes_guest_tree() {
        let mut fs = open_img(build_test_image());
        let dir = std::env::temp_dir().join(format!("twoyi-apexfs-x{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let n = fs.extract_tree("/", dir.to_str().unwrap()).unwrap();
        // hello + d + d's (nothing) + link = 3 materialized entries
        assert_eq!(n, 3);
        assert_eq!(std::fs::read(dir.join("hello")).unwrap(), b"hi apex\n");
        let link = std::fs::read_link(dir.join("link")).unwrap();
        assert_eq!(link.to_str().unwrap(), "bionic");
        assert!(dir.join("d").is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Smoke test against a REAL apex payload image (not run on CI —
    /// set TWOYI_EXT4_TEST_IMAGE to an extracted apex_payload.img).
    #[test]
    fn z305t_real_payload_smoke() {
        let Ok(p) = std::env::var("TWOYI_EXT4_TEST_IMAGE") else {
            return;
        };
        let mut fs = match Ext4Image::open(&p) {
            Ok(f) => f,
            Err(e) => panic!("real payload failed to open: {}", e),
        };
        let entries = fs.list_dir("/").unwrap();
        assert!(!entries.is_empty(), "real payload root must list");
        eprintln!("z305t smoke: {} root entries in {}", entries.len(), p);
    }
}
