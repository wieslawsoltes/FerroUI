package org.ferroui.android;

import android.app.Activity;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;

/**
 * The notifications of the application (#1276): the channel of the application, a notification
 * built from what the native side passes, and their removal, through the notification manager
 * of the system. A click on a notification or on one of its actions starts the launch activity
 * of the application with the arguments of what was clicked in the extra
 * {@link #EXTRA_ARGUMENTS}.
 *
 * The methods are called on the UI thread only; a method that fails answers false.
 */
final class NotificationHelper {
    private NotificationHelper() { }

    /** The channel the notifications of the application are posted to. */
    static final String CHANNEL_ID = "org.ferroui.android.notifications";

    /** The extra of the launch intent with the arguments of what was clicked. */
    static final String EXTRA_ARGUMENTS = "org.ferroui.android.notification.arguments";

    /** The request code of the permission to post notifications. */
    private static final int PERMISSION_REQUEST = 0x4e6f;

    private static final String POST_NOTIFICATIONS = "android.permission.POST_NOTIFICATIONS";

    private static NotificationManager manager(Context context) {
        return context.getSystemService(NotificationManager.class);
    }

    /** Whether the user allows the notifications of the application. */
    static boolean canPost(Context context) {
        NotificationManager manager = manager(context);
        if (manager == null || !manager.areNotificationsEnabled()) {
            return false;
        }
        return Build.VERSION.SDK_INT < 33
            || context.checkSelfPermission(POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED;
    }

    /** Asks the user for the permission to post notifications (Android 13 and later). */
    static void requestPermission(Activity activity) {
        if (activity != null && Build.VERSION.SDK_INT >= 33
            && activity.checkSelfPermission(POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            activity.requestPermissions(new String[] { POST_NOTIFICATIONS }, PERMISSION_REQUEST);
        }
    }

    /**
     * Posts a notification. {@code progress} is the value of the progress bar from 0 to 100, or
     * -1 for none.
     */
    static boolean show(Context context, String tag, int id, String title, String body, String arguments,
                        String[] buttons, String[] buttonArguments, int progress, boolean silent,
                        boolean stays) {
        try {
            NotificationManager manager = manager(context);
            if (manager == null) {
                return false;
            }
            CharSequence label = context.getApplicationInfo().loadLabel(context.getPackageManager());
            NotificationChannel channel =
                new NotificationChannel(CHANNEL_ID, label, NotificationManager.IMPORTANCE_DEFAULT);
            manager.createNotificationChannel(channel);
            int code = (tag + "/" + id).hashCode();
            Notification.Builder builder = new Notification.Builder(context, CHANNEL_ID)
                .setSmallIcon(context.getApplicationInfo().icon)
                .setContentTitle(title)
                .setContentText(body)
                .setStyle(new Notification.BigTextStyle().bigText(body))
                .setAutoCancel(!stays)
                .setOngoing(false)
                .setContentIntent(launch(context, code, arguments));
            for (int index = 0; index < buttons.length; index++) {
                builder.addAction(new Notification.Action.Builder(
                    null, buttons[index], launch(context, code + index + 1, buttonArguments[index])).build());
            }
            if (progress >= 0) {
                builder.setProgress(100, progress, false);
            }
            if (silent && Build.VERSION.SDK_INT >= 31) {
                builder.setSilent(true);
            }
            manager.notify(tag, id, builder.build());
            return true;
        } catch (RuntimeException exception) {
            return false;
        }
    }

    private static PendingIntent launch(Context context, int code, String arguments) {
        Intent intent = context.getPackageManager().getLaunchIntentForPackage(context.getPackageName());
        if (intent == null) {
            intent = new Intent();
        }
        intent.putExtra(EXTRA_ARGUMENTS, arguments);
        intent.addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP);
        return PendingIntent.getActivity(
            context, code, intent, PendingIntent.FLAG_IMMUTABLE | PendingIntent.FLAG_UPDATE_CURRENT);
    }

    /** Removes the notification with the tag and id. */
    static void cancel(Context context, String tag, int id) {
        NotificationManager manager = manager(context);
        if (manager != null) {
            manager.cancel(tag, id);
        }
    }

    /** Removes every notification of the application. */
    static void cancelAll(Context context) {
        NotificationManager manager = manager(context);
        if (manager != null) {
            manager.cancelAll();
        }
    }
}
