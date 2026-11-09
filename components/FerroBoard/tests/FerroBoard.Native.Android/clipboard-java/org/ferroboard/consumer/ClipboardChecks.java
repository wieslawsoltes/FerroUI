package org.ferroboard.consumer;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Intent;
import android.graphics.Rect;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import java.util.ArrayList;

/** Real native panel interactions against explicitly seeded deterministic history. */
@SuppressWarnings("deprecation")
public final class ClipboardChecks extends Instrumentation {
    private final ArrayList<String> checks = new ArrayList<>();
    private boolean contactDown;
    private Rect contact;
    private long downTime;
    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            Intent intent = new Intent(Intent.ACTION_MAIN);
            intent.setClassName("org.ferroboard.clipboardconsumer", "org.ferroui.android.FerroMainActivity");
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            startActivitySync(intent);
            status("READY");
            action("Seed"); status("SEEDED");
            action("Clipboard");
            action("Enable clipboard history");
            check("PASS initial");
            nested("Paste Alpha🙂", "Pin");
            check("PASS pinned");
            action("Paste Alpha🙂");
            check("PASS pasted");
            action("Advance"); status("ADVANCED");
            refresh();
            bounds("Paste Betaé", "TouchButton", false);
            check("PASS boundary");
            action("Advance"); status("ADVANCED");
            refresh();
            check("PASS expired");
            absent("Paste Betaé");
            action("Clear clipboard history");
            check("PASS clear pinned");
            nested("Paste Alpha🙂", "Unpin");
            check("PASS unpinned expired");
            absent("Paste Alpha🙂");
            action("Disable clipboard history");
            bounds("Clipboard history is turned off.", null, false);
            check("PASS disabled");
            action("Enable clipboard history");
            check("PASS enabled");
            action("Seed"); status("SEEDED");
            check("PASS reseeded");
            nested("Paste Gamma", "Delete");
            check("PASS deleted");
            absent("Paste Gamma");
            action("Clear clipboard history");
            check("PASS cleared");
            action("ABC"); key("m");
            check("PASS returned");
            result.putString("status", "passed");
            result.putInt("passed", checks.size());
            finishChecked(Activity.RESULT_OK, result);
        } catch (RuntimeException | AssertionError failure) {
            result.putString("status", "failed");
            result.putString("failure", android.util.Log.getStackTraceString(failure));
            AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
            if (root != null) {
                try {
                    result.putString("active_package", String.valueOf(root.getPackageName()));
                    ArrayList<String> labels = new ArrayList<>();
                    collectLabels(root, labels);
                    result.putStringArrayList("native_control_labels", labels);
                } finally { root.recycle(); }
            }
            finishChecked(Activity.RESULT_CANCELED, result);
        }
    }
    private void refresh() {
        action("Disable clipboard history");
        action("Enable clipboard history");
    }
    private void absent(String label) {
        AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
        if (root == null) throw new AssertionError("Missing native root");
        try {
            AccessibilityNodeInfo found = find(root, label, "TouchButton", false);
            if (found != null) {
                found.recycle();
                throw new AssertionError("Unexpected retained card " + label);
            }
        } finally { root.recycle(); }
    }
    private void nested(String card, String action) {
        AccessibilityNodeInfo parent = waitNode(card, "TouchButton", false);
        try {
            AccessibilityNodeInfo button = find(parent, action, "TouchButton", false);
            if (button == null) throw new AssertionError("Missing nested " + action);
            try {
                Rect rect = new Rect(); button.getBoundsInScreen(rect);
                if (!button.isVisibleToUser() || rect.isEmpty())
                    throw new AssertionError("Nested control is not visible");
                tap(rect);
            } finally { button.recycle(); }
        } finally { parent.recycle(); }
    }
    private void collectLabels(AccessibilityNodeInfo node, ArrayList<String> labels) {
        if ("KeyView".contentEquals(node.getClassName()) || "TouchButton".contentEquals(node.getClassName()))
            labels.add(String.valueOf(node.getText()));
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo child = node.getChild(i); if (child == null) continue;
            try { collectLabels(child, labels); } finally { child.recycle(); }
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
        result.putStringArrayList("checks", checks);
        finish(code, result);
    }
    private void check(String expected) { action("Check"); status(expected); }
    private void action(String label) { tap(bounds(label, "TouchButton", false)); }
    private void key(String label) { tap(bounds(label, "KeyView", false)); }
    private void status(String expected) {
        long deadline = SystemClock.uptimeMillis() + 30000;
        do {
            AccessibilityNodeInfo node = waitNode("PrivacyStatus", null, true);
            try {
                String actual = node.getText() == null ? "" : node.getText().toString();
                if (actual.startsWith("FAIL")) throw new AssertionError(actual);
                if (expected.equals(actual)) { checks.add(expected); return; }
            } finally { node.recycle(); }
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Native status did not reach " + expected);
    }
    private AccessibilityNodeInfo find(AccessibilityNodeInfo node, String value, String type, boolean id) {
        if (!node.refresh()) return null;
        String actualId = node.getViewIdResourceName();
        boolean matches = id ? value.equals(actualId) || actualId != null && actualId.endsWith("/" + value)
                : value.contentEquals(node.getText() == null ? "" : node.getText())
                || value.contentEquals(node.getContentDescription() == null ? "" : node.getContentDescription());
        if (matches && (type == null || type.contentEquals(node.getClassName()))) return AccessibilityNodeInfo.obtain(node);
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo child = node.getChild(i); if (child == null) continue;
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
    private Rect bounds(String value, String type, boolean id) {
        long deadline = SystemClock.uptimeMillis() + 30000;
        do {
            AccessibilityNodeInfo node = waitNode(value, type, id);
            try {
                Rect bounds = new Rect(); node.getBoundsInScreen(bounds);
                if (node.isVisibleToUser() && !bounds.isEmpty()) return bounds;
            } finally { node.recycle(); }
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Native control is not visible: " + value);
    }
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
            if (action == MotionEvent.ACTION_DOWN) contactDown = true;
            else if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_CANCEL) contactDown = false;
            contact = rect;
        } finally { event.recycle(); }
    }
}
