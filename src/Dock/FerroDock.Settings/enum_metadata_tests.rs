use crate::DockOperationMetadata;
use ferroui_base::metadata::MarkupTyped;
#[test]
fn settings_metadata_keeps_all_original_core_enum_handles_and_no_duplicate_operations() {
    use crate::{DockFloatingWindowHostModeMetadata, DockWindowOwnerModeMetadata};
    use ferrodock_model::core::{DockFloatingWindowHostMode, DockWindowOwnerMode};
    use ferroui_base::data::core::ValueType;
    use ferroui_base::metadata::{MarkupType, MarkupTypeKind};
    crate::register_types();
    for (metadata, value_type, count) in [
        (
            crate::DockWindowStateMetadata::MARKUP,
            ValueType::of::<ferrodock_model::core::DockWindowState>(),
            4,
        ),
        (
            crate::MdiWindowStateMetadata::MARKUP,
            ValueType::of::<ferrodock_model::core::MdiWindowState>(),
            3,
        ),
        (
            DockOperationMetadata::MARKUP,
            ValueType::of::<ferrodock_model::core::DockOperation>(),
            11,
        ),
        (
            DockFloatingWindowHostModeMetadata::MARKUP,
            ValueType::of::<DockFloatingWindowHostMode>(),
            3,
        ),
        (
            DockWindowOwnerModeMetadata::MARKUP,
            ValueType::of::<DockWindowOwnerMode>(),
            5,
        ),
    ] {
        assert_eq!(metadata.kind, MarkupTypeKind::Enum);
        assert_eq!((metadata.handles[0])(), value_type);
        assert_eq!(metadata.enum_members.len(), count);
        assert!(std::ptr::eq(
            MarkupType::find_by_handle(value_type.id()).unwrap(),
            metadata
        ));
        for member in metadata.enum_members {
            assert_eq!(
                (metadata.enum_to_value.unwrap())(&*(member.get)()),
                Some(member.value)
            );
            assert!((metadata.enum_from_value.unwrap())(member.value).is_some());
        }
        let open_numeric = value_type == ValueType::of::<DockWindowOwnerMode>()
            || value_type == ValueType::of::<ferrodock_model::core::DockWindowState>()
            || value_type == ValueType::of::<DockFloatingWindowHostMode>();
        for unknown in [i64::from(i32::MIN), -1, 123, i64::from(i32::MAX)] {
            let value = (metadata.enum_from_value.unwrap())(unknown);
            if open_numeric {
                let value = value.expect("original signed window enum value");
                assert_eq!((metadata.enum_to_value.unwrap())(&*value), Some(unknown));
            } else {
                // Other plain enum representations still have the recorded native gap.
                assert!(value.is_none());
            }
        }
        for outside in [
            i64::MIN,
            i64::from(i32::MIN) - 1,
            i64::from(i32::MAX) + 1,
            i64::MAX,
        ] {
            assert!((metadata.enum_from_value.unwrap())(outside).is_none());
        }
    }
}
#[test]
fn original_settings_owned_enums_and_static_field_property_kinds_are_preserved() {
    use crate::{
        DockCommandBarMergingScope as Scope, DockFloatingWindowOwnerPolicy as Policy,
        DockGlobalDockingPreset as Preset, DockSettings, WindowDragDockScope as DragScope,
    };
    assert_eq!(Policy::Default as i32, 0);
    assert_eq!(Policy::AlwaysOwned as i32, 1);
    assert_eq!(Policy::NeverOwned as i32, 2);
    assert_eq!(Preset::LocalFirst as i32, 0);
    assert_eq!(Preset::GlobalFirst as i32, 1);
    assert_eq!(Scope::ActiveDocument as i32, 0);
    assert_eq!(Scope::ActiveDockable as i32, 1);
    assert_eq!(DragScope::FullWindow as i32, 0);
    assert_eq!(DragScope::WindowChrome as i32, 1);
    assert_eq!(Policy::default(), Policy::Default);
    assert_eq!(Preset::default(), Preset::LocalFirst);
    assert_eq!(Scope::default(), Scope::ActiveDocument);
    assert_eq!(DragScope::default(), DragScope::FullWindow);
    assert_eq!(Policy::try_from(2), Ok(Policy::NeverOwned));
    assert_eq!(Policy::try_from(-1), Err(-1));
    assert_eq!(Preset::try_from(1), Ok(Preset::GlobalFirst));
    assert_eq!(Preset::try_from(2), Err(2));
    assert_eq!(Scope::try_from(1), Ok(Scope::ActiveDockable));
    assert_eq!(Scope::try_from(2), Err(2));
    assert_eq!(DragScope::try_from(1), Ok(DragScope::WindowChrome));
    assert_eq!(DragScope::try_from(2), Err(2));
    let metadata = DockSettings::MARKUP;
    assert!(metadata
        .fields
        .iter()
        .any(|f| f.name == "MinimumHorizontalDragDistance"));
    assert!(!metadata
        .static_properties
        .iter()
        .any(|f| f.name == "MinimumHorizontalDragDistance"));
    assert!(metadata
        .static_properties
        .iter()
        .any(|f| f.name == "SelectorEnabled" && f.set.is_some()));
    assert!(metadata
        .static_properties
        .iter()
        .any(|f| f.name == "DocumentSelectorKeyGesture" && f.set.is_some()));
}
