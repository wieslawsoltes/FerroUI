using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using ScreenKeyboard.Controls.Icons;
using ScreenKeyboard.Controls.Primitives;
using ScreenKeyboard.Controls.Themes;
using ScreenKeyboard.Engine;
using ScreenKeyboard.Input;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Settings;
using ScreenKeyboard.Text;

namespace ScreenKeyboard.Controls.Views;

/// <summary>A quick action shown in the smartbar.</summary>
public sealed record SmartbarAction(string Id, KeyIcon Icon, string Name, Action<KeyboardEngine> Execute, Func<KeyboardEngine, bool>? IsActive = null);

/// <summary>
/// The bar above the keys: word suggestions (with auto-correct highlight), clipboard suggestion, glide preview,
/// quick actions and the emoji search strip.
/// </summary>
internal sealed partial class SmartbarView : Grid
{
    private readonly KeyboardEngine _engine;
    private readonly IKeyboardScheduler _scheduler;
    private readonly TouchButton _toggle = new();
    private readonly TouchButton _hide = new();
    private readonly Grid _center = new();
    private readonly Grid _suggestions = new();
    private readonly ScrollViewer _actionsScroller = new()
    {
        HorizontalScrollBarVisibility = ScrollBarVisibility.Hidden,
        VerticalScrollBarVisibility = ScrollBarVisibility.Disabled,
        HorizontalScrollMode = ScrollMode.Enabled,
        VerticalScrollMode = ScrollMode.Disabled,
        IsTabStop = false,
    };
    private readonly StackPanel _actions = new() { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Center };
    private readonly Grid _search = new();
    private KeyboardTheme _theme = KeyboardThemes.FlorisDay;
    private bool _actionsExpanded;
    private double _height = 44;

    public SmartbarView(KeyboardEngine engine, IKeyboardScheduler scheduler)
    {
        _engine = engine;
        _scheduler = scheduler;
        IsTabStop = false;
        ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

        _toggle.SetScheduler(scheduler);
        _toggle.Click += (_, _) =>
        {
            if (EmojiSearch is not null)
            {
                SearchBackRequested?.Invoke(this, EventArgs.Empty);
                return;
            }
            _actionsExpanded = !_actionsExpanded;
            Refresh();
        };
        _hide.SetScheduler(scheduler);
        _hide.Click += (_, _) => HideRequested?.Invoke(this, EventArgs.Empty);
        Grid.SetColumn(_toggle, 0);
        Grid.SetColumn(_center, 1);
        Grid.SetColumn(_hide, 2);
        _actionsScroller.Content = _actions;
        _center.Children.Add(_suggestions);
        _center.Children.Add(_actionsScroller);
        _center.Children.Add(_search);
        Children.Add(_toggle);
        Children.Add(_center);
        Children.Add(_hide);

        Actions = CreateDefaultActions();
    }

    /// <summary>Raised when the hide button is pressed.</summary>
    public event EventHandler? HideRequested;

    /// <summary>Raised when the back button of the emoji search strip is pressed.</summary>
    public event EventHandler? SearchBackRequested;

    /// <summary>Quick actions shown when expanded (or when no suggestions are available).</summary>
    public List<SmartbarAction> Actions { get; set; }

    /// <summary>Emoji search state (set by the keyboard control).</summary>
    public EmojiSearchState? EmojiSearch { get; set; }

    public void SetTheme(KeyboardTheme theme, double height)
    {
        _theme = theme;
        _height = height;
        Height = height;
        Background = Brushes.Get(theme.SmartbarBackground);
        Refresh();
    }

    private static List<SmartbarAction> CreateDefaultActions() =>
    [
        new("undo", KeyIcon.Undo, KeyboardStrings.Undo, e => e.InputKey(TextKeyDataFactory.Function(KeyCode.Undo))),
        new("redo", KeyIcon.Redo, KeyboardStrings.Redo, e => e.InputKey(TextKeyDataFactory.Function(KeyCode.Redo))),
        new("emoji", KeyIcon.Emoji, KeyboardStrings.Emoji, e => e.SetUiMode(KeyboardUiMode.Media)),
        new("clipboard", KeyIcon.Clipboard, KeyboardStrings.Clipboard, e => e.SetUiMode(KeyboardUiMode.Clipboard)),
        new("editing", KeyIcon.TextEditing, KeyboardStrings.TextEditing, e => e.SetUiMode(KeyboardUiMode.Editing)),
        new("numpad", KeyIcon.Numpad, KeyboardStrings.NumberPad, e => e.SetMode(KeyboardMode.NumericAdvanced)),
        new("onehanded", KeyIcon.OneHandedRight, KeyboardStrings.OneHandedMode,
            e => e.Settings.OneHandedMode = e.Settings.OneHandedMode == OneHandedMode.Off ? OneHandedMode.Right : OneHandedMode.Off,
            e => e.Settings.OneHandedMode != OneHandedMode.Off),
        new("split", KeyIcon.SplitKeyboard, KeyboardStrings.SplitKeyboard, e => e.Settings.SplitKeyboard = !e.Settings.SplitKeyboard, e => e.Settings.SplitKeyboard),
        new("floating", KeyIcon.Floating, KeyboardStrings.FloatingKeyboard, e => e.Settings.Floating = !e.Settings.Floating, e => e.Settings.Floating),
        new("resize", KeyIcon.ResizeHeight, KeyboardStrings.ResizeKeyboard, e => e.InputKey(TextKeyDataFactory.Function(KeyCode.ToggleResizeMode))),
        new("incognito", KeyIcon.Incognito, KeyboardStrings.Incognito, e => e.Settings.IncognitoMode = !e.Settings.IncognitoMode, e => e.Settings.IncognitoMode),
        new("autocorrect", KeyIcon.Autocorrect, KeyboardStrings.AutoCorrect, e => e.Settings.AutoCorrect = !e.Settings.AutoCorrect, e => e.Settings.AutoCorrect),
        new("language", KeyIcon.Language, KeyboardStrings.Languages, e => e.InputKey(TextKeyDataFactory.Function(KeyCode.ShowSubtypePicker))),
        new("settings", KeyIcon.Settings, KeyboardStrings.Settings, e => e.InputKey(TextKeyDataFactory.Function(KeyCode.Settings))),
    ];

    public void Refresh()
    {
        var theme = _theme;
        var fg = Brushes.Get(theme.SmartbarForeground);
        var iconSize = Math.Min(22, _height * 0.5);
        var buttonWidth = Math.Max(40, _height);

        if (EmojiSearch is { } search)
        {
            _toggle.Width = buttonWidth;
            _toggle.SetIcon(KeyIcon.ChevronLeft, iconSize, fg, KeyboardStrings.Back);
            _toggle.SetColors(theme.ActionButtonBackground, theme.ActionButtonPressedBackground);
            _hide.Visibility = Visibility.Collapsed;
            _suggestions.Visibility = Visibility.Collapsed;
            _actionsScroller.Visibility = Visibility.Collapsed;
            _search.Visibility = Visibility.Visible;
            BuildSearch(search, fg);
            FocusNeutral.Apply(this);
            return;
        }

        _search.Visibility = Visibility.Collapsed;
        _hide.Visibility = Visibility.Visible;
        _hide.Width = buttonWidth;
        _hide.SetIcon(KeyIcon.HideKeyboard, iconSize, fg, KeyboardStrings.HideKeyboard);
        _hide.SetColors(theme.ActionButtonBackground, theme.ActionButtonPressedBackground);

        var suggestions = _engine.Suggestions;
        var layout = _engine.Settings.SmartbarLayout;
        var showActions = layout == SmartbarLayout.ActionsOnly || _actionsExpanded ||
                          layout == SmartbarLayout.SuggestionsWithActions && suggestions.Count == 0;
        if (layout == SmartbarLayout.SuggestionsOnly)
        {
            showActions = false;
        }

        _toggle.Visibility = layout == SmartbarLayout.SuggestionsWithActions ? Visibility.Visible : Visibility.Collapsed;
        _toggle.Width = buttonWidth;
        _toggle.SetIcon(showActions && _actionsExpanded ? KeyIcon.ChevronLeft : KeyIcon.ChevronRight, iconSize, fg, KeyboardStrings.QuickActions);
        _toggle.SetColors(theme.ActionButtonBackground, theme.ActionButtonPressedBackground);

        _suggestions.Visibility = showActions ? Visibility.Collapsed : Visibility.Visible;
        _actionsScroller.Visibility = showActions ? Visibility.Visible : Visibility.Collapsed;
        if (showActions)
        {
            BuildActions(fg, iconSize, buttonWidth);
        }
        else
        {
            BuildSuggestions(suggestions);
        }
        FocusNeutral.Apply(this);
    }

    private void BuildActions(Brush fg, double iconSize, double buttonWidth)
    {
        _actions.Children.Clear();
        foreach (var action in Actions)
        {
            var button = new TouchButton { Width = buttonWidth, Height = _height, CornerRadius = new CornerRadius(_height / 2) };
            button.SetScheduler(_scheduler);
            var active = action.IsActive?.Invoke(_engine) == true;
            button.SetIcon(action.Icon, iconSize, active ? Brushes.Get(_theme.SuggestionHighlightForeground) : fg, action.Name);
            button.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
            var a = action;
            button.Click += (_, _) =>
            {
                a.Execute(_engine);
                _actionsExpanded = false;
                Refresh();
            };
            _actions.Children.Add(button);
        }
    }

    private void BuildSuggestions(IReadOnlyList<Suggestion> suggestions)
    {
        _suggestions.Children.Clear();
        _suggestions.ColumnDefinitions.Clear();
        var count = suggestions.Count;
        if (count == 0)
        {
            return;
        }
        var fontFamily = _theme.FontFamily is { } ff ? new FontFamily(ff) : null;
        for (var i = 0; i < count; i++)
        {
            if (i > 0)
            {
                _suggestions.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1) });
                var divider = new Border
                {
                    Background = Brushes.Get(_theme.DividerColor),
                    Margin = new Thickness(0, _height * 0.28, 0, _height * 0.28),
                    IsHitTestVisible = false,
                };
                Grid.SetColumn(divider, _suggestions.ColumnDefinitions.Count - 1);
                _suggestions.Children.Add(divider);
            }
            _suggestions.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            var suggestion = suggestions[i];
            var button = new TouchButton { Margin = new Thickness(2, 3, 2, 3), CornerRadius = new CornerRadius(8) };
            button.SetScheduler(_scheduler);
            button.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
            var highlight = suggestion.IsAutoCommit;
            var foreground = Brushes.Get(highlight ? _theme.SuggestionHighlightForeground : _theme.SmartbarForeground);
            if (suggestion.Kind == SuggestionKind.Clipboard)
            {
                var panel = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Center, Spacing = 6 };
                panel.Children.Add(IconFactory.Create(KeyIcon.Paste, 16, foreground));
                panel.Children.Add(new TextBlock
                {
                    Text = suggestion.Text.Replace('\n', ' '),
                    FontSize = _theme.SmartbarFontSize * 0.9,
                    Foreground = foreground,
                    VerticalAlignment = VerticalAlignment.Center,
                    TextTrimming = TextTrimming.CharacterEllipsis,
                    MaxWidth = 180,
                });
                button.SetContent(panel);
                Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(button, string.Format(KeyboardStrings.PasteFormat, suggestion.Text));
            }
            else
            {
                var text = button.SetText(suggestion.Text, suggestion.Kind == SuggestionKind.Emoji ? _theme.SmartbarFontSize * 1.3 : _theme.SmartbarFontSize, foreground, fontFamily);
                if (highlight)
                {
                    text.FontWeight = FontWeights.SemiBold;
                }
            }
            button.Click += (_, _) => _engine.CommitSuggestion(suggestion);
            if (suggestion.IsRemovable)
            {
                button.LongPress += (_, _) => _engine.RemoveSuggestion(suggestion);
            }
            Grid.SetColumn(button, _suggestions.ColumnDefinitions.Count - 1);
            _suggestions.Children.Add(button);
        }
    }

    private void BuildSearch(EmojiSearchState search, Brush fg)
    {
        _search.Children.Clear();
        _search.ColumnDefinitions.Clear();
        _search.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(0.4, GridUnitType.Star) });
        _search.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });

        var queryBorder = new Border
        {
            Background = Brushes.Get(_theme.KeyBackground),
            CornerRadius = new CornerRadius(_height / 2),
            Margin = new Thickness(0, 5, 6, 5),
            Padding = new Thickness(12, 0, 12, 0),
        };
        var queryPanel = new StackPanel { Orientation = Orientation.Horizontal, VerticalAlignment = VerticalAlignment.Center, Spacing = 6 };
        queryPanel.Children.Add(IconFactory.Create(KeyIcon.Search, 16, Brushes.Get(_theme.KeyHintForeground)));
        queryPanel.Children.Add(new TextBlock
        {
            Text = search.Query.Length == 0 ? KeyboardStrings.SearchEmoji : search.Query + "|",
            Foreground = search.Query.Length == 0 ? Brushes.Get(_theme.KeyHintForeground) : Brushes.Get(_theme.KeyForeground),
            FontSize = _theme.SmartbarFontSize * 0.9,
            VerticalAlignment = VerticalAlignment.Center,
            TextTrimming = TextTrimming.CharacterEllipsis,
        });
        queryBorder.Child = queryPanel;
        _search.Children.Add(queryBorder);

        var scroller = new ScrollViewer
        {
            HorizontalScrollBarVisibility = ScrollBarVisibility.Hidden,
            VerticalScrollBarVisibility = ScrollBarVisibility.Disabled,
            HorizontalScrollMode = ScrollMode.Enabled,
            VerticalScrollMode = ScrollMode.Disabled,
            IsTabStop = false,
        };
        var results = new StackPanel { Orientation = Orientation.Horizontal };
        foreach (var emoji in search.Results)
        {
            var button = new TouchButton { Width = _height, Height = _height, CornerRadius = new CornerRadius(8) };
            button.SetScheduler(_scheduler);
            button.SetColors(_theme.ActionButtonBackground, _theme.ActionButtonPressedBackground);
            button.SetText(emoji, _theme.SmartbarFontSize * 1.3, fg);
            var e = emoji;
            button.Click += (_, _) => search.Select(e);
            results.Children.Add(button);
        }
        if (search.Results.Count == 0 && search.Query.Length > 0)
        {
            results.Children.Add(new TextBlock { Text = KeyboardStrings.NoEmojiFound, Foreground = Brushes.Get(_theme.KeyHintForeground), VerticalAlignment = VerticalAlignment.Center, Margin = new Thickness(8, 0, 0, 0) });
        }
        scroller.Content = results;
        Grid.SetColumn(scroller, 1);
        _search.Children.Add(scroller);
    }

}

/// <summary>State of the emoji search mode.</summary>
internal sealed class EmojiSearchState
{
    public string Query { get; set; } = string.Empty;
    public IReadOnlyList<string> Results { get; set; } = [];
    public required Action<string> Select { get; init; }
}

/// <summary>Helpers to create key data.</summary>
internal static class TextKeyDataFactory
{
    public static Layouts.TextKeyData Function(int code) => Layouts.TextKeyData.Function(code, code == KeyCode.Delete ? KeyType.EnterEditing : KeyType.Function);
}
