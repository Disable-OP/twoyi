// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use of your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! Virtual `/sys/class/power_supply/battery` — file-based battery HAL.
//!
//! # Overview
//!
//! This is the simplest HAL in the kr64 crate: there is no socket, no
//! wire protocol, and no real-time pump. The guest's battery service
//! (`health@2.0` / `android.hardware.health.ITransportRegistrar` or the
//! legacy `BatteryService`) polls the files under
//! `/sys/class/power_supply/battery/` every few seconds and surfaces
//! them via `BatteryManager` to apps. We materialise that directory
//! tree inside the guest rootfs and refresh the file contents every
//! 30 s from a dedicated worker thread.
//!
//! VM (Virtual Master) implements its battery HAL the same way: a
//! `BatteryService.java` (top-level, like `AudioService`) that calls
//! into native code to write host-derived values to the sysfs tree
//! under the per-VM rootfs. See `download/HAL_VIRTUALIZATION_ANALYSIS.md`
//! §4 "Battery HAL" and `download/DEVELOPMENT_ROADMAP.md` task 4.10.
//!
//! # Files we materialise
//!
//! All seven files live at `{rootfs}/sys/class/power_supply/battery/`
//! and contain a single ASCII value with a trailing newline (the
//! Linux `power_supply` ABI convention; the trailing newline is
//! harmless — Android's `BatteryMonitor::readStringFromFile` / kin
//! trim it):
//!
//!  | File           | Format                         | Source                  |
//!  |----------------|--------------------------------|-------------------------|
//!  | `capacity`     | ASCII `0`..`100`               | `jni_get_battery_level`|
//!  | `status`       | `Charging`/`Discharging`/...   | `jni_get_battery_status`|
//!  | `charging`     | `0` or `1`                     | derived from status     |
//!  | `voltage`      | ASCII mV (e.g. `4200`)         | `jni_get_battery_voltage`|
//!  | `temperature`  | ASCII 1/10 °C (e.g. `280`)     | `jni_get_battery_temperature`|
//!  | `technology`   | `Li-ion` (constant for now)    | hard-coded              |
//!  | `health`       | `Good`/`Dead`/`Overheat`/...   | hard-coded              |
//!
//! # JNI callback interface
//!
//! Four JNI up-calls (all stubbed for now — see the `jni_*` functions
//! at the bottom of this file). The Java side (`BatteryService.java`
//! to be written in task BATTERY-IMPL-2) will attach the current
//! thread to the JVM, query the host `BatteryManager`, and return:
//!
//!  | Rust stub (here)                | Java method (BATTERY-IMPL-2)                 | Returns                |
//!  |---------------------------------|-----------------------------------------------|------------------------|
//!  | [`jni_get_battery_level`]       | `int getBatteryLevel()`                       | 0..100                 |
//!  | [`jni_get_battery_status`]      | `int getBatteryStatus()`                      | 1..4 (see [`BatteryStatus`]) |
//!  | [`jni_get_battery_voltage`]     | `int getBatteryVoltage()`                     | mV                     |
//!  | [`jni_get_battery_temperature`] | `int getBatteryTemperature()`                 | 1/10 °C                |
//!
//! # Threading
//!
//! One background thread (`kr64-battery-refresh`) loops:
//! refresh files → sleep 30 s (in 1 s ticks so a shutdown signal is
//! observed within ~1 s). No accept thread, no worker pool — there's
//! no inbound connection to accept. The thread is owned by
//! [`BatteryDeviceHandle`] and is joined on drop.
//!
//! # Why no `uevent`?
//!
//! A real Linux battery driver pushes a `CHANGE` uevent on
//! `power_supply` sysfs writes; the guest's `health@2.0` HAL listens
//! via netlink and re-reads the files. Twoyi doesn't yet emulate
//! netlink (see task NETLINK-1 in `DEVELOPMENT_ROADMAP.md` §3.1), so
//! the guest polls the files directly. The 30 s refresh interval is
//! comfortably within the guest's typical 1-minute poll cadence, so
//! `dumpsys battery` reflects host changes within ~1 minute.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

// Crate-local logging macros (defined in lib.rs) — no external `log` crate.
#[allow(unused_imports)]
use crate::{error, info, warning};

// ============================================================================
// Constants
// ============================================================================

/// Path of the battery sysfs tree *inside the rootfs* (relative).
pub const BATTERY_DIR_REL: &str = "sys/class/power_supply/battery";

/// Refresh interval for the periodic update thread (seconds). 30 s
/// matches the spec and is well within the guest's typical poll
/// cadence (1 minute); shorter would burn CPU for no perceptual gain.
pub const BATTERY_REFRESH_INTERVAL_SECS: u64 = 30;

/// Default capacity written at startup (percent). A real device is
/// rarely exactly 100% — 75 is a believable "mid-charge" value that
/// won't make `dumpsys battery` look suspiciously pegged.
pub const DEFAULT_CAPACITY: u32 = 75;

/// Default voltage (mV). 4 200 mV = a typical Li-ion full-charge
/// resting voltage. Matches the value VM's `BatteryService` returns
/// when the host `BatteryManager` is unavailable.
pub const DEFAULT_VOLTAGE_MV: u32 = 4_200;

/// Default temperature (1/10 °C). 28.0 °C is a typical idle battery
/// temperature.
pub const DEFAULT_TEMP_DECIC: u32 = 280;

/// Default technology string.
pub const DEFAULT_TECHNOLOGY: &str = "Li-ion";

/// Default health string. Linux `power_supply` ABI values are:
/// `Good`/`Dead`/`Overheat`/`Over voltage`/`Unspecified failure`/
/// `Unknown`/`Cold`.
pub const DEFAULT_HEALTH: &str = "Good";

// ---- JNI status byte values (mirror `BatteryManager` constants) ------

/// JNI status byte for "Charging".
pub const JNI_STATUS_CHARGING: u8 = 1;
/// JNI status byte for "Discharging".
pub const JNI_STATUS_DISCHARGING: u8 = 2;
/// JNI status byte for "Full" (not charging, battery full).
pub const JNI_STATUS_FULL: u8 = 3;
/// JNI status byte for "Not charging" (plugged in but not charging —
/// distinct from `Full`).
pub const JNI_STATUS_NOT_CHARGING: u8 = 4;

// ============================================================================
// BatteryStatus enum
// ============================================================================

/// Battery charging status. `#[repr(u8)]` so `as u8` gives the exact
/// JNI byte (matches `android.os.BatteryManager`'s constants
/// `BATTERY_STATUS_CHARGING` = 1, etc.).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    /// Plugged in and actively charging.
    Charging      = JNI_STATUS_CHARGING,
    /// On battery power, level dropping.
    Discharging   = JNI_STATUS_DISCHARGING,
    /// Plugged in, battery at 100%.
    Full          = JNI_STATUS_FULL,
    /// Plugged in but not charging (e.g. too hot, or charger undervolted).
    NotCharging   = JNI_STATUS_NOT_CHARGING,
}

impl BatteryStatus {
    /// Parse a raw JNI status byte. Returns `None` for unknown values.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            JNI_STATUS_CHARGING    => Some(Self::Charging),
            JNI_STATUS_DISCHARGING => Some(Self::Discharging),
            JNI_STATUS_FULL        => Some(Self::Full),
            JNI_STATUS_NOT_CHARGING => Some(Self::NotCharging),
            _ => None,
        }
    }

    /// The Linux `power_supply` ABI string for this status (written
    /// verbatim to the `status` file).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Charging     => "Charging",
            Self::Discharging  => "Discharging",
            Self::Full         => "Full",
            // The Linux ABI uses the two-word form "Not charging".
            Self::NotCharging  => "Not charging",
        }
    }

    /// True if this status implies the battery is being charged
    /// (used to derive the `charging` 0/1 file).
    pub fn is_charging(self) -> bool {
        matches!(self, Self::Charging)
    }
}

// ============================================================================
// BatteryDevice — owns the sysfs tree, spawns the refresh thread
// ============================================================================

/// The virtual `/sys/class/power_supply/battery` directory tree.
///
/// Created by [`BatteryDevice::new`]. Call [`BatteryDevice::spawn`]
/// to start the periodic refresh thread (consuming `self`); the
/// returned [`BatteryDeviceHandle`] owns the running thread and will
/// shut it down on drop.
///
/// Unlike [`crate::audio::AudioDevice`] / [`crate::sensors::SensorDevice`]
/// there is no listener to take — battery is pure sysfs, so the only
/// piece of state the Handle owns is the shutdown flag + thread
/// `JoinHandle`.
pub struct BatteryDevice {
    /// Absolute path to `{rootfs}/sys/class/power_supply/battery`.
    dir: PathBuf,
    /// Shutdown flag shared with the refresh thread + the handle.
    shutdown: Arc<AtomicBool>,
}

impl BatteryDevice {
    /// Materialise the sysfs tree under `{rootfs}/sys/class/power_supply/battery/`
    /// and write default values to all seven files.
    ///
    /// Idempotent — calling it twice on the same rootfs simply
    /// overwrites the files (no `EEXIST` errors). Missing parent
    /// directories (`sys/`, `sys/class/`, `sys/class/power_supply/`)
    /// are created on demand with mode 0755; the battery dir itself
    /// is created with mode 0755; the seven files are created with
    /// mode 0644 (world-readable so the guest's `system_server` can
    /// read them regardless of its uid inside the chroot).
    pub fn new(rootfs: &str) -> std::io::Result<Self> {
        let dir = Path::new(rootfs).join(BATTERY_DIR_REL);

        // Create the full directory chain (mkdir -p).
        fs::create_dir_all(&dir)?;
        // 0755 on the battery dir itself (and any parent we created).
        let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o755));

        let dev = Self {
            dir,
            shutdown: Arc::new(AtomicBool::new(false)),
        };

        // Write the default values immediately so a guest that opens
        // the files before the first refresh tick still sees sane
        // values. Idempotent — overwrites any pre-existing content.
        dev.refresh()?;

        info!(
            "[KR64][battery] materialised {} with default values (capacity={}%, status={})",
            dev.dir.display(),
            DEFAULT_CAPACITY,
            BatteryStatus::Discharging.as_str(),
        );

        Ok(dev)
    }

    /// Absolute path to the battery sysfs directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Absolute path to one of the seven files inside the battery dir.
    fn file(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// Write `value` to the named file (0644, ASCII + trailing newline).
    /// Delegates to the free function [`write_file_at`] so the refresh
    /// thread can reuse the exact same logic without owning a
    /// `BatteryDevice`.
    fn write_file(&self, name: &str, value: &str) -> std::io::Result<()> {
        write_file_at(&self.dir, name, value)
    }

    /// Read the first line of a file (trimmed), used by the `read_*`
    /// methods (mostly for tests, but also useful for diagnostics).
    fn read_file_trimmed(&self, name: &str) -> std::io::Result<String> {
        let mut f = fs::File::open(self.file(name))?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        Ok(s.trim().to_string())
    }

    // ---- per-file update methods -------------------------------------

    /// Update the `capacity` file (0-100 percent). Values outside that
    /// range are clamped (writing `150` would be a guest-visible lie
    /// about battery health).
    pub fn update_capacity(&self, pct: u32) -> std::io::Result<()> {
        let clamped = pct.min(100);
        self.write_file("capacity", &clamped.to_string())
    }

    /// Update the `status` file with the Linux ABI string for `status`.
    pub fn update_status(&self, status: BatteryStatus) -> std::io::Result<()> {
        self.write_file("status", status.as_str())
    }

    /// Update the `charging` file (`1` if Charging, else `0`). Derived
    /// from `status` rather than passed in, so callers can't put the
    /// two files out of sync.
    pub fn update_charging(&self, status: BatteryStatus) -> std::io::Result<()> {
        self.write_file("charging", if status.is_charging() { "1" } else { "0" })
    }

    /// Update the `voltage` file (millivolts, matching the unit
    /// returned by `jni_get_battery_voltage`).
    pub fn update_voltage(&self, mv: u32) -> std::io::Result<()> {
        self.write_file("voltage_now", &mv.to_string())
    }

    /// Update the `temperature` file (1/10 °C, matching the unit
    /// returned by `jni_get_battery_temperature`).
    pub fn update_temperature(&self, decic: u32) -> std::io::Result<()> {
        self.write_file("temp", &decic.to_string())
    }

    /// Update the `technology` file (e.g. `Li-ion`, `Li-poly`).
    pub fn update_technology(&self, tech: &str) -> std::io::Result<()> {
        self.write_file("technology", tech)
    }

    /// Update the `health` file (Linux ABI string, e.g. `Good`,
    /// `Overheat`, `Dead`).
    pub fn update_health(&self, health: &str) -> std::io::Result<()> {
        self.write_file("health", health)
    }

    // ---- read-back methods (mostly for tests) ------------------------

    /// Read the `capacity` file back as a `u32`.
    pub fn read_capacity(&self) -> std::io::Result<u32> {
        self.read_file_trimmed("capacity")?
            .parse::<u32>()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Read the `status` file back as a [`BatteryStatus`] (returns
    /// `InvalidData` if the file contains an unknown string).
    pub fn read_status(&self) -> std::io::Result<BatteryStatus> {
        let s = self.read_file_trimmed("status")?;
        [BatteryStatus::Charging, BatteryStatus::Discharging,
         BatteryStatus::Full, BatteryStatus::NotCharging]
            .into_iter()
            .find(|st| st.as_str() == s)
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown status string: {}", s),
            ))
    }

    /// Read the `voltage` file back as a `u32` (mV).
    pub fn read_voltage(&self) -> std::io::Result<u32> {
        self.read_file_trimmed("voltage_now")?
            .parse::<u32>()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Read the `temperature` file back as a `u32` (1/10 °C).
    pub fn read_temperature(&self) -> std::io::Result<u32> {
        self.read_file_trimmed("temp")?
            .parse::<u32>()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Refresh all seven files from the (stubbed) JNI up-calls.
    ///
    /// Called once at `new()` time and then every
    /// [`BATTERY_REFRESH_INTERVAL_SECS`] seconds by the refresh
    /// thread. Delegates to the free function [`refresh_dir`] so the
    /// refresh thread can call the same logic without owning a
    /// `BatteryDevice`.
    pub fn refresh(&self) -> std::io::Result<()> {
        refresh_dir(&self.dir)
    }

    /// Spawn the periodic refresh thread, consuming `self`.
    ///
    /// Returns a [`BatteryDeviceHandle`] that holds the shutdown flag
    /// and the refresh thread's `JoinHandle`. When the handle is
    /// dropped, the shutdown flag is set and the thread is joined.
    pub fn spawn(self) -> std::io::Result<BatteryDeviceHandle> {
        let dir = self.dir.clone();
        let shutdown_for_thread = Arc::clone(&self.shutdown);
        let shutdown_for_handle = Arc::clone(&self.shutdown);

        let thread = thread::Builder::new()
            .name("kr64-battery-refresh".to_string())
            .spawn(move || {
                info!(
                    "[KR64][battery] refresh thread started (interval={}s, dir={})",
                    BATTERY_REFRESH_INTERVAL_SECS, dir.display(),
                );
                // Initial refresh was already done by `new()`, so we
                // sleep first then refresh on each tick.
                while !shutdown_for_thread.load(Ordering::Acquire) {
                    // Sleep in 1 s ticks so a shutdown signal is
                    // observed within ~1 s (rather than waiting up to
                    // 30 s for the next wake).
                    for _ in 0..BATTERY_REFRESH_INTERVAL_SECS {
                        if shutdown_for_thread.load(Ordering::Acquire) {
                            break;
                        }
                        thread::sleep(Duration::from_secs(1));
                    }
                    if shutdown_for_thread.load(Ordering::Acquire) {
                        break;
                    }
                    if let Err(e) = refresh_dir(&dir) {
                        warning!("[KR64][battery] refresh failed: {}", e);
                    }
                }
                info!("[KR64][battery] refresh thread exiting");
            })?;

        Ok(BatteryDeviceHandle {
            shutdown: shutdown_for_handle,
            thread: Some(thread),
        })
    }
}

// ============================================================================
// Free helpers — shared between `BatteryDevice` methods and the refresh
// thread's closure. Lifting these out of the `impl` block lets the
// refresh thread (which owns only a `PathBuf`, not a `BatteryDevice`)
// call the exact same write logic.
// ============================================================================

/// Write `value` (plus trailing newline) to `{dir}/{name}`, creating
/// the file if missing and forcing mode 0644 so a stale 0600 file
/// from a previous run is corrected. Used by every `update_*` method.
fn write_file_at(dir: &Path, name: &str, value: &str) -> std::io::Result<()> {
    let path = dir.join(name);
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)?;
    let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o644));
    writeln!(f, "{}", value)
}

/// Refresh all seven battery files from the (stubbed) JNI up-calls.
/// Called once at `BatteryDevice::new()` time and then every
/// [`BATTERY_REFRESH_INTERVAL_SECS`] seconds by the refresh thread.
///
/// Returns the *first* I/O error encountered (subsequent files are
/// still attempted — a partial refresh is better than none, and the
/// next tick will retry the failed file).
fn refresh_dir(dir: &Path) -> std::io::Result<()> {
    let level = jni_get_battery_level();
    let status = BatteryStatus::from_u8(jni_get_battery_status())
        .unwrap_or(BatteryStatus::Discharging);
    let voltage = jni_get_battery_voltage();
    let temp = jni_get_battery_temperature();

    let mut first_err: Option<std::io::Error> = None;
    macro_rules! try_write {
        ($name:expr, $val:expr) => {
            if let Err(e) = write_file_at(dir, $name, $val) {
                if first_err.is_none() { first_err = Some(e); }
            }
        };
    }
    try_write!("capacity",    &level.min(100).to_string());
    try_write!("status",      status.as_str());
    try_write!("charging",    if status.is_charging() { "1" } else { "0" });
    try_write!("voltage_now",     &voltage.to_string());
    try_write!("temp", &temp.to_string());
    try_write!("technology",  DEFAULT_TECHNOLOGY);
    try_write!("health",      DEFAULT_HEALTH);

    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

// ============================================================================
// BatteryDeviceHandle — owns the refresh thread, joins on drop
// ============================================================================

/// Handle to a running battery refresh thread. Dropping this sets the
/// shutdown flag and joins the thread.
///
/// Created by [`BatteryDevice::spawn`]. The refresh thread keeps
/// running until either the handle is dropped or
/// [`BatteryDeviceHandle::shutdown`] is called.
pub struct BatteryDeviceHandle {
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl BatteryDeviceHandle {
    /// Ask the refresh thread to shut down. (Does not join — that
    /// happens on drop.)
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    /// True if the refresh thread has been asked to shut down.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }
}

impl Drop for BatteryDeviceHandle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        // Note: we deliberately do NOT unlink the sysfs files on drop.
        // They persist across daemon restarts (a new `BatteryDevice::new`
        // will overwrite them via `fs::write`); removing them would
        // race with any guest process that has them open.
    }
}

// ============================================================================
// JNI up-call stubs.
//
// Each is a one-line no-op returning a plausible default. They're
// documented with the exact Java signature they'll need to invoke
// (see BATTERY-IMPL-2) so the follow-up task can fill them in without
// re-reading the analysis doc.
// ============================================================================

/// `BatteryService.getBatteryLevel() -> int` (0..100). Stubbed: returns
/// [`DEFAULT_CAPACITY`].
fn jni_get_battery_level() -> u32 {
    DEFAULT_CAPACITY
}

/// `BatteryService.getBatteryStatus() -> int` (1..4, see
/// [`BatteryStatus`]). Stubbed: returns [`JNI_STATUS_DISCHARGING`] —
/// "on battery power" is the most common real state and is also the
/// safest default (the guest won't try to throttle CPU for charging
/// thermal management, etc.).
fn jni_get_battery_status() -> u8 {
    JNI_STATUS_DISCHARGING
}

/// `BatteryService.getBatteryVoltage() -> int` (mV). Stubbed: returns
/// [`DEFAULT_VOLTAGE_MV`].
fn jni_get_battery_voltage() -> u32 {
    DEFAULT_VOLTAGE_MV
}

/// `BatteryService.getBatteryTemperature() -> int` (1/10 °C). Stubbed:
/// returns [`DEFAULT_TEMP_DECIC`].
fn jni_get_battery_temperature() -> u32 {
    DEFAULT_TEMP_DECIC
}

// ============================================================================
// Tests — pure-Rust, no Android/JNI deps, so they run on the host too.
// (cargo test --lib)
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::AtomicU64;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Each test gets a UNIQUE tmpdir so parallel tests don't collide.
    /// Mirrors the pattern in `audio.rs` / `sensors.rs` / `binder.rs`.
    fn tmpdir() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut p = env::temp_dir();
        p.push(format!("kr64-battery-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&p).unwrap();
        p.to_string_lossy().to_string()
    }

    /// Read a file's full contents (untrimmed) for format validation.
    fn read_raw(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    // -------- BatteryStatus enum --------------------------------------

    #[test]
    fn battery_status_from_u8_roundtrip() {
        assert_eq!(BatteryStatus::from_u8(JNI_STATUS_CHARGING),    Some(BatteryStatus::Charging));
        assert_eq!(BatteryStatus::from_u8(JNI_STATUS_DISCHARGING), Some(BatteryStatus::Discharging));
        assert_eq!(BatteryStatus::from_u8(JNI_STATUS_FULL),        Some(BatteryStatus::Full));
        assert_eq!(BatteryStatus::from_u8(JNI_STATUS_NOT_CHARGING),Some(BatteryStatus::NotCharging));
        assert_eq!(BatteryStatus::from_u8(0),  None);
        assert_eq!(BatteryStatus::from_u8(5),  None);
        assert_eq!(BatteryStatus::from_u8(255),None);
    }

    #[test]
    fn battery_status_repr_matches_jni_byte() {
        // `#[repr(u8)]` makes `as u8` give the exact JNI byte.
        assert_eq!(BatteryStatus::Charging     as u8, JNI_STATUS_CHARGING);
        assert_eq!(BatteryStatus::Discharging  as u8, JNI_STATUS_DISCHARGING);
        assert_eq!(BatteryStatus::Full         as u8, JNI_STATUS_FULL);
        assert_eq!(BatteryStatus::NotCharging  as u8, JNI_STATUS_NOT_CHARGING);
    }

    #[test]
    fn battery_status_as_str_matches_linux_abi() {
        // The four Linux `power_supply` ABI strings, exactly as the
        // guest's health HAL will compare them. "Not charging" is
        // intentionally two words.
        assert_eq!(BatteryStatus::Charging.as_str(),     "Charging");
        assert_eq!(BatteryStatus::Discharging.as_str(),  "Discharging");
        assert_eq!(BatteryStatus::Full.as_str(),         "Full");
        assert_eq!(BatteryStatus::NotCharging.as_str(),  "Not charging");
    }

    #[test]
    fn battery_status_is_charging_only_for_charging() {
        assert!( BatteryStatus::Charging.is_charging());
        assert!(!BatteryStatus::Discharging.is_charging());
        assert!(!BatteryStatus::Full.is_charging());
        assert!(!BatteryStatus::NotCharging.is_charging());
    }

    // -------- BatteryDevice::new --------------------------------------

    #[test]
    fn new_creates_dir_and_all_seven_files() {
        let rootfs = tmpdir();
        let dev = BatteryDevice::new(&rootfs).expect("BatteryDevice::new");

        // The directory must exist.
        assert!(dev.dir().exists(), "battery dir should exist");

        // All seven files must exist with non-empty content.
        for name in ["capacity", "status", "charging", "voltage_now",
                     "temp", "technology", "health"] {
            let p = dev.dir().join(name);
            assert!(p.exists(), "file {} should exist", name);
            let content = fs::read_to_string(&p).unwrap();
            assert!(!content.is_empty(), "file {} should be non-empty", name);
        }

        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn new_creates_nested_sys_class_dirs_if_missing() {
        let rootfs = tmpdir();
        // {rootfs}/sys shouldn't exist yet — BatteryDevice::new must
        // create the entire chain sys/class/power_supply/battery.
        assert!(!Path::new(&format!("{}/sys", rootfs)).exists());
        let dev = BatteryDevice::new(&rootfs).expect("new");
        assert!(dev.dir().exists());
        assert!(Path::new(&format!("{}/sys/class/power_supply", rootfs)).exists());
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn new_is_idempotent() {
        // Calling `new` twice on the same rootfs must not error —
        // it just overwrites the files with defaults.
        let rootfs = tmpdir();
        let dev1 = BatteryDevice::new(&rootfs).expect("first new");
        // Mutate a file so we can detect the overwrite.
        dev1.update_capacity(42).unwrap();
        assert_eq!(dev1.read_capacity().unwrap(), 42);

        let dev2 = BatteryDevice::new(&rootfs).expect("second new");
        assert_eq!(dev2.read_capacity().unwrap(), DEFAULT_CAPACITY);

        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn new_writes_default_values_with_trailing_newline() {
        // The Linux `power_supply` ABI convention is a trailing
        // newline; Android's readers trim it. Verify our format.
        let rootfs = tmpdir();
        let dev = BatteryDevice::new(&rootfs).expect("new");

        assert_eq!(read_raw(&dev.file("capacity")),    format!("{}\n", DEFAULT_CAPACITY));
        assert_eq!(read_raw(&dev.file("voltage_now")),     format!("{}\n", DEFAULT_VOLTAGE_MV));
        assert_eq!(read_raw(&dev.file("temp")), format!("{}\n", DEFAULT_TEMP_DECIC));
        assert_eq!(read_raw(&dev.file("status")),      format!("{}\n", BatteryStatus::Discharging.as_str()));
        assert_eq!(read_raw(&dev.file("charging")),    "0\n");
        assert_eq!(read_raw(&dev.file("technology")),  format!("{}\n", DEFAULT_TECHNOLOGY));
        assert_eq!(read_raw(&dev.file("health")),      format!("{}\n", DEFAULT_HEALTH));

        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- per-file update methods ---------------------------------

    #[test]
    fn update_capacity_clamps_above_100() {
        let rootfs = tmpdir();
        let dev = BatteryDevice::new(&rootfs).unwrap();
        dev.update_capacity(150).unwrap();
        assert_eq!(dev.read_capacity().unwrap(), 100);
        // The on-disk value must also be clamped, not just the
        // read-back value.
        assert_eq!(read_raw(&dev.file("capacity")), "100\n");
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn update_capacity_accepts_zero() {
        let rootfs = tmpdir();
        let dev = BatteryDevice::new(&rootfs).unwrap();
        dev.update_capacity(0).unwrap();
        assert_eq!(dev.read_capacity().unwrap(), 0);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn update_status_writes_each_variant() {
        let rootfs = tmpdir();
        let dev = BatteryDevice::new(&rootfs).unwrap();
        for st in [BatteryStatus::Charging, BatteryStatus::Discharging,
                   BatteryStatus::Full, BatteryStatus::NotCharging] {
            dev.update_status(st).unwrap();
            assert_eq!(dev.read_status().unwrap(), st);
            assert_eq!(read_raw(&dev.file("status")), format!("{}\n", st.as_str()));
        }
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn update_charging_derives_from_status() {
        // `charging` must be `1` iff status == Charging, so the two
        // files never disagree.
        let rootfs = tmpdir();
        let dev = BatteryDevice::new(&rootfs).unwrap();

        dev.update_charging(BatteryStatus::Charging).unwrap();
        assert_eq!(read_raw(&dev.file("charging")), "1\n");

        for st in [BatteryStatus::Discharging, BatteryStatus::Full, BatteryStatus::NotCharging] {
            dev.update_charging(st).unwrap();
            assert_eq!(read_raw(&dev.file("charging")), "0\n",
                "charging should be 0 for {:?}", st);
        }
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn update_voltage_and_temperature_roundtrip() {
        let rootfs = tmpdir();
        let dev = BatteryDevice::new(&rootfs).unwrap();
        dev.update_voltage(4_350).unwrap();
        dev.update_temperature(315).unwrap(); // 31.5 °C
        assert_eq!(dev.read_voltage().unwrap(), 4_350);
        assert_eq!(dev.read_temperature().unwrap(), 315);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn update_technology_and_health_write_arbitrary_strings() {
        let rootfs = tmpdir();
        let dev = BatteryDevice::new(&rootfs).unwrap();
        dev.update_technology("Li-poly").unwrap();
        dev.update_health("Overheat").unwrap();
        assert_eq!(read_raw(&dev.file("technology")), "Li-poly\n");
        assert_eq!(read_raw(&dev.file("health")),     "Overheat\n");
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- read_status validation ----------------------------------

    #[test]
    fn read_status_rejects_unknown_string() {
        let rootfs = tmpdir();
        let dev = BatteryDevice::new(&rootfs).unwrap();
        // Hand-corrupt the status file.
        fs::write(dev.file("status"), "Bogus\n").unwrap();
        let r = dev.read_status();
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- refresh -------------------------------------------------

    #[test]
    fn refresh_writes_all_seven_files_from_jni_stubs() {
        // Since the JNI stubs return DEFAULT_* constants, refresh()
        // must produce exactly the same files as `new` did.
        let rootfs = tmpdir();
        let dev = BatteryDevice::new(&rootfs).unwrap();
        // Mutate everything so refresh has work to do.
        dev.update_capacity(1).unwrap();
        dev.update_voltage(1).unwrap();
        dev.update_temperature(1).unwrap();
        dev.update_status(BatteryStatus::Full).unwrap();
        dev.update_charging(BatteryStatus::Full).unwrap();
        dev.update_technology("X").unwrap();
        dev.update_health("X").unwrap();

        dev.refresh().unwrap();

        assert_eq!(dev.read_capacity().unwrap(),    DEFAULT_CAPACITY);
        assert_eq!(dev.read_voltage().unwrap(),     DEFAULT_VOLTAGE_MV);
        assert_eq!(dev.read_temperature().unwrap(), DEFAULT_TEMP_DECIC);
        assert_eq!(dev.read_status().unwrap(),      BatteryStatus::Discharging);
        assert_eq!(read_raw(&dev.file("charging")),   "0\n");
        assert_eq!(read_raw(&dev.file("technology")), format!("{}\n", DEFAULT_TECHNOLOGY));
        assert_eq!(read_raw(&dev.file("health")),     format!("{}\n", DEFAULT_HEALTH));
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- spawn + Drop --------------------------------------------

    #[test]
    fn spawn_then_drop_joins_cleanly() {
        // The refresh thread must observe the shutdown flag within
        // ~1 s (we sleep in 1 s ticks, not 30 s) and exit cleanly.
        let rootfs = tmpdir();
        let dev = BatteryDevice::new(&rootfs).unwrap();
        let handle = dev.spawn().expect("spawn");
        // Give the thread a moment to enter its sleep loop.
        std::thread::sleep(Duration::from_millis(100));
        assert!(!handle.is_shutdown());
        drop(handle); // must return within ~1 s, not 30 s.
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn spawn_refreshes_files_in_background() {
        // `spawn` consumes the `BatteryDevice`, so we read the files
        // back directly from disk to verify the refresh thread hasn't
        // corrupted them. (We can't change
        // BATTERY_REFRESH_INTERVAL_SECS at runtime to force an
        // immediate refresh tick, so this test verifies the *initial*
        // state written by `new()` remains consistent while the
        // thread is running — i.e., the thread doesn't trample the
        // files on startup.)
        let rootfs = tmpdir();
        let dev = BatteryDevice::new(&rootfs).unwrap();
        let dir = dev.dir().to_path_buf();
        let handle = dev.spawn().expect("spawn");
        std::thread::sleep(Duration::from_millis(50));
        // Files must still be readable while the thread is running.
        assert_eq!(fs::read_to_string(dir.join("capacity")).unwrap().trim(),
                   DEFAULT_CAPACITY.to_string());
        assert_eq!(fs::read_to_string(dir.join("status")).unwrap().trim(),
                   BatteryStatus::Discharging.as_str());
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- JNI stubs return documented defaults -------------------

    #[test]
    fn jni_stubs_return_documented_defaults() {
        assert_eq!(jni_get_battery_level(),       DEFAULT_CAPACITY);
        assert_eq!(jni_get_battery_status(),      JNI_STATUS_DISCHARGING);
        assert_eq!(jni_get_battery_voltage(),     DEFAULT_VOLTAGE_MV);
        assert_eq!(jni_get_battery_temperature(), DEFAULT_TEMP_DECIC);
    }
}
