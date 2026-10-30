//! Original source converter cases needed by the complete document and chrome themes.
use crate::converters::*;
use ferrodock_model::core::{DocumentLayoutMode, DocumentTabLayout, GripMode};
use ferroui_base::{
    data::{
        converters::{IMultiValueConverter, IValueConverter},
        core::ValueType,
    },
    utilities::CultureInfo,
    BoxedValue, UnsetValueType,
};
use std::rc::Rc;
fn converted(converter: &dyn IValueConverter, value: BoxedValue) -> BoxedValue {
    converter
        .convert(
            Some(&value),
            ValueType::of::<bool>(),
            None,
            &CultureInfo::invariant_culture(),
        )
        .unwrap()
        .unwrap()
}
#[test]
fn original_document_tab_orientation_and_dock_converters_map_all_declared_values_and_return_unset()
{
    use ferroui_base::layout::Orientation;
    use ferroui_controls::Dock;
    let orientation = DocumentTabOrientationConverter::instance();
    let dock = DocumentTabDockConverter::instance();
    for (input, o, d) in [
        (DocumentTabLayout::Top, Orientation::Horizontal, Dock::Top),
        (DocumentTabLayout::Left, Orientation::Vertical, Dock::Left),
        (DocumentTabLayout::Right, Orientation::Vertical, Dock::Right),
    ] {
        assert_eq!(
            converted(&*orientation, Rc::new(input)).downcast_ref::<Orientation>(),
            Some(&o)
        );
        assert_eq!(
            converted(&*dock, Rc::new(input)).downcast_ref::<Dock>(),
            Some(&d)
        );
    }
    for c in [orientation as Rc<dyn IValueConverter>, dock] {
        assert!(converted(&*c, Rc::new(123_i32)).is::<UnsetValueType>());
        assert!(c
            .convert_back(
                None,
                ValueType::of::<bool>(),
                None,
                &CultureInfo::invariant_culture()
            )
            .unwrap()
            .unwrap()
            .is::<UnsetValueType>());
    }
}
#[test]
fn original_grip_converter_fields_have_stable_identity_and_all_source_mode_truth_tables() {
    let row = GripModeConverters::grid_row_auto_hide_converter();
    assert!(Rc::ptr_eq(
        &row,
        &GripModeConverters::grid_row_auto_hide_converter()
    ));
    let converters = [
        GripModeConverters::is_visible_visible_converter(),
        GripModeConverters::is_visible_visible_or_hidden_converter(),
        GripModeConverters::is_visible_auto_hide_or_visible_converter(),
        GripModeConverters::is_visible_auto_hide_or_hidden_converter(),
    ];
    for (mode, expected_row, expected) in [
        (GripMode::Visible, 0, [true, true, true, false]),
        (GripMode::Hidden, 0, [false, true, false, true]),
        (GripMode::AutoHide, 1, [false, false, true, true]),
    ] {
        assert_eq!(
            converted(&*row, Rc::new(mode)).downcast_ref::<i32>(),
            Some(&expected_row)
        );
        for (c, e) in converters.iter().zip(expected) {
            assert_eq!(
                converted(&**c, Rc::new(mode)).downcast_ref::<bool>(),
                Some(&e)
            );
        }
    }
}
#[test]
fn original_maximized_converter_checks_actual_state_type_and_rejects_back_conversion() {
    use ferroui_controls::WindowState;
    let c = IsMaximizedConverter::instance();
    assert!(Rc::ptr_eq(&c, &IsMaximizedConverter::instance()));
    for state in [
        WindowState::Normal,
        WindowState::Minimized,
        WindowState::Maximized,
        WindowState::FullScreen,
    ] {
        assert_eq!(
            converted(&*c, Rc::new(state)).downcast_ref::<bool>(),
            Some(&(state == WindowState::Maximized))
        );
    }
    assert_eq!(
        converted(&*c, Rc::new(true)).downcast_ref::<bool>(),
        Some(&false)
    );
    assert!(c
        .convert_back(
            None,
            ValueType::of::<bool>(),
            None,
            &CultureInfo::invariant_culture()
        )
        .is_err());
}
#[test]
fn original_document_layout_match_handles_parameters_and_unset_back_conversion() {
    let c = DocumentLayoutModeMatchConverter::instance();
    for (s, e) in [
        ("Mdi", true),
        ("mdi", true),
        ("Tabbed,Mdi", true),
        ("1", true),
        ("Tabbed", false),
        ("no", false),
    ] {
        let p: BoxedValue = Rc::new(s.to_owned());
        let v: BoxedValue = Rc::new(DocumentLayoutMode::Mdi);
        let result = c
            .convert(
                Some(&v),
                ValueType::of::<bool>(),
                Some(&p),
                &CultureInfo::invariant_culture(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(result.downcast_ref::<bool>(), Some(&e));
    }
    assert!(c
        .convert_back(
            None,
            ValueType::of::<bool>(),
            None,
            &CultureInfo::invariant_culture()
        )
        .unwrap()
        .unwrap()
        .is::<UnsetValueType>());
}
#[test]
fn original_reference_converter_distinguishes_box_identity_from_equal_values_and_recovers_model_handles(
) {
    use ferrodock_model_reactiveui::controls::Document;
    let c = ReferenceEqualsMultiConverter::instance();
    let culture = CultureInfo::invariant_culture();
    let target = ValueType::of::<bool>();
    assert!(c.convert(&[], target, None, &culture).unwrap().is_none());
    let same: BoxedValue = Rc::new(5_i32);
    for (values, expected) in [
        (vec![None], false),
        (vec![Some(same.clone())], true),
        (vec![Some(same.clone()), Some(same)], true),
        (
            vec![Some(Rc::new(5_i32) as BoxedValue), Some(Rc::new(5_i32))],
            false,
        ),
        (vec![None, None], false),
    ] {
        assert_eq!(
            c.convert(&values, target, None, &culture)
                .unwrap()
                .unwrap()
                .downcast_ref::<bool>(),
            Some(&expected)
        );
    }
    let model = Document::new();
    let a = crate::model_projection::to_dockable_value(model.clone());
    let b = crate::model_projection::to_dockable_value(model);
    assert_eq!(
        c.convert(&[Some(a), Some(b)], target, None, &culture)
            .unwrap()
            .unwrap()
            .downcast_ref::<bool>(),
        Some(&true)
    );
}

#[test]
fn original_reference_converter_uses_enclosing_identity_for_all_reactive_class_views() {
    use ferrodock_model_reactiveui::controls::Document;
    use ferrodock_model_reactiveui_ferro::reactive_metadata::DocumentReference;
    ferrodock_model_reactiveui_ferro::register_types();
    let model = Document::new();
    let class = DocumentReference::new(model);
    let values = [
        Some(Rc::new(class.clone()) as BoxedValue),
        Some(Rc::new(class.base()) as BoxedValue),
        Some(Rc::new(class.base().base()) as BoxedValue),
        Some(Rc::new(class.base().base().base()) as BoxedValue),
    ];
    assert_eq!(
        ReferenceEqualsMultiConverter::new()
            .convert(
                &values,
                ValueType::of::<bool>(),
                None,
                &CultureInfo::invariant_culture()
            )
            .unwrap()
            .unwrap()
            .downcast_ref::<bool>(),
        Some(&true)
    );
}
