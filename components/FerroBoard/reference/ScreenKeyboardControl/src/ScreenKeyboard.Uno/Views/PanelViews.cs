using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using ScreenKeyboard.Clipboard;
using ScreenKeyboard.Controls.Icons;
using ScreenKeyboard.Controls.Primitives;
using ScreenKeyboard.Controls.Themes;
using ScreenKeyboard.Engine;
using ScreenKeyboard.Input;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Layouts;

namespace ScreenKeyboard.Controls.Views;

/// <summary>Shared helpers for panel views.</summary>
internal static class PanelUi
{
    public static TouchButton Button(IKeyboardScheduler scheduler, KeyboardTheme theme, bool function = true)
    {
        var button = new TouchButton { CornerRadius = new CornerRadius(theme.KeyCornerRadius), Margin = new Thickness(3) };
        button.SetScheduler(scheduler);
        if (function)
        {
            button.SetColors(theme.FunctionKeyBackground, theme.FunctionKeyPressedBackground);
        }
        else
        {
            button.SetColors(theme.KeyBackground, theme.KeyPressedBackground);
        }
        return button;
    }

    public static TextBlock Title(string text, KeyboardTheme theme) => new()
    {
        Text = text,
        Foreground = Brushes.Get(theme.PanelForeground),
        FontSize = theme.SmartbarFontSize,
        FontWeight = FontWeights.SemiBold,
        VerticalAlignment = VerticalAlignment.Center,
        Margin = new Thickness(8, 0, 8, 0),
    };
}

/// <summary>Clipboard history panel with paste, pin and delete.</summary>
internal sealed partial class ClipboardPanelView : Grid
{
    private readonly KeyboardEngine _engine;
    private readonly IKeyboardScheduler _scheduler;
    private readonly Grid _header = new();
    private readonly ScrollViewer _scroller = new()
    {
        VerticalScrollBarVisibility = ScrollBarVisibility.Hidden,
        HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled,
        VerticalScrollMode = ScrollMode.Enabled,
        HorizontalScrollMode = ScrollMode.Disabled,
        IsTabStop = false,
    };
    private KeyboardTheme _theme = KeyboardThemes.FlorisDay;

    public ClipboardPanelView(KeyboardEngine engine, IKeyboardScheduler scheduler)
    {
        _engine = engine;
        _scheduler = scheduler;
        IsTabStop = false;
        RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        Grid.SetRow(_scroller, 1);
        Children.Add(_header);
        Children.Add(_scroller);
        _engine.ClipboardHistory.Changed += (_, _) =>
        {
            if (Visibility == Visibility.Visible)
            {
                Refresh();
            }
        };
    }

    public double HeaderHeight { get; set; } = 44;

    public void SetTheme(KeyboardTheme theme)
    {
        _theme = theme;
        Background = Brushes.Get(theme.PanelBackground);
        Refresh();
    }

    public void Refresh()
    {
        BuildHeader();
        BuildItems();
        FocusNeutral.Apply(this);
    }

    private void BuildHeader()
    {
        _header.Children.Clear();
        _header.ColumnDefinitions.Clear();
        _header.Height = HeaderHeight;
        _header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        _header.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        _header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        _header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

        var abc = PanelUi.Button(_scheduler, _theme);
        abc.Width = HeaderHeight * 1.4;
        abc.SetText(KeyboardStrings.Abc, _theme.FunctionKeyFontSize * 0.9, Brushes.Get(_theme.FunctionKeyForeground));
        abc.Click += (_, _) => _engine.SetUiMode(KeyboardUiMode.Text);
        _header.Children.Add(abc);

        var title = PanelUi.Title(KeyboardStrings.Clipboard, _theme);
        Grid.SetColumn(title, 1);
        _header.Children.Add(title);

        var fg = Brushes.Get(_theme.PanelForeground);
        var toggle = new TouchButton { Width = HeaderHeight, CornerRadius = new CornerRadius(HeaderHeight / 2) };
        toggle.SetScheduler(_scheduler);
        toggle.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
        var enabled = _engine.Settings.ClipboardHistoryEnabled;
        toggle.SetIcon(enabled ? KeyIcon.Clipboard : KeyIcon.Incognito, 20, enabled ? Brushes.Get(_theme.PanelSelectedForeground) : fg,
            enabled ? KeyboardStrings.DisableClipboardHistory : KeyboardStrings.EnableClipboardHistory);
        toggle.Click += (_, _) =>
        {
            _engine.Settings.ClipboardHistoryEnabled = !_engine.Settings.ClipboardHistoryEnabled;
            Refresh();
        };
        Grid.SetColumn(toggle, 2);
        _header.Children.Add(toggle);

        var clear = new TouchButton { Width = HeaderHeight, CornerRadius = new CornerRadius(HeaderHeight / 2) };
        clear.SetScheduler(_scheduler);
        clear.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
        clear.SetIcon(KeyIcon.Delete, 20, fg, KeyboardStrings.ClearClipboardHistory);
        clear.Click += (_, _) => _engine.ClipboardHistory.Clear();
        Grid.SetColumn(clear, 3);
        _header.Children.Add(clear);
    }

    private void BuildItems()
    {
        var items = _engine.ClipboardHistory.Items;
        if (items.Count == 0)
        {
            _scroller.Content = new TextBlock
            {
                Text = _engine.Settings.ClipboardHistoryEnabled ? KeyboardStrings.ClipboardEmpty : KeyboardStrings.ClipboardDisabled,
                Foreground = Brushes.Get(_theme.KeyHintForeground),
                TextWrapping = TextWrapping.Wrap,
                TextAlignment = TextAlignment.Center,
                HorizontalAlignment = HorizontalAlignment.Center,
                Margin = new Thickness(16, 32, 16, 0),
            };
            return;
        }
        var panel = new UniformWrapPanel { ItemWidth = 220, ItemHeight = 84, Margin = new Thickness(4) };
        foreach (var item in items)
        {
            panel.Children.Add(CreateItem(item));
        }
        _scroller.Content = panel;
    }

    private UIElement CreateItem(ClipboardItem item)
    {
        var card = new TouchButton { Margin = new Thickness(4), CornerRadius = new CornerRadius(12) };
        card.SetScheduler(_scheduler);
        card.SetColors(_theme.PanelItemBackground, KeyStyles.Darken(_theme.PanelItemBackground, 0.92));
        var grid = new Grid { Padding = new Thickness(10, 6, 4, 4) };
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        grid.Children.Add(new TextBlock
        {
            Text = item.Text,
            Foreground = Brushes.Get(_theme.KeyForeground),
            TextWrapping = TextWrapping.Wrap,
            TextTrimming = TextTrimming.CharacterEllipsis,
            MaxLines = 3,
            FontSize = 13,
            VerticalAlignment = VerticalAlignment.Top,
            IsHitTestVisible = false,
        });
        var actions = new StackPanel { Orientation = Orientation.Vertical, VerticalAlignment = VerticalAlignment.Center };
        var pin = SmallAction(item.IsPinned ? KeyIcon.Pin : KeyIcon.Unpin, item.IsPinned ? KeyboardStrings.Unpin : KeyboardStrings.Pin,
            item.IsPinned ? _theme.PanelSelectedForeground : _theme.KeyHintForeground);
        pin.Click += (_, _) => _engine.ClipboardHistory.SetPinned(item, !item.IsPinned);
        var delete = SmallAction(KeyIcon.Close, KeyboardStrings.Delete, _theme.KeyHintForeground);
        delete.Click += (_, _) => _engine.ClipboardHistory.Remove(item);
        actions.Children.Add(pin);
        actions.Children.Add(delete);
        Grid.SetColumn(actions, 1);
        grid.Children.Add(actions);
        card.SetContent(grid);
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(card, string.Format(KeyboardStrings.PasteFormat, item.Text));
        card.Click += (_, _) => _engine.InputText(item.Text);
        return card;
    }

    private TouchButton SmallAction(KeyIcon icon, string name, Windows.UI.Color color)
    {
        var button = new TouchButton { Width = 32, Height = 32, CornerRadius = new CornerRadius(16) };
        button.SetScheduler(_scheduler);
        button.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
        button.SetIcon(icon, 16, Brushes.Get(color), name);
        return button;
    }
}

/// <summary>Text editing panel: cursor pad, selection, clipboard, undo/redo.</summary>
internal sealed partial class EditingPanelView : Grid
{
    private readonly KeyboardEngine _engine;
    private readonly IKeyboardScheduler _scheduler;
    private KeyboardTheme _theme = KeyboardThemes.FlorisDay;
    private TouchButton? _selectButton;

    public EditingPanelView(KeyboardEngine engine, IKeyboardScheduler scheduler)
    {
        _engine = engine;
        _scheduler = scheduler;
        IsTabStop = false;
        Padding = new Thickness(4);
        _engine.StateChanged += (_, _) => UpdateSelectState();
    }

    public void SetTheme(KeyboardTheme theme)
    {
        _theme = theme;
        Background = Brushes.Get(theme.PanelBackground);
        Build();
    }

    private void Build()
    {
        Children.Clear();
        RowDefinitions.Clear();
        ColumnDefinitions.Clear();
        for (var i = 0; i < 4; i++) RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        for (var i = 0; i < 5; i++) ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });

        Add(KeyIcon.Home, KeyboardStrings.LineStart, KeyCode.MoveStartOfLine, 0, 0);
        Add(KeyIcon.ArrowUp, KeyboardStrings.Up, KeyCode.ArrowUp, 0, 1, repeat: true);
        Add(KeyIcon.End, KeyboardStrings.LineEnd, KeyCode.MoveEndOfLine, 0, 2);
        Add(KeyIcon.ArrowLeft, KeyboardStrings.Left, KeyCode.ArrowLeft, 1, 0, repeat: true);
        _selectButton = Add(KeyIcon.Select, KeyboardStrings.Select, KeyCode.ClipboardSelect, 1, 1);
        Add(KeyIcon.ArrowRight, KeyboardStrings.Right, KeyCode.ArrowRight, 1, 2, repeat: true);
        Add(KeyIcon.DocumentStart, KeyboardStrings.StartOfText, KeyCode.MoveStartOfPage, 2, 0);
        Add(KeyIcon.ArrowDown, KeyboardStrings.Down, KeyCode.ArrowDown, 2, 1, repeat: true);
        Add(KeyIcon.DocumentEnd, KeyboardStrings.EndOfText, KeyCode.MoveEndOfPage, 2, 2);

        Add(KeyIcon.SelectAll, KeyboardStrings.SelectAll, KeyCode.ClipboardSelectAll, 0, 3);
        Add(KeyIcon.Copy, KeyboardStrings.Copy, KeyCode.ClipboardCopy, 1, 3);
        Add(KeyIcon.Cut, KeyboardStrings.Cut, KeyCode.ClipboardCut, 2, 3);
        Add(KeyIcon.Paste, KeyboardStrings.Paste, KeyCode.ClipboardPaste, 3, 3);
        Add(KeyIcon.Undo, KeyboardStrings.Undo, KeyCode.Undo, 0, 4);
        Add(KeyIcon.Redo, KeyboardStrings.Redo, KeyCode.Redo, 1, 4);
        Add(KeyIcon.Backspace, KeyboardStrings.Backspace, KeyCode.Delete, 2, 4, repeat: true);
        Add(KeyIcon.Enter, KeyboardStrings.Enter, KeyCode.Enter, 3, 4);

        var abc = PanelUi.Button(_scheduler, _theme);
        abc.SetText(KeyboardStrings.Abc, _theme.FunctionKeyFontSize, Brushes.Get(_theme.FunctionKeyForeground));
        abc.Click += (_, _) => _engine.SetUiMode(KeyboardUiMode.Text);
        Grid.SetRow(abc, 3);
        Grid.SetColumnSpan(abc, 3);
        Children.Add(abc);
        UpdateSelectState();
        FocusNeutral.Apply(this);
    }

    private TouchButton Add(KeyIcon icon, string name, int code, int row, int column, bool repeat = false)
    {
        var button = PanelUi.Button(_scheduler, _theme, function: code is not (KeyCode.ArrowUp or KeyCode.ArrowDown or KeyCode.ArrowLeft or KeyCode.ArrowRight));
        button.SetIcon(icon, 24, Brushes.Get(_theme.FunctionKeyForeground), name);
        if (repeat)
        {
            button.RepeatDelay = TimeSpan.FromMilliseconds(_engine.Settings.KeyRepeatDelay);
            button.RepeatInterval = TimeSpan.FromMilliseconds(Math.Max(40, _engine.Settings.KeyRepeatInterval));
        }
        var data = code == KeyCode.Enter ? TextKeyData.Function(KeyCode.Enter, KeyType.EnterEditing) : TextKeyDataFactory.Function(code);
        button.Click += (_, _) =>
        {
            _engine.InputKey(data);
            UpdateSelectState();
        };
        Grid.SetRow(button, row);
        Grid.SetColumn(button, column);
        Children.Add(button);
        return button;
    }

    private void UpdateSelectState()
    {
        if (_selectButton is null)
        {
            return;
        }
        var active = _engine.Editor?.IsSelecting == true;
        _selectButton.SetIcon(KeyIcon.Select, 22, Brushes.Get(active ? _theme.PanelSelectedForeground : _theme.FunctionKeyForeground), KeyboardStrings.Select);
        _selectButton.BorderBrush = Brushes.Get(_theme.PanelSelectedForeground);
        _selectButton.BorderThickness = new Thickness(active ? 2 : 0);
    }
}

/// <summary>Overlay listing enabled languages (subtypes).</summary>
internal sealed partial class SubtypePickerView : Grid
{
    private readonly KeyboardEngine _engine;
    private readonly IKeyboardScheduler _scheduler;
    private KeyboardTheme _theme = KeyboardThemes.FlorisDay;

    public SubtypePickerView(KeyboardEngine engine, IKeyboardScheduler scheduler)
    {
        _engine = engine;
        _scheduler = scheduler;
        IsTabStop = false;
        Background = Brushes.Get(Windows.UI.Color.FromArgb(0x66, 0, 0, 0));
        PointerPressed += (_, e) =>
        {
            e.Handled = true;
            Close();
        };
    }

    /// <summary>Raised when KeyboardStrings.ManageLanguages is selected.</summary>
    public event EventHandler? ManageRequested;

    public void SetTheme(KeyboardTheme theme) => _theme = theme;

    public void Open()
    {
        Children.Clear();
        var list = new StackPanel { Spacing = 2 };
        foreach (var subtype in _engine.Subtypes)
        {
            var active = subtype.Id == _engine.ActiveSubtype.Id;
            var button = new TouchButton { Height = 44, CornerRadius = new CornerRadius(8), Padding = new Thickness(12, 0, 12, 0) };
            button.SetScheduler(_scheduler);
            button.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
            var layout = _engine.Resources.GetLayout(LayoutType.Characters, subtype.Layouts.Characters)?.Label ?? subtype.Layouts.Characters.ComponentId;
            var text = new TextBlock
            {
                Text = $"{KeyboardEngine.GetSubtypeDisplayName(subtype)}  ·  {layout}",
                Foreground = Brushes.Get(active ? _theme.PanelSelectedForeground : _theme.PopupForeground),
                FontWeight = active ? FontWeights.SemiBold : FontWeights.Normal,
                VerticalAlignment = VerticalAlignment.Center,
                FontSize = 15,
                IsHitTestVisible = false,
            };
            button.SetContent(text);
            var s = subtype;
            button.Click += (_, _) =>
            {
                _engine.SetActiveSubtype(s);
                Close();
            };
            list.Children.Add(button);
        }
        var manage = new TouchButton { Height = 44, CornerRadius = new CornerRadius(8), Padding = new Thickness(12, 0, 12, 0) };
        manage.SetScheduler(_scheduler);
        manage.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
        var managePanel = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, VerticalAlignment = VerticalAlignment.Center };
        managePanel.Children.Add(IconFactory.Create(KeyIcon.Settings, 18, Brushes.Get(_theme.PopupForeground)));
        managePanel.Children.Add(new TextBlock { Text = KeyboardStrings.ManageLanguages, Foreground = Brushes.Get(_theme.PopupForeground), FontSize = 15, VerticalAlignment = VerticalAlignment.Center });
        manage.SetContent(managePanel);
        manage.Click += (_, _) =>
        {
            Close();
            ManageRequested?.Invoke(this, EventArgs.Empty);
        };
        list.Children.Add(manage);

        var card = new Border
        {
            Background = Brushes.Get(_theme.PopupBackground),
            BorderBrush = Brushes.Get(_theme.PopupBorderColor),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(_theme.PopupCornerRadius + 4),
            Padding = new Thickness(6),
            MinWidth = 260,
            MaxWidth = 420,
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
            Child = new ScrollViewer { Content = list, VerticalScrollBarVisibility = ScrollBarVisibility.Auto, IsTabStop = false },
        };
        card.PointerPressed += (_, e) => e.Handled = true;
        Children.Add(card);
        FocusNeutral.Apply(this);
        Visibility = Visibility.Visible;
    }

    public void Close() => Visibility = Visibility.Collapsed;
}
