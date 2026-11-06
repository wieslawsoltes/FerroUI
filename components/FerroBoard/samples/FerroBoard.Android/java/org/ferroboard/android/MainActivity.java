package org.ferroboard.android;

/** Android lifecycle only; keyboard state and UI composition remain native Rust/FerroUI. */
public final class MainActivity extends org.ferroui.android.FerroMainActivity {
    @Override
    protected void onStop() {
        nativeSave();
        super.onStop();
    }

    private static native void nativeSave();
}
