// Generated from original Core constants. No replacement enum values.
use ferroui_base::{
    data::core::ValueType,
    metadata::{MarkupEnumMember, MarkupType, MarkupTypeKind, MarkupTyped},
};
#[doc(hidden)]
pub struct UtilityKeyActionMetadata;
#[allow(non_upper_case_globals)]
impl UtilityKeyActionMetadata {
    pub const Emoji: ferroboard_core::settings::UtilityKeyAction =
        ferroboard_core::settings::UtilityKeyAction::Emoji;
    pub const LanguageSwitch: ferroboard_core::settings::UtilityKeyAction =
        ferroboard_core::settings::UtilityKeyAction::LanguageSwitch;
    pub const Dynamic: ferroboard_core::settings::UtilityKeyAction =
        ferroboard_core::settings::UtilityKeyAction::Dynamic;
    pub const Hidden: ferroboard_core::settings::UtilityKeyAction =
        ferroboard_core::settings::UtilityKeyAction::Hidden;
}
const _: () = assert!(ferroboard_core::settings::UtilityKeyAction::Emoji.0 == 0);
const _: () = assert!(ferroboard_core::settings::UtilityKeyAction::LanguageSwitch.0 == 1);
const _: () = assert!(ferroboard_core::settings::UtilityKeyAction::Dynamic.0 == 2);
const _: () = assert!(ferroboard_core::settings::UtilityKeyAction::Hidden.0 == 3);
impl MarkupTyped for UtilityKeyActionMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("UtilityKeyAction", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Settings");
            m.handles = &[ValueType::of::<ferroboard_core::settings::UtilityKeyAction>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::settings::UtilityKeyAction>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Emoji",
                    value: 0,
                    rust_variant: Some("Emoji"),
                },
                MarkupEnumMember {
                    name: "LanguageSwitch",
                    value: 1,
                    rust_variant: Some("LanguageSwitch"),
                },
                MarkupEnumMember {
                    name: "Dynamic",
                    value: 2,
                    rust_variant: Some("Dynamic"),
                },
                MarkupEnumMember {
                    name: "Hidden",
                    value: 3,
                    rust_variant: Some("Hidden"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::settings::UtilityKeyAction(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::settings::UtilityKeyAction>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct SpaceBarModeMetadata;
#[allow(non_upper_case_globals)]
impl SpaceBarModeMetadata {
    pub const Nothing: ferroboard_core::settings::SpaceBarMode =
        ferroboard_core::settings::SpaceBarMode::Nothing;
    pub const CurrentLanguage: ferroboard_core::settings::SpaceBarMode =
        ferroboard_core::settings::SpaceBarMode::CurrentLanguage;
    pub const SpaceBarKey: ferroboard_core::settings::SpaceBarMode =
        ferroboard_core::settings::SpaceBarMode::SpaceBarKey;
}
const _: () = assert!(ferroboard_core::settings::SpaceBarMode::Nothing.0 == 0);
const _: () = assert!(ferroboard_core::settings::SpaceBarMode::CurrentLanguage.0 == 1);
const _: () = assert!(ferroboard_core::settings::SpaceBarMode::SpaceBarKey.0 == 2);
impl MarkupTyped for SpaceBarModeMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("SpaceBarMode", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Settings");
            m.handles = &[ValueType::of::<ferroboard_core::settings::SpaceBarMode>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::settings::SpaceBarMode>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Nothing",
                    value: 0,
                    rust_variant: Some("Nothing"),
                },
                MarkupEnumMember {
                    name: "CurrentLanguage",
                    value: 1,
                    rust_variant: Some("CurrentLanguage"),
                },
                MarkupEnumMember {
                    name: "SpaceBarKey",
                    value: 2,
                    rust_variant: Some("SpaceBarKey"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::settings::SpaceBarMode(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::settings::SpaceBarMode>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct KeyHintModeMetadata;
#[allow(non_upper_case_globals)]
impl KeyHintModeMetadata {
    pub const Disabled: ferroboard_core::settings::KeyHintMode =
        ferroboard_core::settings::KeyHintMode::Disabled;
    pub const AccentPriority: ferroboard_core::settings::KeyHintMode =
        ferroboard_core::settings::KeyHintMode::AccentPriority;
    pub const HintPriority: ferroboard_core::settings::KeyHintMode =
        ferroboard_core::settings::KeyHintMode::HintPriority;
}
const _: () = assert!(ferroboard_core::settings::KeyHintMode::Disabled.0 == 0);
const _: () = assert!(ferroboard_core::settings::KeyHintMode::AccentPriority.0 == 1);
const _: () = assert!(ferroboard_core::settings::KeyHintMode::HintPriority.0 == 2);
impl MarkupTyped for KeyHintModeMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("KeyHintMode", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Settings");
            m.handles = &[ValueType::of::<ferroboard_core::settings::KeyHintMode>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::settings::KeyHintMode>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Disabled",
                    value: 0,
                    rust_variant: Some("Disabled"),
                },
                MarkupEnumMember {
                    name: "AccentPriority",
                    value: 1,
                    rust_variant: Some("AccentPriority"),
                },
                MarkupEnumMember {
                    name: "HintPriority",
                    value: 2,
                    rust_variant: Some("HintPriority"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::settings::KeyHintMode(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::settings::KeyHintMode>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct OneHandedModeMetadata;
#[allow(non_upper_case_globals)]
impl OneHandedModeMetadata {
    pub const Off: ferroboard_core::settings::OneHandedMode =
        ferroboard_core::settings::OneHandedMode::Off;
    pub const Left: ferroboard_core::settings::OneHandedMode =
        ferroboard_core::settings::OneHandedMode::Left;
    pub const Right: ferroboard_core::settings::OneHandedMode =
        ferroboard_core::settings::OneHandedMode::Right;
}
const _: () = assert!(ferroboard_core::settings::OneHandedMode::Off.0 == 0);
const _: () = assert!(ferroboard_core::settings::OneHandedMode::Left.0 == 1);
const _: () = assert!(ferroboard_core::settings::OneHandedMode::Right.0 == 2);
impl MarkupTyped for OneHandedModeMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("OneHandedMode", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Settings");
            m.handles = &[ValueType::of::<ferroboard_core::settings::OneHandedMode>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::settings::OneHandedMode>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Off",
                    value: 0,
                    rust_variant: Some("Off"),
                },
                MarkupEnumMember {
                    name: "Left",
                    value: 1,
                    rust_variant: Some("Left"),
                },
                MarkupEnumMember {
                    name: "Right",
                    value: 2,
                    rust_variant: Some("Right"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::settings::OneHandedMode(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::settings::OneHandedMode>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct ThemeModeMetadata;
#[allow(non_upper_case_globals)]
impl ThemeModeMetadata {
    pub const FollowSystem: ferroboard_core::settings::ThemeMode =
        ferroboard_core::settings::ThemeMode::FollowSystem;
    pub const Light: ferroboard_core::settings::ThemeMode =
        ferroboard_core::settings::ThemeMode::Light;
    pub const Dark: ferroboard_core::settings::ThemeMode =
        ferroboard_core::settings::ThemeMode::Dark;
}
const _: () = assert!(ferroboard_core::settings::ThemeMode::FollowSystem.0 == 0);
const _: () = assert!(ferroboard_core::settings::ThemeMode::Light.0 == 1);
const _: () = assert!(ferroboard_core::settings::ThemeMode::Dark.0 == 2);
impl MarkupTyped for ThemeModeMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("ThemeMode", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Settings");
            m.handles = &[ValueType::of::<ferroboard_core::settings::ThemeMode>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::settings::ThemeMode>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "FollowSystem",
                    value: 0,
                    rust_variant: Some("FollowSystem"),
                },
                MarkupEnumMember {
                    name: "Light",
                    value: 1,
                    rust_variant: Some("Light"),
                },
                MarkupEnumMember {
                    name: "Dark",
                    value: 2,
                    rust_variant: Some("Dark"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::settings::ThemeMode(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::settings::ThemeMode>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct SwipeActionMetadata;
#[allow(non_upper_case_globals)]
impl SwipeActionMetadata {
    pub const NoAction: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::NoAction;
    pub const CycleToPreviousKeyboardMode: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::CycleToPreviousKeyboardMode;
    pub const CycleToNextKeyboardMode: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::CycleToNextKeyboardMode;
    pub const DeleteCharacter: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::DeleteCharacter;
    pub const DeleteCharactersPrecisely: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::DeleteCharactersPrecisely;
    pub const DeleteWord: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::DeleteWord;
    pub const DeleteWordsPrecisely: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::DeleteWordsPrecisely;
    pub const HideKeyboard: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::HideKeyboard;
    pub const InsertSpace: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::InsertSpace;
    pub const MoveCursorUp: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::MoveCursorUp;
    pub const MoveCursorDown: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::MoveCursorDown;
    pub const MoveCursorLeft: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::MoveCursorLeft;
    pub const MoveCursorRight: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::MoveCursorRight;
    pub const MoveCursorStartOfLine: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::MoveCursorStartOfLine;
    pub const MoveCursorEndOfLine: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::MoveCursorEndOfLine;
    pub const MoveCursorStartOfPage: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::MoveCursorStartOfPage;
    pub const MoveCursorEndOfPage: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::MoveCursorEndOfPage;
    pub const Redo: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::Redo;
    pub const SelectCharactersPrecisely: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::SelectCharactersPrecisely;
    pub const SelectWordsPrecisely: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::SelectWordsPrecisely;
    pub const Shift: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::Shift;
    pub const ShowSubtypePicker: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::ShowSubtypePicker;
    pub const SwitchToPrevSubtype: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::SwitchToPrevSubtype;
    pub const SwitchToNextSubtype: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::SwitchToNextSubtype;
    pub const SwitchToClipboardContext: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::SwitchToClipboardContext;
    pub const SwitchToMediaContext: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::SwitchToMediaContext;
    pub const SwitchToEditingContext: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::SwitchToEditingContext;
    pub const ToggleOneHandedMode: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::ToggleOneHandedMode;
    pub const ToggleSmartbarVisibility: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::ToggleSmartbarVisibility;
    pub const Undo: ferroboard_core::settings::SwipeAction =
        ferroboard_core::settings::SwipeAction::Undo;
}
const _: () = assert!(ferroboard_core::settings::SwipeAction::NoAction.0 == 0);
const _: () = assert!(ferroboard_core::settings::SwipeAction::CycleToPreviousKeyboardMode.0 == 1);
const _: () = assert!(ferroboard_core::settings::SwipeAction::CycleToNextKeyboardMode.0 == 2);
const _: () = assert!(ferroboard_core::settings::SwipeAction::DeleteCharacter.0 == 3);
const _: () = assert!(ferroboard_core::settings::SwipeAction::DeleteCharactersPrecisely.0 == 4);
const _: () = assert!(ferroboard_core::settings::SwipeAction::DeleteWord.0 == 5);
const _: () = assert!(ferroboard_core::settings::SwipeAction::DeleteWordsPrecisely.0 == 6);
const _: () = assert!(ferroboard_core::settings::SwipeAction::HideKeyboard.0 == 7);
const _: () = assert!(ferroboard_core::settings::SwipeAction::InsertSpace.0 == 8);
const _: () = assert!(ferroboard_core::settings::SwipeAction::MoveCursorUp.0 == 9);
const _: () = assert!(ferroboard_core::settings::SwipeAction::MoveCursorDown.0 == 10);
const _: () = assert!(ferroboard_core::settings::SwipeAction::MoveCursorLeft.0 == 11);
const _: () = assert!(ferroboard_core::settings::SwipeAction::MoveCursorRight.0 == 12);
const _: () = assert!(ferroboard_core::settings::SwipeAction::MoveCursorStartOfLine.0 == 13);
const _: () = assert!(ferroboard_core::settings::SwipeAction::MoveCursorEndOfLine.0 == 14);
const _: () = assert!(ferroboard_core::settings::SwipeAction::MoveCursorStartOfPage.0 == 15);
const _: () = assert!(ferroboard_core::settings::SwipeAction::MoveCursorEndOfPage.0 == 16);
const _: () = assert!(ferroboard_core::settings::SwipeAction::Redo.0 == 17);
const _: () = assert!(ferroboard_core::settings::SwipeAction::SelectCharactersPrecisely.0 == 18);
const _: () = assert!(ferroboard_core::settings::SwipeAction::SelectWordsPrecisely.0 == 19);
const _: () = assert!(ferroboard_core::settings::SwipeAction::Shift.0 == 20);
const _: () = assert!(ferroboard_core::settings::SwipeAction::ShowSubtypePicker.0 == 21);
const _: () = assert!(ferroboard_core::settings::SwipeAction::SwitchToPrevSubtype.0 == 22);
const _: () = assert!(ferroboard_core::settings::SwipeAction::SwitchToNextSubtype.0 == 23);
const _: () = assert!(ferroboard_core::settings::SwipeAction::SwitchToClipboardContext.0 == 24);
const _: () = assert!(ferroboard_core::settings::SwipeAction::SwitchToMediaContext.0 == 25);
const _: () = assert!(ferroboard_core::settings::SwipeAction::SwitchToEditingContext.0 == 26);
const _: () = assert!(ferroboard_core::settings::SwipeAction::ToggleOneHandedMode.0 == 27);
const _: () = assert!(ferroboard_core::settings::SwipeAction::ToggleSmartbarVisibility.0 == 28);
const _: () = assert!(ferroboard_core::settings::SwipeAction::Undo.0 == 29);
impl MarkupTyped for SwipeActionMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("SwipeAction", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Settings");
            m.handles = &[ValueType::of::<ferroboard_core::settings::SwipeAction>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::settings::SwipeAction>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "NoAction",
                    value: 0,
                    rust_variant: Some("NoAction"),
                },
                MarkupEnumMember {
                    name: "CycleToPreviousKeyboardMode",
                    value: 1,
                    rust_variant: Some("CycleToPreviousKeyboardMode"),
                },
                MarkupEnumMember {
                    name: "CycleToNextKeyboardMode",
                    value: 2,
                    rust_variant: Some("CycleToNextKeyboardMode"),
                },
                MarkupEnumMember {
                    name: "DeleteCharacter",
                    value: 3,
                    rust_variant: Some("DeleteCharacter"),
                },
                MarkupEnumMember {
                    name: "DeleteCharactersPrecisely",
                    value: 4,
                    rust_variant: Some("DeleteCharactersPrecisely"),
                },
                MarkupEnumMember {
                    name: "DeleteWord",
                    value: 5,
                    rust_variant: Some("DeleteWord"),
                },
                MarkupEnumMember {
                    name: "DeleteWordsPrecisely",
                    value: 6,
                    rust_variant: Some("DeleteWordsPrecisely"),
                },
                MarkupEnumMember {
                    name: "HideKeyboard",
                    value: 7,
                    rust_variant: Some("HideKeyboard"),
                },
                MarkupEnumMember {
                    name: "InsertSpace",
                    value: 8,
                    rust_variant: Some("InsertSpace"),
                },
                MarkupEnumMember {
                    name: "MoveCursorUp",
                    value: 9,
                    rust_variant: Some("MoveCursorUp"),
                },
                MarkupEnumMember {
                    name: "MoveCursorDown",
                    value: 10,
                    rust_variant: Some("MoveCursorDown"),
                },
                MarkupEnumMember {
                    name: "MoveCursorLeft",
                    value: 11,
                    rust_variant: Some("MoveCursorLeft"),
                },
                MarkupEnumMember {
                    name: "MoveCursorRight",
                    value: 12,
                    rust_variant: Some("MoveCursorRight"),
                },
                MarkupEnumMember {
                    name: "MoveCursorStartOfLine",
                    value: 13,
                    rust_variant: Some("MoveCursorStartOfLine"),
                },
                MarkupEnumMember {
                    name: "MoveCursorEndOfLine",
                    value: 14,
                    rust_variant: Some("MoveCursorEndOfLine"),
                },
                MarkupEnumMember {
                    name: "MoveCursorStartOfPage",
                    value: 15,
                    rust_variant: Some("MoveCursorStartOfPage"),
                },
                MarkupEnumMember {
                    name: "MoveCursorEndOfPage",
                    value: 16,
                    rust_variant: Some("MoveCursorEndOfPage"),
                },
                MarkupEnumMember {
                    name: "Redo",
                    value: 17,
                    rust_variant: Some("Redo"),
                },
                MarkupEnumMember {
                    name: "SelectCharactersPrecisely",
                    value: 18,
                    rust_variant: Some("SelectCharactersPrecisely"),
                },
                MarkupEnumMember {
                    name: "SelectWordsPrecisely",
                    value: 19,
                    rust_variant: Some("SelectWordsPrecisely"),
                },
                MarkupEnumMember {
                    name: "Shift",
                    value: 20,
                    rust_variant: Some("Shift"),
                },
                MarkupEnumMember {
                    name: "ShowSubtypePicker",
                    value: 21,
                    rust_variant: Some("ShowSubtypePicker"),
                },
                MarkupEnumMember {
                    name: "SwitchToPrevSubtype",
                    value: 22,
                    rust_variant: Some("SwitchToPrevSubtype"),
                },
                MarkupEnumMember {
                    name: "SwitchToNextSubtype",
                    value: 23,
                    rust_variant: Some("SwitchToNextSubtype"),
                },
                MarkupEnumMember {
                    name: "SwitchToClipboardContext",
                    value: 24,
                    rust_variant: Some("SwitchToClipboardContext"),
                },
                MarkupEnumMember {
                    name: "SwitchToMediaContext",
                    value: 25,
                    rust_variant: Some("SwitchToMediaContext"),
                },
                MarkupEnumMember {
                    name: "SwitchToEditingContext",
                    value: 26,
                    rust_variant: Some("SwitchToEditingContext"),
                },
                MarkupEnumMember {
                    name: "ToggleOneHandedMode",
                    value: 27,
                    rust_variant: Some("ToggleOneHandedMode"),
                },
                MarkupEnumMember {
                    name: "ToggleSmartbarVisibility",
                    value: 28,
                    rust_variant: Some("ToggleSmartbarVisibility"),
                },
                MarkupEnumMember {
                    name: "Undo",
                    value: 29,
                    rust_variant: Some("Undo"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::settings::SwipeAction(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::settings::SwipeAction>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct SwipeDirectionMetadata;
#[allow(non_upper_case_globals)]
impl SwipeDirectionMetadata {
    pub const None: ferroboard_core::settings::SwipeDirection =
        ferroboard_core::settings::SwipeDirection::None;
    pub const Up: ferroboard_core::settings::SwipeDirection =
        ferroboard_core::settings::SwipeDirection::Up;
    pub const Down: ferroboard_core::settings::SwipeDirection =
        ferroboard_core::settings::SwipeDirection::Down;
    pub const Left: ferroboard_core::settings::SwipeDirection =
        ferroboard_core::settings::SwipeDirection::Left;
    pub const Right: ferroboard_core::settings::SwipeDirection =
        ferroboard_core::settings::SwipeDirection::Right;
    pub const UpLeft: ferroboard_core::settings::SwipeDirection =
        ferroboard_core::settings::SwipeDirection::UpLeft;
    pub const UpRight: ferroboard_core::settings::SwipeDirection =
        ferroboard_core::settings::SwipeDirection::UpRight;
    pub const DownLeft: ferroboard_core::settings::SwipeDirection =
        ferroboard_core::settings::SwipeDirection::DownLeft;
    pub const DownRight: ferroboard_core::settings::SwipeDirection =
        ferroboard_core::settings::SwipeDirection::DownRight;
}
const _: () = assert!(ferroboard_core::settings::SwipeDirection::None.0 == 0);
const _: () = assert!(ferroboard_core::settings::SwipeDirection::Up.0 == 1);
const _: () = assert!(ferroboard_core::settings::SwipeDirection::Down.0 == 2);
const _: () = assert!(ferroboard_core::settings::SwipeDirection::Left.0 == 3);
const _: () = assert!(ferroboard_core::settings::SwipeDirection::Right.0 == 4);
const _: () = assert!(ferroboard_core::settings::SwipeDirection::UpLeft.0 == 5);
const _: () = assert!(ferroboard_core::settings::SwipeDirection::UpRight.0 == 6);
const _: () = assert!(ferroboard_core::settings::SwipeDirection::DownLeft.0 == 7);
const _: () = assert!(ferroboard_core::settings::SwipeDirection::DownRight.0 == 8);
impl MarkupTyped for SwipeDirectionMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("SwipeDirection", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Settings");
            m.handles = &[ValueType::of::<ferroboard_core::settings::SwipeDirection>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::settings::SwipeDirection>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "None",
                    value: 0,
                    rust_variant: Some("None"),
                },
                MarkupEnumMember {
                    name: "Up",
                    value: 1,
                    rust_variant: Some("Up"),
                },
                MarkupEnumMember {
                    name: "Down",
                    value: 2,
                    rust_variant: Some("Down"),
                },
                MarkupEnumMember {
                    name: "Left",
                    value: 3,
                    rust_variant: Some("Left"),
                },
                MarkupEnumMember {
                    name: "Right",
                    value: 4,
                    rust_variant: Some("Right"),
                },
                MarkupEnumMember {
                    name: "UpLeft",
                    value: 5,
                    rust_variant: Some("UpLeft"),
                },
                MarkupEnumMember {
                    name: "UpRight",
                    value: 6,
                    rust_variant: Some("UpRight"),
                },
                MarkupEnumMember {
                    name: "DownLeft",
                    value: 7,
                    rust_variant: Some("DownLeft"),
                },
                MarkupEnumMember {
                    name: "DownRight",
                    value: 8,
                    rust_variant: Some("DownRight"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::settings::SwipeDirection(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::settings::SwipeDirection>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct FeedbackKindMetadata;
#[allow(non_upper_case_globals)]
impl FeedbackKindMetadata {
    pub const KeyPress: ferroboard_core::settings::FeedbackKind =
        ferroboard_core::settings::FeedbackKind::KeyPress;
    pub const KeyPressDelete: ferroboard_core::settings::FeedbackKind =
        ferroboard_core::settings::FeedbackKind::KeyPressDelete;
    pub const KeyPressSpace: ferroboard_core::settings::FeedbackKind =
        ferroboard_core::settings::FeedbackKind::KeyPressSpace;
    pub const KeyPressEnter: ferroboard_core::settings::FeedbackKind =
        ferroboard_core::settings::FeedbackKind::KeyPressEnter;
    pub const KeyPressFunction: ferroboard_core::settings::FeedbackKind =
        ferroboard_core::settings::FeedbackKind::KeyPressFunction;
    pub const LongPress: ferroboard_core::settings::FeedbackKind =
        ferroboard_core::settings::FeedbackKind::LongPress;
    pub const KeyRepeat: ferroboard_core::settings::FeedbackKind =
        ferroboard_core::settings::FeedbackKind::KeyRepeat;
    pub const GestureStep: ferroboard_core::settings::FeedbackKind =
        ferroboard_core::settings::FeedbackKind::GestureStep;
}
const _: () = assert!(ferroboard_core::settings::FeedbackKind::KeyPress.0 == 0);
const _: () = assert!(ferroboard_core::settings::FeedbackKind::KeyPressDelete.0 == 1);
const _: () = assert!(ferroboard_core::settings::FeedbackKind::KeyPressSpace.0 == 2);
const _: () = assert!(ferroboard_core::settings::FeedbackKind::KeyPressEnter.0 == 3);
const _: () = assert!(ferroboard_core::settings::FeedbackKind::KeyPressFunction.0 == 4);
const _: () = assert!(ferroboard_core::settings::FeedbackKind::LongPress.0 == 5);
const _: () = assert!(ferroboard_core::settings::FeedbackKind::KeyRepeat.0 == 6);
const _: () = assert!(ferroboard_core::settings::FeedbackKind::GestureStep.0 == 7);
impl MarkupTyped for FeedbackKindMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("FeedbackKind", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Settings");
            m.handles = &[ValueType::of::<ferroboard_core::settings::FeedbackKind>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::settings::FeedbackKind>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "KeyPress",
                    value: 0,
                    rust_variant: Some("KeyPress"),
                },
                MarkupEnumMember {
                    name: "KeyPressDelete",
                    value: 1,
                    rust_variant: Some("KeyPressDelete"),
                },
                MarkupEnumMember {
                    name: "KeyPressSpace",
                    value: 2,
                    rust_variant: Some("KeyPressSpace"),
                },
                MarkupEnumMember {
                    name: "KeyPressEnter",
                    value: 3,
                    rust_variant: Some("KeyPressEnter"),
                },
                MarkupEnumMember {
                    name: "KeyPressFunction",
                    value: 4,
                    rust_variant: Some("KeyPressFunction"),
                },
                MarkupEnumMember {
                    name: "LongPress",
                    value: 5,
                    rust_variant: Some("LongPress"),
                },
                MarkupEnumMember {
                    name: "KeyRepeat",
                    value: 6,
                    rust_variant: Some("KeyRepeat"),
                },
                MarkupEnumMember {
                    name: "GestureStep",
                    value: 7,
                    rust_variant: Some("GestureStep"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::settings::FeedbackKind(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::settings::FeedbackKind>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct SmartbarLayoutMetadata;
#[allow(non_upper_case_globals)]
impl SmartbarLayoutMetadata {
    pub const SuggestionsWithActions: ferroboard_core::settings::SmartbarLayout =
        ferroboard_core::settings::SmartbarLayout::SuggestionsWithActions;
    pub const SuggestionsOnly: ferroboard_core::settings::SmartbarLayout =
        ferroboard_core::settings::SmartbarLayout::SuggestionsOnly;
    pub const ActionsOnly: ferroboard_core::settings::SmartbarLayout =
        ferroboard_core::settings::SmartbarLayout::ActionsOnly;
}
const _: () = assert!(ferroboard_core::settings::SmartbarLayout::SuggestionsWithActions.0 == 0);
const _: () = assert!(ferroboard_core::settings::SmartbarLayout::SuggestionsOnly.0 == 1);
const _: () = assert!(ferroboard_core::settings::SmartbarLayout::ActionsOnly.0 == 2);
impl MarkupTyped for SmartbarLayoutMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("SmartbarLayout", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Settings");
            m.handles = &[ValueType::of::<ferroboard_core::settings::SmartbarLayout>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::settings::SmartbarLayout>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "SuggestionsWithActions",
                    value: 0,
                    rust_variant: Some("SuggestionsWithActions"),
                },
                MarkupEnumMember {
                    name: "SuggestionsOnly",
                    value: 1,
                    rust_variant: Some("SuggestionsOnly"),
                },
                MarkupEnumMember {
                    name: "ActionsOnly",
                    value: 2,
                    rust_variant: Some("ActionsOnly"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::settings::SmartbarLayout(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::settings::SmartbarLayout>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct EmojiSkinToneMetadata;
#[allow(non_upper_case_globals)]
impl EmojiSkinToneMetadata {
    pub const Default: ferroboard_core::emoji::EmojiSkinTone =
        ferroboard_core::emoji::EmojiSkinTone::Default;
    pub const Light: ferroboard_core::emoji::EmojiSkinTone =
        ferroboard_core::emoji::EmojiSkinTone::Light;
    pub const MediumLight: ferroboard_core::emoji::EmojiSkinTone =
        ferroboard_core::emoji::EmojiSkinTone::MediumLight;
    pub const Medium: ferroboard_core::emoji::EmojiSkinTone =
        ferroboard_core::emoji::EmojiSkinTone::Medium;
    pub const MediumDark: ferroboard_core::emoji::EmojiSkinTone =
        ferroboard_core::emoji::EmojiSkinTone::MediumDark;
    pub const Dark: ferroboard_core::emoji::EmojiSkinTone =
        ferroboard_core::emoji::EmojiSkinTone::Dark;
}
const _: () = assert!(ferroboard_core::emoji::EmojiSkinTone::Default.0 == 0);
const _: () = assert!(ferroboard_core::emoji::EmojiSkinTone::Light.0 == 1);
const _: () = assert!(ferroboard_core::emoji::EmojiSkinTone::MediumLight.0 == 2);
const _: () = assert!(ferroboard_core::emoji::EmojiSkinTone::Medium.0 == 3);
const _: () = assert!(ferroboard_core::emoji::EmojiSkinTone::MediumDark.0 == 4);
const _: () = assert!(ferroboard_core::emoji::EmojiSkinTone::Dark.0 == 5);
impl MarkupTyped for EmojiSkinToneMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("EmojiSkinTone", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Emoji");
            m.handles = &[ValueType::of::<ferroboard_core::emoji::EmojiSkinTone>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::emoji::EmojiSkinTone>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Default",
                    value: 0,
                    rust_variant: Some("Default"),
                },
                MarkupEnumMember {
                    name: "Light",
                    value: 1,
                    rust_variant: Some("Light"),
                },
                MarkupEnumMember {
                    name: "MediumLight",
                    value: 2,
                    rust_variant: Some("MediumLight"),
                },
                MarkupEnumMember {
                    name: "Medium",
                    value: 3,
                    rust_variant: Some("Medium"),
                },
                MarkupEnumMember {
                    name: "MediumDark",
                    value: 4,
                    rust_variant: Some("MediumDark"),
                },
                MarkupEnumMember {
                    name: "Dark",
                    value: 5,
                    rust_variant: Some("Dark"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::emoji::EmojiSkinTone(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::emoji::EmojiSkinTone>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct InputKindMetadata;
#[allow(non_upper_case_globals)]
impl InputKindMetadata {
    pub const Text: ferroboard_core::editor::InputKind = ferroboard_core::editor::InputKind::Text;
    pub const Number: ferroboard_core::editor::InputKind =
        ferroboard_core::editor::InputKind::Number;
    pub const Decimal: ferroboard_core::editor::InputKind =
        ferroboard_core::editor::InputKind::Decimal;
    pub const Phone: ferroboard_core::editor::InputKind = ferroboard_core::editor::InputKind::Phone;
    pub const DateTime: ferroboard_core::editor::InputKind =
        ferroboard_core::editor::InputKind::DateTime;
    pub const Email: ferroboard_core::editor::InputKind = ferroboard_core::editor::InputKind::Email;
    pub const Uri: ferroboard_core::editor::InputKind = ferroboard_core::editor::InputKind::Uri;
    pub const Password: ferroboard_core::editor::InputKind =
        ferroboard_core::editor::InputKind::Password;
    pub const NumericPassword: ferroboard_core::editor::InputKind =
        ferroboard_core::editor::InputKind::NumericPassword;
    pub const Search: ferroboard_core::editor::InputKind =
        ferroboard_core::editor::InputKind::Search;
    pub const Chat: ferroboard_core::editor::InputKind = ferroboard_core::editor::InputKind::Chat;
}
const _: () = assert!(ferroboard_core::editor::InputKind::Text.0 == 0);
const _: () = assert!(ferroboard_core::editor::InputKind::Number.0 == 1);
const _: () = assert!(ferroboard_core::editor::InputKind::Decimal.0 == 2);
const _: () = assert!(ferroboard_core::editor::InputKind::Phone.0 == 3);
const _: () = assert!(ferroboard_core::editor::InputKind::DateTime.0 == 4);
const _: () = assert!(ferroboard_core::editor::InputKind::Email.0 == 5);
const _: () = assert!(ferroboard_core::editor::InputKind::Uri.0 == 6);
const _: () = assert!(ferroboard_core::editor::InputKind::Password.0 == 7);
const _: () = assert!(ferroboard_core::editor::InputKind::NumericPassword.0 == 8);
const _: () = assert!(ferroboard_core::editor::InputKind::Search.0 == 9);
const _: () = assert!(ferroboard_core::editor::InputKind::Chat.0 == 10);
impl MarkupTyped for InputKindMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("InputKind", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Editor");
            m.handles = &[ValueType::of::<ferroboard_core::editor::InputKind>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::editor::InputKind>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Text",
                    value: 0,
                    rust_variant: Some("Text"),
                },
                MarkupEnumMember {
                    name: "Number",
                    value: 1,
                    rust_variant: Some("Number"),
                },
                MarkupEnumMember {
                    name: "Decimal",
                    value: 2,
                    rust_variant: Some("Decimal"),
                },
                MarkupEnumMember {
                    name: "Phone",
                    value: 3,
                    rust_variant: Some("Phone"),
                },
                MarkupEnumMember {
                    name: "DateTime",
                    value: 4,
                    rust_variant: Some("DateTime"),
                },
                MarkupEnumMember {
                    name: "Email",
                    value: 5,
                    rust_variant: Some("Email"),
                },
                MarkupEnumMember {
                    name: "Uri",
                    value: 6,
                    rust_variant: Some("Uri"),
                },
                MarkupEnumMember {
                    name: "Password",
                    value: 7,
                    rust_variant: Some("Password"),
                },
                MarkupEnumMember {
                    name: "NumericPassword",
                    value: 8,
                    rust_variant: Some("NumericPassword"),
                },
                MarkupEnumMember {
                    name: "Search",
                    value: 9,
                    rust_variant: Some("Search"),
                },
                MarkupEnumMember {
                    name: "Chat",
                    value: 10,
                    rust_variant: Some("Chat"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::editor::InputKind(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::editor::InputKind>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct EnterActionMetadata;
#[allow(non_upper_case_globals)]
impl EnterActionMetadata {
    pub const Default: ferroboard_core::editor::EnterAction =
        ferroboard_core::editor::EnterAction::Default;
    pub const NewLine: ferroboard_core::editor::EnterAction =
        ferroboard_core::editor::EnterAction::NewLine;
    pub const Go: ferroboard_core::editor::EnterAction = ferroboard_core::editor::EnterAction::Go;
    pub const Search: ferroboard_core::editor::EnterAction =
        ferroboard_core::editor::EnterAction::Search;
    pub const Send: ferroboard_core::editor::EnterAction =
        ferroboard_core::editor::EnterAction::Send;
    pub const Next: ferroboard_core::editor::EnterAction =
        ferroboard_core::editor::EnterAction::Next;
    pub const Previous: ferroboard_core::editor::EnterAction =
        ferroboard_core::editor::EnterAction::Previous;
    pub const Done: ferroboard_core::editor::EnterAction =
        ferroboard_core::editor::EnterAction::Done;
}
const _: () = assert!(ferroboard_core::editor::EnterAction::Default.0 == 0);
const _: () = assert!(ferroboard_core::editor::EnterAction::NewLine.0 == 1);
const _: () = assert!(ferroboard_core::editor::EnterAction::Go.0 == 2);
const _: () = assert!(ferroboard_core::editor::EnterAction::Search.0 == 3);
const _: () = assert!(ferroboard_core::editor::EnterAction::Send.0 == 4);
const _: () = assert!(ferroboard_core::editor::EnterAction::Next.0 == 5);
const _: () = assert!(ferroboard_core::editor::EnterAction::Previous.0 == 6);
const _: () = assert!(ferroboard_core::editor::EnterAction::Done.0 == 7);
impl MarkupTyped for EnterActionMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("EnterAction", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Editor");
            m.handles = &[ValueType::of::<ferroboard_core::editor::EnterAction>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::editor::EnterAction>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Default",
                    value: 0,
                    rust_variant: Some("Default"),
                },
                MarkupEnumMember {
                    name: "NewLine",
                    value: 1,
                    rust_variant: Some("NewLine"),
                },
                MarkupEnumMember {
                    name: "Go",
                    value: 2,
                    rust_variant: Some("Go"),
                },
                MarkupEnumMember {
                    name: "Search",
                    value: 3,
                    rust_variant: Some("Search"),
                },
                MarkupEnumMember {
                    name: "Send",
                    value: 4,
                    rust_variant: Some("Send"),
                },
                MarkupEnumMember {
                    name: "Next",
                    value: 5,
                    rust_variant: Some("Next"),
                },
                MarkupEnumMember {
                    name: "Previous",
                    value: 6,
                    rust_variant: Some("Previous"),
                },
                MarkupEnumMember {
                    name: "Done",
                    value: 7,
                    rust_variant: Some("Done"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::editor::EnterAction(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::editor::EnterAction>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct CapitalizationModeMetadata;
#[allow(non_upper_case_globals)]
impl CapitalizationModeMetadata {
    pub const None: ferroboard_core::editor::CapitalizationMode =
        ferroboard_core::editor::CapitalizationMode::None;
    pub const Characters: ferroboard_core::editor::CapitalizationMode =
        ferroboard_core::editor::CapitalizationMode::Characters;
    pub const Words: ferroboard_core::editor::CapitalizationMode =
        ferroboard_core::editor::CapitalizationMode::Words;
    pub const Sentences: ferroboard_core::editor::CapitalizationMode =
        ferroboard_core::editor::CapitalizationMode::Sentences;
}
const _: () = assert!(ferroboard_core::editor::CapitalizationMode::None.0 == 0);
const _: () = assert!(ferroboard_core::editor::CapitalizationMode::Characters.0 == 1);
const _: () = assert!(ferroboard_core::editor::CapitalizationMode::Words.0 == 2);
const _: () = assert!(ferroboard_core::editor::CapitalizationMode::Sentences.0 == 3);
impl MarkupTyped for CapitalizationModeMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("CapitalizationMode", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Editor");
            m.handles = &[ValueType::of::<ferroboard_core::editor::CapitalizationMode>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::editor::CapitalizationMode>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "None",
                    value: 0,
                    rust_variant: Some("None"),
                },
                MarkupEnumMember {
                    name: "Characters",
                    value: 1,
                    rust_variant: Some("Characters"),
                },
                MarkupEnumMember {
                    name: "Words",
                    value: 2,
                    rust_variant: Some("Words"),
                },
                MarkupEnumMember {
                    name: "Sentences",
                    value: 3,
                    rust_variant: Some("Sentences"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::editor::CapitalizationMode(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::editor::CapitalizationMode>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct KeyTypeMetadata;
#[allow(non_upper_case_globals)]
impl KeyTypeMetadata {
    pub const Character: ferroboard_core::keys::KeyType = ferroboard_core::keys::KeyType::Character;
    pub const EnterEditing: ferroboard_core::keys::KeyType =
        ferroboard_core::keys::KeyType::EnterEditing;
    pub const Function: ferroboard_core::keys::KeyType = ferroboard_core::keys::KeyType::Function;
    pub const Lock: ferroboard_core::keys::KeyType = ferroboard_core::keys::KeyType::Lock;
    pub const Modifier: ferroboard_core::keys::KeyType = ferroboard_core::keys::KeyType::Modifier;
    pub const Navigation: ferroboard_core::keys::KeyType =
        ferroboard_core::keys::KeyType::Navigation;
    pub const SystemGui: ferroboard_core::keys::KeyType = ferroboard_core::keys::KeyType::SystemGui;
    pub const Numeric: ferroboard_core::keys::KeyType = ferroboard_core::keys::KeyType::Numeric;
    pub const Placeholder: ferroboard_core::keys::KeyType =
        ferroboard_core::keys::KeyType::Placeholder;
    pub const Unspecified: ferroboard_core::keys::KeyType =
        ferroboard_core::keys::KeyType::Unspecified;
}
const _: () = assert!(ferroboard_core::keys::KeyType::Character.0 == 0);
const _: () = assert!(ferroboard_core::keys::KeyType::EnterEditing.0 == 1);
const _: () = assert!(ferroboard_core::keys::KeyType::Function.0 == 2);
const _: () = assert!(ferroboard_core::keys::KeyType::Lock.0 == 3);
const _: () = assert!(ferroboard_core::keys::KeyType::Modifier.0 == 4);
const _: () = assert!(ferroboard_core::keys::KeyType::Navigation.0 == 5);
const _: () = assert!(ferroboard_core::keys::KeyType::SystemGui.0 == 6);
const _: () = assert!(ferroboard_core::keys::KeyType::Numeric.0 == 7);
const _: () = assert!(ferroboard_core::keys::KeyType::Placeholder.0 == 8);
const _: () = assert!(ferroboard_core::keys::KeyType::Unspecified.0 == 9);
impl MarkupTyped for KeyTypeMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("KeyType", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Keys");
            m.handles = &[ValueType::of::<ferroboard_core::keys::KeyType>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::keys::KeyType>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Character",
                    value: 0,
                    rust_variant: Some("Character"),
                },
                MarkupEnumMember {
                    name: "EnterEditing",
                    value: 1,
                    rust_variant: Some("EnterEditing"),
                },
                MarkupEnumMember {
                    name: "Function",
                    value: 2,
                    rust_variant: Some("Function"),
                },
                MarkupEnumMember {
                    name: "Lock",
                    value: 3,
                    rust_variant: Some("Lock"),
                },
                MarkupEnumMember {
                    name: "Modifier",
                    value: 4,
                    rust_variant: Some("Modifier"),
                },
                MarkupEnumMember {
                    name: "Navigation",
                    value: 5,
                    rust_variant: Some("Navigation"),
                },
                MarkupEnumMember {
                    name: "SystemGui",
                    value: 6,
                    rust_variant: Some("SystemGui"),
                },
                MarkupEnumMember {
                    name: "Numeric",
                    value: 7,
                    rust_variant: Some("Numeric"),
                },
                MarkupEnumMember {
                    name: "Placeholder",
                    value: 8,
                    rust_variant: Some("Placeholder"),
                },
                MarkupEnumMember {
                    name: "Unspecified",
                    value: 9,
                    rust_variant: Some("Unspecified"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::keys::KeyType(v)) as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::keys::KeyType>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct KeyboardModeMetadata;
#[allow(non_upper_case_globals)]
impl KeyboardModeMetadata {
    pub const Characters: ferroboard_core::keys::KeyboardMode =
        ferroboard_core::keys::KeyboardMode::Characters;
    pub const Symbols: ferroboard_core::keys::KeyboardMode =
        ferroboard_core::keys::KeyboardMode::Symbols;
    pub const Symbols2: ferroboard_core::keys::KeyboardMode =
        ferroboard_core::keys::KeyboardMode::Symbols2;
    pub const Numeric: ferroboard_core::keys::KeyboardMode =
        ferroboard_core::keys::KeyboardMode::Numeric;
    pub const NumericAdvanced: ferroboard_core::keys::KeyboardMode =
        ferroboard_core::keys::KeyboardMode::NumericAdvanced;
    pub const Phone: ferroboard_core::keys::KeyboardMode =
        ferroboard_core::keys::KeyboardMode::Phone;
    pub const Phone2: ferroboard_core::keys::KeyboardMode =
        ferroboard_core::keys::KeyboardMode::Phone2;
}
const _: () = assert!(ferroboard_core::keys::KeyboardMode::Characters.0 == 0);
const _: () = assert!(ferroboard_core::keys::KeyboardMode::Symbols.0 == 1);
const _: () = assert!(ferroboard_core::keys::KeyboardMode::Symbols2.0 == 2);
const _: () = assert!(ferroboard_core::keys::KeyboardMode::Numeric.0 == 3);
const _: () = assert!(ferroboard_core::keys::KeyboardMode::NumericAdvanced.0 == 4);
const _: () = assert!(ferroboard_core::keys::KeyboardMode::Phone.0 == 5);
const _: () = assert!(ferroboard_core::keys::KeyboardMode::Phone2.0 == 6);
impl MarkupTyped for KeyboardModeMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("KeyboardMode", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Keys");
            m.handles = &[ValueType::of::<ferroboard_core::keys::KeyboardMode>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::keys::KeyboardMode>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Characters",
                    value: 0,
                    rust_variant: Some("Characters"),
                },
                MarkupEnumMember {
                    name: "Symbols",
                    value: 1,
                    rust_variant: Some("Symbols"),
                },
                MarkupEnumMember {
                    name: "Symbols2",
                    value: 2,
                    rust_variant: Some("Symbols2"),
                },
                MarkupEnumMember {
                    name: "Numeric",
                    value: 3,
                    rust_variant: Some("Numeric"),
                },
                MarkupEnumMember {
                    name: "NumericAdvanced",
                    value: 4,
                    rust_variant: Some("NumericAdvanced"),
                },
                MarkupEnumMember {
                    name: "Phone",
                    value: 5,
                    rust_variant: Some("Phone"),
                },
                MarkupEnumMember {
                    name: "Phone2",
                    value: 6,
                    rust_variant: Some("Phone2"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::keys::KeyboardMode(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::keys::KeyboardMode>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct KeyVariationMetadata;
#[allow(non_upper_case_globals)]
impl KeyVariationMetadata {
    pub const All: ferroboard_core::keys::KeyVariation = ferroboard_core::keys::KeyVariation::All;
    pub const Normal: ferroboard_core::keys::KeyVariation =
        ferroboard_core::keys::KeyVariation::Normal;
    pub const EmailAddress: ferroboard_core::keys::KeyVariation =
        ferroboard_core::keys::KeyVariation::EmailAddress;
    pub const Uri: ferroboard_core::keys::KeyVariation = ferroboard_core::keys::KeyVariation::Uri;
    pub const Password: ferroboard_core::keys::KeyVariation =
        ferroboard_core::keys::KeyVariation::Password;
}
const _: () = assert!(ferroboard_core::keys::KeyVariation::All.0 == 0);
const _: () = assert!(ferroboard_core::keys::KeyVariation::Normal.0 == 1);
const _: () = assert!(ferroboard_core::keys::KeyVariation::EmailAddress.0 == 2);
const _: () = assert!(ferroboard_core::keys::KeyVariation::Uri.0 == 3);
const _: () = assert!(ferroboard_core::keys::KeyVariation::Password.0 == 4);
impl MarkupTyped for KeyVariationMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("KeyVariation", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Keys");
            m.handles = &[ValueType::of::<ferroboard_core::keys::KeyVariation>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::keys::KeyVariation>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "All",
                    value: 0,
                    rust_variant: Some("All"),
                },
                MarkupEnumMember {
                    name: "Normal",
                    value: 1,
                    rust_variant: Some("Normal"),
                },
                MarkupEnumMember {
                    name: "EmailAddress",
                    value: 2,
                    rust_variant: Some("EmailAddress"),
                },
                MarkupEnumMember {
                    name: "Uri",
                    value: 3,
                    rust_variant: Some("Uri"),
                },
                MarkupEnumMember {
                    name: "Password",
                    value: 4,
                    rust_variant: Some("Password"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::keys::KeyVariation(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::keys::KeyVariation>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct ShiftStateMetadata;
#[allow(non_upper_case_globals)]
impl ShiftStateMetadata {
    pub const Unshifted: ferroboard_core::keys::ShiftState =
        ferroboard_core::keys::ShiftState::Unshifted;
    pub const ShiftedManual: ferroboard_core::keys::ShiftState =
        ferroboard_core::keys::ShiftState::ShiftedManual;
    pub const ShiftedAutomatic: ferroboard_core::keys::ShiftState =
        ferroboard_core::keys::ShiftState::ShiftedAutomatic;
    pub const CapsLock: ferroboard_core::keys::ShiftState =
        ferroboard_core::keys::ShiftState::CapsLock;
}
const _: () = assert!(ferroboard_core::keys::ShiftState::Unshifted.0 == 0);
const _: () = assert!(ferroboard_core::keys::ShiftState::ShiftedManual.0 == 1);
const _: () = assert!(ferroboard_core::keys::ShiftState::ShiftedAutomatic.0 == 2);
const _: () = assert!(ferroboard_core::keys::ShiftState::CapsLock.0 == 3);
impl MarkupTyped for ShiftStateMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("ShiftState", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Keys");
            m.handles = &[ValueType::of::<ferroboard_core::keys::ShiftState>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::keys::ShiftState>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Unshifted",
                    value: 0,
                    rust_variant: Some("Unshifted"),
                },
                MarkupEnumMember {
                    name: "ShiftedManual",
                    value: 1,
                    rust_variant: Some("ShiftedManual"),
                },
                MarkupEnumMember {
                    name: "ShiftedAutomatic",
                    value: 2,
                    rust_variant: Some("ShiftedAutomatic"),
                },
                MarkupEnumMember {
                    name: "CapsLock",
                    value: 3,
                    rust_variant: Some("CapsLock"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::keys::ShiftState(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::keys::ShiftState>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct LayoutDirectionMetadata;
#[allow(non_upper_case_globals)]
impl LayoutDirectionMetadata {
    pub const Ltr: ferroboard_core::keys::LayoutDirection =
        ferroboard_core::keys::LayoutDirection::Ltr;
    pub const Rtl: ferroboard_core::keys::LayoutDirection =
        ferroboard_core::keys::LayoutDirection::Rtl;
}
const _: () = assert!(ferroboard_core::keys::LayoutDirection::Ltr.0 == 0);
const _: () = assert!(ferroboard_core::keys::LayoutDirection::Rtl.0 == 1);
impl MarkupTyped for LayoutDirectionMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("LayoutDirection", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Keys");
            m.handles = &[ValueType::of::<ferroboard_core::keys::LayoutDirection>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::keys::LayoutDirection>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Ltr",
                    value: 0,
                    rust_variant: Some("Ltr"),
                },
                MarkupEnumMember {
                    name: "Rtl",
                    value: 1,
                    rust_variant: Some("Rtl"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::keys::LayoutDirection(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::keys::LayoutDirection>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct KeyboardUiModeMetadata;
#[allow(non_upper_case_globals)]
impl KeyboardUiModeMetadata {
    pub const Text: ferroboard_core::keys::KeyboardUiMode =
        ferroboard_core::keys::KeyboardUiMode::Text;
    pub const Media: ferroboard_core::keys::KeyboardUiMode =
        ferroboard_core::keys::KeyboardUiMode::Media;
    pub const Clipboard: ferroboard_core::keys::KeyboardUiMode =
        ferroboard_core::keys::KeyboardUiMode::Clipboard;
    pub const Editing: ferroboard_core::keys::KeyboardUiMode =
        ferroboard_core::keys::KeyboardUiMode::Editing;
}
const _: () = assert!(ferroboard_core::keys::KeyboardUiMode::Text.0 == 0);
const _: () = assert!(ferroboard_core::keys::KeyboardUiMode::Media.0 == 1);
const _: () = assert!(ferroboard_core::keys::KeyboardUiMode::Clipboard.0 == 2);
const _: () = assert!(ferroboard_core::keys::KeyboardUiMode::Editing.0 == 3);
impl MarkupTyped for KeyboardUiModeMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("KeyboardUiMode", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Keys");
            m.handles = &[ValueType::of::<ferroboard_core::keys::KeyboardUiMode>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::keys::KeyboardUiMode>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Text",
                    value: 0,
                    rust_variant: Some("Text"),
                },
                MarkupEnumMember {
                    name: "Media",
                    value: 1,
                    rust_variant: Some("Media"),
                },
                MarkupEnumMember {
                    name: "Clipboard",
                    value: 2,
                    rust_variant: Some("Clipboard"),
                },
                MarkupEnumMember {
                    name: "Editing",
                    value: 3,
                    rust_variant: Some("Editing"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::keys::KeyboardUiMode(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::keys::KeyboardUiMode>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct KeyIconMetadata;
#[allow(non_upper_case_globals)]
impl KeyIconMetadata {
    pub const None: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::None;
    pub const Backspace: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Backspace;
    pub const Shift: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Shift;
    pub const ShiftActive: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::ShiftActive;
    pub const CapsLock: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::CapsLock;
    pub const Enter: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Enter;
    pub const Search: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Search;
    pub const Send: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Send;
    pub const Go: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Go;
    pub const Next: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Next;
    pub const Previous: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Previous;
    pub const Done: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Done;
    pub const Space: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Space;
    pub const Language: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Language;
    pub const Emoji: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Emoji;
    pub const Clipboard: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Clipboard;
    pub const Settings: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Settings;
    pub const HideKeyboard: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::HideKeyboard;
    pub const ArrowLeft: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::ArrowLeft;
    pub const ArrowRight: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::ArrowRight;
    pub const ArrowUp: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::ArrowUp;
    pub const ArrowDown: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::ArrowDown;
    pub const Undo: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Undo;
    pub const Redo: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Redo;
    pub const Copy: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Copy;
    pub const Cut: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Cut;
    pub const Paste: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Paste;
    pub const SelectAll: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::SelectAll;
    pub const Select: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Select;
    pub const Microphone: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::Microphone;
    pub const OneHandedLeft: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::OneHandedLeft;
    pub const OneHandedRight: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::OneHandedRight;
    pub const Keyboard: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Keyboard;
    pub const Tab: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Tab;
    pub const Incognito: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Incognito;
    pub const TextEditing: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::TextEditing;
    pub const Home: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Home;
    pub const End: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::End;
    pub const DeleteForward: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::DeleteForward;
    pub const SplitKeyboard: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::SplitKeyboard;
    pub const Close: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Close;
    pub const Pin: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Pin;
    pub const Unpin: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Unpin;
    pub const Delete: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Delete;
    pub const Recent: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Recent;
    pub const Smiley: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Smiley;
    pub const Person: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Person;
    pub const Nature: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Nature;
    pub const Food: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Food;
    pub const Travel: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Travel;
    pub const Activity: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Activity;
    pub const Objects: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Objects;
    pub const Symbols: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Symbols;
    pub const Flag: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Flag;
    pub const Emoticon: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Emoticon;
    pub const ChevronLeft: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::ChevronLeft;
    pub const ChevronRight: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::ChevronRight;
    pub const ChevronUp: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::ChevronUp;
    pub const ChevronDown: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::ChevronDown;
    pub const MoreHorizontal: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::MoreHorizontal;
    pub const Resize: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Resize;
    pub const Autocorrect: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::Autocorrect;
    pub const Numpad: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Numpad;
    pub const DocumentStart: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::DocumentStart;
    pub const DocumentEnd: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::DocumentEnd;
    pub const ResizeHeight: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::ResizeHeight;
    pub const Floating: ferroboard_core::keys::KeyIcon = ferroboard_core::keys::KeyIcon::Floating;
    pub const DragHandle: ferroboard_core::keys::KeyIcon =
        ferroboard_core::keys::KeyIcon::DragHandle;
}
const _: () = assert!(ferroboard_core::keys::KeyIcon::None.0 == 0);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Backspace.0 == 1);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Shift.0 == 2);
const _: () = assert!(ferroboard_core::keys::KeyIcon::ShiftActive.0 == 3);
const _: () = assert!(ferroboard_core::keys::KeyIcon::CapsLock.0 == 4);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Enter.0 == 5);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Search.0 == 6);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Send.0 == 7);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Go.0 == 8);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Next.0 == 9);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Previous.0 == 10);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Done.0 == 11);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Space.0 == 12);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Language.0 == 13);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Emoji.0 == 14);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Clipboard.0 == 15);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Settings.0 == 16);
const _: () = assert!(ferroboard_core::keys::KeyIcon::HideKeyboard.0 == 17);
const _: () = assert!(ferroboard_core::keys::KeyIcon::ArrowLeft.0 == 18);
const _: () = assert!(ferroboard_core::keys::KeyIcon::ArrowRight.0 == 19);
const _: () = assert!(ferroboard_core::keys::KeyIcon::ArrowUp.0 == 20);
const _: () = assert!(ferroboard_core::keys::KeyIcon::ArrowDown.0 == 21);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Undo.0 == 22);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Redo.0 == 23);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Copy.0 == 24);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Cut.0 == 25);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Paste.0 == 26);
const _: () = assert!(ferroboard_core::keys::KeyIcon::SelectAll.0 == 27);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Select.0 == 28);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Microphone.0 == 29);
const _: () = assert!(ferroboard_core::keys::KeyIcon::OneHandedLeft.0 == 30);
const _: () = assert!(ferroboard_core::keys::KeyIcon::OneHandedRight.0 == 31);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Keyboard.0 == 32);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Tab.0 == 33);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Incognito.0 == 34);
const _: () = assert!(ferroboard_core::keys::KeyIcon::TextEditing.0 == 35);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Home.0 == 36);
const _: () = assert!(ferroboard_core::keys::KeyIcon::End.0 == 37);
const _: () = assert!(ferroboard_core::keys::KeyIcon::DeleteForward.0 == 38);
const _: () = assert!(ferroboard_core::keys::KeyIcon::SplitKeyboard.0 == 39);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Close.0 == 40);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Pin.0 == 41);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Unpin.0 == 42);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Delete.0 == 43);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Recent.0 == 44);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Smiley.0 == 45);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Person.0 == 46);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Nature.0 == 47);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Food.0 == 48);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Travel.0 == 49);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Activity.0 == 50);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Objects.0 == 51);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Symbols.0 == 52);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Flag.0 == 53);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Emoticon.0 == 54);
const _: () = assert!(ferroboard_core::keys::KeyIcon::ChevronLeft.0 == 55);
const _: () = assert!(ferroboard_core::keys::KeyIcon::ChevronRight.0 == 56);
const _: () = assert!(ferroboard_core::keys::KeyIcon::ChevronUp.0 == 57);
const _: () = assert!(ferroboard_core::keys::KeyIcon::ChevronDown.0 == 58);
const _: () = assert!(ferroboard_core::keys::KeyIcon::MoreHorizontal.0 == 59);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Resize.0 == 60);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Autocorrect.0 == 61);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Numpad.0 == 62);
const _: () = assert!(ferroboard_core::keys::KeyIcon::DocumentStart.0 == 63);
const _: () = assert!(ferroboard_core::keys::KeyIcon::DocumentEnd.0 == 64);
const _: () = assert!(ferroboard_core::keys::KeyIcon::ResizeHeight.0 == 65);
const _: () = assert!(ferroboard_core::keys::KeyIcon::Floating.0 == 66);
const _: () = assert!(ferroboard_core::keys::KeyIcon::DragHandle.0 == 67);
impl MarkupTyped for KeyIconMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("KeyIcon", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Keys");
            m.handles = &[ValueType::of::<ferroboard_core::keys::KeyIcon>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::keys::KeyIcon>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "None",
                    value: 0,
                    rust_variant: Some("None"),
                },
                MarkupEnumMember {
                    name: "Backspace",
                    value: 1,
                    rust_variant: Some("Backspace"),
                },
                MarkupEnumMember {
                    name: "Shift",
                    value: 2,
                    rust_variant: Some("Shift"),
                },
                MarkupEnumMember {
                    name: "ShiftActive",
                    value: 3,
                    rust_variant: Some("ShiftActive"),
                },
                MarkupEnumMember {
                    name: "CapsLock",
                    value: 4,
                    rust_variant: Some("CapsLock"),
                },
                MarkupEnumMember {
                    name: "Enter",
                    value: 5,
                    rust_variant: Some("Enter"),
                },
                MarkupEnumMember {
                    name: "Search",
                    value: 6,
                    rust_variant: Some("Search"),
                },
                MarkupEnumMember {
                    name: "Send",
                    value: 7,
                    rust_variant: Some("Send"),
                },
                MarkupEnumMember {
                    name: "Go",
                    value: 8,
                    rust_variant: Some("Go"),
                },
                MarkupEnumMember {
                    name: "Next",
                    value: 9,
                    rust_variant: Some("Next"),
                },
                MarkupEnumMember {
                    name: "Previous",
                    value: 10,
                    rust_variant: Some("Previous"),
                },
                MarkupEnumMember {
                    name: "Done",
                    value: 11,
                    rust_variant: Some("Done"),
                },
                MarkupEnumMember {
                    name: "Space",
                    value: 12,
                    rust_variant: Some("Space"),
                },
                MarkupEnumMember {
                    name: "Language",
                    value: 13,
                    rust_variant: Some("Language"),
                },
                MarkupEnumMember {
                    name: "Emoji",
                    value: 14,
                    rust_variant: Some("Emoji"),
                },
                MarkupEnumMember {
                    name: "Clipboard",
                    value: 15,
                    rust_variant: Some("Clipboard"),
                },
                MarkupEnumMember {
                    name: "Settings",
                    value: 16,
                    rust_variant: Some("Settings"),
                },
                MarkupEnumMember {
                    name: "HideKeyboard",
                    value: 17,
                    rust_variant: Some("HideKeyboard"),
                },
                MarkupEnumMember {
                    name: "ArrowLeft",
                    value: 18,
                    rust_variant: Some("ArrowLeft"),
                },
                MarkupEnumMember {
                    name: "ArrowRight",
                    value: 19,
                    rust_variant: Some("ArrowRight"),
                },
                MarkupEnumMember {
                    name: "ArrowUp",
                    value: 20,
                    rust_variant: Some("ArrowUp"),
                },
                MarkupEnumMember {
                    name: "ArrowDown",
                    value: 21,
                    rust_variant: Some("ArrowDown"),
                },
                MarkupEnumMember {
                    name: "Undo",
                    value: 22,
                    rust_variant: Some("Undo"),
                },
                MarkupEnumMember {
                    name: "Redo",
                    value: 23,
                    rust_variant: Some("Redo"),
                },
                MarkupEnumMember {
                    name: "Copy",
                    value: 24,
                    rust_variant: Some("Copy"),
                },
                MarkupEnumMember {
                    name: "Cut",
                    value: 25,
                    rust_variant: Some("Cut"),
                },
                MarkupEnumMember {
                    name: "Paste",
                    value: 26,
                    rust_variant: Some("Paste"),
                },
                MarkupEnumMember {
                    name: "SelectAll",
                    value: 27,
                    rust_variant: Some("SelectAll"),
                },
                MarkupEnumMember {
                    name: "Select",
                    value: 28,
                    rust_variant: Some("Select"),
                },
                MarkupEnumMember {
                    name: "Microphone",
                    value: 29,
                    rust_variant: Some("Microphone"),
                },
                MarkupEnumMember {
                    name: "OneHandedLeft",
                    value: 30,
                    rust_variant: Some("OneHandedLeft"),
                },
                MarkupEnumMember {
                    name: "OneHandedRight",
                    value: 31,
                    rust_variant: Some("OneHandedRight"),
                },
                MarkupEnumMember {
                    name: "Keyboard",
                    value: 32,
                    rust_variant: Some("Keyboard"),
                },
                MarkupEnumMember {
                    name: "Tab",
                    value: 33,
                    rust_variant: Some("Tab"),
                },
                MarkupEnumMember {
                    name: "Incognito",
                    value: 34,
                    rust_variant: Some("Incognito"),
                },
                MarkupEnumMember {
                    name: "TextEditing",
                    value: 35,
                    rust_variant: Some("TextEditing"),
                },
                MarkupEnumMember {
                    name: "Home",
                    value: 36,
                    rust_variant: Some("Home"),
                },
                MarkupEnumMember {
                    name: "End",
                    value: 37,
                    rust_variant: Some("End"),
                },
                MarkupEnumMember {
                    name: "DeleteForward",
                    value: 38,
                    rust_variant: Some("DeleteForward"),
                },
                MarkupEnumMember {
                    name: "SplitKeyboard",
                    value: 39,
                    rust_variant: Some("SplitKeyboard"),
                },
                MarkupEnumMember {
                    name: "Close",
                    value: 40,
                    rust_variant: Some("Close"),
                },
                MarkupEnumMember {
                    name: "Pin",
                    value: 41,
                    rust_variant: Some("Pin"),
                },
                MarkupEnumMember {
                    name: "Unpin",
                    value: 42,
                    rust_variant: Some("Unpin"),
                },
                MarkupEnumMember {
                    name: "Delete",
                    value: 43,
                    rust_variant: Some("Delete"),
                },
                MarkupEnumMember {
                    name: "Recent",
                    value: 44,
                    rust_variant: Some("Recent"),
                },
                MarkupEnumMember {
                    name: "Smiley",
                    value: 45,
                    rust_variant: Some("Smiley"),
                },
                MarkupEnumMember {
                    name: "Person",
                    value: 46,
                    rust_variant: Some("Person"),
                },
                MarkupEnumMember {
                    name: "Nature",
                    value: 47,
                    rust_variant: Some("Nature"),
                },
                MarkupEnumMember {
                    name: "Food",
                    value: 48,
                    rust_variant: Some("Food"),
                },
                MarkupEnumMember {
                    name: "Travel",
                    value: 49,
                    rust_variant: Some("Travel"),
                },
                MarkupEnumMember {
                    name: "Activity",
                    value: 50,
                    rust_variant: Some("Activity"),
                },
                MarkupEnumMember {
                    name: "Objects",
                    value: 51,
                    rust_variant: Some("Objects"),
                },
                MarkupEnumMember {
                    name: "Symbols",
                    value: 52,
                    rust_variant: Some("Symbols"),
                },
                MarkupEnumMember {
                    name: "Flag",
                    value: 53,
                    rust_variant: Some("Flag"),
                },
                MarkupEnumMember {
                    name: "Emoticon",
                    value: 54,
                    rust_variant: Some("Emoticon"),
                },
                MarkupEnumMember {
                    name: "ChevronLeft",
                    value: 55,
                    rust_variant: Some("ChevronLeft"),
                },
                MarkupEnumMember {
                    name: "ChevronRight",
                    value: 56,
                    rust_variant: Some("ChevronRight"),
                },
                MarkupEnumMember {
                    name: "ChevronUp",
                    value: 57,
                    rust_variant: Some("ChevronUp"),
                },
                MarkupEnumMember {
                    name: "ChevronDown",
                    value: 58,
                    rust_variant: Some("ChevronDown"),
                },
                MarkupEnumMember {
                    name: "MoreHorizontal",
                    value: 59,
                    rust_variant: Some("MoreHorizontal"),
                },
                MarkupEnumMember {
                    name: "Resize",
                    value: 60,
                    rust_variant: Some("Resize"),
                },
                MarkupEnumMember {
                    name: "Autocorrect",
                    value: 61,
                    rust_variant: Some("Autocorrect"),
                },
                MarkupEnumMember {
                    name: "Numpad",
                    value: 62,
                    rust_variant: Some("Numpad"),
                },
                MarkupEnumMember {
                    name: "DocumentStart",
                    value: 63,
                    rust_variant: Some("DocumentStart"),
                },
                MarkupEnumMember {
                    name: "DocumentEnd",
                    value: 64,
                    rust_variant: Some("DocumentEnd"),
                },
                MarkupEnumMember {
                    name: "ResizeHeight",
                    value: 65,
                    rust_variant: Some("ResizeHeight"),
                },
                MarkupEnumMember {
                    name: "Floating",
                    value: 66,
                    rust_variant: Some("Floating"),
                },
                MarkupEnumMember {
                    name: "DragHandle",
                    value: 67,
                    rust_variant: Some("DragHandle"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::keys::KeyIcon(v)) as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::keys::KeyIcon>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct LayoutTypeMetadata;
#[allow(non_upper_case_globals)]
impl LayoutTypeMetadata {
    pub const Characters: ferroboard_core::layouts::LayoutType =
        ferroboard_core::layouts::LayoutType::Characters;
    pub const CharactersMod: ferroboard_core::layouts::LayoutType =
        ferroboard_core::layouts::LayoutType::CharactersMod;
    pub const Extension: ferroboard_core::layouts::LayoutType =
        ferroboard_core::layouts::LayoutType::Extension;
    pub const Numeric: ferroboard_core::layouts::LayoutType =
        ferroboard_core::layouts::LayoutType::Numeric;
    pub const NumericAdvanced: ferroboard_core::layouts::LayoutType =
        ferroboard_core::layouts::LayoutType::NumericAdvanced;
    pub const NumericRow: ferroboard_core::layouts::LayoutType =
        ferroboard_core::layouts::LayoutType::NumericRow;
    pub const Phone: ferroboard_core::layouts::LayoutType =
        ferroboard_core::layouts::LayoutType::Phone;
    pub const Phone2: ferroboard_core::layouts::LayoutType =
        ferroboard_core::layouts::LayoutType::Phone2;
    pub const Symbols: ferroboard_core::layouts::LayoutType =
        ferroboard_core::layouts::LayoutType::Symbols;
    pub const SymbolsMod: ferroboard_core::layouts::LayoutType =
        ferroboard_core::layouts::LayoutType::SymbolsMod;
    pub const Symbols2: ferroboard_core::layouts::LayoutType =
        ferroboard_core::layouts::LayoutType::Symbols2;
    pub const Symbols2Mod: ferroboard_core::layouts::LayoutType =
        ferroboard_core::layouts::LayoutType::Symbols2Mod;
}
const _: () = assert!(ferroboard_core::layouts::LayoutType::Characters.0 == 0);
const _: () = assert!(ferroboard_core::layouts::LayoutType::CharactersMod.0 == 1);
const _: () = assert!(ferroboard_core::layouts::LayoutType::Extension.0 == 2);
const _: () = assert!(ferroboard_core::layouts::LayoutType::Numeric.0 == 3);
const _: () = assert!(ferroboard_core::layouts::LayoutType::NumericAdvanced.0 == 4);
const _: () = assert!(ferroboard_core::layouts::LayoutType::NumericRow.0 == 5);
const _: () = assert!(ferroboard_core::layouts::LayoutType::Phone.0 == 6);
const _: () = assert!(ferroboard_core::layouts::LayoutType::Phone2.0 == 7);
const _: () = assert!(ferroboard_core::layouts::LayoutType::Symbols.0 == 8);
const _: () = assert!(ferroboard_core::layouts::LayoutType::SymbolsMod.0 == 9);
const _: () = assert!(ferroboard_core::layouts::LayoutType::Symbols2.0 == 10);
const _: () = assert!(ferroboard_core::layouts::LayoutType::Symbols2Mod.0 == 11);
impl MarkupTyped for LayoutTypeMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("LayoutType", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Layouts");
            m.handles = &[ValueType::of::<ferroboard_core::layouts::LayoutType>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::layouts::LayoutType>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Characters",
                    value: 0,
                    rust_variant: Some("Characters"),
                },
                MarkupEnumMember {
                    name: "CharactersMod",
                    value: 1,
                    rust_variant: Some("CharactersMod"),
                },
                MarkupEnumMember {
                    name: "Extension",
                    value: 2,
                    rust_variant: Some("Extension"),
                },
                MarkupEnumMember {
                    name: "Numeric",
                    value: 3,
                    rust_variant: Some("Numeric"),
                },
                MarkupEnumMember {
                    name: "NumericAdvanced",
                    value: 4,
                    rust_variant: Some("NumericAdvanced"),
                },
                MarkupEnumMember {
                    name: "NumericRow",
                    value: 5,
                    rust_variant: Some("NumericRow"),
                },
                MarkupEnumMember {
                    name: "Phone",
                    value: 6,
                    rust_variant: Some("Phone"),
                },
                MarkupEnumMember {
                    name: "Phone2",
                    value: 7,
                    rust_variant: Some("Phone2"),
                },
                MarkupEnumMember {
                    name: "Symbols",
                    value: 8,
                    rust_variant: Some("Symbols"),
                },
                MarkupEnumMember {
                    name: "SymbolsMod",
                    value: 9,
                    rust_variant: Some("SymbolsMod"),
                },
                MarkupEnumMember {
                    name: "Symbols2",
                    value: 10,
                    rust_variant: Some("Symbols2"),
                },
                MarkupEnumMember {
                    name: "Symbols2Mod",
                    value: 11,
                    rust_variant: Some("Symbols2Mod"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::layouts::LayoutType(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::layouts::LayoutType>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
#[doc(hidden)]
pub struct SuggestionKindMetadata;
#[allow(non_upper_case_globals)]
impl SuggestionKindMetadata {
    pub const Typed: ferroboard_core::text::SuggestionKind =
        ferroboard_core::text::SuggestionKind::Typed;
    pub const Completion: ferroboard_core::text::SuggestionKind =
        ferroboard_core::text::SuggestionKind::Completion;
    pub const Correction: ferroboard_core::text::SuggestionKind =
        ferroboard_core::text::SuggestionKind::Correction;
    pub const Prediction: ferroboard_core::text::SuggestionKind =
        ferroboard_core::text::SuggestionKind::Prediction;
    pub const Emoji: ferroboard_core::text::SuggestionKind =
        ferroboard_core::text::SuggestionKind::Emoji;
    pub const Glide: ferroboard_core::text::SuggestionKind =
        ferroboard_core::text::SuggestionKind::Glide;
    pub const Clipboard: ferroboard_core::text::SuggestionKind =
        ferroboard_core::text::SuggestionKind::Clipboard;
}
const _: () = assert!(ferroboard_core::text::SuggestionKind::Typed.0 == 0);
const _: () = assert!(ferroboard_core::text::SuggestionKind::Completion.0 == 1);
const _: () = assert!(ferroboard_core::text::SuggestionKind::Correction.0 == 2);
const _: () = assert!(ferroboard_core::text::SuggestionKind::Prediction.0 == 3);
const _: () = assert!(ferroboard_core::text::SuggestionKind::Emoji.0 == 4);
const _: () = assert!(ferroboard_core::text::SuggestionKind::Glide.0 == 5);
const _: () = assert!(ferroboard_core::text::SuggestionKind::Clipboard.0 == 6);
impl MarkupTyped for SuggestionKindMetadata {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new("SuggestionKind", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroBoard.Text");
            m.handles = &[ValueType::of::<ferroboard_core::text::SuggestionKind>];
            m.nullable = Some(ValueType::of::<Option<ferroboard_core::text::SuggestionKind>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Typed",
                    value: 0,
                    rust_variant: Some("Typed"),
                },
                MarkupEnumMember {
                    name: "Completion",
                    value: 1,
                    rust_variant: Some("Completion"),
                },
                MarkupEnumMember {
                    name: "Correction",
                    value: 2,
                    rust_variant: Some("Correction"),
                },
                MarkupEnumMember {
                    name: "Prediction",
                    value: 3,
                    rust_variant: Some("Prediction"),
                },
                MarkupEnumMember {
                    name: "Emoji",
                    value: 4,
                    rust_variant: Some("Emoji"),
                },
                MarkupEnumMember {
                    name: "Glide",
                    value: 5,
                    rust_variant: Some("Glide"),
                },
                MarkupEnumMember {
                    name: "Clipboard",
                    value: 6,
                    rust_variant: Some("Clipboard"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    std::rc::Rc::new(ferroboard_core::text::SuggestionKind(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroboard_core::text::SuggestionKind>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::UtilityKeyActionMetadata,crate::SpaceBarModeMetadata,crate::KeyHintModeMetadata,crate::OneHandedModeMetadata,crate::ThemeModeMetadata,crate::SwipeActionMetadata,crate::SwipeDirectionMetadata,crate::FeedbackKindMetadata,crate::SmartbarLayoutMetadata,crate::EmojiSkinToneMetadata,crate::InputKindMetadata,crate::EnterActionMetadata,crate::CapitalizationModeMetadata,crate::KeyTypeMetadata,crate::KeyboardModeMetadata,crate::KeyVariationMetadata,crate::ShiftStateMetadata,crate::LayoutDirectionMetadata,crate::KeyboardUiModeMetadata,crate::KeyIconMetadata,crate::LayoutTypeMetadata,crate::SuggestionKindMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[
        UtilityKeyActionMetadata::MARKUP,
        SpaceBarModeMetadata::MARKUP,
        KeyHintModeMetadata::MARKUP,
        OneHandedModeMetadata::MARKUP,
        ThemeModeMetadata::MARKUP,
        SwipeActionMetadata::MARKUP,
        SwipeDirectionMetadata::MARKUP,
        FeedbackKindMetadata::MARKUP,
        SmartbarLayoutMetadata::MARKUP,
        EmojiSkinToneMetadata::MARKUP,
        InputKindMetadata::MARKUP,
        EnterActionMetadata::MARKUP,
        CapitalizationModeMetadata::MARKUP,
        KeyTypeMetadata::MARKUP,
        KeyboardModeMetadata::MARKUP,
        KeyVariationMetadata::MARKUP,
        ShiftStateMetadata::MARKUP,
        LayoutDirectionMetadata::MARKUP,
        KeyboardUiModeMetadata::MARKUP,
        KeyIconMetadata::MARKUP,
        LayoutTypeMetadata::MARKUP,
        SuggestionKindMetadata::MARKUP,
    ]);
    ferroui_base::data::core::ValueTypes::register_global(register_values);
}
fn register_values() {
    ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<ferroboard_core::settings::UtilityKeyAction>(),"::ferroboard_core::settings::UtilityKeyAction"),(||std::any::TypeId::of::<ferroboard_core::settings::SpaceBarMode>(),"::ferroboard_core::settings::SpaceBarMode"),(||std::any::TypeId::of::<ferroboard_core::settings::KeyHintMode>(),"::ferroboard_core::settings::KeyHintMode"),(||std::any::TypeId::of::<ferroboard_core::settings::OneHandedMode>(),"::ferroboard_core::settings::OneHandedMode"),(||std::any::TypeId::of::<ferroboard_core::settings::ThemeMode>(),"::ferroboard_core::settings::ThemeMode"),(||std::any::TypeId::of::<ferroboard_core::settings::SwipeAction>(),"::ferroboard_core::settings::SwipeAction"),(||std::any::TypeId::of::<ferroboard_core::settings::SwipeDirection>(),"::ferroboard_core::settings::SwipeDirection"),(||std::any::TypeId::of::<ferroboard_core::settings::FeedbackKind>(),"::ferroboard_core::settings::FeedbackKind"),(||std::any::TypeId::of::<ferroboard_core::settings::SmartbarLayout>(),"::ferroboard_core::settings::SmartbarLayout"),(||std::any::TypeId::of::<ferroboard_core::emoji::EmojiSkinTone>(),"::ferroboard_core::emoji::EmojiSkinTone"),(||std::any::TypeId::of::<ferroboard_core::editor::InputKind>(),"::ferroboard_core::editor::InputKind"),(||std::any::TypeId::of::<ferroboard_core::editor::EnterAction>(),"::ferroboard_core::editor::EnterAction"),(||std::any::TypeId::of::<ferroboard_core::editor::CapitalizationMode>(),"::ferroboard_core::editor::CapitalizationMode"),(||std::any::TypeId::of::<ferroboard_core::keys::KeyType>(),"::ferroboard_core::keys::KeyType"),(||std::any::TypeId::of::<ferroboard_core::keys::KeyboardMode>(),"::ferroboard_core::keys::KeyboardMode"),(||std::any::TypeId::of::<ferroboard_core::keys::KeyVariation>(),"::ferroboard_core::keys::KeyVariation"),(||std::any::TypeId::of::<ferroboard_core::keys::ShiftState>(),"::ferroboard_core::keys::ShiftState"),(||std::any::TypeId::of::<ferroboard_core::keys::LayoutDirection>(),"::ferroboard_core::keys::LayoutDirection"),(||std::any::TypeId::of::<ferroboard_core::keys::KeyboardUiMode>(),"::ferroboard_core::keys::KeyboardUiMode"),(||std::any::TypeId::of::<ferroboard_core::keys::KeyIcon>(),"::ferroboard_core::keys::KeyIcon"),(||std::any::TypeId::of::<ferroboard_core::layouts::LayoutType>(),"::ferroboard_core::layouts::LayoutType"),(||std::any::TypeId::of::<ferroboard_core::text::SuggestionKind>(),"::ferroboard_core::text::SuggestionKind")]);}
    ferroui_base::data::core::ValueTypes::register_nullable::<
        ferroboard_core::settings::UtilityKeyAction,
    >();
    ferroui_base::data::core::ValueTypes::register_nullable::<
        ferroboard_core::settings::SpaceBarMode,
    >();
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::settings::KeyHintMode>(
    );
    ferroui_base::data::core::ValueTypes::register_nullable::<
        ferroboard_core::settings::OneHandedMode,
    >();
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::settings::ThemeMode>(
    );
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::settings::SwipeAction>(
    );
    ferroui_base::data::core::ValueTypes::register_nullable::<
        ferroboard_core::settings::SwipeDirection,
    >();
    ferroui_base::data::core::ValueTypes::register_nullable::<
        ferroboard_core::settings::FeedbackKind,
    >();
    ferroui_base::data::core::ValueTypes::register_nullable::<
        ferroboard_core::settings::SmartbarLayout,
    >();
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::emoji::EmojiSkinTone>(
    );
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::editor::InputKind>();
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::editor::EnterAction>(
    );
    ferroui_base::data::core::ValueTypes::register_nullable::<
        ferroboard_core::editor::CapitalizationMode,
    >();
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::keys::KeyType>();
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::keys::KeyboardMode>(
    );
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::keys::KeyVariation>(
    );
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::keys::ShiftState>();
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::keys::LayoutDirection>(
    );
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::keys::KeyboardUiMode>(
    );
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::keys::KeyIcon>();
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::layouts::LayoutType>(
    );
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroboard_core::text::SuggestionKind>(
    );
}
