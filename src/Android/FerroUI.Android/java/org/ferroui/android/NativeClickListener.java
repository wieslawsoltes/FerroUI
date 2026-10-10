package org.ferroui.android;

import android.view.View;

/**
 * A click listener whose handler is native: what an application written in Rust sets on a view of
 * the system it created (a native control it embeds).
 */
final class NativeClickListener implements View.OnClickListener {
    private final long nativeHandle;

    NativeClickListener(long nativeHandle) {
        this.nativeHandle = nativeHandle;
    }

    @Override
    public void onClick(View view) {
        nativeOnClick(nativeHandle);
    }

    private static native void nativeOnClick(long handle);
}
