// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use libc::*;
use libc::{c_char, c_int};
use ndk::event::{MotionAction, MotionEvent};
use std::io::Write;
use std::thread;
use uinput_sys::*;
// `input_event` is glob-exported by BOTH `libc` (with field `type_`)
// and `uinput_sys` (with field `kind`). The code below uses the `kind`
// field, so the intended resolution is `uinput_sys::input_event`. With
// newer rustc (≥1.74) glob ambiguity is a hard error rather than a
// warning, so we add an explicit import to disambiguate. This matches
// the long-standing intent and does NOT change the resolved type on
// older toolchains — `uinput_sys::input_event` was already the
// resolved type there.
use uinput_sys::input_event;

use once_cell::sync::Lazy;
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;

use log::info;

const FF_MAX: u16 = 0x7f;

const KEY_DEVICE_NAME: &str = "vkey";
const KEY_DEVICE_UNIQUE_ID: &str = "<keyboard 0>";

/// Touch-events IPC socket path — host-side UnixListener that kr64's
/// `spawn_touch_accept_thread` (commit `c67c498`) connects to as a
/// client. Carries 20-byte `TouchMessage` records (see below) from
/// this host's `handle_touch` JNI callback to kr64's per-connection
/// touch worker, which re-encodes them into the guest's `InputEvent`
/// format via `devices::encode_touch_*`.
///
/// This is NOT the guest-facing `/dev/input/touch` socket — kr64 owns
/// that one now (it builds the `DeviceInfo` header and dispatches
/// `InputEvent`s). The host-side `touch_server` here binds the IPC
/// socket only; the only client is kr64.
fn touch_events_path() -> String {
    crate::core::get_touch_events_path()
}

/// Key device socket path — now dynamic via core::get_key_path().
fn key_path() -> String {
    crate::core::get_key_path()
}

// ============================================================================
// TouchMessage — host→kr64 IPC record format.
//
// This is the EXACT format kr64's `spawn_touch_accept_thread` reads
// (commit `c67c498`, `app/rs/kr64/src/lib.rs`). The two crates MUST
// agree byte-for-byte — a size mismatch or field-order drift would
// silently misparse the IPC stream.
//
// Layout (little-endian, no padding — 20 bytes total):
//
// ```text
//   offset  size  field
//   ------  ----  -----
//     0      4    action      (u32: 0=DOWN, 1=MOVE, 2=UP, 3=CANCEL)
//     4      4    pointer_id  (i32: slot index 0..MAX_POINTERS-1)
//     8      4    x           (i32: pixel x)
//    12      4    y           (i32: pixel y)
//    16      4    pressure    (i32: 0..255)
// ```
//
// `MAX_POINTERS` is 5 (matches `devices::MAX_POINTERS` in kr64 and
// the size of the `tracking_ids` array the dispatcher maintains).
// ============================================================================

/// Size of one `TouchMessage` record on the host→kr64 IPC socket, in
/// bytes. Kept in sync with kr64's `TOUCH_MESSAGE_SIZE` constant
/// (commit `c67c498`) — a unit test (`touch_message_size_is_20_bytes`)
/// guards against accidental drift.
const TOUCH_MESSAGE_SIZE: usize = 20;

/// `TouchMessage::action` values. These match kr64's `touch_action`
/// module (commit `c67c498`) and correspond to the subset of Android's
/// `MotionAction` that the touch dispatcher cares about.
mod touch_action {
    /// A new finger touched the screen (MotionEvent.ACTION_DOWN /
    /// ACTION_POINTER_DOWN).
    pub const DOWN: u32 = 0;
    /// An existing finger moved (MotionEvent.ACTION_MOVE).
    pub const MOVE: u32 = 1;
    /// The last finger lifted (MotionEvent.ACTION_UP).
    pub const UP: u32 = 2;
    /// A non-last finger lifted or the gesture was cancelled
    /// (MotionEvent.ACTION_POINTER_UP / ACTION_CANCEL).
    pub const CANCEL: u32 = 3;
    /// 6-Z293c: synthetic key press — `x` carries the LINUX keycode.
    /// Only meaningful on the abstract evdev bridge (the loader-path
    /// minui never opens the key0 device, so send_key_code dual-writes
    /// here to reach recovery menus with KEY_VOLUMEUP/KEY_POWER style
    /// navigation — the discriminator between "bridge dead" and
    /// "touch state machine ignores us").
    pub const KEY_DOWN: u32 = 4;
    /// 6-Z293c: synthetic key release — `x` carries the LINUX keycode.
    pub const KEY_UP: u32 = 5;
}

/// A serialisable touch message sent over the host→kr64 IPC socket at
/// `{data_dir}/dev/touch-events`. See `TOUCH_MESSAGE_SIZE` for the
/// on-wire layout.
///
/// `derive(Debug, PartialEq, Eq)` so the unit tests can compare
/// `TouchMessage`s directly. `Copy` because it's 20 bytes — cheap to
/// pass by value, and we want to `send()` it through an mpsc channel
/// without cloning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TouchMessage {
    action: u32,
    pointer_id: i32,
    x: i32,
    y: i32,
    pressure: i32,
}

impl TouchMessage {
    /// Serialise this `TouchMessage` into its 20-byte little-endian
    /// on-wire form, ready to `write_all` to the kr64 client connection.
    ///
    /// This is the inverse of kr64's `TouchMessage::parse` (commit
    /// `c67c498`): the byte layout must match EXACTLY.
    fn to_bytes(self) -> [u8; TOUCH_MESSAGE_SIZE] {
        let mut buf = [0u8; TOUCH_MESSAGE_SIZE];
        buf[0..4].copy_from_slice(&self.action.to_le_bytes());
        buf[4..8].copy_from_slice(&self.pointer_id.to_le_bytes());
        buf[8..12].copy_from_slice(&self.x.to_le_bytes());
        buf[12..16].copy_from_slice(&self.y.to_le_bytes());
        buf[16..20].copy_from_slice(&self.pressure.to_le_bytes());
        buf
    }

    /// Parse a 20-byte little-endian record into a `TouchMessage`.
    /// Returns `None` if the buffer is shorter than `TOUCH_MESSAGE_SIZE`.
    ///
    /// `#[cfg(test)]`-only — the host only ever writes TouchMessages
    /// (it never reads them back). Provided so the unit tests can
    /// roundtrip-verify `to_bytes`. Mirrors kr64's `TouchMessage::parse`
    /// exactly (commit `c67c498`).
    #[cfg(test)]
    fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < TOUCH_MESSAGE_SIZE {
            return None;
        }
        let action = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let pointer_id = i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let x = i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let y = i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
        let pressure = i32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
        Some(TouchMessage {
            action,
            pointer_id,
            x,
            y,
            pressure,
        })
    }
}

// ============================================================================
// 6-Z293: negotiated EVDEV bridge protocol (abstract socket).
//
// ROOT CAUSE THIS REPLACES (runs 33919636462/33921156904/33922637471/
// 33924122082 + the R12 comparison run): the abstract bridge carried raw
// 20-byte TouchMessage records, and the fb hook's read() interposer was
// expected to re-encode them into `struct input_event` records on the
// guest side. It never fired — not once, on ANY loader-path image (zero
// "INPUT read"/"INPUT poll" hook logs in every run; the app socket's
// Send-Q drained to 0 while the guest consumed the raw 20-byte records
// through its REAL read path). Lineage-22.2's minui `ev_get_input`
// (disassembled from librecovery_ui.so: read(fd, ev, 24); ret != 24 →
// fail) demands EXACTLY sizeof(struct input_event) bytes per read — a
// 20-byte TouchMessage is permanently rejected, so every tap was
// silently discarded and the menu probe measured frame_delta=0.
//
// FIX: the APP now encodes the final evdev stream itself. The guest's
// fb hook announces its native `struct input_event` size (24 on
// aarch64 — 2×i64 timeval — 16 on arm32/i386 — 2×i32) in a hello
// written right after connect; the app worker encodes each touch
// frame directly into evdev records of that size. The fb hook marks
// the slot RAW and never touches the byte stream again (read/poll
// pass through; ioctl faking stays). One protocol, zero guest-side
// byte decoding, and the guest's `read(fd, buf, sizeof(input_event))`
// gets exactly one complete record per read.
//
// Hello (guest → app, 5 bytes, written immediately after connect):
//   [0xA5, 'T', 'W', 'I', ev_size]
//   ev_size ∈ {16, 24} — sizeof(struct input_event) of the GUEST arch.
// No hello within the timeout (legacy hook): fall back to the legacy
// TouchMessage stream (previous behaviour).
// ============================================================================

/// Hello magic byte 0.
const EVDEV_HELLO_MAGIC: [u8; 4] = [0xA5, b'T', b'W', b'I'];
/// Hello size (magic + ev_size byte).
const EVDEV_HELLO_LEN: usize = 6;
/// Hello byte 5 — the DEVICE CLASS this connection mimics (6-Z294):
/// 0 = the i2c touchscreen mirror, 1 = the gpio-keys keyboard.
const EVDEV_HELLO_CLASS_TOUCH: u8 = 0;
const EVDEV_HELLO_CLASS_KEYS: u8 = 1;
/// `struct input_event` size on a 64-bit-timeval architecture (aarch64).
const EVDEV_SIZE_64: usize = 24;
/// `struct input_event` size on a 32-bit-timeval architecture (arm32/i386).
const EVDEV_SIZE_32: usize = 16;
/// How long the accept-side worker waits for the guest hello before
/// falling back to the legacy TouchMessage stream.
const EVDEV_HELLO_TIMEOUT_MS: i32 = 500;

/// Negotiated per-connection write mode for the abstract bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BridgeMode {
    /// Legacy: forward 20-byte TouchMessage records (the fb hook's
    /// read interposer was supposed to decode these — see the 6-Z293
    /// root-cause note for why that never worked).
    LegacyTouchMessage,
    /// 6-Z293/6-Z294: write final evdev records of the guest's native
    /// `struct input_event` size (the negotiated byte count), on the
    /// negotiated DEVICE CLASS (touchscreen vs gpio-keys mirror).
    RawEvdev { ev_size: usize, keys: bool },
}

/// Encode ONE `struct input_event` record (little-endian) of exactly
/// `ev_size` bytes: `{timeval, u16 type, u16 code, i32 value}`.
///
/// The timeval is zeroed (the guest's minui ignores timestamps and the
/// fb hook's legacy encoder zeroed it too). `ev_size` is the guest's
/// negotiated `sizeof(struct input_event)`: 24 = aarch64 timeval
/// (2×i64), 16 = arm32/i386 timeval (2×i32). Any other size is refused
/// (the hello only advertises these two).
fn encode_evdev_record(ev_size: usize, kind: u16, code: u16, value: i32) -> Option<Vec<u8>> {
    let time_size = ev_size.checked_sub(8)?;
    let mut out = vec![0u8; ev_size];
    match time_size {
        16 => {} // aarch64: 2×i64, zeroed
        8 => {}  // arm32/i386: 2×i32, zeroed
        _ => return None,
    }
    out[time_size..time_size + 2].copy_from_slice(&kind.to_le_bytes());
    out[time_size + 2..time_size + 4].copy_from_slice(&code.to_le_bytes());
    out[time_size + 4..time_size + 8].copy_from_slice(&value.to_le_bytes());
    Some(out)
}

/// Encode the full multi-touch DOWN frame (9 evdev records) — the
/// sequence the guest's input driver expects when a finger first
/// touches the screen. Byte-for-byte the same protocol as kr64's
/// `devices::encode_touch_down` (which additionally serves the
/// /dev/input/touch path), plus the legacy ABS_X/ABS_Y single-touch
/// records the fb hook's encoder also emitted (harmless for Type-B
/// readers, required by Type-A ones):
///
/// 1. `EV_ABS / ABS_MT_SLOT(slot)`
/// 2. `EV_ABS / ABS_MT_TRACKING_ID(tracking_id)`
/// 3. `EV_KEY / BTN_TOUCH(1)`
/// 4. `EV_KEY / BTN_TOOL_FINGER(1)`
/// 5. `EV_ABS / ABS_MT_POSITION_X(x)`
/// 6. `EV_ABS / ABS_MT_POSITION_Y(y)`
/// 7. `EV_ABS / ABS_MT_PRESSURE(pressure)`
/// 8. `EV_ABS / ABS_X(x)`
/// 9. `EV_ABS / ABS_Y(y)`
/// 10. `EV_SYN / SYN_REPORT(0)`
fn encode_evdev_touch_down(
    ev_size: usize,
    slot: i32,
    tracking_id: i32,
    x: i32,
    y: i32,
    pressure: i32,
) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(ev_size * 10);
    for (kind, code, value) in [
        (EV_ABS as u16, ABS_MT_SLOT as u16, slot),
        (EV_ABS as u16, ABS_MT_TRACKING_ID as u16, tracking_id),
        (EV_KEY as u16, BTN_TOUCH as u16, 1),
        (EV_KEY as u16, BTN_TOOL_FINGER as u16, 1),
        (EV_ABS as u16, ABS_MT_POSITION_X as u16, x),
        (EV_ABS as u16, ABS_MT_POSITION_Y as u16, y),
        (EV_ABS as u16, ABS_MT_PRESSURE as u16, pressure),
        (EV_ABS as u16, ABS_X as u16, x),
        (EV_ABS as u16, ABS_Y as u16, y),
        (EV_SYN as u16, SYN_REPORT as u16, 0),
    ] {
        out.extend(encode_evdev_record(ev_size, kind, code, value)?);
    }
    Some(out)
}

/// Encode a multi-touch MOVE frame (7 records) — BTN_TOUCH /
/// BTN_TOOL_FINGER stay pressed from DOWN.
fn encode_evdev_touch_move(
    ev_size: usize,
    slot: i32,
    x: i32,
    y: i32,
    pressure: i32,
) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(ev_size * 7);
    for (kind, code, value) in [
        (EV_ABS as u16, ABS_MT_SLOT as u16, slot),
        (EV_ABS as u16, ABS_MT_POSITION_X as u16, x),
        (EV_ABS as u16, ABS_MT_POSITION_Y as u16, y),
        (EV_ABS as u16, ABS_MT_PRESSURE as u16, pressure),
        (EV_ABS as u16, ABS_X as u16, x),
        (EV_ABS as u16, ABS_Y as u16, y),
        (EV_SYN as u16, SYN_REPORT as u16, 0),
    ] {
        out.extend(encode_evdev_record(ev_size, kind, code, value)?);
    }
    Some(out)
}

/// Encode the multi-touch RELEASE frame (5 records) — BTN_TOUCH=0 /
/// BTN_TOOL_FINGER=0 on release so the guest's InputReader never stays
/// stuck-pressed (same improvement kr64's encoder made over Nogitsune).
fn encode_evdev_touch_release(ev_size: usize, slot: i32) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(ev_size * 5);
    for (kind, code, value) in [
        (EV_ABS as u16, ABS_MT_SLOT as u16, slot),
        (EV_ABS as u16, ABS_MT_TRACKING_ID as u16, -1),
        (EV_KEY as u16, BTN_TOUCH as u16, 0),
        (EV_KEY as u16, BTN_TOOL_FINGER as u16, 0),
        (EV_SYN as u16, SYN_REPORT as u16, 0),
    ] {
        out.extend(encode_evdev_record(ev_size, kind, code, value)?);
    }
    Some(out)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct device_info {
    name: [c_char; 80],
    driver_version: c_int,
    id: input_id,
    physical_location: [c_char; 80],
    unique_id: [c_char; 80],
    key_bitmask: [u8; (KEY_MAX as usize + 1) / 8],
    abs_bitmask: [u8; (ABS_MAX as usize + 1) / 8],
    rel_bitmask: [u8; (REL_MAX as usize + 1) / 8],
    sw_bitmask: [u8; (SW_MAX as usize + 1) / 8],
    led_bitmask: [u8; (LED_MAX as usize + 1) / 8],
    ff_bitmask: [u8; (FF_MAX as usize + 1) / 8],
    prop_bitmask: [u8; (INPUT_PROP_MAX as usize + 1) / 8],
    abs_max: [u32; ABS_CNT as usize],
    abs_min: [u32; ABS_CNT as usize],
}

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

/// Copy a Rust `&str` into a fixed-size C `char` array (NUL-terminated).
///
/// `arr` can be either `&mut [u8; N]` (when the field is declared as
/// `[u8; N]`) or `&mut [c_char; N]` (when the field is declared as
/// `[c_char; N]`, which on aarch64-linux-android is `[i8; N]`). We accept
/// a generic mutable slice and cast each byte via `as`, so both layouts
/// work without changing the call sites.
///
/// Note: this function uses pointer casting internally because Rust's
/// type system doesn't let us write a single generic signature that
/// accepts both `&mut [u8; N]` and `&mut [i8; N]` without `unsafe`.
/// The `unsafe` block is bounded and the operation is sound: we never
/// read past `len` bytes, and `len` is clamped to `COUNT`.
///
/// Robustness notes:
///   - If `data` contains an interior NUL byte, the string is truncated
///     at that byte rather than panicking. The data dir path is supplied
///     by the Java side via JNI (`set_data_dir`); an interior NUL would
///     be malformed but shouldn't kill the input thread (and thus the
///     entire guest's touch/key input).
///   - If the string (after truncation) is longer than `COUNT - 1` bytes,
///     we copy at most `COUNT - 1` bytes and write a NUL at position
///     `COUNT - 1`. The previous implementation set `len = COUNT` in the
///     overflow case, which copied COUNT bytes of string data WITHOUT a
///     NUL terminator — the C consumer (`EventHub::parseDeviceName` etc.)
///     would then read past the array into adjacent struct fields looking
///     for a terminator. In practice the paths are well under 80 bytes,
///     but the fix protects against future longer paths.
fn copy_to_cstr<T, const COUNT: usize>(data: &str, arr: &mut [T; COUNT]) {
    // The device_info fields are zero-initialized by the caller
    // (mem::zeroed() / MaybeUninit::zeroed()), so the trailing bytes
    // are already 0 — we only need to overwrite the prefix we copy.
    let bytes = data.as_bytes();
    // Truncate at the first interior NUL byte, if any, so the rest of
    // the string (after the NUL) isn't copied. Without this, a malformed
    // path like "/data/\0/etc" would leak "/etc" into the array past
    // the implicit NUL terminator.
    let nul_pos = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    // Reserve one byte for the NUL terminator.
    let max_len = if COUNT == 0 { 0 } else { COUNT - 1 };
    let len = nul_pos.min(max_len);

    // Cast the [T; COUNT] array to a [u8; COUNT] pointer for the copy.
    // This is sound because:
    //   - On aarch64-linux-android, c_char == i8, and [i8; N] has the same
    //     memory layout as [u8; N] (both are 1-byte elements, N elements).
    //   - On targets where c_char == u8, this is a no-op cast.
    //   - We only write `len` < COUNT bytes, so we never overflow.
    unsafe {
        let ptr = arr.as_mut_ptr() as *mut u8;
        if len > 0 {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, len);
        }
        // Explicit NUL terminator at position `len`. The caller already
        // zeroed the array, but writing it here makes the invariant
        // explicit and protects against future callers that forget.
        if len < COUNT {
            *ptr.add(len) = 0;
        }
    }
}

const MAX_POINTERS: usize = 5;

// Input channel state. These Mutexes are accessed from the JNI touch/key
// callbacks (handle_touch, send_key_code) AND from the server threads
// (touch_server, key_server). We deliberately recover from poison
// (`.lock().unwrap_or_else(|e| e.into_inner())`) at every call site: a
// panic in one callback must NOT permanently disable input for the rest
// of the session. The alternative — letting PoisonError propagate —
// would mean a single panic kills all subsequent touch/key events,
// bricking the guest's UI with no recovery short of an app restart.
//
// `INPUT_SENDER` carries `TouchMessage` records (NOT raw `input_event`s):
// the host now forwards raw MotionEvent data to kr64 over the
// `{data_dir}/dev/touch-events` IPC socket, and kr64 re-encodes it via
// `devices::encode_touch_*`. This split lets kr64 own the per-slot
// tracking-ID state machine + the DeviceInfo header — the host no
// longer needs to know about `EV_ABS`/`ABS_MT_*`/`BTN_*` constants for
// the touch path (the key path still does, so `KEY_SENDER` is unchanged).
static INPUT_SENDER: Lazy<Mutex<Option<Sender<TouchMessage>>>> = Lazy::new(|| Mutex::new(None));

/// 6-Z294: the gpio-keys-mirror connection of the abstract bridge.
/// minui's ev_init classifies devices by EVIOCGID's bustype — a zeroed
/// id made BOTH bridge fds keyboards and dropped every record; the hook
/// now presents event0 as BUS_I2C touchscreen + event1 as BUS_HOST
/// gpio-keys, and the app routes touch frames and key records to the
/// matching connections. Keys sent only through INPUT_SENDER would land
/// on the touchscreen device whose handler ignores EV_KEY.
static KEY_BRIDGE_SENDER: Lazy<Mutex<Option<(u64, Sender<TouchMessage>)>>> =
    Lazy::new(|| Mutex::new(None));

/// Monotonic connection generation for `detach_bridge_sender`'s identity
/// check (`Sender::same_channel` is newer than the CI toolchain).
static BRIDGE_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static KEY_SENDER: Lazy<Mutex<Option<Sender<input_event>>>> = Lazy::new(|| Mutex::new(None));

pub fn start_input_system(width: i32, height: i32) {
    // `width` and `height` are no longer used by `touch_server` (kr64
    // builds the DeviceInfo itself from `cfg.width`/`cfg.height` in
    // `spawn_touch_accept_thread`, commit `c67c498`). The JNI signature
    // is preserved for binary compatibility with the loaded Java
    // class (`io/twoyi/Renderer.init`) — the values are simply
    // ignored. Prefix with `_` so rustc doesn't warn about unused
    // args.
    let _ = (width, height);
    thread::spawn(|| {
        touch_server();
    });
    // 6-Z182: abstract-namespace mirror — the chroot-proof listener the
    // jailed TWRP fb hook connects to (see touch_server_abstract).
    thread::spawn(|| {
        touch_server_abstract();
    });
    thread::spawn(|| {
        key_server();
    });
}

pub fn input_event_write(
    tx: &std::sync::mpsc::Sender<input_event>,
    kind: i32,
    code: i32,
    val: i32,
) {
    let mut tp = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let _ = unsafe { clock_gettime(CLOCK_MONOTONIC, &mut tp) };
    let tv = timeval {
        tv_sec: tp.tv_sec,
        tv_usec: tp.tv_nsec / 1000,
    };

    let ev = input_event {
        kind: kind as u16,
        code: code as u16,
        value: val,
        time: tv,
    };
    let _ = tx.send(ev);
}

/// JNI entry point: dispatch an Android `MotionEvent` to the guest's
/// touch input device.
///
/// Refactor (commit `4-A`): previously this function encoded the
/// `MotionEvent` into Linux `InputEvent`s (`EV_ABS`/`ABS_MT_*`/
/// `BTN_TOUCH`/`SYN_REPORT`) and wrote them to the guest-facing
/// `/dev/input/touch` socket directly. As of kr64's commit `c67c498`
/// (task 3-A), kr64 OWNS that socket — it binds it, sends the 896-byte
/// `DeviceInfo` header on `accept()`, and dispatches `InputEvent`s
/// itself. The host's role is now to forward RAW `MotionEvent` data
/// (`action` + `pointer_id` + `x` + `y` + `pressure`) to kr64 via the
/// `{data_dir}/dev/touch-events` IPC socket — kr64 re-encodes it via
/// `devices::encode_touch_*` and applies its own per-slot tracking-ID
/// state machine.
///
/// The local `G_INPUT_MT` per-slot state has been REMOVED — kr64 owns
/// that state now (it has its own `tracking_ids: [i32; MAX_POINTERS]`
/// in `touch_connection_loop`). The host's `ACTION_UP` handler now
/// emits UP records for ALL slots `0..MAX_POINTERS`; kr64 drops
/// UP-without-DOWN defensively (commit `c67c498`'s
/// `encode_touch_message` test `up_without_down_is_dropped`), so this
/// is safe — and it preserves the OLD defensive behaviour of releasing
/// any slot that missed a `POINTER_UP` event.
///
/// `MotionAction::PointerUp` and `MotionAction::Cancel` are mapped to
/// `touch_action::CANCEL` (kr64 treats CANCEL identically to UP — see
/// its `encode_touch_message` test `cancel_treated_as_up`). The
/// pointer_id field identifies which slot to release.
///
/// `MotionAction::Move` iterates every pointer in this `MotionEvent`
/// (Android batches historical samples, but the current sample of each
/// active pointer is at indices `0..pointer_count()`) and emits one
/// `TouchMessage` per pointer — same per-pointer iteration the OLD
/// code did, but now sending raw pointer data instead of encoded
/// events.
pub fn handle_touch(ev: MotionEvent) {
    let opt = INPUT_SENDER.lock().unwrap_or_else(|e| e.into_inner());
    let Some(tx) = opt.as_ref() else {
        // No kr64 connection yet (or the previous kr64 client
        // disconnected and the worker drained) — the event is dropped
        // here. 6-Z289d: this silent drop blinded the menu probe
        // (run 33915900634: taps at +22.8/+34.3 s reached the guest,
        // taps at +120 s vanished) — rate-limit-log the drop so the
        // artifacts name it (1/s max; a full tap stream is ~10 ev/s).
        {
            use std::sync::atomic::{AtomicU64, Ordering};
            static LAST_LOG_MS: AtomicU64 = AtomicU64::new(0);
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            let last = LAST_LOG_MS.load(Ordering::Relaxed);
            if now_ms.saturating_sub(last) >= 1000 {
                LAST_LOG_MS.store(now_ms, Ordering::Relaxed);
                log::warn!(
                    "[INPUT] handle_touch DROP: no kr64 touch client connected                      (INPUT_SENDER is None) — touch events are being discarded"
                );
            }
        }
        return;
    };

    let action = ev.action();
    let pointer_index = ev.pointer_index();
    let pointer = ev.pointer_at_index(pointer_index);
    let pointer_id = pointer.pointer_id();
    let pressure = pointer.pressure();

    // Bounds check: pointer_id must be < MAX_POINTERS. kr64's
    // `encode_touch_message` also bounds-checks and drops, but doing
    // it here avoids sending a record that will be silently discarded
    // AND matches the behaviour of the old code (which used pointer_id
    // as an array index into `G_INPUT_MT`).
    if (pointer_id as usize) >= MAX_POINTERS || pointer_id < 0 {
        return;
    }

    match action {
        // ACTION_DOWN / ACTION_POINTER_DOWN: a new finger touched the
        // screen. Forward the pointer_id + position + pressure to kr64,
        // which assigns a fresh tracking ID + emits the 8-event DOWN
        // frame (BTN_TOUCH/BTN_TOOL_FINGER=1 + ABS_MT_SLOT/TRACKING_ID/
        // POSITION_X/Y/PRESSURE + SYN_REPORT — see
        // `devices::encode_touch_down`).
        MotionAction::Down | MotionAction::PointerDown => {
            let msg = TouchMessage {
                action: touch_action::DOWN,
                pointer_id,
                x: pointer.x() as i32,
                y: pointer.y() as i32,
                pressure: pressure as i32,
            };
            // `send` returns Err only if the receiver was dropped —
            // i.e. the kr64 client disconnected and a new connection
            // hasn't been accepted yet. Silently drop the event (the
            // next MotionEvent will see `None` and bail at the top of
            // this function).
            if let Err(e) = tx.send(msg) {
                // 6-Z289f: dead worker (its rx was dropped after a write
                // failure) — this was the LAST silent drop: name it.
                {
                    use std::sync::atomic::{AtomicU64, Ordering};
                    static LAST_LOG_MS: AtomicU64 = AtomicU64::new(0);
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    let last = LAST_LOG_MS.load(Ordering::Relaxed);
                    if now_ms.saturating_sub(last) >= 1000 {
                        LAST_LOG_MS.store(now_ms, Ordering::Relaxed);
                        log::error!(
                            "[INPUT] handle_touch DROP: touch channel send failed ({}) —                              the abstract/path worker is gone; events discarded",
                            e
                        );
                    }
                }
            }
        }
        // ACTION_MOVE: emit one TouchMessage per active pointer in
        // this batched MotionEvent. kr64's `encode_touch_move` writes
        // the 5-event MOVE frame (ABS_MT_SLOT/POSITION_X/Y/PRESSURE +
        // SYN_REPORT) per record.
        //
        // We DON'T filter by "is this slot active?" (the OLD code did,
        // via `G_INPUT_MT[pid] != 0`) — kr64's `encode_touch_message`
        // already drops MOVE-without-DOWN (commit `c67c498`'s test
        // `move_without_down_is_dropped`), so forwarding every pointer
        // is safe and avoids the need for duplicate state on the host.
        MotionAction::Move => {
            for i in 0..ev.pointer_count() {
                let p = ev.pointer_at_index(i);
                let pid = p.pointer_id();
                if pid < 0 || (pid as usize) >= MAX_POINTERS {
                    continue;
                }
                let msg = TouchMessage {
                    action: touch_action::MOVE,
                    pointer_id: pid,
                    x: p.x() as i32,
                    y: p.y() as i32,
                    pressure: p.pressure() as i32,
                };
                if tx.send(msg).is_err() {
                    return; // client disconnected — drop remaining moves
                }
            }
        }
        // ACTION_UP: the LAST remaining pointer has been released.
        // Emit UP records for every slot 0..MAX_POINTERS — kr64 drops
        // UP-without-DOWN defensively, so only slots that received a
        // real DOWN (and haven't been released yet) actually emit the
        // 5-event release frame. This preserves the OLD defensive
        // behaviour of releasing any slot that missed a POINTER_UP.
        //
        // (x/y/pressure are 0 because UP doesn't carry position info
        // — kr64's `encode_touch_release` only emits ABS_MT_SLOT +
        // ABS_MT_TRACKING_ID=-1 + BTN_TOUCH/BTN_TOOL_FINGER=0 +
        // SYN_REPORT.)
        MotionAction::Up => {
            for slot in 0..MAX_POINTERS as i32 {
                let msg = TouchMessage {
                    action: touch_action::UP,
                    pointer_id: slot,
                    x: 0,
                    y: 0,
                    pressure: 0,
                };
                if tx.send(msg).is_err() {
                    return; // client disconnected
                }
            }
        }
        // ACTION_POINTER_UP / ACTION_CANCEL: a single (non-last)
        // pointer went up. Release just that slot. kr64 treats CANCEL
        // identically to UP (test `cancel_treated_as_up`).
        MotionAction::Cancel | MotionAction::PointerUp => {
            let msg = TouchMessage {
                action: touch_action::CANCEL,
                pointer_id,
                x: 0,
                y: 0,
                pressure: 0,
            };
            if let Err(e) = tx.send(msg) {
                // 6-Z289f: dead worker (its rx was dropped after a write
                // failure) — this was the LAST silent drop: name it.
                {
                    use std::sync::atomic::{AtomicU64, Ordering};
                    static LAST_LOG_MS: AtomicU64 = AtomicU64::new(0);
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    let last = LAST_LOG_MS.load(Ordering::Relaxed);
                    if now_ms.saturating_sub(last) >= 1000 {
                        LAST_LOG_MS.store(now_ms, Ordering::Relaxed);
                        log::error!(
                            "[INPUT] handle_touch DROP: touch channel send failed ({}) —                              the abstract/path worker is gone; events discarded",
                            e
                        );
                    }
                }
            }
        }
        // Outside / Hover / ButtonPress / Scroll / etc. — not
        // relevant to a Type-B multi-touch touchscreen. Silently
        // ignore (same as the OLD code's `_ => {}` arm).
        _ => {}
    }
}

/// Bind the host→kr64 touch-events IPC socket at
/// `{data_dir}/dev/touch-events` and accept connections from kr64's
/// `spawn_touch_accept_thread` (commit `c67c498`).
///
/// On `accept()`, the host creates an mpsc channel and stores the
/// `Sender` in `INPUT_SENDER`. The `handle_touch` JNI callback then
/// sends `TouchMessage` records through the channel; a per-connection
/// worker thread reads them and `write_all`s the 20-byte LE bytes to
/// the accepted `UnixStream`.
///
/// The host does NOT send the `DeviceInfo` header (896 bytes) — kr64
/// builds + sends that itself (`devices::make_touch_device` from
/// commit `370b8ee`). The host only forwards raw MotionEvent data.
///
/// Reconnection handling: if kr64 disconnects (e.g. the guest's
/// `EventHub` closes the device after a suspend/resume), the worker
/// thread exits, the `Sender` is dropped, and `handle_touch` silently
/// drops events until a new kr64 connection arrives. The accept loop
/// is structured so a new channel + worker are created on every
/// `accept()` — replacing any previous connection.
fn touch_server() {
    let path = touch_events_path();

    // Make sure the parent directory exists. The path is
    // `{data_dir}/dev/touch-events`, so the parent is `{data_dir}/dev`
    // — which may not exist on a fresh install. `UnixListener::bind`
    // fails with ENOENT if the parent dir is missing.
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Remove stale socket file from a previous run. `UnixListener::bind`
    // fails with EADDRINUSE if the path already exists as a socket
    // file (e.g. from a previous process that crashed without unlinking).
    let _ = std::fs::remove_file(&path);

    // `.unwrap()` here would panic the input thread, killing the
    // touch input system silently. kr64 would then block for 30s on
    // `UnixStream::connect` to this path (commit `c67c498`'s
    // `touch_connection_loop`) and finally log a clear TODO before
    // giving up. Log the error here and exit the thread gracefully
    // instead — kr64's 30s timeout + clear log message handles the
    // host-side failure mode.
    let listener = match unix_socket::UnixListener::bind(&path) {
        Ok(l) => {
            info!(
                "[INPUT] touch-events IPC server listening at {} (kr64 will connect here)",
                path
            );
            l
        }
        Err(e) => {
            log::error!(
                "[INPUT] failed to bind touch-events IPC socket at {}: {} — \
                 kr64 will block for 30s then give up (guest touch device will \
                 be advertised but receive no events). Check permissions + path length.",
                path,
                e
            );
            return;
        }
    };

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                info!("[INPUT] kr64 touch client connected to touch-events IPC socket");

                // Create a new channel for this connection. The Sender
                // is stored in `INPUT_SENDER` (replacing any previous
                // one — the previous worker thread will exit when its
                // receiver hits a channel-closed on the next `recv()`).
                // The Receiver is moved into the worker thread below.
                let (tx, rx) = channel::<TouchMessage>();
                *INPUT_SENDER.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);

                thread::spawn(move || {
                    while let Ok(msg) = rx.recv() {
                        let bytes = msg.to_bytes();
                        if stream.write_all(&bytes).is_err() {
                            // kr64 disconnected — drain any queued
                            // messages so the Sender doesn't stay
                            // "full" forever (the channel is unbounded,
                            // but draining is cheap and makes the
                            // disconnect observable from `handle_touch`
                            // on the NEXT call: `tx.send` returns Err
                            // because the receiver was dropped when
                            // this thread exits).
                            while rx.recv().is_ok() {}
                            return;
                        }
                    }
                    // Channel disconnected — a new kr64 connection took
                    // over (or the host is shutting down). Exit so the
                    // new worker can take its turn on the next `accept()`.
                });
            }
            Err(e) => {
                // A transient accept error (EMFILE, ENOMEM, EINTR…) must
                // not kill the touch server for the rest of the session —
                // back off briefly and keep serving.
                info!(
                    "[INPUT] touch-events server accept error: {} — continuing",
                    e
                );
                thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    info!("[INPUT] drop touch-events listener!");
}

/// 6-Z182: ABSTRACT-namespace mirror of the touch-events socket.
///
/// WHY: the TWRP fb hook's INPUT bridge (twrp_fb_hook.c inbr_connect)
/// runs INSIDE the jailed recovery process — after kr64's chroot/
/// pivot_root, the jail's filesystem root IS the rootfs, so a connect()
/// whose sockaddr carries the ABSOLUTE HOST path
/// `/data/user/0/<pkg>/dev/touch-events` resolves INSIDE the jail and
/// ENOENTs (run 33061152563: every filesystem candidate failed with
/// read-verify -EINVAL — connect never happened). Abstract-namespace
/// AF_UNIX sockets (`\0<name>`) are resolved in the NETWORK namespace,
/// not the filesystem — the jail shares the host netns, so a chrooted
/// guest connects with ZERO path translation. The filesystem listener
/// above stays for compatibility (kr64's spawn_touch_accept_thread and
/// the AOSP EventHub path still use it).
///
/// Wire format is IDENTICAL to the fs listener: accepted clients get
/// 20-byte little-endian TouchMessage records from the same
/// INPUT_SENDER channel (a client here REPLACES any fs-listener client
/// — single-consumer semantics are preserved).
const ABSTRACT_TOUCH_NAME: &[u8] = b"io.twoyi.touch";

fn touch_server_abstract() {
    unsafe {
        let fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0);
        if fd < 0 {
            log::error!(
                "[INPUT] abstract touch socket(): {} — jailed TWRP touch bridge disabled",
                std::io::Error::last_os_error()
            );
            return;
        }
        // sockaddr_un: u16 family + sun_path with LEADING NUL (abstract)
        let mut addr: [u8; 2 + 1 + 32] = [0u8; 2 + 1 + 32];
        addr[0] = AF_UNIX as u8;
        // sun_path[0] stays 0 (abstract marker); name follows
        let namelen = ABSTRACT_TOUCH_NAME.len().min(31);
        addr[3..3 + namelen].copy_from_slice(&ABSTRACT_TOUCH_NAME[..namelen]);
        let addrlen = 3 + namelen; // family(2) + leading NUL + name
        if bind(fd, addr.as_ptr() as *const sockaddr, addrlen as u32) < 0 {
            log::error!(
                "[INPUT] abstract touch bind(\\0{}) failed: {}",
                String::from_utf8_lossy(ABSTRACT_TOUCH_NAME),
                std::io::Error::last_os_error()
            );
            close(fd);
            return;
        }
        if listen(fd, 4) < 0 {
            log::error!(
                "[INPUT] abstract touch listen(): {}",
                std::io::Error::last_os_error()
            );
            close(fd);
            return;
        }
        info!(
            "[INPUT] abstract touch-events IPC server listening (\\0{}) — chroot-proof path for the jailed TWRP fb hook",
            String::from_utf8_lossy(ABSTRACT_TOUCH_NAME)
        );
        loop {
            let client = accept4(fd, std::ptr::null_mut(), std::ptr::null_mut(), SOCK_CLOEXEC);
            if client < 0 {
                let e = std::io::Error::last_os_error();
                if e.raw_os_error() == Some(EINTR) {
                    continue;
                }
                // Transient resource errors: keep the listener alive
                // (previously ANY error returned, permanently disabling
                // the jailed touch bridge until app restart).
                if matches!(
                    e.raw_os_error(),
                    Some(EMFILE) | Some(ENOMEM) | Some(ENFILE) | Some(ECONNABORTED)
                ) {
                    log::warn!(
                        "[INPUT] abstract touch accept() transient: {} — retrying",
                        e
                    );
                    thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                log::error!("[INPUT] abstract touch accept(): {}", e);
                close(fd);
                return;
            }
            info!(
                "[INPUT] jailed touch client connected to abstract touch socket (fd {})",
                client
            );
            let (tx, rx) = channel::<TouchMessage>();
            // Mirror the fs-listener worker: drain the channel into the
            // client fd; exit (releasing the bridge sender) on write
            // error.
            //
            // 6-Z293: the worker first waits for the guest's hello
            // ([0xA5,'T','W','I', ev_size, class]) to pick the write
            // mode. The hello is read HERE — inside the worker — so a
            // client that never speaks (legacy hook) only delays this
            // connection's first delivery by the timeout, never the
            // accept loop (connections keep accepting; early
            // TouchMessages buffer in `rx` until the negotiation
            // resolves).
            thread::spawn(move || {
                let my_gen = BRIDGE_GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let mode = negotiate_bridge_mode(client);
                match mode {
                    BridgeMode::RawEvdev {
                        ev_size,
                        keys: false,
                    } => info!(
                        "[INPUT] abstract bridge negotiated evdev mode (touch device, ev_size={}, fd {})",
                        ev_size, client
                    ),
                    BridgeMode::RawEvdev {
                        ev_size,
                        keys: true,
                    } => info!(
                        "[INPUT] abstract bridge negotiated evdev mode (gpio-keys device, ev_size={}, fd {})",
                        ev_size, client
                    ),
                    BridgeMode::LegacyTouchMessage => {}
                }
                // 6-Z294: register the connection under its negotiated
                // class — last negotiation per class wins (the class is
                // only knowable AFTER the hello, so registration happens
                // here, not at accept time; minui opens both devices
                // within ~1 ms and the first touch arrives seconds later,
                // so the pre-registration window is theoretical).
                // Touchscreen connections serve INPUT_SENDER (MotionEvent
                // frames); gpio-keys connections serve KEY_BRIDGE_SENDER
                // (the send_key_code EV_KEY records) — the guest's
                // touchscreen handler ignores EV_KEY records and the
                // keyboard dispatch drops codes absent from its bitmap.
                match mode {
                    BridgeMode::RawEvdev { keys: true, .. } => {
                        *KEY_BRIDGE_SENDER.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some((my_gen, tx));
                    }
                    BridgeMode::RawEvdev { keys: false, .. } | BridgeMode::LegacyTouchMessage => {
                        *INPUT_SENDER.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
                    }
                }
                let mut finger: Option<(i32, i32)> = None; // (slot, tracking_id)
                loop {
                    match rx.recv() {
                        Ok(msg) => {
                            let bytes = match mode {
                                BridgeMode::LegacyTouchMessage => msg.to_bytes().to_vec(),
                                BridgeMode::RawEvdev { ev_size, .. } => {
                                    match encode_evdev(ev_size, &mut finger, msg) {
                                        Some(b) => b,
                                        // Defensive drop (MOVE/UP without
                                        // DOWN, unknown slot, bad size) —
                                        // same semantics kr64's dispatcher
                                        // applies on this path.
                                        None => continue,
                                    }
                                }
                            };
                            let mut off = 0usize;
                            let mut dead = false;
                            while off < bytes.len() {
                                let n = write(
                                    client,
                                    bytes[off..].as_ptr() as *const c_void,
                                    bytes.len() - off,
                                );
                                if n < 0 {
                                    // EINTR before any byte was written: retry,
                                    // don't kill a healthy connection over a
                                    // spurious signal.
                                    if std::io::Error::last_os_error().raw_os_error() == Some(EINTR)
                                        && off == 0
                                    {
                                        continue;
                                    }
                                    dead = true;
                                    break;
                                }
                                if n == 0 {
                                    dead = true;
                                    break;
                                }
                                off += n as usize;
                            }
                            if dead {
                                // 6-Z289f: this exit is the last silent link in
                                // the touch chain — after it, INPUT_SENDER keeps
                                // a DEAD sender and handle_touch's tx.send fails
                                // silently forever (runs 33917738821/33915900634:
                                // taps delivered at +23/+34 s, everything later
                                // vanished with no log line anywhere). Name the
                                // failure + the errno (rate-limited to 1/s).
                                {
                                    use std::sync::atomic::{AtomicU64, Ordering};
                                    static LAST_LOG_MS: AtomicU64 = AtomicU64::new(0);
                                    let now_ms = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_millis() as u64)
                                        .unwrap_or(0);
                                    let last = LAST_LOG_MS.load(Ordering::Relaxed);
                                    if now_ms.saturating_sub(last) >= 1000 {
                                        LAST_LOG_MS.store(now_ms, Ordering::Relaxed);
                                        let errno = std::io::Error::last_os_error().raw_os_error();
                                        log::error!(
                                            "[INPUT] abstract touch worker DEAD: write to                                          the hook bridge failed (last errno={:?}) —                                          client fd {} closed; ALL further guest touch                                          input is dropped until a reconnect",
                                            errno, client
                                        );
                                    }
                                }
                                // 6-Z294b: NEVER close the client on this
                                // side. A closed app socket EOFs the guest's
                                // fd, and in raw mode the guest's reader
                                // BUSY-SPINS on read()->0 / POLLHUP (the
                                // 6-Z294b TWRP BOOT_FAIL was init starved by
                                // exactly that spin on the replaced touch
                                // connection). Park the worker holding the
                                // fd: the connection stays ESTAB, the guest's
                                // poll blocks cleanly, nothing spins.
                                detach_bridge_sender(my_gen);
                                loop {
                                    std::thread::park_timeout(std::time::Duration::from_secs(3600));
                                }
                            }
                        }
                        Err(_) => {
                            // Channel dead (a newer same-class connection took
                            // over) — see the 6-Z294b note: park, don't close.
                            detach_bridge_sender(my_gen);
                            loop {
                                std::thread::park_timeout(std::time::Duration::from_secs(3600));
                            }
                        }
                    }
                }
            });
        }
    }
}

/// Read the guest's 6-Z293 hello from a freshly accepted abstract-bridge
/// connection and pick the write mode.
///
/// `EVDEV_HELLO_TIMEOUT_MS` bound via `SO_RCVTIMEO` on a THROWAWAY dup
/// of the socket fd: the timeout must only affect THIS read, not the
/// socket's lifetime (the worker later writes touch frames through the
/// original fd with no timeout — a 500 ms write timeout would corrupt
/// the frame stream under load). If the peer closes before the hello,
/// the dup read returns 0 — also legacy mode (the worker's next write
/// will fail and take the normal dead-worker path).
fn negotiate_bridge_mode(client: c_int) -> BridgeMode {
    unsafe {
        let dup_fd = dup(client);
        if dup_fd < 0 {
            return BridgeMode::LegacyTouchMessage;
        }
        let timeout = libc::timeval {
            tv_sec: 0,
            tv_usec: (EVDEV_HELLO_TIMEOUT_MS as i64) * 1000,
        };
        setsockopt(
            dup_fd,
            SOL_SOCKET,
            SO_RCVTIMEO,
            &timeout as *const libc::timeval as *const c_void,
            std::mem::size_of::<libc::timeval>() as u32,
        );
        let mut hello = [0u8; EVDEV_HELLO_LEN];
        let mut got = 0usize;
        while got < EVDEV_HELLO_LEN {
            let n = read(
                dup_fd,
                hello[got..].as_mut_ptr() as *mut c_void,
                EVDEV_HELLO_LEN - got,
            );
            if n < 0 {
                let e = std::io::Error::last_os_error().raw_os_error();
                if e == Some(EINTR) {
                    continue;
                }
                break; // timeout (EAGAIN) or error — legacy mode
            }
            if n == 0 {
                break; // peer hung up — legacy mode (dead-worker path later)
            }
            got += n as usize;
        }
        close(dup_fd);
        if got == EVDEV_HELLO_LEN && hello[0..4] == EVDEV_HELLO_MAGIC {
            let ev_size = hello[4] as usize;
            let keys = hello[5] == EVDEV_HELLO_CLASS_KEYS;
            if ev_size == EVDEV_SIZE_64 || ev_size == EVDEV_SIZE_32 {
                return BridgeMode::RawEvdev { ev_size, keys };
            }
            log::warn!(
                "[INPUT] abstract bridge hello advertised unsupported ev_size={} — legacy mode",
                ev_size
            );
        }
        BridgeMode::LegacyTouchMessage
    }
}

/// Encode one guest-bound `TouchMessage` into the negotiated evdev
/// stream. `finger` carries the per-connection active-finger state:
/// `None` = finger up; `Some((slot, tracking_id))` = pressed.
///
/// Semantics mirror kr64's touch dispatcher: DOWN assigns a fresh
/// tracking id; MOVE/UP for a slot other than the active one are
/// dropped (single-finger virtual screen); UP for the active slot
/// emits ONE release frame (handle_touch's ACTION_UP fans out a
/// record per slot — only the active one produces bytes here).
fn encode_evdev(
    ev_size: usize,
    finger: &mut Option<(i32, i32)>,
    msg: TouchMessage,
) -> Option<Vec<u8>> {
    static NEXT_TRACKING_ID: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);
    match msg.action {
        touch_action::DOWN => {
            let tid = NEXT_TRACKING_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            *finger = Some((msg.pointer_id, tid));
            encode_evdev_touch_down(ev_size, msg.pointer_id, tid, msg.x, msg.y, msg.pressure)
        }
        touch_action::MOVE => match *finger {
            Some((slot, _)) if slot == msg.pointer_id => {
                encode_evdev_touch_move(ev_size, slot, msg.x, msg.y, msg.pressure)
            }
            _ => None,
        },
        touch_action::UP | touch_action::CANCEL => match *finger {
            Some((slot, _)) if slot == msg.pointer_id => {
                *finger = None;
                encode_evdev_touch_release(ev_size, slot)
            }
            _ => None,
        },
        // 6-Z293c: synthetic key press/release through the same evdev
        // stream — EV_KEY(code, value) + SYN_REPORT. `x` = linux keycode.
        touch_action::KEY_DOWN | touch_action::KEY_UP => {
            let mut out = Vec::with_capacity(ev_size * 2);
            out.extend(encode_evdev_record(
                ev_size,
                EV_KEY as u16,
                msg.x as u16,
                if msg.action == touch_action::KEY_DOWN {
                    1
                } else {
                    0
                },
            )?);
            out.extend(encode_evdev_record(
                ev_size,
                EV_SYN as u16,
                SYN_REPORT as u16,
                0,
            )?);
            Some(out)
        }
        _ => None,
    }
}

/// 6-Z294: drop a dead connection's registration from KEY_BRIDGE_SENDER
/// so a reconnect isn't shadowed by a stale sender. Identity = the
/// connection's generation (`Sender::same_channel` is newer than the
/// CI toolchain; INPUT_SENDER only ever holds touch connections under
/// negotiation-time registration, so it needs no cleanup here).
fn detach_bridge_sender(my_gen: u64) {
    let mut key = KEY_BRIDGE_SENDER.lock().unwrap_or_else(|e| e.into_inner());
    if key.as_ref().is_some_and(|(gen, _)| *gen == my_gen) {
        *key = None;
    }
}

/// Set the bit at position `n` in a `key_bitmask` byte array.
/// `key_bitmask` is a bitmap of `KEY_MAX` bits, indexed by Linux keycode.
/// The guest's `EventHub` reads this via `EVIOCGKEY` to learn which keys
/// the virtual keyboard can emit.
fn set_key_bit(bitmask: &mut [u8], keycode: usize) {
    let byte = keycode / 8;
    let bit = keycode % 8;
    if byte < bitmask.len() {
        bitmask[byte] |= 1 << bit;
    }
}

fn generate_key_device() -> device_info {
    let mut info: device_info = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };

    info.driver_version = 0x1;
    info.id.product = 0x1;

    copy_to_cstr(KEY_DEVICE_NAME, &mut info.name);
    copy_to_cstr(&key_path(), &mut info.physical_location);
    copy_to_cstr(KEY_DEVICE_UNIQUE_ID, &mut info.unique_id);

    // Advertise every key that `android_keycode_to_linux` can emit, so the
    // guest's InputReader doesn't drop them as "out of capability range".
    // The legacy `info.key_bitmask[14] = 0x1C` only advertised KEY_BACK,
    // KEY_HOMEPAGE, and KEY_MENU — which meant KEYCODE_HOME silently
    // fell back through Android's compatibility shim.
    for &keycode in &[
        KEY_HOME,       // 102  — KEYCODE_HOME
        KEY_BACK,       // 158  — KEYCODE_BACK
        KEY_END,        // 107  — KEYCODE_ENDCALL
        KEY_VOLUMEUP,   // 115  — KEYCODE_VOLUME_UP
        KEY_VOLUMEDOWN, // 114  — KEYCODE_VOLUME_DOWN
        KEY_POWER,      // 116  — KEYCODE_POWER
        KEY_MENU,       // 139  — KEYCODE_MENU
        KEY_SEARCH,     // 217  — KEYCODE_SEARCH
        KEY_APPSELECT,  // 0x244 — KEYCODE_APP_SWITCH (recents)
        KEY_HOMEPAGE,   // 172  — KEYCODE_HOME alternate
    ] {
        set_key_bit(&mut info.key_bitmask, keycode as usize);
    }

    info
}

/// Map an Android `KeyEvent.KEYCODE_*` constant to the corresponding
/// Linux input subsystem `KEY_*` code, so the guest's `InputManagerService`
/// receives the correct key event.
///
/// Prior to this change, `send_key_code` ignored its `keycode` argument
/// and always emitted `KEY_BACK`. That worked by accident for the only
/// caller (`Render2Activity.onBackPressed` → `KEYCODE_HOME`) because the
/// guest's `InputManagerService` translates a missing `KEY_HOME` capability
/// on the virtual keyboard device into a fallback. But it broke any future
/// caller that wanted to send a different key (e.g. volume, recents, power).
///
/// Constants are aligned with `linux/input-event-codes.h` and
/// `android.view.KeyEvent` so the mapping is stable across kernel versions.
fn android_keycode_to_linux(keycode: i32) -> Option<i32> {
    // Android KeyEvent.KEYCODE_* constants (subset that makes sense for a
    // virtual navigation device). See frameworks/base/core/java/android/view/KeyEvent.java
    // KEY_CALL is intentionally omitted because the uinput-sys crate
    // (tiann/rust-uinput-sys) doesn't export it.
    match keycode {
        3 => Some(KEY_HOME),        // KEYCODE_HOME         → KEY_HOME (102)
        4 => Some(KEY_BACK),        // KEYCODE_BACK         → KEY_BACK (158)
        6 => Some(KEY_END),         // KEYCODE_ENDCALL      → KEY_END (107)
        24 => Some(KEY_VOLUMEUP),   // KEYCODE_VOLUME_UP    → KEY_VOLUMEUP (115)
        25 => Some(KEY_VOLUMEDOWN), // KEYCODE_VOLUME_DOWN  → KEY_VOLUMEDOWN (114)
        26 => Some(KEY_POWER),      // KEYCODE_POWER        → KEY_POWER (116)
        82 => Some(KEY_MENU),       // KEYCODE_MENU         → KEY_MENU (139)
        84 => Some(KEY_SEARCH),     // KEYCODE_SEARCH       → KEY_SEARCH (217)
        187 => Some(KEY_APPSELECT), // KEYCODE_APP_SWITCH   → KEY_APPSELECT (recents, 0x244)
        220 => Some(KEY_HOMEPAGE),  // KEYCODE_HOME (alt)   → KEY_HOMEPAGE (172)
        _ => None,                  // Unknown / unsupported keycode
    }
}

pub fn send_key_code(keycode: i32) {
    let linux_keycode = match android_keycode_to_linux(keycode) {
        Some(k) => k,
        None => {
            info!(
                "send_key_code: unmapped Android keycode {}, ignoring",
                keycode
            );
            return;
        }
    };

    if let Some(ref tx) = *KEY_SENDER.lock().unwrap_or_else(|e| e.into_inner()) {
        // Standard press → sync → release sequence. The guest's EventHub
        // reads this as a single key event from the virtual `vkey` device.
        input_event_write(tx, EV_KEY, linux_keycode, 1);
        input_event_write(tx, EV_SYN, SYN_REPORT, SYN_REPORT);
        input_event_write(tx, EV_KEY, linux_keycode, 0);
        input_event_write(tx, EV_SYN, SYN_REPORT, SYN_REPORT);
    }

    // 6-Z293c + 6-Z294/295: write the key records to BOTH bridge
    // devices. The loader-path recoveries never open the key0 device —
    // their ONLY input fds are the fb hook's bridge sockets. Only
    // touch-class fds enter minui's epoll (the keyboard branch closes
    // its fds), so the touch device is the one that can actually
    // deliver EV_KEY records to the UI's type-switch; the gpio-keys
    // mirror keeps the real-hardware topology and serves whichever
    // reader does consume it (v294c showed fd-9 reads). Both devices
    // advertise the menu keys in EVIOCGBIT(EV_KEY) since 6-Z295.
    let key_targets: [Option<Sender<TouchMessage>>; 2] = [
        KEY_BRIDGE_SENDER
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|(_, tx)| tx.clone()),
        INPUT_SENDER
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|tx| tx.clone()),
    ];
    for tx in key_targets.iter().flatten() {
        let _ = tx.send(TouchMessage {
            action: touch_action::KEY_DOWN,
            pointer_id: 0,
            x: linux_keycode,
            y: 0,
            pressure: 0,
        });
        let _ = tx.send(TouchMessage {
            action: touch_action::KEY_UP,
            pointer_id: 0,
            x: linux_keycode,
            y: 0,
            pressure: 0,
        });
        info!(
            "[INPUT] send_key_code: linux key {} also queued to the abstract evdev bridge",
            linux_keycode
        );
    }
}

fn key_server() {
    let device = generate_key_device();
    let key = key_path();

    // Make sure the parent directory exists (mirrors touch_server).
    if let Some(parent) = std::path::Path::new(&key).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let _ = std::fs::remove_file(&key);

    // Fixed: same panic-on-bind-failure issue as touch_server — see
    // the comment there for rationale.
    let listener = match unix_socket::UnixListener::bind(&key) {
        Ok(l) => {
            info!("[INPUT] key server listening at {}", key);
            l
        }
        Err(e) => {
            log::error!(
                "[INPUT] failed to bind key socket at {}: {} — \
                key input will be unavailable. Check permissions and path length.",
                key,
                e
            );
            return;
        }
    };

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                info!("key client connected!");

                let _ = stream.write_all(unsafe { any_as_u8_slice(&device) });

                let (tx, rx) = channel::<input_event>();
                *KEY_SENDER.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);

                thread::spawn(move || {
                    while let Ok(ev) = rx.recv() {
                        let data = unsafe { any_as_u8_slice(&ev) };
                        if stream.write_all(data).is_err() {
                            return; // write failed — client disconnected
                        }
                    }
                    // Channel disconnected — new client took over
                });
            }
            Err(e) => {
                // Transient accept errors must not permanently kill the
                // key server (a dead key server disables guest hardware
                // keys until app restart).
                info!("key server accept error: {} — continuing", e);
                thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}

// ============================================================================
// Unit tests for the TouchMessage IPC format.
//
// These tests mirror kr64's tests in `app/rs/kr64/src/lib.rs::tests`
// (commit `c67c498`) — same assertions, same byte-layout expectations.
// They guard against drift between the two crates' on-wire format: a
// size mismatch or field-order swap would silently misparse the IPC
// stream and break touch input entirely.
//
// `kr64` reads TouchMessages via `TouchMessage::parse(buf)`; the host
// writes them via `TouchMessage::to_bytes()`. The two MUST agree
// byte-for-byte — these tests verify they do.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a `TouchMessage` (avoids repeating the field list
    /// in every test below). Mirrors kr64's `touch_msg` helper.
    fn touch_msg(action: u32, pointer_id: i32, x: i32, y: i32, pressure: i32) -> TouchMessage {
        TouchMessage {
            action,
            pointer_id,
            x,
            y,
            pressure,
        }
    }

    /// `TOUCH_MESSAGE_SIZE` must be 20 bytes (4×5 little-endian fields).
    /// This is the SAME constant value as kr64's `TOUCH_MESSAGE_SIZE`
    /// (commit `c67c498`) — a size mismatch would silently misparse the
    /// IPC stream between the two crates.
    #[test]
    fn touch_message_size_is_20_bytes() {
        assert_eq!(TOUCH_MESSAGE_SIZE, 20);
    }

    /// `TouchMessage::to_bytes` roundtrips through `parse` byte-for-byte.
    /// Mirrors kr64's `touch_message_parse_roundtrip` test — the two
    /// crates MUST agree on the encoding.
    #[test]
    fn touch_message_to_bytes_parse_roundtrip() {
        let cases = [
            touch_msg(touch_action::DOWN, 0, 100, 200, 128),
            touch_msg(touch_action::MOVE, 1, 500, 750, 200),
            touch_msg(touch_action::UP, 4, 0, 0, 0),
            touch_msg(touch_action::CANCEL, 2, -10, -20, -30),
        ];
        for msg in cases {
            let bytes = msg.to_bytes();
            assert_eq!(bytes.len(), TOUCH_MESSAGE_SIZE);
            let parsed =
                TouchMessage::parse(&bytes).expect("parse should succeed for a 20-byte buffer");
            assert_eq!(parsed, msg, "roundtrip failed for {:?}", msg);
        }
    }

    /// `TouchMessage::parse` returns `None` for buffers shorter than
    /// `TOUCH_MESSAGE_SIZE`. Mirrors kr64's
    /// `touch_message_parse_rejects_short_buffer` test.
    #[test]
    fn touch_message_parse_rejects_short_buffer() {
        assert!(TouchMessage::parse(&[]).is_none());
        assert!(TouchMessage::parse(&[0u8; 19]).is_none());
        // Exactly 20 bytes parses OK.
        let msg = touch_msg(touch_action::DOWN, 0, 1, 2, 3);
        assert!(TouchMessage::parse(&msg.to_bytes()).is_some());
        // Extra bytes are accepted (parse reads only the first 20) —
        // matches kr64's behaviour. In practice `read_exact` always
        // produces exactly 20 bytes.
        let mut buf = msg.to_bytes().to_vec();
        buf.push(0xff);
        assert!(TouchMessage::parse(&buf).is_some());
    }

    // ── 6-Z293: negotiated evdev bridge ─────────────────────────────
    //
    // The guest's minui reads ONE sizeof(struct input_event) record per
    // read() and rejects anything else (lineage-22.2's ev_get_input:
    // read(fd, ev, 24); ret != 24 → fail). These tests pin the exact
    // record layout for both negotiated sizes.

    /// Decode the (type, code, value) of record `i` in an encoded
    /// evdev frame of `ev_size` bytes.
    fn evdev_rec(bytes: &[u8], i: usize, ev_size: usize) -> (u16, u16, i32) {
        let time_size = ev_size - 8;
        let b = &bytes[i * ev_size..(i + 1) * ev_size];
        assert!(
            b[0..time_size].iter().all(|&x| x == 0),
            "timeval must be zeroed"
        );
        (
            u16::from_le_bytes([b[time_size], b[time_size + 1]]),
            u16::from_le_bytes([b[time_size + 2], b[time_size + 3]]),
            i32::from_le_bytes([
                b[time_size + 4],
                b[time_size + 5],
                b[time_size + 6],
                b[time_size + 7],
            ]),
        )
    }

    /// aarch64 (24-byte) DOWN frame: 10 records in the documented
    /// order — MT slot/tracking-id, BTN_TOUCH + BTN_TOOL_FINGER press,
    /// MT position + pressure, legacy ABS_X/ABS_Y, SYN_REPORT.
    #[test]
    fn evdev_down_frame_24_byte_layout() {
        let d = encode_evdev_touch_down(24, 0, 7, 360, 769, 42).unwrap();
        assert_eq!(d.len(), 240);
        let expect: [(u16, u16, i32); 10] = [
            (EV_ABS as u16, ABS_MT_SLOT as u16, 0),
            (EV_ABS as u16, ABS_MT_TRACKING_ID as u16, 7),
            (EV_KEY as u16, BTN_TOUCH as u16, 1),
            (EV_KEY as u16, BTN_TOOL_FINGER as u16, 1),
            (EV_ABS as u16, ABS_MT_POSITION_X as u16, 360),
            (EV_ABS as u16, ABS_MT_POSITION_Y as u16, 769),
            (EV_ABS as u16, ABS_MT_PRESSURE as u16, 42),
            (EV_ABS as u16, ABS_X as u16, 360),
            (EV_ABS as u16, ABS_Y as u16, 769),
            (EV_SYN as u16, SYN_REPORT as u16, 0),
        ];
        for (i, e) in expect.iter().enumerate() {
            assert_eq!(evdev_rec(&d, i, 24), *e, "DOWN24 record {i}");
        }
    }

    /// arm32 (16-byte) MOVE frame: 7 records, type/code/value at
    /// offset 8..16 (2×i32 timeval).
    #[test]
    fn evdev_move_frame_16_byte_layout() {
        let m = encode_evdev_touch_move(16, 0, 150, 250, 200).unwrap();
        assert_eq!(m.len(), 112);
        let expect: [(u16, u16, i32); 7] = [
            (EV_ABS as u16, ABS_MT_SLOT as u16, 0),
            (EV_ABS as u16, ABS_MT_POSITION_X as u16, 150),
            (EV_ABS as u16, ABS_MT_POSITION_Y as u16, 250),
            (EV_ABS as u16, ABS_MT_PRESSURE as u16, 200),
            (EV_ABS as u16, ABS_X as u16, 150),
            (EV_ABS as u16, ABS_Y as u16, 250),
            (EV_SYN as u16, SYN_REPORT as u16, 0),
        ];
        for (i, e) in expect.iter().enumerate() {
            assert_eq!(evdev_rec(&m, i, 16), *e, "MOVE16 record {i}");
        }
    }

    /// RELEASE frame: BTN_TOUCH=0 / BTN_TOOL_FINGER=0 present (the
    /// stuck-press fix), tracking id −1, 5 records in both sizes.
    #[test]
    fn evdev_release_frame_layout() {
        for ev_size in [24usize, 16] {
            let r = encode_evdev_touch_release(ev_size, 0).unwrap();
            assert_eq!(r.len(), ev_size * 5);
            assert_eq!(
                evdev_rec(&r, 1, ev_size),
                (EV_ABS as u16, ABS_MT_TRACKING_ID as u16, -1)
            );
            assert_eq!(
                evdev_rec(&r, 2, ev_size),
                (EV_KEY as u16, BTN_TOUCH as u16, 0)
            );
            assert_eq!(
                evdev_rec(&r, 3, ev_size),
                (EV_KEY as u16, BTN_TOOL_FINGER as u16, 0)
            );
            assert_eq!(
                evdev_rec(&r, 4, ev_size),
                (EV_SYN as u16, SYN_REPORT as u16, 0)
            );
        }
    }

    /// Only 24 (aarch64) and 16 (arm32/i386) byte records are legal —
    /// anything else must be refused (the hello only advertises these).
    #[test]
    fn evdev_record_rejects_unknown_sizes() {
        assert!(encode_evdev_record(20, EV_SYN as u16, 0, 0).is_none());
        assert!(encode_evdev_record(0, EV_SYN as u16, 0, 0).is_none());
        assert!(encode_evdev_touch_down(20, 0, 1, 1, 1, 1).is_none());
        assert!(encode_evdev_touch_move(32, 0, 1, 1, 1).is_none());
        assert!(encode_evdev_touch_release(20, 0).is_none());
    }

    /// 6-Z293c: synthetic KEY records through the bridge —
    /// KEY_DOWN = [EV_KEY(code, 1), SYN_REPORT], KEY_UP = value 0,
    /// both at the negotiated record size (the KEY discriminator the
    /// probe rides: VOLUMEUP must move the menu highlight even if the
    /// guest's touch state machine never reacts to our touch frames).
    #[test]
    fn evdev_key_records_layout() {
        let mut finger: Option<(i32, i32)> = None;
        let k = encode_evdev(
            24,
            &mut finger,
            touch_msg(touch_action::KEY_DOWN, 0, 114, 0, 0),
        )
        .expect("KEY_DOWN must encode");
        assert_eq!(k.len(), 48); // 2 × 24
        assert_eq!(evdev_rec(&k, 0, 24), (EV_KEY as u16, 114, 1));
        assert_eq!(evdev_rec(&k, 1, 24), (EV_SYN as u16, SYN_REPORT as u16, 0));
        // keys must NOT disturb the finger state
        assert!(finger.is_none());

        let k = encode_evdev(
            16,
            &mut finger,
            touch_msg(touch_action::KEY_UP, 0, 114, 0, 0),
        )
        .expect("KEY_UP must encode");
        assert_eq!(k.len(), 32); // 2 × 16
        assert_eq!(evdev_rec(&k, 0, 16), (EV_KEY as u16, 114, 0));
        assert_eq!(evdev_rec(&k, 1, 16), (EV_SYN as u16, SYN_REPORT as u16, 0));
    }

    /// The full per-connection state machine: DOWN assigns a fresh
    /// tracking id and emits the DOWN frame; MOVE for the active slot
    /// encodes; UP emits exactly ONE release frame (handle_touch fans
    /// out a UP record per slot — the rest must be dropped); MOVE/UP
    /// without DOWN are dropped; a new DOWN re-arms.
    #[test]
    fn evdev_state_machine_single_finger() {
        let mut finger: Option<(i32, i32)> = None;

        // UP before any DOWN → dropped.
        assert!(encode_evdev(24, &mut finger, touch_msg(touch_action::UP, 0, 0, 0, 0)).is_none());
        assert!(finger.is_none());

        // DOWN → DOWN frame, finger armed with tracking id 1.
        let b = encode_evdev(
            24,
            &mut finger,
            touch_msg(touch_action::DOWN, 0, 360, 769, 42),
        )
        .expect("DOWN must encode");
        assert_eq!(b.len(), 240);
        assert_eq!(finger, Some((0, 1)));

        // MOVE for the active slot → MOVE frame.
        let b = encode_evdev(
            24,
            &mut finger,
            touch_msg(touch_action::MOVE, 0, 361, 770, 42),
        )
        .expect("MOVE must encode");
        assert_eq!(b.len(), 168); // 7 × 24
        assert_eq!(finger, Some((0, 1)));

        // UP fan-out: slot 0 releases; the other 9 slots are dropped.
        let mut releases = 0;
        for slot in 0..10 {
            if encode_evdev(24, &mut finger, touch_msg(touch_action::UP, slot, 0, 0, 0)).is_some() {
                releases += 1;
            }
        }
        assert_eq!(releases, 1, "exactly one release frame for the active slot");
        assert!(finger.is_none());

        // New DOWN gets a NEW tracking id.
        let b = encode_evdev(24, &mut finger, touch_msg(touch_action::DOWN, 2, 10, 20, 5))
            .expect("re-DOWN must encode");
        assert_eq!(b.len(), 240);
        assert_eq!(finger, Some((2, 2)));
        // CANCEL for the active slot releases too.
        assert!(
            encode_evdev(24, &mut finger, touch_msg(touch_action::CANCEL, 2, 0, 0, 0)).is_some()
        );
        assert!(finger.is_none());
    }

    /// Verify the on-wire byte layout of a `TouchMessage` (little-endian,
    /// fields at the documented offsets). This catches a struct-layout
    /// drift that would break inter-process IPC with kr64. Mirrors
    /// kr64's `touch_message_byte_layout` test EXACTLY — same offsets,
    /// same endianness, same field order.
    #[test]
    fn touch_message_byte_layout_matches_kr64() {
        let msg = touch_msg(touch_action::MOVE, 3, 0x12345678, -5, 255);
        let b = msg.to_bytes();
        assert_eq!(
            u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            touch_action::MOVE,
            "action at offset 0 (u32 LE)"
        );
        assert_eq!(
            i32::from_le_bytes([b[4], b[5], b[6], b[7]]),
            3,
            "pointer_id at offset 4 (i32 LE)"
        );
        assert_eq!(
            i32::from_le_bytes([b[8], b[9], b[10], b[11]]),
            0x12345678,
            "x at offset 8 (i32 LE)"
        );
        assert_eq!(
            i32::from_le_bytes([b[12], b[13], b[14], b[15]]),
            -5,
            "y at offset 12 (i32 LE)"
        );
        assert_eq!(
            i32::from_le_bytes([b[16], b[17], b[18], b[19]]),
            255,
            "pressure at offset 16 (i32 LE)"
        );
    }

    /// Verify `touch_action` constants match kr64's `touch_action`
    /// module (commit `c67c498`). A mismatch would cause kr64 to
    /// misinterpret every event's action field (e.g. treat DOWN as UP).
    #[test]
    fn touch_action_constants_match_kr64() {
        assert_eq!(touch_action::DOWN, 0);
        assert_eq!(touch_action::MOVE, 1);
        assert_eq!(touch_action::UP, 2);
        assert_eq!(touch_action::CANCEL, 3);
    }

    /// Cross-check: a TouchMessage encoded by this crate must be
    /// parseable by the same logic kr64 uses (which is identical —
    /// we copied `parse`/`to_bytes` verbatim from commit `c67c498`).
    /// This is the byte-level integration check — if either crate
    /// changes the layout, this test fails.
    #[test]
    fn touch_message_full_lifecycle_byte_stream() {
        // Simulate a DOWN → MOVE → UP lifecycle on slot 0, encoding
        // each as a TouchMessage and concatenating the bytes (as they
        // would appear on the IPC socket).
        let down = touch_msg(touch_action::DOWN, 0, 100, 200, 128);
        let mv = touch_msg(touch_action::MOVE, 0, 150, 250, 200);
        let up = touch_msg(touch_action::UP, 0, 0, 0, 0);

        let mut stream = Vec::new();
        stream.extend_from_slice(&down.to_bytes());
        stream.extend_from_slice(&mv.to_bytes());
        stream.extend_from_slice(&up.to_bytes());

        // 3 records × 20 bytes = 60 bytes total.
        assert_eq!(stream.len(), 3 * TOUCH_MESSAGE_SIZE);

        // Parse each 20-byte chunk and verify it roundtrips.
        for (i, expected) in [down, mv, up].iter().enumerate() {
            let start = i * TOUCH_MESSAGE_SIZE;
            let end = start + TOUCH_MESSAGE_SIZE;
            let chunk = &stream[start..end];
            let parsed = TouchMessage::parse(chunk).expect("chunk must parse");
            assert_eq!(&parsed, expected, "record {} mismatch", i);
        }
    }

    /// Verify `MAX_POINTERS` is 5 — it MUST match kr64's
    /// `devices::MAX_POINTERS` (commit `370b8ee`). A mismatch would
    /// cause `handle_touch`'s bounds check to either drop events kr64
    /// would accept, or send events kr64 would drop.
    #[test]
    fn max_pointers_matches_kr64() {
        assert_eq!(MAX_POINTERS, 5);
    }
}
