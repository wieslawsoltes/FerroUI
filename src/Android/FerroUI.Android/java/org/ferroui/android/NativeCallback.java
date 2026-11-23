package org.ferroui.android;

/** A call into Rust for the Java class of an optional package; see {@code java_callback.rs}. */
public final class NativeCallback {
    private NativeCallback() {}

    /** Calls the callback with this number. False when there is none. */
    public static native boolean invoke(
            long handle, int kind, long number, int value, boolean first, boolean second,
            String firstText, String secondText);
}
