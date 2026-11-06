using System.ComponentModel;
using Microsoft.UI.Text;
using ScreenKeyboard.Controls;
using ScreenKeyboard.Controls.Themes;
using ScreenKeyboard.Engine;
using ScreenKeyboard.Emoji;
using ScreenKeyboard.Layouts;
using ScreenKeyboard.Settings;

namespace ScreenKeyboard.Sample;

/// <summary>
/// A settings UI bound to <see cref="KeyboardSettings"/>. Built in code to show how every option can be changed
/// at runtime.
/// </summary>
public sealed partial class SettingsPanel : UserControl
{
    private readonly StackPanel _panel = new() { Spacing = 10, Padding = new Thickness(16, 12, 16, 24) };
    private readonly List<Action> _refreshers = new();
    private OnScreenKeyboard? _keyboard;
    private bool _refreshing;

    public SettingsPanel()
    {
        Content = new ScrollViewer { Content = _panel, VerticalScrollBarVisibility = ScrollBarVisibility.Auto };
    }

    private KeyboardSettings Settings => _keyboard!.Engine.Settings;

    private KeyboardEngine Engine => _keyboard!.Engine;

    public void Attach(OnScreenKeyboard keyboard)
    {
        _keyboard = keyboard;
        Build();
        Settings.PropertyChanged += OnSettingsChanged;
    }

    private void OnSettingsChanged(object? sender, PropertyChangedEventArgs e)
    {
        _refreshing = true;
        try
        {
            foreach (var refresh in _refreshers)
            {
                refresh();
            }
        }
        finally
        {
            _refreshing = false;
        }
    }

    private void Build()
    {
        _panel.Children.Clear();
        _refreshers.Clear();

        Header("Appearance");
        Combo("Theme mode", Enum.GetValues<ThemeMode>().Select(v => (v.ToString(), v)), () => Settings.ThemeMode, v => Settings.ThemeMode = v);
        var themes = KeyboardThemes.All.OrderBy(t => t.Name).Select(t => (t.Name, t.Id)).ToList();
        Combo("Light theme", themes, () => Settings.LightTheme, v => Settings.LightTheme = v);
        Combo("Dark theme", themes, () => Settings.DarkTheme, v => Settings.DarkTheme = v);
        Slider("Key height", 36, 90, 1, () => Settings.KeyHeight, v => Settings.KeyHeight = v);
        Slider("Font scale", 0.6, 1.6, 0.05, () => Settings.FontScale, v => Settings.FontScale = v);
        Slider("Horizontal key spacing", 0, 16, 1, () => Settings.KeySpacingHorizontal, v => Settings.KeySpacingHorizontal = v);
        Slider("Vertical key spacing", 0, 24, 1, () => Settings.KeySpacingVertical, v => Settings.KeySpacingVertical = v);
        Combo("One-handed mode", Enum.GetValues<OneHandedMode>().Select(v => (v.ToString(), v)), () => Settings.OneHandedMode, v => Settings.OneHandedMode = v);
        Toggle("Split keyboard", () => Settings.SplitKeyboard, v => Settings.SplitKeyboard = v);
        Toggle("Number row", () => Settings.NumberRow, v => Settings.NumberRow = v);
        Toggle("Smartbar", () => Settings.SmartbarEnabled, v => Settings.SmartbarEnabled = v);
        Combo("Smartbar layout", Enum.GetValues<SmartbarLayout>().Select(v => (v.ToString(), v)), () => Settings.SmartbarLayout, v => Settings.SmartbarLayout = v);
        Toggle("Key press preview", () => Settings.ShowKeyPreview, v => Settings.ShowKeyPreview = v);
        Combo("Space bar label", Enum.GetValues<SpaceBarMode>().Select(v => (v.ToString(), v)), () => Settings.SpaceBarMode, v => Settings.SpaceBarMode = v);
        Combo("Utility key", Enum.GetValues<UtilityKeyAction>().Select(v => (v.ToString(), v)), () => Settings.UtilityKeyAction, v => Settings.UtilityKeyAction = v);
        Combo("Long press priority", Enum.GetValues<KeyHintMode>().Select(v => (v.ToString(), v)), () => Settings.HintMode, v => Settings.HintMode = v);
        Toggle("Number hints", () => Settings.HintedNumberRow, v => Settings.HintedNumberRow = v);
        Toggle("Symbol hints", () => Settings.HintedSymbols, v => Settings.HintedSymbols = v);

        Header("Languages");
        BuildLanguages();

        Header("Typing");
        Toggle("Auto-capitalization", () => Settings.AutoCapitalization, v => Settings.AutoCapitalization = v);
        Toggle("Double space inserts period", () => Settings.DoubleSpacePeriod, v => Settings.DoubleSpacePeriod = v);
        Toggle("Smart punctuation spacing", () => Settings.AutoSpacePunctuation, v => Settings.AutoSpacePunctuation = v);
        Toggle("Show suggestions", () => Settings.ShowSuggestions, v => Settings.ShowSuggestions = v);
        Toggle("Auto-correct", () => Settings.AutoCorrect, v => Settings.AutoCorrect = v);
        Toggle("Next word prediction", () => Settings.NextWordPrediction, v => Settings.NextWordPrediction = v);
        Toggle("Learn new words", () => Settings.LearnWords, v => Settings.LearnWords = v);
        Toggle("Emoji suggestions", () => Settings.EmojiSuggestions, v => Settings.EmojiSuggestions = v);
        Toggle("Incognito mode", () => Settings.IncognitoMode, v => Settings.IncognitoMode = v);
        Slider("Suggestion count", 1, 5, 1, () => Settings.SuggestionCount, v => Settings.SuggestionCount = (int)v);

        Header("Gestures");
        Toggle("Glide typing", () => Settings.GlideTyping, v => Settings.GlideTyping = v);
        Toggle("Show glide trail", () => Settings.GlideShowTrail, v => Settings.GlideShowTrail = v);
        Toggle("Live glide preview", () => Settings.GlidePreview, v => Settings.GlidePreview = v);
        var actions = Enum.GetValues<SwipeAction>().Select(v => (v.ToString(), v)).ToList();
        Combo("Swipe up", actions, () => Settings.SwipeUp, v => Settings.SwipeUp = v);
        Combo("Swipe down", actions, () => Settings.SwipeDown, v => Settings.SwipeDown = v);
        Combo("Swipe left", actions, () => Settings.SwipeLeft, v => Settings.SwipeLeft = v);
        Combo("Swipe right", actions, () => Settings.SwipeRight, v => Settings.SwipeRight = v);
        Combo("Space bar swipe up", actions, () => Settings.SpaceBarSwipeUp, v => Settings.SpaceBarSwipeUp = v);
        Combo("Space bar long press", actions, () => Settings.SpaceBarLongPress, v => Settings.SpaceBarLongPress = v);
        Combo("Delete key swipe left", actions, () => Settings.DeleteKeySwipeLeft, v => Settings.DeleteKeySwipeLeft = v);
        Slider("Long press delay (ms)", 150, 1000, 10, () => Settings.LongPressDelay, v => Settings.LongPressDelay = (int)v);

        Header("Emoji, clipboard & feedback");
        Combo("Default skin tone", Enum.GetValues<EmojiSkinTone>().Select(v => (v.ToString(), v)), () => Settings.EmojiSkinTone, v => Settings.EmojiSkinTone = v);
        Toggle("Clipboard history", () => Settings.ClipboardHistoryEnabled, v => Settings.ClipboardHistoryEnabled = v);
        Toggle("Suggest recently copied text", () => Settings.ClipboardSuggestion, v => Settings.ClipboardSuggestion = v);
        Toggle("Haptic feedback", () => Settings.HapticFeedback, v => Settings.HapticFeedback = v);

        Header("Data");
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        var reset = new Button { Content = "Reset settings" };
        reset.Click += (_, _) => Settings.CopyFrom(new KeyboardSettings());
        var forget = new Button { Content = "Clear learned words" };
        forget.Click += (_, _) =>
        {
            foreach (var dictionary in Engine.LanguageModels.UserDictionaries.Values)
            {
                dictionary.Clear();
            }
        };
        buttons.Children.Add(reset);
        buttons.Children.Add(forget);
        _panel.Children.Add(buttons);

        _panel.Children.Add(new TextBlock
        {
            Text = "Layouts, popup mappings and emoji data come from FlorisBoard (Apache-2.0).",
            Opacity = 0.6,
            TextWrapping = TextWrapping.Wrap,
            FontSize = 12,
            Margin = new Thickness(0, 12, 0, 0),
        });

        OnSettingsChanged(this, new PropertyChangedEventArgs(null));
    }

    private void BuildLanguages()
    {
        var list = new StackPanel { Spacing = 0 };
        var filter = new TextBox { PlaceholderText = "Filter languages", Margin = new Thickness(0, 0, 0, 4) };
        var scroller = new ScrollViewer { Content = list, MaxHeight = 260 };
        var presets = Engine.Resources.SubtypePresets
            .Select(s => (Subtype: s, Name: $"{KeyboardEngine.GetSubtypeDisplayName(s)} ({s.LanguageTag}) — {Engine.Resources.GetLayout(LayoutType.Characters, s.Layouts.Characters)?.Label ?? s.Layouts.Characters.ComponentId}"))
            .OrderBy(p => p.Name, StringComparer.CurrentCultureIgnoreCase)
            .ToList();

        void Populate()
        {
            list.Children.Clear();
            var text = filter.Text?.Trim() ?? string.Empty;
            foreach (var (subtype, name) in presets)
            {
                if (text.Length > 0 && !name.Contains(text, StringComparison.CurrentCultureIgnoreCase))
                {
                    continue;
                }
                var check = new CheckBox { Content = name, IsChecked = Settings.Subtypes.Contains(subtype.Id) };
                var id = subtype.Id;
                check.Checked += (_, _) =>
                {
                    if (!Settings.Subtypes.Contains(id))
                    {
                        Settings.Subtypes = Settings.Subtypes.Append(id).ToList();
                    }
                };
                check.Unchecked += (_, _) =>
                {
                    var remaining = Settings.Subtypes.Where(s => s != id).ToList();
                    Settings.Subtypes = remaining.Count > 0 ? remaining : ["en-US/qwerty"];
                };
                list.Children.Add(check);
            }
        }

        filter.TextChanged += (_, _) => Populate();
        Populate();
        _panel.Children.Add(new TextBlock { Text = "Enable languages; switch with the globe key, a swipe or a long press on space.", TextWrapping = TextWrapping.Wrap, Opacity = 0.7, FontSize = 12 });
        _panel.Children.Add(filter);
        _panel.Children.Add(scroller);
    }

    // ------------------------------------------------------------------ helpers

    private void Header(string text) => _panel.Children.Add(new TextBlock
    {
        Text = text,
        FontSize = 18,
        FontWeight = FontWeights.SemiBold,
        Margin = new Thickness(0, 14, 0, 0),
    });

    private void Toggle(string header, Func<bool> get, Action<bool> set)
    {
        var toggle = new ToggleSwitch { Header = header };
        toggle.Toggled += (_, _) =>
        {
            if (!_refreshing)
            {
                set(toggle.IsOn);
            }
        };
        _refreshers.Add(() => toggle.IsOn = get());
        _panel.Children.Add(toggle);
    }

    private void Slider(string header, double min, double max, double step, Func<double> get, Action<double> set)
    {
        var slider = new Slider { Header = header, Minimum = min, Maximum = max, StepFrequency = step };
        slider.ValueChanged += (_, e) =>
        {
            if (!_refreshing)
            {
                set(e.NewValue);
            }
        };
        _refreshers.Add(() => slider.Value = get());
        _panel.Children.Add(slider);
    }

    private void Combo<T>(string header, IEnumerable<(string Name, T Value)> items, Func<T> get, Action<T> set)
    {
        var list = items.ToList();
        var combo = new ComboBox { Header = header, HorizontalAlignment = HorizontalAlignment.Stretch };
        foreach (var (name, _) in list)
        {
            combo.Items.Add(name);
        }
        combo.SelectionChanged += (_, _) =>
        {
            if (!_refreshing && combo.SelectedIndex >= 0)
            {
                set(list[combo.SelectedIndex].Value);
            }
        };
        _refreshers.Add(() => combo.SelectedIndex = list.FindIndex(i => EqualityComparer<T>.Default.Equals(i.Value, get())));
        _panel.Children.Add(combo);
    }
}
