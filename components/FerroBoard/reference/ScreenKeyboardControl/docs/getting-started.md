# Getting started

## Requirements

- .NET 10 SDK
- An Uno Platform 6.x application (`Uno.Sdk` 6.7 or later). The library supports the `net10.0-desktop`,
  `net10.0-browserwasm`, `net10.0-android`, `net10.0-ios` and `net10.0-windows10.0.26100` target frameworks.

## Install

```bash
dotnet add package ScreenKeyboard.Uno
```

`ScreenKeyboard.Uno` references `ScreenKeyboard.Core`, which contains the engine and all bundled data (layouts,
localization, English dictionary, emoji data).

## Option 1 — `ScreenKeyboardHost` (recommended)

`ScreenKeyboardHost` wraps your content and docks an `OnScreenKeyboard` at the bottom:

```xml
<Page xmlns:sk="using:ScreenKeyboard.Controls">
  <sk:ScreenKeyboardHost x:Name="Host" VisibilityMode="Auto">
    <ScrollViewer>
      <StackPanel Spacing="12" Padding="24">
        <TextBox Header="Message" AcceptsReturn="True" TextWrapping="Wrap" />
        <TextBox Header="Search" InputScope="Search" />
      </StackPanel>
    </ScrollViewer>
  </sk:ScreenKeyboardHost>
</Page>
```

| Property | Description |
|---|---|
| `Content` | Your UI. |
| `Keyboard` | The hosted `OnScreenKeyboard` (configure settings, themes and events through it). |
| `VisibilityMode` | `Auto` (show on text focus, default), `AlwaysVisible` or `Manual`. |
| `IsKeyboardOpen` | Shows/hides the keyboard (two-way). |
| `IsAnimated` | Slide animation (default `true`). |

In `Auto` mode the keyboard opens when a `TextBox`/`PasswordBox` inside the host receives focus, hides when focus
leaves text inputs, re-opens when the focused field is tapped again and brings the focused field into view.
Swiping down on the keyboard (or pressing the hide button in the smartbar) closes it.

## Option 2 — standalone `OnScreenKeyboard`

Put the control wherever you want. With `AutoAttach="True"` (default) it types into the focused text input of its
window:

```xml
<Grid RowDefinitions="*,Auto">
  <TextBox AcceptsReturn="True" />
  <sk:OnScreenKeyboard Grid.Row="1" />
</Grid>
```

Attach manually when needed:

```csharp
keyboard.AutoAttach = false;
keyboard.AttachTo(myTextBox);               // TextBox or PasswordBox
keyboard.Attach(new MyCustomEditorTarget()); // any ITextInputTarget
keyboard.Detach();
```

## Configure

All options live in `KeyboardSettings` (observable, JSON serializable):

```csharp
using ScreenKeyboard.Settings;

var settings = Host.Keyboard.Settings;
settings.Subtypes = ["en-US/qwerty", "de-DE/qwertz"]; // enabled languages
settings.ThemeMode = ThemeMode.FollowSystem;
settings.LightTheme = "material_light";
settings.DarkTheme = "material_dark";
settings.KeyHeight = 58;
settings.GlideTyping = true;
settings.AutoCorrect = true;
```

See [Configuration](configuration.md) for the full list.

## Keyboard modes

| Mode | How to enable |
|---|---|
| One-handed | `settings.OneHandedMode = OneHandedMode.Left/Right` (quick action; side panel to switch sides or exit) |
| Split | `settings.SplitKeyboard = true` (letters and symbols are split in two halves; `SplitGapRatio`) |
| Floating | `settings.Floating = true` with `ScreenKeyboardHost` — a draggable window above your content (`FloatingWidthRatio`) |
| Resize | Quick action or `KeyCode.ToggleResizeMode`: drag up/down to change `HeightScale`, or set `KeyHeight`/`HeightScale` in code |
| Number row | `settings.NumberRow = true` |

## Events

| Event | Raised when |
|---|---|
| `EnterActionRequested` | A single-line field's enter key is pressed with an action (Go, Search, Send, Next, Done). Set `Handled = true` if you handled it. `Next`/`Previous` move focus automatically, `Done` hides the keyboard by default. |
| `SettingsRequested` | The settings key/quick action or "Manage languages" is used. Show your settings UI. |
| `HideRequested` | The user wants to hide the keyboard. |
| `VoiceInputRequested` | A voice input key (if present in a custom layout) is pressed. |
| `FeedbackRequested` | Every key press, long press and gesture step (`FeedbackKind`) — play a click sound here. Haptics are performed automatically on Android/iOS when enabled. |
| `TargetChanged` | The keyboard attached to another input. |

## Emoji on WebAssembly

Browsers do not expose a color emoji font to Uno's Skia renderer. Enable the emoji fallback at startup:

```csharp
public App()
{
    InitializeComponent();
    if (OperatingSystem.IsBrowser())
    {
        ScreenKeyboard.Controls.EmojiFontFallback.UseNotoColorEmoji(); // or .Use(() => OpenBundledFontAsync())
    }
}
```

## Mobile platforms

On Android and iOS the operating system shows its own soft keyboard for focused text boxes. ScreenKeyboard is
primarily intended for platforms without an OS keyboard (desktop, kiosks, embedded Linux, the web on non-touch
devices), but it works on mobile too — for example in apps with custom input fields implementing
`ITextInputTarget`, which never trigger the system keyboard.

## Next steps

- [Input integration](input-integration.md)
- [Layouts & languages](layouts.md)
- [Theming](theming.md)

## Localizing the keyboard UI

Key labels come from the layouts; texts of panels, tooltips and automation names come from `KeyboardStrings` and can
be localized at startup:

```csharp
KeyboardStrings.Clipboard = "Zwischenablage";
KeyboardStrings.SearchEmoji = "Emoji suchen";
KeyboardStrings.PasteFormat = "{0} einfügen";
```
