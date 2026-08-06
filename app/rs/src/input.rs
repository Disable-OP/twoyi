// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use libc::*;
use libc::{c_char, c_int};
use ndk::event::{MotionAction, MotionEvent};
use std::mem;
use std::thread;
use std::{io::Write};
use uinput_sys::*;

use std::sync::mpsc::{ channel, Sender};
use std::sync::Mutex;
use once_cell::sync::Lazy;

use log::info;

const FF_MAX: u16 = 0x7f;

const TOUCH_DEVICE_NAME: &'static str = "vtouch";
const TOUCH_DEVICE_UNIQUE_ID: &'static str = "<vtouch 0>";

const KEY_DEVICE_NAME: &'static str = "vkey";
const KEY_DEVICE_UNIQUE_ID: &'static str = "<keyboard 0>";

/// Touch device socket path — now dynamic via core::get_touch_path().
/// In a work profile, the data dir is /data/user/<uid>/io.twoyi instead
/// of /data/data/io.twoyi, so the path must be resolved at runtime.
fn touch_path() -> String {
    crate::core::get_touch_path()
}

/// Key device socket path — now dynamic via core::get_key_path().
fn key_path() -> String {
    crate::core::get_key_path()
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
fn copy_to_cstr<T, const COUNT: usize>(data: &str, arr: &mut [T; COUNT]) {
    let cstr = std::ffi::CString::new(data).expect("create cstring failed");
    let bytes = cstr.as_bytes_with_nul();
    let mut len = bytes.len();
    if len >= COUNT {
        len = COUNT;
    }
    // Cast the [T; COUNT] array to a [u8; COUNT] pointer for the copy.
    // This is sound because:
    //   - On aarch64-linux-android, c_char == i8, and [i8; N] has the same
    //     memory layout as [u8; N] (both are 1-byte elements, N elements).
    //   - On targets where c_char == u8, this is a no-op cast.
    //   - We only write `len` <= COUNT bytes, so we never overflow.
    unsafe {
        let ptr = arr.as_mut_ptr() as *mut u8;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, len);
    }
}

const MAX_POINTERS: usize = 5;

static INPUT_SENDER: Lazy<Mutex<Option<Sender<input_event>>>> = Lazy::new(|| { Mutex::new(None)});
static KEY_SENDER: Lazy<Mutex<Option<Sender<input_event>>>> = Lazy::new(|| { Mutex::new(None)});

pub fn start_input_system(width: i32, height: i32) {
    thread::spawn(move || {
        touch_server(width, height);
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
    let mut tp = libc::timespec { tv_sec:0, tv_nsec: 0 };
    let _ = unsafe { clock_gettime(CLOCK_MONOTONIC, &mut tp) };
    let tv = timeval {
        tv_sec: tp.tv_sec,
        tv_usec: tp.tv_nsec / 1000
    };

    let ev = input_event {
        kind: kind as u16,
        code: code as u16,
        value: val,
        time: tv,
    };
    let _ = tx.send(ev);
}

pub fn handle_touch(ev: MotionEvent) {
    let opt = INPUT_SENDER.lock().unwrap();
    if let Some(ref fd) = *opt {

        let action = ev.action();
        let pointer_index = ev.pointer_index();
        let pointer = ev.pointer_at_index(pointer_index);
        let pointer_id = pointer.pointer_id();
        let pressure = pointer.pressure();

        // Bounds check: pointer_id must be < MAX_POINTERS to use as array index.
        // Without this, a malformed MotionEvent with pointer_id >= 5 panics,
        // which poisons the mutex and kills ALL subsequent touch input.
        if (pointer_id as usize) >= MAX_POINTERS {
            return;
        }

        // info!("action: {:#?}, pointer_index: {}", action, pointer_index);

        static G_INPUT_MT: Lazy<Mutex<[i32;MAX_POINTERS]>> = Lazy::new(|| {std::sync::Mutex::new([0i32;MAX_POINTERS])});

        match action {
            // -----------------------------------------------------------------
            // Linux Type B multitouch protocol (multi-touch with tracking IDs).
            //
            // The kernel maintains a per-slot state. On a new touch we:
            //   1. Select the new slot (ABS_MT_SLOT = pointer_id)
            //   2. Assign it a fresh, nonzero tracking ID (ABS_MT_TRACKING_ID)
            //   3. Write its position/pressure
            //   4. If this is the FIRST active touch, also press BTN_TOUCH and
            //      BTN_TOOL_FINGER so the guest's EventHub reports a "tool"
            //      (finger) on screen — Android's InputReader requires this.
            //   5. SYN_REPORT to commit the frame.
            //
            // BUGFIX: the original loop iterated ALL active slots and wrote the
            // NEW pointer's data (slot id, tracking id, x, y, pressure) for
            // every one of them. That collapsed all concurrent touches to a
            // single finger — placing a second finger on screen overwrote the
            // first finger's slot with the second finger's coordinates.
            //
            // BUGFIX: BTN_TOUCH / BTN_TOOL_FINGER were emitted with value 108
            // instead of 1. EV_KEY values are 0 (release) / 1 (press) / 2
            // (repeat); 108 is not a valid key-event value. It worked by
            // accident because the kernel treats any nonzero value as "press",
            // but the wrong value is wrong.
            // -----------------------------------------------------------------
            MotionAction::Down | MotionAction::PointerDown => {
                let x = pointer.x();
                let y = pointer.y();

                let mut mt = G_INPUT_MT.lock().unwrap();

                // Was the MT state empty before this touch? If so, this is
                // the first finger and we need to press BTN_TOUCH/BTN_TOOL_FINGER.
                let was_empty = mt.iter().all(|&v| v == 0);

                mt[pointer_id as usize] = 1;

                // Only emit events for the NEW touch. Re-emitting other slots
                // would overwrite their tracked state with the new pointer's
                // coordinates.
                input_event_write(fd, EV_ABS, ABS_MT_SLOT, pointer_id);
                input_event_write(fd, EV_ABS, ABS_MT_TRACKING_ID, pointer_id + 1);

                if was_empty {
                    input_event_write(fd, EV_KEY, BTN_TOUCH, 1);
                    input_event_write(fd, EV_KEY, BTN_TOOL_FINGER, 1);
                }

                input_event_write(fd, EV_ABS, ABS_MT_POSITION_X, x as i32);
                input_event_write(fd, EV_ABS, ABS_MT_POSITION_Y, y as i32);
                input_event_write(fd, EV_ABS, ABS_MT_PRESSURE, pressure as i32);

                input_event_write(fd, EV_SYN, SYN_REPORT, SYN_REPORT);
            }
            // ACTION_UP: the LAST remaining pointer has been released.
            // Release every still-active slot (defensive — handles missed
            // PointerUp events) and then release BTN_TOUCH/BTN_TOOL_FINGER
            // so the guest sees the tool leave the screen.
            //
            // BUGFIX: the original code never sent BTN_TOUCH=0 / BTN_TOOL_FINGER=0,
            // so after the last finger lifted the guest's InputReader still
            // believed a tool was on screen, causing stuck-press states.
            MotionAction::Up => {
                let mut mt = G_INPUT_MT.lock().unwrap();
                for slot in 0..MAX_POINTERS {
                    if mt[slot] != 0 {
                        mt[slot] = 0;
                        input_event_write(fd, EV_ABS, ABS_MT_SLOT, slot as i32);
                        input_event_write(fd, EV_ABS, ABS_MT_TRACKING_ID, -1);
                    }
                }
                // All touches released — release the tool keys.
                input_event_write(fd, EV_KEY, BTN_TOUCH, 0);
                input_event_write(fd, EV_KEY, BTN_TOOL_FINGER, 0);
                input_event_write(fd, EV_SYN, SYN_REPORT, SYN_REPORT);
            }
            MotionAction::Move => {
                // For each active slot, re-emit its position. NOTE: the
                // MotionEvent's pointer_at_index(pointer_index) only gives us
                // ONE pointer's coordinates — for true multi-pointer MOVE we'd
                // need to iterate all pointers via ev.pointer_count(). That's
                // a follow-up; this preserves the existing single-pointer-move
                // behaviour without the try_into().unwrap() panic risk.
                let mt = G_INPUT_MT.lock().unwrap();
                for slot in 0..MAX_POINTERS {
                    if mt[slot] != 0 {
                        let x = pointer.x();
                        let y = pointer.y();
                        let pressure = pointer.pressure();

                        input_event_write(fd, EV_ABS, ABS_MT_SLOT, slot as i32);
                        input_event_write(fd, EV_ABS, ABS_MT_POSITION_X, x as i32);
                        input_event_write(fd, EV_ABS, ABS_MT_POSITION_Y, y as i32);
                        input_event_write(fd, EV_ABS, ABS_MT_PRESSURE, pressure as i32);

                        input_event_write(fd, EV_SYN, SYN_REPORT, SYN_REPORT);
                    }
                }
            }
            // ACTION_POINTER_UP / ACTION_CANCEL: a single (non-last) pointer
            // went up. Release just that slot, and release BTN_TOUCH/
            // BTN_TOOL_FINGER if no touches remain.
            //
            // BUGFIX: same BTN_TOUCH=0 omission as ACTION_UP — fixed here too.
            MotionAction::Cancel | MotionAction::PointerUp => {
                let mut mt = G_INPUT_MT.lock().unwrap();
                if mt[pointer_id as usize] == 0 {
                    return;
                }

                mt[pointer_id as usize] = 0;
                input_event_write(fd, EV_ABS, ABS_MT_SLOT, pointer_id);
                input_event_write(fd, EV_ABS, ABS_MT_TRACKING_ID, -1);

                // If no more touches are active, release the tool keys.
                if mt.iter().all(|&v| v == 0) {
                    input_event_write(fd, EV_KEY, BTN_TOUCH, 0);
                    input_event_write(fd, EV_KEY, BTN_TOOL_FINGER, 0);
                }

                input_event_write(fd, EV_SYN, SYN_REPORT, SYN_REPORT);
            }
            _ => {}
        }
    }
}

fn generate_touch_device(width: i32, height: i32) -> device_info {
    let iid = input_id {
        product: 0x1,
        version: 0,
        vendor: 0,
        bustype: 0,
    };

    let mut info = device_info {
        name: unsafe { mem::zeroed() },
        driver_version: 0x1,
        id: iid,
        physical_location: unsafe { mem::zeroed() },
        unique_id: unsafe { mem::zeroed() },
        key_bitmask: unsafe { mem::zeroed() },
        abs_bitmask: unsafe { mem::zeroed() },
        rel_bitmask: unsafe { mem::zeroed() },
        sw_bitmask: unsafe { mem::zeroed() },
        led_bitmask: unsafe { mem::zeroed() },
        ff_bitmask: unsafe { mem::zeroed() },
        prop_bitmask: unsafe { mem::zeroed() },
        abs_max: unsafe { mem::zeroed() },
        abs_min: unsafe { mem::zeroed() },
    };

    copy_to_cstr(TOUCH_DEVICE_NAME, &mut info.name);
    copy_to_cstr(&touch_path(), &mut info.physical_location);
    copy_to_cstr(TOUCH_DEVICE_UNIQUE_ID, &mut info.unique_id);

    info.prop_bitmask[0] = INPUT_PROP_BUTTONPAD as u8;

    // Set multitouch ABS axis bits using proper bitmap indexing
    // (byte = axis/8, bit = axis%8) — same pattern as set_key_bit
    set_key_bit(&mut info.abs_bitmask, ABS_MT_SLOT as usize);
    set_key_bit(&mut info.abs_bitmask, ABS_MT_POSITION_X as usize);
    set_key_bit(&mut info.abs_bitmask, ABS_MT_POSITION_Y as usize);
    set_key_bit(&mut info.abs_bitmask, ABS_MT_PRESSURE as usize);
    set_key_bit(&mut info.abs_bitmask, ABS_MT_TRACKING_ID as usize);
    set_key_bit(&mut info.abs_bitmask, ABS_MT_TOUCH_MAJOR as usize);

    info.abs_min[ABS_MT_POSITION_X as usize] = 0;
    info.abs_max[ABS_MT_POSITION_X as usize] = width as u32;

    info.abs_min[ABS_MT_POSITION_Y as usize] = 0;
    info.abs_max[ABS_MT_POSITION_Y as usize] = height as u32;

    info.abs_min[ABS_MT_TOUCH_MAJOR as usize] = 0;
    info.abs_min[ABS_MT_TOUCH_MINOR as usize] = 15;

    // Fixed: min/max were inverted (min=4, max=0). This rejected all slots.
    // Now: min=0, max=MAX_POINTERS-1 (supports slots 0..4 for 5 fingers)
    info.abs_min[ABS_MT_SLOT as usize] = 0;
    info.abs_max[ABS_MT_SLOT as usize] = (MAX_POINTERS as u32) - 1;
    info.abs_min[ABS_MT_PRESSURE as usize] = 0;
    info.abs_max[ABS_MT_PRESSURE as usize] = 80;

    info
}

fn touch_server(width: i32, height: i32) {
    let device = generate_touch_device(width, height);
    let touch = touch_path();
    let _ = std::fs::remove_file(&touch);
    let listener = unix_socket::UnixListener::bind(&touch).unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                info!("touch client connected!");

                let _ = stream.write_all(unsafe { any_as_u8_slice(&device) });

                let (tx, rx) = channel::<input_event>();
                *INPUT_SENDER.lock().unwrap() = Some(tx);

                thread::spawn(move || loop {
                    match rx.recv() {
                        Ok(ev) => {
                            let data = unsafe { any_as_u8_slice(&ev) };
                            if stream.write_all(data).is_err() {
                                break; // write failed — client disconnected
                            }
                        }
                        Err(_) => {
                            // Channel disconnected — new client took over
                            break;
                        }
                    }
                });
            }
            Err(_) => {
                info!("touch server error happened!");
                break;
            }
        }
    }

    info!("drop listener!");
}

/// Set the bit at position `n` in a `key_bitmask` byte array.
/// `key_bitmask` is a bitmap of `KEY_MAX` bits, indexed by Linux keycode.
/// The guest's `EventHub` reads this via `EVIOCGKEY` to learn which keys
/// the virtual keyboard can emit.
fn set_key_bit(bitmask: &mut [u8], keycode: usize) {
    let byte = keycode / 8;
    let bit  = keycode % 8;
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
        KEY_HOME,         // 102  — KEYCODE_HOME
        KEY_BACK,         // 158  — KEYCODE_BACK
        KEY_END,          // 107  — KEYCODE_ENDCALL
        KEY_VOLUMEUP,     // 115  — KEYCODE_VOLUME_UP
        KEY_VOLUMEDOWN,   // 114  — KEYCODE_VOLUME_DOWN
        KEY_POWER,        // 116  — KEYCODE_POWER
        KEY_MENU,         // 139  — KEYCODE_MENU
        KEY_SEARCH,       // 217  — KEYCODE_SEARCH
        KEY_APPSELECT,    // 0x244 — KEYCODE_APP_SWITCH (recents)
        KEY_HOMEPAGE,     // 172  — KEYCODE_HOME alternate
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
        3   => Some(KEY_HOME),         // KEYCODE_HOME         → KEY_HOME (102)
        4   => Some(KEY_BACK),         // KEYCODE_BACK         → KEY_BACK (158)
        6   => Some(KEY_END),          // KEYCODE_ENDCALL      → KEY_END (107)
        24  => Some(KEY_VOLUMEUP),     // KEYCODE_VOLUME_UP    → KEY_VOLUMEUP (115)
        25  => Some(KEY_VOLUMEDOWN),   // KEYCODE_VOLUME_DOWN  → KEY_VOLUMEDOWN (114)
        26  => Some(KEY_POWER),        // KEYCODE_POWER        → KEY_POWER (116)
        82  => Some(KEY_MENU),         // KEYCODE_MENU         → KEY_MENU (139)
        84  => Some(KEY_SEARCH),       // KEYCODE_SEARCH       → KEY_SEARCH (217)
        187 => Some(KEY_APPSELECT),    // KEYCODE_APP_SWITCH   → KEY_APPSELECT (recents, 0x244)
        220 => Some(KEY_HOMEPAGE),     // KEYCODE_HOME (alt)   → KEY_HOMEPAGE (172)
        _   => None,                   // Unknown / unsupported keycode
    }
}

pub fn send_key_code(keycode: i32) {
    let linux_keycode = match android_keycode_to_linux(keycode) {
        Some(k) => k,
        None    => {
            info!("send_key_code: unmapped Android keycode {}, ignoring", keycode);
            return;
        }
    };

    if let Some(ref tx) = *KEY_SENDER.lock().unwrap() {
        // Standard press → sync → release sequence. The guest's EventHub
        // reads this as a single key event from the virtual `vkey` device.
        input_event_write(tx, EV_KEY, linux_keycode, 1);
        input_event_write(tx, EV_SYN, SYN_REPORT, SYN_REPORT);
        input_event_write(tx, EV_KEY, linux_keycode, 0);
        input_event_write(tx, EV_SYN, SYN_REPORT, SYN_REPORT);
    }
}

fn key_server() {
    let device = generate_key_device();
    let key = key_path();
    let _ = std::fs::remove_file(&key);
    let listener = unix_socket::UnixListener::bind(&key).unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                info!("key client connected!");

                let _ = stream.write_all(unsafe { any_as_u8_slice(&device) });

                let (tx, rx) = channel::<input_event>();
                *KEY_SENDER.lock().unwrap() = Some(tx);

                thread::spawn(move || loop {
                    match rx.recv() {
                        Ok(ev) => {
                            let data = unsafe { any_as_u8_slice(&ev) };
                            if stream.write_all(data).is_err() {
                                break; // write failed — client disconnected
                            }
                        }
                        Err(_) => {
                            // Channel disconnected — new client took over
                            break;
                        }
                    }
                });
            }
            Err(_) => {
                info!("key server error happened!");
                break;
            }
        }
    }
}
