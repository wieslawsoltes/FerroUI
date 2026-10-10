package org.ferroui.android;

import android.content.Context;
import android.graphics.BlendMode;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.PixelFormat;
import android.graphics.PorterDuff;
import android.graphics.Rect;
import android.os.Build;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;

/**
 * The surface the framework renders to. The callbacks of the surface holder are forwarded to the
 * native side with what it caches of the surface: the surface, the size of its frame and the
 * density of the display.
 */
final class FerroSurfaceView extends SurfaceView implements SurfaceHolder.Callback2 {
    private long nativeHandle;
    private Paint clearPaint;

    FerroSurfaceView(Context context, long nativeHandle, boolean placeOnTop) {
        super(context);
        this.nativeHandle = nativeHandle;

        SurfaceHolder holder = getHolder();
        if (holder == null) {
            throw new IllegalStateException(
                    "SurfaceView.getHolder was not expected to be null during the initialization of the surface view.");
        }

        holder.addCallback(this);
        holder.setFormat(PixelFormat.TRANSPARENT);
        if (placeOnTop) {
            setZOrderOnTop(true);
        }
    }

    void dispose() {
        nativeHandle = 0;
        SurfaceHolder holder = getHolder();
        if (holder != null) {
            holder.removeCallback(this);
        }
    }

    private float density() {
        return getResources() != null && getResources().getDisplayMetrics() != null
                ? getResources().getDisplayMetrics().density
                : 1;
    }

    @Override
    protected void dispatchDraw(Canvas canvas) {
        // Workaround for a screen that remains gray after the splash screen.
        // The base class should punch a hole into the canvas so the surface
        // can be seen below, but it does not.
        if (Build.VERSION.SDK_INT >= 29) {
            if (clearPaint == null) {
                clearPaint = new Paint();
                clearPaint.setColor(0);
                clearPaint.setBlendMode(BlendMode.CLEAR);
            }
            canvas.drawRect(0, 0, getWidth(), getHeight(), clearPaint);
        } else {
            // Android 9 did this
            canvas.drawColor(Color.TRANSPARENT, PorterDuff.Mode.CLEAR);
        }

        super.dispatchDraw(canvas);
    }

    @Override
    protected void onFocusChanged(boolean gainFocus, int direction, Rect previouslyFocusedRect) {
        super.onFocusChanged(gainFocus, direction, previouslyFocusedRect);
        nativeFocusChanged(nativeHandle, gainFocus);
    }

    @Override
    public void surfaceCreated(SurfaceHolder holder) {
        Rect frame = holder.getSurfaceFrame();
        nativeSurfaceCreated(
                nativeHandle,
                holder.getSurface(),
                frame != null,
                frame != null ? frame.width() : 0,
                frame != null ? frame.height() : 0,
                density());
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        Rect frame = holder.getSurfaceFrame();
        nativeSurfaceChanged(
                nativeHandle,
                holder.getSurface(),
                frame != null,
                frame != null ? frame.width() : 0,
                frame != null ? frame.height() : 0,
                density(),
                format,
                width,
                height);
    }

    @Override
    public void surfaceDestroyed(SurfaceHolder holder) {
        nativeSurfaceDestroyed(nativeHandle);
    }

    @Override
    public void surfaceRedrawNeeded(SurfaceHolder holder) {
        nativeSurfaceRedrawNeeded(nativeHandle);
    }

    @Override
    public void surfaceRedrawNeededAsync(SurfaceHolder holder, Runnable drawingFinished) {
        nativeSurfaceRedrawNeededAsync(nativeHandle, drawingFinished);
    }

    private static native void nativeSurfaceCreated(
            long handle, Surface surface, boolean hasFrame, int frameWidth, int frameHeight, float density);

    private static native void nativeSurfaceChanged(
            long handle,
            Surface surface,
            boolean hasFrame,
            int frameWidth,
            int frameHeight,
            float density,
            int format,
            int width,
            int height);

    private static native void nativeSurfaceDestroyed(long handle);

    private static native void nativeSurfaceRedrawNeeded(long handle);

    private static native void nativeSurfaceRedrawNeededAsync(long handle, Runnable drawingFinished);

    private static native void nativeFocusChanged(long handle, boolean hasFocus);
}
