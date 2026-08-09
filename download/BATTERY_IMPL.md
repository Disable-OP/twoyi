# Battery HAL Implementation — kr64 Skeleton

> **Task ID:** BATTERY-IMPL-1
> **Date:** 2026-08-05
> **Author:** general-purpose sub-agent
> **Inputs:** `download/AUDIO_SENSOR_HAL.md` (the HAL-DETAIL-1 analysis,
> for the sibling audio/sensor patterns), `download/HAL_VIRTUALIZATION_ANALYSIS.md`
> §4 "Battery HAL", `download/DEVELOPMENT_ROADMAP.md` task 4.10,
> `app/rs/kr64/src/audio.rs` (AUDIO-IMPL-1 sister module),
> `app/rs/kr64/src/sensors.rs` (SENSOR-IMPL-1 sister module),
> `app/rs/kr64/src/lib.rs` (the `run()` startup sequence).
> **Scope:** Implement the battery HAL skeleton in the kr64 crate as a
> real (compiling, tested) Rust module, mirroring the design documented
> in `DEVELOPMENT_ROADMAP.md` task 4.10 and the patterns established by
> `audio.rs` / `sensors.rs`. Battery is the simplest HAL: file-based,
> no socket, no real-time requirements.

---

## TL;DR

- Added `app/rs/kr64/src/battery.rs` (~855 LOC including ~120 LOC of
  rustdoc and ~240 LOC of tests): materialises the
  `/sys/class/power_supply/battery/` sysfs tree inside the guest
  rootfs with seven files (`capacity`, `status`, `charging`,
  `voltage`, `temperature`, `technology`, `health`), spawns a
  `kr64-battery-refresh` thread that rewrites them every 30 s from
  (stubbed) JNI up-calls, and exposes a `BatteryStatus` enum whose
  `#[repr(u8)]` discriminants match `android.os.BatteryManager`'s
  `BATTERY_STATUS_*` constants.
- Updated `app/rs/kr64/src/lib.rs`: registered the new `battery`
  module and wired `BatteryDevice::new(&cfg.rootfs)?.spawn()` into
  the daemon startup sequence (Step 2.8, right after the sensor
  device setup at Step 2.7, before `/proc` population at Step 3).
  Non-fatal on failure — the guest can boot without a battery sysfs
  (its `BatteryService` will report "unknown" / fall back to
  defaults), but every real device has a battery so we warn loudly.
- The crate compiles clean with `cargo build` and `cargo build
  --all-targets` (0 warnings) and all **144 unit tests pass** with
  `cargo test --lib` (125 pre-existing + 19 new battery tests,
  runtime ~2.04 s).
- **No JNI is wired up yet** — the four up-call functions
  (`jni_get_battery_level`, `jni_get_battery_status`,
  `jni_get_battery_voltage`, `jni_get_battery_temperature`) are
  no-op stubs returning `DEFAULT_CAPACITY` (75),
  `JNI_STATUS_DISCHARGING` (2), `DEFAULT_VOLTAGE_MV` (4200),
  `DEFAULT_TEMP_DECIC` (280 = 28.0 °C). This is the deliberate
  "skeleton" boundary mirroring AUDIO-IMPL-1 / SENSOR-IMPL-1: the
  file-format layer is complete and tested; the host
  `BatteryManager` integration is the next task (BATTERY-IMPL-2).
- No external crates added. The crate still depends on only `libc` +
  `std`.

---

## 1. What was implemented

### 1.1 New file: `app/rs/kr64/src/battery.rs`

The file is organised into seven sections, each documented in the
module-level rustdoc:

1. **Constants** — `BATTERY_DIR_REL` (`sys/class/power_supply/battery`,
   relative to rootfs), `BATTERY_REFRESH_INTERVAL_SECS` (30), the
   seven default values (`DEFAULT_CAPACITY` = 75, `DEFAULT_VOLTAGE_MV`
   = 4200, `DEFAULT_TEMP_DECIC` = 280, `DEFAULT_TECHNOLOGY` = "Li-ion",
   `DEFAULT_HEALTH` = "Good"), and the four JNI status byte constants
   (`JNI_STATUS_CHARGING` = 1, `JNI_STATUS_DISCHARGING` = 2,
   `JNI_STATUS_FULL` = 3, `JNI_STATUS_NOT_CHARGING` = 4 — matching
   `android.os.BatteryManager`'s `BATTERY_STATUS_*` constants).

2. **`BatteryStatus` enum** — 4 variants with `#[repr(u8)]` so `as u8`
   gives the exact JNI byte. Variants: `Charging`, `Discharging`,
   `Full`, `NotCharging`. Methods: `from_u8(u8) -> Option<Self>`,
   `as_str() -> &'static str` (returns the Linux `power_supply` ABI
   string — note "Not charging" is intentionally two words),
   `is_charging() -> bool` (true only for `Charging`, used to derive
   the `charging` 0/1 file).

3. **`BatteryDevice`** — owns the absolute path to the battery sysfs
   dir + a shutdown `Arc<AtomicBool>`. Methods:
   - `new(rootfs)` — `fs::create_dir_all` the full chain
     `sys/class/power_supply/battery`, chmod 0755 on the dir, then
     `refresh()` to write defaults to all seven files immediately
     (so a guest that opens a file before the first refresh tick
     sees sane values). Idempotent — calling twice on the same
     rootfs just overwrites the files.
   - `dir() -> &Path` — absolute path to the battery dir.
   - Seven `update_*` methods — one per file. Each writes ASCII +
     trailing newline, forces mode 0644 (so a stale 0600 file from a
     previous run is corrected). `update_capacity` clamps to 0..100;
     `update_charging` derives `0`/`1` from a `BatteryStatus` (so the
     `charging` and `status` files can never disagree).
   - Four `read_*` methods — `read_capacity`, `read_status`,
     `read_voltage`, `read_temperature`. Mostly for tests, but also
     useful for diagnostics. `read_status` returns `InvalidData` for
     unknown strings.
   - `refresh()` — delegates to the free function `refresh_dir`.
   - `spawn(self)` — consumes the device, clones the shutdown Arc,
     spawns the `kr64-battery-refresh` thread, returns a
     `BatteryDeviceHandle`.

4. **Free helpers** — `write_file_at(dir, name, value)` and
   `refresh_dir(dir)`. These are lifted out of the `impl` block so
   the refresh thread (which owns only a `PathBuf`, not a
   `BatteryDevice`) can call the exact same write logic. This avoids
   the duplication that an earlier draft had via a `BatteryDeviceView`
   struct.

5. **`BatteryDeviceHandle`** — holds the shutdown `Arc<AtomicBool>` +
   the refresh thread's `JoinHandle`. Methods: `shutdown()` (sets the
   flag, doesn't join), `is_shutdown() -> bool`. `Drop` sets the flag
   and joins the thread. Deliberately does NOT unlink the sysfs files
   on drop — they persist across daemon restarts (a new
   `BatteryDevice::new` will overwrite them via `fs::write`); removing
   them would race with any guest process that has them open.

6. **JNI up-call stubs** — `jni_get_battery_level` (returns
   `DEFAULT_CAPACITY`), `jni_get_battery_status` (returns
   `JNI_STATUS_DISCHARGING`), `jni_get_battery_voltage` (returns
   `DEFAULT_VOLTAGE_MV`), `jni_get_battery_temperature` (returns
   `DEFAULT_TEMP_DECIC`). Each is a one-line no-op. Documented with
   the exact Java signature they'll need to invoke
   (`BatteryService.getBatteryLevel() -> int`, etc.) so the
   follow-up task can fill them in without re-reading the analysis
   doc.

7. **Refresh thread** — spawned by `BatteryDevice::spawn`. Loops:
   sleep 30 s (in 1 s ticks so a shutdown signal is observed within
   ~1 s, not 30 s) → call `refresh_dir(&dir)` → repeat. Logs a
   `warning!` on I/O error but continues (the next tick will retry).
   No accept thread, no worker pool — there's no inbound connection
   to accept (battery is pure sysfs).

### 1.2 File-format reference

All seven files live at `{rootfs}/sys/class/power_supply/battery/`
and contain a single ASCII value with a trailing newline (the Linux
`power_supply` ABI convention; Android's readers trim it):

| File           | Format                         | Source                       |
|----------------|--------------------------------|------------------------------|
| `capacity`     | ASCII `0`..`100`               | `jni_get_battery_level`      |
| `status`       | `Charging`/`Discharging`/`Full`/`Not charging` | `jni_get_battery_status` |
| `charging`     | `0` or `1` (derived from status) | derived from `status`      |
| `voltage`      | ASCII mV (e.g. `4200`)         | `jni_get_battery_voltage`    |
| `temperature`  | ASCII 1/10 °C (e.g. `280`)     | `jni_get_battery_temperature`|
| `technology`   | `Li-ion` (constant for now)    | hard-coded                   |
| `health`       | `Good`/`Dead`/`Overheat`/...   | hard-coded                   |

**Unit choices:** `voltage` is in mV and `temperature` is in 1/10 °C
to match the units the JNI stubs return (which in turn match
`android.os.BatteryManager`'s `BatteryManagerExtras` int fields). The
standard Linux `power_supply` ABI uses µV for `voltage_now`, but VM's
file-based battery HAL uses mV — we mirror that to keep the JNI value
and the file content 1:1 (we control both sides of the interface
anyway, so we can pick whatever unit makes the JNI value and file
content identical). If a future guest ROM's battery HAL expects µV,
the fix is a one-line `* 1000` in `update_voltage`.

### 1.3 `lib.rs` integration

Three changes to `app/rs/kr64/src/lib.rs`:

1. **Module declaration** — added `pub mod battery;` (after
   `pub mod sensors;`, before `pub mod seccomp;`).
2. **Module-layout rustdoc** — added a `battery` entry to the
   `# Module layout` list at the top of `lib.rs`.
3. **Startup sequence** — added "Step 2.8" between the sensor device
   setup (Step 2.7) and the `/proc` population (Step 3), calling
   `battery::BatteryDevice::new(&cfg.rootfs)?.spawn()` and storing
   the handle in `_battery_handle`. Failure is non-fatal: a
   `warning!` is logged and the daemon continues (the guest can boot
   without a battery sysfs — its `BatteryService` will report
   "unknown" / fall back to defaults — but every real device has a
   battery so we warn loudly).

### 1.4 Tests

19 new unit tests in `battery::tests`, all `cargo test --lib`:

| Category | Count | Tests |
|----------|------:|-------|
| `BatteryStatus` enum | 4 | `from_u8_roundtrip`, `repr_matches_jni_byte`, `as_str_matches_linux_abi`, `is_charging_only_for_charging` |
| `BatteryDevice::new` | 4 | `creates_dir_and_all_seven_files`, `creates_nested_sys_class_dirs_if_missing`, `is_idempotent`, `writes_default_values_with_trailing_newline` |
| Per-file update methods | 5 | `update_capacity_clamps_above_100`, `update_capacity_accepts_zero`, `update_status_writes_each_variant`, `update_charging_derives_from_status`, `update_voltage_and_temperature_roundtrip`, `update_technology_and_health_write_arbitrary_strings` |
| `read_status` validation | 1 | `read_status_rejects_unknown_string` |
| `refresh` | 1 | `refresh_writes_all_seven_files_from_jni_stubs` |
| `spawn` + `Drop` | 2 | `spawn_then_drop_joins_cleanly`, `spawn_refreshes_files_in_background` |
| JNI stubs | 1 | `jni_stubs_return_documented_defaults` |
| **Total** | **19** | |

Each test gets a UNIQUE tmpdir (via a process-id + atomic-counter
naming scheme) so parallel `cargo test` runs don't collide on the
same sysfs path — mirrors the pattern in `audio.rs` / `sensors.rs` /
`binder.rs`.

---

## 2. Build / test verification

```
$ cd app/rs/kr64
$ cargo build --lib         # 0 warnings, 0.91 s
$ cargo build --all-targets # 0 warnings, 1.20 s
$ cargo test --lib          # 144 passed; 0 failed; 0 ignored; 2.04 s
                            #   (125 pre-existing + 19 new battery)
$ cargo test --lib battery  # 19 passed; 0 failed; 1.01 s
```

No new external dependencies. The crate still depends on only `libc`
(`Cargo.toml` unchanged).

---

## 3. File changes

| File | Status | Lines | Description |
|------|--------|------:|-------------|
| `app/rs/kr64/src/battery.rs` | NEW | 856 | Full battery HAL skeleton with 19 unit tests. |
| `app/rs/kr64/src/lib.rs` | MODIFIED | +31 | `pub mod battery;`, module-layout rustdoc entry, Step 2.8 in `run()`. |
| `Cargo.toml` | unchanged | — | No new deps. |

---

## 4. Design notes

1. **Why file-based and not a socket?** The guest's battery service
   (`health@2.0` / `BatteryService`) polls the sysfs files directly
   — there's no inbound connection to accept. This is the simplest
   HAL: no `UnixListener`, no accept thread, no worker pool, no wire
   protocol. The only piece of concurrency is the 30 s refresh
   thread. Contrast with `audio.rs` (1 accept thread + 8-worker pool
   + per-connection pump) and `sensors.rs` (1 accept thread + 4-worker
   pool + per-connection pump sub-thread).

2. **Why 1 s sleep ticks instead of `thread::sleep(Duration::from_secs(30))`?**
   A 30 s sleep would block `Drop::join()` for up to 30 s on shutdown.
   Sleeping in 1 s ticks and re-checking the shutdown flag between
   ticks means the refresh thread observes shutdown within ~1 s. The
   `spawn_then_drop_joins_cleanly` test asserts this (it would time
   out at the default 60 s test timeout if the thread took 30 s to
   join).

3. **Why `update_charging` takes a `BatteryStatus` instead of a `bool`?**
   The `charging` file is a derivative of `status` (1 iff status ==
   `Charging`). Taking a `BatteryStatus` makes it impossible for the
   caller to put the `charging` and `status` files out of sync. The
   `update_charging_derives_from_status` test asserts this invariant
   for all four status variants.

4. **Why `update_capacity` clamps instead of erroring?** A capacity
   of 150 is a guest-visible lie about battery health, but it's
   recoverable — clamp to 100 and continue. Erroring would cause the
   whole `refresh()` to fail (and the files would retain their
   previous values, which is worse). The `update_capacity_clamps_above_100`
   test asserts both the read-back value AND the on-disk value are
   clamped.

5. **Why no `uevent`?** A real Linux battery driver pushes a `CHANGE`
   uevent on `power_supply` sysfs writes; the guest's `health@2.0`
   HAL listens via netlink and re-reads the files. Twoyi doesn't yet
   emulate netlink (see task NETLINK-1 in `DEVELOPMENT_ROADMAP.md`
   §3.1), so the guest polls the files directly. The 30 s refresh
   interval is comfortably within the guest's typical 1-minute poll
   cadence, so `dumpsys battery` reflects host changes within ~1
   minute. When netlink is implemented, the refresh thread can
   additionally send a `CHANGE` uevent after each write to make the
   guest re-poll immediately.

6. **Why `refresh_dir` is a free function (not a method).** The
   refresh thread owns only a `PathBuf` (moved into the closure by
   `spawn`), not a `BatteryDevice`. Making `refresh_dir` a free
   function that takes `&Path` lets the thread call it directly
   without reconstructing a `BatteryDevice` (which would require a
   duplicate `Arc<AtomicBool>`). An earlier draft used a
   `BatteryDeviceView<'a>` struct for this; the free-function
   approach is simpler and removes ~90 LOC of duplication.

7. **Why `Drop` on the handle does NOT unlink the sysfs files.**
   Unlike a Unix socket file (where a stale socket causes
   `EADDRINUSE` on the next bind), stale sysfs files are harmless —
   `BatteryDevice::new` overwrites them via `fs::write` (truncate +
   write). Removing them on drop would race with any guest process
   that has them open (`unlink` removes the directory entry but the
   open fd stays valid until the guest closes it; a subsequent
   `open()` would then fail with `ENOENT` until the next
   `BatteryDevice::new` recreates them). Leaving them in place is
   the safer choice.

---

## 5. What was deliberately deferred

The skeleton boundary mirrors AUDIO-IMPL-1 and SENSOR-IMPL-1: the
file-format layer is complete and tested; the host `BatteryManager`
integration is the next task.

- **Real JNI.** The four `jni_*` functions are no-op stubs returning
  `DEFAULT_*` constants. BATTERY-IMPL-2 will replace them with real
  `JNIEnv` calls into `io.twoyi.hal.BatteryService.java` (a new Java
  class, ~150 LOC, modeled on VM's `BatteryService`).
- **Netlink uevent.** See design note 5 above.
- **`health` derivation.** Currently hard-coded to "Good". A real
  implementation would derive health from temperature (e.g.
  `Overheat` if temp > 450 = 45.0 °C, `Cold` if temp < 0) and
  capacity (`Dead` if capacity < 5). Tracked as BATTERY-IMPL-3.
- **`technology` derivation.** Currently hard-coded to "Li-ion". A
  real implementation would query the host's
  `BatteryManager.EXTRA_TECHNOLOGY` constant. Tracked as
  BATTERY-IMPL-3.
- **Charger type.** The `charger_type` file (USB/AC/Wireless) is not
  yet materialised. The guest's `BatteryService` infers charger type
  from `status` (Charging implies a charger is connected), so this
  is informational. Tracked as BATTERY-IMPL-3.

---

## 6. Next actions

- **BATTERY-IMPL-2 (next):** Wire up the real JNI. Steps: (a) write
  `io.twoyi.hal.BatteryService.java` (~150 LOC) — register a
  `BroadcastReceiver` for `ACTION_BATTERY_CHANGED`, cache the latest
  `BatteryManager` extras, expose four methods
  (`getBatteryLevel`/`getBatteryStatus`/`getBatteryVoltage`/
  `getBatteryTemperature`) for the Rust side to call via JNI; (b)
  replace the four stub functions in `battery.rs` with real JNI calls
  — either via the `jni` crate (preferred, ~5 lines per function) or
  hand-rolled `JNIEnv` calls like `app/rs/src/input.rs` (zero deps,
  ~25 lines per function); (c) acceptance: `adb shell dumpsys
  battery` inside the guest returns the host's battery level within
  ~1 minute of unplugging/plugging the host charger.

- **BATTERY-IMPL-3 (optional, after IMPL-2):** Derive `health` and
  `technology` from the JNI values. Add a `charger_type` file. ~30
  LOC.

- **NETLINK-1 (blocking for instant-refresh, parallel):** Emulate
  the netlink `KOBJ_CHANGE` uevent on sysfs writes so the guest
  re-polls immediately instead of waiting up to 1 minute. See
  `DEVELOPMENT_ROADMAP.md` §3.1 task list. ~200 LOC, touches
  `proc_emu.rs` and `devices.rs` as well as `battery.rs`.

- **REFACTOR-1 (optional):** Lift the `ThreadPool` out of
  `binder.rs`, `audio.rs`, and `sensors.rs` into a shared
  `app/rs/kr64/src/thread_pool.rs`. Low priority — the duplication
  is harmless and the modules are self-contained. (Battery doesn't
  use a thread pool, so this is unrelated to BATTERY-IMPL-1; flagged
  here for continuity with the SENSOR-IMPL-1 next-actions list.)
