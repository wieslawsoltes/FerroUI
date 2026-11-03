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
import java.util.HashMap;

/** Synthetic native contacts through Android's actual input dispatch, never engine calls. */
@SuppressWarnings("deprecation") // recycle remains necessary on the selected minimum API26.
public final class PointerChecks extends Instrumentation {
    private final ArrayList<String> checks = new ArrayList<>();
    private final ArrayList<String> selectedKeys = new ArrayList<>();
    private final ArrayList<String> injectionTimings = new ArrayList<>();
    private final ArrayList<String> fieldFocusFlags = new ArrayList<>();
    private final HashMap<Integer, Long> pressedAt = new HashMap<>();
    private AccessibilityNodeInfo observedField;
    private long downTime;
    private int[] activeIds = new int[0];
    private Rect[] activeBounds = new Rect[0];

    @Override public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        start();
    }

    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            Intent intent = new Intent(Intent.ACTION_MAIN);
            intent.setClassName("org.ferroboard.consumer", "org.ferroui.android.FerroMainActivity");
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            startActivitySync(intent);
            expect("mail", "initial private field", 30000);

            // Multiple contacts use nonsequential Android IDs and index-based up actions.
            Rect a = key("a"), b = key("b"), shift = key("Shift");
            event(MotionEvent.ACTION_DOWN, new int[]{7}, a);
            expect("mail", "typing waits for release");
            event(MotionEvent.ACTION_POINTER_DOWN | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                    new int[]{7, 29}, a, b);
            expect("amail", "second contact commits rollover predecessor");
            event(MotionEvent.ACTION_POINTER_UP, new int[]{7, 29}, a, b);
            expect("amail", "retired first up does not duplicate text");
            event(MotionEvent.ACTION_UP, new int[]{29}, b);
            expect("abmail", "remaining contact release commits once");
            erase(2);

            event(MotionEvent.ACTION_DOWN, new int[]{29}, a);
            event(MotionEvent.ACTION_POINTER_DOWN | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                    new int[]{29, 7}, a, b);
            event(MotionEvent.ACTION_POINTER_UP | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                    new int[]{29, 7}, a, b);
            expect("abmail", "reverse release commits second contact");
            event(MotionEvent.ACTION_UP, new int[]{29}, a);
            expect("abmail", "reverse release retires first contact without duplication");
            erase(2);

            // A held Shift is a separate captured contact, not a simulated key command.
            event(MotionEvent.ACTION_DOWN, new int[]{7}, shift);
            event(MotionEvent.ACTION_POINTER_DOWN | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                    new int[]{7, 29}, shift, a);
            event(MotionEvent.ACTION_POINTER_UP | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                    new int[]{7, 29}, shift, a);
            event(MotionEvent.ACTION_UP, new int[]{7}, shift);
            expect("Amail", "held Shift chord capitalizes letter");
            erase(1);
            tap(key("b"));
            expect("bmail", "Shift chord leaves subsequent letter lowercase");
            erase(1);

            event(MotionEvent.ACTION_DOWN, new int[]{7}, a);
            event(MotionEvent.ACTION_CANCEL, new int[]{7}, a);
            SystemClock.sleep(650); // source long-press delay has elapsed after cancellation.
            expect("mail", "canceled contact has no late typing or long-press commit");
            tap(key("b"));
            expect("bmail", "new contact works after cancellation");
            erase(1);

            event(MotionEvent.ACTION_DOWN, new int[]{7}, a);
            event(MotionEvent.ACTION_POINTER_DOWN | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                    new int[]{7, 29}, a, b);
            event(MotionEvent.ACTION_CANCEL, new int[]{7, 29}, a, b);
            expect("amail", "cancel preserves prior rollover commit and drops pending contact");
            erase(1);
            tap(key("q"));
            expect("qmail", "new sequence works after multi-contact cancel");
            erase(1);

            // API33's canceled pointer-up retires one contact without committing it.
            if (android.os.Build.VERSION.SDK_INT < 33)
                throw new AssertionError("FLAG_CANCELED checks require API33+");
            event(MotionEvent.ACTION_DOWN, new int[]{7}, a);
            event(MotionEvent.ACTION_POINTER_DOWN | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                    new int[]{7, 29}, a, b);
            canceledUp(1, new int[]{7, 29}, a, b);
            expect("amail", "canceled second contact does not insert");
            event(MotionEvent.ACTION_UP, new int[]{7}, a);
            expect("amail", "first contact retires without duplication after canceled second");
            erase(1);

            event(MotionEvent.ACTION_DOWN, new int[]{7}, a);
            event(MotionEvent.ACTION_POINTER_DOWN | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                    new int[]{7, 29}, a, b);
            canceledUp(0, new int[]{7, 29}, a, b);
            expect("amail", "canceled first contact preserves prior rollover commit");
            event(MotionEvent.ACTION_UP, new int[]{29}, b);
            expect("abmail", "uncanceled second contact still commits");
            erase(2);

            event(MotionEvent.ACTION_DOWN, new int[]{7}, shift);
            event(MotionEvent.ACTION_POINTER_DOWN | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                    new int[]{7, 29}, shift, a);
            canceledUp(1, new int[]{7, 29}, shift, a);
            expect("mail", "canceled chord letter does not insert");
            event(MotionEvent.ACTION_POINTER_DOWN | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                    new int[]{7, 29}, shift, b);
            event(MotionEvent.ACTION_POINTER_UP | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                    new int[]{7, 29}, shift, b);
            event(MotionEvent.ACTION_UP, new int[]{7}, shift);
            expect("Bmail", "uncanceled held Shift still chords a replacement contact");
            erase(1);
            tap(key("q"));
            expect("qmail", "replacement chord returns to lowercase typing");
            erase(1);

            result.putString("status", "passed");
            result.putStringArrayList("checks", checks);
            result.putStringArrayList("key_bounds", selectedKeys);
            result.putInt("passed", checks.size());
            complete(Activity.RESULT_OK, result);
        } catch (RuntimeException | AssertionError failure) {
            result.putString("status", "failed");
            result.putStringArrayList("checks", checks);
            result.putStringArrayList("key_bounds", selectedKeys);
            result.putString("failure", android.util.Log.getStackTraceString(failure));
            complete(Activity.RESULT_CANCELED, result);
        }
    }

    private void complete(int code, Bundle result) {
        // A failed assertion must not leave a synthetic contact down in Android.
        if (activeIds.length != 0) {
            try {
                event(MotionEvent.ACTION_CANCEL, activeIds, activeBounds);
            } catch (RuntimeException | AssertionError failure) {
                result.putString("status", "failed");
                result.putString("cleanup_failure", android.util.Log.getStackTraceString(failure));
                code = Activity.RESULT_CANCELED;
            }
        }
        result.putStringArrayList("injection_timings", injectionTimings);
        result.putStringArrayList("field_focus_flags", fieldFocusFlags);
        result.putString("observation", "cached native TextBox node refresh; no full-tree query during held contacts");
        if (observedField != null) {
            observedField.recycle();
            observedField = null;
        }
        finish(code, result);
    }

    private void erase(int count) {
        for (int index = 0; index < count; index++) tap(key("Backspace"));
        expect("mail", "cleanup restores initial text");
    }

    private void tap(Rect rect) {
        event(MotionEvent.ACTION_DOWN, new int[]{3}, rect);
        event(MotionEvent.ACTION_UP, new int[]{3}, rect);
    }

    private void event(int action, int[] ids, Rect... bounds) {
        eventWithFlags(action, 0, ids, bounds);
    }

    private void canceledUp(int index, int[] ids, Rect... bounds) {
        eventWithFlags(MotionEvent.ACTION_POINTER_UP | (index << MotionEvent.ACTION_POINTER_INDEX_SHIFT),
                MotionEvent.FLAG_CANCELED, ids, bounds);
    }

    private void eventWithFlags(int action, int flags, int[] ids, Rect... bounds) {
        if (ids.length != bounds.length) throw new AssertionError("contact count");
        long now = SystemClock.uptimeMillis();
        if (action == MotionEvent.ACTION_DOWN) downTime = now;
        MotionEvent.PointerProperties[] properties = new MotionEvent.PointerProperties[ids.length];
        MotionEvent.PointerCoords[] coords = new MotionEvent.PointerCoords[ids.length];
        for (int i = 0; i < ids.length; i++) {
            properties[i] = new MotionEvent.PointerProperties();
            properties[i].id = ids[i];
            properties[i].toolType = MotionEvent.TOOL_TYPE_FINGER;
            coords[i] = new MotionEvent.PointerCoords();
            coords[i].x = bounds[i].exactCenterX();
            coords[i].y = bounds[i].exactCenterY();
            coords[i].pressure = 1;
            coords[i].size = 1;
        }
        MotionEvent event = MotionEvent.obtain(downTime, now, action, ids.length, properties,
                coords, 0, 0, 1, 1, 0, 0, InputDevice.SOURCE_TOUCHSCREEN, flags);
        try {
            if (!getUiAutomation().injectInputEvent(event, true))
                throw new AssertionError("Android rejected action " + action);
            long dispatched = SystemClock.uptimeMillis();
            int masked = action & MotionEvent.ACTION_MASK;
            injectionTimings.add("action=" + action + " flags=" + flags
                    + " dispatch_ms=" + (dispatched - now));
            if (masked == MotionEvent.ACTION_DOWN || masked == MotionEvent.ACTION_POINTER_DOWN) {
                int added = (action & MotionEvent.ACTION_POINTER_INDEX_MASK)
                        >> MotionEvent.ACTION_POINTER_INDEX_SHIFT;
                pressedAt.put(ids[added], now);
            } else if (masked == MotionEvent.ACTION_POINTER_UP || masked == MotionEvent.ACTION_UP) {
                int retired = (action & MotionEvent.ACTION_POINTER_INDEX_MASK)
                        >> MotionEvent.ACTION_POINTER_INDEX_SHIFT;
                Long began = pressedAt.remove(ids[retired]);
                injectionTimings.add("up_id=" + ids[retired] + " held_ms="
                        + (began == null ? "missing-down" : Long.toString(dispatched - began)));
            } else if (masked == MotionEvent.ACTION_CANCEL) {
                pressedAt.clear();
            }
            if (masked == MotionEvent.ACTION_CANCEL || masked == MotionEvent.ACTION_UP) {
                activeIds = new int[0];
                activeBounds = new Rect[0];
            } else if (masked == MotionEvent.ACTION_POINTER_UP) {
                int retired = (action & MotionEvent.ACTION_POINTER_INDEX_MASK)
                        >> MotionEvent.ACTION_POINTER_INDEX_SHIFT;
                activeIds = new int[ids.length - 1];
                activeBounds = new Rect[bounds.length - 1];
                for (int from = 0, to = 0; from < ids.length; from++) {
                    if (from == retired) continue;
                    activeIds[to] = ids[from];
                    activeBounds[to++] = bounds[from];
                }
            } else {
                activeIds = ids.clone();
                activeBounds = bounds.clone();
            }
        } finally {
            event.recycle();
        }
        // Synchronous injection already waits for dispatch. Extra idle waits and
        // sleeps can unintentionally turn a Shift chord into its long-press action.
    }

    private Rect key(String label) {
        long deadline = SystemClock.uptimeMillis() + 3000;
        do {
            AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
            if (root != null) {
                try {
                    Rect result = findKey(root, label);
                    if (result != null) {
                        selectedKeys.add(label + ": " + result.toShortString());
                        return result;
                    }
                } finally { root.recycle(); }
            }
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Missing native key " + label);
    }

    private Rect findKey(AccessibilityNodeInfo node, String label) {
        if ("KeyView".contentEquals(node.getClassName()) && node.getText() != null
                && label.equalsIgnoreCase(node.getText().toString())) {
            Rect bounds = new Rect();
            node.getBoundsInScreen(bounds);
            if (node.isVisibleToUser() && !bounds.isEmpty()) return bounds;
        }
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo child = node.getChild(i);
            if (child == null) continue;
            try {
                Rect result = findKey(child, label);
                if (result != null) return result;
            } finally { child.recycle(); }
        }
        return null;
    }

    private AccessibilityNodeInfo findField(AccessibilityNodeInfo node) {
        if ("TextBox".contentEquals(node.getClassName())) return AccessibilityNodeInfo.obtain(node);
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo child = node.getChild(i);
            if (child == null) continue;
            try {
                AccessibilityNodeInfo found = findField(child);
                if (found != null) return found;
            } finally { child.recycle(); }
        }
        return null;
    }

    private String observedText() {
        if (observedField == null) {
            // Discover once, before contacts. Refresh below asks Android for this one
            // real node; repeated whole-tree walks can cross long-press deadlines.
            AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
            if (root == null) return null;
            try { observedField = findField(root); } finally { root.recycle(); }
            if (observedField == null) return null;
        }
        if (!observedField.refresh())
            throw new AssertionError("Observed native TextBox became unavailable");
        return observedField.getText() == null ? "" : observedField.getText().toString();
    }

    private void expect(String expected, String name) {
        expect(expected, name, 3000);
    }

    private void expect(String expected, String name, long timeoutMillis) {
        long deadline = SystemClock.uptimeMillis() + timeoutMillis;
        String actual = null;
        do {
            actual = observedText();
            if (expected.equals(actual)) {
                checks.add(name + ": " + actual);
                fieldFocusFlags.add(name + ": " + observedField.isFocused());
                return;
            }
            SystemClock.sleep(30);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError(name + ": expected " + expected + ", observed " + actual);
    }
}
