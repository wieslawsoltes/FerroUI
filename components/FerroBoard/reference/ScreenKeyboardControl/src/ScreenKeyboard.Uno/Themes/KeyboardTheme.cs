using System.Globalization;
using System.Text;
using System.Text.Json;
using ScreenKeyboard.Keys;
using Windows.UI;

namespace ScreenKeyboard.Controls.Themes;

/// <summary>
/// Overrides the style of specific keys (e.g. the enter key or all function keys). Rules are applied in order;
/// every non-null property of a matching rule overrides the theme default.
/// </summary>
public sealed class KeyStyleRule
{
    /// <summary>Matches keys with one of these codes (empty = any).</summary>
    public List<int> Codes { get; set; } = new();

    /// <summary>Matches keys of this type.</summary>
    public KeyType? Type { get; set; }

    /// <summary>Matches only while the keyboard is in this mode.</summary>
    public KeyboardMode? Mode { get; set; }

    /// <summary>Matches only while the shift state is this value.</summary>
    public ShiftState? ShiftState { get; set; }

    public Color? Background { get; set; }
    public Color? PressedBackground { get; set; }
    public Color? Foreground { get; set; }
    public double? FontSize { get; set; }
    public double? CornerRadius { get; set; }

    /// <summary>Returns <c>true</c> if the rule applies to the key.</summary>
    public bool Matches(int code, KeyType type, KeyboardMode mode, ShiftState shift) =>
        (Codes.Count == 0 || Codes.Contains(code)) &&
        (Type is null || Type == type) &&
        (Mode is null || Mode == mode) &&
        (ShiftState is null || ShiftState == shift);
}

/// <summary>
/// Visual theme of the keyboard. All colors can be changed at runtime; themes can be created in code, in XAML
/// (colors as <c>#AARRGGBB</c> strings) or loaded from JSON (<see cref="FromJson"/>).
/// </summary>
public sealed class KeyboardTheme
{
    /// <summary>Unique id (e.g. <c>floris_day</c>).</summary>
    public string Id { get; set; } = "custom";

    /// <summary>Display name.</summary>
    public string Name { get; set; } = "Custom";

    /// <summary>Whether this is a dark theme.</summary>
    public bool IsDark { get; set; }

    // Surfaces
    public Color Background { get; set; } = Hex("#E0E0E0");
    public Color KeyBackground { get; set; } = Hex("#FFFFFF");
    public Color KeyPressedBackground { get; set; } = Hex("#F5F5F5");
    public Color KeyForeground { get; set; } = Hex("#000000");
    public Color KeyHintForeground { get; set; } = Hex("#5F5F5F");
    public Color FunctionKeyBackground { get; set; } = Hex("#FFFFFF");
    public Color FunctionKeyPressedBackground { get; set; } = Hex("#F5F5F5");
    public Color FunctionKeyForeground { get; set; } = Hex("#000000");
    public Color AccentKeyBackground { get; set; } = Hex("#4CAF50");
    public Color AccentKeyPressedBackground { get; set; } = Hex("#388E3C");
    public Color AccentKeyForeground { get; set; } = Hex("#FFFFFF");
    public Color SpaceKeyForeground { get; set; } = Hex("#5F5F5F");
    public Color CapsLockForeground { get; set; } = Hex("#FF9800");
    public Color KeyBorderColor { get; set; } = Hex("#00000000");
    public double KeyBorderThickness { get; set; }
    public Color KeyShadowColor { get; set; } = Hex("#33000000");
    public double KeyShadowDepth { get; set; } = 1.5;
    public double KeyCornerRadius { get; set; } = 8;

    // Popups & preview
    public Color PopupBackground { get; set; } = Hex("#EEEEEE");
    public Color PopupForeground { get; set; } = Hex("#000000");
    public Color PopupSelectedBackground { get; set; } = Hex("#BDBDBD");
    public Color PopupSelectedForeground { get; set; } = Hex("#000000");
    public Color PopupBorderColor { get; set; } = Hex("#1F000000");
    public double PopupCornerRadius { get; set; } = 10;

    // Smartbar
    public Color SmartbarBackground { get; set; } = Hex("#E0E0E0");
    public Color SmartbarForeground { get; set; } = Hex("#121212");
    public Color SuggestionHighlightForeground { get; set; } = Hex("#388E3C");
    public Color DividerColor { get; set; } = Hex("#40000000");
    public Color ActionButtonBackground { get; set; } = Hex("#00000000");
    public Color ActionButtonPressedBackground { get; set; } = Hex("#1F000000");

    // Panels
    public Color PanelBackground { get; set; } = Hex("#E0E0E0");
    public Color PanelForeground { get; set; } = Hex("#121212");
    public Color PanelItemBackground { get; set; } = Hex("#FFFFFF");
    public Color PanelSelectedForeground { get; set; } = Hex("#4CAF50");
    public Color OneHandedBackground { get; set; } = Hex("#E8F5E9");
    public Color OneHandedForeground { get; set; } = Hex("#424242");

    // Glide trail
    public Color GlideTrailColor { get; set; } = Hex("#4CAF50");
    public double GlideTrailThickness { get; set; } = 7;

    // Typography
    public string? FontFamily { get; set; }
    public double KeyFontSize { get; set; } = 22;
    public double FunctionKeyFontSize { get; set; } = 16;
    public double HintFontSize { get; set; } = 11;
    public double SpaceFontSize { get; set; } = 13;
    public double IconSize { get; set; } = 22;
    public double SmartbarFontSize { get; set; } = 16;
    public double EmojiFontSize { get; set; } = 28;
    public bool UppercaseLabelsWhenShifted { get; set; } = true;

    /// <summary>Per-key overrides.</summary>
    public List<KeyStyleRule> KeyRules { get; set; } = new();

    /// <summary>Creates a copy of the theme.</summary>
    public KeyboardTheme Clone()
    {
        var clone = FromJson(ToJson());
        return clone;
    }

    // ------------------------------------------------------------------ JSON

    /// <summary>Serializes the theme to JSON.</summary>
    public string ToJson()
    {
        using var stream = new MemoryStream();
        using (var w = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = true }))
        {
            w.WriteStartObject();
            w.WriteString("id", Id);
            w.WriteString("name", Name);
            w.WriteBoolean("isDark", IsDark);
            foreach (var (name, getter, _) in ColorProperties)
            {
                w.WriteString(name, ToHex(getter(this)));
            }
            foreach (var (name, getter, _) in NumberProperties)
            {
                w.WriteNumber(name, getter(this));
            }
            if (FontFamily is not null)
            {
                w.WriteString("fontFamily", FontFamily);
            }
            w.WriteBoolean("uppercaseLabelsWhenShifted", UppercaseLabelsWhenShifted);
            w.WriteStartArray("keyRules");
            foreach (var rule in KeyRules)
            {
                w.WriteStartObject();
                if (rule.Codes.Count > 0)
                {
                    w.WriteStartArray("codes");
                    foreach (var c in rule.Codes) w.WriteNumberValue(c);
                    w.WriteEndArray();
                }
                if (rule.Type is { } t) w.WriteString("type", t.ToJsonString());
                if (rule.Mode is { } m) w.WriteString("mode", m.ToString());
                if (rule.ShiftState is { } s) w.WriteString("shiftState", s.ToString());
                if (rule.Background is { } bg) w.WriteString("background", ToHex(bg));
                if (rule.PressedBackground is { } pbg) w.WriteString("pressedBackground", ToHex(pbg));
                if (rule.Foreground is { } fg) w.WriteString("foreground", ToHex(fg));
                if (rule.FontSize is { } fs) w.WriteNumber("fontSize", fs);
                if (rule.CornerRadius is { } cr) w.WriteNumber("cornerRadius", cr);
                w.WriteEndObject();
            }
            w.WriteEndArray();
            w.WriteEndObject();
        }
        return Encoding.UTF8.GetString(stream.ToArray());
    }

    /// <summary>
    /// Loads a theme from JSON. Missing properties keep the values of <paramref name="baseTheme"/> (or the defaults).
    /// </summary>
    public static KeyboardTheme FromJson(string json, KeyboardTheme? baseTheme = null)
    {
        var theme = baseTheme is null ? new KeyboardTheme() : baseTheme.Clone();
        using var doc = JsonDocument.Parse(json, new JsonDocumentOptions { AllowTrailingCommas = true, CommentHandling = JsonCommentHandling.Skip });
        var root = doc.RootElement;
        if (root.TryGetProperty("id", out var id)) theme.Id = id.GetString() ?? theme.Id;
        if (root.TryGetProperty("name", out var name)) theme.Name = name.GetString() ?? theme.Name;
        if (root.TryGetProperty("isDark", out var dark)) theme.IsDark = dark.GetBoolean();
        foreach (var (prop, _, setter) in ColorProperties)
        {
            if (root.TryGetProperty(prop, out var v) && v.GetString() is { } hex)
            {
                setter(theme, Hex(hex));
            }
        }
        foreach (var (prop, _, setter) in NumberProperties)
        {
            if (root.TryGetProperty(prop, out var v) && v.ValueKind == JsonValueKind.Number)
            {
                setter(theme, v.GetDouble());
            }
        }
        if (root.TryGetProperty("fontFamily", out var ff)) theme.FontFamily = ff.GetString();
        if (root.TryGetProperty("uppercaseLabelsWhenShifted", out var up)) theme.UppercaseLabelsWhenShifted = up.GetBoolean();
        if (root.TryGetProperty("keyRules", out var rules))
        {
            theme.KeyRules = new List<KeyStyleRule>();
            foreach (var r in rules.EnumerateArray())
            {
                var rule = new KeyStyleRule();
                if (r.TryGetProperty("codes", out var codes)) rule.Codes = codes.EnumerateArray().Select(c => c.GetInt32()).ToList();
                if (r.TryGetProperty("type", out var type)) rule.Type = KeyEnumExtensions.ParseKeyType(type.GetString());
                if (r.TryGetProperty("mode", out var mode) && Enum.TryParse<KeyboardMode>(mode.GetString(), true, out var km)) rule.Mode = km;
                if (r.TryGetProperty("shiftState", out var ss) && Enum.TryParse<ShiftState>(ss.GetString(), true, out var st)) rule.ShiftState = st;
                if (r.TryGetProperty("background", out var bg)) rule.Background = Hex(bg.GetString()!);
                if (r.TryGetProperty("pressedBackground", out var pbg)) rule.PressedBackground = Hex(pbg.GetString()!);
                if (r.TryGetProperty("foreground", out var fg)) rule.Foreground = Hex(fg.GetString()!);
                if (r.TryGetProperty("fontSize", out var fs)) rule.FontSize = fs.GetDouble();
                if (r.TryGetProperty("cornerRadius", out var cr)) rule.CornerRadius = cr.GetDouble();
                theme.KeyRules.Add(rule);
            }
        }
        return theme;
    }

    private static readonly (string Name, Func<KeyboardTheme, Color> Get, Action<KeyboardTheme, Color> Set)[] ColorProperties =
    [
        ("background", t => t.Background, (t, v) => t.Background = v),
        ("keyBackground", t => t.KeyBackground, (t, v) => t.KeyBackground = v),
        ("keyPressedBackground", t => t.KeyPressedBackground, (t, v) => t.KeyPressedBackground = v),
        ("keyForeground", t => t.KeyForeground, (t, v) => t.KeyForeground = v),
        ("keyHintForeground", t => t.KeyHintForeground, (t, v) => t.KeyHintForeground = v),
        ("functionKeyBackground", t => t.FunctionKeyBackground, (t, v) => t.FunctionKeyBackground = v),
        ("functionKeyPressedBackground", t => t.FunctionKeyPressedBackground, (t, v) => t.FunctionKeyPressedBackground = v),
        ("functionKeyForeground", t => t.FunctionKeyForeground, (t, v) => t.FunctionKeyForeground = v),
        ("accentKeyBackground", t => t.AccentKeyBackground, (t, v) => t.AccentKeyBackground = v),
        ("accentKeyPressedBackground", t => t.AccentKeyPressedBackground, (t, v) => t.AccentKeyPressedBackground = v),
        ("accentKeyForeground", t => t.AccentKeyForeground, (t, v) => t.AccentKeyForeground = v),
        ("spaceKeyForeground", t => t.SpaceKeyForeground, (t, v) => t.SpaceKeyForeground = v),
        ("capsLockForeground", t => t.CapsLockForeground, (t, v) => t.CapsLockForeground = v),
        ("keyBorderColor", t => t.KeyBorderColor, (t, v) => t.KeyBorderColor = v),
        ("keyShadowColor", t => t.KeyShadowColor, (t, v) => t.KeyShadowColor = v),
        ("popupBackground", t => t.PopupBackground, (t, v) => t.PopupBackground = v),
        ("popupForeground", t => t.PopupForeground, (t, v) => t.PopupForeground = v),
        ("popupSelectedBackground", t => t.PopupSelectedBackground, (t, v) => t.PopupSelectedBackground = v),
        ("popupSelectedForeground", t => t.PopupSelectedForeground, (t, v) => t.PopupSelectedForeground = v),
        ("popupBorderColor", t => t.PopupBorderColor, (t, v) => t.PopupBorderColor = v),
        ("smartbarBackground", t => t.SmartbarBackground, (t, v) => t.SmartbarBackground = v),
        ("smartbarForeground", t => t.SmartbarForeground, (t, v) => t.SmartbarForeground = v),
        ("suggestionHighlightForeground", t => t.SuggestionHighlightForeground, (t, v) => t.SuggestionHighlightForeground = v),
        ("dividerColor", t => t.DividerColor, (t, v) => t.DividerColor = v),
        ("actionButtonBackground", t => t.ActionButtonBackground, (t, v) => t.ActionButtonBackground = v),
        ("actionButtonPressedBackground", t => t.ActionButtonPressedBackground, (t, v) => t.ActionButtonPressedBackground = v),
        ("panelBackground", t => t.PanelBackground, (t, v) => t.PanelBackground = v),
        ("panelForeground", t => t.PanelForeground, (t, v) => t.PanelForeground = v),
        ("panelItemBackground", t => t.PanelItemBackground, (t, v) => t.PanelItemBackground = v),
        ("panelSelectedForeground", t => t.PanelSelectedForeground, (t, v) => t.PanelSelectedForeground = v),
        ("oneHandedBackground", t => t.OneHandedBackground, (t, v) => t.OneHandedBackground = v),
        ("oneHandedForeground", t => t.OneHandedForeground, (t, v) => t.OneHandedForeground = v),
        ("glideTrailColor", t => t.GlideTrailColor, (t, v) => t.GlideTrailColor = v),
    ];

    private static readonly (string Name, Func<KeyboardTheme, double> Get, Action<KeyboardTheme, double> Set)[] NumberProperties =
    [
        ("keyBorderThickness", t => t.KeyBorderThickness, (t, v) => t.KeyBorderThickness = v),
        ("keyShadowDepth", t => t.KeyShadowDepth, (t, v) => t.KeyShadowDepth = v),
        ("keyCornerRadius", t => t.KeyCornerRadius, (t, v) => t.KeyCornerRadius = v),
        ("popupCornerRadius", t => t.PopupCornerRadius, (t, v) => t.PopupCornerRadius = v),
        ("glideTrailThickness", t => t.GlideTrailThickness, (t, v) => t.GlideTrailThickness = v),
        ("keyFontSize", t => t.KeyFontSize, (t, v) => t.KeyFontSize = v),
        ("functionKeyFontSize", t => t.FunctionKeyFontSize, (t, v) => t.FunctionKeyFontSize = v),
        ("hintFontSize", t => t.HintFontSize, (t, v) => t.HintFontSize = v),
        ("spaceFontSize", t => t.SpaceFontSize, (t, v) => t.SpaceFontSize = v),
        ("iconSize", t => t.IconSize, (t, v) => t.IconSize = v),
        ("smartbarFontSize", t => t.SmartbarFontSize, (t, v) => t.SmartbarFontSize = v),
        ("emojiFontSize", t => t.EmojiFontSize, (t, v) => t.EmojiFontSize = v),
    ];

    // ------------------------------------------------------------------ colors

    /// <summary>Parses <c>#RGB</c>, <c>#RRGGBB</c> or <c>#AARRGGBB</c>.</summary>
    public static Color Hex(string hex)
    {
        var s = hex.Trim().TrimStart('#');
        if (s.Length == 3)
        {
            s = string.Concat(s.Select(c => new string(c, 2)));
        }
        if (s.Length == 6)
        {
            s = "FF" + s;
        }
        if (s.Length != 8 || !uint.TryParse(s, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out var v))
        {
            throw new FormatException($"Invalid color '{hex}'.");
        }
        return Color.FromArgb((byte)(v >> 24), (byte)(v >> 16), (byte)(v >> 8), (byte)v);
    }

    /// <summary>Formats a color as <c>#AARRGGBB</c>.</summary>
    public static string ToHex(Color c) => $"#{c.A:X2}{c.R:X2}{c.G:X2}{c.B:X2}";
}
