/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi;

import android.app.Activity;
import android.app.ProgressDialog;
import android.content.ContentResolver;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.os.SystemClock;
import android.util.DisplayMetrics;
import android.util.Log;
import android.view.Display;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowManager;
import android.widget.FrameLayout;
import android.widget.TextView;
import android.widget.Toast;

import androidx.annotation.NonNull;

import com.cleveroad.androidmanimation.LoadingAnimationView;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.concurrent.TimeUnit;

import io.twoyi.utils.FileLogger;
import io.twoyi.utils.LogEvents;
import io.twoyi.utils.NavUtils;
import io.twoyi.utils.ProfileManager;
import io.twoyi.utils.ProfileSettings;
import io.twoyi.utils.RomManager;
import io.twoyi.utils.UIHelper;

/**
 * @author weishu
 * @date 2021/10/20.
 */
public class Render2Activity extends Activity implements View.OnTouchListener {

    private static final String TAG = "Render2Activity";
    private static final int REQUEST_SELECT_ROM = 1001;

    private SurfaceView mSurfaceView;

    private ViewGroup mRootView;
    private LoadingAnimationView mLoadingView;
    private TextView mLoadingText;
    private View mLoadingLayout;
    private View mBootLogView;

    private int mVirtualDisplayWidth;
    private int mVirtualDisplayHeight;
    private int mVirtualDisplayDpi;
    
    private int mSurfaceWidth;
    private int mSurfaceHeight;
    private int mSurfaceOffsetX;
    private int mSurfaceOffsetY;

    /**
     * Cached Matrix that maps SurfaceView-local touch coordinates to virtual
     * display coordinates. Pre-computed once in {@link #setupSurfaceViewLayout()}
     * so the per-event touch path doesn't allocate a new Matrix on every
     * MotionEvent (which can fire 100+ times/sec during a drag).
     */
    private final android.graphics.Matrix mTouchMatrix = new android.graphics.Matrix();


    private final SurfaceHolder.Callback mSurfaceCallback = new SurfaceHolder.Callback() {
        @Override
        public void surfaceCreated(@NonNull SurfaceHolder holder) {
            Surface surface = holder.getSurface();

            // Set the data directory BEFORE anything else — the Rust side
            // needs it to resolve rootfs, log, and input socket paths.
            // In a work profile, getDataDir() returns /data/user/<uid>/io.twoyi
            // instead of /data/data/io.twoyi.
            String dataDir = getApplicationInfo().dataDir;
            Renderer.setDataDir(dataDir);
            Log.i(TAG, "Data directory: " + dataDir);
            FileLogger.boot("renderer_set_data_dir", "dataDir=" + dataDir);
            FileLogger.i(TAG, "surfaceCreated: dataDir=" + dataDir);

            // Set the Boot Recovery (TWRP) flag BEFORE init() so the
            // native core knows whether to pass --boot-recovery to kr64.
            // The flag is read from the active profile's settings, so
            // the user can toggle it via Settings → Boot to Recovery (TWRP)
            // and the next container launch will boot TWRP instead of
            // full Android. The flag is stored per-profile, so each
            // profile can have its own boot mode.
            boolean bootRecovery = io.twoyi.utils.ProfileSettings.isBootRecoveryEnabled(getApplicationContext());
            Renderer.setBootRecovery(bootRecovery);
            Log.i(TAG, "Boot Recovery (TWRP) flag: " + bootRecovery);
            FileLogger.boot("boot_recovery_flag", "enabled=" + bootRecovery);

            // Calculate proper DPI based on physical screen and virtual display scaling
            WindowManager windowManager = getWindowManager();
            Display defaultDisplay = windowManager.getDefaultDisplay();
            DisplayMetrics displayMetrics = new DisplayMetrics();
            defaultDisplay.getRealMetrics(displayMetrics);
            
            // Calculate the scaling factor between physical and virtual display
            float scaleX = (float) displayMetrics.widthPixels / (float) mVirtualDisplayWidth;
            float scaleY = (float) displayMetrics.heightPixels / (float) mVirtualDisplayHeight;
            
            // Use the physical DPI scaled appropriately for the virtual display
            // This ensures proper scaling when virtual DPI differs from physical DPI
            float xdpi = displayMetrics.xdpi * scaleX * mVirtualDisplayDpi / displayMetrics.densityDpi;
            float ydpi = displayMetrics.ydpi * scaleY * mVirtualDisplayDpi / displayMetrics.densityDpi;

            Renderer.init(surface, RomManager.getLoaderPath(getApplicationContext()), 
                    mVirtualDisplayWidth, mVirtualDisplayHeight, xdpi, ydpi, (int) getBestFps());

            Log.i(TAG, "surfaceCreated with virtual display: " + mVirtualDisplayWidth + "x" + mVirtualDisplayHeight + 
                    " @ " + mVirtualDisplayDpi + " DPI, calculated xdpi=" + xdpi + ", ydpi=" + ydpi);
            FileLogger.boot("renderer_init", "virt=" + mVirtualDisplayWidth + "x" + mVirtualDisplayHeight
                    + " dpi=" + mVirtualDisplayDpi + " xdpi=" + xdpi + " ydpi=" + ydpi);
        }

        @Override
        public void surfaceChanged(@NonNull SurfaceHolder holder, int format, int width, int height) {
            Surface surface = holder.getSurface();
            // Pass both physical surface dimensions and virtual framebuffer dimensions
            Renderer.resetWindow(surface, 0, 0, width, height, mVirtualDisplayWidth, mVirtualDisplayHeight);
            Log.i(TAG, "surfaceChanged: physical=" + width + "x" + height + ", virtual=" + mVirtualDisplayWidth + "x" + mVirtualDisplayHeight);
            FileLogger.boot("surface_changed", "phys=" + width + "x" + height
                    + " virt=" + mVirtualDisplayWidth + "x" + mVirtualDisplayHeight);
        }

        @Override
        public void surfaceDestroyed(@NonNull SurfaceHolder holder) {
            Renderer.removeWindow(holder.getSurface());
            Log.i(TAG, "surfaceDestroyed!");
            FileLogger.boot("surface_destroyed", null);
        }
    };

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState); // MUST be first — Android requires it

        boolean started = TwoyiStatusManager.getInstance().isStarted();
        Log.i(TAG, "onCreate: " + savedInstanceState + " isStarted: " + started);

        if (started) {
            // we have been started, but WTF we are onCreate again? just reboot ourself.
            finish();
            RomManager.reboot(this);
            return;
        }

        // reset state — both the legacy mStarted flag (for switchOs) and
        // the new BootCompletionServer boot latch (ported from
        // cyanmint/Nogitsune's BootStatus.kt). The boot latch was
        // previously inside TwoyiStatusManager; it now lives in
        // BootCompletionServer so the boot-completion state and the
        // BOOT_COMPLETED socket listener are co-located.
        BootCompletionServer.getInstance().reset();
        TwoyiStatusManager.getInstance().reset();

        NavUtils.hideNavigation(getWindow());

        // Load virtual display settings from profile
        mVirtualDisplayWidth = ProfileSettings.getDisplayWidth(this);
        mVirtualDisplayHeight = ProfileSettings.getDisplayHeight(this);
        mVirtualDisplayDpi = ProfileSettings.getDisplayDpi(this);

        setContentView(R.layout.ac_render);
        mRootView = findViewById(R.id.root);

        mSurfaceView = new SurfaceView(this);
        mSurfaceView.getHolder().addCallback(mSurfaceCallback);

        // Size and center the SurfaceView based on virtual display dimensions
        setupSurfaceViewLayout();

        mLoadingLayout = findViewById(R.id.loadingLayout);
        mLoadingView = findViewById(R.id.loading);
        mLoadingText = findViewById(R.id.loadingText);
        mBootLogView = findViewById(R.id.bootlog);

        mLoadingLayout.setVisibility(View.VISIBLE);
        mLoadingView.startAnimation();

        // The upstream Android-12 guide gate is REMOVED (v5.3).
        // UITips.checkForAndroid12(this, this::bootSystem) used to block
        // bootSystem() behind TWO chained modal dialogs ("Attention — You
        // are running on Android 12..." → "Don't show again" → "Make sure
        // you've followed the tutorial..." → "I confirm it"). That gate is
        // a warning for END USERS on real Android 12+ devices (phantom
        // process killing / ptrace restrictions require manual setup). In
        // THIS fork the container runs under our own kr64 ptrace emulator
        // on controlled hosts (CI emulator / redroid with adb root), where
        // the warning is meaningless — and in CI it froze the whole E2E
        // for the full boot wait: run 32952695067 sat with BOTH dialogs on
        // screen for 600s, bootSystem() never ran, and the workflow still
        // passed green. Boot unconditionally.
        bootSystem();

        mSurfaceView.setOnTouchListener(this);

    }

    @Override
    protected void onDestroy() {
        // Remove the SurfaceHolder callback to prevent surfaceCreated/Changed/Destroyed
        // from firing on a destroyed Activity with a stale Surface pointer.
        if (mSurfaceView != null && mSurfaceCallback != null) {
            mSurfaceView.getHolder().removeCallback(mSurfaceCallback);
        }
        super.onDestroy();
    }

    @Override
    protected void onRestoreInstanceState(@NonNull Bundle savedInstanceState) {
        super.onRestoreInstanceState(savedInstanceState);
        Log.i(TAG, "onRestoreInstanceState: " + savedInstanceState);

        // we don't support state restore, just reboot.
        finish();
        RomManager.reboot(this);
    }

    /**
     * Setup the SurfaceView layout to fit and center the virtual display
     */
    private void setupSurfaceViewLayout() {
        WindowManager windowManager = getWindowManager();
        Display defaultDisplay = windowManager.getDefaultDisplay();
        DisplayMetrics displayMetrics = new DisplayMetrics();
        defaultDisplay.getRealMetrics(displayMetrics);

        int screenWidth = displayMetrics.widthPixels;
        int screenHeight = displayMetrics.heightPixels;

        // Calculate aspect ratios
        float virtualAspect = (float) mVirtualDisplayWidth / (float) mVirtualDisplayHeight;
        float screenAspect = (float) screenWidth / (float) screenHeight;

        // Fit the virtual display within the screen while maintaining aspect ratio
        if (virtualAspect > screenAspect) {
            // Virtual display is wider - fit to width
            mSurfaceWidth = screenWidth;
            mSurfaceHeight = (int) (screenWidth / virtualAspect);
            mSurfaceOffsetX = 0;
            mSurfaceOffsetY = (screenHeight - mSurfaceHeight) / 2;
        } else {
            // Virtual display is taller - fit to height
            mSurfaceHeight = screenHeight;
            mSurfaceWidth = (int) (screenHeight * virtualAspect);
            mSurfaceOffsetX = (screenWidth - mSurfaceWidth) / 2;
            mSurfaceOffsetY = 0;
        }

        // Center the surface view with black letterboxing/pillarboxing
        FrameLayout.LayoutParams params = new FrameLayout.LayoutParams(mSurfaceWidth, mSurfaceHeight);
        params.gravity = android.view.Gravity.CENTER;
        mSurfaceView.setLayoutParams(params);
        
        // Set black background on root to provide letterboxing/pillarboxing
        mRootView.setBackgroundColor(0xFF000000);

        // Pre-compute the touch coordinate transform matrix (SurfaceView-local
        // → virtual display). The matrix never changes between surface layout
        // changes, so caching it avoids allocating a Matrix on every touch event.
        // (mSurfaceWidth/Height are guaranteed > 0 here because setupSurfaceViewLayout
        // always assigns positive values from the screen metrics.)
        if (mSurfaceWidth > 0 && mSurfaceHeight > 0) {
            float scaleX = (float) mVirtualDisplayWidth / (float) mSurfaceWidth;
            float scaleY = (float) mVirtualDisplayHeight / (float) mSurfaceHeight;
            mTouchMatrix.reset();
            mTouchMatrix.postScale(scaleX, scaleY);
        }

        Log.i(TAG, "Virtual display: " + mVirtualDisplayWidth + "x" + mVirtualDisplayHeight +
                ", Screen: " + screenWidth + "x" + screenHeight +
                ", Surface: " + mSurfaceWidth + "x" + mSurfaceHeight +
                ", Offset: " + mSurfaceOffsetX + "," + mSurfaceOffsetY);
    }

    private void bootSystem() {
        FileLogger.boot("boot_system_start", null);
        boolean romExist = RomManager.romExist(this);
        FileLogger.i(TAG, "bootSystem: romExist=" + romExist);

        if (!romExist) {
            // ROM doesn't exist - show message to user and prompt to select ROM
            runOnUiThread(() -> {
                mLoadingView.stopAnimation();
                mLoadingLayout.setVisibility(View.VISIBLE);
                mLoadingText.setText(R.string.no_rootfs_message);
                
                // Show dialog to let user choose ROM file
                UIHelper.getDialogBuilder(this)
                    .setTitle(R.string.no_rootfs_title)
                    .setMessage(R.string.no_rootfs_select_rom)
                    .setPositiveButton(R.string.select_rom_file, (dialog, which) -> {
                        // Prompt user to select ROM file
                        Intent intent = new Intent(Intent.ACTION_GET_CONTENT);
                        intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, false);
                        intent.setType("*/*");
                        intent.addCategory(Intent.CATEGORY_OPENABLE);
                        try {
                            startActivityForResult(intent, REQUEST_SELECT_ROM);
                        } catch (Throwable ignored) {
                            Toast.makeText(this, getString(R.string.error_selecting_file), Toast.LENGTH_SHORT).show();
                            finish();
                        }
                    })
                    .setNegativeButton(android.R.string.cancel, (dialog, which) -> {
                        finish();
                    })
                    .setCancelable(false)
                    .show();
            });
            return;
        }

        // ROM exists, boot normally.
        FileLogger.boot("rom_exists", "starting boot procedure");
        // Clear dalvik-cache if the host build fingerprint changed (e.g. after a host OTA
        // update), so stale ART OAT entries don't cause "No original dex files found" crashes.
        // Called synchronously here (UI thread, before addView) to guarantee the cache is
        // fully cleared before Renderer.init() starts the container.  The existing code
        // already runs shell commands on the main thread inside attachBaseContext, so the
        // brief blocking (~100 ms worst-case for rm -rf) is acceptable.
        //
        // Delegates to DalvikCacheManager (ported from cyanmint/Nogitsune's
        // BootHelper.kt::clearDalvikCacheIfNeeded). RomManager keeps a thin
        // wrapper for backwards-compat with any external callers.
        RomManager.clearDalvikCacheIfNeeded(this);

        // Start the dedicated boot-completion socket server BEFORE the
        // container launches, so it's ready to accept the guest's
        // BOOT_COMPLETED signal the moment the guest sends it. The
        // legacy TwoyiSocketServer on TWOYI_SOCK is also still running
        // and will delegate BOOT_COMPLETED to BootCompletionServer —
        // both paths funnel through BootCompletionServer.markCompleted()
        // (idempotent via AtomicBoolean compare-and-set).
        BootCompletionServer.getInstance().start();
        FileLogger.boot("boot_completion_server_started", null);

        mRootView.addView(mSurfaceView, 0);
        showBootingProcedure();
    }

    private void showBootingProcedure() {
        FileLogger.boot("show_booting_procedure", "waiting for BOOT_COMPLETED (60s timeout)");
        // mLoadingText.setText(R.string.booting_tips);
        mLoadingText.setVisibility(View.GONE);
        mBootLogView.setVisibility(View.VISIBLE);
        new Thread(() -> {

            boolean success = false;
            try {
                // Task 6-Z62: kr64's synthesized BOOT_COMPLETED fires when
                // the recovery child's execve completes — observed at
                // T+118 s after Activity start in the b889666 E2E
                // (Render2Activity 21:30:32 → kr64 daemon 21:32:30), i.e.
                // AFTER the old single 60 s waitBoot deadline. A
                // CyclicBarrier await that times out BREAKS the barrier,
                // so a late markCompleted() could never wake the UI.
                // Poll in 5 s slices instead: waitBoot() now re-arms the
                // latch after each timed-out slice (BootCompletionServer,
                // Task 6-Z62), and the isCompleted() fallback catches the
                // broken-barrier race (BOOT_COMPLETED arrived while we
                // were between slices). Total window 300 s to match the
                // E2E boot_wait.
                long bootDeadlineMs = SystemClock.elapsedRealtime() + 300_000L;
                while (!success && SystemClock.elapsedRealtime() < bootDeadlineMs) {
                    success = BootCompletionServer.getInstance().waitBoot(5, TimeUnit.SECONDS);
                    if (!success && BootCompletionServer.getInstance().isCompleted()) {
                        // BOOT_COMPLETED arrived during a broken-barrier
                        // window — boot DID complete; treat as success.
                        success = true;
                    }
                }
            } catch (Throwable ignored) {
                // BootCompletionServer.waitBoot() catches InterruptedException /
                // BrokenBarrierException internally and returns false, so this
                // catch is a defensive guard against any other unexpected
                // Throwable (e.g. IllegalMonitorStateException from a future
                // refactor). Swallowing these silently made the boot-failure
                // path fire with no diagnostic in logcat — track the exception
                // so crash reporters and developers have a clue when
                // `success == false` leads to trackBootFailure().
                Log.e(TAG, "waitBoot threw — treating as boot failure", ignored);
                FileLogger.e(TAG, "waitBoot threw — treating as boot failure", ignored);
            }

            FileLogger.boot("wait_boot_result", "success=" + success);
            if (!success) {
                FileLogger.boot("boot_failed", "trackBootFailure — NOT calling System.exit (Task 6-Z21: the 2-sec relaunch cycle was caused by System.exit(0) here → Android restarts the process → new kr64 → recovery never reaches framebuffer render. Now we keep the process alive so the existing kr64 can continue running.)");
                LogEvents.trackBootFailure(getApplicationContext());

                runOnUiThread(() -> Toast.makeText(getApplicationContext(), R.string.boot_failed, Toast.LENGTH_SHORT).show());

                // Task 6-Z21: do NOT call System.exit(0). The recovery
                // (kr64 ptrace emulation) is slow — the 60s waitBoot
                // timeout fires before BOOT_COMPLETED, triggering this
                // boot-failure path. Previously System.exit(0) killed
                // the whole process → Android restarted it → a NEW
                // kr64 was spawned → the recovery restarted from
                // scratch every ~2 sec, never reaching the framebuffer
                // render. Now we just log the failure + keep the
                // process (and the existing kr64) alive. The recovery
                // can continue running past the 60s timeout + may
                // eventually reach the framebuffer render.
                // (Removed: SystemClock.sleep(3000) + finish() + System.exit(0).)
                return;
            }

            runOnUiThread(() -> {
            mLoadingView.stopAnimation();
            mLoadingLayout.setVisibility(View.GONE);
            });
        }, "waiting-boot").start();
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);

        if (hasFocus) {
            NavUtils.hideNavigation(getWindow());
        }

        // Update global visibility.
        TwoyiStatusManager.getInstance().updateVisibility(hasFocus);
    }

    @Override
    public boolean onTouch(View v, MotionEvent event) {
        // Transform touch coordinates from surface space to virtual display space.
        // Note: event coordinates are already relative to the SurfaceView (not screen)
        // since the touch listener is attached to mSurfaceView.
        //
        // Performance: MotionEvent.obtain() is a pooled allocation (cheap), but we
        // avoid allocating a new Matrix per event by reusing the pre-computed
        // mTouchMatrix that's refreshed in setupSurfaceViewLayout().
        MotionEvent transformedEvent = MotionEvent.obtain(event);
        transformedEvent.transform(mTouchMatrix);
        Renderer.handleTouch(transformedEvent);
        transformedEvent.recycle();
        return true;
    }

    @Override
    public boolean onKeyDown(int keyCode, KeyEvent event) {
        Log.d(TAG, "onKeyDown: " + keyCode);
        // Volume-key passthrough: the guest owns volume handling.
        return super.onKeyDown(keyCode, event);
    }

    @Override
    public void onBackPressed() {
        // super.onBackPressed();
        Renderer.sendKeycode(KeyEvent.KEYCODE_HOME);
    }

    private float getBestFps() {
        // Fixed: the previous implementation iterated over the display's
        // supported Modes but the `fps = refreshRate` assignment was
        // commented out, so the loop was dead code and the method always
        // returned 45. Worse, the Log.w message said "current fps: 45",
        // implying the device's actual refresh rate was 45 Hz, which is
        // misleading on 60/90/120 Hz devices.
        //
        // The intent (per the original author) appears to have been to pick
        // the highest supported refresh rate, but that path was disabled —
        // probably because running the guest renderer above 45 fps caused
        // dropped frames or excessive battery drain on the test device.
        // Keeping the cap at 45 but documenting it, and dropping the dead
        // loop, so future readers don't waste time tracing it.
        final float fpsCap = 45f;
        Log.i(TAG, "renderer fps cap: " + fpsCap
                + " (display refresh-rate selection intentionally disabled)");
        return fpsCap;
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        
        if (requestCode == REQUEST_SELECT_ROM && resultCode == RESULT_OK) {
            if (data != null && data.getData() != null) {
                importRomAndStart(data.getData());
            }
        }
    }

    private void importRomAndStart(Uri uri) {
        mLoadingLayout.setVisibility(View.VISIBLE);
        mLoadingView.startAnimation();
        mLoadingText.setText(R.string.extracting_tips);

        ProgressDialog dialog = UIHelper.getProgressDialog(this);
        dialog.setCancelable(false);
        dialog.show();

        UIHelper.defer().when(() -> {
            String activeProfile = ProfileManager.getActiveProfile(this);
            File profileRootfsDir = ProfileManager.getProfileRootfsDir(this, activeProfile);

            // 6-Z184 NON-DESTRUCTIVE IMPORT: extract into a STAGING sibling
            // first; only after a successful extraction do we swap it into
            // place. The previous flow deleted the working rootfs BEFORE
            // extracting — a failed copy, revoked Uri grant, corrupt tar or
            // timeout left the app with NO ROM at all ("No ROM Installed",
            // unrecoverable without a re-import).
            File stagingDir = new File(profileRootfsDir.getParentFile(),
                    profileRootfsDir.getName() + ".importing");
            if (stagingDir.exists()) {
                io.twoyi.utils.IOUtils.deleteDirectory(stagingDir);
            }
            stagingDir.mkdirs();

            File tempFile = new File(getCacheDir(), "rootfs_import.tar");
            try {
                ContentResolver contentResolver = getContentResolver();
                try (InputStream inputStream = contentResolver.openInputStream(uri);
                     OutputStream os = new FileOutputStream(tempFile)) {
                    // Fixed: openInputStream can return null if the provider
                    // revokes the grant between picker return and our read.
                    if (inputStream == null) {
                        throw new IOException("ContentResolver returned null stream for " + uri);
                    }
                    byte[] buffer = new byte[8192];
                    int count;
                    while ((count = inputStream.read(buffer)) > 0) {
                        os.write(buffer, 0, count);
                    }
                }

                String tempFilePath = tempFile.getAbsolutePath();
                String stagingPath = stagingDir.getAbsolutePath();

                if (tempFilePath.contains(";") || tempFilePath.contains("&") ||
                    stagingPath.contains(";") || stagingPath.contains("&")) {
                    throw new SecurityException("Invalid path detected");
                }

                // Extract tar to the STAGING directory
                ProcessBuilder pb = new ProcessBuilder(
                    "tar", "-xf", tempFilePath,
                    "-C", stagingPath
                );
                Process process = pb.start();
                // Fixed: waitFor() without a timeout can hang forever on a
                // corrupt archive. Cap at 120 s; 500 MB rootfs tar extracts in
                // ~30 s on most devices.
                int exitCode;
                if (!process.waitFor(120, TimeUnit.SECONDS)) {
                    process.destroyForcibly();
                    throw new IOException("tar extraction timed out after 120 s");
                } else {
                    exitCode = process.exitValue();
                }

                if (exitCode == 0) {
                    // Swap: remove the old rootfs and move staging into place
                    // (rename within the same parent directory = same
                    // filesystem = atomic).
                    if (profileRootfsDir.exists()) {
                        io.twoyi.utils.IOUtils.deleteDirectory(profileRootfsDir);
                    }
                    if (!stagingDir.renameTo(profileRootfsDir)) {
                        throw new IOException("staging rename failed: "
                                + stagingPath + " -> " + profileRootfsDir.getAbsolutePath());
                    }
                    RomManager.initRootfs(this);
                    return true;
                }
                return false;
            } finally {
                // Whatever happened, drop the staged tar and any leftover
                // staging dir (on success it was renamed away; this is a no-op).
                tempFile.delete();
                if (stagingDir.exists()) {
                    io.twoyi.utils.IOUtils.deleteDirectory(stagingDir);
                }
            }
        }).done(result -> {
            UIHelper.dismiss(dialog);
            if (result) {
                // ROM imported successfully, restart to boot
                RomManager.reboot(this);
            } else {
                Toast.makeText(this, getString(R.string.rom_import_failed), Toast.LENGTH_SHORT).show();
                finish();
            }
        }).fail(result -> runOnUiThread(() -> {
            Toast.makeText(this, getString(R.string.rom_import_error, result.getMessage()), Toast.LENGTH_SHORT).show();
            dialog.dismiss();
            finish();
        }));
    }
}
