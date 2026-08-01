package dev.gpui.mobile;

import android.content.ContentProvider;
import android.content.ContentResolver;
import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;
import android.webkit.MimeTypeMap;

import java.io.File;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.util.Locale;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Read-only provider for files explicitly handed to an external system app.
 *
 * <p>Files are addressed by random opaque tokens rather than filesystem paths.
 * The provider is non-exported in the application manifest, and each intent
 * carries only temporary read permission. This is intentionally small and
 * dependency-free so the GPUI Android host does not require AndroidX.</p>
 */
public final class GpuiFileProvider extends ContentProvider {
    public static final String AUTHORITY_SUFFIX = ".gpui.fileprovider";

    private static final ConcurrentHashMap<String, File> FILES =
            new ConcurrentHashMap<>();

    static Uri registerFile(Context context, File file) throws IOException {
        File canonical = file.getCanonicalFile();
        if (!canonical.isFile() || !canonical.canRead()) {
            throw new FileNotFoundException("File is not readable");
        }

        String token = UUID.randomUUID().toString();
        FILES.put(token, canonical);
        return new Uri.Builder()
                .scheme(ContentResolver.SCHEME_CONTENT)
                .authority(context.getPackageName() + AUTHORITY_SUFFIX)
                .appendPath(token)
                .build();
    }

    @Override
    public boolean onCreate() {
        return true;
    }

    @Override
    public String getType(Uri uri) {
        File file = resolve(uri);
        if (file == null) {
            return null;
        }

        String name = file.getName();
        int dot = name.lastIndexOf('.');
        if (dot >= 0 && dot + 1 < name.length()) {
            String extension = name.substring(dot + 1).toLowerCase(Locale.ROOT);
            String type = MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension);
            if (type != null) {
                return type;
            }
        }
        return "application/octet-stream";
    }

    @Override
    public Cursor query(
            Uri uri,
            String[] projection,
            String selection,
            String[] selectionArgs,
            String sortOrder) {
        File file = resolve(uri);
        if (file == null) {
            return null;
        }

        String[] columns = projection == null
                ? new String[] {OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE}
                : projection;
        MatrixCursor cursor = new MatrixCursor(columns, 1);
        Object[] values = new Object[columns.length];
        for (int index = 0; index < columns.length; index++) {
            if (OpenableColumns.DISPLAY_NAME.equals(columns[index])) {
                values[index] = file.getName();
            } else if (OpenableColumns.SIZE.equals(columns[index])) {
                values[index] = file.length();
            }
        }
        cursor.addRow(values);
        return cursor;
    }

    @Override
    public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        if (!"r".equals(mode)) {
            throw new FileNotFoundException("GPUI file provider is read-only");
        }
        File file = resolve(uri);
        if (file == null) {
            throw new FileNotFoundException("Unknown GPUI file token");
        }
        return ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY);
    }

    @Override
    public int delete(Uri uri, String selection, String[] selectionArgs) {
        String token = token(uri);
        return token == null || FILES.remove(token) == null ? 0 : 1;
    }

    @Override
    public Uri insert(Uri uri, ContentValues values) {
        throw new UnsupportedOperationException("GPUI file provider is read-only");
    }

    @Override
    public int update(Uri uri, ContentValues values, String selection, String[] selectionArgs) {
        throw new UnsupportedOperationException("GPUI file provider is read-only");
    }

    private static String token(Uri uri) {
        if (uri == null || uri.getPathSegments().size() != 1) {
            return null;
        }
        return uri.getPathSegments().get(0);
    }

    private static File resolve(Uri uri) {
        String token = token(uri);
        return token == null ? null : FILES.get(token);
    }
}
