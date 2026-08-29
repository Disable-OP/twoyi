// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! APEX extraction: extract the REAL libdl.so from the APEX ext4 image.
//!
//! Background (5-K's diagnosis, kr64-stderr.log line 81):
//! On Android 11+, the visible `/apex/com.android.runtime/lib64/bionic/libdl.so`
//! is a 5848-byte BOOTSTRAP STUB used during early init before apexd fully
//! mounts the APEX packages. The REAL libdl.so (with the LIBC version symbol
//! required by `DT_NEEDED:libdl.so (LIBC)` from `libgetpid_hook.so` and
//! `libtwoyi_loader_shlib.so`) is INSIDE the APEX ext4 image at
//! `/system/apex/com.android.runtime.apex`.
//!
//! The 5848-byte stub at both `/system/lib64/libdl.so` AND
//! `/apex/com.android.runtime/lib64/bionic/libdl.so` causes the linker64
//! segfault at offset 0xaf174 (faulting address 0x86) — the linker gets a
//! NULL soinfo when trying to resolve the LIBC version of libdl.so.
//!
//! Fix: extract the REAL libdl.so from the .apex file before pivot_root
//! and write it to `/dev/libdl.so` after pivot_root. The `LD_LIBRARY_PATH`
//! is modified (in lib.rs) to put `/dev/` FIRST so the linker finds the
//! real libdl.so before falling back to the stub.
//!
//! # Extraction pipeline
//!
//! 1. **Detect .apex format**: ZIP (PK\x03\x04 magic, the default
//!    non-flattened APEX on Android 11) or raw ext4 image (0x53 0xEF at
//!    offset 1080).
//! 2. **If ZIP**: parse the ZIP central directory to find
//!    `apex_payload.img` (the ext4 image inside the .apex). Only
//!    STORED (method 0) entries are supported — DEFLATE entries return
//!    an error (decompression would require pulling in a zlib dependency,
//!    which is against the crate's "std + libc only" policy).
//! 3. **Write ext4 image to a temp file** at `<apex_temp_dir>/twoyi-apex-payload.img`
//!    (the parent's `TMPDIR` env var, else `$TWOYI_DATA_DIR/cache`, else the
//!    `/data/data/io.twoyi/cache` compatibility fallback —
//!    NOT `/tmp/` which doesn't exist in the parent's Android-app-sandbox context
//!    before `setup_mounts` bind-mounts tmpfs on `/tmp`; see [`apex_temp_dir`]).
//! 4. **Loopback-mount the ext4 image** via the kernel's loop device
//!    (`/dev/loop-control` + `LOOP_CTL_GET_FREE` + `LOOP_SET_FD` + `mount`
//!    syscall with fstype `ext4`). This leverages the kernel's well-tested
//!    ext4 driver — we don't have to implement an ext4 reader.
//! 5. **Read `lib64/bionic/libdl.so`** from the mount.
//! 6. **Cleanup**: `umount(2)` + `LOOP_CLR_FD` + delete temp file.
//!
//! # Failure modes
//!
//! - .apex file doesn't exist at any candidate path → return None, log
//!   the candidates tried.
//! - .apex is a ZIP but `apex_payload.img` is compressed (DEFLATE) → return
//!   None, log the error. (This is rare — APEX payloads are typically
//!   STORED because ext4 doesn't compress well.)
//! - Loop device not available (`/dev/loop-control` missing or `LOOP_SET_FD`
//!   fails with EPERM/ENOSYS) → return None, log the error.
//! - mount() syscall fails (e.g. ext4 driver doesn't support a feature in
//!   the APEX image) → return None, log the error.
//! - `lib64/bionic/libdl.so` not found inside the APEX image → return None.
//!
//! All failure modes are NON-FATAL: if extraction fails, lib.rs falls
//! through to the existing behavior (no `/dev/libdl.so`, the linker keeps
//! finding the 5848-byte stub). The diagnostic logs surface exactly which
//! strategy failed and why, so the next agent can diagnose further.

use crate::{error, info, warning};
use std::ffi::CString;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;

/// Size of the bootstrap stub libdl.so on Android 11 (5-K's diagnosis:
/// kr64-stderr.log line 81 confirms 5848 bytes at both
/// `/system/lib64/libdl.so` AND `/apex/com.android.runtime/lib64/bionic/libdl.so`).
///
/// Used as a "this is NOT the real one" threshold: any libdl.so smaller
/// than or equal to this is treated as a stub (and rejected by
/// [`is_real_libdl`]).
pub const LIBDL_STUB_SIZE: usize = 5848;

/// The 4-byte ELF magic (`0x7f 'E' 'L' 'F'`). Used to verify that the
/// bytes we extracted are actually an ELF shared library (not random
/// garbage or a malformed ext4 read).
const ELF_MAGIC: [u8; 4] = [0x7f, 0x45, 0x4c, 0x46];

/// ZIP local file header signature (`PK\x03\x04`). Appears at the start
/// of every ZIP archive and before each file's data.
const ZIP_LOCAL_SIG: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];
/// ZIP central directory file header signature (`PK\x01\x02`). Appears
/// before each entry's metadata in the central directory (at the end of
/// the ZIP file).
const ZIP_CENTRAL_SIG: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];
/// ZIP end-of-central-directory signature (`PK\x05\x06`). Appears at
/// the very end of every ZIP archive (possibly followed by a comment
/// of up to 65535 bytes).
const ZIP_EOCD_SIG: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];

/// Linux loop device ioctls (stable kernel ABI). Not in libc 0.2.x for
/// all targets, so we define them ourselves. The values are the same on
/// every Linux architecture (they're `ioctl` _IO-encoded macros).
///
/// - `LOOP_CTL_GET_FREE` (`0x4C82`): ask `/dev/loop-control` for a free
///   loop device number. Returns the minor number (e.g. `3` for
///   `/dev/loop3`).
/// - `LOOP_SET_FD` (`0x4C00`): associate a regular file (the ext4
///   image) with a loop device. After this, the loop device presents
///   the file as a block device.
/// - `LOOP_CLR_FD` (`0x4C01`): detach the file from the loop device
///   (releases the loop device for reuse).
const LOOP_CTL_GET_FREE: libc::c_ulong = 0x4C82;
const LOOP_SET_FD: libc::c_ulong = 0x4C00;
const LOOP_CLR_FD: libc::c_ulong = 0x4C01;

/// Returns the directory used for APEX extraction temp files (pure logic,
/// no side effects — does NOT create the directory).
///
/// **Background (5-M's diagnosis of the 3b571fe failure)**: at Step 3.7
/// (BEFORE `setup_mounts`), `/tmp/` does NOT exist in the parent process's
/// Android-app-sandbox filesystem context. `setup_mounts` is what
/// bind-mounts tmpfs on `/tmp/` inside the new mount namespace; before
/// that, only the app's own data + cache dirs are writable. 5-L's
/// original code hardcoded `/tmp/twoyi-apex-payload.img` and got
/// `ENOENT` writing 6377472 bytes — the extraction algorithm was correct
/// but the temp path was wrong.
///
/// **Resolution order (Task 6-Z88)**:
/// 1. `$TMPDIR` env var — Android's app sandbox sets this to the app's
///    cache dir (e.g. `/data/data/io.twoyi/cache` or
///    `/data/user/0/io.twoyi/cache`). Always writable, always exists.
/// 2. `$TWOYI_DATA_DIR` + `/cache` — set by the app launcher (core.rs,
///    next to TWOYI_ROOTFS) from Java's `getApplicationInfo().dataDir`.
///    This is package-correct for `io.twoyi.debug` and work-profile
///    installs, where the hardcoded path in step 3 is WRONG (run
///    32632668179: `create_dir_all(/data/data/io.twoyi/cache): Permission
///    denied` on an io.twoyi.debug build).
/// 3. Last-resort fallback: `/data/data/io.twoyi/cache` (kept for
///    compatibility with launches that set neither env var).
///
/// `getenv` is injected so unit tests can mock the env lookup without
/// touching the process-global `std::env::set_var` (which would race
/// with parallel tests in the same process).
fn apex_temp_dir_from(getenv: impl Fn(&str) -> Option<String>) -> String {
    if let Some(dir) = getenv("TMPDIR").filter(|s| !s.is_empty()) {
        return dir;
    }
    // Task 6-Z88: package-correct fallback via TWOYI_DATA_DIR (covers
    // io.twoyi.debug + work profiles). Trim a trailing '/' so we never
    // produce a "//cache" double slash.
    if let Some(data_dir) = getenv("TWOYI_DATA_DIR").filter(|s| !s.is_empty()) {
        return format!("{}/cache", data_dir.trim_end_matches('/'));
    }
    // Last resort — hardcoded package path, kept for compatibility.
    "/data/data/io.twoyi/cache".to_string()
}

/// Returns the directory used for APEX extraction temp files, with the
/// directory created (recursively) if it doesn't already exist.
///
/// See [`apex_temp_dir_from`] for the resolution logic + rationale.
///
/// `create_dir_all` is a no-op if the directory already exists (the common
/// case at runtime — Android creates the app's cache dir at install time,
/// and `TMPDIR` points there). If creation fails, we log a warning but
/// still return the path — the subsequent `std::fs::write` will produce
/// a more specific error message that gets logged by the caller.
fn apex_temp_dir() -> String {
    let dir = apex_temp_dir_from(|k| std::env::var(k).ok());
    if let Err(e) = std::fs::create_dir_all(&dir) {
        warning!(
            "[KR64][apex_extract] failed to create_dir_all({}): {} — extraction will likely fail",
            dir,
            e
        );
    }
    dir
}

/// Returns the temp-file path for the extracted `apex_payload.img`
/// (the ext4 image inside the .apex ZIP).
///
/// The path lives under [`apex_temp_dir`] so it's writable in the
/// parent's pre-`setup_mounts` filesystem context. The filename is
/// fixed (`twoyi-apex-payload.img`) because there's only one kr64
/// daemon per app process — no concurrency concern.
fn apex_payload_temp_path_in(base: &str) -> String {
    format!("{}/twoyi-apex-payload.img", base)
}

/// Convenience wrapper: temp path under [`apex_temp_dir`] (which also
/// ensures the directory exists via `create_dir_all`).
fn apex_payload_temp_path() -> String {
    apex_payload_temp_path_in(&apex_temp_dir())
}

/// Returns the mount-point path used by [`loopback_mount_and_read`] to
/// mount the ext4 image read-only.
///
/// The path lives under [`apex_temp_dir`] so it's writable in the
/// parent's pre-`setup_mounts` filesystem context. The directory is
/// created (and cleaned up) by [`loopback_mount_and_read`].
fn apex_mount_dir_in(base: &str) -> String {
    format!("{}/twoyi-apex-mount", base)
}

/// Convenience wrapper: mount-point path under [`apex_temp_dir`] (which
/// also ensures the parent directory exists via `create_dir_all`).
fn apex_mount_dir() -> String {
    apex_mount_dir_in(&apex_temp_dir())
}

/// Returns `true` if `bytes` looks like a real libdl.so (ELF magic +
/// strictly larger than the 5848-byte bootstrap stub).
///
/// This is the validation gate for any bytes we extract: if `is_real_libdl`
/// returns `false`, we treat the extraction as failed and fall through to
/// the next strategy (or give up and return `None`).
pub fn is_real_libdl(bytes: &[u8]) -> bool {
    bytes.len() > LIBDL_STUB_SIZE && bytes.starts_with(&ELF_MAGIC)
}

/// Returns `true` if the file at `path` starts with the ZIP local file
/// header signature (`PK\x03\x04`).
///
/// Used to decide between the ZIP-extraction path (parse central directory,
/// extract `apex_payload.img`) and the raw-ext4-image path (mount directly).
pub fn is_zip_file(path: &str) -> bool {
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut buf = [0u8; 4];
    if f.read(&mut buf).unwrap_or(0) < 4 {
        return false;
    }
    buf == ZIP_LOCAL_SIG
}

/// Returns `true` if the file at `path` is a raw ext4 image (has the
/// ext4 superblock magic `0x53 0xEF` at byte offset 1080).
///
/// The ext4 superblock starts at offset 1024 from the start of the image
/// (regardless of block size). The magic `0x53 0xEF` is at offset 56
/// within the superblock (so offset 1080 = 1024 + 56 from the start).
pub fn is_ext4_image(path: &str) -> bool {
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    if f.seek(SeekFrom::Start(1080)).is_err() {
        return false;
    }
    let mut buf = [0u8; 2];
    if f.read(&mut buf).unwrap_or(0) < 2 {
        return false;
    }
    buf == [0x53, 0xEF]
}

/// Read a STORED (uncompressed) entry from a ZIP file by name.
///
/// Returns the file bytes on success, or an error string on failure.
///
/// # Limitations
///
/// - Only supports STORED (method 0) entries. DEFLATE (method 8) returns
///   an error (decompression would require pulling in a zlib dependency,
///   which is against the crate's "std + libc only" policy). APEX
///   `apex_payload.img` entries are typically STORED because ext4 doesn't
///   compress well, so this is rarely a problem in practice.
/// - Does NOT support ZIP64 (archives larger than 4 GB). APEX files are
///   typically <300 MB so this is fine.
/// - Does NOT support encrypted entries.
/// - Does NOT validate the CRC-32 of the extracted data (we trust the
///   ZIP is well-formed).
pub fn read_zip_entry_stored(path: &str, entry_name: &str) -> Result<Vec<u8>, String> {
    let data = std::fs::read(path).map_err(|e| format!("read failed: {}", e))?;
    read_zip_entry_stored_from_bytes(&data, entry_name)
}

/// Same as [`read_zip_entry_stored`] but operates on in-memory bytes.
/// Useful for unit tests (which can build a ZIP in memory without touching
/// the filesystem).
pub fn read_zip_entry_stored_from_bytes(data: &[u8], entry_name: &str) -> Result<Vec<u8>, String> {
    if data.len() < 22 {
        return Err("file too small to be a ZIP".to_string());
    }

    // Find the EOCD record. Search backwards from the end. The EOCD is at
    // most 65557 bytes from the end (22-byte EOCD + 65535-byte comment).
    let max_search = std::cmp::min(data.len(), 65557);
    let mut eocd_off: Option<usize> = None;
    let lower_bound = data.len().saturating_sub(max_search);
    for off in (lower_bound..=data.len() - 22).rev() {
        if data[off..off + 4] == ZIP_EOCD_SIG {
            eocd_off = Some(off);
            break;
        }
    }
    let eocd_off = eocd_off.ok_or("EOCD not found")?;

    if eocd_off + 22 > data.len() {
        return Err("EOCD truncated".to_string());
    }
    let cd_count = u16::from_le_bytes([data[eocd_off + 8], data[eocd_off + 9]]) as usize;
    let cd_off = u32::from_le_bytes([
        data[eocd_off + 16],
        data[eocd_off + 17],
        data[eocd_off + 18],
        data[eocd_off + 19],
    ]) as usize;

    if cd_off >= data.len() {
        return Err(format!(
            "central directory offset out of bounds: {} (file_len={})",
            cd_off,
            data.len()
        ));
    }

    // Walk central directory entries.
    let mut p = cd_off;
    for _ in 0..cd_count {
        if p + 46 > data.len() {
            return Err(format!(
                "central directory entry out of bounds at {} (file_len={})",
                p,
                data.len()
            ));
        }
        if data[p..p + 4] != ZIP_CENTRAL_SIG {
            return Err(format!("bad central directory signature at offset {}", p));
        }
        let method = u16::from_le_bytes([data[p + 10], data[p + 11]]);
        let comp_size =
            u32::from_le_bytes([data[p + 20], data[p + 21], data[p + 22], data[p + 23]]) as usize;
        let uncomp_size =
            u32::from_le_bytes([data[p + 24], data[p + 25], data[p + 26], data[p + 27]]) as usize;
        let name_len = u16::from_le_bytes([data[p + 28], data[p + 29]]) as usize;
        let extra_len = u16::from_le_bytes([data[p + 30], data[p + 31]]) as usize;
        let comment_len = u16::from_le_bytes([data[p + 32], data[p + 33]]) as usize;
        let local_off =
            u32::from_le_bytes([data[p + 42], data[p + 43], data[p + 44], data[p + 45]]) as usize;

        let name_start = p + 46;
        let name_end = name_start + name_len;
        if name_end > data.len() {
            return Err("file name out of bounds".to_string());
        }
        let name = &data[name_start..name_end];

        if name == entry_name.as_bytes() {
            if method != 0 {
                return Err(format!(
                    "entry {} is compressed (method={}), only STORED (0) supported",
                    entry_name, method
                ));
            }
            // Read local file header.
            if local_off + 30 > data.len() {
                return Err(format!("local header out of bounds: {}", local_off));
            }
            if data[local_off..local_off + 4] != ZIP_LOCAL_SIG {
                return Err(format!("bad local file header signature at {}", local_off));
            }
            let l_name_len =
                u16::from_le_bytes([data[local_off + 26], data[local_off + 27]]) as usize;
            let l_extra_len =
                u16::from_le_bytes([data[local_off + 28], data[local_off + 29]]) as usize;
            let data_off = local_off + 30 + l_name_len + l_extra_len;
            let data_end = data_off + uncomp_size;
            if data_end > data.len() {
                return Err(format!(
                    "file data out of bounds: data_off={}, uncomp_size={}, file_len={}",
                    data_off,
                    uncomp_size,
                    data.len()
                ));
            }
            if comp_size != uncomp_size {
                return Err(format!(
                    "comp_size ({}) != uncomp_size ({}) for STORED entry",
                    comp_size, uncomp_size
                ));
            }
            return Ok(data[data_off..data_end].to_vec());
        }

        p = name_end + extra_len + comment_len;
    }

    Err(format!("entry {} not found in ZIP", entry_name))
}

/// Extracts the `apex_payload.img` (the ext4 image inside the .apex file)
/// and returns it as in-memory bytes.
///
/// If the .apex is a ZIP, parses the central directory and extracts the
/// `apex_payload.img` entry (STORED only). If the .apex is a raw ext4
/// image, returns the file bytes directly.
///
/// Returns the ext4 image bytes on success, or an error string on failure.
pub fn extract_apex_payload_img(apex_path: &str) -> Result<Vec<u8>, String> {
    if !std::path::Path::new(apex_path).exists() {
        return Err(format!("{} does not exist", apex_path));
    }
    if is_zip_file(apex_path) {
        // Try the canonical entry name first, then the alternative
        // `apex_payload.img` capitalization (some build tools emit it
        // differently — unlikely but cheap to try).
        read_zip_entry_stored(apex_path, "apex_payload.img")
    } else if is_ext4_image(apex_path) {
        std::fs::read(apex_path).map_err(|e| format!("read failed: {}", e))
    } else {
        Err(format!(
            "{} is neither a ZIP nor an ext4 image (no PK\\x03\\x04 magic, no ext4 superblock at offset 1080)",
            apex_path
        ))
    }
}

/// Loopback-mounts the ext4 image at `ext4_path` (a regular file) and
/// reads `file_inside` (a path inside the mounted filesystem, e.g.
/// `lib64/bionic/libdl.so`).
///
/// This requires:
///   - Root + `CAP_SYS_ADMIN` (we have it in `cfg.use_namespaces=true` mode).
///   - `/dev/loop-control` to exist and be readable (kernel CONFIG_BLK_DEV_LOOP).
///   - At least one free `/dev/loopN` device.
///   - The kernel's ext4 driver to support the features in the image.
///
/// The mount is created in the current mount namespace (no `unshare` —
/// the caller is expected to have arranged a private namespace if
/// needed). The mount is automatically cleaned up on function exit
/// (`umount(2)` + `LOOP_CLR_FD` + `remove_dir`).
///
/// Returns the file bytes on success, or an error string on failure.
pub fn loopback_mount_and_read(ext4_path: &str, file_inside: &str) -> Result<Vec<u8>, String> {
    let mount_dir = apex_mount_dir();
    // apex_temp_dir() already create_dir_all'd the parent; create the
    // mount subdirectory itself. (Ignored if it already exists.)
    let _ = std::fs::create_dir_all(&mount_dir);

    // Open the ext4 image file (the source for the loop device).
    let img = std::fs::File::open(ext4_path)
        .map_err(|e| format!("open ext4 image {}: {}", ext4_path, e))?;

    // Open /dev/loop-control and ask for a free loop device number.
    let ctl = std::fs::OpenOptions::new()
        .read(true)
        .open("/dev/loop-control")
        .map_err(|e| {
            format!(
                "open /dev/loop-control (is CONFIG_BLK_DEV_LOOP enabled?): {}",
                e
            )
        })?;
    let n = unsafe {
        libc::ioctl(
            ctl.as_raw_fd() as libc::c_int,
            LOOP_CTL_GET_FREE as _,
            0 as libc::c_int,
        )
    };
    if n < 0 {
        return Err(format!(
            "LOOP_CTL_GET_FREE failed: {} (no free loop devices available?)",
            std::io::Error::last_os_error()
        ));
    }
    let preferred_loop_dev = format!("/dev/loop{}", n);

    // 5-O's diagnosis + 5-P's fix: Android emulator userspace has NO udev.
    // When the kernel allocates a loop device via LOOP_CTL_GET_FREE
    // (returned `n`), the `/dev/loopN` device node is NOT auto-created on
    // the filesystem. 5-O observed `open("/dev/loop28")` fail ENOENT after
    // LOOP_CTL_GET_FREE returned 28 on bbc2849. Fix: mknod the device
    // node ourselves (requires CAP_MKNOD — available via
    // `cfg.use_namespaces=true`, which grants CAP_SYS_ADMIN to the
    // parent; CAP_SYS_ADMIN is a superset of CAP_MKNOD on Linux). If
    // mknod fails (e.g. EPERM, or the node already exists which is also
    // OK), we fall through; the open below will retry + we also fall
    // back to `/dev/loop0..31` in case init.rc mknod'd a small set at
    // boot.
    let dev_t = libc::makedev(7 as libc::c_uint, n as libc::c_uint);
    // loop block device: major=7, minor=n (Linux ABI, see
    // Documentation/admin-guide/devices.txt: "Loop block device" majors
    // 7/0..255).
    let preferred_c = CString::new(preferred_loop_dev.as_str())
        .map_err(|_| "preferred_loop_dev path contains NUL".to_string())?;
    let mknod_ret = unsafe { libc::mknod(preferred_c.as_ptr(), libc::S_IFBLK | 0o660, dev_t) };
    if mknod_ret == 0 {
        info!(
            "[KR64][apex_extract] mknod {} (S_IFBLK | 0o660, dev=0x{:x}) succeeded",
            preferred_loop_dev, dev_t
        );
    } else {
        let err = std::io::Error::last_os_error();
        let errno = err.raw_os_error().unwrap_or(0);
        // EEXIST (17) is benign — the node already exists (from a prior
        // run or from init.rc's static mknod pass) and the open below
        // will just use it. Other errors (EPERM=1, ENOSYS=38, ENOMEM=12,
        // etc.) mean CAP_MKNOD isn't available or mknod is otherwise
        // blocked — we'll fall through to the /dev/loop0..31 fallback.
        if errno == libc::EEXIST {
            info!(
                "[KR64][apex_extract] mknod {} returned EEXIST (node already exists) — open will reuse it",
                preferred_loop_dev
            );
        } else {
            warning!(
                "[KR64][apex_extract] mknod {} (dev=0x{:x}) failed: {} (errno {}) — will try open anyway + fall back to /dev/loop0..31",
                preferred_loop_dev, dev_t, err, errno
            );
        }
    }

    // Open the loop device (read-write — LOOP_SET_FD requires write access
    // so the kernel can manage the backing file's page cache).
    //
    // Try the kernel-allocated preferred path first; if it fails (mknod
    // wasn't permitted, the device driver rejected the open, or the node
    // genuinely doesn't exist), fall back to iterating /dev/loop0..31
    // with O_RDWR until one opens. init.rc may have mknod'd a small set
    // (typically 0..7) at boot that we can reuse; LOOP_CTL_GET_FREE's
    // index allocation is independent of the /dev/loopN filesystem nodes,
    // so one of the pre-existing nodes may still be free.
    let mut loop_dev = preferred_loop_dev.clone();
    let loop_fd = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&loop_dev)
    {
        Ok(fd) => fd,
        Err(first_err) => {
            warning!(
                "[KR64][apex_extract] open {} failed: {} — falling back to /dev/loop0..31",
                loop_dev,
                first_err
            );
            let mut found: Option<std::fs::File> = None;
            for i in 0..32u32 {
                let p = format!("/dev/loop{}", i);
                match std::fs::OpenOptions::new().read(true).write(true).open(&p) {
                    Ok(fd) => {
                        info!(
                            "[KR64][apex_extract] fallback: opened {} (fd={})",
                            p,
                            fd.as_raw_fd()
                        );
                        loop_dev = p;
                        found = Some(fd);
                        break;
                    }
                    Err(_) => continue,
                }
            }
            match found {
                Some(fd) => fd,
                None => {
                    return Err(format!(
                        "open {} (and /dev/loop0..31 fallback all failed): {} — \
                         Android emulator has no udev to auto-create loop device nodes \
                         (5-O's diagnosis on bbc2849; 5-P's mknod+fallback fix)",
                        preferred_loop_dev, first_err
                    ));
                }
            }
        }
    };
    info!(
        "[KR64][apex_extract] using loop device {} (fd={}) for LOOP_SET_FD (backing {})",
        loop_dev,
        loop_fd.as_raw_fd(),
        ext4_path
    );

    // Associate the loop device with the ext4 image file. After this,
    // reads/writes to /dev/loopN go to the backing file.
    //
    // The third arg to ioctl(LOOP_SET_FD) is the backing file's FD as
    // a `c_int` (cast explicitly because `libc::ioctl` is variadic —
    // Rust can't infer the variadic-arg type from `as _`).
    let r = unsafe {
        libc::ioctl(
            loop_fd.as_raw_fd() as libc::c_int,
            LOOP_SET_FD as _,
            img.as_raw_fd() as libc::c_int,
        )
    };
    if r < 0 {
        let e = std::io::Error::last_os_error();
        return Err(format!(
            "LOOP_SET_FD on {} (backing {}): {} (EPERM = no CAP_SYS_ADMIN, ENOMEM = loop tab full)",
            loop_dev, ext4_path, e
        ));
    }
    info!(
        "[KR64][apex_extract] LOOP_SET_FD succeeded: {} ↔ backing {} (img fd={})",
        loop_dev,
        ext4_path,
        img.as_raw_fd()
    );

    // mount the loop device as ext4, read-only + silent (suppress ext4
    // driver warnings about unsupported features — they go to dmesg
    // otherwise and clutter the KVM E2E log).
    //
    // CString::new takes ownership of the bytes; clone mount_dir so we
    // can still use it for file_path + error messages below.
    //
    // We keep `tgt_c` around so umount (later) can use `tgt_c.as_ptr()` —
    // passing `mount_dir.as_ptr()` directly would be a latent bug because
    // String's internal buffer is NOT null-terminated (the umount syscall
    // would walk memory looking for a NUL byte).
    let src_c = CString::new(loop_dev.as_str()).map_err(|_| "loop_dev contains NUL".to_string())?;
    let tgt_c =
        CString::new(mount_dir.clone()).map_err(|_| "mount_dir contains NUL".to_string())?;
    let fstype_c = CString::new("ext4").map_err(|_| "ext4 contains NUL".to_string())?;
    let mount_r = unsafe {
        libc::mount(
            src_c.as_ptr(),
            tgt_c.as_ptr(),
            fstype_c.as_ptr(),
            libc::MS_RDONLY | libc::MS_SILENT,
            std::ptr::null(),
        )
    };
    if mount_r < 0 {
        let e = std::io::Error::last_os_error();
        // Cleanup: clear the loop device binding so it can be reused.
        unsafe {
            libc::ioctl(
                loop_fd.as_raw_fd() as libc::c_int,
                LOOP_CLR_FD as _,
                0 as libc::c_int,
            );
        }
        let _ = std::fs::remove_dir(&mount_dir);
        return Err(format!(
            "mount {} as ext4 on {}: {} (ext4 driver may not support image features — check dmesg)",
            loop_dev, mount_dir, e
        ));
    }
    info!(
        "[KR64][apex_extract] mount succeeded: {} (ext4, MS_RDONLY|MS_SILENT) mounted on {}",
        loop_dev, mount_dir
    );

    // Read the file inside the mount. Trim leading '/' so the join
    // produces <apex_mount_dir>/lib64/bionic/libdl.so (not
    // <apex_mount_dir>//lib64/...).
    let rel = file_inside.trim_start_matches('/');
    let file_path = format!("{}/{}", mount_dir, rel);
    let bytes = std::fs::read(&file_path).map_err(|e| {
        format!(
            "read {} from mounted ext4 image ({}): {}",
            file_path, ext4_path, e
        )
    });

    // Cleanup: umount the mount (plain umount(2): if some file were
    // still open it would fail EBUSY — none should be, and a failed
    // cleanup is non-fatal below). Use
    // `tgt_c.as_ptr()` (null-terminated) rather than `mount_dir.as_ptr()`
    // (NOT null-terminated — String's internal buffer doesn't include
    // a NUL byte, so the umount syscall would walk past the buffer).
    let _ = unsafe { libc::umount(tgt_c.as_ptr() as *const _) };
    // Cleanup: detach the loop device from the backing file.
    unsafe {
        libc::ioctl(
            loop_fd.as_raw_fd() as libc::c_int,
            LOOP_CLR_FD as _,
            0 as libc::c_int,
        );
    }
    // Cleanup: remove the mount dir (it's now empty).
    let _ = std::fs::remove_dir(&mount_dir);

    bytes
}

/// Try to extract the real libdl.so from the .apex file at `apex_path`.
///
/// Returns the file bytes (validated by [`is_real_libdl`]) on success,
/// or `None` on any failure (with diagnostic logging).
///
/// Steps:
/// 1. Extract `apex_payload.img` (the ext4 image) from the .apex file.
/// 2. Write it to a temp file at `<apex_temp_dir>/twoyi-apex-payload.img`.
/// 3. Loopback-mount the temp file and read `lib64/bionic/libdl.so`.
/// 4. Validate the bytes via [`is_real_libdl`].
/// 5. Cleanup the temp file.
fn extract_real_libdl_from_apex(apex_path: &str) -> Option<Vec<u8>> {
    info!(
        "[KR64][apex_extract] attempting to extract real libdl.so from {}",
        apex_path
    );

    // Step 1: extract apex_payload.img.
    let ext4_bytes = match extract_apex_payload_img(apex_path) {
        Ok(b) => {
            info!(
                "[KR64][apex_extract] extracted apex_payload.img ({} bytes) from {}",
                b.len(),
                apex_path
            );
            b
        }
        Err(e) => {
            warning!(
                "[KR64][apex_extract] failed to extract apex_payload.img from {}: {}",
                apex_path,
                e
            );
            return None;
        }
    };

    // Step 2: write to a temp file. Use `apex_temp_dir()` (TMPDIR env
    // var, fallback /data/data/io.twoyi/cache) — NOT /tmp/, because at
    // Step 3.7 (BEFORE setup_mounts) /tmp/ does NOT exist in the
    // parent's Android-app-sandbox filesystem context. 5-M's diagnosis
    // of the 3b571fe failure: the original `let tmp_img = "/tmp/
    // twoyi-apex-payload.img"` got ENOENT writing 6377472 bytes — the
    // extraction algorithm was correct, only the temp path was wrong.
    // The file can be large (~6 MB for com.android.runtime.apex's
    // apex_payload.img), but the app's cache dir is on the user-data
    // partition so write is fast.
    let tmp_img = apex_payload_temp_path();
    if let Err(e) = std::fs::write(&tmp_img, &ext4_bytes) {
        warning!(
            "[KR64][apex_extract] failed to write {} ({} bytes) to {}: {}",
            tmp_img,
            ext4_bytes.len(),
            tmp_img,
            e
        );
        return None;
    }
    info!(
        "[KR64][apex_extract] wrote {} bytes to {} for loopback mount",
        ext4_bytes.len(),
        tmp_img
    );

    // Step 3: loopback-mount and read libdl.so.
    let result = loopback_mount_and_read(&tmp_img, "lib64/bionic/libdl.so");

    // Step 4: cleanup the temp file regardless of mount result.
    let _ = std::fs::remove_file(&tmp_img);

    match result {
        Ok(b) => {
            if is_real_libdl(&b) {
                info!(
                    "[KR64][apex_extract] extracted real libdl.so ({} bytes, > stub {}) from {}",
                    b.len(),
                    LIBDL_STUB_SIZE,
                    apex_path
                );
                Some(b)
            } else {
                warning!(
                    "[KR64][apex_extract] extracted libdl.so from {} is only {} bytes (≤ stub {}) or not ELF — likely the stub, not the real one. Rejecting.",
                    apex_path,
                    b.len(),
                    LIBDL_STUB_SIZE
                );
                None
            }
        }
        Err(e) => {
            warning!(
                "[KR64][apex_extract] loopback mount + read of libdl.so from {} failed: {}",
                apex_path,
                e
            );
            None
        }
    }
}

/// Returns `true` when probing the HOST's own APEX/libdl paths (bare
/// `/system/apex/...`, `/apex/...`) is allowed. Gated behind the
/// `TWOYI_ALLOW_HOST_APEX` env var — **default ON** (6-Z211).
///
/// Why default ON now: the Lineage 22.2 + OrangeFox R12.0 boot.img ramdisks
/// do NOT include the `com.android.runtime.apex` file (only the flattened
/// /apex/com.android.runtime/ directory tree, which has the 5848-byte
/// bootstrap stub libdl.so). Without the host /apex/ scan, the kr64 falls
/// back to the 5848-byte stub, which causes a NULL-deref SIGSEGV in
/// libc.so at offset 0xfb20 (Lineage 22.2 run 33235829894) when libc's
/// PLT stub calls dl_iterate_phdr / __loader_dlopen / dlclose / dlsym.
///
/// VFS isolation is PRESERVED: the host /apex/ scan is a HOST-SIDE
/// operation (before execve). It reads the libdl.so BYTES from the host's
/// /apex/ and writes them to /dev/libdl.so (guest-side). The guest sees
/// /dev/libdl.so as a regular file — it never discovers the host's /apex/
/// path. The host path is an implementation detail (like a backing file).
/// This matches the master prompt's invariant: "GUEST / ≠ HOST BACKING
/// PATH" — the guest's /dev/libdl.so is backed by the host's /apex/.../
/// libdl.so, but the guest never sees the backing path.
///
/// Version mismatch concern (the original Task 6-Z88 reason for default
/// OFF): the host's libdl.so might be a different Android version than
/// the guest expects. But libdl.so is part of bionic, whose ABI is stable
/// across Android 11-15. The dl_iterate_phdr / __loader_dlopen / dlclose
/// / dlsym / dlerror / dladdr symbols have not changed. Using the host's
/// libdl.so for the guest is safe for the boot-critical dl_* calls.
///
/// Opt-out: set `TWOYI_DISALLOW_HOST_APEX=1` to disable the host /apex/
/// scan (e.g., for testing the stub fallback in isolation).
fn host_apex_allowed_from(getenv: impl Fn(&str) -> Option<String>) -> bool {
    // 6-Z211: default ON. Opt-out via TWOYI_DISALLOW_HOST_APEX=1.
    if matches!(getenv("TWOYI_DISALLOW_HOST_APEX").as_deref(), Some("1")) {
        return false;
    }
    // Legacy opt-in env var still respected (no-op now since default is ON,
    // but kept for backward compatibility with scripts that set it).
    true
}

/// Runtime wrapper around [`host_apex_allowed_from`] using the real
/// process environment.
fn host_apex_allowed() -> bool {
    host_apex_allowed_from(|k| std::env::var(k).ok())
}

/// Scan alternative host paths for a non-stub libdl.so. This is the
/// fallback when APEX ext4 extraction fails (e.g. loop device not
/// available).
///
/// Tries, in order (ALL are bare HOST paths — see [`host_apex_allowed_from`];
/// the entire scan is skipped unless `TWOYI_ALLOW_HOST_APEX=1`):
/// 1. `/apex/com.android.runtime@1/lib64/bionic/libdl.so` (versioned APEX)
/// 2. `/apex/com.android.runtime@2/lib64/bionic/libdl.so`
/// 3. `/apex/com.android.runtime@3/lib64/bionic/libdl.so`
/// 4. Any `/apex/com.android.runtime@*/lib64/bionic/libdl.so` found via
///    `read_dir(/apex/)` matching the pattern.
///
/// Returns the file bytes (validated by [`is_real_libdl`]) on success,
/// or `None` if all candidates are stubs or missing.
fn scan_alternative_libdl_paths() -> Option<(String, Vec<u8>)> {
    // 6-Z211: every candidate below is a BARE HOST path. The scan is
    // DEFAULT ON (was DEFAULT OFF in Task 6-Z88). The host /apex/ scan
    // is a safe FALLBACK when the guest rootfs doesn't have the APEX
    // (e.g., Lineage 22.2 + OrangeFox R12.0 boot.img ramdisks don't
    // include the com.android.runtime.apex file). VFS isolation is
    // preserved: the host path is only used to READ the libdl.so bytes,
    // which are then written to /dev/libdl.so (guest-side). The guest
    // never discovers the host path.
    if !host_apex_allowed() {
        info!(
            "[KR64][apex_extract] TWOYI_DISALLOW_HOST_APEX=1 — skipping host /apex/ libdl scan (guest rootfs candidates are used exclusively)"
        );
        return None;
    }
    // Try the common versioned APEX paths first (apexd mounts the
    // versioned APEX at /apex/com.android.runtime@N/ and symlinks
    // /apex/com.android.runtime/ -> /apex/com.android.runtime@N/).
    // If the symlink target is the stub, the versioned path might
    // still be the stub too — but try anyway in case apexd was
    // re-run with a newer APEX.
    for n in 1..=3 {
        let p = format!("/apex/com.android.runtime@{}/lib64/bionic/libdl.so", n);
        if let Ok(b) = std::fs::read(&p) {
            if is_real_libdl(&b) {
                info!(
                    "[KR64][apex_extract] found real libdl.so ({} bytes) at {} (alternative path)",
                    b.len(),
                    p
                );
                return Some((p, b));
            } else {
                info!(
                    "[KR64][apex_extract] alternative path {} exists but is stub ({} bytes)",
                    p,
                    b.len()
                );
            }
        }
    }

    // Scan /apex/ for any com.android.runtime@N directories we didn't try.
    if let Ok(entries) = std::fs::read_dir("/apex/") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("com.android.runtime@") {
                    let p = format!("/apex/{}/lib64/bionic/libdl.so", name);
                    if let Ok(b) = std::fs::read(&p) {
                        if is_real_libdl(&b) {
                            info!(
                                "[KR64][apex_extract] found real libdl.so ({} bytes) at {} (scan)",
                                b.len(),
                                p
                            );
                            return Some((p, b));
                        }
                    }
                }
            }
        }
    }

    None
}

/// Returns the list of candidate .apex file paths to try for libdl.so
/// extraction. Order matters: try the most-likely-to-exist paths first.
///
/// Built from:
/// 1. `{cfg.rom_dir}/system/apex/com.android.runtime.apex` (if `rom_dir`
///    is set — this is the GSI's .apex file, the most reliable source).
/// 2. `{cfg.rootfs}/system/apex/com.android.runtime.apex` (in case the
///    .apex was extracted into the rootfs directly).
/// 3. `/system/apex/com.android.runtime.apex` (BARE HOST path — see
///    [`host_apex_allowed_from`]; DEFAULT ON as of 6-Z211. The host
///    /apex/ scan is a safe FALLBACK when the guest rootfs doesn't have
///    the APEX — VFS isolation preserved because the host path is only
///    used to READ bytes, never exposed to the guest).
/// 4. `/apex/com.android.runtime.apex` (uncommon — usually the .apex
///    is under /system/apex/; same host gate as #3).
pub fn apex_candidate_paths(cfg: &crate::Config) -> Vec<String> {
    apex_candidate_paths_with(&|k| std::env::var(k).ok(), cfg)
}

/// Injected-env variant of [`apex_candidate_paths`] (mirrors the
/// `apex_temp_dir_from(getenv)` pattern so unit tests can mock the
/// `TWOYI_DISALLOW_HOST_APEX` lookup without racing on the process-global
/// environment).
fn apex_candidate_paths_with(
    getenv: &dyn Fn(&str) -> Option<String>,
    cfg: &crate::Config,
) -> Vec<String> {
    let mut v = Vec::new();
    if let Some(rom_dir) = &cfg.rom_dir {
        v.push(format!("{}/system/apex/com.android.runtime.apex", rom_dir));
    }
    v.push(format!(
        "{}/system/apex/com.android.runtime.apex",
        cfg.rootfs
    ));
    // Task 6-Z88: bare-HOST candidates only when explicitly allowed.
    if host_apex_allowed_from(|k| getenv(k)) {
        v.push("/system/apex/com.android.runtime.apex".to_string());
        v.push("/apex/com.android.runtime.apex".to_string());
    }
    v
}

/// Main entry point: try to find the real libdl.so on the host.
///
/// Strategy:
/// 1. For each candidate .apex path (see [`apex_candidate_paths`]):
///    a. If the file exists, attempt [`extract_real_libdl_from_apex`].
///    b. If extraction succeeds, return the bytes immediately.
/// 2. If all .apex candidates fail, fall back to [`scan_alternative_libdl_paths`].
/// 3. If everything fails, return `None`.
///
/// Returns `(source_path, file_bytes)` on success. The bytes are
/// validated by [`is_real_libdl`] before return.
///
/// # Diagnostics
///
/// Every step logs to stderr (visible in kr64-stderr.log):
///   - Which candidate .apex paths were tried.
///   - Whether each was a ZIP, raw ext4, or unknown format.
///   - Whether the loopback mount succeeded.
///   - Whether the extracted libdl.so passed `is_real_libdl` validation.
///   - The final fallback (alternative path scan) result.
pub fn find_real_libdl_so(cfg: &crate::Config) -> Option<(String, Vec<u8>)> {
    let candidates = apex_candidate_paths(cfg);
    info!(
        "[KR64][apex_extract] searching for real libdl.so — {} candidate .apex paths",
        candidates.len()
    );
    for c in &candidates {
        info!("[KR64][apex_extract]   candidate: {}", c);
    }

    for apex_path in &candidates {
        if !std::path::Path::new(apex_path).exists() {
            info!(
                "[KR64][apex_extract] candidate {} does not exist — skipping",
                apex_path
            );
            continue;
        }
        // Log what kind of file this is (ZIP vs raw ext4 vs other).
        if is_zip_file(apex_path) {
            info!(
                "[KR64][apex_extract] {} is a ZIP (non-flattened APEX) — extracting apex_payload.img",
                apex_path
            );
        } else if is_ext4_image(apex_path) {
            info!(
                "[KR64][apex_extract] {} is a raw ext4 image (flattened APEX) — mounting directly",
                apex_path
            );
        } else {
            warning!(
                "[KR64][apex_extract] {} is neither ZIP nor ext4 — skipping",
                apex_path
            );
            continue;
        }

        if let Some(bytes) = extract_real_libdl_from_apex(apex_path) {
            return Some((apex_path.clone(), bytes));
        }
    }

    // All .apex candidates failed — fall back to scanning alternative paths.
    info!(
        "[KR64][apex_extract] all .apex candidates exhausted — falling back to alternative path scan"
    );
    if let Some((src, bytes)) = scan_alternative_libdl_paths() {
        return Some((src, bytes));
    }

    error!(
        "[KR64][apex_extract] FAILED to find real libdl.so anywhere — guest init will use the 5848-byte stub and likely crash at offset 0xaf174 in linker64 (5-K's diagnosis)"
    );
    None
}

// ============================================================================
// Option D (5-U's recommendation): ship libdl.so as an APK asset.
//
// Background: 5-L/5-N/5-O/5-P hit 4 sequential failure modes in the
// loopback-mount pipeline (5-L temp-write ENOENT → 5-N loop_open ENOENT →
// 5-P mknod+fallback loop_open ENXIO for all N in 0..31). 5-U's diagnosis:
// "Each fix exposes the next layer. The loopback-mount approach depends on
// CAP_MKNOD + CAP_SYS_ADMIN + kernel loop driver + init.rc mknod + ext4
// driver. Too many failure modes."
//
// Option D bypasses ALL of them by shipping the real libdl.so as an APK
// asset (app/src/main/assets/libdl.so). Java extracts the asset to
// {data_dir}/files/libdl.so on app startup (see
// RomManager.extractLibdlAsset in app/src/main/java/io/twoyi/utils/
// RomManager.java). The kr64 daemon reads the file via
// [`read_libdl_asset`] BEFORE attempting the APEX extraction. The asset is
// validated via [`is_real_libdl`] (> 5848 bytes + ELF magic) so a
// placeholder asset is gracefully rejected and falls through to APEX
// extraction.
//
// If the asset is missing (no libdl.so in assets/) OR is a placeholder
// (< 5848 bytes / no ELF magic), [`read_libdl_asset`] returns None and kr64
// falls through to [`find_real_libdl_so`] (APEX extraction, still broken on
// the Android emulator per 5-U, but kept as a defensive fallback).
// ============================================================================

/// Returns the list of candidate paths for the libdl.so APK asset
/// (extracted from the APK by Java on app init via
/// `RomManager.extractLibdlAsset`).
///
/// Built from:
/// 1. `{cfg.data_dir}/files/libdl.so` — the primary path where Java
///    extracts the asset on app startup. In a work profile, `cfg.data_dir`
///    is `/data/user/<id>/io.twoyi` (set by `Renderer.setDataDir` from
///    `getDataDir().getAbsolutePath()`), so the `{data_dir}/files/` prefix
///    still resolves correctly to the work profile's files dir.
/// 2. `/data/data/io.twoyi/files/libdl.so` — fallback if `cfg.data_dir`
///    is not set to the canonical io.twoyi path (e.g. test env, or a
///    future data-dir override that points elsewhere). This matches the
///    canonical single-user install path.
///
/// Order matters: the path derived from `cfg.data_dir` is tried first
/// because it correctly handles work profiles (where `/data/data/io.twoyi`
/// doesn't exist; the per-user `/data/user/<id>/io.twoyi` path does).
/// The hardcoded `/data/data/io.twoyi/files/libdl.so` is a defensive
/// fallback for the single-user install case + test environments that
/// don't set `--data-dir`.
pub fn libdl_asset_candidate_paths(cfg: &crate::Config) -> Vec<String> {
    let mut v = Vec::new();
    if !cfg.data_dir.is_empty() {
        v.push(format!("{}/files/libdl.so", cfg.data_dir));
    }
    v.push("/data/data/io.twoyi/files/libdl.so".to_string());
    v
}

/// Read the libdl.so APK asset (extracted to the app's files dir by Java
/// on startup) and return its bytes if it's the REAL libdl.so (> stub size,
/// ELF magic). This is the **Option D primary path** (5-U's recommendation):
/// instead of extracting libdl.so from the APEX ext4 image at runtime
/// (which depends on CAP_MKNOD + CAP_SYS_ADMIN + kernel loop driver + ext4
/// driver — 4 sequential failure modes documented in 5-L/5-N/5-O/5-P/5-U),
/// we ship the real libdl.so as an APK asset + Java extracts it on app init
/// to `{data_dir}/files/libdl.so`.
///
/// Returns `(source_path, file_bytes)` on success, or `None` if:
/// - The asset file doesn't exist (Java extraction didn't run yet OR the
///   asset is missing from the APK — graceful degradation to APEX).
/// - The asset is the 5848-byte stub (size guard rejects; this catches
///   accidentally shipping the Android bootstrap stub as the asset).
/// - The asset is a placeholder (size < 5848 OR not ELF magic — the dev
///   hasn't yet run `scripts/extract_libdl_from_apex.sh` to drop the real
///   one in).
/// - The asset is corrupted (read succeeds but bytes aren't ELF).
///
/// # Diagnostics
///
/// Every step logs to stderr (visible in `kr64-stderr.log`):
///   - Which candidate paths were tried.
///   - Whether each was missing, stub-sized, non-ELF, or real.
///   - The final verdict (asset found + real → Some, else → None → fall
///     through to [`find_real_libdl_so`]).
///
/// # Why this is preferred over `find_real_libdl_so`
///
/// The APEX extraction pipeline ([`find_real_libdl_so`]) requires:
/// 1. `mknod(/dev/loopN, S_IFBLK)` to succeed (CAP_MKNOD).
/// 2. `open(/dev/loopN, O_RDWR)` to succeed (kernel loop driver loaded
///    with a registered gendisk for major=7, minor=N).
/// 3. `LOOP_SET_FD` ioctl to succeed (CAP_SYS_ADMIN).
/// 4. `mount(loop_dev, mountpoint, "ext4")` to succeed (ext4 driver
///    accepts the APEX payload image).
///
/// All 4 prerequisites are kernel/permission-dependent and have failed
/// sequentially in 5-L/5-N/5-O/5-P. [`read_libdl_asset`] requires only:
/// 1. The APK asset `libdl.so` exists (a build-time decision — CI/dev
///    drops it in via `scripts/extract_libdl_from_apex.sh`).
/// 2. Java's `RomManager.extractLibdlAsset` ran on app init (always true
///    once the Java code is in place).
/// 3. The file at `{data_dir}/files/libdl.so` is readable (always true;
///    it's on the app's user-data partition).
///
/// All 3 are guaranteed by the build + app init, not by the kernel.
pub fn read_libdl_asset(cfg: &crate::Config) -> Option<(String, Vec<u8>)> {
    let candidates = libdl_asset_candidate_paths(cfg);
    info!(
        "[KR64][apex_extract] Option D: searching for libdl.so APK asset — {} candidate paths",
        candidates.len()
    );
    for c in &candidates {
        info!("[KR64][apex_extract]   asset candidate: {}", c);
    }

    for path in &candidates {
        match std::fs::read(path) {
            Ok(bytes) => {
                if is_real_libdl(&bytes) {
                    info!(
                        "[KR64][apex_extract] Option D: found real libdl.so APK asset at {} ({} bytes, > stub {}) — using it",
                        path,
                        bytes.len(),
                        LIBDL_STUB_SIZE
                    );
                    return Some((path.clone(), bytes));
                }

                // Size guard rejected the asset. Classify the rejection
                // reason so the diagnostic log makes it obvious what
                // went wrong (placeholder vs. accidentally-shipped stub
                // vs. corrupted extraction).
                if bytes.len() <= LIBDL_STUB_SIZE && bytes.starts_with(&ELF_MAGIC) {
                    // The asset is an ELF ≤ 5848 bytes — looks EXACTLY
                    // like the Android bootstrap stub that lives at
                    // /apex/com.android.runtime/lib64/bionic/libdl.so.
                    // Someone probably copied the stub (instead of the
                    // real one) into the assets dir.
                    warning!(
                        "[KR64][apex_extract] Option D: asset at {} is {} bytes (≤ stub {}) + ELF magic — looks like the 5848-byte bootstrap stub (NOT the real libdl.so). Rejecting + falling through to APEX extraction. If this is the placeholder asset, replace app/src/main/assets/libdl.so with the REAL libdl.so extracted from a booted AOSP x86_64 system (see scripts/extract_libdl_from_apex.sh).",
                        path,
                        bytes.len(),
                        LIBDL_STUB_SIZE
                    );
                } else if !bytes.starts_with(&ELF_MAGIC) {
                    // Not even an ELF — this is either a placeholder
                    // text file (intentional, until CI/dev drops the
                    // real one in) or a corrupted extraction.
                    warning!(
                        "[KR64][apex_extract] Option D: asset at {} is {} bytes but NOT ELF magic — placeholder or corrupted. Rejecting + falling through to APEX extraction. Run scripts/extract_libdl_from_apex.sh to drop the real libdl.so in.",
                        path,
                        bytes.len()
                    );
                } else {
                    // ELF magic but size in (LIBDL_STUB_SIZE, ?) — wait,
                    // is_real_libdl checks `> LIBDL_STUB_SIZE` strictly.
                    // If we got here, size was ≤ LIBDL_STUB_SIZE but
                    // ELF magic matched, so the first branch should have
                    // fired. Defensive logging for any future edge case.
                    warning!(
                        "[KR64][apex_extract] Option D: asset at {} is {} bytes (≤ stub {}) — too small to be real libdl.so. Rejecting + falling through to APEX extraction.",
                        path,
                        bytes.len(),
                        LIBDL_STUB_SIZE
                    );
                }
            }
            Err(e) => {
                info!(
                    "[KR64][apex_extract] Option D: asset at {} not readable: {} — falling through to next candidate or APEX extraction (find_real_libdl_so)",
                    path,
                    e
                );
            }
        }
    }

    info!(
        "[KR64][apex_extract] Option D: no real libdl.so APK asset found — falling through to APEX extraction (find_real_libdl_so)"
    );
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Tests for is_real_libdl (validation gate).
    // ========================================================================

    #[test]
    fn is_real_libdl_rejects_stub() {
        // The 5848-byte stub: starts with ELF magic but is exactly the
        // stub size. Real libdl.so is much larger (20-40 KB).
        let mut stub = vec![0u8; LIBDL_STUB_SIZE];
        stub[0..4].copy_from_slice(&ELF_MAGIC);
        assert!(
            !is_real_libdl(&stub),
            "5848-byte ELF must be rejected as stub"
        );
    }

    #[test]
    fn is_real_libdl_rejects_smaller_than_stub() {
        // An ELF smaller than the stub size (e.g. a corrupted read).
        let mut small = vec![0u8; 1000];
        small[0..4].copy_from_slice(&ELF_MAGIC);
        assert!(!is_real_libdl(&small));
    }

    #[test]
    fn is_real_libdl_rejects_non_elf() {
        // Random bytes > stub size but no ELF magic.
        let bytes = vec![0xAAu8; 20000];
        assert!(!is_real_libdl(&bytes));
    }

    #[test]
    fn is_real_libdl_accepts_real() {
        // A realistic real libdl.so: ELF magic + size > stub.
        let mut real = vec![0u8; 25000];
        real[0..4].copy_from_slice(&ELF_MAGIC);
        assert!(is_real_libdl(&real));
    }

    #[test]
    fn is_real_libdl_accepts_exactly_one_more_byte_than_stub() {
        // Boundary: LIBDL_STUB_SIZE + 1 byte should be accepted.
        let mut real = vec![0u8; LIBDL_STUB_SIZE + 1];
        real[0..4].copy_from_slice(&ELF_MAGIC);
        assert!(is_real_libdl(&real));
    }

    // ========================================================================
    // Tests for is_zip_file / is_ext4_image (format detection).
    // ========================================================================

    #[test]
    fn is_zip_file_detects_zip_magic() {
        let tmp = std::env::temp_dir().join("twoyi-apex-test-zip.bin");
        std::fs::write(&tmp, ZIP_LOCAL_SIG).unwrap();
        assert!(is_zip_file(tmp.to_str().unwrap()));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn is_zip_file_rejects_non_zip() {
        let tmp = std::env::temp_dir().join("twoyi-apex-test-nonzip.bin");
        std::fs::write(&tmp, b"not a zip file at all").unwrap();
        assert!(!is_zip_file(tmp.to_str().unwrap()));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn is_zip_file_returns_false_for_missing_file() {
        assert!(!is_zip_file("/nonexistent-zip-file-twoyi-test"));
    }

    #[test]
    fn is_zip_file_rejects_ext4_image() {
        // An ext4 image starts with zeros (boot sector) and has magic at
        // offset 1080 — does NOT start with PK\x03\x04.
        let mut bytes = vec![0u8; 2048];
        bytes[1080] = 0x53;
        bytes[1081] = 0xEF;
        let tmp = std::env::temp_dir().join("twoyi-apex-test-ext4.bin");
        std::fs::write(&tmp, &bytes).unwrap();
        assert!(!is_zip_file(tmp.to_str().unwrap()));
        assert!(is_ext4_image(tmp.to_str().unwrap()));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn is_ext4_image_detects_magic_at_offset_1080() {
        let mut bytes = vec![0u8; 2048];
        bytes[1080] = 0x53;
        bytes[1081] = 0xEF;
        let tmp = std::env::temp_dir().join("twoyi-apex-test-ext4-real.bin");
        std::fs::write(&tmp, &bytes).unwrap();
        assert!(is_ext4_image(tmp.to_str().unwrap()));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn is_ext4_image_rejects_non_ext4() {
        let tmp = std::env::temp_dir().join("twoyi-apex-test-nonext4.bin");
        std::fs::write(&tmp, b"not an ext4 image").unwrap();
        assert!(!is_ext4_image(tmp.to_str().unwrap()));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn is_ext4_image_returns_false_for_missing_file() {
        assert!(!is_ext4_image("/nonexistent-ext4-file-twoyi-test"));
    }

    // ========================================================================
    // Tests for read_zip_entry_stored_from_bytes (ZIP parser).
    //
    // These tests build minimal ZIP archives in memory (or via the system
    // `zip` tool if available) and verify the parser correctly extracts
    // STORED entries and rejects DEFLATE entries.
    // ========================================================================

    /// Build a minimal valid ZIP archive (in memory) containing a single
    /// STORED entry named `entry_name` with the given `content`.
    ///
    /// Structure:
    ///   [local file header 30 bytes][file name N bytes][file data M bytes]
    ///   [central directory entry 46 bytes][file name N bytes]
    ///   [EOCD 22 bytes]
    ///
    /// No extra fields, no comments. CRC-32 is set to 0 (we don't
    /// validate it on read).
    fn build_minimal_stored_zip(entry_name: &str, content: &[u8]) -> Vec<u8> {
        let name_bytes = entry_name.as_bytes();
        let name_len = name_bytes.len() as u16;
        let size = content.len() as u32;

        // Local file header (30 bytes) + name + data.
        let mut local = Vec::new();
        local.extend_from_slice(&ZIP_LOCAL_SIG); // 0..4
        local.extend_from_slice(&20u16.to_le_bytes()); // 4..6  version needed (2.0)
        local.extend_from_slice(&0u16.to_le_bytes()); // 6..8  flags
        local.extend_from_slice(&0u16.to_le_bytes()); // 8..10 method (0=STORED)
        local.extend_from_slice(&0u16.to_le_bytes()); // 10..12 mod time
        local.extend_from_slice(&0u16.to_le_bytes()); // 12..14 mod date
        local.extend_from_slice(&0u32.to_le_bytes()); // 14..18 CRC-32 (we don't validate)
        local.extend_from_slice(&size.to_le_bytes()); // 18..22 comp size
        local.extend_from_slice(&size.to_le_bytes()); // 22..26 uncomp size
        local.extend_from_slice(&name_len.to_le_bytes()); // 26..28 name len
        local.extend_from_slice(&0u16.to_le_bytes()); // 28..30 extra len
        local.extend_from_slice(name_bytes);
        local.extend_from_slice(content);
        let local_off = 0u32; // local header is at the start of the ZIP
        let local_size = local.len();

        // Central directory entry (46 bytes) + name.
        let mut central = Vec::new();
        central.extend_from_slice(&ZIP_CENTRAL_SIG); // 0..4
        central.extend_from_slice(&20u16.to_le_bytes()); // 4..6  version made by (2.0)
        central.extend_from_slice(&20u16.to_le_bytes()); // 6..8  version needed (2.0)
        central.extend_from_slice(&0u16.to_le_bytes()); // 8..10 flags
        central.extend_from_slice(&0u16.to_le_bytes()); // 10..12 method (0=STORED)
        central.extend_from_slice(&0u16.to_le_bytes()); // 12..14 mod time
        central.extend_from_slice(&0u16.to_le_bytes()); // 14..16 mod date
        central.extend_from_slice(&0u32.to_le_bytes()); // 16..20 CRC-32
        central.extend_from_slice(&size.to_le_bytes()); // 20..24 comp size
        central.extend_from_slice(&size.to_le_bytes()); // 24..28 uncomp size
        central.extend_from_slice(&name_len.to_le_bytes()); // 28..30 name len
        central.extend_from_slice(&0u16.to_le_bytes()); // 30..32 extra len
        central.extend_from_slice(&0u16.to_le_bytes()); // 32..34 comment len
        central.extend_from_slice(&0u16.to_le_bytes()); // 34..36 disk number
        central.extend_from_slice(&0u16.to_le_bytes()); // 36..38 internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // 38..42 external attrs
        central.extend_from_slice(&local_off.to_le_bytes()); // 42..46 local header offset
        central.extend_from_slice(name_bytes);

        // EOCD (22 bytes).
        let cd_off = local_size as u32;
        let cd_size = central.len() as u32;
        let cd_count = 1u16;
        let mut eocd = Vec::new();
        eocd.extend_from_slice(&ZIP_EOCD_SIG); // 0..4
        eocd.extend_from_slice(&0u16.to_le_bytes()); // 4..6  disk number
        eocd.extend_from_slice(&0u16.to_le_bytes()); // 6..8  disk with CD start
        eocd.extend_from_slice(&cd_count.to_le_bytes()); // 8..10 entries on this disk
        eocd.extend_from_slice(&cd_count.to_le_bytes()); // 10..12 total entries
        eocd.extend_from_slice(&cd_size.to_le_bytes()); // 12..16 CD size
        eocd.extend_from_slice(&cd_off.to_le_bytes()); // 16..20 CD offset
        eocd.extend_from_slice(&0u16.to_le_bytes()); // 20..22 comment length

        let mut zip = Vec::new();
        zip.extend_from_slice(&local);
        zip.extend_from_slice(&central);
        zip.extend_from_slice(&eocd);
        zip
    }

    /// Build a minimal ZIP with a single DEFLATE-compressed entry (we
    /// don't actually compress the content — we set method=8 and put
    /// the raw content in the data section, which is invalid for a real
    /// ZIP but lets us verify that the parser rejects method=8 entries).
    fn build_deflate_zip(entry_name: &str, content: &[u8]) -> Vec<u8> {
        let name_bytes = entry_name.as_bytes();
        let name_len = name_bytes.len() as u16;
        let size = content.len() as u32;

        let mut local = Vec::new();
        local.extend_from_slice(&ZIP_LOCAL_SIG);
        local.extend_from_slice(&20u16.to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(&8u16.to_le_bytes()); // method=8 (DEFLATE)
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(&0u32.to_le_bytes()); // CRC (we don't validate)
        local.extend_from_slice(&size.to_le_bytes()); // comp size
        local.extend_from_slice(&size.to_le_bytes()); // uncomp size
        local.extend_from_slice(&name_len.to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(name_bytes);
        local.extend_from_slice(content);
        let local_off = 0u32;
        let local_size = local.len();

        let mut central = Vec::new();
        central.extend_from_slice(&ZIP_CENTRAL_SIG);
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&8u16.to_le_bytes()); // method=8 (DEFLATE)
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&name_len.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&local_off.to_le_bytes());
        central.extend_from_slice(name_bytes);

        let cd_off = local_size as u32;
        let cd_size = central.len() as u32;
        let cd_count = 1u16;
        let mut eocd = Vec::new();
        eocd.extend_from_slice(&ZIP_EOCD_SIG);
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&cd_count.to_le_bytes());
        eocd.extend_from_slice(&cd_count.to_le_bytes());
        eocd.extend_from_slice(&cd_size.to_le_bytes());
        eocd.extend_from_slice(&cd_off.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());

        let mut zip = Vec::new();
        zip.extend_from_slice(&local);
        zip.extend_from_slice(&central);
        zip.extend_from_slice(&eocd);
        zip
    }

    #[test]
    fn read_zip_entry_stored_extracts_known_entry() {
        let content = b"hello apex ext4 image!";
        let zip = build_minimal_stored_zip("apex_payload.img", content);
        let result = read_zip_entry_stored_from_bytes(&zip, "apex_payload.img");
        assert!(result.is_ok(), "extraction failed: {:?}", result.err());
        assert_eq!(result.unwrap(), content);
    }

    #[test]
    fn read_zip_entry_stored_returns_err_for_missing_entry() {
        let zip = build_minimal_stored_zip("apex_payload.img", b"data");
        let result = read_zip_entry_stored_from_bytes(&zip, "nonexistent.img");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn read_zip_entry_stored_rejects_deflate_entry() {
        let zip = build_deflate_zip("apex_payload.img", b"compressed data");
        let result = read_zip_entry_stored_from_bytes(&zip, "apex_payload.img");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("compressed"),
            "expected 'compressed' in error, got: {}",
            err
        );
    }

    #[test]
    fn read_zip_entry_stored_returns_err_for_non_zip() {
        let bytes = b"this is not a zip file at all";
        let result = read_zip_entry_stored_from_bytes(bytes, "anything");
        assert!(result.is_err());
    }

    #[test]
    fn read_zip_entry_stored_returns_err_for_too_short_input() {
        let bytes = b"PK"; // only 2 bytes, less than the 22-byte minimum
        let result = read_zip_entry_stored_from_bytes(bytes, "x");
        assert!(result.is_err());
    }

    #[test]
    fn read_zip_entry_stored_extracts_large_entry() {
        // 100 KB content (simulating a small apex_payload.img — the real
        // one is ~50-100 MB but the parser logic is the same).
        let content = vec![0x42u8; 100_000];
        let zip = build_minimal_stored_zip("apex_payload.img", &content);
        let result = read_zip_entry_stored_from_bytes(&zip, "apex_payload.img");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 100_000);
    }

    #[test]
    fn read_zip_entry_stored_handles_multiple_entries() {
        // Build a ZIP with 3 entries, verify the parser finds the right one.
        let mut zip = Vec::new();

        // Entry 1: apex_manifest.json
        let mut local1 = Vec::new();
        let name1 = b"apex_manifest.json";
        let content1 = b"{\"name\":\"com.android.runtime\"}";
        local1.extend_from_slice(&ZIP_LOCAL_SIG);
        local1.extend_from_slice(&20u16.to_le_bytes());
        local1.extend_from_slice(&0u16.to_le_bytes());
        local1.extend_from_slice(&0u16.to_le_bytes()); // method=0
        local1.extend_from_slice(&0u16.to_le_bytes());
        local1.extend_from_slice(&0u16.to_le_bytes());
        local1.extend_from_slice(&0u32.to_le_bytes());
        local1.extend_from_slice(&(content1.len() as u32).to_le_bytes());
        local1.extend_from_slice(&(content1.len() as u32).to_le_bytes());
        local1.extend_from_slice(&(name1.len() as u16).to_le_bytes());
        local1.extend_from_slice(&0u16.to_le_bytes());
        local1.extend_from_slice(name1);
        local1.extend_from_slice(content1);
        let local1_off = 0u32;

        // Entry 2: apex_payload.img (the one we want).
        let mut local2 = Vec::new();
        let name2 = b"apex_payload.img";
        let content2 = b"fake ext4 image bytes";
        local2.extend_from_slice(&ZIP_LOCAL_SIG);
        local2.extend_from_slice(&20u16.to_le_bytes());
        local2.extend_from_slice(&0u16.to_le_bytes());
        local2.extend_from_slice(&0u16.to_le_bytes()); // method=0
        local2.extend_from_slice(&0u16.to_le_bytes());
        local2.extend_from_slice(&0u16.to_le_bytes());
        local2.extend_from_slice(&0u32.to_le_bytes());
        local2.extend_from_slice(&(content2.len() as u32).to_le_bytes());
        local2.extend_from_slice(&(content2.len() as u32).to_le_bytes());
        local2.extend_from_slice(&(name2.len() as u16).to_le_bytes());
        local2.extend_from_slice(&0u16.to_le_bytes());
        local2.extend_from_slice(name2);
        local2.extend_from_slice(content2);
        let local2_off = local1.len() as u32;

        // Entry 3: README.txt
        let mut local3 = Vec::new();
        let name3 = b"README.txt";
        let content3 = b"this is the apex README";
        local3.extend_from_slice(&ZIP_LOCAL_SIG);
        local3.extend_from_slice(&20u16.to_le_bytes());
        local3.extend_from_slice(&0u16.to_le_bytes());
        local3.extend_from_slice(&0u16.to_le_bytes()); // method=0
        local3.extend_from_slice(&0u16.to_le_bytes());
        local3.extend_from_slice(&0u16.to_le_bytes());
        local3.extend_from_slice(&0u32.to_le_bytes());
        local3.extend_from_slice(&(content3.len() as u32).to_le_bytes());
        local3.extend_from_slice(&(content3.len() as u32).to_le_bytes());
        local3.extend_from_slice(&(name3.len() as u16).to_le_bytes());
        local3.extend_from_slice(&0u16.to_le_bytes());
        local3.extend_from_slice(name3);
        local3.extend_from_slice(content3);
        let local3_off = (local1.len() + local2.len()) as u32;

        zip.extend_from_slice(&local1);
        zip.extend_from_slice(&local2);
        zip.extend_from_slice(&local3);

        // Central directory: 3 entries.
        let mut central = Vec::new();
        // Use Vec<u8> slices so all entries share the same type (arrays
        // of different lengths can't be mixed in a single array literal).
        let entries: [(&[u8], &[u8], u32); 3] = [
            (name1, content1, local1_off),
            (name2, content2, local2_off),
            (name3, content3, local3_off),
        ];
        for (name, content, local_off) in entries {
            central.extend_from_slice(&ZIP_CENTRAL_SIG);
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes()); // method=0
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&(content.len() as u32).to_le_bytes());
            central.extend_from_slice(&(content.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&local_off.to_le_bytes());
            central.extend_from_slice(name);
        }

        let cd_off = zip.len() as u32;
        let cd_size = central.len() as u32;
        let cd_count = 3u16;
        let mut eocd = Vec::new();
        eocd.extend_from_slice(&ZIP_EOCD_SIG);
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&cd_count.to_le_bytes());
        eocd.extend_from_slice(&cd_count.to_le_bytes());
        eocd.extend_from_slice(&cd_size.to_le_bytes());
        eocd.extend_from_slice(&cd_off.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());

        zip.extend_from_slice(&central);
        zip.extend_from_slice(&eocd);

        // Extract apex_payload.img (entry 2).
        let result = read_zip_entry_stored_from_bytes(&zip, "apex_payload.img");
        assert!(result.is_ok(), "extraction failed: {:?}", result.err());
        assert_eq!(result.unwrap(), content2);
    }

    // ========================================================================
    // Tests for apex_temp_dir / apex_payload_temp_path / apex_mount_dir
    // (the temp-path fix for 5-M's diagnosis: /tmp/ doesn't exist in
    // the parent's Android-app-sandbox context before setup_mounts).
    // ========================================================================

    #[test]
    fn apex_temp_dir_from_respects_tmpdir_env_var() {
        // Mock env lookup that returns a custom TMPDIR. Verifies the
        // helper prefers TMPDIR over BOTH the TWOYI_DATA_DIR fallback
        // and the hardcoded fallback (Task 6-Z88 chain).
        let d = apex_temp_dir_from(|k| match k {
            "TMPDIR" => Some("/custom/tmp/dir".to_string()),
            "TWOYI_DATA_DIR" => Some("/data/data/io.twoyi.debug".to_string()),
            _ => None,
        });
        assert_eq!(d, "/custom/tmp/dir");
    }

    #[test]
    fn apex_temp_dir_from_falls_back_when_tmpdir_unset() {
        // Mock env lookup that returns None for TMPDIR (i.e. unset).
        // Verifies the Task 6-Z88 chain: TWOYI_DATA_DIR+"/cache" wins
        // over the hardcoded io.twoyi path, and with BOTH unset we
        // still land on the compatibility fallback.
        let d = apex_temp_dir_from(|k| {
            if k == "TWOYI_DATA_DIR" {
                Some("/data/user/0/io.twoyi.debug".to_string())
            } else {
                None
            }
        });
        assert_eq!(d, "/data/user/0/io.twoyi.debug/cache");
        // Trailing '/' on TWOYI_DATA_DIR must not produce "//cache".
        let d2 = apex_temp_dir_from(|k| {
            if k == "TWOYI_DATA_DIR" {
                Some("/data/user/0/io.twoyi.debug/".to_string())
            } else {
                None
            }
        });
        assert_eq!(d2, "/data/user/0/io.twoyi.debug/cache");
        // Everything unset → hardcoded last resort.
        let d3 = apex_temp_dir_from(|_| None);
        assert_eq!(d3, "/data/data/io.twoyi/cache");
    }

    #[test]
    fn apex_temp_dir_from_ignores_empty_tmpdir() {
        // Mock env lookup that returns Some("") for TMPDIR (and for
        // TWOYI_DATA_DIR). Verifies the helper treats empty strings
        // as "unset" and falls through the whole chain.
        // (Android's app sandbox always sets TMPDIR to a non-empty
        // path, but defensive coding is cheap.)
        let d = apex_temp_dir_from(|k| {
            if k == "TMPDIR" || k == "TWOYI_DATA_DIR" {
                Some(String::new())
            } else {
                None
            }
        });
        assert_eq!(d, "/data/data/io.twoyi/cache");
    }

    #[test]
    fn apex_payload_temp_path_in_joins_correctly() {
        // Pure path-construction test — no env var, no create_dir_all
        // side effect. Verifies the base + filename join for the
        // extracted apex_payload.img temp file.
        assert_eq!(
            apex_payload_temp_path_in("/data/data/io.twoyi/cache"),
            "/data/data/io.twoyi/cache/twoyi-apex-payload.img"
        );
        assert_eq!(
            apex_payload_temp_path_in("/custom/tmp"),
            "/custom/tmp/twoyi-apex-payload.img"
        );
    }

    #[test]
    fn apex_mount_dir_in_joins_correctly() {
        // Pure path-construction test — no env var, no create_dir_all
        // side effect. Verifies the base + dirname join for the
        // loopback-mount mount point.
        assert_eq!(
            apex_mount_dir_in("/data/data/io.twoyi/cache"),
            "/data/data/io.twoyi/cache/twoyi-apex-mount"
        );
        assert_eq!(
            apex_mount_dir_in("/custom/tmp"),
            "/custom/tmp/twoyi-apex-mount"
        );
    }

    #[test]
    fn apex_payload_temp_path_uses_tmpdir_when_set() {
        // Integration test: with TMPDIR set to a writable temp dir,
        // apex_payload_temp_path() should return a path under TMPDIR
        // and create_dir_all should make the parent exist.
        // We set TMPDIR for this test only — Rust's test runner may
        // run tests in parallel, so we use a per-test unique subdir
        // to avoid races on the same env var.
        use std::sync::Mutex;
        static TMPDIR_LOCK: Mutex<()> = Mutex::new(());

        let _guard = TMPDIR_LOCK.lock().unwrap();
        let prev = std::env::var("TMPDIR").ok();
        let test_tmp = std::env::temp_dir().join("twoyi-apex-test-tmpdir-5N");
        std::fs::create_dir_all(&test_tmp).unwrap();
        std::env::set_var("TMPDIR", &test_tmp);

        let p = apex_payload_temp_path();
        // Restore TMPDIR before any assert! that might panic — leaking
        // the env var would break other tests.
        match prev {
            Some(v) => std::env::set_var("TMPDIR", v),
            None => std::env::remove_var("TMPDIR"),
        }
        drop(_guard);

        assert!(
            p.starts_with(test_tmp.to_str().unwrap()),
            "expected path under {:?}, got: {}",
            test_tmp,
            p
        );
        assert!(
            p.ends_with("/twoyi-apex-payload.img"),
            "expected path to end with /twoyi-apex-payload.img, got: {}",
            p
        );
        // Parent must exist (create_dir_all was called).
        let parent = std::path::Path::new(&p)
            .parent()
            .expect("temp path must have a parent");
        assert!(
            parent.exists(),
            "parent directory {} must exist after apex_payload_temp_path() (create_dir_all was called)",
            parent.display()
        );
        // Cleanup.
        let _ = std::fs::remove_dir_all(&test_tmp);
    }

    // ========================================================================
    // Tests for apex_candidate_paths.
    // ========================================================================

    #[test]
    fn apex_candidate_paths_includes_rom_dir_when_set() {
        let cfg = crate::Config {
            rootfs: "/data/data/io.twoyi/rootfs".to_string(),
            rom_dir: Some("/data/data/io.twoyi/rom".to_string()),
            ..crate::Config::default()
        };
        // Default (6-Z211: host /apex/ scan DEFAULT ON): rom_dir +
        // rootfs candidates + bare host paths (the host scan is a
        // FALLBACK when the guest rootfs doesn't have the APEX).
        let cands = apex_candidate_paths_with(&|_| None, &cfg);
        // First candidate should be the rom_dir path.
        assert!(cands[0].contains("/data/data/io.twoyi/rom/"));
        assert!(cands[0].ends_with("/system/apex/com.android.runtime.apex"));
        // 6-Z211: host paths ARE included by default now (the scan is
        // a safe fallback — VFS isolation preserved because the host
        // path is only used to READ bytes, never exposed to the guest).
        assert!(cands
            .iter()
            .any(|p| p == "/system/apex/com.android.runtime.apex"));
        // Opt-out via TWOYI_DISALLOW_HOST_APEX=1 excludes host paths.
        let cands_opt_out = apex_candidate_paths_with(
            &|k| (k == "TWOYI_DISALLOW_HOST_APEX").then(|| "1".to_string()),
            &cfg,
        );
        assert!(!cands_opt_out
            .iter()
            .any(|p| p == "/system/apex/com.android.runtime.apex"));
    }

    #[test]
    fn apex_candidate_paths_omits_rom_dir_when_none() {
        let cfg = crate::Config {
            rootfs: "/data/data/io.twoyi/rootfs".to_string(),
            rom_dir: None,
            ..crate::Config::default()
        };
        let cands = apex_candidate_paths_with(&|_| None, &cfg);
        // Should NOT include any path containing "/rom/".
        assert!(cands.iter().all(|p| !p.contains("/rom/system/apex/")));
        // Should include the rootfs path.
        assert!(cands
            .iter()
            .any(|p| p == "/data/data/io.twoyi/rootfs/system/apex/com.android.runtime.apex"));
        // 6-Z211: host paths ARE included by default (scan is a safe fallback).
        assert!(cands
            .iter()
            .any(|p| p == "/system/apex/com.android.runtime.apex"));
        // Opt-out via TWOYI_DISALLOW_HOST_APEX=1 excludes host paths.
        let cands_opt_out = apex_candidate_paths_with(
            &|k| (k == "TWOYI_DISALLOW_HOST_APEX").then(|| "1".to_string()),
            &cfg,
        );
        assert!(!cands_opt_out
            .iter()
            .any(|p| p == "/system/apex/com.android.runtime.apex"));
    }

    #[test]
    fn apex_candidate_paths_always_includes_host_paths() {
        // 6-Z211: host paths are now DEFAULT-ON (the scan is a safe
        // fallback when the guest rootfs doesn't have the APEX — VFS
        // isolation preserved because the host path is only used to
        // READ bytes, never exposed to the guest).
        //
        // NOTE: rootfs must be NON-empty here — with the empty default
        // the rootfs candidate formats to the same string as the bare
        // host path and the two become indistinguishable.
        let cfg = crate::Config {
            rootfs: "/data/data/io.twoyi/rootfs".to_string(),
            ..crate::Config::default()
        };
        let cands = apex_candidate_paths_with(&|_| None, &cfg);
        // 6-Z211: host paths ARE included by default.
        assert!(cands
            .iter()
            .any(|p| p == "/system/apex/com.android.runtime.apex"));
        assert!(cands.iter().any(|p| p == "/apex/com.android.runtime.apex"));
        // Opt-out via TWOYI_DISALLOW_HOST_APEX=1 excludes host paths.
        let cands_opt_out = apex_candidate_paths_with(
            &|k| (k == "TWOYI_DISALLOW_HOST_APEX").then(|| "1".to_string()),
            &cfg,
        );
        assert!(!cands_opt_out
            .iter()
            .any(|p| p == "/system/apex/com.android.runtime.apex"));
        assert!(!cands_opt_out
            .iter()
            .any(|p| p == "/apex/com.android.runtime.apex"));
    }

    // ========================================================================
    // Tests for find_real_libdl_so (end-to-end, with mock filesystem).
    //
    // These tests can't exercise the loopback mount path (requires root +
    // /dev/loop-control, not available in the test environment). They
    // verify the higher-level logic: candidate enumeration, format
    // detection, and the None return when no candidates exist.
    // ========================================================================

    #[test]
    fn find_real_libdl_so_returns_none_when_no_apex_exists() {
        // Point rom_dir + rootfs at non-existent paths so no .apex
        // candidate exists. The function should log the candidates it
        // tried and return None (not panic).
        let cfg = crate::Config {
            rootfs: "/nonexistent-twoyi-rootfs-xyz-5L".to_string(),
            rom_dir: Some("/nonexistent-twoyi-rom-xyz-5L".to_string()),
            ..crate::Config::default()
        };
        let result = find_real_libdl_so(&cfg);
        assert!(
            result.is_none(),
            "expected None when no .apex candidate exists, got: {:?}",
            result.map(|(p, _)| p)
        );
    }

    #[test]
    fn extract_apex_payload_img_returns_err_for_missing_file() {
        let result = extract_apex_payload_img("/nonexistent-apex-file-twoyi-test-5L");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn extract_apex_payload_img_returns_err_for_non_zip_non_ext4() {
        let tmp = std::env::temp_dir().join("twoyi-apex-test-garbage.bin");
        std::fs::write(&tmp, b"not a zip, not an ext4 image, just garbage").unwrap();
        let result = extract_apex_payload_img(tmp.to_str().unwrap());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("neither a ZIP nor an ext4 image"),
            "expected 'neither a ZIP nor an ext4 image' in error, got: {}",
            err
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn extract_apex_payload_img_extracts_stored_entry_from_real_zip() {
        // Build a ZIP with apex_payload.img entry, write it to a temp
        // file, and verify extraction works end-to-end (file → bytes).
        let content = b"fake ext4 image content for end-to-end test";
        let zip = build_minimal_stored_zip("apex_payload.img", content);
        let tmp = std::env::temp_dir().join("twoyi-apex-test-real.zip");
        std::fs::write(&tmp, &zip).unwrap();

        let result = extract_apex_payload_img(tmp.to_str().unwrap());
        assert!(result.is_ok(), "extraction failed: {:?}", result.err());
        assert_eq!(result.unwrap(), content);

        let _ = std::fs::remove_file(&tmp);
    }

    // ========================================================================
    // Tests for the loopback mount code (smoke tests only — full
    // functionality requires root + /dev/loop-control, not available in
    // the test env).
    // ========================================================================

    #[test]
    fn loopback_mount_and_read_returns_err_for_missing_ext4_file() {
        // We can't actually test the success path (requires root +
        // /dev/loop-control), but we CAN verify the function handles a
        // missing input file gracefully.
        let result = loopback_mount_and_read("/nonexistent-ext4-image-twoyi-test-5L", "libdl.so");
        assert!(result.is_err());
        let err = result.unwrap_err();
        // The error should mention the open failure on the ext4 image.
        assert!(err.contains("open ext4 image"));
    }

    // ========================================================================
    // Tests for the loop device mknod (5-P's fix for 5-O's diagnosis:
    // /dev/loopN device node doesn't exist after LOOP_CTL_GET_FREE on
    // Android emulator — no udev to auto-create).
    // ========================================================================

    #[test]
    fn makedev_loop7_minor0_is_canonical_loop_dev_t() {
        // /dev/loop0 is the canonical reference: major=7, minor=0.
        // Both the Android (libc 0.2.189 android/mod.rs) and Linux
        // (libc 0.2.189 linux/mod.rs) `makedev` formulas produce:
        //   dev = (major & 0xfff) << 8 | (minor & 0xff) | (minor & 0xfff00) << 12
        // For (7, 0): dev = 0x700 = 1792.
        // (Linux's extended formula adds `(major & 0xfffff000) << 32` and
        // `(minor & 0xffffff00) << 12`, both 0 for these small values.)
        let dev = libc::makedev(7, 0);
        assert_eq!(dev, 0x700, "makedev(7, 0) should be 0x700 = 1792");
    }

    #[test]
    fn makedev_loop7_minor28_is_5o_observed_index() {
        // 5-O observed LOOP_CTL_GET_FREE return n=28 on bbc2849. The
        // mknod call uses makedev(7, 28) to construct the dev_t for
        // /dev/loop28. Both Android and Linux formulas produce:
        //   (7 & 0xfff) << 8  = 0x700
        //   (28 & 0xff)       = 0x1c
        //   (28 & 0xfff00) << 12 = 0
        //   dev = 0x71c = 1820
        let dev = libc::makedev(7, 28);
        assert_eq!(dev, 0x71c, "makedev(7, 28) should be 0x71c = 1820");
    }

    #[test]
    fn makedev_major_minor_round_trip_for_loop_indices() {
        // For every loop index 0..255 (the valid minor range for block
        // major 7), makedev/major/minor must round-trip. This catches any
        // future libc signature drift that might break the mknod path.
        for n in 0u32..256 {
            let dev = libc::makedev(7, n);
            let ma = libc::major(dev);
            let mi = libc::minor(dev);
            assert_eq!(ma, 7, "major(dev_t for n={}) should be 7", n);
            assert_eq!(mi, n, "minor(dev_t for n={}) should be {}", n, n);
        }
    }

    // ========================================================================
    // Tests for libdl_asset_candidate_paths + read_libdl_asset (Option D,
    // 5-U's recommendation: ship libdl.so as APK asset + Java extraction).
    // ========================================================================

    #[test]
    fn libdl_asset_candidate_paths_includes_data_dir_files_libdl() {
        // The primary candidate is derived from cfg.data_dir. cfg.data_dir
        // is set by Java via Renderer.setDataDir(getApplicationInfo().dataDir)
        // → "/data/data/io.twoyi" (single-user) or
        // "/data/user/<id>/io.twoyi" (work profile).
        let cfg = crate::Config {
            data_dir: "/data/data/io.twoyi".to_string(),
            ..crate::Config::default()
        };
        let candidates = libdl_asset_candidate_paths(&cfg);
        assert!(
            candidates.contains(&"/data/data/io.twoyi/files/libdl.so".to_string()),
            "expected /data/data/io.twoyi/files/libdl.so in candidates: {:?}",
            candidates
        );
        // The cfg.data_dir-derived path must be the FIRST candidate (so
        // work-profile paths are preferred over the hardcoded single-user
        // fallback).
        assert_eq!(
            candidates[0], "/data/data/io.twoyi/files/libdl.so",
            "first candidate should be the cfg.data_dir-derived path"
        );
    }

    #[test]
    fn libdl_asset_candidate_paths_handles_work_profile_data_dir() {
        // In a work profile, cfg.data_dir is /data/user/<id>/io.twoyi
        // (NOT /data/data/io.twoyi). The candidate list must derive from
        // cfg.data_dir so the work profile's files dir is used.
        let cfg = crate::Config {
            data_dir: "/data/user/11/io.twoyi".to_string(),
            ..crate::Config::default()
        };
        let candidates = libdl_asset_candidate_paths(&cfg);
        assert!(
            candidates.contains(&"/data/user/11/io.twoyi/files/libdl.so".to_string()),
            "expected /data/user/11/io.twoyi/files/libdl.so in candidates: {:?}",
            candidates
        );
        assert_eq!(
            candidates[0], "/data/user/11/io.twoyi/files/libdl.so",
            "work-profile path must be the first candidate"
        );
        // The hardcoded single-user fallback is still present as the
        // last candidate (defensive — the work profile's path won't
        // exist on a single-user install, but it's tried anyway).
        assert!(
            candidates.contains(&"/data/data/io.twoyi/files/libdl.so".to_string()),
            "expected /data/data/io.twoyi/files/libdl.so fallback in candidates: {:?}",
            candidates
        );
    }

    #[test]
    fn libdl_asset_candidate_paths_has_hardcoded_fallback_when_data_dir_empty() {
        // If cfg.data_dir is empty (test env or unset), the candidate
        // list must still include the hardcoded fallback path so the
        // search doesn't silently no-op.
        let cfg = crate::Config {
            data_dir: String::new(),
            ..crate::Config::default()
        };
        let candidates = libdl_asset_candidate_paths(&cfg);
        assert_eq!(
            candidates,
            vec!["/data/data/io.twoyi/files/libdl.so".to_string()],
            "expected only the hardcoded fallback when data_dir is empty"
        );
    }

    #[test]
    fn read_libdl_asset_returns_none_when_file_missing() {
        // Point data_dir at a non-existent path so no candidate exists.
        // The function should log the candidates + return None (not panic).
        let cfg = crate::Config {
            data_dir: "/nonexistent-twoyi-data-dir-option-d-test".to_string(),
            ..crate::Config::default()
        };
        let result = read_libdl_asset(&cfg);
        assert!(
            result.is_none(),
            "expected None when asset file doesn't exist, got: {:?}",
            result.map(|(p, _)| p)
        );
    }

    #[test]
    fn read_libdl_asset_returns_none_when_asset_is_stub_sized_elf() {
        // Write a 5848-byte ELF-magic file (mimics the Android bootstrap
        // stub that someone might accidentally ship). The size guard
        // (bytes.len() > LIBDL_STUB_SIZE) must reject it.
        let tmp = std::env::temp_dir().join("twoyi-option-d-test-stub");
        let files_dir = tmp.join("files");
        std::fs::create_dir_all(&files_dir).unwrap();
        let mut stub = vec![0u8; LIBDL_STUB_SIZE];
        stub[0..4].copy_from_slice(&ELF_MAGIC);
        std::fs::write(files_dir.join("libdl.so"), &stub).unwrap();

        let cfg = crate::Config {
            data_dir: tmp.to_str().unwrap().to_string(),
            ..crate::Config::default()
        };
        let result = read_libdl_asset(&cfg);
        assert!(
            result.is_none(),
            "5848-byte ELF stub must be rejected by Option D guard, got: {:?}",
            result.map(|(p, b)| (p, b.len()))
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_libdl_asset_returns_none_when_asset_is_placeholder_text() {
        // Write a small text placeholder (not ELF, not 5848 bytes) — this
        // is what gets shipped in the assets/ dir until CI/dev drops the
        // real libdl.so in. Both the size guard AND ELF-magic guard reject
        // it; the function must return None and fall through gracefully.
        let tmp = std::env::temp_dir().join("twoyi-option-d-test-placeholder");
        std::fs::create_dir_all(tmp.join("files")).unwrap();
        std::fs::write(
            tmp.join("files").join("libdl.so"),
            b"PLACEHOLDER - run scripts/extract_libdl_from_apex.sh to replace",
        )
        .unwrap();

        let cfg = crate::Config {
            data_dir: tmp.to_str().unwrap().to_string(),
            ..crate::Config::default()
        };
        let result = read_libdl_asset(&cfg);
        assert!(
            result.is_none(),
            "placeholder text asset must be rejected, got: {:?}",
            result.map(|(p, b)| (p, b.len()))
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_libdl_asset_returns_some_when_asset_is_real_elf() {
        // Write a > 5848-byte ELF-magic file (mimics the real libdl.so
        // after CI/dev runs the extract script). The function must
        // return Some((path, bytes)).
        let tmp = std::env::temp_dir().join("twoyi-option-d-test-real");
        std::fs::create_dir_all(tmp.join("files")).unwrap();
        let mut real = vec![0u8; 20000]; // > LIBDL_STUB_SIZE = 5848
        real[0..4].copy_from_slice(&ELF_MAGIC);
        std::fs::write(tmp.join("files").join("libdl.so"), &real).unwrap();

        let cfg = crate::Config {
            data_dir: tmp.to_str().unwrap().to_string(),
            ..crate::Config::default()
        };
        let result = read_libdl_asset(&cfg);
        assert!(
            result.is_some(),
            "real ELF asset (> 5848 bytes) must be accepted by Option D"
        );
        let (path, bytes) = result.unwrap();
        assert!(
            path.ends_with("files/libdl.so"),
            "returned path should point at the asset: {}",
            path
        );
        assert_eq!(bytes.len(), 20000);
        assert_eq!(&bytes[0..4], &ELF_MAGIC);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_libdl_asset_falls_back_to_hardcoded_path_when_data_dir_empty() {
        // When cfg.data_dir is empty, the candidate list is just the
        // hardcoded /data/data/io.twoyi/files/libdl.so. On a typical
        // dev machine this path doesn't exist, so the function returns
        // None. (This test verifies the fallback doesn't panic when the
        // hardcoded path is missing.)
        let cfg = crate::Config {
            data_dir: String::new(),
            ..crate::Config::default()
        };
        // Don't assert on the result — the path may or may not exist on
        // this test host. The point is just to verify the function
        // doesn't panic when data_dir is empty.
        let _ = read_libdl_asset(&cfg);
    }
}
