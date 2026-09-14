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
//! 2. Guest writes the service name: `"pipe:opengles"` — 13 bytes on
//!    some goldfish-opengl builds, **14 bytes (`"pipe:opengles\0"`) on
//!    others** (rn285 decode: the current A11 ranchu stack writes the
//!    name NUL-terminated; sessions forwarded 1/5/1 tail bytes, the 1
//!    being the NUL). On the REAL goldfish transport the service-open
//!    parser consumes that NUL — it must NEVER enter the GL data
//!    stream (a stray NUL shifts `RenderServer::Main`'s
//!    `readFully(clientFlags, 4)` by one byte and deadlocks both
//!    sides: the composer blocks in `read(512KiB)` waiting for the
//!    first renderControl response, the renderer waits for flags that
//!    already arrived garbled).
//! 3. Host reads the channel name (consuming exactly one NUL
//!    terminator when present), connects to `{rootfs}/opengles`.
//! 4. Bytes flow bidirectionally: guest GL commands → renderer,
//!    renderer responses → guest.
//!
//! The first DATA message after the handshake is a 4-byte `clientFlags`
//! little-endian u32 (0 = normal session), then emugl command packets
//! (8-byte header: u32 opcode + u32 packetLen, then payload).

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crate::{error, info, warning};

/// Magic prefix the guest writes immediately after connect.
const PIPE_PREFIX: &str = "pipe:";

/// 6-Z352: the global unknown-channel peek budget — 24 sessions per
/// process, each peeking ≤160B / ≤240ms before the honest close (the
/// decode evidence stays bounded; no session semantics change).
static QZ352_PEEK_BUDGET: AtomicU64 = AtomicU64::new(24);

/// 6-Z353: goldfish SUPPORT channels served by the proxy itself — no
/// renderer socket exists for them and the pinned client protocols are
/// self-contained (see the two serve_* handlers below).
const SERVICE_CHANNELS: [&str; 2] = ["refcount", "GLProcessPipe"];

/// 6-Z353: monotonically increasing per-process unique IDs for the
/// goldfish "GLProcessPipe" handshake. `0` is the client's no-puid
/// sentinel (ProcessPipe.cpp leaves sProcUID = 0 when the handshake
/// fails), so the host-assigned space starts at 1 — the puid is
/// "assigned by the host", never derived from the pid.
static NEXT_GL_PROCESS_PUID: AtomicU64 = AtomicU64::new(1);

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
    // 6-Z268: BLOCKING accept — the shutdown path already wakes accept()
    // via a self-connect (see QemuPipeProxyHandle::shutdown), so the
    // O_NONBLOCK + 20 ms poll bought nothing but 50 wakeups/s of
    // scheduler noise beside the latency-critical tracer thread.

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
                                if let Err(e) = handle_session(guest_stream, &rootfs_clone, sid) {
                                    warning!("[KR64][qemu_pipe] session {} ended: {}", sid, e);
                                }
                            })
                            .ok();
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
    // Step 1: read the "pipe:<channel>" handshake (plus any payload
    // bytes that arrived in the same packet — forwarded to the
    // renderer below so the stream stays in sync). The NUL terminator
    // (when the guest sends one) is CONSUMED here — it is part of the
    // service-open command, not the data stream (6-Z336).
    let (channel, leftover) = read_channel_name(&mut guest)?;
    info!("[KR64][qemu_pipe] session {} channel = {}", sid, channel);

    // 6-Z353: the support channels are served in-process; the leftover
    // handshake bytes ride along (the confirm int / first handle write
    // may have arrived coalesced with the name — the rn285 session-2
    // shape).
    if channel == "refcount" {
        return serve_refcount_channel(guest, sid, leftover);
    }
    if channel == "GLProcessPipe" {
        return serve_gl_process_pipe_channel(guest, sid, leftover);
    }

    if !FORWARD_CHANNELS.contains(&channel.as_str()) {
        // Unknown channel — close. (Future: route "audio", "camera", etc.)
        warning!(
            "[KR64][qemu_pipe] session {} unknown channel '{}', closing",
            sid,
            channel
        );
        // 6-Z352 (rn303 decode): the goldfish GL stack's SUPPORT channels
        // land here — 'GLProcessPipe' ×7 and 'refcount' ×6 in rn303 — and
        // the composer's gralloc1 allocate died NO_RESOURCES in the same
        // era (GraphicBufferAllocator "Failed to allocate (720 x 1600)
        // usage a00: 5"), with the address-space device itself WORKING
        // (offset 0x0 size 0x100). Before implementing the support
        // protocols, capture WHAT the guest actually writes on them:
        // bounded peek (≤160B, ≤4 reads × 60ms, 24-shot global budget)
        // then the same honest close. No semantic change — the session
        // closes as before; only the decode evidence improves.
        if QZ352_PEEK_BUDGET.load(Ordering::Relaxed) > 0 {
            QZ352_PEEK_BUDGET.fetch_sub(1, Ordering::Relaxed);
            let _ = guest.set_read_timeout(Some(std::time::Duration::from_millis(60)));
            let mut peek = [0u8; 160];
            let mut got = 0usize;
            for _ in 0..4 {
                if got == peek.len() {
                    break;
                }
                match std::io::Read::read(&mut guest, &mut peek[got..]) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => got += n,
                }
            }
            if got > 0 {
                let hex: String = peek[..got]
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join("");
                let ascii: String = peek[..got]
                    .iter()
                    .map(|&b| {
                        if (0x20..0x7f).contains(&b) {
                            b as char
                        } else {
                            '.'
                        }
                    })
                    .collect();
                info!(
                    "[KR64][qemu_pipe] 6-Z352 session {} '{}' peek {}B hex={} ascii={}",
                    sid, channel, got, hex, ascii
                );
            } else {
                info!(
                    "[KR64][qemu_pipe] 6-Z352 session {} '{}' peek: no bytes in the 60ms window",
                    sid, channel
                );
            }
        }
        return Ok(());
    }

    // Step 2: open the matching renderer socket under the same rootfs.
    let renderer_path = format!("{}/{}", rootfs, channel);
    let mut renderer = UnixStream::connect(&renderer_path).map_err(|e| {
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

    // If the guest coalesced the handshake with early payload bytes
    // (e.g. the first clientFlags word), push them into the renderer
    // NOW — the pump below only moves bytes that arrive later, so any
    // bytes we swallowed here would be silently dropped and the wire
    // protocol would desync.
    if !leftover.is_empty() {
        info!(
            "[KR64][qemu_pipe] session {} forwarding {} handshake-tail bytes",
            sid,
            leftover.len()
        );
        renderer.write_all(&leftover)?;
    }

    // 6-Z336 wire observer: per-direction byte counters + a bounded
    // stall watchdog. The rn285 deadlock was invisible because NOTHING
    // counted the forwarded bytes — a session could sit half-silent
    // forever. The watchdog fires at +30s and +120s (2 lines max per
    // session): a healthy session closes long before; a stalled one
    // names its direction (g2r==0 → the guest never sent clientFlags;
    // g2r>0 && r2g==0 → the renderer consumed but never replied).
    let g2r_done = Arc::new(AtomicBool::new(false));
    let r2g_done = Arc::new(AtomicBool::new(false));
    let g2r_bytes = Arc::new(AtomicU64::new(0));
    let r2g_bytes = Arc::new(AtomicU64::new(0));
    {
        let g2r_bytes = g2r_bytes.clone();
        let r2g_bytes = r2g_bytes.clone();
        let g2r_done_wd = g2r_done.clone();
        let r2g_done_wd = r2g_done.clone();
        std::thread::Builder::new()
            .name(format!("kr64-pipe-watchdog-{}", sid))
            .spawn(move || {
                // Re-check liveness at each fire; skip if already closed.
                for (delay_ms, mark) in [(30_000u64, "+30s"), (120_000u64, "+120s")] {
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    if g2r_done_wd.load(Ordering::Acquire) && r2g_done_wd.load(Ordering::Acquire) {
                        return;
                    }
                    warning!(
                        "[KR64][qemu_pipe] session {} STALL {}: g2r={}B r2g={}B (still open)",
                        sid,
                        mark,
                        g2r_bytes.load(Ordering::Acquire),
                        r2g_bytes.load(Ordering::Acquire)
                    );
                }
            })
            .ok();
    }

    // We need two clones of each stream: one for reading, one for writing.
    // UnixStream::try_clone() duplicates the fd.
    let mut guest_for_write = guest.try_clone()?;
    let mut renderer_for_read = renderer.try_clone()?;

    // Clone both Arcs BEFORE the first move closure captures them.
    // The first closure moves g2r_done; the second moves r2g_done.
    // Each closure also needs a clone of the OTHER Arc to check when
    // the opposite direction has closed.
    let r2g_done_for_g2r = r2g_done.clone();
    let g2r_done_for_r2g = g2r_done.clone();
    let g2r_bytes_for_g2r = g2r_bytes.clone();
    let r2g_bytes_for_r2g = r2g_bytes.clone();
    let g2r_thread = std::thread::Builder::new()
        .name(format!("kr64-pipe-g2r-{}", sid))
        .spawn(move || {
            pump(
                &mut guest,
                &mut renderer,
                &g2r_done,
                &r2g_done_for_g2r,
                Some(&g2r_bytes_for_g2r),
            );
        })?;

    let r2g_thread = std::thread::Builder::new()
        .name(format!("kr64-pipe-r2g-{}", sid))
        .spawn(move || {
            pump(
                &mut renderer_for_read,
                &mut guest_for_write,
                &r2g_done,
                &g2r_done_for_r2g,
                Some(&r2g_bytes_for_r2g),
            );
        })?;

    let _ = g2r_thread.join();
    let _ = r2g_thread.join();

    info!(
        "[KR64][qemu_pipe] session {} closed (g2r={}B r2g={}B)",
        sid,
        g2r_bytes.load(Ordering::Acquire),
        r2g_bytes.load(Ordering::Acquire)
    );
    Ok(())
}

/// Read the `"pipe:<channel>"` handshake from the guest.
///
/// The guest writes the channel name (e.g. `"pipe:opengles"` — with or
/// without a trailing NUL, build-dependent) in a single `write()`
/// call. We read up to 256 bytes and parse the channel name from the
/// buffer. Returns `(name, leftover)` where `leftover` holds any bytes
/// that arrived in the same packet(s) AFTER the name (and after the
/// name's NUL terminator, which is consumed — 6-Z336): the caller must
/// forward them or the stream desyncs.
///
/// A parse is only accepted when the name is genuinely TERMINATED:
/// either a non-printable byte follows it in the buffer, or the
/// accumulated bytes exactly equal `"pipe:" + a known channel name`.
/// (Accepting end-of-buffer as a terminator would parse the split
/// delivery `"pipe:open" + "gles"` as channel `"open"` and kill the
/// session.)
fn read_channel_name(stream: &mut UnixStream) -> std::io::Result<(String, Vec<u8>)> {
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
        if let Some((name, consumed)) = parse_channel_name(&buf[..total]) {
            let leftover = buf[consumed..total].to_vec();
            return Ok((name.to_string(), leftover));
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "channel name too long or not found",
    ))
}

/// Channels proxied to a renderer socket under the rootfs
/// (`{rootfs}/{channel}`, served by libOpenglRender's RenderServer).
const FORWARD_CHANNELS: [&str; 3] = ["opengles", "opengles2", "opengles3"];

/// A channel name the proxy accepts at all (forwarded OR served in
/// process — 6-Z353 added the two support channels to the accepted set).
fn is_known_channel(name: &str) -> bool {
    FORWARD_CHANNELS.contains(&name) || SERVICE_CHANNELS.contains(&name)
}

/// If `buf` starts with `"pipe:"` and contains a TERMINATED printable
/// name, return `(name, total_bytes_consumed)` — consumed counts from
/// the start of `buf` through the END of the name, PLUS exactly one
/// NUL terminator when the name is NUL-terminated (6-Z336: the NUL is
/// part of the service-open command and must never enter the data
/// stream). Returns `None` if the buffer doesn't start with `"pipe:"`,
/// the name is empty, or no terminator has arrived yet (keep reading).
///
/// No-terminator case (the guest sent the name alone and the connection
/// stays open for the payload): the buffered bytes are accepted as the
/// complete name UNLESS they are still a proper PREFIX of a known
/// channel — "pipe:open" must keep waiting because "opengles" may still
/// arrive (split-delivery regression lock), while "pipe:unknown_channel"
/// cannot grow into anything known and is returned so the caller can
/// reject the unknown channel and close the connection (otherwise the
/// proxy would block on read forever — the hang this rule prevents).
fn parse_channel_name(buf: &[u8]) -> Option<(&str, usize)> {
    if !buf.starts_with(PIPE_PREFIX.as_bytes()) {
        return None;
    }
    let name_bytes = &buf[PIPE_PREFIX.len()..];
    // Channel names are ASCII printable. Stop at NUL, control chars,
    // or non-ASCII (the guest may include clientFlags bytes after
    // the channel name in the same write).
    let end = name_bytes
        .iter()
        .position(|&b| b == 0 || !(0x20..=0x7e).contains(&b));
    let end = match end {
        Some(i) => i,
        // No terminator in the buffer: only accept if the bytes so far
        // spell a complete known channel (AOSP sends exactly
        // "pipe:opengles" with nothing after it, and the connection
        // stays open for the payload).
        None => {
            let candidate = std::str::from_utf8(name_bytes).ok()?;
            if is_known_channel(candidate) {
                return Some((candidate, buf.len()));
            }
            // Split-delivery patience: a PROPER PREFIX of a known
            // channel ("open" of "opengles") must keep reading — the
            // rest of the name may still arrive. Anything else cannot
            // become a known channel, so treat it as a complete
            // (unknown) name; the caller logs + closes instead of
            // blocking forever on the next read.
            if [FORWARD_CHANNELS.as_slice(), SERVICE_CHANNELS.as_slice()]
                .concat()
                .iter()
                .any(|known| known.len() > candidate.len() && known.starts_with(candidate))
            {
                return None;
            }
            return Some((candidate, buf.len()));
        }
    };
    if end == 0 {
        return None;
    }
    let name = std::str::from_utf8(&name_bytes[..end]).ok()?;
    // 6-Z336 (rn285 decode): the service-name write is
    // NUL-terminated on the current A11 ranchu stack ("pipe:opengles\0"
    // = 14 bytes). On the REAL goldfish transport the service-open
    // parser consumes that NUL — it is part of the open command, not
    // the data stream. Consuming it (and forwarding only the bytes
    // AFTER it) keeps `RenderServer::Main`'s readFully(clientFlags, 4)
    // in sync; forwarding the NUL shifted the stream one byte and
    // deadlocked the composer inside hwcomposer.ranchu.so's first
    // renderControl read.
    if name_bytes[end] == 0 {
        return Some((name, PIPE_PREFIX.len() + end + 1));
    }
    // Non-NUL non-printable byte: that byte is stream DATA (e.g. the
    // first clientFlags byte of a nonzero flags word after an
    // unterminated name) — leave it for the caller's leftover.
    Some((name, PIPE_PREFIX.len() + end))
}

// ============================================================================
// 6-Z353: goldfish support channels
// ============================================================================

/// Serve the goldfish `refcount` support channel.
///
/// Pinned client: goldfish-opengl @ android11-release. `allocator3.cpp`
/// (the IAllocator service that turned rn303/rn304's composer allocate
/// into "GraphicBufferAllocator Failed to allocate (720 x 1600) usage
/// a00: 5") opens one connection per allocated color buffer and writes
/// the 4-byte host handle — and HARD-fails the whole allocation with
/// gralloc1 NO_RESOURCES when the open or the write fails, BEFORE any
/// renderer round-trip. `gralloc_30.cpp` is the same shape (the fd is
/// then kept inside the gralloc buffer handle, crossing processes);
/// `gralloc_old.cpp` / `egl.cpp` use it best-effort. The client NEVER
/// reads from this pipe: open + write success is the entire contract,
/// and closing the pipe is the reference-drop event (the real
/// emulator's RefCountPipe service keys color-buffer lifetime to it:
/// every write pins the handle, pipe close drops the pin).
///
/// This port's color buffers live until the owning GL session tears
/// down (rcCloseColorBuffer still frees), so the accounting here is
/// bookkeeping: one summary line per session, no per-write logging.
fn serve_refcount_channel(
    mut guest: UnixStream,
    sid: u64,
    leftover: Vec<u8>,
) -> std::io::Result<()> {
    let mut bytes = leftover.len() as u64;
    let mut buf = [0u8; 64];
    loop {
        match guest.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => bytes += n as u64,
            Err(_) => break,
        }
    }
    info!(
        "[KR64][qemu_pipe] session {} refcount closed ({} handle writes served)",
        sid,
        bytes / 4
    );
    Ok(())
}

/// Serve the goldfish `GLProcessPipe` support channel.
///
/// Pinned client: goldfish-opengl @ android11-release `ProcessPipe.cpp`
/// (`sQemuPipeInit`): once per process the GL stack opens the channel,
/// writes an `int32` confirmation (`100` — the 6-Z352 peek's
/// `64000000`), then BLOCKS reading the host-assigned per-process
/// unique ID (8 bytes, LE). With the handshake complete the client
/// announces the puid on the renderControl stream (`rcSetPuid`, op
/// 10033 — decoded by the renderer since 6-Z353); a failed handshake
/// degrades to the pre-6-Z353 fallback (open fails → "Process pipe
/// failed" → the default resource-cleanup path), which is exactly
/// today's close-on-open behavior.
fn serve_gl_process_pipe_channel(
    mut guest: UnixStream,
    sid: u64,
    leftover: Vec<u8>,
) -> std::io::Result<()> {
    // The confirm int may have arrived coalesced with the handshake
    // (the rn285 session-2 shape); consume what is already buffered and
    // read the remainder. A client that opens but never confirms would
    // wedge this thread forever — the real device would wait, but a
    // bounded 10 s guard turns a broken guest into the same
    // graceful-fallback close.
    let mut confirm = [0u8; 4];
    let from_leftover = leftover.len().min(4);
    confirm[..from_leftover].copy_from_slice(&leftover[..from_leftover]);
    if from_leftover < 4 {
        let _ = guest.set_read_timeout(Some(std::time::Duration::from_secs(10)));
        if guest.read_exact(&mut confirm[from_leftover..]).is_err() {
            warning!(
                "[KR64][qemu_pipe] session {} GLProcessPipe: no confirmation int, closing",
                sid
            );
            return Ok(());
        }
        let _ = guest.set_read_timeout(None);
    }
    let puid = NEXT_GL_PROCESS_PUID.fetch_add(1, Ordering::Relaxed);
    if guest.write_all(&puid.to_le_bytes()).is_err() {
        return Ok(());
    }
    info!(
        "[KR64][qemu_pipe] session {} GLProcessPipe: puid {} assigned (confirm={})",
        sid,
        puid,
        u32::from_le_bytes(confirm)
    );
    // Hold the pipe open — the client keeps it for the process lifetime
    // (gralloc stores the fd inside the buffer handle); EOF is the
    // process-exit event. No further protocol is defined on the pipe.
    let mut sink = [0u8; 64];
    loop {
        match guest.read(&mut sink) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    info!(
        "[KR64][qemu_pipe] session {} GLProcessPipe: puid {} exited",
        sid, puid
    );
    Ok(())
}

/// Bidirectional byte pump. Reads from `from`, writes to `to`.
/// Sets `my_done` when its direction closes; checks `other_done`
/// and exits early if the other direction has closed.
/// `counter` (6-Z336 wire observer) accumulates the forwarded byte
/// total for this direction when provided.
fn pump(
    from: &mut UnixStream,
    to: &mut UnixStream,
    my_done: &Arc<AtomicBool>,
    other_done: &Arc<AtomicBool>,
    counter: Option<&Arc<AtomicU64>>,
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
        if let Some(c) = counter {
            c.fetch_add(n as u64, Ordering::AcqRel);
        }
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
        assert_eq!(
            parse_channel_name(b"pipe:opengles").map(|(n, _)| n),
            Some("opengles")
        );
    }

    #[test]
    fn parse_opengles2() {
        assert_eq!(
            parse_channel_name(b"pipe:opengles2").map(|(n, _)| n),
            Some("opengles2")
        );
    }

    #[test]
    fn parse_opengles3() {
        assert_eq!(
            parse_channel_name(b"pipe:opengles3").map(|(n, _)| n),
            Some("opengles3")
        );
    }

    #[test]
    fn parse_strips_nul_terminator() {
        // 6-Z336 (rn285 decode): the current A11 ranchu stack writes the
        // service name NUL-terminated (14 bytes). The NUL is part of the
        // service-open command — it must be CONSUMED, never forwarded
        // into the GL data stream (the stray-NUL desync deadlocked the
        // composer inside hwcomposer.ranchu.so's first renderControl
        // read).
        assert_eq!(
            parse_channel_name(b"pipe:opengles\x00"),
            Some(("opengles", 5 + 8 + 1))
        );
    }

    #[test]
    fn parse_with_trailing_garbage() {
        // Non-NUL data byte after an unterminated name: that byte is
        // stream DATA (the first byte of a nonzero clientFlags word) —
        // the consumed offset stops BEFORE it so the caller forwards
        // it as leftover.
        assert_eq!(
            parse_channel_name(b"pipe:opengles\x01\x00\x00\x00"),
            Some(("opengles", 5 + 8))
        );
    }

    #[test]
    fn parse_unterminated_name_no_tail() {
        // Classic shape (older goldfish-opengl builds): exactly
        // "pipe:opengles", 13 bytes, no terminator, no tail.
        assert_eq!(parse_channel_name(b"pipe:opengles"), Some(("opengles", 13)));
    }

    #[test]
    fn parse_rejects_unterminated_unknown_name() {
        // Split delivery "pipe:open" + later "gles": the first chunk
        // is NOT a known channel and has no terminator — must keep
        // reading, not parse as channel "open" (regression lock for
        // the end-of-buffer-as-terminator bug).
        assert_eq!(parse_channel_name(b"pipe:open"), None);
        // ...and a full unknown name that IS terminated parses (the
        // caller's known-channel check rejects it afterwards).
        // ("unknown_channel" is 15 chars: 5 prefix + 15 consumed.)
        assert_eq!(
            parse_channel_name(b"pipe:unknown_channel"),
            Some(("unknown_channel", 5 + 15))
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
        let (name, leftover) = read_channel_name(&mut server).unwrap();
        assert_eq!(name, "opengles");
        assert!(leftover.is_empty());
    }

    #[test]
    fn read_channel_name_nul_terminated_later_flags() {
        // rn285 session-1/5 shape: "pipe:opengles\0" (14 bytes), flags
        // arrive in a LATER write. The terminator is consumed; nothing
        // is forwarded from the handshake.
        let (mut server, mut client) = pair();
        client.write_all(b"pipe:opengles\x00").unwrap();
        let (name, leftover) = read_channel_name(&mut server).unwrap();
        assert_eq!(name, "opengles");
        assert!(leftover.is_empty());
    }

    #[test]
    fn read_channel_name_nul_terminated_with_flags() {
        // rn285 session-2 shape: name + NUL terminator + clientFlags(4)
        // in ONE write (18 bytes total). Only the flags may survive as
        // leftover — the NUL must be consumed, or the renderer's
        // readFully(clientFlags, 4) desyncs by one byte.
        let (mut server, mut client) = pair();
        client
            .write_all(b"pipe:opengles\x00\x00\x00\x00\x00")
            .unwrap();
        let (name, leftover) = read_channel_name(&mut server).unwrap();
        assert_eq!(name, "opengles");
        assert_eq!(leftover, vec![0, 0, 0, 0]);
    }

    #[test]
    fn read_channel_name_with_client_flags() {
        // Guest writes channel name + clientFlags in one packet with a
        // NON-NUL first flags byte (no terminator): the flags must come
        // back as leftover untouched (they were previously dropped,
        // desyncing the wire protocol).
        let (mut server, mut client) = pair();
        client.write_all(b"pipe:opengles\x01\x00\x00\x00").unwrap();
        let (name, leftover) = read_channel_name(&mut server).unwrap();
        assert_eq!(name, "opengles");
        assert_eq!(leftover, vec![1, 0, 0, 0]);
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

        // Mock guest: writes the rn285 NUL-terminated channel name,
        // then clientFlags, then a fake emugl packet, and reads back
        // the echo. The NUL terminator must be consumed by the proxy —
        // if it were forwarded, the mock renderer's echo would carry a
        // 1-byte shift and this test would catch the 6-Z336 class.
        let mut guest = UnixStream::connect(&pipe_path).unwrap();
        guest.write_all(b"pipe:opengles\x00").unwrap();
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
        // Anti-hang guard (6-Z185): if the proxy ever regresses to
        // blocking-on-read for an unterminated unknown channel again,
        // this test must FAIL fast, not hang the whole suite.
        guest
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
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

    // ---- 6-Z353 support-channel tests ----

    #[test]
    fn parse_service_channels() {
        // The service channels must parse NUL-terminated…
        assert_eq!(
            parse_channel_name(b"pipe:refcount\x00"),
            Some(("refcount", 5 + 8 + 1))
        );
        assert_eq!(
            parse_channel_name(b"pipe:GLProcessPipe\x00"),
            Some(("GLProcessPipe", 5 + 13 + 1))
        );
        // …and unterminated (older goldfish-opengl builds send the bare
        // name and keep the connection open for the payload).
        assert_eq!(
            parse_channel_name(b"pipe:refcount"),
            Some(("refcount", 5 + 8))
        );
        assert_eq!(
            parse_channel_name(b"pipe:GLProcessPipe"),
            Some(("GLProcessPipe", 5 + 13))
        );
        // No prefix overlap with the forward channels: "refc" must keep
        // waiting for the rest of the name, not parse early.
        assert_eq!(parse_channel_name(b"pipe:refc"), None);
        assert_eq!(parse_channel_name(b"pipe:GLProcess"), None);
    }

    #[test]
    fn refcount_channel_serves_handle_writes_and_holds_open() {
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
        guest.write_all(b"pipe:refcount\x00").unwrap();
        thread::sleep(std::time::Duration::from_millis(100));

        // Two 4-byte host-handle writes (the entire client contract —
        // allocator3.cpp / gralloc_30.cpp) must both be ACCEPTED while
        // the session stays open. A close-on-open regression would make
        // the second write fail with EPIPE, failing this test.
        guest.write_all(&0x24u32.to_le_bytes()).unwrap();
        thread::sleep(std::time::Duration::from_millis(100));
        guest.write_all(&0x25u32.to_le_bytes()).unwrap();

        // The session holds until the guest drops it (gralloc keeps the
        // fd inside the buffer handle).
        thread::sleep(std::time::Duration::from_millis(100));
        guest
            .write_all(&0x26u32.to_le_bytes())
            .expect("refcount session must stay open for the fd lifetime");

        drop(guest);
        drop(proxy);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gl_process_pipe_mints_unique_nonzero_puids() {
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

        // Process 1: handshake with the confirm int COALESCED into the
        // open write (the rn285 session-2 shape) — the puid must still
        // come back.
        let mut g1 = UnixStream::connect(&pipe_path).unwrap();
        g1.set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let mut open = b"pipe:GLProcessPipe\x00".to_vec();
        open.extend_from_slice(&100u32.to_le_bytes());
        g1.write_all(&open).unwrap();
        let mut buf1 = [0u8; 8];
        g1.read_exact(&mut buf1).unwrap();
        let p1 = u64::from_le_bytes(buf1);
        assert_ne!(p1, 0, "0 is the client's no-puid sentinel");
        drop(g1);

        // Process 2: the confirm int in a SEPARATE write (the classic
        // shape) — must get a DIFFERENT, unique puid.
        let mut g2 = UnixStream::connect(&pipe_path).unwrap();
        g2.set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        g2.write_all(b"pipe:GLProcessPipe\x00").unwrap();
        g2.write_all(&100u32.to_le_bytes()).unwrap();
        let mut buf2 = [0u8; 8];
        g2.read_exact(&mut buf2).unwrap();
        let p2 = u64::from_le_bytes(buf2);
        assert_ne!(p2, 0);
        assert_ne!(p2, p1, "puids must be unique per process");

        drop(g2);
        drop(proxy);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gl_process_pipe_without_confirm_closes_gracefully() {
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

        // A client that opens the channel but never sends the confirm
        // int gets the graceful close (the pre-6-Z353 fallback shape),
        // not a wedged proxy thread. The guard is 10 s; this test would
        // take that long only on regression — acceptable as a slow
        // failure, never a hang.
        let mut guest = UnixStream::connect(&pipe_path).unwrap();
        guest
            .set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .unwrap();
        guest.write_all(b"pipe:GLProcessPipe\x00").unwrap();
        let mut sink = [0u8; 8];
        let result = guest.read(&mut sink);
        assert!(
            result.is_err() || result.unwrap() == 0,
            "expected the graceful close after the missing confirm int"
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

        // Drop the proxy — should shut down the accept thread. If the
        // accept thread ignored the shutdown flag, Drop's join() would
        // hang this test — that hang IS the assertion.
        let _path = proxy.path().to_string();
        drop(proxy);

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
