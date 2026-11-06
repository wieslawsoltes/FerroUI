using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Text.Json.Serialization;
using ScreenKeyboard.Emoji;

namespace ScreenKeyboard.Settings;

/// <summary>
/// All user-configurable keyboard preferences. Observable (<see cref="INotifyPropertyChanged"/>) so settings
/// UIs can bind to it, and serializable to JSON via <see cref="ToJson"/>/<see cref="FromJson"/>.
/// Defaults follow FlorisBoard.
/// </summary>
public sealed class KeyboardSettings : INotifyPropertyChanged
{
    /// <inheritdoc />
    public event PropertyChangedEventHandler? PropertyChanged;

    private bool Set<T>(ref T field, T value, [CallerMemberName] string? name = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value))
        {
            return false;
        }
        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
        return true;
    }

    // ------------------------------------------------------------------ languages

    private List<string> _subtypes = ["en-US/qwerty"];
    /// <summary>Enabled subtypes as ids (<c>languageTag/charactersLayoutId</c>), e.g. <c>"de-DE/qwertz"</c>.</summary>
    public List<string> Subtypes { get => _subtypes; set => Set(ref _subtypes, value ?? []); }

    // ------------------------------------------------------------------ layout

    private bool _numberRow;
    /// <summary>Shows a dedicated number row above the letters.</summary>
    public bool NumberRow { get => _numberRow; set => Set(ref _numberRow, value); }

    private bool _hintedNumberRow = true;
    /// <summary>Shows number hints on the top letter row (long press to enter).</summary>
    public bool HintedNumberRow { get => _hintedNumberRow; set => Set(ref _hintedNumberRow, value); }

    private bool _hintedSymbols = true;
    /// <summary>Shows symbol hints on letter keys.</summary>
    public bool HintedSymbols { get => _hintedSymbols; set => Set(ref _hintedSymbols, value); }

    private KeyHintMode _hintMode = KeyHintMode.AccentPriority;
    /// <summary>Which popup key is selected by default on long press.</summary>
    public KeyHintMode HintMode { get => _hintMode; set => Set(ref _hintMode, value); }

    private UtilityKeyAction _utilityKeyAction = UtilityKeyAction.Dynamic;
    /// <summary>Action of the key next to the space bar.</summary>
    public UtilityKeyAction UtilityKeyAction { get => _utilityKeyAction; set => Set(ref _utilityKeyAction, value); }

    private SpaceBarMode _spaceBarMode = SpaceBarMode.CurrentLanguage;
    /// <summary>Space bar label.</summary>
    public SpaceBarMode SpaceBarMode { get => _spaceBarMode; set => Set(ref _spaceBarMode, value); }

    private double _keyHeight = 54;
    /// <summary>Height of a key row in device independent pixels (before <see cref="HeightScale"/>).</summary>
    public double KeyHeight { get => _keyHeight; set => Set(ref _keyHeight, Math.Clamp(value, 24, 160)); }

    private double _heightScale = 1.0;
    /// <summary>Keyboard height multiplier (0.5 - 2.0).</summary>
    public double HeightScale { get => _heightScale; set => Set(ref _heightScale, Math.Clamp(value, 0.5, 2.0)); }

    private double _keySpacingHorizontal = 5;
    /// <summary>Horizontal gap between keys in DIPs.</summary>
    public double KeySpacingHorizontal { get => _keySpacingHorizontal; set => Set(ref _keySpacingHorizontal, Math.Clamp(value, 0, 24)); }

    private double _keySpacingVertical = 9;
    /// <summary>Vertical gap between rows in DIPs.</summary>
    public double KeySpacingVertical { get => _keySpacingVertical; set => Set(ref _keySpacingVertical, Math.Clamp(value, 0, 32)); }

    private double _fontScale = 1.0;
    /// <summary>Key label font scale.</summary>
    public double FontScale { get => _fontScale; set => Set(ref _fontScale, Math.Clamp(value, 0.5, 2.0)); }

    private double _maxKeyboardWidth = 1100;
    /// <summary>Maximum width of the keys area on large screens (0 = unlimited).</summary>
    public double MaxKeyboardWidth { get => _maxKeyboardWidth; set => Set(ref _maxKeyboardWidth, Math.Max(0, value)); }

    private OneHandedMode _oneHandedMode;
    /// <summary>One-handed (compact) mode.</summary>
    public OneHandedMode OneHandedMode { get => _oneHandedMode; set => Set(ref _oneHandedMode, value); }

    private double _oneHandedWidth = 0.82;
    /// <summary>Relative width of the keyboard in one-handed mode.</summary>
    public double OneHandedWidth { get => _oneHandedWidth; set => Set(ref _oneHandedWidth, Math.Clamp(value, 0.5, 0.95)); }

    private bool _splitKeyboard;
    /// <summary>Splits the keys into two halves (tablets / landscape).</summary>
    public bool SplitKeyboard { get => _splitKeyboard; set => Set(ref _splitKeyboard, value); }

    private double _splitGapRatio = 0.22;
    /// <summary>Width of the split gap relative to the keyboard width.</summary>
    public double SplitGapRatio { get => _splitGapRatio; set => Set(ref _splitGapRatio, Math.Clamp(value, 0.05, 0.5)); }

    private bool _floating;
    /// <summary>Shows the keyboard as a floating, draggable window (supported by <c>ScreenKeyboardHost</c>).</summary>
    public bool Floating { get => _floating; set => Set(ref _floating, value); }

    private double _floatingWidthRatio = 0.6;
    /// <summary>Width of the floating keyboard relative to the host width.</summary>
    public double FloatingWidthRatio { get => _floatingWidthRatio; set => Set(ref _floatingWidthRatio, Math.Clamp(value, 0.3, 1.0)); }

    // ------------------------------------------------------------------ keys & popups

    private bool _showKeyPreview = true;
    /// <summary>Shows an enlarged preview bubble above pressed character keys.</summary>
    public bool ShowKeyPreview { get => _showKeyPreview; set => Set(ref _showKeyPreview, value); }

    private int _longPressDelay = 300;
    /// <summary>Long press delay in milliseconds.</summary>
    public int LongPressDelay { get => _longPressDelay; set => Set(ref _longPressDelay, Math.Clamp(value, 100, 1500)); }

    private int _keyRepeatDelay = 400;
    /// <summary>Delay before key repeat starts (delete, arrows) in milliseconds.</summary>
    public int KeyRepeatDelay { get => _keyRepeatDelay; set => Set(ref _keyRepeatDelay, Math.Clamp(value, 100, 2000)); }

    private int _keyRepeatInterval = 50;
    /// <summary>Key repeat interval in milliseconds.</summary>
    public int KeyRepeatInterval { get => _keyRepeatInterval; set => Set(ref _keyRepeatInterval, Math.Clamp(value, 15, 500)); }

    private bool _doubleTapShiftForCapsLock = true;
    /// <summary>Double tap on shift enables caps lock.</summary>
    public bool DoubleTapShiftForCapsLock { get => _doubleTapShiftForCapsLock; set => Set(ref _doubleTapShiftForCapsLock, value); }

    private int _doubleTapDelay = 350;
    /// <summary>Maximum delay for a double tap in milliseconds.</summary>
    public int DoubleTapDelay { get => _doubleTapDelay; set => Set(ref _doubleTapDelay, Math.Clamp(value, 100, 1000)); }

    // ------------------------------------------------------------------ typing

    private bool _autoCapitalization = true;
    /// <summary>Capitalizes the first letter of sentences.</summary>
    public bool AutoCapitalization { get => _autoCapitalization; set => Set(ref _autoCapitalization, value); }

    private bool _doubleSpacePeriod = true;
    /// <summary>Double space inserts a period.</summary>
    public bool DoubleSpacePeriod { get => _doubleSpacePeriod; set => Set(ref _doubleSpacePeriod, value); }

    private bool _autoSpacePunctuation = true;
    /// <summary>Removes the space before punctuation and inserts one after it where appropriate.</summary>
    public bool AutoSpacePunctuation { get => _autoSpacePunctuation; set => Set(ref _autoSpacePunctuation, value); }

    private bool _showSuggestions = true;
    /// <summary>Shows word suggestions in the smartbar.</summary>
    public bool ShowSuggestions { get => _showSuggestions; set => Set(ref _showSuggestions, value); }

    private bool _autoCorrect = true;
    /// <summary>Automatically replaces misspelled words when a separator is typed.</summary>
    public bool AutoCorrect { get => _autoCorrect; set => Set(ref _autoCorrect, value); }

    private bool _nextWordPrediction = true;
    /// <summary>Predicts the next word after a completed word.</summary>
    public bool NextWordPrediction { get => _nextWordPrediction; set => Set(ref _nextWordPrediction, value); }

    private bool _learnWords = true;
    /// <summary>Learns new words and word pairs from typing (disabled in incognito mode).</summary>
    public bool LearnWords { get => _learnWords; set => Set(ref _learnWords, value); }

    private bool _emojiSuggestions = true;
    /// <summary>Suggests emojis matching the typed word or a <c>:shortcode</c>.</summary>
    public bool EmojiSuggestions { get => _emojiSuggestions; set => Set(ref _emojiSuggestions, value); }

    private int _suggestionCount = 3;
    /// <summary>Number of suggestions shown.</summary>
    public int SuggestionCount { get => _suggestionCount; set => Set(ref _suggestionCount, Math.Clamp(value, 1, 7)); }

    private bool _undoAutoCorrectOnBackspace = true;
    /// <summary>Backspace right after an auto-correction reverts it.</summary>
    public bool UndoAutoCorrectOnBackspace { get => _undoAutoCorrectOnBackspace; set => Set(ref _undoAutoCorrectOnBackspace, value); }

    private bool _incognitoMode;
    /// <summary>Incognito mode: nothing is learned or stored (history, clipboard, emoji recents).</summary>
    public bool IncognitoMode { get => _incognitoMode; set => Set(ref _incognitoMode, value); }

    // ------------------------------------------------------------------ gestures

    private bool _glideTyping = true;
    /// <summary>Enables glide (swipe) typing over letter keys.</summary>
    public bool GlideTyping { get => _glideTyping; set => Set(ref _glideTyping, value); }

    private bool _glideShowTrail = true;
    /// <summary>Draws the glide trail.</summary>
    public bool GlideShowTrail { get => _glideShowTrail; set => Set(ref _glideShowTrail, value); }

    private int _glideTrailDuration = 200;
    /// <summary>How long the trail stays visible after lifting the finger (ms).</summary>
    public int GlideTrailDuration { get => _glideTrailDuration; set => Set(ref _glideTrailDuration, Math.Clamp(value, 0, 2000)); }

    private bool _glidePreview = true;
    /// <summary>Shows live glide suggestions while swiping.</summary>
    public bool GlidePreview { get => _glidePreview; set => Set(ref _glidePreview, value); }

    private double _swipeDistanceThreshold = 32;
    /// <summary>Minimum finger travel (DIPs) for a swipe gesture.</summary>
    public double SwipeDistanceThreshold { get => _swipeDistanceThreshold; set => Set(ref _swipeDistanceThreshold, Math.Clamp(value, 8, 200)); }

    private SwipeAction _swipeUp = SwipeAction.Shift;
    /// <summary>Swipe up on the keyboard.</summary>
    public SwipeAction SwipeUp { get => _swipeUp; set => Set(ref _swipeUp, value); }

    private SwipeAction _swipeDown = SwipeAction.HideKeyboard;
    /// <summary>Swipe down on the keyboard.</summary>
    public SwipeAction SwipeDown { get => _swipeDown; set => Set(ref _swipeDown, value); }

    private SwipeAction _swipeLeft = SwipeAction.SwitchToNextSubtype;
    /// <summary>Swipe left on the keyboard (when glide typing is off).</summary>
    public SwipeAction SwipeLeft { get => _swipeLeft; set => Set(ref _swipeLeft, value); }

    private SwipeAction _swipeRight = SwipeAction.SwitchToPrevSubtype;
    /// <summary>Swipe right on the keyboard (when glide typing is off).</summary>
    public SwipeAction SwipeRight { get => _swipeRight; set => Set(ref _swipeRight, value); }

    private SwipeAction _spaceBarSwipeUp = SwipeAction.SwitchToClipboardContext;
    /// <summary>Swipe up on the space bar.</summary>
    public SwipeAction SpaceBarSwipeUp { get => _spaceBarSwipeUp; set => Set(ref _spaceBarSwipeUp, value); }

    private SwipeAction _spaceBarSwipeLeft = SwipeAction.MoveCursorLeft;
    /// <summary>Swipe left on the space bar (moves continuously while dragging).</summary>
    public SwipeAction SpaceBarSwipeLeft { get => _spaceBarSwipeLeft; set => Set(ref _spaceBarSwipeLeft, value); }

    private SwipeAction _spaceBarSwipeRight = SwipeAction.MoveCursorRight;
    /// <summary>Swipe right on the space bar (moves continuously while dragging).</summary>
    public SwipeAction SpaceBarSwipeRight { get => _spaceBarSwipeRight; set => Set(ref _spaceBarSwipeRight, value); }

    private SwipeAction _spaceBarLongPress = SwipeAction.ShowSubtypePicker;
    /// <summary>Long press on the space bar.</summary>
    public SwipeAction SpaceBarLongPress { get => _spaceBarLongPress; set => Set(ref _spaceBarLongPress, value); }

    private SwipeAction _deleteKeySwipeLeft = SwipeAction.DeleteWordsPrecisely;
    /// <summary>Swipe left from the delete key.</summary>
    public SwipeAction DeleteKeySwipeLeft { get => _deleteKeySwipeLeft; set => Set(ref _deleteKeySwipeLeft, value); }

    private SwipeAction _deleteKeyLongPress = SwipeAction.DeleteCharacter;
    /// <summary>Long press on the delete key (repeats).</summary>
    public SwipeAction DeleteKeyLongPress { get => _deleteKeyLongPress; set => Set(ref _deleteKeyLongPress, value); }

    // ------------------------------------------------------------------ feedback

    private bool _hapticFeedback = true;
    /// <summary>Vibrates on key press (where supported).</summary>
    public bool HapticFeedback { get => _hapticFeedback; set => Set(ref _hapticFeedback, value); }

    private int _hapticDuration = 12;
    /// <summary>Vibration duration in milliseconds.</summary>
    public int HapticDuration { get => _hapticDuration; set => Set(ref _hapticDuration, Math.Clamp(value, 1, 100)); }

    private bool _soundFeedback;
    /// <summary>Plays a click sound on key press (the app provides the sound via the feedback event).</summary>
    public bool SoundFeedback { get => _soundFeedback; set => Set(ref _soundFeedback, value); }

    private double _soundVolume = 0.5;
    /// <summary>Sound volume (0-1).</summary>
    public double SoundVolume { get => _soundVolume; set => Set(ref _soundVolume, Math.Clamp(value, 0, 1)); }

    // ------------------------------------------------------------------ smartbar

    private bool _smartbarEnabled = true;
    /// <summary>Shows the smartbar (suggestions and quick actions) above the keys.</summary>
    public bool SmartbarEnabled { get => _smartbarEnabled; set => Set(ref _smartbarEnabled, value); }

    private SmartbarLayout _smartbarLayout = SmartbarLayout.SuggestionsWithActions;
    /// <summary>Smartbar layout.</summary>
    public SmartbarLayout SmartbarLayout { get => _smartbarLayout; set => Set(ref _smartbarLayout, value); }

    // ------------------------------------------------------------------ theme

    private ThemeMode _themeMode = ThemeMode.FollowSystem;
    /// <summary>Light/dark theme selection.</summary>
    public ThemeMode ThemeMode { get => _themeMode; set => Set(ref _themeMode, value); }

    private string _lightTheme = "floris_day";
    /// <summary>Id of the theme used in light mode.</summary>
    public string LightTheme { get => _lightTheme; set => Set(ref _lightTheme, value ?? "floris_day"); }

    private string _darkTheme = "floris_night";
    /// <summary>Id of the theme used in dark mode.</summary>
    public string DarkTheme { get => _darkTheme; set => Set(ref _darkTheme, value ?? "floris_night"); }

    // ------------------------------------------------------------------ emoji & clipboard

    private EmojiSkinTone _emojiSkinTone = EmojiSkinTone.Default;
    /// <summary>Default emoji skin tone.</summary>
    public EmojiSkinTone EmojiSkinTone { get => _emojiSkinTone; set => Set(ref _emojiSkinTone, value); }

    private bool _emojiHistoryEnabled = true;
    /// <summary>Remembers recently used emojis.</summary>
    public bool EmojiHistoryEnabled { get => _emojiHistoryEnabled; set => Set(ref _emojiHistoryEnabled, value); }

    private bool _clipboardHistoryEnabled = true;
    /// <summary>Keeps a clipboard history.</summary>
    public bool ClipboardHistoryEnabled { get => _clipboardHistoryEnabled; set => Set(ref _clipboardHistoryEnabled, value); }

    private bool _clipboardSuggestion = true;
    /// <summary>Suggests the most recently copied text in the smartbar.</summary>
    public bool ClipboardSuggestion { get => _clipboardSuggestion; set => Set(ref _clipboardSuggestion, value); }

    // ------------------------------------------------------------------ serialization

    /// <summary>Serializes the settings to JSON.</summary>
    public string ToJson() => JsonSerializer.Serialize(this, SettingsJsonContext.Default.KeyboardSettings);

    /// <summary>Creates settings from JSON (unknown properties are ignored).</summary>
    public static KeyboardSettings FromJson(string json) =>
        JsonSerializer.Deserialize(json, SettingsJsonContext.Default.KeyboardSettings) ?? new KeyboardSettings();

    /// <summary>Copies all values from another instance (raising change notifications).</summary>
    public void CopyFrom(KeyboardSettings other)
    {
        var clone = FromJson(other.ToJson());
        foreach (var prop in typeof(KeyboardSettings).GetProperties())
        {
            if (prop.CanRead && prop.CanWrite)
            {
                prop.SetValue(this, prop.GetValue(clone));
            }
        }
    }

    /// <summary>Creates a deep copy.</summary>
    public KeyboardSettings Clone() => FromJson(ToJson());
}

[JsonSourceGenerationOptions(WriteIndented = true, UseStringEnumConverter = true, PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase)]
[JsonSerializable(typeof(KeyboardSettings))]
internal sealed partial class SettingsJsonContext : JsonSerializerContext
{
}
