package org.ferroui.android;

import android.content.Context;
import android.content.res.Configuration;
import android.graphics.Color;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewTreeObserver;
import android.widget.FrameLayout;

/**
 * Shows content of the framework in the view hierarchy: a frame layout over the surface view the
 * framework renders to.
 *
 * The class forwards what the system tells a view to the native side, which holds the logic. An
 * application that embeds the framework in a layout of its own creates the view with its context;
 * an activity of the framework ({@link FerroActivity}) creates it by itself.
 */
public class FerroView extends FrameLayout implements ViewTreeObserver.OnGlobalLayoutListener {
    // What the native side answers a motion event with: the result of the helper (none, false,
    // true) and whether the base class is called too.
    private static final int RESULT_MASK = 3;
    private static final int RESULT_NONE = 0;
    private static final int RESULT_TRUE = 2;
    private static final int CALL_BASE = 4;

    private long nativeHandle;

    public FerroView(Context context) {
        super(context);
        nativeHandle = nativeCreate(context);
        setBackgroundColor(Color.TRANSPARENT);
        sendConfigurationChanged(getResources().getConfiguration());
    }

    long getNativeHandle() {
        return nativeHandle;
    }

    /** Releases the native side of the view. The view cannot be used afterwards. */
    public void dispose() {
        long handle = nativeHandle;
        nativeHandle = 0;
        if (handle != 0) {
            nativeDispose(handle);
        }
    }

    void addGlobalLayoutListener() {
        getViewTreeObserver().addOnGlobalLayoutListener(this);
    }

    void removeGlobalLayoutListener() {
        getViewTreeObserver().removeOnGlobalLayoutListener(this);
    }

    @Override
    public void onGlobalLayout() {
        nativeGlobalLayout(nativeHandle);
    }

    @Override
    protected void onAttachedToWindow() {
        sendConfigurationChanged(getResources().getConfiguration());

        super.onAttachedToWindow();
    }

    @Override
    public void onVisibilityAggregated(boolean isVisible) {
        super.onVisibilityAggregated(isVisible);
        nativeVisibilityChanged(nativeHandle, isVisible);
    }

    @Override
    protected void onVisibilityChanged(View changedView, int visibility) {
        super.onVisibilityChanged(changedView, visibility);
        // Called by the constructor of the base class as well, before the native side exists.
        nativeVisibilityChanged(nativeHandle, visibility == VISIBLE);
    }

    @Override
    protected void onConfigurationChanged(Configuration newConfig) {
        super.onConfigurationChanged(newConfig);
        sendConfigurationChanged(newConfig != null ? newConfig : getResources().getConfiguration());
    }

    private void sendConfigurationChanged(Configuration config) {
        boolean night = config != null
                && (config.uiMode & Configuration.UI_MODE_NIGHT_YES) == Configuration.UI_MODE_NIGHT_YES;
        nativeConfigurationChanged(nativeHandle, config != null, night);
    }

    @Override
    protected boolean dispatchHoverEvent(MotionEvent e) {
        if (e == null) {
            return false;
        }
        int answer = forwardMotionEvent(e);
        // The accessibility helper decides whether the base class is called: without it, never.
        return (answer & RESULT_MASK) == RESULT_TRUE;
    }

    @Override
    protected boolean dispatchGenericPointerEvent(MotionEvent e) {
        if (e == null) {
            return super.dispatchGenericPointerEvent(e);
        }
        int answer = forwardMotionEvent(e);
        boolean baseResult = (answer & CALL_BASE) != 0 && super.dispatchGenericPointerEvent(e);
        return (answer & RESULT_MASK) == RESULT_NONE ? baseResult : (answer & RESULT_MASK) == RESULT_TRUE;
    }

    @Override
    public boolean dispatchTouchEvent(MotionEvent e) {
        if (e == null) {
            return super.dispatchTouchEvent(e);
        }
        int answer = forwardMotionEvent(e);
        boolean baseResult = (answer & CALL_BASE) != 0 && super.dispatchTouchEvent(e);

        if ((answer & RESULT_MASK) == RESULT_TRUE) {
            // Request focus for this view
            requestFocus();
        }

        return (answer & RESULT_MASK) == RESULT_NONE ? baseResult : (answer & RESULT_MASK) == RESULT_TRUE;
    }

    /**
     * Copies what the native side reads of a motion event: the event is only valid during the call.
     * Per pointer: id and tool type, and x, y, pressure and orientation; for a move, the same four
     * values of every pointer at every position of the history.
     */
    private int forwardMotionEvent(MotionEvent e) {
        int actionMasked = e.getActionMasked();
        int pointerCount = e.getPointerCount();
        int historySize = actionMasked == MotionEvent.ACTION_MOVE ? e.getHistorySize() : 0;

        int[] pointers = new int[pointerCount * 2];
        float[] values = new float[pointerCount * 4 * (1 + historySize)];
        int at = 0;
        for (int index = 0; index < pointerCount; index++) {
            pointers[index * 2] = e.getPointerId(index);
            pointers[index * 2 + 1] = e.getToolType(index);
            values[at++] = e.getX(index);
            values[at++] = e.getY(index);
            values[at++] = e.getPressure(index);
            values[at++] = e.getOrientation(index);
        }
        for (int index = 0; index < pointerCount; index++) {
            for (int pos = 0; pos < historySize; pos++) {
                values[at++] = e.getHistoricalX(index, pos);
                values[at++] = e.getHistoricalY(index, pos);
                values[at++] = e.getHistoricalPressure(index, pos);
                values[at++] = e.getHistoricalOrientation(index, pos);
            }
        }

        return nativeMotionEvent(
                nativeHandle,
                e.getEventTime(),
                actionMasked,
                e.getActionIndex(),
                e.getMetaState(),
                e.getButtonState(),
                e.getActionButton(),
                historySize,
                pointers,
                values,
                e.getAxisValue(MotionEvent.AXIS_HSCROLL),
                e.getAxisValue(MotionEvent.AXIS_VSCROLL));
    }

    private native long nativeCreate(Context context);

    private static native void nativeDispose(long handle);

    private static native void nativeGlobalLayout(long handle);

    private static native void nativeVisibilityChanged(long handle, boolean isVisible);

    private static native void nativeConfigurationChanged(long handle, boolean hasConfiguration, boolean night);

    private static native int nativeMotionEvent(
            long handle,
            long eventTime,
            int actionMasked,
            int actionIndex,
            int metaState,
            int buttonState,
            int actionButton,
            int historySize,
            int[] pointers,
            float[] values,
            float hscroll,
            float vscroll);
}
