# Sensor HAL Implementation — kr64 Skeleton

> **Task ID:** SENSOR-IMPL-1
> **Date:** 2026-08-05
> **Author:** general-purpose sub-agent
> **Inputs:** `download/AUDIO_SENSOR_HAL.md` (the HAL-DETAIL-1 analysis),
> `app/rs/kr64/src/audio.rs` (the AUDIO-IMPL-1 sister module),
> `app/rs/kr64/src/devices.rs`, `app/rs/kr64/src/binder.rs`,
> `app/rs/kr64/src/lib.rs`.
> **Scope:** Implement the sensor HAL skeleton in the kr64 crate as a
> real (compiling, tested) Rust module, mirroring the design documented
> in `AUDIO_SENSOR_HAL.md` §3.2 and the patterns established by
> `audio.rs` (AUDIO-IMPL-1).

---

## TL;DR

- Added `app/rs/kr64/src/sensors.rs` (~2 290 LOC including ~430 LOC of
  rustdoc and ~640 LOC of tests): the virtual `/dev/sensors` Unix
  socket, a 12-byte control protocol (`ENABLE`/`DISABLE`/
  `CHECK_SUPPORT`/`SET_DELAY`), a 24-byte `SensorEvent` wire format,
  a 12-variant `SensorType` enum matching VM's `SparseIntArray`
  mapping verbatim, a 3-bit `SensorState` bitflags type
  (`SUPPORTED`/`ENABLED`/`ACTIVE`), an accept thread + fixed-size
  worker pool + per-connection pump sub-thread, and five JNI up-call
  stubs.
- Updated `app/rs/kr64/src/lib.rs`: registered the new `sensors`
  module and wired `sensors::create_sensor_device(&cfg.rootfs)?.spawn()`
  into the daemon startup sequence (right after the audio device
  setup at Step 2.6, before `/proc` population at Step 3). Non-fatal
  on failure — the guest can boot without sensors (the guest's
  `SensorManager.getDefaultSensor()` will return null and apps that
  hard-require a sensor will crash, but the boot proceeds).
- The crate compiles clean with `cargo build` and `cargo build
  --all-targets` (0 warnings) and all 125 unit tests pass with
  `cargo test --lib` (38 pre-existing + 27 audio + 60 new sensor
  tests, runtime 1.04 s).
- **No JNI is wired up yet** — the five up-call functions are no-op
  stubs returning `false`/`None`/`()`. The control loop replies `0`
  to every `CHECK_SUPPORT`, the pump never produces events, and
  `ENABLE` is a no-op. This is the deliberate "skeleton" boundary
  mirroring AUDIO-IMPL-1: the protocol layer is complete and tested;
  the host `SensorManager` integration is the next task
  (`SENSOR-IMPL-2`).
- No external crates added. The crate still depends on only `libc` +
  `std`. The thread pool is the same MPMC-via-`mpsc::channel`
  pattern already used in `audio.rs` and `binder.rs` (kept private
  to each module so they're self-contained — a future REFACTOR-1
  could lift it to a shared `thread_pool.rs`).

---

## 1. What was implemented

### 1.1 New file: `app/rs/kr64/src/sensors.rs`

The file is organised into nine sections, each documented in the
module-level rustdoc:

1. **Constants** — `NUM_SENSORS` (12), the four control command codes
   (`CTL_ENABLE` = 1, `CTL_DISABLE` = 2, `CTL_CHECK_SUPPORT` = 3,
   `CTL_SET_DELAY` = 4), `SENSOR_CONTROL_SIZE` (12), `SENSOR_EVENT_SIZE`
   (24), `SENSOR_THREAD_POOL_SIZE` (4), the pump sleep clamps
   (`MIN_POLL_NS` = 1 ms, `MAX_POLL_NS` = 1 s, `SENSOR_IDLE_POLL_MS`
   = 50 ms).

2. **`SensorType` enum** — 12 variants with `#[repr(u32)]` so `as u32`
   gives the exact `android.hardware.Sensor.TYPE_*` constant. Variants
   named after the `TYPE_*` suffix (e.g. `Accelerometer` = 1,
   `MagneticField` = 2, `GyroscopeUncalibrated` = 4). Provides
   `from_type_value(u32) -> Option<Self>` and `type_value() -> u32`.

3. **`SENSOR_TYPE_MAP` constant** — a `[SensorType; 12]` array that
   is the source of truth for the guest-index → host-type mapping.
   Verbatim from `SensorService.java`'s `static {}` block (see
   `AUDIO_SENSOR_HAL.md` §2.1). Plus `index_to_type(u32) ->
   Option<SensorType>` and `type_to_index(SensorType) ->
   Option<u32>` accessors.

4. **`SensorEvent` struct** — `#[repr(C, packed)]`, 24 bytes on the
   wire (compile-time asserted via
   `const _: () = assert!(size_of::<SensorEvent>() == 24);`).
   Fields: `idx: u32`, `ts: u64`, `x/y/z: f32`. Provides `new`,
   `zero`, `is_valid`, `to_bytes` (→ `[u8; 24]` LE), `from_bytes`
   (validates buffer length + idx range). Includes the
   `SensorEventError` enum (`TooShort`, `BadIndex`) with `Display` +
   `std::error::Error` impls.

5. **`SensorControl` struct** — `#[repr(C)]`, 12 bytes (all `u32`
   fields, no padding). Fields: `cmd: u32`, `idx: u32`, `arg: u32`.
   Provides `new`, `to_bytes`, `from_bytes`. Includes the
   `SensorControlError` struct with `Display` +
   `std::error::Error` impls.

6. **`SensorState` bitflags** — hand-rolled (no `bitflags` crate,
   since the project is std + libc only). `#[repr(transparent)]`
   newtype around `u8`. Constants `SUPPORTED` (1), `ENABLED` (2),
   `ACTIVE` (4), `ALL` (7). Methods: `empty`, `is_empty`, `bits`,
   `from_bits` (masks to low 3 bits), `contains`, `insert`, `remove`,
   `is_supported`, `is_enabled`, `is_active`. Trait impls: `BitOr`,
   `BitOrAssign`, `BitAnd`, `BitAndAssign`, `Not`.

7. **`create_sensor_device(rootfs)`** — creates `{rootfs}/dev/sensors`
   as a `UnixListener` (mirrors `audio::create_audio_device` /
   `binder::create_binder_device`). Creates parent dirs, removes
   stale socket files, binds, and `chmod 0666`s the socket so the
   guest can `connect()`. Returns a `SensorDevice` that owns the
   listener.

8. **`SensorDevice` + `SensorDeviceHandle`** — `SensorDevice::spawn(mut
   self)` consumes the device, makes the listener non-blocking, and
   spawns the accept thread (named `kr64-sensor-accept`). The accept
   thread owns a `ThreadPool` of 4 workers (named `kr64-sensor-worker`)
   and dispatches each accepted connection to a worker via
   `pool.execute(move || handle_connection(stream))`. The returned
   `SensorDeviceHandle` holds the shutdown `Arc<AtomicBool>` + accept
   thread `JoinHandle`; its `Drop` sets the flag, joins the thread, and
   unlinks the socket file. Mirrors `AudioDevice` / `AudioDeviceHandle`
   exactly.

9. **Per-connection handler** — `handle_connection` clones the stream
   via `UnixStream::try_clone()`, creates an `Arc<SensorConnState>`,
   spawns a `kr64-sensor-pump` sub-thread (which runs `pump_events`),
   then runs `handle_control` in the worker thread itself. When the
   control read returns EOF, the worker signals shutdown, joins the
   pump sub-thread, and returns.
   - **`handle_control`** — reads 12-byte control messages in a loop,
     dispatches by `cmd`: `CHECK_SUPPORT` queries the JNI stub (cached
     in `SensorConnState`) and writes a 4-byte LE u32 reply; `ENABLE`
     calls the JNI stub and conditionally marks the sensor enabled;
     `DISABLE` calls the JNI stub and marks it disabled; `SET_DELAY`
     calls the JNI stub and updates the per-sensor sampling period.
     Out-of-range `idx` and unknown `cmd` are logged and skipped.
   - **`pump_events`** — the event pump. Snapshots the enabled-sensor
     list, polls each enabled sensor via `jni_read_sensor_event`, and
     writes any returned `SensorEvent` (24 bytes) to the guest. Sleeps
     for the shortest delay among enabled sensors (clamped to
     `MIN_POLL_NS`..`MAX_POLL_NS`) between iterations. When no sensors
     are enabled, sleeps for `SENSOR_IDLE_POLL_MS` (50 ms) and
     re-checks. Exits when the connection's shutdown flag is set.

10. **JNI up-call stubs** — `jni_check_sensor_support`, `jni_enable_sensor`,
    `jni_disable_sensor`, `jni_set_sensor_delay`, `jni_read_sensor_event`.
    Each is a one-line no-op returning `false`/`false`/`()`/`()`/`None`.
    They're documented with the exact Java signature they'll need to
    invoke (`HALManager.CheckSensorsSupport(I)Z`, etc.) so the
    follow-up task can fill them in without re-reading the analysis
    doc. The `JniObject` type alias is `*mut c_void` so the skeleton
    doesn't need the `jni` crate.

11. **`ThreadPool`** — same pattern as `audio.rs::ThreadPool` /
    `binder.rs::ThreadPool`: `mpsc::channel` + `Arc<Mutex<Receiver>>`
    + `Worker` struct that loops on `recv()` and dispatches
    `Message::Job` / `Message::Terminate`. Kept private to `sensors`
    so the three modules are independent.

12. **`read_exact` helper** — blocks until `buf.len()` bytes are read
    or the peer closes (returns `UnexpectedEof`). Used for the
    12-byte control message read.

### 1.2 Wire protocol documentation

The module's rustdoc opens with a full specification of:
- The 12-sensor mapping table (verbatim from `SensorService.java`'s
  `static {}` block, with `Sensor.TYPE_*` values and notes).
- The 12-byte control message layout (offset table + field
  descriptions for `cmd`/`idx`/`arg`).
- The 24-byte sensor event layout (offset table for
  `idx`/`ts`/`x`/`y`/`z`).
- The host→guest data-flow ASCII diagram (host `SensorManager` →
  `onSensorChanged` → `nativeSensorChanged` JNI down-call →
  `write(/dev/sensors, &event, 24)` → guest sensor HAL).
- The 3-bit state machine (`SUPPORTED`/`ENABLED`/`ACTIVE` + the four
  state transitions from `HALManager.java`).
- The JNI callback table (5 stubs ↔ 5 Java methods).
- The threading model (1 accept thread + 4-worker pool + 1 pump
  sub-thread per connection).

### 1.3 `lib.rs` integration

Three changes to `app/rs/kr64/src/lib.rs`:

1. **Module declaration** — added `pub mod sensors;` (after
   `pub mod audio;`, before `pub mod seccomp;`).
2. **Module-layout rustdoc** — added a `sensors` entry to the
   `# Module layout` list at the top of `lib.rs`.
3. **Startup sequence** — added "Step 2.7" between the audio device
   setup (Step 2.6) and the `/proc` population (Step 3), calling
   `sensors::create_sensor_device(&cfg.rootfs)?.spawn()` and storing
   the handle in `_sensor_handle`. Failure is non-fatal: a `warning!`
   is logged and the daemon continues (the guest can boot without
   sensors — `SensorManager.getDefaultSensor()` will return null and
   the guest's sensor framework will fall back to "no sensors
   available" mode).

### 1.4 Tests

60 new unit tests in `sensors::tests`, all `cargo test --lib`:

- **`SensorEvent` layout (1)** — `sensor_event_size_is_24_bytes`.
- **`SensorEvent` serialization (4)** — basic roundtrip, max idx
  (idx 11 + `u64::MAX` ts + `±Infinity` floats), zero roundtrip,
  negative floats (accel `-9.81`), ignores trailing bytes (so
  multiple events can be parsed from one buffer).
- **`SensorEvent` validation (5)** — rejects short buffer, rejects
  bad idx (12, `u32::MAX`), `is_valid` checks idx range, error
  `Display` is informative.
- **`SensorType` enum (5)** — `#[repr(u32)]` values match
  `android.hardware.Sensor.TYPE_*` exactly (1, 2, 3, 4, 5, 6, 7, 8,
  9, 12, 18, 19); `from_type_value` roundtrip; `from_type_value`
  rejects unknown values (0, 10, 11, 13, 17, 20, `u32::MAX`);
  `SENSOR_TYPE_MAP` has 12 entries; `index_to_type` matches VM's
  mapping verbatim; `index_to_type` rejects out-of-range;
  `type_to_index` is the inverse of `index_to_type`;
  `type_to_index` returns `Some` for all 12 variants.
- **`SensorState` bitflags (11)** — bit values match VM (1/2/4);
  `empty` is zero; `from_bits` masks to low 3 bits; `contains`;
  `insert`/`remove`; `BitOrAssign`; `BitOr` combines bits; `BitAnd`;
  `Not`; `ALL` constant; `Default` is empty.
- **`SensorControl` (4)** — size is 12 bytes; roundtrip; rejects
  short buffer; ignores trailing bytes.
- **`create_sensor_device` (3)** — creates the socket file at
  `{rootfs}/dev/sensors`; creates parent `/dev` dir if missing;
  replaces a stale socket file.
- **`SensorDevice::spawn` end-to-end (8)** — accepts a connection
  and handles `CHECK_SUPPORT` (replies 0 since stub returns false);
  handles `ENABLE`/`DISABLE`/`SET_DELAY` without crashing;
  handles unknown command without crashing; handles out-of-range
  `idx` without crashing; handles multiple sequential connections
  (reconnect race); pump sub-thread runs idle loop without crashing;
  rejects short control message (less than 12 bytes + EOF);
  `shutdown()` joins the accept thread; drop-without-shutdown also
  joins.
- **`SensorConnState` (5)** — starts empty; enable/disable
  roundtrip; `set_delay` appears in snapshot; `check_support` caches
  result; shutdown flag roundtrip.
- **`ThreadPool` (3)** — executes jobs; queues jobs beyond worker
  count; panics on `new(0)`.
- **`read_exact` EOF (1)** — returns `UnexpectedEof` when the peer
  closes mid-control-message.
- **JNI stubs (5)** — `jni_check_sensor_support` returns false;
  `jni_enable_sensor` returns false; `jni_disable_sensor` is no-op;
  `jni_set_sensor_delay` is no-op; `jni_read_sensor_event` returns
  `None`.

All tests use the same `tmpdir()` helper pattern as `audio.rs` /
`binder.rs` (unique per-test subdirectory under
`$TMPDIR/kr64-sensor-test-<pid>-<n>`) so parallel test execution
doesn't collide on socket paths.

---

## 2. What was deliberately NOT implemented

These are the skeleton boundaries — each is a follow-up task:

| Item | Why deferred | Follow-up ID |
|---|---|---|
| **Real JNI up-calls** to `SensorManager` | Requires the `jni` crate (or hand-rolled `JNIEnv` calls like `input.rs`), a host `SensorService.java`, and `HIGH_SAMPLING_RATE_SENSORS` permission plumbing (only for >200 Hz sensors). The skeleton's stubs return `false`/`None` so `CHECK_SUPPORT` replies 0 to every query and the pump never produces events. | SENSOR-IMPL-2 |
| **`nativeSensorChanged` JNI down-call** entry point | This is the Java→Rust down-call that pushes a `SensorEvent` into the per-idx `mpsc::Sender`. In the skeleton, `jni_read_sensor_event` is a poll-based stub that returns `None`; in the real impl it would either (a) drain a per-idx `mpsc::Receiver` filled by `nativeSensorChanged`, or (b) call a Java `pollSensorEvent(idx)` method. Either approach is a SENSOR-IMPL-2 change. | SENSOR-IMPL-2 |
| **Write mutex on the socket** | With `try_clone()`, the control worker (writes 4-byte CHECK_SUPPORT replies) and the pump sub-thread (writes 24-byte events) share the same underlying socket via two fd clones. If both write concurrently the bytes can interleave. In the skeleton this is a non-issue (the pump stub returns `None`, so no events are written). For the real impl, wrap writes in an `Arc<Mutex<()>>` or use a single writer thread that drains an mpsc of `enum OutMsg { Reply(u32), Event(SensorEvent) }`. | SENSOR-IMPL-2 |
| **`create_sensor_device` added to `devices::DeviceSet`** | The HAL-DETAIL-1 analysis suggested adding `sensor` to `DeviceSet` (returning a `DeviceSocket`). The task spec instead said `create_sensor_device` should return `SensorDevice` directly (because the sensor pump needs its own accept thread + pool + per-connection pump sub-thread, not the simple "echo a byte and close" pattern the other devices use in the MVP). So `sensor` is NOT in `DeviceSet`; it's a separate step in `lib.rs::run`, mirroring how `audio` and `binder` are handled. | — (deliberate design choice) |
| **Guest ROM audit** (`sensors.<board>.so`) | The highest-risk item from `AUDIO_SENSOR_HAL.md` §4: if the guest's sensor HAL module expects the standard `/dev/input/event*` sysfs path with `EV_ABS` events (rather than the Unix-socket `/dev/sensors` protocol that VM's `libvm.so` expects), this whole approach doesn't work. | SENSOR-RISK-1 (blocking) |
| **`HIGH_SAMPLING_RATE_SENSORS` permission** | Required only for >200 Hz sensors (API 31+). Normal accel/gyro/mag don't need it. The skeleton's stub returns false for every sensor, so this is inert until both the manifest change and the JNI wiring are done. | MANIFEST-1 |
| **Foreground/background pause** | VM's `HALManager.onBackground()` (line 719) unregisters all `ACTIVE` sensors when the VM is backgrounded, and `onForeground()` (line 759) re-registers them. Twoyi should do the same in `Activity.onPause()/onResume()` once the JNI is wired up. | SENSOR-IMPL-2 |
| **`SetDelay` quirk mirroring** | VM's `SetDelay` (line 561) zeroes both the sampling period AND the max-latency arrays, then uses `1` (us) if `0`. Twoyi treats this as a normal set-with-floor — the pump clamps to `MIN_POLL_NS` = 1 ms. Documented as a deliberate deviation in the `jni_set_sensor_delay` doc comment + the module rustdoc. | — (deliberate deviation) |

---

## 3. Build & test verification

```
$ cd /home/z/my-project/app/rs/kr64
$ cargo build                          # Finished, 0 warnings
$ cargo build --all-targets           # Finished, 0 warnings
$ cargo test --lib                     # 125 passed; 0 failed; 0 ignored
                                        # (38 pre-existing + 27 audio
                                        #  + 60 new sensor)
                                        # runtime: 1.04s
```

The crate still depends on **only `libc`** (no `log`, `jni`,
`crossbeam`, `bitflags`, etc.). The `Cargo.toml` is unchanged.

---

## 4. File changes

| File | Change | LOC |
|---|---|---|
| `app/rs/kr64/src/sensors.rs` | **NEW** — full sensor HAL skeleton | ~2 290 (incl. ~430 LOC of rustdoc + ~640 LOC of tests) |
| `app/rs/kr64/src/lib.rs` | Added `pub mod sensors;`, updated module-layout rustdoc, added Step 2.7 in `run()` | +35 |

No other files were modified. No new dependencies were added.

---

## 5. Wire protocol reference (for follow-up tasks)

### 5.1 The 12-sensor mapping (guest index → host `Sensor.TYPE_*`)

| Guest idx | `Sensor.TYPE_*` | Constant name                | `SensorType` variant       |
|----------:|----------------:|------------------------------|----------------------------|
| 0         | 1               | `TYPE_ACCELEROMETER`         | `Accelerometer`            |
| 1         | 2               | `TYPE_MAGNETIC_FIELD`        | `MagneticField`            |
| 2         | 3               | `TYPE_ORIENTATION`           | `Orientation`              |
| 3         | 7               | `TYPE_TEMPERATURE`           | `Temperature`              |
| 4         | 8               | `TYPE_LIGHT`                 | `Light`                    |
| 5         | 5               | `TYPE_PROXIMITY`             | `Proximity`                |
| 6         | 6               | `TYPE_GYROSCOPE`             | `Gyroscope`                |
| 7         | 12              | `TYPE_RELATIVE_HUMIDITY`     | `RelativeHumidity`         |
| 8         | 9               | `TYPE_PRESSURE`              | `Pressure`                 |
| 9         | 19              | `TYPE_GRAVITY`               | `Gravity`                  |
| 10        | 18              | `TYPE_STEP_DETECTOR`         | `StepDetector`             |
| 11        | 4               | `TYPE_GYROSCOPE_UNCALIBRATED` | `GyroscopeUncalibrated`    |

### 5.2 The 12-byte control message

```
 offset  size  field   description
 ------  ----  ------  ------------------------------------------
  0       4    cmd     LE u32 — 1=ENABLE, 2=DISABLE,
                          3=CHECK_SUPPORT, 4=SET_DELAY
  4       4    idx     LE u32 — sensor index 0..11
  8       4    arg     LE u32 — for SET_DELAY: sampling period
                          in nanoseconds; ignored otherwise
 ------  ----
 total = 12 bytes
```

For `CHECK_SUPPORT`, the host replies with a 4-byte LE u32 (0 = not
supported, 1 = supported). The other commands have no reply.

### 5.3 The 24-byte sensor event

```
 offset  size  field   description
 ------  ----  ------  ------------------------------------------
  0       4    idx     LE u32 — sensor index 0..11
  4       8    ts      LE u64 — timestamp in nanoseconds
 12       4    x       LE f32
 16       4    y       LE f32
 20       4    z       LE f32
 ------  ----
 total = 24 bytes
```

### 5.4 The 3-bit `SensorState` mask

| Bit | Mask | Constant    | Meaning                                              |
|----:|-----:|-------------|------------------------------------------------------|
|  0  |   1  | `SUPPORTED` | Host has this sensor (`getDefaultSensor()` ≠ null)  |
|  1  |   2  | `ENABLED`   | Guest requested enable                               |
|  2  |   4  | `ACTIVE`    | `SensorManager.registerListener` was called         |

### 5.5 The 5 JNI up-calls the Rust control/pump code makes into Java

| Rust function (stub today)         | Java method on `HALManager` / `SensorService`           | Returns                  | When called                                  |
|------------------------------------|----------------------------------------------------------|--------------------------|----------------------------------------------|
| `jni_check_sensor_support`         | `boolean CheckSensorsSupport(int idx)`                   | `bool`                   | On `CHECK_SUPPORT` control message           |
| `jni_enable_sensor`                | `boolean EnableSensors(int idx)`                         | `bool` (success)         | On `ENABLE` control message                  |
| `jni_disable_sensor`               | `void DisableSensors(int idx)`                           | `()`                     | On `DISABLE` control message                 |
| `jni_set_sensor_delay`             | `void SetDelay(int idx, int delayNs)`                    | `()`                     | On `SET_DELAY` control message               |
| `jni_read_sensor_event`            | (down-call from `nativeSensorChanged` queue)             | `Option<SensorEvent>`    | Per pump iteration, per enabled sensor       |

### 5.6 The 1 JNI down-call (Java → Rust, not yet wired up)

| Java method (to be added)                          | Rust entry point (to be added)                          | When called                                  |
|----------------------------------------------------|---------------------------------------------------------|----------------------------------------------|
| `HALManager.nativeSensorChanged(long ptr, int idx, long ts, float x, float y, float z)` | `#[no_mangle] pub extern "system" fn Java_io_twoyi_hal_HALManager_nativeSensorChanged(...)` | Each time the host `SensorManager` fires `onSensorChanged` for a sensor the guest has enabled |

---

## 6. Design notes

### 6.1 Why per-connection pump sub-thread (not a single global pump)

VM's `SensorService` is under `HALManager` (unlike `AudioService`
which is top-level) precisely because sensor events are bursty and
low-rate — they don't need a dedicated real-time thread. Twoyi
mirrors that split: each guest connection gets its own pump
sub-thread (named `kr64-sensor-pump`), but there's no global
always-on pump. When the guest disconnects, the pump sub-thread
exits. This keeps the resource footprint proportional to the number
of active guests (currently always 1, but the architecture supports
multi-VM in the future).

### 6.2 Why `try_clone()` instead of mpsc for the pump

The pseudo-Rust in `AUDIO_SENSOR_HAL.md` §3.2 used an
`mpsc::channel<SensorEvent>` between the control thread (which would
receive events from `nativeSensorChanged`) and the pump thread (which
drains the channel and writes to the socket). Twoyi's skeleton
instead polls `jni_read_sensor_event` directly from the pump thread.
The reasons:

1. **The skeleton has no `nativeSensorChanged` down-call yet.** With
   the channel approach, the channel would always be empty and the
   pump would block forever on `recv()`. The poll approach lets the
   pump run its idle loop and exercise the protocol.
2. **The poll approach is closer to how the real impl will likely
   work.** Even with `nativeSensorChanged` wired up, the Rust side
   will probably maintain a per-idx `mpsc::Receiver` (or a
   `crossbeam::ArrayQueue`) that the pump drains on each iteration.
   The `jni_read_sensor_event` function is the natural seam for this
   — in the real impl it becomes `receiver.try_recv().ok()`.
3. **Decouples pump cadence from event arrival.** With the channel
   approach, the pump would wake on every event (which could be
   500+/sec for a fast gyro). With the poll approach, the pump
   controls its own cadence — it sleeps for the shortest `SET_DELAY`
   among enabled sensors, then drains whatever's queued. This
   matches VM's `SensorManager.registerListener(...,
   samplingPeriodUs, maxReportLatencyUs, handler)` API, which
   explicitly separates the sampling period from the delivery
   latency.

### 6.3 Why the `SensorState` bitflags type is hand-rolled

The `bitflags` crate is the standard way to define bitflags in Rust,
but the project constraint is "std + libc only" (no external crates
beyond `libc` for raw syscall bindings). The hand-rolled
`SensorState` newtype provides the same API surface
(`empty`/`contains`/`insert`/`remove`/`bits`/`from_bits` + `BitOr`/
`BitAnd`/`Not`/`Assign` traits) in ~60 LOC. The trade-off is no
`fmt::Binary` impl (so `format!("{:b}", state)` doesn't work) —
`Debug` is derived instead, which prints `SensorState(5)` for
`SUPPORTED | ACTIVE`. Adequate for the skeleton.

### 6.4 Why `#[repr(C, packed)]` for `SensorEvent`

The wire format is 24 bytes packed: `u32 idx` at offset 0, `u64 ts`
at offset 4 (NOT 8 — there's no alignment padding on the wire). With
`#[repr(C)]` (not packed), Rust would insert 4 bytes of padding
after `idx` to align `ts` to 8, making the in-memory struct 32 bytes
— wrong size, wrong layout. `#[repr(C, packed)]` matches the wire
layout exactly, and the compile-time assertion
`const _: () = assert!(size_of::<SensorEvent>() == 24);` guarantees
this can't silently break.

The packed-struct footgun (taking a reference to a mis-aligned field
is undefined behaviour) is avoided by never taking references:
`to_bytes`/`from_bytes` read each field by value (which the
compiler handles via unaligned loads on architectures that support
them, or via a temporary aligned copy otherwise).

### 6.5 Why `MIN_POLL_NS = 1 ms` instead of VM's `1 us`

VM's `SetDelay` (line 561) has a quirk where it uses `1`
(microsecond) if the requested delay is `0`. This is almost
certainly a decompiler artifact (`1` is the smallest non-zero
positive `int`, and the bytecode does `if (delay == 0) delay = 1`).
At 1 µs polling, the pump thread would burn 100% CPU and produce
~1 million events/sec — far beyond what any Android sensor can
physically deliver (the fastest accel on a Pixel 6 Pro is 800 Hz).
Twoyi clamps to 1 ms instead (1000 Hz max), which is still faster
than any real sensor but doesn't burn CPU. Documented as a
deliberate deviation in the `jni_set_sensor_delay` doc comment.

### 6.6 Why the skeleton's `CHECK_SUPPORT` always replies 0

The stubbed `jni_check_sensor_support` returns `false` unconditionally.
This means the guest sees "no sensors available" —
`SensorManager.getDefaultSensor(TYPE_ACCELEROMETER)` returns null,
and the guest's sensor framework falls back to no-sensor mode. This
is the safe default: it doesn't lie about sensor availability (which
could cause guest apps to crash with `NullPointerException` when they
try to read a sensor value that never arrives), and it lets the boot
proceed all the way to the launcher without any JNI wiring.

The SENSOR-IMPL-2 follow-up will replace the stub with a real
`HALManager.CheckSensorsSupport(idx)` call that queries
`SensorManager.getDefaultSensor(TYPE_*)` on the host. At that point
the guest will see the host's actual sensor list.

---

## 7. Next actions

- **SENSOR-IMPL-2 (next):** Wire up the real JNI. Steps:
  1. Add `<uses-permission android:name="android.permission.HIGH_SAMPLING_RATE_SENSORS" />` to `AndroidManifest.xml` (MANIFEST-1) — only needed for >200 Hz sensors; accel/gyro/mag don't need it.
  2. Write `io.twoyi.hal.SensorService.java` — a near-1:1 port of VM's `com.android.vmcore.hal.SensorService` (160 lines), with the `SparseIntArray` mapping table, the `SensorEventListener` impl, and the foreground/background pause hooks.
  3. Write `io.twoyi.hal.HALManager.java` (or extend the existing one) — port VM's `CheckSensorsSupport`/`EnableSensors`/`DisableSensors`/`SetDelay` methods (lines 178, 187, 200, 561) and the `nativeSensorChanged` down-call (line 661).
  4. Replace the five stub functions in `sensors.rs` with real JNI calls — either via the `jni` crate (preferred, ~5 lines per function) or hand-rolled `JNIEnv` calls like `app/rs/src/input.rs` already does (zero deps, ~25 lines per function).
  5. Add the `nativeSensorChanged` JNI entry point as a `#[no_mangle] pub extern "system" fn Java_io_twoyi_hal_HALManager_nativeSensorChanged(...)` that pushes a `SensorEvent` into the per-idx queue.
  6. Add the write mutex on the socket (or refactor to a single writer thread with an mpsc of `enum OutMsg { Reply(u32), Event(SensorEvent) }`) so CHECK_SUPPORT replies can't interleave with sensor events.
  7. Acceptance test: guest `SensorManager.getDefaultSensor(TYPE_ACCELEROMETER)` returns non-null and a tilt-test (rotate the host device, observe the guest's `Display.rotate`) works.

- **SENSOR-RISK-1 (blocking, parallel):** Audit the guest ROM's `sensors.<board>.so` HAL module — confirm it opens `/dev/sensors` (or `/dev/input/sensor`, per the analysis) as a Unix socket. If it expects the standard `/dev/input/event*` sysfs path with `EV_ABS` events, the entire sensor plan needs reworking to mirror twoyi's existing `input.rs` pattern instead. This was already flagged in `AUDIO_SENSOR_HAL.md` §4.2.

- **SENSOR-IMPL-3 (optional, after IMPL-2):** Add the foreground/background pause logic. Mirror VM's `HALManager.onBackground()` (line 719) and `onForeground()` (line 759): when the host `Activity.onPause()` fires, set a flag in the Rust dispatcher that makes `jni_read_sensor_event` return `None` for all sensors (so the pump stops producing events) and call `SensorManager.unregisterListener` for all active sensors. On `onResume()`, clear the flag and re-register.

- **REFACTOR-1 (optional):** Lift the `ThreadPool` out of `binder.rs`, `audio.rs`, and `sensors.rs` into a shared `app/rs/kr64/src/thread_pool.rs` so the three modules don't duplicate ~50 LOC each. Low priority — the duplication is harmless and the modules are self-contained. (This was already flagged in AUDIO-IMPL-1's "Next actions".)

- **MVP shortcut (still on the table):** Per `AUDIO_SENSOR_HAL.md` §2.7, ship only idx 0 (accel), 1 (mag), 6 (gyro) — these cover 95% of real apps. Return `false` from `CheckSensorsSupport` for the other 9 indices. ~50% less code in the Java side. The Rust skeleton already supports all 12, so this is purely a Java-side decision.

— End of summary —
