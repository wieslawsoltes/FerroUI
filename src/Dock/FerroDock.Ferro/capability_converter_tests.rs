// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original converter scenarios and contract edge cases on production models.
use crate::converters::{
    CanRemoveDockableConverter, DockCapabilityConverters as C,
    DocumentCloseButtonVisibilityConverter,
};
use ferrodock_model::{
    collections::ListHandle,
    core::{
        DockCapabilityOverrides, DockCapabilityPolicy, DocumentCloseButtonShowMode, IDock,
        IDockCapabilityPolicy, IDockable,
    },
};
use ferrodock_model_reactiveui::controls::{RootDock, Tool, ToolDock};
use ferroui_base::{
    data::{
        converters::{IMultiValueConverter, IValueConverter},
        core::{ValueType, ValueTypes},
    },
    metadata::MarkupTyped,
    utilities::CultureInfo,
    BoxedValue,
};
use std::rc::Rc;
fn hierarchy() -> (Rc<RootDock>, Rc<ToolDock>, Rc<Tool>) {
    let root = RootDock::new();
    let dock = ToolDock::new();
    let tool = Tool::new();
    dock.set_owner(Some(root.clone()));
    tool.set_owner(Some(dock.clone()));
    root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        dock.clone() as Rc<dyn IDockable>
    )])));
    dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        tool.clone() as Rc<dyn IDockable>
    )])));
    root.set_active_dockable(Some(dock.clone()));
    dock.set_active_dockable(Some(tool.clone()));
    (root, dock, tool)
}
fn value(d: Rc<dyn IDockable>) -> BoxedValue {
    crate::model_projection::to_dockable_value(d)
}
fn single(c: Rc<dyn IValueConverter>, v: Option<&BoxedValue>) -> bool {
    *c.convert(
        v,
        ValueType::of::<String>(),
        Some(&(Rc::new(17_i32) as BoxedValue)),
        &CultureInfo::get_culture_info("tr-TR"),
    )
    .unwrap()
    .unwrap()
    .downcast_ref::<bool>()
    .unwrap()
}
fn multi(c: &dyn IMultiValueConverter, v: &[Option<BoxedValue>]) -> bool {
    *c.convert(
        v,
        ValueType::of::<String>(),
        None,
        &CultureInfo::invariant_culture(),
    )
    .unwrap()
    .unwrap()
    .downcast_ref::<bool>()
    .unwrap()
}
#[test]
fn source_can_close_uses_dock_policy() {
    let (_root, dock, tool) = hierarchy();
    tool.set_can_close(true);
    let p = DockCapabilityPolicy::new();
    p.set_can_close(Some(false));
    dock.set_dock_capability_policy(Some(p));
    assert!(!single(C::can_close_converter(), Some(&value(tool))));
}
#[test]
fn source_can_close_uses_override_over_dock_policy() {
    let (_root, dock, tool) = hierarchy();
    tool.set_can_close(true);
    let p = DockCapabilityPolicy::new();
    p.set_can_close(Some(false));
    dock.set_dock_capability_policy(Some(p));
    let p = DockCapabilityOverrides::new();
    p.set_can_close(Some(true));
    tool.set_dock_capability_overrides(Some(p));
    assert!(single(C::can_close_converter(), Some(&value(tool))));
}
#[test]
fn source_can_drop_uses_root_policy_for_dock() {
    let (root, dock, _tool) = hierarchy();
    let p = DockCapabilityPolicy::new();
    p.set_can_drop(Some(false));
    root.set_root_dock_capability_policy(Some(p));
    assert!(!single(C::can_drop_converter(), Some(&value(dock))));
}
#[test]
fn source_capability_rejects_non_dockable_and_null() {
    assert!(!single(
        C::can_close_converter(),
        Some(&(Rc::new("not-dockable".to_owned()) as BoxedValue))
    ));
    assert!(!single(C::can_close_converter(), None));
    assert!(!multi(&*C::can_close_multi_converter(), &[]));
    assert!(!multi(
        &*C::can_close_multi_converter(),
        &[None, Some(Rc::new(true))]
    ));
}
#[test]
fn source_capability_convert_back_is_unsupported() {
    let error = C::can_close_converter()
        .convert_back(
            None,
            ValueType::of::<bool>(),
            None,
            &CultureInfo::invariant_culture(),
        )
        .unwrap_err();
    assert_eq!(error.to_string(), "Specified method is not supported.");
}
#[test]
fn source_can_dock_as_document_uses_override() {
    let (_root, dock, tool) = hierarchy();
    let p = DockCapabilityPolicy::new();
    p.set_can_dock_as_document(Some(false));
    dock.set_dock_capability_policy(Some(p));
    let p = DockCapabilityOverrides::new();
    p.set_can_dock_as_document(Some(true));
    tool.set_dock_capability_overrides(Some(p));
    assert!(single(
        C::can_dock_as_document_converter(),
        Some(&value(tool))
    ));
}
#[test]
fn every_capability_single_and_multi_field_share_original_instance_and_only_first_value_is_evaluated(
) {
    crate::register_types();
    let (_root, _dock, tool) = hierarchy();
    tool.set_can_close(false);
    tool.set_can_pin(true);
    tool.set_can_float(false);
    tool.set_can_drag(true);
    tool.set_can_drop(false);
    tool.set_can_dock_as_document(true);
    for (name, mname, one, many, expected) in [
        (
            "CanCloseConverter",
            "CanCloseMultiConverter",
            C::can_close_converter(),
            C::can_close_multi_converter(),
            false,
        ),
        (
            "CanPinConverter",
            "CanPinMultiConverter",
            C::can_pin_converter(),
            C::can_pin_multi_converter(),
            true,
        ),
        (
            "CanFloatConverter",
            "CanFloatMultiConverter",
            C::can_float_converter(),
            C::can_float_multi_converter(),
            false,
        ),
        (
            "CanDragConverter",
            "CanDragMultiConverter",
            C::can_drag_converter(),
            C::can_drag_multi_converter(),
            true,
        ),
        (
            "CanDropConverter",
            "CanDropMultiConverter",
            C::can_drop_converter(),
            C::can_drop_multi_converter(),
            false,
        ),
        (
            "CanDockAsDocumentConverter",
            "CanDockAsDocumentMultiConverter",
            C::can_dock_as_document_converter(),
            C::can_dock_as_document_multi_converter(),
            true,
        ),
    ] {
        assert!(std::ptr::addr_eq(Rc::as_ptr(&one), Rc::as_ptr(&many)));
        let one_field = (C::MARKUP.find_field(name).unwrap().get)().unwrap();
        let many_field = (C::MARKUP.find_field(mname).unwrap().get)().unwrap();
        assert!(Rc::ptr_eq(
            one_field.downcast_ref::<Rc<dyn IValueConverter>>().unwrap(),
            &one
        ));
        assert!(Rc::ptr_eq(
            many_field
                .downcast_ref::<Rc<dyn IMultiValueConverter>>()
                .unwrap(),
            &many
        ));
        let v = value(tool.clone());
        assert_eq!(single(one, Some(&v)), expected);
        assert_eq!(
            multi(
                &*many,
                &[
                    Some(v),
                    Some(Rc::new(!expected)),
                    Some(Rc::new("irrelevant".to_owned()))
                ]
            ),
            expected
        );
    }
}
#[test]
fn source_remove_converter_keeps_opened_count_distinct_from_visible_count_and_defaults_true() {
    let c = CanRemoveDockableConverter::instance();
    let dock = RootDock::new();
    dock.set_can_close_last_dockable(false);
    dock.set_opened_dockables_count(1);
    // The single-model overload uses OpenedDockablesCount, not Count.
    dock.set_visible_dockables(Some(ListHandle::from_items(vec![
        Some(Tool::new() as Rc<dyn IDockable>),
        Some(Tool::new()),
    ])));
    assert!(!multi(&*c, &[Some(value(dock.clone()))]));
    for count in [-1, 0, 1, 2, 7] {
        dock.set_opened_dockables_count(count);
        assert_eq!(multi(&*c, &[Some(value(dock.clone()))]), count > 1);
        for can_close in [false, true] {
            assert_eq!(
                multi(&*c, &[Some(Rc::new(can_close)), Some(Rc::new(count)), None]),
                can_close || count > 1
            );
        }
    }
    for vals in [
        vec![],
        vec![None],
        vec![Some(Rc::new(false) as BoxedValue)],
        vec![Some(Rc::new(false)), Some(Rc::new(1_i64))],
        vec![Some(value(dock)), Some(Rc::new(0_i32))],
    ] {
        assert!(multi(&*c, &vals));
    }
}
#[test]
fn close_visibility_retains_typed_text_numeric_composite_enum_and_active_rules() {
    let c = DocumentCloseButtonVisibilityConverter::new();
    assert!(multi(&*c, &[]));
    assert!(multi(
        &*c,
        &[Some(Rc::new(DocumentCloseButtonShowMode::Never))]
    ));
    let modes: Vec<(Option<BoxedValue>, DocumentCloseButtonShowMode)> = vec![
        (None, DocumentCloseButtonShowMode::Always),
        (
            Some(Rc::new(DocumentCloseButtonShowMode::Always)),
            DocumentCloseButtonShowMode::Always,
        ),
        (
            Some(Rc::new(DocumentCloseButtonShowMode::Active)),
            DocumentCloseButtonShowMode::Active,
        ),
        (
            Some(Rc::new(DocumentCloseButtonShowMode::Never)),
            DocumentCloseButtonShowMode::Never,
        ),
        (Some(Rc::new(1_i32)), DocumentCloseButtonShowMode::Always),
    ];
    for (v, mode) in modes {
        for a in [
            None,
            Some(Rc::new(true) as BoxedValue),
            Some(Rc::new(false)),
            Some(Rc::new(1_i32)),
        ] {
            let active = a
                .as_ref()
                .and_then(|v| v.downcast_ref::<bool>())
                .copied()
                .unwrap_or(false);
            assert_eq!(
                multi(&*c, &[v.clone(), a]),
                mode == DocumentCloseButtonShowMode::Always
                    || mode == DocumentCloseButtonShowMode::Active && active
            );
        }
    }
    for (text, expected) in [
        ("Always", true),
        ("aCtIvE", false),
        ("  NeVeR\t", false),
        ("+1", false),
        ("2", false),
        ("-1", true),
        ("3", true),
        ("2147483648", true),
        ("Always, Active", false),
        ("Never,Always", false),
        ("Active,Never", true),
        ("Active,Active", false),
        ("Active,", true),
        ("1,Always", true),
        ("", true),
        ("unknown", true),
        ("Active\0", true),
    ] {
        assert_eq!(
            multi(&*c, &[Some(Rc::new(text.to_owned())), Some(Rc::new(false))]),
            expected,
            "{text:?}"
        );
    }
}
#[test]
fn converter_constructors_instances_metadata_and_native_multi_casts_preserve_identity() {
    crate::register_types();
    let a = CanRemoveDockableConverter::instance();
    let b = DocumentCloseButtonVisibilityConverter::instance();
    assert!(Rc::ptr_eq(&a, &CanRemoveDockableConverter::instance()));
    assert!(!Rc::ptr_eq(&a, &CanRemoveDockableConverter::new()));
    assert!(Rc::ptr_eq(
        &b,
        &DocumentCloseButtonVisibilityConverter::instance()
    ));
    assert!(!Rc::ptr_eq(
        &b,
        &DocumentCloseButtonVisibilityConverter::new()
    ));
    for (metadata, v) in [
        (CanRemoveDockableConverter::MARKUP, Rc::new(a) as BoxedValue),
        (
            DocumentCloseButtonVisibilityConverter::MARKUP,
            Rc::new(b) as BoxedValue,
        ),
    ] {
        let actual =
            ValueTypes::try_cast(&v, ValueType::of::<Rc<dyn IMultiValueConverter>>()).unwrap();
        assert!(multi(
            &**actual
                .downcast_ref::<Rc<dyn IMultiValueConverter>>()
                .unwrap(),
            &[]
        ));
        let made = (metadata.constructors[0].invoke)(&[]).unwrap().unwrap();
        assert!(!v.as_ref().any_value_eq(made.as_ref()));
        let instance = (metadata.find_field("Instance").unwrap().get)().unwrap();
        assert!(v.as_ref().any_value_eq(instance.as_ref()));
    }
}
