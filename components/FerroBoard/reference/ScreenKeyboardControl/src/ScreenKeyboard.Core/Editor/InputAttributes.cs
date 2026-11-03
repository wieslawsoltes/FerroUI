using ScreenKeyboard.Keys;

namespace ScreenKeyboard.Editor;

/// <summary>The kind of content an input field expects.</summary>
public enum InputKind
{
    Text,
    Number,
    Decimal,
    Phone,
    DateTime,
    Email,
    Uri,
    Password,
    NumericPassword,
    Search,
    Chat,
}

/// <summary>The action of the enter key.</summary>
public enum EnterAction
{
    /// <summary>Inserts a new line (multi-line fields) or confirms (single line fields).</summary>
    Default,
    NewLine,
    Go,
    Search,
    Send,
    Next,
    Previous,
    Done,
}

/// <summary>Automatic capitalization behavior.</summary>
public enum CapitalizationMode
{
    None,
    Characters,
    Words,
    Sentences,
}

/// <summary>Describes the field being edited; drives the layout mode, variation, enter key and NLP features.</summary>
public sealed record InputAttributes
{
    /// <summary>Content kind.</summary>
    public InputKind Kind { get; init; } = InputKind.Text;

    /// <summary>Enter key action.</summary>
    public EnterAction EnterAction { get; init; } = EnterAction.Default;

    /// <summary>Capitalization mode.</summary>
    public CapitalizationMode Capitalization { get; init; } = CapitalizationMode.Sentences;

    /// <summary>Whether the field accepts multiple lines.</summary>
    public bool IsMultiline { get; init; }

    /// <summary>Whether suggestions may be shown.</summary>
    public bool AllowSuggestions { get; init; } = true;

    /// <summary>Whether auto-correction may be applied.</summary>
    public bool AllowAutoCorrect { get; init; } = true;

    /// <summary>Whether typed words must not be learned or stored (forces incognito for this field).</summary>
    public bool IsPrivate { get; init; }

    /// <summary>Default attributes for plain text.</summary>
    public static InputAttributes Default { get; } = new();

    /// <summary>Returns <c>true</c> for any password kind.</summary>
    public bool IsPassword => Kind is InputKind.Password or InputKind.NumericPassword;

    /// <summary>The key variation for this field.</summary>
    public KeyVariation Variation => Kind switch
    {
        InputKind.Email => KeyVariation.EmailAddress,
        InputKind.Uri => KeyVariation.Uri,
        InputKind.Password => KeyVariation.Password,
        _ => KeyVariation.Normal,
    };

    /// <summary>The initial keyboard mode for this field.</summary>
    public KeyboardMode InitialMode => Kind switch
    {
        InputKind.Number or InputKind.NumericPassword => KeyboardMode.Numeric,
        InputKind.Decimal => KeyboardMode.NumericAdvanced,
        InputKind.DateTime => KeyboardMode.NumericAdvanced,
        InputKind.Phone => KeyboardMode.Phone,
        _ => KeyboardMode.Characters,
    };

    /// <summary>The effective enter action (resolves <see cref="EnterAction.Default"/>).</summary>
    public EnterAction EffectiveEnterAction => EnterAction switch
    {
        EnterAction.Default when IsMultiline => EnterAction.NewLine,
        EnterAction.Default when Kind == InputKind.Search => EnterAction.Search,
        EnterAction.Default when Kind == InputKind.Uri => EnterAction.Go,
        EnterAction.Default when Kind == InputKind.Chat => EnterAction.Send,
        EnterAction.Default => EnterAction.Done,
        var other => other,
    };

    /// <summary>Whether NLP features (suggestions, learning, autocorrect) apply to this field.</summary>
    public bool SupportsNlp =>
        AllowSuggestions && !IsPassword && Kind is InputKind.Text or InputKind.Search or InputKind.Chat;
}
