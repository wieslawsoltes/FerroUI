// Generated from pinned upstream declarations by scripts/generate_settings.py.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct UtilityKeyAction(pub i32);
#[allow(non_upper_case_globals)]
impl UtilityKeyAction {
    pub const Emoji: Self = Self(0);
    pub const LanguageSwitch: Self = Self(1);
    pub const Dynamic: Self = Self(2);
    pub const Hidden: Self = Self(3);
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SpaceBarMode(pub i32);
#[allow(non_upper_case_globals)]
impl SpaceBarMode {
    pub const Nothing: Self = Self(0);
    pub const CurrentLanguage: Self = Self(1);
    pub const SpaceBarKey: Self = Self(2);
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct KeyHintMode(pub i32);
#[allow(non_upper_case_globals)]
impl KeyHintMode {
    pub const Disabled: Self = Self(0);
    pub const AccentPriority: Self = Self(1);
    pub const HintPriority: Self = Self(2);
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct OneHandedMode(pub i32);
#[allow(non_upper_case_globals)]
impl OneHandedMode {
    pub const Off: Self = Self(0);
    pub const Left: Self = Self(1);
    pub const Right: Self = Self(2);
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ThemeMode(pub i32);
#[allow(non_upper_case_globals)]
impl ThemeMode {
    pub const FollowSystem: Self = Self(0);
    pub const Light: Self = Self(1);
    pub const Dark: Self = Self(2);
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SwipeAction(pub i32);
#[allow(non_upper_case_globals)]
impl SwipeAction {
    pub const NoAction: Self = Self(0);
    pub const CycleToPreviousKeyboardMode: Self = Self(1);
    pub const CycleToNextKeyboardMode: Self = Self(2);
    pub const DeleteCharacter: Self = Self(3);
    pub const DeleteCharactersPrecisely: Self = Self(4);
    pub const DeleteWord: Self = Self(5);
    pub const DeleteWordsPrecisely: Self = Self(6);
    pub const HideKeyboard: Self = Self(7);
    pub const InsertSpace: Self = Self(8);
    pub const MoveCursorUp: Self = Self(9);
    pub const MoveCursorDown: Self = Self(10);
    pub const MoveCursorLeft: Self = Self(11);
    pub const MoveCursorRight: Self = Self(12);
    pub const MoveCursorStartOfLine: Self = Self(13);
    pub const MoveCursorEndOfLine: Self = Self(14);
    pub const MoveCursorStartOfPage: Self = Self(15);
    pub const MoveCursorEndOfPage: Self = Self(16);
    pub const Redo: Self = Self(17);
    pub const SelectCharactersPrecisely: Self = Self(18);
    pub const SelectWordsPrecisely: Self = Self(19);
    pub const Shift: Self = Self(20);
    pub const ShowSubtypePicker: Self = Self(21);
    pub const SwitchToPrevSubtype: Self = Self(22);
    pub const SwitchToNextSubtype: Self = Self(23);
    pub const SwitchToClipboardContext: Self = Self(24);
    pub const SwitchToMediaContext: Self = Self(25);
    pub const SwitchToEditingContext: Self = Self(26);
    pub const ToggleOneHandedMode: Self = Self(27);
    pub const ToggleSmartbarVisibility: Self = Self(28);
    pub const Undo: Self = Self(29);
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SwipeDirection(pub i32);
#[allow(non_upper_case_globals)]
impl SwipeDirection {
    pub const None: Self = Self(0);
    pub const Up: Self = Self(1);
    pub const Down: Self = Self(2);
    pub const Left: Self = Self(3);
    pub const Right: Self = Self(4);
    pub const UpLeft: Self = Self(5);
    pub const UpRight: Self = Self(6);
    pub const DownLeft: Self = Self(7);
    pub const DownRight: Self = Self(8);
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FeedbackKind(pub i32);
#[allow(non_upper_case_globals)]
impl FeedbackKind {
    pub const KeyPress: Self = Self(0);
    pub const KeyPressDelete: Self = Self(1);
    pub const KeyPressSpace: Self = Self(2);
    pub const KeyPressEnter: Self = Self(3);
    pub const KeyPressFunction: Self = Self(4);
    pub const LongPress: Self = Self(5);
    pub const KeyRepeat: Self = Self(6);
    pub const GestureStep: Self = Self(7);
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SmartbarLayout(pub i32);
#[allow(non_upper_case_globals)]
impl SmartbarLayout {
    pub const SuggestionsWithActions: Self = Self(0);
    pub const SuggestionsOnly: Self = Self(1);
    pub const ActionsOnly: Self = Self(2);
}
