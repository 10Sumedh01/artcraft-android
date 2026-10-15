package ai.storyteller.photocraft;

import android.app.NativeActivity;
import android.content.ClipData;
import android.content.ContentResolver;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.provider.OpenableColumns;
import android.util.Log;
import android.view.View;
import android.view.Window;
import android.view.WindowInsets;
import android.view.WindowInsetsController;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** NativeActivity subclass that supplies Android SAF open/save results to the Rust UI shell. */
public final class PhotoCraftActivity extends NativeActivity {
    private static final String TAG = "PhotoCraftSAF";
    private static final int REQ_OPEN = 0x5043;
    private static final int REQ_SAVE = 0x5044;
    private static final String PREFS = "photocraft_saf_uri_map";
    private final ConcurrentLinkedQueue<String> results = new ConcurrentLinkedQueue<>();
    private final Map<Integer, Long> requestIds = new HashMap<>();
    private final Map<Long, String> suggestedNames = new HashMap<>();
    private final ExecutorService io = Executors.newSingleThreadExecutor();

    public String pcFilesDir() { return getFilesDir().getAbsolutePath(); }

    private void applyImmersiveFullscreen() {
        Window window = getWindow();
        if (window == null) {
            return;
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            WindowInsetsController controller = window.getInsetsController();
            if (controller != null) {
                controller.hide(WindowInsets.Type.systemBars());
                controller.setSystemBarsBehavior(
                        WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
            }
        } else {
            window.getDecorView().setSystemUiVisibility(
                    View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                            | View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                            | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_FULLSCREEN);
        }
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        applyImmersiveFullscreen();
        Window window = getWindow();
        if (window != null) {
            window.getDecorView().post(this::applyImmersiveFullscreen);
        }
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus) {
            applyImmersiveFullscreen();
        }
    }

    public String pcPollPickerResult() { return results.poll(); }

    public void pcRequestOpen(long id, boolean multiple) {
        runOnUiThread(() -> {
            try {
                Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                intent.addCategory(Intent.CATEGORY_OPENABLE);
                intent.setType("*/*");
                intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, multiple);
                intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
                requestIds.put(REQ_OPEN, id);
                startActivityForResult(intent, REQ_OPEN);
            } catch (Exception e) {
                enqueueError(id, "open", e.toString());
            }
        });
    }

    public void pcRequestSave(long id, String suggestedName) {
        runOnUiThread(() -> {
            try {
                String name = safeName(suggestedName == null ? "Untitled.pcraft" : suggestedName);
                Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
                intent.addCategory(Intent.CATEGORY_OPENABLE);
                intent.setType(mimeFor(name));
                intent.putExtra(Intent.EXTRA_TITLE, name);
                intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
                requestIds.put(REQ_SAVE, id);
                suggestedNames.put(id, name);
                startActivityForResult(intent, REQ_SAVE);
            } catch (Exception e) {
                enqueueError(id, "save", e.toString());
            }
        });
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != REQ_OPEN && requestCode != REQ_SAVE) return;
        Long boxedId = requestIds.remove(requestCode);
        if (boxedId == null) return;
        final long id = boxedId;
        final String kind = requestCode == REQ_OPEN ? "open" : "save";
        if (resultCode != RESULT_OK || data == null) {
            suggestedNames.remove(id);
            enqueueCancel(id, kind);
            return;
        }
        if (requestCode == REQ_SAVE) {
            Uri uri = data.getData();
            if (uri == null) { enqueueCancel(id, kind); return; }
            persistPermission(uri, data);
            try {
                String name = displayName(uri);
                if (name == null || name.trim().isEmpty()) name = suggestedNames.getOrDefault(id, "Untitled.pcraft");
                File stage = newStageFile(name);
                rememberUri(stage, uri);
                enqueueSave(id, stage.getAbsolutePath());
            } catch (Exception e) {
                enqueueError(id, kind, e.toString());
            } finally {
                suggestedNames.remove(id);
            }
            return;
        }

        List<Uri> uris = new ArrayList<>();
        ClipData clip = data.getClipData();
        if (clip != null) {
            for (int i = 0; i < clip.getItemCount(); i++) {
                Uri uri = clip.getItemAt(i).getUri();
                if (uri != null) uris.add(uri);
            }
        } else if (data.getData() != null) {
            uris.add(data.getData());
        }
        if (uris.isEmpty()) { enqueueCancel(id, kind); return; }
        for (Uri uri : uris) persistPermission(uri, data);
        io.execute(() -> {
            try {
                JSONArray paths = new JSONArray();
                for (Uri uri : uris) {
                    File stage = newStageFile(displayName(uri));
                    copyUriToFile(uri, stage);
                    rememberUri(stage, uri);
                    paths.put(stage.getAbsolutePath());
                }
                JSONObject result = base(id, "open", "ok");
                result.put("paths", paths);
                results.offer(result.toString());
            } catch (Exception e) {
                enqueueError(id, "open", e.toString());
            }
        });
    }

    /** Commit an app-private staged file to its remembered SAF URI; used by Rust after every save. */
    public String pcCommitIfMapped(String path) {
        try {
            String uriText = getSharedPreferences(PREFS, MODE_PRIVATE).getString("uri:" + path, null);
            if (uriText == null) return "NONE";
            Uri uri = Uri.parse(uriText);
            OutputStream out = getContentResolver().openOutputStream(uri, "wt");
            if (out == null) out = getContentResolver().openOutputStream(uri, "w");
            if (out == null) return "ERROR: Android document provider did not open an output stream";
            try (InputStream in = new FileInputStream(new File(path)); OutputStream target = out) {
                byte[] buffer = new byte[64 * 1024];
                int count;
                while ((count = in.read(buffer)) != -1) target.write(buffer, 0, count);
                target.flush();
            }
            return "OK";
        } catch (Exception e) {
            Log.e(TAG, "Could not write a selected document", e);
            return "ERROR: Could not write the selected document: " + e.getMessage();
        }
    }

    private File newStageFile(String name) throws Exception {
        File dir = new File(getFilesDir(), "PhotoCraft/saf/" + UUID.randomUUID());
        if (!dir.mkdirs() && !dir.isDirectory()) throw new Exception("Could not create private staging directory");
        return new File(dir, safeName(name == null ? "imported-file" : name));
    }

    private static String safeName(String name) {
        String clean = name.replace('\\', '/');
        int slash = clean.lastIndexOf('/');
        if (slash >= 0) clean = clean.substring(slash + 1);
        clean = clean.replaceAll("[^A-Za-z0-9._() -]", "_").trim();
        if (clean.isEmpty() || clean.equals(".") || clean.equals("..")) return "untitled";
        return clean;
    }

    private String displayName(Uri uri) {
        try (Cursor cursor = getContentResolver().query(uri, null, null, null, null)) {
            if (cursor != null && cursor.moveToFirst()) {
                int index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if (index >= 0) return safeName(cursor.getString(index));
            }
        } catch (Exception ignored) { }
        String segment = uri.getLastPathSegment();
        return segment == null ? "imported-file" : safeName(segment);
    }

    private void persistPermission(Uri uri, Intent data) {
        try {
            int flags = data.getFlags() & (Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
            if (flags != 0) getContentResolver().takePersistableUriPermission(uri, flags);
        } catch (Exception e) {
            Log.i(TAG, "Provider did not offer a persistable grant for " + uri, e);
        }
    }

    private void rememberUri(File path, Uri uri) {
        getSharedPreferences(PREFS, MODE_PRIVATE).edit().putString("uri:" + path.getAbsolutePath(), uri.toString()).apply();
    }

    private void copyUriToFile(Uri uri, File destination) throws Exception {
        InputStream input = getContentResolver().openInputStream(uri);
        if (input == null) throw new Exception("Android document provider did not open the selected file");
        try (InputStream in = input; FileOutputStream out = new FileOutputStream(destination)) {
            byte[] buffer = new byte[64 * 1024];
            int count;
            while ((count = in.read(buffer)) != -1) out.write(buffer, 0, count);
            out.flush();
        }
    }

    private JSONObject base(long id, String kind, String status) throws Exception {
        JSONObject value = new JSONObject();
        value.put("id", id);
        value.put("kind", kind);
        value.put("status", status);
        return value;
    }

    private void enqueueCancel(long id, String kind) {
        try { results.offer(base(id, kind, "cancel").toString()); }
        catch (Exception e) { Log.e(TAG, "Could not encode cancellation", e); }
    }

    private void enqueueError(long id, String kind, String message) {
        try {
            JSONObject value = base(id, kind, "error");
            value.put("message", message == null ? "Unknown Android file picker error" : message);
            results.offer(value.toString());
        } catch (Exception e) { Log.e(TAG, "Could not encode picker error", e); }
    }

    private void enqueueSave(long id, String path) {
        try {
            JSONObject value = base(id, "save", "ok");
            value.put("path", path);
            results.offer(value.toString());
        } catch (Exception e) { enqueueError(id, "save", e.toString()); }
    }

    private static String mimeFor(String fileName) {
        String lower = fileName.toLowerCase(java.util.Locale.ROOT);
        if (lower.endsWith(".png")) return "image/png";
        if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) return "image/jpeg";
        if (lower.endsWith(".webp")) return "image/webp";
        if (lower.endsWith(".gif")) return "image/gif";
        if (lower.endsWith(".tif") || lower.endsWith(".tiff")) return "image/tiff";
        if (lower.endsWith(".bmp")) return "image/bmp";
        if (lower.endsWith(".svg")) return "image/svg+xml";
        return "application/octet-stream";
    }
}
