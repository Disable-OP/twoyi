// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! Virtual `/dev/sensors` — multiplexed 12-sensor HAL.
//!
//! # Overview
//!
//! This module mirrors what Virtual Master's `libvm.so` does for sensor
//! HAL virtualization (see `download/AUDIO_SENSOR_HAL.md` §2): it owns
//! a Unix-domain socket at `{rootfs}/dev/sensors`, accepts the guest's
//! `sensorservice` connection, and proxies 12 sensor types between the
//! guest and the host's `SensorManager`.
//!
//! VM's `SensorService` lives under `HALManager` (unlike `AudioService`
//! which is top-level) — sensor events are bursty and low-rate, so they
//! don't need the dedicated real-time thread that audio does. Twoyi
//! mirrors that split: this module shares the same `ThreadPool` pattern
//! as `audio.rs`, but each connection spawns only one extra sub-thread
//! (the event pump) instead of dedicating a worker to a long-running
//! pump loop.
//!
//! # The 12-sensor mapping
//!
//! Decompiled verbatim from `SensorService.java` lines 61–74 (the
//! `static {}` block). The `SparseIntArray f9098WWWW` maps **guest
//! sensor index (0..11) → host `Sensor.TYPE_*`** (see
//! [`SENSOR_TYPE_MAP`] / [`SensorType`]):
//!
//! ```text
//!  Guest idx | Sensor.TYPE_* | Constant name                | Notes
//!  ---------:|--------------:|------------------------------|----------------------
//!      0     |   1           | TYPE_ACCELEROMETER           | auto-rotate + games
//!      1     |   2           | TYPE_MAGNETIC_FIELD          | compass
//!      2     |   3           | TYPE_ORIENTATION             | deprecated, emulated
//!      3     |   7           | TYPE_TEMPERATURE             | ambient
//!      4     |   8           | TYPE_LIGHT                   | ambient light
//!      5     |   5           | TYPE_PROXIMITY               | face-detect
//!      6     |   6           | TYPE_GYROSCOPE               | VR/games
//!      7     |  12           | TYPE_RELATIVE_HUMIDITY       | rare
//!      8     |   9           | TYPE_PRESSURE                | barometer
//!      9     |  19           | TYPE_GRAVITY                 | derived
//!     10     |  18           | TYPE_STEP_DETECTOR           | pedometer
//!     11     |   4           | TYPE_GYROSCOPE_UNCALIBRATED  | raw gyro + bias
//! ```
//!
//! The order is **not** contiguous by `TYPE_*` value — it's the order
//! the guest's sensor HAL expects to enumerate them. The guest opens
//! the virtual `/dev/sensors` device and queries each sensor by index
//! 0..11; the Java side translates that index to the host's
//! `Sensor.TYPE_*` via this table.
//!
//! # Wire protocol
//!
//! ## Control messages (guest → host, 12 bytes each)
//!
//! ```text
//!  offset  size  field   description
//!  ------  ----  ------  ------------------------------------------
//!   0       4    cmd     LE u32 — 1=ENABLE, 2=DISABLE,
//!                          3=CHECK_SUPPORT, 4=SET_DELAY
//!   4       4    idx     LE u32 — sensor index 0..11
//!   8       4    arg     LE u32 — for SET_DELAY: sampling period
//!                          in nanoseconds; ignored otherwise
//!  ------  ----
//!  total = 12 bytes (see [`SENSOR_CONTROL_SIZE`])
//! ```
//!
//! For `CHECK_SUPPORT`, the host replies with a 4-byte LE u32 (0 or 1)
//! indicating whether the host has that sensor. The other commands
//! have no reply.
//!
//! ## Sensor events (host → guest, 24 bytes each)
//!
//! ```text
//!  offset  size  field   description
//!  ------  ----  ------  ------------------------------------------
//!   0       4    idx     LE u32 — sensor index 0..11
//!   4       8    ts      LE u64 — timestamp in nanoseconds
//!                          (SystemClock.elapsedRealtimeNanos())
//!  12       4    x       LE f32 — x-axis value
//!  16       4    y       LE f32 — y-axis value
//!  20       4    z       LE f32 — z-axis value (unused slots are 0)
//!  ------  ----
//!  total = 24 bytes (see [`SENSOR_EVENT_SIZE`])
//! ```
//!
//! # Data flow (host → guest)
//!
//! ```text
//!   Host Android                                  Guest Android
//!   ┌──────────────────────────────────────┐    ┌────────────────────────────────────┐
//!   │  SensorManager (system service)      │    │  sensorservice (native daemon)     │
//!   │    SensorEventListener callbacks     │    │    sensor HAL module               │
//!   │       │                              │    │       │                            │
//!   │       ▼ onSensorChanged(SensorEvent) │    │       │ poll(/dev/sensors)         │
//!   │  SensorService.onSensorChanged       │    │       │   (blocking read 24 bytes) │
//!   │    │ posts Runnable to HandlerThread │    │       ▼                            │
//!   │    ▼                                 │    │   struct sensor_event {            │
//!   │  HALManager.SensorChanged(idx, ts,   │    │     idx, ts, x, y, z              │
//!   │                            x, y, z)  │    │   }                               │
//!   │    │                                 │    │       │                            │
//!   │    ▼ JNI down-call                   │    │       ▼                            │
//!   │  nativeSensorChanged(ptr, idx, ts,   │    │   dispatch into sensor framework   │
//!   │                       x, y, z)       │    │   → SensorEventListener in guest   │
//!   │    │                                 │    │                                     │
//!   │    ▼ libvm.so / kr64                 │    │                                     │
//!   │  write(/dev/sensors,                 │───▶│                                     │
//!   │        &sensor_event, 24)            │    │                                     │
//!   └──────────────────────────────────────┘    └────────────────────────────────────┘
//! ```
//!
//! In the skeleton the host side of this flow is stubbed: the JNI
//! down-call `nativeSensorChanged` isn't wired up, and the pump thread
//! polls [`jni_read_sensor_event`] which always returns `None`. So no
//! events actually flow to the guest until SENSOR-IMPL-2 replaces the
//! stubs with real `SensorManager` integration.
//!
//! # The 3-bit state machine
//!
//! Each of the 12 sensors has a 3-bit state mask (see [`SensorState`]):
//!
//!  | Bit | Mask | Name      | Meaning                                              |
//!  |----:|-----:|-----------|------------------------------------------------------|
//!  |  0  |   1  | SUPPORTED | Host has this sensor (`getDefaultSensor()` ≠ null)  |
//!  |  1  |   2  | ENABLED   | Guest requested enable                               |
//!  |  2  |   4  | ACTIVE    | `SensorManager.registerListener` was called         |
//!
//! State transitions (mirroring `HALManager.java`):
//! - **`CheckSensorsSupport(idx)`** (line 178) → `(state[idx] & 1) == 1`.
//! - **`EnableSensors(idx)`** (line 200) → if `(state[idx] & 2) == 2`,
//!   set `state[idx] |= 4` and `registerListener`.
//! - **`DisableSensors(idx)`** (line 187) → `state[idx] &= ~4` and
//!   `unregisterListener`.
//! - **`SetDelay(idx, delay)`** (line 561) → updates the sampling
//!   period. (VM has a quirk where this zeroes both the period and
//!   max-latency arrays, then uses `1` if `0`. Twoyi treats this as
//!   a normal set-with-floor — see [`MIN_POLL_NS`].)
//!
//! # JNI callback interface
//!
//! The actual `SensorManager` integration lives on the Java side (a
//! future `io.twoyi.hal.SensorService.java` modeled on VM's
//! `com.android.vmcore.hal.SensorService`, 160 lines). This Rust
//! module invokes it via five JNI up-calls (stubs in this skeleton):
//!
//!  | Rust function (stub here)         | Java method on `HALManager` / `SensorService`           | Returns                  |
//!  |-----------------------------------|----------------------------------------------------------|--------------------------|
//!  | [`jni_check_sensor_support`]      | `boolean CheckSensorsSupport(int idx)`                   | `bool`                   |
//!  | [`jni_enable_sensor`]             | `boolean EnableSensors(int idx)`                         | `bool` (success)         |
//!  | [`jni_disable_sensor`]            | `void DisableSensors(int idx)`                           | `()`                     |
//!  | [`jni_set_sensor_delay`]          | `void SetDelay(int idx, int delayNs)`                    | `()`                     |
//!  | [`jni_read_sensor_event`]         | (down-call from `nativeSensorChanged` queue)             | `Option<SensorEvent>`    |
//!
//! Each up-call would attach the current thread to the JVM (cached in
//! a thread-local), find the `HALManager` class + method by signature,
//! marshal the args, and return. For the skeleton these are **stubs
//! that return `false`/`None`/`()`** — they let the control + pump
//! loops compile and exercise the protocol, but no real sensor data
//! flows. The real implementation will replace these five functions.
//!
//! # Threading
//!
//! One accept thread + a fixed-size [`ThreadPool`] of control workers
//! (mirrors `audio.rs` / `binder.rs`). Each guest connection is
//! dispatched to a worker, which:
//!
//! 1. Clones the stream via `UnixStream::try_clone()` (two file
//!    descriptors over the same underlying socket — one for reads,
//!    one for writes).
//! 2. Spawns a sub-thread (`kr64-sensor-pump`) that runs
//!    [`pump_events`] — polls enabled sensors via the JNI stub and
//!    writes 24-byte `SensorEvent`s to the guest.
//! 3. Runs [`handle_control`] in the worker thread itself — reads
//!    12-byte control requests, dispatches to the JNI stubs, mutates
//!    the shared `SensorConnState`, and writes 4-byte replies for
//!    `CHECK_SUPPORT`.
//! 4. When the control read returns EOF (guest disconnected), the
//!    worker sets the per-connection shutdown flag, joins the pump
//!    sub-thread, and returns.
//!
//! Because the guest's `sensorservice` typically holds a single
//! connection for the lifetime of the guest, the pool size
//! ([`SENSOR_THREAD_POOL_SIZE`] = 4) only needs to cover the case
//! where the guest reconnects (e.g. after a guest reboot) before the
//! old connection's worker has fully cleaned up.
//!
//! # Skeleton scope
//!
//! What's implemented here:
//! - The full wire protocol (12-byte control + 24-byte event).
//! - The 12-sensor index ↔ type mapping table (verified against the
//!   decompiled `SensorService.java` static block).
//! - The 3-bit `SensorState` bitflags.
//! - The accept thread + worker pool + per-connection pump sub-thread.
//! - The five JNI up-call stubs.
//!
//! What's NOT implemented (deferred to SENSOR-IMPL-2):
//! - The actual `SensorManager.registerListener` / `onSensorChanged`
//!   wiring on the Java side.
//! - The `nativeSensorChanged` JNI down-call entry point (Java → Rust).
//! - The manifest permission `HIGH_SAMPLING_RATE_SENSORS` (only needed
//!   for >200 Hz sensors; accel/gyro/mag don't need it).

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// Crate-local logging macros (defined in lib.rs) — no external `log` crate.
#[allow(unused_imports)]
use crate::{error, info, warning};

// ============================================================================
// Constants
// ============================================================================

/// Number of sensors the guest's sensor HAL enumerates. Matches VM's
/// `SensorService` static `SparseIntArray` (12 entries, indices 0..11).
/// See `download/AUDIO_SENSOR_HAL.md` §2.1.
pub const NUM_SENSORS: usize = 12;

/// Control command: enable a sensor (start sending events for it).
/// Wire value at offset 0 of the 12-byte control message.
pub const CTL_ENABLE: u32 = 1;

/// Control command: disable a sensor (stop sending events).
pub const CTL_DISABLE: u32 = 2;

/// Control command: query whether a sensor is supported. Host replies
/// with a 4-byte LE u32 (0 or 1).
pub const CTL_CHECK_SUPPORT: u32 = 3;

/// Control command: set the sampling period for a sensor.
/// The `arg` field carries the period in nanoseconds.
pub const CTL_SET_DELAY: u32 = 4;

/// Total size of a control message on the wire (3 × u32).
pub const SENSOR_CONTROL_SIZE: usize = 12;

/// Total size of a sensor event on the wire
/// (`u32 idx + u64 ts + 3 × f32` = 4 + 8 + 12 = 24 bytes).
/// See `download/AUDIO_SENSOR_HAL.md` §2.2.
pub const SENSOR_EVENT_SIZE: usize = 24;

/// Number of worker threads in the sensor connection pool. The guest's
/// `sensorservice` typically holds a single connection for the
/// lifetime of the guest, so 4 is ample headroom for reconnect races.
pub const SENSOR_THREAD_POOL_SIZE: usize = 4;

/// Floor for the pump thread's sleep between polls. Prevents a
/// `SET_DELAY 0` request from causing a busy-loop. VM has a quirk
/// where it uses `1` (microsecond) if delay is 0 — we treat that as a
/// bug, not a feature, and clamp to 1 ms.
pub const MIN_POLL_NS: u64 = 1_000_000; // 1 ms

/// Ceiling for the pump thread's sleep between polls. Prevents a
/// huge `SET_DELAY` from making the pump unresponsive to a subsequent
/// `DISABLE`. 1 second matches the slowest realistic sensor
/// (step detector ~1 Hz).
pub const MAX_POLL_NS: u64 = 1_000_000_000; // 1 s

/// Sleep duration for the pump thread when no sensors are enabled.
/// Keeps the thread alive but idle.
pub const SENSOR_IDLE_POLL_MS: u64 = 50;

// ============================================================================
// SensorType enum + index↔type mapping
// ============================================================================

/// The 12 host `Sensor.TYPE_*` constants that the guest's sensor HAL
/// can enumerate. Values match `android.hardware.Sensor.TYPE_*`
/// exactly (verified against the decompiled `SensorService.java`
/// static block — see `download/AUDIO_SENSOR_HAL.md` §2.1).
///
/// `#[repr(u32)]` so `as u32` gives the exact `Sensor.TYPE_*` value.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SensorType {
    /// `Sensor.TYPE_ACCELEROMETER` (1). Most-used; auto-rotate + games.
    Accelerometer = 1,
    /// `Sensor.TYPE_MAGNETIC_FIELD` (2). Compass.
    MagneticField = 2,
    /// `Sensor.TYPE_ORIENTATION` (3). Deprecated in API 8, still emulated.
    Orientation = 3,
    /// `Sensor.TYPE_GYROSCOPE_UNCALIBRATED` (4). Raw gyro + bias.
    GyroscopeUncalibrated = 4,
    /// `Sensor.TYPE_PROXIMITY` (5). Face-detect during calls.
    Proximity = 5,
    /// `Sensor.TYPE_GYROSCOPE` (6). VR/games.
    Gyroscope = 6,
    /// `Sensor.TYPE_TEMPERATURE` (7). Ambient.
    Temperature = 7,
    /// `Sensor.TYPE_LIGHT` (8). Ambient light.
    Light = 8,
    /// `Sensor.TYPE_PRESSURE` (9). Barometer.
    Pressure = 9,
    /// `Sensor.TYPE_RELATIVE_HUMIDITY` (12). Rare.
    RelativeHumidity = 12,
    /// `Sensor.TYPE_STEP_DETECTOR` (18). Pedometer.
    StepDetector = 18,
    /// `Sensor.TYPE_GRAVITY` (19). Derived from accel+gyro.
    Gravity = 19,
}

impl SensorType {
    /// Parse a raw `Sensor.TYPE_*` value. Returns `None` for unknown
    /// values (i.e. types VM doesn't virtualise).
    pub fn from_type_value(v: u32) -> Option<Self> {
        match v {
            1 => Some(Self::Accelerometer),
            2 => Some(Self::MagneticField),
            3 => Some(Self::Orientation),
            4 => Some(Self::GyroscopeUncalibrated),
            5 => Some(Self::Proximity),
            6 => Some(Self::Gyroscope),
            7 => Some(Self::Temperature),
            8 => Some(Self::Light),
            9 => Some(Self::Pressure),
            12 => Some(Self::RelativeHumidity),
            18 => Some(Self::StepDetector),
            19 => Some(Self::Gravity),
            _ => None,
        }
    }

    /// The `Sensor.TYPE_*` numeric constant.
    pub fn type_value(self) -> u32 {
        self as u32
    }
}

/// Guest sensor index (0..11) → host `Sensor.TYPE_*` value.
/// Mirrors VM's `SensorService` static `SparseIntArray` verbatim
/// (see `download/AUDIO_SENSOR_HAL.md` §2.1, table rows 0..11).
///
/// Order matters: the guest enumerates by index, and the Java side
/// translates via this exact table. Changing the order would break
/// the guest's `SensorManager.getSensorList()` enumeration.
pub const SENSOR_TYPE_MAP: [SensorType; NUM_SENSORS] = [
    SensorType::Accelerometer,         // idx 0  → TYPE_ACCELEROMETER (1)
    SensorType::MagneticField,         // idx 1  → TYPE_MAGNETIC_FIELD (2)
    SensorType::Orientation,           // idx 2  → TYPE_ORIENTATION (3)
    SensorType::Temperature,           // idx 3  → TYPE_TEMPERATURE (7)
    SensorType::Light,                 // idx 4  → TYPE_LIGHT (8)
    SensorType::Proximity,             // idx 5  → TYPE_PROXIMITY (5)
    SensorType::Gyroscope,             // idx 6  → TYPE_GYROSCOPE (6)
    SensorType::RelativeHumidity,      // idx 7  → TYPE_RELATIVE_HUMIDITY (12)
    SensorType::Pressure,              // idx 8  → TYPE_PRESSURE (9)
    SensorType::Gravity,               // idx 9  → TYPE_GRAVITY (19)
    SensorType::StepDetector,          // idx 10 → TYPE_STEP_DETECTOR (18)
    SensorType::GyroscopeUncalibrated, // idx 11 → TYPE_GYROSCOPE_UNCALIBRATED (4)
];

/// Map a guest sensor index (0..11) to the host `SensorType`.
/// Returns `None` for out-of-range indices.
pub fn index_to_type(idx: u32) -> Option<SensorType> {
    if (idx as usize) < NUM_SENSORS {
        Some(SENSOR_TYPE_MAP[idx as usize])
    } else {
        None
    }
}

/// Reverse mapping: host `SensorType` → guest sensor index (0..11).
/// Returns `None` if the type isn't one of the 12 VM virtualises.
/// (Used in tests; not needed in production where the guest always
/// addresses sensors by index, not by type.)
pub fn type_to_index(sensor_type: SensorType) -> Option<u32> {
    SENSOR_TYPE_MAP
        .iter()
        .position(|&t| t == sensor_type)
        .map(|i| i as u32)
}

// ============================================================================
// SensorEvent struct (24 bytes on the wire, packed)
// ============================================================================

/// A single sensor event, 24 bytes on the wire.
///
/// Wire layout (little-endian, packed — see module docs for the full
/// table):
/// ```text
///   off 0:  idx  u32 LE — sensor index 0..11
///   off 4:  ts   u64 LE — timestamp in nanoseconds
///   off 12: x    f32 LE
///   off 16: y    f32 LE
///   off 20: z    f32 LE
/// ```
///
/// `#[repr(C, packed)]` makes the in-memory layout match the wire
/// layout (no padding between `idx` and `ts`). The compile-time
/// assertion below guarantees `size_of::<SensorEvent>() == 24`. For
/// actual (de)serialisation we use explicit byte slicing via
/// [`SensorEvent::to_bytes`] / [`SensorEvent::from_bytes`] so the
/// wire format is deterministic regardless of host endianness or
/// padding (and so we never take a reference to a mis-aligned field
/// — which `#[repr(packed)]` would make unsafe).
#[repr(C, packed)]
pub struct SensorEvent {
    /// Offset 0. Sensor index 0..11 (matches the guest's enumeration).
    pub idx: u32,
    /// Offset 4. Timestamp in nanoseconds — should be
    /// `SystemClock.elapsedRealtimeNanos()` on the host.
    pub ts: u64,
    /// Offset 12. X-axis value (units depend on sensor type).
    pub x: f32,
    /// Offset 16. Y-axis value.
    pub y: f32,
    /// Offset 20. Z-axis value. Unused slots are 0.
    pub z: f32,
}

// Note: #[derive(Debug, Clone, Copy, PartialEq)] removed because deriving
// these traits on a #[repr(packed)] struct with misaligned fields (ts at
// offset 4) is undefined behavior — the derived impls take &self.ts which
// is a reference to a misaligned u64. Use to_bytes()/from_bytes() for
// serialization instead.
//
// The trait impls below are hand-written to read each field through
// `std::ptr::addr_of!(...).read_unaligned()`, which never materialises a
// misaligned reference (it does a byte-wise copy into a local, aligned
// temporary). This is the only sound way to implement these traits for a
// `#[repr(packed)]` struct.

impl Clone for SensorEvent {
    // For a `Copy` type, the canonical `clone` impl is just `*self` (a
    // bit-for-bit copy). On `#[repr(packed)]` this is still sound: taking
    // `&Self` (the whole struct) is always aligned — only borrows of
    // individual misaligned fields are UB, and `*self` doesn't create any.
    fn clone(&self) -> Self {
        *self
    }
}

// Copy is sound: SensorEvent is 24 bytes of plain Copy types (u32/u64/f32)
// and contains no heap pointers or destructors. A bit-for-bit copy is a
// valid duplicate.
impl Copy for SensorEvent {}

impl PartialEq for SensorEvent {
    fn eq(&self, other: &Self) -> bool {
        // Compare field-by-field via aligned temporaries. We deliberately
        // avoid `self.field == other.field` because, for packed structs,
        // even an implicit borrow during method dispatch can be unsound —
        // going through `addr_of!` makes the (unaligned) read explicit.
        let (s_idx, s_ts, s_x, s_y, s_z) = unsafe {
            (
                std::ptr::addr_of!(self.idx).read_unaligned(),
                std::ptr::addr_of!(self.ts).read_unaligned(),
                std::ptr::addr_of!(self.x).read_unaligned(),
                std::ptr::addr_of!(self.y).read_unaligned(),
                std::ptr::addr_of!(self.z).read_unaligned(),
            )
        };
        let (o_idx, o_ts, o_x, o_y, o_z) = unsafe {
            (
                std::ptr::addr_of!(other.idx).read_unaligned(),
                std::ptr::addr_of!(other.ts).read_unaligned(),
                std::ptr::addr_of!(other.x).read_unaligned(),
                std::ptr::addr_of!(other.y).read_unaligned(),
                std::ptr::addr_of!(other.z).read_unaligned(),
            )
        };
        s_idx == o_idx && s_ts == o_ts && s_x == o_x && s_y == o_y && s_z == o_z
    }
}

impl std::fmt::Debug for SensorEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Read each field into an aligned local first, then debug-format
        // the locals (formatting would otherwise need `&self.field`).
        let idx = unsafe { std::ptr::addr_of!(self.idx).read_unaligned() };
        let ts = unsafe { std::ptr::addr_of!(self.ts).read_unaligned() };
        let x = unsafe { std::ptr::addr_of!(self.x).read_unaligned() };
        let y = unsafe { std::ptr::addr_of!(self.y).read_unaligned() };
        let z = unsafe { std::ptr::addr_of!(self.z).read_unaligned() };
        f.debug_struct("SensorEvent")
            .field("idx", &idx)
            .field("ts", &ts)
            .field("x", &x)
            .field("y", &y)
            .field("z", &z)
            .finish()
    }
}

// Compile-time assertion: the packed struct must be exactly 24 bytes.
const _: () = assert!(std::mem::size_of::<SensorEvent>() == SENSOR_EVENT_SIZE);

impl SensorEvent {
    /// Construct a new event with the given fields.
    pub fn new(idx: u32, ts: u64, x: f32, y: f32, z: f32) -> Self {
        Self { idx, ts, x, y, z }
    }

    /// Construct a zero-initialised event (all fields 0). Useful as
    /// a placeholder in tests.
    pub fn zero() -> Self {
        Self {
            idx: 0,
            ts: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// True if `idx` is in range 0..[`NUM_SENSORS`].
    pub fn is_valid(&self) -> bool {
        (self.idx as usize) < NUM_SENSORS
    }

    /// Serialise to a 24-byte little-endian array. Reads each field
    /// by value (safe on `#[repr(packed)]` — we never take a
    /// reference to a mis-aligned field).
    pub fn to_bytes(&self) -> [u8; SENSOR_EVENT_SIZE] {
        let mut buf = [0u8; SENSOR_EVENT_SIZE];
        buf[0..4].copy_from_slice(&self.idx.to_le_bytes());
        buf[4..12].copy_from_slice(&self.ts.to_le_bytes());
        buf[12..16].copy_from_slice(&self.x.to_le_bytes());
        buf[16..20].copy_from_slice(&self.y.to_le_bytes());
        buf[20..24].copy_from_slice(&self.z.to_le_bytes());
        buf
    }

    /// Deserialise from a 24-byte (or longer) slice. Only the first
    /// 24 bytes are read; trailing bytes are ignored (the caller is
    /// expected to read the event out of a larger buffer).
    ///
    /// Returns an error if the buffer is too short or the `idx` field
    /// is outside 0..[`NUM_SENSORS`] (a defensive check — the host
    /// should never send an out-of-range idx, but if it does, we
    /// shouldn't silently feed garbage to the guest's sensor
    /// framework).
    pub fn from_bytes(buf: &[u8]) -> Result<Self, SensorEventError> {
        if buf.len() < SENSOR_EVENT_SIZE {
            return Err(SensorEventError::TooShort { got: buf.len() });
        }
        let idx = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let ts = u64::from_le_bytes(buf[4..12].try_into().unwrap());
        let x = f32::from_le_bytes(buf[12..16].try_into().unwrap());
        let y = f32::from_le_bytes(buf[16..20].try_into().unwrap());
        let z = f32::from_le_bytes(buf[20..24].try_into().unwrap());

        if (idx as usize) >= NUM_SENSORS {
            return Err(SensorEventError::BadIndex { got: idx });
        }

        Ok(Self { idx, ts, x, y, z })
    }
}

/// Error returned by [`SensorEvent::from_bytes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SensorEventError {
    /// Buffer was shorter than 24 bytes.
    TooShort { got: usize },
    /// `idx` field was outside 0..[`NUM_SENSORS`].
    BadIndex { got: u32 },
}

impl std::fmt::Display for SensorEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(
                f,
                "sensor event too short: got {} bytes, need {}",
                got, SENSOR_EVENT_SIZE
            ),
            Self::BadIndex { got } => write!(
                f,
                "sensor event bad idx: got {}, need 0..{}",
                got, NUM_SENSORS
            ),
        }
    }
}

impl std::error::Error for SensorEventError {}

// ============================================================================
// SensorControl struct (12 bytes on the wire)
// ============================================================================

/// A 12-byte control message sent by the guest to enable/disable a
/// sensor, query support, or set the sampling period.
///
/// Wire layout (little-endian):
/// ```text
///   off 0:  cmd  u32 LE — CTL_* constant
///   off 4:  idx  u32 LE — sensor index 0..11
///   off 8:  arg  u32 LE — for SET_DELAY: sampling period in ns
/// ```
///
/// `#[repr(C)]` (not packed) is sufficient here because all three
/// fields are `u32` — natural alignment gives no padding and the
/// in-memory size is already 12 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SensorControl {
    /// Offset 0. One of [`CTL_ENABLE`], [`CTL_DISABLE`],
    /// [`CTL_CHECK_SUPPORT`], [`CTL_SET_DELAY`].
    pub cmd: u32,
    /// Offset 4. Sensor index 0..11.
    pub idx: u32,
    /// Offset 8. For `SET_DELAY`: sampling period in ns. Ignored by
    /// other commands.
    pub arg: u32,
}

const _: () = assert!(std::mem::size_of::<SensorControl>() == SENSOR_CONTROL_SIZE);

impl SensorControl {
    /// Construct a control message.
    pub fn new(cmd: u32, idx: u32, arg: u32) -> Self {
        Self { cmd, idx, arg }
    }

    /// Serialise to a 12-byte little-endian array.
    pub fn to_bytes(&self) -> [u8; SENSOR_CONTROL_SIZE] {
        let mut buf = [0u8; SENSOR_CONTROL_SIZE];
        buf[0..4].copy_from_slice(&self.cmd.to_le_bytes());
        buf[4..8].copy_from_slice(&self.idx.to_le_bytes());
        buf[8..12].copy_from_slice(&self.arg.to_le_bytes());
        buf
    }

    /// Deserialise from a 12-byte (or longer) slice. Only the first
    /// 12 bytes are read; trailing bytes are ignored.
    pub fn from_bytes(buf: &[u8]) -> Result<Self, SensorControlError> {
        if buf.len() < SENSOR_CONTROL_SIZE {
            return Err(SensorControlError { got: buf.len() });
        }
        let cmd = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let idx = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        let arg = u32::from_le_bytes(buf[8..12].try_into().unwrap());
        Ok(Self { cmd, idx, arg })
    }
}

/// Error returned by [`SensorControl::from_bytes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorControlError {
    /// The buffer length we got (less than 12).
    pub got: usize,
}

impl std::fmt::Display for SensorControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "sensor control too short: got {} bytes, need {}",
            self.got, SENSOR_CONTROL_SIZE
        )
    }
}

impl std::error::Error for SensorControlError {}

// ============================================================================
// SensorState bitflags (3-bit mask per sensor)
// ============================================================================

/// Per-sensor state bitmask. Mirrors VM's `f9102WWWWWWWW` `int[12]`
/// array (see `download/AUDIO_SENSOR_HAL.md` §2.5). Each of the 12
/// sensors has its own `SensorState`.
///
///  | Bit | Mask | Constant    | Meaning                                              |
///  |----:|-----:|-------------|------------------------------------------------------|
///  |  0  |   1  | [`SUPPORTED`] | Host has this sensor (`getDefaultSensor()` ≠ null)  |
///  |  1  |   2  | [`ENABLED`]   | Guest requested enable                               |
///  |  2  |   4  | [`ACTIVE`]    | `SensorManager.registerListener` was called         |
///
/// This is a hand-rolled bitflags type (no `bitflags` crate — the
/// project is std + libc only). The API mirrors the `bitflags` crate's
/// API closely enough to be familiar.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct SensorState(pub u8);

impl SensorState {
    /// Bit 0 (mask `0b001`): host has this sensor.
    pub const SUPPORTED: SensorState = SensorState(0b001);
    /// Bit 1 (mask `0b010`): guest requested enable.
    pub const ENABLED: SensorState = SensorState(0b010);
    /// Bit 2 (mask `0b100`): host actually registered a listener.
    pub const ACTIVE: SensorState = SensorState(0b100);

    /// All bits set — useful for "any state" queries in tests.
    pub const ALL: SensorState = SensorState(0b111);

    /// Empty state (no bits set).
    pub const fn empty() -> Self {
        Self(0)
    }

    /// True if no bits are set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The raw `u8` bit pattern (only the low 3 bits are meaningful).
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Construct from raw bits (masked to the low 3 bits).
    pub const fn from_bits(b: u8) -> Self {
        Self(b & 0b111)
    }

    /// True if all bits in `other` are also set in `self`.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Set the bits in `other`.
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Clear the bits in `other`.
    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    /// Convenience: is [`SUPPORTED`](Self::SUPPORTED) set?
    pub const fn is_supported(self) -> bool {
        self.contains(Self::SUPPORTED)
    }

    /// Convenience: is [`ENABLED`](Self::ENABLED) set?
    pub const fn is_enabled(self) -> bool {
        self.contains(Self::ENABLED)
    }

    /// Convenience: is [`ACTIVE`](Self::ACTIVE) set?
    pub const fn is_active(self) -> bool {
        self.contains(Self::ACTIVE)
    }
}

impl std::ops::BitOr for SensorState {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for SensorState {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAnd for SensorState {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for SensorState {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl std::ops::Not for SensorState {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0 & 0b111)
    }
}

// ============================================================================
// Per-connection state (shared between control worker + pump sub-thread)
// ============================================================================

/// Inner mutable state for a single guest connection. Protected by a
/// `Mutex` inside [`SensorConnState`].
#[derive(Debug)]
struct ConnStateInner {
    /// Per-sensor ENABLED bit (mirrors bit 1 of VM's state mask).
    /// The pump thread iterates this list to decide which sensors to
    /// poll. SUPPORTED (bit 0) is queried on-demand via JNI and
    /// cached in `supported`; ACTIVE (bit 2) is implicitly true
    /// whenever `enabled[i]` is true and the pump is running.
    enabled: [bool; NUM_SENSORS],
    /// Per-sensor sampling period in nanoseconds (set by SET_DELAY).
    /// 0 means "use the default" (clamped to [`MIN_POLL_NS`] in the
    /// pump loop).
    delays_ns: [u64; NUM_SENSORS],
    /// Per-sensor SUPPORTED cache. Populated lazily by
    /// [`SensorConnState::check_support`] — the first CHECK_SUPPORT
    /// for an index calls the JNI stub, subsequent queries return
    /// the cached value. (Matches VM's `CheckSensorsSupport` which
    /// also reads a pre-populated state array.)
    supported: [bool; NUM_SENSORS],
}

impl ConnStateInner {
    fn new() -> Self {
        Self {
            enabled: [false; NUM_SENSORS],
            delays_ns: [0u64; NUM_SENSORS],
            supported: [false; NUM_SENSORS],
        }
    }
}

/// Per-connection state shared between the control worker thread and
/// the pump sub-thread. Created in [`handle_connection`], dropped
/// when the control worker exits (after joining the pump sub-thread).
///
/// The `Mutex<ConnStateInner>` protects the `enabled`/`delays_ns`/
/// `supported` arrays; the `AtomicBool` shutdown flag is read by the
/// pump thread on every iteration to know when to exit.
struct SensorConnState {
    state: Mutex<ConnStateInner>,
    shutdown: AtomicBool,
}

impl SensorConnState {
    fn new() -> Self {
        Self {
            state: Mutex::new(ConnStateInner::new()),
            shutdown: AtomicBool::new(false),
        }
    }

    /// Mark `idx` as ENABLED. Called from the control worker after
    /// `jni_enable_sensor` returns success.
    fn enable(&self, idx: usize) {
        self.state.lock().unwrap().enabled[idx] = true;
    }

    /// Mark `idx` as not ENABLED. Called from the control worker on
    /// DISABLE.
    fn disable(&self, idx: usize) {
        self.state.lock().unwrap().enabled[idx] = false;
    }

    /// Set the sampling period for `idx`. Called from the control
    /// worker on SET_DELAY.
    fn set_delay(&self, idx: usize, delay_ns: u64) {
        self.state.lock().unwrap().delays_ns[idx] = delay_ns;
    }

    /// Query whether `idx` is supported by the host. The first call
    /// for a given `idx` invokes the JNI stub and caches the result;
    /// subsequent calls return the cached value. (VM's
    /// `CheckSensorsSupport` reads a pre-populated state array that
    /// `HALManager.startHALMgr` fills at boot — twoyi's lazy approach
    /// gives the same observable behaviour with one fewer JNI call
    /// per sensor that the guest never actually queries.)
    fn check_support(&self, idx: usize) -> bool {
        let mut s = self.state.lock().unwrap();
        // Note: this calls jni_check_sensor_support *while holding the
        // mutex*. In the skeleton the stub is a no-op (returns false
        // instantly), so this is fine. In the real impl, the JNI
        // up-call might block on a `SensorManager.getDefaultSensor()`
        // call — at that point we'd want to release the mutex around
        // the JNI call, cache the result, then re-acquire the mutex.
        // For the skeleton this is a non-issue.
        if !s.supported[idx] && jni_check_sensor_support(std::ptr::null_mut(), idx as u32) {
            s.supported[idx] = true;
        }
        s.supported[idx]
    }

    /// Snapshot the enabled-sensor list as `(idx, delay_ns)` pairs.
    /// Called by the pump thread on every iteration. Cheap (12-element
    /// array scan + a small Vec allocation).
    fn snapshot(&self) -> Vec<(u32, u64)> {
        let s = self.state.lock().unwrap();
        let mut out = Vec::with_capacity(NUM_SENSORS);
        for i in 0..NUM_SENSORS {
            if s.enabled[i] {
                out.push((i as u32, s.delays_ns[i]));
            }
        }
        out
    }

    /// Ask the pump thread to exit. Called by the control worker when
    /// the guest disconnects.
    fn signal_shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    /// True if shutdown has been requested.
    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }
}

// ============================================================================
// create_sensor_device
// ============================================================================

/// Create the virtual `/dev/sensors` Unix socket inside `rootfs`.
///
/// Mirrors Virtual Master's pattern (see AUDIO_SENSOR_HAL.md §2.2):
/// the guest's `sensorservice` opens `/dev/sensors`, `connect()`s,
/// and the host's sensor pump (this module's [`SensorDevice::spawn`])
/// is on the other end of the socket.
///
/// This is the sensor-specific equivalent of
/// `audio::create_audio_device` / `devices::create_touch_device`,
/// returning a [`SensorDevice`] that owns the listener directly
/// (rather than a generic `DeviceSocket`), because the sensor pump
/// needs its own accept thread + worker pool + per-connection
/// pump sub-thread and doesn't fit the simple "echo a byte and
/// close" pattern the other MVP devices use.
///
/// # Errors
///
/// Returns an error if directory creation or `UnixListener::bind`
/// fails. Stale socket files from a previous run are best-effort
/// removed before bind (errors are logged but not propagated).
pub fn create_sensor_device(rootfs: &str) -> std::io::Result<SensorDevice> {
    let path = format!("{}/dev/sensors", rootfs);

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
        Ok(()) => info!("[KR64][sensor] removed stale socket: {}", path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warning!("[KR64][sensor] could not remove {}: {}", path, e),
    }

    // Bind. This creates the socket file as a side effect.
    let listener = UnixListener::bind(&path)?;

    // chmod 0666 so the guest (which may run as a different uid
    // inside the chroot) can connect.
    #[cfg(unix)]
    {
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o666));
    }

    info!(
        "[KR64][sensor] bound unix socket: {} (fd={})",
        path,
        listener.as_raw_fd()
    );

    Ok(SensorDevice {
        listener: Some(listener),
        path,
        shutdown: Arc::new(AtomicBool::new(false)),
    })
}

// ============================================================================
// SensorDevice — owns the listener, spawns accept + pump threads
// ============================================================================

/// A bound `/dev/sensors` Unix socket, ready to accept guest connections.
///
/// Created by [`create_sensor_device`]. Call [`SensorDevice::spawn`]
/// to start the accept thread + worker pool (consuming `self`); the
/// returned [`SensorDeviceHandle`] owns the running threads and will
/// shut them down on drop.
///
/// If `spawn` is not called, dropping the `SensorDevice` closes the
/// listener and unlinks the socket file.
pub struct SensorDevice {
    /// The listener itself. `Option<UnixListener>` so `spawn` can
    /// take it out (moving it into the accept thread) without
    /// disturbing the `path` field (needed for the `Drop` impl that
    /// unlinks the socket file).
    listener: Option<UnixListener>,
    /// The filesystem path the socket is bound to.
    path: String,
    /// Shutdown flag shared with the accept thread + the handle.
    shutdown: Arc<AtomicBool>,
}

impl SensorDevice {
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
    /// Returns a [`SensorDeviceHandle`] that holds the shutdown flag
    /// and the accept-thread `JoinHandle`. When the handle is dropped,
    /// the shutdown flag is set and the accept thread is joined.
    pub fn spawn(mut self) -> std::io::Result<SensorDeviceHandle> {
        let listener = self
            .listener
            .take()
            .expect("SensorDevice::spawn: listener already taken");

        // Make the listening socket non-blocking so the accept thread
        // can poll the shutdown flag between accept attempts (mirrors
        // audio.rs / binder.rs).
        let fd = listener.as_raw_fd();
        let _ = unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };

        let shutdown_for_thread = Arc::clone(&self.shutdown);
        let shutdown_for_handle = Arc::clone(&self.shutdown);
        let path = self.path.clone();

        let accept_thread = thread::Builder::new()
            .name("kr64-sensor-accept".to_string())
            .spawn(move || {
                // The pool lives inside the accept thread so its Drop
                // (which joins workers) runs when the accept thread
                // exits. Each worker, in turn, joins its own pump
                // sub-thread before returning, so the whole tree of
                // threads collapses cleanly on shutdown.
                let pool = ThreadPool::new(SENSOR_THREAD_POOL_SIZE);
                info!(
                    "[KR64][sensor] accept loop started (pool_size={})",
                    SENSOR_THREAD_POOL_SIZE
                );

                while !shutdown_for_thread.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _addr)) => {
                            info!("[KR64][sensor] client connected");
                            pool.execute(move || {
                                if let Err(e) = handle_connection(stream) {
                                    warning!("[KR64][sensor] connection handler ended: {}", e);
                                }
                            });
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(std::time::Duration::from_millis(25));
                        }
                        Err(e) => {
                            warning!("[KR64][sensor] accept error: {}", e);
                            std::thread::sleep(std::time::Duration::from_millis(50));
                        }
                    }
                }
                info!("[KR64][sensor] accept loop exiting");
                // pool drops here → workers receive Terminate and join.
                // Each worker that's mid-connection will have its
                // read_exact() return EOF (the guest is gone) or
                // continue normally until the guest disconnects.
            })?;

        Ok(SensorDeviceHandle {
            shutdown: shutdown_for_handle,
            accept_thread: Some(accept_thread),
            path,
        })
    }
}

impl Drop for SensorDevice {
    fn drop(&mut self) {
        // If the user dropped without calling spawn(), we still own
        // the listener — close it and unlink the socket file. If
        // spawn() was called, the listener was moved into the accept
        // thread and self.listener is None — in that case the
        // SensorDeviceHandle owns the unlink responsibility.
        if self.listener.take().is_some() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Handle to a running sensor device. Dropping this sets the shutdown
/// flag and joins the accept thread.
///
/// Created by [`SensorDevice::spawn`]. The accept thread + worker
/// pool keep running until either the handle is dropped or
/// [`SensorDeviceHandle::shutdown`] is called.
pub struct SensorDeviceHandle {
    shutdown: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
    path: String,
}

impl SensorDeviceHandle {
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

impl Drop for SensorDeviceHandle {
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

/// Handle one guest connection: spawn the pump sub-thread, run the
/// control loop in this worker thread, then join the pump on exit.
///
/// The connection stays open until the guest disconnects (control
/// read returns EOF), at which point we signal the pump sub-thread
/// to exit and join it before returning.
fn handle_connection(stream: UnixStream) -> std::io::Result<()> {
    // 1. Clone the stream so the pump sub-thread can write events
    //    while the control worker reads requests. Both clones refer
    //    to the same underlying socket — writes from either are
    //    visible to the guest in order, but if both write concurrently
    //    the bytes can interleave. For the skeleton this is a
    //    non-issue (the pump's JNI stub returns None, so no events
    //    are written). For the real impl, wrap writes in a mutex.
    let mut ev_stream = stream.try_clone()?;
    let conn = Arc::new(SensorConnState::new());
    let conn_for_pump = Arc::clone(&conn);

    // 2. Spawn the pump sub-thread.
    let pump_thread = thread::Builder::new()
        .name("kr64-sensor-pump".to_string())
        .spawn(move || {
            pump_events(&mut ev_stream, conn_for_pump);
        })?;

    // 3. Run the control loop in this worker thread. Returns when
    //    the guest disconnects (EOF) or a write fails.
    let mut ctl_stream = stream;
    let result = handle_control(&mut ctl_stream, &conn);

    // 4. Signal the pump to exit + join it. The pump might be mid-
    //    sleep (up to MAX_POLL_NS = 1 s); joining blocks until it
    //    wakes up and notices the shutdown flag.
    conn.signal_shutdown();
    let _ = pump_thread.join();

    if let Err(e) = result {
        warning!("[KR64][sensor] control loop ended: {}", e);
    }
    info!("[KR64][sensor] connection closed (pump joined)");
    Ok(())
}

/// Read control requests from the guest and dispatch them. Runs in
/// the worker thread; returns when the guest closes the socket (EOF),
/// a read fails, or a CHECK_SUPPORT reply write fails.
fn handle_control(stream: &mut UnixStream, conn: &SensorConnState) -> std::io::Result<()> {
    let mut buf = [0u8; SENSOR_CONTROL_SIZE];
    loop {
        if conn.is_shutdown() {
            break;
        }
        // EOF or read error — guest disconnected. This is the
        // normal exit path.
        read_exact(stream, &mut buf)?;

        let cmd = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let idx = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        let arg = u32::from_le_bytes(buf[8..12].try_into().unwrap());

        if (idx as usize) >= NUM_SENSORS {
            warning!(
                "[KR64][sensor] bad idx={} (cmd={}) — out of range 0..{}",
                idx,
                cmd,
                NUM_SENSORS
            );
            continue;
        }
        let idxu = idx as usize;

        match cmd {
            CTL_CHECK_SUPPORT => {
                // JNI up-call: HALManager.CheckSensorsSupport(idx).
                // Cached in conn state — first call for an idx
                // populates the cache, subsequent calls return it.
                let supported = conn.check_support(idxu);
                let reply: u32 = if supported { 1 } else { 0 };
                if let Err(e) = stream.write_all(&reply.to_le_bytes()) {
                    warning!("[KR64][sensor] check_support reply write failed: {}", e);
                    return Err(e);
                }
                info!("[KR64][sensor] CHECK_SUPPORT idx={} → {}", idxu, supported);
            }
            CTL_ENABLE => {
                // JNI up-call: HALManager.EnableSensors(idx). Returns
                // true if the host sensor was actually registered.
                let ok = jni_enable_sensor(std::ptr::null_mut(), idx);
                if ok {
                    conn.enable(idxu);
                }
                info!(
                    "[KR64][sensor] ENABLE idx={} ok={} (stub=false in skeleton)",
                    idxu, ok
                );
            }
            CTL_DISABLE => {
                // JNI up-call: HALManager.DisableSensors(idx).
                jni_disable_sensor(std::ptr::null_mut(), idx);
                conn.disable(idxu);
                info!("[KR64][sensor] DISABLE idx={}", idxu);
            }
            CTL_SET_DELAY => {
                // JNI up-call: HALManager.SetDelay(idx, delayNs).
                let delay_ns = arg as u64;
                jni_set_sensor_delay(std::ptr::null_mut(), idx, delay_ns);
                conn.set_delay(idxu, delay_ns);
                info!(
                    "[KR64][sensor] SET_DELAY idx={} delay_ns={}",
                    idxu, delay_ns
                );
            }
            _ => {
                warning!(
                    "[KR64][sensor] unknown ctl cmd={} idx={} arg={}",
                    cmd,
                    idxu,
                    arg
                );
            }
        }
    }
    Ok(())
}

/// Pump sensor events to the guest. Runs in a sub-thread spawned by
/// [`handle_connection`]. Returns when the connection's shutdown flag
/// is set (by the control worker on guest disconnect) or a write to
/// the socket fails.
///
/// Loop:
/// 1. Snapshot the enabled-sensor list from the shared state.
/// 2. If empty, sleep for [`SENSOR_IDLE_POLL_MS`] and continue.
/// 3. For each enabled sensor, call [`jni_read_sensor_event`]. If it
///    returns `Some(ev)`, serialise the 24-byte event and write it
///    to the socket.
/// 4. Sleep for the shortest delay among enabled sensors (clamped to
///    [`MIN_POLL_NS`]..[`MAX_POLL_NS`]) and repeat.
///
/// In the skeleton the JNI stub always returns `None`, so no events
/// are written and the loop just sleeps — but the structure is
/// correct for the real impl.
fn pump_events(stream: &mut UnixStream, conn: Arc<SensorConnState>) {
    info!("[KR64][sensor][pump] started");
    while !conn.is_shutdown() {
        let snapshot = conn.snapshot();
        if snapshot.is_empty() {
            // No sensors enabled — sleep idle and re-check.
            thread::sleep(Duration::from_millis(SENSOR_IDLE_POLL_MS));
            continue;
        }

        // Poll each enabled sensor + track the shortest delay for
        // the next iteration's sleep.
        let mut min_delay_ns = u64::MAX;
        for (idx, delay_ns) in &snapshot {
            if let Some(ev) = jni_read_sensor_event(std::ptr::null_mut(), *idx) {
                let bytes = ev.to_bytes();
                if let Err(e) = stream.write_all(&bytes) {
                    warning!("[KR64][sensor][pump] write error: {}", e);
                    return;
                }
            }
            min_delay_ns = min_delay_ns.min(*delay_ns);
        }

        // Sleep until the next sample is due. Clamp to a sane range
        // — a SET_DELAY 0 request shouldn't cause a busy-loop, and a
        // SET_DELAY u32::MAX request shouldn't make us unresponsive.
        let sleep_ns = min_delay_ns.clamp(MIN_POLL_NS, MAX_POLL_NS);
        thread::sleep(Duration::from_nanos(sleep_ns));
    }
    info!("[KR64][sensor][pump] exiting");
}

// ============================================================================
// JNI up-call stubs.
//
// These mirror VM's `HALManager.CheckSensorsSupport` / `EnableSensors` /
// `DisableSensors` / `SetDelay` private methods and the
// `nativeSensorChanged` down-call (see AUDIO_SENSOR_HAL.md §2.4–§2.5).
// Each one, in the real implementation, would:
//   1. Attach the current thread to the JVM (cached in a thread-local
//      so the first call on each worker thread pays the attach cost
//      and subsequent calls are free).
//   2. Find the HALManager class + method by signature.
//   3. Call the method, marshal args/returns.
//   4. Return the Java bool / void / SensorEvent.
//
// For the skeleton these are no-ops returning false/None/() — they
// let the control + pump loops compile and exercise the protocol,
// but no real sensor data flows. The real implementation will
// replace these five functions (likely behind a trait object so the
// Java side can be wired in without touching the pump code).
// ============================================================================

/// Opaque handle to the Java `HALManager` instance. In the real
/// implementation this would be a `jni::sys::jobject` (global ref).
/// For the skeleton it's a `*mut c_void` so we can pass it around
/// without pulling in the `jni` crate. The stubs never actually use
/// it, so it's always null in the skeleton.
pub type JniObject = *mut std::ffi::c_void;

/// `HALManager.CheckSensorsSupport(int idx)` → returns true if the
/// host has sensor `idx`. Stubbed: returns `false` — no JNI in
/// skeleton, so the guest sees "no sensors available".
///
/// Real implementation: attach current thread to JVM, find HALManager
/// class, call `CheckSensorsSupport(I)Z`, return the jboolean as bool.
fn jni_check_sensor_support(_ptr: JniObject, _idx: u32) -> bool {
    // Skeleton: always returns false. This means CHECK_SUPPORT replies
    // 0 to every query, which the guest's sensor HAL should treat as
    // "this sensor doesn't exist" — `SensorManager.getDefaultSensor()`
    // returns null, and the guest falls back to no-sensor mode.
    false
}

/// `HALManager.EnableSensors(int idx)` → returns true if the host
/// `SensorManager.registerListener` succeeded. Stubbed: returns
/// `false` — no JNI in skeleton, so ENABLE is a no-op and the
/// per-connection state's `enabled[]` array stays all-false.
fn jni_enable_sensor(_ptr: JniObject, _idx: u32) -> bool {
    false
}

/// `HALManager.DisableSensors(int idx)` → unregisters the host
/// listener. Stubbed: no-op.
fn jni_disable_sensor(_ptr: JniObject, _idx: u32) {}

/// `HALManager.SetDelay(int idx, int delayNs)` → sets the sampling
/// period. Stubbed: no-op.
///
/// Note: VM's `SetDelay` has a quirk where it zeroes both the
/// sampling period AND the max-latency arrays, then uses `1` (us) if
/// `0` (see AUDIO_SENSOR_HAL.md §2.5). Twoyi treats this as a normal
/// set-with-floor — the pump clamps to [`MIN_POLL_NS`] = 1 ms.
fn jni_set_sensor_delay(_ptr: JniObject, _idx: u32, _delay_ns: u64) {}

/// Read the next pending `SensorEvent` for sensor `idx`. In the real
/// implementation this drains a per-idx `mpsc::Receiver` filled by
/// the `nativeSensorChanged` JNI down-call (Java → Rust). Stubbed:
/// returns `None` — no events are ever produced.
///
/// Real implementation: the `nativeSensorChanged` JNI entry point
/// would be a `#[no_mangle] pub extern "system" fn` that pushes
/// a `SensorEvent` into a per-idx `mpsc::Sender` stored in a global
/// registry. This function would `recv_timeout(0)` from the matching
/// receiver and return `Some(ev)` if a value was available, `None`
/// otherwise.
fn jni_read_sensor_event(_ptr: JniObject, _idx: u32) -> Option<SensorEvent> {
    None
}

// ============================================================================
// Minimal thread pool — fixed-size, MPMC via std::sync::mpsc.
//
// Same pattern as `audio.rs::ThreadPool` and `binder.rs::ThreadPool`.
// Kept private to each module so they're self-contained (a future
// refactor could lift this to a shared `thread_pool.rs`).
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
            .name("kr64-sensor-worker".to_string())
            .spawn(move || loop {
                let msg = receiver.lock().unwrap().recv();
                match msg {
                    Ok(Message::Job(job)) => job(),
                    Ok(Message::Terminate) | Err(_) => break,
                }
            })
            .expect("spawn kr64 sensor worker");
        Worker {
            thread: Some(thread),
        }
    }
}

/// A fixed-size thread pool. Used by [`SensorDevice`] to handle
/// multiple concurrent guest connections.
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
                warning!("[KR64][sensor] thread pool: sender closed, job dropped");
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
    /// Mirrors the pattern in audio.rs / binder.rs test modules.
    fn tmpdir() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut p = env::temp_dir();
        p.push(format!("kr64-sensor-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&p).unwrap();
        p.to_string_lossy().to_string()
    }

    // -------- SensorEvent struct layout --------------------------------

    #[test]
    fn sensor_event_size_is_24_bytes() {
        // Compile-time assertion exists at the top of the file
        // (`const _: () = assert!(size_of::<SensorEvent>() == 24);`),
        // but assert again at runtime for clarity in test output.
        assert_eq!(std::mem::size_of::<SensorEvent>(), SENSOR_EVENT_SIZE);
    }

    // -------- SensorEvent serialization roundtrips ---------------------

    #[test]
    fn sensor_event_roundtrip_basic() {
        let ev = SensorEvent::new(0, 1_000_000, 1.0, 2.0, 3.0);
        let bytes = ev.to_bytes();
        assert_eq!(bytes.len(), SENSOR_EVENT_SIZE);

        let ev2 = SensorEvent::from_bytes(&bytes).expect("from_bytes");
        assert_eq!(ev, ev2);
    }

    #[test]
    fn sensor_event_roundtrip_max_idx() {
        // idx 11 is the last valid sensor (GYRO_UNCAL).
        let ev = SensorEvent::new(11, u64::MAX, f32::INFINITY, f32::NEG_INFINITY, 0.0);
        let bytes = ev.to_bytes();
        let ev2 = SensorEvent::from_bytes(&bytes).expect("from_bytes");
        assert_eq!(ev, ev2);
    }

    #[test]
    fn sensor_event_roundtrip_zero() {
        let ev = SensorEvent::zero();
        let bytes = ev.to_bytes();
        let ev2 = SensorEvent::from_bytes(&bytes).expect("from_bytes");
        assert_eq!(ev, ev2);
    }

    #[test]
    fn sensor_event_roundtrip_negative_floats() {
        // Accel values can be negative (e.g. -9.81 for gravity).
        let ev = SensorEvent::new(0, 123_456, -9.81, 0.0, 0.0);
        let bytes = ev.to_bytes();
        let ev2 = SensorEvent::from_bytes(&bytes).expect("from_bytes");
        assert_eq!(ev, ev2);
    }

    #[test]
    fn sensor_event_from_bytes_ignores_trailing_bytes() {
        // The caller may pass a buffer that's longer than 24 bytes
        // (e.g. multiple events concatenated). Only the first 24
        // bytes should be consumed.
        let ev = SensorEvent::new(3, 999, 1.5, 2.5, 3.5);
        let mut buf = ev.to_bytes().to_vec();
        buf.extend_from_slice(&[0xAA; 32]); // fake next event
        let ev2 = SensorEvent::from_bytes(&buf).expect("from_bytes");
        assert_eq!(ev, ev2);
    }

    // -------- SensorEvent validation -----------------------------------

    #[test]
    fn sensor_event_from_bytes_rejects_short_buffer() {
        let short = [0u8; 16];
        let r = SensorEvent::from_bytes(&short);
        assert_eq!(r, Err(SensorEventError::TooShort { got: 16 }));
    }

    #[test]
    fn sensor_event_from_bytes_rejects_bad_idx() {
        let mut ev = SensorEvent::new(0, 0, 0.0, 0.0, 0.0);
        ev.idx = NUM_SENSORS as u32; // 12 — out of range
        let bytes = ev.to_bytes();
        let r = SensorEvent::from_bytes(&bytes);
        assert_eq!(r, Err(SensorEventError::BadIndex { got: 12 }));
    }

    #[test]
    fn sensor_event_from_bytes_rejects_huge_idx() {
        let mut ev = SensorEvent::new(0, 0, 0.0, 0.0, 0.0);
        ev.idx = u32::MAX;
        let bytes = ev.to_bytes();
        let r = SensorEvent::from_bytes(&bytes);
        assert_eq!(r, Err(SensorEventError::BadIndex { got: u32::MAX }));
    }

    #[test]
    fn sensor_event_is_valid_checks_idx_range() {
        let mut ev = SensorEvent::new(0, 0, 0.0, 0.0, 0.0);
        assert!(ev.is_valid());

        ev.idx = 11;
        assert!(ev.is_valid());

        ev.idx = 12;
        assert!(!ev.is_valid());

        ev.idx = u32::MAX;
        assert!(!ev.is_valid());
    }

    #[test]
    fn sensor_event_error_display_is_informative() {
        let e = SensorEventError::TooShort { got: 4 };
        assert!(e.to_string().contains("4"));
        assert!(e.to_string().contains("24"));

        let e = SensorEventError::BadIndex { got: 99 };
        assert!(e.to_string().contains("99"));
        assert!(e.to_string().contains("0..12"));
    }

    // -------- SensorType enum + mapping --------------------------------

    #[test]
    fn sensor_type_repr_matches_android_constants() {
        // Values must match android.hardware.Sensor.TYPE_* exactly.
        assert_eq!(SensorType::Accelerometer as u32, 1);
        assert_eq!(SensorType::MagneticField as u32, 2);
        assert_eq!(SensorType::Orientation as u32, 3);
        assert_eq!(SensorType::GyroscopeUncalibrated as u32, 4);
        assert_eq!(SensorType::Proximity as u32, 5);
        assert_eq!(SensorType::Gyroscope as u32, 6);
        assert_eq!(SensorType::Temperature as u32, 7);
        assert_eq!(SensorType::Light as u32, 8);
        assert_eq!(SensorType::Pressure as u32, 9);
        assert_eq!(SensorType::RelativeHumidity as u32, 12);
        assert_eq!(SensorType::StepDetector as u32, 18);
        assert_eq!(SensorType::Gravity as u32, 19);
    }

    #[test]
    fn sensor_type_from_type_value_roundtrip() {
        for v in [1u32, 2, 3, 4, 5, 6, 7, 8, 9, 12, 18, 19] {
            let t = SensorType::from_type_value(v).expect("known type");
            assert_eq!(t.type_value(), v);
        }
    }

    #[test]
    fn sensor_type_from_type_value_rejects_unknown() {
        assert!(SensorType::from_type_value(0).is_none());
        assert!(SensorType::from_type_value(10).is_none());
        assert!(SensorType::from_type_value(11).is_none());
        assert!(SensorType::from_type_value(13).is_none());
        assert!(SensorType::from_type_value(17).is_none());
        assert!(SensorType::from_type_value(20).is_none());
        assert!(SensorType::from_type_value(u32::MAX).is_none());
    }

    #[test]
    fn sensor_type_map_has_twelve_entries() {
        assert_eq!(SENSOR_TYPE_MAP.len(), NUM_SENSORS);
    }

    #[test]
    fn index_to_type_matches_vm_mapping() {
        // Verbatim from SensorService.java's static {} block
        // (see AUDIO_SENSOR_HAL.md §2.1).
        assert_eq!(index_to_type(0), Some(SensorType::Accelerometer));
        assert_eq!(index_to_type(1), Some(SensorType::MagneticField));
        assert_eq!(index_to_type(2), Some(SensorType::Orientation));
        assert_eq!(index_to_type(3), Some(SensorType::Temperature));
        assert_eq!(index_to_type(4), Some(SensorType::Light));
        assert_eq!(index_to_type(5), Some(SensorType::Proximity));
        assert_eq!(index_to_type(6), Some(SensorType::Gyroscope));
        assert_eq!(index_to_type(7), Some(SensorType::RelativeHumidity));
        assert_eq!(index_to_type(8), Some(SensorType::Pressure));
        assert_eq!(index_to_type(9), Some(SensorType::Gravity));
        assert_eq!(index_to_type(10), Some(SensorType::StepDetector));
        assert_eq!(index_to_type(11), Some(SensorType::GyroscopeUncalibrated));
    }

    #[test]
    fn index_to_type_rejects_out_of_range() {
        assert_eq!(index_to_type(12), None);
        assert_eq!(index_to_type(100), None);
        assert_eq!(index_to_type(u32::MAX), None);
    }

    #[test]
    fn type_to_index_is_inverse_of_index_to_type() {
        for i in 0..NUM_SENSORS as u32 {
            let t = index_to_type(i).expect("in-range idx");
            assert_eq!(type_to_index(t), Some(i));
        }
    }

    #[test]
    fn type_to_index_returns_none_for_non_virtualised_types() {
        // These Sensor.TYPE_* values aren't in the 12 VM virtualises.
        // We can only construct SensorType variants that exist, so
        // we test via from_type_value first.
        // All 12 variants are virtualised, so this is more of a sanity
        // check that the reverse-map covers all variants.
        for t in [
            SensorType::Accelerometer,
            SensorType::MagneticField,
            SensorType::Orientation,
            SensorType::GyroscopeUncalibrated,
            SensorType::Proximity,
            SensorType::Gyroscope,
            SensorType::Temperature,
            SensorType::Light,
            SensorType::Pressure,
            SensorType::RelativeHumidity,
            SensorType::StepDetector,
            SensorType::Gravity,
        ] {
            assert!(type_to_index(t).is_some(), "no index for {:?}", t);
        }
    }

    // -------- SensorState bitflags -------------------------------------

    #[test]
    fn sensor_state_bit_values_match_vm() {
        // Verbatim from HALManager.java (see AUDIO_SENSOR_HAL.md §2.5).
        assert_eq!(SensorState::SUPPORTED.bits(), 0b001);
        assert_eq!(SensorState::ENABLED.bits(), 0b010);
        assert_eq!(SensorState::ACTIVE.bits(), 0b100);
    }

    #[test]
    fn sensor_state_empty_is_zero() {
        let s = SensorState::empty();
        assert!(s.is_empty());
        assert_eq!(s.bits(), 0);
        assert!(!s.is_supported());
        assert!(!s.is_enabled());
        assert!(!s.is_active());
    }

    #[test]
    fn sensor_state_from_bits_masks_to_low_three() {
        // High bits should be stripped.
        let s = SensorState::from_bits(0b11111001);
        assert_eq!(s.bits(), 0b001);
        assert!(s.is_supported());
        assert!(!s.is_enabled());
        assert!(!s.is_active());
    }

    #[test]
    fn sensor_state_contains() {
        let s = SensorState::SUPPORTED | SensorState::ENABLED;
        assert!(s.contains(SensorState::SUPPORTED));
        assert!(s.contains(SensorState::ENABLED));
        assert!(!s.contains(SensorState::ACTIVE));
    }

    #[test]
    fn sensor_state_insert_and_remove() {
        let mut s = SensorState::empty();
        s.insert(SensorState::SUPPORTED);
        assert!(s.is_supported());
        assert!(!s.is_enabled());

        s.insert(SensorState::ENABLED);
        assert!(s.is_supported());
        assert!(s.is_enabled());
        assert!(!s.is_active());

        s.insert(SensorState::ACTIVE);
        assert_eq!(s.bits(), 0b111);

        s.remove(SensorState::ENABLED);
        assert!(s.is_supported());
        assert!(!s.is_enabled());
        assert!(s.is_active());
        assert_eq!(s.bits(), 0b101);
    }

    #[test]
    fn sensor_state_bitor_assign() {
        let mut s = SensorState::SUPPORTED;
        s |= SensorState::ENABLED;
        s |= SensorState::ACTIVE;
        assert_eq!(s.bits(), 0b111);
    }

    #[test]
    fn sensor_state_bitor_combines_bits() {
        let s = SensorState::SUPPORTED | SensorState::ACTIVE;
        assert_eq!(s.bits(), 0b101);
        assert!(s.is_supported());
        assert!(!s.is_enabled());
        assert!(s.is_active());
    }

    #[test]
    fn sensor_state_bitand() {
        let s = SensorState::ALL & SensorState::SUPPORTED;
        assert_eq!(s.bits(), 0b001);
    }

    #[test]
    fn sensor_state_not() {
        let s = !SensorState::SUPPORTED;
        // Only the low 3 bits matter; bit 0 is cleared, bits 1+2 set.
        assert_eq!(s.bits(), 0b110);
        assert!(!s.is_supported());
        assert!(s.is_enabled());
        assert!(s.is_active());
    }

    #[test]
    fn sensor_state_all_constant() {
        assert_eq!(SensorState::ALL.bits(), 0b111);
        assert!(SensorState::ALL.is_supported());
        assert!(SensorState::ALL.is_enabled());
        assert!(SensorState::ALL.is_active());
    }

    #[test]
    fn sensor_state_default_is_empty() {
        let s: SensorState = Default::default();
        assert!(s.is_empty());
    }

    // -------- SensorControl struct -------------------------------------

    #[test]
    fn sensor_control_size_is_12_bytes() {
        assert_eq!(std::mem::size_of::<SensorControl>(), SENSOR_CONTROL_SIZE);
    }

    #[test]
    fn sensor_control_roundtrip() {
        let c = SensorControl::new(CTL_ENABLE, 6, 1_000_000);
        let bytes = c.to_bytes();
        assert_eq!(bytes.len(), SENSOR_CONTROL_SIZE);
        let c2 = SensorControl::from_bytes(&bytes).expect("from_bytes");
        assert_eq!(c, c2);
    }

    #[test]
    fn sensor_control_from_bytes_rejects_short_buffer() {
        let short = [0u8; 8];
        let r = SensorControl::from_bytes(&short);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().got, 8);
    }

    #[test]
    fn sensor_control_from_bytes_ignores_trailing() {
        let c = SensorControl::new(CTL_CHECK_SUPPORT, 0, 0);
        let mut buf = c.to_bytes().to_vec();
        buf.extend_from_slice(&[0xFF; 16]);
        let c2 = SensorControl::from_bytes(&buf).expect("from_bytes");
        assert_eq!(c, c2);
    }

    // -------- create_sensor_device -------------------------------------

    #[test]
    fn create_sensor_device_creates_socket() {
        let rootfs = tmpdir();
        let dev = create_sensor_device(&rootfs).expect("create_sensor_device");

        assert!(Path::new(&dev.path).exists(), "socket file should exist");
        assert!(dev.path.ends_with("/dev/sensors"));
        assert!(dev.raw_fd() >= 0);

        // drop should unlink the socket file.
        let path = dev.path.clone();
        drop(dev);
        assert!(
            !Path::new(&path).exists(),
            "socket file should be unlinked on drop"
        );

        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn create_sensor_device_creates_dev_dir_if_missing() {
        let rootfs = tmpdir();
        // {rootfs}/dev doesn't exist yet — create_sensor_device must
        // make it.
        assert!(!Path::new(&format!("{}/dev", rootfs)).exists());
        let dev = create_sensor_device(&rootfs).expect("create_sensor_device");
        assert!(Path::new(&format!("{}/dev", rootfs)).exists());
        assert!(Path::new(&dev.path).exists());
        drop(dev);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn create_sensor_device_replaces_stale_socket() {
        let rootfs = tmpdir();
        // First bind.
        let dev1 = create_sensor_device(&rootfs).expect("first create");
        let path = dev1.path.clone();
        drop(dev1);
        // path is now unlinked by Drop. But simulate a stale socket
        // by creating a regular file at the same path.
        fs::write(&path, b"stale").unwrap();
        assert!(Path::new(&path).exists());

        // Second bind should succeed (the stale file is removed first).
        let dev2 = create_sensor_device(&rootfs).expect("second create over stale");
        assert!(Path::new(&path).exists());
        drop(dev2);
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- SensorDevice::spawn + connection handling ---------------

    /// Helper: read exactly N bytes from a UnixStream in a test.
    fn read_exact_test(s: &mut UnixStream, buf: &mut [u8]) -> std::io::Result<()> {
        use std::io::Read;
        s.read_exact(buf)
    }

    #[test]
    fn sensor_device_spawn_accepts_connection_and_handles_check_support() {
        let rootfs = tmpdir();
        let dev = create_sensor_device(&rootfs).expect("create");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");

        // Give the accept thread a moment to start.
        std::thread::sleep(Duration::from_millis(50));

        let mut stream = UnixStream::connect(&path).expect("connect");

        // Send CHECK_SUPPORT for idx 0 (accel).
        let ctl = SensorControl::new(CTL_CHECK_SUPPORT, 0, 0);
        stream.write_all(&ctl.to_bytes()).expect("write ctl");

        // Read 4-byte reply. The stubbed jni_check_sensor_support
        // returns false, so the reply should be 0.
        let mut reply = [0u8; 4];
        read_exact_test(&mut stream, &mut reply).expect("read reply");
        let supported = u32::from_le_bytes(reply);
        assert_eq!(supported, 0, "stubbed CHECK_SUPPORT should reply 0");

        // Try another idx to make sure the control loop continues.
        let ctl2 = SensorControl::new(CTL_CHECK_SUPPORT, 6, 0);
        stream.write_all(&ctl2.to_bytes()).expect("write ctl 2");
        let mut reply2 = [0u8; 4];
        read_exact_test(&mut stream, &mut reply2).expect("read reply 2");
        assert_eq!(u32::from_le_bytes(reply2), 0);

        drop(stream);
        drop(handle);
        assert!(!Path::new(&path).exists(), "socket unlinked on handle drop");
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn sensor_device_spawn_handles_enable_disable_set_delay() {
        let rootfs = tmpdir();
        let dev = create_sensor_device(&rootfs).expect("create");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");

        std::thread::sleep(Duration::from_millis(50));

        let mut stream = UnixStream::connect(&path).expect("connect");

        // ENABLE idx 0 — stub returns false, but the control loop
        // should still acknowledge and continue.
        stream
            .write_all(&SensorControl::new(CTL_ENABLE, 0, 0).to_bytes())
            .expect("write enable");

        // SET_DELAY idx 0, 60 Hz = ~16.67 ms = 16_666_667 ns.
        stream
            .write_all(&SensorControl::new(CTL_SET_DELAY, 0, 16_666_667).to_bytes())
            .expect("write set_delay");

        // DISABLE idx 0.
        stream
            .write_all(&SensorControl::new(CTL_DISABLE, 0, 0).to_bytes())
            .expect("write disable");

        // Give the worker a moment to process all three.
        std::thread::sleep(Duration::from_millis(100));

        // The connection should still be alive — verify by sending
        // a CHECK_SUPPORT and reading a reply.
        stream
            .write_all(&SensorControl::new(CTL_CHECK_SUPPORT, 1, 0).to_bytes())
            .expect("write check_support");
        let mut reply = [0u8; 4];
        read_exact_test(&mut stream, &mut reply).expect("read reply");
        assert_eq!(u32::from_le_bytes(reply), 0);

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn sensor_device_spawn_handles_unknown_command() {
        let rootfs = tmpdir();
        let dev = create_sensor_device(&rootfs).expect("create");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");

        std::thread::sleep(Duration::from_millis(50));

        let mut stream = UnixStream::connect(&path).expect("connect");

        // Send a control message with an unknown cmd. The worker
        // should log a warning and continue.
        stream
            .write_all(&SensorControl::new(99, 0, 0).to_bytes())
            .expect("write unknown");

        std::thread::sleep(Duration::from_millis(50));

        // Verify the connection is still alive.
        stream
            .write_all(&SensorControl::new(CTL_CHECK_SUPPORT, 0, 0).to_bytes())
            .expect("write check_support");
        let mut reply = [0u8; 4];
        read_exact_test(&mut stream, &mut reply).expect("read reply");
        assert_eq!(u32::from_le_bytes(reply), 0);

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn sensor_device_spawn_handles_bad_idx() {
        let rootfs = tmpdir();
        let dev = create_sensor_device(&rootfs).expect("create");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");

        std::thread::sleep(Duration::from_millis(50));

        let mut stream = UnixStream::connect(&path).expect("connect");

        // CHECK_SUPPORT with an out-of-range idx. The worker should
        // log a warning and continue. No reply is sent for bad idx
        // (we `continue` before the reply write).
        stream
            .write_all(&SensorControl::new(CTL_CHECK_SUPPORT, 99, 0).to_bytes())
            .expect("write bad idx");

        std::thread::sleep(Duration::from_millis(50));

        // Verify the connection is still alive with a valid request.
        stream
            .write_all(&SensorControl::new(CTL_CHECK_SUPPORT, 0, 0).to_bytes())
            .expect("write valid");
        let mut reply = [0u8; 4];
        read_exact_test(&mut stream, &mut reply).expect("read reply");
        assert_eq!(u32::from_le_bytes(reply), 0);

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn sensor_device_spawn_handles_multiple_connections() {
        // The guest's sensorservice holds one connection, but the
        // pool should handle reconnects (e.g. after a guest reboot)
        // without leaking threads.
        let rootfs = tmpdir();
        let dev = create_sensor_device(&rootfs).expect("create");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");

        std::thread::sleep(Duration::from_millis(50));

        for _ in 0..3 {
            let mut stream = UnixStream::connect(&path).expect("connect");
            stream
                .write_all(&SensorControl::new(CTL_CHECK_SUPPORT, 0, 0).to_bytes())
                .expect("write");
            let mut reply = [0u8; 4];
            read_exact_test(&mut stream, &mut reply).expect("read");
            assert_eq!(u32::from_le_bytes(reply), 0);
            drop(stream);
            std::thread::sleep(Duration::from_millis(25));
        }

        drop(handle);
        assert!(!Path::new(&path).exists());
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn sensor_device_spawn_pump_runs_without_crashing() {
        // Even with no sensors enabled, the pump sub-thread should
        // run its idle loop without crashing. We verify by opening a
        // connection, sleeping long enough for the pump to iterate
        // several times, then closing.
        let rootfs = tmpdir();
        let dev = create_sensor_device(&rootfs).expect("create");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");

        std::thread::sleep(Duration::from_millis(50));

        let mut stream = UnixStream::connect(&path).expect("connect");

        // Sleep long enough for the pump to run a few idle iterations
        // (SENSOR_IDLE_POLL_MS = 50, so 200 ms = 4 iterations).
        std::thread::sleep(Duration::from_millis(200));

        // The connection should still be responsive.
        stream
            .write_all(&SensorControl::new(CTL_CHECK_SUPPORT, 0, 0).to_bytes())
            .expect("write");
        let mut reply = [0u8; 4];
        read_exact_test(&mut stream, &mut reply).expect("read");
        assert_eq!(u32::from_le_bytes(reply), 0);

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn sensor_device_spawn_rejects_short_control() {
        // If the guest sends fewer than 12 bytes then closes, the
        // control loop's read_exact should hit UnexpectedEof and
        // return Err — the worker logs a warning and exits cleanly.
        let rootfs = tmpdir();
        let dev = create_sensor_device(&rootfs).expect("create");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");

        std::thread::sleep(Duration::from_millis(50));

        let mut stream = UnixStream::connect(&path).expect("connect");
        stream.write_all(&[1, 2, 3, 4]).expect("write short");
        let _ = stream.shutdown(std::net::Shutdown::Write);

        std::thread::sleep(Duration::from_millis(50));

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn sensor_device_handle_shutdown_joins_thread() {
        let rootfs = tmpdir();
        let dev = create_sensor_device(&rootfs).expect("create");
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
    fn sensor_device_handle_drop_without_shutdown_still_joins() {
        let rootfs = tmpdir();
        let dev = create_sensor_device(&rootfs).expect("create");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");
        std::thread::sleep(Duration::from_millis(50));

        // Don't call shutdown() — just drop. Drop's first action is
        // to set the shutdown flag, then join.
        drop(handle);

        assert!(!Path::new(&path).exists());
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- SensorConnState (per-connection state) -------------------

    #[test]
    fn conn_state_starts_empty() {
        let conn = SensorConnState::new();
        let snap = conn.snapshot();
        assert!(
            snap.is_empty(),
            "fresh state should have no enabled sensors"
        );
        assert!(!conn.is_shutdown());
    }

    #[test]
    fn conn_state_enable_disable_roundtrip() {
        let conn = SensorConnState::new();

        // Initially empty.
        assert!(conn.snapshot().is_empty());

        // Enable idx 0 + idx 6.
        conn.enable(0);
        conn.enable(6);
        let snap = conn.snapshot();
        assert_eq!(snap.len(), 2);
        assert!(snap.contains(&(0, 0)));
        assert!(snap.contains(&(6, 0)));

        // Disable idx 0.
        conn.disable(0);
        let snap = conn.snapshot();
        assert_eq!(snap.len(), 1);
        assert!(snap.contains(&(6, 0)));
    }

    #[test]
    fn conn_state_set_delay_appears_in_snapshot() {
        let conn = SensorConnState::new();
        conn.enable(3);
        conn.set_delay(3, 1_000_000); // 1 ms
        let snap = conn.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0], (3, 1_000_000));
    }

    #[test]
    fn conn_state_check_support_caches_result() {
        // The stubbed jni_check_sensor_support always returns false,
        // so check_support should return false and cache false.
        let conn = SensorConnState::new();
        assert!(!conn.check_support(0));
        assert!(!conn.check_support(0)); // second call uses cache
    }

    #[test]
    fn conn_state_shutdown_flag_roundtrip() {
        let conn = SensorConnState::new();
        assert!(!conn.is_shutdown());
        conn.signal_shutdown();
        assert!(conn.is_shutdown());
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

    // -------- I/O helper -----------------------------------------------

    #[test]
    fn read_exact_returns_eof_on_short_read() {
        let rootfs = tmpdir();
        let dev = create_sensor_device(&rootfs).expect("create");
        let path = dev.path().to_string();
        let handle = dev.spawn().expect("spawn");
        std::thread::sleep(Duration::from_millis(50));

        // Connect and write 4 bytes (less than 12). The handler will
        // call read_exact for the 12-byte control message, which
        // should hit UnexpectedEof when the guest closes.
        let mut stream = UnixStream::connect(&path).expect("connect");
        stream.write_all(&[1, 2, 3, 4]).expect("write short");
        let _ = stream.shutdown(std::net::Shutdown::Write);

        std::thread::sleep(Duration::from_millis(50));

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- JNI stubs (verify they return their documented values) ---

    #[test]
    fn jni_check_sensor_support_stub_returns_false() {
        assert!(!jni_check_sensor_support(std::ptr::null_mut(), 0));
        assert!(!jni_check_sensor_support(std::ptr::null_mut(), 6));
        assert!(!jni_check_sensor_support(std::ptr::null_mut(), 11));
    }

    #[test]
    fn jni_enable_sensor_stub_returns_false() {
        assert!(!jni_enable_sensor(std::ptr::null_mut(), 0));
        assert!(!jni_enable_sensor(std::ptr::null_mut(), 6));
    }

    #[test]
    fn jni_disable_sensor_stub_is_noop() {
        // Just verify it doesn't panic.
        jni_disable_sensor(std::ptr::null_mut(), 0);
        jni_disable_sensor(std::ptr::null_mut(), 11);
    }

    #[test]
    fn jni_set_sensor_delay_stub_is_noop() {
        // Just verify it doesn't panic.
        jni_set_sensor_delay(std::ptr::null_mut(), 0, 1_000_000);
        jni_set_sensor_delay(std::ptr::null_mut(), 6, 0);
    }

    #[test]
    fn jni_read_sensor_event_stub_returns_none() {
        assert!(jni_read_sensor_event(std::ptr::null_mut(), 0).is_none());
        assert!(jni_read_sensor_event(std::ptr::null_mut(), 6).is_none());
    }
}
