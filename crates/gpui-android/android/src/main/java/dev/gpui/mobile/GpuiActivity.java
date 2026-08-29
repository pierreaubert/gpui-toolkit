package dev.gpui.mobile;

import android.app.NativeActivity;
import android.content.ActivityNotFoundException;
import android.content.Context;
import android.content.Intent;
import android.content.ComponentName;
import android.content.pm.ActivityInfo;
import android.content.pm.PackageManager;
import android.graphics.Rect;
import android.net.Uri;
import android.os.Bundle;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.text.Editable;
import android.text.Selection;
import android.text.SpannableStringBuilder;
import android.util.Base64;
import android.view.Gravity;
import android.view.View;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import android.view.accessibility.AccessibilityNodeProvider;
import android.widget.FrameLayout;

import java.io.File;
import java.io.IOException;
import java.security.KeyStore;
import java.util.HashMap;
import java.util.Iterator;
import java.util.Map;
import java.util.concurrent.atomic.AtomicBoolean;

import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

/**
 * NativeActivity host used by GPUI Android applications.
 *
 * <p>The small editor view supplies the InputConnection that NativeActivity
 * itself lacks, enabling full commit/composition callbacks for software IMEs.
 * Credential helpers keep encrypted payloads in app-private preferences and
 * the encryption key in AndroidKeyStore.</p>
 */
public class GpuiActivity extends NativeActivity {
    private static final String KEYSTORE = "AndroidKeyStore";
    private static final String CREDENTIAL_PREFS = "gpui_secure_credentials";
    private GpuiInputView inputView;
    private final AtomicBoolean accessibilityChangeScheduled = new AtomicBoolean();

    private static native boolean nativeIsInitialized();
    private static native void nativeOnDeepLink(String url);
    private static native void nativeCommitText(String text);
    private static native void nativeSetComposingText(String text);
    private static native void nativeFinishComposingText();
    private static native void nativeDeleteSurroundingText(int before, int after);
    private static native String nativeAccessibilitySnapshot();
    private static native boolean nativeAccessibilityAction(long nodeId, int action);

    @Override
    protected void onCreate(Bundle state) {
        ensureNativeLibraryVisibleToJvm();
        super.onCreate(state);
        inputView = new GpuiInputView(this);
        inputView.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_YES);
        FrameLayout.LayoutParams params = new FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT);
        params.gravity = Gravity.BOTTOM | Gravity.END;
        addContentView(inputView, params);
        dispatchDeepLink(getIntent());
    }

    /**
     * Associate NativeActivity's library with this Java class loader.
     *
     * <p>NativeActivity loads {@code android.app.lib_name} through its native
     * glue. That is enough for {@code ANativeActivity_onCreate}, but Android's
     * Java JNI resolver does not associate that dlopen handle with this class
     * loader. Explicitly loading the same library is idempotent and makes the
     * input, accessibility, deep-link, and credential JNI entry points
     * resolvable before Android invokes them.</p>
     */
    private void ensureNativeLibraryVisibleToJvm() {
        try {
            ActivityInfo info = getPackageManager().getActivityInfo(
                    new ComponentName(this, getClass()), PackageManager.GET_META_DATA);
            String library = info.metaData == null
                    ? null
                    : info.metaData.getString("android.app.lib_name");
            if (library == null || library.isEmpty()) {
                throw new IllegalStateException(
                        "android.app.lib_name is required for GPUI NativeActivity hosts");
            }
            System.loadLibrary(library);
        } catch (PackageManager.NameNotFoundException error) {
            throw new IllegalStateException("Unable to resolve the GPUI activity metadata", error);
        }
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        dispatchDeepLink(intent);
    }

    private void dispatchDeepLink(Intent intent) {
        if (intent != null && intent.getDataString() != null) {
            nativeOnDeepLink(intent.getDataString());
        }
    }

    /** Called by Rust; safely moves focus and IME operations to the UI thread. */
    public void gpuiShowKeyboard(int inputType) {
        runOnUiThread(() -> {
            inputView.setInputType(inputType);
            inputView.requestFocus();
            InputMethodManager manager =
                    (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
            if (manager != null) {
                manager.restartInput(inputView);
                manager.showSoftInput(inputView, InputMethodManager.SHOW_IMPLICIT);
            }
        });
    }

    /** Called by Rust; safely hides the software keyboard on the UI thread. */
    public void gpuiHideKeyboard() {
        runOnUiThread(() -> {
            InputMethodManager manager =
                    (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
            if (manager != null) {
                manager.hideSoftInputFromWindow(inputView.getWindowToken(), 0);
            }
            inputView.clearFocus();
        });
    }

    /** Clears IME-owned context when focus moves to a different GPUI input. */
    public void gpuiResetInputShadow() {
        runOnUiThread(() -> {
            if (inputView != null) {
                inputView.resetEditable();
            }
        });
    }

    /** Called by Rust after an AccessKit tree update. */
    public void gpuiAccessibilityChanged() {
        if (!accessibilityChangeScheduled.compareAndSet(false, true)) {
            return;
        }
        runOnUiThread(() -> {
            if (inputView == null) {
                accessibilityChangeScheduled.set(false);
                return;
            }
            inputView.postOnAnimation(() -> {
                accessibilityChangeScheduled.set(false);
                inputView.accessibilityProvider.invalidateSnapshot();
                inputView.sendAccessibilityEvent(
                        AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED);
            });
        });
    }

    /**
     * Ask Android to open a local file with the user's system handler.
     *
     * <p>The GPUI platform thread is not guaranteed to be the Android UI
     * thread, so the entire operation is marshalled through
     * {@link #runOnUiThread(Runnable)}. The provider gives the receiving app a
     * read-only, temporary content URI; a {@code file://} URI is rejected by
     * Android 7 and later.</p>
     */
    public void gpuiOpenWithSystem(String path) {
        if (path == null || path.isEmpty()) {
            return;
        }

        runOnUiThread(() -> {
            File file = new File(path);
            if (!file.isFile()) {
                return;
            }

            try {
                Uri uri = GpuiFileProvider.registerFile(this, file);
                String mimeType = getContentResolver().getType(uri);
                if (mimeType == null) {
                    mimeType = "application/octet-stream";
                }

                Intent intent = new Intent(Intent.ACTION_VIEW)
                        .setDataAndType(uri, mimeType)
                        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                startActivity(intent);
            } catch (ActivityNotFoundException | IOException | SecurityException e) {
                // No compatible handler is a normal user/device condition.
                // Do not include the local path in logs.
                android.util.Log.w("GpuiActivity", "Unable to open file with a system handler", e);
            }
        });
    }

    /** Encrypt and persist a credential using a non-exportable AES-GCM key. */
    public void gpuiWriteCredential(String alias, byte[] secret) throws Exception {
        SecretKey key = getOrCreateKey(alias);
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.ENCRYPT_MODE, key);
        String encoded = Base64.encodeToString(cipher.getIV(), Base64.NO_WRAP)
                + ":"
                + Base64.encodeToString(cipher.doFinal(secret), Base64.NO_WRAP);
        getSharedPreferences(CREDENTIAL_PREFS, MODE_PRIVATE)
                .edit()
                .putString(alias, encoded)
                .apply();
    }

    /** Decrypt a credential, or return null when no value exists. */
    public byte[] gpuiReadCredential(String alias) throws Exception {
        String encoded = getSharedPreferences(CREDENTIAL_PREFS, MODE_PRIVATE)
                .getString(alias, null);
        if (encoded == null) {
            return null;
        }
        String[] parts = encoded.split(":", 2);
        if (parts.length != 2) {
            throw new IllegalStateException("Malformed GPUI credential");
        }
        KeyStore store = KeyStore.getInstance(KEYSTORE);
        store.load(null);
        SecretKey key = (SecretKey) store.getKey(alias, null);
        if (key == null) {
            return null;
        }
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(
                Cipher.DECRYPT_MODE,
                key,
                new GCMParameterSpec(128, Base64.decode(parts[0], Base64.NO_WRAP)));
        return cipher.doFinal(Base64.decode(parts[1], Base64.NO_WRAP));
    }

    /** Delete both the encrypted payload and its AndroidKeyStore key. */
    public void gpuiDeleteCredential(String alias) throws Exception {
        getSharedPreferences(CREDENTIAL_PREFS, MODE_PRIVATE).edit().remove(alias).apply();
        KeyStore store = KeyStore.getInstance(KEYSTORE);
        store.load(null);
        if (store.containsAlias(alias)) {
            store.deleteEntry(alias);
        }
    }

    private SecretKey getOrCreateKey(String alias) throws Exception {
        KeyStore store = KeyStore.getInstance(KEYSTORE);
        store.load(null);
        SecretKey existing = (SecretKey) store.getKey(alias, null);
        if (existing != null) {
            return existing;
        }
        KeyGenerator generator =
                KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE);
        generator.init(new KeyGenParameterSpec.Builder(
                        alias,
                        KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true)
                .build());
        return generator.generateKey();
    }

    private static final class GpuiInputView extends View {
        private final Editable editable = new SpannableStringBuilder();
        private final GpuiAccessibilityProvider accessibilityProvider;
        private int inputType = EditorInfo.TYPE_CLASS_TEXT;

        GpuiInputView(Context context) {
            super(context);
            accessibilityProvider = new GpuiAccessibilityProvider(this);
            setFocusable(true);
            setFocusableInTouchMode(true);
        }

        void setInputType(int value) {
            inputType = value;
        }

        @Override
        public boolean onCheckIsTextEditor() {
            return true;
        }

        @Override
        public AccessibilityNodeProvider getAccessibilityNodeProvider() {
            return accessibilityProvider;
        }

        void resetEditable() {
            editable.clear();
            Selection.setSelection(editable, 0);
        }

        private void trimEditable() {
            int excess = InputShadowBufferPolicy.trimPrefixLength(editable.length());
            if (excess > 0) {
                editable.delete(0, excess);
            }
        }

        @Override
        public InputConnection onCreateInputConnection(EditorInfo attributes) {
            attributes.inputType = inputType;
            attributes.imeOptions = EditorInfo.IME_ACTION_NONE
                    | EditorInfo.IME_FLAG_NO_EXTRACT_UI;
            attributes.initialSelStart = editable.length();
            attributes.initialSelEnd = editable.length();
            return new BaseInputConnection(this, true) {
                @Override
                public Editable getEditable() {
                    return editable;
                }

                @Override
                public boolean commitText(CharSequence text, int newCursorPosition) {
                    nativeCommitText(text == null ? "" : text.toString());
                    boolean committed = super.commitText(text, newCursorPosition);
                    trimEditable();
                    return committed;
                }

                @Override
                public boolean setComposingText(CharSequence text, int newCursorPosition) {
                    nativeSetComposingText(text == null ? "" : text.toString());
                    boolean composing = super.setComposingText(text, newCursorPosition);
                    trimEditable();
                    return composing;
                }

                @Override
                public boolean finishComposingText() {
                    nativeFinishComposingText();
                    return super.finishComposingText();
                }

                @Override
                public boolean deleteSurroundingText(int beforeLength, int afterLength) {
                    nativeDeleteSurroundingText(beforeLength, afterLength);
                    boolean deleted = super.deleteSurroundingText(beforeLength, afterLength);
                    trimEditable();
                    return deleted;
                }
            };
        }
    }

    private static final class GpuiAccessibilityProvider extends AccessibilityNodeProvider {
        private static final int HOST_ID = View.NO_ID;
        private static final int ACTION_CLICK = 1;
        private static final int ACTION_FOCUS = 2;
        private static final int ACTION_INCREMENT = 3;
        private static final int ACTION_DECREMENT = 4;

        private final View host;
        private final Map<Long, Integer> virtualIds = new HashMap<>();
        private final Map<Integer, Long> nodeIds = new HashMap<>();
        private final Map<Long, JSONObject> nodesById = new HashMap<>();
        private final Map<Long, Long> parentIds = new HashMap<>();
        private JSONObject cachedSnapshot;
        private int nextVirtualId = 1;
        private int accessibilityFocusedId = HOST_ID;

        GpuiAccessibilityProvider(View host) {
            this.host = host;
        }

        private JSONObject snapshot() throws JSONException {
            if (cachedSnapshot != null) {
                return cachedSnapshot;
            }
            String json = nativeAccessibilitySnapshot();
            JSONObject snapshot = new JSONObject(json == null || json.isEmpty() ? "{}" : json);
            rebuildNodeIndex(snapshot);
            cachedSnapshot = snapshot;
            return snapshot;
        }

        void invalidateSnapshot() {
            cachedSnapshot = null;
            nodesById.clear();
            parentIds.clear();
        }

        private void rebuildNodeIndex(JSONObject snapshot) throws JSONException {
            nodesById.clear();
            parentIds.clear();
            JSONArray nodes = snapshot.optJSONArray("nodes");
            if (nodes != null) {
                for (int index = 0; index < nodes.length(); index++) {
                    JSONObject node = nodes.getJSONObject(index);
                    if (node.has("id")) {
                        nodesById.put(node.getLong("id"), node);
                    }
                }
                for (Map.Entry<Long, JSONObject> entry : nodesById.entrySet()) {
                    JSONArray children = entry.getValue().optJSONArray("children");
                    if (children == null) {
                        continue;
                    }
                    for (int index = 0; index < children.length(); index++) {
                        parentIds.put(children.getLong(index), entry.getKey());
                    }
                }
            }

            Iterator<Map.Entry<Long, Integer>> ids = virtualIds.entrySet().iterator();
            while (ids.hasNext()) {
                Map.Entry<Long, Integer> entry = ids.next();
                if (!nodesById.containsKey(entry.getKey())) {
                    nodeIds.remove(entry.getValue());
                    ids.remove();
                }
            }
        }

        private int virtualId(long nodeId) {
            Integer existing = virtualIds.get(nodeId);
            if (existing != null) {
                return existing;
            }
            int id = nextVirtualId++;
            virtualIds.put(nodeId, id);
            nodeIds.put(id, nodeId);
            return id;
        }

        private JSONObject findNode(long nodeId) {
            return nodesById.get(nodeId);
        }

        private Long findParent(long nodeId) {
            return parentIds.get(nodeId);
        }

        @Override
        public AccessibilityNodeInfo createAccessibilityNodeInfo(int virtualViewId) {
            try {
                JSONObject snapshot = snapshot();
                if (virtualViewId == HOST_ID) {
                    AccessibilityNodeInfo info = AccessibilityNodeInfo.obtain(host);
                    host.onInitializeAccessibilityNodeInfo(info);
                    info.setPackageName(host.getContext().getPackageName());
                    info.setClassName(GpuiActivity.class.getName());
                    info.setSource(host);
                    info.setVisibleToUser(true);
                    if (!snapshot.isNull("root")) {
                    JSONObject root = findNode(snapshot.getLong("root"));
                        JSONArray children = root == null ? null : root.optJSONArray("children");
                        if (children != null) {
                            for (int index = 0; index < children.length(); index++) {
                                info.addChild(host, virtualId(children.getLong(index)));
                            }
                        }
                    }
                    return info;
                }

                Long nodeId = nodeIds.get(virtualViewId);
                if (nodeId == null) {
                    return null;
                }
                JSONObject node = findNode(nodeId);
                if (node == null) {
                    return null;
                }

                AccessibilityNodeInfo info = AccessibilityNodeInfo.obtain();
                info.setPackageName(host.getContext().getPackageName());
                info.setSource(host, virtualViewId);
                Long parent = findParent(nodeId);
                long rootId = snapshot.optLong("root", -1);
                if (parent == null || parent == rootId) {
                    info.setParent(host);
                } else {
                    info.setParent(host, virtualId(parent));
                }

                String role = node.optString("role", "Unknown");
                info.setClassName(classNameForRole(role));
                String label = node.isNull("label") ? "" : node.optString("label", "");
                String value = node.isNull("value") ? "" : node.optString("value", "");
                String description = node.isNull("description")
                        ? ""
                        : node.optString("description", "");
                info.setContentDescription(label.isEmpty() ? description : label);
                if (!value.isEmpty()) {
                    info.setText(value);
                }
                info.setEnabled(!node.optBoolean("disabled", false));
                info.setVisibleToUser(true);

                JSONArray bounds = node.optJSONArray("bounds");
                if (bounds != null && bounds.length() == 4) {
                    Rect parentBounds = new Rect(
                            (int) Math.floor(bounds.getDouble(0)),
                            (int) Math.floor(bounds.getDouble(1)),
                            (int) Math.ceil(bounds.getDouble(2)),
                            (int) Math.ceil(bounds.getDouble(3)));
                    info.setBoundsInParent(parentBounds);
                    int[] location = new int[2];
                    host.getLocationOnScreen(location);
                    parentBounds.offset(location[0], location[1]);
                    info.setBoundsInScreen(parentBounds);
                }

                JSONArray children = node.optJSONArray("children");
                if (children != null) {
                    for (int index = 0; index < children.length(); index++) {
                        info.addChild(host, virtualId(children.getLong(index)));
                    }
                }

                boolean clickable = node.optBoolean("click", false);
                boolean focusable = node.optBoolean("focus", false) || clickable;
                info.setClickable(clickable);
                info.setFocusable(focusable);
                if (clickable) {
                    info.addAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_CLICK);
                }
                if (focusable) {
                    info.addAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_FOCUS);
                }
                info.addAction(
                        virtualViewId == accessibilityFocusedId
                                ? AccessibilityNodeInfo.AccessibilityAction.ACTION_CLEAR_ACCESSIBILITY_FOCUS
                                : AccessibilityNodeInfo.AccessibilityAction.ACTION_ACCESSIBILITY_FOCUS);
                if (node.optBoolean("increment", false)) {
                    info.addAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_SCROLL_FORWARD);
                }
                if (node.optBoolean("decrement", false)) {
                    info.addAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_SCROLL_BACKWARD);
                }
                info.setAccessibilityFocused(virtualViewId == accessibilityFocusedId);
                return info;
            } catch (JSONException error) {
                return null;
            }
        }

        @Override
        public boolean performAction(int virtualViewId, int action, Bundle arguments) {
            Long nodeId = nodeIds.get(virtualViewId);
            if (nodeId == null) {
                return false;
            }
            if (action == AccessibilityNodeInfo.ACTION_ACCESSIBILITY_FOCUS) {
                accessibilityFocusedId = virtualViewId;
                sendVirtualEvent(
                        virtualViewId,
                        AccessibilityEvent.TYPE_VIEW_ACCESSIBILITY_FOCUSED);
                return nativeAccessibilityAction(nodeId, ACTION_FOCUS);
            }
            if (action == AccessibilityNodeInfo.ACTION_CLEAR_ACCESSIBILITY_FOCUS) {
                accessibilityFocusedId = HOST_ID;
                sendVirtualEvent(
                        virtualViewId,
                        AccessibilityEvent.TYPE_VIEW_ACCESSIBILITY_FOCUS_CLEARED);
                return true;
            }
            if (action == AccessibilityNodeInfo.ACTION_CLICK) {
                return nativeAccessibilityAction(nodeId, ACTION_CLICK);
            }
            if (action == AccessibilityNodeInfo.ACTION_FOCUS) {
                return nativeAccessibilityAction(nodeId, ACTION_FOCUS);
            }
            if (action == AccessibilityNodeInfo.ACTION_SCROLL_FORWARD) {
                return nativeAccessibilityAction(nodeId, ACTION_INCREMENT);
            }
            if (action == AccessibilityNodeInfo.ACTION_SCROLL_BACKWARD) {
                return nativeAccessibilityAction(nodeId, ACTION_DECREMENT);
            }
            return false;
        }

        private void sendVirtualEvent(int virtualViewId, int eventType) {
            AccessibilityEvent event = AccessibilityEvent.obtain(eventType);
            event.setPackageName(host.getContext().getPackageName());
            event.setClassName(GpuiActivity.class.getName());
            event.setSource(host, virtualViewId);
            if (host.getParent() != null) {
                host.getParent().requestSendAccessibilityEvent(host, event);
            }
        }

        private static String classNameForRole(String role) {
            if (role.contains("Button")) {
                return android.widget.Button.class.getName();
            }
            if (role.contains("CheckBox")) {
                return android.widget.CheckBox.class.getName();
            }
            if (role.contains("Switch")) {
                return android.widget.Switch.class.getName();
            }
            if (role.contains("TextInput") || role.contains("SearchInput")) {
                return android.widget.EditText.class.getName();
            }
            if (role.contains("Slider")) {
                return android.widget.SeekBar.class.getName();
            }
            if (role.contains("Image")) {
                return android.widget.ImageView.class.getName();
            }
            return android.widget.TextView.class.getName();
        }
    }
}
