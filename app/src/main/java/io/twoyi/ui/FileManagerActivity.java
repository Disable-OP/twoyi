/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.ui;

import android.app.Activity;
import android.app.AlertDialog;
import android.os.Bundle;
import android.system.ErrnoException;
import android.system.Os;
import android.text.InputType;
import android.view.Menu;
import android.view.MenuItem;
import android.view.View;
import android.view.ViewGroup;
import android.widget.BaseAdapter;
import android.widget.EditText;
import android.widget.ListView;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

import androidx.annotation.NonNull;
import androidx.core.content.ContextCompat;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;

import io.twoyi.R;
import io.twoyi.utils.IOUtils;
import io.twoyi.utils.RomManager;

/**
 * 6-Z264: rootfs file manager.
 *
 * Browses the ACTIVE PROFILE's backing rootfs (the same directory tree
 * the guest sees as {@code /}) directly from the host side, and can
 * create, edit, rename, delete and CHMOD entries. This is a
 * HOST-side maintenance tool: the VFS guest-isolation invariants are
 * about what the GUEST may observe — the app owning and servicing its
 * own data directory is exactly the supported administration surface.
 *
 * Feature set (user request: "add a file manager for rootfs to modify
 * everything and edit every file and it's permissions"):
 *   - browse any directory under the rootfs
 *   - view/edit text files (bounded at {@link #MAX_EDIT_BYTES})
 *   - create files and folders
 *   - rename / delete (recursively)
 *   - read + rewrite POSIX permissions in octal (android.system.Os.chmod)
 *   - per-entry details (size, octal mode)
 *
 * A warning banner shows when a container guest is (or was recently)
 * running, because the guest may have files open and host-side edits
 * can be shadowed by the guest's staged copies.
 */
public class FileManagerActivity extends Activity {

    private static final String TAG = "FileManager";
    /** Files larger than this are shown read-only (avoid OOM in a dialog). */
    private static final int MAX_EDIT_BYTES = 256 * 1024;

    private File mCurrentDir;
    private final List<File> mEntries = new ArrayList<>();
    private EntryAdapter mAdapter;
    private TextView mPathView;
    private TextView mWarningView;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.ac_file_manager);

        mPathView = findViewById(R.id.fmPath);
        mWarningView = findViewById(R.id.fmWarning);
        ListView list = findViewById(R.id.fmList);
        mAdapter = new EntryAdapter();
        list.setAdapter(mAdapter);
        list.setEmptyView(findViewById(android.R.id.empty));

        // The path bar doubles as the "go up" affordance.
        mPathView.setOnClickListener(v -> goUp());

        list.setOnItemClickListener((parent, view, position, id) -> {
            File f = mEntries.get(position);
            if (f.isDirectory()) {
                enter(f);
            } else {
                promptOpenFile(f);
            }
        });
        list.setOnItemLongClickListener((parent, view, position, id) -> {
            promptEntryActions(mEntries.get(position));
            return true;
        });

        File root = RomManager.getRootfsDir(this);
        if (savedInstanceState != null) {
            String saved = savedInstanceState.getString("dir");
            if (saved != null) {
                File f = new File(saved);
                if (f.isDirectory()) {
                    root = f;
                }
            }
        }
        enter(root);
    }

    @Override
    protected void onSaveInstanceState(@NonNull Bundle outState) {
        super.onSaveInstanceState(outState);
        outState.putString("dir", mCurrentDir.getAbsolutePath());
    }

    private void enter(File dir) {
        mCurrentDir = dir;
        refresh();
    }

    private void goUp() {
        File parent = mCurrentDir.getParentFile();
        File root = RomManager.getRootfsDir(this);
        if (parent != null && parent.getAbsolutePath().startsWith(root.getAbsolutePath())
                && !mCurrentDir.equals(root)) {
            enter(parent);
        }
    }

    private void refresh() {
        mEntries.clear();
        File[] files = mCurrentDir.listFiles();
        if (files != null) {
            mEntries.addAll(Arrays.asList(files));
            Collections.sort(mEntries, (a, b) -> {
                if (a.isDirectory() != b.isDirectory()) {
                    return a.isDirectory() ? -1 : 1;
                }
                return a.getName().compareToIgnoreCase(b.getName());
            });
        }
        mPathView.setText(mCurrentDir.getAbsolutePath());
        boolean running = Render2ActivityRef.containerRunning();
        mWarningView.setVisibility(running ? View.VISIBLE : View.GONE);
        mAdapter.notifyDataSetChanged();
    }

    private static String modeOctal(File f) {
        try {
            return Integer.toString((int) (Os.stat(f.getAbsolutePath()).st_mode & 07777), 8);
        } catch (Throwable t) {
            return "?";
        }
    }

    private static String sizeLabel(File f) {
        if (f.isDirectory()) {
            return "DIR";
        }
        long n = f.length();
        if (n < 1024) {
            return n + " B";
        }
        if (n < 1024 * 1024) {
            return String.format("%.1f KB", n / 1024.0);
        }
        return String.format("%.1f MB", n / (1024.0 * 1024.0));
    }

    // ── actions ────────────────────────────────────────────────────────

    private void promptOpenFile(File f) {
        long len = f.length();
        if (len > MAX_EDIT_BYTES) {
            promptDetails(f);
            return;
        }
        new AlertDialog.Builder(this)
                .setTitle(f.getName())
                .setItems(new CharSequence[]{
                                getString(R.string.fm_action_edit),
                                getString(R.string.fm_action_details)},
                        (d, which) -> {
                            if (which == 0) {
                                promptEdit(f);
                            } else {
                                promptDetails(f);
                            }
                        })
                .show();
    }

    private void promptEntryActions(File f) {
        new AlertDialog.Builder(this)
                .setTitle(f.getName())
                .setItems(new CharSequence[]{
                                getString(R.string.fm_action_chmod),
                                getString(R.string.fm_action_rename),
                                getString(R.string.fm_action_delete),
                                getString(R.string.fm_action_details)},
                        (d, which) -> {
                            switch (which) {
                                case 0:
                                    promptChmod(f);
                                    break;
                                case 1:
                                    promptRename(f);
                                    break;
                                case 2:
                                    promptDelete(f);
                                    break;
                                default:
                                    promptDetails(f);
                                    break;
                            }
                        })
                .show();
    }

    private void promptDetails(File f) {
        StringBuilder sb = new StringBuilder();
        sb.append(getString(R.string.fm_details_path)).append(": ").append(f.getAbsolutePath()).append('\n');
        sb.append(getString(R.string.fm_details_size)).append(": ").append(sizeLabel(f)).append('\n');
        sb.append(getString(R.string.fm_details_mode)).append(": ").append(modeOctal(f));
        new AlertDialog.Builder(this).setTitle(f.getName()).setMessage(sb).show();
    }

    private void promptChmod(File f) {
        EditText input = new EditText(this);
        input.setInputType(InputType.TYPE_CLASS_TEXT);
        input.setText(modeOctal(f));
        input.setSelection(input.getText().length());
        new AlertDialog.Builder(this)
                .setTitle(getString(R.string.fm_chmod_title, f.getName()))
                .setMessage(R.string.fm_chmod_message)
                .setView(input)
                .setPositiveButton(android.R.string.ok, (d, w) -> {
                    String s = input.getText().toString().trim();
                    try {
                        if (!s.matches("[0-7]{3,4}")) {
                            throw new IllegalArgumentException(s);
                        }
                        Os.chmod(f.getAbsolutePath(), Integer.parseInt(s, 8));
                        refresh();
                    } catch (ErrnoException e) {
                        Toast.makeText(this, getString(R.string.fm_chmod_failed, e.errno), Toast.LENGTH_LONG).show();
                    } catch (Exception e) {
                        Toast.makeText(this, R.string.fm_chmod_invalid, Toast.LENGTH_LONG).show();
                    }
                })
                .setNegativeButton(android.R.string.cancel, null)
                .show();
    }

    private void promptRename(File f) {
        EditText input = new EditText(this);
        input.setText(f.getName());
        input.setSelection(input.getText().length());
        new AlertDialog.Builder(this)
                .setTitle(R.string.fm_rename_title)
                .setView(input)
                .setPositiveButton(android.R.string.ok, (d, w) -> {
                    String name = input.getText().toString().trim();
                    if (name.isEmpty() || name.contains("/")) {
                        Toast.makeText(this, R.string.fm_invalid_name, Toast.LENGTH_SHORT).show();
                        return;
                    }
                    File target = new File(f.getParentFile(), name);
                    if (f.renameTo(target)) {
                        refresh();
                    } else {
                        Toast.makeText(this, R.string.fm_rename_failed, Toast.LENGTH_SHORT).show();
                    }
                })
                .setNegativeButton(android.R.string.cancel, null)
                .show();
    }

    private void promptDelete(File f) {
        new AlertDialog.Builder(this)
                .setTitle(R.string.fm_delete_title)
                .setMessage(getString(R.string.fm_delete_message, f.getName()))
                .setPositiveButton(android.R.string.ok, (d, w) -> {
                    boolean ok = f.isDirectory()
                            ? IOUtils.deleteDirectory(f)
                            : f.delete();
                    if (ok) {
                        refresh();
                    } else {
                        Toast.makeText(this, R.string.fm_delete_failed, Toast.LENGTH_SHORT).show();
                    }
                })
                .setNegativeButton(android.R.string.cancel, null)
                .show();
    }

    private void promptEdit(File f) {
        byte[] bytes;
        try (FileInputStream in = new FileInputStream(f)) {
            bytes = new byte[(int) Math.min(f.length(), MAX_EDIT_BYTES)];
            int read = 0, n;
            while (read < bytes.length && (n = in.read(bytes, read, bytes.length - read)) > 0) {
                read += n;
            }
        } catch (Exception e) {
            Toast.makeText(this, R.string.fm_read_failed, Toast.LENGTH_SHORT).show();
            return;
        }
        final boolean truncated = f.length() > bytes.length;
        String content = new String(bytes, StandardCharsets.UTF_8);

        EditText editor = new EditText(this);
        editor.setText(content, TextView.BufferType.EDITABLE);
        editor.setTextIsSelectable(true);
        editor.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_MULTI_LINE
                | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        editor.setTypeface(android.graphics.Typeface.MONOSPACE);
        editor.setTextSize(12f);
        editor.setMinLines(6);

        ScrollView scroller = new ScrollView(this);
        scroller.addView(editor);

        new AlertDialog.Builder(this)
                .setTitle(f.getName() + (truncated ? getString(R.string.fm_edit_truncated_suffix) : ""))
                .setView(scroller)
                .setPositiveButton(R.string.fm_save, (d, w) -> {
                    // FileOutputStream on an EXISTING file truncates in
                    // place — the inode (and therefore its POSIX mode) is
                    // preserved, so permissions survive every save.
                    try (FileOutputStream out = new FileOutputStream(f)) {
                        out.write(editor.getText().toString().getBytes(StandardCharsets.UTF_8));
                        refresh();
                    } catch (Exception e) {
                        Toast.makeText(this, R.string.fm_save_failed, Toast.LENGTH_SHORT).show();
                    }
                })
                .setNegativeButton(android.R.string.cancel, null)
                .show();
    }

    private void promptCreate(final boolean folder) {
        EditText input = new EditText(this);
        input.setHint(folder ? R.string.fm_new_folder : R.string.fm_new_file);
        new AlertDialog.Builder(this)
                .setTitle(folder ? R.string.fm_new_folder : R.string.fm_new_file)
                .setView(input)
                .setPositiveButton(android.R.string.ok, (d, w) -> {
                    String name = input.getText().toString().trim();
                    if (name.isEmpty() || name.contains("/")) {
                        Toast.makeText(this, R.string.fm_invalid_name, Toast.LENGTH_SHORT).show();
                        return;
                    }
                    File target = new File(mCurrentDir, name);
                    boolean ok = folder ? target.mkdirs() || target.isDirectory() : createNewFile(target);
                    if (ok) {
                        refresh();
                    } else {
                        Toast.makeText(this, R.string.fm_create_failed, Toast.LENGTH_SHORT).show();
                    }
                })
                .setNegativeButton(android.R.string.cancel, null)
                .show();
    }

    private static boolean createNewFile(File target) {
        try {
            return target.createNewFile();
        } catch (Exception e) {
            return false;
        }
    }

    // ── options menu ───────────────────────────────────────────────────

    @Override
    public boolean onCreateOptionsMenu(Menu menu) {
        getMenuInflater().inflate(R.menu.menu_file_manager, menu);
        return true;
    }

    @Override
    public boolean onOptionsItemSelected(@NonNull MenuItem item) {
        int id = item.getItemId();
        if (id == R.id.action_new_folder) {
            promptCreate(true);
            return true;
        } else if (id == R.id.action_new_file) {
            promptCreate(false);
            return true;
        } else if (id == R.id.action_refresh) {
            refresh();
            return true;
        }
        return super.onOptionsItemSelected(item);
    }

    @Override
    public void onBackPressed() {
        File root = RomManager.getRootfsDir(this);
        if (!mCurrentDir.equals(root)) {
            goUp();
        } else {
            super.onBackPressed();
        }
    }

    // ── adapter ────────────────────────────────────────────────────────

    private class EntryAdapter extends BaseAdapter {
        @Override
        public int getCount() {
            return mEntries.size();
        }

        @Override
        public File getItem(int position) {
            return mEntries.get(position);
        }

        @Override
        public long getItemId(int position) {
            return position;
        }

        @Override
        public View getView(int position, View convertView, ViewGroup parent) {
            View v = convertView;
            if (v == null) {
                v = getLayoutInflater().inflate(R.layout.item_file_row, parent, false);
            }
            File f = getItem(position);
            TextView name = v.findViewById(R.id.fileName);
            TextView info = v.findViewById(R.id.fileInfo);
            name.setText((f.isDirectory() ? "▸ " : "") + f.getName());
            info.setText(modeOctal(f) + " · " + sizeLabel(f));
            name.setTextColor(f.canWrite()
                    ? ContextCompat.getColor(FileManagerActivity.this, android.R.color.primary_text_light)
                    : ContextCompat.getColor(FileManagerActivity.this, android.R.color.darker_gray));
            return v;
        }
    }

    /** Indirection so the class name stays short in this file. */
    private static final class Render2ActivityRef {
        static boolean containerRunning() {
            return io.twoyi.Render2Activity.containerRunning;
        }
    }
}
