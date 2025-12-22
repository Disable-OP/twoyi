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
import androidx.core.util.Pair;

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
import io.twoyi.utils.RomManager;
import io.twoyi.utils.UIHelper;

/**
 * @author weishu
 * @date 2022/1/2.
 */

public class SettingsActivity extends AppCompatActivity {

    private static final int REQUEST_GET_FILE = 1000;
    private static final int REQUEST_IMPORT_ROOTFS = 1001;
    private static final int REQUEST_EXPORT_ROOTFS = 1002;

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
            Preference importRootfs = findPreference(R.string.settings_key_import_rootfs);
            Preference exportRootfs = findPreference(R.string.settings_key_export_rootfs);
            Preference replaceRom = findPreference(R.string.settings_key_replace_rom);
            Preference factoryReset = findPreference(R.string.settings_key_factory_reset);

            Preference donate = findPreference(R.string.settings_key_donate);
            Preference sendLog = findPreference(R.string.settings_key_sendlog);
            Preference about = findPreference(R.string.settings_key_about);

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
                showProfileManagerDialog();
                return true;
            });

            importRootfs.setOnPreferenceClickListener(preference -> {
                Intent intent = new Intent(Intent.ACTION_GET_CONTENT);
                intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, false);
                intent.setType("*/*");
                intent.addCategory(Intent.CATEGORY_OPENABLE);
                try {
                    startActivityForResult(intent, REQUEST_IMPORT_ROOTFS);
                } catch (Throwable ignored) {
                    Toast.makeText(getContext(), "Error", Toast.LENGTH_SHORT).show();
                }
                return true;
            });

            exportRootfs.setOnPreferenceClickListener(preference -> {
                exportRootfsToTar();
                return true;
            });

            replaceRom.setOnPreferenceClickListener(preference -> {

                Intent intent = new Intent(Intent.ACTION_GET_CONTENT);

                // you can only select one rootfs
                intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, false);
                intent.setType("*/*"); // apk file
                intent.addCategory(Intent.CATEGORY_OPENABLE);
                try {
                    startActivityForResult(intent, REQUEST_GET_FILE);
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
                            AppKV.setBooleanConfig(getActivity(), AppKV.SHOULD_USE_THIRD_PARTY_ROM, false);
                            AppKV.setBooleanConfig(getActivity(), AppKV.FORCE_ROM_BE_RE_INSTALL, true);
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

        private void showProfileManagerDialog() {
            Activity activity = getActivity();
            if (activity == null) return;

            List<String> profiles = ProfileManager.getProfiles(activity);
            String activeProfile = ProfileManager.getActiveProfile(activity);
            
            String[] items = new String[profiles.size() + 1];
            for (int i = 0; i < profiles.size(); i++) {
                String profile = profiles.get(i);
                if (profile.equals(activeProfile)) {
                    items[i] = profile + " (" + getString(R.string.profile_active) + ")";
                } else {
                    items[i] = profile;
                }
            }
            items[profiles.size()] = getString(R.string.profile_create);

            UIHelper.getDialogBuilder(activity)
                .setTitle(R.string.profile_manager_title)
                .setItems(items, (dialog, which) -> {
                    if (which == profiles.size()) {
                        // Create new profile
                        showCreateProfileDialog();
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

        private void showProfileActionsDialog(String profileName) {
            Activity activity = getActivity();
            if (activity == null) return;

            String[] actions = {getString(R.string.profile_switch), getString(R.string.profile_delete)};
            
            UIHelper.getDialogBuilder(activity)
                .setTitle(profileName)
                .setItems(actions, (dialog, which) -> {
                    if (which == 0) {
                        // Switch profile
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
                    } else if (which == 1) {
                        // Delete profile
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
                    }
                })
                .show();
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
                
                // Create tar archive using system tar command
                Process process = Runtime.getRuntime().exec(new String[]{
                    "tar", "-cf", exportFile.getAbsolutePath(),
                    "-C", rootfsDir.getParent(), rootfsDir.getName()
                });
                process.waitFor();
                
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
                File tempFile = new File(activity.getCacheDir(), "rootfs_import.tar");

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
                rootfsDir.mkdirs();

                // Extract tar archive
                Process process = Runtime.getRuntime().exec(new String[]{
                    "tar", "-xf", tempFile.getAbsolutePath(),
                    "-C", rootfsDir.getParent()
                });
                process.waitFor();

                tempFile.delete();
                return true;
            }).done(result -> {
                UIHelper.dismiss(dialog);
                AppKV.setBooleanConfig(activity, AppKV.SHOULD_USE_THIRD_PARTY_ROM, false);
                Toast.makeText(activity, R.string.import_rootfs_success, Toast.LENGTH_SHORT).show();
            }).fail(result -> activity.runOnUiThread(() -> {
                Toast.makeText(activity, getString(R.string.import_rootfs_failed, result.getMessage()), Toast.LENGTH_SHORT).show();
                dialog.dismiss();
            }));
        }


        @Override
        public void onActivityResult(int requestCode, int resultCode, @Nullable Intent data) {
            super.onActivityResult(requestCode, resultCode, data);

            if (requestCode == REQUEST_IMPORT_ROOTFS && resultCode == Activity.RESULT_OK) {
                if (data != null && data.getData() != null) {
                    importRootfsFromFile(data.getData());
                }
                return;
            }

            if (!(requestCode == REQUEST_GET_FILE && resultCode == Activity.RESULT_OK)) {
                return;
            }

            if (data == null) {
                return;
            }

            Uri uri = data.getData();
            if (uri == null) {
                return;
            }

            Activity activity = getActivity();
            ProgressDialog dialog = UIHelper.getProgressDialog(activity);
            dialog.setCancelable(false);
            dialog.show();

            // start copy 3rd rom
            UIHelper.defer().when(() -> {

                File rootfs3rd = RomManager.get3rdRootfsFile(activity);

                ContentResolver contentResolver = activity.getContentResolver();
                try (InputStream inputStream = contentResolver.openInputStream(uri); OutputStream os = new FileOutputStream(rootfs3rd)) {
                    byte[] buffer = new byte[1024];
                    int count;
                    while ((count = inputStream.read(buffer)) > 0) {
                        os.write(buffer, 0, count);
                    }
                }

                RomManager.RomInfo romInfo = RomManager.getRomInfo(rootfs3rd);
                return Pair.create(rootfs3rd, romInfo);
            }).done(result -> {

                File rootfs3rd = result.first;
                RomManager.RomInfo romInfo = result.second;
                UIHelper.dismiss(dialog);

                // copy finished, show dialog confirm
                if (romInfo.isValid()) {

                    String author = romInfo.author;
                    UIHelper.getDialogBuilder(activity)
                            .setTitle(R.string.replace_rom_confirm_title)
                            .setMessage(getString(R.string.replace_rom_confirm_message, author, romInfo.version, romInfo.desc))
                            .setPositiveButton(R.string.i_confirm_it, (dialog1, which) -> {
                                AppKV.setBooleanConfig(activity, AppKV.SHOULD_USE_THIRD_PARTY_ROM, true);
                                AppKV.setBooleanConfig(activity, AppKV.FORCE_ROM_BE_RE_INSTALL, true);

                                dialog1.dismiss();

                                RomManager.reboot(getActivity());
                            })
                            .setNegativeButton(android.R.string.cancel, (dialog12, which) -> dialog12.dismiss())
                            .show();
                } else {
                    Toast.makeText(activity, R.string.replace_rom_invalid, Toast.LENGTH_SHORT).show();
                    rootfs3rd.delete();
                }
            }).fail(result -> activity.runOnUiThread(() -> {
                Toast.makeText(activity, getResources().getString(R.string.install_failed_reason, result.getMessage()), Toast.LENGTH_SHORT).show();
                dialog.dismiss();
                activity.finish();
            }));

        }
    }
}
