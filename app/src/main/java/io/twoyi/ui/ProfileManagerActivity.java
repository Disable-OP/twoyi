/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.ui;

import android.app.Activity;
import android.app.ProgressDialog;
import android.content.ContentResolver;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.view.MenuItem;
import android.view.View;
import android.view.ViewGroup;
import android.widget.BaseExpandableListAdapter;
import android.widget.ExpandableListView;
import android.widget.TextView;
import android.widget.Toast;

import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import androidx.appcompat.app.ActionBar;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.content.FileProvider;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import java.util.List;

import io.twoyi.R;
import io.twoyi.utils.ProfileManager;
import io.twoyi.utils.RomManager;
import io.twoyi.utils.UIHelper;

/**
 * Activity for managing profiles with expandable list UI
 */
public class ProfileManagerActivity extends AppCompatActivity {

    private static final int REQUEST_IMPORT_PROFILE = 1001;

    private ExpandableListView mListView;
    private ProfilesAdapter mAdapter;
    private String mPendingImportProfileName;

    @Override
    protected void onCreate(@Nullable Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        
        setContentView(R.layout.activity_profile_manager);
        
        ActionBar actionBar = getSupportActionBar();
        if (actionBar != null) {
            actionBar.setDisplayHomeAsUpEnabled(true);
            actionBar.setTitle(R.string.profile_manager_title);
        }

        mListView = findViewById(R.id.profilesList);
        refreshProfiles();

        // Add click listener for child items
        mListView.setOnChildClickListener((parent, v, groupPosition, childPosition, id) -> {
            handleProfileAction(groupPosition, childPosition);
            return true;
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

    private void refreshProfiles() {
        List<String> profiles = ProfileManager.getProfiles(this);
        String activeProfile = ProfileManager.getActiveProfile(this);
        
        mAdapter = new ProfilesAdapter(profiles, activeProfile);
        mListView.setAdapter(mAdapter);
    }

    private void handleProfileAction(int groupPosition, int childPosition) {
        String profileName = mAdapter.getGroup(groupPosition);
        String action = mAdapter.getChild(groupPosition, childPosition);

        if (action.equals(getString(R.string.profile_switch))) {
            switchProfile(profileName);
        } else if (action.equals(getString(R.string.profile_rename))) {
            showRenameDialog(profileName);
        } else if (action.equals(getString(R.string.profile_copy))) {
            showCopyDialog(profileName);
        } else if (action.equals(getString(R.string.profile_export))) {
            exportProfile(profileName);
        } else if (action.equals(getString(R.string.profile_delete))) {
            confirmDelete(profileName);
        } else if (action.equals(getString(R.string.profile_from_scratch))) {
            createEmptyProfile(profileName);
        } else if (action.equals(getString(R.string.profile_from_import))) {
            mPendingImportProfileName = profileName;
            Intent intent = new Intent(Intent.ACTION_GET_CONTENT);
            intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, false);
            intent.setType("*/*");
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            try {
                startActivityForResult(intent, REQUEST_IMPORT_PROFILE);
            } catch (Throwable ignored) {
                Toast.makeText(this, "Error", Toast.LENGTH_SHORT).show();
            }
        }
    }

    private void switchProfile(String profileName) {
        UIHelper.getDialogBuilder(this)
            .setMessage(getString(R.string.profile_switch_confirm, profileName))
            .setPositiveButton(android.R.string.ok, (d, w) -> {
                if (ProfileManager.switchProfile(this, profileName)) {
                    RomManager.reboot(this);
                } else {
                    Toast.makeText(this, "Failed to switch profile", Toast.LENGTH_SHORT).show();
                }
            })
            .setNegativeButton(android.R.string.cancel, null)
            .show();
    }

    private void showRenameDialog(String oldName) {
        android.widget.EditText input = new android.widget.EditText(this);
        input.setHint(R.string.profile_name_hint);
        input.setText(oldName);

        UIHelper.getDialogBuilder(this)
            .setTitle(R.string.profile_rename)
            .setView(input)
            .setPositiveButton(android.R.string.ok, (dialog, which) -> {
                String newName = input.getText().toString().trim();
                if (!newName.isEmpty() && !newName.equals(oldName)) {
                    if (ProfileManager.renameProfile(this, oldName, newName)) {
                        Toast.makeText(this, "Profile renamed to: " + newName, Toast.LENGTH_SHORT).show();
                        refreshProfiles();
                        if (oldName.equals(ProfileManager.getActiveProfile(this))) {
                            RomManager.reboot(this);
                        }
                    } else {
                        Toast.makeText(this, "Failed to rename profile", Toast.LENGTH_SHORT).show();
                    }
                }
            })
            .setNegativeButton(android.R.string.cancel, null)
            .show();
    }

    private void showCopyDialog(String sourceName) {
        android.widget.EditText input = new android.widget.EditText(this);
        input.setHint(R.string.profile_copy_hint);
        input.setText(sourceName + "_copy");

        UIHelper.getDialogBuilder(this)
            .setTitle(R.string.profile_copy)
            .setView(input)
            .setPositiveButton(android.R.string.ok, (dialog, which) -> {
                String targetName = input.getText().toString().trim();
                if (!targetName.isEmpty()) {
                    ProgressDialog progressDialog = UIHelper.getProgressDialog(this);
                    progressDialog.setCancelable(false);
                    progressDialog.show();

                    UIHelper.defer().when(() -> {
                        return ProfileManager.copyProfile(this, sourceName, targetName);
                    }).done(success -> {
                        UIHelper.dismiss(progressDialog);
                        if (success) {
                            Toast.makeText(this, "Profile copied to: " + targetName, Toast.LENGTH_SHORT).show();
                            refreshProfiles();
                        } else {
                            Toast.makeText(this, "Failed to copy profile", Toast.LENGTH_SHORT).show();
                        }
                    }).fail(result -> runOnUiThread(() -> {
                        UIHelper.dismiss(progressDialog);
                        Toast.makeText(this, "Error: " + result.getMessage(), Toast.LENGTH_SHORT).show();
                    }));
                }
            })
            .setNegativeButton(android.R.string.cancel, null)
            .show();
    }

    private void exportProfile(String profileName) {
        ProgressDialog dialog = UIHelper.getProgressDialog(this);
        dialog.setCancelable(false);
        dialog.show();

        UIHelper.defer().when(() -> {
            File profileDir = ProfileManager.getProfileDir(this, profileName);
            File exportFile = new File(getCacheDir(), "profile_" + profileName + "_export.tar");
            
            String profileDirPath = profileDir.getAbsolutePath();
            String exportFilePath = exportFile.getAbsolutePath();
            String parentPath = profileDir.getParent();
            
            if (profileDirPath.contains(";") || profileDirPath.contains("&") || 
                exportFilePath.contains(";") || exportFilePath.contains("&")) {
                throw new SecurityException("Invalid path detected");
            }
            
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
            
            Uri uri = FileProvider.getUriForFile(this, "io.twoyi.fileprovider", exportFile);
            Intent shareIntent = new Intent(Intent.ACTION_SEND);
            shareIntent.putExtra(Intent.EXTRA_STREAM, uri);
            shareIntent.setDataAndType(uri, "application/x-tar");
            shareIntent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            startActivity(Intent.createChooser(shareIntent, getString(R.string.profile_export)));
            
            Toast.makeText(this, R.string.profile_export_success, Toast.LENGTH_SHORT).show();
        }).fail(result -> runOnUiThread(() -> {
            Toast.makeText(this, getString(R.string.profile_export_failed, result.getMessage()), Toast.LENGTH_SHORT).show();
            dialog.dismiss();
        }));
    }

    private void confirmDelete(String profileName) {
        UIHelper.getDialogBuilder(this)
            .setMessage(getString(R.string.profile_delete_confirm, profileName))
            .setPositiveButton(android.R.string.ok, (d, w) -> {
                if (ProfileManager.deleteProfile(this, profileName)) {
                    Toast.makeText(this, "Profile deleted: " + profileName, Toast.LENGTH_SHORT).show();
                    refreshProfiles();
                } else {
                    Toast.makeText(this, "Failed to delete profile", Toast.LENGTH_SHORT).show();
                }
            })
            .setNegativeButton(android.R.string.cancel, null)
            .show();
    }

    private void createEmptyProfile(String profileName) {
        if (ProfileManager.createProfile(this, profileName)) {
            Toast.makeText(this, "Profile created: " + profileName, Toast.LENGTH_SHORT).show();
            refreshProfiles();
        } else {
            Toast.makeText(this, "Failed to create profile", Toast.LENGTH_SHORT).show();
        }
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, @Nullable Intent data) {
        super.onActivityResult(requestCode, resultCode, data);

        if (requestCode == REQUEST_IMPORT_PROFILE && resultCode == Activity.RESULT_OK) {
            if (data != null && data.getData() != null && mPendingImportProfileName != null) {
                performProfileImport(data.getData(), mPendingImportProfileName);
            }
        }
    }

    private void performProfileImport(Uri uri, String profileName) {
        ProgressDialog dialog = UIHelper.getProgressDialog(this);
        dialog.setCancelable(false);
        dialog.show();

        UIHelper.defer().when(() -> {
            File profileDir = ProfileManager.getProfileDir(this, profileName);
            
            if (profileDir.exists()) {
                throw new IOException("Profile already exists: " + profileName);
            }

            File tempFile = new File(getCacheDir(), "profile_import.tar");

            ContentResolver contentResolver = getContentResolver();
            try (InputStream inputStream = contentResolver.openInputStream(uri);
                 OutputStream os = new FileOutputStream(tempFile)) {
                byte[] buffer = new byte[8192];
                int count;
                while ((count = inputStream.read(buffer)) > 0) {
                    os.write(buffer, 0, count);
                }
            }

            profileDir.mkdirs();

            String tempFilePath = tempFile.getAbsolutePath();
            String profilesPath = ProfileManager.getProfilesDir(this).getAbsolutePath();
            
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
                Toast.makeText(this, R.string.profile_import_success, Toast.LENGTH_SHORT).show();
                refreshProfiles();
            } else {
                Toast.makeText(this, R.string.profile_import_failed, Toast.LENGTH_SHORT).show();
            }
        }).fail(result -> runOnUiThread(() -> {
            Toast.makeText(this, getString(R.string.profile_import_failed, result.getMessage()), Toast.LENGTH_SHORT).show();
            dialog.dismiss();
        }));
    }

    private class ProfilesAdapter extends BaseExpandableListAdapter {
        private final List<String> mProfiles;
        private final String mActiveProfile;
        private final String mNewProfilePlaceholder;

        public ProfilesAdapter(List<String> profiles, String activeProfile) {
            mProfiles = new ArrayList<>(profiles);
            mActiveProfile = activeProfile;
            mNewProfilePlaceholder = getString(R.string.profile_create);
            mProfiles.add(mNewProfilePlaceholder);
        }

        @Override
        public int getGroupCount() {
            return mProfiles.size();
        }

        @Override
        public int getChildrenCount(int groupPosition) {
            String profile = mProfiles.get(groupPosition);
            if (profile.equals(mNewProfilePlaceholder)) {
                return 2; // From scratch, From import
            } else if (profile.equals(mActiveProfile)) {
                return 4; // Rename, Copy, Export, Delete (no switch for active)
            } else {
                return 5; // Switch, Rename, Copy, Export, Delete
            }
        }

        @Override
        public String getGroup(int groupPosition) {
            return mProfiles.get(groupPosition);
        }

        @Override
        public String getChild(int groupPosition, int childPosition) {
            String profile = mProfiles.get(groupPosition);
            if (profile.equals(mNewProfilePlaceholder)) {
                return childPosition == 0 ? getString(R.string.profile_from_scratch) : getString(R.string.profile_from_import);
            } else if (profile.equals(mActiveProfile)) {
                switch (childPosition) {
                    case 0: return getString(R.string.profile_rename);
                    case 1: return getString(R.string.profile_copy);
                    case 2: return getString(R.string.profile_export);
                    case 3: return getString(R.string.profile_delete);
                    default: return "";
                }
            } else {
                switch (childPosition) {
                    case 0: return getString(R.string.profile_switch);
                    case 1: return getString(R.string.profile_rename);
                    case 2: return getString(R.string.profile_copy);
                    case 3: return getString(R.string.profile_export);
                    case 4: return getString(R.string.profile_delete);
                    default: return "";
                }
            }
        }

        @Override
        public long getGroupId(int groupPosition) {
            return groupPosition;
        }

        @Override
        public long getChildId(int groupPosition, int childPosition) {
            return childPosition;
        }

        @Override
        public boolean hasStableIds() {
            return false;
        }

        @Override
        public View getGroupView(int groupPosition, boolean isExpanded, View convertView, ViewGroup parent) {
            if (convertView == null) {
                convertView = getLayoutInflater().inflate(android.R.layout.simple_expandable_list_item_1, parent, false);
            }
            
            TextView textView = convertView.findViewById(android.R.id.text1);
            String profile = mProfiles.get(groupPosition);
            
            if (profile.equals(mActiveProfile) && !profile.equals(mNewProfilePlaceholder)) {
                textView.setText(profile + " (" + getString(R.string.profile_active) + ")");
            } else {
                textView.setText(profile);
            }
            
            return convertView;
        }

        @Override
        public View getChildView(int groupPosition, int childPosition, boolean isLastChild, View convertView, ViewGroup parent) {
            if (convertView == null) {
                convertView = getLayoutInflater().inflate(android.R.layout.simple_list_item_1, parent, false);
            }
            
            TextView textView = convertView.findViewById(android.R.id.text1);
            textView.setText("  " + getChild(groupPosition, childPosition));
            
            return convertView;
        }

        @Override
        public boolean isChildSelectable(int groupPosition, int childPosition) {
            return true;
        }
    }
}
