using ScreenKeyboard.Editor;
using ScreenKeyboard.Engine;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Layouts;
using ScreenKeyboard.Settings;
using ScreenKeyboard.Text;

namespace ScreenKeyboard.Core.Tests;

public class EngineTests
{
    private static (KeyboardEngine Engine, TextBufferTarget Target, FakeClock Clock) Create(string text = "", InputAttributes? attrs = null, Action<KeyboardSettings>? configure = null)
    {
        var settings = new KeyboardSettings();
        configure?.Invoke(settings);
        var engine = new KeyboardEngine(settings, KeyboardResources.Default, TestHelpers.SharedModels);
        var clock = new FakeClock();
        engine.Clock = clock.Now;
        var target = new TextBufferTarget(text, attrs ?? InputAttributes.Default);
        engine.Attach(target);
        engine.UpdateLayout(1000, 240);
        return (engine, target, clock);
    }

    private static void Type(KeyboardEngine engine, FakeClock clock, string text)
    {
        foreach (var c in text)
        {
            clock.Advance(TimeSpan.FromMilliseconds(120));
            engine.TypeText(c.ToString());
        }
    }

    [Fact]
    public void Auto_capitalizes_start_of_sentence()
    {
        var (engine, target, clock) = Create();
        Assert.Equal(ShiftState.ShiftedAutomatic, engine.ShiftState);
        Type(engine, clock, "hi");
        Assert.Equal("Hi", target.Text);
        Assert.Equal(ShiftState.Unshifted, engine.ShiftState);
    }

    [Fact]
    public void Auto_capitalization_can_be_disabled()
    {
        var (engine, target, clock) = Create(configure: s => s.AutoCapitalization = false);
        Type(engine, clock, "hi");
        Assert.Equal("hi", target.Text);
    }

    [Fact]
    public void Capitalizes_after_sentence_terminator()
    {
        var (engine, target, clock) = Create(configure: s => s.AutoCorrect = false);
        Type(engine, clock, "ok. yes");
        Assert.Equal("Ok. Yes", target.Text);
    }

    [Fact]
    public void Shift_toggles_and_double_tap_enables_caps_lock()
    {
        var (engine, target, clock) = Create(configure: s => s.AutoCapitalization = false);
        engine.InputKey(TestHelpers.Key(KeyCode.Shift));
        Assert.Equal(ShiftState.ShiftedManual, engine.ShiftState);
        clock.Advance(TimeSpan.FromMilliseconds(100));
        engine.InputKey(TestHelpers.Key(KeyCode.Shift));
        Assert.Equal(ShiftState.CapsLock, engine.ShiftState);
        Type(engine, clock, "abc");
        Assert.Equal("ABC", target.Text);
        Assert.Equal(ShiftState.CapsLock, engine.ShiftState);
        engine.InputKey(TestHelpers.Key(KeyCode.Shift));
        Assert.Equal(ShiftState.Unshifted, engine.ShiftState);
    }

    [Fact]
    public void Shift_chording_resets_after_release()
    {
        var (engine, target, clock) = Create(configure: s => s.AutoCapitalization = false);
        var shift = TestHelpers.Key(KeyCode.Shift);
        engine.OnKeyDown(shift);
        engine.InputKey(new AutoTextKeyData { Code = 'a', Label = "a" }.Compute(engine.CreateComputeContext())!);
        engine.InputKey(new AutoTextKeyData { Code = 'b', Label = "b" }.Compute(engine.CreateComputeContext())!);
        engine.OnKeyUp(shift);
        Assert.Equal("AB", target.Text);
        Assert.Equal(ShiftState.Unshifted, engine.ShiftState);
    }

    [Fact]
    public void Double_space_inserts_period()
    {
        var (engine, target, clock) = Create(configure: s => s.AutoCorrect = false);
        Type(engine, clock, "hello  ");
        Assert.Equal("Hello. ", target.Text);
        Assert.Equal(ShiftState.ShiftedAutomatic, engine.ShiftState);
    }

    [Fact]
    public void Double_space_requires_quick_succession()
    {
        var (engine, target, clock) = Create(configure: s => s.AutoCorrect = false);
        Type(engine, clock, "hello ");
        clock.Advance(TimeSpan.FromSeconds(2));
        engine.TypeText(" ");
        Assert.Equal("Hello  ", target.Text);
    }

    [Fact]
    public void Autocorrects_and_backspace_reverts()
    {
        var (engine, target, clock) = Create(configure: s => s.AutoCapitalization = false);
        Type(engine, clock, "i like teh");
        Assert.Contains(engine.Suggestions, s => s.Text == "the" && s.IsAutoCommit);
        Type(engine, clock, " ");
        Assert.Equal("i like the ", target.Text);
        engine.InputKey(TestHelpers.Key(KeyCode.Delete));
        Assert.Equal("i like teh", target.Text);
        // The rejected correction is not applied again.
        Type(engine, clock, " ");
        Assert.Equal("i like teh ", target.Text);
    }

    [Fact]
    public void No_autocorrect_in_password_fields()
    {
        var (engine, target, clock) = Create(attrs: new InputAttributes { Kind = InputKind.Password });
        Type(engine, clock, "teh ");
        Assert.Equal("teh ", target.Text);
        Assert.Empty(engine.Suggestions);
    }

    [Fact]
    public void Suggestions_complete_prefix()
    {
        var (engine, _, clock) = Create(configure: s => s.AutoCapitalization = false);
        Type(engine, clock, "tha");
        Assert.Contains(engine.Suggestions, s => s.Text is "thank" or "that" or "thanks");
    }

    [Fact]
    public void Committing_suggestion_replaces_word_and_adds_space_then_punctuation_swaps()
    {
        var (engine, target, clock) = Create(configure: s => s.AutoCapitalization = false);
        Type(engine, clock, "hell");
        var suggestion = engine.Suggestions.First(s => s.Kind == SuggestionKind.Completion);
        engine.CommitSuggestion(suggestion);
        Assert.Equal(suggestion.Text + " ", target.Text);
        Type(engine, clock, ".");
        Assert.Equal(suggestion.Text + ". ", target.Text);
    }

    [Fact]
    public void Learns_words_and_predicts_next_word()
    {
        var models = new LanguageModels();
        var engine = new KeyboardEngine(new KeyboardSettings { AutoCorrect = false, AutoCapitalization = false }, KeyboardResources.Default, models);
        var clock = new FakeClock();
        engine.Clock = clock.Now;
        var target = new TextBufferTarget();
        engine.Attach(target);
        for (var i = 0; i < 3; i++)
        {
            Type(engine, clock, "zorblax quux ");
        }
        Type(engine, clock, "zorb");
        Assert.Contains(engine.Suggestions, s => s.Text == "zorblax");
        target.Text = string.Empty;
        target.MoveCaretExternally(0);
        Type(engine, clock, "zorblax ");
        Assert.Contains(engine.Suggestions, s => s.Kind == SuggestionKind.Prediction && s.Text == "quux");
    }

    [Fact]
    public void Incognito_does_not_learn()
    {
        var models = new LanguageModels();
        var engine = new KeyboardEngine(new KeyboardSettings { AutoCorrect = false, IncognitoMode = true }, KeyboardResources.Default, models);
        var target = new TextBufferTarget();
        engine.Attach(target);
        engine.TypeText("zorblax zorblax zorblax ");
        Assert.Empty(models.GetUserDictionary("en").Words);
    }

    [Fact]
    public void Delete_removes_whole_grapheme()
    {
        var (engine, target, _) = Create("family 👨‍👩‍👧 é́");
        engine.InputKey(TestHelpers.Key(KeyCode.Delete));
        Assert.Equal("family 👨‍👩‍👧 ", target.Text);
        engine.InputKey(TestHelpers.Key(KeyCode.Delete));
        engine.InputKey(TestHelpers.Key(KeyCode.Delete));
        Assert.Equal("family ", target.Text);
    }

    [Fact]
    public void Enter_inserts_newline_in_multiline_and_performs_action_otherwise()
    {
        var (engine, target, _) = Create(attrs: new InputAttributes { IsMultiline = true });
        engine.InputKey(TestHelpers.Key(KeyCode.Enter));
        Assert.Equal("\n", target.Text);

        var (engine2, target2, _) = Create(attrs: new InputAttributes { Kind = InputKind.Search });
        engine2.InputKey(TestHelpers.Key(KeyCode.Enter));
        Assert.Equal(string.Empty, target2.Text);
        Assert.Equal([EnterAction.Search], target2.PerformedActions);
    }

    [Fact]
    public void Input_kind_selects_initial_mode_and_variation()
    {
        var (engine, _, _) = Create(attrs: new InputAttributes { Kind = InputKind.Number });
        Assert.Equal(KeyboardMode.Numeric, engine.Mode);
        var (engine2, _, _) = Create(attrs: new InputAttributes { Kind = InputKind.Phone });
        Assert.Equal(KeyboardMode.Phone, engine2.Mode);
        var (engine3, _, _) = Create(attrs: new InputAttributes { Kind = InputKind.Email });
        Assert.Contains(engine3.Keyboard.Keys, k => k.Label == "@");
        Assert.Equal(ShiftState.Unshifted, engine3.ShiftState);
    }

    [Fact]
    public void Mode_switching_and_space_returns_to_characters()
    {
        var (engine, target, clock) = Create(configure: s => s.AutoCapitalization = false);
        engine.InputKey(TestHelpers.Key(KeyCode.ViewSymbols));
        Assert.Equal(KeyboardMode.Symbols, engine.Mode);
        Type(engine, clock, "1");
        engine.InputKey(TestHelpers.Key(KeyCode.Space));
        Assert.Equal(KeyboardMode.Characters, engine.Mode);
        Assert.Equal("1 ", target.Text);
    }

    [Fact]
    public void Subtype_switching_changes_layout()
    {
        var (engine, _, _) = Create();
        var de = engine.FindSubtype("de-DE")!;
        engine.SetSubtypes([Subtype.Default, de]);
        Assert.Equal("q", engine.Keyboard.Rows[0][0].Data.Label.ToLowerInvariant());
        engine.NextSubtype();
        Assert.Equal("de-DE", engine.ActiveSubtype.LanguageTag);
        Assert.Equal("z", engine.Keyboard.Rows[0][5].Data.Label.ToLowerInvariant());
        Assert.NotNull(engine.Keyboard.FindKey(KeyCode.LanguageSwitch));
    }

    [Fact]
    public void Hangul_composer_builds_syllables()
    {
        var (engine, target, _) = Create();
        engine.SetSubtypes([engine.FindSubtype("ko-KR")!]);
        engine.TypeText("ㅎㅏㄴ");
        Assert.Equal("한", target.Text);
    }

    [Fact]
    public void Telex_composer_applies_rules()
    {
        var composer = KeyboardResources.Default.GetComposer(new ComponentName("org.florisboard.composers", "telex"));
        Assert.Equal((1, "â"), composer.GetActions("a", "a"));
        Assert.Equal((1, "Â"), composer.GetActions("A", "a"));
    }

    [Fact]
    public void Kana_composer_applies_dakuten()
    {
        var composer = new KanaComposer();
        Assert.Equal((1, "が"), composer.GetActions("か", "゙"));
    }

    [Fact]
    public void Undo_and_redo_restore_text()
    {
        var (engine, target, clock) = Create(configure: s => { s.AutoCorrect = false; s.AutoCapitalization = false; });
        Type(engine, clock, "one ");
        clock.Advance(TimeSpan.FromSeconds(3));
        Type(engine, clock, "two");
        engine.InputKey(TestHelpers.Key(KeyCode.Undo));
        Assert.Equal("one ", target.Text);
        engine.InputKey(TestHelpers.Key(KeyCode.Redo));
        Assert.Equal("one two", target.Text);
    }

    [Fact]
    public void Arrow_keys_move_caret_and_selection_mode_selects()
    {
        var (engine, target, _) = Create("hello");
        engine.InputKey(TestHelpers.Key(KeyCode.ArrowLeft));
        Assert.Equal(4, target.SelectionStart);
        engine.InputKey(TestHelpers.Key(KeyCode.ClipboardSelect));
        engine.InputKey(TestHelpers.Key(KeyCode.ArrowLeft));
        engine.InputKey(TestHelpers.Key(KeyCode.ArrowLeft));
        Assert.Equal(2, target.SelectionStart);
        Assert.Equal(2, target.SelectionLength);
    }

    [Fact]
    public async Task Clipboard_copy_paste_and_history()
    {
        var (engine, target, _) = Create("copy me");
        engine.InputKey(TestHelpers.Key(KeyCode.ClipboardSelectAll));
        await engine.CopyAsync();
        Assert.Equal("copy me", engine.ClipboardHistory.Primary?.Text);
        engine.InputKey(TestHelpers.Key(KeyCode.ArrowRight));
        await engine.PasteAsync();
        Assert.Equal("copy mecopy me", target.Text);
    }

    [Fact]
    public void Emoji_shortcode_suggests_emoji()
    {
        var (engine, target, clock) = Create(configure: s => s.AutoCapitalization = false);
        Type(engine, clock, "yum :pizza");
        var emoji = engine.Suggestions.FirstOrDefault(s => s.Kind == SuggestionKind.Emoji);
        Assert.NotNull(emoji);
        engine.CommitSuggestion(emoji!);
        Assert.Equal("yum 🍕", target.Text);
        Assert.Contains("🍕", engine.EmojiHistory.Recent);
    }

    [Fact]
    public void Swipe_actions_move_cursor()
    {
        var (engine, target, _) = Create("abcdef");
        engine.ExecuteSwipeAction(SwipeAction.MoveCursorLeft, 3);
        Assert.Equal(3, target.SelectionStart);
        engine.ExecuteSwipeAction(SwipeAction.MoveCursorStartOfLine);
        Assert.Equal(0, target.SelectionStart);
    }

    [Fact]
    public void Precise_delete_removes_words()
    {
        var (engine, target, _) = Create("one two three");
        engine.BeginPreciseSelection();
        engine.UpdatePreciseSelection(2, byWords: true);
        Assert.Equal("two three", target.Text.Substring(target.SelectionStart, target.SelectionLength));
        engine.EndPreciseSelection(delete: true);
        Assert.Equal("one ", target.Text);
    }

    [Fact]
    public void External_changes_update_state()
    {
        var (engine, target, _) = Create("Hello. ");
        Assert.Equal(ShiftState.ShiftedAutomatic, engine.ShiftState);
        target.MoveCaretExternally(3);
        Assert.Equal(ShiftState.Unshifted, engine.ShiftState);
    }

    [Fact]
    public void Settings_changes_rebuild_keyboard()
    {
        var (engine, _, _) = Create();
        Assert.Equal(4, engine.Keyboard.RowCount);
        engine.Settings.NumberRow = true;
        Assert.Equal(5, engine.Keyboard.RowCount);
    }

    [Fact]
    public void Readme_example_works()
    {
        var engine = new KeyboardEngine(languageModels: TestHelpers.SharedModels);
        var field = new TextBufferTarget();
        engine.Attach(field);
        engine.TypeText("i like teh ");
        Assert.Equal("I like the ", field.Text);
    }

    [Fact]
    public void Phone_pad_has_letter_sub_labels_and_space_icon()
    {
        var (engine, _, _) = Create(attrs: new InputAttributes { Kind = InputKind.Phone });
        var two = engine.Keyboard.VisibleKeys.First(k => k.Label == "2");
        Assert.Equal("ABC", two.SubLabel);
        var (numeric, _, _) = Create(attrs: new InputAttributes { Kind = InputKind.Number });
        var space = numeric.Keyboard.FindKey(KeyCode.Space);
        if (space is not null)
        {
            Assert.Equal(KeyIcon.Space, space.Icon);
        }
    }

    [Fact]
    public void Floating_and_resize_keys()
    {
        var (engine, _, _) = Create();
        var resize = false;
        engine.ResizeModeRequested += (_, _) => resize = true;
        engine.InputKey(TestHelpers.Key(KeyCode.ToggleFloatingWindow));
        Assert.True(engine.Settings.Floating);
        engine.InputKey(TestHelpers.Key(KeyCode.ToggleResizeMode));
        Assert.True(resize);
    }

    [Fact]
    public void Space_bar_has_no_hint()
    {
        var (engine, _, _) = Create();
        Assert.Null(engine.Keyboard.FindKey(KeyCode.Space)!.HintLabel);
    }

    [Fact]
    public void Composer_rules_apply_via_custom_composer()
    {
        var resources = KeyboardResources.CreateDefault();
        resources.AddComposer(new RulesComposer("dash", "Dash", new Dictionary<string, string> { ["--"] = "—" }), "test");
        var engine = new KeyboardEngine(new KeyboardSettings { AutoCapitalization = false }, resources, TestHelpers.SharedModels);
        engine.SetSubtypes([Subtype.Default with { Composer = new ComponentName("test", "dash") }]);
        var target = new TextBufferTarget();
        engine.Attach(target);
        engine.TypeText("a--b");
        Assert.Equal("a—b", target.Text);
    }
}
