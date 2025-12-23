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

            factoryReset.setOnPreferenceClickListener(preference -> {
                UIHelper.getDialogBuilder(getActivity())
                        .setTitle(android.R.string.dialog_alert_title)
                        .setMessage(R.string.factory_reset_confirm_message)
                        .setPositiveButton(R.string.i_confirm_it, (dialog, which) -> {
                            // Clear the rootfs completely so next boot will prompt for ROM
                            Activity activity = getActivity();
                            if (activity != null) {
                                File rootfsDir = RomManager.getRootfsDir(activity);
                                io.twoyi.utils.IOUtils.deleteDirectory(rootfsDir);
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
            // No longer handling profile import here - handled by ProfileManagerActivity
        }
    }
}
            Activity activity = getActivity();
            if (activity == null) return;

            String[] actions = {
                getString(R.string.profile_switch), 
                getString(R.string.profile_rename),
                getString(R.string.profile_copy),
                getString(R.string.profile_export),
                getString(R.string.profile_delete)
            };
            
            UIHelper.getDialogBuilder(activity)
                .setTitle(profileName)
                .setItems(actions, (dialog, which) -> {
                    switch (which) {
                        case 0: // Switch
                            UIHelper.getDialogBuilder(activity)
                                .setMessage(getString(R.string.profile_switch_confirm, profileName))
                                .setPositiveButton(android.R.string.ok, (d, w) -> {
                                    if (ProfileManager.switchProfile(activity, profileName)) {
                                        RomManager.reboot(activity);
                                    } else {
                                        Toast.makeText(activity, "Failed to switch profile", Toast.LENGTH_SHORT).show();
                                    }
                                })
                                .setNegativeButton(android.R.string.cancel, null)
                                .show();
                            break;
                        case 1: // Rename
                            showRenameProfileDialog(profileName);
                            break;
                        case 2: // Copy
                            showCopyProfileDialog(profileName);
                            break;
                        case 3: // Export
                            exportProfile(profileName);
                            break;
                        case 4: // Delete
                            UIHelper.getDialogBuilder(activity)
                                .setMessage(getString(R.string.profile_delete_confirm, profileName))
                                .setPositiveButton(android.R.string.ok, (d, w) -> {
                                    if (ProfileManager.deleteProfile(activity, profileName)) {
                                        Toast.makeText(activity, "Profile deleted: " + profileName, Toast.LENGTH_SHORT).show();
                                    } else {
                                        Toast.makeText(activity, "Failed to delete profile", Toast.LENGTH_SHORT).show();
                                    }
                                })
                                .setNegativeButton(android.R.string.cancel, null)
                                .show();
                            break;
                    }
                })
                .show();
        }

        private void showCreateProfileDialog() {
            Activity activity = getActivity();
            if (activity == null) return;

            android.widget.EditText input = new android.widget.EditText(activity);
            input.setHint(R.string.profile_name_hint);

            UIHelper.getDialogBuilder(activity)
                .setTitle(R.string.profile_create)
                .setView(input)
                .setPositiveButton(android.R.string.ok, (dialog, which) -> {
                    String profileName = input.getText().toString().trim();
                    if (!profileName.isEmpty()) {
                        if (ProfileManager.createProfile(activity, profileName)) {
                            Toast.makeText(activity, "Profile created: " + profileName, Toast.LENGTH_SHORT).show();
                        } else {
                            Toast.makeText(activity, "Failed to create profile", Toast.LENGTH_SHORT).show();
                        }
                    }
                })
                .setNegativeButton(android.R.string.cancel, null)
                .show();
        }

        private void showRenameProfileDialog(String oldName) {
            Activity activity = getActivity();
            if (activity == null) return;

            android.widget.EditText input = new android.widget.EditText(activity);
            input.setHint(R.string.profile_name_hint);
            input.setText(oldName);

            UIHelper.getDialogBuilder(activity)
                .setTitle(R.string.profile_rename)
                .setView(input)
                .setPositiveButton(android.R.string.ok, (dialog, which) -> {
                    String newName = input.getText().toString().trim();
                    if (!newName.isEmpty() && !newName.equals(oldName)) {
                        if (ProfileManager.renameProfile(activity, oldName, newName)) {
                            Toast.makeText(activity, "Profile renamed to: " + newName, Toast.LENGTH_SHORT).show();
                            if (oldName.equals(ProfileManager.getActiveProfile(activity))) {
                                // Active profile was renamed, need to reboot
                                RomManager.reboot(activity);
                            }
                        } else {
                            Toast.makeText(activity, "Failed to rename profile", Toast.LENGTH_SHORT).show();
                        }
                    }
                })
                .setNegativeButton(android.R.string.cancel, null)
                .show();
        }

        private void showCopyProfileDialog(String sourceName) {
            Activity activity = getActivity();
            if (activity == null) return;

            android.widget.EditText input = new android.widget.EditText(activity);
            input.setHint(R.string.profile_copy_hint);
            input.setText(sourceName + "_copy");

            UIHelper.getDialogBuilder(activity)
                .setTitle(R.string.profile_copy)
                .setView(input)
                .setPositiveButton(android.R.string.ok, (dialog, which) -> {
                    String targetName = input.getText().toString().trim();
                    if (!targetName.isEmpty()) {
                        ProgressDialog progressDialog = UIHelper.getProgressDialog(activity);
                        progressDialog.setCancelable(false);
                        progressDialog.show();

                        UIHelper.defer().when(() -> {
                            return ProfileManager.copyProfile(activity, sourceName, targetName);
                        }).done(success -> {
                            UIHelper.dismiss(progressDialog);
                            if (success) {
                                Toast.makeText(activity, "Profile copied to: " + targetName, Toast.LENGTH_SHORT).show();
                            } else {
                                Toast.makeText(activity, "Failed to copy profile", Toast.LENGTH_SHORT).show();
                            }
                        }).fail(result -> activity.runOnUiThread(() -> {
                            UIHelper.dismiss(progressDialog);
                            Toast.makeText(activity, "Error: " + result.getMessage(), Toast.LENGTH_SHORT).show();
                        }));
                    }
                })
                .setNegativeButton(android.R.string.cancel, null)
                .show();
        }

        private void exportProfile(String profileName) {
            Activity activity = getActivity();
            if (activity == null) return;

            mCurrentProfileForOperation = profileName;

            ProgressDialog dialog = UIHelper.getProgressDialog(activity);
            dialog.setCancelable(false);
            dialog.show();

            UIHelper.defer().when(() -> {
                File profileDir = ProfileManager.getProfileDir(activity, profileName);
                File exportFile = new File(activity.getCacheDir(), "profile_" + profileName + "_export.tar");
                
                // Validate paths to prevent command injection
                String profileDirPath = profileDir.getAbsolutePath();
                String exportFilePath = exportFile.getAbsolutePath();
                String parentPath = profileDir.getParent();
                
                if (profileDirPath.contains(";") || profileDirPath.contains("&") || 
                    exportFilePath.contains(";") || exportFilePath.contains("&")) {
                    throw new SecurityException("Invalid path detected");
                }
                
                // Create tar archive using ProcessBuilder
                ProcessBuilder pb = new ProcessBuilder(
                    "tar", "-cf", exportFilePath,
                    "-C", parentPath, profileDir.getName()
                );
                Process process = pb.start();
                int exitCode = process.waitFor();
                
                if (exitCode != 0) {
                    throw new IOException("tar command failed with exit code: " + exitCode);
                }
                
                return exportFile;
            }).done(exportFile -> {
                UIHelper.dismiss(dialog);
                
                // Share the file
                Uri uri = FileProvider.getUriForFile(activity, "io.twoyi.fileprovider", exportFile);
                Intent shareIntent = new Intent(Intent.ACTION_SEND);
                shareIntent.putExtra(Intent.EXTRA_STREAM, uri);
                shareIntent.setDataAndType(uri, "application/x-tar");
                shareIntent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                activity.startActivity(Intent.createChooser(shareIntent, getString(R.string.profile_export)));
                
                Toast.makeText(activity, R.string.profile_export_success, Toast.LENGTH_SHORT).show();
            }).fail(result -> activity.runOnUiThread(() -> {
                Toast.makeText(activity, getString(R.string.profile_export_failed, result.getMessage()), Toast.LENGTH_SHORT).show();
                dialog.dismiss();
            }));
        }

        private void showProfileManagerDialog() {
            Activity activity = getActivity();
            if (activity == null) return;

            List<String> profiles = ProfileManager.getProfiles(activity);
            String activeProfile = ProfileManager.getActiveProfile(activity);
            
            String[] items = new String[profiles.size() + 2];
            for (int i = 0; i < profiles.size(); i++) {
                String profile = profiles.get(i);
                if (profile.equals(activeProfile)) {
                    items[i] = profile + " (" + getString(R.string.profile_active) + ")";
                } else {
                    items[i] = profile;
                }
            }
            items[profiles.size()] = getString(R.string.profile_create);
            items[profiles.size() + 1] = getString(R.string.profile_import);

            UIHelper.getDialogBuilder(activity)
                .setTitle(R.string.profile_manager_title)
                .setItems(items, (dialog, which) -> {
                    if (which == profiles.size()) {
                        // Create new profile
                        showCreateProfileDialog();
                    } else if (which == profiles.size() + 1) {
                        // Import profile
                        importProfile();
                    } else {
                        String selectedProfile = profiles.get(which);
                        if (selectedProfile.equals(activeProfile)) {
                            Toast.makeText(activity, R.string.profile_active, Toast.LENGTH_SHORT).show();
                        } else {
                            showProfileActionsDialog(selectedProfile);
                        }
                    }
                })
                .show();
        }

        private void importProfile() {
            Intent intent = new Intent(Intent.ACTION_GET_CONTENT);
            intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, false);
            intent.setType("*/*");
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            try {
                startActivityForResult(intent, REQUEST_IMPORT_PROFILE);
            } catch (Throwable ignored) {
                Toast.makeText(getContext(), "Error", Toast.LENGTH_SHORT).show();
            }
        }

        private void exportRootfsToTar() {
            Activity activity = getActivity();
            if (activity == null) return;

            ProgressDialog dialog = UIHelper.getProgressDialog(activity);
            dialog.setCancelable(false);
            dialog.show();

            UIHelper.defer().when(() -> {
                File rootfsDir = RomManager.getRootfsDir(activity);
                File exportFile = new File(activity.getCacheDir(), "rootfs_export.tar");
                
                // Validate paths to prevent command injection
                String rootfsDirPath = rootfsDir.getAbsolutePath();
                String exportFilePath = exportFile.getAbsolutePath();
                String parentPath = rootfsDir.getParent();
                
                // Ensure paths don't contain shell metacharacters
                if (rootfsDirPath.contains(";") || rootfsDirPath.contains("&") || 
                    exportFilePath.contains(";") || exportFilePath.contains("&")) {
                    throw new SecurityException("Invalid path detected");
                }
                
                // Create tar archive using ProcessBuilder for better security
                ProcessBuilder pb = new ProcessBuilder(
                    "tar", "-cf", exportFilePath,
                    "-C", parentPath, rootfsDir.getName()
                );
                Process process = pb.start();
                int exitCode = process.waitFor();
                
                if (exitCode != 0) {
                    throw new IOException("tar command failed with exit code: " + exitCode);
                }
                
                return exportFile;
            }).done(exportFile -> {
                UIHelper.dismiss(dialog);
                
                // Share the file
                Uri uri = FileProvider.getUriForFile(activity, "io.twoyi.fileprovider", exportFile);
                Intent shareIntent = new Intent(Intent.ACTION_SEND);
                shareIntent.putExtra(Intent.EXTRA_STREAM, uri);
                shareIntent.setDataAndType(uri, "application/x-tar");
                shareIntent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                activity.startActivity(Intent.createChooser(shareIntent, getString(R.string.settings_key_export_rootfs)));
                
                Toast.makeText(activity, R.string.export_rootfs_success, Toast.LENGTH_SHORT).show();
            }).fail(result -> activity.runOnUiThread(() -> {
                Toast.makeText(activity, getString(R.string.export_rootfs_failed, result.getMessage()), Toast.LENGTH_SHORT).show();
                dialog.dismiss();
            }));
        }

        private void importRootfsFromFile(Uri uri) {
            Activity activity = getActivity();
            if (activity == null) return;

            ProgressDialog dialog = UIHelper.getProgressDialog(activity);
            dialog.setCancelable(false);
            dialog.show();

            UIHelper.defer().when(() -> {
                File rootfsDir = RomManager.getRootfsDir(activity);
                
                // Get file name to determine type
                String fileName = uri.getLastPathSegment();
                if (fileName == null) {
                    fileName = "import";
                }
                
                File tempFile;
                boolean is7z = fileName.endsWith(".7z");
                
                if (is7z) {
                    tempFile = new File(activity.getCacheDir(), "rootfs_import.7z");
                } else {
                    tempFile = new File(activity.getCacheDir(), "rootfs_import.tar");
                }

                // Copy file to temp location
                ContentResolver contentResolver = activity.getContentResolver();
                try (InputStream inputStream = contentResolver.openInputStream(uri);
                     OutputStream os = new FileOutputStream(tempFile)) {
                    byte[] buffer = new byte[8192];
                    int count;
                    while ((count = inputStream.read(buffer)) > 0) {
                        os.write(buffer, 0, count);
                    }
                }

                // Delete old rootfs
                io.twoyi.utils.IOUtils.deleteDirectory(rootfsDir);
                rootfsDir.getParentFile().mkdirs();

                int exitCode;
                if (is7z) {
                    // Extract 7z archive using RomManager
                    exitCode = RomManager.extractRootfs(activity, tempFile);
                } else {
                    // Validate paths to prevent command injection
                    String tempFilePath = tempFile.getAbsolutePath();
                    String parentPath = rootfsDir.getParent();
                    
                    if (tempFilePath.contains(";") || tempFilePath.contains("&") ||
                        parentPath.contains(";") || parentPath.contains("&")) {
                        throw new SecurityException("Invalid path detected");
                    }
                    
                    // Extract tar archive using ProcessBuilder for better security
                    ProcessBuilder pb = new ProcessBuilder(
                        "tar", "-xf", tempFilePath,
                        "-C", parentPath
                    );
                    Process process = pb.start();
                    exitCode = process.waitFor();
                }

                tempFile.delete();
                return exitCode == 0;
            }).done(result -> {
                UIHelper.dismiss(dialog);
                if (result) {
                    Toast.makeText(activity, R.string.import_rootfs_success, Toast.LENGTH_SHORT).show();
                } else {
                    Toast.makeText(activity, R.string.import_rootfs_failed, Toast.LENGTH_SHORT).show();
                }
            }).fail(result -> activity.runOnUiThread(() -> {
                Toast.makeText(activity, getString(R.string.import_rootfs_failed, result.getMessage()), Toast.LENGTH_SHORT).show();
                dialog.dismiss();
            }));
        }


        @Override
        public void onActivityResult(int requestCode, int resultCode, @Nullable Intent data) {
            super.onActivityResult(requestCode, resultCode, data);

            if (requestCode == REQUEST_IMPORT_PROFILE && resultCode == Activity.RESULT_OK) {
                if (data != null && data.getData() != null) {
                    importProfileFromFile(data.getData());
                }
            }
        }

        private void importProfileFromFile(Uri uri) {
            Activity activity = getActivity();
            if (activity == null) return;

            // Ask for profile name
            android.widget.EditText input = new android.widget.EditText(activity);
            input.setHint(R.string.profile_name_hint);

            UIHelper.getDialogBuilder(activity)
                .setTitle(R.string.profile_import)
                .setView(input)
                .setPositiveButton(android.R.string.ok, (dialog, which) -> {
                    String profileName = input.getText().toString().trim();
                    if (!profileName.isEmpty()) {
                        performProfileImport(uri, profileName);
                    }
                })
                .setNegativeButton(android.R.string.cancel, null)
                .show();
        }

        private void performProfileImport(Uri uri, String profileName) {
            Activity activity = getActivity();
            if (activity == null) return;

            ProgressDialog dialog = UIHelper.getProgressDialog(activity);
            dialog.setCancelable(false);
            dialog.show();

            UIHelper.defer().when(() -> {
                File profileDir = ProfileManager.getProfileDir(activity, profileName);
                
                if (profileDir.exists()) {
                    throw new IOException("Profile already exists: " + profileName);
                }

                File tempFile = new File(activity.getCacheDir(), "profile_import.tar");

                // Copy file to temp location
                ContentResolver contentResolver = activity.getContentResolver();
                try (InputStream inputStream = contentResolver.openInputStream(uri);
                     OutputStream os = new FileOutputStream(tempFile)) {
                    byte[] buffer = new byte[8192];
                    int count;
                    while ((count = inputStream.read(buffer)) > 0) {
                        os.write(buffer, 0, count);
                    }
                }

                // Create profile directory
                profileDir.mkdirs();

                // Extract tar archive
                String tempFilePath = tempFile.getAbsolutePath();
                String profilesPath = ProfileManager.getProfilesDir(activity).getAbsolutePath();
                
                if (tempFilePath.contains(";") || tempFilePath.contains("&") ||
                    profilesPath.contains(";") || profilesPath.contains("&")) {
                    throw new SecurityException("Invalid path detected");
                }
                
                ProcessBuilder pb = new ProcessBuilder(
                    "tar", "-xf", tempFilePath,
                    "-C", profilesPath
                );
                Process process = pb.start();
                int exitCode = process.waitFor();

                tempFile.delete();
                return exitCode == 0;
            }).done(result -> {
                UIHelper.dismiss(dialog);
                if (result) {
                    Toast.makeText(activity, R.string.profile_import_success, Toast.LENGTH_SHORT).show();
                } else {
                    Toast.makeText(activity, R.string.profile_import_failed, Toast.LENGTH_SHORT).show();
                }
            }).fail(result -> activity.runOnUiThread(() -> {
                Toast.makeText(activity, getString(R.string.profile_import_failed, result.getMessage()), Toast.LENGTH_SHORT).show();
                dialog.dismiss();
            }));
        }
    }
}
