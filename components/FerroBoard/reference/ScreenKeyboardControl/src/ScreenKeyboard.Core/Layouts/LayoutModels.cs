using ScreenKeyboard.Keys;

namespace ScreenKeyboard.Layouts;

/// <summary>The type of a layout file. Values are compatible with FlorisBoard's layout folders.</summary>
public enum LayoutType
{
    Characters,
    CharactersMod,
    Extension,
    Numeric,
    NumericAdvanced,
    NumericRow,
    Phone,
    Phone2,
    Symbols,
    SymbolsMod,
    Symbols2,
    Symbols2Mod,
}

/// <summary>Helpers for <see cref="LayoutType"/>.</summary>
public static class LayoutTypeExtensions
{
    private static readonly LayoutType[] s_all =
    [
        LayoutType.Characters, LayoutType.CharactersMod, LayoutType.Extension, LayoutType.Numeric, LayoutType.NumericAdvanced,
        LayoutType.NumericRow, LayoutType.Phone, LayoutType.Phone2, LayoutType.Symbols, LayoutType.SymbolsMod,
        LayoutType.Symbols2, LayoutType.Symbols2Mod,
    ];

    /// <summary>Returns the JSON/folder id of a layout type (e.g. <c>"charactersMod"</c>).</summary>
    public static string ToId(this LayoutType type) => type switch
    {
        LayoutType.Characters => "characters",
        LayoutType.CharactersMod => "charactersMod",
        LayoutType.Extension => "extension",
        LayoutType.Numeric => "numeric",
        LayoutType.NumericAdvanced => "numericAdvanced",
        LayoutType.NumericRow => "numericRow",
        LayoutType.Phone => "phone",
        LayoutType.Phone2 => "phone2",
        LayoutType.Symbols => "symbols",
        LayoutType.SymbolsMod => "symbolsMod",
        LayoutType.Symbols2 => "symbols2",
        LayoutType.Symbols2Mod => "symbols2Mod",
        _ => throw new ArgumentOutOfRangeException(nameof(type)),
    };

    /// <summary>Parses a layout type id.</summary>
    public static bool TryParse(string id, out LayoutType type)
    {
        foreach (var t in s_all)
        {
            if (string.Equals(t.ToId(), id, StringComparison.OrdinalIgnoreCase))
            {
                type = t;
                return true;
            }
        }
        type = default;
        return false;
    }
}

/// <summary>A reference to a component of an extension in the form <c>extensionId:componentId</c>.</summary>
public readonly record struct ComponentName(string ExtensionId, string ComponentId)
{
    /// <summary>Parses <c>"ext:component"</c>. A value without colon is resolved against <paramref name="defaultExtension"/>.</summary>
    public static ComponentName Parse(string value, string defaultExtension = "")
    {
        var idx = value.IndexOf(':');
        return idx < 0 ? new ComponentName(defaultExtension, value) : new ComponentName(value.Substring(0, idx), value.Substring(idx + 1));
    }

    /// <inheritdoc />
    public override string ToString() => $"{ExtensionId}:{ComponentId}";

    /// <summary>Implicit conversion from string.</summary>
    public static implicit operator ComponentName(string value) => Parse(value);
}

/// <summary>Row-based arrangement of keys.</summary>
public sealed class LayoutArrangement
{
    /// <summary>Creates an arrangement.</summary>
    public LayoutArrangement(IReadOnlyList<IReadOnlyList<AbstractKeyData>> rows)
    {
        Rows = rows;
    }

    /// <summary>The rows of keys.</summary>
    public IReadOnlyList<IReadOnlyList<AbstractKeyData>> Rows { get; }

    /// <summary>An empty arrangement.</summary>
    public static LayoutArrangement Empty { get; } = new([]);
}

/// <summary>A keyboard layout: metadata and arrangement.</summary>
public sealed class KeyboardLayout
{
    /// <summary>Layout type.</summary>
    public required LayoutType Type { get; init; }

    /// <summary>Extension id this layout belongs to.</summary>
    public required string ExtensionId { get; init; }

    /// <summary>Layout id, unique per type within the extension.</summary>
    public required string Id { get; init; }

    /// <summary>Human readable name.</summary>
    public string Label { get; init; } = string.Empty;

    /// <summary>Authors of the layout.</summary>
    public IReadOnlyList<string> Authors { get; init; } = [];

    /// <summary>Text direction.</summary>
    public LayoutDirection Direction { get; init; } = LayoutDirection.Ltr;

    /// <summary>Optional modifier layout to merge with (e.g. <c>org.florisboard.layouts:arabic</c>).</summary>
    public ComponentName? Modifier { get; init; }

    /// <summary>The key arrangement. May be lazily loaded by the registry.</summary>
    public required LayoutArrangement Arrangement { get; init; }

    /// <summary>Qualified name of this layout.</summary>
    public ComponentName Name => new(ExtensionId, Id);
}

/// <summary>Maps key labels to popup sets, per key variation.</summary>
public sealed class PopupMapping
{
    /// <summary>Extension id.</summary>
    public string ExtensionId { get; init; } = string.Empty;

    /// <summary>Mapping id (usually a language code like <c>"de"</c>).</summary>
    public string Id { get; init; } = string.Empty;

    /// <summary>Authors.</summary>
    public IReadOnlyList<string> Authors { get; init; } = [];

    /// <summary>Popup sets by variation and key label.</summary>
    public IReadOnlyDictionary<KeyVariation, IReadOnlyDictionary<string, PopupSet>> Mappings { get; init; } =
        new Dictionary<KeyVariation, IReadOnlyDictionary<string, PopupSet>>();

    /// <summary>Looks up the popup set for a label in the given variation.</summary>
    public PopupSet? Get(KeyVariation variation, string label) =>
        Mappings.TryGetValue(variation, out var map) && map.TryGetValue(label, out var set) ? set : null;
}

/// <summary>Six currency symbols which replace the <c>currency_slot_N</c> keys.</summary>
public sealed class CurrencySet
{
    public string ExtensionId { get; init; } = string.Empty;
    public required string Id { get; init; }
    public string Label { get; init; } = string.Empty;
    public required IReadOnlyList<TextKeyData> Slots { get; init; }

    /// <summary>Gets the key for a currency slot code (-801..-806).</summary>
    public TextKeyData? GetSlot(int code)
    {
        var index = KeyCode.CurrencySlot1 - code;
        return index >= 0 && index < Slots.Count ? Slots[index] : null;
    }
}

/// <summary>Punctuation rules used for auto-spacing and sentence detection.</summary>
public sealed class PunctuationRule
{
    public string ExtensionId { get; init; } = string.Empty;
    public string Id { get; init; } = "default";
    public string Label { get; init; } = "Default";
    public string SymbolsPrecedingAutoSpace { get; init; } = ".,?‽!\"&%)]}»";
    public string SymbolsFollowingAutoSpace { get; init; } = string.Empty;
    public string SymbolsPrecedingPhantomSpace { get; init; } = ".,;:?‽!&%)]}»©®™";
    public string SymbolsFollowingPhantomSpace { get; init; } = "¿⸘¡([{";
    public string SymbolsTerminatingSentence { get; init; } = ".?‽!";

    /// <summary>The default rule.</summary>
    public static PunctuationRule Default { get; } = new();
}

/// <summary>The layouts used by a subtype for each layout type.</summary>
public sealed record SubtypeLayoutMap
{
    public const string CoreLayouts = "org.florisboard.layouts";

    public ComponentName Characters { get; init; } = new(CoreLayouts, "qwerty");
    public ComponentName Symbols { get; init; } = new(CoreLayouts, "western");
    public ComponentName Symbols2 { get; init; } = new(CoreLayouts, "western");
    public ComponentName Numeric { get; init; } = new(CoreLayouts, "western_arabic");
    public ComponentName NumericAdvanced { get; init; } = new(CoreLayouts, "western_arabic");
    public ComponentName NumericRow { get; init; } = new(CoreLayouts, "western_arabic");
    public ComponentName Phone { get; init; } = new(CoreLayouts, "telpad");
    public ComponentName Phone2 { get; init; } = new(CoreLayouts, "telpad");

    /// <summary>Gets the layout for the given main layout type.</summary>
    public ComponentName Get(LayoutType type) => type switch
    {
        LayoutType.Characters => Characters,
        LayoutType.Symbols => Symbols,
        LayoutType.Symbols2 => Symbols2,
        LayoutType.Numeric => Numeric,
        LayoutType.NumericAdvanced => NumericAdvanced,
        LayoutType.NumericRow => NumericRow,
        LayoutType.Phone => Phone,
        LayoutType.Phone2 => Phone2,
        _ => throw new ArgumentOutOfRangeException(nameof(type)),
    };
}

/// <summary>
/// A subtype is a language + layout combination (e.g. "English (US) - QWERTY"). It references the
/// composer, currency set, popup mapping and layouts to use.
/// </summary>
public sealed record Subtype
{
    public const string CoreComposers = "org.florisboard.composers";
    public const string CoreCurrencySets = "org.florisboard.currencysets";
    public const string CoreLocalization = "org.florisboard.localization";

    /// <summary>BCP-47 language tag (e.g. <c>en-US</c>).</summary>
    public required string LanguageTag { get; init; }

    /// <summary>Optional display name override.</summary>
    public string? DisplayName { get; init; }

    public ComponentName Composer { get; init; } = new(CoreComposers, "appender");
    public ComponentName CurrencySet { get; init; } = new(CoreCurrencySets, "dollar");
    public ComponentName PopupMapping { get; init; } = new(CoreLocalization, "default");
    public ComponentName PunctuationRule { get; init; } = new(CoreLocalization, "default");
    public SubtypeLayoutMap Layouts { get; init; } = new();

    /// <summary>A stable identifier (language tag + characters layout).</summary>
    public string Id => $"{LanguageTag}/{Layouts.Characters.ComponentId}";

    /// <summary>The default English (US) QWERTY subtype.</summary>
    public static Subtype Default { get; } = new()
    {
        LanguageTag = "en-US",
        PopupMapping = new(CoreLocalization, "en"),
    };

    /// <summary>Short language code for the space bar (e.g. <c>"EN"</c>).</summary>
    public string ShortLabel
    {
        get
        {
            var idx = LanguageTag.IndexOf('-');
            return (idx > 0 ? LanguageTag.Substring(0, idx) : LanguageTag).ToUpperInvariant();
        }
    }
}

/// <summary>Definition of a composer contained in an extension.</summary>
public sealed class ComposerDefinition
{
    public string ExtensionId { get; init; } = string.Empty;
    public required string Kind { get; init; }
    public required string Id { get; init; }
    public string Label { get; init; } = string.Empty;
    public IReadOnlyDictionary<string, string> Rules { get; init; } = new Dictionary<string, string>();
}
