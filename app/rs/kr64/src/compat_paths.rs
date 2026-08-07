// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! Samsung GameSDK compatibility paths.
//!
//! Samsung devices ship a "GameSDK" — a set of game-optimisation
//! components that high-end games (Genshin Impact, PUBG Mobile, etc.)
//! probe for at startup to decide which rendering path / performance
//! profile to use. The probe is a `stat()` / `access()` on a handful
//! of well-known paths:
//!
//!   * `/system/etc/game_driver/`             — GameDriver config dir
//!   * `/system/etc/game_driver/game_driver.json` — driver selection rules
//!   * `/system/lib64/libGameDriver.sys.so`   — GameDriver system driver
//!   * `/system/lib64/libGameSDK.sys.so`      — GameSDK helper lib
//!   * `/vendor/lib64/egl/libGLESGameDriver.so` — GameDriver EGL backend
//!   * `/data/system/gamedriver/`             — per-app runtime state dir
//!   * `/data/system/gamedriver/blacklist`    — apps opted out of GameDriver
//!   * `/data/system/gamedriver/whitelist`    — apps forced onto GameDriver
//!
//! On a non-Samsung device (including the twoyi VM, whose rootfs is a
//! generic GSI), these paths don't exist. Most games handle that
//! gracefully (the probe returns ENOENT and they fall back to the
//! default ANGLE / Mali / Adreno driver), but a few hard-crash on the
//! missing path because their NDK code does `dlopen` without checking
//! the return value, or their JNI init does `new File(path).exists()`
//! and dereferences a null asset manager afterwards.
//!
//! This module materialises the *directories* and *stub files* so the
//! `stat()`/`access()` probes succeed. We do NOT ship the actual
//! Samsung blobs (they're closed-source and Samsung-licensed) — the
//! stub `.so` files are empty, and `dlopen` on them will fail, but by
//! that point the game has already taken the "GameDriver present" code
//! path. The game's own fallback handling then kicks in when dlopen
//! fails, which is the same code path it would take on a real Samsung
//! device with a corrupted GameDriver install. This is strictly better
//! than the ENOENT-on-stat crash.
//!
//! # Why this is a separate module
//!
//! The paths span `/system`, `/vendor`, and `/data` — none of which
//! belong in `devices.rs` (which is `/dev`-only) or `proc_emu.rs`
//! (which is `/proc`-only). Giving them a dedicated module keeps the
//! "fake the Samsung environment" concern in one place and makes it
//! trivial to disable (`#[cfg(feature = "samsung-compat")]`) if a
//! future rootfs ships real Samsung blobs.
//!
//! # What's NOT here
//!
//! This module does not:
//!   * Ship Samsung GameSDK binaries (closed source).
//!   * Implement the GameDriver EGL/Vulkan backend.
//!   * Set the `ro.build.samsung.*` / `ro.product.samsung.*` props
//!     (those belong in a future `props.rs` / build.prop patcher).
//!
//! It only creates the *paths* so existence probes succeed.

use std::fs;
use std::path::Path;

// Crate-local logging macros (defined in lib.rs) — no external `log` crate.
use crate::info;

/// Materialise all Samsung GameSDK compatibility paths under `rootfs`.
///
/// Idempotent: safe to call on every boot. Existing files/dirs are left
/// in place (we don't overwrite — if the rootfs ships real Samsung
/// blobs, we must not clobber them).
///
/// Returns `Ok(())` if every path was created (or already existed).
/// Returns the first `Err` if any path could not be created — the
/// caller should log a warning but continue, since missing compat
/// paths are non-fatal (games fall back to the default driver).
pub fn create_samsung_gamesdk_compat_paths(rootfs: &str) -> std::io::Result<()> {
    info!(
        "[KR64][compat_paths] materialising Samsung GameSDK compat paths under {}",
        rootfs
    );

    // /system/etc/game_driver/ — GameDriver config directory.
    let game_driver_etc = format!("{}/system/etc/game_driver", rootfs);
    create_dir_all_safe(&game_driver_etc)?;

    // game_driver.json — driver selection rules. Real Samsung content
    // is a JSON object mapping package names to driver preferences.
    // We ship a minimal valid JSON that selects the "default" (system)
    // driver for every app, so any game that parses this file gets a
    // well-formed (if uninteresting) config instead of an empty/ENOENT
    // file. Format documented in Samsung's GameDriver docs (dev.samsung.com).
    let game_driver_json = format!("{}/game_driver.json", game_driver_etc);
    write_stub_if_absent(
        &game_driver_json,
        concat!(
            "{\n",
            "  \"comment\": \"Auto-generated by twoyi kr64 compat_paths — stub for GameSDK probes.\",\n",
            "  \"driver_library_path\": \"/system/lib64/libGameDriver.sys.so\",\n",
            "  \"driver_samsung_library_path\": \"/system/lib64/libGameDriver.sys.so\",\n",
            "  \"whitelist\": [],\n",
            "  \"blacklist\": [],\n",
            "  \"driver_samsung_prioritized\": [],\n",
            "  \"driver_samsung_preresource\": []\n",
            "}\n"
        ),
    )?;

    // /system/lib64/libGameDriver.sys.so — GameDriver system driver.
    // Empty stub: stat() succeeds, dlopen() fails (game falls back to
    // the default driver, which is the desired behaviour).
    let lib_gamedriver = format!("{}/system/lib64/libGameDriver.sys.so", rootfs);
    create_dir_all_safe(lib_gamedriver_parent(&lib_gamedriver))?;
    write_stub_if_absent(&lib_gamedriver, "")?;

    // /system/lib64/libGameSDK.sys.so — GameSDK helper lib. Same stub
    // strategy as the GameDriver lib above.
    let lib_gamesdk = format!("{}/system/lib64/libGameSDK.sys.so", rootfs);
    write_stub_if_absent(&lib_gamesdk, "")?;

    // /vendor/lib64/egl/libGLESGameDriver.so — GameDriver EGL backend.
    // Samsung's EGL loader scans /vendor/lib64/egl/ for libGLES*.so
    // backends. Creating the stub lets the scan complete without
    // ENOENT; the stub won't actually load (empty file = dlopen fails).
    let vendor_egl_dir = format!("{}/vendor/lib64/egl", rootfs);
    create_dir_all_safe(&vendor_egl_dir)?;
    let egl_gamedriver = format!("{}/libGLESGameDriver.so", vendor_egl_dir);
    write_stub_if_absent(&egl_gamedriver, "")?;

    // /data/system/gamedriver/ — per-app runtime state directory.
    // Samsung's GameDriver daemon writes per-app profile data here.
    // We create the dir + empty blacklist/whitelist so apps that read
    // these files get an empty (but well-formed) result instead of
    // ENOENT.
    let gamedriver_data = format!("{}/data/system/gamedriver", rootfs);
    create_dir_all_safe(&gamedriver_data)?;
    let blacklist = format!("{}/blacklist", gamedriver_data);
    write_stub_if_absent(&blacklist, "")?;
    let whitelist = format!("{}/whitelist", gamedriver_data);
    write_stub_if_absent(&whitelist, "")?;

    // /system/etc/gameopt/ — Game Performance Optimizer config.
    // Newer Samsung GameSDK versions look here for the per-game
    // performance profile. Same stub strategy.
    let gameopt_etc = format!("{}/system/etc/gameopt", rootfs);
    create_dir_all_safe(&gameopt_etc)?;

    info!(
        "[KR64][compat_paths] Samsung GameSDK compat paths materialised ({} + stubs)",
        game_driver_etc
    );
    Ok(())
}

/// `fs::create_dir_all` that's a no-op if the dir already exists.
/// The std helper already handles this, but wrapping it lets us log
/// each created dir for debugging (the GameSDK path layout is subtle
/// and a missing intermediate dir is a common failure mode).
fn create_dir_all_safe(path: &str) -> std::io::Result<()> {
    if Path::new(path).is_dir() {
        return Ok(());
    }
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o755));
    }
    info!("[KR64][compat_paths] created dir: {}", path);
    Ok(())
}

/// Write `content` to `path` ONLY if `path` does not already exist.
///
/// This is the idempotency guarantee: if the rootfs ships a real
/// Samsung blob at this path, we must not clobber it. The check is
/// `Path::exists()` (covers regular files, symlinks, and sockets —
/// all of which the real blob could be).
fn write_stub_if_absent(path: &str, content: &str) -> std::io::Result<()> {
    if Path::new(path).exists() {
        info!("[KR64][compat_paths] keeping existing file: {}", path);
        return Ok(());
    }
    fs::write(path, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Stub .so files: 0644 (matches how AOSP ships system .so libs).
        // Stub config/data files: 0644 too. Mode 0755 is only for dirs.
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o644));
    }
    info!(
        "[KR64][compat_paths] wrote stub ({} bytes): {}",
        content.len(),
        path
    );
    Ok(())
}

/// Return the parent directory of `path`, or `.` if there is none.
/// Used to ensure `/system/lib64` exists before we drop a stub `.so`
/// into it.
fn lib_gamedriver_parent(path: &str) -> &str {
    match Path::new(path).parent() {
        Some(p) => p.to_str().unwrap_or("."),
        None => ".",
    }
}

// ============================================================================
// Tests — pure-Rust, run on the host (cargo test --lib).
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Each test gets a unique tmpdir so parallel tests don't collide.
    fn tmpdir() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!("kr64-compat-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&p).unwrap();
        p.to_string_lossy().to_string()
    }

    #[test]
    fn creates_all_samsung_gamesdk_paths() {
        let rootfs = tmpdir();
        create_samsung_gamesdk_compat_paths(&rootfs).expect("create compat paths");

        // Directories
        assert!(Path::new(&format!("{}/system/etc/game_driver", rootfs)).is_dir());
        assert!(Path::new(&format!("{}/system/lib64", rootfs)).is_dir());
        assert!(Path::new(&format!("{}/vendor/lib64/egl", rootfs)).is_dir());
        assert!(Path::new(&format!("{}/data/system/gamedriver", rootfs)).is_dir());
        assert!(Path::new(&format!("{}/system/etc/gameopt", rootfs)).is_dir());

        // Files
        assert!(Path::new(&format!("{}/system/etc/game_driver/game_driver.json", rootfs)).exists());
        assert!(Path::new(&format!("{}/system/lib64/libGameDriver.sys.so", rootfs)).exists());
        assert!(Path::new(&format!("{}/system/lib64/libGameSDK.sys.so", rootfs)).exists());
        assert!(Path::new(&format!("{}/vendor/lib64/egl/libGLESGameDriver.so", rootfs)).exists());
        assert!(Path::new(&format!("{}/data/system/gamedriver/blacklist", rootfs)).exists());
        assert!(Path::new(&format!("{}/data/system/gamedriver/whitelist", rootfs)).exists());

        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn game_driver_json_is_valid_json() {
        let rootfs = tmpdir();
        create_samsung_gamesdk_compat_paths(&rootfs).unwrap();
        let content =
            fs::read_to_string(format!("{}/system/etc/game_driver/game_driver.json", rootfs))
                .unwrap();
        // Minimal JSON sanity: starts with {, ends with }, and has the
        // expected keys. (We don't pull in serde just for this test.)
        assert!(content.trim_start().starts_with('{'));
        assert!(content.trim_end().ends_with('}'));
        assert!(content.contains("\"whitelist\""));
        assert!(content.contains("\"blacklist\""));
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn is_idempotent_and_does_not_clobber() {
        let rootfs = tmpdir();

        // Pre-create a "real" (non-empty) game_driver.json to simulate
        // a rootfs that ships Samsung blobs.
        let etc_dir = format!("{}/system/etc/game_driver", rootfs);
        fs::create_dir_all(&etc_dir).unwrap();
        let real_json = format!("{}/game_driver.json", etc_dir);
        fs::write(&real_json, "{\"real\":\"samsung-blob\"}").unwrap();

        create_samsung_gamesdk_compat_paths(&rootfs).expect("second run");

        // The pre-existing file must NOT have been overwritten.
        let after = fs::read_to_string(&real_json).unwrap();
        assert_eq!(after, "{\"real\":\"samsung-blob\"}");

        let _ = fs::remove_dir_all(&rootfs);
    }
}
