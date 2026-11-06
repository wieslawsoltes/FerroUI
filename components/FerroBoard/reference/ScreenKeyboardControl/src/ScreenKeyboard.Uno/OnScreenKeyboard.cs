using System.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using ScreenKeyboard.Controls.Icons;
using ScreenKeyboard.Controls.Primitives;
using ScreenKeyboard.Controls.Services;
using ScreenKeyboard.Controls.Themes;
using ScreenKeyboard.Controls.Views;
using ScreenKeyboard.Editor;
using ScreenKeyboard.Engine;
using ScreenKeyboard.Input;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Layouts;
using ScreenKeyboard.Settings;
using Windows.Foundation;

namespace ScreenKeyboard.Controls;

/// <summary>Arguments of <see cref="OnScreenKeyboard.EnterActionRequested"/>.</summary>
public sealed class EnterActionRequestedEventArgs(UIElement? element, EnterAction action) : EventArgs
{
    /// <summary>The element being edited, if any.</summary>
    public UIElement? Element { get; } = element;

    /// <summary>The requested action.</summary>
    public EnterAction Action { get; } = action;

    /// <summary>Set to <c>true</c> when the action was handled.</summary>
    public bool Handled { get; set; }
}

/// <summary>
/// A complete on-screen keyboard for Uno Platform: smartbar with suggestions and quick actions, FlorisBoard
/// compatible layouts, glide typing, gestures, emoji, clipboard and editing panels. Place it anywhere in your UI
/// (or use <see cref="ScreenKeyboardHost"/>) and it types into the focused <see cref="TextBox"/>/<see cref="PasswordBox"/>.
/// </summary>
public sealed partial class OnScreenKeyboard : UserControl
{
    /// <summary>Identifies the <see cref="Settings"/> property.</summary>
    public static readonly DependencyProperty SettingsProperty = DependencyProperty.Register(
        nameof(Settings), typeof(KeyboardSettings), typeof(OnScreenKeyboard), new PropertyMetadata(null, (d, e) => ((OnScreenKeyboard)d).OnSettingsReplaced(e.NewValue as KeyboardSettings)));

    /// <summary>Identifies the <see cref="Theme"/> property.</summary>
    public static readonly DependencyProperty ThemeProperty = DependencyProperty.Register(
        nameof(Theme), typeof(KeyboardTheme), typeof(OnScreenKeyboard), new PropertyMetadata(null, (d, _) => ((OnScreenKeyboard)d).ApplyTheme()));

    /// <summary>Identifies the <see cref="AutoAttach"/> property.</summary>
    public static readonly DependencyProperty AutoAttachProperty = DependencyProperty.Register(
        nameof(AutoAttach), typeof(bool), typeof(OnScreenKeyboard), new PropertyMetadata(true, (d, _) => ((OnScreenKeyboard)d).UpdateFocusTracking()));

    private readonly Grid _root = new();
    private readonly Grid _body = new();
    private readonly Grid _main = new();
    private readonly Border _sidePanel = new();
    private readonly Canvas _overlay = new() { IsHitTestVisible = false };
    private readonly Grid _resizeOverlay = new() { Visibility = Visibility.Collapsed, AllowFocusOnInteraction = false };
    private readonly Border _preview = new() { IsHitTestVisible = false, Visibility = Visibility.Collapsed };
    private readonly TextBlock _previewText = new() { HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center, IsHitTestVisible = false };
    private readonly Border _popup = new() { IsHitTestVisible = false, Visibility = Visibility.Collapsed };
    private readonly Canvas _popupCanvas = new() { IsHitTestVisible = false };
    private readonly DispatcherKeyboardScheduler _scheduler;
    private readonly KeysView _keysView;
    private readonly SmartbarView _smartbar;
    private readonly EmojiPanelView _emojiPanel;
    private readonly ClipboardPanelView _clipboardPanel;
    private readonly EditingPanelView _editingPanel;
    private readonly SubtypePickerView _subtypePicker;
    private KeyboardTheme _effectiveTheme = KeyboardThemes.FlorisDay;
    private KeyboardEngine _engine;
    private IDisposable? _ownedTarget;
    private UIElement? _targetElement;
    private ITextInputTarget? _searchOriginalTarget;
    private TextBufferTarget? _searchBuffer;
    private EmojiSearchState? _searchState;
    private bool _focusTracking;
    private bool _isLoaded;

    /// <summary>Creates a keyboard with default settings.</summary>
    public OnScreenKeyboard() : this(null)
    {
    }

    /// <summary>Creates a keyboard around an existing engine (share one engine between several views if needed).</summary>
    public OnScreenKeyboard(KeyboardEngine? engine)
    {
        _scheduler = new DispatcherKeyboardScheduler();
        _engine = engine ?? new KeyboardEngine();
        if (_engine.ClipboardService is Clipboard.InMemoryClipboardService)
        {
            _engine.ClipboardService = new SystemClipboardService();
        }
        TouchProcessor = new KeyboardTouchProcessor(_engine, _scheduler);
        _keysView = new KeysView(_engine, TouchProcessor, _scheduler);
        _smartbar = new SmartbarView(_engine, _scheduler);
        _emojiPanel = new EmojiPanelView(_engine, _scheduler);
        _clipboardPanel = new ClipboardPanelView(_engine, _scheduler);
        _editingPanel = new EditingPanelView(_engine, _scheduler);
        _subtypePicker = new SubtypePickerView(_engine, _scheduler) { Visibility = Visibility.Collapsed };

        IsTabStop = false;
        AllowFocusOnInteraction = false;
        HorizontalAlignment = HorizontalAlignment.Stretch;
        VerticalAlignment = VerticalAlignment.Bottom;
        BuildVisualTree();
        SetValue(SettingsProperty, _engine.Settings);
        HookEngine(_engine);

        _smartbar.HideRequested += (_, _) => OnHideRequested();
        _smartbar.SearchBackRequested += (_, _) => EndEmojiSearch(returnToEmoji: true);
        _emojiPanel.SearchRequested += (_, _) => BeginEmojiSearch();
        _subtypePicker.ManageRequested += (_, _) => SettingsRequested?.Invoke(this, EventArgs.Empty);
        TouchProcessor.PopupChanged += (_, _) => UpdatePopup();
        _keysView.KeyPressedVisual += (_, key) => ShowPreview(key);
        _keysView.KeyReleasedVisual += (_, key) => HidePreview(key);

        Loaded += (_, _) =>
        {
            _isLoaded = true;
            UpdateFocusTracking();
            ApplyTheme();
            SubscribeSystemClipboard(true);
        };
        Unloaded += (_, _) =>
        {
            _isLoaded = false;
            UpdateFocusTracking();
            TouchProcessor.CancelAll();
            SubscribeSystemClipboard(false);
        };
        ActualThemeChanged += (_, _) => ApplyTheme();
        SizeChanged += (_, e) =>
        {
            if (Math.Abs(e.NewSize.Width - e.PreviousSize.Width) > 0.5 &&
                (_engine.Settings.OneHandedMode != OneHandedMode.Off || _engine.Settings.SplitKeyboard))
            {
                UpdateLayoutMode();
            }
        };
        KeyboardThemes.Changed += (_, _) => DispatcherQueue?.TryEnqueue(ApplyTheme);
    }

    // ================================================================== public API

    /// <summary>The keyboard engine (state, layouts, suggestions...).</summary>
    public KeyboardEngine Engine => _engine;

    /// <summary>The touch processor (exposed for advanced customization).</summary>
    public KeyboardTouchProcessor TouchProcessor { get; }

    /// <summary>Keyboard settings. Replacing the instance copies its values into the engine settings.</summary>
    public KeyboardSettings Settings
    {
        get => (KeyboardSettings)GetValue(SettingsProperty);
        set => SetValue(SettingsProperty, value);
    }

    /// <summary>
    /// Explicit theme. When <c>null</c> (default), the theme is chosen from <see cref="KeyboardSettings.ThemeMode"/>,
    /// <see cref="KeyboardSettings.LightTheme"/> and <see cref="KeyboardSettings.DarkTheme"/>.
    /// </summary>
    public KeyboardTheme? Theme
    {
        get => (KeyboardTheme?)GetValue(ThemeProperty);
        set => SetValue(ThemeProperty, value);
    }

    /// <summary>The theme currently in use.</summary>
    public KeyboardTheme EffectiveTheme => _effectiveTheme;

    /// <summary>
    /// When <c>true</c> (default) the keyboard automatically attaches to the focused text input of its window.
    /// </summary>
    public bool AutoAttach
    {
        get => (bool)GetValue(AutoAttachProperty);
        set => SetValue(AutoAttachProperty, value);
    }

    /// <summary>The element the keyboard is currently typing into, if any.</summary>
    public UIElement? TargetElement => _targetElement;

    /// <summary>Quick actions shown in the smartbar (modifiable).</summary>
    public IList<SmartbarAction> SmartbarActions => _smartbar.Actions;

    /// <summary>Raised when the user requests the keyboard settings (settings key, quick action).</summary>
    public event EventHandler? SettingsRequested;

    /// <summary>Raised when the user requests to hide the keyboard (swipe down, hide button).</summary>
    public event EventHandler? HideRequested;

    /// <summary>Raised when the voice input key is pressed.</summary>
    public event EventHandler? VoiceInputRequested;

    /// <summary>Raised for enter actions of single-line fields (Go, Search, Send, Done...).</summary>
    public event EventHandler<EnterActionRequestedEventArgs>? EnterActionRequested;

    /// <summary>Raised when the keyboard wants to give feedback (use it to play sounds).</summary>
    public event EventHandler<FeedbackKind>? FeedbackRequested;

    /// <summary>Raised when the keyboard attached to a new input element (or detached).</summary>
    public event EventHandler? TargetChanged;

    /// <summary>Attaches the keyboard to an input element (TextBox, PasswordBox) or detaches when <c>null</c>.</summary>
    public bool AttachTo(UIElement? element)
    {
        if (ReferenceEquals(element, _targetElement) && element is not null)
        {
            return true;
        }
        EndEmojiSearch(returnToEmoji: false);
        ITextInputTarget? target = element switch
        {
            TextBox tb when ScreenKeyboardInput.IsEnabledFor(tb) => new TextBoxInputTarget(tb, a => RaiseEnterAction(tb, a)),
            PasswordBox pb when ScreenKeyboardInput.IsEnabledFor(pb) => new PasswordBoxInputTarget(pb, a => RaiseEnterAction(pb, a)),
            _ => null,
        };
        if (element is not null && target is null)
        {
            return false;
        }
        _ownedTarget?.Dispose();
        _ownedTarget = target as IDisposable;
        _targetElement = element;
        if (target is null)
        {
            _engine.Detach();
        }
        else
        {
            _engine.Attach(target);
        }
        TargetChanged?.Invoke(this, EventArgs.Empty);
        return true;
    }

    /// <summary>Attaches the keyboard to any custom <see cref="ITextInputTarget"/>.</summary>
    public void Attach(ITextInputTarget target)
    {
        EndEmojiSearch(returnToEmoji: false);
        _ownedTarget?.Dispose();
        _ownedTarget = null;
        _targetElement = null;
        _engine.Attach(target);
        TargetChanged?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>Detaches from the current target.</summary>
    public void Detach() => AttachTo(null);

    /// <summary>Returns <c>true</c> if the element is an input the keyboard can type into.</summary>
    public static bool IsTextInput(object? element) =>
        element is TextBox { IsReadOnly: false } or PasswordBox && ScreenKeyboardInput.IsEnabledFor((DependencyObject)element);

    // ================================================================== visual tree

    private void BuildVisualTree()
    {
        _root.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        _root.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        Grid.SetRow(_smartbar, 0);
        Grid.SetRow(_body, 1);

        _main.Children.Add(_keysView);
        _main.Children.Add(_emojiPanel);
        _main.Children.Add(_clipboardPanel);
        _main.Children.Add(_editingPanel);
        _main.Children.Add(_subtypePicker);
        _body.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        _body.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        _body.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        _body.Children.Add(_main);
        _body.Children.Add(_sidePanel);

        _resizeOverlay.PointerPressed += OnResizePointerPressed;
        _resizeOverlay.PointerMoved += OnResizePointerMoved;
        _resizeOverlay.PointerReleased += OnResizePointerReleased;
        _resizeOverlay.PointerCanceled += OnResizePointerReleased;
        _preview.Child = _previewText;
        _popup.Child = _popupCanvas;
        _overlay.Children.Add(_preview);
        _overlay.Children.Add(_popup);
        Grid.SetRowSpan(_overlay, 2);

        Grid.SetRow(_resizeOverlay, 1);
        _root.Children.Add(_smartbar);
        _root.Children.Add(_body);
        _root.Children.Add(_resizeOverlay);
        _root.Children.Add(_overlay);
        Content = _root;
    }

    private void HookEngine(KeyboardEngine engine)
    {
        engine.KeyboardChanged += (_, _) => _keysView.Refresh();
        engine.StateChanged += (_, _) => OnEngineStateChanged();
        engine.SuggestionsChanged += (_, _) => _smartbar.Refresh();
        engine.HideRequested += (_, _) => OnHideRequested();
        engine.SettingsRequested += (_, _) => SettingsRequested?.Invoke(this, EventArgs.Empty);
        engine.VoiceInputRequested += (_, _) => VoiceInputRequested?.Invoke(this, EventArgs.Empty);
        engine.SubtypePickerRequested += (_, _) => _subtypePicker.Open();
        engine.ResizeModeRequested += (_, _) => ShowResizeOverlay();
        engine.FeedbackRequested += (_, kind) => OnFeedback(kind);
        engine.Settings.PropertyChanged += OnSettingsPropertyChanged;
    }

    private void OnSettingsReplaced(KeyboardSettings? settings)
    {
        if (settings is null)
        {
            SetValue(SettingsProperty, _engine.Settings);
            return;
        }
        if (!ReferenceEquals(settings, _engine.Settings))
        {
            _engine.Settings.CopyFrom(settings);
            SetValue(SettingsProperty, _engine.Settings);
        }
    }

    private void OnSettingsPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        switch (e.PropertyName)
        {
            case nameof(KeyboardSettings.ThemeMode):
            case nameof(KeyboardSettings.LightTheme):
            case nameof(KeyboardSettings.DarkTheme):
            case nameof(KeyboardSettings.KeyHeight):
            case nameof(KeyboardSettings.HeightScale):
            case nameof(KeyboardSettings.FontScale):
            case nameof(KeyboardSettings.KeySpacingHorizontal):
            case nameof(KeyboardSettings.KeySpacingVertical):
            case nameof(KeyboardSettings.OneHandedMode):
            case nameof(KeyboardSettings.OneHandedWidth):
            case nameof(KeyboardSettings.SplitKeyboard):
            case nameof(KeyboardSettings.SplitGapRatio):
            case nameof(KeyboardSettings.MaxKeyboardWidth):
            case nameof(KeyboardSettings.SmartbarEnabled):
            case nameof(KeyboardSettings.SmartbarLayout):
            case nameof(KeyboardSettings.NumberRow):
                ApplyTheme();
                break;
            case nameof(KeyboardSettings.IncognitoMode):
            case nameof(KeyboardSettings.AutoCorrect):
                _smartbar.Refresh();
                break;
        }
    }

    // ================================================================== theme & sizing

    private KeyboardTheme ResolveTheme()
    {
        if (Theme is { } explicitTheme)
        {
            return explicitTheme;
        }
        var settings = _engine.Settings;
        var dark = settings.ThemeMode switch
        {
            ThemeMode.Dark => true,
            ThemeMode.Light => false,
            _ => ActualTheme == ElementTheme.Dark,
        };
        return (dark ? KeyboardThemes.Get(settings.DarkTheme) ?? KeyboardThemes.FlorisNight : KeyboardThemes.Get(settings.LightTheme) ?? KeyboardThemes.FlorisDay);
    }

    /// <summary>Re-applies theme, sizes and layout options.</summary>
    public void ApplyTheme()
    {
        var theme = ResolveTheme();
        _effectiveTheme = theme;
        var settings = _engine.Settings;
        var scale = settings.HeightScale;
        var rowHeight = settings.KeyHeight * scale;
        var rows = settings.NumberRow ? 5 : 4;
        var bodyHeight = rows * rowHeight + 6;
        var smartbarHeight = Math.Round(Math.Max(36, rowHeight * 0.82));

        _root.Background = Brushes.Get(theme.Background);
        _body.Height = bodyHeight;
        _body.Padding = new Thickness(3, 3, 3, 3);
        _smartbar.Visibility = settings.SmartbarEnabled ? Visibility.Visible : Visibility.Collapsed;
        _smartbar.SetTheme(theme, smartbarHeight);

        var fontFamily = theme.FontFamily is { } ff ? new FontFamily(ff) : null;
        _keysView.FontScale = settings.FontScale;
        _keysView.LabelFontFamily = fontFamily;
        _keysView.Geometry = new KeyboardGeometryOptions
        {
            KeySpacingHorizontal = settings.KeySpacingHorizontal,
            KeySpacingVertical = settings.KeySpacingVertical,
            SplitGap = 0,
        };
        // Popups may extend above the keyboard (over the application content) like on mobile keyboards.
        TouchProcessor.PopupMinY = -(settings.SmartbarEnabled ? smartbarHeight : 0) - rowHeight * 2;
        _keysView.SetTheme(theme);
        _emojiPanel.BarHeight = smartbarHeight;
        _emojiPanel.SetTheme(theme);
        _clipboardPanel.HeaderHeight = smartbarHeight;
        _clipboardPanel.SetTheme(theme);
        _editingPanel.SetTheme(theme);
        _subtypePicker.SetTheme(theme);

        _preview.Background = Brushes.Get(theme.PopupBackground);
        _preview.BorderBrush = Brushes.Get(theme.PopupBorderColor);
        _preview.BorderThickness = new Thickness(1);
        _preview.CornerRadius = new CornerRadius(theme.PopupCornerRadius);
        _previewText.Foreground = Brushes.Get(theme.PopupForeground);
        if (fontFamily is not null)
        {
            _previewText.FontFamily = fontFamily;
        }
        _popup.Background = Brushes.Get(theme.PopupBackground);
        _popup.BorderBrush = Brushes.Get(theme.PopupBorderColor);
        _popup.BorderThickness = new Thickness(1);
        _popup.CornerRadius = new CornerRadius(theme.PopupCornerRadius);

        UpdateLayoutMode();
        UpdatePanels();
        FocusNeutral.Apply(_root);
    }

    private void UpdateLayoutMode()
    {
        var settings = _engine.Settings;
        var theme = _effectiveTheme;
        _sidePanel.Child = null;
        _sidePanel.Visibility = Visibility.Collapsed;
        _main.MaxWidth = settings.MaxKeyboardWidth > 0 ? settings.MaxKeyboardWidth : double.PositiveInfinity;
        _main.HorizontalAlignment = HorizontalAlignment.Stretch;
        Grid.SetColumnSpan(_main, 3);
        Grid.SetColumn(_main, 0);

        var width = ActualWidth > 0 ? ActualWidth : 800;
        if (settings.OneHandedMode != OneHandedMode.Off)
        {
            var mainWidth = Math.Min(width * settings.OneHandedWidth, _main.MaxWidth);
            _main.Width = mainWidth;
            _main.HorizontalAlignment = settings.OneHandedMode == OneHandedMode.Left ? HorizontalAlignment.Left : HorizontalAlignment.Right;
            _sidePanel.Width = Math.Max(48, width - mainWidth - 6);
            _sidePanel.HorizontalAlignment = settings.OneHandedMode == OneHandedMode.Left ? HorizontalAlignment.Right : HorizontalAlignment.Left;
            Grid.SetColumnSpan(_sidePanel, 3);
            _sidePanel.Background = Brushes.Get(theme.OneHandedBackground);
            _sidePanel.CornerRadius = new CornerRadius(theme.KeyCornerRadius);
            _sidePanel.Child = FocusNeutral.Apply(BuildOneHandedPanel(theme));
            _sidePanel.Visibility = Visibility.Visible;
        }
        else
        {
            _main.Width = double.NaN;
        }

        var splitGap = settings.SplitKeyboard && _engine.Mode is KeyboardMode.Characters or KeyboardMode.Symbols or KeyboardMode.Symbols2 && settings.OneHandedMode == OneHandedMode.Off
            ? Math.Min(width, _main.MaxWidth) * settings.SplitGapRatio
            : 0;
        if (Math.Abs(_keysView.Geometry.SplitGap - splitGap) > 0.1)
        {
            _keysView.Geometry = _keysView.Geometry with { SplitGap = splitGap };
        }
        _keysView.Relayout();
    }

    private UIElement BuildOneHandedPanel(KeyboardTheme theme)
    {
        var fg = Brushes.Get(theme.OneHandedForeground);
        var panel = new StackPanel { VerticalAlignment = VerticalAlignment.Center, HorizontalAlignment = HorizontalAlignment.Center, Spacing = 12 };
        var exit = new TouchButton { Width = 44, Height = 44, CornerRadius = new CornerRadius(22) };
        exit.SetScheduler(_scheduler);
        exit.SetColors(theme.ActionButtonBackground, theme.ActionButtonPressedBackground);
        exit.SetIcon(KeyIcon.Resize, 22, fg, KeyboardStrings.ExitOneHanded);
        exit.Click += (_, _) => _engine.Settings.OneHandedMode = OneHandedMode.Off;
        var move = new TouchButton { Width = 44, Height = 44, CornerRadius = new CornerRadius(22) };
        move.SetScheduler(_scheduler);
        move.SetColors(theme.ActionButtonBackground, theme.ActionButtonPressedBackground);
        var toLeft = _engine.Settings.OneHandedMode == OneHandedMode.Right;
        move.SetIcon(toLeft ? KeyIcon.ChevronLeft : KeyIcon.ChevronRight, 26, fg, toLeft ? KeyboardStrings.MoveKeyboardLeft : KeyboardStrings.MoveKeyboardRight);
        move.Click += (_, _) => _engine.Settings.OneHandedMode = toLeft ? OneHandedMode.Left : OneHandedMode.Right;
        panel.Children.Add(exit);
        panel.Children.Add(move);
        return panel;
    }

    // ================================================================== state

    private KeyboardUiMode _shownUiMode = (KeyboardUiMode)(-1);
    private KeyboardMode _shownMode = (KeyboardMode)(-1);

    private void OnEngineStateChanged()
    {
        if (_engine.Mode != _shownMode)
        {
            _shownMode = _engine.Mode;
            if (_engine.Settings.SplitKeyboard)
            {
                UpdateLayoutMode();
            }
        }
        UpdatePanels();
        _keysView.Refresh();
        _smartbar.Refresh();
    }

    private void UpdatePanels()
    {
        var mode = _engine.UiMode;
        _keysView.Visibility = mode == KeyboardUiMode.Text ? Visibility.Visible : Visibility.Collapsed;
        _emojiPanel.Visibility = mode == KeyboardUiMode.Media ? Visibility.Visible : Visibility.Collapsed;
        _clipboardPanel.Visibility = mode == KeyboardUiMode.Clipboard ? Visibility.Visible : Visibility.Collapsed;
        _editingPanel.Visibility = mode == KeyboardUiMode.Editing ? Visibility.Visible : Visibility.Collapsed;
        if (mode != _shownUiMode)
        {
            _shownUiMode = mode;
            TouchProcessor.CancelAll();
            HidePreview(null);
            _subtypePicker.Close();
            switch (mode)
            {
                case KeyboardUiMode.Media:
                    _emojiPanel.OnShown();
                    break;
                case KeyboardUiMode.Clipboard:
                    _clipboardPanel.Refresh();
                    break;
            }
        }
    }

    private void OnHideRequested()
    {
        EndEmojiSearch(returnToEmoji: false);
        HideRequested?.Invoke(this, EventArgs.Empty);
    }

    private bool RaiseEnterAction(UIElement element, EnterAction action)
    {
        var args = new EnterActionRequestedEventArgs(element, action);
        EnterActionRequested?.Invoke(this, args);
        if (!args.Handled && action == EnterAction.Done)
        {
            // KeyboardStrings.Done closes the keyboard by default.
            OnHideRequested();
            return true;
        }
        return args.Handled;
    }

    private void OnFeedback(FeedbackKind kind)
    {
        var settings = _engine.Settings;
        if (settings.HapticFeedback)
        {
            KeyboardHaptics.Perform(kind, settings.HapticDuration);
        }
        FeedbackRequested?.Invoke(this, kind);
    }

    // ================================================================== preview & popup

    private ComputedKey? _previewKey;

    private void ShowPreview(ComputedKey key)
    {
        if (!_engine.Settings.ShowKeyPreview || !key.IsCharacterKey || key.Label is null || _engine.IsGliding)
        {
            return;
        }
        var bounds = key.VisibleBounds;
        var origin = _keysView.TransformToVisual(_overlay).TransformPoint(new Point(bounds.X, bounds.Y));
        var width = Math.Max(bounds.Width * 1.3, 36);
        var height = Math.Max(bounds.Height * 1.15, 40);
        _preview.Width = width;
        _preview.Height = height;
        _previewText.Text = key.Label;
        _previewText.FontSize = Math.Min(_effectiveTheme.KeyFontSize * 1.4 * _engine.Settings.FontScale, height * 0.62);
        var x = Math.Clamp(origin.X + bounds.Width / 2 - width / 2, 0, Math.Max(0, ActualWidth - width));
        Canvas.SetLeft(_preview, x);
        Canvas.SetTop(_preview, origin.Y - height - 4);
        _preview.Visibility = Visibility.Visible;
        _previewKey = key;
    }

    private void HidePreview(ComputedKey? key)
    {
        if (key is null || ReferenceEquals(key, _previewKey))
        {
            _preview.Visibility = Visibility.Collapsed;
            _previewKey = null;
        }
    }

    private void UpdatePopup()
    {
        var popup = TouchProcessor.Popup;
        if (popup is null)
        {
            _popup.Visibility = Visibility.Collapsed;
            _popupCanvas.Children.Clear();
            return;
        }
        HidePreview(null);
        var theme = _effectiveTheme;
        var frame = popup.Frame;
        var origin = _keysView.TransformToVisual(_overlay).TransformPoint(new Point(frame.X, frame.Y));
        _popup.Width = frame.Width + 2;
        _popup.Height = frame.Height + 2;
        Canvas.SetLeft(_popup, origin.X - 1);
        Canvas.SetTop(_popup, origin.Y - 1);
        _popupCanvas.Children.Clear();
        var ctx = _engine.CreateComputeContext();
        for (var i = 0; i < popup.Keys.Count; i++)
        {
            var b = popup.Bounds[i];
            var selected = i == popup.SelectedIndex;
            var cell = new Border
            {
                Width = b.Width,
                Height = b.Height,
                CornerRadius = new CornerRadius(theme.PopupCornerRadius * 0.7),
                Background = selected ? Brushes.Get(theme.PopupSelectedBackground) : Brushes.Transparent,
                IsHitTestVisible = false,
            };
            var fg = Brushes.Get(selected ? theme.PopupSelectedForeground : theme.PopupForeground);
            var (label, icon) = KeyLabels.GetDisplay(popup.Keys[i], ctx);
            if (icon != KeyIcon.None && IconFactory.HasIcon(icon))
            {
                var path = IconFactory.Create(icon, Math.Min(22, b.Height * 0.5), fg);
                path.HorizontalAlignment = HorizontalAlignment.Center;
                path.VerticalAlignment = VerticalAlignment.Center;
                cell.Child = path;
            }
            else
            {
                cell.Child = new TextBlock
                {
                    Text = label ?? string.Empty,
                    Foreground = fg,
                    FontSize = Math.Min(theme.KeyFontSize * _engine.Settings.FontScale, b.Height * 0.55),
                    HorizontalAlignment = HorizontalAlignment.Center,
                    VerticalAlignment = VerticalAlignment.Center,
                    IsHitTestVisible = false,
                };
            }
            Canvas.SetLeft(cell, b.X - frame.X);
            Canvas.SetTop(cell, b.Y - frame.Y);
            _popupCanvas.Children.Add(cell);
        }
        _popup.Visibility = Visibility.Visible;
    }

    // ================================================================== emoji search

    private void BeginEmojiSearch()
    {
        if (_engine.Target is null)
        {
            return;
        }
        _searchOriginalTarget = _engine.Target;
        _searchBuffer = new TextBufferTarget(string.Empty, new InputAttributes
        {
            Kind = InputKind.Search,
            Capitalization = CapitalizationMode.None,
            AllowSuggestions = false,
            AllowAutoCorrect = false,
            IsPrivate = true,
        })
        {
            EnterActionHandler = _ =>
            {
                if (_searchState?.Results.FirstOrDefault() is { } first)
                {
                    InsertSearchResult(first);
                }
                return true;
            },
        };
        _searchState = new EmojiSearchState { Select = InsertSearchResult };
        _searchBuffer.Edited += (_, _) => UpdateSearchResults();
        _engine.Attach(_searchBuffer);
        _smartbar.EmojiSearch = _searchState;
        UpdateSearchResults();
    }

    private void UpdateSearchResults()
    {
        if (_searchState is null || _searchBuffer is null)
        {
            return;
        }
        _searchState.Query = _searchBuffer.Text;
        _searchState.Results = _searchBuffer.Text.Trim().Length == 0
            ? _engine.EmojiHistory.Combined.Take(20).ToList()
            : _engine.EmojiCatalog.Search(_searchBuffer.Text, 40).Select(e => e.WithSkinTone(_engine.EmojiHistory.GetPreferredTone(e.Value) ?? _engine.Settings.EmojiSkinTone)).ToList();
        _smartbar.Refresh();
    }

    private void InsertSearchResult(string emoji)
    {
        if (_searchOriginalTarget is null || _searchBuffer is null)
        {
            return;
        }
        _engine.Attach(_searchOriginalTarget);
        _engine.InputEmoji(emoji);
        _engine.Attach(_searchBuffer);
        UpdateSearchResults();
    }

    private void EndEmojiSearch(bool returnToEmoji)
    {
        if (_searchState is null)
        {
            return;
        }
        var original = _searchOriginalTarget;
        _searchState = null;
        _searchBuffer = null;
        _searchOriginalTarget = null;
        _smartbar.EmojiSearch = null;
        if (original is not null)
        {
            _engine.Attach(original);
        }
        if (returnToEmoji)
        {
            _engine.SetUiMode(KeyboardUiMode.Media);
        }
        _smartbar.Refresh();
    }

    // ================================================================== resize mode

    private double _resizeStartY;
    private double _resizeStartScale;
    private uint? _resizePointer;

    /// <summary>Shows the interactive resize overlay (drag vertically to change the keyboard height).</summary>
    public void ShowResizeOverlay()
    {
        var theme = _effectiveTheme;
        _subtypePicker.Close();
        _resizeOverlay.Children.Clear();
        _resizeOverlay.Background = Brushes.Get(Windows.UI.Color.FromArgb(0xE6, theme.Background.R, theme.Background.G, theme.Background.B));
        var fg = Brushes.Get(theme.PanelForeground);
        var panel = new StackPanel { HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center, Spacing = 10 };
        var icon = IconFactory.Create(KeyIcon.ResizeHeight, 32, fg);
        icon.HorizontalAlignment = HorizontalAlignment.Center;
        panel.Children.Add(icon);
        panel.Children.Add(new TextBlock { Text = KeyboardStrings.ResizeHint, Foreground = fg, FontSize = 15, HorizontalAlignment = HorizontalAlignment.Center });
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, HorizontalAlignment = HorizontalAlignment.Center };
        var reset = new TouchButton { Padding = new Thickness(16, 8, 16, 8), CornerRadius = new CornerRadius(18) };
        reset.SetScheduler(_scheduler);
        reset.SetColors(theme.FunctionKeyBackground, theme.FunctionKeyPressedBackground);
        reset.SetText(KeyboardStrings.Reset, 14, Brushes.Get(theme.FunctionKeyForeground));
        reset.Click += (_, _) => _engine.Settings.HeightScale = 1;
        var done = new TouchButton { Padding = new Thickness(16, 8, 16, 8), CornerRadius = new CornerRadius(18) };
        done.SetScheduler(_scheduler);
        done.SetColors(theme.AccentKeyBackground, theme.AccentKeyPressedBackground);
        done.SetText(KeyboardStrings.Done, 14, Brushes.Get(theme.AccentKeyForeground));
        done.Click += (_, _) => _resizeOverlay.Visibility = Visibility.Collapsed;
        buttons.Children.Add(reset);
        buttons.Children.Add(done);
        panel.Children.Add(buttons);
        _resizeOverlay.Children.Add(panel);
        FocusNeutral.Apply(_resizeOverlay);
        _resizeOverlay.Visibility = Visibility.Visible;
    }

    private void OnResizePointerPressed(object sender, PointerRoutedEventArgs e)
    {
        _resizePointer = e.Pointer.PointerId;
        _resizeStartY = e.GetCurrentPoint(this).Position.Y;
        _resizeStartScale = _engine.Settings.HeightScale;
        _resizeOverlay.CapturePointer(e.Pointer);
        e.Handled = true;
    }

    private void OnResizePointerMoved(object sender, PointerRoutedEventArgs e)
    {
        if (_resizePointer != e.Pointer.PointerId)
        {
            return;
        }
        var settings = _engine.Settings;
        var rows = settings.NumberRow ? 5 : 4;
        var delta = _resizeStartY - e.GetCurrentPoint(this).Position.Y;
        var scale = Math.Round(_resizeStartScale + delta / (rows * settings.KeyHeight), 2);
        if (Math.Abs(scale - settings.HeightScale) >= 0.02)
        {
            settings.HeightScale = scale;
        }
        e.Handled = true;
    }

    private void OnResizePointerReleased(object sender, PointerRoutedEventArgs e)
    {
        _resizePointer = null;
        _resizeOverlay.ReleasePointerCapture(e.Pointer);
    }

    // ================================================================== system clipboard

    private bool _clipboardSubscribed;

    private void SubscribeSystemClipboard(bool subscribe)
    {
        if (subscribe == _clipboardSubscribed || _engine.ClipboardService is not SystemClipboardService)
        {
            return;
        }
        try
        {
            if (subscribe)
            {
                Windows.ApplicationModel.DataTransfer.Clipboard.ContentChanged += OnSystemClipboardChanged;
            }
            else
            {
                Windows.ApplicationModel.DataTransfer.Clipboard.ContentChanged -= OnSystemClipboardChanged;
            }
            _clipboardSubscribed = subscribe;
        }
        catch (Exception)
        {
            // Clipboard change notifications are not available on every platform.
        }
    }

    private async void OnSystemClipboardChanged(object? sender, object e)
    {
        try
        {
            var text = await _engine.ClipboardService.GetTextAsync();
            if (!string.IsNullOrEmpty(text))
            {
                DispatcherQueue?.TryEnqueue(() => _engine.RecordClipboard(text!));
            }
        }
        catch (Exception)
        {
            // Ignore clipboard access errors.
        }
    }

    // ================================================================== focus tracking

    private void UpdateFocusTracking()
    {
        var shouldTrack = AutoAttach && _isLoaded;
        if (shouldTrack == _focusTracking)
        {
            return;
        }
        _focusTracking = shouldTrack;
        if (shouldTrack)
        {
            FocusManager.GotFocus += OnGlobalGotFocus;
            if (XamlRoot is { } root && FocusManager.GetFocusedElement(root) is UIElement focused && IsTextInput(focused))
            {
                AttachTo(focused);
            }
        }
        else
        {
            FocusManager.GotFocus -= OnGlobalGotFocus;
        }
    }

    private void OnGlobalGotFocus(object? sender, FocusManagerGotFocusEventArgs e)
    {
        if (e.NewFocusedElement is not UIElement element || element.XamlRoot != XamlRoot)
        {
            return;
        }
        if (IsTextInput(element))
        {
            AttachTo(element);
        }
    }
}
