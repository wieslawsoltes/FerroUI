using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Markup;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Animation;

namespace ScreenKeyboard.Controls;

/// <summary>When the <see cref="ScreenKeyboardHost"/> shows its keyboard.</summary>
public enum KeyboardVisibilityMode
{
    /// <summary>Shown when a text input gets focus, hidden when focus leaves text inputs.</summary>
    Auto,
    /// <summary>Always visible.</summary>
    AlwaysVisible,
    /// <summary>Controlled by <see cref="ScreenKeyboardHost.IsKeyboardOpen"/> only.</summary>
    Manual,
}

/// <summary>
/// Hosts application content and docks an <see cref="OnScreenKeyboard"/> at the bottom. The keyboard slides in
/// when a <see cref="TextBox"/> or <see cref="PasswordBox"/> inside the host gets focus, and the content is
/// resized so the focused field stays visible.
/// </summary>
/// <example>
/// <code language="xml"><![CDATA[
/// <sk:ScreenKeyboardHost>
///   <StackPanel>
///     <TextBox PlaceholderText="Type here" />
///   </StackPanel>
/// </sk:ScreenKeyboardHost>
/// ]]></code>
/// </example>
[ContentProperty(Name = nameof(Content))]
public sealed partial class ScreenKeyboardHost : Grid
{
    /// <summary>Identifies the <see cref="Content"/> property.</summary>
    public static readonly DependencyProperty ContentProperty = DependencyProperty.Register(
        nameof(Content), typeof(UIElement), typeof(ScreenKeyboardHost), new PropertyMetadata(null, (d, e) => ((ScreenKeyboardHost)d).OnContentChanged(e.OldValue as UIElement, e.NewValue as UIElement)));

    /// <summary>Identifies the <see cref="IsKeyboardOpen"/> property.</summary>
    public static readonly DependencyProperty IsKeyboardOpenProperty = DependencyProperty.Register(
        nameof(IsKeyboardOpen), typeof(bool), typeof(ScreenKeyboardHost), new PropertyMetadata(false, (d, e) => ((ScreenKeyboardHost)d).OnIsOpenChanged((bool)e.NewValue)));

    /// <summary>Identifies the <see cref="VisibilityMode"/> property.</summary>
    public static readonly DependencyProperty VisibilityModeProperty = DependencyProperty.Register(
        nameof(VisibilityMode), typeof(KeyboardVisibilityMode), typeof(ScreenKeyboardHost), new PropertyMetadata(KeyboardVisibilityMode.Auto, (d, _) => ((ScreenKeyboardHost)d).OnVisibilityModeChanged()));

    /// <summary>Identifies the <see cref="IsAnimated"/> property.</summary>
    public static readonly DependencyProperty IsAnimatedProperty = DependencyProperty.Register(
        nameof(IsAnimated), typeof(bool), typeof(ScreenKeyboardHost), new PropertyMetadata(true));

    private readonly Grid _contentHost = new();
    private readonly Canvas _floatLayer = new();
    private readonly Border _floatFrame = new();
    private readonly Grid _floatGrid = new();
    private readonly Border _dragHandle = new();
    private bool _isFloating;
    private Windows.Foundation.Point? _floatPosition;
    private Windows.Foundation.Point _dragStart;
    private Windows.Foundation.Point _dragOrigin;
    private uint? _dragPointer;
    private readonly TranslateTransform _translate = new();
    private Storyboard? _storyboard;
    private UIElement? _focusedInput;
    private bool _pendingClose;

    /// <summary>Creates a host with a new keyboard.</summary>
    public ScreenKeyboardHost()
    {
        RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        Keyboard = new OnScreenKeyboard { AutoAttach = false };
        Keyboard.RenderTransform = _translate;
        Keyboard.Visibility = Visibility.Collapsed;
        Grid.SetRow(_contentHost, 0);
        Grid.SetRow(Keyboard, 1);
        Children.Add(_contentHost);
        Children.Add(Keyboard);

        Keyboard.HideRequested += (_, _) => IsKeyboardOpen = VisibilityMode == KeyboardVisibilityMode.AlwaysVisible;
        Grid.SetRowSpan(_floatLayer, 2);
        _floatGrid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        _floatGrid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        _dragHandle.Height = 22;
        _dragHandle.Child = new Border
        {
            Width = 44,
            Height = 5,
            CornerRadius = new CornerRadius(2.5),
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
            IsHitTestVisible = false,
        };
        _dragHandle.PointerPressed += OnDragPressed;
        _dragHandle.PointerMoved += OnDragMoved;
        _dragHandle.PointerReleased += OnDragReleased;
        _dragHandle.PointerCanceled += OnDragReleased;
        _floatGrid.Children.Add(_dragHandle);
        _floatFrame.Child = _floatGrid;
        _floatFrame.CornerRadius = new CornerRadius(14);
        _floatFrame.BorderThickness = new Thickness(1);
        _floatLayer.Children.Add(_floatFrame);
        Primitives.FocusNeutral.Apply(_floatFrame);
        Children.Add(_floatLayer);
        _floatLayer.Visibility = Visibility.Collapsed;
        Keyboard.Settings.PropertyChanged += (_, e) =>
        {
            if (e.PropertyName is nameof(Settings.KeyboardSettings.Floating) or nameof(Settings.KeyboardSettings.FloatingWidthRatio))
            {
                ApplyFloating();
            }
        };
        SizeChanged += (_, _) =>
        {
            if (_isFloating)
            {
                PositionFloatingFrame();
            }
        };
        Loaded += (_, _) => ApplyFloating();
        GotFocus += OnGotFocus;
        LostFocus += OnLostFocus;
        AddHandler(PointerPressedEvent, new PointerEventHandler(OnPointerPressed), handledEventsToo: true);
    }

    /// <summary>The docked keyboard.</summary>
    public OnScreenKeyboard Keyboard { get; }

    /// <summary>The application content.</summary>
    public UIElement? Content
    {
        get => (UIElement?)GetValue(ContentProperty);
        set => SetValue(ContentProperty, value);
    }

    /// <summary>Whether the keyboard is shown.</summary>
    public bool IsKeyboardOpen
    {
        get => (bool)GetValue(IsKeyboardOpenProperty);
        set => SetValue(IsKeyboardOpenProperty, value);
    }

    /// <summary>When the keyboard is shown.</summary>
    public KeyboardVisibilityMode VisibilityMode
    {
        get => (KeyboardVisibilityMode)GetValue(VisibilityModeProperty);
        set => SetValue(VisibilityModeProperty, value);
    }

    /// <summary>Whether showing and hiding is animated.</summary>
    public bool IsAnimated
    {
        get => (bool)GetValue(IsAnimatedProperty);
        set => SetValue(IsAnimatedProperty, value);
    }

    /// <summary>Raised when <see cref="IsKeyboardOpen"/> changed.</summary>
    public event EventHandler? KeyboardOpenChanged;

    /// <summary>Whether the keyboard currently floats above the content.</summary>
    public bool IsFloating => _isFloating;

    private void ApplyFloating()
    {
        var floating = Keyboard.Settings.Floating;
        if (floating == _isFloating && (!floating || _floatGrid.Children.Contains(Keyboard)))
        {
            if (floating)
            {
                PositionFloatingFrame();
            }
            return;
        }
        _isFloating = floating;
        _storyboard?.Stop();
        _translate.Y = 0;
        if (floating)
        {
            Children.Remove(Keyboard);
            Grid.SetRow(Keyboard, 1);
            _floatGrid.Children.Add(Keyboard);
            var theme = Keyboard.EffectiveTheme;
            _floatFrame.Background = Primitives.Brushes.Get(theme.Background);
            _floatFrame.BorderBrush = Primitives.Brushes.Get(theme.PopupBorderColor);
            ((Border)_dragHandle.Child).Background = Primitives.Brushes.Get(theme.KeyHintForeground);
            _dragHandle.Background = Primitives.Brushes.Get(theme.SmartbarBackground);
            Keyboard.Visibility = Visibility.Visible;
            _floatLayer.Visibility = IsKeyboardOpen ? Visibility.Visible : Visibility.Collapsed;
            PositionFloatingFrame();
        }
        else
        {
            _floatGrid.Children.Remove(Keyboard);
            Grid.SetRow(Keyboard, 1);
            Children.Insert(1, Keyboard);
            _floatLayer.Visibility = Visibility.Collapsed;
            Keyboard.Width = double.NaN;
            Keyboard.Visibility = IsKeyboardOpen ? Visibility.Visible : Visibility.Collapsed;
        }
    }

    private void PositionFloatingFrame()
    {
        if (ActualWidth <= 0)
        {
            return;
        }
        var width = Math.Max(320, Math.Min(ActualWidth, ActualWidth * Keyboard.Settings.FloatingWidthRatio));
        Keyboard.Width = width;
        _floatFrame.Width = width + 2;
        _floatFrame.UpdateLayout();
        var height = _floatFrame.ActualHeight > 0 ? _floatFrame.ActualHeight : 320;
        var position = _floatPosition ?? new Windows.Foundation.Point((ActualWidth - width) / 2, ActualHeight - height - 16);
        var x = Math.Clamp(position.X, 0, Math.Max(0, ActualWidth - width - 2));
        var y = Math.Clamp(position.Y, 0, Math.Max(0, ActualHeight - height));
        Canvas.SetLeft(_floatFrame, x);
        Canvas.SetTop(_floatFrame, y);
    }

    private void OnDragPressed(object sender, PointerRoutedEventArgs e)
    {
        _dragPointer = e.Pointer.PointerId;
        _dragStart = e.GetCurrentPoint(this).Position;
        _dragOrigin = new Windows.Foundation.Point(Canvas.GetLeft(_floatFrame), Canvas.GetTop(_floatFrame));
        _dragHandle.CapturePointer(e.Pointer);
        e.Handled = true;
    }

    private void OnDragMoved(object sender, PointerRoutedEventArgs e)
    {
        if (_dragPointer != e.Pointer.PointerId)
        {
            return;
        }
        var p = e.GetCurrentPoint(this).Position;
        _floatPosition = new Windows.Foundation.Point(_dragOrigin.X + p.X - _dragStart.X, _dragOrigin.Y + p.Y - _dragStart.Y);
        PositionFloatingFrame();
        e.Handled = true;
    }

    private void OnDragReleased(object sender, PointerRoutedEventArgs e)
    {
        _dragPointer = null;
        _dragHandle.ReleasePointerCapture(e.Pointer);
    }

    private void OnContentChanged(UIElement? oldContent, UIElement? newContent)
    {
        if (oldContent is not null)
        {
            _contentHost.Children.Remove(oldContent);
        }
        if (newContent is not null)
        {
            _contentHost.Children.Add(newContent);
        }
    }

    private void OnVisibilityModeChanged()
    {
        if (VisibilityMode == KeyboardVisibilityMode.AlwaysVisible)
        {
            IsKeyboardOpen = true;
        }
    }

    private void OnGotFocus(object sender, RoutedEventArgs e)
    {
        if (e.OriginalSource is UIElement element && OnScreenKeyboard.IsTextInput(element) && !IsInsideKeyboard(element))
        {
            _pendingClose = false;
            _focusedInput = element;
            Keyboard.AttachTo(element);
            if (VisibilityMode != KeyboardVisibilityMode.Manual)
            {
                IsKeyboardOpen = true;
            }
            BringIntoViewLater(element);
        }
    }

    private void OnLostFocus(object sender, RoutedEventArgs e)
    {
        if (e.OriginalSource is not UIElement element || !ReferenceEquals(element, _focusedInput))
        {
            return;
        }
        _pendingClose = true;
        // Defer: focus may move to another text input.
        DispatcherQueue.TryEnqueue(() =>
        {
            if (!_pendingClose)
            {
                return;
            }
            var focused = XamlRoot is null ? null : FocusManager.GetFocusedElement(XamlRoot);
            if (focused is UIElement f && OnScreenKeyboard.IsTextInput(f))
            {
                return;
            }
            _focusedInput = null;
            Keyboard.Detach();
            if (VisibilityMode == KeyboardVisibilityMode.Auto)
            {
                IsKeyboardOpen = false;
            }
        });
    }

    private void OnPointerPressed(object sender, PointerRoutedEventArgs e)
    {
        // Tapping the already focused field re-opens a keyboard that was hidden with a gesture.
        if (!IsKeyboardOpen && VisibilityMode == KeyboardVisibilityMode.Auto && _focusedInput is not null &&
            e.OriginalSource is DependencyObject source && IsDescendantOf(source, _focusedInput))
        {
            IsKeyboardOpen = true;
        }
    }

    private bool IsInsideKeyboard(DependencyObject element) => IsDescendantOf(element, Keyboard);

    private static bool IsDescendantOf(DependencyObject element, DependencyObject ancestor)
    {
        var current = element;
        while (current is not null)
        {
            if (ReferenceEquals(current, ancestor))
            {
                return true;
            }
            current = VisualTreeHelper.GetParent(current);
        }
        return false;
    }

    private void BringIntoViewLater(UIElement element)
    {
        DispatcherQueue.TryEnqueue(Microsoft.UI.Dispatching.DispatcherQueuePriority.Low, () =>
        {
            try
            {
                element.StartBringIntoView(new BringIntoViewOptions { AnimationDesired = true, VerticalAlignmentRatio = 0.5 });
            }
            catch (Exception)
            {
                // Not supported on every platform/container.
            }
        });
    }

    private void OnIsOpenChanged(bool open)
    {
        _storyboard?.Stop();
        if (_isFloating)
        {
            _floatLayer.Visibility = open ? Visibility.Visible : Visibility.Collapsed;
            if (open)
            {
                PositionFloatingFrame();
            }
            else
            {
                Keyboard.TouchProcessor.CancelAll();
            }
            KeyboardOpenChanged?.Invoke(this, EventArgs.Empty);
            return;
        }
        if (open)
        {
            Keyboard.Visibility = Visibility.Visible;
            if (IsAnimated)
            {
                Keyboard.UpdateLayout();
                var height = Keyboard.ActualHeight > 0 ? Keyboard.ActualHeight : 300;
                Animate(height, 0, null);
            }
            if (_focusedInput is not null)
            {
                BringIntoViewLater(_focusedInput);
            }
        }
        else
        {
            Keyboard.TouchProcessor.CancelAll();
            if (IsAnimated && Keyboard.ActualHeight > 0)
            {
                Animate(0, Keyboard.ActualHeight, () => Keyboard.Visibility = IsKeyboardOpen ? Visibility.Visible : Visibility.Collapsed);
            }
            else
            {
                Keyboard.Visibility = Visibility.Collapsed;
            }
        }
        KeyboardOpenChanged?.Invoke(this, EventArgs.Empty);
    }

    private void Animate(double from, double to, Action? completed)
    {
        var animation = new DoubleAnimation
        {
            From = from,
            To = to,
            Duration = new Duration(TimeSpan.FromMilliseconds(180)),
            EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut },
            EnableDependentAnimation = true,
        };
        Storyboard.SetTarget(animation, _translate);
        Storyboard.SetTargetProperty(animation, nameof(TranslateTransform.Y));
        var storyboard = new Storyboard();
        storyboard.Children.Add(animation);
        if (completed is not null)
        {
            storyboard.Completed += (_, _) => completed();
        }
        _storyboard = storyboard;
        storyboard.Begin();
    }
}
