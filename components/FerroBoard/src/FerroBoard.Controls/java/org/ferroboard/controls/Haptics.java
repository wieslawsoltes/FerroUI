package org.ferroboard.controls;

import android.content.Context;
import android.os.VibrationEffect;
import android.os.Vibrator;

/** The actual Android vibration API. Absence/denial matches upstream's caught fallback. */
public final class Haptics {
    private Haptics() { }

    @SuppressWarnings("deprecation") // Context.VIBRATOR_SERVICE remains available at min API26.
    public static boolean vibrate(Context context, int durationMs) {
        try {
            Vibrator device = (Vibrator) context.getSystemService(Context.VIBRATOR_SERVICE);
            if (device == null || !device.hasVibrator() || durationMs <= 0) {
                return false;
            }
            device.vibrate(VibrationEffect.createOneShot(durationMs, VibrationEffect.DEFAULT_AMPLITUDE));
            return true;
        } catch (RuntimeException unavailable) {
            return false;
        }
    }
}
