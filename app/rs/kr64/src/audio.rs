// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! Virtual `/dev/audio` — bidirectional PCM pump.
//!
//! # Overview
//!
//! This module mirrors what Virtual Master's `libvm.so` does for audio
//! HAL virtualization (see `download/AUDIO_SENSOR_HAL.md` §1): it owns
//! a Unix-domain socket at `{rootfs}/dev/audio`, accepts the guest's
//! AudioFlinger connections, and pumps raw 16-bit PCM in both
//! directions between the guest and the host's `AudioTrack` /
//! `AudioRecord`.
//!
//! VM's `AudioService` is a *top-level* service (not under
//! `HALManager`) — instantiated directly by `VMInstance` alongside
//! Input/Display, because audio has hard real-time requirements (a
//! single blocked `write()` on the PCM path causes an audible glitch).
//! Twoyi mirrors that split: this module owns its own thread and has
//! no shared state with the other HALs.
//!
//! # Wire protocol — the 16-byte header
//!
//! Every connection begins with a 16-byte header sent by the guest:
//!
//! ```text
//!  offset  size  field         description
//!  ------  ----  -----------   ------------------------------------------
//!   0       4    magic         Little-endian u32, must be
//!                               [`AUDIO_HEADER_MAGIC`] = 0x4F445541
//!                               (ASCII `'AUDO'` in little-endian byte
//!                               order; documented in AUDIO_SENSOR_HAL.md §3.1).
//!   4       1    direction     1 = PLAYBACK, 2 = CAPTURE
//!                               (see [`AUDIO_DIR_PLAYBACK`] /
//!                                [`AUDIO_DIR_CAPTURE`]).
//!   5       3    reserved      Must be zero (padding to align the
//!                               next u32 to offset 8). Ignored on read.
//!   8       4    sample_rate   Little-endian u32. INFORMATIONAL ONLY —
//!                               VM hard-codes 44 100 Hz for playback
//!                               and 11 025 Hz for capture regardless
//!                               of what the guest sends here.
//!  12       2    channels      Little-endian u16. INFORMATIONAL ONLY —
//!                               VM hard-codes stereo (2) for playback
//!                               and mono (1) for capture.
//!  14       2    reserved      Must be zero (trailing padding).
//!  ------  ----
//!  total = 16 bytes (see [`AUDIO_HEADER_SIZE`])
//! ```
//!
//! After the header, the connection carries a raw byte stream of
//! 16-bit PCM samples (host byte order, which on Android is always
//! little-endian). There is no length prefix, no framing, no
//! compression — the bytes flow continuously until the guest closes
//! the socket.
//!
//! # Playback flow (guest → host)
//!
//! ```text
//!   Guest AudioFlinger               Host (this module + Java)
//!   ──────────────────              ────────────────────────────
//!   PlaybackThread mixer
//!     → PCM_16BIT @ 44100 stereo
//!     → write(/dev/audio, buf, n)
//!                                    accept() on /dev/audio
//!                                    read 16-byte header (dir=Playback)
//!                                    [JNI up-call] acquireAudioTrack()
//!                                       → host AudioTrack (STREAM_MUSIC,
//!                                         44100, stereo, PCM_16BIT,
//!                                         MODE_STREAM)
//!                                    loop {
//!                                      recv(sock, buf, minBuf)
//!                                      [JNI up-call] writeAudioData(track, buf, n)
//!                                        → AudioTrack.write(buf, off, len)
//!                                    }
//!                                    [JNI up-call] releaseAudioTrack(track)
//!   close(/dev/audio)
//!                                    connection closed
//! ```
//!
//! # Capture flow (host → guest)
//!
//! ```text
//!   Guest AudioFlinger               Host (this module + Java)
//!   ──────────────────              ────────────────────────────
//!   RecordThread
//!     → read(/dev/audio, buf, n)
//!                                    accept() on /dev/audio
//!                                    read 16-byte header (dir=Capture)
//!                                    [JNI up-call] acquireAudioRecord()
//!                                       → host AudioRecord (MIC source,
//!                                         11025, mono, PCM_16BIT)
//!                                    loop {
//!                                      [JNI up-call] readRecordData(rec, buf, n)
//!                                        → AudioRecord.read(buf, off, len)
//!                                      send(sock, buf, n)
//!                                    }
//!                                    [JNI up-call] releaseAudioRecord(rec)
//!   close(/dev/audio)
//!                                    connection closed
//! ```
//!
//! # JNI callback interface
//!
//! The actual `AudioTrack` / `AudioRecord` integration lives on the
//! Java side (a future `io.twoyi.hal.AudioService.java` modeled on
//! VM's `com.android.vmcore.hal.AudioService`, 222 lines). This Rust
//! module invokes it via six JNI up-calls, each mirroring a private
//! method on VM's `AudioService` (see AUDIO_SENSOR_HAL.md §1.5):
//!
//!  | Rust function (stub here)       | Java method (VM's AudioService)              | Returns                  |
//!  |---------------------------------|-----------------------------------------------|--------------------------|
//!  | [`jni_acquire_audio_track`]     | `AudioTrack acquireAudioTrack(int[] minBuf)`  | `(jtrack, minBufSize)`   |
//!  | [`jni_acquire_audio_record`]    | `AudioRecord acquireAudioRecord(int[] minBuf)`| `(jrecord, minBufSize)`  |
//!  | [`jni_write_audio_data`]        | `int writeAudioData(AudioTrack, byte[], o, l)`| bytes written            |
//!  | [`jni_read_record_data`]        | `int readRecordData(AudioRecord, byte[], o, l)`| bytes read              |
//!  | [`jni_release_audio_track`]     | `void releaseAudioTrack(AudioTrack)`          | `()`                     |
//!  | [`jni_release_audio_record`]    | `void releaseAudioRecord(AudioRecord)`        | `()`                     |
//!
//! Each up-call would attach the current thread to the JVM (cached in
//! a thread-local), find the AudioService class + method by signature,
//! marshal the args, and return. For the skeleton these are **stubs
//! that return null/0** — they let the pump loop compile and exercise
//! the protocol, but no sound is actually produced. The real
//! implementation will replace these six functions (likely behind a
//! trait object so the Java side can be wired in without touching the
//! pump code).
//!
//! # Threading
//!
//! One accept thread + a fixed-size [`ThreadPool`] of pump workers
//! (mirrors the pattern in `binder.rs`). Each guest connection is
//! dispatched to a worker, which runs the appropriate pump loop until
//! the guest disconnects. Because the pump loops block on socket I/O,
//! the pool size bounds the maximum number of concurrent audio
//! streams — default 8, enough for media + ringtone + notification +
//! alarm + system + voice call + 2 spare (see
//! [`AUDIO_THREAD_POOL_SIZE`]).
//!
//! # Hard-coded format (matches VM)
//!
//! VM hard-codes the sample rate / channel count in
//! `acquireAudioTrack` / `acquireAudioRecord` (see
//! `download/AUDIO_SENSOR_HAL.md` §1.6 — verbatim from the decompiled
//! Java source):
//!
//!  | Direction | Rate (Hz) | Channels | Encoding   | Android constant                    |
//!  |-----------|----------:|---------:|------------|-------------------------------------|
//!  | Playback  |    44 100 |        2 | PCM_16BIT  | STREAM_MUSIC, CHANNEL_OUT_STEREO    |
//!  | Capture   |    11 025 |        1 | PCM_16BIT  | MIC source, CHANNEL_IN_MONO         |
//!
//! The header's `sample_rate` / `channels` fields are informational
//! only — they're sent by the guest but ignored by the host (the host
//! always creates an `AudioTrack` at 44 100 Hz stereo or an
//! `AudioRecord` at 11 025 Hz mono regardless). This means twoyi's
//! guest ROM **must** also expect these exact rates from its audio
//! HAL module. If a future twoyi ROM asks for 48 000 Hz, the host's
//! `AudioTrack` will still play it (Android resamples internally),
//! but the guest's perception of its own sample rate will be wrong.
//!
//! # Latency
//!
//! VM uses `AudioTrack` in `MODE_STREAM` which gives ~125 ms host-side
//! latency + ~80 ms guest AudioFlinger latency ≈ 200 ms total. For a
//! rhythm game this is perceptible. The fix is `AudioTrack.Builder`
//! with `setPerformanceMode(PERFORMANCE_MODE_LOW_LATENCY)` and
//! `setBufferSizeInFrames(192)` (API 26+) — drops to ~20 ms host-side.
//! This is a Java-only change in `acquireAudioTrack`; the Rust pump
//! doesn't care. Tracked as follow-up `AUDIO-IMPL-2`.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

// Crate-local logging macros (defined in lib.rs) — no external `log` crate.
#[allow(unused_imports)]
use crate::{error, info, warning};

// ============================================================================
// Constants
// ============================================================================

/// Magic value at offset 0 of the audio header. Documented in
/// `download/AUDIO_SENSOR_HAL.md` §3.1. Both sides (guest audio HAL +
/// host pump) must agree on this value. The numeric value is the
/// little-endian encoding of the ASCII mnemonic `'AUDO'`
/// (`u32::from_le_bytes(*b"AUDO") == 0x4F445541`), so a guest HAL
/// that builds the constant from the string `'AUDO'` will match.
pub const AUDIO_HEADER_MAGIC: u32 = 0x4F445541;

/// Total size of the audio header on the wire (and of the
/// [`AudioHeader`] struct via `#[repr(C)]`).
pub const AUDIO_HEADER_SIZE: usize = 16;

/// Direction byte at offset 4 of the header: 1 = playback.
pub const AUDIO_DIR_PLAYBACK: u8 = 1;

/// Direction byte at offset 4 of the header: 2 = capture.
pub const AUDIO_DIR_CAPTURE: u8 = 2;

/// Hard-coded playback sample rate (Hz). Matches VM's `acquireAudioTrack`
/// (see AUDIO_SENSOR_HAL.md §1.6). Informational — the host always
/// creates an AudioTrack at this rate regardless of what the guest sends
/// in the header.
pub const PLAYBACK_SAMPLE_RATE: u32 = 44_100;

/// Hard-coded playback channel count. 2 = stereo.
pub const PLAYBACK_CHANNELS: u16 = 2;

/// Hard-coded capture sample rate (Hz). Matches VM's `acquireAudioRecord`
/// — note this is the voice rate 11.025 kHz, NOT 44.1.
pub const CAPTURE_SAMPLE_RATE: u32 = 11_025;

/// Hard-coded capture channel count. 1 = mono.
pub const CAPTURE_CHANNELS: u16 = 1;

/// Number of worker threads in the audio connection pool. Bounds the
/// maximum number of simultaneous audio streams the guest can open.
/// 8 covers media + ringtone + notification + alarm + system + voice
/// call + 2 spare.
pub const AUDIO_THREAD_POOL_SIZE: usize = 8;

/// Default scratch-buffer size for the pump loops (8 KiB). The real
/// buffer size is `min(host_min_buf, 8 KiB)` clamped to ≥256 bytes —
/// the host's `AudioTrack.getMinBufferSize()` return value (typically
/// ~125 ms × 176 KB/s ≈ 22 KB for stereo 16-bit @ 44.1 kHz) wins when
/// available, but we cap it so a misbehaving Java side can't make us
/// allocate a huge buffer per connection.
pub const AUDIO_PUMP_BUF_SIZE: usize = 8 * 1024;

// ============================================================================
// AudioDirection enum
// ============================================================================

/// The direction of an audio connection, encoded as a single byte at
/// offset 4 of the header.
///
/// `#[repr(u8)]` so `as u8` gives the exact wire value.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioDirection {
    /// Guest → host PCM stream. Host plays it via `AudioTrack`.
    Playback = AUDIO_DIR_PLAYBACK,
    /// Host → guest PCM stream. Host captures it via `AudioRecord`.
    Capture = AUDIO_DIR_CAPTURE,
}

impl AudioDirection {
    /// Parse a raw direction byte. Returns `None` for unknown values.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            AUDIO_DIR_PLAYBACK => Some(Self::Playback),
            AUDIO_DIR_CAPTURE => Some(Self::Capture),
            _ => None,
        }
    }

    /// The sample rate VM hard-codes for this direction.
    pub fn default_sample_rate(self) -> u32 {
        match self {
            Self::Playback => PLAYBACK_SAMPLE_RATE,
            Self::Capture => CAPTURE_SAMPLE_RATE,
        }
    }

    /// The channel count VM hard-codes for this direction.
    pub fn default_channels(self) -> u16 {
        match self {
            Self::Playback => PLAYBACK_CHANNELS,
            Self::Capture => CAPTURE_CHANNELS,
        }
    }
}

// ============================================================================
// AudioHeader struct (16 bytes on the wire)
// ============================================================================

/// The 16-byte audio connection header.
///
/// Wire layout (little-endian, see module docs for the full table):
/// ```text
///   off 0:  magic        u32 LE
///   off 4:  direction    u8
///   off 5:  reserved     3 bytes (zero)
///   off 8:  sample_rate  u32 LE
///   off 12: channels     u16 LE
///   off 14: reserved     2 bytes (zero)
/// ```
///
/// `#[repr(C)]` makes the in-memory layout match the wire layout on
/// all supported targets (aarch64 / x86_64 — both little-endian with
/// the same struct padding rules). The compile-time assertion below
/// guarantees `size_of::<AudioHeader>() == 16`. For actual (de)serial-
/// isation we use explicit byte slicing via [`AudioHeader::to_bytes`]
/// / [`AudioHeader::from_bytes`] so the wire format is deterministic
/// regardless of host endianness or padding.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioHeader {
    /// Offset 0. Must be [`AUDIO_HEADER_MAGIC`].
    pub magic: u32,
    /// Offset 4. 1 = Playback, 2 = Capture.
    pub direction: u8,
    /// Offset 8. Informational — host ignores.
    pub sample_rate: u32,
    /// Offset 12. Informational — host ignores.
    pub channels: u16,
}

// Compile-time assertion: the C-repr struct must be exactly 16 bytes.
// (3 bytes padding after `direction` to align `sample_rate`, plus 2
// bytes trailing padding after `channels`.)
const _: () = assert!(std::mem::size_of::<AudioHeader>() == AUDIO_HEADER_SIZE);

impl AudioHeader {
    /// Construct a header with the given direction and the
    /// VM-hard-coded sample rate / channel count for that direction.
    pub fn new(direction: AudioDirection) -> Self {
        Self::with_format(direction, direction.default_sample_rate(), direction.default_channels())
    }

    /// Construct a header with an explicit sample rate / channel
    /// count (used in tests; in production the host ignores these
    /// fields anyway).
    pub fn with_format(direction: AudioDirection, sample_rate: u32, channels: u16) -> Self {
        Self {
            magic: AUDIO_HEADER_MAGIC,
            direction: direction as u8,
            sample_rate,
            channels,
        }
    }

    /// Parsed direction, or `None` if `self.direction` is not a
    /// recognised value.
    pub fn direction(&self) -> Option<AudioDirection> {
        AudioDirection::from_u8(self.direction)
    }

    /// True if magic is correct and direction is recognised.
    pub fn is_valid(&self) -> bool {
        self.magic == AUDIO_HEADER_MAGIC && self.direction().is_some()
    }

    /// Serialise to a 16-byte little-endian array. See the struct docs
    /// for the exact wire layout.
    pub fn to_bytes(&self) -> [u8; AUDIO_HEADER_SIZE] {
        let mut buf = [0u8; AUDIO_HEADER_SIZE];
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());
        buf[4] = self.direction;
        // bytes 5..8 are reserved (already zero)
        buf[8..12].copy_from_slice(&self.sample_rate.to_le_bytes());
        buf[12..14].copy_from_slice(&self.channels.to_le_bytes());
        // bytes 14..16 are reserved (already zero)
        buf
    }

    /// Deserialise from a 16-byte (or longer) slice. Only the first
    /// 16 bytes are read; any trailing bytes are ignored (the caller
    /// is expected to read the header separately from the PCM stream).
    ///
    /// Returns an error if the buffer is too short, the magic is
    /// wrong, or the direction byte is not a recognised value.
    pub fn from_bytes(buf: &[u8]) -> Result<Self, AudioHeaderError> {
        if buf.len() < AUDIO_HEADER_SIZE {
            return Err(AudioHeaderError::TooShort { got: buf.len() });
        }
        let magic = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let direction = buf[4];
        let sample_rate = u32::from_le_bytes(buf[8..12].try_into().unwrap());
        let channels = u16::from_le_bytes(buf[12..14].try_into().unwrap());

        if magic != AUDIO_HEADER_MAGIC {
            return Err(AudioHeaderError::BadMagic { got: magic });
        }
        if AudioDirection::from_u8(direction).is_none() {
            return Err(AudioHeaderError::BadDirection { got: direction });
        }

        Ok(Self { magic, direction, sample_rate, channels })
    }
}

/// Error returned by [`AudioHeader::from_bytes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioHeaderError {
    /// Buffer was shorter than 16 bytes.
    TooShort { got: usize },
    /// Magic value at offset 0 didn't match [`AUDIO_HEADER_MAGIC`].
    BadMagic { got: u32 },
    /// Direction byte at offset 4 wasn't 1 (Playback) or 2 (Capture).
    BadDirection { got: u8 },
}

impl std::fmt::Display for AudioHeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(
                f,
                "audio header too short: got {} bytes, need {}",
                got, AUDIO_HEADER_SIZE
            ),
            Self::BadMagic { got } => write!(
                f,
                "audio header bad magic: got 0x{:08X}, expected 0x{:08X}",
                got, AUDIO_HEADER_MAGIC
            ),
            Self::BadDirection { got } => write!(
                f,
                "audio header bad direction: got {}, expected {} (Playback) or {} (Capture)",
                got, AUDIO_DIR_PLAYBACK, AUDIO_DIR_CAPTURE
            ),
        }
    }
}

impl std::error::Error for AudioHeaderError {}

// ============================================================================
// create_audio_device
// ============================================================================

/// Create the virtual `/dev/audio` Unix socket inside `rootfs`.
///
/// Mirrors Virtual Master's pattern (see AUDIO_SENSOR_HAL.md §1.2):
/// the guest's `AudioFlinger` opens `/dev/audio`, `connect()`s, and
/// the host's audio pump (this module's [`AudioDevice::spawn`]) is on
/// the other end of the socket.
///
/// This is the audio-specific equivalent of `devices::create_touch_device`
/// / `create_key_device` / etc., but it returns an [`AudioDevice`] that
/// owns the listener directly (rather than a generic `DeviceSocket`),
/// because the audio pump needs its own accept thread + worker pool
/// and doesn't fit the simple "echo a byte and close" pattern the
/// other devices use in the MVP.
///
/// # Errors
///
/// Returns an error if directory creation or `UnixListener::bind`
/// fails. Stale socket files from a previous run are best-effort
/// removed before bind (errors are logged but not propagated).
pub fn create_audio_device(rootfs: &str) -> std::io::Result<AudioDevice> {
    let path = format!("{}/dev/audio", rootfs);

    // Make sure {rootfs}/dev exists.
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o755));
        }
    }

    // Remove stale socket file from a previous run. If the path is a
    // non-socket file we still try to remove it — `bind()` would fail
    // with EADDRINUSE otherwise.
    match fs::remove_file(&path) {
        Ok(()) => info!("[KR64][audio] removed stale socket: {}", path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warning!("[KR64][audio] could not remove {}: {}", path, e),
    }

    // Bind. This creates the socket file as a side effect. (A
    // production version would first `mknodat(S_IFSOCK|0666)` then
    // `bind()` to it — matching VM's exact pattern at libkr64.so
    // offset 0x11d770 — but `mknodat` requires CAP_MKNOD which is
    // unavailable in many sandboxes. `UnixListener::bind` is the
    // unprivileged fallback and works fine for the skeleton.)
    let listener = UnixListener::bind(&path)?;

    // chmod 0666 so the guest (which may run as a different uid
    // inside the chroot) can connect. UnixListener::bind creates the
    // file with mode 0755 by default (modified by umask).
    #[cfg(unix)]
    {
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o666));
    }

    info!(
        "[KR64][audio] bound unix socket: {} (fd={})",
        path,
        listener.as_raw_fd()
    );

    Ok(AudioDevice {
        listener: Some(listener),
        path,
        shutdown: Arc::new(AtomicBool::new(false)),
    })
}

// ============================================================================
// AudioDevice — owns the listener, spawns accept + pump threads
// ============================================================================

/// A bound `/dev/audio` Unix socket, ready to accept guest connections.
///
/// Created by [`create_audio_device`]. Call [`AudioDevice::spawn`] to
/// start the accept thread + worker pool (consuming `self`); the
/// returned [`AudioDeviceHandle`] owns the running threads and will
/// shut them down on drop.
///
/// If `spawn` is not called, dropping the `AudioDevice` closes the
/// listener and unlinks the socket file.
pub struct AudioDevice {
    /// The listener itself. `Option<UnixListener>` so `spawn` can
    /// take it out (moving it into the accept thread) without
    /// disturbing the `path` field (needed for the `Drop` impl that
    /// unlinks the socket file).
    listener: Option<UnixListener>,
    /// The filesystem path the socket is bound to (so we can unlink
    /// it on drop, and so the caller can pass the path to `connect()`).
    path: String,
    /// Shutdown flag shared with the accept thread + the handle.
    /// Set to true by [`AudioDeviceHandle::shutdown`] / drop to ask
    /// the accept thread to exit.
    shutdown: Arc<AtomicBool>,
}

impl AudioDevice {
    /// The filesystem path the socket is bound to.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The raw file descriptor of the listener, or -1 if the listener
    /// has been taken (i.e. `spawn` was called).
    pub fn raw_fd(&self) -> RawFd {
        match self.listener.as_ref() {
            Some(l) => l.as_raw_fd(),
            None => -1,
        }
    }

    /// Spawn the accept thread + worker pool, consuming self.
    ///
    /// Returns an [`AudioDeviceHandle`] that holds the shutdown flag
    /// and the accept-thread `JoinHandle`. When the handle is dropped,
    /// the shutdown flag is set and the accept thread is joined.
    pub fn spawn(mut self) -> std::io::Result<AudioDeviceHandle> {
        let listener = self
            .listener
            .take()
            .expect("AudioDevice::spawn: listener already taken");

        // Make the listening socket non-blocking so the accept thread
        // can poll the shutdown flag between accept attempts (mirrors
        // BinderProxy::spawn in binder.rs).
        let fd = listener.as_raw_fd();
        let _ = unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };

        // Clone the shutdown Arc twice: one for the accept thread, one
        // for the returned handle. Both share the same AtomicBool.
        let shutdown_for_thread = Arc::clone(&self.shutdown);
        let shutdown_for_handle = Arc::clone(&self.shutdown);
        let path = self.path.clone();

        let accept_thread = thread::Builder::new()
            .name("kr64-audio-accept".to_string())
            .spawn(move || {
                // The pool lives inside the accept thread so its Drop
                // (which joins workers) runs when the accept thread
                // exits. This ensures workers are joined BEFORE the
                // accept thread returns.
                let pool = ThreadPool::new(AUDIO_THREAD_POOL_SIZE);
                info!(
                    "[KR64][audio] accept loop started (pool_size={})",
                    AUDIO_THREAD_POOL_SIZE
                );

                while !shutdown_for_thread.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _addr)) => {
                            info!("[KR64][audio] client connected");
                            pool.execute(move || {
                                if let Err(e) = handle_connection(stream) {
                                    warning!("[KR64][audio] connection handler ended: {}", e);
                                }
                            });
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // No pending connection — sleep briefly so
                            // we don't burn CPU.
                            std::thread::sleep(std::time::Duration::from_millis(25));
                        }
                        Err(e) => {
                            warning!("[KR64][audio] accept error: {}", e);
                            std::thread::sleep(std::time::Duration::from_millis(50));
                        }
                    }
                }
                info!("[KR64][audio] accept loop exiting");
                // pool drops here → workers receive Terminate and join.
            })?;

        Ok(AudioDeviceHandle {
            shutdown: shutdown_for_handle,
            accept_thread: Some(accept_thread),
            path,
        })
    }
}

impl Drop for AudioDevice {
    fn drop(&mut self) {
        // If the user dropped without calling spawn(), we still own
        // the listener — close it and unlink the socket file. If
        // spawn() was called, the listener was moved into the accept
        // thread and self.listener is None — in that case the
        // AudioDeviceHandle owns the unlink responsibility (and the
        // socket file must stay alive for guest connect()s).
        if self.listener.take().is_some() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Handle to a running audio device. Dropping this sets the shutdown
/// flag and joins the accept thread.
///
/// Created by [`AudioDevice::spawn`]. The accept thread + worker pool
/// keep running until either the handle is dropped or
/// [`AudioDeviceHandle::shutdown`] is called.
pub struct AudioDeviceHandle {
    shutdown: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
    path: String,
}

impl AudioDeviceHandle {
    /// Ask the accept thread to shut down. (Does not join — that
    /// happens on drop.)
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    /// The socket path the device is listening on.
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl Drop for AudioDeviceHandle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        if let Some(t) = self.accept_thread.take() {
            let _ = t.join();
        }
        // Best-effort unlink the socket file so a re-run of the daemon
        // doesn't fail with EADDRINUSE.
        let _ = fs::remove_file(&self.path);
    }
}

// ============================================================================
// Per-connection handler — dispatched to a worker thread by the accept loop.
// ============================================================================

/// Handle one guest connection: read the header, dispatch to the
/// playback or capture pump. Returns when the guest disconnects (EOF),
/// the header is invalid, or a JNI up-call reports an error.
fn handle_connection(mut stream: UnixStream) -> std::io::Result<()> {
    // 1. Read the 16-byte header.
    let mut hdr_buf = [0u8; AUDIO_HEADER_SIZE];
    read_exact(&mut stream, &mut hdr_buf)?;

    // 2. Parse + validate.
    let header = match AudioHeader::from_bytes(&hdr_buf) {
        Ok(h) => h,
        Err(e) => {
            warning!("[KR64][audio] rejecting connection: bad header: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()));
        }
    };

    // from_bytes already validated direction, so unwrap is safe.
    let dir = header.direction().expect("from_bytes validated direction");

    info!(
        "[KR64][audio] connection: direction={:?}, sample_rate={}, channels={}",
        dir, header.sample_rate, header.channels
    );

    // 3. Dispatch to the appropriate pump loop.
    match dir {
        AudioDirection::Playback => handle_playback(&mut stream, header),
        AudioDirection::Capture => handle_capture(&mut stream, header),
    }
}

/// Playback pump: guest writes PCM → we read from socket → call Java
/// `AudioTrack.write()` via the JNI stub. Returns when the guest
/// closes the socket (EOF) or the JNI up-call reports an error.
///
/// See the module docs for the full data-flow diagram.
fn handle_playback(stream: &mut UnixStream, header: AudioHeader) -> std::io::Result<()> {
    // 1. Acquire a host AudioTrack via the (stubbed) JNI up-call.
    //    VM's acquireAudioTrack ignores the rate/channel args and
    //    always creates a 44100/stereo/PCM_16BIT track; we pass the
    //    header's values anyway so a future Java side that DOES
    //    honour them will Just Work.
    let (track, min_buf) = jni_acquire_audio_track(header.sample_rate, header.channels);
    if track.is_null() {
        warning!(
            "[KR64][audio][playback] acquireAudioTrack returned null — closing connection \
             (JNI not yet wired up; sound will not be produced)"
        );
        return Ok(());
    }

    // 2. Size the scratch buffer. Host's min_buf wins when available,
    //    capped to AUDIO_PUMP_BUF_SIZE so a misbehaving Java side
    //    can't make us allocate a huge buffer per connection.
    let buf_size = if min_buf > 0 {
        (min_buf as usize).clamp(256, AUDIO_PUMP_BUF_SIZE)
    } else {
        AUDIO_PUMP_BUF_SIZE
    };
    let mut buf = vec![0u8; buf_size];

    // 3. Pump loop: read PCM from socket, push into AudioTrack.
    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) => break, // guest closed the socket — clean shutdown
            Ok(n) => n,
            Err(e) => {
                warning!("[KR64][audio][playback] socket read error: {}", e);
                break;
            }
        };

        // JNI up-call: AudioService.writeAudioData(track, buf, 0, n)
        let written = jni_write_audio_data(track, &buf[..n]);
        if written <= 0 {
            warning!("[KR64][audio][playback] writeAudioData returned {}", written);
            break;
        }
    }

    // 4. Release the AudioTrack (frees the host AudioTrack + its
    //    buffer; matches VM's releaseAudioTrack).
    jni_release_audio_track(track);
    info!("[KR64][audio][playback] connection closed, track released");
    Ok(())
}

/// Capture pump: call Java `AudioRecord.read()` via the JNI stub to
/// pull mic data → write to socket. Returns when the guest closes the
/// socket (write fails) or AudioRecord returns <=0.
///
/// See the module docs for the full data-flow diagram.
fn handle_capture(stream: &mut UnixStream, header: AudioHeader) -> std::io::Result<()> {
    // 1. Acquire a host AudioRecord via the (stubbed) JNI up-call.
    //    Returns null if RECORD_AUDIO permission isn't granted or the
    //    host has no mic — in either case we just close the connection.
    let (record, min_buf) = jni_acquire_audio_record(header.sample_rate, header.channels);
    if record.is_null() {
        warning!(
            "[KR64][audio][capture] acquireAudioRecord returned null — closing connection \
             (RECORD_AUDIO permission denied, no mic, or JNI not yet wired up)"
        );
        return Ok(());
    }

    let buf_size = if min_buf > 0 {
        (min_buf as usize).clamp(256, AUDIO_PUMP_BUF_SIZE)
    } else {
        AUDIO_PUMP_BUF_SIZE
    };
    let mut buf = vec![0u8; buf_size];

    // 2. Pump loop: pull mic data from AudioRecord, push to socket.
    loop {
        // JNI up-call: AudioService.readRecordData(record, buf, 0, len)
        let n = jni_read_record_data(record, &mut buf);
        if n <= 0 {
            warning!("[KR64][audio][capture] readRecordData returned {}", n);
            break;
        }
        if let Err(e) = stream.write_all(&buf[..n as usize]) {
            warning!("[KR64][audio][capture] socket write error: {}", e);
            break;
        }
    }

    // 3. Release the AudioRecord.
    jni_release_audio_record(record);
    info!("[KR64][audio][capture] connection closed, record released");
    Ok(())
}

// ============================================================================
// JNI up-call stubs.
//
// These mirror VM's `acquireAudioTrack`, `acquireAudioRecord`,
// `writeAudioData`, `readRecordData`, `releaseAudioTrack`,
// `releaseAudioRecord` private methods (see AUDIO_SENSOR_HAL.md §1.5).
// Each one, in the real implementation, would:
//   1. Attach the current thread to the JVM (cached in a thread-local
//      so the first call on each worker thread pays the attach cost
//      and subsequent calls are free).
//   2. Find the AudioService class + method by signature.
//   3. Call the method, marshal args/returns.
//   4. Return the Java object ref (for acquire) or the int (for
//      read/write). The acquire functions also return the host's
//      min-buffer-size (via the int[1] out-param in Java) so the pump
//      can size its scratch buffer.
//
// For the skeleton these are no-ops returning null/0 — they let the
// pump loop compile and exercise the protocol, but no sound is
// actually produced. The real implementation will replace these six
// functions (likely behind a trait object so the Java side can be
// wired in without touching the pump code).
// ============================================================================

/// Opaque handle to a host `AudioTrack` or `AudioRecord` object.
///
/// In the real implementation this would be a `jni::sys::jobject`
/// (global ref). For the skeleton it's a `*mut c_void` so we can pass
/// it around without pulling in the `jni` crate. The stubs never
/// actually allocate anything, so this is always null in the skeleton.
pub type JniObject = *mut std::ffi::c_void;

/// `AudioService.acquireAudioTrack(int[] outMinBuf)` → returns a
/// playing `AudioTrack`. Stubbed: returns `(null, 0)` — no JNI in
/// skeleton.
///
/// Real implementation: attach current thread to JVM, find AudioService
/// class, call `acquireAudioTrack([I)Landroid/media/AudioTrack;`,
/// read `outMinBuf[0]` for the buffer size, return the jobject as a
/// global ref.
fn jni_acquire_audio_track(_sample_rate: u32, _channels: u16) -> (JniObject, i32) {
    // Skeleton: return null + 0. handle_playback treats null as
    // "no audio output available" and gracefully closes the connection.
    (std::ptr::null_mut(), 0)
}

/// `AudioService.acquireAudioRecord(int[] outMinBuf)` → returns a
/// recording `AudioRecord`. Stubbed: returns `(null, 0)`.
fn jni_acquire_audio_record(_sample_rate: u32, _channels: u16) -> (JniObject, i32) {
    (std::ptr::null_mut(), 0)
}

/// `AudioService.writeAudioData(AudioTrack t, byte[] buf, int off, int len)`
/// → returns bytes written. Stubbed: returns 0 (so the pump loop exits).
fn jni_write_audio_data(_track: JniObject, _buf: &[u8]) -> i32 {
    0
}

/// `AudioService.readRecordData(AudioRecord r, byte[] buf, int off, int len)`
/// → returns bytes read. Stubbed: returns 0 (so the pump loop exits).
fn jni_read_record_data(_record: JniObject, _buf: &mut [u8]) -> i32 {
    0
}

/// `AudioService.releaseAudioTrack(AudioTrack t)` — releases the host
/// AudioTrack. Stubbed: no-op.
fn jni_release_audio_track(_track: JniObject) {}

/// `AudioService.releaseAudioRecord(AudioRecord r)` — releases the host
/// AudioRecord. Stubbed: no-op.
fn jni_release_audio_record(_record: JniObject) {}

// ============================================================================
// Minimal thread pool — fixed-size, MPMC via std::sync::mpsc.
//
// We can't add `rayon` / `crossbeam` / etc. (the crate is std + libc
// only), so we roll our own. This is the classic Rust-book ThreadPool
// with a Terminate control message added for clean shutdown. Mirrors
// the implementation in binder.rs (kept private to each module to
// keep them self-contained).
// ============================================================================

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    Job(Job),
    Terminate,
}

struct Worker {
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Self {
        let thread = thread::Builder::new()
            .name("kr64-audio-worker".to_string())
            .spawn(move || loop {
                let msg = receiver.lock().unwrap().recv();
                match msg {
                    Ok(Message::Job(job)) => job(),
                    Ok(Message::Terminate) | Err(_) => break,
                }
            })
            .expect("spawn kr64 audio worker");
        Worker { thread: Some(thread) }
    }
}

/// A fixed-size thread pool. Used by [`AudioDevice`] to handle multiple
/// concurrent guest connections.
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Message>>,
}

impl ThreadPool {
    /// Create a pool with `size` worker threads. Panics if `size == 0`.
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "ThreadPool::new: size must be > 0");
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);
        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&receiver)));
        }
        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Submit a job to the pool. If all workers are busy, the job is
    /// queued until one becomes free.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if let Some(s) = &self.sender {
            if s.send(Message::Job(Box::new(f))).is_err() {
                warning!("[KR64][audio] thread pool: sender closed, job dropped");
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Send Terminate to each worker so their recv() returns and
        // they exit cleanly.
        if let Some(s) = self.sender.take() {
            for _ in &self.workers {
                let _ = s.send(Message::Terminate);
            }
        }
        // Join each worker so we don't leak threads.
        for w in &mut self.workers {
            if let Some(t) = w.thread.take() {
                let _ = t.join();
            }
        }
    }
}

// ============================================================================
// I/O helpers.
// ============================================================================

/// Read exactly `buf.len()` bytes from `s`, blocking until all bytes
/// are available or the peer closes (returns `UnexpectedEof`).
fn read_exact(s: &mut UnixStream, buf: &mut [u8]) -> std::io::Result<()> {
    let mut filled = 0;
    while filled < buf.len() {
        let n = s.read(&mut buf[filled..])?;
        if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        filled += n;
    }
    Ok(())
}

// ============================================================================
// Tests — pure-Rust, no Android/JNI deps, so they run on the host too.
// (cargo test --lib)
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::os::unix::net::UnixStream;
    use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
    use std::time::Duration;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Each test gets a UNIQUE tmpdir so parallel tests don't collide
    /// on the same socket path (which would cause EADDRINUSE on bind).
    /// Mirrors the pattern in binder.rs's test module.
    fn tmpdir() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut p = env::temp_dir();
        p.push(format!("kr64-audio-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&p).unwrap();
        p.to_string_lossy().to_string()
    }

    // -------- AudioHeader struct layout --------------------------------

    #[test]
    fn audio_header_size_is_16_bytes() {
        // Compile-time assertion exists at the top of the file
        // (`const _: () = assert!(size_of::<AudioHeader>() == 16);`),
        // but assert again at runtime for clarity in test output.
        assert_eq!(std::mem::size_of::<AudioHeader>(), AUDIO_HEADER_SIZE);
    }

    // -------- AudioHeader serialization roundtrips ---------------------

    #[test]
    fn audio_header_roundtrip_playback() {
        let h = AudioHeader::new(AudioDirection::Playback);
        assert_eq!(h.magic, AUDIO_HEADER_MAGIC);
        assert_eq!(h.direction, AUDIO_DIR_PLAYBACK);
        assert_eq!(h.sample_rate, PLAYBACK_SAMPLE_RATE);
        assert_eq!(h.channels, PLAYBACK_CHANNELS);

        let bytes = h.to_bytes();
        assert_eq!(bytes.len(), AUDIO_HEADER_SIZE);

        let h2 = AudioHeader::from_bytes(&bytes).expect("from_bytes");
        assert_eq!(h, h2);
    }

    #[test]
    fn audio_header_roundtrip_capture() {
        let h = AudioHeader::new(AudioDirection::Capture);
        assert_eq!(h.direction, AUDIO_DIR_CAPTURE);
        assert_eq!(h.sample_rate, CAPTURE_SAMPLE_RATE);
        assert_eq!(h.channels, CAPTURE_CHANNELS);

        let bytes = h.to_bytes();
        let h2 = AudioHeader::from_bytes(&bytes).expect("from_bytes");
        assert_eq!(h, h2);
    }

    #[test]
    fn audio_header_roundtrip_custom_format() {
        // VM ignores these fields, but the protocol still has to
        // round-trip them faithfully (a guest that asks for 48 kHz
        // 5.1 should be able to read its own header back).
        let h = AudioHeader::with_format(AudioDirection::Playback, 48_000, 6);
        let bytes = h.to_bytes();
        let h2 = AudioHeader::from_bytes(&bytes).expect("from_bytes");
        assert_eq!(h, h2);
        assert_eq!(h2.sample_rate, 48_000);
        assert_eq!(h2.channels, 6);
    }

    #[test]
    fn audio_header_reserved_bytes_are_zero() {
        // The wire format reserves bytes 5..8 and 14..16 — make sure
        // to_bytes always writes zero there (a future Rust struct
        // field added in those slots must not leak into the wire).
        let h = AudioHeader::new(AudioDirection::Playback);
        let b = h.to_bytes();
        assert_eq!(&b[5..8], &[0, 0, 0], "reserved bytes 5..8 must be zero");
        assert_eq!(&b[14..16], &[0, 0], "reserved bytes 14..16 must be zero");
    }

    #[test]
    fn audio_header_from_bytes_ignores_trailing_bytes() {
        // The caller may pass a buffer that's longer than 16 bytes
        // (e.g. the header + the first chunk of PCM). Only the first
        // 16 bytes should be consumed.
        let h = AudioHeader::new(AudioDirection::Playback);
        let mut buf = h.to_bytes().to_vec();
        buf.extend_from_slice(&[0xAA; 32]); // fake PCM payload
        let h2 = AudioHeader::from_bytes(&buf).expect("from_bytes");
        assert_eq!(h, h2);
    }

    // -------- AudioHeader validation -----------------------------------

    #[test]
    fn audio_header_from_bytes_rejects_short_buffer() {
        let short = [0u8; 8];
        let r = AudioHeader::from_bytes(&short);
        assert_eq!(r, Err(AudioHeaderError::TooShort { got: 8 }));
    }

    #[test]
    fn audio_header_from_bytes_rejects_bad_magic() {
        let mut b = AudioHeader::new(AudioDirection::Playback).to_bytes();
        // Corrupt magic at offset 0..4.
        b[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
        let r = AudioHeader::from_bytes(&b);
        assert_eq!(r, Err(AudioHeaderError::BadMagic { got: 0xDEADBEEF }));
    }

    #[test]
    fn audio_header_from_bytes_rejects_bad_direction() {
        let mut b = AudioHeader::new(AudioDirection::Playback).to_bytes();
        b[4] = 99; // invalid direction
        let r = AudioHeader::from_bytes(&b);
        assert_eq!(r, Err(AudioHeaderError::BadDirection { got: 99 }));
    }

    #[test]
    fn audio_header_from_bytes_rejects_zero_direction() {
        let mut b = AudioHeader::new(AudioDirection::Playback).to_bytes();
        b[4] = 0; // 0 is not a valid direction
        let r = AudioHeader::from_bytes(&b);
        assert_eq!(r, Err(AudioHeaderError::BadDirection { got: 0 }));
    }

    #[test]
    fn audio_header_is_valid_checks_magic_and_direction() {
        let good = AudioHeader::new(AudioDirection::Playback);
        assert!(good.is_valid());

        let mut bad_magic = good;
        bad_magic.magic = 0x12345678;
        assert!(!bad_magic.is_valid());

        let mut bad_dir = good;
        bad_dir.direction = 99;
        assert!(!bad_dir.is_valid());
    }

    #[test]
    fn audio_header_error_display_is_informative() {
        let e = AudioHeaderError::TooShort { got: 4 };
        assert!(e.to_string().contains("4"));
        assert!(e.to_string().contains("16"));

        let e = AudioHeaderError::BadMagic { got: 0xDEADBEEF };
        assert!(e.to_string().contains("0xDEADBEEF"));
        assert!(e.to_string().contains("0x4F445541"));

        let e = AudioHeaderError::BadDirection { got: 99 };
        assert!(e.to_string().contains("99"));
        assert!(e.to_string().contains("Playback"));
    }

    // -------- AudioDirection enum --------------------------------------

    #[test]
    fn audio_direction_from_u8_roundtrip() {
        assert_eq!(
            AudioDirection::from_u8(AUDIO_DIR_PLAYBACK),
            Some(AudioDirection::Playback)
        );
        assert_eq!(
            AudioDirection::from_u8(AUDIO_DIR_CAPTURE),
            Some(AudioDirection::Capture)
        );
        assert_eq!(AudioDirection::from_u8(0), None);
        assert_eq!(AudioDirection::from_u8(3), None);
        assert_eq!(AudioDirection::from_u8(255), None);
    }

    #[test]
    fn audio_direction_repr_matches_wire_byte() {
        // `#[repr(u8)]` makes `as u8` give the exact wire value.
        assert_eq!(AudioDirection::Playback as u8, AUDIO_DIR_PLAYBACK);
        assert_eq!(AudioDirection::Capture as u8, AUDIO_DIR_CAPTURE);
    }

    #[test]
    fn audio_direction_defaults_match_vm() {
        // VM hard-codes these (see AUDIO_SENSOR_HAL.md §1.6).
        // Don't change them without breaking protocol compat.
        assert_eq!(AudioDirection::Playback.default_sample_rate(), 44_100);
        assert_eq!(AudioDirection::Playback.default_channels(), 2);
        assert_eq!(AudioDirection::Capture.default_sample_rate(), 11_025);
        assert_eq!(AudioDirection::Capture.default_channels(), 1);
    }

    // -------- create_audio_device --------------------------------------

    #[test]
    fn create_audio_device_creates_socket() {
        let rootfs = tmpdir();
        let dev = create_audio_device(&rootfs).expect("create_audio_device");

        assert!(Path::new(&dev.path).exists(), "socket file should exist");
        assert!(dev.path.ends_with("/dev/audio"));
        assert!(dev.raw_fd() >= 0);

        // drop should unlink the socket file (DeviceSocket::Drop).
        let path = dev.path.clone();
        drop(dev);
        assert!(!Path::new(&path).exists(), "socket file should be unlinked on drop");

        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn create_audio_device_creates_dev_dir_if_missing() {
        let rootfs = tmpdir();
        // {rootfs}/dev doesn't exist yet — create_audio_device must
        // make it.
        assert!(!Path::new(&format!("{}/dev", rootfs)).exists());
        let dev = create_audio_device(&rootfs).expect("create_audio_device");
        assert!(Path::new(&format!("{}/dev", rootfs)).exists());
        assert!(Path::new(&dev.path).exists());
        drop(dev);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn create_audio_device_replaces_stale_socket() {
        let rootfs = tmpdir();
        // First bind.
        let dev1 = create_audio_device(&rootfs).expect("first create");
        let path = dev1.path.clone();
        drop(dev1);
        // path is now unlinked by Drop. But simulate a stale socket
        // by creating a regular file at the same path.
        fs::write(&path, b"stale").unwrap();
        assert!(Path::new(&path).exists());

        // Second bind should succeed (the stale file is removed first).
        let dev2 = create_audio_device(&rootfs).expect("second create over stale");
        assert!(Path::new(&path).exists());
        drop(dev2);
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- AudioDevice::spawn + connection handling ----------------

    #[test]
    fn audio_device_spawn_accepts_playback_connection() {
        let rootfs = tmpdir();
        let dev = create_audio_device(&rootfs).expect("create_audio_device");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");

        // Give the accept thread a moment to start.
        std::thread::sleep(Duration::from_millis(50));

        // Connect and send a valid playback header. The stubbed
        // jni_acquire_audio_track returns (null, 0), so the handler
        // logs a warning and returns Ok — the stream is closed by the
        // handler side.
        let mut stream = UnixStream::connect(&path).expect("connect");
        let hdr = AudioHeader::new(AudioDirection::Playback);
        stream.write_all(&hdr.to_bytes()).expect("write header");

        // Wait for the worker to process the connection.
        std::thread::sleep(Duration::from_millis(50));
        drop(stream);

        // A second connection should still work (worker pool doesn't
        // leak threads).
        let mut s2 = UnixStream::connect(&path).expect("connect 2");
        let hdr2 = AudioHeader::new(AudioDirection::Capture);
        s2.write_all(&hdr2.to_bytes()).expect("write header 2");
        std::thread::sleep(Duration::from_millis(50));

        drop(s2);
        drop(handle);
        assert!(!Path::new(&path).exists(), "socket unlinked on handle drop");
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn audio_device_spawn_rejects_bad_header() {
        let rootfs = tmpdir();
        let dev = create_audio_device(&rootfs).expect("create_audio_device");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");

        std::thread::sleep(Duration::from_millis(50));

        // Send a header with a bad magic — the handler should return
        // Err (logged as a warning), then drop the stream.
        let mut stream = UnixStream::connect(&path).expect("connect");
        let mut bad = AudioHeader::new(AudioDirection::Playback).to_bytes();
        bad[0..4].copy_from_slice(&0xBADC0FFEu32.to_le_bytes());
        stream.write_all(&bad).expect("write bad header");

        // Wait for the worker to process and close.
        std::thread::sleep(Duration::from_millis(50));

        // The server side has closed the stream — a subsequent read
        // should return 0 (EOF) or a connection-reset error.
        let mut buf = [0u8; 4];
        let _ = stream.read(&mut buf); // may return 0 or Err — both are fine

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn audio_device_spawn_rejects_short_header() {
        let rootfs = tmpdir();
        let dev = create_audio_device(&rootfs).expect("create_audio_device");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");

        std::thread::sleep(Duration::from_millis(50));

        // Connect and send only 4 bytes (less than the 16-byte
        // header), then close. The handler's read_exact should hit
        // UnexpectedEof and return Err.
        let mut stream = UnixStream::connect(&path).expect("connect");
        stream.write_all(&[1, 2, 3, 4]).expect("write short");
        // Close the write side so the server's read returns EOF.
        let _ = stream.shutdown(std::net::Shutdown::Write);

        std::thread::sleep(Duration::from_millis(50));

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn audio_device_handle_shutdown_joins_thread() {
        let rootfs = tmpdir();
        let dev = create_audio_device(&rootfs).expect("create_audio_device");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");
        std::thread::sleep(Duration::from_millis(50));

        // Explicit shutdown — should signal the accept thread.
        handle.shutdown();
        // Drop joins the accept thread.
        drop(handle);

        // Socket file should be unlinked by the handle's Drop.
        assert!(!Path::new(&path).exists());
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn audio_device_handle_drop_without_shutdown_still_joins() {
        let rootfs = tmpdir();
        let dev = create_audio_device(&rootfs).expect("create_audio_device");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");
        std::thread::sleep(Duration::from_millis(50));

        // Don't call shutdown() — just drop. Drop's first action is
        // to set the shutdown flag, then join.
        drop(handle);

        assert!(!Path::new(&path).exists());
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- ThreadPool -----------------------------------------------

    #[test]
    fn thread_pool_executes_jobs() {
        let pool = ThreadPool::new(2);
        let counter = Arc::new(AtomicI32::new(0));
        let c1 = Arc::clone(&counter);
        let c2 = Arc::clone(&counter);
        pool.execute(move || {
            c1.fetch_add(1, Ordering::SeqCst);
        });
        pool.execute(move || {
            c2.fetch_add(1, Ordering::SeqCst);
        });

        // Drop the pool — Drop joins all workers, which guarantees
        // the jobs have completed before the assert runs.
        drop(pool);

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn thread_pool_queues_jobs_beyond_worker_count() {
        // 2 workers, 5 jobs — all should run because they're queued.
        let pool = ThreadPool::new(2);
        let counter = Arc::new(AtomicI32::new(0));
        for _ in 0..5 {
            let c = Arc::clone(&counter);
            pool.execute(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        drop(pool);
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[test]
    #[should_panic(expected = "size must be > 0")]
    fn thread_pool_new_zero_panics() {
        let _ = ThreadPool::new(0);
    }

    // -------- I/O helper ------------------------------------------------

    #[test]
    fn read_exact_returns_eof_on_short_read() {
        let rootfs = tmpdir();
        let dev = create_audio_device(&rootfs).expect("create_audio_device");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");
        std::thread::sleep(Duration::from_millis(50));

        // Connect and write 4 bytes (less than 16). The handler will
        // call read_exact for the 16-byte header, which should hit
        // UnexpectedEof when the guest closes.
        let mut stream = UnixStream::connect(&path).expect("connect");
        stream.write_all(&[1, 2, 3, 4]).expect("write short");
        let _ = stream.shutdown(std::net::Shutdown::Write);

        std::thread::sleep(Duration::from_millis(50));

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }
}
