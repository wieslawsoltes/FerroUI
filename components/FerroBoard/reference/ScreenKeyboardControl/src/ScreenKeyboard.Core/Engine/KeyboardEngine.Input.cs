using ScreenKeyboard.Editor;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Layouts;
using ScreenKeyboard.Settings;

namespace ScreenKeyboard.Engine;

public sealed partial class KeyboardEngine
{
    /// <summary>Simulates a full key press (down and up).</summary>
    public void InputKey(TextKeyData data)
    {
        OnKeyDown(data);
        OnKeyUp(data);
    }

    /// <summary>Types a string as if each character had been pressed.</summary>
    public void TypeText(string text)
    {
        foreach (var cp in TextUtils.GetCodePoints(text))
        {
            var key = cp switch
            {
                '\n' => TextKeyData.Function(KeyCode.Enter, KeyType.EnterEditing),
                ' ' => TextKeyData.SpaceKey,
                _ => AutoTextKeyData.CreateFromText(ApplyShift(char.ConvertFromUtf32(cp)), KeyType.Character, 0, null),
            };
            InputKey(key);
        }
    }

    private string ApplyShift(string text) =>
        _shiftState.IsUppercase() && text.Length > 0 && char.IsLower(text, 0) ? AutoTextKeyData.ToUpper(text, Culture) : text;

    /// <summary>Called when a key is pressed down.</summary>
    public void OnKeyDown(TextKeyData data)
    {
        switch (data.Code)
        {
            case KeyCode.Shift:
                HandleShiftDown();
                break;
            case KeyCode.CapsLock:
                SetShiftState(_shiftState == ShiftState.CapsLock ? ShiftState.Unshifted : ShiftState.CapsLock);
                break;
        }
    }

    /// <summary>Called when a key is released (this commits the key).</summary>
    public void OnKeyUp(TextKeyData data)
    {
        if (data.Code is not (KeyCode.Delete or KeyCode.Shift or KeyCode.CapsLock))
        {
            // Any key but delete invalidates the "undo auto-correction" opportunity.
            _lastAutoCorrection = null;
        }
        if (data.Code != KeyCode.Space)
        {
            _lastSpaceTime = DateTime.MinValue;
        }

        switch (data.Code)
        {
            case KeyCode.Shift:
                HandleShiftUp();
                return;
            case KeyCode.CapsLock:
                return;
            case KeyCode.Delete:
                Feedback(FeedbackKind.KeyPressDelete);
                HandleDelete();
                break;
            case KeyCode.DeleteWord:
                Feedback(FeedbackKind.KeyPressDelete);
                Edit(e => e.DeleteWordBackward());
                break;
            case KeyCode.ForwardDelete:
                Feedback(FeedbackKind.KeyPressDelete);
                Edit(e => e.DeleteForward());
                break;
            case KeyCode.ForwardDeleteWord:
                Feedback(FeedbackKind.KeyPressDelete);
                Edit(e => e.DeleteWordForward());
                break;
            case KeyCode.Enter:
                Feedback(FeedbackKind.KeyPressEnter);
                HandleEnter();
                break;
            case KeyCode.Space:
                Feedback(FeedbackKind.KeyPressSpace);
                HandleSpace();
                break;
            case KeyCode.CjkSpace:
                Feedback(FeedbackKind.KeyPressSpace);
                CommitRaw("　");
                break;
            case KeyCode.Tab:
                Feedback(FeedbackKind.KeyPressFunction);
                CommitRaw("\t");
                break;
            default:
                if (data.ProducesText)
                {
                    Feedback(FeedbackKind.KeyPress);
                    HandleCharacter(data.AsString(false));
                }
                else
                {
                    Feedback(FeedbackKind.KeyPressFunction);
                    HandleFunction(data);
                }
                break;
        }

        if (_shiftHeld && data.Code != KeyCode.Shift)
        {
            _keyPressedWhileShiftHeld = true;
        }
    }

    /// <summary>Called repeatedly while a repeatable key (delete, arrows) is held down.</summary>
    public void OnKeyRepeat(TextKeyData data)
    {
        Feedback(FeedbackKind.KeyRepeat);
        switch (data.Code)
        {
            case KeyCode.Delete:
                _lastAutoCorrection = null;
                Edit(e => e.DeleteBackward());
                break;
            case KeyCode.ForwardDelete:
                Edit(e => e.DeleteForward());
                break;
            case KeyCode.ArrowLeft:
            case KeyCode.ArrowRight:
            case KeyCode.ArrowUp:
            case KeyCode.ArrowDown:
                HandleFunction(data);
                break;
            case KeyCode.Space when !data.ProducesText:
                break;
            default:
                if (data.ProducesText && data.Code != KeyCode.Space)
                {
                    HandleCharacter(data.AsString(false));
                }
                break;
        }
    }

    /// <summary>Called when a key press was cancelled (e.g. finger moved to a popup or gesture).</summary>
    public void OnKeyCancel(TextKeyData data)
    {
        if (data.Code == KeyCode.Shift)
        {
            _shiftHeld = false;
        }
    }

    /// <summary>Returns <c>true</c> if the key repeats while held.</summary>
    public static bool IsRepeatable(TextKeyData data) => data.Code is KeyCode.Delete or KeyCode.ForwardDelete or
        KeyCode.ArrowLeft or KeyCode.ArrowRight or KeyCode.ArrowUp or KeyCode.ArrowDown;

    // ================================================================== shift

    private void HandleShiftDown()
    {
        var now = Clock();
        _shiftHeld = true;
        _keyPressedWhileShiftHeld = false;
        if (Settings.DoubleTapShiftForCapsLock && _shiftState is ShiftState.ShiftedManual &&
            now - _lastShiftUp < TimeSpan.FromMilliseconds(Settings.DoubleTapDelay))
        {
            SetShiftState(ShiftState.CapsLock);
        }
        else if (_shiftState == ShiftState.Unshifted)
        {
            SetShiftState(ShiftState.ShiftedManual);
        }
        else
        {
            SetShiftState(ShiftState.Unshifted);
        }
    }

    private bool _shiftDirty;

    private void HandleShiftUp()
    {
        _shiftHeld = false;
        _lastShiftUp = Clock();
        if (_keyPressedWhileShiftHeld && _shiftState != ShiftState.CapsLock)
        {
            // Shift was used as a chord modifier (held while typing).
            _lastShiftUp = DateTime.MinValue;
            SetShiftState(ShiftState.Unshifted);
            UpdateAutoCapitalization();
        }
        _keyPressedWhileShiftHeld = false;
    }

    private void AfterCharacterShift()
    {
        // A typed character releases a one-shot shift; the next UpdateAutoCapitalization re-evaluates it.
        if (_shiftState is ShiftState.ShiftedManual or ShiftState.ShiftedAutomatic && !_shiftHeld)
        {
            _shiftState = ShiftState.Unshifted;
            _shiftDirty = true;
        }
    }

    /// <summary>Recomputes the automatic shift state based on the text before the cursor.</summary>
    public void UpdateAutoCapitalization()
    {
        var newState = _shiftState;
        if (_editor is not null && _mode == KeyboardMode.Characters && _shiftState is not (ShiftState.CapsLock or ShiftState.ShiftedManual) && !_shiftHeld)
        {
            var shouldCap = Settings.AutoCapitalization && _editor.ShouldAutoCapitalize(Attributes.Capitalization, PunctuationRule) &&
                            Attributes.Kind is not (InputKind.Email or InputKind.Uri or InputKind.Password);
            newState = shouldCap ? ShiftState.ShiftedAutomatic : ShiftState.Unshifted;
        }
        if (newState != _shiftState || _shiftDirty)
        {
            _shiftState = newState;
            _shiftDirty = false;
            RecomputeKeyboard();
            OnStateChanged();
        }
    }

    // ================================================================== characters

    private void HandleCharacter(string text)
    {
        if (_editor is null || text.Length == 0)
        {
            return;
        }
        var rule = PunctuationRule;
        var isWordChar = TextUtils.IsWordCoreChar(text[0]);

        // Punctuation after a word: optionally auto-correct the word first.
        if (!isWordChar && !IsIntraWordPunctuation(text))
        {
            ApplyAutoCorrection(text);
            LearnCurrentWord();
        }

        // Swap the automatic space inserted after a suggestion with punctuation: "word ." → "word. "
        if (_lastInsertWasAutoSpace && rule.SymbolsPrecedingAutoSpace.Contains(text) && _editor.GetTextBeforeCursor(1) == " ")
        {
            WithoutTargetEvents(() =>
            {
                _editor.ReplaceBeforeCursor(1, text);
                if (Settings.AutoSpacePunctuation && !IsFollowedBySpace())
                {
                    _editor.CommitText(" ");
                    _lastInsertWasAutoSpace = true;
                }
                else
                {
                    _lastInsertWasAutoSpace = false;
                }
            });
            AfterCharacterShift();
            AfterEdit(text);
            return;
        }

        _lastInsertWasAutoSpace = false;
        var composer = Composer;
        WithoutTargetEvents(() =>
        {
            var preceding = composer.ToRead > 0 ? _editor.GetTextBeforeCursor(composer.ToRead) : string.Empty;
            var (delete, insert) = composer.GetActions(preceding, text);
            if (delete > 0)
            {
                _editor.ReplaceBeforeCursor(delete, insert);
            }
            else if (insert.Length > 0)
            {
                _editor.CommitText(insert);
            }
        });

        // Leave symbols after typing a closing punctuation followed by space is handled in HandleSpace.
        AfterCharacterShift();
        AfterEdit(text);
    }

    private static bool IsIntraWordPunctuation(string text) => text is "'" or "’" or "-" or "_";

    private bool IsFollowedBySpace()
    {
        var after = _editor?.GetTextAfterCursor(1) ?? string.Empty;
        return after.Length > 0 && char.IsWhiteSpace(after[0]);
    }

    private void HandleSpace()
    {
        if (_editor is null)
        {
            return;
        }
        var now = Clock();
        var before = _editor.GetTextBeforeCursor(3);

        // Double space → ". "
        if (Settings.DoubleSpacePeriod && Attributes.SupportsNlp && now - _lastSpaceTime < DoubleSpaceWindow &&
            before.Length >= 2 && before[before.Length - 1] == ' ' && TextUtils.IsWordCoreChar(before[before.Length - 2]))
        {
            WithoutTargetEvents(() => _editor.ReplaceBeforeCursor(1, ". "));
            _lastSpaceTime = DateTime.MinValue;
            _lastInsertWasAutoSpace = true;
            AfterEdit(". ");
            UpdateAutoCapitalization();
            return;
        }

        ApplyAutoCorrection(" ");
        LearnCurrentWord();
        _lastInsertWasAutoSpace = false;
        WithoutTargetEvents(() => _editor.CommitText(" "));
        _lastSpaceTime = now;
        if (_mode is KeyboardMode.Symbols or KeyboardMode.Symbols2)
        {
            SetMode(KeyboardMode.Characters);
        }
        AfterCharacterShift();
        AfterEdit(" ");
    }

    private void HandleEnter()
    {
        if (_editor is null || _target is null)
        {
            return;
        }
        FinishGlide(commit: false);
        ApplyAutoCorrection("\n");
        LearnCurrentWord();
        _lastInsertWasAutoSpace = false;
        var action = Attributes.EffectiveEnterAction;
        if (action == EnterAction.NewLine && Attributes.IsMultiline)
        {
            WithoutTargetEvents(() => _editor.CommitText("\n"));
            AfterEdit("\n");
            return;
        }
        var handled = _target.PerformEnterAction(action);
        if (!handled && Attributes.IsMultiline)
        {
            WithoutTargetEvents(() => _editor.CommitText("\n"));
            AfterEdit("\n");
        }
        else
        {
            UpdateSuggestions();
        }
    }

    private void HandleDelete()
    {
        if (_editor is null)
        {
            return;
        }
        FinishGlide(commit: false);
        if (_lastAutoCorrection is { } ac && Settings.UndoAutoCorrectOnBackspace && !_editor.HasSelection)
        {
            var expected = ac.Corrected + ac.Separator;
            var before = _editor.GetTextBeforeCursor(expected.Length);
            if (before == expected && _editor.SelectionStart == ac.End)
            {
                WithoutTargetEvents(() => _editor.ReplaceBeforeCursor(expected.Length, ac.Original));
                _rejectedCorrections.Add(ac.Original);
                _lastAutoCorrection = null;
                AfterEdit(null);
                return;
            }
        }
        _lastAutoCorrection = null;
        if (_lastInsertWasAutoSpace)
        {
            _lastInsertWasAutoSpace = false;
        }
        Edit(e => e.DeleteBackward());
    }

    private void CommitRaw(string text)
    {
        if (_editor is null)
        {
            return;
        }
        WithoutTargetEvents(() => _editor.CommitText(text));
        AfterEdit(text);
    }

    /// <summary>Commits arbitrary text (e.g. an emoji or a clipboard item) at the cursor.</summary>
    public void InputText(string text)
    {
        if (_editor is null || text.Length == 0)
        {
            return;
        }
        _lastAutoCorrection = null;
        _lastInsertWasAutoSpace = false;
        WithoutTargetEvents(() => _editor.CommitText(text));
        AfterEdit(text);
    }

    /// <summary>Inputs an emoji and records it in the history.</summary>
    public void InputEmoji(string emoji)
    {
        Feedback(FeedbackKind.KeyPress);
        InputText(emoji);
        if (!IsIncognito && Settings.EmojiHistoryEnabled)
        {
            EmojiHistory.Add(emoji);
        }
    }

    private void Edit(Func<EditorController, bool> action)
    {
        if (_editor is null)
        {
            return;
        }
        var changed = false;
        WithoutTargetEvents(() => changed = action(_editor));
        if (changed)
        {
            AfterEdit(null);
        }
    }

    private void AfterEdit(string? committed)
    {
        if (committed is not null)
        {
            TextCommitted?.Invoke(this, committed);
        }
        UpdateAutoCapitalization();
        UpdateSuggestions();
    }

    private void WithoutTargetEvents(Action action)
    {
        var previous = _suppressTargetChanged;
        _suppressTargetChanged = true;
        try
        {
            action();
        }
        finally
        {
            _suppressTargetChanged = previous;
        }
    }

    // ================================================================== functions

    private void HandleFunction(TextKeyData data)
    {
        var editor = _editor;
        switch (data.Code)
        {
            case KeyCode.ViewCharacters: SetMode(KeyboardMode.Characters); break;
            case KeyCode.ViewSymbols: SetMode(KeyboardMode.Symbols); break;
            case KeyCode.ViewSymbols2: SetMode(KeyboardMode.Symbols2); break;
            case KeyCode.ViewNumeric: SetMode(KeyboardMode.Numeric); break;
            case KeyCode.ViewNumericAdvanced: SetMode(KeyboardMode.NumericAdvanced); break;
            case KeyCode.ViewPhone: SetMode(KeyboardMode.Phone); break;
            case KeyCode.ViewPhone2: SetMode(KeyboardMode.Phone2); break;

            case KeyCode.ImeUiModeText: SetUiMode(KeyboardUiMode.Text); break;
            case KeyCode.ImeUiModeMedia: SetUiMode(KeyboardUiMode.Media); break;
            case KeyCode.ImeUiModeClipboard: SetUiMode(KeyboardUiMode.Clipboard); break;
            case KeyCode.ImeUiModeEditing: SetUiMode(KeyboardUiMode.Editing); break;

            case KeyCode.LanguageSwitch:
            case KeyCode.ImeNextSubtype:
            case KeyCode.SystemNextInputMethod:
                NextSubtype();
                break;
            case KeyCode.ImePrevSubtype:
            case KeyCode.SystemPrevInputMethod:
                PreviousSubtype();
                break;
            case KeyCode.ShowSubtypePicker:
            case KeyCode.ImeSubtypePicker:
            case KeyCode.SystemInputMethodPicker:
                SubtypePickerRequested?.Invoke(this, EventArgs.Empty);
                break;
            case KeyCode.Settings:
                SettingsRequested?.Invoke(this, EventArgs.Empty);
                break;
            case KeyCode.ImeHideUi:
                HideRequested?.Invoke(this, EventArgs.Empty);
                break;
            case KeyCode.VoiceInput:
                VoiceInputRequested?.Invoke(this, EventArgs.Empty);
                break;

            case KeyCode.ArrowLeft: Navigate(e => e.MoveHorizontal(-1)); break;
            case KeyCode.ArrowRight: Navigate(e => e.MoveHorizontal(1)); break;
            case KeyCode.ArrowUp: Navigate(e => e.MoveVertical(-1)); break;
            case KeyCode.ArrowDown: Navigate(e => e.MoveVertical(1)); break;
            case KeyCode.MoveStartOfLine: Navigate(e => e.MoveLineStart()); break;
            case KeyCode.MoveEndOfLine: Navigate(e => e.MoveLineEnd()); break;
            case KeyCode.MoveStartOfPage: Navigate(e => e.MoveDocumentStart()); break;
            case KeyCode.MoveEndOfPage: Navigate(e => e.MoveDocumentEnd()); break;

            case KeyCode.ClipboardCopy: _ = CopyAsync(); break;
            case KeyCode.ClipboardCut: _ = CutAsync(); break;
            case KeyCode.ClipboardPaste: _ = PasteAsync(); break;
            case KeyCode.ClipboardSelect:
                editor?.ToggleSelectionMode();
                OnStateChanged();
                break;
            case KeyCode.ClipboardSelectAll:
                Navigate(e => e.SelectAll());
                break;
            case KeyCode.ClipboardClearHistory: ClipboardHistory.Clear(); break;
            case KeyCode.ClipboardClearFullHistory: ClipboardHistory.Clear(includePinned: true); break;
            case KeyCode.ClipboardClearPrimaryClip:
                if (ClipboardHistory.Primary is { } primary) ClipboardHistory.Remove(primary);
                break;

            case KeyCode.Undo: Edit(e => e.Undo()); break;
            case KeyCode.Redo: Edit(e => e.Redo()); break;

            case KeyCode.CompactLayoutToLeft:
                Settings.OneHandedMode = OneHandedMode.Left;
                break;
            case KeyCode.CompactLayoutToRight:
                Settings.OneHandedMode = OneHandedMode.Right;
                break;
            case KeyCode.ToggleCompactLayout:
                Settings.OneHandedMode = Settings.OneHandedMode == OneHandedMode.Off ? OneHandedMode.Right : OneHandedMode.Off;
                break;
            case KeyCode.ToggleFloatingWindow: Settings.Floating = !Settings.Floating; break;
            case KeyCode.ToggleResizeMode: ResizeModeRequested?.Invoke(this, EventArgs.Empty); break;
            case KeyCode.SplitLayout: Settings.SplitKeyboard = true; break;
            case KeyCode.MergeLayout: Settings.SplitKeyboard = false; break;
            case KeyCode.ToggleIncognitoMode: Settings.IncognitoMode = !Settings.IncognitoMode; break;
            case KeyCode.ToggleAutocorrect: Settings.AutoCorrect = !Settings.AutoCorrect; break;
            case KeyCode.ToggleSmartbarVisibility: Settings.SmartbarEnabled = !Settings.SmartbarEnabled; break;

            case KeyCode.CharWidthSwitcher:
                _isCharHalfWidth = !_isCharHalfWidth;
                RecomputeKeyboard();
                OnStateChanged();
                break;
            case KeyCode.CharWidthFull:
                _isCharHalfWidth = false;
                RecomputeKeyboard();
                OnStateChanged();
                break;
            case KeyCode.CharWidthHalf:
                _isCharHalfWidth = true;
                RecomputeKeyboard();
                OnStateChanged();
                break;
            case KeyCode.KanaSwitcher:
                _isKanaKata = !_isKanaKata;
                RecomputeKeyboard();
                OnStateChanged();
                break;
            case KeyCode.KanaHira:
                _isKanaKata = false;
                RecomputeKeyboard();
                OnStateChanged();
                break;
            case KeyCode.KanaKata:
            case KeyCode.KanaHalfKata:
                _isKanaKata = true;
                RecomputeKeyboard();
                OnStateChanged();
                break;
        }
    }

    private void Navigate(Action<EditorController> action)
    {
        if (_editor is null)
        {
            return;
        }
        FinishGlide(commit: false);
        _lastAutoCorrection = null;
        _lastInsertWasAutoSpace = false;
        WithoutTargetEvents(() => action(_editor));
        _editor.BreakUndoGroup();
        UpdateAutoCapitalization();
        UpdateSuggestions();
    }

    /// <summary>Moves the caret horizontally by <paramref name="steps"/> graphemes (negative = left).</summary>
    public void MoveCursor(int steps) => Navigate(e => e.MoveHorizontal(steps));

    // ================================================================== clipboard

    /// <summary>Copies the selection to the clipboard (and history).</summary>
    public async Task CopyAsync()
    {
        var text = _editor?.SelectedText;
        if (string.IsNullOrEmpty(text))
        {
            return;
        }
        await ClipboardService.SetTextAsync(text!).ConfigureAwait(true);
        RecordClipboard(text!);
    }

    /// <summary>Cuts the selection to the clipboard (and history).</summary>
    public async Task CutAsync()
    {
        var text = _editor?.SelectedText;
        if (string.IsNullOrEmpty(text))
        {
            return;
        }
        await ClipboardService.SetTextAsync(text!).ConfigureAwait(true);
        RecordClipboard(text!);
        Edit(e => e.DeleteBackward());
    }

    /// <summary>Pastes the clipboard content.</summary>
    public async Task PasteAsync()
    {
        var text = await ClipboardService.GetTextAsync().ConfigureAwait(true);
        if (string.IsNullOrEmpty(text))
        {
            text = ClipboardHistory.Primary?.Text;
        }
        if (!string.IsNullOrEmpty(text))
        {
            InputText(text!);
        }
    }

    /// <summary>Records text in the clipboard history (unless incognito or disabled).</summary>
    public void RecordClipboard(string text)
    {
        if (Settings.ClipboardHistoryEnabled && !IsIncognito)
        {
            ClipboardHistory.Add(text);
        }
    }

    // ================================================================== swipe actions

    /// <summary>Executes a configurable swipe action.</summary>
    public void ExecuteSwipeAction(SwipeAction action, int count = 1)
    {
        switch (action)
        {
            case SwipeAction.NoAction:
                return;
            case SwipeAction.CycleToPreviousKeyboardMode:
                SetMode(_mode switch
                {
                    KeyboardMode.Characters => KeyboardMode.Numeric,
                    KeyboardMode.Symbols => KeyboardMode.Characters,
                    KeyboardMode.Symbols2 => KeyboardMode.Symbols,
                    _ => KeyboardMode.Characters,
                });
                break;
            case SwipeAction.CycleToNextKeyboardMode:
                SetMode(_mode switch
                {
                    KeyboardMode.Characters => KeyboardMode.Symbols,
                    KeyboardMode.Symbols => KeyboardMode.Symbols2,
                    KeyboardMode.Symbols2 => KeyboardMode.Characters,
                    _ => KeyboardMode.Characters,
                });
                break;
            case SwipeAction.DeleteCharacter:
                for (var i = 0; i < count; i++) Edit(e => e.DeleteBackward());
                break;
            case SwipeAction.DeleteWord:
                for (var i = 0; i < count; i++) Edit(e => e.DeleteWordBackward());
                break;
            case SwipeAction.DeleteCharactersPrecisely:
            case SwipeAction.DeleteWordsPrecisely:
            case SwipeAction.SelectCharactersPrecisely:
            case SwipeAction.SelectWordsPrecisely:
                // Handled by the precise selection API (BeginPreciseSelection/UpdatePreciseSelection).
                break;
            case SwipeAction.HideKeyboard:
                HideRequested?.Invoke(this, EventArgs.Empty);
                break;
            case SwipeAction.InsertSpace:
                HandleSpace();
                break;
            case SwipeAction.MoveCursorUp:
                for (var i = 0; i < count; i++) Navigate(e => e.MoveVertical(-1));
                break;
            case SwipeAction.MoveCursorDown:
                for (var i = 0; i < count; i++) Navigate(e => e.MoveVertical(1));
                break;
            case SwipeAction.MoveCursorLeft:
                MoveCursor(-count);
                break;
            case SwipeAction.MoveCursorRight:
                MoveCursor(count);
                break;
            case SwipeAction.MoveCursorStartOfLine:
                Navigate(e => e.MoveLineStart());
                break;
            case SwipeAction.MoveCursorEndOfLine:
                Navigate(e => e.MoveLineEnd());
                break;
            case SwipeAction.MoveCursorStartOfPage:
                Navigate(e => e.MoveDocumentStart());
                break;
            case SwipeAction.MoveCursorEndOfPage:
                Navigate(e => e.MoveDocumentEnd());
                break;
            case SwipeAction.Redo:
                Edit(e => e.Redo());
                break;
            case SwipeAction.Undo:
                Edit(e => e.Undo());
                break;
            case SwipeAction.Shift:
                if (_mode == KeyboardMode.Characters)
                {
                    SetShiftState(_shiftState == ShiftState.Unshifted || _shiftState == ShiftState.ShiftedAutomatic ? ShiftState.ShiftedManual : ShiftState.Unshifted);
                }
                break;
            case SwipeAction.ShowSubtypePicker:
                SubtypePickerRequested?.Invoke(this, EventArgs.Empty);
                break;
            case SwipeAction.SwitchToPrevSubtype:
                PreviousSubtype();
                break;
            case SwipeAction.SwitchToNextSubtype:
                NextSubtype();
                break;
            case SwipeAction.SwitchToClipboardContext:
                SetUiMode(KeyboardUiMode.Clipboard);
                break;
            case SwipeAction.SwitchToMediaContext:
                SetUiMode(KeyboardUiMode.Media);
                break;
            case SwipeAction.SwitchToEditingContext:
                SetUiMode(KeyboardUiMode.Editing);
                break;
            case SwipeAction.ToggleOneHandedMode:
                Settings.OneHandedMode = Settings.OneHandedMode == OneHandedMode.Off ? OneHandedMode.Right : OneHandedMode.Off;
                break;
            case SwipeAction.ToggleSmartbarVisibility:
                Settings.SmartbarEnabled = !Settings.SmartbarEnabled;
                break;
        }
        Feedback(FeedbackKind.GestureStep);
    }

    /// <summary>Starts a precise selection gesture (e.g. swipe left on delete).</summary>
    public void BeginPreciseSelection()
    {
        if (_editor is null)
        {
            return;
        }
        _preciseDeleteOrigin = _editor.SelectionEnd;
    }

    /// <summary>
    /// Updates a precise selection: selects <paramref name="steps"/> characters (or words) before the origin.
    /// </summary>
    public void UpdatePreciseSelection(int steps, bool byWords)
    {
        if (_editor is null || _preciseDeleteOrigin is not { } origin)
        {
            return;
        }
        var text = _editor.Text;
        origin = Math.Min(origin, text.Length);
        var start = origin;
        for (var i = 0; i < Math.Max(0, steps); i++)
        {
            if (start == 0)
            {
                break;
            }
            start = byWords ? TextUtils.FindPreviousWordBoundary(text, start) : start - TextUtils.LastGraphemeLength(text.Substring(Math.Max(0, start - 16), Math.Min(16, start)));
        }
        WithoutTargetEvents(() => _editor.Target.Select(start, origin - start));
        Feedback(FeedbackKind.GestureStep);
    }

    /// <summary>Ends a precise selection: deletes the selection (<paramref name="delete"/>) or keeps it.</summary>
    public void EndPreciseSelection(bool delete)
    {
        if (_editor is null || _preciseDeleteOrigin is null)
        {
            return;
        }
        _preciseDeleteOrigin = null;
        if (delete && _editor.HasSelection)
        {
            Edit(e => e.DeleteBackward());
        }
        else
        {
            UpdateSuggestions();
        }
    }

    /// <summary>Cancels a precise selection, restoring the caret.</summary>
    public void CancelPreciseSelection()
    {
        if (_editor is null || _preciseDeleteOrigin is not { } origin)
        {
            return;
        }
        _preciseDeleteOrigin = null;
        WithoutTargetEvents(() => _editor.SetCaret(origin));
    }
}
