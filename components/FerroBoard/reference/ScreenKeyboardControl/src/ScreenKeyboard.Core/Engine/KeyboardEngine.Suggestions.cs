using ScreenKeyboard.Emoji;
using ScreenKeyboard.Settings;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Text;

namespace ScreenKeyboard.Engine;

public sealed partial class KeyboardEngine
{
    private readonly Dictionary<string, ISuggestionProvider> _providers = new(StringComparer.OrdinalIgnoreCase);

    /// <summary>The suggestion provider for the active subtype, or <c>null</c> if no dictionary exists.</summary>
    public ISuggestionProvider? SuggestionProvider => GetProvider();

    private ISuggestionProvider? GetProvider()
    {
        if (SuggestionProviderFactory?.Invoke(_activeSubtype) is { } custom)
        {
            return custom;
        }
        var lang = _activeSubtype.LanguageTag.Split('-')[0];
        if (_providers.TryGetValue(lang, out var provider))
        {
            return provider;
        }
        var dictionary = LanguageModels.GetDictionary(lang);
        if (dictionary is null)
        {
            return null;
        }
        provider = new DictionarySuggestionProvider(dictionary, LanguageModels.GetUserDictionary(lang));
        _providers[lang] = provider;
        return provider;
    }

    /// <summary>Recomputes <see cref="Suggestions"/> for the current editor state.</summary>
    public void UpdateSuggestions()
    {
        if (_glideActive)
        {
            return;
        }
        if (_editor is null || _uiMode != KeyboardUiMode.Text)
        {
            SetSuggestions([]);
            return;
        }

        var list = new List<Suggestion>();
        var max = Settings.SuggestionCount;
        var before = _editor.GetTextBeforeCursor(64);

        // Emoji shortcode (":smile").
        if (Settings.EmojiSuggestions && !Attributes.IsPassword)
        {
            var m = EmojiShortcodeRegex().Match(before);
            if (m.Success)
            {
                foreach (var e in EmojiCatalog.Search(m.Groups[1].Value.Replace('_', ' '), max))
                {
                    list.Add(new Suggestion(ApplyDefaultSkinTone(e), SuggestionKind.Emoji, 1) { Description = e.Name });
                }
                SetSuggestions(list);
                return;
            }
        }

        var nlp = Settings.ShowSuggestions && Attributes.SupportsNlp;
        var word = _editor.IsComposingWord ? _editor.CurrentWord : string.Empty;

        // Recently copied text.
        if (word.Length == 0 && Settings.ClipboardSuggestion && !IsIncognito && ClipboardHistory.Primary is { } clip &&
            clip.Text != _lastClipboardSuggestion && DateTimeOffset.UtcNow - clip.Timestamp < TimeSpan.FromMinutes(1))
        {
            list.Add(new Suggestion(clip.Text, SuggestionKind.Clipboard, 1) { Description = "clipboard" });
        }

        if (nlp && GetProvider() is { } provider)
        {
            if (word.Length > 0 || Settings.NextWordPrediction)
            {
                var request = new SuggestionRequest
                {
                    Word = word,
                    PrecedingText = before.Substring(0, before.Length - word.Length),
                    MaxCount = max,
                    AllowAutoCorrect = Settings.AutoCorrect && Attributes.AllowAutoCorrect && !_rejectedCorrections.Contains(word),
                    Culture = Culture,
                    KeyProximity = CreateKeyProximity(),
                };
                try
                {
                    list.AddRange(provider.Suggest(request));
                }
                catch (Exception)
                {
                    // Suggestion providers must never break typing.
                }
            }
            if (Settings.EmojiSuggestions && word.Length >= 3)
            {
                var emoji = EmojiCatalog.Search(word, 1).FirstOrDefault(e => e.Name.Split(' ').Contains(word.ToLowerInvariant()) || e.Keywords.Contains(word.ToLowerInvariant()));
                if (emoji is not null && list.Count > 0)
                {
                    if (list.Count >= max)
                    {
                        list.RemoveAt(list.Count - 1);
                    }
                    list.Add(new Suggestion(ApplyDefaultSkinTone(emoji), SuggestionKind.Emoji, 0.5) { Description = emoji.Name });
                }
            }
        }
        SetSuggestions(list.Take(Math.Max(max, 1) + (list.FirstOrDefault()?.Kind == SuggestionKind.Clipboard ? 1 : 0)).ToList());
    }

    private string ApplyDefaultSkinTone(EmojiInfo emoji)
    {
        var tone = EmojiHistory.GetPreferredTone(emoji.Value) ?? Settings.EmojiSkinTone;
        return emoji.WithSkinTone(tone);
    }

    private void SetSuggestions(IReadOnlyList<Suggestion> suggestions)
    {
        if (_suggestions.Count == 0 && suggestions.Count == 0)
        {
            return;
        }
        _suggestions = suggestions;
        SuggestionsChanged?.Invoke(this, EventArgs.Empty);
    }

    private Func<char, char, double>? CreateKeyProximity()
    {
        if (_keyboard is null || _mode != Keys.KeyboardMode.Characters || _layoutWidth <= 0)
        {
            return null;
        }
        var centers = new Dictionary<char, (double X, double Y)>();
        double keyWidth = 0;
        foreach (var key in _keyboard.CharacterKeys)
        {
            var output = key.Data.AsString(false);
            if (output.Length == 1)
            {
                var c = TextUtils.BaseChar(output[0]);
                centers[c] = (key.TouchBounds.CenterX, key.TouchBounds.CenterY);
                keyWidth = Math.Max(keyWidth, key.TouchBounds.Width);
            }
        }
        if (centers.Count == 0 || keyWidth <= 0)
        {
            return null;
        }
        var threshold = keyWidth * 1.5;
        return (a, b) =>
        {
            if (centers.TryGetValue(a, out var pa) && centers.TryGetValue(b, out var pb))
            {
                var dx = pa.X - pb.X;
                var dy = pa.Y - pb.Y;
                return Math.Sqrt(dx * dx + dy * dy) <= threshold ? 0.6 : 1.0;
            }
            return 1.0;
        };
    }

    /// <summary>Applies the pending auto-correction (called before a separator is inserted).</summary>
    private void ApplyAutoCorrection(string separator)
    {
        if (_editor is null || !Settings.AutoCorrect || !Attributes.AllowAutoCorrect || !Attributes.SupportsNlp || !Settings.ShowSuggestions)
        {
            return;
        }
        if (!_editor.IsComposingWord)
        {
            return;
        }
        var word = _editor.CurrentWord;
        if (_rejectedCorrections.Contains(word))
        {
            return;
        }
        var correction = _suggestions.FirstOrDefault(s => s.IsAutoCommit && s.Kind == SuggestionKind.Correction);
        if (correction is null || correction.Text == word)
        {
            return;
        }
        WithoutTargetEvents(() => _editor.ReplaceCurrentWord(correction.Text));
        _lastAutoCorrection = new AutoCorrection(word, correction.Text, separator, _editor.SelectionStart + separator.Length);
    }

    private void LearnCurrentWord()
    {
        if (_editor is null || IsIncognito || !Settings.LearnWords || !Attributes.SupportsNlp || !_editor.IsComposingWord)
        {
            return;
        }
        var word = _editor.CurrentWord;
        var before = _editor.GetTextBeforeCursor(96);
        var context = before.Substring(0, before.Length - word.Length).TrimEnd();
        string? previous = null;
        if (context.Length > 0 && TextUtils.IsWordCoreChar(context[context.Length - 1]))
        {
            previous = TextUtils.GetCurrentWord(context);
        }
        GetProvider()?.Learn(word, previous);
    }

    /// <summary>Commits a suggestion chosen by the user.</summary>
    public void CommitSuggestion(Suggestion suggestion)
    {
        if (_editor is null)
        {
            return;
        }
        Feedback(FeedbackKind.KeyPress);
        _lastAutoCorrection = null;
        switch (suggestion.Kind)
        {
            case SuggestionKind.Clipboard:
                _lastClipboardSuggestion = suggestion.Text;
                InputText(suggestion.Text);
                return;
            case SuggestionKind.Emoji:
            {
                var before = _editor.GetTextBeforeCursor(64);
                var m = EmojiShortcodeRegex().Match(before);
                WithoutTargetEvents(() =>
                {
                    if (m.Success)
                    {
                        _editor.ReplaceBeforeCursor(m.Groups[1].Length + 1, suggestion.Text);
                    }
                    else
                    {
                        _editor.CommitText((_editor.IsComposingWord ? " " : string.Empty) + suggestion.Text);
                    }
                });
                if (!IsIncognito && Settings.EmojiHistoryEnabled)
                {
                    EmojiHistory.Add(suggestion.Text);
                }
                AfterEdit(suggestion.Text);
                return;
            }
        }

        var word = _editor.IsComposingWord ? _editor.CurrentWord : string.Empty;
        if (suggestion.Kind == SuggestionKind.Typed)
        {
            _rejectedCorrections.Add(word);
        }
        WithoutTargetEvents(() =>
        {
            if (suggestion.Kind == SuggestionKind.Prediction && word.Length == 0)
            {
                var before = _editor.GetTextBeforeCursor(1);
                var prefix = before.Length > 0 && !char.IsWhiteSpace(before[0]) ? " " : string.Empty;
                _editor.CommitText(prefix + suggestion.Text);
            }
            else if (suggestion.Kind == SuggestionKind.Glide && _lastGlideWord is not null && _editor.CurrentWord == _lastGlideWord)
            {
                _editor.ReplaceCurrentWord(suggestion.Text);
            }
            else
            {
                _editor.ReplaceCurrentWord(suggestion.Text);
            }
        });
        if (suggestion.Kind == SuggestionKind.Glide)
        {
            _lastGlideWord = suggestion.Text;
            AfterEdit(suggestion.Text);
            return;
        }
        LearnCurrentWord();
        if (!IsFollowedBySpace())
        {
            WithoutTargetEvents(() => _editor.CommitText(" "));
            _lastInsertWasAutoSpace = true;
        }
        AfterCharacterShift();
        AfterEdit(suggestion.Text);
    }

    /// <summary>Removes a suggestion permanently (learned words) and refreshes the list.</summary>
    public void RemoveSuggestion(Suggestion suggestion)
    {
        GetProvider()?.Forget(suggestion.Text);
        UpdateSuggestions();
    }

    // ================================================================== glide typing

    private void UpdateGlideWords()
    {
        if (!Settings.GlideTyping)
        {
            return;
        }
        var dictionary = LanguageModels.GetDictionary(_activeSubtype.LanguageTag);
        if (dictionary is null)
        {
            GlideClassifier.SetWords(Array.Empty<(string, int)>());
            return;
        }
        var user = LanguageModels.GetUserDictionary(_activeSubtype.LanguageTag);
        var words = dictionary.Words.Select(w => (w, dictionary.GetFrequency(w)))
            .Concat(user.Words.Where(p => p.Value >= user.LearnThreshold).Select(p => (p.Key, 128)));
        GlideClassifier.SetWords(words);
    }

    /// <summary>Whether glide typing can currently be used.</summary>
    public bool CanGlide =>
        Settings.GlideTyping && _mode == Keys.KeyboardMode.Characters && _uiMode == KeyboardUiMode.Text &&
        Attributes.SupportsNlp && LanguageModels.HasDictionary(_activeSubtype.LanguageTag);

    /// <summary>Starts a glide gesture at the given point.</summary>
    public void BeginGlide(double x, double y)
    {
        if (!CanGlide)
        {
            return;
        }
        if (!GlideClassifier.IsReady)
        {
            UpdateGlideWords();
            if (_keyboard is not null)
            {
                GlideClassifier.SetLayout(_keyboard.CharacterKeys);
            }
        }
        _glideActive = true;
        _glidePointsSincePreview = 0;
        GlideClassifier.Clear();
        GlideClassifier.AddPoint(x, y);
        OnStateChanged();
    }

    /// <summary>Adds a point to the current glide gesture (updates live suggestions).</summary>
    public void AddGlidePoint(double x, double y)
    {
        if (!_glideActive)
        {
            return;
        }
        GlideClassifier.AddPoint(x, y);
        if (Settings.GlidePreview && ++_glidePointsSincePreview >= 6)
        {
            _glidePointsSincePreview = 0;
            var words = GlideClassifier.GetSuggestions(Settings.SuggestionCount);
            _suggestions = words.Select(w => new Suggestion(ApplyGlideCase(w), SuggestionKind.Glide, 1)).ToList();
            SuggestionsChanged?.Invoke(this, EventArgs.Empty);
        }
    }

    /// <summary>Completes (or cancels) the glide gesture. When committed, the best word is typed.</summary>
    public void FinishGlide(bool commit = true)
    {
        if (!_glideActive)
        {
            return;
        }
        _glideActive = false;
        if (!commit || _editor is null)
        {
            GlideClassifier.Clear();
            UpdateSuggestions();
            OnStateChanged();
            return;
        }
        var words = GlideClassifier.GetSuggestions(Math.Max(3, Settings.SuggestionCount)).Select(ApplyGlideCase).ToList();
        GlideClassifier.Clear();
        if (words.Count == 0)
        {
            UpdateSuggestions();
            OnStateChanged();
            return;
        }
        var best = words[0];
        _lastAutoCorrection = null;
        WithoutTargetEvents(() =>
        {
            var before = _editor.GetTextBeforeCursor(1);
            var needsSpace = before.Length > 0 && !char.IsWhiteSpace(before[0]) && !"([{¿¡\"'".Contains(before[0]);
            _editor.CommitText((needsSpace ? " " : string.Empty) + best);
        });
        _lastGlideWord = best;
        _lastInsertWasAutoSpace = false;
        Feedback(FeedbackKind.KeyPress);
        TextCommitted?.Invoke(this, best);
        AfterCharacterShift();
        UpdateAutoCapitalization();
        _suggestions = words.Select(w => new Suggestion(w, SuggestionKind.Glide, 1)).ToList();
        SuggestionsChanged?.Invoke(this, EventArgs.Empty);
        OnStateChanged();
    }

    private string ApplyGlideCase(string word) => _shiftState switch
    {
        Keys.ShiftState.CapsLock => word.ToUpper(Culture),
        Keys.ShiftState.ShiftedManual or Keys.ShiftState.ShiftedAutomatic => TextUtils.Capitalize(word, Culture),
        _ => word,
    };
}
