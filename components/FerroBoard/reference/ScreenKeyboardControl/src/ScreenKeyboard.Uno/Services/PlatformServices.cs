using Microsoft.UI.Dispatching;
using ScreenKeyboard.Clipboard;
using ScreenKeyboard.Input;
using ScreenKeyboard.Settings;
using Windows.ApplicationModel.DataTransfer;

namespace ScreenKeyboard.Controls.Services;

/// <summary>Schedules keyboard timers on the UI thread using <see cref="DispatcherQueueTimer"/>.</summary>
public sealed class DispatcherKeyboardScheduler : IKeyboardScheduler
{
    private readonly DispatcherQueue _queue;

    /// <summary>Creates a scheduler for the current thread's dispatcher queue.</summary>
    public DispatcherKeyboardScheduler(DispatcherQueue? queue = null)
    {
        _queue = queue ?? DispatcherQueue.GetForCurrentThread();
    }

    /// <inheritdoc />
    public IDisposable Schedule(TimeSpan delay, Action action)
    {
        var timer = _queue.CreateTimer();
        timer.Interval = delay <= TimeSpan.Zero ? TimeSpan.FromMilliseconds(1) : delay;
        timer.IsRepeating = false;
        var handle = new TimerHandle(timer);
        timer.Tick += (_, _) =>
        {
            timer.Stop();
            if (!handle.IsDisposed)
            {
                action();
            }
        };
        timer.Start();
        return handle;
    }

    private sealed class TimerHandle(DispatcherQueueTimer timer) : IDisposable
    {
        public bool IsDisposed { get; private set; }

        public void Dispose()
        {
            IsDisposed = true;
            timer.Stop();
        }
    }
}

/// <summary>System clipboard implementation based on <see cref="Windows.ApplicationModel.DataTransfer.Clipboard"/>.</summary>
public sealed class SystemClipboardService : IClipboardService
{
    /// <inheritdoc />
    public async Task<string?> GetTextAsync()
    {
        try
        {
            var content = Windows.ApplicationModel.DataTransfer.Clipboard.GetContent();
            if (content is not null && content.Contains(StandardDataFormats.Text))
            {
                return await content.GetTextAsync();
            }
        }
        catch (Exception)
        {
            // Clipboard access can be denied (e.g. browser permissions); fall back to history.
        }
        return null;
    }

    /// <inheritdoc />
    public Task SetTextAsync(string text)
    {
        try
        {
            var package = new DataPackage();
            package.SetText(text);
            Windows.ApplicationModel.DataTransfer.Clipboard.SetContent(package);
        }
        catch (Exception)
        {
            // Ignore: the keyboard history still records the text.
        }
        return Task.CompletedTask;
    }
}

/// <summary>Haptic feedback helper (vibration on Android and iOS, no-op elsewhere).</summary>
public static class KeyboardHaptics
{
    /// <summary>Vibrates for a short time according to the feedback kind.</summary>
    public static void Perform(FeedbackKind kind, int durationMs)
    {
#if __ANDROID__ || __IOS__
        try
        {
            var duration = kind == FeedbackKind.LongPress ? durationMs * 2 : durationMs;
            Windows.Phone.Devices.Notification.VibrationDevice.GetDefault()?.Vibrate(TimeSpan.FromMilliseconds(duration));
        }
        catch (Exception)
        {
            // Vibration may be unavailable or not permitted.
        }
#endif
    }
}
