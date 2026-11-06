# Configuration

Every option lives in `ScreenKeyboard.Settings.KeyboardSettings`. The class implements `INotifyPropertyChanged`
(bind it to a settings UI), validates ranges, and every change is applied immediately to a running keyboard.

```csharp
var settings = keyboard.Settings;           // OnScreenKeyboard.Settings or KeyboardEngine.Settings
settings.KeyHeight = 60;
settings.SwipeUp = SwipeAction.Undo;

// Persist
string json = settings.ToJson();
settings.CopyFrom(KeyboardSettings.FromJson(json));
```

Assigning a different instance to `OnScreenKeyboard.Settings` copies its values into the keyboard's settings.
JSON uses camelCase property names and enum names as strings; unknown properties are ignored, so stored settings
stay compatible across versions.

## Persisting user data

Besides settings, the engine exposes serializable user data:

| Data | Export | Import |
|---|---|---|
| Learned words and word pairs | `engine.LanguageModels.GetUserDictionary("en").Export()` | `.Import(json)` |
| Emoji recents, pins and skin tones | `engine.EmojiHistory.Export()` | `.Import(json)` |
| Clipboard history | `engine.ClipboardHistory.Export(pinnedOnly: true)` | `.Import(json)` |

The sample app's [`SampleStorage`](../samples/ScreenKeyboard.Sample/SampleStorage.cs) shows how to store them with
`ApplicationData`.

## Reference

### Languages

| Setting | Type | Default | Description |
|---|---|---|---|
| `Subtypes` | `List<string>` | `["en-US/qwerty"]` | Enabled subtypes as ids (`languageTag/charactersLayoutId`), e.g. `"de-DE/qwertz"`. |

### Layout

| Setting | Type | Default | Description |
|---|---|---|---|
| `NumberRow` | `bool` | `false` | Shows a dedicated number row above the letters. |
| `HintedNumberRow` | `bool` | `true` | Shows number hints on the top letter row (long press to enter). |
| `HintedSymbols` | `bool` | `true` | Shows symbol hints on letter keys. |
| `HintMode` | `KeyHintMode` | `AccentPriority` | Which popup key is selected by default on long press. |
| `UtilityKeyAction` | `UtilityKeyAction` | `Dynamic` | Action of the key next to the space bar. |
| `SpaceBarMode` | `SpaceBarMode` | `CurrentLanguage` | Space bar label. |
| `KeyHeight` | `double` | `54` | Height of a key row in device independent pixels (before `HeightScale`). |
| `HeightScale` | `double` | `1.0` | Keyboard height multiplier (0.5 - 2.0). |
| `KeySpacingHorizontal` | `double` | `5` | Horizontal gap between keys in DIPs. |
| `KeySpacingVertical` | `double` | `9` | Vertical gap between rows in DIPs. |
| `FontScale` | `double` | `1.0` | Key label font scale. |
| `MaxKeyboardWidth` | `double` | `1100` | Maximum width of the keys area on large screens (0 = unlimited). |
| `OneHandedMode` | `OneHandedMode` | `Off` | One-handed (compact) mode. |
| `OneHandedWidth` | `double` | `0.82` | Relative width of the keyboard in one-handed mode. |
| `SplitKeyboard` | `bool` | `false` | Splits the keys into two halves (tablets / landscape). |
| `SplitGapRatio` | `double` | `0.22` | Width of the split gap relative to the keyboard width. |
| `Floating` | `bool` | `false` | Shows the keyboard as a floating, draggable window (supported by `ScreenKeyboardHost`). |
| `FloatingWidthRatio` | `double` | `0.6` | Width of the floating keyboard relative to the host width. |

### Keys & popups

| Setting | Type | Default | Description |
|---|---|---|---|
| `ShowKeyPreview` | `bool` | `true` | Shows an enlarged preview bubble above pressed character keys. |
| `LongPressDelay` | `int` | `300` | Long press delay in milliseconds. |
| `KeyRepeatDelay` | `int` | `400` | Delay before key repeat starts (delete, arrows) in milliseconds. |
| `KeyRepeatInterval` | `int` | `50` | Key repeat interval in milliseconds. |
| `DoubleTapShiftForCapsLock` | `bool` | `true` | Double tap on shift enables caps lock. |
| `DoubleTapDelay` | `int` | `350` | Maximum delay for a double tap in milliseconds. |

### Typing

| Setting | Type | Default | Description |
|---|---|---|---|
| `AutoCapitalization` | `bool` | `true` | Capitalizes the first letter of sentences. |
| `DoubleSpacePeriod` | `bool` | `true` | Double space inserts a period. |
| `AutoSpacePunctuation` | `bool` | `true` | Removes the space before punctuation and inserts one after it where appropriate. |
| `ShowSuggestions` | `bool` | `true` | Shows word suggestions in the smartbar. |
| `AutoCorrect` | `bool` | `true` | Automatically replaces misspelled words when a separator is typed. |
| `NextWordPrediction` | `bool` | `true` | Predicts the next word after a completed word. |
| `LearnWords` | `bool` | `true` | Learns new words and word pairs from typing (disabled in incognito mode). |
| `EmojiSuggestions` | `bool` | `true` | Suggests emojis matching the typed word or a `:shortcode`. |
| `SuggestionCount` | `int` | `3` | Number of suggestions shown. |
| `UndoAutoCorrectOnBackspace` | `bool` | `true` | Backspace right after an auto-correction reverts it. |
| `IncognitoMode` | `bool` | `false` | Incognito mode: nothing is learned or stored (history, clipboard, emoji recents). |

### Gestures

| Setting | Type | Default | Description |
|---|---|---|---|
| `GlideTyping` | `bool` | `true` | Enables glide (swipe) typing over letter keys. |
| `GlideShowTrail` | `bool` | `true` | Draws the glide trail. |
| `GlideTrailDuration` | `int` | `200` | How long the trail stays visible after lifting the finger (ms). |
| `GlidePreview` | `bool` | `true` | Shows live glide suggestions while swiping. |
| `SwipeDistanceThreshold` | `double` | `32` | Minimum finger travel (DIPs) for a swipe gesture. |
| `SwipeUp` | `SwipeAction` | `Shift` | Swipe up on the keyboard. |
| `SwipeDown` | `SwipeAction` | `HideKeyboard` | Swipe down on the keyboard. |
| `SwipeLeft` | `SwipeAction` | `SwitchToNextSubtype` | Swipe left on the keyboard (when glide typing is off). |
| `SwipeRight` | `SwipeAction` | `SwitchToPrevSubtype` | Swipe right on the keyboard (when glide typing is off). |
| `SpaceBarSwipeUp` | `SwipeAction` | `SwitchToClipboardContext` | Swipe up on the space bar. |
| `SpaceBarSwipeLeft` | `SwipeAction` | `MoveCursorLeft` | Swipe left on the space bar (moves continuously while dragging). |
| `SpaceBarSwipeRight` | `SwipeAction` | `MoveCursorRight` | Swipe right on the space bar (moves continuously while dragging). |
| `SpaceBarLongPress` | `SwipeAction` | `ShowSubtypePicker` | Long press on the space bar. |
| `DeleteKeySwipeLeft` | `SwipeAction` | `DeleteWordsPrecisely` | Swipe left from the delete key. |
| `DeleteKeyLongPress` | `SwipeAction` | `DeleteCharacter` | Long press on the delete key (repeats). |

### Feedback

| Setting | Type | Default | Description |
|---|---|---|---|
| `HapticFeedback` | `bool` | `true` | Vibrates on key press (where supported). |
| `HapticDuration` | `int` | `12` | Vibration duration in milliseconds. |
| `SoundFeedback` | `bool` | `false` | Plays a click sound on key press (the app provides the sound via the feedback event). |
| `SoundVolume` | `double` | `0.5` | Sound volume (0-1). |

### Smartbar

| Setting | Type | Default | Description |
|---|---|---|---|
| `SmartbarEnabled` | `bool` | `true` | Shows the smartbar (suggestions and quick actions) above the keys. |
| `SmartbarLayout` | `SmartbarLayout` | `SuggestionsWithActions` | Smartbar layout. |

### Theme

| Setting | Type | Default | Description |
|---|---|---|---|
| `ThemeMode` | `ThemeMode` | `FollowSystem` | Light/dark theme selection. |
| `LightTheme` | `string` | `"floris_day"` | Id of the theme used in light mode. |
| `DarkTheme` | `string` | `"floris_night"` | Id of the theme used in dark mode. |

### Emoji & clipboard

| Setting | Type | Default | Description |
|---|---|---|---|
| `EmojiSkinTone` | `EmojiSkinTone` | `Default` | Default emoji skin tone. |
| `EmojiHistoryEnabled` | `bool` | `true` | Remembers recently used emojis. |
| `ClipboardHistoryEnabled` | `bool` | `true` | Keeps a clipboard history. |
| `ClipboardSuggestion` | `bool` | `true` | Suggests the most recently copied text in the smartbar. |

## Enumerations

| Enum | Values |
|---|---|
| `SwipeAction` | `NoAction`, `CycleToPreviousKeyboardMode`, `CycleToNextKeyboardMode`, `DeleteCharacter`, `DeleteCharactersPrecisely`, `DeleteWord`, `DeleteWordsPrecisely`, `HideKeyboard`, `InsertSpace`, `MoveCursorUp/Down/Left/Right`, `MoveCursorStartOfLine/EndOfLine/StartOfPage/EndOfPage`, `Redo`, `Undo`, `SelectCharactersPrecisely`, `SelectWordsPrecisely`, `Shift`, `ShowSubtypePicker`, `SwitchToPrevSubtype`, `SwitchToNextSubtype`, `SwitchToClipboardContext`, `SwitchToMediaContext`, `SwitchToEditingContext`, `ToggleOneHandedMode`, `ToggleSmartbarVisibility` |
| `UtilityKeyAction` | `Emoji`, `LanguageSwitch`, `Dynamic` (globe with several languages, emoji otherwise), `Hidden` |
| `SpaceBarMode` | `Nothing`, `CurrentLanguage`, `SpaceBarKey` |
| `KeyHintMode` | `Disabled`, `AccentPriority`, `HintPriority` |
| `OneHandedMode` | `Off`, `Left`, `Right` |
| `ThemeMode` | `FollowSystem`, `Light`, `Dark` |
| `SmartbarLayout` | `SuggestionsWithActions`, `SuggestionsOnly`, `ActionsOnly` |
| `EmojiSkinTone` | `Default`, `Light`, `MediumLight`, `Medium`, `MediumDark`, `Dark` |
