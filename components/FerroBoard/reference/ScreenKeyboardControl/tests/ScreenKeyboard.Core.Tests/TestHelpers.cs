using ScreenKeyboard.Editor;
using ScreenKeyboard.Engine;
using ScreenKeyboard.Input;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Layouts;
using ScreenKeyboard.Settings;

namespace ScreenKeyboard.Core.Tests;

internal static class TestHelpers
{
    private static readonly Lazy<LanguageModels> s_models = new(() => new LanguageModels());

    public static LanguageModels SharedModels => s_models.Value;

    public static (KeyboardEngine Engine, TextBufferTarget Target) CreateEngine(string text = "", InputAttributes? attributes = null, KeyboardSettings? settings = null)
    {
        settings ??= new KeyboardSettings();
        var engine = new KeyboardEngine(settings, KeyboardResources.Default, SharedModels);
        var clock = new FakeClock();
        engine.Clock = clock.Now;
        var target = new TextBufferTarget(text, attributes ?? InputAttributes.Default);
        engine.Attach(target);
        engine.UpdateLayout(1000, 240);
        return (engine, target);
    }

    public static TextKeyData Key(int code) => code switch
    {
        KeyCode.Space => TextKeyData.SpaceKey,
        KeyCode.Enter => TextKeyData.Function(KeyCode.Enter, KeyType.EnterEditing),
        KeyCode.Shift => TextKeyData.Function(KeyCode.Shift, KeyType.Modifier),
        KeyCode.Delete => TextKeyData.Function(KeyCode.Delete, KeyType.EnterEditing),
        _ when KeyCode.IsCharacter(code) => new TextKeyData { Code = code, Label = char.ConvertFromUtf32(code) },
        _ => TextKeyData.Function(code),
    };
}

internal sealed class FakeClock
{
    public DateTime Current { get; set; } = new(2026, 1, 1, 12, 0, 0, DateTimeKind.Utc);

    public DateTime Now() => Current;

    public void Advance(TimeSpan span) => Current += span;
}

/// <summary>A manually driven scheduler for touch processor tests.</summary>
internal sealed class FakeScheduler : IKeyboardScheduler
{
    private readonly List<Entry> _entries = new();

    public TimeSpan Now { get; private set; }

    public IDisposable Schedule(TimeSpan delay, Action action)
    {
        var entry = new Entry(Now + delay, action, this);
        _entries.Add(entry);
        return entry;
    }

    public void Advance(TimeSpan span)
    {
        var target = Now + span;
        while (true)
        {
            var next = _entries.Where(e => e.Due <= target).OrderBy(e => e.Due).FirstOrDefault();
            if (next is null)
            {
                break;
            }
            _entries.Remove(next);
            Now = next.Due;
            next.Action();
        }
        Now = target;
    }

    private sealed class Entry(TimeSpan due, Action action, FakeScheduler owner) : IDisposable
    {
        public TimeSpan Due { get; } = due;
        public Action Action { get; } = action;
        public void Dispose() => owner._entries.Remove(this);
    }
}
