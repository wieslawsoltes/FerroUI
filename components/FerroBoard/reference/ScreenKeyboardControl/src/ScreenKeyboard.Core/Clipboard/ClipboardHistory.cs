using System.Text;
using System.Text.Json;

namespace ScreenKeyboard.Clipboard;

/// <summary>Access to the system clipboard (implemented by the UI layer).</summary>
public interface IClipboardService
{
    /// <summary>Reads text from the clipboard, or <c>null</c>.</summary>
    Task<string?> GetTextAsync();

    /// <summary>Writes text to the clipboard.</summary>
    Task SetTextAsync(string text);
}

/// <summary>An in-process clipboard (used when no system clipboard is available and in tests).</summary>
public sealed class InMemoryClipboardService : IClipboardService
{
    private string? _text;

    /// <inheritdoc />
    public Task<string?> GetTextAsync() => Task.FromResult(_text);

    /// <inheritdoc />
    public Task SetTextAsync(string text)
    {
        _text = text;
        return Task.CompletedTask;
    }
}

/// <summary>An entry of the clipboard history.</summary>
public sealed class ClipboardItem
{
    /// <summary>Unique id.</summary>
    public Guid Id { get; init; } = Guid.NewGuid();

    /// <summary>The copied text.</summary>
    public required string Text { get; init; }

    /// <summary>When the item was copied (UTC).</summary>
    public DateTimeOffset Timestamp { get; set; } = DateTimeOffset.UtcNow;

    /// <summary>Whether the item is pinned (never expires, kept on clear).</summary>
    public bool IsPinned { get; set; }

    /// <inheritdoc />
    public override string ToString() => Text;
}

/// <summary>Clipboard history with pinning, size limit and expiry.</summary>
public sealed class ClipboardHistory
{
    private readonly List<ClipboardItem> _items = new();

    /// <summary>Raised when the history changed.</summary>
    public event EventHandler? Changed;

    /// <summary>Maximum number of unpinned items.</summary>
    public int MaxItems { get; set; } = 30;

    /// <summary>Unpinned items older than this are removed (<c>null</c> = never).</summary>
    public TimeSpan? ExpireAfter { get; set; } = TimeSpan.FromHours(1);

    /// <summary>Clock (overridable for tests).</summary>
    public Func<DateTimeOffset> Clock { get; set; } = () => DateTimeOffset.UtcNow;

    /// <summary>Items: pinned first, then most recent first.</summary>
    public IReadOnlyList<ClipboardItem> Items
    {
        get
        {
            Prune();
            return _items.Where(i => i.IsPinned).Concat(_items.Where(i => !i.IsPinned)).ToList();
        }
    }

    /// <summary>The most recent item, if any.</summary>
    public ClipboardItem? Primary => _items.Where(i => !i.IsPinned).FirstOrDefault() ?? _items.FirstOrDefault();

    /// <summary>Adds text (moving an identical entry to the top).</summary>
    public ClipboardItem? Add(string text)
    {
        if (string.IsNullOrEmpty(text))
        {
            return null;
        }
        var existing = _items.FirstOrDefault(i => i.Text == text);
        if (existing is not null)
        {
            _items.Remove(existing);
            existing.Timestamp = Clock();
            _items.Insert(0, existing);
        }
        else
        {
            existing = new ClipboardItem { Text = text, Timestamp = Clock() };
            _items.Insert(0, existing);
        }
        Prune();
        Changed?.Invoke(this, EventArgs.Empty);
        return existing;
    }

    /// <summary>Pins or unpins an item.</summary>
    public void SetPinned(ClipboardItem item, bool pinned)
    {
        item.IsPinned = pinned;
        Changed?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>Removes an item.</summary>
    public void Remove(ClipboardItem item)
    {
        if (_items.Remove(item))
        {
            Changed?.Invoke(this, EventArgs.Empty);
        }
    }

    /// <summary>Removes all unpinned items (or everything when <paramref name="includePinned"/>).</summary>
    public void Clear(bool includePinned = false)
    {
        _items.RemoveAll(i => includePinned || !i.IsPinned);
        Changed?.Invoke(this, EventArgs.Empty);
    }

    private void Prune()
    {
        var now = Clock();
        if (ExpireAfter is { } expiry)
        {
            _items.RemoveAll(i => !i.IsPinned && now - i.Timestamp > expiry);
        }
        var unpinned = _items.Where(i => !i.IsPinned).ToList();
        for (var i = MaxItems; i < unpinned.Count; i++)
        {
            _items.Remove(unpinned[i]);
        }
    }

    /// <summary>Serializes the history to JSON.</summary>
    public string Export(bool pinnedOnly = false)
    {
        using var stream = new MemoryStream();
        using (var w = new Utf8JsonWriter(stream))
        {
            w.WriteStartArray();
            foreach (var item in _items.Where(i => !pinnedOnly || i.IsPinned))
            {
                w.WriteStartObject();
                w.WriteString("text", item.Text);
                w.WriteString("timestamp", item.Timestamp);
                w.WriteBoolean("pinned", item.IsPinned);
                w.WriteEndObject();
            }
            w.WriteEndArray();
        }
        return Encoding.UTF8.GetString(stream.ToArray());
    }

    /// <summary>Restores items from JSON produced by <see cref="Export"/>.</summary>
    public void Import(string json)
    {
        using var doc = JsonDocument.Parse(json);
        _items.Clear();
        foreach (var el in doc.RootElement.EnumerateArray())
        {
            _items.Add(new ClipboardItem
            {
                Text = el.GetProperty("text").GetString() ?? string.Empty,
                Timestamp = el.TryGetProperty("timestamp", out var ts) ? ts.GetDateTimeOffset() : Clock(),
                IsPinned = el.TryGetProperty("pinned", out var p) && p.GetBoolean(),
            });
        }
        Changed?.Invoke(this, EventArgs.Empty);
    }
}
