using ScreenKeyboard.Keys;
using ScreenKeyboard.Layouts;

namespace ScreenKeyboard.Core.Tests;

public class LayoutTests
{
    private static KeyComputeContext Ctx(KeyboardMode mode = KeyboardMode.Characters, ShiftState shift = ShiftState.Unshifted) =>
        new() { Mode = mode, ShiftState = shift, SpaceBarLabel = "English" };

    [Fact]
    public void Default_resources_contain_bundled_florisboard_data()
    {
        var resources = KeyboardResources.Default;
        Assert.True(resources.GetLayoutMetadata(LayoutType.Characters).Count >= 70);
        Assert.True(resources.SubtypePresets.Count >= 60);
        Assert.True(resources.CurrencySets.Count >= 10);
        Assert.Contains(resources.Composers, c => c.Id == "hangul-unicode");
        Assert.NotNull(resources.GetPopupMapping(new ComponentName("org.florisboard.localization", "de")));
    }

    [Fact]
    public void Every_bundled_layout_parses()
    {
        var resources = KeyboardResources.Default;
        var count = 0;
        foreach (LayoutType type in Enum.GetValues<LayoutType>())
        {
            foreach (var name in resources.GetLayoutNames(type))
            {
                var layout = resources.GetLayout(type, name);
                Assert.NotNull(layout);
                count++;
            }
        }
        Assert.True(count > 120, $"Only {count} layouts found");
    }

    [Fact]
    public void Every_popup_mapping_parses()
    {
        var resources = KeyboardResources.Default;
        foreach (var name in resources.PopupMappingNames)
        {
            Assert.NotNull(resources.GetPopupMapping(name));
        }
    }

    [Fact]
    public void Every_subtype_preset_computes_all_modes()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        foreach (var subtype in KeyboardResources.Default.SubtypePresets)
        {
            foreach (var mode in Enum.GetValues<KeyboardMode>())
            {
                var keyboard = computer.Compute(mode, subtype, numberRow: true);
                keyboard.Compute(Ctx(mode));
                keyboard.Layout(800, 250);
                Assert.True(keyboard.RowCount > 0, $"{subtype.Id} {mode} has no rows");
            }
        }
    }

    [Fact]
    public void Qwerty_is_merged_with_modifier_layout()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        var keyboard = computer.Compute(KeyboardMode.Characters, Subtype.Default);
        keyboard.Compute(Ctx());
        Assert.Equal(4, keyboard.RowCount);
        Assert.Equal("q", keyboard.Rows[0][0].Label);
        Assert.Equal(KeyCode.Shift, keyboard.Rows[2][0].Code);
        Assert.Equal(KeyCode.Delete, keyboard.Rows[2][^1].Code);
        Assert.Contains(keyboard.Rows[3], k => k.Code == KeyCode.Space);
        Assert.Contains(keyboard.Rows[3], k => k.Code == KeyCode.Enter);
    }

    [Fact]
    public void Number_row_adds_a_row()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        var keyboard = computer.Compute(KeyboardMode.Characters, Subtype.Default, numberRow: true);
        keyboard.Compute(Ctx());
        Assert.Equal(5, keyboard.RowCount);
        Assert.Equal("1", keyboard.Rows[0][0].Label);
    }

    [Fact]
    public void Shift_uppercases_auto_text_keys()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        var keyboard = computer.Compute(KeyboardMode.Characters, Subtype.Default);
        keyboard.Compute(Ctx(shift: ShiftState.ShiftedManual));
        Assert.Equal("Q", keyboard.Rows[0][0].Label);
        Assert.Equal(KeyIcon.ShiftActive, keyboard.Rows[2][0].Icon);
        keyboard.Compute(Ctx(shift: ShiftState.CapsLock));
        Assert.Equal(KeyIcon.CapsLock, keyboard.Rows[2][0].Icon);
    }

    [Fact]
    public void Keys_have_hints_and_popups()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        var keyboard = computer.Compute(KeyboardMode.Characters, Subtype.Default);
        keyboard.Compute(Ctx());
        var q = keyboard.Rows[0][0];
        Assert.Equal("1", q.HintLabel);
        Assert.Contains(q.PopupKeys, k => k.AsString(false) == "1");
        var a = keyboard.Rows[1][0];
        Assert.Equal("a", a.Label);
        Assert.Contains(a.PopupKeys, k => k.AsString(false) == "á");
        Assert.Equal("@", a.HintLabel);
    }

    [Fact]
    public void Popups_are_uppercased_when_shifted()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        var keyboard = computer.Compute(KeyboardMode.Characters, Subtype.Default);
        keyboard.Compute(Ctx(shift: ShiftState.ShiftedManual));
        var a = keyboard.Rows[1][0];
        Assert.Contains(a.PopupKeys, k => k.AsString(false) == "Á");
    }

    [Fact]
    public void Layout_fills_width_without_overlap()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        var keyboard = computer.Compute(KeyboardMode.Characters, Subtype.Default);
        keyboard.Compute(Ctx());
        keyboard.Layout(1000, 240, new KeyboardGeometryOptions { KeySpacingHorizontal = 4, KeySpacingVertical = 8 });
        foreach (var row in keyboard.Rows)
        {
            var visible = row.Where(k => k.IsVisible).ToList();
            Assert.Equal(0, visible[0].TouchBounds.Left, 3);
            Assert.Equal(1000, visible[^1].TouchBounds.Right, 3);
            for (var i = 1; i < visible.Count; i++)
            {
                Assert.Equal(visible[i - 1].TouchBounds.Right, visible[i].TouchBounds.Left, 3);
                Assert.True(visible[i].VisibleBounds.Left >= visible[i - 1].VisibleBounds.Right);
            }
        }
        var space = keyboard.FindKey(KeyCode.Space)!;
        Assert.True(space.TouchBounds.Width > 300, "space should grow");
    }

    [Fact]
    public void Split_layout_inserts_a_gap()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        var keyboard = computer.Compute(KeyboardMode.Characters, Subtype.Default);
        keyboard.Compute(Ctx());
        keyboard.Layout(1000, 240, new KeyboardGeometryOptions { SplitGap = 200 });
        var t = keyboard.Rows[0][4];
        var y = keyboard.Rows[0][5];
        Assert.True(y.TouchBounds.Left - t.TouchBounds.Right >= 199);
        Assert.Equal(y, keyboard.HitTest(y.TouchBounds.CenterX, y.TouchBounds.CenterY));
        // Only the space bar may cross the gap; all other keys stay in their half.
        const double mid = 400, gapEnd = 600;
        foreach (var key in keyboard.VisibleKeys.Where(k => k.Code != Keys.KeyCode.Space))
        {
            Assert.False(key.VisibleBounds.Left < gapEnd - 0.5 && key.VisibleBounds.Right > mid + 0.5, $"{key} crosses the split gap");
        }
        Assert.True(keyboard.VisibleKeys.All(k => k.TouchBounds.Right <= 1000.01));
    }

    [Fact]
    public void Hit_test_returns_key_under_point()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        var keyboard = computer.Compute(KeyboardMode.Characters, Subtype.Default);
        keyboard.Compute(Ctx());
        keyboard.Layout(1000, 240);
        var w = keyboard.Rows[0][1];
        Assert.Same(w, keyboard.HitTest(w.TouchBounds.CenterX, w.TouchBounds.CenterY));
    }

    [Fact]
    public void Utility_key_visibility_follows_settings()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        var keyboard = computer.Compute(KeyboardMode.Characters, Subtype.Default);
        keyboard.Compute(Ctx() with { HasMultipleSubtypes = false });
        Assert.NotNull(keyboard.FindKey(KeyCode.ImeUiModeMedia));
        Assert.Null(keyboard.FindKey(KeyCode.LanguageSwitch));
        keyboard.Compute(Ctx() with { HasMultipleSubtypes = true });
        Assert.Null(keyboard.FindKey(KeyCode.ImeUiModeMedia));
        Assert.NotNull(keyboard.FindKey(KeyCode.LanguageSwitch));
    }

    [Fact]
    public void Currency_slots_are_resolved()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        var keyboard = computer.Compute(KeyboardMode.Symbols, Subtype.Default);
        var euro = KeyboardResources.Default.GetCurrencySet(new ComponentName("org.florisboard.currencysets", "euro"));
        keyboard.Compute(Ctx(KeyboardMode.Symbols) with { CurrencySet = euro });
        Assert.Contains(keyboard.Keys, k => k.Label == "€");
        Assert.DoesNotContain(keyboard.Keys, k => KeyCode.IsCurrencySlot(k.Code));
    }

    [Fact]
    public void Custom_layout_json_roundtrips()
    {
        const string json = """
        {
          "type": "characters",
          "id": "abc",
          "label": "ABC test",
          "arrangement": [
            [ "a", "b", "c", { "code": 100, "label": "d", "popup": { "main": { "code": 101, "label": "e" } } } ],
            [ { "$": "case_selector", "lower": { "code": 102, "label": "f" }, "upper": { "code": 70, "label": "F" } } ]
          ]
        }
        """;
        var resources = KeyboardResources.CreateDefault();
        var layout = resources.AddLayoutJson(json);
        Assert.Equal(2, layout.Arrangement.Rows.Count);
        var written = LayoutJson.WriteLayoutDocument(layout);
        var reparsed = LayoutJson.ReadLayoutDocument(written);
        Assert.Equal(4, reparsed.Arrangement.Rows[0].Count);
        var subtype = Subtype.Default with { Layouts = new SubtypeLayoutMap { Characters = layout.Name } };
        var keyboard = new LayoutComputer(resources).Compute(KeyboardMode.Characters, subtype);
        keyboard.Compute(Ctx());
        Assert.Equal("a", keyboard.Rows[0][0].Label);
        Assert.Contains(keyboard.Keys, k => k.Code == KeyCode.Space);
    }

    [Fact]
    public void Popup_arrangement_places_default_above_key()
    {
        var computer = new LayoutComputer(KeyboardResources.Default);
        var keyboard = computer.Compute(KeyboardMode.Characters, Subtype.Default);
        keyboard.Compute(Ctx());
        keyboard.Layout(1000, 240);
        var e = keyboard.Rows[0][2];
        var (bounds, selected) = Input.KeyboardTouchProcessor.ArrangePopup(e, e.PopupKeys.Count, e.PopupDefaultIndex, 1000);
        Assert.Equal(e.PopupKeys.Count, bounds.Count);
        Assert.True(bounds[selected].Bottom <= e.VisibleBounds.Top + 0.01);
        Assert.True(bounds.All(b => b.Left >= 0 && b.Right <= 1000.01));
        // No two popup keys overlap.
        for (var i = 0; i < bounds.Count; i++)
            for (var j = i + 1; j < bounds.Count; j++)
                Assert.False(bounds[i].Left < bounds[j].Right - 0.01 && bounds[j].Left < bounds[i].Right - 0.01 &&
                             bounds[i].Top < bounds[j].Bottom - 0.01 && bounds[j].Top < bounds[i].Bottom - 0.01);
    }
}
