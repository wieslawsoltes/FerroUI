# Theming

Every color, radius and font of the keyboard comes from a `ScreenKeyboard.Controls.Themes.KeyboardTheme`.

## Choosing a theme

By default the keyboard follows the application's light/dark theme:

```csharp
settings.ThemeMode = ThemeMode.FollowSystem;  // or Light / Dark
settings.LightTheme = "floris_day";
settings.DarkTheme = "floris_night";
```

Or force a specific theme instance (overrides the settings):

```csharp
keyboard.Theme = KeyboardThemes.Get("nord");
```

### Built-in themes

| Id | Name | Dark |
|---|---|---|
| `floris_day` | Floris Day | |
| `floris_day_borderless` | Floris Day (borderless) | |
| `floris_night` | Floris Night | ✔ |
| `floris_night_borderless` | Floris Night (borderless) | ✔ |
| `floris_pure_night` | Floris Pure Night (AMOLED) | ✔ |
| `material_light` | Material Light | |
| `material_dark` | Material Dark | ✔ |
| `ocean` | Ocean | ✔ |
| `rose` | Rosé | |
| `nord` | Nord | ✔ |
| `high_contrast` | High Contrast (bordered keys) | ✔ |
| `terminal` | Terminal (monospace) | ✔ |

`KeyboardThemes.All` lists every registered theme; `KeyboardThemes.Register(theme)` adds your own (they then appear
in `settings.LightTheme`/`DarkTheme`).

## Creating a theme in C#

```csharp
var theme = KeyboardThemes.FlorisNight.Clone();
theme.Id = "brand_dark";
theme.Name = "Brand (dark)";
theme.AccentKeyBackground = KeyboardTheme.Hex("#FF6200EE");
theme.GlideTrailColor = KeyboardTheme.Hex("#FFBB86FC");
theme.KeyCornerRadius = 12;
theme.FontFamily = "Segoe UI Variable";
KeyboardThemes.Register(theme);
settings.DarkTheme = "brand_dark";
```

## Creating a theme in XAML

```xml
<Application.Resources>
  <skt:KeyboardTheme x:Key="SunsetTheme" xmlns:skt="using:ScreenKeyboard.Controls.Themes"
                     Id="sunset" Name="Sunset" IsDark="True"
                     Background="#FF2B1B2E" KeyBackground="#FF46304A" KeyForeground="#FFFFE9D6"
                     AccentKeyBackground="#FFFF7B54" AccentKeyForeground="#FF2B1B2E"
                     KeyCornerRadius="14" />
</Application.Resources>
```

```csharp
KeyboardThemes.Register((KeyboardTheme)Application.Current.Resources["SunsetTheme"]);
```

(See the sample's `App.xaml` for a complete XAML theme.)

## JSON themes

Themes serialize to JSON — store them in files, download them, or let users share them:

```csharp
string json = theme.ToJson();
var loaded = KeyboardTheme.FromJson(json);                          // missing values use defaults
var derived = KeyboardTheme.FromJson(overridesJson, KeyboardThemes.FlorisDay); // or a base theme
```

```json
{
  "id": "mint",
  "name": "Mint",
  "isDark": false,
  "background": "#FFE8F5EE",
  "keyBackground": "#FFFFFFFF",
  "accentKeyBackground": "#FF2BB673",
  "keyCornerRadius": 10,
  "keyRules": [
    { "codes": [32], "foreground": "#FF2BB673" },
    { "type": "system_gui", "background": "#FFD7EFE2" }
  ]
}
```

## Properties

| Group | Properties |
|---|---|
| Surfaces | `Background`, `KeyBackground`, `KeyPressedBackground`, `KeyForeground`, `KeyHintForeground`, `FunctionKeyBackground`, `FunctionKeyPressedBackground`, `FunctionKeyForeground`, `AccentKeyBackground`, `AccentKeyPressedBackground`, `AccentKeyForeground` (enter key), `SpaceKeyForeground`, `CapsLockForeground` |
| Shape | `KeyCornerRadius`, `KeyBorderColor`, `KeyBorderThickness`, `KeyShadowColor`, `KeyShadowDepth` |
| Popups & preview | `PopupBackground`, `PopupForeground`, `PopupSelectedBackground`, `PopupSelectedForeground`, `PopupBorderColor`, `PopupCornerRadius` |
| Smartbar | `SmartbarBackground`, `SmartbarForeground`, `SuggestionHighlightForeground` (auto-correct candidate, active quick actions), `DividerColor`, `ActionButtonBackground`, `ActionButtonPressedBackground` |
| Panels | `PanelBackground`, `PanelForeground`, `PanelItemBackground`, `PanelSelectedForeground`, `OneHandedBackground`, `OneHandedForeground` |
| Glide | `GlideTrailColor`, `GlideTrailThickness` |
| Typography | `FontFamily`, `KeyFontSize`, `FunctionKeyFontSize`, `HintFontSize`, `SpaceFontSize`, `IconSize`, `SmartbarFontSize`, `EmojiFontSize` |

Font sizes are multiplied by `KeyboardSettings.FontScale` and clamped to fit the key.

## Key style rules

`KeyRules` override the style of matching keys (evaluated in order):

```csharp
theme.KeyRules.Add(new KeyStyleRule
{
    Codes = [KeyCode.Space],
    Background = KeyboardTheme.Hex("#FF3A3A3A"),
    Foreground = KeyboardTheme.Hex("#FFFFFFFF"),
});
theme.KeyRules.Add(new KeyStyleRule { Type = KeyType.SystemGui, FontSize = 14 });
theme.KeyRules.Add(new KeyStyleRule { Codes = [KeyCode.Shift], ShiftState = ShiftState.CapsLock, Background = KeyboardTheme.Hex("#FFFFA000") });
```

A rule matches when all of its non-null conditions match: `Codes`, `Type`, `Mode` (keyboard mode) and `ShiftState`.

## Icons

Key icons are vector paths (Material Symbols) rendered with the theme's foreground colors, so they scale with the
key size and never depend on platform icon fonts. Replace any icon with `IconFactory.Register(KeyIcon, pathData)`
(absolute `M`/`L`/`C`/`Z` commands in a 0..1 box). `build/scripts/generate_icons.py` converts SVG files.
