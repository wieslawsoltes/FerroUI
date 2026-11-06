# Suggestions & dictionaries

The smartbar shows up to `SuggestionCount` candidates from the active language's `ISuggestionProvider`.

## What the default provider does

| Feature | Details |
|---|---|
| Completion | Most frequent dictionary words starting with the typed prefix (case and diacritic insensitive: `cafe` → `café`). |
| Correction | Bounded Damerau-Levenshtein search in a trie. Substitutions of physically adjacent keys and transpositions are cheaper, so typos like `teh` or `hwllo` rank well. |
| Auto-correct | When the typed word is unknown (or a rare spelling one edit away from a much more frequent word), the best correction is highlighted and applied when you type a space or punctuation. Backspace right after it restores what you typed (`UndoAutoCorrectOnBackspace`) and the word is not corrected again. |
| Typed word | Correctly spelled words are shown first; unknown words are offered so they can be kept. |
| Learning | Committed words and word pairs are learned (`LearnWords`, disabled in incognito mode, password and private fields). New words are suggested after being used twice (`UserDictionary.LearnThreshold`). |
| Next-word prediction | After a word, the most frequent learned follow-up words are offered (`NextWordPrediction`). |
| Removing | Long press a learned suggestion to remove and block it. |
| Emoji | `:pizza` style shortcodes suggest emojis, and words matching an emoji name add one emoji candidate (`EmojiSuggestions`). |
| Clipboard | Text copied within the last minute is offered once (`ClipboardSuggestion`). |

Related typing features: auto-capitalization at sentence start (respecting the field's capitalization mode),
double-space period, and smart punctuation spacing (`word .` → `word. ` after a suggestion was committed).

## Dictionaries

English (~50,000 words with frequencies, from FlorisBoard) is bundled. Register other languages with a
`WordDictionary` — FlorisBoard JSON (`{"word": frequency}`), plain text (`word [frequency]` per line) or pairs:

```csharp
var models = keyboard.Engine.LanguageModels;
models.Register("de", () => WordDictionary.FromText(File.ReadAllText("de_wordlist.txt"), "de"));
models.Register("fr", () => WordDictionary.FromJson(File.ReadAllText("fr.json"), "fr"));
models.Register("xx", WordDictionary.FromWords([("hello", 200), ("world", 180)], "xx"));
```

Frequencies are on a 0–255 scale (higher is more frequent). Dictionaries load lazily on first use. Glide typing
uses the same dictionaries.

## User dictionaries

```csharp
var user = keyboard.Engine.LanguageModels.GetUserDictionary("en");
string json = user.Export();   // persist
user.Import(json);             // restore
user.Block("damn");            // never suggest
user.Clear();                  // forget learned words
keyboard.Engine.LanguageModels.UserDictionaryChanged += (_, language) => Save(language);
```

## Custom providers

Plug in any engine (a server side language model, a domain vocabulary, ...):

```csharp
public sealed class ProductNames : ISuggestionProvider
{
    public IReadOnlyList<Suggestion> Suggest(SuggestionRequest request) =>
        _catalog.Where(p => p.StartsWith(request.Word, StringComparison.OrdinalIgnoreCase))
                .Take(request.MaxCount)
                .Select(p => new Suggestion(p, SuggestionKind.Completion, 0.9))
                .ToList();

    public void Learn(string word, string? previousWord) { }
    public void Forget(string word) { }
}

keyboard.Engine.SuggestionProviderFactory = subtype => subtype.LanguageTag.StartsWith("en") ? new ProductNames() : null;
```

Return `IsAutoCommit = true` on a suggestion to have it applied on the next separator. `SuggestionRequest.KeyProximity`
provides the key-distance cost function of the current layout.

## Controlling suggestions per field

- `TextBox.IsTextPredictionEnabled="False"` or `sk:ScreenKeyboardInput.SuggestionsEnabled="False"` disables them.
- Password fields never show suggestions and never learn.
- `sk:ScreenKeyboardInput.IsPrivate="True"` keeps suggestions but disables learning and history.
