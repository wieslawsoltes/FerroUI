using ScreenKeyboard.Resources;
using ScreenKeyboard.Text;

namespace ScreenKeyboard.Layouts;

/// <summary>
/// Registry of all keyboard resources: layouts, popup mappings, subtype presets, currency sets,
/// composers and punctuation rules. <see cref="Default"/> contains the bundled FlorisBoard data
/// (90+ layouts, 70+ language presets); custom resources can be added at any time.
/// </summary>
public sealed class KeyboardResources
{
    /// <summary>Id of the bundled layouts extension.</summary>
    public const string CoreLayoutsExtension = "org.florisboard.layouts";

    /// <summary>Default extension id used for custom resources.</summary>
    public const string CustomExtension = "custom";

    private static readonly string[] s_builtInExtensions =
    [
        "org.florisboard.layouts",
        "org.florisboard.localization",
        "org.florisboard.composers",
        "org.florisboard.currencysets",
    ];

    private static readonly Lazy<KeyboardResources> s_default = new(CreateDefault);

    private readonly object _gate = new();
    private readonly Dictionary<(LayoutType, ComponentName), LayoutEntry> _layouts = new();
    private readonly Dictionary<ComponentName, Func<PopupMapping>> _popupMappingLoaders = new();
    private readonly Dictionary<ComponentName, PopupMapping> _popupMappings = new();
    private readonly Dictionary<ComponentName, CurrencySet> _currencySets = new();
    private readonly Dictionary<ComponentName, IComposer> _composers = new();
    private readonly Dictionary<ComponentName, PunctuationRule> _punctuationRules = new();
    private readonly List<Subtype> _subtypePresets = new();
    private readonly List<KeyboardExtensionManifest> _extensions = new();

    /// <summary>Raised when resources are added or replaced.</summary>
    public event EventHandler? Changed;

    /// <summary>
    /// The shared default registry containing the bundled FlorisBoard resources. Custom layouts added
    /// to it are visible to every keyboard using the default registry.
    /// </summary>
    public static KeyboardResources Default => s_default.Value;

    /// <summary>Creates a new registry with the bundled resources loaded.</summary>
    public static KeyboardResources CreateDefault()
    {
        var resources = new KeyboardResources();
        foreach (var ext in s_builtInExtensions)
        {
            var root = $"keyboard/{ext}";
            var manifestJson = EmbeddedResources.ReadText($"{root}/extension.json");
            if (manifestJson is null)
            {
                continue;
            }
            var manifest = LayoutJson.ReadExtensionManifest(manifestJson);
            resources.AddExtension(manifest, relative => EmbeddedResources.ReadText($"{root}/{relative}"));
        }
        return resources;
    }

    /// <summary>Creates an empty registry (only the appender composer and default punctuation rule).</summary>
    public static KeyboardResources CreateEmpty() => new();

    private KeyboardResources()
    {
        _composers[new ComponentName(Subtype.CoreComposers, "appender")] = AppenderComposer.Instance;
        _punctuationRules[new ComponentName(Subtype.CoreLocalization, "default")] = PunctuationRule.Default;
    }

    /// <summary>All registered extensions.</summary>
    public IReadOnlyList<KeyboardExtensionManifest> Extensions
    {
        get { lock (_gate) return _extensions.ToList(); }
    }

    /// <summary>All subtype presets (language + layout combinations).</summary>
    public IReadOnlyList<Subtype> SubtypePresets
    {
        get { lock (_gate) return _subtypePresets.ToList(); }
    }

    /// <summary>All currency sets.</summary>
    public IReadOnlyList<CurrencySet> CurrencySets
    {
        get { lock (_gate) return _currencySets.Values.ToList(); }
    }

    /// <summary>All composers.</summary>
    public IReadOnlyList<IComposer> Composers
    {
        get { lock (_gate) return _composers.Values.Distinct().ToList(); }
    }

    /// <summary>All popup mapping names.</summary>
    public IReadOnlyList<ComponentName> PopupMappingNames
    {
        get { lock (_gate) return _popupMappingLoaders.Keys.ToList(); }
    }

    /// <summary>
    /// Registers an extension. <paramref name="fileProvider"/> resolves files relative to the extension root
    /// (<c>layouts/characters/qwerty.json</c>, <c>popupMappings/de.json</c>) and is invoked lazily.
    /// </summary>
    public void AddExtension(KeyboardExtensionManifest manifest, Func<string, string?> fileProvider)
    {
        lock (_gate)
        {
            _extensions.RemoveAll(e => e.Id == manifest.Id);
            _extensions.Add(manifest);
            foreach (var meta in manifest.Layouts)
            {
                var path = $"layouts/{meta.Type.ToId()}/{meta.Id}.json";
                var m = meta;
                _layouts[(meta.Type, new ComponentName(manifest.Id, meta.Id))] = new LayoutEntry(meta, manifest.Id, () =>
                {
                    var json = fileProvider(path);
                    return json is null ? LayoutArrangement.Empty : LayoutJson.ReadArrangement(json);
                });
            }
            foreach (var (id, authors) in manifest.PopupMappings)
            {
                var name = new ComponentName(manifest.Id, id);
                _popupMappings.Remove(name);
                _popupMappingLoaders[name] = () =>
                {
                    var json = fileProvider($"popupMappings/{id}.json") ?? "{}";
                    return LayoutJson.ReadPopupMapping(json, id, manifest.Id, authors);
                };
            }
            foreach (var cs in manifest.CurrencySets)
            {
                _currencySets[new ComponentName(manifest.Id, cs.Id)] = cs;
            }
            foreach (var composer in manifest.Composers)
            {
                _composers[new ComponentName(manifest.Id, composer.Id)] = CreateComposer(composer);
            }
            foreach (var rule in manifest.PunctuationRules)
            {
                _punctuationRules[new ComponentName(manifest.Id, rule.Id)] = rule;
            }
            foreach (var subtype in manifest.SubtypePresets)
            {
                _subtypePresets.RemoveAll(s => s.Id == subtype.Id);
                _subtypePresets.Add(subtype);
            }
        }
        OnChanged();
    }

    /// <summary>Registers (or replaces) a layout.</summary>
    public void AddLayout(KeyboardLayout layout)
    {
        lock (_gate)
        {
            var meta = new LayoutMetadata
            {
                Type = layout.Type,
                Id = layout.Id,
                Label = layout.Label,
                Authors = layout.Authors,
                Direction = layout.Direction,
                Modifier = layout.Modifier,
            };
            _layouts[(layout.Type, layout.Name)] = new LayoutEntry(meta, layout.ExtensionId, () => layout.Arrangement) { Cached = layout };
        }
        OnChanged();
    }

    /// <summary>Parses and registers a standalone layout document (see <see cref="LayoutJson.ReadLayoutDocument"/>).</summary>
    public KeyboardLayout AddLayoutJson(string json, string extensionId = CustomExtension)
    {
        var layout = LayoutJson.ReadLayoutDocument(json, extensionId);
        AddLayout(layout);
        return layout;
    }

    /// <summary>Registers (or replaces) a popup mapping.</summary>
    public void AddPopupMapping(PopupMapping mapping)
    {
        lock (_gate)
        {
            var name = new ComponentName(mapping.ExtensionId, mapping.Id);
            _popupMappings[name] = mapping;
            _popupMappingLoaders[name] = () => mapping;
        }
        OnChanged();
    }

    /// <summary>Registers a subtype preset.</summary>
    public void AddSubtypePreset(Subtype subtype)
    {
        lock (_gate)
        {
            _subtypePresets.RemoveAll(s => s.Id == subtype.Id);
            _subtypePresets.Add(subtype);
        }
        OnChanged();
    }

    /// <summary>Registers a currency set.</summary>
    public void AddCurrencySet(CurrencySet set)
    {
        lock (_gate) _currencySets[new ComponentName(set.ExtensionId, set.Id)] = set;
        OnChanged();
    }

    /// <summary>Registers a composer under <c>extensionId:composer.Id</c>.</summary>
    public void AddComposer(IComposer composer, string extensionId = CustomExtension)
    {
        lock (_gate) _composers[new ComponentName(extensionId, composer.Id)] = composer;
        OnChanged();
    }

    /// <summary>Registers a punctuation rule.</summary>
    public void AddPunctuationRule(PunctuationRule rule)
    {
        lock (_gate) _punctuationRules[new ComponentName(rule.ExtensionId, rule.Id)] = rule;
        OnChanged();
    }

    /// <summary>Returns metadata of all layouts of the given type.</summary>
    public IReadOnlyList<LayoutMetadata> GetLayoutMetadata(LayoutType type)
    {
        lock (_gate)
        {
            return _layouts.Where(p => p.Key.Item1 == type).Select(p => p.Value.Metadata).OrderBy(m => m.Label, StringComparer.CurrentCultureIgnoreCase).ToList();
        }
    }

    /// <summary>Returns the qualified names of all layouts of the given type.</summary>
    public IReadOnlyList<ComponentName> GetLayoutNames(LayoutType type)
    {
        lock (_gate)
        {
            return _layouts.Keys.Where(k => k.Item1 == type).Select(k => k.Item2).ToList();
        }
    }

    /// <summary>Gets a layout (loading its arrangement on first access), or <c>null</c>.</summary>
    public KeyboardLayout? GetLayout(LayoutType type, ComponentName name)
    {
        LayoutEntry? entry;
        lock (_gate)
        {
            if (!_layouts.TryGetValue((type, name), out entry))
            {
                return null;
            }
            if (entry.Cached is not null)
            {
                return entry.Cached;
            }
        }
        var arrangement = entry.Loader();
        var layout = new KeyboardLayout
        {
            Type = type,
            ExtensionId = entry.ExtensionId,
            Id = entry.Metadata.Id,
            Label = entry.Metadata.Label,
            Authors = entry.Metadata.Authors,
            Direction = entry.Metadata.Direction,
            Modifier = entry.Metadata.Modifier,
            Arrangement = arrangement,
        };
        lock (_gate)
        {
            entry.Cached ??= layout;
            return entry.Cached;
        }
    }

    /// <summary>Gets a popup mapping, or <c>null</c>.</summary>
    public PopupMapping? GetPopupMapping(ComponentName name)
    {
        Func<PopupMapping>? loader;
        lock (_gate)
        {
            if (_popupMappings.TryGetValue(name, out var cached))
            {
                return cached;
            }
            if (!_popupMappingLoaders.TryGetValue(name, out loader))
            {
                return null;
            }
        }
        var mapping = loader();
        lock (_gate)
        {
            _popupMappings[name] = mapping;
        }
        return mapping;
    }

    /// <summary>Gets a currency set, or <c>null</c>.</summary>
    public CurrencySet? GetCurrencySet(ComponentName name)
    {
        lock (_gate) return _currencySets.TryGetValue(name, out var set) ? set : null;
    }

    /// <summary>Gets a composer (falls back to the appender).</summary>
    public IComposer GetComposer(ComponentName name)
    {
        lock (_gate) return _composers.TryGetValue(name, out var composer) ? composer : AppenderComposer.Instance;
    }

    /// <summary>Gets a punctuation rule (falls back to the default rule).</summary>
    public PunctuationRule GetPunctuationRule(ComponentName name)
    {
        lock (_gate) return _punctuationRules.TryGetValue(name, out var rule) ? rule : PunctuationRule.Default;
    }

    /// <summary>
    /// Finds the best subtype preset for a language tag (exact match first, then language only).
    /// </summary>
    public Subtype? FindSubtypePreset(string languageTag)
    {
        lock (_gate)
        {
            var exact = _subtypePresets.FirstOrDefault(s => string.Equals(s.LanguageTag, languageTag, StringComparison.OrdinalIgnoreCase));
            if (exact is not null)
            {
                return exact;
            }
            var lang = languageTag.Split('-')[0];
            return _subtypePresets.FirstOrDefault(s => string.Equals(s.LanguageTag.Split('-')[0], lang, StringComparison.OrdinalIgnoreCase));
        }
    }

    /// <summary>Creates a composer instance from its definition.</summary>
    public static IComposer CreateComposer(ComposerDefinition definition) => definition.Kind switch
    {
        "hangul-unicode" => new HangulComposer(),
        "kana-unicode" => new KanaComposer(),
        "with-rules" => new RulesComposer(definition.Id, definition.Label, definition.Rules),
        _ => AppenderComposer.Instance,
    };

    private void OnChanged() => Changed?.Invoke(this, EventArgs.Empty);

    private sealed class LayoutEntry(LayoutMetadata metadata, string extensionId, Func<LayoutArrangement> loader)
    {
        public LayoutMetadata Metadata { get; } = metadata;
        public string ExtensionId { get; } = extensionId;
        public Func<LayoutArrangement> Loader { get; } = loader;
        public KeyboardLayout? Cached { get; set; }
    }
}
