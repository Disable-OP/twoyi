/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

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
import java.io.InputStream;
import java.io.OutputStream;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

import io.twoyi.utils.AppKV;
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

    private final AtomicBoolean mIsExtracting = new AtomicBoolean(false);

    private final SurfaceHolder.Callback mSurfaceCallback = new SurfaceHolder.Callback() {
        @Override
        public void surfaceCreated(@NonNull SurfaceHolder holder) {
            Surface surface = holder.getSurface();
            
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
        }

        @Override
        public void surfaceChanged(@NonNull SurfaceHolder holder, int format, int width, int height) {
            Surface surface = holder.getSurface();
            // Pass both physical surface dimensions and virtual framebuffer dimensions
            Renderer.resetWindow(surface, 0, 0, width, height, mVirtualDisplayWidth, mVirtualDisplayHeight);
            Log.i(TAG, "surfaceChanged: physical=" + width + "x" + height + ", virtual=" + mVirtualDisplayWidth + "x" + mVirtualDisplayHeight);
        }

        @Override
        public void surfaceDestroyed(@NonNull SurfaceHolder holder) {
            Renderer.removeWindow(holder.getSurface());
            Log.i(TAG, "surfaceDestroyed!");
        }
    };

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        boolean started = TwoyiStatusManager.getInstance().isStarted();
        Log.i(TAG, "onCreate: " + savedInstanceState + " isStarted: " + started);

        if (started) {
            // we have been started, but WTF we are onCreate again? just reboot ourself.
            finish();
            RomManager.reboot(this);
            return;
        }

        // reset state
        TwoyiStatusManager.getInstance().reset();

        NavUtils.hideNavigation(getWindow());

        super.onCreate(savedInstanceState);

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

        UITips.checkForAndroid12(this, this::bootSystem);

        mSurfaceView.setOnTouchListener(this);

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

        Log.i(TAG, "Virtual display: " + mVirtualDisplayWidth + "x" + mVirtualDisplayHeight +
                ", Screen: " + screenWidth + "x" + screenHeight +
                ", Surface: " + mSurfaceWidth + "x" + mSurfaceHeight +
                ", Offset: " + mSurfaceOffsetX + "," + mSurfaceOffsetY);
    }

    private void bootSystem() {
        boolean romExist = RomManager.romExist(this);

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
                            Toast.makeText(this, "Error selecting file", Toast.LENGTH_SHORT).show();
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

        // ROM exists, boot normally
        mRootView.addView(mSurfaceView, 0);
        showBootingProcedure();
    }

    private void showTipsForFirstBoot() {
        mLoadingText.setText(R.string.extracting_tips);
        mRootView.postDelayed(() -> {
            if (mIsExtracting.get()) {
                mLoadingText.setText(R.string.first_boot_tips);
            }
        }, 5000);

        mRootView.postDelayed(() -> {
            if (mIsExtracting.get()) {
                mLoadingText.setText(R.string.first_boot_tips2);
            }
        }, 10 * 1000);

        mRootView.postDelayed(() -> {
            if (mIsExtracting.get()) {
                mLoadingText.setText(R.string.first_boot_tips3);
            }
        }, 15 * 1000);
    }

    private void showBootingProcedure() {
        // mLoadingText.setText(R.string.booting_tips);
        mLoadingText.setVisibility(View.GONE);
        mBootLogView.setVisibility(View.VISIBLE);
        new Thread(() -> {

            if (true) {
                boolean success = false;
                try {
                    success = TwoyiStatusManager.getInstance().waitBoot(15, TimeUnit.SECONDS);
                } catch (Throwable ignored) {
                }

                if (!success) {
                    LogEvents.trackBootFailure(getApplicationContext());

                    runOnUiThread(() -> Toast.makeText(getApplicationContext(), R.string.boot_failed, Toast.LENGTH_SHORT).show());

                    // waiting for track
                    SystemClock.sleep(3000);

                    finish();
                    System.exit(0);
                    return;
                }
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
        // Transform touch coordinates from surface space to virtual display space
        // Note: event coordinates are already relative to the SurfaceView (not screen)
        // since the touch listener is attached to mSurfaceView
        MotionEvent transformedEvent = MotionEvent.obtain(event);
        
        // Calculate the transformation matrix
        android.graphics.Matrix matrix = new android.graphics.Matrix();
        
        // Scale from surface dimensions to virtual display dimensions
        // No translation needed since coordinates are already relative to SurfaceView
        float scaleX = (float) mVirtualDisplayWidth / mSurfaceWidth;
        float scaleY = (float) mVirtualDisplayHeight / mSurfaceHeight;
        matrix.postScale(scaleX, scaleY);
        
        // Transform the event
        transformedEvent.transform(matrix);
        
        Renderer.handleTouch(transformedEvent);
        transformedEvent.recycle();
        return true;
    }

    @Override
    public boolean onKeyDown(int keyCode, KeyEvent event) {
        Log.d(TAG, "onKeyDown: " + keyCode);
        if (keyCode == KeyEvent.KEYCODE_VOLUME_UP || keyCode == KeyEvent.KEYCODE_VOLUME_DOWN) {
            // TODO: 2021/10/26 Add Volume control
        }
        return super.onKeyDown(keyCode, event);
    }

    @Override
    public void onBackPressed() {
        // super.onBackPressed();
        Renderer.sendKeycode(KeyEvent.KEYCODE_HOME);
    }

    private float getBestFps() {
        WindowManager windowManager = getWindowManager();
        Display defaultDisplay = windowManager.getDefaultDisplay();
        Display.Mode[] supportedModes = defaultDisplay.getSupportedModes();
        float fps = 45;
        for (Display.Mode supportedMode : supportedModes) {
            float refreshRate = supportedMode.getRefreshRate();
            if (refreshRate > fps) {
                // fps = refreshRate;
            }
        }

        Log.w(TAG, "current fps: " + fps);
        return fps;
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
            
            // Clear existing rootfs
            if (profileRootfsDir.exists()) {
                io.twoyi.utils.IOUtils.deleteDirectory(profileRootfsDir);
            }
            profileRootfsDir.mkdirs();
            
            File tempFile = new File(getCacheDir(), "rootfs_import.tar");

            ContentResolver contentResolver = getContentResolver();
            try (InputStream inputStream = contentResolver.openInputStream(uri);
                 OutputStream os = new FileOutputStream(tempFile)) {
                byte[] buffer = new byte[8192];
                int count;
                while ((count = inputStream.read(buffer)) > 0) {
                    os.write(buffer, 0, count);
                }
            }

            String tempFilePath = tempFile.getAbsolutePath();
            String rootfsPath = profileRootfsDir.getAbsolutePath();
            
            if (tempFilePath.contains(";") || tempFilePath.contains("&") ||
                rootfsPath.contains(";") || rootfsPath.contains("&")) {
                throw new SecurityException("Invalid path detected");
            }
            
            // Extract tar to rootfs directory
            ProcessBuilder pb = new ProcessBuilder(
                "tar", "-xf", tempFilePath,
                "-C", rootfsPath
            );
            Process process = pb.start();
            int exitCode = process.waitFor();
            
            tempFile.delete();
            
            if (exitCode == 0) {
                RomManager.initRootfs(this);
            }
            
            return exitCode == 0;
        }).done(result -> {
            UIHelper.dismiss(dialog);
            if (result) {
                // ROM imported successfully, restart to boot
                RomManager.reboot(this);
            } else {
                Toast.makeText(this, "Failed to import ROM", Toast.LENGTH_SHORT).show();
                finish();
            }
        }).fail(result -> runOnUiThread(() -> {
            Toast.makeText(this, "Error importing ROM: " + result.getMessage(), Toast.LENGTH_SHORT).show();
            dialog.dismiss();
            finish();
        }));
    }
}
