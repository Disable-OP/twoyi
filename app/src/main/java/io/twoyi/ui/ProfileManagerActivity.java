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
import android.content.SharedPreferences;
import android.net.Uri;
import android.os.Bundle;
import android.util.Log;
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
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.List;

import io.twoyi.R;
import io.twoyi.utils.IOUtils;
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
            showCreateProfileDialog(false);
        } else if (action.equals(getString(R.string.profile_from_import))) {
            showCreateProfileDialog(true);
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
            File profileRootfs = ProfileManager.getProfileRootfsDir(this, profileName);
            File exportFile = new File(getCacheDir(), "profile_" + profileName + "_export.tar");
            File tempDir = new File(getCacheDir(), "profile_export_temp");
            
            // Clean up temp directory if it exists
            if (tempDir.exists()) {
                IOUtils.deleteDirectory(tempDir);
            }
            tempDir.mkdirs();
            
            try {
                // Export SharedPreferences to XML file
                File prefsXml = new File(tempDir, "preference.xml");
                SharedPreferences prefs = getSharedPreferences("profile_settings_" + profileName, Context.MODE_PRIVATE);
                exportPreferencesToXml(prefs, prefsXml);
                
                // Copy rootfs contents to temp directory if rootfs exists
                if (profileRootfs.exists() && profileRootfs.isDirectory()) {
                    File[] rootfsFiles = profileRootfs.listFiles();
                    if (rootfsFiles != null) {
                        for (File file : rootfsFiles) {
                            File destFile = new File(tempDir, file.getName());
                            if (file.isDirectory()) {
                                // For directories, create symlink or copy
                                if (Files.isSymbolicLink(file.toPath())) {
                                    Path linkTarget = Files.readSymbolicLink(file.toPath());
                                    Files.createSymbolicLink(destFile.toPath(), linkTarget);
                                } else {
                                    // Create hard link or copy
                                    try {
                                        Files.createLink(destFile.toPath(), file.toPath());
                                    } catch (Exception e) {
                                        // If hard link fails, create symlink to original
                                        Files.createSymbolicLink(destFile.toPath(), file.toPath());
                                    }
                                }
                            } else if (Files.isSymbolicLink(file.toPath())) {
                                Path linkTarget = Files.readSymbolicLink(file.toPath());
                                Files.createSymbolicLink(destFile.toPath(), linkTarget);
                            } else {
                                // For regular files, create hard link or copy
                                try {
                                    Files.createLink(destFile.toPath(), file.toPath());
                                } catch (Exception e) {
                                    // If hard link fails, copy the file
                                    Files.copy(file.toPath(), destFile.toPath(), StandardCopyOption.REPLACE_EXISTING);
                                }
                            }
                        }
                    }
                }
                
                String tempDirPath = tempDir.getAbsolutePath();
                String exportFilePath = exportFile.getAbsolutePath();
                
                if (tempDirPath.contains(";") || tempDirPath.contains("&") || 
                    exportFilePath.contains(";") || exportFilePath.contains("&")) {
                    throw new SecurityException("Invalid path detected");
                }
                
                // Create single tar archive with all contents (preserve symlinks)
                ProcessBuilder pb = new ProcessBuilder(
                    "tar", "-cf", exportFilePath,
                    "-C", tempDirPath, "."
                );
                pb.redirectErrorStream(true);
                Process process = pb.start();
                
                // Read any error output
                java.io.BufferedReader reader = new java.io.BufferedReader(
                    new java.io.InputStreamReader(process.getInputStream()));
                StringBuilder output = new StringBuilder();
                String line;
                while ((line = reader.readLine()) != null) {
                    output.append(line).append("\n");
                }
                
                int exitCode = process.waitFor();
                
                if (exitCode != 0) {
                    Log.e("ProfileManager", "tar create output: " + output.toString());
                    throw new IOException("tar command failed with exit code: " + exitCode);
                }
                
                return exportFile;
            } finally {
                // Clean up temp directory
                IOUtils.deleteDirectory(tempDir);
            }
        }).done(exportFile -> {
            UIHelper.dismiss(dialog);
            
            // Share the file like log export
            Uri uri = FileProvider.getUriForFile(this, "io.twoyi.fileprovider", exportFile);
            Intent shareIntent = new Intent(Intent.ACTION_SEND);
            shareIntent.putExtra(Intent.EXTRA_STREAM, uri);
            shareIntent.setDataAndType(uri, "application/x-tar");
            shareIntent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            startActivity(Intent.createChooser(shareIntent, getString(R.string.profile_export)));
        }).fail(result -> runOnUiThread(() -> {
            UIHelper.dismiss(dialog);
            Toast.makeText(this, getString(R.string.profile_export_failed, result.getMessage()), Toast.LENGTH_SHORT).show();
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

    private void showCreateProfileDialog(boolean isImport) {
        android.widget.EditText input = new android.widget.EditText(this);
        input.setHint(R.string.profile_name_hint);

        UIHelper.getDialogBuilder(this)
            .setTitle(R.string.profile_create)
            .setView(input)
            .setPositiveButton(android.R.string.ok, (dialog, which) -> {
                String profileName = input.getText().toString().trim();
                if (!profileName.isEmpty()) {
                    if (isImport) {
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
                    } else {
                        createEmptyProfile(profileName);
                    }
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
            File profileRootfs = ProfileManager.getProfileRootfsDir(this, profileName);
            
            if (profileDir.exists()) {
                throw new IOException("Profile already exists: " + profileName);
            }

            File tempFile = new File(getCacheDir(), "profile_import.tar");
            File tempExtractDir = new File(getCacheDir(), "profile_import_temp");

            // Clean up temp extract directory if it exists
            if (tempExtractDir.exists()) {
                IOUtils.deleteDirectory(tempExtractDir);
            }
            tempExtractDir.mkdirs();

            try {
                // Copy uploaded file to temp
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
                String tempExtractPath = tempExtractDir.getAbsolutePath();
                
                if (tempFilePath.contains(";") || tempFilePath.contains("&") ||
                    tempExtractPath.contains(";") || tempExtractPath.contains("&")) {
                    throw new SecurityException("Invalid path detected");
                }
                
                // Extract tar to temp directory
                ProcessBuilder pb = new ProcessBuilder(
                    "tar", "-xf", tempFilePath,
                    "-C", tempExtractPath
                );
                Process process = pb.start();
                int exitCode = process.waitFor();

                if (exitCode != 0) {
                    throw new IOException("tar extract failed with exit code: " + exitCode);
                }

                // Create profile directory structure
                profileDir.mkdirs();
                profileRootfs.mkdirs();

                // Import preferences from XML if it exists
                File prefsXml = new File(tempExtractDir, "preference.xml");
                if (prefsXml.exists()) {
                    SharedPreferences prefs = getSharedPreferences("profile_settings_" + profileName, Context.MODE_PRIVATE);
                    importPreferencesFromXml(prefsXml, prefs);
                }

                // Move all files except preference.xml to rootfs
                File[] files = tempExtractDir.listFiles();
                if (files != null) {
                    for (File file : files) {
                        if (!file.getName().equals("preference.xml")) {
                            File targetFile = new File(profileRootfs, file.getName());
                            if (file.isDirectory()) {
                                moveDirectory(file, targetFile);
                            } else {
                                Files.move(file.toPath(), targetFile.toPath(), StandardCopyOption.REPLACE_EXISTING);
                            }
                        }
                    }
                }

                return true;
            } finally {
                // Clean up temp files
                tempFile.delete();
                IOUtils.deleteDirectory(tempExtractDir);
            }
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

    /**
     * Export SharedPreferences to XML file
     */
    private void exportPreferencesToXml(SharedPreferences prefs, File xmlFile) throws IOException {
        StringBuilder xml = new StringBuilder();
        xml.append("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        xml.append("<preferences>\n");
        
        for (String key : prefs.getAll().keySet()) {
            Object value = prefs.getAll().get(key);
            if (value instanceof Boolean) {
                xml.append("  <boolean name=\"").append(key).append("\" value=\"").append(value).append("\" />\n");
            } else if (value instanceof String) {
                String escapedValue = ((String) value).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;");
                xml.append("  <string name=\"").append(key).append("\">").append(escapedValue).append("</string>\n");
            } else if (value instanceof Integer) {
                xml.append("  <int name=\"").append(key).append("\" value=\"").append(value).append("\" />\n");
            } else if (value instanceof Long) {
                xml.append("  <long name=\"").append(key).append("\" value=\"").append(value).append("\" />\n");
            } else if (value instanceof Float) {
                xml.append("  <float name=\"").append(key).append("\" value=\"").append(value).append("\" />\n");
            }
        }
        
        xml.append("</preferences>\n");
        IOUtils.writeContent(xmlFile, xml.toString());
    }

    /**
     * Import SharedPreferences from XML file
     */
    private void importPreferencesFromXml(File xmlFile, SharedPreferences prefs) throws IOException {
        String xmlContent = IOUtils.readContent(xmlFile);
        if (xmlContent == null || xmlContent.isEmpty()) {
            return;
        }

        SharedPreferences.Editor editor = prefs.edit();
        editor.clear();

        // Simple XML parsing (avoiding full XML parser for minimal dependencies)
        String[] lines = xmlContent.split("\n");
        for (String line : lines) {
            line = line.trim();
            
            try {
                if (line.startsWith("<boolean")) {
                    String name = extractAttribute(line, "name");
                    String value = extractAttribute(line, "value");
                    if (name != null && value != null) {
                        editor.putBoolean(name, Boolean.parseBoolean(value));
                    }
                } else if (line.startsWith("<string")) {
                    String name = extractAttribute(line, "name");
                    String value = extractTagContent(line);
                    if (name != null && value != null) {
                        // Unescape XML entities (order matters: do &amp; last to avoid re-escaping)
                        value = value.replace("&quot;", "\"").replace("&gt;", ">").replace("&lt;", "<").replace("&amp;", "&");
                        editor.putString(name, value);
                    }
                } else if (line.startsWith("<int")) {
                    String name = extractAttribute(line, "name");
                    String value = extractAttribute(line, "value");
                    if (name != null && value != null) {
                        editor.putInt(name, Integer.parseInt(value));
                    }
                } else if (line.startsWith("<long")) {
                    String name = extractAttribute(line, "name");
                    String value = extractAttribute(line, "value");
                    if (name != null && value != null) {
                        editor.putLong(name, Long.parseLong(value));
                    }
                } else if (line.startsWith("<float")) {
                    String name = extractAttribute(line, "name");
                    String value = extractAttribute(line, "value");
                    if (name != null && value != null) {
                        editor.putFloat(name, Float.parseFloat(value));
                    }
                }
            } catch (NumberFormatException e) {
                // Skip malformed numeric values
                Log.w("ProfileManager", "Skipping malformed preference value in line: " + line, e);
            }
        }

        editor.commit();
    }

    /**
     * Extract attribute value from XML tag
     */
    private String extractAttribute(String line, String attributeName) {
        String pattern = attributeName + "=\"";
        int start = line.indexOf(pattern);
        if (start == -1) {
            return null;
        }
        start += pattern.length();
        int end = line.indexOf("\"", start);
        if (end == -1) {
            return null;
        }
        return line.substring(start, end);
    }

    /**
     * Extract content from XML tag
     */
    private String extractTagContent(String line) {
        int start = line.indexOf(">") + 1;
        int end = line.indexOf("<", start);
        if (start <= 0 || end == -1) {
            return null;
        }
        return line.substring(start, end);
    }

    /**
     * Move directory recursively, preserving symlinks
     */
    private void moveDirectory(File source, File target) throws IOException {
        if (!target.exists()) {
            target.mkdirs();
        }

        File[] files = source.listFiles();
        if (files != null) {
            for (File file : files) {
                File targetFile = new File(target, file.getName());
                Path sourcePath = file.toPath();
                Path targetPath = targetFile.toPath();
                
                if (Files.isSymbolicLink(sourcePath)) {
                    Path linkTarget = Files.readSymbolicLink(sourcePath);
                    Files.createSymbolicLink(targetPath, linkTarget);
                    Files.delete(sourcePath);
                } else if (file.isDirectory()) {
                    moveDirectory(file, targetFile);
                    Files.delete(sourcePath);
                } else {
                    Files.move(sourcePath, targetPath, StandardCopyOption.REPLACE_EXISTING);
                }
            }
        }
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
