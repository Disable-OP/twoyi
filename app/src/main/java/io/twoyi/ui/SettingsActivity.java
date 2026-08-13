/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.ui;

import android.app.Activity;
import android.app.ProgressDialog;
import android.content.ContentResolver;
import android.content.Context;
import android.content.Intent;
import android.graphics.drawable.ColorDrawable;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.preference.CheckBoxPreference;
import android.preference.Preference;
import android.preference.PreferenceFragment;
import android.provider.DocumentsContract;
import android.view.MenuItem;
import android.view.View;
import android.widget.Toast;

import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import androidx.annotation.StringRes;
import androidx.appcompat.app.ActionBar;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.content.ContextCompat;
import androidx.core.content.FileProvider;

import com.microsoft.appcenter.crashes.Crashes;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.file.Files;

import io.twoyi.R;
import android.util.Log;
import io.twoyi.utils.LogEvents;
import io.twoyi.utils.ProfileManager;
import io.twoyi.utils.ProfileSettings;
import io.twoyi.utils.RomManager;
import io.twoyi.utils.UIHelper;

/**
 * @author weishu
 * @date 2022/1/2.
 */

public class SettingsActivity extends AppCompatActivity {

    private static final int REQUEST_SELECT_ROM = 1001;
    private static final String TAG = "SettingsActivity";

    /** Reference to the hosted SettingsFragment so the activity can forward
     *  ACTION_VIEW intents (file manager / `am start`) to the same import
     *  path the SAF picker uses via onActivityResult. */
    private SettingsFragment mSettingsFragment;

    /** Guards against re-processing the same ACTION_VIEW intent twice
     *  (e.g. on configuration change, which restarts the activity with the
     *  same launching Intent). */
    private boolean mHandledViewIntent = false;

    @Override
    protected void onCreate(@Nullable Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        setContentView(R.layout.activity_settings);
        mSettingsFragment = new SettingsFragment();
        getFragmentManager().beginTransaction()
                .replace(R.id.settingsFrameLayout, mSettingsFragment)
                .commit();

        ActionBar actionBar = getSupportActionBar();

        if (actionBar != null) {
            actionBar.setDisplayHomeAsUpEnabled(true);
            // Color resource → ColorDrawable (avoids deprecated getResources().getDrawable
            // which would throw on a color resource on API 22+).
            actionBar.setBackgroundDrawable(new ColorDrawable(ContextCompat.getColor(this, R.color.colorPrimary)));
            actionBar.setTitle(R.string.title_settings);
        }

        // Cold-start path: a file manager (or `am start`) launched us with
        // ACTION_VIEW + a file/content URI. Defer handling until the
        // SettingsFragment is attached (the fragment transaction is async).
        handleViewIntent(getIntent());
    }

    /**
     * Warm-start path: SettingsActivity has launchMode="singleTask", so a
     * new ACTION_VIEW intent (e.g. user taps another ROM file while twoyi
     * is in the foreground) is delivered here without recreating the
     * activity. Forward it to the same handler.
     */
    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        // Allow a fresh ACTION_VIEW to be processed even if a previous one
        // was already handled (the user picked a different file).
        mHandledViewIntent = false;
        handleViewIntent(intent);
    }

    /**
     * If the launching (or new) intent is ACTION_VIEW with a file/content
     * URI, treat it as a ROM selection request — identical to what the SAF
     * picker does via onActivityResult. The URI is forwarded to the hosted
     * SettingsFragment.importRomForActiveProfile(Uri).
     *
     * This is what makes `am start -a android.intent.action.VIEW -d
     * "file:///sdcard/Download/recovery.img" -t "*/*"` work for the E2E
     * UI test, and what lets a user tap a .img file in their file manager
     * to import it directly into twoyi (no picker UI required).
     */
    private void handleViewIntent(Intent intent) {
        if (intent == null) return;
        if (!Intent.ACTION_VIEW.equals(intent.getAction())) return;
        if (mHandledViewIntent) return;
        Uri uri = intent.getData();
        if (uri == null) return;
        mHandledViewIntent = true;
        Log.i(TAG, "Received ACTION_VIEW for ROM file: " + uri);

        // The fragment transaction in onCreate is committed but not yet
        // attached when onCreate is still on the call stack. Post to the
        // main looper so the import runs AFTER the fragment is attached
        // (otherwise getActivity()/findPreference() inside the fragment
        // would NPE).
        final Uri finalUri = uri;
        new Handler(Looper.getMainLooper()).post(() -> {
            if (mSettingsFragment != null && mSettingsFragment.isAdded()) {
                mSettingsFragment.importRomForActiveProfile(finalUri);
            } else {
                // Retry once after a short delay in case the fragment still
                // isn't ready (rare race during cold start).
                new Handler(Looper.getMainLooper()).postDelayed(() -> {
                    if (mSettingsFragment != null && mSettingsFragment.isAdded()) {
                        mSettingsFragment.importRomForActiveProfile(finalUri);
                    } else {
                        Log.w(TAG, "SettingsFragment not attached — cannot import ROM from " + finalUri);
                    }
                }, 500);
            }
        });
    }

    @Override
    public boolean onOptionsItemSelected(@NonNull MenuItem item) {
        if (item.getItemId() == android.R.id.home) {
            onBackPressed();
            return true;
        }

        return super.onOptionsItemSelected(item);
    }

    public static class SettingsFragment extends PreferenceFragment {
        
        // Validation constants
        private static final int MAX_DISPLAY_DIMENSION = 4096;
        private static final int MAX_DPI = 640;
        private static final int MIN_VALUE = 1;
        
        @Override
        public void onCreate(@Nullable Bundle savedInstanceState) {
            super.onCreate(savedInstanceState);
            addPreferencesFromResource(R.xml.pref_settings);
        }

        private Preference findPreference(@StringRes int id) {
            String key = getString(id);
            return findPreference(key);
        }

        @Override
        public void onViewCreated(View view, @Nullable Bundle savedInstanceState) {
            super.onViewCreated(view, savedInstanceState);

            Preference launchContainer = findPreference(R.string.settings_key_launch_container);
            Preference importApp = findPreference(R.string.settings_key_import_app);
            Preference export = findPreference(R.string.settings_key_manage_files);

            Preference shutdown = findPreference(R.string.settings_key_shutdown);
            Preference reboot = findPreference(R.string.settings_key_reboot);
            
            Preference profileManager = findPreference(R.string.settings_key_profile_manager);
            CheckBoxPreference verboseLogging = (CheckBoxPreference) findPreference(R.string.settings_key_verbose_logging);
            Preference displayWidth = findPreference(R.string.settings_key_display_width);
            Preference displayHeight = findPreference(R.string.settings_key_display_height);
            Preference displayDpi = findPreference(R.string.settings_key_display_dpi);
            android.preference.ListPreference displayColorDepth =
                    (android.preference.ListPreference) findPreference(R.string.settings_key_display_color_depth);
            CheckBoxPreference debugRenderer = (CheckBoxPreference) findPreference(R.string.settings_key_debug_renderer);
            CheckBoxPreference bootRecovery = (CheckBoxPreference) findPreference(R.string.settings_key_boot_recovery);
            Preference selectRom = findPreference(R.string.settings_key_select_rom);
            Preference factoryReset = findPreference(R.string.settings_key_factory_reset);

            Preference sendLog = findPreference(R.string.settings_key_sendlog);
            Preference about = findPreference(R.string.settings_key_about);

            // Initialize verbose logging checkbox with profile-specific value
            verboseLogging.setChecked(ProfileSettings.isVerboseLoggingEnabled(getActivity()));
            verboseLogging.setOnPreferenceChangeListener((preference, newValue) -> {
                ProfileSettings.setVerboseLogging(getActivity(), (Boolean) newValue);
                return true;
            });

            // Initialize display configuration preferences
            // Default values (0) mean "auto-detect from physical screen".
            android.preference.EditTextPreference displayWidthPref = (android.preference.EditTextPreference) displayWidth;
            android.preference.EditTextPreference displayHeightPref = (android.preference.EditTextPreference) displayHeight;
            android.preference.EditTextPreference displayDpiPref = (android.preference.EditTextPreference) displayDpi;

            // Show the auto-detected screen values in the summary even when
            // the stored value is 0 (auto-detect).
            int actualWidth = ProfileSettings.getDisplayWidth(getActivity());
            displayWidthPref.setText(String.valueOf(actualWidth));
            displayWidthPref.setSummary("Virtual display width (current: " + actualWidth + ", 0 = auto-detect screen)");
            displayWidthPref.setOnPreferenceChangeListener((preference, newValue) -> {
                try {
                    int width = Integer.parseInt(newValue.toString());
                    if (width == 0 || (width >= MIN_VALUE && width <= MAX_DISPLAY_DIMENSION)) {
                        ProfileSettings.setDisplayWidth(getActivity(), width);
                        int display = width > 0 ? width : ProfileSettings.getDisplayWidth(getActivity());
                        displayWidthPref.setSummary("Virtual display width (current: " + display + ", 0 = auto-detect screen)");
                        Toast.makeText(getActivity(), R.string.settings_display_change_reboot, Toast.LENGTH_SHORT).show();
                        return true;
                    } else {
                        Toast.makeText(getActivity(), getString(R.string.settings_width_range_error, MIN_VALUE, MAX_DISPLAY_DIMENSION), Toast.LENGTH_SHORT).show();
                        return false;
                    }
                } catch (NumberFormatException e) {
                    Toast.makeText(getActivity(), getString(R.string.settings_invalid_number), Toast.LENGTH_SHORT).show();
                    return false;
                }
            });

            int actualHeight = ProfileSettings.getDisplayHeight(getActivity());
            displayHeightPref.setText(String.valueOf(actualHeight));
            displayHeightPref.setSummary("Virtual display height (current: " + actualHeight + ", 0 = auto-detect screen)");
            displayHeightPref.setOnPreferenceChangeListener((preference, newValue) -> {
                try {
                    int height = Integer.parseInt(newValue.toString());
                    if (height == 0 || (height >= MIN_VALUE && height <= MAX_DISPLAY_DIMENSION)) {
                        ProfileSettings.setDisplayHeight(getActivity(), height);
                        int display = height > 0 ? height : ProfileSettings.getDisplayHeight(getActivity());
                        displayHeightPref.setSummary("Virtual display height (current: " + display + ", 0 = auto-detect screen)");
                        Toast.makeText(getActivity(), R.string.settings_display_change_reboot, Toast.LENGTH_SHORT).show();
                        return true;
                    } else {
                        Toast.makeText(getActivity(), getString(R.string.settings_height_range_error, MIN_VALUE, MAX_DISPLAY_DIMENSION), Toast.LENGTH_SHORT).show();
                        return false;
                    }
                } catch (NumberFormatException e) {
                    Toast.makeText(getActivity(), getString(R.string.settings_invalid_number), Toast.LENGTH_SHORT).show();
                    return false;
                }
            });

            int actualDpi = ProfileSettings.getDisplayDpi(getActivity());
            displayDpiPref.setText(String.valueOf(actualDpi));
            displayDpiPref.setSummary("Virtual display DPI (current: " + actualDpi + ", 0 = auto-detect screen)");
            displayDpiPref.setOnPreferenceChangeListener((preference, newValue) -> {
                try {
                    int dpi = Integer.parseInt(newValue.toString());
                    if (dpi == 0 || (dpi >= MIN_VALUE && dpi <= MAX_DPI)) {
                        ProfileSettings.setDisplayDpi(getActivity(), dpi);
                        int display = dpi > 0 ? dpi : ProfileSettings.getDisplayDpi(getActivity());
                        displayDpiPref.setSummary("Virtual display DPI (current: " + display + ", 0 = auto-detect screen)");
                        Toast.makeText(getActivity(), R.string.settings_display_change_reboot, Toast.LENGTH_SHORT).show();
                        return true;
                    } else {
                        Toast.makeText(getActivity(), getString(R.string.settings_dpi_range_error, MIN_VALUE, MAX_DPI), Toast.LENGTH_SHORT).show();
                        return false;
                    }
                } catch (NumberFormatException e) {
                    Toast.makeText(getActivity(), getString(R.string.settings_invalid_number), Toast.LENGTH_SHORT).show();
                    return false;
                }
            });

            // Initialize display color depth with profile-specific value
            if (displayColorDepth != null) {
                int currentDepth = ProfileSettings.getDisplayColorDepth(getActivity());
                displayColorDepth.setValue(String.valueOf(currentDepth));
                displayColorDepth.setSummary("Color depth: " + currentDepth + " bpp");
                displayColorDepth.setOnPreferenceChangeListener((preference, newValue) -> {
                    try {
                        int depth = Integer.parseInt(newValue.toString());
                        ProfileSettings.setDisplayColorDepth(getActivity(), depth);
                        displayColorDepth.setSummary("Color depth: " + depth + " bpp");
                        Toast.makeText(getActivity(), R.string.settings_display_change_reboot, Toast.LENGTH_SHORT).show();
                        return true;
                    } catch (NumberFormatException e) {
                        Toast.makeText(getActivity(), R.string.settings_invalid_number, Toast.LENGTH_SHORT).show();
                        return false;
                    }
                });
            }

            // Initialize debug renderer checkbox with profile-specific value
            debugRenderer.setChecked(ProfileSettings.isDebugRendererEnabled(getActivity()));
            debugRenderer.setOnPreferenceChangeListener((preference, newValue) -> {
                ProfileSettings.setDebugRenderer(getActivity(), (Boolean) newValue);
                Toast.makeText(getActivity(), R.string.settings_display_change_reboot, Toast.LENGTH_SHORT).show();
                return true;
            });

            // Initialize Boot Recovery (TWRP) checkbox with profile-specific value.
            // When checked, the next container launch will pass --boot-recovery to
            // kr64, booting a TWRP recovery image instead of full Android. The
            // user is responsible for installing a TWRP rootfs into the profile's
            // rootfs directory before enabling this (e.g. via
            // scripts/extract-twrp-ramdisk.py).
            bootRecovery.setChecked(ProfileSettings.isBootRecoveryEnabled(getActivity()));
            bootRecovery.setOnPreferenceChangeListener((preference, newValue) -> {
                ProfileSettings.setBootRecovery(getActivity(), (Boolean) newValue);
                Toast.makeText(getActivity(), R.string.settings_display_change_reboot, Toast.LENGTH_SHORT).show();
                return true;
            });

            launchContainer.setOnPreferenceClickListener(preference -> {
                Intent intent = new Intent(getContext(), io.twoyi.Render2Activity.class);
                startActivity(intent);
                return true;
            });

            importApp.setOnPreferenceClickListener(preference -> {
                UIHelper.startActivity(getContext(), SelectAppActivity.class);
                return true;
            });

            export.setOnPreferenceClickListener(preference -> {
                Intent intent = new Intent(Intent.ACTION_VIEW);
                intent.setType(DocumentsContract.Document.MIME_TYPE_DIR);
                startActivity(intent);
                return true;
            });

            shutdown.setOnPreferenceClickListener(preference -> {
                Activity activity = getActivity();
                activity.finishAffinity();
                RomManager.shutdown(activity);
                return true;
            });

            reboot.setOnPreferenceClickListener(preference -> {
                Activity activity = getActivity();
                activity.finishAndRemoveTask();
                RomManager.reboot(activity);
                return true;
            });

            profileManager.setOnPreferenceClickListener(preference -> {
                UIHelper.startActivity(getContext(), ProfileManagerActivity.class);
                return true;
            });

            selectRom.setOnPreferenceClickListener(preference -> {
                Intent intent = new Intent(Intent.ACTION_GET_CONTENT);
                intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, false);
                intent.setType("*/*");
                intent.addCategory(Intent.CATEGORY_OPENABLE);
                try {
                    startActivityForResult(intent, REQUEST_SELECT_ROM);
                } catch (Throwable ignored) {
                    Toast.makeText(getContext(), getString(R.string.error_generic), Toast.LENGTH_SHORT).show();
                }
                return true;
            });

            factoryReset.setOnPreferenceClickListener(preference -> {
                UIHelper.getDialogBuilder(getActivity())
                        .setTitle(android.R.string.dialog_alert_title)
                        .setMessage(R.string.factory_reset_confirm_message)
                        .setPositiveButton(R.string.i_confirm_it, (dialog, which) -> {
                            // Clear the active profile's rootfs completely so next boot will prompt for ROM
                            Activity activity = getActivity();
                            if (activity != null) {
                                String activeProfile = ProfileManager.getActiveProfile(activity);
                                File profileRootfsDir = ProfileManager.getProfileRootfsDir(activity, activeProfile);
                                io.twoyi.utils.IOUtils.deleteDirectory(profileRootfsDir);
                            }
                            dialog.dismiss();

                            RomManager.reboot(getActivity());
                        })
                        .setNegativeButton(android.R.string.cancel, (dialog, which) -> dialog.dismiss())
                        .show();
                return true;
            });

            sendLog.setOnPreferenceClickListener(preference -> {
                final Context context = getActivity();
                if (context == null) {
                    return true;
                }
                // Pack all logs from /sdcard/Android/data/io.twoyi/files/log/
                // into a .zip file, then share via ACTION_SEND.
                final ProgressDialog progressDialog = UIHelper.getProgressDialog(context);
                progressDialog.setMessage(getString(R.string.settings_key_sendlog));
                progressDialog.setCancelable(false);
                progressDialog.show();

                UIHelper.GLOBAL_EXECUTOR.execute(() -> {
                    // Get the external log directory (same as FileLogger uses)
                    File logDir = context.getExternalFilesDir("log");
                    if (logDir == null || !logDir.exists()) {
                        new Handler(Looper.getMainLooper()).post(() -> {
                            UIHelper.dismiss(progressDialog);
                            Toast.makeText(context, "No logs found", Toast.LENGTH_SHORT).show();
                        });
                        return;
                    }

                    // Create a zip file in the cache dir containing all
                    // external log files + internal kr64-app-stderr.log.
                    final File zipFile = new File(context.getCacheDir(), "twoyi-logs.zip");
                    try (java.util.zip.ZipOutputStream zos = new java.util.zip.ZipOutputStream(
                            new java.io.FileOutputStream(zipFile))) {
                        // Add all files from the external log directory
                        File[] logFiles = logDir.listFiles();
                        if (logFiles != null) {
                            for (File logFile : logFiles) {
                                if (logFile.isFile()) {
                                    zos.putNextEntry(new java.util.zip.ZipEntry(logFile.getName()));
                                    java.nio.file.Files.copy(logFile.toPath(), zos);
                                    zos.closeEntry();
                                }
                            }
                        }
                        // Also add the internal kr64-app-stderr.log
                        File kr64Log = new File(context.getDataDir(), "kr64-app-stderr.log");
                        if (kr64Log.exists() && kr64Log.length() > 0) {
                            zos.putNextEntry(new java.util.zip.ZipEntry("kr64-app-stderr.log"));
                            java.nio.file.Files.copy(kr64Log.toPath(), zos);
                            zos.closeEntry();
                        }
                        // Also add log.txt (fallback linker path)
                        File logTxt = new File(context.getDataDir(), "log.txt");
                        if (logTxt.exists() && logTxt.length() > 0) {
                            zos.putNextEntry(new java.util.zip.ZipEntry("log.txt"));
                            java.nio.file.Files.copy(logTxt.toPath(), zos);
                            zos.closeEntry();
                        }
                    } catch (IOException e) {
                        Crashes.trackError(e);
                        new Handler(Looper.getMainLooper()).post(() -> {
                            UIHelper.dismiss(progressDialog);
                            Toast.makeText(context, "Failed to pack logs: " + e.getMessage(), Toast.LENGTH_SHORT).show();
                        });
                        return;
                    }

                    final Uri uri = FileProvider.getUriForFile(context, "io.twoyi.fileprovider", zipFile);

                    final Intent shareIntent = new Intent(Intent.ACTION_SEND);
                    shareIntent.putExtra(Intent.EXTRA_STREAM, uri);
                    shareIntent.setDataAndType(uri, "application/zip");
                    shareIntent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);

                    new Handler(Looper.getMainLooper()).post(() -> {
                        UIHelper.dismiss(progressDialog);
                        try {
                            context.startActivity(Intent.createChooser(shareIntent,
                                    getString(R.string.settings_key_sendlog)));
                        } catch (Throwable ignored) {
                            Toast.makeText(context, getString(R.string.error_sharing_log), Toast.LENGTH_SHORT).show();
                        }
                    });
                });

                return true;
            });

            about.setOnPreferenceClickListener(preference -> {
                UIHelper.startActivity(getContext(), AboutActivity.class);
                return true;
            });
        }

        @Override
        public void onActivityResult(int requestCode, int resultCode, @Nullable Intent data) {
            super.onActivityResult(requestCode, resultCode, data);
            
            if (requestCode == REQUEST_SELECT_ROM && resultCode == Activity.RESULT_OK) {
                if (data != null && data.getData() != null) {
                    importRomForActiveProfile(data.getData());
                }
            }
        }

        public void importRomForActiveProfile(Uri uri) {
            Activity activity = getActivity();
            if (activity == null) return;

            ProgressDialog dialog = UIHelper.getProgressDialog(activity);
            dialog.setCancelable(false);
            dialog.show();

            UIHelper.defer().when(() -> {
                String activeProfile = ProfileManager.getActiveProfile(activity);
                File profileRootfsDir = ProfileManager.getProfileRootfsDir(activity, activeProfile);

                // Clear existing rootfs
                if (profileRootfsDir.exists()) {
                    io.twoyi.utils.IOUtils.deleteDirectory(profileRootfsDir);
                }
                profileRootfsDir.mkdirs();

                // Use RamdiskImporter which supports .tar, .img, .cpio, .zip formats
                boolean success = false;
                String errorMsg = null;
                try {
                    success = io.twoyi.utils.RamdiskImporter.importRamdisk(activity, uri, profileRootfsDir);
                } catch (Exception e) {
                    errorMsg = e.getMessage();
                    Log.e("SettingsActivity", "Import failed", e);
                }

                if (success) {
                    RomManager.initRootfs(activity);
                }

                // Return error message if failed (for display to user)
                if (!success && errorMsg == null) {
                    errorMsg = "Import returned false (unknown reason)";
                }
                return success ? "SUCCESS" : ("FAIL:" + (errorMsg != null ? errorMsg : "unknown"));
            }).done(result -> {
                UIHelper.dismiss(dialog);
                if ("SUCCESS".equals(result)) {
                    Toast.makeText(activity, getString(R.string.rom_imported_successfully), Toast.LENGTH_SHORT).show();
                    // Update the 'Select ROM' preference summary to reflect
                    // the just-imported file. This is purely cosmetic for
                    // end-users (they see the file name in the preference
                    // row), but it is ALSO what the E2E UI test
                    // (scripts/ui-navigate.py::verify_rom_imported) inspects
                    // to confirm a ROM was actually imported — without this,
                    // the summary would still show the default "Import
                    // rootfs (.tar), ..." prompt and the test would abort.
                    Preference selectRomPref = findPreference(R.string.settings_key_select_rom);
                    if (selectRomPref != null && uri != null) {
                        String name = uri.getLastPathSegment();
                        if (name == null || name.isEmpty()) name = uri.toString();
                        selectRomPref.setSummary(name);
                    }
                } else {
                    String msg = result != null && result.startsWith("FAIL:") ? result.substring(5) : "unknown error";
                    Toast.makeText(activity, getString(R.string.rom_import_failed) + ": " + msg, Toast.LENGTH_LONG).show();
                }
            }).fail(result -> activity.runOnUiThread(() -> {
                Toast.makeText(activity, getString(R.string.rom_import_error, result.getMessage()), Toast.LENGTH_SHORT).show();
                dialog.dismiss();
            }));
        }
    }
}
