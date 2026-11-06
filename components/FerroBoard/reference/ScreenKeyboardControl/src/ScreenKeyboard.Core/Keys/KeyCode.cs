namespace ScreenKeyboard.Keys;

/// <summary>
/// Well-known key codes. Positive values are Unicode code points, negative values are internal
/// function codes. The numeric values are compatible with FlorisBoard layout files.
/// </summary>
public static class KeyCode
{
    /// <summary>Minimum code point considered a character.</summary>
    public const int CharactersMin = 1;

    /// <summary>Maximum code point considered a character (full Unicode range).</summary>
    public const int CharactersMax = 0x10FFFF;

    /// <summary>Lower bound of internal (function) key codes.</summary>
    public const int InternalMin = -9999;

    /// <summary>Upper bound of internal (function) key codes.</summary>
    public const int InternalMax = -1;

    public const int Unspecified = 0;
    public const int PhoneWait = 59;
    public const int PhonePause = 44;
    public const int Space = 32;
    public const int Escape = 27;
    public const int Enter = 10;
    public const int Tab = 9;

    public const int Ctrl = -1;
    public const int CtrlLock = -2;
    public const int Alt = -3;
    public const int AltLock = -4;
    public const int Fn = -5;
    public const int FnLock = -6;
    public const int Delete = -7;
    public const int DeleteWord = -8;
    public const int ForwardDelete = -9;
    public const int ForwardDeleteWord = -10;
    public const int Shift = -11;
    public const int CapsLock = -13;

    public const int ArrowLeft = -21;
    public const int ArrowRight = -22;
    public const int ArrowUp = -23;
    public const int ArrowDown = -24;
    public const int MoveStartOfPage = -25;
    public const int MoveEndOfPage = -26;
    public const int MoveStartOfLine = -27;
    public const int MoveEndOfLine = -28;

    public const int ClipboardCopy = -31;
    public const int ClipboardCut = -32;
    public const int ClipboardPaste = -33;
    public const int ClipboardSelect = -34;
    public const int ClipboardSelectAll = -35;
    public const int ClipboardClearHistory = -36;
    public const int ClipboardClearFullHistory = -37;
    public const int ClipboardClearPrimaryClip = -38;

    public const int ToggleFloatingWindow = -109;
    public const int ToggleCompactLayout = -110;
    public const int CompactLayoutToLeft = -111;
    public const int CompactLayoutToRight = -112;
    public const int SplitLayout = -113;
    public const int MergeLayout = -114;
    public const int ToggleResizeMode = -115;

    public const int Undo = -131;
    public const int Redo = -132;

    public const int ViewCharacters = -201;
    public const int ViewSymbols = -202;
    public const int ViewSymbols2 = -203;
    public const int ViewNumeric = -204;
    public const int ViewNumericAdvanced = -205;
    public const int ViewPhone = -206;
    public const int ViewPhone2 = -207;

    public const int ImeUiModeText = -211;
    public const int ImeUiModeMedia = -212;
    public const int ImeUiModeClipboard = -213;
    /// <summary>Opens the text editing panel (arrow keys, selection, clipboard operations).</summary>
    public const int ImeUiModeEditing = -214;

    public const int SystemInputMethodPicker = -221;
    public const int SystemPrevInputMethod = -222;
    public const int SystemNextInputMethod = -223;
    public const int ImeSubtypePicker = -224;
    public const int ImePrevSubtype = -225;
    public const int ImeNextSubtype = -226;
    public const int LanguageSwitch = -227;
    public const int ShowSubtypePicker = -228;

    public const int ImeShowUi = -231;
    public const int ImeHideUi = -232;
    public const int VoiceInput = -233;

    public const int ToggleSmartbarVisibility = -241;
    public const int ToggleActionsOverflow = -242;
    public const int ToggleActionsEditor = -243;
    public const int ToggleIncognitoMode = -244;
    public const int ToggleAutocorrect = -245;

    public const int UriComponentTld = -255;
    public const int Settings = -301;

    public const int CurrencySlot1 = -801;
    public const int CurrencySlot2 = -802;
    public const int CurrencySlot3 = -803;
    public const int CurrencySlot4 = -804;
    public const int CurrencySlot5 = -805;
    public const int CurrencySlot6 = -806;

    public const int MultipleCodePoints = -902;
    public const int DragMarker = -991;
    public const int Noop = -999;

    public const int CharWidthSwitcher = -9701;
    public const int CharWidthFull = -9702;
    public const int CharWidthHalf = -9703;

    public const int KanaSmall = 12307;
    public const int KanaSwitcher = -9710;
    public const int KanaHira = -9711;
    public const int KanaKata = -9712;
    public const int KanaHalfKata = -9713;

    public const int Kashida = 1600;
    public const int HalfSpace = 8204;
    public const int CjkSpace = 12288;

    /// <summary>Returns <c>true</c> if <paramref name="code"/> is a Unicode code point.</summary>
    public static bool IsCharacter(int code) => code >= CharactersMin && code <= CharactersMax;

    /// <summary>Returns <c>true</c> if <paramref name="code"/> is an internal function code.</summary>
    public static bool IsInternal(int code) => code >= InternalMin && code <= InternalMax;

    /// <summary>Returns <c>true</c> for any of the six currency slot codes.</summary>
    public static bool IsCurrencySlot(int code) => code <= CurrencySlot1 && code >= CurrencySlot6;

    /// <summary>Returns <c>true</c> for space-like codes.</summary>
    public static bool IsSpace(int code) => code is Space or CjkSpace;

    private static readonly Dictionary<string, int> s_labelToCode = new(StringComparer.Ordinal)
    {
        ["ctrl"] = Ctrl, ["ctrl_lock"] = CtrlLock, ["alt"] = Alt, ["alt_lock"] = AltLock, ["fn"] = Fn, ["fn_lock"] = FnLock,
        ["delete"] = Delete, ["delete_word"] = DeleteWord, ["forward_delete"] = ForwardDelete, ["forward_delete_word"] = ForwardDeleteWord,
        ["shift"] = Shift, ["capslock"] = CapsLock, ["caps_lock"] = CapsLock,
        ["arrow_left"] = ArrowLeft, ["arrow_right"] = ArrowRight, ["arrow_up"] = ArrowUp, ["arrow_down"] = ArrowDown,
        ["move_start_of_page"] = MoveStartOfPage, ["move_end_of_page"] = MoveEndOfPage,
        ["move_start_of_line"] = MoveStartOfLine, ["move_end_of_line"] = MoveEndOfLine,
        ["clipboard_copy"] = ClipboardCopy, ["clipboard_cut"] = ClipboardCut, ["clipboard_paste"] = ClipboardPaste,
        ["clipboard_select"] = ClipboardSelect, ["clipboard_select_all"] = ClipboardSelectAll,
        ["clipboard_clear_history"] = ClipboardClearHistory, ["clipboard_clear_full_history"] = ClipboardClearFullHistory,
        ["clipboard_clear_primary_clip"] = ClipboardClearPrimaryClip,
        ["toggle_floating_window"] = ToggleFloatingWindow, ["toggle_compact_layout"] = ToggleCompactLayout,
        ["compact_layout_to_left"] = CompactLayoutToLeft, ["compact_layout_to_right"] = CompactLayoutToRight,
        ["split_layout"] = SplitLayout, ["merge_layout"] = MergeLayout, ["toggle_resize_mode"] = ToggleResizeMode,
        ["undo"] = Undo, ["redo"] = Redo,
        ["view_characters"] = ViewCharacters, ["view_symbols"] = ViewSymbols, ["view_symbols2"] = ViewSymbols2,
        ["view_numeric"] = ViewNumeric, ["view_numeric_advanced"] = ViewNumericAdvanced,
        ["view_phone"] = ViewPhone, ["view_phone2"] = ViewPhone2,
        ["ime_ui_mode_text"] = ImeUiModeText, ["ime_ui_mode_media"] = ImeUiModeMedia,
        ["ime_ui_mode_clipboard"] = ImeUiModeClipboard, ["ime_ui_mode_editing"] = ImeUiModeEditing,
        ["system_input_method_picker"] = SystemInputMethodPicker, ["system_prev_input_method"] = SystemPrevInputMethod,
        ["system_next_input_method"] = SystemNextInputMethod, ["ime_subtype_picker"] = ImeSubtypePicker,
        ["ime_prev_subtype"] = ImePrevSubtype, ["ime_next_subtype"] = ImeNextSubtype,
        ["language_switch"] = LanguageSwitch, ["show_subtype_picker"] = ShowSubtypePicker,
        ["ime_show_ui"] = ImeShowUi, ["ime_hide_ui"] = ImeHideUi, ["voice_input"] = VoiceInput,
        ["toggle_smartbar_visibility"] = ToggleSmartbarVisibility, ["toggle_actions_overflow"] = ToggleActionsOverflow,
        ["toggle_actions_editor"] = ToggleActionsEditor, ["toggle_incognito_mode"] = ToggleIncognitoMode,
        ["toggle_autocorrect"] = ToggleAutocorrect, ["settings"] = Settings,
        ["currency_slot_1"] = CurrencySlot1, ["currency_slot_2"] = CurrencySlot2, ["currency_slot_3"] = CurrencySlot3,
        ["currency_slot_4"] = CurrencySlot4, ["currency_slot_5"] = CurrencySlot5, ["currency_slot_6"] = CurrencySlot6,
        ["char_width_switcher"] = CharWidthSwitcher, ["char_width_full"] = CharWidthFull, ["char_width_half"] = CharWidthHalf,
        ["kana_switcher"] = KanaSwitcher, ["kana_hira"] = KanaHira, ["kana_kata"] = KanaKata, ["kana_half_kata"] = KanaHalfKata,
        ["noop"] = Noop,
    };

    private static readonly Dictionary<int, string> s_codeToName = BuildReverse();

    private static Dictionary<int, string> BuildReverse()
    {
        var map = new Dictionary<int, string>();
        foreach (var pair in s_labelToCode)
        {
            if (!map.ContainsKey(pair.Value))
            {
                map[pair.Value] = pair.Key;
            }
        }
        return map;
    }

    /// <summary>Resolves a FlorisBoard internal label (e.g. <c>"view_symbols"</c>) to its code.</summary>
    public static bool TryGetCode(string label, out int code) => s_labelToCode.TryGetValue(label, out code);

    /// <summary>Gets the canonical internal name of a function code, or <c>null</c>.</summary>
    public static string? GetName(int code) => s_codeToName.TryGetValue(code, out var name) ? name : null;
}
