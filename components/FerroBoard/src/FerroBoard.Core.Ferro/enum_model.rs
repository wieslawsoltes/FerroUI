// Generated foreign enum facts; all compiler parsing/emission stays in FerroUI.
use ferroui_build_scan::model::{AssemblyModel, EnumMemberModel, RustType, TypeKind, TypeModel};
pub fn append(model: &mut AssemblyModel) {
    {
        let mut t = TypeModel::new(
            "UtilityKeyAction",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::settings::UtilityKeyAction"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Settings".into();
        t.explicit_namespace = Some("FerroBoard.Settings".into());
        t.public_path = Some("::ferroboard_core_ferro::UtilityKeyActionMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::settings::UtilityKeyAction",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "Emoji".into(),
            rust_variant: Some("Emoji".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "LanguageSwitch".into(),
            rust_variant: Some("LanguageSwitch".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Dynamic".into(),
            rust_variant: Some("Dynamic".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Hidden".into(),
            rust_variant: Some("Hidden".into()),
            rust_value: None,
            value: Some(3),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "SpaceBarMode",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::settings::SpaceBarMode"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Settings".into();
        t.explicit_namespace = Some("FerroBoard.Settings".into());
        t.public_path = Some("::ferroboard_core_ferro::SpaceBarModeMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::settings::SpaceBarMode",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "Nothing".into(),
            rust_variant: Some("Nothing".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "CurrentLanguage".into(),
            rust_variant: Some("CurrentLanguage".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SpaceBarKey".into(),
            rust_variant: Some("SpaceBarKey".into()),
            rust_value: None,
            value: Some(2),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "KeyHintMode",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::settings::KeyHintMode"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Settings".into();
        t.explicit_namespace = Some("FerroBoard.Settings".into());
        t.public_path = Some("::ferroboard_core_ferro::KeyHintModeMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::settings::KeyHintMode",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "Disabled".into(),
            rust_variant: Some("Disabled".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "AccentPriority".into(),
            rust_variant: Some("AccentPriority".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "HintPriority".into(),
            rust_variant: Some("HintPriority".into()),
            rust_value: None,
            value: Some(2),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "OneHandedMode",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::settings::OneHandedMode"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Settings".into();
        t.explicit_namespace = Some("FerroBoard.Settings".into());
        t.public_path = Some("::ferroboard_core_ferro::OneHandedModeMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::settings::OneHandedMode",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "Off".into(),
            rust_variant: Some("Off".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Left".into(),
            rust_variant: Some("Left".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Right".into(),
            rust_variant: Some("Right".into()),
            rust_value: None,
            value: Some(2),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "ThemeMode",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::settings::ThemeMode"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Settings".into();
        t.explicit_namespace = Some("FerroBoard.Settings".into());
        t.public_path = Some("::ferroboard_core_ferro::ThemeModeMetadata".into());
        t.handles = vec![RustType::resolved("::ferroboard_core::settings::ThemeMode")];
        t.enum_members.push(EnumMemberModel {
            name: "FollowSystem".into(),
            rust_variant: Some("FollowSystem".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Light".into(),
            rust_variant: Some("Light".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Dark".into(),
            rust_variant: Some("Dark".into()),
            rust_value: None,
            value: Some(2),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "SwipeAction",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::settings::SwipeAction"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Settings".into();
        t.explicit_namespace = Some("FerroBoard.Settings".into());
        t.public_path = Some("::ferroboard_core_ferro::SwipeActionMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::settings::SwipeAction",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "NoAction".into(),
            rust_variant: Some("NoAction".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "CycleToPreviousKeyboardMode".into(),
            rust_variant: Some("CycleToPreviousKeyboardMode".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "CycleToNextKeyboardMode".into(),
            rust_variant: Some("CycleToNextKeyboardMode".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "DeleteCharacter".into(),
            rust_variant: Some("DeleteCharacter".into()),
            rust_value: None,
            value: Some(3),
        });
        t.enum_members.push(EnumMemberModel {
            name: "DeleteCharactersPrecisely".into(),
            rust_variant: Some("DeleteCharactersPrecisely".into()),
            rust_value: None,
            value: Some(4),
        });
        t.enum_members.push(EnumMemberModel {
            name: "DeleteWord".into(),
            rust_variant: Some("DeleteWord".into()),
            rust_value: None,
            value: Some(5),
        });
        t.enum_members.push(EnumMemberModel {
            name: "DeleteWordsPrecisely".into(),
            rust_variant: Some("DeleteWordsPrecisely".into()),
            rust_value: None,
            value: Some(6),
        });
        t.enum_members.push(EnumMemberModel {
            name: "HideKeyboard".into(),
            rust_variant: Some("HideKeyboard".into()),
            rust_value: None,
            value: Some(7),
        });
        t.enum_members.push(EnumMemberModel {
            name: "InsertSpace".into(),
            rust_variant: Some("InsertSpace".into()),
            rust_value: None,
            value: Some(8),
        });
        t.enum_members.push(EnumMemberModel {
            name: "MoveCursorUp".into(),
            rust_variant: Some("MoveCursorUp".into()),
            rust_value: None,
            value: Some(9),
        });
        t.enum_members.push(EnumMemberModel {
            name: "MoveCursorDown".into(),
            rust_variant: Some("MoveCursorDown".into()),
            rust_value: None,
            value: Some(10),
        });
        t.enum_members.push(EnumMemberModel {
            name: "MoveCursorLeft".into(),
            rust_variant: Some("MoveCursorLeft".into()),
            rust_value: None,
            value: Some(11),
        });
        t.enum_members.push(EnumMemberModel {
            name: "MoveCursorRight".into(),
            rust_variant: Some("MoveCursorRight".into()),
            rust_value: None,
            value: Some(12),
        });
        t.enum_members.push(EnumMemberModel {
            name: "MoveCursorStartOfLine".into(),
            rust_variant: Some("MoveCursorStartOfLine".into()),
            rust_value: None,
            value: Some(13),
        });
        t.enum_members.push(EnumMemberModel {
            name: "MoveCursorEndOfLine".into(),
            rust_variant: Some("MoveCursorEndOfLine".into()),
            rust_value: None,
            value: Some(14),
        });
        t.enum_members.push(EnumMemberModel {
            name: "MoveCursorStartOfPage".into(),
            rust_variant: Some("MoveCursorStartOfPage".into()),
            rust_value: None,
            value: Some(15),
        });
        t.enum_members.push(EnumMemberModel {
            name: "MoveCursorEndOfPage".into(),
            rust_variant: Some("MoveCursorEndOfPage".into()),
            rust_value: None,
            value: Some(16),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Redo".into(),
            rust_variant: Some("Redo".into()),
            rust_value: None,
            value: Some(17),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SelectCharactersPrecisely".into(),
            rust_variant: Some("SelectCharactersPrecisely".into()),
            rust_value: None,
            value: Some(18),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SelectWordsPrecisely".into(),
            rust_variant: Some("SelectWordsPrecisely".into()),
            rust_value: None,
            value: Some(19),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Shift".into(),
            rust_variant: Some("Shift".into()),
            rust_value: None,
            value: Some(20),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ShowSubtypePicker".into(),
            rust_variant: Some("ShowSubtypePicker".into()),
            rust_value: None,
            value: Some(21),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SwitchToPrevSubtype".into(),
            rust_variant: Some("SwitchToPrevSubtype".into()),
            rust_value: None,
            value: Some(22),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SwitchToNextSubtype".into(),
            rust_variant: Some("SwitchToNextSubtype".into()),
            rust_value: None,
            value: Some(23),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SwitchToClipboardContext".into(),
            rust_variant: Some("SwitchToClipboardContext".into()),
            rust_value: None,
            value: Some(24),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SwitchToMediaContext".into(),
            rust_variant: Some("SwitchToMediaContext".into()),
            rust_value: None,
            value: Some(25),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SwitchToEditingContext".into(),
            rust_variant: Some("SwitchToEditingContext".into()),
            rust_value: None,
            value: Some(26),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ToggleOneHandedMode".into(),
            rust_variant: Some("ToggleOneHandedMode".into()),
            rust_value: None,
            value: Some(27),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ToggleSmartbarVisibility".into(),
            rust_variant: Some("ToggleSmartbarVisibility".into()),
            rust_value: None,
            value: Some(28),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Undo".into(),
            rust_variant: Some("Undo".into()),
            rust_value: None,
            value: Some(29),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "SwipeDirection",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::settings::SwipeDirection"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Settings".into();
        t.explicit_namespace = Some("FerroBoard.Settings".into());
        t.public_path = Some("::ferroboard_core_ferro::SwipeDirectionMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::settings::SwipeDirection",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "None".into(),
            rust_variant: Some("None".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Up".into(),
            rust_variant: Some("Up".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Down".into(),
            rust_variant: Some("Down".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Left".into(),
            rust_variant: Some("Left".into()),
            rust_value: None,
            value: Some(3),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Right".into(),
            rust_variant: Some("Right".into()),
            rust_value: None,
            value: Some(4),
        });
        t.enum_members.push(EnumMemberModel {
            name: "UpLeft".into(),
            rust_variant: Some("UpLeft".into()),
            rust_value: None,
            value: Some(5),
        });
        t.enum_members.push(EnumMemberModel {
            name: "UpRight".into(),
            rust_variant: Some("UpRight".into()),
            rust_value: None,
            value: Some(6),
        });
        t.enum_members.push(EnumMemberModel {
            name: "DownLeft".into(),
            rust_variant: Some("DownLeft".into()),
            rust_value: None,
            value: Some(7),
        });
        t.enum_members.push(EnumMemberModel {
            name: "DownRight".into(),
            rust_variant: Some("DownRight".into()),
            rust_value: None,
            value: Some(8),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "FeedbackKind",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::settings::FeedbackKind"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Settings".into();
        t.explicit_namespace = Some("FerroBoard.Settings".into());
        t.public_path = Some("::ferroboard_core_ferro::FeedbackKindMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::settings::FeedbackKind",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "KeyPress".into(),
            rust_variant: Some("KeyPress".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "KeyPressDelete".into(),
            rust_variant: Some("KeyPressDelete".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "KeyPressSpace".into(),
            rust_variant: Some("KeyPressSpace".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "KeyPressEnter".into(),
            rust_variant: Some("KeyPressEnter".into()),
            rust_value: None,
            value: Some(3),
        });
        t.enum_members.push(EnumMemberModel {
            name: "KeyPressFunction".into(),
            rust_variant: Some("KeyPressFunction".into()),
            rust_value: None,
            value: Some(4),
        });
        t.enum_members.push(EnumMemberModel {
            name: "LongPress".into(),
            rust_variant: Some("LongPress".into()),
            rust_value: None,
            value: Some(5),
        });
        t.enum_members.push(EnumMemberModel {
            name: "KeyRepeat".into(),
            rust_variant: Some("KeyRepeat".into()),
            rust_value: None,
            value: Some(6),
        });
        t.enum_members.push(EnumMemberModel {
            name: "GestureStep".into(),
            rust_variant: Some("GestureStep".into()),
            rust_value: None,
            value: Some(7),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "SmartbarLayout",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::settings::SmartbarLayout"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Settings".into();
        t.explicit_namespace = Some("FerroBoard.Settings".into());
        t.public_path = Some("::ferroboard_core_ferro::SmartbarLayoutMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::settings::SmartbarLayout",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "SuggestionsWithActions".into(),
            rust_variant: Some("SuggestionsWithActions".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SuggestionsOnly".into(),
            rust_variant: Some("SuggestionsOnly".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ActionsOnly".into(),
            rust_variant: Some("ActionsOnly".into()),
            rust_value: None,
            value: Some(2),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "EmojiSkinTone",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::emoji::EmojiSkinTone"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Emoji".into();
        t.explicit_namespace = Some("FerroBoard.Emoji".into());
        t.public_path = Some("::ferroboard_core_ferro::EmojiSkinToneMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::emoji::EmojiSkinTone",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "Default".into(),
            rust_variant: Some("Default".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Light".into(),
            rust_variant: Some("Light".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "MediumLight".into(),
            rust_variant: Some("MediumLight".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Medium".into(),
            rust_variant: Some("Medium".into()),
            rust_value: None,
            value: Some(3),
        });
        t.enum_members.push(EnumMemberModel {
            name: "MediumDark".into(),
            rust_variant: Some("MediumDark".into()),
            rust_value: None,
            value: Some(4),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Dark".into(),
            rust_variant: Some("Dark".into()),
            rust_value: None,
            value: Some(5),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "InputKind",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::editor::InputKind"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Editor".into();
        t.explicit_namespace = Some("FerroBoard.Editor".into());
        t.public_path = Some("::ferroboard_core_ferro::InputKindMetadata".into());
        t.handles = vec![RustType::resolved("::ferroboard_core::editor::InputKind")];
        t.enum_members.push(EnumMemberModel {
            name: "Text".into(),
            rust_variant: Some("Text".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Number".into(),
            rust_variant: Some("Number".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Decimal".into(),
            rust_variant: Some("Decimal".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Phone".into(),
            rust_variant: Some("Phone".into()),
            rust_value: None,
            value: Some(3),
        });
        t.enum_members.push(EnumMemberModel {
            name: "DateTime".into(),
            rust_variant: Some("DateTime".into()),
            rust_value: None,
            value: Some(4),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Email".into(),
            rust_variant: Some("Email".into()),
            rust_value: None,
            value: Some(5),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Uri".into(),
            rust_variant: Some("Uri".into()),
            rust_value: None,
            value: Some(6),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Password".into(),
            rust_variant: Some("Password".into()),
            rust_value: None,
            value: Some(7),
        });
        t.enum_members.push(EnumMemberModel {
            name: "NumericPassword".into(),
            rust_variant: Some("NumericPassword".into()),
            rust_value: None,
            value: Some(8),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Search".into(),
            rust_variant: Some("Search".into()),
            rust_value: None,
            value: Some(9),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Chat".into(),
            rust_variant: Some("Chat".into()),
            rust_value: None,
            value: Some(10),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "EnterAction",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::editor::EnterAction"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Editor".into();
        t.explicit_namespace = Some("FerroBoard.Editor".into());
        t.public_path = Some("::ferroboard_core_ferro::EnterActionMetadata".into());
        t.handles = vec![RustType::resolved("::ferroboard_core::editor::EnterAction")];
        t.enum_members.push(EnumMemberModel {
            name: "Default".into(),
            rust_variant: Some("Default".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "NewLine".into(),
            rust_variant: Some("NewLine".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Go".into(),
            rust_variant: Some("Go".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Search".into(),
            rust_variant: Some("Search".into()),
            rust_value: None,
            value: Some(3),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Send".into(),
            rust_variant: Some("Send".into()),
            rust_value: None,
            value: Some(4),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Next".into(),
            rust_variant: Some("Next".into()),
            rust_value: None,
            value: Some(5),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Previous".into(),
            rust_variant: Some("Previous".into()),
            rust_value: None,
            value: Some(6),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Done".into(),
            rust_variant: Some("Done".into()),
            rust_value: None,
            value: Some(7),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "CapitalizationMode",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::editor::CapitalizationMode"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Editor".into();
        t.explicit_namespace = Some("FerroBoard.Editor".into());
        t.public_path = Some("::ferroboard_core_ferro::CapitalizationModeMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::editor::CapitalizationMode",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "None".into(),
            rust_variant: Some("None".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Characters".into(),
            rust_variant: Some("Characters".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Words".into(),
            rust_variant: Some("Words".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Sentences".into(),
            rust_variant: Some("Sentences".into()),
            rust_value: None,
            value: Some(3),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "KeyType",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::keys::KeyType"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Keys".into();
        t.explicit_namespace = Some("FerroBoard.Keys".into());
        t.public_path = Some("::ferroboard_core_ferro::KeyTypeMetadata".into());
        t.handles = vec![RustType::resolved("::ferroboard_core::keys::KeyType")];
        t.enum_members.push(EnumMemberModel {
            name: "Character".into(),
            rust_variant: Some("Character".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "EnterEditing".into(),
            rust_variant: Some("EnterEditing".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Function".into(),
            rust_variant: Some("Function".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Lock".into(),
            rust_variant: Some("Lock".into()),
            rust_value: None,
            value: Some(3),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Modifier".into(),
            rust_variant: Some("Modifier".into()),
            rust_value: None,
            value: Some(4),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Navigation".into(),
            rust_variant: Some("Navigation".into()),
            rust_value: None,
            value: Some(5),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SystemGui".into(),
            rust_variant: Some("SystemGui".into()),
            rust_value: None,
            value: Some(6),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Numeric".into(),
            rust_variant: Some("Numeric".into()),
            rust_value: None,
            value: Some(7),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Placeholder".into(),
            rust_variant: Some("Placeholder".into()),
            rust_value: None,
            value: Some(8),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Unspecified".into(),
            rust_variant: Some("Unspecified".into()),
            rust_value: None,
            value: Some(9),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "KeyboardMode",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::keys::KeyboardMode"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Keys".into();
        t.explicit_namespace = Some("FerroBoard.Keys".into());
        t.public_path = Some("::ferroboard_core_ferro::KeyboardModeMetadata".into());
        t.handles = vec![RustType::resolved("::ferroboard_core::keys::KeyboardMode")];
        t.enum_members.push(EnumMemberModel {
            name: "Characters".into(),
            rust_variant: Some("Characters".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Symbols".into(),
            rust_variant: Some("Symbols".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Symbols2".into(),
            rust_variant: Some("Symbols2".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Numeric".into(),
            rust_variant: Some("Numeric".into()),
            rust_value: None,
            value: Some(3),
        });
        t.enum_members.push(EnumMemberModel {
            name: "NumericAdvanced".into(),
            rust_variant: Some("NumericAdvanced".into()),
            rust_value: None,
            value: Some(4),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Phone".into(),
            rust_variant: Some("Phone".into()),
            rust_value: None,
            value: Some(5),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Phone2".into(),
            rust_variant: Some("Phone2".into()),
            rust_value: None,
            value: Some(6),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "KeyVariation",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::keys::KeyVariation"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Keys".into();
        t.explicit_namespace = Some("FerroBoard.Keys".into());
        t.public_path = Some("::ferroboard_core_ferro::KeyVariationMetadata".into());
        t.handles = vec![RustType::resolved("::ferroboard_core::keys::KeyVariation")];
        t.enum_members.push(EnumMemberModel {
            name: "All".into(),
            rust_variant: Some("All".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Normal".into(),
            rust_variant: Some("Normal".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "EmailAddress".into(),
            rust_variant: Some("EmailAddress".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Uri".into(),
            rust_variant: Some("Uri".into()),
            rust_value: None,
            value: Some(3),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Password".into(),
            rust_variant: Some("Password".into()),
            rust_value: None,
            value: Some(4),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "ShiftState",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::keys::ShiftState"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Keys".into();
        t.explicit_namespace = Some("FerroBoard.Keys".into());
        t.public_path = Some("::ferroboard_core_ferro::ShiftStateMetadata".into());
        t.handles = vec![RustType::resolved("::ferroboard_core::keys::ShiftState")];
        t.enum_members.push(EnumMemberModel {
            name: "Unshifted".into(),
            rust_variant: Some("Unshifted".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ShiftedManual".into(),
            rust_variant: Some("ShiftedManual".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ShiftedAutomatic".into(),
            rust_variant: Some("ShiftedAutomatic".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "CapsLock".into(),
            rust_variant: Some("CapsLock".into()),
            rust_value: None,
            value: Some(3),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "LayoutDirection",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::keys::LayoutDirection"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Keys".into();
        t.explicit_namespace = Some("FerroBoard.Keys".into());
        t.public_path = Some("::ferroboard_core_ferro::LayoutDirectionMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::keys::LayoutDirection",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "Ltr".into(),
            rust_variant: Some("Ltr".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Rtl".into(),
            rust_variant: Some("Rtl".into()),
            rust_value: None,
            value: Some(1),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "KeyboardUiMode",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::keys::KeyboardUiMode"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Keys".into();
        t.explicit_namespace = Some("FerroBoard.Keys".into());
        t.public_path = Some("::ferroboard_core_ferro::KeyboardUiModeMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::keys::KeyboardUiMode",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "Text".into(),
            rust_variant: Some("Text".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Media".into(),
            rust_variant: Some("Media".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Clipboard".into(),
            rust_variant: Some("Clipboard".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Editing".into(),
            rust_variant: Some("Editing".into()),
            rust_value: None,
            value: Some(3),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "KeyIcon",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::keys::KeyIcon"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Keys".into();
        t.explicit_namespace = Some("FerroBoard.Keys".into());
        t.public_path = Some("::ferroboard_core_ferro::KeyIconMetadata".into());
        t.handles = vec![RustType::resolved("::ferroboard_core::keys::KeyIcon")];
        t.enum_members.push(EnumMemberModel {
            name: "None".into(),
            rust_variant: Some("None".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Backspace".into(),
            rust_variant: Some("Backspace".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Shift".into(),
            rust_variant: Some("Shift".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ShiftActive".into(),
            rust_variant: Some("ShiftActive".into()),
            rust_value: None,
            value: Some(3),
        });
        t.enum_members.push(EnumMemberModel {
            name: "CapsLock".into(),
            rust_variant: Some("CapsLock".into()),
            rust_value: None,
            value: Some(4),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Enter".into(),
            rust_variant: Some("Enter".into()),
            rust_value: None,
            value: Some(5),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Search".into(),
            rust_variant: Some("Search".into()),
            rust_value: None,
            value: Some(6),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Send".into(),
            rust_variant: Some("Send".into()),
            rust_value: None,
            value: Some(7),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Go".into(),
            rust_variant: Some("Go".into()),
            rust_value: None,
            value: Some(8),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Next".into(),
            rust_variant: Some("Next".into()),
            rust_value: None,
            value: Some(9),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Previous".into(),
            rust_variant: Some("Previous".into()),
            rust_value: None,
            value: Some(10),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Done".into(),
            rust_variant: Some("Done".into()),
            rust_value: None,
            value: Some(11),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Space".into(),
            rust_variant: Some("Space".into()),
            rust_value: None,
            value: Some(12),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Language".into(),
            rust_variant: Some("Language".into()),
            rust_value: None,
            value: Some(13),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Emoji".into(),
            rust_variant: Some("Emoji".into()),
            rust_value: None,
            value: Some(14),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Clipboard".into(),
            rust_variant: Some("Clipboard".into()),
            rust_value: None,
            value: Some(15),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Settings".into(),
            rust_variant: Some("Settings".into()),
            rust_value: None,
            value: Some(16),
        });
        t.enum_members.push(EnumMemberModel {
            name: "HideKeyboard".into(),
            rust_variant: Some("HideKeyboard".into()),
            rust_value: None,
            value: Some(17),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ArrowLeft".into(),
            rust_variant: Some("ArrowLeft".into()),
            rust_value: None,
            value: Some(18),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ArrowRight".into(),
            rust_variant: Some("ArrowRight".into()),
            rust_value: None,
            value: Some(19),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ArrowUp".into(),
            rust_variant: Some("ArrowUp".into()),
            rust_value: None,
            value: Some(20),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ArrowDown".into(),
            rust_variant: Some("ArrowDown".into()),
            rust_value: None,
            value: Some(21),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Undo".into(),
            rust_variant: Some("Undo".into()),
            rust_value: None,
            value: Some(22),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Redo".into(),
            rust_variant: Some("Redo".into()),
            rust_value: None,
            value: Some(23),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Copy".into(),
            rust_variant: Some("Copy".into()),
            rust_value: None,
            value: Some(24),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Cut".into(),
            rust_variant: Some("Cut".into()),
            rust_value: None,
            value: Some(25),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Paste".into(),
            rust_variant: Some("Paste".into()),
            rust_value: None,
            value: Some(26),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SelectAll".into(),
            rust_variant: Some("SelectAll".into()),
            rust_value: None,
            value: Some(27),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Select".into(),
            rust_variant: Some("Select".into()),
            rust_value: None,
            value: Some(28),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Microphone".into(),
            rust_variant: Some("Microphone".into()),
            rust_value: None,
            value: Some(29),
        });
        t.enum_members.push(EnumMemberModel {
            name: "OneHandedLeft".into(),
            rust_variant: Some("OneHandedLeft".into()),
            rust_value: None,
            value: Some(30),
        });
        t.enum_members.push(EnumMemberModel {
            name: "OneHandedRight".into(),
            rust_variant: Some("OneHandedRight".into()),
            rust_value: None,
            value: Some(31),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Keyboard".into(),
            rust_variant: Some("Keyboard".into()),
            rust_value: None,
            value: Some(32),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Tab".into(),
            rust_variant: Some("Tab".into()),
            rust_value: None,
            value: Some(33),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Incognito".into(),
            rust_variant: Some("Incognito".into()),
            rust_value: None,
            value: Some(34),
        });
        t.enum_members.push(EnumMemberModel {
            name: "TextEditing".into(),
            rust_variant: Some("TextEditing".into()),
            rust_value: None,
            value: Some(35),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Home".into(),
            rust_variant: Some("Home".into()),
            rust_value: None,
            value: Some(36),
        });
        t.enum_members.push(EnumMemberModel {
            name: "End".into(),
            rust_variant: Some("End".into()),
            rust_value: None,
            value: Some(37),
        });
        t.enum_members.push(EnumMemberModel {
            name: "DeleteForward".into(),
            rust_variant: Some("DeleteForward".into()),
            rust_value: None,
            value: Some(38),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SplitKeyboard".into(),
            rust_variant: Some("SplitKeyboard".into()),
            rust_value: None,
            value: Some(39),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Close".into(),
            rust_variant: Some("Close".into()),
            rust_value: None,
            value: Some(40),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Pin".into(),
            rust_variant: Some("Pin".into()),
            rust_value: None,
            value: Some(41),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Unpin".into(),
            rust_variant: Some("Unpin".into()),
            rust_value: None,
            value: Some(42),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Delete".into(),
            rust_variant: Some("Delete".into()),
            rust_value: None,
            value: Some(43),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Recent".into(),
            rust_variant: Some("Recent".into()),
            rust_value: None,
            value: Some(44),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Smiley".into(),
            rust_variant: Some("Smiley".into()),
            rust_value: None,
            value: Some(45),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Person".into(),
            rust_variant: Some("Person".into()),
            rust_value: None,
            value: Some(46),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Nature".into(),
            rust_variant: Some("Nature".into()),
            rust_value: None,
            value: Some(47),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Food".into(),
            rust_variant: Some("Food".into()),
            rust_value: None,
            value: Some(48),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Travel".into(),
            rust_variant: Some("Travel".into()),
            rust_value: None,
            value: Some(49),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Activity".into(),
            rust_variant: Some("Activity".into()),
            rust_value: None,
            value: Some(50),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Objects".into(),
            rust_variant: Some("Objects".into()),
            rust_value: None,
            value: Some(51),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Symbols".into(),
            rust_variant: Some("Symbols".into()),
            rust_value: None,
            value: Some(52),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Flag".into(),
            rust_variant: Some("Flag".into()),
            rust_value: None,
            value: Some(53),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Emoticon".into(),
            rust_variant: Some("Emoticon".into()),
            rust_value: None,
            value: Some(54),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ChevronLeft".into(),
            rust_variant: Some("ChevronLeft".into()),
            rust_value: None,
            value: Some(55),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ChevronRight".into(),
            rust_variant: Some("ChevronRight".into()),
            rust_value: None,
            value: Some(56),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ChevronUp".into(),
            rust_variant: Some("ChevronUp".into()),
            rust_value: None,
            value: Some(57),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ChevronDown".into(),
            rust_variant: Some("ChevronDown".into()),
            rust_value: None,
            value: Some(58),
        });
        t.enum_members.push(EnumMemberModel {
            name: "MoreHorizontal".into(),
            rust_variant: Some("MoreHorizontal".into()),
            rust_value: None,
            value: Some(59),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Resize".into(),
            rust_variant: Some("Resize".into()),
            rust_value: None,
            value: Some(60),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Autocorrect".into(),
            rust_variant: Some("Autocorrect".into()),
            rust_value: None,
            value: Some(61),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Numpad".into(),
            rust_variant: Some("Numpad".into()),
            rust_value: None,
            value: Some(62),
        });
        t.enum_members.push(EnumMemberModel {
            name: "DocumentStart".into(),
            rust_variant: Some("DocumentStart".into()),
            rust_value: None,
            value: Some(63),
        });
        t.enum_members.push(EnumMemberModel {
            name: "DocumentEnd".into(),
            rust_variant: Some("DocumentEnd".into()),
            rust_value: None,
            value: Some(64),
        });
        t.enum_members.push(EnumMemberModel {
            name: "ResizeHeight".into(),
            rust_variant: Some("ResizeHeight".into()),
            rust_value: None,
            value: Some(65),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Floating".into(),
            rust_variant: Some("Floating".into()),
            rust_value: None,
            value: Some(66),
        });
        t.enum_members.push(EnumMemberModel {
            name: "DragHandle".into(),
            rust_variant: Some("DragHandle".into()),
            rust_value: None,
            value: Some(67),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "LayoutType",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::layouts::LayoutType"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Layouts".into();
        t.explicit_namespace = Some("FerroBoard.Layouts".into());
        t.public_path = Some("::ferroboard_core_ferro::LayoutTypeMetadata".into());
        t.handles = vec![RustType::resolved("::ferroboard_core::layouts::LayoutType")];
        t.enum_members.push(EnumMemberModel {
            name: "Characters".into(),
            rust_variant: Some("Characters".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "CharactersMod".into(),
            rust_variant: Some("CharactersMod".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Extension".into(),
            rust_variant: Some("Extension".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Numeric".into(),
            rust_variant: Some("Numeric".into()),
            rust_value: None,
            value: Some(3),
        });
        t.enum_members.push(EnumMemberModel {
            name: "NumericAdvanced".into(),
            rust_variant: Some("NumericAdvanced".into()),
            rust_value: None,
            value: Some(4),
        });
        t.enum_members.push(EnumMemberModel {
            name: "NumericRow".into(),
            rust_variant: Some("NumericRow".into()),
            rust_value: None,
            value: Some(5),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Phone".into(),
            rust_variant: Some("Phone".into()),
            rust_value: None,
            value: Some(6),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Phone2".into(),
            rust_variant: Some("Phone2".into()),
            rust_value: None,
            value: Some(7),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Symbols".into(),
            rust_variant: Some("Symbols".into()),
            rust_value: None,
            value: Some(8),
        });
        t.enum_members.push(EnumMemberModel {
            name: "SymbolsMod".into(),
            rust_variant: Some("SymbolsMod".into()),
            rust_value: None,
            value: Some(9),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Symbols2".into(),
            rust_variant: Some("Symbols2".into()),
            rust_value: None,
            value: Some(10),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Symbols2Mod".into(),
            rust_variant: Some("Symbols2Mod".into()),
            rust_value: None,
            value: Some(11),
        });
        model.types.push(t);
    }
    {
        let mut t = TypeModel::new(
            "SuggestionKind",
            TypeKind::Enum,
            RustType::resolved("::ferroboard_core::text::SuggestionKind"),
            "ferroboard_core_ferro",
        );
        t.namespace = "FerroBoard.Text".into();
        t.explicit_namespace = Some("FerroBoard.Text".into());
        t.public_path = Some("::ferroboard_core_ferro::SuggestionKindMetadata".into());
        t.handles = vec![RustType::resolved(
            "::ferroboard_core::text::SuggestionKind",
        )];
        t.enum_members.push(EnumMemberModel {
            name: "Typed".into(),
            rust_variant: Some("Typed".into()),
            rust_value: None,
            value: Some(0),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Completion".into(),
            rust_variant: Some("Completion".into()),
            rust_value: None,
            value: Some(1),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Correction".into(),
            rust_variant: Some("Correction".into()),
            rust_value: None,
            value: Some(2),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Prediction".into(),
            rust_variant: Some("Prediction".into()),
            rust_value: None,
            value: Some(3),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Emoji".into(),
            rust_variant: Some("Emoji".into()),
            rust_value: None,
            value: Some(4),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Glide".into(),
            rust_variant: Some("Glide".into()),
            rust_value: None,
            value: Some(5),
        });
        t.enum_members.push(EnumMemberModel {
            name: "Clipboard".into(),
            rust_variant: Some("Clipboard".into()),
            rust_value: None,
            value: Some(6),
        });
        model.types.push(t);
    }
}
