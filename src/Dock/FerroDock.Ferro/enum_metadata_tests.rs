use crate::DockOperationMetadata;
use ferrodock_model::core::DockOperation;
use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::{MarkupType, MarkupTypeKind, MarkupTyped};
#[test]
fn runtime_enum_metadata_retains_original_values_and_rejects_unknown_numbers() {
    crate::register_types();
    let metadata = DockOperationMetadata::MARKUP;
    assert_eq!(metadata.kind, MarkupTypeKind::Enum);
    assert_eq!(metadata.namespace(), "FerroDock.Model.Core");
    assert_eq!((metadata.handles[0])(), ValueType::of::<DockOperation>());
    assert_eq!(
        (metadata.nullable.unwrap())(),
        ValueType::of::<Option<DockOperation>>()
    );
    let expected = [
        DockOperation::None,
        DockOperation::Fill,
        DockOperation::Left,
        DockOperation::Bottom,
        DockOperation::Right,
        DockOperation::Top,
        DockOperation::Window,
        DockOperation::OuterLeft,
        DockOperation::OuterTop,
        DockOperation::OuterRight,
        DockOperation::OuterBottom,
    ];
    assert_eq!(metadata.enum_members.len(), expected.len());
    for (member, expected) in metadata.enum_members.iter().zip(expected) {
        assert_eq!(member.value, expected as i64);
        let value = (member.get)();
        assert_eq!(value.downcast_ref::<DockOperation>(), Some(&expected));
        assert_eq!(
            (metadata.enum_to_value.unwrap())(&*value),
            Some(expected as i64)
        );
        let converted = (metadata.enum_from_value.unwrap())(expected as i64).unwrap();
        assert_eq!(converted.downcast_ref::<DockOperation>(), Some(&expected));
    }
    for unknown in [-1, 11, i64::MIN, i64::MAX] {
        assert!((metadata.enum_from_value.unwrap())(unknown).is_none());
    }
    let found = MarkupType::find_by_handle(std::any::TypeId::of::<DockOperation>()).unwrap();
    assert!(std::ptr::eq(found, metadata));
}
