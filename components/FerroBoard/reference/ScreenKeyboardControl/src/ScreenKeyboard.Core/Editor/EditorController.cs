using System.Globalization;
using ScreenKeyboard.Layouts;

namespace ScreenKeyboard.Editor;

/// <summary>
/// High level editing operations on top of an <see cref="ITextInputTarget"/>: grapheme-aware deletion,
/// word and line navigation, selection, suggestion replacement and a keyboard-level undo/redo history.
/// </summary>
public sealed class EditorController
{
    private readonly List<Snapshot> _undo = new();
    private readonly List<Snapshot> _redo = new();
    private DateTime _lastEditTime;
    private EditKind _lastEditKind;
    private int? _preferredColumn;

    /// <summary>Creates a controller for the target.</summary>
    public EditorController(ITextInputTarget target)
    {
        Target = target;
    }

    /// <summary>The underlying target.</summary>
    public ITextInputTarget Target { get; }

    /// <summary>Maximum number of undo steps.</summary>
    public int MaxUndoSteps { get; set; } = 100;

    /// <summary>Typing pause after which a new undo step is started.</summary>
    public TimeSpan UndoGroupTimeout { get; set; } = TimeSpan.FromSeconds(1.5);

    /// <summary>Clock used for undo grouping (overridable for tests).</summary>
    public Func<DateTime> Clock { get; set; } = () => DateTime.UtcNow;

    /// <summary>Anchor for selection mode (shift-like selection with arrow keys), or <c>null</c>.</summary>
    public int? SelectionAnchor { get; private set; }

    /// <summary>Whether selection mode is active.</summary>
    public bool IsSelecting => SelectionAnchor is not null;

    /// <summary>Full text.</summary>
    public string Text => Target.Text ?? string.Empty;

    /// <summary>Selection start.</summary>
    public int SelectionStart => Math.Clamp(Target.SelectionStart, 0, Text.Length);

    /// <summary>Selection length.</summary>
    public int SelectionLength => Math.Clamp(Target.SelectionLength, 0, Text.Length - SelectionStart);

    /// <summary>Selection end.</summary>
    public int SelectionEnd => SelectionStart + SelectionLength;

    /// <summary>Whether there is a non-empty selection.</summary>
    public bool HasSelection => SelectionLength > 0;

    /// <summary>The selected text.</summary>
    public string SelectedText => Text.Substring(SelectionStart, SelectionLength);

    /// <summary>Returns <c>true</c> if undo is possible.</summary>
    public bool CanUndo => _undo.Count > 0;

    /// <summary>Returns <c>true</c> if redo is possible.</summary>
    public bool CanRedo => _redo.Count > 0;

    /// <summary>Text before the caret (or selection start).</summary>
    public string GetTextBeforeCursor(int maxLength = int.MaxValue)
    {
        var start = SelectionStart;
        var from = Math.Max(0, start - maxLength);
        return Text.Substring(from, start - from);
    }

    /// <summary>Text after the caret (or selection end).</summary>
    public string GetTextAfterCursor(int maxLength = int.MaxValue)
    {
        var end = SelectionEnd;
        var len = Math.Min(maxLength, Text.Length - end);
        return Text.Substring(end, len);
    }

    /// <summary>The word directly before the caret (the word being composed).</summary>
    public string CurrentWord => HasSelection ? string.Empty : TextUtils.GetCurrentWord(GetTextBeforeCursor(64));

    /// <summary>Returns <c>true</c> if the caret is directly after a word character and not followed by one.</summary>
    public bool IsComposingWord => CurrentWord.Length > 0 && !StartsWithWordChar(GetTextAfterCursor(1));

    private static bool StartsWithWordChar(string s) => s.Length > 0 && TextUtils.IsWordCoreChar(s[0]);

    /// <summary>Inserts text, replacing the selection.</summary>
    public void CommitText(string text)
    {
        if (text.Length == 0 && !HasSelection)
        {
            return;
        }
        RecordUndo(text.Length > 0 && char.IsWhiteSpace(text[text.Length - 1]) ? EditKind.Separator : EditKind.Typing);
        Target.Replace(SelectionStart, SelectionLength, text);
        EndSelectionMode();
        _preferredColumn = null;
    }

    /// <summary>Deletes <paramref name="count"/> characters before the caret and inserts <paramref name="text"/>.</summary>
    public void ReplaceBeforeCursor(int count, string text)
    {
        var start = Math.Max(0, SelectionStart - count);
        RecordUndo(EditKind.Typing);
        Target.Replace(start, SelectionStart - start + SelectionLength, text);
        _preferredColumn = null;
    }

    /// <summary>Replaces the word before the caret (e.g. by a suggestion).</summary>
    public void ReplaceCurrentWord(string replacement)
    {
        var word = CurrentWord;
        RecordUndo(EditKind.Other);
        Target.Replace(SelectionStart - word.Length, word.Length, replacement);
        _preferredColumn = null;
    }

    /// <summary>Deletes the selection or the grapheme before the caret.</summary>
    public bool DeleteBackward()
    {
        if (HasSelection)
        {
            RecordUndo(EditKind.Delete);
            Target.Replace(SelectionStart, SelectionLength, string.Empty);
            EndSelectionMode();
            return true;
        }
        var before = GetTextBeforeCursor(32);
        if (before.Length == 0)
        {
            return false;
        }
        var len = TextUtils.LastGraphemeLength(before);
        RecordUndo(EditKind.Delete);
        Target.Replace(SelectionStart - len, len, string.Empty);
        _preferredColumn = null;
        return true;
    }

    /// <summary>Deletes the selection or the word before the caret.</summary>
    public bool DeleteWordBackward()
    {
        if (HasSelection)
        {
            return DeleteBackward();
        }
        var start = TextUtils.FindPreviousWordBoundary(Text, SelectionStart);
        if (start >= SelectionStart)
        {
            return false;
        }
        RecordUndo(EditKind.Other);
        Target.Replace(start, SelectionStart - start, string.Empty);
        _preferredColumn = null;
        return true;
    }

    /// <summary>Deletes the selection or the grapheme after the caret.</summary>
    public bool DeleteForward()
    {
        if (HasSelection)
        {
            return DeleteBackward();
        }
        var after = GetTextAfterCursor(32);
        if (after.Length == 0)
        {
            return false;
        }
        var len = TextUtils.FirstGraphemeLength(after);
        RecordUndo(EditKind.Delete);
        Target.Replace(SelectionStart, len, string.Empty);
        return true;
    }

    /// <summary>Deletes the selection or the word after the caret.</summary>
    public bool DeleteWordForward()
    {
        if (HasSelection)
        {
            return DeleteBackward();
        }
        var end = TextUtils.FindNextWordBoundary(Text, SelectionStart);
        if (end <= SelectionStart)
        {
            return false;
        }
        RecordUndo(EditKind.Other);
        Target.Replace(SelectionStart, end - SelectionStart, string.Empty);
        return true;
    }

    /// <summary>Deletes <paramref name="count"/> characters before the caret (used by precise delete gestures).</summary>
    public void DeleteCharactersBefore(int count)
    {
        var start = Math.Max(0, SelectionStart - count);
        if (start == SelectionStart)
        {
            return;
        }
        RecordUndo(EditKind.Other);
        Target.Replace(start, SelectionStart - start, string.Empty);
    }

    // ------------------------------------------------------------ caret movement

    /// <summary>Moves the caret horizontally by graphemes. Extends the selection in selection mode.</summary>
    public void MoveHorizontal(int delta)
    {
        _preferredColumn = null;
        var text = Text;
        var active = ActiveEnd;
        if (!IsSelecting && HasSelection)
        {
            // Collapse the selection in the direction of movement.
            var pos = delta < 0 ? SelectionStart : SelectionEnd;
            SetCaret(pos);
            return;
        }
        var target = active;
        for (var i = 0; i < Math.Abs(delta); i++)
        {
            if (delta < 0)
            {
                if (target == 0) break;
                target -= TextUtils.LastGraphemeLength(text.Substring(Math.Max(0, target - 16), Math.Min(16, target)));
            }
            else
            {
                if (target >= text.Length) break;
                target += TextUtils.FirstGraphemeLength(text.Substring(target, Math.Min(16, text.Length - target)));
            }
        }
        MoveActiveEndTo(target);
    }

    /// <summary>Moves the caret by words.</summary>
    public void MoveWord(int direction)
    {
        _preferredColumn = null;
        var pos = direction < 0 ? TextUtils.FindPreviousWordBoundary(Text, ActiveEnd) : TextUtils.FindNextWordBoundary(Text, ActiveEnd);
        MoveActiveEndTo(pos);
    }

    /// <summary>Moves the caret one line up (<c>-1</c>) or down (<c>+1</c>), keeping the column.</summary>
    public void MoveVertical(int direction)
    {
        var text = Text;
        var pos = ActiveEnd;
        var lineStart = LineStart(text, pos);
        var column = _preferredColumn ?? pos - lineStart;
        int target;
        if (direction < 0)
        {
            if (lineStart == 0)
            {
                target = 0;
            }
            else
            {
                var prevLineEnd = lineStart - 1;
                var prevLineStart = LineStart(text, prevLineEnd);
                target = Math.Min(prevLineStart + column, prevLineEnd);
            }
        }
        else
        {
            var lineEnd = LineEnd(text, pos);
            if (lineEnd >= text.Length)
            {
                target = text.Length;
            }
            else
            {
                var nextLineStart = lineEnd + 1;
                var nextLineEnd = LineEnd(text, nextLineStart);
                target = Math.Min(nextLineStart + column, nextLineEnd);
            }
        }
        MoveActiveEndTo(target);
        _preferredColumn = column;
    }

    /// <summary>Moves to the start of the current line.</summary>
    public void MoveLineStart() { _preferredColumn = null; MoveActiveEndTo(LineStart(Text, ActiveEnd)); }

    /// <summary>Moves to the end of the current line.</summary>
    public void MoveLineEnd() { _preferredColumn = null; MoveActiveEndTo(LineEnd(Text, ActiveEnd)); }

    /// <summary>Moves to the start of the text.</summary>
    public void MoveDocumentStart() { _preferredColumn = null; MoveActiveEndTo(0); }

    /// <summary>Moves to the end of the text.</summary>
    public void MoveDocumentEnd() { _preferredColumn = null; MoveActiveEndTo(Text.Length); }

    /// <summary>Places the caret.</summary>
    public void SetCaret(int position)
    {
        Target.Select(Math.Clamp(position, 0, Text.Length), 0);
    }

    /// <summary>Selects all text.</summary>
    public void SelectAll()
    {
        SelectionAnchor = null;
        Target.Select(0, Text.Length);
    }

    /// <summary>Toggles selection mode: arrow keys then extend the selection from the anchor.</summary>
    public void ToggleSelectionMode()
    {
        if (IsSelecting)
        {
            EndSelectionMode();
        }
        else
        {
            SelectionAnchor = HasSelection ? SelectionStart : SelectionStart;
        }
    }

    /// <summary>Leaves selection mode (keeps the current selection).</summary>
    public void EndSelectionMode() => SelectionAnchor = null;

    private int ActiveEnd
    {
        get
        {
            if (SelectionAnchor is { } anchor && HasSelection)
            {
                return anchor == SelectionStart ? SelectionEnd : SelectionStart;
            }
            return SelectionStart;
        }
    }

    private void MoveActiveEndTo(int position)
    {
        position = Math.Clamp(position, 0, Text.Length);
        if (SelectionAnchor is { } anchor)
        {
            anchor = Math.Clamp(anchor, 0, Text.Length);
            var start = Math.Min(anchor, position);
            Target.Select(start, Math.Abs(position - anchor));
        }
        else
        {
            Target.Select(position, 0);
        }
    }

    private static int LineStart(string text, int pos)
    {
        pos = Math.Clamp(pos, 0, text.Length);
        var idx = pos == 0 ? -1 : text.LastIndexOf('\n', pos - 1);
        return idx + 1;
    }

    private static int LineEnd(string text, int pos)
    {
        pos = Math.Clamp(pos, 0, text.Length);
        var idx = text.IndexOf('\n', pos);
        return idx < 0 ? text.Length : idx;
    }

    // ------------------------------------------------------------ undo / redo

    /// <summary>Starts a new undo group on the next edit.</summary>
    public void BreakUndoGroup() => _lastEditKind = EditKind.None;

    /// <summary>Reverts the last edit group.</summary>
    public bool Undo()
    {
        if (_undo.Count == 0)
        {
            return false;
        }
        var snapshot = _undo[_undo.Count - 1];
        _undo.RemoveAt(_undo.Count - 1);
        _redo.Add(TakeSnapshot());
        Restore(snapshot);
        _lastEditKind = EditKind.None;
        return true;
    }

    /// <summary>Re-applies the last undone edit group.</summary>
    public bool Redo()
    {
        if (_redo.Count == 0)
        {
            return false;
        }
        var snapshot = _redo[_redo.Count - 1];
        _redo.RemoveAt(_redo.Count - 1);
        _undo.Add(TakeSnapshot());
        Restore(snapshot);
        _lastEditKind = EditKind.None;
        return true;
    }

    /// <summary>Clears undo/redo history.</summary>
    public void ClearHistory()
    {
        _undo.Clear();
        _redo.Clear();
        _lastEditKind = EditKind.None;
    }

    private void RecordUndo(EditKind kind)
    {
        var now = Clock();
        var withinTimeout = now - _lastEditTime < UndoGroupTimeout;
        var continues = withinTimeout && (
            kind is EditKind.Typing or EditKind.Separator && _lastEditKind == EditKind.Typing ||
            kind == EditKind.Delete && _lastEditKind == EditKind.Delete);
        _lastEditTime = now;
        // A separator (space, new line) closes the current typing group.
        _lastEditKind = kind == EditKind.Separator ? EditKind.None : kind;
        if (continues)
        {
            return;
        }
        _undo.Add(TakeSnapshot());
        if (_undo.Count > MaxUndoSteps)
        {
            _undo.RemoveAt(0);
        }
        _redo.Clear();
    }

    private Snapshot TakeSnapshot() => new(Text, SelectionStart, SelectionLength);

    private void Restore(Snapshot snapshot)
    {
        Target.Replace(0, Text.Length, snapshot.Text);
        Target.Select(snapshot.SelectionStart, snapshot.SelectionLength);
    }

    // ------------------------------------------------------------ capitalization

    /// <summary>
    /// Returns <c>true</c> if the next character should be capitalized automatically according to
    /// <paramref name="mode"/> and the sentence terminators of <paramref name="rule"/>.
    /// </summary>
    public bool ShouldAutoCapitalize(CapitalizationMode mode, PunctuationRule rule)
    {
        if (mode == CapitalizationMode.None || HasSelection)
        {
            return false;
        }
        if (mode == CapitalizationMode.Characters)
        {
            return true;
        }
        var before = GetTextBeforeCursor(128);
        if (before.Length == 0)
        {
            return true;
        }
        var last = before[before.Length - 1];
        if (mode == CapitalizationMode.Words)
        {
            return char.IsWhiteSpace(last);
        }
        // Sentences
        if (last == '\n')
        {
            return true;
        }
        if (!char.IsWhiteSpace(last))
        {
            return false;
        }
        var trimmed = before.TrimEnd();
        if (trimmed.Length == 0)
        {
            return true;
        }
        // Skip closing quotes/brackets after a terminator ("Hello." or (Hello.) ).
        var i = trimmed.Length - 1;
        while (i > 0 && trimmed[i] is '"' or '\'' or ')' or ']' or '»' or '”' or '’')
        {
            i--;
        }
        return rule.SymbolsTerminatingSentence.IndexOf(trimmed[i]) >= 0;
    }

    private enum EditKind
    {
        None,
        Typing,
        Separator,
        Delete,
        Other,
    }

    private readonly record struct Snapshot(string Text, int SelectionStart, int SelectionLength);
}
