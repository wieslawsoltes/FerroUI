package org.ferroui.android;

import android.content.Context;
import android.content.res.AssetFileDescriptor;
import android.database.Cursor;
import android.net.Uri;
import android.provider.DocumentsContract;

import java.io.InputStream;
import java.util.ArrayList;

/**
 * The calls of the storage items and of the storage provider that can throw: queries of a content
 * resolver and calls of the documents contract. An exception a call into Java leaves pending ends
 * the native side, so each method catches what its call throws, keeps the text of the exception
 * for {@link #takeLastError} and answers with null, false or an empty value. What a failure means
 * is decided on the native side, which reads the error after the call.
 *
 * The methods are called on the UI thread only.
 */
final class StorageHelper {
    private StorageHelper() { }

    /** The exception of the last call that failed, until it is taken. */
    private static String lastError;

    /** The text of the exception the last call caught, or null; the text is forgotten. */
    static String takeLastError() {
        String error = lastError;
        lastError = null;
        return error;
    }

    private static void failed(Throwable exception) {
        lastError = String.valueOf(exception);
    }

    // ---- columns ------------------------------------------------------------------------------

    /**
     * The text of a column of the first row a query answers with, or null when there is no row,
     * no such column, or the query failed.
     */
    static String getColumnValue(Context context, Uri contentUri, String column, String selection,
            String[] selectionArgs) {
        lastError = null;
        try {
            String[] projection = { column };
            try (Cursor cursor = context.getContentResolver()
                    .query(contentUri, projection, selection, selectionArgs, null)) {
                if (cursor != null && cursor.moveToFirst()) {
                    int columnIndex = cursor.getColumnIndex(column);
                    if (columnIndex != -1) {
                        return cursor.getString(columnIndex);
                    }
                }
            }
        } catch (Exception e) {
            failed(e);
        }
        return null;
    }

    /**
     * The numbers of columns of the first row a query answers with, one element per column: the
     * number in decimal digits, null for a column the row does not have, or an exclamation mark
     * and the text of the exception when reading the column failed. Null when there is no row or
     * the query failed; an UnsupportedOperationException of the query is not kept as an error
     * when {@code silentUnsupported} is set.
     */
    static String[] queryLongs(Context context, Uri uri, String[] columns,
            boolean silentUnsupported) {
        lastError = null;
        try (Cursor cursor = context.getContentResolver()
                .query(uri, columns, null, null, null)) {
            if (cursor == null || !cursor.moveToFirst()) {
                return null;
            }
            String[] values = new String[columns.length];
            for (int i = 0; i < columns.length; i++) {
                try {
                    int columnIndex = cursor.getColumnIndex(columns[i]);
                    if (columnIndex != -1) {
                        values[i] = Long.toString(cursor.getLong(columnIndex));
                    }
                } catch (Exception e) {
                    values[i] = "!" + e;
                }
            }
            return values;
        } catch (UnsupportedOperationException e) {
            if (!silentUnsupported) {
                failed(e);
            }
            return null;
        } catch (Exception e) {
            failed(e);
            return null;
        }
    }

    /**
     * The rows a query of the children of a document answers with, as the texts of the columns
     * of the projection, row after row. Empty without a cursor; null when the query failed.
     */
    static String[] queryChildren(Context context, Uri childrenUri, String[] projection) {
        lastError = null;
        try (Cursor cursor = context.getContentResolver()
                .query(childrenUri, projection, null, null, null)) {
            ArrayList<String> rows = new ArrayList<>();
            if (cursor != null) {
                while (cursor.moveToNext()) {
                    for (int i = 0; i < projection.length; i++) {
                        rows.add(cursor.getString(i));
                    }
                }
            }
            return rows.toArray(new String[0]);
        } catch (Exception e) {
            failed(e);
            return null;
        }
    }

    /**
     * Whether a query of the document answers with a row, which is what the single document
     * file of AndroidX calls existing; false when the query failed.
     */
    static boolean documentExists(Context context, Uri uri) {
        lastError = null;
        String[] projection = { DocumentsContract.Document.COLUMN_DOCUMENT_ID };
        try (Cursor cursor = context.getContentResolver()
                .query(uri, projection, null, null, null)) {
            return cursor != null && cursor.getCount() > 0;
        } catch (Exception e) {
            failed(e);
            return false;
        }
    }

    // ---- the documents contract ---------------------------------------------------------------

    /** The document id of a tree URI, or null when the URI is not one. */
    static String treeDocumentId(Uri uri) {
        lastError = null;
        try {
            return DocumentsContract.getTreeDocumentId(uri);
        } catch (Exception e) {
            failed(e);
            return null;
        }
    }

    /** The document id of a document URI, or null when the URI is not one. */
    static String documentId(Uri uri) {
        lastError = null;
        try {
            return DocumentsContract.getDocumentId(uri);
        } catch (Exception e) {
            failed(e);
            return null;
        }
    }

    /**
     * The URI the properties of a folder are queried with: the document of the tree of the URI,
     * the URI itself when the contract answers that it does not support the URI, and null when
     * it fails in another way.
     */
    static Uri folderQueryUri(Uri uri) {
        lastError = null;
        try {
            try {
                String folderId = DocumentsContract.getTreeDocumentId(uri);
                return DocumentsContract.buildDocumentUriUsingTree(uri, folderId);
            } catch (UnsupportedOperationException e) {
                return uri;
            }
        } catch (Exception e) {
            failed(e);
            return null;
        }
    }

    /** Creates a document; null when the provider creates none or the call failed. */
    static Uri createDocument(Context context, Uri parentDocumentUri, String mimeType,
            String displayName) {
        lastError = null;
        try {
            return DocumentsContract.createDocument(context.getContentResolver(), parentDocumentUri,
                    mimeType, displayName);
        } catch (Exception e) {
            failed(e);
            return null;
        }
    }

    /** Deletes a document; false when the call failed. */
    static boolean deleteDocument(Context context, Uri documentUri) {
        lastError = null;
        try {
            DocumentsContract.deleteDocument(context.getContentResolver(), documentUri);
            return true;
        } catch (Exception e) {
            failed(e);
            return false;
        }
    }

    /** Moves a document (API 24); null when the provider moves none or the call failed. */
    static Uri moveDocument(Context context, Uri sourceDocumentUri, Uri sourceParentDocumentUri,
            Uri targetParentDocumentUri) {
        lastError = null;
        try {
            return DocumentsContract.moveDocument(context.getContentResolver(), sourceDocumentUri,
                    sourceParentDocumentUri, targetParentDocumentUri);
        } catch (Exception e) {
            failed(e);
            return null;
        }
    }

    // ---- permissions --------------------------------------------------------------------------

    /** Takes the persistable permission that was offered for a URI; false when the call failed. */
    static boolean takePersistableUriPermission(Context context, Uri uri, int modeFlags) {
        lastError = null;
        try {
            context.getContentResolver().takePersistableUriPermission(uri, modeFlags);
            return true;
        } catch (Exception e) {
            failed(e);
            return false;
        }
    }

    /** Gives a persistable permission for a URI back; false when the call failed. */
    static boolean releasePersistableUriPermission(Context context, Uri uri, int modeFlags) {
        lastError = null;
        try {
            context.getContentResolver().releasePersistableUriPermission(uri, modeFlags);
            return true;
        } catch (Exception e) {
            failed(e);
            return false;
        }
    }

    // ---- content ------------------------------------------------------------------------------

    /** Whether the content of a URI can be opened for reading; false when it cannot. */
    static boolean canOpenInputStream(Context context, Uri uri) {
        lastError = null;
        try (InputStream stream = context.getContentResolver().openInputStream(uri)) {
            return stream != null;
        } catch (Exception e) {
            failed(e);
            return false;
        }
    }

    /**
     * The file descriptor of the content of a URI, with the range of the content in it, which is
     * what the streams of a content resolver read and write through. Null when the provider
     * answers with none or the call failed.
     */
    static AssetFileDescriptor openAssetFileDescriptor(Context context, Uri uri, String mode) {
        lastError = null;
        try {
            return context.getContentResolver().openAssetFileDescriptor(uri, mode);
        } catch (Exception e) {
            failed(e);
            return null;
        }
    }

    /** The MIME types a URI can be read as that match the filter; null when the call failed. */
    static String[] getStreamTypes(Context context, Uri uri, String mimeTypeFilter) {
        lastError = null;
        try {
            return context.getContentResolver().getStreamTypes(uri, mimeTypeFilter);
        } catch (Exception e) {
            failed(e);
            return null;
        }
    }

    /**
     * The file descriptor of the content of a URI as a MIME type. Null when the provider answers
     * with none or the call failed.
     */
    static AssetFileDescriptor openTypedAssetFileDescriptor(Context context, Uri uri,
            String mimeType) {
        lastError = null;
        try {
            return context.getContentResolver().openTypedAssetFileDescriptor(uri, mimeType, null);
        } catch (Exception e) {
            failed(e);
            return null;
        }
    }
}
