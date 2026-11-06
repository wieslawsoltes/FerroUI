using System.Globalization;
using System.Text;
using ScreenKeyboard.Keys;

namespace ScreenKeyboard.Layouts;

/// <summary>
/// Context used to resolve dynamic key data (selectors, automatic upper-casing...) into a
/// concrete <see cref="TextKeyData"/>.
/// </summary>
public interface IKeyEvaluator
{
    /// <summary>Current keyboard mode.</summary>
    KeyboardMode Mode { get; }

    /// <summary>Current shift state.</summary>
    ShiftState ShiftState { get; }

    /// <summary>Current key variation (derived from the input field type).</summary>
    KeyVariation Variation { get; }

    /// <summary>Current layout direction.</summary>
    LayoutDirection Direction { get; }

    /// <summary>Whether half-width characters are active (CJK layouts).</summary>
    bool IsCharHalfWidth { get; }

    /// <summary>Whether Katakana (vs Hiragana) is active (Japanese layouts).</summary>
    bool IsKanaKata { get; }

    /// <summary>Culture used for case conversions.</summary>
    CultureInfo Culture { get; }
}

/// <summary>A simple immutable evaluator implementation.</summary>
public sealed record KeyEvaluator(
    KeyboardMode Mode = KeyboardMode.Characters,
    ShiftState ShiftState = ShiftState.Unshifted,
    KeyVariation Variation = KeyVariation.Normal,
    LayoutDirection Direction = LayoutDirection.Ltr,
    bool IsCharHalfWidth = false,
    bool IsKanaKata = false,
    CultureInfo? CultureOverride = null) : IKeyEvaluator
{
    /// <summary>A default evaluator (characters, unshifted, LTR).</summary>
    public static KeyEvaluator Default { get; } = new();

    /// <inheritdoc />
    public CultureInfo Culture => CultureOverride ?? CultureInfo.InvariantCulture;
}

/// <summary>
/// Base class for all key definitions found in a layout arrangement. Mirrors the FlorisBoard
/// layout JSON model: a key is either concrete (<see cref="TextKeyData"/>) or a selector that
/// resolves to a concrete key depending on the keyboard state.
/// </summary>
public abstract class AbstractKeyData
{
    /// <summary>Resolves the key into concrete data for the given state; <c>null</c> hides the key.</summary>
    public abstract TextKeyData? Compute(IKeyEvaluator evaluator);

    /// <summary>Returns the character output of the key (or a display string).</summary>
    public abstract string AsString(bool isForDisplay);
}

/// <summary>A concrete key with a code, label, type and optional popup keys.</summary>
public class TextKeyData : AbstractKeyData
{
    /// <summary>Default group.</summary>
    public const int GroupDefault = 0;
    /// <summary>Group of the key left of the space bar (popup mapping label <c>~left</c>).</summary>
    public const int GroupLeft = 1;
    /// <summary>Group of the key right of the space bar (popup mapping label <c>~right</c>).</summary>
    public const int GroupRight = 2;
    /// <summary>Group of the enter key (popup mapping label <c>~enter</c>).</summary>
    public const int GroupEnter = 3;
    /// <summary>Group of the kana key (popup mapping label <c>~kana</c>).</summary>
    public const int GroupKana = 97;

    /// <summary>The key type.</summary>
    public KeyType Type { get; init; } = KeyType.Character;

    /// <summary>Unicode code point (positive) or function code (negative, see <see cref="KeyCode"/>).</summary>
    public int Code { get; init; }

    /// <summary>The label (for characters the character itself, for functions a name such as <c>"delete"</c>).</summary>
    public string Label { get; init; } = string.Empty;

    /// <summary>Popup group of the key.</summary>
    public int GroupId { get; init; }

    /// <summary>Explicit popup keys defined in the layout.</summary>
    public PopupSet? Popup { get; init; }

    /// <summary>Unspecified/empty key.</summary>
    public static TextKeyData Unspecified { get; } = new() { Type = KeyType.Unspecified, Code = KeyCode.Unspecified, Label = string.Empty };

    /// <summary>Space key.</summary>
    public static TextKeyData SpaceKey { get; } = new() { Code = KeyCode.Space, Label = "space" };

    /// <summary>Creates a character key.</summary>
    public static TextKeyData Char(string text, PopupSet? popup = null)
    {
        var code = char.ConvertToUtf32(text, 0);
        return new TextKeyData { Code = code, Label = text, Popup = popup };
    }

    /// <summary>Creates a function key from a <see cref="KeyCode"/> value.</summary>
    public static TextKeyData Function(int code, KeyType type = KeyType.Function, string? label = null) =>
        new() { Code = code, Type = type, Label = label ?? KeyCode.GetName(code) ?? string.Empty };

    /// <summary>Gets the popup mapping lookup label (e.g. <c>"a"</c>, <c>"~enter"</c>).</summary>
    public string PopupLookupLabel => GroupId switch
    {
        GroupLeft => "~left",
        GroupRight => "~right",
        GroupEnter => "~enter",
        GroupKana => "~kana",
        _ => Label,
    };

    /// <summary>Returns <c>true</c> if this key is a space key.</summary>
    public bool IsSpace => Type == KeyType.Character && KeyCode.IsSpace(Code) || Code == KeyCode.Space;

    /// <summary>Returns <c>true</c> if the key produces text when pressed.</summary>
    public virtual bool ProducesText => Type is KeyType.Character or KeyType.Numeric && KeyCode.IsCharacter(Code);

    /// <inheritdoc />
    public override TextKeyData? Compute(IKeyEvaluator evaluator) => this;

    /// <inheritdoc />
    public override string AsString(bool isForDisplay)
    {
        if (KeyCode.IsCharacter(Code) && Code != KeyCode.Enter && Code != KeyCode.Tab)
        {
            return char.ConvertFromUtf32(Code);
        }
        return isForDisplay || Code == KeyCode.Unspecified ? Label : string.Empty;
    }

    /// <summary>Creates a modified copy.</summary>
    public virtual TextKeyData With(int? code = null, string? label = null, KeyType? type = null, PopupSet? popup = null, int? groupId = null) => new()
    {
        Code = code ?? Code,
        Label = label ?? Label,
        Type = type ?? Type,
        Popup = popup ?? Popup,
        GroupId = groupId ?? GroupId,
    };

    /// <inheritdoc />
    public override string ToString() => $"{GetType().Name}(code={Code}, label=\"{Label}\", type={Type.ToJsonString()})";
}

/// <summary>A character key that automatically becomes upper case when shift is active.</summary>
public sealed class AutoTextKeyData : TextKeyData
{
    /// <inheritdoc />
    public override TextKeyData? Compute(IKeyEvaluator evaluator)
    {
        if (!evaluator.ShiftState.IsUppercase() || !KeyCode.IsCharacter(Code))
        {
            return this;
        }
        var upper = ToUpper(Label, evaluator.Culture);
        if (upper == Label)
        {
            return this;
        }
        return CreateFromText(upper, Type, GroupId, Popup);
    }

    /// <inheritdoc />
    public override TextKeyData With(int? code = null, string? label = null, KeyType? type = null, PopupSet? popup = null, int? groupId = null) => new AutoTextKeyData
    {
        Code = code ?? Code,
        Label = label ?? Label,
        Type = type ?? Type,
        Popup = popup ?? Popup,
        GroupId = groupId ?? GroupId,
    };

    internal static string ToUpper(string text, CultureInfo culture)
    {
        // German sharp s has a dedicated capital letter; avoid "SS" expansion on a single key.
        if (text == "ß")
        {
            return "ẞ";
        }
        return text.ToUpper(culture);
    }

    internal static TextKeyData CreateFromText(string text, KeyType type, int groupId, PopupSet? popup)
    {
        var codePoints = TextUtils.GetCodePoints(text);
        if (codePoints.Length == 1)
        {
            return new TextKeyData { Code = codePoints[0], Label = text, Type = type, GroupId = groupId, Popup = popup };
        }
        return new MultiTextKeyData { CodePoints = codePoints, Label = text, Type = type, GroupId = groupId, Popup = popup };
    }
}

/// <summary>A key which outputs multiple code points at once.</summary>
public sealed class MultiTextKeyData : TextKeyData
{
    /// <summary>The code points to output.</summary>
    public int[] CodePoints { get; init; } = [];

    /// <summary>Creates a new multi text key.</summary>
    public MultiTextKeyData()
    {
        Code = KeyCode.MultipleCodePoints;
    }

    /// <inheritdoc />
    public override bool ProducesText => CodePoints.Length > 0;

    /// <inheritdoc />
    public override string AsString(bool isForDisplay)
    {
        var sb = new StringBuilder();
        foreach (var cp in CodePoints)
        {
            sb.Append(char.ConvertFromUtf32(cp));
        }
        return sb.ToString();
    }

    /// <inheritdoc />
    public override TextKeyData With(int? code = null, string? label = null, KeyType? type = null, PopupSet? popup = null, int? groupId = null) => new MultiTextKeyData
    {
        CodePoints = CodePoints,
        Label = label ?? Label,
        Type = type ?? Type,
        Popup = popup ?? Popup,
        GroupId = groupId ?? GroupId,
    };
}

/// <summary>Selects between two keys based on case.</summary>
public sealed class CaseSelector : AbstractKeyData
{
    public required AbstractKeyData Lower { get; init; }
    public required AbstractKeyData Upper { get; init; }

    /// <inheritdoc />
    public override TextKeyData? Compute(IKeyEvaluator evaluator) =>
        (evaluator.ShiftState.IsUppercase() ? Upper : Lower).Compute(evaluator);

    /// <inheritdoc />
    public override string AsString(bool isForDisplay) => string.Empty;
}

/// <summary>Selects a key based on the exact shift state.</summary>
public sealed class ShiftStateSelector : AbstractKeyData
{
    public AbstractKeyData? Unshifted { get; init; }
    public AbstractKeyData? Shifted { get; init; }
    public AbstractKeyData? ShiftedManual { get; init; }
    public AbstractKeyData? ShiftedAutomatic { get; init; }
    public AbstractKeyData? CapsLock { get; init; }
    public AbstractKeyData? Default { get; init; }

    /// <inheritdoc />
    public override TextKeyData? Compute(IKeyEvaluator evaluator)
    {
        var data = evaluator.ShiftState switch
        {
            ShiftState.Unshifted => Unshifted ?? Default,
            ShiftState.ShiftedManual => ShiftedManual ?? Shifted ?? Default,
            ShiftState.ShiftedAutomatic => ShiftedAutomatic ?? Shifted ?? Default,
            ShiftState.CapsLock => CapsLock ?? Shifted ?? Default,
            _ => Default,
        };
        return data?.Compute(evaluator);
    }

    /// <inheritdoc />
    public override string AsString(bool isForDisplay) => string.Empty;
}

/// <summary>Selects a key based on the input field variation (email, URI, password...).</summary>
public sealed class VariationSelector : AbstractKeyData
{
    public AbstractKeyData? Default { get; init; }
    public AbstractKeyData? Email { get; init; }
    public AbstractKeyData? Uri { get; init; }
    public AbstractKeyData? Normal { get; init; }
    public AbstractKeyData? Password { get; init; }

    /// <inheritdoc />
    public override TextKeyData? Compute(IKeyEvaluator evaluator)
    {
        var data = evaluator.Variation switch
        {
            KeyVariation.EmailAddress => Email ?? Default,
            KeyVariation.Uri => Uri ?? Default,
            KeyVariation.Normal => Normal ?? Default,
            KeyVariation.Password => Password ?? Default,
            _ => Default,
        };
        return data?.Compute(evaluator);
    }

    /// <inheritdoc />
    public override string AsString(bool isForDisplay) => string.Empty;
}

/// <summary>Selects a key based on layout direction.</summary>
public sealed class LayoutDirectionSelector : AbstractKeyData
{
    public required AbstractKeyData Ltr { get; init; }
    public required AbstractKeyData Rtl { get; init; }

    /// <inheritdoc />
    public override TextKeyData? Compute(IKeyEvaluator evaluator) =>
        (evaluator.Direction == LayoutDirection.Rtl ? Rtl : Ltr).Compute(evaluator);

    /// <inheritdoc />
    public override string AsString(bool isForDisplay) => string.Empty;
}

/// <summary>Selects a key based on full/half character width (CJK).</summary>
public sealed class CharWidthSelector : AbstractKeyData
{
    public AbstractKeyData? Full { get; init; }
    public AbstractKeyData? Half { get; init; }

    /// <inheritdoc />
    public override TextKeyData? Compute(IKeyEvaluator evaluator) =>
        (evaluator.IsCharHalfWidth ? Half : Full)?.Compute(evaluator);

    /// <inheritdoc />
    public override string AsString(bool isForDisplay) => string.Empty;
}

/// <summary>Selects a key based on Hiragana/Katakana mode.</summary>
public sealed class KanaSelector : AbstractKeyData
{
    public required AbstractKeyData Hira { get; init; }
    public required AbstractKeyData Kata { get; init; }

    /// <inheritdoc />
    public override TextKeyData? Compute(IKeyEvaluator evaluator) =>
        (evaluator.IsKanaKata ? Kata : Hira).Compute(evaluator);

    /// <inheritdoc />
    public override string AsString(bool isForDisplay) => string.Empty;
}

/// <summary>A set of popup keys: an optional main (primary) key and a list of relevant keys.</summary>
public sealed class PopupSet
{
    /// <summary>The primary popup key (shown as hint and pre-selected on long press).</summary>
    public AbstractKeyData? Main { get; init; }

    /// <summary>Additional popup keys.</summary>
    public IReadOnlyList<AbstractKeyData> Relevant { get; init; } = [];

    /// <summary>An empty popup set.</summary>
    public static PopupSet Empty { get; } = new();

    /// <summary>Returns <c>true</c> if there are no popup keys.</summary>
    public bool IsEmpty => Main is null && Relevant.Count == 0;
}
