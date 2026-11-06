using System.Globalization;
using System.Text;
using System.Text.Json;

namespace ScreenKeyboard.Text;

/// <summary>Kind of a suggestion candidate.</summary>
public enum SuggestionKind
{
    /// <summary>The literally typed word.</summary>
    Typed,
    /// <summary>A completion of the typed prefix.</summary>
    Completion,
    /// <summary>A spelling correction.</summary>
    Correction,
    /// <summary>A next word prediction.</summary>
    Prediction,
    /// <summary>An emoji.</summary>
    Emoji,
    /// <summary>A glide typing candidate.</summary>
    Glide,
    /// <summary>A recently copied clipboard item.</summary>
    Clipboard,
}

/// <summary>A suggestion candidate displayed in the smartbar.</summary>
public sealed record Suggestion(string Text, SuggestionKind Kind, double Confidence = 0, bool IsAutoCommit = false)
{
    /// <summary>Optional secondary text (e.g. emoji name).</summary>
    public string? Description { get; init; }

    /// <summary>Whether the user can remove this suggestion (learned words).</summary>
    public bool IsRemovable { get; init; }
}

/// <summary>Input for a suggestion query.</summary>
public sealed record SuggestionRequest
{
    /// <summary>The word being typed (may be empty for next-word prediction).</summary>
    public string Word { get; init; } = string.Empty;

    /// <summary>Text before the current word (for context).</summary>
    public string PrecedingText { get; init; } = string.Empty;

    /// <summary>Maximum number of candidates.</summary>
    public int MaxCount { get; init; } = 3;

    /// <summary>Whether a correction may be auto-committed.</summary>
    public bool AllowAutoCorrect { get; init; } = true;

    /// <summary>Culture for case operations.</summary>
    public CultureInfo Culture { get; init; } = CultureInfo.InvariantCulture;

    /// <summary>Optional key-proximity substitution cost function (0..1).</summary>
    public Func<char, char, double>? KeyProximity { get; init; }
}

/// <summary>Produces suggestions and learns from user input.</summary>
public interface ISuggestionProvider
{
    /// <summary>Returns suggestions for the request.</summary>
    IReadOnlyList<Suggestion> Suggest(SuggestionRequest request);

    /// <summary>Learns a committed word with its preceding word (for predictions).</summary>
    void Learn(string word, string? previousWord);

    /// <summary>Removes a learned word and prevents it from being suggested.</summary>
    void Forget(string word);
}

/// <summary>
/// Learned words and word pairs (bigrams) of the user. Persist it with <see cref="Export"/> / <see cref="Import"/>.
/// </summary>
public sealed class UserDictionary
{
    private readonly Dictionary<string, int> _words = new(StringComparer.Ordinal);
    private readonly Dictionary<string, Dictionary<string, int>> _bigrams = new(StringComparer.Ordinal);
    private readonly HashSet<string> _blocked = new(StringComparer.OrdinalIgnoreCase);
    private readonly object _gate = new();

    /// <summary>Raised when the dictionary changed (useful for persistence).</summary>
    public event EventHandler? Changed;

    /// <summary>Number of times an unknown word must be typed before it is suggested.</summary>
    public int LearnThreshold { get; set; } = 2;

    /// <summary>Maximum number of learned words kept.</summary>
    public int MaxWords { get; set; } = 5000;

    /// <summary>All learned words and their usage counts.</summary>
    public IReadOnlyDictionary<string, int> Words
    {
        get { lock (_gate) return new Dictionary<string, int>(_words); }
    }

    /// <summary>Records a word usage.</summary>
    public void Learn(string word, string? previousWord)
    {
        if (word.Length == 0 || IsBlocked(word))
        {
            return;
        }
        lock (_gate)
        {
            _words[word] = _words.TryGetValue(word, out var count) ? count + 1 : 1;
            if (_words.Count > MaxWords)
            {
                var victim = _words.OrderBy(p => p.Value).First().Key;
                _words.Remove(victim);
                _bigrams.Remove(victim);
            }
            if (!string.IsNullOrEmpty(previousWord))
            {
                var prev = previousWord!.ToLowerInvariant();
                if (!_bigrams.TryGetValue(prev, out var next))
                {
                    next = new Dictionary<string, int>(StringComparer.Ordinal);
                    _bigrams[prev] = next;
                }
                next[word] = next.TryGetValue(word, out var c) ? c + 1 : 1;
            }
        }
        Changed?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>Usage count of a word.</summary>
    public int GetCount(string word)
    {
        lock (_gate) return _words.TryGetValue(word, out var c) ? c : 0;
    }

    /// <summary>Returns <c>true</c> if a word is blocked.</summary>
    public bool IsBlocked(string word)
    {
        lock (_gate) return _blocked.Contains(word);
    }

    /// <summary>Removes a word and blocks it from being suggested.</summary>
    public void Block(string word)
    {
        lock (_gate)
        {
            _words.Remove(word);
            _blocked.Add(word);
            foreach (var map in _bigrams.Values)
            {
                map.Remove(word);
            }
        }
        Changed?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>Unblocks a word.</summary>
    public void Unblock(string word)
    {
        lock (_gate) _blocked.Remove(word);
        Changed?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>Learned words starting with the prefix that reached the learn threshold.</summary>
    public IEnumerable<(string Word, int Count)> Complete(string prefix)
    {
        var norm = WordDictionary.Normalize(prefix);
        lock (_gate)
        {
            return _words.Where(p => p.Value >= LearnThreshold && WordDictionary.Normalize(p.Key).StartsWith(norm, StringComparison.Ordinal))
                .Select(p => (p.Key, p.Value)).ToList();
        }
    }

    /// <summary>Most likely next words after <paramref name="previousWord"/>.</summary>
    public IReadOnlyList<(string Word, int Count)> Predict(string previousWord, int max)
    {
        lock (_gate)
        {
            if (!_bigrams.TryGetValue(previousWord.ToLowerInvariant(), out var next))
            {
                return [];
            }
            return next.Where(p => !_blocked.Contains(p.Key)).OrderByDescending(p => p.Value).Take(max).Select(p => (p.Key, p.Value)).ToList();
        }
    }

    /// <summary>Clears all learned data (not the block list).</summary>
    public void Clear()
    {
        lock (_gate)
        {
            _words.Clear();
            _bigrams.Clear();
        }
        Changed?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>Serializes the dictionary to JSON.</summary>
    public string Export()
    {
        using var stream = new MemoryStream();
        using (var w = new Utf8JsonWriter(stream))
        {
            lock (_gate)
            {
                w.WriteStartObject();
                w.WriteStartObject("words");
                foreach (var p in _words) w.WriteNumber(p.Key, p.Value);
                w.WriteEndObject();
                w.WriteStartObject("bigrams");
                foreach (var p in _bigrams)
                {
                    w.WriteStartObject(p.Key);
                    foreach (var n in p.Value) w.WriteNumber(n.Key, n.Value);
                    w.WriteEndObject();
                }
                w.WriteEndObject();
                w.WriteStartArray("blocked");
                foreach (var b in _blocked) w.WriteStringValue(b);
                w.WriteEndArray();
                w.WriteEndObject();
            }
        }
        return Encoding.UTF8.GetString(stream.ToArray());
    }

    /// <summary>Loads data previously produced by <see cref="Export"/> (merging with existing data).</summary>
    public void Import(string json)
    {
        using var doc = JsonDocument.Parse(json);
        lock (_gate)
        {
            if (doc.RootElement.TryGetProperty("words", out var words))
            {
                foreach (var p in words.EnumerateObject()) _words[p.Name] = p.Value.GetInt32();
            }
            if (doc.RootElement.TryGetProperty("bigrams", out var bigrams))
            {
                foreach (var p in bigrams.EnumerateObject())
                {
                    var map = new Dictionary<string, int>(StringComparer.Ordinal);
                    foreach (var n in p.Value.EnumerateObject()) map[n.Name] = n.Value.GetInt32();
                    _bigrams[p.Name] = map;
                }
            }
            if (doc.RootElement.TryGetProperty("blocked", out var blocked))
            {
                foreach (var b in blocked.EnumerateArray()) _blocked.Add(b.GetString() ?? string.Empty);
            }
        }
        Changed?.Invoke(this, EventArgs.Empty);
    }
}

/// <summary>
/// Default suggestion provider: prefix completion, keyboard-aware spelling correction, learned words and
/// next-word prediction on top of a <see cref="WordDictionary"/> and a <see cref="UserDictionary"/>.
/// </summary>
public sealed class DictionarySuggestionProvider : ISuggestionProvider
{
    /// <summary>Creates a provider.</summary>
    public DictionarySuggestionProvider(WordDictionary dictionary, UserDictionary? userDictionary = null)
    {
        Dictionary = dictionary;
        UserDictionary = userDictionary ?? new UserDictionary();
    }

    /// <summary>The main dictionary.</summary>
    public WordDictionary Dictionary { get; }

    /// <summary>The user dictionary.</summary>
    public UserDictionary UserDictionary { get; }

    /// <summary>Maximum edit distance for corrections of long words.</summary>
    public double MaxCorrectionDistance { get; set; } = 2;

    /// <summary>
    /// Minimum confidence (0..1) needed for a correction to be auto-committed on space.
    /// </summary>
    public double AutoCorrectThreshold { get; set; } = 0.55;

    /// <summary>
    /// Frequency difference (0-255 scale) above which a known but rare word is auto-corrected to a much more
    /// frequent word at edit distance 1.
    /// </summary>
    public int RareWordFrequencyGap { get; set; } = 70;

    /// <summary>Common sentence starters used when there is no context.</summary>
    public IReadOnlyList<string> DefaultPredictions { get; set; } = ["I", "The", "Thanks"];

    /// <inheritdoc />
    public IReadOnlyList<Suggestion> Suggest(SuggestionRequest request)
    {
        var word = request.Word;
        var culture = request.Culture;
        var max = Math.Max(1, request.MaxCount);
        if (word.Length == 0)
        {
            return Predict(request, max);
        }

        var candidates = new Dictionary<string, Suggestion>(StringComparer.Ordinal);
        var typedFrequency = Dictionary.GetFrequency(word);
        var typedLearned = UserDictionary.GetCount(word) >= UserDictionary.LearnThreshold;
        var typedKnown = typedFrequency > 0 || typedLearned;
        var maxFreq = Math.Max(1.0, Dictionary.MaxFrequency);

        void Add(string text, SuggestionKind kind, double confidence, bool removable = false)
        {
            if (UserDictionary.IsBlocked(text))
            {
                return;
            }
            var cased = TextUtils.MatchCase(text, word, culture);
            if (string.Equals(cased, word, StringComparison.Ordinal) && kind != SuggestionKind.Typed)
            {
                return;
            }
            if (!candidates.TryGetValue(cased, out var existing) || existing.Confidence < confidence)
            {
                candidates[cased] = new Suggestion(cased, kind, confidence) { IsRemovable = removable };
            }
        }

        // Completions from the dictionary.
        foreach (var (w, f) in Dictionary.Complete(word, max + 3))
        {
            if (w.Length <= word.Length && WordDictionary.Normalize(w) == WordDictionary.Normalize(word))
            {
                // A diacritic variant of the typed word ("cafe" → "café") is a strong correction.
                if (!string.Equals(w, word, StringComparison.OrdinalIgnoreCase))
                {
                    Add(w, SuggestionKind.Correction, 0.8 + 0.2 * f / maxFreq);
                }
                continue;
            }
            // Longer completions are less likely.
            var lengthPenalty = 1.0 / (1 + 0.15 * (w.Length - word.Length));
            Add(w, SuggestionKind.Completion, 0.5 * (f / maxFreq) * lengthPenalty + 0.1);
        }

        // Learned words.
        foreach (var (w, count) in UserDictionary.Complete(word))
        {
            Add(w, SuggestionKind.Completion, Math.Min(0.9, 0.4 + 0.05 * count), removable: true);
        }

        // Corrections.
        var distance = word.Length <= 2 ? 0 : word.Length <= 4 ? 1 : MaxCorrectionDistance;
        FuzzyMatch? bestCorrection = null;
        var bestConfidence = 0.0;
        if (distance > 0)
        {
            foreach (var m in Dictionary.FindSimilar(word, distance, 12, request.KeyProximity))
            {
                if (string.Equals(m.Word, word, StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }
                var freqScore = m.Frequency / maxFreq;
                var confidence = Math.Clamp(0.85 * freqScore * Math.Exp(-1.2 * m.Distance) + (m.Distance <= 1 ? 0.2 : 0), 0, 1);
                if (typedKnown && m.Frequency - typedFrequency < RareWordFrequencyGap)
                {
                    // The typed word is a real word: corrections are unlikely to be intended.
                    confidence *= 0.3;
                }
                Add(m.Word, SuggestionKind.Correction, confidence);
                if (confidence > bestConfidence)
                {
                    bestCorrection = m;
                    bestConfidence = confidence;
                }
            }
        }

        var ordered = candidates.Values
            .OrderByDescending(s => s.Confidence)
            .Take(max)
            .ToList();

        // Decide about auto-correction. Unknown words are corrected when a confident candidate exists; known but
        // rare spellings (dictionaries built from web corpora contain typos such as "teh") are corrected when a
        // much more frequent word is a single edit away.
        var correctable = !typedLearned && (typedFrequency == 0 ||
            bestCorrection is { } bc && bc.Distance <= 1 && bc.Frequency - typedFrequency >= RareWordFrequencyGap);
        if (request.AllowAutoCorrect && correctable && bestCorrection is { } best && bestConfidence >= AutoCorrectThreshold * 0.5)
        {
            var cased = TextUtils.MatchCase(best.Word, word, culture);
            var index = ordered.FindIndex(s => s.Text == cased);
            if (index < 0 && candidates.TryGetValue(cased, out var missing))
            {
                ordered.Insert(0, missing);
                index = 0;
            }
            if (index >= 0)
            {
                var s = ordered[index];
                ordered.RemoveAt(index);
                ordered.Insert(0, s with { IsAutoCommit = true });
                typedKnown = false;
            }
        }

        // A correctly spelled word is offered first so it can be confirmed with a tap.
        if (typedKnown && !ordered.Any(o => o.IsAutoCommit) && !ordered.Any(o => o.Text == word))
        {
            ordered.Insert(0, new Suggestion(word, SuggestionKind.Typed, 1));
            if (ordered.Count > max)
            {
                ordered.RemoveAt(ordered.Count - 1);
            }
        }

        // Always offer the typed word when it's unknown so the user can keep it.
        if (ordered.Count > 0 && ordered[0].IsAutoCommit && !ordered.Any(o => o.Text == word))
        {
            ordered.Insert(0, new Suggestion(word, SuggestionKind.Typed, 1));
            if (ordered.Count > max)
            {
                ordered.RemoveAt(ordered.Count - 1);
            }
        }
        return ordered;
    }

    private IReadOnlyList<Suggestion> Predict(SuggestionRequest request, int max)
    {
        var previous = LastWord(request.PrecedingText);
        var result = new List<Suggestion>();
        if (previous is not null)
        {
            foreach (var (w, count) in UserDictionary.Predict(previous, max))
            {
                result.Add(new Suggestion(w, SuggestionKind.Prediction, Math.Min(1, 0.3 + 0.1 * count)) { IsRemovable = true });
            }
        }
        else if (request.PrecedingText.Trim().Length == 0)
        {
            foreach (var w in DefaultPredictions.Take(max))
            {
                result.Add(new Suggestion(w, SuggestionKind.Prediction, 0.1));
            }
        }
        return result;
    }

    private static string? LastWord(string text)
    {
        var trimmed = text.TrimEnd();
        if (trimmed.Length == 0)
        {
            return null;
        }
        var word = TextUtils.GetCurrentWord(trimmed);
        return word.Length == 0 ? null : word;
    }

    /// <inheritdoc />
    public void Learn(string word, string? previousWord)
    {
        if (word.Length < 2 || !word.Any(char.IsLetter))
        {
            return;
        }
        UserDictionary.Learn(word, previousWord);
    }

    /// <inheritdoc />
    public void Forget(string word) => UserDictionary.Block(word);
}
