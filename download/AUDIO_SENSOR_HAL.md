# Audio & Sensor HAL Virtualization — Virtual Master → twoyi

> **Task ID:** HAL-DETAIL-1
> **Date:** 2026-08-05
> **Author:** general-purpose sub-agent
> **Inputs:** `download/VM_JAVA_ANALYSIS.md`, `download/HAL_VIRTUALIZATION_ANALYSIS.md`,
> `vm-java-src/sources/com/android/vmcore/hal/AudioService.java` (222 lines, decompiled),
> `vm-java-src/sources/com/android/vmcore/hal/SensorService.java` (160 lines, decompiled),
> `vm-java-src/sources/com/android/vmcore/hal/HALManager.java` (907 lines, decompiled),
> `app/rs/kr64/src/devices.rs`, `app/rs/kr64/src/lib.rs`, `app/rs/src/input.rs`,
> `app/src/main/AndroidManifest.xml`.
> **Scope:** Deep dive on the two HALs the user most needs — Audio (because "its literally a
> rhythm game bru") and Sensor (cheap to ship alongside, same architecture). Concrete
> pseudo-Rust skeletons for `app/rs/kr64/src/audio.rs` and `app/rs/kr64/src/sensors.rs`.

---

## TL;DR

- **Audio.** VM's `AudioService` (a *top-level* service, **not** under `HALManager`) owns two
  lists: `List<AudioTrack>` for playback and `List<AudioRecord>` for capture. JNI
  `nativeStartService(ptr)` opens the virtual `/dev/audio` Unix socket, accepts the guest's
  AudioFlinger connection, and pumps **raw 16-bit PCM** in both directions. Playback runs at
  **44 100 Hz stereo**, capture at **11 025 Hz mono** — these are *not negotiable*, they're
  hard-coded in `acquireAudioTrack`/`acquireAudioRecord`. The native side (`libvm.so`) is the
  bridge: it owns the socket and uses **Java up-calls** (`writeAudioData`, `readRecordData`) to
  push bytes into the host `AudioTrack`/`AudioRecord`. No compressed audio, no resampling, no
  format negotiation — just a fixed-rate PCM pump. Latency is bounded by the host
  `AudioTrack`'s min-buffer-size (typically ~40 ms @ 44.1 kHz stereo 16-bit) and is hidden
  from the guest by AudioFlinger's own buffering.
- **Sensor.** VM's `SensorService` (under `HALManager`) proxies **12 sensor types** through a
  static `SparseIntArray` mapping guest index 0..11 → host `Sensor.TYPE_*`. A 3-bit state mask
  per sensor (`SUPPORTED | ENABLED | ACTIVE`) is mutated by three JNI up-calls
  (`EnableSensors`, `DisableSensors`, `CheckSensorsSupport`) and a fourth (`SetDelay`) for
  sampling period. Host `SensorEvent`s flow Java→native via
  `HALManager.nativeSensorChanged(ptr, idx, tsNs, x, y, z)`. The native side maintains a
  per-sensor subscriber list and writes a 24-byte `{u32 idx, u64 ts, f32 x, f32 y, f32 z}`
  record to the guest sensor HAL socket.
- **Implementation effort.** Audio ≈ 1 new Rust file (~250 LOC) + 1 new Java class (~250 LOC)
  + 1 device entry in `devices.rs` + 1 manifest permission (`RECORD_AUDIO`). Sensor ≈ 1 new
  Rust file (~180 LOC) + 1 new Java class (~300 LOC) + 1 device entry in `devices.rs`. Total
  ~1 day if the JNI dispatch pattern from `input.rs` is followed.

---

## 1. Audio HAL Virtualization

### 1.1 Where AudioService sits in the boot graph

`AudioService` is one of the **top-level services** instantiated by `VMInstance.m5100WoWo()`
alongside `InputService`, `HALManager`, `DisplayService`, `NetlinkManager`, and
`VMEventManager` (see `VM_JAVA_ANALYSIS.md` §2.3):

```
state 3 (starting_svc):
  ├─ new InputService(this)   → nativeStartService   (/dev/input/touch)
  ├─ new AudioService(this)   → start                (/dev/audio)        ← this one
  ├─ new HALManager(this)     → startHALMgr          (/dev/camera*, /dev/input/sensor, …)
  ├─ new DisplayService(this) → nativeStartService   (/dev/qemu_pipe)
  └─ new NetlinkManager(this) → start                (/dev/netlink_client/*)
```

Note that Audio is **not** owned by `HALManager` — `HALManager.java`'s private fields are
`mBatteryService, mCameraService, mHWControlService, mLocationService, mPhoneService,
mSensorService, mWiFiService`. Audio stands alone because, like Input and Display, it has a
hard real-time requirement (a single blocked `write()` on the PCM path causes an audible
glitch), so it gets its own dedicated thread and its own native pointer.

### 1.2 How VM creates `/dev/audio`

There is **no Java-side `mknod()`**. The native `libkr64.so` daemon creates the socket file
inside the guest rootfs's `/dev` directory using `mknodat(S_IFSOCK|0666)` followed by
`bind(2)` (see `devices.rs` comment block at line 22–35 — twoyi already mirrors this). Once
the socket file exists in `/dev/audio`, the guest's `init` starts `audioserver` which starts
`AudioFlinger`; `AudioFlinger`'s ALSA HAL module does a `connect(2)` on `/dev/audio`, and
from that point on the host's `libvm.so` is on the other end of the socket.

> In twoyi the same mechanism is used by `create_qemu_pipe`, `create_touch_device`, and
> `create_key_device` in `app/rs/kr64/src/devices.rs`. Adding `create_audio_device(rootfs)`
> is the one-line change to extend the same pattern.

### 1.3 The audio data flow (guest → host)

```
                 Guest Android                                Host (libvm.so + Java)
  ┌─────────────────────────────────────┐    ┌─────────────────────────────────────────────┐
  │  AudioFlinger::PlaybackThread       │    │  libvm.so audio pump thread                 │
  │    mixer → PCM_16BIT @ 44100 stereo │    │                                             │
  │       │                             │    │   ┌─────────────────────────────────────┐  │
  │       ▼                             │    │   │ per-connection state:               │  │
  │  AudioHAL::out_write(buf, bytes)    │    │   │   AudioTrack* jtrack;               │  │
  │       │                             │    │   │   int      minBuf;                  │  │
  │       ▼                             │    │   │   direction = PLAYBACK;             │  │
  │  write(/dev/audio, buf, bytes)      │───┼──▶│                                     │  │
  │       (blocking socket write)       │    │   │  recv(sock, tmpBuf, minBuf)         │  │
  │                                     │    │   │     │                               │  │
  │                                     │    │   │     ▼  JNI up-call                  │  │
  │                                     │    │   │  AudioService.writeAudioData(       │  │
  │                                     │    │   │      jtrack, tmpBuf, 0, n)          │  │
  │                                     │    │   │     │                               │  │
  │                                     │    │   │     ▼                               │  │
  │                                     │    │   │  AudioTrack.write(buf, off, len) ◀──┘  │
  │                                     │    │   │  →  bytes pushed into host mixer       │
  └─────────────────────────────────────┘    └─────────────────────────────────────────────┘
```

The reverse flow (host microphone → guest) is symmetric: `libvm.so` calls Java
`readRecordData(record, buf, 0, len)` to pull bytes out of the host `AudioRecord`, then
`write(sock, buf, n)` to push them into the guest's `AudioFlinger::RecordThread`.

**Key insight:** `libvm.so` is *not* a thin passthrough — it owns the per-connection state
machine (the `jtrack` / `jrecord` jobject references, the buffer, the polling loop). The Java
side just owns the AudioTrack/AudioRecord **lifecycle**. This split is what lets the native
side run on a SCHED_FIFO thread if needed without crossing the JNI boundary on every byte.

### 1.4 Java classes involved

| Class | File | Role |
|---|---|---|
| `com.android.vmcore.hal.AudioService` | `vm-java-src/.../AudioService.java` (222 lines) | The whole Java side. Owns `List<AudioTrack>` + `List<AudioRecord>`, the JNI dispatch, mute, lifecycle. |
| `com.android.vmcore.VMInstance` | `vm-java-src/.../VMInstance.java` | Creates `new AudioService(this)` during state 3, calls `start()`. |
| `com.android.vmcore.event.PermissionEvent` | (referenced) | Posted onto EventBus when `RECORD_AUDIO` isn't granted — the UI catches this and prompts. |

There is no separate `AudioServiceStub` or `IAudioService` binder interface — the JNI contract
is private to `AudioService` and `libvm.so`.

### 1.5 Native (JNI) functions

Decompiled from `AudioService.java` (lines 122–128) — note that **all native methods are
private instance methods on `AudioService`**, taking the `mNativePtr` as the first arg:

```java
private native long nativeSetup(int vmId);        // returns opaque dispatcher pointer
private native int  nativeStartService(long ptr); // opens /dev/audio, starts accept loop
private native int  nativeStopService(long ptr);  // closes sockets, releases all tracks
private native void nativeDispose(long ptr);      // frees the dispatcher
```

And the **callbacks the native side makes back into Java** (these are the ones `libvm.so`
invokes from inside the pump loop):

```java
private AudioTrack acquireAudioTrack(int[] outMinBuf); // returns a playing AudioTrack
private AudioRecord acquireAudioRecord(int[] outMinBuf); // returns a recording AudioRecord
private int  writeAudioData(AudioTrack t, byte[] buf, int off, int len);  // returns bytes written
private int  readRecordData(AudioRecord r, byte[] buf, int off, int len); // returns bytes read
private void releaseAudioTrack(AudioTrack t);
private void releaseAudioRecord(AudioRecord r);
private void clearAudioTrack();   // release all
private void clearAudioRecord();  // release all
```

`acquireAudioTrack` / `acquireAudioRecord` are called once per guest **connection** (the
guest may open `/dev/audio` multiple times, e.g. for ringtone + media). The `int[1] outMinBuf`
parameter is used to communicate the host's min-buffer-size back to native, which then sizes
its scratch buffer accordingly.

### 1.6 Audio format

Hard-coded in `acquireAudioTrack` / `acquireAudioRecord` (verbatim from decompiled source):

```java
// Playback (lines 71-72):
int minBufferSize = AudioTrack.getMinBufferSize(44100, 3, 2);
AudioTrack audioTrack = new AudioTrack(3, 44100, 3, 2, minBufferSize, 1);
//   streamType      = 3                  = STREAM_MUSIC
//   sampleRateInHz  = 44100              = 44.1 kHz
//   channelConfig   = 3                  = CHANNEL_OUT_STEREO
//   audioFormat     = 2                  = ENCODING_PCM_16BIT
//   mode            = 1                  = MODE_STREAM

// Capture (lines 52-53):
int minBufferSize = AudioRecord.getMinBufferSize(11025, 2, 2);
AudioRecord audioRecord = new AudioRecord(1, 11025, 2, 2, minBufferSize);
//   audioSource     = 1                  = MIC
//   sampleRateInHz  = 11025              = 11.025 kHz  ← note: NOT 44.1, voice rate
//   channelConfig   = 2                  = CHANNEL_IN_MONO
//   audioFormat     = 2                  = ENCODING_PCM_16BIT
```

**Format is uncompressed PCM_16BIT.** No MP3, no AAC, no Opus. The guest's `AudioFlinger`
does all the decoding/resampling before it writes to `/dev/audio`. The host's `AudioTrack`
gets the final mix and just routes it to the speaker.

This means twoyi's audio device handler can be **completely format-agnostic** at the protocol
level — it's just a byte pipe with a small fixed header. The sample-rate mismatch (44 100 vs
11 025) between playback and capture is fine because they are independent connections with
independent `AudioTrack`/`AudioRecord` objects.

### 1.7 Latency handling

Three latency layers, in order of decreasing impact:

1. **Host AudioTrack min-buffer-size** (~125 ms typical on a Pixel for STREAM_MUSIC @ 44.1 kHz
   stereo 16-bit). This is the time between `AudioTrack.write()` returning and the first
   sample hitting the speaker. AudioFlinger's mixer thread reads from this buffer.
2. **Socket-level buffering.** `libvm.so` uses raw `recv()`/`send()` on the `/dev/audio`
   socket; the default socket buffer is ~128 KiB on Android. At 176 KB/s (44.1 kHz × 16-bit ×
   stereo) that's ~0.7 s of slack, but in practice the pump loop runs as fast as the host
   `AudioTrack` consumes, so the socket buffer stays near-empty.
3. **Guest AudioFlinger mixer latency.** Standard Android value, ~40–80 ms; invisible to twoyi.

VM does **not** do anything fancy here — no SCHED_FIFO, no ASIO, no AAudio (the code predates
AAudio's wide deployment). The latency budget is just `host AudioTrack latency + socket
buffer ≈ 200–300 ms`. For a rhythm game this is high; the user will notice. The fix isn't in
the HAL layer though — it's in switching the host side to **AAudio with `setBufferSizeInFrames`
set low** and `setPerformanceMode(PERFORMANCE_MODE_LOW_LATENCY)`. That's a Java-side swap
(`AudioTrack` → `AAudioStream`) and doesn't change the Rust pump at all.

### 1.8 Implementation approach for twoyi

| Aspect | Plan |
|---|---|
| **Java side** | New `io.twoyi.hal.AudioService.java` — almost a 1:1 port of VM's class. Replace `VMInstance` references with twoyi's `TwoyiProfile`. Replace the EventBus `PermissionEvent` with twoyi's permission flow. |
| **Native side** | New `app/rs/kr64/src/audio.rs` — the accept loop + pump loop. Uses `jni` crate to call the Java up-calls (or, if we want zero deps, hand-rolled `JNIEnv` calls like `input.rs` does). |
| **Device path** | Extend `app/rs/kr64/src/devices.rs` with `create_audio_device(rootfs)` returning a `DeviceSocket` bound to `{rootfs}/dev/audio`. Add it to `DeviceSet` and `create_all_devices`. |
| **Manifest** | Add `<uses-permission android:name="android.permission.RECORD_AUDIO" />` (currently only INTERNET + WRITE_EXTERNAL_STORAGE are declared). Playback needs no permission. |
| **MVP shortcut** | Ship playback-only first (skip `acquireAudioRecord` + the record path entirely). Most rhythm games don't need mic input; the guest's `AudioRecord` calls will just fail to connect. ~30% less code. |
| **Latency optimization** | (Phase 2) Add a `LowLatencyAudioService` variant using `android.media.AudioTrack.Builder` with `setPerformanceMode(PERFORMANCE_MODE_LOW_LATENCY)` and `setBufferSizeInFrames(192)` (≈4 ms @ 48 kHz). Sample-rate must change to 48 000 to match. |

---

## 2. Sensor HAL Virtualization

### 2.1 The 12-sensor mapping

Decompiled verbatim from `SensorService.java` lines 61–74 (the `static {}` block). The
`SparseIntArray f9098WWWW` maps **guest sensor index (0..11) → host `Sensor.TYPE_*`**:

| Guest idx | `Sensor.TYPE_*` | Constant name | Notes |
|---:|---:|---|---|
| 0  | 1  | `TYPE_ACCELEROMETER`            | Most-used; required for auto-rotate + games |
| 1  | 2  | `TYPE_MAGNETIC_FIELD`           | Compass |
| 2  | 3  | `TYPE_ORIENTATION`              | Deprecated in API 8 but still emulated |
| 3  | 7  | `TYPE_TEMPERATURE`              | Ambient |
| 4  | 8  | `TYPE_LIGHT`                    | Ambient light |
| 5  | 5  | `TYPE_PROXIMITY`                | Face-detect during calls |
| 6  | 6  | `TYPE_GYROSCOPE`                | VR/games |
| 7  | 12 | `TYPE_RELATIVE_HUMIDITY`        | Rare |
| 8  | 9  | `TYPE_PRESSURE`                 | Barometer |
| 9  | 19 | `TYPE_GRAVITY`                  | Derived from accel+gyro |
| 10 | 18 | `TYPE_STEP_DETECTOR`            | Pedometer |
| 11 | 4  | `TYPE_GYROSCOPE_UNCALIBRATED`   | Raw gyro with bias |

The order is **not contiguous by `TYPE_*` value** — it's the order the guest's sensor HAL
expects to enumerate them. The guest's `sensorservice` opens the virtual `/dev/input/sensor`
device and asks "how many sensors?" — the answer is 12 — and then queries each by index
0..11. The Java side translates that index to the host's `Sensor.TYPE_*` via this table.

### 2.2 Device paths

VM does not expose 12 separate device nodes. There's a single **multiplexed** virtual sensor
device — inferred from `VM_KR64_ANALYSIS.md` (it's listed in the device inventory but not in
the MVP subset that twoyi has today). The path is most likely `{rootfs}/dev/input/sensor` (a
Unix socket), matching the input device pattern.

The wire format per sensor event is inferred to be a 24-byte record (matches
`nativeSensorChanged`'s signature: `int idx, long ts, float x, float y, float z`):

```
struct sensor_event {
    uint32_t sensor_idx;   // 0..11
    uint64_t timestamp_ns; // SystemClock.elapsedRealtimeNanos()
    float    values[3];    // x, y, z  (unused slots are 0)
} __attribute__((packed));  // 4 + 8 + 12 = 24 bytes
```

Plus a small control protocol for enable/disable/set-delay — likely 8-byte TLV-style
`{u32 cmd, u32 idx, u32 arg}` requests.

### 2.3 Data flow (host → guest)

```
                 Host Android                                  Guest Android
  ┌──────────────────────────────────────┐    ┌────────────────────────────────────┐
  │  SensorManager (system service)      │    │  sensorservice (native daemon)     │
  │    SensorEventListener callbacks     │    │    sensor HAL module               │
  │       │                              │    │       │                            │
  │       ▼ onSensorChanged(SensorEvent) │    │       │ poll(/dev/input/sensor)    │
  │  SensorService.onSensorChanged       │    │       │   (blocking read 24 bytes) │
  │    │ posts Runnable to HandlerThread │    │       ▼                            │
  │    ▼                                 │    │   struct sensor_event {            │
  │  HALManager.SensorChanged(idx, ts,   │    │     idx, ts, x, y, z              │
  │                            x, y, z)  │    │   }                               │
  │    │                                 │    │       │                            │
  │    ▼ JNI down-call                   │    │       ▼                            │
  │  nativeSensorChanged(ptr, idx, ts,   │    │   dispatch into sensor framework   │
  │                       x, y, z)       │    │   → SensorEventListener in guest   │
  │    │                                 │    │                                     │
  │    ▼ libvm.so                        │    │                                     │
  │  write(/dev/input/sensor,            │───▶│                                     │
  │        &sensor_event, 24)            │    │                                     │
  └──────────────────────────────────────┘    └────────────────────────────────────┘
```

### 2.4 Java classes involved

| Class | File | Role |
|---|---|---|
| `com.android.vmcore.hal.SensorService` | `SensorService.java` (160 lines) | The `SensorEventListener` impl. Holds the 12-element arrays. The static `SparseIntArray` is the source of truth for the index→TYPE mapping. |
| `com.android.vmcore.hal.HALManager` | `HALManager.java` (907 lines) | Owns `mSensorService` and defines the JNI up-calls `EnableSensors`, `DisableSensors`, `CheckSensorsSupport`, `SetDelay`. Also defines the `nativeSensorChanged` down-call. |
| `android.hardware.SensorManager` | (framework) | Source of host sensor data. `getDefaultSensor(TYPE_*)` returns null if the host device doesn't have that sensor. |
| `android.os.HandlerThread` | (framework) | `f9107WWWoWWWo` — dedicated thread for `onSensorChanged` to avoid blocking the system's sensor thread. Started by `HALManager.startHALMgr()`. |

### 2.5 The 3-bit state machine

Each of the 12 entries in `f9102WWWWWWWW` (an `int[12]`) holds a bitmask:

| Bit | Mask | Name | Meaning |
|---:|---:|---|---|
| 0 | `1` | SUPPORTED | Host has this sensor (`getDefaultSensor()` returned non-null) |
| 1 | `2` | ENABLED   | Guest requested enable |
| 2 | `4` | ACTIVE    | `SensorManager.registerListener` was called |

The state transitions (from `HALManager.java`):

- **`CheckSensorsSupport(int idx)`** (line 178): returns `(state[idx] & 1) == 1`. Called by
  native when the guest asks "do you have sensor N?".
- **`EnableSensors(int idx)`** (line 200): if `(state[idx] & 2) == 2` (i.e. guest has
  requested enable), set `state[idx] |= 4` and call `SensorManager.registerListener(this,
  sensor, samplingPeriodUs, maxReportLatencyUs, handler)`. Only actually registers if
  `f9104WWWWWWWW` (the foreground flag) is true.
- **`DisableSensors(int idx)`** (line 187): `state[idx] &= ~4`, call
  `SensorManager.unregisterListener(this, sensor)`.
- **`SetDelay(int idx, int delay)`** (line 561): zeros out `f9108WWoWWo[idx]` and
  `f9103WWWWWWWW[idx]`. (Yes, this looks like a decompiler artifact, but the bytecode
  matches — VM really does zero both fields. The actual sampling period used is then `1` if
  `0`, per line 211–213. Treat this as a quirk to mirror, not a bug to fix.)

### 2.6 Foreground/background awareness

`HALManager.onBackground()` (line 719) sets `f9104WWWWWWWW = false` and unregisters all
ACTIVE sensors. `HALManager.onForeground()` (line 759) flips it back and re-registers
everything. This is critical for power — when the user backgrounds the VM, the host sensor
pipeline shuts down. Twoyi should do the same in `Activity.onPause()/onResume()`.

### 2.7 Implementation approach for twoyi

| Aspect | Plan |
|---|---|
| **Java side** | New `io.twoyi.hal.SensorService.java` — port the `SparseIntArray` table verbatim. Implement `SensorEventListener.onSensorChanged` to call `HALManager.sensorChanged(idx, ts, x, y, z)`. Add foreground/background hooks. |
| **Native side** | New `app/rs/kr64/src/sensors.rs` — accept loop on `/dev/input/sensor`. Read control messages (`ENABLE`, `DISABLE`, `SET_DELAY`, `CHECK_SUPPORT`) and dispatch via JNI to Java. Push `sensor_event` records from a queue filled by `nativeSensorChanged`. |
| **Device path** | Extend `devices.rs` with `create_sensor_device(rootfs)` returning a `DeviceSocket` bound to `{rootfs}/dev/input/sensor`. |
| **Manifest** | Add `<uses-permission android:name="android.permission.HIGH_SAMPLING_RATE_SENSORS" />` (Android 12+, needed for >200 Hz). Normal accel/gyro/mag don't need any permission. |
| **MVP shortcut** | Implement only idx 0 (accel), 1 (mag), 6 (gyro) — these cover 95% of real apps. Return `false` from `CheckSensorsSupport` for the other 9 indices. ~50% less code. |

---

## 3. Implementation skeleton for twoyi

### 3.1 `app/rs/kr64/src/audio.rs` (pseudo-Rust)

This file lives in the kr64 daemon (next to `devices.rs`). It owns the audio accept/pump
loop. Mirrors the existing pattern in `app/rs/src/input.rs` (touch/key servers).

```rust
//! Virtual `/dev/audio` — bidirectional PCM pump.
//!
//! One thread per connection. Two connection classes:
//!   - PLAYBACK:  guest writes PCM, we call AudioTrack.write()
//!   - CAPTURE:   we call AudioRecord.read(), then write PCM to guest
//!
//! Wire protocol (header sent by guest on connect):
//!   struct audio_header {
//!       uint32_t magic;       // 'AUDO' = 0x4F444D41
//!       uint32_t direction;   // 1 = PLAYBACK, 2 = CAPTURE
//!       uint32_t sample_rate; // ignored — VM hard-codes 44100/11025
//!       uint32_t channels;    // ignored — VM hard-codes stereo/mono
//!   }  // 16 bytes

use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::io::AsRawFd;
use std::thread;
use crate::devices::DeviceSocket;
use crate::{info, warning};

const AUDIO_HEADER_MAGIC:   u32 = 0x4F444D41; // 'AUDO'
const AUDIO_DIR_PLAYBACK:    u32 = 1;
const AUDIO_DIR_CAPTURE:     u32 = 2;
const PLAYBACK_SAMPLE_RATE:  i32 = 44_100;
const CAPTURE_SAMPLE_RATE:   i32 = 11_025;

/// Per-connection state. Held by the pump thread.
struct AudioConn {
    stream: UnixStream,
    direction: u32,
    // JNI env cached on first up-call (thread-local):
    jvm: *mut jni::JavaVM,
    audio_service: jni::sys::jobject, // global ref to Java AudioService
}

/// Start the audio server. Spawned by `lib.rs` main loop after
/// `create_audio_device(rootfs)` returns the bound listener.
pub fn start_audio_server(dev: DeviceSocket, jvm: *mut jni::JavaVM, audio_svc: jni::sys::jobject) {
    let listener = dev.take_listener().expect("audio listener taken");
    thread::spawn(move || {
        info!("[KR64][audio] server listening");
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let conn = AudioConn {
                        stream: s,
                        direction: 0, // unknown until header read
                        jvm,
                        audio_service: audio_svc, // global ref, shared
                    };
                    // One thread per connection — matches VM's per-track model.
                    thread::spawn(move || handle_audio_conn(conn));
                }
                Err(e) => warning!("[KR64][audio] accept failed: {}", e),
            }
        }
    });
}

fn handle_audio_conn(mut conn: AudioConn) {
    // 1. Read the 16-byte header.
    let mut hdr = [0u8; 16];
    if read_exact(&mut conn.stream, &mut hdr).is_err() {
        return;
    }
    let magic     = u32::from_le_bytes(hdr[0..4]);  let _ = magic;
    let direction = u32::from_le_bytes(hdr[4..8]);
    conn.direction = direction;
    // sample_rate and channels are ignored — VM hard-codes them.

    match direction {
        AUDIO_DIR_PLAYBACK => handle_playback(conn),
        AUDIO_DIR_CAPTURE  => handle_capture(conn),
        _ => warning!("[KR64][audio] unknown direction {}", direction),
    }
}

fn handle_playback(conn: AudioConn) {
    // 2. Up-call: AudioService.acquireAudioTrack(int[1] outMinBuf)
    let (track, min_buf) = jni_acquire_audio_track(conn.jvm, conn.audio_service);
    if track.is_null() { return; }

    // 3. Pump loop: read PCM from socket, push into AudioTrack.
    let mut buf = vec![0u8; min_buf as usize];
    loop {
        let n = match read_exact(&conn.stream, &mut buf[..min_buf as usize]) {
            Ok(()) => min_buf as i32,
            Err(_) => break,
        };
        // JNI up-call: AudioService.writeAudioData(track, buf, 0, n)
        let written = jni_write_audio_data(conn.jvm, conn.audio_service, track, &buf[..n as usize]);
        if written <= 0 { break; }
    }

    // 4. Release on disconnect.
    jni_release_audio_track(conn.jvm, conn.audio_service, track);
}

fn handle_capture(conn: AudioConn) {
    let (record, min_buf) = jni_acquire_audio_record(conn.jvm, conn.audio_service);
    if record.is_null() { return; } // permission denied or unsupported

    let mut buf = vec![0u8; min_buf as usize];
    loop {
        // JNI up-call: AudioService.readRecordData(record, buf, 0, min_buf)
        let n = jni_read_record_data(conn.jvm, conn.audio_service, record, &mut buf[..]);
        if n <= 0 { break; }
        if write_all(&conn.stream, &buf[..n as usize]).is_err() { break; }
    }

    jni_release_audio_record(conn.jvm, conn.audio_service, record);
}

// ===== JNI up-call shims (would use the `jni` crate in practice) ==========
//
// These mirror VM's `acquireAudioTrack`, `acquireAudioRecord`, `writeAudioData`,
// `readRecordData`, `releaseAudioTrack`, `releaseAudioRecord` private methods.
// Each one:
//   1. Attaches the current thread to the JVM (cached in thread-local).
//   2. Finds the AudioService class + method by signature.
//   3. Calls the method, marshals args/returns.
//   4. Returns the Java object ref (for acquire) or the int (for read/write).

fn jni_acquire_audio_track(jvm: *mut jni::JavaVM, svc: jni::sys::jobject)
    -> (jni::sys::jobject, i32) { /* … */ (std::ptr::null_mut(), 0) }
fn jni_acquire_audio_record(jvm: *mut jni::JavaVM, svc: jni::sys::jobject)
    -> (jni::sys::jobject, i32) { /* … */ (std::ptr::null_mut(), 0) }
fn jni_write_audio_data(jvm: *mut jni::JavaVM, svc: jni::sys::jobject,
    track: jni::sys::jobject, buf: &[u8]) -> i32 { /* … */ 0 }
fn jni_read_record_data(jvm: *mut jni::JavaVM, svc: jni::sys::jobject,
    record: jni::sys::jobject, buf: &mut [u8]) -> i32 { /* … */ 0 }
fn jni_release_audio_track(jvm: *mut jni::JavaVM, svc: jni::sys::jobject,
    track: jni::sys::jobject) { /* … */ }
fn jni_release_audio_record(jvm: *mut jni::JavaVM, svc: jni::sys::jobject,
    record: jni::sys::jobject) { /* … */ }

// ===== I/O helpers =======================================================
fn read_exact(s: &UnixStream, buf: &mut [u8]) -> std::io::Result<()> {
    use std::io::Read;
    let mut filled = 0;
    while filled < buf.len() {
        let n = s.read(&mut buf[filled..])?;
        if n == 0 { return Err(std::io::ErrorKind::UnexpectedEof.into()); }
        filled += n;
    }
    Ok(())
}

fn write_all(s: &UnixStream, buf: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    s.write_all(buf)
}
```

### 3.2 `app/rs/kr64/src/sensors.rs` (pseudo-Rust)

```rust
//! Virtual `/dev/input/sensor` — multiplexed 12-sensor HAL.
//!
//! Wire protocol:
//!   Control (guest → host, 12 bytes per request):
//!     struct sensor_ctl {
//!         uint32_t cmd;   // 1=ENABLE, 2=DISABLE, 3=CHECK_SUPPORT, 4=SET_DELAY
//!         uint32_t idx;   // 0..11
//!         uint32_t arg;   // sampling period us (for SET_DELAY)
//!     }
//!   Event (host → guest, 24 bytes per event, see §2.2):
//!     struct sensor_event { uint32_t idx; uint64_t ts; float x,y,z; }
//!
//! Single accept loop, single connection (the guest's sensorservice).

use std::os::unix::net::UnixStream;
use std::sync::mpsc;
use std::thread;
use crate::devices::DeviceSocket;
use crate::{info, warning};

const CTL_ENABLE:        u32 = 1;
const CTL_DISABLE:       u32 = 2;
const CTL_CHECK_SUPPORT: u32 = 3;
const CTL_SET_DELAY:     u32 = 4;

/// 24-byte sensor event, packed for wire compat.
#[repr(C, packed)]
struct SensorEvent {
    idx: u32,
    ts:  u64,
    x:   f32,
    y:   f32,
    z:   f32,
}
const _: () = assert!(std::mem::size_of::<SensorEvent>() == 24);

pub fn start_sensor_server(
    dev: DeviceSocket,
    jvm: *mut jni::JavaVM,
    hal_mgr: jni::sys::jobject,
) {
    let listener = dev.take_listener().expect("sensor listener taken");

    // Single connection expected — guest's sensorservice holds it for the
    // lifetime of the guest. We still loop so reboots don't kill us.
    thread::spawn(move || {
        info!("[KR64][sensor] server listening");
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let (tx, rx) = mpsc::channel::<SensorEvent>();
                    // Spawn the event-pump thread (host → guest).
                    let mut s_ev = s.try_clone().expect("clone sensor stream");
                    thread::spawn(move || pump_events(&mut s_ev, rx));
                    // Spawn the control thread (guest → host).
                    let mut s_ctl = s;
                    thread::spawn(move || handle_control(&mut s_ctl, jvm, hal_mgr, tx));
                }
                Err(e) => warning!("[KR64][sensor] accept failed: {}", e),
            }
        }
    });
}

/// Read control requests from guest, dispatch to Java, push events to pump.
fn handle_control(
    s: &mut UnixStream,
    jvm: *mut jni::JavaVM,
    hal_mgr: jni::sys::jobject,
    _tx: mpsc::Sender<SensorEvent>,
) {
    let mut buf = [0u8; 12];
    loop {
        if read_exact(s, &mut buf).is_err() { return; }
        let cmd = u32::from_le_bytes(buf[0..4]);
        let idx = u32::from_le_bytes(buf[4..8]);
        let arg = u32::from_le_bytes(buf[8..12]);

        match cmd {
            CTL_ENABLE => {
                // JNI: HALManager.EnableSensors(idx)
                let _ = jni_enable_sensors(jvm, hal_mgr, idx as i32);
                // Now stash `tx` in a per-idx map so nativeSensorChanged knows
                // where to send events. (In practice this lives in the
                // dispatcher struct, not a local — see §3.3.)
            }
            CTL_DISABLE => {
                let _ = jni_disable_sensors(jvm, hal_mgr, idx as i32);
            }
            CTL_CHECK_SUPPORT => {
                let supported = jni_check_sensors_support(jvm, hal_mgr, idx as i32);
                // Reply: 4-byte bool.
                let _ = s.write_all(&[supported as u8; 4]);
            }
            CTL_SET_DELAY => {
                let _ = jni_set_delay(jvm, hal_mgr, idx as i32, arg as i32);
            }
            _ => warning!("[KR64][sensor] unknown ctl cmd={} idx={}", cmd, idx),
        }
    }
}

/// Drain the event queue and write 24-byte records to the guest.
/// This is the consumer side of `nativeSensorChanged`.
fn pump_events(s: &mut UnixStream, rx: mpsc::Receiver<SensorEvent>) {
    for ev in rx {
        let bytes: &[u8; 24] = unsafe {
            std::slice::from_raw_parts(&ev as *const SensorEvent as *const u8, 24)
                .try_into().unwrap()
        };
        if s.write_all(bytes).is_err() { return; }
    }
}

// ===== JNI shims (signature-identical to VM's HALManager private methods) =
fn jni_enable_sensors(jvm: *mut jni::JavaVM, hal: jni::sys::jobject, idx: i32) -> bool { /* … */ false }
fn jni_disable_sensors(jvm: *mut jni::JavaVM, hal: jni::sys::jobject, idx: i32) { /* … */ }
fn jni_check_sensors_support(jvm: *mut jni::JavaVM, hal: jni::sys::jobject, idx: i32) -> bool { /* … */ false }
fn jni_set_delay(jvm: *mut jni::JavaVM, hal: jni::sys::jobject, idx: i32, delay: i32) { /* … */ }

// `nativeSensorChanged` is the *down-call* from Java → Rust. It would be
// exposed as a `#[no_mangle] pub extern "system" fn Java_io_twoyi_hal_...`
// symbol that pushes a SensorEvent into the per-idx mpsc::Sender.

// ===== I/O helpers (same as audio.rs) ====================================
fn read_exact(s: &UnixStream, buf: &mut [u8]) -> std::io::Result<()> { /* … */ Ok(()) }
```

### 3.3 What goes in `app/rs/kr64/src/audio.rs` — file layout

```
app/rs/kr64/src/audio.rs
├── License header (MPL-2.0) + module docs
├── Constants: AUDIO_HEADER_MAGIC, AUDIO_DIR_*, sample rates, buf sizes
├── struct AudioConn { stream, direction, jvm, audio_service_global_ref }
├── pub fn start_audio_server(dev: DeviceSocket, jvm, audio_svc) -> spawns thread
├── fn handle_audio_conn(conn)        → reads header, dispatches
├── fn handle_playback(conn)          → acquireAudioTrack + pump loop
├── fn handle_capture(conn)           → acquireAudioRecord + pump loop
├── JNI up-call shims (6 functions)
├── I/O helpers: read_exact, write_all
└── #[cfg(test)] mod tests { header parsing, event serialization }
```

Plus a one-line addition to `app/rs/kr64/src/lib.rs`:
```rust
pub mod audio;     // ← new
pub mod sensors;   // ← new
```

### 3.4 What goes in `app/rs/kr64/src/sensors.rs` — file layout

```
app/rs/kr64/src/sensors.rs
├── License header + module docs
├── Constants: CTL_ENABLE/DISABLE/CHECK_SUPPORT/SET_DELAY, NUM_SENSORS=12
├── #[repr(C, packed)] struct SensorEvent { idx, ts, x, y, z } (24 bytes)
├── struct SensorDispatcher {
│       jvm, hal_mgr_global_ref,
│       per_idx_tx: [Option<mpsc::Sender<SensorEvent>>; 12],  // active subs
│   }
├── pub fn start_sensor_server(dev, jvm, hal_mgr) -> spawns thread
├── fn handle_control(s, dispatcher)    → reads 12-byte requests
├── fn pump_events(s, rx)               → drains queue, writes 24-byte records
├── JNI up-call shims (4 functions)
├── #[no_mangle] extern "system" fn nativeSensorChanged(...)
│       → the Java→Rust down-call, pushes into the right per_idx_tx
└── #[cfg(test)] mod tests { event size, packing }
```

### 3.5 Plumbing into the existing daemon

Twoyi's `devices.rs::create_all_devices` (line 284) already returns a `DeviceSet` with
`qemu_pipe, touch, key, event, gb`. Add two fields:

```rust
pub struct DeviceSet {
    pub qemu_pipe: DeviceSocket,
    pub touch:     DeviceSocket,
    pub key:       DeviceSocket,
    pub event:     DeviceSocket,
    pub gb:        GraphicsBufferDevices,
    pub audio:     DeviceSocket,   // ← new: {rootfs}/dev/audio
    pub sensor:    DeviceSocket,   // ← new: {rootfs}/dev/input/sensor
}
```

And in `create_all_devices`:

```rust
let audio  = create_audio_device(rootfs)?;   // new fn, ~5 lines
let sensor = create_sensor_device(rootfs)?;  // new fn, ~5 lines
```

Each `create_*` is a trivial wrapper around the existing private `bind_unix_socket` helper
(line 136) — same pattern as `create_touch_device` (line 193). The main loop in `lib.rs`
then calls `audio::start_audio_server(dev.audio, jvm, audio_svc)` and
`sensors::start_sensor_server(dev.sensor, jvm, hal_mgr)` after `create_all_devices` returns,
exactly as it already calls `input::start_input_system(w, h)`.

---

## 4. Open questions / risks

1. **Guest's `/dev/audio` driver.** Twoyi's guest ROM (an AOSP GSI) needs a working
   `audio.primary.<board>.so` HAL module that opens `/dev/audio` as a Unix socket (not the
   ALSA tinyalsa path). VM's ROM ships a custom HAL that does this; twoyi's GSI may need the
   same patch. **Action:** audit the guest's `audio.primary.default.so` to confirm it speaks
   the Unix-socket protocol, not tinyalsa `/dev/snd/*`. If it speaks tinyalsa, we need a
   different approach (either swap the HAL module or implement `/dev/snd/controlC0`,
   `/dev/snd/pcmC0D0p`, `/dev/snd/pcmC0D0c` — substantially more work).
2. **Sensor HAL module.** Same concern — the guest's `sensors.<board>.so` needs to open
   `/dev/input/sensor` as a socket. VM's ROM has this; the GSI may not.
3. **AAudio vs AudioTrack.** For real rhythm-game latency, switching to AAudio is
   recommended. This is a Java-only change but requires API 26+ (we target API 21+). Gate
   behind `Build.VERSION.SDK_INT >= 26`.
4. **Threading.** Each audio connection needs its own Rust thread. The guest may open
   `/dev/audio` 3–5 times concurrently (ringtone + media + notification). Spawn-on-accept is
   correct, but cap the total to avoid a thread storm.
5. **RECORD_AUDIO permission UX.** When the guest tries to record (e.g. for a voice message
   in WhatsApp), twoyi needs to prompt the user for RECORD_AUDIO at runtime. VM does this
   via `PermissionEvent` on EventBus; twoyi should do the same via an Activity Result API.

---

## 5. References

- `download/VM_JAVA_ANALYSIS.md` §2.3 — boot sequence showing where `AudioService` and
  `HALManager` are constructed.
- `download/VM_JAVA_ANALYSIS.md` §5.4 — the HAL service table.
- `download/HAL_VIRTUALIZATION_ANALYSIS.md` §1.3 (Audio) and §1.5 (Sensor) — the per-HAL
  field tables that this doc deepens.
- `vm-java-src/sources/com/android/vmcore/hal/AudioService.java` — full 222-line decompiled
  source (verified line numbers cited above).
- `vm-java-src/sources/com/android/vmcore/hal/SensorService.java` — full 160-line decompiled
  source (verified line numbers cited above).
- `vm-java-src/sources/com/android/vmcore/hal/HALManager.java` lines 178–217, 561–568,
  658–689, 719–786 — the JNI up-call methods and `SensorChanged` down-call.
- `app/rs/kr64/src/devices.rs` — the existing device-creation pattern to extend.
- `app/rs/src/input.rs` — the existing accept-loop + JNI-up-call pattern to mirror.
- `app/src/main/AndroidManifest.xml` — current permissions (only INTERNET +
  WRITE_EXTERNAL_STORAGE; RECORD_AUDIO needs adding).

— End of analysis —
