package org.ferroui.android;

import android.app.Activity;
import android.content.Context;
import android.content.res.Configuration;
import android.content.res.Resources;
import android.content.res.TypedArray;
import android.graphics.Color;
import android.graphics.Insets;
import android.graphics.Rect;
import android.graphics.drawable.ColorDrawable;
import android.hardware.display.DisplayManager;
import android.os.Build;
import android.os.LocaleList;
import android.provider.Settings;
import android.util.DisplayMetrics;
import android.view.Display;
import android.view.DisplayCutout;
import android.view.View;
import android.view.ViewConfiguration;
import android.view.Window;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.view.WindowManager;

import java.util.Locale;

/**
 * Calls into the framework of the system that return several values, or that differ between the
 * versions of the system: each answers with primitive values the native side reads in one call.
 * What the values mean is decided there.
 *
 * The reference makes these calls through the compatibility classes of AndroidX; below API 30
 * the methods do what those classes do there.
 */
final class PlatformHelper {
    private PlatformHelper() { }

    // ---- insets -------------------------------------------------------------------------------

    /** The number of values {@link #getRootInsets} answers with. */
    private static final int ROOT_INSETS_LENGTH = 8;

    /**
     * The insets of the root window of the activity, or null when the window has none yet:
     * the status bars, navigation bars and display cutout together (left, top, right, bottom),
     * the bottom of the navigation bars, the bottom of the input method, whether the input method
     * is visible, and whether the status and navigation bars are visible.
     */
    static int[] getRootInsets(Activity activity) {
        Window window = activity.getWindow();
        if (window == null) {
            return null;
        }
        return describeInsets(window.getDecorView().getRootWindowInsets());
    }

    @SuppressWarnings("deprecation")
    private static int[] describeInsets(WindowInsets insets) {
        if (insets == null) {
            return null;
        }

        int[] result = new int[ROOT_INSETS_LENGTH];
        if (Build.VERSION.SDK_INT >= 30) {
            Insets bars = insets.getInsets(
                    WindowInsets.Type.statusBars() | WindowInsets.Type.navigationBars() | WindowInsets.Type.displayCutout());
            result[0] = bars.left;
            result[1] = bars.top;
            result[2] = bars.right;
            result[3] = bars.bottom;
            result[4] = insets.getInsets(WindowInsets.Type.navigationBars()).bottom;
            result[5] = insets.getInsets(WindowInsets.Type.ime()).bottom;
            result[6] = insets.isVisible(WindowInsets.Type.ime()) ? 1 : 0;
            result[7] = insets.isVisible(WindowInsets.Type.statusBars() | WindowInsets.Type.navigationBars()) ? 1 : 0;
        } else {
            int left = insets.getStableInsetLeft();
            int top = insets.getStableInsetTop();
            int right = insets.getStableInsetRight();
            int bottom = insets.getStableInsetBottom();
            if (Build.VERSION.SDK_INT >= 28) {
                DisplayCutout cutout = insets.getDisplayCutout();
                if (cutout != null) {
                    left = Math.max(left, cutout.getSafeInsetLeft());
                    top = Math.max(top, cutout.getSafeInsetTop());
                    right = Math.max(right, cutout.getSafeInsetRight());
                    bottom = Math.max(bottom, cutout.getSafeInsetBottom());
                }
            }
            result[0] = left;
            result[1] = top;
            result[2] = right;
            result[3] = bottom;
            result[4] = insets.getStableInsetBottom();
            // The input method is what the system window insets have beyond the stable ones.
            int systemBottom = insets.getSystemWindowInsetBottom();
            result[5] = systemBottom > insets.getStableInsetBottom() ? systemBottom : 0;
            result[6] = result[5] > 0 ? 1 : 0;
            result[7] = insets.getSystemWindowInsetTop() > 0 || systemBottom > 0 ? 1 : 0;
        }
        return result;
    }

    /** Tells the native insets manager of the given number when insets are applied to the window. */
    static void setInsetsListener(Activity activity, final long handle) {
        Window window = activity.getWindow();
        if (window == null) {
            return;
        }
        window.getDecorView().setOnApplyWindowInsetsListener((view, insets) -> {
            WindowInsets applied = view.onApplyWindowInsets(insets);
            int[] described = describeInsets(applied);
            nativeApplyWindowInsets(
                    handle, described != null, described != null && described[6] != 0, described != null ? described[5] : 0);
            return applied;
        });
    }

    /** Whether the system displays every application that targets it edge to edge. */
    static boolean isDisplayEdgeToEdgeForced(Activity activity) {
        Context application = activity.getApplicationContext();
        return application != null
                && application.getApplicationInfo() != null
                && application.getApplicationInfo().targetSdkVersion >= 35
                && Build.VERSION.SDK_INT >= 35;
    }

    static void setLayoutInDisplayCutoutMode(Activity activity, boolean shortEdges) {
        Window window = activity.getWindow();
        if (Build.VERSION.SDK_INT >= 28 && window != null && window.getAttributes() != null) {
            window.getAttributes().layoutInDisplayCutoutMode = shortEdges
                    ? WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
                    : WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_DEFAULT;
        }
    }

    @SuppressWarnings("deprecation")
    static void setDecorFitsSystemWindows(Activity activity, boolean decorFitsSystemWindows) {
        Window window = activity.getWindow();
        if (window == null) {
            return;
        }
        if (Build.VERSION.SDK_INT >= 30) {
            window.setDecorFitsSystemWindows(decorFitsSystemWindows);
        } else {
            View decorView = window.getDecorView();
            int flags = View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                    | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                    | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN;
            int visibility = decorView.getSystemUiVisibility();
            decorView.setSystemUiVisibility(decorFitsSystemWindows ? visibility & ~flags : visibility | flags);
        }
    }

    @SuppressWarnings("deprecation")
    static void setTranslucentBars(Activity activity, boolean translucent) {
        Window window = activity.getWindow();
        if (window == null) {
            return;
        }
        int flags = WindowManager.LayoutParams.FLAG_TRANSLUCENT_STATUS | WindowManager.LayoutParams.FLAG_TRANSLUCENT_NAVIGATION;
        if (translucent) {
            window.addFlags(flags);
        } else {
            window.clearFlags(flags);
        }
    }

    /** The colour is ARGB. Does nothing to the colours on API 35 and later, where they cannot change. */
    @SuppressWarnings("deprecation")
    static void setSystemBarColor(Activity activity, int color) {
        Window window = activity.getWindow();
        if (window == null) {
            return;
        }
        setTranslucentBars(activity, false);
        window.addFlags(WindowManager.LayoutParams.FLAG_DRAWS_SYSTEM_BAR_BACKGROUNDS);

        // Status and navigation bar colors can't be changed at all with API level >= 35
        if (Build.VERSION.SDK_INT < 35) {
            window.setStatusBarColor(color);
            // The foreground of the navigation bar can only be changed on API 26 and newer, so
            // its background is only changed there.
            if (Build.VERSION.SDK_INT >= 26) {
                window.setNavigationBarColor(color);
            }
        }
    }

    static void setNavigationBarContrastEnforced(Activity activity, boolean enforced) {
        Window window = activity.getWindow();
        if (Build.VERSION.SDK_INT >= 29 && window != null) {
            window.setNavigationBarContrastEnforced(enforced);
        }
    }

    /** Whether the status bar is drawn for a light background; light when it cannot be asked. */
    @SuppressWarnings("deprecation")
    static boolean getAppearanceLightStatusBars(Activity activity) {
        try {
            Window window = activity.getWindow();
            View decorView = window.getDecorView();
            if (Build.VERSION.SDK_INT >= 30) {
                WindowInsetsController controller = decorView.getWindowInsetsController();
                return controller != null
                        && (controller.getSystemBarsAppearance() & WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS) != 0;
            }
            return (decorView.getSystemUiVisibility() & View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR) != 0;
        } catch (RuntimeException e) {
            return true;
        }
    }

    @SuppressWarnings("deprecation")
    static void setAppearanceLightBars(Activity activity, boolean light) {
        Window window = activity.getWindow();
        if (window == null) {
            return;
        }
        View decorView = window.getDecorView();
        if (Build.VERSION.SDK_INT >= 30) {
            WindowInsetsController controller = decorView.getWindowInsetsController();
            if (controller != null) {
                int mask = WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
                        | WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS;
                controller.setSystemBarsAppearance(light ? mask : 0, mask);
            }
        } else {
            int flags = View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR | View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
            int visibility = decorView.getSystemUiVisibility();
            decorView.setSystemUiVisibility(light ? visibility | flags : visibility & ~flags);
        }
    }

    @SuppressWarnings("deprecation")
    static void setSystemBarsVisible(Activity activity, boolean visible) {
        Window window = activity.getWindow();
        if (window == null) {
            return;
        }
        View decorView = window.getDecorView();
        if (Build.VERSION.SDK_INT >= 30) {
            WindowInsetsController controller = decorView.getWindowInsetsController();
            if (controller == null) {
                return;
            }
            int bars = WindowInsets.Type.statusBars() | WindowInsets.Type.navigationBars();
            if (visible) {
                controller.show(bars);
            } else {
                controller.hide(bars);
                controller.setSystemBarsBehavior(WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
            }
        } else {
            int flags = View.SYSTEM_UI_FLAG_FULLSCREEN
                    | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                    | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY;
            int visibility = decorView.getSystemUiVisibility();
            decorView.setSystemUiVisibility(visible ? visibility & ~flags : visibility | flags);
        }
    }

    /**
     * Applies a transparency level to the window of the activity: 0 for none, 1 for transparent,
     * 2 for blur. The caller has checked that the system has the level.
     */
    static void setWindowTransparency(Activity activity, int level) {
        Window window = activity.getWindow();
        if (level == 0) {
            if (Build.VERSION.SDK_INT >= 30) {
                activity.setTranslucent(false);
            }
            if (window != null) {
                window.setBackgroundDrawable(new ColorDrawable(Color.WHITE));
            }
        } else if (Build.VERSION.SDK_INT >= 30) {
            activity.setTranslucent(true);
            setBlurBehind(activity, level == 2 ? 120 : 0);
            if (window != null) {
                window.setBackgroundDrawable(new ColorDrawable(Color.TRANSPARENT));
            }
        }
    }

    private static void setBlurBehind(Activity activity, int radius) {
        Window window = activity.getWindow();
        if (window == null) {
            return;
        }
        if (radius == 0) {
            window.clearFlags(WindowManager.LayoutParams.FLAG_BLUR_BEHIND);
        } else {
            window.addFlags(WindowManager.LayoutParams.FLAG_BLUR_BEHIND);
        }

        if (Build.VERSION.SDK_INT >= 31 && window.getAttributes() != null) {
            WindowManager.LayoutParams attributes = window.getAttributes();
            attributes.setBlurBehindRadius(radius);
            window.setAttributes(attributes);
        }
    }

    /** The API level of the system. */
    static int sdkInt() {
        return Build.VERSION.SDK_INT;
    }

    // ---- displays -----------------------------------------------------------------------------

    /** The number of values {@link #getDisplays} answers with for each display. */
    private static final int DISPLAY_LENGTH = 9;

    private static Display[] displays(Context context) {
        DisplayManager manager = (DisplayManager) context.getSystemService(Context.DISPLAY_SERVICE);
        if (manager != null) {
            Display[] displays = manager.getDisplays();
            if (displays != null) {
                return displays;
            }
        }
        if (Build.VERSION.SDK_INT >= 30) {
            try {
                Display display = context.getDisplay();
                if (display != null) {
                    return new Display[] { display };
                }
            } catch (UnsupportedOperationException e) {
                // A context that is not associated with a display.
            }
        }
        return new Display[0];
    }

    /**
     * For each display: its id, its rotation, whether the bounds are the maximum window metrics
     * (API 30), the bounds (left, top, width, height), the scaling, and the orientation of its
     * configuration (0 when unknown).
     */
    @SuppressWarnings("deprecation")
    static float[] getDisplays(Context context) {
        Display[] displays = displays(context);
        float[] result = new float[displays.length * DISPLAY_LENGTH];
        for (int i = 0; i < displays.length; i++) {
            Display display = displays[i];
            int at = i * DISPLAY_LENGTH;
            result[at] = display.getDisplayId();
            result[at + 1] = display.getRotation();

            boolean described = false;
            if (Build.VERSION.SDK_INT >= 30) {
                try {
                    // A display context is guaranteed to be created for a display on API 30 and
                    // above, with the application context as the fallback.
                    Context displayContext = context.createDisplayContext(display);
                    if (displayContext == null) {
                        displayContext = context;
                    }
                    // The bounds of the display
                    WindowManager windowManager = displayContext.getSystemService(WindowManager.class);
                    Rect bounds = windowManager.getMaximumWindowMetrics().getBounds();
                    result[at + 2] = 1;
                    result[at + 3] = bounds.left;
                    result[at + 4] = bounds.top;
                    result[at + 5] = bounds.width();
                    result[at + 6] = bounds.height();

                    Configuration config = context.getResources() != null ? context.getResources().getConfiguration() : null;
                    result[at + 7] = config != null ? config.densityDpi / (float) DisplayMetrics.DENSITY_DEFAULT : 0;

                    Configuration displayConfig =
                            displayContext.getResources() != null ? displayContext.getResources().getConfiguration() : null;
                    result[at + 8] = displayConfig != null ? displayConfig.orientation : 0;
                    described = true;
                } catch (RuntimeException e) {
                    // The metrics of the display below.
                }
            }
            if (!described) {
                DisplayMetrics displayMetrics = new DisplayMetrics();
                display.getRealMetrics(displayMetrics);
                result[at + 2] = 0;
                result[at + 3] = 0;
                result[at + 4] = 0;
                result[at + 5] = displayMetrics.widthPixels;
                result[at + 6] = displayMetrics.heightPixels;
                result[at + 7] = displayMetrics.density;
                result[at + 8] = 0;
            }
        }
        return result;
    }

    /** The names of the displays, in the order of {@link #getDisplays}. */
    static String[] getDisplayNames(Context context) {
        Display[] displays = displays(context);
        String[] names = new String[displays.length];
        for (int i = 0; i < displays.length; i++) {
            names[i] = displays[i].getName();
        }
        return names;
    }

    /** The id of the display the view is on; -1 when it is on none. */
    static int getDisplayId(View view) {
        Display display = view.getDisplay();
        return display != null ? display.getDisplayId() : -1;
    }

    /** Registers a listener that tells the native screens of the given number when the displays change; the listener. */
    static Object registerDisplayListener(Context context, final long handle) {
        DisplayManager manager = (DisplayManager) context.getSystemService(Context.DISPLAY_SERVICE);
        if (manager == null) {
            return null;
        }
        DisplayManager.DisplayListener listener = new DisplayManager.DisplayListener() {
            @Override
            public void onDisplayAdded(int displayId) {
                nativeDisplaysChanged(handle);
            }

            @Override
            public void onDisplayChanged(int displayId) {
                nativeDisplaysChanged(handle);
            }

            @Override
            public void onDisplayRemoved(int displayId) {
                nativeDisplaysChanged(handle);
            }
        };
        manager.registerDisplayListener(listener, null);
        return listener;
    }

    static void unregisterDisplayListener(Context context, Object listener) {
        DisplayManager manager = (DisplayManager) context.getSystemService(Context.DISPLAY_SERVICE);
        if (manager != null && listener instanceof DisplayManager.DisplayListener) {
            manager.unregisterDisplayListener((DisplayManager.DisplayListener) listener);
        }
    }

    // ---- platform settings ---------------------------------------------------------------------

    /**
     * What the platform settings read of a context for the colour values: whether the night mode
     * of the configuration is "yes", whether the high contrast text is on, the number of accent
     * colours that follow (three of the system from API 31, one of the theme below, or none), and
     * the colours.
     */
    static int[] getColorValues(Context context) {
        int[] result = new int[6];
        Resources resources = context.getResources();
        Configuration configuration = resources != null ? resources.getConfiguration() : null;
        int uiMode = configuration != null
                ? configuration.uiMode & Configuration.UI_MODE_NIGHT_MASK
                : Configuration.UI_MODE_NIGHT_NO;
        result[0] = uiMode == Configuration.UI_MODE_NIGHT_YES ? 1 : 0;

        try {
            result[1] = Settings.Secure.getInt(context.getContentResolver(), "high_text_contrast_enabled", 0) == 1
                    ? 1 : 0;
        } catch (RuntimeException e) {
            result[1] = 0;
        }

        if (Build.VERSION.SDK_INT >= 31) {
            if (resources != null) {
                // See https://developer.android.com/reference/android/R.color
                result[2] = 3;
                result[3] = resources.getColor(android.R.color.system_accent1_500, context.getTheme());
                result[4] = resources.getColor(android.R.color.system_accent2_500, context.getTheme());
                result[5] = resources.getColor(android.R.color.system_accent3_500, context.getTheme());
            }
        } else {
            // See https://developer.android.com/reference/android/R.attr
            Resources.Theme theme = context.getTheme();
            TypedArray array = theme != null
                    ? theme.obtainStyledAttributes(new int[] { android.R.attr.colorAccent })
                    : null;
            if (array != null) {
                try {
                    result[2] = 1;
                    result[3] = array.getColor(0, 0);
                } finally {
                    array.recycle();
                }
            }
        }
        return result;
    }

    /**
     * What the platform settings read of the view configuration: the long press timeout, the multi
     * press timeout (API 31) or a negative number, whether the context has a view configuration,
     * its scaled double tap slop and scaled touch slop, and the density of the display metrics.
     */
    static float[] getInputConfigValues(Context context) {
        float[] result = new float[6];
        result[0] = ViewConfiguration.getLongPressTimeout();
        result[1] = Build.VERSION.SDK_INT >= 31 ? ViewConfiguration.getMultiPressTimeout() : -1;
        ViewConfiguration config = ViewConfiguration.get(context);
        if (config != null) {
            result[2] = 1;
            result[3] = config.getScaledDoubleTapSlop();
            result[4] = config.getScaledTouchSlop();
        }
        Resources resources = context.getResources();
        DisplayMetrics metrics = resources != null ? resources.getDisplayMetrics() : null;
        result[5] = metrics != null ? metrics.density : 1;
        return result;
    }

    /** The language tag of the first locale of the configuration of the context, or null. */
    static String getPreferredApplicationLanguage(Context context) {
        Resources resources = context.getResources();
        Configuration configuration = resources != null ? resources.getConfiguration() : null;
        LocaleList locales = configuration != null ? configuration.getLocales() : null;
        Locale locale = locales != null && !locales.isEmpty() ? locales.get(0) : null;
        return locale != null ? locale.toLanguageTag() : null;
    }

    private static native void nativeApplyWindowInsets(long handle, boolean hasInsets, boolean imeVisible, int imeBottom);

    private static native void nativeDisplaysChanged(long handle);
}
