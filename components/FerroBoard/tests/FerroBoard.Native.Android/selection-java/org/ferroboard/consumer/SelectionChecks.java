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

/** Synthetic native taps trigger real FerroUI selection operations in an opt-in fixture. */
@SuppressWarnings("deprecation")
public final class SelectionChecks extends Instrumentation {
    private final ArrayList<String> checks = new ArrayList<>();
    private boolean contactDown;
    private Rect contact;
    private long downTime;
    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        int code = Activity.RESULT_OK;
        try {
            Intent intent = new Intent(Intent.ACTION_MAIN);
            intent.setClassName("org.ferroboard.selectionconsumer", "org.ferroui.android.FerroMainActivity");
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            startActivitySync(intent);
            status("READY selection", 30000);
            String[] labels = {"Range", "Reentry", "Collapse", "Inactive"};
            int[] masks = {1, 3, 7, 15};
            for (int i = 0; i < labels.length; i++) {
                AccessibilityNodeInfo button = waitButton(labels[i]);
                Rect bounds = new Rect();
                try { button.getBoundsInScreen(bounds); } finally { button.recycle(); }
                if (bounds.isEmpty()) throw new AssertionError("Empty native button bounds: " + labels[i]);
                contact = bounds;
                event(MotionEvent.ACTION_DOWN);
                event(MotionEvent.ACTION_UP);
                String expected = "PASS " + labels[i] + " mask=" + masks[i] + " first="
                        + (i == 2 ? "None" : "Some((1, 5))");
                status(expected, 5000);
                String focused = i == 3 ? "SelectionOther" : "SelectionMessage";
                requireFocus(focused);
                checks.add(expected + " focus=" + focused);
            }
            result.putString("status", "passed");
            result.putInt("passed", checks.size());
        } catch (RuntimeException | AssertionError failure) {
            code = Activity.RESULT_CANCELED;
            result.putString("status", "failed");
            result.putString("failure", android.util.Log.getStackTraceString(failure));
        } finally {
            if (contactDown) {
                try { event(MotionEvent.ACTION_CANCEL); }
                catch (RuntimeException | AssertionError failure) {
                    code = Activity.RESULT_CANCELED;
                    result.putString("status", "failed");
                    result.putString("cleanup_failure", android.util.Log.getStackTraceString(failure));
                }
            }
        }
        result.putStringArrayList("checks", checks);
        finish(code, result);
    }
    private void event(int action) {
        long now = SystemClock.uptimeMillis();
        if (action == MotionEvent.ACTION_DOWN) downTime = now;
        MotionEvent.PointerProperties properties = new MotionEvent.PointerProperties();
        properties.id = 29;
        properties.toolType = MotionEvent.TOOL_TYPE_FINGER;
        MotionEvent.PointerCoords coords = new MotionEvent.PointerCoords();
        coords.x = contact.exactCenterX(); coords.y = contact.exactCenterY();
        coords.pressure = 1; coords.size = 1;
        MotionEvent event = MotionEvent.obtain(downTime, now, action, 1,
                new MotionEvent.PointerProperties[]{properties},
                new MotionEvent.PointerCoords[]{coords}, 0, 0, 1, 1, 0, 0,
                InputDevice.SOURCE_TOUCHSCREEN, 0);
        try {
            if (!getUiAutomation().injectInputEvent(event, true))
                throw new AssertionError("Native injection rejected: " + action);
            contactDown = action == MotionEvent.ACTION_DOWN;
        } finally { event.recycle(); }
    }
    private AccessibilityNodeInfo find(AccessibilityNodeInfo node, String label, boolean button) {
        if (node == null) return null;
        String text = node.getText() == null ? "" : node.getText().toString();
        String name = node.getContentDescription() == null ? "" : node.getContentDescription().toString();
        String kind = node.getClassName() == null ? "" : node.getClassName().toString();
        if (node.isVisibleToUser() && (label.equals(text) || label.equals(name) || label.equals(node.getViewIdResourceName()))
                && (!button || kind.contains("TouchButton"))) return node;
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo child = find(node.getChild(i), label, button);
            if (child != null) { node.recycle(); return child; }
        }
        node.recycle();
        return null;
    }
    private AccessibilityNodeInfo waitButton(String label) {
        long end = SystemClock.uptimeMillis() + 5000;
        do {
            AccessibilityNodeInfo found = find(getUiAutomation().getRootInActiveWindow(), label, true);
            if (found != null) return found;
            SystemClock.sleep(20);
        } while (SystemClock.uptimeMillis() < end);
        throw new AssertionError("Native button missing: " + label);
    }
    private void status(String expected, long timeout) {
        long end = SystemClock.uptimeMillis() + timeout;
        do {
            AccessibilityNodeInfo found = find(getUiAutomation().getRootInActiveWindow(), expected, false);
            if (found != null) { found.recycle(); return; }
            SystemClock.sleep(20);
        } while (SystemClock.uptimeMillis() < end);
        throw new AssertionError("Native status missing: " + expected);
    }
    private void requireFocus(String expected) {
        AccessibilityNodeInfo first = find(getUiAutomation().getRootInActiveWindow(), "SelectionMessage", false);
        AccessibilityNodeInfo second = find(getUiAutomation().getRootInActiveWindow(), "SelectionOther", false);
        AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
        AccessibilityNodeInfo focused = root == null ? null : root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
        try {
            String actual = focused == null ? null : focused.getViewIdResourceName();
            if (first == null || second == null
                    || first.isFocused() != "SelectionMessage".equals(expected)
                    || second.isFocused() != "SelectionOther".equals(expected)
                    || !expected.equals(actual)) {
                throw new AssertionError("Keyboard AX focus mismatch: expected=" + expected
                        + " findFocus=" + actual + " message=" + (first == null ? null : first.isFocused())
                        + " other=" + (second == null ? null : second.isFocused()) + " tree=" + describe(root));
            }
        } finally {
            if (first != null) first.recycle();
            if (second != null) second.recycle();
            if (focused != null) focused.recycle();
            if (root != null) root.recycle();
        }
    }
    private String describe(AccessibilityNodeInfo node) {
        if (node == null) return "null";
        StringBuilder text = new StringBuilder();
        text.append('[').append(node.getPackageName()).append(':').append(node.getClassName())
                .append(':').append(node.getContentDescription()).append(':').append(node.getText())
                .append(" focused=").append(node.isFocused()).append(" visible=").append(node.isVisibleToUser()).append(']');
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo child = node.getChild(i);
            try { text.append(describe(child)); } finally { if (child != null) child.recycle(); }
        }
        return text.toString();
    }
}
