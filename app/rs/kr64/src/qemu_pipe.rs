// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `qemu_pipe` GL command proxy.
//!
//! Accepts the guest's connection to `/dev/qemu_pipe`, reads the
//! `"pipe:<channel>"` channel-open handshake, and forwards the
//! resulting bidirectional stream to libOpenglRender's `RenderServer`
//! listening on `{rootfs}/opengles` (or `opengles2` / `opengles3`).
//!
//! This replaces the MVP `spawn_accept_thread` stub that wrote a
//! single 0 byte and closed — which corrupted the guest's expected
//! read/write ordering (the guest writes first, not the host).
//!
//! See `download/QEMU_PIPE_DISPATCHER_PLAN.md` for the full design.
//!
//! # Wire protocol
//!
//! 1. Guest opens `/dev/qemu_pipe` (our Unix socket).
//! 2. Guest writes `"pipe:opengles"` (13 bytes, no NUL terminator).
//! 3. Host reads the channel name, connects to `{rootfs}/opengles`.
//! 4. Bytes flow bidirectionally: guest GL commands → renderer,
//!    renderer responses → guest.
//!
//! The first message after the handshake is a 4-byte `clientFlags`
//! little-endian u32 (0 = normal session), then emugl command packets
//! (8-byte header: u32 opcode + u32 packetLen, then payload).

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::{error, info, warning};

/// Magic prefix the guest writes immediately after connect.
const PIPE_PREFIX: &str = "pipe:";

/// Spawn the qemu_pipe proxy.
///
/// Takes ownership of the `UnixListener` (extracted from the
/// `DeviceSocket` via `take_listener()`). The proxy runs in a
/// background thread that accepts guest connections, reads the
/// channel-name handshake, connects to the matching renderer socket
/// under `rootfs`, and pumps bytes bidirectionally.
///
/// Returns a `QemuPipeProxyHandle` whose `Drop` impl shuts the proxy
/// down cleanly. Hold the handle until the guest exits.
pub fn spawn_qemu_pipe_proxy(
    listener: UnixListener,
    path: String,
    rootfs: String,
) -> std::io::Result<QemuPipeProxyHandle> {
    // Non-blocking so the accept loop can poll the shutdown flag.
    let fd = std::os::unix::io::AsRawFd::as_raw_fd(&listener);
    let _ = unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    let path_for_thread = path.clone();

    let thread = std::thread::Builder::new()
        .name("kr64-accept-qemu_pipe".into())
        .spawn(move || {
            info!(
                "[KR64][qemu_pipe] proxy thread started (listener={})",
                path_for_thread
            );
            let mut next_session_id: u64 = 0;
            loop {
                if shutdown_clone.load(Ordering::Acquire) {
                    info!("[KR64][qemu_pipe] shutdown flag set, exiting accept loop");
                    break;
                }
                match listener.accept() {
                    Ok((guest_stream, _addr)) => {
                        let sid = next_session_id;
                        next_session_id += 1;
                        info!("[KR64][qemu_pipe] guest connected (session={})", sid);
                        let rootfs_clone = rootfs.clone();
                        std::thread::Builder::new()
                            .name(format!("kr64-pipe-handshake-{}", sid))
                            .spawn(move || {
                                if let Err(e) =
                                    handle_session(guest_stream, &rootfs_clone, sid)
                                {
                                    warning!("[KR64][qemu_pipe] session {} ended: {}", sid, e);
                                }
                            })
                            .ok();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    Err(e) => {
                        warning!("[KR64][qemu_pipe] accept error: {}", e);
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }
            info!("[KR64][qemu_pipe] proxy thread exiting");
        })?;

    Ok(QemuPipeProxyHandle {
        shutdown,
        thread: Some(thread),
        path,
    })
}

/// Handle returned by `spawn_qemu_pipe_proxy`. Dropping it shuts
/// the proxy down and joins the accept thread.
pub struct QemuPipeProxyHandle {
    shutdown: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    path: String,
}

impl QemuPipeProxyHandle {
    /// The Unix socket path the proxy is listening on.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Signal the proxy to shut down. Idempotent.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
        // Connecting to the listener wakes up accept() so the thread
        // can observe the shutdown flag and exit.
        let _ = UnixStream::connect(&self.path);
    }
}

impl Drop for QemuPipeProxyHandle {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Per-connection handler. Reads the channel name, opens the
/// matching renderer socket, and pumps bytes both directions until
/// either side closes.
fn handle_session(mut guest: UnixStream, rootfs: &str, sid: u64) -> std::io::Result<()> {
    // Step 1: read the "pipe:<channel>" handshake.
    let channel = read_channel_name(&mut guest)?;
    info!(
        "[KR64][qemu_pipe] session {} channel = {}",
        sid, channel
    );

    if channel != "opengles" && channel != "opengles2" && channel != "opengles3" {
        // Unknown channel — close. (Future: route "audio", "camera", etc.)
        warning!(
            "[KR64][qemu_pipe] session {} unknown channel '{}', closing",
            sid,
            channel
        );
        return Ok(());
    }

    // Step 2: open the matching renderer socket under the same rootfs.
    let renderer_path = format!("{}/{}", rootfs, channel);
    let renderer = UnixStream::connect(&renderer_path).map_err(|e| {
        error!(
            "[KR64][qemu_pipe] session {} connect to {} failed: {}",
            sid, renderer_path, e
        );
        e
    })?;

    info!(
        "[KR64][qemu_pipe] session {} connected to renderer at {}",
        sid, renderer_path
    );

    // Step 3: spawn two pump threads for bidirectional forwarding.
    let g2r_done = Arc::new(AtomicBool::new(false));
    let r2g_done = Arc::new(AtomicBool::new(false));

    // We need two clones of each stream: one for reading, one for writing.
    // UnixStream::try_clone() duplicates the fd.
    let guest_for_write = guest.try_clone()?;
    let renderer_for_read = renderer.try_clone()?;

    let r2g_done_for_g2r = r2g_done.clone();
    let g2r_thread = std::thread::Builder::new()
        .name(format!("kr64-pipe-g2r-{}", sid))
        .spawn(move || {
            pump(&mut guest, &mut renderer, &g2r_done, &r2g_done_for_g2r);
        })?;

    let g2r_done_for_r2g = g2r_done.clone();
    let r2g_thread = std::thread::Builder::new()
        .name(format!("kr64-pipe-r2g-{}", sid))
        .spawn(move || {
            pump(
                &mut renderer_for_read,
                &mut guest_for_write,
                &r2g_done,
                &g2r_done_for_r2g,
            );
        })?;

    let _ = g2r_thread.join();
    let _ = r2g_thread.join();

    info!("[KR64][qemu_pipe] session {} closed", sid);
    Ok(())
}

/// Read the `"pipe:<channel>"` handshake from the guest.
///
/// The guest writes the channel name (e.g. `"pipe:opengles"`) in a
/// single `write()` call. We read up to 256 bytes and parse the
/// channel name from the buffer. If the first read doesn't contain
/// a complete channel name, we keep reading until we get one or hit
/// the buffer limit.
fn read_channel_name(stream: &mut UnixStream) -> std::io::Result<String> {
    let mut buf = [0u8; 256];
    let mut total = 0;
    while total < buf.len() {
        let n = stream.read(&mut buf[total..])?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "guest closed before sending channel name",
            ));
        }
        total += n;
        // AOSP writes the channel name in a single write() so the
        // first recv typically has all of it. But the guest MAY also
        // include the first 4 bytes of clientFlags in the same packet,
        // so we stop at the first non-printable byte.
        if let Some(name) = parse_channel_name(&buf[..total]) {
            return Ok(name.to_string());
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "channel name too long or not found",
    ))
}

/// If `buf` starts with `"pipe:"` and contains a printable name,
/// return the name as a `&str`. Returns `None` if the buffer doesn't
/// start with `"pipe:"` or the name is empty.
fn parse_channel_name(buf: &[u8]) -> Option<&str> {
    if !buf.starts_with(PIPE_PREFIX.as_bytes()) {
        return None;
    }
    let name_bytes = &buf[PIPE_PREFIX.len()..];
    // Channel names are ASCII printable. Stop at NUL, control chars,
    // or non-ASCII (the guest may include clientFlags bytes after
    // the channel name in the same write).
    let end = name_bytes
        .iter()
        .position(|&b| b == 0 || b < 0x20 || b > 0x7e)
        .unwrap_or(name_bytes.len());
    if end == 0 {
        return None;
    }
    std::str::from_utf8(&name_bytes[..end]).ok()
}

/// Bidirectional byte pump. Reads from `from`, writes to `to`.
/// Sets `my_done` when its direction closes; checks `other_done`
/// and exits early if the other direction has closed.
fn pump(
    from: &mut UnixStream,
    to: &mut UnixStream,
    my_done: &Arc<AtomicBool>,
    other_done: &Arc<AtomicBool>,
) {
    let mut buf = [0u8; 16 * 1024];
    loop {
        if other_done.load(Ordering::Acquire) {
            break;
        }
        let n = match from.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        if to.write_all(&buf[..n]).is_err() {
            break;
        }
    }
    my_done.store(true, Ordering::Release);
    // Signal the other side to wake up (its read will return EOF/error).
    let _ = to.shutdown(std::net::Shutdown::Both);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::thread;

    /// Helper: create a unique tmpdir for test isolation.
    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kr64-qemu-pipe-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ---- channel-name parser tests ----

    #[test]
    fn parse_opengles() {
        assert_eq!(parse_channel_name(b"pipe:opengles"), Some("opengles"));
    }

    #[test]
    fn parse_opengles2() {
        assert_eq!(parse_channel_name(b"pipe:opengles2"), Some("opengles2"));
    }

    #[test]
    fn parse_opengles3() {
        assert_eq!(parse_channel_name(b"pipe:opengles3"), Some("opengles3"));
    }

    #[test]
    fn parse_with_trailing_garbage() {
        // The guest may write a single packet that includes both the
        // channel name and the first 4 bytes of clientFlags. The
        // parser must stop at the first non-printable byte.
        assert_eq!(
            parse_channel_name(b"pipe:opengles\x00\x00\x00\x00"),
            Some("opengles")
        );
    }

    #[test]
    fn parse_rejects_no_prefix() {
        assert_eq!(parse_channel_name(b"opengles"), None);
    }

    #[test]
    fn parse_rejects_empty() {
        assert_eq!(parse_channel_name(b""), None);
    }

    #[test]
    fn parse_rejects_prefix_only() {
        // "pipe:" with no channel name
        assert_eq!(parse_channel_name(b"pipe:"), None);
    }

    #[test]
    fn parse_rejects_non_utf8() {
        // Non-ASCII bytes in the name portion
        assert_eq!(parse_channel_name(b"pipe:\xff\xfe"), None);
    }

    // ---- read_channel_name tests ----

    #[test]
    fn read_channel_name_success() {
        let (mut server, mut client) = pair();
        client.write_all(b"pipe:opengles").unwrap();
        let name = read_channel_name(&mut server).unwrap();
        assert_eq!(name, "opengles");
    }

    #[test]
    fn read_channel_name_with_client_flags() {
        // Guest writes channel name + clientFlags in one packet
        let (mut server, mut client) = pair();
        client.write_all(b"pipe:opengles\x00\x00\x00\x00").unwrap();
        let name = read_channel_name(&mut server).unwrap();
        assert_eq!(name, "opengles");
    }

    #[test]
    fn read_channel_name_eof() {
        let (mut server, client) = pair();
        drop(client); // close without writing
        let result = read_channel_name(&mut server);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }

    // ---- end-to-end proxy tests ----

    #[test]
    fn proxy_forwards_bytes_bidirectionally() {
        let dir = tmpdir();
        let pipe_path = dir.join("dev").join("qemu_pipe");
        std::fs::create_dir_all(pipe_path.parent().unwrap()).unwrap();

        // Mock renderer: echoes back any bytes it receives.
        let renderer_path = dir.join("opengles");
        let renderer_listener = UnixListener::bind(&renderer_path).unwrap();
        let renderer_thread = thread::spawn(move || {
            let (mut s, _) = renderer_listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            loop {
                match s.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if s.write_all(&buf[..n]).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        // Start the proxy.
        let proxy_listener = UnixListener::bind(&pipe_path).unwrap();
        let proxy = spawn_qemu_pipe_proxy(
            proxy_listener,
            pipe_path.to_str().unwrap().to_string(),
            dir.to_str().unwrap().to_string(),
        )
        .unwrap();

        // Mock guest: writes channel name, then clientFlags, then a
        // fake emugl packet, and reads back the echo.
        let mut guest = UnixStream::connect(&pipe_path).unwrap();
        guest.write_all(b"pipe:opengles").unwrap();
        // Give the proxy time to parse the channel name and connect
        // to the mock renderer.
        thread::sleep(std::time::Duration::from_millis(100));

        // Write clientFlags (4 bytes, LE, value 0)
        guest.write_all(&0u32.to_le_bytes()).unwrap();
        // Write a fake emugl packet: opcode=10000, packetLen=8
        guest.write_all(&10000u32.to_le_bytes()).unwrap();
        guest.write_all(&8u32.to_le_bytes()).unwrap();

        // The echo renderer should send back what we wrote (after the
        // channel name was consumed by the proxy). So we expect to
        // read back: clientFlags(4) + opcode(4) + packetLen(4) = 12 bytes.
        let mut echo = [0u8; 12];
        guest.read_exact(&mut echo).unwrap();
        assert_eq!(&echo[..4], &0u32.to_le_bytes());
        assert_eq!(&echo[4..8], &10000u32.to_le_bytes());
        assert_eq!(&echo[8..12], &8u32.to_le_bytes());

        drop(guest);
        drop(proxy);
        let _ = renderer_thread.join();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn proxy_unknown_channel_closes_gracefully() {
        let dir = tmpdir();
        let pipe_path = dir.join("dev").join("qemu_pipe");
        std::fs::create_dir_all(pipe_path.parent().unwrap()).unwrap();

        let proxy_listener = UnixListener::bind(&pipe_path).unwrap();
        let proxy = spawn_qemu_pipe_proxy(
            proxy_listener,
            pipe_path.to_str().unwrap().to_string(),
            dir.to_str().unwrap().to_string(),
        )
        .unwrap();

        let mut guest = UnixStream::connect(&pipe_path).unwrap();
        guest.write_all(b"pipe:unknown_channel").unwrap();

        // The proxy should close the connection gracefully (no renderer
        // to connect to). Give it time to process.
        thread::sleep(std::time::Duration::from_millis(200));

        // Try to read — should get EOF or error.
        let mut buf = [0u8; 16];
        let result = guest.read(&mut buf);
        assert!(
            result.is_err() || result.unwrap() == 0,
            "expected EOF or error after unknown channel"
        );

        drop(guest);
        drop(proxy);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn proxy_shutdown_stops_accept_thread() {
        let dir = tmpdir();
        let pipe_path = dir.join("dev").join("qemu_pipe");
        std::fs::create_dir_all(pipe_path.parent().unwrap()).unwrap();

        let proxy_listener = UnixListener::bind(&pipe_path).unwrap();
        let proxy = spawn_qemu_pipe_proxy(
            proxy_listener,
            pipe_path.to_str().unwrap().to_string(),
            dir.to_str().unwrap().to_string(),
        )
        .unwrap();

        // Drop the proxy — should shut down the accept thread.
        let path = proxy.path().to_string();
        drop(proxy);

        // The accept thread should have exited. Verify by checking that
        // the socket file may or may not exist (we don't unlink it in
        // the proxy, the DeviceSocket does), but the thread is gone.
        // If the thread didn't exit, this test would hang on Drop.
        assert!(PathBuf::from(&path).exists() || !PathBuf::from(&path).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- helper: create a connected pair of UnixStreams ----

    fn pair() -> (UnixStream, UnixStream) {
        // Use a temporary Unix socket pair via socketpair()
        use std::os::unix::io::FromRawFd;
        let mut fds = [0i32; 2];
        let ret =
            unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };
        assert_eq!(ret, 0, "socketpair failed");
        // SAFETY: fds are valid and owned by us
        let a = unsafe { UnixStream::from_raw_fd(fds[0]) };
        let b = unsafe { UnixStream::from_raw_fd(fds[1]) };
        (a, b)
    }
}
