package org.ferroboard.consumer;

import android.app.Activity;
import android.app.Instrumentation;
import android.accessibilityservice.AccessibilityServiceInfo;
import android.content.Intent;
import android.graphics.Rect;
import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import android.view.accessibility.AccessibilityWindowInfo;
import java.util.ArrayList;
import java.util.List;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.TimeoutException;

/** OS touch input and live accessibility checks; never disclose the synthetic secret in results. */
@SuppressWarnings("deprecation") // minimum API26 uses recyclable nodes/events.
public final class PrivacyChecks extends Instrumentation {
    private final ArrayList<String> checks = new ArrayList<>();
    private final List<String> payloads = new ArrayList<>();
    private final ArrayList<String> previewTitles = new ArrayList<>();
    private int previewSamples;
    private boolean contactDown;
    private Rect contact;
    private long downTime;

    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            AccessibilityServiceInfo service = getUiAutomation().getServiceInfo();
            service.flags |= AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS;
            getUiAutomation().setServiceInfo(service);
            getUiAutomation().setOnAccessibilityEventListener(event -> {
                synchronized (payloads) {
                    payloads.add(event.getText().toString());
                    payloads.add(String.valueOf(event.getBeforeText()));
                    payloads.add(String.valueOf(event.getContentDescription()));
                }
            });
            Intent intent = new Intent(Intent.ACTION_MAIN);
            intent.setClassName("org.ferroboard.privacyconsumer", "org.ferroui.android.FerroMainActivity");
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            startActivitySync(intent);
            status("READY");
            privateNode("PasswordInput");
            action("Emoji");
            status("PASS private emoji");
            privateNode("PasswordInput");
            key("Backspace");
            for (String label : new String[]{"z", "y", "p", "Space", "Backspace", "m", "n"}) key(label);
            action("Verify");
            status("PASS password");
            privateNode("PasswordInput");
            action("Guard");
            status("PASS locked");
            key("Backspace");
            action("Verify");
            status("PASS password");
            action("Guard");
            status("PASS unlocked");
            key("Backspace");
            key("m");
            action("Verify");
            status("PASS password");
            action("Reveal");
            status("PASS reveal");
            privateNode("PasswordInput");
            noDisclosures(true);

            // Native key action Next must replace the password target with the PIN target.
            key("Enter");
            for (String label : new String[]{"7", "3", "9", "4"}) key(label);
            action("Verify");
            status("PASS pin");
            privateNode("PinInput");
            noDisclosures(true);

            tap(nodeBounds("PublicInput"));
            for (String label : new String[]{"q", "z", "x", "Space"}) key(label);
            action("Verify");
            status("PASS public");
            action("Emoji");
            status("PASS public emoji");
            noDisclosures(false);
            action("Sync");
            status("PASS synchronized");
            privateNode("PinInput");
            key("m");
            AccessibilityNodeInfo publicInput = waitNode("PublicInput", null, true);
            try {
                if (!publicInput.refresh() || !"m".contentEquals(publicInput.getText()))
                    throw new AssertionError("Native typing after target synchronization failed");
                checks.add("Native typing after target synchronization");
            } finally { publicInput.recycle(); }
            noDisclosures(false);
            result.putString("status", "passed");
            result.putInt("passed", checks.size());
            finishChecked(Activity.RESULT_OK, result);
        } catch (RuntimeException | AssertionError | TimeoutException failure) {
            result.putString("status", "failed");
            result.putString("failure", android.util.Log.getStackTraceString(failure));
            AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
            if (root != null) {
                try {
                    result.putString("active_package", String.valueOf(root.getPackageName()));
                    ArrayList<String> labels = new ArrayList<>();
                    collectKeyLabels(root, labels);
                    result.putStringArrayList("native_key_labels", labels);
                } finally { root.recycle(); }
            }
            finishChecked(Activity.RESULT_CANCELED, result);
        }
    }
    private void collectKeyLabels(AccessibilityNodeInfo node, ArrayList<String> labels) {
        if ("KeyView".contentEquals(node.getClassName())) labels.add(String.valueOf(node.getText()));
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo child = node.getChild(i); if (child == null) continue;
            try { collectKeyLabels(child, labels); } finally { child.recycle(); }
        }
    }
    private void finishChecked(int code, Bundle result) {
        if (contactDown) {
            try { event(MotionEvent.ACTION_CANCEL, contact); }
            catch (RuntimeException | AssertionError failure) {
                result.putString("status", "failed");
                result.putString("cleanup_failure", android.util.Log.getStackTraceString(failure));
                code = Activity.RESULT_CANCELED;
            }
        }
        getUiAutomation().setOnAccessibilityEventListener(null);
        result.putStringArrayList("checks", checks);
        result.putStringArrayList("clipboard_preview_titles", previewTitles);
        result.putInt("clipboard_preview_samples", previewSamples);
        finish(code, result);
    }
    private void key(String label) { tap(bounds(label, "KeyView")); }
    private void action(String label) { tap(bounds(label, "TouchButton")); }
    private void tap(Rect rect) { event(MotionEvent.ACTION_DOWN, rect); event(MotionEvent.ACTION_UP, rect); }
    private void event(int action, Rect rect) {
        long now = SystemClock.uptimeMillis();
        if (action == MotionEvent.ACTION_DOWN) downTime = now;
        MotionEvent.PointerProperties pointer = new MotionEvent.PointerProperties();
        pointer.id = 3; pointer.toolType = MotionEvent.TOOL_TYPE_FINGER;
        MotionEvent.PointerCoords point = new MotionEvent.PointerCoords();
        point.x = rect.exactCenterX(); point.y = rect.exactCenterY(); point.pressure = 1; point.size = 1;
        MotionEvent event = MotionEvent.obtain(downTime, now, action, 1,
                new MotionEvent.PointerProperties[]{pointer}, new MotionEvent.PointerCoords[]{point},
                0, 0, 1, 1, 0, 0, InputDevice.SOURCE_TOUCHSCREEN, 0);
        try {
            if (!getUiAutomation().injectInputEvent(event, true)) throw new AssertionError("Native touch rejected");
            contactDown = action == MotionEvent.ACTION_DOWN;
            contact = rect;
        } finally { event.recycle(); }
    }
    private AccessibilityNodeInfo find(AccessibilityNodeInfo node, String value, String type, boolean id) {
        if (!node.refresh()) return null;
        String actualId = node.getViewIdResourceName();
        boolean matches = id ? value.equals(actualId) || actualId != null && actualId.endsWith("/" + value)
                : value.contentEquals(node.getText() == null ? "" : node.getText())
                || value.contentEquals(node.getContentDescription() == null ? "" : node.getContentDescription());
        if (matches && (type == null || type.contentEquals(node.getClassName()))) return AccessibilityNodeInfo.obtain(node);
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo child = node.getChild(i);
            if (child == null) continue;
            try { AccessibilityNodeInfo found = find(child, value, type, id); if (found != null) return found; }
            finally { child.recycle(); }
        }
        return null;
    }
    private AccessibilityNodeInfo waitNode(String value, String type, boolean id) {
        long deadline = SystemClock.uptimeMillis() + 30000;
        do {
            AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
            if (root != null) {
                try { AccessibilityNodeInfo found = find(root, value, type, id); if (found != null) return found; }
                finally { root.recycle(); }
            }
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Missing native control " + value);
    }
    private Rect nodeBounds(String id) {
        AccessibilityNodeInfo node = waitNode(id, null, true);
        try { return visibleBounds(node); } finally { node.recycle(); }
    }
    private Rect bounds(String label, String type) {
        AccessibilityNodeInfo node = waitNode(label, type, false);
        try { return visibleBounds(node); } finally { node.recycle(); }
    }
    private Rect visibleBounds(AccessibilityNodeInfo node) {
        Rect bounds = new Rect(); node.getBoundsInScreen(bounds);
        if (!node.isVisibleToUser() || bounds.isEmpty()) throw new AssertionError("Native control not visible");
        return bounds;
    }
    private void status(String expected) {
        long deadline = SystemClock.uptimeMillis() + 30000;
        do {
            AccessibilityNodeInfo node = waitNode("PrivacyStatus", null, true);
            try {
                if (!node.refresh()) throw new AssertionError("Native status became unavailable");
                String actual = node.getText() == null ? "" : node.getText().toString();
                if (actual.startsWith("FAIL")) throw new AssertionError(actual);
                if (expected.equals(actual)) {
                    checks.add(expected);
                    if (expected.equals("PASS password") || expected.equals("PASS pin")
                            || expected.equals("PASS public")) waitClipboardPreview();
                    return;
                }
            } finally { node.recycle(); }
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Native privacy status did not reach " + expected);
    }
    private void waitClipboardPreview() {
        // OS clipboard writes produce a touchable System UI preview over lower keys.
        // Wait for that real window to disappear, rather than tapping through it or
        // modifying keyboard delays/System UI settings. No contact is held here.
        long deadline = SystemClock.uptimeMillis() + 15000;
        long quietSince = SystemClock.uptimeMillis();
        do {
            boolean preview = false;
            for (AccessibilityWindowInfo window : getUiAutomation().getWindows()) {
                try {
                    CharSequence title = window.getTitle();
                    if (title != null && title.toString().contains("ClipboardOverlay")) {
                        preview = true;
                        previewSamples++;
                        if (!previewTitles.contains(title.toString())) previewTitles.add(title.toString());
                    }
                } finally { window.recycle(); }
            }
            // This System UI overlay is not consistently included in the accessibility
            // window list. Query its actual WindowManager entry with shell read access.
            try (BufferedReader reader = new BufferedReader(new InputStreamReader(
                    new ParcelFileDescriptor.AutoCloseInputStream(
                            getUiAutomation().executeShellCommand("dumpsys window windows")),
                    StandardCharsets.UTF_8))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    if (line.contains("Window #") && line.contains("ClipboardOverlay")) {
                        preview = true;
                        previewSamples++;
                        if (!previewTitles.contains("WindowManager ClipboardOverlay"))
                            previewTitles.add("WindowManager ClipboardOverlay");
                    }
                }
            } catch (IOException failure) {
                throw new AssertionError("Cannot inspect clipboard preview window", failure);
            }
            long now = SystemClock.uptimeMillis();
            if (preview) quietSince = now;
            else if (now - quietSince >= 500) return;
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Android clipboard preview did not dismiss");
    }
    private void privateNode(String id) {
        AccessibilityNodeInfo node = waitNode(id, null, true);
        try {
            if (!node.refresh()) throw new AssertionError("Native password node became unavailable");
            if (node.getChildCount() != 0) throw new AssertionError("Password exposes native text children");
            String label = "PasswordInput".equals(id) ? "Password input" : "PIN input";
            for (CharSequence value : new CharSequence[]{node.getText(), node.getContentDescription()})
                if (value != null && value.length() != 0 && !label.contentEquals(value))
                    throw new AssertionError("Password exposes a native text value");
            checks.add(id + " redacted, no text children");
        } finally { node.recycle(); }
    }
    private void inspectText(CharSequence value) {
        if (value == null) return;
        String text = value.toString();
        for (String secret : new String[]{"zyp", "739", "867"})
            if (text.contains(secret)) throw new AssertionError("Synthetic private value disclosed through accessibility");
    }
    private void inspectTree(AccessibilityNodeInfo node) {
        if (!node.refresh()) throw new AssertionError("Native accessibility snapshot became unavailable");
        inspectText(node.getText()); inspectText(node.getContentDescription());
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo child = node.getChild(i); if (child == null) continue;
            try { inspectTree(child); } finally { child.recycle(); }
        }
    }
    private void noDisclosures(boolean privatePhase) throws TimeoutException {
        getUiAutomation().waitForIdle(80, 3000);
        AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
        if (root == null) throw new AssertionError("Missing native accessibility root");
        try { inspectTree(root); } finally { root.recycle(); }
        synchronized (payloads) {
            for (String value : payloads) {
                inspectText(value);
                if (privatePhase && value.contains("🙂")) throw new AssertionError("Private emoji disclosed in native event");
            }
            payloads.clear();
        }
        checks.add("Native tree/event payloads contain no private values");
    }
}
