//! Virtual Filesystem layer for kr64.
//!
//! Replaces the ad-hoc `translate_path()` in ptrace_emu.rs with a unified
//! path-resolution layer. Both TWRP and the Android guest boot path use the
//! same Vfs; only the populated entries differ.
//!
//! Design (from worklog 1-B Task 3):
//! - `VfsNode` enum: how a guest path is satisfied
//! - `Vfs` struct: owns the path → VfsNode map
//! - `Vfs::resolve(guest_path) -> VfsNode` replaces translate_path()

use std::collections::HashMap;
use std::path::PathBuf;

/// How a guest path is resolved by the VFS.
pub enum VfsNode {
    /// Defer to the real host kernel at this (translated) path.
    HostKernel(PathBuf),
    /// A real file/dir inside the rootfs sandbox (the existing behavior).
    RootfsFile(PathBuf),
    /// A synthetic file with fixed bytes (e.g. /proc/cmdline).
    Synthetic(Vec<u8>),
    /// A synthetic directory listing.
    SyntheticDir(Vec<VfsDirEntry>),
    /// A dynamically-generated file (regenerated on each read).
    Dynamic(Box<dyn Fn() -> Vec<u8> + Send + Sync>),
    /// Explicitly absent — return ENOENT.
    Absent,
}

impl std::fmt::Debug for VfsNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VfsNode::HostKernel(p) => f.debug_tuple("HostKernel").field(p).finish(),
            VfsNode::RootfsFile(p) => f.debug_tuple("RootfsFile").field(p).finish(),
            VfsNode::Synthetic(bytes) => f
                .debug_tuple("Synthetic")
                .field(&format!("{} bytes", bytes.len()))
                .finish(),
            VfsNode::SyntheticDir(entries) => f
                .debug_tuple("SyntheticDir")
                .field(&format!("{} entries", entries.len()))
                .finish(),
            VfsNode::Dynamic(_) => f.debug_tuple("Dynamic").field(&"<closure>").finish(),
            VfsNode::Absent => write!(f, "Absent"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VfsDirEntry {
    pub name: String,
    pub is_dir: bool,
}

/// The Virtual Filesystem. Owns the guest-path → resolution map.
///
/// NOTE: this is the FIRST incremental slice. It starts by providing the
/// `/dev/__properties__/properties_serial` Dynamic node (the proper fix for
/// the find_property binary-patch suppressed crash — see worklog 1-A F.1).
/// The existing `translate_path()` callsites are NOT migrated in this PR —
/// that migration is a follow-up. This slice only:
///   (a) creates the module,
///   (b) implements the property-area Dynamic node,
///   (c) wires it into the SIGSYS handler so property reads hit it,
///   (d) removes the find_property binary patch.
pub struct Vfs {
    entries: HashMap<String, VfsNode>,
}

impl Vfs {
    /// Create a Vfs pre-populated for TWRP-mode boot.
    pub fn new_twrp() -> Self {
        let mut entries = HashMap::new();
        // The property area — a minimal valid __system_property_area__ that
        // makes find_property() iterate over 0 properties and return NULL
        // for any lookup. This is the SAME observable behavior as the old
        // binary patch, but achieved through the VFS instead of binary
        // mutation. Foundation for serving real property values later.
        entries.insert(
            "/dev/__properties__/properties_serial".to_string(),
            VfsNode::Dynamic(Box::new(make_minimal_property_area)),
        );
        // Also expose the parent dir so readdir works.
        entries.insert(
            "/dev/__properties__".to_string(),
            VfsNode::SyntheticDir(vec![VfsDirEntry {
                name: "properties_serial".to_string(),
                is_dir: false,
            }]),
        );
        Vfs { entries }
    }

    /// Resolve a guest path to a VfsNode.
    pub fn resolve(&self, guest_path: &str) -> Option<&VfsNode> {
        // Exact match first, then longest-prefix directory match.
        if let Some(node) = self.entries.get(guest_path) {
            return Some(node);
        }
        // (Directory entries are matched by exact key for now; a more
        //  sophisticated prefix trie can be added later if needed.)
        None
    }

    /// Check if a path is a known VFS synthetic (for the SIGSYS handler to
    /// decide whether to short-circuit an open/openat).
    pub fn is_synthetic(&self, guest_path: &str) -> bool {
        self.entries.contains_key(guest_path)
    }
}

/// Build a minimal valid AOSP `__system_property_area__` header.
///
/// Layout (from bionic/libc/include/sys/_system_properties.h):
///   struct prop_area {
///     unsigned bytes_used;       // 4 — payload bytes used
///     unsigned volatile serial;  // 4 — increment on write (0 = stable)
///     unsigned magic;            // 4 = 0x504f5250 ("PROP")
///     unsigned version;          // 4 = PROP_AREA_VERSION (1)
///     unsigned reserved[28];    // 112
///     char data[];               // payload: prop_info structs (empty here)
///   };
/// Total header = 128 bytes. Minimal area = 128 bytes (empty data).
///
/// This makes find_property() see a valid area with 0 properties → returns
/// NULL for every lookup, which is what TWRP init tolerates (it checks for
/// NULL and falls back). Same behavior as the old binary patch, no mutation.
fn make_minimal_property_area() -> Vec<u8> {
    const PROP_AREA_MAGIC: u32 = 0x504f5250; // "PROP" little-endian
    const PROP_AREA_VERSION: u32 = 1;
    let mut buf = Vec::with_capacity(128);
    // bytes_used: 0 (no properties)
    buf.extend_from_slice(&0u32.to_le_bytes());
    // serial: 0 (stable, no concurrent writes)
    buf.extend_from_slice(&0u32.to_le_bytes());
    // magic
    buf.extend_from_slice(&PROP_AREA_MAGIC.to_le_bytes());
    // version
    buf.extend_from_slice(&PROP_AREA_VERSION.to_le_bytes());
    // reserved[28] = 112 bytes of zero
    buf.extend_from_slice(&[0u8; 112]);
    assert_eq!(
        buf.len(),
        128,
        "property area header must be exactly 128 bytes"
    );
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_property_area_layout() {
        let buf = make_minimal_property_area();
        assert_eq!(buf.len(), 128);
        // bytes_used = 0
        assert_eq!(&buf[0..4], &0u32.to_le_bytes());
        // serial = 0
        assert_eq!(&buf[4..8], &0u32.to_le_bytes());
        // magic = "PROP"
        assert_eq!(&buf[8..12], &0x504f5250u32.to_le_bytes());
        // version = 1
        assert_eq!(&buf[12..16], &1u32.to_le_bytes());
    }

    #[test]
    fn test_vfs_resolves_properties_serial() {
        let vfs = Vfs::new_twrp();
        let node = vfs.resolve("/dev/__properties__/properties_serial");
        assert!(node.is_some(), "properties_serial must be in the VFS");
        match node.unwrap() {
            VfsNode::Dynamic(_) => { /* ok */ }
            other => panic!("expected Dynamic, got {:?}", other),
        }
    }

    #[test]
    fn test_vfs_is_synthetic() {
        let vfs = Vfs::new_twrp();
        assert!(vfs.is_synthetic("/dev/__properties__/properties_serial"));
        assert!(vfs.is_synthetic("/dev/__properties__"));
        assert!(!vfs.is_synthetic("/dev/null"));
    }
}
