package org.ferroui.android;

import android.content.ClipboardManager;

/**
 * Tells Rust that the primary clip of the clipboard changed ({@code IClipboard::content_changed};
 * see {@code platform/clipboard_impl.rs}). It reads nothing: the application asks for the data when
 * it wants it. Android reports the changes while the application is in the foreground.
 */
final class ClipboardChangeListener implements ClipboardManager.OnPrimaryClipChangedListener {
    private final long handle;

    ClipboardChangeListener(long handle) {
        this.handle = handle;
    }

    @Override
    public void onPrimaryClipChanged() {
        NativeCallback.invoke(handle, 0, 0, 0, false, false, null, null);
    }
}
