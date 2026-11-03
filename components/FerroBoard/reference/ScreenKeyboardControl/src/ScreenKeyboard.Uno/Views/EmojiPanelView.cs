using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using ScreenKeyboard.Controls.Primitives;
using ScreenKeyboard.Controls.Themes;
using ScreenKeyboard.Emoji;
using ScreenKeyboard.Engine;
using ScreenKeyboard.Input;
using ScreenKeyboard.Keys;
using Windows.Foundation;

namespace ScreenKeyboard.Controls.Views;

/// <summary>
/// Emoji and emoticon panel: category tabs, recently used/pinned emojis, skin tone selection on long press,
/// emoticons, search entry and a repeating backspace.
/// </summary>
internal sealed partial class EmojiPanelView : Grid
{
    private const string EmoticonsTab = "emoticons";

    private readonly KeyboardEngine _engine;
    private readonly IKeyboardScheduler _scheduler;
    private readonly ScrollViewer _scroller = new()
    {
        VerticalScrollBarVisibility = ScrollBarVisibility.Hidden,
        HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled,
        VerticalScrollMode = ScrollMode.Enabled,
        HorizontalScrollMode = ScrollMode.Disabled,
        IsTabStop = false,
    };
    private readonly Grid _bottomBar = new();
    private readonly StackPanel _tabs = new() { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Center };
    private readonly Canvas _overlay = new();
    private readonly Dictionary<string, UniformWrapPanel> _pages = new();
    private KeyboardTheme _theme = KeyboardThemes.FlorisDay;
    private string _currentTab = string.Empty;
    private Border? _tonePopup;

    public EmojiPanelView(KeyboardEngine engine, IKeyboardScheduler scheduler)
    {
        _engine = engine;
        _scheduler = scheduler;
        IsTabStop = false;
        RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        Grid.SetRow(_scroller, 0);
        Grid.SetRow(_bottomBar, 1);
        Grid.SetRowSpan(_overlay, 2);
        Children.Add(_scroller);
        Children.Add(_bottomBar);
        Children.Add(_overlay);
        _engine.EmojiHistory.Changed += (_, _) =>
        {
            _pages.Remove(nameof(EmojiCategory.Recent));
        };
        AddHandler(PointerPressedEvent, new Microsoft.UI.Xaml.Input.PointerEventHandler((_, e) =>
        {
            if (_tonePopup is not null && !IsInside(_tonePopup, e.OriginalSource as DependencyObject))
            {
                CloseTonePopup();
            }
        }), handledEventsToo: true);
    }

    /// <summary>Raised when the search button is pressed.</summary>
    public event EventHandler? SearchRequested;

    /// <summary>Height of the bottom bar.</summary>
    public double BarHeight { get; set; } = 44;

    public void SetTheme(KeyboardTheme theme)
    {
        _theme = theme;
        Background = Brushes.Get(theme.PanelBackground);
        _pages.Clear();
        BuildBottomBar();
        FocusNeutral.Apply(this);
        if (_currentTab.Length > 0)
        {
            ShowTab(_currentTab);
        }
    }

    /// <summary>Called when the panel becomes visible.</summary>
    public void OnShown()
    {
        CloseTonePopup();
        if (_currentTab.Length == 0)
        {
            ShowTab(_engine.EmojiHistory.Combined.Count > 0 ? nameof(EmojiCategory.Recent) : nameof(EmojiCategory.SmileysEmotion));
        }
        else if (_currentTab == nameof(EmojiCategory.Recent))
        {
            ShowTab(_currentTab);
        }
    }

    private static KeyIcon IconFor(EmojiCategory category) => category switch
    {
        EmojiCategory.Recent => KeyIcon.Recent,
        EmojiCategory.SmileysEmotion => KeyIcon.Smiley,
        EmojiCategory.PeopleBody => KeyIcon.Person,
        EmojiCategory.AnimalsNature => KeyIcon.Nature,
        EmojiCategory.FoodDrink => KeyIcon.Food,
        EmojiCategory.TravelPlaces => KeyIcon.Travel,
        EmojiCategory.Activities => KeyIcon.Activity,
        EmojiCategory.Objects => KeyIcon.Objects,
        EmojiCategory.Symbols => KeyIcon.Symbols,
        EmojiCategory.Flags => KeyIcon.Flag,
        _ => KeyIcon.Smiley,
    };

    private void BuildBottomBar()
    {
        _bottomBar.Children.Clear();
        _bottomBar.ColumnDefinitions.Clear();
        _bottomBar.Height = BarHeight;
        _bottomBar.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        _bottomBar.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        _bottomBar.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        _bottomBar.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

        var fg = Brushes.Get(_theme.PanelForeground);
        var abc = new TouchButton { Width = BarHeight * 1.4, Margin = new Thickness(4, 4, 2, 4), CornerRadius = new CornerRadius(_theme.KeyCornerRadius) };
        abc.SetScheduler(_scheduler);
        abc.SetColors(_theme.FunctionKeyBackground, _theme.FunctionKeyPressedBackground);
        abc.SetText(KeyboardStrings.Abc, _theme.FunctionKeyFontSize * 0.9, Brushes.Get(_theme.FunctionKeyForeground));
        abc.Click += (_, _) => _engine.SetUiMode(KeyboardUiMode.Text);
        Grid.SetColumn(abc, 0);
        _bottomBar.Children.Add(abc);

        var tabScroller = new ScrollViewer
        {
            HorizontalScrollBarVisibility = ScrollBarVisibility.Hidden,
            VerticalScrollBarVisibility = ScrollBarVisibility.Disabled,
            HorizontalScrollMode = ScrollMode.Enabled,
            VerticalScrollMode = ScrollMode.Disabled,
            IsTabStop = false,
            Content = _tabs,
        };
        Grid.SetColumn(tabScroller, 1);
        _bottomBar.Children.Add(tabScroller);
        BuildTabs();

        var search = new TouchButton { Width = BarHeight, Margin = new Thickness(2, 4, 2, 4), CornerRadius = new CornerRadius(BarHeight / 2) };
        search.SetScheduler(_scheduler);
        search.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
        search.SetIcon(KeyIcon.Search, 20, fg, KeyboardStrings.SearchEmoji);
        search.Click += (_, _) => SearchRequested?.Invoke(this, EventArgs.Empty);
        Grid.SetColumn(search, 2);
        _bottomBar.Children.Add(search);

        var backspace = new TouchButton
        {
            Width = BarHeight * 1.4,
            Margin = new Thickness(2, 4, 4, 4),
            CornerRadius = new CornerRadius(_theme.KeyCornerRadius),
            RepeatDelay = TimeSpan.FromMilliseconds(_engine.Settings.KeyRepeatDelay),
            RepeatInterval = TimeSpan.FromMilliseconds(Math.Max(40, _engine.Settings.KeyRepeatInterval)),
        };
        backspace.SetScheduler(_scheduler);
        backspace.SetColors(_theme.FunctionKeyBackground, _theme.FunctionKeyPressedBackground);
        backspace.SetIcon(KeyIcon.Backspace, 20, Brushes.Get(_theme.FunctionKeyForeground), KeyboardStrings.Backspace);
        backspace.Click += (_, _) => _engine.InputKey(TextKeyDataFactory.Function(KeyCode.Delete));
        Grid.SetColumn(backspace, 3);
        _bottomBar.Children.Add(backspace);
    }

    private void BuildTabs()
    {
        _tabs.Children.Clear();
        var tabs = new List<(string Id, KeyIcon Icon, string Name)> { (nameof(EmojiCategory.Recent), KeyIcon.Recent, KeyboardStrings.Recent) };
        foreach (var category in _engine.EmojiCatalog.Categories)
        {
            tabs.Add((category.ToString(), IconFor(category), category.ToString()));
        }
        tabs.Add((EmoticonsTab, KeyIcon.None, KeyboardStrings.Emoticons));

        var size = Math.Min(BarHeight - 8, 40);
        foreach (var (id, icon, name) in tabs)
        {
            var selected = id == _currentTab;
            var brush = Brushes.Get(selected ? _theme.PanelSelectedForeground : _theme.PanelForeground);
            var button = new TouchButton { Width = size, Height = size, CornerRadius = new CornerRadius(size / 2), Margin = new Thickness(1, 0, 1, 0) };
            button.SetScheduler(_scheduler);
            button.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
            if (icon == KeyIcon.None)
            {
                button.SetText(":-)", 13, brush);
            }
            else
            {
                button.SetIcon(icon, 20, brush, name);
            }
            if (selected)
            {
                button.BorderBrush = brush;
                button.BorderThickness = new Thickness(0, 0, 0, 2);
            }
            var tabId = id;
            button.Click += (_, _) => ShowTab(tabId);
            _tabs.Children.Add(button);
        }
        FocusNeutral.Apply(_tabs);
    }

    private void ShowTab(string id)
    {
        CloseTonePopup();
        var changed = id != _currentTab;
        _currentTab = id;
        if (!_pages.TryGetValue(id, out var page))
        {
            page = BuildPage(id);
            _pages[id] = page;
        }
        _scroller.Content = FocusNeutral.Apply(page);
        if (changed)
        {
            _scroller.ChangeView(null, 0, null, disableAnimation: true);
            BuildTabs();
        }
    }

    private UniformWrapPanel BuildPage(string id)
    {
        var cell = Math.Max(40, _theme.EmojiFontSize * 1.6);
        var page = new UniformWrapPanel { ItemWidth = cell, ItemHeight = cell, Margin = new Thickness(4) };
        var fg = Brushes.Get(_theme.PanelForeground);

        if (id == EmoticonsTab)
        {
            page.ItemWidth = cell * 2;
            foreach (var emoticon in EmojiCatalog.LoadEmoticons())
            {
                var button = CreateCell(emoticon.Text, _theme.SmartbarFontSize, fg);
                var text = emoticon.Text;
                button.Click += (_, _) => _engine.InputText(text);
                page.Children.Add(button);
            }
            return page;
        }

        IEnumerable<(string Emoji, EmojiInfo? Info)> items;
        if (id == nameof(EmojiCategory.Recent))
        {
            items = _engine.EmojiHistory.Combined.Select(e => (e, _engine.EmojiCatalog.Get(e)));
        }
        else if (Enum.TryParse<EmojiCategory>(id, out var category))
        {
            items = _engine.EmojiCatalog.GetEmojis(category).Select(e => (DefaultVariant(e), (EmojiInfo?)e));
        }
        else
        {
            items = [];
        }

        foreach (var (emoji, info) in items)
        {
            var button = CreateCell(emoji, _theme.EmojiFontSize, fg);
            var value = emoji;
            button.Click += (_, _) => _engine.InputEmoji(value);
            if (info is { HasSkinTones: true } || id == nameof(EmojiCategory.Recent))
            {
                var inf = info;
                var btn = button;
                button.LongPress += (_, _) => OnEmojiLongPress(btn, value, inf);
            }
            page.Children.Add(button);
        }
        if (page.Children.Count == 0)
        {
            page.ItemWidth = 400;
            page.Children.Add(new TextBlock
            {
                Text = KeyboardStrings.RecentEmojiEmpty,
                Foreground = Brushes.Get(_theme.KeyHintForeground),
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
                Margin = new Thickness(0, 24, 0, 0),
            });
        }
        return page;
    }

    private string DefaultVariant(EmojiInfo info)
    {
        if (!info.HasSkinTones)
        {
            return info.Value;
        }
        var tone = _engine.EmojiHistory.GetPreferredTone(info.Value) ?? _engine.Settings.EmojiSkinTone;
        return info.WithSkinTone(tone);
    }

    private TouchButton CreateCell(string text, double fontSize, Brush fg)
    {
        var button = new TouchButton { CornerRadius = new CornerRadius(8) };
        button.SetScheduler(_scheduler);
        button.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
        button.SetText(text, fontSize, fg);
        return button;
    }

    private void OnEmojiLongPress(TouchButton anchor, string emoji, EmojiInfo? info)
    {
        CloseTonePopup();
        var variants = new List<string>();
        if (info is { HasSkinTones: true })
        {
            variants.Add(info.Value);
            variants.AddRange(info.Variants.Take(5));
        }
        var isRecent = _currentTab == nameof(EmojiCategory.Recent);
        if (variants.Count == 0 && !isRecent)
        {
            return;
        }

        var cell = anchor.ActualHeight;
        var row = new StackPanel { Orientation = Orientation.Horizontal };
        foreach (var variant in variants)
        {
            var button = CreateCell(variant, _theme.EmojiFontSize, Brushes.Get(_theme.PopupForeground));
            button.Width = cell;
            button.Height = cell;
            var v = variant;
            button.Click += (_, _) =>
            {
                CloseTonePopup();
                _engine.InputEmoji(v);
                _pages.Remove(_currentTab);
                if (_currentTab != nameof(EmojiCategory.Recent))
                {
                    // Re-render the category with the new preferred tone.
                    ShowTab(_currentTab);
                }
            };
            row.Children.Add(button);
        }
        if (isRecent)
        {
            var pinned = _engine.EmojiHistory.Pinned.Contains(emoji);
            var pin = new TouchButton { Width = cell, Height = cell, CornerRadius = new CornerRadius(8) };
            pin.SetScheduler(_scheduler);
            pin.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
            pin.SetIcon(pinned ? KeyIcon.Unpin : KeyIcon.Pin, 20, Brushes.Get(_theme.PopupForeground), pinned ? KeyboardStrings.Unpin : KeyboardStrings.Pin);
            pin.Click += (_, _) =>
            {
                CloseTonePopup();
                if (pinned) _engine.EmojiHistory.Unpin(emoji); else _engine.EmojiHistory.Pin(emoji);
                ShowTab(nameof(EmojiCategory.Recent));
            };
            row.Children.Add(pin);
            var remove = new TouchButton { Width = cell, Height = cell, CornerRadius = new CornerRadius(8) };
            remove.SetScheduler(_scheduler);
            remove.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
            remove.SetIcon(KeyIcon.Delete, 20, Brushes.Get(_theme.PopupForeground), KeyboardStrings.Remove);
            remove.Click += (_, _) =>
            {
                CloseTonePopup();
                _engine.EmojiHistory.Remove(emoji);
                _engine.EmojiHistory.Unpin(emoji);
                ShowTab(nameof(EmojiCategory.Recent));
            };
            row.Children.Add(remove);
        }

        _tonePopup = new Border
        {
            Child = row,
            Background = Brushes.Get(_theme.PopupBackground),
            BorderBrush = Brushes.Get(_theme.PopupBorderColor),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(_theme.PopupCornerRadius),
            Padding = new Thickness(2),
        };
        var position = anchor.TransformToVisual(this).TransformPoint(new Point(0, 0));
        var width = row.Children.Count * cell + 6;
        var x = Math.Clamp(position.X + anchor.ActualWidth / 2 - width / 2, 0, Math.Max(0, ActualWidth - width));
        var y = Math.Max(0, position.Y - cell - 8);
        Canvas.SetLeft(_tonePopup, x);
        Canvas.SetTop(_tonePopup, y);
        _overlay.Children.Add(FocusNeutral.Apply(_tonePopup));
    }

    private static bool IsInside(DependencyObject container, DependencyObject? element)
    {
        while (element is not null)
        {
            if (ReferenceEquals(element, container))
            {
                return true;
            }
            element = VisualTreeHelper.GetParent(element);
        }
        return false;
    }

    private void CloseTonePopup()
    {
        if (_tonePopup is not null)
        {
            _overlay.Children.Remove(_tonePopup);
            _tonePopup = null;
        }
    }
}
