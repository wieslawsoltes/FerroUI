using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using ScreenKeyboard.Controls.Primitives;
using ScreenKeyboard.Controls.Themes;
using ScreenKeyboard.Engine;
using ScreenKeyboard.Input;
using ScreenKeyboard.Layouts;
using Windows.Foundation;

namespace ScreenKeyboard.Controls.Views;

/// <summary>
/// Renders the keys of the current <see cref="ComputedKeyboard"/> and routes pointer input to the
/// <see cref="KeyboardTouchProcessor"/>. Also draws the glide trail.
/// </summary>
internal sealed partial class KeysView : Grid
{
    private readonly Canvas _keysLayer = new() { IsHitTestVisible = false };
    private readonly Canvas _trailLayer = new() { IsHitTestVisible = false };
    private readonly Polyline _trail = new()
    {
        StrokeLineJoin = PenLineJoin.Round,
        StrokeStartLineCap = PenLineCap.Round,
        StrokeEndLineCap = PenLineCap.Round,
        IsHitTestVisible = false,
    };
    private readonly Dictionary<ComputedKey, KeyView> _views = new();
    private readonly HashSet<uint> _capturedPointers = new();
    private readonly KeyboardEngine _engine;
    private readonly KeyboardTouchProcessor _touch;
    private ComputedKeyboard? _renderedKeyboard;
    private int _renderedVersion = -1;
    private KeyboardTheme _theme = KeyboardThemes.FlorisDay;
    private IDisposable? _trailFadeTimer;
    private readonly IKeyboardScheduler _scheduler;

    public KeysView(KeyboardEngine engine, KeyboardTouchProcessor touch, IKeyboardScheduler scheduler)
    {
        _engine = engine;
        _touch = touch;
        _scheduler = scheduler;
        Background = Brushes.Transparent;
        IsTabStop = false;
        AllowFocusOnInteraction = false;
        ManipulationMode = Microsoft.UI.Xaml.Input.ManipulationModes.None;
        _trailLayer.Children.Add(_trail);
        Children.Add(_keysLayer);
        Children.Add(_trailLayer);

        SizeChanged += (_, e) => OnSizeChanged(e.NewSize);
        PointerPressed += OnPointerPressed;
        PointerMoved += OnPointerMoved;
        PointerReleased += OnPointerReleased;
        PointerCanceled += OnPointerCanceled;
        PointerCaptureLost += OnPointerCanceled;

        _touch.KeyPressed += (_, key) => SetPressed(key, true);
        _touch.KeyReleased += (_, key) => SetPressed(key, false);
        _touch.GlideTrailChanged += (_, _) => UpdateTrail();
        _touch.GlideEnded += (_, _) => ScheduleTrailFade();
    }

    /// <summary>Geometry options applied on layout.</summary>
    public KeyboardGeometryOptions Geometry { get; set; } = new();

    /// <summary>Font scale.</summary>
    public double FontScale { get; set; } = 1;

    /// <summary>Optional font family for labels.</summary>
    public FontFamily? LabelFontFamily { get; set; }

    /// <summary>Raised when a key was visually pressed (for previews).</summary>
    public event EventHandler<ComputedKey>? KeyPressedVisual;

    /// <summary>Raised when a key was visually released.</summary>
    public event EventHandler<ComputedKey>? KeyReleasedVisual;

    public void SetTheme(KeyboardTheme theme)
    {
        _theme = theme;
        _trail.Stroke = Brushes.Get(theme.GlideTrailColor);
        _trail.StrokeThickness = theme.GlideTrailThickness;
        _trail.Opacity = 0.75;
        Refresh(force: true);
    }

    private void OnSizeChanged(Size size)
    {
        if (size.Width <= 0 || size.Height <= 0)
        {
            return;
        }
        _engine.UpdateLayout(size.Width, size.Height, Geometry);
        Refresh(force: true);
    }

    /// <summary>Re-applies the layout (e.g. after geometry option changes).</summary>
    public void Relayout()
    {
        if (ActualWidth > 0 && ActualHeight > 0)
        {
            _engine.UpdateLayout(ActualWidth, ActualHeight, Geometry);
        }
        Refresh(force: true);
    }

    /// <summary>Synchronizes key views with the engine's keyboard.</summary>
    public void Refresh(bool force = false)
    {
        var keyboard = _engine.Keyboard;
        if (ActualWidth > 0 && ActualHeight > 0 && (keyboard.Width != ActualWidth || keyboard.Height != ActualHeight))
        {
            _engine.UpdateLayout(ActualWidth, ActualHeight, Geometry);
        }
        if (!ReferenceEquals(keyboard, _renderedKeyboard))
        {
            _keysLayer.Children.Clear();
            _views.Clear();
            foreach (var key in keyboard.Keys)
            {
                var view = new KeyView(key);
                _views[key] = view;
                _keysLayer.Children.Add(view);
            }
            _renderedKeyboard = keyboard;
            force = true;
        }
        if (!force && _renderedVersion == keyboard.Version)
        {
            return;
        }
        _renderedVersion = keyboard.Version;
        foreach (var view in _views.Values)
        {
            view.Update(_theme, _engine.Mode, _engine.ShiftState, FontScale, LabelFontFamily);
        }
    }

    private void SetPressed(ComputedKey key, bool pressed)
    {
        if (_views.TryGetValue(key, out var view))
        {
            view.IsPressed = pressed;
            view.Update(_theme, _engine.Mode, _engine.ShiftState, FontScale, LabelFontFamily);
        }
        if (pressed)
        {
            KeyPressedVisual?.Invoke(this, key);
        }
        else
        {
            KeyReleasedVisual?.Invoke(this, key);
        }
    }

    // ------------------------------------------------------------------ pointer input

    private void OnPointerPressed(object sender, PointerRoutedEventArgs e)
    {
        e.Handled = true;
        if (CapturePointer(e.Pointer))
        {
            _capturedPointers.Add(e.Pointer.PointerId);
        }
        var p = e.GetCurrentPoint(this).Position;
        _trailFadeTimer?.Dispose();
        _touch.PointerDown(e.Pointer.PointerId, p.X, p.Y);
    }

    private void OnPointerMoved(object sender, PointerRoutedEventArgs e)
    {
        if (!_capturedPointers.Contains(e.Pointer.PointerId) && !e.Pointer.IsInContact)
        {
            return;
        }
        e.Handled = true;
        var points = e.GetIntermediatePoints(this);
        if (points is { Count: > 1 })
        {
            // Intermediate points are ordered newest first.
            for (var i = points.Count - 1; i >= 0; i--)
            {
                var ip = points[i].Position;
                _touch.PointerMove(e.Pointer.PointerId, ip.X, ip.Y);
            }
        }
        else
        {
            var p = e.GetCurrentPoint(this).Position;
            _touch.PointerMove(e.Pointer.PointerId, p.X, p.Y);
        }
    }

    private void OnPointerReleased(object sender, PointerRoutedEventArgs e)
    {
        e.Handled = true;
        var p = e.GetCurrentPoint(this).Position;
        _capturedPointers.Remove(e.Pointer.PointerId);
        ReleasePointerCapture(e.Pointer);
        _touch.PointerUp(e.Pointer.PointerId, p.X, p.Y);
    }

    private void OnPointerCanceled(object sender, PointerRoutedEventArgs e)
    {
        if (_capturedPointers.Remove(e.Pointer.PointerId))
        {
            _touch.PointerCancel(e.Pointer.PointerId);
        }
    }

    // ------------------------------------------------------------------ glide trail

    private void UpdateTrail()
    {
        var trail = _touch.GlideTrail;
        var points = new PointCollection();
        var start = Math.Max(0, trail.Count - 80);
        for (var i = start; i < trail.Count; i++)
        {
            points.Add(new Point(trail[i].X, trail[i].Y));
        }
        _trail.Points = points;
        _trail.Opacity = 0.75;
        _trail.Visibility = trail.Count > 1 ? Visibility.Visible : Visibility.Collapsed;
    }

    private void ScheduleTrailFade()
    {
        _trailFadeTimer?.Dispose();
        var duration = Math.Max(0, _engine.Settings.GlideTrailDuration);
        const int steps = 6;
        var step = 0;
        void Fade()
        {
            step++;
            if (step >= steps)
            {
                _touch.ClearGlideTrail();
                _trail.Visibility = Visibility.Collapsed;
                return;
            }
            _trail.Opacity = 0.75 * (1 - step / (double)steps);
            _trailFadeTimer = _scheduler.Schedule(TimeSpan.FromMilliseconds(Math.Max(1, duration / steps)), Fade);
        }
        _trailFadeTimer = _scheduler.Schedule(TimeSpan.FromMilliseconds(Math.Max(1, duration / steps)), Fade);
    }
}
