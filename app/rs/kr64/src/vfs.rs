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
//!
//! ## Slice 2 (this revision — worklog 4-B)
//!
//! Adds the Android-guest `/proc/self/*` Dynamic nodes that the Android
//! linker and runtime need (worklog 1-B section C.4 listed these as the
//! missing pieces 2-5 for full Android guest boot — Goal #3):
//!
//! - `/proc/self/maps`   — synthetic process memory map.
//! - `/proc/self/status` — synthetic process status with `Pid:`/`VmRSS:`/`Threads:` etc.
//! - `/proc/self/cmdline` — NUL-separated `/sbin/init\0--second-stage\0`.
//! - `/proc/self/auxv`   — minimal-but-valid 64-bit ELF auxiliary vector
//!   (AT_PHDR, AT_PHENT, AT_PHNUM, AT_PAGESZ, AT_BASE, AT_FLAGS, AT_ENTRY,
//!   AT_UID/EUID/GID/EGID, AT_PLATFORM, AT_HWCAP, AT_CLKTCK, AT_SECURE,
//!   AT_RANDOM, AT_HWCAP2, AT_EXECFN, AT_NULL terminator).
//! - `/proc/version`, `/proc/cpuinfo`, `/proc/meminfo` — duplicate the
//!   generators in `proc_emu.rs` (which keeps them private) as Vfs
//!   Dynamic nodes so future namespace-isolated mode can serve them via
//!   the Vfs without going through `proc_emu::populate_proc`.
//!
//! **Why this matters:** 1-A's section C in the worklog notes that the
//! Android guest has NEVER booted on twoyi. The Android linker reads
//! `/proc/self/maps` and `/proc/self/auxv` (well, actually reads them off
//! the kernel-provided stack at exec time — but the VFS-served versions
//! let future per-fd interception work). The runtime also reads
//! `/proc/self/status` for `Threads:` / `VmRSS:` accounting. Without
//! these Dynamic nodes, the VFS layer is incomplete; with them, a future
//! ptrace-emulator wiring (separate task) can serve them when the guest
//! opens the path. Today they are additive — `materialize()` will write
//! them to `{rootfs}/proc/self/*` but the host kernel still serves the
//! real `/proc/self/*` for the actual init process in non-root mode.

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
    ///
    /// TWRP's init is conceptually PID 1 in the container's view; we
    /// delegate to `new_android(1)` which populates the Android-guest
    /// `/proc/self/*` Dynamic nodes alongside the property-area nodes.
    /// Adding the Android nodes here is harmless for TWRP (the host
    /// kernel still serves the real `/proc/*` in non-root mode), and it
    /// keeps a single source of truth for what the Vfs knows about.
    ///
    /// ## Property area: OLD single-file format (AOSP 5.1 bionic)
    ///
    /// TWRP's bionic (AOSP 5.1) opens `/dev/__properties__` as a SINGLE
    /// FILE — the OLD pre-Android-8 property area layout. It does NOT
    /// use `/dev/__properties__/properties_serial` (the NEW Android 8+
    /// subdirectory format that `new_android()` registers below). So
    /// `new_twrp()` overrides the parent `/dev/__properties__` entry to
    /// be a `Synthetic` FILE (not `SyntheticDir`) with the OLD-format
    /// `prop_area` header (128KB) — and removes the Android-guest
    /// `properties_serial` entry that can't exist when the parent is a
    /// file.
    ///
    /// This is the root-cause fix for the SIGSEGV at rip=0x809255d in
    /// TWRP init's `find_property()` (NULL `__system_property_area__`
    /// because `__system_property_area_init` could not open/mmap
    /// `/dev/__properties__`). See worklog 5-Z's disassembly report.
    pub fn new_twrp() -> Self {
        let mut vfs = Self::new_android(1);
        // Override /dev/__properties__ for TWRP's AOSP 5.1 bionic: provide
        // a 128KB Synthetic FILE with the OLD-format prop_area header.
        vfs.entries.insert(
            "/dev/__properties__".to_string(),
            VfsNode::Synthetic(make_old_format_property_area()),
        );
        // The Android-guest `/dev/__properties__/properties_serial` path
        // is not used by TWRP's bionic and is unreachable once the parent
        // `/dev/__properties__` is a regular file. Remove it so the SIGSYS
        // handler's `materialize()` doesn't try to create a child file
        // inside a file (which would fail with ENOTDIR).
        vfs.entries.remove("/dev/__properties__/properties_serial");
        vfs
    }

    /// Create a Vfs pre-populated for Android-guest boot.
    ///
    /// `pid` is the tracee's PID (the init process). All `/proc/self/*`
    /// Dynamic nodes capture this pid so their generated content is
    /// consistent. For TWRP/non-root mode the host kernel serves the
    /// actual `/proc/<pid>/*` and the Vfs-served content is fallback
    /// material (used when the guest path doesn't exist on disk and the
    /// ptrace-emulator asks the Vfs to materialize it).
    pub fn new_android(pid: u32) -> Self {
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
        // ----- Android-guest /proc/self/* Dynamic nodes (worklog 4-B) -----
        // These are required by the Android linker + zygote + system_server
        // (worklog 1-A section C — Android guest never booted). They are
        // generated on each read so the closure sees current state.
        entries.insert(
            "/proc/self/maps".to_string(),
            VfsNode::Dynamic(Box::new(make_proc_self_maps)),
        );
        entries.insert(
            "/proc/self/status".to_string(),
            VfsNode::Dynamic(Box::new(move || make_proc_self_status(pid))),
        );
        entries.insert(
            "/proc/self/cmdline".to_string(),
            VfsNode::Dynamic(Box::new(make_proc_self_cmdline)),
        );
        entries.insert(
            "/proc/self/auxv".to_string(),
            VfsNode::Dynamic(Box::new(make_proc_self_auxv)),
        );
        // ----- /proc/<top-level> mirrors of proc_emu's static files -----
        // proc_emu::populate_proc already writes these as static files into
        // {rootfs}/proc/* at boot. Registering them here as Dynamic nodes
        // makes the Vfs the single source of truth for future per-fd
        // interception (worklog 1-B section C.4 follow-up). The generators
        // duplicate proc_emu's content (proc_emu keeps them private — we
        // cannot reach into that module without violating the file-scope
        // ground rule).
        entries.insert(
            "/proc/version".to_string(),
            VfsNode::Dynamic(Box::new(make_proc_version)),
        );
        entries.insert(
            "/proc/cpuinfo".to_string(),
            VfsNode::Dynamic(Box::new(make_proc_cpuinfo)),
        );
        entries.insert(
            "/proc/meminfo".to_string(),
            VfsNode::Dynamic(Box::new(make_proc_meminfo)),
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

    /// Materialize a VFS node's content into the host filesystem at
    /// `{rootfs}{guest_path}` (creating parent dirs as needed) so a
    /// subsequent real `open()` by the traced child finds the file.
    ///
    /// This is called from the ptrace ENTRY-stop path BEFORE
    /// `translate_path()` runs: by the time the kernel's `open` actually
    /// executes against the translated path (`{rootfs}{guest_path}` for
    /// `/dev/*` paths), the file already exists with the right content.
    ///
    /// Behaviour per `VfsNode` variant:
    /// - `Dynamic` / `Synthetic` — write the bytes to the file (overwrite).
    /// - `SyntheticDir` — `create_dir_all` the path.
    /// - `HostKernel` / `RootfsFile` — no-op (the host/rootfs file is
    ///   already there, or will be created by another path).
    /// - `Absent` — no-op. The caller should let the `open` fail
    ///   naturally with `ENOENT` (the file does not exist).
    ///
    /// Returns `Ok(())` if the file system operation succeeded OR if the
    /// node variant is a no-op. Returns `Err(...)` on actual I/O failure
    /// so the caller can log it.
    pub fn materialize(&self, guest_path: &str, rootfs: &str) -> std::io::Result<()> {
        let Some(node) = self.entries.get(guest_path) else {
            // Path is not a known VFS entry — nothing to materialize.
            return Ok(());
        };
        // The host path where the file should land. For paths under
        // /dev/*, translate_path() rewrites them to `{rootfs}/dev/...`,
        // so we write to the same location. For other paths the caller
        // must ensure the rootfs-relative layout matches.
        let host_path = if guest_path.starts_with('/') {
            format!("{}{}", rootfs, guest_path)
        } else {
            format!("{}/{}", rootfs, guest_path)
        };
        match node {
            VfsNode::Synthetic(bytes) => {
                if let Some(parent) = std::path::Path::new(&host_path).parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&host_path, bytes)?;
            }
            VfsNode::Dynamic(generator) => {
                let bytes = generator();
                if let Some(parent) = std::path::Path::new(&host_path).parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&host_path, &bytes)?;
            }
            VfsNode::SyntheticDir(_entries) => {
                std::fs::create_dir_all(&host_path)?;
            }
            VfsNode::HostKernel(_) | VfsNode::RootfsFile(_) | VfsNode::Absent => {
                // No-op — see method doc.
            }
        }
        Ok(())
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

/// Standard property area file size (128 KB) — `__system_property_area_init`
/// calls `ftruncate(fd, 0x20000)` to extend the file to this size, and mmaps
/// the same length. Pre-sizing the file avoids any filesystem-specific
/// behaviour around ftruncate on a 0-byte file.
pub const PROP_AREA_SIZE: usize = 0x20000;

/// OLD-format `prop_area` version constant (AOSP 5.1 bionic).
///
/// This is the value `__system_property_area_init` writes to
/// `prop_area.version` in TWRP's i386 init binary (extracted from the
/// disassembly at worklog 5-Z Step 3 — the version is the constant stored
/// at offset 12 of the prop_area header). Stock AOSP 5.1 source defines
/// this as `PROP_AREA_VERSION` in `bionic/libc/include/sys/_system_properties.h`.
/// NOTE: this is NOT 1 (the NEW Android 8+ format version that
/// `make_minimal_property_area()` uses for `/dev/__properties__/properties_serial`).
pub const PROP_AREA_VERSION_OLD: u32 = 0xfc6ed0ab;

/// Build a valid OLD-format AOSP `__system_property_area__` for TWRP's
/// AOSP 5.1 bionic — the proper root-cause fix for the SIGSEGV at
/// rip=0x809255d in `find_property()` (worklog 5-Z).
///
/// TWRP's bionic opens `/dev/__properties__` as a SINGLE FILE (no
/// subdirectory) — the OLD pre-Android-8 property area layout. The NEW
/// Android 8+ layout uses `/dev/__properties__/properties_serial` (a
/// file under a subdirectory), which TWRP's bionic never opens.
///
/// Layout (from `bionic/libc/include/sys/_system_properties.h` AOSP 5.1):
/// ```c
/// struct prop_area {
///     unsigned bytes_used;       // 4 — payload bytes used (0 for empty)
///     unsigned volatile serial;  // 4 — increment on write (0 = stable)
///     unsigned magic;            // 4 = PROP_AREA_MAGIC (0x504f5250 "PROP")
///     unsigned version;          // 4 = PROP_AREA_VERSION (0xfc6ed0ab OLD)
///     unsigned reserved[28];     // 112 bytes of zero (rounds out 128-byte header)
///     char data[];               // payload: prop_info structs (empty)
/// };
/// ```
/// Total header = 128 bytes. Standard area file size = 0x20000 (128 KB).
///
/// The OLD-format version is `0xfc6ed0ab` (NOT `1` like the NEW format
/// used by `make_minimal_property_area()` for the Android 8+ path). The
/// magic is the same `0x504f5250` ("PROP") in both formats.
///
/// The 128 KB file size matches what `__system_property_area_init` calls
/// `ftruncate(fd, 0x20000)` + `mmap(NULL, 0x20000, ...)` against. With a
/// pre-sized file the open→ftruncate→mmap sequence has the best chance of
/// succeeding regardless of host filesystem quirks (some filesystems reject
/// `mmap(MAP_SHARED)` on a 0-byte regular file).
///
/// Even though `__system_property_area_init` will `memset` the in-memory
/// header and overwrite `magic/version/bytes_used/serial` itself, we still
/// emit the correct header bytes — this makes the file self-describing if
/// any code reads it without going through bionic's init path (e.g. host
/// diagnostics, `adb pull` inspection, future property-injection hooks).
pub fn make_old_format_property_area() -> Vec<u8> {
    const PROP_AREA_MAGIC: u32 = 0x504f5250; // "PROP" little-endian
    const PROP_AREA_HEADER_SIZE: usize = 128;
    let mut buf = Vec::with_capacity(PROP_AREA_SIZE);
    // bytes_used: 0 (no properties have been written to the data area).
    // NOTE: AOSP 5.1 stock bionic's `init_property_area()` sets
    // `pa->bytes_used = sizeof(prop_area)` (= 128) after mmap. We emit 0
    // here; init will overwrite it on first mmap+memset regardless. The
    // find_property() crash fix only needs the data area zero-initialised
    // (which it is, via the resize() zero-fill below) so the root prop_bt
    // is all-zeros and the tree-walk returns NULL immediately.
    buf.extend_from_slice(&0u32.to_le_bytes());
    // serial: 0 (stable, no concurrent writes).
    buf.extend_from_slice(&0u32.to_le_bytes());
    // magic = "PROP"
    buf.extend_from_slice(&PROP_AREA_MAGIC.to_le_bytes());
    // version = 0xfc6ed0ab (OLD AOSP 5.1 format)
    buf.extend_from_slice(&PROP_AREA_VERSION_OLD.to_le_bytes());
    // reserved[28] = 112 bytes of zero (rounds out the 128-byte header)
    buf.extend_from_slice(&[0u8; 112]);
    debug_assert_eq!(
        buf.len(),
        PROP_AREA_HEADER_SIZE,
        "OLD-format prop_area header must be exactly 128 bytes"
    );
    // data area: zero-pad to 0x20000 (128 KB) so ftruncate + mmap MAP_SHARED
    // succeed (the file needs to be backed by real storage of the right size).
    buf.resize(PROP_AREA_SIZE, 0u8);
    buf
}

// ============================================================================
// Android-guest /proc/self/* generators (worklog 4-B).
//
// These are PRIVATE to the vfs module. They are wired into the Vfs as
// `Dynamic` nodes (closures invoked on each read). Each generator returns
// a `Vec<u8>` whose content matches what a real Linux kernel would put
// in the corresponding `/proc/self/*` file.
//
// The exact field values are synthetic placeholders — the guest reads them
// but rarely validates them against reality. The FORMAT is what matters.
// ============================================================================

/// Synthesize `/proc/self/maps` — one line per memory mapping.
///
/// Format (per `man 5 proc`):
/// ```text
/// address           perms offset  dev   inode      pathname
/// 0123456789abcdef-0123456789abcdef r-xp 00000000 fe:00 1234567 /system/lib/libc.so
/// ```
///
/// We emit a minimal-but-realistic map for the kr64 guest: the init binary,
/// the dynamic linker, the three core shared libs (libc/libm/libdl), plus
/// the standard `[stack]` / `[heap]` / `[vdso]` pseudo-mappings. The exact
/// addresses are placeholder values; the linker's actual memory map comes
/// from its own `mmap()`s (the kernel sets it up), not from this file.
/// Tools that read `/proc/self/maps` for diagnostics (e.g. logd when
/// dumping crash backtraces) just need the format to be valid.
fn make_proc_self_maps() -> Vec<u8> {
    // Hardcoded lines — no per-line format!() needed; matches the proc_emu
    // style of `concat!()` for static-ish content.
    let maps = concat!(
        "00400000-00401000 r-xp 00000000 fe:00 1001     /sbin/init\n",
        "70000000-70010000 r-xp 00000000 fe:00 1002     /sbin/linker64\n",
        "71000000-71010000 r-xp 00000000 fe:00 1003     /system/lib/libc.so\n",
        "71100000-71110000 r-xp 00000000 fe:00 1004     /system/lib/libm.so\n",
        "71200000-71210000 r-xp 00000000 fe:00 1005     /system/lib/libdl.so\n",
        "7ff00000-7ff10000 rw-p 00000000 00:00 0        [stack]\n",
        "7ff20000-7ff30000 rw-p 00000000 00:00 0        [heap]\n",
        "7fff0000-7fff1000 r-xp 00000000 00:00 0        [vdso]\n",
    );
    maps.as_bytes().to_vec()
}

/// Synthesize `/proc/self/status` — process status fields (one per line).
///
/// Format (per `man 5 proc`):
/// ```text
/// Name:   init
/// Umask:  0077
/// State:  S (sleeping)
/// Tgid:   <pid>
/// Pid:    <pid>
/// VmRSS:   1234 kB
/// VmSize:  5678 kB
/// Threads: 1
/// ```
///
/// `pid` is the tracee's PID (captured by the Dynamic closure). For init
/// this is 1. Sensible defaults for an Android init process are used.
fn make_proc_self_status(pid: u32) -> Vec<u8> {
    let s = format!(
        "Name:\tinit\n\
         Umask:\t0077\n\
         State:\tS (sleeping)\n\
         Tgid:\t{pid}\n\
         Ngid:\t0\n\
         Pid:\t{pid}\n\
         PPid:\t0\n\
         TracerPid:\t0\n\
         Uid:\t0\t0\t0\t0\n\
         Gid:\t0\t0\t0\t0\n\
         FDSize:\t256\n\
         Groups:\n\
         VmPeak:\t   10000 kB\n\
         VmSize:\t    8000 kB\n\
         VmLck:\t        0 kB\n\
         VmPin:\t        0 kB\n\
         VmHWM:\t    4000 kB\n\
         VmRSS:\t    4000 kB\n\
         VmData:\t    2000 kB\n\
         VmStk:\t      132 kB\n\
         VmExe:\t     1024 kB\n\
         VmLib:\t    4000 kB\n\
         VmPTE:\t       64 kB\n\
         VmSwap:\t        0 kB\n\
         Threads:\t1\n\
         SigQ:\t0/256\n\
         SigPnd:\t0000000000000000\n\
         ShdPnd:\t0000000000000000\n\
         SigBlk:\t0000000000000000\n\
         SigIgn:\t0000000000000000\n\
         SigCgt:\t0000000180000000\n\
         CapInh:\t0000000000000000\n\
         CapPrm:\t000001ffffffffff\n\
         CapEff:\t000001ffffffffff\n\
         CapBnd:\t000001ffffffffff\n\
         CapAmb:\t0000000000000000\n\
         Seccomp:\t0\n\
         Cpus_allowed:\tff\n\
         Cpus_allowed_list:\t0-7\n",
        pid = pid,
    );
    s.into_bytes()
}

/// Synthesize `/proc/self/cmdline` — NUL-separated argv.
///
/// Format: `argv[0]\0argv[1]\0...argv[N-1]\0` — NUL after each element
/// (including the last). For the Android init process this is
/// `/sbin/init\0--second-stage\0` after init transitions to second stage
/// (the early-boot form is just `/init\0` — we emit the post-second-stage
/// form because that's what guest services observe when they read
/// `/proc/<pid>/cmdline`).
fn make_proc_self_cmdline() -> Vec<u8> {
    b"/sbin/init\0--second-stage\0".to_vec()
}

/// Synthesize `/proc/self/auxv` — the 64-bit ELF auxiliary vector.
///
/// Layout (64-bit): an array of `Elf64_auxv_t` entries, each 16 bytes
/// (8-byte `a_type` + 8-byte `a_val`), terminated by `AT_NULL` (0, 0).
/// The kernel populates this at exec() time from the initial stack area;
/// the value of `a_val` for string-typed entries (AT_PLATFORM, AT_EXECFN,
/// AT_RANDOM) is a pointer into the process's stack memory — NOT into the
/// auxv file itself.
///
/// We emit a minimal-but-valid set covering the entries the task brief
/// lists (worklog 4-B Step 1). String-valued entries get placeholder
/// addresses; tools that read `/proc/self/auxv` only inspect the
/// type/value pairs without dereferencing those addresses. The linker
/// itself uses the kernel-passed auxv on its stack, not this file.
fn make_proc_self_auxv() -> Vec<u8> {
    // Standard Linux UAPI AT_* values (from <linux/auxvec.h>).
    const AT_NULL: u64 = 0;
    const AT_PHDR: u64 = 3;
    const AT_PHENT: u64 = 4;
    const AT_PHNUM: u64 = 5;
    const AT_PAGESZ: u64 = 6;
    const AT_BASE: u64 = 7;
    const AT_FLAGS: u64 = 8;
    const AT_ENTRY: u64 = 9;
    const AT_UID: u64 = 11;
    const AT_EUID: u64 = 12;
    const AT_GID: u64 = 13;
    const AT_EGID: u64 = 14;
    const AT_PLATFORM: u64 = 15;
    const AT_HWCAP: u64 = 16;
    const AT_CLKTCK: u64 = 17;
    const AT_SECURE: u64 = 23;
    const AT_RANDOM: u64 = 25;
    const AT_HWCAP2: u64 = 26;
    const AT_EXECFN: u64 = 31;

    // Placeholder addresses for string-valued entries. In a real
    // process these point into the initial stack area set up by the
    // kernel; tools reading /proc/self/auxv only inspect the type/value
    // pairs without dereferencing these addresses.
    const ADDR_PHDR: u64 = 0x4000_0000;
    const ADDR_BASE: u64 = 0x7000_0000;
    const ADDR_ENTRY: u64 = 0x4000_1000;
    const ADDR_PLATFORM: u64 = 0x7fff_f000;
    const ADDR_RANDOM: u64 = 0x7fff_f010;
    const ADDR_EXECFN: u64 = 0x7fff_f020;

    // HWCAP bitmask — arch-dependent. The exact bits don't matter much
    // here (the linker reads its own stack-passed AT_HWCAP for actual
    // capability gating); a representative value keeps the format valid.
    let hwcap: u64 = if cfg!(target_arch = "aarch64") {
        // FP|ASIMD|EVTSTRM|AES|PMULL|SHA1|SHA2|CRC32|ATOMICS = 0x1ff
        0x1ff
    } else if cfg!(target_arch = "x86_64") {
        // FPU|VME|DE|PSE|TSC|MSR|PAE|MCE|CX8|APIC|SEP|MTRR|PGE|MCA|CMOV|PAT = 0xffff
        0xffff
    } else {
        0
    };

    let entries: [(u64, u64); 19] = [
        (AT_PHDR, ADDR_PHDR),
        (AT_PHENT, 56), // sizeof(Elf64_Phdr)
        (AT_PHNUM, 6),
        (AT_PAGESZ, 4096),
        (AT_BASE, ADDR_BASE),
        (AT_FLAGS, 0),
        (AT_ENTRY, ADDR_ENTRY),
        (AT_UID, 0),
        (AT_EUID, 0),
        (AT_GID, 0),
        (AT_EGID, 0),
        (AT_PLATFORM, ADDR_PLATFORM),
        (AT_HWCAP, hwcap),
        (AT_CLKTCK, 100),
        (AT_SECURE, 0),
        (AT_RANDOM, ADDR_RANDOM),
        (AT_HWCAP2, 0),
        (AT_EXECFN, ADDR_EXECFN),
        (AT_NULL, 0), // terminator
    ];

    let mut buf = Vec::with_capacity(entries.len() * 16);
    for (a_type, a_val) in entries {
        buf.extend_from_slice(&a_type.to_le_bytes());
        buf.extend_from_slice(&a_val.to_le_bytes());
    }
    assert!(
        buf.len() % 16 == 0,
        "auxv buffer must be a multiple of 16 bytes (one Elf64_auxv_t per entry)"
    );
    buf
}

/// Synthesize `/proc/version` — duplicate of `proc_emu::write_proc_version`
/// (kept private in proc_emu.rs — we duplicate here since the file-scope
/// ground rule forbids touching proc_emu.rs).
fn make_proc_version() -> Vec<u8> {
    concat!(
        "Linux version 4.14.190-g45619c7d3dc8-ab7891234 ",
        "(build-user@build-host) (Android clang 11.0.5) ",
        "4.14.190-g45619c7d3dc8-ab7891234 #1 SMP PREEMPT ",
        "Mon Jan 01 00:00:00 UTC 2026 (aarch64)\n",
    )
    .as_bytes()
    .to_vec()
}

/// Synthesize `/proc/cpuinfo` — minimal duplicate of
/// `proc_emu::write_proc_cpuinfo`. proc_emu takes a `cpu_count` and emits
/// one block per CPU; here we hardcode a single CPU block (the guest's
/// ActivityManagerService mostly just needs the file to exist; future
/// per-fd interception can pass a real cpu_count if needed).
fn make_proc_cpuinfo() -> Vec<u8> {
    let mut content = String::new();
    #[cfg(target_arch = "aarch64")]
    {
        content.push_str(
            "processor\t: 0\n\
             BogoMIPS\t: 200.00\n\
             Features\t: fp asimd evtstrm aes pmull sha1 sha2 crc32 atomics\n\
             CPU implementer\t: 0x51\n\
             CPU architecture: 8\n\
             CPU variant\t: 0xc\n\
             CPU part\t: 0x805\n\
             CPU revision\t: 14\n\n",
        );
    }
    #[cfg(target_arch = "x86_64")]
    {
        content.push_str(
            "processor\t: 0\n\
             vendor_id\t: GenuineIntel\n\
             cpu family\t: 6\n\
             model\t\t: 85\n\
             model name\t: Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz\n\
             stepping\t: 7\n\
             cpu MHz\t\t: 2793.438\n\
             cache size\t: 49152 KB\n\
             bogomips\t: 5586.87\n\n",
        );
    }
    // For other architectures, emit a minimal block.
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        content.push_str("processor\t: 0\n\n");
    }
    content.into_bytes()
}

/// Synthesize `/proc/meminfo` — minimal duplicate of
/// `proc_emu::write_proc_meminfo`. proc_emu takes a `mem_mb` parameter;
/// here we hardcode 4096 MB (matching the lib.rs:2205 default). We emit a
/// reduced field set (the lines Android's ActivityManagerService actually
/// reads) — full parity with proc_emu can be added later if a guest
/// service needs it.
fn make_proc_meminfo() -> Vec<u8> {
    const MEM_MB: u64 = 4096;
    let mem_total_kb: u64 = MEM_MB * 1024;
    let mem_free_kb: u64 = MEM_MB * 1024 / 4;
    let mem_avail_kb: u64 = MEM_MB * 1024 / 2;
    let cached_kb: u64 = MEM_MB * 1024 / 4;
    let buffers_kb: u64 = MEM_MB * 1024 / 16;
    let s = format!(
        "MemTotal:       {:>8} kB\n\
         MemFree:        {:>8} kB\n\
         MemAvailable:   {:>8} kB\n\
         Buffers:        {:>8} kB\n\
         Cached:         {:>8} kB\n\
         SwapCached:            0 kB\n\
         Active:         {:>8} kB\n\
         Inactive:       {:>8} kB\n\
         SwapTotal:            0 kB\n\
         SwapFree:             0 kB\n\
         Dirty:                0 kB\n\
         Writeback:            0 kB\n\
         AnonPages:      {:>8} kB\n\
         Mapped:         {:>8} kB\n\
         Shmem:          {:>8} kB\n\
         Slab:           {:>8} kB\n\
         SReclaimable:   {:>8} kB\n\
         SUnreclaim:     {:>8} kB\n\
         KernelStack:       16384 kB\n\
         PageTables:        32768 kB\n\
         CommitLimit:   {:>8} kB\n\
         Committed_AS:  {:>8} kB\n\
         VmallocTotal:  536870912 kB\n\
         VmallocUsed:   {:>8} kB\n\
         VmallocChunk:  536870912 kB\n\
         HugePages_Total:     0\n\
         HugePages_Free:      0\n\
         Hugepagesize:      2048 kB\n",
        mem_total_kb, // MemTotal
        mem_free_kb,  // MemFree
        mem_avail_kb, // MemAvailable
        buffers_kb,   // Buffers
        cached_kb,    // Cached
        mem_avail_kb, // Active
        cached_kb,    // Inactive
        mem_avail_kb, // AnonPages
        mem_avail_kb, // Mapped
        cached_kb,    // Shmem
        cached_kb,    // Slab
        cached_kb,    // SReclaimable
        cached_kb,    // SUnreclaim
        mem_total_kb, // CommitLimit
        mem_avail_kb, // Committed_AS
        cached_kb,    // VmallocUsed
    );
    s.into_bytes()
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
    fn test_vfs_resolves_properties_serial_in_android_mode() {
        // The NEW Android 8+ format `/dev/__properties__/properties_serial`
        // is registered in `new_android()` (Android-guest path). It is NOT
        // in `new_twrp()` (TWRP path uses the OLD single-file format).
        let vfs = Vfs::new_android(1);
        let node = vfs.resolve("/dev/__properties__/properties_serial");
        assert!(node.is_some(), "properties_serial must be in new_android()");
        match node.unwrap() {
            VfsNode::Dynamic(_) => { /* ok */ }
            other => panic!("expected Dynamic, got {:?}", other),
        }
    }

    #[test]
    fn test_vfs_is_synthetic_android_mode() {
        let vfs = Vfs::new_android(1);
        assert!(vfs.is_synthetic("/dev/__properties__/properties_serial"));
        assert!(vfs.is_synthetic("/dev/__properties__"));
        assert!(!vfs.is_synthetic("/dev/null"));
    }

    #[test]
    fn test_vfs_materialize_writes_properties_serial_file_android_mode() {
        // Materialize the NEW Android 8+ format property-area Dynamic node
        // into a temp rootfs and verify the file content matches
        // make_minimal_property_area(). Uses new_android(1) because
        // new_twrp() overrides /dev/__properties__ with the OLD-format file.
        let tmp = std::env::temp_dir().join(format!("kr64_vfs_test_{}", std::process::id()));
        let rootfs = tmp.to_str().unwrap();
        // Clean up any prior run + recreate
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let vfs = Vfs::new_android(1);
        vfs.materialize("/dev/__properties__/properties_serial", rootfs)
            .expect("materialize must succeed");
        let written = std::fs::read(format!("{}/dev/__properties__/properties_serial", rootfs))
            .expect("file must exist after materialize");
        assert_eq!(written, make_minimal_property_area());
        assert_eq!(written.len(), 128);
        // The directory itself is registered as a SyntheticDir entry in
        // new_android(1) — materializing it must create_dir_all the dir at rootfs.
        vfs.materialize("/dev/__properties__", rootfs)
            .expect("SyntheticDir materialize must succeed");
        let md = std::fs::metadata(format!("{}/dev/__properties__", rootfs))
            .expect("dir must exist after SyntheticDir materialize");
        assert!(md.is_dir());
        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ===== Tests for the OLD-format /dev/__properties__ file (worklog 6-A) =====
    // TWRP's AOSP 5.1 bionic opens /dev/__properties__ as a SINGLE FILE
    // (not the Android 8+ subdirectory). new_twrp() registers it as a
    // Synthetic FILE with the OLD-format prop_area header (128KB).

    #[test]
    fn test_old_format_property_area_header() {
        let buf = make_old_format_property_area();
        assert_eq!(
            buf.len(),
            PROP_AREA_SIZE,
            "total file size must be 0x20000 (128 KB)"
        );
        // Header is the first 128 bytes; verify each field.
        // bytes_used = 0 (no properties written yet)
        assert_eq!(&buf[0..4], &0u32.to_le_bytes(), "bytes_used must be 0");
        // serial = 0 (stable, no concurrent writes)
        assert_eq!(&buf[4..8], &0u32.to_le_bytes(), "serial must be 0");
        // magic = "PROP" (0x504f5250 little-endian)
        assert_eq!(
            &buf[8..12],
            &0x504f5250u32.to_le_bytes(),
            "magic must be 0x504f5250 ('PROP')"
        );
        // version = 0xfc6ed0ab (OLD AOSP 5.1 format)
        assert_eq!(
            &buf[12..16],
            &PROP_AREA_VERSION_OLD.to_le_bytes(),
            "version must be 0xfc6ed0ab (OLD AOSP 5.1 format)"
        );
        // Sanity: must NOT be the NEW format version (1).
        assert_ne!(
            &buf[12..16],
            &1u32.to_le_bytes(),
            "must not be NEW format (version 1)"
        );
    }

    #[test]
    fn test_old_format_property_area_size() {
        let buf = make_old_format_property_area();
        // Standard property area size = 0x20000 = 131072 bytes = 128 KB.
        assert_eq!(
            buf.len(),
            0x20000,
            "OLD-format property area must be 0x20000 bytes"
        );
        assert_eq!(buf.len(), 128 * 1024, "must be 128 KB exactly");
        // All bytes beyond the 128-byte header must be zero (no props).
        for (i, &b) in buf[128..].iter().enumerate() {
            assert_eq!(
                b,
                0u8,
                "byte at offset {} (in data area) must be zero",
                128 + i
            );
        }
    }

    #[test]
    fn test_vfs_resolves_dev_properties_old_format() {
        // new_twrp() must register /dev/__properties__ as a Synthetic FILE
        // (the OLD-format single-file layout TWRP's AOSP 5.1 bionic opens).
        let vfs = Vfs::new_twrp();
        let node = vfs.resolve("/dev/__properties__");
        assert!(
            node.is_some(),
            "/dev/__properties__ must resolve in new_twrp()"
        );
        match node.unwrap() {
            VfsNode::Synthetic(bytes) => {
                assert_eq!(
                    bytes.len(),
                    PROP_AREA_SIZE,
                    "Synthetic /dev/__properties__ must be 0x20000 bytes"
                );
                // Header sanity: magic + version must match OLD format.
                assert_eq!(
                    &bytes[8..12],
                    &0x504f5250u32.to_le_bytes(),
                    "magic must be PROP"
                );
                assert_eq!(
                    &bytes[12..16],
                    &PROP_AREA_VERSION_OLD.to_le_bytes(),
                    "version must be OLD-format 0xfc6ed0ab"
                );
            }
            other => panic!("expected Synthetic, got {:?}", other),
        }
    }

    #[test]
    fn test_vfs_twrp_does_not_register_properties_serial() {
        // new_twrp() must NOT register the Android 8+ subdirectory path
        // (it conflicts with the file-only /dev/__properties__ entry).
        let vfs = Vfs::new_twrp();
        assert!(
            vfs.resolve("/dev/__properties__/properties_serial")
                .is_none(),
            "new_twrp() must not register /dev/__properties__/properties_serial"
        );
    }

    #[test]
    fn test_vfs_materialize_writes_old_format_properties_file() {
        // Materialize the OLD-format Synthetic file into a temp rootfs and
        // verify the on-disk file is a regular file (NOT a directory) with
        // the full 128KB OLD-format content.
        let tmp =
            std::env::temp_dir().join(format!("kr64_vfs_old_prop_test_{}", std::process::id()));
        let rootfs = tmp.to_str().unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let vfs = Vfs::new_twrp();
        vfs.materialize("/dev/__properties__", rootfs)
            .expect("materialize /dev/__properties__ must succeed");
        let written = std::fs::read(format!("{}/dev/__properties__", rootfs))
            .expect("file must exist after materialize");
        assert_eq!(written, make_old_format_property_area());
        assert_eq!(written.len(), PROP_AREA_SIZE);
        // Must be a regular file (NOT a directory) — TWRP's bionic opens it
        // with O_RDWR and mmaps it as a regular file.
        let md = std::fs::metadata(format!("{}/dev/__properties__", rootfs))
            .expect("metadata must succeed");
        assert!(
            md.is_file(),
            "/dev/__properties__ must be a regular FILE in new_twrp()"
        );
        assert!(
            !md.is_dir(),
            "/dev/__properties__ must NOT be a directory in new_twrp()"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_vfs_materialize_no_op_for_unknown_path() {
        let vfs = Vfs::new_twrp();
        // A path that is NOT in the VFS must materialize as a no-op Ok.
        vfs.materialize("/dev/null", "/tmp/nonexistent_rootfs")
            .expect("materialize on unknown path must be Ok no-op");
    }

    #[test]
    fn test_vfs_is_synthetic_twrp_old_format() {
        // new_twrp() must register /dev/__properties__ (as a Synthetic FILE)
        // and NOT register /dev/__properties__/properties_serial.
        let vfs = Vfs::new_twrp();
        assert!(
            vfs.is_synthetic("/dev/__properties__"),
            "/dev/__properties__ must be synthetic in new_twrp()"
        );
        assert!(
            !vfs.is_synthetic("/dev/__properties__/properties_serial"),
            "properties_serial must NOT be synthetic in new_twrp()"
        );
        assert!(!vfs.is_synthetic("/dev/null"));
    }

    // ===== Tests for the new /proc/self/* Dynamic nodes (worklog 4-B) =====

    #[test]
    fn test_proc_self_maps_format() {
        let buf = make_proc_self_maps();
        let s = std::str::from_utf8(&buf).expect("maps must be valid UTF-8");
        // Must contain at least one r-xp mapping line for a real path.
        assert!(
            s.contains("r-xp"),
            "maps must contain at least one r-xp mapping; got: {s}"
        );
        // First non-empty line must start with a hex address range like
        // 0123456789abcdef-0123456789abcdef.
        let first_line = s.lines().next().expect("maps must have at least one line");
        assert!(
            first_line.starts_with(|c: char| c.is_ascii_hexdigit()),
            "first line must start with a hex digit; got: {first_line}"
        );
        let dash = first_line.find('-');
        assert!(dash.is_some(), "first line must contain a '-' separator");
        // Pathname must be present somewhere (e.g. /system/lib/libc.so).
        assert!(
            s.contains("/system/lib/libc.so"),
            "maps must reference /system/lib/libc.so; got: {s}"
        );
        // Pseudo-mappings must be present.
        assert!(s.contains("[stack]"), "maps must contain [stack]");
        assert!(s.contains("[heap]"), "maps must contain [heap]");
        assert!(s.contains("[vdso]"), "maps must contain [vdso]");
    }

    #[test]
    fn test_proc_self_status_contains_pid() {
        let pid = 42u32;
        let buf = make_proc_self_status(pid);
        let s = std::str::from_utf8(&buf).expect("status must be valid UTF-8");
        // Must contain `Name:\tinit` (init process).
        assert!(
            s.contains("Name:\tinit"),
            "status must contain Name: init; got: {s}"
        );
        // Must contain `Pid:\t<pid>` exactly.
        let expected_pid_line = format!("Pid:\t{}", pid);
        assert!(
            s.contains(&expected_pid_line),
            "status must contain '{}'; got: {}",
            expected_pid_line,
            s
        );
        // Must contain `Tgid:\t<pid>` (thread group leader id).
        let expected_tgid_line = format!("Tgid:\t{}", pid);
        assert!(
            s.contains(&expected_tgid_line),
            "status must contain '{}'; got: {}",
            expected_tgid_line,
            s
        );
        // Must contain VmRSS and Threads lines (task brief).
        assert!(s.contains("VmRSS:"), "status must contain VmRSS line");
        assert!(s.contains("Threads:"), "status must contain Threads line");
    }

    #[test]
    fn test_proc_self_cmdline_null_separated() {
        let buf = make_proc_self_cmdline();
        // Must contain at least one NUL separator.
        assert!(
            buf.contains(&0u8),
            "cmdline must contain NUL separators; got: {:?}",
            buf
        );
        // First NUL-separated token must be the binary path (/sbin/init).
        let first = buf.split(|&b| b == 0u8).next().unwrap_or(&[]);
        assert_eq!(
            std::str::from_utf8(first).unwrap_or(""),
            "/sbin/init",
            "first cmdline token must be /sbin/init; got: {:?}",
            first
        );
        // Second token must be --second-stage.
        let mut tokens = buf.split(|&b| b == 0u8);
        let _ = tokens.next();
        let second = tokens.next().unwrap_or(&[]);
        assert_eq!(
            std::str::from_utf8(second).unwrap_or(""),
            "--second-stage",
            "second cmdline token must be --second-stage; got: {:?}",
            second
        );
    }

    #[test]
    fn test_proc_self_auxv_nonempty_or_stub() {
        let buf = make_proc_self_auxv();
        // Either non-empty (real implementation) OR explicitly labeled stub.
        assert!(
            !buf.is_empty(),
            "auxv buffer must be non-empty (real implementation, not stub)"
        );
        // Each Elf64_auxv_t entry is 16 bytes (8-byte a_type + 8-byte a_val).
        // Buffer length must be a multiple of 16.
        assert_eq!(
            buf.len() % 16,
            0,
            "auxv buffer length must be a multiple of 16 (one Elf64_auxv_t per entry); got {}",
            buf.len()
        );
        // Must end with an AT_NULL terminator (a_type=0, a_val=0).
        let n = buf.len();
        assert!(n >= 16, "auxv must have at least one entry (the AT_NULL)");
        let last_type = u64::from_le_bytes(buf[n - 16..n - 8].try_into().unwrap());
        let last_val = u64::from_le_bytes(buf[n - 8..n].try_into().unwrap());
        assert_eq!(
            last_type, 0,
            "last auxv entry's a_type must be AT_NULL (0); got {}",
            last_type
        );
        assert_eq!(
            last_val, 0,
            "last auxv entry's a_val must be 0; got {}",
            last_val
        );
    }

    #[test]
    fn test_proc_version_has_linux_prefix() {
        let buf = make_proc_version();
        let s = std::str::from_utf8(&buf).expect("version must be valid UTF-8");
        assert!(
            s.starts_with("Linux version "),
            "proc/version must start with 'Linux version '; got: {s}"
        );
    }

    #[test]
    fn test_proc_cpuinfo_has_processor() {
        let buf = make_proc_cpuinfo();
        let s = std::str::from_utf8(&buf).expect("cpuinfo must be valid UTF-8");
        assert!(
            s.contains("processor\t: 0"),
            "cpuinfo must contain 'processor\\t: 0'; got: {s}"
        );
    }

    #[test]
    fn test_proc_meminfo_has_memtotal() {
        let buf = make_proc_meminfo();
        let s = std::str::from_utf8(&buf).expect("meminfo must be valid UTF-8");
        assert!(
            s.contains("MemTotal:"),
            "meminfo must contain 'MemTotal:'; got: {s}"
        );
    }

    #[test]
    fn test_vfs_resolves_proc_self_maps() {
        let vfs = Vfs::new_android(123);
        let node = vfs.resolve("/proc/self/maps");
        assert!(node.is_some(), "/proc/self/maps must be in the VFS");
        match node.unwrap() {
            VfsNode::Dynamic(_) => { /* ok */ }
            other => panic!("expected Dynamic, got {:?}", other),
        }
    }

    #[test]
    fn test_vfs_resolves_proc_self_status_with_pid() {
        // The status Dynamic closure must capture the pid passed to
        // new_android() — verify by materializing and checking the
        // generated content contains the right `Pid:` line.
        let vfs = Vfs::new_android(777);
        let tmp =
            std::env::temp_dir().join(format!("kr64_vfs_proc_status_test_{}", std::process::id()));
        let rootfs = tmp.to_str().unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        vfs.materialize("/proc/self/status", rootfs)
            .expect("materialize must succeed");
        let written = std::fs::read(format!("{rootfs}/proc/self/status"))
            .expect("file must exist after materialize");
        let s = std::str::from_utf8(&written).expect("status must be UTF-8");
        assert!(
            s.contains("Pid:\t777"),
            "status must reflect the pid passed to new_android(); got: {s}"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_vfs_resolves_proc_self_auxv() {
        let vfs = Vfs::new_android(1);
        let node = vfs.resolve("/proc/self/auxv");
        assert!(node.is_some(), "/proc/self/auxv must be in the VFS");
    }

    #[test]
    fn test_vfs_resolves_proc_top_level_mirrors() {
        let vfs = Vfs::new_android(1);
        assert!(
            vfs.resolve("/proc/version").is_some(),
            "/proc/version missing"
        );
        assert!(
            vfs.resolve("/proc/cpuinfo").is_some(),
            "/proc/cpuinfo missing"
        );
        assert!(
            vfs.resolve("/proc/meminfo").is_some(),
            "/proc/meminfo missing"
        );
    }

    #[test]
    fn test_vfs_materialize_proc_self_maps_into_rootfs() {
        let tmp =
            std::env::temp_dir().join(format!("kr64_vfs_proc_maps_test_{}", std::process::id()));
        let rootfs = tmp.to_str().unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let vfs = Vfs::new_android(1);
        vfs.materialize("/proc/self/maps", rootfs)
            .expect("materialize /proc/self/maps must succeed");
        let written = std::fs::read(format!("{rootfs}/proc/self/maps"))
            .expect("file must exist after materialize");
        assert_eq!(written, make_proc_self_maps());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_vfs_materialize_proc_self_auxv_into_rootfs() {
        let tmp =
            std::env::temp_dir().join(format!("kr64_vfs_proc_auxv_test_{}", std::process::id()));
        let rootfs = tmp.to_str().unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let vfs = Vfs::new_android(1);
        vfs.materialize("/proc/self/auxv", rootfs)
            .expect("materialize /proc/self/auxv must succeed");
        let written = std::fs::read(format!("{rootfs}/proc/self/auxv"))
            .expect("file must exist after materialize");
        // Length must be a multiple of 16 (one Elf64_auxv_t per entry).
        assert_eq!(written.len() % 16, 0);
        assert!(!written.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_new_twrp_delegates_to_new_android_pid_1() {
        // new_twrp() must produce a Vfs whose /proc/self/status reflects
        // pid=1 (the init process), since TWRP's init is conceptually
        // PID 1 in the container's view.
        let vfs = Vfs::new_twrp();
        let tmp =
            std::env::temp_dir().join(format!("kr64_vfs_new_twrp_test_{}", std::process::id()));
        let rootfs = tmp.to_str().unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        vfs.materialize("/proc/self/status", rootfs)
            .expect("materialize must succeed");
        let written = std::fs::read(format!("{rootfs}/proc/self/status"))
            .expect("file must exist after materialize");
        let s = std::str::from_utf8(&written).expect("status must be UTF-8");
        assert!(
            s.contains("Pid:\t1"),
            "new_twrp() must use pid=1 for /proc/self/status; got: {s}"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
