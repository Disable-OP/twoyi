// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! Virtual `/dev/` tree materialiser.
//!
//! This module mirrors what Virtual Master's `libkr64.so` does at the
//! `mknodat()` call site at `0x11d770` (see `VM_KR64_ANALYSIS.md` §6):
//! it creates the per-VM virtual device nodes (sockets, char devices,
//! marker files) inside the guest rootfs's `/dev` directory, then binds
//! a Unix-domain socket to each socket-type node so the daemon can
//! `accept()` connections from the guest.
//!
//! # Differences from VM
//!
//! * VM creates sockets via `mknodat(S_IFSOCK)` then `bind()` (because
//!   Android's `socket()` + `bind()` to an existing path doesn't work
//!   for non-root — `mknodat` lets the socket file appear in `/dev`
//!   which the guest expects). Twoyi runs in the same model: we create
//!   the socket file via `mknodat` first, then `bind()` to it.
//! * For the skeleton we don't actually call `mknodat()` (which would
//!   require `CAP_MKNOD` and is forbidden in many sandboxes). Instead
//!   we use `UnixListener::bind()` directly — this creates the socket
//!   file as a side effect. The guest's `connect()` will succeed, which
//!   is all that matters for the MVP. A production version would
//!   pre-create the node via `mknodat(S_IFSOCK|0666)` and then `bind()`
//!   to it (matching VM's exact pattern), gated behind a capability
//!   check at startup.
//! * We use `std::os::unix::net::UnixListener` instead of the legacy
//!   `unix_socket` crate (the task spec asks for this — it removes one
//!   external dep and matches what the rest of twoyi has been
//!   migrating to).
//!
//! # Device inventory (MVP subset)
//!
//! | Path                                | Type    | Created by                |
//! |-------------------------------------|---------|---------------------------|
//! | `{rootfs}/dev/qemu_pipe`            | socket  | `create_qemu_pipe`        |
//! | `{rootfs}/dev/input/touch`          | socket  | `create_touch_device`     |
//! | `{rootfs}/dev/input/key0`           | socket  | `create_key_device`       |
//! | `{data_dir}/dev/event`              | socket  | `create_event_socket`     |
//! | `{rootfs}/dev/gb`                   | socket  | `create_graphics_buffer_devices` |
//! | `{rootfs}/dev/gb2`                  | socket  | `create_graphics_buffer_devices` |
//!
//! The full VM device inventory (see `VM_KR64_ANALYSIS.md` §4.2 / §6)
//! adds: `/dev/vmproc`, `/dev/__kmsg__`, `/dev/__kmsg2__`, `/dev/__krlog__`,
//! `/dev/__properties__`, `/dev/ashmem`, `/dev/ashmemsim`, `/dev/.busybox`,
//! `/dev/.coldboot_done`, `/dev/socket/process_pid`, `/dev/socket/logdw`,
//! `/dev/socket/logdr`, `/dev/block/vdc`, `/dev/fuse`, `/dev/hal/power_supply*`,
//! `/dev/tmpfs`, `/dev/tmpfs/ns`, plus three netlink sockets. Those will
//! be added incrementally in follow-up tasks.

use std::fs;
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::UnixListener;
use std::path::Path;

// Crate-local logging macros (defined in lib.rs) — no external `log` crate.
use crate::{info, warning};

/// A wrapped `UnixListener` that remembers the path it was bound to and
/// the raw FD it owns. Returned by every `create_*` function below so
/// the caller (the daemon main loop in `lib.rs`) can `accept()` on it.
///
/// Holding the `UnixListener` keeps the socket alive — dropping it
/// closes the FD and unlinks the path (via the `Drop` impl below).
pub struct DeviceSocket {
    /// The listener itself. `Option<UnixListener>` so `Drop` can take
    /// it out without disturbing the path field.
    pub listener: Option<UnixListener>,
    /// The filesystem path the socket is bound to (so we can unlink it
    /// on drop, and so the caller can pass the path to `connect()`).
    pub path: String,
    /// Whether to unlink the socket file on Drop. Set to false when
    /// `take_listener()` is called, so the worker thread (which now
    /// owns the listener) is responsible for cleanup — NOT the
    /// `DeviceSocket` shell that gets dropped immediately after
    /// `spawn_accept_thread` returns.
    should_unlink: bool,
}

impl DeviceSocket {
    /// Borrow the underlying listener (panics if already taken).
    pub fn listener(&self) -> &UnixListener {
        self.listener
            .as_ref()
            .expect("DeviceSocket: listener already taken")
    }

    /// Take ownership of the underlying listener (for moving into a
    /// thread that owns the accept loop, e.g.).
    ///
    /// **Important:** This also disables the `Drop` impl's `unlink`
    /// so the socket file stays alive while the worker thread holds
    /// the listener. Without this, the `DeviceSocket` shell is dropped
    /// immediately after `take_listener()` returns, unlinking the socket
    /// file before the guest can `connect()` to it.
    pub fn take_listener(&mut self) -> Option<UnixListener> {
        self.should_unlink = false; // Worker thread now owns the path
        self.listener.take()
    }

    /// The raw file descriptor — caller uses this to `accept()` or
    /// `poll()` on the socket. -1 if the listener has been taken.
    pub fn raw_fd(&self) -> RawFd {
        match self.listener.as_ref() {
            Some(l) => l.as_raw_fd(),
            None => -1,
        }
    }
}

impl Drop for DeviceSocket {
    fn drop(&mut self) {
        // Closing the listener FD is handled by `UnixListener`'s own
        // Drop. We additionally unlink the socket file so a re-run of
        // the daemon doesn't fail with "address already in use".
        // Errors here are non-fatal (the file may already be gone).
        //
        // Only unlink if we still own the path (i.e. `take_listener`
        // was NOT called). If the listener was taken by a worker
        // thread, that thread is responsible for unlinking.
        if self.should_unlink {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Make sure the parent directory of `path` exists, with mode 0755.
/// The guest's `init` expects `/dev`, `/dev/input`, etc. to be there.
fn ensure_parent_dir(path: &str) -> std::io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
        // Best-effort chmod — Android's init scans /dev and gets
        // confused if the dir is mode 0700 (it expects 0755).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o755));
        }
    }
    Ok(())
}

/// Internal helper: bind a `UnixListener` to `path`, removing any
/// stale socket file first. This is the common implementation behind
/// every public `create_*` function.
fn bind_unix_socket(path: &str) -> std::io::Result<UnixListener> {
    ensure_parent_dir(path)?;

    // Remove stale socket file from a previous run. If the path is a
    // non-socket file (regular file, dir, etc.) we still try to remove
    // it — `bind()` will fail with EADDRINUSE otherwise.
    match fs::remove_file(path) {
        Ok(()) => info!("[KR64][devices] removed stale socket: {}", path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            // If remove_file failed, the path might be a directory.
            // Try removing it as a directory (empty only).
            match fs::remove_dir(path) {
                Ok(()) => {
                    info!("[KR64][devices] removed stale dir: {}", path);
                }
                Err(_) => {
                    // Last resort: chmod the path so we can remove it,
                    // then try remove_file again.
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o777));
                        let _ = fs::remove_file(path);
                    }
                    warning!("[KR64][devices] could not remove {}: {}", path, e);
                }
            }
        }
    }

    // Bind. This creates the socket file as a side effect.
    let listener = UnixListener::bind(path)?;

    // chmod the socket file to 0666 so the guest (which may run as a
    // different uid inside the chroot) can connect. UnixListener::bind
    // creates the file with mode 0755 by default (modified by umask).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o666));
    }

    info!(
        "[KR64][devices] bound unix socket: {} (fd={})",
        path,
        listener.as_raw_fd()
    );
    Ok(listener)
}

/// Create `{rootfs}/dev/qemu_pipe` — the GL command transport.
///
/// The guest's SurfaceFlinger opens this socket and writes OpenGL ES
/// command streams to it (see `VM_KR64_ANALYSIS.md` §2.4 — the "GL
/// command transport" row). The daemon accepts the connection and
/// hands the resulting stream off to `libOpenglRender_aosp.so` (which
/// is already built and shipped in `app/src/main/jniLibs/<abi>/`).
///
/// On Android 11 VM also creates `/dev/goldfish_pipe` as an alias — we
/// skip that for the MVP (it's a fallback name, not a separate
/// transport).
pub fn create_qemu_pipe(rootfs: &str) -> std::io::Result<DeviceSocket> {
    let path = format!("{}/dev/qemu_pipe", rootfs);
    let listener = bind_unix_socket(&path)?;
    Ok(DeviceSocket {
        listener: Some(listener),
        path,
        should_unlink: true,
    })
}

/// Create `{rootfs}/dev/input/touch` — the multi-touch input device.
///
/// The guest's `EventHub` (in
/// `frameworks/base/services/core/java/com/android/server/input/EventHub.java`)
/// opens this path and reads `input_event` structs from it. The daemon
/// (or the host app) writes `EV_ABS`/`EV_SYN` events into the stream
/// when the user touches the SurfaceView.
///
/// This is the same protocol used by the existing twoyi input system
/// (see `app/rs/src/input.rs::touch_server`) — we're just relocating
/// the socket from `{data_dir}/rootfs/dev/input/touch` to
/// `{rootfs}/dev/input/touch` so the kr64 daemon owns it.
pub fn create_touch_device(rootfs: &str) -> std::io::Result<DeviceSocket> {
    let path = format!("{}/dev/input/touch", rootfs);
    let listener = bind_unix_socket(&path)?;
    Ok(DeviceSocket {
        listener: Some(listener),
        path,
        should_unlink: true,
    })
}

/// Create `{rootfs}/dev/input/key0` — the virtual key device.
///
/// Same protocol as the touch device but emits `EV_KEY` events for the
/// back/home/recents/volume/power keys (see
/// `app/rs/src/input.rs::android_keycode_to_linux` for the keycode
/// mapping). The guest's `InputReader` translates these into Android
/// `KeyEvent`s and dispatches them to the focused window.
pub fn create_key_device(rootfs: &str) -> std::io::Result<DeviceSocket> {
    let path = format!("{}/dev/input/key0", rootfs);
    let listener = bind_unix_socket(&path)?;
    Ok(DeviceSocket {
        listener: Some(listener),
        path,
        should_unlink: true,
    })
}

/// Create `{data_dir}/dev/event` — the event IPC socket.
///
/// This is the channel the guest uses to signal lifecycle events back
/// to the host: `BOOT_COMPLETED`, `SHUTDOWN`, `START_INSTALL_APP`,
/// `CLIPBOARD_DATA`, etc. The host's `TwoyiSocketServer.java`
/// (existing) accepts the connection and reads newline-or-backtick
/// separated UTF-8 strings.
///
/// Note: this socket is bound under `{data_dir}/dev/`, NOT
/// `{rootfs}/dev/`, because the host Java process is the one that
/// accepts connections on it — the host can't see inside the chrooted
/// rootfs. The guest's `init.rc` is patched to `connect()` to a path
/// that resolves to `{data_dir}/dev/event` from the host's perspective
/// (typically via a bind mount of `{data_dir}/dev/event` into the
/// rootfs at `/dev/event_host`).
///
/// This mirrors VM's `VMEventManager.java` which runs a
/// `LocalServerSocket("<vmDataDir>/dev/event")` in the host process.
pub fn create_event_socket(data_dir: &str) -> std::io::Result<DeviceSocket> {
    let path = format!("{}/dev/event", data_dir);
    let listener = bind_unix_socket(&path)?;
    Ok(DeviceSocket {
        listener: Some(listener),
        path,
        should_unlink: true,
    })
}

/// Create `{rootfs}/dev/gb` and `{rootfs}/dev/gb2` — the graphics
/// buffer devices.
///
/// These are Android 11+ additions (see `VM_KR64_ANALYSIS.md` §4.3 —
/// the Android 7 variant doesn't have them). They expose a
/// `gralloc`-like ioctl interface for allocating graphics buffers that
/// SurfaceFlinger can composite. `gb` is for framework gralloc, `gb2`
/// is for vendor (hwbinder) gralloc.
///
/// For the MVP both sockets just accept connections and respond to a
/// minimal `ALLOCATE`/`LOCK`/`UNLOCK`/`RELEASE` ioctl set — the
/// actual buffer management is delegated to
/// `libOpenglRender_aosp.so::ColorBuffer` (which already exists in
/// `app/rs/openglrenderer/src/gralloc.rs`).
///
/// Returns both sockets in a struct so the caller can dispatch on
/// `accept()` events from either.
pub struct GraphicsBufferDevices {
    pub gb: DeviceSocket,
    pub gb2: DeviceSocket,
}

pub fn create_graphics_buffer_devices(rootfs: &str) -> std::io::Result<GraphicsBufferDevices> {
    let gb_path = format!("{}/dev/gb", rootfs);
    let gb2_path = format!("{}/dev/gb2", rootfs);

    let gb_listener = bind_unix_socket(&gb_path)?;
    let gb2_listener = bind_unix_socket(&gb2_path)?;

    Ok(GraphicsBufferDevices {
        gb: DeviceSocket {
            listener: Some(gb_listener),
            path: gb_path,
            should_unlink: true,
        },
        gb2: DeviceSocket {
            listener: Some(gb2_listener),
            path: gb2_path,
            should_unlink: true,
        },
    })
}

/// Convenience: create ALL the MVP devices in one call.
///
/// This is what `main.rs` calls during startup. Returns a struct
/// holding all the bound listeners; the daemon main loop then spawns
/// one thread per listener to `accept()` and dispatch.
pub struct DeviceSet {
    pub qemu_pipe: DeviceSocket,
    pub touch: DeviceSocket,
    pub key: DeviceSocket,
    pub event: DeviceSocket,
    pub gb: GraphicsBufferDevices,
}

pub fn create_all_devices(rootfs: &str, data_dir: &str) -> std::io::Result<DeviceSet> {
    info!(
        "[KR64][devices] creating virtual /dev tree under {}/dev",
        rootfs
    );

    // Make sure /dev and /dev/input exist before any create_* call.
    let dev_dir = format!("{}/dev", rootfs);
    let dev_input_dir = format!("{}/dev/input", rootfs);
    fs::create_dir_all(&dev_dir)?;
    fs::create_dir_all(&dev_input_dir)?;
    fs::create_dir_all(format!("{}/dev", data_dir))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dev_dir, fs::Permissions::from_mode(0o755));
        let _ = fs::set_permissions(&dev_input_dir, fs::Permissions::from_mode(0o755));
    }

    let qemu_pipe = create_qemu_pipe(rootfs)?;
    let touch = create_touch_device(rootfs)?;
    let key = create_key_device(rootfs)?;
    let event = create_event_socket(data_dir)?;
    let gb = create_graphics_buffer_devices(rootfs)?;

    info!("[KR64][devices] all MVP devices created:");
    info!(
        "[KR64][devices]   qemu_pipe = {} (fd={})",
        qemu_pipe.path,
        qemu_pipe.raw_fd()
    );
    info!(
        "[KR64][devices]   touch     = {} (fd={})",
        touch.path,
        touch.raw_fd()
    );
    info!(
        "[KR64][devices]   key       = {} (fd={})",
        key.path,
        key.raw_fd()
    );
    info!(
        "[KR64][devices]   event     = {} (fd={})",
        event.path,
        event.raw_fd()
    );
    info!(
        "[KR64][devices]   gb        = {} (fd={})",
        gb.gb.path,
        gb.gb.raw_fd()
    );
    info!(
        "[KR64][devices]   gb2       = {} (fd={})",
        gb.gb2.path,
        gb.gb2.raw_fd()
    );

    Ok(DeviceSet {
        qemu_pipe,
        touch,
        key,
        event,
        gb,
    })
}

/// Create a marker file (used for `/dev/.coldboot_done`, `/dev/.busybox`,
/// etc.). VM creates these via `openat(O_CREAT|O_RDWR)` then writes 8
/// bytes (see `VM_KR64_ANALYSIS.md` §6 — the fall-through path of the
/// `__kr_mknod` dispatcher). For the MVP we just touch the file.
///
/// These markers are checked by the guest's `init` to decide when to
/// proceed past coldboot / when busybox is available.
pub fn create_marker_file(rootfs: &str, name: &str) -> std::io::Result<()> {
    let path = format!("{}/dev/{}", rootfs, name);
    ensure_parent_dir(&path)?;
    fs::write(&path, [0u8; 8])?;
    info!("[KR64][devices] wrote marker file: {}", path);
    Ok(())
}

/// Create `{rootfs}/dev/.coldboot_done` — the marker the guest's
/// `init` waits on before starting post-coldboot services.
pub fn create_coldboot_done_marker(rootfs: &str) -> std::io::Result<()> {
    create_marker_file(rootfs, ".coldboot_done")
}

/// Create `{rootfs}/dev/.busybox` — the marker that signals busybox
/// is installed (the guest's init.rc has `[ -f /dev/.busybox ]` guards
/// around busybox-specific commands).
pub fn create_busybox_marker(rootfs: &str) -> std::io::Result<()> {
    create_marker_file(rootfs, ".busybox")
}

/// Create the Magisk marker files in the guest rootfs.
///
/// This mirrors what VM does to make Magisk-aware apps (and Magisk itself,
/// if installed inside the guest) detect a "Magisk-compatible" environment.
/// Magisk uses a small set of marker files under `/dev` and `/sbin` to
/// signal its presence and to coordinate with its own daemon:
///
///   * `/dev/.magisk`              — main presence marker (unified build)
///   * `/dev/.magisk_unmount`      — signals "unmount modules" mode
///   * `/dev/.magisk.block`        — used by MagiskHide / DenyList probe
///   * `/sbin/.magisk/config`      — Magisk config dir marker (legacy path)
///
/// For twoyi we don't actually run a Magisk daemon — these markers exist
/// purely so guest apps that probe for Magisk (e.g. banking apps, SafetyNet
/// helpers, root checkers) see a consistent "rooted VM" environment and
/// don't crash on missing paths. The guest's own Magisk (if the user
/// installed it inside the VM) overlays these with real content via its
/// boot script.
///
/// Each marker is a tiny text file (not the 8-byte binary zero blob used by
/// `create_marker_file`) because Magisk itself writes short ASCII strings
/// into them (e.g. the Magisk version code). We follow that convention.
pub fn create_magisk_marker(rootfs: &str) -> std::io::Result<()> {
    // /dev/.magisk — main presence marker. Content is the Magisk version
    // code (we pretend to be Magisk 26.1 = version code 26100, the last
    // stable release before the Magisk/DenyList split). Apps that read
    // this file compare it against their minimum-supported version.
    let dev_magisk = format!("{}/dev/.magisk", rootfs);
    ensure_parent_dir(&dev_magisk)?;
    fs::write(&dev_magisk, "26100\n")?;
    info!(
        "[KR64][devices] wrote Magisk presence marker: {}",
        dev_magisk
    );

    // /dev/.magisk_unmount — empty marker (existence == "unmount mode on").
    // We create it empty so MagiskHide-style apps see the flag is set
    // without twoyi actually performing any unmount.
    let dev_unmount = format!("{}/dev/.magisk_unmount", rootfs);
    fs::write(&dev_unmount, "")?;
    info!(
        "[KR64][devices] wrote Magisk unmount marker: {}",
        dev_unmount
    );

    // /dev/.magisk.block — used by the Magisk daemon to coordinate boot.
    // Empty file; the guest's magiskd (if present) overwrites it.
    let dev_block = format!("{}/dev/.magisk.block", rootfs);
    fs::write(&dev_block, "")?;
    info!("[KR64][devices] wrote Magisk block marker: {}", dev_block);

    // /sbin/.magisk/ — the legacy Magisk working directory. Modern Magisk
    // uses /debug/.magisk on Android 11+, but the /sbin path is still
    // probed by older modules. We create the dir + a stub config so
    // `ls /sbin/.magisk` doesn't ENOENT.
    let sbin_magisk = format!("{}/sbin/.magisk", rootfs);
    fs::create_dir_all(&sbin_magisk)?;
    let sbin_config = format!("{}/config", sbin_magisk);
    fs::write(
        &sbin_config,
        "KEEPVERITY=false\nKEEPFORCEENCRYPT=false\nPATCHVBMETAFLAG=false\nRECOVERYMODE=false\n",
    )?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&sbin_magisk, fs::Permissions::from_mode(0o700));
        let _ = fs::set_permissions(&dev_magisk, fs::Permissions::from_mode(0o644));
        let _ = fs::set_permissions(&dev_unmount, fs::Permissions::from_mode(0o644));
        let _ = fs::set_permissions(&dev_block, fs::Permissions::from_mode(0o644));
    }

    info!(
        "[KR64][devices] Magisk marker tree materialised under {}/dev/.magisk*",
        rootfs
    );
    Ok(())
}

/// Create `{rootfs}/dev/dm-user` — the userspace device-mapper control
/// socket for Android 12+.
///
/// Android 12 (API 31) introduced `/dev/dm-user` as part of the
/// userspace device-mapper (`dm-user`) infrastructure used by
/// `virtual_ab` / `snapshotctl` for OTA snapshot merges and by
/// `userdata` checkpointing. The guest's `vold` and `init` open this
/// device during early boot to set up the dm-user target; if the node
/// is missing, vold logs `Failed to open /dev/dm-user` and falls back
/// to a degraded mode (no checkpoint, no snapshot merge), which on
/// some GSIs causes a boot-loop because `init` waits on a property
/// that vold never sets.
///
/// We can't `mknod()` a real char device (no CAP_MKNOD in the app
/// sandbox), so — like the other devices in this module — we bind a
/// Unix-domain socket to the path. The guest's `open("/dev/dm-user")`
/// succeeds, and the kr64 daemon's accept thread (see `lib.rs`)
/// handles the ioctl-style messages vold sends. For the MVP the
/// handler just accepts and closes; the production version will
/// implement the dm-user message protocol (DM_USER_MSG_MAP /
/// DM_USER_MSG_DONE).
///
/// See `VM_KR64_ANALYSIS.md` §4.2 (Android 12 device inventory) and
/// `GSI_BOOT_PLAN.md` §3.3 for why this node is required for A12 GSIs.
pub fn create_dm_user_device(rootfs: &str) -> std::io::Result<DeviceSocket> {
    let path = format!("{}/dev/dm-user", rootfs);
    let listener = bind_unix_socket(&path)?;
    info!(
        "[KR64][devices] created /dev/dm-user socket for Android 12+ dm-user target (path={})",
        path
    );
    Ok(DeviceSocket {
        listener: Some(listener),
        path,
        should_unlink: true,
    })
}

// ============================================================================
// Tests — pure-Rust, no Android deps, so they run on the host too.
// (cargo test --lib)
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Each test gets a UNIQUE tmpdir so parallel tests don't collide
    /// on the same socket path (which would cause EADDRINUSE on bind).
    fn tmpdir() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut p = env::temp_dir();
        p.push(format!("kr64-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&p).unwrap();
        p.to_string_lossy().to_string()
    }

    #[test]
    fn create_qemu_pipe_creates_socket_file() {
        let rootfs = tmpdir();
        let dev = create_qemu_pipe(&rootfs).expect("bind");
        assert!(Path::new(&dev.path).exists(), "socket file should exist");
        // raw_fd should be >= 0 (a real FD)
        assert!(dev.raw_fd() >= 0);
        // drop should unlink the socket
        drop(dev);
        assert!(!Path::new(&format!("{}/dev/qemu_pipe", rootfs)).exists());
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn create_all_devices_succeeds() {
        let rootfs = tmpdir();
        let data_dir = tmpdir();
        let set = create_all_devices(&rootfs, &data_dir).expect("create_all_devices");
        assert!(Path::new(&set.qemu_pipe.path).exists());
        assert!(Path::new(&set.touch.path).exists());
        assert!(Path::new(&set.key.path).exists());
        assert!(Path::new(&set.event.path).exists());
        assert!(Path::new(&set.gb.gb.path).exists());
        assert!(Path::new(&set.gb.gb2.path).exists());
        let _ = fs::remove_dir_all(&rootfs);
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn marker_files_are_created() {
        let rootfs = tmpdir();
        create_coldboot_done_marker(&rootfs).expect("coldboot");
        create_busybox_marker(&rootfs).expect("busybox");
        assert!(Path::new(&format!("{}/dev/.coldboot_done", rootfs)).exists());
        assert!(Path::new(&format!("{}/dev/.busybox", rootfs)).exists());
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn magisk_markers_are_created() {
        let rootfs = tmpdir();
        create_magisk_marker(&rootfs).expect("magisk");
        assert!(Path::new(&format!("{}/dev/.magisk", rootfs)).exists());
        assert!(Path::new(&format!("{}/dev/.magisk_unmount", rootfs)).exists());
        assert!(Path::new(&format!("{}/dev/.magisk.block", rootfs)).exists());
        assert!(Path::new(&format!("{}/sbin/.magisk/config", rootfs)).exists());
        // The presence marker should contain a version code.
        let v = fs::read_to_string(format!("{}/dev/.magisk", rootfs)).unwrap();
        assert!(
            v.trim().parse::<u32>().is_ok(),
            "magisk marker should be numeric"
        );
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn dm_user_device_is_created() {
        let rootfs = tmpdir();
        let dev = create_dm_user_device(&rootfs).expect("dm-user");
        assert!(Path::new(&dev.path).exists(), "dm-user socket should exist");
        assert!(dev.raw_fd() >= 0);
        drop(dev);
        assert!(!Path::new(&format!("{}/dev/dm-user", rootfs)).exists());
        let _ = fs::remove_dir_all(&rootfs);
    }
}
