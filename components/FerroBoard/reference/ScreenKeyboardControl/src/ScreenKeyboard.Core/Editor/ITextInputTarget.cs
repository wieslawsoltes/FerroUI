namespace ScreenKeyboard.Editor;

/// <summary>
/// The text field the keyboard types into. Implementations exist for Uno/WinUI <c>TextBox</c>,
/// <c>PasswordBox</c> and an in-memory buffer (<see cref="TextBufferTarget"/>). Implement it to connect the
/// keyboard to any custom editor.
/// </summary>
public interface ITextInputTarget
{
    /// <summary>Attributes of the field (kind, enter action, capitalization...).</summary>
    InputAttributes Attributes { get; }

    /// <summary>The full text of the field.</summary>
    string Text { get; }

    /// <summary>Selection start (caret position when <see cref="SelectionLength"/> is 0).</summary>
    int SelectionStart { get; }

    /// <summary>Selection length.</summary>
    int SelectionLength { get; }

    /// <summary>
    /// Replaces <paramref name="length"/> characters at <paramref name="start"/> with <paramref name="text"/> and
    /// places the caret right after the inserted text.
    /// </summary>
    void Replace(int start, int length, string text);

    /// <summary>Sets the selection.</summary>
    void Select(int start, int length);

    /// <summary>
    /// Performs the editor action for the enter key. Returns <c>true</c> if handled; otherwise the keyboard
    /// inserts a line break for multi-line fields.
    /// </summary>
    bool PerformEnterAction(EnterAction action);

    /// <summary>Raised when the text or the selection changed outside the keyboard.</summary>
    event EventHandler? Changed;
}

/// <summary>An in-memory <see cref="ITextInputTarget"/>, useful for tests, previews and search boxes.</summary>
public sealed class TextBufferTarget : ITextInputTarget
{
    private string _text;
    private int _selectionStart;
    private int _selectionLength;

    /// <summary>Creates a buffer.</summary>
    public TextBufferTarget(string text = "", InputAttributes? attributes = null)
    {
        _text = text;
        _selectionStart = text.Length;
        Attributes = attributes ?? InputAttributes.Default;
    }

    /// <inheritdoc />
    public InputAttributes Attributes { get; set; }

    /// <inheritdoc />
    public string Text
    {
        get => _text;
        set
        {
            _text = value ?? string.Empty;
            _selectionStart = Math.Min(_selectionStart, _text.Length);
            _selectionLength = Math.Min(_selectionLength, _text.Length - _selectionStart);
            Changed?.Invoke(this, EventArgs.Empty);
        }
    }

    /// <inheritdoc />
    public int SelectionStart => _selectionStart;

    /// <inheritdoc />
    public int SelectionLength => _selectionLength;

    /// <summary>Enter actions performed (for inspection in tests).</summary>
    public List<EnterAction> PerformedActions { get; } = new();

    /// <summary>Handler for enter actions; return <c>true</c> if handled.</summary>
    public Func<EnterAction, bool>? EnterActionHandler { get; set; }

    /// <inheritdoc />
    public event EventHandler? Changed;

    /// <summary>Raised whenever the keyboard modified the buffer.</summary>
    public event EventHandler? Edited;

    /// <inheritdoc />
    public void Replace(int start, int length, string text)
    {
        start = Math.Clamp(start, 0, _text.Length);
        length = Math.Clamp(length, 0, _text.Length - start);
        _text = _text.Remove(start, length).Insert(start, text);
        _selectionStart = start + text.Length;
        _selectionLength = 0;
        Edited?.Invoke(this, EventArgs.Empty);
    }

    /// <inheritdoc />
    public void Select(int start, int length)
    {
        start = Math.Clamp(start, 0, _text.Length);
        _selectionStart = start;
        _selectionLength = Math.Clamp(length, 0, _text.Length - start);
        Edited?.Invoke(this, EventArgs.Empty);
    }

    /// <inheritdoc />
    public bool PerformEnterAction(EnterAction action)
    {
        PerformedActions.Add(action);
        return EnterActionHandler?.Invoke(action) ?? false;
    }

    /// <summary>Simulates a caret move made by the user (raises <see cref="Changed"/>).</summary>
    public void MoveCaretExternally(int position)
    {
        _selectionStart = Math.Clamp(position, 0, _text.Length);
        _selectionLength = 0;
        Changed?.Invoke(this, EventArgs.Empty);
    }

    /// <inheritdoc />
    public override string ToString()
    {
        var caret = _text.Insert(_selectionStart + _selectionLength, _selectionLength > 0 ? "]" : "|");
        return _selectionLength > 0 ? caret.Insert(_selectionStart, "[") : caret;
    }
}
