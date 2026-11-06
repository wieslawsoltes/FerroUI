using System.Text;
using System.Text.Json;
using ScreenKeyboard.Resources;

namespace ScreenKeyboard.Emoji;

/// <summary>Emoji categories (Unicode CLDR groups).</summary>
public enum EmojiCategory
{
    Recent,
    SmileysEmotion,
    PeopleBody,
    AnimalsNature,
    FoodDrink,
    TravelPlaces,
    Activities,
    Objects,
    Symbols,
    Flags,
}

/// <summary>Fitzpatrick skin tone modifiers.</summary>
public enum EmojiSkinTone
{
    Default,
    Light,
    MediumLight,
    Medium,
    MediumDark,
    Dark,
}

/// <summary>An emoji with its metadata.</summary>
public sealed class EmojiInfo
{
    /// <summary>The emoji string (may contain several code points).</summary>
    public required string Value { get; init; }

    /// <summary>CLDR short name (e.g. "grinning face"), empty if unknown.</summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>Search keywords.</summary>
    public IReadOnlyList<string> Keywords { get; init; } = [];

    /// <summary>Category.</summary>
    public EmojiCategory Category { get; init; }

    /// <summary>Skin tone variants (empty when the emoji does not support skin tones).</summary>
    public IReadOnlyList<string> Variants { get; internal set; } = [];

    /// <summary>Whether skin tone variants exist.</summary>
    public bool HasSkinTones => Variants.Count > 0;

    /// <summary>Returns the variant for a skin tone (falls back to the default emoji).</summary>
    public string WithSkinTone(EmojiSkinTone tone)
    {
        if (tone == EmojiSkinTone.Default || Variants.Count == 0)
        {
            return Value;
        }
        var modifier = char.ConvertFromUtf32(0x1F3FB + (int)tone - 1);
        // Prefer a variant where every modifier is the requested tone.
        foreach (var v in Variants)
        {
            if (v.Contains(modifier) && EmojiCatalog.CountModifiers(v) == EmojiCatalog.CountModifier(v, modifier))
            {
                return v;
            }
        }
        return Variants.FirstOrDefault(v => v.Contains(modifier)) ?? Value;
    }

    /// <inheritdoc />
    public override string ToString() => $"{Value} {Name}";
}

/// <summary>A classic text emoticon (e.g. <c>:-)</c>).</summary>
public sealed record Emoticon(string Text, IReadOnlyList<string> Meanings);

/// <summary>
/// The emoji catalog, loaded from the bundled CLDR v48 data (FlorisBoard format). Supports categories,
/// skin tone variants and keyword search in English, German, Spanish, French, Italian and Portuguese.
/// </summary>
public sealed class EmojiCatalog
{
    private static readonly Dictionary<string, EmojiCategory> s_categoryIds = new(StringComparer.Ordinal)
    {
        ["smileys_emotion"] = EmojiCategory.SmileysEmotion,
        ["people_body"] = EmojiCategory.PeopleBody,
        ["animals_nature"] = EmojiCategory.AnimalsNature,
        ["food_drink"] = EmojiCategory.FoodDrink,
        ["travel_places"] = EmojiCategory.TravelPlaces,
        ["activities"] = EmojiCategory.Activities,
        ["objects"] = EmojiCategory.Objects,
        ["symbols"] = EmojiCategory.Symbols,
        ["flags"] = EmojiCategory.Flags,
    };

    private static readonly Dictionary<string, EmojiCatalog> s_cache = new(StringComparer.OrdinalIgnoreCase);

    private readonly Dictionary<EmojiCategory, List<EmojiInfo>> _byCategory = new();
    private readonly Dictionary<string, EmojiInfo> _byValue = new(StringComparer.Ordinal);

    /// <summary>Locales with bundled emoji annotations.</summary>
    public static IReadOnlyList<string> BundledLocales { get; } = ["en", "de", "es", "fr", "it", "pt"];

    /// <summary>Locale of the annotations.</summary>
    public string Locale { get; }

    private EmojiCatalog(string locale)
    {
        Locale = locale;
    }

    /// <summary>
    /// Loads (and caches) the bundled catalog for a language (e.g. <c>"de"</c> or <c>"de-AT"</c>). Unknown languages
    /// fall back to English.
    /// </summary>
    public static EmojiCatalog Load(string language = "en")
    {
        var lang = language.Split('-', '_')[0].ToLowerInvariant();
        if (!BundledLocales.Contains(lang))
        {
            lang = "en";
        }
        lock (s_cache)
        {
            if (s_cache.TryGetValue(lang, out var cached))
            {
                return cached;
            }
            var text = EmbeddedResources.ReadText($"media/emoji/{lang}.txt") ?? EmbeddedResources.ReadText("media/emoji/root.txt") ?? string.Empty;
            var catalog = Parse(text, lang);
            s_cache[lang] = catalog;
            return catalog;
        }
    }

    /// <summary>
    /// Parses emoji data in FlorisBoard format: <c>[category]</c> headers, lines of
    /// <c>emoji;name;keyword|keyword</c>, and tab-indented skin tone variants.
    /// </summary>
    public static EmojiCatalog Parse(string text, string locale = "en")
    {
        var catalog = new EmojiCatalog(locale);
        var category = EmojiCategory.SmileysEmotion;
        EmojiInfo? lastBase = null;
        List<string>? variants = null;

        void FlushVariants()
        {
            if (lastBase is not null && variants is { Count: > 0 })
            {
                lastBase.Variants = variants;
            }
            variants = null;
        }

        foreach (var rawLine in text.Split('\n'))
        {
            var line = rawLine.TrimEnd('\r');
            if (line.Length == 0 || line[0] == '#')
            {
                continue;
            }
            if (line[0] == '[')
            {
                FlushVariants();
                lastBase = null;
                var id = line.Trim('[', ']', ' ');
                category = s_categoryIds.TryGetValue(id, out var c) ? c : category;
                continue;
            }
            var isVariant = line[0] == '\t' || line[0] == ' ';
            var parts = line.Trim().Split(';');
            var value = parts[0];
            if (value.Length == 0)
            {
                continue;
            }
            if (isVariant)
            {
                (variants ??= new List<string>()).Add(value);
                continue;
            }
            FlushVariants();
            var info = new EmojiInfo
            {
                Value = value,
                Name = parts.Length > 1 ? parts[1] : string.Empty,
                Keywords = parts.Length > 2 && parts[2].Length > 0 ? parts[2].Split('|') : [],
                Category = category,
            };
            if (!catalog._byCategory.TryGetValue(category, out var list))
            {
                list = new List<EmojiInfo>();
                catalog._byCategory[category] = list;
            }
            list.Add(info);
            catalog._byValue[value] = info;
            lastBase = info;
        }
        FlushVariants();
        foreach (var info in catalog._byValue.Values.ToList())
        {
            foreach (var v in info.Variants)
            {
                catalog._byValue.TryAdd(v, info);
            }
        }
        return catalog;
    }

    /// <summary>Categories with emojis, in display order (without <see cref="EmojiCategory.Recent"/>).</summary>
    public IReadOnlyList<EmojiCategory> Categories => _byCategory.Keys.OrderBy(c => (int)c).ToList();

    /// <summary>All emojis.</summary>
    public IEnumerable<EmojiInfo> All => Categories.SelectMany(c => _byCategory[c]);

    /// <summary>Emojis of a category.</summary>
    public IReadOnlyList<EmojiInfo> GetEmojis(EmojiCategory category) =>
        _byCategory.TryGetValue(category, out var list) ? list : [];

    /// <summary>Gets the info of an emoji or one of its skin tone variants.</summary>
    public EmojiInfo? Get(string emoji) => _byValue.TryGetValue(emoji, out var info) ? info : null;

    /// <summary>
    /// Searches emojis by name and keywords. Every query term must match the start of a name word or keyword.
    /// </summary>
    public IReadOnlyList<EmojiInfo> Search(string query, int maxResults = 50)
    {
        var terms = query.Trim().ToLowerInvariant().Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries);
        if (terms.Length == 0)
        {
            return [];
        }
        var scored = new List<(EmojiInfo Info, int Score)>();
        foreach (var info in All)
        {
            var total = 0;
            var ok = true;
            var nameWords = info.Name.ToLowerInvariant().Split(' ', '-', ':');
            foreach (var term in terms)
            {
                var score = 0;
                if (info.Name.Equals(term, StringComparison.OrdinalIgnoreCase)) score = 100;
                else if (nameWords.Any(w => w == term)) score = 60;
                else if (nameWords.Any(w => w.StartsWith(term, StringComparison.Ordinal))) score = 40;
                else if (info.Keywords.Any(k => k.Equals(term, StringComparison.OrdinalIgnoreCase))) score = 35;
                else if (info.Keywords.Any(k => k.StartsWith(term, StringComparison.OrdinalIgnoreCase))) score = 20;
                if (score == 0)
                {
                    ok = false;
                    break;
                }
                total += score;
            }
            if (ok)
            {
                // Prefer shorter names (more generic).
                scored.Add((info, total * 100 - info.Name.Length));
            }
        }
        return scored.OrderByDescending(s => s.Score).Take(maxResults).Select(s => s.Info).ToList();
    }

    /// <summary>Loads the bundled emoticons.</summary>
    public static IReadOnlyList<Emoticon> LoadEmoticons()
    {
        var json = EmbeddedResources.ReadText("media/emoticon/emoticons.json");
        if (json is null)
        {
            return [];
        }
        var result = new List<Emoticon>();
        using var doc = JsonDocument.Parse(json);
        foreach (var row in doc.RootElement.GetProperty("arrangement").EnumerateArray())
        {
            foreach (var item in row.EnumerateArray())
            {
                var meanings = item.TryGetProperty("meaning", out var m) ? m.EnumerateArray().Select(x => x.GetString() ?? string.Empty).ToList() : new List<string>();
                result.Add(new Emoticon(item.GetProperty("icon").GetString() ?? string.Empty, meanings));
            }
        }
        return result;
    }

    internal static int CountModifiers(string s)
    {
        var count = 0;
        foreach (var cp in TextUtils.GetCodePoints(s))
        {
            if (cp is >= 0x1F3FB and <= 0x1F3FF) count++;
        }
        return count;
    }

    internal static int CountModifier(string s, string modifier)
    {
        var target = char.ConvertToUtf32(modifier, 0);
        return TextUtils.GetCodePoints(s).Count(cp => cp == target);
    }

    /// <summary>Returns the emoji with any skin tone modifier removed.</summary>
    public static string StripSkinTone(string emoji)
    {
        var sb = new StringBuilder();
        foreach (var cp in TextUtils.GetCodePoints(emoji))
        {
            if (cp is < 0x1F3FB or > 0x1F3FF)
            {
                sb.Append(char.ConvertFromUtf32(cp));
            }
        }
        return sb.ToString();
    }
}

/// <summary>Recently used and pinned emojis. Persist with <see cref="Export"/>/<see cref="Import"/>.</summary>
public sealed class EmojiHistory
{
    private readonly List<string> _recent = new();
    private readonly List<string> _pinned = new();
    private readonly Dictionary<string, EmojiSkinTone> _preferredTones = new(StringComparer.Ordinal);

    /// <summary>Raised when the history changed.</summary>
    public event EventHandler? Changed;

    /// <summary>Maximum number of recent emojis.</summary>
    public int MaxRecent { get; set; } = 40;

    /// <summary>Recently used emojis, most recent first.</summary>
    public IReadOnlyList<string> Recent => _recent;

    /// <summary>Pinned emojis.</summary>
    public IReadOnlyList<string> Pinned => _pinned;

    /// <summary>Recent and pinned emojis for the "Recent" tab (pinned first).</summary>
    public IReadOnlyList<string> Combined => _pinned.Concat(_recent.Where(r => !_pinned.Contains(r))).ToList();

    /// <summary>Records an emoji usage (moves it to the front).</summary>
    public void Add(string emoji)
    {
        _recent.Remove(emoji);
        _recent.Insert(0, emoji);
        while (_recent.Count > MaxRecent)
        {
            _recent.RemoveAt(_recent.Count - 1);
        }
        var baseEmoji = EmojiCatalog.StripSkinTone(emoji);
        if (baseEmoji != emoji)
        {
            foreach (var cp in TextUtils.GetCodePoints(emoji))
            {
                if (cp is >= 0x1F3FB and <= 0x1F3FF)
                {
                    _preferredTones[baseEmoji] = (EmojiSkinTone)(cp - 0x1F3FB + 1);
                    break;
                }
            }
        }
        Changed?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>The last skin tone chosen for a base emoji.</summary>
    public EmojiSkinTone? GetPreferredTone(string baseEmoji) => _preferredTones.TryGetValue(baseEmoji, out var t) ? t : null;

    /// <summary>Pins an emoji.</summary>
    public void Pin(string emoji)
    {
        if (!_pinned.Contains(emoji))
        {
            _pinned.Add(emoji);
            Changed?.Invoke(this, EventArgs.Empty);
        }
    }

    /// <summary>Unpins an emoji.</summary>
    public void Unpin(string emoji)
    {
        if (_pinned.Remove(emoji))
        {
            Changed?.Invoke(this, EventArgs.Empty);
        }
    }

    /// <summary>Removes an emoji from the recent list.</summary>
    public void Remove(string emoji)
    {
        if (_recent.Remove(emoji))
        {
            Changed?.Invoke(this, EventArgs.Empty);
        }
    }

    /// <summary>Clears the recent list.</summary>
    public void ClearRecent()
    {
        _recent.Clear();
        Changed?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>Serializes the history to JSON.</summary>
    public string Export()
    {
        using var stream = new MemoryStream();
        using (var w = new Utf8JsonWriter(stream))
        {
            w.WriteStartObject();
            w.WriteStartArray("recent");
            foreach (var e in _recent) w.WriteStringValue(e);
            w.WriteEndArray();
            w.WriteStartArray("pinned");
            foreach (var e in _pinned) w.WriteStringValue(e);
            w.WriteEndArray();
            w.WriteStartObject("tones");
            foreach (var p in _preferredTones) w.WriteNumber(p.Key, (int)p.Value);
            w.WriteEndObject();
            w.WriteEndObject();
        }
        return Encoding.UTF8.GetString(stream.ToArray());
    }

    /// <summary>Restores history from JSON produced by <see cref="Export"/>.</summary>
    public void Import(string json)
    {
        using var doc = JsonDocument.Parse(json);
        _recent.Clear();
        _pinned.Clear();
        if (doc.RootElement.TryGetProperty("recent", out var recent))
        {
            _recent.AddRange(recent.EnumerateArray().Select(e => e.GetString() ?? string.Empty).Where(e => e.Length > 0));
        }
        if (doc.RootElement.TryGetProperty("pinned", out var pinned))
        {
            _pinned.AddRange(pinned.EnumerateArray().Select(e => e.GetString() ?? string.Empty).Where(e => e.Length > 0));
        }
        if (doc.RootElement.TryGetProperty("tones", out var tones))
        {
            foreach (var p in tones.EnumerateObject()) _preferredTones[p.Name] = (EmojiSkinTone)p.Value.GetInt32();
        }
        Changed?.Invoke(this, EventArgs.Empty);
    }
}
