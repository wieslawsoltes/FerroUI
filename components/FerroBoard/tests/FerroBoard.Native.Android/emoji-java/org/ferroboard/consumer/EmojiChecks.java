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

/** Actual default panel/search/back controls and key touches, with fresh native observations. */
@SuppressWarnings("deprecation") // API26 recyclable accessibility nodes.
public final class EmojiChecks extends Instrumentation {
    private final ArrayList<String> checks = new ArrayList<>();
    private boolean contactDown;
    private Rect contact;
    private long downTime;
    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            Intent intent = new Intent(Intent.ACTION_MAIN);
            intent.setClassName("org.ferroboard.emojiconsumer", "org.ferroui.android.FerroMainActivity");
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            startActivitySync(intent);
            status("READY");
            check("PASS private initial");
            privateNode();
            action("Emoji");
            action("Search emoji");
            query();
            queryVisible();
            privateNode();
            action("🚀");
            queryVisible();
            privateNode();
            action("Back");
            action("ABC");
            check("PASS private selected");
            key("Backspace");
            check("PASS private cleared");

            tap(bounds("PublicInput", null, true));
            check("PASS public initial");
            action("Emoji");
            action("Search emoji");
            query();
            queryVisible();
            publicText("");
            key("Enter");
            queryVisible();
            publicText("🚀");
            action("Back");
            action("ABC");
            check("PASS public selected");
            key("m");
            check("PASS public typed");
            publicText("🚀m");
            privateNode();
            action("Emoji");
            action("PeopleBody");
            check("PASS category initial");
            swipeEmojiUp();
            check("PASS category scrolled");
            action("SmileysEmotion");
            action("PeopleBody");
            check("PASS category reset");
            longPress("👋", "👋🏽");
            publicText("🚀m");
            action("👋🏽");
            check("PASS tone selected");
            action("Recent");
            longPress("👋🏽", "Pin");
            action("Pin");
            check("PASS recent pinned");
            longPress("👋🏽", "Unpin");
            action("Remove");
            check("PASS recent removed");
            action("PeopleBody");
            action("👋🏽");
            check("PASS preferred selected");
            action("ABC");
            key("Backspace");
            check("PASS tone deleted");
            publicText("🚀m👋🏽");
            tap(bounds("PasswordInput", null, true));
            check("PASS private tone initial");
            action("Emoji");
            longPress("👋🏽", "👋🏿");
            action("👋🏿");
            privateNode();
            action("ABC");
            check("PASS private tone selected");
            key("Backspace");
            check("PASS private tone cleared");
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
    private void query() { for (String label : new String[]{"r", "o", "c", "k", "e", "t", "Space"}) key(label); }
    private void longPress(String label, String popupControl) {
        Rect anchor = bounds(label, "TouchButton", false);
        event(MotionEvent.ACTION_DOWN, anchor);
        // Observe the real timer-created popup, holding the native contact throughout.
        bounds(popupControl, "TouchButton", false);
        event(MotionEvent.ACTION_UP, anchor);
        checks.add("Native long press opens " + popupControl);
    }
    private void swipeEmojiUp() {
        Rect viewport = bounds("EmojiScrollBody", "ScrollViewer", true);
        AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
        if (root == null) throw new AssertionError("Missing native scroll root");
        Rect cell;
        try { cell = bottomCell(root, viewport, null); } finally { root.recycle(); }
        if (cell == null || cell.centerY() <= viewport.centerY())
            throw new AssertionError("Missing fully visible native emoji cell for drag");
        int x = cell.centerX();
        int start = cell.centerY();
        int end = viewport.top + viewport.height() / 5;
        event(MotionEvent.ACTION_DOWN, new Rect(x, start, x + 1, start + 1));
        for (int i = 1; i <= 12; i++) {
            int y = start + (end - start) * i / 12;
            event(MotionEvent.ACTION_MOVE, new Rect(x, y, x + 1, y + 1));
            SystemClock.sleep(20);
        }
        // A stopped-finger drag: avoid launching a fling while verifying the offset.
        // The selected framework treats >40ms without movement as stopped.
        SystemClock.sleep(80);
        event(MotionEvent.ACTION_UP, new Rect(x, end, x + 1, end + 1));
    }
    private Rect bottomCell(AccessibilityNodeInfo node, Rect viewport, Rect best) {
        if ("TouchButton".contentEquals(node.getClassName()) && node.isVisibleToUser()) {
            Rect bounds = new Rect(); node.getBoundsInScreen(bounds);
            if (!bounds.isEmpty() && viewport.contains(bounds)
                    && (best == null || bounds.bottom > best.bottom)) best = bounds;
        }
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo child = node.getChild(i); if (child == null) continue;
            try { best = bottomCell(child, viewport, best); } finally { child.recycle(); }
        }
        return best;
    }
    private void queryVisible() {
        AccessibilityNodeInfo node = waitNode("rocket |", null, false);
        try {
            if (!node.isVisibleToUser()) throw new AssertionError("Native query is not visible");
            checks.add("Native query remains in private search strip");
        } finally { node.recycle(); }
    }
    private void publicText(String expected) {
        long deadline = SystemClock.uptimeMillis() + 5000;
        String observed = null;
        do {
            AccessibilityNodeInfo node = waitNode("PublicInput", null, true);
            try {
                String actual = node.getText() == null ? "" : node.getText().toString();
                observed = actual;
                if (expected.equals(actual)) { checks.add("Native public value matches"); return; }
            } finally { node.recycle(); }
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Native public text mismatch; observed=" + observed);
    }
    private void privateNode() {
        AccessibilityNodeInfo node = waitNode("PasswordInput", null, true);
        try {
            if (!node.refresh() || node.getChildCount() != 0) throw new AssertionError("Native password children exposed");
            for (CharSequence value : new CharSequence[]{node.getText(), node.getContentDescription()})
                if (value != null && value.length() != 0 && !"Password input".contentEquals(value))
                    throw new AssertionError("Native password value exposed");
            checks.add("Native password remains opaque");
        } finally { node.recycle(); }
    }
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
