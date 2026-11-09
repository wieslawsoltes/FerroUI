using System.Text;
using System.Text.Json;
using ScreenKeyboard.Keys;

namespace ScreenKeyboard.Layouts;

/// <summary>
/// Reads and writes FlorisBoard compatible layout, popup mapping and extension JSON files.
/// The implementation is reflection free and therefore trimming/AOT safe.
/// </summary>
public static class LayoutJson
{
    private static readonly JsonDocumentOptions s_options = new()
    {
        AllowTrailingCommas = true,
        CommentHandling = JsonCommentHandling.Skip,
    };

    /// <summary>Parses a layout arrangement (array of rows of keys).</summary>
    public static LayoutArrangement ReadArrangement(string json)
    {
        using var doc = JsonDocument.Parse(json, s_options);
        var root = doc.RootElement;
        // Allow both the plain FlorisBoard format ([[...]]) and an object with an "arrangement" property.
        if (root.ValueKind == JsonValueKind.Object && root.TryGetProperty("arrangement", out var arr))
        {
            root = arr;
        }
        return ReadArrangement(root);
    }

    /// <summary>Parses a layout arrangement element.</summary>
    public static LayoutArrangement ReadArrangement(JsonElement root)
    {
        if (root.ValueKind != JsonValueKind.Array)
        {
            throw new FormatException("Layout arrangement must be a JSON array of rows.");
        }
        var rows = new List<IReadOnlyList<AbstractKeyData>>();
        foreach (var rowEl in root.EnumerateArray())
        {
            var row = new List<AbstractKeyData>();
            foreach (var keyEl in rowEl.EnumerateArray())
            {
                row.Add(ReadKey(keyEl));
            }
            rows.Add(row);
        }
        return new LayoutArrangement(rows);
    }

    /// <summary>
    /// Parses a complete standalone layout document: <c>{ "type": "characters", "id": "...", "label": "...",
    /// "direction": "ltr", "modifier": "...", "arrangement": [[...]] }</c>.
    /// </summary>
    public static KeyboardLayout ReadLayoutDocument(string json, string extensionId = "custom")
    {
        using var doc = JsonDocument.Parse(json, s_options);
        var root = doc.RootElement;
        var typeId = GetString(root, "type") ?? "characters";
        if (!LayoutTypeExtensions.TryParse(typeId, out var type))
        {
            throw new FormatException($"Unknown layout type '{typeId}'.");
        }
        var id = GetString(root, "id") ?? throw new FormatException("Layout document requires an 'id'.");
        return new KeyboardLayout
        {
            Type = type,
            ExtensionId = extensionId,
            Id = id,
            Label = GetString(root, "label") ?? id,
            Authors = GetStringArray(root, "authors"),
            Direction = ParseDirection(GetString(root, "direction")),
            Modifier = GetString(root, "modifier") is { } mod ? (ComponentName?)ComponentName.Parse(mod, extensionId) : null,
            Arrangement = root.TryGetProperty("arrangement", out var arr) ? ReadArrangement(arr) : LayoutArrangement.Empty,
        };
    }

    /// <summary>Parses a single key element.</summary>
    public static AbstractKeyData ReadKey(JsonElement el)
    {
        if (el.ValueKind == JsonValueKind.String)
        {
            // Shorthand: "q" → auto text key.
            var text = el.GetString() ?? string.Empty;
            var cps = TextUtils.GetCodePoints(text);
            if (cps.Length == 1)
            {
                return new AutoTextKeyData { Code = cps[0], Label = text };
            }
            return new MultiTextKeyData { CodePoints = cps, Label = text };
        }
        if (el.ValueKind != JsonValueKind.Object)
        {
            throw new FormatException($"Invalid key definition: {el.GetRawText()}");
        }

        var kind = GetString(el, "$") ?? "text_key";
        switch (kind)
        {
            case "text_key":
            case "auto_text_key":
            {
                var type = KeyEnumExtensions.ParseKeyType(GetString(el, "type"));
                var code = GetInt(el, "code") ?? 0;
                var label = GetString(el, "label") ?? string.Empty;
                if (code == 0 && type != KeyType.Placeholder && label.Length > 0)
                {
                    if (KeyCode.TryGetCode(label, out var fn))
                    {
                        code = fn;
                    }
                    else
                    {
                        var cps = TextUtils.GetCodePoints(label);
                        if (cps.Length == 1)
                        {
                            code = cps[0];
                        }
                    }
                }
                if (label.Length == 0 && KeyCode.IsCharacter(code))
                {
                    label = char.ConvertFromUtf32(code);
                }
                var groupId = GetInt(el, "groupId") ?? 0;
                var popup = el.TryGetProperty("popup", out var popupEl) ? ReadPopupSet(popupEl) : null;
                return kind == "auto_text_key"
                    ? new AutoTextKeyData { Type = type, Code = code, Label = label, GroupId = groupId, Popup = popup }
                    : new TextKeyData { Type = type, Code = code, Label = label, GroupId = groupId, Popup = popup };
            }
            case "multi_text_key":
            {
                var cps = new List<int>();
                if (el.TryGetProperty("codePoints", out var cpsEl))
                {
                    foreach (var c in cpsEl.EnumerateArray())
                    {
                        cps.Add(c.GetInt32());
                    }
                }
                var label = GetString(el, "label") ?? string.Concat(cps.Select(char.ConvertFromUtf32));
                return new MultiTextKeyData
                {
                    CodePoints = cps.ToArray(),
                    Label = label,
                    Type = KeyEnumExtensions.ParseKeyType(GetString(el, "type")),
                    GroupId = GetInt(el, "groupId") ?? 0,
                    Popup = el.TryGetProperty("popup", out var popupEl) ? ReadPopupSet(popupEl) : null,
                };
            }
            case "case_selector":
                return new CaseSelector { Lower = ReadRequired(el, "lower"), Upper = ReadRequired(el, "upper") };
            case "shift_state_selector":
                return new ShiftStateSelector
                {
                    Unshifted = ReadOptional(el, "unshifted"),
                    Shifted = ReadOptional(el, "shifted"),
                    ShiftedManual = ReadOptional(el, "shiftedManual"),
                    ShiftedAutomatic = ReadOptional(el, "shiftedAutomatic"),
                    CapsLock = ReadOptional(el, "capsLock"),
                    Default = ReadOptional(el, "default"),
                };
            case "variation_selector":
                return new VariationSelector
                {
                    Default = ReadOptional(el, "default"),
                    Email = ReadOptional(el, "email"),
                    Uri = ReadOptional(el, "uri"),
                    Normal = ReadOptional(el, "normal"),
                    Password = ReadOptional(el, "password"),
                };
            case "layout_direction_selector":
                return new LayoutDirectionSelector { Ltr = ReadRequired(el, "ltr"), Rtl = ReadRequired(el, "rtl") };
            case "char_width_selector":
                return new CharWidthSelector { Full = ReadOptional(el, "full"), Half = ReadOptional(el, "half") };
            case "kana_selector":
                return new KanaSelector { Hira = ReadRequired(el, "hira"), Kata = ReadRequired(el, "kata") };
            default:
                throw new FormatException($"Unknown key data type '{kind}'.");
        }
    }

    /// <summary>Parses a popup set element (<c>{ "main": ..., "relevant": [...] }</c>).</summary>
    public static PopupSet ReadPopupSet(JsonElement el)
    {
        if (el.ValueKind == JsonValueKind.Array)
        {
            return new PopupSet { Relevant = el.EnumerateArray().Select(ReadKey).ToList() };
        }
        var main = el.TryGetProperty("main", out var mainEl) && mainEl.ValueKind != JsonValueKind.Null ? ReadKey(mainEl) : null;
        var relevant = el.TryGetProperty("relevant", out var relEl) && relEl.ValueKind == JsonValueKind.Array
            ? relEl.EnumerateArray().Select(ReadKey).ToList()
            : new List<AbstractKeyData>();
        return new PopupSet { Main = main, Relevant = relevant };
    }

    /// <summary>Parses a popup mapping JSON document.</summary>
    public static PopupMapping ReadPopupMapping(string json, string id, string extensionId, IReadOnlyList<string>? authors = null)
    {
        using var doc = JsonDocument.Parse(json, s_options);
        var result = new Dictionary<KeyVariation, IReadOnlyDictionary<string, PopupSet>>();
        foreach (var variationProp in doc.RootElement.EnumerateObject())
        {
            var variation = variationProp.Name switch
            {
                "all" => KeyVariation.All,
                "normal" => KeyVariation.Normal,
                "email" or "emailAddress" => KeyVariation.EmailAddress,
                "uri" => KeyVariation.Uri,
                "password" => KeyVariation.Password,
                _ => (KeyVariation?)null,
            };
            if (variation is null)
            {
                continue;
            }
            var map = new Dictionary<string, PopupSet>(StringComparer.Ordinal);
            foreach (var labelProp in variationProp.Value.EnumerateObject())
            {
                map[labelProp.Name] = ReadPopupSet(labelProp.Value);
            }
            result[variation.Value] = map;
        }
        return new PopupMapping { Id = id, ExtensionId = extensionId, Authors = authors ?? [], Mappings = result };
    }

    /// <summary>Parses an extension manifest (<c>extension.json</c>).</summary>
    public static KeyboardExtensionManifest ReadExtensionManifest(string json)
    {
        using var doc = JsonDocument.Parse(json, s_options);
        var root = doc.RootElement;
        var meta = root.GetProperty("meta");
        var extId = GetString(meta, "id") ?? throw new FormatException("Extension meta requires an 'id'.");

        var layouts = new List<LayoutMetadata>();
        if (root.TryGetProperty("layouts", out var layoutsEl))
        {
            foreach (var typeProp in layoutsEl.EnumerateObject())
            {
                if (!LayoutTypeExtensions.TryParse(typeProp.Name, out var type))
                {
                    continue;
                }
                foreach (var l in typeProp.Value.EnumerateArray())
                {
                    var id = GetString(l, "id")!;
                    layouts.Add(new LayoutMetadata
                    {
                        Type = type,
                        Id = id,
                        Label = GetString(l, "label") ?? id,
                        Authors = GetStringArray(l, "authors"),
                        Direction = ParseDirection(GetString(l, "direction")),
                        Modifier = GetString(l, "modifier") is { } mod ? (ComponentName?)ComponentName.Parse(mod, extId) : null,
                    });
                }
            }
        }

        var popupMappings = new List<(string Id, IReadOnlyList<string> Authors)>();
        if (root.TryGetProperty("popupMappings", out var pmEl))
        {
            foreach (var pm in pmEl.EnumerateArray())
            {
                popupMappings.Add((GetString(pm, "id")!, GetStringArray(pm, "authors")));
            }
        }

        var subtypes = new List<Subtype>();
        if (root.TryGetProperty("subtypePresets", out var stEl))
        {
            foreach (var st in stEl.EnumerateArray())
            {
                subtypes.Add(ReadSubtype(st));
            }
        }

        var currencySets = new List<CurrencySet>();
        if (root.TryGetProperty("currencySets", out var csEl))
        {
            foreach (var cs in csEl.EnumerateArray())
            {
                var slots = cs.GetProperty("slots").EnumerateArray().Select(s => (TextKeyData)ReadKey(s)).ToList();
                currencySets.Add(new CurrencySet { ExtensionId = extId, Id = GetString(cs, "id")!, Label = GetString(cs, "label") ?? string.Empty, Slots = slots });
            }
        }

        var composers = new List<ComposerDefinition>();
        if (root.TryGetProperty("composers", out var cpEl))
        {
            foreach (var cp in cpEl.EnumerateArray())
            {
                var kind = GetString(cp, "$") ?? "appender";
                var rules = new Dictionary<string, string>(StringComparer.Ordinal);
                if (cp.TryGetProperty("rules", out var rulesEl))
                {
                    foreach (var r in rulesEl.EnumerateObject())
                    {
                        rules[r.Name] = r.Value.GetString() ?? string.Empty;
                    }
                }
                composers.Add(new ComposerDefinition
                {
                    ExtensionId = extId,
                    Kind = kind,
                    Id = GetString(cp, "id") ?? kind,
                    Label = GetString(cp, "label") ?? kind,
                    Rules = rules,
                });
            }
        }

        var punctuation = new List<PunctuationRule>();
        if (root.TryGetProperty("punctuationRules", out var prEl))
        {
            foreach (var pr in prEl.EnumerateArray())
            {
                punctuation.Add(new PunctuationRule
                {
                    ExtensionId = extId,
                    Id = GetString(pr, "id") ?? "default",
                    Label = GetString(pr, "label") ?? "Default",
                    SymbolsPrecedingAutoSpace = GetString(pr, "symbolsPrecedingAutoSpace") ?? string.Empty,
                    SymbolsFollowingAutoSpace = GetString(pr, "symbolsFollowingAutoSpace") ?? string.Empty,
                    SymbolsPrecedingPhantomSpace = GetString(pr, "symbolsPrecedingPhantomSpace") ?? string.Empty,
                    SymbolsFollowingPhantomSpace = GetString(pr, "symbolsFollowingPhantomSpace") ?? string.Empty,
                    SymbolsTerminatingSentence = GetString(pr, "symbolsTerminatingSentence") ?? ".?!",
                });
            }
        }

        return new KeyboardExtensionManifest
        {
            Id = extId,
            Version = GetString(meta, "version") ?? "0.0.0",
            Title = GetString(meta, "title") ?? extId,
            Description = GetString(meta, "description") ?? string.Empty,
            License = GetString(meta, "license") ?? string.Empty,
            Dependencies = GetStringArray(root, "dependencies"),
            Layouts = layouts,
            PopupMappings = popupMappings,
            SubtypePresets = subtypes,
            CurrencySets = currencySets,
            Composers = composers,
            PunctuationRules = punctuation,
        };
    }

    /// <summary>Parses a subtype (preset) element.</summary>
    public static Subtype ReadSubtype(JsonElement st)
    {
        var map = new SubtypeLayoutMap();
        if (st.TryGetProperty("preferred", out var pref) || st.TryGetProperty("layouts", out pref))
        {
            foreach (var p in pref.EnumerateObject())
            {
                var name = ComponentName.Parse(p.Value.GetString()!, SubtypeLayoutMap.CoreLayouts);
                map = p.Name switch
                {
                    "characters" => map with { Characters = name },
                    "symbols" => map with { Symbols = name },
                    "symbols2" => map with { Symbols2 = name },
                    "numeric" => map with { Numeric = name },
                    "numericAdvanced" => map with { NumericAdvanced = name },
                    "numericRow" => map with { NumericRow = name },
                    "phone" => map with { Phone = name },
                    "phone2" => map with { Phone2 = name },
                    _ => map,
                };
            }
        }
        var subtype = new Subtype { LanguageTag = GetString(st, "languageTag") ?? "en-US", Layouts = map, DisplayName = GetString(st, "displayName") };
        if (GetString(st, "composer") is { } composer)
        {
            subtype = subtype with { Composer = ComponentName.Parse(composer, Subtype.CoreComposers) };
        }
        if (GetString(st, "currencySet") is { } currency)
        {
            subtype = subtype with { CurrencySet = ComponentName.Parse(currency, Subtype.CoreCurrencySets) };
        }
        if (GetString(st, "popupMapping") is { } popup)
        {
            subtype = subtype with { PopupMapping = ComponentName.Parse(popup, Subtype.CoreLocalization) };
        }
        if (GetString(st, "punctuationRule") is { } punct)
        {
            subtype = subtype with { PunctuationRule = ComponentName.Parse(punct, Subtype.CoreLocalization) };
        }
        return subtype;
    }

    /// <summary>Parses a subtype from a JSON string.</summary>
    public static Subtype ReadSubtype(string json)
    {
        using var doc = JsonDocument.Parse(json, s_options);
        return ReadSubtype(doc.RootElement);
    }

    // ---------------------------------------------------------------- writing

    /// <summary>Serializes an arrangement to FlorisBoard compatible JSON.</summary>
    public static string WriteArrangement(LayoutArrangement arrangement, bool indented = true)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = indented, Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping }))
        {
            writer.WriteStartArray();
            foreach (var row in arrangement.Rows)
            {
                writer.WriteStartArray();
                foreach (var key in row)
                {
                    WriteKey(writer, key);
                }
                writer.WriteEndArray();
            }
            writer.WriteEndArray();
        }
        return Encoding.UTF8.GetString(stream.ToArray());
    }

    /// <summary>Serializes a complete standalone layout document.</summary>
    public static string WriteLayoutDocument(KeyboardLayout layout, bool indented = true)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = indented, Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping }))
        {
            writer.WriteStartObject();
            writer.WriteString("type", layout.Type.ToId());
            writer.WriteString("id", layout.Id);
            writer.WriteString("label", layout.Label);
            writer.WriteString("direction", layout.Direction == LayoutDirection.Rtl ? "rtl" : "ltr");
            if (layout.Modifier is { } mod)
            {
                writer.WriteString("modifier", mod.ToString());
            }
            if (layout.Authors.Count > 0)
            {
                writer.WriteStartArray("authors");
                foreach (var a in layout.Authors)
                {
                    writer.WriteStringValue(a);
                }
                writer.WriteEndArray();
            }
            writer.WriteStartArray("arrangement");
            foreach (var row in layout.Arrangement.Rows)
            {
                writer.WriteStartArray();
                foreach (var key in row)
                {
                    WriteKey(writer, key);
                }
                writer.WriteEndArray();
            }
            writer.WriteEndArray();
            writer.WriteEndObject();
        }
        return Encoding.UTF8.GetString(stream.ToArray());
    }

    /// <summary>Writes a key definition.</summary>
    public static void WriteKey(Utf8JsonWriter writer, AbstractKeyData key)
    {
        writer.WriteStartObject();
        switch (key)
        {
            case MultiTextKeyData multi:
                writer.WriteString("$", "multi_text_key");
                writer.WriteStartArray("codePoints");
                foreach (var cp in multi.CodePoints)
                {
                    writer.WriteNumberValue(cp);
                }
                writer.WriteEndArray();
                writer.WriteString("label", multi.Label);
                WriteCommon(writer, multi, writeCode: false);
                break;
            case AutoTextKeyData auto:
                writer.WriteString("$", "auto_text_key");
                WriteCommon(writer, auto, writeCode: true);
                break;
            case TextKeyData text:
                WriteCommon(writer, text, writeCode: true);
                break;
            case CaseSelector cs:
                writer.WriteString("$", "case_selector");
                WriteChild(writer, "lower", cs.Lower);
                WriteChild(writer, "upper", cs.Upper);
                break;
            case ShiftStateSelector ss:
                writer.WriteString("$", "shift_state_selector");
                WriteChild(writer, "unshifted", ss.Unshifted);
                WriteChild(writer, "shifted", ss.Shifted);
                WriteChild(writer, "shiftedManual", ss.ShiftedManual);
                WriteChild(writer, "shiftedAutomatic", ss.ShiftedAutomatic);
                WriteChild(writer, "capsLock", ss.CapsLock);
                WriteChild(writer, "default", ss.Default);
                break;
            case VariationSelector vs:
                writer.WriteString("$", "variation_selector");
                WriteChild(writer, "default", vs.Default);
                WriteChild(writer, "email", vs.Email);
                WriteChild(writer, "uri", vs.Uri);
                WriteChild(writer, "normal", vs.Normal);
                WriteChild(writer, "password", vs.Password);
                break;
            case LayoutDirectionSelector ld:
                writer.WriteString("$", "layout_direction_selector");
                WriteChild(writer, "ltr", ld.Ltr);
                WriteChild(writer, "rtl", ld.Rtl);
                break;
            case CharWidthSelector cw:
                writer.WriteString("$", "char_width_selector");
                WriteChild(writer, "full", cw.Full);
                WriteChild(writer, "half", cw.Half);
                break;
            case KanaSelector ks:
                writer.WriteString("$", "kana_selector");
                WriteChild(writer, "hira", ks.Hira);
                WriteChild(writer, "kata", ks.Kata);
                break;
            default:
                throw new NotSupportedException($"Cannot serialize key type {key.GetType().Name}.");
        }
        writer.WriteEndObject();
    }

    private static void WriteCommon(Utf8JsonWriter writer, TextKeyData key, bool writeCode)
    {
        if (writeCode)
        {
            writer.WriteNumber("code", key.Code);
            writer.WriteString("label", key.Label);
        }
        if (key.Type != KeyType.Character)
        {
            writer.WriteString("type", key.Type.ToJsonString());
        }
        if (key.GroupId != 0)
        {
            writer.WriteNumber("groupId", key.GroupId);
        }
        if (key.Popup is { IsEmpty: false } popup)
        {
            writer.WriteStartObject("popup");
            if (popup.Main is not null)
            {
                writer.WritePropertyName("main");
                WriteKey(writer, popup.Main);
            }
            if (popup.Relevant.Count > 0)
            {
                writer.WriteStartArray("relevant");
                foreach (var r in popup.Relevant)
                {
                    WriteKey(writer, r);
                }
                writer.WriteEndArray();
            }
            writer.WriteEndObject();
        }
    }

    private static void WriteChild(Utf8JsonWriter writer, string name, AbstractKeyData? data)
    {
        if (data is null)
        {
            return;
        }
        writer.WritePropertyName(name);
        WriteKey(writer, data);
    }

    // ---------------------------------------------------------------- helpers

    private static AbstractKeyData ReadRequired(JsonElement el, string name) =>
        el.TryGetProperty(name, out var child) ? ReadKey(child) : throw new FormatException($"Missing '{name}' in selector.");

    private static AbstractKeyData? ReadOptional(JsonElement el, string name) =>
        el.TryGetProperty(name, out var child) && child.ValueKind != JsonValueKind.Null ? ReadKey(child) : null;

    internal static string? GetString(JsonElement el, string name) =>
        el.ValueKind == JsonValueKind.Object && el.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() : null;

    internal static int? GetInt(JsonElement el, string name) =>
        el.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.Number ? v.GetInt32() : null;

    internal static IReadOnlyList<string> GetStringArray(JsonElement el, string name) =>
        el.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.Array
            ? v.EnumerateArray().Select(x => x.GetString() ?? string.Empty).ToList()
            : [];

    internal static LayoutDirection ParseDirection(string? value) =>
        string.Equals(value, "rtl", StringComparison.OrdinalIgnoreCase) ? LayoutDirection.Rtl : LayoutDirection.Ltr;
}

/// <summary>Metadata of a layout declared in an extension manifest.</summary>
public sealed class LayoutMetadata
{
    public required LayoutType Type { get; init; }
    public required string Id { get; init; }
    public string Label { get; init; } = string.Empty;
    public IReadOnlyList<string> Authors { get; init; } = [];
    public LayoutDirection Direction { get; init; }
    public ComponentName? Modifier { get; init; }
}

/// <summary>A parsed <c>extension.json</c> manifest.</summary>
public sealed class KeyboardExtensionManifest
{
    public required string Id { get; init; }
    public string Version { get; init; } = "0.0.0";
    public string Title { get; init; } = string.Empty;
    public string Description { get; init; } = string.Empty;
    public string License { get; init; } = string.Empty;
    public IReadOnlyList<string> Dependencies { get; init; } = [];
    public IReadOnlyList<LayoutMetadata> Layouts { get; init; } = [];
    public IReadOnlyList<(string Id, IReadOnlyList<string> Authors)> PopupMappings { get; init; } = [];
    public IReadOnlyList<Subtype> SubtypePresets { get; init; } = [];
    public IReadOnlyList<CurrencySet> CurrencySets { get; init; } = [];
    public IReadOnlyList<ComposerDefinition> Composers { get; init; } = [];
    public IReadOnlyList<PunctuationRule> PunctuationRules { get; init; } = [];
}
