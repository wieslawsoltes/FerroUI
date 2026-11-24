package org.ferroui.android;

import android.content.ClipData;
import android.view.DragEvent;
import android.view.View;

/**
 * The drag listener of the surface view of a top-level: forwards the drag events of the system to
 * the native side, which raises them on the top-level, and starts the drags of the application.
 *
 * <p>The listener is the local state of the drags it starts, which is how the end of a drag tells
 * the drags of this view from the drags of other views and applications.
 */
final class FerroDragListener implements View.OnDragListener {
    private long nativeHandle;

    private FerroDragListener(long nativeHandle) {
        this.nativeHandle = nativeHandle;
    }

    /** Sets a listener that calls back with {@code nativeHandle} on {@code view}. */
    static FerroDragListener attach(View view, long nativeHandle) {
        FerroDragListener listener = new FerroDragListener(nativeHandle);
        view.setOnDragListener(listener);
        return listener;
    }

    /** Removes the listener from {@code view}; the native side is not called afterwards. */
    void detach(View view) {
        nativeHandle = 0;
        view.setOnDragListener(null);
    }

    /**
     * Starts a drag of {@code clipData} from {@code view}, readable by other applications, with a
     * shadow of the view. Returns whether the system started it.
     */
    boolean startDrag(View view, ClipData clipData) {
        int flags = View.DRAG_FLAG_GLOBAL | View.DRAG_FLAG_GLOBAL_URI_READ;
        return view.startDragAndDrop(clipData, new View.DragShadowBuilder(view), this, flags);
    }

    @Override
    public boolean onDrag(View view, DragEvent event) {
        if (nativeHandle == 0 || event == null) {
            return false;
        }
        int action = event.getAction();
        if (action == DragEvent.ACTION_DRAG_ENDED && event.getLocalState() != this) {
            return false;
        }
        return nativeOnDrag(
                nativeHandle,
                action,
                event.getX(),
                event.getY(),
                event.getResult(),
                event.getLocalState() == this,
                event);
    }

    private static native boolean nativeOnDrag(
            long handle, int action, float x, float y, boolean result, boolean isOwnDrag, DragEvent event);
}
