namespace ScreenKeyboard.Settings;

/// <summary>What the utility key (next to the space bar) does.</summary>
public enum UtilityKeyAction
{
    /// <summary>Opens the emoji panel.</summary>
    Emoji,
    /// <summary>Switches to the next language.</summary>
    LanguageSwitch,
    /// <summary>Language switch when multiple languages are enabled, otherwise emoji.</summary>
    Dynamic,
    /// <summary>The utility key is hidden.</summary>
    Hidden,
}

/// <summary>What the space bar displays.</summary>
public enum SpaceBarMode
{
    Nothing,
    CurrentLanguage,
    SpaceBarKey,
}

/// <summary>How symbol/number hints on character keys are prioritized on long press.</summary>
public enum KeyHintMode
{
    /// <summary>No hints are shown or offered.</summary>
    Disabled,
    /// <summary>Accent popups are selected first, the hint is also offered.</summary>
    AccentPriority,
    /// <summary>The hint symbol is selected first on long press.</summary>
    HintPriority,
}

/// <summary>One-handed keyboard mode.</summary>
public enum OneHandedMode
{
    Off,
    Left,
    Right,
}

/// <summary>Theme selection mode.</summary>
public enum ThemeMode
{
    /// <summary>Follows the system/app light or dark theme.</summary>
    FollowSystem,
    Light,
    Dark,
}

/// <summary>Actions that can be bound to swipe gestures (compatible with FlorisBoard's swipe actions).</summary>
public enum SwipeAction
{
    NoAction,
    CycleToPreviousKeyboardMode,
    CycleToNextKeyboardMode,
    DeleteCharacter,
    DeleteCharactersPrecisely,
    DeleteWord,
    DeleteWordsPrecisely,
    HideKeyboard,
    InsertSpace,
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorStartOfLine,
    MoveCursorEndOfLine,
    MoveCursorStartOfPage,
    MoveCursorEndOfPage,
    Redo,
    SelectCharactersPrecisely,
    SelectWordsPrecisely,
    Shift,
    ShowSubtypePicker,
    SwitchToPrevSubtype,
    SwitchToNextSubtype,
    SwitchToClipboardContext,
    SwitchToMediaContext,
    SwitchToEditingContext,
    ToggleOneHandedMode,
    ToggleSmartbarVisibility,
    Undo,
}

/// <summary>Swipe gesture directions.</summary>
public enum SwipeDirection
{
    None,
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

/// <summary>Kind of feedback requested by the engine (for haptics and sounds).</summary>
public enum FeedbackKind
{
    KeyPress,
    KeyPressDelete,
    KeyPressSpace,
    KeyPressEnter,
    KeyPressFunction,
    LongPress,
    KeyRepeat,
    GestureStep,
}

/// <summary>Where extra quick actions are displayed in the smartbar.</summary>
public enum SmartbarLayout
{
    /// <summary>Suggestions with a quick actions toggle.</summary>
    SuggestionsWithActions,
    /// <summary>Only suggestions.</summary>
    SuggestionsOnly,
    /// <summary>Only quick actions.</summary>
    ActionsOnly,
}
