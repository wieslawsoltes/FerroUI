package org.ferroui.android;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.view.WindowManager;
import android.window.OnBackInvokedCallback;
import android.window.OnBackInvokedDispatcher;

/**
 * An activity that shows a view of the framework. The main activity of an application derives from
 * {@link FerroMainActivity}.
 *
 * The class forwards the lifecycle, the intents, the results and the back button to the native
 * side, which holds the logic.
 */
public class FerroActivity extends Activity {
    private long nativeHandle;
    private OnBackInvokedCallback backCallback;

    /** Whether this is the main activity of the application. */
    protected boolean isMainActivity() {
        return false;
    }

    private static Uri data(Intent intent) {
        return intent != null ? intent.getData() : null;
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        // The view is created before the activity is: the content view is set first.
        nativeHandle = nativeOnCreate(isMainActivity());
        super.onCreate(savedInstanceState);
        nativeOnCreated(nativeHandle);
        nativeHandleIntent(nativeHandle, data(getIntent()));
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);

        nativeHandleIntent(nativeHandle, data(intent));
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

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);

        nativeOnActivityResult(nativeHandle, requestCode, resultCode, data);
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);

        nativeOnRequestPermissionsResult(nativeHandle, requestCode, permissions, grantResults);
    }

    // ---- the back button ----------------------------------------------------------------------

    @Override
    @SuppressWarnings("deprecation")
    public void onBackPressed() {
        if (Build.VERSION.SDK_INT >= 33) {
            // A system that does not call the back callback of the window (an application that
            // opted out of it) calls this: the callback handles it.
            handleOnBackPressed();
            return;
        }

        if (!nativeOnBackPressed(nativeHandle)) {
            super.onBackPressed();
        }
    }

    /** Registers the back callback of the activity with its window (API 33). */
    void addBackCallback() {
        if (Build.VERSION.SDK_INT >= 33 && backCallback == null) {
            backCallback = this::handleOnBackPressed;
            getOnBackInvokedDispatcher()
                    .registerOnBackInvokedCallback(OnBackInvokedDispatcher.PRIORITY_DEFAULT, backCallback);
        }
    }

    /** Removes the back callback of the activity from its window. */
    void removeBackCallback() {
        if (Build.VERSION.SDK_INT >= 33 && backCallback != null) {
            getOnBackInvokedDispatcher().unregisterOnBackInvokedCallback(backCallback);
            backCallback = null;
        }
    }

    /**
     * The back callback: the native side raises the back request, and answers whether nobody
     * handled it, in which case the activity does what the system does by default.
     */
    @SuppressWarnings("deprecation")
    private void handleOnBackPressed() {
        if (nativeHandleOnBackPressed(nativeHandle)) {
            super.onBackPressed();
        }
    }

    private native long nativeOnCreate(boolean isMainActivity);

    private static native void nativeOnCreated(long handle);

    private static native void nativeHandleIntent(long handle, Uri data);

    private static native void nativeOnStart(long handle);

    private static native void nativeOnStop(long handle);

    private static native void nativeOnResume(long handle);

    private static native void nativeOnDestroy(long handle);

    private static native void nativeOnActivityResult(long handle, int requestCode, int resultCode, Intent data);

    private static native void nativeOnRequestPermissionsResult(
            long handle, int requestCode, String[] permissions, int[] grantResults);

    private static native boolean nativeOnBackPressed(long handle);

    private static native boolean nativeHandleOnBackPressed(long handle);
}
