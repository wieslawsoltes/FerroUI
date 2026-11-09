using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using ScreenKeyboard.Controls.Icons;
using ScreenKeyboard.Input;
using ScreenKeyboard.Keys;
using Windows.UI;

namespace ScreenKeyboard.Controls.Primitives;

/// <summary>Caches solid color brushes.</summary>
internal static class Brushes
{
    private static readonly Dictionary<Color, SolidColorBrush> s_cache = new();

    public static SolidColorBrush Get(Color color)
    {
        if (!s_cache.TryGetValue(color, out var brush))
        {
            brush = new SolidColorBrush(color);
            s_cache[color] = brush;
        }
        return brush;
    }

    public static readonly SolidColorBrush Transparent = new(Color.FromArgb(0, 0, 0, 0));
}

/// <summary>
/// A lightweight button used inside the keyboard (a <see cref="Grid"/> because <see cref="Border"/> is sealed in WinUI). Unlike <see cref="Button"/> it never takes keyboard focus
/// (so the edited text box keeps focus), supports press-and-hold repeat and long press, and is fully themed in code.
/// </summary>
public sealed partial class TouchButton : Grid
{
    private readonly Grid _content = new();
    private IKeyboardScheduler? _scheduler;
    private IDisposable? _timer;
    private bool _pressed;
    private bool _longPressed;
    private uint? _pointerId;
    private Color _background;
    private Color _pressedBackground;

    /// <summary>Creates a button.</summary>
    public TouchButton()
    {
        Children.Add(_content);
        IsTabStop = false;
        AllowFocusOnInteraction = false;
        Background = Brushes.Transparent;
        PointerPressed += OnPointerPressed;
        PointerReleased += OnPointerReleased;
        PointerCanceled += OnPointerCanceled;
        PointerCaptureLost += OnPointerCanceled;
        PointerExited += OnPointerExited;
    }

    /// <summary>Raised on click (release inside the button).</summary>
    public event EventHandler? Click;

    /// <summary>Raised after holding the button for <see cref="LongPressDelay"/> (when no repeat is configured).</summary>
    public event EventHandler? LongPress;

    /// <summary>When set, <see cref="Click"/> repeats while the button is held.</summary>
    public TimeSpan? RepeatDelay { get; set; }

    /// <summary>Repeat interval.</summary>
    public TimeSpan RepeatInterval { get; set; } = TimeSpan.FromMilliseconds(50);

    /// <summary>Long press delay.</summary>
    public TimeSpan LongPressDelay { get; set; } = TimeSpan.FromMilliseconds(450);

    /// <summary>Optional value associated with the button.</summary>
    public object? Value { get; set; }

    /// <summary>The content grid.</summary>
    public Grid ContentGrid => _content;

    /// <summary>Sets content (replacing existing content).</summary>
    public void SetContent(UIElement element)
    {
        _content.Children.Clear();
        _content.Children.Add(element);
    }

    /// <summary>Sets an icon as content.</summary>
    public void SetIcon(KeyIcon icon, double size, Brush foreground, string? automationName = null)
    {
        SetContent(IconFactory.Create(icon, size, foreground));
        AutomationProperties.SetName(this, automationName ?? icon.ToString());
    }

    /// <summary>Sets text as content.</summary>
    public TextBlock SetText(string text, double fontSize, Brush foreground, FontFamily? fontFamily = null)
    {
        var tb = new TextBlock
        {
            Text = text,
            FontSize = fontSize,
            Foreground = foreground,
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
            TextTrimming = TextTrimming.CharacterEllipsis,
            TextWrapping = TextWrapping.NoWrap,
            IsHitTestVisible = false,
        };
        if (fontFamily is not null)
        {
            tb.FontFamily = fontFamily;
        }
        SetContent(tb);
        AutomationProperties.SetName(this, text);
        return tb;
    }

    /// <summary>Sets the normal and pressed background colors.</summary>
    public void SetColors(Color background, Color pressedBackground)
    {
        _background = background;
        _pressedBackground = pressedBackground;
        UpdateBackground();
    }

    internal void SetScheduler(IKeyboardScheduler scheduler) => _scheduler = scheduler;

    private void UpdateBackground() => Background = Brushes.Get(_pressed ? _pressedBackground : _background);

    private void OnPointerPressed(object sender, PointerRoutedEventArgs e)
    {
        e.Handled = true;
        _pointerId = e.Pointer.PointerId;
        CapturePointer(e.Pointer);
        _pressed = true;
        _longPressed = false;
        UpdateBackground();
        _timer?.Dispose();
        if (_scheduler is null)
        {
            return;
        }
        if (RepeatDelay is { } delay)
        {
            _timer = _scheduler.Schedule(delay, Repeat);
        }
        else if (LongPress is not null)
        {
            _timer = _scheduler.Schedule(LongPressDelay, () =>
            {
                if (_pressed)
                {
                    _longPressed = true;
                    LongPress?.Invoke(this, EventArgs.Empty);
                }
            });
        }
    }

    private void Repeat()
    {
        if (!_pressed || _scheduler is null)
        {
            return;
        }
        _longPressed = true; // prevent an extra click on release
        Click?.Invoke(this, EventArgs.Empty);
        _timer = _scheduler.Schedule(RepeatInterval, Repeat);
    }

    private void OnPointerReleased(object sender, PointerRoutedEventArgs e)
    {
        e.Handled = true;
        var wasPressed = _pressed;
        Reset(e.Pointer);
        if (wasPressed && !_longPressed)
        {
            var p = e.GetCurrentPoint(this).Position;
            if (p.X >= -8 && p.Y >= -8 && p.X <= ActualWidth + 8 && p.Y <= ActualHeight + 8)
            {
                Click?.Invoke(this, EventArgs.Empty);
            }
        }
    }

    private void OnPointerExited(object sender, PointerRoutedEventArgs e)
    {
        if (_pointerId is null)
        {
            _pressed = false;
            UpdateBackground();
        }
    }

    private void OnPointerCanceled(object sender, PointerRoutedEventArgs e) => Reset(e.Pointer);

    private void Reset(Pointer pointer)
    {
        ReleasePointerCapture(pointer);
        ResetCore();
    }

    private void ResetCore()
    {
        _pointerId = null;
        _pressed = false;
        _timer?.Dispose();
        _timer = null;
        UpdateBackground();
    }
}
