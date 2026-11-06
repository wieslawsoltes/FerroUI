using System.Globalization;
using System.Text.Json;
using ScreenKeyboard.Resources;

namespace ScreenKeyboard.Text;

/// <summary>
/// An immutable-ish word list with frequencies (0-255) backed by a compact trie. Lookups are case and
/// diacritic insensitive; the original spelling of each word is preserved. Supports prefix completion
/// and bounded fuzzy search (Damerau-Levenshtein with optional keyboard proximity costs).
/// </summary>
public sealed class WordDictionary
{
    private readonly Node _root = new('\0');
    private readonly Dictionary<string, WordEntry> _words = new(StringComparer.Ordinal);
    private readonly object _gate = new();

    /// <summary>Creates an empty dictionary.</summary>
    public WordDictionary(string languageTag = "en")
    {
        LanguageTag = languageTag;
    }

    /// <summary>Language of the dictionary.</summary>
    public string LanguageTag { get; }

    /// <summary>Number of words.</summary>
    public int Count
    {
        get { lock (_gate) return _words.Count; }
    }

    /// <summary>Highest frequency value found in the dictionary.</summary>
    public int MaxFrequency { get; private set; } = 1;

    /// <summary>All words (original spelling).</summary>
    public IReadOnlyList<string> Words
    {
        get { lock (_gate) return _words.Values.Select(w => w.Word).ToList(); }
    }

    /// <summary>
    /// Loads a dictionary in FlorisBoard's JSON format (<c>{"word": frequency, ...}</c>).
    /// </summary>
    public static WordDictionary FromJson(string json, string languageTag = "en")
    {
        var dict = new WordDictionary(languageTag);
        using var doc = JsonDocument.Parse(json);
        foreach (var prop in doc.RootElement.EnumerateObject())
        {
            if (prop.Value.ValueKind == JsonValueKind.Number)
            {
                dict.Add(prop.Name, prop.Value.GetInt32());
            }
        }
        return dict;
    }

    /// <summary>
    /// Loads a plain text word list: one word per line, optionally followed by a whitespace/tab/comma separated
    /// frequency. Lines starting with <c>#</c> are ignored. Words without a frequency get a descending rank-based frequency.
    /// </summary>
    public static WordDictionary FromText(string text, string languageTag = "en")
    {
        var dict = new WordDictionary(languageTag);
        var lines = text.Split('\n');
        var rank = 0;
        foreach (var raw in lines)
        {
            var line = raw.Trim();
            if (line.Length == 0 || line[0] == '#')
            {
                continue;
            }
            var parts = line.Split(new[] { ' ', '\t', ',' }, StringSplitOptions.RemoveEmptyEntries);
            var freq = parts.Length > 1 && int.TryParse(parts[1], NumberStyles.Integer, CultureInfo.InvariantCulture, out var f)
                ? f
                : Math.Max(1, 255 - rank / 200);
            dict.Add(parts[0], freq);
            rank++;
        }
        return dict;
    }

    /// <summary>Creates a dictionary from word/frequency pairs.</summary>
    public static WordDictionary FromWords(IEnumerable<(string Word, int Frequency)> words, string languageTag = "en")
    {
        var dict = new WordDictionary(languageTag);
        foreach (var (w, f) in words)
        {
            dict.Add(w, f);
        }
        return dict;
    }

    /// <summary>Loads the bundled English dictionary (~50k words).</summary>
    public static WordDictionary LoadBundledEnglish()
    {
        var json = EmbeddedResources.ReadText("dict/en.json") ?? "{}";
        return FromJson(json, "en");
    }

    /// <summary>Adds or updates a word. The highest frequency wins.</summary>
    public void Add(string word, int frequency)
    {
        word = word.Trim();
        if (word.Length == 0)
        {
            return;
        }
        var key = Normalize(word);
        lock (_gate)
        {
            var node = _root;
            node.Touch(frequency);
            foreach (var c in key)
            {
                node = node.GetOrAdd(c);
                node.Touch(frequency);
            }
            if (_words.TryGetValue(word, out var existing))
            {
                existing.Frequency = Math.Max(existing.Frequency, frequency);
            }
            else
            {
                var entry = new WordEntry(word, frequency);
                _words[word] = entry;
                (node.Entries ??= new List<WordEntry>(1)).Add(entry);
            }
            MaxFrequency = Math.Max(MaxFrequency, frequency);
        }
    }

    /// <summary>Returns <c>true</c> if the word (any casing) exists.</summary>
    public bool Contains(string word) => GetFrequency(word) > 0;

    /// <summary>Returns the frequency of a word (case/diacritic insensitive match with preference for exact spelling), or 0.</summary>
    public int GetFrequency(string word)
    {
        lock (_gate)
        {
            if (_words.TryGetValue(word, out var exact))
            {
                return Math.Max(1, exact.Frequency);
            }
            var node = Find(Normalize(word));
            if (node?.Entries is { Count: > 0 } entries)
            {
                // Case-insensitive match only (not diacritic-insensitive).
                var match = entries.FirstOrDefault(e => string.Equals(e.Word, word, StringComparison.OrdinalIgnoreCase));
                return match is null ? 0 : Math.Max(1, match.Frequency);
            }
            return 0;
        }
    }

    /// <summary>Returns entries matching the normalized form of <paramref name="word"/> (e.g. "cafe" → "café").</summary>
    public IReadOnlyList<(string Word, int Frequency)> GetVariants(string word)
    {
        lock (_gate)
        {
            var node = Find(Normalize(word));
            return node?.Entries?.Select(e => (e.Word, e.Frequency)).ToList() ?? (IReadOnlyList<(string, int)>)Array.Empty<(string, int)>();
        }
    }

    /// <summary>Returns up to <paramref name="max"/> most frequent words starting with <paramref name="prefix"/>.</summary>
    public IReadOnlyList<(string Word, int Frequency)> Complete(string prefix, int max)
    {
        var key = Normalize(prefix);
        var best = new List<WordEntry>();
        lock (_gate)
        {
            var node = Find(key);
            if (node is null)
            {
                return [];
            }
            Collect(node, best, max, 0);
        }
        return best.Select(e => (e.Word, e.Frequency)).ToList();
    }

    private static void Collect(Node node, List<WordEntry> best, int max, int depth)
    {
        if (node.Entries is not null)
        {
            foreach (var e in node.Entries)
            {
                InsertTop(best, e, max);
            }
        }
        if (node.Children is null || depth > 40)
        {
            return;
        }
        // Prune: skip subtrees whose max frequency cannot beat the current worst.
        foreach (var child in node.Children)
        {
            if (best.Count >= max && child.MaxFrequency <= best[best.Count - 1].Frequency)
            {
                continue;
            }
            Collect(child, best, max, depth + 1);
        }
    }

    private static void InsertTop(List<WordEntry> best, WordEntry entry, int max)
    {
        var i = best.Count;
        while (i > 0 && best[i - 1].Frequency < entry.Frequency)
        {
            i--;
        }
        if (i >= max)
        {
            return;
        }
        best.Insert(i, entry);
        if (best.Count > max)
        {
            best.RemoveAt(best.Count - 1);
        }
    }

    /// <summary>
    /// Finds words within <paramref name="maxDistance"/> edits of <paramref name="word"/>. Substitution costs can be
    /// reduced for keys that are physically close via <paramref name="substitutionCost"/> (returns 0..1).
    /// </summary>
    public IReadOnlyList<FuzzyMatch> FindSimilar(string word, double maxDistance, int maxResults, Func<char, char, double>? substitutionCost = null)
    {
        var key = Normalize(word);
        if (key.Length == 0)
        {
            return [];
        }
        var results = new List<FuzzyMatch>();
        var previousRow = new double[key.Length + 1];
        for (var i = 0; i <= key.Length; i++)
        {
            previousRow[i] = i;
        }
        lock (_gate)
        {
            if (_root.Children is null)
            {
                return [];
            }
            foreach (var child in _root.Children)
            {
                Search(child, '\0', key, previousRow, null, maxDistance, substitutionCost, results, 1);
            }
        }
        return results
            .OrderBy(r => r.Distance)
            .ThenByDescending(r => r.Frequency)
            .Take(maxResults)
            .ToList();
    }

    private static void Search(Node node, char previousChar, string key, double[] previousRow, double[]? prePreviousRow, double maxDistance,
        Func<char, char, double>? subCost, List<FuzzyMatch> results, int depth)
    {
        var columns = key.Length + 1;
        var currentRow = new double[columns];
        currentRow[0] = previousRow[0] + 1;
        var rowMin = currentRow[0];
        var c = node.Char;
        for (var i = 1; i < columns; i++)
        {
            var insertCost = currentRow[i - 1] + 1;
            var deleteCost = previousRow[i] + 1;
            var replace = key[i - 1] == c ? 0 : subCost?.Invoke(key[i - 1], c) ?? 1;
            var replaceCost = previousRow[i - 1] + replace;
            var value = Math.Min(Math.Min(insertCost, deleteCost), replaceCost);
            // Transposition
            if (prePreviousRow is not null && i > 1 && key[i - 1] == previousChar && key[i - 2] == c)
            {
                value = Math.Min(value, prePreviousRow[i - 2] + TranspositionCost);
            }
            currentRow[i] = value;
            rowMin = Math.Min(rowMin, value);
        }

        if (currentRow[columns - 1] <= maxDistance && node.Entries is not null)
        {
            foreach (var e in node.Entries)
            {
                results.Add(new FuzzyMatch(e.Word, e.Frequency, currentRow[columns - 1]));
            }
        }

        if (rowMin <= maxDistance && node.Children is not null && depth < key.Length + (int)Math.Ceiling(maxDistance) + 1)
        {
            foreach (var child in node.Children)
            {
                Search(child, c, key, currentRow, previousRow, maxDistance, subCost, results, depth + 1);
            }
        }
    }

    private Node? Find(string key)
    {
        var node = _root;
        foreach (var c in key)
        {
            node = node.Get(c);
            if (node is null)
            {
                return null;
            }
        }
        return node;
    }

    /// <summary>Cost of swapping two adjacent characters (a very common typing error).</summary>
    public const double TranspositionCost = 0.5;

    /// <summary>Normalizes a word for lookups: lower case, without diacritics.</summary>
    public static string Normalize(string word) => TextUtils.RemoveDiacritics(word.ToLowerInvariant());

    private sealed class WordEntry(string word, int frequency)
    {
        public string Word { get; } = word;
        public int Frequency { get; set; } = frequency;
    }

    private sealed class Node(char c)
    {
        public char Char { get; } = c;
        public List<Node>? Children { get; private set; }
        public List<WordEntry>? Entries { get; set; }
        public int MaxFrequency { get; private set; }

        public Node? Get(char c)
        {
            if (Children is null)
            {
                return null;
            }
            foreach (var child in Children)
            {
                if (child.Char == c)
                {
                    return child;
                }
            }
            return null;
        }

        public Node GetOrAdd(char c)
        {
            var existing = Get(c);
            if (existing is not null)
            {
                return existing;
            }
            var node = new Node(c);
            (Children ??= new List<Node>(2)).Add(node);
            return node;
        }

        public void Touch(int frequency) => MaxFrequency = Math.Max(MaxFrequency, frequency);
    }

    internal void RebuildMaxFrequencies()
    {
        lock (_gate)
        {
            Rebuild(_root);
        }

        static int Rebuild(Node node)
        {
            var max = node.Entries?.Max(e => e.Frequency) ?? 0;
            if (node.Children is not null)
            {
                foreach (var child in node.Children)
                {
                    max = Math.Max(max, Rebuild(child));
                }
            }
            node.Touch(max);
            return max;
        }
    }
}

/// <summary>A fuzzy dictionary match.</summary>
public readonly record struct FuzzyMatch(string Word, int Frequency, double Distance);
