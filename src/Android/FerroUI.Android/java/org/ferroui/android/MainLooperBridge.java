package org.ferroui.android;

import android.os.Handler;
import android.os.Looper;
import android.os.MessageQueue;

/**
 * What the dispatcher of the framework needs of the main looper: a handler, the three runnables it
 * posts, and the idle handler of the message queue. The decisions are made on the native side.
 */
final class MainLooperBridge implements MessageQueue.IdleHandler {
    private final Handler handler;
    private final MessageQueue queue;
    private final Runnable signaler = MainLooperBridge::nativeSignaled;
    private final Runnable timerSignaler = MainLooperBridge::nativeTimer;
    private final Runnable wakeupSignaler = () -> { };

    /** Created on the main thread. */
    MainLooperBridge() {
        handler = new Handler(Looper.getMainLooper());
        queue = Looper.myQueue();
        queue.addIdleHandler(this);
    }

    static boolean isMainThread() {
        return Looper.getMainLooper().isCurrentThread();
    }

    /** Called from any thread. */
    void postSignal() {
        handler.post(signaler);
    }

    /** A negative delay removes the timer; zero posts it at once. */
    void updateTimer(long delayMillis) {
        handler.removeCallbacks(timerSignaler);
        if (delayMillis > 0) {
            handler.postDelayed(timerSignaler, delayMillis);
        } else if (delayMillis == 0) {
            handler.post(timerSignaler);
        }
    }

    void postWakeup() {
        handler.post(wakeupSignaler);
    }

    boolean isIdle() {
        return queue.isIdle();
    }

    @Override
    public boolean queueIdle() {
        nativeIdle();
        return true;
    }

    private static native void nativeSignaled();

    private static native void nativeTimer();

    private static native void nativeIdle();
}
