using ScreenKeyboard.Keys;
using static ScreenKeyboard.Controls.Themes.KeyboardTheme;

namespace ScreenKeyboard.Controls.Themes;

/// <summary>Registry of available keyboard themes (built-in themes are registered automatically).</summary>
public static class KeyboardThemes
{
    private static readonly Dictionary<string, KeyboardTheme> s_themes = new(StringComparer.OrdinalIgnoreCase);

    static KeyboardThemes()
    {
        foreach (var theme in CreateBuiltIn())
        {
            if (theme.Id == "high_contrast")
            {
                ApplyHighContrastBorders(theme);
            }
            s_themes[theme.Id] = theme;
        }
    }

    /// <summary>Raised when a theme was registered.</summary>
    public static event EventHandler? Changed;

    /// <summary>All registered themes.</summary>
    public static IReadOnlyList<KeyboardTheme> All
    {
        get { lock (s_themes) return s_themes.Values.ToList(); }
    }

    /// <summary>Registers (or replaces) a theme.</summary>
    public static void Register(KeyboardTheme theme)
    {
        lock (s_themes) s_themes[theme.Id] = theme;
        Changed?.Invoke(null, EventArgs.Empty);
    }

    /// <summary>Gets a theme by id, or <c>null</c>.</summary>
    public static KeyboardTheme? Get(string id)
    {
        lock (s_themes) return s_themes.TryGetValue(id, out var theme) ? theme : null;
    }

    /// <summary>Default light theme (FlorisBoard Day).</summary>
    public static KeyboardTheme FlorisDay => Get("floris_day")!;

    /// <summary>Default dark theme (FlorisBoard Night).</summary>
    public static KeyboardTheme FlorisNight => Get("floris_night")!;

    private static KeyboardTheme Floris(string id, string name, bool dark, bool borderless,
        string primary, string primaryVariant, string secondary, string background, string surface, string surfaceVariant,
        string popupSurface, string focusedPopup, string onBackground, string onSurface, string onSurfaceVariant,
        string oneHandBg, string oneHandFg)
    {
        var t = new KeyboardTheme
        {
            Id = id,
            Name = name,
            IsDark = dark,
            Background = Hex(background),
            KeyBackground = borderless ? Hex("#00000000") : Hex(surface),
            KeyPressedBackground = Hex(surfaceVariant),
            KeyForeground = Hex(onSurface),
            KeyHintForeground = Hex(onSurfaceVariant),
            FunctionKeyBackground = borderless ? Hex("#00000000") : Hex(surface),
            FunctionKeyPressedBackground = Hex(surfaceVariant),
            FunctionKeyForeground = Hex(onSurface),
            AccentKeyBackground = Hex(primary),
            AccentKeyPressedBackground = Hex(primaryVariant),
            AccentKeyForeground = Hex("#F0F0F0"),
            SpaceKeyForeground = Hex(onSurfaceVariant),
            CapsLockForeground = Hex(secondary),
            KeyShadowColor = borderless ? Hex("#00000000") : dark ? Hex("#66000000") : Hex("#33000000"),
            KeyShadowDepth = borderless ? 0 : 1.5,
            PopupBackground = Hex(popupSurface),
            PopupForeground = Hex(onSurface),
            PopupSelectedBackground = Hex(focusedPopup),
            PopupSelectedForeground = Hex(onSurface),
            PopupBorderColor = dark ? Hex("#33FFFFFF") : Hex("#1F000000"),
            SmartbarBackground = Hex(background),
            SmartbarForeground = Hex(onBackground),
            SuggestionHighlightForeground = Hex(primary),
            DividerColor = dark ? Hex("#40FFFFFF") : Hex("#40000000"),
            ActionButtonPressedBackground = dark ? Hex("#26FFFFFF") : Hex("#1F000000"),
            PanelBackground = Hex(background),
            PanelForeground = Hex(onBackground),
            PanelItemBackground = Hex(surface),
            PanelSelectedForeground = Hex(primary),
            OneHandedBackground = Hex(oneHandBg),
            OneHandedForeground = Hex(oneHandFg),
            GlideTrailColor = Hex(primary),
        };
        return t;
    }

    private static KeyboardTheme Custom(string id, string name, bool dark, string background, string key, string keyPressed, string keyFg,
        string function, string functionPressed, string accent, string accentPressed, string accentFg, string hint, string popup,
        string popupSelected, string trail, string corner = "8", string? font = null, string? secondary = null)
    {
        var t = new KeyboardTheme
        {
            Id = id,
            Name = name,
            IsDark = dark,
            Background = Hex(background),
            KeyBackground = Hex(key),
            KeyPressedBackground = Hex(keyPressed),
            KeyForeground = Hex(keyFg),
            KeyHintForeground = Hex(hint),
            FunctionKeyBackground = Hex(function),
            FunctionKeyPressedBackground = Hex(functionPressed),
            FunctionKeyForeground = Hex(keyFg),
            AccentKeyBackground = Hex(accent),
            AccentKeyPressedBackground = Hex(accentPressed),
            AccentKeyForeground = Hex(accentFg),
            SpaceKeyForeground = Hex(hint),
            CapsLockForeground = Hex(secondary ?? accent),
            KeyShadowColor = dark ? Hex("#55000000") : Hex("#2A000000"),
            KeyShadowDepth = 1,
            KeyCornerRadius = double.Parse(corner, System.Globalization.CultureInfo.InvariantCulture),
            PopupBackground = Hex(popup),
            PopupForeground = Hex(keyFg),
            PopupSelectedBackground = Hex(popupSelected),
            PopupSelectedForeground = Hex(accentFg),
            PopupBorderColor = dark ? Hex("#33FFFFFF") : Hex("#1F000000"),
            SmartbarBackground = Hex(background),
            SmartbarForeground = Hex(keyFg),
            SuggestionHighlightForeground = Hex(accent),
            DividerColor = dark ? Hex("#40FFFFFF") : Hex("#33000000"),
            ActionButtonPressedBackground = dark ? Hex("#26FFFFFF") : Hex("#1F000000"),
            PanelBackground = Hex(background),
            PanelForeground = Hex(keyFg),
            PanelItemBackground = Hex(key),
            PanelSelectedForeground = Hex(accent),
            OneHandedBackground = Hex(function),
            OneHandedForeground = Hex(keyFg),
            GlideTrailColor = Hex(trail),
            FontFamily = font,
        };
        return t;
    }

    private static IEnumerable<KeyboardTheme> CreateBuiltIn()
    {
        // FlorisBoard themes (palettes from FlorisBoard's Snygg stylesheets, Apache-2.0).
        yield return Floris("floris_day", "Floris Day", false, false,
            "#4CAF50", "#388E3C", "#FF9800", "#E0E0E0", "#FFFFFF", "#F5F5F5", "#EEEEEE", "#BDBDBD", "#121212", "#000000", "#5F5F5F", "#E8F5E9", "#424242");
        yield return Floris("floris_day_borderless", "Floris Day (borderless)", false, true,
            "#4CAF50", "#388E3C", "#FF9800", "#E0E0E0", "#FFFFFF", "#D0D0D0", "#EEEEEE", "#BDBDBD", "#121212", "#000000", "#5F5F5F", "#E8F5E9", "#424242");
        yield return Floris("floris_night", "Floris Night", true, false,
            "#4CAF50", "#388E3C", "#F57C00", "#212121", "#424242", "#616161", "#757575", "#BDBDBD", "#DCDCDC", "#FFFFFF", "#A0A0A0", "#1B5E20", "#EEEEEE");
        yield return Floris("floris_night_borderless", "Floris Night (borderless)", true, true,
            "#4CAF50", "#388E3C", "#F57C00", "#212121", "#424242", "#3A3A3A", "#757575", "#BDBDBD", "#DCDCDC", "#FFFFFF", "#A0A0A0", "#1B5E20", "#EEEEEE");
        yield return Floris("floris_pure_night", "Floris Pure Night (AMOLED)", true, false,
            "#388E3C", "#306D32", "#FF9800", "#000000", "#212121", "#3D3D3D", "#424242", "#707070", "#EEEEEE", "#EEEEEE", "#B3FFFFFF", "#1B5E20", "#EEEEEE");

        // Additional themes.
        yield return Custom("material_light", "Material Light", false,
            "#E8EAED", "#FFFFFF", "#DADCE0", "#202124", "#D2D5DA", "#BDC1C6", "#1A73E8", "#1557B0", "#FFFFFF", "#5F6368",
            "#FFFFFF", "#1A73E8", "#1A73E8", corner: "6");
        yield return Custom("material_dark", "Material Dark", true,
            "#202124", "#3C4043", "#5F6368", "#E8EAED", "#2D2F31", "#4A4D51", "#8AB4F8", "#AECBFA", "#202124", "#9AA0A6",
            "#3C4043", "#8AB4F8", "#8AB4F8", corner: "6");
        yield return Custom("ocean", "Ocean", true,
            "#0B1D2E", "#16324A", "#21476A", "#E3F2FD", "#10273C", "#1A3A58", "#00B8D4", "#0097A7", "#00202A", "#7FA7C9",
            "#1C3D5C", "#00B8D4", "#18FFFF", corner: "10", secondary: "#FFB74D");
        yield return Custom("rose", "Rosé", false,
            "#FBE9EC", "#FFFFFF", "#F8D7DD", "#3E2327", "#F5D0D7", "#EDB8C2", "#D81B60", "#AD1457", "#FFFFFF", "#9C6B74",
            "#FFFFFF", "#D81B60", "#EC407A", corner: "12");
        yield return Custom("nord", "Nord", true,
            "#2E3440", "#3B4252", "#4C566A", "#ECEFF4", "#343B48", "#434C5E", "#88C0D0", "#81A1C1", "#2E3440", "#A3ABB9",
            "#434C5E", "#88C0D0", "#88C0D0", corner: "8", secondary: "#EBCB8B");
        yield return Custom("high_contrast", "High Contrast", true,
            "#000000", "#000000", "#333333", "#FFFFFF", "#000000", "#333333", "#FFFF00", "#CCCC00", "#000000", "#FFFF00",
            "#000000", "#FFFF00", "#FFFF00", corner: "4", secondary: "#00FFFF");
        yield return Custom("terminal", "Terminal", true,
            "#000000", "#0A0F0A", "#133313", "#33FF66", "#050805", "#0F260F", "#1B5E20", "#2E7D32", "#CCFFCC", "#1FAA4A",
            "#0A160A", "#33FF66", "#33FF66", corner: "2", font: "Courier New");
    }

    internal static void ApplyHighContrastBorders(KeyboardTheme theme)
    {
        theme.KeyBorderColor = theme.KeyForeground;
        theme.KeyBorderThickness = 1.5;
    }
}
