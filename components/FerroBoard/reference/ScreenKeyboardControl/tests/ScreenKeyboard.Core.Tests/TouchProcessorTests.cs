using ScreenKeyboard.Editor;
using ScreenKeyboard.Engine;
using ScreenKeyboard.Input;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Layouts;
using ScreenKeyboard.Settings;

namespace ScreenKeyboard.Core.Tests;

public class TouchProcessorTests
{
    private sealed record Fixture(KeyboardEngine Engine, TextBufferTarget Target, KeyboardTouchProcessor Touch, FakeScheduler Scheduler);

    private static Fixture Create(string text = "", Action<KeyboardSettings>? configure = null)
    {
        var settings = new KeyboardSettings { AutoCapitalization = false };
        configure?.Invoke(settings);
        var engine = new KeyboardEngine(settings, KeyboardResources.Default, TestHelpers.SharedModels);
        var target = new TextBufferTarget(text);
        engine.Attach(target);
        engine.UpdateLayout(1000, 240);
        var scheduler = new FakeScheduler();
        return new Fixture(engine, target, new KeyboardTouchProcessor(engine, scheduler), scheduler);
    }

    private static ComputedKey KeyOf(KeyboardEngine engine, string label) =>
        engine.Keyboard.VisibleKeys.First(k => k.Label == label);

    private static void Tap(Fixture f, ComputedKey key, long id = 1)
    {
        f.Touch.PointerDown(id, key.TouchBounds.CenterX, key.TouchBounds.CenterY);
        f.Scheduler.Advance(TimeSpan.FromMilliseconds(50));
        f.Touch.PointerUp(id, key.TouchBounds.CenterX, key.TouchBounds.CenterY);
    }

    [Fact]
    public void Tap_commits_character()
    {
        var f = Create();
        var pressed = new List<ComputedKey>();
        f.Touch.KeyPressed += (_, k) => pressed.Add(k);
        Tap(f, KeyOf(f.Engine, "h"));
        Tap(f, KeyOf(f.Engine, "i"));
        Assert.Equal("hi", f.Target.Text);
        Assert.Equal(2, pressed.Count);
    }

    [Fact]
    public void Rollover_commits_first_key_when_second_finger_goes_down()
    {
        var f = Create();
        var a = KeyOf(f.Engine, "a");
        var s = KeyOf(f.Engine, "s");
        f.Touch.PointerDown(1, a.TouchBounds.CenterX, a.TouchBounds.CenterY);
        f.Touch.PointerDown(2, s.TouchBounds.CenterX, s.TouchBounds.CenterY);
        Assert.Equal("a", f.Target.Text);
        f.Touch.PointerUp(1, a.TouchBounds.CenterX, a.TouchBounds.CenterY);
        f.Touch.PointerUp(2, s.TouchBounds.CenterX, s.TouchBounds.CenterY);
        Assert.Equal("as", f.Target.Text);
    }

    [Fact]
    public void Long_press_opens_popup_and_commits_selection()
    {
        var f = Create();
        var e = KeyOf(f.Engine, "e");
        f.Touch.PointerDown(1, e.TouchBounds.CenterX, e.TouchBounds.CenterY);
        f.Scheduler.Advance(TimeSpan.FromMilliseconds(f.Engine.Settings.LongPressDelay + 10));
        var popup = f.Touch.Popup;
        Assert.NotNull(popup);
        // Move onto the second popup key and release.
        var targetIndex = popup!.Keys.Count > 1 ? 1 : 0;
        var bounds = popup.Bounds[targetIndex];
        f.Touch.PointerMove(1, bounds.CenterX, bounds.CenterY);
        Assert.Equal(targetIndex, f.Touch.Popup!.SelectedIndex);
        f.Touch.PointerUp(1, bounds.CenterX, bounds.CenterY);
        Assert.Null(f.Touch.Popup);
        Assert.Equal(popup.Keys[targetIndex].AsString(false), f.Target.Text);
    }

    [Fact]
    public void Delete_repeats_while_held()
    {
        var f = Create("abcdefghij");
        var del = f.Engine.Keyboard.FindKey(KeyCode.Delete)!;
        f.Touch.PointerDown(1, del.TouchBounds.CenterX, del.TouchBounds.CenterY);
        f.Scheduler.Advance(TimeSpan.FromMilliseconds(f.Engine.Settings.KeyRepeatDelay + f.Engine.Settings.KeyRepeatInterval * 3 + 5));
        f.Touch.PointerUp(1, del.TouchBounds.CenterX, del.TouchBounds.CenterY);
        Assert.Equal("abcdef", f.Target.Text);
    }

    [Fact]
    public void Space_drag_moves_cursor()
    {
        var f = Create("hello world");
        var space = f.Engine.Keyboard.FindKey(KeyCode.Space)!;
        var x = space.TouchBounds.CenterX;
        var y = space.TouchBounds.CenterY;
        f.Touch.PointerDown(1, x, y);
        for (var i = 1; i <= 10; i++)
        {
            f.Touch.PointerMove(1, x - i * 7, y);
        }
        f.Touch.PointerUp(1, x - 70, y);
        Assert.Equal("hello world", f.Target.Text);
        Assert.Equal(11 - 5, f.Target.SelectionStart);
    }

    [Fact]
    public void Glide_over_keys_types_word()
    {
        var f = Create();
        var keys = "hello".Select(c => KeyOf(f.Engine, c.ToString())).ToList();
        f.Touch.PointerDown(1, keys[0].VisibleBounds.CenterX, keys[0].VisibleBounds.CenterY);
        for (var i = 1; i < keys.Count; i++)
        {
            var from = keys[i - 1].VisibleBounds;
            var to = keys[i].VisibleBounds;
            for (var t = 0.1; t <= 1.0001; t += 0.1)
            {
                f.Touch.PointerMove(1, from.CenterX + (to.CenterX - from.CenterX) * t, from.CenterY + (to.CenterY - from.CenterY) * t);
            }
        }
        Assert.True(f.Engine.IsGliding);
        Assert.NotEmpty(f.Touch.GlideTrail);
        f.Touch.PointerUp(1, keys[^1].VisibleBounds.CenterX, keys[^1].VisibleBounds.CenterY);
        Assert.Equal("hello", f.Target.Text);
        Assert.False(f.Engine.IsGliding);
    }

    [Fact]
    public void Swipe_down_hides_keyboard_when_glide_disabled()
    {
        var f = Create(configure: s => s.GlideTyping = false);
        var hidden = false;
        f.Engine.HideRequested += (_, _) => hidden = true;
        var g = KeyOf(f.Engine, "g");
        f.Touch.PointerDown(1, g.TouchBounds.CenterX, g.TouchBounds.Top + 2);
        f.Touch.PointerMove(1, g.TouchBounds.CenterX, g.TouchBounds.Top + 40);
        f.Touch.PointerMove(1, g.TouchBounds.CenterX, g.TouchBounds.Top + 90);
        f.Touch.PointerUp(1, g.TouchBounds.CenterX, g.TouchBounds.Top + 90);
        Assert.True(hidden);
        Assert.Equal(string.Empty, f.Target.Text);
    }

    [Fact]
    public void Delete_swipe_left_deletes_words_precisely()
    {
        var f = Create("one two three");
        var del = f.Engine.Keyboard.FindKey(KeyCode.Delete)!;
        var x = del.TouchBounds.CenterX;
        var y = del.TouchBounds.CenterY;
        f.Touch.PointerDown(1, x, y);
        f.Touch.PointerMove(1, x - 20, y);
        f.Touch.PointerMove(1, x - 35, y);
        f.Touch.PointerUp(1, x - 35, y);
        Assert.Equal("one ", f.Target.Text);
    }

    [Fact]
    public void Long_press_shift_enables_caps_lock()
    {
        var f = Create();
        var shift = f.Engine.Keyboard.FindKey(KeyCode.Shift)!;
        f.Touch.PointerDown(1, shift.TouchBounds.CenterX, shift.TouchBounds.CenterY);
        f.Scheduler.Advance(TimeSpan.FromMilliseconds(f.Engine.Settings.LongPressDelay + 10));
        f.Touch.PointerUp(1, shift.TouchBounds.CenterX, shift.TouchBounds.CenterY);
        Assert.Equal(ShiftState.CapsLock, f.Engine.ShiftState);
    }
}
