# Audio HAL Implementation — kr64 Skeleton

> **Task ID:** AUDIO-IMPL-1
> **Date:** 2026-08-05
> **Author:** general-purpose sub-agent
> **Inputs:** `download/AUDIO_SENSOR_HAL.md` (the HAL-DETAIL-1 analysis),
> `app/rs/kr64/src/devices.rs`, `app/rs/kr64/src/binder.rs`,
> `app/rs/kr64/src/lib.rs`.
> **Scope:** Implement the audio HAL skeleton in the kr64 crate as a
> real (compiling, tested) Rust module, mirroring the design documented
> in `AUDIO_SENSOR_HAL.md` §3.1.

---

## TL;DR

- Added `app/rs/kr64/src/audio.rs` (~990 LOC including docs + tests): the
  virtual `/dev/audio` Unix socket, a 16-byte connection header
  (`AudioHeader`), a `Playback`/`Capture` direction enum, an accept
  thread + fixed-size worker thread pool, per-connection PCM pump
  loops, and six JNI up-call stubs that mirror VM's `AudioService`
  private methods.
- Updated `app/rs/kr64/src/lib.rs`: registered the new `audio` module
  and wired `audio::create_audio_device(&cfg.rootfs)?.spawn()` into the
  daemon startup sequence (right after the binder proxy setup, before
  `/proc` population). Non-fatal on failure — the guest can boot
  without sound.
- The crate compiles clean with `cargo build` (no warnings) and all 65
  unit tests pass with `cargo test` (38 pre-existing + 27 new audio
  tests, runtime 0.38 s).
- **No JNI is wired up yet** — the six up-call functions are no-op
  stubs that return `null`/`0`. The pump loops therefore close the
  connection after the header is read. This is the deliberate
  "skeleton" boundary: the protocol layer is complete and tested; the
  host `AudioTrack`/`AudioRecord` integration is the next task
  (`AUDIO-IMPL-2`).
- No external crates added. The crate still depends on only `libc` +
  `std`. The thread pool is the same MPMC-via-`mpsc::channel` pattern
  already used in `binder.rs` (kept private to each module so they're
  self-contained).

---

## 1. What was implemented

### 1.1 New file: `app/rs/kr64/src/audio.rs`

The file is organised into seven sections, each documented in the
module-level rustdoc:

1. **Constants** — `AUDIO_HEADER_MAGIC` (`0x4F444D41`),
   `AUDIO_HEADER_SIZE` (16), `AUDIO_DIR_PLAYBACK` (1),
   `AUDIO_DIR_CAPTURE` (2), the VM-hard-coded sample rates
   (`PLAYBACK_SAMPLE_RATE = 44_100`, `CAPTURE_SAMPLE_RATE = 11_025`),
   channel counts (`PLAYBACK_CHANNELS = 2`, `CAPTURE_CHANNELS = 1`),
   pool size (`AUDIO_THREAD_POOL_SIZE = 8`), and the pump scratch
   buffer cap (`AUDIO_PUMP_BUF_SIZE = 8 KiB`).

2. **`AudioDirection` enum** — `#[repr(u8)]` so `as u8` gives the exact
   wire byte. Has `from_u8`, `default_sample_rate`, and
   `default_channels` accessors.

3. **`AudioHeader` struct** — `#[repr(C)]`, 16 bytes on the wire
   (compile-time asserted via `const _: () = assert!(size_of::<AudioHeader>() == 16);`).
   Provides `new`, `with_format`, `direction`, `is_valid`, `to_bytes`
   (→ `[u8; 16]` little-endian), and `from_bytes` (validates magic +
   direction). Includes the `AudioHeaderError` enum (`TooShort`,
   `BadMagic`, `BadDirection`) with `Display` + `std::error::Error`
   impls.

4. **`create_audio_device(rootfs)`** — creates `{rootfs}/dev/audio` as
   a `UnixListener` (mirrors `devices::create_touch_device` /
   `binder::create_binder_device`). Creates parent dirs, removes stale
   socket files, binds, and `chmod 0666`s the socket so the guest can
   `connect()`. Returns an `AudioDevice` that owns the listener.

5. **`AudioDevice` + `AudioDeviceHandle`** — `AudioDevice::spawn(mut
   self)` consumes the device, makes the listener non-blocking, and
   spawns the accept thread (named `kr64-audio-accept`). The accept
   thread owns a `ThreadPool` of 8 workers (named `kr64-audio-worker`)
   and dispatches each accepted connection to a worker via
   `pool.execute(move || handle_connection(stream))`. The returned
   `AudioDeviceHandle` holds the shutdown `Arc<AtomicBool>` + accept
   thread `JoinHandle`; its `Drop` sets the flag, joins the thread,
   and unlinks the socket file. Mirrors `BinderProxy` / `BinderProxyHandle`
   exactly.

6. **Per-connection handler** — `handle_connection` reads the 16-byte
   header, validates it, dispatches to `handle_playback` or
   `handle_capture`. The playback pump reads PCM from the socket in
   chunks of `min(host_min_buf, 8 KiB)` bytes and calls
   `jni_write_audio_data(track, buf)` per chunk. The capture pump calls
   `jni_read_record_data(record, buf)` and writes the returned bytes to
   the socket. Both pumps release the host object on exit.

7. **JNI up-call stubs** — `jni_acquire_audio_track`,
   `jni_acquire_audio_record`, `jni_write_audio_data`,
   `jni_read_record_data`, `jni_release_audio_track`,
   `jni_release_audio_record`. Each is a one-line no-op returning
   `null`/`0`/`()`. They're documented with the exact Java signature
   they'll need to invoke (`AudioService.acquireAudioTrack([I)Landroid/media/AudioTrack;`
   etc.) so the follow-up task can fill them in without re-reading the
   analysis doc. The `JniObject` type alias is `*mut c_void` so the
   skeleton doesn't need the `jni` crate.

8. **`ThreadPool`** — same pattern as `binder.rs::ThreadPool`:
   `mpsc::channel` + `Arc<Mutex<Receiver>>` + `Worker` struct that
   loops on `recv()` and dispatches `Message::Job` / `Message::Terminate`.
   Kept private to `audio` so the two modules are independent (a future
   refactor could lift it to a shared `thread_pool.rs`, but that's out
   of scope here).

9. **`read_exact` helper** — blocks until `buf.len()` bytes are read or
   the peer closes (returns `UnexpectedEof`). Used for the 16-byte
   header read.

### 1.2 Wire protocol documentation

The module's rustdoc opens with a full specification of the 16-byte
header (offset table + field descriptions), the playback data-flow
ASCII diagram (guest AudioFlinger → socket → host AudioTrack), the
capture data-flow ASCII diagram (host AudioRecord → socket → guest
RecordThread), the JNI callback table (6 stubs ↔ 6 Java methods), the
threading model (1 accept thread + 8 worker pool), the VM-hard-coded
format table, and a latency note pointing at the `AUDIO-IMPL-2`
low-latency follow-up.

### 1.3 `lib.rs` integration

Three changes to `app/rs/kr64/src/lib.rs`:

1. **Module declaration** — added `pub mod audio;` (after `pub mod
   binder;`, before `pub mod seccomp;`).
2. **Module-layout rustdoc** — added an `audio` entry to the
   `# Module layout` list at the top of `lib.rs`.
3. **Startup sequence** — added "Step 2.6" between the binder proxy
   setup and the `/proc` population, calling
   `audio::create_audio_device(&cfg.rootfs)?.spawn()` and storing the
   handle in `_audio_handle`. Failure is non-fatal: a `warning!` is
   logged and the daemon continues (the guest can boot without sound —
   AudioFlinger's `connect()` to `/dev/audio` will fail and the guest's
   audio HAL will fall back to silence).

### 1.4 Tests

27 new unit tests in `audio::tests`, all `cargo test --lib`:

- **Header layout (1)** — `audio_header_size_is_16_bytes`.
- **Header roundtrips (4)** — playback, capture, custom format, reserved
  bytes are zero, ignores trailing bytes (so the header can be read
  from the start of a larger buffer that includes PCM data).
- **Header validation (5)** — rejects short buffer, bad magic, bad
  direction (99), zero direction; `is_valid` checks both magic +
  direction; `AudioHeaderError::Display` is informative.
- **`AudioDirection` (3)** — `from_u8` roundtrip, `#[repr(u8)]` matches
  wire byte, defaults match VM's hard-coded rates (44 100/11 025,
  stereo/mono).
- **`create_audio_device` (3)** — creates the socket file, creates
  parent `/dev` dir if missing, replaces a stale socket file.
- **`AudioDevice::spawn` end-to-end (5)** — accepts a playback
  connection + sends a Capture header on a second connection (verifies
  the worker pool doesn't leak threads); rejects bad magic; rejects
  short header (less than 16 bytes + EOF); explicit `shutdown()` joins
  the accept thread; drop-without-shutdown also joins.
- **`ThreadPool` (3)** — executes jobs, queues jobs beyond worker
  count, panics on `new(0)`.
- **`read_exact` EOF (1)** — returns `UnexpectedEof` when the peer
  closes mid-header.

All tests use the same `tmpdir()` helper pattern as `binder.rs` (unique
per-test subdirectory under `$TMPDIR/kr64-audio-test-<pid>-<n>`) so
parallel test execution doesn't collide on socket paths.

---

## 2. What was deliberately NOT implemented

These are the skeleton boundaries — each is a follow-up task:

| Item | Why deferred | Follow-up ID |
|---|---|---|
| **Real JNI up-calls** to `AudioTrack`/`AudioRecord` | Requires the `jni` crate (or hand-rolled `JNIEnv` calls like `input.rs`), a host `AudioService.java`, and `RECORD_AUDIO` permission plumbing. The skeleton's stubs return `null`/`0` so the pump loops exit immediately after the header is read. | AUDIO-IMPL-2 |
| **Low-latency playback** via `AudioTrack.Builder.setPerformanceMode(PERFORMANCE_MODE_LOW_LATENCY)` | Java-only change in `acquireAudioTrack`; the Rust pump doesn't care. Drops host-side latency from ~125 ms to ~20 ms — critical for the user's rhythm-game use case. | AUDIO-IMPL-2 |
| **`create_audio_device` added to `devices::DeviceSet`** | The HAL-DETAIL-1 analysis suggested adding `audio` to `DeviceSet` (returning a `DeviceSocket`). The task spec instead said `create_audio_device` should return `AudioDevice` directly (because the audio pump needs its own accept thread + pool, not the simple "echo a byte and close" pattern the other devices use in the MVP). So `audio` is NOT in `DeviceSet`; it's a separate step in `lib.rs::run`, mirroring how `binder` is handled. | — (deliberate design choice) |
| **Guest ROM audit** (`audio.primary.<board>.so`) | The single highest-risk item from AUDIO_SENSOR_HAL.md §4: if the guest's audio HAL module speaks tinyalsa (`/dev/snd/*`) instead of the Unix-socket `/dev/audio` protocol, this whole approach doesn't work. | AUDIO-RISK-1 (blocking) |
| **`RECORD_AUDIO` permission** in `AndroidManifest.xml` | Required for the capture path. Playback needs no permission. The skeleton's stub returns null for `acquireAudioRecord`, so capture is inert until both the manifest change and the JNI wiring are done. | MANIFEST-1 |
| **`sensors.rs`** | Sister module to `audio.rs`; same architecture but for the 12-sensor multiplexed HAL. Separate task. | SENSOR-IMPL-1 |

---

## 3. Build & test verification

```
$ cd /home/z/my-project/app/rs/kr64
$ cargo build                          # Finished, 0 warnings
$ cargo build --all-targets           # Finished, 0 warnings
$ cargo test --lib                     # 65 passed; 0 failed; 0 ignored
                                        # (38 pre-existing + 27 new audio)
                                        # runtime: 0.38s
```

The crate still depends on **only `libc`** (no `log`, `jni`,
`crossbeam`, etc.). The `Cargo.toml` is unchanged.

---

## 4. File changes

| File | Change | LOC |
|---|---|---|
| `app/rs/kr64/src/audio.rs` | **NEW** — full audio HAL skeleton | ~990 (incl. ~250 LOC of rustdoc + ~280 LOC of tests) |
| `app/rs/kr64/src/lib.rs` | Added `pub mod audio;`, updated module-layout rustdoc, added Step 2.6 in `run()` | +33 |

No other files were modified. No new dependencies were added.

---

## 5. Wire protocol reference (for follow-up tasks)

### 5.1 The 16-byte header

```
 offset  size  field         description
 ------  ----  -----------   ------------------------------------------
  0       4    magic         LE u32, must be 0x4F444D41 (AUDIO_HEADER_MAGIC)
  4       1    direction     1 = Playback, 2 = Capture
  5       3    reserved      zero (padding)
  8       4    sample_rate   LE u32, informational (host ignores)
 12       2    channels      LE u16, informational (host ignores)
 14       2    reserved      zero (padding)
 ------  ----
 total = 16 bytes
```

### 5.2 The 6 JNI up-calls the Rust pump makes into Java

| Rust function (stub today)        | Java method on `AudioService`                    | Returns                  | When called                      |
|-----------------------------------|--------------------------------------------------|--------------------------|----------------------------------|
| `jni_acquire_audio_track`         | `AudioTrack acquireAudioTrack(int[] outMinBuf)`  | `(jtrack, minBufSize)`   | Once per Playback connection     |
| `jni_acquire_audio_record`        | `AudioRecord acquireAudioRecord(int[] outMinBuf)`| `(jrecord, minBufSize)`  | Once per Capture connection      |
| `jni_write_audio_data`            | `int writeAudioData(AudioTrack, byte[], o, l)`   | bytes written            | Per PCM chunk read from socket   |
| `jni_read_record_data`            | `int readRecordData(AudioRecord, byte[], o, l)`  | bytes read               | Per PCM chunk written to socket  |
| `jni_release_audio_track`         | `void releaseAudioTrack(AudioTrack)`             | `()`                     | On Playback connection close     |
| `jni_release_audio_record`        | `void releaseAudioRecord(AudioRecord)`           | `()`                     | On Capture connection close      |

### 5.3 Hard-coded format (matches VM's `acquireAudioTrack` / `acquireAudioRecord`)

| Direction | Rate (Hz) | Channels | Encoding   | Android constants                        |
|-----------|----------:|---------:|------------|------------------------------------------|
| Playback  |    44 100 |        2 | PCM_16BIT  | STREAM_MUSIC, CHANNEL_OUT_STEREO, MODE_STREAM |
| Capture   |    11 025 |        1 | PCM_16BIT  | MIC source, CHANNEL_IN_MONO              |

---

## 6. Next actions

- **AUDIO-IMPL-2 (next):** Wire up the real JNI. Steps:
  1. Add `<uses-permission android:name="android.permission.RECORD_AUDIO" />` to `AndroidManifest.xml` (MANIFEST-1).
  2. Write `io.twoyi.hal.AudioService.java` — a near-1:1 port of VM's `com.android.vmcore.hal.AudioService` (222 lines), with `acquireAudioTrack`/`acquireAudioRecord`/`writeAudioData`/`readRecordData`/`releaseAudioTrack`/`releaseAudioRecord` as `private` methods.
  3. Replace the six stub functions in `audio.rs` with real JNI calls — either via the `jni` crate (preferred, ~5 lines per function) or hand-rolled `JNIEnv` calls like `app/rs/src/input.rs` already does (zero deps, ~25 lines per function).
  4. For the rhythm-game latency fix, use `AudioTrack.Builder.setPerformanceMode(PERFORMANCE_MODE_LOW_LATENCY).setBufferSizeInFrames(192)` (API 26+; gate behind `Build.VERSION.SDK_INT`).
  5. Acceptance test: a guest app that calls `AudioTrack.play()` produces audible sound on the host.

- **AUDIO-RISK-1 (blocking, parallel):** Audit the guest ROM's `audio.primary.<board>.so` HAL module — confirm it speaks the Unix-socket `/dev/audio` protocol that VM's `libvm.so` (and now twoyi's `audio.rs`) expects. If it speaks tinyalsa (`/dev/snd/controlC0` + `/dev/snd/pcmC0D0p` + `/dev/snd/pcmC0D0c`), the entire audio plan needs reworking — that's a 2-week task, not a 1-day task. This was already flagged in `AUDIO_SENSOR_HAL.md` §4.1.

- **SENSOR-IMPL-1 (sister task):** Mirror this skeleton for `app/rs/kr64/src/sensors.rs` — same architecture (accept thread + worker pool), different protocol (12-byte control requests + 24-byte sensor events). Effort ~1 day. Acceptance: guest `SensorManager.getDefaultSensor(TYPE_ACCELEROMETER)` returns non-null and tilt-test works.

— End of summary —
