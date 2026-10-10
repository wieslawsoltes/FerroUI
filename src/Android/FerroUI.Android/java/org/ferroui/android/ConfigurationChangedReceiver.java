package org.ferroui.android;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.os.Build;

/**
 * Tells the platform settings that the configuration of the device changed (the night mode, the
 * locale, the density): the native side reads the values again.
 */
final class ConfigurationChangedReceiver extends BroadcastReceiver {
    /** Registers a receiver of the configuration changes with the application context. */
    static void register(Context context) {
        IntentFilter filter = new IntentFilter(Intent.ACTION_CONFIGURATION_CHANGED);
        ConfigurationChangedReceiver receiver = new ConfigurationChangedReceiver();
        if (Build.VERSION.SDK_INT >= 33) {
            context.registerReceiver(receiver, filter, Context.RECEIVER_NOT_EXPORTED);
        } else {
            // The broadcast is one only the system may send.
            context.registerReceiver(receiver, filter);
        }
    }

    @Override
    public void onReceive(Context context, Intent intent) {
        if (context == null) {
            return;
        }

        nativeOnReceive();
    }

    private static native void nativeOnReceive();
}
