//! Original converter behavior through the shared converter and metadata contracts.
use crate::converters::NotNullConverter;
use ferroui_base::{
    data::{
        converters::IValueConverter,
        core::{ValueType, ValueTypes},
    },
    metadata::MarkupTyped,
    utilities::CultureInfo,
    BoxedValue,
};
use std::rc::Rc;

#[test]
fn nullness_and_inversion_preserve_original_boolean_and_ordinal_string_rules() {
    let converter = NotNullConverter::new();
    let parameters: Vec<(Option<BoxedValue>, bool)> = vec![
        (None, false),
        (Some(Rc::new(false)), false),
        (Some(Rc::new(true)), true),
        (Some(Rc::new("Not".to_owned())), true),
        (Some(Rc::new("nOt".to_owned())), true),
        (Some(Rc::new("NOT".to_owned())), true),
        (Some(Rc::new(" Not".to_owned())), false),
        (Some(Rc::new("Not ".to_owned())), false),
        (Some(Rc::new("".to_owned())), false),
        (Some(Rc::new(1_i32)), false),
    ];
    // False, zero and empty string are objects, not null.
    let values: Vec<Option<BoxedValue>> = vec![
        None,
        Some(Rc::new(false)),
        Some(Rc::new(0_i32)),
        Some(Rc::new(String::new())),
    ];
    for culture in [
        CultureInfo::invariant_culture(),
        CultureInfo::get_culture_info("tr-TR"),
    ] {
        for (parameter, invert) in &parameters {
            for value in &values {
                let result = converter
                    .convert(
                        value.as_ref(),
                        ValueType::of::<String>(),
                        parameter.as_ref(),
                        &culture,
                    )
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    result.downcast_ref::<bool>(),
                    Some(&(value.is_some() != *invert))
                );
            }
        }
    }
    assert!(converter
        .convert_back(
            None,
            ValueType::of::<bool>(),
            None,
            &CultureInfo::invariant_culture()
        )
        .is_err());
}

#[test]
fn static_converter_metadata_retains_instance_identity_and_shared_interface_cast() {
    crate::register_types();
    let instance = NotNullConverter::instance();
    assert!(Rc::ptr_eq(&instance, &NotNullConverter::instance()));
    assert!(!Rc::ptr_eq(&instance, &NotNullConverter::new()));
    let field = NotNullConverter::MARKUP
        .fields
        .iter()
        .find(|field| field.name == "Instance")
        .unwrap();
    let value = (field.get)().unwrap();
    assert!(Rc::ptr_eq(
        value.downcast_ref::<Rc<NotNullConverter>>().unwrap(),
        &instance
    ));
    let cast = ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IValueConverter>>()).unwrap();
    let expected: Rc<dyn IValueConverter> = instance;
    assert!(Rc::ptr_eq(
        cast.downcast_ref::<Rc<dyn IValueConverter>>().unwrap(),
        &expected
    ));
}

#[test]
fn managed_tool_converters_resolve_original_window_layout_and_keep_same_tool_dock() {
    use crate::{
        controls::ManagedDockWindowDocument,
        converters::{
            ManagedDockableIsToolWindowConverter as IsTool,
            ManagedDockableToolDockConverter as ToolDockConverter,
        },
        model_projection,
    };
    use ferrodock_model::controls::IToolDock;
    use ferrodock_model_reactiveui::{
        controls::{DocumentDock, RootDock, ToolDock},
        core::DockWindow,
        Factory,
    };
    crate::register_types();
    let factory = Factory::new();
    let root = RootDock::new();
    let tool = ToolDock::new();
    let document_dock = DocumentDock::new();
    root.set_factory(Some(factory.clone()));
    root.set_visible_dockables(Some(ferrodock_model::collections::ListHandle::from_plain(
        vec![Some(tool.clone()), Some(document_dock.clone())],
    )));
    root.set_active_dockable(Some(tool.clone()));
    let window = DockWindow::new();
    window.set_factory(Some(factory));
    window.set_layout(Some(root.clone()));
    root.set_window(Some(window.clone()));
    let document = ManagedDockWindowDocument::new(window.clone());
    let inputs = vec![
        Rc::new(document.clone()) as BoxedValue,
        model_projection::to_dockable_value(document.clone()),
        model_projection::to_mdi_document_value(document.clone()),
        model_projection::to_managed_dockable_value(document.clone()),
    ];
    let culture = CultureInfo::invariant_culture();
    let target = ValueType::of::<String>();
    for input in &inputs {
        let value = IsTool::instance()
            .convert(Some(input), target, None, &culture)
            .unwrap()
            .unwrap();
        assert_eq!(value.downcast_ref::<bool>(), Some(&true));
        let converted = ToolDockConverter::instance()
            .convert(Some(input), target, None, &culture)
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(
            converted.downcast_ref::<Rc<dyn IToolDock>>().unwrap(),
            &(tool.clone() as Rc<dyn IToolDock>)
        ));
    }
    root.set_active_dockable(Some(document_dock));
    for input in &inputs {
        assert_eq!(
            IsTool::instance()
                .convert(Some(input), target, None, &culture)
                .unwrap()
                .unwrap()
                .downcast_ref::<bool>(),
            Some(&false)
        );
        assert!(ToolDockConverter::instance()
            .convert(Some(input), target, None, &culture)
            .unwrap()
            .is_none());
    }
    root.set_active_dockable(None);
    assert!(ToolDockConverter::instance()
        .convert(Some(&inputs[0]), target, None, &culture)
        .unwrap()
        .is_none());
    window.set_layout(None);
    assert_eq!(
        IsTool::instance()
            .convert(Some(&inputs[0]), target, None, &culture)
            .unwrap()
            .unwrap()
            .downcast_ref::<bool>(),
        Some(&false)
    );
    document.dispose();
    assert!(ToolDockConverter::instance()
        .convert(Some(&inputs[0]), target, None, &culture)
        .unwrap()
        .is_none());
}
#[test]
fn managed_tool_converter_inversion_null_unrelated_objects_and_error_paths_match_source() {
    use crate::converters::{
        ManagedDockableIsToolWindowConverter as IsTool,
        ManagedDockableToolDockConverter as ToolDockConverter,
    };
    use ferrodock_model_reactiveui::controls::Document;
    crate::register_types();
    let parameters: Vec<(Option<BoxedValue>, bool)> = vec![
        (None, false),
        (Some(Rc::new(false)), false),
        (Some(Rc::new(true)), true),
        (Some(Rc::new("Not".to_owned())), true),
        (Some(Rc::new("nOt".to_owned())), true),
        (Some(Rc::new(" Not".to_owned())), false),
        (Some(Rc::new("Not ".to_owned())), false),
        (Some(Rc::new(1_i32)), false),
    ];
    let values = vec![
        None,
        Some(Rc::new(42_i32) as BoxedValue),
        Some(crate::model_projection::to_dockable_value(Document::new())),
    ];
    for culture in [
        CultureInfo::invariant_culture(),
        CultureInfo::get_culture_info("tr-TR"),
    ] {
        for value in &values {
            for (parameter, expected) in &parameters {
                assert_eq!(
                    IsTool::instance()
                        .convert(
                            value.as_ref(),
                            ValueType::of::<bool>(),
                            parameter.as_ref(),
                            &culture
                        )
                        .unwrap()
                        .unwrap()
                        .downcast_ref::<bool>(),
                    Some(expected)
                );
                assert!(ToolDockConverter::instance()
                    .convert(
                        value.as_ref(),
                        ValueType::of::<bool>(),
                        parameter.as_ref(),
                        &culture
                    )
                    .unwrap()
                    .is_none());
            }
        }
        assert!(IsTool::new()
            .convert_back(None, ValueType::of::<bool>(), None, &culture)
            .is_err());
        assert!(ToolDockConverter::new()
            .convert_back(None, ValueType::of::<bool>(), None, &culture)
            .is_err());
    }
}
#[test]
fn managed_converter_metadata_shares_source_singletons_and_native_value_converter_interfaces() {
    use crate::converters::{
        ManagedDockableIsToolWindowConverter as IsTool,
        ManagedDockableToolDockConverter as ToolDockConverter,
    };
    crate::register_types();
    let is_tool = IsTool::instance();
    let tool = ToolDockConverter::instance();
    assert!(Rc::ptr_eq(&is_tool, &IsTool::instance()));
    assert!(Rc::ptr_eq(&tool, &ToolDockConverter::instance()));
    assert!(!Rc::ptr_eq(&is_tool, &IsTool::new()));
    assert!(!Rc::ptr_eq(&tool, &ToolDockConverter::new()));
    for (metadata, expected) in [
        (IsTool::MARKUP, is_tool as Rc<dyn IValueConverter>),
        (ToolDockConverter::MARKUP, tool as Rc<dyn IValueConverter>),
    ] {
        assert_eq!(metadata.namespace(), "FerroDock.Ferro.Converters");
        let field = metadata
            .fields
            .iter()
            .find(|f| f.name == "Instance")
            .unwrap();
        let instance = (field.get)().unwrap();
        let cast =
            ValueTypes::try_cast(&instance, ValueType::of::<Rc<dyn IValueConverter>>()).unwrap();
        assert!(Rc::ptr_eq(
            cast.downcast_ref::<Rc<dyn IValueConverter>>().unwrap(),
            &expected
        ));
    }
}
