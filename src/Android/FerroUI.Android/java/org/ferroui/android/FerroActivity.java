package org.ferroui.android;

import android.app.Activity;
import android.os.Build;
import android.os.Bundle;
import android.view.WindowManager;

/**
 * An activity that shows a view of the framework. The main activity of an application derives from
 * {@link FerroMainActivity}.
 *
 * The class forwards the lifecycle to the native side, which holds the logic.
 */
public class FerroActivity extends Activity {
    private long nativeHandle;

    /** Whether this is the main activity of the application. */
    protected boolean isMainActivity() {
        return false;
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        // The view is created before the activity is: the content view is set first.
        nativeHandle = nativeOnCreate(isMainActivity());
        super.onCreate(savedInstanceState);
        nativeOnCreated(nativeHandle);
    }

    @Override
    protected void onStart() {
        nativeOnStart(nativeHandle);
        super.onStart();
    }

    @Override
    protected void onStop() {
        nativeOnStop(nativeHandle);
        super.onStop();
    }

    @Override
    protected void onResume() {
        super.onResume();

        // Android only respects the cutout mode if it has been set once before the window becomes
        // visible.
        if (Build.VERSION.SDK_INT >= 28 && getWindow() != null) {
            getWindow().getAttributes().layoutInDisplayCutoutMode =
                    WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
        }

        nativeOnResume(nativeHandle);
    }

    @Override
    protected void onDestroy() {
        nativeOnDestroy(nativeHandle);
        nativeHandle = 0;
        super.onDestroy();
    }

    private native long nativeOnCreate(boolean isMainActivity);

    private static native void nativeOnCreated(long handle);

    private static native void nativeOnStart(long handle);

    private static native void nativeOnStop(long handle);

    private static native void nativeOnResume(long handle);

    private static native void nativeOnDestroy(long handle);
}
