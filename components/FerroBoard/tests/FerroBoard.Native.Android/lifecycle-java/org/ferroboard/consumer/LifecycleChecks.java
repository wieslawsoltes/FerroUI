package org.ferroboard.consumer;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Intent;
import android.content.pm.ActivityInfo;
import android.content.res.Configuration;
import android.graphics.Rect;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import java.util.ArrayList;

/** Real native host tree changes and OS activity stop/resume/recreation. */
@SuppressWarnings("deprecation")
public final class LifecycleChecks extends Instrumentation {
    private final ArrayList<String> checks = new ArrayList<>();
    private boolean contactDown;
    private Rect contact;
    private int[] activeIds = new int[0];
    private Rect[] activeRects = new Rect[0];
    private long downTime;
    private volatile int stops;
    private volatile int resumes;
    private volatile int destroys;
    private volatile Activity current;
    private String lastKey = "none";
    private final ArrayList<String> keyPackets = new ArrayList<>();
    private boolean firstTapOnly;
    @Override public void callActivityOnStop(Activity activity) {
        super.callActivityOnStop(activity); stops++;
    }
    @Override public void callActivityOnResume(Activity activity) {
        super.callActivityOnResume(activity); current = activity; resumes++;
    }
    @Override public void callActivityOnDestroy(Activity activity) {
        super.callActivityOnDestroy(activity); destroys++;
    }
    @Override public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        String scenario = arguments == null ? "lifecycle" : arguments.getString("scenario", "lifecycle");
        if (!"lifecycle".equals(scenario) && !"first-tap".equals(scenario))
            throw new IllegalArgumentException("Unknown native scenario: " + scenario);
        firstTapOnly = "first-tap".equals(scenario);
        start();
    }
    private Intent launch() {
        Intent intent = new Intent(Intent.ACTION_MAIN);
        intent.setClassName("org.ferroboard.lifecycleconsumer", "org.ferroui.android.FerroMainActivity");
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        return intent;
    }
    private void waitCounter(String name, int before) {
        long deadline = SystemClock.uptimeMillis() + 30000;
        do {
            int observed = "stop".equals(name) ? stops : "resume".equals(name) ? resumes : destroys;
            if (observed > before) { checks.add("Native activity " + name); return; }
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Activity did not " + name);
    }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            Intent intent = launch().addFlags(Intent.FLAG_ACTIVITY_CLEAR_TASK);
            startActivitySync(intent);
            status("READY 1"); check("PASS initial");
            key("a"); check("PASS typed");
            if (firstTapOnly) {
                result.putString("status", "passed");
                result.putInt("passed", checks.size());
                finishChecked(Activity.RESULT_OK, result);
                return;
            }
            action("Remove"); status("REMOVED"); check("PASS removed");
            action("Reattach"); status("REATTACHED"); check("PASS reattached");
            key("b"); check("PASS retyped");
            int oldStops = stops;
            int oldResumes = resumes;
            // Leave a real native contact down. Android must cancel it when the
            // activity loses its window; it must never type the pending d.
            event(MotionEvent.ACTION_DOWN, bounds("d", "KeyView", false));
            sendKeyDownUpSync(KeyEvent.KEYCODE_HOME);
            waitCounter("stop", oldStops);
            getTargetContext().startActivity(launch().addFlags(
                    Intent.FLAG_ACTIVITY_REORDER_TO_FRONT | Intent.FLAG_ACTIVITY_SINGLE_TOP));
            waitCounter("resume", oldResumes);
            bounds("Check", "TouchButton", false);
            // Observe before injecting any new touch/cancel: otherwise the driver's
            // cleanup could conceal a contact that Android failed to cancel.
            AccessibilityNodeInfo observer = waitNode("Check", "TouchButton", false);
            try {
                if (!observer.performAction(AccessibilityNodeInfo.ACTION_CLICK))
                    throw new AssertionError("Native accessibility observation rejected");
            } finally { observer.recycle(); }
            status("PASS resumed");
            event(MotionEvent.ACTION_CANCEL, contact);
            key("c"); check("PASS resume typed");
            action("Release"); status("RELEASED"); check("PASS released");
            int oldDestroys = destroys;
            int beforeRecreate = resumes;
            Activity activity = current;
            if (activity == null) throw new AssertionError("Missing resumed activity");
            runOnMainSync(activity::recreate);
            waitCounter("destroy", oldDestroys);
            waitCounter("resume", beforeRecreate);
            status("READY 2"); check("PASS initial");
            key("z"); check("PASS typed");
            // This destruction has a live native adapter and IME guard: no explicit
            // library shutdown is called. The next fixture observes weak lifetimes.
            oldDestroys = destroys;
            beforeRecreate = resumes;
            Activity live = current;
            if (live == null) throw new AssertionError("Missing recreated activity");
            runOnMainSync(live::recreate);
            waitCounter("destroy", oldDestroys);
            waitCounter("resume", beforeRecreate);
            status("READY 3"); check("PASS initial");
            key("z"); check("PASS typed");
            check("PASS layout docked");
            action("Floating keyboard"); check("PASS floating");
            dragFloating(true); check("PASS floating clamped");
            dragFloating(false); check("PASS floating moved");
            action("Floating keyboard"); check("PASS dock restored");
            action("One-handed mode"); check("PASS one right");
            action("Move keyboard left"); check("PASS one left");
            action("Move keyboard right"); check("PASS one moved right");
            action("Exit one-handed mode"); check("PASS one exited");
            action("Split keyboard"); check("PASS split");
            Rect left = bounds("t", "KeyView", false);
            Rect right = bounds("y", "KeyView", false);
            int gap = left.right + (right.left - left.right) / 4;
            tap(new Rect(gap, left.centerY(), gap + 1, left.centerY() + 1));
            check("PASS split gap");
            action("One-handed mode"); check("PASS split one handed");
            action("Exit one-handed mode"); check("PASS split restored");
            action("Split keyboard"); check("PASS split disabled");
            action("Resize keyboard"); check("PASS resize opened");
            Rect hint = bounds("Drag up or down to resize", null, false);
            int originalKeyHeight = bounds("q", "KeyView", false).height();
            drag(hint, hint.centerX(), hint.centerY() - 96);
            waitKeyHeight(originalKeyHeight, true);
            check("PASS resized");
            action("Reset"); waitKeyHeight(originalKeyHeight, false);
            check("PASS resize reset");
            action("Done"); check("PASS resize done");
            key("m"); check("PASS layout typed");
            action("Scroll"); status("SCROLL READY");
            check("PASS scroll initial");
            key("n"); check("PASS scroll typed");
            action("First"); status("FOCUS FIRST");
            check("PASS scroll first");
            key("k"); check("PASS scroll first typed");
            action("Last"); status("FOCUS LAST");
            check("PASS scroll last");
            action("Resize keyboard"); check("PASS scroll resize opened");
            Rect scrollHint = bounds("Drag up or down to resize", null, false);
            int scrollKeyHeight = bounds("q", "KeyView", false).height();
            drag(scrollHint, scrollHint.centerX(), scrollHint.centerY() - 96);
            waitKeyHeight(scrollKeyHeight, true);
            check("PASS scroll resized");
            action("Reset"); waitKeyHeight(scrollKeyHeight, false);
            check("PASS scroll reset");
            action("Done"); key("p"); check("PASS scroll final");
            canceledDrags();
            orientationChanges();
            transformedRoots();
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
    private void observe(String expected) {
        AccessibilityNodeInfo observer = waitNode("Check", "TouchButton", false);
        try {
            if (!observer.performAction(AccessibilityNodeInfo.ACTION_CLICK))
                throw new AssertionError("Native capture observation rejected");
        } finally { observer.recycle(); }
        status(expected);
    }
    private void captureAction(String name, String expected) {
        AccessibilityNodeInfo node = waitNode(name, "TouchButton", false);
        try {
            if (!node.performAction(AccessibilityNodeInfo.ACTION_CLICK))
                throw new AssertionError("Native capture action rejected: " + name);
        } finally { node.recycle(); }
        status(expected);
    }
    private void canceledDrags() {
        action("Resize keyboard"); check("PASS cancel resize ready");
        int initial = bounds("q", "KeyView", false).height();
        Rect hint = bounds("Drag up or down to resize", null, false);
        beginDrag(hint, hint.centerX(), hint.centerY() - 96);
        waitKeyHeight(initial, true); observe("PASS resize held");
        event(MotionEvent.ACTION_CANCEL, contact); observe("PASS resize canceled");
        int canceled = bounds("q", "KeyView", false).height();
        hint = bounds("Drag up or down to resize", null, false);
        drag(hint, hint.centerX(), hint.centerY() - 96);
        waitKeyHeight(canceled, true); check("PASS resize recovers");
        action("Reset"); waitKeyHeight(initial, false);
        hint = bounds("Drag up or down to resize", null, false);
        beginDrag(hint, hint.centerX(), hint.centerY() - 96);
        waitKeyHeight(initial, true);
        captureAction("Steal", "CAPTURE STOLEN"); observe("PASS resize capture stolen");
        event(MotionEvent.ACTION_MOVE, new Rect(contact.left, contact.top - 50,
                contact.right, contact.bottom - 50));
        observe("PASS resize ignores lost capture");
        captureAction("Drop", "CAPTURE DROPPED");
        event(MotionEvent.ACTION_UP, contact);
        action("Reset"); waitKeyHeight(initial, false); action("Done");
        check("PASS resize lost recovery");

        action("Floating keyboard"); check("PASS cancel floating ready");
        Rect handle = bounds("FloatingDragHandle", null, true);
        beginDrag(handle, handle.centerX(), handle.centerY() - 120);
        observe("PASS floating held");
        event(MotionEvent.ACTION_CANCEL, contact); observe("PASS floating canceled");
        handle = bounds("FloatingDragHandle", null, true);
        drag(handle, handle.centerX(), handle.centerY() + 120);
        check("PASS floating recovers");
        handle = bounds("FloatingDragHandle", null, true);
        beginDrag(handle, handle.centerX(), handle.centerY() - 80);
        captureAction("Steal", "CAPTURE STOLEN"); observe("PASS floating capture stolen");
        event(MotionEvent.ACTION_MOVE, new Rect(contact.left, contact.top - 50,
                contact.right, contact.bottom - 50));
        observe("PASS floating ignores lost capture");
        captureAction("Drop", "CAPTURE DROPPED");
        event(MotionEvent.ACTION_UP, contact); check("PASS floating lost recovery");

        handle = bounds("FloatingDragHandle", null, true);
        Rect first = new Rect(handle.centerX() - 8, handle.centerY(),
                handle.centerX() - 7, handle.centerY() + 1);
        Rect second = new Rect(handle.centerX() + 8, handle.centerY(),
                handle.centerX() + 9, handle.centerY() + 1);
        eventMany(MotionEvent.ACTION_DOWN, new int[]{3}, new Rect[]{first});
        eventMany(MotionEvent.ACTION_POINTER_DOWN | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                new int[]{3, 7}, new Rect[]{first, second});
        second.offset(0, -40);
        eventMany(MotionEvent.ACTION_MOVE, new int[]{3, 7}, new Rect[]{first, second});
        observe("PASS overlap latest");
        eventMany(MotionEvent.ACTION_POINTER_UP, new int[]{3, 7}, new Rect[]{first, second});
        second.offset(0, -40);
        eventMany(MotionEvent.ACTION_MOVE, new int[]{7}, new Rect[]{second});
        observe("PASS overlap earlier release");
        eventMany(MotionEvent.ACTION_CANCEL, new int[]{7}, new Rect[]{second});
        observe("PASS overlap canceled");
        action("Floating keyboard"); key("x"); check("PASS cancel typed");
    }
    private void orientationChanges() {
        check("PASS orientation initial");
        int fullHeight = bounds("q", "KeyView", false).height();
        action("Resize keyboard"); check("PASS compact resize");
        Rect hint = bounds("Drag up or down to resize", null, false);
        drag(hint, hint.centerX(), hint.centerY() + 192);
        waitKeyShrink(fullHeight); check("PASS compact height");
        action("Done"); key("y"); check("PASS compact done");
        rotate(false); check("PASS landscape");
        key("l"); check("PASS landscape typed");
        action("Split keyboard"); check("PASS landscape split");
        rotate(true); check("PASS portrait split");
        action("Split keyboard"); check("PASS portrait restored");
        key("z"); check("PASS portrait typed");
        action("One-handed mode"); action("Move keyboard left");
        check("PASS portrait one handed");
        rotate(false); check("PASS landscape one handed");
        action("Move keyboard right"); check("PASS landscape one moved");
        rotate(true); check("PASS portrait one handed again");
        action("Exit one-handed mode"); check("PASS orientation exited");
        action("Resize keyboard"); action("Reset"); waitKeyHeight(fullHeight, false);
        check("PASS orientation reset");
        action("Done"); key("c"); check("PASS orientation typed");
    }
    private void transformedRoots() {
        check("PASS transform initial");
        int height = bounds("q", "KeyView", false).height();
        action("Zoom"); status("TRANSFORM SCALED"); waitKeyShrink(height);
        check("PASS transform scaled");
        key("b"); check("PASS transform typed");
        action("Resize keyboard"); check("PASS transform resize");
        Rect hint = bounds("Drag up or down to resize", null, false);
        event(MotionEvent.ACTION_DOWN, hint); observe("PASS transform held");
        captureAction("Restore", "TRANSFORM RESTORED"); waitKeyHeight(height, false);
        observe("PASS transform changed held");
        moveDrag(hint, hint.centerX(), hint.centerY() - 96);
        waitKeyHeight(height, true); observe("PASS transform moved");
        event(MotionEvent.ACTION_UP, contact); observe("PASS transform released");
        action("Reset"); waitKeyHeight(height, false); check("PASS transform reset");
        hint = bounds("Drag up or down to resize", null, false);
        drag(hint, hint.centerX(), hint.centerY() - 96);
        waitKeyHeight(height, true); check("PASS transform fresh drag");
        action("Reset"); waitKeyHeight(height, false); check("PASS transform final reset");
        action("Done"); key("n"); check("PASS transform final typed");
    }
    private void rotate(boolean portrait) {
        Activity activity = current;
        if (activity == null) throw new AssertionError("Missing activity before rotation");
        int oldDestroys = destroys;
        int oldWidth = bounds("q", "KeyView", false).width();
        int expected = portrait ? Configuration.ORIENTATION_PORTRAIT : Configuration.ORIENTATION_LANDSCAPE;
        runOnMainSync(() -> activity.setRequestedOrientation(portrait
                ? ActivityInfo.SCREEN_ORIENTATION_PORTRAIT : ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE));
        long deadline = SystemClock.uptimeMillis() + 30000;
        do {
            int[] orientation = new int[1];
            runOnMainSync(() -> orientation[0] = activity.getResources().getConfiguration().orientation);
            AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
            boolean aspect = false;
            if (root != null) {
                try {
                    Rect rect = new Rect(); root.getBoundsInScreen(rect);
                    aspect = "org.ferroboard.lifecycleconsumer".contentEquals(root.getPackageName())
                            && (portrait ? rect.height() > rect.width() : rect.width() > rect.height());
                } finally { root.recycle(); }
            }
            if (orientation[0] == expected && aspect) {
                int width = bounds("q", "KeyView", false).width();
                if (portrait ? width < oldWidth : width > oldWidth) {
                    if (current != activity || destroys != oldDestroys)
                        throw new AssertionError("Configuration-handling activity was recreated");
                    checks.add("Native rotation " + (portrait ? "portrait" : "landscape"));
                    return;
                }
            }
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Native rotation/layout did not settle: " + expected);
    }
    private void waitKeyShrink(int baseline) {
        long deadline = SystemClock.uptimeMillis() + 30000;
        do {
            if (bounds("q", "KeyView", false).height() < baseline) return;
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Native key layout did not shrink: " + baseline);
    }
    private void waitKeyHeight(int baseline, boolean grow) {
        long deadline = SystemClock.uptimeMillis() + 30000;
        int observed;
        do {
            observed = bounds("q", "KeyView", false).height();
            if (grow ? observed > baseline : observed == baseline) return;
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Native key layout did not " + (grow ? "grow" : "reset")
                + "; baseline=" + baseline + ", observed=" + observed);
    }
    private void dragFloating(boolean clamp) {
        Rect handle = bounds("FloatingDragHandle", null, true);
        if (!clamp) { drag(handle, handle.centerX(), handle.centerY() - 120); return; }
        AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
        if (root == null) throw new AssertionError("Missing native window");
        Rect window = new Rect();
        try { root.getBoundsInScreen(window); } finally { root.recycle(); }
        drag(handle, window.left + 20, window.bottom - 80);
    }
    private void drag(Rect start, int x, int y) {
        beginDrag(start, x, y);
        event(MotionEvent.ACTION_UP, new Rect(x, y, x + 1, y + 1));
    }
    private void beginDrag(Rect start, int x, int y) {
        event(MotionEvent.ACTION_DOWN, start);
        moveDrag(start, x, y);
    }
    private void moveDrag(Rect start, int x, int y) {
        for (int i = 1; i <= 12; i++) {
            int nextX = start.centerX() + (x - start.centerX()) * i / 12;
            int nextY = start.centerY() + (y - start.centerY()) * i / 12;
            event(MotionEvent.ACTION_MOVE, new Rect(nextX, nextY, nextX + 1, nextY + 1));
            SystemClock.sleep(20);
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
            try { eventMany(MotionEvent.ACTION_CANCEL, activeIds, activeRects); }
            catch (RuntimeException | AssertionError failure) {
                result.putString("status", "failed");
                result.putString("cleanup_failure", android.util.Log.getStackTraceString(failure));
                code = Activity.RESULT_CANCELED;
            }
        }
        result.putStringArrayList("checks", checks);
        result.putString("last_key", lastKey);
        result.putStringArrayList("key_packets", keyPackets);
        finish(code, result);
    }
    private void check(String expected) { action("Check"); status(expected); }
    private void action(String label) { tap(bounds(label, "TouchButton", false)); }
    private void key(String label) {
        Rect rect = bounds(label, "KeyView", false);
        long start = SystemClock.uptimeMillis();
        lastKey = label + " bounds=" + rect + " down=" + start;
        tap(rect);
        lastKey += " elapsed_ms=" + (SystemClock.uptimeMillis() - start);
        keyPackets.add(lastKey);
    }
    private void status(String expected) {
        long deadline = SystemClock.uptimeMillis() + 30000;
        do {
            AccessibilityNodeInfo node = waitNode("LifecycleStatus", null, true);
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
        eventMany(action, new int[]{3}, new Rect[]{rect});
    }
    private void eventMany(int action, int[] ids, Rect[] rects) {
        if (ids.length == 0 || ids.length != rects.length)
            throw new AssertionError("Invalid native contact arrays");
        long now = SystemClock.uptimeMillis();
        int masked = action & MotionEvent.ACTION_MASK;
        if (masked == MotionEvent.ACTION_DOWN) downTime = now;
        MotionEvent.PointerProperties[] properties = new MotionEvent.PointerProperties[ids.length];
        MotionEvent.PointerCoords[] coords = new MotionEvent.PointerCoords[ids.length];
        for (int i = 0; i < ids.length; i++) {
            properties[i] = new MotionEvent.PointerProperties();
            properties[i].id = ids[i]; properties[i].toolType = MotionEvent.TOOL_TYPE_FINGER;
            coords[i] = new MotionEvent.PointerCoords();
            coords[i].x = rects[i].exactCenterX(); coords[i].y = rects[i].exactCenterY();
            coords[i].pressure = 1; coords[i].size = 1;
        }
        MotionEvent event = MotionEvent.obtain(downTime, now, action, ids.length, properties, coords,
                0, 0, 1, 1, 0, 0, InputDevice.SOURCE_TOUCHSCREEN, 0);
        try {
            if (!getUiAutomation().injectInputEvent(event, true)) throw new AssertionError("Native touch rejected");
            if (masked == MotionEvent.ACTION_UP || masked == MotionEvent.ACTION_CANCEL) {
                activeIds = new int[0]; activeRects = new Rect[0];
            } else if (masked == MotionEvent.ACTION_POINTER_UP) {
                int index = (action & MotionEvent.ACTION_POINTER_INDEX_MASK) >> MotionEvent.ACTION_POINTER_INDEX_SHIFT;
                activeIds = new int[ids.length - 1]; activeRects = new Rect[ids.length - 1];
                for (int i = 0, next = 0; i < ids.length; i++) if (i != index) {
                    activeIds[next] = ids[i]; activeRects[next++] = new Rect(rects[i]);
                }
            } else {
                activeIds = ids.clone(); activeRects = new Rect[rects.length];
                for (int i = 0; i < rects.length; i++) activeRects[i] = new Rect(rects[i]);
            }
            contactDown = activeIds.length != 0;
            contact = new Rect(rects[rects.length - 1]);
        } finally { event.recycle(); }
    }
}
