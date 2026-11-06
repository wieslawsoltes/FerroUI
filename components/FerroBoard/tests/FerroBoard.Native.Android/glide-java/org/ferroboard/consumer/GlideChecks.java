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

/** Actual Android batched and unbatched glide gestures; no engine calls. */
@SuppressWarnings("deprecation")
public final class GlideChecks extends Instrumentation {
    private final ArrayList<String> checks = new ArrayList<>();
    private boolean contactDown;
    private float[] contact;
    private long downTime;
    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            Intent intent = new Intent(Intent.ACTION_MAIN);
            intent.setClassName("org.ferroboard.glideconsumer", "org.ferroui.android.FerroMainActivity");
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            startActivitySync(intent);
            status("READY"); check("PASS initial");
            ArrayList<float[]> hello = path("hello");
            event(MotionEvent.ACTION_DOWN, hello.get(0)); observe("PASS pressed");
            batch(hello); observe("PASS batch preview");
            event(MotionEvent.ACTION_UP, contact); check("PASS batch committed");
            tap(bounds("GlideAlternate", "TouchButton", true)); check("PASS replacement");
            event(MotionEvent.ACTION_DOWN, hello.get(0));
            for (int i = 1; i < hello.size(); i++) event(MotionEvent.ACTION_MOVE, hello.get(i));
            observe("PASS unbatched preview");
            event(MotionEvent.ACTION_UP, contact); check("PASS unbatched committed");
            ArrayList<float[]> world = path("world");
            event(MotionEvent.ACTION_DOWN, world.get(0));
            batch(world); observe("PASS cancel preview");
            event(MotionEvent.ACTION_CANCEL, contact); observe("PASS canceled");
            tap(bounds("x", "KeyView", false)); check("PASS recovered");
            casingAndGeometry();
            logicalLayout();
            result.putString("status", "passed");
            result.putInt("passed", checks.size());
            finishChecked(Activity.RESULT_OK, result);
        } catch (RuntimeException | AssertionError failure) {
            result.putString("status", "failed");
            result.putString("failure", android.util.Log.getStackTraceString(failure));
            finishChecked(Activity.RESULT_CANCELED, result);
        }
    }
    private void casingAndGeometry() {
        Rect shift = bounds("Shift", "KeyView", false);
        tap(shift); check("PASS manual ready");
        ArrayList<float[]> upper = path("HELLO");
        event(MotionEvent.ACTION_DOWN, upper.get(0)); batch(upper); observe("PASS manual preview");
        event(MotionEvent.ACTION_UP, contact); check("PASS manual committed");
        float[] shiftPoint = {shift.exactCenterX(), shift.exactCenterY()};
        event(MotionEvent.ACTION_DOWN, shiftPoint);
        SystemClock.sleep(450); observe("PASS caps held");
        event(MotionEvent.ACTION_UP, contact); check("PASS caps ready");
        upper = path("HELLO");
        event(MotionEvent.ACTION_DOWN, upper.get(0)); batch(upper); observe("PASS caps preview");
        event(MotionEvent.ACTION_UP, contact); check("PASS caps committed");
        tap(bounds("X", "KeyView", false)); check("PASS caps typed");
        tap(shift); check("PASS caps exited");
        int height = bounds("q", "KeyView", false).height();
        ArrayList<float[]> start = path("hello");
        event(MotionEvent.ACTION_DOWN, start.get(0));
        for (int i = 1; i <= 8; i++) event(MotionEvent.ACTION_MOVE, start.get(i));
        observe("PASS geometry before");
        invoke("Zoom"); status("ZOOMED"); waitKeyHeight(height, true);
        observe("PASS geometry changed");
        ArrayList<float[]> rest = path("ello");
        for (float[] point : rest) event(MotionEvent.ACTION_MOVE, point);
        observe("PASS geometry preview");
        event(MotionEvent.ACTION_UP, contact); check("PASS geometry committed");
        tap(bounds("Restore", "TouchButton", false)); status("RESTORED"); waitKeyHeight(height, false);
        check("PASS geometry restored");
        tap(bounds("x", "KeyView", false)); check("PASS case final");
    }
    private void waitKeyHeight(int height, boolean shrink) {
        long deadline = SystemClock.uptimeMillis() + 30000;
        do {
            int observed = bounds("q", "KeyView", false).height();
            if (shrink ? observed < height : observed == height) return;
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Native transform bounds did not settle");
    }
    private void logicalLayout() {
        Rect a = bounds("a", "KeyView", false);
        event(MotionEvent.ACTION_DOWN, new float[]{a.exactCenterX(), a.exactCenterY()});
        observe("PASS layout pressed");
        SystemClock.sleep(450);
        invoke("Row"); status("ROW ON"); waitNumberRow(true);
        observe("PASS popup layout replaced");
        event(MotionEvent.ACTION_UP, contact); check("PASS popup layout released");
        ArrayList<float[]> start = path("hello");
        event(MotionEvent.ACTION_DOWN, start.get(0));
        for (int i = 1; i <= 8; i++) event(MotionEvent.ACTION_MOVE, start.get(i));
        observe("PASS layout glide before");
        invoke("Row"); status("ROW OFF"); waitNumberRow(false);
        observe("PASS layout glide changed");
        for (float[] point : path("ello")) event(MotionEvent.ACTION_MOVE, point);
        observe("PASS layout glide preview");
        event(MotionEvent.ACTION_UP, contact); check("PASS layout glide committed");
        tap(bounds("x", "KeyView", false)); check("PASS layout recovery");
    }
    private void waitNumberRow(boolean present) {
        long deadline = SystemClock.uptimeMillis() + 30000;
        do {
            AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
            if (root != null) {
                try {
                    AccessibilityNodeInfo key = find(root, "1", "KeyView", false);
                    boolean found = key != null;
                    if (key != null) key.recycle();
                    if (found == present) return;
                } finally { root.recycle(); }
            }
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Live number-row layout did not settle");
    }
    private ArrayList<float[]> path(String word) {
        ArrayList<float[]> points = new ArrayList<>();
        float[] previous = null;
        for (int c = 0; c < word.length(); c++) {
            Rect rect = bounds(word.substring(c, c + 1), "KeyView", false);
            float[] next = {rect.exactCenterX(), rect.exactCenterY()};
            if (previous == null) points.add(next);
            else for (int step = 1; step <= 8; step++) {
                float t = step / 8f;
                points.add(new float[]{previous[0] + (next[0] - previous[0]) * t,
                        previous[1] + (next[1] - previous[1]) * t});
            }
            previous = next;
        }
        return points;
    }
    private void batch(ArrayList<float[]> points) {
        // Build a real MOVE history with past, increasing event times. This short
        // collection window remains below the source's unchanged long-press delay.
        SystemClock.sleep(100);
        long now = SystemClock.uptimeMillis();
        long first = now - (points.size() - 2) * 2L;
        if (first < downTime) throw new AssertionError("Invalid native history timing");
        MotionEvent event = packet(MotionEvent.ACTION_MOVE, points.get(1), first);
        try {
            for (int i = 2; i < points.size(); i++) {
                MotionEvent.PointerCoords[] coords = {coords(points.get(i))};
                event.addBatch(first + (i - 1) * 2L, coords, 0);
            }
            if (event.getHistorySize() != points.size() - 2)
                throw new AssertionError("Native MOVE history was not constructed");
            if (!getUiAutomation().injectInputEvent(event, true))
                throw new AssertionError("Native batched touch rejected");
            contact = points.get(points.size() - 1).clone();
        } finally { event.recycle(); }
    }
    private void finishChecked(int code, Bundle result) {
        if (contactDown) {
            try { event(MotionEvent.ACTION_CANCEL, contact); }
            catch (RuntimeException | AssertionError failure) {
                result.putString("cleanup_failure", android.util.Log.getStackTraceString(failure));
                result.putString("status", "failed"); code = Activity.RESULT_CANCELED;
            }
        }
        result.putStringArrayList("checks", checks); finish(code, result);
    }
    private void check(String expected) { tap(bounds("Check", "TouchButton", false)); status(expected); }
    private void observe(String expected) {
        invoke("Check");
        status(expected);
    }
    private void invoke(String name) {
        AccessibilityNodeInfo node = waitNode(name, "TouchButton", false);
        try {
            if (!node.performAction(AccessibilityNodeInfo.ACTION_CLICK))
                throw new AssertionError("Native glide observation rejected");
        } finally { node.recycle(); }
    }
    private void status(String expected) {
        long deadline = SystemClock.uptimeMillis() + 30000;
        do {
            AccessibilityNodeInfo node = waitNode("GlideStatus", null, true);
            try {
                String actual = node.getText() == null ? "" : node.getText().toString();
                if (actual.startsWith("FAIL")) throw new AssertionError(actual);
                if (actual.equals(expected)) { checks.add(expected); return; }
            } finally { node.recycle(); }
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Native glide status did not reach " + expected);
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

    private void tap(Rect rect) {
        float[] point = {rect.exactCenterX(), rect.exactCenterY()};
        event(MotionEvent.ACTION_DOWN, point); event(MotionEvent.ACTION_UP, point);
    }
    private MotionEvent.PointerCoords coords(float[] point) {
        MotionEvent.PointerCoords coords = new MotionEvent.PointerCoords();
        coords.x = point[0]; coords.y = point[1]; coords.pressure = 1; coords.size = 1;
        return coords;
    }
    private MotionEvent packet(int action, float[] point, long time) {
        MotionEvent.PointerProperties properties = new MotionEvent.PointerProperties();
        properties.id = 29; properties.toolType = MotionEvent.TOOL_TYPE_FINGER;
        return MotionEvent.obtain(downTime, time, action, 1,
                new MotionEvent.PointerProperties[]{properties},
                new MotionEvent.PointerCoords[]{coords(point)}, 0, 0, 1, 1, 0, 0,
                InputDevice.SOURCE_TOUCHSCREEN, 0);
    }
    private void event(int action, float[] point) {
        long now = SystemClock.uptimeMillis();
        if (action == MotionEvent.ACTION_DOWN) downTime = now;
        MotionEvent event = packet(action, point, now);
        try {
            if (!getUiAutomation().injectInputEvent(event, true))
                throw new AssertionError("Native glide touch rejected");
            contactDown = action != MotionEvent.ACTION_UP && action != MotionEvent.ACTION_CANCEL;
            contact = point.clone();
        } finally { event.recycle(); }
    }
}
