//! Virtual haptics (vibrator) + backlight sysfs for the guest (§13/§16).
//!
//! Recoveries reach the vibrator and display backlight through STANDARD
//! Linux/Android sysfs interfaces — no recovery-specific code lives here,
//! only the well-known ABI surfaces:
//!
//! Legacy vibrator (Android ≤ 9 era, TWRP 2.x/3.x poll these on EVERY
//! page transition — a missing file costs 3+ syscalls through the hook's
//! retry ladder and the tracer, and old TWRP's UI watchdog eventually
//! re-execs from the resulting syscall storm):
//!   * `/sys/class/timed_output/vibrator/enable`      (write: ms)
//!   * `/sys/class/leds/vibrator/activate`            (write: 0/1)
//!   * `/sys/class/leds/vibrator/duration`            (write: ms)
//!   * `/sys/class/leds/vibrator/max_brightness`      (leds-style vibrator)
//!
//! Backlight (TWRP's TWFunc::Set_Brightness probes, in order):
//!   * `/sys/class/leds/lcd-backlight/brightness`     (write: 0..max)
//!   * `/sys/class/leds/lcd-backlight/max_brightness`
//!   * `/sys/class/backlight/panel/brightness`        (Find_File scan of
//!     `/sys/class/backlight` wants at least one subdir with `brightness`)
//!   * `/sys/class/backlight/panel/max_brightness`
//!   * `/sys/class/backlight/panel/bl_power`
//!
//! Implementation model mirrors [`crate::battery`]: plain files under
//! `{rootfs}/sys/...` created at boot, guest writes simply land in the
//! files. A virtual backend is acceptable in CI per the compatibility
//! spec — what matters is that the guest-visible open/read/write
//! behavior matches the real ABI. A drain thread zeroes `enable` and
//! `activate` after the requested duration so a subsequent read looks
//! like a real one-shot trigger (full ABI fidelity for recoveries that
//! read the state back).
//!
//! This module is deliberately recovery-agnostic: it implements the
//! LINUX/ANDROID environment, not any particular recovery's quirks.

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::info;

/// Where the legacy timed_output vibrator lives, relative to the rootfs.
pub const TIMED_OUTPUT_DIR_REL: &str = "sys/class/timed_output/vibrator";
/// Where the leds-style vibrator lives, relative to the rootfs.
pub const LEDS_VIBRATOR_DIR_REL: &str = "sys/class/leds/vibrator";
/// Where the lcd-backlight LED lives, relative to the rootfs.
pub const LEDS_BACKLIGHT_DIR_REL: &str = "sys/class/leds/lcd-backlight";
/// Where the panel backlight lives, relative to the rootfs (TWRP's
/// `/sys/class/backlight` Find_File scan).
pub const BACKLIGHT_DIR_REL: &str = "sys/class/backlight/panel";

/// Default maximum brightness (a common panel value).
pub const DEFAULT_MAX_BRIGHTNESS: u32 = 255;
/// Default active brightness written at boot.
pub const DEFAULT_BRIGHTNESS: u32 = 160;
/// Poll interval of the one-shot drain thread (ms).
const DRAIN_POLL_MS: u64 = 250;

// ============================================================================
// HapticsDevice
// ============================================================================

/// The virtual vibrator + backlight sysfs tree.
///
/// Created by [`HapticsDevice::new`] (idempotent); [`HapticsDevice::spawn`]
/// starts the one-shot drain thread and returns a handle that stops it on
/// drop.
pub struct HapticsDevice {
    timed_output_dir: PathBuf,
    leds_vibrator_dir: PathBuf,
    backlight_dirs: Vec<PathBuf>,
    shutdown: Arc<AtomicBool>,
}

impl HapticsDevice {
    /// Materialise every sysfs surface above under `rootfs`.
    pub fn new(rootfs: &str) -> io::Result<Self> {
        let timed_output_dir = Path::new(rootfs).join(TIMED_OUTPUT_DIR_REL);
        let leds_vibrator_dir = Path::new(rootfs).join(LEDS_VIBRATOR_DIR_REL);
        let leds_backlight_dir = Path::new(rootfs).join(LEDS_BACKLIGHT_DIR_REL);
        let panel_backlight_dir = Path::new(rootfs).join(BACKLIGHT_DIR_REL);

        for d in [
            &timed_output_dir,
            &leds_vibrator_dir,
            &leds_backlight_dir,
            &panel_backlight_dir,
        ] {
            fs::create_dir_all(d)?;
            let _ = fs::set_permissions(d, fs::Permissions::from_mode(0o755));
        }

        // Legacy timed_output vibrator: `enable` (write ms), plus the
        // state file kernel convention `state` (reads 0 when idle).
        write_file(&timed_output_dir, "enable", "0")?;
        write_file(&timed_output_dir, "state", "0")?;

        // leds-style vibrator.
        write_file(&leds_vibrator_dir, "activate", "0")?;
        write_file(&leds_vibrator_dir, "duration", "0")?;
        write_file(&leds_vibrator_dir, "max_brightness", "1")?;
        write_file(&leds_vibrator_dir, "brightness", "0")?;

        // lcd-backlight (TWRP's first TW_BRIGHTNESS_PATH probe).
        write_file(
            &leds_backlight_dir,
            "brightness",
            &DEFAULT_BRIGHTNESS.to_string(),
        )?;
        write_file(
            &leds_backlight_dir,
            "max_brightness",
            &DEFAULT_MAX_BRIGHTNESS.to_string(),
        )?;

        // panel backlight (TWRP's /sys/class/backlight Find_File scan).
        write_file(
            &panel_backlight_dir,
            "brightness",
            &DEFAULT_BRIGHTNESS.to_string(),
        )?;
        write_file(
            &panel_backlight_dir,
            "max_brightness",
            &DEFAULT_MAX_BRIGHTNESS.to_string(),
        )?;
        write_file(&panel_backlight_dir, "bl_power", "0")?;

        info!(
            "[KR64][haptics] materialised vibrator (timed_output + leds) \
             and backlight (lcd-backlight + panel) sysfs under {}",
            Path::new(rootfs).join("sys/class").display()
        );

        Ok(Self {
            timed_output_dir,
            leds_vibrator_dir,
            backlight_dirs: vec![leds_backlight_dir, panel_backlight_dir],
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Start the one-shot drain thread: a non-zero `enable`/`activate`
    /// value schedules a reset to 0 after that many milliseconds, which
    /// is what a real timed_output/vibrator does after the timeout.
    pub fn spawn(self) -> io::Result<HapticsDeviceHandle> {
        let shutdown = self.shutdown.clone();
        let timed_dir = self.timed_output_dir.clone();
        let leds_dir = self.leds_vibrator_dir.clone();
        let _backlights = self.backlight_dirs.clone();

        let join = thread::Builder::new()
            .name("kr64-haptics".into())
            .spawn(move || {
                loop {
                    if shutdown.load(Ordering::Relaxed) {
                        return;
                    }
                    // Read enable/duration; emulate the one-shot.
                    let _ = drain_one_shot(&timed_dir, "enable");
                    let _ = drain_one_shot(&leds_dir, "activate");
                    thread::sleep(Duration::from_millis(DRAIN_POLL_MS));
                }
            })?;
        Ok(HapticsDeviceHandle {
            shutdown: self.shutdown,
            _join: join,
        })
    }
}

/// Handle owning the drain thread; stops it on drop.
pub struct HapticsDeviceHandle {
    shutdown: Arc<AtomicBool>,
    _join: thread::JoinHandle<()>,
}

impl Drop for HapticsDeviceHandle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

// ============================================================================
// helpers
// ============================================================================

/// Write `value` into `dir/name` (0644, ASCII, trailing newline — the
/// sysfs convention). Truncating overwrite, like the real sysfs.
fn write_file(dir: &Path, name: &str, value: &str) -> io::Result<()> {
    let p = dir.join(name);
    fs::write(&p, format!("{}\n", value))?;
    let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o644));
    Ok(())
}

/// One-shot emulation: read `name` as a duration; if non-zero, schedule
/// the reset for after that duration (first tick past the deadline).
fn drain_one_shot(dir: &Path, name: &str) -> io::Result<()> {
    let p = dir.join(name);
    let raw = fs::read_to_string(&p).unwrap_or_else(|_| "0".into());
    let ms: u64 = raw.trim().parse().unwrap_or(0);
    if ms > 0 {
        // Busy-wait the duration (bounded: vibrates are ≤ a few seconds),
        // then reset. Sleeping on the haptics thread is safe — no guest
        // ever blocks on this file.
        thread::sleep(Duration::from_millis(ms));
        fs::write(&p, "0\n")?;
        // The companion state file reads 0 when idle.
        let state = dir.join("state");
        if state.exists() {
            fs::write(&state, "0\n")?;
        }
    }
    Ok(())
}

// ============================================================================
// tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("kr64-haptics-test-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn materialises_all_legacy_surfaces() {
        let root = tmpdir("surfaces");
        HapticsDevice::new(root.to_str().unwrap()).unwrap();
        for rel in [
            "sys/class/timed_output/vibrator/enable",
            "sys/class/timed_output/vibrator/state",
            "sys/class/leds/vibrator/activate",
            "sys/class/leds/vibrator/duration",
            "sys/class/leds/lcd-backlight/brightness",
            "sys/class/leds/lcd-backlight/max_brightness",
            "sys/class/backlight/panel/brightness",
            "sys/class/backlight/panel/max_brightness",
            "sys/class/backlight/panel/bl_power",
        ] {
            assert!(root.join(rel).exists(), "missing {}", rel);
        }
        // values are sane sysfs ASCII with trailing newline
        let b = fs::read_to_string(root.join("sys/class/leds/lcd-backlight/brightness")).unwrap();
        assert_eq!(b, format!("{}\n", DEFAULT_BRIGHTNESS));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn idempotent_recreate() {
        let root = tmpdir("idempotent");
        HapticsDevice::new(root.to_str().unwrap()).unwrap();
        // A guest write lands in the file...
        fs::write(root.join("sys/class/timed_output/vibrator/enable"), "250\n").unwrap();
        // ...and re-materialising resets it to the idle default.
        HapticsDevice::new(root.to_str().unwrap()).unwrap();
        let v = fs::read_to_string(root.join("sys/class/timed_output/vibrator/enable")).unwrap();
        assert_eq!(v, "0\n");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn drain_one_shot_resets_after_duration() {
        let root = tmpdir("drain");
        fs::create_dir_all(root.join("vib")).unwrap();
        fs::write(root.join("vib/enable"), "30\n").unwrap();
        drain_one_shot(&root.join("vib"), "enable").unwrap();
        let v = fs::read_to_string(root.join("vib/enable")).unwrap();
        assert_eq!(v, "0\n", "enable must reset after the duration");
        let _ = fs::remove_dir_all(&root);
    }
}
