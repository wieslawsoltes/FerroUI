namespace ScreenKeyboard.Keys;

/// <summary>The semantic type of a key.</summary>
public enum KeyType
{
    Character,
    EnterEditing,
    Function,
    Lock,
    Modifier,
    Navigation,
    SystemGui,
    Numeric,
    Placeholder,
    Unspecified,
}

/// <summary>The main keyboard modes (a mode maps to a specific layout type).</summary>
public enum KeyboardMode
{
    Characters,
    Symbols,
    Symbols2,
    Numeric,
    NumericAdvanced,
    Phone,
    Phone2,
}

/// <summary>Variation of the characters layout depending on the field type.</summary>
public enum KeyVariation
{
    All,
    Normal,
    EmailAddress,
    Uri,
    Password,
}

/// <summary>Shift state machine value.</summary>
public enum ShiftState
{
    Unshifted,
    ShiftedManual,
    ShiftedAutomatic,
    CapsLock,
}

/// <summary>Layout text direction.</summary>
public enum LayoutDirection
{
    Ltr,
    Rtl,
}

/// <summary>The top-level panel currently shown by the keyboard.</summary>
public enum KeyboardUiMode
{
    /// <summary>The text keyboard (characters, symbols, numeric...).</summary>
    Text,
    /// <summary>Emoji and emoticon panel.</summary>
    Media,
    /// <summary>Clipboard history panel.</summary>
    Clipboard,
    /// <summary>Text editing panel with cursor keys and selection tools.</summary>
    Editing,
}

/// <summary>Helpers for enums.</summary>
public static class KeyEnumExtensions
{
    /// <summary>Parses a FlorisBoard key type string (e.g. <c>"enter_editing"</c>).</summary>
    public static KeyType ParseKeyType(string? value) => value switch
    {
        null or "" => KeyType.Character,
        "character" => KeyType.Character,
        "enter_editing" => KeyType.EnterEditing,
        "function" => KeyType.Function,
        "lock" => KeyType.Lock,
        "modifier" => KeyType.Modifier,
        "navigation" => KeyType.Navigation,
        "system_gui" => KeyType.SystemGui,
        "numeric" => KeyType.Numeric,
        "placeholder" => KeyType.Placeholder,
        _ => KeyType.Unspecified,
    };

    /// <summary>Converts a key type into the FlorisBoard string representation.</summary>
    public static string ToJsonString(this KeyType type) => type switch
    {
        KeyType.Character => "character",
        KeyType.EnterEditing => "enter_editing",
        KeyType.Function => "function",
        KeyType.Lock => "lock",
        KeyType.Modifier => "modifier",
        KeyType.Navigation => "navigation",
        KeyType.SystemGui => "system_gui",
        KeyType.Numeric => "numeric",
        KeyType.Placeholder => "placeholder",
        _ => "unspecified",
    };

    /// <summary>Returns <c>true</c> if the shift state produces upper-case letters.</summary>
    public static bool IsUppercase(this ShiftState state) => state != ShiftState.Unshifted;
}
