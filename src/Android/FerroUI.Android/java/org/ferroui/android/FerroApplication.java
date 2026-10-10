package org.ferroui.android;

import android.app.Application;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;

/**
 * The application class of a FerroUI application: loads the native library of the application and
 * hands the start to it.
 *
 * The manifest names the library in a meta-data element of the application:
 * {@code <meta-data android:name="org.ferroui.android.library" android:value="my_app"/>} for
 * {@code libmy_app.so}.
 */
public class FerroApplication extends Application {
    /** The name of the meta-data element that names the native library. */
    public static final String LIBRARY_META_DATA = "org.ferroui.android.library";

    @Override
    public void onCreate() {
        super.onCreate();
        System.loadLibrary(libraryName());
        nativeOnCreate();
    }

    private String libraryName() {
        try {
            ApplicationInfo info =
                    getPackageManager().getApplicationInfo(getPackageName(), PackageManager.GET_META_DATA);
            String name = info.metaData != null ? info.metaData.getString(LIBRARY_META_DATA) : null;
            if (name == null || name.isEmpty()) {
                throw new IllegalStateException(
                        "The manifest does not name the native library: add the meta-data element "
                                + LIBRARY_META_DATA + " to the application.");
            }
            return name;
        } catch (PackageManager.NameNotFoundException e) {
            throw new IllegalStateException(e);
        }
    }

    private native void nativeOnCreate();
}
