/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.provider;

import android.database.Cursor;
import android.database.MatrixCursor;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract.Document;
import android.provider.DocumentsContract.Root;
import android.provider.DocumentsProvider;
import android.webkit.MimeTypeMap;

import androidx.annotation.Nullable;

import java.io.File;
import java.io.FileNotFoundException;
import java.io.IOException;

import io.twoyi.R;

/**
 * @author weishu
 * @date 2022/2/16.
 */
public class TwoyiDocumentsProvider extends DocumentsProvider {

    private static final String ALL_MIME_TYPES = "*/*";

    private static final String DEFAULT_ROOT_ID = "0";

    private static final String[] DEFAULT_ROOT_PROJECTION = new String[]{
            Root.COLUMN_ROOT_ID,
            Root.COLUMN_MIME_TYPES,
            Root.COLUMN_FLAGS,
            Root.COLUMN_ICON,
            Root.COLUMN_TITLE,
            Root.COLUMN_SUMMARY,
            Root.COLUMN_DOCUMENT_ID,
            Root.COLUMN_AVAILABLE_BYTES
    };

    private static final String[] DEFAULT_DOCUMENT_PROJECTION = new String[]{
            Document.COLUMN_DOCUMENT_ID,
            Document.COLUMN_MIME_TYPE,
            Document.COLUMN_DISPLAY_NAME,
            Document.COLUMN_LAST_MODIFIED,
            Document.COLUMN_FLAGS,
            Document.COLUMN_SIZE
    };

    @Override
    public Cursor queryRoots(String[] projection) {

        File BASE_DIR = getContext().getDataDir();

        final MatrixCursor result = new MatrixCursor(projection != null ? projection : DEFAULT_ROOT_PROJECTION);
        final String applicationName = getContext().getString(R.string.app_name);

        final MatrixCursor.RowBuilder row = result.newRow();
        row.add(Root.COLUMN_ROOT_ID, DEFAULT_ROOT_ID);
        row.add(Root.COLUMN_DOCUMENT_ID, getDocId(BASE_DIR));
        row.add(Root.COLUMN_SUMMARY, getRootSummary());
        row.add(Root.COLUMN_FLAGS, Root.FLAG_SUPPORTS_CREATE | Root.FLAG_LOCAL_ONLY | Root.FLAG_SUPPORTS_IS_CHILD);
        row.add(Root.COLUMN_TITLE, applicationName);
        row.add(Root.COLUMN_MIME_TYPES, ALL_MIME_TYPES);
        row.add(Root.COLUMN_AVAILABLE_BYTES, BASE_DIR.getFreeSpace());
        row.add(Root.COLUMN_ICON, R.mipmap.ic_launcher);

        return result;
    }

    @Override
    public Cursor queryDocument(String documentId, String[] projection) throws FileNotFoundException {
        final MatrixCursor result = new MatrixCursor(projection != null ? projection : DEFAULT_DOCUMENT_PROJECTION);
        includeFile(result, documentId, null);
        return result;
    }

    @Override
    public Cursor queryChildDocuments(String parentDocumentId, String[] projection, String sortOrder) throws FileNotFoundException {
        final MatrixCursor result = new MatrixCursor(projection != null ? projection : DEFAULT_DOCUMENT_PROJECTION);
        final File parent = getFileById(parentDocumentId);
        File[] files = parent.listFiles();
        if (files != null) {
            for (File file : files) {
                includeFile(result, null, file);
            }
        }
        return result;
    }

    @Override
    public ParcelFileDescriptor openDocument(String documentId, String mode, @Nullable CancellationSignal signal) throws FileNotFoundException {
        final File file = getFileById(documentId);
        final int accessMode = ParcelFileDescriptor.parseMode(mode);
        return ParcelFileDescriptor.open(file, accessMode);
    }

    @Override
    public boolean onCreate() {
        return true;
    }

    @Override
    public String createDocument(String parentDocumentId, String mimeType, String displayName) throws FileNotFoundException {
        // Fixed: displayName comes from caller-supplied data and was passed
        // straight to `new File(parent, displayName)`. A malicious SAF client
        // could supply "../../other/file" or "/etc/passwd" as displayName and
        // escape the provider root. Reject any name that:
        //   - contains a path separator (File.separatorChar or '/')
        //   - is "." or ".." or empty
        //   - is a Windows-style drive prefix
        // This mirrors the restrictions AOSP's own FileSystemProvider applies.
        if (displayName == null || displayName.isEmpty()
                || displayName.indexOf('/') >= 0
                || displayName.indexOf(File.separatorChar) >= 0
                || ".".equals(displayName)
                || "..".equals(displayName)) {
            throw new FileNotFoundException("Invalid display name: " + displayName);
        }
        File parentFile = new File(parentDocumentId);
        // Defense-in-depth: make sure the parent itself is inside our root
        // before we create anything inside it.
        if (!isWithinRoot(parentFile)) {
            throw new FileNotFoundException("Parent outside root: " + parentDocumentId);
        }
        File newFile = new File(parentFile, displayName);
        int noConflictId = 2;
        while (newFile.exists()) {
            newFile = new File(parentFile, displayName + " (" + noConflictId++ + ")");
        }
        try {
            boolean succeeded;
            if (Document.MIME_TYPE_DIR.equals(mimeType)) {
                succeeded = newFile.mkdir();
            } else {
                succeeded = newFile.createNewFile();
            }
            if (!succeeded) {
                throw new FileNotFoundException("Failed to create document: " + newFile.getPath());
            }
        } catch (IOException e) {
            throw new FileNotFoundException("Failed to create document:" + newFile.getPath());
        }
        return newFile.getPath();
    }

    @Override
    public void deleteDocument(String documentId) throws FileNotFoundException {
        File file = getFileById(documentId);
        if (!file.delete()) {
            throw new FileNotFoundException("Failed to delete document:" + documentId);
        }
    }

    @Override
    public String getDocumentType(String documentId) throws FileNotFoundException {
        File file = getFileById(documentId);
        return getMimeType(file);
    }

    @Override
    public boolean isChildDocument(String parentDocumentId, String documentId) {
        if (parentDocumentId == null || documentId == null) {
            return false;
        }
        // Fixed: was using String.startsWith which is vulnerable to:
        // 1. Sibling-prefix bypass: "/data/files_evil" startsWith "/data/files"
        // 2. Path traversal: "/data/files/../../other" startsWith "/data/files"
        // Now: canonicalize both paths and use Path.startsWith
        try {
            java.nio.file.Path parent = new File(parentDocumentId).getCanonicalFile().toPath();
            java.nio.file.Path child = new File(documentId).getCanonicalFile().toPath();
            return child.startsWith(parent);
        } catch (java.io.IOException e) {
            return false;
        }
    }

    private static String getDocId(File file) {
        return file.getAbsolutePath();
    }

    private File getFileById(String docId) throws FileNotFoundException {
        final File f = new File(docId);
        if (!f.exists()) throw new FileNotFoundException(f.getAbsolutePath() + " not found");
        // Security: verify the file is within the allowed root directory.
        // Without this, any app with a SAF tree grant can read/write/delete
        // arbitrary files by passing an absolute path as documentId.
        // (Path traversal via "../../" is also blocked by canonicalization.)
        //
        // Fixed: the previous revision's comment claimed "we also check here
        // as defense-in-depth" but the method returned `f` without any check.
        // A persisted URI grant for the root would let a client reach any
        // path the io.twoyi UID can read, just by tampering with the
        // documentId segment of the URI.
        //
        // Also: this method was previously `static`, which forced it to
        // skip the root check (isWithinRoot needs getContext()). Made it
        // non-static so the check can run; all callers go through the
        // instance already.
        if (!isWithinRoot(f)) {
            throw new FileNotFoundException(f.getAbsolutePath() + " outside provider root");
        }
        return f;
    }

    /// Check if a file is within the provider's root directory.
    /// Uses canonical paths to prevent symlink and "../" traversal attacks.
    private boolean isWithinRoot(File file) {
        try {
            File root = getRootDir().getCanonicalFile();
            File canonical = file.getCanonicalFile();
            return canonical.toPath().startsWith(root.toPath());
        } catch (java.io.IOException e) {
            return false;
        }
    }

    private static String getMimeType(File file) {
        if (file.isDirectory()) {
            return Document.MIME_TYPE_DIR;
        } else {
            final String name = file.getName();
            final int lastDot = name.lastIndexOf('.');
            if (lastDot >= 0) {
                final String extension = name.substring(lastDot + 1).toLowerCase();
                final String mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension);
                if (mime != null) return mime;
            }
            return "application/octet-stream";
        }
    }

    private File getRootDir() {
        return getContext().getDataDir();
    }

    private String getRootSummary() {
        // Use the actual data directory — works in work profiles too
        return getContext().getDataDir().getAbsolutePath();
    }

    private void includeFile(MatrixCursor result, String docId, File file)
            throws FileNotFoundException {
        if (docId == null) {
            docId = getDocId(file);
        } else {
            file = getFileById(docId);
        }

        int flags = 0;
        if (file.isDirectory()) {
            if (file.canWrite()) flags |= Document.FLAG_DIR_SUPPORTS_CREATE;
        } else if (file.canWrite()) {
            flags |= Document.FLAG_SUPPORTS_WRITE;
        }

        File parentFile = file.getParentFile();
        if (parentFile != null && parentFile.canWrite()) flags |= Document.FLAG_SUPPORTS_DELETE;

        boolean isRoot = file.equals(getRootDir());

        String displayName = isRoot ? getRootSummary() : file.getName();
        final String mimeType = getMimeType(file);
        if (mimeType.startsWith("image/")) flags |= Document.FLAG_SUPPORTS_THUMBNAIL;

        final MatrixCursor.RowBuilder row = result.newRow();
        row.add(Document.COLUMN_DOCUMENT_ID, docId);
        row.add(Document.COLUMN_DISPLAY_NAME, displayName);
        row.add(Document.COLUMN_SIZE, file.length());
        row.add(Document.COLUMN_MIME_TYPE, mimeType);
        row.add(Document.COLUMN_LAST_MODIFIED, file.lastModified());
        row.add(Document.COLUMN_FLAGS, flags);
        row.add(Document.COLUMN_ICON, R.mipmap.ic_launcher);
    }
}
