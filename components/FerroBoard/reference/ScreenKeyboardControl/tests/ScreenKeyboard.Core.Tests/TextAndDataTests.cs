using ScreenKeyboard.Clipboard;
using ScreenKeyboard.Editor;
using ScreenKeyboard.Emoji;
using ScreenKeyboard.Glide;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Layouts;
using ScreenKeyboard.Settings;
using ScreenKeyboard.Text;

namespace ScreenKeyboard.Core.Tests;

public class TextAndDataTests
{
    private static WordDictionary English => TestHelpers.SharedModels.GetDictionary("en")!;

    [Fact]
    public void Bundled_dictionary_loads()
    {
        Assert.True(English.Count > 40000);
        Assert.True(English.GetFrequency("the") > 200);
        Assert.True(English.Contains("Hello"));
    }

    [Fact]
    public void Completion_is_frequency_ordered()
    {
        var results = English.Complete("th", 5);
        Assert.Equal(5, results.Count);
        Assert.Equal("the", results[0].Word);
        for (var i = 1; i < results.Count; i++)
        {
            Assert.True(results[i - 1].Frequency >= results[i].Frequency);
        }
    }

    [Fact]
    public void Fuzzy_search_finds_corrections()
    {
        var matches = English.FindSimilar("helo", 1, 10);
        Assert.Contains(matches, m => m.Word == "hello");
        var transposed = English.FindSimilar("teh", 1, 10);
        Assert.Contains(transposed, m => m.Word == "the");
    }

    [Fact]
    public void Provider_suggests_corrections_and_marks_autocorrect()
    {
        var provider = new DictionarySuggestionProvider(English);
        var result = provider.Suggest(new SuggestionRequest { Word = "becuase", MaxCount = 3 });
        Assert.Contains(result, s => s.Text == "because");
        Assert.Contains(result, s => s.IsAutoCommit && s.Text == "because");
    }

    [Fact]
    public void Provider_keeps_case_pattern()
    {
        var provider = new DictionarySuggestionProvider(English);
        var result = provider.Suggest(new SuggestionRequest { Word = "Tha", MaxCount = 3 });
        Assert.All(result, s => Assert.True(char.IsUpper(s.Text[0])));
    }

    [Fact]
    public void User_dictionary_roundtrips()
    {
        var user = new UserDictionary();
        user.Learn("foo", null);
        user.Learn("bar", "foo");
        user.Block("baz");
        var copy = new UserDictionary();
        copy.Import(user.Export());
        Assert.Equal(1, copy.GetCount("foo"));
        Assert.True(copy.IsBlocked("baz"));
        Assert.Equal("bar", copy.Predict("foo", 1)[0].Word);
    }

    [Fact]
    public void Word_list_text_format_is_supported()
    {
        var dict = WordDictionary.FromText("# comment\nhallo 200\nwelt\n");
        Assert.Equal(200, dict.GetFrequency("hallo"));
        Assert.True(dict.Contains("welt"));
    }

    [Fact]
    public void Glide_classifier_recognizes_word_from_ideal_path()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        var keyboard = computer.Compute(KeyboardMode.Characters, Subtype.Default);
        keyboard.Compute(new KeyComputeContext());
        keyboard.Layout(1000, 240);
        var classifier = new GlideTypingClassifier();
        classifier.SetLayout(keyboard.CharacterKeys);
        classifier.SetWords(English);

        foreach (var word in new[] { "hello", "world", "keyboard", "the" })
        {
            var points = new List<GesturePoint>();
            ComputedKey? previous = null;
            foreach (var c in word)
            {
                var key = keyboard.CharacterKeys.First(k => k.Data.AsString(false) == c.ToString());
                if (previous is not null)
                {
                    // Interpolate between key centers like a real finger path.
                    for (var t = 0.1; t < 1; t += 0.1)
                    {
                        points.Add(new GesturePoint(
                            previous.VisibleBounds.CenterX + (key.VisibleBounds.CenterX - previous.VisibleBounds.CenterX) * t,
                            previous.VisibleBounds.CenterY + (key.VisibleBounds.CenterY - previous.VisibleBounds.CenterY) * t));
                    }
                }
                points.Add(new GesturePoint(key.VisibleBounds.CenterX, key.VisibleBounds.CenterY));
                previous = key;
            }
            var result = classifier.Classify(points, 5);
            Assert.Contains(word, result.Take(3));
        }
    }

    [Fact]
    public void Emoji_catalog_loads_categories_and_searches()
    {
        var catalog = EmojiCatalog.Load("en");
        Assert.Equal(9, catalog.Categories.Count);
        Assert.True(catalog.All.Count() > 1800);
        Assert.Equal("🍕", catalog.Search("pizza").First().Value);
        var wave = catalog.Get("👋")!;
        Assert.True(wave.HasSkinTones);
        Assert.Equal("👋🏽", wave.WithSkinTone(EmojiSkinTone.Medium));
        Assert.Same(wave, catalog.Get("👋🏿"));
    }

    [Fact]
    public void Emoji_catalog_supports_other_languages()
    {
        var de = EmojiCatalog.Load("de-DE");
        Assert.Equal("de", de.Locale);
        Assert.NotEmpty(de.Search("Pizza"));
        Assert.Equal("en", EmojiCatalog.Load("xx").Locale);
    }

    [Fact]
    public void Emoji_history_orders_and_persists()
    {
        var history = new EmojiHistory { MaxRecent = 3 };
        history.Add("😀");
        history.Add("👋🏽");
        history.Add("😀");
        history.Add("🍕");
        history.Add("🎉");
        Assert.Equal(["🎉", "🍕", "😀"], history.Recent);
        Assert.Equal(EmojiSkinTone.Medium, history.GetPreferredTone("👋"));
        history.Pin("❤️");
        var copy = new EmojiHistory();
        copy.Import(history.Export());
        Assert.Equal(history.Combined, copy.Combined);
    }

    [Fact]
    public void Emoticons_load()
    {
        Assert.Contains(EmojiCatalog.LoadEmoticons(), e => e.Text == ":-)");
    }

    [Fact]
    public void Clipboard_history_limits_pins_and_expires()
    {
        var now = DateTimeOffset.UtcNow;
        var history = new ClipboardHistory { MaxItems = 2, ExpireAfter = TimeSpan.FromMinutes(10), Clock = () => now };
        var a = history.Add("a")!;
        history.SetPinned(a, true);
        history.Add("b");
        history.Add("c");
        history.Add("d");
        Assert.Equal(["a", "d", "c"], history.Items.Select(i => i.Text));
        now += TimeSpan.FromMinutes(11);
        Assert.Equal(["a"], history.Items.Select(i => i.Text));
        var copy = new ClipboardHistory();
        copy.Import(history.Export());
        Assert.True(copy.Items.Single().IsPinned);
    }

    [Fact]
    public void Settings_roundtrip_json()
    {
        var settings = new KeyboardSettings { NumberRow = true, SwipeUp = SwipeAction.Undo, ThemeMode = ThemeMode.Dark, Subtypes = ["de-DE/qwertz"] };
        var json = settings.ToJson();
        Assert.Contains("\"swipeUp\": \"Undo\"", json);
        var copy = KeyboardSettings.FromJson(json);
        Assert.True(copy.NumberRow);
        Assert.Equal(SwipeAction.Undo, copy.SwipeUp);
        Assert.Equal(["de-DE/qwertz"], copy.Subtypes);
    }

    [Fact]
    public void Settings_raise_property_changed()
    {
        var settings = new KeyboardSettings();
        var changed = new List<string?>();
        settings.PropertyChanged += (_, e) => changed.Add(e.PropertyName);
        settings.GlideTyping = false;
        settings.GlideTyping = false;
        Assert.Equal([nameof(KeyboardSettings.GlideTyping)], changed);
    }

    [Theory]
    [InlineData("hello world", 11, 6)]
    [InlineData("hello world  ", 13, 6)]
    [InlineData("a, b", 4, 3)]
    [InlineData("end...", 6, 3)]
    public void Previous_word_boundary(string text, int index, int expected)
    {
        Assert.Equal(expected, TextUtils.FindPreviousWordBoundary(text, index));
    }

    [Fact]
    public void Editor_vertical_navigation_keeps_column()
    {
        var target = new TextBufferTarget("abcdef\nab\nabcdef");
        var editor = new EditorController(target);
        editor.SetCaret(5);
        editor.MoveVertical(1);
        Assert.Equal(9, target.SelectionStart);
        editor.MoveVertical(1);
        Assert.Equal(15, target.SelectionStart);
        editor.MoveVertical(-1);
        editor.MoveVertical(-1);
        Assert.Equal(5, target.SelectionStart);
    }

    [Fact]
    public void Current_word_is_extracted()
    {
        Assert.Equal("don't", TextUtils.GetCurrentWord("I don't"));
        Assert.Equal("", TextUtils.GetCurrentWord("word "));
        Assert.Equal("über", TextUtils.GetCurrentWord("(über"));
    }
}
