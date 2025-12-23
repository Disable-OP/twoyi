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
import android.net.Uri;
import android.os.Bundle;
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
import androidx.core.content.FileProvider;

import com.microsoft.appcenter.crashes.Crashes;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.file.Files;
import java.util.List;

import io.twoyi.R;
import io.twoyi.utils.AppKV;
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

    @Override
    protected void onCreate(@Nullable Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        setContentView(R.layout.activity_settings);
        SettingsFragment settingsFragment = new SettingsFragment();
        getFragmentManager().beginTransaction()
                .replace(R.id.settingsFrameLayout, settingsFragment)
                .commit();

        ActionBar actionBar = getSupportActionBar();

        if (actionBar != null) {
            actionBar.setDisplayHomeAsUpEnabled(true);
            actionBar.setBackgroundDrawable(getResources().getDrawable(R.color.colorPrimary));
            actionBar.setTitle(R.string.title_settings);
        }
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
            Preference selectRom = findPreference(R.string.settings_key_select_rom);
            Preference factoryReset = findPreference(R.string.settings_key_factory_reset);

            Preference donate = findPreference(R.string.settings_key_donate);
            Preference sendLog = findPreference(R.string.settings_key_sendlog);
            Preference about = findPreference(R.string.settings_key_about);

            // Initialize verbose logging checkbox with profile-specific value
            verboseLogging.setChecked(ProfileSettings.isVerboseLoggingEnabled(getActivity()));
            verboseLogging.setOnPreferenceChangeListener((preference, newValue) -> {
                ProfileSettings.setVerboseLogging(getActivity(), (Boolean) newValue);
                return true;
            });

            // Initialize display configuration preferences
            android.preference.EditTextPreference displayWidthPref = (android.preference.EditTextPreference) displayWidth;
            android.preference.EditTextPreference displayHeightPref = (android.preference.EditTextPreference) displayHeight;
            android.preference.EditTextPreference displayDpiPref = (android.preference.EditTextPreference) displayDpi;

            displayWidthPref.setText(String.valueOf(ProfileSettings.getDisplayWidth(getActivity())));
            displayWidthPref.setSummary("Virtual display width in pixels (current: " + ProfileSettings.getDisplayWidth(getActivity()) + ")");
            displayWidthPref.setOnPreferenceChangeListener((preference, newValue) -> {
                try {
                    int width = Integer.parseInt(newValue.toString());
                    if (width >= MIN_VALUE && width <= MAX_DISPLAY_DIMENSION) {
                        ProfileSettings.setDisplayWidth(getActivity(), width);
                        displayWidthPref.setSummary("Virtual display width in pixels (current: " + width + ")");
                        Toast.makeText(getActivity(), R.string.settings_display_change_reboot, Toast.LENGTH_SHORT).show();
                        return true;
                    } else {
                        Toast.makeText(getActivity(), "Width must be between " + MIN_VALUE + " and " + MAX_DISPLAY_DIMENSION, Toast.LENGTH_SHORT).show();
                        return false;
                    }
                } catch (NumberFormatException e) {
                    Toast.makeText(getActivity(), "Invalid number", Toast.LENGTH_SHORT).show();
                    return false;
                }
            });

            displayHeightPref.setText(String.valueOf(ProfileSettings.getDisplayHeight(getActivity())));
            displayHeightPref.setSummary("Virtual display height in pixels (current: " + ProfileSettings.getDisplayHeight(getActivity()) + ")");
            displayHeightPref.setOnPreferenceChangeListener((preference, newValue) -> {
                try {
                    int height = Integer.parseInt(newValue.toString());
                    if (height >= MIN_VALUE && height <= MAX_DISPLAY_DIMENSION) {
                        ProfileSettings.setDisplayHeight(getActivity(), height);
                        displayHeightPref.setSummary("Virtual display height in pixels (current: " + height + ")");
                        Toast.makeText(getActivity(), R.string.settings_display_change_reboot, Toast.LENGTH_SHORT).show();
                        return true;
                    } else {
                        Toast.makeText(getActivity(), "Height must be between " + MIN_VALUE + " and " + MAX_DISPLAY_DIMENSION, Toast.LENGTH_SHORT).show();
                        return false;
                    }
                } catch (NumberFormatException e) {
                    Toast.makeText(getActivity(), "Invalid number", Toast.LENGTH_SHORT).show();
                    return false;
                }
            });

            displayDpiPref.setText(String.valueOf(ProfileSettings.getDisplayDpi(getActivity())));
            displayDpiPref.setSummary("Virtual display DPI (current: " + ProfileSettings.getDisplayDpi(getActivity()) + ")");
            displayDpiPref.setOnPreferenceChangeListener((preference, newValue) -> {
                try {
                    int dpi = Integer.parseInt(newValue.toString());
                    if (dpi >= MIN_VALUE && dpi <= MAX_DPI) {
                        ProfileSettings.setDisplayDpi(getActivity(), dpi);
                        displayDpiPref.setSummary("Virtual display DPI (current: " + dpi + ")");
                        Toast.makeText(getActivity(), R.string.settings_display_change_reboot, Toast.LENGTH_SHORT).show();
                        return true;
                    } else {
                        Toast.makeText(getActivity(), "DPI must be between " + MIN_VALUE + " and " + MAX_DPI, Toast.LENGTH_SHORT).show();
                        return false;
                    }
                } catch (NumberFormatException e) {
                    Toast.makeText(getActivity(), "Invalid number", Toast.LENGTH_SHORT).show();
                    return false;
                }
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
                    Toast.makeText(getContext(), "Error", Toast.LENGTH_SHORT).show();
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

            donate.setOnPreferenceClickListener(preference -> {
                Context context = getContext();
                if (context instanceof Activity) {
                    UIHelper.showDonateDialog((Activity) context);
                    return true;
                }
                return false;
            });

            sendLog.setOnPreferenceClickListener(preference -> {
                Context context = getActivity();
                byte[] bugreport = LogEvents.getBugreport(context);
                File tmpLog = new File(context.getCacheDir(), "bugreport.zip");
                try {
                    Files.write(tmpLog.toPath(), bugreport);
                } catch (IOException e) {
                    Crashes.trackError(e);
                }
                Uri uri = FileProvider.getUriForFile(context, "io.twoyi.fileprovider", tmpLog);

                Intent shareIntent = new Intent(Intent.ACTION_SEND);
                shareIntent.putExtra(Intent.EXTRA_STREAM, uri);
                shareIntent.setDataAndType(uri, "application/zip");
                shareIntent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);

                context.startActivity(Intent.createChooser(shareIntent, getString(R.string.settings_key_sendlog)));

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

        private void importRomForActiveProfile(Uri uri) {
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
                
                File tempFile = new File(activity.getCacheDir(), "rootfs_import.tar");

                ContentResolver contentResolver = activity.getContentResolver();
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
                    RomManager.initRootfs(activity);
                }
                
                return exitCode == 0;
            }).done(result -> {
                UIHelper.dismiss(dialog);
                if (result) {
                    Toast.makeText(activity, "ROM imported successfully. Please reboot.", Toast.LENGTH_SHORT).show();
                } else {
                    Toast.makeText(activity, "Failed to import ROM", Toast.LENGTH_SHORT).show();
                }
            }).fail(result -> activity.runOnUiThread(() -> {
                Toast.makeText(activity, "Error importing ROM: " + result.getMessage(), Toast.LENGTH_SHORT).show();
                dialog.dismiss();
            }));
        }
    }
}
