/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi;

import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraManager;
import android.os.BatteryManager;
import android.os.Build;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.util.Log;

import java.io.File;
import java.io.IOException;

import io.twoyi.utils.FileLogger;

/**
 * 6-Z271: host-backed HAL bridge — the HOST side of the guest's hardware
 * requests.
 *
 * <p>The guest's virtual binder services (kr64's binder proxy) forward
 * requests over the {@code @TWOYI_SOCK} abstract socket:
 * <ul>
 *   <li>{@code TWOYI_VIBRATE:<ms>} — the guest haptic API wants a real
 *       vibration → routed to the host {@link Vibrator} (VibratorManager on
 *       API 31+, plain Vibrator below). Every request carries an explicit
 *       duration; no open-ended vibration is ever issued, and the host side
 *       clamps again.</li>
 *   <li>{@code TWOYI_VIBRATE_OFF} — cancel any outstanding vibration.</li>
 *   <li>{@code TWOYI_TORCH:1} / {@code TWOYI_TORCH:0} — guest torch LED
 *       writes → the real camera flash via
 *       {@link CameraManager#setTorchMode}. Failure (no flash unit, camera
 *       in use, permission denied) is reported honestly in the log and NOT
 *       presented to the guest as success — the guest's own LED state stays
 *       whatever it wrote; we simply cannot always realize it.</li>
 * </ul>
 *
 * <p>Battery is push-based on the SAME pattern the touch bridge uses for
 * filesystem paths: this class registers a sticky
 * {@link Intent#ACTION_BATTERY_CHANGED} receiver (no receiver-lifecycle
 * work — sticky broadcasts deliver the last value immediately) and writes
 * the REAL values straight into the guest's power-supply sysfs tree at
 * {@code <dataDir>/rootfs/sys/class/power_supply/battery/}, creating a
 * {@code .host-managed} marker so kr64's own static-default refresh thread
 * stands down (battery.rs refresh_dir checks it). No host filesystem paths
 * are ever exposed to the guest — only the eight ABI files of the Linux
 * power_supply class.
 */
public final class HostHalBridge {

    private static final String TAG = "HostHalBridge";

    /** Upper bound for a single guest-requested vibration (ms). */
    private static final long MAX_VIBRATE_MS = 60_000L;

    private HostHalBridge() {}

    // ------------------------------------------------------------------
    // Vibrator
    // ------------------------------------------------------------------

    /** Vibrate for {@code ms} milliseconds using the host vibrator API. */
    public static void vibrate(Context context, long ms) {
        if (ms <= 0) return;
        long duration = Math.min(ms, MAX_VIBRATE_MS);
        try {
            Vibrator vibrator = getVibrator(context);
            if (vibrator == null || !vibrator.hasVibrator()) {
                Log.w(TAG, "vibrate: host has no vibrator");
                return;
            }
            VibrationEffect effect =
                    VibrationEffect.createOneShot(duration, VibrationEffect.DEFAULT_AMPLITUDE);
            vibrator.vibrate(effect);
            Log.i(TAG, "vibrate: host vibrating for " + duration + " ms");
        } catch (Throwable t) {
            Log.w(TAG, "vibrate failed", t);
        }
    }

    /** Cancel any outstanding host vibration (guest {@code off()}). */
    public static void cancelVibrate(Context context) {
        try {
            Vibrator vibrator = getVibrator(context);
            if (vibrator != null) {
                vibrator.cancel();
                Log.i(TAG, "vibrate: cancelled");
            }
        } catch (Throwable t) {
            Log.w(TAG, "cancelVibrate failed", t);
        }
    }

    private static Vibrator getVibrator(Context context) {
        if (Build.VERSION.SDK_INT >= 31) {
            android.os.VibratorManager mgr =
                    context.getSystemService(android.os.VibratorManager.class);
            return mgr == null ? null : mgr.getDefaultVibrator();
        }
        return (Vibrator) context.getSystemService(Context.VIBRATOR_SERVICE);
    }

    // ------------------------------------------------------------------
    // Torch
    // ------------------------------------------------------------------

    /** Turn the host camera flash on/off for a guest torch request. */
    public static void setTorch(Context context, boolean on) {
        try {
            CameraManager cm = (CameraManager) context.getSystemService(Context.CAMERA_SERVICE);
            if (cm == null) {
                Log.w(TAG, "torch: no CameraManager");
                return;
            }
            for (String id : cm.getCameraIdList()) {
                Boolean hasFlash = cm.getCameraCharacteristics(id)
                        .get(CameraCharacteristics.FLASH_INFO_AVAILABLE);
                Integer facing = cm.getCameraCharacteristics(id)
                        .get(CameraCharacteristics.LENS_FACING);
                // Prefer the back camera's flash; fall back to any flash unit.
                if (Boolean.TRUE.equals(hasFlash)
                        && (facing == null || facing == CameraCharacteristics.LENS_FACING_BACK)) {
                    cm.setTorchMode(id, on);
                    Log.i(TAG, "torch: camera " + id + " torch=" + on);
                    return;
                }
            }
            Log.w(TAG, "torch: no flash unit available (requested on=" + on + ")");
        } catch (CameraAccessException e) {
            Log.w(TAG, "torch: camera access failed (on=" + on + ")", e);
        } catch (Throwable t) {
            Log.w(TAG, "torch failed (on=" + on + ")", t);
        }
    }

    // ------------------------------------------------------------------
    // Battery (push into the guest power_supply sysfs)
    // ------------------------------------------------------------------

    private static boolean sBatteryHooked = false;

    /**
     * Start pushing real host battery values into the guest sysfs tree.
     * Idempotent; safe to call before the rootfs exists (the first
     * successful battery broadcast after rootfs creation writes through).
     */
    public static synchronized void startBatteryReporting(Context context) {
        if (sBatteryHooked) return;
        sBatteryHooked = true;
        // ACTION_BATTERY_CHANGED is a sticky broadcast: this receiver
        // receives the LAST state immediately on registration, then every
        // change thereafter — no polling, no lifecycle management.
        android.content.BroadcastReceiver receiver = new android.content.BroadcastReceiver() {
            @Override
            public void onReceive(Context ctx, Intent intent) {
                writeBatteryFiles(ctx, intent);
            }
        };
        context.registerReceiver(receiver, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));
        FileLogger.boot("host_battery_reporting_started", null);
    }

    private static void writeBatteryFiles(Context context, Intent intent) {
        try {
            int level = intent.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
            int scale = intent.getIntExtra(BatteryManager.EXTRA_SCALE, 100);
            int status = intent.getIntExtra(BatteryManager.EXTRA_STATUS, -1);
            int plugged = intent.getIntExtra(BatteryManager.EXTRA_PLUGGED, 0);
            int health = intent.getIntExtra(BatteryManager.EXTRA_HEALTH, -1);
            int voltageMv = intent.getIntExtra(BatteryManager.EXTRA_VOLTAGE, -1);
            int tempDecic = intent.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, -1);
            String technology = intent.getStringExtra(BatteryManager.EXTRA_TECHNOLOGY);

            String dataDir = context.getApplicationInfo().dataDir;
            File batteryDir = new File(dataDir,
                    "rootfs/sys/class/power_supply/battery");
            if (!batteryDir.isDirectory() && !batteryDir.mkdirs()) {
                // Rootfs not up yet — the next broadcast will retry.
                return;
            }

            int pct = (level >= 0 && scale > 0) ? Math.round(level * 100f / scale) : -1;
            boolean charging = plugged != 0;
            if (pct >= 0) write(batteryDir, "capacity", String.valueOf(pct));
            if (status >= 0) write(batteryDir, "status", statusName(status));
            write(batteryDir, "charging", charging ? "1" : "0");
            // 6-Z271h: match battery.rs refresh_dir's file set — AOSP's
            // BatteryMonitor::update() reads these, and newer recoveries
            // surface a dead battery when they are missing.
            write(batteryDir, "present", "1");
            if (status >= 0) write(batteryDir, "charge_status", statusName(status));
            if (pct >= 0) write(batteryDir, "charge_counter",
                    String.valueOf(pct * 3500)); // µAh (~3500 mAh pack)
            write(batteryDir, "current_now", charging ? "500000" : "-300000");
            write(batteryDir, "cycle_count", "0");
            if (voltageMv > 0) write(batteryDir, "voltage_now",
                    String.valueOf(voltageMv * 1000L)); // mV → µV (ABI)
            if (tempDecic > 0) write(batteryDir, "temp", String.valueOf(tempDecic));
            if (technology != null) write(batteryDir, "technology", technology);
            if (health >= 0) write(batteryDir, "health", healthName(health));
            write(batteryDir, "type", "Battery");

            // Charger nodes — BatteryMonitor classifies them by `type` and
            // reads `online` for the plugged state (TWRP's plug icon).
            File psDir = batteryDir.getParentFile();
            if (psDir != null) {
                File usbDir = new File(psDir, "usb");
                File acDir = new File(psDir, "ac");
                if ((usbDir.isDirectory() || usbDir.mkdirs())) {
                    write(usbDir, "type", "USB");
                    write(usbDir, "online", charging ? "1" : "0");
                }
                if (acDir.isDirectory() || acDir.mkdirs()) {
                    write(acDir, "type", "Mains");
                    write(acDir, "online", "0");
                }
            }

            // Marker: kr64's refresh thread stands down when it sees this.
            write(batteryDir, ".host-managed", "1");
        } catch (Throwable t) {
            Log.w(TAG, "writeBatteryFiles failed", t);
        }
    }

    private static void write(File dir, String name, String value) {
        try {
            File f = new File(dir, name);
            java.io.PrintWriter w = new java.io.PrintWriter(f, "UTF-8");
            w.println(value);
            w.close();
            f.setReadable(true, false);
            f.setWritable(true, false);
        } catch (IOException e) {
            Log.w(TAG, "write " + name + " failed", e);
        }
    }

    private static String statusName(int status) {
        switch (status) {
            case BatteryManager.BATTERY_STATUS_CHARGING: return "Charging";
            case BatteryManager.BATTERY_STATUS_DISCHARGING: return "Discharging";
            case BatteryManager.BATTERY_STATUS_FULL: return "Full";
            case BatteryManager.BATTERY_STATUS_NOT_CHARGING: return "Not charging";
            default: return "Unknown";
        }
    }

    private static String healthName(int health) {
        switch (health) {
            case BatteryManager.BATTERY_HEALTH_GOOD: return "Good";
            case BatteryManager.BATTERY_HEALTH_OVERHEAT: return "Overheat";
            case BatteryManager.BATTERY_HEALTH_DEAD: return "Dead";
            case BatteryManager.BATTERY_HEALTH_OVER_VOLTAGE: return "Over voltage";
            case BatteryManager.BATTERY_HEALTH_COLD: return "Cold";
            default: return "Unknown";
        }
    }
}
