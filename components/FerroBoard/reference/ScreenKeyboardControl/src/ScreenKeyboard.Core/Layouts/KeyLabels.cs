using ScreenKeyboard.Editor;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Settings;

namespace ScreenKeyboard.Layouts;

/// <summary>Computes the display label or icon of a key.</summary>
public static class KeyLabels
{
    /// <summary>Returns the text label (or <c>null</c>) and icon for a key in the given context.</summary>
    public static (string? Label, KeyIcon Icon) GetDisplay(TextKeyData data, KeyComputeContext ctx)
    {
        switch (data.Code)
        {
            case KeyCode.Space when ctx.Mode is not (KeyboardMode.Characters or KeyboardMode.Symbols or KeyboardMode.Symbols2):
                return (null, KeyIcon.Space);
            case KeyCode.Space:
                return ctx.SpaceBarMode switch
                {
                    SpaceBarMode.CurrentLanguage => (ctx.SpaceBarLabel, KeyIcon.None),
                    SpaceBarMode.SpaceBarKey => (null, KeyIcon.Space),
                    _ => (string.Empty, KeyIcon.None),
                };
            case KeyCode.CjkSpace:
                return (ctx.SpaceBarLabel, KeyIcon.None);
            case KeyCode.Enter:
                return (null, GetEnterIcon(ctx.EnterAction));
            case KeyCode.Tab:
                return (null, KeyIcon.Tab);
            case KeyCode.Shift:
                return (null, ctx.ShiftState switch
                {
                    ShiftState.CapsLock => KeyIcon.CapsLock,
                    ShiftState.ShiftedManual or ShiftState.ShiftedAutomatic => KeyIcon.ShiftActive,
                    _ => KeyIcon.Shift,
                });
            case KeyCode.CapsLock:
                return (null, KeyIcon.CapsLock);
            case KeyCode.Delete:
            case KeyCode.DeleteWord:
                return (null, KeyIcon.Backspace);
            case KeyCode.ForwardDelete:
            case KeyCode.ForwardDeleteWord:
                return (null, KeyIcon.DeleteForward);
            case KeyCode.ViewCharacters:
                return ("ABC", KeyIcon.None);
            case KeyCode.ViewSymbols:
                return ("?123", KeyIcon.None);
            case KeyCode.ViewSymbols2:
                return ("=\\<", KeyIcon.None);
            case KeyCode.ViewNumeric:
            case KeyCode.ViewNumericAdvanced:
                return ("123", KeyIcon.None);
            case KeyCode.ViewPhone:
                return ("123", KeyIcon.None);
            case KeyCode.ViewPhone2:
                return ("*#(", KeyIcon.None);
            case KeyCode.LanguageSwitch:
            case KeyCode.ImeNextSubtype:
            case KeyCode.ImePrevSubtype:
            case KeyCode.ShowSubtypePicker:
            case KeyCode.ImeSubtypePicker:
                return (null, KeyIcon.Language);
            case KeyCode.ImeUiModeMedia:
                return (null, KeyIcon.Emoji);
            case KeyCode.ImeUiModeClipboard:
                return (null, KeyIcon.Clipboard);
            case KeyCode.ImeUiModeEditing:
                return (null, KeyIcon.TextEditing);
            case KeyCode.ImeUiModeText:
                return ("ABC", KeyIcon.None);
            case KeyCode.Settings:
                return (null, KeyIcon.Settings);
            case KeyCode.ImeHideUi:
                return (null, KeyIcon.HideKeyboard);
            case KeyCode.VoiceInput:
                return (null, KeyIcon.Microphone);
            case KeyCode.ArrowLeft:
                return (null, KeyIcon.ArrowLeft);
            case KeyCode.ArrowRight:
                return (null, KeyIcon.ArrowRight);
            case KeyCode.ArrowUp:
                return (null, KeyIcon.ArrowUp);
            case KeyCode.ArrowDown:
                return (null, KeyIcon.ArrowDown);
            case KeyCode.MoveStartOfLine:
                return (null, KeyIcon.Home);
            case KeyCode.MoveStartOfPage:
                return (null, KeyIcon.DocumentStart);
            case KeyCode.MoveEndOfLine:
                return (null, KeyIcon.End);
            case KeyCode.MoveEndOfPage:
                return (null, KeyIcon.DocumentEnd);
            case KeyCode.Undo:
                return (null, KeyIcon.Undo);
            case KeyCode.Redo:
                return (null, KeyIcon.Redo);
            case KeyCode.ClipboardCopy:
                return (null, KeyIcon.Copy);
            case KeyCode.ClipboardCut:
                return (null, KeyIcon.Cut);
            case KeyCode.ClipboardPaste:
                return (null, KeyIcon.Paste);
            case KeyCode.ClipboardSelect:
                return (null, KeyIcon.Select);
            case KeyCode.ClipboardSelectAll:
                return (null, KeyIcon.SelectAll);
            case KeyCode.CompactLayoutToLeft:
                return (null, KeyIcon.OneHandedLeft);
            case KeyCode.CompactLayoutToRight:
            case KeyCode.ToggleCompactLayout:
                return (null, KeyIcon.OneHandedRight);
            case KeyCode.SplitLayout:
            case KeyCode.MergeLayout:
                return (null, KeyIcon.SplitKeyboard);
            case KeyCode.ToggleIncognitoMode:
                return (null, KeyIcon.Incognito);
            case KeyCode.ToggleAutocorrect:
                return (null, KeyIcon.Autocorrect);
            case KeyCode.ToggleResizeMode:
                return (null, KeyIcon.ResizeHeight);
            case KeyCode.ToggleFloatingWindow:
                return (null, KeyIcon.Floating);
            case KeyCode.CharWidthSwitcher:
                return (ctx.IsCharHalfWidth ? "全" : "半", KeyIcon.None);
            case KeyCode.CharWidthFull:
                return ("全", KeyIcon.None);
            case KeyCode.CharWidthHalf:
                return ("半", KeyIcon.None);
            case KeyCode.KanaSwitcher:
                return (ctx.IsKanaKata ? "かな" : "カナ", KeyIcon.None);
            case KeyCode.KanaHira:
                return ("かな", KeyIcon.None);
            case KeyCode.KanaKata:
                return ("カナ", KeyIcon.None);
            case KeyCode.KanaHalfKata:
                return ("ｶﾅ", KeyIcon.None);
            case KeyCode.HalfSpace:
                return ("ZWNJ", KeyIcon.None);
            case KeyCode.KanaSmall:
                return ("小", KeyIcon.None);
        }

        if (data is MultiTextKeyData multi)
        {
            return (multi.Label.Length > 0 ? multi.Label : multi.AsString(true), KeyIcon.None);
        }
        if (KeyCode.IsCharacter(data.Code))
        {
            if (data.Type == KeyType.Numeric || data.Label.Length == 0 || TextUtils.GetCodePoints(data.Label).Length == 1)
            {
                return (data.AsString(true), KeyIcon.None);
            }
            // Layout authors can give character keys descriptive labels (e.g. telpad "pause").
            return (Capitalize(data.Label), KeyIcon.None);
        }
        return (data.Label, KeyIcon.None);
    }

    /// <summary>Letters printed under the digits of a phone pad.</summary>
    public static string? GetPhoneSubLabel(int code) => code switch
    {
        '0' => "+",
        '2' => "ABC",
        '3' => "DEF",
        '4' => "GHI",
        '5' => "JKL",
        '6' => "MNO",
        '7' => "PQRS",
        '8' => "TUV",
        '9' => "WXYZ",
        _ => null,
    };

    /// <summary>Icon for an enter action.</summary>
    public static KeyIcon GetEnterIcon(EnterAction action) => action switch
    {
        EnterAction.Go => KeyIcon.Go,
        EnterAction.Search => KeyIcon.Search,
        EnterAction.Send => KeyIcon.Send,
        EnterAction.Next => KeyIcon.Next,
        EnterAction.Previous => KeyIcon.Previous,
        EnterAction.Done => KeyIcon.Done,
        _ => KeyIcon.Enter,
    };

    private static string Capitalize(string label) =>
        label.Length == 0 ? label : char.ToUpperInvariant(label[0]) + label.Substring(1);
}
