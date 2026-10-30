use crate::DockOperationMetadata;
use ferroui_base::metadata::MarkupTyped;
#[test]
fn open_window_enum_metadata_preserves_original_typed_numeric_and_nullable_values() {
    use ferrodock_model::core::{DockWindowOwnerMode, DockWindowState};
    use ferroui_base::{
        data::core::{ValueType, ValueTypes},
        BoxedValue,
    };
    use std::rc::Rc;
    crate::register_types();
    macro_rules! verify {
        ($type:ty,$metadata:ty,$kind:literal) => {{
            let metadata = <$metadata>::MARKUP;
            assert!(!metadata.is_flags);
            for row in
                include_str!("../../../tests/Dock/data/serializer-json-audit.window.values.tsv").lines()
            {
                let fields: Vec<_> = row.split('\t').collect();
                if fields[0] != $kind {
                    continue;
                }
                let raw = fields[1].parse::<i32>().unwrap();
                let expected = <$type>::from_raw(raw);
                let value = (metadata.enum_from_value.unwrap())(i64::from(raw)).unwrap();
                assert_eq!(value.downcast_ref::<$type>(), Some(&expected));
                assert_eq!(
                    (metadata.enum_to_value.unwrap())(&*value),
                    Some(i64::from(raw))
                );
                let input: BoxedValue = Rc::new(fields[1].to_owned());
                let converted = ValueTypes::try_convert(Some(&input), ValueType::of::<$type>())
                    .unwrap()
                    .unwrap();
                assert_eq!(converted.downcast_ref::<$type>(), Some(&expected));
                let optional =
                    ValueTypes::try_convert(Some(&input), ValueType::of::<Option<$type>>())
                        .unwrap()
                        .unwrap();
                assert_eq!(
                    optional.downcast_ref::<Option<$type>>(),
                    Some(&Some(expected))
                );
            }
            for raw in [
                i64::MIN,
                i64::from(i32::MIN) - 1,
                i64::from(i32::MAX) + 1,
                i64::MAX,
            ] {
                assert!((metadata.enum_from_value.unwrap())(raw).is_none());
            }
        }};
    }
    verify!(
        DockWindowOwnerMode,
        crate::DockWindowOwnerModeMetadata,
        "OWNER"
    );
    verify!(DockWindowState, crate::DockWindowStateMetadata, "STATE");
}
#[test]
fn docking_state_flags_export_original_names_storage_and_runtime_conversion() {
    use crate::DockingWindowStateMetadata as Descriptor;
    use ferrodock_model::core::DockingWindowState as State;
    use ferroui_base::{
        data::core::{ValueType, ValueTypes},
        metadata::{MarkupType, MarkupTypeKind},
        BoxedValue,
    };
    use std::rc::Rc;
    crate::register_types();
    let metadata = Descriptor::MARKUP;
    assert_eq!(
        metadata.full_name(),
        "FerroDock.Model.Core.DockingWindowState"
    );
    assert_eq!(metadata.kind, MarkupTypeKind::Enum);
    assert!(metadata.is_flags);
    assert_eq!(metadata.enum_members.len(), 6);
    assert_eq!((metadata.handles[0])(), ValueType::of::<State>());
    assert_eq!(
        metadata.nullable.unwrap()(),
        ValueType::of::<Option<State>>()
    );
    assert!(std::ptr::eq(
        MarkupType::find_by_handle(ValueType::of::<State>().id()).unwrap(),
        metadata
    ));
    let model = ferroui_build_scan::model::AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferrodock_model_ferro.xamlmeta"
    )))
    .unwrap();
    let exported = model
        .types
        .iter()
        .find(|type_| type_.name == "DockingWindowState")
        .unwrap();
    assert_eq!(exported.kind, ferroui_build_scan::model::TypeKind::Enum);
    assert!(exported.is_flags);
    assert_eq!(
        exported.handles[0].text,
        "::ferrodock_model::core::DockingWindowState"
    );
    assert_eq!(exported.enum_members.len(), 6);
    for ((name, bits), (member, export)) in [
        ("None", 0),
        ("Docked", 1),
        ("Pinned", 2),
        ("Document", 4),
        ("Floating", 8),
        ("Hidden", 16),
    ]
    .into_iter()
    .zip(metadata.enum_members.iter().zip(&exported.enum_members))
    {
        assert_eq!((member.name, member.value), (name, bits));
        assert_eq!((export.name.as_str(), export.value), (name, Some(bits)));
        assert_eq!(
            (member.get)().downcast_ref::<State>().unwrap().bits(),
            bits as i32
        );
        assert!(member.rust_variant.is_none());
    }
    let mut count = 0;
    for line in include_str!("../../../tests/Dock/data/docking-window-state-display.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let (bits, text) = line.split_once('\t').unwrap();
        let bits = bits.parse::<i32>().unwrap();
        let value = (metadata.enum_from_value.unwrap())(i64::from(bits)).unwrap();
        assert_eq!(value.downcast_ref::<State>().unwrap().to_string(), text);
        assert_eq!(
            (metadata.enum_to_value.unwrap())(&*value),
            Some(i64::from(bits))
        );
        assert_eq!(Descriptor::__markup_flags(i64::from(bits)).bits(), bits);
        let optional = ValueTypes::try_cast(&value, ValueType::of::<Option<State>>()).unwrap();
        assert_eq!(
            optional.downcast_ref::<Option<State>>(),
            Some(&Some(State::from_bits_retain(bits)))
        );
        count += 1;
    }
    assert_eq!(count, 98);
    for outside in [
        i64::MIN,
        i64::from(i32::MIN) - 1,
        i64::from(i32::MAX) + 1,
        i64::MAX,
    ] {
        assert!((metadata.enum_from_value.unwrap())(outside).is_none());
    }
    let unrelated: BoxedValue = Rc::new(4_i32);
    assert!((metadata.enum_to_value.unwrap())(&*unrelated).is_none());
    assert_eq!(
        [
            Descriptor::None,
            Descriptor::Docked,
            Descriptor::Pinned,
            Descriptor::Document,
            Descriptor::Floating,
            Descriptor::Hidden
        ]
        .map(|state| state.bits()),
        [0, 1, 2, 4, 8, 16]
    );
}

#[test]
fn generated_build_export_agrees_with_runtime_descriptor_and_original_handle() {
    let model = ferroui_build_scan::model::AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferrodock_model_ferro.xamlmeta"
    )))
    .unwrap();
    let type_ = model
        .types
        .iter()
        .find(|t| t.name == "DockOperation")
        .unwrap();
    let metadata = DockOperationMetadata::MARKUP;
    assert_eq!(type_.kind, ferroui_build_scan::model::TypeKind::Enum);
    assert_eq!(
        type_.handles[0].text,
        "::ferrodock_model::core::DockOperation"
    );
    assert_eq!(type_.namespace, metadata.namespace());
    assert_eq!(type_.enum_members.len(), metadata.enum_members.len());
    for (member, runtime) in type_.enum_members.iter().zip(metadata.enum_members) {
        assert_eq!(member.name, runtime.name);
        assert_eq!(member.value, Some(runtime.value));
        assert_eq!(member.rust_variant.as_deref(), runtime.rust_variant);
    }
}

#[test]
fn command_bar_foreign_enum_metadata_uses_original_domain_handles_and_exported_names() {
    use crate::{DockCommandBarKindMetadata, DockCommandBarMergeModeMetadata};
    use ferrodock_model::command_bars::{DockCommandBarKind, DockCommandBarMergeMode};
    use ferroui_base::{data::core::ValueType, metadata::MarkupType};
    crate::register_types();
    let model = ferroui_build_scan::model::AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferrodock_model_ferro.xamlmeta"
    )))
    .unwrap();
    for (metadata, handle, names) in [
        (
            DockCommandBarKindMetadata::MARKUP,
            ValueType::of::<DockCommandBarKind>(),
            ["Menu", "ToolBar", "Ribbon"],
        ),
        (
            DockCommandBarMergeModeMetadata::MARKUP,
            ValueType::of::<DockCommandBarMergeMode>(),
            ["Replace", "Append", "MergeByGroup"],
        ),
    ] {
        assert_eq!((metadata.handles[0])(), handle);
        assert!(std::ptr::eq(
            MarkupType::find_by_handle(handle.id()).unwrap(),
            metadata
        ));
        assert_eq!(metadata.namespace(), "FerroDock.Model.CommandBars");
        let exported = model
            .types
            .iter()
            .find(|t| t.name == metadata.name)
            .unwrap();
        assert_eq!(exported.namespace, metadata.namespace());
        for (index, (member, name)) in metadata.enum_members.iter().zip(names).enumerate() {
            assert_eq!(member.name, name);
            assert_eq!(member.value, index as i64);
            assert_eq!(exported.enum_members[index].name, name);
            assert_eq!(
                (metadata.enum_to_value.unwrap())(&*(member.get)()),
                Some(index as i64)
            );
        }
        assert!((metadata.enum_from_value.unwrap())(3).is_none());
    }
}

#[test]
fn dock_control_enums_keep_original_handles_names_signed_values_and_compiler_models() {
    use ferrodock_model::core::{
        Alignment, DockCapability, DocumentCloseButtonShowMode, DocumentLayoutMode,
        DocumentTabLayout, GripMode,
    };
    use ferroui_base::{data::core::ValueType, metadata::MarkupType};
    crate::register_types();
    let model = ferroui_build_scan::model::AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferrodock_model_ferro.xamlmeta"
    )))
    .unwrap();
    for (metadata, handle, count) in [
        (
            crate::PinnedDockDisplayModeMetadata::MARKUP,
            ValueType::of::<ferrodock_model::core::PinnedDockDisplayMode>(),
            2,
        ),
        (
            crate::DocumentLayoutModeMetadata::MARKUP,
            ValueType::of::<DocumentLayoutMode>(),
            2,
        ),
        (
            crate::DocumentTabLayoutMetadata::MARKUP,
            ValueType::of::<DocumentTabLayout>(),
            3,
        ),
        (
            crate::DocumentCloseButtonShowModeMetadata::MARKUP,
            ValueType::of::<DocumentCloseButtonShowMode>(),
            3,
        ),
        (
            crate::AlignmentMetadata::MARKUP,
            ValueType::of::<Alignment>(),
            5,
        ),
        (
            crate::GripModeMetadata::MARKUP,
            ValueType::of::<GripMode>(),
            3,
        ),
        (
            crate::DockCapabilityMetadata::MARKUP,
            ValueType::of::<DockCapability>(),
            6,
        ),
    ] {
        assert_eq!(metadata.namespace(), "FerroDock.Model.Core");
        assert_eq!(metadata.enum_members.len(), count);
        assert!(std::ptr::eq(
            MarkupType::find_by_handle(handle.id()).unwrap(),
            metadata
        ));
        let exported = model
            .types
            .iter()
            .find(|t| t.name == metadata.name)
            .unwrap();
        assert_eq!(exported.enum_members.len(), count);
        for (source, member) in exported.enum_members.iter().zip(metadata.enum_members) {
            assert_eq!(source.name, member.name);
            assert_eq!(source.value, Some(member.value));
            let value = (metadata.enum_from_value.unwrap())(member.value).unwrap();
            assert_eq!(
                (metadata.enum_to_value.unwrap())(&*value),
                Some(member.value)
            );
        }
        assert!((metadata.enum_from_value.unwrap())(i64::MAX).is_none());
    }
    assert_eq!(crate::AlignmentMetadata::Unset, Alignment::Unset);
    assert_eq!(Alignment::Unset as i32, -1);
}

#[test]
fn model_assembly_owns_split_view_enum_metadata_and_keeps_original_signed_values() {
    use ferroui_base::{
        data::core::ValueType,
        metadata::{MarkupAssembly, MarkupType},
    };
    crate::register_types();
    assert_eq!(crate::ASSEMBLY.name, "FerroDock.Model");
    assert!(std::ptr::eq(
        MarkupAssembly::find("FerroDock.Model").unwrap(),
        &crate::ASSEMBLY
    ));
    let model = ferroui_build_scan::model::AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferrodock_model_ferro.xamlmeta"
    )))
    .unwrap();
    assert_eq!(model.name, "FerroDock.Model");
    for (metadata, handle, names) in [
        (
            crate::SplitViewDisplayModeMetadata::MARKUP,
            ValueType::of::<ferrodock_model::core::SplitViewDisplayMode>(),
            ["Inline", "CompactInline", "Overlay", "CompactOverlay"],
        ),
        (
            crate::SplitViewPanePlacementMetadata::MARKUP,
            ValueType::of::<ferrodock_model::core::SplitViewPanePlacement>(),
            ["Left", "Right", "Top", "Bottom"],
        ),
    ] {
        assert!(std::ptr::eq(
            MarkupType::find_by_handle(handle.id()).unwrap(),
            metadata
        ));
        let exported = model
            .types
            .iter()
            .find(|value| value.name == metadata.name)
            .unwrap();
        assert_eq!(metadata.namespace(), "FerroDock.Model.Core");
        assert_eq!(
            metadata
                .enum_members
                .iter()
                .map(|value| value.name)
                .collect::<Vec<_>>(),
            names
        );
        for (i, member) in metadata.enum_members.iter().enumerate() {
            assert_eq!(member.value, i as i64);
            assert_eq!(exported.enum_members[i].value, Some(i as i64));
        }
        assert!((metadata.enum_from_value.unwrap())(-1).is_none());
    }
}
